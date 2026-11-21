//! Build script for Rustad: compiles the `rules/` directory into:
//! 1. `src/generated/rules.rs` (pre-sorted Rust slices for native engine)
//! 2. `rustad/rules.json` (Chrome Manifest V3 declarativeNetRequest rules)

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

fn clean_domain(raw: &str) -> String {
    let mut s = raw.trim();
    if let Some(stripped) = s.strip_prefix("||") {
        s = stripped;
    }
    if let Some(stripped) = s.strip_prefix('*') {
        s = stripped;
    }
    if let Some(stripped) = s.strip_prefix('.') {
        s = stripped;
    }
    if let Some(stripped) = s.strip_suffix('^') {
        s = stripped;
    }
    s.trim().to_ascii_lowercase()
}

fn is_valid_domain(d: &str) -> bool {
    if d.is_empty() || d.len() > 253 || !d.contains('.') {
        return false;
    }
    if d.starts_with('.') || d.ends_with('.') || d.starts_with('-') || d.ends_with('-') {
        return false;
    }
    d.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'.' || b == b'-')
}

fn escape_json(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

fn is_valid_url_filter(s: &str) -> bool {
    if s.is_empty() || s.len() > 256 || !s.is_ascii() {
        return false;
    }
    // Cannot contain whitespace or control chars
    if s.bytes().any(|b| b <= b' ' || b == 127) {
        return false;
    }
    // Cannot contain cosmetic or regex syntax
    if s.contains('#') || s.contains('\\') || s.contains('[') || s.contains(']') || s.contains('{') || s.contains('}') {
        return false;
    }
    // Cannot be a regex rule /.../
    if s.starts_with('/') && s.ends_with('/') && s.len() > 2 {
        return false;
    }
    // Cannot start with ||* or |* or ^
    if s.starts_with("||*") || s.starts_with("|*") || s.starts_with('^') {
        return false;
    }
    // Cannot end with *| or ^|
    if s.ends_with("*|") || s.ends_with("^|") {
        return false;
    }
    // Cannot contain || anywhere except at the start
    if s.len() > 2 && s[2..].contains("||") {
        return false;
    }
    // Cannot contain consecutive wildcards
    if s.contains("**") {
        return false;
    }
    true
}

fn main() {
    println!("cargo:rerun-if-changed=rules/domains.txt");
    println!("cargo:rerun-if-changed=rules/patterns.txt");
    println!("cargo:rerun-if-changed=rules/exceptions.txt");
    println!("cargo:rerun-if-changed=rules/cosmetics.txt");
    println!("cargo:rerun-if-changed=rules/scriptlets.txt");

    let mut blocked_domains = BTreeSet::new();
    let mut blocked_patterns = BTreeSet::new();
    let mut allowed_domains = BTreeSet::new();
    let mut allowed_patterns = BTreeSet::new();
    let mut global_cosmetics = BTreeSet::new();
    let mut domain_cosmetics = BTreeSet::new();
    let mut cosmetic_exceptions = BTreeSet::new();
    let mut domain_scriptlets = BTreeSet::new();

    // 1. Process rules/domains.txt
    if let Ok(content) = fs::read_to_string("rules/domains.txt") {
        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('!') || line.starts_with('[') {
                continue;
            }
            let dom = clean_domain(line);
            if !dom.is_empty() {
                blocked_domains.insert(dom);
            }
        }
    }

    // 2. Process rules/patterns.txt
    if let Ok(content) = fs::read_to_string("rules/patterns.txt") {
        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('!') || line.starts_with('[') {
                continue;
            }
            blocked_patterns.insert(line.to_string());
        }
    }

    // 3. Process rules/exceptions.txt
    if let Ok(content) = fs::read_to_string("rules/exceptions.txt") {
        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('!') || line.starts_with('[') {
                continue;
            }
            let rule = if let Some(stripped) = line.strip_prefix("@@") {
                stripped.trim()
            } else {
                line
            };

            if rule.starts_with("||") && rule.ends_with('^') && !rule[2..rule.len()-1].contains('/') && !rule[2..rule.len()-1].contains('*') {
                let dom = clean_domain(rule);
                if !dom.is_empty() {
                    allowed_domains.insert(dom);
                }
            } else {
                allowed_patterns.insert(rule.to_string());
            }
        }
    }

    // 4. Process rules/cosmetics.txt
    if let Ok(content) = fs::read_to_string("rules/cosmetics.txt") {
        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('!') || line.starts_with('[') {
                continue;
            }

            if let Some(pos) = line.find("#@#") {
                let domains_part = &line[..pos];
                let selector = line[pos + 3..].trim();
                for dom in domains_part.split(',') {
                    let d = clean_domain(dom);
                    if !d.is_empty() && !selector.is_empty() {
                        cosmetic_exceptions.insert((d, selector.to_string()));
                    }
                }
            } else if let Some(stripped) = line.strip_prefix("##") {
                let selector = stripped.trim();
                if !selector.is_empty() {
                    global_cosmetics.insert(selector.to_string());
                }
            } else if let Some(pos) = line.find("##") {
                let domains_part = &line[..pos];
                let selector = line[pos + 2..].trim();
                for dom in domains_part.split(',') {
                    let d = clean_domain(dom);
                    if !d.is_empty() && !selector.is_empty() {
                        domain_cosmetics.insert((d, selector.to_string()));
                    }
                }
            }
        }
    }

    // 5. Process rules/scriptlets.txt
    if let Ok(content) = fs::read_to_string("rules/scriptlets.txt") {
        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('!') || line.starts_with('[') {
                continue;
            }

            if let Some(pos) = line.find("##+js(") {
                let domains_part = &line[..pos];
                let body = line[pos + 6..].trim();
                let scriptlet = body.strip_suffix(')').unwrap_or(body).trim();
                for dom in domains_part.split(',') {
                    let d = clean_domain(dom);
                    if !d.is_empty() && !scriptlet.is_empty() {
                        domain_scriptlets.insert((d, scriptlet.to_string()));
                    }
                }
            }
        }
    }

    // Ensure output directories exist
    let _ = fs::create_dir_all("src/generated");
    let _ = fs::create_dir_all("rustad");

    // -------------------------------------------------------------------------
    // Calculate compile-time buffer size for cosmetic CSS
    // -------------------------------------------------------------------------
    let global_css_len: usize = global_cosmetics.iter().map(|s| s.len() + 2).sum();
    let mut domain_sizes: std::collections::BTreeMap<&str, usize> = std::collections::BTreeMap::new();
    for (d, s) in &domain_cosmetics {
        *domain_sizes.entry(d.as_str()).or_default() += s.len() + 2;
    }
    let max_domain_selectors_len = domain_sizes.values().max().copied().unwrap_or(0);
    let suffix_len = 512;
    let worst_case_len = global_css_len + max_domain_selectors_len + suffix_len + 1024;
    let wasm_page = 65536;
    let max_cosmetic_len = std::cmp::max(wasm_page, worst_case_len.div_ceil(wasm_page) * wasm_page);

    // -------------------------------------------------------------------------
    // Calculate compile-time buffer size for scriptlet JSON
    // -------------------------------------------------------------------------
    let mut scriptlet_domain_sizes: std::collections::BTreeMap<&str, usize> = std::collections::BTreeMap::new();
    for (d, s) in &domain_scriptlets {
        *scriptlet_domain_sizes.entry(d.as_str()).or_default() += s.len() + 4;
    }
    let max_scriptlet_domain_len = scriptlet_domain_sizes.values().max().copied().unwrap_or(0);
    let max_scriptlets_len = std::cmp::max(8192, (max_scriptlet_domain_len + 512).next_power_of_two());

    // -------------------------------------------------------------------------
    // Emit src/generated/rules.rs
    // -------------------------------------------------------------------------
    let mut out_rs = String::new();
    out_rs.push_str("// AUTO-GENERATED BY build.rs — DO NOT EDIT MANUALLY\n");
    out_rs.push_str("//! Auto-generated static filter rule definitions.\n\n");

    out_rs.push_str("/// Compile-time calculated maximum output buffer size for cosmetic CSS.\n");
    out_rs.push_str("/// Sized to fit the worst-case combination of global and domain rules in whole 64KB WASM pages.\n");
    out_rs.push_str(&format!("pub const MAX_COSMETIC_CSS_LEN: usize = {};\n\n", max_cosmetic_len));

    out_rs.push_str("/// Compile-time calculated maximum buffer size for scriptlet JSON output.\n");
    out_rs.push_str(&format!("pub const MAX_SCRIPTLETS_LEN: usize = {};\n\n", max_scriptlets_len));

    out_rs.push_str("/// Lexicographically sorted slice of blocked domain names.\n");
    out_rs.push_str("pub static BLOCKED_DOMAINS: &[&str] = &[\n");
    for d in &blocked_domains {
        out_rs.push_str(&format!("    {:?},\n", d));
    }
    out_rs.push_str("];\n\n");

    out_rs.push_str("/// Slice of blocked path, wildcard, and separator patterns.\n");
    out_rs.push_str("pub static BLOCKED_PATTERNS: &[&str] = &[\n");
    for p in &blocked_patterns {
        out_rs.push_str(&format!("    {:?},\n", p));
    }
    out_rs.push_str("];\n\n");

    out_rs.push_str("/// Lexicographically sorted slice of whitelisted/allowed domain names.\n");
    out_rs.push_str("pub static ALLOWED_DOMAINS: &[&str] = &[\n");
    for d in &allowed_domains {
        out_rs.push_str(&format!("    {:?},\n", d));
    }
    out_rs.push_str("];\n\n");

    out_rs.push_str("/// Slice of whitelisted/allowed URL patterns.\n");
    out_rs.push_str("pub static ALLOWED_PATTERNS: &[&str] = &[\n");
    for p in &allowed_patterns {
        out_rs.push_str(&format!("    {:?},\n", p));
    }
    out_rs.push_str("];\n\n");

    out_rs.push_str("/// Global CSS element-hiding selectors applied across all domains.\n");
    out_rs.push_str("pub static GLOBAL_COSMETIC: &[&str] = &[\n");
    for s in &global_cosmetics {
        out_rs.push_str(&format!("    {:?},\n", s));
    }
    out_rs.push_str("];\n\n");

    out_rs.push_str("/// Domain-specific CSS element-hiding selectors sorted by domain.\n");
    out_rs.push_str("pub static DOMAIN_COSMETIC: &[(&str, &str)] = &[\n");
    for (d, s) in &domain_cosmetics {
        out_rs.push_str(&format!("    ({:?}, {:?}),\n", d, s));
    }
    out_rs.push_str("];\n\n");

    out_rs.push_str("/// Domain-specific cosmetic exceptions (#@#) sorted by domain.\n");
    out_rs.push_str("pub static COSMETIC_EXCEPTIONS: &[(&str, &str)] = &[\n");
    for (d, s) in &cosmetic_exceptions {
        out_rs.push_str(&format!("    ({:?}, {:?}),\n", d, s));
    }
    out_rs.push_str("];\n\n");

    out_rs.push_str("/// Domain-specific scriptlet injection rules sorted by domain.\n");
    out_rs.push_str("pub static DOMAIN_SCRIPTLETS: &[(&str, &str)] = &[\n");
    for (d, s) in &domain_scriptlets {
        out_rs.push_str(&format!("    ({:?}, {:?}),\n", d, s));
    }
    out_rs.push_str("];\n");

    fs::write("src/generated/rules.rs", out_rs).expect("Failed to write src/generated/rules.rs");

    // -------------------------------------------------------------------------
    // Emit rustad/rules.json (Chrome MV3 DeclarativeNetRequest)
    // -------------------------------------------------------------------------
    let mut out_json = String::new();
    out_json.push_str("[\n");

    let mut rule_id = 1;
    let resource_types = "[\"main_frame\",\"sub_frame\",\"stylesheet\",\"script\",\"image\",\"font\",\"object\",\"xmlhttprequest\",\"ping\",\"csp_report\",\"media\",\"websocket\",\"other\"]";

    // 1. Exception rules (priority 2, allow)
    for d in &allowed_domains {
        if !is_valid_domain(d) { continue; }
        if rule_id > 1 { out_json.push_str(",\n"); }
        out_json.push_str(&format!(
            "  {{\"id\":{},\"priority\":2,\"action\":{{\"type\":\"allow\"}},\"condition\":{{\"urlFilter\":\"||{}^\",\"resourceTypes\":{}}}}}",
            rule_id, escape_json(d), resource_types
        ));
        rule_id += 1;
    }
    for p in &allowed_patterns {
        if !is_valid_url_filter(p) { continue; }
        if rule_id > 1 { out_json.push_str(",\n"); }
        out_json.push_str(&format!(
            "  {{\"id\":{},\"priority\":2,\"action\":{{\"type\":\"allow\"}},\"condition\":{{\"urlFilter\":\"{}\",\"resourceTypes\":{}}}}}",
            rule_id, escape_json(p), resource_types
        ));
        rule_id += 1;
    }

    // 2. Blocked pattern rules (priority 1, block - highest impact paths/wildcards)
    for p in &blocked_patterns {
        if !is_valid_url_filter(p) { continue; }
        if rule_id > 1 { out_json.push_str(",\n"); }
        out_json.push_str(&format!(
            "  {{\"id\":{},\"priority\":1,\"action\":{{\"type\":\"block\"}},\"condition\":{{\"urlFilter\":\"{}\",\"resourceTypes\":{}}}}}",
            rule_id, escape_json(p), resource_types
        ));
        rule_id += 1;
    }

    // 3. Blocked domain rules (priority 1, block)
    for d in &blocked_domains {
        if !is_valid_domain(d) { continue; }
        if rule_id > 1 { out_json.push_str(",\n"); }
        out_json.push_str(&format!(
            "  {{\"id\":{},\"priority\":1,\"action\":{{\"type\":\"block\"}},\"condition\":{{\"urlFilter\":\"||{}^\",\"resourceTypes\":{}}}}}",
            rule_id, escape_json(d), resource_types
        ));
        rule_id += 1;
    }

    out_json.push_str("\n]\n");
    fs::write("rustad/rules.json", out_json).expect("Failed to write rustad/rules.json");

    // Copy compiled wasm if present
    let compiled_wasm = Path::new("target/wasm32-unknown-unknown/release/rustad.wasm");
    if compiled_wasm.exists() {
        let _ = fs::copy(compiled_wasm, "rustad/rustad.wasm");
    }
}
