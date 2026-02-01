use super::{
    preview_line, preview_line_colored, render_field_line, render_nested_field,
    render_section_header,
};
use crate::app::App;
use loot_core::config::{Config, CurrencyConfig};
use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
};

pub fn render_preview(config: &Config, id: &str) -> Vec<Line<'static>> {
    let Some(curr) = config.currencies.get(id) else {
        return vec![Line::from("Currency not found")];
    };

    let mut lines = vec![
        Line::from(Span::styled(
            curr.name.clone(),
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )),
        preview_line("ID", &curr.id),
    ];

    if !curr.category.is_empty() {
        lines.push(preview_line_colored(
            "Category",
            &curr.category,
            Color::Yellow,
        ));
    }

    if !curr.description.is_empty() {
        lines.push(Line::from(Span::styled(
            curr.description.clone(),
            Style::default().fg(Color::DarkGray),
        )));
    }

    lines.push(Line::from(""));

    // Requirements
    let reqs = &curr.requires;
    if !reqs.rarities.is_empty() || reqs.has_affix || reqs.has_affix_slot {
        lines.push(render_section_header("Requirements"));

        if !reqs.rarities.is_empty() {
            let rarities: Vec<String> = reqs.rarities.iter().map(|r| format!("{:?}", r)).collect();
            lines.push(preview_line("  Rarities", &rarities.join(", ")));
        }
        if reqs.has_affix {
            lines.push(preview_line_colored("  Has Affix", "true", Color::Green));
        }
        if reqs.has_affix_slot {
            lines.push(preview_line_colored(
                "  Has Affix Slot",
                "true",
                Color::Green,
            ));
        }
        lines.push(Line::from(""));
    }

    // Effects
    let effects = &curr.effects;
    lines.push(render_section_header("Effects"));

    if let Some(rarity) = effects.set_rarity {
        lines.push(preview_line_colored(
            "  Set Rarity",
            &format!("{:?}", rarity),
            Color::Yellow,
        ));
    }
    if effects.clear_affixes {
        lines.push(preview_line_colored("  Clear Affixes", "true", Color::Red));
    }
    if let Some(ref count) = effects.add_affixes {
        let range = if count.min == count.max {
            format!("{}", count.min)
        } else {
            format!("{}-{}", count.min, count.max)
        };
        lines.push(preview_line_colored("  Add Affixes", &range, Color::Green));
    }
    if !effects.add_specific_affix.is_empty() {
        let total_weight: u32 = effects.add_specific_affix.iter().map(|s| s.weight).sum();
        lines.push(Line::from(Span::styled(
            format!(
                "  Add Specific: {} affix(es)",
                effects.add_specific_affix.len()
            ),
            Style::default().fg(Color::Green),
        )));
        for specific in &effects.add_specific_affix {
            let chance = if total_weight > 0 {
                (specific.weight as f64 / total_weight as f64) * 100.0
            } else {
                0.0
            };
            let tier_str = specific
                .tier
                .map(|t| format!(" T{}", t))
                .unwrap_or_default();
            lines.push(Line::from(vec![
                Span::raw("    - "),
                Span::styled(
                    format!("{}{}", specific.id, tier_str),
                    Style::default().fg(Color::White),
                ),
                Span::styled(
                    format!(" ({:.1}%)", chance),
                    Style::default().fg(Color::Yellow),
                ),
            ]));
        }
    }
    if let Some(count) = effects.remove_affixes {
        lines.push(preview_line_colored(
            "  Remove Affixes",
            &count.to_string(),
            Color::Red,
        ));
    }
    if let Some(count) = effects.reroll_affixes {
        lines.push(preview_line_colored(
            "  Reroll Affixes",
            &count.to_string(),
            Color::Yellow,
        ));
    }
    if effects.try_unique {
        lines.push(preview_line_colored(
            "  Try Unique",
            "true",
            Color::Rgb(175, 95, 0),
        ));
    }
    if !effects.affix_pools.is_empty() {
        lines.push(preview_line(
            "  Affix Pools",
            &effects.affix_pools.join(", "),
        ));
    }

    lines
}

