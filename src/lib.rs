//! # Rustad
//!
//! High-performance, zero-dependency browser-agnostic adblocking engine in pure Rust and WebAssembly.

pub mod cosmetic;
pub mod domain;
pub mod engine;
pub mod pattern;
pub mod scriptlet;

#[path = "generated/rules.rs"]
pub mod rules;

// Re-export primary engine functions for idiomatic Rust callers
pub use engine::{check_url, get_cosmetic_css, write_cosmetic_css, Verdict};
pub use scriptlet::{for_each_domain_scriptlet, get_scriptlets_json, write_scriptlets_json};

// ==============================================================================
// C-ABI / WebAssembly Exports
// Zero-allocation memory protocol for JS / WebAssembly runtimes
// ==============================================================================

/// Static 2KB input buffer for receiving URLs from host runtimes.
static mut INPUT: [u8; 2048] = [0u8; 2048];

/// Static output buffer sized at compile time to fit the worst-case cosmetic CSS.
static mut OUTPUT: [u8; rules::MAX_COSMETIC_CSS_LEN] = [0u8; rules::MAX_COSMETIC_CSS_LEN];

/// Returns a pointer to the input buffer.
///
/// # Safety
///
/// The caller must ensure single-threaded execution when writing to this buffer.
#[no_mangle]
#[must_use]
pub unsafe extern "C" fn rustad_input_ptr() -> *mut u8 {
    core::ptr::addr_of_mut!(INPUT).cast::<u8>()
}

/// Returns the byte capacity of the input buffer.
///
/// # Safety
///
/// Safe to call from any context.
#[no_mangle]
#[must_use]
pub unsafe extern "C" fn rustad_input_len() -> usize {
    2048
}

/// Returns a pointer to the output buffer.
///
/// # Safety
///
/// The caller must ensure single-threaded execution when reading from this buffer.
#[no_mangle]
#[must_use]
pub unsafe extern "C" fn rustad_output_ptr() -> *mut u8 {
    core::ptr::addr_of_mut!(OUTPUT).cast::<u8>()
}

/// Returns the byte capacity of the output buffer.
///
/// # Safety
///
/// Safe to call from any context.
#[no_mangle]
#[must_use]
pub unsafe extern "C" fn rustad_output_len() -> usize {
    rules::MAX_COSMETIC_CSS_LEN
}

/// Evaluates the URL written in the input buffer.
///
/// Returns:
/// - `0`: `Verdict::Pass`
/// - `1`: `Verdict::Block`
/// - `2`: `Verdict::Allow`
///
/// # Safety
///
/// The caller must ensure `url_len` bytes in the input buffer are valid UTF-8.
#[no_mangle]
pub unsafe extern "C" fn rustad_check_url(url_len: usize) -> u32 {
    let len = url_len.min(2048);
    if len == 0 {
        return Verdict::Pass as u32;
    }

    let slice = unsafe {
        core::slice::from_raw_parts(core::ptr::addr_of!(INPUT).cast::<u8>(), len)
    };
    if let Ok(url_str) = core::str::from_utf8(slice) {
        check_url(url_str) as u32
    } else {
        Verdict::Pass as u32
    }
}

/// Writes cosmetic CSS for the URL in the input buffer into the output buffer.
///
/// Returns the number of bytes written.
///
/// # Safety
///
/// The caller must ensure `url_len` bytes in the input buffer are valid UTF-8.
#[no_mangle]
pub unsafe extern "C" fn rustad_get_cosmetics(url_len: usize) -> usize {
    let len = url_len.min(2048);
    if len == 0 {
        return 0;
    }

    let slice = unsafe {
        core::slice::from_raw_parts(core::ptr::addr_of!(INPUT).cast::<u8>(), len)
    };
    if let Ok(url_str) = core::str::from_utf8(slice) {
        let out_slice = unsafe {
            core::slice::from_raw_parts_mut(
                core::ptr::addr_of_mut!(OUTPUT).cast::<u8>(),
                rules::MAX_COSMETIC_CSS_LEN,
            )
        };
        write_cosmetic_css(url_str, out_slice)
    } else {
        0
    }
}

/// Writes matching scriptlets as a JSON string for the URL in the input buffer into the output buffer.
///
/// Returns the number of bytes written.
///
/// # Safety
///
/// The caller must ensure `url_len` bytes in the input buffer are valid UTF-8.
#[no_mangle]
pub unsafe extern "C" fn rustad_get_scriptlets(url_len: usize) -> usize {
    let len = url_len.min(2048);
    if len == 0 {
        return 0;
    }

    let slice = unsafe {
        core::slice::from_raw_parts(core::ptr::addr_of!(INPUT).cast::<u8>(), len)
    };
    if let Ok(url_str) = core::str::from_utf8(slice) {
        let out_slice = unsafe {
            core::slice::from_raw_parts_mut(
                core::ptr::addr_of_mut!(OUTPUT).cast::<u8>(),
                rules::MAX_COSMETIC_CSS_LEN,
            )
        };
        scriptlet::write_scriptlets_json(url_str, out_slice)
    } else {
        0
    }
}
