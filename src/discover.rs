//! External search functionality for the Discover tab
//!
//! This module provides trait-based search sources for discovering tools
//! from various package registries, GitHub, and AI recommendations.

use std::collections::HashSet;
use std::process::Command;

use anyhow::{Context, Result};

use crate::config::HoardConfig;
use crate::http::HTTP_AGENT;
use crate::tui::DiscoverSource;

/// An install option for a discovered tool
#[derive(Debug, Clone)]
pub struct InstallOption {
    pub source: DiscoverSource,
    pub install_command: String,
}

/// Extended discover result with multiple install options
#[derive(Debug, Clone)]
pub struct DiscoverResult {
    pub name: String,
    pub description: Option<String>,
    pub source: DiscoverSource,
    pub stars: Option<u64>,
    pub url: Option<String>,
    pub install_options: Vec<InstallOption>,
}

impl DiscoverResult {
    /// Create a new result with a single install option
    pub fn new(
        name: String,
        description: Option<String>,
        source: DiscoverSource,
        install_command: String,
    ) -> Self {
        Self {
            name,
            description,
            source: source.clone(),
            stars: None,
            url: None,
            install_options: vec![InstallOption {
                source,
                install_command,
            }],
        }
    }

    /// Add stars to the result
    pub fn with_stars(mut self, stars: u64) -> Self {
        self.stars = Some(stars);
        self
    }

    /// Add URL to the result
    pub fn with_url(mut self, url: String) -> Self {
        self.url = Some(url);
        self
    }
}

/// Trait for search sources
pub trait SearchSource: Send + Sync {
    /// Name of this search source
    fn name(&self) -> &'static str;

    /// The DiscoverSource this maps to
    fn discover_source(&self) -> DiscoverSource;

    /// Search for tools matching the query
    fn search(&self, query: &str, limit: usize) -> Result<Vec<DiscoverResult>>;
}

// ============================================================================
// crates.io Search
// ============================================================================

pub struct CratesIoSearch;

impl SearchSource for CratesIoSearch {
    fn name(&self) -> &'static str {
        "crates.io"
    }

    fn discover_source(&self) -> DiscoverSource {
        DiscoverSource::CratesIo
    }

    fn search(&self, query: &str, limit: usize) -> Result<Vec<DiscoverResult>> {
        let url = format!(
            "https://crates.io/api/v1/crates?q={}&per_page={}",
            urlencoding::encode(query),
            limit
        );

        let mut response = HTTP_AGENT
            .get(&url)
            .call()
            .context("Failed to fetch from crates.io")?;
        let response: serde_json::Value = response
            .body_mut()
            .read_json()
            .context("Failed to parse crates.io response")?;

        let empty_vec = vec![];
        let crates = response["crates"]
            .as_array()
            .unwrap_or(&empty_vec)
            .iter()
            .filter_map(|c| {
                let name = c["name"].as_str()?.to_string();
                let description = c["description"].as_str().map(String::from);
                let downloads = c["downloads"].as_u64().unwrap_or(0);

                Some(
                    DiscoverResult::new(
                        name.clone(),
                        description,
                        DiscoverSource::CratesIo,
                        format!("cargo install {}", name),
                    )
                    .with_stars(downloads / 1000) // Use downloads/1000 as pseudo-stars
                    .with_url(format!("https://crates.io/crates/{}", name)),
                )
            })
            .collect();

        Ok(crates)
    }
}

// ============================================================================
// npm Search
// ============================================================================

pub struct NpmSearch;

