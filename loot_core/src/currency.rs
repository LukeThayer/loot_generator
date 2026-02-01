use crate::config::{
    CurrencyConfig, MappingMode, RecipeAffixRequirement, SpecificAffix, UniqueRecipeConfig,
};
use crate::generator::Generator;
use crate::item::{Item, Modifier};
use crate::types::*;
use rand::Rng;
use rand_chacha::ChaCha8Rng;

/// Apply a currency to an item using the generic config-driven system
pub fn apply_currency(
    generator: &Generator,
    item: &mut Item,
    currency: &CurrencyConfig,
    rng: &mut ChaCha8Rng,
) -> Result<(), CurrencyError> {
    // Check requirements
    check_requirements(generator, item, currency)?;

    // Apply effects in order
    let effects = &currency.effects;

    // 1. Set rarity (if specified)
    if let Some(new_rarity) = effects.set_rarity {
        item.rarity = new_rarity;
        if new_rarity == Rarity::Rare && item.name == item.base_name {
            item.name = generator.generate_rare_name(rng);
        }
    }

    // 2. Clear affixes (if specified)
    if effects.clear_affixes {
        item.prefixes.clear();
        item.suffixes.clear();
        // Reset name to base name if becoming normal
        if item.rarity == Rarity::Normal {
            item.name = item.base_name.clone();
        }
    }

    // 3. Remove random affixes (if specified)
    if let Some(count) = effects.remove_affixes {
        for _ in 0..count {
            remove_random_affix(item, rng)?;
        }
    }

    // 4. Reroll random affixes (if specified)
    if let Some(count) = effects.reroll_affixes {
        for _ in 0..count {
            reroll_random_affix(generator, item, &effects.affix_pools, rng)?;
        }
    }

    // 5. Add random affixes (if specified)
    if let Some(ref affix_count) = effects.add_affixes {
        let count = if affix_count.min == affix_count.max {
            affix_count.min
        } else {
            rng.gen_range(affix_count.min..=affix_count.max)
        };

        for _ in 0..count {
            if !add_random_affix(generator, item, &effects.affix_pools, rng) {
                break; // No more valid affixes or slots
            }
        }
    }

    // 6. Add specific affix from set (if specified)
    if !effects.add_specific_affix.is_empty() {
        add_specific_affix_from_set(generator, item, &effects.add_specific_affix, rng)?;
    }

    // 7. Try unique transformation (if specified)
    if effects.try_unique {
        try_unique_transformation(generator, item, rng)?;
    }

    Ok(())
}

/// Check if currency requirements are met
fn check_requirements(
    generator: &Generator,
    item: &Item,
    currency: &CurrencyConfig,
) -> Result<(), CurrencyError> {
    let reqs = &currency.requires;
    let effects = &currency.effects;

    // Check rarity requirement
    if !reqs.rarities.is_empty() && !reqs.rarities.contains(&item.rarity) {
        return Err(CurrencyError::InvalidRarity {
            expected: reqs.rarities.clone(),
            got: item.rarity,
        });
    }

    // Check has_affix requirement
    if reqs.has_affix && item.prefixes.is_empty() && item.suffixes.is_empty() {
        return Err(CurrencyError::NoAffixesToRemove);
    }

    // Check has_affix_slot requirement
    // If the currency will change rarity, check against target rarity's limits
    if reqs.has_affix_slot {
        let target_rarity = effects.set_rarity.unwrap_or(item.rarity);
        let prefix_count = if effects.clear_affixes {
            0
        } else {
            item.prefixes.len()
        };
        let suffix_count = if effects.clear_affixes {
            0
        } else {
            item.suffixes.len()
        };

        let can_add_prefix = prefix_count < target_rarity.max_prefixes();
        let can_add_suffix = suffix_count < target_rarity.max_suffixes();

        if !can_add_prefix && !can_add_suffix {
            return Err(CurrencyError::NoAffixSlots);
        }
    }

    // Check if specific affixes can actually be added (validate upfront before any changes)
    // This prevents imbue currencies from upgrading rarity when the affix can't be added
    if !effects.add_specific_affix.is_empty() {
        let target_rarity = effects.set_rarity.unwrap_or(item.rarity);
        if !can_add_any_specific_affix(
            generator,
            item,
            &effects.add_specific_affix,
            target_rarity,
            effects.clear_affixes,
        ) {
            return Err(CurrencyError::NoValidAffixes);
        }
    }

    // Check that affix_pools is specified when adding or rerolling random affixes
    let needs_pools = effects.add_affixes.is_some() || effects.reroll_affixes.is_some();
    if needs_pools && effects.affix_pools.is_empty() {
        return Err(CurrencyError::NoAffixPoolsSpecified);
    }

    Ok(())
}

