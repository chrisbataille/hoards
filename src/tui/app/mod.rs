//! Application state for the TUI
//!
//! This module is organized into submodules:
//! - `types`: Enums, state structs, and constants
//! - `traits`: SelectableList trait for unified navigation
//! - `components`: CacheManager, CommandPalette, fuzzy matching
//! - `list_state`: BundleState and related list management
//! - `actions`: Selection, undo/redo, install/uninstall actions
//! - `config_menu`: Configuration menu operations
//! - `discover`: Discover tab operations
//! - `readme`: README popup operations

mod actions;
mod command_exec;
mod components;
mod config_menu;
mod discover;
mod list_state;
mod readme;
#[cfg(test)]
mod tests;
mod traits;
mod types;

use std::collections::{HashMap, HashSet};

use anyhow::Result;

use crate::Update;
use crate::config::{AiProvider, HoardConfig};
use crate::db::Database;
use crate::models::{Bundle, Tool};
use crate::sources::PackageManagerStatus;

// Re-exports
pub use components::{CacheManager, CommandPalette, fuzzy_match, fuzzy_match_positions};
pub use list_state::BundleState;
pub use traits::SelectableList;
pub use types::{
    ActionHistory, BackgroundOp, ConfigMenuState, ConfigSection, DiscoverResult, DiscoverSortBy,
    DiscoverSource, ErrorModal, InputMode, InstallOption, InstallResult, InstallTask,
    LabelFilterSort, LoadingProgress, Notification, NotificationLevel, OutputLine, OutputLineType,
    PACKAGE_MANAGERS, PendingAction, ReadmePopup, SortBy, StatusMessage, Tab, config_menu_layout,
};

/// Tracks an async AI operation running in a background thread
pub struct AiOperation {
    pub start_time: std::time::Instant,
    pub thread_handle:
        std::thread::JoinHandle<Result<Vec<crate::discover::DiscoverResult>, String>>,
}

/// Tracks an async AI error analysis operation
pub struct AiErrorAnalysis {
    pub start_time: std::time::Instant,
    pub thread_handle: std::thread::JoinHandle<Result<String, String>>,
}

/// Tracks a running install command that can be cancelled
pub struct InstallOperation {
    pub task_name: String,
    pub child: std::process::Child,
    pub start_time: std::time::Instant,
    /// Receiver for stdout/stderr lines from reader threads
    pub output_receiver: std::sync::mpsc::Receiver<OutputLine>,
    /// Log file path for full output
    pub log_path: std::path::PathBuf,
    /// Log file writer
    pub log_writer: Option<std::io::BufWriter<std::fs::File>>,
}

/// Main application state
pub struct App {
    pub running: bool,
    pub tab: Tab,
    pub input_mode: InputMode,
    pub search_query: String,
    pub source_filter: Option<String>, // Filter by source (cargo, apt, etc.)
    pub label_filter: HashSet<String>, // Filter by labels (multiple allowed)
    pub favorites_only: bool,          // Filter to show only favorites
    pub show_label_filter_popup: bool, // Show label filter selection popup
    pub label_filter_selected: usize,  // Selected index in label filter popup
    pub label_filter_scroll: usize,    // Scroll offset for label filter popup
    pub label_filter_search: String,   // Search input for filtering labels
    pub label_filter_sort: types::LabelFilterSort, // Sort mode for label list
    pub show_label_edit_popup: bool,   // Show label edit popup
    pub label_edit_tool: Option<String>, // Tool being edited in label popup
    pub label_edit_input: String,      // Input field for new/search label
    pub label_edit_selected: usize,    // Selected: 0=input, 1..=suggestions, then existing labels
    pub label_edit_labels: Vec<String>, // Current labels on the tool being edited
    pub label_edit_suggestions: Vec<String>, // Fuzzy-matched existing labels from all tools

    // Tool list state
    pub all_tools: Vec<Tool>, // All tools for current tab (unfiltered)
    pub tools: Vec<Tool>,     // Filtered/sorted tools to display
    pub selected_index: usize,
    pub list_offset: usize,

    // Extracted components
    pub cache: CacheManager,     // Usage, GitHub info, labels caches
    pub bundles: BundleState,    // Bundle list and selection
    pub command: CommandPalette, // Command palette input and history

    // Updates state
    pub available_updates: HashMap<String, Update>,
    pub updates_checked: bool,
    pub updates_loading: bool,

    // UI state
    pub show_help: bool,
    pub show_details_popup: bool,
    pub sort_by: SortBy,
    pub theme_variant: super::theme::ThemeVariant,

    // Multi-selection
    pub selected_tools: HashSet<String>,

    // Actions
    pub pending_action: Option<PendingAction>,
    pub status_message: Option<StatusMessage>,

    // Background operations (executed by main loop with loading indicator)
    pub background_op: Option<BackgroundOp>,
    pub loading_progress: LoadingProgress,

    // Undo/redo history
    pub history: ActionHistory,

    // Mouse interaction state
    pub last_list_area: Option<(u16, u16, u16, u16)>, // (x, y, width, height) of tool list
    pub last_tab_area: Option<(u16, u16, u16, u16)>,  // (x, y, width, height) of tabs
    pub last_config_popup_area: Option<(u16, u16, u16, u16)>, // (x, y, width, height) of config popup

    // Feature availability status (for footer display)
    pub ai_available: bool,                     // AI provider is configured
    pub gh_available: bool,                     // GitHub CLI is installed
    pub package_managers: PackageManagerStatus, // Package manager availability and versions

    // Last sync timestamp
    pub last_sync: Option<chrono::DateTime<chrono::Utc>>,

    // Discover tab state
    pub discover_query: String,
    pub discover_results: Vec<DiscoverResult>,
    pub discover_selected: usize,
    pub discover_loading: bool,
    pub discover_sort_by: DiscoverSortBy,
    pub discover_ai_enabled: bool,                // AI search toggle
    pub discover_source_filters: HashSet<String>, // Selected source filters
    pub discover_history: Vec<crate::db::DiscoverSearchEntry>, // Search history
    pub discover_history_index: Option<usize>,    // Current position in history (None = new)
    pub discover_filter_focus: Option<usize>,     // Focused filter chip index

    // Config menu state
    pub show_config_menu: bool,
    pub config_menu: ConfigMenuState,

    // Toast notifications (auto-dismiss)
    pub notifications: Vec<Notification>,

    // Error modal (blocks until dismissed)
    pub error_modal: Option<ErrorModal>,

    // README popup
    pub readme_popup: Option<ReadmePopup>,

    // Async AI operation tracking
    pub ai_operation: Option<AiOperation>,

    // Async AI error analysis tracking
    pub ai_error_analysis: Option<AiErrorAnalysis>,

    // Async install operation tracking (for cancellation)
    pub install_operation: Option<InstallOperation>,

    // Sudo password input for apt/snap installs
    pub password_input: String,
    pub pending_sudo_tasks: Option<(Vec<InstallTask>, bool)>, // (tasks, is_update)

    // Live output from install/update commands
    pub install_output: Vec<OutputLine>,
    pub install_output_scroll: usize,

    // Last failed install log path (for AI analysis)
    pub last_install_log: Option<std::path::PathBuf>,
}