impl SearchSource for NpmSearch {
    fn name(&self) -> &'static str {
        "npm"
    }

    fn discover_source(&self) -> DiscoverSource {
        DiscoverSource::Npm
    }

    fn search(&self, query: &str, limit: usize) -> Result<Vec<DiscoverResult>> {
        let url = format!(
            "https://registry.npmjs.org/-/v1/search?text={}&size={}",
            urlencoding::encode(query),
            limit
        );

        let mut response = HTTP_AGENT
            .get(&url)
            .call()
            .context("Failed to fetch from npm")?;
        let response: serde_json::Value = response
            .body_mut()
            .read_json()
            .context("Failed to parse npm response")?;

        let empty_vec = vec![];
        let packages = response["objects"]
            .as_array()
            .unwrap_or(&empty_vec)
            .iter()
            .filter_map(|obj| {
                let pkg = &obj["package"];
                let name = pkg["name"].as_str()?.to_string();
                let description = pkg["description"].as_str().map(String::from);

                // Use search score as a proxy for popularity
                let score = obj["score"]["final"].as_f64().unwrap_or(0.0);
                let pseudo_stars = (score * 1000.0) as u64;

                Some(
                    DiscoverResult::new(
                        name.clone(),
                        description,
                        DiscoverSource::Npm,
                        format!("npm install -g {}", name),
                    )
                    .with_stars(pseudo_stars)
                    .with_url(format!("https://www.npmjs.com/package/{}", name)),
                )
            })
            .collect();

        Ok(packages)
    }
}

// ============================================================================
// PyPI Search
// ============================================================================

pub struct PyPISearch;

