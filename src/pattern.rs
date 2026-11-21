//! Iterative wildcard (*) and separator (^) pattern matcher.

use crate::rules;

/// Returns `true` if the ASCII byte is a separator per the Adblock Plus specification.
///
/// A separator is any ASCII character that is not alphanumeric and not `_`, `-`, `.`, `%`.
#[inline]
pub fn is_separator(b: u8) -> bool {
    !b.is_ascii_alphanumeric() && b != b'_' && b != b'-' && b != b'.' && b != b'%'
}

/// Evaluates whether `text` matches `pat` using wildcards `*` and separators `^`.
///
/// Runs iteratively with $O(N)$ time complexity and zero heap allocations.
pub fn glob_match(text: &[u8], pat: &[u8]) -> bool {
    let mut t_idx = 0;
    let mut p_idx = 0;
    let mut star_p_idx = usize::MAX;
    let mut backtrack_t_idx = 0;

    while t_idx < text.len() {
        if p_idx < pat.len() && pat[p_idx] == b'*' {
            star_p_idx = p_idx;
            p_idx += 1;
            backtrack_t_idx = t_idx;
        } else if p_idx < pat.len() && pat[p_idx] == b'^' {
            if is_separator(text[t_idx]) {
                p_idx += 1;
                t_idx += 1;
            } else if star_p_idx != usize::MAX {
                p_idx = star_p_idx + 1;
                backtrack_t_idx += 1;
                t_idx = backtrack_t_idx;
            } else {
                return false;
            }
        } else if p_idx < pat.len() && pat[p_idx].eq_ignore_ascii_case(&text[t_idx]) {
            p_idx += 1;
            t_idx += 1;
        } else if star_p_idx != usize::MAX {
            p_idx = star_p_idx + 1;
            backtrack_t_idx += 1;
            t_idx = backtrack_t_idx;
        } else {
            return false;
        }
    }

    // Trailing * or ^ matches end-of-string
    while p_idx < pat.len() && (pat[p_idx] == b'*' || pat[p_idx] == b'^') {
        p_idx += 1;
    }

    p_idx == pat.len()
}

