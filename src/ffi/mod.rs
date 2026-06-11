//! C-compatible FFI exports for DSPRE and other native consumers.
//!
//! Phase 1 provides foundation primitives only; feature exports are added in later phases.

use crate::game::RomIdentity;
use crate::rom_header::RomHeader;
use std::cell::RefCell;
use std::ffi::{c_char, c_int, CStr, CString};
use std::path::PathBuf;

pub const UXIE_OK: c_int = 0;
pub const UXIE_ERR_INVALID_INPUT: c_int = -1;
pub const UXIE_ERR_IO: c_int = -2;
pub const UXIE_ERR_PARSE: c_int = -3;
pub const UXIE_ERR_NOT_FOUND: c_int = -4;
pub const UXIE_ERR_UNKNOWN: c_int = -99;

thread_local! {
    static LAST_ERROR: RefCell<Option<CString>> = const { RefCell::new(None) };
}

/// Record an error message for retrieval via `uxie_last_error`.
pub(crate) fn set_last_error(msg: impl Into<String>) {
    let text = msg.into();
    let c = CString::new(text)
        .unwrap_or_else(|_| CString::new("error message contained interior NUL").unwrap());
    LAST_ERROR.with(|slot| *slot.borrow_mut() = Some(c));
}

/// Clears the thread-local last-error slot before a fallible call succeeds.
pub(crate) fn clear_last_error() {
    LAST_ERROR.with(|slot| *slot.borrow_mut() = None);
}

/// Returns the last error string for this thread, or null if none.
///
/// Pointer is owned by uxie; do NOT free it, and copy before the next FFI call.
#[unsafe(no_mangle)]
pub extern "C" fn uxie_last_error() -> *const c_char {
    LAST_ERROR.with(|slot| {
        slot.borrow()
            .as_ref()
            .map_or(std::ptr::null(), |c| c.as_ptr())
    })
}

/// Convert a Rust String into a heap C string for the caller to own.
///
/// Returns null (and sets last_error) if the string contains an interior NUL.
pub(crate) fn string_to_c(s: String) -> *mut c_char {
    CString::new(s).map_or_else(
        |_| {
            set_last_error("returned string contained an interior NUL byte");
            std::ptr::null_mut()
        },
        |c| c.into_raw(),
    )
}

/// Free a string previously returned by a uxie FFI function.
///
/// # Safety
/// `ptr` must be a pointer returned by a uxie string-returning function and not freed before.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn uxie_free_string(ptr: *mut c_char) {
    if !ptr.is_null() {
        // SAFETY: caller guarantees `ptr` came from `string_to_c` and was not freed yet.
        unsafe {
            let _ = CString::from_raw(ptr);
        }
    }
}

/// # Safety
/// `ptr` must be a valid null-terminated C string, or null.
pub(crate) unsafe fn c_str_to_path(ptr: *const c_char) -> Option<PathBuf> {
    if ptr.is_null() {
        return None;
    }
    // SAFETY: caller guarantees `ptr` is a valid null-terminated C string.
    unsafe { CStr::from_ptr(ptr).to_str().ok().map(PathBuf::from) }
}

/// Returns the uxie crate version as a heap C string. Free with `uxie_free_string`.
#[unsafe(no_mangle)]
pub extern "C" fn uxie_version() -> *mut c_char {
    clear_last_error();
    string_to_c(env!("CARGO_PKG_VERSION").to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CString;

    fn write_header_yaml(dir: &std::path::Path, gamecode: &str) -> CString {
        let yaml = dir.join("header.yaml");
        std::fs::write(
            &yaml,
            format!("title: TESTROM\ngamecode: {gamecode}\nmakercode: '01'\nrom_version: 0\n"),
        )
        .unwrap();
        CString::new(yaml.to_str().unwrap()).unwrap()
    }

    #[test]
    fn identify_rom_returns_identity_json() {
        let dir = tempfile::tempdir().unwrap();
        let path = write_header_yaml(dir.path(), "CPUE");

        let ptr = unsafe { uxie_identify_rom(path.as_ptr()) };
        assert!(!ptr.is_null());

        let json = unsafe { CStr::from_ptr(ptr) }.to_str().unwrap().to_string();
        unsafe { uxie_free_string(ptr) };

        let v: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(v["game_code"], "CPUE");
        assert_eq!(v["game"], "Platinum");
        assert_eq!(v["family"], "Platinum");
        assert_eq!(v["language"], "English");
    }

    #[test]
    fn identify_rom_unknown_code_returns_null_with_error() {
        let dir = tempfile::tempdir().unwrap();
        let path = write_header_yaml(dir.path(), "ZZZZ");

        let ptr = unsafe { uxie_identify_rom(path.as_ptr()) };
        assert!(ptr.is_null());

        let err = uxie_last_error();
        assert!(!err.is_null());
        let msg = unsafe { CStr::from_ptr(err) }.to_str().unwrap();
        assert!(msg.contains("ZZZZ"), "error should mention the bad code: {msg}");
    }

    #[test]
    fn identify_rom_null_path_returns_null_with_error() {
        let ptr = unsafe { uxie_identify_rom(std::ptr::null()) };
        assert!(ptr.is_null());
        assert!(!uxie_last_error().is_null());
    }
}

/// Identify a ROM from a header path, returning `RomIdentity` as a JSON C string.
///
/// `path` may point at a binary `header.bin`, a ds-rom `header.yaml`, or any file
/// `RomHeader::open` accepts. Returns null and sets `uxie_last_error` on a null/invalid
/// path, an IO/parse failure, or an unrecognized Gen 4 game code. Free with `uxie_free_string`.
///
/// # Safety
/// `path` must be a valid null-terminated C string, or null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn uxie_identify_rom(path: *const c_char) -> *mut c_char {
    clear_last_error();

    let Some(path) = (unsafe { c_str_to_path(path) }) else {
        set_last_error("uxie_identify_rom: path was null or not valid UTF-8");
        return std::ptr::null_mut();
    };

    let header = match RomHeader::open(&path) {
        Ok(header) => header,
        Err(e) => {
            set_last_error(format!(
                "uxie_identify_rom: failed to read ROM header from {}: {e}",
                path.display()
            ));
            return std::ptr::null_mut();
        }
    };

    let Some(identity) = RomIdentity::from_header(&header) else {
        set_last_error(format!(
            "uxie_identify_rom: '{}' is not a recognized Gen 4 Pokémon game code",
            header.game_code
        ));
        return std::ptr::null_mut();
    };

    match serde_json::to_string(&identity) {
        Ok(json) => string_to_c(json),
        Err(e) => {
            set_last_error(format!("uxie_identify_rom: failed to serialize identity: {e}"));
            std::ptr::null_mut()
        }
    }
}
