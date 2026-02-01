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
tags = ["melee", "physical", "attack", "sword"]

[base_types.implicit]
stat = "added_accuracy"
min = 10
max = 20

[base_types.damage]
attack_speed = 1.3
critical_chance = 5.0
spell_efficiency = 0.0     # 0% = pure attack weapon

[[base_types.damage.damages]]
type = "physical"
min = 5
max = 12

[base_types.requirements]
level = 1
strength = 10
```

**Hybrid weapon example (physical + elemental):**

```toml
[[base_types]]
id = "infernal_sceptre"
name = "Infernal Sceptre"
class = "one_hand_mace"
tags = ["caster", "melee", "fire"]

[base_types.implicit]
stat = "fire_resistance"
min = 15
max = 25

[base_types.damage]
attack_speed = 1.2
critical_chance = 6.0
spell_efficiency = 80.0    # 80% spell damage effectiveness

[[base_types.damage.damages]]
type = "physical"
min = 5
max = 10

[[base_types.damage.damages]]
type = "fire"
min = 8
max = 16

[base_types.requirements]
level = 30
strength = 20
intelligence = 35
```

**Armour example:**

```toml
[[base_types]]
id = "plate_vest"
name = "Plate Vest"
class = "body_armour"
tags = ["armour", "strength"]

[base_types.defenses]
armour = { min = 80, max = 100 }

[base_types.requirements]
level = 10
strength = 30
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
allowed_classes = ["one_hand_sword", "two_hand_sword", "dagger", "bow"]

[[affixes.tiers]]
tier = 1
weight = 100      # Rarer (lower weight)
min = 25
max = 35
min_ilvl = 68     # Only on level 68+ items

[[affixes.tiers]]
tier = 2
weight = 300
min = 18
max = 24
min_ilvl = 45

[[affixes.tiers]]
tier = 3
weight = 600
min = 10
max = 17
min_ilvl = 25

[[affixes.tiers]]
tier = 4
weight = 1000     # Most common (higher weight)
min = 4
max = 9
min_ilvl = 1      # Can appear on any item
```

**Affix field reference:**

| Field | Type | Description |
|-------|------|-------------|
| `id` | `String` | Unique identifier |
| `name` | `String` | Display name |
| `type` | `"prefix"` or `"suffix"` | Affix slot type |
| `stat` | `StatType` | Which stat this modifies |
| `tags` | `[String]` | Tags for spawn weighting |
| `allowed_classes` | `[ItemClass]` | Classes this can roll on (empty = all) |

**Tier field reference:**

| Field | Type | Description |
|-------|------|-------------|
| `tier` | `u32` | Tier number (1 = best) |
| `weight` | `u32` | Spawn weight (higher = more common) |
| `min` | `i32` | Minimum rolled value |
| `max` | `i32` | Maximum rolled value |
| `min_ilvl` | `u32` | Minimum item level required |

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

## Core Systems

This section explains how the item generation systems work in detail.

### Item Level and Tier Restrictions

Every item has a **level requirement** (from its base type). This level determines which affix tiers can roll on the item.

Each affix tier has a `min_ilvl` (minimum item level) requirement:

```toml
[[affixes.tiers]]
tier = 1          # Best tier
weight = 100
min = 90
max = 110
min_ilvl = 68     # Only drops on level 68+ items

[[affixes.tiers]]
tier = 4          # Worst tier
weight = 1000
min = 15
max = 34
min_ilvl = 1      # Can drop on any item
```

When rolling an affix, only tiers where `min_ilvl <= item.requirements.level` are eligible. This prevents powerful T1 affixes from appearing on low-level items.

**Example:** An Iron Helmet (level 5) can only roll T4 affixes. A Dragon Crown (level 68) can roll any tier.

### Tags and Spawn Weighting

Tags are strings that describe item and affix characteristics. They create thematic connections between items and the affixes that can appear on them.

**Why tags exist:**
- Ensure thematic consistency (fire weapons get fire affixes, not cold)
- Create build diversity (caster items get spell affixes, attack items get attack affixes)
- Allow fine-grained control beyond just item class restrictions
- Enable weighted preferences without hard restrictions

**Tags serve two purposes:**

1. **Filtering** - Affixes must have at least one tag matching the item to be eligible to roll
2. **Weighting** - Each matching tag increases spawn weight by 50%, making thematically appropriate affixes more common

**Tag matching rules:**
- An affix with tags `["physical", "damage"]` can only roll on items that have at least one of those tags
- An affix with no tags can roll on any item (no filtering applied)
- Each matching tag increases spawn weight by **50%**
- Tags are case-sensitive strings

**Example - Eligible affix:**
```
Item: Iron Sword
  tags: ["melee", "physical", "attack", "sword"]

