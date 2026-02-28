use super::types::{
    ItemBitfield, ItemData, ItemPartyUseFlagsBits, ItemPartyUseParam, ItemPartyUseValues,
};
use binrw::{BinRead, BinWrite};
use std::io::{self, Read, Seek, Write};

pub const ITEM_DATA_SIZE: usize = 34;

#[binrw::binrw]
#[brw(little)]
struct ItemPartyUseParamBinary {
    flags: ItemPartyUseFlagsBits,
    values: ItemPartyUseValues,
}

impl ItemPartyUseParam {
    pub fn from_binary<R: Read + Seek>(reader: &mut R) -> io::Result<Self> {
        let bin = ItemPartyUseParamBinary::read_le(reader)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        Ok(Self::from_parts(bin.flags, bin.values))
    }

    pub fn to_binary<W: Write + Seek>(&self, writer: &mut W) -> io::Result<()> {
        let (flags, values) = self.to_parts();
        let bin = ItemPartyUseParamBinary { flags, values };
        bin.write_le(writer)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
    }
}

#[binrw::binrw]
#[brw(little)]
struct ItemDataBinary {
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
    #[brw(pad_after = 1)]
    party_use: u8,
    party_use_flags: ItemPartyUseFlagsBits,
    #[brw(pad_after = 2)]
    party_use_values: ItemPartyUseValues,
}

impl ItemData {
    pub fn from_binary<R: Read + Seek>(reader: &mut R) -> io::Result<Self> {
        let bin = ItemDataBinary::read_le(reader)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

        let party_use_param =
            ItemPartyUseParam::from_parts(bin.party_use_flags, bin.party_use_values);

        Self::from_parts(
            bin.price,
            bin.hold_effect,
            bin.hold_effect_param,
            bin.pluck_effect,
            bin.fling_effect,
            bin.fling_power,
            bin.natural_gift_power,
            bin.bitfield,
            bin.field_use_func,
            bin.battle_use_func,
            bin.party_use,
            party_use_param,
        )
    }

