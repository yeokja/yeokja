//! XHTML content documents (EPUB) read as XML. See
//! `docs/superpowers/specs/2026-10-22-xhtml-parser-design.md`.
//!
//! A translation unit is a run of text and inline elements inside a block
//! element; a block child ends the run. Reconstruction splices translations
//! into the source, so everything outside the runs stays byte for byte.

mod entities;
pub mod xml;

use std::collections::HashMap;
use std::ops::Range;
use xml::{Element, Node, XmlError};
use yeokja_core::hash::content_hash;
use yeokja_core::model::{Block, BlockRole, BlockType, Document, Section, Segment, SegmentId};
use yeokja_core::parser::{DocumentParseError, DocumentParser, Markup, TranslationMap};
use yeokja_parser_utils::{apply_splices, join_segments_with_translations, split_sentences};

pub struct XhtmlParser;

/// Elements that end a run of inline content.
const BLOCK: &[&str] = &[
    "address",
    "article",
    "aside",
    "blockquote",
    "body",
    "caption",
    "center",
    "dd",
    "details",
    "dialog",
    "dir",
    "div",
    "dl",
    "dt",
    "fieldset",
    "figcaption",
    "figure",
    "footer",
    "form",
    "h1",
    "h2",
    "h3",
    "h4",
    "h5",
    "h6",
    "head",
    "header",
    "hgroup",
    "hr",
    "html",
    "legend",
    "li",
    "main",
    "menu",
    "nav",
    "noscript",
    "ol",
    "p",
    "pre",
    "section",
    "summary",
    "table",
    "tbody",
    "td",
    "tfoot",
    "th",
    "thead",
    "tr",
    "ul",
];

/// Subtrees never offered for translation.
pub const SKIP: &[&str] = &[
    "pre", "script", "style", "math", "svg", "object", "iframe", "template", "textarea", "select",
];

/// Inline elements whose text is code: shown to the model, never changed.
pub const CODE: &[&str] = &["code", "kbd", "samp", "tt"];

/// Attributes whose values are translated.
const TRANSLATED_ATTRS: &[&str] = &["alt", "title"];

fn is_block(e: &Element) -> bool {
    BLOCK.contains(&e.local())
}

/// Never translated, wherever it sits: a skipped element or one marked
/// `translate="no"`.
pub fn is_untranslated(e: &Element, source: &str) -> bool {
    SKIP.contains(&e.local()) || e.attr(source, "translate") == Some("no")
}

/// What one translatable unit is, before it becomes a [`Block`].
#[derive(Debug, Clone)]
enum Item {
    Run {
        span: Range<usize>,
        segments: Vec<Range<usize>>,
        block_type: BlockType,
        heading_level: Option<u8>,
        role: BlockRole,
    },
    /// A translated attribute: the tag it is in and its value.
    Attr {
        tag: Range<usize>,
        value: Range<usize>,
    },
}

#[derive(Default)]
struct Walk {
    items: Vec<Item>,
    tables: usize,
    table: Option<usize>,
    row: usize,
    column: usize,
}

/// The translatable units of `source`, in document order.
fn analyze(source: &str) -> Result<Vec<Item>, XmlError> {
    let nodes = xml::parse(source)?;
    let mut walk = Walk::default();
    for node in &nodes {
        if let Node::Element(e) = node {
            walk.block(source, e, None)?;
        }
    }
    Ok(walk.items)
}

