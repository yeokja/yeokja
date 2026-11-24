//! Front matter at the top of a Markdown-family file: YAML (`---`) and TOML
//! (`+++`). A parser blanks the block in its shadow and offers the values of
//! chosen keys as literal strings; reconstruction writes a translated value
//! back the way its quotes need, so the front matter stays valid.

use std::ops::Range;
use yeokja_core::model::BlockType;

/// Configured dotted key paths (`summary.sentence`) as their segments. An
/// empty path or path segment is skipped; the config check rejects those
/// before they get here.
pub fn key_paths<K: AsRef<str>>(keys: &[K]) -> Vec<Vec<String>> {
    keys.iter()
        .map(|k| k.as_ref().split('.').map(str::to_string).collect::<Vec<_>>())
        .filter(|path| !path.is_empty() && path.iter().all(|k| !k.is_empty()))
        .collect()
}

/// Byte ranges of each line, without its newline.
fn lines(source: &str) -> Vec<Range<usize>> {
    let mut out = Vec::new();
    let mut start = 0;
    for (i, b) in source.bytes().enumerate() {
        if b == b'\n' {
            out.push(start..i);
            start = i + 1;
        }
    }
    if start < source.len() {
        out.push(start..source.len());
    }
    out
}

// ---- YAML ------------------------------------------------------------------

/// The YAML front matter, closing line included, when the file starts with it.
pub fn yaml_front_matter(source: &str) -> Option<Range<usize>> {
    let lines = lines(source);
    if lines.first().map(|l| source[l.clone()].trim_end()) != Some("---") {
        return None;
    }
    lines.iter().skip(1).find(|l| matches!(source[(*l).clone()].trim_end(), "---" | "...")).map(|l| 0..l.end)
}

/// A `key:` mapping entry of the front matter: the line's indentation, its
/// key (which may be quoted: `"og:title":`) and the offset of the colon.
fn yaml_key(line: &str) -> Option<(usize, &str, usize)> {
    let indent = line.len() - line.trim_start_matches(' ').len();
    let rest = &line[indent..];
    if rest.starts_with(['#', '-', ' ', '\t']) || rest.is_empty() {
        return None;
    }
    let (key, colon) = match rest.as_bytes()[0] {
        q @ (b'"' | b'\'') => {
            let close = rest[1..].find(q as char)? + 1;
            (&rest[1..close], close + 1)
        }
        _ => {
            let end = rest.find(':')?;
            (rest[..end].trim_end(), end)
        }
    };
    let after = rest[colon..].strip_prefix(':')?;
    (after.is_empty() || after.starts_with([' ', '\t'])).then_some((indent, key, indent + colon))
}

/// The value of the mapping entry on line `k` of `all`, as a range without
/// its quotes, or nothing when the entry has no value to translate: a block
/// scalar (`|`, `>`), an alias (`*name`), or an empty value. An `&anchor`
/// before the value stays outside the range. A quoted value may run over
/// several lines; a plain one takes the following more-indented lines along.
fn yaml_scalar(source: &str, front: &Range<usize>, all: &[Range<usize>], k: usize) -> Option<Range<usize>> {
    let b = source.as_bytes();
    let line = &all[k];
    let text = &source[line.clone()];
    let (indent, _, colon) = yaml_key(text)?;
    let after_colon = &text[colon + 1..];
    let mut start = line.start + colon + 1 + (after_colon.len() - after_colon.trim_start().len());
    if b.get(start) == Some(&b'&') {
        let name_end = source[start..line.end].find([' ', '\t']).map_or(line.end, |n| start + n);
        let rest = &source[name_end..line.end];
        start = name_end + (rest.len() - rest.trim_start().len());
    }
    if start >= line.end {
        return None;
    }
    let value = match b[start] {
        q @ (b'"' | b'\'') => {
            let mut j = start + 1;
            let mut close = None;
            while j < front.end {
                if q == b'"' && b[j] == b'\\' {
                    j += 2;
                    continue;
                }
                if b[j] == q {
                    if q == b'\'' && b.get(j + 1) == Some(&b'\'') {
                        j += 2;
                        continue;
                    }
                    close = Some(j);
                    break;
                }
                j += 1;
            }
            start + 1..close?
        }
        b'|' | b'>' | b'*' => return None,
        _ => {
            let mut end = line.start + text.trim_end().len();
            for next in &all[k + 1..] {
                let t = &source[next.clone()];
                if t.trim().is_empty() || t.len() - t.trim_start().len() <= indent {
                    break;
                }
                end = next.start + t.trim_end().len();
            }
            start..end
        }
    };
    (!source[value.clone()].trim().is_empty()).then_some(value)
}

