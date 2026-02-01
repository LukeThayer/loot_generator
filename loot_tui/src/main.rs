use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use loot_core::config::{Config, ConfigError, MappingMode};
use loot_core::currency::apply_currency;
use loot_core::generator::Generator;
use loot_core::storage::{Operation, StoredItem};
use loot_core::Item;
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Tabs, Wrap},
    Frame, Terminal,
};
use std::io;
use std::path::Path;

fn main() -> Result<(), io::Error> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Load config and create app
    let config_path = Path::new("config");
    let mut app = match Config::load_from_dir(config_path) {
        Ok(config) => App::new(config),
        Err(e) => App::with_config_error(e),
    };

    // Run app
    let res = run_app(&mut terminal, &mut app);

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("Error: {err:?}");
    }

    Ok(())
}

#[derive(PartialEq, Eq, Clone, Copy)]
enum Focus {
    Inventory,
    BaseTypes,
    Uniques,
    Currencies,
    AddAffix,
}

#[derive(PartialEq, Eq, Clone, Copy)]
enum DetailTab {
    Stats,
    Seed,
    Recipes,
}

/// Tracks which affixes were changed by the last currency operation
#[derive(Default, Clone)]
struct ChangedAffixes {
    /// Indices of changed/new prefixes
    prefixes: Vec<usize>,
    /// Indices of changed/new suffixes
    suffixes: Vec<usize>,
}

/// State for the Add Affix popup
#[derive(Default)]
struct AddAffixState {
    /// Available affixes for the current item
    affixes: Vec<(String, String, loot_core::AffixType)>, // (id, name, type)
    /// Available tiers for the selected affix
    tiers: Vec<(u32, i32, i32, Option<(i32, i32)>)>, // (tier, min, max, max_value range)
    /// Current selection in the affix list
    affix_state: ListState,
    /// Current selection in the tier list
    tier_state: ListState,
    /// Which column is focused (0 = affixes, 1 = tiers)
    column: usize,
}

/// State for the Currency popup
#[derive(Default)]
struct CurrencyPopupState {
    /// Available categories (sorted)
    categories: Vec<String>,
    /// Currently selected category index
    selected_category: usize,
    /// Currencies in the current category (id, name, description)
    currencies: Vec<(String, String, String)>,
    /// List state for currency selection
    list_state: ListState,
}

struct App {
    generator: Option<Generator>,
    inventory: Vec<(Item, StoredItem)>,
    inventory_state: ListState,
    base_type_state: ListState,
    unique_state: ListState,
    focus: Focus,
    detail_tab: DetailTab,
    show_base_types: bool,
    show_uniques: bool,
    show_add_affix: bool,
    show_currencies: bool,
    message: Option<String>,
    base_type_ids: Vec<String>,
    unique_ids: Vec<String>,
    /// Tracks which affixes changed from the last operation
    changed_affixes: ChangedAffixes,
    /// State for the Add Affix popup
    add_affix_state: AddAffixState,
    /// State for the Currency popup
    currency_popup_state: CurrencyPopupState,
    /// Config error if loading failed
    config_error: Option<ConfigError>,
}

impl App {
    fn new(config: Config) -> Self {
        let generator = Generator::new(config);
        let base_type_ids: Vec<String> = generator.base_type_ids().into_iter().cloned().collect();
        let unique_ids: Vec<String> = generator.unique_ids().into_iter().cloned().collect();

        // Build currency categories
        let mut categories: Vec<String> = generator
            .config()
            .currencies
            .values()
            .map(|c| {
                if c.category.is_empty() {
                    "Other".to_string()
                } else {
                    c.category.clone()
                }
            })
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();
        categories.sort();

        let mut base_type_state = ListState::default();
        if !base_type_ids.is_empty() {
            base_type_state.select(Some(0));
        }

        let mut unique_state = ListState::default();
        if !unique_ids.is_empty() {
            unique_state.select(Some(0));
        }

        let currency_popup_state = CurrencyPopupState {
            categories,
            selected_category: 0,
            currencies: Vec::new(),
            list_state: ListState::default(),
        };

        App {
            generator: Some(generator),
            inventory: Vec::new(),
            inventory_state: ListState::default(),
            base_type_state,
            unique_state,
            focus: Focus::Inventory,
            detail_tab: DetailTab::Stats,
            show_base_types: false,
            show_uniques: false,
            show_add_affix: false,
            show_currencies: false,
            message: None,
            base_type_ids,
            unique_ids,
            changed_affixes: ChangedAffixes::default(),
            add_affix_state: AddAffixState::default(),
            currency_popup_state,
            config_error: None,
        }
    }

    fn with_config_error(error: ConfigError) -> Self {
        App {
            generator: None,
            inventory: Vec::new(),
            inventory_state: ListState::default(),
            base_type_state: ListState::default(),
            unique_state: ListState::default(),
            focus: Focus::Inventory,
            detail_tab: DetailTab::Stats,
            show_base_types: false,
            show_uniques: false,
            show_add_affix: false,
            show_currencies: false,
            message: None,
            base_type_ids: Vec::new(),
            unique_ids: Vec::new(),
            changed_affixes: ChangedAffixes::default(),
            add_affix_state: AddAffixState::default(),
            currency_popup_state: CurrencyPopupState::default(),
            config_error: Some(error),
        }
    }

    fn generator(&self) -> &Generator {
        self.generator.as_ref().expect("Generator not available - config error")
    }

    fn selected_item(&self) -> Option<&(Item, StoredItem)> {
        self.inventory_state
            .selected()
            .and_then(|i| self.inventory.get(i))
    }

    fn generate_item(&mut self, base_type_id: &str) {
        let seed: u64 = rand::random();
        let mut rng = Generator::make_rng(seed);

        if let Some(item) = self.generator().generate_normal(base_type_id, &mut rng) {
            let stored = StoredItem::new(base_type_id.to_string(), seed);
            self.message = Some(format!("Generated: {}", item.name));
            self.inventory.push((item, stored));
            self.inventory_state.select(Some(self.inventory.len() - 1));
        }
    }

    fn generate_unique_item(&mut self, unique_id: &str) {
        let seed: u64 = rand::random();
        let mut rng = Generator::make_rng(seed);

        if let Some(item) = self.generator().generate_unique(unique_id, &mut rng) {
            // Store with unique_ prefix to distinguish from normal items
            let stored = StoredItem::new(format!("unique:{}", unique_id), seed);
            self.message = Some(format!("Generated unique: {}", item.name));
            self.inventory.push((item, stored));
            self.inventory_state.select(Some(self.inventory.len() - 1));
        }
    }

    fn apply_currency_by_id(&mut self, currency_id: &str) {
        // Clear previous highlights
        self.changed_affixes = ChangedAffixes::default();

        let Some(currency) = self.generator().config().currencies.get(currency_id) else {
            self.message = Some(format!("Unknown currency: {}", currency_id));
            return;
        };

        let currency_name = currency.name.clone();

        let Some(idx) = self.inventory_state.selected() else {
            self.message = Some("No item selected".to_string());
            return;
        };

        // Clone currency config before getting mutable reference to inventory
        let currency = self.generator().config().currencies.get(currency_id).unwrap().clone();

        let Some((item, stored)) = self.inventory.get_mut(idx) else {
            self.message = Some("No item selected".to_string());
            return;
        };

        // Save affix state before applying
        let before_prefix_ids: Vec<String> =
            item.prefixes.iter().map(|m| format!("{}:{}", m.affix_id, m.value)).collect();
        let before_suffix_ids: Vec<String> =
            item.suffixes.iter().map(|m| format!("{}:{}", m.affix_id, m.value)).collect();

        let op_seed: u64 = rand::random();
        let mut rng = Generator::make_rng(op_seed);

        // Get generator reference - safe because we're not borrowing inventory anymore here
        let generator = self.generator.as_ref().unwrap();

        match apply_currency(generator, item, &currency, &mut rng) {
            Ok(()) => {
                stored.push_operation(Operation::Currency(currency_id.to_string()));
                self.message = Some(format!("Applied {} -> {}", currency_name, item.name));

                // Find which affixes changed
                for (i, prefix) in item.prefixes.iter().enumerate() {
                    let key = format!("{}:{}", prefix.affix_id, prefix.value);
                    if !before_prefix_ids.contains(&key) {
                        self.changed_affixes.prefixes.push(i);
                    }
                }
                for (i, suffix) in item.suffixes.iter().enumerate() {
                    let key = format!("{}:{}", suffix.affix_id, suffix.value);
                    if !before_suffix_ids.contains(&key) {
                        self.changed_affixes.suffixes.push(i);
                    }
                }
            }
            Err(e) => {
                self.message = Some(format!("Error: {}", e));
            }
        }
    }