/// Check if any of the specific affixes can be added to the item
/// This is used for upfront validation before applying currency effects
fn can_add_any_specific_affix(
    generator: &Generator,
    item: &Item,
    candidates: &[SpecificAffix],
    target_rarity: Rarity,
    will_clear_affixes: bool,
) -> bool {
    // Get existing affix IDs (will be empty if clearing)
    let existing: Vec<&str> = if will_clear_affixes {
        Vec::new()
    } else {
        item.prefixes
            .iter()
            .chain(item.suffixes.iter())
            .map(|m| m.affix_id.as_str())
            .collect()
    };

    // Calculate available slots based on target rarity
    let prefix_count = if will_clear_affixes {
        0
    } else {
        item.prefixes.len()
    };
    let suffix_count = if will_clear_affixes {
        0
    } else {
        item.suffixes.len()
    };
    let can_add_prefix = prefix_count < target_rarity.max_prefixes();
    let can_add_suffix = suffix_count < target_rarity.max_suffixes();

    // Check if any candidate can be added
    candidates.iter().any(|c| {
        // Check if affix exists
        let Some(affix) = generator.config().affixes.get(&c.id) else {
            return false;
        };

        // Check if already on item
        if existing.contains(&c.id.as_str()) {
            return false;
        }

        // Check if allowed for item class
        if !affix.allowed_classes.is_empty() && !affix.allowed_classes.contains(&item.class) {
            return false;
        }

        // Check if there's a slot for this affix type
        match affix.affix_type {
            AffixType::Prefix => can_add_prefix,
            AffixType::Suffix => can_add_suffix,
        }
    })
}

#[derive(Debug, Clone)]
pub enum CurrencyError {
    InvalidRarity { expected: Vec<Rarity>, got: Rarity },
    NoAffixSlots,
    NoAffixesToRemove,
    NoValidAffixes,
    NoMatchingRecipe,
    AffixNotFound(String),
    AffixAlreadyPresent(String),
    AffixNotAllowed(String),
    TierNotFound { affix_id: String, tier: u32 },
    NoAffixPoolsSpecified,
    UnknownCurrency(String),
}

impl std::fmt::Display for CurrencyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CurrencyError::InvalidRarity { expected, got } => {
                write!(f, "Invalid rarity: expected {:?}, got {:?}", expected, got)
            }
            CurrencyError::NoAffixSlots => write!(f, "No affix slots available"),
            CurrencyError::NoAffixesToRemove => write!(f, "No affixes to remove"),
            CurrencyError::NoValidAffixes => write!(f, "No valid affixes to add"),
            CurrencyError::NoMatchingRecipe => write!(f, "No matching unique recipe"),
            CurrencyError::AffixNotFound(id) => write!(f, "Affix not found: {}", id),
            CurrencyError::AffixAlreadyPresent(id) => write!(f, "Affix already on item: {}", id),
            CurrencyError::AffixNotAllowed(id) => {
                write!(f, "Affix not allowed on this item: {}", id)
            }
            CurrencyError::TierNotFound { affix_id, tier } => {
                write!(f, "Tier {} not found for affix {}", tier, affix_id)
            }
            CurrencyError::NoAffixPoolsSpecified => {
                write!(f, "No affix pools specified for currency")
            }
            CurrencyError::UnknownCurrency(id) => {
                write!(f, "Unknown currency: {}", id)
            }
        }
    }
}

impl std::error::Error for CurrencyError {}

