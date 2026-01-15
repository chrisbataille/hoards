//! Config management commands
//!
//! Commands for managing dotfiles and tool configurations.

use crate::db::Database;
use crate::models::Config;
use anyhow::{Context, Result, bail};
use colored::Colorize;
use std::fs;
use std::os::unix::fs as unix_fs;
use std::path::{Path, PathBuf};

/// Expand ~ to home directory
fn expand_path(path: &str) -> PathBuf {
    if path.starts_with("~/")
        && let Some(home) = dirs::home_dir()
    {
        return home.join(&path[2..]);
    }
    PathBuf::from(path)
}

/// Check if a path is a symlink pointing to the expected target
fn is_valid_symlink(link_path: &Path, expected_target: &Path) -> bool {
    if !link_path.is_symlink() {
        return false;
    }

    match fs::read_link(link_path) {
        Ok(target) => {
            // Normalize both paths for comparison
            let target = if target.is_absolute() {
                target
            } else {
                link_path.parent().unwrap_or(Path::new(".")).join(target)
            };

            // Compare canonical paths if possible
            match (target.canonicalize(), expected_target.canonicalize()) {
                (Ok(a), Ok(b)) => a == b,
                _ => target == expected_target,
            }
        }
        Err(_) => false,
    }
}

/// Link a config to be managed by hoard
pub fn cmd_config_link(
    db: &Database,
    name: &str,
    target: &str,
    source: &str,
    tool: Option<String>,
) -> Result<()> {
    // Check if config already exists
    if db.get_config_by_name(name)?.is_some() {
        bail!(
            "Config '{}' already exists. Use 'hoards config edit' to modify it.",
            name
        );
    }

    let target_path = expand_path(target);
    let source_path = expand_path(source);

    // Verify source exists
    if !source_path.exists() {
        bail!("Source path does not exist: {}", source_path.display());
    }

    // Create the config entry
    let mut config = Config::new(
        name,
        source_path.to_string_lossy(),
        target_path.to_string_lossy(),
    );

    // Link to tool if specified
    if let Some(ref tool_name) = tool {
        let tool_entry = db
            .get_tool_by_name(tool_name)?
            .ok_or_else(|| anyhow::anyhow!("Tool '{}' not found", tool_name))?;
        config.tool_id = tool_entry.id;
    }

    db.insert_config(&config)?;

    println!("{} Added config '{}'", "✓".green(), name);
    println!("  Source: {}", source_path.display());
    println!("  Target: {}", target_path.display());
    if let Some(tool_name) = tool {
        println!("  Tool:   {}", tool_name);
    }
    println!();
    println!("Run {} to create the symlink", "hoards config sync".cyan());

    Ok(())
}

/// Unlink a config
pub fn cmd_config_unlink(
    db: &Database,
    name: &str,
    remove_symlink: bool,
    force: bool,
) -> Result<()> {
    let config = db
        .get_config_by_name(name)?
        .ok_or_else(|| anyhow::anyhow!("Config '{}' not found", name))?;

    if !force {
        println!("Remove config '{}'?", name);
        println!("  Source: {}", config.source_path);
        println!("  Target: {}", config.target_path);
        if remove_symlink {
            println!("  {} Symlink will be removed", "!".yellow());
        }

        let confirm = dialoguer::Confirm::new()
            .with_prompt("Continue?")
            .default(false)
            .interact()?;

        if !confirm {
            println!("Cancelled.");
            return Ok(());
        }
    }

    // Remove symlink if requested
    if remove_symlink {
        let target_path = expand_path(&config.target_path);
        if target_path.is_symlink() {
            fs::remove_file(&target_path)
                .with_context(|| format!("Failed to remove symlink: {}", target_path.display()))?;
            println!("{} Removed symlink: {}", "✓".green(), target_path.display());
        }
    }

    db.delete_config(name)?;
    println!("{} Removed config '{}'", "✓".green(), name);

    Ok(())
}

