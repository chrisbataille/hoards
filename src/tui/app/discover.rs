//! Discover tab operations for the TUI
//!
//! This module contains all methods related to the Discover tab functionality,
//! including search, navigation, source filtering, and history management.

use crate::config::{AiProvider, HoardConfig};
use crate::db::Database;

use super::types::{
    BackgroundOp, DiscoverResult, DiscoverSortBy, DiscoverSource, InstallOption, LoadingProgress,
};
use super::{AiOperation, App};

impl App {
    // ========================================================================
    // Discover Tab Navigation
    // ========================================================================

    /// Move discover selection down
    pub fn select_next_discover(&mut self) {
        if !self.discover_results.is_empty() {
            self.discover_selected =
                (self.discover_selected + 1).min(self.discover_results.len() - 1);
        }
    }

    /// Move discover selection up
    pub fn select_prev_discover(&mut self) {
        if self.discover_selected > 0 {
            self.discover_selected -= 1;
        }
    }

    /// Move discover selection to top
    pub fn select_first_discover(&mut self) {
        self.discover_selected = 0;
    }

    /// Move discover selection to bottom
    pub fn select_last_discover(&mut self) {
        if !self.discover_results.is_empty() {
            self.discover_selected = self.discover_results.len() - 1;
        }
    }

    /// Get the currently selected discover result
    pub fn selected_discover(&self) -> Option<&DiscoverResult> {
        self.discover_results.get(self.discover_selected)
    }

    // ========================================================================
    // Discover Sorting
    // ========================================================================

    /// Cycle discover sort option
    pub fn cycle_discover_sort(&mut self) {
        self.discover_sort_by = self.discover_sort_by.next();
        self.sort_discover_results();
    }

    /// Sort discover results based on current sort option
    pub fn sort_discover_results(&mut self) {
        match self.discover_sort_by {
            DiscoverSortBy::Stars => {
                self.discover_results
                    .sort_by(|a, b| match (b.stars, a.stars) {
                        (Some(bs), Some(as_)) => bs.cmp(&as_),
                        (Some(_), None) => std::cmp::Ordering::Less,
                        (None, Some(_)) => std::cmp::Ordering::Greater,
                        (None, None) => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
                    });
            }
            DiscoverSortBy::Name => {
                self.discover_results.sort_by_key(|a| a.name.to_lowercase());
            }
            DiscoverSortBy::Source => {
                self.discover_results.sort_by(|a, b| {
                    let source_order = |s: &DiscoverSource| match s {
                        DiscoverSource::CratesIo => 0,
                        DiscoverSource::Npm => 1,
                        DiscoverSource::PyPI => 2,
                        DiscoverSource::Homebrew => 3,
                        DiscoverSource::Apt => 4,
                        DiscoverSource::Go => 5,
                        DiscoverSource::GitHub => 6,
                        DiscoverSource::AI => 7,
                    };
                    source_order(&a.source)
                        .cmp(&source_order(&b.source))
                        .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
                });
            }
        }
        // Reset selection to top after sorting
        self.discover_selected = 0;
    }

    // ========================================================================
    // Discover URL and Browser
    // ========================================================================

    /// Open URL for the selected discover result
    pub fn open_discover_url(&mut self) {
        if let Some(result) = self.selected_discover() {
            if let Some(url) = &result.url {
                // Spawn browser with stdout/stderr suppressed to avoid corrupting TUI
                let spawn_result = {
                    #[cfg(target_os = "linux")]
                    {
                        std::process::Command::new("xdg-open")
                            .arg(url)
                            .stdout(std::process::Stdio::null())
                            .stderr(std::process::Stdio::null())
                            .spawn()
                    }

                    #[cfg(target_os = "macos")]
                    {
                        std::process::Command::new("open")
                            .arg(url)
                            .stdout(std::process::Stdio::null())
                            .stderr(std::process::Stdio::null())
                            .spawn()
                    }

                    #[cfg(target_os = "windows")]
                    {
                        std::process::Command::new("cmd")
                            .args(["/C", "start", "", url])
                            .stdout(std::process::Stdio::null())
                            .stderr(std::process::Stdio::null())
                            .spawn()
                    }
                };

                match spawn_result {
                    Ok(_) => self.set_status(format!("Opening {}", url), false),
                    Err(e) => self.set_error(format!("Failed to open URL: {}", e)),
                }
            } else {
                self.set_status("No URL available for this result", false);
            }
        }
    }

    // ========================================================================
    // Discover Source Filters
    // ========================================================================

    /// Toggle AI search mode for discover
    pub fn toggle_discover_ai(&mut self) {
        self.discover_ai_enabled = !self.discover_ai_enabled;
        self.discover_history_index = None; // Reset history navigation
    }

