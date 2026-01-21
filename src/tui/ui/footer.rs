//! Footer rendering
//!
//! This module handles rendering of the footer bar with mode-specific content.

use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::Style,
    text::{Line, Span},
    widgets::Paragraph,
};

use super::super::app::{App, InputMode, Tab};
use super::super::theme::Theme;
use super::helpers::format_relative_time;

/// Build the right-side status indicators (AI, GitHub, sync time, version)
fn build_footer_right_status(app: &App, theme: &Theme) -> (Vec<Span<'static>>, usize) {
    let ai_color = if app.ai_available {
        theme.green
    } else {
        theme.surface1
    };
    let gh_color = if app.gh_available {
        theme.green
    } else {
        theme.surface1
    };
    let version = env!("CARGO_PKG_VERSION");

    let sync_str = app
        .last_sync
        .as_ref()
        .map(|dt| format!("⟳ {}", format_relative_time(dt)))
        .unwrap_or_default();
    let sync_len = if sync_str.is_empty() {
        0
    } else {
        sync_str.chars().count() + 1
    };

    let mut spans = vec![
        Span::styled("🤖", Style::default().fg(ai_color)),
        Span::styled(" ", Style::default()),
        Span::styled("\u{f09b}", Style::default().fg(gh_color)),
        Span::styled("  ", Style::default()),
    ];

    if !sync_str.is_empty() {
        spans.push(Span::styled(
            sync_str,
            Style::default().fg(theme.subtext0).dim(),
        ));
        spans.push(Span::styled(" ", Style::default()));
    }

    spans.push(Span::styled(
        format!("v{}", version),
        Style::default().fg(theme.subtext0),
    ));
    spans.push(Span::styled(" ", Style::default()));

    let width = 2 + 1 + 1 + 2 + sync_len + 1 + version.len() + 1;
    (spans, width)
}

/// Build footer content for Normal mode
fn build_normal_mode_footer(app: &App, theme: &Theme) -> Vec<Span<'static>> {
    // Discover tab has different shortcuts
    if app.tab == Tab::Discover {
        return build_discover_footer(app, theme);
    }

    // Tab-specific hints
    let mut spans = vec![
        Span::styled(" j/k", Style::default().fg(theme.blue)),
        Span::styled(" nav ", Style::default().fg(theme.subtext0)),
    ];

    match app.tab {
        Tab::Installed => {
            spans.extend([
                Span::styled(" Space", Style::default().fg(theme.blue)),
                Span::styled(" select ", Style::default().fg(theme.subtext0)),
                Span::styled(" D", Style::default().fg(theme.red)),
                Span::styled(" uninstall ", Style::default().fg(theme.subtext0)),
                Span::styled(" u", Style::default().fg(theme.yellow)),
                Span::styled(" update ", Style::default().fg(theme.subtext0)),
                Span::styled(" Ctrl+r", Style::default().fg(theme.blue)),
                Span::styled(" refresh ", Style::default().fg(theme.subtext0)),
            ]);
        }
        Tab::Available => {
            spans.extend([
                Span::styled(" Space", Style::default().fg(theme.blue)),
                Span::styled(" select ", Style::default().fg(theme.subtext0)),
                Span::styled(" i", Style::default().fg(theme.green)),
                Span::styled(" install ", Style::default().fg(theme.subtext0)),
                Span::styled(" Ctrl+r", Style::default().fg(theme.blue)),
                Span::styled(" refresh ", Style::default().fg(theme.subtext0)),
            ]);
        }
        Tab::Updates => {
            spans.extend([
                Span::styled(" Space", Style::default().fg(theme.blue)),
                Span::styled(" select ", Style::default().fg(theme.subtext0)),
                Span::styled(" u", Style::default().fg(theme.yellow)),
                Span::styled(" update ", Style::default().fg(theme.subtext0)),
                Span::styled(" U", Style::default().fg(theme.yellow).bold()),
                Span::styled(" update all ", Style::default().fg(theme.subtext0)),
                Span::styled(" Ctrl+r", Style::default().fg(theme.blue)),
                Span::styled(" check ", Style::default().fg(theme.subtext0)),
            ]);
        }
        Tab::Bundles => {
            spans.extend([
                Span::styled(" Space", Style::default().fg(theme.blue)),
                Span::styled(" select ", Style::default().fg(theme.subtext0)),
                Span::styled(" i", Style::default().fg(theme.green)),
                Span::styled(" install ", Style::default().fg(theme.subtext0)),
                Span::styled(" D", Style::default().fg(theme.red)),
                Span::styled(" delete ", Style::default().fg(theme.subtext0)),
            ]);
        }
        Tab::Discover => unreachable!(), // Handled above
    }

    spans.extend([
        Span::styled(" ?", Style::default().fg(theme.blue)),
        Span::styled(" help", Style::default().fg(theme.subtext0)),
    ]);

    if app.selection_count() > 0 {
        spans.push(Span::styled(" │ ", Style::default().fg(theme.surface1)));
        spans.push(Span::styled(
            format!("{} selected", app.selection_count()),
            Style::default().fg(theme.blue),
        ));
    } else if !app.search_query.is_empty()
        || app.source_filter.is_some()
        || app.label_filter.is_some()
        || app.favorites_only
    {
        spans.extend(build_filter_status(app, theme));
    }

    spans
}

