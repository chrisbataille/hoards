//! UI rendering for the TUI

use chrono::{DateTime, Datelike, Local, Utc};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Margin, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{
        Block, Borders, Clear, List, ListItem, ListState, Paragraph, Scrollbar,
        ScrollbarOrientation, ScrollbarState, Tabs, Wrap,
    },
};

use super::app::{App, InputMode, Tab, fuzzy_match_positions};
use super::theme::Theme;

/// Custom stylesheet for markdown rendering that uses the TUI theme
#[derive(Clone)]
struct ThemedStyleSheet {
    heading_color: Color,
    code_color: Color,
    link_color: Color,
    blockquote_color: Color,
    meta_color: Color,
}

impl ThemedStyleSheet {
    fn from_theme(theme: &Theme) -> Self {
        Self {
            heading_color: theme.blue,
            code_color: theme.green,
            link_color: theme.teal,
            blockquote_color: theme.subtext0,
            meta_color: theme.subtext0,
        }
    }
}

impl tui_markdown::StyleSheet for ThemedStyleSheet {
    fn heading(&self, level: u8) -> Style {
        let modifier = if level == 1 {
            Modifier::BOLD | Modifier::UNDERLINED
        } else {
            Modifier::BOLD
        };
        Style::default()
            .fg(self.heading_color)
            .add_modifier(modifier)
    }

    fn code(&self) -> Style {
        Style::default().fg(self.code_color)
    }

    fn link(&self) -> Style {
        Style::default()
            .fg(self.link_color)
            .add_modifier(Modifier::UNDERLINED)
    }

    fn blockquote(&self) -> Style {
        Style::default()
            .fg(self.blockquote_color)
            .add_modifier(Modifier::ITALIC)
    }

    fn heading_meta(&self) -> Style {
        Style::default().fg(self.meta_color)
    }

    fn metadata_block(&self) -> Style {
        Style::default().fg(self.meta_color)
    }
}

/// Get a consistent color for a label based on its hash
fn label_color(label: &str, theme: &Theme) -> Color {
    let colors = [
        theme.blue,
        theme.green,
        theme.yellow,
        theme.mauve,
        theme.peach,
        theme.teal,
        theme.red,
    ];
    let hash: usize = label.bytes().map(|b| b as usize).sum();
    colors[hash % colors.len()]
}

/// Format an RFC3339 datetime string to a friendly local time format
/// e.g., "Today at 3:45 PM", "Yesterday at 10:30 AM", "Jan 15 at 2:00 PM", "Jan 15, 2025"
fn format_friendly_datetime(rfc3339: &str) -> String {
    let Ok(dt) = DateTime::parse_from_rfc3339(rfc3339) else {
        return rfc3339.to_string(); // Fallback to raw if parsing fails
    };

    let local_dt = dt.with_timezone(&Local);
    let now = Local::now();
    let today = now.date_naive();
    let dt_date = local_dt.date_naive();

    let time_str = local_dt.format("%-I:%M %p").to_string();

    if dt_date == today {
        format!("Today at {}", time_str)
    } else if dt_date == today.pred_opt().unwrap_or(today) {
        format!("Yesterday at {}", time_str)
    } else if (today - dt_date).num_days() < 7 {
        // Within a week: "Mon at 3:45 PM"
        format!("{} at {}", local_dt.format("%a"), time_str)
    } else if local_dt.year() == now.year() {
        // Same year: "Jan 15 at 3:45 PM"
        format!("{} at {}", local_dt.format("%b %-d"), time_str)
    } else {
        // Different year: "Jan 15, 2025"
        local_dt.format("%b %-d, %Y").to_string()
    }
}

