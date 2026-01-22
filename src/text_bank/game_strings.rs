use rustc_hash::FxHashMap;
use serde::Deserialize;
use std::path::Path;

use crate::GameFamily;
use crate::game::GameLanguage;

#[derive(Debug, Clone, Copy)]
pub struct TextBankIds {
    pub species: u16,
    pub items: u16,
    pub moves: u16,
    pub abilities: u16,
    pub types: u16,
}

impl TextBankIds {
    pub fn for_game(family: GameFamily, language: GameLanguage) -> Self {
        let is_jp = language.is_japanese();

        match family {
            GameFamily::Platinum => Self {
                species: 412,
                items: 392,
                moves: 647,
                abilities: 610,
                types: 624,
            },
            GameFamily::DP => Self {
                species: 362,
                items: if is_jp { 341 } else { 344 },
                moves: 588,
                abilities: 552,
                types: 565,
            },
            GameFamily::HGSS => Self {
                species: if is_jp { 232 } else { 237 },
                items: if is_jp { 219 } else { 222 },
                moves: if is_jp { 739 } else { 750 },
                abilities: 720,
                types: 735,
            },
        }
    }
}

/// JSON structure for Chatot text archives
#[derive(Debug, Deserialize)]
struct ChatotTextArchive {
    #[allow(dead_code)]
    key: u16,
    messages: Vec<ChatotMessage>,
}

#[derive(Debug, Deserialize)]
struct ChatotMessage {
    #[allow(dead_code)]
    id: String,
    #[serde(flatten)]
    lang_content: FxHashMap<String, MessageContent>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum MessageContent {
    Single(String),
    Multi(Vec<String>),
}

/// Holds all game string tables loaded from DSPRE's expanded/textArchives folder.
/// Provides name-to-ID and ID-to-name resolution for species, items, moves, abilities, and types.
#[derive(Debug, Clone, Default)]
pub struct GameStrings {
    /// Species names indexed by species ID
    pub species: Vec<String>,
    /// Lowercase species name -> ID
    species_to_id: FxHashMap<String, u16>,

    /// Item names indexed by item ID
    pub items: Vec<String>,
    /// Lowercase item name -> ID
    items_to_id: FxHashMap<String, u16>,

    /// Move names indexed by move ID
    pub moves: Vec<String>,
    /// Lowercase move name -> ID
    moves_to_id: FxHashMap<String, u16>,

    /// Ability names indexed by ability ID
    pub abilities: Vec<String>,
    /// Lowercase ability name -> ID
    abilities_to_id: FxHashMap<String, u16>,

    /// Type names indexed by type ID
    pub types: Vec<String>,
    /// Lowercase type name -> ID
    types_to_id: FxHashMap<String, u16>,
}

impl GameStrings {
    /// Create a new empty GameStrings
    pub fn new() -> Self {
        Self::default()
    }

    /// Load game strings from a DSPRE project's expanded/textArchives folder
    pub fn load_from_dspre(
        project_path: impl AsRef<Path>,
        family: GameFamily,
        language: GameLanguage,
    ) -> std::io::Result<Self> {
        let text_archives_path = project_path.as_ref().join("expanded/textArchives");

        if !text_archives_path.exists() {
            return Ok(Self::new());
        }

        let bank_ids = TextBankIds::for_game(family, language);

        let mut gs = Self::new();

        if let Ok(names) = load_text_archive(&text_archives_path, bank_ids.species) {
            gs.species_to_id = build_lookup_table(&names);
            gs.species = names;
        }

        if let Ok(names) = load_text_archive(&text_archives_path, bank_ids.items) {
            gs.items_to_id = build_lookup_table(&names);
            gs.items = names;
        }

        if let Ok(names) = load_text_archive(&text_archives_path, bank_ids.moves) {
            gs.moves_to_id = build_lookup_table(&names);
            gs.moves = names;
        }

        if let Ok(names) = load_text_archive(&text_archives_path, bank_ids.abilities) {
            gs.abilities_to_id = build_lookup_table(&names);
            gs.abilities = names;
        }

        if let Ok(names) = load_text_archive(&text_archives_path, bank_ids.types) {
            gs.types_to_id = build_lookup_table(&names);
            gs.types = names;
        }

        Ok(gs)
    }

