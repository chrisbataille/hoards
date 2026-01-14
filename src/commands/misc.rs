//! Miscellaneous command implementations
//!
//! Export, import, doctor, and edit commands.

use anyhow::Result;
use colored::Colorize;
use dialoguer::{Confirm, Input, Select};

use crate::{Database, InstallSource, Tool};

/// Maximum number of items to display in doctor command output
const MAX_DISPLAY_ITEMS: usize = 10;

/// Export tools to JSON or TOML
pub fn cmd_export(db: &Database, output: Option<String>, format: &str, installed_only: bool) -> Result<()> {
    use std::io::Write;

    let tools = if installed_only {
        db.list_tools(true, None)?
    } else {
        db.get_all_tools()?
    };

    if tools.is_empty() {
        println!("{} No tools to export", "!".yellow());
        return Ok(());
    }

    // Convert to exportable format
    #[derive(serde::Serialize)]
    struct ExportTool {
        name: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        description: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        category: Option<String>,
        source: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        install_command: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        binary_name: Option<String>,
        installed: bool,
    }

    #[derive(serde::Serialize)]
    struct Export {
        version: String,
        exported_at: String,
        tools: Vec<ExportTool>,
    }

    let export = Export {
        version: "1.0".to_string(),
        exported_at: chrono::Utc::now().to_rfc3339(),
        tools: tools.iter().map(|t| ExportTool {
            name: t.name.clone(),
            description: t.description.clone(),
            category: t.category.clone(),
            source: t.source.to_string(),
            install_command: t.install_command.clone(),
            binary_name: t.binary_name.clone(),
            installed: t.is_installed,
        }).collect(),
    };

    let content = match format {
        "toml" => toml::to_string_pretty(&export)?,
        _ => serde_json::to_string_pretty(&export)?,
    };

    match output {
        Some(path) => {
            // Validate path to prevent directory traversal
            let path = std::path::Path::new(&path);
            if path.components().any(|c| matches!(c, std::path::Component::ParentDir)) {
                anyhow::bail!("Output path cannot contain '..' components");
            }
            let mut file = std::fs::File::create(path)?;
            file.write_all(content.as_bytes())?;
            println!(
                "{} Exported {} tools to {}",
                "+".green(),
                tools.len(),
                path.display().to_string().cyan()
            );
        }
        None => {
            println!("{}", content);
        }
    }

    Ok(())
}

/// Import tools from JSON or TOML
pub fn cmd_import(db: &Database, file: &str, skip_existing: bool, dry_run: bool) -> Result<()> {
    use std::fs;

    let content = fs::read_to_string(file)?;

    #[derive(serde::Deserialize)]
    struct ImportTool {
        name: String,
        description: Option<String>,
        category: Option<String>,
        source: Option<String>,
        install_command: Option<String>,
        binary_name: Option<String>,
        #[serde(default)]
        installed: bool,
    }

    #[derive(serde::Deserialize)]
    struct Import {
        tools: Vec<ImportTool>,
    }

    let import: Import = if file.ends_with(".toml") {
        toml::from_str(&content)?
    } else {
        serde_json::from_str(&content)?
    };

    println!(
        "{} Found {} tools in {}",
        ">".cyan(),
        import.tools.len(),
        file
    );

    let mut added = 0;
    let mut skipped = 0;

    for tool in import.tools {
        let exists = db.get_tool_by_name(&tool.name)?.is_some();

        if exists {
            if skip_existing {
                skipped += 1;
                continue;
            } else if !dry_run {
                // Update existing tool
                // For now, skip - could add update logic later
                skipped += 1;
                continue;
            }
        }

        if dry_run {
            println!(
                "  {} {} ({})",
                "[dry]".yellow(),
                tool.name.cyan(),
                tool.source.as_deref().unwrap_or("unknown")
            );
        } else {
            let mut new_tool = Tool::new(&tool.name);
            if let Some(desc) = tool.description {
                new_tool = new_tool.with_description(desc);
            }
            if let Some(cat) = tool.category {
                new_tool = new_tool.with_category(cat);
            }
            if let Some(src) = tool.source {
                new_tool = new_tool.with_source(InstallSource::from(src.as_str()));
            }
            if let Some(cmd) = tool.install_command {
                new_tool = new_tool.with_install_command(cmd);
            }
            if let Some(bin) = tool.binary_name {
                new_tool = new_tool.with_binary(bin);
            }
            if tool.installed {
                new_tool = new_tool.installed();
            }

            db.insert_tool(&new_tool)?;
            println!("  {} {}", "+".green(), tool.name.cyan());
        }
        added += 1;
    }

    println!();
    if dry_run {
        println!(
            "{} Would add {} tools ({} skipped). Run without {} to apply.",
            ">".cyan(),
            added,
            skipped,
            "--dry-run".yellow()
        );
    } else {
        println!(
            "{} Added {} tools ({} skipped)",
            "+".green(),
            added,
            skipped
        );
    }

    Ok(())
}