/// Add a random affix to the item, returns false if no valid affix/slot available
/// If pools is non-empty, only affixes from those pools will be considered
fn add_random_affix(
    generator: &Generator,
    item: &mut Item,
    pools: &[String],
    rng: &mut ChaCha8Rng,
) -> bool {
    let existing: Vec<String> = item
        .prefixes
        .iter()
        .chain(item.suffixes.iter())
        .map(|m| m.affix_id.clone())
        .collect();

    let can_prefix = item.can_add_prefix();
    let can_suffix = item.can_add_suffix();

    if !can_prefix && !can_suffix {
        return false;
    }

    // Determine which type to try
    let affix_type = match (can_prefix, can_suffix) {
        (true, true) => {
            if rng.gen_bool(0.5) {
                AffixType::Prefix
            } else {
                AffixType::Suffix
            }
        }
        (true, false) => AffixType::Prefix,
        (false, true) => AffixType::Suffix,
        (false, false) => return false,
    };

    let item_level = item.requirements.level as u32;
    if let Some(modifier) = generator.roll_affix_from_pools(
        item.class, &item.tags, affix_type, &existing, pools, item_level, rng,
    ) {
        match affix_type {
            AffixType::Prefix => item.prefixes.push(modifier),
            AffixType::Suffix => item.suffixes.push(modifier),
        }
        true
    } else {
        // Try the other type if first failed
        let other_type = match affix_type {
            AffixType::Prefix => AffixType::Suffix,
            AffixType::Suffix => AffixType::Prefix,
        };

        let can_other = match other_type {
            AffixType::Prefix => can_prefix,
            AffixType::Suffix => can_suffix,
        };

        if can_other {
            if let Some(modifier) = generator.roll_affix_from_pools(
                item.class, &item.tags, other_type, &existing, pools, item_level, rng,
            ) {
                match other_type {
                    AffixType::Prefix => item.prefixes.push(modifier),
                    AffixType::Suffix => item.suffixes.push(modifier),
                }
                return true;
            }
        }
        false
    }
}

/// Add an affix from a set of possible affixes (randomly selected based on weights)
fn add_specific_affix_from_set(
    generator: &Generator,
    item: &mut Item,
    candidates: &[crate::config::SpecificAffix],
    rng: &mut ChaCha8Rng,
) -> Result<(), CurrencyError> {
    // Get existing affix IDs
    let existing: Vec<&str> = item
        .prefixes
        .iter()
        .chain(item.suffixes.iter())
        .map(|m| m.affix_id.as_str())
        .collect();

    // Filter to valid candidates
    let valid_candidates: Vec<_> = candidates
        .iter()
        .filter(|c| {
            // Check if affix exists
            let Some(affix) = generator.config().affixes.get(&c.id) else {
                return false;
            };

            // Check if already on item
            if existing.contains(&c.id.as_str()) {
                return false;
            }

            // Check if allowed for item class
            if !affix.allowed_classes.is_empty() && !affix.allowed_classes.contains(&item.class) {
                return false;
            }

            // Check if there's a slot
            match affix.affix_type {
                AffixType::Prefix => item.can_add_prefix(),
                AffixType::Suffix => item.can_add_suffix(),
            }
        })
        .collect();

    if valid_candidates.is_empty() {
        return Err(CurrencyError::NoValidAffixes);
    }

    // Select one based on weights
    let total_weight: u32 = valid_candidates.iter().map(|c| c.weight).sum();
    let selected = if total_weight == 0 || valid_candidates.len() == 1 {
        valid_candidates[0]
    } else {
        let mut roll = rng.gen_range(0..total_weight);
        let mut chosen = valid_candidates[0];
        for candidate in &valid_candidates {
            if roll < candidate.weight {
                chosen = candidate;
                break;
            }
            roll -= candidate.weight;
        }
        chosen
    };

    // Now add the selected affix
    add_affix_by_id(generator, item, &selected.id, selected.tier, rng)
}

/// Add a specific affix to the item by ID
fn add_affix_by_id(
    generator: &Generator,
    item: &mut Item,
    affix_id: &str,
    tier: Option<u32>,
    rng: &mut ChaCha8Rng,
) -> Result<(), CurrencyError> {
    // Get the affix config
    let affix = generator
        .config()
        .affixes
        .get(affix_id)
        .ok_or_else(|| CurrencyError::AffixNotFound(affix_id.to_string()))?;

    let item_level = item.requirements.level as u32;

    // Select tier
    let selected_tier = if let Some(specific_tier) = tier {
        // Use specified tier (but verify it's allowed for item level)
        let tier_cfg = affix
            .tiers
            .iter()
            .find(|t| t.tier == specific_tier)
            .ok_or_else(|| CurrencyError::TierNotFound {
                affix_id: affix_id.to_string(),
                tier: specific_tier,
            })?;
        // Allow specified tier even if item level is too low (explicit override)
        tier_cfg
    } else {
        // Roll tier based on weights, filtered by item level
        let eligible_tiers: Vec<_> = affix
            .tiers
            .iter()
            .filter(|t| t.min_ilvl <= item_level)
            .collect();

        if eligible_tiers.is_empty() {
            return Err(CurrencyError::NoValidAffixes);
        }

        let total_weight: u32 = eligible_tiers.iter().map(|t| t.weight).sum();
        if total_weight == 0 {
            return Err(CurrencyError::NoValidAffixes);
        }

        let mut roll = rng.gen_range(0..total_weight);
        let mut selected = None;
        for tier_cfg in &eligible_tiers {
            if roll < tier_cfg.weight {
                selected = Some(*tier_cfg);
                break;
            }
            roll -= tier_cfg.weight;
        }
        selected.ok_or(CurrencyError::NoValidAffixes)?
    };

    // Roll value within tier range
    let value = rng.gen_range(selected_tier.min..=selected_tier.max);

    // Roll max value if this is a damage range stat
    let value_max = selected_tier
        .max_value
        .map(|range| rng.gen_range(range.min..=range.max));

    // Create the modifier
    let modifier = Modifier {
        affix_id: affix.id.clone(),
        name: affix.name.clone(),
        stat: affix.stat,
        scope: affix.scope,
        tier: selected_tier.tier,
        value,
        value_max,
        tier_min: selected_tier.min,
        tier_max: selected_tier.max,
        tier_max_value: selected_tier.max_value.map(|r| (r.min, r.max)),
    };

    // Add to appropriate list
    match affix.affix_type {
        AffixType::Prefix => item.prefixes.push(modifier),
        AffixType::Suffix => item.suffixes.push(modifier),
    }

    Ok(())
}

