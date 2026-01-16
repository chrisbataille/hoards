//! Application state for the TUI

use std::collections::{HashMap, HashSet};

use anyhow::Result;

use crate::Update;
use crate::db::{Database, GitHubInfo, ToolUsage};
use crate::models::{Bundle, Tool};

/// Fuzzy match a query against a target string (fzf-style)
/// Returns Some(score) if matches, None if no match
/// Higher scores = better matches
fn fuzzy_match(query: &str, target: &str) -> Option<i32> {
    let query = query.to_lowercase();
    let target = target.to_lowercase();

    if query.is_empty() {
        return Some(0);
    }

    let query_chars: Vec<char> = query.chars().collect();
    let target_chars: Vec<char> = target.chars().collect();

    let mut query_idx = 0;
    let mut score = 0i32;
    let mut prev_match_idx: Option<usize> = None;
    let mut consecutive_bonus = 0i32;

    for (target_idx, &tc) in target_chars.iter().enumerate() {
        if query_idx < query_chars.len() && tc == query_chars[query_idx] {
            // Character matched
            score += 1;

            // Bonus for consecutive matches
            if let Some(prev) = prev_match_idx {
                if target_idx == prev + 1 {
                    consecutive_bonus += 2;
                    score += consecutive_bonus;
                } else {
                    consecutive_bonus = 0;
                }
            }

            // Bonus for matching at word boundaries
            if target_idx == 0
                || target_chars
                    .get(target_idx.wrapping_sub(1))
                    .map(|c| !c.is_alphanumeric())
                    .unwrap_or(true)
            {
                score += 3;
            }

            prev_match_idx = Some(target_idx);
            query_idx += 1;
        }
    }

    // All query characters must match
    if query_idx == query_chars.len() {
        // Bonus for exact match
        if query == target {
            score += 100;
        }
        // Bonus for prefix match
        else if target.starts_with(&query) {
            score += 50;
        }
        Some(score)
    } else {
        None
    }
}

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
    Command, // Vim-style command palette with ':'
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

/// Undoable action for history
#[derive(Debug, Clone)]
pub enum UndoableAction {
    /// Selection change (stores previous selection state)
    Selection(HashSet<String>),
    /// Filter/search change (stores previous query)
    Filter(String),
    /// Tab switch (stores previous tab)
    TabSwitch(Tab),
    /// Sort change (stores previous sort)
    Sort(SortBy),
}

/// Action history for undo/redo
#[derive(Debug, Default)]
pub struct ActionHistory {
    undo_stack: Vec<UndoableAction>,
    redo_stack: Vec<UndoableAction>,
    max_size: usize,
}

impl ActionHistory {
    pub fn new(max_size: usize) -> Self {
        Self {
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            max_size,
        }
    }

    /// Push an action to the undo stack
    pub fn push(&mut self, action: UndoableAction) {
        if self.undo_stack.len() >= self.max_size {
            self.undo_stack.remove(0);
        }
        self.undo_stack.push(action);
        self.redo_stack.clear(); // Clear redo on new action
    }

    /// Pop an action for undo
    pub fn pop_undo(&mut self) -> Option<UndoableAction> {
        self.undo_stack.pop()
    }

    /// Push to redo stack
    pub fn push_redo(&mut self, action: UndoableAction) {
        if self.redo_stack.len() >= self.max_size {
            self.redo_stack.remove(0);
        }
        self.redo_stack.push(action);
    }

    /// Pop an action for redo
    pub fn pop_redo(&mut self) -> Option<UndoableAction> {
        self.redo_stack.pop()
    }

