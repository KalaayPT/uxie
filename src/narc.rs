//! Nintendo Archive (NARC) file format reader backed by `nitroarc`.
//!
//! This module replaces Uxie's legacy in-house parser with a thin, safe Rust
//! wrapper around the `nitroarc` FFI layer provided by the repository-root
//! `nitroarc` git submodule.

use crate::error::{Result, UxieError};
use std::ffi::{CStr, CString};
use std::io::{Read, Seek, Write};
use std::os::raw::{c_char, c_int, c_uint, c_void};
use std::path::Path;
use std::ptr::NonNull;

#[allow(non_camel_case_types)]
type uint16_t = u16;
#[allow(non_camel_case_types)]
type uint32_t = u32;

#[repr(C)]
struct nitroarcffi_archive_t {
    _private: [u8; 0],
}

#[repr(C)]
struct nitroarcffi_builder_t {
    _private: [u8; 0],
}

unsafe extern "C" {
    fn nitroarcffi_errs(errc: c_int) -> *const c_char;

    fn nitroarcffi_archive_open(
        stream: *const c_void,
        size: uint32_t,
        out_archive: *mut *mut nitroarcffi_archive_t,
    ) -> c_int;

    fn nitroarcffi_archive_close(archive: *mut nitroarcffi_archive_t);

    fn nitroarcffi_archive_count(
        archive: *const nitroarcffi_archive_t,
        out_count: *mut uint16_t,
    ) -> c_int;

    fn nitroarcffi_archive_geti(
        archive: *const nitroarcffi_archive_t,
        index: uint16_t,
        out_member: *mut *const c_void,
        out_size: *mut uint32_t,
    ) -> c_int;

    fn nitroarcffi_archive_gets(
        archive: *const nitroarcffi_archive_t,
        path: *const c_char,
        out_member: *mut *const c_void,
        out_size: *mut uint32_t,
    ) -> c_int;

    fn nitroarcffi_archive_nameof_alloc(
        archive: *const nitroarcffi_archive_t,
        index: uint16_t,
        out_name: *mut *mut c_char,
    ) -> c_int;

    fn nitroarcffi_builder_open(
        nfiles: uint16_t,
        named: c_uint,
        stripped: c_uint,
        out_builder: *mut *mut nitroarcffi_builder_t,
    ) -> c_int;

    fn nitroarcffi_builder_ppack(
        builder: *mut nitroarcffi_builder_t,
        data: *const c_void,
        size: uint32_t,
        name: *const c_char,
    ) -> c_int;

    fn nitroarcffi_builder_pseal(
        builder: *mut nitroarcffi_builder_t,
        out_data: *mut *mut c_void,
        out_size: *mut uint32_t,
    ) -> c_int;

    fn nitroarcffi_builder_close(builder: *mut nitroarcffi_builder_t);

    fn nitroarcffi_free(ptr: *mut c_void);
}

#[link(name = "nitroarc_ffi")]
unsafe extern "C" {}

#[derive(Debug)]
pub struct Narc {
    archive: NonNull<nitroarcffi_archive_t>,
}

