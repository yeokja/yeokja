//! Bikeshed flavour of the Markdown parser (`Markup Shorthands: markdown yes`):
//! Markdown paragraphs and lists mixed with HTML blocks, one sentence per line.
//!
//! The source is first *rewritten*: every definition autolink `[=key=]`,
//! `[=key|text=]` and section reference with text `[[#id|text]]` becomes a
//! Markdown link whose destination carries the key under the reserved
//! `bikeshed:` scheme. Segments, the inline-tag codec and the link evaluator
//! then treat them as ordinary links — the visible text is translated, the
//! key is not — and `reconstruct` writes the shorthands back after splicing.
//! `Document.source` is the rewritten text.
//!
//! The rewritten text is then read through `parse_with` with a shadow in which
//! * the `<pre class=metadata>` block is blanked (its `Abstract:` value is
//!   offered as a literal paragraph; the rest stays verbatim), and
//! * an indented code block inside an open HTML element is blanked and read
//!   as prose (`prose_extras`): CommonMark ends an HTML block at a blank
//!   line, so the four-space list in a `<div class=note>` would be code, and
//! * a run of definition-list shorthand lines (`: term`, `:: definition`,
//!   each with its indented continuation lines) is blanked and every item
//!   offered as its own paragraph, the prefix outside the span: to CommonMark
//!   the run is one paragraph, and a translation reflowing it loses the `<dl>`.
//!
//! The prose of HTML blocks keeps phrasing-level tags (`<sub>`, `<dfn>`, …)
//! inside its chunks ([`ParseOptions::inline_html`]), so an algorithm step
//! `C1<sub>16</sub>` is one list item and the tag travels as inline HTML.
//!
//! On reconstruct a `<dfn>` whose text changed gets `lt="original text"`,
//! so autolinks written against the original term keep resolving.

use pulldown_cmark::{CodeBlockKind, Event, Parser, Tag};
use std::ops::Range;
use yeokja_core::model::*;
use yeokja_core::parser::{DocumentParser, Markup, TranslationMap};
use yeokja_parser_markdown_dialect::{
    Extra, ParseOptions, html_opaque_ranges, markdown_options, parse_with_options,
    prose_extras_with,
};
use yeokja_parser_utils::{
    apply_splices, collect_splices, normalize_inline_text, resolve_reference_links,
};

pub struct BikeshedParser;

const SCHEME: &str = "bikeshed:";

const OPTIONS: ParseOptions = ParseOptions {
    preserve_heading_ids: true,
    inline_html: true,
};

impl DocumentParser for BikeshedParser {
    fn markup(&self) -> Markup {
        Markup::Bikeshed
    }

    fn parse(&self, source: &str) -> Document {
        let rewritten = rewrite(source);
        let layout = analyze(&rewritten);
        let mut shadow = rewritten.clone().into_bytes();
        let mut extras = Vec::new();
        if let Some(metadata) = &layout.metadata {
            blank(&mut shadow, metadata.clone());
            if let Some(value) = abstract_value(&rewritten, metadata.clone()) {
                extras.push(Extra {
                    range: value,
                    block_type: BlockType::Paragraph,
                    literal: true,
                });
            }
        }
        for range in &layout.element_prose {
            blank(&mut shadow, range.clone());
            extras.extend(prose_extras_with(&rewritten, range.clone(), OPTIONS));
        }
        let parsed = String::from_utf8(shadow).expect("blanking replaces whole ASCII lines");
        let runs = definition_lists(&parsed);
        let mut shadow = parsed.into_bytes();
        for run in runs {
            blank(&mut shadow, run.range);
            for item in run.items {
                extras.extend(prose_extras_with(&rewritten, item, OPTIONS));
            }
        }
        extras.sort_by_key(|extra| extra.range.start);
        let shadow = String::from_utf8(shadow).expect("blanking replaces whole ASCII lines");
        parse_with_options(&shadow, &rewritten, OPTIONS, extras)
    }

    fn reconstruct(&self, document: &Document, translations: &TranslationMap) -> String {
        let translations = resolve_reference_links(document, translations);
        let translations = dfn_terms_in_segments(document, &translations);
        let mut splices = collect_splices(document, &translations);
        splices.extend(dfn_terms_around_blocks(document, &translations));
        restore(&apply_splices(&document.source, splices))
    }
}

fn blank(shadow: &mut [u8], range: Range<usize>) {
    for b in &mut shadow[range] {
        if *b != b'\n' {
            *b = b' ';
        }
    }
}

// --- layout ------------------------------------------------------------

struct Layout {
    /// Regions no shorthand is read in: literal HTML elements and comments,
    /// fenced code, indented code outside any element.
    literal: Vec<Range<usize>>,
    /// Indented code blocks inside an open HTML element: prose.
    element_prose: Vec<Range<usize>>,
    /// The `<pre class=metadata>` element.
    metadata: Option<Range<usize>>,
}

