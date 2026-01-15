//! Usage tracking command implementations
//!
//! Commands for tracking and analyzing tool usage from shell history.

use anyhow::Result;
use colored::Colorize;

use crate::Database;

/// Show all labels
pub fn cmd_labels(db: &Database) -> Result<()> {
    use comfy_table::{
        Cell, Color, ContentArrangement, Table, modifiers::UTF8_ROUND_CORNERS, presets::UTF8_FULL,
    };

    let label_counts = db.get_label_counts()?;

    if label_counts.is_empty() {
        println!("{} No labels found", "!".yellow());
        println!("  Sync with GitHub: {}", "hoards gh sync".cyan());
        return Ok(());
    }

    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .apply_modifier(UTF8_ROUND_CORNERS)
        .set_content_arrangement(ContentArrangement::Dynamic)
        .set_header(vec![
            Cell::new("🏷 Label").fg(Color::Cyan),
            Cell::new("Tools").fg(Color::Cyan),
        ]);

    for (label, count) in &label_counts {
        table.add_row(vec![Cell::new(label), Cell::new(count)]);
    }

    println!("{table}");
    println!();
    println!(
        "{} List tools by label: {}",
        ">".cyan(),
        "hoards list --label <label>".yellow()
    );

    Ok(())
}

/// Scan shell history for usage data
pub fn cmd_usage_scan(db: &Database, dry_run: bool, reset: bool) -> Result<()> {
    use crate::history::parse_all_histories;

    println!("{} Scanning shell history...", ">".cyan());

    // Parse all shell histories
    let counts = parse_all_histories()?;

    if counts.is_empty() {
        println!("{} No shell history found", "!".yellow());
        return Ok(());
    }

    println!(
        "{} Found {} unique commands in history",
        ">".cyan(),
        counts.len()
    );

    // Get tool binaries from database for matching
    let tool_binaries = db.get_tool_binaries()?;
    let binary_to_tool: std::collections::HashMap<String, String> = tool_binaries
        .iter()
        .map(|(name, binary)| (binary.clone(), name.clone()))
        .collect();

    // Also match by tool name directly
    let tool_names: std::collections::HashSet<String> =
        tool_binaries.iter().map(|(name, _)| name.clone()).collect();

    // Reset if requested
    if reset && !dry_run {
        db.clear_usage()?;
        println!("{} Cleared existing usage data", ">".cyan());
    }

    // Match commands to tools
    let mut matched = 0;
    let mut total_uses = 0i64;

    let mut tool_counts: Vec<(String, i64)> = Vec::new();

    for (cmd, count) in &counts {
        // Check if command matches a tool binary or name
        let tool_name = binary_to_tool.get(cmd).cloned().or_else(|| {
            if tool_names.contains(cmd) {
                Some(cmd.clone())
            } else {
                None
            }
        });

        if let Some(name) = tool_name {
            tool_counts.push((name, *count));
            matched += 1;
            total_uses += count;
        }
    }

    // Sort by count descending
    tool_counts.sort_by(|a, b| b.1.cmp(&a.1));

    if tool_counts.is_empty() {
        println!("{} No matching tools found in history", "!".yellow());
        return Ok(());
    }

    println!();
    println!(
        "{} Matched {} tool{} ({} total uses):",
        "+".green(),
        matched,
        if matched == 1 { "" } else { "s" },
        total_uses
    );

    // Show top results
    for (name, count) in tool_counts.iter().take(20) {
        if dry_run {
            println!("  {} {:20} {:>6} uses", "[dry]".yellow(), name, count);
        } else {
            db.record_usage(name, *count, None)?;
            println!("  {} {:20} {:>6} uses", "+".green(), name, count);
        }
    }

    if tool_counts.len() > 20 {
        let remaining = tool_counts.len() - 20;
        if !dry_run {
            for (name, count) in tool_counts.iter().skip(20) {
                db.record_usage(name, *count, None)?;
            }
        }
        println!("  {} ...and {} more", "".dimmed(), remaining);
    }

    println!();
    if dry_run {
        println!(
            "{} Run without {} to save usage data",
            ">".cyan(),
            "--dry-run".yellow()
        );
    } else {
        println!("{} Usage data saved", "+".green());
    }

    Ok(())
}

