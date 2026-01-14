//! AI provider integration for smart features
//!
//! Provides functions to invoke configured AI CLI tools (claude, gemini, codex, opencode)
//! and parse their responses for categorization, description generation, and bundle suggestions.
//!
//! Prompts are loaded from `~/.config/hoard/prompts/` and can be customized by the user.
//! If a prompt file is missing, embedded defaults are used.

use crate::config::{AiProvider, HoardConfig};
use crate::models::{Bundle, Tool};
use anyhow::{Context, Result, bail};
use std::path::PathBuf;
use std::process::Command;

// ==================== Embedded default prompts ====================

const DEFAULT_CATEGORIZE_PROMPT: &str = r#"You are helping categorize CLI tools. Here are the existing categories: {{CATEGORIES}}

Categorize these tools into the most appropriate category. If none fit well, use "misc".
Only respond with a JSON object mapping tool names to categories, nothing else.
Example: {"ripgrep": "search", "bat": "files", "htop": "system"}

Tools to categorize:
{{TOOLS}}
"#;

const DEFAULT_DESCRIBE_PROMPT: &str = r#"Generate brief descriptions (max 100 chars each) for these CLI tools.
Only respond with a JSON object mapping tool names to descriptions, nothing else.
Example: {"ripgrep": "Fast regex search tool, replacement for grep", "bat": "Cat clone with syntax highlighting"}

Tools needing descriptions:
{{TOOLS}}
"#;

const DEFAULT_SUGGEST_BUNDLE_PROMPT: &str = r#"Analyze these CLI tools and suggest {{COUNT}} logical bundles (groups of related tools that work well together).
Consider tools that: share workflows, complement each other, or are commonly used together.

IMPORTANT: Do NOT suggest tools that are already in existing bundles (listed below).
{{EXISTING_BUNDLES}}

Only respond with a JSON array of objects with "name", "description", and "tools" fields.
Example: [{"name": "rust-dev", "description": "Rust development essentials", "tools": ["cargo-watch", "cargo-edit", "cargo-outdated"]}]

Available tools (not yet bundled):
{{TOOLS}}
"#;

// ==================== Prompt loading ====================

/// Get the prompts directory path
pub fn prompts_dir() -> Result<PathBuf> {
    let config_dir = dirs::config_dir()
        .context("Could not determine config directory")?
        .join("hoard")
        .join("prompts");
    Ok(config_dir)
}

/// Load a prompt template from file, falling back to embedded default
fn load_prompt(name: &str, default: &str) -> String {
    let path = match prompts_dir() {
        Ok(dir) => dir.join(format!("{}.txt", name)),
        Err(_) => return default.to_string(),
    };

    match std::fs::read_to_string(&path) {
        Ok(content) => content,
        Err(_) => default.to_string(),
    }
}

// ==================== AI invocation ====================

/// Invoke the configured AI provider with a prompt
pub fn invoke_ai(prompt: &str) -> Result<String> {
    let config = HoardConfig::load()?;
    let provider = &config.ai.provider;

    if *provider == AiProvider::None {
        bail!("No AI provider configured. Run 'hoard ai set <provider>' first.");
    }

    let cmd_name = provider
        .command()
        .context("Invalid AI provider configuration")?;

    if !provider.is_installed() {
        bail!(
            "AI provider '{}' is not installed. Please install it first.",
            cmd_name
        );
    }

    // Build the command based on provider
    let output = match provider {
        AiProvider::Claude => {
            // claude -p "prompt" for non-interactive mode
            Command::new(cmd_name)
                .arg("-p")
                .arg(prompt)
                .output()
                .context("Failed to execute claude")?
        }
        AiProvider::Gemini => {
            // gemini "prompt"
            Command::new(cmd_name)
                .arg(prompt)
                .output()
                .context("Failed to execute gemini")?
        }
        AiProvider::Codex => {
            // codex -q "prompt" for quiet mode
            Command::new(cmd_name)
                .arg("-q")
                .arg(prompt)
                .output()
                .context("Failed to execute codex")?
        }
        AiProvider::Opencode => {
            // opencode "prompt"
            Command::new(cmd_name)
                .arg(prompt)
                .output()
                .context("Failed to execute opencode")?
        }
        AiProvider::None => unreachable!(),
    };

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("AI command failed: {}", stderr);
    }

    let response = String::from_utf8_lossy(&output.stdout).to_string();
    Ok(response.trim().to_string())
}

// ==================== Categorize ====================

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

/// Parse categorization response from AI
pub fn parse_categorize_response(
    response: &str,
) -> Result<std::collections::HashMap<String, String>> {
    let json_str = extract_json_object(response)?;

    let map: std::collections::HashMap<String, String> =
        serde_json::from_str(&json_str).context("Failed to parse AI response as JSON")?;

    Ok(map)
}

