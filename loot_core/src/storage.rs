use crate::generator::Generator;
use crate::item::Item;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::{self, Read, Write};

/// Current binary format version
const BINARY_VERSION: u8 = 1;

/// Magic bytes for item collection files
const COLLECTION_MAGIC: &[u8; 4] = b"LOOT";

/// An operation that was applied to an item
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum Operation {
    /// Apply a currency by ID
    Currency(String),
}

/// Operation type discriminants for binary encoding
#[repr(u8)]
enum OpType {
    Currency = 0,
}

impl TryFrom<u8> for OpType {
    type Error = DecodeError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(OpType::Currency),
            _ => Err(DecodeError::InvalidOperationType(value)),
        }
    }
}

/// Errors that can occur during decoding
#[derive(Debug)]
pub enum DecodeError {
    Io(io::Error),
    InvalidVersion(u8),
    InvalidMagic,
    InvalidUtf8,
    InvalidOperationType(u8),
    UnexpectedEof,
    InvalidStringIndex(u16),
    /// Base type not found during reconstruction
    BaseTypeNotFound(String),
}

impl std::fmt::Display for DecodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DecodeError::Io(e) => write!(f, "IO error: {}", e),
            DecodeError::InvalidVersion(v) => write!(f, "Invalid version: {}", v),
            DecodeError::InvalidMagic => write!(f, "Invalid magic bytes"),
            DecodeError::InvalidUtf8 => write!(f, "Invalid UTF-8 string"),
            DecodeError::InvalidOperationType(t) => write!(f, "Invalid operation type: {}", t),
            DecodeError::UnexpectedEof => write!(f, "Unexpected end of data"),
            DecodeError::InvalidStringIndex(i) => write!(f, "Invalid string index: {}", i),
            DecodeError::BaseTypeNotFound(id) => write!(f, "Base type not found: {}", id),
        }
    }
}

impl std::error::Error for DecodeError {}

impl From<io::Error> for DecodeError {
    fn from(e: io::Error) -> Self {
        DecodeError::Io(e)
    }
}

/// Trait for types that can be encoded to a compact binary format
pub trait BinaryEncode {
    fn encode<W: Write>(&self, writer: &mut W) -> io::Result<()>;

    fn encode_to_vec(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        self.encode(&mut buf).expect("Vec write failed");
        buf
    }
}

/// Trait for types that can be decoded from a compact binary format
pub trait BinaryDecode: Sized {
    /// Decode from a reader, using the generator to reconstruct the item
    fn decode<R: Read>(reader: &mut R, generator: &Generator) -> Result<Self, DecodeError>;

    fn decode_from_slice(data: &[u8], generator: &Generator) -> Result<Self, DecodeError> {
        let mut cursor = std::io::Cursor::new(data);
        Self::decode(&mut cursor, generator)
    }
}

impl BinaryEncode for Item {
    /// Encode item to binary format.
    ///
    /// Format (version 1):
    /// - version: u8
    /// - base_type_id_len: u8
    /// - base_type_id: [u8; base_type_id_len]
    /// - seed: u64 (little-endian)
    /// - operations_count: u16 (little-endian)
    /// - for each operation:
    ///   - op_type: u8
    ///   - if Currency: currency_id_len: u8, currency_id: [u8; currency_id_len]
    fn encode<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        // Version
        writer.write_all(&[BINARY_VERSION])?;

        // Base type ID
        write_string(writer, &self.base_type_id)?;

        // Seed
        writer.write_all(&self.seed.to_le_bytes())?;

        // Operations
        let ops_count = self.operations.len().min(u16::MAX as usize) as u16;
        writer.write_all(&ops_count.to_le_bytes())?;

        for op in self.operations.iter().take(ops_count as usize) {
            match op {
                Operation::Currency(currency_id) => {
                    writer.write_all(&[OpType::Currency as u8])?;
                    write_string(writer, currency_id)?;
                }
            }
        }

        Ok(())
    }
}

impl BinaryDecode for Item {
    fn decode<R: Read>(reader: &mut R, generator: &Generator) -> Result<Self, DecodeError> {
        // Version
        let version = read_u8(reader)?;
        if version != BINARY_VERSION {
            return Err(DecodeError::InvalidVersion(version));
        }

        // Base type ID
        let base_type_id = read_string(reader)?;

        // Seed
        let seed = read_u64(reader)?;

        // Operations
        let ops_count = read_u16(reader)?;
        let mut operations = Vec::with_capacity(ops_count as usize);

        for _ in 0..ops_count {
            let op_type = OpType::try_from(read_u8(reader)?)?;
            let op = match op_type {
                OpType::Currency => {
                    let currency_id = read_string(reader)?;
                    Operation::Currency(currency_id)
                }
            };
            operations.push(op);
        }

        // Reconstruct the item
        generator
            .reconstruct(&base_type_id, seed, &operations)
            .ok_or_else(|| DecodeError::BaseTypeNotFound(base_type_id))
    }
}

