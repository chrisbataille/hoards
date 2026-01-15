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
use serde::{Deserialize, Serialize};
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

const DEFAULT_SUGGEST_BUNDLE_PROMPT: &str = r#"Analyze this user's CLI tools and suggest {{COUNT}} logical bundles based on their ACTUAL USAGE PATTERNS.

Guidelines:
1. PRIORITIZE tools the user actually uses (higher usage count = more important)
2. Group tools that share workflows or complement each other
3. Each bundle should tell a story (e.g., "Modern Unix", "Git Power Tools", "Rust Development")
4. Include 3-6 tools per bundle for practical utility
5. Focus on installed tools with usage > 0 when possible

IMPORTANT: Do NOT suggest tools that are already in existing bundles:
{{EXISTING_BUNDLES}}

Respond ONLY with a JSON array. Each object must have:
- "name": short bundle name (kebab-case, e.g., "modern-unix")
- "description": one-line description explaining the theme
- "tools": array of tool names from the list below
- "reasoning": brief explanation of why these tools belong together

Example:
[{"name": "modern-unix", "description": "Modern replacements for traditional Unix tools", "tools": ["ripgrep", "fd", "eza", "bat"], "reasoning": "User heavily uses ripgrep (847x) and fd (423x), suggesting preference for modern alternatives"}]

Available tools with usage data (format: name [category] (usage count): description):
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

const DEFAULT_CHEATSHEET_PROMPT: &str = r#"Create a concise CLI cheatsheet for the tool "{{TOOL_NAME}}" based on its --help output.

Guidelines:
1. Group commands by category (BASIC USAGE, FILE FILTERING, OUTPUT, etc.)
2. Show the most useful/common commands first
3. Keep descriptions very short (2-4 words max)
4. Include 3-5 categories with 3-5 commands each
5. Use the actual binary name in examples

Respond with JSON:
{
  "title": "tool-name (binary) - Short description",
  "sections": [
    {
      "name": "CATEGORY NAME",
      "commands": [
        {"cmd": "binary -flag arg", "desc": "Brief description"}
      ]
    }
  ]
}

Tool --help output:
{{HELP_OUTPUT}}
"#;

const DEFAULT_BUNDLE_CHEATSHEET_PROMPT: &str = r#"Create a workflow-oriented cheatsheet for a bundle of related CLI tools.

IMPORTANT: Organize by WORKFLOW/TASK, not by individual tool. Group related commands from different tools together based on what task they accomplish.

Bundle name: {{BUNDLE_NAME}}
Tools in bundle: {{TOOL_LIST}}

Guidelines:
1. Create categories based on workflows (e.g., "PROJECT SETUP", "DAILY WORKFLOW", "CODE QUALITY", "DEBUGGING")
2. Mix commands from different tools when they relate to the same workflow
3. Show the most common workflow patterns first
4. Keep descriptions very short (2-4 words max)
5. Include 4-6 categories with 3-6 commands each
6. Prefix commands with the tool name if ambiguous

Respond with JSON:
{
  "title": "Bundle Name - Workflow description",
  "sections": [
    {
      "name": "WORKFLOW CATEGORY",
      "commands": [
        {"cmd": "tool command -flag", "desc": "Brief description"}
      ]
    }
  ]
}

Tool help outputs:
{{HELP_OUTPUTS}}
"#;

const DEFAULT_DISCOVERY_PROMPT: &str = r#"You are a CLI tool expert. Based on the user's description of what they're working on, recommend relevant command-line tools.

User's context: {{QUERY}}

Already installed tools: {{INSTALLED_TOOLS}}

Guidelines:
1. Recommend 5-10 highly relevant tools
2. Categorize as "essential" (must-have) or "recommended" (nice-to-have)
3. Don't recommend tools they already have installed
4. Focus on well-maintained, popular tools
5. Include the exact install command for each
6. Be specific about why each tool is relevant

Respond with JSON:
{
  "summary": "Brief description of the recommendations",
  "tools": [
    {
      "name": "tool-name",
      "binary": "binary-name",
      "description": "What it does (1 sentence)",
      "category": "essential|recommended",
      "reason": "Why it's relevant to their query",
      "source": "cargo|pip|npm|apt|brew",
      "install_cmd": "cargo install tool-name",
      "github": "owner/repo"
    }
  ]
}
"#;

const DEFAULT_ANALYZE_PROMPT: &str = r#"Analyze this CLI usage data and provide a brief personalized insight.

Traditional tool usage (from shell history):
{{TRADITIONAL_USAGE}}

Modern replacement tools installed:
{{MODERN_TOOLS}}

