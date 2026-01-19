use super::types::PersonalData;
use binrw::{BinRead, BinWrite};
use std::io::{self, Read, Seek, Write};

pub const PERSONAL_DATA_SIZE: usize = 44;

impl PersonalData {
    pub fn from_binary<R: Read + Seek>(reader: &mut R) -> io::Result<Self> {
        Self::read_le(reader).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
    }

    pub fn to_binary<W: Write + Seek>(&self, writer: &mut W) -> io::Result<()> {
        self.write_le(writer)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut buf = std::io::Cursor::new(Vec::with_capacity(PERSONAL_DATA_SIZE));
        self.to_binary(&mut buf).unwrap();
        buf.into_inner()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    fn create_test_personal_data() -> PersonalData {
        PersonalData {
            hp: 45,
            attack: 49,
            defense: 49,
            speed: 45,
            sp_attack: 65,
            sp_defense: 65,
            type1: 12, // Grass
            type2: 3,  // Poison
            catch_rate: 45,
            base_exp: 64,
            ev_yield: 0b0000_0001_0000_0001, // bits 8-9 = Sp.Atk (1), bits 0-1 = HP (1)
            item1: 0,
            item2: 0,
            gender_ratio: 31,
            egg_cycles: 20,
            base_friendship: 70,
            growth_rate: 3,
            egg_group1: 7, // Monster
            egg_group2: 1, // Plant
            ability1: 65,  // Overgrow
            ability2: 34,  // Chlorophyll
            safari_flee_rate: 0,
            color_flip: 5,
            tm_compatibility: [0xFF; 16],
        }
    }

    #[test]
    fn test_roundtrip() {
        let original = create_test_personal_data();
        let bytes = original.to_bytes();
        assert_eq!(bytes.len(), PERSONAL_DATA_SIZE);

        let mut cursor = Cursor::new(bytes);
        let parsed = PersonalData::from_binary(&mut cursor).unwrap();

        assert_eq!(original, parsed);
    }

    #[test]
    fn test_ev_yield_accessors() {
        let data = create_test_personal_data();
        assert_eq!(data.ev_yield_hp(), 1);
        assert_eq!(data.ev_yield_attack(), 0);
        assert_eq!(data.ev_yield_defense(), 0);
        assert_eq!(data.ev_yield_speed(), 0);
        assert_eq!(data.ev_yield_sp_attack(), 1);
        assert_eq!(data.ev_yield_sp_defense(), 0);
    }

    #[test]
    fn test_tm_compatibility() {
        let data = create_test_personal_data();
        for i in 0..128 {
            assert!(data.can_learn_tm(i));
        }
        assert!(!data.can_learn_tm(128));
    }
}
