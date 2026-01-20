//! Overlay rendering
//!
//! This module handles rendering of overlay widgets like help, loading, and notifications.

use ratatui::{
    Frame,
    layout::{Alignment, Rect},
    style::Style,
    text::{Line, Span, Text},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
};

use super::super::app::{App, BackgroundOp, NotificationLevel, OutputLineType};
use super::super::theme::Theme;
use super::dialogs::centered_rect;
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::widgets::{Scrollbar, ScrollbarOrientation, ScrollbarState};

/// Render the help overlay
pub fn render_help_overlay(frame: &mut Frame, theme: &Theme, area: Rect) {
    // Center the help popup
    let popup_area = centered_rect(60, 80, area);

    let help_text = vec![
        Line::from(Span::styled(
            "Keyboard Shortcuts",
            Style::default().fg(theme.mauve).bold(),
        )),
        Line::from(""),
        Line::from(vec![Span::styled(
            "Navigation",
            Style::default().fg(theme.blue).bold(),
        )]),
        Line::from(vec![
            Span::styled("  j/↓      ", Style::default().fg(theme.yellow)),
            Span::styled("Move down", Style::default().fg(theme.text)),
        ]),
        Line::from(vec![
            Span::styled("  k/↑      ", Style::default().fg(theme.yellow)),
            Span::styled("Move up", Style::default().fg(theme.text)),
        ]),
        Line::from(vec![
            Span::styled("  g        ", Style::default().fg(theme.yellow)),
            Span::styled("Go to top", Style::default().fg(theme.text)),
        ]),
        Line::from(vec![
            Span::styled("  G        ", Style::default().fg(theme.yellow)),
            Span::styled("Go to bottom", Style::default().fg(theme.text)),
        ]),
        Line::from(vec![
            Span::styled("  n/N      ", Style::default().fg(theme.yellow)),
            Span::styled("Next/prev match (wrap)", Style::default().fg(theme.text)),
        ]),
        Line::from(vec![
            Span::styled("  f<char>  ", Style::default().fg(theme.peach)),
            Span::styled("Jump to letter", Style::default().fg(theme.text)),
        ]),
        Line::from(vec![
            Span::styled("  Ctrl+d   ", Style::default().fg(theme.yellow)),
            Span::styled("Page down", Style::default().fg(theme.text)),
        ]),
        Line::from(vec![
            Span::styled("  Ctrl+u   ", Style::default().fg(theme.yellow)),
            Span::styled("Page up", Style::default().fg(theme.text)),
        ]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "Tabs",
            Style::default().fg(theme.blue).bold(),
        )]),
        Line::from(vec![
            Span::styled("  1-4      ", Style::default().fg(theme.yellow)),
            Span::styled("Switch to tab", Style::default().fg(theme.text)),
        ]),
        Line::from(vec![
            Span::styled("  Tab/]    ", Style::default().fg(theme.yellow)),
            Span::styled("Next tab", Style::default().fg(theme.text)),
        ]),
        Line::from(vec![
            Span::styled("  S-Tab/[  ", Style::default().fg(theme.yellow)),
            Span::styled("Previous tab", Style::default().fg(theme.text)),
        ]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "Selection",
            Style::default().fg(theme.blue).bold(),
        )]),
        Line::from(vec![
            Span::styled("  Space    ", Style::default().fg(theme.yellow)),
            Span::styled("Toggle selection", Style::default().fg(theme.text)),
        ]),
        Line::from(vec![
            Span::styled("  Ctrl+a   ", Style::default().fg(theme.yellow)),
            Span::styled("Select all", Style::default().fg(theme.text)),
        ]),
        Line::from(vec![
            Span::styled("  x        ", Style::default().fg(theme.yellow)),
            Span::styled("Clear selection", Style::default().fg(theme.text)),
        ]),
        Line::from(vec![
            Span::styled("  *        ", Style::default().fg(theme.yellow)),
            Span::styled("Toggle favorite", Style::default().fg(theme.text)),
        ]),
        Line::from(vec![
            Span::styled("  F        ", Style::default().fg(theme.yellow)),
            Span::styled("Toggle favorites filter", Style::default().fg(theme.text)),
        ]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "Actions",
            Style::default().fg(theme.blue).bold(),
        )]),
        Line::from(vec![
            Span::styled("  i        ", Style::default().fg(theme.green)),
            Span::styled("Install tool(s)", Style::default().fg(theme.text)),
        ]),
        Line::from(vec![
            Span::styled("  D        ", Style::default().fg(theme.red)),
            Span::styled("Uninstall tool(s)", Style::default().fg(theme.text)),
        ]),
        Line::from(vec![
            Span::styled("  u        ", Style::default().fg(theme.yellow)),
            Span::styled("Update tool(s)", Style::default().fg(theme.text)),
        ]),
        Line::from(vec![
            Span::styled("  Enter    ", Style::default().fg(theme.yellow)),
            Span::styled("Show details popup", Style::default().fg(theme.text)),
        ]),
        Line::from(vec![
            Span::styled("  /        ", Style::default().fg(theme.yellow)),
            Span::styled("Search/filter tools", Style::default().fg(theme.text)),
        ]),
        Line::from(vec![
            Span::styled("  :        ", Style::default().fg(theme.mauve)),
            Span::styled(
                "Command palette (vim-style)",
                Style::default().fg(theme.text),
            ),
        ]),
        Line::from(vec![
            Span::styled("  s        ", Style::default().fg(theme.yellow)),
            Span::styled(
                "Cycle sort (name/usage/recent)",
                Style::default().fg(theme.text),
            ),
        ]),
        Line::from(vec![
            Span::styled("  Esc      ", Style::default().fg(theme.yellow)),
            Span::styled("Clear search filter", Style::default().fg(theme.text)),
        ]),
        Line::from(vec![
            Span::styled("  r        ", Style::default().fg(theme.yellow)),
            Span::styled("Refresh list", Style::default().fg(theme.text)),
        ]),
        Line::from(vec![
            Span::styled("  t        ", Style::default().fg(theme.teal)),
            Span::styled("Cycle theme", Style::default().fg(theme.text)),
        ]),
        Line::from(vec![
            Span::styled("  Ctrl+z   ", Style::default().fg(theme.peach)),
            Span::styled("Undo", Style::default().fg(theme.text)),
        ]),
        Line::from(vec![
            Span::styled("  Ctrl+y   ", Style::default().fg(theme.peach)),
            Span::styled("Redo", Style::default().fg(theme.text)),
        ]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "Mouse",
            Style::default().fg(theme.blue).bold(),
        )]),
        Line::from(vec![
            Span::styled("  Click    ", Style::default().fg(theme.green)),
            Span::styled("Select item / switch tab", Style::default().fg(theme.text)),
        ]),
        Line::from(vec![
            Span::styled("  R-Click  ", Style::default().fg(theme.green)),
            Span::styled("Toggle selection", Style::default().fg(theme.text)),
        ]),
        Line::from(vec![
            Span::styled("  Scroll   ", Style::default().fg(theme.green)),
            Span::styled("Navigate list", Style::default().fg(theme.text)),
        ]),
        Line::from(vec![
            Span::styled("  ?        ", Style::default().fg(theme.yellow)),
            Span::styled("Toggle help", Style::default().fg(theme.text)),
        ]),
        Line::from(vec![
            Span::styled("  q        ", Style::default().fg(theme.yellow)),
            Span::styled("Quit", Style::default().fg(theme.text)),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "Press ? or Esc to close",
            Style::default().fg(theme.subtext0),
        )),
    ];

    let help = Paragraph::new(help_text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme.mauve))
                .title(Span::styled(
                    " Help ",
                    Style::default().fg(theme.mauve).bold(),
                ))
                .style(Style::default().bg(theme.base)),
        )
        .wrap(Wrap { trim: true });

    frame.render_widget(Clear, popup_area);
    frame.render_widget(help, popup_area);
}

