//! MyST Markdown flavour of the Markdown parser, as written for Sphinx.
//!
//! MyST adds a handful of block constructs to CommonMark that pulldown-cmark
//! cannot see through: YAML front matter, `(label)=` targets, `{key=value}`
//! attribute blocks and, above all, directives written as colon fences
//! (`:::{note}` … `:::`) or backtick fences (```` ```{glossary} ````). Left
//! alone, the closing `---` of front matter becomes a setext underline, a
//! colon fence becomes part of the surrounding paragraph, and every backtick
//! directive — prose or not — becomes an opaque code block.
//!
//! This parser scans the source line by line first and sorts what it finds:
//!
//! * Front matter, targets, attribute blocks, fence opener and closer lines
//!   and directive option lines (`:class: warning`) are blanked to spaces —
//!   newlines kept — in a shadow copy handed to pulldown-cmark, so byte
//!   offsets line up with the original and the body of a prose directive
//!   parses as ordinary Markdown, whatever its indentation.
//! * A directive's title (`:::{admonition} Notable uses`) is offered as a
//!   [`BlockType::Heading`] block of its own; no escaping applies.
//! * Literal directives (`code-block`, `literalinclude`, `toctree`, …) stay
//!   with pulldown-cmark as code blocks, but their `:caption:` values and a
//!   toctree's explicit entry titles (`Title <target>`) are offered as
//!   [`BlockType::Paragraph`] blocks.
//! * A `glossary` body is a definition list, which CommonMark would read as
//!   indented code, so it is blanked whole and walked here: each indented
//!   paragraph is a [`BlockType::Paragraph`] and each list item a
//!   [`BlockType::ListItem`], with markers and leading indentation kept
//!   outside the span so reconstruction keeps the layout Sphinx expects.
//!   Term lines stay verbatim, since `{term}` roles refer to them by their
//!   source text.
//!
//! Which fences are prose is decided by the directive name: colon fences are
//! prose unless the name is a known literal directive, backtick fences are
//! literal unless the name is a known prose directive. Roles (`` {term}`Nix` ``)
//! are inline code spans to CommonMark and travel inside segments unchanged.

use std::ops::Range;
use yeokja_core::model::*;
use yeokja_core::parser::{DocumentParser, Markup, TranslationMap};
use yeokja_parser_utils::{resolve_reference_links, splice_reconstruct};

use yeokja_parser_markdown_dialect::{Extra, parse_with};

/// MyST parser: Markdown with Sphinx directives, roles and targets.
pub struct MystParser;

/// Directives whose body is verbatim content, never prose.
const LITERAL_DIRECTIVES: &[&str] = &[
    "code",
    "code-block",
    "code-cell",
    "sourcecode",
    "literalinclude",
    "toctree",
    "contributors",
    "include",
    "image",
    "figure",
    "raw",
    "math",
    "mermaid",
    "csv-table",
    "highlight",
    "only",
    "index",
];

/// Directives whose body is Markdown prose.
const PROSE_DIRECTIVES: &[&str] = &[
    "admonition",
    "attention",
    "caution",
    "danger",
    "error",
    "hint",
    "important",
    "note",
    "seealso",
    "tip",
    "warning",
    "dropdown",
    "tab-set",
    "tab-item",
    "grid",
    "grid-item",
    "grid-item-card",
    "card",
    "div",
    "aside",
    "sidebar",
    "topic",
    "margin",
    "epigraph",
    "list-table",
    "versionadded",
    "versionchanged",
    "deprecated",
];

/// How a directive's body is read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Body {
    /// Ordinary Markdown: the fence lines are blanked, the body is left to
    /// pulldown-cmark.
    Prose,
    /// Verbatim content: a backtick fence stays a code block, a colon fence is
    /// blanked whole.
    Literal,
    /// A definition list, blanked whole and walked by [`scan_glossary`].
    Glossary,
}

/// A user-visible string inside a region Markdown does not see.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Field {
    range: Range<usize>,
    block_type: BlockType,
}

/// What the line scan found.
#[derive(Debug, Default)]
struct Layout {
    /// Regions to blank out before Markdown parsing, each kept verbatim as an
    /// opaque block. Adjacent lines are merged into one region.
    opaque: Vec<Range<usize>>,
    /// Translatable strings inside those regions, or inside code blocks.
    fields: Vec<Field>,
}

impl Layout {
    /// Blank `range`, merging it with the previous region when contiguous.
    fn blank(&mut self, range: Range<usize>) {
        if let Some(last) = self.opaque.last_mut()
            && last.end == range.start
        {
            last.end = range.end;
            return;
        }
        self.opaque.push(range);
    }

    fn field(&mut self, source: &str, range: Range<usize>, block_type: BlockType) {
        if !source[range.clone()].trim().is_empty() {
            self.fields.push(Field { range, block_type });
        }
    }

