//! UI rendering for the TUI

use chrono::{DateTime, Utc};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Tabs, Wrap},
};

use super::app::{App, InputMode, Tab};
use super::theme::Theme;
use crate::db::Database;
use crate::icons::source_icon;

/// Generate a sparkline string from usage data
/// Uses Unicode block elements: ▁▂▃▄▅▆▇█
fn sparkline(data: &[i64]) -> String {
    if data.is_empty() || data.iter().all(|&x| x == 0) {
        return "·······".to_string(); // No data indicator
    }

    let max = *data.iter().max().unwrap_or(&1).max(&1);
    let blocks = ['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];

    data.iter()
        .map(|&value| {
            if value == 0 {
                ' '
            } else {
                // Scale to 0-7 range
                let idx = ((value as f64 / max as f64) * 7.0).round() as usize;
                blocks[idx.min(7)]
            }
        })
        .collect()
}

/// Determine health status based on usage recency
/// Returns (indicator, color) tuple
fn health_indicator(
    last_used: Option<&str>,
    use_count: i64,
    theme: &Theme,
) -> (&'static str, Color) {
    // Parse last_used timestamp
    let days_since_use = last_used
        .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
        .map(|dt| {
            let now = Utc::now();
            let used = dt.with_timezone(&Utc);
            (now - used).num_days()
        });

    match (use_count, days_since_use) {
        // Never used - red
        (0, _) => ("●", theme.red),
        // Used within last 7 days - green
        (_, Some(days)) if days < 7 => ("●", theme.green),
        // Used within last 30 days - yellow
        (_, Some(days)) if days < 30 => ("●", theme.yellow),
        // Used but more than 30 days ago - red
        (_, Some(_)) => ("●", theme.red),
        // Has usage but no timestamp (legacy data) - green
        (_, None) => ("●", theme.green),
    }
}

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

    render_header(frame, app, &theme, chunks[0]);
    render_body(frame, app, db, &theme, chunks[1]);
    render_footer(frame, app, &theme, chunks[2]);

    // Render overlays (in order of priority)
    if app.show_help {
        render_help_overlay(frame, &theme, area);
    }

    if app.show_details_popup {
        render_details_popup(frame, app, db, &theme, area);
    }

    // Confirmation dialog takes highest priority
    if app.has_pending_action() {
        render_confirmation_dialog(frame, app, &theme, area);
    }

    // Loading overlay takes absolute highest priority
    if app.has_background_op() {
        render_loading_overlay(frame, app, &theme, area);
    }
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
        .select(app.tab.index());

    frame.render_widget(tabs, area);
}

fn render_body(frame: &mut Frame, app: &mut App, db: &Database, theme: &Theme, area: Rect) {
    // Responsive layout: side-by-side for wide terminals, stacked for narrow
    let min_width_for_split = 80;

    // Bundles tab has its own rendering
    if app.tab == super::app::Tab::Bundles {
        if area.width >= min_width_for_split {
            let chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
                .split(area);

            render_bundle_list(frame, app, theme, chunks[0]);
            render_bundle_details(frame, app, db, theme, chunks[1]);
        } else {
            render_bundle_list(frame, app, theme, area);
        }
        return;
    }

    if area.width >= min_width_for_split {
        // Wide terminal: side-by-side layout
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
            .split(area);

        render_tool_list(frame, app, theme, chunks[0]);
        render_details(frame, app, db, theme, chunks[1]);
    } else {
        // Narrow terminal: list only (details on Enter in future)
        render_tool_list(frame, app, theme, area);
    }
}

