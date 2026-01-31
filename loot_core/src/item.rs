use crate::config::{AffixConfig, AffixTierConfig, BaseTypeConfig};
use crate::types::*;
use serde::{Deserialize, Serialize};

/// A fully realized item with all stats computed
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Item {
    /// Reference to the base type ID
    pub base_type_id: String,
    /// Display name (for rares, this is the generated name)
    pub name: String,
    /// Base type display name
    pub base_name: String,
    /// Item class
    pub class: ItemClass,
    /// Current rarity
    pub rarity: Rarity,
    /// Tags inherited from base type
    pub tags: Vec<Tag>,
    /// Requirements to equip
    pub requirements: Requirements,
    /// Implicit modifier (if any)
    pub implicit: Option<Modifier>,
    /// Rolled prefix modifiers
    pub prefixes: Vec<Modifier>,
    /// Rolled suffix modifiers
    pub suffixes: Vec<Modifier>,
    /// Base defenses (for armour)
    pub defenses: Defenses,
    /// Base damage (for weapons)
    pub damage: Option<WeaponDamage>,
}

impl Item {
    /// Create a new normal (white) item from a base type
    pub fn new_normal(base: &BaseTypeConfig) -> Self {
        let defenses = if let Some(ref def) = base.defenses {
            Defenses {
                armour: def.armour.map(|r| r.min), // Will be rolled properly with seed
                evasion: def.evasion.map(|r| r.min),
                energy_shield: def.energy_shield.map(|r| r.min),
            }
        } else {
            Defenses::default()
        };

        let damage = base.damage.as_ref().map(|d| WeaponDamage {
            min: d.min,
            max: d.max,
            attack_speed: d.attack_speed,
            critical_chance: d.critical_chance,
        });

        Item {
            base_type_id: base.id.clone(),
            name: base.name.clone(),
            base_name: base.name.clone(),
            class: base.class,
            rarity: Rarity::Normal,
            tags: base.tags.clone(),
            requirements: base.requirements.clone(),
            implicit: None, // Will be rolled with seed
            prefixes: Vec::new(),
            suffixes: Vec::new(),
            defenses,
            damage,
        }
    }

    /// Count total affixes
    pub fn affix_count(&self) -> usize {
        self.prefixes.len() + self.suffixes.len()
    }

    /// Check if item can have more prefixes
    pub fn can_add_prefix(&self) -> bool {
        self.prefixes.len() < self.rarity.max_prefixes()
    }

    /// Check if item can have more suffixes
    pub fn can_add_suffix(&self) -> bool {
        self.suffixes.len() < self.rarity.max_suffixes()
    }

    /// Export item to markdown format
    pub fn to_markdown(&self) -> String {
        let mut md = String::new();

        // Header with name
        md.push_str(&format!("## {}\n", self.name));
        md.push_str(&format!("**{}** ({:?})\n\n", self.base_name, self.rarity));

        // Defenses (for armour)
        if self.defenses.has_any() {
            md.push_str("### Defenses\n");
            if let Some(armour) = self.defenses.armour {
                md.push_str(&format!("- Armour: {}\n", armour));
            }
            if let Some(evasion) = self.defenses.evasion {
                md.push_str(&format!("- Evasion: {}\n", evasion));
            }
            if let Some(es) = self.defenses.energy_shield {
                md.push_str(&format!("- Energy Shield: {}\n", es));
            }
            md.push('\n');
        }

        // Damage (for weapons)
        if let Some(ref dmg) = self.damage {
            md.push_str("### Damage\n");
            md.push_str(&format!("- Physical: {}-{}\n", dmg.min, dmg.max));
            if dmg.attack_speed > 0.0 {
                md.push_str(&format!("- Attack Speed: {:.2}\n", dmg.attack_speed));
            }
            if dmg.critical_chance > 0.0 {
                md.push_str(&format!("- Critical Chance: {:.1}%\n", dmg.critical_chance));
            }
            md.push('\n');
        }

        // Implicit
        if let Some(ref imp) = self.implicit {
            md.push_str("### Implicit\n");
            md.push_str(&format!("- {}\n\n", imp.display()));
        }

        // Explicit mods
        if !self.prefixes.is_empty() || !self.suffixes.is_empty() {
            md.push_str("### Modifiers\n");
            for prefix in &self.prefixes {
                md.push_str(&format!("- {} (P)\n", prefix.display()));
            }
            for suffix in &self.suffixes {
                md.push_str(&format!("- {} (S)\n", suffix.display()));
            }
            md.push('\n');
        }

        // Requirements
        if self.requirements.level > 0
            || self.requirements.strength > 0
            || self.requirements.dexterity > 0
            || self.requirements.intelligence > 0
        {
            let mut reqs = Vec::new();
            if self.requirements.level > 0 {
                reqs.push(format!("Level {}", self.requirements.level));
            }
            if self.requirements.strength > 0 {
                reqs.push(format!("{} Str", self.requirements.strength));
            }
            if self.requirements.dexterity > 0 {
                reqs.push(format!("{} Dex", self.requirements.dexterity));
            }
            if self.requirements.intelligence > 0 {
                reqs.push(format!("{} Int", self.requirements.intelligence));
            }
            md.push_str(&format!("*Requires: {}*\n", reqs.join(", ")));
        }

        md
    }
}

