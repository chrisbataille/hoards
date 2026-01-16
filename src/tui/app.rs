//! Application state for the TUI

use std::collections::{HashMap, HashSet};

use anyhow::Result;

use crate::Update;
use crate::db::{Database, GitHubInfo, ToolUsage};
use crate::models::{Bundle, Tool};

/// Available tabs in the TUI
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Tab {
    #[default]
    Installed,
    Available,
    Updates,
    Bundles,
}

impl Tab {
    pub fn all() -> &'static [Tab] {
        &[Tab::Installed, Tab::Available, Tab::Updates, Tab::Bundles]
    }

    pub fn title(&self) -> &'static str {
        match self {
            Tab::Installed => "Installed",
            Tab::Available => "Available",
            Tab::Updates => "Updates",
            Tab::Bundles => "Bundles",
        }
    }

    pub fn index(&self) -> usize {
        match self {
            Tab::Installed => 0,
            Tab::Available => 1,
            Tab::Updates => 2,
            Tab::Bundles => 3,
        }
    }

    pub fn from_index(index: usize) -> Option<Tab> {
        match index {
            0 => Some(Tab::Installed),
            1 => Some(Tab::Available),
            2 => Some(Tab::Updates),
            3 => Some(Tab::Bundles),
            _ => None,
        }
    }
}

/// Input mode for the application
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum InputMode {
    #[default]
    Normal,
    Search,
}

/// Background operation that needs loading indicator
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BackgroundOp {
    CheckUpdates { step: usize },
}

impl BackgroundOp {
    pub fn title(&self) -> &'static str {
        match self {
            BackgroundOp::CheckUpdates { .. } => "Checking for Updates",
        }
    }
}

/// Progress information for loading overlay
#[derive(Debug, Clone, Default)]
pub struct LoadingProgress {
    pub current_step: usize,
    pub total_steps: usize,
    pub step_name: String,
    pub found_count: usize,
}

/// Package manager info for update checking
const PACKAGE_MANAGERS: &[(&str, &str)] = &[
    ("cargo", "Cargo (Rust)"),
    ("pip", "pip (Python)"),
    ("npm", "npm (Node.js)"),
    ("apt", "apt (Debian/Ubuntu)"),
    ("brew", "Homebrew"),
];

/// Pending action requiring confirmation
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PendingAction {
    Install(Vec<String>),   // Tool names to install
    Uninstall(Vec<String>), // Tool names to uninstall
    Update(Vec<String>),    // Tool names to update
}

impl PendingAction {
    pub fn description(&self) -> String {
        match self {
            PendingAction::Install(tools) => {
                if tools.len() == 1 {
                    format!("Install {}?", tools[0])
                } else {
                    format!("Install {} tools?", tools.len())
                }
            }
            PendingAction::Uninstall(tools) => {
                if tools.len() == 1 {
                    format!("Uninstall {}?", tools[0])
                } else {
                    format!("Uninstall {} tools?", tools.len())
                }
            }
            PendingAction::Update(tools) => {
                if tools.len() == 1 {
                    format!("Update {}?", tools[0])
                } else {
                    format!("Update {} tools?", tools.len())
                }
            }
        }
    }

    pub fn tools(&self) -> &[String] {
        match self {
            PendingAction::Install(t) | PendingAction::Uninstall(t) | PendingAction::Update(t) => t,
        }
    }
}

/// Status message to display temporarily
#[derive(Debug, Clone)]
pub struct StatusMessage {
    pub text: String,
    pub is_error: bool,
}

/// Sort options for tool list
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SortBy {
    #[default]
    Name,
    Usage,
    Recent,
}

impl SortBy {
    pub fn next(&self) -> SortBy {
        match self {
            SortBy::Name => SortBy::Usage,
            SortBy::Usage => SortBy::Recent,
            SortBy::Recent => SortBy::Name,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            SortBy::Name => "name",
            SortBy::Usage => "usage",
            SortBy::Recent => "recent",
        }
    }
}

/// Main application state
pub struct App {
    pub running: bool,
    pub tab: Tab,
    pub input_mode: InputMode,
    pub search_query: String,

    // Tool list state
    pub all_tools: Vec<Tool>, // All tools for current tab (unfiltered)
    pub tools: Vec<Tool>,     // Filtered/sorted tools to display
    pub selected_index: usize,
    pub list_offset: usize,

    // Cached data
    pub usage_data: HashMap<String, ToolUsage>,
    pub daily_usage: HashMap<String, Vec<i64>>, // 7-day usage for sparklines
    pub github_cache: HashMap<String, GitHubInfo>,

    // Updates state
    pub available_updates: HashMap<String, Update>,
    pub updates_checked: bool,
    pub updates_loading: bool,

    // Bundle list state (for Bundles tab)
    pub bundles: Vec<Bundle>,
    pub bundle_selected: usize,

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
}