impl Walk {
    fn block(
        &mut self,
        source: &str,
        e: &Element,
        cell: Option<BlockRole>,
    ) -> Result<(), XmlError> {
        let name = e.local();
        if is_untranslated(e, source) {
            return Ok(());
        }
        if name == "head" {
            for child in &e.children {
                if let Node::Element(title) = child
                    && title.local() == "title"
                {
                    self.run(
                        source,
                        &title.children,
                        BlockType::Heading,
                        None,
                        BlockRole::None,
                    )?;
                }
            }
            return Ok(());
        }
        self.attrs(source, e);
        let mut cell = cell;
        match name {
            "table" => {
                self.table = Some(self.tables);
                self.tables += 1;
                self.row = 0;
            }
            "tr" => self.column = 0,
            "td" | "th" => {
                cell = self.table.map(|table| BlockRole::TableCell {
                    table,
                    column: self.column,
                    label_row: self.row == 0,
                    header: None,
                });
                self.column += 1;
            }
            _ => {}
        }
        let (block_type, level) = match name {
            "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => (BlockType::Heading, name[1..].parse().ok()),
            "li" | "dt" | "dd" => (BlockType::ListItem, None),
            "td" | "th" => (BlockType::Table, None),
            "blockquote" => (BlockType::BlockQuote, None),
            _ => (BlockType::Paragraph, None),
        };
        let role = if matches!(name, "td" | "th") {
            cell.clone().unwrap_or_default()
        } else {
            BlockRole::None
        };
        let mut run: Vec<Node> = Vec::new();
        for child in &e.children {
            match child {
                Node::Element(c) if is_block(c) => {
                    self.run(source, &run, block_type, level, role.clone())?;
                    run.clear();
                    self.block(source, c, cell.clone())?;
                }
                other => run.push(other.clone()),
            }
        }
        self.run(source, &run, block_type, level, role)?;
        if name == "tr" {
            self.row += 1;
        }
        if name == "table" {
            self.table = None;
        }
        Ok(())
    }

    /// The translated attributes of `e` itself.
    fn attrs(&mut self, source: &str, e: &Element) {
        for attr in &e.attrs {
            if TRANSLATED_ATTRS.contains(&attr.name.as_str())
                && has_letter(&source[attr.value.clone()])
            {
                self.items.push(Item::Attr {
                    tag: e.start.clone(),
                    value: attr.value.clone(),
                });
            }
        }
    }

    /// The translated attributes inside a run's inline elements.
    fn inline_attrs(&mut self, source: &str, nodes: &[Node]) {
        for node in nodes {
            if let Node::Element(e) = node {
                if is_untranslated(e, source) {
                    continue;
                }
                self.attrs(source, e);
                self.inline_attrs(source, &e.children);
            }
        }
    }

    fn run(
        &mut self,
        source: &str,
        nodes: &[Node],
        block_type: BlockType,
        heading_level: Option<u8>,
        role: BlockRole,
    ) -> Result<(), XmlError> {
        let Some(span) = trimmed_span(source, nodes) else {
            return Ok(());
        };
        let mut chars = Vec::new();
        project(source, nodes, 0, false, &mut chars)?;
        if chars.iter().any(|c| c.c.is_alphabetic() && !c.code) {
            let segments = split(source, &span, &chars);
            self.items.push(Item::Run {
                span,
                segments,
                block_type,
                heading_level,
                role,
            });
        }
        self.inline_attrs(source, nodes);
        Ok(())
    }
}

fn has_letter(text: &str) -> bool {
    text.chars().any(char::is_alphabetic)
}

/// The run's bytes without leading and trailing whitespace.
fn trimmed_span(source: &str, nodes: &[Node]) -> Option<Range<usize>> {
    let start = nodes.first()?.range().start;
    let end = nodes.last()?.range().end;
    let text = &source[start..end];
    let lead = text.len()
        - text
            .trim_start_matches(|c: char| c.is_ascii_whitespace())
            .len();
    let trail = text.len()
        - text
            .trim_end_matches(|c: char| c.is_ascii_whitespace())
            .len();
    (start + lead < end - trail).then(|| start + lead..end - trail)
}

/// One character a reader sees, where it was written, and how deep in the
/// run's inline elements it sits.
#[derive(Debug, Clone)]
struct Shown {
    c: char,
    bytes: Range<usize>,
    depth: usize,
    code: bool,
}

fn project(
    source: &str,
    nodes: &[Node],
    depth: usize,
    code: bool,
    out: &mut Vec<Shown>,
) -> Result<(), XmlError> {
    for node in nodes {
        match node {
            Node::Text(range) => {
                xml::decode_each(source, range.clone(), |c, bytes| {
                    out.push(Shown {
                        c,
                        bytes,
                        depth,
                        code,
                    })
                })?;
            }
            Node::Element(e) if is_untranslated(e, source) => {}
            Node::Element(e) => {
                let code = code || CODE.contains(&e.local());
                project(source, &e.children, depth + 1, code, out)?;
            }
            Node::Other(_) => {}
        }
    }
    Ok(())
}