    fn delete_selected(&mut self) {
        if let Some(idx) = self.inventory_state.selected() {
            if idx < self.inventory.len() {
                self.inventory.remove(idx);
                if self.inventory.is_empty() {
                    self.inventory_state.select(None);
                } else if idx >= self.inventory.len() {
                    self.inventory_state.select(Some(self.inventory.len() - 1));
                }
                self.message = Some("Item deleted".to_string());
            }
        }
    }

    fn open_add_affix(&mut self) {
        let Some(idx) = self.inventory_state.selected() else {
            self.message = Some("No item selected".to_string());
            return;
        };

        let Some((item, _)) = self.inventory.get(idx) else {
            return;
        };

        // Can't add affixes to Unique items
        if item.rarity == loot_core::Rarity::Unique {
            self.message = Some("Cannot add affixes to Unique items".to_string());
            return;
        }

        // Get existing affix IDs to filter out
        let existing_ids: Vec<String> = item
            .prefixes
            .iter()
            .chain(item.suffixes.iter())
            .map(|m| m.affix_id.clone())
            .collect();

        // Get available affixes for this item class
        // Allow up to 3 prefixes and 3 suffixes regardless of current rarity
        let mut affixes: Vec<(String, String, loot_core::AffixType)> = Vec::new();

        for affix in self.generator().config().affixes.values() {
            // Skip if already on item
            if existing_ids.contains(&affix.id) {
                continue;
            }

            // Check if allowed for this item class
            if !affix.allowed_classes.is_empty() && !affix.allowed_classes.contains(&item.class) {
                continue;
            }

            // Check if there's room for this affix type (max 3 each)
            let can_add = match affix.affix_type {
                loot_core::AffixType::Prefix => item.prefixes.len() < 3,
                loot_core::AffixType::Suffix => item.suffixes.len() < 3,
            };

            if can_add {
                affixes.push((affix.id.clone(), affix.name.clone(), affix.affix_type));
            }
        }

        if affixes.is_empty() {
            self.message = Some("No affixes available".to_string());
            return;
        }

        // Sort by name
        affixes.sort_by(|a, b| a.1.cmp(&b.1));

        self.add_affix_state.affixes = affixes;
        self.add_affix_state.affix_state.select(Some(0));
        self.add_affix_state.column = 0;
        self.update_affix_tiers();

        self.show_add_affix = true;
        self.focus = Focus::AddAffix;
    }

    fn update_affix_tiers(&mut self) {
        let Some(idx) = self.add_affix_state.affix_state.selected() else {
            return;
        };

        let Some((affix_id, _, _)) = self.add_affix_state.affixes.get(idx) else {
            return;
        };

        let Some(affix) = self.generator().config().affixes.get(affix_id) else {
            return;
        };

        let tiers: Vec<(u32, i32, i32, Option<(i32, i32)>)> = affix
            .tiers
            .iter()
            .map(|t| (t.tier, t.min, t.max, t.max_value.map(|r| (r.min, r.max))))
            .collect();

        self.add_affix_state.tiers = tiers;
        self.add_affix_state.tier_state.select(Some(0));
    }

    fn add_selected_affix(&mut self) {
        let affix_idx = self.add_affix_state.affix_state.selected();
        let tier_idx = self.add_affix_state.tier_state.selected();

        let Some(affix_idx) = affix_idx else { return };
        let Some(tier_idx) = tier_idx else { return };

        let Some((affix_id, _, _)) = self.add_affix_state.affixes.get(affix_idx) else {
            return;
        };

        let Some(affix) = self.generator().config().affixes.get(affix_id).cloned() else {
            return;
        };

        let Some(tier) = affix.tiers.get(tier_idx).cloned() else {
            return;
        };

        let Some(inv_idx) = self.inventory_state.selected() else {
            return;
        };

        // Roll a value within the tier's range
        let (value, value_max) = {
            use rand::Rng;
            let mut rng = rand::thread_rng();
            let v = rng.gen_range(tier.min..=tier.max);
            let v_max = tier.max_value.map(|range| rng.gen_range(range.min..=range.max));
            (v, v_max)
        };

        let modifier = loot_core::item::Modifier {
            affix_id: affix.id.clone(),
            name: affix.name.clone(),
            stat: affix.stat,
            tier: tier.tier,
            value,
            value_max,
            tier_min: tier.min,
            tier_max: tier.max,
            tier_max_value: tier.max_value.map(|r| (r.min, r.max)),
        };

        let affix_type = affix.affix_type;
        let affix_name = affix.name.clone();
        let tier_num = tier.tier;

        let Some((item, _)) = self.inventory.get_mut(inv_idx) else {
            return;
        };

        // Track the change
        self.changed_affixes = ChangedAffixes::default();

        match affix_type {
            loot_core::AffixType::Prefix => {
                self.changed_affixes.prefixes.push(item.prefixes.len());
                item.prefixes.push(modifier);
            }
            loot_core::AffixType::Suffix => {
                self.changed_affixes.suffixes.push(item.suffixes.len());
                item.suffixes.push(modifier);
            }
        }

        // Upgrade rarity based on total affix count
        let total_affixes = item.prefixes.len() + item.suffixes.len();
        if total_affixes >= 3 && item.rarity != loot_core::Rarity::Rare {
            item.rarity = loot_core::Rarity::Rare;
            // Generate a rare name if upgrading to rare
            let seed: u64 = rand::random();
            let mut rng = Generator::make_rng(seed);
            item.name = self.generator.as_ref().unwrap().generate_rare_name(&mut rng);
        } else if total_affixes >= 1 && item.rarity == loot_core::Rarity::Normal {
            item.rarity = loot_core::Rarity::Magic;
        }

        self.message = Some(format!("Added {} T{} ({})", affix_name, tier_num, value));
        self.show_add_affix = false;
        self.focus = Focus::Inventory;
    }

    fn open_currency_popup(&mut self) {
        if self.inventory_state.selected().is_none() {
            self.message = Some("No item selected".to_string());
            return;
        }

        self.update_currency_list();
        self.show_currencies = true;
        self.focus = Focus::Currencies;
    }

    fn update_currency_list(&mut self) {
        let category = self.currency_popup_state.categories
            .get(self.currency_popup_state.selected_category)
            .cloned()
            .unwrap_or_default();

        let mut currencies: Vec<(String, String, String)> = self.generator()
            .config()
            .currencies
            .values()
            .filter(|c| {
                let c_cat = if c.category.is_empty() { "Other" } else { &c.category };
                c_cat == category
            })
            .map(|c| (c.id.clone(), c.name.clone(), c.description.clone()))
            .collect();

        currencies.sort_by(|a, b| a.1.cmp(&b.1));

        self.currency_popup_state.currencies = currencies;
        if !self.currency_popup_state.currencies.is_empty() {
            self.currency_popup_state.list_state.select(Some(0));
        } else {
            self.currency_popup_state.list_state.select(None);
        }
    }

