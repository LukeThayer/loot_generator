use super::{
    preview_line, preview_line_colored, render_field_line, render_nested_field,
    render_section_header,
};
use crate::app::App;
use loot_core::config::{AffixConfig, Config};
use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
};

pub fn render_preview(config: &Config, id: &str) -> Vec<Line<'static>> {
    let Some(affix) = config.affixes.get(id) else {
        return vec![Line::from("Affix not found")];
    };

    let type_color = match affix.affix_type {
        loot_core::types::AffixType::Prefix => Color::Cyan,
        loot_core::types::AffixType::Suffix => Color::Green,
    };

    let mut lines = vec![
        Line::from(Span::styled(
            affix.name.clone(),
            Style::default().fg(type_color).add_modifier(Modifier::BOLD),
        )),
        preview_line("ID", &affix.id),
        preview_line_colored("Type", &format!("{:?}", affix.affix_type), type_color),
        preview_line_colored("Stat", &format!("{:?}", affix.stat), Color::Yellow),
        Line::from(""),
    ];

    // Tags
    if !affix.tags.is_empty() {
        lines.push(render_section_header("Tags"));
        lines.push(Line::from(Span::styled(
            affix.tags.join(", "),
            Style::default().fg(Color::Yellow),
        )));
        lines.push(Line::from(""));
    }

    // Allowed classes
    if !affix.allowed_classes.is_empty() {
        lines.push(render_section_header("Allowed Classes"));
        let classes: Vec<String> = affix
            .allowed_classes
            .iter()
            .map(|c| format!("{:?}", c))
            .collect();
        lines.push(Line::from(classes.join(", ")));
        lines.push(Line::from(""));
    }

    // Tiers
    lines.push(render_section_header("Tiers"));
    for tier in &affix.tiers {
        lines.push(Line::from(vec![
            Span::styled(
                format!("  T{} ", tier.tier),
                Style::default().fg(Color::Yellow),
            ),
            Span::styled(
                format!("({}-{}) ", tier.min, tier.max),
                Style::default().fg(Color::White),
            ),
            Span::styled(
                format!("ilvl:{} ", tier.min_ilvl),
                Style::default().fg(Color::Magenta),
            ),
            Span::styled(
                format!("w:{}", tier.weight),
                Style::default().fg(Color::DarkGray),
            ),
        ]));
    }

    lines
}