// ==================== Describe ====================

/// Generate a prompt for describing tools
pub fn describe_prompt(tools: &[Tool]) -> String {
    let tool_list: Vec<String> = tools.iter().map(|t| format!("- {}", t.name)).collect();

    let template = load_prompt("describe", DEFAULT_DESCRIBE_PROMPT);
    template.replace("{{TOOLS}}", &tool_list.join("\n"))
}

/// Parse description response from AI
pub fn parse_describe_response(
    response: &str,
) -> Result<std::collections::HashMap<String, String>> {
    let json_str = extract_json_object(response)?;

    let map: std::collections::HashMap<String, String> =
        serde_json::from_str(&json_str).context("Failed to parse AI response as JSON")?;

    Ok(map)
}

// ==================== Suggest Bundle ====================

/// Bundle suggestion from AI
#[derive(Debug)]
pub struct BundleSuggestion {
    pub name: String,
    pub description: String,
    pub tools: Vec<String>,
}

/// Generate a prompt for bundle suggestions
pub fn suggest_bundle_prompt(tools: &[Tool], existing_bundles: &[Bundle], count: usize) -> String {
    // Collect all tools that are already in bundles
    let bundled_tools: std::collections::HashSet<&str> = existing_bundles
        .iter()
        .flat_map(|b| b.tools.iter().map(|s| s.as_str()))
        .collect();

    // Filter out already-bundled tools
    let unbundled_tools: Vec<&Tool> = tools
        .iter()
        .filter(|t| !bundled_tools.contains(t.name.as_str()))
        .collect();

    let tool_list: Vec<String> = unbundled_tools
        .iter()
        .map(|t| {
            let cat = t.category.as_deref().unwrap_or("uncategorized");
            let desc = t.description.as_deref().unwrap_or("");
            format!("- {} [{}]: {}", t.name, cat, desc)
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

/// Parse bundle suggestion response from AI
pub fn parse_bundle_response(response: &str) -> Result<Vec<BundleSuggestion>> {
    let json_str = extract_json_array(response)?;

    #[derive(serde::Deserialize)]
    struct RawSuggestion {
        name: String,
        description: String,
        tools: Vec<String>,
    }

    let raw: Vec<RawSuggestion> =
        serde_json::from_str(&json_str).context("Failed to parse AI response as JSON")?;

    Ok(raw
        .into_iter()
        .map(|r| BundleSuggestion {
            name: r.name,
            description: r.description,
            tools: r.tools,
        })
        .collect())
}

// ==================== JSON extraction helpers ====================

/// Extract a JSON object from a response that might contain extra text
fn extract_json_object(response: &str) -> Result<String> {
    let start = response
        .find('{')
        .context("No JSON object found in response")?;
    let end = response
        .rfind('}')
        .context("No closing brace found in response")?;

    if end <= start {
        bail!("Invalid JSON structure in response");
    }

    Ok(response[start..=end].to_string())
}

/// Extract a JSON array from a response that might contain extra text
fn extract_json_array(response: &str) -> Result<String> {
    let start = response
        .find('[')
        .context("No JSON array found in response")?;
    let end = response
        .rfind(']')
        .context("No closing bracket found in response")?;

    if end <= start {
        bail!("Invalid JSON structure in response");
    }

    Ok(response[start..=end].to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_json_object() {
        let response = r#"Here's the categorization:
{"ripgrep": "search", "bat": "files"}
Done!"#;
        let json = extract_json_object(response).unwrap();
        assert_eq!(json, r#"{"ripgrep": "search", "bat": "files"}"#);
    }

    #[test]
    fn test_extract_json_array() {
        let response = r#"Here are my suggestions:
[{"name": "test", "description": "desc", "tools": ["a", "b"]}]
"#;
        let json = extract_json_array(response).unwrap();
        assert!(json.starts_with('['));
        assert!(json.ends_with(']'));
    }

    #[test]
    fn test_default_prompts_have_placeholders() {
        assert!(DEFAULT_CATEGORIZE_PROMPT.contains("{{CATEGORIES}}"));
        assert!(DEFAULT_CATEGORIZE_PROMPT.contains("{{TOOLS}}"));
        assert!(DEFAULT_DESCRIBE_PROMPT.contains("{{TOOLS}}"));
        assert!(DEFAULT_SUGGEST_BUNDLE_PROMPT.contains("{{COUNT}}"));
        assert!(DEFAULT_SUGGEST_BUNDLE_PROMPT.contains("{{TOOLS}}"));
    }
}
