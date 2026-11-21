// ==============================================================================
// RUSTAD COSMETIC ENGINE (src/cosmetic.rs)
// Fast CSS element hiding selector resolution & exception (#@#) handling
// ==============================================================================

use crate::domain::ascii_lowercase_cmp;
use crate::rules;

/// Checks if selector is excepted for the given host in COSMETIC_EXCEPTIONS.
pub fn is_excepted(host: &str, selector: &str) -> bool {
    if host.is_empty() || rules::COSMETIC_EXCEPTIONS.is_empty() {
        return false;
    }

    let mut current = host;
    loop {
        // Binary search for any exception matching current domain
        let start = rules::COSMETIC_EXCEPTIONS
            .partition_point(|&(d, _)| ascii_lowercase_cmp(d, current).is_lt());

        for &(d, sel) in &rules::COSMETIC_EXCEPTIONS[start..] {
            if ascii_lowercase_cmp(d, current).is_ne() {
                break;
            }
            if sel == selector {
                return true;
            }
        }

        if let Some(dot_pos) = current.find('.') {
            current = &current[dot_pos + 1..];
            if current.is_empty() {
                break;
            }
        } else {
            break;
        }
    }

    false
}

/// Iterates all matching domain-specific selectors for `host` (and parent domains).
pub fn for_each_domain_selector<F>(host: &str, mut callback: F)
where
    F: FnMut(&'static str),
{
    if host.is_empty() || rules::DOMAIN_COSMETIC.is_empty() {
        return;
    }

    let mut current = host;
    loop {
        let start = rules::DOMAIN_COSMETIC
            .partition_point(|&(d, _)| ascii_lowercase_cmp(d, current).is_lt());

        for &(d, sel) in &rules::DOMAIN_COSMETIC[start..] {
            if ascii_lowercase_cmp(d, current).is_ne() {
                break;
            }
            callback(sel);
        }

        if let Some(dot_pos) = current.find('.') {
            current = &current[dot_pos + 1..];
            if current.is_empty() {
                break;
            }
        } else {
            break;
        }
    }
}

/// Returns global cosmetic selectors.
#[inline]
pub fn global_selectors() -> &'static [&'static str] {
    rules::GLOBAL_COSMETIC
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cosmetic_lookup() {
        let mut count = 0;
        for_each_domain_selector("youtube.com", |_sel| {
            count += 1;
        });
        assert!(count >= 2);
    }

    #[test]
    fn test_cosmetic_exception() {
        assert!(is_excepted("news.ycombinator.com", ".sponsor-banner"));
        assert!(!is_excepted("youtube.com", "#masthead-ad"));
    }
}
