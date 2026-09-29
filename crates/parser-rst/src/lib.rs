//! Span-based reStructuredText parser.
//!
//! reStructuredText block structure hangs on indentation and on two-line
//! constructs (a title and its underline, a paragraph and the literal block its
//! trailing `::` announces), so this parser works over a pre-split vector of
//! lines with lookahead rather than a streaming scan. Each translatable block
//! records the byte range (`Block::span`) of its text — excluding list markers,
//! directive lines, field names, and the `::` that introduces a literal block —
//! and segments keep the raw inline markup (`**bold**`, ``` ``literal`` ```,
//! :ref:`target`, `link <url>`_). Reconstruction splices translations into the
//! original source, preserving everything outside the translated spans
//! byte-for-byte; title underlines and overlines are redrawn to the
//! translation's display width, since docutils requires them to cover it.

mod directive_table;
pub mod include;
mod table;

use std::ops::Range;
use yeokja_core::model::*;
use yeokja_core::parser::{DocumentParser, Markup, TranslationMap};
use yeokja_parser_utils::{
    apply_splices, collect_splices, join_segments_with_translations, make_segments,
    normalize_inline_text,
};

pub struct RstParser;

/// Parser for Python Enhancement Proposals.
///
/// A PEP starts with an RFC 2822 metadata preamble.  Most of its values are
/// machine-readable enums, identifiers, dates, URLs, and author names, so the
/// generic RST parser must not offer the preamble as prose.  Only the value of
/// ``Title:`` is reader-facing natural language; the body after the first blank
/// line is ordinary reStructuredText.
pub struct PepParser;

/// Parser for the withdrawn plaintext template in PEP 9.  Its entire body is
/// deliberately wrapped in one RST literal block, although the contents are
/// prose.  Removing only that outer marker in the parse shadow exposes the
/// prose while reconstruction still targets the untouched original source.
pub struct PepPlaintextParser;

/// PEP variant that treats explicitly selected ``code-block:: text`` bodies
/// as reader-facing prose.  Used narrowly for PEP 20, whose Zen aphorisms are
/// marked as a text code block for layout rather than because they are code.
pub struct PepTextBlockParser;

/// Parser for the Mathematics in Lean source format.
///
/// The book keeps reStructuredText prose inside `/- TEXT:` blocks in Lean
/// files. The surrounding Lean program, exercise directives, and quoted code
/// must remain byte-for-byte intact. We therefore mask everything outside
/// those blocks with spaces (while retaining byte offsets and newlines), let
/// the regular RST parser discover the prose spans, and finally reconstruct
/// against the original Lean source.
pub struct MilParser;

const MIL_TEXT_START: &str = "/- TEXT:";
const MIL_TEXT_ENDS: [&str; 5] = [
    "TEXT. -/",
    "EXAMPLES: -/",
    "SOLUTIONS: -/",
    "BOTH: -/",
    "OMIT: -/",
];

fn mask_pep_source(source: &str) -> Result<String, String> {
    let mut masked = vec![b' '; source.len()];
    for (index, byte) in source.as_bytes().iter().enumerate() {
        if matches!(byte, b'\n' | b'\r') {
            masked[index] = *byte;
        }
    }

    let mut offset = 0usize;
    let mut saw_pep = false;
    let mut saw_title = false;
    let mut body_start = None;
    for line in source.split_inclusive('\n') {
        let content = line.trim_end_matches(['\r', '\n']);
        if content.is_empty() {
            body_start = Some(offset + line.len());
            break;
        }
        if offset == 0 {
            saw_pep = content.starts_with("PEP:");
        }
        if let Some(value) = content.strip_prefix("Title:") {
            let leading = value.len() - value.trim_start().len();
            let start = offset + "Title:".len() + leading;
            masked[start..offset + content.len()]
                .copy_from_slice(&source.as_bytes()[start..offset + content.len()]);
            saw_title = true;
        }
        offset += line.len();
    }

    if !saw_pep {
        return Err("PEP source must begin with an RFC 2822 `PEP:` header".to_string());
    }
    if !saw_title {
        return Err("PEP source has no `Title:` header".to_string());
    }
    let body_start = body_start.ok_or_else(|| {
        "PEP metadata preamble is not terminated by a blank line".to_string()
    })?;
    // License notices are legal terms rather than translation prose.  Keep the
    // final Copyright/License section (and anything following it) byte-for-byte
    // so an exact grant or restriction can never be altered by the model.
    let mut license_start = None;
    let mut offset = 0usize;
    let mut lines = source.split_inclusive('\n').peekable();
    while let Some(line) = lines.next() {
        let content = line.trim_end_matches(['\r', '\n']);
        let heading = content.trim().to_ascii_lowercase();
        let is_license_heading = matches!(
            heading.as_str(),
            "copyright"
                | "copyright and license"
                | "copyright/license"
                | "copyright and/or license"
                | "license"
        );
        if is_license_heading
            && let Some(next) = lines.peek()
        {
            let adornment = next.trim();
            if let Some(marker) = adornment.chars().next()
                && adornment.len() >= 3
                && marker.is_ascii_punctuation()
                && adornment.chars().all(|character| character == marker)
            {
                license_start = Some(offset);
            }
        }
        offset += line.len();
    }
    let translatable_end = license_start.unwrap_or(source.len());
    if translatable_end > body_start {
        masked[body_start..translatable_end]
            .copy_from_slice(&source.as_bytes()[body_start..translatable_end]);
    }

    String::from_utf8(masked).map_err(|error| error.to_string())
}

fn split_zen_aphorisms(document: &mut Document) {
    for section in &mut document.sections {
        let mut blocks = Vec::new();
        for block in section.blocks.drain(..) {
            if !block.raw_content.contains("Beautiful is better than ugly.") {
                blocks.push(block);
                continue;
            }
            let span_start = block.span.as_ref().map(|span| span.start).unwrap_or(0);
            let mut offset = 0usize;
            for line in block.raw_content.split_inclusive('\n') {
                let content = line.trim_end_matches(['\r', '\n']);
                let prose = content.trim();
                if !prose.is_empty() {
                    let leading = content.find(prose).unwrap();
                    let start = span_start + offset + leading;
                    let end = start + prose.len();
                    blocks.push(Block {
                        block_type: BlockType::Paragraph,
                        segments: make_segments(prose, BlockType::Paragraph, 0, 0),
                        raw_content: prose.to_string(),
                        heading_level: None,
                        span: Some(start..end),
                        role: BlockRole::None,
                        translatable: true,
                    });
                }
                offset += line.len();
            }
        }
        section.blocks = blocks;
    }
    for (section_index, section) in document.sections.iter_mut().enumerate() {
        for (block_index, block) in section.blocks.iter_mut().enumerate() {
            for (segment_index, segment) in block.segments.iter_mut().enumerate() {
                segment.id = SegmentId::new(section_index, block_index, segment_index);
            }
        }
    }
}

fn mask_mil_source(source: &str) -> Result<String, String> {
    let mut masked = vec![b' '; source.len()];
    for (index, byte) in source.as_bytes().iter().enumerate() {
        if matches!(byte, b'\n' | b'\r') {
            masked[index] = *byte;
        }
    }

    let mut in_text = false;
    let mut saw_text = false;
    let mut offset = 0usize;
    for line in source.split_inclusive('\n') {
        let content = line.trim_end_matches(['\r', '\n']);
        let trimmed = content.trim();
        if !in_text && trimmed.starts_with(MIL_TEXT_START) {
            in_text = true;
            saw_text = true;
        } else if in_text && MIL_TEXT_ENDS.iter().any(|end| trimmed.starts_with(end)) {
            in_text = false;
        } else if in_text {
            masked[offset..offset + line.len()].copy_from_slice(line.as_bytes());
        }
        offset += line.len();
    }

    if in_text {
        return Err("unterminated `/- TEXT:` block in Mathematics in Lean source".to_string());
    }
    if !saw_text {
        return Err("no `/- TEXT:` blocks found in Mathematics in Lean source".to_string());
    }

    // Bytes copied from the source remain valid UTF-8 and every masked byte is
    // ASCII. Non-ASCII bytes outside text blocks became one space per byte,
    // deliberately preserving all offsets used by the span parser.
    String::from_utf8(masked).map_err(|error| error.to_string())
}

/// Directives whose body is data, not prose: code, markup passed through raw,
/// document structure, or references resolved elsewhere. Everything not listed
/// keeps its body in the document's language — admonitions, `function::` and
/// friends describe things in prose — so an unknown directive stays
/// translatable and only its marker line is held back.
const OPAQUE_DIRECTIVES: [&str; 20] = [
    "code",
    "code-block",
    "sourcecode",
    "literalinclude",
    "parsed-literal",
    "doctest",
    "raw",
    "math",
    "toctree",
    "contents",
    "include",
    "highlight",
    "image",
    "figure",
    "csv-table",
    "productionlist",
    "tabularcolumns",
    "graphviz",
    "graph",
    "digraph",
];

const INLINE_PROSE_DIRECTIVES: [&str; 4] = ["note", "seealso", "tip", "impl-detail"];
const TITLED_PROSE_DIRECTIVES: [&str; 5] = ["admonition", "sidebar", "rubric", "topic", "tab"];

/// The punctuation characters docutils accepts as a title adornment.
const ADORNMENT_CHARS: &str = "!\"#$%&'()*+,-./:;<=>?@[\\]^_`{|}~";

struct Line<'a> {
    start: usize,
    /// Line text without its newline.
    content: &'a str,
}

impl Line<'_> {
    fn end(&self) -> usize {
        self.start + self.content.len()
    }

    fn trimmed(&self) -> &str {
        self.content.trim()
    }

    fn is_blank(&self) -> bool {
        self.trimmed().is_empty()
    }

    /// Byte column of the first non-space character.
    fn indent(&self) -> usize {
        self.content.len() - self.content.trim_start().len()
    }
}

/// A translatable run being accumulated: its type, the byte span so far, and
/// the column its text starts at — continuation lines have to match it, since
/// in reStructuredText a change of indent is a change of structure.
struct Run {
    block_type: BlockType,
    span: Range<usize>,
    text_col: usize,
    /// Index of the line the run started on, so an adornment line directly
    /// under a single-line run can claim it as a title.
    start_line: usize,
    last_line: usize,
}

struct ParseState<'a> {
    source: &'a str,
    sections: Vec<Section>,
    section_idx: usize,
    block_idx: usize,
    run: Option<Run>,
    /// Adornment styles in order of first appearance; a style's position gives
    /// the heading level, the way docutils assigns them.
    styles: Vec<(char, bool)>,
    /// Set when a flushed paragraph ended with `::`: the column its text
    /// started at. The next block indented past it is a literal block.
    pending_literal: Option<usize>,
    /// Tables whose geometry parsed, in document order. Reconstruction counts
    /// its anchors the same way, so the index in a cell's role finds them.
    table_idx: usize,
    /// The last list enumerator at each open indentation, shallowest first.
    /// docutils reads an ambiguous `i.` as the letter after `h.` only while
    /// that list goes on, so any other line at or left of a marker's column
    /// forgets it.
    enumerators: Vec<(usize, Enumerator)>,
    /// Inside a `glossary` directive: every line above the definitions'
    /// indentation is a term of its own.
    glossary: Option<Glossary>,
}

#[derive(Clone, Copy)]
struct Glossary {
    directive_indent: usize,
    /// Column of the terms, set by the first body line that is not an option.
    term_indent: Option<usize>,
}

impl ParseState<'_> {
    fn has_body_prose(&self) -> bool {
        self.run.is_some()
            || self.sections.iter().flat_map(|section| &section.blocks).any(|block| {
                block.translatable && block.block_type != BlockType::Heading
            })
    }

    /// Close the current run and emit it as a translatable block. A run whose
    /// text ends with `::` announces a literal block; the marker stays outside
    /// the span so the announcement survives the translation.
    fn flush_run(&mut self) {
        let Some(run) = self.run.take() else { return };
        let mut span = run.span;
        let raw = &self.source[span.clone()];
        if let Some(before) = raw.strip_suffix("::") {
            self.pending_literal = Some(run.text_col);
            span.end = span.start + before.trim_end().len();
        }
        self.push_span_block(run.block_type, span, BlockRole::None);
    }

    /// Emit a translatable block covering `span`, unless it carries no word —
    /// a `::` paragraph reduced to nothing, a decorative line. Emitting no
    /// block leaves the source untouched.
    fn push_span_block(&mut self, block_type: BlockType, span: Range<usize>, role: BlockRole) {
        let raw = &self.source[span.clone()];
        if !raw.chars().any(char::is_alphanumeric) {
            return;
        }
        let normalized = normalize_inline_text(raw);
        let segments = make_segments(&normalized, block_type, self.section_idx, self.block_idx);
        self.push_block(Block {
            block_type,
            segments,
            raw_content: raw.to_string(),
            heading_level: None,
            span: Some(span),
            translatable: block_type.is_translatable(),
            role,
        });
    }

    /// Emit an untranslatable block kept verbatim (code, tables, raw markup).
    fn push_opaque_block(&mut self, block_type: BlockType, range: Range<usize>) {
        self.push_block(Block {
            block_type,
            segments: Vec::new(),
            raw_content: self.source[range].to_string(),
            heading_level: None,
            span: None,
            translatable: false,
            role: BlockRole::None,
        });
    }

    /// Emit a table whose geometry parsed as an anchor block plus one block
    /// per prose cell; a table the geometry cannot lay out stays one opaque
    /// verbatim block, exactly as before cell translation existed.
    ///
    /// The anchor is untranslatable but keeps the table's span, which is what
    /// `table_edits` redraws over; cells carry no span of their own, since a
    /// translated cell moves every border around it.
    fn push_table(&mut self, range: Range<usize>) {
        let raw = &self.source[range.clone()];
        let Some(geometry) = table::parse(raw) else {
            self.push_opaque_block(BlockType::Table, range);
            return;
        };
        let table = self.table_idx;
        self.table_idx += 1;
        self.push_block(Block {
            block_type: BlockType::Table,
            segments: Vec::new(),
            raw_content: raw.to_string(),
            heading_level: None,
            span: Some(range),
            translatable: false,
            role: BlockRole::None,
        });

        let labels: Vec<Option<String>> = match geometry.rows.iter().find(|r| r.label) {
            Some(row) => row
                .cells
                .iter()
                .map(|c| (!c.text.is_empty()).then(|| c.text.clone()))
                .collect(),
            None => Vec::new(),
        };
        for row in &geometry.rows {
            for (column, cell) in row.cells.iter().enumerate() {
                if !table::cell_is_translatable(&cell.text) {
                    continue;
                }
                let segments = make_segments(
                    &cell.text,
                    BlockType::Table,
                    self.section_idx,
                    self.block_idx,
                );
                self.push_block(Block {
                    block_type: BlockType::Table,
                    segments,
                    raw_content: cell.text.clone(),
                    heading_level: None,
                    span: None,
                    translatable: BlockType::Table.is_translatable(),
                    role: BlockRole::TableCell {
                        table,
                        column,
                        label_row: row.label,
                        header: if row.label {
                            None
                        } else {
                            labels.get(column).cloned().flatten()
                        },
                    },
                });
            }
        }
    }

    /// Emit a directive-table anchor and one independently spliced block for
    /// each prose cell. Unlike grid and simple tables, this directive keeps
    /// its markers, indentation, and wrapping verbatim during reconstruction.
    fn push_directive_table(
        &mut self,
        range: Range<usize>,
        parse: fn(&str, Range<usize>) -> Option<directive_table::DirectiveTable>,
    ) {
        let Some(table_data) = parse(self.source, range.clone()) else {
            // Keep opaque directive tables in the shared table order. Their
            // bodies remain wholly verbatim, but a later supported table must
            // not reuse this table's reconstruction index.
            self.table_idx += 1;
            self.push_block(Block {
                block_type: BlockType::Table,
                segments: Vec::new(),
                raw_content: self.source[range.clone()].to_string(),
                heading_level: None,
                span: Some(range),
                translatable: false,
                role: BlockRole::None,
            });
            return;
        };
        let table = self.table_idx;
        self.table_idx += 1;
        self.push_block(Block {
            block_type: BlockType::Table,
            segments: Vec::new(),
            raw_content: self.source[range.clone()].to_string(),
            heading_level: None,
            span: Some(range),
            translatable: false,
            role: BlockRole::None,
        });

        if let Some(title) = table_data.title {
            debug_assert_eq!(&self.source[title.span.clone()], title.text);
            self.push_span_block(BlockType::Paragraph, title.span, BlockRole::None);
        }
        for cell in table_data.cells {
            if cell.text.is_empty() {
                continue;
            }
            let segments = make_segments(
                &cell.text,
                BlockType::Table,
                self.section_idx,
                self.block_idx,
            );
            self.push_block(Block {
                block_type: BlockType::Table,
                segments,
                raw_content: cell.text,
                heading_level: None,
                span: Some(cell.span),
                translatable: BlockType::Table.is_translatable(),
                role: BlockRole::TableCell {
                    table,
                    column: cell.column,
                    label_row: cell.label_row,
                    header: cell.header,
                },
            });
        }
    }

    fn push_list_table(&mut self, range: Range<usize>) {
        self.push_directive_table(range, directive_table::parse_list);
    }

    fn push_csv_table(&mut self, range: Range<usize>) {
        self.push_directive_table(range, directive_table::parse_csv);
    }

    fn push_block(&mut self, block: Block) {
        self.sections.last_mut().unwrap().blocks.push(block);
        self.block_idx += 1;
    }

    /// Heading level of an adornment style: its position among the styles seen,
    /// the way docutils assigns levels.
    fn style_level(&mut self, marker: char, has_overline: bool) -> u8 {
        let key = (marker, has_overline);
        let pos = match self.styles.iter().position(|s| *s == key) {
            Some(pos) => pos,
            None => {
                self.styles.push(key);
                self.styles.len() - 1
            }
        };
        (pos + 1).min(u8::MAX as usize) as u8
    }

    fn start_section_if_needed(&mut self, level: u8) {
        if level <= 2 && !self.sections.last().unwrap().blocks.is_empty() {
            self.sections.push(Section { blocks: Vec::new() });
            self.section_idx += 1;
            self.block_idx = 0;
        }
    }

    fn push_title(&mut self, span: Range<usize>, level: u8, role: BlockRole) {
        self.start_section_if_needed(level);
        let raw = &self.source[span.clone()];
        let segments = make_segments(
            &normalize_inline_text(raw),
            BlockType::Heading,
            self.section_idx,
            self.block_idx,
        );
        self.push_block(Block {
            block_type: BlockType::Heading,
            segments,
            raw_content: raw.to_string(),
            heading_level: Some(level),
            span: Some(span),
            translatable: BlockType::Heading.is_translatable(),
            role,
        });
    }
}

/// The adornment character of a line made of one punctuation character
/// repeated, at least twice. `..` is explicit markup, never an adornment.
fn adornment_char(trimmed: &str) -> Option<char> {
    let first = trimmed.chars().next()?;
    (ADORNMENT_CHARS.contains(first)
        && trimmed.len() >= 2
        && trimmed != ".."
        && trimmed.chars().all(|c| c == first))
    .then_some(first)
}

/// Does an adornment of this length hold under `title`? Docutils warns on a
/// short underline but reads the title anyway once the line is unambiguous, so
/// length gates only the short lines that are likelier prose or a delimiter.
fn adornment_covers(adornment: &str, title: &str) -> bool {
    let len = adornment.chars().count();
    len >= 4 || len >= title.trim().chars().count()
}

/// The adornment character of a line that can underline a one-line title:
/// docutils also takes a single punctuation character there (`R` over `-`),
/// though never as an overline or a transition.
fn underline_char(trimmed: &str) -> Option<char> {
    adornment_char(trimmed).or_else(|| {
        let mut chars = trimmed.chars();
        let first = chars.next()?;
        (chars.next().is_none() && ADORNMENT_CHARS.contains(first)).then_some(first)
    })
}

/// `- item`, `* item`, `+ item` → byte offset of the item text.
fn parse_bullet_item(content: &str) -> Option<usize> {
    let indent = content.len() - content.trim_start().len();
    let rest = &content[indent..];
    let first = rest.chars().next()?;
    if !matches!(first, '-' | '*' | '+') {
        return None;
    }
    let after = &rest[first.len_utf8()..];
    let ws = after.len() - after.trim_start().len();
    if ws == 0 || after.trim().is_empty() {
        return None;
    }
    Some(indent + first.len_utf8() + ws)
}