impl SearchSource for PyPISearch {
    fn name(&self) -> &'static str {
        "PyPI"
    }

    fn discover_source(&self) -> DiscoverSource {
        DiscoverSource::PyPI
    }

    fn search(&self, query: &str, limit: usize) -> Result<Vec<DiscoverResult>> {
        // PyPI doesn't have a proper search API, so we scrape the search page
        let url = format!(
            "https://pypi.org/search/?q={}&o=",
            urlencoding::encode(query)
        );

        let mut resp = HTTP_AGENT
            .get(&url)
            .call()
            .context("Failed to fetch from PyPI")?;
        let response = resp
            .body_mut()
            .read_to_string()
            .context("Failed to read PyPI response")?;

        // Parse HTML to extract package names and descriptions
        // This is a simple regex-based extraction
        let mut results = Vec::new();

        // Match package names from search results
        // Pattern: <span class="package-snippet__name">name</span>
        let name_re =
            regex::Regex::new(r#"class="package-snippet__name"[^>]*>([^<]+)</span>"#).unwrap();
        let desc_re =
            regex::Regex::new(r#"class="package-snippet__description"[^>]*>([^<]*)</p>"#).unwrap();

        let names: Vec<String> = name_re
            .captures_iter(&response)
            .map(|c| c[1].trim().to_string())
            .collect();

        let descriptions: Vec<Option<String>> = desc_re
            .captures_iter(&response)
            .map(|c| {
                let desc = c[1].trim();
                if desc.is_empty() {
                    None
                } else {
                    Some(desc.to_string())
                }
            })
            .collect();

        for (i, name) in names.into_iter().take(limit).enumerate() {
            let description = descriptions.get(i).cloned().flatten();
            results.push(
                DiscoverResult::new(
                    name.clone(),
                    description,
                    DiscoverSource::PyPI,
                    format!("pip install {}", name),
                )
                .with_url(format!("https://pypi.org/project/{}/", name)),
            );
        }

        Ok(results)
    }
}

// ============================================================================
// Homebrew Search
// ============================================================================

pub struct BrewSearch;

impl SearchSource for BrewSearch {
    fn name(&self) -> &'static str {
        "Homebrew"
    }

    fn discover_source(&self) -> DiscoverSource {
        DiscoverSource::Homebrew
    }

    fn search(&self, query: &str, limit: usize) -> Result<Vec<DiscoverResult>> {
        // Use brew search command for local search
        let output = Command::new("brew")
            .args(["search", query])
            .output()
            .context("Failed to run brew search")?;

        if !output.status.success() {
            return Ok(Vec::new());
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let results: Vec<DiscoverResult> = stdout
            .lines()
            .filter(|line| !line.is_empty() && !line.starts_with("==>"))
            .take(limit)
            .map(|name| {
                let name = name.trim().to_string();
                DiscoverResult::new(
                    name.clone(),
                    None, // Brew search doesn't return descriptions
                    DiscoverSource::Homebrew,
                    format!("brew install {}", name),
                )
                .with_url(format!("https://formulae.brew.sh/formula/{}", name))
            })
            .collect();

        Ok(results)
    }
}

// ============================================================================
// Apt Search
// ============================================================================

pub struct AptSearch;

impl SearchSource for AptSearch {
    fn name(&self) -> &'static str {
        "apt"
    }

    fn discover_source(&self) -> DiscoverSource {
        DiscoverSource::Apt
    }

    fn search(&self, query: &str, limit: usize) -> Result<Vec<DiscoverResult>> {
        let output = Command::new("apt-cache")
            .args(["search", query])
            .output()
            .context("Failed to run apt-cache search")?;

        if !output.status.success() {
            return Ok(Vec::new());
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let results: Vec<DiscoverResult> = stdout
            .lines()
            .filter(|line| !line.is_empty())
            .take(limit)
            .filter_map(|line| {
                // apt-cache search format: "package - description"
                let parts: Vec<&str> = line.splitn(2, " - ").collect();
                let name = parts.first()?.trim().to_string();
                let description = parts.get(1).map(|d| d.trim().to_string());

                Some(DiscoverResult::new(
                    name.clone(),
                    description,
                    DiscoverSource::Apt,
                    format!("sudo apt install {}", name),
                ))
            })
            .collect();

        Ok(results)
    }
}

// ============================================================================
// GitHub Search
// ============================================================================

pub struct GitHubSearch;

impl GitHubSearch {
    /// Map GitHub language to DiscoverSource
    fn language_to_source(language: &str) -> Option<DiscoverSource> {
        match language.to_lowercase().as_str() {
            "rust" => Some(DiscoverSource::CratesIo),
            "python" => Some(DiscoverSource::PyPI),
            "javascript" | "typescript" => Some(DiscoverSource::Npm),
            _ => None,
        }
    }

    /// Generate install command based on language
    fn install_command(name: &str, language: &str) -> Option<String> {
        match language.to_lowercase().as_str() {
            "rust" => Some(format!("cargo install {}", name)),
            "python" => Some(format!("pip install {}", name)),
            "javascript" | "typescript" => Some(format!("npm install -g {}", name)),
            _ => None,
        }
    }
}

impl SearchSource for GitHubSearch {
    fn name(&self) -> &'static str {
        "GitHub"
    }

    fn discover_source(&self) -> DiscoverSource {
        DiscoverSource::GitHub
    }

    fn search(&self, query: &str, limit: usize) -> Result<Vec<DiscoverResult>> {
        // Use gh CLI for searching
        let output = Command::new("gh")
            .args([
                "search",
                "repos",
                query,
                "--limit",
                &limit.to_string(),
                "--json",
                "name,description,stargazersCount,language,url",
            ])
            .output()
            .context("Failed to run gh search")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            if stderr.contains("rate limit") {
                anyhow::bail!("GitHub API rate limit exceeded");
            }
            return Ok(Vec::new());
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let repos: Vec<serde_json::Value> =
            serde_json::from_str(&stdout).context("Failed to parse gh output")?;

        let results: Vec<DiscoverResult> = repos
            .into_iter()
            .filter_map(|repo| {
                let name = repo["name"].as_str()?.to_string();
                let description = repo["description"].as_str().map(String::from);
                let stars = repo["stargazersCount"].as_u64().unwrap_or(0);
                let language = repo["language"].as_str().unwrap_or("");
                let url = repo["url"].as_str().map(String::from);

                // Only include if language maps to an install source
                let source = Self::language_to_source(language)?;
                let install_cmd = Self::install_command(&name, language)?;

                let mut result = DiscoverResult::new(name, description, source, install_cmd);
                result.stars = Some(stars);
                if let Some(u) = url {
                    result.url = Some(u);
                }

                Some(result)
            })
            .collect();

        Ok(results)
    }
}

