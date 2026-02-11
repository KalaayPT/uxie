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

        Ok(Self::from_parts(
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
        ))
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
    use std::io::Cursor;

    #[test]
    fn test_roundtrip() {
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
            party_use_param: ItemPartyUseParam {
                hp_restore: true,
                hp_restored: 20,
                ..Default::default()
            },
        };

        let bytes = item.to_bytes();
        assert_eq!(bytes.len(), ITEM_DATA_SIZE);

        let mut cursor = Cursor::new(bytes);
        let parsed = ItemData::from_binary(&mut cursor).unwrap();

        assert_eq!(item.price, parsed.price);
        assert_eq!(item.field_pocket, parsed.field_pocket);
        assert_eq!(item.is_selectable, parsed.is_selectable);
        assert_eq!(
            item.party_use_param.hp_restored,
            parsed.party_use_param.hp_restored
        );
    }

    #[test]
    fn test_party_use_param_roundtrip() {
        let param = ItemPartyUseParam {
            heal_sleep: true,
            heal_poison: true,
            hp_restore: true,
            hp_restored: 50,
            give_hp_evs: true,
            hp_evs: 10,
            friendship_low: 5,
            friendship_med: 3,
            friendship_high: 1,
            give_friendship_low: true,
            give_friendship_med: true,
            give_friendship_high: true,
            ..Default::default()
        };

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
    #[ignore]
    fn test_real_rom_roundtrip() {
        use std::fs::File;
        use std::io::BufReader;

        let Some(dspre_path) = crate::test_env::existing_path_from_env(
            "UXIE_TEST_PLATINUM_DSPRE_PATH",
            "item data real ROM roundtrip test",
        ) else {
            return;
        };
        let narc_path = dspre_path.join("data/itemtool/itemdata/pl_item_data.narc");
        if !narc_path.exists() {
            eprintln!(
                "Skipping: test data not available at {}",
                narc_path.display()
            );
            return;
        }

        let file = File::open(&narc_path).expect("Failed to open NARC");
        let mut reader = BufReader::new(file);
        let narc = crate::Narc::from_binary(&mut reader).expect("Failed to load NARC");

        for (i, original_bytes) in narc.members.iter().enumerate().take(100) {
            if original_bytes.len() != ITEM_DATA_SIZE {
                continue;
            }

            let mut cursor = Cursor::new(original_bytes.as_slice());
            let item =
                ItemData::from_binary(&mut cursor).expect(&format!("Failed to parse item {}", i));

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
