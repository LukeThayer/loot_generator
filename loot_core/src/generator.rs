use crate::config::{AffixConfig, BaseTypeConfig, Config, UniqueConfig};
use crate::currency::{apply_currency, CurrencyError};
use crate::item::{Item, Modifier};
use crate::types::*;
use rand::prelude::*;
use rand_chacha::ChaCha8Rng;

/// Item generator using seeded RNG for deterministic results
pub struct Generator {
    config: Config,
}

impl Generator {
    pub fn new(config: Config) -> Self {
        Generator { config }
    }

    pub fn config(&self) -> &Config {
        &self.config
    }

    /// Create a seeded RNG from a u64 seed
    pub fn make_rng(seed: u64) -> ChaCha8Rng {
        ChaCha8Rng::seed_from_u64(seed)
    }

    /// Generate a normal item from a base type
    pub fn generate_normal(&self, base_type_id: &str, rng: &mut ChaCha8Rng) -> Option<Item> {
        let base = self.config.base_types.get(base_type_id)?;
        let mut item = Item::new_normal(base);

        // Roll implicit if present
        if let Some(ref implicit_cfg) = base.implicit {
            let value = rng.gen_range(implicit_cfg.min..=implicit_cfg.max);
            item.implicit = Some(Modifier {
                affix_id: "implicit".to_string(),
                name: "Implicit".to_string(),
                stat: implicit_cfg.stat,
                scope: AffixScope::Local,
                tier: 0,
                value,
                value_max: None,
                tier_min: implicit_cfg.min,
                tier_max: implicit_cfg.max,
                tier_max_value: None,
            });
        }

        // Roll base defenses
        if let Some(ref def_cfg) = base.defenses {
            if let Some(range) = def_cfg.armour {
                item.defenses.armour = Some(rng.gen_range(range.min..=range.max));
            }
            if let Some(range) = def_cfg.evasion {
                item.defenses.evasion = Some(rng.gen_range(range.min..=range.max));
            }
            if let Some(range) = def_cfg.energy_shield {
                item.defenses.energy_shield = Some(rng.gen_range(range.min..=range.max));
            }
        }

        Some(item)
    }

    /// Get affixes valid for an item class
    pub fn get_valid_affixes(&self, class: ItemClass, affix_type: AffixType) -> Vec<&AffixConfig> {
        self.config
            .affixes
            .values()
            .filter(|affix| {
                affix.affix_type == affix_type
                    && (affix.allowed_classes.is_empty() || affix.allowed_classes.contains(&class))
            })
            .collect()
    }

    /// Get affixes valid for an item class, filtered by affix pools
    /// If pools is empty, returns all valid affixes
    pub fn get_valid_affixes_from_pools(
        &self,
        class: ItemClass,
        affix_type: AffixType,
        pools: &[String],
    ) -> Vec<&AffixConfig> {
        // If no pools specified, return all valid affixes
        if pools.is_empty() {
            return self.get_valid_affixes(class, affix_type);
        }

        // Build set of allowed affix IDs from specified pools
        let allowed_ids: std::collections::HashSet<&str> = pools
            .iter()
            .filter_map(|pool_id| self.config.affix_pools.get(pool_id))
            .flat_map(|pool| pool.affixes.iter().map(|s| s.as_str()))
            .collect();

        self.config
            .affixes
            .values()
            .filter(|affix| {
                affix.affix_type == affix_type
                    && (affix.allowed_classes.is_empty() || affix.allowed_classes.contains(&class))
                    && allowed_ids.contains(affix.id.as_str())
            })
            .collect()
    }

    /// Calculate spawn weight for an affix based on tag matching
    fn calculate_weight(&self, affix: &AffixConfig, item_tags: &[Tag]) -> u32 {
        let base_weight: u32 = affix.tiers.iter().map(|t| t.weight).sum();

        // Count matching tags
        let matching_tags = affix
            .tags
            .iter()
            .filter(|tag| item_tags.contains(tag))
            .count();

        // Each matching tag increases weight by 50%
        let multiplier = 1.0 + (matching_tags as f32 * 0.5);
        (base_weight as f32 * multiplier) as u32
    }

    /// Roll a random affix for an item
    pub fn roll_affix(
        &self,
        class: ItemClass,
        item_tags: &[Tag],
        affix_type: AffixType,
        existing_affix_ids: &[String],
        item_level: u32,
        rng: &mut ChaCha8Rng,
    ) -> Option<Modifier> {
        self.roll_affix_from_pools(
            class,
            item_tags,
            affix_type,
            existing_affix_ids,
            &[],
            item_level,
            rng,
        )
    }