/// `1. item`, `#. item`, `a) item`, `(iv) item` → byte offset of the item
/// text, without docutils' second-line check (see `reads_as_block_start`).
fn parse_enumerated_item(content: &str) -> Option<usize> {
    let found = enumerator(content)?;
    (found.ordinal.is_some() && found.text_offset < content.trim_end().len())
        .then_some(found.text_offset)
}

/// The sequence an enumerator counts in; `#` is docutils' auto-enumerator.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum EnumKind {
    Arabic,
    Auto,
    LowerAlpha,
    UpperAlpha,
    LowerRoman,
    UpperRoman,
}

/// How an enumerator is punctuated: `1.`, `1)`, or `(1)`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum EnumFormat {
    Period,
    RParen,
    Parens,
}

/// A list enumerator at the start of a line, read the way docutils does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Enumerator {
    kind: EnumKind,
    format: EnumFormat,
    /// `None` for text that looks like an enumerator but counts nothing
    /// (`iiii.`); docutils reads such a line as text.
    ordinal: Option<u32>,
    /// Byte offset in the line where the item text starts, after all the
    /// spaces that follow the marker (the line's length when none follows).
    text_offset: usize,
}

/// The enumerator a line opens with, with no list in progress.
fn enumerator(content: &str) -> Option<Enumerator> {
    enumerator_after(content, None)
}

/// The enumerator a line opens with. `previous` is the item before it in
/// the same list: docutils tries that list's sequence first, so `i.` after
/// `h.` is the ninth letter while `i.` alone is the Roman one.
fn enumerator_after(content: &str, previous: Option<&Enumerator>) -> Option<Enumerator> {
    let indent = content.len() - content.trim_start().len();
    let rest = &content[indent..];
    let (open_paren, body) = match rest.strip_prefix('(') {
        Some(body) => (true, body),
        None => (false, rest),
    };
    let token_len = if body.starts_with('#') {
        1
    } else {
        body.chars().take_while(char::is_ascii_alphanumeric).count()
    };
    if token_len == 0 {
        return None;
    }
    let token = &body[..token_len];
    let after_token = &body[token_len..];
    let format = match (open_paren, after_token.chars().next()?) {
        (true, ')') => EnumFormat::Parens,
        (false, ')') => EnumFormat::RParen,
        (false, '.') => EnumFormat::Period,
        _ => return None,
    };
    let after = &after_token[1..];
    if !(after.is_empty() || after.starts_with([' ', '\t'])) {
        return None;
    }
    let spaces = after.len() - after.trim_start().len();
    let text_offset = indent + usize::from(open_paren) + token_len + 1 + spaces;

    if token == "#" {
        return Some(Enumerator { kind: EnumKind::Auto, format, ordinal: Some(1), text_offset });
    }
    let candidates: Vec<EnumKind> = [
        EnumKind::Arabic,
        EnumKind::LowerAlpha,
        EnumKind::UpperAlpha,
        EnumKind::LowerRoman,
        EnumKind::UpperRoman,
    ]
    .into_iter()
    .filter(|kind| sequence_matches(*kind, token))
    .collect();
    let expected = previous.and_then(|previous| {
        let kind = previous.kind;
        let continues = kind != EnumKind::Auto
            && previous.format == format
            && candidates.contains(&kind)
            && previous.ordinal.is_some_and(|ordinal| {
                ordinal_of(kind, token) == Some(ordinal + 1)
            });
        continues.then_some(kind)
    });
    let kind = match expected {
        Some(kind) => kind,
        None if token == "i" => EnumKind::LowerRoman,
        None if token == "I" => EnumKind::UpperRoman,
        None => *candidates.first()?,
    };
    Some(Enumerator { kind, format, ordinal: ordinal_of(kind, token), text_offset })
}

fn sequence_matches(kind: EnumKind, token: &str) -> bool {
    match kind {
        EnumKind::Arabic => token.bytes().all(|b| b.is_ascii_digit()),
        EnumKind::Auto => token == "#",
        EnumKind::LowerAlpha => token.len() == 1 && token.bytes().all(|b| b.is_ascii_lowercase()),
        EnumKind::UpperAlpha => token.len() == 1 && token.bytes().all(|b| b.is_ascii_uppercase()),
        EnumKind::LowerRoman => token.bytes().all(|b| b"ivxlcdm".contains(&b)),
        EnumKind::UpperRoman => token.bytes().all(|b| b"IVXLCDM".contains(&b)),
    }
}

fn ordinal_of(kind: EnumKind, token: &str) -> Option<u32> {
    match kind {
        EnumKind::Arabic => token.parse().ok(),
        EnumKind::Auto => Some(1),
        EnumKind::LowerAlpha | EnumKind::UpperAlpha => {
            Some(u32::from(token.as_bytes()[0].to_ascii_lowercase() - b'a') + 1)
        }
        EnumKind::LowerRoman | EnumKind::UpperRoman => from_roman(token),
    }
}

/// A Roman numeral in its canonical spelling, as docutils' `roman` module
/// accepts it (1 to 4999); `iiii` or `ic` counts nothing.
fn from_roman(token: &str) -> Option<u32> {
    let upper = token.to_ascii_uppercase();
    let mut value = 0;
    let mut chars = upper.chars().peekable();
    while let Some(c) = chars.next() {
        let digit = roman_digit(c)?;
        match chars.peek().and_then(|next| roman_digit(*next)) {
            Some(next) if next > digit => {
                value += next - digit;
                chars.next();
            }
            _ => value += digit,
        }
    }
    (to_roman(value)? == upper).then_some(value)
}

fn roman_digit(c: char) -> Option<u32> {
    Some(match c {
        'I' => 1,
        'V' => 5,
        'X' => 10,
        'L' => 50,
        'C' => 100,
        'D' => 500,
        'M' => 1000,
        _ => return None,
    })
}

fn to_roman(mut value: u32) -> Option<String> {
    if !(1..5000).contains(&value) {
        return None;
    }
    const TABLE: [(u32, &str); 13] = [
        (1000, "M"),
        (900, "CM"),
        (500, "D"),
        (400, "CD"),
        (100, "C"),
        (90, "XC"),
        (50, "L"),
        (40, "XL"),
        (10, "X"),
        (9, "IX"),
        (5, "V"),
        (4, "IV"),
        (1, "I"),
    ];
    let mut out = String::new();
    for (step, numeral) in TABLE {
        while value >= step {
            out.push_str(numeral);
            value -= step;
        }
    }
    Some(out)
}

/// The markers docutils accepts for the item after `found`: the next
/// enumerator in its sequence and the auto-enumerator, each in the same
/// punctuation and followed by a space.
fn next_enumerators(found: &Enumerator) -> Vec<String> {
    let (prefix, suffix) = match found.format {
        EnumFormat::Period => ("", "."),
        EnumFormat::RParen => ("", ")"),
        EnumFormat::Parens => ("(", ")"),
    };
    let mut markers = vec![format!("{prefix}#{suffix} ")];
    let Some(next) = found.ordinal.map(|ordinal| ordinal + 1) else {
        return markers;
    };
    let token = match found.kind {
        EnumKind::Auto => None,
        EnumKind::Arabic => Some(next.to_string()),
        EnumKind::LowerAlpha => (next <= 26).then(|| char::from(b'a' + next as u8 - 1).to_string()),
        EnumKind::UpperAlpha => (next <= 26).then(|| char::from(b'A' + next as u8 - 1).to_string()),
        EnumKind::LowerRoman => to_roman(next).map(|roman| roman.to_ascii_lowercase()),
        EnumKind::UpperRoman => to_roman(next),
    };
    if let Some(token) = token {
        markers.push(format!("{prefix}{token}{suffix} "));
    }
    markers
}

/// Would docutils open a list, field list, option list, line block, or
/// doctest block at `first_line`? Both lines carry their indentation.
///
/// An enumerator opens a list only when the line after it is missing, blank,
/// indented past it (or back out of the enclosing block), or starts the next
/// item — otherwise ("A. Einstein was ...") docutils reads a paragraph. An
/// option marker with no description on its own line needs an indented
/// description below it. `previous` is the item before an enumerator in the
/// same list, which settles whether `i.` is a letter or a Roman numeral.
pub(crate) fn reads_as_block_start(
    first_line: &str,
    next_line: Option<&str>,
    previous: Option<&Enumerator>,
) -> bool {
    let indent = first_line.len() - first_line.trim_start().len();
    let rest = first_line[indent..].trim_end();
    if rest.is_empty() {
        return false;
    }
    let next_indented = next_line.is_some_and(|next| {
        !next.trim().is_empty() && next.len() - next.trim_start().len() > indent
    });

    let mut chars = rest.chars();
    let first = chars.next().unwrap();
    let after_first = chars.as_str();
    if matches!(first, '-' | '+' | '*' | '•' | '‣' | '⁃')
        && (after_first.is_empty() || after_first.starts_with(' '))
    {
        return true;
    }
    if let Some(found) = enumerator_after(first_line, previous) {
        if found.ordinal.is_none() {
            return false;
        }
        let Some(next) = next_line else { return true };
        let next_indent = next.len() - next.trim_start().len();
        if next.trim().is_empty() || next_indent != indent {
            return true;
        }
        let next_rest = &next[indent..];
        return next_enumerators(&found)
            .iter()
            .any(|marker| next_rest.starts_with(marker.as_str()));
    }
    if docutils_field_marker(rest) {
        return true;
    }
    if let Some(description) = option_marker_description(rest) {
        return !description.trim().is_empty() || next_indented;
    }
    ["|", ">>>"].iter().any(|marker| {
        rest.strip_prefix(marker)
            .is_some_and(|after| after.is_empty() || after.starts_with(' '))
    })
}

/// docutils' field marker: `:name:` followed by a space or the line's end,
/// where the name neither starts with a space or `:` nor ends with a space,
/// and holds a `:` only when no space or backtick follows it.
fn docutils_field_marker(rest: &str) -> bool {
    let Some(body) = rest.strip_prefix(':') else { return false };
    if body.starts_with([':', ' ']) || body.is_empty() {
        return false;
    }
    let bytes = body.as_bytes();
    let mut at = 0;
    while at < bytes.len() {
        match bytes[at] {
            b'\\' => at += 2,
            b':' => {
                let after = &body[at + 1..];
                if after.is_empty() || after.starts_with(' ') {
                    return bytes[at - 1] != b' ';
                }
                if after.starts_with('`') {
                    return false;
                }
                at += 1;
            }
            _ => at += 1,
        }
    }
    false
}

