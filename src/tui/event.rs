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

    // Handle overlays (help and details popup)
    if app.show_help {
        if matches!(
            key.code,
            KeyCode::Esc | KeyCode::Char('?') | KeyCode::Char('q')
        ) {
            app.show_help = false;
        }
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
    }
}

fn handle_normal_mode(app: &mut App, key: KeyEvent, db: &Database) {
    match key.code {
        // Quit
        KeyCode::Char('q') => app.quit(),
        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => app.quit(),

        // Navigation - vim style (handles both tools and bundles)
        KeyCode::Char('j') | KeyCode::Down => {
            if app.tab == Tab::Bundles {
                app.select_next_bundle();
            } else {
                app.select_next();
            }
        }
        KeyCode::Char('k') | KeyCode::Up => {
            if app.tab == Tab::Bundles {
                app.select_prev_bundle();
            } else {
                app.select_prev();
            }
        }
        KeyCode::Char('g') => {
            if app.tab == Tab::Bundles {
                app.select_first_bundle();
            } else {
                app.select_first();
            }
        }
        KeyCode::Char('G') => {
            if app.tab == Tab::Bundles {
                app.select_last_bundle();
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

        // Search
        KeyCode::Char('/') => app.enter_search(),

        // Command palette (vim-style)
        KeyCode::Char(':') => app.enter_command(),

        // Clear search filter
        KeyCode::Esc => app.clear_search(),

        // Sort
        KeyCode::Char('s') => app.cycle_sort(),

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
        KeyCode::Char('D') => app.request_uninstall(), // Shift+d for uninstall (safer)
        KeyCode::Char('u') => app.request_update(),    // Update tools with available updates

        // Details popup (for narrow terminals or quick view)
        KeyCode::Enter => app.toggle_details_popup(),

        // Help
        KeyCode::Char('?') => app.toggle_help(),

        // Theme cycling
        KeyCode::Char('t') => app.cycle_theme(),

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
    match key.code {
        KeyCode::Esc => app.exit_search(),
        KeyCode::Enter => {
            // TODO: Execute search
            app.exit_search();
        }
        KeyCode::Backspace => app.search_pop(),
        KeyCode::Char(c) => app.search_push(c),
        _ => {}
    }
}

fn handle_command_mode(app: &mut App, key: KeyEvent, db: &Database) {
    match key.code {
        KeyCode::Esc => app.exit_command(),
        KeyCode::Enter => app.execute_command(db),
        KeyCode::Backspace => {
            if app.command_input.is_empty() {
                app.exit_command();
            } else {
                app.command_pop();
            }
        }
        KeyCode::Char(c) => app.command_push(c),
        _ => {}
    }
}

fn handle_mouse_event(app: &mut App, mouse: crossterm::event::MouseEvent, db: &Database) {
    // Don't handle mouse during overlays or special modes
    if app.show_help || app.show_details_popup || app.has_pending_action() {
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
            } else {
                app.select_prev();
            }
        }
        // Scroll down
        MouseEventKind::ScrollDown => {
            if app.tab == Tab::Bundles {
                app.select_next_bundle();
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
                app.click_tab(x, db);
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
