use super::types::{LEARNSET_TERMINATOR, LearnsetData, LearnsetEntry};
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use std::io::{self, Read, Write};

impl LearnsetData {
    pub fn from_binary<R: Read>(reader: &mut R) -> io::Result<Self> {
        let mut entries = Vec::new();
        loop {
            let packed = reader.read_u16::<LittleEndian>()?;
            if packed == LEARNSET_TERMINATOR {
                break;
            }
            entries.push(LearnsetEntry::from_packed(packed));
        }
        Ok(Self { entries })
    }

    pub fn to_binary<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        for entry in &self.entries {
            writer.write_u16::<LittleEndian>(entry.to_packed())?;
        }
        writer.write_u16::<LittleEndian>(LEARNSET_TERMINATOR)?;
        Ok(())
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut buf = std::io::Cursor::new(Vec::with_capacity((self.entries.len() + 1) * 2));
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
    fn test_entry_packing() {
        let entry = LearnsetEntry::new(33, 15);
        let packed = entry.to_packed();
        let unpacked = LearnsetEntry::from_packed(packed);
        assert_eq!(entry, unpacked);
    }

    #[test]
    fn test_roundtrip() {
        let learnset = LearnsetData {
            entries: vec![
                LearnsetEntry::new(33, 1),
                LearnsetEntry::new(45, 4),
                LearnsetEntry::new(36, 7),
                LearnsetEntry::new(98, 13),
            ],
        };

        let bytes = learnset.to_bytes();
        let mut cursor = Cursor::new(bytes);
        let parsed = LearnsetData::from_binary(&mut cursor).unwrap();

        assert_eq!(learnset, parsed);
    }

    #[test]
    fn test_empty_learnset() {
        let learnset = LearnsetData::default();
        let bytes = learnset.to_bytes();
        assert_eq!(bytes.len(), 2);
        assert_eq!(&bytes, &[0xFF, 0xFF]);

        let mut cursor = Cursor::new(bytes);
        let parsed = LearnsetData::from_binary(&mut cursor).unwrap();
        assert!(parsed.entries.is_empty());
    }

    #[test]
    fn test_moves_at_level() {
        let learnset = LearnsetData {
            entries: vec![
                LearnsetEntry::new(1, 1),
                LearnsetEntry::new(2, 5),
                LearnsetEntry::new(3, 10),
                LearnsetEntry::new(4, 15),
            ],
        };

        assert_eq!(learnset.moves_at_level(10).count(), 3);

        let learned_at_5: Vec<_> = learnset.moves_learned_at(5).collect();
        assert_eq!(learned_at_5.len(), 1);
        assert_eq!(learned_at_5[0].move_id, 2);
    }

    fn valid_entry_strategy() -> impl Strategy<Value = LearnsetEntry> {
        (0u16..=0x1FF, 0u8..=0x7F)
            .prop_filter("avoid reserved terminator encoding", |(move_id, level)| {
                !(*move_id == 0x1FF && *level == 0x7F)
            })
            .prop_map(|(move_id, level)| LearnsetEntry { move_id, level })
    }

    fn learnset_strategy() -> impl Strategy<Value = LearnsetData> {
        prop::collection::vec(valid_entry_strategy(), 0..64)
            .prop_map(|entries| LearnsetData { entries })
    }

    proptest! {
        #![proptest_config(ProptestConfig {
            cases: 64,
            .. ProptestConfig::default()
        })]

        #[test]
        fn prop_entry_pack_roundtrip(entry in valid_entry_strategy()) {
            let packed = entry.to_packed();
            prop_assert_ne!(packed, LEARNSET_TERMINATOR);
            let unpacked = LearnsetEntry::from_packed(packed);
            prop_assert_eq!(entry, unpacked);
        }

        #[test]
        fn prop_learnset_roundtrip(learnset in learnset_strategy()) {
            let bytes = learnset.to_bytes();
            prop_assert_eq!(bytes.len(), (learnset.entries.len() + 1) * 2);
            prop_assert_eq!(&bytes[bytes.len() - 2..], &[0xFF, 0xFF]);

            let mut cursor = Cursor::new(bytes);
            let parsed = LearnsetData::from_binary(&mut cursor).unwrap();
            prop_assert_eq!(learnset, parsed);
        }

        #[test]
        fn prop_level_query_helpers_match_filters(learnset in learnset_strategy(), level in any::<u8>()) {
            let expected_at_level: Vec<_> = learnset.entries.iter().filter(|e| e.level <= level).collect();
            let expected_learned_at: Vec<_> = learnset.entries.iter().filter(|e| e.level == level).collect();

            let actual_at_level: Vec<_> = learnset.moves_at_level(level).collect();
            let actual_learned_at: Vec<_> = learnset.moves_learned_at(level).collect();

            prop_assert_eq!(actual_at_level, expected_at_level);
            prop_assert_eq!(actual_learned_at, expected_learned_at);
        }
    }

    fn assert_real_learnset_narc_roundtrip(narc_path: &Path) {
        let file = File::open(narc_path).expect("Failed to open learnset NARC");
        let mut reader = BufReader::new(file);
        let narc = crate::Narc::from_binary(&mut reader).expect("Failed to load learnset NARC");

        let mut roundtripped_members = 0usize;
        let members = narc.members_owned().unwrap();
        for (i, original_bytes) in members.iter().enumerate().take(700) {
            let mut cursor = Cursor::new(original_bytes.as_slice());
            let learnset = LearnsetData::from_binary(&mut cursor)
                .unwrap_or_else(|_| panic!("Failed to parse learnset member {}", i));
            let serialized = learnset.to_bytes();
            roundtripped_members += 1;

            assert_eq!(
                original_bytes.as_slice(),
                serialized.as_slice(),
                "Roundtrip failed for learnset member {}",
                i
            );
        }

        assert!(
            roundtripped_members > 0,
            "expected at least one learnset entry in {}",
            narc_path.display()
        );
    }

    #[test]
    #[ignore = "requires a real Platinum DSPRE project via UXIE_TEST_PLATINUM_DSPRE_PATH"]
    fn integration_real_rom_roundtrip_platinum() {
        let Some(narc_path) = crate::test_env::existing_file_under_project_env(
            "UXIE_TEST_PLATINUM_DSPRE_PATH",
            &["data/poketool/personal/pms.narc", "data/pbr/pms.narc"],
            "learnset data real ROM roundtrip test (platinum)",
        ) else {
            return;
        };

        assert_real_learnset_narc_roundtrip(&narc_path);
    }

    #[test]
    #[ignore = "requires a real HGSS DSPRE project via UXIE_TEST_HGSS_DSPRE_PATH"]
    fn integration_real_rom_roundtrip_hgss() {
        let Some(narc_path) = crate::test_env::existing_file_under_project_env(
            "UXIE_TEST_HGSS_DSPRE_PATH",
            &["data/poketool/personal/pms.narc", "data/pbr/pms.narc"],
            "learnset data real ROM roundtrip test (hgss)",
        ) else {
            return;
        };

        assert_real_learnset_narc_roundtrip(&narc_path);
    }
}
