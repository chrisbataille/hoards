//! Prompt builder functions
//!
//! Functions that build prompts from templates and input data.

use std::collections::{HashMap, HashSet};

use crate::models::{Bundle, Tool};

use super::prompts::*;

/// Get the prompts directory path
pub fn prompts_dir() -> anyhow::Result<std::path::PathBuf> {
    let config_dir = dirs::config_dir()
        .ok_or_else(|| anyhow::anyhow!("Could not determine config directory"))?
        .join("hoards")
        .join("prompts");
    Ok(config_dir)
}

/// Load a prompt template from file, falling back to embedded default
pub fn load_prompt(name: &str, default: &str) -> String {
    let path = match prompts_dir() {
        Ok(dir) => dir.join(format!("{}.txt", name)),
        Err(_) => return default.to_string(),
    };

    match std::fs::read_to_string(&path) {
        Ok(content) => content,
        Err(_) => default.to_string(),
    }
}

/// Generate a prompt for categorizing tools
pub fn categorize_prompt(tools: &[Tool], existing_categories: &[String]) -> String {
    let tool_list: Vec<String> = tools
        .iter()
        .map(|t| {
            if let Some(desc) = &t.description {
                format!("- {} : {}", t.name, desc)
            } else {
                format!("- {}", t.name)
            }
        })
        .collect();

    let categories = if existing_categories.is_empty() {
        "dev, shell, files, search, git, network, system, editor, data, security, misc".to_string()
    } else {
        existing_categories.join(", ")
    };

    let template = load_prompt("categorize", DEFAULT_CATEGORIZE_PROMPT);
    template
        .replace("{{CATEGORIES}}", &categories)
        .replace("{{TOOLS}}", &tool_list.join("\n"))
}

/// Generate a prompt for describing tools
pub fn describe_prompt(tools: &[Tool]) -> String {
    let tool_list: Vec<String> = tools.iter().map(|t| format!("- {}", t.name)).collect();

    let template = load_prompt("describe", DEFAULT_DESCRIBE_PROMPT);
    template.replace("{{TOOLS}}", &tool_list.join("\n"))
}

/// Generate a prompt for bundle suggestions with usage data
pub fn suggest_bundle_prompt(
    tools: &[Tool],
    existing_bundles: &[Bundle],
    usage_data: &HashMap<String, i64>,
    count: usize,
) -> String {
    // Collect all tools that are already in bundles
    let bundled_tools: HashSet<&str> = existing_bundles
        .iter()
        .flat_map(|b| b.tools.iter().map(|s| s.as_str()))
        .collect();

    // Filter out already-bundled tools and sort by usage
    let mut unbundled_tools: Vec<&Tool> = tools
        .iter()
        .filter(|t| !bundled_tools.contains(t.name.as_str()))
        .collect();

    // Sort by usage count (most used first)
    unbundled_tools.sort_by(|a, b| {
        let usage_a = usage_data.get(&a.name).unwrap_or(&0);
        let usage_b = usage_data.get(&b.name).unwrap_or(&0);
        usage_b.cmp(usage_a)
    });

    let tool_list: Vec<String> = unbundled_tools
        .iter()
        .map(|t| {
            let cat = t.category.as_deref().unwrap_or("uncategorized");
            let desc = t.description.as_deref().unwrap_or("");
            let usage = usage_data.get(&t.name).unwrap_or(&0);
            format!("- {} [{}] ({}x): {}", t.name, cat, usage, desc)
        })
        .collect();

    // Format existing bundles for the prompt
    let bundles_str = if existing_bundles.is_empty() {
        "No existing bundles.".to_string()
    } else {
        existing_bundles
            .iter()
            .map(|b| format!("- {}: {}", b.name, b.tools.join(", ")))
            .collect::<Vec<_>>()
            .join("\n")
    };

    let template = load_prompt("suggest-bundle", DEFAULT_SUGGEST_BUNDLE_PROMPT);
    template
        .replace("{{COUNT}}", &count.to_string())
        .replace("{{EXISTING_BUNDLES}}", &bundles_str)
        .replace("{{TOOLS}}", &tool_list.join("\n"))
}

/// Generate extraction prompt
pub fn extract_prompt(readme: &str) -> String {
    // Truncate README if too long (keep first ~8000 chars to leave room for prompt)
    let readme_truncated = if readme.len() > 8000 {
        // Find safe UTF-8 boundary near 8000 chars
        let boundary = readme
            .char_indices()
            .take_while(|(i, _)| *i < 8000)
            .last()
            .map_or(0, |(i, c)| i + c.len_utf8());
        format!("{}...\n[README truncated]", &readme[..boundary])
    } else {
        readme.to_string()
    };

    let template = load_prompt("extract", DEFAULT_EXTRACT_PROMPT);
    template.replace("{{README}}", &readme_truncated)
}