/// List all managed configs
pub fn cmd_config_list(db: &Database, broken_only: bool, format: &str) -> Result<()> {
    use crate::icons::config_status_icon;
    use comfy_table::{
        Cell, Color, ContentArrangement, Table, modifiers::UTF8_ROUND_CORNERS, presets::UTF8_FULL,
    };

    let configs = db.list_configs()?;

    if configs.is_empty() {
        println!(
            "No configs managed. Use {} to add one.",
            "hoards config link".cyan()
        );
        return Ok(());
    }

    if format == "json" {
        let json = serde_json::to_string_pretty(&configs)?;
        println!("{}", json);
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
            Cell::new("⚙ Config").fg(Color::Cyan),
            Cell::new("Target").fg(Color::Cyan),
            Cell::new("Source").fg(Color::Cyan),
            Cell::new("✓").fg(Color::Cyan),
        ]);

    let mut shown = 0;
    for config in configs {
        let target_path = expand_path(&config.target_path);
        let source_path = expand_path(&config.source_path);

        let (status_text, status_color) = if !source_path.exists() {
            ("missing", Color::Red)
        } else if is_valid_symlink(&target_path, &source_path) {
            ("linked", Color::Green)
        } else if target_path.exists() {
            ("conflict", Color::Yellow)
        } else {
            ("unlinked", Color::Grey)
        };

        // Filter if showing broken only
        if broken_only && (status_text == "linked" || status_text == "unlinked") {
            continue;
        }

        table.add_row(vec![
            Cell::new(&config.name),
            Cell::new(&config.target_path),
            Cell::new(&config.source_path),
            Cell::new(config_status_icon(status_text)).fg(status_color),
        ]);
        shown += 1;
    }

    println!("{table}");
    println!(
        "{} 🔗 linked  ❌ missing  ⚠ conflict  ◯ unlinked",
        "".dimmed()
    );
    println!("{} {} configs", ">".cyan(), shown);
    Ok(())
}

/// Show details for a specific config
pub fn cmd_config_show(db: &Database, name: &str) -> Result<()> {
    let config = db
        .get_config_by_name(name)?
        .ok_or_else(|| anyhow::anyhow!("Config '{}' not found", name))?;

    let target_path = expand_path(&config.target_path);
    let source_path = expand_path(&config.source_path);

    println!("{}", config.name.bold());
    println!();
    println!("  Source: {}", config.source_path);
    println!("  Target: {}", config.target_path);

    // Status
    let status = if !source_path.exists() {
        "Source missing".red()
    } else if is_valid_symlink(&target_path, &source_path) {
        "Linked".green()
    } else if target_path.exists() {
        "Conflict (target exists)".yellow()
    } else {
        "Not linked".dimmed()
    };
    println!("  Status: {}", status);

    // Associated tool
    if let Some(tool_id) = config.tool_id {
        // Try to find the tool name
        let tools = db.list_tools(false, None)?;
        if let Some(tool) = tools.iter().find(|t| t.id == Some(tool_id)) {
            println!("  Tool:   {}", tool.name);
        }
    }

    println!("  Added:  {}", config.created_at.format("%Y-%m-%d %H:%M"));

    Ok(())
}

/// Sync all configs (create symlinks)
pub fn cmd_config_sync(db: &Database, dry_run: bool, force: bool) -> Result<()> {
    let configs = db.list_configs()?;

    if configs.is_empty() {
        println!("No configs to sync.");
        return Ok(());
    }

    let mut created = 0;
    let mut skipped = 0;
    let mut errors = 0;

    for config in configs {
        let target_path = expand_path(&config.target_path);
        let source_path = expand_path(&config.source_path);

        // Check source exists
        if !source_path.exists() {
            println!(
                "{} {} - source missing: {}",
                "✗".red(),
                config.name,
                source_path.display()
            );
            errors += 1;
            continue;
        }

        // Check if already correctly linked
        if is_valid_symlink(&target_path, &source_path) {
            skipped += 1;
            continue;
        }

        // Check for conflicts
        if target_path.exists() || target_path.is_symlink() {
            if force {
                if dry_run {
                    println!(
                        "{} {} - would remove existing: {}",
                        "!".yellow(),
                        config.name,
                        target_path.display()
                    );
                } else if target_path.is_dir() && !target_path.is_symlink() {
                    fs::remove_dir_all(&target_path)?;
                } else {
                    fs::remove_file(&target_path)?;
                }
            } else {
                println!(
                    "{} {} - target exists: {} (use --force to overwrite)",
                    "!".yellow(),
                    config.name,
                    target_path.display()
                );
                skipped += 1;
                continue;
            }
        }

        // Create parent directory if needed
        if let Some(parent) = target_path.parent()
            && !parent.exists()
        {
            if dry_run {
                println!("  Would create directory: {}", parent.display());
            } else {
                fs::create_dir_all(parent)?;
            }
        }

        // Create symlink
        if dry_run {
            println!(
                "{} {} → {}",
                "→".cyan(),
                target_path.display(),
                source_path.display()
            );
        } else {
            unix_fs::symlink(&source_path, &target_path).with_context(|| {
                format!(
                    "Failed to create symlink: {} → {}",
                    target_path.display(),
                    source_path.display()
                )
            })?;

            db.set_config_symlinked(&config.name, true)?;
            println!(
                "{} {} → {}",
                "✓".green(),
                config.name,
                target_path.display()
            );
        }
        created += 1;
    }

    println!();
    if dry_run {
        println!(
            "Dry run: {} would be created, {} already linked, {} errors",
            created, skipped, errors
        );
    } else {
        println!(
            "Synced: {} created, {} already linked, {} errors",
            created, skipped, errors
        );
    }

    Ok(())
}