/// Build footer content for Discover tab
fn build_discover_footer(app: &App, theme: &Theme) -> Vec<Span<'static>> {
    // Build F-key range based on available sources
    let source_count = app.get_available_discover_sources().len();
    let fkey_label = if source_count == 0 {
        String::new()
    } else if source_count == 1 {
        " F1".to_string()
    } else {
        format!(" F1-F{}", source_count)
    };

    let mut spans = vec![
        Span::styled(" j/k", Style::default().fg(theme.blue)),
        Span::styled(" nav ", Style::default().fg(theme.subtext0)),
        Span::styled(" /", Style::default().fg(theme.yellow)),
        Span::styled(" search ", Style::default().fg(theme.subtext0)),
        Span::styled(" A", Style::default().fg(theme.green)),
        Span::styled(" AI ", Style::default().fg(theme.subtext0)),
    ];

    if !fkey_label.is_empty() {
        spans.push(Span::styled(fkey_label, Style::default().fg(theme.blue)));
        spans.push(Span::styled(
            " filters ",
            Style::default().fg(theme.subtext0),
        ));
    }

    spans.extend([
        Span::styled(" i", Style::default().fg(theme.green)),
        Span::styled(" install ", Style::default().fg(theme.subtext0)),
        Span::styled(" Enter", Style::default().fg(theme.blue)),
        Span::styled(" readme ", Style::default().fg(theme.subtext0)),
        Span::styled(" ?", Style::default().fg(theme.blue)),
        Span::styled(" help", Style::default().fg(theme.subtext0)),
    ]);

    // Show current filter state
    let enabled_count = app.discover_source_filters.len();
    let total_sources = app.get_available_discover_sources().len();

    spans.push(Span::styled(" │ ", Style::default().fg(theme.surface1)));

    if app.discover_ai_enabled {
        spans.push(Span::styled(
            "🤖 AI",
            Style::default().fg(theme.green).bold(),
        ));
    } else {
        spans.push(Span::styled(
            format!("{}/{} sources", enabled_count, total_sources),
            Style::default().fg(theme.blue),
        ));
    }

    spans
}

/// Build filter status spans (called from normal mode footer)
fn build_filter_status(app: &App, theme: &Theme) -> Vec<Span<'static>> {
    let mut spans = vec![Span::styled(" │ ", Style::default().fg(theme.surface1))];

    let filtered = app.tools.len();
    let total = app.all_tools.len();
    spans.push(Span::styled(
        format!("{}/{} ", filtered, total),
        Style::default().fg(theme.blue),
    ));

    if app.favorites_only {
        spans.push(Span::styled("★", Style::default().fg(theme.yellow)));
        if app.source_filter.is_some() || app.label_filter.is_some() || !app.search_query.is_empty()
        {
            spans.push(Span::styled(" ", Style::default()));
        }
    }
    if let Some(ref source) = app.source_filter {
        spans.push(Span::styled("src:", Style::default().fg(theme.mauve)));
        spans.push(Span::styled(
            source.clone(),
            Style::default().fg(theme.text),
        ));
        if app.label_filter.is_some() || !app.search_query.is_empty() {
            spans.push(Span::styled(" ", Style::default()));
        }
    }
    if let Some(ref label) = app.label_filter {
        spans.push(Span::styled("label:", Style::default().fg(theme.teal)));
        spans.push(Span::styled(label.clone(), Style::default().fg(theme.text)));
        if !app.search_query.is_empty() {
            spans.push(Span::styled(" ", Style::default()));
        }
    }
    if !app.search_query.is_empty() {
        spans.push(Span::styled("filter:", Style::default().fg(theme.yellow)));
        spans.push(Span::styled(
            app.search_query.clone(),
            Style::default().fg(theme.text),
        ));
    }

    spans
}

/// Build footer content for Search mode
fn build_search_mode_footer(app: &App, theme: &Theme) -> Vec<Span<'static>> {
    vec![
        Span::styled(" Search: ", Style::default().fg(theme.yellow)),
        Span::styled(app.search_query.clone(), Style::default().fg(theme.text)),
        Span::styled("│", Style::default().fg(theme.blue)),
        Span::styled("  Enter", Style::default().fg(theme.blue)),
        Span::styled(" apply ", Style::default().fg(theme.subtext0)),
        Span::styled(" Esc", Style::default().fg(theme.blue)),
        Span::styled(" cancel", Style::default().fg(theme.subtext0)),
    ]
}

