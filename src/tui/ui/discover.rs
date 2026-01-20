//! Discover tab rendering
//!
//! This module handles rendering of the Discover tab including search controls,
//! results list, and details panel.

use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Margin, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, Borders, List, ListItem, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState,
        Wrap,
    },
};

use super::super::app::{App, InputMode, Tab};
use super::super::theme::Theme;
use super::helpers::format_stars;

/// Render the discover tab
pub fn render_discover_tab(frame: &mut Frame, app: &mut App, theme: &Theme, area: Rect) {
    // Split into controls area (filters + search) and content
    let vertical_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(0)])
        .split(area);

    // Render the search controls area (AI toggle, filters, search bar)
    render_discover_search_controls(frame, app, theme, vertical_chunks[0]);

    // Content area (results + details)
    let content_area = vertical_chunks[1];

    // Results area
    if app.discover_results.is_empty() {
        render_discover_empty_state(frame, app, theme, content_area);
    } else {
        // Split horizontally for list and details (if wide enough)
        let min_width_for_split = 80;
        if content_area.width >= min_width_for_split {
            let horizontal_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
                .split(content_area);

            render_discover_list(frame, app, theme, horizontal_chunks[0]);
            render_discover_details(frame, app, theme, horizontal_chunks[1]);
        } else {
            // Narrow terminal: list only
            render_discover_list(frame, app, theme, content_area);
        }
    }
}

/// Render the discover search controls (AI toggle, source filters, search input)
fn render_discover_search_controls(frame: &mut Frame, app: &App, theme: &Theme, area: Rect) {
    let is_search_mode = app.input_mode == InputMode::Search && app.tab == Tab::Discover;

    // Get available sources (config-aware)
    let available_sources = app.get_available_discover_sources();

    // Calculate widths dynamically
    // AI toggle: "[x]🤖" = ~6 chars
    let ai_width = 7u16;

    // Filter chips: each is "F1[x]🦀 " (Fn + checkbox + icon + space)
    // Note: emojis display as 2 cells wide in terminal
    let filter_chips_width: u16 = available_sources
        .iter()
        .enumerate()
        .map(|(idx, (_, icon, _))| {
            // "Fn" (2-3 chars) + "[x]" (3) + icon (2 for emoji, 1 for nerd font) + space (1)
            let fkey_width = if idx + 1 >= 10 { 3 } else { 2 }; // F1 vs F10
            // Emojis are 2 cells wide, nerd font icons are 1 cell
            let icon_width = if icon.chars().any(|c| c > '\u{1000}') { 2 } else { 1 };
            (fkey_width + 3 + icon_width + 1) as u16
        })
        .sum::<u16>()
        + 2; // borders only (block has 1 char border on each side)

    let min_search_width = 20u16;
    let controls_width = ai_width + filter_chips_width;

    // If not enough space, just show search bar
    if area.width < controls_width + min_search_width {
        render_discover_search_bar_only(frame, app, theme, area, is_search_mode);
        return;
    }

    // Split horizontally: AI toggle | Filters | Search bar
    let horizontal_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(ai_width),
            Constraint::Length(filter_chips_width),
            Constraint::Min(min_search_width),
        ])
        .split(area);

    // AI toggle with robot emoji
    let ai_checkbox = if app.discover_ai_enabled {
        "[x]"
    } else {
        "[ ]"
    };
    let ai_text = format!("{}🤖", ai_checkbox);
    let ai_style = if app.discover_ai_enabled {
        Style::default().fg(theme.green).bold()
    } else {
        Style::default().fg(theme.subtext0)
    };
    let ai_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(if app.discover_ai_enabled {
            theme.green
        } else {
            theme.surface1
        }));
    let ai_para = Paragraph::new(Span::styled(ai_text, ai_style)).block(ai_block);
    frame.render_widget(ai_para, horizontal_chunks[0]);

    // Source filter chips with icons only (dimmed if AI is enabled)
    // F1-F6 keys map to sources by index
    let mut filter_spans: Vec<Span> = Vec::new();
    for (idx, (key, icon, _display)) in available_sources.iter().enumerate() {
        let is_enabled = app.is_discover_source_enabled(key);
        let checkbox = if is_enabled { "x" } else { " " };
        let fkey = idx + 1; // F1, F2, etc.

        // Dim filters when AI mode is active (they're ignored)
        let style = if app.discover_ai_enabled {
            Style::default().fg(theme.surface1) // Very dim when AI mode
        } else if is_enabled {
            Style::default().fg(theme.blue)
        } else {
            Style::default().fg(theme.subtext0)
        };
        // Format: "F1[x]🦀 " to show the key binding
        filter_spans.push(Span::styled(
            format!("F{}[{}]{} ", fkey, checkbox, icon),
            style,
        ));
    }

    let filter_border_color = if app.discover_ai_enabled {
        theme.surface0
    } else {
        theme.surface1
    };
    let filter_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(filter_border_color));
    let filter_para = Paragraph::new(Line::from(filter_spans)).block(filter_block);
    frame.render_widget(filter_para, horizontal_chunks[1]);

    // Search bar
    let search_title = if app.discover_loading {
        if !app.loading_progress.step_name.is_empty() {
            format!(
                " {} ({}/{}) ",
                app.loading_progress.step_name,
                app.loading_progress.current_step,
                app.loading_progress.total_steps
            )
        } else {
            " Searching... ".to_string()
        }
    } else if let Some(idx) = app.discover_history_index {
        format!(" History [{}/{}] ", idx + 1, app.discover_history.len())
    } else {
        " Search ".to_string()
    };

    let search_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(if is_search_mode {
            theme.blue
        } else {
            theme.surface1
        }))
        .title(Span::styled(search_title, Style::default().fg(theme.text)));

    let search_text = if app.discover_query.is_empty() && !is_search_mode {
        Span::styled(
            "Press / to search, ↑↓ history",
            Style::default().fg(theme.subtext0),
        )
    } else {
        Span::styled(&app.discover_query, Style::default().fg(theme.text))
    };

    let search_para = Paragraph::new(search_text).block(search_block);
    frame.render_widget(search_para, horizontal_chunks[2]);
}

