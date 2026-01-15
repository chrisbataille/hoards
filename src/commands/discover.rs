//! Discovery commands: suggest, similar, trending

use std::collections::HashMap;

use anyhow::Result;
use colored::Colorize;

use crate::db::Database;
use crate::models::Tool;
use crate::scanner::scan_missing_tools;

/// Suggest tools the user might want
pub fn cmd_suggest(category: Option<String>) -> Result<()> {
    println!("{} Tools you might want to try:\n", ">".cyan());

    let missing = scan_missing_tools();

    let filtered: Vec<_> = if let Some(ref cat) = category {
        missing
            .into_iter()
            .filter(|t| t.category.as_deref() == Some(cat.as_str()))
            .collect()
    } else {
        missing
    };

    if filtered.is_empty() {
        println!("No suggestions available");
        return Ok(());
    }

    // Group by category
    let mut by_category: HashMap<&str, Vec<&Tool>> = HashMap::new();
    for tool in &filtered {
        let cat = tool.category.as_deref().unwrap_or("other");
        by_category.entry(cat).or_default().push(tool);
    }

    for (cat, tools) in by_category.iter() {
        println!("{}:", cat.bold());
        for tool in tools {
            println!(
                "  {} - {}",
                tool.name.cyan(),
                tool.description.as_deref().unwrap_or("")
            );
            if let Some(cmd) = &tool.install_command {
                println!("    {}", cmd.dimmed());
            }
        }
        println!();
    }

    Ok(())
}

/// Find tools similar to a given tool
pub fn cmd_similar(db: &Database, tool_name: &str) -> Result<()> {
    let tool = match db.get_tool_by_name(tool_name)? {
        Some(t) => t,
        None => {
            println!("Tool '{}' not found", tool_name);
            return Ok(());
        }
    };

    println!("{} Tools similar to '{}':\n", ">".cyan(), tool_name.bold());

    // Find tools in the same category
    let mut similar: Vec<Tool> = Vec::new();

    if let Some(ref cat) = tool.category {
        let same_category = db.list_tools(false, Some(cat))?;
        for t in same_category {
            if t.name != tool_name {
                similar.push(t);
            }
        }
    }

    if similar.is_empty() {
        println!("No similar tools found");
        return Ok(());
    }

    // Sort alphabetically
    similar.sort_by(|a, b| a.name.cmp(&b.name));

    for t in similar.iter().take(10) {
        let status = if t.is_installed {
            "installed".green()
        } else {
            "not installed".dimmed()
        };

        println!("  {} {} [{}]", t.name.bold(), status, t.source);
        if let Some(desc) = &t.description {
            println!("    {}", desc.dimmed());
        }
    }

    Ok(())
}

/// Show trending tools by GitHub stars
pub fn cmd_trending(db: &Database, category: Option<String>, limit: usize) -> Result<()> {
    println!("{} Trending tools by GitHub stars:\n", ">".cyan());

    let tools = db.list_tools(false, category.as_deref())?;

    // Collect tools with their GitHub star counts
    let mut tools_with_stars: Vec<(Tool, i64)> = Vec::new();
    for tool in tools {
        if let Ok(Some(gh_info)) = db.get_github_info(&tool.name) {
            tools_with_stars.push((tool, gh_info.stars));
        }
    }

    // Sort by stars descending
    tools_with_stars.sort_by(|a, b| b.1.cmp(&a.1));

    if tools_with_stars.is_empty() {
        println!("No tools with GitHub star data found.");
        println!("Run 'hoards sync --github' to fetch star counts.");
        return Ok(());
    }

    for (tool, stars) in tools_with_stars.iter().take(limit) {
        let status = if tool.is_installed {
            "✓".green()
        } else {
            " ".normal()
        };

        println!(
            "  {} {:>6} ★  {}  [{}]",
            status,
            stars.to_string().yellow(),
            tool.name.bold(),
            tool.category.as_deref().unwrap_or("-")
        );
    }

    Ok(())
}
