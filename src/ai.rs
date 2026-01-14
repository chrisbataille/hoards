//! AI provider integration for smart features
//!
//! Provides functions to invoke configured AI CLI tools (claude, gemini, codex, opencode)
//! and parse their responses for categorization, description generation, and bundle suggestions.
//!
//! Prompts are loaded from `~/.config/hoards/prompts/` and can be customized by the user.
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

const DEFAULT_EXTRACT_PROMPT: &str = r#"Extract CLI tool information from this GitHub README.

Return a JSON object with these fields:
- "name": tool name (required)
- "binary": binary name if different from tool name (optional, null if same)
- "source": installation source, one of: "cargo", "pip", "npm", "apt", "brew", "snap", "flatpak", "manual" (required)
- "install_command": the install command, e.g. "cargo install ripgrep" (optional)
- "description": brief description, max 100 chars (required)
- "category": suggested category from: dev, shell, files, search, git, network, system, editor, data, security, misc (required)

Example response:
{"name": "ripgrep", "binary": "rg", "source": "cargo", "install_command": "cargo install ripgrep", "description": "Fast regex search tool, replacement for grep", "category": "search"}

README content:
{{README}}
"#;

// ==================== Prompt loading ====================

/// Get the prompts directory path
pub fn prompts_dir() -> Result<PathBuf> {
    let config_dir = dirs::config_dir()
        .context("Could not determine config directory")?
        .join("hoards")
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
        bail!("No AI provider configured. Run 'hoards ai set <provider>' first.");
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

// ==================== Extract ====================

/// Extracted tool information from a GitHub README
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ExtractedTool {
    pub name: String,
    pub binary: Option<String>,
    pub source: String,
    pub install_command: Option<String>,
    pub description: String,
    pub category: String,
}

/// Cache entry for extracted tool info
#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct ExtractCache {
    /// Repository owner/name
    repo: String,
    /// Commit SHA or tag at time of extraction
    version: String,
    /// Extracted tool info
    tool: ExtractedTool,
    /// Timestamp of extraction
    extracted_at: String,
}

/// Get the cache directory for extractions
fn extract_cache_dir() -> Result<PathBuf> {
    let cache_dir = dirs::cache_dir()
        .context("Could not determine cache directory")?
        .join("hoards")
        .join("extractions");
    std::fs::create_dir_all(&cache_dir)?;
    Ok(cache_dir)
}

/// Get cache file path for a repo
fn cache_path_for_repo(owner: &str, repo: &str) -> Result<PathBuf> {
    let cache_dir = extract_cache_dir()?;
    Ok(cache_dir.join(format!("{}_{}.json", owner, repo)))
}

/// Check cache for an extraction
pub fn get_cached_extraction(owner: &str, repo: &str, version: &str) -> Option<ExtractedTool> {
    let path = cache_path_for_repo(owner, repo).ok()?;
    let content = std::fs::read_to_string(&path).ok()?;
    let cache: ExtractCache = serde_json::from_str(&content).ok()?;

    // Only return if version matches
    if cache.version == version {
        Some(cache.tool)
    } else {
        None
    }
}

/// Save extraction to cache
pub fn cache_extraction(
    owner: &str,
    repo: &str,
    version: &str,
    tool: &ExtractedTool,
) -> Result<()> {
    let path = cache_path_for_repo(owner, repo)?;
    let cache = ExtractCache {
        repo: format!("{}/{}", owner, repo),
        version: version.to_string(),
        tool: tool.clone(),
        extracted_at: chrono::Utc::now().to_rfc3339(),
    };
    let content = serde_json::to_string_pretty(&cache)?;
    std::fs::write(&path, content)?;
    Ok(())
}

/// Parse a GitHub URL to extract owner and repo
pub fn parse_github_url(url: &str) -> Result<(String, String)> {
    // Handle various GitHub URL formats:
    // https://github.com/owner/repo
    // https://github.com/owner/repo.git
    // https://github.com/owner/repo/...
    // git@github.com:owner/repo.git
    // owner/repo (shorthand)

    let url = url.trim();

    // Shorthand format: owner/repo
    if !url.contains("github.com") && url.contains('/') && !url.contains(':') {
        let parts: Vec<&str> = url.split('/').collect();
        if parts.len() >= 2 {
            return Ok((
                parts[0].to_string(),
                parts[1].trim_end_matches(".git").to_string(),
            ));
        }
    }

    // SSH format: git@github.com:owner/repo.git
    if url.starts_with("git@github.com:") {
        let path = url.trim_start_matches("git@github.com:");
        let parts: Vec<&str> = path.split('/').collect();
        if parts.len() >= 2 {
            return Ok((
                parts[0].to_string(),
                parts[1].trim_end_matches(".git").to_string(),
            ));
        }
    }

    // HTTPS format
    if let Some(path) = url
        .strip_prefix("https://github.com/")
        .or_else(|| url.strip_prefix("http://github.com/"))
    {
        let parts: Vec<&str> = path.split('/').collect();
        if parts.len() >= 2 {
            return Ok((
                parts[0].to_string(),
                parts[1].trim_end_matches(".git").to_string(),
            ));
        }
    }

    bail!("Invalid GitHub URL format: {}", url)
}