/// Fallback for narrow terminals - just show search bar
fn render_discover_search_bar_only(
    frame: &mut Frame,
    app: &App,
    theme: &Theme,
    area: Rect,
    is_search_mode: bool,
) {
    let search_title = if app.discover_loading {
        " Searching... ".to_string()
    } else {
        " Search ".to_string()
    };

    let search_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(if is_search_mode {
            theme.blue
        } else {
            theme.surface1
        }))
        .title(Span::styled(search_title, Style::default().fg(theme.text)));

    let search_text = if app.discover_query.is_empty() && !is_search_mode {
        Span::styled("Press / to search", Style::default().fg(theme.subtext0))
    } else {
        Span::styled(&app.discover_query, Style::default().fg(theme.text))
    };

    let search_para = Paragraph::new(search_text).block(search_block);
    frame.render_widget(search_para, area);
}

/// Render the discover empty state with instructions
fn render_discover_empty_state(frame: &mut Frame, app: &App, theme: &Theme, area: Rect) {
    let message = if app.discover_query.is_empty() {
        // Build dynamic source list based on what's available
        let available_sources = app.get_available_discover_sources();
        let mut lines = vec![
            Line::from(""),
            Line::from(Span::styled(
                "🔍 Discover new tools",
                Style::default().fg(theme.text).add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "Search across multiple sources:",
                Style::default().fg(theme.subtext0),
            )),
        ];

        // Add each available source
        for (key, icon, _display) in &available_sources {
            let desc = match *key {
                "cargo" => "Rust packages (crates.io)",
                "npm" => "Node.js packages",
                "pip" => "Python packages (PyPI)",
                "brew" => "Homebrew formulae",
                "apt" => "Debian/Ubuntu packages",
                "github" => "GitHub repositories",
                _ => continue,
            };
            lines.push(Line::from(vec![
                Span::styled(format!("  {} ", icon), Style::default().fg(theme.subtext0)),
                Span::styled(desc, Style::default().fg(theme.text)),
            ]));
        }

        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "─── Keyboard shortcuts ───",
            Style::default().fg(theme.surface1),
        )));
        lines.push(Line::from(vec![
            Span::styled("  /         ", Style::default().fg(theme.blue)),
            Span::styled("Enter search mode", Style::default().fg(theme.subtext0)),
        ]));
        lines.push(Line::from(vec![
            Span::styled("  ↑/↓       ", Style::default().fg(theme.blue)),
            Span::styled(
                "Browse search history (in search mode)",
                Style::default().fg(theme.subtext0),
            ),
        ]));
        lines.push(Line::from(vec![
            Span::styled("  Shift+A   ", Style::default().fg(theme.blue)),
            Span::styled("Toggle AI search mode", Style::default().fg(theme.subtext0)),
        ]));
        // Build dynamic F-key range based on available sources
        let fkey_label = if available_sources.is_empty() {
            "  -         ".to_string()
        } else if available_sources.len() == 1 {
            "  F1        ".to_string()
        } else {
            format!("  F1-F{}     ", available_sources.len())
        };
        lines.push(Line::from(vec![
            Span::styled(fkey_label, Style::default().fg(theme.blue)),
            Span::styled("Toggle source filters", Style::default().fg(theme.subtext0)),
        ]));
        lines.push(Line::from(vec![
            Span::styled("  Enter     ", Style::default().fg(theme.blue)),
            Span::styled(
                "View README for selected result",
                Style::default().fg(theme.subtext0),
            ),
        ]));
        lines.push(Line::from(""));

        if app.ai_available {
            lines.push(Line::from(vec![
                Span::styled("🤖 ", Style::default()),
                Span::styled("AI mode: ", Style::default().fg(theme.green).bold()),
                Span::styled(
                    "Press Shift+A to enable AI-powered discovery",
                    Style::default().fg(theme.subtext0),
                ),
            ]));
        }

        lines
    } else {
        vec![
            Line::from(""),
            Line::from(Span::styled(
                "No results found",
                Style::default().fg(theme.subtext0),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "Try a different search term",
                Style::default().fg(theme.subtext0),
            )),
        ]
    };

    let empty = Paragraph::new(message)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme.surface1))
                .title(Span::styled(" Results ", Style::default().fg(theme.text))),
        )
        .alignment(Alignment::Center);

    frame.render_widget(empty, area);
}