/// Generate a cheatsheet prompt from --help output
pub fn cheatsheet_prompt(tool_name: &str, help_output: &str) -> String {
    let template = load_prompt("cheatsheet", DEFAULT_CHEATSHEET_PROMPT);

    // Truncate help output if too long (keep first 4000 chars)
    let truncated_help = if help_output.len() > 4000 {
        // Find safe UTF-8 boundary near 4000 chars
        let boundary = help_output
            .char_indices()
            .take_while(|(i, _)| *i < 4000)
            .last()
            .map_or(0, |(i, c)| i + c.len_utf8());
        format!("{}...\n[truncated]", &help_output[..boundary])
    } else {
        help_output.to_string()
    };

    template
        .replace("{{TOOL_NAME}}", tool_name)
        .replace("{{HELP_OUTPUT}}", &truncated_help)
}

/// Generate a bundle cheatsheet prompt from multiple tools' --help outputs
pub fn bundle_cheatsheet_prompt(
    bundle_name: &str,
    tools_help: &[(String, String)], // (tool_name, help_output)
) -> String {
    let template = load_prompt("bundle_cheatsheet", DEFAULT_BUNDLE_CHEATSHEET_PROMPT);

    let tool_list = tools_help
        .iter()
        .map(|(name, _)| name.as_str())
        .collect::<Vec<_>>()
        .join(", ");

    // Combine help outputs with clear separators, truncating each if needed
    let mut combined_help = String::new();
    for (name, help) in tools_help {
        combined_help.push_str(&format!("\n=== {} ===\n", name));
        if help.len() > 2000 {
            // Find safe UTF-8 boundary near 2000 chars
            let boundary = help
                .char_indices()
                .take_while(|(i, _)| *i < 2000)
                .last()
                .map_or(0, |(i, c)| i + c.len_utf8());
            combined_help.push_str(&format!("{}...\n[truncated]\n", &help[..boundary]));
        } else {
            combined_help.push_str(help);
        }
    }

    // Overall truncation if still too long
    let final_help = if combined_help.len() > 12000 {
        // Find safe UTF-8 boundary near 12000 chars
        let boundary = combined_help
            .char_indices()
            .take_while(|(i, _)| *i < 12000)
            .last()
            .map_or(0, |(i, c)| i + c.len_utf8());
        format!("{}...\n[truncated]", &combined_help[..boundary])
    } else {
        combined_help
    };

    template
        .replace("{{BUNDLE_NAME}}", bundle_name)
        .replace("{{TOOL_LIST}}", &tool_list)
        .replace("{{HELP_OUTPUTS}}", &final_help)
}

/// Generate a discovery prompt from user query and context
pub fn discovery_prompt(
    query: &str,
    installed_tools: &[String],
    enabled_sources: &[&str],
) -> String {
    let template = load_prompt("discovery", DEFAULT_DISCOVERY_PROMPT);

    let installed_list = if installed_tools.is_empty() {
        "None".to_string()
    } else {
        installed_tools.join(", ")
    };

    let sources_list = if enabled_sources.is_empty() {
        "cargo, pip, npm, apt, brew".to_string() // Default to all if none specified
    } else {
        enabled_sources.join(", ")
    };

    template
        .replace("{{QUERY}}", query)
        .replace("{{INSTALLED_TOOLS}}", &installed_list)
        .replace("{{ENABLED_SOURCES}}", &sources_list)
}

/// Generate an analyze prompt from usage data
pub fn analyze_prompt(
    traditional_usage: &[(String, i64)],
    modern_tools: &[String],
    unused_tools: &[String],
) -> String {
    let template = load_prompt("analyze", DEFAULT_ANALYZE_PROMPT);

    let traditional_str = if traditional_usage.is_empty() {
        "None detected".to_string()
    } else {
        traditional_usage
            .iter()
            .map(|(cmd, count)| format!("{} ({}x)", cmd, count))
            .collect::<Vec<_>>()
            .join(", ")
    };

    let modern_str = if modern_tools.is_empty() {
        "None".to_string()
    } else {
        modern_tools.join(", ")
    };

    let unused_str = if unused_tools.is_empty() {
        "None".to_string()
    } else {
        unused_tools.join(", ")
    };

    template
        .replace("{{TRADITIONAL_USAGE}}", &traditional_str)
        .replace("{{MODERN_TOOLS}}", &modern_str)
        .replace("{{UNUSED_TOOLS}}", &unused_str)
}

/// Build prompt for migration benefit descriptions
pub fn migrate_prompt(tools: &[(String, String, String, String, String)]) -> String {
    let prompt_template = load_prompt("migrate", DEFAULT_MIGRATE_PROMPT);

    let tools_str = tools
        .iter()
        .map(|(name, from_source, from_ver, to_source, to_ver)| {
            format!(
                "- {} ({} {} → {} {})",
                name, from_source, from_ver, to_source, to_ver
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    prompt_template.replace("{{TOOLS}}", &tools_str)
}