/// Fetch README content from GitHub using gh CLI
pub fn fetch_readme(owner: &str, repo: &str) -> Result<String> {
    let output = Command::new("gh")
        .args(["api", &format!("repos/{}/{}/readme", owner, repo)])
        .output()
        .context("Failed to run gh api")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("Failed to fetch README: {}", stderr);
    }

    #[derive(serde::Deserialize)]
    struct ReadmeResponse {
        content: String,
        encoding: String,
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let readme: ReadmeResponse =
        serde_json::from_str(&stdout).context("Failed to parse README response")?;

    if readme.encoding != "base64" {
        bail!("Unexpected README encoding: {}", readme.encoding);
    }

    // Decode base64 content
    use base64::{Engine as _, engine::general_purpose};
    let decoded = general_purpose::STANDARD
        .decode(readme.content.replace('\n', ""))
        .context("Failed to decode README content")?;

    String::from_utf8(decoded).context("README is not valid UTF-8")
}

/// Fetch the latest commit SHA for a repo (used for cache versioning)
pub fn fetch_repo_version(owner: &str, repo: &str) -> Result<String> {
    let output = Command::new("gh")
        .args([
            "api",
            &format!("repos/{}/{}/commits/HEAD", owner, repo),
            "--jq",
            ".sha",
        ])
        .output()
        .context("Failed to run gh api")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("Failed to fetch repo version: {}", stderr);
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

/// Generate extraction prompt
pub fn extract_prompt(readme: &str) -> String {
    // Truncate README if too long (keep first ~8000 chars to leave room for prompt)
    let readme_truncated = if readme.len() > 8000 {
        format!("{}...\n[README truncated]", &readme[..8000])
    } else {
        readme.to_string()
    };

    let template = load_prompt("extract", DEFAULT_EXTRACT_PROMPT);
    template.replace("{{README}}", &readme_truncated)
}

/// Parse extraction response from AI
pub fn parse_extract_response(response: &str) -> Result<ExtractedTool> {
    let json_str = extract_json_object(response)?;

    let tool: ExtractedTool =
        serde_json::from_str(&json_str).context("Failed to parse AI extraction response")?;

    // Validate required fields
    if tool.name.is_empty() {
        bail!("Extracted tool has no name");
    }
    if tool.description.is_empty() {
        bail!("Extracted tool has no description");
    }

    Ok(tool)
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
        assert!(DEFAULT_EXTRACT_PROMPT.contains("{{README}}"));
    }

    #[test]
    fn test_parse_github_url_https() {
        let (owner, repo) = parse_github_url("https://github.com/BurntSushi/ripgrep").unwrap();
        assert_eq!(owner, "BurntSushi");
        assert_eq!(repo, "ripgrep");
    }

    #[test]
    fn test_parse_github_url_https_with_git() {
        let (owner, repo) = parse_github_url("https://github.com/BurntSushi/ripgrep.git").unwrap();
        assert_eq!(owner, "BurntSushi");
        assert_eq!(repo, "ripgrep");
    }

    #[test]
    fn test_parse_github_url_https_with_path() {
        let (owner, repo) =
            parse_github_url("https://github.com/BurntSushi/ripgrep/tree/master").unwrap();
        assert_eq!(owner, "BurntSushi");
        assert_eq!(repo, "ripgrep");
    }

    #[test]
    fn test_parse_github_url_ssh() {
        let (owner, repo) = parse_github_url("git@github.com:BurntSushi/ripgrep.git").unwrap();
        assert_eq!(owner, "BurntSushi");
        assert_eq!(repo, "ripgrep");
    }

    #[test]
    fn test_parse_github_url_shorthand() {
        let (owner, repo) = parse_github_url("BurntSushi/ripgrep").unwrap();
        assert_eq!(owner, "BurntSushi");
        assert_eq!(repo, "ripgrep");
    }

    #[test]
    fn test_parse_github_url_invalid() {
        assert!(parse_github_url("not-a-url").is_err());
        assert!(parse_github_url("https://gitlab.com/foo/bar").is_err());
    }

    #[test]
    fn test_parse_extract_response() {
        let response = r#"Here's the extracted info:
{"name": "ripgrep", "binary": "rg", "source": "cargo", "install_command": "cargo install ripgrep", "description": "Fast regex search", "category": "search"}
"#;
        let tool = parse_extract_response(response).unwrap();
        assert_eq!(tool.name, "ripgrep");
        assert_eq!(tool.binary, Some("rg".to_string()));
        assert_eq!(tool.source, "cargo");
        assert_eq!(tool.category, "search");
    }

    #[test]
    fn test_parse_extract_response_minimal() {
        let response =
            r#"{"name": "foo", "source": "pip", "description": "A tool", "category": "misc"}"#;
        let tool = parse_extract_response(response).unwrap();
        assert_eq!(tool.name, "foo");
        assert_eq!(tool.binary, None);
        assert_eq!(tool.install_command, None);
    }

    #[test]
    fn test_extract_prompt_truncates_long_readme() {
        let long_readme = "x".repeat(10000);
        let prompt = extract_prompt(&long_readme);
        assert!(prompt.contains("[README truncated]"));
        assert!(prompt.len() < 10000);
    }
}
