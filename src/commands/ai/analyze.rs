//! AI usage analysis commands
//!
//! Commands for analyzing CLI usage patterns and suggesting optimizations.

use anyhow::Result;
use colored::Colorize;
use std::cmp::Reverse;

use crate::Database;

use super::discover::format_stars;

/// Detect shell aliases from config files
///
/// Returns a map of alias name -> target command
fn detect_shell_aliases() -> std::collections::HashMap<String, String> {
    use std::collections::HashMap;
    use std::fs;

    let mut aliases: HashMap<String, String> = HashMap::new();

    // Check common shell config files
    let home = match dirs::home_dir() {
        Some(h) => h,
        None => return aliases,
    };

    let config_files = [
        home.join(".bashrc"),
        home.join(".bash_aliases"),
        home.join(".zshrc"),
        home.join(".zsh_aliases"),
        home.join(".config/fish/config.fish"),
        home.join(".config/fish/aliases.fish"),
    ];

    for file in &config_files {
        if let Ok(content) = fs::read_to_string(file) {
            // Parse bash/zsh style: alias name='command' or alias name="command"
            for line in content.lines() {
                let line = line.trim();
                if let Some(rest) = line.strip_prefix("alias ") {
                    // Handle: alias cat='bat' or alias cat="bat --paging=never"
                    if let Some(eq_pos) = rest.find('=') {
                        let name = rest[..eq_pos].trim();
                        let value = rest[eq_pos + 1..].trim();
                        // Remove surrounding quotes
                        let value = value
                            .strip_prefix('\'')
                            .and_then(|v| v.strip_suffix('\''))
                            .or_else(|| value.strip_prefix('"').and_then(|v| v.strip_suffix('"')))
                            .unwrap_or(value);
                        aliases.insert(name.to_string(), value.to_string());
                    }
                }
                // Parse fish style: alias name 'command' or abbr -a name command
                else if line.starts_with("alias ") || line.starts_with("abbr ") {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if parts.len() >= 3 {
                        let name = parts[1].trim_start_matches("-a").trim();
                        let value = parts[2..].join(" ");
                        let value = value.trim_matches('\'').trim_matches('"').to_string();
                        if !name.is_empty() {
                            aliases.insert(name.to_string(), value);
                        }
                    }
                }
            }
        }
    }

    aliases
}

