//! GitHub integration for fetching repo info, topics, and descriptions
//!
//! Uses the `gh` CLI to query GitHub's API for repository information.
//! Includes rate limit awareness to avoid hitting GitHub API limits.

use anyhow::{bail, Context, Result};
use serde::Deserialize;
use std::process::Command;

/// GitHub API rate limit info
#[derive(Debug, Clone, Deserialize)]
pub struct RateLimit {
    pub limit: i64,
    pub remaining: i64,
    pub reset: i64,
    pub used: i64,
}

impl RateLimit {
    /// Minutes until rate limit resets
    pub fn reset_minutes(&self) -> i64 {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);
        ((self.reset - now) / 60).max(0)
    }

    /// Seconds until rate limit resets
    pub fn reset_seconds(&self) -> i64 {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);
        (self.reset - now).max(0)
    }

    /// Check if we have enough remaining calls
    pub fn has_remaining(&self, needed: i64) -> bool {
        self.remaining >= needed
    }
}

/// Combined rate limit info (core + search)
#[derive(Debug, Clone)]
pub struct RateLimits {
    pub core: RateLimit,
    pub search: RateLimit,
}

/// Get current GitHub API rate limit status (core API - 5000/hour)
pub fn get_rate_limit() -> Result<RateLimit> {
    let output = Command::new("gh")
        .args(["api", "rate_limit", "--jq", ".rate"])
        .output()
        .context("Failed to run gh api rate_limit")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("gh api rate_limit failed: {}", stderr);
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let rate: RateLimit = serde_json::from_str(&stdout)
        .context("Failed to parse rate limit response")?;

    Ok(rate)
}

/// Get Search API rate limit (30/minute - stricter!)
pub fn get_search_rate_limit() -> Result<RateLimit> {
    let output = Command::new("gh")
        .args(["api", "rate_limit", "--jq", ".resources.search"])
        .output()
        .context("Failed to run gh api rate_limit")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("gh api rate_limit failed: {}", stderr);
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let rate: RateLimit = serde_json::from_str(&stdout)
        .context("Failed to parse search rate limit response")?;

    Ok(rate)
}

/// Get both core and search rate limits
pub fn get_all_rate_limits() -> Result<RateLimits> {
    let core = get_rate_limit()?;
    let search = get_search_rate_limit()?;
    Ok(RateLimits { core, search })
}

/// Repository info from GitHub
#[derive(Debug, Clone, Deserialize)]
pub struct RepoInfo {
    pub name: String,
    pub full_name: String,
    pub description: Option<String>,
    #[serde(rename = "stargazersCount")]
    pub stars: i64,
    pub language: Option<String>,
    pub homepage: Option<String>,
    pub topics: Vec<String>,
    pub owner: RepoOwner,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RepoOwner {
    pub login: String,
}

/// Search result from GitHub
#[derive(Debug, Clone, Deserialize)]
pub struct SearchResult {
    pub name: String,
    #[serde(rename = "fullName")]
    pub full_name: String,
    pub description: Option<String>,
    #[serde(rename = "stargazersCount")]
    pub stars: i64,
    pub owner: SearchOwner,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SearchOwner {
    pub login: String,
}

/// Check if `gh` CLI is available
pub fn is_gh_available() -> bool {
    Command::new("gh")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Map installation source to GitHub language filter for better search accuracy
pub fn source_to_language_filter(source: Option<&str>) -> Option<&'static str> {
    match source {
        Some("cargo") => Some("language:rust"),
        Some("pip") => Some("language:python"),
        Some("npm") => Some("language:javascript OR language:typescript"),
        Some("go") => Some("language:go"),
        _ => None,
    }
}

/// Search GitHub for a repository by name, optionally using source for language filtering
pub fn search_repo(name: &str, source: Option<&str>) -> Result<Option<SearchResult>> {
    // Build search query with language filter based on installation source
    let query = match source_to_language_filter(source) {
        Some(lang_filter) => format!("{} {}", name, lang_filter),
        None => name.to_string(),
    };

    let output = Command::new("gh")
        .args([
            "search",
            "repos",
            &query,
            "--json",
            "name,fullName,description,stargazersCount,owner",
            "--limit",
            "1",
        ])
        .output()
        .context("Failed to run gh search")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("gh search failed: {}", stderr);
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let results: Vec<SearchResult> = serde_json::from_str(&stdout)
        .context("Failed to parse gh search output")?;

    Ok(results.into_iter().next())
}

/// Get detailed repo info including topics
pub fn get_repo_info(owner: &str, repo: &str) -> Result<RepoInfo> {
    let output = Command::new("gh")
        .args([
            "api",
            &format!("repos/{}/{}", owner, repo),
            "--jq",
            r#"{name, full_name: .full_name, description, stargazersCount: .stargazers_count, language, homepage, topics, owner: {login: .owner.login}}"#,
        ])
        .output()
        .context("Failed to run gh api")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("gh api failed: {}", stderr);
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let info: RepoInfo = serde_json::from_str(&stdout)
        .context("Failed to parse gh api output")?;

    Ok(info)
}

/// Search and get full repo info for a tool, using source for language filtering
pub fn find_repo(tool_name: &str, source: Option<&str>) -> Result<Option<RepoInfo>> {
    // First search for the repo, using language filter based on source
    let search_result = search_repo(tool_name, source)?;

    match search_result {
        Some(result) => {
            // Get full info including topics
            let info = get_repo_info(&result.owner.login, &result.name)?;
            Ok(Some(info))
        }
        None => Ok(None),
    }
}

/// Map GitHub topics to a category using the mapping config
pub fn topics_to_category(topics: &[String], mapping: &TopicMapping) -> Option<String> {
    // Count matches for each category
    let mut scores: std::collections::HashMap<&str, usize> = std::collections::HashMap::new();

    for topic in topics {
        let topic_lower = topic.to_lowercase();
        for (category, keywords) in &mapping.categories {
            if keywords.contains(&topic_lower) {
                *scores.entry(category.as_str()).or_insert(0) += 1;
            }
        }
    }

    // Return category with highest score
    scores
        .into_iter()
        .max_by_key(|(_, score)| *score)
        .map(|(cat, _)| cat.to_string())
}

/// Topic to category mapping configuration
#[derive(Debug, Clone, Default)]
pub struct TopicMapping {
    pub categories: std::collections::HashMap<String, Vec<String>>,
}

impl TopicMapping {
    /// Load mapping from TOML file or use defaults
    pub fn load() -> Self {
        let config_path = dirs::config_dir()
            .map(|d| d.join("hoard").join("topic-mapping.toml"));

        if let Some(path) = config_path
            && path.exists()
                && let Ok(content) = std::fs::read_to_string(&path)
                    && let Ok(mapping) = Self::parse_toml(&content) {
                        return mapping;
                    }

        Self::default_mapping()
    }