/// The run's segments: its sentences, cut only where no inline element is
/// open, so every segment is balanced markup.
fn split(source: &str, span: &Range<usize>, chars: &[Shown]) -> Vec<Range<usize>> {
    let shown: String = chars.iter().map(|c| c.c).collect();
    let starts: Vec<usize> = shown.char_indices().map(|(at, _)| at).collect();
    let index_of = |byte: usize| starts.partition_point(|&s| s < byte);
    let mut ends = Vec::new(); // char index of each sentence's last character
    let mut cursor = 0;
    for sentence in split_sentences(&shown) {
        let Some(at) = shown[cursor..].find(&sentence) else {
            return vec![span.clone()];
        };
        let end = cursor + at + sentence.len();
        ends.push(index_of(end) - 1);
        cursor = end;
    }
    let mut segments = Vec::new();
    let mut start = span.start;
    for &last in ends.iter().take(ends.len().saturating_sub(1)) {
        let mut at = chars[last].bytes.end;
        let mut depth = chars[last].depth;
        while depth > 0 && source[at..span.end].starts_with("</") {
            match source[at..].find('>') {
                Some(close) => at += close + 1,
                None => break,
            }
            depth -= 1;
        }
        let ws = source[at..span.end].len()
            - source[at..span.end]
                .trim_start_matches(|c: char| c.is_ascii_whitespace())
                .len();
        if depth != 0 || ws == 0 || at + ws >= span.end {
            continue;
        }
        segments.push(start..at);
        start = at + ws;
    }
    segments.push(start..span.end);
    segments
}

fn document(source: &str, items: &[Item]) -> Document {
    let mut sections: Vec<Section> = vec![Section { blocks: Vec::new() }];
    for item in items {
        let (block_type, heading_level, role, span, ranges) = match item {
            Item::Run {
                span,
                segments,
                block_type,
                heading_level,
                role,
            } => (
                *block_type,
                *heading_level,
                role.clone(),
                span.clone(),
                segments.clone(),
            ),
            Item::Attr { value, .. } => (
                BlockType::Paragraph,
                None,
                BlockRole::Literal,
                value.clone(),
                vec![value.clone()],
            ),
        };
        if block_type == BlockType::Heading && !sections.last().is_some_and(|s| s.blocks.is_empty())
        {
            sections.push(Section { blocks: Vec::new() });
        }
        let section_idx = sections.len() - 1;
        let blocks = &mut sections.last_mut().expect("a section").blocks;
        let block_idx = blocks.len();
        let segments = ranges
            .iter()
            .enumerate()
            .map(|(i, range)| {
                let text = source[range.clone()].to_string();
                Segment {
                    id: SegmentId::new(section_idx, block_idx, i),
                    source_hash: content_hash(&text),
                    source: text,
                    block_type,
                }
            })
            .collect();
        blocks.push(Block {
            block_type,
            segments,
            raw_content: source[span.clone()].to_string(),
            heading_level,
            span: Some(span),
            role,
            translatable: true,
        });
    }
    sections.retain(|s| !s.blocks.is_empty());
    Document {
        sections,
        source: source.to_string(),
    }
}

impl DocumentParser for XhtmlParser {
    fn markup(&self) -> Markup {
        Markup::Xhtml
    }

    fn parse(&self, source: &str) -> Document {
        self.parse_checked(source).unwrap_or_else(|_| Document {
            sections: Vec::new(),
            source: source.to_string(),
        })
    }

    fn parse_checked(&self, source: &str) -> Result<Document, DocumentParseError> {
        let items = analyze(source)
            .map_err(|e| DocumentParseError(format!("not well-formed XHTML: {e}")))?;
        Ok(document(source, &items))
    }