    fn next_currency_category(&mut self) {
        if !self.currency_popup_state.categories.is_empty() {
            self.currency_popup_state.selected_category =
                (self.currency_popup_state.selected_category + 1) % self.currency_popup_state.categories.len();
            self.update_currency_list();
        }
    }

    fn prev_currency_category(&mut self) {
        if !self.currency_popup_state.categories.is_empty() {
            if self.currency_popup_state.selected_category == 0 {
                self.currency_popup_state.selected_category = self.currency_popup_state.categories.len() - 1;
            } else {
                self.currency_popup_state.selected_category -= 1;
            }
            self.update_currency_list();
        }
    }

    fn apply_selected_currency(&mut self) {
        let currency_id = self.currency_popup_state.list_state.selected()
            .and_then(|i| self.currency_popup_state.currencies.get(i))
            .map(|(id, _, _)| id.clone());

        if let Some(id) = currency_id {
            // Don't close the popup - let user keep applying currencies
            self.apply_currency_by_id(&id);
        }
    }

    fn get_selected_currency_id(&self) -> Option<&str> {
        self.currency_popup_state.list_state.selected()
            .and_then(|i| self.currency_popup_state.currencies.get(i))
            .map(|(id, _, _)| id.as_str())
    }
}

fn run_app(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>, app: &mut App) -> io::Result<()> {
    loop {
        terminal.draw(|f| ui(f, app))?;

        if let Event::Key(key) = event::read()? {
            // Only handle key press events, not release or repeat
            if key.kind != KeyEventKind::Press {
                continue;
            }

            // Clear message on any keypress
            app.message = None;

            // Global keys
            match key.code {
                KeyCode::Char('q') => return Ok(()),
                KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    return Ok(())
                }
                KeyCode::Tab => {
                    app.detail_tab = match app.detail_tab {
                        DetailTab::Stats => DetailTab::Seed,
                        DetailTab::Seed => DetailTab::Recipes,
                        DetailTab::Recipes => DetailTab::Stats,
                    };
                    continue;
                }
                KeyCode::Char('n') => {
                    app.show_base_types = true;
                    app.focus = Focus::BaseTypes;
                    continue;
                }
                KeyCode::Char('U') => {
                    if !app.unique_ids.is_empty() {
                        app.show_uniques = true;
                        app.focus = Focus::Uniques;
                    } else {
                        app.message = Some("No uniques configured".to_string());
                    }
                    continue;
                }
                KeyCode::Char('A') => {
                    app.open_add_affix();
                    continue;
                }
                KeyCode::Char('c') => {
                    if app.show_currencies {
                        app.show_currencies = false;
                        app.focus = Focus::Inventory;
                    } else {
                        app.open_currency_popup();
                    }
                    continue;
                }
                KeyCode::Esc => {
                    if app.show_base_types {
                        app.show_base_types = false;
                        app.focus = Focus::Inventory;
                    }
                    if app.show_uniques {
                        app.show_uniques = false;
                        app.focus = Focus::Inventory;
                    }
                    if app.show_add_affix {
                        app.show_add_affix = false;
                        app.focus = Focus::Inventory;
                    }
                    if app.show_currencies {
                        app.show_currencies = false;
                        app.focus = Focus::Inventory;
                    }
                    continue;
                }
                _ => {}
            }

            // Handle based on current focus
            match app.focus {
                Focus::Inventory => handle_inventory_keys(app, key.code),
                Focus::BaseTypes => handle_base_type_keys(app, key.code),
                Focus::Uniques => handle_unique_keys(app, key.code),
                Focus::Currencies => handle_currency_keys(app, key.code),
                Focus::AddAffix => handle_add_affix_keys(app, key.code),
            }
        }
    }
}

fn handle_inventory_keys(app: &mut App, code: KeyCode) {
    match code {
        KeyCode::Up | KeyCode::Char('k') => {
            if let Some(selected) = app.inventory_state.selected() {
                if selected > 0 {
                    app.inventory_state.select(Some(selected - 1));
                }
            }
        }
        KeyCode::Down | KeyCode::Char('j') => {
            if let Some(selected) = app.inventory_state.selected() {
                if selected < app.inventory.len().saturating_sub(1) {
                    app.inventory_state.select(Some(selected + 1));
                }
            } else if !app.inventory.is_empty() {
                app.inventory_state.select(Some(0));
            }
        }
        KeyCode::Char('d') | KeyCode::Delete => {
            app.delete_selected();
        }
        _ => {}
    }
}

fn handle_base_type_keys(app: &mut App, code: KeyCode) {
    match code {
        KeyCode::Up | KeyCode::Char('k') => {
            if let Some(selected) = app.base_type_state.selected() {
                if selected > 0 {
                    app.base_type_state.select(Some(selected - 1));
                }
            }
        }
        KeyCode::Down | KeyCode::Char('j') => {
            if let Some(selected) = app.base_type_state.selected() {
                if selected < app.base_type_ids.len().saturating_sub(1) {
                    app.base_type_state.select(Some(selected + 1));
                }
            }
        }
        KeyCode::Enter => {
            if let Some(idx) = app.base_type_state.selected() {
                if let Some(id) = app.base_type_ids.get(idx).cloned() {
                    app.generate_item(&id);
                    app.show_base_types = false;
                    app.focus = Focus::Inventory;
                }
            }
        }
        _ => {}
    }
}

fn handle_unique_keys(app: &mut App, code: KeyCode) {
    match code {
        KeyCode::Up | KeyCode::Char('k') => {
            if let Some(selected) = app.unique_state.selected() {
                if selected > 0 {
                    app.unique_state.select(Some(selected - 1));
                }
            }
        }
        KeyCode::Down | KeyCode::Char('j') => {
            if let Some(selected) = app.unique_state.selected() {
                if selected < app.unique_ids.len().saturating_sub(1) {
                    app.unique_state.select(Some(selected + 1));
                }
            }
        }
        KeyCode::Enter => {
            if let Some(idx) = app.unique_state.selected() {
                if let Some(id) = app.unique_ids.get(idx).cloned() {
                    app.generate_unique_item(&id);
                    app.show_uniques = false;
                    app.focus = Focus::Inventory;
                }
            }
        }
        _ => {}
    }
}

fn handle_currency_keys(app: &mut App, code: KeyCode) {
    match code {
        KeyCode::Up | KeyCode::Char('k') => {
            if let Some(selected) = app.currency_popup_state.list_state.selected() {
                if selected > 0 {
                    app.currency_popup_state.list_state.select(Some(selected - 1));
                }
            }
        }
        KeyCode::Down | KeyCode::Char('j') => {
            if let Some(selected) = app.currency_popup_state.list_state.selected() {
                if selected < app.currency_popup_state.currencies.len().saturating_sub(1) {
                    app.currency_popup_state.list_state.select(Some(selected + 1));
                }
            } else if !app.currency_popup_state.currencies.is_empty() {
                app.currency_popup_state.list_state.select(Some(0));
            }
        }
        KeyCode::Left | KeyCode::Char('h') => {
            app.prev_currency_category();
        }
        KeyCode::Right | KeyCode::Char('l') => {
            app.next_currency_category();
        }
        KeyCode::Tab => {
            app.next_currency_category();
        }
        KeyCode::BackTab => {
            app.prev_currency_category();
        }
        KeyCode::Enter => {
            app.apply_selected_currency();
        }
        _ => {}
    }
}