/// Show usage statistics
pub fn cmd_usage_show(db: &Database, limit: usize) -> Result<()> {
    use comfy_table::{
        Cell, Color, ContentArrangement, Table, modifiers::UTF8_ROUND_CORNERS, presets::UTF8_FULL,
    };

    let usage = db.get_all_usage()?;

    if usage.is_empty() {
        println!(
            "{} No usage data yet. Run {} first.",
            "!".yellow(),
            "hoards usage scan".cyan()
        );
        return Ok(());
    }

    let total: i64 = usage.iter().map(|(_, u)| u.use_count).sum();

    let term_width = terminal_size::terminal_size()
        .map(|(w, _)| w.0)
        .unwrap_or(120);

    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .apply_modifier(UTF8_ROUND_CORNERS)
        .set_content_arrangement(ContentArrangement::Dynamic)
        .set_width(term_width)
        .set_header(vec![
            Cell::new("📊 Tool").fg(Color::Cyan),
            Cell::new("Uses").fg(Color::Cyan),
            Cell::new("%").fg(Color::Cyan),
            Cell::new("Usage").fg(Color::Cyan),
        ]);

    for (name, stats) in usage.iter().take(limit) {
        let percent = (stats.use_count as f64 / total as f64) * 100.0;
        let bar_len = (percent / 5.0).round() as usize;
        let bar = "█".repeat(bar_len);

        table.add_row(vec![
            Cell::new(name),
            Cell::new(stats.use_count),
            Cell::new(format!("{:.1}", percent)),
            Cell::new(bar).fg(Color::Green),
        ]);
    }

    println!("{table}");

    if usage.len() > limit {
        println!(
            "{} Showing top {} of {} tools. Use {} to see more.",
            ">".cyan(),
            limit,
            usage.len(),
            "--limit".yellow()
        );
    }

    println!("📈 Total: {} uses across {} tools", total, usage.len());

    Ok(())
}

/// Show usage for a specific tool
pub fn cmd_usage_tool(db: &Database, name: &str) -> Result<()> {
    let usage = db.get_usage(name)?;

    match usage {
        Some(stats) => {
            println!("{} {}", "Usage for".bold(), name.cyan());
            println!("  Uses:       {}", stats.use_count);
            if let Some(last) = &stats.last_used {
                println!("  Last used:  {}", last);
            }
            println!("  First seen: {}", stats.first_seen);
        }
        None => {
            println!("{} No usage data for '{}'", "!".yellow(), name);
            println!("  Run {} to scan shell history", "hoards usage scan".cyan());
        }
    }

    Ok(())
}

/// Show unused tools
pub fn cmd_unused(db: &Database) -> Result<()> {
    use crate::icons::source_icon;
    use comfy_table::{
        Cell, Color, ContentArrangement, Table, modifiers::UTF8_ROUND_CORNERS, presets::UTF8_FULL,
    };

    let unused = db.get_unused_tools()?;

    if unused.is_empty() {
        println!("{} All installed tools have been used!", "✓".green());
        println!(
            "  Run {} first if you haven't already",
            "hoards usage scan".cyan()
        );
        return Ok(());
    }

    println!("{}", "🗑 Installed tools with no recorded usage:".bold());
    println!();

    let term_width = terminal_size::terminal_size()
        .map(|(w, _)| w.0)
        .unwrap_or(120);

    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .apply_modifier(UTF8_ROUND_CORNERS)
        .set_content_arrangement(ContentArrangement::Dynamic)
        .set_width(term_width)
        .set_header(vec![
            Cell::new("Tool").fg(Color::Cyan),
            Cell::new("Src").fg(Color::Cyan),
            Cell::new("Description").fg(Color::Cyan),
        ]);

    for tool in &unused {
        let desc = tool.description.as_deref().unwrap_or("-");
        let src_icon = source_icon(&tool.source.to_string());

        table.add_row(vec![
            Cell::new(&tool.name),
            Cell::new(src_icon),
            Cell::new(desc),
        ]);
    }

    println!("{table}");
    crate::icons::print_legend_compact();
    println!(
        "{} Found {} unused tool{}",
        "!".yellow(),
        unused.len(),
        if unused.len() == 1 { "" } else { "s" }
    );
    println!(
        "  Consider uninstalling with: {}",
        "hoards uninstall <tool>".cyan()
    );

    Ok(())
}