    fn reconstruct(&self, document: &Document, translations: &TranslationMap) -> String {
        let source = &document.source;
        let Ok(items) = analyze(source) else {
            return source.clone();
        };
        let blocks: Vec<&Block> = document.sections.iter().flat_map(|s| &s.blocks).collect();
        if blocks.len() != items.len() {
            return yeokja_parser_utils::splice_reconstruct(document, translations);
        }
        let joined = |block: &Block| {
            block
                .segments
                .iter()
                .any(|seg| translations.contains_key(&seg.id))
                .then(|| join_segments_with_translations(&block.segments, translations))
        };
        // A translated attribute rewrites its whole start tag, which a run's
        // translation carries as source bytes when the tag is inside it.
        let mut values: HashMap<Range<usize>, Vec<(Range<usize>, String)>> = HashMap::new();
        for (item, block) in items.iter().zip(&blocks) {
            if let (Item::Attr { tag, value }, Some(text)) = (item, joined(block)) {
                let local = value.start - tag.start..value.end - tag.start;
                values.entry(tag.clone()).or_default().push((local, text));
            }
        }
        let tags: HashMap<Range<usize>, String> = values
            .into_iter()
            .map(|(tag, edits)| {
                let updated = apply_splices(&source[tag.clone()], edits);
                (tag, updated)
            })
            .collect();
        let mut splices = Vec::new();
        let mut placed: Vec<Range<usize>> = Vec::new();
        for (item, block) in items.iter().zip(&blocks) {
            let Item::Run { span, .. } = item else {
                continue;
            };
            let Some(mut text) = joined(block) else {
                continue;
            };
            for (tag, updated) in &tags {
                if span.start <= tag.start && tag.end <= span.end {
                    text = text.replace(&source[tag.clone()], updated);
                    placed.push(tag.clone());
                }
            }
            splices.push((span.clone(), text));
        }
        for (tag, updated) in tags {
            if !placed.contains(&tag) {
                splices.push((tag, updated));
            }
        }
        apply_splices(source, splices)
    }
}

/// Emphasis that Korean word order may split in two.
const EMPHASIS: &[&str] = &["em", "i", "cite", "dfn", "var", "strong", "b"];

/// The element structure of `source` without its text or the values of the
/// translated attributes: block elements in order, and the inline elements
/// inside each block as a multiset — translation moves them with the words
/// they wrap, and may split an emphasis, which only has to be there. Two
/// documents with the same skeleton differ only in what translation changes.
pub fn skeleton(source: &str) -> Result<String, XmlError> {
    fn head(source: &str, e: &Element) -> String {
        let mut attrs: Vec<String> = e
            .attrs
            .iter()
            .map(|attr| {
                if TRANSLATED_ATTRS.contains(&attr.name.as_str()) {
                    attr.name.clone()
                } else {
                    format!(
                        "{}=\"{}\"",
                        attr.name,
                        xml::decode(source, attr.value.clone()).unwrap_or_default()
                    )
                }
            })
            .collect();
        attrs.sort();
        format!("<{} {}>", e.name, attrs.join(" "))
    }
    fn element(source: &str, e: &Element) -> String {
        let mut blocks = Vec::new();
        let mut inline = Vec::new();
        collect(source, &e.children, &mut blocks, &mut inline);
        let mut counted: Vec<String> = Vec::new();
        let mut present: Vec<String> = Vec::new();
        for (name, h) in inline {
            if EMPHASIS.contains(&name.as_str()) {
                present.push(h)
            } else {
                counted.push(h)
            }
        }
        counted.sort();
        present.sort();
        present.dedup();
        format!(
            "{}[{}]{{{}}}{{{}}}",
            head(source, e),
            blocks.join(""),
            counted.join(""),
            present.join("")
        )
    }
    fn collect(
        source: &str,
        nodes: &[Node],
        blocks: &mut Vec<String>,
        inline: &mut Vec<(String, String)>,
    ) {
        for node in nodes {
            if let Node::Element(e) = node {
                if is_block(e) {
                    blocks.push(element(source, e));
                } else {
                    inline.push((e.local().to_string(), head(source, e)));
                    collect(source, &e.children, blocks, inline);
                }
            }
        }
    }
    let nodes = xml::parse(source)?;
    let mut blocks = Vec::new();
    let mut inline = Vec::new();
    collect(source, &nodes, &mut blocks, &mut inline);
    Ok(blocks.join(""))
}

#[cfg(test)]
mod tests;
