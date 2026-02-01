use super::{
    preview_line, preview_line_colored, render_field_line, render_nested_field,
    render_section_header,
};
use crate::app::App;
use loot_core::config::{BaseTypeConfig, Config};
use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
};

pub fn render_preview(config: &Config, id: &str) -> Vec<Line<'static>> {
    let Some(bt) = config.base_types.get(id) else {
        return vec![Line::from("Base type not found")];
    };

    let mut lines = vec![
        Line::from(Span::styled(
            bt.name.clone(),
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        )),
        preview_line("ID", &bt.id),
        preview_line_colored("Class", &format!("{:?}", bt.class), Color::Cyan),
        Line::from(""),
    ];

    // Tags
    if !bt.tags.is_empty() {
        lines.push(render_section_header("Tags"));
        lines.push(Line::from(Span::styled(
            bt.tags.join(", "),
            Style::default().fg(Color::Yellow),
        )));
        lines.push(Line::from(""));
    }

    // Requirements
    let req = &bt.requirements;
    if req.level > 0 || req.strength > 0 || req.dexterity > 0 || req.intelligence > 0 {
        lines.push(render_section_header("Requirements"));
        if req.level > 0 {
            lines.push(preview_line("  Level", &req.level.to_string()));
        }
        if req.strength > 0 {
            lines.push(preview_line("  Strength", &req.strength.to_string()));
        }
        if req.dexterity > 0 {
            lines.push(preview_line("  Dexterity", &req.dexterity.to_string()));
        }
        if req.intelligence > 0 {
            lines.push(preview_line(
                "  Intelligence",
                &req.intelligence.to_string(),
            ));
        }
        lines.push(Line::from(""));
    }

    // Damage
    if let Some(ref dmg) = bt.damage {
        lines.push(render_section_header("Damage"));
        // Show each damage type with its range
        for entry in &dmg.damages {
            let color = match entry.damage_type {
                loot_core::types::DamageType::Physical => Color::White,
                loot_core::types::DamageType::Fire => Color::Red,
                loot_core::types::DamageType::Cold => Color::Cyan,
                loot_core::types::DamageType::Lightning => Color::Yellow,
                loot_core::types::DamageType::Chaos => Color::Magenta,
            };
            lines.push(Line::from(vec![
                Span::styled("  ", Style::default()),
                Span::styled(
                    format!("{:?}: ", entry.damage_type),
                    Style::default().fg(color),
                ),
                Span::styled(
                    format!("{}-{}", entry.min, entry.max),
                    Style::default().fg(Color::White),
                ),
            ]));
        }
        if dmg.attack_speed > 0.0 {
            lines.push(preview_line(
                "  Attack Speed",
                &format!("{:.2}", dmg.attack_speed),
            ));
        }
        if dmg.critical_chance > 0.0 {
            lines.push(preview_line(
                "  Crit Chance",
                &format!("{:.1}%", dmg.critical_chance),
            ));
        }
        if dmg.spell_efficiency > 0.0 {
            lines.push(preview_line(
                "  Spell Efficiency",
                &format!("{:.0}%", dmg.spell_efficiency),
            ));
        }
        lines.push(Line::from(""));
    }

    // Defenses
    if let Some(ref def) = bt.defenses {
        lines.push(render_section_header("Defenses"));
        if let Some(armour) = &def.armour {
            lines.push(preview_line(
                "  Armour",
                &format!("{}-{}", armour.min, armour.max),
            ));
        }
        if let Some(evasion) = &def.evasion {
            lines.push(preview_line(
                "  Evasion",
                &format!("{}-{}", evasion.min, evasion.max),
            ));
        }
        if let Some(es) = &def.energy_shield {
            lines.push(preview_line(
                "  Energy Shield",
                &format!("{}-{}", es.min, es.max),
            ));
        }
        lines.push(Line::from(""));
    }

    // Implicit
    if let Some(ref imp) = bt.implicit {
        lines.push(render_section_header("Implicit"));
        lines.push(preview_line_colored(
            &format!("  {:?}", imp.stat),
            &format!("{}-{}", imp.min, imp.max),
            Color::Magenta,
        ));
    }

    // Quick reference
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "─── Quick Reference ───",
        Style::default().fg(Color::DarkGray),
    )));
    lines.push(Line::from(Span::styled(
        "Class: Equipment slot & valid affixes",
        Style::default().fg(Color::DarkGray),
    )));
    lines.push(Line::from(Span::styled(
        "Tags: Boost matching affix weights",
        Style::default().fg(Color::DarkGray),
    )));
    lines.push(Line::from(Span::styled(
        "Implicit: Always-present bonus",
        Style::default().fg(Color::DarkGray),
    )));
    lines.push(Line::from(Span::styled(
        "Spell Eff: Casting power for wands/staves",
        Style::default().fg(Color::DarkGray),
    )));

    lines
}

