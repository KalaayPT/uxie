//! Nintendo Archive (NARC) file format reader
//!
//! NARC is the archive format used extensively in Pokemon Gen 4 games
//! for packing multiple files into a single archive.

use byteorder::{LittleEndian, ReadBytesExt};
use std::io::{self, Read, Seek, SeekFrom};

pub struct Narc {
    pub members: Vec<Vec<u8>>,
}

impl Narc {
    pub fn from_binary<R: Read + Seek>(reader: &mut R) -> io::Result<Self> {
        let mut magic = [0u8; 4];
        reader.read_exact(&mut magic)?;
        if &magic != b"NARC" {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Not a NARC file",
            ));
        }

        reader.seek(SeekFrom::Current(12))?;

        let mut btaf_magic = [0u8; 4];
        reader.read_exact(&mut btaf_magic)?;
        if &btaf_magic != b"BTAF" {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Missing BTAF chunk",
            ));
        }
        let btaf_size = reader.read_u32::<LittleEndian>()?;
        let entry_count = reader.read_u32::<LittleEndian>()?;

        let mut entries = Vec::with_capacity(entry_count as usize);
        for _ in 0..entry_count {
            let start = reader.read_u32::<LittleEndian>()?;
            let end = reader.read_u32::<LittleEndian>()?;
            entries.push((start, end));
        }

        reader.seek(SeekFrom::Start(16 + btaf_size as u64))?;
        let mut btnf_magic = [0u8; 4];
        reader.read_exact(&mut btnf_magic)?;
        let btnf_size = reader.read_u32::<LittleEndian>()?;

        reader.seek(SeekFrom::Start(16 + btaf_size as u64 + btnf_size as u64))?;
        let mut gmif_magic = [0u8; 4];
        reader.read_exact(&mut gmif_magic)?;
        let _gmif_size = reader.read_u32::<LittleEndian>()?;
        let gmif_offset = reader.stream_position()?;

        let mut members = Vec::with_capacity(entry_count as usize);
        for (start, end) in entries {
            let size = end - start;
            reader.seek(SeekFrom::Start(gmif_offset + start as u64))?;
            let mut data = vec![0u8; size as usize];
            reader.read_exact(&mut data)?;
            members.push(data);
        }

        Ok(Self { members })
    }
}
