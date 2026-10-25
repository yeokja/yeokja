//! XML documents read through a vocabulary: XHTML content documents (EPUB),
//! OpenStax CNXML and PreTeXt. See
//! `docs/superpowers/specs/2026-10-22-xhtml-parser-design.md` and
//! `docs/superpowers/specs/2026-10-25-xml-vocabulary-profiles-design.md`.
//!
//! A translation unit is a run of text and inline elements inside a block
//! element; a block child ends the run. Reconstruction splices translations
//! into the source, so everything outside the runs stays byte for byte.

mod entities;
mod vocab;
pub mod xml;

pub use vocab::{BlockKind, InlineKind, Vocabulary};

use std::collections::HashMap;
use std::ops::Range;
use xml::{Element, Node, XmlError};
use yeokja_core::hash::content_hash;
use yeokja_core::model::{Block, BlockRole, BlockType, Document, Section, Segment, SegmentId};
use yeokja_core::parser::{DocumentParseError, DocumentParser, Markup, TranslationMap};
use yeokja_parser_utils::{apply_splices, split_sentences};

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
    /// Text translated on its own inside markup kept as it is: an
    /// attribute value inside its start tag, or a MathML `mtext` inside its
    /// `math`. Translating it rewrites the host.
    Inner {
        host: Range<usize>,
        value: Range<usize>,
    },
}

/// Where a block element sits.
#[derive(Clone, Default)]
struct Ctx {
    /// Sectioning elements around it.
    depth: u8,
    /// Whether its parent is a sectioning element.
    in_section: bool,
}

struct Walk {
    vocab: Vocabulary,
    items: Vec<Item>,
    tables: usize,
    table: Option<usize>,
    row: usize,
    column: usize,
    /// Inside a table section of label rows (`thead`).
    head_rows: bool,
}

/// The translatable units of `source`, in document order.
fn analyze(vocab: Vocabulary, source: &str) -> Result<Vec<Item>, XmlError> {
    let nodes = xml::parse(source)?;
    let mut walk = Walk {
        vocab,
        items: Vec::new(),
        tables: 0,
        table: None,
        row: 0,
        column: 0,
        head_rows: false,
    };
    for node in &nodes {
        if let Node::Element(e) = node {
            walk.block(source, e, &Ctx::default())?;
        }
    }
    Ok(walk.items)
}

impl Walk {
    fn block(&mut self, source: &str, e: &Element, ctx: &Ctx) -> Result<(), XmlError> {
        let vocab = self.vocab;
        let name = e.local();
        if vocab.is_untranslated(e, source) {
            return Ok(());
        }
        if let Some(title) = vocab.head_title(name) {
            for child in &e.children {
                if let Node::Element(t) = child
                    && t.local() == title
                {
                    let nodes: Vec<&Node> = t.children.iter().collect();
                    self.run(source, &nodes, BlockType::Heading, None, BlockRole::None)?;
                }
            }
            return Ok(());
        }
        self.attrs(source, e);
        let mut role = BlockRole::None;
        if vocab.is_table(name) {
            self.table = Some(self.tables);
            self.tables += 1;
            self.row = 0;
        } else if vocab.is_row(name) {
            self.column = 0;
        } else if vocab.is_cell(name) {
            role = self
                .table
                .map(|table| BlockRole::TableCell {
                    table,
                    column: self.column,
                    label_row: self.row == 0 || self.head_rows,
                    header: None,
                })
                .unwrap_or_default();
            self.column += 1;
        }
        let (block_type, level) = match vocab.block_kind(name) {
            BlockKind::Heading(level) => (BlockType::Heading, level),
            BlockKind::Title => {
                let level = if ctx.in_section {
                    ctx.depth
                } else {
                    ctx.depth.saturating_add(1)
                };
                (BlockType::Heading, Some(level.max(1)))
            }
            BlockKind::ListItem => (BlockType::ListItem, None),
            BlockKind::TableCell => (BlockType::Table, None),
            BlockKind::BlockQuote => (BlockType::BlockQuote, None),
            BlockKind::Paragraph => (BlockType::Paragraph, None),
        };
        // An inline element walked only because it holds a block is
        // transparent: what it holds sits where the element does.
        let inner = if vocab.is_container(e) {
            ctx.clone()
        } else {
            let sectioning = vocab.is_sectioning(name);
            Ctx {
                depth: ctx.depth.saturating_add(u8::from(sectioning)),
                in_section: sectioning,
            }
        };
        let head_rows = self.head_rows;
        if vocab.is_table_head(name) {
            self.head_rows = true;
        }
        let mut run: Vec<&Node> = Vec::new();
        for child in &e.children {
            match child {
                Node::Element(c) if vocab.breaks_run(c, source) => {
                    self.run(source, &run, block_type, level, role.clone())?;
                    run.clear();
                    self.block(source, c, &inner)?;
                }
                other => run.push(other),
            }
        }
        self.run(source, &run, block_type, level, role)?;
        self.head_rows = head_rows;
        if vocab.is_row(name) {
            self.row += 1;
        }
        if vocab.is_table(name) {
            self.table = None;
        }
        Ok(())
    }