impl Narc {
    pub fn from_binary<R: Read + Seek>(reader: &mut R) -> Result<Self> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes)?;
        Self::from_bytes(&bytes)
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        let size = u32::try_from(bytes.len())
            .map_err(|_| UxieError::invalid_format("NARC stream is too large"))?;

        let mut archive = std::ptr::null_mut();
        let errc = unsafe {
            nitroarcffi_archive_open(bytes.as_ptr().cast::<c_void>(), size, &raw mut archive)
        };
        check_nitroarc(errc)?;

        let archive = NonNull::new(archive)
            .ok_or_else(|| UxieError::invalid_format("nitroarc returned a null archive handle"))?;

        Ok(Self { archive })
    }

    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let bytes = std::fs::read(path)?;
        Self::from_bytes(&bytes)
    }

    #[must_use]
    pub fn len(&self) -> usize {
        let mut count = 0u16;
        let errc = unsafe { nitroarcffi_archive_count(self.archive.as_ptr(), &raw mut count) };
        debug_assert_eq!(errc, 0, "nitroarcffi_archive_count failed: {}", errc);
        usize::from(count)
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn member(&self, index: usize) -> Result<&[u8]> {
        let index = u16::try_from(index)
            .map_err(|_| UxieError::invalid_format("NARC member index exceeds nitroarc range"))?;

        let mut data = std::ptr::null();
        let mut size = 0u32;
        let errc = unsafe {
            nitroarcffi_archive_geti(self.archive.as_ptr(), index, &raw mut data, &raw mut size)
        };
        check_nitroarc(errc)?;

        if data.is_null() && size != 0 {
            return Err(UxieError::invalid_format(
                "nitroarc returned a null member pointer for non-empty data",
            ));
        }

        let len = usize::try_from(size)
            .map_err(|_| UxieError::invalid_format("NARC member size exceeds usize"))?;

        let slice = unsafe { std::slice::from_raw_parts(data.cast::<u8>(), len) };
        Ok(slice)
    }

    pub fn member_owned(&self, index: usize) -> Result<Vec<u8>> {
        Ok(self.member(index)?.to_vec())
    }

    pub fn first_member(&self) -> Result<&[u8]> {
        self.member(0)
    }

    pub fn members(&self) -> Members<'_> {
        Members {
            narc: self,
            next_index: 0,
            len: self.len(),
        }
    }

    pub fn members_owned(&self) -> Result<Vec<Vec<u8>>> {
        self.members()
            .map(|member| member.map(ToOwned::to_owned))
            .collect()
    }

    pub fn member_name(&self, index: usize) -> Result<Option<String>> {
        let index = u16::try_from(index)
            .map_err(|_| UxieError::invalid_format("NARC member index exceeds nitroarc range"))?;

        let mut name_ptr = std::ptr::null_mut();
        let errc = unsafe {
            nitroarcffi_archive_nameof_alloc(self.archive.as_ptr(), index, &raw mut name_ptr)
        };
        check_nitroarc(errc)?;

        if name_ptr.is_null() {
            return Ok(None);
        }

        let name = unsafe { CStr::from_ptr(name_ptr) }
            .to_string_lossy()
            .into_owned();
        unsafe { nitroarcffi_free(name_ptr.cast::<c_void>()) };
        if name.is_empty() {
            Ok(None)
        } else {
            Ok(Some(name))
        }
    }

    pub fn member_by_name(&self, path: &str) -> Result<&[u8]> {
        let path = CString::new(path).map_err(|_| {
            UxieError::invalid_format("NARC member path contains an interior NUL byte")
        })?;

        let mut data = std::ptr::null();
        let mut size = 0u32;
        let errc = unsafe {
            nitroarcffi_archive_gets(
                self.archive.as_ptr(),
                path.as_ptr(),
                &raw mut data,
                &raw mut size,
            )
        };
        check_nitroarc(errc)?;

        if data.is_null() && size != 0 {
            return Err(UxieError::invalid_format(
                "nitroarc returned a null member pointer for non-empty named data",
            ));
        }

        let len = usize::try_from(size)
            .map_err(|_| UxieError::invalid_format("NARC member size exceeds usize"))?;
        let slice = unsafe { std::slice::from_raw_parts(data.cast::<u8>(), len) };
        Ok(slice)
    }

    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        let len = self.len();
        let count = u16::try_from(len)
            .map_err(|_| UxieError::invalid_format("NARC member count exceeds nitroarc range"))?;

        let mut builder = std::ptr::null_mut();
        let errc = unsafe { nitroarcffi_builder_open(count, 0, 0, &raw mut builder) };
        check_nitroarc(errc)?;

        let builder = NonNull::new(builder)
            .ok_or_else(|| UxieError::invalid_format("nitroarc returned a null builder handle"))?;

        for index in 0..len {
            let member = self.member(index)?;
            let member_size = u32::try_from(member.len())
                .map_err(|_| UxieError::invalid_format("NARC member is too large"))?;

            let errc = unsafe {
                nitroarcffi_builder_ppack(
                    builder.as_ptr(),
                    member.as_ptr().cast::<c_void>(),
                    member_size,
                    std::ptr::null(),
                )
            };

            if let Err(err) = check_nitroarc(errc) {
                unsafe { nitroarcffi_builder_close(builder.as_ptr()) };
                return Err(err);
            }
        }

        let mut out_data = std::ptr::null_mut();
        let mut out_size = 0u32;
        let errc = unsafe {
            nitroarcffi_builder_pseal(builder.as_ptr(), &raw mut out_data, &raw mut out_size)
        };

        if errc != 0 {
            unsafe { nitroarcffi_builder_close(builder.as_ptr()) };
            check_nitroarc(errc)?;
        }

        let len = usize::try_from(out_size)
            .map_err(|_| UxieError::invalid_format("Serialized NARC size exceeds usize"))?;
        let bytes = unsafe { std::slice::from_raw_parts(out_data.cast::<u8>(), len) }.to_vec();
        unsafe { nitroarcffi_free(out_data) };
        Ok(bytes)
    }

    pub fn write_to<W: Write>(&self, writer: &mut W) -> Result<()> {
        writer.write_all(&self.to_bytes()?)?;
        Ok(())
    }

    pub fn write_to_file(&self, path: impl AsRef<Path>) -> Result<()> {
        std::fs::write(path, self.to_bytes()?)?;
        Ok(())
    }
}

