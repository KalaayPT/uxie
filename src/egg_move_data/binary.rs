use super::types::{EGG_MOVE_SPECIES_OFFSET, EGG_MOVE_TERMINATOR, EggMoveData, EggMoveEntry};
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
    use std::io::Cursor;

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
}