/// Run health checks on the database
pub fn cmd_doctor(db: &Database, fix: bool) -> Result<()> {
    println!("{}", "Running health checks...".bold());
    println!();

    let mut issues_found = 0;
    let mut fixed = 0;

    // Check 1: Tools marked as installed but binary not found
    println!("{}", "Checking installed tools...".dimmed());
    let tools = db.get_all_tools()?;
    let mut missing_binaries: Vec<(String, String)> = Vec::new();

    for tool in &tools {
        if tool.is_installed {
            let binary = tool.binary_name.as_ref().unwrap_or(&tool.name);
            if which::which(binary).is_err() {
                missing_binaries.push((tool.name.clone(), binary.clone()));
            }
        }
    }

    if !missing_binaries.is_empty() {
        println!(
            "  {} {} tools marked installed but binary not found:",
            "!".yellow(),
            missing_binaries.len()
        );
        for (name, binary) in &missing_binaries {
            println!("    {} (binary: {})", name.red(), binary);
        }
        issues_found += missing_binaries.len();

        if fix {
            for (name, _) in &missing_binaries {
                db.set_tool_installed(name, false)?;
                fixed += 1;
            }
            println!("    {} Marked {} tools as not installed", "✓".green(), missing_binaries.len());
        }
    } else {
        println!("  {} All installed tools have valid binaries", "✓".green());
    }

    // Check 2: Tools without descriptions
    println!("{}", "Checking for missing descriptions...".dimmed());
    let no_description: Vec<_> = tools.iter()
        .filter(|t| t.description.is_none())
        .collect();

    if !no_description.is_empty() {
        println!(
            "  {} {} tools have no description:",
            "!".yellow(),
            no_description.len()
        );
        for tool in no_description.iter().take(MAX_DISPLAY_ITEMS) {
            println!("    {}", tool.name);
        }
        if no_description.len() > MAX_DISPLAY_ITEMS {
            println!("    ... and {} more", no_description.len() - MAX_DISPLAY_ITEMS);
        }
        issues_found += no_description.len();
        println!("    {} Run {} to fetch from package registries", "?".blue(), "hoard fetch-descriptions".cyan());
        println!("    {} Run {} to fetch from GitHub", "?".blue(), "hoard gh sync".cyan());
    } else {
        println!("  {} All tools have descriptions", "✓".green());
    }

    // Check 3: Tools without categories
    println!("{}", "Checking for missing categories...".dimmed());
    let no_category: Vec<_> = tools.iter()
        .filter(|t| t.category.is_none())
        .collect();

    if !no_category.is_empty() {
        println!(
            "  {} {} tools have no category:",
            "!".yellow(),
            no_category.len()
        );
        for tool in no_category.iter().take(MAX_DISPLAY_ITEMS) {
            println!("    {}", tool.name);
        }
        if no_category.len() > MAX_DISPLAY_ITEMS {
            println!("    ... and {} more", no_category.len() - MAX_DISPLAY_ITEMS);
        }
        issues_found += no_category.len();
        println!("    {} Run {} to auto-categorize", "?".blue(), "hoard ai categorize".cyan());
    } else {
        println!("  {} All tools have categories", "✓".green());
    }

    // Check 4: Tools without installation source
    println!("{}", "Checking for missing sources...".dimmed());
    let no_source: Vec<_> = tools.iter()
        .filter(|t| matches!(t.source, InstallSource::Unknown))
        .collect();

    if !no_source.is_empty() {
        println!(
            "  {} {} tools have no installation source:",
            "!".yellow(),
            no_source.len()
        );
        for tool in no_source.iter().take(MAX_DISPLAY_ITEMS) {
            println!("    {}", tool.name);
        }
        if no_source.len() > MAX_DISPLAY_ITEMS {
            println!("    ... and {} more", no_source.len() - MAX_DISPLAY_ITEMS);
        }
        issues_found += no_source.len();
    } else {
        println!("  {} All tools have installation sources", "✓".green());
    }

    // Check 5: Orphaned usage records
    println!("{}", "Checking usage records...".dimmed());
    let orphaned_count = db.count_orphaned_usage()?;

    if orphaned_count > 0 {
        println!(
            "  {} {} orphaned usage records found",
            "!".yellow(),
            orphaned_count
        );
        issues_found += orphaned_count;

        if fix {
            db.delete_orphaned_usage()?;
            fixed += orphaned_count;
            println!("    {} Deleted {} orphaned records", "✓".green(), orphaned_count);
        }
    } else {
        println!("  {} No orphaned usage records", "✓".green());
    }

    // Check 6: Duplicate binaries (different tools pointing to same binary)
    println!("{}", "Checking for duplicate binaries...".dimmed());
    let mut binary_map: std::collections::HashMap<String, Vec<String>> = std::collections::HashMap::new();
    for tool in &tools {
        let binary = tool.binary_name.as_ref().unwrap_or(&tool.name).clone();
        binary_map.entry(binary).or_default().push(tool.name.clone());
    }
    let duplicates: Vec<_> = binary_map.iter()
        .filter(|(_, names)| names.len() > 1)
        .collect();

    if !duplicates.is_empty() {
        println!(
            "  {} {} binaries shared by multiple tools:",
            "!".yellow(),
            duplicates.len()
        );
        for (binary, tools) in &duplicates {
            println!("    {} -> {}", binary.cyan(), tools.join(", "));
        }
        issues_found += duplicates.len();
    } else {
        println!("  {} No duplicate binaries", "✓".green());
    }

    // Summary
    println!();
    if issues_found == 0 {
        println!("{} {}", "✓".green().bold(), "Database is healthy!".green().bold());
    } else {
        println!(
            "{} {} issues found{}",
            "!".yellow().bold(),
            issues_found,
            if fix { format!(", {} fixed", fixed) } else { String::new() }
        );
        if !fix && fixed < issues_found {
            println!("  {} Run {} to auto-fix some issues", "?".blue(), "hoard doctor --fix".cyan());
        }
    }

    Ok(())
}

