//! Label management commands

use anyhow::{Context, Result};
use colored::Colorize;

use crate::db::Database;
use crate::models::InstallSource;

/// Add labels to a tool
pub fn cmd_label_add(db: &Database, name: &str, labels: &[String]) -> Result<()> {
    // Check if tool exists
    let _tool = db
        .get_tool_by_name(name)?
        .context(format!("Tool '{}' not found", name))?;

    let normalized: Vec<String> = labels.iter().map(|l| normalize_label(l)).collect();

    db.add_labels(name, &normalized)?;

    println!(
        "{} Added {} label(s) to {}",
        "✓".green(),
        normalized.len(),
        name.cyan()
    );
    for label in &normalized {
        println!("  + {}", label.yellow());
    }

    Ok(())
}

/// Remove labels from a tool
pub fn cmd_label_remove(db: &Database, name: &str, labels: &[String]) -> Result<()> {
    // Check if tool exists
    let _tool = db
        .get_tool_by_name(name)?
        .context(format!("Tool '{}' not found", name))?;

    let normalized: Vec<String> = labels.iter().map(|l| normalize_label(l)).collect();

    // Get current labels to check which ones exist
    let current = db.get_labels(name)?;
    let mut removed = 0;

    for label in &normalized {
        if current.contains(label) {
            db.remove_label(name, label)?;
            println!("  - {}", label.red());
            removed += 1;
        } else {
            println!("  {} {} (not found)", "⚠".yellow(), label);
        }
    }

    println!(
        "{} Removed {} label(s) from {}",
        "✓".green(),
        removed,
        name.cyan()
    );

    Ok(())
}

/// List all labels or labels for a specific tool
pub fn cmd_label_list(db: &Database, tool_name: Option<&str>) -> Result<()> {
    match tool_name {
        Some(name) => {
            // List labels for specific tool
            let _tool = db
                .get_tool_by_name(name)?
                .context(format!("Tool '{}' not found", name))?;

            let labels = db.get_labels(name)?;

            if labels.is_empty() {
                println!("{} has no labels", name.cyan());
            } else {
                println!("{} labels for {}:", labels.len(), name.cyan());
                for label in labels {
                    println!("  {}", label.yellow());
                }
            }
        }
        None => {
            // List all labels with counts
            let counts = db.get_label_counts()?;

            if counts.is_empty() {
                println!("No labels found");
            } else {
                println!("{}", "Labels".bold());
                println!();
                for (label, count) in counts {
                    println!(
                        "  {} {} tool{}",
                        label.yellow(),
                        count,
                        if count == 1 { "" } else { "s" }
                    );
                }
            }
        }
    }

    Ok(())
}

/// Clear all labels from a tool
pub fn cmd_label_clear(db: &Database, name: &str) -> Result<()> {
    // Check if tool exists
    let _tool = db
        .get_tool_by_name(name)?
        .context(format!("Tool '{}' not found", name))?;

    let labels = db.get_labels(name)?;
    let count = labels.len();

    db.clear_labels(name)?;

    println!(
        "{} Cleared {} label(s) from {}",
        "✓".green(),
        count,
        name.cyan()
    );

    Ok(())
}

/// Auto-label tools based on metadata and AI
pub fn cmd_label_auto(
    db: &Database,
    tool_name: Option<&str>,
    force: bool,
    use_ai: bool,
    dry_run: bool,
) -> Result<()> {
    let tools = match tool_name {
        Some(name) => {
            let tool = db
                .get_tool_by_name(name)?
                .context(format!("Tool '{}' not found", name))?;
            vec![tool]
        }
        None => db.list_tools(false, None)?,
    };

    let mut labeled_count = 0;
    let mut skipped_count = 0;

    for tool in &tools {
        let current_labels = db.get_labels(&tool.name)?;

        // Skip if already has enough labels (unless force)
        if !force && current_labels.len() >= 2 {
            skipped_count += 1;
            continue;
        }

        // Collect metadata labels
        let mut new_labels =
            collect_metadata_labels(db, &tool.name, &tool.source, tool.category.as_deref())?;

        // Deduplicate with existing
        new_labels.retain(|l| !current_labels.contains(l));

        // AI fallback if still under threshold
        let total_labels = current_labels.len() + new_labels.len();
        if use_ai
            && total_labels < 2
            && let Some(ai_labels) = generate_ai_labels(&tool.name, tool.description.as_deref())?
        {
            for label in ai_labels {
                let normalized = normalize_label(&label);
                if !current_labels.contains(&normalized) && !new_labels.contains(&normalized) {
                    new_labels.push(normalized);
                }
            }
        }

        if new_labels.is_empty() {
            skipped_count += 1;
            continue;
        }

        if dry_run {
            println!(
                "{} {} would get: {}",
                "→".blue(),
                tool.name.cyan(),
                new_labels.join(", ").yellow()
            );
        } else {
            db.add_labels(&tool.name, &new_labels)?;
            println!(
                "{} {} labeled: {}",
                "✓".green(),
                tool.name.cyan(),
                new_labels.join(", ").yellow()
            );
        }
        labeled_count += 1;
    }

    println!();
    if dry_run {
        println!(
            "Would label {} tool(s), {} already labeled",
            labeled_count, skipped_count
        );
    } else {
        println!(
            "{} Labeled {} tool(s), {} already labeled",
            "✓".green(),
            labeled_count,
            skipped_count
        );
    }

    Ok(())
}

