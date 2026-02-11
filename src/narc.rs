//! Nintendo Archive (NARC) file format reader and writer
//!
//! NARC is the archive format used extensively in Pokemon Gen 4 games
//! for packing multiple files into a single archive.

use crate::error::{Result, UxieError};
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use std::io::{Read, Seek, SeekFrom, Write};

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

    pub fn to_bytes(&self) -> Vec<u8> {
        let entry_count = self.members.len() as u32;
        let btaf_size = 12 + entry_count * 8;
        let btnf_size = 16u32;

        let mut file_offsets = Vec::with_capacity(self.members.len());
        let mut current_offset = 0u32;
        for data in &self.members {
            file_offsets.push((current_offset, current_offset + data.len() as u32));
            current_offset += data.len() as u32;
        }
        let gmif_size = 8 + current_offset;
        let total_size = 16 + btaf_size + btnf_size + gmif_size;

        let mut buf = Vec::with_capacity(total_size as usize);

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
        for data in &self.members {
            buf.extend_from_slice(data);
        }

        buf
    }

    pub fn write_to<W: Write>(&self, writer: &mut W) -> Result<()> {
        writer.write_all(&self.to_bytes())?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
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

    #[test]
    fn test_roundtrip_single_file() {
        let file_content = b"Test data for roundtrip";
        let narc_data = create_minimal_narc(&[file_content]);
        let mut cursor = Cursor::new(narc_data);

        let narc = Narc::from_binary(&mut cursor).unwrap();
        let written = narc.to_bytes();
        let mut cursor2 = Cursor::new(written);
        let narc2 = Narc::from_binary(&mut cursor2).unwrap();

        assert_eq!(narc.members, narc2.members);
    }

    #[test]
    fn test_roundtrip_multiple_files() {
        let files: Vec<&[u8]> = vec![b"First", b"Second file", b"Third"];
        let narc_data = create_minimal_narc(&files);
        let mut cursor = Cursor::new(narc_data);

        let narc = Narc::from_binary(&mut cursor).unwrap();
        let written = narc.to_bytes();
        let mut cursor2 = Cursor::new(written);
        let narc2 = Narc::from_binary(&mut cursor2).unwrap();

        assert_eq!(narc.members.len(), 3);
        assert_eq!(narc.members, narc2.members);
    }

    #[test]
    fn test_roundtrip_empty_narc() {
        let narc_data = create_minimal_narc(&[]);
        let mut cursor = Cursor::new(narc_data);

        let narc = Narc::from_binary(&mut cursor).unwrap();
        let written = narc.to_bytes();
        let mut cursor2 = Cursor::new(written);
        let narc2 = Narc::from_binary(&mut cursor2).unwrap();

        assert_eq!(narc.members.len(), 0);
        assert_eq!(narc.members, narc2.members);
    }

    fn narc_strategy() -> impl Strategy<Value = Narc> {
        prop::collection::vec(prop::collection::vec(any::<u8>(), 0..256), 0..64)
            .prop_map(|members| Narc { members })
    }

    proptest! {
        #![proptest_config(ProptestConfig {
            cases: 64,
            .. ProptestConfig::default()
        })]

        #[test]
        fn prop_narc_roundtrip(narc in narc_strategy()) {
            let bytes = narc.to_bytes();
            let mut cursor = Cursor::new(bytes);
            let parsed = Narc::from_binary(&mut cursor).unwrap();
            prop_assert_eq!(parsed.members, narc.members);
        }

        #[test]
        fn prop_write_to_matches_to_bytes(narc in narc_strategy()) {
            let expected = narc.to_bytes();
            let mut written = Vec::new();
            narc.write_to(&mut written).unwrap();
            prop_assert_eq!(written.as_slice(), expected.as_slice());

            let mut cursor = Cursor::new(written);
            let parsed = Narc::from_binary(&mut cursor).unwrap();
            prop_assert_eq!(parsed.members, narc.members);
        }

        #[test]
        fn prop_rejects_non_narc_magic(
            magic in any::<[u8; 4]>().prop_filter("must not be NARC magic", |m| m != b"NARC"),
            tail in prop::collection::vec(any::<u8>(), 0..64)
        ) {
            let mut bytes = magic.to_vec();
            bytes.extend_from_slice(&tail);
            let mut cursor = Cursor::new(bytes);
            let result = Narc::from_binary(&mut cursor);
            prop_assert!(result.is_err());
        }
    }
}
