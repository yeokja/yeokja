//! Shared span extraction for Markdown-family translation parsers.
//! Dialects supply an offset-preserving shadow and additional prose spans.
mod html;

use pulldown_cmark::{Event, HeadingLevel, Options, Parser, Tag, TagEnd};
use std::collections::VecDeque;
use std::ops::Range;
use yeokja_core::model::*;

use yeokja_parser_utils::{
    make_segments, normalize_inline_text,
};

/// A block a flavour found on its own, outside what pulldown-cmark sees.
///
/// Emitted in document order among the Markdown blocks. A translatable
/// `block_type` offers `range` as the block's span; any other keeps the bytes
/// verbatim.
pub struct Extra {
    pub range: Range<usize>,
    pub block_type: BlockType,
    /// The container quotes the text rather than parsing it as markup
    /// ([`BlockRole::Literal`]).
    pub literal: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Frame {
    Heading(u8),
    Paragraph,
    Item,
    BlockQuote,
    TableCell,
    /// Any container whose text content must not be translated (code blocks,
    /// metadata blocks, raw HTML blocks).
    Opaque,
}

struct ParseState<'a> {
    /// The text blocks are cut from. Offsets from the parsed text index into
    /// it, so a flavour may parse a shadow of the source as long as the two
    /// have the same length.
    source: &'a str,
    /// Preserve explicit heading ids outside translated title spans.
    preserve_heading_ids: bool,
    /// Flavour-found blocks not yet emitted, in document order.
    extras: VecDeque<Extra>,
    sections: Vec<Section>,
    section_idx: usize,
    block_idx: usize,
    stack: Vec<Frame>,
    run: Option<Range<usize>>,
    html_opaque: Vec<Range<usize>>,
}

impl ParseState<'_> {
    fn in_opaque(&self) -> bool {
        self.stack.contains(&Frame::Opaque)
    }

    /// Emit every pending extra that starts before `offset`, so it lands
    /// among the Markdown blocks in document order.
    fn emit_extras_before(&mut self, offset: usize) {
        while self.extras.front().is_some_and(|extra| extra.range.start < offset) {
            let extra = self.extras.pop_front().unwrap();
            self.push_extra(extra);
        }
    }

    fn push_extra(&mut self, extra: Extra) {
        if self.html_opaque.iter().any(|r| r.start < extra.range.end && extra.range.start < r.end) {
            return;
        }
        if !extra.block_type.is_translatable() {
            self.push_opaque_block(extra.block_type, extra.range);
            return;
        }
        let raw = &self.source[extra.range.clone()];
        let normalized = normalize_inline_text(raw);
        let segments = make_segments(&normalized, extra.block_type, self.section_idx, self.block_idx);
        self.push_block(Block {
            block_type: extra.block_type,
            segments,
            raw_content: raw.to_string(),
            heading_level: None,
            span: Some(extra.range),
            translatable: true,
            role: if extra.literal { BlockRole::Literal } else { BlockRole::None },
        });
    }

    /// Shrink a heading run so an explicit id stays outside the span, verbatim
    /// after the translated title: MDX `\{#id}`/`{#id}` and Python-Markdown
    /// attr_list `{ #id }`/`{: #id .class }`, together with an ATX closing
    /// sequence written before it (`Title ### {#id}`).
    fn strip_heading_id(&self, span: Range<usize>) -> Range<usize> {
        let raw = &self.source[span.clone()];
        let trimmed = raw.trim_end();
        let Some(open) = trimmed.strip_suffix('}').and_then(|s| s.rfind('{')) else {
            return span;
        };
        let inner = trimmed[open + 1..trimmed.len() - 1].trim();
        let inner = inner.strip_prefix(':').unwrap_or(inner).trim();
        let has_id = inner.split_whitespace().any(|t| t.len() > 1 && t.starts_with('#'));
        if !has_id {
            return span;
        }
        let mut cut = open;
        if trimmed[..cut].ends_with('\\') {
            cut -= 1;
        }
        let mut title = trimmed[..cut].trim_end();
        let without_hashes = title.trim_end_matches('#');
        if without_hashes.len() < title.len() && without_hashes.ends_with(char::is_whitespace) {
            title = without_hashes.trim_end();
        }
        span.start..span.start + title.len()
    }

    fn current_block_type(&self) -> (BlockType, Option<u8>) {
        for frame in self.stack.iter().rev() {
            match frame {
                Frame::Heading(level) => return (BlockType::Heading, Some(*level)),
                Frame::TableCell => return (BlockType::Table, None),
                Frame::Item => return (BlockType::ListItem, None),
                Frame::BlockQuote => return (BlockType::BlockQuote, None),
                Frame::Opaque => return (BlockType::CodeBlock, None),
                // A paragraph inherits the label of its enclosing container
                // (blockquote, list item), so keep scanning outward.
                Frame::Paragraph => continue,
            }
        }
        (BlockType::Paragraph, None)
    }

    fn extend_run(&mut self, range: Range<usize>) {
        if self.html_opaque.iter().any(|r| r.start < range.end && range.start < r.end) {
            self.flush_run();
            return;
        }
        if self.in_opaque() {
            return;
        }
        match &mut self.run {
            Some(run) => run.end = run.end.max(range.end),
            None => {
                let mut range = range;
                if starts_after_escape(self.source.as_bytes(), range.start) {
                    range.start -= 1;
                }
                self.run = Some(range);
            }
        }
    }

    /// Close the current inline run and emit it as a translatable block.
    fn flush_run(&mut self) {
        let Some(mut span) = self.run.take() else { return };
        if self.preserve_heading_ids && matches!(self.stack.last(), Some(Frame::Heading(_))) {
            span = self.strip_heading_id(span);
        }
        let raw = &self.source[span.clone()];
        if raw.trim().is_empty() {
            return;
        }
        let (block_type, heading_level) = self.current_block_type();
        if !block_type.is_translatable() {
            return;
        }
        let normalized = normalize_inline_text(raw);
        let segments = make_segments(&normalized, block_type, self.section_idx, self.block_idx);
        self.push_block(Block {
            block_type,
            segments,
            raw_content: raw.to_string(),
            heading_level,
            span: Some(span),
            translatable: block_type.is_translatable(),
            role: BlockRole::None,
        });
    }

    fn push_block(&mut self, block: Block) {
        self.sections.last_mut().unwrap().blocks.push(block);
        self.block_idx += 1;
    }

    fn push_html_block(&mut self, range: Range<usize>) {
        let raw = &self.source[range.clone()];
        let mut found_prose = false;
        for prose in html::prose_ranges(raw) {
            let untrimmed = &raw[prose.clone()];
            let text = untrimmed.trim();
            let leading = untrimmed.len() - untrimmed.trim_start().len();
            let nested = parse_events(text, text, self.preserve_heading_ids, Vec::new(), false);
            for block in nested.sections.into_iter().flat_map(|section| section.blocks) {
                if let Some(span) = block.span {
                    let offset = range.start + prose.start + leading;
                    self.push_extra(Extra {
                        range: offset + span.start..offset + span.end,
                        block_type: block.block_type,
                        literal: false,
                    });
                    found_prose = true;
                }
            }
        }
        if !found_prose {
            self.push_opaque_block(BlockType::HtmlBlock, range);
        }
    }

    fn push_opaque_block(&mut self, block_type: BlockType, range: Range<usize>) {
        let raw = self.source[range].to_string();
        self.push_block(Block {
            block_type,
            segments: Vec::new(),
            raw_content: raw,
            heading_level: None,
            span: None,
            translatable: block_type.is_translatable(),
            role: BlockRole::None,
        });
    }

    fn maybe_start_section(&mut self, level: HeadingLevel) {
        let splits_section = matches!(level, HeadingLevel::H1 | HeadingLevel::H2);
        if splits_section && !self.sections.last().unwrap().blocks.is_empty() {
            self.sections.push(Section { blocks: Vec::new() });
            self.section_idx += 1;
            self.block_idx = 0;
        }
    }
}

