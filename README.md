# Loot Generator

A configurable loot generation system for video games, inspired by Path of Exile's itemization. Features a data-driven currency system, affix pools, seed-based determinism, and unique item crafting.

## Features

- **Data-Driven Design** - All items, affixes, currencies, and pools defined in TOML
- **Generic Currency System** - Currencies are configured, not hardcoded
- **Affix Pools** - Group affixes into pools for specialized crafting
- **Seed-Based Determinism** - Reproducible item generation from compact seeds
- **Unique Item Crafting** - Transform items into uniques based on affix requirements
- **Tiered Affixes** - Multi-tier affixes with weighted roll ranges

## Architecture

```
loot_generator/
├── loot_core/          # Library crate - all generation logic
├── loot_tui/           # Binary crate - terminal UI for experimentation
└── config/             # TOML configuration files
    ├── base_types/     # Item base definitions
    ├── affixes/        # Affix definitions with tiers
    ├── affix_pools/    # Affix pool groupings
    ├── currencies/     # Currency definitions
    └── uniques/        # Unique item templates
```

## Library Usage

### Basic Setup

```rust
use loot_core::{Config, Generator, Item, Rarity};
use loot_core::currency::{apply_currency, CurrencyError};
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load configuration from directory
    let config = Config::load_from_dir(Path::new("config"))?;

    // Create generator
    let generator = Generator::new(config);

    // Create seeded RNG for deterministic results
    let mut rng = Generator::make_rng(12345);

    Ok(())
}
```

### Generating Items

```rust
// Generate a normal (white) item
let mut item = generator.generate_normal("iron_sword", &mut rng)
    .expect("Base type not found");

println!("Created: {} ({:?})", item.name, item.rarity);
// Output: Created: Iron Sword (Normal)

// Generate a unique item directly
let unique = generator.generate_unique("starforge", &mut rng)
    .expect("Unique not found");
```

### Applying Currencies

```rust
// Get a currency configuration
let transmute = generator.config().currencies.get("upgrade_to_magic")
    .expect("Currency not found");

// Apply currency to item
match apply_currency(&generator, &mut item, transmute, &mut rng) {
    Ok(()) => println!("Item is now {:?}", item.rarity),
    Err(CurrencyError::InvalidRarity { expected, got }) => {
        println!("Cannot use on {:?} items", got);
    }
    Err(e) => println!("Failed: {}", e),
}
```

### Checking Currency Applicability

```rust
// Check if currency can be applied before trying
fn can_apply_currency(item: &Item, currency: &CurrencyConfig) -> bool {
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
```

### Working with Item Stats

```rust
// Access item properties
println!("Base: {}", item.base_name);
println!("Rarity: {:?}", item.rarity);
println!("Class: {:?}", item.class);

// Access modifiers
if let Some(implicit) = &item.implicit {
    println!("Implicit: {:?} +{}", implicit.stat, implicit.value);
}

for prefix in &item.prefixes {
    println!("Prefix: {} (T{}) +{}", prefix.name, prefix.tier, prefix.value);
}

for suffix in &item.suffixes {
    println!("Suffix: {} (T{}) +{}", suffix.name, suffix.tier, suffix.value);
}

// Check affix capacity
println!("Can add prefix: {}", item.can_add_prefix());
println!("Can add suffix: {}", item.can_add_suffix());
```

### Seed-Based Storage

```rust
use loot_core::StoredItem;

// Store item compactly
let stored = StoredItem {
    base_type_id: "iron_sword".to_string(),
    seed: 12345,
    operations: vec!["upgrade_to_magic".to_string(), "add_affix".to_string()],
};

// Reconstruct item from stored representation
fn reconstruct_item(
    generator: &Generator,
    stored: &StoredItem,
) -> Option<Item> {
    let mut rng = Generator::make_rng(stored.seed);
    let mut item = generator.generate_normal(&stored.base_type_id, &mut rng)?;

    for op_id in &stored.operations {
        if let Some(currency) = generator.config().currencies.get(op_id) {
            let _ = apply_currency(generator, &mut item, currency, &mut rng);
        }
    }

    Some(item)
}
```

### Listing Available Content

```rust
// List all base types
for id in generator.base_type_ids() {
    let base = generator.get_base_type(id).unwrap();
    println!("{}: {} ({:?})", id, base.name, base.class);
}

// List all currencies by category
let mut by_category: HashMap<String, Vec<&CurrencyConfig>> = HashMap::new();
for currency in generator.config().currencies.values() {
    by_category.entry(currency.category.clone())
        .or_default()
        .push(currency);
}

for (category, currencies) in by_category {
    println!("{}:", category);
    for c in currencies {
        println!("  {} - {}", c.name, c.description);
    }
}

// List affix pools
for (id, pool) in &generator.config().affix_pools {
    println!("{}: {} affixes", id, pool.affixes.len());
}
```