/// The line of `all` that a dotted key path names: each segment is a `key:`
/// line nested one level deeper than the previous one, found before the
/// previous mapping ends (a line indented no deeper than its key).
fn yaml_path_line(source: &str, all: &[Range<usize>], path: &[String]) -> Option<usize> {
    let mut from = 0;
    let mut parent: Option<usize> = None;
    let mut found = None;
    for segment in path {
        let mut level = None;
        found = None;
        for (k, line) in all.iter().enumerate().skip(from) {
            let text = &source[line.clone()];
            if text.trim().is_empty() || text.trim_start().starts_with('#') {
                continue;
            }
            let indent = text.len() - text.trim_start_matches(' ').len();
            if parent.is_some_and(|p| indent <= p) {
                break;
            }
            if level.is_none() {
                level = Some(indent);
            }
            if Some(indent) != level {
                continue;
            }
            if yaml_key(text).is_some_and(|(_, key, _)| key == segment) {
                found = Some((k, indent));
                break;
            }
        }
        let (k, indent) = found?;
        from = k + 1;
        parent = Some(indent);
    }
    found.map(|(k, _)| k)
}

/// The value ranges of the visible-text keys of the front matter, without
/// their quotes: the parser's built-in top-level keys (`builtin`, each with
/// its block type), then the configured key paths as paragraphs. A value is
/// offered once, by the first rule that finds it.
pub fn yaml_values(
    source: &str,
    front: Range<usize>,
    builtin: &[(&str, BlockType)],
    keys: &[Vec<String>],
) -> Vec<(Range<usize>, BlockType)> {
    let all = lines(&source[..front.end]);
    let mut out: Vec<(Range<usize>, BlockType)> = Vec::new();
    for (k, line) in all.iter().enumerate() {
        let Some((0, key, _)) = yaml_key(&source[line.clone()]) else { continue };
        let Some(&(_, block_type)) = builtin.iter().find(|(name, _)| *name == key) else { continue };
        if let Some(value) = yaml_scalar(source, &front, &all, k) {
            out.push((value, block_type));
        }
    }
    for path in keys {
        let Some(k) = yaml_path_line(source, &all, path) else { continue };
        if let Some(value) = yaml_scalar(source, &front, &all, k)
            && !out.iter().any(|(r, _)| *r == value)
        {
            out.push((value, BlockType::Paragraph));
        }
    }
    out.sort_by_key(|(r, _)| r.start);
    out
}

/// Put `\` before each `quote` the translation leaves unescaped.
pub fn escape_quote(text: &str, quote: char) -> String {
    let mut out = String::with_capacity(text.len());
    let mut backslashes = 0;
    for c in text.chars() {
        if c == quote && backslashes % 2 == 0 {
            out.push('\\');
        }
        backslashes = if c == '\\' { backslashes + 1 } else { 0 };
        out.push(c);
    }
    out
}

/// A plain YAML scalar the translation would end or change: quote it.
fn yaml_plain(text: String) -> String {
    let risky = text.contains(": ")
        || text.contains(" #")
        || text.ends_with(':')
        || text.starts_with(['-', '?', ':', ',', '[', ']', '{', '}', '#', '&', '*', '!', '|', '>', '\'', '"', '%', '@', '`']);
    if risky { format!("\"{}\"", text.replace('\\', "\\\\").replace('"', "\\\"")) } else { text }
}

