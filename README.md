# Loot Generator

A configurable loot generation system for games, inspired by Path of Exile's itemization. Features seed-based determinism, a data-driven currency system, and compact binary storage.

## Quick Start

```rust
use loot_core::{Config, Generator, StoredItem};
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load config and create generator
    let config = Config::load_from_dir(Path::new("config"))?;
    let generator = Generator::new(config);

    // Generate an item with a seed
    let mut rng = Generator::make_rng(12345);
    let mut item = generator.generate_normal("iron_sword", &mut rng).unwrap();

    // Apply currencies to craft the item
    generator.apply_currency(&mut item, "transmute", &mut rng);  // Normal -> Magic
    generator.apply_currency(&mut item, "augment", &mut rng);    // Add another affix

    // Store compactly as seed + operations
    let stored = StoredItem::new("iron_sword", 12345)
        .with_currency("transmute")
        .with_currency("augment");

    // Reconstruct identically from stored form
    let reconstructed = stored.reconstruct(&generator).unwrap();

    Ok(())
}
```

## Core Concepts

- **Seed-based determinism** - Items store a seed + operation history, not full stats. Reconstruction is deterministic.
- **Data-driven currencies** - All crafting operations defined in TOML, not code.
- **Tag-based affix weighting** - Items and affixes have tags; matching tags increase spawn probability.
- **Compact binary storage** - Items encode to ~30 bytes vs hundreds for JSON.

## API Reference

### Creating a Generator

```rust
use loot_core::{Config, Generator};
use std::path::Path;

let config = Config::load_from_dir(Path::new("config"))?;
let generator = Generator::new(config);
```

### Generating Items

```rust
// Create a seeded RNG (same seed = same results)
let mut rng = Generator::make_rng(12345);

// Generate a normal (white) item
let item = generator.generate_normal("iron_sword", &mut rng).unwrap();

// Generate a unique item directly
let unique = generator.generate_unique("starforge", &mut rng).unwrap();
```

### Applying Currencies

```rust
use loot_core::CurrencyError;

// Apply by currency ID (preferred)
match generator.apply_currency(&mut item, "transmute", &mut rng) {
    Some(Ok(())) => println!("Success!"),
    Some(Err(CurrencyError::InvalidRarity { .. })) => println!("Wrong rarity"),
    Some(Err(e)) => println!("Error: {}", e),
    None => println!("Currency not found"),
}

// Check if currency can be applied
if generator.can_apply_currency(&item, "augment") {
    generator.apply_currency(&mut item, "augment", &mut rng);
}
```

### Reading Item Properties

```rust
println!("Name: {}", item.name);
println!("Base: {}", item.base_name);
println!("Rarity: {:?}", item.rarity);
println!("Class: {:?}", item.class);

// Modifiers
if let Some(implicit) = &item.implicit {
    println!("Implicit: {}", implicit.display());
}

for prefix in &item.prefixes {
    println!("Prefix [T{}]: {}", prefix.tier, prefix.display());
}

for suffix in &item.suffixes {
    println!("Suffix [T{}]: {}", suffix.tier, suffix.display());
}

// Capacity
println!("Can add prefix: {}", item.can_add_prefix());
println!("Can add suffix: {}", item.can_add_suffix());
```

### Storing Items

Items are stored as seed + operation history for compact, deterministic reconstruction.

```rust
use loot_core::{StoredItem, Operation, ItemCollection};

// Build with fluent API
let stored = StoredItem::new("iron_sword", 12345)
    .with_currency("transmute")
    .with_currency("augment");

// Or build incrementally
let mut stored = StoredItem::new("iron_sword", 12345);
stored.push_currency("transmute");
stored.push_currency("augment");

// Reconstruct the full item
let item = stored.reconstruct(&generator).unwrap();
```

### Serialization

Both JSON and compact binary formats are supported.

```rust
use loot_core::{StoredItem, ItemCollection, BinaryEncode, BinaryDecode};

// Single item - JSON
let json = stored.to_json()?;
let loaded = StoredItem::from_json(&json)?;

// Single item - Binary (~33 bytes vs ~150 for JSON)
let bytes = stored.encode_to_vec();
let loaded = StoredItem::decode_from_slice(&bytes)?;

// Collections with string interning (even more compact)
let mut collection = ItemCollection::new();
collection.add(stored);

// JSON
collection.save_to_file(Path::new("items.json"))?;
let loaded = ItemCollection::load_from_file(Path::new("items.json"))?;

// Binary (3-4x smaller than JSON)
collection.save_binary(Path::new("items.bin"))?;
let loaded = ItemCollection::load_binary(Path::new("items.bin"))?;
```

### Querying Configuration

```rust
// List base types
for id in generator.base_type_ids() {
    let base = generator.get_base_type(id).unwrap();
    println!("{}: {} ({:?})", id, base.name, base.class);
}

// List currencies
for (id, currency) in &generator.config().currencies {
    println!("{}: {}", currency.name, currency.description);
}

// List uniques
for id in generator.unique_ids() {
    let unique = generator.get_unique(id).unwrap();
    println!("{}: {}", unique.name, unique.base_type);
}
```

## Error Handling

```rust
use loot_core::CurrencyError;

match generator.apply_currency(&mut item, "chaos", &mut rng) {
    Some(Ok(())) => { /* Success */ }
    Some(Err(CurrencyError::InvalidRarity { expected, got })) => {
        println!("Need {:?}, have {:?}", expected, got);
    }
    Some(Err(CurrencyError::NoAffixSlots)) => {
        println!("Item is full");
    }
    Some(Err(CurrencyError::NoValidAffixes)) => {
        println!("No affixes available for this item");
    }
    Some(Err(e)) => println!("Error: {}", e),
    None => println!("Currency '{}' not found", "chaos"),
}
```

## Architecture

```
loot_generator/
├── loot_core/          # Library - generation, storage, config
├── loot_tui/           # Terminal UI for experimentation
└── config/             # TOML configuration
    ├── base_types/     # Item base definitions
    ├── affixes/        # Affix definitions with tiers
    ├── affix_pools/    # Affix groupings for currencies
    ├── currencies/     # Currency effect definitions
    └── uniques/        # Unique item templates
```

## Configuration

See [CONFIG.md](CONFIG.md) for detailed configuration documentation including:
- Base type definitions (weapons, armor, accessories)
- Affix tiers and spawn weights
- Currency requirements and effects
- Unique item recipes
- Tag system and spawn weighting

## TUI Application

```bash
cargo run -p loot_tui
```

| Key | Action |
|-----|--------|
| `n` | New item |
| `c` | Currency popup |
| `Tab` | Toggle detail view |
| `d` | Delete item |
| `q` | Quit |

## License

MIT
