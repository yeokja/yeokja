use yeokja_core::model::*;
use yeokja_core::parser::{DocumentParser, Markup, TranslationMap};
use yeokja_parser_markdown_dialect::parse_with;
use yeokja_parser_utils::{resolve_reference_links, splice_reconstruct};

/// Span-based Markdown parser.
///
/// `parse` records the byte range of each block's inline content in the source.
/// Segments therefore carry the raw inline markdown (links, emphasis, code spans),
/// which lets evaluators verify markup preservation and lets the LLM see it.
/// `reconstruct` splices translations into the original source, leaving everything
/// outside the translated spans (code fences, list markers, blockquote prefixes,
/// front matter, blank lines) byte-for-byte intact.
pub struct MarkdownParser;

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
