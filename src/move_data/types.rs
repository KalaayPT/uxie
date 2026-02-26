use binrw::{BinRead, BinWrite};
use bitflags::bitflags;
use serde::{Deserialize, Serialize};
use std::io;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, BinRead, BinWrite)]
#[brw(repr = u8)]
pub enum MoveSplit {
    Physical = 0,
    Special = 1,
    Status = 2,
}

impl From<u8> for MoveSplit {
    fn from(v: u8) -> Self {
        match v {
            0 => MoveSplit::Physical,
            1 => MoveSplit::Special,
            _ => MoveSplit::Status,
        }
    }
}

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
    pub struct MoveFlags: u8 {
        const MAKES_CONTACT = 0b0000_0001;
        const AFFECTED_BY_PROTECT = 0b0000_0010;
        const AFFECTED_BY_MAGIC_COAT = 0b0000_0100;
        const AFFECTED_BY_SNATCH = 0b0000_1000;
        const USABLE_BY_MIRROR_MOVE = 0b0001_0000;
        const AFFECTED_BY_KINGS_ROCK = 0b0010_0000;
    }
}

impl MoveFlags {
    pub fn from_bits_strict(bits: u8) -> io::Result<Self> {
        Self::from_bits(bits).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Invalid move flags bits '{}'", bits),
            )
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, BinRead, BinWrite)]
#[brw(little)]
pub struct MoveData {
    pub battle_effect: u16,
    pub split: MoveSplit,
    pub power: u8,
    pub move_type: u8,
    pub accuracy: u8,
    pub pp: u8,
    pub side_effect_chance: u8,
    pub target: u16,
    pub priority: i8,
    #[br(try_map = MoveFlags::from_bits_strict)]
    #[bw(map = |f: &MoveFlags| f.bits())]
    pub flags: MoveFlags,
    pub contest_appeal: u8,
    pub contest_condition: u8,
}