    /// Toggle a source filter for discover search
    pub fn toggle_discover_source_filter(&mut self, source: &str) {
        if self.discover_source_filters.contains(source) {
            self.discover_source_filters.remove(source);
        } else {
            self.discover_source_filters.insert(source.to_string());
        }
        self.discover_history_index = None; // Reset history navigation
    }

    /// Get all available discover sources based on config
    pub fn get_available_discover_sources(
        &self,
    ) -> Vec<(&'static str, &'static str, &'static str)> {
        let config = HoardConfig::load().unwrap_or_default();
        let mut sources = Vec::new();

        // Only show sources that are enabled in config and have search capability
        if config.sources.cargo {
            sources.push(("cargo", "🦀", "cargo"));
        }
        if config.sources.npm {
            sources.push(("npm", "📦", "npm"));
        }
        if config.sources.pip {
            sources.push(("pip", "🐍", "pip"));
        }
        if config.sources.brew {
            sources.push(("brew", "🍺", "brew"));
        }
        if config.sources.apt {
            sources.push(("apt", "📋", "apt"));
        }
        if config.sources.go {
            sources.push(("go", "🐹", "go"));
        }
        // GitHub is always available if gh CLI is installed
        if self.gh_available {
            sources.push(("github", "🐙", "GitHub"));
        }

        sources
    }

    /// Refresh discover source filters from config (call after config changes)
    pub fn refresh_discover_sources(&mut self) {
        let config = HoardConfig::load().unwrap_or_default();
        let enabled = config.sources.enabled_sources();

        // Remove any filters that are no longer available
        self.discover_source_filters
            .retain(|s| enabled.contains(&s.as_str()) || s == "github");

        // Add github if gh is available and not already present
        if self.gh_available && !self.discover_source_filters.contains("github") {
            self.discover_source_filters.insert("github".to_string());
        }
    }

    /// Check if a source is enabled for discover search
    pub fn is_discover_source_enabled(&self, source: &str) -> bool {
        self.discover_source_filters.contains(source)
    }

    // ========================================================================
    // Discover Search History
    // ========================================================================

    /// Navigate to previous (older) search in history
    pub fn discover_history_up(&mut self) {
        if self.discover_history.is_empty() {
            return;
        }

        match self.discover_history_index {
            None => {
                // First time going into history - save current state and go to most recent
                self.discover_history_index = Some(0);
            }
            Some(idx) => {
                // Go to older entry
                if idx + 1 < self.discover_history.len() {
                    self.discover_history_index = Some(idx + 1);
                }
            }
        }

        // Apply the history entry
        self.apply_history_entry();
    }

    /// Navigate to next (newer) search in history
    pub fn discover_history_down(&mut self) {
        if self.discover_history.is_empty() {
            return;
        }

        match self.discover_history_index {
            None => {
                // Already at "new search" state, nothing to do
            }
            Some(0) => {
                // At most recent - go back to "new search"
                self.discover_history_index = None;
                self.discover_query.clear();
                // Reset to default filters from config
                if let Ok(config) = HoardConfig::load() {
                    self.discover_source_filters = config
                        .sources
                        .enabled_sources()
                        .into_iter()
                        .map(String::from)
                        .collect();
                }
                self.discover_ai_enabled = false;
            }
            Some(idx) => {
                // Go to newer entry
                self.discover_history_index = Some(idx - 1);
                self.apply_history_entry();
            }
        }
    }

    /// Apply the current history entry to search state
    fn apply_history_entry(&mut self) {
        if let Some(idx) = self.discover_history_index
            && let Some(entry) = self.discover_history.get(idx)
        {
            self.discover_query = entry.query.clone();
            self.discover_ai_enabled = entry.ai_enabled;
            self.discover_source_filters = entry.source_filters.iter().cloned().collect();
        }
    }

    /// Save current search to history (called when search is executed)
    pub fn save_discover_search_to_history(&mut self, db: &crate::db::Database) {
        let query = self.discover_query.trim();
        if query.is_empty() {
            return;
        }

        let filters: Vec<String> = self.discover_source_filters.iter().cloned().collect();

        // Save to database
        if let Ok(id) = db.save_discover_search(query, self.discover_ai_enabled, &filters) {
            // Add to in-memory history (prepend as most recent)
            self.discover_history.insert(
                0,
                crate::db::DiscoverSearchEntry {
                    id,
                    query: query.to_string(),
                    ai_enabled: self.discover_ai_enabled,
                    source_filters: filters,
                    created_at: chrono::Utc::now().to_rfc3339(),
                },
            );

            // Keep only last 100 entries in memory
            if self.discover_history.len() > 100 {
                self.discover_history.truncate(100);
            }
        }

        // Reset history navigation index
        self.discover_history_index = None;
    }

