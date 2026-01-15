//! Updates commands: updates, updates_tracked, updates_cross

use anyhow::Result;
use colored::Colorize;

use crate::db::Database;
use crate::updates::*;

/// Check for available updates
pub fn cmd_updates(
    db: &Database,
    source_filter: Option<String>,
    cross: bool,
    tracked: bool,
    all_versions: bool,
) -> Result<()> {
    if cross {
        return cmd_updates_cross(db);
    }

    // If --tracked or --all-versions, use the tracked tools mode
    if tracked || all_versions {
        return cmd_updates_tracked(db, source_filter, all_versions);
    }

    println!("{} Checking for updates...\n", ">".cyan());

    let mut total_updates = 0;

    let check_source = |name: &str, check_fn: fn() -> Result<Vec<Update>>| -> Result<usize> {
        print!("  {} {}... ", ">".cyan(), name);
        std::io::Write::flush(&mut std::io::stdout())?;

        match check_fn() {
            Ok(updates) if updates.is_empty() => {
                println!("{}", "up to date".green());
                Ok(0)
            }
            Ok(updates) => {
                println!("{} available", updates.len().to_string().yellow());
                for update in &updates {
                    println!(
                        "    {} {} -> {}",
                        update.name.bold(),
                        update.current.dimmed(),
                        update.latest.green()
                    );
                }
                Ok(updates.len())
            }
            Err(e) => {
                println!(
                    "{} ({})",
                    "skipped".dimmed(),
                    e.to_string().chars().take(30).collect::<String>()
                );
                Ok(0)
            }
        }
    };

    #[allow(clippy::type_complexity)]
    let sources: Vec<(&str, fn() -> Result<Vec<Update>>)> = vec![
        ("cargo", check_cargo_updates),
        ("pip", check_pip_updates),
        ("npm", check_npm_updates),
        ("apt", check_apt_updates),
        ("brew", check_brew_updates),
    ];

    for (name, check_fn) in sources {
        if let Some(ref filter) = source_filter
            && filter != name
        {
            continue;
        }
        total_updates += check_source(name, check_fn)?;
    }

    println!();
    if total_updates == 0 {
        println!("{} All tools are up to date!", "+".green());
    } else {
        println!("{} {} update(s) available", "!".yellow(), total_updates);
    }

    Ok(())
}

/// Check tracked tools for updates
pub fn cmd_updates_tracked(
    db: &Database,
    source_filter: Option<String>,
    all_versions: bool,
) -> Result<()> {
    println!(
        "{} Checking tracked tools for updates{}...\n",
        ">".cyan(),
        if all_versions { " (all versions)" } else { "" }
    );

    // Get all installed tools from database
    let tools = db.list_tools(true, None)?;

    // Filter by source if specified
    let tools: Vec<_> = tools
        .into_iter()
        .filter(|t| {
            if let Some(ref filter) = source_filter {
                t.source.to_string() == *filter
            } else {
                // Only check sources we can query (cargo, pip, npm)
                matches!(t.source.to_string().as_str(), "cargo" | "pip" | "npm")
            }
        })
        .collect();

    if tools.is_empty() {
        println!("No tracked tools found for the specified source(s).");
        println!("  Note: Only cargo, pip, and npm tools can be checked for updates.");
        return Ok(());
    }

    let mut updates_found = 0;

    for tool in &tools {
        let source = tool.source.to_string();

        // Get current installed version
        let current = match get_installed_version(&tool.name, &source) {
            Some(v) => v,
            None => continue,
        };

        if all_versions {
            // Get all newer versions
            let versions = get_available_versions(&tool.name, &source, &current);
            if !versions.is_empty() {
                updates_found += 1;
                println!(
                    "  {} ({}) {} -> ",
                    tool.name.bold(),
                    source.cyan(),
                    current.dimmed()
                );
                for (i, ver) in versions.iter().enumerate() {
                    let marker = if i == versions.len() - 1 {
                        "(latest)"
                    } else {
                        ""
                    };
                    println!("    {} {}", ver.green(), marker.dimmed());
                }
            }
        } else {
            // Just check for latest
            let latest = match &source[..] {
                "cargo" => get_crates_io_latest(&tool.name),
                "pip" => get_pypi_latest(&tool.name),
                "npm" => get_npm_latest(&tool.name),
                _ => None,
            };

            if let Some(latest) = latest
                && version_is_newer(&latest, &current)
            {
                updates_found += 1;
                println!(
                    "  {} ({}) {} -> {}",
                    tool.name.bold(),
                    source.cyan(),
                    current.dimmed(),
                    latest.green()
                );
            }
        }
    }

    println!();
    if updates_found == 0 {
        println!("{} All tracked tools are up to date!", "+".green());
    } else {
        println!(
            "{} {} tool(s) have updates available",
            "!".yellow(),
            updates_found
        );
        if all_versions {
            println!(
                "  Use {} to install a specific version",
                "hoards upgrade <tool> --version <ver>".cyan()
            );
        }
    }

    Ok(())
}

/// Check for cross-source upgrade opportunities
pub fn cmd_updates_cross(db: &Database) -> Result<()> {
    println!(
        "{} Checking apt/snap tools for newer versions on other sources...\n",
        ">".cyan()
    );

    // Get all apt/snap tools from database with their versions
    let tools = db.list_tools(true, None)?;
    let apt_snap_tools: Vec<(String, String, String)> = tools
        .into_iter()
        .filter(|t| {
            let source = t.source.to_string();
            source == "apt" || source == "snap"
        })
        .filter_map(|t| {
            // Get current installed version
            let version = get_apt_version(&t.name)?;
            Some((t.name, version, t.source.to_string()))
        })
        .collect();

    if apt_snap_tools.is_empty() {
        println!("No apt/snap tools found in database.");
        return Ok(());
    }

    println!("  Checking {} apt/snap tools...\n", apt_snap_tools.len());

    let upgrades = check_cross_source_upgrades(&apt_snap_tools);

    if upgrades.is_empty() {
        println!("{} No cross-source upgrades found.", "+".green());
        println!("  All apt/snap tools are either up-to-date or not available on cargo/pip/npm.");
    } else {
        println!(
            "{} {} tool(s) have newer versions on other sources:\n",
            "!".yellow(),
            upgrades.len()
        );

        for upgrade in &upgrades {
            println!(
                "  {} ({} {}) -> {} {} {}",
                upgrade.name.bold(),
                upgrade.current_source.dimmed(),
                upgrade.current_version.dimmed(),
                upgrade.better_source.cyan(),
                upgrade.better_version.green(),
                format!("({})", upgrade.better_source).dimmed()
            );

            // Show install command
            let install_cmd = match upgrade.better_source.as_str() {
                "cargo" => format!("cargo install {}", upgrade.name),
                "pip" => format!("pip install {}", upgrade.name),
                "npm" => format!("npm install -g {}", upgrade.name),
                _ => String::new(),
            };
            if !install_cmd.is_empty() {
                println!("    {}", install_cmd.dimmed());
            }
        }
    }

    Ok(())
}
