// ==============================================================================
// RUSTAD PATTERN MATCHER (src/pattern.rs)
// Iterative, zero-recursion, zero-allocation glob (*) & separator (^) matcher
// ==============================================================================

use crate::rules;

/// Separator character per Adblock Plus specification:
/// Any ASCII byte that is NOT alphanumeric and NOT '_', '-', '.', '%'
#[inline]
pub fn is_separator(b: u8) -> bool {
    !b.is_ascii_alphanumeric() && b != b'_' && b != b'-' && b != b'.' && b != b'%'
}

/// Matches `text` against a pattern containing `*` wildcards and `^` separators.
/// Pure iterative matcher: O(N) time, zero recursion, zero heap allocations.
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
        } else if p_idx < pat.len()
            && pat[p_idx].to_ascii_lowercase() == text[t_idx].to_ascii_lowercase()
        {
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

/// Checks if a URL matches a rule pattern with ABP semantics:
/// - "||pattern": domain anchor (matches at domain boundary or any subdomain)
/// - "|pattern": matches beginning of URL
/// - "pattern|": matches end of URL
/// - "pattern": matches anywhere in URL
pub fn matches_pattern(url: &str, pattern: &str) -> bool {
    let url_bytes = url.as_bytes();
    let pat_trimmed = pattern.trim();

    // 1. Domain anchor (||)
    if let Some(rest) = pat_trimmed.strip_prefix("||") {
        // Find where host starts in url
        let start = if let Some(pos) = url.find("://") {
            pos + 3
        } else {
            0
        };

        let mut current_offset = start;
        while current_offset < url_bytes.len() {
            if glob_match(&url_bytes[current_offset..], rest.as_bytes()) {
                return true;
            }
            // Move to next subdomain dot until path start
            if let Some(next_dot) = url[current_offset..].find('.') {
                let abs_dot = current_offset + next_dot + 1;
                // Don't cross into the path
                if let Some(slash_pos) = url[current_offset..].find('/') {
                    if next_dot > slash_pos {
                        break;
                    }
                }
                current_offset = abs_dot;
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
        // Match from start of URL, allow trailing characters
        let mut p_with_star = [0u8; 512];
        if p_bytes.len() + 1 <= p_with_star.len() {
            p_with_star[..p_bytes.len()].copy_from_slice(p_bytes);
            p_with_star[p_bytes.len()] = b'*';
            glob_match(url_bytes, &p_with_star[..p_bytes.len() + 1])
        } else {
            glob_match(url_bytes, p_bytes)
        }
    } else if match_end_only {
        // Match at end of URL
        let mut p_with_star = [0u8; 512];
        if p_bytes.len() + 1 <= p_with_star.len() {
            p_with_star[0] = b'*';
            p_with_star[1..p_bytes.len() + 1].copy_from_slice(p_bytes);
            glob_match(url_bytes, &p_with_star[..p_bytes.len() + 1])
        } else {
            glob_match(url_bytes, p_bytes)
        }
    } else {
        // Default: matches anywhere in URL
        let mut p_with_stars = [0u8; 514];
        if p_bytes.len() + 2 <= p_with_stars.len() {
            p_with_stars[0] = b'*';
            p_with_stars[1..p_bytes.len() + 1].copy_from_slice(p_bytes);
            p_with_stars[p_bytes.len() + 1] = b'*';
            glob_match(url_bytes, &p_with_stars[..p_bytes.len() + 2])
        } else {
            glob_match(url_bytes, p_bytes)
        }
    }
}

pub fn url_matches_blocked_pattern(url: &str) -> bool {
    for pat in rules::BLOCKED_PATTERNS {
        if matches_pattern(url, pat) {
            return true;
        }
    }
    false
}

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
        assert!(matches_pattern("https://tracker.com/pixel.gif", "|https://tracker.com*"));
    }
}