/// docutils' option marker (`-a`, `-o FILE`, `--long=VALUE`, `/V`, joined by
/// `, `) followed by two spaces or the line's end → the text after it.
fn option_marker_description(rest: &str) -> Option<&str> {
    fn optname_len(s: &str) -> usize {
        let bytes = s.as_bytes();
        if !bytes.first().is_some_and(u8::is_ascii_alphanumeric) {
            return 0;
        }
        1 + bytes[1..]
            .iter()
            .take_while(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-'))
            .count()
    }
    fn optarg_len(s: &str) -> Option<usize> {
        if let Some(inner) = s.strip_prefix('<') {
            let close = inner.find(['<', '>'])?;
            return (inner.as_bytes()[close] == b'>' && close > 0).then_some(close + 2);
        }
        s.starts_with(|c: char| c.is_ascii_alphabetic()).then(|| optname_len(s))
    }
    // Every length one option can take, longest first, like the regex's
    // backtracking.
    fn option_lengths(s: &str) -> Vec<usize> {
        let long = s
            .strip_prefix("--")
            .or_else(|| s.strip_prefix('/'))
            .map(|name| (s.len() - name.len(), optname_len(name)));
        let (head, arg_separators): (usize, &[&str]) = if let Some((prefix, name)) = long {
            if name == 0 {
                return Vec::new();
            }
            (prefix + name, &[" ", "="])
        } else if s.starts_with(['-', '+'])
            && s.as_bytes().get(1).is_some_and(u8::is_ascii_alphanumeric)
        {
            (2, &[" ", ""])
        } else {
            return Vec::new();
        };
        let mut lengths = Vec::new();
        for separator in arg_separators {
            if let Some(arg) = s[head..].strip_prefix(separator)
                && let Some(arg_len) = optarg_len(arg)
            {
                lengths.push(head + separator.len() + arg_len);
            }
        }
        lengths.push(head);
        lengths
    }
    fn group_end(s: &str) -> Option<usize> {
        for len in option_lengths(s) {
            let after = &s[len..];
            if let Some(more) = after.strip_prefix(", ")
                && let Some(end) = group_end(more)
            {
                return Some(len + 2 + end);
            }
            if after.starts_with("  ") || after.is_empty() || after == " " {
                return Some(len);
            }
        }
        None
    }
    let end = group_end(rest)?;
    Some(&rest[end..])
}

/// `:name: value` field marker → whether this line opens a field. The marker's
/// closing colon must be followed by a space or end the line; a backtick there
/// means an inline role (`:ref:`target``), which is prose.
fn field_marker(trimmed: &str) -> Option<(&str, &str)> {
    let rest = trimmed.strip_prefix(':')?;
    let end = rest.find(':')?;
    let name = &rest[..end];
    if name.is_empty() || name.contains(char::is_whitespace) {
        return None;
    }
    let after = &rest[end + 1..];
    (after.is_empty() || after.starts_with(' ')).then_some((name, after))
}

fn is_field_marker(trimmed: &str) -> bool {
    field_marker(trimmed).is_some()
}

/// Byte offset of reader-facing prose after a standalone field marker.
/// Indented fields are directive options, while standard bibliographic fields
/// are document metadata; both remain verbatim.
fn standalone_field_body_start(content: &str, has_body_prose: bool) -> Option<usize> {
    if content.starts_with(char::is_whitespace) {
        return None;
    }
    let (name, after) = field_marker(content)?;
    let lower = name.to_ascii_lowercase();
    if matches!(
        lower.as_str(),
        "author"
            | "authors"
            | "organization"
            | "address"
            | "contact"
            | "version"
            | "revision"
            | "status"
            | "date"
            | "copyright"
            | "orphan"
            | "tocdepth"
            | "nocomments"
            | "nosearch"
    ) {
        return None;
    }
    if !has_body_prose && !matches!(lower.as_str(), "abstract" | "dedication") {
        return None;
    }
    let leading = after.len() - after.trim_start().len();
    (!after.trim().is_empty()).then_some(content.len() - after.len() + leading)
}

/// `.. name:: arguments` → the directive's name.
///
/// Follows docutils' directive-name grammar so that Sphinx domain directives
/// (`py:function`, `c:function`) are recognized: the first character is
/// alphanumeric, later characters are alphanumeric or `-_.+:`, and the name
/// does not end in `:` (which would make it a hyperlink target or comment
/// instead, e.g. `.. _target: http://x`).
fn parse_directive(trimmed: &str) -> Option<&str> {
    let rest = trimmed.strip_prefix("..")?.strip_prefix(char::is_whitespace)?;
    let end = rest.find("::")?;
    let name = &rest[..end];
    let mut chars = name.chars();
    let first_ok = chars.next().is_some_and(|c| c.is_ascii_alphanumeric());
    let rest_ok = chars.all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.' | '+' | ':'));
    let last_ok = !name.ends_with(':');
    (first_ok && rest_ok && last_ok).then_some(name)
}

fn parse_substitution_image_directive(trimmed: &str) -> Option<&str> {
    let rest = trimmed.strip_prefix("..")?.strip_prefix(char::is_whitespace)?;
    let after_open = rest.strip_prefix('|')?;
    let name_end = after_open.find("| ")?;
    let directive = &after_open[name_end + 2..];
    let marker_end = directive.find("::")?;
    let name = &directive[..marker_end];
    name.eq_ignore_ascii_case("image").then_some(name)
}

fn directive_argument_metadata(
    line: &Line,
    directive: &str,
) -> Option<(Range<usize>, BlockType)> {
    let lower = directive.to_ascii_lowercase();
    let block_type = if INLINE_PROSE_DIRECTIVES.contains(&lower.as_str()) {
        BlockType::Paragraph
    } else if TITLED_PROSE_DIRECTIVES.contains(&lower.as_str()) {
        BlockType::Heading
    } else {
        return None;
    };
    let indent = line.indent();
    let content = &line.content[indent..];
    let marker_end = content.find("::")? + 2;
    let after = &content[marker_end..];
    let leading = after.len() - after.trim_start().len();
    let value = after[leading..].trim_end();
    if value.is_empty() {
        return None;
    }
    let start = line.start + indent + marker_end + leading;
    Some((start..start + value.len(), block_type))
}

fn directive_option_metadata(
    lines: &[Line],
    directive_at: usize,
    directive_end: usize,
    directive: &str,
) -> Vec<Range<usize>> {
    let lower = directive.to_ascii_lowercase();
    let option = match lower.as_str() {
        "image" | "figure" => "alt",
        "code" | "code-block" => "caption",
        _ => return Vec::new(),
    };
    let directive_indent = lines[directive_at].indent();
    let option_indent = directive_indent + 3;
    let marker = format!(":{option}:");
    let mut spans = Vec::new();
    let mut at = directive_at + 1;
    while at <= directive_end {
        let line = &lines[at];
        if line.indent() != option_indent {
            at += 1;
            continue;
        }
        let Some(after_marker) = line.content[option_indent..].strip_prefix(&marker) else {
            at += 1;
            continue;
        };
        let leading = after_marker.len() - after_marker.trim_start().len();
        let value = after_marker[leading..].trim_end();
        if value.is_empty() || (option == "caption" && !value.contains(char::is_whitespace)) {
            at += 1;
            continue;
        }
        let start = line.start + option_indent + marker.len() + leading;
        let mut end = start + value.len();
        let mut continuation = at + 1;
        while continuation <= directive_end
            && !lines[continuation].is_blank()
            && lines[continuation].indent() > option_indent
        {
            end = lines[continuation].end();
            continuation += 1;
        }
        spans.push(start..end);
        at = continuation;
    }
    spans
}

/// `.. [1] text`, `.. [#label] text`, `.. [CIT2002] text` → byte offset of the
/// footnote or citation text within the line.
fn parse_footnote(content: &str) -> Option<usize> {
    let indent = content.len() - content.trim_start().len();
    let rest = &content[indent..];
    let after_dots = rest.strip_prefix("..")?.strip_prefix(' ')?;
    let inner = after_dots.strip_prefix('[')?;
    let close = inner.find(']')?;
    if inner[..close].is_empty() || inner[..close].contains(char::is_whitespace) {
        return None;
    }
    let after = &inner[close + 1..];
    let ws = after.len() - after.trim_start().len();
    if ws == 0 || after.trim().is_empty() {
        return None;
    }
    Some(indent + 3 + 1 + close + 1 + ws)
}

/// A grid table border: `+---+---+` or `+===+`.
fn is_grid_border(trimmed: &str) -> bool {
    trimmed.len() >= 2
        && trimmed.starts_with('+')
        && trimmed.ends_with('+')
        && trimmed.chars().all(|c| matches!(c, '+' | '-' | '='))
        && trimmed.chars().any(|c| c == '-' || c == '=')
}

/// A simple table border: runs of `=` separated by spaces, at least two runs.
fn is_simple_table_border(trimmed: &str) -> bool {
    trimmed.chars().all(|c| c == '=' || c == ' ')
        && trimmed.split_whitespace().count() >= 2
        && trimmed.split_whitespace().all(|run| run.chars().all(|c| c == '='))
}

/// Display width the way docutils counts it for title adornments: East Asian
/// Wide and Fullwidth characters occupy two columns.
fn display_width(text: &str) -> usize {
    text.chars().map(|c| if is_wide(c) { 2 } else { 1 }).sum()
}

/// Unicode East Asian Width W or F, over the ranges that occur in practice.
fn is_wide(c: char) -> bool {
    matches!(c as u32,
        0x1100..=0x115F           // Hangul Jamo
        | 0x2E80..=0x303E         // CJK Radicals .. CJK Symbols and Punctuation
        | 0x3041..=0x33FF         // Hiragana .. CJK Compatibility
        | 0x3400..=0x4DBF         // CJK Extension A
        | 0x4E00..=0x9FFF         // CJK Unified Ideographs
        | 0xA000..=0xA4CF         // Yi
        | 0xAC00..=0xD7A3         // Hangul Syllables
        | 0xF900..=0xFAFF         // CJK Compatibility Ideographs
        | 0xFE30..=0xFE4F         // CJK Compatibility Forms
        | 0xFF00..=0xFF60         // Fullwidth Forms
        | 0xFFE0..=0xFFE6
        | 0x20000..=0x2FFFD
        | 0x30000..=0x3FFFD)
}

impl DocumentParser for RstParser {
    fn markup(&self) -> Markup {
        Markup::Rst
    }

    fn parse(&self, source: &str) -> Document {
        let lines: Vec<Line> = {
            let mut offset = 0;
            source
                .split_inclusive('\n')
                .map(|raw| {
                    let start = offset;
                    offset += raw.len();
                    let content = raw.strip_suffix('\n').unwrap_or(raw);
                    let content = content.strip_suffix('\r').unwrap_or(content);
                    Line { start, content }
                })
                .collect()
        };

        let mut state = ParseState {
            source,
            sections: vec![Section { blocks: Vec::new() }],
            section_idx: 0,
            block_idx: 0,
            run: None,
            styles: Vec::new(),
            pending_literal: None,
            table_idx: 0,
            enumerators: Vec::new(),
            glossary: None,
        };

        let mut i = 0;
        while i < lines.len() {
            let line = &lines[i];
            let trimmed = line.trimmed();

            if line.is_blank() {
                state.flush_run();
                i += 1;
                continue;
            }

            // The list item before this line at its column, if the list is
            // still open; this line closes every list at or right of it.
            let previous_enumerator = state
                .enumerators
                .iter()
                .find(|(indent, _)| *indent == line.indent())
                .map(|(_, found)| *found);
            state.enumerators.retain(|(indent, _)| *indent < line.indent());

            // A literal block announced by `::` claims everything indented
            // past the announcing paragraph, before any other reading: inside
            // it, a directive is code and a bullet is a bullet character.
            if let Some(threshold) = state.pending_literal {
                if line.indent() > threshold {
                    let end = consume_while_indented(&lines, i, threshold);
                    state.push_opaque_block(
                        BlockType::CodeBlock,
                        line.start..lines[end].end(),
                    );
                    state.pending_literal = None;
                    i = end + 1;
                    continue;
                }
                state.pending_literal = None;
            }

            // In a glossary, each line left of the definitions is one term;
            // Sphinx reads a group of such lines as several terms of the
            // same definition. Options before the first term and comments
            // keep their ordinary reading.
            if let Some(glossary) = state.glossary {
                if line.indent() <= glossary.directive_indent {
                    state.glossary = None;
                } else if !(glossary.term_indent.is_none() && is_field_marker(trimmed))
                    && !trimmed.starts_with("..")
                {
                    let term_indent = glossary.term_indent.unwrap_or(line.indent());
                    state.glossary = Some(Glossary {
                        term_indent: Some(term_indent),
                        ..glossary
                    });
                    if line.indent() <= term_indent {
                        state.flush_run();
                        state.push_span_block(
                            BlockType::Heading,
                            line.start + line.indent()..line.end(),
                            BlockRole::None,
                        );
                        i += 1;
                        continue;
                    }
                }
            }

            // `::` alone is the literal-block paragraph, or the continuation
            // of one — never a transition, though the colon is an adornment
            // character.
            if trimmed == "::" {
                match &mut state.run {
                    Some(run) if run.text_col == line.indent() => {
                        run.span.end = line.end();
                        run.last_line = i;
                    }
                    _ => {
                        state.flush_run();
                        state.pending_literal = Some(line.indent());
                    }
                }
                i += 1;
                continue;
            }

            // An adornment line under a single-line run is that run's
            // underline; the pair is a section title. A single character
            // underlines only at the title's own column.
            if let Some(run) = &state.run
                && run.start_line == run.last_line
                && run.last_line + 1 == i
                && run.block_type == BlockType::Paragraph
                && let Some(marker) = underline_char(trimmed)
                && (trimmed.len() > 1 || line.indent() == run.text_col)
                && adornment_covers(trimmed, &source[run.span.clone()])
            {
                let run = state.run.take().unwrap();
                let level = state.style_level(marker, false);
                state.pending_literal = None;
                state.push_title(
                    run.span,
                    level,
                    BlockRole::AdornedTitle {
                        underline: line.start..line.end(),
                        overline: None,
                    },
                );
                i += 1;
                continue;
            }

            // An adornment line with a title and a matching adornment right
            // below opens an overlined title; anything else all-punctuation is
            // a transition, kept verbatim.
            if let Some(marker) = adornment_char(trimmed) {
                state.flush_run();
                if let (Some(title), Some(under)) = (lines.get(i + 1), lines.get(i + 2))
                    && !title.is_blank()
                    && adornment_char(title.trimmed()).is_none()
                    && under.trimmed() == trimmed
                    && adornment_covers(trimmed, title.trimmed())
                {
                    let level = state.style_level(marker, true);
                    let text_start = title.start + title.indent();
                    state.push_title(
                        text_start..title.start + title.indent() + title.trimmed().len(),
                        level,
                        BlockRole::AdornedTitle {
                            underline: under.start..under.end(),
                            overline: Some(line.start..line.end()),
                        },
                    );
                    i += 3;
                    continue;
                }
                i += 1;
                continue;
            }

            // Explicit markup: directives, targets, substitutions, footnotes,
            // comments. All begin with `..` — alone, or followed by a space.
            if trimmed == ".."
                || (trimmed.starts_with("..") && trimmed[2..].starts_with(char::is_whitespace))
            {
                state.flush_run();
                if let Some(name) = parse_directive(trimmed)
                    .or_else(|| parse_substitution_image_directive(trimmed))
                {
                    if name.eq_ignore_ascii_case("list-table") {
                        let end = consume_while_indented(&lines, i, line.indent());
                        state.push_list_table(line.start..lines[end].end());
                        i = end + 1;
                    } else if name.eq_ignore_ascii_case("csv-table") {
                        let end = consume_while_indented(&lines, i, line.indent());
                        state.push_csv_table(line.start..lines[end].end());
                        i = end + 1;
                    } else if name.eq_ignore_ascii_case("glossary") {
                        state.glossary = Some(Glossary {
                            directive_indent: line.indent(),
                            term_indent: None,
                        });
                        i += 1;
                    } else if OPAQUE_DIRECTIVES.contains(&name.to_ascii_lowercase().as_str()) {
                        let end = consume_while_indented(&lines, i, line.indent());
                        state.push_opaque_block(
                            BlockType::CodeBlock,
                            line.start..lines[end].end(),
                        );
                        for span in directive_option_metadata(&lines, i, end, name) {
                            state.push_span_block(
                                BlockType::Paragraph,
                                span,
                                BlockRole::None,
                            );
                        }
                        i = end + 1;
                    } else {
                        // The marker line stays verbatim; the indented body
                        // parses as ordinary content.
                        if let Some((span, block_type)) =
                            directive_argument_metadata(line, name)
                        {
                            state.push_span_block(block_type, span, BlockRole::None);
                        }
                        i += 1;
                    }
                    continue;
                }
                if let Some(text_offset) = parse_footnote(line.content) {
                    // The first paragraph of the body goes on over the lines
                    // indented under the marker, at the column the second
                    // line sets. Without one it ends on this line.
                    let continuation = lines
                        .get(i + 1)
                        .filter(|next| !next.is_blank() && next.indent() > line.indent())
                        .map(Line::indent);
                    state.run = Some(Run {
                        block_type: BlockType::Paragraph,
                        span: line.start + text_offset..line.end(),
                        text_col: continuation.unwrap_or(text_offset),
                        start_line: i,
                        last_line: i,
                    });
                    if continuation.is_none() {
                        state.flush_run();
                    }
                    i += 1;
                    continue;
                }
                // Target, substitution definition, or comment: verbatim, along
                // with its indented continuation.
                let end = consume_while_indented(&lines, i, line.indent());
                i = end + 1;
                continue;
            }

            // Doctest block: opaque until the next blank line.
            if state.run.is_none() && trimmed.starts_with(">>>") {
                let mut end = i;
                while end + 1 < lines.len() && !lines[end + 1].is_blank() {
                    end += 1;
                }
                state.push_opaque_block(BlockType::CodeBlock, line.start..lines[end].end());
                i = end + 1;
                continue;
            }

            // Grid table: consecutive `+--+` / `| … |` lines. Cells whose
            // geometry parses are offered for translation; the rest of the
            // table stays verbatim until reconstruction redraws it.
            if state.run.is_none() && is_grid_border(trimmed) {
                let mut end = i;
                while end + 1 < lines.len() {
                    let t = lines[end + 1].trimmed();
                    if t.starts_with('+') || t.starts_with('|') {
                        end += 1;
                    } else {
                        break;
                    }
                }
                state.push_table(line.start..lines[end].end());
                i = end + 1;
                continue;
            }

            // Simple table: from one `==  ==` border to the one a blank line
            // (or the file's end) follows.
            if state.run.is_none() && is_simple_table_border(trimmed) {
                let mut end = None;
                for j in i + 1..lines.len() {
                    if is_simple_table_border(lines[j].trimmed())
                        && lines.get(j + 1).is_none_or(|l| l.is_blank())
                    {
                        end = Some(j);
                        break;
                    }
                }
                if let Some(end) = end {
                    state.push_table(line.start..lines[end].end());
                    i = end + 1;
                    continue;
                }
                // No closing border: not a table after all; fall through.
            }

            // A standalone field keeps its marker exact while exposing its
            // reader-facing body. Continuations belong to the same body span.
            if let Some(body_start) =
                standalone_field_body_start(line.content, state.has_body_prose())
            {
                state.flush_run();
                let mut end = i;
                while end + 1 < lines.len()
                    && !lines[end + 1].is_blank()
                    && lines[end + 1].indent() > line.indent()
                {
                    end += 1;
                }
                state.push_span_block(
                    BlockType::Paragraph,
                    line.start + body_start..lines[end].end(),
                    BlockRole::None,
                );
                i = end + 1;
                continue;
            }

            // Directive options, document metadata, empty fields, and line
            // blocks are structure rather than reader-facing prose.
            if is_field_marker(trimmed) || trimmed == "|" || trimmed.starts_with("| ") {
                state.flush_run();
                i += 1;
                continue;
            }

            // A paragraph runs to the next blank line, so a line that goes on
            // at its column is text even when it opens like a list item — a
            // wrapped inline literal can leave `B)` or `-` there.
            let continues_run = state
                .run
                .as_ref()
                .is_some_and(|run| run.text_col == line.indent());
            let list_item = if continues_run {
                None
            } else if let Some(text_offset) = parse_bullet_item(line.content) {
                Some(text_offset)
            } else if let Some(text_offset) = parse_enumerated_item(line.content)
                && let Some(found) = enumerator_after(line.content, previous_enumerator.as_ref())
            {
                // docutils reads an enumerator line with an adornment below
                // as a title; it has long been kept a list item here (its
                // underline is redrawn in `adornment_edits`), and staying so
                // keeps its segment IDs.
                let next = lines.get(i + 1).map(|next| next.content);
                let over_adornment =
                    next.is_some_and(|next| adornment_char(next.trim()).is_some());
                (over_adornment
                    || reads_as_block_start(line.content, next, previous_enumerator.as_ref()))
                .then(|| {
                    state.enumerators.push((line.indent(), found));
                    text_offset
                })
            } else {
                None
            };
            if let Some(text_offset) = list_item {
                state.flush_run();
                state.run = Some(Run {
                    block_type: BlockType::ListItem,
                    span: line.start + text_offset..line.end(),
                    text_col: text_offset,
                    start_line: i,
                    last_line: i,
                });
                i += 1;
                continue;
            }

            // Plain text: continue the run when the indent holds, break the
            // block when it does not — in reStructuredText a change of indent
            // is a change of structure (a definition, a quote), and joining
            // across it would splice that structure away.
            let col = line.indent();
            match &mut state.run {
                Some(run) if run.text_col == col => {
                    run.span.end = line.end();
                    run.last_line = i;
                }
                _ => {
                    state.flush_run();
                    state.run = Some(Run {
                        block_type: BlockType::Paragraph,
                        span: line.start + col..line.end(),
                        text_col: col,
                        start_line: i,
                        last_line: i,
                    });
                }
            }
            i += 1;
        }
        state.flush_run();

        let mut sections = state.sections;
        sections.retain(|s| !s.blocks.is_empty());

        Document {
            sections,
            source: source.to_string(),
        }
    }

    fn reconstruct(&self, document: &Document, translations: &TranslationMap) -> String {
        reconstruct_rst(document, translations)
    }
}

impl DocumentParser for MilParser {
    fn parse(&self, source: &str) -> Document {
        self.parse_checked(source).unwrap_or_else(|_| Document {
            sections: Vec::new(),
            source: source.to_string(),
        })
    }

    fn parse_checked(
        &self,
        source: &str,
    ) -> Result<Document, yeokja_core::parser::DocumentParseError> {
        let masked = mask_mil_source(source).map_err(yeokja_core::parser::DocumentParseError)?;
        let mut document = RstParser.parse(&masked);
        document.source = source.to_string();
        Ok(document)
    }

    fn reconstruct(&self, document: &Document, translations: &TranslationMap) -> String {
        RstParser.reconstruct(document, translations)
    }

    fn markup(&self) -> Markup {
        Markup::Rst
    }
}

impl DocumentParser for PepParser {
    fn parse(&self, source: &str) -> Document {
        self.parse_checked(source).unwrap_or_else(|_| Document {
            sections: Vec::new(),
            source: source.to_string(),
        })
    }

    fn parse_checked(
        &self,
        source: &str,
    ) -> Result<Document, yeokja_core::parser::DocumentParseError> {
        let masked = mask_pep_source(source).map_err(yeokja_core::parser::DocumentParseError)?;
        let mut document = RstParser.parse(&masked);
        document.source = source.to_string();
        Ok(document)
    }

    fn reconstruct(&self, document: &Document, translations: &TranslationMap) -> String {
        reconstruct_rst(document, translations)
    }

    fn markup(&self) -> Markup {
        Markup::Rst
    }
}

impl DocumentParser for PepPlaintextParser {
    fn parse(&self, source: &str) -> Document {
        self.parse_checked(source).unwrap_or_else(|_| Document {
            sections: Vec::new(),
            source: source.to_string(),
        })
    }

    fn parse_checked(
        &self,
        source: &str,
    ) -> Result<Document, yeokja_core::parser::DocumentParseError> {
        let mut masked = mask_pep_source(source).map_err(yeokja_core::parser::DocumentParseError)?;
        let marker = masked.find("\n::\n").ok_or_else(|| {
            yeokja_core::parser::DocumentParseError(
                "plaintext PEP has no outer `::` literal marker".to_string(),
            )
        })?;
        masked.replace_range(marker + 1..marker + 3, "  ");
        let mut document = RstParser.parse(&masked);
        document.source = source.to_string();
        Ok(document)
    }

    fn reconstruct(&self, document: &Document, translations: &TranslationMap) -> String {
        reconstruct_rst(document, translations)
    }

    fn markup(&self) -> Markup {
        Markup::Rst
    }
}

impl DocumentParser for PepTextBlockParser {
    fn parse(&self, source: &str) -> Document {
        self.parse_checked(source).unwrap_or_else(|_| Document {
            sections: Vec::new(),
            source: source.to_string(),
        })
    }

    fn parse_checked(
        &self,
        source: &str,
    ) -> Result<Document, yeokja_core::parser::DocumentParseError> {
        let mut masked = mask_pep_source(source).map_err(yeokja_core::parser::DocumentParseError)?;
        let mut offset = 0usize;
        let mut found = false;
        for line in masked.clone().split_inclusive('\n') {
            let content = line.trim_end_matches(['\r', '\n']);
            if content.trim() == ".. code-block:: text" {
                masked.replace_range(offset..offset + content.len(), &" ".repeat(content.len()));
                found = true;
            }
            offset += line.len();
        }
        if !found {
            return Err(yeokja_core::parser::DocumentParseError(
                "selected PEP has no `code-block:: text` prose block".to_string(),
            ));
        }
        let mut document = RstParser.parse(&masked);
        split_zen_aphorisms(&mut document);
        document.source = source.to_string();
        Ok(document)
    }

    fn reconstruct(&self, document: &Document, translations: &TranslationMap) -> String {
        reconstruct_rst(document, translations)
    }

    fn markup(&self) -> Markup {
        Markup::Rst
    }
}

/// Reconstruct RST while retaining the implicit target names of translated
/// section titles that are referenced elsewhere in the document. In RST,
/// `` `Future Directions`_ `` resolves to the heading text itself; translating
/// only the heading silently changes that target to its Korean spelling.
fn reconstruct_rst(document: &Document, translations: &TranslationMap) -> String {
    // Zero-width insertions must sort before the title replacement at the same
    // byte position, or `apply_splices` will correctly discard them as overlap.
    let mut splices = embedded_link_target_edits(document, translations);
    let (heading_edits, heading_aliases) = heading_anchor_edits(document, translations);
    splices.extend(heading_edits);
    // Anonymous hyperlink targets (``__ URL``) are RST structure, even though
    // the legacy segmentation represents consecutive target lines as a prose
    // block. Never splice their normalized segment text back into the source:
    // doing so joins adjacent targets and changes their count. Filtering only
    // at reconstruction keeps existing segment IDs and translation state
    // stable while preserving the original target lines byte-for-byte.
    let mut prose_translations = translations.clone();
    rewrite_heading_references(&mut prose_translations, &heading_aliases);
    let list_table_edits = directive_table_edits(document, translations);
    for block in document.sections.iter().flat_map(|section| &section.blocks) {
        if matches!(block.role, BlockRole::TableCell { .. }) && block.span.is_some() {
            for segment in &block.segments {
                prose_translations.remove(&segment.id);
            }
        }
        if block
            .raw_content
            .lines()
            .all(|line| matches!(line.trim(), "__") || line.trim().starts_with("__ "))
        {
            for segment in &block.segments {
                prose_translations.remove(&segment.id);
            }
        }
    }
    splices.extend(collect_splices(document, &prose_translations));
    splices.extend(list_table_edits);
    splices.extend(adornment_edits(document, translations));
    splices.extend(table_edits(document, translations));
    let mut output = apply_splices(&document.source, splices);
    rewrite_heading_references_in_text(&mut output, &heading_aliases);
    repair_korean_reference_boundaries(&output)
}

/// Keep an RST reference recognizable when a Korean particle follows it.
///
/// The per-segment format repair deliberately avoids guessing when an embedded
/// link starts in one parser segment and closes in another. Once the complete
/// document has been reconstructed, however, the closing backtick and its
/// reference suffix are unambiguous. A backslash-escaped space is invisible in
/// rendered output and gives docutils the boundary it requires.
fn repair_korean_reference_boundaries(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut insertions = std::collections::BTreeSet::new();

    for at in 0..chars.len() {
        let suffix_end = if chars[at] == '`' && chars.get(at + 1) == Some(&'_') {
            if chars.get(at + 2) == Some(&'_') {
                at + 3
            } else {
                at + 2
            }
        } else if chars[at] == ']' && chars.get(at + 1) == Some(&'_') {
            at + 2
        } else {
            continue;
        };
        if chars.get(suffix_end).is_some_and(|ch| ('\u{ac00}'..='\u{d7a3}').contains(ch)) {
            insertions.insert(suffix_end);
        }
    }

    if insertions.is_empty() {
        return text.to_string();
    }
    let mut repaired = String::with_capacity(text.len() + insertions.len() * 2);
    for boundary in 0..=chars.len() {
        if insertions.contains(&boundary) {
            repaired.push_str("\\ ");
        }
        if let Some(ch) = chars.get(boundary) {
            repaired.push(*ch);
        }
    }
    repaired
}

/// An embedded RST link also defines a reusable target named by its visible
/// label. If that label is translated, an earlier `` `label`_ `` reference
/// breaks even though the URL itself survives. Preserve an explicit original
/// label-to-URL target near the start of the document.
fn embedded_link_target_edits(
    document: &Document,
    translations: &TranslationMap,
) -> Vec<(Range<usize>, String)> {
    let mut targets: Vec<(String, String)> = Vec::new();
    for (label, target, link_start) in embedded_links(&document.source) {
        let link_is_translated = document
            .sections
            .iter()
            .flat_map(|section| &section.blocks)
            .any(|block| {
                block.span.as_ref().is_some_and(|span| span.contains(&link_start))
                    && block
                        .segments
                        .iter()
                        .any(|segment| translations.contains_key(&segment.id))
            });
        if !link_is_translated {
            continue;
        }
        let quoted_reference = format!("`{label}`_");
        if !document.source.contains(&quoted_reference) {
            continue;
        }
        let plain_target = format!(".. _{label}:");
        let quoted_target = format!(".. _`{label}`:");
        if document
            .source
            .lines()
            .any(|line| matches!(line.trim(), existing if existing == plain_target || existing.starts_with(&quoted_target)))
            || targets.iter().any(|(existing, _)| existing == &label)
        {
            continue;
        }
        targets.push((label, target));
    }
    if targets.is_empty() {
        return Vec::new();
    }

    let insertion = if document.source.starts_with("PEP:") {
        document
            .source
            .find("\n\n")
            .map(|start| start + 2)
            .unwrap_or(0)
    } else {
        0
    };
    let directives = targets
        .into_iter()
        .map(|(label, target)| format!(".. _`{label}`: {target}\n"))
        .collect::<String>()
        + "\n";
    vec![(insertion..insertion, directives)]
}

fn embedded_links(source: &str) -> Vec<(String, String, usize)> {
    let mut links = Vec::new();
    let mut offset = 0usize;
    while let Some(open) = source[offset..].find('`') {
        let content_start = offset + open + 1;
        let Some(close) = source[content_start..].find('`') else { break };
        let content_end = content_start + close;
        let content = &source[content_start..content_end];
        if source[content_end + 1..].starts_with('_')
            && let Some(target_start) = content.rfind(" <")
            && let Some(target) = content[target_start + 2..].strip_suffix('>')
        {
            let label = content[..target_start]
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" ");
            if !label.is_empty() && !label.contains('`') && !target.ends_with('_') {
                links.push((label, target.to_string(), offset + open));
            }
        }
        offset = content_end + 1;
    }
    links
}