pub fn render_edit_form(curr: &CurrencyConfig, app: &App) -> Vec<Line<'static>> {
    let state = app.current_view_state();
    let cursor = if matches!(app.focus, crate::app::Focus::Field(_)) {
        Some(app.text_input.cursor())
    } else {
        None
    };

    // Clone values we need
    let id = curr.id.clone();
    let name = curr.name.clone();
    let description = curr.description.clone();
    let category = curr.category.clone();

    // Requirements summary
    let reqs = &curr.requires;
    let reqs_summary = {
        let mut parts = Vec::new();
        if !reqs.rarities.is_empty() {
            parts.push(format!("{} rarities", reqs.rarities.len()));
        }
        if reqs.has_affix {
            parts.push("has_affix".to_string());
        }
        if reqs.has_affix_slot {
            parts.push("has_slot".to_string());
        }
        if parts.is_empty() {
            "None".to_string()
        } else {
            parts.join(", ")
        }
    };

    // Effects summary
    let effects = &curr.effects;
    let effects_summary = {
        let mut parts = Vec::new();
        if effects.set_rarity.is_some() {
            parts.push("set_rarity".to_string());
        }
        if effects.clear_affixes {
            parts.push("clear".to_string());
        }
        if effects.add_affixes.is_some() {
            parts.push("add".to_string());
        }
        if !effects.add_specific_affix.is_empty() {
            parts.push("specific".to_string());
        }
        if effects.remove_affixes.is_some() {
            parts.push("remove".to_string());
        }
        if effects.try_unique {
            parts.push("unique".to_string());
        }
        if parts.is_empty() {
            "None".to_string()
        } else {
            parts.join(", ")
        }
    };

    let mut lines = vec![
        render_section_header("Currency"),
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
        render_field_line(
            "Category",
            &category,
            3,
            app,
            if state.field_index == 3 { cursor } else { None },
        ),
        Line::from(""),
    ];

    // Requirements - nested
    lines.push(render_nested_field("Requirements", &reqs_summary, 4, app));

    // Show requirements details when in nested mode
    if state.field_index == 4 && state.nested_depth > 0 {
        let reqs = &curr.requires;
        lines.push(Line::from(""));

        // Special handling for rarities list (nested_index == 0 and nested_depth >= 2)
        if state.nested_index == 0 && state.nested_depth >= 2 {
            // Rarities list editing mode
            let input_display = format!("{}|", app.text_input.value());
            lines.push(Line::from(vec![
                Span::styled("     Add: ", Style::default().fg(Color::Gray)),
                Span::styled(
                    input_display,
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ),
            ]));

            // Show existing rarities as a selectable list
            if reqs.rarities.is_empty() {
                lines.push(Line::from(Span::styled(
                    "     (no rarities - press Enter to add)",
                    Style::default().fg(Color::DarkGray),
                )));
            } else {
                for (i, rarity) in reqs.rarities.iter().enumerate() {
                    let is_selected = i == app.nested_sub_field_index;
                    let marker = if is_selected { "     >> " } else { "        " };
                    let style = if is_selected {
                        Style::default().fg(Color::Cyan)
                    } else {
                        Style::default().fg(Color::White)
                    };
                    lines.push(Line::from(vec![
                        Span::styled(marker.to_string(), Style::default().fg(Color::Green)),
                        Span::styled(format!("{:?}", rarity), style),
                    ]));
                }
            }

            lines.push(Line::from(Span::styled(
                "     [Enter: add, x: remove, Up/Down: select, Esc: back]",
                Style::default().fg(Color::DarkGray),
            )));
            lines.push(Line::from(Span::styled(
                "     Valid: Normal, Magic, Rare, Unique",
                Style::default().fg(Color::DarkGray),
            )));

            // Also show the other requirement fields (non-editable in this mode)
            lines.push(Line::from(""));
            lines.push(Line::from(vec![
                Span::styled("     ", Style::default()),
                Span::styled("Has Affix: ", Style::default().fg(Color::Yellow)),
                Span::styled(
                    reqs.has_affix.to_string(),
                    Style::default().fg(Color::DarkGray),
                ),
            ]));
            lines.push(Line::from(vec![
                Span::styled("     ", Style::default()),
                Span::styled("Has Affix Slot: ", Style::default().fg(Color::Yellow)),
                Span::styled(
                    reqs.has_affix_slot.to_string(),
                    Style::default().fg(Color::DarkGray),
                ),
            ]));
        } else {
            // Normal requirements field selection mode
            if state.nested_depth >= 2 && state.nested_index != 0 {
                let input_display = format!("{}|", app.text_input.value());
                lines.push(Line::from(vec![
                    Span::styled("     Edit: ", Style::default().fg(Color::Gray)),
                    Span::styled(
                        input_display,
                        Style::default()
                            .fg(Color::Cyan)
                            .add_modifier(Modifier::BOLD),
                    ),
                ]));
            }

            // Requirements sub-fields
            let req_items = [
                ("Rarities", {
                    if reqs.rarities.is_empty() {
                        "none".to_string()
                    } else {
                        format!("{} item(s)", reqs.rarities.len())
                    }
                }),
                ("Has Affix", reqs.has_affix.to_string()),
                ("Has Affix Slot", reqs.has_affix_slot.to_string()),
            ];

            for (i, (name, value)) in req_items.iter().enumerate() {
                let is_selected = i == state.nested_index;
                let marker = if is_selected { "  >> " } else { "     " };
                let style = if is_selected && state.nested_depth < 2 {
                    Style::default().fg(Color::Cyan)
                } else {
                    Style::default()
                };

                lines.push(Line::from(vec![
                    Span::styled(marker.to_string(), Style::default().fg(Color::Green)),
                    Span::styled(format!("{}: ", name), Style::default().fg(Color::Yellow)),
                    Span::styled(value.clone(), style),
                ]));
            }

            if state.nested_depth >= 2 {
                // Show format hints based on selected field
                let hint = match state.nested_index {
                    1 | 2 => "Format: true or false",
                    _ => "",
                };
                if !hint.is_empty() {
                    lines.push(Line::from(Span::styled(
                        format!("     {}", hint),
                        Style::default().fg(Color::DarkGray),
                    )));
                }
                lines.push(Line::from(Span::styled(
                    "     [Enter: save, Esc: cancel]",
                    Style::default().fg(Color::DarkGray),
                )));
            } else {
                lines.push(Line::from(Span::styled(
                    "     [Enter: edit, Up/Down: select, Esc: back]",
                    Style::default().fg(Color::DarkGray),
                )));
            }
        }
    }

    // Effects - nested
    lines.push(render_nested_field("Effects", &effects_summary, 5, app));

    // Show effects details when in nested mode
    if state.field_index == 5 && state.nested_depth > 0 {
        let effects = &curr.effects;
        lines.push(Line::from(""));

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
            ]));
        }

        // Effects sub-fields
        let effect_items: Vec<(&str, String)> = vec![
            (
                "Set Rarity",
                effects
                    .set_rarity
                    .map(|r| format!("{:?}", r))
                    .unwrap_or_else(|| "none".to_string()),
            ),
            ("Clear Affixes", effects.clear_affixes.to_string()),
            (
                "Add Affixes",
                effects
                    .add_affixes
                    .as_ref()
                    .map(|c| format!("{}-{}", c.min, c.max))
                    .unwrap_or_else(|| "none".to_string()),
            ),
            (
                "Add Specific",
                format!("{} affix(es)", effects.add_specific_affix.len()),
            ),
            (
                "Remove Affixes",
                effects
                    .remove_affixes
                    .map(|n| n.to_string())
                    .unwrap_or_else(|| "none".to_string()),
            ),
            (
                "Reroll Affixes",
                effects
                    .reroll_affixes
                    .map(|n| n.to_string())
                    .unwrap_or_else(|| "none".to_string()),
            ),
            ("Try Unique", effects.try_unique.to_string()),
            (
                "Affix Pools",
                if effects.affix_pools.is_empty() {
                    "none".to_string()
                } else {
                    effects.affix_pools.join(", ")
                },
            ),
        ];

        for (i, (name, value)) in effect_items.iter().enumerate() {
            let is_selected = i == state.nested_index;
            let marker = if is_selected { "  >> " } else { "     " };
            let style = if is_selected && state.nested_depth < 2 {
                Style::default().fg(Color::Cyan)
            } else {
                Style::default()
            };

            lines.push(Line::from(vec![
                Span::styled(marker.to_string(), Style::default().fg(Color::Green)),
                Span::styled(format!("{}: ", name), Style::default().fg(Color::Yellow)),
                Span::styled(value.clone(), style),
            ]));
        }

        // Special handling for Add Specific field (nested_index == 3)
        if state.nested_index == 3 && state.nested_depth >= 2 {
            // In list editing mode for specific affixes
            let total_weight: u32 = effects.add_specific_affix.iter().map(|s| s.weight).sum();
            lines.push(Line::from(""));

            // Show input field when editing (depth 3)
            if state.nested_depth >= 3 {
                let is_adding_new = app.nested_sub_field_index == usize::MAX;
                let label = if is_adding_new {
                    "     Add: "
                } else {
                    "     Edit: "
                };
                let input_display = format!("{}|", app.text_input.value());
                lines.push(Line::from(vec![
                    Span::styled(label, Style::default().fg(Color::Gray)),
                    Span::styled(
                        input_display,
                        Style::default()
                            .fg(Color::Cyan)
                            .add_modifier(Modifier::BOLD),
                    ),
                ]));
                lines.push(Line::from(Span::styled(
                    "     Format: affix_id [tier] weight",
                    Style::default().fg(Color::DarkGray),
                )));
            }

            // Show the list with selection marker
            if effects.add_specific_affix.is_empty() {
                lines.push(Line::from(Span::styled(
                    "     (no specific affixes - press + to add)",
                    Style::default().fg(Color::DarkGray),
                )));
            } else {
                for (i, specific) in effects.add_specific_affix.iter().enumerate() {
                    let is_selected = i == app.nested_sub_field_index;
                    let marker = if is_selected { "     >> " } else { "        " };
                    let style = if is_selected && state.nested_depth < 3 {
                        Style::default().fg(Color::Cyan)
                    } else {
                        Style::default().fg(Color::White)
                    };

                    let chance = if total_weight > 0 {
                        (specific.weight as f64 / total_weight as f64) * 100.0
                    } else {
                        0.0
                    };
                    let tier_str = specific
                        .tier
                        .map(|t| format!(" T{}", t))
                        .unwrap_or_default();

                    lines.push(Line::from(vec![
                        Span::styled(marker.to_string(), Style::default().fg(Color::Green)),
                        Span::styled(format!("{}{}", specific.id, tier_str), style),
                        Span::styled(
                            format!(" - {:.1}% (w:{})", chance, specific.weight),
                            Style::default().fg(Color::DarkGray),
                        ),
                    ]));
                }
            }

            if state.nested_depth >= 3 {
                lines.push(Line::from(Span::styled(
                    "     [Enter: save, Esc: cancel]",
                    Style::default().fg(Color::DarkGray),
                )));
            } else {
                lines.push(Line::from(Span::styled(
                    "     [Enter: edit, +: add, x: remove, Up/Down: select, Esc: back]",
                    Style::default().fg(Color::DarkGray),
                )));
            }

            // Show valid affixes
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                "     Valid affixes:",
                Style::default().fg(Color::Gray),
            )));
            let all_affixes = app.get_all_affix_ids();
            for chunk in all_affixes.chunks(4) {
                lines.push(Line::from(Span::styled(
                    format!("     {}", chunk.join(", ")),
                    Style::default().fg(Color::DarkGray),
                )));
            }
        } else if state.nested_depth >= 2 {
            // Other effect fields - show format hints
            let hint = match state.nested_index {
                0 => "Format: Normal, Magic, Rare, or Unique (or 'none')",
                1 | 6 => "Format: true or false",
                2 => "Format: min max (or single number, or 'none')",
                4 | 5 => "Format: number (or 'none')",
                7 => "Format: pool1, pool2 (comma-separated, or 'none')",
                _ => "",
            };
            lines.push(Line::from(Span::styled(
                format!("     {}", hint),
                Style::default().fg(Color::DarkGray),
            )));

            // Show valid options for certain fields
            if state.nested_index == 0 {
                lines.push(Line::from(Span::styled(
                    "     Valid: Normal, Magic, Rare, Unique",
                    Style::default().fg(Color::DarkGray),
                )));
            } else if state.nested_index == 7 {
                lines.push(Line::from(Span::styled(
                    "     Valid pools:",
                    Style::default().fg(Color::Gray),
                )));
                let all_pools = app.get_all_affix_pool_ids();
                for chunk in all_pools.chunks(4) {
                    lines.push(Line::from(Span::styled(
                        format!("     {}", chunk.join(", ")),
                        Style::default().fg(Color::DarkGray),
                    )));
                }
            }

            lines.push(Line::from(Span::styled(
                "     [Enter: save, Esc: cancel]",
                Style::default().fg(Color::DarkGray),
            )));
        } else {
            lines.push(Line::from(Span::styled(
                "     [Enter: edit, Up/Down: select, Esc: back]",
                Style::default().fg(Color::DarkGray),
            )));
        }
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "Tab/Shift+Tab: navigate fields | Ctrl+S: save | Esc: cancel".to_string(),
        Style::default().fg(Color::DarkGray),
    )));

    // Field explanations footnote
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "─── Field Reference ───",
        Style::default().fg(Color::DarkGray),
    )));
    lines.push(Line::from(Span::styled(
        "Requirements (item must match to use currency):",
        Style::default().fg(Color::Gray),
    )));
    lines.push(Line::from(Span::styled(
        "  Rarities: Item must be one of these rarities",
        Style::default().fg(Color::DarkGray),
    )));
    lines.push(Line::from(Span::styled(
        "  Has Affix: Item must have at least one affix",
        Style::default().fg(Color::DarkGray),
    )));
    lines.push(Line::from(Span::styled(
        "  Has Affix Slot: Item must have room for more affixes",
        Style::default().fg(Color::DarkGray),
    )));
    lines.push(Line::from(Span::styled(
        "Effects (what the currency does):",
        Style::default().fg(Color::Gray),
    )));
    lines.push(Line::from(Span::styled(
        "  Set Rarity: Change item to this rarity",
        Style::default().fg(Color::DarkGray),
    )));
    lines.push(Line::from(Span::styled(
        "  Clear Affixes: Remove all existing affixes",
        Style::default().fg(Color::DarkGray),
    )));
    lines.push(Line::from(Span::styled(
        "  Add Affixes: Add random affixes (min-max range)",
        Style::default().fg(Color::DarkGray),
    )));
    lines.push(Line::from(Span::styled(
        "  Add Specific: Add weighted specific affixes (w/ tier)",
        Style::default().fg(Color::DarkGray),
    )));
    lines.push(Line::from(Span::styled(
        "  Remove Affixes: Remove N random affixes",
        Style::default().fg(Color::DarkGray),
    )));
    lines.push(Line::from(Span::styled(
        "  Reroll Affixes: Reroll values of N random affixes",
        Style::default().fg(Color::DarkGray),
    )));
    lines.push(Line::from(Span::styled(
        "  Try Unique: Chance to upgrade to unique version",
        Style::default().fg(Color::DarkGray),
    )));
    lines.push(Line::from(Span::styled(
        "  Affix Pools: Limit affixes to these pools only",
        Style::default().fg(Color::DarkGray),
    )));

    lines
}