fn handle_add_affix_keys(app: &mut App, code: KeyCode) {
    match code {
        KeyCode::Up | KeyCode::Char('k') => {
            if app.add_affix_state.column == 0 {
                // Navigate affix list
                if let Some(selected) = app.add_affix_state.affix_state.selected() {
                    if selected > 0 {
                        app.add_affix_state.affix_state.select(Some(selected - 1));
                        app.update_affix_tiers();
                    }
                }
            } else {
                // Navigate tier list
                if let Some(selected) = app.add_affix_state.tier_state.selected() {
                    if selected > 0 {
                        app.add_affix_state.tier_state.select(Some(selected - 1));
                    }
                }
            }
        }
        KeyCode::Down | KeyCode::Char('j') => {
            if app.add_affix_state.column == 0 {
                // Navigate affix list
                if let Some(selected) = app.add_affix_state.affix_state.selected() {
                    if selected < app.add_affix_state.affixes.len().saturating_sub(1) {
                        app.add_affix_state.affix_state.select(Some(selected + 1));
                        app.update_affix_tiers();
                    }
                }
            } else {
                // Navigate tier list
                if let Some(selected) = app.add_affix_state.tier_state.selected() {
                    if selected < app.add_affix_state.tiers.len().saturating_sub(1) {
                        app.add_affix_state.tier_state.select(Some(selected + 1));
                    }
                }
            }
        }
        KeyCode::Left | KeyCode::Char('h') => {
            if app.add_affix_state.column > 0 {
                app.add_affix_state.column = 0;
            }
        }
        KeyCode::Right | KeyCode::Char('l') | KeyCode::Tab => {
            if app.add_affix_state.column == 0 {
                app.add_affix_state.column = 1;
            }
        }
        KeyCode::Enter => {
            app.add_selected_affix();
        }
        _ => {}
    }
}

fn ui(f: &mut Frame, app: &mut App) {
    // If there's a config error, show it instead of the normal UI
    if let Some(ref error) = app.config_error {
        render_config_error(f, error);
        return;
    }

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(3)])
        .split(f.area());

    let main_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
        .split(chunks[0]);

    // Inventory panel (full left side)
    render_inventory(f, app, main_chunks[0]);

    // Detail panel (right side)
    render_detail(f, app, main_chunks[1]);

    // Help bar
    render_help(f, app, chunks[1]);

    // Popups
    if app.show_base_types {
        render_base_type_popup(f, app);
    }

    if app.show_uniques {
        render_unique_popup(f, app);
    }

    if app.show_add_affix {
        render_add_affix_popup(f, app);
    }

    if app.show_currencies {
        render_currency_popup(f, app);
    }
}

fn render_config_error(f: &mut Frame, error: &ConfigError) {
    let area = f.area();

    // Create centered area for error display
    let error_area = centered_rect(80, 60, area);

    // Build error content
    let mut lines = vec![
        Line::from(vec![
            Span::styled("Configuration Error", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD))
        ]),
        Line::from(""),
    ];

    // Error type
    let error_type = match error {
        ConfigError::Io { .. } => "File I/O Error",
        ConfigError::Parse { .. } => "TOML Parse Error",
    };
    lines.push(Line::from(vec![
        Span::styled("Type: ", Style::default().fg(Color::Yellow)),
        Span::raw(error_type),
    ]));
    lines.push(Line::from(""));

    // File location
    lines.push(Line::from(vec![
        Span::styled("Location:", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
    ]));
    for line in error.location_description().lines() {
        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled(line.to_string(), Style::default().fg(Color::Cyan)),
        ]));
    }
    lines.push(Line::from(""));

    // Error message
    lines.push(Line::from(vec![
        Span::styled("Error:", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
    ]));
    for line in error.error_message().lines() {
        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::raw(line.to_string()),
        ]));
    }
    lines.push(Line::from(""));

    // Instructions
    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled("Press ", Style::default().fg(Color::DarkGray)),
        Span::styled("q", Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
        Span::styled(" to quit and fix the configuration file.", Style::default().fg(Color::DarkGray)),
    ]));

    let paragraph = Paragraph::new(lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Red))
                .title(" Config Error ")
                .title_style(Style::default().fg(Color::Red).add_modifier(Modifier::BOLD))
        )
        .wrap(Wrap { trim: false });

    f.render_widget(ratatui::widgets::Clear, error_area);
    f.render_widget(paragraph, error_area);
}

fn render_inventory(f: &mut Frame, app: &mut App, area: Rect) {
    let items: Vec<ListItem> = app
        .inventory
        .iter()
        .map(|(item, _)| {
            let rarity_color = match item.rarity {
                loot_core::Rarity::Normal => Color::White,
                loot_core::Rarity::Magic => Color::Blue,
                loot_core::Rarity::Rare => Color::Yellow,
                loot_core::Rarity::Unique => Color::Rgb(175, 95, 0),
            };
            ListItem::new(Line::from(vec![
                Span::styled(&item.name, Style::default().fg(rarity_color)),
                Span::raw(" "),
                Span::styled(
                    format!("({:?})", item.rarity),
                    Style::default().fg(Color::DarkGray),
                ),
            ]))
        })
        .collect();

    let border_style = if app.focus == Focus::Inventory {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default()
    };

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(border_style)
                .title(" Inventory "),
        )
        .highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("> ");

    f.render_stateful_widget(list, area, &mut app.inventory_state);
}

fn render_currency_popup(f: &mut Frame, app: &mut App) {
    let area = centered_rect(85, 80, f.area());

    // Clear the area
    f.render_widget(ratatui::widgets::Clear, area);

    // Split into left (currency list) and right (preview)
    let main_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    // Left side: tabs and currency list
    let left_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(0)])
        .split(main_chunks[0]);

    // Category tabs
    let tab_titles: Vec<&str> = app.currency_popup_state.categories
        .iter()
        .map(|s| s.as_str())
        .collect();

    let tabs = Tabs::new(tab_titles)
        .select(app.currency_popup_state.selected_category)
        .style(Style::default())
        .highlight_style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .divider("|");

    let tab_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan))
        .title(" ←/→ Tab | Enter: Apply | Esc: Close ");
    f.render_widget(tabs.block(tab_block), left_chunks[0]);

    // Currency list
    let items: Vec<ListItem> = app.currency_popup_state.currencies
        .iter()
        .map(|(_, name, _)| {
            ListItem::new(Span::styled(
                name,
                Style::default().add_modifier(Modifier::BOLD),
            ))
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan))
                .title(" Currencies "),
        )
        .highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("> ");

    f.render_stateful_widget(list, left_chunks[1], &mut app.currency_popup_state.list_state);

    // Right side: preview panel
    let preview_content = build_currency_preview(app);
    let preview = Paragraph::new(preview_content)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan))
                .title(" Preview "),
        )
        .wrap(Wrap { trim: false });

    f.render_widget(preview, main_chunks[1]);
}

