use super::types::{EggMoveData, EggMoveEntry, EGG_MOVE_SPECIES_OFFSET, EGG_MOVE_TERMINATOR};
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use std::io::{self, Read, Write};

impl EggMoveData {
    pub fn from_binary<R: Read>(reader: &mut R) -> io::Result<Self> {
        let mut entries = Vec::new();
        let mut current_entry: Option<EggMoveEntry> = None;

        loop {
            let value = reader.read_u16::<LittleEndian>()?;

            if value == EGG_MOVE_TERMINATOR {
                if let Some(entry) = current_entry.take() {
                    entries.push(entry);
                }
                break;
            }

            if value > EGG_MOVE_SPECIES_OFFSET {
                if let Some(entry) = current_entry.take() {
                    entries.push(entry);
                }
                let species_id = value - EGG_MOVE_SPECIES_OFFSET;
                current_entry = Some(EggMoveEntry::new(species_id, Vec::new()));
            } else if let Some(ref mut entry) = current_entry {
                entry.move_ids.push(value);
            }
        }

        Ok(Self { entries })
    }

    pub fn to_binary<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        for entry in &self.entries {
            writer.write_u16::<LittleEndian>(entry.species_id + EGG_MOVE_SPECIES_OFFSET)?;
            for &move_id in &entry.move_ids {
                writer.write_u16::<LittleEndian>(move_id)?;
            }
        }
        writer.write_u16::<LittleEndian>(EGG_MOVE_TERMINATOR)?;
        Ok(())
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(self.total_byte_size());
        self.to_binary(&mut buf).unwrap();
        buf
    }
}

impl EggMoveEntry {
    pub fn from_binary_simple<R: Read>(reader: &mut R) -> io::Result<Self> {
        let mut move_ids = Vec::new();
        loop {
            let value = reader.read_u16::<LittleEndian>()?;
            if value == EGG_MOVE_TERMINATOR {
                break;
            }
            move_ids.push(value);
        }
        Ok(Self {
            species_id: 0,
            move_ids,
        })
    }