/// Render the loading overlay
pub fn render_loading_overlay(frame: &mut Frame, app: &App, theme: &Theme, area: Rect) {
    // Check if this is an install/update operation with output
    let is_install_op = matches!(
        app.background_op,
        Some(BackgroundOp::ExecuteInstall { .. } | BackgroundOp::ExecuteUpdate { .. })
    );

    // Use larger area for install operations with output window
    let popup_area = if is_install_op && !app.install_output.is_empty() {
        centered_rect(70, 60, area)
    } else {
        centered_rect(50, 30, area)
    };

    let title = app
        .background_op
        .as_ref()
        .map(|op| op.title())
        .unwrap_or("Working");

    let progress = &app.loading_progress;

    // Build progress bar
    let bar_width = 30;
    let filled = if progress.total_steps > 0 {
        (progress.current_step * bar_width) / progress.total_steps
    } else {
        0
    };
    let empty = bar_width - filled;
    let progress_bar = format!(
        "[{}{}] {}/{}",
        "█".repeat(filled),
        "░".repeat(empty),
        progress.current_step,
        progress.total_steps
    );

    // For install operations with output, render split layout
    if is_install_op && !app.install_output.is_empty() {
        render_install_overlay_with_output(frame, app, theme, popup_area, title, &progress_bar);
        return;
    }

    // Standard loading overlay (no output)
    let mut lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            &progress.step_name,
            Style::default().fg(theme.blue).bold(),
        )),
        Line::from(""),
        Line::from(Span::styled(
            progress_bar,
            Style::default().fg(theme.yellow),
        )),
        Line::from(""),
    ];

    // Show found count if any
    if progress.found_count > 0 {
        lines.push(Line::from(vec![
            Span::styled("Found: ", Style::default().fg(theme.subtext0)),
            Span::styled(
                format!("{} update(s)", progress.found_count),
                Style::default().fg(theme.green),
            ),
        ]));
        lines.push(Line::from(""));
    }

    lines.push(Line::from(Span::styled(
        "Please wait...",
        Style::default().fg(theme.subtext0),
    )));

    let content = Text::from(lines);

    let popup = Paragraph::new(content)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme.yellow))
                .title(Span::styled(
                    format!(" {} ", title),
                    Style::default().fg(theme.yellow).bold(),
                ))
                .style(Style::default().bg(theme.base)),
        )
        .alignment(Alignment::Center);

    frame.render_widget(Clear, popup_area);
    frame.render_widget(popup, popup_area);
}

