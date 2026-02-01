# Configuration Guide

This document covers the TOML configuration format for loot_core.

## Directory Structure

```
config/
├── base_types/
│   ├── weapons.toml
│   ├── armour.toml
│   └── accessories.toml
├── affixes/
│   ├── weapon.toml
│   ├── armour.toml
│   └── accessories.toml
├── affix_pools/
│   └── default.toml
├── currencies/
│   ├── rarity.toml
│   ├── affix_modification.toml
│   └── imbue.toml
└── uniques/
    └── titans_grip.toml
```

## Base Types

Base types define the item templates that can be generated.

```toml
[[base_types]]
id = "iron_sword"
name = "Iron Sword"
class = "one_hand_sword"
tags = ["melee", "physical", "attack", "sword", "strength"]

[base_types.implicit]
stat = "added_accuracy"
min = 10
max = 20

[base_types.damage]
attack_speed = 1.3
critical_chance = 5.0
spell_efficiency = 0.0

[[base_types.damage.damages]]
type = "physical"
min = 5
max = 12

[base_types.requirements]
level = 1
strength = 10
```

### Fields

| Field | Type | Description |
|-------|------|-------------|
| `id` | String | Unique identifier |
| `name` | String | Display name |
| `class` | ItemClass | Item class (see below) |
| `tags` | [String] | Tags for affix weighting |
| `implicit` | Optional | Implicit modifier |
| `damage` | Optional | Weapon damage config |
| `defenses` | Optional | Armour defense config |
| `requirements` | Object | Level/attribute requirements |

### Item Classes

**Weapons:**
- `one_hand_sword`, `two_hand_sword`
- `one_hand_axe`, `two_hand_axe`
- `one_hand_mace`, `two_hand_mace`
- `dagger`, `claw`
- `wand`, `staff`, `bow`

**Armour:**
- `helmet`, `body_armour`, `gloves`, `boots`, `shield`

**Accessories:**
- `ring`, `amulet`, `belt`

### Damage Types

```toml
[[base_types.damage.damages]]
type = "physical"  # or fire, cold, lightning, chaos
min = 5
max = 12
```

### Defense Types

```toml
[base_types.defenses]
armour = { min = 80, max = 100 }
evasion = { min = 60, max = 80 }
energy_shield = { min = 40, max = 55 }
```

## Affixes

Affixes define modifiers that can roll on items.

```toml
[[affixes]]
id = "added_physical_damage"
name = "Heavy"
type = "prefix"
stat = "added_physical_damage"
scope = "local"
tags = ["physical", "damage", "attack"]
allowed_classes = ["one_hand_sword", "two_hand_sword", "dagger", "bow"]

[[affixes.tiers]]
tier = 1
weight = 100
min = 25
max = 35
min_ilvl = 68

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
weight = 1000
min = 4
max = 9
min_ilvl = 1
```

### Affix Fields

| Field | Type | Description |
|-------|------|-------------|
| `id` | String | Unique identifier |
| `name` | String | Display name |
| `type` | "prefix" or "suffix" | Affix slot |
| `stat` | StatType | Stat to modify |
| `scope` | "local" or "global" | Scope of effect |
| `tags` | [String] | Tags for spawn weighting |
| `allowed_classes` | [ItemClass] | Restricted classes (empty = all) |

### Tier Fields

| Field | Type | Description |
|-------|------|-------------|
| `tier` | u32 | Tier number (1 = best) |
| `weight` | u32 | Spawn weight (higher = more common) |
| `min` | i32 | Minimum value |
| `max` | i32 | Maximum value |
| `min_ilvl` | u32 | Minimum item level required |
| `max_value` | Optional | For damage ranges: `{ min, max }` |

### Damage Range Affixes

For "adds X to Y damage" affixes:

```toml
[[affixes.tiers]]
tier = 1
weight = 100
min = 20      # min damage minimum
max = 28      # min damage maximum
max_value = { min = 32, max = 48 }  # max damage range
min_ilvl = 68
```

## Affix Pools

Pools group affixes for currencies to draw from.

```toml
[[pools]]
id = "common"
name = "Common Affixes"
description = "Standard affix pool"
affixes = [
    "added_physical_damage",
    "added_fire_damage",
    "added_life",
    "fire_resistance",
]

[[pools]]
id = "elemental"
name = "Elemental Affixes"
affixes = ["added_fire_damage", "added_cold_damage", "added_lightning_damage"]
```

## Currencies

Currencies are data-driven crafting operations.