    /// Offer a directive's argument as its title, unless it is not prose at
    /// all (`{grid} 2`, `{versionadded} 1.2`).
    fn title(&mut self, source: &str, range: Range<usize>) {
        if source[range.clone()].contains(char::is_alphabetic) {
            self.fields.push(Field {
                range,
                block_type: BlockType::Heading,
            });
        }
    }

    /// The source with every opaque region replaced by spaces, newlines kept,
    /// so offsets into it are offsets into the original.
    fn shadow(&self, source: &str) -> String {
        let mut bytes = source.as_bytes().to_vec();
        for range in &self.opaque {
            for byte in &mut bytes[range.clone()] {
                if *byte != b'\n' && *byte != b'\r' {
                    *byte = b' ';
                }
            }
        }
        String::from_utf8(bytes).expect("blanking ASCII bytes keeps UTF-8 valid")
    }

    fn extras(&self) -> Vec<Extra> {
        let mut extras: Vec<Extra> = self
            .opaque
            .iter()
            .map(|range| Extra {
                range: range.clone(),
                block_type: BlockType::HtmlBlock,
            })
            .chain(self.fields.iter().map(|field| Extra {
                range: field.range.clone(),
                block_type: field.block_type,
            }))
            .collect();
        extras.sort_by_key(|extra| (extra.range.start, extra.range.end));
        extras
    }
}

fn scan(source: &str) -> Layout {
    let mut layout = Layout::default();
    let body_start = scan_front_matter(source, &mut layout);
    scan_body(source, body_start, &mut layout);
    layout
}

/// Byte ranges of every line from `from`, each including its newline.
fn line_ranges(source: &str, from: usize) -> Vec<Range<usize>> {
    let mut ranges = Vec::new();
    let mut start = from;
    while start < source.len() {
        let end = source[start..]
            .find('\n')
            .map(|i| start + i + 1)
            .unwrap_or(source.len());
        ranges.push(start..end);
        start = end;
    }
    ranges
}

fn line_text<'a>(source: &'a str, range: &Range<usize>) -> &'a str {
    source[range.clone()].trim_end_matches(['\n', '\r'])
}

/// Record the YAML front matter, if any, and return where the body starts.
fn scan_front_matter(source: &str, layout: &mut Layout) -> usize {
    let lines = line_ranges(source, 0);
    let Some(first) = lines.first() else { return 0 };
    if line_text(source, first) != "---" {
        return 0;
    }
    let Some(close) = lines[1..]
        .iter()
        .position(|range| matches!(line_text(source, range), "---" | "..."))
        .map(|i| i + 1)
    else {
        return 0;
    };
    layout.blank(0..lines[close].end);
    lines[close].end
}

/// A fence line: its marker byte and how many of them open it.
struct FenceMarker {
    marker: u8,
    len: usize,
}

/// The run of 3+ backticks, tildes or colons a line starts with, if any.
fn fence_marker(trimmed: &str) -> Option<FenceMarker> {
    let marker = *trimmed.as_bytes().first()?;
    if !matches!(marker, b'`' | b'~' | b':') {
        return None;
    }
    let len = trimmed.bytes().take_while(|&b| b == marker).count();
    (len >= 3).then_some(FenceMarker { marker, len })
}

/// A directive opener: `:::{name} title` or ```` ```{name} args ````. The
/// info string is trimmed, so `::: {note}` opens a directive too.
struct Opener {
    marker: FenceMarker,
    name: String,
    /// The trimmed text after `{name}`, relative to the line's content start.
    title: Range<usize>,
}

fn directive_opener(trimmed: &str) -> Option<Opener> {
    let marker = fence_marker(trimmed)?;
    let info = trimmed[marker.len..].trim_start();
    let brace = trimmed.len() - info.len();
    let name = info.strip_prefix('{')?;
    let close = name.find('}')?;
    let name = &name[..close];
    if name.is_empty()
        || !name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == ':')
    {
        return None;
    }
    let name_end = brace + 1 + close + 1;
    let after = &trimmed[name_end..];
    let title_start = name_end + (after.len() - after.trim_start().len());
    let title_end = name_end + after.trim_end().len();
    Some(Opener {
        marker,
        name: name.to_string(),
        title: title_start..title_end.max(title_start),
    })
}

/// Whether a line closes the fence opened with `marker`.
fn closes(trimmed: &str, marker: &FenceMarker) -> bool {
    let count = trimmed.bytes().take_while(|&b| b == marker.marker).count();
    count >= marker.len && trimmed[count..].trim().is_empty()
}

/// A directive option line: `:key:` or `:key: value`.
fn option_line(trimmed: &str) -> Option<(&str, &str)> {
    let rest = trimmed.strip_prefix(':')?;
    let colon = rest.find(':')?;
    let key = &rest[..colon];
    if key.is_empty()
        || !key
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        return None;
    }
    let value = &rest[colon + 1..];
    if !(value.is_empty() || value.starts_with(' ') || value.starts_with('\t')) {
        return None;
    }
    Some((key, value.trim()))
}