/// Show status of all config symlinks
pub fn cmd_config_status(db: &Database) -> Result<()> {
    let configs = db.list_configs()?;

    if configs.is_empty() {
        println!("No configs managed.");
        return Ok(());
    }

    let mut linked = 0;
    let mut unlinked = 0;
    let mut broken = 0;
    let mut conflicts = 0;

    println!("{}", "Config Status".bold());
    println!();

    for config in &configs {
        let target_path = expand_path(&config.target_path);
        let source_path = expand_path(&config.source_path);

        let (icon, status) = if !source_path.exists() {
            broken += 1;
            ("✗".red(), "source missing".red())
        } else if is_valid_symlink(&target_path, &source_path) {
            linked += 1;
            ("✓".green(), "linked".green())
        } else if target_path.exists() {
            conflicts += 1;
            ("!".yellow(), "conflict".yellow())
        } else {
            unlinked += 1;
            ("○".dimmed(), "not linked".dimmed())
        };

        println!("  {} {:<20} {}", icon, config.name, status);
    }

    println!();
    println!(
        "Total: {} configs ({} linked, {} unlinked, {} conflicts, {} broken)",
        configs.len(),
        linked,
        unlinked,
        conflicts,
        broken
    );

    if unlinked > 0 || conflicts > 0 {
        println!();
        println!(
            "Run {} to create missing symlinks",
            "hoards config sync".cyan()
        );
    }

    Ok(())
}

/// Edit a config's paths
pub fn cmd_config_edit(
    db: &Database,
    name: &str,
    target: Option<String>,
    source: Option<String>,
    tool: Option<String>,
) -> Result<()> {
    let config = db
        .get_config_by_name(name)?
        .ok_or_else(|| anyhow::anyhow!("Config '{}' not found", name))?;

    let new_source = source.unwrap_or(config.source_path.clone());
    let new_target = target.unwrap_or(config.target_path.clone());

    // Update paths if changed
    if new_source != config.source_path || new_target != config.target_path {
        db.update_config_paths(name, &new_source, &new_target)?;
        println!("{} Updated paths for '{}'", "✓".green(), name);
        if new_source != config.source_path {
            println!("  Source: {} → {}", config.source_path.dimmed(), new_source);
        }
        if new_target != config.target_path {
            println!("  Target: {} → {}", config.target_path.dimmed(), new_target);
        }
    }

    // Update tool association if specified
    if let Some(tool_name) = tool {
        db.link_config_to_tool(name, &tool_name)?;
        println!("{} Linked config to tool '{}'", "✓".green(), tool_name);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_expand_path_tilde() {
        let path = expand_path("~/test");
        assert!(path.to_string_lossy().contains("test"));
        assert!(!path.to_string_lossy().starts_with("~"));
    }

    #[test]
    fn test_expand_path_absolute() {
        let path = expand_path("/absolute/path");
        assert_eq!(path, PathBuf::from("/absolute/path"));
    }

    #[test]
    fn test_expand_path_relative() {
        let path = expand_path("relative/path");
        assert_eq!(path, PathBuf::from("relative/path"));
    }

    #[test]
    fn test_is_valid_symlink() {
        let temp = TempDir::new().unwrap();
        let source = temp.path().join("source");
        let link = temp.path().join("link");

        // Create source file
        fs::write(&source, "test").unwrap();

        // Create symlink
        unix_fs::symlink(&source, &link).unwrap();

        assert!(is_valid_symlink(&link, &source));
        assert!(!is_valid_symlink(&source, &link)); // source is not a symlink
    }

    #[test]
    fn test_is_valid_symlink_wrong_target() {
        let temp = TempDir::new().unwrap();
        let source = temp.path().join("source");
        let other = temp.path().join("other");
        let link = temp.path().join("link");

        fs::write(&source, "test").unwrap();
        fs::write(&other, "other").unwrap();
        unix_fs::symlink(&source, &link).unwrap();

        assert!(!is_valid_symlink(&link, &other));
    }
}
