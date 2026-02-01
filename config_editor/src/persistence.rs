use crate::app::ConfigTab;
use loot_core::config::{
    AffixConfig, AffixPoolConfig, BaseTypeConfig, Config, CurrencyConfig, UniqueConfig,
};
use serde::Serialize;
use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// Tracks the origin file for each config entry
#[derive(Debug, Default)]
pub struct ConfigOrigins {
    pub base_types: HashMap<String, PathBuf>,
    pub affixes: HashMap<String, PathBuf>,
    pub affix_pools: HashMap<String, PathBuf>,
    pub currencies: HashMap<String, PathBuf>,
    pub uniques: HashMap<String, PathBuf>,
}

impl ConfigOrigins {
    /// Load origins by scanning config directory
    pub fn load_from_dir(config_dir: &Path) -> Self {
        let mut origins = Self::default();

        // Scan base_types
        if let Ok(entries) = fs::read_dir(config_dir.join("base_types")) {
            for entry in entries.filter_map(|e| e.ok()) {
                let path = entry.path();
                if path.extension().map_or(false, |ext| ext == "toml") {
                    if let Ok(content) = fs::read_to_string(&path) {
                        if let Ok(wrapper) = toml::from_str::<BaseTypesWrapper>(&content) {
                            for bt in wrapper.base_types {
                                origins.base_types.insert(bt.id.clone(), path.clone());
                            }
                        }
                    }
                }
            }
        }

        // Scan affixes
        if let Ok(entries) = fs::read_dir(config_dir.join("affixes")) {
            for entry in entries.filter_map(|e| e.ok()) {
                let path = entry.path();
                if path.extension().map_or(false, |ext| ext == "toml") {
                    if let Ok(content) = fs::read_to_string(&path) {
                        if let Ok(wrapper) = toml::from_str::<AffixesWrapper>(&content) {
                            for affix in wrapper.affixes {
                                origins.affixes.insert(affix.id.clone(), path.clone());
                            }
                        }
                    }
                }
            }
        }

        // Scan affix_pools
        if let Ok(entries) = fs::read_dir(config_dir.join("affix_pools")) {
            for entry in entries.filter_map(|e| e.ok()) {
                let path = entry.path();
                if path.extension().map_or(false, |ext| ext == "toml") {
                    if let Ok(content) = fs::read_to_string(&path) {
                        if let Ok(wrapper) = toml::from_str::<AffixPoolsWrapper>(&content) {
                            for pool in wrapper.pools {
                                origins.affix_pools.insert(pool.id.clone(), path.clone());
                            }
                        }
                    }
                }
            }
        }

        // Scan currencies
        if let Ok(entries) = fs::read_dir(config_dir.join("currencies")) {
            for entry in entries.filter_map(|e| e.ok()) {
                let path = entry.path();
                if path.extension().map_or(false, |ext| ext == "toml") {
                    if let Ok(content) = fs::read_to_string(&path) {
                        if let Ok(wrapper) = toml::from_str::<CurrenciesWrapper>(&content) {
                            for curr in wrapper.currencies {
                                origins.currencies.insert(curr.id.clone(), path.clone());
                            }
                        }
                    }
                }
            }
        }

        // Scan uniques (each file is one unique)
        if let Ok(entries) = fs::read_dir(config_dir.join("uniques")) {
            for entry in entries.filter_map(|e| e.ok()) {
                let path = entry.path();
                if path.extension().map_or(false, |ext| ext == "toml") {
                    if let Ok(content) = fs::read_to_string(&path) {
                        if let Ok(file_cfg) = toml::from_str::<UniqueFileConfig>(&content) {
                            origins
                                .uniques
                                .insert(file_cfg.unique.id.clone(), path.clone());
                        }
                    }
                }
            }
        }

        origins
    }

    pub fn get_origin(&self, tab: ConfigTab, id: &str) -> Option<&PathBuf> {
        match tab {
            ConfigTab::BaseTypes => self.base_types.get(id),
            ConfigTab::Affixes => self.affixes.get(id),
            ConfigTab::AffixPools => self.affix_pools.get(id),
            ConfigTab::Currencies => self.currencies.get(id),
            ConfigTab::Uniques => self.uniques.get(id),
        }
    }

    pub fn set_origin(&mut self, tab: ConfigTab, id: &str, path: PathBuf) {
        match tab {
            ConfigTab::BaseTypes => {
                self.base_types.insert(id.to_string(), path);
            }
            ConfigTab::Affixes => {
                self.affixes.insert(id.to_string(), path);
            }
            ConfigTab::AffixPools => {
                self.affix_pools.insert(id.to_string(), path);
            }
            ConfigTab::Currencies => {
                self.currencies.insert(id.to_string(), path);
            }
            ConfigTab::Uniques => {
                self.uniques.insert(id.to_string(), path);
            }
        }
    }
}

// Wrapper types for TOML parsing (same as in config.rs)
#[derive(serde::Deserialize)]
struct BaseTypesWrapper {
    #[serde(default)]
    base_types: Vec<BaseTypeConfig>,
}

