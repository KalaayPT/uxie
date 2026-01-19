use bitflags::bitflags;
use serde::{Deserialize, Serialize};

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
    pub struct TrainerFlags: u8 {
        const HAS_MOVES = 0b01;
        const HAS_ITEMS = 0b10;
    }
}

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
    pub struct AiFlags: u32 {
        const BASIC            = 1 << 0;
        const EVAL_ATTACK      = 1 << 1;
        const EXPERT           = 1 << 2;
        const SETUP_FIRST_TURN = 1 << 3;
        const RISKY            = 1 << 4;
        const DAMAGE_PRIORITY  = 1 << 5;
        const BATON_PASS       = 1 << 6;
        const TAG_STRATEGY     = 1 << 7;
        const CHECK_HP         = 1 << 8;
        const WEATHER          = 1 << 9;
        const HARRASSMENT      = 1 << 10;
        const ROAMING_POKEMON  = 1 << 29;
        const SAFARI           = 1 << 30;
        const CATCH_TUTORIAL   = 1 << 31;
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TrainerProperties {
    pub flags: TrainerFlags,
    pub trainer_class: u8,
    pub unknown: u8,
    pub party_count: u8,
    pub items: [u16; 4],
    pub ai_flags: AiFlags,
    pub double_battle: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PartyPokemon {
    pub difficulty: u8,
    pub gender_ability: u8,
    pub level: u16,
    pub species: u16,
    pub form: u8,
    pub held_item: Option<u16>,
    pub moves: Option<[u16; 4]>,
    pub ball_seal: Option<u16>,
}

impl PartyPokemon {
    pub fn species_id(&self) -> u16 {
        self.species & 0x3FF
    }

    pub fn form_id(&self) -> u8 {
        self.form
    }

    pub fn ability_slot(&self) -> u8 {
        self.gender_ability & 0x0F
    }

    pub fn gender(&self) -> u8 {
        (self.gender_ability >> 4) & 0x0F
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TrainerData {
    pub properties: TrainerProperties,
    pub party: Vec<PartyPokemon>,
}