```toml
[[currencies]]
id = "transmute"
name = "Orb of Transmutation"
description = "Upgrades a normal item to magic"
category = "Rarity"

[currencies.requires]
rarities = ["normal"]

[currencies.effects]
set_rarity = "magic"
add_affixes = { min = 1, max = 1 }
affix_pools = ["common"]
```

### Requirements

| Field | Type | Description |
|-------|------|-------------|
| `rarities` | [Rarity] | Required rarity (normal/magic/rare) |
| `has_affix` | bool | Must have at least one affix |
| `has_affix_slot` | bool | Must have room for another affix |

### Effects

| Field | Type | Description |
|-------|------|-------------|
| `set_rarity` | Rarity | Change rarity |
| `clear_affixes` | bool | Remove all affixes |
| `add_affixes` | {min, max} | Add random affixes |
| `remove_affixes` | u32 | Remove N random affixes |
| `reroll_affixes` | u32 | Reroll N random affixes |
| `affix_pools` | [String] | Pools to draw from |
| `add_specific_affix` | [...] | Add from weighted set |
| `try_unique` | bool | Attempt unique transformation |

### Specific Affix Selection

```toml
[currencies.effects]
add_specific_affix = [
    { id = "added_fire_damage", weight = 100 },
    { id = "added_cold_damage", weight = 100 },
    { id = "added_lightning_damage", weight = 100 },
]
```

Force a specific tier:

```toml
add_specific_affix = [{ id = "added_life", tier = 1 }]
```

### Imbue Currencies

Upgrade to magic with a guaranteed affix:

```toml
[[currencies]]
id = "imbue_fire"
name = "Ember Imbue"
description = "Imbues with fire damage"
category = "Imbue"

[currencies.requires]
rarities = ["normal"]

[currencies.effects]
set_rarity = "magic"
add_specific_affix = [{ id = "added_fire_damage" }]
```

## Unique Items

```toml
[unique]
id = "titans_grip"
name = "Titan's Grip"
base_type = "iron_gauntlets"
flavor = "The mountain bows to no one."

[[unique.mods]]
stat = "added_strength"
min = 40
max = 60

[[unique.mods]]
stat = "increased_physical_damage"
min = 80
max = 120
```

### Unique Recipes

Transform items into uniques based on affixes:

```toml
[recipe]
weight = 100

[[recipe.required_affixes]]
stat = "added_strength"
affix_type = "suffix"
min_tier = 1
max_tier = 3

[[recipe.mappings]]
from_stat = "added_strength"
to_mod_index = 0
mode = "percentage"
influence = 0.8
```

## Tag System

Tags create thematic connections between items and affixes.

### How Tags Work

1. **Filtering** - Affixes must share at least one tag with the item
2. **Weighting** - Each matching tag increases spawn weight by 50%

### Example

```
Item: Iron Sword
  tags: ["melee", "physical", "attack", "sword"]

Affix: "Heavy" (added physical damage)
  tags: ["physical", "damage", "attack"]

Matching: "physical", "attack" (2 tags)
Weight multiplier: 1.0 + (2 × 0.5) = 2.0
```

### Common Tags

| Category | Tags |
|----------|------|
| Damage | `physical`, `fire`, `cold`, `lightning`, `chaos` |
| Style | `melee`, `ranged`, `attack`, `caster` |
| Defense | `armour`, `evasion`, `energy_shield` |
| Attribute | `strength`, `dexterity`, `intelligence` |
| General | `life`, `defense`, `damage`, `speed` |

## Stat Types

### Flat Additions
- `added_physical_damage`, `added_fire_damage`, `added_cold_damage`, `added_lightning_damage`, `added_chaos_damage`
- `added_armour`, `added_evasion`, `added_energy_shield`
- `added_life`, `added_mana`, `added_accuracy`
- `added_strength`, `added_dexterity`, `added_intelligence`

### Percentage Increases
- `increased_physical_damage`, `increased_elemental_damage`
- `increased_attack_speed`, `increased_critical_chance`, `increased_critical_damage`
- `increased_armour`, `increased_evasion`, `increased_energy_shield`
- `increased_life`, `increased_mana`, `increased_movement_speed`

### Resistances
- `fire_resistance`, `cold_resistance`, `lightning_resistance`, `chaos_resistance`, `all_resistances`

### Recovery
- `life_regeneration`, `mana_regeneration`, `life_on_hit`, `life_leech`, `mana_leech`

## Rarities

| Rarity | Prefixes | Suffixes | Total |
|--------|----------|----------|-------|
| Normal | 0 | 0 | 0 |
| Magic | 0-1 | 0-1 | 1-2 |
| Rare | 0-3 | 0-3 | 4-6 |
| Unique | - | - | Fixed |