/// Render the discover results list
pub fn render_discover_list(frame: &mut Frame, app: &mut App, theme: &Theme, area: Rect) {
    // Store list area for mouse interaction
    app.set_list_area(area.x, area.y, area.width, area.height);

    let items: Vec<ListItem> = app
        .discover_results
        .iter()
        .map(|result| {
            let icon = result.source.icon();
            let stars_str = result
                .stars
                .map(|s| format!(" ★ {}", format_stars(s as i64)))
                .unwrap_or_default();

            let content = Line::from(vec![
                Span::styled(format!("{} ", icon), Style::default()),
                Span::styled(&result.name, Style::default().fg(theme.text)),
                Span::styled(stars_str, Style::default().fg(theme.yellow)),
            ]);

            ListItem::new(content)
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme.surface1))
                .title(Span::styled(
                    format!(
                        " Results [{}] ({}↕) ",
                        app.discover_results.len(),
                        app.discover_sort_by.label()
                    ),
                    Style::default().fg(theme.text),
                )),
        )
        .highlight_style(Style::default().bg(theme.surface0));

    // Calculate scroll offset to keep selection visible
    let visible_height = area.height.saturating_sub(2) as usize;
    let mut state = ratatui::widgets::ListState::default();

    if !app.discover_results.is_empty() && visible_height > 0 {
        // Keep selection centered when possible
        let offset = app.discover_selected.saturating_sub(visible_height / 2);
        let max_offset = app.discover_results.len().saturating_sub(visible_height);
        *state.offset_mut() = offset.min(max_offset);
        state.select(Some(app.discover_selected));
    }

    frame.render_stateful_widget(list, area, &mut state);

    // Scrollbar for results
    if app.discover_results.len() > visible_height {
        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(Some("↑"))
            .end_symbol(Some("↓"))
            .track_symbol(Some("│"))
            .thumb_symbol("█");

        let mut scrollbar_state =
            ScrollbarState::new(app.discover_results.len()).position(app.discover_selected);

        frame.render_stateful_widget(
            scrollbar,
            area.inner(Margin {
                vertical: 1,
                horizontal: 0,
            }),
            &mut scrollbar_state,
        );
    }
}

