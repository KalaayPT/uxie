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
    use std::io::Cursor;

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

        let at_level_10: Vec<_> = learnset.moves_at_level(10).collect();
        assert_eq!(at_level_10.len(), 3);

        let learned_at_5: Vec<_> = learnset.moves_learned_at(5).collect();
        assert_eq!(learned_at_5.len(), 1);
        assert_eq!(learned_at_5[0].move_id, 2);
    }
}