/// A MyST target line: `(label)=`.
fn is_target_line(trimmed: &str) -> bool {
    trimmed
        .strip_prefix('(')
        .and_then(|rest| rest.strip_suffix(")="))
        .is_some_and(|label| !label.is_empty() && !label.contains(['(', ')', ' ', '\t']))
}

/// An attribute block line: `{key=value …}`.
fn is_attribute_line(trimmed: &str) -> bool {
    trimmed
        .strip_prefix('{')
        .and_then(|rest| rest.strip_suffix('}'))
        .is_some_and(|inner| {
            inner.starts_with(|c: char| c.is_ascii_alphabetic() || c == '.' || c == '#')
                && !inner.contains(['{', '}'])
        })
}

fn body_kind(name: &str, marker: u8) -> Body {
    if name == "glossary" {
        Body::Glossary
    } else if LITERAL_DIRECTIVES.contains(&name) {
        Body::Literal
    } else if marker == b':' || PROSE_DIRECTIVES.contains(&name) {
        Body::Prose
    } else {
        Body::Literal
    }
}

/// An open fence on the scan stack.
struct Fence {
    marker: FenceMarker,
    body: Body,
    /// Blank every line inside (a literal colon fence, which pulldown-cmark
    /// would otherwise read as prose).
    blank_body: bool,
}

fn scan_body(source: &str, body_start: usize, layout: &mut Layout) {
    let lines = line_ranges(source, body_start);
    let mut stack: Vec<Fence> = Vec::new();
    let mut index = 0;

    while index < lines.len() {
        let range = &lines[index];
        let text = line_text(source, range);
        let trimmed = text.trim_start();
        let indent = text.len() - trimmed.len();
        let content_start = range.start + indent;

        // Inside verbatim content only the closer matters.
        if let Some(top) = stack.last()
            && top.body == Body::Literal
        {
            if closes(trimmed, &top.marker) {
                if top.blank_body {
                    layout.blank(range.clone());
                }
                stack.pop();
            } else if top.blank_body {
                layout.blank(range.clone());
            }
            index += 1;
            continue;
        }

        if let Some(top) = stack.last()
            && top.marker.marker == b':'
            && closes(trimmed, &top.marker)
        {
            layout.blank(range.clone());
            stack.pop();
            index += 1;
            continue;
        }
        if let Some(top) = stack.last()
            && top.marker.marker != b':'
            && closes(trimmed, &top.marker)
        {
            // A prose backtick directive ends: its closer must not open a code block.
            layout.blank(range.clone());
            stack.pop();
            index += 1;
            continue;
        }

        if let Some(opener) = directive_opener(trimmed) {
            let body = body_kind(&opener.name, opener.marker.marker);
            let title = content_start + opener.title.start..content_start + opener.title.end;
            match body {
                Body::Prose => {
                    layout.blank(range.clone());
                    layout.title(source, title);
                    index = blank_options(source, &lines, index + 1, layout);
                    stack.push(Fence {
                        marker: opener.marker,
                        body,
                        blank_body: false,
                    });
                }
                Body::Literal if opener.marker.marker == b':' => {
                    layout.blank(range.clone());
                    stack.push(Fence {
                        marker: opener.marker,
                        body,
                        blank_body: true,
                    });
                    index += 1;
                }
                Body::Literal => {
                    index = literal_fields(source, &lines, index + 1, &opener, layout);
                    stack.push(Fence {
                        marker: opener.marker,
                        body,
                        blank_body: false,
                    });
                }
                Body::Glossary => {
                    let close = (index + 1..lines.len())
                        .find(|&i| {
                            closes(line_text(source, &lines[i]).trim_start(), &opener.marker)
                        })
                        .unwrap_or(lines.len());
                    layout.blank(range.clone());
                    let options_end = blank_options(source, &lines, index + 1, layout);
                    scan_glossary(source, &lines[options_end..close], indent, layout);
                    if close < lines.len() {
                        layout.blank(lines[close].clone());
                    }
                    index = close + 1;
                }
            }
            continue;
        }

        // A plain code fence: verbatim until its closer.
        if let Some(marker) = fence_marker(trimmed)
            && marker.marker != b':'
        {
            stack.push(Fence {
                marker,
                body: Body::Literal,
                blank_body: false,
            });
            index += 1;
            continue;
        }

        if is_target_line(trimmed) || is_attribute_line(trimmed) {
            layout.blank(range.clone());
        }
        index += 1;
    }
}

