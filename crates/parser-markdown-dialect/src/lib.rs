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
            role: BlockRole::None,
        });
    }

    /// Shrink a heading run so an MDX custom id (`\{#id}` or `{#id}`) stays
    /// outside the span, verbatim after the translated title.
    fn strip_heading_id(&self, span: Range<usize>) -> Range<usize> {
        let raw = &self.source[span.clone()];
        let trimmed = raw.trim_end();
        let Some(open) = trimmed.strip_suffix('}').and_then(|s| s.rfind('{')) else {
            return span;
        };
        let inner = &trimmed[open + 1..trimmed.len() - 1];
        if !inner.starts_with('#') || inner.len() < 2 || inner.contains(char::is_whitespace) {
            return span;
        }
        let mut cut = open;
        if trimmed[..cut].ends_with('\\') {
            cut -= 1;
        }
        span.start..span.start + raw[..cut].trim_end().len()
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
            None => self.run = Some(range),
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
            let nested = parse_events(text, text, false, Vec::new(), false);
            for block in nested.sections.into_iter().flat_map(|section| section.blocks) {
                if let Some(span) = block.span {
                    let offset = range.start + prose.start + leading;
                    self.push_extra(Extra {
                        range: offset + span.start..offset + span.end,
                        block_type: block.block_type,
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