/// Format a timestamp as relative time (e.g., "5m", "2h", "3d")
fn format_relative_time(dt: &DateTime<Utc>) -> String {
    let now = Utc::now();
    let duration = now.signed_duration_since(*dt);

    if duration.num_seconds() < 60 {
        "now".to_string()
    } else if duration.num_minutes() < 60 {
        format!("{}m", duration.num_minutes())
    } else if duration.num_hours() < 24 {
        format!("{}h", duration.num_hours())
    } else if duration.num_days() < 7 {
        format!("{}d", duration.num_days())
    } else if duration.num_weeks() < 4 {
        format!("{}w", duration.num_weeks())
    } else {
        format!("{}mo", duration.num_days() / 30)
    }
}

/// Create spans for a tool name with fuzzy match highlighting
fn highlight_matches(
    name: &str,
    query: &str,
    normal: Color,
    highlight: Color,
) -> Vec<Span<'static>> {
    if query.is_empty() {
        return vec![Span::styled(name.to_string(), Style::default().fg(normal))];
    }

    if let Some((_, positions)) = fuzzy_match_positions(query, name) {
        let chars: Vec<char> = name.chars().collect();
        let mut spans = Vec::new();
        let mut current_span = String::new();
        let mut in_highlight = false;

        for (i, c) in chars.iter().enumerate() {
            let should_highlight = positions.contains(&i);

            if should_highlight != in_highlight {
                // State changed, emit current span
                if !current_span.is_empty() {
                    let color = if in_highlight { highlight } else { normal };
                    spans.push(Span::styled(
                        current_span.clone(),
                        Style::default().fg(color),
                    ));
                    current_span.clear();
                }
                in_highlight = should_highlight;
            }
            current_span.push(*c);
        }

        // Emit final span
        if !current_span.is_empty() {
            let color = if in_highlight { highlight } else { normal };
            spans.push(Span::styled(current_span, Style::default().fg(color)));
        }

        spans
    } else {
        vec![Span::styled(name.to_string(), Style::default().fg(normal))]
    }
}
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

    // Store areas for mouse interaction
    app.set_tab_area(chunks[0].x, chunks[0].y, chunks[0].width, chunks[0].height);

    render_header(frame, app, &theme, chunks[0]);
    render_body(frame, app, db, &theme, chunks[1]);
    render_footer(frame, app, &theme, chunks[2]);

    // Render overlays (in order of priority)
    if app.show_help {
        render_help_overlay(frame, &theme, area);
    }

    if app.show_config_menu {
        render_config_menu(frame, app, &theme, area);
    }

    if app.show_details_popup {
        render_details_popup(frame, app, db, &theme, area);
    }

    // README popup
    if app.has_readme_popup() {
        render_readme_popup(frame, app, &theme, area);
    }

    // Confirmation dialog takes high priority
    if app.has_pending_action() {
        render_confirmation_dialog(frame, app, &theme, area);
    }

    // Error modal blocks all input
    if app.has_error_modal() {
        render_error_modal(frame, app, &theme, area);
    }

    // Loading overlay takes absolute highest priority
    if app.has_background_op() {
        render_loading_overlay(frame, app, &theme, area);
    }

    // Toast notifications always on top (but don't block input)
    render_notifications(frame, app, &theme, area);
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
    if app.tab == super::app::Tab::Bundles {
        if area.width >= min_width_for_split {
            let chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
                .split(area);

            // Store list area for mouse interaction
            app.set_list_area(chunks[0].x, chunks[0].y, chunks[0].width, chunks[0].height);
            render_bundle_list(frame, app, theme, chunks[0]);
            render_bundle_details(frame, app, db, theme, chunks[1]);
        } else {
            app.set_list_area(area.x, area.y, area.width, area.height);
            render_bundle_list(frame, app, theme, area);
        }
        return;
    }

    // Discover tab has its own rendering (needs mutable access for list area)
    if app.tab == super::app::Tab::Discover {
        render_discover_tab(frame, app, theme, area);
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
        render_tool_list(frame, app, theme, chunks[0]);
        render_details(frame, app, db, theme, chunks[1]);
    } else {
        // Narrow terminal: list only (details on Enter in future)
        app.set_list_area(area.x, area.y, area.width, area.height);
        render_tool_list(frame, app, theme, area);
    }
}