impl Item {
    /// Save item to file in binary format
    pub fn save_binary(&self, path: &std::path::Path) -> io::Result<()> {
        let data = self.encode_to_vec();
        std::fs::write(path, data)
    }

    /// Load item from file in binary format
    pub fn load_binary(path: &std::path::Path, generator: &Generator) -> Result<Self, DecodeError> {
        let data = std::fs::read(path)?;
        Self::decode_from_slice(&data, generator)
    }

    /// Export to JSON (includes full computed state)
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    /// Import from JSON
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }
}

/// Collection of items for batch storage
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ItemCollection {
    pub items: Vec<Item>,
}

impl ItemCollection {
    pub fn new() -> Self {
        ItemCollection { items: Vec::new() }
    }

    pub fn add(&mut self, item: Item) {
        self.items.push(item);
    }

    /// Save to file in binary format
    pub fn save_binary(&self, path: &std::path::Path) -> io::Result<()> {
        let data = self.encode_to_vec();
        std::fs::write(path, data)
    }

    /// Load from file in binary format
    pub fn load_binary(path: &std::path::Path, generator: &Generator) -> Result<Self, DecodeError> {
        let data = std::fs::read(path)?;
        Self::decode_from_slice(&data, generator)
    }

    /// Save to file in JSON format
    pub fn save_json(&self, path: &std::path::Path) -> io::Result<()> {
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        std::fs::write(path, json)
    }

    /// Load from file in JSON format
    pub fn load_json(path: &std::path::Path) -> io::Result<Self> {
        let json = std::fs::read_to_string(path)?;
        serde_json::from_str(&json)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
    }
}

impl BinaryEncode for ItemCollection {
    /// Encode collection to binary format with string interning.
    ///
    /// Format:
    /// - magic: [u8; 4] = "LOOT"
    /// - version: u8
    /// - string_table_count: u16 (little-endian)
    /// - for each string: len: u8, data: [u8; len]
    /// - items_count: u32 (little-endian)
    /// - for each item:
    ///   - base_type_id_index: u16 (little-endian)
    ///   - seed: u64 (little-endian)
    ///   - operations_count: u16 (little-endian)
    ///   - for each operation:
    ///     - op_type: u8
    ///     - if Currency: currency_id_index: u16 (little-endian)
    fn encode<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        // Build string table
        let mut string_table: Vec<String> = Vec::new();
        let mut string_indices: HashMap<String, u16> = HashMap::new();

        let mut intern = |s: &str| -> u16 {
            if let Some(&idx) = string_indices.get(s) {
                idx
            } else {
                let idx = string_table.len() as u16;
                string_table.push(s.to_string());
                string_indices.insert(s.to_string(), idx);
                idx
            }
        };

        // Pre-collect all strings
        for item in &self.items {
            intern(&item.base_type_id);
            for op in &item.operations {
                match op {
                    Operation::Currency(id) => {
                        intern(id);
                    }
                }
            }
        }

        // Write header
        writer.write_all(COLLECTION_MAGIC)?;
        writer.write_all(&[BINARY_VERSION])?;

        // Write string table
        let table_count = string_table.len().min(u16::MAX as usize) as u16;
        writer.write_all(&table_count.to_le_bytes())?;
        for s in string_table.iter().take(table_count as usize) {
            write_string(writer, s)?;
        }

        // Write items
        let items_count = self.items.len().min(u32::MAX as usize) as u32;
        writer.write_all(&items_count.to_le_bytes())?;

        for item in self.items.iter().take(items_count as usize) {
            let base_idx = *string_indices.get(&item.base_type_id).unwrap();
            writer.write_all(&base_idx.to_le_bytes())?;
            writer.write_all(&item.seed.to_le_bytes())?;

            let ops_count = item.operations.len().min(u16::MAX as usize) as u16;
            writer.write_all(&ops_count.to_le_bytes())?;

            for op in item.operations.iter().take(ops_count as usize) {
                match op {
                    Operation::Currency(currency_id) => {
                        writer.write_all(&[OpType::Currency as u8])?;
                        let idx = *string_indices.get(currency_id).unwrap();
                        writer.write_all(&idx.to_le_bytes())?;
                    }
                }
            }
        }