pub fn render_edit_form(bt: &BaseTypeConfig, app: &App) -> Vec<Line<'static>> {
    let state = app.current_view_state();
    let cursor = if matches!(app.focus, crate::app::Focus::Field(_)) {
        Some(app.text_input.cursor())
    } else {
        None
    };

    // Clone values we need to own
    let id = bt.id.clone();
    let name = bt.name.clone();
    let class_str = format!("{:?}", bt.class);
    let tags_str = bt.tags.join(", ");

    let implicit_summary = bt
        .implicit
        .as_ref()
        .map(|i| format!("{:?}: {}-{}", i.stat, i.min, i.max))
        .unwrap_or_else(|| "None".to_string());

    let defenses_summary = bt
        .defenses
        .as_ref()
        .map(|d| {
            let mut parts = Vec::new();
            if d.armour.is_some() {
                parts.push("Armour");
            }
            if d.evasion.is_some() {
                parts.push("Evasion");
            }
            if d.energy_shield.is_some() {
                parts.push("ES");
            }
            if parts.is_empty() {
                "None".to_string()
            } else {
                parts.join(", ")
            }
        })
        .unwrap_or_else(|| "None".to_string());

    let damage_summary = bt
        .damage
        .as_ref()
        .map(|d| {
            if d.damages.is_empty() {
                "No damage".to_string()
            } else {
                let parts: Vec<String> = d
                    .damages
                    .iter()
                    .map(|e| format!("{:?}:{}-{}", e.damage_type, e.min, e.max))
                    .collect();
                parts.join(", ")
            }
        })
        .unwrap_or_else(|| "None".to_string());

    let req_summary = {
        let req = &bt.requirements;
        let mut parts = Vec::new();
        if req.level > 0 {
            parts.push(format!("Lv{}", req.level));
        }
        if req.strength > 0 {
            parts.push(format!("{}Str", req.strength));
        }
        if req.dexterity > 0 {
            parts.push(format!("{}Dex", req.dexterity));
        }
        if req.intelligence > 0 {
            parts.push(format!("{}Int", req.intelligence));
        }
        if parts.is_empty() {
            "None".to_string()
        } else {
            parts.join(", ")
        }
    };

    let mut lines = vec![
        render_section_header("Base Type"),
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

    // Class as enum field
    let is_class_focused = state.field_index == 2;
    let class_marker = if is_class_focused { "> " } else { "  " };
    let class_style = if is_class_focused {
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default()
    };

    if is_class_focused {
        let input_display = format!("{}|", app.text_input.value());
        lines.push(Line::from(vec![
            Span::styled(class_marker.to_string(), Style::default().fg(Color::Cyan)),
            Span::styled("Class: ", Style::default().fg(Color::Gray)),
            Span::styled(input_display, class_style),
        ]));
        lines.push(Line::from(Span::styled(
            format!("     Current: {}", class_str),
            Style::default().fg(Color::DarkGray),
        )));
        lines.push(Line::from(Span::styled(
            "     [Enter: set class]",
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
            Span::styled(class_marker.to_string(), Style::default().fg(Color::Cyan)),
            Span::styled("Class: ", Style::default().fg(Color::Gray)),
            Span::styled(class_str, class_style),
        ]));
    }

    // Tags as list field
    let is_tags_focused = state.field_index == 3;
    let tags_marker = if is_tags_focused { "> " } else { "  " };
    let tags_style = if is_tags_focused {
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default()
    };

    if is_tags_focused {
        let input_display = format!("{}|", app.text_input.value());
        lines.push(Line::from(vec![
            Span::styled(tags_marker.to_string(), Style::default().fg(Color::Cyan)),
            Span::styled("Tags: ", Style::default().fg(Color::Gray)),
            Span::styled(input_display, tags_style),
        ]));
        for (i, tag) in bt.tags.iter().enumerate() {
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

    // Nested fields

    // Implicit field (field 4)
    lines.push(render_nested_field("Implicit", &implicit_summary, 4, app));
    if state.field_index == 4 && state.nested_depth > 0 {
        lines.push(Line::from(""));

        if state.nested_depth >= 2 {
            // Editing mode - show input
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
            lines.push(Line::from(Span::styled(
                "     Format: StatType min max (or 'none' to clear)",
                Style::default().fg(Color::DarkGray),
            )));
            lines.push(Line::from(Span::styled(
                "     [Enter: save, Esc: cancel]",
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
            // Selection mode - show current value
            let current = bt
                .implicit
                .as_ref()
                .map(|i| format!("{:?}: {}-{}", i.stat, i.min, i.max))
                .unwrap_or_else(|| "none".to_string());
            lines.push(Line::from(vec![
                Span::styled("  >> ", Style::default().fg(Color::Green)),
                Span::styled(current, Style::default().fg(Color::Cyan)),
            ]));
            lines.push(Line::from(Span::styled(
                "     [Enter: edit, Esc: back]",
                Style::default().fg(Color::DarkGray),
            )));
        }
    }

    // Defenses field (field 5) - has sub-items
    lines.push(render_nested_field("Defenses", &defenses_summary, 5, app));
    if state.field_index == 5 && state.nested_depth > 0 {
        lines.push(Line::from(""));

        if state.nested_depth >= 2 {
            // Editing mode - show input
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
            lines.push(Line::from(Span::styled(
                "     Format: min max (or 'none' to clear)",
                Style::default().fg(Color::DarkGray),
            )));
        }

        // Show defense sub-items with selection marker
        let defense_items = [
            (
                "Armour",
                bt.defenses.as_ref().and_then(|d| d.armour.as_ref()),
            ),
            (
                "Evasion",
                bt.defenses.as_ref().and_then(|d| d.evasion.as_ref()),
            ),
            (
                "Energy Shield",
                bt.defenses.as_ref().and_then(|d| d.energy_shield.as_ref()),
            ),
        ];

        for (i, (name, value)) in defense_items.iter().enumerate() {
            let is_selected = i == state.nested_index;
            let marker = if is_selected { "  >> " } else { "     " };
            let style = if is_selected && state.nested_depth < 2 {
                Style::default().fg(Color::Cyan)
            } else {
                Style::default()
            };

            let value_str = value
                .map(|r| format!("{}-{}", r.min, r.max))
                .unwrap_or_else(|| "none".to_string());

            lines.push(Line::from(vec![
                Span::styled(marker.to_string(), Style::default().fg(Color::Green)),
                Span::styled(format!("{}: ", name), Style::default().fg(Color::Yellow)),
                Span::styled(value_str, style),
            ]));
        }

        if state.nested_depth >= 2 {
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

    // Damage field (field 6) - now has sub-items
    lines.push(render_nested_field("Damage", &damage_summary, 6, app));
    if state.field_index == 6 && state.nested_depth > 0 {
        lines.push(Line::from(""));

        // Show input field when in edit mode
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
            // Show format hint based on what's being edited
            if state.nested_index < 3 {
                lines.push(Line::from(Span::styled(
                    "     Enter a number (or 'none' to clear damage entirely)",
                    Style::default().fg(Color::DarkGray),
                )));
            } else {
                lines.push(Line::from(Span::styled(
                    "     Format: Type min max (e.g., Physical 10 20)",
                    Style::default().fg(Color::DarkGray),
                )));
                lines.push(Line::from(Span::styled(
                    "     Types: Physical, Fire, Cold, Lightning, Chaos",
                    Style::default().fg(Color::DarkGray),
                )));
            }
        }

        // Show the sub-items: attack_speed, crit_chance, spell_efficiency, then damage entries
        let dmg = bt.damage.as_ref();
        let items = [
            (
                "Attack Speed",
                dmg.map(|d| format!("{:.2}", d.attack_speed))
                    .unwrap_or_else(|| "0".to_string()),
            ),
            (
                "Crit Chance",
                dmg.map(|d| format!("{:.1}%", d.critical_chance))
                    .unwrap_or_else(|| "0%".to_string()),
            ),
            (
                "Spell Efficiency",
                dmg.map(|d| format!("{:.0}%", d.spell_efficiency))
                    .unwrap_or_else(|| "0%".to_string()),
            ),
        ];

        for (i, (name, value)) in items.iter().enumerate() {
            let is_selected = i == state.nested_index && state.nested_depth < 2;
            let marker = if is_selected { "  >> " } else { "     " };
            let style = if is_selected {
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

        // Damage entries header
        let damages_selected = state.nested_index == 3 && state.nested_depth < 2;
        let damages_marker = if damages_selected { "  >> " } else { "     " };
        lines.push(Line::from(vec![
            Span::styled(
                damages_marker.to_string(),
                Style::default().fg(Color::Green),
            ),
            Span::styled("Damage Types:", Style::default().fg(Color::Yellow)),
        ]));

        // Show damage entries
        if let Some(ref d) = dmg {
            for (i, entry) in d.damages.iter().enumerate() {
                let entry_idx = 4 + i; // Offset by 4 (3 stats + header)
                let is_selected = entry_idx == state.nested_index && state.nested_depth < 2;
                let marker = if is_selected { "     >> " } else { "        " };
                let color = match entry.damage_type {
                    loot_core::types::DamageType::Physical => Color::White,
                    loot_core::types::DamageType::Fire => Color::Red,
                    loot_core::types::DamageType::Cold => Color::Cyan,
                    loot_core::types::DamageType::Lightning => Color::Yellow,
                    loot_core::types::DamageType::Chaos => Color::Magenta,
                };
                let style = if is_selected {
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                };
                lines.push(Line::from(vec![
                    Span::styled(marker.to_string(), Style::default().fg(Color::Green)),
                    Span::styled(
                        format!("{:?}: ", entry.damage_type),
                        Style::default().fg(color),
                    ),
                    Span::styled(format!("{}-{}", entry.min, entry.max), style),
                ]));
            }
        }

        // Help text
        if state.nested_depth >= 2 {
            lines.push(Line::from(Span::styled(
                "     [Enter: save, Esc: cancel]",
                Style::default().fg(Color::DarkGray),
            )));
        } else {
            lines.push(Line::from(Span::styled(
                "     [Enter: edit, +: add damage type, Ctrl+Del: remove, Esc: back]",
                Style::default().fg(Color::DarkGray),
            )));
        }
    }

    // Requirements field (field 7)
    lines.push(render_nested_field("Requirements", &req_summary, 7, app));
    if state.field_index == 7 && state.nested_depth > 0 {
        lines.push(Line::from(""));

        if state.nested_depth >= 2 {
            // Editing mode - show input
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
            lines.push(Line::from(Span::styled(
                "     Format: level strength dexterity intelligence",
                Style::default().fg(Color::DarkGray),
            )));
            lines.push(Line::from(Span::styled(
                "     [Enter: save, Esc: cancel]",
                Style::default().fg(Color::DarkGray),
            )));
        } else {
            // Selection mode - show current values
            let req = &bt.requirements;
            lines.push(Line::from(vec![
                Span::styled("  >> ", Style::default().fg(Color::Green)),
                Span::styled(
                    format!(
                        "Lv{} Str{} Dex{} Int{}",
                        req.level, req.strength, req.dexterity, req.intelligence
                    ),
                    Style::default().fg(Color::Cyan),
                ),
            ]));
            lines.push(Line::from(Span::styled(
                "     [Enter: edit, Esc: back]",
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
        "Basic Fields:",
        Style::default().fg(Color::Gray),
    )));
    lines.push(Line::from(Span::styled(
        "  ID: Unique identifier used in config references",
        Style::default().fg(Color::DarkGray),
    )));
    lines.push(Line::from(Span::styled(
        "  Name: Display name shown to players",
        Style::default().fg(Color::DarkGray),
    )));
    lines.push(Line::from(Span::styled(
        "  Class: Item category (determines valid affixes and equipment slot)",
        Style::default().fg(Color::DarkGray),
    )));
    lines.push(Line::from(Span::styled(
        "  Tags: Keywords for affix weighting (matching tags = higher roll chance)",
        Style::default().fg(Color::DarkGray),
    )));
    lines.push(Line::from(Span::styled(
        "Implicit:",
        Style::default().fg(Color::Gray),
    )));
    lines.push(Line::from(Span::styled(
        "  Built-in stat bonus that's always present (not from affixes)",
        Style::default().fg(Color::DarkGray),
    )));
    lines.push(Line::from(Span::styled(
        "  Format: StatType min max (e.g., AddedAccuracy 10 20)",
        Style::default().fg(Color::DarkGray),
    )));
    lines.push(Line::from(Span::styled(
        "Defenses (for armour pieces):",
        Style::default().fg(Color::Gray),
    )));
    lines.push(Line::from(Span::styled(
        "  Armour: Reduces physical damage taken",
        Style::default().fg(Color::DarkGray),
    )));
    lines.push(Line::from(Span::styled(
        "  Evasion: Chance to avoid attacks entirely",
        Style::default().fg(Color::DarkGray),
    )));
    lines.push(Line::from(Span::styled(
        "  Energy Shield: Absorbs damage before life, recharges over time",
        Style::default().fg(Color::DarkGray),
    )));
    lines.push(Line::from(Span::styled(
        "Damage (for weapons):",
        Style::default().fg(Color::Gray),
    )));
    lines.push(Line::from(Span::styled(
        "  Attack Speed: Attacks per second (higher = faster)",
        Style::default().fg(Color::DarkGray),
    )));
    lines.push(Line::from(Span::styled(
        "  Crit Chance: Base % chance for critical strikes",
        Style::default().fg(Color::DarkGray),
    )));
    lines.push(Line::from(Span::styled(
        "  Spell Efficiency: % effectiveness for spells (0=melee, 80-120=caster)",
        Style::default().fg(Color::DarkGray),
    )));
    lines.push(Line::from(Span::styled(
        "  Damage Types: Each type has its own min-max range (use + to add)",
        Style::default().fg(Color::DarkGray),
    )));
    lines.push(Line::from(Span::styled(
        "    Physical/Fire/Cold/Lightning/Chaos - format: Type min max",
        Style::default().fg(Color::DarkGray),
    )));
    lines.push(Line::from(Span::styled(
        "Requirements:",
        Style::default().fg(Color::Gray),
    )));
    lines.push(Line::from(Span::styled(
        "  Minimum character stats needed to equip the item",
        Style::default().fg(Color::DarkGray),
    )));
    lines.push(Line::from(Span::styled(
        "  Level: Character level | Str/Dex/Int: Attribute requirements",
        Style::default().fg(Color::DarkGray),
    )));

    lines
}
