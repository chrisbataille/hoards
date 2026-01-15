//! Bundle command implementations
//!
//! Bundles are collections of tools that can be installed together.

use anyhow::Result;
use colored::Colorize;

use crate::{
    Bundle, Database, InstallSource, SafeCommand, get_safe_install_command,
    get_safe_uninstall_command, is_installed,
};

/// Create a new bundle
pub fn cmd_bundle_create(
    db: &Database,
    name: &str,
    tools: Vec<String>,
    description: Option<String>,
) -> Result<()> {
    // Check if bundle already exists
    if db.get_bundle(name)?.is_some() {
        println!("{} Bundle '{}' already exists", "!".yellow(), name);
        println!(
            "  Use {} to add tools",
            format!("hoards bundle add {} <tools>", name).cyan()
        );
        return Ok(());
    }

    let mut bundle = Bundle::new(name, tools.clone());
    if let Some(desc) = description {
        bundle = bundle.with_description(desc);
    }

    db.create_bundle(&bundle)?;

    println!("{} Created bundle '{}'", "+".green(), name.bold());
    println!("  Tools: {}", tools.join(", "));

    Ok(())
}

/// List all bundles
pub fn cmd_bundle_list(db: &Database) -> Result<()> {
    use comfy_table::{
        Cell, Color, ContentArrangement, Table, modifiers::UTF8_ROUND_CORNERS, presets::UTF8_FULL,
    };

    let bundles = db.list_bundles()?;

    if bundles.is_empty() {
        println!("No bundles found. Create one with: hoard bundle create <name> <tools...>");
        return Ok(());
    }

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
            Cell::new("📦 Bundle").fg(Color::Cyan),
            Cell::new("#").fg(Color::Cyan),
            Cell::new("Description").fg(Color::Cyan),
        ]);

    for bundle in &bundles {
        let desc = bundle.description.as_deref().unwrap_or("-");

        table.add_row(vec![
            Cell::new(&bundle.name),
            Cell::new(bundle.tools.len()),
            Cell::new(desc),
        ]);
    }

    println!("{table}");
    println!("{} {} bundles", ">".cyan(), bundles.len());
    Ok(())
}

/// Show details of a specific bundle
pub fn cmd_bundle_show(db: &Database, name: &str) -> Result<()> {
    use crate::icons::{source_icon, status_icon};
    use comfy_table::{
        Cell, Color, ContentArrangement, Table, modifiers::UTF8_ROUND_CORNERS, presets::UTF8_FULL,
    };

    let bundle = match db.get_bundle(name)? {
        Some(b) => b,
        None => {
            println!("Bundle '{}' not found", name);
            return Ok(());
        }
    };

    println!("{} {}", "📦 Bundle:".bold(), bundle.name.cyan());
    if let Some(desc) = &bundle.description {
        println!("{}", desc.dimmed());
    }
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
            Cell::new("✓").fg(Color::Cyan),
            Cell::new("Description").fg(Color::Cyan),
        ]);

    let mut installed_count = 0;
    for tool_name in &bundle.tools {
        if let Some(tool) = db.get_tool_by_name(tool_name)? {
            let src_icon = source_icon(&tool.source.to_string());
            let (status, color) = if tool.is_installed {
                installed_count += 1;
                (status_icon(true), Color::Green)
            } else {
                (status_icon(false), Color::Red)
            };
            let desc = tool.description.as_deref().unwrap_or("-");
            table.add_row(vec![
                Cell::new(tool_name),
                Cell::new(src_icon),
                Cell::new(status).fg(color),
                Cell::new(desc),
            ]);
        } else {
            table.add_row(vec![
                Cell::new(tool_name),
                Cell::new("?"),
                Cell::new("⚠").fg(Color::Yellow),
                Cell::new("not in database"),
            ]);
        }
    }

    println!("{table}");
    crate::icons::print_legend_compact();
    println!(
        "{} {}/{} installed",
        ">".cyan(),
        installed_count,
        bundle.tools.len()
    );
    Ok(())
}

