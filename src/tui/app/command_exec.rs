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
}
