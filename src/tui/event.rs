//! Event handling for the TUI

use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEventKind};
use std::time::Duration;

use super::app::{App, InputMode, PendingAction, Tab};
use crate::db::Database;

const POLL_TIMEOUT: Duration = Duration::from_millis(100);

/// Handle all input events
pub fn handle_events(app: &mut App, db: &Database) -> Result<()> {
    if event::poll(POLL_TIMEOUT)? {
        match event::read()? {
            Event::Key(key) => handle_key_event(app, key, db),
            Event::Mouse(mouse) => handle_mouse_event(app, mouse, db),
            Event::Resize(_, _) => {} // Terminal will redraw automatically
            _ => {}
        }
    }
    Ok(())
}

fn handle_key_event(app: &mut App, key: KeyEvent, db: &Database) {
    // Handle pending action confirmation first
    if app.has_pending_action() {
        match key.code {
            KeyCode::Char('y') | KeyCode::Char('Y') => {
                if let Some(action) = app.confirm_action() {
                    // Execute the action
                    execute_action(app, &action, db);
                }
            }
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                app.cancel_action();
            }
            _ => {} // Ignore other keys during confirmation
        }
        return;
    }

    // Handle error modal (highest priority - blocks all input)
    if app.has_error_modal() {
        if matches!(key.code, KeyCode::Esc | KeyCode::Enter | KeyCode::Char('q')) {
            app.close_error_modal();
        }
        return;
    }

    // Handle README popup
    if app.has_readme_popup() {
        // Check if link picker is showing
        let showing_links = app
            .readme_popup
            .as_ref()
            .map(|p| p.show_links)
            .unwrap_or(false);

        if showing_links {
            // Link picker mode
            match key.code {
                KeyCode::Esc | KeyCode::Char('q') => {
                    // Close link picker, not the whole popup
                    if let Some(popup) = &mut app.readme_popup {
                        popup.show_links = false;
                    }
                }
                KeyCode::Char('j') | KeyCode::Down => app.select_next_link(),
                KeyCode::Char('k') | KeyCode::Up => app.select_prev_link(),
                KeyCode::Enter => app.open_selected_link(),
                _ => {}
            }
        } else {
            // Normal README viewing mode
            match key.code {
                KeyCode::Esc | KeyCode::Char('q') => app.close_readme(),
                KeyCode::Char('j') | KeyCode::Down => app.scroll_readme_down(1),
                KeyCode::Char('k') | KeyCode::Up => app.scroll_readme_up(1),
                KeyCode::Char('o') => app.toggle_readme_links(), // Open link picker
                KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    app.scroll_readme_down(10)
                }
                KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    app.scroll_readme_up(10)
                }
                KeyCode::PageDown => app.scroll_readme_down(20),
                KeyCode::PageUp => app.scroll_readme_up(20),
                KeyCode::Home | KeyCode::Char('g') => {
                    if let Some(popup) = &mut app.readme_popup {
                        popup.scroll_offset = 0;
                    }
                }
                KeyCode::End | KeyCode::Char('G') => {
                    // Scroll to bottom (will be clamped by render)
                    if let Some(popup) = &mut app.readme_popup {
                        popup.scroll_offset = u16::MAX;
                    }
                }
                _ => {}
            }
        }
        return;
    }

    // Handle overlays (help, config menu, and details popup)
    if app.show_help {
        if matches!(
            key.code,
            KeyCode::Esc | KeyCode::Char('?') | KeyCode::Char('q')
        ) {
            app.show_help = false;
        }
        return;
    }

    if app.show_config_menu {
        handle_config_menu(app, key);
        return;
    }

    if app.show_details_popup {
        if matches!(key.code, KeyCode::Esc | KeyCode::Enter | KeyCode::Char('q')) {
            app.close_details_popup();
        }
        return;
    }

    // Clear status message on any key press
    app.clear_status();

    match app.input_mode {
        InputMode::Normal => handle_normal_mode(app, key, db),
        InputMode::Search => handle_search_mode(app, key, db),
        InputMode::Command => handle_command_mode(app, key, db),
        InputMode::JumpToLetter => handle_jump_mode(app, key),
    }
}

fn handle_jump_mode(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Esc => app.exit_jump_mode(),
        KeyCode::Char(c) if c.is_ascii_alphabetic() => app.jump_to_letter(c),
        _ => app.exit_jump_mode(), // Cancel on any other key
    }
}