/// Render install overlay with live output window
fn render_install_overlay_with_output(
    frame: &mut Frame,
    app: &App,
    theme: &Theme,
    popup_area: Rect,
    title: &str,
    progress_bar: &str,
) {
    frame.render_widget(Clear, popup_area);

    // Split into header (progress) and output sections
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(5), // Header with progress
            Constraint::Min(5),    // Output window
            Constraint::Length(2), // Footer with hints
        ])
        .split(popup_area);

    let progress = &app.loading_progress;

    // Header - progress info
    let header_lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            &progress.step_name,
            Style::default().fg(theme.blue).bold(),
        )),
        Line::from(""),
        Line::from(Span::styled(
            progress_bar,
            Style::default().fg(theme.yellow),
        )),
    ];

    let header = Paragraph::new(header_lines)
        .block(
            Block::default()
                .borders(Borders::TOP | Borders::LEFT | Borders::RIGHT)
                .border_style(Style::default().fg(theme.yellow))
                .title(Span::styled(
                    format!(" {} ", title),
                    Style::default().fg(theme.yellow).bold(),
                ))
                .style(Style::default().bg(theme.base)),
        )
        .alignment(Alignment::Center);
    frame.render_widget(header, chunks[0]);

    // Output window - scrollable
    // Note: Many tools (cargo, etc.) output progress to stderr, so we don't color
    // all stderr red. Instead, we detect actual error lines by content.
    let output_lines: Vec<Line> = app
        .install_output
        .iter()
        .map(|line| {
            let content_lower = line.content.to_lowercase();
            let is_error_line = content_lower.contains("error")
                || content_lower.contains("failed")
                || content_lower.contains("cannot")
                || content_lower.contains("not found");

            let color = match line.line_type {
                OutputLineType::Stdout => theme.text,
                OutputLineType::Stderr if is_error_line => theme.red,
                OutputLineType::Stderr => theme.subtext0, // Dimmed for normal stderr
                OutputLineType::Status => theme.blue,
            };
            Line::from(Span::styled(&line.content, Style::default().fg(color)))
        })
        .collect();

    let visible_height = chunks[1].height.saturating_sub(2) as usize; // Account for borders
    let max_scroll = app.install_output.len().saturating_sub(visible_height);
    let scroll_offset = app.install_output_scroll.min(max_scroll);

    let output = Paragraph::new(output_lines)
        .block(
            Block::default()
                .borders(Borders::LEFT | Borders::RIGHT)
                .border_style(Style::default().fg(theme.surface1))
                .style(Style::default().bg(theme.surface0)),
        )
        .scroll((scroll_offset as u16, 0));

    frame.render_widget(output, chunks[1]);

    // Scrollbar for output
    if app.install_output.len() > visible_height {
        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(Some("▲"))
            .end_symbol(Some("▼"))
            .track_symbol(Some("│"))
            .thumb_symbol("█")
            .track_style(Style::default().fg(theme.surface0))
            .thumb_style(Style::default().fg(theme.blue));

        let mut scrollbar_state =
            ScrollbarState::new(app.install_output.len()).position(scroll_offset);

        let scrollbar_area = Rect {
            x: chunks[1].x + chunks[1].width - 2,
            y: chunks[1].y + 1,
            width: 1,
            height: chunks[1].height.saturating_sub(2),
        };
        frame.render_stateful_widget(scrollbar, scrollbar_area, &mut scrollbar_state);
    }

    // Footer - keyboard hints
    let footer = Paragraph::new(Line::from(vec![
        Span::styled(" j/k ", Style::default().fg(theme.blue)),
        Span::styled("scroll ", Style::default().fg(theme.subtext0)),
        Span::styled(" Esc ", Style::default().fg(theme.red)),
        Span::styled("cancel", Style::default().fg(theme.subtext0)),
    ]))
    .block(
        Block::default()
            .borders(Borders::BOTTOM | Borders::LEFT | Borders::RIGHT)
            .border_style(Style::default().fg(theme.yellow))
            .style(Style::default().bg(theme.base)),
    )
    .alignment(Alignment::Center);
    frame.render_widget(footer, chunks[2]);
}