    fn parse_toml(content: &str) -> Result<Self> {
        #[derive(Deserialize)]
        struct TomlMapping {
            categories: std::collections::HashMap<String, Vec<String>>,
        }

        let parsed: TomlMapping = toml::from_str(content)?;
        Ok(Self {
            categories: parsed.categories,
        })
    }

    /// Default topic to category mapping
    pub fn default_mapping() -> Self {
        let mut categories = std::collections::HashMap::new();

        categories.insert("search".to_string(), vec![
            "search", "grep", "regex", "find", "ripgrep", "ag", "ack",
        ].into_iter().map(String::from).collect());

        categories.insert("files".to_string(), vec![
            "files", "filesystem", "ls", "file-manager", "directory", "tree", "disk",
        ].into_iter().map(String::from).collect());

        categories.insert("git".to_string(), vec![
            "git", "github", "gitlab", "version-control", "vcs",
        ].into_iter().map(String::from).collect());

        categories.insert("shell".to_string(), vec![
            "shell", "terminal", "cli", "command-line", "bash", "zsh", "fish",
            "prompt", "readline",
        ].into_iter().map(String::from).collect());

        categories.insert("container".to_string(), vec![
            "docker", "container", "kubernetes", "k8s", "podman", "oci",
        ].into_iter().map(String::from).collect());

        categories.insert("editor".to_string(), vec![
            "editor", "vim", "neovim", "emacs", "text-editor", "ide",
        ].into_iter().map(String::from).collect());

        categories.insert("network".to_string(), vec![
            "network", "http", "curl", "api", "rest", "web", "dns", "proxy",
        ].into_iter().map(String::from).collect());

        categories.insert("data".to_string(), vec![
            "json", "yaml", "csv", "jq", "data", "parsing", "xml", "toml",
        ].into_iter().map(String::from).collect());

        categories.insert("system".to_string(), vec![
            "system", "process", "monitoring", "htop", "top", "performance",
            "benchmark", "profiling",
        ].into_iter().map(String::from).collect());

        categories.insert("security".to_string(), vec![
            "security", "encryption", "password", "ssh", "gpg", "crypto",
            "vault", "secrets",
        ].into_iter().map(String::from).collect());

        categories.insert("dev".to_string(), vec![
            "development", "programming", "compiler", "linter", "formatter",
            "testing", "debugging", "build",
        ].into_iter().map(String::from).collect());

        Self { categories }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_topics_to_category() {
        let mapping = TopicMapping::default_mapping();

        // "search" and "grep" both map to "search" category (2 points vs 1 for cli->shell)
        let topics = vec!["cli".to_string(), "search".to_string(), "grep".to_string()];
        assert_eq!(topics_to_category(&topics, &mapping), Some("search".to_string()));

        // "git" and "github" both map to "git" category (2 points vs 1 for cli->shell)
        let topics = vec!["git".to_string(), "github".to_string(), "cli".to_string()];
        assert_eq!(topics_to_category(&topics, &mapping), Some("git".to_string()));

        // No matching topics
        let topics = vec!["unknown".to_string(), "random".to_string()];
        assert_eq!(topics_to_category(&topics, &mapping), None);

        // Empty topics
        let topics: Vec<String> = vec![];
        assert_eq!(topics_to_category(&topics, &mapping), None);
    }

    #[test]
    fn test_source_to_language_filter() {
        assert_eq!(source_to_language_filter(Some("cargo")), Some("language:rust"));
        assert_eq!(source_to_language_filter(Some("pip")), Some("language:python"));
        assert_eq!(source_to_language_filter(Some("npm")), Some("language:javascript OR language:typescript"));
        assert_eq!(source_to_language_filter(Some("go")), Some("language:go"));
        assert_eq!(source_to_language_filter(Some("apt")), None);
        assert_eq!(source_to_language_filter(None), None);
    }
}
