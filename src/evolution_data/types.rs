use binrw::{BinRead, BinWrite};
use serde::{Deserialize, Serialize};

pub const EVOLUTIONS_PER_SPECIES: usize = 7;
pub const EVOLUTION_ENTRY_SIZE: usize = 6;
pub const EVOLUTION_FILE_SIZE: usize = EVOLUTIONS_PER_SPECIES * EVOLUTION_ENTRY_SIZE;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u16)]
pub enum EvolutionMethod {
    None = 0,
    Happiness = 1,
    HappinessDay = 2,
    HappinessNight = 3,
    LevelUp = 4,
    Trade = 5,
    TradeWithItem = 6,
    UseItem = 7,
    LevelUpAtkGtDef = 8,
    LevelUpAtkEqDef = 9,
    LevelUpDefGtAtk = 10,
    LevelUpPersonalityLow = 11,
    LevelUpPersonalityHigh = 12,
    LevelUpSpawnPokemon = 13,
    LevelUpSpawnEmptySlot = 14,
    Beauty = 15,
    UseItemMale = 16,
    UseItemFemale = 17,
    LevelUpWithItem = 18,
    LevelUpWithMoveType = 19,
    LevelUpWithPartyMember = 20,
    LevelUpMale = 21,
    LevelUpFemale = 22,
    LevelUpAtLocation = 23,
    LevelUpAtMtCoronet = 24,
    LevelUpNearMossRock = 25,
    LevelUpNearIceRock = 26,
    Unknown(u16),
}

impl From<u16> for EvolutionMethod {
    fn from(v: u16) -> Self {
        match v {
            0 => Self::None,
            1 => Self::Happiness,
            2 => Self::HappinessDay,
            3 => Self::HappinessNight,
            4 => Self::LevelUp,
            5 => Self::Trade,
            6 => Self::TradeWithItem,
            7 => Self::UseItem,
            8 => Self::LevelUpAtkGtDef,
            9 => Self::LevelUpAtkEqDef,
            10 => Self::LevelUpDefGtAtk,
            11 => Self::LevelUpPersonalityLow,
            12 => Self::LevelUpPersonalityHigh,
            13 => Self::LevelUpSpawnPokemon,
            14 => Self::LevelUpSpawnEmptySlot,
            15 => Self::Beauty,
            16 => Self::UseItemMale,
            17 => Self::UseItemFemale,
            18 => Self::LevelUpWithItem,
            19 => Self::LevelUpWithMoveType,
            20 => Self::LevelUpWithPartyMember,
            21 => Self::LevelUpMale,
            22 => Self::LevelUpFemale,
            23 => Self::LevelUpAtLocation,
            24 => Self::LevelUpAtMtCoronet,
            25 => Self::LevelUpNearMossRock,
            26 => Self::LevelUpNearIceRock,
            n => Self::Unknown(n),
        }
    }
}

impl std::fmt::Display for EvolutionMethod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::None => write!(f, "None"),
            Self::Happiness => write!(f, "Happiness"),
            Self::HappinessDay => write!(f, "HappinessDay"),
            Self::HappinessNight => write!(f, "HappinessNight"),
            Self::LevelUp => write!(f, "LevelUp"),
            Self::Trade => write!(f, "Trade"),
            Self::TradeWithItem => write!(f, "TradeWithItem"),
            Self::UseItem => write!(f, "UseItem"),
            Self::LevelUpAtkGtDef => write!(f, "LevelUpAtkGtDef"),
            Self::LevelUpAtkEqDef => write!(f, "LevelUpAtkEqDef"),
            Self::LevelUpDefGtAtk => write!(f, "LevelUpDefGtAtk"),
            Self::LevelUpPersonalityLow => write!(f, "LevelUpPersonalityLow"),
            Self::LevelUpPersonalityHigh => write!(f, "LevelUpPersonalityHigh"),
            Self::LevelUpSpawnPokemon => write!(f, "LevelUpSpawnPokemon"),
            Self::LevelUpSpawnEmptySlot => write!(f, "LevelUpSpawnEmptySlot"),
            Self::Beauty => write!(f, "Beauty"),
            Self::UseItemMale => write!(f, "UseItemMale"),
            Self::UseItemFemale => write!(f, "UseItemFemale"),
            Self::LevelUpWithItem => write!(f, "LevelUpWithItem"),
            Self::LevelUpWithMoveType => write!(f, "LevelUpWithMoveType"),
            Self::LevelUpWithPartyMember => write!(f, "LevelUpWithPartyMember"),
            Self::LevelUpMale => write!(f, "LevelUpMale"),
            Self::LevelUpFemale => write!(f, "LevelUpFemale"),
            Self::LevelUpAtLocation => write!(f, "LevelUpAtLocation"),
            Self::LevelUpAtMtCoronet => write!(f, "LevelUpAtMtCoronet"),
            Self::LevelUpNearMossRock => write!(f, "LevelUpNearMossRock"),
            Self::LevelUpNearIceRock => write!(f, "LevelUpNearIceRock"),
            Self::Unknown(n) => write!(f, "Unknown({})", n),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, BinRead, BinWrite, Default)]
#[brw(little)]
pub struct EvolutionEntry {
    pub method: u16,
    pub param: u16,
    pub target_species: u16,
}

impl EvolutionEntry {
    pub fn is_empty(&self) -> bool {
        self.method == 0 && self.param == 0 && self.target_species == 0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, BinRead, BinWrite)]
#[brw(little)]
pub struct EvolutionData {
    pub entries: [EvolutionEntry; EVOLUTIONS_PER_SPECIES],
}

impl Default for EvolutionData {
    fn default() -> Self {
        Self {
            entries: [EvolutionEntry::default(); EVOLUTIONS_PER_SPECIES],
        }
    }
}

impl EvolutionData {
    pub fn active_evolutions(&self) -> impl Iterator<Item = &EvolutionEntry> {
        self.entries.iter().filter(|e| !e.is_empty())
    }
}
