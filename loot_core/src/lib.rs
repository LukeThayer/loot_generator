pub mod config;
pub mod currency;
pub mod generator;
pub mod item;
pub mod storage;
pub mod types;

pub use config::Config;
pub use currency::CurrencyError;
pub use generator::Generator;
pub use item::Item;
pub use storage::{BinaryDecode, BinaryEncode, DecodeError, ItemCollection, Operation};
pub use types::*;

#[cfg(test)]
mod tests {
    use super::item::Modifier;
    use super::types::{AffixScope, StatType};

    #[test]
    fn test_damage_range_display() {
        let modifier = Modifier {
            affix_id: "added_fire_damage".to_string(),
            name: "Flaming".to_string(),
            stat: StatType::AddedFireDamage,
            scope: AffixScope::Local,
            tier: 1,
            value: 20,
            value_max: Some(35),
            tier_min: 18,
            tier_max: 28,
            tier_max_value: Some((32, 48)),
        };

        assert_eq!(modifier.display(), "Adds 20 to 35 Fire Damage");
    }

    #[test]
    fn test_non_damage_display() {
        let modifier = Modifier {
            affix_id: "added_life".to_string(),
            name: "Robust".to_string(),
            stat: StatType::AddedLife,
            scope: AffixScope::Global,
            tier: 1,
            value: 50,
            value_max: None,
            tier_min: 40,
            tier_max: 60,
            tier_max_value: None,
        };

        assert_eq!(modifier.display(), "+50 Added Life");
    }
}