/// A YAML front matter value's translation, written the way its quotes need:
/// a double-quoted value escapes its quote with `\`, a single-quoted one
/// doubles it, and a plain value is quoted when it has to be.
pub fn quote_yaml(source: &str, span: Range<usize>, text: String) -> String {
    let b = source.as_bytes();
    let before = span.start.checked_sub(1).map(|i| b[i]);
    match (before, b.get(span.end).copied()) {
        (Some(b'\''), Some(b'\'')) => text.replace("''", "'").replace('\'', "''"),
        (Some(b'"'), Some(b'"')) => escape_quote(&text, '"'),
        _ => yaml_plain(text),
    }
}

// ---- TOML ------------------------------------------------------------------

/// The TOML front matter (Hugo, Zola), closing line included, when the file
/// starts with a `+++` line and a later `+++` line closes it.
pub fn toml_front_matter(source: &str) -> Option<Range<usize>> {
    let lines = lines(source);
    if lines.first().map(|l| source[l.clone()].trim_end()) != Some("+++") {
        return None;
    }
    lines.iter().skip(1).find(|l| source[(*l).clone()].trim_end() == "+++").map(|l| 0..l.end)
}

/// A dotted TOML key starting at `at` in `line` (`a`, `a.b`, `"x y".c`,
/// `a . b`), ended by `end`: its segments and the offset after `end`. Bare
/// segments are `A-Za-z0-9_-`; quoted ones are taken as written.
fn toml_dotted(line: &str, mut at: usize, end: u8) -> Option<(Vec<String>, usize)> {
    let b = line.as_bytes();
    let skip_ws = |mut i: usize| {
        while b.get(i).is_some_and(|c| *c == b' ' || *c == b'\t') {
            i += 1;
        }
        i
    };
    let mut segments = Vec::new();
    loop {
        at = skip_ws(at);
        let segment_end = match *b.get(at)? {
            b'"' => {
                let mut j = at + 1;
                while j < b.len() && b[j] != b'"' {
                    j += if b[j] == b'\\' { 2 } else { 1 };
                }
                (j < b.len()).then_some(())?;
                segments.push(line[at + 1..j].to_string());
                j + 1
            }
            b'\'' => {
                let close = at + 1 + line[at + 1..].find('\'')?;
                segments.push(line[at + 1..close].to_string());
                close + 1
            }
            _ => {
                let len = b[at..].iter().take_while(|c| c.is_ascii_alphanumeric() || **c == b'_' || **c == b'-').count();
                (len > 0).then_some(())?;
                segments.push(line[at..at + len].to_string());
                at + len
            }
        };
        at = skip_ws(segment_end);
        match *b.get(at)? {
            b'.' => at += 1,
            c if c == end => return Some((segments, at + 1)),
            _ => return None,
        }
    }
}

/// The inside of a one-line basic (`"…"`) or literal (`'…'`) string that
/// starts at `at` in `line`, or nothing for another kind of value.
fn toml_string(line: &str, at: usize) -> Option<Range<usize>> {
    let rest = &line[at..];
    if rest.starts_with("\"\"\"") || rest.starts_with("'''") {
        return None;
    }
    let b = line.as_bytes();
    match *b.get(at)? {
        b'"' => {
            let mut j = at + 1;
            while j < b.len() && b[j] != b'"' {
                j += if b[j] == b'\\' { 2 } else { 1 };
            }
            (j < b.len()).then_some(at + 1..j)
        }
        b'\'' => Some(at + 1..at + 1 + line[at + 1..].find('\'')?),
        _ => None,
    }
}

