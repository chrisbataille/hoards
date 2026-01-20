//! Type definitions for the TUI application state
//!
//! This module contains enums, state structs, and constants used throughout the TUI.

use std::collections::HashSet;

use crate::config::{AiProvider, ClaudeModel, HoardConfig, SourcesConfig, TuiTheme, UsageMode};
use crate::models::InstallSource;

/// An install option for a discovered tool
#[derive(Debug, Clone)]
pub struct InstallOption {
    pub source: DiscoverSource,
    pub install_command: String,
}

/// A search result from the Discover tab
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
    /// Get the primary install command
    pub fn install_command(&self) -> Option<&str> {
        self.install_options
            .first()
            .map(|o| o.install_command.as_str())
    }
}

/// Source of a discover result
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiscoverSource {
    GitHub,
    CratesIo,
    PyPI,
    Npm,
    Apt,
    Homebrew,
    AI,
}

impl DiscoverSource {
    pub fn to_install_source(&self) -> InstallSource {
        match self {
            DiscoverSource::GitHub => InstallSource::Unknown,
            DiscoverSource::CratesIo => InstallSource::Cargo,
            DiscoverSource::PyPI => InstallSource::Pip,
            DiscoverSource::Npm => InstallSource::Npm,
            DiscoverSource::Apt => InstallSource::Apt,
            DiscoverSource::Homebrew => InstallSource::Brew,
            DiscoverSource::AI => InstallSource::Unknown,
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            DiscoverSource::GitHub => "\u{f09b}", //
            DiscoverSource::CratesIo => "🦀",
            DiscoverSource::PyPI => "🐍",
            DiscoverSource::Npm => "\u{e71e}", //
            DiscoverSource::Apt => "📦",
            DiscoverSource::Homebrew => "🍺",
            DiscoverSource::AI => "🤖",
        }
    }
}

/// Section of the config menu
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ConfigSection {
    #[default]
    AiProvider,
    ClaudeModel, // Only relevant when Claude is selected
    Theme,
    Sources,
    UsageMode,
    Buttons, // Save/Cancel
}

impl ConfigSection {
    pub fn all() -> &'static [ConfigSection] {
        &[
            ConfigSection::AiProvider,
            ConfigSection::ClaudeModel,
            ConfigSection::Theme,
            ConfigSection::Sources,
            ConfigSection::UsageMode,
            ConfigSection::Buttons,
        ]
    }

    pub fn next(&self) -> ConfigSection {
        match self {
            ConfigSection::AiProvider => ConfigSection::ClaudeModel,
            ConfigSection::ClaudeModel => ConfigSection::Theme,
            ConfigSection::Theme => ConfigSection::Sources,
            ConfigSection::Sources => ConfigSection::UsageMode,
            ConfigSection::UsageMode => ConfigSection::Buttons,
            ConfigSection::Buttons => ConfigSection::AiProvider,
        }
    }

    pub fn prev(&self) -> ConfigSection {
        match self {
            ConfigSection::AiProvider => ConfigSection::Buttons,
            ConfigSection::ClaudeModel => ConfigSection::AiProvider,
            ConfigSection::Theme => ConfigSection::ClaudeModel,
            ConfigSection::Sources => ConfigSection::Theme,
            ConfigSection::UsageMode => ConfigSection::Sources,
            ConfigSection::Buttons => ConfigSection::UsageMode,
        }
    }

    /// Get the starting line number for this section in the config menu.
    /// Used for click detection and auto-scroll.
    ///
    /// Layout (without custom theme description):
    /// - Lines 0-5: AI Provider (header + 5 options)
    /// - Line 6: empty
    /// - Lines 7-10: Claude Model (header + 3 options)
    /// - Line 11: empty
    /// - Lines 12-19: Theme (header + 7 options)
    /// - Line 20: empty
    /// - Lines 21-28: Sources (header + 7 options)
    /// - Line 29: empty
    /// - Lines 30-32: Usage (header + 2 options)
    /// - Line 33: empty
    /// - Line 34: Buttons
    pub fn start_line(&self, custom_theme_selected: bool) -> usize {
        let theme_extra = if custom_theme_selected { 1 } else { 0 };
        match self {
            Self::AiProvider => 0,
            Self::ClaudeModel => 7,
            Self::Theme => 12,
            Self::Sources => 21 + theme_extra,
            Self::UsageMode => 30 + theme_extra,
            Self::Buttons => 34 + theme_extra,
        }
    }

    /// Get the line range for items in this section (excluding header).
    /// Returns (first_item_line, last_item_line) inclusive.
    pub fn item_lines(&self, custom_theme_selected: bool) -> (usize, usize) {
        let theme_extra = if custom_theme_selected { 1 } else { 0 };
        match self {
            Self::AiProvider => (1, 5),                              // 5 AI providers
            Self::ClaudeModel => (8, 10), // 3 models (Haiku, Sonnet, Opus)
            Self::Theme => (13, 19),      // 7 themes (indices 0-6)
            Self::Sources => (22 + theme_extra, 28 + theme_extra), // 7 sources
            Self::UsageMode => (31 + theme_extra, 32 + theme_extra), // 2 modes
            Self::Buttons => (34 + theme_extra, 34 + theme_extra), // 1 line
        }
    }

    /// Number of selectable items in this section
    pub fn item_count(&self) -> usize {
        match self {
            Self::AiProvider => 5,  // None, Claude, Gemini, Codex, Opencode
            Self::ClaudeModel => 3, // Haiku, Sonnet, Opus
            Self::Theme => 7,       // 6 built-in + Custom
            Self::Sources => 7,     // cargo, apt, pip, npm, brew, flatpak, manual
            Self::UsageMode => 2,   // Scan, Hook
            Self::Buttons => 2,     // Save, Cancel
        }
    }
}

