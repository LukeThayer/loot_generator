# Loot Generator

A configurable loot generation system for games, inspired by Path of Exile's itemization. Features seed-based determinism, a data-driven currency system, and compact binary storage.

## Quick Start

```rust
use loot_core::{Config, Generator, BinaryEncode, BinaryDecode, Item};
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load config and create generator
    let config = Config::load_from_dir(Path::new("config"))?;
    let generator = Generator::new(config);

    // Generate an item with a seed (same seed = same item)
    let item = generator.generate("iron_sword", 12345).unwrap();

    // Apply currencies - returns a new item each time (immutable)
    let item = generator.apply_currency(&item, "transmute")?;  // Normal -> Magic
    let item = generator.apply_currency(&item, "augment")?;    // Add another affix

    // Encode to compact binary (item stores seed + operations)
    let bytes = item.encode_to_vec();

    // Decode back to full item (deterministically reconstructed)
    let reconstructed = Item::decode_from_slice(&bytes, &generator)?;

    Ok(())
}
```

## Core Concepts

- **Seed-based determinism** - Items store a seed + operation history internally. Reconstruction is deterministic.
- **Immutable operations** - `apply_currency` returns a new item rather than mutating.
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
// Generate a normal (white) item with a seed
let item = generator.generate("iron_sword", 12345).unwrap();

// Generate a unique item directly
let unique = generator.generate_unique("starforge", 12345).unwrap();
```

### Applying Currencies

Currencies return a new item with the effect applied. The original item is unchanged.

```rust
use loot_core::CurrencyError;

// Apply currency - returns Result<Item, CurrencyError>
let item = generator.generate("iron_sword", 12345).unwrap();

match generator.apply_currency(&item, "transmute") {
    Ok(new_item) => println!("Success! New item: {}", new_item.name),
    Err(CurrencyError::InvalidRarity { .. }) => println!("Wrong rarity"),
    Err(CurrencyError::UnknownCurrency(id)) => println!("Unknown currency: {}", id),
    Err(e) => println!("Error: {}", e),
}

// Chain multiple applications
let item = generator.generate("iron_sword", 12345).unwrap();
let item = generator.apply_currency(&item, "transmute")?;
let item = generator.apply_currency(&item, "augment")?;

// Check if currency can be applied before applying
if generator.can_apply_currency(&item, "chaos") {
    let item = generator.apply_currency(&item, "chaos")?;
}
```

### Reading Item Properties

```rust
println!("Name: {}", item.name);
println!("Base: {}", item.base_name);
println!("Rarity: {:?}", item.rarity);
println!("Class: {:?}", item.class);

// Seed and operations (for storage/debugging)
println!("Seed: 0x{:016X}", item.seed);
println!("Operations: {:?}", item.operations);

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

### Binary Serialization

Items encode to a compact binary format storing only seed + operations. Full stats are reconstructed deterministically.

```rust
use loot_core::{Item, BinaryEncode, BinaryDecode};

// Encode item to binary (~30 bytes vs hundreds for JSON)
let bytes = item.encode_to_vec();

// Decode back to full item (requires generator for reconstruction)
let loaded = Item::decode_from_slice(&bytes, &generator)?;
```

### Item Collections

For storing multiple items with string interning for even more compact storage:

```rust
use loot_core::ItemCollection;
use std::path::Path;

let mut collection = ItemCollection::new();
collection.add(item);

// Binary format (3-4x smaller than JSON)
collection.save_binary(Path::new("items.bin"))?;
let loaded = ItemCollection::load_binary(Path::new("items.bin"), &generator)?;
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

match generator.apply_currency(&item, "chaos") {
    Ok(new_item) => { /* Success - use new_item */ }
    Err(CurrencyError::InvalidRarity { expected, got }) => {
        println!("Need {:?}, have {:?}", expected, got);
    }
    Err(CurrencyError::NoAffixSlots) => {
        println!("Item is full");
    }
    Err(CurrencyError::NoValidAffixes) => {
        println!("No affixes available for this item");
    }
    Err(CurrencyError::UnknownCurrency(id)) => {
        println!("Currency '{}' not found", id);
    }
    Err(e) => println!("Error: {}", e),
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
