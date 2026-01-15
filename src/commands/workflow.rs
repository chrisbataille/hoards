//! Workflow commands: init, maintain, cleanup

use anyhow::Result;
use colored::Colorize;

use crate::db::Database;

use super::completions::cmd_completions_install;
use super::github::cmd_gh_sync;
use super::helpers::confirm;
use super::misc::cmd_doctor;
use super::sync::{cmd_fetch_descriptions, cmd_scan, cmd_sync_status};
use super::updates_cmd::cmd_updates;
use super::usage::cmd_usage_scan;

/// Run AI categorization if available
fn try_ai_categorize() {
    // Import dynamically to avoid circular dependency
    if let Err(e) = super::ai::cmd_ai_categorize(false) {
        println!("  {} AI categorization failed: {}", "!".yellow(), e);
    }
}

/// First-time setup wizard
pub fn cmd_init(db: &Database, auto: bool) -> Result<()> {
    println!("{}", "═══════════════════════════════════════".cyan());
    println!("{}", "        HOARD FIRST-TIME SETUP          ".bold());
    println!("{}", "═══════════════════════════════════════".cyan());
    println!();

    // Step 1: Scan for tools
    println!("{} Scanning system for installed tools...", "1.".bold());
    cmd_scan(db, false)?;

    // Step 2: Sync status
    println!("\n{} Syncing installation status...", "2.".bold());
    cmd_sync_status(db, false)?;

    // Step 3: Fetch descriptions
    println!("\n{} Fetching descriptions from registries...", "3.".bold());
    cmd_fetch_descriptions(db, false)?;

    // Step 4: Install shell completions
    println!("\n{} Installing shell completions...", "4.".bold());
    if let Err(e) = cmd_completions_install(None, false) {
        println!("  {} Failed to install completions: {}", "!".yellow(), e);
    }

    if !auto {
        // Step 5: Optional GitHub sync
        print!("\n{} Sync GitHub data (stars, topics)? [y/N] ", "5.".bold());
        std::io::Write::flush(&mut std::io::stdout())?;

        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;

        if input.trim().eq_ignore_ascii_case("y") {
            println!();
            cmd_gh_sync(db, false, None, 2000)?;
        }

        // Step 6: Optional AI categorization
        print!("\n{} Auto-categorize tools with AI? [y/N] ", "6.".bold());
        std::io::Write::flush(&mut std::io::stdout())?;

        input.clear();
        std::io::stdin().read_line(&mut input)?;

        if input.trim().eq_ignore_ascii_case("y") {
            println!();
            try_ai_categorize();
        }
    }

    println!();
    println!("{}", "═══════════════════════════════════════".green());
    println!("{}", "        SETUP COMPLETE!                 ".bold());
    println!("{}", "═══════════════════════════════════════".green());
    println!();
    println!("Next steps:");
    println!("  {} - see overview", "hoards insights overview".cyan());
    println!(
        "  {} - find tools you're missing",
        "hoards discover missing".cyan()
    );
    println!("  {} - keep things up to date", "hoards maintain".cyan());

    Ok(())
}

/// Periodic maintenance routine
pub fn cmd_maintain(db: &Database, auto: bool, dry_run: bool) -> Result<()> {
    println!("{}", "═══════════════════════════════════════".cyan());
    println!("{}", "        HOARD MAINTENANCE               ".bold());
    println!("{}", "═══════════════════════════════════════".cyan());
    println!();

    // Step 1: Sync status
    println!("{} Syncing installation status...", "1.".bold());
    cmd_sync_status(db, dry_run)?;

    // Step 2: Check for updates
    println!("\n{} Checking for updates...", "2.".bold());
    cmd_updates(db, None, false, true, false)?;

    // Step 3: Scan usage
    println!("\n{} Scanning shell history for usage...", "3.".bold());
    cmd_usage_scan(db, dry_run, false)?;

    // Step 4: Health check
    println!("\n{} Running health check...", "4.".bold());
    cmd_doctor(db, false)?;

    if !auto && !dry_run {
        println!();
        println!("{} Maintenance complete!", "+".green());
    } else if dry_run {
        println!();
        println!("{} Dry run complete - no changes made", "i".cyan());
    }

    Ok(())
}

/// Cleanup wizard for removing unused tools
pub fn cmd_cleanup(db: &Database, force: bool, dry_run: bool) -> Result<()> {
    println!("{}", "═══════════════════════════════════════".cyan());
    println!("{}", "        HOARD CLEANUP WIZARD            ".bold());
    println!("{}", "═══════════════════════════════════════".cyan());
    println!();

    // Step 1: Show unused tools
    println!("{} Unused installed tools:", "1.".bold());
    let unused = db.get_unused_tools()?;

    if unused.is_empty() {
        println!("   {} No unused tools found", "+".green());
    } else {
        println!(
            "   Found {} installed tools with no recorded usage:\n",
            unused.len()
        );
        for tool in &unused {
            println!("   {} {} ({})", "-".yellow(), tool.name, tool.source);
        }
    }

    // Step 2: Check for orphaned entries (not installed, not in usage table)
    println!(
        "\n{} Checking for orphaned database entries...",
        "2.".bold()
    );
    let all_tools = db.list_tools(false, None)?;
    let orphaned: Vec<_> = all_tools
        .iter()
        .filter(|t| !t.is_installed)
        .filter(|t| db.get_usage(&t.name).ok().flatten().is_none())
        .collect();

    if orphaned.is_empty() {
        println!("   {} No orphaned entries found", "+".green());
    } else {
        println!(
            "   Found {} tools not installed with no usage:\n",
            orphaned.len()
        );
        for tool in orphaned.iter().take(10) {
            println!("   {} {}", "-".dimmed(), tool.name.dimmed());
        }
        if orphaned.len() > 10 {
            println!("   ... and {} more", orphaned.len() - 10);
        }

        if !dry_run && (force || confirm("Remove orphaned entries?")?) {
            for tool in &orphaned {
                db.delete_tool(&tool.name)?;
            }
            println!(
                "   {} Removed {} orphaned entries",
                "+".green(),
                orphaned.len()
            );
        }
    }

    // Step 3: Run health fix
    println!("\n{} Running health checks...", "3.".bold());
    cmd_doctor(db, !dry_run && force)?;

    println!();
    if dry_run {
        println!("{} Dry run complete - no changes made", "i".cyan());
    } else {
        println!("{} Cleanup complete!", "+".green());
    }

    Ok(())
}