// ============================================================================
// AI Search
// ============================================================================

pub struct AiSearch {
    installed_tools: Vec<String>,
    enabled_sources: Vec<String>,
}

impl AiSearch {
    pub fn new(installed_tools: Vec<String>, enabled_sources: Vec<String>) -> Self {
        Self {
            installed_tools,
            enabled_sources,
        }
    }
}

impl SearchSource for AiSearch {
    fn name(&self) -> &'static str {
        "AI"
    }

    fn discover_source(&self) -> DiscoverSource {
        DiscoverSource::AI
    }

    fn search(&self, query: &str, _limit: usize) -> Result<Vec<DiscoverResult>> {
        use crate::ai::{discovery_prompt, invoke_ai, parse_discovery_response};

        let sources_refs: Vec<&str> = self.enabled_sources.iter().map(|s| s.as_str()).collect();
        let prompt = discovery_prompt(query, &self.installed_tools, &sources_refs);
        let response = invoke_ai(&prompt)?;
        let discovery = parse_discovery_response(&response)?;

        let results: Vec<DiscoverResult> = discovery
            .tools
            .into_iter()
            .map(|tool| {
                let source = match tool.source.to_lowercase().as_str() {
                    "cargo" | "crates.io" => DiscoverSource::CratesIo,
                    "pip" | "pypi" => DiscoverSource::PyPI,
                    "npm" => DiscoverSource::Npm,
                    "apt" => DiscoverSource::Apt,
                    "brew" | "homebrew" => DiscoverSource::Homebrew,
                    _ => DiscoverSource::AI,
                };

                let mut result = DiscoverResult::new(
                    tool.name,
                    Some(tool.description),
                    source,
                    tool.install_cmd,
                );
                if let Some(stars) = tool.stars {
                    result.stars = Some(stars);
                }
                if let Some(github) = tool.github {
                    result.url = Some(github);
                }

                result
            })
            .collect();

        Ok(results)
    }
}

// ============================================================================
// Multi-source Search
// ============================================================================

/// Get all available search sources based on config
pub fn get_enabled_sources(
    config: &HoardConfig,
    installed_tools: Vec<String>,
) -> Vec<Box<dyn SearchSource>> {
    let enabled = config.sources.enabled_sources();
    let mut sources: Vec<Box<dyn SearchSource>> = Vec::new();

    // Store enabled sources for AI before the loop consumes them
    let ai_sources: Vec<String> = enabled.iter().map(|s| s.to_string()).collect();

    // Map enabled source names to search implementations
    for source_name in enabled {
        match source_name {
            "cargo" => sources.push(Box::new(CratesIoSearch)),
            "npm" => sources.push(Box::new(NpmSearch)),
            "pip" => sources.push(Box::new(PyPISearch)),
            "brew" => sources.push(Box::new(BrewSearch)),
            "apt" => sources.push(Box::new(AptSearch)),
            _ => {} // Skip sources without search implementations (flatpak, manual)
        }
    }

    // Always add GitHub search (filtered by enabled sources)
    sources.push(Box::new(GitHubSearch));

    // Add AI search if AI provider is configured
    if config.ai.provider != crate::config::AiProvider::None {
        sources.push(Box::new(AiSearch::new(installed_tools, ai_sources)));
    }

    sources
}

/// Normalize a tool name for deduplication
fn normalize_name(name: &str) -> String {
    name.to_lowercase().replace(['-', '_'], "")
}

