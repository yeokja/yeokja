mod html;
mod mdx;
mod myst;

pub use mdx::MdxParser;
pub use myst::MystParser;

use pulldown_cmark::{Event, HeadingLevel, Options, Parser, Tag, TagEnd};
use std::collections::VecDeque;
use std::ops::Range;
use yeokja_core::model::*;
use yeokja_core::parser::{DocumentParser, Markup, TranslationMap};
use yeokja_parser_utils::{
    make_segments, normalize_inline_text, resolve_reference_links, splice_reconstruct,
};

/// A block a flavour found on its own, outside what pulldown-cmark sees.
///
/// Emitted in document order among the Markdown blocks. A translatable
/// `block_type` offers `range` as the block's span; any other keeps the bytes
/// verbatim.
pub(crate) struct Extra {
    pub(crate) range: Range<usize>,
    pub(crate) block_type: BlockType,
}

/// Span-based Markdown parser.
///
/// `parse` records the byte range of each block's inline content in the source.
/// Segments therefore carry the raw inline markdown (links, emphasis, code spans),
/// which lets evaluators verify markup preservation and lets the LLM see it.
/// `reconstruct` splices translations into the original source, leaving everything
/// outside the translated spans (code fences, list markers, blockquote prefixes,
/// front matter, blank lines) byte-for-byte intact.
pub struct MarkdownParser;

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
    /// Whether MDX conventions apply (custom heading ids).
    mdx: bool,
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
        if self.mdx && matches!(self.stack.last(), Some(Frame::Heading(_))) {
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

impl DocumentParser for MarkdownParser {
    fn markup(&self) -> Markup {
        Markup::Markdown
    }

    fn parse(&self, source: &str) -> Document {
        parse_with(source, source, false, Vec::new())
    }

    fn reconstruct(&self, document: &Document, translations: &TranslationMap) -> String {
        // Shortcut and collapsed reference links name their definition by
        // their text; rewrite them to the full form so they still resolve.
        let translations = resolve_reference_links(document, translations);
        splice_reconstruct(document, &translations)
    }
}

/// Run the Markdown event loop over `parsed`, cutting block text from
/// `source`, which must have the same length (a flavour blanks the regions
/// it handles itself and lists them as `extras`).
pub(crate) fn parse_with(parsed: &str, source: &str, mdx: bool, extras: Vec<Extra>) -> Document {
    parse_events(parsed, source, mdx, extras, true)
}

fn parse_events(parsed: &str, source: &str, mdx: bool, extras: Vec<Extra>, translate_html: bool) -> Document {
    debug_assert_eq!(parsed.len(), source.len());
    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_TASKLISTS);
    options.insert(Options::ENABLE_FOOTNOTES);
    options.insert(Options::ENABLE_YAML_STYLE_METADATA_BLOCKS);

    let mut state = ParseState {
        source,
        mdx,
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

    #[test]
    fn parse_simple_paragraph() {
        let parser = MarkdownParser;
        let doc = parser.parse("Hello world. Goodbye world.");
        assert_eq!(doc.sections.len(), 1);
        let segments = doc.translatable_segments();
        assert_eq!(segments.len(), 2);
        assert_eq!(segments[0].source, "Hello world.");
        assert_eq!(segments[1].source, "Goodbye world.");
    }

    #[test]
    fn parse_heading_starts_new_section() {
        let parser = MarkdownParser;
        let doc = parser.parse("# Chapter 1\n\nSome text.\n\n## Chapter 2\n\nMore text.");
        assert!(doc.sections.len() >= 2);
    }

    #[test]
    fn code_blocks_not_translatable() {
        let parser = MarkdownParser;
        let doc = parser.parse("Some text.\n\n```\nfn main() {}\n```\n\nMore text.");
        let translatable = doc.translatable_segments();
        for seg in &translatable {
            assert_ne!(seg.block_type, BlockType::CodeBlock);
        }
        assert_eq!(translatable.len(), 2);
    }

    #[test]
    fn segments_keep_inline_markup() {
        let parser = MarkdownParser;
        let doc = parser.parse(
            "Visit [the docs](https://example.com/docs) for **more details** about `git init`.",
        );
        let segments = doc.translatable_segments();
        assert_eq!(segments.len(), 1);
        assert_eq!(
            segments[0].source,
            "Visit [the docs](https://example.com/docs) for **more details** about `git init`."
        );
    }

    #[test]
    fn reconstruct_with_translations() {
        let parser = MarkdownParser;
        let doc = parser.parse("Hello world.");
        let segments = doc.translatable_segments();
        let mut translations = TranslationMap::new();
        translations.insert(segments[0].id.clone(), "안녕하세요.".to_string());

        let output = parser.reconstruct(&doc, &translations);
        assert_eq!(output, "안녕하세요.");
    }

    #[test]
    fn reconstruct_falls_back_to_original() {
        let parser = MarkdownParser;
        let doc = parser.parse("Hello world.");
        let translations = TranslationMap::new(); // empty

        let output = parser.reconstruct(&doc, &translations);
        assert!(output.contains("Hello world."));
    }

    #[test]
    fn reconstruct_preserves_code_fence_language() {
        let parser = MarkdownParser;
        let source = "Intro text.\n\n```sh\ngit init\n```\n\nOutro text.\n";
        let doc = parser.parse(source);
        let segments = doc.translatable_segments();
        let mut translations = TranslationMap::new();
        translations.insert(segments[0].id.clone(), "소개.".to_string());
        translations.insert(segments[1].id.clone(), "마무리.".to_string());

        let output = parser.reconstruct(&doc, &translations);
        assert_eq!(output, "소개.\n\n```sh\ngit init\n```\n\n마무리.\n");
    }

    #[test]
    fn reconstruct_preserves_list_markers() {
        let parser = MarkdownParser;
        let source = "- First item.\n- Second item.\n";
        let doc = parser.parse(source);
        let segments = doc.translatable_segments();
        assert_eq!(segments.len(), 2);
        assert_eq!(segments[0].block_type, BlockType::ListItem);

        let mut translations = TranslationMap::new();
        translations.insert(segments[0].id.clone(), "첫째.".to_string());
        translations.insert(segments[1].id.clone(), "둘째.".to_string());

        let output = parser.reconstruct(&doc, &translations);
        assert_eq!(output, "- 첫째.\n- 둘째.\n");
    }

    #[test]
    fn reconstruct_preserves_nested_list_structure() {
        let parser = MarkdownParser;
        let source = "- Parent item.\n  - Child item.\n";
        let doc = parser.parse(source);
        let segments = doc.translatable_segments();
        assert_eq!(segments.len(), 2);

        let mut translations = TranslationMap::new();
        translations.insert(segments[0].id.clone(), "부모.".to_string());
        translations.insert(segments[1].id.clone(), "자식.".to_string());

        let output = parser.reconstruct(&doc, &translations);
        assert_eq!(output, "- 부모.\n  - 자식.\n");
    }

    #[test]
    fn reconstruct_preserves_heading_markers() {
        let parser = MarkdownParser;
        let source = "# Title\n\nBody text.\n";
        let doc = parser.parse(source);
        let segments = doc.translatable_segments();
        let mut translations = TranslationMap::new();
        translations.insert(segments[0].id.clone(), "제목".to_string());
        translations.insert(segments[1].id.clone(), "본문.".to_string());

        let output = parser.reconstruct(&doc, &translations);
        assert_eq!(output, "# 제목\n\n본문.\n");
    }

    #[test]
    fn reconstruct_preserves_blockquote_prefix() {
        let parser = MarkdownParser;
        let source = "> Quoted text.\n";
        let doc = parser.parse(source);
        let segments = doc.translatable_segments();
        assert_eq!(segments.len(), 1);
        assert_eq!(segments[0].block_type, BlockType::BlockQuote);

        let mut translations = TranslationMap::new();
        translations.insert(segments[0].id.clone(), "인용문.".to_string());

        let output = parser.reconstruct(&doc, &translations);
        assert_eq!(output, "> 인용문.\n");
    }

    #[test]
    fn reconstruct_preserves_table_structure() {
        let parser = MarkdownParser;
        let source = "| Name | Desc |\n|------|------|\n| Repo | The repository. |\n";
        let doc = parser.parse(source);
        let segments = doc.translatable_segments();
        assert!(segments.iter().any(|s| s.source == "The repository."));

        let mut translations = TranslationMap::new();
        for seg in &segments {
            if seg.source == "The repository." {
                translations.insert(seg.id.clone(), "저장소.".to_string());
            }
        }

        let output = parser.reconstruct(&doc, &translations);
        assert!(output.contains("| Repo | 저장소. |"));
        assert!(output.contains("|------|------|"));
    }

    #[test]
    fn front_matter_not_translated() {
        let parser = MarkdownParser;
        let source = "---\ntitle: Hello\n---\n\nBody text.\n";
        let doc = parser.parse(source);
        let segments = doc.translatable_segments();
        assert_eq!(segments.len(), 1);
        assert_eq!(segments[0].source, "Body text.");

        let mut translations = TranslationMap::new();
        translations.insert(segments[0].id.clone(), "본문.".to_string());
        let output = parser.reconstruct(&doc, &translations);
        assert_eq!(output, "---\ntitle: Hello\n---\n\n본문.\n");
    }

    #[test]
    fn multiline_paragraph_normalized_to_single_segment_text() {
        let parser = MarkdownParser;
        let source = "This is one\nwrapped sentence.\n";
        let doc = parser.parse(source);
        let segments = doc.translatable_segments();
        assert_eq!(segments.len(), 1);
        assert_eq!(segments[0].source, "This is one wrapped sentence.");
    }

    #[test]
    fn task_list_marker_preserved() {
        let parser = MarkdownParser;
        let source = "- [x] Done task.\n- [ ] Open task.\n";
        let doc = parser.parse(source);
        let segments = doc.translatable_segments();
        assert_eq!(segments.len(), 2);
        assert_eq!(segments[0].source, "Done task.");

        let mut translations = TranslationMap::new();
        translations.insert(segments[0].id.clone(), "완료됨.".to_string());
        translations.insert(segments[1].id.clone(), "미완료.".to_string());
        let output = parser.reconstruct(&doc, &translations);
        assert_eq!(output, "- [x] 완료됨.\n- [ ] 미완료.\n");
    }

    #[test]
    fn hard_break_preserved() {
        let parser = MarkdownParser;
        let source = "First line.  \nSecond line.\n";
        let doc = parser.parse(source);
        let segments = doc.translatable_segments();
        assert_eq!(segments.len(), 2);
        assert_eq!(segments[0].source, "First line.");
        assert_eq!(segments[1].source, "Second line.");

        let mut translations = TranslationMap::new();
        translations.insert(segments[0].id.clone(), "첫 줄.".to_string());
        translations.insert(segments[1].id.clone(), "둘째 줄.".to_string());
        let output = parser.reconstruct(&doc, &translations);
        assert_eq!(output, "첫 줄.  \n둘째 줄.\n");
    }

    #[test]
    fn backslash_hard_break_preserved() {
        let parser = MarkdownParser;
        let source = "First line.\\\nSecond line.\n";
        let doc = parser.parse(source);
        let segments = doc.translatable_segments();
        assert_eq!(segments.len(), 2);

        let mut translations = TranslationMap::new();
        translations.insert(segments[0].id.clone(), "첫 줄.".to_string());
        translations.insert(segments[1].id.clone(), "둘째 줄.".to_string());
        let output = parser.reconstruct(&doc, &translations);
        assert_eq!(output, "첫 줄.\\\n둘째 줄.\n");
    }

    #[test]
    fn reference_links_keep_resolving_after_translation() {
        let parser = MarkdownParser;
        let source = "Flakes are [experimental] with a [standard structure].\n\n[Experimental]: https://a\n[standard structure]: https://b\n";
        let doc = parser.parse(source);
        let segments = doc.translatable_segments();
        let mut translations = TranslationMap::new();
        translations.insert(
            segments[0].id.clone(),
            "플레이크는 [실험적]이며 [표준 구조]를 가집니다.".to_string(),
        );
        assert_eq!(
            parser.reconstruct(&doc, &translations),
            "플레이크는 [실험적][experimental]이며 [표준 구조][standard structure]를 가집니다.\n\n[Experimental]: https://a\n[standard structure]: https://b\n"
        );
    }

    #[test]
    fn html_block_translates_prose_and_preserves_tags() {
        let parser = MarkdownParser;
        let source = "Text before.\n\n<div class=\"note\">\nraw html\n</div>\n\nText after.\n";
        let doc = parser.parse(source);
        let segments = doc.translatable_segments();
        assert_eq!(segments.len(), 3);

        let mut translations = TranslationMap::new();
        translations.insert(segments[0].id.clone(), "이전.".to_string());
        translations.insert(segments[1].id.clone(), "HTML 본문".to_string());
        translations.insert(segments[2].id.clone(), "이후.".to_string());
        let output = parser.reconstruct(&doc, &translations);
        assert_eq!(output, "이전.\n\n<div class=\"note\">\nHTML 본문\n</div>\n\n이후.\n");
    }
    #[test]
    fn html_warning_and_summary_are_translatable_without_blank_lines() {
        let source = "<div class=\"warning\">\nThis page is outdated.\n</div>\n\n<details><summary><b>An example</b></summary>\n\nBody text.\n\n</details>\n";
        let doc = MarkdownParser.parse(source);
        let segments = doc.translatable_segments();
        assert_eq!(segments.iter().map(|s| s.source.as_str()).collect::<Vec<_>>(),
            ["This page is outdated.", "An example", "Body text."]);
        let translations = segments.iter().map(|s| (s.id.clone(), "번역".to_string())).collect();
        assert_eq!(MarkdownParser.reconstruct(&doc, &translations),
            "<div class=\"warning\">\n번역\n</div>\n\n<details><summary><b>번역</b></summary>\n\n번역\n\n</details>\n");
    }

    #[test]
    fn html_literal_elements_comments_and_attributes_are_not_translated() {
        let source = "<div title=\"Keep > this\">\nVisible prose.<!-- Hidden text. -->\n<pre><code>Keep code.</code></pre>\n<script>if (a < b) { alert('Keep script.'); }</script>\n<style>.a { content: 'Keep style.'; }</style>\nMore prose.\n</div>";
        let doc = MarkdownParser.parse(source);
        assert_eq!(doc.translatable_segments().iter().map(|s| s.source.as_str()).collect::<Vec<_>>(),
            ["Visible prose.", "More prose."]);
        assert_eq!(MarkdownParser.reconstruct(&doc, &TranslationMap::new()), source);
    }

    #[test]
    fn html_prose_keeps_markdown_code_and_autolinks() {
        let source = "<div>\nRead **this** and `Vec<T>`.\n<https://example.com/a>\n</div>\n\n```html\n<div>Do not translate.</div>\n```\n";
        let doc = MarkdownParser.parse(source);
        let segments = doc.translatable_segments();
        assert!(segments.iter().any(|s| s.source.contains("**this**")));
        assert!(segments.iter().any(|s| s.source.contains("`Vec<T>`")));
        assert!(segments.iter().any(|s| s.source.contains("<https://example.com/a>")));
        assert!(!segments.iter().any(|s| s.source.contains("Do not translate")));
    }

    #[test]
    fn html_cdata_and_processing_instructions_stay_verbatim() {
        for source in ["<![CDATA[ do > keep this ]]>\n", "<?target do > keep this ?>\n"] {
            let doc = MarkdownParser.parse(source);
            assert!(doc.translatable_segments().is_empty());
            assert_eq!(MarkdownParser.reconstruct(&doc, &TranslationMap::new()), source);
        }
    }

    #[test]
    fn html_indentation_is_not_a_markdown_code_block() {
        let source = "<div><p>    Human visible prose.</p></div>\n";
        let doc = MarkdownParser.parse(source);
        let segments = doc.translatable_segments();
        assert_eq!(segments.len(), 1);
        let map = [(segments[0].id.clone(), "눈에 보이는 본문.".to_string())].into();
        assert_eq!(MarkdownParser.reconstruct(&doc, &map),
            "<div><p>    눈에 보이는 본문.</p></div>\n");
    }

    #[test]
    fn html_literal_elements_remain_opaque_across_blank_lines() {
        let source = "<div>\n<pre><code>one\n\ntwo</code></pre>\n</div>\n\nVisible prose.\n";
        let doc = MarkdownParser.parse(source);
        assert_eq!(doc.translatable_segments().iter().map(|s| s.source.as_str()).collect::<Vec<_>>(), ["Visible prose."]);
    }

    #[test]
    fn html_literals_after_longer_closing_fences_are_preserved() {
        let source = "```\ncode\n````\n\n<code>keep literal</code>\n";
        assert!(MarkdownParser.parse(source).translatable_segments().is_empty());
    }

}