/// Render the discover details panel
pub fn render_discover_details(frame: &mut Frame, app: &App, theme: &Theme, area: Rect) {
    let content = if let Some(result) = app.selected_discover() {
        let mut lines = vec![
            Line::from(vec![
                Span::styled("Name: ", Style::default().fg(theme.subtext0)),
                Span::styled(result.name.clone(), Style::default().fg(theme.blue).bold()),
            ]),
            Line::from(""),
        ];

        // Description
        if let Some(desc) = &result.description {
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

        // Source info section (like GitHub section in installed view)
        lines.push(Line::from(Span::styled(
            "Package Info:",
            Style::default()
                .fg(theme.subtext0)
                .add_modifier(Modifier::BOLD),
        )));

        // Source
        let icon = result.source.icon();
        lines.push(Line::from(vec![
            Span::styled("  Source: ", Style::default().fg(theme.subtext0)),
            Span::styled(
                format!("{} {:?}", icon, result.source),
                Style::default().fg(theme.peach),
            ),
        ]));

        // Language (explicit or inferred from source)
        let language = result.get_language();
        if let Some(lang) = language {
            lines.push(Line::from(vec![
                Span::styled("  Language: ", Style::default().fg(theme.subtext0)),
                Span::styled(lang.to_string(), Style::default().fg(theme.peach)),
            ]));
        }

        // Stars
        if let Some(stars) = result.stars {
            lines.push(Line::from(vec![
                Span::styled("  ★ Stars: ", Style::default().fg(theme.yellow)),
                Span::styled(
                    format_stars(stars as i64),
                    Style::default().fg(theme.yellow),
                ),
            ]));
        }

        // URL / Repo
        if let Some(url) = &result.url {
            // Try to extract repo info from URL
            let repo_display = if url.contains("github.com") {
                url.trim_start_matches("https://github.com/")
                    .trim_end_matches('/')
                    .to_string()
            } else {
                url.clone()
            };
            lines.push(Line::from(vec![
                Span::styled("  URL: ", Style::default().fg(theme.subtext0)),
                Span::styled(repo_display, Style::default().fg(theme.blue)),
            ]));
        }

        lines.push(Line::from(""));

        // Install section
        if !result.install_options.is_empty() {
            lines.push(Line::from(Span::styled(
                "Install:",
                Style::default()
                    .fg(theme.subtext0)
                    .add_modifier(Modifier::BOLD),
            )));
            for opt in &result.install_options {
                let opt_icon = opt.source.icon();
                lines.push(Line::from(vec![
                    Span::styled(format!("  {} ", opt_icon), Style::default()),
                    Span::styled(
                        opt.install_command.clone(),
                        Style::default().fg(theme.green),
                    ),
                ]));
            }
            lines.push(Line::from(""));
        }

        // Keyboard hints
        lines.push(Line::from(Span::styled(
            "─── Actions ───",
            Style::default().fg(theme.surface1),
        )));
        lines.push(Line::from(vec![
            Span::styled("i", Style::default().fg(theme.mauve)),
            Span::styled(" install  ", Style::default().fg(theme.subtext0)),
            Span::styled("Enter", Style::default().fg(theme.mauve)),
            Span::styled(" view README", Style::default().fg(theme.subtext0)),
        ]));

        lines
    } else {
        vec![
            Line::from(""),
            Line::from(Span::styled(
                "Select a result to see details",
                Style::default().fg(theme.subtext0),
            )),
        ]
    };

    let paragraph = Paragraph::new(content)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme.surface1))
                .title(Span::styled(" Details ", Style::default().fg(theme.text))),
        )
        .wrap(Wrap { trim: true });

    frame.render_widget(paragraph, area);
}