Unused installed tools with high potential:
{{UNUSED_TOOLS}}

Provide a brief (2-3 sentence) personalized insight about:
1. The user's apparent workflow patterns
2. Which specific unused tools would benefit them most based on their usage

Respond with JSON:
{"insight": "Your personalized analysis here"}
"#;

const DEFAULT_MIGRATE_PROMPT: &str = r#"For each tool being migrated between package sources, provide a brief benefit description (5-10 words) explaining why the newer version is better.

Tools being migrated:
{{TOOLS}}

For each tool, explain the key improvement in the newer version (e.g., new features, performance improvements, bug fixes).

Respond with JSON:
{"benefits": {"tool_name": "brief benefit description", ...}}
"#;

// ==================== Modern tool replacements ====================

/// A mapping from a traditional Unix tool to its modern replacement
#[derive(Debug, Clone)]
pub struct ToolReplacement {
    /// Traditional tool name (e.g., "grep")
    pub traditional: &'static str,
    /// Modern replacement tool name (e.g., "ripgrep")
    pub modern: &'static str,
    /// Binary name of the modern tool (e.g., "rg")
    pub modern_binary: &'static str,
    /// Suggested action/alias (e.g., "alias grep='rg'")
    pub tip: &'static str,
    /// Benefit description (e.g., "10x faster")
    pub benefit: &'static str,
}

/// Known mappings of traditional tools to modern replacements
pub const MODERN_REPLACEMENTS: &[ToolReplacement] = &[
    ToolReplacement {
        traditional: "grep",
        modern: "ripgrep",
        modern_binary: "rg",
        tip: "alias grep='rg'",
        benefit: "10x faster regex search",
    },
    ToolReplacement {
        traditional: "find",
        modern: "fd",
        modern_binary: "fd",
        tip: "fd <pattern>",
        benefit: "5x faster, simpler syntax",
    },
    ToolReplacement {
        traditional: "cat",
        modern: "bat",
        modern_binary: "bat",
        tip: "alias cat='bat'",
        benefit: "syntax highlighting, git integration",
    },
    ToolReplacement {
        traditional: "ls",
        modern: "eza",
        modern_binary: "eza",
        tip: "alias ls='eza'",
        benefit: "git status, icons, better colors",
    },
    ToolReplacement {
        traditional: "du",
        modern: "dust",
        modern_binary: "dust",
        tip: "dust",
        benefit: "intuitive visual output",
    },
    ToolReplacement {
        traditional: "df",
        modern: "duf",
        modern_binary: "duf",
        tip: "duf",
        benefit: "better formatting, colors",
    },
    ToolReplacement {
        traditional: "ps",
        modern: "procs",
        modern_binary: "procs",
        tip: "procs",
        benefit: "structured output, colors",
    },
    ToolReplacement {
        traditional: "top",
        modern: "btop",
        modern_binary: "btop",
        tip: "btop",
        benefit: "interactive TUI, resource graphs",
    },
    ToolReplacement {
        traditional: "htop",
        modern: "btop",
        modern_binary: "btop",
        tip: "btop",
        benefit: "more visual, better resource graphs",
    },
    ToolReplacement {
        traditional: "sed",
        modern: "sd",
        modern_binary: "sd",
        tip: "sd 'old' 'new' file",
        benefit: "simpler syntax, no escaping",
    },
    ToolReplacement {
        traditional: "diff",
        modern: "delta",
        modern_binary: "delta",
        tip: "git config core.pager delta",
        benefit: "syntax highlighting, side-by-side",
    },
    ToolReplacement {
        traditional: "man",
        modern: "tldr",
        modern_binary: "tldr",
        tip: "tldr <command>",
        benefit: "practical examples, concise",
    },
    ToolReplacement {
        traditional: "curl",
        modern: "xh",
        modern_binary: "xh",
        tip: "xh httpbin.org/get",
        benefit: "cleaner output, easier syntax",
    },
    ToolReplacement {
        traditional: "cut",
        modern: "choose",
        modern_binary: "choose",
        tip: "choose -f 1,3",
        benefit: "human-friendly field selection",
    },
    ToolReplacement {
        traditional: "ping",
        modern: "gping",
        modern_binary: "gping",
        tip: "gping google.com",
        benefit: "graphical ping visualization",
    },
];

// ==================== Analyze types ====================

/// An optimization tip from usage analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalyzeTip {
    pub traditional: String,
    pub traditional_uses: i64,
    pub modern: String,
    pub modern_binary: String,
    pub benefit: String,
    pub action: String,
}