/// Recommend tools based on usage
pub fn cmd_recommend(db: &Database, count: usize) -> Result<()> {
    let usage = db.get_all_usage()?;

    if usage.is_empty() {
        println!(
            "{} No usage data yet. Run {} first.",
            "!".yellow(),
            "hoards usage scan".cyan()
        );
        return Ok(());
    }

    // Get categories of most-used tools
    let mut category_scores: std::collections::HashMap<String, i64> =
        std::collections::HashMap::new();

    for (name, stats) in &usage {
        if let Ok(Some(tool)) = db.get_tool_by_name(name)
            && let Some(cat) = tool.category
        {
            *category_scores.entry(cat).or_insert(0) += stats.use_count;
        }
    }

    // Sort categories by usage
    let mut cats: Vec<_> = category_scores.into_iter().collect();
    cats.sort_by(|a, b| b.1.cmp(&a.1));

    if cats.is_empty() {
        println!("{} Not enough data for recommendations", "!".yellow());
        return Ok(());
    }

    println!("{}", "Tool Recommendations".bold());
    println!("{}", "-".repeat(60));
    println!();

    // Get tools you don't have from top categories
    let mut recommendations = Vec::new();
    let used_tools: std::collections::HashSet<_> = usage.iter().map(|(n, _)| n.clone()).collect();

    for (category, score) in cats.iter().take(3) {
        let tools = db.list_tools(false, Some(category))?;
        for tool in tools {
            if !tool.is_installed
                && !used_tools.contains(&tool.name)
                && recommendations.len() < count
            {
                recommendations.push((tool, category.clone(), *score));
            }
        }
    }

    if recommendations.is_empty() {
        println!(
            "{} You have all the tools in your top categories!",
            "+".green()
        );
        println!("\n{} Your top categories by usage:", ">".cyan());
        for (cat, score) in cats.iter().take(5) {
            println!("  {} {:15} ({} uses)", ">".dimmed(), cat.cyan(), score);
        }
        return Ok(());
    }

    println!("{} Based on your usage, you might like:", ">".cyan());
    println!();

    for (tool, category, _) in &recommendations {
        let desc = tool.description.as_deref().unwrap_or("No description");
        println!(
            "  {} {} ({})",
            "+".green(),
            tool.name.cyan(),
            category.dimmed()
        );
        println!("    {}", desc.dimmed());
        println!();
    }

    println!(
        "{} Install with: {}",
        ">".cyan(),
        "hoards install <tool>".yellow()
    );

    Ok(())
}

/// Log a single command usage (for shell hooks)
/// This is called by shell preexec hooks and must be fast and silent
pub fn cmd_usage_log(db: &Database, command: &str) -> Result<()> {
    use crate::history::extract_command;

    // Extract base command (handles sudo, env vars, etc.)
    let cmd = match extract_command(command) {
        Some(c) => c,
        None => return Ok(()),
    };

    if cmd.is_empty() {
        return Ok(());
    }

    // Fast lookup: is this a tracked tool?
    if let Some(tool_name) = db.match_command_to_tool(cmd)? {
        let now = chrono::Utc::now().to_rfc3339();
        db.record_usage(&tool_name, 1, Some(&now))?;
    }

    Ok(())
}

/// Detect the current shell from environment
fn detect_shell() -> String {
    // Try SHELL env var first
    if let Ok(shell) = std::env::var("SHELL") {
        if shell.contains("fish") {
            return "fish".to_string();
        } else if shell.contains("zsh") {
            return "zsh".to_string();
        } else if shell.contains("bash") {
            return "bash".to_string();
        }
    }

    // Fallback: check parent process or default to bash
    "bash".to_string()
}