/// Blank the option lines that follow a directive opener at `from`, whether
/// written as `:key: value` lines or as a `---` YAML block. Returns the index
/// of the first body line.
fn blank_options(source: &str, lines: &[Range<usize>], from: usize, layout: &mut Layout) -> usize {
    let mut index = from;
    if index < lines.len() && line_text(source, &lines[index]).trim() == "---" {
        layout.blank(lines[index].clone());
        index += 1;
        while index < lines.len() {
            let text = line_text(source, &lines[index]).trim();
            layout.blank(lines[index].clone());
            index += 1;
            if text == "---" {
                break;
            }
        }
        return index;
    }
    while index < lines.len()
        && option_line(line_text(source, &lines[index]).trim_start()).is_some()
    {
        layout.blank(lines[index].clone());
        index += 1;
    }
    index
}

/// Offer the reader-visible strings inside a literal backtick directive: its
/// `:caption:` and, for a toctree, explicit entry titles. Nothing is blanked;
/// the fence is pulldown-cmark's code block. Returns the index of the first
/// body line after the options.
fn literal_fields(
    source: &str,
    lines: &[Range<usize>],
    from: usize,
    opener: &Opener,
    layout: &mut Layout,
) -> usize {
    let mut index = from;
    while index < lines.len() {
        let range = &lines[index];
        let text = line_text(source, range);
        let trimmed = text.trim_start();
        let Some((key, value)) = option_line(trimmed) else {
            break;
        };
        if key == "caption" && !value.is_empty() {
            let start = range.start + text.len() - value.len();
            layout.field(source, start..start + value.len(), BlockType::Paragraph);
        }
        index += 1;
    }
    if opener.name == "toctree" {
        let mut entry = index;
        while entry < lines.len() {
            let range = &lines[entry];
            let text = line_text(source, range);
            let trimmed = text.trim_start();
            if closes(trimmed, &opener.marker) {
                break;
            }
            if let Some(title) = toctree_entry_title(trimmed) {
                let start = range.start + text.len() - trimmed.len();
                layout.field(source, start..start + title.len(), BlockType::Paragraph);
            }
            entry += 1;
        }
    }
    index
}

/// The title of a `Title <target>` toctree entry.
fn toctree_entry_title(trimmed: &str) -> Option<&str> {
    let rest = trimmed.strip_suffix('>')?;
    let open = rest.rfind('<')?;
    if rest[open + 1..].contains(char::is_whitespace) {
        return None;
    }
    let title = rest[..open].trim_end();
    (!title.is_empty()).then_some(title)
}

/// The list marker at the start of `trimmed`, with the whitespace after it.
fn list_marker_len(trimmed: &str) -> Option<usize> {
    let bytes = trimmed.as_bytes();
    let marker_end = match bytes.first()? {
        b'-' | b'*' | b'+' => 1,
        b if b.is_ascii_digit() => {
            let digits = bytes.iter().take_while(|b| b.is_ascii_digit()).count();
            if !matches!(bytes.get(digits), Some(b'.' | b')')) {
                return None;
            }
            digits + 1
        }
        _ => return None,
    };
    if !matches!(bytes.get(marker_end), Some(b' ' | b'\t')) {
        return None;
    }
    let spaces = bytes[marker_end..]
        .iter()
        .take_while(|b| matches!(b, b' ' | b'\t'))
        .count();
    Some(marker_end + spaces)
}

/// Walk a glossary body: terms at the directive's own indentation (kept
/// verbatim), indented definitions made of paragraphs, lists and nested
/// fences. Every line is blanked for Markdown; the translatable runs are
/// offered as fields whose spans start after any indentation or list marker.
fn scan_glossary(source: &str, lines: &[Range<usize>], base_indent: usize, layout: &mut Layout) {
    let mut fences: Vec<FenceMarker> = Vec::new();
    let mut literal: Option<FenceMarker> = None;
    let mut in_options = false;
    // The paragraph or list item being accumulated: span so far and its type.
    let mut block: Option<(Range<usize>, BlockType)> = None;

    for range in lines {
        layout.blank(range.clone());
        let text = line_text(source, range);
        let trimmed = text.trim_start();
        let indent = text.len() - trimmed.len();
        let content_start = range.start + indent;
        let content_end = range.start + text.trim_end().len();

        if let Some(marker) = &literal {
            if closes(trimmed, marker) {
                literal = None;
            }
            continue;
        }
        if trimmed.is_empty() {
            flush_glossary_block(&mut block, layout);
            in_options = false;
            continue;
        }
        if in_options && option_line(trimmed).is_some() {
            continue;
        }
        in_options = false;

        if let Some(top) = fences.last()
            && closes(trimmed, top)
        {
            flush_glossary_block(&mut block, layout);
            fences.pop();
            continue;
        }
        if let Some(opener) = directive_opener(trimmed) {
            flush_glossary_block(&mut block, layout);
            match body_kind(&opener.name, opener.marker.marker) {
                Body::Prose => {
                    let title =
                        content_start + opener.title.start..content_start + opener.title.end;
                    layout.title(source, title);
                    fences.push(opener.marker);
                    in_options = true;
                }
                Body::Literal | Body::Glossary => literal = Some(opener.marker),
            }
            continue;
        }
        if let Some(marker) = fence_marker(trimmed)
            && marker.marker != b':'
        {
            flush_glossary_block(&mut block, layout);
            literal = Some(marker);
            continue;
        }

        // A term stays verbatim: `{term}` roles across the corpus refer to
        // it by its source text, and Sphinx would not resolve a translation.
        if indent <= base_indent {
            flush_glossary_block(&mut block, layout);
            continue;
        }
        if let Some(marker_len) = list_marker_len(trimmed) {
            flush_glossary_block(&mut block, layout);
            block = Some((content_start + marker_len..content_end, BlockType::ListItem));
            continue;
        }
        match &mut block {
            Some((span, _)) => span.end = content_end,
            None => block = Some((content_start..content_end, BlockType::Paragraph)),
        }
    }
    flush_glossary_block(&mut block, layout);
}