/// An underutilized installed tool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnderutilizedTool {
    pub name: String,
    pub description: Option<String>,
    pub stars: Option<u64>,
}

/// Result of usage analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisResult {
    pub tips: Vec<AnalyzeTip>,
    pub underutilized: Vec<UnderutilizedTool>,
    pub ai_insight: Option<String>,
}

/// A tool that can be migrated to a different source
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationCandidate {
    pub name: String,
    pub from_source: String,
    pub from_version: String,
    pub to_source: String,
    pub to_version: String,
    pub to_package_name: String, // Package name on target source (may differ)
    pub benefit: Option<String>, // AI-generated
}

/// Result of migration analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationResult {
    pub candidates: Vec<MigrationCandidate>,
    pub ai_summary: Option<String>,
}

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
    pub reasoning: Option<String>,
}

/// Generate a prompt for bundle suggestions with usage data
pub fn suggest_bundle_prompt(
    tools: &[Tool],
    existing_bundles: &[Bundle],
    usage_data: &std::collections::HashMap<String, i64>,
    count: usize,
) -> String {
    // Collect all tools that are already in bundles
    let bundled_tools: std::collections::HashSet<&str> = existing_bundles
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

/// Parse bundle suggestion response from AI
pub fn parse_bundle_response(response: &str) -> Result<Vec<BundleSuggestion>> {
    let json_str = extract_json_array(response)?;

    #[derive(serde::Deserialize)]
    struct RawSuggestion {
        name: String,
        description: String,
        tools: Vec<String>,
        reasoning: Option<String>,
    }

    let raw: Vec<RawSuggestion> =
        serde_json::from_str(&json_str).context("Failed to parse AI response as JSON")?;

    Ok(raw
        .into_iter()
        .map(|r| BundleSuggestion {
            name: r.name,
            description: r.description,
            tools: r.tools,
            reasoning: r.reasoning,
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

// ==================== Cheatsheet ====================

/// A command in a cheatsheet section
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CheatsheetCommand {
    pub cmd: String,
    pub desc: String,
}

/// A section in a cheatsheet
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CheatsheetSection {
    pub name: String,
    pub commands: Vec<CheatsheetCommand>,
}

/// Generated cheatsheet for a tool
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Cheatsheet {
    pub title: String,
    pub sections: Vec<CheatsheetSection>,
}

/// Cached cheatsheet with version info for invalidation
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CachedCheatsheet {
    pub version: Option<String>,
    pub cheatsheet: Cheatsheet,
}

// ==================== Discovery types ====================

/// A tool recommendation from AI discovery
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ToolRecommendation {
    pub name: String,
    #[serde(default)]
    pub binary: Option<String>,
    pub description: String,
    pub category: String, // "essential" or "recommended"
    pub reason: String,
    pub source: String,
    pub install_cmd: String,
    #[serde(default)]
    pub github: Option<String>,
    #[serde(skip)]
    pub stars: Option<u64>,
    #[serde(skip)]
    pub installed: bool,
}

/// Discovery response from AI
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DiscoveryResponse {
    pub summary: String,
    pub tools: Vec<ToolRecommendation>,
}

/// Get tool version by running `tool --version`
pub fn get_tool_version(binary: &str) -> Option<String> {
    use std::process::Command;

    let output = Command::new(binary).arg("--version").output().ok()?;

    if output.status.success() {
        let version = String::from_utf8_lossy(&output.stdout);
        let version = version.trim();
        if !version.is_empty() {
            // Extract just the version number if possible (first line, cleaned up)
            let first_line = version.lines().next().unwrap_or(version);
            return Some(first_line.to_string());
        }
    }

    None
}

/// Generate a cheatsheet prompt from --help output
pub fn cheatsheet_prompt(tool_name: &str, help_output: &str) -> String {
    let template = load_prompt("cheatsheet", DEFAULT_CHEATSHEET_PROMPT);

    // Truncate help output if too long (keep first 4000 chars)
    let truncated_help = if help_output.len() > 4000 {
        format!("{}...\n[truncated]", &help_output[..4000])
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
            combined_help.push_str(&format!("{}...\n[truncated]\n", &help[..2000]));
        } else {
            combined_help.push_str(help);
        }
    }

    // Overall truncation if still too long
    let final_help = if combined_help.len() > 12000 {
        format!("{}...\n[truncated]", &combined_help[..12000])
    } else {
        combined_help
    };

    template
        .replace("{{BUNDLE_NAME}}", bundle_name)
        .replace("{{TOOL_LIST}}", &tool_list)
        .replace("{{HELP_OUTPUTS}}", &final_help)
}

/// Generate a discovery prompt from user query and context
pub fn discovery_prompt(query: &str, installed_tools: &[String]) -> String {
    let template = load_prompt("discovery", DEFAULT_DISCOVERY_PROMPT);

    let installed_list = if installed_tools.is_empty() {
        "None".to_string()
    } else {
        installed_tools.join(", ")
    };

    template
        .replace("{{QUERY}}", query)
        .replace("{{INSTALLED_TOOLS}}", &installed_list)
}

/// Parse discovery response from AI
pub fn parse_discovery_response(response: &str) -> Result<DiscoveryResponse> {
    let json_str = extract_json_object(response)?;
    let discovery: DiscoveryResponse =
        serde_json::from_str(&json_str).context("Failed to parse discovery response")?;
    Ok(discovery)
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

/// Parse analyze insight response from AI
pub fn parse_analyze_response(response: &str) -> Result<String> {
    let json_str = extract_json_object(response)?;

    #[derive(serde::Deserialize)]
    struct AnalyzeInsight {
        insight: String,
    }

    let insight: AnalyzeInsight =
        serde_json::from_str(&json_str).context("Failed to parse analyze response")?;
    Ok(insight.insight)
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

/// Parse migration benefits response from AI
pub fn parse_migrate_response(response: &str) -> Result<std::collections::HashMap<String, String>> {
    let json_str = extract_json_object(response)?;

    #[derive(serde::Deserialize)]
    struct MigrateBenefits {
        benefits: std::collections::HashMap<String, String>,
    }

    let result: MigrateBenefits =
        serde_json::from_str(&json_str).context("Failed to parse migrate response")?;
    Ok(result.benefits)
}

/// Check if a binary is installed on the system
pub fn is_binary_installed(binary: &str) -> bool {
    which::which(binary).is_ok()
}

/// Parse cheatsheet response from AI
pub fn parse_cheatsheet_response(response: &str) -> Result<Cheatsheet> {
    let json_str = extract_json_object(response)?;
    let cheatsheet: Cheatsheet =
        serde_json::from_str(&json_str).context("Failed to parse AI cheatsheet response")?;
    Ok(cheatsheet)
}

/// Get --help output for a tool
pub fn get_help_output(binary: &str) -> Result<String> {
    use std::process::Command;

    // Try --help first, then -h
    let output = Command::new(binary)
        .arg("--help")
        .output()
        .or_else(|_| Command::new(binary).arg("-h").output())
        .with_context(|| format!("Failed to run {} --help", binary))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    // Some tools output help to stderr
    let help_text = if stdout.len() > stderr.len() {
        stdout.to_string()
    } else {
        stderr.to_string()
    };

    if help_text.trim().is_empty() {
        bail!("No help output from {}", binary);
    }

    Ok(help_text)
}

/// Format a cheatsheet for terminal display using comfy-table
pub fn format_cheatsheet(cheatsheet: &Cheatsheet) -> String {
    use comfy_table::{
        Attribute, Cell, Color, ContentArrangement, Table, modifiers::UTF8_ROUND_CORNERS,
        presets::UTF8_FULL,
    };

    let mut output = Vec::new();

    // Create title table
    let mut title_table = Table::new();
    title_table
        .load_preset(UTF8_FULL)
        .apply_modifier(UTF8_ROUND_CORNERS)
        .set_content_arrangement(ContentArrangement::Dynamic)
        .set_width(72);

    title_table.add_row(vec![
        Cell::new(&cheatsheet.title)
            .add_attribute(Attribute::Bold)
            .fg(Color::Cyan),
    ]);

    output.push(title_table.to_string());
    output.push(String::new());

    // Create a table for each section
    for section in &cheatsheet.sections {
        let mut table = Table::new();
        table
            .load_preset(UTF8_FULL)
            .apply_modifier(UTF8_ROUND_CORNERS)
            .set_content_arrangement(ContentArrangement::Dynamic)
            .set_width(72);

        // Section header
        table.set_header(vec![
            Cell::new(&section.name)
                .add_attribute(Attribute::Bold)
                .fg(Color::Green),
            Cell::new(""),
        ]);

        // Commands
        for cmd in &section.commands {
            table.add_row(vec![
                Cell::new(&cmd.cmd).fg(Color::Yellow),
                Cell::new(&cmd.desc),
            ]);
        }

        output.push(table.to_string());
        output.push(String::new());
    }

    // Remove trailing empty line
    if output.last().map(|s| s.is_empty()).unwrap_or(false) {
        output.pop();
    }

    output.join("\n")
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