pub fn render_edit_form(affix: &AffixConfig, app: &App) -> Vec<Line<'static>> {
    let state = app.current_view_state();
    let cursor = if matches!(app.focus, crate::app::Focus::Field(_)) {
        Some(app.text_input.cursor())
    } else {
        None
    };

    // Clone values we need
    let id = affix.id.clone();
    let name = affix.name.clone();
    let type_str = format!("{:?}", affix.affix_type);
    let stat_str = format!("{:?}", affix.stat);
    let tags_str = affix.tags.join(", ");
    let classes_str = affix
        .allowed_classes
        .iter()
        .map(|c| format!("{:?}", c))
        .collect::<Vec<_>>()
        .join(", ");
    let tiers_summary = format!("{} tiers", affix.tiers.len());

    let mut lines = vec![
        render_section_header("Affix"),
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
    ];

    // Type field (enum)
    let is_type_focused = state.field_index == 2;
    let type_marker = if is_type_focused { "> " } else { "  " };
    let type_style = if is_type_focused {
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default()
    };

    if is_type_focused {
        let input_display = format!("{}|", app.text_input.value());
        lines.push(Line::from(vec![
            Span::styled(type_marker.to_string(), Style::default().fg(Color::Cyan)),
            Span::styled("Type: ", Style::default().fg(Color::Gray)),
            Span::styled(input_display, type_style),
        ]));
        lines.push(Line::from(Span::styled(
            format!("     Current: {}", type_str),
            Style::default().fg(Color::DarkGray),
        )));
        lines.push(Line::from(Span::styled(
            "     [Enter: set] Valid: Prefix, Suffix",
            Style::default().fg(Color::DarkGray),
        )));
    } else {
        lines.push(Line::from(vec![
            Span::styled(type_marker.to_string(), Style::default().fg(Color::Cyan)),
            Span::styled("Type: ", Style::default().fg(Color::Gray)),
            Span::styled(type_str, type_style),
        ]));
    }

    // Stat field (enum)
    let is_stat_focused = state.field_index == 3;
    let stat_marker = if is_stat_focused { "> " } else { "  " };
    let stat_style = if is_stat_focused {
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default()
    };

    if is_stat_focused {
        let input_display = format!("{}|", app.text_input.value());
        lines.push(Line::from(vec![
            Span::styled(stat_marker.to_string(), Style::default().fg(Color::Cyan)),
            Span::styled("Stat: ", Style::default().fg(Color::Gray)),
            Span::styled(input_display, stat_style),
        ]));
        lines.push(Line::from(Span::styled(
            format!("     Current: {}", stat_str),
            Style::default().fg(Color::DarkGray),
        )));
        lines.push(Line::from(Span::styled(
            "     [Enter: set]",
            Style::default().fg(Color::DarkGray),
        )));
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "     Valid StatTypes:",
            Style::default().fg(Color::Gray),
        )));
        let all_stats = crate::app::App::get_all_stat_types();
        for chunk in all_stats.chunks(4) {
            lines.push(Line::from(Span::styled(
                format!("     {}", chunk.join(", ")),
                Style::default().fg(Color::DarkGray),
            )));
        }
    } else {
        lines.push(Line::from(vec![
            Span::styled(stat_marker.to_string(), Style::default().fg(Color::Cyan)),
            Span::styled("Stat: ", Style::default().fg(Color::Gray)),
            Span::styled(stat_str, stat_style),
        ]));
    }

    // Tags as list field
    let is_tags_focused = state.field_index == 4;
    let tags_marker = if is_tags_focused { "> " } else { "  " };
    let tags_style = if is_tags_focused {
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default()
    };

    if is_tags_focused {
        // Show input field when focused
        let input_display = format!("{}|", app.text_input.value());
        lines.push(Line::from(vec![
            Span::styled(tags_marker.to_string(), Style::default().fg(Color::Cyan)),
            Span::styled("Tags: ", Style::default().fg(Color::Gray)),
            Span::styled(input_display, tags_style),
        ]));
        // Show existing tags as a list
        for (i, tag) in affix.tags.iter().enumerate() {
            let item_marker = if i == state.nested_index {
                "  >> "
            } else {
                "     "
            };
            lines.push(Line::from(vec![
                Span::styled(item_marker.to_string(), Style::default().fg(Color::Green)),
                Span::raw(tag.clone()),
            ]));
        }
        lines.push(Line::from(Span::styled(
            "     [Enter: add, x: remove, Up/Down: select]".to_string(),
            Style::default().fg(Color::DarkGray),
        )));
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "     Valid tags (from config):",
            Style::default().fg(Color::Gray),
        )));
        // Get tags from config and display in rows
        let all_tags = app.get_all_tags();
        for chunk in all_tags.chunks(6) {
            lines.push(Line::from(Span::styled(
                format!("     {}", chunk.join(", ")),
                Style::default().fg(Color::DarkGray),
            )));
        }
    } else {
        lines.push(Line::from(vec![
            Span::styled(tags_marker.to_string(), Style::default().fg(Color::Cyan)),
            Span::styled("Tags: ", Style::default().fg(Color::Gray)),
            Span::styled(
                if tags_str.is_empty() {
                    "(none)".to_string()
                } else {
                    tags_str
                },
                Style::default().fg(Color::DarkGray),
            ),
        ]));
    }

    lines.push(Line::from(""));

    // Allowed classes as list field
    let is_classes_focused = state.field_index == 5;
    let classes_marker = if is_classes_focused { "> " } else { "  " };
    let classes_style = if is_classes_focused {
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default()
    };

    if is_classes_focused {
        // Show input field when focused
        let input_display = format!("{}|", app.text_input.value());
        lines.push(Line::from(vec![
            Span::styled(classes_marker.to_string(), Style::default().fg(Color::Cyan)),
            Span::styled("Allowed Classes: ", Style::default().fg(Color::Gray)),
            Span::styled(input_display, classes_style),
        ]));
        // Show existing classes as a list
        for (i, class) in affix.allowed_classes.iter().enumerate() {
            let item_marker = if i == state.nested_index {
                "  >> "
            } else {
                "     "
            };
            lines.push(Line::from(vec![
                Span::styled(item_marker.to_string(), Style::default().fg(Color::Green)),
                Span::raw(format!("{:?}", class)),
            ]));
        }
        lines.push(Line::from(Span::styled(
            "     [Enter: add, x: remove, Up/Down: select]".to_string(),
            Style::default().fg(Color::DarkGray),
        )));
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "     Valid classes (from types.rs):",
            Style::default().fg(Color::Gray),
        )));
        let all_classes = crate::app::App::get_all_classes();
        for chunk in all_classes.chunks(6) {
            lines.push(Line::from(Span::styled(
                format!("     {}", chunk.join(", ")),
                Style::default().fg(Color::DarkGray),
            )));
        }
    } else {
        lines.push(Line::from(vec![
            Span::styled(classes_marker.to_string(), Style::default().fg(Color::Cyan)),
            Span::styled("Allowed Classes: ", Style::default().fg(Color::Gray)),
            Span::styled(
                if classes_str.is_empty() {
                    "(none)".to_string()
                } else {
                    classes_str
                },
                Style::default().fg(Color::DarkGray),
            ),
        ]));
    }

    lines.push(Line::from(""));

    // Tiers as nested
    lines.push(render_nested_field("Tiers", &tiers_summary, 6, app));

    // If in tiers, show them
    if state.field_index == 6 && state.nested_depth > 0 {
        lines.push(Line::from(""));

        // Show input for new/edit tier
        if state.nested_depth >= 2 {
            let input_display = format!("{}|", app.text_input.value());
            lines.push(Line::from(vec![
                Span::styled("     Edit: ", Style::default().fg(Color::Gray)),
                Span::styled(
                    input_display,
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    " (format: tier weight min max min_ilvl)",
                    Style::default().fg(Color::DarkGray),
                ),
            ]));
        }

        for (i, tier) in affix.tiers.iter().enumerate() {
            let is_selected = i == state.nested_index;
            let marker = if is_selected { "  >> " } else { "     " };
            let style = if is_selected && state.nested_depth < 2 {
                Style::default().fg(Color::Cyan)
            } else {
                Style::default()
            };

            lines.push(Line::from(vec![
                Span::styled(marker.to_string(), Style::default().fg(Color::Green)),
                Span::styled(
                    format!("T{}: ", tier.tier),
                    Style::default().fg(Color::Yellow),
                ),
                Span::styled(
                    format!("{}-{} ", tier.min, tier.max),
                    style,
                ),
                Span::styled(
                    format!("ilvl:{} ", tier.min_ilvl),
                    Style::default().fg(Color::Magenta),
                ),
                Span::styled(
                    format!("w:{}", tier.weight),
                    Style::default().fg(Color::DarkGray),
                ),
            ]));
        }

        if state.nested_depth >= 2 {
            lines.push(Line::from(Span::styled(
                "     [Enter: save, Esc: cancel]".to_string(),
                Style::default().fg(Color::DarkGray),
            )));
        } else {
            lines.push(Line::from(Span::styled(
                "     [Enter: edit, Ctrl+Del: remove, +: add, Esc: back]".to_string(),
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