/// Render toast notifications in top-right corner
pub fn render_notifications(frame: &mut Frame, app: &App, theme: &Theme, area: Rect) {
    if app.notifications.is_empty() {
        return;
    }

    // Stack notifications from top-right
    // Use 60% of screen width for notifications, min 40, max 80
    let max_width = (area.width * 60 / 100)
        .clamp(40, 80)
        .min(area.width.saturating_sub(4));
    let mut y_offset = 1u16;

    for notification in &app.notifications {
        let (border_color, icon) = match notification.level {
            NotificationLevel::Info => (theme.blue, "ℹ"),
            NotificationLevel::Warning => (theme.yellow, "⚠"),
            NotificationLevel::Error => (theme.red, "✗"),
        };

        // Calculate wrapped lines
        let inner_width = (max_width as usize).saturating_sub(4); // borders + padding
        let text = &notification.text;

        // Word wrap the text
        let mut lines: Vec<Line> = Vec::new();
        let mut current_line = format!("{} ", icon);

        for word in text.split_whitespace() {
            if current_line.len() + word.len() + 1 > inner_width {
                lines.push(Line::from(Span::styled(
                    current_line.clone(),
                    Style::default().fg(if lines.is_empty() {
                        border_color
                    } else {
                        theme.text
                    }),
                )));
                current_line = format!("  {}", word); // indent continuation
            } else {
                if !current_line.ends_with(' ') && !current_line.is_empty() {
                    current_line.push(' ');
                }
                current_line.push_str(word);
            }
        }
        if !current_line.is_empty() {
            lines.push(Line::from(Span::styled(
                current_line,
                Style::default().fg(if lines.is_empty() {
                    border_color
                } else {
                    theme.text
                }),
            )));
        }

        // Calculate height needed (lines + 2 for borders)
        let height = (lines.len() as u16 + 2).min(area.height.saturating_sub(y_offset));

        if y_offset + height > area.height {
            break; // No more room
        }

        let toast_area = Rect {
            x: area.width.saturating_sub(max_width + 2),
            y: y_offset,
            width: max_width,
            height,
        };

        let content = Paragraph::new(lines)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(border_color))
                    .style(Style::default().bg(theme.surface0)),
            )
            .wrap(ratatui::widgets::Wrap { trim: false });

        frame.render_widget(Clear, toast_area);
        frame.render_widget(content, toast_area);

        y_offset += height + 1; // toast height + gap
    }
}