fn render_tool_list(frame: &mut Frame, app: &App, theme: &Theme, area: Rect) {
    // Special handling for Updates tab when not checked yet
    if app.tab == super::app::Tab::Updates && !app.updates_checked {
        let message = if app.updates_loading {
            "Checking for updates..."
        } else {
            "Press 'r' to check for updates"
        };
        let paragraph = Paragraph::new(message)
            .style(Style::default().fg(theme.subtext0))
            .alignment(Alignment::Center)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(theme.surface1))
                    .title(Span::styled(" Updates ", Style::default().fg(theme.text))),
            );
        frame.render_widget(paragraph, area);
        return;
    }

    let items: Vec<ListItem> = app
        .tools
        .iter()
        .enumerate()
        .map(|(i, tool)| {
            // Selection checkbox
            let selected = app.is_selected(&tool.name);
            let checkbox = if selected { "☑" } else { "☐" };
            let checkbox_color = if selected { theme.blue } else { theme.surface1 };

            // Source icon
            let src_icon = source_icon(&tool.source.to_string());

            // For Updates tab, show version info instead of usage
            let (extra_info, spark) = if app.tab == super::app::Tab::Updates {
                let info = if let Some(update) = app.get_update(&tool.name) {
                    format!(" {} → {}", update.current, update.latest)
                } else {
                    String::new()
                };
                (info, String::new())
            } else {
                // Usage count and sparkline for other tabs
                let usage = app.get_usage(&tool.name).map(|u| u.use_count).unwrap_or(0);
                let daily = app.daily_usage.get(&tool.name);
                let spark_str = daily.map(|d| sparkline(d)).unwrap_or_default();
                let info = if usage > 0 {
                    format!(" ({usage})")
                } else {
                    String::new()
                };
                (info, spark_str)
            };

            // Status indicator based on health
            let (status, status_color) = if app.tab == super::app::Tab::Updates {
                // Show update indicator for Updates tab
                ("↑", theme.yellow)
            } else if !tool.is_installed {
                // Not installed - hollow circle
                ("○", theme.subtext0)
            } else {
                // Use health indicator based on usage recency
                let usage = app.get_usage(&tool.name);
                let use_count = usage.as_ref().map(|u| u.use_count).unwrap_or(0);
                let last_used = usage.as_ref().and_then(|u| u.last_used.as_deref());
                health_indicator(last_used, use_count, theme)
            };

            let extra_color = if app.tab == super::app::Tab::Updates {
                theme.yellow
            } else {
                theme.subtext0
            };

            // Build sparkline span (show only if not empty)
            let spark_span = if spark.is_empty() {
                Span::raw("")
            } else {
                Span::styled(format!(" {spark}"), Style::default().fg(theme.teal))
            };

            // GitHub stars (show compact format if available)
            let stars_span = app
                .github_cache
                .get(&tool.name)
                .filter(|gh| gh.stars > 0)
                .map(|gh| {
                    Span::styled(
                        format!(" ★{}", format_stars(gh.stars)),
                        Style::default().fg(theme.yellow),
                    )
                })
                .unwrap_or_else(|| Span::raw(""));

            let content = Line::from(vec![
                Span::styled(format!("{checkbox} "), Style::default().fg(checkbox_color)),
                Span::styled(format!("{src_icon} "), Style::default()),
                Span::styled(format!("{status} "), Style::default().fg(status_color)),
                Span::styled(&tool.name, Style::default().fg(theme.text)),
                stars_span,
                Span::styled(extra_info, Style::default().fg(extra_color)),
                spark_span,
            ]);

            let style = if i == app.selected_index {
                Style::default().bg(theme.surface0)
            } else {
                Style::default()
            };

            ListItem::new(content).style(style)
        })
        .collect();

    // Title with count, selection count, and sort indicator
    let selection_info = if app.selection_count() > 0 {
        format!(" ({} selected)", app.selection_count())
    } else {
        String::new()
    };

    let title_text = if app.tab == super::app::Tab::Updates {
        format!(" Updates [{}]{} ", app.tools.len(), selection_info)
    } else {
        format!(
            " Tools [{}]{} ({}↕) ",
            app.tools.len(),
            selection_info,
            app.sort_by.label()
        )
    };

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme.surface1))
                .title(Span::styled(title_text, Style::default().fg(theme.text))),
        )
        .highlight_style(
            Style::default()
                .bg(theme.surface0)
                .add_modifier(Modifier::BOLD),
        );

    let mut state = ListState::default();
    state.select(Some(app.selected_index));

    // Scroll list to keep selection visible
    let visible_height = area.height.saturating_sub(2) as usize; // Subtract border
    if visible_height > 0 {
        *state.offset_mut() = app.selected_index.saturating_sub(visible_height / 2);
    }

    frame.render_stateful_widget(list, area, &mut state);
}

