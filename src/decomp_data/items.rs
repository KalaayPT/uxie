//! Item data parser for decompilation CSV format
//!
//! Parses `res/items/pl_item_data.csv` and converts to `ItemData`.

use super::util::resolve_required_constant;
use crate::item_data::{
    BattlePocket, FieldPocket, ItemData, ItemPartyUseFlagsBits, ItemPartyUseParam,
    ItemPartyUseValues,
};
use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::Path;

// Mirrors decomp CSV schema one-to-one; many boolean switches are intentional.
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone)]
pub struct DecompItemData {
    pub name: String,
    pub price: u16,
    pub hold_effect: String,
    pub hold_effect_param: u8,
    pub pluck_effect: u8,
    pub fling_effect: u8,
    pub fling_power: u8,
    pub natural_gift_power: u8,
    pub natural_gift_type: u8,
    pub prevent_toss: bool,
    pub selectable: bool,
    pub field_pocket: String,
    pub battle_pocket: String,
    pub field_use_func: String,
    pub battle_use_func: u8,
    pub party_use: u8,
    pub heal_sleep: bool,
    pub heal_poison: bool,
    pub heal_burn: bool,
    pub heal_freeze: bool,
    pub heal_paralysis: bool,
    pub heal_confusion: bool,
    pub heal_attract: bool,
    pub guard_spec: bool,
    pub revive: bool,
    pub revive_all: bool,
    pub level_up: bool,
    pub evolve: bool,
    pub atk_stages: u8,
    pub def_stages: u8,
    pub spatk_stages: u8,
    pub spdef_stages: u8,
    pub speed_stages: u8,
    pub acc_stages: u8,
    pub crit_stages: u8,
    pub pp_up: bool,
    pub pp_max: bool,
    pub pp_restore: bool,
    pub pp_restore_all: bool,
    pub hp_restore: bool,
    pub give_hp_evs: bool,
    pub give_atk_evs: bool,
    pub give_def_evs: bool,
    pub give_speed_evs: bool,
    pub give_spatk_evs: bool,
    pub give_spdef_evs: bool,
    pub give_friendship_low: bool,
    pub give_friendship_med: bool,
    pub give_friendship_high: bool,
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

impl DecompItemData {
    /// Convert decomp item data into binary [`ItemData`].
    ///
    /// The `resolve_constant` callback supplies numeric values for symbolic
    /// names used by decomp CSV fields.
    ///
    /// Typical usage is passing through workspace symbol resolution:
    /// `to_item_data(|name| workspace.resolve_constant(name))`.
    ///
    /// Returns `InvalidData` when any required constant cannot be resolved.
    pub fn to_item_data<F>(&self, resolve_constant: F) -> io::Result<ItemData>
    where
        F: Fn(&str) -> Option<i64>,
    {
        let hold_effect =
            resolve_required_constant(&resolve_constant, &self.hold_effect, "holdEffect", "item")?
                as u8;

        let field_pocket = parse_field_pocket(&self.field_pocket)?;
        let battle_pocket = parse_battle_pocket(&self.battle_pocket)?;

        let field_use_func = resolve_required_constant(
            &resolve_constant,
            &self.field_use_func,
            "fieldUseFunc",
            "item",
        )? as u8;

        let party_use_flags = ItemPartyUseFlagsBits::new()
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
        let party_use_values = ItemPartyUseValues {
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
        let party_use_param = ItemPartyUseParam::from_parts(party_use_flags, party_use_values);

        Ok(ItemData {
            price: self.price,
            hold_effect,
            hold_effect_param: self.hold_effect_param,
            pluck_effect: self.pluck_effect,
            fling_effect: self.fling_effect,
            fling_power: self.fling_power,
            natural_gift_power: self.natural_gift_power,
            natural_gift_type: self.natural_gift_type,
            prevent_toss: self.prevent_toss,
            is_selectable: self.selectable,
            field_pocket,
            battle_pocket,
            field_use_func,
            battle_use_func: self.battle_use_func,
            party_use: self.party_use,
            party_use_param,
        })
    }
}

fn parse_field_pocket(value: &str) -> io::Result<FieldPocket> {
    match value {
        "POCKET_ITEMS" => Ok(FieldPocket::Items),
        "POCKET_MEDICINE" => Ok(FieldPocket::Medicine),
        "POCKET_BALLS" => Ok(FieldPocket::Balls),
        "POCKET_TM_HMS" => Ok(FieldPocket::TmHms),
        "POCKET_BERRIES" => Ok(FieldPocket::Berries),
        "POCKET_MAIL" => Ok(FieldPocket::Mail),
        "POCKET_BATTLE_ITEMS" => Ok(FieldPocket::BattleItems),
        "POCKET_KEY_ITEMS" => Ok(FieldPocket::KeyItems),
        _ => match value.parse::<u8>() {
            Ok(0) => Ok(FieldPocket::Items),
            Ok(1) => Ok(FieldPocket::Medicine),
            Ok(2) => Ok(FieldPocket::Balls),
            Ok(3) => Ok(FieldPocket::TmHms),
            Ok(4) => Ok(FieldPocket::Berries),
            Ok(5) => Ok(FieldPocket::Mail),
            Ok(6) => Ok(FieldPocket::BattleItems),
            Ok(7) => Ok(FieldPocket::KeyItems),
            Ok(other) => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Invalid numeric field pocket '{}' (expected 0..=7)", other),
            )),
            Err(_) => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Invalid field pocket '{}'", value),
            )),
        },
    }
}