fn flush_glossary_block(block: &mut Option<(Range<usize>, BlockType)>, layout: &mut Layout) {
    if let Some((range, block_type)) = block.take() {
        layout.fields.push(Field { range, block_type });
    }
}

impl DocumentParser for MystParser {
    fn markup(&self) -> Markup {
        Markup::Markdown
    }

    fn parse(&self, source: &str) -> Document {
        let layout = scan(source);
        let shadow = layout.shadow(source);
        parse_with(&shadow, source, false, layout.extras())
    }

    fn reconstruct(&self, document: &Document, translations: &TranslationMap) -> String {
        let translations = resolve_reference_links(document, translations);
        splice_reconstruct(document, &translations)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use yeokja_parser_markdown::MarkdownParser;

    fn sources(doc: &Document) -> Vec<String> {
        doc.translatable_segments()
            .iter()
            .map(|seg| seg.source.clone())
            .collect()
    }

    /// Translate every segment with `f` and reconstruct.
    fn translate_all(source: &str, f: impl Fn(&str) -> String) -> String {
        let parser = MystParser;
        let doc = parser.parse(source);
        let mut translations = TranslationMap::new();
        for seg in doc.translatable_segments() {
            translations.insert(seg.id.clone(), f(&seg.source));
        }
        parser.reconstruct(&doc, &translations)
    }

    fn bracket(s: &str) -> String {
        format!("[{s}]")
    }

    #[test]
    fn front_matter_is_opaque() {
        let source = "---\nmyst:\n  html_meta:\n    \"description lang=en\": \"Official docs.\"\n---\n\n# Welcome\n\nBody text.\n";
        let doc = MystParser.parse(source);
        assert_eq!(sources(&doc), ["Welcome", "Body text."]);
        assert_eq!(
            translate_all(source, bracket),
            "---\nmyst:\n  html_meta:\n    \"description lang=en\": \"Official docs.\"\n---\n\n# [Welcome]\n\n[Body text.]\n"
        );
    }

    #[test]
    fn colon_fence_body_is_prose_and_title_is_a_heading() {
        let source = "Intro.\n\n:::{admonition} Notable uses of the Nix language\n:class: note\n\nFirst line.\nSecond line.\n\nAnother para.\n:::\n\nTail.\n";
        let doc = MystParser.parse(source);
        assert_eq!(
            sources(&doc),
            [
                "Intro.",
                "Notable uses of the Nix language",
                "First line.",
                "Second line.",
                "Another para.",
                "Tail."
            ]
        );
        let title = doc.translatable_segments()[1];
        assert_eq!(title.block_type, BlockType::Heading);
        assert_eq!(
            translate_all(source, bracket),
            "[Intro.]\n\n:::{admonition} [Notable uses of the Nix language]\n:class: note\n\n[First line.] [Second line.]\n\n[Another para.]\n:::\n\n[Tail.]\n"
        );
    }

    #[test]
    fn untitled_colon_fence_offers_only_its_body() {
        let source = ":::{note}\nA note.\n:::\n";
        assert_eq!(sources(&MystParser.parse(source)), ["A note."]);
        assert_eq!(
            translate_all(source, bracket),
            ":::{note}\n[A note.]\n:::\n"
        );
    }

    #[test]
    fn space_before_the_directive_brace_is_allowed() {
        let source = "::: {note}\nA note.\n:::\n\n``` {warning} Careful\nBody.\n```\n";
        let doc = MystParser.parse(source);
        assert_eq!(sources(&doc), ["A note.", "Careful", "Body."]);
        assert_eq!(
            translate_all(source, bracket),
            "::: {note}\n[A note.]\n:::\n\n``` {warning} [Careful]\n[Body.]\n```\n"
        );
    }

    #[test]
    fn nested_colon_fences_with_options() {
        let source = "::::{grid} 2\n:::{grid-item-card} Tutorials\n:link: tutorials\n:link-type: ref\n:text-align: center\n\nSeries of lessons\n:::\n\n:::{grid-item-card} Guides\n:link: guides\n\nGuides to getting things done\n:::\n::::\n\n## What next?\n";
        let doc = MystParser.parse(source);
        assert_eq!(
            sources(&doc),
            [
                "Tutorials",
                "Series of lessons",
                "Guides",
                "Guides to getting things done",
                "What next?"
            ]
        );
        assert_eq!(
            translate_all(source, bracket),
            "::::{grid} 2\n:::{grid-item-card} [Tutorials]\n:link: tutorials\n:link-type: ref\n:text-align: center\n\n[Series of lessons]\n:::\n\n:::{grid-item-card} [Guides]\n:link: guides\n\n[Guides to getting things done]\n:::\n::::\n\n## [What next?]\n"
        );
    }

    #[test]
    fn tab_set_titles_and_bodies() {
        let source = ":::::{tab-set}\n::::{tab-item} Linux\nOn Linux, run:\n\n```shell-session\n$ nix\n```\n::::\n\n::::{tab-item} macOS\n\nOn macOS, run the same.\n::::\n:::::\n";
        let doc = MystParser.parse(source);
        assert_eq!(
            sources(&doc),
            [
                "Linux",
                "On Linux, run:",
                "macOS",
                "On macOS, run the same."
            ]
        );
        assert_eq!(MystParser.reconstruct(&doc, &TranslationMap::new()), source);
        assert_eq!(
            translate_all(source, bracket),
            ":::::{tab-set}\n::::{tab-item} [Linux]\n[On Linux, run:]\n\n```shell-session\n$ nix\n```\n::::\n\n::::{tab-item} [macOS]\n\n[On macOS, run the same.]\n::::\n:::::\n"
        );
    }

    #[test]
    fn indented_colon_fence_inside_list_item() {
        let source = "- Run the program.\n\n  :::{note}\n  `nix-ld` does not work for 32-bit executables.\n  :::\n\n- Next item.\n";
        let doc = MystParser.parse(source);
        assert_eq!(
            sources(&doc),
            [
                "Run the program.",
                "`nix-ld` does not work for 32-bit executables.",
                "Next item."
            ]
        );
        assert_eq!(
            translate_all(source, bracket),
            "- [Run the program.]\n\n  :::{note}\n  [`nix-ld` does not work for 32-bit executables.]\n  :::\n\n- [Next item.]\n"
        );
    }

    #[test]
    fn code_block_directive_stays_literal_but_caption_is_offered() {
        let source = "Create it:\n\n```{code-block} nix\n:caption: hello.nix\n:emphasize-lines: 2\n{ writeShellScriptBin }:\nwriteShellScriptBin \"hello\" \"echo Hello\"\n```\n\nDone.\n";
        let doc = MystParser.parse(source);
        assert_eq!(sources(&doc), ["Create it:", "hello.nix", "Done."]);
        assert_eq!(
            doc.translatable_segments()[1].block_type,
            BlockType::Paragraph
        );
        assert_eq!(
            translate_all(source, bracket),
            "[Create it:]\n\n```{code-block} nix\n:caption: [hello.nix]\n:emphasize-lines: 2\n{ writeShellScriptBin }:\nwriteShellScriptBin \"hello\" \"echo Hello\"\n```\n\n[Done.]\n"
        );
    }

    #[test]
    fn toctree_caption_and_explicit_titles_are_offered() {
        let source = "```{toctree}\n:caption: Recipes\n:maxdepth: 1\n\nadd-binary-cache.md\nAutomatic environments <direnv>\nsharing-dependencies.md\n```\n";
        let doc = MystParser.parse(source);
        assert_eq!(sources(&doc), ["Recipes", "Automatic environments"]);
        assert_eq!(
            translate_all(source, bracket),
            "```{toctree}\n:caption: [Recipes]\n:maxdepth: 1\n\nadd-binary-cache.md\n[Automatic environments] <direnv>\nsharing-dependencies.md\n```\n"
        );
    }

    #[test]
    fn literal_directives_without_captions_offer_nothing() {
        let source = "```{contributors}\n:authors: NobbZ\n:editors: fricklerhandwerk\n```\n\n```{literalinclude} default.nix\n:language: nix\n```\n\nText.\n";
        let doc = MystParser.parse(source);
        assert_eq!(sources(&doc), ["Text."]);
        assert_eq!(
            translate_all(source, |_| "글".to_string()),
            source.replace("Text.", "글")
        );
    }

    #[test]
    fn backtick_prose_directive_body_is_translated() {
        let source = "````{note}\nA note with code:\n\n```nix\n{ }\n```\n````\n\nAfter.\n";
        let doc = MystParser.parse(source);
        assert_eq!(sources(&doc), ["A note with code:", "After."]);
        assert_eq!(
            translate_all(source, bracket),
            "````{note}\n[A note with code:]\n\n```nix\n{ }\n```\n````\n\n[After.]\n"
        );
    }

    #[test]
    fn unknown_backtick_directive_is_literal_and_unknown_colon_directive_is_prose() {
        let source = "```{mystery}\nnot prose\n```\n\n:::{mystery} Title\nprose\n:::\n";
        let doc = MystParser.parse(source);
        assert_eq!(sources(&doc), ["Title", "prose"]);
    }

    #[test]
    fn literal_colon_fence_is_blanked_whole() {
        let source = ":::{code-block} nix\nlet x = 1; in x\n:::\n\nAfter.\n";
        let doc = MystParser.parse(source);
        assert_eq!(sources(&doc), ["After."]);
        assert_eq!(
            translate_all(source, bracket),
            ":::{code-block} nix\nlet x = 1; in x\n:::\n\n[After.]\n"
        );
    }

    #[test]
    fn glossary_terms_definitions_and_nested_fences() {
        let source = "# Glossary\n\n```{glossary}\nNix\n    Build system and package manager.\n\n    Read /nɪks/ (\"Niks\").\n\n    :::{seealso}\n    - [Nix reference manual](./nix-manual.md)\n    - [Nix source code](https://github.com/NixOS/nix)\n    :::\n\nNix language\n    Programming language to declare packages\n    and configurations for {term}`Nix`.\n\n    1. First step.\n       Continued.\n    2. Second step.\n```\n\nAfter.\n";
        let doc = MystParser.parse(source);
        assert_eq!(
            sources(&doc),
            [
                "Glossary",
                "Build system and package manager.",
                "Read /nɪks/ (\"Niks\").",
                "[Nix reference manual](./nix-manual.md)",
                "[Nix source code](https://github.com/NixOS/nix)",
                "Programming language to declare packages and configurations for {term}`Nix`.",
                "First step.",
                "Continued.",
                "Second step.",
                "After."
            ]
        );
        let segments = doc.translatable_segments();
        assert_eq!(segments[1].block_type, BlockType::Paragraph);
        assert_eq!(segments[3].block_type, BlockType::ListItem);
        assert!(segments.iter().all(|seg| seg.source != "Nix"));
        assert_eq!(MystParser.reconstruct(&doc, &TranslationMap::new()), source);
        assert_eq!(
            translate_all(source, bracket),
            "# [Glossary]\n\n```{glossary}\nNix\n    [Build system and package manager.]\n\n    [Read /nɪks/ (\"Niks\").]\n\n    :::{seealso}\n    - [[Nix reference manual](./nix-manual.md)]\n    - [[Nix source code](https://github.com/NixOS/nix)]\n    :::\n\nNix language\n    [Programming language to declare packages and configurations for {term}`Nix`.]\n\n    1. [First step.] [Continued.]\n    2. [Second step.]\n```\n\n[After.]\n"
        );
    }

    #[test]
    fn targets_and_attribute_blocks_are_opaque() {
        let source = "(callpackage-tutorial)=\n# Package parameters\n\nCreate a file:\n\n{lineno-start=1 emphasize-lines=\"2\"}\n```nix\nlet\n```\n\n(other-label)=\n\nMore.\n";
        let doc = MystParser.parse(source);
        assert_eq!(
            sources(&doc),
            ["Package parameters", "Create a file:", "More."]
        );
        assert_eq!(
            translate_all(source, bracket),
            "(callpackage-tutorial)=\n# [Package parameters]\n\n[Create a file:]\n\n{lineno-start=1 emphasize-lines=\"2\"}\n```nix\nlet\n```\n\n(other-label)=\n\n[More.]\n"
        );
    }

    #[test]
    fn roles_survive_inside_segments() {
        let source = "If you're new here, {ref}`install Nix <install-nix>` and read the {term}`Nix language` docs; see {download}`the file <../x.nix>`.\n";
        let doc = MystParser.parse(source);
        assert_eq!(sources(&doc), [source.trim_end()]);
        let out = translate_all(source, |_| {
            "{ref}`Nix 설치 <install-nix>`를 하고 {term}`Nix language` 문서를 읽으세요.".to_string()
        });
        assert_eq!(
            out,
            "{ref}`Nix 설치 <install-nix>`를 하고 {term}`Nix language` 문서를 읽으세요.\n"
        );
    }

    #[test]
    fn reference_links_are_rewritten_to_full_form() {
        let source = ":::{note}\nUse the [`nix` command][] on a [standard structure].\n:::\n\n[`nix` command]: https://a\n[standard structure]: https://b\n";
        let doc = MystParser.parse(source);
        assert_eq!(
            sources(&doc),
            ["Use the [`nix` command][] on a [standard structure]."]
        );
        let out = translate_all(source, |_| {
            "[표준 구조]에서 [`nix` 명령][]을 쓰세요.".to_string()
        });
        assert_eq!(
            out,
            ":::{note}\n[표준 구조][`nix` command]에서 [`nix` 명령][standard structure]을 쓰세요.\n:::\n\n[`nix` command]: https://a\n[standard structure]: https://b\n"
        );
    }

    #[test]
    fn footnote_definitions_are_paragraphs() {
        let source = "Text with a note.[^1]\n\n[^1]: The note itself.\n";
        let doc = MystParser.parse(source);
        assert_eq!(sources(&doc), ["Text with a note.[^1]", "The note itself."]);
        assert_eq!(
            translate_all(source, bracket),
            "[Text with a note.[^1]]\n\n[^1]: [The note itself.]\n"
        );
    }

    #[test]
    fn html_tags_stay_verbatim_while_prose_is_translated() {
        let source = "Before.\n\n<div class=\"x\">\nraw\n</div>\n\nAfter.\n";
        let doc = MystParser.parse(source);
        assert_eq!(sources(&doc), ["Before.", "raw", "After."]);
        assert_eq!(
            translate_all(source, bracket),
            "[Before.]\n\n<div class=\"x\">\n[raw]\n</div>\n\n[After.]\n"
        );
    }

    #[test]
    fn fences_inside_code_blocks_are_not_directives() {
        let source = "```md\n:::{note}\nnot a directive\n:::\n(not-a-target)=\n```\n\nText.\n";
        let doc = MystParser.parse(source);
        assert_eq!(sources(&doc), ["Text."]);
        assert_eq!(
            translate_all(source, |_| "글".to_string()),
            source.replace("Text.", "글")
        );
    }

    #[test]
    fn plain_markdown_parser_is_unchanged() {
        let doc = MarkdownParser.parse(":::{note}\ntext\n:::\n");
        assert_eq!(sources(&doc), [":::{note} text :::"]);
        let doc = MarkdownParser.parse("---\ntitle: x\n---\n\nBody.\n");
        assert_eq!(sources(&doc), ["Body."]);
    }

    #[test]
    fn nix_dev_corpus_round_trips_and_offers_prose() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../projects/nix-dev/upstream/source");
        if !root.is_dir() {
            eprintln!("skipping: {} is not checked out", root.display());
            return;
        }
        let mut files = Vec::new();
        collect_md(&root, &mut files);
        files.sort();
        assert!(!files.is_empty());

        let mut counts = Vec::new();
        for path in &files {
            let source = std::fs::read_to_string(path).unwrap();
            let doc = MystParser.parse(&source);
            let segments = doc.translatable_segments();
            for seg in &segments {
                let text = &seg.source;
                // Outside code spans only: prose may well mention `:::{dropdown}`.
                let outside_code = text.split('`').step_by(2).any(|part| part.contains(":::"));
                assert!(
                    !outside_code,
                    "{}: fence in segment {text:?}",
                    path.display()
                );
                assert!(
                    !text.contains(":caption:"),
                    "{}: caption key in segment {text:?}",
                    path.display()
                );
                assert!(
                    !(text.starts_with('(') && text.ends_with(")=")),
                    "{}: target in segment {text:?}",
                    path.display()
                );
                assert!(
                    !text.trim_start().starts_with("```"),
                    "{}: code fence in segment {text:?}",
                    path.display()
                );
            }
            assert_eq!(
                MystParser.reconstruct(&doc, &TranslationMap::new()),
                source,
                "{} does not round-trip",
                path.display()
            );
            // Every segment translated: markers around each keep the layout.
            let mut translations = TranslationMap::new();
            for seg in &segments {
                translations.insert(seg.id.clone(), format!("[{}]", seg.source));
            }
            let out = MystParser.reconstruct(&doc, &translations);
            assert_eq!(structure(&out), structure(&source), "{}", path.display());
            let rel = path.strip_prefix(&root).unwrap().display().to_string();
            counts.push((rel, segments.len()));
        }
        for (rel, count) in &counts {
            println!("{count:5}  {rel}");
        }
        let count = |name: &str| counts.iter().find(|(rel, _)| rel == name).map(|(_, n)| *n);
        assert!(count("reference/glossary.md").unwrap() >= 15);
        assert!(count("index.md").unwrap() >= 30);
    }

    /// The lines a translation must leave alone: fences, options, targets.
    fn structure(text: &str) -> Vec<&str> {
        text.lines()
            .filter(|line| {
                let trimmed = line.trim_start();
                fence_marker(trimmed).is_some()
                    || option_line(trimmed).is_some_and(|(key, _)| key != "caption")
                    || is_target_line(trimmed)
                    || is_attribute_line(trimmed)
            })
            // A directive title is translated, so compare openers up to `}`.
            .map(|line| line.find('}').map_or(line, |end| &line[..=end]))
            .collect()
    }

    fn collect_md(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
        for entry in std::fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                collect_md(&path, out);
            } else if path.extension().is_some_and(|ext| ext == "md") {
                out.push(path);
            }
        }
    }
}