    /// Check if an affix has at least one tag matching the item's tags
    fn has_matching_tag(affix: &AffixConfig, item_tags: &[Tag]) -> bool {
        // If affix has no tags, it can roll on anything
        if affix.tags.is_empty() {
            return true;
        }
        // Otherwise, require at least one matching tag
        affix.tags.iter().any(|tag| item_tags.contains(tag))
    }

    /// Roll a random affix for an item, filtered by affix pools
    /// If pools is empty, uses all valid affixes
    /// Affixes must have at least one matching tag with the item
    pub fn roll_affix_from_pools(
        &self,
        class: ItemClass,
        item_tags: &[Tag],
        affix_type: AffixType,
        existing_affix_ids: &[String],
        pools: &[String],
        item_level: u32,
        rng: &mut ChaCha8Rng,
    ) -> Option<Modifier> {
        let valid_affixes: Vec<_> = self
            .get_valid_affixes_from_pools(class, affix_type, pools)
            .into_iter()
            .filter(|a| !existing_affix_ids.contains(&a.id))
            // Require at least one matching tag between affix and item
            .filter(|a| Self::has_matching_tag(a, item_tags))
            .collect();

        if valid_affixes.is_empty() {
            return None;
        }

        // Calculate weights
        let weights: Vec<u32> = valid_affixes
            .iter()
            .map(|a| self.calculate_weight(a, item_tags))
            .collect();

        let total_weight: u32 = weights.iter().sum();
        if total_weight == 0 {
            return None;
        }

        // Weighted random selection for affix
        let mut roll = rng.gen_range(0..total_weight);
        let mut selected_affix = None;
        for (affix, &weight) in valid_affixes.iter().zip(weights.iter()) {
            if roll < weight {
                selected_affix = Some(*affix);
                break;
            }
            roll -= weight;
        }

        let affix = selected_affix?;

        // Select tier (weighted by tier weight, filtered by item level)
        // Only include tiers where min_ilvl <= item_level
        let eligible_tiers: Vec<_> = affix
            .tiers
            .iter()
            .filter(|t| t.min_ilvl <= item_level)
            .collect();

        if eligible_tiers.is_empty() {
            return None;
        }

        let tier_total: u32 = eligible_tiers.iter().map(|t| t.weight).sum();
        if tier_total == 0 {
            return None;
        }

        let mut tier_roll = rng.gen_range(0..tier_total);
        let mut selected_tier = None;
        for tier in &eligible_tiers {
            if tier_roll < tier.weight {
                selected_tier = Some(*tier);
                break;
            }
            tier_roll -= tier.weight;
        }

        let tier = selected_tier?;

        // Roll value within tier range
        let value = rng.gen_range(tier.min..=tier.max);

        // Roll max value if this is a damage range stat
        let value_max = tier
            .max_value
            .map(|range| rng.gen_range(range.min..=range.max));

        Some(Modifier::from_affix(affix, tier, value, value_max))
    }

