//! Core commands: add, list, search, show, remove

use anyhow::Result;
use colored::Colorize;
use comfy_table::{
    Cell, Color, ContentArrangement, Table, modifiers::UTF8_ROUND_CORNERS, presets::UTF8_FULL,
};

use crate::db::Database;
use crate::icons::{category_icon, print_legend_compact, source_icon, status_icon};
use crate::models::{InstallSource, Tool};

/// Add a new tool to the database
#[allow(clippy::too_many_arguments)]
pub fn cmd_add(
    db: &Database,
    name: String,
    description: Option<String>,
    category: Option<String>,
    source: Option<String>,
    install_cmd: Option<String>,
    binary: Option<String>,
    installed: bool,
) -> Result<()> {
    // Check if tool already exists
    if db.get_tool_by_name(&name)?.is_some() {
        println!("{} Tool '{}' already exists", "!".yellow(), name);
        return Ok(());
    }

    let mut tool = Tool::new(&name);

    if let Some(desc) = description {
        tool = tool.with_description(desc);
    }
    if let Some(cat) = category {
        tool = tool.with_category(cat);
    }
    if let Some(src) = source {
        tool = tool.with_source(InstallSource::from(src.as_str()));
    }
    if let Some(cmd) = install_cmd {
        tool = tool.with_install_command(cmd);
    }
    if let Some(bin) = binary {
        tool = tool.with_binary(bin);
    }
    if installed {
        tool = tool.installed();
    }

    db.insert_tool(&tool)?;
    println!("{} Added '{}'", "+".green(), name);

    Ok(())
}

/// List tools in the database
pub fn cmd_list(
    db: &Database,
    installed_only: bool,
    category: Option<String>,
    label: Option<String>,
    format: &str,
) -> Result<()> {
    // If filtering by label, use the label-specific query
    let tools = if let Some(lbl) = &label {
        db.list_tools_by_label(lbl)?
    } else {
        db.list_tools(installed_only, category.as_deref())?
    };

    if tools.is_empty() {
        println!("No tools found");
        return Ok(());
    }

    match format {
        "json" => {
            println!("{}", serde_json::to_string_pretty(&tools)?);
        }
        _ => {
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
                    Cell::new("Name").fg(Color::Cyan),
                    Cell::new("Cat").fg(Color::Cyan),
                    Cell::new("Src").fg(Color::Cyan),
                    Cell::new("✓").fg(Color::Cyan),
                    Cell::new("Description").fg(Color::Cyan),
                ]);

            for tool in &tools {
                let cat = tool.category.as_deref().unwrap_or("-");
                let cat_display = format!("{} {}", category_icon(cat), cat);

                let src = tool.source.to_string();
                let src_display = source_icon(&src).to_string();

                let status_cell = if tool.is_installed {
                    Cell::new(status_icon(true)).fg(Color::Green)
                } else {
                    Cell::new(status_icon(false)).fg(Color::Red)
                };

                let desc = tool.description.as_deref().unwrap_or("");

                table.add_row(vec![
                    Cell::new(&tool.name),
                    Cell::new(cat_display),
                    Cell::new(src_display),
                    status_cell,
                    Cell::new(desc),
                ]);
            }

            println!("{table}");
            print_legend_compact();
            println!("{} {} tools", ">".cyan(), tools.len());
        }
    }

    Ok(())
}

/// Search for tools
pub fn cmd_search(db: &Database, query: &str) -> Result<()> {
    let tools = db.search_tools(query)?;

    if tools.is_empty() {
        println!("No tools found matching '{}'", query);
        return Ok(());
    }

    println!("Found {} tool(s):\n", tools.len());

    for tool in tools {
        let status = if tool.is_installed {
            "installed".green()
        } else {
            "missing".red()
        };

        println!(
            "  {} {} [{}]",
            tool.name.bold(),
            status,
            tool.category.as_deref().unwrap_or("uncategorized")
        );
        if let Some(desc) = &tool.description {
            println!("    {}", desc.dimmed());
        }
    }

    Ok(())
}

/// Show details of a specific tool
pub fn cmd_show(db: &Database, name: &str) -> Result<()> {
    match db.get_tool_by_name(name)? {
        Some(tool) => {
            println!("{}", tool.name.bold());
            println!("{}", "=".repeat(tool.name.len()));

            if let Some(desc) = &tool.description {
                println!("\n{}", desc);
            }

            println!(
                "\n{}: {}",
                "Category".bold(),
                tool.category.as_deref().unwrap_or("-")
            );
            println!("{}: {}", "Source".bold(), tool.source);

            let status = if tool.is_installed {
                "installed".green()
            } else {
                "not installed".red()
            };
            println!("{}: {}", "Status".bold(), status);

            if let Some(bin) = &tool.binary_name {
                println!("{}: {}", "Binary".bold(), bin);
            }

            if let Some(cmd) = &tool.install_command {
                println!("{}: {}", "Install".bold(), cmd);
            }

            // Show GitHub info if available
            if let Ok(Some(gh_info)) = db.get_github_info(&tool.name) {
                println!("\n{}", "GitHub:".bold());
                println!("  Repo: {}/{}", gh_info.repo_owner, gh_info.repo_name);
                println!("  Stars: {}", gh_info.stars.to_string().yellow());
            }

            // Show usage if available
            if let Ok(Some(usage)) = db.get_usage(&tool.name)
                && usage.use_count > 0
            {
                println!(
                    "\n{}: {} times",
                    "Usage".bold(),
                    usage.use_count.to_string().cyan()
                );
            }

            if let Some(notes) = &tool.notes {
                println!("\n{}", "Notes:".bold());
                println!("{}", notes);
            }

            println!(
                "\n{}: {}",
                "Added".dimmed(),
                tool.created_at.format("%Y-%m-%d %H:%M")
            );
        }
        None => {
            println!("Tool '{}' not found", name);
        }
    }

    Ok(())
}

/// Remove a tool from the database
pub fn cmd_remove(db: &Database, name: &str, force: bool) -> Result<()> {
    if !force {
        print!("Remove tool '{}'? [y/N] ", name);
        std::io::Write::flush(&mut std::io::stdout())?;

        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;

        if !input.trim().eq_ignore_ascii_case("y") {
            println!("Cancelled");
            return Ok(());
        }
    }

    if db.delete_tool(name)? {
        println!("{} Removed '{}'", "-".red(), name);
    } else {
        println!("Tool '{}' not found", name);
    }

    Ok(())
}
