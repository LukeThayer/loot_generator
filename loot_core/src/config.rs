use crate::types::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

/// Complete game configuration loaded from TOML files
#[derive(Debug, Clone, Default)]
pub struct Config {
    pub base_types: HashMap<String, BaseTypeConfig>,
    pub affixes: HashMap<String, AffixConfig>,
    pub affix_pools: HashMap<String, AffixPoolConfig>,
    pub currencies: HashMap<String, CurrencyConfig>,
    pub uniques: HashMap<String, UniqueConfig>,
    pub unique_recipes: Vec<UniqueRecipeConfig>,
}

impl Config {
    /// Load configuration from a directory containing subdirectories for each config type
    /// Expected structure:
    ///   config/
    ///     base_types/    - .toml files containing [[base_types]] arrays
    ///     affixes/       - .toml files containing [[affixes]] arrays
    ///     affix_pools/   - .toml files containing [[pools]] arrays
    ///     currencies/    - .toml files containing [[currencies]] arrays
    ///     uniques/       - .toml files each containing [unique] and optional [recipe]
    pub fn load_from_dir(dir: &Path) -> Result<Self, ConfigError> {
        let base_types = Self::load_base_types_dir(&dir.join("base_types"))?;
        let affixes = Self::load_affixes_dir(&dir.join("affixes"))?;
        let affix_pools = Self::load_affix_pools_dir(&dir.join("affix_pools"))?;
        let currencies = Self::load_currencies_dir(&dir.join("currencies"))?;
        let (uniques, unique_recipes) = Self::load_uniques_dir(&dir.join("uniques"))?;

        Ok(Config {
            base_types,
            affixes,
            affix_pools,
            currencies,
            uniques,
            unique_recipes,
        })
    }

    /// Load all base type files from a directory
    /// Each file can contain one or more [[base_types]] entries
    fn load_base_types_dir(dir: &Path) -> Result<HashMap<String, BaseTypeConfig>, ConfigError> {
        let mut result = HashMap::new();

        if !dir.exists() {
            return Ok(result);
        }

        for entry in Self::read_dir_with_context(dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.extension().map_or(false, |ext| ext == "toml") {
                let content = Self::read_file_with_context(&path)?;
                let wrapper: BaseTypesWrapper = Self::parse_toml_with_context(&content, &path)?;
                for bt in wrapper.base_types {
                    result.insert(bt.id.clone(), bt);
                }
            }
        }

        Ok(result)
    }

    /// Load all affix files from a directory
    /// Each file can contain one or more [[affixes]] entries
    fn load_affixes_dir(dir: &Path) -> Result<HashMap<String, AffixConfig>, ConfigError> {
        let mut result = HashMap::new();

        if !dir.exists() {
            return Ok(result);
        }

        for entry in Self::read_dir_with_context(dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.extension().map_or(false, |ext| ext == "toml") {
                let content = Self::read_file_with_context(&path)?;
                let wrapper: AffixesWrapper = Self::parse_toml_with_context(&content, &path)?;
                for affix in wrapper.affixes {
                    result.insert(affix.id.clone(), affix);
                }
            }
        }

        Ok(result)
    }

    /// Load all affix pool files from a directory
    /// Each file can contain one or more [[pools]] entries
    fn load_affix_pools_dir(dir: &Path) -> Result<HashMap<String, AffixPoolConfig>, ConfigError> {
        let mut result = HashMap::new();

        if !dir.exists() {
            return Ok(result);
        }

        for entry in Self::read_dir_with_context(dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.extension().map_or(false, |ext| ext == "toml") {
                let content = Self::read_file_with_context(&path)?;
                let wrapper: AffixPoolsWrapper = Self::parse_toml_with_context(&content, &path)?;
                for pool in wrapper.pools {
                    result.insert(pool.id.clone(), pool);
                }
            }
        }

        Ok(result)
    }

    /// Load all currency files from a directory
    /// Each file can contain one or more [[currencies]] entries
    fn load_currencies_dir(dir: &Path) -> Result<HashMap<String, CurrencyConfig>, ConfigError> {
        let mut result = HashMap::new();

        if !dir.exists() {
            return Ok(result);
        }

        for entry in Self::read_dir_with_context(dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.extension().map_or(false, |ext| ext == "toml") {
                let content = Self::read_file_with_context(&path)?;
                let wrapper: CurrenciesWrapper = Self::parse_toml_with_context(&content, &path)?;
                for currency in wrapper.currencies {
                    result.insert(currency.id.clone(), currency);
                }
            }
        }

        Ok(result)
    }

