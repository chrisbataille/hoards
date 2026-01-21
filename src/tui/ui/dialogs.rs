//! Dialog and modal rendering
//!
//! This module handles rendering of modal dialogs and popups.

use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span, Text},
    widgets::{
        Block, Borders, Clear, List, ListItem, ListState, Paragraph, Scrollbar,
        ScrollbarOrientation, ScrollbarState, Wrap,
    },
};

use super::super::app::{App, PendingAction};
use super::super::theme::Theme;
use super::helpers::{ThemedStyleSheet, format_friendly_datetime, format_stars, label_color};
use crate::db::Database;
use crate::icons::source_icon;

/// Helper function to create a centered rectangle
pub fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
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

/// Render the details popup for a selected tool
pub fn render_details_popup(
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

        // Labels (as colored pills)
        if let Some(labels) = app.cache.labels_cache.get(&tool.name)
            && !labels.is_empty()
        {
            let mut spans = vec![Span::styled(
                "Labels: ",
                Style::default().fg(theme.subtext0),
            )];
            for (i, label) in labels.iter().enumerate() {
                if i > 0 {
                    spans.push(Span::raw(" "));
                }
                let color = label_color(label, theme);
                spans.push(Span::styled(
                    format!(" {} ", label),
                    Style::default().fg(theme.base).bg(color),
                ));
            }
            lines.push(Line::from(spans));
        }

        lines.push(Line::from(""));

        // Usage
        if let Some(usage) = app.cache.usage_data.get(&tool.name) {
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
                    Span::styled(
                        format_friendly_datetime(last),
                        Style::default().fg(theme.text),
                    ),
                ]));
            }
        }

        // GitHub
        if let Some(gh) = app.cache.github_cache.get(&tool.name) {
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

/// Render error modal (centered, blocking)
pub fn render_error_modal(frame: &mut Frame, app: &App, theme: &Theme, area: Rect) {
    let modal = match &app.error_modal {
        Some(m) => m,
        None => return,
    };

    let popup_area = centered_rect(60, 40, area);

    // Wrap message text
    let max_line_len = (popup_area.width as usize).saturating_sub(4);
    let wrapped_lines: Vec<Line> = modal
        .message
        .lines()
        .flat_map(|line| {
            if line.len() <= max_line_len {
                vec![Line::from(Span::styled(
                    line.to_string(),
                    Style::default().fg(theme.text),
                ))]
            } else {
                // Simple word wrap
                line.chars()
                    .collect::<Vec<_>>()
                    .chunks(max_line_len)
                    .map(|chunk| {
                        Line::from(Span::styled(
                            chunk.iter().collect::<String>(),
                            Style::default().fg(theme.text),
                        ))
                    })
                    .collect()
            }
        })
        .collect();

    let mut lines = vec![Line::from("")];
    lines.extend(wrapped_lines);
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "Press Enter or Esc to close",
        Style::default().fg(theme.subtext0).italic(),
    )));

    let content = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.red))
            .title(Span::styled(
                format!(" ✗ {} ", modal.title),
                Style::default().fg(theme.red).bold(),
            ))
            .style(Style::default().bg(theme.base)),
    );

    frame.render_widget(Clear, popup_area);
    frame.render_widget(content, popup_area);
}

