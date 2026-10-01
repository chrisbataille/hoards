//! External search functionality for the Discover tab
//!
//! This module provides trait-based search sources for discovering tools
//! from various package registries, GitHub, and AI recommendations.

use std::cmp::Reverse;
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
    pub language: Option<String>,
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
            language: None,
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

    /// Add language to the result
    pub fn with_language(mut self, language: String) -> Self {
        self.language = Some(language);
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
// GitHub URL Normalization Helpers
// ============================================================================

/// Extract the "owner/repo" part from various GitHub URL formats.
/// Handles formats like:
/// - https://github.com/owner/repo
/// - git+https://github.com/owner/repo.git
/// - git://github.com/owner/repo.git
/// - github:owner/repo
/// - ssh://git@github.com/owner/repo.git
/// - git@github.com:owner/repo.git
fn extract_github_owner_repo(url: &str) -> Option<String> {
    let mut s = url.trim();

    // Handle github:owner/repo shorthand
    if let Some(rest) = s.strip_prefix("github:") {
        let cleaned = rest.trim_end_matches(".git");
        if cleaned.contains('/') {
            return Some(cleaned.to_lowercase());
        }
    }

    // Strip common prefixes one by one
    if let Some(rest) = s.strip_prefix("git+") {
        s = rest;
    }
    if let Some(rest) = s.strip_prefix("ssh://") {
        s = rest;
    }
    if let Some(rest) = s.strip_prefix("git://") {
        s = rest;
    }
    if let Some(rest) = s.strip_prefix("https://") {
        s = rest;
    }
    if let Some(rest) = s.strip_prefix("http://") {
        s = rest;
    }
    if let Some(rest) = s.strip_prefix("git@") {
        s = rest;
    }

    // Now should be: github.com/owner/repo or github.com:owner/repo
    let path = s
        .strip_prefix("github.com/")
        .or_else(|| s.strip_prefix("github.com:"))?;

    // Remove .git suffix and any trailing slashes
    let cleaned = path.trim_end_matches(".git").trim_end_matches('/');

    // Extract owner/repo (first two path segments)
    let parts: Vec<&str> = cleaned.split('/').take(2).collect();
    if parts.len() == 2 && !parts[0].is_empty() && !parts[1].is_empty() {
        Some(format!("{}/{}", parts[0], parts[1]).to_lowercase())
    } else {
        None
    }
}

/// Check if two URLs point to the same GitHub repository
fn github_urls_match(url1: &str, url2: &str) -> bool {
    match (
        extract_github_owner_repo(url1),
        extract_github_owner_repo(url2),
    ) {
        (Some(a), Some(b)) => a == b,
        _ => false,
    }
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
        // Fetch more results than needed to filter out library-only crates
        let fetch_limit = limit * 3;
        let url = format!(
            "https://crates.io/api/v1/crates?q={}&per_page={}",
            urlencoding::encode(query),
            fetch_limit
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
        let candidates: Vec<_> = response["crates"]
            .as_array()
            .unwrap_or(&empty_vec)
            .iter()
            .filter_map(|c| {
                let name = c["name"].as_str()?.to_string();
                let description = c["description"].as_str().map(String::from);
                let downloads = c["downloads"].as_u64().unwrap_or(0);
                // Get repository URL (usually GitHub)
                let repository = c["repository"].as_str().map(String::from);
                Some((name, description, downloads, repository))
            })
            .collect();

        // Check each crate for binaries (in parallel for speed)
        let crates: Vec<_> = std::thread::scope(|s| {
            let handles: Vec<_> = candidates
                .iter()
                .map(|(name, description, downloads, repository)| {
                    let name = name.clone();
                    let description = description.clone();
                    let downloads = *downloads;
                    let repository = repository.clone();
                    s.spawn(move || {
                        if crate_has_binaries(&name) == Some(true) {
                            let mut result = DiscoverResult::new(
                                name.clone(),
                                description,
                                DiscoverSource::CratesIo,
                                format!("cargo install {}", name),
                            )
                            .with_stars(downloads / 1000);
                            // Use repository URL if available (usually GitHub), otherwise crates.io
                            if let Some(repo) = repository {
                                result = result.with_url(repo);
                            } else {
                                result =
                                    result.with_url(format!("https://crates.io/crates/{}", name));
                            }
                            Some(result)
                        } else {
                            None
                        }
                    })
                })
                .collect();

            handles
                .into_iter()
                .filter_map(|h| h.join().ok().flatten())
                .take(limit)
                .collect()
        });

        Ok(crates)
    }
}

/// Check if a crate has binaries by fetching its details from crates.io
/// Returns Some(true) if crate exists and has binaries
/// Returns Some(false) if crate exists but has no binaries
/// Returns None if crate doesn't exist (404)
fn crate_has_binaries(name: &str) -> Option<bool> {
    let url = format!("https://crates.io/api/v1/crates/{}", name);

    let response = match HTTP_AGENT.get(&url).call() {
        Ok(resp) => resp,
        Err(ureq::Error::StatusCode(404)) => {
            // Package doesn't exist on crates.io
            return None;
        }
        Err(_) => {
            // Network error - can't determine, assume it might have binaries
            return Some(true);
        }
    };

    let Ok(data): Result<serde_json::Value, _> = response.into_body().read_json() else {
        return Some(true);
    };

    // Check if newest version has binaries
    // The API returns versions array, check the first (newest) one
    let empty_vec = vec![];
    let versions = data["versions"].as_array().unwrap_or(&empty_vec);

    if let Some(latest) = versions.first() {
        let bin_names = latest["bin_names"].as_array().unwrap_or(&empty_vec);
        Some(!bin_names.is_empty())
    } else {
        // No versions found, assume it might have binaries
        Some(true)
    }
}

/// Check if a crate exists, has binaries, AND its repository matches the given GitHub URL.
/// Used by GitHubSearch to validate that a crate with the same name as a repo
/// is actually the same project.
/// Returns Some(true) if crate exists, has binaries, and repo matches
/// Returns Some(false) if crate exists but has no binaries
/// Returns None if crate doesn't exist or repo doesn't match
fn crate_matches_github_repo(name: &str, github_url: &str) -> Option<bool> {
    let url = format!("https://crates.io/api/v1/crates/{}", name);

    let response = match HTTP_AGENT.get(&url).call() {
        Ok(resp) => resp,
        Err(ureq::Error::StatusCode(404)) => {
            return None;
        }
        Err(_) => {
            // Network error - be conservative, don't assume match
            return None;
        }
    };

    let Ok(data): Result<serde_json::Value, _> = response.into_body().read_json() else {
        return None;
    };

    // Check if the crate's repository matches the GitHub URL. No repository
    // URL in the crate metadata means it can't be verified: skip it.
    let repo_url = data["crate"]["repository"].as_str()?;
    if !github_urls_match(repo_url, github_url) {
        // Repository doesn't match - this is a different project
        return None;
    }

    // Repository matches, now check for binaries
    let empty_vec = vec![];
    let versions = data["versions"].as_array().unwrap_or(&empty_vec);

    if let Some(latest) = versions.first() {
        let bin_names = latest["bin_names"].as_array().unwrap_or(&empty_vec);
        Some(!bin_names.is_empty())
    } else {
        Some(true)
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
        // Fetch more results to filter out library-only packages
        let fetch_limit = limit * 3;
        let url = format!(
            "https://registry.npmjs.org/-/v1/search?text={}&size={}",
            urlencoding::encode(query),
            fetch_limit
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
        let candidates: Vec<_> = response["objects"]
            .as_array()
            .unwrap_or(&empty_vec)
            .iter()
            .filter_map(|obj| {
                let pkg = &obj["package"];
                let name = pkg["name"].as_str()?.to_string();
                let description = pkg["description"].as_str().map(String::from);
                let score = obj["score"]["final"].as_f64().unwrap_or(0.0);
                Some((name, description, score))
            })
            .collect();

        // Check each package for CLI binaries (in parallel)
        let packages: Vec<_> = std::thread::scope(|s| {
            let handles: Vec<_> = candidates
                .iter()
                .map(|(name, description, score)| {
                    let name = name.clone();
                    let description = description.clone();
                    let score = *score;
                    s.spawn(move || {
                        if npm_package_has_bin(&name) {
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
                        } else {
                            None
                        }
                    })
                })
                .collect();

            handles
                .into_iter()
                .filter_map(|h| h.join().ok().flatten())
                .take(limit)
                .collect()
        });

        Ok(packages)
    }
}

/// Check if an npm package has CLI binaries
fn npm_package_has_bin(name: &str) -> bool {
    let url = format!("https://registry.npmjs.org/{}", urlencoding::encode(name));

    let response = match HTTP_AGENT.get(&url).call() {
        Ok(resp) => resp,
        Err(ureq::Error::StatusCode(404)) => {
            // Package doesn't exist on npm
            return false;
        }
        Err(_) => {
            // Network error - can't determine, assume it might have bin
            return true;
        }
    };

    let Ok(data): Result<serde_json::Value, _> = response.into_body().read_json() else {
        return true;
    };

    // Check if package has bin field (can be string or object)
    let latest_version = data["dist-tags"]["latest"].as_str().unwrap_or("");
    if let Some(version_data) = data["versions"].get(latest_version) {
        // bin can be a string (single binary) or object (multiple binaries)
        version_data.get("bin").is_some()
    } else {
        true // Can't determine, include it
    }
}

/// Check if an npm package exists, has binaries, AND its repository matches the given GitHub URL.
/// Used by GitHubSearch to validate that an npm package with the same name as a repo
/// is actually the same project.
fn npm_package_matches_github_repo(name: &str, github_url: &str) -> bool {
    let url = format!("https://registry.npmjs.org/{}", urlencoding::encode(name));

    let response = match HTTP_AGENT.get(&url).call() {
        Ok(resp) => resp,
        Err(_) => {
            return false;
        }
    };

    let Ok(data): Result<serde_json::Value, _> = response.into_body().read_json() else {
        return false;
    };

    // Check if the package's repository matches the GitHub URL
    // npm repository can be an object with url field or a string
    let repo_url = if let Some(repo_obj) = data["repository"].as_object() {
        repo_obj.get("url").and_then(|u| u.as_str())
    } else {
        data["repository"].as_str()
    };

    let Some(repo_url) = repo_url else {
        // No repository URL - can't verify, skip
        return false;
    };

    if !github_urls_match(repo_url, github_url) {
        // Repository doesn't match - this is a different project
        return false;
    }

    // Repository matches, now check for binaries
    let latest_version = data["dist-tags"]["latest"].as_str().unwrap_or("");
    if let Some(version_data) = data["versions"].get(latest_version) {
        version_data.get("bin").is_some()
    } else {
        false
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
        // Fetch more results to filter out library-only packages
        let fetch_limit = limit * 3;
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
        let name_re =
            regex::Regex::new(r#"class="package-snippet__name"[^>]*>([^<]+)</span>"#).unwrap();
        let desc_re =
            regex::Regex::new(r#"class="package-snippet__description"[^>]*>([^<]*)</p>"#).unwrap();

        let names: Vec<String> = name_re
            .captures_iter(&response)
            .take(fetch_limit)
            .map(|c| c[1].trim().to_string())
            .collect();

        let descriptions: Vec<Option<String>> = desc_re
            .captures_iter(&response)
            .take(fetch_limit)
            .map(|c| {
                let desc = c[1].trim();
                if desc.is_empty() {
                    None
                } else {
                    Some(desc.to_string())
                }
            })
            .collect();

        // Pair names with descriptions
        let candidates: Vec<_> = names
            .into_iter()
            .enumerate()
            .map(|(i, name)| {
                let description = descriptions.get(i).cloned().flatten();
                (name, description)
            })
            .collect();

        // Check each package for CLI entry points (in parallel)
        let results: Vec<_> = std::thread::scope(|s| {
            let handles: Vec<_> = candidates
                .iter()
                .map(|(name, description)| {
                    let name = name.clone();
                    let description = description.clone();
                    s.spawn(move || {
                        if pypi_package_has_cli(&name) {
                            Some(
                                DiscoverResult::new(
                                    name.clone(),
                                    description,
                                    DiscoverSource::PyPI,
                                    format!("pip install {}", name),
                                )
                                .with_url(format!("https://pypi.org/project/{}/", name)),
                            )
                        } else {
                            None
                        }
                    })
                })
                .collect();

            handles
                .into_iter()
                .filter_map(|h| h.join().ok().flatten())
                .take(limit)
                .collect()
        });

        Ok(results)
    }
}

/// Check if a PyPI package has CLI entry points (console_scripts)
fn pypi_package_has_cli(name: &str) -> bool {
    let url = format!("https://pypi.org/pypi/{}/json", urlencoding::encode(name));

    let response = match HTTP_AGENT.get(&url).call() {
        Ok(resp) => resp,
        Err(ureq::Error::StatusCode(404)) => {
            // Package doesn't exist on PyPI
            return false;
        }
        Err(_) => {
            // Network error - can't determine, assume it might have CLI
            return true;
        }
    };

    let Ok(data): Result<serde_json::Value, _> = response.into_body().read_json() else {
        return true;
    };

    // Check for console_scripts in info.project_urls or classifiers
    // Also check if description mentions "CLI", "command-line", etc.
    let info = &data["info"];

    // Check classifiers for "Environment :: Console"
    if let Some(classifiers) = info["classifiers"].as_array() {
        for classifier in classifiers {
            if let Some(c) = classifier.as_str()
                && (c.contains("Environment :: Console") || c.contains("Command-line"))
            {
                return true;
            }
        }
    }

    // Check if summary/description mentions CLI
    let summary = info["summary"].as_str().unwrap_or("");
    let description = info["description"].as_str().unwrap_or("");
    let combined = format!("{} {}", summary, description).to_lowercase();

    if combined.contains("command-line")
        || combined.contains("command line")
        || combined.contains(" cli ")
        || combined.contains("cli tool")
        || combined.contains("cli for")
    {
        return true;
    }

    // Check project URLs for potential CLI indicators
    if let Some(urls) = info["project_urls"].as_object() {
        for key in urls.keys() {
            if key.to_lowercase().contains("cli") {
                return true;
            }
        }
    }

    // Check requires_dist for typical CLI dependencies like click, argparse, typer
    if let Some(requires) = info["requires_dist"].as_array() {
        for req in requires {
            if let Some(r) = req.as_str() {
                let r_lower = r.to_lowercase();
                if r_lower.starts_with("click")
                    || r_lower.starts_with("typer")
                    || r_lower.starts_with("fire")
                    || r_lower.starts_with("argcomplete")
                {
                    return true;
                }
            }
        }
    }

    false
}

/// Check if a PyPI package exists, has CLI indicators, AND its repository matches the given GitHub URL.
/// Used by GitHubSearch to validate that a PyPI package with the same name as a repo
/// is actually the same project.
fn pypi_package_matches_github_repo(name: &str, github_url: &str) -> bool {
    let url = format!("https://pypi.org/pypi/{}/json", urlencoding::encode(name));

    let response = match HTTP_AGENT.get(&url).call() {
        Ok(resp) => resp,
        Err(_) => {
            return false;
        }
    };

    let Ok(data): Result<serde_json::Value, _> = response.into_body().read_json() else {
        return false;
    };

    let info = &data["info"];

    // Check if any project URL matches the GitHub URL
    // PyPI stores URLs in project_urls with various keys like "Repository", "Source", "GitHub", etc.
    let mut repo_matches = false;

    if let Some(urls) = info["project_urls"].as_object() {
        for (key, value) in urls {
            let key_lower = key.to_lowercase();
            // Check keys that typically contain repository URLs
            let is_repo_key = key_lower.contains("repository")
                || key_lower.contains("source")
                || key_lower.contains("github")
                || key_lower.contains("code")
                || key_lower.contains("homepage");

            if is_repo_key
                && let Some(url_str) = value.as_str()
                && github_urls_match(url_str, github_url)
            {
                repo_matches = true;
                break;
            }
        }
    }

    // Also check the homepage field
    if !repo_matches
        && let Some(homepage) = info["home_page"].as_str()
        && github_urls_match(homepage, github_url)
    {
        repo_matches = true;
    }

    if !repo_matches {
        // Repository doesn't match - this is a different project
        return false;
    }

    // Repository matches, now check for CLI indicators
    // Check classifiers for "Environment :: Console"
    if let Some(classifiers) = info["classifiers"].as_array() {
        for classifier in classifiers {
            if let Some(c) = classifier.as_str()
                && (c.contains("Environment :: Console") || c.contains("Command-line"))
            {
                return true;
            }
        }
    }

    // Check if summary/description mentions CLI
    let summary = info["summary"].as_str().unwrap_or("");
    let description = info["description"].as_str().unwrap_or("");
    let combined = format!("{} {}", summary, description).to_lowercase();

    if combined.contains("command-line")
        || combined.contains("command line")
        || combined.contains(" cli ")
        || combined.contains("cli tool")
        || combined.contains("cli for")
    {
        return true;
    }

    // Check requires_dist for typical CLI dependencies
    if let Some(requires) = info["requires_dist"].as_array() {
        for req in requires {
            if let Some(r) = req.as_str() {
                let r_lower = r.to_lowercase();
                if r_lower.starts_with("click")
                    || r_lower.starts_with("typer")
                    || r_lower.starts_with("fire")
                    || r_lower.starts_with("argcomplete")
                {
                    return true;
                }
            }
        }
    }

    false
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

impl SearchSource for GitHubSearch {
    fn name(&self) -> &'static str {
        "GitHub"
    }

    fn discover_source(&self) -> DiscoverSource {
        DiscoverSource::GitHub
    }

    fn search(&self, query: &str, limit: usize) -> Result<Vec<DiscoverResult>> {
        // Fetch more results to filter out repos without installable packages
        let fetch_limit = limit * 3;

        // Use gh CLI for searching
        let output = Command::new("gh")
            .args([
                "search",
                "repos",
                query,
                "--limit",
                &fetch_limit.to_string(),
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

        // Extract candidates with language info
        let candidates: Vec<_> = repos
            .into_iter()
            .filter_map(|repo| {
                let name = repo["name"].as_str()?.to_string();
                let description = repo["description"].as_str().map(String::from);
                let stars = repo["stargazersCount"].as_u64().unwrap_or(0);
                let language = repo["language"].as_str().unwrap_or("").to_string();
                let url = repo["url"].as_str().map(String::from);
                Some((name, description, stars, language, url))
            })
            .collect();

        // Cross-check against package registries (in parallel)
        let results: Vec<_> = std::thread::scope(|s| {
            let handles: Vec<_> = candidates
                .iter()
                .map(|(name, description, stars, language, url)| {
                    let name = name.clone();
                    let description = description.clone();
                    let stars = *stars;
                    let language = language.clone();
                    let url = url.clone();
                    s.spawn(move || {
                        // Determine source and install command based on language
                        let (source, install_cmd) = match language.to_lowercase().as_str() {
                            "rust" => {
                                let github_url = url.as_ref()?;
                                match crate_matches_github_repo(&name, github_url) {
                                    Some(true) => {
                                        // Exists on crates.io with binaries and repo matches
                                        (
                                            DiscoverSource::CratesIo,
                                            format!("cargo install {}", name),
                                        )
                                    }
                                    Some(false) => {
                                        // Exists on crates.io but no binaries - skip
                                        return None;
                                    }
                                    None => {
                                        // Not on crates.io or repo doesn't match - offer git install
                                        (
                                            DiscoverSource::GitHub,
                                            format!("cargo install --git {}", github_url),
                                        )
                                    }
                                }
                            }
                            "python" => {
                                let github_url = url.as_ref()?;
                                if pypi_package_matches_github_repo(&name, github_url) {
                                    (DiscoverSource::PyPI, format!("pip install {}", name))
                                } else {
                                    return None;
                                }
                            }
                            "javascript" | "typescript" => {
                                let github_url = url.as_ref()?;
                                if npm_package_matches_github_repo(&name, github_url) {
                                    (DiscoverSource::Npm, format!("npm install -g {}", name))
                                } else {
                                    return None;
                                }
                            }
                            "go" => {
                                // Go installs directly from GitHub - no registry validation needed
                                // Strip https:// prefix - go install needs github.com/user/repo format
                                // Most Go CLI tools use the /cmd/name pattern for main package
                                let git_url = url.as_ref()?;
                                let go_path = git_url.strip_prefix("https://").unwrap_or(git_url);
                                // Use /cmd/{name} pattern - standard for Go CLI tools
                                let install_path = format!("{}/cmd/{}", go_path, name);
                                (
                                    DiscoverSource::Go,
                                    format!("go install {}@latest", install_path),
                                )
                            }
                            _ => return None,
                        };

                        let mut result =
                            DiscoverResult::new(name, description, source, install_cmd);
                        result.stars = Some(stars);
                        result.url = url;
                        result.language = Some(language);
                        Some(result)
                    })
                })
                .collect();

            handles
                .into_iter()
                .filter_map(|h| h.join().ok().flatten())
                .take(limit)
                .collect()
        });

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

        // Convert AI recommendations to candidates for validation
        let candidates: Vec<_> = discovery
            .tools
            .into_iter()
            .map(|tool| {
                let source_str = tool.source.to_lowercase();
                let source = match source_str.as_str() {
                    "cargo" | "crates.io" => DiscoverSource::CratesIo,
                    "pip" | "pypi" => DiscoverSource::PyPI,
                    "npm" => DiscoverSource::Npm,
                    "apt" => DiscoverSource::Apt,
                    "brew" | "homebrew" => DiscoverSource::Homebrew,
                    _ => DiscoverSource::AI,
                };
                (
                    tool.name,
                    tool.description,
                    source,
                    tool.install_cmd,
                    tool.stars,
                    tool.github,
                )
            })
            .collect();

        // Validate AI recommendations against package registries (in parallel)
        let results: Vec<_> = std::thread::scope(|s| {
            let handles: Vec<_> = candidates
                .iter()
                .map(|(name, description, source, install_cmd, stars, github)| {
                    let name = name.clone();
                    let description = description.clone();
                    let source = source.clone();
                    let install_cmd = install_cmd.clone();
                    let stars = *stars;
                    let github = github.clone();
                    s.spawn(move || {
                        // Validate the package exists and is installable
                        let is_valid = match source {
                            DiscoverSource::CratesIo => {
                                // For AI-recommended crates.io packages, require they exist with binaries
                                crate_has_binaries(&name) == Some(true)
                            }
                            DiscoverSource::PyPI => pypi_package_has_cli(&name),
                            DiscoverSource::Npm => npm_package_has_bin(&name),
                            // apt/brew recommendations are assumed valid
                            DiscoverSource::Apt | DiscoverSource::Homebrew => true,
                            // Unknown source, can't validate
                            _ => true,
                        };

                        if is_valid {
                            let mut result =
                                DiscoverResult::new(name, Some(description), source, install_cmd);
                            if let Some(s) = stars {
                                result.stars = Some(s);
                            }
                            if let Some(g) = github {
                                result.url = Some(g);
                            }
                            Some(result)
                        } else {
                            None
                        }
                    })
                })
                .collect();

            handles
                .into_iter()
                .filter_map(|h| h.join().ok().flatten())
                .collect()
        });

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
            sorted.sort_by_key(|a| Reverse(a.stars));

            let mut primary = sorted.remove(0);

            // Helper to check if URL is a GitHub URL
            let is_github_url = |url: &Option<String>| -> bool {
                url.as_ref()
                    .is_some_and(|u| u.contains("github.com") || u.contains("github.io"))
            };

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
                // Prefer GitHub URL from any source over non-GitHub URLs
                if is_github_url(&other.url) && !is_github_url(&primary.url) {
                    primary.url = other.url.clone();
                    // Also take description from the source with GitHub URL
                    if other.description.is_some() {
                        primary.description = other.description;
                    }
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

    #[test]
    fn test_extract_github_owner_repo() {
        // Standard HTTPS URL
        assert_eq!(
            extract_github_owner_repo("https://github.com/owner/repo"),
            Some("owner/repo".to_string())
        );

        // With .git suffix
        assert_eq!(
            extract_github_owner_repo("https://github.com/owner/repo.git"),
            Some("owner/repo".to_string())
        );

        // git+ prefix (common in npm)
        assert_eq!(
            extract_github_owner_repo("git+https://github.com/owner/repo.git"),
            Some("owner/repo".to_string())
        );

        // SSH URL
        assert_eq!(
            extract_github_owner_repo("git@github.com:owner/repo.git"),
            Some("owner/repo".to_string())
        );

        // github: shorthand
        assert_eq!(
            extract_github_owner_repo("github:owner/repo"),
            Some("owner/repo".to_string())
        );

        // Case insensitive
        assert_eq!(
            extract_github_owner_repo("https://github.com/Owner/Repo"),
            Some("owner/repo".to_string())
        );

        // With trailing slash
        assert_eq!(
            extract_github_owner_repo("https://github.com/owner/repo/"),
            Some("owner/repo".to_string())
        );

        // Invalid URLs
        assert_eq!(extract_github_owner_repo("not-a-url"), None);
        assert_eq!(
            extract_github_owner_repo("https://gitlab.com/owner/repo"),
            None
        );
    }

    #[test]
    fn test_github_urls_match() {
        // Same repo, different formats
        assert!(github_urls_match(
            "https://github.com/owner/repo",
            "git+https://github.com/owner/repo.git"
        ));

        // Case insensitive
        assert!(github_urls_match(
            "https://github.com/Owner/Repo",
            "https://github.com/owner/repo"
        ));

        // Different repos
        assert!(!github_urls_match(
            "https://github.com/owner1/repo",
            "https://github.com/owner2/repo"
        ));

        // npm happy vs happy-coder case (the bug we're fixing)
        assert!(!github_urls_match(
            "git+https://github.com/franciscop/happy.git",
            "https://github.com/slopus/happy"
        ));
    }
}