/// Source edits (range, replacement) and target-name aliases (from, to).
type AnchorEdits = (Vec<(Range<usize>, String)>, Vec<(String, String)>);

fn heading_anchor_edits(document: &Document, translations: &TranslationMap) -> AnchorEdits {
    let mut edits = Vec::new();
    let mut aliases = Vec::new();
    let namespace = document_target_namespace(&document.source);
    for block in document.sections.iter().flat_map(|section| &section.blocks) {
        if !matches!(block.role, BlockRole::AdornedTitle { .. })
            || !block
                .segments
                .iter()
                .any(|segment| translations.contains_key(&segment.id))
        {
            continue;
        }
        let original = block.raw_content.trim();
        if original.is_empty() || original.contains(['\n', '\r']) {
            continue;
        }
        let translated_heading = join_segments_with_translations(&block.segments, translations);
        if translated_heading.trim() == normalize_inline_text(original).trim() {
            continue;
        }
        // Inline literals do not form part of an implicit section target's
        // name: ````ImageSize`` Class`` is referenced as
        // `` `ImageSize class`_ ``. Support this common, unambiguous markup
        // while still declining headings with single-backtick links or roles.
        let target_name = if original.contains("``") {
            strip_emphasis_markup(&original.replace("``", ""))
        } else if original.contains('`') {
            continue;
        } else {
            strip_emphasis_markup(original)
        };
        let Some(span) = &block.span else { continue };
        let unique_target = format!("yeokja-{namespace}-target-{}", span.start);
        // The heading is referenced exactly when rewriting its references
        // changes the source. Reusing the rewriter keeps detection and
        // rewriting in agreement: a title that only occurs as a substring of
        // another reference (``<allowed patterns_>`` for the heading
        // ``Patterns``) is not a reference to it and gets no label.
        let mut probe = document.source.clone();
        rewrite_heading_references_in_text(
            &mut probe,
            &[(target_name.clone(), unique_target.clone())],
        );
        if probe == document.source {
            continue;
        }
        let plain_target = format!(".. _{target_name}:");
        let quoted_target = format!(".. _`{target_name}`:");
        if document
            .source
            .lines()
            .any(|line| {
                let target = line.trim();
                target == plain_target
                    || target == quoted_target
                    || target.starts_with(&format!("{plain_target} "))
                    || target.starts_with(&format!("{quoted_target} "))
            })
        {
            continue;
        }
        let insertion_start = match &block.role {
            BlockRole::AdornedTitle {
                overline: Some(overline),
                ..
            } => document.source[..overline.start]
                .rfind('\n')
                .map_or(0, |at| at + 1),
            _ => span.start,
        };
        edits.push((
            insertion_start..insertion_start,
            format!(".. _{unique_target}:\n\n"),
        ));
        aliases.push((target_name, unique_target));
    }
    (edits, aliases)
}

fn strip_emphasis_markup(text: &str) -> String {
    if text.matches('*').count() >= 2 {
        text.replace('*', "")
    } else {
        text.to_string()
    }
}

fn document_target_namespace(source: &str) -> String {
    if let Some(number) = source
        .lines()
        .next()
        .and_then(|line| line.strip_prefix("PEP:"))
        .map(str::trim)
        .filter(|number| number.chars().all(|character| character.is_ascii_digit()))
    {
        return format!("pep-{number:0>4}");
    }

    let hash = source.as_bytes().iter().fold(0xcbf29ce484222325u64, |hash, byte| {
        (hash ^ u64::from(*byte)).wrapping_mul(0x100000001b3)
    });
    format!("doc-{hash:016x}")
}

fn rewrite_heading_references(
    translations: &mut TranslationMap,
    aliases: &[(String, String)],
) {
    for translation in translations.values_mut() {
        rewrite_heading_references_in_text(translation, aliases);
    }
}

fn rewrite_heading_references_in_text(text: &mut String, aliases: &[(String, String)]) {
    for (target_name, unique_target) in aliases {
        // Hyperlink target bodies first: docutils does not parse an embedded
        // alias there, so they need the bare ``label_`` form rather than the
        // ``` `Title <label_>`_ ``` form used in running text. Rewriting them
        // first also keeps the passes below off those lines.
        *text = rewrite_indirect_targets(text, target_name, unique_target);

        *text = replace_embedded_alias(text, target_name, unique_target);

        let sphinx = format!(":ref:`{target_name}`");
        *text = replace_ascii_case_insensitive(
            text,
            &sphinx,
            &format!(":ref:`{target_name} <{unique_target}>`"),
        );

        let quoted_replacement = format!("`{target_name} <{unique_target}_>`_");
        *text = replace_flexible_quoted_reference(text, target_name, &quoted_replacement);

        if !target_name.contains(char::is_whitespace) {
            *text = replace_bare_rst_reference(
                text,
                &format!("{target_name}_"),
                &format!("`{target_name} <{unique_target}_>`_"),
            );
        }
    }
}