impl App {
    pub fn new(db: &Database) -> Result<Self> {
        let all_tools = db.list_tools(true, None)?; // installed only
        let bundles = db.list_bundles()?;
        let tools = all_tools.clone();

        // Load config and check feature availability
        let config_exists = HoardConfig::exists();
        let config = HoardConfig::load().unwrap_or_default();
        let ai_available = config.ai.provider != AiProvider::None;
        let gh_available = which::which("gh").is_ok();
        let package_managers = PackageManagerStatus::detect();

        // Get theme from config
        let theme_variant = super::theme::ThemeVariant::from_config_theme(config.tui.theme);

        // Auto-show config menu if no config file exists
        let show_config_menu = !config_exists;
        let config_menu = if show_config_menu {
            ConfigMenuState::from_config(&config)
        } else {
            ConfigMenuState::default()
        };

        Ok(Self {
            running: true,
            tab: Tab::Installed,
            input_mode: InputMode::Normal,
            search_query: String::new(),
            source_filter: None,
            label_filter: HashSet::new(),
            favorites_only: false,
            show_label_filter_popup: false,
            label_filter_selected: 0,
            label_filter_scroll: 0,
            label_filter_search: String::new(),
            label_filter_sort: types::LabelFilterSort::default(),
            show_label_edit_popup: false,
            label_edit_tool: None,
            label_edit_input: String::new(),
            label_edit_selected: 0,
            label_edit_labels: Vec::new(),
            label_edit_suggestions: Vec::new(),
            all_tools,
            tools,
            selected_index: 0,
            list_offset: 0,
            cache: CacheManager::new(db),
            bundles: BundleState::new(bundles),
            command: CommandPalette::new(),
            available_updates: HashMap::new(),
            updates_checked: false,
            updates_loading: false,
            show_help: false,
            show_details_popup: false,
            sort_by: SortBy::default(),
            theme_variant,
            selected_tools: HashSet::new(),
            pending_action: None,
            status_message: None,
            background_op: None,
            loading_progress: LoadingProgress::default(),
            history: ActionHistory::new(50), // Keep 50 actions max
            last_list_area: None,
            last_tab_area: None,
            last_config_popup_area: None,
            ai_available,
            gh_available,
            package_managers,
            last_sync: db.get_last_sync_time().ok().flatten(),
            discover_query: String::new(),
            discover_results: Vec::new(),
            discover_selected: 0,
            discover_loading: false,
            discover_sort_by: DiscoverSortBy::default(),
            discover_ai_enabled: false,
            discover_source_filters: config
                .sources
                .enabled_sources()
                .into_iter()
                .map(String::from)
                .collect(),
            discover_history: db.get_discover_history(100).unwrap_or_default(),
            discover_history_index: None,
            discover_filter_focus: None,
            show_config_menu,
            config_menu,
            notifications: Vec::new(),
            error_modal: None,
            readme_popup: None,
            ai_operation: None,
            ai_error_analysis: None,
            install_operation: None,
            password_input: String::new(),
            pending_sudo_tasks: None,
            install_output: Vec::new(),
            install_output_scroll: 0,
            last_install_log: None,
        })
    }

    // ========================================================================
    // Core Lifecycle
    // ========================================================================

    /// Quit the application
    pub fn quit(&mut self) {
        self.running = false;
    }

    /// Cycle to the next theme
    pub fn cycle_theme(&mut self) {
        self.theme_variant = self.theme_variant.next();
        self.set_status(
            format!("Theme: {}", self.theme_variant.display_name()),
            false,
        );
    }

    /// Get the current theme
    pub fn theme(&self) -> super::theme::Theme {
        self.theme_variant.theme()
    }

    // ========================================================================
    // Tab Navigation
    // ========================================================================

    /// Switch to a specific tab
    pub fn switch_tab(&mut self, tab: Tab, db: &Database) {
        if self.tab != tab {
            self.tab = tab;
            self.selected_index = 0;
            self.list_offset = 0;
            self.search_query.clear();
            self.refresh_tools(db);
        }
    }

    /// Go to next tab
    pub fn next_tab(&mut self, db: &Database) {
        let next_index = (self.tab.index() + 1) % Tab::all().len();
        if let Some(tab) = Tab::from_index(next_index) {
            self.switch_tab(tab, db);
        }
    }

    /// Go to previous tab
    pub fn prev_tab(&mut self, db: &Database) {
        let prev_index = if self.tab.index() == 0 {
            Tab::all().len() - 1
        } else {
            self.tab.index() - 1
        };
        if let Some(tab) = Tab::from_index(prev_index) {
            self.switch_tab(tab, db);
        }
    }

    // ========================================================================
    // Tool List Management
    // ========================================================================

    /// Refresh tool list based on current tab
    pub fn refresh_tools(&mut self, db: &Database) {
        let result = match self.tab {
            Tab::Installed => db.list_tools(true, None),
            Tab::Available => db.list_tools(false, None),
            Tab::Updates => {
                // For Updates tab, only show tools with available updates
                if self.updates_checked {
                    let update_names: HashSet<_> = self.available_updates.keys().cloned().collect();
                    db.list_tools(true, None).map(|mut tools| {
                        tools.retain(|t| update_names.contains(&t.name));
                        tools
                    })
                } else {
                    // No updates checked yet, show empty list
                    Ok(Vec::new())
                }
            }
            Tab::Bundles => db.list_tools(true, None),
            Tab::Discover => Ok(Vec::new()), // Discover has its own search results
        };

        if let Ok(mut tools) = result {
            // For Available tab, filter to only non-installed tools
            if self.tab == Tab::Available {
                tools.retain(|t| !t.is_installed);
            }
            self.all_tools = tools;
            self.apply_filter_and_sort();
        }

        // Also refresh bundles if on that tab
        if self.tab == Tab::Bundles {
            let _ = self.bundles.reload(db);
        }
    }

    /// Get update info for a tool if available
    pub fn get_update(&self, tool_name: &str) -> Option<&Update> {
        self.available_updates.get(tool_name)
    }