/// Offer to set up shell hook automatically, or print manual instructions
fn print_hook_instructions(shell: &str) {
    // For bash, the setup is handled by offer_bash_preexec_install
    if shell == "bash" {
        return;
    }

    // For fish and zsh, offer automatic setup
    if let Err(e) = offer_shell_hook_setup(shell) {
        // If interactive setup fails (e.g., not a terminal), show manual instructions
        eprintln!("{} Could not run interactive setup: {}", "!".yellow(), e);
        print_manual_hook_instructions(shell);
    }
}

/// Offer automatic shell hook setup for fish/zsh
fn offer_shell_hook_setup(shell: &str) -> Result<()> {
    use dialoguer::Confirm;

    let home = dirs::home_dir().unwrap_or_default();

    let (config_path, hook_code) = match shell {
        "fish" => {
            let path = home.join(".config/fish/config.fish");
            let code = r#"
# Hoards usage tracking (added by hoards)
function __hoard_log --on-event fish_preexec
    command hoards usage log "$argv[1]" &>/dev/null &
    disown 2>/dev/null
end
"#;
            (path, code)
        }
        "zsh" => {
            let path = home.join(".zshrc");
            let code = r#"
# Hoards usage tracking (added by hoards)
preexec() { command hoards usage log "$1" &>/dev/null & }
"#;
            (path, code)
        }
        _ => {
            println!("{} Unsupported shell: {}", "!".yellow(), shell);
            return Ok(());
        }
    };

    // Check if hook is already installed
    let hook_installed = if config_path.exists() {
        let content = std::fs::read_to_string(&config_path).unwrap_or_default();
        content.contains("hoards usage log")
    } else {
        false
    };

    if hook_installed {
        println!();
        println!(
            "{} Hook already configured in {:?}",
            "+".green(),
            config_path
        );
        return Ok(());
    }

    println!();
    let auto_setup = Confirm::new()
        .with_prompt(format!("Add hook to {:?} automatically?", config_path))
        .default(true)
        .interact()?;

    if !auto_setup {
        print_manual_hook_instructions(shell);
        return Ok(());
    }

    // Ensure parent directory exists (for fish)
    if let Some(parent) = config_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    // Append hook to config file
    println!("{} Adding hook to {:?}...", ">".cyan(), config_path);

    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&config_path)?;

    use std::io::Write;
    file.write_all(hook_code.as_bytes())?;

    println!("{} Hook added successfully!", "+".green());
    println!();

    let source_cmd = match shell {
        "fish" => "source ~/.config/fish/config.fish".to_string(),
        _ => format!("source ~/.{}rc", shell),
    };

    println!(
        "{} Restart your shell or run: {}",
        ">".cyan(),
        source_cmd.yellow()
    );

    Ok(())
}

