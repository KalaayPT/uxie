//! Game version detection and configuration

use serde::{Deserialize, Serialize};

/// Game language/region
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum GameLanguage {
    /// English (USA/Europe)
    #[default]
    English,
    /// Japanese
    Japanese,
    /// French
    French,
    /// German
    German,
    /// Italian
    Italian,
    /// Spanish
    Spanish,
    /// Korean
    Korean,
}

/// Semantic identity of a ROM, derived from its [`crate::rom_header::RomHeader`].
///
/// `RomHeader` carries the raw header bytes; `RomIdentity` is the resolved
/// game / family / region / language that consumers (DSPRE, decomp tooling)
/// actually need. This is the stable contract that crosses the FFI boundary.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RomIdentity {
    pub game_code: String,
    pub game: Game,
    pub family: GameFamily,
    pub region: Option<String>,
    pub language: GameLanguage,
    pub rom_version: u8,
}

/// Game family grouping
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum GameFamily {
    /// Diamond and Pearl
    DP,
    /// Platinum
    Platinum,
    /// HeartGold and SoulSilver
    HGSS,
}

/// Specific game version
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Game {
    Diamond,
    Pearl,
    Platinum,
    HeartGold,
    SoulSilver,
}

impl Game {
    /// Get the game family for this game
    pub fn family(&self) -> GameFamily {
        match self {
            Game::Diamond | Game::Pearl => GameFamily::DP,
            Game::Platinum => GameFamily::Platinum,
            Game::HeartGold | Game::SoulSilver => GameFamily::HGSS,
        }
    }

    /// Get the map header size in bytes for this game
    pub fn map_header_size(&self) -> usize {
        // All Gen 4 games use 24-byte headers in binary format
        24
    }
}

impl GameFamily {
    /// Returns a representative game for this family.
    ///
    /// Use this only when a specific [`Game`] is required but only a family is known
    /// (e.g. fallback workspace construction). Prefer [`Workspace::open`] when possible.
    pub fn default_game(self) -> Game {
        match self {
            GameFamily::DP => Game::Diamond,
            GameFamily::Platinum => Game::Platinum,
            GameFamily::HGSS => Game::HeartGold,
        }
    }

    /// Get the map header size in bytes for this game family
    pub fn map_header_size(&self) -> usize {
        24
    }

    /// Get the null encounter file ID for this game family
    pub fn null_encounter_id(&self) -> u16 {
        match self {
            GameFamily::DP | GameFamily::Platinum => 0xFFFF,
            GameFamily::HGSS => 0xFF,
        }
    }
}

impl GameLanguage {
    pub fn from_region_code(code: char) -> Self {
        match code {
            'J' => GameLanguage::Japanese,
            'E' => GameLanguage::English,
            'P' => GameLanguage::English, // Europe uses English text banks
            'F' => GameLanguage::French,
            'D' => GameLanguage::German,
            'I' => GameLanguage::Italian,
            'S' => GameLanguage::Spanish,
            'K' => GameLanguage::Korean,
            _ => GameLanguage::English,
        }
    }

    pub fn is_japanese(&self) -> bool {
        matches!(self, GameLanguage::Japanese)
    }

    /// Return the DSPRE/Decomp locale key for this language.
    pub fn locale_key(&self) -> &'static str {
        match self {
            GameLanguage::English => "en_US",
            GameLanguage::Japanese => "ja_JP",
            GameLanguage::French => "fr_FR",
            GameLanguage::German => "de_DE",
            GameLanguage::Italian => "it_IT",
            GameLanguage::Spanish => "es_ES",
            GameLanguage::Korean => "ko_KR",
        }
    }
}
