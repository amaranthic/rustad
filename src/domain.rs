// ==============================================================================
// RUSTAD DOMAIN MATCHER (src/domain.rs)
// Ultra-fast, zero-allocation hostname extraction & binary search lookup
// ==============================================================================

use crate::rules;

#[inline]
pub fn ascii_lowercase_cmp(a: &str, b: &str) -> core::cmp::Ordering {
    let mut a_bytes = a.bytes().map(|c| c.to_ascii_lowercase());
    let mut b_bytes = b.bytes().map(|c| c.to_ascii_lowercase());
    loop {
        match (a_bytes.next(), b_bytes.next()) {
            (Some(x), Some(y)) => match x.cmp(&y) {
                core::cmp::Ordering::Equal => continue,
                other => return other,
            },
            (None, None) => return core::cmp::Ordering::Equal,
            (None, Some(_)) => return core::cmp::Ordering::Less,
            (Some(_), None) => return core::cmp::Ordering::Greater,
        }
    }
}

/// Extracts hostname from a full or partial URL without heap allocation.
/// "https://sub.example.com:8080/path?q=1" -> "sub.example.com"
pub fn extract_host(url: &str) -> &str {
    let mut s = url.trim();

    // Strip scheme if present
    if let Some(pos) = s.find("://") {
        s = &s[pos + 3..];
    }

    // Cut off path, query, fragment
    let host_end = s
        .find(|c| c == '/' || c == '?' || c == '#')
        .unwrap_or(s.len());
    s = &s[..host_end];

    // Strip port if present
    if let Some(colon_pos) = s.rfind(':') {
        // Ensure it's not an IPv6 address literal
        if !s.ends_with(']') {
            s = &s[..colon_pos];
        }
    }

    // Strip userinfo if present (user:pass@host)
    if let Some(at_pos) = s.find('@') {
        s = &s[at_pos + 1..];
    }

    s.trim_matches('.')
}

/// Checks if `host` or any of its parent domains exist in `list` using binary search.
/// For example, for "tracker.sub.doubleclick.net":
/// Checks "tracker.sub.doubleclick.net", then "sub.doubleclick.net", then "doubleclick.net".
pub fn is_domain_in_list(list: &[&str], host: &str) -> bool {
    if host.is_empty() || list.is_empty() {
        return false;
    }

    let mut current = host;
    loop {
        // Binary search the sorted list
        if list
            .binary_search_by(|probe| ascii_lowercase_cmp(probe, current))
            .is_ok()
        {
            return true;
        }

        // Advance to next parent subdomain
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

#[inline]
pub fn domain_is_blocked(host: &str) -> bool {
    is_domain_in_list(rules::BLOCKED_DOMAINS, host)
}

#[inline]
pub fn domain_is_allowed(host: &str) -> bool {
    is_domain_in_list(rules::ALLOWED_DOMAINS, host)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_host() {
        assert_eq!(extract_host("https://example.com/path"), "example.com");
        assert_eq!(extract_host("http://sub.domain.org:8080/a?b=c"), "sub.domain.org");
        assert_eq!(extract_host("doubleclick.net/ads.js"), "doubleclick.net");
        assert_eq!(extract_host("https://user:pass@secret.com:443/"), "secret.com");
    }

    #[test]
    fn test_is_domain_in_list() {
        let list = &["doubleclick.net", "google.com"];
        assert!(is_domain_in_list(list, "doubleclick.net"));
        assert!(is_domain_in_list(list, "ad.doubleclick.net"));
        assert!(is_domain_in_list(list, "sub.ad.doubleclick.net"));
        assert!(!is_domain_in_list(list, "notdoubleclick.net"));
        assert!(!is_domain_in_list(list, "yahoo.com"));
    }
}