/// Render README popup with markdown rendering
pub fn render_readme_popup(frame: &mut Frame, app: &mut App, theme: &Theme, area: Rect) {
    // Extract data we need to avoid borrow conflicts
    let (tool_name, content, loading, stored_scroll, links, show_links, selected_link) =
        match &app.readme_popup {
            Some(p) => (
                p.tool_name.clone(),
                p.content.clone(),
                p.loading,
                p.scroll_offset,
                p.links.clone(),
                p.show_links,
                p.selected_link,
            ),
            None => return,
        };

    let popup_area = centered_rect(80, 85, area);

    if loading {
        // Show loading state
        let content = Paragraph::new(vec![
            Line::from(""),
            Line::from(Span::styled(
                "Loading README...",
                Style::default().fg(theme.blue),
            )),
        ])
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme.blue))
                .title(Span::styled(
                    format!(" {} - README ", tool_name),
                    Style::default().fg(theme.blue).bold(),
                ))
                .style(Style::default().bg(theme.base)),
        )
        .alignment(Alignment::Center);

        frame.render_widget(Clear, popup_area);
        frame.render_widget(content, popup_area);
        return;
    }

    // Parse markdown to ratatui Text with themed styling
    // Wrap in catch_unwind because tui-markdown can panic on some edge cases
    let stylesheet = ThemedStyleSheet::from_theme(theme);
    let options = tui_markdown::Options::new(stylesheet);

    let markdown_text = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        tui_markdown::from_str_with_options(&content, &options)
    }));

    let markdown_text = match markdown_text {
        Ok(text) => text,
        Err(_) => {
            // Fallback to plain text if markdown parsing panics
            Text::from(content.clone())
        }
    };

    // Calculate content height for scroll limiting
    let content_height = markdown_text.lines.len() as u16;
    let visible_height = popup_area.height.saturating_sub(2); // Account for borders

    // Clamp scroll offset
    let max_scroll = content_height.saturating_sub(visible_height);
    let scroll_offset = stored_scroll.min(max_scroll);

    // Update scroll offset in app state if it was clamped
    if let Some(p) = &mut app.readme_popup {
        p.scroll_offset = scroll_offset;
    }

    // Build keyboard hints based on link count
    let link_hint = if links.is_empty() {
        vec![]
    } else {
        vec![
            Span::styled(" │ ", Style::default().fg(theme.surface1)),
            Span::styled("o ", Style::default().fg(theme.subtext0)),
            Span::styled(
                format!("links({})", links.len()),
                Style::default().fg(theme.subtext0),
            ),
        ]
    };

    let mut hints = vec![
        Span::styled(" j/k ", Style::default().fg(theme.subtext0)),
        Span::styled("scroll", Style::default().fg(theme.subtext0)),
        Span::styled(" │ ", Style::default().fg(theme.surface1)),
        Span::styled("q/Esc ", Style::default().fg(theme.subtext0)),
        Span::styled("close", Style::default().fg(theme.subtext0)),
    ];
    hints.extend(link_hint);

    let paragraph = Paragraph::new(markdown_text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme.blue))
                .title(Span::styled(
                    format!(" {} - README ", tool_name),
                    Style::default().fg(theme.blue).bold(),
                ))
                .title_bottom(Line::from(hints))
                .style(Style::default().bg(theme.base)),
        )
        .scroll((scroll_offset, 0));

    // Add scrollbar
    let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
        .begin_symbol(Some("▲"))
        .end_symbol(Some("▼"))
        .track_symbol(Some("│"))
        .thumb_symbol("█")
        .track_style(Style::default().fg(theme.surface1))
        .thumb_style(Style::default().fg(theme.blue));

    let mut scrollbar_state =
        ScrollbarState::new(content_height as usize).position(scroll_offset as usize);

    frame.render_widget(Clear, popup_area);
    frame.render_widget(paragraph, popup_area);

    // Render scrollbar in the inner area (excluding borders)
    let scrollbar_area = Rect {
        x: popup_area.x + popup_area.width - 1,
        y: popup_area.y + 1,
        width: 1,
        height: popup_area.height.saturating_sub(2),
    };
    frame.render_stateful_widget(scrollbar, scrollbar_area, &mut scrollbar_state);

    // Render link picker overlay if showing
    if show_links && !links.is_empty() {
        render_link_picker(frame, theme, popup_area, &links, selected_link);
    }
}

