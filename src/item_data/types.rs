use binrw::{BinRead, BinWrite};
use bitflags::bitflags;
use modular_bitfield::prelude::*;
use serde::{Deserialize, Serialize};
use std::io;

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
    pub struct BattlePocket: u8 {
        const POKE_BALLS = 1 << 0;
        const BATTLE_ITEMS = 1 << 1;
        const HP_RESTORE = 1 << 2;
        const STATUS_HEALERS = 1 << 3;
        const PP_RESTORE = 1 << 4;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, BinRead, BinWrite)]
#[brw(repr = u8)]
pub enum FieldPocket {
    #[default]
    Items = 0,
    Medicine = 1,
    Balls = 2,
    TmHms = 3,
    Berries = 4,
    Mail = 5,
    BattleItems = 6,
    KeyItems = 7,
}

impl FieldPocket {
    pub fn try_from_u8(value: u8) -> Option<Self> {
        match value {
            0 => Some(FieldPocket::Items),
            1 => Some(FieldPocket::Medicine),
            2 => Some(FieldPocket::Balls),
            3 => Some(FieldPocket::TmHms),
            4 => Some(FieldPocket::Berries),
            5 => Some(FieldPocket::Mail),
            6 => Some(FieldPocket::BattleItems),
            7 => Some(FieldPocket::KeyItems),
            _ => None,
        }
    }
}

/// Packed bitfield for item properties (16 bits / 2 bytes)
/// Layout: [natural_gift_type:5][prevent_toss:1][is_selectable:1][field_pocket:4][battle_pocket:5]
#[bitfield(bits = 16)]
#[derive(Debug, Clone, Copy, Default, BinRead, BinWrite)]
#[br(map = Self::from_bytes)]
#[bw(map = |s: &Self| s.into_bytes())]
#[allow(unused_parens)]
pub struct ItemBitfield {
    pub natural_gift_type: B5,
    pub prevent_toss: bool,
    pub is_selectable: bool,
    pub field_pocket: B4,
    pub battle_pocket: B5,
}

/// Packed bitfield for party use flags (7 bytes / 56 bits)
#[bitfield(bits = 56)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, BinRead, BinWrite)]
#[br(map = Self::from_bytes)]
#[bw(map = |s: &Self| s.into_bytes())]
#[allow(unused_parens)]
pub struct ItemPartyUseFlagsBits {
    // Byte 0: Status healing flags
    pub heal_sleep: bool,
    pub heal_poison: bool,
    pub heal_burn: bool,
    pub heal_freeze: bool,
    pub heal_paralysis: bool,
    pub heal_confusion: bool,
    pub heal_attract: bool,
    pub guard_spec: bool,

    // Byte 1: Revival/level flags + attack stages
    pub revive: bool,
    pub revive_all: bool,
    pub level_up: bool,
    pub evolve: bool,
    pub atk_stages: B4,

    // Byte 2: Defense and SpAtk stages
    pub def_stages: B4,
    pub spatk_stages: B4,

    // Byte 3: SpDef and Speed stages
    pub spdef_stages: B4,
    pub speed_stages: B4,

    // Byte 4: Accuracy, Crit stages, PP flags
    pub acc_stages: B4,
    pub crit_stages: B2,
    pub pp_up: bool,
    pub pp_max: bool,

    // Byte 5: PP/HP restore flags, EV boost flags (part 1)
    pub pp_restore: bool,
    pub pp_restore_all: bool,
    pub hp_restore: bool,
    pub give_hp_evs: bool,
    pub give_atk_evs: bool,
    pub give_def_evs: bool,
    pub give_speed_evs: bool,
    pub give_spatk_evs: bool,

    // Byte 6: EV boost flags (part 2), Friendship flags
    pub give_spdef_evs: bool,
    pub give_friendship_low: bool,
    pub give_friendship_med: bool,
    pub give_friendship_high: bool,
    #[skip]
    __unused: B4,
}

/// Scalar values following the party use flags (11 bytes)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, BinRead, BinWrite)]
#[brw(little)]
pub struct ItemPartyUseValues {
    pub hp_evs: i8,
    pub atk_evs: i8,
    pub def_evs: i8,
    pub speed_evs: i8,
    pub spatk_evs: i8,
    pub spdef_evs: i8,
    pub hp_restored: u8,
    pub pp_restored: u8,
    pub friendship_low: i8,
    pub friendship_med: i8,
    pub friendship_high: i8,
}