/// Remove a random affix from the item
fn remove_random_affix(item: &mut Item, rng: &mut ChaCha8Rng) -> Result<(), CurrencyError> {
    let total_affixes = item.prefixes.len() + item.suffixes.len();
    if total_affixes == 0 {
        return Err(CurrencyError::NoAffixesToRemove);
    }

    let idx = rng.gen_range(0..total_affixes);

    if idx < item.prefixes.len() {
        item.prefixes.remove(idx);
    } else {
        item.suffixes.remove(idx - item.prefixes.len());
    }

    Ok(())
}

/// Reroll a random affix (remove it and add a new one of the same type)
/// If pools is non-empty, only affixes from those pools will be considered
fn reroll_random_affix(
    generator: &Generator,
    item: &mut Item,
    pools: &[String],
    rng: &mut ChaCha8Rng,
) -> Result<(), CurrencyError> {
    let prefix_count = item.prefixes.len();
    let suffix_count = item.suffixes.len();
    let total = prefix_count + suffix_count;

    if total == 0 {
        return Err(CurrencyError::NoAffixesToRemove);
    }

    let idx = rng.gen_range(0..total);
    let is_prefix = idx < prefix_count;
    let item_level = item.requirements.level as u32;

    if is_prefix {
        item.prefixes.remove(idx);

        let existing_ids: Vec<String> = item
            .prefixes
            .iter()
            .chain(item.suffixes.iter())
            .map(|m| m.affix_id.clone())
            .collect();

        if let Some(modifier) = generator.roll_affix_from_pools(
            item.class,
            &item.tags,
            AffixType::Prefix,
            &existing_ids,
            pools,
            item_level,
            rng,
        ) {
            item.prefixes.push(modifier);
        }
    } else {
        let removed_idx = idx - prefix_count;
        item.suffixes.remove(removed_idx);

        let existing_ids: Vec<String> = item
            .prefixes
            .iter()
            .chain(item.suffixes.iter())
            .map(|m| m.affix_id.clone())
            .collect();

        if let Some(modifier) = generator.roll_affix_from_pools(
            item.class,
            &item.tags,
            AffixType::Suffix,
            &existing_ids,
            pools,
            item_level,
            rng,
        ) {
            item.suffixes.push(modifier);
        }
    }

    Ok(())
}