fn render_details(frame: &mut Frame, app: &mut App, db: &Database, theme: &Theme, area: Rect) {
    // Clone selected tool to avoid borrow issues
    let tool = app.selected_tool().cloned();

    let content = if let Some(tool) = tool {
        // Pre-fetch GitHub info while we have mutable access
        let _ = app.get_github_info(&tool.name, db);

        let mut lines = vec![
            Line::from(vec![
                Span::styled("Name: ", Style::default().fg(theme.subtext0)),
                Span::styled(tool.name.clone(), Style::default().fg(theme.blue).bold()),
            ]),
            Line::from(""),
        ];

        // Description
        if let Some(desc) = &tool.description {
            lines.push(Line::from(Span::styled(
                "Description:",
                Style::default().fg(theme.subtext0),
            )));
            lines.push(Line::from(Span::styled(
                desc.clone(),
                Style::default().fg(theme.text),
            )));
            lines.push(Line::from(""));
        }

        // Source and install command
        let src_icon = source_icon(&tool.source.to_string());
        lines.push(Line::from(vec![
            Span::styled("Source: ", Style::default().fg(theme.subtext0)),
            Span::styled(
                format!("{src_icon} {}", tool.source),
                Style::default().fg(theme.peach),
            ),
        ]));

        if let Some(cmd) = &tool.install_command {
            lines.push(Line::from(vec![
                Span::styled("Install: ", Style::default().fg(theme.subtext0)),
                Span::styled(cmd.clone(), Style::default().fg(theme.green)),
            ]));
        }

        // Binary name
        if let Some(binary) = &tool.binary_name {
            lines.push(Line::from(vec![
                Span::styled("Binary: ", Style::default().fg(theme.subtext0)),
                Span::styled(binary.clone(), Style::default().fg(theme.text)),
            ]));
        }

        // Category
        if let Some(category) = &tool.category {
            lines.push(Line::from(vec![
                Span::styled("Category: ", Style::default().fg(theme.subtext0)),
                Span::styled(category.clone(), Style::default().fg(theme.mauve)),
            ]));
        }

        lines.push(Line::from(""));

        // Usage statistics
        if let Some(usage) = app.usage_data.get(&tool.name) {
            lines.push(Line::from(Span::styled(
                "Usage:",
                Style::default()
                    .fg(theme.subtext0)
                    .add_modifier(Modifier::BOLD),
            )));
            lines.push(Line::from(vec![
                Span::styled("  Invocations: ", Style::default().fg(theme.subtext0)),
                Span::styled(
                    format!("{}", usage.use_count),
                    Style::default().fg(theme.teal),
                ),
            ]));
            if let Some(last) = &usage.last_used {
                lines.push(Line::from(vec![
                    Span::styled("  Last used: ", Style::default().fg(theme.subtext0)),
                    Span::styled(last.clone(), Style::default().fg(theme.text)),
                ]));
            }
            lines.push(Line::from(""));
        }

        // GitHub info (already fetched above)
        if let Some(gh) = app.github_cache.get(&tool.name) {
            lines.push(Line::from(Span::styled(
                "GitHub:",
                Style::default()
                    .fg(theme.subtext0)
                    .add_modifier(Modifier::BOLD),
            )));
            lines.push(Line::from(vec![
                Span::styled("  ★ Stars: ", Style::default().fg(theme.yellow)),
                Span::styled(format_stars(gh.stars), Style::default().fg(theme.yellow)),
            ]));
            if let Some(lang) = &gh.language {
                lines.push(Line::from(vec![
                    Span::styled("  Language: ", Style::default().fg(theme.subtext0)),
                    Span::styled(lang.clone(), Style::default().fg(theme.peach)),
                ]));
            }
            lines.push(Line::from(vec![
                Span::styled("  Repo: ", Style::default().fg(theme.subtext0)),
                Span::styled(
                    format!("{}/{}", gh.repo_owner, gh.repo_name),
                    Style::default().fg(theme.blue),
                ),
            ]));
            lines.push(Line::from(""));
        }

        // Status with health indicator
        let (status_text, status_color, health_hint) = if !tool.is_installed {
            ("Not installed", theme.yellow, None)
        } else {
            let usage = app.usage_data.get(&tool.name);
            let use_count = usage.map(|u| u.use_count).unwrap_or(0);
            let last_used = usage.and_then(|u| u.last_used.as_deref());

            // Calculate days since use
            let days_since = last_used
                .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
                .map(|dt| (Utc::now() - dt.with_timezone(&Utc)).num_days());

            match (use_count, days_since) {
                (0, _) => (
                    "Installed (never used)",
                    theme.red,
                    Some("Consider using or removing"),
                ),
                (_, Some(d)) if d < 7 => ("Installed (active)", theme.green, None),
                (_, Some(d)) if d < 30 => (
                    "Installed (idle)",
                    theme.yellow,
                    Some(&format!("Last used {} days ago", d) as &str).map(|_| "Not used recently"),
                ),
                (_, Some(_)) => ("Installed (stale)", theme.red, Some("Not used in 30+ days")),
                (_, None) => ("Installed", theme.green, None),
            }
        };
        lines.push(Line::from(vec![
            Span::styled("Status: ", Style::default().fg(theme.subtext0)),
            Span::styled(status_text, Style::default().fg(status_color)),
        ]));
        if let Some(hint) = health_hint {
            lines.push(Line::from(Span::styled(
                format!("  ↳ {hint}"),
                Style::default().fg(theme.subtext0),
            )));
        }

        if tool.is_favorite {
            lines.push(Line::from(Span::styled(
                "★ Favorite",
                Style::default().fg(theme.yellow),
            )));
        }

        Text::from(lines)
    } else {
        Text::from(Span::styled(
            "No tool selected",
            Style::default().fg(theme.subtext0),
        ))
    };

    let details = Paragraph::new(content)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme.surface1))
                .title(Span::styled(" Details ", Style::default().fg(theme.text))),
        )
        .wrap(Wrap { trim: true });

    frame.render_widget(details, area);
}