    /// Get species name by ID
    pub fn get_species_name(&self, id: u16) -> Option<&str> {
        self.species.get(id as usize).map(|s| s.as_str())
    }

    /// Get species ID by name (case-insensitive)
    pub fn get_species_id(&self, name: &str) -> Option<u16> {
        self.species_to_id.get(&normalize_name(name)).copied()
    }

    /// Get item name by ID
    pub fn get_item_name(&self, id: u16) -> Option<&str> {
        self.items.get(id as usize).map(|s| s.as_str())
    }

    /// Get item ID by name (case-insensitive)
    pub fn get_item_id(&self, name: &str) -> Option<u16> {
        self.items_to_id.get(&normalize_name(name)).copied()
    }

    /// Get move name by ID
    pub fn get_move_name(&self, id: u16) -> Option<&str> {
        self.moves.get(id as usize).map(|s| s.as_str())
    }

    /// Get move ID by name (case-insensitive)
    pub fn get_move_id(&self, name: &str) -> Option<u16> {
        self.moves_to_id.get(&normalize_name(name)).copied()
    }

    /// Get ability name by ID
    pub fn get_ability_name(&self, id: u16) -> Option<&str> {
        self.abilities.get(id as usize).map(|s| s.as_str())
    }

    /// Get ability ID by name (case-insensitive)
    pub fn get_ability_id(&self, name: &str) -> Option<u16> {
        self.abilities_to_id.get(&normalize_name(name)).copied()
    }

    /// Get type name by ID
    pub fn get_type_name(&self, id: u16) -> Option<&str> {
        self.types.get(id as usize).map(|s| s.as_str())
    }

    /// Get type ID by name (case-insensitive)
    pub fn get_type_id(&self, name: &str) -> Option<u16> {
        self.types_to_id.get(&normalize_name(name)).copied()
    }

    /// Check if any strings are loaded
    pub fn is_empty(&self) -> bool {
        self.species.is_empty()
            && self.items.is_empty()
            && self.moves.is_empty()
            && self.abilities.is_empty()
            && self.types.is_empty()
    }
}

/// Load a text archive JSON and extract all message strings
fn load_text_archive(base_path: &Path, bank_id: u16) -> std::io::Result<Vec<String>> {
    let json_path = base_path.join(format!("{:04}.json", bank_id));
    let content = std::fs::read_to_string(&json_path)?;

    let archive: ChatotTextArchive = serde_json::from_str(&content)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

    let mut names = Vec::with_capacity(archive.messages.len());

    for msg in archive.messages {
        // Prefer en_US, fall back to first available language
        let text = if let Some(content) = msg.lang_content.get("en_US") {
            extract_text(content)
        } else if let Some((_, content)) = msg.lang_content.iter().next() {
            extract_text(content)
        } else {
            String::new()
        };
        names.push(text);
    }

    Ok(names)
}

/// Extract plain text from MessageContent
fn extract_text(content: &MessageContent) -> String {
    match content {
        MessageContent::Single(s) => s.clone(),
        MessageContent::Multi(parts) => parts.join(""),
    }
}

/// Normalize a name for lookup (lowercase, remove spaces, strip accents)
fn normalize_name(name: &str) -> String {
    deunicode::deunicode(name)
        .to_lowercase()
        .replace([' ', '-'], "")
}

/// Build a lookup table from name -> index
fn build_lookup_table(names: &[String]) -> FxHashMap<String, u16> {
    let mut map = FxHashMap::default();
    for (i, name) in names.iter().enumerate() {
        let normalized = normalize_name(name);
        if !normalized.is_empty() && !map.contains_key(&normalized) {
            map.insert(normalized, i as u16);
        }
    }
    map
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_name() {
        assert_eq!(normalize_name("Bulbasaur"), "bulbasaur");
        assert_eq!(normalize_name("MR. MIME"), "mr.mime");
        assert_eq!(normalize_name("Poké Ball"), "pokeball");
        assert_eq!(normalize_name("Ho-Oh"), "hooh");
    }
}