    /// Load all unique files from a directory
    /// Each file contains a unique definition and optionally a recipe
    fn load_uniques_dir(
        dir: &Path,
    ) -> Result<(HashMap<String, UniqueConfig>, Vec<UniqueRecipeConfig>), ConfigError> {
        let mut uniques = HashMap::new();
        let mut recipes = Vec::new();

        if !dir.exists() {
            return Ok((uniques, recipes));
        }

        for entry in Self::read_dir_with_context(dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.extension().map_or(false, |ext| ext == "toml") {
                let content = Self::read_file_with_context(&path)?;
                let file_config: UniqueFileConfig = Self::parse_toml_with_context(&content, &path)?;

                let unique_id = file_config.unique.id.clone();
                let base_type = file_config.unique.base_type.clone();

                uniques.insert(unique_id.clone(), file_config.unique);

                // If there's a recipe, add it with the unique_id and base_type filled in
                if let Some(mut recipe) = file_config.recipe {
                    recipe.unique_id = unique_id;
                    recipe.base_type = base_type;
                    recipes.push(recipe);
                }
            }
        }

        Ok((uniques, recipes))
    }

    // Helper functions for error context

    fn read_dir_with_context(dir: &Path) -> Result<std::fs::ReadDir, ConfigError> {
        std::fs::read_dir(dir).map_err(|e| ConfigError::Io {
            error: e,
            path: Some(dir.to_path_buf()),
        })
    }

    fn read_file_with_context(path: &Path) -> Result<String, ConfigError> {
        std::fs::read_to_string(path).map_err(|e| ConfigError::Io {
            error: e,
            path: Some(path.to_path_buf()),
        })
    }

    fn parse_toml_with_context<T: serde::de::DeserializeOwned>(
        content: &str,
        path: &Path,
    ) -> Result<T, ConfigError> {
        toml::from_str(content).map_err(|e| ConfigError::Parse {
            error: e,
            path: path.to_path_buf(),
        })
    }
}

#[derive(Debug)]
pub enum ConfigError {
    /// IO error with optional file path
    Io {
        error: std::io::Error,
        path: Option<std::path::PathBuf>,
    },
    /// TOML parse error with file path and location details
    Parse {
        error: toml::de::Error,
        path: std::path::PathBuf,
    },
}

impl ConfigError {
    /// Get the file path associated with this error, if any
    pub fn file_path(&self) -> Option<&std::path::Path> {
        match self {
            ConfigError::Io { path, .. } => path.as_deref(),
            ConfigError::Parse { path, .. } => Some(path),
        }
    }

    /// Get a user-friendly description of where the error occurred
    pub fn location_description(&self) -> String {
        match self {
            ConfigError::Io { path: Some(p), .. } => {
                format!("File: {}", p.display())
            }
            ConfigError::Io { path: None, .. } => "Unknown location".to_string(),
            ConfigError::Parse { error, path } => {
                let mut desc = format!("File: {}", path.display());
                if let Some(span) = error.span() {
                    desc.push_str(&format!("\nPosition: bytes {}..{}", span.start, span.end));
                }
                desc
            }
        }
    }

    /// Get the underlying error message
    pub fn error_message(&self) -> String {
        match self {
            ConfigError::Io { error, .. } => error.to_string(),
            ConfigError::Parse { error, .. } => {
                // Extract just the message part, not the span info (we show that separately)
                let msg = error.message();
                msg.to_string()
            }
        }
    }
}

impl From<std::io::Error> for ConfigError {
    fn from(e: std::io::Error) -> Self {
        ConfigError::Io {
            error: e,
            path: None,
        }
    }
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigError::Io {
                error,
                path: Some(p),
            } => {
                write!(f, "IO error in '{}': {}", p.display(), error)
            }
            ConfigError::Io { error, path: None } => {
                write!(f, "IO error: {}", error)
            }
            ConfigError::Parse { error, path } => {
                write!(f, "Parse error in '{}': {}", path.display(), error)
            }
        }
    }
}

impl std::error::Error for ConfigError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ConfigError::Io { error, .. } => Some(error),
            ConfigError::Parse { error, .. } => Some(error),
        }
    }
}