    /// Apply current search filter and sort to tools
    pub fn apply_filter_and_sort(&mut self) {
        // Start with all tools, optionally filtered by source, label, and favorites
        let source_filtered: Vec<&Tool> = self
            .all_tools
            .iter()
            .filter(|t| {
                // Filter by source if set
                if let Some(ref source) = self.source_filter
                    && format!("{:?}", t.source).to_lowercase() != *source
                {
                    return false;
                }
                // Filter by labels if set (tool must have ALL selected labels)
                if !self.label_filter.is_empty() {
                    let tool_labels = self.cache.labels_cache.get(&t.name);
                    let has_all_labels = tool_labels.is_some_and(|labels: &Vec<String>| {
                        self.label_filter.iter().all(|l| labels.contains(l))
                    });
                    if !has_all_labels {
                        return false;
                    }
                }
                // Filter by favorites if enabled
                if self.favorites_only && !t.is_favorite {
                    return false;
                }
                true
            })
            .collect();

        // Apply fuzzy search filter
        let mut filtered: Vec<(Tool, i32)> = if self.search_query.is_empty() {
            source_filtered
                .into_iter()
                .map(|t| (t.clone(), 0))
                .collect()
        } else {
            // Fuzzy match against name, description, and category
            source_filtered
                .into_iter()
                .filter_map(|t| {
                    // Get best score across all fields
                    let name_score = fuzzy_match(&self.search_query, &t.name);
                    let desc_score = t
                        .description
                        .as_ref()
                        .and_then(|d| fuzzy_match(&self.search_query, d));
                    let cat_score = t
                        .category
                        .as_ref()
                        .and_then(|c| fuzzy_match(&self.search_query, c));

                    // Use best score (name matches get priority bonus)
                    let score = [
                        name_score.map(|s| s + 10), // Bonus for name match
                        desc_score,
                        cat_score,
                    ]
                    .into_iter()
                    .flatten()
                    .max();

                    score.map(|s| (t.clone(), s))
                })
                .collect()
        };

        // Sort by fuzzy score when searching, otherwise by user preference
        if !self.search_query.is_empty() {
            // Sort by score descending (best matches first)
            filtered.sort_by(|a, b| b.1.cmp(&a.1));
        } else {
            // Sort by user preference
            match self.sort_by {
                SortBy::Name => filtered.sort_by(|a, b| a.0.name.cmp(&b.0.name)),
                SortBy::Usage => {
                    let usage = &self.cache.usage_data;
                    filtered.sort_by(|a, b| {
                        let a_usage = usage.get(&a.0.name).map(|u| u.use_count).unwrap_or(0);
                        let b_usage = usage.get(&b.0.name).map(|u| u.use_count).unwrap_or(0);
                        b_usage.cmp(&a_usage) // Descending
                    });
                }
                SortBy::Recent => {
                    let usage = &self.cache.usage_data;
                    filtered.sort_by(|a, b| {
                        let a_last = usage.get(&a.0.name).and_then(|u| u.last_used.as_ref());
                        let b_last = usage.get(&b.0.name).and_then(|u| u.last_used.as_ref());
                        // Sort by last_used descending (most recent first)
                        // Tools with no usage go to the end
                        match (a_last, b_last) {
                            (Some(a), Some(b)) => b.cmp(a), // Descending: most recent first
                            (Some(_), None) => std::cmp::Ordering::Less, // a has usage, comes first
                            (None, Some(_)) => std::cmp::Ordering::Greater, // b has usage, comes first
                            (None, None) => a.0.name.cmp(&b.0.name),        // Alphabetical fallback
                        }
                    });
                }
            }
        }

        self.tools = filtered.into_iter().map(|(t, _)| t).collect();

        // Adjust selection if needed
        if self.selected_index >= self.tools.len() {
            self.selected_index = self.tools.len().saturating_sub(1);
        }
    }

    /// Cycle through sort options
    pub fn cycle_sort(&mut self) {
        self.sort_by = self.sort_by.next();
        self.apply_filter_and_sort();
    }

    // ========================================================================
    // Tool List Navigation
    // ========================================================================

    /// Move selection down
    pub fn select_next(&mut self) {
        if !self.tools.is_empty() {
            self.selected_index = (self.selected_index + 1).min(self.tools.len() - 1);
        }
    }

    /// Move selection up
    pub fn select_prev(&mut self) {
        self.selected_index = self.selected_index.saturating_sub(1);
    }

    /// Move to next match with wrapping (vim n)
    pub fn search_next(&mut self) {
        if self.tools.is_empty() {
            return;
        }
        // Move to next item, wrap to start if at end
        if self.selected_index + 1 >= self.tools.len() {
            self.selected_index = 0;
            self.set_status("Search wrapped to top".to_string(), false);
        } else {
            self.selected_index += 1;
        }
    }

    /// Move to previous match with wrapping (vim N)
    pub fn search_prev(&mut self) {
        if self.tools.is_empty() {
            return;
        }
        // Move to previous item, wrap to end if at start
        if self.selected_index == 0 {
            self.selected_index = self.tools.len() - 1;
            self.set_status("Search wrapped to bottom".to_string(), false);
        } else {
            self.selected_index -= 1;
        }
    }

    /// Enter jump-to-letter mode (vim f)
    pub fn enter_jump_mode(&mut self) {
        self.input_mode = InputMode::JumpToLetter;
    }

    /// Exit jump-to-letter mode
    pub fn exit_jump_mode(&mut self) {
        self.input_mode = InputMode::Normal;
    }

    /// Jump to first tool starting with the given letter
    pub fn jump_to_letter(&mut self, letter: char) {
        let letter = letter.to_ascii_lowercase();
        for (i, tool) in self.tools.iter().enumerate() {
            if tool.name.to_lowercase().starts_with(letter) {
                self.selected_index = i;
                self.set_status(format!("Jumped to '{}'", letter), false);
                break;
            }
        }
        self.exit_jump_mode();
    }

    /// Toggle favorite status for the selected tool
    pub fn toggle_favorite(&mut self, db: &Database) {
        if let Some(tool) = self.selected_tool() {
            let name = tool.name.clone();
            let new_status = !tool.is_favorite;

            match db.set_tool_favorite(&name, new_status) {
                Ok(true) => {
                    // Update local state
                    for t in &mut self.all_tools {
                        if t.name == name {
                            t.is_favorite = new_status;
                            break;
                        }
                    }
                    for t in &mut self.tools {
                        if t.name == name {
                            t.is_favorite = new_status;
                            break;
                        }
                    }
                    let status = if new_status {
                        "★ Added to favorites"
                    } else {
                        "Removed from favorites"
                    };
                    self.set_status(format!("{}: {}", name, status), false);
                }
                Ok(false) => {
                    self.set_status(format!("Tool not found: {}", name), true);
                }
                Err(e) => {
                    self.set_status(format!("Failed to update favorite: {}", e), true);
                }
            }
        }
    }

    /// Move selection to top
    pub fn select_first(&mut self) {
        self.selected_index = 0;
    }

    /// Move selection to bottom
    pub fn select_last(&mut self) {
        if !self.tools.is_empty() {
            self.selected_index = self.tools.len() - 1;
        }
    }

    // ========================================================================
    // Bundle Navigation
    // ========================================================================

    /// Move bundle selection down
    pub fn select_next_bundle(&mut self) {
        self.bundles.next();
    }

    /// Move bundle selection up
    pub fn select_prev_bundle(&mut self) {
        self.bundles.prev();
    }

    /// Move bundle selection to top
    pub fn select_first_bundle(&mut self) {
        self.bundles.first();
    }

    /// Move bundle selection to bottom
    pub fn select_last_bundle(&mut self) {
        self.bundles.last();
    }

    /// Get the currently selected bundle
    pub fn selected_bundle(&self) -> Option<&Bundle> {
        self.bundles.selected_bundle()
    }

    /// Get the currently selected tool
    pub fn selected_tool(&self) -> Option<&Tool> {
        self.tools.get(self.selected_index)
    }

    /// Get usage for a tool
    pub fn get_usage(&self, tool_name: &str) -> Option<&crate::db::ToolUsage> {
        self.cache.usage_data.get(tool_name)
    }

    /// Get GitHub info for a tool (cached, or fetch from db)
    pub fn get_github_info(
        &mut self,
        tool_name: &str,
        db: &Database,
    ) -> Option<&crate::db::GitHubInfo> {
        if !self.cache.github_cache.contains_key(tool_name)
            && let Ok(Some(info)) = db.get_github_info(tool_name)
        {
            self.cache.github_cache.insert(tool_name.to_string(), info);
        }
        self.cache.github_cache.get(tool_name)
    }

    // ========================================================================
    // Help and Search
    // ========================================================================

    /// Toggle help overlay
    pub fn toggle_help(&mut self) {
        self.show_help = !self.show_help;
    }

    /// Enter search mode
    pub fn enter_search(&mut self) {
        self.record_filter(); // Record current filter for undo
        self.input_mode = InputMode::Search;
        self.search_query.clear();
    }

