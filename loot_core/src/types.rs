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
}

/// Affix type: prefix or suffix
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AffixType {
    Prefix,
    Suffix,
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
    IncreasedElementalDamage,
    IncreasedChaosDamage,
    IncreasedAttackSpeed,
    IncreasedCriticalChance,
    IncreasedCriticalDamage,
    // Poison/Ailment
    PoisonDamageOverTime,
    ChanceToPoison,
    IncreasedPoisonDuration,
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
