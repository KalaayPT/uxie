//! Item data parser for decompilation CSV format
//!
//! Parses `res/items/pl_item_data.csv` and converts to `ItemData`.

use crate::item_data::{BattlePocket, FieldPocket, ItemData, ItemPartyUseParam};
use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::Path;

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
    pub fn to_item_data<F>(&self, resolve_constant: F) -> ItemData
    where
        F: Fn(&str) -> Option<i64>,
    {
        let hold_effect = resolve_constant(&self.hold_effect).unwrap_or(0) as u8;

        let field_pocket = match self.field_pocket.as_str() {
            "POCKET_ITEMS" => FieldPocket::Items,
            "POCKET_MEDICINE" => FieldPocket::Medicine,
            "POCKET_BALLS" => FieldPocket::Balls,
            "POCKET_TM_HMS" => FieldPocket::TmHms,
            "POCKET_BERRIES" => FieldPocket::Berries,
            "POCKET_MAIL" => FieldPocket::Mail,
            "POCKET_BATTLE_ITEMS" => FieldPocket::BattleItems,
            "POCKET_KEY_ITEMS" => FieldPocket::KeyItems,
            _ => FieldPocket::Items,
        };

        let mut battle_pocket = BattlePocket::empty();
        for flag in self.battle_pocket.split('|').map(str::trim) {
            match flag {
                "BATTLE_POCKET_MASK_POKE_BALLS" => battle_pocket |= BattlePocket::POKE_BALLS,
                "BATTLE_POCKET_MASK_BATTLE_ITEMS" => battle_pocket |= BattlePocket::BATTLE_ITEMS,
                "BATTLE_POCKET_MASK_RECOVER_HP" => battle_pocket |= BattlePocket::HP_RESTORE,
                "BATTLE_POCKET_MASK_RECOVER_STATUS" => {
                    battle_pocket |= BattlePocket::STATUS_HEALERS
                }
                "BATTLE_POCKET_MASK_RECOVER_PP" => battle_pocket |= BattlePocket::PP_RESTORE,
                "BATTLE_POCKET_MASK_RECOVER_HP_STATUS" => {
                    battle_pocket |= BattlePocket::HP_RESTORE | BattlePocket::STATUS_HEALERS
                }
                _ => {}
            }
        }

        let field_use_func = resolve_constant(&self.field_use_func).unwrap_or(0) as u8;

        let party_use_param = ItemPartyUseParam {
            heal_sleep: self.heal_sleep,
            heal_poison: self.heal_poison,
            heal_burn: self.heal_burn,
            heal_freeze: self.heal_freeze,
            heal_paralysis: self.heal_paralysis,
            heal_confusion: self.heal_confusion,
            heal_attract: self.heal_attract,
            guard_spec: self.guard_spec,
            revive: self.revive,
            revive_all: self.revive_all,
            level_up: self.level_up,
            evolve: self.evolve,
            atk_stages: self.atk_stages,
            def_stages: self.def_stages,
            spatk_stages: self.spatk_stages,
            spdef_stages: self.spdef_stages,
            speed_stages: self.speed_stages,
            acc_stages: self.acc_stages,
            crit_stages: self.crit_stages,
            pp_up: self.pp_up,
            pp_max: self.pp_max,
            pp_restore: self.pp_restore,
            pp_restore_all: self.pp_restore_all,
            hp_restore: self.hp_restore,
            give_hp_evs: self.give_hp_evs,
            give_atk_evs: self.give_atk_evs,
            give_def_evs: self.give_def_evs,
            give_speed_evs: self.give_speed_evs,
            give_spatk_evs: self.give_spatk_evs,
            give_spdef_evs: self.give_spdef_evs,
            give_friendship_low: self.give_friendship_low,
            give_friendship_med: self.give_friendship_med,
            give_friendship_high: self.give_friendship_high,
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

        ItemData {
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
        }
    }
}

fn parse_bool(s: &str) -> bool {
    matches!(s.to_lowercase().as_str(), "true" | "1" | "yes")
}