fn parse_battle_pocket(value: &str) -> io::Result<BattlePocket> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Ok(BattlePocket::empty());
    }

    if let Ok(bits) = trimmed.parse::<u8>() {
        return BattlePocket::from_bits(bits).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "Invalid numeric battle pocket '{}' (contains unsupported flag bits)",
                    bits
                ),
            )
        });
    }

    let mut battle_pocket = BattlePocket::empty();
    for flag in trimmed.split('|').map(str::trim) {
        match flag {
            "BATTLE_POCKET_MASK_NONE" => {}
            "BATTLE_POCKET_MASK_POKE_BALLS" => battle_pocket |= BattlePocket::POKE_BALLS,
            "BATTLE_POCKET_MASK_BATTLE_ITEMS" => battle_pocket |= BattlePocket::BATTLE_ITEMS,
            "BATTLE_POCKET_MASK_RECOVER_HP" => battle_pocket |= BattlePocket::HP_RESTORE,
            "BATTLE_POCKET_MASK_RECOVER_STATUS" => {
                battle_pocket |= BattlePocket::STATUS_HEALERS;
            }
            "BATTLE_POCKET_MASK_RECOVER_PP" => battle_pocket |= BattlePocket::PP_RESTORE,
            "BATTLE_POCKET_MASK_RECOVER_HP_STATUS" => {
                battle_pocket |= BattlePocket::HP_RESTORE | BattlePocket::STATUS_HEALERS;
            }
            _ => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("Invalid battle pocket flag '{}'", flag),
                ));
            }
        }
    }

    Ok(battle_pocket)
}

fn parse_bool(s: &str, row_number: usize, column: &str) -> io::Result<bool> {
    let normalized = s.trim().to_ascii_lowercase();
    match normalized.as_str() {
        "" | "false" | "0" | "no" => Ok(false),
        "true" | "1" | "yes" => Ok(true),
        _ => Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "Invalid boolean value '{}' for column '{}' at row {}",
                s, column, row_number
            ),
        )),
    }
}

fn parse_optional_number<T>(s: &str, row_number: usize, column: &str) -> io::Result<T>
where
    T: std::str::FromStr + Default,
    T::Err: std::fmt::Display,
{
    let trimmed = s.trim();
    if trimmed.is_empty() {
        return Ok(T::default());
    }

    trimmed.parse::<T>().map_err(|e| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "Invalid value '{}' for column '{}' at row {}: {}",
                trimmed, column, row_number, e
            ),
        )
    })
}

