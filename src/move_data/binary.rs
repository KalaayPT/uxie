use super::types::MoveData;
#[cfg(test)]
use super::types::{MoveFlags, MoveSplit};
use binrw::{BinRead, BinWrite};
use std::io::{self, Read, Seek, Write};

pub const MOVE_DATA_SIZE: usize = 14;

impl MoveData {
    pub fn from_binary<R: Read + Seek>(reader: &mut R) -> io::Result<Self> {
        Self::read_le(reader).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
    }

    pub fn to_binary<W: Write + Seek>(&self, writer: &mut W) -> io::Result<()> {
        self.write_le(writer)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut buf = std::io::Cursor::new(Vec::with_capacity(MOVE_DATA_SIZE));
        self.to_binary(&mut buf).unwrap();
        buf.into_inner()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    use std::fs::File;
    use std::io::BufReader;
    use std::io::Cursor;
    use std::path::Path;

    #[test]
    fn test_roundtrip() {
        let move_data = MoveData {
            battle_effect: 1,
            split: MoveSplit::Physical,
            power: 40,
            move_type: 0,
            accuracy: 100,
            pp: 35,
            side_effect_chance: 0,
            target: 0,
            priority: 0,
            flags: MoveFlags::MAKES_CONTACT,
            contest_appeal: 4,
            contest_condition: 0,
        };

        let bytes = move_data.to_bytes();
        assert_eq!(bytes.len(), MOVE_DATA_SIZE);

        let mut cursor = Cursor::new(bytes);
        let parsed = MoveData::from_binary(&mut cursor).unwrap();

        assert_eq!(move_data, parsed);
    }

    #[test]
    fn test_negative_priority() {
        let move_data = MoveData {
            battle_effect: 100,
            split: MoveSplit::Status,
            power: 0,
            move_type: 14,
            accuracy: 0,
            pp: 10,
            side_effect_chance: 0,
            target: 0,
            priority: -6,
            flags: MoveFlags::empty(),
            contest_appeal: 0,
            contest_condition: 0,
        };

        let bytes = move_data.to_bytes();
        let mut cursor = Cursor::new(bytes);
        let parsed = MoveData::from_binary(&mut cursor).unwrap();

        assert_eq!(parsed.priority, -6);
    }

    fn move_split_strategy() -> impl Strategy<Value = MoveSplit> {
        prop_oneof![
            Just(MoveSplit::Physical),
            Just(MoveSplit::Special),
            Just(MoveSplit::Status),
        ]
    }

    fn move_data_strategy() -> impl Strategy<Value = MoveData> {
        (
            any::<u16>(),
            move_split_strategy(),
            any::<u8>(),
            any::<u8>(),
            any::<u8>(),
            any::<u8>(),
            any::<u8>(),
            any::<u16>(),
            any::<i8>(),
            any::<u8>(),
            any::<u8>(),
            any::<u8>(),
        )
            .prop_map(
                |(
                    battle_effect,
                    split,
                    power,
                    move_type,
                    accuracy,
                    pp,
                    side_effect_chance,
                    target,
                    priority,
                    flags_bits,
                    contest_appeal,
                    contest_condition,
                )| MoveData {
                    battle_effect,
                    split,
                    power,
                    move_type,
                    accuracy,
                    pp,
                    side_effect_chance,
                    target,
                    priority,
                    flags: MoveFlags::from_bits_truncate(flags_bits),
                    contest_appeal,
                    contest_condition,
                },
            )
    }

    proptest! {
        #![proptest_config(ProptestConfig {
            cases: 64,
            .. ProptestConfig::default()
        })]

        #[test]
        fn prop_move_data_roundtrip(move_data in move_data_strategy()) {
            let bytes = move_data.to_bytes();
            prop_assert_eq!(bytes.len(), MOVE_DATA_SIZE);
            let mut cursor = Cursor::new(bytes);
            let parsed = MoveData::from_binary(&mut cursor).unwrap();
            prop_assert_eq!(move_data, parsed);
        }

        #[test]
        fn prop_move_split_from_u8_maps(v in any::<u8>()) {
            let mapped = MoveSplit::from(v);
            let expected = match v {
                0 => MoveSplit::Physical,
                1 => MoveSplit::Special,
                _ => MoveSplit::Status,
            };
            prop_assert_eq!(mapped, expected);
        }
    }

    fn assert_real_move_narc_roundtrip(narc_path: &Path) {
        let file = File::open(narc_path).expect("Failed to open move-data NARC");
        let mut reader = BufReader::new(file);
        let narc = crate::Narc::from_binary(&mut reader).expect("Failed to load move-data NARC");

        let mut roundtripped_members = 0usize;
        for (i, original_bytes) in narc.members.iter().enumerate().take(700) {
            if original_bytes.len() != MOVE_DATA_SIZE {
                continue;
            }

            roundtripped_members += 1;
            let mut cursor = Cursor::new(original_bytes.as_slice());
            let move_data = MoveData::from_binary(&mut cursor)
                .unwrap_or_else(|_| panic!("Failed to parse move-data member {}", i));
            let serialized = move_data.to_bytes();

            assert_eq!(
                original_bytes.as_slice(),
                serialized.as_slice(),
                "Roundtrip failed for move-data member {}",
                i
            );
        }

        assert!(
            roundtripped_members > 0,
            "expected at least one move-data entry with {} bytes in {}",
            MOVE_DATA_SIZE,
            narc_path.display()
        );
    }

    #[test]
    #[ignore = "requires a real Platinum DSPRE project via UXIE_TEST_PLATINUM_DSPRE_PATH"]
    fn integration_real_rom_roundtrip_platinum() {
        let Some(narc_path) = crate::test_env::existing_file_under_project_env(
            "UXIE_TEST_PLATINUM_DSPRE_PATH",
            &[
                "data/poketool/waza/pl_waza_tbl.narc",
                "data/poketool/waza/waza_tbl.narc",
                "data/pbr/waza_tbl.narc",
            ],
            "move data real ROM roundtrip test (platinum)",
        ) else {
            return;
        };

        assert_real_move_narc_roundtrip(&narc_path);
    }

    #[test]
    #[ignore = "requires a real HGSS DSPRE project via UXIE_TEST_HGSS_DSPRE_PATH"]
    fn integration_real_rom_roundtrip_hgss() {
        let Some(narc_path) = crate::test_env::existing_file_under_project_env(
            "UXIE_TEST_HGSS_DSPRE_PATH",
            &[
                "data/data/kowaza.narc",
                "data/poketool/waza/waza_tbl.narc",
                "data/poketool/waza/pl_waza_tbl.narc",
                "data/pbr/waza_tbl.narc",
            ],
            "move data real ROM roundtrip test (hgss)",
        ) else {
            return;
        };

        assert_real_move_narc_roundtrip(&narc_path);
    }
}