Affix: "Heavy" (added physical damage)
  tags: ["physical", "damage", "attack"]

Step 1 - Eligibility check:
  Matching tags: "physical", "attack" ✓
  Affix is eligible (at least one match found)

Step 2 - Weight calculation:
  Base weight: 1000
  Matching tags: 2 ("physical", "attack")
  Multiplier: 1.0 + (2 × 0.5) = 2.0
  Final weight: 1000 × 2.0 = 2000
```

**Example - Ineligible affix:**
```
Item: Crystal Wand
  tags: ["caster", "ranged", "elemental"]

Affix: "Heavy" (added physical damage)
  tags: ["physical", "damage", "attack"]

Matching tags: 0
Affix is NOT eligible (no matching tags)
→ Physical damage affixes cannot roll on caster wands
```

**Common tag categories:**

| Category | Tags | Purpose |
|----------|------|---------|
| Damage type | `physical`, `fire`, `cold`, `lightning`, `chaos` | Match damage affixes to weapon types |
| Combat style | `melee`, `ranged`, `attack`, `caster` | Match affixes to playstyle |
| Defense type | `armour`, `evasion`, `energy_shield` | Match defense affixes to armour types |
| Attribute | `strength`, `dexterity`, `intelligence` | Attribute-based weighting |
| Special | `life`, `defense`, `damage`, `speed` | General categories |

**Design tips:**
- Give base types 3-5 tags that describe their intended use
- Give affixes 2-3 tags that describe what builds want them
- Use overlapping tags to create natural affinities
- Affixes with no tags are "universal" and can roll anywhere (use sparingly)

**Interaction with other systems:**
- Tags work alongside `allowed_classes` - both must pass for an affix to be eligible
- Tags are independent of affix pools - pools control which affixes a currency can add
- Tags do not affect tier selection - only which affixes can roll, not which tier

### Item Classes

Item classes restrict which affixes can roll on which items via the `allowed_classes` field.

**Weapon Classes:**

| Class | TOML ID | Description |
|-------|---------|-------------|
| One-Hand Sword | `one_hand_sword` | Fast, balanced melee |
| Two-Hand Sword | `two_hand_sword` | High damage melee |
| One-Hand Axe | `one_hand_axe` | Crit-focused melee |
| Two-Hand Axe | `two_hand_axe` | Heavy damage |
| One-Hand Mace | `one_hand_mace` | Blunt melee |
| Two-Hand Mace | `two_hand_mace` | Slow, high damage |
| Dagger | `dagger` | Fast, crit-focused |
| Claw | `claw` | Fast dual-wield |
| Wand | `wand` | Caster ranged |
| Bow | `bow` | Physical ranged |
| Staff | `staff` | Two-hand caster |

**Armour Classes:**

| Class | TOML ID | Description |
|-------|---------|-------------|
| Helmet | `helmet` | Head slot |
| Body Armour | `body_armour` | Chest slot |
| Gloves | `gloves` | Hand slot |
| Boots | `boots` | Feet slot |
| Shield | `shield` | Off-hand defense |

**Accessory Classes:**

| Class | TOML ID | Description |
|-------|---------|-------------|
| Ring | `ring` | Finger slot (2 slots) |
| Amulet | `amulet` | Neck slot |
| Belt | `belt` | Waist slot |

**Class restrictions in affixes:**

```toml
[[affixes]]
id = "added_physical_damage"
allowed_classes = ["one_hand_sword", "two_hand_sword", "dagger", "bow"]
# Empty allowed_classes = can roll on any class
```

### Affix Pools

Pools are named collections of affix IDs. Currencies must specify which pools they draw from.

**Why pools exist:**
- Separate "common" affixes from "special" affixes
- Create themed crafting (elemental, poison, etc.)
- Control which affixes specific currencies can add

**Pool structure:**

```toml
# config/affix_pools/default.toml
[[pools]]
id = "common"
name = "Common Affixes"
affixes = ["added_life", "fire_resistance", "added_physical_damage", ...]

[[pools]]
id = "elemental"
name = "Elemental Affixes"
affixes = ["added_fire_damage", "added_cold_damage", "added_lightning_damage"]

# config/affix_pools/poison.toml
[[pools]]
id = "poison"
name = "Poison Affixes"
affixes = ["poison_damage_flat", "chance_to_poison", "poison_duration"]
```

**Currencies reference pools:**

```toml
[[currencies]]
id = "chaos_orb"
[currencies.effects]
affix_pools = ["common"]           # Draw from common pool only