/// pulldown-cmark reports an escaped character (`\[`) without its backslash.
/// A run that starts there must take the backslash along, or the splice
/// leaves it in front of the translation.
fn starts_after_escape(bytes: &[u8], at: usize) -> bool {
    if at == 0 || bytes[at - 1] != b'\\' || !bytes.get(at).is_some_and(u8::is_ascii_punctuation) {
        return false;
    }
    let backslashes = bytes[..at].iter().rev().take_while(|&&b| b == b'\\').count();
    backslashes % 2 == 1
}

fn heading_level_num(level: HeadingLevel) -> u8 {
    match level {
        HeadingLevel::H1 => 1,
        HeadingLevel::H2 => 2,
        HeadingLevel::H3 => 3,
        HeadingLevel::H4 => 4,
        HeadingLevel::H5 => 5,
        HeadingLevel::H6 => 6,
    }
}

/// Run the Markdown event loop over `parsed`, cutting block text from
/// `source`, which must have the same length (a flavour blanks the regions
/// it handles itself and lists them as `extras`).
pub fn parse_with(parsed: &str, source: &str, preserve_heading_ids: bool, extras: Vec<Extra>) -> Document {
    parse_events(parsed, source, preserve_heading_ids, extras, true)
}

fn parse_events(parsed: &str, source: &str, preserve_heading_ids: bool, extras: Vec<Extra>, translate_html: bool) -> Document {
    debug_assert_eq!(parsed.len(), source.len());
    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_TASKLISTS);
    options.insert(Options::ENABLE_FOOTNOTES);
    options.insert(Options::ENABLE_YAML_STYLE_METADATA_BLOCKS);
    // GitHub alerts (`> [!NOTE]`): the marker becomes the blockquote's kind
    // instead of prose, so it stays out of the segment and keeps its own line
    // when the translation is spliced back. A marker joined onto the prose
    // line is no longer an alert to GitHub or mdBook.
    options.insert(Options::ENABLE_GFM);

    let mut state = ParseState {
        source,
        preserve_heading_ids,
        extras: extras.into(),
        sections: vec![Section { blocks: Vec::new() }],
        section_idx: 0,
        block_idx: 0,
        stack: Vec::new(),
        run: None,
        html_opaque: if translate_html { html::opaque_ranges(parsed, options) } else { Vec::new() },
    };

    for (event, range) in Parser::new_ext(parsed, options).into_offset_iter() {
        // Extras sit in regions Markdown sees as blank, so none starts inside
        // a run: emitting them at the next event keeps document order.
        state.emit_extras_before(range.start);
        match event {
            Event::Start(tag) => match tag {
                Tag::Heading { level, .. } => {
                    state.flush_run();
                    state.maybe_start_section(level);
                    state.stack.push(Frame::Heading(heading_level_num(level)));
                }
                Tag::Paragraph => {
                    state.flush_run();
                    state.stack.push(Frame::Paragraph);
                }
                Tag::Item => {
                    state.flush_run();
                    state.stack.push(Frame::Item);
                }
                Tag::BlockQuote(_) => {
                    state.flush_run();
                    state.stack.push(Frame::BlockQuote);
                }
                Tag::TableCell => {
                    state.flush_run();
                    state.stack.push(Frame::TableCell);
                }
                Tag::CodeBlock(_) | Tag::MetadataBlock(_) | Tag::HtmlBlock => {
                    state.flush_run();
                    state.stack.push(Frame::Opaque);
                }
                Tag::List(_) | Tag::Table(_) | Tag::TableHead | Tag::TableRow
                | Tag::FootnoteDefinition(_) => {
                    state.flush_run();
                }
                // Inline containers: their full range (including markers) is
                // part of the surrounding run.
                Tag::Emphasis | Tag::Strong | Tag::Strikethrough
                | Tag::Link { .. } | Tag::Image { .. } => {
                    state.extend_run(range);
                }
                _ => {}
            },
            Event::End(tag_end) => match tag_end {
                TagEnd::Heading(_)
                | TagEnd::Paragraph
                | TagEnd::Item
                | TagEnd::BlockQuote(_)
                | TagEnd::TableCell => {
                    state.flush_run();
                    state.stack.pop();
                }
                TagEnd::CodeBlock => {
                    state.stack.pop();
                    state.push_opaque_block(BlockType::CodeBlock, range);
                }
                TagEnd::MetadataBlock(_) => {
                    state.stack.pop();
                    state.push_opaque_block(BlockType::HtmlBlock, range);
                }
                TagEnd::HtmlBlock => {
                    state.stack.pop();
                    if translate_html {
                        state.push_html_block(range);
                    } else {
                        state.push_opaque_block(BlockType::HtmlBlock, range);
                    }
                }
                _ => {}
            },
            Event::Text(_)
            | Event::Code(_)
            | Event::InlineMath(_)
            | Event::DisplayMath(_)
            | Event::InlineHtml(_)
            | Event::FootnoteReference(_) => {
                state.extend_run(range);
            }
            // Soft breaks only extend an already-started run; they never start one.
            Event::SoftBreak => {
                if state.run.is_some() {
                    state.extend_run(range);
                }
            }
            // A hard break (trailing spaces or backslash) is semantic: end the
            // run so the break bytes stay outside spans and survive splicing.
            Event::HardBreak => {
                state.flush_run();
            }
            Event::Rule => {
                state.flush_run();
                state.push_opaque_block(BlockType::ThematicBreak, range);
            }
            Event::Html(_) => {
                // Block-level HTML outside an HtmlBlock tag (rare); keep verbatim.
                state.flush_run();
            }
            Event::TaskListMarker(_) => {}
        }
    }
    state.flush_run();
    state.emit_extras_before(usize::MAX);

    let mut sections = state.sections;
    sections.retain(|s| !s.blocks.is_empty());

    Document {
        sections,
        source: source.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn heading_sources(source: &str) -> Vec<String> {
        parse_with(source, source, true, Vec::new())
            .translatable_segments()
            .iter()
            .map(|s| s.source.clone())
            .collect()
    }

    #[test]
    fn attr_list_ids_stay_outside_heading_span() {
        assert_eq!(heading_sources("## Implementation { #implementation }\n"), ["Implementation"]);
        assert_eq!(heading_sources("## Title {: #tid .cls }\n"), ["Title"]);
        assert_eq!(heading_sources("## Title {#tid}\n"), ["Title"]);
        assert_eq!(heading_sources("## Title \\{#tid}\n"), ["Title"]);
    }

    #[test]
    fn closing_hashes_before_attr_list_stay_outside() {
        assert_eq!(heading_sources("## Implementation ### { #implementation}\n"), ["Implementation"]);
    }

    /// cp-algorithms puts an anchor `<div>` right above a heading, so the
    /// heading line is part of the HTML block and reaches the nested parse.
    #[test]
    fn heading_id_inside_an_html_block_stays_outside() {
        assert_eq!(
            heading_sources("<div id=\"old\"></div>\n### Title {: #tid }\n\nBody.\n"),
            ["Title", "Body."]
        );
    }

    #[test]
    fn braces_without_an_id_are_title_text() {
        assert_eq!(heading_sources("## Sets {a, b}\n"), ["Sets {a, b}"]);
    }
}
