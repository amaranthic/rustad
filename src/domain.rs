//! Domain matching, hostname extraction, and binary search lookups.

use crate::rules;

/// Compares two strings case-insensitively byte-by-byte without heap allocations.
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

/// Extracts the hostname component from a URL without heap allocation.
///
/// Correctly handles userinfo (`user:pass@`), ports, and IPv6 literals.
///
/// # Examples
///
/// ```
/// use rustad::domain::extract_host;
/// assert_eq!(extract_host("https://sub.example.com/path"), "sub.example.com");
/// assert_eq!(extract_host("https://alice:secret@example.com:8080/"), "example.com");
/// assert_eq!(extract_host("http://[::1]:8080/"), "[::1]");
/// ```
pub fn extract_host(url: &str) -> &str {
    let mut s = url.trim();

    // Strip scheme if present
    if let Some(pos) = s.find("://") {
        s = &s[pos + 3..];
    }

    // Cut off path, query, fragment
    let host_end = s.find(['/', '?', '#']).unwrap_or(s.len());
    s = &s[..host_end];

    // Strip userinfo FIRST if present (e.g. user:pass@host)
    if let Some(at_pos) = s.find('@') {
        s = &s[at_pos + 1..];
    }

    // Handle port and IPv6 brackets
    if s.starts_with('[') {
        // IPv6 literal: find closing ']'
        if let Some(closing) = s.find(']') {
            s = &s[..=closing];
        }
    } else if let Some(colon_pos) = s.rfind(':') {
        s = &s[..colon_pos];
    }

    s.trim_matches('.')
}

/// Checks if `host` or any of its parent domains exist in `list` using binary search.
///
/// For example, checking `"tracker.sub.doubleclick.net"` checks:
/// 1. `"tracker.sub.doubleclick.net"`
/// 2. `"sub.doubleclick.net"`
/// 3. `"doubleclick.net"`
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

/// Checks if `host` or any parent domain matches a blocked domain.
#[inline]
pub fn domain_is_blocked(host: &str) -> bool {
    is_domain_in_list(rules::BLOCKED_DOMAINS, host)
}

/// Checks if `host` or any parent domain matches an allowed/whitelisted domain.
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
        assert_eq!(extract_host("https://alice:secret123@example.com/"), "example.com");
        assert_eq!(extract_host("http://[::1]:8080/path"), "[::1]");
        assert_eq!(extract_host("http://[2001:db8::1]/path"), "[2001:db8::1]");
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
