//! Usage tracking command implementations
//!
//! Commands for tracking and analyzing tool usage from shell history.

use anyhow::Result;
use colored::Colorize;

use crate::Database;

/// Show all labels
pub fn cmd_labels(db: &Database) -> Result<()> {
    let label_counts = db.get_label_counts()?;

    if label_counts.is_empty() {
        println!("{} No labels found", "!".yellow());
        println!("  Sync with GitHub: {}", "hoard gh sync".cyan());
        return Ok(());
    }

    println!("{}", "Labels:".bold());
    for (label, count) in &label_counts {
        println!(
            "  {} {} ({})",
            "*".cyan(),
            label,
            format!("{} tools", count).dimmed()
        );
    }

    println!();
    println!(
        "{} List tools by label: {}",
        ">".cyan(),
        "hoard list --label <label>".yellow()
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
    let usage = db.get_all_usage()?;

    if usage.is_empty() {
        println!(
            "{} No usage data yet. Run {} first.",
            "!".yellow(),
            "hoard usage scan".cyan()
        );
        return Ok(());
    }

    println!("{}", "Tool Usage Statistics".bold());
    println!("{}", "-".repeat(50));

    let total: i64 = usage.iter().map(|(_, u)| u.use_count).sum();

    println!(
        "{:20} {:>10} {:>10}",
        "TOOL".bold(),
        "USES".bold(),
        "PERCENT".bold()
    );
    println!("{}", "-".repeat(50));

    for (name, stats) in usage.iter().take(limit) {
        let percent = (stats.use_count as f64 / total as f64) * 100.0;
        println!(
            "{:20} {:>10} {:>9.1}%",
            name.cyan(),
            stats.use_count,
            percent
        );
    }

    if usage.len() > limit {
        println!(
            "\n{} Showing top {} of {} tools. Use {} to see more.",
            ">".cyan(),
            limit,
            usage.len(),
            "--limit".yellow()
        );
    }

    println!("\n{} Total tracked uses: {}", ">".cyan(), total);

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
            println!("  Run {} to scan shell history", "hoard usage scan".cyan());
        }
    }

    Ok(())
}

/// Show unused tools
pub fn cmd_unused(db: &Database) -> Result<()> {
    let unused = db.get_unused_tools()?;

    if unused.is_empty() {
        println!("{} All installed tools have been used!", "+".green());
        println!(
            "  Run {} first if you haven't already",
            "hoard usage scan".cyan()
        );
        return Ok(());
    }

    println!("{}", "Installed tools with no recorded usage:".bold());
    println!("{}", "-".repeat(60));

    for tool in &unused {
        let desc = tool.description.as_deref().unwrap_or("-");
        let desc_short: String = desc.chars().take(40).collect();
        println!(
            "  {} {:20} {}",
            "-".red(),
            tool.name.cyan(),
            desc_short.dimmed()
        );
    }

    println!();
    println!(
        "{} Found {} unused tool{}",
        "!".yellow(),
        unused.len(),
        if unused.len() == 1 { "" } else { "s" }
    );
    println!(
        "  Consider uninstalling with: {}",
        "hoard uninstall <tool>".cyan()
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
            "hoard usage scan".cyan()
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
        "hoard install <tool>".yellow()
    );

    Ok(())
}