/// Config menu layout constants
pub mod config_menu_layout {
    /// Base number of lines in config menu (without custom theme description)
    /// Updated for Claude Model section: 35 lines total
    pub const TOTAL_LINES_BASE: usize = 35;
    /// Extra line when custom theme is selected (for file path hint)
    pub const CUSTOM_THEME_EXTRA_LINES: usize = 1;
    /// Index of custom theme
    pub const CUSTOM_THEME_INDEX: usize = 6;

    /// Calculate total lines based on whether custom theme is selected
    pub fn total_lines(custom_theme_selected: bool) -> usize {
        if custom_theme_selected {
            TOTAL_LINES_BASE + CUSTOM_THEME_EXTRA_LINES
        } else {
            TOTAL_LINES_BASE
        }
    }
}

/// State for the config menu
#[derive(Debug, Clone)]
pub struct ConfigMenuState {
    /// Currently focused section
    pub section: ConfigSection,
    /// Selected index within current section (for radio buttons)
    pub ai_selected: usize,
    /// Claude model selection (0=Haiku, 1=Sonnet, 2=Opus)
    pub claude_model_selected: usize,
    pub theme_selected: usize,
    pub usage_selected: usize,
    /// Source toggles (separate state for checkboxes)
    pub sources: SourcesConfig,
    /// Which source is focused (0-6)
    pub source_focused: usize,
    /// Button focus (0=Save, 1=Cancel)
    pub button_focused: usize,
    /// Scroll offset for the config menu content
    pub scroll_offset: usize,
}

impl Default for ConfigMenuState {
    fn default() -> Self {
        Self {
            section: ConfigSection::AiProvider,
            ai_selected: 0,           // None
            claude_model_selected: 0, // Haiku (default)
            theme_selected: 0,
            usage_selected: 0, // Scan
            sources: SourcesConfig::default(),
            source_focused: 0,
            button_focused: 0, // Save
            scroll_offset: 0,
        }
    }
}

impl ConfigMenuState {
    /// Initialize from existing config
    pub fn from_config(config: &HoardConfig) -> Self {
        Self {
            section: ConfigSection::AiProvider,
            ai_selected: AiProvider::all()
                .iter()
                .position(|p| *p == config.ai.provider)
                .unwrap_or(0),
            claude_model_selected: match config.ai.claude_model {
                ClaudeModel::Haiku => 0,
                ClaudeModel::Sonnet => 1,
                ClaudeModel::Opus => 2,
            },
            theme_selected: config.tui.theme.index(),
            usage_selected: match config.usage.mode {
                UsageMode::Scan => 0,
                UsageMode::Hook => 1,
            },
            sources: config.sources.clone(),
            source_focused: 0,
            button_focused: 0,
            scroll_offset: 0,
        }
    }