/// Render link picker overlay
fn render_link_picker(
    frame: &mut Frame,
    theme: &Theme,
    parent_area: Rect,
    links: &[(String, String)],
    selected: usize,
) {
    // Size the picker based on content - ensure minimum usable size
    let visible_items = links.len().min(15); // Show at most 15 items at a time
    let max_height = (visible_items + 2).max(5) as u16; // +2 for borders, min 5

    // Calculate width based on content
    let content_width = links
        .iter()
        .map(|(text, url)| {
            if text == url {
                text.chars().count()
            } else {
                text.chars().count() + url.chars().count() + 4 // " → "
            }
        })
        .max()
        .unwrap_or(30)
        .min(70) as u16
        + 6; // borders + padding

    // Calculate picker area - use fixed percentages that work better
    let picker_width = content_width
        .max(40)
        .min(parent_area.width.saturating_sub(4));
    let picker_height = max_height.min(parent_area.height.saturating_sub(4));

    let picker_area = Rect {
        x: parent_area.x + (parent_area.width.saturating_sub(picker_width)) / 2,
        y: parent_area.y + (parent_area.height.saturating_sub(picker_height)) / 2,
        width: picker_width,
        height: picker_height,
    };

    // Build list items without individual styling (highlight_style handles selection)
    let items: Vec<ListItem> = links
        .iter()
        .map(|(text, url)| {
            let display = if text == url {
                text.clone()
            } else {
                format!("{} → {}", text, url)
            };

            // Truncate if too long (character-aware for UTF-8 safety)
            let max_chars = (picker_area.width as usize).saturating_sub(4);
            let display = if display.chars().count() > max_chars {
                let truncated: String = display.chars().take(max_chars.saturating_sub(3)).collect();
                format!("{}...", truncated)
            } else {
                display
            };

            ListItem::new(Line::from(Span::styled(
                display,
                Style::default().fg(theme.teal),
            )))
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme.teal))
                .title(Span::styled(
                    format!(" Links [{}/{}] ", selected + 1, links.len()),
                    Style::default().fg(theme.teal).bold(),
                ))
                .title_bottom(Line::from(vec![
                    Span::styled(" j/k ", Style::default().fg(theme.subtext0)),
                    Span::styled("select", Style::default().fg(theme.subtext0)),
                    Span::styled(" │ ", Style::default().fg(theme.surface1)),
                    Span::styled("Enter ", Style::default().fg(theme.subtext0)),
                    Span::styled("open", Style::default().fg(theme.subtext0)),
                    Span::styled(" │ ", Style::default().fg(theme.surface1)),
                    Span::styled("Esc ", Style::default().fg(theme.subtext0)),
                    Span::styled("close", Style::default().fg(theme.subtext0)),
                ]))
                .style(Style::default().bg(theme.base)),
        )
        .highlight_style(
            Style::default()
                .bg(theme.surface0)
                .fg(theme.blue)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("▶ ");

    // Use ListState for proper selection and scrolling
    let mut list_state = ListState::default();
    list_state.select(Some(selected));

    frame.render_widget(Clear, picker_area);
    frame.render_stateful_widget(list, picker_area, &mut list_state);
}

/// Maximum number of items to display in confirmation dialogs before truncating
const MAX_DISPLAY_ITEMS: usize = 5;