/// Defense values on an armour piece
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Defenses {
    pub armour: Option<i32>,
    pub evasion: Option<i32>,
    pub energy_shield: Option<i32>,
}

impl Defenses {
    pub fn has_any(&self) -> bool {
        self.armour.is_some() || self.evasion.is_some() || self.energy_shield.is_some()
    }
}

/// Weapon damage values
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeaponDamage {
    pub min: i32,
    pub max: i32,
    pub attack_speed: f32,
    pub critical_chance: f32,
}

/// A rolled modifier instance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Modifier {
    /// Reference to the affix ID
    pub affix_id: String,
    /// Display name of the affix
    pub name: String,
    /// The stat this modifies
    pub stat: StatType,
    /// The rolled tier
    pub tier: u32,
    /// The rolled value within the tier's range
    pub value: i32,
    /// Minimum value for this tier
    pub tier_min: i32,
    /// Maximum value for this tier
    pub tier_max: i32,
}

impl Modifier {
    /// Create a modifier from an affix config and rolled values
    pub fn from_affix(affix: &AffixConfig, tier: &AffixTierConfig, value: i32) -> Self {
        Modifier {
            affix_id: affix.id.clone(),
            name: affix.name.clone(),
            stat: affix.stat,
            tier: tier.tier,
            value,
            tier_min: tier.min,
            tier_max: tier.max,
        }
    }

    /// Display the modifier as a human-readable string
    pub fn display(&self) -> String {
        let stat_name = format!("{:?}", self.stat)
            .chars()
            .fold(String::new(), |mut acc, c| {
                if c.is_uppercase() && !acc.is_empty() {
                    acc.push(' ');
                }
                acc.push(c);
                acc
            });

        // Determine if this is a percentage or flat value based on stat type
        let is_percent = matches!(
            self.stat,
            StatType::IncreasedPhysicalDamage
                | StatType::IncreasedElementalDamage
                | StatType::IncreasedAttackSpeed
                | StatType::IncreasedCriticalChance
                | StatType::IncreasedCriticalDamage
                | StatType::IncreasedArmour
                | StatType::IncreasedEvasion
                | StatType::IncreasedEnergyShield
                | StatType::IncreasedLife
                | StatType::IncreasedMana
                | StatType::IncreasedAccuracy
                | StatType::IncreasedMovementSpeed
                | StatType::IncreasedItemRarity
                | StatType::IncreasedItemQuantity
                | StatType::FireResistance
                | StatType::ColdResistance
                | StatType::LightningResistance
                | StatType::ChaosResistance
                | StatType::AllResistances
                | StatType::LifeLeech
                | StatType::ManaLeech
        );

        if is_percent {
            format!("+{}% {}", self.value, stat_name)
        } else {
            format!("+{} {}", self.value, stat_name)
        }
    }
}