/// Install all tools in a bundle
pub fn cmd_bundle_install(db: &Database, name: &str, force: bool) -> Result<()> {
    let bundle = match db.get_bundle(name)? {
        Some(b) => b,
        None => {
            println!("Bundle '{}' not found", name);
            return Ok(());
        }
    };

    if bundle.tools.is_empty() {
        println!("Bundle '{}' has no tools", name);
        return Ok(());
    }

    // Build install plan
    println!(
        "{} Install plan for bundle '{}':\n",
        ">".cyan(),
        name.bold()
    );

    let mut to_install: Vec<(&str, String, SafeCommand)> = Vec::new(); // (name, source, command)
    let mut already_installed = 0;
    let mut unknown_source = 0;

    for tool_name in &bundle.tools {
        // Get tool info from database first
        let tool_info = db.get_tool_by_name(tool_name)?;

        // Check if installed using binary_name if available
        let binary = tool_info
            .as_ref()
            .and_then(|t| t.binary_name.as_deref())
            .unwrap_or(tool_name);

        if is_installed(binary) {
            println!(
                "  {} {} (already installed)",
                "-".dimmed(),
                tool_name.dimmed()
            );
            already_installed += 1;
            continue;
        }

        // Get source from database or skip
        let source = if let Some(ref tool) = tool_info {
            tool.source.to_string()
        } else {
            println!(
                "  {} {} (not in database, skipping)",
                "?".yellow(),
                tool_name
            );
            unknown_source += 1;
            continue;
        };

        // Get safe install command (validates package name)
        match get_safe_install_command(tool_name, &source, None) {
            Ok(Some(cmd)) => {
                println!("  {} {} ({})", "+".green(), tool_name, source.cyan());
                to_install.push((tool_name, source, cmd));
            }
            Ok(None) => {
                println!(
                    "  {} {} (unknown source: {})",
                    "?".yellow(),
                    tool_name,
                    source
                );
                unknown_source += 1;
            }
            Err(e) => {
                println!("  {} {} (invalid name: {})", "!".red(), tool_name, e);
                unknown_source += 1;
            }
        }
    }

    if to_install.is_empty() {
        println!("\nNothing to install.");
        if already_installed > 0 {
            println!("  {} tool(s) already installed", already_installed);
        }
        return Ok(());
    }

    println!(
        "\n  {} to install, {} already installed, {} unknown",
        to_install.len().to_string().green(),
        already_installed,
        unknown_source
    );

    // Confirm
    if !force {
        println!();
        print!("Proceed? [y/N] ");
        std::io::Write::flush(&mut std::io::stdout())?;

        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;

        if !input.trim().eq_ignore_ascii_case("y") {
            println!("Cancelled");
            return Ok(());
        }
    }

    println!();

    // Execute installs (safe: no shell interpolation)
    let mut success = 0;
    let mut failed = 0;

    for (tool_name, source, cmd) in &to_install {
        println!(
            "{} Installing {} from {}...",
            ">".cyan(),
            tool_name.bold(),
            source
        );

        let status = cmd.execute()?;

        if status.success() {
            db.set_tool_installed(tool_name, true)?;
            println!("{} Installed {}", "+".green(), tool_name);
            success += 1;
        } else {
            println!("{} Failed to install {}", "!".red(), tool_name);
            failed += 1;
        }
    }

    println!();
    println!(
        "{} Bundle '{}': {} installed, {} failed, {} skipped",
        if failed == 0 {
            "+".green()
        } else {
            "!".yellow()
        },
        name,
        success.to_string().green(),
        failed.to_string().red(),
        (already_installed + unknown_source).to_string().dimmed()
    );

    Ok(())
}

/// Add tools to an existing bundle
pub fn cmd_bundle_add(db: &Database, name: &str, tools: Vec<String>) -> Result<()> {
    if !db.add_to_bundle(name, &tools)? {
        println!("Bundle '{}' not found", name);
        return Ok(());
    }

    println!("{} Added to bundle '{}':", "+".green(), name);
    for tool in &tools {
        println!("  + {}", tool);
    }

    Ok(())
}

/// Remove tools from a bundle
pub fn cmd_bundle_remove(db: &Database, name: &str, tools: Vec<String>) -> Result<()> {
    if !db.remove_from_bundle(name, &tools)? {
        println!("Bundle '{}' not found", name);
        return Ok(());
    }

    println!("{} Removed from bundle '{}':", "-".red(), name);
    for tool in &tools {
        println!("  - {}", tool);
    }

    Ok(())
}