impl Drop for Narc {
    fn drop(&mut self) {
        unsafe {
            nitroarcffi_archive_close(self.archive.as_ptr());
        }
    }
}

pub struct Members<'a> {
    narc: &'a Narc,
    next_index: usize,
    len: usize,
}

impl<'a> Iterator for Members<'a> {
    type Item = Result<&'a [u8]>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.next_index >= self.len {
            return None;
        }

        let index = self.next_index;
        self.next_index += 1;
        Some(self.narc.member(index))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.len.saturating_sub(self.next_index);
        (remaining, Some(remaining))
    }
}

impl ExactSizeIterator for Members<'_> {}

fn check_nitroarc(errc: c_int) -> Result<()> {
    if errc == 0 {
        return Ok(());
    }

    let message = unsafe {
        let ptr = nitroarcffi_errs(errc);
        if ptr.is_null() {
            format!("nitroarc error {}", errc)
        } else {
            CStr::from_ptr(ptr).to_string_lossy().into_owned()
        }
    };

    Err(UxieError::invalid_format(format!(
        "nitroarc error {}: {}",
        errc, message
    )))
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    use std::fs::File;
    use std::io::{BufReader, Cursor};
    use std::path::Path;

    fn create_minimal_narc(file_data: &[&[u8]]) -> Vec<u8> {
        let count = u16::try_from(file_data.len()).unwrap();

        let mut builder = std::ptr::null_mut();
        let errc = unsafe { nitroarcffi_builder_open(count, 0, 0, &raw mut builder) };
        assert_eq!(errc, 0);

        let builder = NonNull::new(builder).unwrap();

        for data in file_data {
            let errc = unsafe {
                nitroarcffi_builder_ppack(
                    builder.as_ptr(),
                    data.as_ptr().cast::<c_void>(),
                    u32::try_from(data.len()).unwrap(),
                    std::ptr::null(),
                )
            };
            assert_eq!(errc, 0);
        }

        let mut out_data = std::ptr::null_mut();
        let mut out_size = 0u32;
        let errc = unsafe {
            nitroarcffi_builder_pseal(builder.as_ptr(), &raw mut out_data, &raw mut out_size)
        };
        assert_eq!(errc, 0);

        let bytes = unsafe {
            std::slice::from_raw_parts(out_data.cast::<u8>(), usize::try_from(out_size).unwrap())
        }
        .to_vec();

        unsafe { nitroarcffi_free(out_data) };
        bytes
    }

    #[test]
    fn test_parse_valid_narc() {
        let file1 = b"Hello";
        let file2 = b"World!";
        let narc_data = create_minimal_narc(&[file1, file2]);
        let mut cursor = Cursor::new(narc_data);

        let narc = Narc::from_binary(&mut cursor).unwrap();

        assert_eq!(narc.len(), 2);
        assert_eq!(narc.member(0).unwrap(), b"Hello");
        assert_eq!(narc.member(1).unwrap(), b"World!");
    }

    #[test]
    fn test_parse_empty_narc() {
        let result = Narc::from_bytes(&[]);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_single_file_narc() {
        let file_content = b"Single file content with more data";
        let narc_data = create_minimal_narc(&[file_content]);
        let mut cursor = Cursor::new(narc_data);

        let narc = Narc::from_binary(&mut cursor).unwrap();

        assert_eq!(narc.len(), 1);
        assert_eq!(narc.member(0).unwrap(), file_content.as_slice());
    }

    #[test]
    fn test_invalid_magic() {
        let mut data = vec![0u8; 100];
        data[..4].copy_from_slice(b"NOPE");
        let mut cursor = Cursor::new(data);

        let result = Narc::from_binary(&mut cursor);

        assert!(result.is_err());
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
        let written = narc.to_bytes().unwrap();
        let mut cursor2 = Cursor::new(written);
        let narc2 = Narc::from_binary(&mut cursor2).unwrap();

        assert_eq!(narc.member(0).unwrap(), narc2.member(0).unwrap());
    }

    #[test]
    fn test_roundtrip_multiple_files() {
        let files: Vec<&[u8]> = vec![b"First", b"Second file", b"Third"];
        let narc_data = create_minimal_narc(&files);
        let mut cursor = Cursor::new(narc_data);

        let narc = Narc::from_binary(&mut cursor).unwrap();
        let written = narc.to_bytes().unwrap();
        let mut cursor2 = Cursor::new(written);
        let narc2 = Narc::from_binary(&mut cursor2).unwrap();

        assert_eq!(narc.len(), 3);
        assert_eq!(
            narc.members_owned().unwrap(),
            narc2.members_owned().unwrap()
        );
    }

    fn assert_real_narc_parse_roundtrip(narc_path: &Path) {
        let file = File::open(narc_path).expect("Failed to open real NARC");
        let mut reader = BufReader::new(file);
        let narc = Narc::from_binary(&mut reader).expect("Failed to parse real NARC");

        assert!(
            !narc.is_empty(),
            "expected at least one member in real NARC {}",
            narc_path.display()
        );

        let mut serialized = Vec::new();
        narc.write_to(&mut serialized)
            .expect("Failed to serialize real NARC");
        assert!(
            serialized.starts_with(b"NARC"),
            "serialized real NARC does not start with NARC magic for {}",
            narc_path.display()
        );

        let mut cursor = Cursor::new(serialized);
        let reparsed =
            Narc::from_binary(&mut cursor).expect("Failed to parse serialized real NARC bytes");
        assert_eq!(
            narc.members_owned().unwrap(),
            reparsed.members_owned().unwrap(),
            "member mismatch after real-NARC parse/serialize roundtrip for {}",
            narc_path.display()
        );
    }

    #[test]
    #[ignore = "requires a real Platinum DSPRE project via UXIE_TEST_PLATINUM_DSPRE_PATH"]
    fn integration_parse_real_narc_platinum() {
        let Some(narc_path) = crate::test_env::existing_file_under_project_env(
            "UXIE_TEST_PLATINUM_DSPRE_PATH",
            &[
                "data/poketool/personal/pl_personal.narc",
                "data/poketool/personal/personal.narc",
                "data/itemtool/itemdata/pl_item_data.narc",
                "data/poketool/waza/pl_waza_tbl.narc",
            ],
            "narc real-parse integration test (platinum)",
        ) else {
            return;
        };

        assert_real_narc_parse_roundtrip(&narc_path);
    }

    #[test]
    #[ignore = "requires a real HGSS DSPRE project via UXIE_TEST_HGSS_DSPRE_PATH"]
    fn integration_parse_real_narc_hgss() {
        let Some(narc_path) = crate::test_env::existing_file_under_project_env(
            "UXIE_TEST_HGSS_DSPRE_PATH",
            &[
                "data/poketool/personal/pms.narc",
                "data/pbr/personal.narc",
                "data/pbr/item_data.narc",
                "data/pbr/waza_tbl.narc",
                "data/data/kowaza.narc",
            ],
            "narc real-parse integration test (hgss)",
        ) else {
            return;
        };

        assert_real_narc_parse_roundtrip(&narc_path);
    }

    prop_compose! {
        fn member_vec_strategy()
            (members in proptest::collection::vec(
                proptest::collection::vec(any::<u8>(), 0..128),
                1..16
            )) -> Vec<Vec<u8>> {
                members
            }
    }

    proptest! {
        #[test]
        fn prop_roundtrip_members(members in member_vec_strategy()) {
            let refs: Vec<&[u8]> = members.iter().map(Vec::as_slice).collect();
            let bytes = create_minimal_narc(&refs);
            let narc = Narc::from_bytes(&bytes).unwrap();
            let reparsed = Narc::from_bytes(&narc.to_bytes().unwrap()).unwrap();
            prop_assert_eq!(members, reparsed.members_owned().unwrap());
        }
    }
}