/// Interactive tool editor
pub fn cmd_edit(db: &Database, name: &str) -> Result<()> {
    let tool = db.get_tool_by_name(name)?;

    let mut tool = match tool {
        Some(t) => t,
        None => {
            println!("{} Tool '{}' not found", "✗".red(), name);
            return Ok(());
        }
    };

    println!("{} {}", "Editing:".bold(), tool.name.cyan().bold());
    println!();

    // Show current values and let user edit each field
    let new_description: String = Input::new()
        .with_prompt("Description")
        .with_initial_text(tool.description.clone().unwrap_or_default())
        .allow_empty(true)
        .interact_text()?;

    let categories = db.get_categories()?;
    let category_options: Vec<String> = std::iter::once("(none)".to_string())
        .chain(categories.iter().cloned())
        .chain(std::iter::once("(new category)".to_string()))
        .collect();

    let current_cat_idx = if let Some(ref cat) = tool.category {
        categories.iter().position(|c| c == cat).map(|i| i + 1).unwrap_or(0)
    } else {
        0
    };

    let cat_selection = Select::new()
        .with_prompt("Category")
        .items(&category_options)
        .default(current_cat_idx)
        .interact()?;

    let new_category = if cat_selection == 0 {
        None
    } else if cat_selection == category_options.len() - 1 {
        // New category
        let custom: String = Input::new()
            .with_prompt("New category name")
            .interact_text()?;
        if custom.is_empty() { None } else { Some(custom) }
    } else {
        Some(categories[cat_selection - 1].clone())
    };

    let sources = ["cargo", "pip", "npm", "apt", "brew", "snap", "manual", "unknown"];
    let current_src_str = tool.source.to_string();
    let current_src_idx = sources.iter().position(|s| *s == current_src_str).unwrap_or(sources.len() - 1);

    let src_selection = Select::new()
        .with_prompt("Installation source")
        .items(&sources)
        .default(current_src_idx)
        .interact()?;

    let new_source = InstallSource::from(sources[src_selection]);

    let new_binary: String = Input::new()
        .with_prompt("Binary name")
        .with_initial_text(tool.binary_name.clone().unwrap_or_default())
        .allow_empty(true)
        .interact_text()?;

    let new_install_cmd: String = Input::new()
        .with_prompt("Install command")
        .with_initial_text(tool.install_command.clone().unwrap_or_default())
        .allow_empty(true)
        .interact_text()?;

    let new_installed = Confirm::new()
        .with_prompt("Installed?")
        .default(tool.is_installed)
        .interact()?;

    // Show summary and confirm
    println!();
    println!("{}", "Changes:".bold());

    let mut changes = Vec::new();

    let new_desc_opt = if new_description.is_empty() { None } else { Some(new_description.clone()) };
    if new_desc_opt != tool.description {
        println!("  {} Description: {} -> {}",
            "~".yellow(),
            tool.description.as_deref().unwrap_or("(none)").dimmed(),
            new_desc_opt.as_deref().unwrap_or("(none)")
        );
        changes.push("description");
    }

    if new_category != tool.category {
        println!("  {} Category: {} -> {}",
            "~".yellow(),
            tool.category.as_deref().unwrap_or("(none)").dimmed(),
            new_category.as_deref().unwrap_or("(none)")
        );
        changes.push("category");
    }

    if new_source != tool.source {
        println!("  {} Source: {} -> {}",
            "~".yellow(),
            tool.source.to_string().dimmed(),
            new_source
        );
        changes.push("source");
    }

    let new_binary_opt = if new_binary.is_empty() { None } else { Some(new_binary.clone()) };
    if new_binary_opt != tool.binary_name {
        println!("  {} Binary: {} -> {}",
            "~".yellow(),
            tool.binary_name.as_deref().unwrap_or("(none)").dimmed(),
            new_binary_opt.as_deref().unwrap_or("(none)")
        );
        changes.push("binary");
    }

    let new_cmd_opt = if new_install_cmd.is_empty() { None } else { Some(new_install_cmd.clone()) };
    if new_cmd_opt != tool.install_command {
        println!("  {} Install cmd: {} -> {}",
            "~".yellow(),
            tool.install_command.as_deref().unwrap_or("(none)").dimmed(),
            new_cmd_opt.as_deref().unwrap_or("(none)")
        );
        changes.push("install_cmd");
    }

    if new_installed != tool.is_installed {
        println!("  {} Installed: {} -> {}",
            "~".yellow(),
            tool.is_installed.to_string().dimmed(),
            new_installed
        );
        changes.push("installed");
    }

    if changes.is_empty() {
        println!("  {} No changes", "=".dimmed());
        return Ok(());
    }

    println!();
    if !Confirm::new()
        .with_prompt("Save changes?")
        .default(true)
        .interact()?
    {
        println!("{} Cancelled", "!".yellow());
        return Ok(());
    }

    // Apply changes by updating the tool struct and calling update_tool
    tool.description = new_desc_opt;
    tool.category = new_category;
    tool.source = new_source;
    tool.binary_name = new_binary_opt;
    tool.install_command = new_cmd_opt;
    tool.is_installed = new_installed;

    db.update_tool(&tool)?;

    println!("{} Updated '{}'", "✓".green(), name);

    Ok(())
}