    /// Build config from current state
    pub fn to_config(&self) -> HoardConfig {
        let mut config = HoardConfig::default();
        config.ai.provider = AiProvider::all()[self.ai_selected];
        config.ai.claude_model = match self.claude_model_selected {
            0 => ClaudeModel::Haiku,
            1 => ClaudeModel::Sonnet,
            _ => ClaudeModel::Opus,
        };
        config.tui.theme = TuiTheme::from_index(self.theme_selected);
        config.usage.mode = if self.usage_selected == 0 {
            UsageMode::Scan
        } else {
            UsageMode::Hook
        };
        config.sources = self.sources.clone();
        config
    }

    /// Move to next item in current section
    pub fn next_item(&mut self) {
        let count = self.section.item_count();
        match self.section {
            ConfigSection::AiProvider => {
                self.ai_selected = (self.ai_selected + 1) % count;
            }
            ConfigSection::ClaudeModel => {
                self.claude_model_selected = (self.claude_model_selected + 1) % count;
            }
            ConfigSection::Theme => {
                self.theme_selected = (self.theme_selected + 1) % count;
            }
            ConfigSection::Sources => {
                self.source_focused = (self.source_focused + 1) % count;
            }
            ConfigSection::UsageMode => {
                self.usage_selected = (self.usage_selected + 1) % count;
            }
            ConfigSection::Buttons => {
                self.button_focused = (self.button_focused + 1) % count;
            }
        }
    }

    /// Move to prev item in current section
    pub fn prev_item(&mut self) {
        let count = self.section.item_count();
        match self.section {
            ConfigSection::AiProvider => {
                self.ai_selected = if self.ai_selected == 0 {
                    count - 1
                } else {
                    self.ai_selected - 1
                };
            }
            ConfigSection::ClaudeModel => {
                self.claude_model_selected = if self.claude_model_selected == 0 {
                    count - 1
                } else {
                    self.claude_model_selected - 1
                };
            }
            ConfigSection::Theme => {
                self.theme_selected = if self.theme_selected == 0 {
                    count - 1
                } else {
                    self.theme_selected - 1
                };
            }
            ConfigSection::Sources => {
                self.source_focused = if self.source_focused == 0 {
                    count - 1
                } else {
                    self.source_focused - 1
                };
            }
            ConfigSection::UsageMode => {
                self.usage_selected = if self.usage_selected == 0 {
                    count - 1
                } else {
                    self.usage_selected - 1
                };
            }
            ConfigSection::Buttons => {
                self.button_focused = if self.button_focused == 0 {
                    count - 1
                } else {
                    self.button_focused - 1
                };
            }
        }
    }

    /// Toggle the current source checkbox (only for Sources section)
    pub fn toggle_current_source(&mut self) {
        if self.section == ConfigSection::Sources {
            let sources = SourcesConfig::all_sources();
            if self.source_focused < sources.len() {
                self.sources.toggle(sources[self.source_focused]);
            }
        }
    }

    /// Scroll up by one line
    pub fn scroll_up(&mut self) {
        self.scroll_offset = self.scroll_offset.saturating_sub(1);
    }

    /// Scroll down by one line (with max limit)
    pub fn scroll_down(&mut self, max_scroll: usize) {
        if self.scroll_offset < max_scroll {
            self.scroll_offset += 1;
        }
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
    Discover,
}

impl Tab {
    pub fn all() -> &'static [Tab] {
        &[
            Tab::Installed,
            Tab::Available,
            Tab::Updates,
            Tab::Bundles,
            Tab::Discover,
        ]
    }

