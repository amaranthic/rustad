//! High-level coordinator evaluating network requests and building cosmetic CSS rules.

use crate::cosmetic::{for_each_domain_selector, global_selectors, is_excepted};
use crate::domain::{domain_is_allowed, domain_is_blocked, extract_host};
use crate::pattern::{url_matches_allowed_pattern, url_matches_blocked_pattern};

/// Verdict returned when evaluating a URL against filter rules.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum Verdict {
    /// No matching rules found; request should proceed.
    Pass = 0,
    /// Request matched a blocking rule; request should be canceled or blocked.
    Block = 1,
    /// Request matched an explicit whitelist/exception rule; request must be allowed.
    Allow = 2,
}

impl Verdict {
    /// Returns `true` if the verdict indicates the request should be blocked.
    #[inline]
    #[must_use]
    pub fn is_blocked(self) -> bool {
        matches!(self, Verdict::Block)
    }

    /// Returns `true` if the verdict indicates the request is explicitly allowed.
    #[inline]
    #[must_use]
    pub fn is_allowed(self) -> bool {
        matches!(self, Verdict::Allow)
    }

    /// Returns `true` if the request passed through without matching rules.
    #[inline]
    #[must_use]
    pub fn is_pass(self) -> bool {
        matches!(self, Verdict::Pass)
    }
}

/// Evaluates `url` against filter rules and returns a `Verdict`.
///
/// Priority order follows standard ad-blocking rules:
/// 1. Whitelist / Allow rules (`@@`)
/// 2. Blocked domain rules (`||domain^`)
/// 3. Blocked pattern rules (`/ads/`, `*wildcard*`)
/// 4. Pass (no rules matched)
#[must_use]
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

/// Generates cosmetic CSS hiding rules for `url` directly into `out`.
///
/// Hardened to completely collapse layout boxes, padding, margins, and borders.
/// Guaranteed to leave room for the CSS rule suffix so rules are always valid.
pub fn write_cosmetic_css(url: &str, out: &mut [u8]) -> usize {
    let host = extract_host(url);
    let mut written = 0;
    let mut count = 0;

    let suffix = " { display: none !important; visibility: hidden !important; opacity: 0 !important; pointer-events: none !important; height: 0 !important; min-height: 0 !important; max-height: 0 !important; width: 0 !important; min-width: 0 !important; max-width: 0 !important; margin: 0 !important; padding: 0 !important; border: none !important; overflow: hidden !important; line-height: 0 !important; font-size: 0 !important; }\n";
    let max_content_len = out.len().saturating_sub(suffix.len() + 16);

    let append = |s: &str, out: &mut [u8], written: &mut usize, limit: usize| -> bool {
        let bytes = s.as_bytes();
        if *written + bytes.len() <= limit {
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
            if count > 0 && !append(", ", out, &mut written, max_content_len) {
                break;
            }
            if !append(sel, out, &mut written, max_content_len) {
                break;
            }
            count += 1;
        }
    }

    // 2. Domain-specific selectors
    for_each_domain_selector(host, |sel| {
        if !is_excepted(host, sel) {
            if count > 0 && !append(", ", out, &mut written, max_content_len) {
                return;
            }
            if !append(sel, out, &mut written, max_content_len) {
                return;
            }
            count += 1;
        }
    });

    if count > 0 {
        // Append closing collapse styles
        let suffix_bytes = suffix.as_bytes();
        if written + suffix_bytes.len() <= out.len() {
            out[written..written + suffix_bytes.len()].copy_from_slice(suffix_bytes);
            written += suffix_bytes.len();
        }
    }

    written
}

/// Generates cosmetic CSS as a `String` (useful for native applications and tests).
#[must_use]
pub fn get_cosmetic_css(url: &str) -> String {
    let mut buffer = vec![0u8; crate::rules::MAX_COSMETIC_CSS_LEN];
    let len = write_cosmetic_css(url, &mut buffer);
    String::from_utf8_lossy(&buffer[..len]).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_check_url_verdicts() {
        assert_eq!(
            check_url("https://adservice.google.com/ad.js"),
            Verdict::Block
        );
        assert_eq!(
            check_url("https://sub.doubleclick.net/pagead/id"),
            Verdict::Block
        );
        assert_eq!(
            check_url("https://adservice.google.com/essential.js"),
            Verdict::Allow
        );
        assert_eq!(
            check_url("https://somesite.com/banner-ad.js"),
            Verdict::Block
        );
        assert_eq!(
            check_url("https://wikipedia.org/wiki/Rust"),
            Verdict::Pass
        );
    }

    #[test]
    fn test_cosmetic_css_generation() {
        let css = get_cosmetic_css("https://youtube.com");
        assert!(css.contains("display: none !important;"));
        assert!(css.contains("visibility: hidden !important;"));
        assert!(css.contains("height: 0 !important;"));
        assert!(css.contains("#masthead-ad"));
        assert!(css.contains(".ad-banner"));
    }
}
