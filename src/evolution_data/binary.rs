#[cfg(test)]
use super::types::EvolutionEntry;
use super::types::{EVOLUTION_FILE_SIZE, EvolutionData};
use binrw::{BinRead, BinWrite};
use std::io::{self, Read, Seek, Write};

impl EvolutionData {
    pub fn from_binary<R: Read + Seek>(reader: &mut R) -> io::Result<Self> {
        Self::read_le(reader).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
    }

    pub fn to_binary<W: Write + Seek>(&self, writer: &mut W) -> io::Result<()> {
        self.write_le(writer)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut buf = std::io::Cursor::new(Vec::with_capacity(EVOLUTION_FILE_SIZE));
        self.to_binary(&mut buf).unwrap();
        buf.into_inner()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn test_roundtrip() {
        let mut evo = EvolutionData::default();
        evo.entries[0] = EvolutionEntry {
            method: 4,
            param: 16,
            target_species: 2,
        };
        evo.entries[1] = EvolutionEntry {
            method: 1,
            param: 32,
            target_species: 3,
        };

        let bytes = evo.to_bytes();
        assert_eq!(bytes.len(), EVOLUTION_FILE_SIZE);

        let mut cursor = Cursor::new(bytes);
        let parsed = EvolutionData::from_binary(&mut cursor).unwrap();

        assert_eq!(evo, parsed);
    }

    #[test]
    fn test_active_evolutions() {
        let mut evo = EvolutionData::default();
        evo.entries[0] = EvolutionEntry {
            method: 4,
            param: 16,
            target_species: 2,
        };
        evo.entries[2] = EvolutionEntry {
            method: 1,
            param: 32,
            target_species: 3,
        };

        let active: Vec<_> = evo.active_evolutions().collect();
        assert_eq!(active.len(), 2);
    }
}