// Wrapper types for TOML parsing

#[derive(Deserialize)]
struct BaseTypesWrapper {
    #[serde(default)]
    base_types: Vec<BaseTypeConfig>,
}

#[derive(Deserialize)]
struct AffixesWrapper {
    #[serde(default)]
    affixes: Vec<AffixConfig>,
}

#[derive(Deserialize)]
struct AffixPoolsWrapper {
    #[serde(default)]
    pools: Vec<AffixPoolConfig>,
}

#[derive(Deserialize)]
struct CurrenciesWrapper {
    #[serde(default)]
    currencies: Vec<CurrencyConfig>,
}

/// Config structure for individual unique files
/// Each file contains the unique definition and optionally a recipe
#[derive(Deserialize)]
struct UniqueFileConfig {
    unique: UniqueConfig,
    #[serde(default)]
    recipe: Option<UniqueRecipeConfig>,
}

/// Base item type configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaseTypeConfig {
    pub id: String,
    pub name: String,
    pub class: ItemClass,
    #[serde(default)]
    pub tags: Vec<Tag>,
    #[serde(default)]
    pub implicit: Option<ImplicitConfig>,
    #[serde(default)]
    pub defenses: Option<DefensesConfig>,
    #[serde(default)]
    pub damage: Option<DamageConfig>,
    #[serde(default)]
    pub requirements: Requirements,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImplicitConfig {
    pub stat: StatType,
    pub min: i32,
    pub max: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DefensesConfig {
    #[serde(default)]
    pub armour: Option<RollRange>,
    #[serde(default)]
    pub evasion: Option<RollRange>,
    #[serde(default)]
    pub energy_shield: Option<RollRange>,
}

/// Individual damage type with its own range
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DamageEntry {
    #[serde(rename = "type")]
    pub damage_type: DamageType,
    pub min: i32,
    pub max: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DamageConfig {
    /// List of damage types, each with their own min/max range
    #[serde(default)]
    pub damages: Vec<DamageEntry>,
    #[serde(default)]
    pub attack_speed: f32,
    #[serde(default)]
    pub critical_chance: f32,
    /// Spell efficiency percentage (for casting weapons like wands/staves)
    #[serde(default)]
    pub spell_efficiency: f32,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct RollRange {
    pub min: i32,
    pub max: i32,
}

/// Affix configuration with tiers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AffixConfig {
    pub id: String,
    pub name: String,
    #[serde(rename = "type")]
    pub affix_type: AffixType,
    pub stat: StatType,
    /// Whether this modifier applies locally to the item or globally to the character
    #[serde(default)]
    pub scope: AffixScope,
    #[serde(default)]
    pub tags: Vec<Tag>,
    #[serde(default)]
    pub allowed_classes: Vec<ItemClass>,
    pub tiers: Vec<AffixTierConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AffixTierConfig {
    pub tier: u32,
    pub weight: u32,
    /// Minimum value (or minimum of the low range for damage stats)
    pub min: i32,
    /// Maximum value (or maximum of the low range for damage stats)
    pub max: i32,
    /// For damage range stats: the range for the high end of damage
    /// When present, this stat becomes a range (e.g., "Adds 5-10 Fire Damage")
    #[serde(default)]
    pub max_value: Option<RollRange>,
    /// Minimum item level required for this tier to roll
    #[serde(default)]
    pub min_ilvl: u32,
}

/// Affix pool configuration - groups of affixes that can be referenced by currencies
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AffixPoolConfig {
    pub id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub description: String,
    /// List of affix IDs in this pool
    pub affixes: Vec<String>,
}

/// Currency configuration - generic and data-driven
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurrencyConfig {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
    /// Category for UI organization (e.g., "Rarity", "Affixes", "Elemental")
    #[serde(default)]
    pub category: String,
    /// Requirements to use this currency
    #[serde(default)]
    pub requires: CurrencyRequirements,
    /// Effects when currency is applied
    #[serde(default)]
    pub effects: CurrencyEffects,
}

/// Requirements for using a currency
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CurrencyRequirements {
    /// Item must be one of these rarities
    #[serde(default)]
    pub rarities: Vec<Rarity>,
    /// Item must have at least one affix
    #[serde(default)]
    pub has_affix: bool,
    /// Item must have room for at least one more affix
    #[serde(default)]
    pub has_affix_slot: bool,
}

/// Effects when a currency is applied
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CurrencyEffects {
    /// Set the item's rarity
    #[serde(default)]
    pub set_rarity: Option<Rarity>,
    /// Remove all existing affixes before other effects
    #[serde(default)]
    pub clear_affixes: bool,
    /// Add this many random affixes (can be a range)
    #[serde(default)]
    pub add_affixes: Option<AffixCount>,
    /// Add a specific affix from a set of possible affixes (randomly selected if multiple)
    #[serde(default)]
    pub add_specific_affix: Vec<SpecificAffix>,
    /// Remove this many random affixes
    #[serde(default)]
    pub remove_affixes: Option<u32>,
    /// Reroll this many random affixes (remove and re-add)
    #[serde(default)]
    pub reroll_affixes: Option<u32>,
    /// Try to transform into a unique based on recipes
    #[serde(default)]
    pub try_unique: bool,
    /// Affix pools to draw from when adding random affixes (if empty, uses all affixes)
    #[serde(default)]
    pub affix_pools: Vec<String>,
}

/// Specifies a specific affix to add
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpecificAffix {
    /// The affix ID to add
    pub id: String,
    /// Optional specific tier (if not specified, rolls randomly based on weights)
    #[serde(default)]
    pub tier: Option<u32>,
    /// Weight for random selection (default 100)
    #[serde(default = "default_affix_weight")]
    pub weight: u32,
}

fn default_affix_weight() -> u32 {
    100
}

/// Specifies how many affixes to add
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AffixCount {
    pub min: u32,
    pub max: u32,
}

/// Unique item template configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UniqueConfig {
    pub id: String,
    pub name: String,
    pub base_type: String,
    #[serde(default)]
    pub flavor: Option<String>,
    pub mods: Vec<UniqueModConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UniqueModConfig {
    pub stat: StatType,
    pub min: i32,
    pub max: i32,
}

/// Recipe for crafting a unique item from a rare/magic item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UniqueRecipeConfig {
    /// The unique item this recipe creates (filled in from unique config)
    #[serde(default)]
    pub unique_id: String,
    /// Required base type (filled in from unique config)
    #[serde(default)]
    pub base_type: String,
    /// Weight for random selection when multiple recipes match
    #[serde(default = "default_weight")]
    pub weight: u32,
    /// Required affixes on the item
    pub required_affixes: Vec<RecipeAffixRequirement>,
    /// How to map original affix values to unique mod values
    #[serde(default)]
    pub mappings: Vec<RecipeMapping>,
}

fn default_weight() -> u32 {
    100
}

/// Requirement for an affix in a unique recipe
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecipeAffixRequirement {
    /// The stat type required
    pub stat: StatType,
    /// Whether this must be a prefix or suffix (optional - if not specified, either works)
    #[serde(default)]
    pub affix_type: Option<AffixType>,
    /// Minimum tier allowed (1 = best, higher = worse). Default 1.
    #[serde(default = "default_min_tier")]
    pub min_tier: u32,
    /// Maximum tier allowed. Default 99 (any tier).
    #[serde(default = "default_max_tier")]
    pub max_tier: u32,
}

fn default_min_tier() -> u32 {
    1
}

fn default_max_tier() -> u32 {
    99
}

/// Mapping from original item affix to unique mod value
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecipeMapping {
    /// Source stat from the original item's affix
    pub from_stat: StatType,
    /// Target stat on the unique (index into unique's mods)
    pub to_mod_index: usize,
    /// How to map the value: "percentage" (default), "direct", or "random"
    #[serde(default = "default_mapping_mode")]
    pub mode: MappingMode,
    /// How much the original value influences the result (0.0-1.0, default 1.0)
    /// At 1.0, result is fully determined by original. At 0.0, result is random.
    /// Values in between blend the mapped value with a random roll.
    #[serde(default = "default_influence")]
    pub influence: f32,
}

fn default_mapping_mode() -> MappingMode {
    MappingMode::Percentage
}

fn default_influence() -> f32 {
    1.0
}

/// How to map original affix values to unique mod values
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MappingMode {
    /// Map quality percentage (0-100% of affix range) to same percentage of unique range
    Percentage,
    /// Transfer the numeric value directly (clamped to unique's range)
    Direct,
    /// Ignore original value, roll randomly within unique's range
    Random,
}