const VOID: &[&str] = &[
    "area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta", "param", "source",
    "track", "wbr",
];

/// A tag at `start` (a `<`): its end, lower-case name, whether it closes and
/// whether it is self-closing. Quoted `>` in attributes are respected.
fn tag(text: &str, start: usize) -> Option<(usize, String, bool, bool)> {
    let bytes = text.as_bytes();
    let mut i = start + 1;
    let closing = bytes.get(i) == Some(&b'/');
    if closing {
        i += 1;
    }
    let name_start = i;
    if !bytes.get(i)?.is_ascii_alphabetic() {
        return None;
    }
    while bytes
        .get(i)
        .is_some_and(|b| b.is_ascii_alphanumeric() || *b == b'-')
    {
        i += 1;
    }
    let name = text[name_start..i].to_ascii_lowercase();
    if !bytes
        .get(i)
        .is_some_and(|b| b.is_ascii_whitespace() || matches!(b, b'/' | b'>'))
    {
        return None;
    }
    let mut quote = None;
    while let Some(&b) = bytes.get(i) {
        match (quote, b) {
            (Some(q), b) if b == q => quote = None,
            (None, b'\'' | b'"') => quote = Some(b),
            (None, b'>') => return Some((i + 1, name, closing, bytes[i - 1] == b'/')),
            _ => {}
        }
        i += 1;
    }
    None
}

/// The stack of HTML elements the block-level tags have opened, with the
/// end-tag omissions HTML allows (`<td>` closes a `<td>`, …). Only whether
/// it is empty matters.
fn open_element(stack: &mut Vec<String>, name: &str, self_closing: bool) {
    let implied: &[&str] = match name {
        "li" => &["p", "li"],
        "dt" | "dd" => &["p", "dt", "dd"],
        "td" | "th" => &["p", "td", "th"],
        "tr" => &["p", "td", "th", "tr"],
        "thead" | "tbody" | "tfoot" => &["p", "td", "th", "tr", "thead", "tbody", "tfoot"],
        "p" => &[],
        _ => &["p"],
    };
    while stack
        .last()
        .is_some_and(|top| implied.contains(&top.as_str()))
    {
        stack.pop();
    }
    if !self_closing && !VOID.contains(&name) {
        stack.push(name.to_string());
    }
}

fn close_element(stack: &mut Vec<String>, name: &str) {
    if let Some(at) = stack.iter().rposition(|open| open == name) {
        stack.truncate(at);
    }
}

fn analyze(text: &str) -> Layout {
    let opaque = html_opaque_ranges(text);
    let mut metadata = None;
    for range in &opaque {
        let open = &text[range.clone()];
        if open.starts_with("<pre")
            && tag(text, range.start).is_some_and(|(end, ..)| {
                let attrs = &text[range.start..end];
                attrs.contains("class=metadata")
                    || attrs.contains("class=\"metadata\"")
                    || attrs.contains("class='metadata'")
            })
        {
            metadata = Some(range.clone());
            break;
        }
    }
    let mut literal = opaque.clone();
    let mut element_prose = Vec::new();
    let mut stack: Vec<String> = Vec::new();
    let mut opaque_index = 0;
    for (event, range) in Parser::new_ext(text, markdown_options()).into_offset_iter() {
        match event {
            Event::Html(_) => {
                let mut i = range.start;
                while i < range.end {
                    while opaque.get(opaque_index).is_some_and(|r| r.end <= i) {
                        opaque_index += 1;
                    }
                    if let Some(r) = opaque.get(opaque_index).filter(|r| r.contains(&i)) {
                        i = r.end;
                        continue;
                    }
                    if text.as_bytes()[i] == b'<'
                        && let Some((end, name, closing, self_closing)) = tag(text, i)
                    {
                        if closing {
                            close_element(&mut stack, &name);
                        } else {
                            open_element(&mut stack, &name, self_closing);
                        }
                        i = end;
                        continue;
                    }
                    i += 1;
                }
            }
            Event::Start(Tag::CodeBlock(CodeBlockKind::Fenced(_))) => literal.push(range),
            Event::Start(Tag::CodeBlock(CodeBlockKind::Indented)) => {
                if stack.is_empty() {
                    literal.push(range);
                } else {
                    element_prose.push(range);
                }
            }
            _ => {}
        }
    }
    literal.sort_by_key(|r| r.start);
    Layout {
        literal,
        element_prose,
        metadata,
    }
}

/// A run of definition-list shorthand lines inside one CommonMark paragraph.
struct DefinitionList {
    /// From the first `:`/`::` line to the end of the paragraph: blanked.
    range: Range<usize>,
    /// The text of each `<dt>`/`<dd>`: after the prefix, with the lines that
    /// continue the item.
    items: Vec<Range<usize>>,
}