// This mapping is intentionally explicit so each CSV column/alias is readable and
// tied to field-level parse errors with row/column context.
#[allow(clippy::too_many_lines)]
fn parse_csv_row(
    header: &[&str],
    values: &[&str],
    row_number: usize,
) -> io::Result<Option<DecompItemData>> {
    if values.is_empty() {
        return Ok(None);
    }

    let mut fields: HashMap<&str, &str> = HashMap::new();
    for (i, &h) in header.iter().enumerate() {
        if let Some(&v) = values.get(i) {
            fields.insert(h.trim(), v.trim());
        }
    }

    let get_string = |keys: &[&str]| {
        keys.iter()
            .find_map(|&key| fields.get(key).copied())
            .unwrap_or("")
            .to_string()
    };
    let get_u8 = |keys: &[&str]| {
        for key in keys {
            if let Some(&value) = fields.get(key) {
                return parse_optional_number::<u8>(value, row_number, key);
            }
        }
        parse_optional_number::<u8>("", row_number, keys[0])
    };
    let get_i8 = |keys: &[&str]| {
        for key in keys {
            if let Some(&value) = fields.get(key) {
                return parse_optional_number::<i8>(value, row_number, key);
            }
        }
        parse_optional_number::<i8>("", row_number, keys[0])
    };
    let get_u16 = |keys: &[&str]| {
        for key in keys {
            if let Some(&value) = fields.get(key) {
                return parse_optional_number::<u16>(value, row_number, key);
            }
        }
        parse_optional_number::<u16>("", row_number, keys[0])
    };
    let get_bool = |keys: &[&str]| {
        for key in keys {
            if let Some(&value) = fields.get(key) {
                return parse_bool(value, row_number, key);
            }
        }
        parse_bool("", row_number, keys[0])
    };

    let name = get_string(&["item"]);
    if name.is_empty() {
        return Ok(None);
    }

    Ok(Some(DecompItemData {
        name,
        price: get_u16(&["price"])?,
        hold_effect: get_string(&["holdEffect"]),
        hold_effect_param: get_u8(&["holdEffectParam"])?,
        pluck_effect: get_u8(&["pluckEffect"])?,
        fling_effect: get_u8(&["flingEffect"])?,
        fling_power: get_u8(&["flingPower"])?,
        natural_gift_power: get_u8(&["naturalGiftPower"])?,
        natural_gift_type: get_u8(&["naturalGiftType"])?,
        prevent_toss: get_bool(&["prevent_toss"])?,
        selectable: get_bool(&["selectable"])?,
        field_pocket: get_string(&["fieldPocket"]),
        battle_pocket: get_string(&["battlePocket"]),
        field_use_func: get_string(&["fieldUseFunc"]),
        battle_use_func: get_u8(&["battleUseFunc"])?,
        party_use: get_u8(&["partyUse"])?,
        heal_sleep: get_bool(&["healSleep", "slp_heal"])?,
        heal_poison: get_bool(&["healPoison", "psn_heal"])?,
        heal_burn: get_bool(&["healBurn", "brn_heal"])?,
        heal_freeze: get_bool(&["healFreeze", "frz_heal"])?,
        heal_paralysis: get_bool(&["healParalysis", "prz_heal"])?,
        heal_confusion: get_bool(&["healConfusion", "cfs_heal"])?,
        heal_attract: get_bool(&["healAttract", "inf_heal"])?,
        guard_spec: get_bool(&["guardSpec", "guard_spec"])?,
        revive: get_bool(&["revive"])?,
        revive_all: get_bool(&["reviveAll", "revive_all"])?,
        level_up: get_bool(&["levelUp", "level_up"])?,
        evolve: get_bool(&["evolve"])?,
        atk_stages: get_u8(&["atkStages", "atk_stages"])?,
        def_stages: get_u8(&["defStages", "def_stages"])?,
        spatk_stages: get_u8(&["spatkStages", "spatk_stages"])?,
        spdef_stages: get_u8(&["spdefStages", "spdef_stages"])?,
        speed_stages: get_u8(&["speedStages", "speed_stages"])?,
        acc_stages: get_u8(&["accStages", "accuracy_stages"])?,
        crit_stages: get_u8(&["critStages", "critrate_stages"])?,
        pp_up: get_bool(&["ppUp", "pp_up"])?,
        pp_max: get_bool(&["ppMax", "pp_max"])?,
        pp_restore: get_bool(&["ppRestore", "pp_restore"])?,
        pp_restore_all: get_bool(&["ppRestoreAll", "pp_restore_all"])?,
        hp_restore: get_bool(&["hpRestore", "hp_restore"])?,
        give_hp_evs: get_bool(&["giveHPEVs", "hp_ev_up"])?,
        give_atk_evs: get_bool(&["giveAtkEVs", "atk_ev_up"])?,
        give_def_evs: get_bool(&["giveDefEVs", "def_ev_up"])?,
        give_speed_evs: get_bool(&["giveSpeedEVs", "speed_ev_up"])?,
        give_spatk_evs: get_bool(&["giveSpAtkEVs", "spatk_ev_up"])?,
        give_spdef_evs: get_bool(&["giveSpDefEVs", "spdef_ev_up"])?,
        give_friendship_low: get_bool(&["giveFriendshipLow", "friendship_mod_lo"])?,
        give_friendship_med: get_bool(&["giveFriendshipMed", "friendship_mod_med"])?,
        give_friendship_high: get_bool(&["giveFriendshipHigh", "friendship_mod_hi"])?,
        hp_evs: get_i8(&["hpEVs", "hp_ev_up_param"])?,
        atk_evs: get_i8(&["atkEVs", "atk_ev_up_param"])?,
        def_evs: get_i8(&["defEVs", "def_ev_up_param"])?,
        speed_evs: get_i8(&["speedEVs", "speed_ev_up_param"])?,
        spatk_evs: get_i8(&["spatkEVs", "spatk_ev_up_param"])?,
        spdef_evs: get_i8(&["spdefEVs", "spdef_ev_up_param"])?,
        hp_restored: get_u8(&["hpRestored", "hp_restore_param"])?,
        pp_restored: get_u8(&["ppRestored", "pp_restore_param"])?,
        friendship_low: get_i8(&["friendshipLow", "friendship_mod_lo_param"])?,
        friendship_med: get_i8(&["friendshipMed", "friendship_mod_med_param"])?,
        friendship_high: get_i8(&["friendshipHigh", "friendship_mod_hi_param"])?,
    }))
}