    // ========================================================================
    // Discover Search Operations
    // ========================================================================

    /// Start a discover search operation
    pub fn start_discover_search(&mut self) {
        let query = self.discover_query.trim().to_string();
        if query.is_empty() {
            return;
        }

        // Load config only to check AI provider availability
        let config = HoardConfig::load().unwrap_or_default();

        let mut source_names: Vec<String> = Vec::new();

        // Use app state for AI toggle instead of prefix
        if self.discover_ai_enabled {
            // AI-only search
            if config.ai.provider == AiProvider::None {
                self.set_status("No AI provider configured", true);
                return;
            }
            source_names.push("AI".to_string());
        } else {
            // Standard multi-source search using app's source filters
            for source in &self.discover_source_filters {
                match source.as_str() {
                    "cargo" => source_names.push("crates.io".to_string()),
                    "npm" => source_names.push("npm".to_string()),
                    "pip" => source_names.push("PyPI".to_string()),
                    "brew" => source_names.push("Homebrew".to_string()),
                    "apt" => source_names.push("apt".to_string()),
                    "go" => {
                        // Go search is handled via GitHub search with language filter
                        // We add GitHub to search for Go projects
                        if self.gh_available {
                            source_names.push("GitHub".to_string());
                        }
                    }
                    "github" if self.gh_available => {
                        source_names.push("GitHub".to_string());
                    }
                    _ => {} // Skip unknown sources
                }
            }
        }

        if source_names.is_empty() {
            self.set_status("No search sources enabled", true);
            return;
        }

        // Schedule the search operation
        self.schedule_op(BackgroundOp::DiscoverSearch {
            query: query.clone(),
            step: 0,
            source_names,
        });
    }

    /// Execute one step of a discover search operation
    pub(super) fn execute_discover_search_step(
        &mut self,
        db: &Database,
        query: String,
        step: usize,
        source_names: Vec<String>,
    ) -> bool {
        use crate::discover::{
            AptSearch, BrewSearch, CratesIoSearch, GitHubSearch, NpmSearch, PyPISearch,
            SearchSource,
        };

        // Initialize on first step
        if step == 0 {
            self.discover_results.clear();
            self.discover_loading = true;
            self.discover_selected = 0;
        }

        // Get the current source name
        let total_steps = source_names.len();
        let current_source = &source_names[step];

        // Handle AI search specially - it runs async
        if current_source == "AI" {
            return self.handle_ai_search_step(db, query, step, source_names);
        }

        // Update progress for UI
        self.loading_progress = LoadingProgress {
            current_step: step + 1,
            total_steps,
            step_name: current_source.clone(),
            found_count: self.discover_results.len(),
        };

        // Create the appropriate search source and execute (non-AI sources)
        let search_result: Result<Vec<crate::discover::DiscoverResult>, _> =
            match current_source.as_str() {
                "crates.io" => CratesIoSearch.search(&query, 20),
                "npm" => NpmSearch.search(&query, 20),
                "PyPI" => PyPISearch.search(&query, 20),
                "Homebrew" => BrewSearch.search(&query, 20),
                "apt" => AptSearch.search(&query, 20),
                "GitHub" => GitHubSearch.search(&query, 20),
                _ => Ok(Vec::new()),
            };

        // Convert and accumulate results
        if let Ok(results) = search_result {
            for r in results {
                let install_options: Vec<InstallOption> = r
                    .install_options
                    .into_iter()
                    .map(|o| InstallOption {
                        source: o.source,
                        install_command: o.install_command,
                    })
                    .collect();

                self.discover_results.push(DiscoverResult {
                    name: r.name,
                    description: r.description,
                    source: r.source,
                    stars: r.stars,
                    url: r.url,
                    language: r.language,
                    install_options,
                });
            }
        }

        // Check if there are more steps
        let next_step = step + 1;
        if next_step < total_steps {
            // More sources to search
            self.background_op = Some(BackgroundOp::DiscoverSearch {
                query,
                step: next_step,
                source_names,
            });
            true
        } else {
            // All done - deduplicate and finalize
            self.finalize_discover_search()
        }
    }

