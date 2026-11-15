//! MkDocs flavour of the Markdown parser: Python-Markdown with the
//! pymdown-extensions that MkDocs Material sites use.
//!
//! pulldown-cmark reads CommonMark, so this parser hands it a shadow of the
//! source — same length, same newlines — in which MkDocs-only syntax is
//! rewritten to CommonMark with the same structure. Spans still index the
//! real source, so reconstruction splices translations exactly as the plain
//! Markdown parser does.
//!
//! * Math (`$..$`, `$$..$$`, `\(..\)`, `\[..\]`, `\begin{..}..\end{..}`) is
//!   filled with `x`, so nothing inside it reads as Markdown. Inline math
//!   stays in the segment text; a block that is only math is not offered.
//!   A source whose site has no pymdownx.arithmatex turns this off
//!   (`math = false`): a `$` is then a dollar sign.
//! * With `definition_lists = true` (Python-Markdown's `def_list`), each term
//!   line is a block of its own and a `:   definition` line becomes a list
//!   item (see `scan_definition_lists`).
//! * An admonition, details or tab header becomes a list item whose content
//!   column is the body indentation (see `scan_block_header`).
//! * YAML front matter is blanked (its closing `---` may carry trailing
//!   whitespace); only a `title:` value is offered.
//! * Jinja statement lines (`{% include ... %}`) are blanked.
//! * An ATX heading written without a space (`##Title`) gets one: its last
//!   `#` becomes a space, so the title span still starts at the title.

use std::ops::Range;
use yeokja_core::model::*;
use yeokja_core::parser::{DocumentParser, Markup, TranslationMap};
use yeokja_parser_markdown_dialect::{Extra, parse_with};
use yeokja_parser_utils::{resolve_reference_links, splice_reconstruct};

/// The MkDocs parser, set up for what a site's `markdown_extensions` change
/// about the way a page is read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MkdocsParser {
    math: bool,
    definition_lists: bool,
}

impl Default for MkdocsParser {
    /// Math on (pymdownx.arithmatex), definition lists off.
    fn default() -> Self {
        Self { math: true, definition_lists: false }
    }
}

impl MkdocsParser {
    /// `math`: the site enables pymdownx.arithmatex, so `$..$` and the other
    /// delimiters hold formulas; off, a `$` is a dollar sign.
    /// `definition_lists`: the site enables Python-Markdown's `def_list`.
    pub fn configured(math: bool, definition_lists: bool) -> Self {
        Self { math, definition_lists }
    }
}

impl DocumentParser for MkdocsParser {
    fn markup(&self) -> Markup {
        if self.math { Markup::MkDocs } else { Markup::MkDocsNoMath }
    }

    fn parse(&self, source: &str) -> Document {
        let layout = scan(source, *self);
        let shadow = String::from_utf8(layout.shadow).expect("shadow edits are ASCII over whole characters");
        let mut doc = parse_with(&shadow, source, true, layout.extras);
        drop_math_only_blocks(&mut doc, &layout.math);
        doc
    }

    fn reconstruct(&self, document: &Document, translations: &TranslationMap) -> String {
        let translations = resolve_reference_links(document, translations);
        splice_reconstruct(document, &translations)
    }
}

