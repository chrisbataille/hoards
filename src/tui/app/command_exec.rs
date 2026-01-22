//! Command execution for the TUI
//!
//! This module contains the execute_command method and related helpers
//! for theme, sort, and filter commands.

use crate::db::Database;

use super::App;
use super::types::{COMMANDS, SortBy, Tab};

impl App {
    // ========================================================================
    // Command Palette Methods
    // ========================================================================

    /// Enter command mode (vim-style ':')
    pub fn enter_command(&mut self) {
        use super::types::InputMode;
        self.input_mode = InputMode::Command;
        self.command.clear();
    }

    /// Exit command mode
    pub fn exit_command(&mut self) {
        use super::types::InputMode;
        self.input_mode = InputMode::Normal;
        self.command.clear();
    }

    /// Add character to command input
    pub fn command_push(&mut self, c: char) {
        self.command.input.push(c);
    }

    /// Remove last character from command input
    pub fn command_pop(&mut self) {
        self.command.input.pop();
    }

    /// Get command suggestions based on current input
    pub fn get_command_suggestions(&self) -> Vec<(&'static str, &'static str)> {
        let input = self.command.input.trim().to_lowercase();
        if input.is_empty() {
            return Vec::new();
        }

        COMMANDS
            .iter()
            .filter(|(cmd, _)| cmd.starts_with(&input))
            .take(5) // Limit to 5 suggestions
            .copied()
            .collect()
    }

    /// Autocomplete the current command with the first suggestion
    pub fn autocomplete_command(&mut self) {
        let suggestions = self.get_command_suggestions();
        if let Some((cmd, _)) = suggestions.first() {
            self.command.input = cmd.to_string();
        }
    }

    /// Navigate to previous command in history (Up arrow)
    pub fn command_history_prev(&mut self) {
        self.command.history_prev();
    }

    /// Navigate to next command in history (Down arrow)
    pub fn command_history_next(&mut self) {
        self.command.history_next();
    }

