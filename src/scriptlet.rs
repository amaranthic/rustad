//! Scriptlet lookup and JSON emission for DOM/JS runtime defusing.

use crate::domain::{ascii_lowercase_cmp, extract_host};
use crate::rules;

/// Invokes `callback` for every domain-specific scriptlet matching `host` and its parent domains.
pub fn for_each_domain_scriptlet<F>(host: &str, mut callback: F)
where
    F: FnMut(&'static str),
{
    if host.is_empty() || rules::DOMAIN_SCRIPTLETS.is_empty() {
        return;
    }

    let mut current = host;
    loop {
        let start = rules::DOMAIN_SCRIPTLETS
            .partition_point(|&(d, _)| ascii_lowercase_cmp(d, current).is_lt());

        for &(d, scriptlet) in &rules::DOMAIN_SCRIPTLETS[start..] {
            if ascii_lowercase_cmp(d, current).is_ne() {
                break;
            }
            callback(scriptlet);
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

/// Generates a JSON array string of scriptlet definitions matching `url` directly into `out`.
///
/// Returns the number of bytes written.
pub fn write_scriptlets_json(url: &str, out: &mut [u8]) -> usize {
    let host = extract_host(url);
    let mut written = 0;
    let mut count = 0;

    if out.is_empty() {
        return 0;
    }

    out[written] = b'[';
    written += 1;

    for_each_domain_scriptlet(host, |scriptlet| {
        // Space needed: comma (if count > 0) + '"' + scriptlet + '"' + closing ']'
        let needed = if count > 0 { 1 } else { 0 } + 1 + scriptlet.len() + 1 + 1;
        if written + needed <= out.len() {
            if count > 0 {
                out[written] = b',';
                written += 1;
            }
            out[written] = b'"';
            written += 1;
            out[written..written + scriptlet.len()].copy_from_slice(scriptlet.as_bytes());
            written += scriptlet.len();
            out[written] = b'"';
            written += 1;
            count += 1;
        }
    });

    if written < out.len() {
        out[written] = b']';
        written += 1;
        written
    } else {
        0
    }
}

/// Returns matching scriptlets as a JSON `String` (useful for testing and native callers).
#[must_use]
pub fn get_scriptlets_json(url: &str) -> String {
    let mut buffer = vec![0u8; rules::MAX_SCRIPTLETS_LEN];
    let len = write_scriptlets_json(url, &mut buffer);
    String::from_utf8_lossy(&buffer[..len]).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scriptlet_lookup() {
        let json = get_scriptlets_json("https://www.youtube.com/watch?v=123");
        assert!(json.starts_with('['));
        assert!(json.ends_with(']'));
        assert!(json.contains("ytInitialPlayerResponse.adPlacements"));
    }
}
