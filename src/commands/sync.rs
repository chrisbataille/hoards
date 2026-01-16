//! Sync commands: sync_status, scan, fetch_descriptions

use std::collections::HashSet;
use std::thread;

use anyhow::Result;
use colored::Colorize;

use crate::db::Database;
use crate::models::Tool;
use crate::scanner::{is_installed, scan_known_tools, scan_path_tools};
use crate::sources::all_sources;

use super::helpers::fetch_tool_description;

/// Sync installation status of tracked tools
pub fn cmd_sync_status(db: &Database, dry_run: bool) -> Result<()> {
    println!("{} Syncing installation status...\n", ">".cyan());

    let tools = db.list_tools(false, None)?;

    if tools.is_empty() {
        println!("No tools in database. Run 'hoards sync --scan' first.");
        return Ok(());
    }

    let mut changed = 0;

    for tool in tools {
        // Determine binary to check
        let binary = tool.binary_name.as_deref().unwrap_or(&tool.name);
        let currently_installed = is_installed(binary);

        if currently_installed != tool.is_installed {
            let status = if currently_installed {
                "installed".green()
            } else {
                "missing".red()
            };

            println!("  {} {} -> {}", "~".yellow(), tool.name, status);

            if !dry_run {
                db.set_tool_installed(&tool.name, currently_installed)?;
            }
            changed += 1;
        }
    }

    if changed == 0 {
        println!("{} Database is in sync", "+".green());
    } else if dry_run {
        println!("{} Would update {} tools", "i".cyan(), changed);
    } else {
        println!("{} Updated {} tools", "+".green(), changed);
    }

    Ok(())
}

/// Scan system for new tools
pub fn cmd_scan(db: &Database, dry_run: bool) -> Result<()> {
    println!("{} Scanning for new tools...\n", ">".cyan());

    let mut added = 0;
    let mut skipped = 0;
    let mut tracked_binaries: HashSet<String> = HashSet::new();
    let mut newly_added: Vec<Tool> = Vec::new();

    // Collect binaries already in database
    for tool in db.list_tools(false, None)? {
        if let Some(bin) = tool.binary_name {
            tracked_binaries.insert(bin);
        }
        tracked_binaries.insert(tool.name);
    }

    // Helper to process tools from any source
    let mut process_tools =
        |tools: Vec<Tool>, source_name: &str, track: bool| -> Result<Vec<Tool>> {
            if tools.is_empty() {
                return Ok(Vec::new());
            }

            println!("{} {} tools:", ">".cyan(), source_name);
            let mut added_tools = Vec::new();

            for tool in tools {
                // Track binary for PATH scan exclusion
                if track {
                    if let Some(ref bin) = tool.binary_name {
                        tracked_binaries.insert(bin.clone());
                    }
                    tracked_binaries.insert(tool.name.clone());
                }

                // Check if already in database
                if db.get_tool_by_name(&tool.name)?.is_some() {
                    skipped += 1;
                    continue;
                }

                println!(
                    "  {} {} ({})",
                    "+".green(),
                    tool.name,
                    tool.category.as_deref().unwrap_or("?")
                );

                if !dry_run {
                    db.insert_tool(&tool)?;
                }
                added += 1;

                // Track tools that need descriptions
                if tool.description.is_none() {
                    added_tools.push(tool);
                }
            }
            println!();
            Ok(added_tools)
        };

    // 1. Scan known tools (curated list with good metadata)
    newly_added.extend(process_tools(scan_known_tools(), "Known", true)?);

    // 2. Scan all package sources using the trait-based system
    for source in all_sources() {
        // Skip manual source in the main scan loop
        if source.name() == "manual" {
            continue;
        }

        match source.scan() {
            Ok(tools) => {
                let label = format!("{} ({})", source.name(), tools.len());
                newly_added.extend(process_tools(tools, &label, true)?);
            }
            Err(e) => {
                // Skip silently if source not installed (e.g., brew)
                let err_str = e.to_string();
                if !err_str.contains("No such file") && !err_str.contains("not found") {
                    eprintln!("  {} {} scan: {}", "!".yellow(), source.name(), e);
                }
            }
        }
    }

    // Scan PATH for untracked binaries (go tools, manual installs, etc.)
    match scan_path_tools(&tracked_binaries) {
        Ok(tools) if !tools.is_empty() => {
            println!("{} PATH (untracked) tools:", ">".cyan());
            for tool in tools {
                if db.get_tool_by_name(&tool.name)?.is_some() {
                    skipped += 1;
                    continue;
                }
                println!(
                    "  {} {} ({})",
                    "+".green(),
                    tool.name,
                    tool.category.as_deref().unwrap_or("?")
                );
                if !dry_run {
                    db.insert_tool(&tool)?;
                }
                added += 1;
                if tool.description.is_none() {
                    newly_added.push(tool);
                }
            }
            println!();
        }
        Ok(_) => {}
        Err(e) => eprintln!("  {} path scan: {}", "!".yellow(), e),
    }

    // Fetch descriptions in parallel for newly added tools
    if !newly_added.is_empty() && !dry_run {
        println!(
            "{} Fetching descriptions for {} tools in parallel...",
            ">".cyan(),
            newly_added.len()
        );

        let results: Vec<_> = thread::scope(|s| {
            let handles: Vec<_> = newly_added
                .iter()
                .map(|tool| {
                    s.spawn(move || {
                        let desc = fetch_tool_description(tool);
                        (tool.name.clone(), desc)
                    })
                })
                .collect();

            handles.into_iter().map(|h| h.join().unwrap()).collect()
        });

        let mut desc_updated = 0;
        for (name, result) in results {
            if let Some((desc, _source)) = result {
                db.update_tool_description(&name, &desc)?;
                desc_updated += 1;
            }
        }
        println!("  {} {} descriptions fetched\n", "+".green(), desc_updated);
    }

    // Summary
    if added == 0 && skipped == 0 {
        println!("No new tools found on system");
    } else if dry_run {
        println!(
            "{} Would add {} tools ({} already tracked)",
            "i".cyan(),
            added,
            skipped
        );
    } else {
        println!(
            "{} Added {} tools ({} already tracked)",
            "+".green(),
            added,
            skipped
        );
    }

    Ok(())
}