    pub fn to_binary_simple<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        for &move_id in &self.move_ids {
            writer.write_u16::<LittleEndian>(move_id)?;
        }
        writer.write_u16::<LittleEndian>(EGG_MOVE_TERMINATOR)?;
        Ok(())
    }

    pub fn to_bytes_simple(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity((self.move_ids.len() + 1) * 2);
        self.to_binary_simple(&mut buf).unwrap();
        buf
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
    fn test_roundtrip_normal_format() {
        let data = EggMoveData {
            entries: vec![
                EggMoveEntry::new(1, vec![33, 45, 64]),
                EggMoveEntry::new(4, vec![52, 53]),
                EggMoveEntry::new(7, vec![55, 56, 57, 58]),
            ],
        };

        let bytes = data.to_bytes();
        let mut cursor = Cursor::new(bytes);
        let parsed = EggMoveData::from_binary(&mut cursor).unwrap();

        assert_eq!(data, parsed);
    }

    #[test]
    fn test_empty_data() {
        let data = EggMoveData::default();
        let bytes = data.to_bytes();
        assert_eq!(bytes.len(), 2);
        assert_eq!(&bytes, &[0xFF, 0xFF]);

        let mut cursor = Cursor::new(bytes);
        let parsed = EggMoveData::from_binary(&mut cursor).unwrap();
        assert!(parsed.entries.is_empty());
    }

    #[test]
    fn test_species_with_no_moves() {
        let data = EggMoveData {
            entries: vec![EggMoveEntry::new(25, vec![])],
        };

        let bytes = data.to_bytes();
        let mut cursor = Cursor::new(bytes);
        let parsed = EggMoveData::from_binary(&mut cursor).unwrap();

        assert_eq!(data, parsed);
        assert!(parsed.entries[0].move_ids.is_empty());
    }

    #[test]
    fn test_simple_format_roundtrip() {
        let entry = EggMoveEntry::new(25, vec![85, 86, 87, 344]);

        let bytes = entry.to_bytes_simple();
        let mut cursor = Cursor::new(bytes);
        let parsed = EggMoveEntry::from_binary_simple(&mut cursor).unwrap();

        assert_eq!(entry.move_ids, parsed.move_ids);
    }

    #[test]
    fn test_simple_format_empty() {
        let entry = EggMoveEntry::new(1, vec![]);
        let bytes = entry.to_bytes_simple();
        assert_eq!(bytes.len(), 2);
        assert_eq!(&bytes, &[0xFF, 0xFF]);

        let mut cursor = Cursor::new(bytes);
        let parsed = EggMoveEntry::from_binary_simple(&mut cursor).unwrap();
        assert!(parsed.move_ids.is_empty());
    }

    #[test]
    fn test_get_by_species() {
        let data = EggMoveData {
            entries: vec![
                EggMoveEntry::new(1, vec![33]),
                EggMoveEntry::new(4, vec![52]),
                EggMoveEntry::new(7, vec![55]),
            ],
        };

        assert!(data.get_by_species(4).is_some());
        assert_eq!(data.get_by_species(4).unwrap().move_ids, vec![52]);
        assert!(data.get_by_species(999).is_none());
    }

    #[test]
    fn test_can_learn() {
        let data = EggMoveData {
            entries: vec![EggMoveEntry::new(25, vec![85, 86, 87])],
        };

        assert!(data.can_learn(25, 85));
        assert!(data.can_learn(25, 87));
        assert!(!data.can_learn(25, 100));
        assert!(!data.can_learn(1, 85));
    }

    #[test]
    fn test_byte_size() {
        let entry = EggMoveEntry::new(1, vec![33, 45, 64]);
        assert_eq!(entry.byte_size(), 8);

        let data = EggMoveData {
            entries: vec![
                EggMoveEntry::new(1, vec![33, 45]),
                EggMoveEntry::new(4, vec![52]),
            ],
        };
        assert_eq!(data.total_byte_size(), 12);
    }

    #[test]
    fn test_binary_format_values() {
        let data = EggMoveData {
            entries: vec![EggMoveEntry::new(1, vec![33, 45])],
        };

        let bytes = data.to_bytes();

        let mut cursor = Cursor::new(&bytes);
        let species_marker = cursor.read_u16::<LittleEndian>().unwrap();
        assert_eq!(species_marker, 1 + EGG_MOVE_SPECIES_OFFSET);

        let move1 = cursor.read_u16::<LittleEndian>().unwrap();
        assert_eq!(move1, 33);

        let move2 = cursor.read_u16::<LittleEndian>().unwrap();
        assert_eq!(move2, 45);

        let terminator = cursor.read_u16::<LittleEndian>().unwrap();
        assert_eq!(terminator, EGG_MOVE_TERMINATOR);
    }

    fn normal_move_id_strategy() -> impl Strategy<Value = u16> {
        0u16..=EGG_MOVE_SPECIES_OFFSET
    }

    fn species_id_strategy() -> impl Strategy<Value = u16> {
        // Avoid species marker colliding with the 0xFFFF stream terminator.
        1u16..(u16::MAX - EGG_MOVE_SPECIES_OFFSET)
    }

    fn egg_move_entry_strategy() -> impl Strategy<Value = EggMoveEntry> {
        (
            species_id_strategy(),
            prop::collection::vec(normal_move_id_strategy(), 0..24),
        )
            .prop_map(|(species_id, move_ids)| EggMoveEntry {
                species_id,
                move_ids,
            })
    }

    fn egg_move_data_strategy() -> impl Strategy<Value = EggMoveData> {
        prop::collection::vec(egg_move_entry_strategy(), 0..48)
            .prop_map(|entries| EggMoveData { entries })
    }

    fn simple_move_id_strategy() -> impl Strategy<Value = u16> {
        any::<u16>().prop_filter("0xFFFF is the simple-format terminator", |v| {
            *v != EGG_MOVE_TERMINATOR
        })
    }

    proptest! {
        #![proptest_config(ProptestConfig {
            cases: 64,
            .. ProptestConfig::default()
        })]

        #[test]
        fn prop_egg_move_data_roundtrip(data in egg_move_data_strategy()) {
            let bytes = data.to_bytes();
            prop_assert_eq!(bytes.len(), data.total_byte_size());
            prop_assert_eq!(&bytes[bytes.len() - 2..], &[0xFF, 0xFF]);

            let mut cursor = Cursor::new(bytes);
            let parsed = EggMoveData::from_binary(&mut cursor).unwrap();
            prop_assert_eq!(data, parsed);
        }

        #[test]
        fn prop_simple_format_roundtrip(move_ids in prop::collection::vec(simple_move_id_strategy(), 0..64)) {
            let entry = EggMoveEntry::new(123, move_ids.clone());
            let bytes = entry.to_bytes_simple();
            prop_assert_eq!(bytes.len(), (move_ids.len() + 1) * 2);
            prop_assert_eq!(&bytes[bytes.len() - 2..], &[0xFF, 0xFF]);

            let mut cursor = Cursor::new(bytes);
            let parsed = EggMoveEntry::from_binary_simple(&mut cursor).unwrap();
            prop_assert_eq!(parsed.move_ids, move_ids);
            prop_assert_eq!(parsed.species_id, 0);
        }

        #[test]
        fn prop_get_by_species_matches_first_entry(data in egg_move_data_strategy(), species_id in any::<u16>()) {
            let expected = data.entries.iter().find(|e| e.species_id == species_id);
            let actual = data.get_by_species(species_id);
            prop_assert_eq!(actual, expected);
        }

        #[test]
        fn prop_can_learn_matches_manual_lookup(data in egg_move_data_strategy(), species_id in any::<u16>(), move_id in any::<u16>()) {
            let expected = data
                .entries
                .iter()
                .find(|e| e.species_id == species_id)
                .map(|e| e.move_ids.contains(&move_id))
                .unwrap_or(false);
            let actual = data.can_learn(species_id, move_id);
            prop_assert_eq!(actual, expected);
        }
    }

    fn assert_real_hgss_kowaza_roundtrip(narc_path: &Path) {
        let file = File::open(narc_path).expect("Failed to open HGSS kowaza NARC");
        let mut reader = BufReader::new(file);
        let narc = crate::Narc::from_binary(&mut reader).expect("Failed to load HGSS kowaza NARC");
        let member = narc
            .first_member()
            .expect("Expected first member in HGSS kowaza NARC");
        let mut cursor = Cursor::new(member);
        let data =
            EggMoveData::from_binary(&mut cursor).expect("Failed to parse HGSS egg move data");

        assert!(
            !data.entries.is_empty(),
            "expected at least one HGSS egg move entry in {}",
            narc_path.display()
        );
        let serialized = data.to_bytes();
        assert!(
            member.starts_with(serialized.as_slice()),
            "serialized HGSS egg move table is not a prefix of source member in {}",
            narc_path.display()
        );

        let reparsed = EggMoveData::from_binary(&mut Cursor::new(serialized.as_slice()))
            .expect("Failed to reparse serialized HGSS egg move data");
        assert_eq!(data, reparsed);
    }

    fn assert_real_platinum_overlay_roundtrip(overlay_path: &Path) {
        const PLATINUM_EGG_MOVE_OFFSET: usize = 0x29222;
        let overlay_data =
            std::fs::read(overlay_path).expect("Failed to read Platinum overlay_0005.bin");
        assert!(
            overlay_data.len() > PLATINUM_EGG_MOVE_OFFSET,
            "overlay too short for Platinum egg move offset: {}",
            overlay_path.display()
        );

        let mut cursor = Cursor::new(&overlay_data[PLATINUM_EGG_MOVE_OFFSET..]);
        let data =
            EggMoveData::from_binary(&mut cursor).expect("Failed to parse Platinum egg move data");

        assert!(
            !data.entries.is_empty(),
            "expected at least one Platinum egg move entry in {}",
            overlay_path.display()
        );
        let serialized = data.to_bytes();
        assert!(
            overlay_data[PLATINUM_EGG_MOVE_OFFSET..].starts_with(serialized.as_slice()),
            "serialized Platinum egg move table is not a prefix of overlay bytes in {}",
            overlay_path.display()
        );

        let reparsed = EggMoveData::from_binary(&mut Cursor::new(serialized.as_slice()))
            .expect("Failed to reparse serialized Platinum egg move data");
        assert_eq!(data, reparsed);
    }

    #[test]
    #[ignore = "requires a real Platinum DSPRE project via UXIE_TEST_PLATINUM_DSPRE_PATH"]
    fn integration_real_rom_roundtrip_platinum() {
        let Some(overlay_path) = crate::test_env::existing_file_under_project_env(
            "UXIE_TEST_PLATINUM_DSPRE_PATH",
            &["overlay/overlay_0005.bin"],
            "egg move data real ROM roundtrip test (platinum)",
        ) else {
            return;
        };

        assert_real_platinum_overlay_roundtrip(&overlay_path);
    }

    #[test]
    #[ignore = "requires a real HGSS DSPRE project via UXIE_TEST_HGSS_DSPRE_PATH"]
    fn integration_real_rom_roundtrip_hgss() {
        let Some(kowaza_path) = crate::test_env::existing_file_under_project_env(
            "UXIE_TEST_HGSS_DSPRE_PATH",
            &["data/data/kowaza.narc"],
            "egg move data real ROM roundtrip test (hgss)",
        ) else {
            return;
        };

        assert_real_hgss_kowaza_roundtrip(&kowaza_path);
    }
}