fn handle_config_menu(app: &mut App, key: KeyEvent) {
    use super::app::ConfigSection;
    use crate::config::TuiTheme;

    match key.code {
        // Close without saving
        KeyCode::Esc => app.close_config_menu(),

        // Navigate between sections (Tab / Shift+Tab)
        KeyCode::Tab => app.config_menu_next_section(),
        KeyCode::BackTab => app.config_menu_prev_section(),

        // Navigate within section (j/k or arrows)
        KeyCode::Char('j') | KeyCode::Down => {
            app.config_menu_next_item();
            // Live preview changes
            match app.config_menu.section {
                ConfigSection::Theme => {
                    let theme = TuiTheme::from_index(app.config_menu.theme_selected);
                    app.theme_variant = super::theme::ThemeVariant::from_config_theme(theme);
                }
                ConfigSection::AiProvider => {
                    // Live preview AI provider status indicator
                    app.ai_available = app.config_menu.ai_selected != 0; // 0 = None
                }
                _ => {}
            }
        }
        KeyCode::Char('k') | KeyCode::Up => {
            app.config_menu_prev_item();
            // Live preview changes
            match app.config_menu.section {
                ConfigSection::Theme => {
                    let theme = TuiTheme::from_index(app.config_menu.theme_selected);
                    app.theme_variant = super::theme::ThemeVariant::from_config_theme(theme);
                }
                ConfigSection::AiProvider => {
                    // Live preview AI provider status indicator
                    app.ai_available = app.config_menu.ai_selected != 0; // 0 = None
                }
                _ => {}
            }
        }

        // Left/right navigation for buttons
        KeyCode::Char('h') | KeyCode::Left => {
            if app.config_menu.section == ConfigSection::Buttons {
                app.config_menu.button_focused = 0; // Save
            }
        }
        KeyCode::Char('l') | KeyCode::Right => {
            if app.config_menu.section == ConfigSection::Buttons {
                app.config_menu.button_focused = 1; // Cancel
            }
        }

        // Toggle checkbox / select radio / activate button
        KeyCode::Char(' ') => {
            match app.config_menu.section {
                ConfigSection::Sources => app.config_menu_toggle_source(),
                ConfigSection::Buttons => app.config_menu_select(),
                _ => {} // Radio buttons auto-select on navigation
            }
        }

        // Select current item / confirm
        KeyCode::Enter => app.config_menu_select(),

        // Quick save (s or Ctrl+S)
        KeyCode::Char('s') => app.save_config_menu(),

        _ => {}
    }
}