impl App {
    pub fn new(db: &Database) -> Result<Self> {
        let all_tools = db.list_tools(true, None)?; // installed only
        let bundles = db.list_bundles()?;

        // Load usage data
        let usage_data: HashMap<String, ToolUsage> = db.get_all_usage()?.into_iter().collect();

        // Load 7-day daily usage for sparklines
        let daily_usage = db.get_all_daily_usage(7).unwrap_or_default();

        // Preload GitHub info for stars display
        let github_cache: HashMap<String, GitHubInfo> = db
            .get_all_github_info()
            .unwrap_or_default()
            .into_iter()
            .collect();

        let tools = all_tools.clone();

        Ok(Self {
            running: true,
            tab: Tab::Installed,
            input_mode: InputMode::Normal,
            search_query: String::new(),
            all_tools,
            tools,
            selected_index: 0,
            list_offset: 0,
            usage_data,
            daily_usage,
            github_cache,
            available_updates: HashMap::new(),
            updates_checked: false,
            updates_loading: false,
            bundles,
            bundle_selected: 0,
            show_help: false,
            show_details_popup: false,
            sort_by: SortBy::default(),
            theme_variant: super::theme::ThemeVariant::default(),
            selected_tools: HashSet::new(),
            pending_action: None,
            status_message: None,
            background_op: None,
            loading_progress: LoadingProgress::default(),
        })
    }

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
        if self.tab == Tab::Bundles
            && let Ok(bundles) = db.list_bundles()
        {
            self.bundles = bundles;
        }
    }

    /// Get update info for a tool if available
    pub fn get_update(&self, tool_name: &str) -> Option<&Update> {
        self.available_updates.get(tool_name)
    }

    /// Apply current search filter and sort to tools
    pub fn apply_filter_and_sort(&mut self) {
        // Start with all tools
        let mut filtered: Vec<Tool> = if self.search_query.is_empty() {
            self.all_tools.clone()
        } else {
            let query = self.search_query.to_lowercase();
            self.all_tools
                .iter()
                .filter(|t| {
                    t.name.to_lowercase().contains(&query)
                        || t.description
                            .as_ref()
                            .is_some_and(|d| d.to_lowercase().contains(&query))
                        || t.category
                            .as_ref()
                            .is_some_and(|c| c.to_lowercase().contains(&query))
                })
                .cloned()
                .collect()
        };

        // Sort
        match self.sort_by {
            SortBy::Name => filtered.sort_by(|a, b| a.name.cmp(&b.name)),
            SortBy::Usage => {
                let usage = &self.usage_data;
                filtered.sort_by(|a, b| {
                    let a_usage = usage.get(&a.name).map(|u| u.use_count).unwrap_or(0);
                    let b_usage = usage.get(&b.name).map(|u| u.use_count).unwrap_or(0);
                    b_usage.cmp(&a_usage) // Descending
                });
            }
            SortBy::Recent => {
                filtered.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
            }
        }

        self.tools = filtered;

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

    // ==================== Bundle Navigation ====================

    /// Move bundle selection down
    pub fn select_next_bundle(&mut self) {
        if !self.bundles.is_empty() {
            self.bundle_selected = (self.bundle_selected + 1).min(self.bundles.len() - 1);
        }
    }

    /// Move bundle selection up
    pub fn select_prev_bundle(&mut self) {
        self.bundle_selected = self.bundle_selected.saturating_sub(1);
    }

    /// Move bundle selection to top
    pub fn select_first_bundle(&mut self) {
        self.bundle_selected = 0;
    }

    /// Move bundle selection to bottom
    pub fn select_last_bundle(&mut self) {
        if !self.bundles.is_empty() {
            self.bundle_selected = self.bundles.len() - 1;
        }
    }

    /// Get the currently selected bundle
    pub fn selected_bundle(&self) -> Option<&Bundle> {
        self.bundles.get(self.bundle_selected)
    }

    /// Get the currently selected tool
    pub fn selected_tool(&self) -> Option<&Tool> {
        self.tools.get(self.selected_index)
    }

    /// Get usage for a tool
    pub fn get_usage(&self, tool_name: &str) -> Option<&ToolUsage> {
        self.usage_data.get(tool_name)
    }

    /// Get GitHub info for a tool (cached, or fetch from db)
    pub fn get_github_info(&mut self, tool_name: &str, db: &Database) -> Option<&GitHubInfo> {
        if !self.github_cache.contains_key(tool_name)
            && let Ok(Some(info)) = db.get_github_info(tool_name)
        {
            self.github_cache.insert(tool_name.to_string(), info);
        }
        self.github_cache.get(tool_name)
    }

    /// Toggle help overlay
    pub fn toggle_help(&mut self) {
        self.show_help = !self.show_help;
    }

    /// Enter search mode
    pub fn enter_search(&mut self) {
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
        self.search_query.clear();
        self.apply_filter_and_sort();
    }

    // ==================== Selection ====================

    /// Toggle selection of current tool
    pub fn toggle_selection(&mut self) {
        if let Some(tool) = self.selected_tool() {
            let name = tool.name.clone();
            if self.selected_tools.contains(&name) {
                self.selected_tools.remove(&name);
            } else {
                self.selected_tools.insert(name);
            }
        }
    }

    /// Check if a tool is selected
    pub fn is_selected(&self, tool_name: &str) -> bool {
        self.selected_tools.contains(tool_name)
    }

    /// Clear all selections
    pub fn clear_selection(&mut self) {
        self.selected_tools.clear();
    }

    /// Select all visible tools
    pub fn select_all(&mut self) {
        for tool in &self.tools {
            self.selected_tools.insert(tool.name.clone());
        }
    }

    /// Get count of selected tools
    pub fn selection_count(&self) -> usize {
        self.selected_tools.len()
    }

    /// Get names of selected tools
    pub fn get_selected_tools(&self) -> Vec<String> {
        self.selected_tools.iter().cloned().collect()
    }

    // ==================== Details Popup ====================

    /// Toggle details popup (for narrow terminals)
    pub fn toggle_details_popup(&mut self) {
        self.show_details_popup = !self.show_details_popup;
    }

    /// Close details popup
    pub fn close_details_popup(&mut self) {
        self.show_details_popup = false;
    }

    // ==================== Actions ====================

    /// Request install action for selected tools (or current tool if none selected)
    pub fn request_install(&mut self) {
        let tools = if self.selected_tools.is_empty() {
            // Use current tool if nothing selected
            self.selected_tool()
                .filter(|t| !t.is_installed)
                .map(|t| vec![t.name.clone()])
                .unwrap_or_default()
        } else {
            // Use selected tools that aren't installed
            self.selected_tools
                .iter()
                .filter(|name| {
                    self.tools
                        .iter()
                        .any(|t| &t.name == *name && !t.is_installed)
                })
                .cloned()
                .collect()
        };

        if !tools.is_empty() {
            self.pending_action = Some(PendingAction::Install(tools));
        }
    }

    /// Request uninstall action for selected tools (or current tool if none selected)
    pub fn request_uninstall(&mut self) {
        let tools = if self.selected_tools.is_empty() {
            // Use current tool if nothing selected
            self.selected_tool()
                .filter(|t| t.is_installed)
                .map(|t| vec![t.name.clone()])
                .unwrap_or_default()
        } else {
            // Use selected tools that are installed
            self.selected_tools
                .iter()
                .filter(|name| {
                    self.tools
                        .iter()
                        .any(|t| &t.name == *name && t.is_installed)
                })
                .cloned()
                .collect()
        };

        if !tools.is_empty() {
            self.pending_action = Some(PendingAction::Uninstall(tools));
        }
    }

    /// Request update action for selected tools (or current tool if none selected)
    pub fn request_update(&mut self) {
        let tools = if self.selected_tools.is_empty() {
            // Use current tool if it has an update
            self.selected_tool()
                .filter(|t| self.available_updates.contains_key(&t.name))
                .map(|t| vec![t.name.clone()])
                .unwrap_or_default()
        } else {
            // Use selected tools that have updates
            self.selected_tools
                .iter()
                .filter(|name| self.available_updates.contains_key(*name))
                .cloned()
                .collect()
        };

        if !tools.is_empty() {
            self.pending_action = Some(PendingAction::Update(tools));
        }
    }

    /// Request install for missing tools in selected bundle
    pub fn request_bundle_install(&mut self, db: &Database) {
        let Some(bundle) = self.selected_bundle() else {
            return;
        };

        // Find tools that aren't installed
        let missing_tools: Vec<String> = bundle
            .tools
            .iter()
            .filter(|name| {
                !db.get_tool_by_name(name)
                    .ok()
                    .flatten()
                    .map(|t| t.is_installed)
                    .unwrap_or(false)
            })
            .cloned()
            .collect();

        if !missing_tools.is_empty() {
            self.pending_action = Some(PendingAction::Install(missing_tools));
        } else {
            self.set_status("All tools in bundle are already installed", false);
        }
    }

    /// Confirm and return the pending action
    pub fn confirm_action(&mut self) -> Option<PendingAction> {
        self.pending_action.take()
    }

    /// Cancel the pending action
    pub fn cancel_action(&mut self) {
        self.pending_action = None;
    }

    /// Check if there's a pending action
    pub fn has_pending_action(&self) -> bool {
        self.pending_action.is_some()
    }

    /// Set a status message
    pub fn set_status(&mut self, text: impl Into<String>, is_error: bool) {
        self.status_message = Some(StatusMessage {
            text: text.into(),
            is_error,
        });
    }

    /// Clear status message
    pub fn clear_status(&mut self) {
        self.status_message = None;
    }

    // ==================== Background Operations ====================

    /// Schedule a background operation (will be executed by main loop)
    pub fn schedule_op(&mut self, op: BackgroundOp) {
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
        }
    }
}