/// Render confirmation dialog for install/uninstall/update actions
pub fn render_confirmation_dialog(frame: &mut Frame, app: &App, theme: &Theme, area: Rect) {
    let popup_area = centered_rect(60, 40, area);

    let Some(action) = &app.pending_action else {
        return;
    };

    let (title, lines, color) = match action {
        PendingAction::Install(tasks) => {
            let mut lines = vec![
                Line::from(""),
                Line::from(Span::styled(
                    format!("Install {} tool(s)?", tasks.len()),
                    Style::default().fg(theme.text),
                )),
                Line::from(""),
            ];
            for task in tasks.iter().take(MAX_DISPLAY_ITEMS) {
                lines.push(Line::from(vec![
                    Span::styled("  $ ", Style::default().fg(theme.subtext0)),
                    Span::styled(&task.display_command, Style::default().fg(theme.green)),
                ]));
            }
            if tasks.len() > MAX_DISPLAY_ITEMS {
                lines.push(Line::from(Span::styled(
                    format!("  ... and {} more", tasks.len() - MAX_DISPLAY_ITEMS),
                    Style::default().fg(theme.subtext0),
                )));
            }
            (" Install ", lines, theme.green)
        }
        PendingAction::DiscoverInstall(task) => {
            let lines = vec![
                Line::from(""),
                Line::from(Span::styled(
                    format!("Install {}?", task.name),
                    Style::default().fg(theme.text),
                )),
                Line::from(""),
                Line::from(vec![
                    Span::styled("  $ ", Style::default().fg(theme.subtext0)),
                    Span::styled(&task.display_command, Style::default().fg(theme.green)),
                ]),
            ];
            (" Install ", lines, theme.green)
        }
        PendingAction::Uninstall(tools) => {
            let tool_list = if tools.len() <= MAX_DISPLAY_ITEMS {
                tools.join(", ")
            } else {
                format!(
                    "{}, ... and {} more",
                    tools[..2].join(", "),
                    tools.len() - 2
                )
            };
            let lines = vec![
                Line::from(""),
                Line::from(Span::styled(
                    format!("Uninstall {} tool(s)?", tools.len()),
                    Style::default().fg(theme.text),
                )),
                Line::from(""),
                Line::from(Span::styled(
                    format!("Tools: {}", tool_list),
                    Style::default().fg(theme.red),
                )),
            ];
            (" Uninstall ", lines, theme.red)
        }
        PendingAction::Update(tasks) => {
            let mut lines = vec![
                Line::from(""),
                Line::from(Span::styled(
                    format!("Update {} tool(s)?", tasks.len()),
                    Style::default().fg(theme.text),
                )),
                Line::from(""),
            ];
            for task in tasks.iter().take(MAX_DISPLAY_ITEMS) {
                lines.push(Line::from(vec![
                    Span::styled("  $ ", Style::default().fg(theme.subtext0)),
                    Span::styled(&task.display_command, Style::default().fg(theme.yellow)),
                ]));
            }
            if tasks.len() > MAX_DISPLAY_ITEMS {
                lines.push(Line::from(Span::styled(
                    format!("  ... and {} more", tasks.len() - MAX_DISPLAY_ITEMS),
                    Style::default().fg(theme.subtext0),
                )));
            }
            (" Update ", lines, theme.yellow)
        }
        PendingAction::DiscoverSelectSource(name, options, selected, _) => {
            let mut lines = vec![
                Line::from(""),
                Line::from(Span::styled(
                    format!("Select install source for {}", name),
                    Style::default().fg(theme.text),
                )),
                Line::from(""),
            ];
            for (i, option) in options.iter().enumerate() {
                let is_selected = i == *selected;
                let prefix = if is_selected { "▶ " } else { "  " };
                let style = if is_selected {
                    Style::default().fg(theme.green).bold()
                } else {
                    Style::default().fg(theme.subtext0)
                };
                lines.push(Line::from(vec![
                    Span::styled(prefix, style),
                    Span::styled(option.source.icon(), style),
                    Span::raw(" "),
                    Span::styled(&option.install_command, style),
                ]));
            }
            (" Select Source ", lines, theme.blue)
        }
    };

    // Check if this is source selection (different hints)
    let is_source_selection = matches!(
        app.pending_action,
        Some(PendingAction::DiscoverSelectSource(..))
    );

    // Add confirmation hint
    let mut content_lines = lines;
    content_lines.push(Line::from(""));
    if is_source_selection {
        content_lines.push(Line::from(vec![
            Span::styled("j/k", Style::default().fg(theme.blue).bold()),
            Span::styled(" navigate  ", Style::default().fg(theme.subtext0)),
            Span::styled("Enter", Style::default().fg(theme.green).bold()),
            Span::styled(" select  ", Style::default().fg(theme.subtext0)),
            Span::styled("Esc", Style::default().fg(theme.yellow).bold()),
            Span::styled(" cancel", Style::default().fg(theme.subtext0)),
        ]));
    } else {
        content_lines.push(Line::from(vec![
            Span::styled("Press ", Style::default().fg(theme.subtext0)),
            Span::styled("y", Style::default().fg(theme.green).bold()),
            Span::styled(" to confirm, ", Style::default().fg(theme.subtext0)),
            Span::styled("n", Style::default().fg(theme.red).bold()),
            Span::styled(" or ", Style::default().fg(theme.subtext0)),
            Span::styled("Esc", Style::default().fg(theme.yellow).bold()),
            Span::styled(" to cancel", Style::default().fg(theme.subtext0)),
        ]));
    }

    let content = Text::from(content_lines);

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

/// Render label filter popup
pub fn render_label_filter_popup(
    frame: &mut Frame,
    app: &App,
    db: &Database,
    theme: &Theme,
    area: Rect,
) {
    let popup_area = centered_rect(40, 50, area);

    // Get all labels with counts
    let label_counts = db.get_label_counts().unwrap_or_default();

    let mut lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            "Select a label to filter by:",
            Style::default().fg(theme.text),
        )),
        Line::from(""),
    ];

    // Add "Clear filter" option at the top
    let clear_style = if app.label_filter_selected == 0 {
        Style::default().fg(theme.green).bold()
    } else {
        Style::default().fg(theme.subtext0)
    };
    let clear_prefix = if app.label_filter_selected == 0 {
        "▶ "
    } else {
        "  "
    };
    let clear_suffix = if app.label_filter.is_some() {
        " ✓"
    } else {
        ""
    };
    lines.push(Line::from(vec![
        Span::styled(clear_prefix, clear_style),
        Span::styled(format!("(Clear filter){}", clear_suffix), clear_style),
    ]));

    // Add labels
    for (i, (label, count)) in label_counts.iter().enumerate() {
        let is_selected = app.label_filter_selected == i + 1;
        let is_active = app.label_filter.as_ref() == Some(label);

        let style = if is_selected {
            Style::default().fg(theme.green).bold()
        } else if is_active {
            Style::default().fg(theme.yellow)
        } else {
            Style::default().fg(theme.subtext0)
        };

        let prefix = if is_selected { "▶ " } else { "  " };
        let suffix = if is_active { " ✓" } else { "" };

        lines.push(Line::from(vec![
            Span::styled(prefix, style),
            Span::styled(format!("{} ({}){}", label, count, suffix), style),
        ]));
    }

    // Add hint
    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled("j/k", Style::default().fg(theme.blue).bold()),
        Span::styled(" navigate  ", Style::default().fg(theme.subtext0)),
        Span::styled("Enter", Style::default().fg(theme.green).bold()),
        Span::styled(" select  ", Style::default().fg(theme.subtext0)),
        Span::styled("Esc", Style::default().fg(theme.yellow).bold()),
        Span::styled(" close", Style::default().fg(theme.subtext0)),
    ]));

    let content = Text::from(lines);

    let popup = Paragraph::new(content)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme.blue))
                .title(Span::styled(
                    " Label Filter ",
                    Style::default().fg(theme.blue).bold(),
                ))
                .style(Style::default().bg(theme.base)),
        )
        .alignment(Alignment::Left)
        .wrap(Wrap { trim: true });

    frame.render_widget(Clear, popup_area);
    frame.render_widget(popup, popup_area);
}

