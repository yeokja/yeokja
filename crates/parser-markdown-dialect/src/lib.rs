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

/// How a flavour reads its source.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ParseOptions {
    /// Preserve explicit heading ids outside translated title spans.
    pub preserve_heading_ids: bool,
    /// Keep phrasing-level HTML tags (`<sub>`, `<dfn>`, `<a>`, …) inside the
    /// prose of an HTML block instead of cutting the prose at every tag, so
    /// that a Markdown list item holding one is read whole. Off by default:
    /// it changes the segments of a document that writes such tags.
    pub inline_html: bool,
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
    options: ParseOptions,
    /// Flavour-found blocks not yet emitted, in document order.
    extras: VecDeque<Extra>,
    sections: Vec<Section>,
    section_idx: usize,
    block_idx: usize,
    stack: Vec<Frame>,
    run: Option<Range<usize>>,
    /// End of an inline container whose run a hard break cut short; the next
    /// run opened inside the same block is extended to it.
    resume_end: Option<usize>,
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
        // An HTML comment inside a sentence (`as of <!-- date-check --> March
        // 2023`) stays in it: cutting the paragraph there made two blocks,
        // translated apart, of one sentence. The comment is markup the
        // translation carries over, as an inline tag where tags are used.
        let inline_comment = self.run.is_some()
            && self.source[range.clone()].starts_with("<!--")
            && self.source[range.clone()].trim_end().ends_with("-->")
            && !self.source[range.clone()].contains('\n');
        if !inline_comment && self.html_opaque.iter().any(|r| r.start < range.end && range.start < r.end) {
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
                if let Some(end) = self.resume_end.take() {
                    range.end = range.end.max(end);
                }
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
        if self.options.preserve_heading_ids && matches!(self.stack.last(), Some(Frame::Heading(_))) {
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
        for prose in html::prose_ranges(raw, self.options.inline_html) {
            let prose = range.start + prose.start..range.start + prose.end;
            for extra in prose_extras_with(self.source, prose, self.options) {
                self.push_extra(extra);
                found_prose = true;
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
    parse_with_options(parsed, source, ParseOptions { preserve_heading_ids, inline_html: false }, extras)
}

/// [`parse_with`] with every option spelled out.
pub fn parse_with_options(parsed: &str, source: &str, options: ParseOptions, extras: Vec<Extra>) -> Document {
    parse_events(parsed, source, options, extras, true)
}

/// The prose of `source[range]` read as Markdown on its own, as the blocks a
/// flavour hands to [`parse_with`] as extras: ranges index `source`.
///
/// Prose inside an HTML element is usually indented as a whole. Read with
/// that indentation, a line four spaces deeper than the first is an indented
/// code block, so the common indentation of the non-blank lines is removed
/// from every line first (the first prose line loses all of its leading
/// whitespace, as before), and the spans found are mapped back through a
/// per-line offset table. A span that crosses lines keeps the continuation
/// lines' indentation in the source bytes it covers; `normalize_inline_text`
/// folds it away in the segment text.
pub fn prose_extras(source: &str, range: Range<usize>, preserve_heading_ids: bool) -> Vec<Extra> {
    prose_extras_with(source, range, ParseOptions { preserve_heading_ids, inline_html: false })
}

/// [`prose_extras`] with every option spelled out.
pub fn prose_extras_with(source: &str, range: Range<usize>, options: ParseOptions) -> Vec<Extra> {
    let dedented = dedent(&source[range.clone()]);
    let nested = parse_events(&dedented.text, &dedented.text, options, Vec::new(), false);
    nested
        .sections
        .into_iter()
        .flat_map(|section| section.blocks)
        .filter_map(|block| {
            let span = block.span?;
            Some(Extra {
                range: range.start + dedented.source_offset(span.start)..range.start + dedented.source_offset(span.end),
                block_type: block.block_type,
                literal: false,
            })
        })
        .collect()
}

struct Dedented {
    text: String,
    /// `(start in text, start in the original)` of each line kept.
    lines: Vec<(usize, usize)>,
}

impl Dedented {
    fn source_offset(&self, at: usize) -> usize {
        let i = self.lines.partition_point(|&(text_start, _)| text_start <= at).saturating_sub(1);
        let (text_start, source_start) = self.lines[i];
        source_start + (at - text_start)
    }
}

/// `raw` without the common leading whitespace of its non-blank lines, the
/// leading blank lines, all leading whitespace of the first prose line and
/// trailing whitespace.
fn dedent(raw: &str) -> Dedented {
    let indent_of = |line: &str| line.len() - line.trim_start().len();
    let common = raw
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(indent_of)
        .min()
        .unwrap_or(0);
    let mut text = String::with_capacity(raw.len());
    let mut lines = Vec::new();
    let mut first = true;
    let mut source_start = 0;
    for line in raw.split_inclusive('\n') {
        let is_blank = line.trim().is_empty();
        if first && is_blank {
            source_start += line.len();
            continue;
        }
        let strip = if first { indent_of(line) } else { common.min(indent_of(line)) };
        first = false;
        lines.push((text.len(), source_start + strip));
        text.push_str(&line[strip..]);
        source_start += line.len();
    }
    let trimmed = text.trim_end().len();
    text.truncate(trimmed);
    Dedented { text, lines }
}

/// The byte ranges of `source` that a Markdown parser treats as raw HTML
/// to keep: literal elements (`<pre>`, `<code>`, `<style>`, …) with their
/// content, comments, CDATA sections and processing instructions.
pub fn html_opaque_ranges(source: &str) -> Vec<Range<usize>> {
    html::opaque_ranges(source, options())
}

/// The raw HTML blocks of `parsed`, as pulldown-cmark cuts them: from the
/// opening line to the blank line (or closing condition) that ends each.
pub fn html_block_ranges(parsed: &str) -> Vec<Range<usize>> {
    Parser::new_ext(parsed, options())
        .into_offset_iter()
        .filter_map(|(event, range)| matches!(event, Event::Start(Tag::HtmlBlock)).then_some(range))
        .collect()
}

/// The pulldown-cmark options every Markdown-family parser reads with, for a
/// flavour that inspects the event stream itself.
pub fn markdown_options() -> Options {
    options()
}

fn options() -> Options {
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
    options
}

fn parse_events(parsed: &str, source: &str, parse_options: ParseOptions, extras: Vec<Extra>, translate_html: bool) -> Document {
    debug_assert_eq!(parsed.len(), source.len());
    let options = options();

    let mut state = ParseState {
        source,
        options: parse_options,
        extras: extras.into(),
        sections: vec![Section { blocks: Vec::new() }],
        section_idx: 0,
        block_idx: 0,
        stack: Vec::new(),
        run: None,
        resume_end: None,
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
                    state.resume_end = None;
                    state.maybe_start_section(level);
                    state.stack.push(Frame::Heading(heading_level_num(level)));
                }
                Tag::Paragraph => {
                    state.flush_run();
                    state.resume_end = None;
                    state.stack.push(Frame::Paragraph);
                }
                Tag::Item => {
                    state.flush_run();
                    state.resume_end = None;
                    state.stack.push(Frame::Item);
                }
                Tag::BlockQuote(_) => {
                    state.flush_run();
                    state.resume_end = None;
                    state.stack.push(Frame::BlockQuote);
                }
                Tag::TableCell => {
                    state.flush_run();
                    state.resume_end = None;
                    state.stack.push(Frame::TableCell);
                }
                Tag::CodeBlock(_) | Tag::MetadataBlock(_) | Tag::HtmlBlock => {
                    state.flush_run();
                    state.resume_end = None;
                    state.stack.push(Frame::Opaque);
                }
                Tag::List(_) | Tag::Table(_) | Tag::TableHead | Tag::TableRow
                | Tag::FootnoteDefinition(_) => {
                    state.flush_run();
                    state.resume_end = None;
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
                    state.resume_end = None;
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
                // Inside an inline container the run already reaches the
                // container's end; cut it at the break and remember that end.
                if let Some(run) = state.run.as_mut().filter(|r| r.end > range.start) {
                    state.resume_end = Some(run.end);
                    run.end = range.start;
                }
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

    /// Bikeshed notes indent their prose by two spaces and a list inside by
    /// four. Read with the first line's indentation only, the list lines
    /// keep four spaces and become an indented code block.
    #[test]
    fn html_prose_loses_its_common_indentation_before_the_nested_parse() {
        let source = "<div class=note>\n  Note text\n  on two lines.\n    * First bullet with |n|.\n    * Second bullet.\n</div>\n";
        let doc = parse_with(source, source, true, Vec::new());
        assert_eq!(
            doc.translatable_segments().iter().map(|s| s.source.as_str()).collect::<Vec<_>>(),
            ["Note text on two lines.", "First bullet with |n|.", "Second bullet."]
        );
        for block in doc.sections.iter().flat_map(|s| &s.blocks) {
            if let Some(span) = &block.span {
                assert_eq!(&source[span.clone()], block.raw_content, "span indexes the source");
            }
        }
        assert_eq!(yeokja_parser_utils::splice_reconstruct(&doc, &Default::default()), source);
        let translations = doc
            .translatable_segments()
            .iter()
            .map(|s| (s.id.clone(), "번역".to_string()))
            .collect();
        assert_eq!(
            yeokja_parser_utils::splice_reconstruct(&doc, &translations),
            "<div class=note>\n  번역\n    * 번역\n    * 번역\n</div>\n"
        );
    }

    /// Inside an `<ol class=algorithm>` a list item holds `<sub>` tags.
    /// Cut at every tag, the items after the first tag collapse into one
    /// paragraph; with `inline_html` the tags stay in the prose and the
    /// nested parse sees whole items.
    #[test]
    fn inline_html_keeps_tags_inside_list_items() {
        let source = "<ol class=algorithm>\n    1. If |b|[0] is C1<sub>16</sub>, let |n| be a name\n        whose bytes are |b|.\n    2. Let |e|<sub>|i|</sub> be `x<sub>1</sub>` for each |i|.\n    3. Return |n| as <code>n<sub>0</sub></code>.\n</ol>\n";
        let cut = parse_with(source, source, true, Vec::new());
        assert!(cut.translatable_segments().iter().any(|s| s.source == "16"), "cut at the tag by default");
        let options = ParseOptions { preserve_heading_ids: true, inline_html: true };
        let doc = parse_with_options(source, source, options, Vec::new());
        assert_eq!(
            doc.translatable_segments().iter().map(|s| s.source.as_str()).collect::<Vec<_>>(),
            [
                "If |b|[0] is C1<sub>16</sub>, let |n| be a name whose bytes are |b|.",
                "Let |e|<sub>|i|</sub> be `x<sub>1</sub>` for each |i|.",
                "Return |n| as",
                "."
            ],
            "a code span stays whole; <code> content is literal and cuts the prose"
        );
        assert!(doc.translatable_segments()[..2].iter().all(|s| s.block_type == BlockType::ListItem));
        assert_eq!(yeokja_parser_utils::splice_reconstruct(&doc, &Default::default()), source);
        let identity = doc.translatable_segments().iter().map(|s| (s.id.clone(), s.source.clone())).collect();
        assert_eq!(yeokja_parser_utils::splice_reconstruct(&doc, &identity), source.replace("a name\n        whose", "a name whose"));
    }

    /// A phrasing tag alone on its line would read as an HTML block in the
    /// nested parse, so it still cuts the prose; so does an element that
    /// opens its chunk (`<td><dfn>name</dfn>`), whose text was a segment of
    /// its own before.
    #[test]
    fn an_inline_tag_alone_on_its_line_or_opening_its_chunk_still_cuts() {
        let source = "<div>\n<span class=x>\nText here.\n</span>\n</div>\n<table>\n<tr>\n  <td><dfn for=type>u8</dfn>\n  <td>Base 2<sup>8</sup>\n  <td>\n    <a href=x>Only a link</a>\n</table>\n";
        let options = ParseOptions { preserve_heading_ids: true, inline_html: true };
        let doc = parse_with_options(source, source, options, Vec::new());
        assert_eq!(
            doc.translatable_segments().iter().map(|s| s.source.as_str()).collect::<Vec<_>>(),
            ["Text here.", "u8", "Base 2<sup>8</sup>", "Only a link"]
        );
        assert_eq!(yeokja_parser_utils::splice_reconstruct(&doc, &Default::default()), source);
    }

    #[test]
    fn prose_extras_map_dedented_spans_back_to_the_source() {
        let source = "x\n    1. One.\n        Still one.\n    2. Two.\n";
        let extras = prose_extras(source, 2..source.len(), true);
        let texts: Vec<&str> = extras.iter().map(|e| &source[e.range.clone()]).collect();
        assert_eq!(texts, ["One.\n        Still one.", "Two."]);
    }
}