    pub fn title(&self) -> &'static str {
        match self {
            Tab::Installed => "Installed",
            Tab::Available => "Available",
            Tab::Updates => "Updates",
            Tab::Bundles => "Bundles",
            Tab::Discover => "Discover",
        }
    }

    pub fn index(&self) -> usize {
        match self {
            Tab::Installed => 0,
            Tab::Available => 1,
            Tab::Updates => 2,
            Tab::Bundles => 3,
            Tab::Discover => 4,
        }
    }

    pub fn from_index(index: usize) -> Option<Tab> {
        match index {
            0 => Some(Tab::Installed),
            1 => Some(Tab::Available),
            2 => Some(Tab::Updates),
            3 => Some(Tab::Bundles),
            4 => Some(Tab::Discover),
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
    Command,      // Vim-style command palette with ':'
    JumpToLetter, // Waiting for letter input to jump to
}

/// Info needed to install/update a tool
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstallTask {
    pub name: String,
    pub source: String,          // "cargo", "pip", etc.
    pub version: Option<String>, // Target version for updates
    pub display_command: String, // For confirmation dialog
}

/// Result of an install/update attempt
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstallResult {
    pub name: String,
    pub success: bool,
    pub error: Option<String>,
}

/// Background operation that needs loading indicator
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BackgroundOp {
    CheckUpdates {
        step: usize,
    },
    DiscoverSearch {
        query: String,
        step: usize,
        source_names: Vec<String>,
    },
    ExecuteInstall {
        tasks: Vec<InstallTask>,
        current: usize,
        results: Vec<InstallResult>,
    },
    ExecuteUpdate {
        tasks: Vec<InstallTask>,
        current: usize,
        results: Vec<InstallResult>,
    },
}

impl BackgroundOp {
    pub fn title(&self) -> &'static str {
        match self {
            BackgroundOp::CheckUpdates { .. } => "Checking for Updates",
            BackgroundOp::DiscoverSearch { .. } => "Searching",
            BackgroundOp::ExecuteInstall { .. } => "Installing",
            BackgroundOp::ExecuteUpdate { .. } => "Updating",
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
pub const PACKAGE_MANAGERS: &[(&str, &str)] = &[
    ("cargo", "Cargo (Rust)"),
    ("pip", "pip (Python)"),
    ("npm", "npm (Node.js)"),
    ("apt", "apt (Debian/Ubuntu)"),
    ("brew", "Homebrew"),
];

/// Pending action requiring confirmation
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PendingAction {
    Install(Vec<InstallTask>),    // Tools to install (with metadata)
    Uninstall(Vec<String>),       // Tool names to uninstall
    Update(Vec<InstallTask>),     // Tools to update (with metadata)
    DiscoverInstall(InstallTask), // Single discover install
}

impl PendingAction {
    pub fn description(&self) -> String {
        match self {
            PendingAction::Install(tasks) => {
                if tasks.len() == 1 {
                    format!("Install {}?", tasks[0].name)
                } else {
                    format!("Install {} tools?", tasks.len())
                }
            }
            PendingAction::Uninstall(tools) => {
                if tools.len() == 1 {
                    format!("Uninstall {}?", tools[0])
                } else {
                    format!("Uninstall {} tools?", tools.len())
                }
            }
            PendingAction::Update(tasks) => {
                if tasks.len() == 1 {
                    format!("Update {}?", tasks[0].name)
                } else {
                    format!("Update {} tools?", tasks.len())
                }
            }
            PendingAction::DiscoverInstall(task) => {
                format!("Install {}?", task.name)
            }
        }
    }

    /// Get tool names for display
    pub fn tool_names(&self) -> Vec<&str> {
        match self {
            PendingAction::Install(tasks) => tasks.iter().map(|t| t.name.as_str()).collect(),
            PendingAction::Uninstall(tools) => tools.iter().map(|s| s.as_str()).collect(),
            PendingAction::Update(tasks) => tasks.iter().map(|t| t.name.as_str()).collect(),
            PendingAction::DiscoverInstall(task) => vec![task.name.as_str()],
        }
    }

    /// Get install tasks (for Install/Update/DiscoverInstall)
    pub fn install_tasks(&self) -> Option<Vec<&InstallTask>> {
        match self {
            PendingAction::Install(tasks) => Some(tasks.iter().collect()),
            PendingAction::Update(tasks) => Some(tasks.iter().collect()),
            PendingAction::DiscoverInstall(task) => Some(vec![task]),
            PendingAction::Uninstall(_) => None,
        }
    }
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
    pub undo_stack: Vec<UndoableAction>,
    pub redo_stack: Vec<UndoableAction>,
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

/// Status message to display temporarily
#[derive(Debug, Clone)]
pub struct StatusMessage {
    pub text: String,
    pub is_error: bool,
}

/// Notification level for toast display
#[derive(Debug, Clone, PartialEq)]
pub enum NotificationLevel {
    Info,
    Warning,
    Error,
}

/// Toast notification with auto-dismiss
#[derive(Debug, Clone)]
pub struct Notification {
    pub text: String,
    pub level: NotificationLevel,
    pub created_at: std::time::Instant,
}

impl Notification {
    pub fn info(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            level: NotificationLevel::Info,
            created_at: std::time::Instant::now(),
        }
    }

    pub fn warning(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            level: NotificationLevel::Warning,
            created_at: std::time::Instant::now(),
        }
    }

    pub fn error(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            level: NotificationLevel::Error,
            created_at: std::time::Instant::now(),
        }
    }

    /// Duration before auto-dismiss (errors stay longer)
    pub fn dismiss_duration(&self) -> std::time::Duration {
        match self.level {
            NotificationLevel::Info => std::time::Duration::from_secs(3),
            NotificationLevel::Warning => std::time::Duration::from_secs(5),
            NotificationLevel::Error => std::time::Duration::from_secs(8),
        }
    }

    /// Check if notification should be dismissed
    pub fn should_dismiss(&self) -> bool {
        self.created_at.elapsed() >= self.dismiss_duration()
    }
}

/// Modal error popup that blocks until dismissed
#[derive(Debug, Clone)]
pub struct ErrorModal {
    pub title: String,
    pub message: String,
}

/// README popup with markdown content
#[derive(Debug, Clone)]
pub struct ReadmePopup {
    pub tool_name: String,
    pub content: String,
    pub scroll_offset: u16,
    pub loading: bool,
    pub links: Vec<(String, String)>, // (text, url) pairs
    pub show_links: bool,             // Whether to show link picker
    pub selected_link: usize,         // Currently selected link in picker
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

/// Sort options for discover results
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DiscoverSortBy {
    #[default]
    Stars,
    Name,
    Source,
}

impl DiscoverSortBy {
    pub fn next(&self) -> DiscoverSortBy {
        match self {
            DiscoverSortBy::Stars => DiscoverSortBy::Name,
            DiscoverSortBy::Name => DiscoverSortBy::Source,
            DiscoverSortBy::Source => DiscoverSortBy::Stars,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            DiscoverSortBy::Stars => "stars",
            DiscoverSortBy::Name => "name",
            DiscoverSortBy::Source => "source",
        }
    }
}

/// Available commands for the command palette with descriptions
pub const COMMANDS: &[(&str, &str)] = &[
    ("q", "quit - exit the application"),
    ("quit", "quit - exit the application"),
    ("exit", "exit the application"),
    ("h", "help - show help"),
    ("help", "show help dialog"),
    ("r", "refresh - reload tools"),
    ("refresh", "reload tools from database"),
    ("t", "theme [name] - cycle or set theme"),
    ("theme", "theme [name] - cycle or set theme"),
    ("s", "sort [field] - cycle or set sort"),
    (
        "sort",
        "sort [field] - cycle or set sort (name/usage/recent)",
    ),
    (
        "filter",
        "filter [source] - filter by source (cargo/apt/pip/npm)",
    ),
    ("source", "source [name] - filter by source"),
    ("src", "src [name] - filter by source"),
    ("fav", "fav - toggle favorites filter"),
    ("favorites", "favorites - toggle favorites filter"),
    ("starred", "starred - toggle favorites filter"),
    ("1", "go to Installed tab"),
    ("installed", "go to Installed tab"),
    ("2", "go to Available tab"),
    ("available", "go to Available tab"),
    ("3", "go to Updates tab"),
    ("updates", "go to Updates tab"),
    ("4", "go to Bundles tab"),
    ("bundles", "go to Bundles tab"),
    ("5", "go to Discover tab"),
    ("discover", "go to Discover tab"),
    ("i", "install selected item"),
    ("install", "install selected tool/bundle"),
    ("d", "delete/uninstall selected"),
    ("delete", "delete selected tool"),
    ("uninstall", "uninstall selected tool"),
    ("u", "update selected"),
    ("update", "update selected tool"),
    ("upgrade", "upgrade selected tool"),
    ("undo", "undo last action"),
    ("z", "undo last action"),
    ("redo", "redo undone action"),
    ("y", "redo undone action"),
    ("c", "config - open configuration menu"),
    ("config", "open configuration menu"),
    ("settings", "open configuration menu"),
    ("cfg", "open configuration menu"),
    ("create-theme", "create custom theme file"),
    ("new-theme", "create custom theme file"),
    ("edit-theme", "show custom theme file path"),
];
