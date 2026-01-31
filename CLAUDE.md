# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build & Development Commands

```bash
# Build
cargo build              # Debug build
cargo build --release    # Release build

# Test
cargo test               # Run all tests
cargo test <name>        # Run specific test by name

# Lint & Format
cargo clippy             # Run lints
cargo fmt                # Format code
cargo fmt --check        # Check formatting without changes

# Run TUI
cargo run -p loot_tui    # Run the terminal UI
cargo watch -x 'run -p loot_tui'  # Auto-rebuild on changes
```

## Development Environment

This project uses Nix flakes for reproducible development. Enter the dev shell with:

```bash
nix develop
```

This provides: Rust stable toolchain, cargo-watch, cargo-edit, rust-analyzer, and git.

## Architecture

Cargo workspace with two crates:

- **loot_core/**: Library crate containing all generation logic, data models, and configuration loading
- **loot_tui/**: Binary crate providing a terminal UI for experimenting with items

See `README.md` for complete design documentation including item structure, affix system, currencies, and configuration format.

### Key Design Concepts

- **Seed-based storage**: Items store a seed + operation history, replayed deterministically to reconstruct full stats
- **Tag-based affix weighting**: Items and affixes have tags; matching tags increase spawn weight
- **Modular config**: `base_types.toml`, `affixes.toml`, `currencies.toml`, `uniques.toml`
- **Markdown export**: Human-readable item display for clipboard

### TUI Keybindings

| Key | Action |
|-----|--------|
| `n` | New item (opens base type selector) |
| `j/k` or `↑/↓` | Navigate lists |
| `h/l` or `←/→` | Switch between inventory and currency panels |
| `Tab` | Toggle between Stats and Seed/Ops view |
| `d` or `Delete` | Delete selected item |
| `Enter` | Confirm selection (base type or currency) |
| `Esc` | Cancel popup |
| `q` | Quit |

**Quick Currency Keys** (when in inventory):
`t` Transmute, `u` Augment, `a` Alteration, `y` Alchemy, `c` Chaos, `e` Exalt, `x` Annul, `s` Scour