/// The value ranges, without their quotes, of the one-line string values
/// that the key paths name in the TOML front matter `front`. A path is the
/// current `[table]` header's keys followed by the line's dotted key; keys
/// after an `[[array]]` header are not addressable. Multi-line strings and
/// other values (numbers, dates, arrays, tables) are not offered, nor is a
/// value of only whitespace.
pub fn toml_values(source: &str, front: Range<usize>, keys: &[Vec<String>]) -> Vec<Range<usize>> {
    let all = lines(&source[..front.end]);
    let mut table: Option<Vec<String>> = Some(Vec::new());
    let mut out = Vec::new();
    for line in all.iter().skip(1).take(all.len().saturating_sub(2)) {
        let text = &source[line.clone()];
        let indent = text.len() - text.trim_start().len();
        let trimmed = text.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if trimmed.starts_with("[[") {
            table = None;
            continue;
        }
        if trimmed.starts_with('[') {
            table = toml_dotted(text, indent + 1, b']').map(|(segments, _)| segments);
            continue;
        }
        let Some(table) = &table else { continue };
        let Some((key, after)) = toml_dotted(text, indent, b'=') else { continue };
        let path: Vec<String> = table.iter().cloned().chain(key).collect();
        if !keys.contains(&path) {
            continue;
        }
        let value_at = after + (text[after..].len() - text[after..].trim_start().len());
        if let Some(value) = toml_string(text, value_at)
            && !text[value.clone()].trim().is_empty()
        {
            out.push(line.start + value.start..line.start + value.end);
        }
    }
    out
}

/// Whether the backslash at the start of `rest` begins a TOML escape.
fn is_toml_escape(rest: &str) -> bool {
    let mut chars = rest.chars().skip(1);
    match chars.next() {
        Some('b' | 't' | 'n' | 'f' | 'r' | '"' | '\\') => true,
        Some(u @ ('u' | 'U')) => {
            let digits = if u == 'u' { 4 } else { 8 };
            chars.take(digits).filter(char::is_ascii_hexdigit).count() == digits
        }
        _ => false,
    }
}

/// `text` as the inside of a TOML basic string. With `keep_escapes`, a
/// backslash that begins a valid escape stays one (the source's `\"` and
/// `\n` carried through the translation); every other backslash and quote
/// is escaped, and so is a control character.
fn toml_basic(text: &str, keep_escapes: bool) -> String {
    let mut out = String::with_capacity(text.len() + 2);
    let mut rest = text;
    while let Some(c) = rest.chars().next() {
        let mut len = c.len_utf8();
        match c {
            '\\' if keep_escapes && is_toml_escape(rest) => {
                len += rest[1..].chars().next().map_or(0, char::len_utf8);
                out.push_str(&rest[..len]);
            }
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            '\r' => out.push_str("\\r"),
            c if c.is_control() => out.push_str(&format!("\\u{:04X}", c as u32)),
            c => out.push(c),
        }
        rest = &rest[len..];
    }
    out
}