/// Analyze CLI usage and suggest optimizations
pub fn cmd_ai_analyze(db: &Database, json_output: bool, no_ai: bool, min_uses: i64) -> Result<()> {
    use crate::ai::{
        AnalysisResult, AnalyzeTip, MODERN_REPLACEMENTS, UnderutilizedTool, analyze_prompt,
        invoke_ai, is_binary_installed, parse_analyze_response,
    };
    use crate::history::parse_all_histories;
    use indicatif::{ProgressBar, ProgressStyle};

    if !json_output {
        println!("{}", "Usage Analysis".bold());
        println!();
    }

    // 1. Parse ALL shell history to get raw command counts (including untracked tools)
    let spinner = if !json_output {
        let sp = ProgressBar::new_spinner();
        sp.set_style(
            ProgressStyle::default_spinner()
                .template("{spinner:.cyan} {msg}")
                .unwrap(),
        );
        sp.set_message("Scanning shell history...");
        sp.enable_steady_tick(std::time::Duration::from_millis(80));
        Some(sp)
    } else {
        None
    };

    let raw_counts = parse_all_histories()?;

    if let Some(ref sp) = spinner {
        sp.finish_and_clear();
    }

    // 2. Detect shell aliases (to avoid false positives like "use bat" when alias cat='bat' exists)
    let aliases = detect_shell_aliases();

    // 3. Find optimization opportunities (traditional tool used + modern alternative installed)
    let mut tips: Vec<AnalyzeTip> = Vec::new();
    let mut traditional_usage: Vec<(String, i64)> = Vec::new();
    let mut modern_installed: Vec<String> = Vec::new();

    for replacement in MODERN_REPLACEMENTS {
        let trad_uses = raw_counts
            .get(replacement.traditional)
            .copied()
            .unwrap_or(0);
        let modern_uses = raw_counts
            .get(replacement.modern_binary)
            .copied()
            .unwrap_or(0);
        let modern_available = is_binary_installed(replacement.modern_binary);

        if modern_available {
            modern_installed.push(replacement.modern.to_string());
        }

        // Skip if:
        // - Traditional tool usage is below threshold
        // - Modern tool is not installed
        // - Modern tool is already being used directly (modern_uses >= 5)
        // - There's an alias from traditional -> modern (e.g., alias cat='bat')
        let has_alias = aliases
            .get(replacement.traditional)
            .is_some_and(|target: &String| target.contains(replacement.modern_binary));
        let already_using_modern = modern_uses >= 5;

        if trad_uses >= min_uses && modern_available && !already_using_modern && !has_alias {
            traditional_usage.push((replacement.traditional.to_string(), trad_uses));
            tips.push(AnalyzeTip {
                traditional: replacement.traditional.to_string(),
                traditional_uses: trad_uses,
                modern: replacement.modern.to_string(),
                modern_binary: replacement.modern_binary.to_string(),
                benefit: replacement.benefit.to_string(),
                action: replacement.tip.to_string(),
            });
        }
    }

    // Sort tips by usage count (most used first)
    tips.sort_by_key(|a| Reverse(a.traditional_uses));

    // 3. Get unused installed tools (high-value ones)
    let unused_tools = db.get_unused_tools()?;
    let mut underutilized: Vec<UnderutilizedTool> = Vec::new();

    for tool in unused_tools.iter().take(10) {
        // Get GitHub stars if available (stars is i64, convert to Option<u64>)
        let stars = db.get_github_info(&tool.name)?.map(|gh| gh.stars as u64);

        underutilized.push(UnderutilizedTool {
            name: tool.name.clone(),
            description: tool.description.clone(),
            stars,
        });
    }

    // Sort by stars (most popular first) to highlight high-value unused tools
    underutilized.sort_by_key(|a| Reverse(a.stars.unwrap_or(0)));
    underutilized.truncate(5);

    // 4. Optional AI insights
    let ai_insight = if !no_ai && (!tips.is_empty() || !underutilized.is_empty()) {
        if !json_output {
            let sp = ProgressBar::new_spinner();
            sp.set_style(
                ProgressStyle::default_spinner()
                    .template("{spinner:.cyan} {msg}")
                    .unwrap(),
            );
            sp.set_message("Getting AI insights...");
            sp.enable_steady_tick(std::time::Duration::from_millis(80));

            let unused_names: Vec<String> = underutilized.iter().map(|t| t.name.clone()).collect();
            let prompt = analyze_prompt(&traditional_usage, &modern_installed, &unused_names);

            match invoke_ai(&prompt) {
                Ok(response) => {
                    sp.finish_and_clear();
                    parse_analyze_response(&response).ok()
                }
                Err(_) => {
                    sp.finish_and_clear();
                    None
                }
            }
        } else {
            let unused_names: Vec<String> = underutilized.iter().map(|t| t.name.clone()).collect();
            let prompt = analyze_prompt(&traditional_usage, &modern_installed, &unused_names);
            invoke_ai(&prompt)
                .ok()
                .and_then(|r| parse_analyze_response(&r).ok())
        }
    } else {
        None
    };

    // 5. Build result
    let result = AnalysisResult {
        tips,
        underutilized,
        ai_insight,
    };

    // 6. Output results
    if json_output {
        println!("{}", serde_json::to_string_pretty(&result)?);
        return Ok(());
    }

    // Display tips
    if result.tips.is_empty() {
        println!("{} No optimization opportunities found", "+".green());
        println!(
            "  {} Either you're already using modern tools, or no traditional tools",
            ">".dimmed()
        );
        println!(
            "    {} met the minimum usage threshold ({}x)",
            ">".dimmed(),
            min_uses
        );
    } else {
        println!("{}", "Optimization Tips:".green().bold());
        println!();

        for (i, tip) in result.tips.iter().enumerate() {
            println!(
                "{}. You use {} but have {} installed.",
                i + 1,
                format!("{} ({}x)", tip.traditional, tip.traditional_uses).yellow(),
                tip.modern.cyan()
            );
            println!(
                "   {} {}",
                tip.benefit.dimmed(),
                format!("Consider: {}", tip.action).green()
            );
            println!();
        }
    }

    // Display underutilized tools
    if !result.underutilized.is_empty() {
        println!("{}", "High-value unused tools:".blue().bold());
        for tool in &result.underutilized {
            let stars = tool
                .stars
                .map(|s| format!(" ({})", format_stars(s)))
                .unwrap_or_default();
            let desc = tool.description.as_deref().unwrap_or("No description");
            println!(
                "   {} {}{} - {}",
                "•".cyan(),
                tool.name.cyan(),
                stars.dimmed(),
                desc.dimmed()
            );
        }
        println!();
    }

    // Display AI insight if available
    if let Some(insight) = &result.ai_insight {
        println!("{}", "AI Insight:".magenta().bold());
        println!("  {}", insight);
        println!();
    }

    // Summary
    let total_tips = result.tips.len();
    let total_unused = result.underutilized.len();
    if total_tips > 0 || total_unused > 0 {
        println!(
            "{} Found {} optimization tip{} and {} high-value unused tool{}",
            ">".cyan(),
            total_tips,
            if total_tips == 1 { "" } else { "s" },
            total_unused,
            if total_unused == 1 { "" } else { "s" }
        );
    }

    Ok(())
}