    /// The translated attributes of `e` itself.
    fn attrs(&mut self, source: &str, e: &Element) {
        for attr in &e.attrs {
            if self.vocab.translates_attr(&attr.name) && has_letter(&source[attr.value.clone()]) {
                self.items.push(Item::Inner {
                    host: e.start.clone(),
                    value: attr.value.clone(),
                });
            }
        }
    }

    /// The translated attributes and islands inside a run's inline elements.
    fn inline_attrs<'a>(&mut self, source: &str, nodes: impl IntoIterator<Item = &'a Node>) {
        for node in nodes {
            if let Node::Element(e) = node {
                if self.vocab.is_untranslated(e, source) {
                    if let Some(island) = self.vocab.island(e.local()) {
                        self.islands(source, e.range(), &e.children, island);
                    }
                    continue;
                }
                self.attrs(source, e);
                self.inline_attrs(source, &e.children);
            }
        }
    }

    /// The text of every `island` element under a skipped element, each
    /// translated on its own; the spaces around it stay.
    fn islands(&mut self, source: &str, host: Range<usize>, nodes: &[Node], island: &str) {
        for node in nodes {
            let Node::Element(e) = node else { continue };
            if e.local() != island {
                self.islands(source, host.clone(), &e.children, island);
                continue;
            }
            if e.children.iter().any(|n| !matches!(n, Node::Text(_))) {
                continue;
            }
            let inner = e.inner();
            let text = &source[inner.clone()];
            let lead = text.len()
                - text
                    .trim_start_matches(|c: char| c.is_ascii_whitespace())
                    .len();
            let trail = text.len()
                - text
                    .trim_end_matches(|c: char| c.is_ascii_whitespace())
                    .len();
            if lead + trail >= text.len() || !has_letter(text) {
                continue;
            }
            self.items.push(Item::Inner {
                host: host.clone(),
                value: inner.start + lead..inner.end - trail,
            });
        }
    }

    fn run(
        &mut self,
        source: &str,
        nodes: &[&Node],
        block_type: BlockType,
        heading_level: Option<u8>,
        role: BlockRole,
    ) -> Result<(), XmlError> {
        let Some(span) = trimmed_span(source, nodes) else {
            return Ok(());
        };
        let mut chars = Vec::new();
        for node in nodes {
            project(self.vocab, source, node, 0, false, &mut chars)?;
        }
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
        self.inline_attrs(source, nodes.iter().copied());
        Ok(())
    }
}

fn has_letter(text: &str) -> bool {
    text.chars().any(char::is_alphabetic)
}