/// A TOML front matter value's translation, as the range to replace and the
/// text to write there, so the value stays a valid string. A basic string
/// keeps the escapes it can and escapes the rest. A literal string is
/// written as it is unless the translation holds a `'` or a control
/// character, which a literal string cannot: then the quotes are replaced
/// too and the value becomes a basic string.
pub fn quote_toml(source: &str, span: Range<usize>, text: &str) -> (Range<usize>, String) {
    match span.start.checked_sub(1).map(|i| source.as_bytes()[i]) {
        Some(b'\'') if text.contains('\'') || text.chars().any(char::is_control) => {
            (span.start - 1..span.end + 1, format!("\"{}\"", toml_basic(text, false)))
        }
        Some(b'\'') => (span, text.to_string()),
        _ => (span, toml_basic(text, true)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TOML: &str = "+++\ntitle = \"Memory Safety\"\ndate = 2026-11-09\ndescription = 'A literal C:\\path value'\nslug = \"x\"\ntags = [\"a\", \"b\"]\nmulti = \"\"\"long\"\"\"\nempty = \"  \"\n\"quoted key\" = \"Q\"\nsite . name = \"Dotted\"\n# title = \"comment\"\n[params]\nsummary = \"Nested \\\"quoted\\\" text\" # comment\n[[authors]]\ntitle = \"array table\"\n[extra.\"deep\"]\nnote = \"Deep\"\n+++\n\nBody.\n";

    fn values(source: &str, keys: &[&str]) -> Vec<String> {
        let front = toml_front_matter(source).unwrap();
        toml_values(source, front, &key_paths(keys)).into_iter().map(|r| source[r].to_string()).collect()
    }

    #[test]
    fn toml_front_matter_runs_to_the_closing_line() {
        let front = toml_front_matter(TOML).unwrap();
        assert!(TOML[..front.end].ends_with("note = \"Deep\"\n+++"));
        assert_eq!(toml_front_matter("+++ \na = 1\n+++\t\nBody"), Some(0..15));
        assert_eq!(toml_front_matter("+++\na = 1\n"), None, "unclosed");
        assert_eq!(toml_front_matter("Body\n+++\na = 1\n+++\n"), None, "not at the top");
        assert_eq!(toml_front_matter("---\na: 1\n---\n"), None);
    }

    #[test]
    fn toml_values_follow_tables_and_dotted_keys() {
        let keys = [
            "title", "description", "params.summary", "quoted key", "site.name", "extra.deep.note", "tags", "multi",
            "empty", "authors.title", "date", "missing",
        ];
        assert_eq!(
            values(TOML, &keys),
            ["Memory Safety", "A literal C:\\path value", "Q", "Dotted", "Nested \\\"quoted\\\" text", "Deep"]
        );
        assert_eq!(values(TOML, &["slug"]), ["x"]);
        assert!(values(TOML, &[]).is_empty());
    }

    fn requote(source: &str, value: &str, text: &str) -> String {
        let start = source.find(value).unwrap();
        let (range, written) = quote_toml(source, start..start + value.len(), text);
        format!("{}{}{}", &source[..range.start], written, &source[range.end..])
    }

    #[test]
    fn basic_strings_keep_valid_escapes_and_escape_the_rest() {
        let source = "t = \"x\"\n";
        assert_eq!(requote(source, "x", "그는 \"안녕\" 했다"), "t = \"그는 \\\"안녕\\\" 했다\"\n");
        assert_eq!(requote(source, "x", "이미 \\\"이스케이프\\\" 됨"), "t = \"이미 \\\"이스케이프\\\" 됨\"\n");
        assert_eq!(requote(source, "x", "C:\\path 끝\\"), "t = \"C:\\\\path 끝\\\\\"\n");
        assert_eq!(requote(source, "x", "\\u00e9 \\n 탭\t"), "t = \"\\u00e9 \\n 탭\\t\"\n");
    }

    #[test]
    fn literal_strings_become_basic_strings_only_when_they_must() {
        let source = "t = 'x'\n";
        assert_eq!(requote(source, "x", "C:\\경로"), "t = 'C:\\경로'\n");
        assert_eq!(requote(source, "x", "it's C:\\경로 \"q\""), "t = \"it's C:\\\\경로 \\\"q\\\"\"\n");
    }

    #[test]
    fn yaml_values_take_builtin_keys_and_paths() {
        let source = "---\ntitle: \"T\"\nsummary:\n  sentence: S\n---\nBody\n";
        let front = yaml_front_matter(source).unwrap();
        let found: Vec<_> = yaml_values(source, front.clone(), &[], &key_paths(&["summary.sentence"]))
            .into_iter()
            .map(|(r, _)| &source[r])
            .collect();
        assert_eq!(found, ["S"]);
        let found: Vec<_> = yaml_values(source, front, &[("title", BlockType::Heading)], &[])
            .into_iter()
            .map(|(r, t)| (&source[r], t))
            .collect();
        assert_eq!(found, [("T", BlockType::Heading)]);
    }
}