/// Build footer content for Command mode
fn build_command_mode_footer(app: &App, theme: &Theme) -> Vec<Span<'static>> {
    let mut spans = vec![
        Span::styled(" :", Style::default().fg(theme.mauve)),
        Span::styled(app.command.input.clone(), Style::default().fg(theme.text)),
        Span::styled("│", Style::default().fg(theme.blue)),
    ];

    let suggestions = app.get_command_suggestions();
    if !suggestions.is_empty() {
        spans.push(Span::styled("  ", Style::default()));
        for (i, (cmd, desc)) in suggestions.iter().take(3).enumerate() {
            if i > 0 {
                spans.push(Span::styled(" │ ", Style::default().fg(theme.surface1)));
            }
            spans.push(Span::styled(
                cmd.to_string(),
                Style::default().fg(theme.green),
            ));
            spans.push(Span::styled(
                format!(" {}", desc.split('-').next().unwrap_or("").trim()),
                Style::default().fg(theme.subtext0).dim(),
            ));
        }
        spans.push(Span::styled("  Tab", Style::default().fg(theme.blue)));
        spans.push(Span::styled(
            " complete",
            Style::default().fg(theme.subtext0),
        ));
    } else if app.command.input.is_empty() {
        spans.push(Span::styled("  Enter", Style::default().fg(theme.blue)));
        spans.push(Span::styled(
            " execute ",
            Style::default().fg(theme.subtext0),
        ));
        spans.push(Span::styled(" Esc", Style::default().fg(theme.blue)));
        spans.push(Span::styled(
            " cancel ",
            Style::default().fg(theme.subtext0),
        ));
        spans.push(Span::styled(
            " (h for help)",
            Style::default().fg(theme.subtext0).dim(),
        ));
    }

    spans
}

/// Build footer content for JumpToLetter mode
fn build_jump_mode_footer(theme: &Theme) -> Vec<Span<'static>> {
    vec![
        Span::styled(" f", Style::default().fg(theme.peach).bold()),
        Span::styled(
            "  Type a letter to jump to first tool starting with it...".to_string(),
            Style::default().fg(theme.text),
        ),
        Span::styled("  Esc", Style::default().fg(theme.blue)),
        Span::styled(" cancel", Style::default().fg(theme.subtext0)),
    ]
}

/// Build footer content for Password mode (sudo)
fn build_password_mode_footer(app: &App, theme: &Theme) -> Vec<Span<'static>> {
    // Show masked password input
    let masked = "*".repeat(app.password_input.len());
    vec![
        Span::styled(" 🔒 Password: ", Style::default().fg(theme.yellow).bold()),
        Span::styled(masked, Style::default().fg(theme.text)),
        Span::styled("█", Style::default().fg(theme.blue)), // Cursor
        Span::styled("  Enter", Style::default().fg(theme.blue)),
        Span::styled(" confirm  ", Style::default().fg(theme.subtext0)),
        Span::styled("Esc", Style::default().fg(theme.blue)),
        Span::styled(" cancel", Style::default().fg(theme.subtext0)),
    ]
}

/// Render the footer bar
pub fn render_footer(frame: &mut Frame, app: &App, theme: &Theme, area: Rect) {
    let (right_status, right_width) = build_footer_right_status(app, theme);

    // Show status message if present (takes priority)
    if let Some(status) = &app.status_message {
        let color = if status.is_error {
            theme.red
        } else {
            theme.green
        };
        let left_content = vec![
            Span::styled(" ", Style::default()),
            Span::styled(status.text.clone(), Style::default().fg(color)),
        ];

        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Min(0), Constraint::Length(right_width as u16)])
            .split(area);

        frame.render_widget(
            Paragraph::new(Line::from(left_content)).style(Style::default().bg(theme.surface0)),
            chunks[0],
        );
        frame.render_widget(
            Paragraph::new(Line::from(right_status)).style(Style::default().bg(theme.surface0)),
            chunks[1],
        );
        return;
    }

    let mode_text = match app.input_mode {
        InputMode::Normal => build_normal_mode_footer(app, theme),
        InputMode::Search => build_search_mode_footer(app, theme),
        InputMode::Command => build_command_mode_footer(app, theme),
        InputMode::JumpToLetter => build_jump_mode_footer(theme),
        InputMode::Password => build_password_mode_footer(app, theme),
    };

    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(0), Constraint::Length(right_width as u16)])
        .split(area);

    frame.render_widget(
        Paragraph::new(Line::from(mode_text)).style(Style::default().bg(theme.surface0)),
        chunks[0],
    );
    frame.render_widget(
        Paragraph::new(Line::from(right_status)).style(Style::default().bg(theme.surface0)),
        chunks[1],
    );
}