[[currencies]]
id = "poison_imbue"
[currencies.effects]
affix_pools = ["poison"]           # Draw from poison pool only
```

**Pool selection rules:**
1. If `affix_pools` is empty, currency fails with `NoAffixPoolsSpecified`
2. Multiple pools are combined (union of all affix IDs)
3. Affixes must still pass class restrictions

### Affix Tiers

Each affix has multiple tiers with different value ranges and spawn weights.

```toml
[[affixes]]
id = "added_life"
stat = "added_life"

[[affixes.tiers]]
tier = 1           # Tier number (1 = best)
weight = 100       # Lower = rarer
min = 90           # Minimum rolled value
max = 110          # Maximum rolled value
min_ilvl = 68      # Minimum item level required

[[affixes.tiers]]
tier = 2
weight = 300       # 3x more common than T1
min = 60
max = 89
min_ilvl = 45

[[affixes.tiers]]
tier = 3
weight = 600       # 6x more common than T1
min = 35
max = 59
min_ilvl = 25

[[affixes.tiers]]
tier = 4
weight = 1000      # 10x more common than T1
min = 15
max = 34
min_ilvl = 1
```

**Tier selection process:**
1. Filter tiers by `min_ilvl <= item_level`
2. Sum weights of eligible tiers
3. Random weighted selection
4. Roll value within tier's min-max range

### Damage Types

Weapons can deal multiple damage types, each with its own range:

```toml
[base_types.damage]
attack_speed = 1.4
critical_chance = 5.0
spell_efficiency = 0.0      # 0% for pure attack weapons

[[base_types.damage.damages]]
type = "physical"
min = 10
max = 25

[[base_types.damage.damages]]
type = "fire"
min = 5
max = 15
```

**Available damage types:**

| Type | TOML ID | Color (TUI) |
|------|---------|-------------|
| Physical | `physical` | White |
| Fire | `fire` | Red |
| Cold | `cold` | Cyan |
| Lightning | `lightning` | Yellow |
| Chaos | `chaos` | Magenta |

### Stat Types

All stats that affixes can modify:

**Flat Additions:**
- `added_physical_damage`, `added_fire_damage`, `added_cold_damage`, `added_lightning_damage`, `added_chaos_damage`
- `added_armour`, `added_evasion`, `added_energy_shield`
- `added_life`, `added_mana`, `added_accuracy`
- `added_strength`, `added_dexterity`, `added_intelligence`, `added_constitution`, `added_wisdom`, `added_charisma`, `added_all_attributes`

**Percentage Increases:**
- `increased_physical_damage`, `increased_elemental_damage`, `increased_chaos_damage`
- `increased_attack_speed`, `increased_critical_chance`, `increased_critical_damage`
- `increased_armour`, `increased_evasion`, `increased_energy_shield`
- `increased_life`, `increased_mana`, `increased_accuracy`, `increased_movement_speed`
- `increased_item_rarity`, `increased_item_quantity`

**Resistances (displayed as %):**
- `fire_resistance`, `cold_resistance`, `lightning_resistance`, `chaos_resistance`, `all_resistances`

**Resource Recovery:**
- `life_regeneration`, `mana_regeneration`, `life_on_hit`, `life_leech`, `mana_leech`

**Poison/Ailment:**
- `poison_damage_over_time`, `chance_to_poison`, `increased_poison_duration`

### Requirements

Items have attribute requirements that must be met to equip:

```toml
[base_types.requirements]
level = 45         # Character level
strength = 65      # Attribute requirements
dexterity = 0
constitution = 0
intelligence = 0
wisdom = 0
charisma = 0
```

**The level requirement is particularly important** as it determines which affix tiers can roll (see Item Level section above).

### Rarities

| Rarity | Prefixes | Suffixes | Total | Description |
|--------|----------|----------|-------|-------------|
| Normal | 0 | 0 | 0 | No affixes, white items |
| Magic | 0-1 | 0-1 | 1-2 | Blue items |
| Rare | 0-3 | 0-3 | 4-6 | Yellow items, random name |
| Unique | - | - | Fixed | Orange items, predetermined mods |

### Implicit Modifiers

Base types can have an implicit modifier that is always present (separate from affixes):

```toml
[base_types.implicit]
stat = "added_accuracy"
min = 10
max = 20
```

Implicits:
- Are rolled when the item is created
- Cannot be removed or rerolled by currencies
- Do not count toward affix limits
- Are displayed separately from explicit modifiers

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