/// docutils' reference-name normalization: case-insensitive with whitespace
/// runs collapsed to a single space.
fn fold_reference_name(name: &str) -> String {
    name.to_ascii_lowercase()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// The body of an explicit hyperlink target line (``.. __: body``,
/// ``.. _name: body`` or ``.. _`name`: body``), as a byte range of `line`.
fn hyperlink_target_body(line: &str) -> Option<Range<usize>> {
    let indent = line.len() - line.trim_start().len();
    let rest = line[indent..].strip_prefix(".. _")?;
    let name_start = indent + 4;
    let body_offset = if rest.starts_with("_:") {
        2
    } else if let Some(quoted) = rest.strip_prefix('`') {
        1 + quoted.find("`:")? + 2
    } else {
        // A plain name ends at the first colon that is not escaped or part
        // of the name (``.. _a:b: url`` is unusual enough to skip).
        let colon = rest.find(':')?;
        let name = &rest[..colon];
        if name.is_empty() || name.starts_with('_') || name.contains('`') {
            return None;
        }
        colon + 1
    };
    let body_start = name_start + body_offset;
    if body_start < line.len() && !line[body_start..].starts_with(char::is_whitespace) {
        return None;
    }
    Some(body_start..line.len())
}

/// Rewrite ``.. __: `Title`_`` (or ``Title_``) to ``.. __: label_`` when
/// `Title` is the heading named by `target_name`.
fn rewrite_indirect_targets(input: &str, target_name: &str, unique_target: &str) -> String {
    let folded_target = fold_reference_name(target_name);
    let mut output = String::with_capacity(input.len());
    for line in input.split_inclusive('\n') {
        let content = line.trim_end_matches(['\n', '\r']);
        let line_ending = &line[content.len()..];
        let Some(body_range) = hyperlink_target_body(content) else {
            output.push_str(line);
            continue;
        };
        let body = content[body_range.clone()].trim();
        let reference_name = body
            .strip_suffix('_')
            .filter(|name| !name.ends_with('_'))
            .map(|name| {
                name.strip_prefix('`')
                    .and_then(|quoted| quoted.strip_suffix('`'))
                    .unwrap_or(name)
            });
        match reference_name {
            Some(name) if fold_reference_name(name) == folded_target => {
                output.push_str(&content[..body_range.start]);
                output.push(' ');
                output.push_str(unique_target);
                output.push('_');
                output.push_str(line_ending);
            }
            _ => output.push_str(line),
        }
    }
    output
}

/// Rewrite the alias of an embedded-alias reference,
/// ``` `text <Title_>`_ ```, when the whole alias names the heading.
/// Only the alias is a candidate: ``<allowed patterns_>`` is not a reference
/// to a heading called ``Patterns``.
fn replace_embedded_alias(input: &str, target_name: &str, unique_target: &str) -> String {
    let folded_target = fold_reference_name(target_name);
    let mut output = String::with_capacity(input.len());
    let mut offset = 0usize;
    while let Some(open) = input[offset..].find('<') {
        let start = offset + open;
        let content_start = start + 1;
        let Some(close) = input[content_start..].find('>') else {
            break;
        };
        let content_end = content_start + close;
        let content = &input[content_start..content_end];
        let is_heading_alias = input[content_end + 1..].starts_with('`')
            && !content.contains(['<', '`'])
            && content
                .strip_suffix('_')
                .is_some_and(|alias| fold_reference_name(alias) == folded_target);
        if is_heading_alias {
            output.push_str(&input[offset..start]);
            output.push('<');
            output.push_str(unique_target);
            output.push_str("_>");
            offset = content_end + 1;
        } else {
            output.push_str(&input[offset..content_start]);
            offset = content_start;
        }
    }
    output.push_str(&input[offset..]);
    output
}

fn replace_ascii_case_insensitive(input: &str, needle: &str, replacement: &str) -> String {
    if needle.is_empty() {
        return input.to_string();
    }
    let folded_input = input.to_ascii_lowercase();
    let folded_needle = needle.to_ascii_lowercase();
    let mut output = String::with_capacity(input.len());
    let mut offset = 0usize;
    while let Some(found) = folded_input[offset..].find(&folded_needle) {
        let start = offset + found;
        output.push_str(&input[offset..start]);
        output.push_str(replacement);
        offset = start + needle.len();
    }
    output.push_str(&input[offset..]);
    output
}

fn replace_flexible_quoted_reference(input: &str, target_name: &str, replacement: &str) -> String {
    let folded_target = fold_reference_name(target_name);
    let mut output = String::with_capacity(input.len());
    let mut offset = 0usize;
    while let Some(open) = input[offset..].find('`') {
        let start = offset + open;
        let content_start = start + 1;
        let Some(close) = input[content_start..].find("`_") else {
            break;
        };
        let content_end = content_start + close;
        if fold_reference_name(&input[content_start..content_end]) == folded_target {
            output.push_str(&input[offset..start]);
            output.push_str(replacement);
            offset = content_end + 2;
        } else {
            output.push_str(&input[offset..content_start]);
            offset = content_start;
        }
    }
    output.push_str(&input[offset..]);
    output
}

/// Byte ranges of backtick-delimited inline markup: inline literals,
/// interpreted text, roles and phrase references including the ``<alias_>``
/// part of an embedded reference. A bare ``name_`` inside one of these is
/// not a standalone reference.
fn backtick_spans(text: &str) -> Vec<Range<usize>> {
    let mut spans = Vec::new();
    let mut offset = 0usize;
    while let Some(open) = text[offset..].find('`') {
        let start = offset + open;
        let (delimiter, content_start) = if text[start..].starts_with("``") {
            ("``", start + 2)
        } else {
            ("`", start + 1)
        };
        let Some(close) = text[content_start..].find(delimiter) else {
            break;
        };
        let end = content_start + close + delimiter.len();
        spans.push(start..end);
        offset = end;
    }
    spans
}

fn replace_bare_rst_reference(input: &str, reference: &str, replacement: &str) -> String {
    let folded_input = input.to_ascii_lowercase();
    let folded_reference = reference.to_ascii_lowercase();
    let protected = backtick_spans(input);
    let mut output = String::with_capacity(input.len());
    let mut offset = 0usize;
    while let Some(found) = folded_input[offset..].find(&folded_reference) {
        let start = offset + found;
        let end = start + reference.len();
        let before = input[..start].chars().next_back();
        let after = input[end..].chars().next();
        let is_name_character = |character: char| {
            character.is_alphanumeric() || matches!(character, '_' | '-')
        };
        let inside_markup = protected
            .iter()
            .any(|span| span.start < end && start < span.end);
        if !inside_markup
            && before.is_none_or(|character| !is_name_character(character))
            && after.is_none_or(|character| !is_name_character(character))
        {
            output.push_str(&input[offset..start]);
            output.push_str(replacement);
            offset = end;
        } else {
            output.push_str(&input[offset..end]);
            offset = end;
        }
    }
    output.push_str(&input[offset..]);
    output
}

/// The last line of the indented block starting under line `at`: lines more
/// indented than `threshold` belong to it, and blank lines do not end it —
/// only a dedent does.
fn consume_while_indented(lines: &[Line], at: usize, threshold: usize) -> usize {
    let mut last = at;
    let mut j = at + 1;
    while j < lines.len() {
        if lines[j].is_blank() {
            j += 1;
            continue;
        }
        if lines[j].indent() > threshold {
            last = j;
            j += 1;
        } else {
            break;
        }
    }
    last
}

/// Redraw the adornment lines of each translated title. Docutils requires an
/// underline (and overline) to cover the title's display width, where a Hangul
/// syllable is two columns; a translation almost never keeps the original
/// width, so leaving the lines verbatim would demote the title or fail the
/// build.
fn adornment_edits(
    document: &Document,
    translations: &TranslationMap,
) -> Vec<(Range<usize>, String)> {
    let mut edits = Vec::new();
    for block in document.sections.iter().flat_map(|s| s.blocks.iter()) {
        let (underline, overline) = match &block.role {
            BlockRole::AdornedTitle { underline, overline } => {
                (underline.clone(), overline.clone())
            }
            _ if block.block_type == BlockType::ListItem => {
                let Some(underline) = enumerated_title_underline(document, block) else {
                    continue;
                };
                (underline, None)
            }
            _ => continue,
        };
        if !block
            .segments
            .iter()
            .any(|seg| translations.contains_key(&seg.id))
        {
            continue;
        }
        let Some(marker) = document.source[underline.clone()].chars().next() else {
            continue;
        };
        let translated_width = display_width(&join_segments_with_translations(
            &block.segments,
            translations,
        ));
        let fixed_whitespace_width = block.span.as_ref().map_or(0, |span| {
            let line_start = document.source[..span.start]
                .rfind('\n')
                .map_or(0, |at| at + 1);
            let line_end = document.source[span.end..]
                .find('\n')
                .map_or(document.source.len(), |at| span.end + at);
            display_width(&document.source[line_start..span.start])
                + display_width(&document.source[span.end..line_end])
        });
        let width = translated_width + fixed_whitespace_width;
        let drawn = marker.to_string().repeat(width.max(4));
        edits.push((underline, drawn.clone()));
        if let Some(over) = overline {
            edits.push((over, drawn));
        }
    }
    edits
}

/// A title such as ``6. METH_FASTCALL is private`` initially looks like an
/// enumerated list item. Keep that legacy block classification (and therefore
/// stable translation IDs), but recognize its following adornment while
/// reconstructing so a wider translation receives a sufficiently long line.
fn enumerated_title_underline(document: &Document, block: &Block) -> Option<Range<usize>> {
    let span = block.span.as_ref()?;
    let line_start = document.source[..span.start]
        .rfind('\n')
        .map_or(0, |at| at + 1);
    let line_end = document.source[span.end..]
        .find('\n')
        .map(|at| span.end + at)?;
    let line = &document.source[line_start..line_end];
    if parse_enumerated_item(line)? != span.start - line_start {
        return None;
    }

    let next_start = line_end + 1;
    let next_end = document.source[next_start..]
        .find('\n')
        .map_or(document.source.len(), |at| next_start + at);
    let next = &document.source[next_start..next_end];
    let trimmed = next.trim();
    adornment_char(trimmed)?;
    if !adornment_covers(trimmed, line) {
        return None;
    }
    let leading = next.len() - next.trim_start().len();
    Some(next_start + leading..next_start + leading + trimmed.len())
}

/// Redraw each table that has a translated cell. The anchor block keeps the
/// table's span; its geometry is re-read from the source (the same `parse`
/// that emitted the cell blocks, so both enumerate cells identically), the
/// translated texts are laid into it, and the whole span is replaced —
/// borders, padding, and wrapping follow the cells' display widths.
fn table_edits(
    document: &Document,
    translations: &TranslationMap,
) -> Vec<(Range<usize>, String)> {
    let blocks: Vec<&Block> = document
        .sections
        .iter()
        .flat_map(|s| s.blocks.iter())
        .collect();

    let mut edits = Vec::new();
    let mut table_idx = 0usize;
    for block in &blocks {
        if block.block_type != BlockType::Table || block.translatable || block.span.is_none() {
            continue;
        }
        let Some(span) = block.span.clone() else { continue };
        let index = table_idx;
        table_idx += 1;

        let cells: Vec<&&Block> = blocks
            .iter()
            .filter(|b| matches!(&b.role, BlockRole::TableCell { table, .. } if *table == index))
            .collect();
        if !cells.iter().any(|b| {
            b.segments
                .iter()
                .any(|seg| translations.contains_key(&seg.id))
        }) {
            continue;
        }
        let Some(geometry) = table::parse(&document.source[span.clone()]) else {
            continue;
        };

        // Pair geometry cells with their blocks in the shared row-major order.
        let mut remaining = cells.iter();
        let mut texts: Vec<Vec<Option<String>>> = Vec::with_capacity(geometry.rows.len());
        let mut aligned = true;
        for row in &geometry.rows {
            let mut row_texts = Vec::with_capacity(row.cells.len());
            for cell in &row.cells {
                if !table::cell_is_translatable(&cell.text) {
                    row_texts.push(None);
                    continue;
                }
                match remaining.next() {
                    Some(b) => row_texts
                        .push(Some(join_segments_with_translations(&b.segments, translations))),
                    None => aligned = false,
                }
            }
            texts.push(row_texts);
        }
        if !aligned || remaining.next().is_some() {
            continue; // cells and geometry disagree; keep the table verbatim
        }
        edits.push((span, table::render(&geometry, &texts)));
    }
    edits
}

/// Replace only translated directive-table cell spans. The directive itself is
/// never redrawn: its markers, field list, indentation, and unrelated cells
/// remain byte-for-byte as authored.
fn directive_table_edits(
    document: &Document,
    translations: &TranslationMap,
) -> Vec<(Range<usize>, String)> {
    let blocks: Vec<&Block> = document
        .sections
        .iter()
        .flat_map(|section| section.blocks.iter())
        .collect();

    let mut edits = Vec::new();
    let mut table_idx = 0usize;
    for block in &blocks {
        if block.block_type != BlockType::Table || block.translatable || block.span.is_none() {
            continue;
        }
        let Some(span) = block.span.clone() else {
            continue;
        };
        let index = table_idx;
        table_idx += 1;
        let Some(table_data) = directive_table::parse_list(&document.source, span.clone())
            .or_else(|| directive_table::parse_csv(&document.source, span))
        else {
            continue;
        };
        let parsed_cells: Vec<_> = table_data
            .cells
            .into_iter()
            .filter(|cell| !cell.text.is_empty())
            .collect();
        let cell_blocks: Vec<_> = blocks
            .iter()
            .filter(|candidate| {
                matches!(&candidate.role, BlockRole::TableCell { table, .. } if *table == index)
                    && candidate.span.is_some()
            })
            .collect();
        if parsed_cells.len() != cell_blocks.len() {
            continue;
        }
        for (parsed, cell) in parsed_cells.into_iter().zip(cell_blocks) {
            if cell
                .segments
                .iter()
                .any(|segment| translations.contains_key(&segment.id))
            {
                let translated = join_segments_with_translations(&cell.segments, translations);
                edits.push((
                    parsed.span,
                    if parsed.csv_quoted {
                        translated.replace('"', "\"\"")
                    } else {
                        translated
                    },
                ));
            }
        }
    }
    edits
}

#[cfg(test)]
mod tests {
    use super::*;

    fn translate_all(parser: &RstParser, source: &str, pairs: &[(&str, &str)]) -> String {
        let doc = parser.parse(source);
        let mut translations = TranslationMap::new();
        for seg in doc.all_segments() {
            if let Some((_, t)) = pairs.iter().find(|(s, _)| *s == seg.source) {
                translations.insert(seg.id.clone(), t.to_string());
            }
        }
        parser.reconstruct(&doc, &translations)
    }

    #[test]
    fn mil_parser_translates_only_text_blocks() {
        let source = "import MIL.Common\n/- TEXT:\nGetting Started\n===============\n\nRead this.\nTEXT. -/\nexample (alpha : Type) : alpha = alpha := rfl\n";
        let document = MilParser.parse_checked(source).unwrap();
        let segments = document.translatable_segments();
        assert_eq!(
            segments
                .iter()
                .map(|segment| segment.source.as_str())
                .collect::<Vec<_>>(),
            vec!["Getting Started", "Read this."]
        );
        assert!(
            !segments
                .iter()
                .any(|segment| segment.source.contains("example"))
        );
    }

    #[test]
    fn mil_parser_reconstruction_preserves_lean_and_unicode_bytes() {
        let source = "import MIL.Common\n/- TEXT:\nGetting Started\n===============\n\nRead this.\nBOTH: -/\nexample (alpha : Type) (x : alpha) : x = x := by rfl -- alpha α\n";
        let document = MilParser.parse_checked(source).unwrap();
        let mut translations = TranslationMap::new();
        for segment in document.translatable_segments() {
            let translated = match segment.source.as_str() {
                "Getting Started" => "시작하기",
                "Read this." => "이 글을 읽으세요.",
                other => panic!("unexpected segment: {other}"),
            };
            translations.insert(segment.id.clone(), translated.to_string());
        }
        let output = MilParser.reconstruct(&document, &translations);
        assert_eq!(
            output,
            "import MIL.Common\n/- TEXT:\n시작하기\n========\n\n이 글을 읽으세요.\nBOTH: -/\nexample (alpha : Type) (x : alpha) : x = x := by rfl -- alpha α\n"
        );
    }

    #[test]
    fn mil_parser_requires_closed_text_blocks() {
        let error = MilParser
            .parse_checked("/- TEXT:\nNever closed.\n")
            .unwrap_err();
        assert!(error.to_string().contains("unterminated"));
    }

    #[test]
    fn parse_simple_paragraph() {
        let doc = RstParser.parse("Hello world. Goodbye world.");
        let segments = doc.translatable_segments();
        assert_eq!(segments.len(), 2);
        assert_eq!(segments[0].source, "Hello world.");
        assert_eq!(segments[1].source, "Goodbye world.");
    }

    #[test]
    fn wrapped_paragraph_is_one_block() {
        let doc = RstParser.parse("This is one\nwrapped sentence.\n");
        let segments = doc.translatable_segments();
        assert_eq!(segments.len(), 1);
        assert_eq!(segments[0].source, "This is one wrapped sentence.");
    }

    #[test]
    fn underlined_title_is_a_heading() {
        let doc = RstParser.parse("Chapter One\n===========\n\nBody text.\n");
        let segments = doc.translatable_segments();
        assert_eq!(segments[0].source, "Chapter One");
        assert_eq!(segments[0].block_type, BlockType::Heading);
    }

    #[test]
    fn adornment_styles_order_gives_levels() {
        let doc = RstParser.parse(
            "Title\n=====\n\nA.\n\nSection\n-------\n\nB.\n\nOther\n=====\n\nC.\n",
        );
        let blocks: Vec<&Block> = doc.sections.iter().flat_map(|s| &s.blocks).collect();
        let levels: Vec<u8> = blocks
            .iter()
            .filter(|b| b.block_type == BlockType::Heading)
            .map(|b| b.heading_level.unwrap())
            .collect();
        assert_eq!(levels, vec![1, 2, 1]);
    }

    #[test]
    fn overlined_title_is_a_heading() {
        let doc = RstParser.parse("=====\nTitle\n=====\n\nBody.\n");
        let segments = doc.translatable_segments();
        assert_eq!(segments[0].source, "Title");
        assert_eq!(segments[0].block_type, BlockType::Heading);
    }

    #[test]
    fn translated_title_redraws_its_underline_to_display_width() {
        let output = translate_all(
            &RstParser,
            "Getting Started\n===============\n\nBody.\n",
            &[("Getting Started", "시작하기"), ("Body.", "본문.")],
        );
        // Four Hangul syllables are eight columns wide.
        assert_eq!(output, "시작하기\n========\n\n본문.\n");
    }

    #[test]
    fn translated_overlined_title_redraws_both_lines() {
        let output = translate_all(
            &RstParser,
            "===============\nGetting Started\n===============\n\nBody.\n",
            &[("Getting Started", "시작하기"), ("Body.", "본문.")],
        );
        assert_eq!(output, "========\n시작하기\n========\n\n본문.\n");
    }

    #[test]
    fn translated_overlined_title_counts_preserved_indentation() {
        let output = translate_all(
            &RstParser,
            "==========\n Abstract\n==========\n\nBody.\n",
            &[("Abstract", "초록"), ("Body.", "본문.")],
        );
        assert_eq!(output, "=====\n 초록\n=====\n\n본문.\n");
    }

    #[test]
    fn literal_block_after_double_colon_is_opaque() {
        let source = "Some code::\n\n    print(\"hi\")\n    more()\n\nAfter text.\n";
        let doc = RstParser.parse(source);
        let segments = doc.translatable_segments();
        assert_eq!(segments.len(), 2);
        assert_eq!(segments[0].source, "Some code");
        assert_eq!(segments[1].source, "After text.");

        let output = translate_all(
            &RstParser,
            source,
            &[("Some code", "예시 코드"), ("After text.", "이후.")],
        );
        assert_eq!(output, "예시 코드::\n\n    print(\"hi\")\n    more()\n\n이후.\n");
    }

    #[test]
    fn standalone_double_colon_paragraph_stays_verbatim() {
        let source = "Intro.\n\n::\n\n    literal\n\nAfter.\n";
        let doc = RstParser.parse(source);
        let segments = doc.translatable_segments();
        assert_eq!(segments.len(), 2);
        let output = translate_all(&RstParser, source, &[("Intro.", "소개."), ("After.", "이후.")]);
        assert_eq!(output, "소개.\n\n::\n\n    literal\n\n이후.\n");
    }

    #[test]
    fn double_colon_directly_under_a_paragraph_continues_it() {
        let source = "Intro.\n::\n\n    literal\n\nAfter.\n";
        let doc = RstParser.parse(source);
        let segments = doc.translatable_segments();
        assert_eq!(segments.len(), 2);
        assert_eq!(segments[0].source, "Intro.");
        assert_eq!(segments[1].source, "After.");
    }

    #[test]
    fn expanded_double_colon_keeps_its_space() {
        // `text ::` renders without the colon; the marker still stays outside
        // the span.
        let source = "As follows ::\n\n    x = 1\n";
        let doc = RstParser.parse(source);
        assert_eq!(doc.translatable_segments()[0].source, "As follows");
        let output = translate_all(&RstParser, source, &[("As follows", "다음과 같이")]);
        assert_eq!(output, "다음과 같이 ::\n\n    x = 1\n");
    }

    #[test]
    fn code_block_directive_is_opaque() {
        let source = "Before.\n\n.. code-block:: python\n   :linenos:\n\n   def f():\n       pass\n\nAfter.\n";
        let doc = RstParser.parse(source);
        let segments = doc.translatable_segments();
        assert_eq!(segments.len(), 2);
        assert_eq!(segments[0].source, "Before.");
        assert_eq!(segments[1].source, "After.");
    }

    #[test]
    fn inline_admonition_metadata_is_translatable() {
        let source = "\
.. note:: Remember the first line.
   Continue the note.

.. seealso:: Read the companion guide.

.. tip:: Prefer the focused command.

.. impl-detail:: This is an implementation detail.
";
        let doc = RstParser.parse(source);
        let sources: Vec<_> = doc
            .translatable_segments()
            .iter()
            .map(|segment| segment.source.as_str())
            .collect();
        assert_eq!(
            sources,
            [
                "Remember the first line.",
                "Continue the note.",
                "Read the companion guide.",
                "Prefer the focused command.",
                "This is an implementation detail.",
            ]
        );

        let output = translate_all(
            &RstParser,
            source,
            &[
                ("Remember the first line.", "첫 줄을 기억하십시오."),
                ("Continue the note.", "메모를 이어 갑니다."),
                ("Read the companion guide.", "관련 안내서를 읽으십시오."),
                ("Prefer the focused command.", "범위가 좁은 명령을 사용하십시오."),
                (
                    "This is an implementation detail.",
                    "이는 구현 세부 사항입니다.",
                ),
            ],
        );
        assert_eq!(
            output,
            "\
.. note:: 첫 줄을 기억하십시오.
   메모를 이어 갑니다.

.. seealso:: 관련 안내서를 읽으십시오.

.. tip:: 범위가 좁은 명령을 사용하십시오.

.. impl-detail:: 이는 구현 세부 사항입니다.
"
        );
    }

    #[test]
    fn titled_directive_metadata_is_translatable() {
        let source = "\
.. admonition:: Compiler knowledge is optional.

   Admonition body.

.. sidebar:: Sentence case

   Sidebar body.

.. rubric:: Footnotes
";
        let doc = RstParser.parse(source);
        let sources: Vec<_> = doc
            .translatable_segments()
            .iter()
            .map(|segment| segment.source.as_str())
            .collect();
        assert_eq!(
            sources,
            [
                "Compiler knowledge is optional.",
                "Admonition body.",
                "Sentence case",
                "Sidebar body.",
                "Footnotes",
            ]
        );

        let output = translate_all(
            &RstParser,
            source,
            &[
                ("Compiler knowledge is optional.", "컴파일러 지식은 선택 사항입니다."),
                ("Admonition body.", "권고문 본문입니다."),
                ("Sentence case", "문장형 대소문자"),
                ("Sidebar body.", "사이드바 본문입니다."),
                ("Footnotes", "각주"),
            ],
        );
        assert_eq!(
            output,
            "\
.. admonition:: 컴파일러 지식은 선택 사항입니다.

   권고문 본문입니다.

.. sidebar:: 문장형 대소문자

   사이드바 본문입니다.

.. rubric:: 각주
"
        );
    }

    #[test]
    fn topic_title_is_translatable_while_its_marker_stays_exact() {
        let source = ".. topic:: Jane Doe (Canada)\n\n   Topic body.\n";
        let doc = RstParser.parse(source);
        let sources: Vec<&str> = doc
            .translatable_segments()
            .iter()
            .map(|segment| segment.source.as_str())
            .collect();
        assert_eq!(sources, ["Jane Doe (Canada)", "Topic body."]);

        let output = translate_all(
            &RstParser,
            source,
            &[
                ("Jane Doe (Canada)", "Jane Doe (캐나다)"),
                ("Topic body.", "토픽 본문입니다."),
            ],
        );
        assert_eq!(output, ".. topic:: Jane Doe (캐나다)\n\n   토픽 본문입니다.\n");
    }

    #[test]
    fn tab_title_is_translatable_while_technical_tabs_can_stay_exact() {
        let source = "\
.. tab:: Other / pip

   Install with pip.

.. tab:: Python 3.15+

   Run Python.
";
        let doc = RstParser.parse(source);
        let sources: Vec<&str> = doc
            .translatable_segments()
            .iter()
            .map(|segment| segment.source.as_str())
            .collect();
        assert_eq!(
            sources,
            ["Other / pip", "Install with pip.", "Python 3.15+", "Run Python."]
        );

        let output = translate_all(
            &RstParser,
            source,
            &[
                ("Other / pip", "기타 / pip"),
                ("Install with pip.", "pip로 설치합니다."),
                ("Python 3.15+", "Python 3.15+"),
                ("Run Python.", "Python을 실행합니다."),
            ],
        );
        assert!(output.contains(".. tab:: 기타 / pip"));
        assert!(output.contains(".. tab:: Python 3.15+"));
    }

    #[test]
    fn image_alt_metadata_is_translatable_while_opaque_content_is_preserved() {
        let source = "\
.. figure:: chart.svg
   :class: only-light
   :alt: A chart showing translator workload
         falling with team size.

   Keep this opaque figure body.

.. image:: badge.svg
   :target: https://example.com/
   :alt: Documentation status
";
        let doc = RstParser.parse(source);
        let sources: Vec<_> = doc
            .translatable_segments()
            .iter()
            .map(|segment| segment.source.as_str())
            .collect();
        assert_eq!(
            sources,
            [
                "A chart showing translator workload falling with team size.",
                "Documentation status",
            ]
        );

        let output = translate_all(
            &RstParser,
            source,
            &[
                (
                    "A chart showing translator workload falling with team size.",
                    "번역가가 늘수록 작업량이 줄어드는 차트.",
                ),
                ("Documentation status", "문서 상태"),
            ],
        );
        assert_eq!(
            output,
            "\
.. figure:: chart.svg
   :class: only-light
   :alt: 번역가가 늘수록 작업량이 줄어드는 차트.

   Keep this opaque figure body.

.. image:: badge.svg
   :target: https://example.com/
   :alt: 문서 상태
"
        );
    }

    #[test]
    fn substitution_image_alt_is_translatable_while_structure_is_preserved() {
        let source = "\
.. |Status| image:: https://example.com/badge.svg
   :target: https://example.com/status
   :alt: Documentation status
";
        let doc = RstParser.parse(source);
        let segments = doc.translatable_segments();
        assert_eq!(segments.len(), 1);
        assert_eq!(segments[0].source, "Documentation status");

        let output = translate_all(
            &RstParser,
            source,
            &[("Documentation status", "문서 상태")],
        );
        assert_eq!(
            output,
            "\
.. |Status| image:: https://example.com/badge.svg
   :target: https://example.com/status
   :alt: 문서 상태
"
        );
    }

    #[test]
    fn prose_code_caption_is_translatable_while_code_is_byte_identical() {
        let source = "\
.. code-block:: python
   :caption: Rendering a greeting.
   :emphasize-lines: 1

   print(\"hello\")
";
        let doc = RstParser.parse(source);
        let segments = doc.translatable_segments();
        assert_eq!(segments.len(), 1);
        assert_eq!(segments[0].source, "Rendering a greeting.");

        let output = translate_all(
            &RstParser,
            source,
            &[("Rendering a greeting.", "인사말 렌더링.")],
        );
        assert_eq!(
            output,
            "\
.. code-block:: python
   :caption: 인사말 렌더링.
   :emphasize-lines: 1

   print(\"hello\")
"
        );
    }

    #[test]
    fn path_code_caption_stays_opaque() {
        let source = "\
.. code-block:: python
   :caption: Lib/example.py

   print(\"hello\")
";
        let doc = RstParser.parse(source);
        assert!(doc.translatable_segments().is_empty());
        assert_eq!(RstParser.reconstruct(&doc, &TranslationMap::new()), source);
    }

    #[test]
    fn note_directive_body_is_translatable() {
        let source = ".. note::\n\n   Remember this fact.\n\nAfter.\n";
        let doc = RstParser.parse(source);
        let segments = doc.translatable_segments();
        assert_eq!(segments.len(), 2);
        assert_eq!(segments[0].source, "Remember this fact.");

        let output = translate_all(
            &RstParser,
            source,
            &[("Remember this fact.", "이 사실을 기억하세요."), ("After.", "이후.")],
        );
        assert_eq!(output, ".. note::\n\n   이 사실을 기억하세요.\n\n이후.\n");
    }

    #[test]
    fn function_directive_body_is_translatable_but_signature_is_not() {
        let source = ".. function:: attach_gdb()\n\n   Run an interp-level gdb.\n";
        let doc = RstParser.parse(source);
        let segments = doc.translatable_segments();
        assert_eq!(segments.len(), 1);
        assert_eq!(segments[0].source, "Run an interp-level gdb.");
    }

    #[test]
    fn py_domain_function_directive_body_is_translatable_but_signature_is_not() {
        let source = ".. py:function:: getexecutioncontext()\n\n   Return the currently active execution context.\n";
        let doc = RstParser.parse(source);
        let segments = doc.translatable_segments();
        assert_eq!(segments.len(), 1);
        assert_eq!(
            segments[0].source,
            "Return the currently active execution context."
        );
    }

    #[test]
    fn c_domain_function_directive_body_is_translatable_but_signature_is_not() {
        let source = ".. c:function:: int f(void)\n\n   Does things.\n";
        let doc = RstParser.parse(source);
        let segments = doc.translatable_segments();
        assert_eq!(segments.len(), 1);
        assert_eq!(segments[0].source, "Does things.");
    }

    #[test]
    fn graphviz_directive_is_opaque() {
        let source = ".. graphviz::\n\n    digraph G{\n    a -> b; // c\n    }\n";
        let doc = RstParser.parse(source);
        let segments = doc.translatable_segments();
        assert_eq!(segments.len(), 0);
        let output = translate_all(&RstParser, source, &[]);
        assert_eq!(output, source);
    }

    #[test]
    fn digraph_directive_is_opaque() {
        let source = ".. digraph:: t\n\n   a -> b\n";
        let doc = RstParser.parse(source);
        let segments = doc.translatable_segments();
        assert_eq!(segments.len(), 0);
        let output = translate_all(&RstParser, source, &[]);
        assert_eq!(output, source);
    }

    #[test]
    fn hyperlink_target_line_is_still_not_a_directive() {
        let source = ".. _target: http://x\n";
        let doc = RstParser.parse(source);
        let segments = doc.translatable_segments();
        assert_eq!(segments.len(), 0);
        let output = translate_all(&RstParser, source, &[]);
        assert_eq!(output, source);
    }

    #[test]
    fn substitution_replace_line_is_still_not_a_directive() {
        let source = ".. |sub| replace:: x\n";
        let doc = RstParser.parse(source);
        let segments = doc.translatable_segments();
        assert_eq!(segments.len(), 0);
        let output = translate_all(&RstParser, source, &[]);
        assert_eq!(output, source);
    }

    #[test]
    fn toctree_is_opaque() {
        let source = "Head.\n\n.. toctree::\n  :maxdepth: 1\n\n  introduction\n  architecture\n\nTail.\n";
        let doc = RstParser.parse(source);
        let segments = doc.translatable_segments();
        assert_eq!(segments.len(), 2);
        let output = translate_all(&RstParser, source, &[("Head.", "머리."), ("Tail.", "꼬리.")]);
        assert_eq!(
            output,
            "머리.\n\n.. toctree::\n  :maxdepth: 1\n\n  introduction\n  architecture\n\n꼬리.\n"
        );
    }

    #[test]
    fn hyperlink_targets_stay_verbatim() {
        let source = "See the site.\n\n.. _fast: https://speed.pypy.org\n.. _Python: https://python.org/\n";
        let doc = RstParser.parse(source);
        let segments = doc.translatable_segments();
        assert_eq!(segments.len(), 1);
        assert_eq!(segments[0].source, "See the site.");
    }

    #[test]
    fn consecutive_anonymous_hyperlink_targets_stay_on_separate_lines() {
        let source = "See `one`__ and `two`__.\n\n__ https://example.com/one\n__ https://example.com/two\n";
        let doc = RstParser.parse(source);
        let segments = doc.translatable_segments();
        assert_eq!(segments.len(), 2);
        assert_eq!(segments[0].source, "See `one`__ and `two`__.");
        assert_eq!(
            segments[1].source,
            "__ https://example.com/one __ https://example.com/two"
        );
        let mut translations = TranslationMap::new();
        translations.insert(segments[1].id.clone(), "broken joined targets".to_string());
        assert_eq!(RstParser.reconstruct(&doc, &translations), source);
    }

    #[test]
    fn comment_is_opaque() {
        let source = ".. this is a comment\n   continued here\n\nReal text.\n";
        let doc = RstParser.parse(source);
        let segments = doc.translatable_segments();
        assert_eq!(segments.len(), 1);
        assert_eq!(segments[0].source, "Real text.");
    }

    #[test]
    fn doctest_block_is_opaque() {
        let source = "Try it:\n\n>>> 1 + 1\n2\n\nDone.\n";
        let doc = RstParser.parse(source);
        let segments = doc.translatable_segments();
        assert_eq!(segments.len(), 2);
        assert_eq!(segments[0].source, "Try it:");
        assert_eq!(segments[1].source, "Done.");
    }

    #[test]
    fn bullet_list_items_are_separate() {
        let source = "* First item.\n* Second item.\n";
        let doc = RstParser.parse(source);
        let segments = doc.translatable_segments();
        assert_eq!(segments.len(), 2);
        assert_eq!(segments[0].block_type, BlockType::ListItem);
        let output = translate_all(
            &RstParser,
            source,
            &[("First item.", "첫째."), ("Second item.", "둘째.")],
        );
        assert_eq!(output, "* 첫째.\n* 둘째.\n");
    }

    #[test]
    fn wrapped_list_item_joins_its_continuation() {
        let source = "* If you want to help develop PyPy, have a look\n  at contributing.\n";
        let doc = RstParser.parse(source);
        let segments = doc.translatable_segments();
        assert_eq!(segments.len(), 1);
        assert_eq!(
            segments[0].source,
            "If you want to help develop PyPy, have a look at contributing."
        );
    }

    #[test]
    fn a_list_marker_inside_a_paragraph_is_text() {
        // docutils reads every line up to a blank one as the paragraph: a
        // wrapped inline literal can leave `B)` or `-` at the start of a line.
        let source = "Also, ``isinstance(x, B)`` is equivalent to ``issubclass(x.__class__,\n\
                      B) or issubclass(type(x), B)``.  (It is possible\n\
                      - or not.)\n";
        let doc = RstParser.parse(source);
        let segments = doc.translatable_segments();
        assert_eq!(segments.len(), 1);
        assert_eq!(segments[0].block_type, BlockType::Paragraph);
        assert_eq!(
            segments[0].source,
            "Also, ``isinstance(x, B)`` is equivalent to ``issubclass(x.__class__, \
             B) or issubclass(type(x), B)``.  (It is possible - or not.)"
        );
    }

    #[test]
    fn a_list_marker_inside_a_list_item_is_text() {
        let source = "1. First item text that\n   B) continues.\n2. Second.\n";
        let doc = RstParser.parse(source);
        let segments = doc.translatable_segments();
        assert_eq!(
            segments
                .iter()
                .map(|s| s.source.as_str())
                .collect::<Vec<_>>(),
            vec!["First item text that B) continues.", "Second."]
        );
    }

    #[test]
    fn enumerated_list_items_are_separate() {
        let source = "1. First step.\n2. Second step.\n#. Third step.\n";
        let doc = RstParser.parse(source);
        let segments = doc.translatable_segments();
        assert_eq!(segments.len(), 3);
        assert!(segments.iter().all(|s| s.block_type == BlockType::ListItem));
    }

    #[test]
    fn indented_alpha_enumeration_keeps_each_item_on_its_source_line() {
        let source = "- Unpack.\n\n  a. Parse the metadata.\n  b. Check the version.\n  c. Finish.\n";
        let doc = RstParser.parse(source);
        assert_eq!(RstParser.reconstruct(&doc, &TranslationMap::new()), source);

        let mut translations = TranslationMap::new();
        for segment in doc.translatable_segments() {
            translations.insert(segment.id.clone(), segment.source.clone());
        }

        assert_eq!(RstParser.reconstruct(&doc, &translations), source);
    }

    #[test]
    fn enumerated_title_underline_expands_without_changing_its_block_id() {
        let source = "6. METH_FASTCALL is private\n---------------------------\n";
        let doc = RstParser.parse(source);
        let segment = &doc.translatable_segments()[0];
        assert_eq!(segment.id.0, "section:0/block:0/seg:0");
        assert_eq!(segment.block_type, BlockType::ListItem);

        let output = translate_all(
            &RstParser,
            source,
            &[("METH_FASTCALL is private", "METH_FASTCALL은 비공개입니다")],
        );
        assert_eq!(
            output,
            "6. METH_FASTCALL은 비공개입니다\n-------------------------------\n"
        );
    }

    #[test]
    fn enumerated_title_with_inline_literal_expands_its_underline() {
        let source = "1. Add an operator for ``Union[type1, type2]``?\n--------------------------------------------------\n";
        let output = translate_all(
            &RstParser,
            source,
            &[(
                "Add an operator for ``Union[type1, type2]``?",
                "``Union[type1, type2]``\\ 를 위한 새 연산자를 추가할까요?",
            )],
        );
        let mut lines = output.lines();
        let title = lines.next().unwrap();
        let underline = lines.next().unwrap();
        assert!(underline.len() > 50, "{title}\n{underline}");
    }

    fn sources(source: &str) -> Vec<String> {
        RstParser
            .parse(source)
            .translatable_segments()
            .iter()
            .map(|segment| segment.source.clone())
            .collect()
    }

    /// Translate every segment to `<source>!` so a test can see where each
    /// translation lands and which bytes stay verbatim around it.
    fn mark_all(source: &str) -> String {
        let doc = RstParser.parse(source);
        let mut translations = TranslationMap::new();
        for segment in doc.translatable_segments() {
            translations.insert(segment.id.clone(), format!("{}!", segment.source));
        }
        RstParser.reconstruct(&doc, &translations)
    }

    #[test]
    fn an_initial_followed_by_its_paragraph_is_not_a_list_marker() {
        // The sentence splitter still cuts after `J.`; the point is that the
        // two lines stay one paragraph with the initial inside it.
        let doc = RstParser.parse("J. Smith wrote this\nand more.\n");
        let blocks = &doc.sections[0].blocks;
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].block_type, BlockType::Paragraph);
        assert_eq!(blocks[0].raw_content, "J. Smith wrote this\nand more.");
        assert!(parse_enumerated_item("1. A real item").is_some());
        assert!(parse_enumerated_item("a) Also real").is_some());
    }

    #[test]
    fn a_one_line_initial_before_a_blank_line_is_a_list_item_as_in_docutils() {
        let doc = RstParser.parse("J. Smith wrote this.\n\nNext.\n");
        let segments = doc.translatable_segments();
        assert_eq!(segments[0].source, "Smith wrote this.");
        assert_eq!(segments[0].block_type, BlockType::ListItem);
        assert_eq!(segments[1].source, "Next.");
    }

    #[test]
    fn alpha_and_roman_enumerators_keep_their_markers_and_spacing() {
        let source = "    a. Let x.\n    b. For each:\n\n        i.   Set a\n        ii.  Set b\n        iii. Set c.\n";
        assert_eq!(
            sources(source),
            vec!["Let x.", "For each:", "Set a", "Set b", "Set c."]
        );
        assert_eq!(
            mark_all(source),
            "    a. Let x.!\n    b. For each:!\n\n        i.   Set a!\n        ii.  Set b!\n        iii. Set c.!\n"
        );
    }

    #[test]
    fn a_marker_followed_by_two_spaces_keeps_its_body_column() {
        let source = "    a.  Use a model\n\n        Instead of this.\n";
        assert_eq!(sources(source), vec!["Use a model", "Instead of this."]);
        assert_eq!(
            mark_all(source),
            "    a.  Use a model!\n\n        Instead of this.!\n"
        );
    }

    #[test]
    fn an_enumerator_whose_next_line_continues_unindented_is_a_paragraph() {
        let doc = RstParser.parse("a. It should enable\nto re-enable.\n");
        let blocks = &doc.sections[0].blocks;
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].block_type, BlockType::Paragraph);
        assert_eq!(
            normalize_inline_text(&blocks[0].raw_content),
            "a. It should enable to re-enable."
        );
    }

    #[test]
    fn i_after_h_is_alphabetic_and_alone_is_roman() {
        let doc = RstParser.parse("h. x\ni. y\n");
        let segments = doc.translatable_segments();
        assert_eq!(
            segments.iter().map(|s| s.source.as_str()).collect::<Vec<_>>(),
            vec!["x", "y"]
        );
        assert!(segments.iter().all(|s| s.block_type == BlockType::ListItem));

        let previous = enumerator("h. x").unwrap();
        assert_eq!(previous.kind, EnumKind::LowerAlpha);
        let after_h = enumerator_after("i. y", Some(&previous)).unwrap();
        assert_eq!(after_h.kind, EnumKind::LowerAlpha);
        assert_eq!(after_h.ordinal, Some(9));
        let alone = enumerator("i. x").unwrap();
        assert_eq!(alone.kind, EnumKind::LowerRoman);
        assert_eq!(alone.ordinal, Some(1));

        let doc = RstParser.parse("i. x\n\n");
        let segments = doc.translatable_segments();
        assert_eq!(segments[0].source, "x");
        assert_eq!(segments[0].block_type, BlockType::ListItem);
    }

    #[test]
    fn enumerator_reads_every_docutils_form() {
        let paren = enumerator("(iv) text").unwrap();
        assert_eq!((paren.kind, paren.format, paren.ordinal), (EnumKind::LowerRoman, EnumFormat::Parens, Some(4)));
        assert_eq!(paren.text_offset, 5);
        let rparen = enumerator("  B) text").unwrap();
        assert_eq!((rparen.kind, rparen.format, rparen.ordinal), (EnumKind::UpperAlpha, EnumFormat::RParen, Some(2)));
        assert_eq!(rparen.text_offset, 5);
        let auto = enumerator("#. text").unwrap();
        assert_eq!((auto.kind, auto.ordinal), (EnumKind::Auto, Some(1)));
        let arabic = enumerator("12.   text").unwrap();
        assert_eq!((arabic.kind, arabic.ordinal, arabic.text_offset), (EnumKind::Arabic, Some(12), 6));
        assert_eq!(enumerator("iiii. text").unwrap().ordinal, None);
        assert!(enumerator("ab. text").is_none());
        assert!(enumerator("1.text").is_none());
        assert!(enumerator("(1. text").is_none());
        assert!(enumerator("1.").is_some());
    }

    #[test]
    fn block_start_follows_docutils_markers() {
        for line in [
            "- item", "+ item", "* item", "• item", "‣ item", "⁃ item", "-",
            "1. item", "(a) item", "iv) item", "#. item", "A.",
            ":field: body", ":field name:", "-x  option", "-o FILE  desc", "/V  desc",
            "+x  desc", "-x, --long  desc",
            "| line", "|", ">>> 1 + 1", ">>>",
        ] {
            assert!(reads_as_block_start(line, None, None), "{line:?}");
        }
        for line in [
            "-item", "(2009 was", "1.5 times", "ab. word", "iiii. no",
            ":ref:`x` is", "-x option", "--long=VALUE", "|x|", ">>>x", "plain text",
        ] {
            assert!(!reads_as_block_start(line, None, None), "{line:?}");
        }
    }

    #[test]
    fn block_start_applies_the_second_line_check_to_enumerators_only() {
        assert!(!reads_as_block_start("a. It should", Some("go on."), None));
        assert!(reads_as_block_start("a. It should", Some(""), None));
        assert!(reads_as_block_start("a. It should", Some("   go on."), None));
        assert!(reads_as_block_start("a. first", Some("b. second"), None));
        assert!(reads_as_block_start("a. first", Some("#. second"), None));
        assert!(!reads_as_block_start("a. first", Some("c. third"), None));
        assert!(!reads_as_block_start("a. first", Some("b) second"), None));
        assert!(reads_as_block_start("  (1) first", Some("  (2) second"), None));
        assert!(reads_as_block_start("  1. first", Some("next at a shallower block"), None));
        assert!(reads_as_block_start("* bullet", Some("goes on"), None));
        assert!(reads_as_block_start(":field: x", Some("goes on"), None));
        assert!(reads_as_block_start("-x option", Some("   desc"), None));
        assert!(reads_as_block_start("--long=VALUE", Some("   desc"), None));
        assert!(!reads_as_block_start("--long=VALUE", Some("desc"), None));
        assert!(reads_as_block_start("iv. x", Some("v. y"), None));
        assert!(reads_as_block_start("I. x", Some("II. y"), None));
        assert!(!reads_as_block_start("z. x", Some("aa. y"), None));
    }

    #[test]
    fn a_one_character_underline_under_a_one_line_title_makes_a_section() {
        let source = "Intro.\n\nR\n-\n\nR is a language.\n";
        let doc = RstParser.parse(source);
        assert_eq!(doc.sections.len(), 2);
        let title = &doc.sections[1].blocks[0];
        assert_eq!(title.block_type, BlockType::Heading);
        assert_eq!(title.segments[0].source, "R");
        assert_eq!(RstParser.reconstruct(&doc, &TranslationMap::new()), source);
    }

    #[test]
    fn a_one_character_line_is_not_an_overline_or_transition() {
        assert_eq!(adornment_char("-"), None);
        assert_eq!(underline_char("-"), Some('-'));
        assert_eq!(underline_char(".."), None);
        assert_eq!(underline_char("-a"), None);
    }

    #[test]
    fn each_line_of_a_glossary_term_group_is_its_own_term() {
        let source = ".. glossary::\n\n    license expression\n    SPDX expression\n        A string.\n";
        let doc = RstParser.parse(source);
        let segments = doc.translatable_segments();
        assert_eq!(
            segments.iter().map(|s| s.source.as_str()).collect::<Vec<_>>(),
            vec!["license expression", "SPDX expression", "A string."]
        );
        assert_eq!(segments[0].block_type, BlockType::Heading);
        assert_eq!(segments[1].block_type, BlockType::Heading);
        assert_eq!(segments[2].block_type, BlockType::Paragraph);
        assert_eq!(
            mark_all(source),
            ".. glossary::\n\n    license expression!\n    SPDX expression!\n        A string.!\n"
        );
    }

    #[test]
    fn glossary_options_stay_verbatim_and_terms_end_with_the_directive() {
        let source = ".. glossary::\n   :sorted:\n\n   term\n      Definition.\n\n   other\n      More.\n\nAfter the glossary\nit goes on.\n";
        assert_eq!(
            sources(source),
            vec!["term", "Definition.", "other", "More.", "After the glossary it goes on."]
        );
        assert_eq!(RstParser.reconstruct(&RstParser.parse(source), &TranslationMap::new()), source);
    }

    #[test]
    fn docinfo_field_list_stays_verbatim() {
        let source = ":author: Someone\n:date: today\n\nBody.\n";
        let doc = RstParser.parse(source);
        let segments = doc.translatable_segments();
        assert_eq!(segments.len(), 1);
        assert_eq!(segments[0].source, "Body.");
    }

    #[test]
    fn structural_sphinx_fields_stay_verbatim() {
        let source = ":orphan:\n:tocdepth: 2\n:nocomments: true\n:nosearch: yes\n";
        let doc = RstParser.parse(source);
        assert!(doc.translatable_segments().is_empty());
        assert_eq!(RstParser.reconstruct(&doc, &TranslationMap::new()), source);
    }

    #[test]
    fn rendered_abstract_and_dedication_bodies_are_translatable() {
        let source = ":abstract: A concise summary.\n:dedication: For contributors.\n";
        let doc = RstParser.parse(source);
        let sources: Vec<&str> = doc
            .translatable_segments()
            .iter()
            .map(|segment| segment.source.as_str())
            .collect();
        assert_eq!(sources, ["A concise summary.", "For contributors."]);

        let output = translate_all(
            &RstParser,
            source,
            &[
                ("A concise summary.", "간결한 요약입니다."),
                ("For contributors.", "기여자에게 바칩니다."),
            ],
        );
        assert_eq!(
            output,
            ":abstract: 간결한 요약입니다.\n:dedication: 기여자에게 바칩니다.\n"
        );
    }

    #[test]
    fn standalone_field_body_is_translatable_without_touching_its_name() {
        let source = "Intro.\n\n:feature: Ready for readers.\n";
        let doc = RstParser.parse(source);
        let segments = doc.translatable_segments();
        assert_eq!(segments.len(), 2);
        assert_eq!(segments[1].source, "Ready for readers.");

        let output = translate_all(
            &RstParser,
            source,
            &[("Ready for readers.", "독자를 위한 설명입니다.")],
        );
        assert_eq!(output, "Intro.\n\n:feature: 독자를 위한 설명입니다.\n");
    }

    #[test]
    fn multiline_standalone_field_body_reconstructs_as_one_valid_field() {
        let source = "Intro.\n\n:feature: First line\n   and second line.\n\nAfter.\n";
        let doc = RstParser.parse(source);
        let segments = doc.translatable_segments();
        assert_eq!(segments.len(), 3);
        assert_eq!(segments[1].source, "First line and second line.");

        let output = translate_all(
            &RstParser,
            source,
            &[(
                "First line and second line.",
                "두 줄로 작성된 필드 설명입니다.",
            )],
        );
        assert_eq!(
            output,
            "Intro.\n\n:feature: 두 줄로 작성된 필드 설명입니다.\n\nAfter.\n"
        );
    }

    #[test]
    fn directive_option_signatures_and_paths_stay_opaque() {
        let source = "\
.. code-block:: python
   :name: Lib/example.py
   :signature: str(object='')
   :caption: Lib/example.py

   print('hello')
";
        let doc = RstParser.parse(source);
        assert!(doc.translatable_segments().is_empty());
        assert_eq!(RstParser.reconstruct(&doc, &TranslationMap::new()), source);
    }

    #[test]
    fn a_role_at_line_start_is_not_a_field() {
        let source = "See\n:ref:`contact` for details.\n";
        let doc = RstParser.parse(source);
        let segments = doc.translatable_segments();
        assert_eq!(segments.len(), 1);
        assert_eq!(segments[0].source, "See :ref:`contact` for details.");
    }

    #[test]
    fn definition_body_splits_from_its_term() {
        // The indent change is structure; joining across it would splice the
        // definition list away.
        let source = "term\n    the definition text\n";
        let doc = RstParser.parse(source);
        let segments = doc.translatable_segments();
        assert_eq!(segments.len(), 2);
        assert_eq!(segments[0].source, "term");
        assert_eq!(segments[1].source, "the definition text");

        let output = translate_all(
            &RstParser,
            source,
            &[("term", "용어"), ("the definition text", "정의 텍스트")],
        );
        assert_eq!(output, "용어\n    정의 텍스트\n");
    }

    #[test]
    fn grid_table_cells_become_segments() {
        let source = "+----+----+\n| a  | b  |\n+====+====+\n| c  | d  |\n+----+----+\n\nAfter.\n";
        let doc = RstParser.parse(source);
        let segments = doc.translatable_segments();
        let sources: Vec<&str> = segments.iter().map(|s| s.source.as_str()).collect();
        assert_eq!(sources, vec!["a", "b", "c", "d", "After."]);

        let cells: Vec<&Block> = doc
            .sections
            .iter()
            .flat_map(|s| &s.blocks)
            .filter(|b| matches!(b.role, BlockRole::TableCell { .. }))
            .collect();
        assert_eq!(cells.len(), 4);
        assert!(matches!(
            &cells[0].role,
            BlockRole::TableCell { table: 0, column: 0, label_row: true, header: None }
        ));
        assert!(matches!(
            &cells[3].role,
            BlockRole::TableCell { table: 0, column: 1, label_row: false, header: Some(h) }
                if h == "b"
        ));
    }

    #[test]
    fn list_table_cells_are_segments_with_headers() {
        let source = ".. list-table::\n   :header-rows: 1\n\n   * - Avoid\n     - Instead\n   * - whitelist\n     - allowlist\n";
        let doc = RstParser.parse(source);
        let sources: Vec<&str> = doc
            .translatable_segments()
            .iter()
            .map(|segment| segment.source.as_str())
            .collect();
        assert_eq!(sources, vec!["Avoid", "Instead", "whitelist", "allowlist"]);

        let cells: Vec<&Block> = doc.sections.iter().flat_map(|s| &s.blocks)
            .filter(|block| matches!(block.role, BlockRole::TableCell { .. }))
            .collect();
        assert!(matches!(cells[0].role,
            BlockRole::TableCell { table: 0, column: 0, label_row: true, .. }));
        assert!(matches!(&cells[3].role,
            BlockRole::TableCell { table: 0, column: 1, label_row: false, header: Some(header) }
            if header == "Instead"));
    }

    #[test]
    fn translated_list_table_keeps_directive_structure() {
        let source = ".. list-table::\n   :header-rows: 1\n\n   * - Avoid\n     - Instead\n   * - whitelist\n     - allowlist\n";
        let output = translate_all(
            &RstParser,
            source,
            &[("Avoid", "피할 표현"), ("Instead", "대신 사용할 표현"),
              ("whitelist", "허용 목록"), ("allowlist", "허용 목록")],
        );
        assert_eq!(output,
            ".. list-table::\n   :header-rows: 1\n\n   * - 피할 표현\n     - 대신 사용할 표현\n   * - 허용 목록\n     - 허용 목록\n");
    }

    #[test]
    fn translated_wrapped_list_table_cell_collapses_without_touching_next_cell() {
        let source = ".. list-table::\n\n   * - `Usage <https://example.com>`__,\n       `Limitations <https://example.com/limits>`__\n     - maintainer\n";
        let output = translate_all(
            &RstParser,
            source,
            &[("`Usage <https://example.com>`__, `Limitations <https://example.com/limits>`__",
               "`사용법 <https://example.com>`__, `제한 사항 <https://example.com/limits>`__")],
        );
        assert!(output.contains("   * - `사용법 <https://example.com>`__, `제한 사항 <https://example.com/limits>`__\n     - maintainer"));
    }

    #[test]
    fn list_table_with_non_integer_header_rows_stays_opaque() {
        let source = ".. list-table::\n   :header-rows: one\n\n   * - Avoid\n     - Instead\n";
        let doc = RstParser.parse(source);
        assert!(doc.all_segments().is_empty());
        assert_eq!(RstParser.reconstruct(&doc, &TranslationMap::new()), source);
    }

    #[test]
    fn list_table_with_too_many_header_rows_stays_opaque() {
        let source = ".. list-table::\n   :header-rows: 2\n\n   * - Avoid\n     - Instead\n";
        let doc = RstParser.parse(source);
        assert!(doc.all_segments().is_empty());
        assert_eq!(RstParser.reconstruct(&doc, &TranslationMap::new()), source);
    }

    #[test]
    fn list_table_with_inconsistent_nonempty_row_widths_stays_opaque() {
        let source = ".. list-table::\n\n   * - Avoid\n     - Instead\n   * - whitelist\n";
        let doc = RstParser.parse(source);
        assert!(doc.all_segments().is_empty());
        assert_eq!(RstParser.reconstruct(&doc, &TranslationMap::new()), source);
    }

    #[test]
    fn list_table_with_malformed_post_row_lines_stays_opaque() {
        for source in [
            ".. list-table::\n\n   * - Keep me\n   malformed same-level text\n",
            ".. list-table::\n\n   * - Keep me\n   * malformed row\n",
        ] {
            assert!(directive_table::parse_list(source, 0..source.len()).is_none());
            let doc = RstParser.parse(source);
            assert!(doc.all_segments().is_empty());
            assert_eq!(RstParser.reconstruct(&doc, &TranslationMap::new()), source);
        }
    }

    #[test]
    fn empty_or_rowless_list_table_stays_opaque() {
        for source in [
            ".. list-table:: Empty\n",
            ".. list-table::\n   :header-rows: 0\n",
        ] {
            assert!(directive_table::parse_list(source, 0..source.len()).is_none());
            let doc = RstParser.parse(source);
            assert!(doc.all_segments().is_empty());
            assert_eq!(RstParser.reconstruct(&doc, &TranslationMap::new()), source);
        }
    }

    #[test]
    fn empty_list_table_cells_keep_later_cells_in_their_column() {
        let source = ".. list-table::\n   :header-rows: 1\n\n   * -\n     - Preferred\n   * - Avoid\n     - allowlist\n";
        let doc = RstParser.parse(source);
        let cells: Vec<&Block> = doc.sections.iter().flat_map(|s| &s.blocks)
            .filter(|block| matches!(block.role, BlockRole::TableCell { .. }))
            .collect();

        assert_eq!(cells.len(), 3);
        assert!(matches!(&cells[0].role,
            BlockRole::TableCell { column: 1, label_row: true, header: None, .. }));
        assert!(matches!(&cells[1].role,
            BlockRole::TableCell { column: 0, label_row: false, header: None, .. }));
        assert!(matches!(&cells[2].role,
            BlockRole::TableCell { column: 1, label_row: false, header: Some(header), .. }
            if header == "Preferred"));
    }

    #[test]
    fn translated_list_table_title_uses_its_directive_span() {
        let source = ".. list-table:: Preferred terminology\n\n   * - Avoid\n";
        let output = translate_all(
            &RstParser,
            source,
            &[("Preferred terminology", "권장 용어"), ("Avoid", "피할 표현")],
        );
        assert_eq!(output, ".. list-table:: 권장 용어\n\n   * - 피할 표현\n");
    }

    #[test]
    fn inline_csv_table_offers_title_headers_and_quoted_prose_only() {
        let source = ".. csv-table:: **Current references**\n   :header: \"Title\", \"Brief\", \"Author\", \"Version\"\n\n    \"Guide\", \"Parser docs\", Louie Lu, 3.15\n";
        let doc = RstParser.parse(source);
        let sources: Vec<&str> = doc
            .translatable_segments()
            .iter()
            .map(|segment| segment.source.as_str())
            .collect();
        assert_eq!(
            sources,
            vec![
                "**Current references**",
                "Title",
                "Brief",
                "Author",
                "Version",
                "Guide",
                "Parser docs",
            ]
        );
        assert!(!sources.contains(&"Louie Lu"));
        assert!(!sources.contains(&"3.15"));
    }

    #[test]
    fn csv_translation_escapes_ascii_quotes_inside_quoted_field() {
        let source =
            ".. csv-table::\n   :header: \"Title\", \"Brief\"\n\n    \"Guide\", \"Parser docs\"\n";
        let output = translate_all(
            &RstParser,
            source,
            &[("Guide", "안내서"), ("Parser docs", "파서 \"문서\"")],
        );
        assert!(output.contains("\"안내서\", \"파서 \"\"문서\"\"\""));
    }

    #[test]
    fn file_backed_csv_table_stays_opaque() {
        let source = ".. csv-table::\n   :header-rows: 1\n   :file: include/branches.csv\n";
        assert!(RstParser.parse(source).translatable_segments().is_empty());
    }

    #[test]
    fn malformed_unquoted_csv_quote_stays_opaque() {
        let source = ".. csv-table::\n   :header: \"Title\", \"Brief\"\n\n    bad\"csv, \"prose\"\n";
        assert!(RstParser.parse(source).translatable_segments().is_empty());
    }

    #[test]
    fn unequal_width_or_url_backed_csv_stays_opaque() {
        let unequal_widths = ".. csv-table::\n   :header: \"Title\", \"Brief\"\n\n    \"Guide\"\n";
        let url_backed = ".. csv-table::\n   :url: https://example.com/branches.csv\n";
        assert!(RstParser.parse(unequal_widths).translatable_segments().is_empty());
        assert!(RstParser.parse(url_backed).translatable_segments().is_empty());
    }

    #[test]
    fn opaque_csv_before_inline_csv_keeps_later_table_translation() {
        let source = ".. csv-table::\n   :file: include/branches.csv\n\n.. csv-table::\n   :header: \"Title\", \"Brief\"\n\n    \"Guide\", \"Parser docs\"\n";
        let doc = RstParser.parse(source);
        let cell = doc.sections.iter().flat_map(|section| &section.blocks)
            .find(|block| matches!(block.role, BlockRole::TableCell { .. })).unwrap();
        assert!(matches!(cell.role, BlockRole::TableCell { table: 1, .. }));
        let output = translate_all(&RstParser, source, &[("Guide", "안내서")]);
        assert!(output.contains("\"안내서\", \"Parser docs\""));
    }

    #[test]
    fn translated_grid_table_is_redrawn_to_display_width() {
        let source = "+----+----+\n| a  | b  |\n+====+====+\n| c  | d  |\n+----+----+\n\nAfter.\n";
        let output = translate_all(
            &RstParser,
            source,
            &[("a", "가나"), ("b", "b"), ("c", "c"), ("d", "d"), ("After.", "이후.")],
        );
        assert_eq!(
            output,
            "+------+----+\n| 가나 | b  |\n+======+====+\n| c    | d  |\n+------+----+\n\n이후.\n"
        );
    }

    #[test]
    fn translated_grid_table_restores_each_line_block_marker() {
        let source = "+----+----------+\n| x  | | first  |\n|    | | second |\n+----+----------+\n";
        let output = translate_all(
            &RstParser,
            source,
            &[("x", "x"), ("| first | second", "| one two | three")],
        );

        assert!(output.lines().filter(|line| line.contains("| | ")).count() >= 2);
        assert!(!output.contains("two |"));
    }

    #[test]
    fn untranslated_table_stays_byte_for_byte() {
        let source = "+----+----+\n| a  | b  |\n+----+----+\n\nAfter.\n";
        let doc = RstParser.parse(source);
        let mut translations = TranslationMap::new();
        for seg in doc.translatable_segments() {
            if seg.source == "After." {
                translations.insert(seg.id.clone(), "이후.".to_string());
            }
        }
        assert_eq!(
            RstParser.reconstruct(&doc, &translations),
            "+----+----+\n| a  | b  |\n+----+----+\n\n이후.\n"
        );
    }

    #[test]
    fn simple_table_cells_become_segments() {
        let source = "=====  =====\ncol A  col B\n=====  =====\nx      y\n=====  =====\n\nAfter.\n";
        let doc = RstParser.parse(source);
        let sources: Vec<&str> = doc
            .translatable_segments()
            .iter()
            .map(|s| s.source.as_str())
            .collect();
        assert_eq!(sources, vec!["col A", "col B", "x", "y", "After."]);
    }

    #[test]
    fn translated_simple_table_is_redrawn() {
        let source = "=====  =====\ncol A  col B\n=====  =====\nx      y\n=====  =====\n\nAfter.\n";
        let output = translate_all(
            &RstParser,
            source,
            &[("col A", "열 하나"), ("col B", "col B"), ("x", "x"), ("y", "y")],
        );
        assert_eq!(
            output,
            "=======  =====\n열 하나  col B\n=======  =====\nx        y\n=======  =====\n\nAfter.\n"
        );
    }

    #[test]
    fn spanned_table_falls_back_to_verbatim() {
        // A column span the geometry cannot redraw: the table stays one
        // opaque block with no translatable cells, as before.
        let source = "+----+----+\n| spanning |\n+----+----+\n\nAfter.\n";
        let doc = RstParser.parse(source);
        let segments = doc.translatable_segments();
        assert_eq!(segments.len(), 1);
        assert_eq!(segments[0].source, "After.");
        let output = translate_all(&RstParser, source, &[("After.", "이후.")]);
        assert_eq!(output, "+----+----+\n| spanning |\n+----+----+\n\n이후.\n");
    }

    #[test]
    fn two_tables_get_distinct_indices() {
        let source = "==  ==\na1  b1\n==  ==\n\n==  ==\na2  b2\n==  ==\n\nAfter.\n";
        let doc = RstParser.parse(source);
        let tables: Vec<usize> = doc
            .sections
            .iter()
            .flat_map(|s| &s.blocks)
            .filter_map(|b| match &b.role {
                BlockRole::TableCell { table, .. } => Some(*table),
                _ => None,
            })
            .collect();
        assert_eq!(tables, vec![0, 0, 1, 1]);

        // Translating a cell of the second table redraws only that table; the
        // last column is unbounded, so its border keeps the original length.
        let output = translate_all(&RstParser, source, &[("b2", "둘째")]);
        assert_eq!(
            output,
            "==  ==\na1  b1\n==  ==\n\n==  ==\na2  둘째\n==  ==\n\nAfter.\n"
        );
    }

    #[test]
    fn footnote_text_is_translatable() {
        let source = ".. [1] The footnote text.\n";
        let doc = RstParser.parse(source);
        let segments = doc.translatable_segments();
        assert_eq!(segments.len(), 1);
        assert_eq!(segments[0].source, "The footnote text.");
        let output = translate_all(&RstParser, source, &[("The footnote text.", "각주.")]);
        assert_eq!(output, ".. [1] 각주.\n");
    }

    /// With no indented line to continue on, a footnote ends where it began:
    /// a column-zero line after it is no title underline or paragraph line.
    #[test]
    fn a_footnote_without_continuation_ends_on_its_line() {
        let doc = RstParser.parse(".. [1] Footnote text.\n---------------------\n");
        let segments = doc.translatable_segments();
        assert_eq!(segments.len(), 1);
        assert_eq!(segments[0].block_type, BlockType::Paragraph);

        let doc = RstParser.parse(".. [1] X.\n>>> print(1)\n1\n");
        assert_eq!(
            doc.translatable_segments()
                .iter()
                .map(|s| s.source.as_str())
                .collect::<Vec<_>>(),
            vec!["X."]
        );
    }

    #[test]
    fn footnote_text_continues_on_its_indented_lines() {
        let source = ".. [2] `'where' statement in Python\n   <https://example.com/x>`__\n\n\
                      .. [3] Some long footnote text that\n   continues here.\n\n   A second paragraph.\n";
        let doc = RstParser.parse(source);
        let segments = doc.translatable_segments();
        assert_eq!(
            segments
                .iter()
                .map(|s| s.source.as_str())
                .collect::<Vec<_>>(),
            vec![
                "`'where' statement in Python <https://example.com/x>`__",
                "Some long footnote text that continues here.",
                "A second paragraph.",
            ]
        );
        let output = translate_all(
            &RstParser,
            source,
            &[(
                "Some long footnote text that continues here.",
                "여기서 이어지는 긴 각주입니다.",
            )],
        );
        assert_eq!(
            output,
            ".. [2] `'where' statement in Python\n   <https://example.com/x>`__\n\n\
             .. [3] 여기서 이어지는 긴 각주입니다.\n\n   A second paragraph.\n"
        );
    }

    #[test]
    fn segments_keep_inline_markup() {
        let doc = RstParser.parse(
            "Consult :source:`LICENSE` or the `PyPy website`_ for ``code`` and **bold**.\n",
        );
        let segments = doc.translatable_segments();
        assert_eq!(segments.len(), 1);
        assert_eq!(
            segments[0].source,
            "Consult :source:`LICENSE` or the `PyPy website`_ for ``code`` and **bold**."
        );
    }

    #[test]
    fn transition_stays_verbatim() {
        let source = "Before.\n\n----\n\nAfter.\n";
        let doc = RstParser.parse(source);
        let segments = doc.translatable_segments();
        assert_eq!(segments.len(), 2);
        let output = translate_all(&RstParser, source, &[("Before.", "전."), ("After.", "후.")]);
        assert_eq!(output, "전.\n\n----\n\n후.\n");
    }

    #[test]
    fn reconstruct_without_translations_is_identity() {
        let source = "Title\n=====\n\nBody text::\n\n    code\n\n* item\n\n.. note::\n\n   text\n";
        let doc = RstParser.parse(source);
        assert_eq!(RstParser.reconstruct(&doc, &TranslationMap::new()), source);
    }

    #[test]
    fn pep_parser_translates_only_title_and_body_prose() {
        let source = "PEP: 9999\nTitle: A Useful Proposal\nAuthor: Jane Example <jane@example.com>\nStatus: Draft\nType: Standards Track\nCreated: 01-Jan-2026\nPost-History:\n\nAbstract\n========\n\nThis proposal helps readers.\n";
        let doc = PepParser.parse_checked(source).unwrap();
        let segments = doc.translatable_segments();
        let sources: Vec<&str> = segments.iter().map(|segment| segment.source.as_str()).collect();
        assert_eq!(
            sources,
            vec!["A Useful Proposal", "Abstract", "This proposal helps readers."]
        );

        let mut translations = TranslationMap::new();
        for segment in segments {
            let translation = match segment.source.as_str() {
                "A Useful Proposal" => "유용한 제안",
                "Abstract" => "초록",
                "This proposal helps readers." => "이 제안은 독자에게 도움이 됩니다.",
                other => panic!("unexpected segment: {other}"),
            };
            translations.insert(segment.id.clone(), translation.to_string());
        }
        assert_eq!(
            PepParser.reconstruct(&doc, &translations),
            "PEP: 9999\nTitle: 유용한 제안\nAuthor: Jane Example <jane@example.com>\nStatus: Draft\nType: Standards Track\nCreated: 01-Jan-2026\nPost-History:\n\n초록\n====\n\n이 제안은 독자에게 도움이 됩니다.\n"
        );
    }

    #[test]
    fn pep_parser_rejects_non_pep_input() {
        let error = PepParser.parse_checked("Title: Not a PEP\n\nBody.\n").unwrap_err();
        assert!(error.0.contains("must begin"));
    }

    #[test]
    fn pep_parser_keeps_license_notice_verbatim() {
        let source = "PEP: 9999\nTitle: Example\nAuthor: Jane Example\nStatus: Draft\nType: Process\nCreated: 01-Jan-2026\nPost-History:\n\nBody text.\n\nCopyright\n=========\n\nThis document has been placed in the public domain.\n";
        let doc = PepParser.parse_checked(source).unwrap();
        let sources: Vec<&str> = doc
            .translatable_segments()
            .iter()
            .map(|segment| segment.source.as_str())
            .collect();
        assert_eq!(sources, vec!["Example", "Body text."]);
        assert_eq!(PepParser.reconstruct(&doc, &TranslationMap::new()), source);
    }

    #[test]
    fn pep_parser_preserves_a_referenced_heading_target_when_translated() {
        let source = "PEP: 9999\nTitle: Example\nAuthor: Jane Example\nStatus: Draft\nType: Process\nCreated: 01-Jan-2026\nPost-History:\n\nSee `Future Directions`_ below.\n\nFuture Directions\n=================\n\nMore details.\n";
        let doc = PepParser.parse_checked(source).unwrap();
        let heading = doc
            .translatable_segments()
            .into_iter()
            .find(|segment| segment.source == "Future Directions")
            .unwrap();
        let mut translations = TranslationMap::new();
        translations.insert(heading.id.clone(), "향후 방향".to_string());

        let rebuilt = PepParser.reconstruct(&doc, &translations);
        assert!(rebuilt.contains(".. _yeokja-pep-9999-target-"));
        assert!(rebuilt.contains("\n\n향후 방향\n========="));
        assert!(rebuilt.contains("See `Future Directions <yeokja-pep-9999-target-"));
    }

    #[test]
    fn pep_parser_matches_referenced_heading_targets_case_insensitively() {
        let source = "PEP: 9999\nTitle: Example\nAuthor: Jane Example\nStatus: Draft\nType: Process\nCreated: 01-Jan-2026\nPost-History:\n\nSee `Backward compatibility`_ below.\n\nBackward Compatibility\n======================\n\nMore details.\n";
        let doc = PepParser.parse_checked(source).unwrap();
        let heading = doc
            .translatable_segments()
            .into_iter()
            .find(|segment| segment.source == "Backward Compatibility")
            .unwrap();
        let mut translations = TranslationMap::new();
        translations.insert(heading.id.clone(), "하위 호환성".to_string());

        let rebuilt = PepParser.reconstruct(&doc, &translations);
        assert!(rebuilt.contains(".. _yeokja-pep-9999-target-"));
        assert!(rebuilt.contains("\n\n하위 호환성"));
        assert!(rebuilt.contains(
            "See `Backward Compatibility <yeokja-pep-9999-target-"
        ));
    }

    #[test]
    fn pep_parser_matches_referenced_heading_targets_across_line_breaks() {
        let source = "PEP: 9999\nTitle: Example\nAuthor: Jane Example\nStatus: Draft\nType: Process\nCreated: 01-Jan-2026\nPost-History:\n\nSee the `Docutils\nProject Model`_ below.\n\nDocutils Project Model\n======================\n\nMore details.\n";
        let doc = PepParser.parse_checked(source).unwrap();
        let heading = doc
            .translatable_segments()
            .into_iter()
            .find(|segment| segment.source == "Docutils Project Model")
            .unwrap();
        let mut translations = TranslationMap::new();
        translations.insert(heading.id.clone(), "Docutils 프로젝트 모델".to_string());

        let rebuilt = PepParser.reconstruct(&doc, &translations);
        assert!(rebuilt.contains(".. _yeokja-pep-9999-target-"));
        assert!(rebuilt.contains("\n\nDocutils 프로젝트 모델"));
        assert!(rebuilt.contains(
            "`Docutils Project Model <yeokja-pep-9999-target-"
        ));
    }

    #[test]
    fn pep_parser_preserves_the_plain_target_name_of_a_literal_heading() {
        let source = "PEP: 9999\nTitle: Example\nAuthor: Jane Example\nStatus: Draft\nType: Process\nCreated: 01-Jan-2026\nPost-History:\n\nSee the `ImageSize class`_.\n\n``ImageSize`` Class\n-------------------\n\nMore details.\n";
        let doc = PepParser.parse_checked(source).unwrap();
        let heading = doc
            .translatable_segments()
            .into_iter()
            .find(|segment| segment.source == "``ImageSize`` Class")
            .unwrap();
        let mut translations = TranslationMap::new();
        translations.insert(heading.id.clone(), "``ImageSize`` 클래스".to_string());

        let rebuilt = PepParser.reconstruct(&doc, &translations);
        assert!(rebuilt.contains(".. _yeokja-pep-9999-target-"));
        assert!(rebuilt.contains("\n\n``ImageSize`` 클래스"));
        assert!(rebuilt.contains("`ImageSize Class <yeokja-pep-9999-target-"));
    }

    #[test]
    fn pep_parser_places_an_anchor_before_an_overlined_heading() {
        let source = "PEP: 9999\nTitle: Example\nAuthor: Jane Example\nStatus: Draft\nType: Process\nCreated: 01-Jan-2026\nPost-History:\n\nSee `Fancy Heading`_.\n\n=============\nFancy Heading\n=============\n\nMore details.\n";
        let doc = PepParser.parse_checked(source).unwrap();
        let heading = doc
            .translatable_segments()
            .into_iter()
            .find(|segment| segment.source == "Fancy Heading")
            .unwrap();
        let mut translations = TranslationMap::new();
        translations.insert(heading.id.clone(), "화려한 제목".to_string());

        let rebuilt = PepParser.reconstruct(&doc, &translations);
        assert!(rebuilt.contains(".. _yeokja-pep-9999-target-"));
        assert!(rebuilt.contains("\n\n.. _yeokja-pep-9999-target-"));
        assert!(rebuilt.contains("\n\n===========\n화려한 제목\n==========="));
        assert!(!rebuilt.contains("===========\n.. _yeokja"));
    }

    #[test]
    fn pep_parser_preserves_the_plain_target_name_of_an_emphasized_heading() {
        let source = "PEP: 9999\nTitle: Example\nAuthor: Jane Example\nStatus: Draft\nType: Process\nCreated: 01-Jan-2026\nPost-History:\n\nSee the `Distutils register Command`_.\n\nDistutils *register* Command\n----------------------------\n\nMore details.\n";
        let doc = PepParser.parse_checked(source).unwrap();
        let heading = doc
            .translatable_segments()
            .into_iter()
            .find(|segment| segment.source == "Distutils *register* Command")
            .unwrap();
        let mut translations = TranslationMap::new();
        translations.insert(heading.id.clone(), "Distutils *register* 명령".to_string());

        let rebuilt = PepParser.reconstruct(&doc, &translations);
        assert!(rebuilt.contains(".. _yeokja-pep-9999-target-"));
        assert!(rebuilt.contains("\n\nDistutils *register* 명령"));
        assert!(rebuilt.contains(
            "`Distutils register Command <yeokja-pep-9999-target-"
        ));
    }

    #[test]
    fn pep_parser_preserves_a_literal_escaped_star_in_a_heading_target() {
        let source = "PEP: 9999\nTitle: Example\nAuthor: Jane Example\nStatus: Draft\nType: Process\nCreated: 01-Jan-2026\nPost-History:\n\nSee `Programming Without 'except \\*'`_.\n\nProgramming Without 'except \\*'\n--------------------------------\n\nMore details.\n";
        let doc = PepParser.parse_checked(source).unwrap();
        let heading = doc
            .translatable_segments()
            .into_iter()
            .find(|segment| segment.source == "Programming Without 'except \\*'")
            .unwrap();
        let mut translations = TranslationMap::new();
        translations.insert(
            heading.id.clone(),
            "'except \\*' 없는 프로그래밍".to_string(),
        );

        let rebuilt = PepParser.reconstruct(&doc, &translations);
        assert!(rebuilt.contains(".. _yeokja-pep-9999-target-"));
        assert!(rebuilt.contains("\n\n'except \\*' 없는 프로그래밍"));
        assert!(rebuilt.contains(
            "`Programming Without 'except \\*' <yeokja-pep-9999-target-"
        ));
    }

    #[test]
    fn reconstruction_repairs_a_korean_particle_after_a_split_embedded_link() {
        assert_eq!(
            repair_korean_reference_boundaries(
                "`DaCapo Benchmarks\nAnalysis <https://example.com>`_입니다."
            ),
            "`DaCapo Benchmarks\nAnalysis <https://example.com>`_\\ 입니다."
        );
        assert_eq!(
            repair_korean_reference_boundaries("참고문헌 [#named]_에서 설명합니다."),
            "참고문헌 [#named]_\\ 에서 설명합니다."
        );
    }

    #[test]
    fn pep_parser_does_not_add_an_anchor_to_an_unreferenced_heading() {
        let source = "PEP: 9999\nTitle: Example\nAuthor: Jane Example\nStatus: Draft\nType: Process\nCreated: 01-Jan-2026\nPost-History:\n\nFuture Directions\n=================\n\nMore details.\n";
        let doc = PepParser.parse_checked(source).unwrap();
        let heading = doc
            .translatable_segments()
            .into_iter()
            .find(|segment| segment.source == "Future Directions")
            .unwrap();
        let mut translations = TranslationMap::new();
        translations.insert(heading.id.clone(), "향후 방향".to_string());

        let rebuilt = PepParser.reconstruct(&doc, &translations);
        assert!(!rebuilt.contains(".. _`Future Directions`:"));
    }

    #[test]
    fn pep_parser_does_not_treat_a_suffix_of_another_reference_as_a_heading_reference() {
        let source = "PEP: 9999\nTitle: Example\nAuthor: Jane Example\nStatus: Draft\nType: Process\nCreated: 01-Jan-2026\nPost-History:\n\nSee git-svn_.\n\nsvn\n---\n\nMore details.\n";
        let doc = PepParser.parse_checked(source).unwrap();
        let heading = doc
            .translatable_segments()
            .into_iter()
            .find(|segment| segment.source == "svn")
            .unwrap();
        let mut translations = TranslationMap::new();
        translations.insert(heading.id.clone(), "서브버전".to_string());

        let rebuilt = PepParser.reconstruct(&doc, &translations);
        assert!(!rebuilt.contains(".. _`svn`:"));
    }

    #[test]
    fn pep_parser_does_not_duplicate_an_explicit_link_target_named_like_a_heading() {
        let source = "PEP: 9999\nTitle: Example\nAuthor: Jane Example\nStatus: Draft\nType: Process\nCreated: 01-Jan-2026\nPost-History:\n\nSee PyPy_.\n\n.. _PyPy: https://www.pypy.org/\n\nPyPy\n----\n\nMore details.\n";
        let doc = PepParser.parse_checked(source).unwrap();
        let heading = doc
            .translatable_segments()
            .into_iter()
            .find(|segment| segment.source == "PyPy")
            .unwrap();
        let mut translations = TranslationMap::new();
        translations.insert(heading.id.clone(), "파이파이".to_string());

        let rebuilt = PepParser.reconstruct(&doc, &translations);
        assert_eq!(rebuilt.matches(".. _PyPy:").count(), 1);
        assert!(!rebuilt.contains(".. _`PyPy`:"));
    }

    #[test]
    fn pep_parser_preserves_a_heading_target_used_by_an_embedded_alias() {
        let source = "PEP: 9999\nTitle: Example\nAuthor: Jane Example\nStatus: Draft\nType: Process\nCreated: 01-Jan-2026\nPost-History:\n\nSee `the old role <Python's BDFL_>`_.\n\nPython's BDFL\n=============\n\nMore details.\n";
        let doc = PepParser.parse_checked(source).unwrap();
        let heading = doc
            .translatable_segments()
            .into_iter()
            .find(|segment| segment.source == "Python's BDFL")
            .unwrap();
        let mut translations = TranslationMap::new();
        translations.insert(heading.id.clone(), "파이썬의 BDFL".to_string());

        let rebuilt = PepParser.reconstruct(&doc, &translations);
        assert!(rebuilt.contains(".. _yeokja-pep-9999-target-"));
        assert!(rebuilt.contains("\n\n파이썬의 BDFL"));
        assert!(rebuilt.contains("<yeokja-pep-9999-target-"));
    }

    #[test]
    fn rst_indirect_target_to_a_translated_heading_uses_the_plain_reference_form() {
        // docutils does not parse an embedded alias inside a hyperlink
        // target body, so ``.. __: `Title <label_>`_`` makes the whole string
        // the refname and Sphinx reports a missing indirect target.
        let source = "Intro\n=====\n\n* Automatic unlimited stack (must be emulated__ so far)\n\n.. __: `recursion depth limit`_\n\nRecursion depth limit\n~~~~~~~~~~~~~~~~~~~~~\n\nYou can use continulets.\n";
        let doc = RstParser.parse(source);
        let heading = doc
            .translatable_segments()
            .into_iter()
            .find(|segment| segment.source == "Recursion depth limit")
            .unwrap();
        let mut translations = TranslationMap::new();
        translations.insert(heading.id.clone(), "재귀 깊이 제한".to_string());

        let rebuilt = RstParser.reconstruct(&doc, &translations);
        let label = rebuilt
            .lines()
            .find_map(|line| line.strip_prefix(".. _yeokja-doc-"))
            .and_then(|rest| rest.strip_suffix(':'))
            .map(|rest| format!("yeokja-doc-{rest}"))
            .expect("synthetic label");
        assert!(rebuilt.contains(&format!("\n.. __: {label}_\n")), "{rebuilt}");
        assert!(!rebuilt.contains(".. __: `"), "{rebuilt}");
        assert!(rebuilt.contains("\n재귀 깊이 제한\n"));
    }

    #[test]
    fn rst_named_indirect_target_to_a_translated_heading_uses_the_plain_reference_form() {
        let source = "Intro\n=====\n\nSee `the details`_ and `How do I compile my own interpreters?`_.\n\n.. _the details: `How do I compile my own interpreters?`_\n\nHow do I compile my own interpreters?\n-------------------------------------\n\nLike this.\n";
        let doc = RstParser.parse(source);
        let heading = doc
            .translatable_segments()
            .into_iter()
            .find(|segment| segment.source.starts_with("How do I compile"))
            .unwrap();
        let mut translations = TranslationMap::new();
        translations.insert(
            heading.id.clone(),
            "인터프리터는 어떻게 컴파일합니까?".to_string(),
        );

        let rebuilt = RstParser.reconstruct(&doc, &translations);
        let target_line = rebuilt
            .lines()
            .find(|line| line.starts_with(".. _the details: "))
            .expect("named target line");
        assert!(
            target_line.starts_with(".. _the details: yeokja-doc-"),
            "{target_line}"
        );
        assert!(target_line.contains("-target-"), "{target_line}");
        assert!(target_line.ends_with('_'), "{target_line}");
        assert!(!target_line.contains('`'), "{target_line}");
        assert!(!target_line.contains('<'), "{target_line}");
        assert!(rebuilt.contains(
            "See `the details`_ and `How do I compile my own interpreters? <yeokja-doc-"
        ));
    }

    #[test]
    fn pep_parser_does_not_rewrite_a_heading_name_inside_an_embedded_alias() {
        // PEP 622: "Patterns" is a section title, but the only occurrences of
        // ``patterns_`` are the aliases of other sections. Matching by
        // substring produced a nested link such as
        // ``<capture `Patterns <yeokja-pep-0622-target-N_>`_>``.
        let source = "PEP: 622\nTitle: Example\nAuthor: Jane Example\nStatus: Draft\nType: Process\nCreated: 01-Jan-2026\nPost-History:\n\nPatterns\n========\n\nThe allowed patterns are described in detail below in the `patterns\n<allowed patterns_>`_ subsection.\n\nThe wildcard can be used (see the section on `capture_pattern <capture patterns_>`_)\nas a final pattern.\n\nAllowed patterns\n================\n\nDetails.\n\nCapture Patterns\n================\n\nMore details.\n";
        let doc = PepParser.parse_checked(source).unwrap();
        let mut translations = TranslationMap::new();
        for segment in doc.translatable_segments() {
            let translation = match segment.source.as_str() {
                "Patterns" => "패턴",
                "Allowed patterns" => "허용되는 패턴",
                "Capture Patterns" => "캡처 패턴",
                _ => continue,
            };
            translations.insert(segment.id.clone(), translation.to_string());
        }

        let rebuilt = PepParser.reconstruct(&doc, &translations);
        assert!(!rebuilt.contains("`Patterns <"), "{rebuilt}");
        assert!(!rebuilt.contains("_>`_>`_"), "{rebuilt}");
        assert!(
            rebuilt.contains("`patterns\n<yeokja-pep-0622-target-"),
            "{rebuilt}"
        );
        assert!(
            rebuilt.contains("`capture_pattern <yeokja-pep-0622-target-"),
            "{rebuilt}"
        );
        // "Patterns" itself is never referenced, so it gets no synthetic label.
        assert_eq!(
            rebuilt.matches(".. _yeokja-pep-0622-target-").count(),
            2,
            "{rebuilt}"
        );
    }

    #[test]
    fn pep_parser_rewrites_a_translated_embedded_alias_only_when_it_equals_a_heading() {
        let source = "PEP: 622\nTitle: Example\nAuthor: Jane Example\nStatus: Draft\nType: Process\nCreated: 01-Jan-2026\nPost-History:\n\nPatterns\n========\n\nSee the `patterns <allowed patterns_>`_ subsection and `Patterns`_.\n\nAllowed patterns\n================\n\nDetails.\n";
        let doc = PepParser.parse_checked(source).unwrap();
        let mut translations = TranslationMap::new();
        for segment in doc.translatable_segments() {
            let translation = match segment.source.as_str() {
                "Patterns" => "패턴",
                "Allowed patterns" => "허용되는 패턴",
                source if source.starts_with("See the") => {
                    "`패턴 <allowed patterns_>`_ 하위 절과 `Patterns`_\\ 를 보십시오."
                }
                _ => continue,
            };
            translations.insert(segment.id.clone(), translation.to_string());
        }

        let rebuilt = PepParser.reconstruct(&doc, &translations);
        assert!(rebuilt.contains("`패턴 <yeokja-pep-0622-target-"), "{rebuilt}");
        assert!(
            rebuilt.contains("`Patterns <yeokja-pep-0622-target-"),
            "{rebuilt}"
        );
        assert!(!rebuilt.contains("_>`_>`_"), "{rebuilt}");
        assert_eq!(
            rebuilt.matches(".. _yeokja-pep-0622-target-").count(),
            2,
            "{rebuilt}"
        );
    }

    #[test]
    fn pep_parser_preserves_a_reused_embedded_link_target() {
        let source = "PEP: 9999\nTitle: Example\nAuthor: Jane Example\nStatus: Draft\nType: Process\nCreated: 01-Jan-2026\nPost-History:\n\nSee the `PEPs repository`_.\n\nThe source is in the `PEPs repository <https://github.com/python/peps/>`_.\n";
        let doc = PepParser.parse_checked(source).unwrap();
        let link = doc
            .translatable_segments()
            .into_iter()
            .find(|segment| segment.source.contains("source is in"))
            .unwrap();
        let mut translations = TranslationMap::new();
        translations.insert(
            link.id.clone(),
            "소스는 `PEP 저장소 <https://github.com/python/peps/>`_\\ 에 있습니다."
                .to_string(),
        );

        let rebuilt = PepParser.reconstruct(&doc, &translations);
        assert!(rebuilt.contains(
            "Post-History:\n\n.. _`PEPs repository`: https://github.com/python/peps/\n\n"
        ));
        assert!(rebuilt.contains("`PEPs repository`_"));
        assert!(rebuilt.contains("`PEP 저장소 <https://github.com/python/peps/>`_"));
    }

    #[test]
    fn pep_parser_preserves_a_reused_multiline_embedded_link_target() {
        let source = "PEP: 9999\nTitle: Example\nAuthor: Jane Example\nStatus: Draft\nType: Process\nCreated: 01-Jan-2026\nPost-History:\n\nSee the `Python recipe 576540`_.\n\n* `Python recipe\n  576540 <https://example.com/576540/>`_ by Jane.\n";
        let doc = PepParser.parse_checked(source).unwrap();
        let link = doc
            .translatable_segments()
            .into_iter()
            .find(|segment| segment.source.starts_with("`Python recipe"))
            .unwrap();
        let mut translations = TranslationMap::new();
        translations.insert(
            link.id.clone(),
            "`Python 레시피 576540 <https://example.com/576540/>`_".to_string(),
        );

        let rebuilt = PepParser.reconstruct(&doc, &translations);
        assert!(rebuilt.contains(".. _`Python recipe 576540`: https://example.com/576540/"));
    }

    #[test]
    fn plaintext_pep_parser_exposes_the_outer_literal_as_prose() {
        let source = "PEP: 9\nTitle: Plaintext Template\nAuthor: Jane Example\nStatus: Withdrawn\nType: Process\nCreated: 01-Jan-2001\nPost-History:\n\n::\n\n  Abstract\n\n      Reader-facing prose.\n\n  Copyright\n  =========\n\n  This document has been placed in the public domain.\n";
        let doc = PepPlaintextParser.parse_checked(source).unwrap();
        let sources: Vec<&str> = doc
            .translatable_segments()
            .iter()
            .map(|segment| segment.source.as_str())
            .collect();
        assert_eq!(sources, vec!["Plaintext Template", "Abstract", "Reader-facing prose."]);
        assert_eq!(PepPlaintextParser.reconstruct(&doc, &TranslationMap::new()), source);
    }

    #[test]
    fn text_block_pep_parser_exposes_selected_aphorisms_but_not_python() {
        let source = "PEP: 20\nTitle: Example\nAuthor: Jane Example\nStatus: Active\nType: Informational\nCreated: 01-Jan-2001\nPost-History:\n\n.. code-block:: text\n\n    Beautiful is better than ugly.\n    Explicit is better than implicit.\n\n.. code-block:: pycon\n\n    >>> import this\n\nCopyright\n=========\n\nThis document has been placed in the public domain.\n";
        let doc = PepTextBlockParser.parse_checked(source).unwrap();
        let sources: Vec<&str> = doc
            .translatable_segments()
            .iter()
            .map(|segment| segment.source.as_str())
            .collect();
        assert_eq!(
            sources,
            vec![
                "Example",
                "Beautiful is better than ugly.",
                "Explicit is better than implicit."
            ]
        );
        assert_eq!(PepTextBlockParser.reconstruct(&doc, &TranslationMap::new()), source);

        let mut translations = TranslationMap::new();
        for segment in doc.translatable_segments() {
            let translation = match segment.source.as_str() {
                "Beautiful is better than ugly." => "추한 것보다 아름다운 것이 낫다.",
                "Explicit is better than implicit." => "암시적인 것보다 명시적인 것이 낫다.",
                _ => continue,
            };
            translations.insert(segment.id.clone(), translation.to_string());
        }
        assert!(
            PepTextBlockParser
                .reconstruct(&doc, &translations)
                .contains("    추한 것보다 아름다운 것이 낫다.\n    암시적인 것보다 명시적인 것이 낫다.\n")
        );
    }

    #[test]
    fn nested_directive_inside_note_body() {
        let source = ".. note::\n\n   Prose here.\n\n   .. code-block:: python\n\n      x = 1\n\nAfter.\n";
        let doc = RstParser.parse(source);
        let segments = doc.translatable_segments();
        assert_eq!(segments.len(), 2);
        assert_eq!(segments[0].source, "Prose here.");
        assert_eq!(segments[1].source, "After.");
    }

    #[test]
    fn titles_split_sections() {
        let doc = RstParser.parse("One\n===\n\nA.\n\nTwo\n===\n\nB.\n");
        assert_eq!(doc.sections.len(), 2);
    }
}
