//! Nintendo Archive (NARC) file format reader
//!
//! NARC is the archive format used extensively in Pokemon Gen 4 games
//! for packing multiple files into a single archive.

use crate::error::{Result, UxieError};
use byteorder::{LittleEndian, ReadBytesExt};
use std::io::{Read, Seek, SeekFrom};

#[derive(Debug)]
pub struct Narc {
    pub members: Vec<Vec<u8>>,
}

impl Narc {
    pub fn from_binary<R: Read + Seek>(reader: &mut R) -> Result<Self> {
        let mut magic = [0u8; 4];
        reader.read_exact(&mut magic)?;
        if &magic != b"NARC" {
            return Err(UxieError::invalid_format("Not a NARC file"));
        }

        reader.seek(SeekFrom::Current(12))?;

        let mut btaf_magic = [0u8; 4];
        reader.read_exact(&mut btaf_magic)?;
        if &btaf_magic != b"BTAF" {
            return Err(UxieError::invalid_format("Missing BTAF chunk"));
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    fn create_minimal_narc(file_data: &[&[u8]]) -> Vec<u8> {
        use byteorder::WriteBytesExt;
        let mut buf = Vec::new();

        let entry_count = file_data.len() as u32;
        let btaf_size = 12 + entry_count * 8;
        let btnf_size = 16u32;

        let mut file_offsets = Vec::new();
        let mut current_offset = 0u32;
        for data in file_data {
            file_offsets.push((current_offset, current_offset + data.len() as u32));
            current_offset += data.len() as u32;
        }
        let gmif_size = 8 + current_offset;
        let total_size = 16 + btaf_size + btnf_size + gmif_size;

        buf.extend_from_slice(b"NARC");
        buf.write_u16::<LittleEndian>(0xFFFE).unwrap();
        buf.write_u16::<LittleEndian>(0x0100).unwrap();
        buf.write_u32::<LittleEndian>(total_size).unwrap();
        buf.write_u16::<LittleEndian>(16).unwrap();
        buf.write_u16::<LittleEndian>(3).unwrap();

        buf.extend_from_slice(b"BTAF");
        buf.write_u32::<LittleEndian>(btaf_size).unwrap();
        buf.write_u32::<LittleEndian>(entry_count).unwrap();
        for (start, end) in &file_offsets {
            buf.write_u32::<LittleEndian>(*start).unwrap();
            buf.write_u32::<LittleEndian>(*end).unwrap();
        }

        buf.extend_from_slice(b"BTNF");
        buf.write_u32::<LittleEndian>(btnf_size).unwrap();
        buf.extend_from_slice(&[0u8; 8]);

        buf.extend_from_slice(b"GMIF");
        buf.write_u32::<LittleEndian>(gmif_size).unwrap();
        for data in file_data {
            buf.extend_from_slice(data);
        }

        buf
    }

    #[test]
    fn test_parse_valid_narc() {
        let file1 = b"Hello";
        let file2 = b"World!";
        let narc_data = create_minimal_narc(&[file1, file2]);
        let mut cursor = Cursor::new(narc_data);

        let narc = Narc::from_binary(&mut cursor).unwrap();

        assert_eq!(narc.members.len(), 2);
        assert_eq!(narc.members[0], b"Hello");
        assert_eq!(narc.members[1], b"World!");
    }

    #[test]
    fn test_parse_empty_narc() {
        let narc_data = create_minimal_narc(&[]);
        let mut cursor = Cursor::new(narc_data);

        let narc = Narc::from_binary(&mut cursor).unwrap();

        assert_eq!(narc.members.len(), 0);
    }

    #[test]
    fn test_parse_single_file_narc() {
        let file_content = b"Single file content with more data";
        let narc_data = create_minimal_narc(&[file_content]);
        let mut cursor = Cursor::new(narc_data);

        let narc = Narc::from_binary(&mut cursor).unwrap();

        assert_eq!(narc.members.len(), 1);
        assert_eq!(narc.members[0], file_content.as_slice());
    }

    #[test]
    fn test_invalid_magic() {
        let mut data = vec![0u8; 100];
        data[..4].copy_from_slice(b"NOPE");
        let mut cursor = Cursor::new(data);

        let result = Narc::from_binary(&mut cursor);

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("Not a NARC file"));
    }

    #[test]
    fn test_missing_btaf_chunk() {
        let mut data = vec![0u8; 100];
        data[..4].copy_from_slice(b"NARC");
        data[16..20].copy_from_slice(b"NOPE");
        let mut cursor = Cursor::new(data);

        let result = Narc::from_binary(&mut cursor);

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("Missing BTAF chunk"));
    }

    #[test]
    fn test_truncated_file() {
        let data = b"NAR";
        let mut cursor = Cursor::new(data.to_vec());

        let result = Narc::from_binary(&mut cursor);

        assert!(result.is_err());
    }

    #[test]
    fn test_empty_file() {
        let data: Vec<u8> = vec![];
        let mut cursor = Cursor::new(data);

        let result = Narc::from_binary(&mut cursor);

        assert!(result.is_err());
    }
}
