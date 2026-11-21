// ==============================================================================
// RUSTAD ROOT (src/lib.rs)
// Public Rust API + Minimal C-ABI WebAssembly Bridge
// ==============================================================================

pub mod cosmetic;
pub mod domain;
pub mod engine;
pub mod pattern;
pub mod types;

#[path = "generated/rules.rs"]
pub mod rules;

// Re-export primary engine functions for idiomatic Rust callers
pub use engine::{check_url, get_cosmetic_css, write_cosmetic_css};
pub use types::Verdict;

// ==============================================================================
// C-ABI / WebAssembly Exports
// Zero-allocation memory protocol for JS / WebAssembly runtimes
// ==============================================================================

/// Static 16KB scratch buffer for passing strings between JS and WASM
static mut SCRATCH: [u8; 16384] = [0u8; 16384];

/// Returns pointer to static scratch buffer
#[no_mangle]
pub unsafe extern "C" fn rustad_scratch_ptr() -> *mut u8 {
    core::ptr::addr_of_mut!(SCRATCH) as *mut u8
}

/// Returns capacity of static scratch buffer
#[no_mangle]
pub unsafe extern "C" fn rustad_scratch_len() -> usize {
    16384
}

/// Checks URL passed in linear memory:
/// Returns:
/// 0 = Pass
/// 1 = Block
/// 2 = Allow
#[no_mangle]
pub unsafe extern "C" fn rustad_check_url(url_ptr: *const u8, url_len: usize) -> u32 {
    if url_ptr.is_null() || url_len == 0 {
        return Verdict::Pass as u32;
    }

    let slice = core::slice::from_raw_parts(url_ptr, url_len);
    if let Ok(url_str) = core::str::from_utf8(slice) {
        check_url(url_str) as u32
    } else {
        Verdict::Pass as u32
    }
}

/// Writes cosmetic CSS for given URL into SCRATCH buffer.
/// Returns number of bytes written.
#[no_mangle]
pub unsafe extern "C" fn rustad_get_cosmetics(url_ptr: *const u8, url_len: usize) -> usize {
    if url_ptr.is_null() || url_len == 0 {
        return 0;
    }

    let slice = core::slice::from_raw_parts(url_ptr, url_len);
    if let Ok(url_str) = core::str::from_utf8(slice) {
        let scratch_slice =
            core::slice::from_raw_parts_mut(core::ptr::addr_of_mut!(SCRATCH) as *mut u8, 16384);
        write_cosmetic_css(url_str, scratch_slice)
    } else {
        0
    }
}