/// Print manual hook setup instructions
fn print_manual_hook_instructions(shell: &str) {
    println!();
    println!("{} Add this to your shell config:", ">".cyan());
    println!();

    match shell {
        "fish" => {
            println!("{}", "# Add to ~/.config/fish/config.fish".dimmed());
            println!(r#"function __hoard_log --on-event fish_preexec"#);
            println!(r#"    command hoards usage log "$argv[1]" &>/dev/null &"#);
            println!(r#"    disown 2>/dev/null"#);
            println!(r#"end"#);
        }
        "zsh" => {
            println!("{}", "# Add to ~/.zshrc".dimmed());
            println!(r#"preexec() {{"#);
            println!(r#"    command hoards usage log "$1" &>/dev/null &"#);
            println!(r#"}}"#);
        }
        "bash" => {
            println!(
                "{}",
                "# Add to ~/.bashrc (after sourcing bash-preexec)".dimmed()
            );
            println!(r#"[[ -f ~/.bash-preexec.sh ]] && source ~/.bash-preexec.sh"#);
            println!(r#"preexec() {{"#);
            println!(r#"    command hoards usage log "$1" &>/dev/null &"#);
            println!(r#"}}"#);
        }
        _ => {
            println!("{} Unsupported shell: {}", "!".yellow(), shell);
        }
    }

    println!();
    let source_cmd = match shell {
        "fish" => "source ~/.config/fish/config.fish",
        _ => &format!("source ~/.{}rc", shell),
    };
    println!(
        "{} After adding, restart your shell or run: {}",
        ">".cyan(),
        source_cmd.yellow()
    );
}

/// Show shell hook setup instructions
pub fn cmd_usage_init(
    config: &crate::config::HoardConfig,
    shell_override: Option<String>,
) -> Result<()> {
    use crate::config::UsageMode;

    let mode = config.usage.mode.as_ref();

    match mode {
        Some(UsageMode::Scan) => {
            println!("{} Usage tracking is set to 'scan' mode.", ">".cyan());
            println!(
                "  Run {} to update usage counts from shell history.",
                "hoards usage scan".yellow()
            );
            println!();
            println!(
                "  To switch to hook mode: {}",
                "hoards usage config --mode hook".yellow()
            );
        }
        Some(UsageMode::Hook) | None => {
            let shell = shell_override
                .or_else(|| config.usage.shell.clone())
                .unwrap_or_else(detect_shell);

            print_hook_instructions(&shell);
        }
    }

    Ok(())
}

/// View or change usage tracking configuration
pub fn cmd_usage_config(
    config: &mut crate::config::HoardConfig,
    mode: Option<String>,
) -> Result<()> {
    use crate::config::UsageMode;

    match mode {
        None => {
            // Show current config
            println!("{}", "Usage Tracking Configuration".bold());
            println!("{}", "-".repeat(40));

            match &config.usage.mode {
                Some(UsageMode::Scan) => {
                    println!("  Mode:  {} (manual)", "scan".cyan());
                    println!("  Info:  Run 'hoards usage scan' periodically");
                }
                Some(UsageMode::Hook) => {
                    let shell = config.usage.shell.as_deref().unwrap_or("unknown");
                    println!("  Mode:  {} (automatic)", "hook".cyan());
                    println!("  Shell: {}", shell.cyan());
                    println!("  Info:  Commands tracked in real-time via shell hook");
                }
                None => {
                    println!("  Mode:  {} (not configured)", "none".yellow());
                    println!();
                    println!(
                        "{} Run {} to set up usage tracking",
                        ">".cyan(),
                        "hoards usage config --mode <scan|hook>".yellow()
                    );
                }
            }
        }
        Some(new_mode) => {
            // Change mode (doesn't reset counters)
            let mode = match new_mode.as_str() {
                "scan" => {
                    println!("{} Switching to scan mode...", ">".cyan());
                    UsageMode::Scan
                }
                "hook" => {
                    let shell = detect_shell();
                    config.usage.shell = Some(shell.clone());

                    println!("{} Switching to hook mode...", ">".cyan());
                    println!("{} Detected shell: {}", ">".cyan(), shell.cyan());

                    // Offer bash-preexec installation for bash users
                    if shell == "bash" {
                        offer_bash_preexec_install()?;
                    }

                    print_hook_instructions(&shell);
                    UsageMode::Hook
                }
                _ => {
                    anyhow::bail!("Invalid mode '{}'. Use 'scan' or 'hook'.", new_mode);
                }
            };

            config.usage.mode = Some(mode);
            config.save()?;
            println!("{} Configuration saved.", "+".green());
        }
    }

    Ok(())
}

/// Offer to install bash-preexec and configure .bashrc for bash users
fn offer_bash_preexec_install() -> Result<()> {
    use dialoguer::Confirm;

    let home = dirs::home_dir().unwrap_or_default();
    let preexec_path = home.join(".bash-preexec.sh");
    let bashrc_path = home.join(".bashrc");

    println!();
    println!(
        "{} Bash requires {} for shell hooks.",
        "!".yellow(),
        "bash-preexec".cyan()
    );
    println!("  https://github.com/rcaloras/bash-preexec");
    println!();

    // Check if bash-preexec is already downloaded
    let preexec_exists = preexec_path.exists();

    // Check if hook is already in .bashrc
    let hook_installed = if bashrc_path.exists() {
        let content = std::fs::read_to_string(&bashrc_path).unwrap_or_default();
        content.contains("hoards usage log")
    } else {
        false
    };

    if preexec_exists && hook_installed {
        println!("{} bash-preexec and hook already configured.", "+".green());
        return Ok(());
    }

    // Offer automatic setup
    let auto_setup = Confirm::new()
        .with_prompt("Set up bash-preexec and hook automatically?")
        .default(true)
        .interact()?;

    if !auto_setup {
        println!();
        println!("{} Manual setup required:", ">".cyan());
        println!();
        println!("1. Download bash-preexec:");
        println!("   curl -o ~/.bash-preexec.sh \\");
        println!(
            "     https://raw.githubusercontent.com/rcaloras/bash-preexec/master/bash-preexec.sh"
        );
        println!();
        println!("2. Add to ~/.bashrc:");
        println!("   [[ -f ~/.bash-preexec.sh ]] && source ~/.bash-preexec.sh");
        println!("   preexec() {{ command hoards usage log \"$1\" &>/dev/null & }}");
        println!();
        return Ok(());
    }

    // Download bash-preexec.sh if needed
    if !preexec_exists {
        println!("{} Downloading bash-preexec...", ">".cyan());

        let url = "https://raw.githubusercontent.com/rcaloras/bash-preexec/master/bash-preexec.sh";
        let mut response = ureq::get(url).call()?;
        let content = response.body_mut().read_to_string()?;

        std::fs::write(&preexec_path, content)?;
        println!("{} Installed to ~/.bash-preexec.sh", "+".green());
    } else {
        println!("{} bash-preexec.sh already exists.", "+".green());
    }

    // Add hook to .bashrc if needed
    if !hook_installed {
        println!("{} Adding hook to ~/.bashrc...", ">".cyan());

        let hook_code = r#"

# Hoards usage tracking (added by hoards)
[[ -f ~/.bash-preexec.sh ]] && source ~/.bash-preexec.sh
preexec() { command hoards usage log "$1" &>/dev/null & }
"#;

        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&bashrc_path)?;

        use std::io::Write;
        file.write_all(hook_code.as_bytes())?;

        println!("{} Hook added to ~/.bashrc", "+".green());
    } else {
        println!("{} Hook already in ~/.bashrc", "+".green());
    }

    println!();
    println!(
        "{} Restart your shell or run: {}",
        ">".cyan(),
        "source ~/.bashrc".yellow()
    );

    Ok(())
}

/// Reset all usage counters to zero
pub fn cmd_usage_reset(db: &Database, force: bool) -> Result<()> {
    use dialoguer::Confirm;

    if !force {
        let confirm = Confirm::new()
            .with_prompt("Reset all usage counters to zero?")
            .default(false)
            .interact()?;

        if !confirm {
            println!("{} Cancelled.", "!".yellow());
            return Ok(());
        }
    }

    db.clear_usage()?;
    println!("{} Usage counters reset.", "+".green());
    Ok(())
}

/// Ensure usage tracking is configured (interactive setup if not)
pub fn ensure_usage_configured(config: &mut crate::config::HoardConfig) -> Result<()> {
    use crate::config::UsageMode;
    use dialoguer::Select;

    if config.usage.mode.is_some() {
        return Ok(()); // Already configured
    }

    println!("{} Usage tracking is not configured.", ">".cyan());
    println!();
    println!("How would you like to track tool usage?");
    println!();

    let items = vec![
        "History scan (manual) - Run 'hoards usage scan' periodically",
        "Shell hook (automatic) - Track commands in real-time",
    ];

    let selection = Select::new()
        .with_prompt("Select tracking mode")
        .items(&items)
        .default(0)
        .interact()?;

    let mode = if selection == 0 {
        println!();
        println!("{} Selected: history scan mode", "+".green());
        println!(
            "  Run {} to scan your shell history.",
            "hoards usage scan".yellow()
        );
        UsageMode::Scan
    } else {
        let shell = detect_shell();
        config.usage.shell = Some(shell.clone());

        println!();
        println!("{} Selected: shell hook mode", "+".green());
        println!("{} Detected shell: {}", ">".cyan(), shell.cyan());

        if shell == "bash" {
            offer_bash_preexec_install()?;
        }

        print_hook_instructions(&shell);
        UsageMode::Hook
    };

    config.usage.mode = Some(mode);
    config.save()?;
    println!("{} Configuration saved.", "+".green());

    Ok(())
}
