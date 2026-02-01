use super::{preview_line, render_field_line, render_section_header};
use crate::app::App;
use loot_core::config::{AffixPoolConfig, Config};
use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
};

pub fn render_preview(config: &Config, id: &str) -> Vec<Line<'static>> {
    let Some(pool) = config.affix_pools.get(id) else {
        return vec![Line::from("Affix pool not found")];
    };

    let mut lines = vec![
        Line::from(Span::styled(
            if pool.name.is_empty() {
                pool.id.clone()
            } else {
                pool.name.clone()
            },
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        )),
        preview_line("ID", &pool.id),
    ];

    if !pool.description.is_empty() {
        lines.push(Line::from(Span::styled(
            pool.description.clone(),
            Style::default().fg(Color::DarkGray),
        )));
    }

    lines.push(Line::from(""));
    lines.push(render_section_header(&format!(
        "Affixes ({})",
        pool.affixes.len()
    )));

    for affix_id in &pool.affixes {
        // Look up the affix to get its name
        let display = config
            .affixes
            .get(affix_id)
            .map(|a| {
                let type_indicator = match a.affix_type {
                    loot_core::types::AffixType::Prefix => "[P]",
                    loot_core::types::AffixType::Suffix => "[S]",
                };
                format!("{} {} - {}", type_indicator, affix_id, a.name)
            })
            .unwrap_or_else(|| format!("{} (not found)", affix_id));

        let color = config
            .affixes
            .get(affix_id)
            .map(|a| match a.affix_type {
                loot_core::types::AffixType::Prefix => Color::Cyan,
                loot_core::types::AffixType::Suffix => Color::Green,
            })
            .unwrap_or(Color::Red);

        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled(display, Style::default().fg(color)),
        ]));
    }

    lines
}

pub fn render_edit_form(pool: &AffixPoolConfig, app: &App) -> Vec<Line<'static>> {
    let state = app.current_view_state();
    let cursor = if matches!(app.focus, crate::app::Focus::Field(_)) {
        Some(app.text_input.cursor())
    } else {
        None
    };

    // Clone values to avoid lifetime issues
    let id = pool.id.clone();
    let name = pool.name.clone();
    let description = pool.description.clone();
    let affixes = pool.affixes.clone();

    let mut lines = vec![
        render_section_header("Affix Pool"),
        Line::from(""),
        render_field_line(
            "ID",
            &id,
            0,
            app,
            if state.field_index == 0 { cursor } else { None },
        ),
        render_field_line(
            "Name",
            &name,
            1,
            app,
            if state.field_index == 1 { cursor } else { None },
        ),
        render_field_line(
            "Description",
            &description,
            2,
            app,
            if state.field_index == 2 { cursor } else { None },
        ),
        Line::from(""),
    ];

    // Affixes field
    let is_affixes_focused = state.field_index == 3;
    let focus_marker = if is_affixes_focused { "> " } else { "  " };

    lines.push(Line::from(vec![
        Span::styled(focus_marker.to_string(), Style::default().fg(Color::Cyan)),
        Span::styled("Affixes: ".to_string(), Style::default().fg(Color::Gray)),
        Span::styled(
            format!("[{} items]", affixes.len()),
            Style::default().fg(Color::DarkGray),
        ),
    ]));

    if is_affixes_focused {
        // Show input field
        let input_display = format!("{}|", app.text_input.value());
        lines.push(Line::from(vec![
            Span::styled("     Input: ", Style::default().fg(Color::Gray)),
            Span::styled(
                input_display,
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
        ]));

        // Show current affixes with selection
        if affixes.is_empty() {
            lines.push(Line::from(Span::styled(
                "     (no affixes)",
                Style::default().fg(Color::DarkGray),
            )));
        } else {
            for (i, affix_id) in affixes.iter().enumerate() {
                let marker = if i == state.nested_index {
                    "  >> "
                } else {
                    "     "
                };
                lines.push(Line::from(vec![
                    Span::styled(marker.to_string(), Style::default().fg(Color::Green)),
                    Span::raw(affix_id.clone()),
                ]));
            }
        }
        lines.push(Line::from(Span::styled(
            "     [Enter: add, x: remove, Up/Down: select]".to_string(),
            Style::default().fg(Color::DarkGray),
        )));

        // Show valid affixes from config
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "     Valid affixes (from config):",
            Style::default().fg(Color::Gray),
        )));
        let all_affixes = app.get_all_affix_ids();
        for chunk in all_affixes.chunks(4) {
            lines.push(Line::from(Span::styled(
                format!("     {}", chunk.join(", ")),
                Style::default().fg(Color::DarkGray),
            )));
        }
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "Tab: next field | Ctrl+S: save | Esc: cancel".to_string(),
        Style::default().fg(Color::DarkGray),
    )));

    lines
}
