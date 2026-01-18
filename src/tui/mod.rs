//! Terminal User Interface for Hoard
//!
//! This module provides a full-featured TUI built with Ratatui.

mod app;
mod event;
pub mod theme;
mod ui;

pub use app::{App, DiscoverResult, DiscoverSource, InstallOption};
pub use theme::{Theme, ThemeVariant};

use anyhow::Result;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, prelude::CrosstermBackend};
use std::io::{self, Stdout};

use crate::db::Database;

type Tui = Terminal<CrosstermBackend<Stdout>>;

/// Initialize the terminal for TUI mode
fn init_terminal() -> Result<Tui> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let terminal = Terminal::new(backend)?;
    Ok(terminal)
}

/// Restore the terminal to its original state
fn restore_terminal(terminal: &mut Tui) -> Result<()> {
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;
    Ok(())
}

/// Run the TUI application
pub fn run(db: &Database) -> Result<()> {
    let mut terminal = init_terminal()?;
    let mut app = App::new(db)?;

    let result = run_app(&mut terminal, &mut app, db);

    // Always restore terminal, even if app errored
    restore_terminal(&mut terminal)?;

    result
}

fn run_app(terminal: &mut Tui, app: &mut App, db: &Database) -> Result<()> {
    while app.running {
        terminal.draw(|frame| ui::render(frame, app, db))?;
        event::handle_events(app, db)?;

        // Clean up expired notifications
        app.tick_notifications();

        // Execute background operations step by step with loading indicator
        while app.has_background_op() {
            // Redraw to show current progress
            terminal.draw(|frame| ui::render(frame, app, db))?;
            // Execute one step (returns true if more steps remain)
            if !app.execute_background_step(db) {
                break;
            }
        }
    }
    Ok(())
}