/// Try to transform item into a unique based on recipes
fn try_unique_transformation(
    generator: &Generator,
    item: &mut Item,
    rng: &mut ChaCha8Rng,
) -> Result<(), CurrencyError> {
    // Find all matching recipes
    let matching_recipes: Vec<_> = generator
        .config()
        .unique_recipes
        .iter()
        .filter(|recipe| recipe_matches(recipe, item))
        .collect();

    if matching_recipes.is_empty() {
        return Err(CurrencyError::NoMatchingRecipe);
    }

    // Weighted random selection
    let total_weight: u32 = matching_recipes.iter().map(|r| r.weight).sum();
    let mut roll = rng.gen_range(0..total_weight);
    let mut selected_recipe = None;

    for recipe in &matching_recipes {
        if roll < recipe.weight {
            selected_recipe = Some(*recipe);
            break;
        }
        roll -= recipe.weight;
    }

    let recipe = selected_recipe.ok_or(CurrencyError::NoMatchingRecipe)?;

    // Get the unique config
    let unique = generator
        .get_unique(&recipe.unique_id)
        .ok_or(CurrencyError::NoMatchingRecipe)?;

    // Build a map of stat values from the original item's affixes
    let mut stat_values: std::collections::HashMap<StatType, (i32, String)> =
        std::collections::HashMap::new();

    for modifier in item.prefixes.iter().chain(item.suffixes.iter()) {
        stat_values.insert(modifier.stat, (modifier.value, modifier.affix_id.clone()));
    }

    // Transform the item into the unique
    item.rarity = Rarity::Unique;
    item.name = unique.name.clone();
    item.prefixes.clear();
    item.suffixes.clear();

    // Create unique mods, mapping values from original affixes where specified
    for (mod_index, mod_cfg) in unique.mods.iter().enumerate() {
        let unique_range = mod_cfg.max - mod_cfg.min;
        let random_value = rng.gen_range(mod_cfg.min..=mod_cfg.max);

        let value = if let Some(mapping) =
            recipe.mappings.iter().find(|m| m.to_mod_index == mod_index)
        {
            match mapping.mode {
                MappingMode::Random => random_value,
                MappingMode::Direct | MappingMode::Percentage => {
                    if let Some((orig_value, affix_id)) = stat_values.get(&mapping.from_stat) {
                        let mapped_value = match mapping.mode {
                            MappingMode::Direct => (*orig_value).clamp(mod_cfg.min, mod_cfg.max),
                            MappingMode::Percentage => {
                                let affix_config = generator.config().affixes.get(affix_id);

                                let percentage = if let Some(affix) = affix_config {
                                    let overall_max = affix
                                        .tiers
                                        .iter()
                                        .filter(|t| t.tier == 1)
                                        .map(|t| t.max)
                                        .next()
                                        .unwrap_or_else(|| {
                                            affix.tiers.iter().map(|t| t.max).max().unwrap_or(0)
                                        });

                                    let overall_min =
                                        affix.tiers.iter().map(|t| t.min).min().unwrap_or(0);

                                    let full_range = overall_max - overall_min;
                                    if full_range > 0 {
                                        ((*orig_value - overall_min) as f32 / full_range as f32)
                                            .clamp(0.0, 1.0)
                                    } else {
                                        0.5
                                    }
                                } else {
                                    0.5
                                };

                                mod_cfg.min + (unique_range as f32 * percentage) as i32
                            }
                            MappingMode::Random => unreachable!(),
                        };

                        let influence = mapping.influence.clamp(0.0, 1.0);
                        if influence >= 1.0 {
                            mapped_value
                        } else if influence <= 0.0 {
                            random_value
                        } else {
                            let blended = (influence * mapped_value as f32)
                                + ((1.0 - influence) * random_value as f32);
                            (blended as i32).clamp(mod_cfg.min, mod_cfg.max)
                        }
                    } else {
                        random_value
                    }
                }
            }
        } else {
            random_value
        };

        let modifier = Modifier {
            affix_id: format!("unique_{}", recipe.unique_id),
            name: unique.name.clone(),
            stat: mod_cfg.stat,
            scope: AffixScope::Global,
            tier: 0,
            value,
            value_max: None,
            tier_min: mod_cfg.min,
            tier_max: mod_cfg.max,
            tier_max_value: None,
        };
        item.prefixes.push(modifier);
    }

    Ok(())
}

/// Check if a recipe matches the given item
fn recipe_matches(recipe: &UniqueRecipeConfig, item: &Item) -> bool {
    // Check base type
    if recipe.base_type != item.base_type_id {
        return false;
    }

    // Check all required affixes
    for req in &recipe.required_affixes {
        if !affix_requirement_met(req, item) {
            return false;
        }
    }

    true
}

/// Check if a single affix requirement is met by the item
fn affix_requirement_met(req: &RecipeAffixRequirement, item: &Item) -> bool {
    let prefixes_match = item.prefixes.iter().any(|m| {
        m.stat == req.stat
            && m.tier >= req.min_tier
            && m.tier <= req.max_tier
            && (req.affix_type.is_none() || req.affix_type == Some(AffixType::Prefix))
    });

    let suffixes_match = item.suffixes.iter().any(|m| {
        m.stat == req.stat
            && m.tier >= req.min_tier
            && m.tier <= req.max_tier
            && (req.affix_type.is_none() || req.affix_type == Some(AffixType::Suffix))
    });

    prefixes_match || suffixes_match
}