    /// Add affixes to make an item magic (1-2 affixes)
    pub fn make_magic(&self, item: &mut Item, rng: &mut ChaCha8Rng) {
        item.rarity = Rarity::Magic;
        item.prefixes.clear();
        item.suffixes.clear();

        // Roll 1-2 affixes total
        let affix_count = rng.gen_range(1..=2);

        for _ in 0..affix_count {
            let existing: Vec<String> = item
                .prefixes
                .iter()
                .chain(item.suffixes.iter())
                .map(|m| m.affix_id.clone())
                .collect();

            // Randomly choose prefix or suffix if both available
            let can_prefix = item.can_add_prefix();
            let can_suffix = item.can_add_suffix();

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
                (false, false) => break,
            };

            let item_level = item.requirements.level as u32;
            if let Some(modifier) = self.roll_affix(
                item.class, &item.tags, affix_type, &existing, item_level, rng,
            ) {
                match affix_type {
                    AffixType::Prefix => item.prefixes.push(modifier),
                    AffixType::Suffix => item.suffixes.push(modifier),
                }
            }
        }
    }

    /// Add affixes to make an item rare (4-6 affixes)
    pub fn make_rare(&self, item: &mut Item, rng: &mut ChaCha8Rng) {
        item.rarity = Rarity::Rare;
        item.prefixes.clear();
        item.suffixes.clear();

        // Generate a random name
        item.name = self.generate_rare_name(rng);

        // Roll 4-6 affixes total
        let affix_count = rng.gen_range(4..=6);

        for _ in 0..affix_count {
            let existing: Vec<String> = item
                .prefixes
                .iter()
                .chain(item.suffixes.iter())
                .map(|m| m.affix_id.clone())
                .collect();

            let can_prefix = item.can_add_prefix();
            let can_suffix = item.can_add_suffix();

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
                (false, false) => break,
            };

            let item_level = item.requirements.level as u32;
            if let Some(modifier) = self.roll_affix(
                item.class, &item.tags, affix_type, &existing, item_level, rng,
            ) {
                match affix_type {
                    AffixType::Prefix => item.prefixes.push(modifier),
                    AffixType::Suffix => item.suffixes.push(modifier),
                }
            }
        }
    }

    /// Generate a random rare item name
    pub fn generate_rare_name(&self, rng: &mut ChaCha8Rng) -> String {
        const PREFIXES: &[&str] = &[
            "Doom", "Wrath", "Storm", "Dread", "Soul", "Death", "Blood", "Shadow", "Grim", "Hate",
            "Plague", "Blight", "Rune", "Spirit", "Mind", "Skull", "Bone", "Venom", "Foe", "Pain",
        ];

        const SUFFIXES: &[&str] = &[
            "Bane", "Edge", "Fang", "Bite", "Roar", "Song", "Call", "Cry", "Grasp", "Touch",
            "Strike", "Blow", "Mark", "Brand", "Scar", "Ward", "Guard", "Veil", "Shroud", "Mantle",
        ];

        let prefix = PREFIXES[rng.gen_range(0..PREFIXES.len())];
        let suffix = SUFFIXES[rng.gen_range(0..SUFFIXES.len())];

        format!("{} {}", prefix, suffix)
    }

    /// Get a base type by ID
    pub fn get_base_type(&self, id: &str) -> Option<&BaseTypeConfig> {
        self.config.base_types.get(id)
    }

    /// List all base type IDs
    pub fn base_type_ids(&self) -> Vec<&String> {
        self.config.base_types.keys().collect()
    }

    /// Get a unique by ID
    pub fn get_unique(&self, id: &str) -> Option<&UniqueConfig> {
        self.config.uniques.get(id)
    }

    /// List all unique IDs
    pub fn unique_ids(&self) -> Vec<&String> {
        self.config.uniques.keys().collect()
    }

    /// Generate a unique item
    pub fn generate_unique(&self, unique_id: &str, rng: &mut ChaCha8Rng) -> Option<Item> {
        let unique = self.config.uniques.get(unique_id)?;
        let base = self.config.base_types.get(&unique.base_type)?;

        let mut item = Item::new_normal(base);
        item.rarity = Rarity::Unique;
        item.name = unique.name.clone();

        // Roll implicit if present
        if let Some(ref implicit_cfg) = base.implicit {
            let value = rng.gen_range(implicit_cfg.min..=implicit_cfg.max);
            item.implicit = Some(Modifier {
                affix_id: "implicit".to_string(),
                name: "Implicit".to_string(),
                stat: implicit_cfg.stat,
                scope: AffixScope::Local,
                tier: 0,
                value,
                value_max: None,
                tier_min: implicit_cfg.min,
                tier_max: implicit_cfg.max,
                tier_max_value: None,
            });
        }

        // Roll base defenses
        if let Some(ref def_cfg) = base.defenses {
            if let Some(range) = def_cfg.armour {
                item.defenses.armour = Some(rng.gen_range(range.min..=range.max));
            }
            if let Some(range) = def_cfg.evasion {
                item.defenses.evasion = Some(rng.gen_range(range.min..=range.max));
            }
            if let Some(range) = def_cfg.energy_shield {
                item.defenses.energy_shield = Some(rng.gen_range(range.min..=range.max));
            }
        }

        // Roll unique mods (these go into prefixes for display, but uniques don't really have prefix/suffix distinction)
        for mod_cfg in &unique.mods {
            let value = rng.gen_range(mod_cfg.min..=mod_cfg.max);
            let modifier = Modifier {
                affix_id: format!("unique_{}", unique_id),
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

        Some(item)
    }

    /// Apply a currency to an item by currency ID
    ///
    /// Returns `None` if the currency ID doesn't exist, or `Some(Result)` with
    /// the result of applying the currency.
    pub fn apply_currency(
        &self,
        item: &mut Item,
        currency_id: &str,
        rng: &mut ChaCha8Rng,
    ) -> Option<Result<(), CurrencyError>> {
        let currency = self.config.currencies.get(currency_id)?;
        Some(apply_currency(self, item, currency, rng))
    }

    /// Check if a currency can be applied to an item
    pub fn can_apply_currency(&self, item: &Item, currency_id: &str) -> bool {
        let Some(currency) = self.config.currencies.get(currency_id) else {
            return false;
        };

        let reqs = &currency.requires;

        // Check rarity requirement
        if !reqs.rarities.is_empty() && !reqs.rarities.contains(&item.rarity) {
            return false;
        }

        // Check has_affix requirement
        if reqs.has_affix && item.prefixes.is_empty() && item.suffixes.is_empty() {
            return false;
        }

        // Check has_affix_slot requirement
        if reqs.has_affix_slot && !item.can_add_prefix() && !item.can_add_suffix() {
            return false;
        }

        true
    }
}