/// Render label edit popup
pub fn render_label_edit_popup(frame: &mut Frame, app: &App, theme: &Theme, area: Rect) {
    let popup_area = centered_rect(50, 60, area);

    let tool_name = app.label_edit_tool.as_deref().unwrap_or("Unknown");

    let mut lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            format!("Edit labels for: {}", tool_name),
            Style::default().fg(theme.text),
        )),
        Line::from(""),
    ];

    // Input field for new label
    let input_selected = app.label_edit_selected == 0;
    let input_style = if input_selected {
        Style::default().fg(theme.green).bold()
    } else {
        Style::default().fg(theme.subtext0)
    };
    let input_prefix = if input_selected { "▶ " } else { "  " };
    let cursor = if input_selected { "▌" } else { "" };

    lines.push(Line::from(vec![
        Span::styled(input_prefix, input_style),
        Span::styled("Add: ", input_style),
        Span::styled(
            format!("{}{}", app.label_edit_input, cursor),
            Style::default().fg(theme.text).bg(if input_selected {
                theme.surface0
            } else {
                theme.base
            }),
        ),
    ]));

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "Current labels:",
        Style::default().fg(theme.subtext0),
    )));

    // Show existing labels
    if app.label_edit_labels.is_empty() {
        lines.push(Line::from(Span::styled(
            "  (no labels)",
            Style::default().fg(theme.subtext0).italic(),
        )));
    } else {
        for (i, label) in app.label_edit_labels.iter().enumerate() {
            let is_selected = app.label_edit_selected == i + 1;
            let style = if is_selected {
                Style::default().fg(theme.yellow).bold()
            } else {
                Style::default().fg(theme.teal)
            };
            let prefix = if is_selected { "▶ " } else { "  " };
            let suffix = if is_selected { "  [d] to delete" } else { "" };

            lines.push(Line::from(vec![
                Span::styled(prefix, style),
                Span::styled(label.clone(), style),
                Span::styled(suffix, Style::default().fg(theme.red)),
            ]));
        }
    }

    // Add hints
    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled("Tab", Style::default().fg(theme.blue).bold()),
        Span::styled(" navigate  ", Style::default().fg(theme.subtext0)),
        Span::styled("Enter", Style::default().fg(theme.green).bold()),
        Span::styled(" add  ", Style::default().fg(theme.subtext0)),
        Span::styled("d", Style::default().fg(theme.red).bold()),
        Span::styled(" delete  ", Style::default().fg(theme.subtext0)),
        Span::styled("Esc", Style::default().fg(theme.yellow).bold()),
        Span::styled(" close", Style::default().fg(theme.subtext0)),
    ]));

    let content = Text::from(lines);

    let popup = Paragraph::new(content)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme.mauve))
                .title(Span::styled(
                    " Edit Labels ",
                    Style::default().fg(theme.mauve).bold(),
                ))
                .style(Style::default().bg(theme.base)),
        )
        .alignment(Alignment::Left)
        .wrap(Wrap { trim: true });

    frame.render_widget(Clear, popup_area);
    frame.render_widget(popup, popup_area);
}
