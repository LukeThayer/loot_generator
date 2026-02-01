use serde::{Deserialize, Serialize};

/// Core attributes for requirements and scaling
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Attribute {
    Strength,
    Dexterity,
    Constitution,
    Intelligence,
    Wisdom,
    Charisma,
}

/// Defense types for armour pieces
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DefenseType {
    Armour,
    Evasion,
    EnergyShield,
}

/// Damage types for weapons
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum DamageType {
    #[default]
    Physical,
    Fire,
    Cold,
    Lightning,
    Chaos,
}

/// Status effect types that damage can be converted to
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StatusEffect {
    Freeze,
    Chill,
    Burn,
    Fear,
    Slow,
    Static,
    Poison,
    Bleed,
}

/// Item rarity levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum Rarity {
    #[default]
    Normal,
    Magic,
    Rare,
    Unique,
}

impl Rarity {
    pub fn max_prefixes(&self) -> usize {
        match self {
            Rarity::Normal => 0,
            Rarity::Magic => 1,
            Rarity::Rare => 3,
            Rarity::Unique => 0, // Uniques have fixed mods
        }
    }

    pub fn max_suffixes(&self) -> usize {
        match self {
            Rarity::Normal => 0,
            Rarity::Magic => 1,
            Rarity::Rare => 3,
            Rarity::Unique => 0,
        }
    }
}

/// Granular item class categories
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ItemClass {
    // One-handed weapons
    OneHandSword,
    OneHandAxe,
    OneHandMace,
    Dagger,
    Claw,
    Wand,
    // Two-handed weapons
    TwoHandSword,
    TwoHandAxe,
    TwoHandMace,
    Bow,
    Staff,
    // Off-hand
    Shield,
    // Armour
    Helmet,
    BodyArmour,
    Gloves,
    Boots,
    // Accessories
    Ring,
    Amulet,
    Belt,
}

impl ItemClass {
    pub fn is_weapon(&self) -> bool {
        matches!(
            self,
            ItemClass::OneHandSword
                | ItemClass::OneHandAxe
                | ItemClass::OneHandMace
                | ItemClass::Dagger
                | ItemClass::Claw
                | ItemClass::Wand
                | ItemClass::TwoHandSword
                | ItemClass::TwoHandAxe
                | ItemClass::TwoHandMace
                | ItemClass::Bow
                | ItemClass::Staff
        )
    }

    pub fn is_armour(&self) -> bool {
        matches!(
            self,
            ItemClass::Helmet
                | ItemClass::BodyArmour
                | ItemClass::Gloves
                | ItemClass::Boots
                | ItemClass::Shield
        )
    }

    pub fn is_accessory(&self) -> bool {
        matches!(self, ItemClass::Ring | ItemClass::Amulet | ItemClass::Belt)
    }
}

/// Affix type: prefix or suffix
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AffixType {
    Prefix,
    Suffix,
}

/// Affix scope: whether the modifier applies locally to the item or globally to the character
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum AffixScope {
    /// Modifier applies to the item's base stats (e.g., added damage on a weapon)
    #[default]
    Local,
    /// Modifier applies to the character's stats (e.g., added damage to all attacks)
    Global,
}

/// Stat modifier types that affixes can grant
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StatType {
    // Flat additions
    AddedPhysicalDamage,
    AddedFireDamage,
    AddedColdDamage,
    AddedLightningDamage,
    AddedChaosDamage,
    // Percentage increases
    IncreasedPhysicalDamage,
    IncreasedFireDamage,
    IncreasedColdDamage,
    IncreasedLightningDamage,
    IncreasedElementalDamage,
    IncreasedChaosDamage,
    IncreasedAttackSpeed,
    IncreasedCriticalChance,
    IncreasedCriticalDamage,
    // Status effect - Poison
    PoisonDamageOverTime,
    IncreasedPoisonDuration,
    PoisonMagnitude,
    PoisonMaxStacks,
    ConvertPhysicalToPoison,
    ConvertFireToPoison,
    ConvertColdToPoison,
    ConvertLightningToPoison,
    ConvertChaosToPoison,
    // Status effect - Bleed
    BleedDamageOverTime,
    IncreasedBleedDuration,
    BleedMagnitude,
    BleedMaxStacks,
    ConvertPhysicalToBleed,
    ConvertFireToBleed,
    ConvertColdToBleed,
    ConvertLightningToBleed,
    ConvertChaosToBleed,
    // Status effect - Burn
    BurnDamageOverTime,
    IncreasedBurnDuration,
    BurnMagnitude,
    BurnMaxStacks,
    ConvertPhysicalToBurn,
    ConvertFireToBurn,
    ConvertColdToBurn,
    ConvertLightningToBurn,
    ConvertChaosToBurn,
    // Status effect - Freeze
    IncreasedFreezeDuration,
    FreezeMagnitude,
    FreezeMaxStacks,
    ConvertPhysicalToFreeze,
    ConvertFireToFreeze,
    ConvertColdToFreeze,
    ConvertLightningToFreeze,
    ConvertChaosToFreeze,
    // Status effect - Chill
    IncreasedChillDuration,
    ChillMagnitude,
    ChillMaxStacks,
    ConvertPhysicalToChill,
    ConvertFireToChill,
    ConvertColdToChill,
    ConvertLightningToChill,
    ConvertChaosToChill,
    // Status effect - Static
    IncreasedStaticDuration,
    StaticMagnitude,
    StaticMaxStacks,
    ConvertPhysicalToStatic,
    ConvertFireToStatic,
    ConvertColdToStatic,
    ConvertLightningToStatic,
    ConvertChaosToStatic,
    // Status effect - Fear
    IncreasedFearDuration,
    FearMagnitude,
    FearMaxStacks,
    ConvertPhysicalToFear,
    ConvertFireToFear,
    ConvertColdToFear,
    ConvertLightningToFear,
    ConvertChaosToFear,
    // Status effect - Slow
    IncreasedSlowDuration,
    SlowMagnitude,
    SlowMaxStacks,
    ConvertPhysicalToSlow,
    ConvertFireToSlow,
    ConvertColdToSlow,
    ConvertLightningToSlow,
    ConvertChaosToSlow,
    // Defenses
    AddedArmour,
    AddedEvasion,
    AddedEnergyShield,
    IncreasedArmour,
    IncreasedEvasion,
    IncreasedEnergyShield,
    // Attributes
    AddedStrength,
    AddedDexterity,
    AddedConstitution,
    AddedIntelligence,
    AddedWisdom,
    AddedCharisma,
    AddedAllAttributes,
    // Life and resources
    AddedLife,
    AddedMana,
    IncreasedLife,
    IncreasedMana,
    LifeRegeneration,
    ManaRegeneration,
    LifeOnHit,
    LifeLeech,
    ManaLeech,
    // Resistances
    FireResistance,
    ColdResistance,
    LightningResistance,
    ChaosResistance,
    AllResistances,
    // Accuracy and utility
    AddedAccuracy,
    IncreasedAccuracy,
    IncreasedMovementSpeed,
    IncreasedItemRarity,
    IncreasedItemQuantity,
}

/// Attribute requirements for equipping an item
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Requirements {
    #[serde(default)]
    pub level: u32,
    #[serde(default)]
    pub strength: u32,
    #[serde(default)]
    pub dexterity: u32,
    #[serde(default)]
    pub constitution: u32,
    #[serde(default)]
    pub intelligence: u32,
    #[serde(default)]
    pub wisdom: u32,
    #[serde(default)]
    pub charisma: u32,
}

/// A tag used for spawn weighting
pub type Tag = String;