    /// Exit search mode
    pub fn exit_search(&mut self) {
        self.input_mode = InputMode::Normal;
    }

    /// Add character to search query and filter
    pub fn search_push(&mut self, c: char) {
        self.search_query.push(c);
        self.apply_filter_and_sort();
    }

    /// Remove last character from search query and filter
    pub fn search_pop(&mut self) {
        self.search_query.pop();
        self.apply_filter_and_sort();
    }

    /// Clear search and show all tools
    pub fn clear_search(&mut self) {
        if !self.search_query.is_empty() {
            self.record_filter(); // Record for undo
            self.search_query.clear();
            self.apply_filter_and_sort();
        }
    }

    // ========================================================================
    // Details Popup
    // ========================================================================

    /// Toggle details popup (for narrow terminals)
    pub fn toggle_details_popup(&mut self) {
        self.show_details_popup = !self.show_details_popup;
    }

    /// Close details popup
    pub fn close_details_popup(&mut self) {
        self.show_details_popup = false;
    }

    // ========================================================================
    // Mouse Support
    // ========================================================================

    /// Set the list area for mouse interaction
    pub fn set_list_area(&mut self, x: u16, y: u16, width: u16, height: u16) {
        self.last_list_area = Some((x, y, width, height));
    }

    /// Set the tab area for mouse interaction
    pub fn set_tab_area(&mut self, x: u16, y: u16, width: u16, height: u16) {
        self.last_tab_area = Some((x, y, width, height));
    }

    /// Handle mouse click on list item
    pub fn click_list_item(&mut self, row: u16) {
        if self.tab == Tab::Bundles {
            // Handle bundle list clicks
            let target_index = row as usize; // Bundles don't scroll currently
            self.bundles.select(target_index);
        } else if self.tab == Tab::Discover {
            // Handle discover list clicks
            let target_index = row as usize;
            if target_index < self.discover_results.len() {
                self.discover_selected = target_index;
            }
        } else {
            // Handle tool list clicks
            let target_index = self.list_offset + row as usize;
            if target_index < self.tools.len() {
                self.selected_index = target_index;
            }
        }
    }

    /// Handle mouse click on tab
    pub fn click_tab(&mut self, x: u16, db: &Database) {
        if let Some((area_x, _, _, _)) = self.last_tab_area {
            // Account for block border (1 char on left)
            let content_start = area_x + 1;
            let relative_x = x.saturating_sub(content_start) as usize;

            // Tab layout (with padding("", "") set in UI):
            // Each tab: " title " = title.len() + 2
            // Divider between tabs: "│" (1 char)
            let tabs = Tab::all();
            let mut pos = 0;

            for (i, tab) in tabs.iter().enumerate() {
                let tab_width = tab.title().len() + 2; // " title "

                if relative_x >= pos && relative_x < pos + tab_width {
                    self.switch_tab(*tab, db);
                    return;
                }

                pos += tab_width;

                // Add divider width (1 char) after each tab except the last
                if i < tabs.len() - 1 {
                    pos += 1;
                }
            }
        }
    }

    /// Check if click is in list area and return relative row
    pub fn get_list_row(&self, x: u16, y: u16) -> Option<u16> {
        if let Some((area_x, area_y, width, height)) = self.last_list_area
            && x >= area_x
            && x < area_x + width
            && y >= area_y
            && y < area_y + height
        {
            // Skip header row
            if y > area_y {
                return Some(y - area_y - 1);
            }
        }
        None
    }

    /// Check if click is in tab area
    pub fn is_in_tab_area(&self, x: u16, y: u16) -> bool {
        if let Some((area_x, area_y, width, height)) = self.last_tab_area {
            x >= area_x && x < area_x + width && y >= area_y && y < area_y + height
        } else {
            false
        }
    }

    // ========================================================================
    // Status Messages
    // ========================================================================

    /// Set a status message
    pub fn set_status(&mut self, text: impl Into<String>, is_error: bool) {
        self.status_message = Some(StatusMessage {
            text: text.into(),
            is_error,
        });
    }

    /// Set an error message (convenience wrapper for set_status with is_error=true)
    pub fn set_error(&mut self, text: impl Into<String>) {
        self.set_status(text, true);
    }

    /// Clear status message
    pub fn clear_status(&mut self) {
        self.status_message = None;
    }

    // ========================================================================
    // Toast Notifications
    // ========================================================================

    /// Add an info toast notification
    pub fn notify_info(&mut self, text: impl Into<String>) {
        self.notifications.push(Notification::info(text));
    }

    /// Add a warning toast notification
    pub fn notify_warning(&mut self, text: impl Into<String>) {
        self.notifications.push(Notification::warning(text));
    }

    /// Add an error toast notification
    pub fn notify_error(&mut self, text: impl Into<String>) {
        self.notifications.push(Notification::error(text));
    }

    /// Remove expired notifications (call this in main loop tick)
    pub fn tick_notifications(&mut self) {
        self.notifications.retain(|n| !n.should_dismiss());
    }

    /// Dismiss all notifications
    pub fn clear_notifications(&mut self) {
        self.notifications.clear();
    }

    /// Start AI analysis of the last failed install
    pub fn analyze_last_error(&mut self) {
        let Some(log_path) = self.last_install_log.clone() else {
            self.notify_warning("No recent install failure to analyze");
            return;
        };

        if !log_path.exists() {
            self.notify_error(format!("Log file not found: {}", log_path.display()));
            return;
        }

        // Check if AI is configured
        match crate::config::HoardConfig::load() {
            Ok(config) if config.ai.provider == crate::config::AiProvider::None => {
                self.notify_warning("AI not configured. Run 'hoards config' to set up AI.");
                return;
            }
            Err(e) => {
                self.notify_error(format!("Failed to load config: {}", e));
                return;
            }
            _ => {}
        }

        self.schedule_op(BackgroundOp::AnalyzeError { log_path });
    }

    // ========================================================================
    // Error Modal
    // ========================================================================

    /// Show a modal error dialog (blocks until dismissed)
    pub fn show_error_modal(&mut self, title: impl Into<String>, message: impl Into<String>) {
        self.error_modal = Some(ErrorModal {
            title: title.into(),
            message: message.into(),
        });
    }

    /// Close the error modal
    pub fn close_error_modal(&mut self) {
        self.error_modal = None;
    }

    /// Check if error modal is showing
    pub fn has_error_modal(&self) -> bool {
        self.error_modal.is_some()
    }

    // ========================================================================
    // Background Operations
    // ========================================================================

    /// Schedule a background operation (will be executed by main loop)
    pub fn schedule_op(&mut self, op: BackgroundOp) {
        // Clear previous install output when starting a new install/update
        if matches!(
            op,
            BackgroundOp::ExecuteInstall { .. } | BackgroundOp::ExecuteUpdate { .. }
        ) {
            self.install_output.clear();
            self.install_output_scroll = 0;
        }
        self.background_op = Some(op);
    }

    /// Check if there's a pending background operation
    pub fn has_background_op(&self) -> bool {
        self.background_op.is_some()
    }

