use binrw::{BinRead, BinWrite};

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, BinRead, BinWrite)]
#[brw(little)]
pub struct PersonalData {
    pub hp: u8,
    pub attack: u8,
    pub defense: u8,
    pub speed: u8,
    pub sp_attack: u8,
    pub sp_defense: u8,
    pub type1: u8,
    pub type2: u8,
    pub catch_rate: u8,
    pub base_exp: u8,
    pub ev_yield: u16,
    pub item1: u16,
    pub item2: u16,
    pub gender_ratio: u8,
    pub egg_cycles: u8,
    pub base_friendship: u8,
    pub growth_rate: u8,
    pub egg_group1: u8,
    pub egg_group2: u8,
    pub ability1: u8,
    pub ability2: u8,
    pub safari_flee_rate: u8,
    pub color_flip: u8,
    #[brw(pad_before = 2)]
    pub tm_compatibility: [u8; 16],
}

impl PersonalData {
    pub fn ev_yield_hp(&self) -> u8 {
        (self.ev_yield & 0b11) as u8
    }

    pub fn ev_yield_attack(&self) -> u8 {
        ((self.ev_yield >> 2) & 0b11) as u8
    }

    pub fn ev_yield_defense(&self) -> u8 {
        ((self.ev_yield >> 4) & 0b11) as u8
    }

    pub fn ev_yield_speed(&self) -> u8 {
        ((self.ev_yield >> 6) & 0b11) as u8
    }

    pub fn ev_yield_sp_attack(&self) -> u8 {
        ((self.ev_yield >> 8) & 0b11) as u8
    }

    pub fn ev_yield_sp_defense(&self) -> u8 {
        ((self.ev_yield >> 10) & 0b11) as u8
    }

    pub fn can_learn_tm(&self, tm_index: u8) -> bool {
        if tm_index >= 128 {
            return false;
        }
        let byte_idx = (tm_index / 8) as usize;
        let bit_idx = tm_index % 8;
        (self.tm_compatibility[byte_idx] & (1 << bit_idx)) != 0
    }
}
