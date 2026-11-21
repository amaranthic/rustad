// ==============================================================================
// RUSTAD COORDINATOR ENGINE (src/engine.rs)
// Browser-agnostic engine coordinating matching and cosmetic generation
// ==============================================================================

use crate::cosmetic::{for_each_domain_selector, global_selectors, is_excepted};
use crate::domain::{domain_is_allowed, domain_is_blocked, extract_host};
use crate::pattern::{url_matches_allowed_pattern, url_matches_blocked_pattern};
use crate::types::Verdict;

/// Evaluates a URL against all rules according to ABP priority:
/// 1. Whitelist / Allow rules (exceptions win unconditionally)
/// 2. Domain block rules
/// 3. Path / pattern block rules
/// 4. Pass (no rules matched)
pub fn check_url(url: &str) -> Verdict {
    let host = extract_host(url);

    // 1. Highest Priority: Allow / Exception rules
    if domain_is_allowed(host) || url_matches_allowed_pattern(url) {
        return Verdict::Allow;
    }

    // 2. High Speed: Domain block rules (O(log N) binary search)
    if domain_is_blocked(host) {
        return Verdict::Block;
    }

    // 3. Pattern / Wildcard block rules
    if url_matches_blocked_pattern(url) {
        return Verdict::Block;
    }

    Verdict::Pass
}

/// Generates a combined CSS string for a given URL/hostname into a pre-allocated byte slice.
/// Returns number of bytes written. Format:
/// "selector1, selector2 { display: none !important; }"
/// Zero heap allocations in hot path.
pub fn write_cosmetic_css(url: &str, out: &mut [u8]) -> usize {
    let host = extract_host(url);
    let mut written = 0;
    let mut count = 0;

    // Helper closure to append a string slice
    let append = |s: &str, out: &mut [u8], written: &mut usize| -> bool {
        let bytes = s.as_bytes();
        if *written + bytes.len() <= out.len() {
            out[*written..*written + bytes.len()].copy_from_slice(bytes);
            *written += bytes.len();
            true
        } else {
            false
        }
    };

    // 1. Global selectors
    for &sel in global_selectors() {
        if !is_excepted(host, sel) {
            if count > 0 && !append(", ", out, &mut written) {
                break;
            }
            if !append(sel, out, &mut written) {
                break;
            }
            count += 1;
        }
    }

    // 2. Domain-specific selectors
    for_each_domain_selector(host, |sel| {
        if !is_excepted(host, sel) {
            if count > 0 && !append(", ", out, &mut written) {
                return;
            }
            if !append(sel, out, &mut written) {
                return;
            }
            count += 1;
        }
    });

    if count > 0 {
        let suffix = " { display: none !important; }\n";
        append(suffix, out, &mut written);
    }

    written
}

/// Native convenience helper returning a String (useful for native CLI/tests)
pub fn get_cosmetic_css(url: &str) -> String {
    let mut buffer = [0u8; 8192];
    let len = write_cosmetic_css(url, &mut buffer);
    String::from_utf8_lossy(&buffer[..len]).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_check_url_verdicts() {
        // Blocked domain
        assert_eq!(
            check_url("https://adservice.google.com/ad.js"),
            Verdict::Block
        );
        assert_eq!(
            check_url("https://sub.doubleclick.net/pagead/id"),
            Verdict::Block
        );

        // Allowed exception overrides domain
        assert_eq!(
            check_url("https://adservice.google.com/essential.js"),
            Verdict::Allow
        );

        // Blocked pattern
        assert_eq!(
            check_url("https://somesite.com/banner-ad.js"),
            Verdict::Block
        );

        // Neutral / pass
        assert_eq!(
            check_url("https://wikipedia.org/wiki/Rust"),
            Verdict::Pass
        );
    }

    #[test]
    fn test_cosmetic_css_generation() {
        let css = get_cosmetic_css("https://youtube.com");
        assert!(css.contains("display: none !important;"));
        assert!(css.contains("#masthead-ad"));
        assert!(css.contains(".ad-banner"));
    }
}