fn handle_normal_mode(app: &mut App, key: KeyEvent, db: &Database) {
    match key.code {
        // Quit
        KeyCode::Char('q') => app.quit(),
        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => app.quit(),

        // Navigation - vim style (handles tools, bundles, and discover results)
        KeyCode::Char('j') | KeyCode::Down => {
            if app.tab == Tab::Bundles {
                app.select_next_bundle();
            } else if app.tab == Tab::Discover {
                app.select_next_discover();
            } else {
                app.select_next();
            }
        }
        KeyCode::Char('k') | KeyCode::Up => {
            if app.tab == Tab::Bundles {
                app.select_prev_bundle();
            } else if app.tab == Tab::Discover {
                app.select_prev_discover();
            } else {
                app.select_prev();
            }
        }
        KeyCode::Char('g') => {
            if app.tab == Tab::Bundles {
                app.select_first_bundle();
            } else if app.tab == Tab::Discover {
                app.select_first_discover();
            } else {
                app.select_first();
            }
        }
        KeyCode::Char('G') => {
            if app.tab == Tab::Bundles {
                app.select_last_bundle();
            } else if app.tab == Tab::Discover {
                app.select_last_discover();
            } else {
                app.select_last();
            }
        }

        // Page navigation
        KeyCode::PageDown | KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            for _ in 0..10 {
                app.select_next();
            }
        }
        KeyCode::PageUp | KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            for _ in 0..10 {
                app.select_prev();
            }
        }

        // Tab switching
        KeyCode::Tab | KeyCode::Char(']') => app.next_tab(db),
        KeyCode::BackTab | KeyCode::Char('[') => app.prev_tab(db),
        KeyCode::Char('1') => app.switch_tab(Tab::Installed, db),
        KeyCode::Char('2') => app.switch_tab(Tab::Available, db),
        KeyCode::Char('3') => app.switch_tab(Tab::Updates, db),
        KeyCode::Char('4') => app.switch_tab(Tab::Bundles, db),
        KeyCode::Char('5') => app.switch_tab(Tab::Discover, db),

        // Search
        KeyCode::Char('/') => app.enter_search(),

        // Search navigation (n/N for next/prev match with wrapping)
        KeyCode::Char('n') => app.search_next(),
        KeyCode::Char('N') => app.search_prev(),

        // Jump to letter (vim f)
        KeyCode::Char('f') => app.enter_jump_mode(),

        // Toggle favorite on selected tool
        KeyCode::Char('*') => app.toggle_favorite(db),

        // Toggle favorites-only filter
        KeyCode::Char('F') => app.toggle_favorites_filter(),

        // Command palette (vim-style)
        KeyCode::Char(':') => app.enter_command(),

        // Clear search filter
        KeyCode::Esc => app.clear_search(),

        // Sort
        KeyCode::Char('s') => {
            if app.tab == Tab::Discover {
                app.cycle_discover_sort();
            } else {
                app.cycle_sort();
            }
        }

        // Discover tab: Toggle AI mode (Shift+A)
        KeyCode::Char('A') if app.tab == Tab::Discover => {
            app.toggle_discover_ai();
        }

        // Discover tab: Toggle source filters dynamically
        // F1-F6 map to available sources based on their index in config
        KeyCode::F(n @ 1..=6) if app.tab == Tab::Discover => {
            let available = app.get_available_discover_sources();
            let index = (n - 1) as usize;
            if let Some((key, _, _)) = available.get(index) {
                app.toggle_discover_source_filter(key);
            }
        }

        // Selection
        KeyCode::Char(' ') => {
            app.toggle_selection();
            app.select_next(); // Move to next after selecting
        }
        KeyCode::Char('a') if key.modifiers.contains(KeyModifiers::CONTROL) => app.select_all(),
        KeyCode::Char('x') => app.clear_selection(),

        // Actions
        KeyCode::Char('i') => {
            if app.tab == Tab::Bundles {
                app.request_bundle_install(db);
            } else {
                app.request_install();
            }
        }
        KeyCode::Char('a') if app.tab == Tab::Bundles => {
            app.track_bundle_tools(db); // Add missing bundle tools to Available
        }
        KeyCode::Char('D') => app.request_uninstall(), // Shift+d for uninstall (safer)
        KeyCode::Char('u') => app.request_update(),    // Update tools with available updates

        // Details popup (for narrow terminals or quick view)
        // For Discover tab, open README popup
        KeyCode::Enter => {
            if app.tab == Tab::Discover {
                // Open README popup with markdown rendering
                if let Some(result) = app.selected_discover() {
                    let name = result.name.clone();
                    let url = result.url.clone();
                    app.open_readme(name, url.as_deref());
                }
            } else {
                app.toggle_details_popup();
            }
        }

        // Help
        KeyCode::Char('?') => app.toggle_help(),

        // Theme cycling
        KeyCode::Char('t') => app.cycle_theme(),

        // Config menu
        KeyCode::Char('c') => app.open_config_menu(),

        // Undo/redo
        KeyCode::Char('z') if key.modifiers.contains(KeyModifiers::CONTROL) => app.undo(),
        KeyCode::Char('y') if key.modifiers.contains(KeyModifiers::CONTROL) => app.redo(),

        // Refresh (check for updates on Updates tab)
        KeyCode::Char('r') => {
            if app.tab == Tab::Updates {
                // Schedule background operation (main loop will show loading state)
                app.schedule_op(super::app::BackgroundOp::CheckUpdates { step: 0 });
            } else {
                app.refresh_tools(db);
            }
        }

        _ => {}
    }
}