fn build_currency_preview(app: &App) -> Text<'static> {
    let mut lines: Vec<Line> = Vec::new();

    // Get selected item
    let item = app.selected_item().map(|(item, _)| item);

    // Get selected currency
    let currency = app.get_selected_currency_id()
        .and_then(|id| app.generator().config().currencies.get(id));

    // Show current item header
    lines.push(Line::from(Span::styled(
        "Current Item".to_string(),
        Style::default().add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
    )));

    if let Some(item) = item {
        let rarity_color = match item.rarity {
            loot_core::Rarity::Normal => Color::White,
            loot_core::Rarity::Magic => Color::Blue,
            loot_core::Rarity::Rare => Color::Yellow,
            loot_core::Rarity::Unique => Color::Rgb(175, 95, 0),
        };
        lines.push(Line::from(vec![
            Span::styled(item.name.clone(), Style::default().fg(rarity_color)),
            Span::styled(
                format!(" ({:?})", item.rarity),
                Style::default().fg(Color::DarkGray),
            ),
        ]));
        lines.push(Line::from(Span::styled(
            item.base_name.clone(),
            Style::default().fg(Color::Gray),
        )));

        // Defenses
        if item.defenses.has_any() {
            if let Some(armour) = item.defenses.armour {
                lines.push(Line::from(Span::styled(
                    format!("  Armour: {}", armour),
                    Style::default().fg(Color::DarkGray),
                )));
            }
            if let Some(evasion) = item.defenses.evasion {
                lines.push(Line::from(Span::styled(
                    format!("  Evasion: {}", evasion),
                    Style::default().fg(Color::DarkGray),
                )));
            }
            if let Some(es) = item.defenses.energy_shield {
                lines.push(Line::from(Span::styled(
                    format!("  Energy Shield: {}", es),
                    Style::default().fg(Color::DarkGray),
                )));
            }
        }

        // Damage
        if let Some(ref dmg) = item.damage {
            for entry in &dmg.damages {
                let color = match entry.damage_type {
                    loot_core::types::DamageType::Physical => Color::White,
                    loot_core::types::DamageType::Fire => Color::Red,
                    loot_core::types::DamageType::Cold => Color::Cyan,
                    loot_core::types::DamageType::Lightning => Color::Yellow,
                    loot_core::types::DamageType::Chaos => Color::Magenta,
                };
                lines.push(Line::from(Span::styled(
                    format!("  {:?}: {}-{}", entry.damage_type, entry.min, entry.max),
                    Style::default().fg(color),
                )));
            }
        }

        // Implicit
        if let Some(ref imp) = item.implicit {
            lines.push(Line::from(Span::styled(
                format!("  {}", imp.display()),
                Style::default().fg(Color::Magenta),
            )));
        }

        // Affixes with change markers
        let changed = &app.changed_affixes;

        if !item.prefixes.is_empty() || !item.suffixes.is_empty() {
            lines.push(Line::from(Span::styled(
                format!("Mods ({}/{}P {}/{}S):",
                    item.prefixes.len(), item.rarity.max_prefixes(),
                    item.suffixes.len(), item.rarity.max_suffixes()),
                Style::default().fg(Color::Gray),
            )));

            for (i, prefix) in item.prefixes.iter().enumerate() {
                let marker = if changed.prefixes.contains(&i) {
                    Span::styled(">> ", Style::default().fg(Color::LightGreen))
                } else {
                    Span::raw("   ")
                };
                lines.push(Line::from(vec![
                    marker,
                    Span::styled(prefix.display(), Style::default().fg(Color::Cyan)),
                    Span::styled(format!(" [T{}]", prefix.tier), Style::default().fg(Color::DarkGray)),
                ]));
            }
            for (i, suffix) in item.suffixes.iter().enumerate() {
                let marker = if changed.suffixes.contains(&i) {
                    Span::styled(">> ", Style::default().fg(Color::LightGreen))
                } else {
                    Span::raw("   ")
                };
                lines.push(Line::from(vec![
                    marker,
                    Span::styled(suffix.display(), Style::default().fg(Color::Green)),
                    Span::styled(format!(" [T{}]", suffix.tier), Style::default().fg(Color::DarkGray)),
                ]));
            }
        } else {
            lines.push(Line::from(Span::styled(
                "  (no affixes)".to_string(),
                Style::default().fg(Color::DarkGray),
            )));
        }
    } else {
        lines.push(Line::from(Span::styled(
            "No item selected".to_string(),
            Style::default().fg(Color::DarkGray),
        )));
    }

    lines.push(Line::from(""));

    // Show currency details
    if let Some(currency) = currency {
        lines.push(Line::from(Span::styled(
            currency.name.clone(),
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        )));
        lines.push(Line::from(Span::styled(
            currency.description.clone(),
            Style::default().fg(Color::DarkGray),
        )));
        lines.push(Line::from(""));

        // Requirements
        lines.push(Line::from(Span::styled(
            "Requirements".to_string(),
            Style::default().add_modifier(Modifier::UNDERLINED),
        )));

        let reqs = &currency.requires;
        if !reqs.rarities.is_empty() {
            let rarity_str: Vec<String> = reqs.rarities.iter().map(|r| format!("{:?}", r)).collect();
            let meets_req = item.map(|i| reqs.rarities.contains(&i.rarity)).unwrap_or(false);
            let color = if meets_req { Color::Green } else { Color::Red };
            lines.push(Line::from(vec![
                Span::raw("  Rarity: ".to_string()),
                Span::styled(rarity_str.join(", "), Style::default().fg(color)),
            ]));
        }
        if reqs.has_affix {
            let meets_req = item.map(|i| !i.prefixes.is_empty() || !i.suffixes.is_empty()).unwrap_or(false);
            let color = if meets_req { Color::Green } else { Color::Red };
            lines.push(Line::from(Span::styled("  Must have affixes".to_string(), Style::default().fg(color))));
        }
        if reqs.has_affix_slot {
            let meets_req = item.map(|i| i.can_add_prefix() || i.can_add_suffix()).unwrap_or(false);
            // Check against target rarity if set_rarity is specified
            let meets_req = if !meets_req && currency.effects.set_rarity.is_some() {
                let target = currency.effects.set_rarity.unwrap();
                item.map(|i| {
                    let prefix_count = if currency.effects.clear_affixes { 0 } else { i.prefixes.len() };
                    let suffix_count = if currency.effects.clear_affixes { 0 } else { i.suffixes.len() };
                    prefix_count < target.max_prefixes() || suffix_count < target.max_suffixes()
                }).unwrap_or(false)
            } else {
                meets_req
            };
            let color = if meets_req { Color::Green } else { Color::Red };
            lines.push(Line::from(Span::styled("  Must have affix slot".to_string(), Style::default().fg(color))));
        }

        lines.push(Line::from(""));

        // Effects
        lines.push(Line::from(Span::styled(
            "Effects".to_string(),
            Style::default().add_modifier(Modifier::UNDERLINED),
        )));

        let effects = &currency.effects;
        if let Some(rarity) = effects.set_rarity {
            lines.push(Line::from(vec![
                Span::raw("  Set rarity: ".to_string()),
                Span::styled(format!("{:?}", rarity), Style::default().fg(Color::Yellow)),
            ]));
        }
        if effects.clear_affixes {
            lines.push(Line::from(Span::styled("  Clear all affixes".to_string(), Style::default().fg(Color::Red))));
        }
        if let Some(count) = effects.remove_affixes {
            lines.push(Line::from(Span::styled(
                format!("  Remove {} random affix(es)", count),
                Style::default().fg(Color::Red),
            )));
        }
        if let Some(count) = effects.reroll_affixes {
            lines.push(Line::from(Span::styled(
                format!("  Reroll {} random affix(es)", count),
                Style::default().fg(Color::Yellow),
            )));
        }
        if let Some(ref count) = effects.add_affixes {
            let range = if count.min == count.max {
                format!("{}", count.min)
            } else {
                format!("{}-{}", count.min, count.max)
            };
            lines.push(Line::from(Span::styled(
                format!("  Add {} random affix(es)", range),
                Style::default().fg(Color::Green),
            )));
        }
        if !effects.add_specific_affix.is_empty() {
            if effects.add_specific_affix.len() == 1 {
                let affix_id = &effects.add_specific_affix[0].id;
                let affix_name = app.generator().config().affixes.get(affix_id)
                    .map(|a| a.name.clone())
                    .unwrap_or_else(|| affix_id.clone());
                lines.push(Line::from(Span::styled(
                    format!("  Add: {}", affix_name),
                    Style::default().fg(Color::Green),
                )));
            } else {
                lines.push(Line::from(Span::styled(
                    format!("  Add one of {} affixes:", effects.add_specific_affix.len()),
                    Style::default().fg(Color::Green),
                )));
                for specific in &effects.add_specific_affix {
                    let affix_name = app.generator().config().affixes.get(&specific.id)
                        .map(|a| a.name.clone())
                        .unwrap_or_else(|| specific.id.clone());
                    let tier_str = specific.tier.map(|t| format!(" (T{})", t)).unwrap_or_default();
                    lines.push(Line::from(Span::styled(
                        format!("    - {}{}", affix_name, tier_str),
                        Style::default().fg(Color::DarkGray),
                    )));
                }
            }
        }
        if effects.try_unique {
            lines.push(Line::from(Span::styled(
                "  Transform to unique (if recipe matches)".to_string(),
                Style::default().fg(Color::Rgb(175, 95, 0)),
            )));
        }
    } else {
        lines.push(Line::from(Span::styled(
            "Select a currency".to_string(),
            Style::default().fg(Color::DarkGray),
        )));
    }

    Text::from(lines)
}