    /// Execute one step of the pending background operation
    /// Returns true if there are more steps to execute
    pub fn execute_background_step(&mut self, db: &Database) -> bool {
        use crate::{
            check_apt_updates, check_brew_updates, check_cargo_updates, check_npm_updates,
            check_pip_updates,
        };

        let Some(op) = self.background_op.take() else {
            return false;
        };

        match op {
            BackgroundOp::CheckUpdates { step } => {
                let checkers: &[fn() -> anyhow::Result<Vec<Update>>] = &[
                    check_cargo_updates,
                    check_pip_updates,
                    check_npm_updates,
                    check_apt_updates,
                    check_brew_updates,
                ];

                // Initialize on first step
                if step == 0 {
                    self.available_updates.clear();
                    self.updates_loading = true;
                }

                // Get tracked tool names to filter updates
                let tracked_tools: HashSet<String> = db
                    .list_tools(true, None)
                    .map(|tools| tools.into_iter().map(|t| t.name).collect())
                    .unwrap_or_default();

                // Update progress for UI
                self.loading_progress = LoadingProgress {
                    current_step: step + 1,
                    total_steps: PACKAGE_MANAGERS.len(),
                    step_name: PACKAGE_MANAGERS[step].1.to_string(),
                    found_count: self.available_updates.len(),
                };

                // Execute this step's checker - only keep updates for tracked tools
                if let Ok(updates) = checkers[step]() {
                    for update in updates {
                        if tracked_tools.contains(&update.name) {
                            self.available_updates.insert(update.name.clone(), update);
                        }
                    }
                }

                // Check if there are more steps
                let next_step = step + 1;
                if next_step < checkers.len() {
                    // More steps to go
                    self.background_op = Some(BackgroundOp::CheckUpdates { step: next_step });
                    true
                } else {
                    // All done - finalize
                    self.updates_checked = true;
                    self.updates_loading = false;
                    self.refresh_tools(db);

                    let count = self.available_updates.len();
                    if count == 0 {
                        self.set_status("All tools are up to date!", false);
                    } else {
                        self.set_status(format!("{} update(s) available", count), false);
                    }
                    false
                }
            }
            BackgroundOp::DiscoverSearch {
                query,
                step,
                source_names,
            } => self.execute_discover_search_step(db, query, step, source_names),
            BackgroundOp::ExecuteInstall {
                tasks,
                current,
                results,
            } => self.execute_install_step(db, tasks, current, results, false),
            BackgroundOp::ExecuteUpdate {
                tasks,
                current,
                results,
            } => self.execute_install_step(db, tasks, current, results, true),
            BackgroundOp::AnalyzeError { log_path } => self.execute_error_analysis_step(log_path),
        }
    }

    /// Execute AI error analysis step
    fn execute_error_analysis_step(&mut self, log_path: std::path::PathBuf) -> bool {
        // Check if we already have an AI analysis running
        if let Some(ref ai_analysis) = self.ai_error_analysis {
            // Update progress
            let elapsed = ai_analysis.start_time.elapsed().as_secs();
            self.loading_progress = LoadingProgress {
                current_step: 1,
                total_steps: 1,
                step_name: format!("Analyzing error... ({}s)", elapsed),
                found_count: 0,
            };

            // Check if done
            if ai_analysis.thread_handle.is_finished() {
                let analysis = self.ai_error_analysis.take().unwrap();
                match analysis.thread_handle.join() {
                    Ok(Ok(suggestion)) => {
                        // Show the AI suggestion in error modal (better for longer text)
                        self.error_modal = Some(ErrorModal {
                            title: "AI Analysis".to_string(),
                            message: suggestion,
                        });
                    }
                    Ok(Err(e)) => {
                        self.notify_error(format!("AI analysis failed: {}", e));
                    }
                    Err(_) => {
                        self.notify_error("AI analysis thread panicked");
                    }
                }
                return false;
            }

            // Keep polling
            self.background_op = Some(BackgroundOp::AnalyzeError { log_path });
            true
        } else {
            // Start the analysis thread
            self.loading_progress = LoadingProgress {
                current_step: 1,
                total_steps: 1,
                step_name: "Starting AI analysis...".to_string(),
                found_count: 0,
            };

            let log_path_clone = log_path.clone();
            let thread_handle = std::thread::spawn(move || {
                // Read the log file (last 200 lines to avoid token overflow)
                let log_content = std::fs::read_to_string(&log_path_clone)
                    .map_err(|e| format!("Failed to read log: {}", e))?;

                let lines: Vec<&str> = log_content.lines().collect();
                let truncated = if lines.len() > 200 {
                    format!(
                        "... ({} lines truncated)\n{}",
                        lines.len() - 200,
                        lines[lines.len() - 200..].join("\n")
                    )
                } else {
                    log_content.clone()
                };

                // Build prompt for AI
                let prompt = format!(
                    r#"Analyze this install error log and provide a concise troubleshooting suggestion.
Focus on:
1. The root cause of the failure
2. Specific steps to fix it (missing packages, permissions, etc.)
3. Alternative approaches if applicable

Keep your response under 300 words and be direct.

=== INSTALL LOG ===
{}
=== END LOG ==="#,
                    truncated
                );

                // Call AI
                crate::ai::invoke_ai(&prompt).map_err(|e| e.to_string())
            });

            self.ai_error_analysis = Some(AiErrorAnalysis {
                start_time: std::time::Instant::now(),
                thread_handle,
            });

            self.background_op = Some(BackgroundOp::AnalyzeError { log_path });
            true
        }
    }

