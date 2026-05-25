//! C-compatible FFI exports for DSPRE and other native consumers.
//!
//! Phase 1 provides foundation primitives only; feature exports are added in later phases.

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
#[allow(dead_code)] // consumed by feature exports in Phase 2+
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
#[allow(dead_code)] // consumed by feature exports in Phase 2+
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
