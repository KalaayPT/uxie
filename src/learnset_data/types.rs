use serde::{Deserialize, Serialize};

pub const LEARNSET_TERMINATOR: u16 = 0xFFFF;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct LearnsetEntry {
    pub move_id: u16,
    pub level: u8,
}

impl LearnsetEntry {
    pub fn new(move_id: u16, level: u8) -> Self {
        Self { move_id, level }
    }

    pub fn from_packed(packed: u16) -> Self {
        let move_id = packed & 0x1FF;
        let level = ((packed >> 9) & 0x7F) as u8;
        Self { move_id, level }
    }

    pub fn to_packed(&self) -> u16 {
        (self.move_id & 0x1FF) | ((self.level as u16 & 0x7F) << 9)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct LearnsetData {
    pub entries: Vec<LearnsetEntry>,
}

impl LearnsetData {
    pub fn moves_at_level(&self, level: u8) -> impl Iterator<Item = &LearnsetEntry> {
        self.entries.iter().filter(move |e| e.level <= level)
    }

    pub fn moves_learned_at(&self, level: u8) -> impl Iterator<Item = &LearnsetEntry> {
        self.entries.iter().filter(move |e| e.level == level)
    }
}