/// Party use parameters (18 bytes: 7 bytes flags + 11 bytes values)
///
/// This keeps the binary shape as the canonical model to avoid duplicating
/// each flag/value in a second flattened struct representation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct ItemPartyUseParam {
    #[serde(with = "item_party_use_flags_serde")]
    flags: ItemPartyUseFlagsBits,
    values: ItemPartyUseValues,
}

impl ItemPartyUseParam {
    /// Convert from binary representation
    pub fn from_parts(flags: ItemPartyUseFlagsBits, values: ItemPartyUseValues) -> Self {
        Self { flags, values }
    }

    /// Convert to binary representation
    pub fn to_parts(&self) -> (ItemPartyUseFlagsBits, ItemPartyUseValues) {
        (self.flags, self.values)
    }
}

mod item_party_use_flags_serde {
    use super::ItemPartyUseFlagsBits;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub fn serialize<S>(flags: &ItemPartyUseFlagsBits, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        flags.into_bytes().serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<ItemPartyUseFlagsBits, D::Error>
    where
        D: Deserializer<'de>,
    {
        let bytes = <[u8; 7]>::deserialize(deserializer)?;
        Ok(ItemPartyUseFlagsBits::from_bytes(bytes))
    }
}

/// Item data structure (34 bytes total)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ItemData {
    pub price: u16,
    pub hold_effect: u8,
    pub hold_effect_param: u8,
    pub pluck_effect: u8,
    pub fling_effect: u8,
    pub fling_power: u8,
    pub natural_gift_power: u8,
    // Bitfield values (from 16-bit packed field)
    pub natural_gift_type: u8,
    pub prevent_toss: bool,
    pub is_selectable: bool,
    pub field_pocket: FieldPocket,
    pub battle_pocket: BattlePocket,
    // Remaining fields
    pub field_use_func: u8,
    pub battle_use_func: u8,
    pub party_use: u8,
    pub party_use_param: ItemPartyUseParam,
}

impl ItemData {
    /// Convert from binary representation parts
    // Explicit positional mapping from the binary layout keeps decode paths straightforward.
    #[allow(clippy::too_many_arguments)]
    pub fn from_parts(
        price: u16,
        hold_effect: u8,
        hold_effect_param: u8,
        pluck_effect: u8,
        fling_effect: u8,
        fling_power: u8,
        natural_gift_power: u8,
        bitfield: ItemBitfield,
        field_use_func: u8,
        battle_use_func: u8,
        party_use: u8,
        party_use_param: ItemPartyUseParam,
    ) -> io::Result<Self> {
        let field_pocket_bits = bitfield.field_pocket();
        let field_pocket = FieldPocket::try_from_u8(field_pocket_bits).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "Invalid field pocket bits '{}' in item bitfield (expected 0..=7)",
                    field_pocket_bits
                ),
            )
        })?;
        let battle_pocket_bits = bitfield.battle_pocket();
        let battle_pocket = BattlePocket::from_bits(battle_pocket_bits).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "Invalid battle pocket bits '{}' in item bitfield",
                    battle_pocket_bits
                ),
            )
        })?;

        Ok(Self {
            price,
            hold_effect,
            hold_effect_param,
            pluck_effect,
            fling_effect,
            fling_power,
            natural_gift_power,
            natural_gift_type: bitfield.natural_gift_type(),
            prevent_toss: bitfield.prevent_toss(),
            is_selectable: bitfield.is_selectable(),
            field_pocket,
            battle_pocket,
            field_use_func,
            battle_use_func,
            party_use,
            party_use_param,
        })
    }

    /// Convert to binary bitfield
    pub fn to_bitfield(&self) -> ItemBitfield {
        ItemBitfield::new()
            .with_natural_gift_type(self.natural_gift_type)
            .with_prevent_toss(self.prevent_toss)
            .with_is_selectable(self.is_selectable)
            .with_field_pocket(self.field_pocket as u8)
            .with_battle_pocket(self.battle_pocket.bits())
    }
}