/// Checks if a URL matches a pattern rule with standard ABP anchors:
/// - `||pattern`: domain anchor (matches domain boundary or subdomains)
/// - `|pattern`: matches start of URL
/// - `pattern|`: matches end of URL
/// - `pattern`: matches anywhere in URL
pub fn matches_pattern(url: &str, pattern: &str) -> bool {
    let url_bytes = url.as_bytes();
    let pat_trimmed = pattern.trim();

    // 1. Domain anchor (||)
    if let Some(rest) = pat_trimmed.strip_prefix("||") {
        let (end_anchored, rest_no_end) = if let Some(stripped) = rest.strip_suffix('|') {
            (true, stripped)
        } else {
            (false, rest)
        };

        // Prepare pattern slice: if not end-anchored and doesn't end with '*', allow trailing chars
        let rest_bytes = rest_no_end.as_bytes();
        let mut p_buf = [0u8; 512];
        let p_match: &[u8] = if !end_anchored && !rest_no_end.ends_with('*') {
            if rest_bytes.len() < p_buf.len() {
                p_buf[..rest_bytes.len()].copy_from_slice(rest_bytes);
                p_buf[rest_bytes.len()] = b'*';
                &p_buf[..=rest_bytes.len()]
            } else {
                rest_bytes
            }
        } else {
            rest_bytes
        };

        let start = if let Some(pos) = url.find("://") {
            pos + 3
        } else {
            0
        };

        // Strip userinfo from domain search start if present
        let auth_end = url[start..].find(['/', '?', '#']).unwrap_or(url.len() - start);
        let host_start = if let Some(at_pos) = url[start..start + auth_end].find('@') {
            start + at_pos + 1
        } else {
            start
        };

        let mut current_offset = host_start;
        while current_offset < url_bytes.len() {
            if glob_match(&url_bytes[current_offset..], p_match) {
                return true;
            }

            // Subdomain walking: never cross into path, query, or fragment
            let authority_remaining = url[current_offset..].find(['/', '?', '#']).unwrap_or(url.len() - current_offset);
            if let Some(next_dot) = url[current_offset..current_offset + authority_remaining].find('.') {
                current_offset += next_dot + 1;
            } else {
                break;
            }
        }
        return false;
    }

    // 2. Exact start anchor (|)
    let (match_start_only, pat_after_start) = if let Some(rest) = pat_trimmed.strip_prefix('|') {
        (true, rest)
    } else {
        (false, pat_trimmed)
    };

    // 3. Exact end anchor (|)
    let (match_end_only, final_pat) = if let Some(rest) = pat_after_start.strip_suffix('|') {
        (true, rest)
    } else {
        (false, pat_after_start)
    };

    let p_bytes = final_pat.as_bytes();

    if match_start_only && match_end_only {
        glob_match(url_bytes, p_bytes)
    } else if match_start_only {
        let mut p_with_star = [0u8; 512];
        if p_bytes.len() < p_with_star.len() {
            p_with_star[..p_bytes.len()].copy_from_slice(p_bytes);
            p_with_star[p_bytes.len()] = b'*';
            glob_match(url_bytes, &p_with_star[..=p_bytes.len()])
        } else {
            glob_match(url_bytes, p_bytes)
        }
    } else if match_end_only {
        let mut p_with_star = [0u8; 512];
        if p_bytes.len() < p_with_star.len() {
            p_with_star[0] = b'*';
            p_with_star[1..=p_bytes.len()].copy_from_slice(p_bytes);
            glob_match(url_bytes, &p_with_star[..=p_bytes.len()])
        } else {
            glob_match(url_bytes, p_bytes)
        }
    } else {
        let mut p_with_stars = [0u8; 514];
        if p_bytes.len() + 1 < p_with_stars.len() {
            p_with_stars[0] = b'*';
            p_with_stars[1..=p_bytes.len()].copy_from_slice(p_bytes);
            p_with_stars[p_bytes.len() + 1] = b'*';
            glob_match(url_bytes, &p_with_stars[..p_bytes.len() + 2])
        } else {
            glob_match(url_bytes, p_bytes)
        }
    }
}

/// Checks if `url` matches any pattern in `BLOCKED_PATTERNS`.
pub fn url_matches_blocked_pattern(url: &str) -> bool {
    for pat in rules::BLOCKED_PATTERNS {
        if matches_pattern(url, pat) {
            return true;
        }
    }
    false
}

/// Checks if `url` matches any pattern in `ALLOWED_PATTERNS`.
pub fn url_matches_allowed_pattern(url: &str) -> bool {
    for pat in rules::ALLOWED_PATTERNS {
        if matches_pattern(url, pat) {
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_glob_match_simple() {
        assert!(glob_match(b"https://example.com/ads.js", b"*ads.js"));
        assert!(glob_match(b"https://example.com/banner-123.png", b"*banner-*.png"));
        assert!(!glob_match(b"https://example.com/safe.png", b"*banner-*.png"));
    }

    #[test]
    fn test_glob_match_separator() {
        assert!(glob_match(b"example.com/path", b"example.com^*"));
        assert!(glob_match(b"example.com?query", b"example.com^*"));
        assert!(!glob_match(b"example.commercial.com", b"example.com^*"));
    }

    #[test]
    fn test_matches_pattern_anchors() {
        assert!(matches_pattern("https://example.com/ads/banner.js", "/ads/"));
        assert!(matches_pattern("https://sub.example.com/tracking/123", "||example.com/tracking/*"));
        assert!(matches_pattern("https://example.com/ads/banner.js", "||example.com/ads^"));
        assert!(matches_pattern("https://example.com/ads", "||example.com/ads^"));
        assert!(matches_pattern("https://example.com?query=1.2.3", "||example.com^"));
        assert!(matches_pattern("https://user:pass@sub.example.com/ads/1", "||example.com/ads^"));
        assert!(matches_pattern("https://tracker.com/pixel.gif", "|https://tracker.com*"));
    }
}