/// The byte offset where the text of a `: term` or `:: definition` line
/// starts, as Bikeshed reads it: the stripped line begins with one or two
/// colons followed by whitespace or the end of the line.
fn definition_prefix(line: &str) -> Option<usize> {
    let indent = line.len() - line.trim_start().len();
    let colons = line[indent..].bytes().take_while(|b| *b == b':').count();
    if !(1..=2).contains(&colons) {
        return None;
    }
    let rest = &line[indent + colons..];
    if !rest.trim_end_matches(['\n', '\r']).is_empty() && !rest.starts_with([' ', '\t']) {
        return None;
    }
    Some(indent + colons + rest.bytes().take_while(|b| matches!(b, b' ' | b'\t')).count())
}

/// Bikeshed's definition-list shorthand: a line whose stripped text starts
/// with `: ` opens a `<dt>` and `:: ` a `<dd>`; the lines up to the next
/// such line continue the item. The tokenizer works line by line, so the
/// run starts at the first such line of a CommonMark paragraph (its earlier
/// lines stay a paragraph) and ends with it. Lines inside a nested item
/// (a deeper prefix) are read as items of the same list.
fn definition_lists(parsed: &str) -> Vec<DefinitionList> {
    let mut runs = Vec::new();
    for (event, range) in Parser::new_ext(parsed, markdown_options()).into_offset_iter() {
        if !matches!(event, Event::Start(Tag::Paragraph)) {
            continue;
        }
        let mut at = range.start;
        let mut starts: Vec<usize> = Vec::new();
        let mut run_start = None;
        for line in parsed[range.clone()].split_inclusive('\n') {
            if let Some(text) = definition_prefix(line) {
                run_start.get_or_insert(at);
                starts.push(at + text);
            }
            at += line.len();
        }
        let Some(run_start) = run_start else { continue };
        let items = starts
            .iter()
            .enumerate()
            .map(|(i, &start)| {
                let end = match starts.get(i + 1) {
                    Some(&next) => parsed[..next].rfind('\n').map_or(next, |n| n + 1),
                    None => range.end,
                };
                start..start.max(parsed[..end].trim_end().len())
            })
            .filter(|item| !parsed[item.clone()].trim().is_empty())
            .collect();
        runs.push(DefinitionList { range: run_start..range.end, items });
    }
    runs
}

/// The value of the `Abstract:` line of the metadata block.
fn abstract_value(text: &str, metadata: Range<usize>) -> Option<Range<usize>> {
    let mut at = metadata.start;
    for line in text[metadata].split_inclusive('\n') {
        if let Some(rest) = line.strip_prefix("Abstract:") {
            let value = rest.trim();
            if value.is_empty() {
                return None;
            }
            let start = at + "Abstract:".len() + (rest.len() - rest.trim_start().len());
            return Some(start..start + value.len());
        }
        at += line.len();
    }
    None
}

// --- shorthands ----------------------------------------------------------