// ============================================================================
// Tool List Rendering Helpers
// ============================================================================

/// Render empty state for Updates tab when updates haven't been checked
fn render_updates_empty_state(frame: &mut Frame, app: &App, theme: &Theme, area: Rect) {
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
}

/// Build extra info and sparkline for a tool item
fn build_tool_extra_info(app: &App, tool: &crate::models::Tool) -> (String, String) {
    if app.tab == super::app::Tab::Updates {
        let info = if let Some(update) = app.get_update(&tool.name) {
            format!(" {} → {}", update.current, update.latest)
        } else {
            String::new()
        };
        (info, String::new())
    } else {
        let usage = app.get_usage(&tool.name).map(|u| u.use_count).unwrap_or(0);
        let daily = app.cache.daily_usage.get(&tool.name);
        let spark_str = daily.map(|d| sparkline(d)).unwrap_or_default();
        let info = if usage > 0 {
            format!(" ({usage})")
        } else {
            String::new()
        };
        (info, spark_str)
    }
}

/// Get status indicator for a tool based on its state
fn get_tool_status_indicator(
    app: &App,
    tool: &crate::models::Tool,
    theme: &Theme,
) -> (&'static str, Color) {
    if app.tab == super::app::Tab::Updates {
        ("↑", theme.yellow)
    } else if !tool.is_installed {
        ("○", theme.subtext0)
    } else {
        let usage = app.get_usage(&tool.name);
        let use_count = usage.as_ref().map(|u| u.use_count).unwrap_or(0);
        let last_used = usage.as_ref().and_then(|u| u.last_used.as_deref());
        health_indicator(last_used, use_count, theme)
    }
}

/// Build a single tool list item
fn build_tool_list_item(
    app: &App,
    tool: &crate::models::Tool,
    index: usize,
    theme: &Theme,
) -> ListItem<'static> {
    // Selection checkbox
    let selected = app.is_selected(&tool.name);
    let checkbox = if selected { "☑" } else { "☐" };
    let checkbox_color = if selected { theme.blue } else { theme.surface1 };

    // Source icon
    let src_icon = source_icon(&tool.source.to_string());

    // Extra info (usage or version)
    let (extra_info, spark) = build_tool_extra_info(app, tool);
    let (status, status_color) = get_tool_status_indicator(app, tool, theme);
    let extra_color = if app.tab == super::app::Tab::Updates {
        theme.yellow
    } else {
        theme.subtext0
    };

    // Sparkline span
    let spark_span = if spark.is_empty() {
        Span::raw("")
    } else {
        Span::styled(format!(" {spark}"), Style::default().fg(theme.teal))
    };

    // GitHub stars
    let stars_span = app
        .cache
        .github_cache
        .get(&tool.name)
        .filter(|gh| gh.stars > 0)
        .map(|gh| {
            Span::styled(
                format!(" ★ {}", format_stars(gh.stars)),
                Style::default().fg(theme.yellow),
            )
        })
        .unwrap_or_else(|| Span::raw(""));

    // Build content spans
    let mut spans = vec![
        Span::styled(format!("{checkbox} "), Style::default().fg(checkbox_color)),
        Span::styled(format!("{src_icon} "), Style::default()),
        Span::styled(format!("{status} "), Style::default().fg(status_color)),
    ];
    spans.extend(highlight_matches(
        &tool.name,
        &app.search_query,
        theme.text,
        theme.yellow,
    ));
    spans.push(stars_span);
    spans.push(Span::styled(extra_info, Style::default().fg(extra_color)));
    spans.push(spark_span);

    let style = if index == app.selected_index {
        Style::default().bg(theme.surface0)
    } else {
        Style::default()
    };

    ListItem::new(Line::from(spans)).style(style)
}