/// Deduplicate results from multiple sources, merging install options
pub fn deduplicate_results(mut results: Vec<DiscoverResult>) -> Vec<DiscoverResult> {
    use std::collections::HashMap;

    // Group by normalized name
    let mut groups: HashMap<String, Vec<DiscoverResult>> = HashMap::new();

    for result in results.drain(..) {
        let key = normalize_name(&result.name);
        groups.entry(key).or_default().push(result);
    }

    // Merge each group
    let mut merged: Vec<DiscoverResult> = groups
        .into_values()
        .map(|group| {
            // Sort by stars (highest first), then pick primary
            let mut sorted: Vec<_> = group.into_iter().collect();
            sorted.sort_by(|a, b| b.stars.cmp(&a.stars));

            let mut primary = sorted.remove(0);

            // Merge install options from other sources
            for other in sorted {
                for opt in other.install_options {
                    // Avoid duplicate install options
                    let already_has = primary
                        .install_options
                        .iter()
                        .any(|o| o.source == opt.source);
                    if !already_has {
                        primary.install_options.push(opt);
                    }
                }
                // Prefer GitHub description if available
                if other.source == DiscoverSource::GitHub && other.description.is_some() {
                    primary.description = other.description;
                }
                // Prefer GitHub URL
                if other.source == DiscoverSource::GitHub && other.url.is_some() {
                    primary.url = other.url;
                }
            }

            primary
        })
        .collect();

    // Sort by stars desc, then alphabetically
    merged.sort_by(|a, b| match (b.stars, a.stars) {
        (Some(bs), Some(as_)) => bs.cmp(&as_),
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (None, None) => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
    });

    merged
}

/// Filter GitHub results to only include results for enabled sources
pub fn filter_github_results(
    results: Vec<DiscoverResult>,
    enabled_sources: &HashSet<&str>,
) -> Vec<DiscoverResult> {
    results
        .into_iter()
        .filter(|r| {
            // Always keep non-GitHub results
            if r.source != DiscoverSource::GitHub {
                return true;
            }

            // For GitHub results, check if the mapped source is enabled
            // The source is already mapped from language, so check if it's enabled
            match r.source {
                DiscoverSource::CratesIo => enabled_sources.contains("cargo"),
                DiscoverSource::PyPI => enabled_sources.contains("pip"),
                DiscoverSource::Npm => enabled_sources.contains("npm"),
                _ => true,
            }
        })
        .collect()
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_name() {
        assert_eq!(normalize_name("ripgrep"), "ripgrep");
        assert_eq!(normalize_name("rip-grep"), "ripgrep");
        assert_eq!(normalize_name("rip_grep"), "ripgrep");
        assert_eq!(normalize_name("Rip-Grep"), "ripgrep");
    }

    #[test]
    fn test_deduplicate_results() {
        let results = vec![
            DiscoverResult::new(
                "ripgrep".to_string(),
                Some("Fast grep".to_string()),
                DiscoverSource::CratesIo,
                "cargo install ripgrep".to_string(),
            )
            .with_stars(100),
            DiscoverResult::new(
                "rip-grep".to_string(),
                Some("Line-oriented search tool".to_string()),
                DiscoverSource::GitHub,
                "cargo install ripgrep".to_string(),
            )
            .with_stars(50000),
        ];

        let merged = deduplicate_results(results);
        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].name, "rip-grep"); // GitHub one has more stars
        assert_eq!(merged[0].install_options.len(), 2);
    }

    #[test]
    fn test_github_language_mapping() {
        assert_eq!(
            GitHubSearch::language_to_source("Rust"),
            Some(DiscoverSource::CratesIo)
        );
        assert_eq!(
            GitHubSearch::language_to_source("Python"),
            Some(DiscoverSource::PyPI)
        );
        assert_eq!(
            GitHubSearch::language_to_source("JavaScript"),
            Some(DiscoverSource::Npm)
        );
        assert_eq!(
            GitHubSearch::language_to_source("TypeScript"),
            Some(DiscoverSource::Npm)
        );
        assert_eq!(GitHubSearch::language_to_source("Go"), None);
    }

    #[test]
    fn test_discover_result_builder() {
        let result = DiscoverResult::new(
            "test".to_string(),
            Some("desc".to_string()),
            DiscoverSource::CratesIo,
            "cargo install test".to_string(),
        )
        .with_stars(100)
        .with_url("https://example.com".to_string());

        assert_eq!(result.stars, Some(100));
        assert_eq!(result.url, Some("https://example.com".to_string()));
    }
}