    /// Execute the current command
    pub fn execute_command(&mut self, db: &Database) {
        let cmd = self.command.input.trim().to_lowercase();
        let parts: Vec<&str> = cmd.split_whitespace().collect();

        if parts.is_empty() {
            self.exit_command();
            return;
        }

        // Add to command history
        self.command.add_to_history(cmd.clone());
        self.command.clear_history_nav();

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

            // Source filter commands
            "filter" | "source" | "src" => {
                if parts.len() > 1 {
                    self.set_source_filter(Some(parts[1]));
                } else {
                    self.set_source_filter(None); // Clear filter
                }
                self.exit_command();
            }

            // Favorites commands
            "fav" | "favorites" | "starred" => {
                self.toggle_favorites_filter();
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
            "discover" | "5" => {
                self.switch_tab(Tab::Discover, db);
                self.exit_command();
            }

            // Install/Uninstall/Update
            "i" | "install" => {
                if self.tab == Tab::Bundles {
                    self.request_bundle_install(db);
                } else if self.tab == Tab::Discover {
                    self.request_discover_install();
                } else {
                    self.request_install(db);
                }
                self.exit_command();
            }
            "d" | "delete" | "uninstall" => {
                self.request_uninstall();
                self.exit_command();
            }
            "u" | "update" | "upgrade" => {
                self.request_update(db);
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

            // Config
            "c" | "config" | "settings" | "cfg" => {
                self.open_config_menu();
                self.exit_command();
            }

            // Create custom theme
            "create-theme" | "new-theme" => {
                self.create_custom_theme();
                self.exit_command();
            }

            // Edit custom theme (open file location)
            "edit-theme" => {
                self.show_custom_theme_path();
                self.exit_command();
            }

            // Label commands
            "label" | "labels" => {
                if parts.len() == 1 || parts.get(1) == Some(&"filter") {
                    // :label or :label filter or :labels -> open filter popup
                    self.exit_command();
                    self.toggle_label_filter_popup();
                } else if parts.get(1) == Some(&"auto") {
                    self.auto_label_selected(db);
                    self.exit_command();
                } else if parts.get(1) == Some(&"edit") {
                    self.exit_command();
                    self.open_label_edit_popup();
                } else if parts.get(1) == Some(&"clear") {
                    self.clear_label_filter();
                    self.exit_command();
                } else {
                    self.set_status("Usage: label [auto|filter|edit|clear]".to_string(), true);
                    self.exit_command();
                }
            }

            // Unknown command
            _ => {
                self.set_status(format!("Unknown command: {}", parts[0]), true);
                self.exit_command();
            }
        }
    }

    // ========================================================================
    // Theme Helpers
    // ========================================================================

    /// Set theme by name
    pub(super) fn set_theme_by_name(&mut self, name: &str) {
        use super::super::theme::{CustomTheme, ThemeVariant};
        self.theme_variant = match name {
            "mocha" | "catppuccin" | "catppuccin-mocha" => ThemeVariant::CatppuccinMocha,
            "latte" | "catppuccin-latte" => ThemeVariant::CatppuccinLatte,
            "dracula" => ThemeVariant::Dracula,
            "nord" => ThemeVariant::Nord,
            "tokyo" | "tokyo-night" | "tokyonight" => ThemeVariant::TokyoNight,
            "gruvbox" => ThemeVariant::Gruvbox,
            "custom" => {
                if CustomTheme::exists() {
                    ThemeVariant::Custom
                } else {
                    self.set_status(
                        "Custom theme not found. Use :create-theme to create one.".to_string(),
                        true,
                    );
                    return;
                }
            }
            _ => {
                self.set_status(
                    "Themes: mocha, latte, dracula, nord, tokyo, gruvbox, custom".to_string(),
                    true,
                );
                return;
            }
        };
        self.set_status(format!("Theme: {}", self.theme().name), false);
    }

    /// Create custom theme file
    pub(super) fn create_custom_theme(&mut self) {
        use super::super::theme::CustomTheme;

        if CustomTheme::exists() {
            if let Ok(path) = CustomTheme::file_path() {
                self.set_status(
                    format!("Custom theme already exists: {}", path.display()),
                    false,
                );
            } else {
                self.set_status("Custom theme already exists".to_string(), false);
            }
            return;
        }

        match CustomTheme::create_default_if_missing() {
            Ok(true) => {
                if let Ok(path) = CustomTheme::file_path() {
                    self.set_status(format!("Created custom theme: {}", path.display()), false);
                } else {
                    self.set_status("Created custom theme file".to_string(), false);
                }
            }
            Ok(false) => {
                self.set_status("Custom theme already exists".to_string(), false);
            }
            Err(e) => {
                self.set_status(format!("Failed to create theme: {}", e), true);
            }
        }
    }

    /// Show custom theme file path
    pub(super) fn show_custom_theme_path(&mut self) {
        use super::super::theme::CustomTheme;

        match CustomTheme::file_path() {
            Ok(path) => {
                if path.exists() {
                    self.set_status(format!("Custom theme: {}", path.display()), false);
                } else {
                    self.set_status(
                        "Custom theme not found. Create with :create-theme".to_string(),
                        true,
                    );
                }
            }
            Err(e) => {
                self.set_status(format!("Error: {}", e), true);
            }
        }
    }

    // ========================================================================
    // Sort Helpers
    // ========================================================================

    /// Set sort by name
    pub(super) fn set_sort_by_name(&mut self, name: &str) {
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

    // ========================================================================
    // Filter Helpers
    // ========================================================================

    /// Set source filter
    pub fn set_source_filter(&mut self, source: Option<&str>) {
        match source {
            Some(s) if !s.is_empty() => {
                self.source_filter = Some(s.to_lowercase());
                self.set_status(format!("Filter: source={}", s), false);
            }
            _ => {
                self.source_filter = None;
                self.set_status("Source filter cleared".to_string(), false);
            }
        }
        self.apply_filter_and_sort();
    }

    /// Toggle favorites-only filter
    pub fn toggle_favorites_filter(&mut self) {
        self.favorites_only = !self.favorites_only;
        let status = if self.favorites_only {
            "Showing favorites only"
        } else {
            "Showing all tools"
        };
        self.set_status(status.to_string(), false);
        self.apply_filter_and_sort();
    }

    /// Toggle label filter popup
    pub fn toggle_label_filter_popup(&mut self) {
        self.show_label_filter_popup = !self.show_label_filter_popup;
        if self.show_label_filter_popup {
            // Start at first label (index 1), not "Clear filter" (index 0)
            // If there are no labels, the render/event will handle it
            self.label_filter_selected = 1;
            self.label_filter_scroll = 0;
            self.label_filter_search.clear();
        }
    }

    /// Close label filter popup
    pub fn close_label_filter_popup(&mut self) {
        self.show_label_filter_popup = false;
    }

    /// Toggle a label in the filter (add if not present, remove if present)
    pub fn toggle_label_filter(&mut self, label: &str) {
        if self.label_filter.contains(label) {
            self.label_filter.remove(label);
            if self.label_filter.is_empty() {
                self.set_status("Label filter cleared".to_string(), false);
            } else {
                self.set_status(format!("Removed label: {}", label), false);
            }
        } else {
            self.label_filter.insert(label.to_string());
            self.set_status(format!("Added label filter: {}", label), false);
        }
        self.apply_filter_and_sort();
    }

    /// Clear all label filters
    pub fn clear_label_filter(&mut self) {
        self.label_filter.clear();
        self.set_status("Label filter cleared".to_string(), false);
        self.apply_filter_and_sort();
    }

    /// Open label edit popup for the selected tool
    pub fn open_label_edit_popup(&mut self) {
        if let Some(tool) = self.selected_tool() {
            let tool_name = tool.name.clone();
            let labels = self
                .cache
                .labels_cache
                .get(&tool_name)
                .cloned()
                .unwrap_or_default();
            self.label_edit_tool = Some(tool_name);
            self.label_edit_labels = labels;
            self.label_edit_input = String::new();
            self.label_edit_selected = 0;
            self.label_edit_suggestions = Vec::new();
            self.show_label_edit_popup = true;
        }
    }

    /// Close label edit popup
    pub fn close_label_edit_popup(&mut self) {
        self.show_label_edit_popup = false;
        self.label_edit_tool = None;
        self.label_edit_input.clear();
        self.label_edit_labels.clear();
        self.label_edit_suggestions.clear();
        self.label_edit_selected = 0;
    }

    /// Update label suggestions based on current input
    pub fn update_label_suggestions(&mut self, db: &Database) {
        use super::fuzzy_match;

        if self.label_edit_input.is_empty() {
            self.label_edit_suggestions.clear();
            return;
        }

        // Get all unique labels from the database
        let all_labels = db.get_all_labels().unwrap_or_default();

        // Filter by fuzzy match and exclude labels already on the tool
        let query = self.label_edit_input.to_lowercase();
        let mut matches: Vec<(String, i32)> = all_labels
            .into_iter()
            .filter(|label| {
                // Exclude labels already on this tool
                !self.label_edit_labels.contains(label)
            })
            .filter_map(|label| fuzzy_match(&query, &label).map(|score| (label, score)))
            .collect();

        // Sort by score (higher = better match)
        matches.sort_by(|a, b| b.1.cmp(&a.1));

        // Take top 5 suggestions
        self.label_edit_suggestions = matches.into_iter().take(5).map(|(l, _)| l).collect();
    }

    /// Add a label to the tool being edited
    /// If a suggestion is selected, add that; otherwise add the typed input
    pub fn label_edit_add(&mut self, db: &Database) {
        let suggestions_count = self.label_edit_suggestions.len();

        // Determine which label to add
        let label = if self.label_edit_selected > 0 && self.label_edit_selected <= suggestions_count
        {
            // Selected a suggestion
            self.label_edit_suggestions[self.label_edit_selected - 1].clone()
        } else {
            // Use typed input
            self.label_edit_input
                .trim()
                .to_lowercase()
                .replace(' ', "-")
        };

        if label.is_empty() {
            return;
        }

        if let Some(ref tool_name) = self.label_edit_tool
            && !self.label_edit_labels.contains(&label)
            && db
                .add_labels(tool_name, std::slice::from_ref(&label))
                .is_ok()
        {
            self.label_edit_labels.push(label.clone());
            self.label_edit_labels.sort();
            // Update cache
            self.cache
                .labels_cache
                .insert(tool_name.clone(), self.label_edit_labels.clone());
            self.set_status(format!("Added label: {}", label), false);
        }

        // Clear input and suggestions, reset selection
        self.label_edit_input.clear();
        self.label_edit_suggestions.clear();
        self.label_edit_selected = 0;
    }

    /// Remove the selected label from the tool being edited
    pub fn label_edit_remove(&mut self, db: &Database) {
        // Selection layout: 0=input, 1..=suggestions, then existing labels
        let labels_start = 1 + self.label_edit_suggestions.len();

        if self.label_edit_selected < labels_start || self.label_edit_labels.is_empty() {
            return;
        }

        let label_idx = self.label_edit_selected - labels_start;
        if label_idx < self.label_edit_labels.len()
            && let Some(ref tool_name) = self.label_edit_tool
        {
            let label = self.label_edit_labels[label_idx].clone();
            if db.remove_label(tool_name, &label).is_ok() {
                self.label_edit_labels.remove(label_idx);
                // Update cache
                self.cache
                    .labels_cache
                    .insert(tool_name.clone(), self.label_edit_labels.clone());
                self.set_status(format!("Removed label: {}", label), false);
            }
        }
    }

    /// Auto-label selected tool(s)
    pub fn auto_label_selected(&mut self, db: &Database) {
        use crate::commands::cmd_label_auto;

        // Get selected tools or current tool
        let tools_to_label: Vec<String> = if self.selected_tools.is_empty() {
            if let Some(tool) = self.selected_tool() {
                vec![tool.name.clone()]
            } else {
                self.set_status("No tool selected".to_string(), true);
                return;
            }
        } else {
            self.selected_tools.iter().cloned().collect()
        };

        let count = tools_to_label.len();
        let mut success = 0;

        for tool_name in &tools_to_label {
            // Run auto-label for each tool
            if cmd_label_auto(db, Some(tool_name.as_str()), false, false, false).is_ok() {
                success += 1;
                // Update cache with new labels
                if let Ok(labels) = db.get_labels(tool_name) {
                    self.cache.labels_cache.insert(tool_name.clone(), labels);
                }
            }
        }

        if success > 0 {
            self.set_status(format!("Auto-labeled {} tool(s)", success), false);
        } else {
            self.set_status("No tools were labeled".to_string(), true);
        }

        // Clear selection after operation
        if count > 1 {
            self.selected_tools.clear();
        }
    }
}