pub fn load_item_data_from_csv(
    path: impl AsRef<Path>,
) -> io::Result<HashMap<String, DecompItemData>> {
    let path = path.as_ref();
    let content = fs::read_to_string(path)?;
    let mut lines = content.lines();

    let header_line = lines
        .next()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "Empty CSV file"))?;
    let header: Vec<&str> = header_line.split(',').map(str::trim).collect();

    let mut result = HashMap::new();

    for (row_index, line) in lines.enumerate() {
        if line.trim().is_empty() {
            continue;
        }

        let row_number = row_index + 2;
        let values: Vec<&str> = line.split(',').map(str::trim).collect();
        let item = parse_csv_row(&header, &values, row_number).map_err(|e| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Failed to parse {}: {}", path.display(), e),
            )
        })?;
        if let Some(item) = item {
            result.insert(item.name.clone(), item);
        }
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    const CSV_HEADER: &str = "item,price,holdEffect,holdEffectParam,pluckEffect,flingEffect,flingPower,naturalGiftPower,naturalGiftType,prevent_toss,selectable,fieldPocket,battlePocket,fieldUseFunc,battleUseFunc,partyUse,healSleep,healPoison,healBurn,healFreeze,healParalysis,healConfusion,healAttract,guardSpec,revive,reviveAll,levelUp,evolve,atkStages,defStages,spatkStages,spdefStages,speedStages,accStages,critStages,ppUp,ppMax,ppRestore,ppRestoreAll,hpRestore,giveHPEVs,giveAtkEVs,giveDefEVs,giveSpeedEVs,giveSpAtkEVs,giveSpDefEVs,giveFriendshipLow,giveFriendshipMed,giveFriendshipHigh,hpEVs,atkEVs,defEVs,speedEVs,spatkEVs,spdefEVs,hpRestored,ppRestored,friendshipLow,friendshipMed,friendshipHigh";
    const CSV_VALID_ROW: &str = "ITEM_POTION,300,HOLD_EFFECT_NONE,0,0,0,0,0,0,false,true,POCKET_MEDICINE,BATTLE_POCKET_MASK_RECOVER_HP,ITEMUSE_NONE,0,0,false,false,false,false,false,false,false,false,false,false,false,false,0,0,0,0,0,0,0,false,false,false,false,true,false,false,false,false,false,false,true,true,true,0,0,0,0,0,0,20,0,3,2,1";

    #[test]
    fn test_load_item_data_from_csv_loads_valid_entries() {
        let dir = tempdir().unwrap();
        let csv_path = dir.path().join("pl_item_data.csv");
        let csv = format!(
            "{header}\n{row}\n",
            header = CSV_HEADER,
            row = CSV_VALID_ROW
        );
        fs::write(&csv_path, csv).unwrap();

        let loaded = load_item_data_from_csv(&csv_path).unwrap();
        assert!(loaded.contains_key("ITEM_POTION"));

        let item = loaded.get("ITEM_POTION").unwrap();
        assert_eq!(item.price, 300);
        assert_eq!(item.field_pocket, "POCKET_MEDICINE");
    }

    #[test]
    fn test_load_item_data_from_csv_empty_file_returns_error() {
        let dir = tempdir().unwrap();
        let csv_path = dir.path().join("pl_item_data.csv");
        fs::write(&csv_path, "").unwrap();

        let err = load_item_data_from_csv(&csv_path).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
        assert!(err.to_string().contains("Empty CSV file"));
    }

    #[test]
    fn test_to_item_data_resolves_required_constants() {
        let dir = tempdir().unwrap();
        let csv_path = dir.path().join("pl_item_data.csv");
        let csv = format!(
            "{header}\n{row}\n",
            header = CSV_HEADER,
            row = CSV_VALID_ROW
        );
        fs::write(&csv_path, csv).unwrap();

        let loaded = load_item_data_from_csv(&csv_path).unwrap();
        let item = loaded.get("ITEM_POTION").unwrap();
        let data = item
            .to_item_data(|name| match name {
                "HOLD_EFFECT_NONE" => Some(0),
                "ITEMUSE_NONE" => Some(0),
                _ => None,
            })
            .unwrap();

        assert_eq!(data.price, 300);
        assert_eq!(data.hold_effect, 0);
        assert_eq!(data.field_use_func, 0);
    }

    #[test]
    fn test_to_item_data_unresolved_required_constant_returns_error() {
        let dir = tempdir().unwrap();
        let csv_path = dir.path().join("pl_item_data.csv");
        let csv = format!(
            "{header}\n{row}\n",
            header = CSV_HEADER,
            row = CSV_VALID_ROW
        );
        fs::write(&csv_path, csv).unwrap();

        let loaded = load_item_data_from_csv(&csv_path).unwrap();
        let item = loaded.get("ITEM_POTION").unwrap();
        let err = item
            .to_item_data(|name| match name {
                "ITEMUSE_NONE" => Some(0),
                _ => None,
            })
            .unwrap_err();

        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
        assert!(err.to_string().contains("HOLD_EFFECT_NONE"));
        assert!(err.to_string().contains("holdEffect"));
    }

    #[test]
    fn test_to_item_data_invalid_field_pocket_returns_error() {
        let dir = tempdir().unwrap();
        let csv_path = dir.path().join("pl_item_data.csv");
        let mut cols: Vec<&str> = CSV_VALID_ROW.split(',').collect();
        cols[11] = "POCKET_UNKNOWN";
        let csv = format!("{header}\n{}\n", cols.join(","), header = CSV_HEADER);
        fs::write(&csv_path, csv).unwrap();

        let loaded = load_item_data_from_csv(&csv_path).unwrap();
        let item = loaded.get("ITEM_POTION").unwrap();
        let err = item
            .to_item_data(|name| match name {
                "HOLD_EFFECT_NONE" => Some(0),
                "ITEMUSE_NONE" => Some(0),
                _ => None,
            })
            .unwrap_err();

        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
        assert!(err.to_string().contains("Invalid field pocket"));
    }

    #[test]
    fn test_to_item_data_invalid_battle_pocket_flag_returns_error() {
        let dir = tempdir().unwrap();
        let csv_path = dir.path().join("pl_item_data.csv");
        let mut cols: Vec<&str> = CSV_VALID_ROW.split(',').collect();
        cols[12] = "BATTLE_POCKET_MASK_UNKNOWN";
        let csv = format!("{header}\n{}\n", cols.join(","), header = CSV_HEADER);
        fs::write(&csv_path, csv).unwrap();

        let loaded = load_item_data_from_csv(&csv_path).unwrap();
        let item = loaded.get("ITEM_POTION").unwrap();
        let err = item
            .to_item_data(|name| match name {
                "HOLD_EFFECT_NONE" => Some(0),
                "ITEMUSE_NONE" => Some(0),
                _ => None,
            })
            .unwrap_err();

        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
        assert!(err.to_string().contains("Invalid battle pocket flag"));
    }

    #[test]
    fn test_to_item_data_numeric_field_and_battle_pocket_are_supported() {
        let dir = tempdir().unwrap();
        let csv_path = dir.path().join("pl_item_data.csv");
        let mut cols: Vec<&str> = CSV_VALID_ROW.split(',').collect();
        cols[11] = "2";
        cols[12] = "5";
        let csv = format!("{header}\n{}\n", cols.join(","), header = CSV_HEADER);
        fs::write(&csv_path, csv).unwrap();

        let loaded = load_item_data_from_csv(&csv_path).unwrap();
        let item = loaded.get("ITEM_POTION").unwrap();
        let data = item
            .to_item_data(|name| match name {
                "HOLD_EFFECT_NONE" => Some(0),
                "ITEMUSE_NONE" => Some(0),
                _ => None,
            })
            .unwrap();

        assert_eq!(data.field_pocket, FieldPocket::Balls);
        assert!(data.battle_pocket.contains(BattlePocket::POKE_BALLS));
        assert!(data.battle_pocket.contains(BattlePocket::HP_RESTORE));
    }

    #[test]
    fn test_load_item_data_from_csv_invalid_numeric_returns_error() {
        let dir = tempdir().unwrap();
        let csv_path = dir.path().join("pl_item_data.csv");

        let mut cols: Vec<&str> = CSV_VALID_ROW.split(',').collect();
        cols[1] = "not_a_number";
        let csv = format!("{header}\n{}\n", cols.join(","), header = CSV_HEADER);
        fs::write(&csv_path, csv).unwrap();

        let err = load_item_data_from_csv(&csv_path).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
        assert!(err.to_string().contains("Failed to parse"));
        assert!(err.to_string().contains("price"));
        assert!(err.to_string().contains("row 2"));
    }

    #[test]
    fn test_load_item_data_from_csv_invalid_boolean_returns_error() {
        let dir = tempdir().unwrap();
        let csv_path = dir.path().join("pl_item_data.csv");

        let mut cols: Vec<&str> = CSV_VALID_ROW.split(',').collect();
        cols[16] = "maybe";
        let csv = format!("{header}\n{}\n", cols.join(","), header = CSV_HEADER);
        fs::write(&csv_path, csv).unwrap();

        let err = load_item_data_from_csv(&csv_path).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
        assert!(err.to_string().contains("Failed to parse"));
        assert!(err.to_string().contains("healSleep"));
        assert!(err.to_string().contains("row 2"));
    }

    #[test]
    fn test_load_item_data_from_csv_hgss_alias_columns() {
        let dir = tempdir().unwrap();
        let csv_path = dir.path().join("item_data.csv");
        let csv = "item,price,holdEffect,holdEffectParam,pluckEffect,flingEffect,flingPower,naturalGiftPower,naturalGiftType,prevent_toss,selectable,fieldPocket,battlePocket,fieldUseFunc,battleUseFunc,partyUse,slp_heal,psn_heal,brn_heal,frz_heal,prz_heal,cfs_heal,inf_heal,guard_spec,revive,revive_all,level_up,evolve,atk_stages,def_stages,spatk_stages,spdef_stages,speed_stages,accuracy_stages,critrate_stages,pp_up,pp_max,pp_restore,pp_restore_all,hp_restore,hp_ev_up,atk_ev_up,def_ev_up,speed_ev_up,spatk_ev_up,spdef_ev_up,friendship_mod_lo,friendship_mod_med,friendship_mod_hi,hp_ev_up_param,atk_ev_up_param,def_ev_up_param,speed_ev_up_param,spatk_ev_up_param,spdef_ev_up_param,hp_restore_param,pp_restore_param,friendship_mod_lo_param,friendship_mod_med_param,friendship_mod_hi_param\nITEM_TEST,100,HOLD_EFFECT_NONE,1,2,3,4,5,6,false,true,POCKET_MEDICINE,5,ITEMUSE_NONE,7,8,true,false,true,false,true,false,true,true,true,false,true,false,1,2,3,4,5,6,2,true,false,true,false,true,true,false,true,false,true,false,true,false,true,10,11,12,13,14,15,16,17,18,19,20\n";
        fs::write(&csv_path, csv).unwrap();

        let loaded = load_item_data_from_csv(&csv_path).unwrap();
        let item = loaded.get("ITEM_TEST").unwrap();

        assert!(item.heal_sleep);
        assert!(item.heal_burn);
        assert!(item.heal_paralysis);
        assert!(item.heal_attract);
        assert!(item.guard_spec);
        assert!(item.revive);
        assert!(item.level_up);
        assert_eq!(item.acc_stages, 6);
        assert_eq!(item.crit_stages, 2);
        assert!(item.pp_up);
        assert!(item.pp_restore);
        assert!(item.hp_restore);
        assert!(item.give_hp_evs);
        assert!(item.give_def_evs);
        assert!(item.give_spatk_evs);
        assert!(item.give_friendship_low);
        assert!(item.give_friendship_high);
        assert_eq!(item.hp_evs, 10);
        assert_eq!(item.pp_restored, 17);
        assert_eq!(item.friendship_high, 20);
    }

    #[test]
    #[ignore = "requires local Platinum decomp fixture via UXIE_TEST_PLATINUM_DECOMP_PATH"]
    fn integration_load_item_data_from_csv_platinum_real_fixture() {
        let Some(root) = crate::test_env::existing_path_from_env(
            "UXIE_TEST_PLATINUM_DECOMP_PATH",
            "decomp_data items integration test",
        ) else {
            return;
        };

        let csv_path = root.join("res/items/pl_item_data.csv");
        if !csv_path.exists() {
            eprintln!(
                "Skipping decomp_data items integration test: CSV file does not exist: {}",
                csv_path.display()
            );
            return;
        }

        let loaded = load_item_data_from_csv(&csv_path).unwrap();
        assert!(
            !loaded.is_empty(),
            "expected at least one item from {}",
            csv_path.display()
        );
        assert!(
            loaded.keys().any(|name| name.starts_with("ITEM_")),
            "expected at least one ITEM_* constant-style key from {}",
            csv_path.display()
        );
    }

    #[test]
    #[ignore = "requires local HGSS decomp fixture via UXIE_TEST_HGSS_DECOMP_PATH"]
    fn integration_load_item_data_from_csv_hgss_real_fixture() {
        let Some(root) = crate::test_env::existing_path_from_env(
            "UXIE_TEST_HGSS_DECOMP_PATH",
            "decomp_data items integration test (hgss)",
        ) else {
            return;
        };

        let csv_path = root.join("files/itemtool/itemdata/item_data.csv");
        if !csv_path.exists() {
            eprintln!(
                "Skipping decomp_data items integration test (hgss): CSV file does not exist: {}",
                csv_path.display()
            );
            return;
        }

        let loaded = load_item_data_from_csv(&csv_path).unwrap();
        assert!(
            !loaded.is_empty(),
            "expected at least one item from {}",
            csv_path.display()
        );

        let poke_ball = loaded
            .get("ITEM_POKE_BALL")
            .expect("expected ITEM_POKE_BALL");
        assert_eq!(poke_ball.price, 200);
    }
}