/// Delete a bundle
pub fn cmd_bundle_delete(db: &Database, name: &str, force: bool) -> Result<()> {
    // Check bundle exists
    let bundle = match db.get_bundle(name)? {
        Some(b) => b,
        None => {
            println!("Bundle '{}' not found", name);
            return Ok(());
        }
    };

    // Confirm
    if !force {
        print!(
            "Delete bundle '{}' ({} tools)? [y/N] ",
            name,
            bundle.tools.len()
        );
        std::io::Write::flush(&mut std::io::stdout())?;

        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;

        if !input.trim().eq_ignore_ascii_case("y") {
            println!("Cancelled");
            return Ok(());
        }
    }

    db.delete_bundle(name)?;
    println!("{} Deleted bundle '{}'", "-".red(), name);

    Ok(())
}

/// Check for updates in bundle tools
pub fn cmd_bundle_update(db: &Database, name: &str, auto_yes: bool) -> Result<()> {
    use crate::updates::*;

    let bundle = match db.get_bundle(name)? {
        Some(b) => b,
        None => {
            println!("Bundle '{}' not found", name);
            return Ok(());
        }
    };

    if bundle.tools.is_empty() {
        println!("Bundle '{}' has no tools", name);
        return Ok(());
    }

    println!(
        "{} Checking updates for bundle '{}'...\n",
        ">".cyan(),
        name.bold()
    );

    // Collect tools with available updates
    struct ToolUpdate {
        name: String,
        source: String,
        current: String,
        latest: String,
        all_versions: Vec<String>,
    }

    let mut updates: Vec<ToolUpdate> = Vec::new();
    let mut up_to_date = 0;
    let mut not_installed = 0;
    let mut unknown = 0;

    for tool_name in &bundle.tools {
        // Get tool info from database
        let tool = match db.get_tool_by_name(tool_name)? {
            Some(t) => t,
            None => {
                unknown += 1;
                continue;
            }
        };

        // Check if installed
        let binary = tool.binary_name.as_deref().unwrap_or(tool_name);
        if !is_installed(binary) {
            not_installed += 1;
            continue;
        }

        let source = tool.source.to_string();

        // Get current version
        let current = match get_installed_version(tool_name, &source) {
            Some(v) => v,
            None => continue,
        };

        // Get available versions
        let all_versions = get_available_versions(tool_name, &source, &current);

        if all_versions.is_empty() {
            up_to_date += 1;
            continue;
        }

        let latest = all_versions.last().cloned().unwrap_or_default();

        updates.push(ToolUpdate {
            name: tool_name.clone(),
            source,
            current,
            latest,
            all_versions,
        });
    }

    if updates.is_empty() {
        println!("{} All tools are up to date!", "+".green());
        println!(
            "  {} up to date, {} not installed, {} unknown",
            up_to_date, not_installed, unknown
        );
        return Ok(());
    }

    println!(
        "Found {} tool(s) with updates ({} up to date, {} not installed, {} unknown)\n",
        updates.len().to_string().yellow(),
        up_to_date,
        not_installed,
        unknown
    );

    // Process each tool
    let mut updated = 0;
    let mut skipped = 0;

    for tool_update in &updates {
        println!(
            "{} {} ({}) {} -> {}",
            ">".cyan(),
            tool_update.name.bold(),
            tool_update.source.cyan(),
            tool_update.current.dimmed(),
            tool_update.latest.green()
        );

        if tool_update.all_versions.len() > 1 {
            println!(
                "  Available: {}",
                tool_update.all_versions.join(", ").dimmed()
            );
        }

        // Get user choice
        let choice = if auto_yes {
            'u'
        } else {
            print!("  [U]pdate to latest, [V]ersion, [S]witch source, [N]o skip? ");
            std::io::Write::flush(&mut std::io::stdout())?;

            let mut input = String::new();
            std::io::stdin().read_line(&mut input)?;
            input.trim().to_lowercase().chars().next().unwrap_or('n')
        };

        match choice {
            'u' => {
                // Update to latest (safe: validates input)
                let cmd = match get_safe_install_command(
                    &tool_update.name,
                    &tool_update.source,
                    Some(&tool_update.latest),
                ) {
                    Ok(Some(c)) => c,
                    Ok(None) => {
                        println!("  {} Don't know how to update", "!".red());
                        skipped += 1;
                        continue;
                    }
                    Err(e) => {
                        println!("  {} Invalid input: {}", "!".red(), e);
                        skipped += 1;
                        continue;
                    }
                };

                println!("  {} {}", ">".cyan(), cmd.to_string().dimmed());
                let status = cmd.execute()?;

                if status.success() {
                    println!("  {} Updated to {}", "+".green(), tool_update.latest);
                    updated += 1;
                } else {
                    println!("  {} Update failed", "!".red());
                    skipped += 1;
                }
            }
            'v' => {
                // Pick specific version
                print!("  Enter version: ");
                std::io::Write::flush(&mut std::io::stdout())?;

                let mut version = String::new();
                std::io::stdin().read_line(&mut version)?;
                let version = version.trim();

                if version.is_empty() {
                    println!("  Skipped");
                    skipped += 1;
                    continue;
                }

                // Validate and get safe command
                let cmd = match get_safe_install_command(
                    &tool_update.name,
                    &tool_update.source,
                    Some(version),
                ) {
                    Ok(Some(c)) => c,
                    Ok(None) => {
                        println!("  {} Don't know how to install version", "!".red());
                        skipped += 1;
                        continue;
                    }
                    Err(e) => {
                        println!("  {} Invalid input: {}", "!".red(), e);
                        skipped += 1;
                        continue;
                    }
                };

                println!("  {} {}", ">".cyan(), cmd.to_string().dimmed());
                let status = cmd.execute()?;

                if status.success() {
                    println!("  {} Installed version {}", "+".green(), version);
                    updated += 1;
                } else {
                    println!("  {} Install failed", "!".red());
                    skipped += 1;
                }
            }
            's' => {
                // Switch source
                print!("  Switch to source (cargo/pip/npm/apt/brew/snap): ");
                std::io::Write::flush(&mut std::io::stdout())?;

                let mut new_source = String::new();
                std::io::stdin().read_line(&mut new_source)?;
                let new_source = new_source.trim();

                if new_source.is_empty() {
                    println!("  Skipped");
                    skipped += 1;
                    continue;
                }

                // Uninstall from old source (safe: validates input)
                match get_safe_uninstall_command(&tool_update.name, &tool_update.source) {
                    Ok(Some(uninstall_cmd)) => {
                        println!(
                            "  {} Uninstalling from {}...",
                            ">".cyan(),
                            tool_update.source
                        );
                        let status = uninstall_cmd.execute()?;
                        if !status.success() {
                            println!("  {} Uninstall failed, skipping", "!".red());
                            skipped += 1;
                            continue;
                        }
                    }
                    Ok(None) => {
                        println!(
                            "  {} Don't know how to uninstall from {}",
                            "!".red(),
                            tool_update.source
                        );
                        skipped += 1;
                        continue;
                    }
                    Err(e) => {
                        println!("  {} Invalid input: {}", "!".red(), e);
                        skipped += 1;
                        continue;
                    }
                }

                // Install from new source (safe: validates input)
                let install_cmd =
                    match get_safe_install_command(&tool_update.name, new_source, None) {
                        Ok(Some(c)) => c,
                        Ok(None) => {
                            println!(
                                "  {} Don't know how to install from {}",
                                "!".red(),
                                new_source
                            );
                            skipped += 1;
                            continue;
                        }
                        Err(e) => {
                            println!("  {} Invalid input: {}", "!".red(), e);
                            skipped += 1;
                            continue;
                        }
                    };

                println!("  {} Installing from {}...", ">".cyan(), new_source);
                let status = install_cmd.execute()?;

                if status.success() {
                    // Update database
                    if let Some(mut tool) = db.get_tool_by_name(&tool_update.name)? {
                        tool.source = InstallSource::from(new_source);
                        tool.install_command = Some(install_cmd.to_string());
                        db.update_tool(&tool)?;
                    }
                    println!(
                        "  {} Switched {} -> {}",
                        "+".green(),
                        tool_update.source,
                        new_source
                    );
                    updated += 1;
                } else {
                    println!("  {} Install failed", "!".red());
                    skipped += 1;
                }
            }
            _ => {
                println!("  Skipped");
                skipped += 1;
            }
        }
        println!();
    }

    println!(
        "{} Bundle '{}': {} updated, {} skipped",
        if updated > 0 { "+".green() } else { "i".cyan() },
        name,
        updated.to_string().green(),
        skipped.to_string().dimmed()
    );

    Ok(())
}