fn render_detail(f: &mut Frame, app: &App, area: Rect) {
    let tabs = Tabs::new(vec!["Stats", "Seed/Ops", "Recipes"])
        .select(match app.detail_tab {
            DetailTab::Stats => 0,
            DetailTab::Seed => 1,
            DetailTab::Recipes => 2,
        })
        .style(Style::default())
        .highlight_style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        );

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(0)])
        .split(area);

    let tab_block = Block::default()
        .borders(Borders::ALL)
        .title(" Item Detail ");
    f.render_widget(tabs.block(tab_block), chunks[0]);

    let content = match app.detail_tab {
        DetailTab::Stats => {
            if let Some((item, _)) = app.selected_item() {
                render_item_stats(item, &app.changed_affixes, app.generator())
            } else {
                Text::from("No item selected\n\nPress 'n' to create a new item")
            }
        }
        DetailTab::Seed => {
            if let Some((_, stored)) = app.selected_item() {
                render_item_seed(stored)
            } else {
                Text::from("No item selected\n\nPress 'n' to create a new item")
            }
        }
        DetailTab::Recipes => render_recipes(app.generator()),
    };

    let paragraph = Paragraph::new(content)
        .block(Block::default().borders(Borders::ALL))
        .wrap(Wrap { trim: false });

    f.render_widget(paragraph, chunks[1]);
}

fn render_item_stats(item: &Item, changed: &ChangedAffixes, generator: &Generator) -> Text<'static> {
    let mut lines: Vec<Line> = Vec::new();

    // Header
    let rarity_color = match item.rarity {
        loot_core::Rarity::Normal => Color::White,
        loot_core::Rarity::Magic => Color::Blue,
        loot_core::Rarity::Rare => Color::Yellow,
        loot_core::Rarity::Unique => Color::Rgb(175, 95, 0),
    };

    lines.push(Line::from(vec![Span::styled(
        item.name.clone(),
        Style::default()
            .fg(rarity_color)
            .add_modifier(Modifier::BOLD),
    )]));
    lines.push(Line::from(vec![
        Span::styled(item.base_name.clone(), Style::default().fg(Color::Gray)),
        Span::raw(" "),
        Span::styled(
            format!("({:?})", item.rarity),
            Style::default().fg(Color::DarkGray),
        ),
    ]));

    // Item tags
    if !item.tags.is_empty() {
        let tag_spans: Vec<Span> = item.tags.iter().enumerate().flat_map(|(i, tag)| {
            let mut spans = vec![Span::styled(tag.clone(), Style::default().fg(Color::Cyan))];
            if i < item.tags.len() - 1 {
                spans.push(Span::styled(", ", Style::default().fg(Color::DarkGray)));
            }
            spans
        }).collect();
        let mut line_spans = vec![Span::styled("Tags: ", Style::default().fg(Color::DarkGray))];
        line_spans.extend(tag_spans);
        lines.push(Line::from(line_spans));
    }

    lines.push(Line::from(""));

    // Defenses
    if item.defenses.has_any() {
        lines.push(Line::from(Span::styled(
            "Defenses".to_string(),
            Style::default().add_modifier(Modifier::UNDERLINED),
        )));
        if let Some(armour) = item.defenses.armour {
            lines.push(Line::from(format!("  Armour: {}", armour)));
        }
        if let Some(evasion) = item.defenses.evasion {
            lines.push(Line::from(format!("  Evasion: {}", evasion)));
        }
        if let Some(es) = item.defenses.energy_shield {
            lines.push(Line::from(format!("  Energy Shield: {}", es)));
        }
        lines.push(Line::from(""));
    }

    // Damage
    if let Some(ref dmg) = item.damage {
        lines.push(Line::from(Span::styled(
            "Damage".to_string(),
            Style::default().add_modifier(Modifier::UNDERLINED),
        )));
        for entry in &dmg.damages {
            let color = match entry.damage_type {
                loot_core::types::DamageType::Physical => Color::White,
                loot_core::types::DamageType::Fire => Color::Red,
                loot_core::types::DamageType::Cold => Color::Cyan,
                loot_core::types::DamageType::Lightning => Color::Yellow,
                loot_core::types::DamageType::Chaos => Color::Magenta,
            };
            lines.push(Line::from(Span::styled(
                format!("  {:?}: {}-{}", entry.damage_type, entry.min, entry.max),
                Style::default().fg(color),
            )));
        }
        if dmg.attack_speed > 0.0 {
            lines.push(Line::from(format!(
                "  Attack Speed: {:.2}",
                dmg.attack_speed
            )));
        }
        if dmg.critical_chance > 0.0 {
            lines.push(Line::from(format!(
                "  Crit Chance: {:.1}%",
                dmg.critical_chance
            )));
        }
        if dmg.spell_efficiency > 0.0 {
            lines.push(Line::from(format!(
                "  Spell Efficiency: {:.0}%",
                dmg.spell_efficiency
            )));
        }
        lines.push(Line::from(""));
    }

    // Implicit
    if let Some(ref imp) = item.implicit {
        lines.push(Line::from(Span::styled(
            "Implicit".to_string(),
            Style::default().add_modifier(Modifier::UNDERLINED),
        )));
        lines.push(Line::from(vec![
            Span::styled(
                format!("  {} ", imp.display()),
                Style::default().fg(Color::Magenta),
            ),
            Span::styled(
                format!("({}-{})", imp.tier_min, imp.tier_max),
                Style::default().fg(Color::DarkGray),
            ),
        ]));
        lines.push(Line::from(""));
    }

    // Explicit mods
    if !item.prefixes.is_empty() || !item.suffixes.is_empty() {
        lines.push(Line::from(Span::styled(
            "Modifiers".to_string(),
            Style::default().add_modifier(Modifier::UNDERLINED),
        )));
        for (i, prefix) in item.prefixes.iter().enumerate() {
            let marker = if changed.prefixes.contains(&i) {
                Span::styled(">> ", Style::default().fg(Color::LightRed))
            } else {
                Span::raw("   ")
            };
            let tier_range = if let Some((max_min, max_max)) = prefix.tier_max_value {
                format!("({}-{} to {}-{}) ", prefix.tier_min, prefix.tier_max, max_min, max_max)
            } else {
                format!("({}-{}) ", prefix.tier_min, prefix.tier_max)
            };
            lines.push(Line::from(vec![
                marker,
                Span::styled(
                    format!("{} ", prefix.display()),
                    Style::default().fg(Color::Cyan),
                ),
                Span::styled(
                    format!("[T{}] ", prefix.tier),
                    Style::default().fg(Color::Yellow),
                ),
                Span::styled(
                    tier_range,
                    Style::default().fg(Color::DarkGray),
                ),
                Span::styled("P".to_string(), Style::default().fg(Color::DarkGray)),
            ]));
            // Show affix scope and tags
            if let Some(affix_config) = generator.config().affixes.get(&prefix.affix_id) {
                let scope_color = match affix_config.scope {
                    loot_core::types::AffixScope::Local => Color::Blue,
                    loot_core::types::AffixScope::Global => Color::Magenta,
                };
                let mut info_parts = vec![
                    Span::raw("      "),
                    Span::styled(format!("{:?}", affix_config.scope), Style::default().fg(scope_color)),
                ];
                if !affix_config.tags.is_empty() {
                    let tags_str = affix_config.tags.join(", ");
                    info_parts.push(Span::styled(format!(" | tags: {}", tags_str), Style::default().fg(Color::DarkGray)));
                }
                lines.push(Line::from(info_parts));
            }
        }
        for (i, suffix) in item.suffixes.iter().enumerate() {
            let marker = if changed.suffixes.contains(&i) {
                Span::styled(">> ", Style::default().fg(Color::LightRed))
            } else {
                Span::raw("   ")
            };
            let tier_range = if let Some((max_min, max_max)) = suffix.tier_max_value {
                format!("({}-{} to {}-{}) ", suffix.tier_min, suffix.tier_max, max_min, max_max)
            } else {
                format!("({}-{}) ", suffix.tier_min, suffix.tier_max)
            };
            lines.push(Line::from(vec![
                marker,
                Span::styled(
                    format!("{} ", suffix.display()),
                    Style::default().fg(Color::Green),
                ),
                Span::styled(
                    format!("[T{}] ", suffix.tier),
                    Style::default().fg(Color::Yellow),
                ),
                Span::styled(
                    tier_range,
                    Style::default().fg(Color::DarkGray),
                ),
                Span::styled("S".to_string(), Style::default().fg(Color::DarkGray)),
            ]));
            // Show affix scope and tags
            if let Some(affix_config) = generator.config().affixes.get(&suffix.affix_id) {
                let scope_color = match affix_config.scope {
                    loot_core::types::AffixScope::Local => Color::Blue,
                    loot_core::types::AffixScope::Global => Color::Magenta,
                };
                let mut info_parts = vec![
                    Span::raw("      "),
                    Span::styled(format!("{:?}", affix_config.scope), Style::default().fg(scope_color)),
                ];
                if !affix_config.tags.is_empty() {
                    let tags_str = affix_config.tags.join(", ");
                    info_parts.push(Span::styled(format!(" | tags: {}", tags_str), Style::default().fg(Color::DarkGray)));
                }
                lines.push(Line::from(info_parts));
            }
        }
        lines.push(Line::from(""));
    }

    // Requirements
    let mut reqs = Vec::new();
    if item.requirements.level > 0 {
        reqs.push(format!("Level {}", item.requirements.level));
    }
    if item.requirements.strength > 0 {
        reqs.push(format!("{} Str", item.requirements.strength));
    }
    if item.requirements.dexterity > 0 {
        reqs.push(format!("{} Dex", item.requirements.dexterity));
    }
    if item.requirements.intelligence > 0 {
        reqs.push(format!("{} Int", item.requirements.intelligence));
    }
    if !reqs.is_empty() {
        lines.push(Line::from(Span::styled(
            format!("Requires: {}", reqs.join(", ")),
            Style::default().fg(Color::DarkGray),
        )));
    }

    Text::from(lines)
}

