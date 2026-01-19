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
}