#[derive(serde::Deserialize)]
struct AffixesWrapper {
    #[serde(default)]
    affixes: Vec<AffixConfig>,
}

#[derive(serde::Deserialize)]
struct AffixPoolsWrapper {
    #[serde(default)]
    pools: Vec<AffixPoolConfig>,
}

#[derive(serde::Deserialize)]
struct CurrenciesWrapper {
    #[serde(default)]
    currencies: Vec<CurrencyConfig>,
}

#[derive(serde::Deserialize)]
struct UniqueFileConfig {
    unique: UniqueConfig,
    #[serde(default)]
    recipe: Option<toml::Value>,
}

// Serialization wrapper types
#[derive(Serialize)]
struct BaseTypesWrapperSer<'a> {
    base_types: Vec<&'a BaseTypeConfig>,
}

#[derive(Serialize)]
struct AffixesWrapperSer<'a> {
    affixes: Vec<&'a AffixConfig>,
}

#[derive(Serialize)]
struct AffixPoolsWrapperSer<'a> {
    pools: Vec<&'a AffixPoolConfig>,
}

#[derive(Serialize)]
struct CurrenciesWrapperSer<'a> {
    currencies: Vec<&'a CurrencyConfig>,
}

#[derive(Serialize)]
struct UniqueFileSer<'a> {
    unique: &'a UniqueConfig,
    #[serde(skip_serializing_if = "Option::is_none")]
    recipe: Option<RecipeSerWrapper<'a>>,
}

#[derive(Serialize)]
struct RecipeSerWrapper<'a> {
    #[serde(skip_serializing_if = "is_default_weight")]
    weight: u32,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    required_affixes: &'a Vec<loot_core::config::RecipeAffixRequirement>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    mappings: &'a Vec<loot_core::config::RecipeMapping>,
}

fn is_default_weight(weight: &u32) -> bool {
    *weight == 100
}

/// Save a single entry to its origin file
pub fn save_entry(
    config: &Config,
    origins: &ConfigOrigins,
    tab: ConfigTab,
    id: &str,
    path: &Path,
) -> io::Result<()> {
    // Ensure parent directory exists
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    match tab {
        ConfigTab::BaseTypes => save_base_types(config, origins, path),
        ConfigTab::Affixes => save_affixes(config, origins, path),
        ConfigTab::AffixPools => save_affix_pools(config, origins, path),
        ConfigTab::Currencies => save_currencies(config, origins, path),
        ConfigTab::Uniques => save_unique(config, id, path),
    }
}

fn save_base_types(config: &Config, origins: &ConfigOrigins, path: &Path) -> io::Result<()> {
    // Collect all base types that belong to this file
    let base_types: Vec<&BaseTypeConfig> = config
        .base_types
        .values()
        .filter(|bt| origins.base_types.get(&bt.id).map_or(false, |p| p == path))
        .collect();

    let wrapper = BaseTypesWrapperSer { base_types };
    let content = toml::to_string_pretty(&wrapper)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

    fs::write(path, content)
}

fn save_affixes(config: &Config, origins: &ConfigOrigins, path: &Path) -> io::Result<()> {
    let affixes: Vec<&AffixConfig> = config
        .affixes
        .values()
        .filter(|a| origins.affixes.get(&a.id).map_or(false, |p| p == path))
        .collect();

    let wrapper = AffixesWrapperSer { affixes };
    let content = toml::to_string_pretty(&wrapper)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

    fs::write(path, content)
}

fn save_affix_pools(config: &Config, origins: &ConfigOrigins, path: &Path) -> io::Result<()> {
    let pools: Vec<&AffixPoolConfig> = config
        .affix_pools
        .values()
        .filter(|p| {
            origins
                .affix_pools
                .get(&p.id)
                .map_or(false, |op| op == path)
        })
        .collect();

    let wrapper = AffixPoolsWrapperSer { pools };
    let content = toml::to_string_pretty(&wrapper)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

    fs::write(path, content)
}

fn save_currencies(config: &Config, origins: &ConfigOrigins, path: &Path) -> io::Result<()> {
    let currencies: Vec<&CurrencyConfig> = config
        .currencies
        .values()
        .filter(|c| origins.currencies.get(&c.id).map_or(false, |p| p == path))
        .collect();

    let wrapper = CurrenciesWrapperSer { currencies };
    let content = toml::to_string_pretty(&wrapper)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

    fs::write(path, content)
}

fn save_unique(config: &Config, id: &str, path: &Path) -> io::Result<()> {
    let unique = config.uniques.get(id).ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::NotFound,
            format!("Unique '{}' not found", id),
        )
    })?;

    // Find the recipe for this unique from config.unique_recipes
    let recipe = config.unique_recipes.iter().find(|r| r.unique_id == id);

    // Build the output
    let recipe_wrapper = recipe.map(|r| RecipeSerWrapper {
        weight: r.weight,
        required_affixes: &r.required_affixes,
        mappings: &r.mappings,
    });

    let unique_ser = UniqueFileSer {
        unique,
        recipe: recipe_wrapper,
    };
    let content = toml::to_string_pretty(&unique_ser)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

    fs::write(path, content)
}
