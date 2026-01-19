use binrw::{BinRead, BinWrite};
use bitflags::bitflags;
use modular_bitfield::prelude::*;
use serde::{Deserialize, Serialize};

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
    pub fn from_u8(value: u8) -> Self {
        match value {
            0 => FieldPocket::Items,
            1 => FieldPocket::Medicine,
            2 => FieldPocket::Balls,
            3 => FieldPocket::TmHms,
            4 => FieldPocket::Berries,
            5 => FieldPocket::Mail,
            6 => FieldPocket::BattleItems,
            7 => FieldPocket::KeyItems,
            _ => FieldPocket::Items,
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
#[derive(Debug, Clone, Copy, Default, BinRead, BinWrite)]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, BinRead, BinWrite)]
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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ItemPartyUseParam {
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
    pub atk_stages: u8,

    // Byte 2: Defense and SpAtk stages
    pub def_stages: u8,
    pub spatk_stages: u8,

    // Byte 3: SpDef and Speed stages
    pub spdef_stages: u8,
    pub speed_stages: u8,

    // Byte 4: Accuracy, Crit stages, PP flags
    pub acc_stages: u8,
    pub crit_stages: u8,
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

    // Bytes 7-17: Raw values (11 bytes)
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

impl Default for ItemPartyUseParam {
    fn default() -> Self {
        Self {
            heal_sleep: false,
            heal_poison: false,
            heal_burn: false,
            heal_freeze: false,
            heal_paralysis: false,
            heal_confusion: false,
            heal_attract: false,
            guard_spec: false,
            revive: false,
            revive_all: false,
            level_up: false,
            evolve: false,
            atk_stages: 0,
            def_stages: 0,
            spatk_stages: 0,
            spdef_stages: 0,
            speed_stages: 0,
            acc_stages: 0,
            crit_stages: 0,
            pp_up: false,
            pp_max: false,
            pp_restore: false,
            pp_restore_all: false,
            hp_restore: false,
            give_hp_evs: false,
            give_atk_evs: false,
            give_def_evs: false,
            give_speed_evs: false,
            give_spatk_evs: false,
            give_spdef_evs: false,
            give_friendship_low: false,
            give_friendship_med: false,
            give_friendship_high: false,
            hp_evs: 0,
            atk_evs: 0,
            def_evs: 0,
            speed_evs: 0,
            spatk_evs: 0,
            spdef_evs: 0,
            hp_restored: 0,
            pp_restored: 0,
            friendship_low: 0,
            friendship_med: 0,
            friendship_high: 0,
        }
    }
}

impl ItemPartyUseParam {
    /// Convert from binary representation
    pub fn from_parts(flags: ItemPartyUseFlagsBits, values: ItemPartyUseValues) -> Self {
        Self {
            heal_sleep: flags.heal_sleep(),
            heal_poison: flags.heal_poison(),
            heal_burn: flags.heal_burn(),
            heal_freeze: flags.heal_freeze(),
            heal_paralysis: flags.heal_paralysis(),
            heal_confusion: flags.heal_confusion(),
            heal_attract: flags.heal_attract(),
            guard_spec: flags.guard_spec(),
            revive: flags.revive(),
            revive_all: flags.revive_all(),
            level_up: flags.level_up(),
            evolve: flags.evolve(),
            atk_stages: flags.atk_stages(),
            def_stages: flags.def_stages(),
            spatk_stages: flags.spatk_stages(),
            spdef_stages: flags.spdef_stages(),
            speed_stages: flags.speed_stages(),
            acc_stages: flags.acc_stages(),
            crit_stages: flags.crit_stages(),
            pp_up: flags.pp_up(),
            pp_max: flags.pp_max(),
            pp_restore: flags.pp_restore(),
            pp_restore_all: flags.pp_restore_all(),
            hp_restore: flags.hp_restore(),
            give_hp_evs: flags.give_hp_evs(),
            give_atk_evs: flags.give_atk_evs(),
            give_def_evs: flags.give_def_evs(),
            give_speed_evs: flags.give_speed_evs(),
            give_spatk_evs: flags.give_spatk_evs(),
            give_spdef_evs: flags.give_spdef_evs(),
            give_friendship_low: flags.give_friendship_low(),
            give_friendship_med: flags.give_friendship_med(),
            give_friendship_high: flags.give_friendship_high(),
            hp_evs: values.hp_evs,
            atk_evs: values.atk_evs,
            def_evs: values.def_evs,
            speed_evs: values.speed_evs,
            spatk_evs: values.spatk_evs,
            spdef_evs: values.spdef_evs,
            hp_restored: values.hp_restored,
            pp_restored: values.pp_restored,
            friendship_low: values.friendship_low,
            friendship_med: values.friendship_med,
            friendship_high: values.friendship_high,
        }
    }

    /// Convert to binary representation
    pub fn to_parts(&self) -> (ItemPartyUseFlagsBits, ItemPartyUseValues) {
        let flags = ItemPartyUseFlagsBits::new()
            .with_heal_sleep(self.heal_sleep)
            .with_heal_poison(self.heal_poison)
            .with_heal_burn(self.heal_burn)
            .with_heal_freeze(self.heal_freeze)
            .with_heal_paralysis(self.heal_paralysis)
            .with_heal_confusion(self.heal_confusion)
            .with_heal_attract(self.heal_attract)
            .with_guard_spec(self.guard_spec)
            .with_revive(self.revive)
            .with_revive_all(self.revive_all)
            .with_level_up(self.level_up)
            .with_evolve(self.evolve)
            .with_atk_stages(self.atk_stages)
            .with_def_stages(self.def_stages)
            .with_spatk_stages(self.spatk_stages)
            .with_spdef_stages(self.spdef_stages)
            .with_speed_stages(self.speed_stages)
            .with_acc_stages(self.acc_stages)
            .with_crit_stages(self.crit_stages)
            .with_pp_up(self.pp_up)
            .with_pp_max(self.pp_max)
            .with_pp_restore(self.pp_restore)
            .with_pp_restore_all(self.pp_restore_all)
            .with_hp_restore(self.hp_restore)
            .with_give_hp_evs(self.give_hp_evs)
            .with_give_atk_evs(self.give_atk_evs)
            .with_give_def_evs(self.give_def_evs)
            .with_give_speed_evs(self.give_speed_evs)
            .with_give_spatk_evs(self.give_spatk_evs)
            .with_give_spdef_evs(self.give_spdef_evs)
            .with_give_friendship_low(self.give_friendship_low)
            .with_give_friendship_med(self.give_friendship_med)
            .with_give_friendship_high(self.give_friendship_high);

        let values = ItemPartyUseValues {
            hp_evs: self.hp_evs,
            atk_evs: self.atk_evs,
            def_evs: self.def_evs,
            speed_evs: self.speed_evs,
            spatk_evs: self.spatk_evs,
            spdef_evs: self.spdef_evs,
            hp_restored: self.hp_restored,
            pp_restored: self.pp_restored,
            friendship_low: self.friendship_low,
            friendship_med: self.friendship_med,
            friendship_high: self.friendship_high,
        };

        (flags, values)
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
    ) -> Self {
        Self {
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
            field_pocket: FieldPocket::from_u8(bitfield.field_pocket()),
            battle_pocket: BattlePocket::from_bits_truncate(bitfield.battle_pocket()),
            field_use_func,
            battle_use_func,
            party_use,
            party_use_param,
        }
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
