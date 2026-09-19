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

pub struct MkdocsParser;

impl DocumentParser for MkdocsParser {
    fn markup(&self) -> Markup {
        Markup::MkDocs
    }

    fn parse(&self, source: &str) -> Document {
        let layout = scan(source);
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
    for c in [b'`', b'~'] {
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

fn scan(source: &str) -> Layout {
    let mut layout = Layout { shadow: source.as_bytes().to_vec(), extras: Vec::new(), math: Vec::new() };
    let lines = lines(source);
    let mut first = 0;
    if let Some(end) = front_matter_end(source, &lines) {
        for line in &lines[..=end] {
            if let Some(title) = title_value(source, line.clone()) {
                layout.extras.push(Extra { range: title, block_type: BlockType::Heading });
            }
            fill(&mut layout.shadow, line.clone(), b' ');
        }
        first = end + 1;
    }
    let body_start = lines.get(first).map_or(source.len(), |l| l.start);
    layout.math = math_spans_from(source, body_start);
    let mut fence = None;
    for line in &lines[first..] {
        let text = &source[line.clone()];
        if let Some(open) = fence {
            if fence_closes(text, open) {
                fence = None;
            }
            continue;
        }
        if let Some(open) = fence_open(text) {
            fence = Some(open);
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
    }
    for m in &layout.math {
        fill(&mut layout.shadow, m.clone(), b'x');
    }
    layout.extras.sort_by_key(|e| e.range.start);
    layout
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
            layout.extras.push(Extra { range: begin..begin + len, block_type: BlockType::Heading });
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
        layout.extras.push(Extra { range: base + open + 1..base + close, block_type: BlockType::Heading });
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
        MkdocsParser.parse(source).translatable_segments().iter().map(|s| s.source.clone()).collect()
    }

    fn translate_all(source: &str, f: impl Fn(&str) -> String) -> String {
        let doc = MkdocsParser.parse(source);
        let mut map = TranslationMap::new();
        for seg in doc.translatable_segments() {
            map.insert(seg.id.clone(), f(&seg.source));
        }
        MkdocsParser.reconstruct(&doc, &map)
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
}