/// The run's bytes without leading and trailing whitespace.
fn trimmed_span(source: &str, nodes: &[&Node]) -> Option<Range<usize>> {
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
    vocab: Vocabulary,
    source: &str,
    node: &Node,
    depth: usize,
    code: bool,
    out: &mut Vec<Shown>,
) -> Result<(), XmlError> {
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
        // A display equation that ends its sentence (`… = 0.</me> Then …`)
        // reads as the mark it holds, so the next sentence is a segment of
        // its own; the equation stays with the sentence it ends.
        Node::Element(e) if vocab.is_untranslated(e, source) => {
            if let Some(mark) = vocab.ends_sentence(e, source) {
                out.push(Shown {
                    c: mark,
                    bytes: e.range(),
                    depth,
                    code: false,
                });
            }
        }
        Node::Element(e) => {
            let code = code || vocab.is_code(e, source);
            for child in &e.children {
                project(vocab, source, child, depth + 1, code, out)?;
            }
        }
        Node::Other(_) => {}
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
            Item::Inner { value, .. } => (
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

fn parse_with(vocab: Vocabulary, source: &str) -> Result<Document, DocumentParseError> {
    let items = analyze(vocab, source)
        .map_err(|e| DocumentParseError(format!("not well-formed {}: {e}", vocab.name())))?;
    Ok(document(source, &items))
}

/// A block's text with its translations and the source's own whitespace
/// between its segments, or `None` when none of them is translated.
fn joined(
    source: &str,
    item: &Item,
    block: &Block,
    translations: &TranslationMap,
) -> Option<String> {
    if !block
        .segments
        .iter()
        .any(|seg| translations.contains_key(&seg.id))
    {
        return None;
    }
    let ranges = match item {
        Item::Run { segments, .. } => segments.as_slice(),
        Item::Inner { value, .. } => std::slice::from_ref(value),
    };
    let mut out = String::new();
    for (i, seg) in block.segments.iter().enumerate() {
        if i > 0 {
            match (ranges.get(i - 1), ranges.get(i)) {
                (Some(before), Some(after)) if ranges.len() == block.segments.len() => {
                    out.push_str(&source[before.end..after.start]);
                }
                _ => out.push(' '),
            }
        }
        out.push_str(translations.get(&seg.id).unwrap_or(&seg.source));
    }
    Some(out)
}

fn reconstruct_with(
    vocab: Vocabulary,
    document: &Document,
    translations: &TranslationMap,
) -> String {
    let source = &document.source;
    let Ok(items) = analyze(vocab, source) else {
        return source.clone();
    };
    let blocks: Vec<&Block> = document.sections.iter().flat_map(|s| &s.blocks).collect();
    if blocks.len() != items.len() {
        return yeokja_parser_utils::splice_reconstruct(document, translations);
    }
    // A translated attribute rewrites its whole start tag, and an `mtext`
    // its whole `math`, which a run's translation carries as source bytes
    // when the host is inside it.
    let mut values: HashMap<Range<usize>, Vec<(Range<usize>, String)>> = HashMap::new();
    for (item, block) in items.iter().zip(&blocks) {
        if let (Item::Inner { host, value }, Some(text)) =
            (item, joined(source, item, block, translations))
        {
            let local = value.start - host.start..value.end - host.start;
            values.entry(host.clone()).or_default().push((local, text));
        }
    }
    let hosts: HashMap<Range<usize>, String> = values
        .into_iter()
        .map(|(host, edits)| {
            let updated = apply_splices(&source[host.clone()], edits);
            (host, updated)
        })
        .collect();
    let mut splices = Vec::new();
    let mut placed: Vec<Range<usize>> = Vec::new();
    for (item, block) in items.iter().zip(&blocks) {
        let Item::Run { span, .. } = item else {
            continue;
        };
        let Some(mut text) = joined(source, item, block, translations) else {
            continue;
        };
        for (host, updated) in &hosts {
            if span.start <= host.start && host.end <= span.end {
                text = text.replace(&source[host.clone()], updated);
                placed.push(host.clone());
            }
        }
        splices.push((span.clone(), text));
    }
    for (host, updated) in hosts {
        if !placed.contains(&host) {
            splices.push((host, updated));
        }
    }
    apply_splices(source, splices)
}

macro_rules! vocabulary_parser {
    ($(#[$doc:meta])* $parser:ident, $vocab:expr) => {
        $(#[$doc])*
        pub struct $parser;

        impl DocumentParser for $parser {
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
                parse_with($vocab, source)
            }

            fn reconstruct(&self, document: &Document, translations: &TranslationMap) -> String {
                reconstruct_with($vocab, document, translations)
            }
        }
    };
}

vocabulary_parser!(
    /// The `xhtml` parser: XHTML content documents.
    XhtmlParser,
    Vocabulary::Xhtml
);
vocabulary_parser!(
    /// The `cnxml` parser: OpenStax CNXML modules and collections.
    CnxmlParser,
    Vocabulary::Cnxml
);
vocabulary_parser!(
    /// The `pretext` parser: PreTeXt and MathBook XML.
    PretextParser,
    Vocabulary::Pretext
);

/// The element structure of an XHTML document; see [`skeleton_with`].
pub fn skeleton(source: &str) -> Result<String, XmlError> {
    skeleton_with(Vocabulary::Xhtml, source)
}

/// The element structure of `source` without its text or the values of the
/// translated attributes: block elements in order, and the inline elements
/// inside each block as a multiset — translation moves them with the words
/// they wrap, and may split an emphasis, which only has to be there. Two
/// documents with the same skeleton differ only in what translation changes.
pub fn skeleton_with(vocab: Vocabulary, source: &str) -> Result<String, XmlError> {
    fn head(vocab: Vocabulary, source: &str, e: &Element) -> String {
        let mut attrs: Vec<String> = e
            .attrs
            .iter()
            .map(|attr| {
                if vocab.translates_attr(&attr.name) {
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
    fn element(vocab: Vocabulary, source: &str, e: &Element) -> String {
        let mut blocks = Vec::new();
        let mut inline = Vec::new();
        collect(vocab, source, &e.children, &mut blocks, &mut inline);
        let mut counted: Vec<String> = Vec::new();
        let mut present: Vec<String> = Vec::new();
        for (emphasis, h) in inline {
            if emphasis {
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
            head(vocab, source, e),
            blocks.join(""),
            counted.join(""),
            present.join("")
        )
    }
    fn collect(
        vocab: Vocabulary,
        source: &str,
        nodes: &[Node],
        blocks: &mut Vec<String>,
        inline: &mut Vec<(bool, String)>,
    ) {
        for node in nodes {
            if let Node::Element(e) = node {
                if vocab.breaks_run(e, source) {
                    blocks.push(element(vocab, source, e));
                } else {
                    inline.push((vocab.is_emphasis(e, source), head(vocab, source, e)));
                    collect(vocab, source, &e.children, blocks, inline);
                }
            }
        }
    }
    let nodes = xml::parse(source)?;
    let mut blocks = Vec::new();
    let mut inline = Vec::new();
    collect(vocab, source, &nodes, &mut blocks, &mut inline);
    Ok(blocks.join(""))
}

#[cfg(test)]
mod tests;