fn encode(key: &str) -> String {
    let mut out = String::new();
    for b in key.bytes() {
        if b.is_ascii_alphanumeric() || b"-._~/#:".contains(&b) {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

fn decode(encoded: &str) -> String {
    let bytes = encoded.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%'
            && i + 2 < bytes.len()
            && let Ok(b) = u8::from_str_radix(&encoded[i + 1..i + 3], 16)
        {
            out.push(b);
            i += 3;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8(out).unwrap_or_else(|_| encoded.to_string())
}

/// What Bikeshed displays for a bare autolink: the term after its `for`.
fn display(key: &str) -> &str {
    key.rsplit('/').next().unwrap_or(key)
}

fn plain_text(text: &str) -> bool {
    !text.is_empty() && !text.contains(['[', ']', '`', '\n'])
}

/// `[=key=]`, `[=key|text=]` and `[[ref|text]]` as `bikeshed:` links, outside
/// literal regions and code spans.
pub fn rewrite(source: &str) -> String {
    let layout = analyze(source);
    let bytes = source.as_bytes();
    let mut out = String::with_capacity(source.len() + source.len() / 8);
    let mut literal = layout.literal.iter().peekable();
    let mut i = 0;
    while i < bytes.len() {
        while literal.peek().is_some_and(|r| r.end <= i) {
            literal.next();
        }
        if let Some(r) = literal.peek().filter(|r| r.contains(&i)) {
            out.push_str(&source[i..r.end]);
            i = r.end;
            continue;
        }
        match bytes[i] {
            b'`' => {
                let len = bytes[i..].iter().take_while(|&&b| b == b'`').count();
                let end = code_span_end(source, i, len).unwrap_or(i + len);
                out.push_str(&source[i..end]);
                i = end;
            }
            b'[' if source[i..].starts_with("[=") => match autolink(source, i) {
                Some((end, replacement)) => {
                    out.push_str(&replacement);
                    i = end;
                }
                None => {
                    out.push('[');
                    i += 1;
                }
            },
            b'[' if source[i..].starts_with("[[") && !source[i..].starts_with("[[=") => {
                match reference(source, i) {
                    Some((end, replacement)) => {
                        out.push_str(&replacement);
                        i = end;
                    }
                    None => {
                        out.push('[');
                        i += 1;
                    }
                }
            }
            _ => {
                let c = source[i..].chars().next().unwrap();
                out.push(c);
                i += c.len_utf8();
            }
        }
    }
    out
}

/// The end of a code span opened by `len` backticks at `start`, when a run
/// of the same length closes it.
fn code_span_end(text: &str, start: usize, len: usize) -> Option<usize> {
    let bytes = text.as_bytes();
    let mut i = start + len;
    while i < bytes.len() {
        if bytes[i] == b'`' {
            let run = bytes[i..].iter().take_while(|&&b| b == b'`').count();
            if run == len {
                return Some(i + run);
            }
            i += run;
        } else {
            i += 1;
        }
    }
    None
}

/// The autolink at `start` (`[=`): its end and rewritten form.
fn autolink(text: &str, start: usize) -> Option<(usize, String)> {
    let line_end = text[start..].find('\n').map_or(text.len(), |n| start + n);
    let close = start + 2 + text[start + 2..line_end].find("=]")?;
    let inner = &text[start + 2..close];
    let replacement = match inner.split_once('|') {
        Some((key, label)) if plain_text(label) && !key.is_empty() => {
            format!("[{label}]({SCHEME}={}|=)", encode(key))
        }
        None if plain_text(display(inner)) => {
            format!("[{}]({SCHEME}={}=)", display(inner), encode(inner))
        }
        _ => return None,
    };
    Some((close + 2, replacement))
}

/// The reference at `start` (`[[`) when it has display text: its end and
/// rewritten form. A bare reference stays as it is.
fn reference(text: &str, start: usize) -> Option<(usize, String)> {
    let line_end = text[start..].find('\n').map_or(text.len(), |n| start + n);
    let close = start + 2 + text[start + 2..line_end].find("]]")?;
    let inner = &text[start + 2..close];
    let (key, label) = inner.split_once('|')?;
    if !plain_text(label) || key.is_empty() || key.contains('[') {
        return None;
    }
    Some((close + 2, format!("[{label}]({SCHEME}[[{}]])", encode(key))))
}

/// The `[` that opens the link text ending at `close` (a `]`), skipping
/// balanced brackets inside the text.
fn matching_open(text: &str, close: usize) -> Option<usize> {
    let bytes = text.as_bytes();
    let mut depth = 0usize;
    let mut i = close;
    while i > 0 {
        i -= 1;
        match bytes[i] {
            b']' => depth += 1,
            b'[' if depth == 0 => return Some(i),
            b'[' => depth -= 1,
            b'\n' => return None,
            _ => {}
        }
    }
    None
}

fn shorthand(destination: &str, label: &str) -> Option<String> {
    if let Some(key) = destination
        .strip_prefix('=')
        .and_then(|d| d.strip_suffix("|="))
    {
        return Some(format!("[={}|{label}=]", decode(key)));
    }
    if let Some(key) = destination
        .strip_prefix('=')
        .and_then(|d| d.strip_suffix('='))
    {
        let key = decode(key);
        return Some(if label == display(&key) {
            format!("[={key}=]")
        } else {
            format!("[={key}|{label}=]")
        });
    }
    if let Some(key) = destination
        .strip_prefix("[[")
        .and_then(|d| d.strip_suffix("]]"))
    {
        return Some(format!("[[{}|{label}]]", decode(key)));
    }
    None
}

/// Every `[text](bikeshed:…)` link written back as its shorthand.
pub fn restore(text: &str) -> String {
    let needle = format!("]({SCHEME}");
    let mut out = String::with_capacity(text.len());
    let mut pos = 0;
    while let Some(found) = text[pos..].find(&needle) {
        let close = pos + found;
        let destination_start = close + needle.len();
        let (Some(open), Some(destination_len)) = (
            matching_open(text, close),
            text[destination_start..].find(')'),
        ) else {
            out.push_str(&text[pos..destination_start]);
            pos = destination_start;
            continue;
        };
        let destination = &text[destination_start..destination_start + destination_len];
        let label = &text[open + 1..close];
        match shorthand(destination, label) {
            Some(written) => {
                out.push_str(&text[pos..open]);
                out.push_str(&written);
                pos = destination_start + destination_len + 1;
            }
            None => {
                out.push_str(&text[pos..destination_start]);
                pos = destination_start;
            }
        }
    }
    out.push_str(&text[pos..]);
    out
}

// --- <dfn> ----------------------------------------------------------------

/// The `<dfn …>` open tags and inner texts of `text`, in order.
fn dfns(text: &str) -> Vec<(Range<usize>, Range<usize>)> {
    let mut found = Vec::new();
    let mut pos = 0;
    while let Some(at) = text[pos..].find("<dfn") {
        let start = pos + at;
        pos = start + 4;
        let Some((open_end, name, false, false)) = tag(text, start) else {
            continue;
        };
        if name != "dfn" {
            continue;
        }
        let Some(close) = text[open_end..].find("</dfn>") else {
            break;
        };
        found.push((start..open_end, open_end..open_end + close));
        pos = open_end + close + 6;
    }
    found
}

fn has_lt(open_tag: &str) -> bool {
    open_tag
        .split(|c: char| c.is_whitespace() || c == '>')
        .skip(1)
        .any(|attr| {
            attr.eq_ignore_ascii_case("lt")
                || attr
                    .get(..3)
                    .is_some_and(|head| head.eq_ignore_ascii_case("lt="))
        })
}

fn escape_attribute(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            c => out.push(c),
        }
    }
    out
}

/// The text content of `inner` as Bikeshed matches it: shorthands written
/// back, tags removed, one line.
fn term_of(inner: &str) -> String {
    let restored = restore(inner);
    let mut out = String::with_capacity(restored.len());
    let mut i = 0;
    while i < restored.len() {
        if restored.as_bytes()[i] == b'<'
            && let Some((end, ..)) = tag(&restored, i)
        {
            i = end;
            continue;
        }
        let c = restored[i..].chars().next().unwrap();
        out.push(c);
        i += c.len_utf8();
    }
    normalize_inline_text(&out)
}

/// `open_tag` with ` lt="term"` added before its `>`.
fn with_lt(open_tag: &str, term: &str) -> String {
    let body = open_tag.strip_suffix('>').unwrap_or(open_tag).trim_end();
    format!("{body} lt=\"{}\">", escape_attribute(term))
}

/// Translations whose `<dfn>` elements, paired with the segment's in order,
/// name the original term when their text changed.
fn dfn_terms_in_segments(document: &Document, translations: &TranslationMap) -> TranslationMap {
    let mut out = translations.clone();
    for segment in document
        .sections
        .iter()
        .flat_map(|s| &s.blocks)
        .flat_map(|b| &b.segments)
    {
        let Some(translation) = translations.get(&segment.id) else {
            continue;
        };
        let in_source = dfns(&segment.source);
        let in_translation = dfns(translation);
        if in_source.is_empty() || in_source.len() != in_translation.len() {
            continue;
        }
        let mut edited = translation.clone();
        for ((_, source_inner), (open, inner)) in in_source.iter().zip(in_translation.iter()).rev()
        {
            let open_tag = &translation[open.clone()];
            if segment.source[source_inner.clone()] == translation[inner.clone()]
                || has_lt(open_tag)
            {
                continue;
            }
            edited.replace_range(
                open.clone(),
                &with_lt(open_tag, &term_of(&segment.source[source_inner.clone()])),
            );
        }
        out.insert(segment.id.clone(), edited);
    }
    out
}

/// Splices for the `<dfn>` open tags that wrap a whole translated block
/// (`<td><dfn>name</dfn>`: the prose of an HTML block is cut between its
/// tags, so the tags sit outside the span).
fn dfn_terms_around_blocks(
    document: &Document,
    translations: &TranslationMap,
) -> Vec<(Range<usize>, String)> {
    let source = &document.source;
    let mut splices = Vec::new();
    for block in document.sections.iter().flat_map(|s| &s.blocks) {
        let Some(span) = &block.span else { continue };
        if !block.translatable || !source[span.end..].starts_with("</dfn>") {
            continue;
        }
        let changed = block.segments.iter().any(|segment| {
            translations
                .get(&segment.id)
                .is_some_and(|t| *t != segment.source)
        });
        if !changed {
            continue;
        }
        let Some(open_start) = source[..span.start].rfind('<') else {
            continue;
        };
        let Some((open_end, name, false, false)) = tag(source, open_start) else {
            continue;
        };
        if name != "dfn" || open_end != span.start || has_lt(&source[open_start..open_end]) {
            continue;
        }
        splices.push((
            open_start..open_end,
            with_lt(&source[open_start..open_end], &term_of(&block.raw_content)),
        ));
    }
    splices
}

#[cfg(test)]
mod tests {
    use super::*;

    const EXCERPT: &str = include_str!("../tests/fixtures/index-excerpt.bs");

    fn sources(doc: &Document) -> Vec<String> {
        doc.translatable_segments()
            .iter()
            .map(|s| s.source.clone())
            .collect()
    }

    fn translate_all(doc: &Document, with: impl Fn(&str) -> String) -> TranslationMap {
        doc.translatable_segments()
            .iter()
            .map(|s| (s.id.clone(), with(&s.source)))
            .collect()
    }

    #[test]
    fn autolinks_and_references_become_links_and_come_back() {
        let source = "See [=entry name=], [=const/HEADER_MAGIC=], [=match|matching=], [[#block|the block]], [[#block]] and [[!AGE]].\n";
        let rewritten = rewrite(source);
        assert_eq!(
            rewritten,
            "See [entry name](bikeshed:=entry%20name=), [HEADER_MAGIC](bikeshed:=const/HEADER_MAGIC=), \
             [matching](bikeshed:=match|=), [the block](bikeshed:[[#block]]), [[#block]] and [[!AGE]].\n"
        );
        assert_eq!(restore(&rewritten), source);
    }

    #[test]
    fn translated_link_text_writes_the_explicit_form() {
        assert_eq!(
            restore("[항목 이름](bikeshed:=entry%20name=)"),
            "[=entry name|항목 이름=]"
        );
        assert_eq!(
            restore("[entry name](bikeshed:=entry%20name=)"),
            "[=entry name=]"
        );
        assert_eq!(
            restore("[matching](bikeshed:=match|=)"),
            "[=match|matching=]"
        );
        assert_eq!(restore("[match](bikeshed:=match|=)"), "[=match|match=]");
        assert_eq!(restore("[블록](bikeshed:[[#block]])"), "[[#block|블록]]");
    }

    #[test]
    fn neighbouring_brackets_and_parentheses_survive() {
        let source = "[=u8=][[=block/size=]] and [=u=]([=meta/x=])[[=meta/n=] + 1]\n";
        let rewritten = rewrite(source);
        assert_eq!(
            rewritten,
            "[u8](bikeshed:=u8=)[[size](bikeshed:=block/size=)] and [u](bikeshed:=u=)([x](bikeshed:=meta/x=))[[n](bikeshed:=meta/n=) + 1]\n"
        );
        assert_eq!(restore(&rewritten), source);
        let doc = BikeshedParser.parse(source);
        assert_eq!(sources(&doc), [rewritten.trim_end()]);
    }

    #[test]
    fn literal_regions_and_code_spans_are_not_rewritten() {
        let source = "<pre class=cddl>\n[=not=] here\n</pre>\n\n`[=code=]` and <code>[=code=]</code> but [=yes=].\n\n    [=indented code=]\n\n```\n[=fenced=]\n```\n";
        let rewritten = rewrite(source);
        assert_eq!(
            rewritten,
            source.replace("but [=yes=]", "but [yes](bikeshed:=yes=)")
        );
        assert_eq!(restore(&rewritten), source);
    }

    #[test]
    fn metadata_offers_only_the_abstract_as_a_literal() {
        let source = "<h1>Title</h1>\n<pre class=metadata>\nShortname: x\nAbstract: An abstract about [=terms=].\nMarkup Shorthands: markdown yes\n</pre>\n\nBody.\n";
        let doc = BikeshedParser.parse(source);
        assert_eq!(
            sources(&doc),
            ["Title", "An abstract about [=terms=].", "Body."]
        );
        let abstract_block = doc
            .sections
            .iter()
            .flat_map(|s| &s.blocks)
            .find(|b| b.raw_content.starts_with("An abstract"))
            .unwrap();
        assert_eq!(abstract_block.role, BlockRole::Literal);
        assert_eq!(abstract_block.block_type, BlockType::Paragraph);
        let translations = translate_all(&doc, |s| format!("K {s}"));
        assert_eq!(
            BikeshedParser.reconstruct(&doc, &translations),
            "<h1>K Title</h1>\n<pre class=metadata>\nShortname: x\nAbstract: K An abstract about [=terms=].\nMarkup Shorthands: markdown yes\n</pre>\n\nK Body.\n"
        );
    }

    #[test]
    fn indented_lists_inside_an_open_element_are_prose_and_outside_are_code() {
        let source = "<div class=note>\n  Intro.\n\n    * Bullet with |n|.\n    * Second.\n</div>\n\n    code line\n\nAfter.\n";
        let doc = BikeshedParser.parse(source);
        assert_eq!(
            sources(&doc),
            ["Intro.", "Bullet with |n|.", "Second.", "After."]
        );
        assert_eq!(
            BikeshedParser.reconstruct(&doc, &TranslationMap::new()),
            source
        );
        let translations = translate_all(&doc, |_| "번역".to_string());
        assert_eq!(
            BikeshedParser.reconstruct(&doc, &translations),
            "<div class=note>\n  번역\n\n    * 번역\n    * 번역\n</div>\n\n    code line\n\n번역\n"
        );
    }

    #[test]
    fn algorithm_steps_with_inline_tags_stay_whole() {
        let source = "<ol class=algorithm>\n    1. If |bytes|[0] is C1<sub>16</sub>, let |name| be an [=entry name=]\n        whose bytes are |bytes|.\n    2. Let |e|<sub>|i|</sub> be the <dfn>visible range</dfn>.\n    3. Return |name|.\n</ol>\n";
        let doc = BikeshedParser.parse(source);
        assert_eq!(
            sources(&doc),
            [
                "If |bytes|[0] is C1<sub>16</sub>, let |name| be an [entry name](bikeshed:=entry%20name=) whose bytes are |bytes|.",
                "Let |e|<sub>|i|</sub> be the <dfn>visible range</dfn>.",
                "Return |name|."
            ]
        );
        assert_eq!(BikeshedParser.reconstruct(&doc, &TranslationMap::new()), source);
        let identity = translate_all(&doc, |s| s.to_string());
        assert_eq!(
            BikeshedParser.reconstruct(&doc, &identity),
            source.replace("[=entry name=]\n        whose", "[=entry name=] whose")
        );
    }

    #[test]
    fn definition_list_shorthand_offers_each_item_on_its_own() {
        let source = "Imports are:\n\n: `aqr_memory`\n:: A memory, the <dfn>working memory</dfn>.\n: <dfn>aqr_request</dfn>(|k|, |hi|)\n:: The |k|-th input MUST be imported.\n    |hi| MUST be in [0, |n|].\n    Then set |hi|.\n:\n:: Done.\n\nAfter.\n";
        let doc = BikeshedParser.parse(source);
        let blocks: Vec<&Block> = doc.sections.iter().flat_map(|s| &s.blocks).filter(|b| b.translatable).collect();
        assert_eq!(
            blocks.iter().map(|b| b.raw_content.as_str()).collect::<Vec<_>>(),
            [
                "Imports are:",
                "`aqr_memory`",
                "A memory, the <dfn>working memory</dfn>.",
                "<dfn>aqr_request</dfn>(|k|, |hi|)",
                "The |k|-th input MUST be imported.\n    |hi| MUST be in [0, |n|].\n    Then set |hi|.",
                "Done.",
                "After."
            ]
        );
        assert!(blocks.iter().all(|b| b.block_type == BlockType::Paragraph));
        assert_eq!(BikeshedParser.reconstruct(&doc, &TranslationMap::new()), source);
        let translations = translate_all(&doc, |s| format!("K {s}"));
        assert_eq!(
            BikeshedParser.reconstruct(&doc, &translations),
            "K Imports are:\n\n: K `aqr_memory`\n:: K A memory, the <dfn>working memory</dfn>.\n: K <dfn>aqr_request</dfn>(|k|, |hi|)\n:: K The |k|-th input MUST be imported. |hi| MUST be in [0, |n|]. K Then set |hi|.\n:\n:: K Done.\n\nK After.\n"
        );
    }

    #[test]
    fn a_colon_line_in_the_middle_of_a_paragraph_starts_the_list() {
        let source = "Lead line.\n: term\n:: definition\n";
        let doc = BikeshedParser.parse(source);
        assert_eq!(sources(&doc), ["Lead line.", "term", "definition"]);
        assert_eq!(BikeshedParser.reconstruct(&doc, &TranslationMap::new()), source);
        assert!(definition_prefix("::: not a dd\n").is_none());
        assert!(definition_prefix(":nope\n").is_none());
        assert_eq!(definition_prefix("  ::  text\n"), Some(6));
        assert_eq!(definition_prefix(":\n"), Some(1));
    }

    #[test]
    fn unclosed_table_cells_do_not_keep_the_element_open_forever() {
        let source = "<table>\n<tr><td>a\n<td>b\n</table>\n\n    code\n";
        let doc = BikeshedParser.parse(source);
        assert_eq!(sources(&doc), ["a", "b"]);
    }

    #[test]
    fn a_changed_dfn_gets_its_original_term_as_lt() {
        let source = "A header and a footer <dfn>match</dfn> when equal.\n\n<table>\n<tr>\n  <td><dfn for=type lt=\"u8\">u8</dfn>\n  <td><dfn noexport>entry \"name\" &amp; more</dfn>\n</table>\n";
        let doc = BikeshedParser.parse(source);
        let translations = translate_all(&doc, |s| {
            s.replace("<dfn>match</dfn>", "<dfn>일치</dfn>")
                .replace("entry \"name\" &amp; more", "항목")
        });
        assert_eq!(
            BikeshedParser.reconstruct(&doc, &translations),
            "A header and a footer <dfn lt=\"match\">일치</dfn> when equal.\n\n<table>\n<tr>\n  <td><dfn for=type lt=\"u8\">u8</dfn>\n  <td><dfn noexport lt=\"entry &quot;name&quot; &amp;amp; more\">항목</dfn>\n</table>\n"
        );
        let unchanged = translate_all(&doc, |s| s.to_string());
        assert_eq!(BikeshedParser.reconstruct(&doc, &unchanged), source);
    }

    #[test]
    fn excerpt_round_trips_byte_for_byte_without_translations() {
        let doc = BikeshedParser.parse(EXCERPT);
        assert_eq!(
            BikeshedParser.reconstruct(&doc, &TranslationMap::new()),
            EXCERPT
        );
        for block in doc.sections.iter().flat_map(|s| &s.blocks) {
            if let Some(span) = &block.span {
                assert_eq!(&doc.source[span.clone()], block.raw_content);
            }
        }
    }

    #[test]
    fn excerpt_offers_the_expected_prose() {
        let doc = BikeshedParser.parse(EXCERPT);
        let sources = sources(&doc);
        let has = |text: &str| sources.iter().any(|s| s.contains(text));
        assert!(
            has("This specification defines the Aqueous archive format"),
            "abstract"
        );
        assert!(
            !has("Shortname") && !has("urlPrefix") && !has("spec:infra"),
            "metadata and pre blocks stay opaque"
        );
        assert!(
            !has("aqr-opt-metadata") && !has("$$aqr"),
            "cddl stays opaque"
        );
        assert!(
            has("An uncompressed leaf block without a checksum"),
            "indented list in a note"
        );
        assert!(
            has("0x756e (ASi UNIX) is absorbed like 0x000d:"),
            "indented list in the last note"
        );
        assert!(
            !has("Four spaces after a blank line outside any element"),
            "top-level indented code"
        );
        assert!(
            has("If [=file=]")
                || has("If |file| starts with [HEADER_MAGIC](bikeshed:=const/HEADER_MAGIC=)"),
            "algorithm steps"
        );
        assert!(
            has("[mirror blocks](bikeshed:[[#mirror-block]])"),
            "reference with text"
        );
        assert!(
            has("[[#container]]") && has("[[RFC8610]]") && has("[[!AGE]]"),
            "bare references stay"
        );
        assert!(has("`[=not a link=]`"), "code span untouched");
        assert!(sources.iter().any(|s| s == "HEADER_TYPE"), "dfn cell text");
        assert!(sources.iter().any(|s| s == "application"), "dd text");
        assert!(
            !sources.iter().any(|s| s.contains("bikeshed:=u8=)[[size]"))
                || has("[u8](bikeshed:=u8=)[[size](bikeshed:=block/size=)]")
        );
        assert!(
            !sources.iter().any(|s| s.contains("[=") && !s.contains('`')),
            "no raw autolink left: {sources:?}"
        );
    }

    /// `YEOKJA_BIKESHED_FILE=path cargo test -p yeokja-parser-bikeshed -- --ignored`:
    /// a whole specification round-trips without translations, and with
    /// every segment translated no `bikeshed:` link is left behind.
    #[test]
    #[ignore]
    fn a_real_specification_round_trips() {
        let path = std::env::var("YEOKJA_BIKESHED_FILE").expect("YEOKJA_BIKESHED_FILE");
        let source = std::fs::read_to_string(&path).unwrap();
        let doc = BikeshedParser.parse(&source);
        assert_eq!(
            BikeshedParser.reconstruct(&doc, &TranslationMap::new()),
            source
        );
        let translations = translate_all(&doc, |s| format!("K {s}"));
        let out = BikeshedParser.reconstruct(&doc, &translations);
        assert!(!out.contains("bikeshed:"));
        let sources = sources(&doc);
        eprintln!(
            "{} segments, {} with bikeshed links",
            sources.len(),
            sources.iter().filter(|s| s.contains("bikeshed:")).count()
        );
        assert!(
            !sources.iter().any(|s| s.contains("[=") && !s.contains('`')),
            "raw autolink left"
        );
    }

    #[test]
    fn excerpt_translated_everywhere_restores_every_shorthand() {
        let doc = BikeshedParser.parse(EXCERPT);
        let translations = translate_all(&doc, |s| format!("K {s}"));
        let out = BikeshedParser.reconstruct(&doc, &translations);
        assert!(!out.contains("bikeshed:"), "{out}");
        assert!(out.contains("[=const/HEADER_MAGIC=]") && out.contains("[=match|matching=]"));
        assert!(out.contains("[[#mirror-block|mirror blocks]]"));
        assert!(out.contains("<pre class=cddl>\naqr-opt-metadata = {"));
        assert!(
            out.contains("<dfn>match</dfn> when"),
            "an unchanged dfn text needs no lt: {out}"
        );
        assert!(
            out.contains("<dfn noexport lt=\"HEADER_TYPE\">K HEADER_TYPE</dfn>"),
            "{out}"
        );
    }
}