/// Collect labels from tool metadata
fn collect_metadata_labels(
    _db: &Database,
    _tool_name: &str,
    source: &InstallSource,
    category: Option<&str>,
) -> Result<Vec<String>> {
    let mut labels = Vec::new();

    // Add source as label
    let source_str = source.to_string().to_lowercase();
    if source_str != "unknown" && source_str != "manual" {
        labels.push(source_str.clone());
    }

    // Add language based on source
    if let Some(lang) = source_to_language(source) {
        labels.push(lang.to_string());
    }

    // Add category as label
    if let Some(cat) = category {
        labels.push(normalize_label(cat));
    }

    // Note: GitHub topics are synced separately during `hoard sync --github`
    // and stored as labels, so they'll already be in the database

    Ok(labels)
}

/// Map install source to programming language
fn source_to_language(source: &InstallSource) -> Option<&'static str> {
    match source {
        InstallSource::Cargo => Some("rust"),
        InstallSource::Pip => Some("python"),
        InstallSource::Npm => Some("javascript"),
        InstallSource::Go => Some("go"),
        _ => None,
    }
}

/// Generate labels using AI
fn generate_ai_labels(name: &str, description: Option<&str>) -> Result<Option<Vec<String>>> {
    use crate::ai::invoke_ai;
    use crate::config::{AiProvider, HoardConfig};

    let config = HoardConfig::load().unwrap_or_default();

    // Skip if no AI provider configured
    if config.ai.provider == AiProvider::None {
        return Ok(None);
    }

    let desc = description.unwrap_or("No description available");
    let prompt = format!(
        "Tool: {}\nDescription: {}\n\nSuggest 3-5 lowercase, hyphenated labels for categorizing this CLI tool. Return only the labels, comma-separated, nothing else.",
        name, desc
    );

    match invoke_ai(&prompt) {
        Ok(response) => {
            let labels: Vec<String> = response
                .split(',')
                .map(|s| normalize_label(s.trim()))
                .filter(|s| !s.is_empty() && s.len() < 30)
                .take(5)
                .collect();
            Ok(Some(labels))
        }
        Err(_) => Ok(None), // Silently skip AI if it fails
    }
}

/// Normalize a label (lowercase, spaces to hyphens)
fn normalize_label(label: &str) -> String {
    label
        .trim()
        .to_lowercase()
        .chars()
        .filter_map(|c| match c {
            ' ' | '_' => Some('-'),
            c if c.is_alphanumeric() || c == '-' => Some(c),
            _ => None,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_label() {
        assert_eq!(normalize_label("File Manager"), "file-manager");
        assert_eq!(normalize_label("  RUST  "), "rust");
        assert_eq!(normalize_label("text_processing"), "text-processing");
        assert_eq!(normalize_label("cli-tool"), "cli-tool");
    }

    #[test]
    fn test_source_to_language() {
        assert_eq!(source_to_language(&InstallSource::Cargo), Some("rust"));
        assert_eq!(source_to_language(&InstallSource::Pip), Some("python"));
        assert_eq!(source_to_language(&InstallSource::Npm), Some("javascript"));
        assert_eq!(source_to_language(&InstallSource::Go), Some("go"));
        assert_eq!(source_to_language(&InstallSource::Apt), None);
    }
}