/// Format star count (e.g., 1234 -> "1.2K")
fn format_stars(stars: i64) -> String {
    if stars >= 1000 {
        format!("{:.1}K", stars as f64 / 1000.0)
    } else {
        stars.to_string()
    }
}

fn render_bundle_list(frame: &mut Frame, app: &App, theme: &Theme, area: Rect) {
    if app.bundles.is_empty() {
        let message =
            "No bundles yet. Create one with: hoards bundle create <name> --tools tool1,tool2";
        let paragraph = Paragraph::new(message)
            .style(Style::default().fg(theme.subtext0))
            .alignment(Alignment::Center)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(theme.surface1))
                    .title(Span::styled(" Bundles ", Style::default().fg(theme.text))),
            );
        frame.render_widget(paragraph, area);
        return;
    }

    let items: Vec<ListItem> = app
        .bundles
        .iter()
        .enumerate()
        .map(|(i, bundle)| {
            let tool_count = bundle.tools.len();
            let count_str = if tool_count == 1 {
                "1 tool".to_string()
            } else {
                format!("{} tools", tool_count)
            };

            let content = Line::from(vec![
                Span::styled("📦 ", Style::default()),
                Span::styled(&bundle.name, Style::default().fg(theme.text).bold()),
                Span::styled(
                    format!(" ({})", count_str),
                    Style::default().fg(theme.subtext0),
                ),
            ]);

            let style = if i == app.bundle_selected {
                Style::default().bg(theme.surface0)
            } else {
                Style::default()
            };

            ListItem::new(content).style(style)
        })
        .collect();

    let title = format!(" Bundles [{}] ", app.bundles.len());

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme.surface1))
                .title(Span::styled(title, Style::default().fg(theme.text))),
        )
        .highlight_style(
            Style::default()
                .bg(theme.surface0)
                .add_modifier(Modifier::BOLD),
        );

    let mut state = ListState::default();
    state.select(Some(app.bundle_selected));

    frame.render_stateful_widget(list, area, &mut state);
}