        Ok(())
    }
}

impl BinaryDecode for ItemCollection {
    fn decode<R: Read>(reader: &mut R, generator: &Generator) -> Result<Self, DecodeError> {
        // Read and verify magic
        let mut magic = [0u8; 4];
        reader.read_exact(&mut magic)?;
        if &magic != COLLECTION_MAGIC {
            return Err(DecodeError::InvalidMagic);
        }

        // Version
        let version = read_u8(reader)?;
        if version != BINARY_VERSION {
            return Err(DecodeError::InvalidVersion(version));
        }

        // Read string table
        let table_count = read_u16(reader)?;
        let mut string_table = Vec::with_capacity(table_count as usize);
        for _ in 0..table_count {
            string_table.push(read_string(reader)?);
        }

        // Read items
        let items_count = read_u32(reader)?;
        let mut items = Vec::with_capacity(items_count as usize);

        for _ in 0..items_count {
            let base_idx = read_u16(reader)?;
            let base_type_id = string_table
                .get(base_idx as usize)
                .ok_or(DecodeError::InvalidStringIndex(base_idx))?
                .clone();

            let seed = read_u64(reader)?;

            let ops_count = read_u16(reader)?;
            let mut operations = Vec::with_capacity(ops_count as usize);

            for _ in 0..ops_count {
                let op_type = OpType::try_from(read_u8(reader)?)?;
                let op = match op_type {
                    OpType::Currency => {
                        let idx = read_u16(reader)?;
                        let currency_id = string_table
                            .get(idx as usize)
                            .ok_or(DecodeError::InvalidStringIndex(idx))?
                            .clone();
                        Operation::Currency(currency_id)
                    }
                };
                operations.push(op);
            }

            // Reconstruct item
            let item = generator
                .reconstruct(&base_type_id, seed, &operations)
                .ok_or_else(|| DecodeError::BaseTypeNotFound(base_type_id))?;

            items.push(item);
        }

        Ok(ItemCollection { items })
    }
}

// Helper functions for binary I/O

fn write_string<W: Write>(writer: &mut W, s: &str) -> io::Result<()> {
    let bytes = s.as_bytes();
    let len = bytes.len().min(255) as u8;
    writer.write_all(&[len])?;
    writer.write_all(&bytes[..len as usize])?;
    Ok(())
}

fn read_u8<R: Read>(reader: &mut R) -> Result<u8, DecodeError> {
    let mut buf = [0u8; 1];
    reader.read_exact(&mut buf).map_err(|e| {
        if e.kind() == io::ErrorKind::UnexpectedEof {
            DecodeError::UnexpectedEof
        } else {
            DecodeError::Io(e)
        }
    })?;
    Ok(buf[0])
}

fn read_u16<R: Read>(reader: &mut R) -> Result<u16, DecodeError> {
    let mut buf = [0u8; 2];
    reader.read_exact(&mut buf).map_err(|e| {
        if e.kind() == io::ErrorKind::UnexpectedEof {
            DecodeError::UnexpectedEof
        } else {
            DecodeError::Io(e)
        }
    })?;
    Ok(u16::from_le_bytes(buf))
}

fn read_u32<R: Read>(reader: &mut R) -> Result<u32, DecodeError> {
    let mut buf = [0u8; 4];
    reader.read_exact(&mut buf).map_err(|e| {
        if e.kind() == io::ErrorKind::UnexpectedEof {
            DecodeError::UnexpectedEof
        } else {
            DecodeError::Io(e)
        }
    })?;
    Ok(u32::from_le_bytes(buf))
}

fn read_u64<R: Read>(reader: &mut R) -> Result<u64, DecodeError> {
    let mut buf = [0u8; 8];
    reader.read_exact(&mut buf).map_err(|e| {
        if e.kind() == io::ErrorKind::UnexpectedEof {
            DecodeError::UnexpectedEof
        } else {
            DecodeError::Io(e)
        }
    })?;
    Ok(u64::from_le_bytes(buf))
}