fn handle_search_mode(app: &mut App, key: KeyEvent, _db: &Database) {
    // Discover tab has different search behavior
    if app.tab == Tab::Discover {
        match key.code {
            KeyCode::Esc => {
                app.exit_search();
            }
            KeyCode::Enter => {
                // Save search to history and trigger external search
                app.save_discover_search_to_history(_db);
                app.exit_search();
                app.start_discover_search();
            }
            KeyCode::Up => {
                // Navigate to previous (older) history entry
                app.discover_history_up();
            }
            KeyCode::Down => {
                // Navigate to next (newer) history entry
                app.discover_history_down();
            }
            KeyCode::Backspace => {
                app.discover_query.pop();
                app.discover_history_index = None; // Reset history when typing
            }
            KeyCode::Char(c) => {
                app.discover_query.push(c);
                app.discover_history_index = None; // Reset history when typing
            }
            _ => {}
        }
    } else {
        // Standard search for other tabs (local filtering)
        match key.code {
            KeyCode::Esc => app.exit_search(),
            KeyCode::Enter => {
                app.exit_search();
            }
            KeyCode::Backspace => app.search_pop(),
            KeyCode::Char(c) => app.search_push(c),
            _ => {}
        }
    }
}

fn handle_command_mode(app: &mut App, key: KeyEvent, db: &Database) {
    match key.code {
        KeyCode::Esc => app.exit_command(),
        KeyCode::Enter => app.execute_command(db),
        KeyCode::Tab => app.autocomplete_command(),
        KeyCode::Up => app.command_history_prev(),
        KeyCode::Down => app.command_history_next(),
        KeyCode::Backspace => {
            if app.command.input.is_empty() {
                app.exit_command();
            } else {
                app.command_pop();
            }
        }
        KeyCode::Char(c) => app.command_push(c),
        _ => {}
    }
}

fn handle_mouse_event(app: &mut App, mouse: crossterm::event::MouseEvent, _db: &Database) {
    // Handle README popup mouse scrolling
    if app.has_readme_popup() {
        match mouse.kind {
            MouseEventKind::ScrollUp => app.scroll_readme_up(3),
            MouseEventKind::ScrollDown => app.scroll_readme_down(3),
            _ => {}
        }
        return;
    }

    // Handle config menu mouse events separately
    if app.show_config_menu {
        handle_config_menu_mouse(app, mouse);
        return;
    }

    // Don't handle mouse during overlays or special modes
    if app.show_help || app.show_details_popup || app.has_pending_action() || app.has_error_modal()
    {
        return;
    }

    // Only handle mouse in normal mode
    if app.input_mode != InputMode::Normal {
        return;
    }

    match mouse.kind {
        // Scroll up
        MouseEventKind::ScrollUp => {
            if app.tab == Tab::Bundles {
                app.select_prev_bundle();
            } else if app.tab == Tab::Discover {
                app.select_prev_discover();
            } else {
                app.select_prev();
            }
        }
        // Scroll down
        MouseEventKind::ScrollDown => {
            if app.tab == Tab::Bundles {
                app.select_next_bundle();
            } else if app.tab == Tab::Discover {
                app.select_next_discover();
            } else {
                app.select_next();
            }
        }
        // Left click
        MouseEventKind::Down(MouseButton::Left) => {
            let x = mouse.column;
            let y = mouse.row;

            // Check if clicking in tab area
            if app.is_in_tab_area(x, y) {
                app.click_tab(x, _db);
                return;
            }

            // Check if clicking in list area
            if let Some(row) = app.get_list_row(x, y) {
                app.click_list_item(row);
            }
        }
        // Right click to toggle selection
        MouseEventKind::Down(MouseButton::Right) => {
            let x = mouse.column;
            let y = mouse.row;

            if let Some(row) = app.get_list_row(x, y) {
                app.click_list_item(row);
                app.toggle_selection();
            }
        }
        _ => {}
    }
}