fn parse_csv_row(header: &[&str], values: &[&str]) -> Option<DecompItemData> {
    if values.is_empty() {
        return None;
    }

    let mut fields: HashMap<&str, &str> = HashMap::new();
    for (i, &h) in header.iter().enumerate() {
        if let Some(&v) = values.get(i) {
            fields.insert(h, v);
        }
    }

    let get = |key: &str| fields.get(key).copied().unwrap_or("");
    let get_u8 = |key: &str| get(key).parse::<u8>().unwrap_or(0);
    let get_i8 = |key: &str| get(key).parse::<i8>().unwrap_or(0);
    let get_u16 = |key: &str| get(key).parse::<u16>().unwrap_or(0);
    let get_bool = |key: &str| parse_bool(get(key));

    Some(DecompItemData {
        name: get("item").to_string(),
        price: get_u16("price"),
        hold_effect: get("holdEffect").to_string(),
        hold_effect_param: get_u8("holdEffectParam"),
        pluck_effect: get_u8("pluckEffect"),
        fling_effect: get_u8("flingEffect"),
        fling_power: get_u8("flingPower"),
        natural_gift_power: get_u8("naturalGiftPower"),
        natural_gift_type: get_u8("naturalGiftType"),
        prevent_toss: get_bool("prevent_toss"),
        selectable: get_bool("selectable"),
        field_pocket: get("fieldPocket").to_string(),
        battle_pocket: get("battlePocket").to_string(),
        field_use_func: get("fieldUseFunc").to_string(),
        battle_use_func: get_u8("battleUseFunc"),
        party_use: get_u8("partyUse"),
        heal_sleep: get_bool("healSleep"),
        heal_poison: get_bool("healPoison"),
        heal_burn: get_bool("healBurn"),
        heal_freeze: get_bool("healFreeze"),
        heal_paralysis: get_bool("healParalysis"),
        heal_confusion: get_bool("healConfusion"),
        heal_attract: get_bool("healAttract"),
        guard_spec: get_bool("guardSpec"),
        revive: get_bool("revive"),
        revive_all: get_bool("reviveAll"),
        level_up: get_bool("levelUp"),
        evolve: get_bool("evolve"),
        atk_stages: get_u8("atkStages"),
        def_stages: get_u8("defStages"),
        spatk_stages: get_u8("spatkStages"),
        spdef_stages: get_u8("spdefStages"),
        speed_stages: get_u8("speedStages"),
        acc_stages: get_u8("accStages"),
        crit_stages: get_u8("critStages"),
        pp_up: get_bool("ppUp"),
        pp_max: get_bool("ppMax"),
        pp_restore: get_bool("ppRestore"),
        pp_restore_all: get_bool("ppRestoreAll"),
        hp_restore: get_bool("hpRestore"),
        give_hp_evs: get_bool("giveHPEVs"),
        give_atk_evs: get_bool("giveAtkEVs"),
        give_def_evs: get_bool("giveDefEVs"),
        give_speed_evs: get_bool("giveSpeedEVs"),
        give_spatk_evs: get_bool("giveSpAtkEVs"),
        give_spdef_evs: get_bool("giveSpDefEVs"),
        give_friendship_low: get_bool("giveFriendshipLow"),
        give_friendship_med: get_bool("giveFriendshipMed"),
        give_friendship_high: get_bool("giveFriendshipHigh"),
        hp_evs: get_i8("hpEVs"),
        atk_evs: get_i8("atkEVs"),
        def_evs: get_i8("defEVs"),
        speed_evs: get_i8("speedEVs"),
        spatk_evs: get_i8("spatkEVs"),
        spdef_evs: get_i8("spdefEVs"),
        hp_restored: get_u8("hpRestored"),
        pp_restored: get_u8("ppRestored"),
        friendship_low: get_i8("friendshipLow"),
        friendship_med: get_i8("friendshipMed"),
        friendship_high: get_i8("friendshipHigh"),
    })
}

pub fn load_item_data_from_csv(
    path: impl AsRef<Path>,
) -> io::Result<HashMap<String, DecompItemData>> {
    let content = fs::read_to_string(path)?;
    let mut lines = content.lines();

    let header_line = lines
        .next()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "Empty CSV file"))?;
    let header: Vec<&str> = header_line.split(',').collect();

    let mut result = HashMap::new();

    for line in lines {
        if line.trim().is_empty() {
            continue;
        }

        let values: Vec<&str> = line.split(',').collect();
        if let Some(item) = parse_csv_row(&header, &values) {
            if !item.name.is_empty() {
                result.insert(item.name.clone(), item);
            }
        }
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    const CSV_HEADER: &str = "item,price,holdEffect,holdEffectParam,pluckEffect,flingEffect,flingPower,naturalGiftPower,naturalGiftType,prevent_toss,selectable,fieldPocket,battlePocket,fieldUseFunc,battleUseFunc,partyUse,healSleep,healPoison,healBurn,healFreeze,healParalysis,healConfusion,healAttract,guardSpec,revive,reviveAll,levelUp,evolve,atkStages,defStages,spatkStages,spdefStages,speedStages,accStages,critStages,ppUp,ppMax,ppRestore,ppRestoreAll,hpRestore,giveHPEVs,giveAtkEVs,giveDefEVs,giveSpeedEVs,giveSpAtkEVs,giveSpDefEVs,giveFriendshipLow,giveFriendshipMed,giveFriendshipHigh,hpEVs,atkEVs,defEVs,speedEVs,spatkEVs,spdefEVs,hpRestored,ppRestored,friendshipLow,friendshipMed,friendshipHigh";

    #[test]
    fn test_load_item_data_from_csv_loads_valid_entries() {
        let dir = tempdir().unwrap();
        let csv_path = dir.path().join("pl_item_data.csv");
        let csv = format!(
            "{header}\nITEM_POTION,300,HOLD_EFFECT_NONE,0,0,0,0,0,0,false,true,POCKET_MEDICINE,BATTLE_POCKET_MASK_RECOVER_HP,ITEMUSE_NONE,0,0,false,false,false,false,false,false,false,false,false,false,false,false,0,0,0,0,0,0,0,false,false,false,false,true,false,false,false,false,false,false,true,true,true,0,0,0,0,0,0,20,0,3,2,1\n",
            header = CSV_HEADER
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
}