fn render_bundle_details(frame: &mut Frame, app: &App, db: &Database, theme: &Theme, area: Rect) {
    let content = if let Some(bundle) = app.bundles.get(app.bundle_selected) {
        let mut lines = vec![
            Line::from(Span::styled(
                &bundle.name,
                Style::default()
                    .fg(theme.blue)
                    .bold()
                    .add_modifier(Modifier::UNDERLINED),
            )),
            Line::from(""),
        ];

        // Description
        if let Some(desc) = &bundle.description {
            lines.push(Line::from(Span::styled(
                desc.clone(),
                Style::default().fg(theme.text),
            )));
            lines.push(Line::from(""));
        }

        // Tool count
        lines.push(Line::from(vec![
            Span::styled("Tools: ", Style::default().fg(theme.subtext0)),
            Span::styled(
                format!("{}", bundle.tools.len()),
                Style::default().fg(theme.teal),
            ),
        ]));
        lines.push(Line::from(""));

        // List tools with installation status
        lines.push(Line::from(Span::styled(
            "─── Contents ───",
            Style::default().fg(theme.surface1),
        )));

        for tool_name in &bundle.tools {
            // Check if tool is installed
            let is_installed = db
                .get_tool_by_name(tool_name)
                .ok()
                .flatten()
                .map(|t| t.is_installed)
                .unwrap_or(false);

            let (status, status_color) = if is_installed {
                ("●", theme.green)
            } else {
                ("○", theme.subtext0)
            };

            lines.push(Line::from(vec![
                Span::styled(format!("  {} ", status), Style::default().fg(status_color)),
                Span::styled(tool_name.clone(), Style::default().fg(theme.text)),
            ]));
        }

        lines.push(Line::from(""));

        // Install hint
        let not_installed: Vec<_> = bundle
            .tools
            .iter()
            .filter(|name| {
                !db.get_tool_by_name(name)
                    .ok()
                    .flatten()
                    .map(|t| t.is_installed)
                    .unwrap_or(false)
            })
            .collect();

        if !not_installed.is_empty() {
            lines.push(Line::from(Span::styled(
                format!(
                    "Press 'i' to install {} missing tool(s)",
                    not_installed.len()
                ),
                Style::default().fg(theme.green),
            )));
        } else {
            lines.push(Line::from(Span::styled(
                "All tools installed ✓",
                Style::default().fg(theme.green),
            )));
        }

        Text::from(lines)
    } else {
        Text::from("No bundle selected")
    };

    let details = Paragraph::new(content)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme.surface1))
                .title(Span::styled(
                    " Bundle Details ",
                    Style::default().fg(theme.text),
                )),
        )
        .wrap(Wrap { trim: true });

    frame.render_widget(details, area);
}

