//! # Rustad
//!
//! High-performance, zero-dependency browser-agnostic adblocking engine in pure Rust and WebAssembly.

pub mod cosmetic;
pub mod domain;
pub mod engine;
pub mod pattern;

#[path = "generated/rules.rs"]
pub mod rules;

// Re-export primary engine functions for idiomatic Rust callers
pub use engine::{check_url, get_cosmetic_css, write_cosmetic_css, Verdict};

// ==============================================================================
// C-ABI / WebAssembly Exports
// Zero-allocation memory protocol for JS / WebAssembly runtimes
// ==============================================================================

/// Static 16KB scratch buffer for passing strings between JS and WASM.
static mut SCRATCH: [u8; 16384] = [0u8; 16384];

/// Returns a pointer to the static scratch buffer for foreign function callers.
///
/// # Safety
///
/// The caller must ensure single-threaded execution when accessing the returned memory buffer.
#[no_mangle]
#[must_use]
pub unsafe extern "C" fn rustad_scratch_ptr() -> *mut u8 {
    core::ptr::addr_of_mut!(SCRATCH).cast::<u8>()
}

/// Returns the byte capacity of the static scratch buffer.
///
/// # Safety
///
/// This function is safe to call from any context.
#[no_mangle]
#[must_use]
pub unsafe extern "C" fn rustad_scratch_len() -> usize {
    16384
}

/// Evaluates a URL provided via raw pointer and length.
///
/// Returns:
/// - `0`: `Verdict::Pass`
/// - `1`: `Verdict::Block`
/// - `2`: `Verdict::Allow`
///
/// # Safety
///
/// The caller must ensure `url_ptr` points to at least `url_len` readable UTF-8 bytes.
#[no_mangle]
pub unsafe extern "C" fn rustad_check_url(url_ptr: *const u8, url_len: usize) -> u32 {
    if url_ptr.is_null() || url_len == 0 {
        return Verdict::Pass as u32;
    }

    let slice = unsafe { core::slice::from_raw_parts(url_ptr, url_len) };
    if let Ok(url_str) = core::str::from_utf8(slice) {
        check_url(url_str) as u32
    } else {
        Verdict::Pass as u32
    }
}

/// Writes cosmetic CSS for the given URL into the static scratch buffer.
///
/// Returns the number of bytes written.
///
/// # Safety
///
/// The caller must ensure `url_ptr` points to at least `url_len` readable UTF-8 bytes.
#[no_mangle]
pub unsafe extern "C" fn rustad_get_cosmetics(url_ptr: *const u8, url_len: usize) -> usize {
    if url_ptr.is_null() || url_len == 0 {
        return 0;
    }

    let slice = unsafe { core::slice::from_raw_parts(url_ptr, url_len) };
    if let Ok(url_str) = core::str::from_utf8(slice) {
        let scratch_slice = unsafe {
            core::slice::from_raw_parts_mut(core::ptr::addr_of_mut!(SCRATCH).cast::<u8>(), 16384)
        };
        write_cosmetic_css(url_str, scratch_slice)
    } else {
        0
    }
}