fn read_string<R: Read>(reader: &mut R) -> Result<String, DecodeError> {
    let len = read_u8(reader)?;
    let mut buf = vec![0u8; len as usize];
    reader.read_exact(&mut buf).map_err(|e| {
        if e.kind() == io::ErrorKind::UnexpectedEof {
            DecodeError::UnexpectedEof
        } else {
            DecodeError::Io(e)
        }
    })?;
    String::from_utf8(buf).map_err(|_| DecodeError::InvalidUtf8)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Config;
    use std::path::Path;

    fn make_generator() -> Generator {
        let config = Config::load_from_dir(Path::new("../config")).unwrap();
        Generator::new(config)
    }

    #[test]
    fn test_item_encode_decode_roundtrip() {
        let generator = make_generator();

        // Generate item and apply currencies
        let item = generator.generate("iron_sword", 12345).unwrap();
        let item = generator.apply_currency(&item, "transmute").unwrap();
        let item = generator.apply_currency(&item, "augment").unwrap();

        // Encode and decode
        let encoded = item.encode_to_vec();
        let decoded = Item::decode_from_slice(&encoded, &generator).unwrap();

        assert_eq!(decoded.base_type_id, item.base_type_id);
        assert_eq!(decoded.seed, item.seed);
        assert_eq!(decoded.operations, item.operations);
        assert_eq!(decoded.rarity, item.rarity);
        assert_eq!(decoded.prefixes.len(), item.prefixes.len());
    }

    #[test]
    fn test_item_no_operations() {
        let generator = make_generator();
        let item = generator.generate("iron_sword", 999).unwrap();

        let encoded = item.encode_to_vec();
        let decoded = Item::decode_from_slice(&encoded, &generator).unwrap();

        assert_eq!(decoded.base_type_id, "iron_sword");
        assert_eq!(decoded.seed, 999);
        assert!(decoded.operations.is_empty());
    }

    #[test]
    fn test_item_collection_encode_decode_roundtrip() {
        let generator = make_generator();

        let mut collection = ItemCollection::new();

        let item1 = generator.generate("iron_sword", 111).unwrap();
        let item1 = generator.apply_currency(&item1, "transmute").unwrap();
        collection.add(item1);

        let item2 = generator.generate("iron_sword", 222).unwrap();
        let item2 = generator.apply_currency(&item2, "transmute").unwrap();
        let item2 = generator.apply_currency(&item2, "augment").unwrap();
        collection.add(item2);

        let item3 = generator.generate("leather_boots", 333).unwrap();
        collection.add(item3);

        let encoded = collection.encode_to_vec();
        let decoded = ItemCollection::decode_from_slice(&encoded, &generator).unwrap();

        assert_eq!(decoded.items.len(), 3);
        assert_eq!(decoded.items[0].base_type_id, "iron_sword");
        assert_eq!(decoded.items[0].seed, 111);
        assert_eq!(decoded.items[1].base_type_id, "iron_sword");
        assert_eq!(decoded.items[1].seed, 222);
        assert_eq!(decoded.items[2].base_type_id, "leather_boots");
        assert_eq!(decoded.items[2].seed, 333);
    }

    #[test]
    fn test_empty_collection() {
        let generator = make_generator();
        let collection = ItemCollection::new();
        let encoded = collection.encode_to_vec();
        let decoded = ItemCollection::decode_from_slice(&encoded, &generator).unwrap();
        assert!(decoded.items.is_empty());
    }

    #[test]
    fn test_item_binary_size() {
        let generator = make_generator();

        let item = generator.generate("iron_sword", u64::MAX).unwrap();
        let item = generator.apply_currency(&item, "transmute").unwrap();

        let binary = item.encode_to_vec();

        // Binary: 1 (version) + 1 + 10 (base_type) + 8 (seed) + 2 (ops count) + 1 (op type) + 1 + 9 (currency) = 33 bytes
        assert_eq!(binary.len(), 33);
    }

    #[test]
    fn test_deterministic_reconstruction() {
        let generator = make_generator();

        // Create and craft an item
        let item1 = generator.generate("iron_sword", 12345).unwrap();
        let item1 = generator.apply_currency(&item1, "transmute").unwrap();
        let item1 = generator.apply_currency(&item1, "augment").unwrap();

        // Encode/decode
        let bytes = item1.encode_to_vec();
        let item2 = Item::decode_from_slice(&bytes, &generator).unwrap();

        // Should be identical
        assert_eq!(item1.name, item2.name);
        assert_eq!(item1.rarity, item2.rarity);
        assert_eq!(item1.prefixes.len(), item2.prefixes.len());
        assert_eq!(item1.suffixes.len(), item2.suffixes.len());

        for (p1, p2) in item1.prefixes.iter().zip(item2.prefixes.iter()) {
            assert_eq!(p1.affix_id, p2.affix_id);
            assert_eq!(p1.value, p2.value);
        }
    }
}