    pub fn to_binary<W: Write + Seek>(&self, writer: &mut W) -> io::Result<()> {
        let (party_use_flags, party_use_values) = self.party_use_param.to_parts();

        let bin = ItemDataBinary {
            price: self.price,
            hold_effect: self.hold_effect,
            hold_effect_param: self.hold_effect_param,
            pluck_effect: self.pluck_effect,
            fling_effect: self.fling_effect,
            fling_power: self.fling_power,
            natural_gift_power: self.natural_gift_power,
            bitfield: self.to_bitfield(),
            field_use_func: self.field_use_func,
            battle_use_func: self.battle_use_func,
            party_use: self.party_use,
            party_use_flags,
            party_use_values,
        };

        bin.write_le(writer)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut buf = std::io::Cursor::new(Vec::with_capacity(ITEM_DATA_SIZE));
        self.to_binary(&mut buf).unwrap();
        buf.into_inner()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::item_data::types::{BattlePocket, FieldPocket};
    use proptest::prelude::*;
    use std::io::{self, Cursor};

    #[test]
    fn test_roundtrip() {
        let party_use_flags = ItemPartyUseFlagsBits::new().with_hp_restore(true);
        let party_use_values = ItemPartyUseValues {
            hp_restored: 20,
            ..Default::default()
        };
        let item = ItemData {
            price: 200,
            hold_effect: 0,
            hold_effect_param: 0,
            pluck_effect: 0,
            fling_effect: 0,
            fling_power: 30,
            natural_gift_power: 0,
            natural_gift_type: 0,
            prevent_toss: false,
            is_selectable: true,
            field_pocket: FieldPocket::Items,
            battle_pocket: BattlePocket::empty(),
            field_use_func: 2,
            battle_use_func: 0,
            party_use: 1,
            party_use_param: ItemPartyUseParam::from_parts(party_use_flags, party_use_values),
        };

        let bytes = item.to_bytes();
        assert_eq!(bytes.len(), ITEM_DATA_SIZE);

        let mut cursor = Cursor::new(bytes);
        let parsed = ItemData::from_binary(&mut cursor).unwrap();

        assert_eq!(item.price, parsed.price);
        assert_eq!(item.field_pocket, parsed.field_pocket);
        assert_eq!(item.is_selectable, parsed.is_selectable);
        let (_, item_values) = item.party_use_param.to_parts();
        let (_, parsed_values) = parsed.party_use_param.to_parts();
        assert_eq!(item_values.hp_restored, parsed_values.hp_restored);
    }

    #[test]
    fn test_party_use_param_roundtrip() {
        let flags = ItemPartyUseFlagsBits::new()
            .with_heal_sleep(true)
            .with_heal_poison(true)
            .with_hp_restore(true)
            .with_give_hp_evs(true)
            .with_give_friendship_low(true)
            .with_give_friendship_med(true)
            .with_give_friendship_high(true);
        let values = ItemPartyUseValues {
            hp_restored: 50,
            hp_evs: 10,
            friendship_low: 5,
            friendship_med: 3,
            friendship_high: 1,
            ..Default::default()
        };
        let param = ItemPartyUseParam::from_parts(flags, values);

        let mut buf = Cursor::new(Vec::new());
        param.to_binary(&mut buf).unwrap();
        assert_eq!(buf.get_ref().len(), 18);

        buf.set_position(0);
        let parsed = ItemPartyUseParam::from_binary(&mut buf).unwrap();

        assert_eq!(param, parsed);
    }

    #[test]
    fn test_bitfield_packing() {
        let item = ItemData {
            price: 0,
            hold_effect: 0,
            hold_effect_param: 0,
            pluck_effect: 0,
            fling_effect: 0,
            fling_power: 0,
            natural_gift_power: 0,
            natural_gift_type: 17,
            prevent_toss: true,
            is_selectable: false,
            field_pocket: FieldPocket::KeyItems,
            battle_pocket: BattlePocket::POKE_BALLS | BattlePocket::HP_RESTORE,
            field_use_func: 0,
            battle_use_func: 0,
            party_use: 0,
            party_use_param: ItemPartyUseParam::default(),
        };

        let bytes = item.to_bytes();
        let mut cursor = Cursor::new(bytes);
        let parsed = ItemData::from_binary(&mut cursor).unwrap();

        assert_eq!(parsed.natural_gift_type, 17);
        assert!(parsed.prevent_toss);
        assert!(!parsed.is_selectable);
        assert_eq!(parsed.field_pocket, FieldPocket::KeyItems);
        assert!(parsed.battle_pocket.contains(BattlePocket::POKE_BALLS));
        assert!(parsed.battle_pocket.contains(BattlePocket::HP_RESTORE));
    }

    #[test]
    fn test_from_binary_invalid_field_pocket_bits_returns_error() {
        let bin = ItemDataBinary {
            price: 0,
            hold_effect: 0,
            hold_effect_param: 0,
            pluck_effect: 0,
            fling_effect: 0,
            fling_power: 0,
            natural_gift_power: 0,
            bitfield: ItemBitfield::new()
                .with_natural_gift_type(0)
                .with_prevent_toss(false)
                .with_is_selectable(false)
                .with_field_pocket(15)
                .with_battle_pocket(0),
            field_use_func: 0,
            battle_use_func: 0,
            party_use: 0,
            party_use_flags: ItemPartyUseFlagsBits::new(),
            party_use_values: ItemPartyUseValues::default(),
        };

        let mut buf = Cursor::new(Vec::new());
        bin.write_le(&mut buf).unwrap();
        buf.set_position(0);

        let err = ItemData::from_binary(&mut buf).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
        assert!(err.to_string().contains("Invalid field pocket bits"));
    }

    fn field_pocket_strategy() -> impl Strategy<Value = FieldPocket> {
        prop_oneof![
            Just(FieldPocket::Items),
            Just(FieldPocket::Medicine),
            Just(FieldPocket::Balls),
            Just(FieldPocket::TmHms),
            Just(FieldPocket::Berries),
            Just(FieldPocket::Mail),
            Just(FieldPocket::BattleItems),
            Just(FieldPocket::KeyItems),
        ]
    }

    fn party_use_param_strategy() -> impl Strategy<Value = ItemPartyUseParam> {
        let flags = any::<[u8; 7]>();
        let values = (
            any::<i8>(),
            any::<i8>(),
            any::<i8>(),
            any::<i8>(),
            any::<i8>(),
            any::<i8>(),
            any::<u8>(),
            any::<u8>(),
            any::<i8>(),
            any::<i8>(),
            any::<i8>(),
        );

        (flags, values).prop_map(
            |(
                flags_bytes,
                (
                    hp_evs,
                    atk_evs,
                    def_evs,
                    speed_evs,
                    spatk_evs,
                    spdef_evs,
                    hp_restored,
                    pp_restored,
                    friendship_low,
                    friendship_med,
                    friendship_high,
                ),
            )| {
                let flags = ItemPartyUseFlagsBits::from_bytes(flags_bytes);
                let values = ItemPartyUseValues {
                    hp_evs,
                    atk_evs,
                    def_evs,
                    speed_evs,
                    spatk_evs,
                    spdef_evs,
                    hp_restored,
                    pp_restored,
                    friendship_low,
                    friendship_med,
                    friendship_high,
                };

                ItemPartyUseParam::from_parts(flags, values)
            },
        )
    }

    fn item_data_strategy() -> impl Strategy<Value = ItemData> {
        let part1 = (
            any::<u16>(),
            any::<u8>(),
            any::<u8>(),
            any::<u8>(),
            any::<u8>(),
            any::<u8>(),
            any::<u8>(),
            0u8..32,
            any::<bool>(),
            any::<bool>(),
        );
        let part2 = (
            field_pocket_strategy(),
            0u8..32,
            any::<u8>(),
            any::<u8>(),
            any::<u8>(),
            party_use_param_strategy(),
        );

        (part1, part2).prop_map(
            |(
                (
                    price,
                    hold_effect,
                    hold_effect_param,
                    pluck_effect,
                    fling_effect,
                    fling_power,
                    natural_gift_power,
                    natural_gift_type,
                    prevent_toss,
                    is_selectable,
                ),
                (
                    field_pocket,
                    battle_pocket_bits,
                    field_use_func,
                    battle_use_func,
                    party_use,
                    party_use_param,
                ),
            )| ItemData {
                price,
                hold_effect,
                hold_effect_param,
                pluck_effect,
                fling_effect,
                fling_power,
                natural_gift_power,
                natural_gift_type,
                prevent_toss,
                is_selectable,
                field_pocket,
                battle_pocket: BattlePocket::from_bits(battle_pocket_bits)
                    .expect("0..31 should always be valid battle-pocket bits"),
                field_use_func,
                battle_use_func,
                party_use,
                party_use_param,
            },
        )
    }

    proptest! {
        #![proptest_config(ProptestConfig {
            cases: 64,
            .. ProptestConfig::default()
        })]

        #[test]
        fn prop_item_party_use_param_roundtrip(param in party_use_param_strategy()) {
            let mut buf = Cursor::new(Vec::new());
            param.to_binary(&mut buf).unwrap();
            prop_assert_eq!(buf.get_ref().len(), 18);
            buf.set_position(0);
            let parsed = ItemPartyUseParam::from_binary(&mut buf).unwrap();
            prop_assert_eq!(param, parsed);
        }

        #[test]
        fn prop_item_data_roundtrip(item in item_data_strategy()) {
            let bytes = item.to_bytes();
            prop_assert_eq!(bytes.len(), ITEM_DATA_SIZE);
            let mut cursor = Cursor::new(bytes);
            let parsed = ItemData::from_binary(&mut cursor).unwrap();
            prop_assert_eq!(item, parsed);
        }
    }

    #[test]
    #[ignore = "requires a real Platinum DSPRE project via UXIE_TEST_PLATINUM_DSPRE_PATH"]
    fn integration_real_rom_roundtrip_platinum() {
        use std::fs::File;
        use std::io::BufReader;

        let Some(narc_path) = crate::test_env::existing_file_under_project_env(
            "UXIE_TEST_PLATINUM_DSPRE_PATH",
            &[
                "data/itemtool/itemdata/pl_item_data.narc",
                "data/itemtool/itemdata/item_data.narc",
                "data/pbr/item_data.narc",
            ],
            "item data real ROM roundtrip test (platinum)",
        ) else {
            return;
        };

        let file = File::open(&narc_path).expect("Failed to open NARC");
        let mut reader = BufReader::new(file);
        let narc = crate::Narc::from_binary(&mut reader).expect("Failed to load NARC");

        for (i, original_bytes) in narc.members.iter().enumerate().take(100) {
            if original_bytes.len() != ITEM_DATA_SIZE {
                continue;
            }

            let mut cursor = Cursor::new(original_bytes.as_slice());
            let item = ItemData::from_binary(&mut cursor)
                .unwrap_or_else(|_| panic!("Failed to parse item {}", i));

            let serialized = item.to_bytes();
            assert_eq!(
                original_bytes.as_slice(),
                serialized.as_slice(),
                "Roundtrip failed for item {}",
                i
            );
        }
    }

    #[test]
    #[ignore = "requires a real HGSS DSPRE project via UXIE_TEST_HGSS_DSPRE_PATH"]
    fn integration_real_rom_roundtrip_hgss() {
        use std::fs::File;
        use std::io::BufReader;

        let Some(narc_path) = crate::test_env::existing_file_under_project_env(
            "UXIE_TEST_HGSS_DSPRE_PATH",
            &[
                "data/itemtool/itemdata/item_data.narc",
                "data/itemtool/itemdata/pl_item_data.narc",
                "data/pbr/item_data.narc",
            ],
            "item data real ROM roundtrip test (hgss)",
        ) else {
            return;
        };

        let file = File::open(&narc_path).expect("Failed to open NARC");
        let mut reader = BufReader::new(file);
        let narc = crate::Narc::from_binary(&mut reader).expect("Failed to load NARC");

        for (i, original_bytes) in narc.members.iter().enumerate().take(100) {
            if original_bytes.len() != ITEM_DATA_SIZE {
                continue;
            }

            let mut cursor = Cursor::new(original_bytes.as_slice());
            let item = ItemData::from_binary(&mut cursor)
                .unwrap_or_else(|_| panic!("Failed to parse item {}", i));

            let serialized = item.to_bytes();
            assert_eq!(
                original_bytes.as_slice(),
                serialized.as_slice(),
                "Roundtrip failed for item {}",
                i
            );
        }
    }
}