fn handle_config_menu_mouse(app: &mut App, mouse: crossterm::event::MouseEvent) {
    use super::app::{ConfigSection, config_menu_layout};
    use crate::config::TuiTheme;

    // Use stored popup area from renderer (avoids calculation mismatch)
    let Some((popup_x, popup_y, popup_width, popup_height)) = app.last_config_popup_area else {
        return; // Popup hasn't been rendered yet
    };

    // Content area is inside borders (top/bottom only, 2 chars total)
    let content_x = popup_x + 1;
    let content_y = popup_y + 1;
    let content_height = popup_height.saturating_sub(2) as usize;

    // Calculate total content lines using constants
    let custom_selected = app.config_menu.theme_selected == config_menu_layout::CUSTOM_THEME_INDEX;
    let total_lines = config_menu_layout::total_lines(custom_selected);
    let max_scroll = total_lines.saturating_sub(content_height);

    match mouse.kind {
        MouseEventKind::ScrollUp => {
            app.config_menu_scroll_up();
        }
        MouseEventKind::ScrollDown => {
            app.config_menu_scroll_down(total_lines, content_height);
        }
        MouseEventKind::Down(MouseButton::Left) => {
            let x = mouse.column;
            let y = mouse.row;

            // Check if click is inside popup content area
            if x >= content_x
                && x < popup_x + popup_width - 1
                && y >= content_y
                && y < popup_y + popup_height - 1
            {
                // Calculate which line was clicked (accounting for scroll)
                let clicked_line =
                    (y - content_y) as usize + app.config_menu.scroll_offset.min(max_scroll);

                // Use ConfigSection methods for line detection
                let (ai_start, ai_end) = ConfigSection::AiProvider.item_lines(custom_selected);
                let (theme_start, theme_end) = ConfigSection::Theme.item_lines(custom_selected);
                let (sources_start, sources_end) =
                    ConfigSection::Sources.item_lines(custom_selected);
                let (usage_start, usage_end) = ConfigSection::UsageMode.item_lines(custom_selected);
                let buttons_line = ConfigSection::Buttons.start_line(custom_selected);

                if clicked_line >= ai_start && clicked_line <= ai_end {
                    // AI Provider item clicked
                    app.config_menu.section = ConfigSection::AiProvider;
                    let item = clicked_line - ai_start;
                    if item < ConfigSection::AiProvider.item_count() {
                        app.config_menu.ai_selected = item;
                    }
                } else if clicked_line >= theme_start && clicked_line <= theme_end {
                    // Theme item clicked
                    app.config_menu.section = ConfigSection::Theme;
                    let item = clicked_line - theme_start;
                    if item < ConfigSection::Theme.item_count() {
                        app.config_menu.theme_selected = item;
                        let theme = TuiTheme::from_index(app.config_menu.theme_selected);
                        app.theme_variant = super::theme::ThemeVariant::from_config_theme(theme);
                    }
                } else if clicked_line >= sources_start && clicked_line <= sources_end {
                    // Sources item clicked
                    app.config_menu.section = ConfigSection::Sources;
                    let item = clicked_line - sources_start;
                    if item < ConfigSection::Sources.item_count() {
                        app.config_menu.source_focused = item;
                        app.config_menu_toggle_source();
                    }
                } else if clicked_line >= usage_start && clicked_line <= usage_end {
                    // Usage item clicked
                    app.config_menu.section = ConfigSection::UsageMode;
                    let item = clicked_line - usage_start;
                    if item < ConfigSection::UsageMode.item_count() {
                        app.config_menu.usage_selected = item;
                    }
                } else if clicked_line >= buttons_line {
                    // Buttons clicked
                    app.config_menu.section = ConfigSection::Buttons;
                    app.config_menu_select();
                }
            }
        }
        _ => {}
    }
}

/// Execute a confirmed action
fn execute_action(app: &mut App, action: &PendingAction, db: &Database) {
    match action {
        PendingAction::Install(tools) => {
            // For now, just show status - actual install requires shell commands
            // which should be done outside the TUI event loop
            let count = tools.len();
            if count == 1 {
                app.set_status(
                    format!(
                        "Install {} - use CLI: hoards install {}",
                        tools[0], tools[0]
                    ),
                    false,
                );
            } else {
                app.set_status(
                    format!("Install {} tools - use CLI for batch install", count),
                    false,
                );
            }
            app.clear_selection();
        }
        PendingAction::Uninstall(tools) => {
            // For now, just show status - actual uninstall requires shell commands
            let count = tools.len();
            if count == 1 {
                app.set_status(
                    format!(
                        "Uninstall {} - use CLI: hoards uninstall {}",
                        tools[0], tools[0]
                    ),
                    false,
                );
            } else {
                app.set_status(
                    format!("Uninstall {} tools - use CLI for batch uninstall", count),
                    false,
                );
            }
            app.clear_selection();
        }
        PendingAction::Update(tools) => {
            // For now, just show status - actual upgrade requires shell commands
            let count = tools.len();
            if count == 1 {
                app.set_status(
                    format!("Update {} - use CLI: hoards upgrade {}", tools[0], tools[0]),
                    false,
                );
            } else {
                app.set_status(
                    format!("Update {} tools - use CLI for batch upgrade", count),
                    false,
                );
            }
            app.clear_selection();
        }
    }
    // Refresh tools list after action
    app.refresh_tools(db);
}