    /// Handle async AI search step
    fn handle_ai_search_step(
        &mut self,
        db: &Database,
        query: String,
        step: usize,
        source_names: Vec<String>,
    ) -> bool {
        use crate::discover::{AiSearch, SearchSource};

        let total_steps = source_names.len();

        // Check if we already have an AI operation running
        if let Some(ref ai_op) = self.ai_operation {
            // Check elapsed time for progress display
            let elapsed = ai_op.start_time.elapsed();
            let elapsed_secs = elapsed.as_secs();

            self.loading_progress = LoadingProgress {
                current_step: step + 1,
                total_steps,
                step_name: format!("AI ({}.{}s)", elapsed_secs, elapsed.subsec_millis() / 100),
                found_count: self.discover_results.len(),
            };

            // Check if the thread is finished (non-blocking)
            if ai_op.thread_handle.is_finished() {
                // Take ownership of the operation
                let ai_op = self.ai_operation.take().unwrap();

                match ai_op.thread_handle.join() {
                    Ok(Ok(results)) => {
                        // Accumulate AI results
                        for r in results {
                            let install_options: Vec<InstallOption> = r
                                .install_options
                                .into_iter()
                                .map(|o| InstallOption {
                                    source: o.source,
                                    install_command: o.install_command,
                                })
                                .collect();

                            self.discover_results.push(DiscoverResult {
                                name: r.name,
                                description: r.description,
                                source: r.source,
                                stars: r.stars,
                                url: r.url,
                                language: r.language,
                                install_options,
                            });
                        }
                        self.set_status(
                            format!("AI search completed in {:.1}s", elapsed.as_secs_f32()),
                            false,
                        );
                    }
                    Ok(Err(e)) => {
                        self.set_status(format!("AI search failed: {}", e), true);
                    }
                    Err(_) => {
                        self.set_status("AI search thread panicked", true);
                    }
                }

                // Move to next step
                let next_step = step + 1;
                if next_step < total_steps {
                    self.background_op = Some(BackgroundOp::DiscoverSearch {
                        query,
                        step: next_step,
                        source_names,
                    });
                    return true;
                } else {
                    // AI was the last step - finalize
                    return self.finalize_discover_search();
                }
            }

            // Thread still running - keep polling
            self.background_op = Some(BackgroundOp::DiscoverSearch {
                query,
                step,
                source_names,
            });
            return true;
        }

        // No AI operation running - start one
        // Get installed tools for the prompt
        let installed_tools: Vec<String> = db
            .list_tools(false, None)
            .unwrap_or_default()
            .iter()
            .map(|t| t.name.clone())
            .collect();

        // Get enabled sources for AI to recommend from
        let enabled_sources: Vec<String> = self.discover_source_filters.iter().cloned().collect();

        let query_clone = query.clone();

        // Spawn the AI search in a background thread
        let thread_handle = std::thread::spawn(move || {
            let ai_search = AiSearch::new(installed_tools, enabled_sources);
            ai_search
                .search(&query_clone, 20)
                .map_err(|e| e.to_string())
        });

        self.ai_operation = Some(AiOperation {
            start_time: std::time::Instant::now(),
            thread_handle,
        });

        self.loading_progress = LoadingProgress {
            current_step: step + 1,
            total_steps,
            step_name: "AI (0.0s)".to_string(),
            found_count: self.discover_results.len(),
        };

        // Keep polling
        self.background_op = Some(BackgroundOp::DiscoverSearch {
            query,
            step,
            source_names,
        });
        true
    }

    /// Finalize discover search results (deduplication and status)
    pub(super) fn finalize_discover_search(&mut self) -> bool {
        use crate::discover::deduplicate_results as dedup;

        self.discover_loading = false;

        // Convert to discover module format, deduplicate, then convert back
        let dedup_input: Vec<crate::discover::DiscoverResult> = self
            .discover_results
            .drain(..)
            .map(|r| {
                let mut dr = crate::discover::DiscoverResult::new(
                    r.name,
                    r.description,
                    r.source,
                    r.install_options
                        .first()
                        .map(|o| o.install_command.clone())
                        .unwrap_or_default(),
                );
                dr.stars = r.stars;
                dr.url = r.url;
                // Add remaining install options
                for opt in r.install_options.into_iter().skip(1) {
                    dr.install_options.push(crate::discover::InstallOption {
                        source: opt.source,
                        install_command: opt.install_command,
                    });
                }
                dr
            })
            .collect();

        let deduped = dedup(dedup_input);

        // Convert back
        for r in deduped {
            let install_options: Vec<InstallOption> = r
                .install_options
                .into_iter()
                .map(|o| InstallOption {
                    source: o.source,
                    install_command: o.install_command,
                })
                .collect();

            self.discover_results.push(DiscoverResult {
                name: r.name,
                description: r.description,
                source: r.source,
                stars: r.stars,
                url: r.url,
                language: r.language,
                install_options,
            });
        }

        let count = self.discover_results.len();
        if count == 0 {
            self.set_status("No results found", false);
        } else {
            self.set_status(format!("Found {} tool(s)", count), false);
        }
        false
    }
}