fn render_footer(frame: &mut Frame, app: &App, theme: &Theme, area: Rect) {
    // Show status message if present (takes priority)
    if let Some(status) = &app.status_message {
        let color = if status.is_error {
            theme.red
        } else {
            theme.green
        };
        let footer = Paragraph::new(Line::from(vec![
            Span::styled(" ", Style::default()),
            Span::styled(&status.text, Style::default().fg(color)),
        ]))
        .style(Style::default().bg(theme.surface0));

        frame.render_widget(footer, area);
        return;
    }

    let mode_text = match app.input_mode {
        InputMode::Normal => {
            let mut spans = vec![
                Span::styled(" j/k", Style::default().fg(theme.blue)),
                Span::styled(" nav ", Style::default().fg(theme.subtext0)),
                Span::styled(" Space", Style::default().fg(theme.blue)),
                Span::styled(" select ", Style::default().fg(theme.subtext0)),
                Span::styled(" i", Style::default().fg(theme.green)),
                Span::styled(" install ", Style::default().fg(theme.subtext0)),
                Span::styled(" D", Style::default().fg(theme.red)),
                Span::styled(" uninstall ", Style::default().fg(theme.subtext0)),
                Span::styled(" u", Style::default().fg(theme.yellow)),
                Span::styled(" update ", Style::default().fg(theme.subtext0)),
                Span::styled(" ?", Style::default().fg(theme.blue)),
                Span::styled(" help", Style::default().fg(theme.subtext0)),
            ];

            // Show selection count or filter
            if app.selection_count() > 0 {
                spans.push(Span::styled(" │ ", Style::default().fg(theme.surface1)));
                spans.push(Span::styled(
                    format!("{} selected", app.selection_count()),
                    Style::default().fg(theme.blue),
                ));
            } else if !app.search_query.is_empty() {
                spans.push(Span::styled(" │ ", Style::default().fg(theme.surface1)));
                spans.push(Span::styled("filter:", Style::default().fg(theme.yellow)));
                spans.push(Span::styled(
                    &app.search_query,
                    Style::default().fg(theme.text),
                ));
            }

            spans
        }
        InputMode::Search => {
            vec![
                Span::styled(" Search: ", Style::default().fg(theme.yellow)),
                Span::styled(&app.search_query, Style::default().fg(theme.text)),
                Span::styled("│", Style::default().fg(theme.blue)), // Cursor
                Span::styled("  Enter", Style::default().fg(theme.blue)),
                Span::styled(" apply ", Style::default().fg(theme.subtext0)),
                Span::styled(" Esc", Style::default().fg(theme.blue)),
                Span::styled(" cancel", Style::default().fg(theme.subtext0)),
            ]
        }
    };

    let footer = Paragraph::new(Line::from(mode_text)).style(Style::default().bg(theme.surface0));

    frame.render_widget(footer, area);
}

fn render_help_overlay(frame: &mut Frame, theme: &Theme, area: Rect) {
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

fn render_details_popup(
    frame: &mut Frame,
    app: &mut App,
    db: &Database,
    theme: &Theme,
    area: Rect,
) {
    let popup_area = centered_rect(70, 80, area);

    let content = if let Some(tool) = app.selected_tool().cloned() {
        // Pre-fetch GitHub info
        let _ = app.get_github_info(&tool.name, db);

        let mut lines = vec![
            Line::from(Span::styled(
                tool.name.clone(),
                Style::default()
                    .fg(theme.blue)
                    .bold()
                    .add_modifier(Modifier::UNDERLINED),
            )),
            Line::from(""),
        ];

        // Description
        if let Some(desc) = &tool.description {
            lines.push(Line::from(Span::styled(
                desc.clone(),
                Style::default().fg(theme.text),
            )));
            lines.push(Line::from(""));
        }

        // Source and install
        let src_icon = source_icon(&tool.source.to_string());
        lines.push(Line::from(vec![
            Span::styled("Source: ", Style::default().fg(theme.subtext0)),
            Span::styled(
                format!("{src_icon} {}", tool.source),
                Style::default().fg(theme.peach),
            ),
        ]));

        if let Some(cmd) = &tool.install_command {
            lines.push(Line::from(vec![
                Span::styled("Install: ", Style::default().fg(theme.subtext0)),
                Span::styled(cmd.clone(), Style::default().fg(theme.green)),
            ]));
        }

        if let Some(binary) = &tool.binary_name {
            lines.push(Line::from(vec![
                Span::styled("Binary: ", Style::default().fg(theme.subtext0)),
                Span::styled(binary.clone(), Style::default().fg(theme.text)),
            ]));
        }

        if let Some(category) = &tool.category {
            lines.push(Line::from(vec![
                Span::styled("Category: ", Style::default().fg(theme.subtext0)),
                Span::styled(category.clone(), Style::default().fg(theme.mauve)),
            ]));
        }

        lines.push(Line::from(""));

        // Usage
        if let Some(usage) = app.usage_data.get(&tool.name) {
            lines.push(Line::from(vec![
                Span::styled("Usage: ", Style::default().fg(theme.subtext0)),
                Span::styled(
                    format!("{} invocations", usage.use_count),
                    Style::default().fg(theme.teal),
                ),
            ]));
            if let Some(last) = &usage.last_used {
                lines.push(Line::from(vec![
                    Span::styled("Last used: ", Style::default().fg(theme.subtext0)),
                    Span::styled(last.clone(), Style::default().fg(theme.text)),
                ]));
            }
        }

        // GitHub
        if let Some(gh) = app.github_cache.get(&tool.name) {
            lines.push(Line::from(vec![
                Span::styled("★ Stars: ", Style::default().fg(theme.yellow)),
                Span::styled(format_stars(gh.stars), Style::default().fg(theme.yellow)),
                Span::styled("  ", Style::default()),
                Span::styled(&gh.repo_owner, Style::default().fg(theme.subtext0)),
                Span::styled("/", Style::default().fg(theme.subtext0)),
                Span::styled(&gh.repo_name, Style::default().fg(theme.blue)),
            ]));
        }

        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "Press Enter or Esc to close",
            Style::default().fg(theme.subtext0),
        )));

        Text::from(lines)
    } else {
        Text::from("No tool selected")
    };

    let popup = Paragraph::new(content)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme.blue))
                .title(Span::styled(
                    " Details ",
                    Style::default().fg(theme.blue).bold(),
                ))
                .style(Style::default().bg(theme.base)),
        )
        .wrap(Wrap { trim: true });

    frame.render_widget(Clear, popup_area);
    frame.render_widget(popup, popup_area);
}