fn render_item_seed(stored: &StoredItem) -> Text<'static> {
    let mut lines: Vec<Line> = Vec::new();

    lines.push(Line::from(Span::styled(
        "Seed Data".to_string(),
        Style::default().add_modifier(Modifier::BOLD),
    )));
    lines.push(Line::from(""));

    lines.push(Line::from(vec![
        Span::styled("Base Type: ".to_string(), Style::default().fg(Color::Gray)),
        Span::raw(stored.base_type_id.clone()),
    ]));

    lines.push(Line::from(vec![
        Span::styled("Seed: ".to_string(), Style::default().fg(Color::Gray)),
        Span::styled(
            format!("0x{:016X}", stored.seed),
            Style::default().fg(Color::Yellow),
        ),
    ]));

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "Operations".to_string(),
        Style::default().add_modifier(Modifier::BOLD),
    )));

    if stored.operations.is_empty() {
        lines.push(Line::from(Span::styled(
            "  (none)".to_string(),
            Style::default().fg(Color::DarkGray),
        )));
    } else {
        for (i, op) in stored.operations.iter().enumerate() {
            let op_str = match op {
                Operation::Currency(action) => format!("{:?}", action),
            };
            lines.push(Line::from(vec![
                Span::styled(
                    format!("  {}. ", i + 1),
                    Style::default().fg(Color::DarkGray),
                ),
                Span::styled(op_str, Style::default().fg(Color::Cyan)),
            ]));
        }
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "JSON Export".to_string(),
        Style::default().add_modifier(Modifier::BOLD),
    )));
    lines.push(Line::from(""));

    if let Ok(json) = stored.to_json() {
        for line in json.lines() {
            lines.push(Line::from(Span::styled(
                line.to_string(),
                Style::default().fg(Color::Green),
            )));
        }
    }

    Text::from(lines)
}