struct Layout {
    shadow: Vec<u8>,
    extras: Vec<Extra>,
    math: Vec<Range<usize>>,
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

fn fill(shadow: &mut [u8], range: Range<usize>, with: u8) {
    for b in &mut shadow[range] {
        if *b != b'\n' {
            *b = with;
        }
    }
}

/// A fence opener: its character and length.
fn fence_open(line: &str) -> Option<(u8, usize)> {
    let t = line.trim_start();
    for &c in b"`~" {
        let n = t.bytes().take_while(|&b| b == c).count();
        if n >= 3 && !(c == b'`' && t[n..].contains('`')) {
            return Some((c, n));
        }
    }
    None
}

fn fence_closes(line: &str, (c, n): (u8, usize)) -> bool {
    let t = line.trim();
    t.len() >= n && t.bytes().all(|b| b == c)
}

/// Line index where YAML front matter ends, when the file starts with it.
fn front_matter_end(source: &str, lines: &[Range<usize>]) -> Option<usize> {
    if lines.first().map(|l| source[l.clone()].trim_end()) != Some("---") {
        return None;
    }
    lines.iter().enumerate().skip(1).find_map(|(i, l)| {
        let t = source[l.clone()].trim_end();
        (t == "---" || t == "...").then_some(i)
    })
}

/// The value of a `title:` line, without surrounding quotes.
fn title_value(source: &str, line: Range<usize>) -> Option<Range<usize>> {
    let text = &source[line.clone()];
    let rest = text.strip_prefix("title:")?;
    let lead = rest.len() - rest.trim_start().len();
    let value = rest.trim();
    if value.is_empty() {
        return None;
    }
    let start = line.start + "title:".len() + lead;
    let (start, len) = match value.as_bytes()[0] {
        q @ (b'"' | b'\'') if value.len() >= 2 && value.as_bytes()[value.len() - 1] == q => (start + 1, value.len() - 2),
        _ => (start, value.len()),
    };
    (len > 0).then_some(start..start + len)
}

fn is_jinja_statement(line: &str) -> bool {
    let t = line.trim();
    t.starts_with("{%") && t.ends_with("%}")
}

/// Offset of the `#` to blank in `##Title`, if the line is such a heading.
fn unspaced_atx(line: &str) -> Option<usize> {
    let indent = line.len() - line.trim_start_matches(' ').len();
    if indent > 3 {
        return None;
    }
    let hashes = line[indent..].bytes().take_while(|&b| b == b'#').count();
    let next = line[indent + hashes..].chars().next()?;
    ((1..=6).contains(&hashes) && !next.is_whitespace() && next != '#').then(|| indent + hashes - 1)
}

/// What the line scan made of a line, for the definition-list pass.
#[derive(Clone, Copy, PartialEq)]
enum LineKind {
    Blank,
    /// A fence line or a line inside a fence.
    Code,
    /// A line the scan read as structure: front matter, a block header, a
    /// Jinja statement, an ATX heading.
    Structure,
    Text,
}

fn scan(source: &str, options: MkdocsParser) -> Layout {
    let mut layout = Layout { shadow: source.as_bytes().to_vec(), extras: Vec::new(), math: Vec::new() };
    let lines = lines(source);
    let mut kinds = vec![LineKind::Structure; lines.len()];
    let mut first = 0;
    if let Some(end) = front_matter_end(source, &lines) {
        for line in &lines[..=end] {
            if let Some(title) = title_value(source, line.clone()) {
                layout.extras.push(Extra { range: title, block_type: BlockType::Heading, literal: false });
            }
            fill(&mut layout.shadow, line.clone(), b' ');
        }
        first = end + 1;
    }
    let body_start = lines.get(first).map_or(source.len(), |l| l.start);
    if options.math {
        layout.math = math_spans_from(source, body_start);
    }
    let mut fence = None;
    for (line, kind) in lines.iter().zip(&mut kinds).skip(first) {
        let text = &source[line.clone()];
        if let Some(open) = fence {
            if fence_closes(text, open) {
                fence = None;
            }
            *kind = LineKind::Code;
            continue;
        }
        if let Some(open) = fence_open(text) {
            fence = Some(open);
            *kind = LineKind::Code;
            continue;
        }
        if scan_block_header(source, line.clone(), &mut layout) {
            continue;
        }
        if is_jinja_statement(text) {
            fill(&mut layout.shadow, line.clone(), b' ');
            continue;
        }
        if let Some(hash) = unspaced_atx(text) {
            layout.shadow[line.start + hash] = b' ';
        }
        scan_heading_attr_list(source, line.clone(), &mut layout);
        *kind = match text.trim() {
            "" => LineKind::Blank,
            _ if is_atx_heading(text) => LineKind::Structure,
            _ => LineKind::Text,
        };
    }
    for m in &layout.math {
        fill(&mut layout.shadow, m.clone(), b'x');
    }
    if options.definition_lists {
        scan_definition_lists(source, &lines, &kinds, &mut layout);
    }
    layout.extras.sort_by_key(|e| e.range.start);
    layout
}

fn indent_of(line: &str) -> usize {
    line.len() - line.trim_start_matches([' ', '\t']).len()
}

/// The offset of the text after a definition marker — a colon and at least
/// one space (Python-Markdown's `def_list`: `[ ]{0,3}:[ ]{1,3}(.*?)`, read at
/// any indentation because a container's body is dedented first).
fn definition_text(line: &str) -> Option<usize> {
    let indent = indent_of(line);
    let rest = line[indent..].strip_prefix(':')?;
    let spaces = rest.len() - rest.trim_start_matches(' ').len();
    (spaces > 0).then_some(indent + 1 + spaces)
}

/// A line that opens a block of its own, which Python-Markdown reads before
/// it looks for a definition: a list item, quote, table row, HTML, a setext
/// underline or thematic break, a link or footnote definition.
fn opens_other_block(line: &str) -> bool {
    let t = line.trim();
    let b = t.as_bytes();
    let bullet = matches!(b, [b'-' | b'*' | b'+'] | [b'-' | b'*' | b'+', b' ' | b'\t', ..]);
    let digits = b.iter().take_while(|c| c.is_ascii_digit()).count();
    let ordered =
        digits > 0 && matches!(b.get(digits), Some(b'.' | b')')) && matches!(b.get(digits + 1), None | Some(b' ' | b'\t'));
    let rule = t.chars().all(|c| matches!(c, '-' | '=' | '*' | '_' | ' '));
    bullet || ordered || rule || t.starts_with(['>', '|', '<']) || (t.starts_with('[') && t.contains("]:"))
}

/// Python-Markdown `def_list`: term lines, then `:   definition` lines.
///
/// ```text
/// Term
/// :   Definition, which may continue
///     on lines indented four columns.
/// ```
///
/// In the shadow the marker's colon becomes a bullet, so the definition is a
/// list item: its text is a block of its own and the lines indented under it
/// are the item's content, as Python-Markdown reads them after dedenting.
/// Each term line is blanked and offered as a paragraph of its own — every
/// line above a definition is one `<dt>`, and a line after a definition that
/// is not indented starts the next term rather than continuing the item.
/// The marker and the line structure are outside every span, so they come
/// back verbatim.
///
/// A colon line is a definition when term lines sit right above it, or when
/// it follows a blank line or another definition. Under a line that opens
/// another block (a list item, a quote, a table row, a heading) it is not:
/// Python-Markdown gives those lines to their own processors first.
fn scan_definition_lists(source: &str, lines: &[Range<usize>], kinds: &[LineKind], layout: &mut Layout) {
    #[derive(PartialEq)]
    enum Above {
        Blank,
        Terms,
        Definition,
        Other,
    }
    let in_math = |layout: &Layout, range: &Range<usize>| layout.math.iter().any(|m| m.start < range.end && range.start < m.end);
    let mut above = Above::Blank;
    // Text lines right above the current one that may be terms.
    let mut terms: Vec<usize> = Vec::new();
    // Indentation of the definition whose body the lines may still belong to.
    let mut body_of: Option<usize> = None;
    for (i, line) in lines.iter().enumerate() {
        let text = &source[line.clone()];
        let indent = indent_of(text);
        match kinds[i] {
            LineKind::Blank => {
                terms.clear();
                if above != Above::Definition {
                    above = Above::Blank;
                }
                continue;
            }
            // A fenced block inside a definition body leaves the body open.
            LineKind::Code => {
                terms.clear();
                if !body_of.is_some_and(|d| indent >= d + 4) {
                    body_of = None;
                    above = Above::Other;
                }
                continue;
            }
            LineKind::Structure => {
                terms.clear();
                body_of = None;
                above = Above::Other;
                continue;
            }
            LineKind::Text => {}
        }
        if let Some(text_at) = definition_text(text)
            && !in_math(layout, &(line.start + indent..line.start + text_at))
        {
            // Terms share the definition's block level: within three columns.
            let from = terms
                .iter()
                .rposition(|&t| indent_of(&source[lines[t].clone()]).abs_diff(indent) > 3)
                .map_or(0, |p| p + 1);
            if from < terms.len() || matches!(above, Above::Blank | Above::Definition) {
                for &t in &terms[from..] {
                    let term = lines[t].clone();
                    let raw = &source[term.clone()];
                    let start = term.start + indent_of(raw);
                    fill(&mut layout.shadow, term, b' ');
                    layout.extras.push(Extra {
                        range: start..start + raw.trim().len(),
                        block_type: BlockType::Paragraph,
                        literal: false,
                    });
                }
                // `-` and up to four spaces open a list item; more would make
                // its text indented code, so a wider gap is bridged by nested
                // bullets two columns apart.
                let (colon, text_start) = (line.start + indent, line.start + text_at);
                layout.shadow[colon] = b'-';
                if text_start - colon > 5 {
                    for bullet in (colon + 2..=text_start - 2).step_by(2) {
                        layout.shadow[bullet] = b'-';
                    }
                }
                terms.clear();
                body_of = Some(indent);
                above = Above::Definition;
                continue;
            }
        }
        if body_of.is_some_and(|d| indent >= d + 4) {
            terms.clear();
            above = Above::Definition;
            continue;
        }
        body_of = None;
        if opens_other_block(text) || in_math(layout, line) {
            terms.clear();
            above = Above::Other;
        } else {
            terms.push(i);
            above = Above::Terms;
        }
    }
}

/// Whether `line` is an ATX heading, at any indentation (headings inside an
/// admonition body are indented) and with or without a space after the `#`s.
fn is_atx_heading(line: &str) -> bool {
    let rest = line.trim_start_matches([' ', '\t']);
    let hashes = rest.bytes().take_while(|&b| b == b'#').count();
    (1..=6).contains(&hashes) && rest[hashes..].chars().next().is_none_or(|c| c != '#')
}

/// The `{...}` Python-Markdown's attr_list takes from the end of a heading
/// line (`HEADER_RE`: `[ ]+\{\:?[ ]*([^\}\n ][^\n]*)[ ]*\}[ ]*$`, leftmost
/// match), as a range within `line`.
fn heading_attr_list(line: &str) -> Option<Range<usize>> {
    let t = line.trim_end_matches(' ');
    if !t.ends_with('}') {
        return None;
    }
    let b = t.as_bytes();
    t.match_indices(" {").find_map(|(i, _)| {
        let open = i + 1;
        let mut k = open + 1;
        if b.get(k) == Some(&b':') {
            k += 1;
        }
        while b.get(k) == Some(&b' ') {
            k += 1;
        }
        (k < t.len() - 1).then_some(open..t.len())
    })
}

/// Keep a heading's attr_list out of its title and offer the value of a
/// `data-toc-label` in it, which the table of contents shows instead of the
/// title.
fn scan_heading_attr_list(source: &str, line: Range<usize>, layout: &mut Layout) {
    let text = &source[line.clone()];
    if !is_atx_heading(text) {
        return;
    }
    let Some(attrs) = heading_attr_list(text) else { return };
    let start = line.start + attrs.start;
    if layout.math.iter().any(|m| m.contains(&start)) {
        return;
    }
    fill(&mut layout.shadow, start..line.start + attrs.end, b' ');
    let inner = &text[attrs.clone()];
    if let Some(at) = inner.find("data-toc-label=") {
        let value = at + "data-toc-label=".len();
        if let Some(&q @ (b'"' | b'\'')) = inner.as_bytes().get(value)
            && let Some(len) = inner[value + 1..].find(q as char)
            && len > 0
        {
            let begin = start + value + 1;
            layout.extras.push(Extra { range: begin..begin + len, block_type: BlockType::Heading, literal: false });
        }
    }
}

/// `!!! type "title"`, `??? type "title"`, `???+ ...`, `=== "title"`.
///
/// The header becomes the list item `-   ***` in the shadow: a bullet whose
/// content column is the header indentation plus four and whose first content
/// is a thematic break (the mixed `-`/`*` line is not itself a break). The
/// body, indented four columns (spaces or a tab) past the header, then parses
/// as the item's content even across blank lines — the structure
/// Python-Markdown gets by dedenting it. A non-empty bullet item interrupts a
/// paragraph, so a header right after a paragraph line works too.
fn scan_block_header(source: &str, line: Range<usize>, layout: &mut Layout) -> bool {
    let text = &source[line.clone()];
    let indent = text.len() - text.trim_start_matches([' ', '\t']).len();
    let rest = &text[indent..];
    let marker = ["???+", "!!!", "???", "==="].into_iter().find(|m| rest.starts_with(m));
    let Some(marker) = marker else { return false };
    let after = &rest[marker.len()..];
    if !after.starts_with([' ', '\t']) {
        return false;
    }
    if let (Some(open), Some(close)) = (after.find('"'), after.rfind('"'))
        && close > open + 1
    {
        let base = line.start + indent + marker.len();
        layout.extras.push(Extra { range: base + open + 1..base + close, block_type: BlockType::Heading, literal: false });
    }
    let start = line.start + indent;
    fill(&mut layout.shadow, start..line.end, b' ');
    let item: &[u8] = if line.end - start >= 7 { b"-   ***" } else { b"- ***" };
    layout.shadow[start..start + item.len()].copy_from_slice(item);
    true
}

/// Math spans as pymdownx.arithmatex (generic mode) finds them, skipping code.
pub fn math_spans(text: &str) -> Vec<Range<usize>> {
    math_spans_from(text, 0)
}

fn math_spans_from(text: &str, start: usize) -> Vec<Range<usize>> {
    let b = text.as_bytes();
    let code = code_ranges(text);
    let in_code = |i: usize| code.iter().any(|r| r.contains(&i));
    let mut out = Vec::new();
    let mut i = start;
    while i < b.len() {
        if in_code(i) {
            i += 1;
            continue;
        }
        let found = match b[i] {
            b'\\' => match b.get(i + 1) {
                Some(b'(') => find(b, i + 2, b"\\)").map(|e| i..e + 2),
                Some(b'[') => find(b, i + 2, b"\\]").map(|e| i..e + 2),
                Some(b'b') if text[i..].starts_with("\\begin{") && at_line_start(b, i) => begin_end(text, i),
                Some(_) => {
                    i += 2;
                    continue;
                }
                None => None,
            },
            b'$' if b.get(i + 1) == Some(&b'$') => find(b, i + 2, b"$$").map(|e| i..e + 2),
            b'$' => inline_dollar(b, i),
            _ => None,
        };
        match found {
            Some(r) => {
                i = r.end;
                out.push(r);
            }
            None => i += 1,
        }
    }
    out
}

fn at_line_start(b: &[u8], i: usize) -> bool {
    b[..i].iter().rev().take_while(|&&c| c != b'\n').all(|c| c.is_ascii_whitespace())
}

fn find(b: &[u8], from: usize, needle: &[u8]) -> Option<usize> {
    let mut j = from;
    while j + needle.len() <= b.len() {
        if b[j] == b'\\' && needle[0] != b'\\' {
            j += 2;
            continue;
        }
        if &b[j..j + needle.len()] == needle {
            return Some(j);
        }
        j += 1;
    }
    None
}

fn begin_end(text: &str, i: usize) -> Option<Range<usize>> {
    let name_end = text[i + 7..].find('}')? + i + 7;
    let closing = format!("\\end{{{}}}", &text[i + 7..name_end]);
    let end = text[name_end..].find(&closing)? + name_end + closing.len();
    Some(i..end)
}

/// `$...$`: no space after the opener or before the closer, not `\$`, and
/// not across a blank line.
fn inline_dollar(b: &[u8], i: usize) -> Option<Range<usize>> {
    let escaped = b[..i].iter().rev().take_while(|&&c| c == b'\\').count() % 2 == 1;
    if escaped || b.get(i + 1).is_none_or(|c| c.is_ascii_whitespace()) {
        return None;
    }
    let mut j = i + 1;
    while j < b.len() {
        match b[j] {
            b'\\' => j += 2,
            b'$' if !b[j - 1].is_ascii_whitespace() => return Some(i..j + 1),
            b'\n' if b[j + 1..].iter().take_while(|&&c| c != b'\n').all(|c| c.is_ascii_whitespace()) => return None,
            _ => j += 1,
        }
    }
    None
}

/// Fenced code blocks and inline code spans, where `$` is not math.
fn code_ranges(text: &str) -> Vec<Range<usize>> {
    let mut out = Vec::new();
    let mut open: Option<((u8, usize), usize)> = None;
    for line in lines(text) {
        let t = &text[line.clone()];
        match open {
            Some((f, start)) if fence_closes(t, f) => {
                out.push(start..line.end);
                open = None;
            }
            Some(_) => {}
            None => {
                if let Some(f) = fence_open(t) {
                    open = Some((f, line.start));
                }
            }
        }
    }
    if let Some((_, start)) = open {
        out.push(start..text.len());
    }
    let b = text.as_bytes();
    let mut i = 0;
    while i < b.len() {
        if b[i] != b'`' || out.iter().any(|r| r.contains(&i)) {
            i += 1;
            continue;
        }
        let n = b[i..].iter().take_while(|&&c| c == b'`').count();
        let mut j = i + n;
        let mut close = None;
        while j < b.len() {
            if b[j] == b'`' {
                let m = b[j..].iter().take_while(|&&c| c == b'`').count();
                if m == n {
                    close = Some(j + m);
                    break;
                }
                j += m;
            } else if b[j] == b'\n' && b.get(j + 1) == Some(&b'\n') {
                break;
            } else {
                j += 1;
            }
        }
        match close {
            Some(end) => {
                out.push(i..end);
                i = end;
            }
            None => i += n,
        }
    }
    out
}

/// Replace every math span with the same placeholder, for checks that must
/// not read `_` or `*` inside formulas as emphasis.
pub fn mask_math(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut pos = 0;
    for r in math_spans(text) {
        out.push_str(&text[pos..r.start]);
        out.push_str("⟦M⟧");
        pos = r.end;
    }
    out.push_str(&text[pos..]);
    out
}

/// A block whose text is nothing but math (and punctuation) is not prose.
fn drop_math_only_blocks(doc: &mut Document, math: &[Range<usize>]) {
    let source = doc.source.clone();
    for block in doc.sections.iter_mut().flat_map(|s| s.blocks.iter_mut()) {
        let Some(span) = block.span.clone() else { continue };
        if !block.translatable {
            continue;
        }
        let inside: Vec<&Range<usize>> = math.iter().filter(|m| m.start >= span.start && m.end <= span.end).collect();
        if inside.is_empty() {
            continue;
        }
        let mut rest = String::new();
        let mut pos = span.start;
        for m in inside {
            rest.push_str(&source[pos..m.start]);
            pos = m.end;
        }
        rest.push_str(&source[pos..span.end]);
        if rest.chars().all(|c| c.is_whitespace() || ".,;:".contains(c)) {
            block.translatable = false;
            block.segments.clear();
            block.span = None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sources(source: &str) -> Vec<String> {
        sources_with(MkdocsParser::default(), source)
    }

    fn translate_all(source: &str, f: impl Fn(&str) -> String) -> String {
        translate_all_with(MkdocsParser::default(), source, f)
    }

    fn sources_with(parser: MkdocsParser, source: &str) -> Vec<String> {
        parser.parse(source).translatable_segments().iter().map(|s| s.source.clone()).collect()
    }

    fn translate_all_with(parser: MkdocsParser, source: &str, f: impl Fn(&str) -> String) -> String {
        let doc = parser.parse(source);
        let mut map = TranslationMap::new();
        for seg in doc.translatable_segments() {
            map.insert(seg.id.clone(), f(&seg.source));
        }
        parser.reconstruct(&doc, &map)
    }

    fn def_lists() -> MkdocsParser {
        MkdocsParser::configured(true, true)
    }

    fn no_math() -> MkdocsParser {
        MkdocsParser::configured(false, false)
    }

    fn ko(s: &str) -> String {
        format!("가 {s}")
    }

    #[test]
    fn math_spans_cover_every_arithmatex_form() {
        let text = "a $x_1$ b $$\\sum_i\n i$$ c \\(y\\) d \\[z\\] e \\$5 and $ 6$ f `$code$`";
        let found: Vec<&str> = math_spans(text).into_iter().map(|r| &text[r]).collect();
        assert_eq!(found, ["$x_1$", "$$\\sum_i\n i$$", "\\(y\\)", "\\[z\\]"]);
    }

    #[test]
    fn begin_end_block_is_math() {
        let text = "Text.\n\n\\begin{align}\na &= b \\\\\nc &= d\n\\end{align}\n\nMore.\n";
        let found: Vec<&str> = math_spans(text).into_iter().map(|r| &text[r]).collect();
        assert_eq!(found, ["\\begin{align}\na &= b \\\\\nc &= d\n\\end{align}"]);
        assert_eq!(sources(text), ["Text.", "More."]);
    }

    #[test]
    fn inline_math_stays_in_the_segment_with_markdown_characters_inside() {
        assert_eq!(sources("Let $a_i * b_j$ be *the* product.\n"), ["Let $a_i * b_j$ be *the* product."]);
    }

    #[test]
    fn display_math_only_paragraph_is_not_offered() {
        let source = "Intro:\n\n$$\nd[v] = \\infty \\\\\n= 0\n$$\n\nOutro.\n";
        assert_eq!(sources(source), ["Intro:", "Outro."]);
        assert_eq!(translate_all(source, ko), "가 Intro:\n\n$$\nd[v] = \\infty \\\\\n= 0\n$$\n\n가 Outro.\n");
    }

    #[test]
    fn math_line_that_looks_like_a_list_or_setext_does_not_split_the_formula() {
        let source = "Consider\n$$\n0. x\n===\n$$\nthen stop.\n";
        let segs = sources(source);
        assert_eq!(segs.len(), 1);
        assert!(segs[0].contains("$$ 0. x === $$") || segs[0].contains("$$\n0. x"));
    }

    #[test]
    fn front_matter_with_trailing_tab_is_blanked_and_title_offered() {
        let source = "---\ntags:\n  - Translated\ntitle: Ternary Search\ne_maxx_link: ternary_search\n---\t\n# Ternary search\n\nBody.\n";
        assert_eq!(sources(source), ["Ternary Search", "Ternary search", "Body."]);
        assert_eq!(
            translate_all(source, ko),
            "---\ntags:\n  - Translated\ntitle: 가 Ternary Search\ne_maxx_link: ternary_search\n---\t\n# 가 Ternary search\n\n가 Body.\n"
        );
    }

    #[test]
    fn quoted_title_keeps_its_quotes() {
        let source = "---\ntitle: \"Main Page\"\n---\n\nBody.\n";
        assert_eq!(translate_all(source, ko), "---\ntitle: \"가 Main Page\"\n---\n\n가 Body.\n");
    }

    #[test]
    fn jinja_statement_line_is_kept_verbatim() {
        let source = "---\ntitle: Main Page\n---\n\n{% include 'index_body' %}\n";
        assert_eq!(sources(source), ["Main Page"]);
        assert_eq!(translate_all(source, ko), "---\ntitle: 가 Main Page\n---\n\n{% include 'index_body' %}\n");
    }

    #[test]
    fn atx_heading_without_space_is_a_heading() {
        assert_eq!(sources("Text.\n\n##Implementation\n\nMore.\n"), ["Text.", "Implementation", "More."]);
    }

    #[test]
    fn dollar_in_code_fence_is_not_math() {
        let source = "```cpp\nint $a = 1; // $b$\n```\n\nText $x$.\n";
        let found: Vec<&str> = math_spans(source).into_iter().map(|r| &source[r]).collect();
        assert_eq!(found, ["$x$"]);
    }

    #[test]
    fn mask_math_replaces_each_span_with_one_placeholder() {
        assert_eq!(mask_math("a $x_1$ b $$y$$"), "a ⟦M⟧ b ⟦M⟧");
    }

    #[test]
    fn admonition_title_and_body_are_separate_prose() {
        let source = "Before.\n\n!!! info \"Lemma\"\n    The body is prose.\n    It spans lines.\n\nAfter.\n";
        assert_eq!(sources(source), ["Before.", "Lemma", "The body is prose.", "It spans lines.", "After."]);
        assert_eq!(
            translate_all(source, ko),
            "가 Before.\n\n!!! info \"가 Lemma\"\n    가 The body is prose. 가 It spans lines.\n\n가 After.\n"
        );
    }

    #[test]
    fn body_after_a_blank_line_is_prose_not_code() {
        let source = "??? hint \"Solution\"\n\n    Use a stack.\n\n    ```cpp\n    int x;\n\n    int y;\n    ```\n\nDone.\n";
        assert_eq!(sources(source), ["Solution", "Use a stack.", "Done."]);
    }

    #[test]
    fn tab_indented_body_and_header_after_paragraph() {
        let source = "Intro line\n!!! note\n\tBody with tab.\n";
        assert_eq!(sources(source), ["Intro line", "Body with tab."]);
    }

    #[test]
    fn tabs_keep_code_out_of_prose() {
        let source = "=== \"C++\"\n    ```cpp\n    int main() {}\n    ```\n=== \"Python\"\n    ```py\n    print(1)\n    ```\n";
        assert_eq!(sources(source), ["C++", "Python"]);
    }

    #[test]
    fn nested_admonition_and_list_inside_body() {
        let source = "!!! example \"Outer\"\n    - item one\n      continues\n\n    !!! note \"Inner\"\n        Inner body.\n";
        assert_eq!(sources(source), ["Outer", "item one continues", "Inner", "Inner body."]);
    }

    /// Python-Markdown's attr_list takes everything from the first ` {` of a
    /// heading line that ends in `}`. The toc label is visible text (the
    /// table of contents shows it), so its value is offered on its own.
    #[test]
    fn heading_attr_list_is_kept_and_its_toc_label_offered() {
        let source = "## Paths of length $k$ {data-toc-label=\"Paths of length k\"}\n\nBody.\n";
        assert_eq!(sources(source), ["Paths of length $k$", "Paths of length k", "Body."]);
        assert_eq!(
            translate_all(source, ko),
            "## 가 Paths of length $k$ {data-toc-label=\"가 Paths of length k\"}\n\n가 Body.\n"
        );
    }

    #[test]
    fn toc_label_with_raw_html_and_an_id_in_the_attr_list() {
        let source = "### Definition of $g(i)$ { #def data-toc-label='Definition of <script type=\"math/tex\">g(i)</script>' }\n";
        assert_eq!(
            sources(source),
            ["Definition of $g(i)$", "Definition of <script type=\"math/tex\">g(i)</script>"]
        );
    }

    #[test]
    fn heading_attr_list_without_label_is_not_title_text() {
        assert_eq!(sources("## Sets {a, b}\n"), ["Sets"]);
        assert_eq!(sources("## Title {: #tid }\n"), ["Title"]);
        assert_eq!(sources("## Sets{a}\n"), ["Sets{a}"]);
    }

    #[test]
    fn admonition_without_title_offers_only_body() {
        assert_eq!(sources("!!! warning\n    Careful.\n"), ["Careful."]);
    }

    /// Python-Markdown's `def_list`: a term line and the `:   ` line under
    /// it. Read as CommonMark the two are one paragraph, so the translation
    /// would come back on one line and stop being a definition list.
    #[test]
    fn definition_list_term_and_definition_are_separate_and_keep_their_lines() {
        let source = "Intro.\n\nLanded\n:   Linux, January 2026 (CVE-2026-23089)\n\nDriver\n:   `snd-usb-audio` (`sound/usb/mixer.c`)\n\nAfter.\n";
        assert_eq!(
            sources_with(def_lists(), source),
            ["Intro.", "Landed", "Linux, January 2026 (CVE-2026-23089)", "Driver", "`snd-usb-audio` (`sound/usb/mixer.c`)", "After."]
        );
        assert_eq!(
            translate_all_with(def_lists(), source, ko),
            "가 Intro.\n\n가 Landed\n:   가 Linux, January 2026 (CVE-2026-23089)\n\n가 Driver\n:   가 `snd-usb-audio` (`sound/usb/mixer.c`)\n\n가 After.\n"
        );
        assert_eq!(translate_all_with(def_lists(), source, str::to_string), source);
    }

    #[test]
    fn definition_lists_are_off_unless_the_source_enables_them() {
        assert_eq!(sources("Landed\n:   Linux, January 2026\n"), ["Landed :   Linux, January 2026"]);
    }

    #[test]
    fn a_term_takes_several_definitions_and_terms_follow_without_a_blank_line() {
        let source = "Apple\n:   A fruit.\n:   A company.\nOrange\n:   Another fruit.\n";
        assert_eq!(sources_with(def_lists(), source), ["Apple", "A fruit.", "A company.", "Orange", "Another fruit."]);
        assert_eq!(
            translate_all_with(def_lists(), source, ko),
            "가 Apple\n:   가 A fruit.\n:   가 A company.\n가 Orange\n:   가 Another fruit.\n"
        );
    }

    /// Every line above the definition is a term of its own (`<dt>`).
    #[test]
    fn several_term_lines_stay_one_per_line() {
        let source = "Term one\nTerm two\n:   Shared definition.\n";
        assert_eq!(sources_with(def_lists(), source), ["Term one", "Term two", "Shared definition."]);
        assert_eq!(translate_all_with(def_lists(), source, ko), "가 Term one\n가 Term two\n:   가 Shared definition.\n");
    }

    #[test]
    fn definition_body_continues_on_indented_lines_and_paragraphs() {
        let source = "Term\n:   First line\n    continues here.\n\n    Second paragraph.\n\n    ```c\n    int x;\n    ```\n\nAfter.\n";
        assert_eq!(sources_with(def_lists(), source), ["Term", "First line continues here.", "Second paragraph.", "After."]);
        assert_eq!(
            translate_all_with(def_lists(), source, ko),
            "가 Term\n:   가 First line continues here.\n\n    가 Second paragraph.\n\n    ```c\n    int x;\n    ```\n\n가 After.\n"
        );
    }

    #[test]
    fn definition_after_a_blank_line_and_inside_an_admonition() {
        let source = "Term\n\n:   Loose definition.\n\n!!! note \"Fields\"\n    Name\n    :   The name.\n";
        assert_eq!(sources_with(def_lists(), source), ["Term", "Loose definition.", "Fields", "Name", "The name."]);
        assert_eq!(
            translate_all_with(def_lists(), source, ko),
            "가 Term\n\n:   가 Loose definition.\n\n!!! note \"가 Fields\"\n    가 Name\n    :   가 The name.\n"
        );
    }

    /// A colon line under a list item or a quote is not a definition to
    /// Python-Markdown (those block processors run first), a colon line in a
    /// fence is code, and `:::` opens a mkdocstrings block.
    #[test]
    fn colon_lines_that_are_not_definitions_are_left_alone() {
        for source in ["- item\n:   text\n", "```\nFoo(int a)\n: v(a) {}\n```\n", "> quote\n: text\n", "::: module.path\n"] {
            assert_eq!(sources_with(def_lists(), source), sources(source), "{source:?}");
        }
    }

    #[test]
    fn term_markup_and_colon_spacing_survive() {
        let source = "**Bold** `term`\n: One space.\n";
        assert_eq!(sources_with(def_lists(), source), ["**Bold** `term`", "One space."]);
        assert_eq!(translate_all_with(def_lists(), source, ko), "가 **Bold** `term`\n: 가 One space.\n");
    }

    /// More than four spaces after the bullet would make the text indented
    /// code, so the shadow bridges the gap with nested bullets.
    #[test]
    fn a_wide_gap_after_the_colon_is_still_a_definition() {
        let source = "Term\n:      Far text.\n";
        assert_eq!(sources_with(def_lists(), source), ["Term", "Far text."]);
        assert_eq!(translate_all_with(def_lists(), source, ko), "가 Term\n:      가 Far text.\n");
    }

    /// Without pymdownx.arithmatex a `$` is a dollar sign. Read as math, the
    /// span from one item's `$` to the next item's hides the second bullet.
    #[test]
    fn without_math_a_dollar_is_text() {
        let source = "- [Turning into 10000$](https://a.example/x) — the writeup\n- [LWN: Turning into 10000$](https://b.example/y) — the brief\n";
        assert_eq!(sources(source).len(), 1, "math on: the two items read as one");
        assert_eq!(
            sources_with(no_math(), source),
            ["[Turning into 10000$](https://a.example/x) — the writeup", "[LWN: Turning into 10000$](https://b.example/y) — the brief"]
        );
        assert_eq!(translate_all_with(no_math(), source, str::to_string), source);
    }

    #[test]
    fn without_math_dollars_are_prose_and_the_markup_says_so() {
        assert_eq!(sources_with(no_math(), "Costs $5 and *$6*.\n\n$$\nx\n$$\n"), ["Costs $5 and *$6*.", "$$ x $$"]);
        assert_eq!(no_math().markup(), Markup::MkDocsNoMath);
        assert_eq!(MkdocsParser::default().markup(), Markup::MkDocs);
    }
}
