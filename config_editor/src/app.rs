use crate::input::TextInputState;
use crate::persistence::{save_entry, ConfigOrigins};
use crate::views;
use loot_core::config::{
    AffixConfig, AffixPoolConfig, AffixTierConfig, BaseTypeConfig, Config, CurrencyConfig,
    MappingMode, RecipeAffixRequirement, RecipeMapping, UniqueConfig, UniqueRecipeConfig,
};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Tabs},
    Frame,
};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigTab {
    BaseTypes,
    Affixes,
    AffixPools,
    Currencies,
    Uniques,
}

impl ConfigTab {
    pub fn as_str(&self) -> &'static str {
        match self {
            ConfigTab::BaseTypes => "Base Types",
            ConfigTab::Affixes => "Affixes",
            ConfigTab::AffixPools => "Affix Pools",
            ConfigTab::Currencies => "Currencies",
            ConfigTab::Uniques => "Uniques",
        }
    }

    pub fn index(&self) -> usize {
        match self {
            ConfigTab::BaseTypes => 0,
            ConfigTab::Affixes => 1,
            ConfigTab::AffixPools => 2,
            ConfigTab::Currencies => 3,
            ConfigTab::Uniques => 4,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Browse,
    Edit,
    Create,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Focus {
    List,
    Detail,
    Field(usize),
}

/// Tracks unsaved changes
#[derive(Debug, Default)]
pub struct DirtyTracker {
    base_types: HashMap<String, bool>,
    affixes: HashMap<String, bool>,
    affix_pools: HashMap<String, bool>,
    currencies: HashMap<String, bool>,
    uniques: HashMap<String, bool>,
}

impl DirtyTracker {
    pub fn mark_dirty(&mut self, tab: ConfigTab, id: &str) {
        match tab {
            ConfigTab::BaseTypes => {
                self.base_types.insert(id.to_string(), true);
            }
            ConfigTab::Affixes => {
                self.affixes.insert(id.to_string(), true);
            }
            ConfigTab::AffixPools => {
                self.affix_pools.insert(id.to_string(), true);
            }
            ConfigTab::Currencies => {
                self.currencies.insert(id.to_string(), true);
            }
            ConfigTab::Uniques => {
                self.uniques.insert(id.to_string(), true);
            }
        }
    }

    pub fn mark_clean(&mut self, tab: ConfigTab, id: &str) {
        match tab {
            ConfigTab::BaseTypes => {
                self.base_types.remove(id);
            }
            ConfigTab::Affixes => {
                self.affixes.remove(id);
            }
            ConfigTab::AffixPools => {
                self.affix_pools.remove(id);
            }
            ConfigTab::Currencies => {
                self.currencies.remove(id);
            }
            ConfigTab::Uniques => {
                self.uniques.remove(id);
            }
        }
    }

    pub fn is_entry_dirty(&self, tab: ConfigTab, id: &str) -> bool {
        match tab {
            ConfigTab::BaseTypes => self.base_types.get(id).copied().unwrap_or(false),
            ConfigTab::Affixes => self.affixes.get(id).copied().unwrap_or(false),
            ConfigTab::AffixPools => self.affix_pools.get(id).copied().unwrap_or(false),
            ConfigTab::Currencies => self.currencies.get(id).copied().unwrap_or(false),
            ConfigTab::Uniques => self.uniques.get(id).copied().unwrap_or(false),
        }
    }

    pub fn is_dirty(&self) -> bool {
        !self.base_types.is_empty()
            || !self.affixes.is_empty()
            || !self.affix_pools.is_empty()
            || !self.currencies.is_empty()
            || !self.uniques.is_empty()
    }
}

/// State for a view tab
#[derive(Debug)]
pub struct ViewState {
    pub list_state: ListState,
    pub ids: Vec<String>,
    pub field_index: usize,
    pub nested_depth: usize,
    pub nested_index: usize,
}

impl Default for ViewState {
    fn default() -> Self {
        let mut list_state = ListState::default();
        list_state.select(Some(0));
        Self {
            list_state,
            ids: Vec::new(),
            field_index: 0,
            nested_depth: 0,
            nested_index: 0,
        }
    }
}

impl ViewState {
    pub fn selected_id(&self) -> Option<&str> {
        self.list_state
            .selected()
            .and_then(|i| self.ids.get(i).map(|s| s.as_str()))
    }
}

/// Editing state for a specific entry type
#[derive(Debug, Clone)]
pub enum EditingEntry {
    BaseType(BaseTypeConfig),
    Affix(AffixConfig),
    AffixPool(AffixPoolConfig),
    Currency(CurrencyConfig),
    Unique(UniqueConfig),
}

pub struct App {
    pub config_dir: PathBuf,
    pub config: Config,
    pub origins: ConfigOrigins,

    pub current_tab: ConfigTab,
    pub mode: Mode,
    pub focus: Focus,

    pub base_types_state: ViewState,
    pub affixes_state: ViewState,
    pub affix_pools_state: ViewState,
    pub currencies_state: ViewState,
    pub uniques_state: ViewState,

    pub editing: Option<EditingEntry>,
    pub text_input: TextInputState,
    pub dirty: DirtyTracker,
    pub message: Option<String>,

    // Enum picker state
    pub enum_picker_index: usize,
    pub enum_picker_options: Vec<String>,

    // Nested editor state
    pub nested_sub_field_index: usize,

    // Recipe editing for uniques
    pub editing_recipe: Option<UniqueRecipeConfig>,

    // Popups
    pub show_quit_confirm: bool,
    pub show_delete_confirm: bool,
    pub show_file_picker: bool,
    pub show_new_file_input: bool,
    pub file_picker_state: ListState,
    pub file_picker_files: Vec<PathBuf>,
    pub new_file_name: String,

    // Config load error
    pub config_error: Option<String>,
}

impl App {
    pub fn new(config_dir: &Path) -> Self {
        let config_dir = config_dir.to_path_buf();
        let (config, origins, error) = match Config::load_from_dir(&config_dir) {
            Ok(config) => {
                let origins = ConfigOrigins::load_from_dir(&config_dir);
                (config, origins, None)
            }
            Err(e) => (
                Config::default(),
                ConfigOrigins::default(),
                Some(e.to_string()),
            ),
        };

        let mut app = Self {
            config_dir,
            config,
            origins,
            current_tab: ConfigTab::BaseTypes,
            mode: Mode::Browse,
            focus: Focus::List,
            base_types_state: ViewState::default(),
            affixes_state: ViewState::default(),
            affix_pools_state: ViewState::default(),
            currencies_state: ViewState::default(),
            uniques_state: ViewState::default(),
            editing: None,
            editing_recipe: None,
            enum_picker_index: 0,
            enum_picker_options: Vec::new(),
            nested_sub_field_index: 0,
            text_input: TextInputState::default(),
            dirty: DirtyTracker::default(),
            message: None,
            show_quit_confirm: false,
            show_delete_confirm: false,
            show_file_picker: false,
            show_new_file_input: false,
            file_picker_state: ListState::default(),
            file_picker_files: Vec::new(),
            new_file_name: String::new(),
            config_error: error,
        };

        app.refresh_ids();
        app
    }

    fn refresh_ids(&mut self) {
        let mut base_type_ids: Vec<String> = self.config.base_types.keys().cloned().collect();
        base_type_ids.sort();
        self.base_types_state.ids = base_type_ids;

        let mut affix_ids: Vec<String> = self.config.affixes.keys().cloned().collect();
        affix_ids.sort();
        self.affixes_state.ids = affix_ids;

        let mut pool_ids: Vec<String> = self.config.affix_pools.keys().cloned().collect();
        pool_ids.sort();
        self.affix_pools_state.ids = pool_ids;

        let mut currency_ids: Vec<String> = self.config.currencies.keys().cloned().collect();
        currency_ids.sort();
        self.currencies_state.ids = currency_ids;

        let mut unique_ids: Vec<String> = self.config.uniques.keys().cloned().collect();
        unique_ids.sort();
        self.uniques_state.ids = unique_ids;

        // Ensure selections are valid
        Self::clamp_selection_static(&mut self.base_types_state);
        Self::clamp_selection_static(&mut self.affixes_state);
        Self::clamp_selection_static(&mut self.affix_pools_state);
        Self::clamp_selection_static(&mut self.currencies_state);
        Self::clamp_selection_static(&mut self.uniques_state);
    }

    fn clamp_selection_static(state: &mut ViewState) {
        if state.ids.is_empty() {
            state.list_state.select(None);
        } else if let Some(idx) = state.list_state.selected() {
            if idx >= state.ids.len() {
                state.list_state.select(Some(state.ids.len() - 1));
            }
        } else {
            state.list_state.select(Some(0));
        }
    }

    pub fn current_view_state(&self) -> &ViewState {
        match self.current_tab {
            ConfigTab::BaseTypes => &self.base_types_state,
            ConfigTab::Affixes => &self.affixes_state,
            ConfigTab::AffixPools => &self.affix_pools_state,
            ConfigTab::Currencies => &self.currencies_state,
            ConfigTab::Uniques => &self.uniques_state,
        }
    }

    pub fn current_view_state_mut(&mut self) -> &mut ViewState {
        match self.current_tab {
            ConfigTab::BaseTypes => &mut self.base_types_state,
            ConfigTab::Affixes => &mut self.affixes_state,
            ConfigTab::AffixPools => &mut self.affix_pools_state,
            ConfigTab::Currencies => &mut self.currencies_state,
            ConfigTab::Uniques => &mut self.uniques_state,
        }
    }

    pub fn clear_message(&mut self) {
        self.message = None;
    }

    pub fn switch_tab(&mut self, tab: ConfigTab) {
        if self.mode == Mode::Browse {
            self.current_tab = tab;
        }
    }

    pub fn list_up(&mut self) {
        let state = self.current_view_state_mut();
        if let Some(selected) = state.list_state.selected() {
            if selected > 0 {
                state.list_state.select(Some(selected - 1));
            }
        }
    }

    pub fn list_down(&mut self) {
        let state = self.current_view_state_mut();
        let len = state.ids.len();
        if let Some(selected) = state.list_state.selected() {
            if selected < len.saturating_sub(1) {
                state.list_state.select(Some(selected + 1));
            }
        } else if len > 0 {
            state.list_state.select(Some(0));
        }
    }

    pub fn enter_edit_mode(&mut self) {
        let state = self.current_view_state();
        let Some(id) = state.selected_id().map(|s| s.to_string()) else {
            return;
        };

        self.editing = match self.current_tab {
            ConfigTab::BaseTypes => self
                .config
                .base_types
                .get(&id)
                .cloned()
                .map(EditingEntry::BaseType),
            ConfigTab::Affixes => self
                .config
                .affixes
                .get(&id)
                .cloned()
                .map(EditingEntry::Affix),
            ConfigTab::AffixPools => self
                .config
                .affix_pools
                .get(&id)
                .cloned()
                .map(EditingEntry::AffixPool),
            ConfigTab::Currencies => self
                .config
                .currencies
                .get(&id)
                .cloned()
                .map(EditingEntry::Currency),
            ConfigTab::Uniques => {
                // Also load associated recipe if it exists
                self.editing_recipe = self
                    .config
                    .unique_recipes
                    .iter()
                    .find(|r| r.unique_id == id)
                    .cloned();
                self.config
                    .uniques
                    .get(&id)
                    .cloned()
                    .map(EditingEntry::Unique)
            }
        };

        if self.editing.is_some() {
            self.mode = Mode::Edit;
            self.focus = Focus::Field(0);
            let state = self.current_view_state_mut();
            state.field_index = 0;
            state.nested_depth = 0;
            state.nested_index = 0;
            self.nested_sub_field_index = 0;
            self.update_text_input_for_field();
        }
    }

    pub fn enter_create_mode(&mut self) {
        self.editing = Some(match self.current_tab {
            ConfigTab::BaseTypes => EditingEntry::BaseType(BaseTypeConfig {
                id: String::new(),
                name: String::new(),
                class: loot_core::types::ItemClass::OneHandSword,
                tags: Vec::new(),
                implicit: None,
                defenses: None,
                damage: None,
                requirements: Default::default(),
            }),
            ConfigTab::Affixes => EditingEntry::Affix(AffixConfig {
                id: String::new(),
                name: String::new(),
                affix_type: loot_core::types::AffixType::Prefix,
                stat: loot_core::types::StatType::AddedPhysicalDamage,
                tags: Vec::new(),
                allowed_classes: Vec::new(),
                tiers: vec![AffixTierConfig {
                    tier: 1,
                    weight: 100,
                    min: 1,
                    max: 10,
                    min_ilvl: 1,
                }],
            }),
            ConfigTab::AffixPools => EditingEntry::AffixPool(AffixPoolConfig {
                id: String::new(),
                name: String::new(),
                description: String::new(),
                affixes: Vec::new(),
            }),
            ConfigTab::Currencies => EditingEntry::Currency(CurrencyConfig {
                id: String::new(),
                name: String::new(),
                description: String::new(),
                category: String::new(),
                requires: Default::default(),
                effects: Default::default(),
            }),
            ConfigTab::Uniques => {
                // Initialize empty recipe for new uniques
                self.editing_recipe = None;
                EditingEntry::Unique(UniqueConfig {
                    id: String::new(),
                    name: String::new(),
                    base_type: String::new(),
                    flavor: None,
                    mods: Vec::new(),
                })
            }
        });

        self.mode = Mode::Create;
        self.focus = Focus::Field(0);
        let state = self.current_view_state_mut();
        state.field_index = 0;
        state.nested_depth = 0;
        state.nested_index = 0;
        self.nested_sub_field_index = 0;
        self.text_input = TextInputState::new(String::new());
    }

    pub fn cancel_edit(&mut self) {
        self.editing = None;
        self.editing_recipe = None;
        self.mode = Mode::Browse;
        self.focus = Focus::List;
    }

    pub fn save_and_exit_edit(&mut self) {
        if let Some(ref editing) = self.editing {
            let id = match editing {
                EditingEntry::BaseType(e) => e.id.clone(),
                EditingEntry::Affix(e) => e.id.clone(),
                EditingEntry::AffixPool(e) => e.id.clone(),
                EditingEntry::Currency(e) => e.id.clone(),
                EditingEntry::Unique(e) => e.id.clone(),
            };

            if id.is_empty() {
                self.message = Some("ID cannot be empty".to_string());
                return;
            }

            // For create mode, check if ID already exists
            if self.mode == Mode::Create {
                let exists = match self.current_tab {
                    ConfigTab::BaseTypes => self.config.base_types.contains_key(&id),
                    ConfigTab::Affixes => self.config.affixes.contains_key(&id),
                    ConfigTab::AffixPools => self.config.affix_pools.contains_key(&id),
                    ConfigTab::Currencies => self.config.currencies.contains_key(&id),
                    ConfigTab::Uniques => self.config.uniques.contains_key(&id),
                };
                if exists {
                    self.message = Some(format!("ID '{}' already exists", id));
                    return;
                }
            }

            // Check if we have an origin file, or need to pick one
            let has_origin = self.origins.get_origin(self.current_tab, &id).is_some();

            if !has_origin && self.mode == Mode::Create {
                // Need to pick a file for new entry
                self.show_file_picker_for_current_tab();
                return;
            }

            self.apply_edit_to_config();
            self.dirty.mark_dirty(self.current_tab, &id);
            self.message = Some(format!("Updated {}", id));
        }

        self.editing = None;
        self.mode = Mode::Browse;
        self.focus = Focus::List;
        self.refresh_ids();
    }

    fn apply_edit_to_config(&mut self) {
        let Some(editing) = self.editing.take() else {
            return;
        };

        match editing {
            EditingEntry::BaseType(e) => {
                self.config.base_types.insert(e.id.clone(), e);
            }
            EditingEntry::Affix(e) => {
                self.config.affixes.insert(e.id.clone(), e);
            }
            EditingEntry::AffixPool(e) => {
                self.config.affix_pools.insert(e.id.clone(), e);
            }
            EditingEntry::Currency(e) => {
                self.config.currencies.insert(e.id.clone(), e);
            }
            EditingEntry::Unique(e) => {
                let unique_id = e.id.clone();
                let base_type = e.base_type.clone();
                self.config.uniques.insert(unique_id.clone(), e);

                // Handle recipe: remove old, add new if exists
                self.config
                    .unique_recipes
                    .retain(|r| r.unique_id != unique_id);
                if let Some(mut recipe) = self.editing_recipe.take() {
                    recipe.unique_id = unique_id;
                    recipe.base_type = base_type;
                    self.config.unique_recipes.push(recipe);
                }
            }
        }
    }

    pub fn request_delete(&mut self) {
        if self.current_view_state().selected_id().is_some() {
            self.show_delete_confirm = true;
        }
    }

    pub fn confirm_delete(&mut self) {
        let state = self.current_view_state();
        let Some(id) = state.selected_id().map(|s| s.to_string()) else {
            return;
        };

        match self.current_tab {
            ConfigTab::BaseTypes => {
                self.config.base_types.remove(&id);
            }
            ConfigTab::Affixes => {
                self.config.affixes.remove(&id);
            }
            ConfigTab::AffixPools => {
                self.config.affix_pools.remove(&id);
            }
            ConfigTab::Currencies => {
                self.config.currencies.remove(&id);
            }
            ConfigTab::Uniques => {
                self.config.uniques.remove(&id);
            }
        }

        self.dirty.mark_dirty(self.current_tab, &id);
        self.message = Some(format!("Deleted {}", id));
        self.refresh_ids();
    }

    pub fn save_current(&mut self) {
        let state = self.current_view_state();
        let Some(id) = state.selected_id().map(|s| s.to_string()) else {
            self.message = Some("No entry selected".to_string());
            return;
        };

        let Some(path) = self.origins.get_origin(self.current_tab, &id).cloned() else {
            self.message = Some("No origin file for this entry".to_string());
            return;
        };

        match save_entry(&self.config, &self.origins, self.current_tab, &id, &path) {
            Ok(()) => {
                self.dirty.mark_clean(self.current_tab, &id);
                self.message = Some(format!("Saved {} to {}", id, path.display()));
            }
            Err(e) => {
                self.message = Some(format!("Save failed: {}", e));
            }
        }
    }

    fn show_file_picker_for_current_tab(&mut self) {
        let subdir = match self.current_tab {
            ConfigTab::BaseTypes => "base_types",
            ConfigTab::Affixes => "affixes",
            ConfigTab::AffixPools => "affix_pools",
            ConfigTab::Currencies => "currencies",
            ConfigTab::Uniques => "uniques",
        };

        let dir = self.config_dir.join(subdir);
        self.file_picker_files = if dir.exists() {
            std::fs::read_dir(&dir)
                .ok()
                .map(|entries| {
                    entries
                        .filter_map(|e| e.ok())
                        .map(|e| e.path())
                        .filter(|p| p.extension().map_or(false, |ext| ext == "toml"))
                        .collect()
                })
                .unwrap_or_default()
        } else {
            Vec::new()
        };

        self.file_picker_files.sort();
        self.file_picker_state.select(Some(0));
        self.show_file_picker = true;
    }

    pub fn file_picker_up(&mut self) {
        if let Some(selected) = self.file_picker_state.selected() {
            if selected > 0 {
                self.file_picker_state.select(Some(selected - 1));
            }
        }
    }

    pub fn file_picker_down(&mut self) {
        if let Some(selected) = self.file_picker_state.selected() {
            if selected < self.file_picker_files.len().saturating_sub(1) {
                self.file_picker_state.select(Some(selected + 1));
            }
        }
    }

    pub fn file_picker_select(&mut self) {
        if let Some(idx) = self.file_picker_state.selected() {
            if let Some(path) = self.file_picker_files.get(idx).cloned() {
                self.complete_save_with_file(path);
            }
        }
    }

    pub fn file_picker_new(&mut self) {
        self.new_file_name.clear();
        self.show_new_file_input = true;
    }

    pub fn confirm_new_file(&mut self) {
        if self.new_file_name.is_empty() {
            return;
        }

        let subdir = match self.current_tab {
            ConfigTab::BaseTypes => "base_types",
            ConfigTab::Affixes => "affixes",
            ConfigTab::AffixPools => "affix_pools",
            ConfigTab::Currencies => "currencies",
            ConfigTab::Uniques => "uniques",
        };

        let mut filename = self.new_file_name.clone();
        if !filename.ends_with(".toml") {
            filename.push_str(".toml");
        }

        let path = self.config_dir.join(subdir).join(filename);
        self.show_new_file_input = false;
        self.show_file_picker = false;
        self.complete_save_with_file(path);
    }

    fn complete_save_with_file(&mut self, path: PathBuf) {
        let Some(ref editing) = self.editing else {
            return;
        };

        let id = match editing {
            EditingEntry::BaseType(e) => e.id.clone(),
            EditingEntry::Affix(e) => e.id.clone(),
            EditingEntry::AffixPool(e) => e.id.clone(),
            EditingEntry::Currency(e) => e.id.clone(),
            EditingEntry::Unique(e) => e.id.clone(),
        };

        // Set the origin
        self.origins.set_origin(self.current_tab, &id, path.clone());

        // Apply to config
        self.apply_edit_to_config();

        // Save to file
        match save_entry(&self.config, &self.origins, self.current_tab, &id, &path) {
            Ok(()) => {
                self.message = Some(format!("Saved {} to {}", id, path.display()));
            }
            Err(e) => {
                self.message = Some(format!("Save failed: {}", e));
            }
        }

        self.show_file_picker = false;
        self.editing = None;
        self.mode = Mode::Browse;
        self.focus = Focus::List;
        self.refresh_ids();
    }

    // Field navigation
    pub fn next_field(&mut self) {
        let max_fields = self.get_field_count();
        let state = self.current_view_state_mut();
        if state.field_index < max_fields.saturating_sub(1) {
            state.field_index += 1;
            let new_idx = state.field_index;
            self.focus = Focus::Field(new_idx);
            self.update_text_input_for_field();
        }
    }

    pub fn prev_field(&mut self) {
        let state = self.current_view_state_mut();
        if state.field_index > 0 {
            state.field_index -= 1;
            let new_idx = state.field_index;
            self.focus = Focus::Field(new_idx);
            self.update_text_input_for_field();
        }
    }

    fn get_field_count(&self) -> usize {
        match &self.editing {
            Some(EditingEntry::BaseType(_)) => 8, // id, name, class, tags, implicit, defenses, damage, requirements
            Some(EditingEntry::Affix(_)) => 7, // id, name, type, stat, tags, allowed_classes, tiers
            Some(EditingEntry::AffixPool(_)) => 4, // id, name, description, affixes
            Some(EditingEntry::Currency(_)) => 6, // id, name, description, category, requires, effects
            Some(EditingEntry::Unique(_)) => 6,   // id, name, base_type, flavor, mods, recipe
            None => 0,
        }
    }

    fn update_text_input_for_field(&mut self) {
        let field_idx = self.current_view_state().field_index;
        let value = self.get_field_value(field_idx);
        self.text_input = TextInputState::new(value);
    }

    fn get_field_value(&self, field_idx: usize) -> String {
        match &self.editing {
            Some(EditingEntry::AffixPool(pool)) => match field_idx {
                0 => pool.id.clone(),
                1 => pool.name.clone(),
                2 => pool.description.clone(),
                3 => String::new(), // List field - start empty for adding
                _ => String::new(),
            },
            Some(EditingEntry::BaseType(bt)) => match field_idx {
                0 => bt.id.clone(),
                1 => bt.name.clone(),
                2 => format!("{:?}", bt.class),
                3 => String::new(), // List field - start empty for adding
                _ => String::new(),
            },
            Some(EditingEntry::Affix(affix)) => match field_idx {
                0 => affix.id.clone(),
                1 => affix.name.clone(),
                2 => format!("{:?}", affix.affix_type),
                3 => format!("{:?}", affix.stat),
                4 => String::new(), // List field - start empty for adding
                5 => String::new(), // List field - start empty for adding
                _ => String::new(),
            },
            Some(EditingEntry::Currency(curr)) => match field_idx {
                0 => curr.id.clone(),
                1 => curr.name.clone(),
                2 => curr.description.clone(),
                3 => curr.category.clone(),
                _ => String::new(),
            },
            Some(EditingEntry::Unique(uniq)) => match field_idx {
                0 => uniq.id.clone(),
                1 => uniq.name.clone(),
                2 => uniq.base_type.clone(),
                3 => uniq.flavor.clone().unwrap_or_default(),
                _ => String::new(),
            },
            None => String::new(),
        }
    }

    fn set_field_value(&mut self, field_idx: usize, value: String) {
        match &mut self.editing {
            Some(EditingEntry::AffixPool(pool)) => match field_idx {
                0 => pool.id = value,
                1 => pool.name = value,
                2 => pool.description = value,
                3 => {
                    pool.affixes = value
                        .split(',')
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty())
                        .collect()
                }
                _ => {}
            },
            Some(EditingEntry::BaseType(bt)) => match field_idx {
                0 => bt.id = value,
                1 => bt.name = value,
                3 => {
                    bt.tags = value
                        .split(',')
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty())
                        .collect()
                }
                _ => {}
            },
            Some(EditingEntry::Affix(affix)) => match field_idx {
                0 => affix.id = value,
                1 => affix.name = value,
                4 => {
                    affix.tags = value
                        .split(',')
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty())
                        .collect()
                }
                _ => {}
            },
            Some(EditingEntry::Currency(curr)) => match field_idx {
                0 => curr.id = value,
                1 => curr.name = value,
                2 => curr.description = value,
                3 => curr.category = value,
                _ => {}
            },
            Some(EditingEntry::Unique(uniq)) => match field_idx {
                0 => uniq.id = value,
                1 => uniq.name = value,
                2 => uniq.base_type = value,
                3 => uniq.flavor = if value.is_empty() { None } else { Some(value) },
                _ => {}
            },
            None => {}
        }
    }

    // Text input handlers
    pub fn text_input_char(&mut self, c: char) {
        self.text_input.insert(c);
        let nested_depth = self.current_view_state().nested_depth;
        if nested_depth >= 2 {
            // In nested sub-field editing mode - apply immediately
            self.apply_nested_text_input();
        } else if !self.is_list_field() {
            // Don't update field value for list fields - they use list_field_add
            let value = self.text_input.value().to_string();
            let field_idx = self.current_view_state().field_index;
            self.set_field_value(field_idx, value);
        }
    }

    pub fn text_input_backspace(&mut self) {
        self.text_input.backspace();
        let nested_depth = self.current_view_state().nested_depth;
        if nested_depth >= 2 {
            self.apply_nested_text_input();
        } else if !self.is_list_field() {
            let value = self.text_input.value().to_string();
            let field_idx = self.current_view_state().field_index;
            self.set_field_value(field_idx, value);
        }
    }

    pub fn text_input_delete(&mut self) {
        self.text_input.delete();
        let nested_depth = self.current_view_state().nested_depth;
        if nested_depth >= 2 {
            self.apply_nested_text_input();
        } else if !self.is_list_field() {
            let value = self.text_input.value().to_string();
            let field_idx = self.current_view_state().field_index;
            self.set_field_value(field_idx, value);
        }
    }

    pub fn text_input_left(&mut self) {
        self.text_input.move_left();
    }

    pub fn text_input_right(&mut self) {
        self.text_input.move_right();
    }

    pub fn text_input_home(&mut self) {
        self.text_input.move_home();
    }

    pub fn text_input_end(&mut self) {
        self.text_input.move_end();
    }

    // Field type checks
    pub fn is_enum_field(&self) -> bool {
        let field_idx = self.current_view_state().field_index;
        match &self.editing {
            // Note: BaseType class uses text input, not enum picker
            Some(EditingEntry::Affix(_)) => field_idx == 2 || field_idx == 3, // type, stat
            _ => false,
        }
    }

    pub fn is_list_field(&self) -> bool {
        let field_idx = self.current_view_state().field_index;
        match &self.editing {
            Some(EditingEntry::AffixPool(_)) => field_idx == 3, // affixes
            Some(EditingEntry::BaseType(_)) => field_idx == 2 || field_idx == 3, // class (single), tags (list)
            Some(EditingEntry::Affix(_)) => field_idx == 4 || field_idx == 5, // tags, allowed_classes
            _ => false,
        }
    }

    pub fn is_nested_field(&self) -> bool {
        let field_idx = self.current_view_state().field_index;
        match &self.editing {
            Some(EditingEntry::BaseType(_)) => field_idx >= 4, // implicit, defenses, damage, requirements
            Some(EditingEntry::Affix(_)) => field_idx == 6,    // tiers
            Some(EditingEntry::Currency(_)) => field_idx >= 4, // requires, effects
            Some(EditingEntry::Unique(_)) => field_idx >= 4,   // mods, recipe
            _ => false,
        }
    }

    pub fn in_nested_editor(&self) -> bool {
        self.current_view_state().nested_depth > 0
    }

    pub fn enum_picker_up(&mut self) {
        if self.enum_picker_index > 0 {
            self.enum_picker_index -= 1;
        }
    }

    pub fn enum_picker_down(&mut self) {
        if self.enum_picker_index < self.enum_picker_options.len().saturating_sub(1) {
            self.enum_picker_index += 1;
        }
    }

    pub fn enum_picker_select(&mut self) {
        // Use text input value for enum selection
        let value = self.text_input.value().trim().to_string();
        if value.is_empty() {
            return;
        }
        let field_idx = self.current_view_state().field_index;
        self.apply_enum_selection(field_idx, &value);
    }

    fn apply_enum_selection(&mut self, field_idx: usize, value: &str) {
        let result = match &mut self.editing {
            Some(EditingEntry::BaseType(bt)) if field_idx == 2 => {
                // Parse ItemClass
                match Self::parse_item_class(value) {
                    Ok(class) => {
                        bt.class = class;
                        Ok(format!("Class set to {:?}", class))
                    }
                    Err(_) => Err(format!("Unknown class: {}", value)),
                }
            }
            Some(EditingEntry::Affix(affix)) if field_idx == 2 => {
                // Parse AffixType
                match Self::parse_affix_type(value) {
                    Ok(affix_type) => {
                        affix.affix_type = affix_type;
                        Ok(format!("Type set to {:?}", affix_type))
                    }
                    Err(_) => Err(format!("Unknown type: {} (use Prefix or Suffix)", value)),
                }
            }
            Some(EditingEntry::Affix(affix)) if field_idx == 3 => {
                // Parse StatType
                match Self::parse_stat_type(value) {
                    Ok(stat) => {
                        affix.stat = stat;
                        Ok(format!("Stat set to {:?}", stat))
                    }
                    Err(_) => Err(format!("Unknown stat: {}", value)),
                }
            }
            _ => Ok(String::new()),
        };

        match result {
            Ok(msg) => {
                if !msg.is_empty() {
                    self.message = Some(msg);
                }
                self.text_input = TextInputState::new(String::new());
            }
            Err(msg) => {
                self.message = Some(msg);
            }
        }
    }

    fn parse_item_class(s: &str) -> Result<loot_core::types::ItemClass, ()> {
        use loot_core::types::ItemClass;
        match s {
            "OneHandSword" => Ok(ItemClass::OneHandSword),
            "OneHandAxe" => Ok(ItemClass::OneHandAxe),
            "OneHandMace" => Ok(ItemClass::OneHandMace),
            "Dagger" => Ok(ItemClass::Dagger),
            "Claw" => Ok(ItemClass::Claw),
            "Wand" => Ok(ItemClass::Wand),
            "TwoHandSword" => Ok(ItemClass::TwoHandSword),
            "TwoHandAxe" => Ok(ItemClass::TwoHandAxe),
            "TwoHandMace" => Ok(ItemClass::TwoHandMace),
            "Bow" => Ok(ItemClass::Bow),
            "Staff" => Ok(ItemClass::Staff),
            "Shield" => Ok(ItemClass::Shield),
            "Helmet" => Ok(ItemClass::Helmet),
            "BodyArmour" => Ok(ItemClass::BodyArmour),
            "Gloves" => Ok(ItemClass::Gloves),
            "Boots" => Ok(ItemClass::Boots),
            _ => Err(()),
        }
    }

    fn parse_affix_type(s: &str) -> Result<loot_core::types::AffixType, ()> {
        use loot_core::types::AffixType;
        match s {
            "Prefix" => Ok(AffixType::Prefix),
            "Suffix" => Ok(AffixType::Suffix),
            _ => Err(()),
        }
    }

    fn parse_rarity(s: &str) -> Result<loot_core::types::Rarity, ()> {
        use loot_core::types::Rarity;
        match s {
            "Normal" => Ok(Rarity::Normal),
            "Magic" => Ok(Rarity::Magic),
            "Rare" => Ok(Rarity::Rare),
            "Unique" => Ok(Rarity::Unique),
            _ => Err(()),
        }
    }

    pub fn get_all_rarities() -> Vec<&'static str> {
        vec!["Normal", "Magic", "Rare", "Unique"]
    }

    pub fn get_all_affix_pool_ids(&self) -> Vec<String> {
        let mut ids: Vec<String> = self.config.affix_pools.keys().cloned().collect();
        ids.sort();
        ids
    }

    fn parse_stat_type(s: &str) -> Result<loot_core::types::StatType, ()> {
        use loot_core::types::StatType;
        match s {
            "AddedPhysicalDamage" => Ok(StatType::AddedPhysicalDamage),
            "AddedFireDamage" => Ok(StatType::AddedFireDamage),
            "AddedColdDamage" => Ok(StatType::AddedColdDamage),
            "AddedLightningDamage" => Ok(StatType::AddedLightningDamage),
            "AddedChaosDamage" => Ok(StatType::AddedChaosDamage),
            "IncreasedPhysicalDamage" => Ok(StatType::IncreasedPhysicalDamage),
            "IncreasedElementalDamage" => Ok(StatType::IncreasedElementalDamage),
            "IncreasedChaosDamage" => Ok(StatType::IncreasedChaosDamage),
            "IncreasedAttackSpeed" => Ok(StatType::IncreasedAttackSpeed),
            "IncreasedCriticalChance" => Ok(StatType::IncreasedCriticalChance),
            "IncreasedCriticalDamage" => Ok(StatType::IncreasedCriticalDamage),
            "PoisonDamageOverTime" => Ok(StatType::PoisonDamageOverTime),
            "ChanceToPoison" => Ok(StatType::ChanceToPoison),
            "IncreasedPoisonDuration" => Ok(StatType::IncreasedPoisonDuration),
            "AddedArmour" => Ok(StatType::AddedArmour),
            "AddedEvasion" => Ok(StatType::AddedEvasion),
            "AddedEnergyShield" => Ok(StatType::AddedEnergyShield),
            "IncreasedArmour" => Ok(StatType::IncreasedArmour),
            "IncreasedEvasion" => Ok(StatType::IncreasedEvasion),
            "IncreasedEnergyShield" => Ok(StatType::IncreasedEnergyShield),
            "AddedStrength" => Ok(StatType::AddedStrength),
            "AddedDexterity" => Ok(StatType::AddedDexterity),
            "AddedConstitution" => Ok(StatType::AddedConstitution),
            "AddedIntelligence" => Ok(StatType::AddedIntelligence),
            "AddedWisdom" => Ok(StatType::AddedWisdom),
            "AddedCharisma" => Ok(StatType::AddedCharisma),
            "AddedAllAttributes" => Ok(StatType::AddedAllAttributes),
            "AddedLife" => Ok(StatType::AddedLife),
            "AddedMana" => Ok(StatType::AddedMana),
            "IncreasedLife" => Ok(StatType::IncreasedLife),
            "IncreasedMana" => Ok(StatType::IncreasedMana),
            "LifeRegeneration" => Ok(StatType::LifeRegeneration),
            "ManaRegeneration" => Ok(StatType::ManaRegeneration),
            "LifeOnHit" => Ok(StatType::LifeOnHit),
            "LifeLeech" => Ok(StatType::LifeLeech),
            "ManaLeech" => Ok(StatType::ManaLeech),
            "FireResistance" => Ok(StatType::FireResistance),
            "ColdResistance" => Ok(StatType::ColdResistance),
            "LightningResistance" => Ok(StatType::LightningResistance),
            "ChaosResistance" => Ok(StatType::ChaosResistance),
            "AllResistances" => Ok(StatType::AllResistances),
            "AddedAccuracy" => Ok(StatType::AddedAccuracy),
            "IncreasedAccuracy" => Ok(StatType::IncreasedAccuracy),
            "IncreasedMovementSpeed" => Ok(StatType::IncreasedMovementSpeed),
            "IncreasedItemRarity" => Ok(StatType::IncreasedItemRarity),
            "IncreasedItemQuantity" => Ok(StatType::IncreasedItemQuantity),
            _ => Err(()),
        }
    }

    fn parse_damage_type(s: &str) -> Result<loot_core::types::DamageType, ()> {
        use loot_core::types::DamageType;
        match s.to_lowercase().as_str() {
            "physical" => Ok(DamageType::Physical),
            "fire" => Ok(DamageType::Fire),
            "cold" => Ok(DamageType::Cold),
            "lightning" => Ok(DamageType::Lightning),
            "chaos" => Ok(DamageType::Chaos),
            _ => Err(()),
        }
    }

    /// Get all unique tags from the config (affixes and base_types)
    pub fn get_all_tags(&self) -> Vec<String> {
        let mut tags: std::collections::HashSet<String> = std::collections::HashSet::new();

        for affix in self.config.affixes.values() {
            for tag in &affix.tags {
                tags.insert(tag.clone());
            }
        }

        for bt in self.config.base_types.values() {
            for tag in &bt.tags {
                tags.insert(tag.clone());
            }
        }

        let mut result: Vec<String> = tags.into_iter().collect();
        result.sort();
        result
    }

    /// Get all affix IDs from config
    pub fn get_all_affix_ids(&self) -> Vec<String> {
        let mut ids: Vec<String> = self.config.affixes.keys().cloned().collect();
        ids.sort();
        ids
    }

    /// Check if an affix ID exists in the config
    pub fn is_valid_affix(&self, affix_id: &str) -> bool {
        self.config.affixes.contains_key(affix_id)
    }

    /// Get all ItemClass variants (from types.rs enum)
    pub fn get_all_classes() -> Vec<&'static str> {
        vec![
            // One-handed weapons
            "OneHandSword",
            "OneHandAxe",
            "OneHandMace",
            "Dagger",
            "Claw",
            "Wand",
            // Two-handed weapons
            "TwoHandSword",
            "TwoHandAxe",
            "TwoHandMace",
            "Bow",
            "Staff",
            // Off-hand
            "Shield",
            // Armour
            "Helmet",
            "BodyArmour",
            "Gloves",
            "Boots",
        ]
    }

    pub fn get_all_stat_types() -> Vec<&'static str> {
        vec![
            // Flat additions
            "AddedPhysicalDamage",
            "AddedFireDamage",
            "AddedColdDamage",
            "AddedLightningDamage",
            "AddedChaosDamage",
            // Percentage increases
            "IncreasedPhysicalDamage",
            "IncreasedElementalDamage",
            "IncreasedChaosDamage",
            "IncreasedAttackSpeed",
            "IncreasedCriticalChance",
            "IncreasedCriticalDamage",
            // Poison/Ailment
            "PoisonDamageOverTime",
            "ChanceToPoison",
            "IncreasedPoisonDuration",
            // Defenses
            "AddedArmour",
            "AddedEvasion",
            "AddedEnergyShield",
            "IncreasedArmour",
            "IncreasedEvasion",
            "IncreasedEnergyShield",
            // Attributes
            "AddedStrength",
            "AddedDexterity",
            "AddedConstitution",
            "AddedIntelligence",
            "AddedWisdom",
            "AddedCharisma",
            "AddedAllAttributes",
            // Life and resources
            "AddedLife",
            "AddedMana",
            "IncreasedLife",
            "IncreasedMana",
            "LifeRegeneration",
            "ManaRegeneration",
            "LifeOnHit",
            "LifeLeech",
            "ManaLeech",
            // Resistances
            "FireResistance",
            "ColdResistance",
            "LightningResistance",
            "ChaosResistance",
            "AllResistances",
            // Accuracy and utility
            "AddedAccuracy",
            "IncreasedAccuracy",
            "IncreasedMovementSpeed",
            "IncreasedItemRarity",
            "IncreasedItemQuantity",
        ]
    }

    fn is_valid_tag(&self, tag: &str) -> bool {
        let all_tags = self.get_all_tags();
        all_tags.iter().any(|t| t == tag)
    }

    pub fn get_enum_options(&self) -> Vec<String> {
        let field_idx = self.current_view_state().field_index;
        match &self.editing {
            Some(EditingEntry::BaseType(_)) if field_idx == 2 => vec![
                "OneHandSword",
                "OneHandAxe",
                "OneHandMace",
                "Dagger",
                "Claw",
                "Wand",
                "TwoHandSword",
                "TwoHandAxe",
                "TwoHandMace",
                "Bow",
                "Staff",
                "Shield",
                "Helmet",
                "BodyArmour",
                "Gloves",
                "Boots",
            ]
            .into_iter()
            .map(String::from)
            .collect(),
            Some(EditingEntry::Affix(_)) if field_idx == 2 => vec!["Prefix", "Suffix"]
                .into_iter()
                .map(String::from)
                .collect(),
            Some(EditingEntry::Affix(_)) if field_idx == 3 => vec![
                "AddedPhysicalDamage",
                "AddedFireDamage",
                "AddedColdDamage",
                "AddedLightningDamage",
                "AddedChaosDamage",
                "IncreasedPhysicalDamage",
                "IncreasedElementalDamage",
                "IncreasedChaosDamage",
                "IncreasedAttackSpeed",
                "IncreasedCriticalChance",
                "IncreasedCriticalDamage",
                "PoisonDamageOverTime",
                "ChanceToPoison",
                "IncreasedPoisonDuration",
                "AddedArmour",
                "AddedEvasion",
                "AddedEnergyShield",
                "IncreasedArmour",
                "IncreasedEvasion",
                "IncreasedEnergyShield",
                "AddedStrength",
                "AddedDexterity",
                "AddedConstitution",
                "AddedIntelligence",
                "AddedWisdom",
                "AddedCharisma",
                "AddedAllAttributes",
                "AddedLife",
                "AddedMana",
                "IncreasedLife",
                "IncreasedMana",
                "LifeRegeneration",
                "ManaRegeneration",
                "LifeOnHit",
                "LifeLeech",
                "ManaLeech",
                "FireResistance",
                "ColdResistance",
                "LightningResistance",
                "ChaosResistance",
                "AllResistances",
                "AddedAccuracy",
                "IncreasedAccuracy",
                "IncreasedMovementSpeed",
                "IncreasedItemRarity",
                "IncreasedItemQuantity",
            ]
            .into_iter()
            .map(String::from)
            .collect(),
            _ => Vec::new(),
        }
    }

    pub fn list_field_up(&mut self) {
        let state = self.current_view_state_mut();
        if state.nested_index > 0 {
            state.nested_index -= 1;
        }
    }

    pub fn list_field_down(&mut self) {
        let list_len = self.get_current_list_len();
        let state = self.current_view_state_mut();
        if state.nested_index < list_len.saturating_sub(1) {
            state.nested_index += 1;
        }
    }

    fn get_current_list_len(&self) -> usize {
        let field_idx = self.current_view_state().field_index;
        match &self.editing {
            Some(EditingEntry::AffixPool(pool)) if field_idx == 3 => pool.affixes.len(),
            Some(EditingEntry::BaseType(bt)) if field_idx == 3 => bt.tags.len(),
            Some(EditingEntry::Affix(affix)) if field_idx == 4 => affix.tags.len(),
            Some(EditingEntry::Affix(affix)) if field_idx == 5 => affix.allowed_classes.len(),
            _ => 0,
        }
    }

    pub fn list_field_add(&mut self) {
        let value = self.text_input.value().trim().to_string();
        if value.is_empty() {
            return;
        }
        let field_idx = self.current_view_state().field_index;

        // Pre-validate tags before mutable borrow
        let tag_valid = match &self.editing {
            Some(EditingEntry::Affix(_)) if field_idx == 4 => self.is_valid_tag(&value),
            Some(EditingEntry::BaseType(_)) if field_idx == 3 => self.is_valid_tag(&value),
            _ => true,
        };

        // Pre-validate affix ID for AffixPool
        let affix_valid = match &self.editing {
            Some(EditingEntry::AffixPool(_)) if field_idx == 3 => self.is_valid_affix(&value),
            _ => true,
        };

        // Pre-validate class for BaseType
        let class_result = match &self.editing {
            Some(EditingEntry::BaseType(_)) if field_idx == 2 => {
                Some(Self::parse_item_class(&value))
            }
            _ => None,
        };

        match &mut self.editing {
            Some(EditingEntry::AffixPool(pool)) if field_idx == 3 => {
                if affix_valid {
                    if !pool.affixes.contains(&value) {
                        pool.affixes.push(value);
                    }
                    self.text_input = TextInputState::new(String::new());
                } else {
                    self.message = Some(format!("Unknown affix: {}", value));
                }
            }
            Some(EditingEntry::BaseType(bt)) if field_idx == 2 => {
                // Set class
                if let Some(Ok(class)) = class_result {
                    bt.class = class;
                    self.text_input = TextInputState::new(String::new());
                    self.message = Some(format!("Class set to {:?}", bt.class));
                } else {
                    self.message = Some(format!("Unknown class: {}", value));
                }
            }
            Some(EditingEntry::BaseType(bt)) if field_idx == 3 => {
                // Validate tag
                if tag_valid {
                    if !bt.tags.contains(&value) {
                        bt.tags.push(value);
                    }
                    self.text_input = TextInputState::new(String::new());
                } else {
                    self.message = Some(format!("Unknown tag: {}", value));
                }
            }
            Some(EditingEntry::Affix(affix)) if field_idx == 4 => {
                // Validate tag (already checked above)
                if tag_valid {
                    if !affix.tags.contains(&value) {
                        affix.tags.push(value);
                    }
                    self.text_input = TextInputState::new(String::new());
                } else {
                    self.message = Some(format!("Unknown tag: {}", value));
                }
            }
            Some(EditingEntry::Affix(affix)) if field_idx == 5 => {
                // Parse ItemClass from string
                if let Ok(class) = Self::parse_item_class(&value) {
                    if !affix.allowed_classes.contains(&class) {
                        affix.allowed_classes.push(class);
                        self.text_input = TextInputState::new(String::new());
                    }
                } else {
                    self.message = Some(format!("Unknown class: {}", value));
                }
            }
            _ => {}
        }
    }

    pub fn list_field_remove(&mut self) {
        let field_idx = self.current_view_state().field_index;
        let nested_idx = self.current_view_state().nested_index;

        // Track new list length for adjustment
        let new_len = match &mut self.editing {
            Some(EditingEntry::AffixPool(pool)) if field_idx == 3 => {
                if nested_idx < pool.affixes.len() {
                    pool.affixes.remove(nested_idx);
                    Some(pool.affixes.len())
                } else {
                    None
                }
            }
            Some(EditingEntry::BaseType(bt)) if field_idx == 3 => {
                if nested_idx < bt.tags.len() {
                    bt.tags.remove(nested_idx);
                    Some(bt.tags.len())
                } else {
                    None
                }
            }
            Some(EditingEntry::Affix(affix)) if field_idx == 4 => {
                if nested_idx < affix.tags.len() {
                    affix.tags.remove(nested_idx);
                    Some(affix.tags.len())
                } else {
                    None
                }
            }
            Some(EditingEntry::Affix(affix)) if field_idx == 5 => {
                if nested_idx < affix.allowed_classes.len() {
                    affix.allowed_classes.remove(nested_idx);
                    Some(affix.allowed_classes.len())
                } else {
                    None
                }
            }
            _ => None,
        };

        // Adjust nested index if needed
        if let Some(len) = new_len {
            let state = self.current_view_state_mut();
            if state.nested_index >= len && state.nested_index > 0 {
                state.nested_index -= 1;
            }
        }
    }

    pub fn enter_nested_editor(&mut self) {
        let state = self.current_view_state_mut();
        state.nested_depth += 1;
        state.nested_index = 0;
    }

    pub fn exit_nested_editor(&mut self) {
        let nested_depth = self.current_view_state().nested_depth;

        // Special handling for Currency add_specific_affix list
        if self.is_in_specific_affix_list() {
            if nested_depth >= 3 {
                // Exit from editing a specific affix back to list browsing
                self.current_view_state_mut().nested_depth = 2;
            } else if nested_depth == 2 {
                // Exit from list browsing back to effects field selection
                self.current_view_state_mut().nested_depth = 1;
                self.nested_sub_field_index = 0;
            }
            return;
        }

        // Special handling for Currency rarities list
        if self.is_in_rarities_list() {
            // Exit from list browsing back to requirements field selection
            self.current_view_state_mut().nested_depth = 1;
            self.nested_sub_field_index = 0;
            self.text_input = TextInputState::new(String::new());
            return;
        }

        // Special handling for recipe required_affixes list
        if self.is_in_recipe_affixes_list() {
            self.current_view_state_mut().nested_depth = 1;
            self.nested_sub_field_index = 0;
            self.text_input = TextInputState::new(String::new());
            return;
        }

        // Special handling for recipe mappings list
        if self.is_in_recipe_mappings_list() {
            self.current_view_state_mut().nested_depth = 1;
            self.nested_sub_field_index = 0;
            self.text_input = TextInputState::new(String::new());
            return;
        }

        if nested_depth > 1 {
            // Exit sub-field editing back to item selection
            self.apply_nested_text_input();
            self.current_view_state_mut().nested_depth = 1;
            self.nested_sub_field_index = 0;
        } else if nested_depth > 0 {
            self.current_view_state_mut().nested_depth -= 1;
            self.nested_sub_field_index = 0;
        }
    }

    pub fn nested_item_up(&mut self) {
        // Special handling for Currency add_specific_affix
        if self.is_in_specific_affix_list() {
            if self.nested_sub_field_index > 0 {
                self.nested_sub_field_index -= 1;
            }
            return;
        }

        // Special handling for Currency rarities list
        if self.is_in_rarities_list() {
            if self.nested_sub_field_index > 0 {
                self.nested_sub_field_index -= 1;
            }
            return;
        }

        // Special handling for recipe required_affixes list
        if self.is_in_recipe_affixes_list() {
            if self.nested_sub_field_index > 0 {
                self.nested_sub_field_index -= 1;
            }
            return;
        }

        // Special handling for recipe mappings list
        if self.is_in_recipe_mappings_list() {
            if self.nested_sub_field_index > 0 {
                self.nested_sub_field_index -= 1;
            }
            return;
        }

        let state = self.current_view_state_mut();
        if state.nested_index > 0 {
            state.nested_index -= 1;
        }
    }

    pub fn nested_item_down(&mut self) {
        // Special handling for Currency add_specific_affix
        if self.is_in_specific_affix_list() {
            let max_idx = self.get_specific_affix_count().saturating_sub(1);
            if self.nested_sub_field_index < max_idx {
                self.nested_sub_field_index += 1;
            }
            return;
        }

        // Special handling for Currency rarities list
        if self.is_in_rarities_list() {
            let max_idx = self.get_rarities_count().saturating_sub(1);
            if self.nested_sub_field_index < max_idx {
                self.nested_sub_field_index += 1;
            }
            return;
        }

        // Special handling for recipe required_affixes list
        if self.is_in_recipe_affixes_list() {
            let max_idx = self.get_recipe_affixes_count().saturating_sub(1);
            if self.nested_sub_field_index < max_idx {
                self.nested_sub_field_index += 1;
            }
            return;
        }

        // Special handling for recipe mappings list
        if self.is_in_recipe_mappings_list() {
            let max_idx = self.get_recipe_mappings_count().saturating_sub(1);
            if self.nested_sub_field_index < max_idx {
                self.nested_sub_field_index += 1;
            }
            return;
        }

        let max_idx = self.get_nested_item_count().saturating_sub(1);
        let state = self.current_view_state_mut();
        if state.nested_index < max_idx {
            state.nested_index += 1;
        }
    }

    /// Check if we're in the specific affix list editing mode
    fn is_in_specific_affix_list(&self) -> bool {
        let field_idx = self.current_view_state().field_index;
        let nested_idx = self.current_view_state().nested_index;
        let nested_depth = self.current_view_state().nested_depth;

        matches!(&self.editing, Some(EditingEntry::Currency(_)))
            && field_idx == 5
            && nested_idx == 3
            && nested_depth >= 2
    }

    /// Check if we're in the rarities list editing mode
    pub fn is_in_rarities_list(&self) -> bool {
        let field_idx = self.current_view_state().field_index;
        let nested_idx = self.current_view_state().nested_index;
        let nested_depth = self.current_view_state().nested_depth;

        matches!(&self.editing, Some(EditingEntry::Currency(_)))
            && field_idx == 4
            && nested_idx == 0
            && nested_depth >= 2
    }

    fn get_specific_affix_count(&self) -> usize {
        match &self.editing {
            Some(EditingEntry::Currency(curr)) => curr.effects.add_specific_affix.len(),
            _ => 0,
        }
    }

    fn get_rarities_count(&self) -> usize {
        match &self.editing {
            Some(EditingEntry::Currency(curr)) => curr.requires.rarities.len(),
            _ => 0,
        }
    }

    /// Check if we're in the recipe required_affixes list editing mode
    pub fn is_in_recipe_affixes_list(&self) -> bool {
        let field_idx = self.current_view_state().field_index;
        let nested_idx = self.current_view_state().nested_index;
        let nested_depth = self.current_view_state().nested_depth;

        matches!(&self.editing, Some(EditingEntry::Unique(_)))
            && field_idx == 5
            && nested_idx == 1  // required_affixes is index 1 in recipe sub-fields
            && nested_depth >= 2
    }

    /// Check if we're in the recipe mappings list editing mode
    pub fn is_in_recipe_mappings_list(&self) -> bool {
        let field_idx = self.current_view_state().field_index;
        let nested_idx = self.current_view_state().nested_index;
        let nested_depth = self.current_view_state().nested_depth;

        matches!(&self.editing, Some(EditingEntry::Unique(_)))
            && field_idx == 5
            && nested_idx == 2  // mappings is index 2 in recipe sub-fields
            && nested_depth >= 2
    }

    fn get_recipe_affixes_count(&self) -> usize {
        self.editing_recipe
            .as_ref()
            .map(|r| r.required_affixes.len())
            .unwrap_or(0)
    }

    fn get_recipe_mappings_count(&self) -> usize {
        self.editing_recipe
            .as_ref()
            .map(|r| r.mappings.len())
            .unwrap_or(0)
    }

    fn get_nested_item_count(&self) -> usize {
        let field_idx = self.current_view_state().field_index;
        match &self.editing {
            Some(EditingEntry::Unique(uniq)) if field_idx == 4 => uniq.mods.len(),
            Some(EditingEntry::Unique(_)) if field_idx == 5 => 3, // recipe: weight, required_affixes, mappings
            Some(EditingEntry::Affix(affix)) if field_idx == 6 => affix.tiers.len(),
            Some(EditingEntry::BaseType(_)) if field_idx == 4 => 1, // implicit is single item
            Some(EditingEntry::BaseType(_)) if field_idx == 5 => 3, // defenses: armour, evasion, es
            Some(EditingEntry::BaseType(bt)) if field_idx == 6 => {
                // damage: 3 stats + 1 header + damage entries
                let damage_count = bt.damage.as_ref().map(|d| d.damages.len()).unwrap_or(0);
                4 + damage_count // attack_speed, crit_chance, spell_eff, "Damage Types:" header, + entries
            }
            Some(EditingEntry::BaseType(_)) if field_idx == 7 => 1, // requirements is single item (edited as one)
            Some(EditingEntry::Currency(_)) if field_idx == 4 => 3, // requires: rarities, has_affix, has_affix_slot
            Some(EditingEntry::Currency(_)) if field_idx == 5 => 8, // effects: multiple fields
            _ => 0,
        }
    }

    pub fn nested_item_add(&mut self) {
        let field_idx = self.current_view_state().field_index;
        let nested_idx = self.current_view_state().nested_index;
        match &mut self.editing {
            Some(EditingEntry::Unique(uniq)) if field_idx == 4 => {
                // Add a new mod with default values
                uniq.mods.push(loot_core::config::UniqueModConfig {
                    stat: loot_core::types::StatType::AddedPhysicalDamage,
                    min: 1,
                    max: 10,
                });
                let new_idx = uniq.mods.len() - 1;
                self.current_view_state_mut().nested_index = new_idx;
                // Enter edit mode for the new item
                let text = "AddedPhysicalDamage 1 10".to_string();
                self.text_input = TextInputState::new(text);
                self.current_view_state_mut().nested_depth = 2;
            }
            Some(EditingEntry::Affix(affix)) if field_idx == 6 => {
                // Add a new tier
                let next_tier = affix.tiers.iter().map(|t| t.tier).max().unwrap_or(0) + 1;
                affix.tiers.push(AffixTierConfig {
                    tier: next_tier,
                    weight: 100,
                    min: 1,
                    max: 10,
                    min_ilvl: 1,
                });
                let new_idx = affix.tiers.len() - 1;
                self.current_view_state_mut().nested_index = new_idx;
                // Enter edit mode for the new item
                let text = format!("{} 100 1 10", next_tier);
                self.text_input = TextInputState::new(text);
                self.current_view_state_mut().nested_depth = 2;
            }
            Some(EditingEntry::BaseType(bt)) if field_idx == 6 => {
                // Add a new damage entry
                let damage = bt.damage.get_or_insert_with(|| loot_core::config::DamageConfig {
                    damages: Vec::new(),
                    attack_speed: 1.0,
                    critical_chance: 5.0,
                    spell_efficiency: 0.0,
                });
                damage.damages.push(loot_core::config::DamageEntry {
                    damage_type: loot_core::types::DamageType::Physical,
                    min: 1,
                    max: 10,
                });
                let new_idx = 4 + damage.damages.len() - 1; // 3 stats + header + new entry
                self.current_view_state_mut().nested_index = new_idx;
                // Enter edit mode for the new item
                self.text_input = TextInputState::new("Physical 1 10".to_string());
                self.current_view_state_mut().nested_depth = 2;
                self.message = Some("Added new damage type".to_string());
            }
            Some(EditingEntry::Currency(_)) if field_idx == 5 && nested_idx == 3 => {
                // Don't add entry yet - just enter "add new" mode
                // Entry will be added in apply_nested_text_input after validation
                self.nested_sub_field_index = usize::MAX; // Special marker for "adding new"
                self.text_input = TextInputState::new(String::new());
                self.current_view_state_mut().nested_depth = 3;
            }
            Some(EditingEntry::Currency(curr)) if field_idx == 4 && nested_idx == 0 => {
                // Add rarity - validate input first
                let value = self.text_input.value().trim().to_string();
                if value.is_empty() {
                    self.message = Some("Enter a rarity: Normal, Magic, Rare, Unique".to_string());
                    return;
                }
                match Self::parse_rarity(&value) {
                    Ok(rarity) => {
                        if curr.requires.rarities.contains(&rarity) {
                            self.message = Some(format!("{:?} already in list", rarity));
                        } else {
                            curr.requires.rarities.push(rarity);
                            self.nested_sub_field_index = curr.requires.rarities.len() - 1;
                            self.message = Some(format!("Added {:?}", rarity));
                            self.text_input = TextInputState::new(String::new());
                        }
                    }
                    Err(_) => {
                        self.message = Some(format!(
                            "Invalid rarity: {}. Valid: Normal, Magic, Rare, Unique",
                            value
                        ));
                    }
                }
            }
            Some(EditingEntry::Unique(_)) if field_idx == 5 && nested_idx == 1 => {
                // Add recipe required_affix - validate input first
                let value = self.text_input.value().trim().to_string();
                if value.is_empty() {
                    self.message = Some("Enter: StatType [Prefix|Suffix] [min_tier] [max_tier]".to_string());
                    return;
                }
                let parts: Vec<&str> = value.split_whitespace().collect();
                if parts.is_empty() {
                    self.message = Some("Enter at least a StatType".to_string());
                    return;
                }
                match Self::parse_stat_type(parts[0]) {
                    Ok(stat) => {
                        let affix_type = parts.get(1).and_then(|s| Self::parse_affix_type(s).ok());
                        let min_tier = parts.get(2).and_then(|s| s.parse().ok()).unwrap_or(1);
                        let max_tier = parts.get(3).and_then(|s| s.parse().ok()).unwrap_or(99);

                        let recipe = self.editing_recipe.get_or_insert_with(|| UniqueRecipeConfig {
                            unique_id: String::new(),
                            base_type: String::new(),
                            weight: 100,
                            required_affixes: Vec::new(),
                            mappings: Vec::new(),
                        });
                        recipe.required_affixes.push(RecipeAffixRequirement {
                            stat,
                            affix_type,
                            min_tier,
                            max_tier,
                        });
                        self.nested_sub_field_index = recipe.required_affixes.len() - 1;
                        self.message = Some(format!("Added requirement: {:?}", stat));
                        self.text_input = TextInputState::new(String::new());
                    }
                    Err(_) => {
                        self.message = Some(format!("Invalid stat: {}", parts[0]));
                    }
                }
            }
            Some(EditingEntry::Unique(_)) if field_idx == 5 && nested_idx == 2 => {
                // Add recipe mapping - validate input first
                let value = self.text_input.value().trim().to_string();
                if value.is_empty() {
                    self.message = Some("Enter: StatType mod_index [percentage|direct|random] [influence]".to_string());
                    return;
                }
                let parts: Vec<&str> = value.split_whitespace().collect();
                if parts.len() < 2 {
                    self.message = Some("Enter at least: StatType mod_index".to_string());
                    return;
                }
                match Self::parse_stat_type(parts[0]) {
                    Ok(from_stat) => {
                        let to_mod_index = match parts[1].parse::<usize>() {
                            Ok(idx) => idx,
                            Err(_) => {
                                self.message = Some("Invalid mod_index (must be a number)".to_string());
                                return;
                            }
                        };

                        // Check for duplicate mod_index - only first mapping to each mod is used
                        if let Some(ref recipe) = self.editing_recipe {
                            if recipe.mappings.iter().any(|m| m.to_mod_index == to_mod_index) {
                                self.message = Some(format!(
                                    "Mod index {} already has a mapping. Remove it first to replace.",
                                    to_mod_index
                                ));
                                return;
                            }
                        }

                        let mode = parts.get(2).map(|s| match *s {
                            "direct" => MappingMode::Direct,
                            "random" => MappingMode::Random,
                            _ => MappingMode::Percentage,
                        }).unwrap_or(MappingMode::Percentage);
                        let influence = parts.get(3).and_then(|s| s.parse().ok()).unwrap_or(1.0);

                        let recipe = self.editing_recipe.get_or_insert_with(|| UniqueRecipeConfig {
                            unique_id: String::new(),
                            base_type: String::new(),
                            weight: 100,
                            required_affixes: Vec::new(),
                            mappings: Vec::new(),
                        });
                        recipe.mappings.push(RecipeMapping {
                            from_stat,
                            to_mod_index,
                            mode,
                            influence,
                        });
                        self.nested_sub_field_index = recipe.mappings.len() - 1;
                        self.message = Some(format!("Added mapping: {:?} -> mod {}", from_stat, to_mod_index));
                        self.text_input = TextInputState::new(String::new());
                    }
                    Err(_) => {
                        self.message = Some(format!("Invalid stat: {}", parts[0]));
                    }
                }
            }
            _ => {}
        }
    }

    pub fn nested_item_remove(&mut self) {
        let field_idx = self.current_view_state().field_index;
        let nested_idx = self.current_view_state().nested_index;

        // For Currency add_specific_affix, use nested_sub_field_index
        if let Some(EditingEntry::Currency(curr)) = &mut self.editing {
            if field_idx == 5 && nested_idx == 3 {
                let sub_idx = self.nested_sub_field_index;
                if sub_idx < curr.effects.add_specific_affix.len() {
                    curr.effects.add_specific_affix.remove(sub_idx);
                    let new_len = curr.effects.add_specific_affix.len();
                    if self.nested_sub_field_index >= new_len && self.nested_sub_field_index > 0 {
                        self.nested_sub_field_index -= 1;
                    }
                }
                return;
            }
            // For Currency rarities list
            if field_idx == 4 && nested_idx == 0 {
                let sub_idx = self.nested_sub_field_index;
                if sub_idx < curr.requires.rarities.len() {
                    let removed = curr.requires.rarities.remove(sub_idx);
                    self.message = Some(format!("Removed {:?}", removed));
                    let new_len = curr.requires.rarities.len();
                    if self.nested_sub_field_index >= new_len && self.nested_sub_field_index > 0 {
                        self.nested_sub_field_index -= 1;
                    }
                }
                return;
            }
        }

        // For Unique recipe required_affixes
        if matches!(&self.editing, Some(EditingEntry::Unique(_))) && field_idx == 5 && nested_idx == 1 {
            if let Some(ref mut recipe) = self.editing_recipe {
                let sub_idx = self.nested_sub_field_index;
                if sub_idx < recipe.required_affixes.len() {
                    let removed = recipe.required_affixes.remove(sub_idx);
                    self.message = Some(format!("Removed requirement: {:?}", removed.stat));
                    let new_len = recipe.required_affixes.len();
                    if self.nested_sub_field_index >= new_len && self.nested_sub_field_index > 0 {
                        self.nested_sub_field_index -= 1;
                    }
                }
            }
            return;
        }

        // For Unique recipe mappings
        if matches!(&self.editing, Some(EditingEntry::Unique(_))) && field_idx == 5 && nested_idx == 2 {
            if let Some(ref mut recipe) = self.editing_recipe {
                let sub_idx = self.nested_sub_field_index;
                if sub_idx < recipe.mappings.len() {
                    let removed = recipe.mappings.remove(sub_idx);
                    self.message = Some(format!("Removed mapping: {:?}", removed.from_stat));
                    let new_len = recipe.mappings.len();
                    if self.nested_sub_field_index >= new_len && self.nested_sub_field_index > 0 {
                        self.nested_sub_field_index -= 1;
                    }
                }
            }
            return;
        }

        let new_len = match &mut self.editing {
            Some(EditingEntry::Unique(uniq)) if field_idx == 4 => {
                if nested_idx < uniq.mods.len() && uniq.mods.len() > 0 {
                    uniq.mods.remove(nested_idx);
                    Some(uniq.mods.len())
                } else {
                    None
                }
            }
            Some(EditingEntry::Affix(affix)) if field_idx == 6 => {
                if nested_idx < affix.tiers.len() && affix.tiers.len() > 1 {
                    affix.tiers.remove(nested_idx);
                    Some(affix.tiers.len())
                } else {
                    None
                }
            }
            Some(EditingEntry::BaseType(bt)) if field_idx == 6 => {
                // Only remove damage entries (index 4+), not stats
                if nested_idx >= 4 {
                    let entry_idx = nested_idx - 4;
                    if let Some(ref mut damage) = bt.damage {
                        if entry_idx < damage.damages.len() {
                            damage.damages.remove(entry_idx);
                            Some(4 + damage.damages.len()) // Return new total count
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
            _ => None,
        };

        if let Some(len) = new_len {
            let state = self.current_view_state_mut();
            if state.nested_index >= len && state.nested_index > 0 {
                state.nested_index -= 1;
            }
        }
    }

    pub fn nested_item_edit(&mut self) {
        let field_idx = self.current_view_state().field_index;
        let nested_idx = self.current_view_state().nested_index;
        let nested_depth = self.current_view_state().nested_depth;

        match &self.editing {
            Some(EditingEntry::Unique(uniq)) if field_idx == 4 => {
                if nested_depth == 1 {
                    // Enter editing mode - populate with current values
                    if let Some(mod_cfg) = uniq.mods.get(nested_idx) {
                        let text = format!("{:?} {} {}", mod_cfg.stat, mod_cfg.min, mod_cfg.max);
                        self.text_input = TextInputState::new(text);
                        self.current_view_state_mut().nested_depth = 2;
                    }
                } else if nested_depth >= 2 {
                    // Save the edit
                    self.apply_nested_text_input();
                    self.current_view_state_mut().nested_depth = 1;
                }
            }
            Some(EditingEntry::Affix(affix)) if field_idx == 6 => {
                if nested_depth == 1 {
                    // Enter editing mode - populate with current values
                    if let Some(tier) = affix.tiers.get(nested_idx) {
                        let text = format!(
                            "{} {} {} {} {}",
                            tier.tier, tier.weight, tier.min, tier.max, tier.min_ilvl
                        );
                        self.text_input = TextInputState::new(text);
                        self.current_view_state_mut().nested_depth = 2;
                    }
                } else if nested_depth >= 2 {
                    // Save the edit
                    self.apply_nested_text_input();
                    self.current_view_state_mut().nested_depth = 1;
                }
            }
            Some(EditingEntry::BaseType(bt)) if field_idx == 4 => {
                // Implicit: "StatType min max" or "none" to clear
                if nested_depth == 1 {
                    let text = bt
                        .implicit
                        .as_ref()
                        .map(|i| format!("{:?} {} {}", i.stat, i.min, i.max))
                        .unwrap_or_else(|| "none".to_string());
                    self.text_input = TextInputState::new(text);
                    self.current_view_state_mut().nested_depth = 2;
                } else if nested_depth >= 2 {
                    self.apply_nested_text_input();
                    self.current_view_state_mut().nested_depth = 1;
                }
            }
            Some(EditingEntry::BaseType(bt)) if field_idx == 5 => {
                // Defenses: nested_idx 0=armour, 1=evasion, 2=energy_shield
                if nested_depth == 1 {
                    let text = match nested_idx {
                        0 => bt
                            .defenses
                            .as_ref()
                            .and_then(|d| d.armour.as_ref())
                            .map(|r| format!("{} {}", r.min, r.max))
                            .unwrap_or_else(|| "none".to_string()),
                        1 => bt
                            .defenses
                            .as_ref()
                            .and_then(|d| d.evasion.as_ref())
                            .map(|r| format!("{} {}", r.min, r.max))
                            .unwrap_or_else(|| "none".to_string()),
                        2 => bt
                            .defenses
                            .as_ref()
                            .and_then(|d| d.energy_shield.as_ref())
                            .map(|r| format!("{} {}", r.min, r.max))
                            .unwrap_or_else(|| "none".to_string()),
                        _ => "none".to_string(),
                    };
                    self.text_input = TextInputState::new(text);
                    self.current_view_state_mut().nested_depth = 2;
                } else if nested_depth >= 2 {
                    self.apply_nested_text_input();
                    self.current_view_state_mut().nested_depth = 1;
                }
            }
            Some(EditingEntry::BaseType(bt)) if field_idx == 6 => {
                // Damage: nested_idx 0-2 = stats, 3 = header (skip), 4+ = damage entries
                if nested_depth == 1 {
                    if nested_idx == 3 {
                        // "Damage Types:" header - don't edit, skip to add mode
                        self.message = Some("Press + to add a damage type".to_string());
                        return;
                    }
                    let text = match nested_idx {
                        0 => bt.damage.as_ref().map(|d| format!("{}", d.attack_speed)).unwrap_or_else(|| "1.0".to_string()),
                        1 => bt.damage.as_ref().map(|d| format!("{}", d.critical_chance)).unwrap_or_else(|| "5.0".to_string()),
                        2 => bt.damage.as_ref().map(|d| format!("{}", d.spell_efficiency)).unwrap_or_else(|| "0".to_string()),
                        _ => {
                            // Damage entry (index 4+)
                            let entry_idx = nested_idx - 4;
                            bt.damage.as_ref()
                                .and_then(|d| d.damages.get(entry_idx))
                                .map(|e| format!("{:?} {} {}", e.damage_type, e.min, e.max))
                                .unwrap_or_else(|| "Physical 1 10".to_string())
                        }
                    };
                    self.text_input = TextInputState::new(text);
                    self.current_view_state_mut().nested_depth = 2;
                } else if nested_depth >= 2 {
                    self.apply_nested_text_input();
                    self.current_view_state_mut().nested_depth = 1;
                }
            }
            Some(EditingEntry::BaseType(bt)) if field_idx == 7 => {
                // Requirements: "level str dex int"
                if nested_depth == 1 {
                    let r = &bt.requirements;
                    let text = format!("{} {} {} {}", r.level, r.strength, r.dexterity, r.intelligence);
                    self.text_input = TextInputState::new(text);
                    self.current_view_state_mut().nested_depth = 2;
                } else if nested_depth >= 2 {
                    self.apply_nested_text_input();
                    self.current_view_state_mut().nested_depth = 1;
                }
            }
            Some(EditingEntry::Currency(curr)) if field_idx == 4 => {
                // Requirements: nested_idx 0=rarities (list), 1=has_affix, 2=has_affix_slot
                if nested_idx == 0 {
                    // Rarities is now a list editor
                    if nested_depth == 1 {
                        // Enter list mode - reset sub-field index
                        self.nested_sub_field_index = 0;
                        self.text_input = TextInputState::new(String::new());
                        self.current_view_state_mut().nested_depth = 2;
                    } else if nested_depth >= 2 {
                        // In list mode - Enter adds new rarity (handled by nested_item_add)
                        self.nested_item_add();
                    }
                } else {
                    // Other requirement fields - direct edit
                    if nested_depth == 1 {
                        let text = match nested_idx {
                            1 => curr.requires.has_affix.to_string(),
                            2 => curr.requires.has_affix_slot.to_string(),
                            _ => String::new(),
                        };
                        self.text_input = TextInputState::new(text);
                        self.current_view_state_mut().nested_depth = 2;
                    } else if nested_depth >= 2 {
                        self.apply_nested_text_input();
                        self.current_view_state_mut().nested_depth = 1;
                    }
                }
            }
            Some(EditingEntry::Currency(curr)) if field_idx == 5 => {
                // Effects: multiple sub-fields
                // nested_idx 3 (add_specific_affix) is special - it's a list
                if nested_idx == 3 {
                    // add_specific_affix list handling
                    if nested_depth == 1 {
                        // Enter list mode - reset sub-field index
                        self.nested_sub_field_index = 0;
                        self.text_input = TextInputState::new(String::new());
                        self.current_view_state_mut().nested_depth = 2;
                    } else if nested_depth == 2 {
                        // In list mode - Enter edits selected item
                        if let Some(specific) = curr.effects.add_specific_affix.get(self.nested_sub_field_index) {
                            let tier_str = specific.tier.map(|t| t.to_string()).unwrap_or_default();
                            let text = format!("{} {} {}", specific.id, tier_str, specific.weight);
                            self.text_input = TextInputState::new(text.trim().to_string());
                            self.current_view_state_mut().nested_depth = 3;
                        }
                    } else if nested_depth >= 3 {
                        // Save edit to selected item
                        self.apply_nested_text_input();
                        self.current_view_state_mut().nested_depth = 2;
                    }
                } else {
                    // Other effect fields - direct edit
                    if nested_depth == 1 {
                        let text = match nested_idx {
                            0 => curr.effects.set_rarity
                                .map(|r| format!("{:?}", r))
                                .unwrap_or_else(|| "none".to_string()),
                            1 => curr.effects.clear_affixes.to_string(),
                            2 => curr.effects.add_affixes.as_ref()
                                .map(|c| format!("{} {}", c.min, c.max))
                                .unwrap_or_else(|| "none".to_string()),
                            4 => curr.effects.remove_affixes
                                .map(|n| n.to_string())
                                .unwrap_or_else(|| "none".to_string()),
                            5 => curr.effects.reroll_affixes
                                .map(|n| n.to_string())
                                .unwrap_or_else(|| "none".to_string()),
                            6 => curr.effects.try_unique.to_string(),
                            7 => {
                                if curr.effects.affix_pools.is_empty() {
                                    "none".to_string()
                                } else {
                                    curr.effects.affix_pools.join(", ")
                                }
                            }
                            _ => String::new(),
                        };
                        self.text_input = TextInputState::new(text);
                        self.current_view_state_mut().nested_depth = 2;
                    } else if nested_depth >= 2 {
                        self.apply_nested_text_input();
                        self.current_view_state_mut().nested_depth = 1;
                    }
                }
            }
            Some(EditingEntry::Unique(_)) if field_idx == 5 => {
                // Recipe: nested_idx 0=weight, 1=required_affixes (list), 2=mappings (list)
                match nested_idx {
                    0 => {
                        // Weight - direct edit
                        if nested_depth == 1 {
                            let text = self.editing_recipe.as_ref()
                                .map(|r| r.weight.to_string())
                                .unwrap_or_else(|| "100".to_string());
                            self.text_input = TextInputState::new(text);
                            self.current_view_state_mut().nested_depth = 2;
                        } else if nested_depth >= 2 {
                            // Save weight
                            if let Ok(weight) = self.text_input.value().trim().parse::<u32>() {
                                let recipe = self.editing_recipe.get_or_insert_with(|| UniqueRecipeConfig {
                                    unique_id: String::new(),
                                    base_type: String::new(),
                                    weight: 100,
                                    required_affixes: Vec::new(),
                                    mappings: Vec::new(),
                                });
                                recipe.weight = weight;
                                self.message = Some(format!("Weight set to {}", weight));
                            }
                            self.current_view_state_mut().nested_depth = 1;
                        }
                    }
                    1 => {
                        // Required affixes list
                        if nested_depth == 1 {
                            self.nested_sub_field_index = 0;
                            self.text_input = TextInputState::new(String::new());
                            self.current_view_state_mut().nested_depth = 2;
                        } else if nested_depth >= 2 {
                            // In list mode - Enter adds new item
                            self.nested_item_add();
                        }
                    }
                    2 => {
                        // Mappings list
                        if nested_depth == 1 {
                            self.nested_sub_field_index = 0;
                            self.text_input = TextInputState::new(String::new());
                            self.current_view_state_mut().nested_depth = 2;
                        } else if nested_depth >= 2 {
                            // In list mode - Enter adds new item
                            self.nested_item_add();
                        }
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }

    fn apply_nested_text_input(&mut self) {
        let field_idx = self.current_view_state().field_index;
        let nested_idx = self.current_view_state().nested_index;
        let value = self.text_input.value().to_string();

        match &mut self.editing {
            Some(EditingEntry::Unique(uniq)) if field_idx == 4 => {
                // Parse format: "StatType min max"
                let parts: Vec<&str> = value.split_whitespace().collect();
                if parts.len() >= 3 {
                    if let Some(mod_cfg) = uniq.mods.get_mut(nested_idx) {
                        if let Ok(stat) = Self::parse_stat_type(parts[0]) {
                            mod_cfg.stat = stat;
                        }
                        if let Ok(min) = parts[1].parse::<i32>() {
                            mod_cfg.min = min;
                        }
                        if let Ok(max) = parts[2].parse::<i32>() {
                            mod_cfg.max = max;
                        }
                    }
                }
            }
            Some(EditingEntry::Affix(affix)) if field_idx == 6 => {
                // Parse format: "tier weight min max min_ilvl"
                let parts: Vec<&str> = value.split_whitespace().collect();
                if parts.len() >= 4 {
                    if let Some(tier) = affix.tiers.get_mut(nested_idx) {
                        if let Ok(t) = parts[0].parse::<u32>() {
                            tier.tier = t;
                        }
                        if let Ok(w) = parts[1].parse::<u32>() {
                            tier.weight = w;
                        }
                        if let Ok(min) = parts[2].parse::<i32>() {
                            tier.min = min;
                        }
                        if let Ok(max) = parts[3].parse::<i32>() {
                            tier.max = max;
                        }
                        // Parse min_ilvl if provided (5th field)
                        if let Some(ilvl_str) = parts.get(4) {
                            if let Ok(min_ilvl) = ilvl_str.parse::<u32>() {
                                tier.min_ilvl = min_ilvl;
                            }
                        }
                    }
                }
            }
            Some(EditingEntry::BaseType(bt)) if field_idx == 4 => {
                // Implicit: "StatType min max" or "none" to clear
                if value.trim().eq_ignore_ascii_case("none") {
                    bt.implicit = None;
                } else {
                    let parts: Vec<&str> = value.split_whitespace().collect();
                    if parts.len() >= 3 {
                        if let Ok(stat) = Self::parse_stat_type(parts[0]) {
                            if let (Ok(min), Ok(max)) = (parts[1].parse::<i32>(), parts[2].parse::<i32>()) {
                                bt.implicit = Some(loot_core::config::ImplicitConfig { stat, min, max });
                            }
                        }
                    }
                }
            }
            Some(EditingEntry::BaseType(bt)) if field_idx == 5 => {
                // Defenses: "min max" or "none" to clear
                if bt.defenses.is_none() {
                    bt.defenses = Some(loot_core::config::DefensesConfig {
                        armour: None,
                        evasion: None,
                        energy_shield: None,
                    });
                }
                if let Some(ref mut defenses) = bt.defenses {
                    if value.trim().eq_ignore_ascii_case("none") {
                        match nested_idx {
                            0 => defenses.armour = None,
                            1 => defenses.evasion = None,
                            2 => defenses.energy_shield = None,
                            _ => {}
                        }
                    } else {
                        let parts: Vec<&str> = value.split_whitespace().collect();
                        if parts.len() >= 2 {
                            if let (Ok(min), Ok(max)) = (parts[0].parse::<i32>(), parts[1].parse::<i32>()) {
                                let range = loot_core::config::RollRange { min, max };
                                match nested_idx {
                                    0 => defenses.armour = Some(range),
                                    1 => defenses.evasion = Some(range),
                                    2 => defenses.energy_shield = Some(range),
                                    _ => {}
                                }
                            }
                        }
                    }
                }
            }
            Some(EditingEntry::BaseType(bt)) if field_idx == 6 => {
                // Damage: nested_idx 0-2 = stats, 3 = header (skip), 4+ = damage entries
                if value.trim().eq_ignore_ascii_case("none") {
                    bt.damage = None;
                    self.message = Some("Damage cleared".to_string());
                } else {
                    // Ensure damage config exists
                    let damage = bt.damage.get_or_insert_with(|| loot_core::config::DamageConfig {
                        damages: Vec::new(),
                        attack_speed: 1.0,
                        critical_chance: 5.0,
                        spell_efficiency: 0.0,
                    });

                    match nested_idx {
                        0 => {
                            // Attack speed
                            if let Ok(v) = value.trim().parse::<f32>() {
                                damage.attack_speed = v;
                            }
                        }
                        1 => {
                            // Crit chance
                            if let Ok(v) = value.trim().parse::<f32>() {
                                damage.critical_chance = v;
                            }
                        }
                        2 => {
                            // Spell efficiency
                            if let Ok(v) = value.trim().parse::<f32>() {
                                damage.spell_efficiency = v;
                            }
                        }
                        3 => {
                            // Header - do nothing
                        }
                        _ => {
                            // Damage entry: "Type min max"
                            let entry_idx = nested_idx - 4;
                            let parts: Vec<&str> = value.split_whitespace().collect();
                            if parts.len() >= 3 {
                                if let Ok(damage_type) = Self::parse_damage_type(parts[0]) {
                                    if let (Ok(min), Ok(max)) = (parts[1].parse::<i32>(), parts[2].parse::<i32>()) {
                                        if let Some(entry) = damage.damages.get_mut(entry_idx) {
                                            entry.damage_type = damage_type;
                                            entry.min = min;
                                            entry.max = max;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            Some(EditingEntry::BaseType(bt)) if field_idx == 7 => {
                // Requirements: "level str dex int"
                let parts: Vec<&str> = value.split_whitespace().collect();
                if parts.len() >= 4 {
                    if let (Ok(level), Ok(strength), Ok(dexterity), Ok(intelligence)) = (
                        parts[0].parse::<u32>(),
                        parts[1].parse::<u32>(),
                        parts[2].parse::<u32>(),
                        parts[3].parse::<u32>(),
                    ) {
                        bt.requirements.level = level;
                        bt.requirements.strength = strength;
                        bt.requirements.dexterity = dexterity;
                        bt.requirements.intelligence = intelligence;
                    }
                }
            }
            Some(EditingEntry::Currency(curr)) if field_idx == 4 => {
                // Requirements: nested_idx 0=rarities (list editor), 1=has_affix, 2=has_affix_slot
                match nested_idx {
                    0 => {
                        // Rarities now uses list editor - this case is handled by nested_item_add
                    }
                    1 => {
                        curr.requires.has_affix = value.trim().eq_ignore_ascii_case("true");
                    }
                    2 => {
                        curr.requires.has_affix_slot = value.trim().eq_ignore_ascii_case("true");
                    }
                    _ => {}
                }
            }
            Some(EditingEntry::Currency(curr)) if field_idx == 5 => {
                // Effects: multiple sub-fields
                match nested_idx {
                    0 => {
                        // set_rarity
                        if value.trim().eq_ignore_ascii_case("none") {
                            curr.effects.set_rarity = None;
                        } else if let Ok(rarity) = Self::parse_rarity(value.trim()) {
                            curr.effects.set_rarity = Some(rarity);
                        }
                    }
                    1 => {
                        // clear_affixes
                        curr.effects.clear_affixes = value.trim().eq_ignore_ascii_case("true");
                    }
                    2 => {
                        // add_affixes (min max or "none")
                        if value.trim().eq_ignore_ascii_case("none") {
                            curr.effects.add_affixes = None;
                        } else {
                            let parts: Vec<&str> = value.split_whitespace().collect();
                            if parts.len() >= 2 {
                                if let (Ok(min), Ok(max)) = (parts[0].parse::<u32>(), parts[1].parse::<u32>()) {
                                    curr.effects.add_affixes = Some(loot_core::config::AffixCount { min, max });
                                }
                            } else if let Ok(n) = value.trim().parse::<u32>() {
                                // Single number means min=max
                                curr.effects.add_affixes = Some(loot_core::config::AffixCount { min: n, max: n });
                            }
                        }
                    }
                    3 => {
                        // add_specific_affix - edit existing or add new
                        // format: "affix_id [tier] [weight]"
                        let parts: Vec<&str> = value.split_whitespace().collect();
                        if !parts.is_empty() {
                            let id = parts[0].to_string();
                            // Validate the affix ID exists
                            if !self.config.affixes.contains_key(&id) {
                                self.message = Some(format!("Unknown affix: {}", id));
                                return;
                            }
                            let tier = parts.get(1).and_then(|s| s.parse::<u32>().ok());
                            let weight = parts.get(2).and_then(|s| s.parse::<u32>().ok()).unwrap_or(100);
                            let sub_idx = self.nested_sub_field_index;

                            if sub_idx == usize::MAX {
                                // Adding new entry
                                curr.effects.add_specific_affix.push(loot_core::config::SpecificAffix {
                                    id,
                                    tier,
                                    weight,
                                });
                                self.nested_sub_field_index = curr.effects.add_specific_affix.len() - 1;
                                self.message = Some("Affix added".to_string());
                            } else if let Some(specific) = curr.effects.add_specific_affix.get_mut(sub_idx) {
                                // Editing existing entry
                                specific.id = id;
                                specific.tier = tier;
                                specific.weight = weight;
                                self.message = Some("Affix updated".to_string());
                            }
                        }
                    }
                    4 => {
                        // remove_affixes
                        if value.trim().eq_ignore_ascii_case("none") {
                            curr.effects.remove_affixes = None;
                        } else if let Ok(n) = value.trim().parse::<u32>() {
                            curr.effects.remove_affixes = Some(n);
                        }
                    }
                    5 => {
                        // reroll_affixes
                        if value.trim().eq_ignore_ascii_case("none") {
                            curr.effects.reroll_affixes = None;
                        } else if let Ok(n) = value.trim().parse::<u32>() {
                            curr.effects.reroll_affixes = Some(n);
                        }
                    }
                    6 => {
                        // try_unique
                        curr.effects.try_unique = value.trim().eq_ignore_ascii_case("true");
                    }
                    7 => {
                        // affix_pools (comma-separated or "none")
                        if value.trim().eq_ignore_ascii_case("none") {
                            curr.effects.affix_pools.clear();
                        } else {
                            curr.effects.affix_pools = value
                                .split(',')
                                .map(|s| s.trim().to_string())
                                .filter(|s| !s.is_empty())
                                .collect();
                        }
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }

    // Rendering
    pub fn render(&mut self, f: &mut Frame) {
        // Show config error if any
        if let Some(ref error) = self.config_error {
            self.render_config_error(f, error.clone());
            return;
        }

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Tab bar
                Constraint::Min(0),    // Main content
                Constraint::Length(3), // Help bar
            ])
            .split(f.area());

        self.render_tabs(f, chunks[0]);
        self.render_main(f, chunks[1]);
        self.render_help(f, chunks[2]);

        // Render popups
        if self.show_quit_confirm {
            self.render_quit_confirm(f);
        }
        if self.show_delete_confirm {
            self.render_delete_confirm(f);
        }
        if self.show_file_picker {
            self.render_file_picker(f);
        }
        if self.show_new_file_input {
            self.render_new_file_input(f);
        }
    }

    fn render_config_error(&self, f: &mut Frame, error: String) {
        let area = centered_rect(60, 40, f.area());
        f.render_widget(Clear, area);

        let text = vec![
            Line::from(Span::styled(
                "Configuration Error",
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(error),
            Line::from(""),
            Line::from(Span::styled(
                "Press q to quit",
                Style::default().fg(Color::DarkGray),
            )),
        ];

        let paragraph = Paragraph::new(text).block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Red)),
        );
        f.render_widget(paragraph, area);
    }

    fn render_tabs(&self, f: &mut Frame, area: Rect) {
        let titles: Vec<&str> = vec![
            "1:BaseTypes",
            "2:Affixes",
            "3:AffixPools",
            "4:Currencies",
            "5:Uniques",
        ];

        let tabs = Tabs::new(titles)
            .select(self.current_tab.index())
            .style(Style::default())
            .highlight_style(
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )
            .divider("|")
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" Config Editor "),
            );

        f.render_widget(tabs, area);
    }

    fn render_main(&mut self, f: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
            .split(area);

        self.render_list(f, chunks[0]);
        self.render_detail(f, chunks[1]);
    }

    fn render_list(&mut self, f: &mut Frame, area: Rect) {
        // Clone ids to avoid borrow conflicts
        let ids: Vec<String> = self.current_view_state().ids.clone();

        let items: Vec<ListItem> = ids
            .iter()
            .map(|id| {
                let dirty = self.dirty.is_entry_dirty(self.current_tab, id);
                let marker = if dirty { "* " } else { "  " };
                ListItem::new(Line::from(vec![
                    Span::styled(marker.to_string(), Style::default().fg(Color::Yellow)),
                    Span::raw(id.clone()),
                ]))
            })
            .collect();

        let border_style = if self.focus == Focus::List {
            Style::default().fg(Color::Cyan)
        } else {
            Style::default()
        };

        let title = format!(" {} ", self.current_tab.as_str());
        let list = List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(border_style)
                    .title(title),
            )
            .highlight_style(
                Style::default()
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("> ");

        let state_mut = self.current_view_state_mut();
        f.render_stateful_widget(list, area, &mut state_mut.list_state);
    }

    fn render_detail(&mut self, f: &mut Frame, area: Rect) {
        match self.mode {
            Mode::Browse => self.render_detail_preview(f, area),
            Mode::Edit | Mode::Create => self.render_edit_form(f, area),
        }
    }

    fn render_detail_preview(&self, f: &mut Frame, area: Rect) {
        let state = self.current_view_state();
        let content = match state.selected_id() {
            Some(id) => match self.current_tab {
                ConfigTab::BaseTypes => views::base_types::render_preview(&self.config, id),
                ConfigTab::Affixes => views::affixes::render_preview(&self.config, id),
                ConfigTab::AffixPools => views::affix_pools::render_preview(&self.config, id),
                ConfigTab::Currencies => views::currencies::render_preview(&self.config, id),
                ConfigTab::Uniques => views::uniques::render_preview(&self.config, id),
            },
            None => vec![Line::from("No entry selected")],
        };

        let paragraph =
            Paragraph::new(content).block(Block::default().borders(Borders::ALL).title(" Detail "));
        f.render_widget(paragraph, area);
    }

    fn render_edit_form(&self, f: &mut Frame, area: Rect) {
        let title = if self.mode == Mode::Create {
            " Create New "
        } else {
            " Edit "
        };

        let content = match &self.editing {
            Some(EditingEntry::BaseType(bt)) => views::base_types::render_edit_form(bt, self),
            Some(EditingEntry::Affix(affix)) => views::affixes::render_edit_form(affix, self),
            Some(EditingEntry::AffixPool(pool)) => views::affix_pools::render_edit_form(pool, self),
            Some(EditingEntry::Currency(curr)) => views::currencies::render_edit_form(curr, self),
            Some(EditingEntry::Unique(uniq)) => views::uniques::render_edit_form(uniq, self),
            None => vec![Line::from("No entry being edited")],
        };

        let paragraph = Paragraph::new(content).block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan))
                .title(title),
        );
        f.render_widget(paragraph, area);
    }

    fn render_help(&self, f: &mut Frame, area: Rect) {
        let help_text = if let Some(ref msg) = self.message {
            Span::styled(msg.clone(), Style::default().fg(Color::Yellow))
        } else {
            match self.mode {
                Mode::Browse => {
                    Span::raw("1-5:Tab | j/k:Nav | e:Edit | n:New | d:Delete | s:Save | q:Quit")
                }
                Mode::Edit | Mode::Create => Span::raw("Tab:Next | Ctrl+S:Save | Esc:Cancel"),
            }
        };

        let dirty_indicator = if self.dirty.is_dirty() {
            Span::styled(" [*Modified] ", Style::default().fg(Color::Yellow))
        } else {
            Span::raw("")
        };

        let help = Paragraph::new(Line::from(vec![help_text, dirty_indicator]))
            .block(Block::default().borders(Borders::ALL).title(" Help "));
        f.render_widget(help, area);
    }

    fn render_quit_confirm(&self, f: &mut Frame) {
        let area = centered_rect(40, 20, f.area());
        f.render_widget(Clear, area);

        let text = vec![
            Line::from("You have unsaved changes."),
            Line::from(""),
            Line::from("Quit without saving? (y/n)"),
        ];

        let paragraph = Paragraph::new(text).block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Yellow))
                .title(" Confirm Quit "),
        );
        f.render_widget(paragraph, area);
    }

    fn render_delete_confirm(&self, f: &mut Frame) {
        let area = centered_rect(40, 20, f.area());
        f.render_widget(Clear, area);

        let id = self.current_view_state().selected_id().unwrap_or("???");
        let text = vec![
            Line::from(format!("Delete '{}'?", id)),
            Line::from(""),
            Line::from("Press y to confirm, n to cancel"),
        ];

        let paragraph = Paragraph::new(text).block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Red))
                .title(" Confirm Delete "),
        );
        f.render_widget(paragraph, area);
    }

    fn render_file_picker(&mut self, f: &mut Frame) {
        let area = centered_rect(50, 60, f.area());
        f.render_widget(Clear, area);

        let items: Vec<ListItem> = self
            .file_picker_files
            .iter()
            .map(|p| {
                let name = p
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default();
                ListItem::new(name)
            })
            .collect();

        let list = List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Cyan))
                    .title(" Select File (n:New | Enter:Select | Esc:Cancel) "),
            )
            .highlight_style(
                Style::default()
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("> ");

        f.render_stateful_widget(list, area, &mut self.file_picker_state);
    }

    fn render_new_file_input(&self, f: &mut Frame) {
        let area = centered_rect(40, 15, f.area());
        f.render_widget(Clear, area);

        let text = vec![
            Line::from("Enter new filename:"),
            Line::from(""),
            Line::from(Span::styled(
                format!("{}_", self.new_file_name),
                Style::default().fg(Color::Cyan),
            )),
        ];

        let paragraph = Paragraph::new(text).block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan))
                .title(" New File "),
        );
        f.render_widget(paragraph, area);
    }
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
