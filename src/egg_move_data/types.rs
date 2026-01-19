use serde::{Deserialize, Serialize};

pub const EGG_MOVE_TERMINATOR: u16 = 0xFFFF;
pub const EGG_MOVE_SPECIES_OFFSET: u16 = 20000;

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct EggMoveEntry {
    pub species_id: u16,
    pub move_ids: Vec<u16>,
}

impl EggMoveEntry {
    pub fn new(species_id: u16, move_ids: Vec<u16>) -> Self {
        Self {
            species_id,
            move_ids,
        }
    }

    pub fn byte_size(&self) -> usize {
        2 + (2 * self.move_ids.len())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct EggMoveData {
    pub entries: Vec<EggMoveEntry>,
}

impl EggMoveData {
    pub fn get_by_species(&self, species_id: u16) -> Option<&EggMoveEntry> {
        self.entries.iter().find(|e| e.species_id == species_id)
    }

    pub fn can_learn(&self, species_id: u16, move_id: u16) -> bool {
        self.get_by_species(species_id)
            .map(|e| e.move_ids.contains(&move_id))
            .unwrap_or(false)
    }

    pub fn total_byte_size(&self) -> usize {
        let entries_size: usize = self.entries.iter().map(|e| e.byte_size()).sum();
        entries_size + 2
    }
}
