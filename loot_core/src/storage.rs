use crate::currency::apply_currency;
use crate::generator::Generator;
use crate::item::Item;
use serde::{Deserialize, Serialize};

/// Compact storage format for an item: seed + operation history
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredItem {
    /// Base type ID
    pub base_type_id: String,
    /// RNG seed for deterministic recreation
    pub seed: u64,
    /// Sequence of operations applied to the item
    pub operations: Vec<Operation>,
}

/// An operation that was applied to an item
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Operation {
    /// Apply a currency by ID
    Currency(String),
}

impl StoredItem {
    /// Create a new stored item with just a base type and seed
    pub fn new(base_type_id: String, seed: u64) -> Self {
        StoredItem {
            base_type_id,
            seed,
            operations: Vec::new(),
        }
    }

    /// Add an operation to the history
    pub fn push_operation(&mut self, op: Operation) {
        self.operations.push(op);
    }

    /// Reconstruct the full item by replaying the seed and operations
    pub fn reconstruct(&self, generator: &Generator) -> Option<Item> {
        let mut rng = Generator::make_rng(self.seed);

        // Generate the base normal item
        let mut item = generator.generate_normal(&self.base_type_id, &mut rng)?;

        // Replay each operation
        for op in &self.operations {
            match op {
                Operation::Currency(currency_id) => {
                    if let Some(currency) = generator.config().currencies.get(currency_id) {
                        // Errors during reconstruction are ignored - the item stays as-is
                        let _ = apply_currency(generator, &mut item, currency, &mut rng);
                    }
                }
            }
        }

        Some(item)
    }

    /// Export to JSON string
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    /// Import from JSON string
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }
}

/// Collection of stored items
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ItemCollection {
    pub items: Vec<StoredItem>,
}

impl ItemCollection {
    pub fn new() -> Self {
        ItemCollection { items: Vec::new() }
    }

    pub fn add(&mut self, item: StoredItem) {
        self.items.push(item);
    }

    pub fn save_to_file(&self, path: &std::path::Path) -> std::io::Result<()> {
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        std::fs::write(path, json)
    }

    pub fn load_from_file(path: &std::path::Path) -> std::io::Result<Self> {
        let json = std::fs::read_to_string(path)?;
        serde_json::from_str(&json)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
    }
}