/// Fetch descriptions for tools missing them
pub fn cmd_fetch_descriptions(db: &Database, dry_run: bool) -> Result<()> {
    println!("{} Fetching missing descriptions...\n", ">".cyan());

    let tools = db.list_tools(false, None)?;

    // Filter tools without descriptions
    let tools_without_desc: Vec<_> = tools
        .into_iter()
        .filter(|t| t.description.is_none())
        .collect();

    if tools_without_desc.is_empty() {
        println!("{} All tools already have descriptions", "+".green());
        return Ok(());
    }

    let count = tools_without_desc.len();
    println!("  Found {} tools without descriptions", count);
    println!("  Fetching in parallel...\n");

    // Fetch descriptions in parallel using scoped threads
    let results: Vec<_> = thread::scope(|s| {
        let handles: Vec<_> = tools_without_desc
            .iter()
            .map(|tool| {
                s.spawn(move || {
                    let desc = fetch_tool_description(tool);
                    (tool.name.clone(), desc)
                })
            })
            .collect();

        handles.into_iter().map(|h| h.join().unwrap()).collect()
    });

    // Process results and update database
    let mut updated = 0;

    for (name, result) in results {
        if let Some((desc, source)) = result {
            println!(
                "  {} {} [{}]: {}",
                "+".green(),
                name,
                source.dimmed(),
                desc.chars().take(60).collect::<String>()
            );

            if !dry_run {
                db.update_tool_description(&name, &desc)?;
            }
            updated += 1;
        } else {
            println!("  {} {}: no description found", "-".dimmed(), name.dimmed());
        }
    }

    println!();
    if updated == 0 {
        println!("{} No descriptions found to update", "i".cyan());
    } else if dry_run {
        println!("{} Would update {} descriptions", "i".cyan(), updated);
    } else {
        println!("{} Updated {} descriptions", "+".green(), updated);
    }

    Ok(())
}
