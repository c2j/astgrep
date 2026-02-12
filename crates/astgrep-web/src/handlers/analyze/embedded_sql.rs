use astgrep_core::Language;

// --- Embedded SQL extraction helpers (lightweight, no external deps) ---
#[derive(Clone, Debug)]
pub(crate) struct EmbeddedSqlSnippet {
    pub(crate) sql: String,
    pub(crate) start_line: usize,
    pub(crate) context: Option<String>,
}

pub(crate) fn extract_embedded_sql_snippets(
    source_code: &str,
    language: Language,
) -> Vec<EmbeddedSqlSnippet> {
    match language {
        Language::Java => extract_embedded_sql_from_java(source_code),
        Language::Xml => extract_embedded_sql_from_xml(source_code),
        _ => Vec::new(),
    }
}

fn extract_embedded_sql_from_java(src: &str) -> Vec<EmbeddedSqlSnippet> {
    let mut out = Vec::new();
    // Patterns to look for: annotations and common JDBC/native queries
    for &marker in &["Select(", "Query("] {
        let mut idx = 0usize;
        while let Some(pos) = src[idx..].find(marker) {
            let abs = idx + pos;
            // Best-effort: ensure it looks like annotation or FQN annotation
            // e.g., @org.xxx.Select("...") or @Select("...") or .Select("...")
            let start_line = 1 + byte_offset_to_line(src, abs);
            if let Some(qpos) = src[abs + marker.len()..].find('"') {
                let qabs = abs + marker.len() + qpos;
                if let Some((lit, end_idx)) = read_java_string_literal(src, qabs) {
                    let sql = normalize_sql(&unescape_java_string(&lit));
                    out.push(EmbeddedSqlSnippet {
                        sql,
                        start_line,
                        context: Some("@Select/@Query".to_string()),
                    });
                    idx = end_idx;
                    continue;
                }
            }
            idx = abs + marker.len();
        }
    }
    for &marker in &["prepareStatement(", "executeQuery(", "createNativeQuery("] {
        let mut idx = 0usize;
        while let Some(pos) = src[idx..].find(marker) {
            let abs = idx + pos;
            let start_line = 1 + byte_offset_to_line(src, abs);
            if let Some(qpos) = src[abs + marker.len()..].find('"') {
                let qabs = abs + marker.len() + qpos;
                if let Some((lit, end_idx)) = read_java_string_literal(src, qabs) {
                    let sql = normalize_sql(&unescape_java_string(&lit));
                    out.push(EmbeddedSqlSnippet {
                        sql,
                        start_line,
                        context: Some("JDBC".to_string()),
                    });
                    idx = end_idx;
                    continue;
                }
            }
            idx = abs + marker.len();
        }
    }
    out
}

fn read_java_string_literal(src: &str, first_quote_idx: usize) -> Option<(String, usize)> {
    // first_quote_idx points to the opening '"'
    let bytes = src.as_bytes();
    if *bytes.get(first_quote_idx)? != b'"' {
        return None;
    }
    let mut i = first_quote_idx + 1;
    let mut out = String::new();
    while i < bytes.len() {
        let b = bytes[i];
        if b == b'\\' {
            // escape: include next char as-is
            if i + 1 < bytes.len() {
                out.push(src[i + 1..=i + 1].chars().next().unwrap_or('\u{0}'));
                i += 2;
                continue;
            } else {
                break;
            }
        }
        if b == b'"' {
            // closing quote
            return Some((out, i + 1));
        }
        out.push(src[i..=i].chars().next().unwrap_or('\u{0}'));
        i += 1;
    }
    None
}

fn extract_embedded_sql_from_xml(src: &str) -> Vec<EmbeddedSqlSnippet> {
    let mut out = Vec::new();
    let mut idx = 0usize;
    while let Some(start_tag) = src[idx..].to_lowercase().find("<select") {
        let abs_start = idx + start_tag;
        if let Some(gt_rel) = src[abs_start..].find('>') {
            let gt = abs_start + gt_rel;
            if let Some(end_rel) = src[gt..].to_lowercase().find("</select>") {
                let end = gt + end_rel;
                let inner = &src[gt + 1..end];
                let start_line = 1 + byte_offset_to_line(src, abs_start);
                let sql = normalize_sql(inner);
                out.push(EmbeddedSqlSnippet {
                    sql,
                    start_line,
                    context: Some("<select>".to_string()),
                });
                idx = end + "</select>".len();
                continue;
            }
        }
        idx = abs_start + 7; // move past "<select"
    }
    out
}

fn byte_offset_to_line(source: &str, byte_idx: usize) -> usize {
    let mut count = 0usize;
    for (i, b) in source.as_bytes().iter().enumerate() {
        if i >= byte_idx {
            break;
        }
        if *b == b'\n' {
            count += 1;
        }
    }
    count
}

fn unescape_java_string(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut chars = input.chars();
    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.next() {
                Some('n') => out.push('\n'),
                Some('r') => out.push('\r'),
                Some('t') => out.push('\t'),
                Some('"') => out.push('"'),
                Some('\\') => out.push('\\'),
                Some('\'') => out.push('\''),
                Some('u') => {
                    let mut hex = String::new();
                    for _ in 0..4 {
                        if let Some(h) = chars.next() {
                            hex.push(h);
                        }
                    }
                    if let Ok(cp) = u16::from_str_radix(&hex, 16) {
                        if let Some(ch) = std::char::from_u32(cp as u32) {
                            out.push(ch);
                        }
                    }
                }
                Some(other) => {
                    out.push(other);
                }
                None => {}
            }
        } else {
            out.push(c);
        }
    }
    out
}

fn normalize_sql(raw: &str) -> String {
    // Replace MyBatis placeholders best-effort and collapse whitespace
    let tmp = replace_placeholders(raw, "${", '}', "T0");
    let replaced = replace_placeholders(&tmp, "#{", '}', "1");
    let mut out = String::with_capacity(replaced.len());
    let mut prev_ws = false;
    for ch in replaced.chars() {
        if ch.is_whitespace() {
            if !prev_ws {
                out.push(' ');
                prev_ws = true;
            }
        } else {
            out.push(ch);
            prev_ws = false;
        }
    }
    let s = out.trim().to_string();
    if s.ends_with(';') {
        s
    } else {
        format!("{};", s)
    }
}

fn replace_placeholders(input: &str, start_pat: &str, end_ch: char, replacement: &str) -> String {
    let mut out = String::new();
    let mut i = 0usize;
    let bytes = input.as_bytes();
    while i < bytes.len() {
        if i + start_pat.len() <= bytes.len() && &input[i..i + start_pat.len()] == start_pat {
            // consume until end_ch
            i += start_pat.len();
            while i < bytes.len() {
                let ch = input[i..=i].chars().next().unwrap_or('\u{0}');
                i += ch.len_utf8();
                if ch == end_ch {
                    break;
                }
            }
            out.push_str(replacement);
        } else {
            let ch = input[i..=i].chars().next().unwrap_or('\u{0}');
            i += ch.len_utf8();
            out.push(ch);
        }
    }
    out
}