    /// Check if undo is available
    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    /// Check if redo is available
    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }
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
    pub command_input: String, // Command palette input (after ':')

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

    // Undo/redo history
    pub history: ActionHistory,

    // Mouse interaction state
    pub last_list_area: Option<(u16, u16, u16, u16)>, // (x, y, width, height) of tool list
    pub last_tab_area: Option<(u16, u16, u16, u16)>,  // (x, y, width, height) of tabs
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
            command_input: String::new(),
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
            history: ActionHistory::new(50), // Keep 50 actions max
            last_list_area: None,
            last_tab_area: None,
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
        let mut filtered: Vec<(Tool, i32)> = if self.search_query.is_empty() {
            self.all_tools.iter().map(|t| (t.clone(), 0)).collect()
        } else {
            // Fuzzy match against name, description, and category
            self.all_tools
                .iter()
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
                    let usage = &self.usage_data;
                    filtered.sort_by(|a, b| {
                        let a_usage = usage.get(&a.0.name).map(|u| u.use_count).unwrap_or(0);
                        let b_usage = usage.get(&b.0.name).map(|u| u.use_count).unwrap_or(0);
                        b_usage.cmp(&a_usage) // Descending
                    });
                }
                SortBy::Recent => {
                    filtered.sort_by(|a, b| b.0.updated_at.cmp(&a.0.updated_at));
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

    // ==================== Command Palette ====================

    /// Enter command mode (vim-style ':')
    pub fn enter_command(&mut self) {
        self.input_mode = InputMode::Command;
        self.command_input.clear();
    }

    /// Exit command mode
    pub fn exit_command(&mut self) {
        self.input_mode = InputMode::Normal;
        self.command_input.clear();
    }

    /// Add character to command input
    pub fn command_push(&mut self, c: char) {
        self.command_input.push(c);
    }

    /// Remove last character from command input
    pub fn command_pop(&mut self) {
        self.command_input.pop();
    }

    /// Execute the current command
    pub fn execute_command(&mut self, db: &Database) {
        let cmd = self.command_input.trim().to_lowercase();
        let parts: Vec<&str> = cmd.split_whitespace().collect();

        if parts.is_empty() {
            self.exit_command();
            return;
        }

        match parts[0] {
            // Quit commands
            "q" | "quit" | "exit" => self.quit(),

            // Help
            "h" | "help" => {
                self.show_help = true;
                self.exit_command();
            }

            // Refresh
            "r" | "refresh" => {
                self.refresh_tools(db);
                self.exit_command();
            }

            // Theme commands
            "theme" | "t" => {
                if parts.len() > 1 {
                    self.set_theme_by_name(parts[1]);
                } else {
                    self.cycle_theme();
                }
                self.exit_command();
            }

            // Sort commands
            "sort" | "s" => {
                if parts.len() > 1 {
                    self.set_sort_by_name(parts[1]);
                } else {
                    self.cycle_sort();
                }
                self.exit_command();
            }

            // Tab navigation
            "installed" | "1" => {
                self.switch_tab(Tab::Installed, db);
                self.exit_command();
            }
            "available" | "2" => {
                self.switch_tab(Tab::Available, db);
                self.exit_command();
            }
            "updates" | "3" => {
                self.switch_tab(Tab::Updates, db);
                self.exit_command();
            }
            "bundles" | "4" => {
                self.switch_tab(Tab::Bundles, db);
                self.exit_command();
            }

            // Install/Uninstall/Update
            "i" | "install" => {
                if self.tab == Tab::Bundles {
                    self.request_bundle_install(db);
                } else {
                    self.request_install();
                }
                self.exit_command();
            }
            "d" | "delete" | "uninstall" => {
                self.request_uninstall();
                self.exit_command();
            }
            "u" | "update" | "upgrade" => {
                self.request_update();
                self.exit_command();
            }

            // Undo/Redo
            "undo" | "z" => {
                self.undo();
                self.exit_command();
            }
            "redo" | "y" => {
                self.redo();
                self.exit_command();
            }

            // Unknown command
            _ => {
                self.set_status(format!("Unknown command: {}", parts[0]), true);
                self.exit_command();
            }
        }
    }

    /// Set theme by name
    fn set_theme_by_name(&mut self, name: &str) {
        use super::theme::ThemeVariant;
        self.theme_variant = match name {
            "mocha" | "catppuccin" | "catppuccin-mocha" => ThemeVariant::CatppuccinMocha,
            "latte" | "catppuccin-latte" => ThemeVariant::CatppuccinLatte,
            "dracula" => ThemeVariant::Dracula,
            "nord" => ThemeVariant::Nord,
            "tokyo" | "tokyo-night" | "tokyonight" => ThemeVariant::TokyoNight,
            "gruvbox" => ThemeVariant::Gruvbox,
            _ => {
                self.set_status(
                    "Themes: mocha, latte, dracula, nord, tokyo, gruvbox".to_string(),
                    true,
                );
                return;
            }
        };
        self.set_status(format!("Theme: {}", self.theme().name), false);
    }

    /// Set sort by name
    fn set_sort_by_name(&mut self, name: &str) {
        self.sort_by = match name {
            "name" | "n" | "alpha" => SortBy::Name,
            "usage" | "u" | "used" => SortBy::Usage,
            "recent" | "r" | "last" => SortBy::Recent,
            _ => {
                self.set_status("Sort: name, usage, recent".to_string(), true);
                return;
            }
        };
        self.apply_filter_and_sort();
        self.set_status(format!("Sort by: {:?}", self.sort_by), false);
    }

    /// Get available commands for autocomplete hints
    #[allow(dead_code)]
    pub fn get_command_suggestions(&self) -> Vec<(&'static str, &'static str)> {
        vec![
            ("q, quit", "Exit the application"),
            ("h, help", "Show help"),
            ("r, refresh", "Refresh tools list"),
            (
                "t, theme [name]",
                "Cycle or set theme (mocha, latte, dracula, nord, tokyo, gruvbox)",
            ),
            ("s, sort [field]", "Cycle or set sort (name, usage, recent)"),
            ("1-4", "Switch to tab by number"),
            ("i, install", "Install selected"),
            ("d, delete", "Uninstall selected"),
            ("u, update", "Update selected"),
            ("undo, z", "Undo last action"),
            ("redo, y", "Redo last undone action"),
        ]
    }

    // ==================== Selection ====================

    /// Toggle selection of current tool
    pub fn toggle_selection(&mut self) {
        // Get tool name first to avoid borrow checker issues
        let tool_name = self.selected_tool().map(|t| t.name.clone());
        if let Some(name) = tool_name {
            self.record_selection(); // Record for undo
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
        if !self.selected_tools.is_empty() {
            self.record_selection(); // Record for undo
            self.selected_tools.clear();
        }
    }

    /// Select all visible tools
    pub fn select_all(&mut self) {
        self.record_selection(); // Record for undo
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

    // ==================== Mouse Support ====================

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
        // row is relative to list area top
        let target_index = self.list_offset + row as usize;
        if target_index < self.tools.len() {
            self.selected_index = target_index;
        }
    }

    /// Handle mouse click on tab
    pub fn click_tab(&mut self, x: u16, db: &Database) {
        // Simple heuristic: divide tab area into 4 equal parts
        if let Some((area_x, _, width, _)) = self.last_tab_area {
            let relative_x = x.saturating_sub(area_x);
            let tab_width = width / 4;
            let tab_index = relative_x / tab_width.max(1);

            match tab_index {
                0 => self.switch_tab(Tab::Installed, db),
                1 => self.switch_tab(Tab::Available, db),
                2 => self.switch_tab(Tab::Updates, db),
                3 => self.switch_tab(Tab::Bundles, db),
                _ => {}
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

    // ==================== Undo/Redo ====================

    /// Undo the last action
    pub fn undo(&mut self) {
        if let Some(action) = self.history.pop_undo() {
            // Save current state for redo
            let redo_action = match &action {
                UndoableAction::Selection(_) => {
                    UndoableAction::Selection(self.selected_tools.clone())
                }
                UndoableAction::Filter(_) => UndoableAction::Filter(self.search_query.clone()),
                UndoableAction::TabSwitch(_) => UndoableAction::TabSwitch(self.tab),
                UndoableAction::Sort(_) => UndoableAction::Sort(self.sort_by),
            };
            self.history.push_redo(redo_action);

            // Restore previous state
            match action {
                UndoableAction::Selection(prev) => {
                    self.selected_tools = prev;
                    self.set_status("Selection restored".to_string(), false);
                }
                UndoableAction::Filter(prev) => {
                    self.search_query = prev;
                    self.apply_filter_and_sort();
                    self.set_status("Filter restored".to_string(), false);
                }
                UndoableAction::TabSwitch(prev) => {
                    self.tab = prev;
                    self.set_status(format!("Tab: {:?}", self.tab), false);
                }
                UndoableAction::Sort(prev) => {
                    self.sort_by = prev;
                    self.apply_filter_and_sort();
                    self.set_status(format!("Sort: {:?}", self.sort_by), false);
                }
            }
        } else {
            self.set_status("Nothing to undo".to_string(), true);
        }
    }

    /// Redo the last undone action
    pub fn redo(&mut self) {
        if let Some(action) = self.history.pop_redo() {
            // Save current state for undo
            let undo_action = match &action {
                UndoableAction::Selection(_) => {
                    UndoableAction::Selection(self.selected_tools.clone())
                }
                UndoableAction::Filter(_) => UndoableAction::Filter(self.search_query.clone()),
                UndoableAction::TabSwitch(_) => UndoableAction::TabSwitch(self.tab),
                UndoableAction::Sort(_) => UndoableAction::Sort(self.sort_by),
            };
            self.history.undo_stack.push(undo_action);

            // Apply the redo action
            match action {
                UndoableAction::Selection(new) => {
                    self.selected_tools = new;
                    self.set_status("Selection redone".to_string(), false);
                }
                UndoableAction::Filter(new) => {
                    self.search_query = new;
                    self.apply_filter_and_sort();
                    self.set_status("Filter redone".to_string(), false);
                }
                UndoableAction::TabSwitch(new) => {
                    self.tab = new;
                    self.set_status(format!("Tab: {:?}", self.tab), false);
                }
                UndoableAction::Sort(new) => {
                    self.sort_by = new;
                    self.apply_filter_and_sort();
                    self.set_status(format!("Sort: {:?}", self.sort_by), false);
                }
            }
        } else {
            self.set_status("Nothing to redo".to_string(), true);
        }
    }

    /// Record a selection change
    fn record_selection(&mut self) {
        self.history
            .push(UndoableAction::Selection(self.selected_tools.clone()));
    }

    /// Record a filter change
    fn record_filter(&mut self) {
        self.history
            .push(UndoableAction::Filter(self.search_query.clone()));
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fuzzy_match_exact() {
        assert!(fuzzy_match("ripgrep", "ripgrep").is_some());
        let score = fuzzy_match("ripgrep", "ripgrep").unwrap();
        assert!(score > 100); // Exact match bonus
    }

    #[test]
    fn test_fuzzy_match_prefix() {
        assert!(fuzzy_match("rip", "ripgrep").is_some());
        let score = fuzzy_match("rip", "ripgrep").unwrap();
        assert!(score > 50); // Prefix bonus
    }

    #[test]
    fn test_fuzzy_match_subsequence() {
        // "rg" matches "ripgrep" (r...g)
        assert!(fuzzy_match("rg", "ripgrep").is_some());

        // "fdf" matches "fd-find"
        assert!(fuzzy_match("fdf", "fd-find").is_some());
    }

    #[test]
    fn test_fuzzy_match_no_match() {
        // Characters must appear in order in target
        assert!(fuzzy_match("xyz", "ripgrep").is_none());
        assert!(fuzzy_match("abc", "ripgrep").is_none());
        // "gr" actually matches ripGRep (g at 3, r at 4)
        assert!(fuzzy_match("gr", "ripgrep").is_some());
    }

    #[test]
    fn test_fuzzy_match_case_insensitive() {
        assert!(fuzzy_match("RIP", "ripgrep").is_some());
        assert!(fuzzy_match("rip", "RIPGREP").is_some());
    }

    #[test]
    fn test_fuzzy_match_word_boundary_bonus() {
        // Matching at word boundary should score higher
        let boundary_score = fuzzy_match("f", "fd-find").unwrap();
        let mid_score = fuzzy_match("i", "fd-find").unwrap();
        assert!(boundary_score > mid_score);
    }

    #[test]
    fn test_fuzzy_match_consecutive_bonus() {
        // Consecutive matches should score higher
        let consecutive = fuzzy_match("rip", "ripgrep").unwrap();
        let spread = fuzzy_match("rgp", "ripgrep").unwrap(); // r...g...p (positions 0,3,6)
        assert!(consecutive > spread);
    }

    // ==================== Command Palette Tests ====================

    #[test]
    fn test_command_mode_enter_exit() {
        let db = Database::open_in_memory().unwrap();
        let mut app = App::new(&db).unwrap();

        assert_eq!(app.input_mode, InputMode::Normal);
        assert!(app.command_input.is_empty());

        app.enter_command();
        assert_eq!(app.input_mode, InputMode::Command);
        assert!(app.command_input.is_empty());

        app.command_push('q');
        assert_eq!(app.command_input, "q");

        app.exit_command();
        assert_eq!(app.input_mode, InputMode::Normal);
        assert!(app.command_input.is_empty());
    }

    #[test]
    fn test_command_push_pop() {
        let db = Database::open_in_memory().unwrap();
        let mut app = App::new(&db).unwrap();

        app.enter_command();
        app.command_push('h');
        app.command_push('e');
        app.command_push('l');
        app.command_push('p');
        assert_eq!(app.command_input, "help");

        app.command_pop();
        assert_eq!(app.command_input, "hel");

        app.command_pop();
        app.command_pop();
        app.command_pop();
        assert!(app.command_input.is_empty());
    }

    #[test]
    fn test_command_execute_help() {
        let db = Database::open_in_memory().unwrap();
        let mut app = App::new(&db).unwrap();

        app.enter_command();
        app.command_push('h');
        app.execute_command(&db);

        assert!(app.show_help);
        assert_eq!(app.input_mode, InputMode::Normal);
    }

    #[test]
    fn test_command_execute_quit() {
        let db = Database::open_in_memory().unwrap();
        let mut app = App::new(&db).unwrap();

        assert!(app.running);
        app.enter_command();
        app.command_push('q');
        app.execute_command(&db);

        assert!(!app.running);
    }

    #[test]
    fn test_command_unknown() {
        let db = Database::open_in_memory().unwrap();
        let mut app = App::new(&db).unwrap();

        app.enter_command();
        for c in "invalidcmd".chars() {
            app.command_push(c);
        }
        app.execute_command(&db);

        // Should have status message about unknown command
        assert!(app.status_message.is_some());
        assert!(app.status_message.as_ref().unwrap().is_error);
    }

    // ==================== Undo/Redo Tests ====================

    #[test]
    fn test_undo_selection() {
        let db = Database::open_in_memory().unwrap();
        let mut app = App::new(&db).unwrap();

        // Initial state - no selections
        assert!(app.selected_tools.is_empty());

        // Record initial empty state, then add selections
        app.record_selection();
        app.selected_tools.insert("tool1".to_string());
        app.selected_tools.insert("tool2".to_string());

        // Undo should restore to empty state
        app.undo();
        assert!(app.selected_tools.is_empty());
    }

    #[test]
    fn test_undo_filter() {
        let db = Database::open_in_memory().unwrap();
        let mut app = App::new(&db).unwrap();

        // Set a filter and record it
        app.search_query = "old_filter".to_string();
        app.record_filter();
        app.search_query = "new_filter".to_string();

        // Undo should restore old filter
        app.undo();
        assert_eq!(app.search_query, "old_filter");
    }

    #[test]
    fn test_redo() {
        let db = Database::open_in_memory().unwrap();
        let mut app = App::new(&db).unwrap();

        // Set filter and record
        app.search_query = "filter1".to_string();
        app.record_filter();
        app.search_query = "filter2".to_string();

        // Undo
        app.undo();
        assert_eq!(app.search_query, "filter1");

        // Redo should restore to filter2
        app.redo();
        assert_eq!(app.search_query, "filter2");
    }

    #[test]
    fn test_action_history() {
        let mut history = ActionHistory::new(3);

        // Initially empty
        assert!(!history.can_undo());
        assert!(!history.can_redo());

        // Add actions
        history.push(UndoableAction::Filter("a".to_string()));
        history.push(UndoableAction::Filter("b".to_string()));
        assert!(history.can_undo());

        // Pop undo
        let action = history.pop_undo().unwrap();
        if let UndoableAction::Filter(s) = action {
            assert_eq!(s, "b");
        }

        // Push to redo
        history.push_redo(UndoableAction::Filter("b".to_string()));
        assert!(history.can_redo());

        // Pop redo
        let action = history.pop_redo().unwrap();
        if let UndoableAction::Filter(s) = action {
            assert_eq!(s, "b");
        }
    }

    #[test]
    fn test_history_max_size() {
        let mut history = ActionHistory::new(2);

        history.push(UndoableAction::Filter("a".to_string()));
        history.push(UndoableAction::Filter("b".to_string()));
        history.push(UndoableAction::Filter("c".to_string()));

        // Should only have 2 actions (oldest removed)
        assert!(history.can_undo());
        let _ = history.pop_undo(); // c
        let action = history.pop_undo(); // b
        if let Some(UndoableAction::Filter(s)) = action {
            assert_eq!(s, "b");
        }

        // No more undo
        assert!(!history.can_undo());
    }
}