fn render_recipes(generator: &Generator) -> Text<'static> {
    let mut lines: Vec<Line> = Vec::new();

    lines.push(Line::from(Span::styled(
        "Unique Recipes".to_string(),
        Style::default().add_modifier(Modifier::BOLD),
    )));
    lines.push(Line::from(Span::styled(
        "Use a transform_unique currency on matching items".to_string(),
        Style::default().fg(Color::DarkGray),
    )));
    lines.push(Line::from(""));

    let recipes = &generator.config().unique_recipes;

    if recipes.is_empty() {
        lines.push(Line::from(Span::styled(
            "No recipes configured".to_string(),
            Style::default().fg(Color::DarkGray),
        )));
    } else {
        for recipe in recipes {
            // Get unique name
            let unique_name = generator
                .get_unique(&recipe.unique_id)
                .map(|u| u.name.clone())
                .unwrap_or_else(|| recipe.unique_id.clone());

            // Get base type name
            let base_name = generator
                .get_base_type(&recipe.base_type)
                .map(|b| b.name.clone())
                .unwrap_or_else(|| recipe.base_type.clone());

            // Recipe header
            lines.push(Line::from(vec![
                Span::styled(
                    unique_name,
                    Style::default()
                        .fg(Color::Rgb(175, 95, 0))
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    format!(" (weight: {})", recipe.weight),
                    Style::default().fg(Color::DarkGray),
                ),
            ]));

            // Base type requirement
            lines.push(Line::from(vec![
                Span::styled("  Base: ".to_string(), Style::default().fg(Color::Gray)),
                Span::styled(base_name, Style::default().fg(Color::White)),
            ]));

            // Required affixes
            lines.push(Line::from(Span::styled(
                "  Required Affixes:".to_string(),
                Style::default().fg(Color::Gray),
            )));

            for req in &recipe.required_affixes {
                let affix_type_str = match req.affix_type {
                    Some(loot_core::AffixType::Prefix) => " (Prefix)",
                    Some(loot_core::AffixType::Suffix) => " (Suffix)",
                    None => "",
                };

                let tier_str = if req.min_tier == 1 && req.max_tier == 99 {
                    "any tier".to_string()
                } else if req.min_tier == req.max_tier {
                    format!("T{}", req.min_tier)
                } else {
                    format!("T{}-T{}", req.min_tier, req.max_tier)
                };

                // Check if this required affix has a mapping
                let has_mapping = recipe.mappings.iter().any(|m| m.from_stat == req.stat);
                let mapping_indicator = if has_mapping {
                    Span::styled(" → mapped", Style::default().fg(Color::Green))
                } else {
                    Span::styled(" (gate only)", Style::default().fg(Color::DarkGray))
                };

                lines.push(Line::from(vec![
                    Span::raw("    • "),
                    Span::styled(
                        format!("{:?}", req.stat),
                        Style::default().fg(Color::Cyan),
                    ),
                    Span::styled(affix_type_str.to_string(), Style::default().fg(Color::Yellow)),
                    Span::styled(format!(" [{}]", tier_str), Style::default().fg(Color::DarkGray)),
                    mapping_indicator,
                ]));
            }

            // Show unique mods and their mappings
            if let Some(unique) = generator.get_unique(&recipe.unique_id) {
                lines.push(Line::from(Span::styled(
                    "  Unique Mods:".to_string(),
                    Style::default().fg(Color::Gray),
                )));

                for (mod_idx, mod_cfg) in unique.mods.iter().enumerate() {
                    // Check if this mod has a mapping
                    let mapping = recipe.mappings.iter().find(|m| m.to_mod_index == mod_idx);

                    let mut spans = vec![
                        Span::raw("    "),
                        Span::styled(
                            format!("#{} {:?}", mod_idx + 1, mod_cfg.stat),
                            Style::default().fg(Color::Magenta),
                        ),
                        Span::styled(
                            format!(" ({}-{})", mod_cfg.min, mod_cfg.max),
                            Style::default().fg(Color::DarkGray),
                        ),
                    ];

                    if let Some(mapping) = mapping {
                        let mode_str = match mapping.mode {
                            MappingMode::Percentage => "pct",
                            MappingMode::Direct => "direct",
                            MappingMode::Random => "random",
                        };

                        let influence_str = if mapping.mode == MappingMode::Random {
                            String::new()
                        } else if mapping.influence >= 1.0 {
                            String::new()
                        } else {
                            format!(" {:.0}%", mapping.influence * 100.0)
                        };

                        spans.push(Span::styled(
                            format!(" ← {:?}", mapping.from_stat),
                            Style::default().fg(Color::Green),
                        ));
                        spans.push(Span::styled(
                            format!(" [{}{}]", mode_str, influence_str),
                            Style::default().fg(Color::Yellow),
                        ));
                    } else {
                        spans.push(Span::styled(
                            " (random)".to_string(),
                            Style::default().fg(Color::DarkGray),
                        ));
                    }

                    lines.push(Line::from(spans));
                }
            }

            lines.push(Line::from(""));
        }
    }

    Text::from(lines)
}

fn render_help(f: &mut Frame, app: &App, area: Rect) {
    let help_text = if let Some(ref msg) = app.message {
        Span::styled(msg.clone(), Style::default().fg(Color::Yellow))
    } else {
        Span::raw("n: New | U: Unique | c: Currency | A: Add Affix | Tab: Detail | d: Delete | q: Quit")
    };

    let help = Paragraph::new(Line::from(help_text))
        .block(Block::default().borders(Borders::ALL).title(" Help "));

    f.render_widget(help, area);
}

fn render_base_type_popup(f: &mut Frame, app: &mut App) {
    let area = centered_rect(50, 60, f.area());

    // Clear the area
    f.render_widget(ratatui::widgets::Clear, area);

    // Collect base type data first to avoid borrow conflicts
    let generator = app.generator();
    let item_data: Vec<(String, String)> = app
        .base_type_ids
        .iter()
        .filter_map(|id| {
            generator.get_base_type(id).map(|base| {
                (base.name.clone(), format!("{:?}", base.class))
            })
        })
        .collect();

    let items: Vec<ListItem> = item_data
        .iter()
        .map(|(name, class)| {
            ListItem::new(Line::from(vec![
                Span::raw(name.as_str()),
                Span::styled(
                    format!(" ({})", class),
                    Style::default().fg(Color::DarkGray),
                ),
            ]))
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan))
                .title(" Select Base Type (Enter to confirm, Esc to cancel) "),
        )
        .highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("> ");

    f.render_stateful_widget(list, area, &mut app.base_type_state);
}

fn render_unique_popup(f: &mut Frame, app: &mut App) {
    let area = centered_rect(50, 60, f.area());

    // Clear the area
    f.render_widget(ratatui::widgets::Clear, area);

    // Collect unique data first to avoid borrow conflicts
    let generator = app.generator();
    let item_data: Vec<(String, String)> = app
        .unique_ids
        .iter()
        .filter_map(|id| {
            generator.get_unique(id).map(|unique| {
                let base_name = generator
                    .get_base_type(&unique.base_type)
                    .map(|b| b.name.clone())
                    .unwrap_or_else(|| "???".to_string());
                (unique.name.clone(), base_name)
            })
        })
        .collect();

    let items: Vec<ListItem> = item_data
        .iter()
        .map(|(name, base_name)| {
            ListItem::new(Line::from(vec![
                Span::styled(
                    name.as_str(),
                    Style::default().fg(Color::Rgb(175, 95, 0)), // Orange for uniques
                ),
                Span::styled(
                    format!(" ({})", base_name),
                    Style::default().fg(Color::DarkGray),
                ),
            ]))
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Rgb(175, 95, 0)))
                .title(" Select Unique (Enter to confirm, Esc to cancel) "),
        )
        .highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("> ");

    f.render_stateful_widget(list, area, &mut app.unique_state);
}

fn render_add_affix_popup(f: &mut Frame, app: &mut App) {
    let area = centered_rect(70, 70, f.area());

    // Clear the area
    f.render_widget(ratatui::widgets::Clear, area);

    // Split into two columns: affixes and tiers
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
        .split(area);

    // Affix list
    let affix_items: Vec<ListItem> = app
        .add_affix_state
        .affixes
        .iter()
        .map(|(_, name, affix_type)| {
            let type_char = match affix_type {
                loot_core::AffixType::Prefix => "P",
                loot_core::AffixType::Suffix => "S",
            };
            let type_color = match affix_type {
                loot_core::AffixType::Prefix => Color::Cyan,
                loot_core::AffixType::Suffix => Color::Green,
            };
            ListItem::new(Line::from(vec![
                Span::styled(format!("[{}] ", type_char), Style::default().fg(type_color)),
                Span::raw(name),
            ]))
        })
        .collect();

    let affix_border_color = if app.add_affix_state.column == 0 {
        Color::Cyan
    } else {
        Color::DarkGray
    };

    let affix_list = List::new(affix_items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(affix_border_color))
                .title(" Select Affix (←→ to switch, Enter to add) "),
        )
        .highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("> ");

    f.render_stateful_widget(affix_list, chunks[0], &mut app.add_affix_state.affix_state);

    // Tier list
    let tier_items: Vec<ListItem> = app
        .add_affix_state
        .tiers
        .iter()
        .map(|(tier, min, max, max_value)| {
            let range_str = if let Some((max_min, max_max)) = max_value {
                format!(" ({}-{} to {}-{})", min, max, max_min, max_max)
            } else {
                format!(" ({}-{})", min, max)
            };
            ListItem::new(Line::from(vec![
                Span::styled(format!("T{}", tier), Style::default().fg(Color::Yellow)),
                Span::styled(range_str, Style::default().fg(Color::DarkGray)),
            ]))
        })
        .collect();

    let tier_border_color = if app.add_affix_state.column == 1 {
        Color::Cyan
    } else {
        Color::DarkGray
    };

    let tier_list = List::new(tier_items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(tier_border_color))
                .title(" Select Tier "),
        )
        .highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("> ");

    f.render_stateful_widget(tier_list, chunks[1], &mut app.add_affix_state.tier_state);
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
