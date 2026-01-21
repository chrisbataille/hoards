//! UI rendering for the TUI
//!
//! This module handles all rendering for the TUI application.

mod bundles;
mod config;
mod dialogs;
mod discover;
mod footer;
mod helpers;
mod overlays;
mod tool_list;

use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, Tabs},
};

use super::app::{App, Tab};
use super::theme::Theme;
use crate::db::Database;

/// Main render function
pub fn render(frame: &mut Frame, app: &mut App, db: &Database) {
    let area = frame.area();
    let theme = app.theme();

    // Main layout: header, body, footer
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Header with tabs
            Constraint::Min(0),    // Body
            Constraint::Length(1), // Footer
        ])
        .split(area);

    // Store areas for mouse interaction
    app.set_tab_area(chunks[0].x, chunks[0].y, chunks[0].width, chunks[0].height);

    render_header(frame, app, &theme, chunks[0]);
    render_body(frame, app, db, &theme, chunks[1]);
    footer::render_footer(frame, app, &theme, chunks[2]);

    // Render overlays (in order of priority)
    if app.show_help {
        overlays::render_help_overlay(frame, &theme, area);
    }

    if app.show_config_menu {
        config::render_config_menu(frame, app, &theme, area);
    }

    if app.show_details_popup {
        dialogs::render_details_popup(frame, app, db, &theme, area);
    }

    // Label filter popup
    if app.show_label_filter_popup {
        dialogs::render_label_filter_popup(frame, app, db, &theme, area);
    }

    // Label edit popup
    if app.show_label_edit_popup {
        dialogs::render_label_edit_popup(frame, app, &theme, area);
    }

    // README popup
    if app.has_readme_popup() {
        dialogs::render_readme_popup(frame, app, &theme, area);
    }

    // Confirmation dialog takes high priority
    if app.has_pending_action() {
        dialogs::render_confirmation_dialog(frame, app, &theme, area);
    }

    // Error modal blocks all input
    if app.has_error_modal() {
        dialogs::render_error_modal(frame, app, &theme, area);
    }

    // Loading overlay takes absolute highest priority
    if app.has_background_op() {
        overlays::render_loading_overlay(frame, app, &theme, area);
    }

    // Toast notifications always on top (but don't block input)
    overlays::render_notifications(frame, app, &theme, area);
}

fn render_header(frame: &mut Frame, app: &App, theme: &Theme, area: Rect) {
    let titles: Vec<Line> = Tab::all()
        .iter()
        .map(|t| {
            let style = if *t == app.tab {
                Style::default().fg(theme.blue).bold()
            } else {
                Style::default().fg(theme.subtext0)
            };
            Line::from(Span::styled(format!(" {} ", t.title()), style))
        })
        .collect();

    let tabs = Tabs::new(titles)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme.surface1))
                .title(Span::styled(
                    " hoard ",
                    Style::default().fg(theme.mauve).bold(),
                )),
        )
        .highlight_style(Style::default().fg(theme.blue))
        .padding("", "") // No extra padding - we include spaces in titles
        .select(app.tab.index());

    frame.render_widget(tabs, area);
}

fn render_body(frame: &mut Frame, app: &mut App, db: &Database, theme: &Theme, area: Rect) {
    // Responsive layout: side-by-side for wide terminals, stacked for narrow
    let min_width_for_split = 80;

    // Bundles tab has its own rendering
    if app.tab == Tab::Bundles {
        if area.width >= min_width_for_split {
            let chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
                .split(area);

            // Store list area for mouse interaction
            app.set_list_area(chunks[0].x, chunks[0].y, chunks[0].width, chunks[0].height);
            bundles::render_bundle_list(frame, app, theme, chunks[0]);
            bundles::render_bundle_details(frame, app, db, theme, chunks[1]);
        } else {
            app.set_list_area(area.x, area.y, area.width, area.height);
            bundles::render_bundle_list(frame, app, theme, area);
        }
        return;
    }

    // Discover tab has its own rendering (needs mutable access for list area)
    if app.tab == Tab::Discover {
        discover::render_discover_tab(frame, app, theme, area);
        return;
    }

    if area.width >= min_width_for_split {
        // Wide terminal: side-by-side layout
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
            .split(area);

        // Store list area for mouse interaction
        app.set_list_area(chunks[0].x, chunks[0].y, chunks[0].width, chunks[0].height);
        tool_list::render_tool_list(frame, app, theme, chunks[0]);
        tool_list::render_details(frame, app, db, theme, chunks[1]);
    } else {
        // Narrow terminal: list only (details on Enter in future)
        app.set_list_area(area.x, area.y, area.width, area.height);
        tool_list::render_tool_list(frame, app, theme, area);
    }
}