fn render_loading_overlay(frame: &mut Frame, app: &App, theme: &Theme, area: Rect) {
    let popup_area = centered_rect(50, 30, area);

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

fn render_confirmation_dialog(frame: &mut Frame, app: &App, theme: &Theme, area: Rect) {
    let popup_area = centered_rect(50, 30, area);

    let (title, description, color) = if let Some(action) = &app.pending_action {
        match action {
            super::app::PendingAction::Install(tools) => {
                let desc = action.description();
                let tool_list = if tools.len() <= 3 {
                    tools.join(", ")
                } else {
                    format!(
                        "{}, ... and {} more",
                        tools[..2].join(", "),
                        tools.len() - 2
                    )
                };
                (
                    " Install ",
                    format!("{}\n\nTools: {}", desc, tool_list),
                    theme.green,
                )
            }
            super::app::PendingAction::Uninstall(tools) => {
                let desc = action.description();
                let tool_list = if tools.len() <= 3 {
                    tools.join(", ")
                } else {
                    format!(
                        "{}, ... and {} more",
                        tools[..2].join(", "),
                        tools.len() - 2
                    )
                };
                (
                    " Uninstall ",
                    format!("{}\n\nTools: {}", desc, tool_list),
                    theme.red,
                )
            }
            super::app::PendingAction::Update(tools) => {
                let desc = action.description();
                let tool_list = if tools.len() <= 3 {
                    tools.join(", ")
                } else {
                    format!(
                        "{}, ... and {} more",
                        tools[..2].join(", "),
                        tools.len() - 2
                    )
                };
                (
                    " Update ",
                    format!("{}\n\nTools: {}", desc, tool_list),
                    theme.yellow,
                )
            }
        }
    } else {
        return;
    };

    let content = Text::from(vec![
        Line::from(""),
        Line::from(Span::styled(description, Style::default().fg(theme.text))),
        Line::from(""),
        Line::from(""),
        Line::from(vec![
            Span::styled("Press ", Style::default().fg(theme.subtext0)),
            Span::styled("y", Style::default().fg(theme.green).bold()),
            Span::styled(" to confirm, ", Style::default().fg(theme.subtext0)),
            Span::styled("n", Style::default().fg(theme.red).bold()),
            Span::styled(" or ", Style::default().fg(theme.subtext0)),
            Span::styled("Esc", Style::default().fg(theme.yellow).bold()),
            Span::styled(" to cancel", Style::default().fg(theme.subtext0)),
        ]),
    ]);

    let popup = Paragraph::new(content)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(color))
                .title(Span::styled(title, Style::default().fg(color).bold()))
                .style(Style::default().bg(theme.base)),
        )
        .alignment(Alignment::Center)
        .wrap(Wrap { trim: true });

    frame.render_widget(Clear, popup_area);
    frame.render_widget(popup, popup_area);
}

/// Helper function to create a centered rectangle
fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(area);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