    /// Execute one step of an install/update operation
    /// Uses spawned processes for cancellation support
    fn execute_install_step(
        &mut self,
        db: &Database,
        tasks: Vec<InstallTask>,
        current: usize,
        mut results: Vec<InstallResult>,
        is_update: bool,
    ) -> bool {
        use crate::commands::install::get_safe_install_command_with_url;
        use crate::models::{InstallSource, Tool};
        use std::process::Command;

        // Check if all tasks are done
        if current >= tasks.len() {
            self.install_operation = None;
            self.finalize_install(&results, db, is_update);
            return false; // Done
        }

        let task = &tasks[current];

        // Update progress with elapsed time if operation is running
        let elapsed_info = self
            .install_operation
            .as_ref()
            .map(|op| format!(" ({:.1}s)", op.start_time.elapsed().as_secs_f32()))
            .unwrap_or_default();

        self.loading_progress = LoadingProgress {
            current_step: current + 1,
            total_steps: tasks.len(),
            step_name: format!(
                "{} {}{}...",
                if is_update { "Updating" } else { "Installing" },
                task.name,
                elapsed_info
            ),
            found_count: results.iter().filter(|r| r.success).count(),
        };

        // Check if we have a running install operation
        if let Some(ref mut op) = self.install_operation {
            // Poll for new output lines (non-blocking)
            // Limit reads per tick to avoid blocking input processing
            use std::io::Write;
            use std::sync::mpsc::TryRecvError;
            const MAX_LINES_PER_TICK: usize = 20;
            let mut lines_read = 0;

            while lines_read < MAX_LINES_PER_TICK {
                match op.output_receiver.try_recv() {
                    Ok(line) => {
                        // Write to log file
                        if let Some(ref mut w) = op.log_writer {
                            let prefix = match line.line_type {
                                OutputLineType::Stdout => "",
                                OutputLineType::Stderr => "[stderr] ",
                                OutputLineType::Status => "[status] ",
                            };
                            let _ = writeln!(w, "{}{}", prefix, line.content);
                        }
                        self.install_output.push(line);
                        lines_read += 1;
                    }
                    Err(TryRecvError::Empty) => break,
                    Err(TryRecvError::Disconnected) => break,
                }
            }

            // Auto-scroll to bottom if user hasn't scrolled up
            if lines_read > 0 {
                let visible_lines = 10;
                // Only auto-scroll if we're near the bottom (within 2 lines)
                if self.install_output_scroll
                    >= self
                        .install_output
                        .len()
                        .saturating_sub(visible_lines + lines_read + 2)
                {
                    self.install_output_scroll =
                        self.install_output.len().saturating_sub(visible_lines);
                }
            }

            // Poll the child process (non-blocking)
            match op.child.try_wait() {
                Ok(Some(status)) => {
                    // Process finished - take ownership
                    let mut finished_op = self.install_operation.take().unwrap();

                    // Drain any remaining output from the channel
                    // (reader threads might still be sending after process exit)
                    while let Ok(line) = finished_op.output_receiver.try_recv() {
                        // Write to log file
                        if let Some(ref mut w) = finished_op.log_writer {
                            let prefix = match line.line_type {
                                OutputLineType::Stdout => "",
                                OutputLineType::Stderr => "[stderr] ",
                                OutputLineType::Status => "[status] ",
                            };
                            let _ = writeln!(w, "{}{}", prefix, line.content);
                        }
                        self.install_output.push(line);
                    }

                    // Write final status to log and flush
                    let log_path = finished_op.log_path.clone();
                    if let Some(ref mut w) = finished_op.log_writer {
                        let _ = writeln!(w, "\n=== Result ===");
                        let _ = writeln!(w, "Exit code: {:?}", status.code());
                        let _ = writeln!(w, "Success: {}", status.success());
                        let _ = w.flush();
                    }
                    drop(finished_op.log_writer); // Close the file

                    // Collect stderr lines - prioritize actual error lines
                    let all_stderr: Vec<&str> = self
                        .install_output
                        .iter()
                        .filter(|l| l.line_type == OutputLineType::Stderr)
                        .map(|l| l.content.as_str())
                        .collect();

                    // Find lines containing error indicators
                    let error_lines: Vec<&str> = all_stderr
                        .iter()
                        .copied()
                        .filter(|line| {
                            let lower = line.to_lowercase();
                            lower.contains("error")
                                || lower.contains("failed")
                                || lower.contains("cannot")
                                || lower.contains("could not")
                                || lower.contains("sigsegv")
                                || lower.contains("signal:")
                        })
                        .collect();

                    // Use error lines if found, otherwise use last N lines of stderr
                    let stderr_output = if !error_lines.is_empty() {
                        // Take last 10 error lines
                        let last_errors: Vec<&str> = error_lines
                            .into_iter()
                            .rev()
                            .take(10)
                            .collect::<Vec<_>>()
                            .into_iter()
                            .rev()
                            .collect();
                        last_errors.join("\n")
                    } else if !all_stderr.is_empty() {
                        // No explicit errors, take last 15 lines
                        let last_lines: Vec<&str> = all_stderr
                            .into_iter()
                            .rev()
                            .take(15)
                            .collect::<Vec<_>>()
                            .into_iter()
                            .rev()
                            .collect();
                        last_lines.join("\n")
                    } else {
                        String::new()
                    };

                    let stderr_output = if stderr_output.is_empty() {
                        None
                    } else if stderr_output.len() > 1000 {
                        // Truncate from beginning if still too long
                        Some(format!(
                            "...{}",
                            &stderr_output[stderr_output.len() - 1000..]
                        ))
                    } else {
                        Some(stderr_output)
                    };

                    // Handle result
                    let result = if status.success() {
                        // Track database sync warnings separately
                        let mut db_warning: Option<String> = None;

                        // Upsert tool to database - handles both new tools and existing ones
                        // First check if tool exists to preserve existing data
                        let existing = db.get_tool_by_name(&task.name).ok().flatten();
                        let tool = if let Some(mut t) = existing {
                            // Update existing tool
                            t.is_installed = true;
                            // Add description from Discover if not already set
                            if t.description.is_none() {
                                t.description = task.description.clone();
                            }
                            t
                        } else {
                            // Create new tool (for discover installs)
                            let mut new_tool = Tool::new(&task.name)
                                .with_source(InstallSource::from(task.source.as_str()))
                                .installed();
                            // Add description from Discover metadata
                            if let Some(ref desc) = task.description {
                                new_tool = new_tool.with_description(desc);
                            }
                            new_tool
                        };

                        if let Err(e) = db.upsert_tool(&tool) {
                            db_warning = Some(format!("DB sync failed: {:#}", e));
                        }

                        // Sync GitHub info if we have stars and a GitHub URL
                        if let (Some(stars), Some(url)) = (task.stars, &task.url)
                            && let Ok((owner, repo)) = crate::ai::parse_github_url(url)
                        {
                            let gh_info = crate::db::GitHubInfoInput {
                                repo_owner: &owner,
                                repo_name: &repo,
                                description: task.description.as_deref(),
                                stars: stars as i64,
                                language: None,
                                homepage: None,
                            };
                            if let Err(e) = db.set_github_info(&task.name, gh_info) {
                                // Append to warning if exists, or create new
                                let msg = format!("GitHub sync failed: {:#}", e);
                                db_warning = Some(match db_warning {
                                    Some(w) => format!("{}; {}", w, msg),
                                    None => msg,
                                });
                            }
                        }

                        InstallResult {
                            name: task.name.clone(),
                            success: true,
                            error: db_warning,
                        }
                    } else {
                        // Store log path for AI analysis
                        self.last_install_log = Some(log_path.clone());

                        // Build error message with stderr output and log path
                        let exit_code = status
                            .code()
                            .map_or("unknown".to_string(), |c| c.to_string());
                        let error_msg = match stderr_output {
                            Some(stderr) => format!(
                                "Exit code {}: {}\n\nFull log: {}",
                                exit_code,
                                stderr,
                                log_path.display()
                            ),
                            None => format!(
                                "Command failed with exit code: {}\n\nFull log: {}",
                                exit_code,
                                log_path.display()
                            ),
                        };

                        InstallResult {
                            name: task.name.clone(),
                            success: false,
                            error: Some(error_msg),
                        }
                    };

                    // Add status line to output
                    self.install_output.push(OutputLine {
                        line_type: if result.success {
                            OutputLineType::Status
                        } else {
                            OutputLineType::Stderr
                        },
                        content: if result.success {
                            format!("✓ {} completed", result.name)
                        } else {
                            format!("✗ {} failed", result.name)
                        },
                    });

                    results.push(result);

                    // Schedule next task
                    let next = current + 1;
                    if is_update {
                        self.background_op = Some(BackgroundOp::ExecuteUpdate {
                            tasks,
                            current: next,
                            results,
                        });
                    } else {
                        self.background_op = Some(BackgroundOp::ExecuteInstall {
                            tasks,
                            current: next,
                            results,
                        });
                    }
                    return true;
                }
                Ok(None) => {
                    // Still running - keep polling
                    if is_update {
                        self.background_op = Some(BackgroundOp::ExecuteUpdate {
                            tasks,
                            current,
                            results,
                        });
                    } else {
                        self.background_op = Some(BackgroundOp::ExecuteInstall {
                            tasks,
                            current,
                            results,
                        });
                    }
                    return true;
                }
                Err(e) => {
                    // Error polling - treat as failure
                    results.push(InstallResult {
                        name: task.name.clone(),
                        success: false,
                        error: Some(format!("Failed to poll process: {:#}", e)),
                    });
                    self.install_operation = None;

                    // Schedule next task
                    let next = current + 1;
                    if is_update {
                        self.background_op = Some(BackgroundOp::ExecuteUpdate {
                            tasks,
                            current: next,
                            results,
                        });
                    } else {
                        self.background_op = Some(BackgroundOp::ExecuteInstall {
                            tasks,
                            current: next,
                            results,
                        });
                    }
                    return true;
                }
            }
        }

        // No running operation - start a new one
        // Clear output buffer for new task
        self.install_output.clear();
        self.install_output_scroll = 0;

        // Add status message for starting
        self.install_output.push(OutputLine {
            line_type: OutputLineType::Status,
            content: format!("$ {}", task.display_command),
        });

        let result = match get_safe_install_command_with_url(
            &task.name,
            &task.source,
            task.version.as_deref(),
            task.url.as_deref(),
        ) {
            Ok(Some(cmd)) => {
                use std::io::BufRead;
                use std::io::Write;
                use std::sync::mpsc;

                // Create log file for full output
                let timestamp = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_secs())
                    .unwrap_or(0);
                let log_path = std::path::PathBuf::from(format!(
                    "/tmp/hoards-install-{}-{}.log",
                    task.name, timestamp
                ));
                let mut log_writer = std::fs::File::create(&log_path)
                    .ok()
                    .map(std::io::BufWriter::new);

                // Write header with context info to log
                if let Some(ref mut w) = log_writer {
                    let _ = writeln!(w, "=== Hoards Install Log ===");
                    let _ = writeln!(w, "Tool: {}", task.name);
                    let _ = writeln!(w, "Source: {}", task.source);
                    if let Some(ref v) = task.version {
                        let _ = writeln!(w, "Version: {}", v);
                    }
                    let _ = writeln!(w, "Command: {}", task.display_command);
                    let _ = writeln!(
                        w,
                        "Timestamp: {}",
                        chrono::Local::now().format("%Y-%m-%d %H:%M:%S")
                    );
                    // OS info
                    if let Ok(os) = std::fs::read_to_string("/etc/os-release")
                        && let Some(pretty) = os.lines().find(|l| l.starts_with("PRETTY_NAME="))
                    {
                        let _ = writeln!(
                            w,
                            "OS: {}",
                            pretty.trim_start_matches("PRETTY_NAME=").trim_matches('"')
                        );
                    }
                    // Installer version
                    let version_cmd = match task.source.as_str() {
                        "cargo" => Some(("cargo", vec!["--version"])),
                        "pip" | "pipx" => Some(("pip", vec!["--version"])),
                        "npm" => Some(("npm", vec!["--version"])),
                        "brew" => Some(("brew", vec!["--version"])),
                        "apt" => Some(("apt", vec!["--version"])),
                        _ => None,
                    };
                    if let Some((prog, args)) = version_cmd
                        && let Ok(out) = Command::new(prog).args(&args).output()
                    {
                        let ver = String::from_utf8_lossy(&out.stdout);
                        let _ =
                            writeln!(w, "Installer: {}", ver.lines().next().unwrap_or("unknown"));
                    }
                    let _ = writeln!(w, "===========================\n");
                    let _ = w.flush();
                }

                // Spawn the command with both stdout and stderr piped
                match Command::new(cmd.program)
                    .args(&cmd.args)
                    .stdout(std::process::Stdio::piped())
                    .stderr(std::process::Stdio::piped())
                    .spawn()
                {
                    Ok(mut child) => {
                        // Create channel for output lines
                        let (tx, rx) = mpsc::channel::<OutputLine>();

                        // Spawn thread to read stdout
                        if let Some(stdout) = child.stdout.take() {
                            let tx_stdout = tx.clone();
                            std::thread::spawn(move || {
                                let reader = std::io::BufReader::new(stdout);
                                for line in reader.lines().map_while(Result::ok) {
                                    let _ = tx_stdout.send(OutputLine {
                                        line_type: OutputLineType::Stdout,
                                        content: line,
                                    });
                                }
                            });
                        }

                        // Spawn thread to read stderr
                        if let Some(stderr) = child.stderr.take() {
                            let tx_stderr = tx;
                            std::thread::spawn(move || {
                                let reader = std::io::BufReader::new(stderr);
                                for line in reader.lines().map_while(Result::ok) {
                                    let _ = tx_stderr.send(OutputLine {
                                        line_type: OutputLineType::Stderr,
                                        content: line,
                                    });
                                }
                            });
                        }

                        self.install_operation = Some(InstallOperation {
                            task_name: task.name.clone(),
                            child,
                            start_time: std::time::Instant::now(),
                            output_receiver: rx,
                            log_path,
                            log_writer,
                        });

                        // Keep polling this task
                        if is_update {
                            self.background_op = Some(BackgroundOp::ExecuteUpdate {
                                tasks,
                                current,
                                results,
                            });
                        } else {
                            self.background_op = Some(BackgroundOp::ExecuteInstall {
                                tasks,
                                current,
                                results,
                            });
                        }
                        return true;
                    }
                    Err(e) => InstallResult {
                        name: task.name.clone(),
                        success: false,
                        error: Some(format!("Failed to spawn command: {:#}", e)),
                    },
                }
            }
            Ok(None) => InstallResult {
                name: task.name.clone(),
                success: false,
                error: Some(format!("Unknown install source: {}", task.source)),
            },
            Err(e) => InstallResult {
                name: task.name.clone(),
                success: false,
                error: Some(format!("Invalid package name: {:#}", e)),
            },
        };

