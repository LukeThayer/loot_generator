use super::{
    preview_line, preview_line_colored, render_field_line, render_nested_field,
    render_section_header,
};
use crate::app::App;
use loot_core::config::{Config, UniqueConfig, UniqueRecipeConfig};
use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
};

pub fn render_preview(config: &Config, id: &str) -> Vec<Line<'static>> {
    let Some(uniq) = config.uniques.get(id) else {
        return vec![Line::from("Unique not found")];
    };

    let mut lines = vec![
        Line::from(Span::styled(
            uniq.name.clone(),
            Style::default()
                .fg(Color::Rgb(175, 95, 0))
                .add_modifier(Modifier::BOLD),
        )),
        preview_line("ID", &uniq.id),
    ];

    // Base type
    let base_name = config
        .base_types
        .get(&uniq.base_type)
        .map(|bt| bt.name.clone())
        .unwrap_or_else(|| format!("{} (not found)", uniq.base_type));
    lines.push(preview_line_colored("Base Type", &base_name, Color::White));

    // Flavor text
    if let Some(ref flavor) = uniq.flavor {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            format!("\"{}\"", flavor),
            Style::default()
                .fg(Color::DarkGray)
                .add_modifier(Modifier::ITALIC),
        )));
    }

    lines.push(Line::from(""));

    // Mods
    lines.push(render_section_header("Mods"));
    for mod_cfg in &uniq.mods {
        lines.push(Line::from(vec![
            Span::styled(
                format!("  {:?}: ", mod_cfg.stat),
                Style::default().fg(Color::Magenta),
            ),
            Span::styled(
                format!("{}-{}", mod_cfg.min, mod_cfg.max),
                Style::default().fg(Color::White),
            ),
        ]));
    }

    // Check for recipe and show details
    if let Some(recipe) = config
        .unique_recipes
        .iter()
        .find(|r| r.unique_id == uniq.id)
    {
        lines.push(Line::from(""));
        lines.push(render_section_header("Crafting Recipe"));
        lines.push(preview_line("Weight", &recipe.weight.to_string()));

        if !recipe.required_affixes.is_empty() {
            lines.push(Line::from(Span::styled(
                format!("  Required Affixes: {}", recipe.required_affixes.len()),
                Style::default().fg(Color::Yellow),
            )));
            for req in &recipe.required_affixes {
                let affix_type_str = req
                    .affix_type
                    .map(|t| format!(" ({:?})", t))
                    .unwrap_or_default();
                let tier_str = if req.min_tier == 1 && req.max_tier == 99 {
                    String::new()
                } else {
                    format!(" T{}-{}", req.min_tier, req.max_tier)
                };
                lines.push(Line::from(Span::styled(
                    format!("    {:?}{}{}", req.stat, affix_type_str, tier_str),
                    Style::default().fg(Color::White),
                )));
            }
        }

        if !recipe.mappings.is_empty() {
            lines.push(Line::from(Span::styled(
                format!("  Mappings: {}", recipe.mappings.len()),
                Style::default().fg(Color::Yellow),
            )));
            for mapping in &recipe.mappings {
                lines.push(Line::from(Span::styled(
                    format!(
                        "    {:?} -> mod[{}] ({:?}, {:.0}%)",
                        mapping.from_stat,
                        mapping.to_mod_index,
                        mapping.mode,
                        mapping.influence * 100.0
                    ),
                    Style::default().fg(Color::White),
                )));
            }
        }
    }

    lines
}

