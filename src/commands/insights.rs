//! Insights commands: stats, info, overview, categories

use anyhow::Result;
use colored::Colorize;

use crate::db::Database;
use crate::scanner::KNOWN_TOOLS;

/// Show statistics about tracked tools
pub fn cmd_stats(db: &Database) -> Result<()> {
    let (total, installed, favorites) = db.get_stats()?;
    let categories = db.get_categories()?;

    println!("{}", "Hoard Statistics".bold());
    println!("{}", "=".repeat(20));
    println!();
    println!("Total tools:     {}", total);
    println!("Installed:       {}", installed.to_string().green());
    println!("Missing:         {}", (total - installed).to_string().red());
    println!("Favorites:       {}", favorites.to_string().yellow());
    println!("Categories:      {}", categories.len());
    println!();
    println!("Known tools:     {}", KNOWN_TOOLS.len());

    Ok(())
}

/// Show info about hoard itself
pub fn cmd_info() -> Result<()> {
    let db_path = Database::db_path()?;

    println!("{}", "Hoard Info".bold());
    println!("{}", "=".repeat(20));
    println!();
    println!("Database: {}", db_path.display());

    if db_path.exists() {
        let metadata = std::fs::metadata(&db_path)?;
        let size = metadata.len();
        println!("Size:     {} bytes", size);
    } else {
        println!(
            "Status:   {} (will be created on first use)",
            "not created".yellow()
        );
    }

    Ok(())
}

/// Show overview dashboard
pub fn cmd_overview(db: &Database) -> Result<()> {
    println!("{}", "═══════════════════════════════════════".cyan());
    println!("{}", "         HOARD OVERVIEW DASHBOARD       ".bold());
    println!("{}", "═══════════════════════════════════════".cyan());
    println!();

    // Stats
    let (total, installed, _favorites) = db.get_stats()?;
    println!(
        "📦 {} tools tracked ({} installed, {} missing)",
        total.to_string().bold(),
        installed.to_string().green(),
        (total - installed).to_string().red()
    );
    println!();

    // Top used tools (get from usage table)
    println!("{}", "📊 Top Used Tools:".bold());
    let tools = db.list_tools(true, None)?;

    // Collect usage data for each tool
    let mut tools_with_usage: Vec<(String, i64)> = Vec::new();
    for tool in &tools {
        if let Ok(Some(usage)) = db.get_usage(&tool.name)
            && usage.use_count > 0
        {
            tools_with_usage.push((tool.name.clone(), usage.use_count));
        }
    }
    tools_with_usage.sort_by(|a, b| b.1.cmp(&a.1));

    if tools_with_usage.is_empty() {
        println!("   (no usage data - run 'hoards sync --usage')");
    } else {
        for (name, count) in tools_with_usage.iter().take(5) {
            println!("   {:20} {} uses", name, count.to_string().cyan());
        }
    }
    println!();

    // Health check
    println!("{}", "🔍 Quick Health Check:".bold());

    let db_path = Database::db_path()?;
    if db_path.exists() {
        let metadata = std::fs::metadata(&db_path)?;
        println!(
            "   Database: {} ({} KB)",
            "OK".green(),
            metadata.len() / 1024
        );
    }

    let missing_desc: usize = tools.iter().filter(|t| t.description.is_none()).count();
    if missing_desc > 0 {
        println!(
            "   {} tools missing descriptions (run 'hoards sync --descriptions')",
            missing_desc.to_string().yellow()
        );
    } else {
        println!("   Descriptions: {}", "All tools have descriptions".green());
    }

    let uncategorized: usize = tools.iter().filter(|t| t.category.is_none()).count();
    if uncategorized > 0 {
        println!(
            "   {} tools uncategorized (run 'hoards ai enrich --categorize')",
            uncategorized.to_string().yellow()
        );
    } else {
        println!("   Categories: {}", "All tools categorized".green());
    }

    println!();

    Ok(())
}

/// Show all categories with counts
pub fn cmd_categories(db: &Database) -> Result<()> {
    let category_counts = db.get_category_counts()?;

    if category_counts.is_empty() {
        println!("No categories found. Add some tools first.");
        return Ok(());
    }

    println!("{}", "Categories".bold());
    println!();
    for (cat, count) in category_counts {
        println!("  {} ({})", cat, count);
    }

    Ok(())
}