        // Command failed to start - add result and continue to next
        results.push(result);
        let next = current + 1;
        if is_update {
            self.background_op = Some(BackgroundOp::ExecuteUpdate {
                tasks,
                current: next,
                results,
            });
        } else {
            self.background_op = Some(BackgroundOp::ExecuteInstall {
                tasks,
                current: next,
                results,
            });
        }
        true
    }

    /// Cancel any running install operation
    pub fn cancel_install(&mut self) {
        if let Some(mut op) = self.install_operation.take() {
            let _ = op.child.kill();
            self.set_status(format!("Cancelled install of {}", op.task_name), true);
        }
        self.background_op = None;
    }

    /// Check if any task in the list requires sudo
    pub fn tasks_need_sudo(tasks: &[InstallTask]) -> bool {
        tasks
            .iter()
            .any(|t| t.source == "apt" || t.source == "snap")
    }

    /// Prompt for sudo password before starting install
    pub fn prompt_sudo_password(&mut self, tasks: Vec<InstallTask>, is_update: bool) {
        self.pending_sudo_tasks = Some((tasks, is_update));
        self.password_input.clear();
        self.input_mode = InputMode::Password;
        self.set_status("Enter sudo password to continue", false);
    }

    /// Start install with sudo password (for apt/snap)
    pub fn start_sudo_install(
        &mut self,
        tasks: Vec<InstallTask>,
        is_update: bool,
        password: String,
        db: &Database,
    ) {
        use crate::commands::install::get_safe_install_command_with_url;
        use crate::models::{InstallSource, Tool};
        use std::io::Write;
        use std::process::{Command, Stdio};

        let mut results = Vec::new();

        for task in &tasks {
            // Update progress
            self.loading_progress = LoadingProgress {
                current_step: results.len() + 1,
                total_steps: tasks.len(),
                step_name: format!(
                    "{} {}...",
                    if is_update { "Updating" } else { "Installing" },
                    task.name
                ),
                found_count: results
                    .iter()
                    .filter(|r: &&InstallResult| r.success)
                    .count(),
            };

            let result = match get_safe_install_command_with_url(
                &task.name,
                &task.source,
                task.version.as_deref(),
                task.url.as_deref(),
            ) {
                Ok(Some(cmd)) => {
                    // For sudo commands, use sudo -S to read password from stdin
                    let child = Command::new("sudo")
                        .arg("-S") // Read password from stdin
                        .arg("--")
                        .arg(cmd.program)
                        .args(&cmd.args)
                        .stdin(Stdio::piped())
                        .stdout(Stdio::null())
                        .stderr(Stdio::piped())
                        .spawn();

                    match child {
                        Ok(mut child) => {
                            // Write password to stdin
                            if let Some(mut stdin) = child.stdin.take() {
                                let _ = writeln!(stdin, "{}", password);
                            }

                            // Wait for completion
                            match child.wait_with_output() {
                                Ok(output) => {
                                    if output.status.success() {
                                        // Update database
                                        let mut db_warning: Option<String> = None;
                                        let existing =
                                            db.get_tool_by_name(&task.name).ok().flatten();
                                        let tool = if let Some(mut t) = existing {
                                            t.is_installed = true;
                                            // Add description from Discover if not already set
                                            if t.description.is_none() {
                                                t.description = task.description.clone();
                                            }
                                            t
                                        } else {
                                            let mut new_tool = Tool::new(&task.name)
                                                .with_source(InstallSource::from(
                                                    task.source.as_str(),
                                                ))
                                                .installed();
                                            // Add description from Discover metadata
                                            if let Some(ref desc) = task.description {
                                                new_tool = new_tool.with_description(desc);
                                            }
                                            new_tool
                                        };
                                        if let Err(e) = db.upsert_tool(&tool) {
                                            db_warning = Some(format!("DB sync failed: {:#}", e));
                                        }

                                        // Sync GitHub info if we have stars and a GitHub URL
                                        if let (Some(stars), Some(url)) = (task.stars, &task.url)
                                            && let Ok((owner, repo)) =
                                                crate::ai::parse_github_url(url)
                                        {
                                            let gh_info = crate::db::GitHubInfoInput {
                                                repo_owner: &owner,
                                                repo_name: &repo,
                                                description: task.description.as_deref(),
                                                stars: stars as i64,
                                                language: None,
                                                homepage: None,
                                            };
                                            if let Err(e) = db.set_github_info(&task.name, gh_info)
                                            {
                                                let msg = format!("GitHub sync failed: {:#}", e);
                                                db_warning = Some(match db_warning {
                                                    Some(w) => format!("{}; {}", w, msg),
                                                    None => msg,
                                                });
                                            }
                                        }

                                        InstallResult {
                                            name: task.name.clone(),
                                            success: true,
                                            error: db_warning,
                                        }
                                    } else {
                                        let stderr = String::from_utf8_lossy(&output.stderr);
                                        let stderr_trimmed = stderr.trim();
                                        // Filter out sudo password prompt from error
                                        let filtered_stderr: String = stderr_trimmed
                                            .lines()
                                            .filter(|line| {
                                                !line.contains("[sudo]")
                                                    && !line.contains("password for")
                                            })
                                            .collect::<Vec<_>>()
                                            .join("\n");
                                        let error_msg = if filtered_stderr.is_empty() {
                                            format!(
                                                "Exit code: {}",
                                                output.status.code().unwrap_or(-1)
                                            )
                                        } else if filtered_stderr.len() > 500 {
                                            format!("{}...", &filtered_stderr[..500])
                                        } else {
                                            filtered_stderr
                                        };
                                        InstallResult {
                                            name: task.name.clone(),
                                            success: false,
                                            error: Some(error_msg),
                                        }
                                    }
                                }
                                Err(e) => InstallResult {
                                    name: task.name.clone(),
                                    success: false,
                                    error: Some(format!("Failed to wait for command: {:#}", e)),
                                },
                            }
                        }
                        Err(e) => InstallResult {
                            name: task.name.clone(),
                            success: false,
                            error: Some(format!("Failed to spawn command: {:#}", e)),
                        },
                    }
                }
                Ok(None) => InstallResult {
                    name: task.name.clone(),
                    success: false,
                    error: Some(format!("Unknown install source: {}", task.source)),
                },
                Err(e) => InstallResult {
                    name: task.name.clone(),
                    success: false,
                    error: Some(format!("Invalid package name: {:#}", e)),
                },
            };

            results.push(result);
        }

        self.finalize_install(&results, db, is_update);
    }

    /// Finalize install/update operation and show results
    fn finalize_install(&mut self, results: &[InstallResult], db: &Database, is_update: bool) {
        // Scroll output to bottom so user sees the final result
        let visible_lines = 10; // Approximate visible lines
        self.install_output_scroll = self.install_output.len().saturating_sub(visible_lines);

        let action = if is_update { "update" } else { "install" };
        let success_count = results.iter().filter(|r| r.success).count();
        let fail_count = results.len() - success_count;

        // Check for DB sync warnings on successful installs
        let db_warnings: Vec<_> = results
            .iter()
            .filter(|r| r.success && r.error.is_some())
            .collect();

        if fail_count == 0 {
            if results.len() == 1 {
                let mut msg = format!("Successfully {}ed {}", action, results[0].name);
                if let Some(warning) = &results[0].error {
                    msg = format!("{} (warning: {})", msg, warning);
                }
                self.set_status(msg, !db_warnings.is_empty());
            } else {
                let mut msg = format!("Successfully {}ed {} tool(s)", action, success_count);
                if !db_warnings.is_empty() {
                    msg = format!("{} ({} DB sync warning(s))", msg, db_warnings.len());
                }
                self.set_status(msg, !db_warnings.is_empty());
            }
        } else if success_count == 0 {
            // All failed - show error modal with details
            let failed_details: Vec<String> = results
                .iter()
                .filter(|r| !r.success)
                .map(|r| {
                    let err = r.error.as_deref().unwrap_or("Unknown error");
                    format!("• {}: {}", r.name, err)
                })
                .collect();

            let message = if results.len() == 1 {
                failed_details.first().cloned().unwrap_or_default()
            } else {
                failed_details.join("\n")
            };

            self.show_error_modal(format!("Failed to {}", action), message);
            self.set_status(format!("Failed to {} {} tool(s)", action, fail_count), true);
        } else {
            // Partial success - show error modal for failures
            let failed_details: Vec<String> = results
                .iter()
                .filter(|r| !r.success)
                .map(|r| {
                    let err = r.error.as_deref().unwrap_or("Unknown error");
                    format!("• {}: {}", r.name, err)
                })
                .collect();

            self.show_error_modal(
                format!("Partial {} failure", action),
                format!(
                    "Succeeded: {}\nFailed: {}\n\n{}",
                    success_count,
                    fail_count,
                    failed_details.join("\n")
                ),
            );
            self.set_status(
                format!(
                    "{}ed {}, {} failed",
                    if is_update { "Updated" } else { "Installed" },
                    success_count,
                    fail_count,
                ),
                true,
            );
        }

        // Only clear successfully installed tools from selection (keep failed ones for retry)
        for result in results.iter().filter(|r| r.success) {
            self.selected_tools.remove(&result.name);
        }

        self.refresh_tools(db);
    }
}