/// Build the list title with count and selection info
fn build_tool_list_title(app: &App) -> String {
    let selection_info = if app.selection_count() > 0 {
        format!(" ({} selected)", app.selection_count())
    } else {
        String::new()
    };

    if app.tab == super::app::Tab::Updates {
        format!(" Updates [{}]{} ", app.tools.len(), selection_info)
    } else {
        format!(
            " Tools [{}]{} ({}↕) ",
            app.tools.len(),
            selection_info,
            app.sort_by.label()
        )
    }
}

fn render_tool_list(frame: &mut Frame, app: &mut App, theme: &Theme, area: Rect) {
    // Handle empty state for Updates tab
    if app.tab == super::app::Tab::Updates && !app.updates_checked {
        render_updates_empty_state(frame, app, theme, area);
        return;
    }

    // Build list items
    let items: Vec<ListItem> = app
        .tools
        .iter()
        .enumerate()
        .map(|(i, tool)| build_tool_list_item(app, tool, i, theme))
        .collect();

    let title_text = build_tool_list_title(app);

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

    // Scroll handling
    let visible_height = area.height.saturating_sub(2) as usize;
    let offset = if visible_height > 0 {
        let offset = app.selected_index.saturating_sub(visible_height / 2);
        *state.offset_mut() = offset;
        app.list_offset = offset;
        offset
    } else {
        0
    };

    frame.render_stateful_widget(list, area, &mut state);

    // Scrollbar
    if app.tools.len() > visible_height {
        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(Some("↑"))
            .end_symbol(Some("↓"))
            .track_symbol(Some("│"))
            .thumb_symbol("█");

        let mut scrollbar_state = ScrollbarState::new(app.tools.len()).position(offset);
        frame.render_stateful_widget(scrollbar, area, &mut scrollbar_state);
    }
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

        // Usage statistics
        if let Some(usage) = app.cache.usage_data.get(&tool.name) {
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
                    Span::styled(
                        format_friendly_datetime(last),
                        Style::default().fg(theme.text),
                    ),
                ]));
            }
            lines.push(Line::from(""));
        }

        // GitHub info (already fetched above)
        if let Some(gh) = app.cache.github_cache.get(&tool.name) {
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
            let usage = app.cache.usage_data.get(&tool.name);
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

            let style = if i == app.bundles.selected {
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
    state.select(Some(app.bundles.selected));

    frame.render_stateful_widget(list, area, &mut state);

    // Render scrollbar if list is longer than visible area
    let visible_height = area.height.saturating_sub(2) as usize;
    if app.bundles.len() > visible_height {
        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(Some("↑"))
            .end_symbol(Some("↓"))
            .track_symbol(Some("│"))
            .thumb_symbol("█");

        let mut scrollbar_state =
            ScrollbarState::new(app.bundles.len()).position(app.bundles.selected);

        frame.render_stateful_widget(scrollbar, area, &mut scrollbar_state);
    }
}

fn render_bundle_details(frame: &mut Frame, app: &App, db: &Database, theme: &Theme, area: Rect) {
    let content = if let Some(bundle) = app.bundles.get(app.bundles.selected) {
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

        // Categorize tools: untracked (not in db), tracked but not installed, installed
        let mut untracked = 0;
        let mut not_installed = 0;

        for name in &bundle.tools {
            match db.get_tool_by_name(name).ok().flatten() {
                None => untracked += 1,
                Some(t) if !t.is_installed => not_installed += 1,
                _ => {}
            }
        }

        let missing = untracked + not_installed;

        // Action hints
        if missing > 0 {
            lines.push(Line::from(Span::styled(
                format!("Press 'i' to install {} missing tool(s)", missing),
                Style::default().fg(theme.green),
            )));
        } else {
            lines.push(Line::from(Span::styled(
                "All tools installed ✓",
                Style::default().fg(theme.green),
            )));
        }

        if untracked > 0 {
            lines.push(Line::from(Span::styled(
                format!(
                    "Press 'a' to add {} untracked tool(s) to Available",
                    untracked
                ),
                Style::default().fg(theme.blue),
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

fn render_discover_tab(frame: &mut Frame, app: &mut App, theme: &Theme, area: Rect) {
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
    use super::app::InputMode;

    let is_search_mode =
        app.input_mode == InputMode::Search && app.tab == super::app::Tab::Discover;

    // Get available sources (config-aware)
    let available_sources = app.get_available_discover_sources();

    // Calculate widths dynamically
    // AI toggle: "[x]🤖" = ~6 chars
    let ai_width = 7u16;

    // Filter chips: each is "F1[x]🦀 " (Fn + checkbox + icon + space) = ~8 chars each
    let filter_chips_width: u16 = available_sources
        .iter()
        .enumerate()
        .map(|(idx, (_, icon, _))| {
            // "Fn" (2-3 chars) + "[x]" (3) + icon + space
            let fkey_width = if idx + 1 >= 10 { 3 } else { 2 }; // F1 vs F10
            (fkey_width + 3 + icon.chars().count() + 1) as u16
        })
        .sum::<u16>()
        + 4; // borders + padding

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

fn render_discover_list(frame: &mut Frame, app: &mut App, theme: &Theme, area: Rect) {
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

fn render_discover_details(frame: &mut Frame, app: &App, theme: &Theme, area: Rect) {
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

        // Source
        let icon = result.source.icon();
        lines.push(Line::from(vec![
            Span::styled("Source: ", Style::default().fg(theme.subtext0)),
            Span::styled(
                format!("{} {:?}", icon, result.source),
                Style::default().fg(theme.peach),
            ),
        ]));

        // Stars
        if let Some(stars) = result.stars {
            lines.push(Line::from(vec![
                Span::styled("Stars: ", Style::default().fg(theme.subtext0)),
                Span::styled(
                    format!("★ {}", format_stars(stars as i64)),
                    Style::default().fg(theme.yellow),
                ),
            ]));
        }

        // URL
        if let Some(url) = &result.url {
            lines.push(Line::from(vec![
                Span::styled("URL: ", Style::default().fg(theme.subtext0)),
                Span::styled(url.clone(), Style::default().fg(theme.blue)),
            ]));
        }

        lines.push(Line::from(""));

        // Install options
        if !result.install_options.is_empty() {
            lines.push(Line::from(Span::styled(
                "Install commands:",
                Style::default().fg(theme.subtext0),
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
        }

        lines.push(Line::from(""));
        lines.push(Line::from(""));

        // Keyboard hints
        lines.push(Line::from(Span::styled(
            "─── Actions ───",
            Style::default().fg(theme.surface1),
        )));
        lines.push(Line::from(vec![
            Span::styled("i", Style::default().fg(theme.mauve)),
            Span::styled(" install  ", Style::default().fg(theme.subtext0)),
            Span::styled("Enter", Style::default().fg(theme.mauve)),
            Span::styled(" open URL", Style::default().fg(theme.subtext0)),
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

// ============================================================================
// Footer Rendering Helpers
// ============================================================================

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
    if app.tab == super::app::Tab::Discover {
        return build_discover_footer(app, theme);
    }

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

    if app.selection_count() > 0 {
        spans.push(Span::styled(" │ ", Style::default().fg(theme.surface1)));
        spans.push(Span::styled(
            format!("{} selected", app.selection_count()),
            Style::default().fg(theme.blue),
        ));
    } else if !app.search_query.is_empty() || app.source_filter.is_some() || app.favorites_only {
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
        if app.source_filter.is_some() || !app.search_query.is_empty() {
            spans.push(Span::styled(" ", Style::default()));
        }
    }
    if let Some(ref source) = app.source_filter {
        spans.push(Span::styled("src:", Style::default().fg(theme.mauve)));
        spans.push(Span::styled(
            source.clone(),
            Style::default().fg(theme.text),
        ));
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

fn render_footer(frame: &mut Frame, app: &App, theme: &Theme, area: Rect) {
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

// ============================================================================
// Config Menu Rendering Helpers
// ============================================================================

/// Create a radio button line for config menu
fn make_radio_line<'a>(selected: bool, focused: bool, label: String, theme: &Theme) -> Line<'a> {
    let bullet = if selected { "●" } else { "○" };
    let style = if focused {
        Style::default().fg(theme.blue).bold()
    } else if selected {
        Style::default().fg(theme.green)
    } else {
        Style::default().fg(theme.subtext0)
    };
    Line::from(vec![
        Span::styled(format!("  {} ", bullet), style),
        Span::styled(label, style),
    ])
}

/// Create a checkbox line for config menu
fn make_checkbox_line<'a>(checked: bool, focused: bool, label: String, theme: &Theme) -> Line<'a> {
    let mark = if checked { "☑" } else { "☐" };
    let style = if focused {
        Style::default().fg(theme.blue).bold()
    } else if checked {
        Style::default().fg(theme.green)
    } else {
        Style::default().fg(theme.subtext0)
    };
    Line::from(vec![
        Span::styled(format!("  {} ", mark), style),
        Span::styled(label, style),
    ])
}

/// Create a section header line
fn make_section_header<'a>(title: &'static str, focused: bool, theme: &Theme) -> Line<'a> {
    Line::from(Span::styled(
        title,
        if focused {
            Style::default().fg(theme.blue).bold()
        } else {
            Style::default().fg(theme.text).bold()
        },
    ))
}

/// Create a dimmed section header for disabled sections
fn make_section_header_dimmed<'a>(title: &'static str, theme: &Theme) -> Line<'a> {
    Line::from(Span::styled(
        title,
        Style::default().fg(theme.subtext0).italic(),
    ))
}

/// Render AI Provider section lines
fn render_config_ai_section(
    state: &super::app::ConfigMenuState,
    theme: &Theme,
) -> Vec<Line<'static>> {
    use super::app::ConfigSection;
    use crate::config::AiProvider;

    let ai_focused = state.section == ConfigSection::AiProvider;
    let mut lines = vec![make_section_header("AI Provider", ai_focused, theme)];

    for (i, provider) in AiProvider::all().iter().enumerate() {
        let label = match provider {
            AiProvider::None => "None (disabled)",
            AiProvider::Claude => "Claude",
            AiProvider::Gemini => "Gemini",
            AiProvider::Codex => "Codex",
            AiProvider::Opencode => "Opencode",
        };
        let selected = i == state.ai_selected;
        let focused = ai_focused && selected;
        lines.push(make_radio_line(selected, focused, label.to_string(), theme));
    }

    lines.push(Line::from(""));
    lines
}

/// Render Claude Model section lines (only shown when Claude is selected)
fn render_config_claude_model_section(
    state: &super::app::ConfigMenuState,
    theme: &Theme,
) -> Vec<Line<'static>> {
    use super::app::ConfigSection;
    use crate::config::AiProvider;

    let claude_focused = state.section == ConfigSection::ClaudeModel;

    // Check if Claude is selected as the AI provider
    let claude_provider_index = AiProvider::all()
        .iter()
        .position(|p| *p == AiProvider::Claude)
        .unwrap_or(1);
    let is_claude_selected = state.ai_selected == claude_provider_index;

    // Only show full section if Claude is selected as provider
    if !is_claude_selected {
        return vec![
            make_section_header_dimmed("Claude Model (select Claude above)", theme),
            Line::from(""),
        ];
    }

    let mut lines = vec![make_section_header("Claude Model", claude_focused, theme)];

    let models = [
        ("Haiku", "Fast and cost-effective"),
        ("Sonnet", "Balanced intelligence"),
        ("Opus", "Most capable"),
    ];

    for (i, (name, desc)) in models.iter().enumerate() {
        let selected = i == state.claude_model_selected;
        let focused = claude_focused && selected;
        let label = format!("{} - {}", name, desc);
        lines.push(make_radio_line(selected, focused, label, theme));
    }

    lines.push(Line::from(""));
    lines
}

/// Render Theme section lines
fn render_config_theme_section(
    state: &super::app::ConfigMenuState,
    theme: &Theme,
) -> Vec<Line<'static>> {
    use super::app::ConfigSection;
    use crate::config::TuiTheme;

    let theme_focused = state.section == ConfigSection::Theme;
    let mut lines = vec![make_section_header("Theme", theme_focused, theme)];

    let builtin_themes = [
        TuiTheme::CatppuccinMocha,
        TuiTheme::CatppuccinLatte,
        TuiTheme::Dracula,
        TuiTheme::Nord,
        TuiTheme::TokyoNight,
        TuiTheme::Gruvbox,
    ];

    for (i, t) in builtin_themes.iter().enumerate() {
        let selected = i == state.theme_selected;
        let focused = theme_focused && selected;
        lines.push(make_radio_line(selected, focused, t.to_string(), theme));
    }

    // Custom theme option
    let custom_exists = super::theme::CustomTheme::exists();
    let custom_selected = state.theme_selected == 6;
    let custom_focused = theme_focused && custom_selected;
    let custom_label = if custom_exists {
        "Custom".to_string()
    } else {
        "Custom (use :create-theme to create)".to_string()
    };
    lines.push(make_radio_line(
        custom_selected,
        custom_focused,
        custom_label,
        theme,
    ));

    // Show file path hint when Custom is selected
    if custom_selected && let Ok(path) = super::theme::CustomTheme::file_path() {
        lines.push(Line::from(Span::styled(
            format!("    Edit: {}", path.display()),
            Style::default().fg(theme.subtext0).italic(),
        )));
    }

    lines.push(Line::from(""));
    lines
}

/// Render Package Managers section lines
fn render_config_sources_section(
    state: &super::app::ConfigMenuState,
    theme: &Theme,
) -> Vec<Line<'static>> {
    use super::app::ConfigSection;
    use crate::config::SourcesConfig;

    let sources_focused = state.section == ConfigSection::Sources;
    let mut lines = vec![make_section_header(
        "Package Managers",
        sources_focused,
        theme,
    )];

    let source_names = SourcesConfig::all_sources();
    let source_labels = ["Cargo", "Apt", "Pip", "npm", "Brew", "Flatpak", "Manual"];
    for (i, (&name, label)) in source_names.iter().zip(source_labels.iter()).enumerate() {
        let checked = state.sources.is_enabled(name);
        let focused = sources_focused && i == state.source_focused;
        lines.push(make_checkbox_line(
            checked,
            focused,
            label.to_string(),
            theme,
        ));
    }

    lines.push(Line::from(""));
    lines
}

/// Render Usage Tracking section lines
fn render_config_usage_section(
    state: &super::app::ConfigMenuState,
    theme: &Theme,
) -> Vec<Line<'static>> {
    use super::app::ConfigSection;

    let usage_focused = state.section == ConfigSection::UsageMode;
    let mut lines = vec![make_section_header("Usage Tracking", usage_focused, theme)];

    lines.push(make_radio_line(
        state.usage_selected == 0,
        usage_focused && state.usage_selected == 0,
        "Scan (manual)".to_string(),
        theme,
    ));
    lines.push(make_radio_line(
        state.usage_selected == 1,
        usage_focused && state.usage_selected == 1,
        "Hook (real-time)".to_string(),
        theme,
    ));

    lines.push(Line::from(""));
    lines
}

/// Render Buttons section line
fn render_config_buttons_section(
    state: &super::app::ConfigMenuState,
    theme: &Theme,
) -> Line<'static> {
    use super::app::ConfigSection;

    let buttons_focused = state.section == ConfigSection::Buttons;
    let save_style = if buttons_focused && state.button_focused == 0 {
        Style::default().fg(theme.base).bg(theme.green).bold()
    } else {
        Style::default().fg(theme.green)
    };
    let cancel_style = if buttons_focused && state.button_focused == 1 {
        Style::default().fg(theme.base).bg(theme.red).bold()
    } else {
        Style::default().fg(theme.red)
    };

    Line::from(vec![
        Span::styled("  ", Style::default()),
        Span::styled(" Save ", save_style),
        Span::styled("  ", Style::default()),
        Span::styled(" Cancel ", cancel_style),
    ])
}

/// Render the config menu popup
fn render_config_menu(frame: &mut Frame, app: &mut App, theme: &Theme, area: Rect) {
    let popup_area = centered_rect(60, 85, area);
    app.last_config_popup_area = Some((
        popup_area.x,
        popup_area.y,
        popup_area.width,
        popup_area.height,
    ));

    let state = &app.config_menu;

    // Build content lines from section helpers
    let mut lines = Vec::new();
    lines.extend(render_config_ai_section(state, theme));
    lines.extend(render_config_claude_model_section(state, theme));
    lines.extend(render_config_theme_section(state, theme));
    lines.extend(render_config_sources_section(state, theme));
    lines.extend(render_config_usage_section(state, theme));
    lines.push(render_config_buttons_section(state, theme));

    let total_lines = lines.len();
    let content_height = popup_area.height.saturating_sub(3) as usize;
    let scroll_offset = state
        .scroll_offset
        .min(total_lines.saturating_sub(content_height));

    let config_widget = Paragraph::new(lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme.mauve))
                .title(Span::styled(
                    " Configuration ",
                    Style::default().fg(theme.mauve).bold(),
                ))
                .title_bottom(Line::from(vec![
                    Span::styled(" s", Style::default().fg(theme.green).bold()),
                    Span::styled(" Save ", Style::default().fg(theme.subtext0)),
                    Span::styled("Esc", Style::default().fg(theme.red).bold()),
                    Span::styled(" Cancel ", Style::default().fg(theme.subtext0)),
                    Span::styled("↑↓", Style::default().fg(theme.blue).bold()),
                    Span::styled(" Nav ", Style::default().fg(theme.subtext0)),
                    Span::styled("Tab", Style::default().fg(theme.blue).bold()),
                    Span::styled(" Section ", Style::default().fg(theme.subtext0)),
                ]))
                .style(Style::default().bg(theme.base)),
        )
        .scroll((scroll_offset as u16, 0))
        .wrap(Wrap { trim: true });

    frame.render_widget(Clear, popup_area);
    frame.render_widget(config_widget, popup_area);

    // Render scrollbar if needed
    let max_scroll = total_lines.saturating_sub(content_height);
    if max_scroll > 0 {
        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(Some("▲"))
            .end_symbol(Some("▼"))
            .track_symbol(Some("│"))
            .thumb_symbol("█");

        let mut scrollbar_state = ScrollbarState::new(max_scroll).position(scroll_offset);
        let scrollbar_area = Rect {
            x: popup_area.x + popup_area.width - 2,
            y: popup_area.y + 1,
            width: 1,
            height: popup_area.height.saturating_sub(2),
        };

        frame.render_stateful_widget(scrollbar, scrollbar_area, &mut scrollbar_state);
    }
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

/// Render toast notifications in top-right corner
fn render_notifications(frame: &mut Frame, app: &App, theme: &Theme, area: Rect) {
    if app.notifications.is_empty() {
        return;
    }

    use super::app::NotificationLevel;

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

/// Render error modal (centered, blocking)
fn render_error_modal(frame: &mut Frame, app: &App, theme: &Theme, area: Rect) {
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
fn render_readme_popup(frame: &mut Frame, app: &mut App, theme: &Theme, area: Rect) {
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