## Configuration

### Directory Structure

```
config/
├── base_types/
│   ├── weapons.toml
│   └── armour.toml
├── affixes/
│   ├── weapon.toml
│   ├── armour.toml
│   └── poison.toml      # Special affixes
├── affix_pools/
│   ├── default.toml     # common, elemental, physical, defense pools
│   └── poison.toml      # Exclusive poison pool
├── currencies/
│   ├── rarity.toml      # Rarity-changing currencies
│   ├── affix_modification.toml
│   ├── specific_affixes.toml
│   ├── imbue.toml       # Normal->magic with specific affix
│   └── poison.toml      # Poison pool currencies
└── uniques/
    └── starforge.toml
```

### Base Types

```toml
# config/base_types/weapons.toml
[[base_types]]
id = "iron_sword"
name = "Iron Sword"
class = "one_hand_sword"
tags = ["melee", "physical", "attack"]

[base_types.implicit]
stat = "added_accuracy"
min = 8
max = 12

[base_types.damage]
min = 5
max = 15
attack_speed = 1.3
critical_chance = 5.0

[base_types.requirements]
level = 1
strength = 10
```

### Affixes

```toml
# config/affixes/weapon.toml
[[affixes]]
id = "phys_damage_flat"
name = "Heavy"
type = "prefix"
stat = "added_physical_damage"
tags = ["physical", "damage", "attack"]
allowed_classes = ["one_hand_sword", "two_hand_sword", "dagger"]

[[affixes.tiers]]
tier = 1
weight = 100    # Rarer
min = 25
max = 35

[[affixes.tiers]]
tier = 2
weight = 300
min = 18
max = 24

[[affixes.tiers]]
tier = 3
weight = 600
min = 10
max = 17

[[affixes.tiers]]
tier = 4
weight = 1000   # Most common
min = 4
max = 9
```

### Affix Pools

Pools group affixes for specialized currencies. Currencies must specify which pools they draw from.

```toml
# config/affix_pools/default.toml
[[pools]]
id = "common"
name = "Common Affixes"
description = "All standard affixes"
affixes = [
    "phys_damage_flat",
    "fire_damage_flat",
    "cold_damage_flat",
    "life_flat",
    "fire_resist",
    # ... all standard affixes
]

[[pools]]
id = "elemental"
name = "Elemental Affixes"
affixes = ["fire_damage_flat", "cold_damage_flat", "lightning_damage_flat"]

# config/affix_pools/poison.toml
[[pools]]
id = "poison"
name = "Poison Affixes"
description = "Exclusive poison affixes"
affixes = [
    "poison_damage_flat",
    "chance_to_poison",
    "poison_duration",
]
```

### Currencies

Currencies are fully data-driven with requirements and effects.

```toml
# config/currencies/rarity.toml
[[currencies]]
id = "upgrade_to_magic"
name = "Magic Essence"
description = "Upgrades a normal item to magic"
category = "Rarity"

[currencies.requires]
rarities = ["normal"]          # Must be normal rarity

[currencies.effects]
set_rarity = "magic"           # Change to magic
clear_affixes = true           # Remove existing affixes
add_affixes = { min = 1, max = 1 }  # Add 1 random affix
affix_pools = ["common"]       # Draw from common pool
```

#### Currency Requirements

| Field | Type | Description |
|-------|------|-------------|
| `rarities` | `[Rarity]` | Item must be one of these rarities |
| `has_affix` | `bool` | Item must have at least one affix |
| `has_affix_slot` | `bool` | Item must have room for another affix |

#### Currency Effects

| Field | Type | Description |
|-------|------|-------------|
| `set_rarity` | `Rarity` | Change item's rarity |
| `clear_affixes` | `bool` | Remove all existing affixes |
| `add_affixes` | `{min, max}` | Add random affixes from pools |
| `add_specific_affix` | `[{id, tier?, weight?}]` | Add specific affix(es) |
| `remove_affixes` | `u32` | Remove N random affixes |
| `reroll_affixes` | `u32` | Reroll N random affixes |
| `affix_pools` | `[String]` | Pools to draw random affixes from (required) |
| `try_unique` | `bool` | Attempt unique transformation |

#### Specific Affix Selection

Add one affix from a weighted set:

```toml
[[currencies]]
id = "elemental_enchant"
name = "Prismatic Crystal"
description = "Adds random elemental damage"
category = "Elemental"

[currencies.requires]
rarities = ["magic", "rare"]
has_affix_slot = true

[currencies.effects]
add_specific_affix = [
    { id = "fire_damage_flat", weight = 100 },
    { id = "cold_damage_flat", weight = 100 },
    { id = "lightning_damage_flat", weight = 100 },
]
```

#### Guaranteed Tier

Force a specific tier:

```toml
[currencies.effects]
add_specific_affix = [{ id = "life_flat", tier = 1 }]  # Always T1
```

#### Imbue Currencies

Upgrade normal to magic with a specific affix:

```toml
[[currencies]]
id = "imbue_fire"
name = "Ember Imbue"
description = "Imbues a normal item with fire damage"
category = "Imbue"

[currencies.requires]
rarities = ["normal"]

[currencies.effects]
set_rarity = "magic"
add_specific_affix = [{ id = "fire_damage_flat" }]
```

### Unique Items

```toml
# config/uniques/starforge.toml
[unique]
id = "starforge"
name = "Starforge"
base_type = "infernal_sword"
flavor = "The cosmos burns within."

[[unique.mods]]
stat = "increased_physical_damage"
min = 400
max = 500

[[unique.mods]]
stat = "added_physical_damage"
min = 50
max = 100

# Optional: Recipe for crafting this unique
[recipe]
weight = 100

[[recipe.required_affixes]]
stat = "increased_physical_damage"
min_tier = 1
max_tier = 2

[[recipe.mappings]]
from_stat = "increased_physical_damage"
to_mod_index = 0
mode = "percentage"
influence = 0.8
```

## Item Structure

### Rarities

| Rarity | Prefixes | Suffixes | Description |
|--------|----------|----------|-------------|
| Normal | 0 | 0 | No affixes |
| Magic | 0-1 | 0-1 | Up to 2 total |
| Rare | 0-3 | 0-3 | Up to 6 total, random name |
| Unique | - | - | Fixed mods, fixed name |

### Item Classes

**Weapons:** `one_hand_sword`, `two_hand_sword`, `one_hand_axe`, `two_hand_axe`, `one_hand_mace`, `two_hand_mace`, `dagger`, `claw`, `bow`, `wand`, `staff`

**Armour:** `helmet`, `body_armour`, `gloves`, `boots`, `shield`

### Stat Types

Flat additions: `added_physical_damage`, `added_fire_damage`, `added_cold_damage`, `added_lightning_damage`, `added_chaos_damage`, `added_life`, `added_accuracy`, etc.

Percentage increases: `increased_physical_damage`, `increased_attack_speed`, `increased_critical_chance`, `increased_armour`, `increased_movement_speed`, etc.

Resistances: `fire_resistance`, `cold_resistance`, `lightning_resistance`, `chaos_resistance`

Poison/Ailment: `poison_damage_over_time`, `chance_to_poison`, `increased_poison_duration`

## TUI Application

Interactive terminal interface for testing:

```bash
cargo run -p loot_tui
```

### Keybindings

| Key | Action |
|-----|--------|
| `n` | New item (opens base type selector) |
| `c` | Toggle currency popup |
| `j/k` or `↑/↓` | Navigate lists |
| `Enter` | Apply selected currency |
| `d` | Delete selected item |
| `q` | Quit |

### Currency Popup

Press `c` to open the currency popup:
- Left side: Categories and currencies
- Right side: Current item stats with change markers (`>>`)
- The popup stays open for chaining multiple currencies

## Error Handling

```rust
use loot_core::currency::CurrencyError;

match apply_currency(&generator, &mut item, currency, &mut rng) {
    Ok(()) => { /* Success */ }
    Err(CurrencyError::InvalidRarity { expected, got }) => {
        // Item rarity doesn't match requirements
    }
    Err(CurrencyError::NoAffixSlots) => {
        // Item has no room for more affixes
    }
    Err(CurrencyError::NoAffixesToRemove) => {
        // Item has no affixes to remove/reroll
    }
    Err(CurrencyError::NoValidAffixes) => {
        // No valid affixes for this item (class restrictions, etc.)
    }
    Err(CurrencyError::NoAffixPoolsSpecified) => {
        // Currency config missing affix_pools
    }
    Err(CurrencyError::NoMatchingRecipe) => {
        // No unique recipe matches (for try_unique)
    }
    Err(e) => {
        // Other errors: AffixNotFound, TierNotFound, etc.
    }
}
```

## License

MIT
