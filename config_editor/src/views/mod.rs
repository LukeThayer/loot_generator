pub mod affix_pools;
pub mod affixes;
pub mod base_types;
pub mod currencies;
pub mod uniques;

use crate::app::{App, Focus};
use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
};

/// Helper to render a field line in edit form
pub fn render_field_line(
    label: &str,
    value: &str,
    field_idx: usize,
    app: &App,
    cursor_pos: Option<usize>,
) -> Line<'static> {
    let is_focused = matches!(app.focus, Focus::Field(i) if i == field_idx);
    let state = app.current_view_state();
    let is_current = state.field_index == field_idx;

    let label_style = Style::default().fg(Color::Gray);
    let value_style = if is_current {
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default()
    };

    let focus_marker = if is_focused { "> " } else { "  " };

    // For focused fields, show cursor
    let display_value = if is_focused {
        if let Some(pos) = cursor_pos {
            let mut v = value.to_string();
            if pos <= v.len() {
                v.insert(pos, '|');
            }
            v
        } else {
            format!("{}|", value)
        }
    } else {
        value.to_string()
    };

    Line::from(vec![
        Span::styled(focus_marker.to_string(), Style::default().fg(Color::Cyan)),
        Span::styled(format!("{}: ", label), label_style),
        Span::styled(display_value, value_style),
    ])
}

/// Helper to render a section header
pub fn render_section_header(title: &str) -> Line<'static> {
    Line::from(Span::styled(
        title.to_string(),
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
    ))
}

/// Helper to render a nested field indicator
pub fn render_nested_field(
    label: &str,
    summary: &str,
    field_idx: usize,
    app: &App,
) -> Line<'static> {
    let state = app.current_view_state();
    let is_current = state.field_index == field_idx;

    let label_style = Style::default().fg(Color::Gray);
    let value_style = if is_current {
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let focus_marker = if is_current { "> " } else { "  " };

    Line::from(vec![
        Span::styled(focus_marker.to_string(), Style::default().fg(Color::Cyan)),
        Span::styled(format!("{}: ", label), label_style),
        Span::styled(summary.to_string(), value_style),
        Span::styled(
            " [Enter to edit]".to_string(),
            Style::default().fg(Color::DarkGray),
        ),
    ])
}

/// Helper to render a list field
pub fn render_list_field(
    label: &str,
    items: &[String],
    field_idx: usize,
    app: &App,
) -> Vec<Line<'static>> {
    let state = app.current_view_state();
    let is_current = state.field_index == field_idx;

    let label_style = Style::default().fg(Color::Gray);
    let focus_marker = if is_current { "> " } else { "  " };

    let mut lines = vec![Line::from(vec![
        Span::styled(focus_marker.to_string(), Style::default().fg(Color::Cyan)),
        Span::styled(format!("{}: ", label), label_style),
        Span::styled(
            format!("[{} items]", items.len()),
            Style::default().fg(Color::DarkGray),
        ),
    ])];

    if is_current {
        for (i, item) in items.iter().enumerate() {
            let item_marker = if i == state.nested_index {
                "  >> "
            } else {
                "     "
            };
            lines.push(Line::from(vec![
                Span::styled(item_marker.to_string(), Style::default().fg(Color::Green)),
                Span::raw(item.clone()),
            ]));
        }
        lines.push(Line::from(Span::styled(
            "     [Enter: add, Ctrl+Del: remove]".to_string(),
            Style::default().fg(Color::DarkGray),
        )));
    }

    lines
}

/// Helper for preview key-value line
pub fn preview_line(label: &str, value: &str) -> Line<'static> {
    Line::from(vec![
        Span::styled(format!("{}: ", label), Style::default().fg(Color::Gray)),
        Span::raw(value.to_string()),
    ])
}

/// Helper for preview key-value line with colored value
pub fn preview_line_colored(label: &str, value: &str, color: Color) -> Line<'static> {
    Line::from(vec![
        Span::styled(format!("{}: ", label), Style::default().fg(Color::Gray)),
        Span::styled(value.to_string(), Style::default().fg(color)),
    ])
}