pub fn render_edit_form_with_recipe(
    uniq: &UniqueConfig,
    recipe: Option<&UniqueRecipeConfig>,
    app: &App,
) -> Vec<Line<'static>> {
    let state = app.current_view_state();
    let cursor = if matches!(app.focus, crate::app::Focus::Field(_)) {
        Some(app.text_input.cursor())
    } else {
        None
    };

    // Clone values we need
    let id = uniq.id.clone();
    let name = uniq.name.clone();
    let base_type = uniq.base_type.clone();
    let flavor = uniq.flavor.clone().unwrap_or_default();
    let mods_summary = format!("{} mods", uniq.mods.len());

    let mut lines = vec![
        render_section_header("Unique"),
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
            "Base Type",
            &base_type,
            2,
            app,
            if state.field_index == 2 { cursor } else { None },
        ),
        render_field_line(
            "Flavor",
            &flavor,
            3,
            app,
            if state.field_index == 3 { cursor } else { None },
        ),
        Line::from(""),
    ];

    // Mods - nested
    lines.push(render_nested_field("Mods", &mods_summary, 4, app));

    // If editing mods, show them
    if state.field_index == 4 && state.nested_depth > 0 {
        lines.push(Line::from(""));

        // Show input for editing
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
                    " (format: StatType min max)",
                    Style::default().fg(Color::DarkGray),
                ),
            ]));
        }

        for (i, mod_cfg) in uniq.mods.iter().enumerate() {
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
                    format!("{:?}: ", mod_cfg.stat),
                    Style::default().fg(Color::Magenta),
                ),
                Span::styled(format!("{}-{}", mod_cfg.min, mod_cfg.max), style),
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

    // Recipe section
    let recipe_summary = recipe
        .map(|r| {
            format!(
                "w:{}, {} affixes, {} mappings",
                r.weight,
                r.required_affixes.len(),
                r.mappings.len()
            )
        })
        .unwrap_or_else(|| "None (press Enter to create)".to_string());
    lines.push(render_nested_field("Recipe", &recipe_summary, 5, app));

    // Show recipe details when editing
    if state.field_index == 5 && state.nested_depth > 0 {
        lines.push(Line::from(""));

        // Recipe sub-fields
        let recipe_items: Vec<(&str, String)> = vec![
            (
                "Weight",
                recipe
                    .map(|r| r.weight.to_string())
                    .unwrap_or_else(|| "100".to_string()),
            ),
            (
                "Required Affixes",
                recipe
                    .map(|r| format!("{} item(s)", r.required_affixes.len()))
                    .unwrap_or_else(|| "0 item(s)".to_string()),
            ),
            (
                "Mappings",
                recipe
                    .map(|r| format!("{} item(s)", r.mappings.len()))
                    .unwrap_or_else(|| "0 item(s)".to_string()),
            ),
        ];

        // Show input field when editing weight or in list mode
        if state.nested_depth >= 2 {
            let input_display = format!("{}|", app.text_input.value());
            let label = match state.nested_index {
                0 => "     Edit: ",
                1 => "     Add affix: ",
                2 => "     Add mapping: ",
                _ => "     Input: ",
            };
            lines.push(Line::from(vec![
                Span::styled(label, Style::default().fg(Color::Gray)),
                Span::styled(
                    input_display,
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ),
            ]));

            // Show format hints
            let hint = match state.nested_index {
                0 => "Format: number (weight for random selection)",
                1 => "Format: StatType [Prefix|Suffix] [min_tier] [max_tier]",
                2 => "Format: StatType mod_index [percentage|direct|random] [influence]",
                _ => "",
            };
            if !hint.is_empty() {
                lines.push(Line::from(Span::styled(
                    format!("     {}", hint),
                    Style::default().fg(Color::DarkGray),
                )));
            }
        }

        for (i, (name, value)) in recipe_items.iter().enumerate() {
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

        // Show list items when in required_affixes or mappings editing
        if state.nested_index == 1 && state.nested_depth >= 2 {
            // Required affixes list
            if let Some(r) = recipe {
                if r.required_affixes.is_empty() {
                    lines.push(Line::from(Span::styled(
                        "     (no required affixes - press Enter to add)",
                        Style::default().fg(Color::DarkGray),
                    )));
                } else {
                    for (i, req) in r.required_affixes.iter().enumerate() {
                        let is_selected = i == app.nested_sub_field_index;
                        let marker = if is_selected { "     >> " } else { "        " };
                        let style = if is_selected {
                            Style::default().fg(Color::Cyan)
                        } else {
                            Style::default().fg(Color::White)
                        };
                        let affix_type_str = req
                            .affix_type
                            .map(|t| format!(" {:?}", t))
                            .unwrap_or_default();
                        lines.push(Line::from(vec![
                            Span::styled(marker.to_string(), Style::default().fg(Color::Green)),
                            Span::styled(
                                format!(
                                    "{:?}{} T{}-{}",
                                    req.stat, affix_type_str, req.min_tier, req.max_tier
                                ),
                                style,
                            ),
                        ]));
                    }
                }
            }
            lines.push(Line::from(Span::styled(
                "     [Enter: add, x: remove, Up/Down: select, Esc: back]",
                Style::default().fg(Color::DarkGray),
            )));

            // Show valid stat types
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
        } else if state.nested_index == 2 && state.nested_depth >= 2 {
            // Mappings list
            if let Some(r) = recipe {
                if r.mappings.is_empty() {
                    lines.push(Line::from(Span::styled(
                        "     (no mappings - press Enter to add)",
                        Style::default().fg(Color::DarkGray),
                    )));
                } else {
                    for (i, mapping) in r.mappings.iter().enumerate() {
                        let is_selected = i == app.nested_sub_field_index;
                        let marker = if is_selected { "     >> " } else { "        " };
                        let style = if is_selected {
                            Style::default().fg(Color::Cyan)
                        } else {
                            Style::default().fg(Color::White)
                        };
                        lines.push(Line::from(vec![
                            Span::styled(marker.to_string(), Style::default().fg(Color::Green)),
                            Span::styled(
                                format!(
                                    "{:?} -> [{}] {:?} {:.0}%",
                                    mapping.from_stat,
                                    mapping.to_mod_index,
                                    mapping.mode,
                                    mapping.influence * 100.0
                                ),
                                style,
                            ),
                        ]));
                    }
                }

                // Show available stats to map from (the required affixes)
                lines.push(Line::from(""));
                if r.required_affixes.is_empty() {
                    lines.push(Line::from(Span::styled(
                        "     ⚠ Add Required Affixes first to map from them",
                        Style::default().fg(Color::Yellow),
                    )));
                } else {
                    lines.push(Line::from(Span::styled(
                        "     Available stats to map from (Required Affixes):",
                        Style::default().fg(Color::Gray),
                    )));
                    for req in &r.required_affixes {
                        lines.push(Line::from(Span::styled(
                            format!("       {:?}", req.stat),
                            Style::default().fg(Color::DarkGray),
                        )));
                    }
                }

                // Show unique mods to map to
                lines.push(Line::from(""));
                lines.push(Line::from(Span::styled(
                    "     Unique mods to map to (by index):",
                    Style::default().fg(Color::Gray),
                )));
                for (i, mod_cfg) in uniq.mods.iter().enumerate() {
                    lines.push(Line::from(Span::styled(
                        format!(
                            "       [{}] {:?} ({}-{})",
                            i, mod_cfg.stat, mod_cfg.min, mod_cfg.max
                        ),
                        Style::default().fg(Color::DarkGray),
                    )));
                }
            }
            lines.push(Line::from(Span::styled(
                "     [Enter: add, x: remove, Up/Down: select, Esc: back]",
                Style::default().fg(Color::DarkGray),
            )));
        } else if state.nested_depth >= 2 {
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

    // Field reference footnote
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "─── Recipe Reference ───",
        Style::default().fg(Color::DarkGray),
    )));
    lines.push(Line::from(Span::styled(
        "Recipe allows crafting this unique from a matching rare/magic item.",
        Style::default().fg(Color::Gray),
    )));
    lines.push(Line::from(Span::styled(
        "  Weight: Selection weight when multiple recipes match (higher = more likely)",
        Style::default().fg(Color::DarkGray),
    )));
    lines.push(Line::from(Span::styled(
        "  Required Affixes: Stats the input item must have to trigger the recipe",
        Style::default().fg(Color::DarkGray),
    )));
    lines.push(Line::from(Span::styled(
        "    - Prefix/Suffix: Optional, restricts to that affix type only",
        Style::default().fg(Color::DarkGray),
    )));
    lines.push(Line::from(Span::styled(
        "    - Tier range: T1=best tier, higher=worse. Default 1-99 (any tier)",
        Style::default().fg(Color::DarkGray),
    )));
    lines.push(Line::from(Span::styled(
        "  Mappings: Transfer affix values from input item to unique mod values",
        Style::default().fg(Color::DarkGray),
    )));
    lines.push(Line::from(Span::styled(
        "    - from_stat: Which required affix stat to read from",
        Style::default().fg(Color::DarkGray),
    )));
    lines.push(Line::from(Span::styled(
        "    - mod_index: Which unique mod to write to (0-based index)",
        Style::default().fg(Color::DarkGray),
    )));
    lines.push(Line::from(Span::styled(
        "    Mapping Modes:",
        Style::default().fg(Color::Gray),
    )));
    lines.push(Line::from(Span::styled(
        "      percentage: If affix rolled 75% of its range, unique mod gets 75% of its range",
        Style::default().fg(Color::DarkGray),
    )));
    lines.push(Line::from(Span::styled(
        "                  Example: Affix 5-15 rolled 12 (70%) -> Unique 10-30 gets 24 (70%)",
        Style::default().fg(Color::DarkGray),
    )));
    lines.push(Line::from(Span::styled(
        "      direct: Copy the exact number (clamped to unique's range)",
        Style::default().fg(Color::DarkGray),
    )));
    lines.push(Line::from(Span::styled(
        "              Example: Affix rolled 12 -> Unique 10-30 gets 12",
        Style::default().fg(Color::DarkGray),
    )));
    lines.push(Line::from(Span::styled(
        "      random: Ignore input value, roll fresh random within unique's range",
        Style::default().fg(Color::DarkGray),
    )));
    lines.push(Line::from(Span::styled(
        "    Influence (0.0-1.0): Blends mapped value with random. 1.0=fully mapped, 0.5=half random",
        Style::default().fg(Color::DarkGray),
    )));

    lines
}

// Wrapper for backwards compatibility
pub fn render_edit_form(uniq: &UniqueConfig, app: &App) -> Vec<Line<'static>> {
    render_edit_form_with_recipe(uniq, app.editing_recipe.as_ref(), app)
}
