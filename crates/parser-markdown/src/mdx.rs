//! MDX flavour of the Markdown parser.
//!
//! MDX files mix Markdown with JSX components, ESM statements and YAML front
//! matter. CommonMark treats a line beginning with a tag as raw HTML and swallows
//! every following line up to the next blank one, so prose written directly
//! under `<Admonition …>` would never be offered for translation.
//!
//! This parser scans the source line by line first and records the constructs
//! Markdown must not see: front matter, block-level JSX tag lines (which may
//! span several lines until their closing `>`), `{…}` expression lines and
//! `import`/`export` runs. Those regions are blanked to spaces — newlines kept
//! — in a shadow copy handed to pulldown-cmark, so byte offsets line up with the
//! original and the children of a component parse as ordinary Markdown. The
//! scan also finds the user-visible strings inside those regions (selected
//! front matter fields, `title="…"` attributes) and offers them as blocks of
//! their own; on reconstruction each is escaped the way its container needs.

use std::ops::Range;
use yeokja_core::model::*;
use yeokja_core::parser::{DocumentParser, Markup, TranslationMap};
use yeokja_parser_utils::{apply_splices, collect_splices};

use crate::{Extra, parse_with};

/// Top-level front matter keys whose values readers see.
const TRANSLATED_FIELDS: &[&str] = &["title", "snippet", "description", "summary"];

/// JSX attributes whose values readers see.
const TRANSLATED_ATTRIBUTES: &[&str] = &["title"];

/// MDX parser: Markdown with JSX components and translatable front matter.
pub struct MdxParser;

/// How a translatable string is quoted by its container, which decides the
/// escaping its translation needs on the way back in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Quoting {
    /// A YAML plain scalar (`title: Nix flakes`). Quoted only if the
    /// translation would otherwise be read as something else.
    YamlPlain,
    /// The body of a YAML block scalar (`snippet: |`). Literal; nothing to escape.
    YamlBlock,
    /// Inside YAML double quotes.
    YamlDoubleQuoted,
    /// Inside YAML single quotes.
    YamlSingleQuoted,
    /// Inside a JSX attribute's double quotes.
    JsxDoubleQuoted,
    /// Inside a JSX attribute's single quotes.
    JsxSingleQuoted,
}

impl Quoting {
    fn escape(self, text: &str) -> String {
        match self {
            Quoting::YamlPlain => {
                if yaml_plain_is_safe(text) {
                    text.to_string()
                } else {
                    format!("\"{}\"", Quoting::YamlDoubleQuoted.escape(text))
                }
            }
            Quoting::YamlBlock => text.to_string(),
            Quoting::YamlDoubleQuoted => text.replace('\\', "\\\\").replace('"', "\\\""),
            Quoting::YamlSingleQuoted => text.replace('\'', "''"),
            Quoting::JsxDoubleQuoted => text.replace('"', "&quot;"),
            Quoting::JsxSingleQuoted => text.replace('\'', "&#39;"),
        }
    }
}

/// Whether `text` can stand as a YAML plain scalar and still read as itself.
fn yaml_plain_is_safe(text: &str) -> bool {
    let Some(first) = text.chars().next() else { return true };
    if "-?:,[]{}#&*!|>'\"%@`".contains(first) {
        return false;
    }
    !(text.contains(": ")
        || text.contains(" #")
        || text.ends_with(':')
        || text.contains('\t')
        || text.contains('\n'))
}

/// A user-visible string inside a region Markdown does not see.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Field {
    range: Range<usize>,
    block_type: BlockType,
    quoting: Quoting,
}

/// What the line scan found.
#[derive(Debug, Default)]
struct Layout {
    /// Regions to blank out before Markdown parsing, each kept verbatim as an
    /// opaque block.
    opaque: Vec<Range<usize>>,
    /// Translatable strings inside those regions.
    fields: Vec<Field>,
}

impl Layout {
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

/// Byte offset of the start of `line` within `source`, given line iteration.
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
    let block = &lines[1..close];
    layout.opaque.push(0..lines[close].end);

    let mut index = 0;
    while index < block.len() {
        let text = line_text(source, &block[index]);
        let Some((key, rest)) = split_top_level_key(text) else {
            index += 1;
            continue;
        };
        // The value extends until the next top-level key (or the end).
        let mut end = index + 1;
        while end < block.len() {
            let candidate = line_text(source, &block[end]);
            if split_top_level_key(candidate).is_some() {
                break;
            }
            end += 1;
        }
        if TRANSLATED_FIELDS.contains(&key) {
            let value_offset = block[index].start + (text.len() - rest.len());
            let value_lines = &block[index + 1..end];
            scan_field_value(source, key, value_offset, rest, value_lines, layout);
        }
        index = end;
    }
    lines[close].end
}

/// `key: rest` when the line starts a top-level mapping entry.
fn split_top_level_key(line: &str) -> Option<(&str, &str)> {
    let first = line.chars().next()?;
    if !(first.is_ascii_alphabetic() || first == '_') {
        return None;
    }
    let colon = line.find(':')?;
    let key = &line[..colon];
    if !key
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
    {
        return None;
    }
    let rest = &line[colon + 1..];
    if !(rest.is_empty() || rest.starts_with(' ') || rest.starts_with('\t')) {
        return None;
    }
    Some((key, rest))
}

fn scan_field_value(
    source: &str,
    key: &str,
    value_offset: usize,
    rest: &str,
    value_lines: &[Range<usize>],
    layout: &mut Layout,
) {
    let block_type = if key == "title" {
        BlockType::Heading
    } else {
        BlockType::Paragraph
    };
    let trimmed = rest.trim();
    let value_start = value_offset + (rest.len() - rest.trim_start().len());
    let content_lines: Vec<&Range<usize>> = value_lines
        .iter()
        .filter(|range| !line_text(source, range).trim().is_empty())
        .collect();

    let mut push = |range: Range<usize>, quoting: Quoting| {
        if !source[range.clone()].trim().is_empty() {
            layout.fields.push(Field {
                range,
                block_type,
                quoting,
            });
        }
    };

    if trimmed.is_empty() {
        // Value on the following lines: a block scalar without indicator is
        // not YAML, so this is either a flow sequence or nothing.
        let Some(first) = content_lines.first() else { return };
        if line_text(source, first).trim_start().starts_with('[') {
            let end = content_lines.last().unwrap().end;
            for (range, quoting) in quoted_strings(source, first.start..end) {
                push(range, quoting);
            }
        }
        return;
    }
    let indicator = trimmed.trim_end_matches(|c: char| c == '-' || c == '+' || c.is_ascii_digit());
    if matches!(indicator, "|" | ">") {
        let (Some(first), Some(last)) = (content_lines.first(), content_lines.last()) else {
            return;
        };
        let first_text = &source[(*first).clone()];
        let start = first.start + (first_text.len() - first_text.trim_start().len());
        let end = last.start + line_text(source, last).trim_end().len();
        push(start..end, Quoting::YamlBlock);
        return;
    }
    if trimmed.starts_with('[') {
        let end = content_lines
            .last()
            .map(|range| range.end)
            .unwrap_or(value_offset + rest.len());
        for (range, quoting) in quoted_strings(source, value_start..end) {
            push(range, quoting);
        }
        return;
    }
    if trimmed.starts_with('"') || trimmed.starts_with('\'') {
        if let Some((range, quoting)) = quoted_strings(source, value_start..value_offset + rest.len())
            .into_iter()
            .next()
        {
            push(range, quoting);
        }
        return;
    }
    // A plain scalar, possibly folded over indented continuation lines.
    let mut end = value_offset + rest.trim_end().len();
    if let Some(last) = content_lines.last() {
        end = last.start + line_text(source, last).trim_end().len();
    }
    push(value_start..end, Quoting::YamlPlain);
}

/// The insides of every quoted string in `range`, with their quoting.
fn quoted_strings(source: &str, range: Range<usize>) -> Vec<(Range<usize>, Quoting)> {
    let bytes = source.as_bytes();
    let mut found = Vec::new();
    let mut i = range.start;
    while i < range.end {
        match bytes[i] {
            b'"' => {
                let start = i + 1;
                let mut j = start;
                while j < range.end && bytes[j] != b'"' {
                    if bytes[j] == b'\\' {
                        j += 1;
                    }
                    j += 1;
                }
                if j >= range.end {
                    break;
                }
                found.push((start..j, Quoting::YamlDoubleQuoted));
                i = j + 1;
            }
            b'\'' => {
                let start = i + 1;
                let mut j = start;
                loop {
                    while j < range.end && bytes[j] != b'\'' {
                        j += 1;
                    }
                    if j + 1 < range.end && bytes[j + 1] == b'\'' {
                        j += 2;
                        continue;
                    }
                    break;
                }
                if j >= range.end {
                    break;
                }
                found.push((start..j, Quoting::YamlSingleQuoted));
                i = j + 1;
            }
            _ => i += 1,
        }
    }
    found
}

fn scan_body(source: &str, body_start: usize, layout: &mut Layout) {
    let lines = line_ranges(source, body_start);
    let mut index = 0;
    let mut fence: Option<(u8, usize)> = None;
    let mut previous_blank = true;

    while index < lines.len() {
        let range = &lines[index];
        let text = line_text(source, range);
        let trimmed = text.trim_start();
        let indent = text.len() - trimmed.len();

        // Fenced code is Markdown's business; a tag inside it is not JSX.
        if let Some((marker, len)) = fence {
            if indent <= 3
                && trimmed.bytes().take_while(|&b| b == marker).count() >= len
                && trimmed.trim_start_matches(marker as char).trim().is_empty()
            {
                fence = None;
            }
            previous_blank = false;
            index += 1;
            continue;
        }
        if indent <= 3 && (trimmed.starts_with("```") || trimmed.starts_with("~~~")) {
            let marker = trimmed.as_bytes()[0];
            let len = trimmed.bytes().take_while(|&b| b == marker).count();
            fence = Some((marker, len));
            previous_blank = false;
            index += 1;
            continue;
        }

        if trimmed.is_empty() {
            previous_blank = true;
            index += 1;
            continue;
        }

        // ESM: an `import`/`export` paragraph, kept whole up to the blank line.
        if previous_blank
            && indent == 0
            && (trimmed.starts_with("import ") || trimmed.starts_with("export "))
        {
            let start = range.start;
            let mut end = index;
            while end < lines.len() && !line_text(source, &lines[end]).trim().is_empty() {
                end += 1;
            }
            let last = &lines[end - 1];
            layout
                .opaque
                .push(start..last.start + line_text(source, last).len());
            previous_blank = false;
            index = end;
            continue;
        }

        let line_content_start = range.start + indent;
        if trimmed.starts_with('<')
            && let Some(tag) = scan_jsx_tag(source, line_content_start)
            && rest_of_line_is_blank(source, tag.end)
        {
            layout.opaque.push(line_content_start..tag.end);
            layout.fields.extend(tag.fields);
            previous_blank = false;
            index = line_index_after(&lines, tag.end);
            continue;
        }
        if trimmed.starts_with('{')
            && let Some(end) = balanced_end(source, line_content_start, b'{', b'}')
            && rest_of_line_is_blank(source, end)
        {
            layout.opaque.push(line_content_start..end);
            previous_blank = false;
            index = line_index_after(&lines, end);
            continue;
        }

        previous_blank = false;
        index += 1;
    }
}

fn rest_of_line_is_blank(source: &str, from: usize) -> bool {
    source[from..]
        .split('\n')
        .next()
        .unwrap_or("")
        .trim()
        .is_empty()
}

/// Index of the first line starting at or after `offset` has been consumed.
fn line_index_after(lines: &[Range<usize>], offset: usize) -> usize {
    lines
        .iter()
        .position(|range| range.end > offset)
        .map(|i| i + 1)
        .unwrap_or(lines.len())
}

struct JsxTag {
    /// Offset just past the closing `>`.
    end: usize,
    fields: Vec<Field>,
}

fn is_tag_name_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || matches!(b, b'.' | b'-' | b'_' | b':')
}

/// Parse the JSX tag opening at `start` (which holds `<`), spanning lines if
/// its attributes do. `None` if the bytes there are not a complete tag.
fn scan_jsx_tag(source: &str, start: usize) -> Option<JsxTag> {
    let bytes = source.as_bytes();
    let mut i = start + 1;
    if bytes.get(i) == Some(&b'/') {
        i += 1;
    }
    if !bytes.get(i)?.is_ascii_alphabetic() {
        return None;
    }
    while i < bytes.len() && is_tag_name_byte(bytes[i]) {
        i += 1;
    }

    let mut fields = Vec::new();
    loop {
        while i < bytes.len() && bytes[i].is_ascii_whitespace() {
            i += 1;
        }
        match bytes.get(i)? {
            b'>' => return Some(JsxTag { end: i + 1, fields }),
            b'/' => {
                return (bytes.get(i + 1) == Some(&b'>')).then_some(JsxTag { end: i + 2, fields });
            }
            b'{' => {
                // A spread attribute: `{...props}`.
                i = balanced_end(source, i, b'{', b'}')?;
            }
            b if b.is_ascii_alphabetic() || *b == b'_' => {
                let name_start = i;
                while i < bytes.len() && is_tag_name_byte(bytes[i]) {
                    i += 1;
                }
                let name = &source[name_start..i];
                let mut j = i;
                while j < bytes.len() && bytes[j].is_ascii_whitespace() {
                    j += 1;
                }
                if bytes.get(j) != Some(&b'=') {
                    continue; // boolean attribute
                }
                j += 1;
                while j < bytes.len() && bytes[j].is_ascii_whitespace() {
                    j += 1;
                }
                match bytes.get(j)? {
                    quote @ (b'"' | b'\'') => {
                        let value_start = j + 1;
                        let value_end = value_start + source[value_start..].find(*quote as char)?;
                        if TRANSLATED_ATTRIBUTES.contains(&name)
                            && !source[value_start..value_end].trim().is_empty()
                        {
                            fields.push(Field {
                                range: value_start..value_end,
                                block_type: BlockType::Heading,
                                quoting: if *quote == b'"' {
                                    Quoting::JsxDoubleQuoted
                                } else {
                                    Quoting::JsxSingleQuoted
                                },
                            });
                        }
                        i = value_end + 1;
                    }
                    b'{' => i = balanced_end(source, j, b'{', b'}')?,
                    _ => return None,
                }
            }
            _ => return None,
        }
    }
}

/// Offset just past the bracket closing the one at `start`, skipping quoted
/// strings and nested brackets. `None` when it never closes.
fn balanced_end(source: &str, start: usize, open: u8, close: u8) -> Option<usize> {
    let bytes = source.as_bytes();
    let mut depth = 0usize;
    let mut i = start;
    while i < bytes.len() {
        match bytes[i] {
            b'"' | b'\'' | b'`' => {
                let quote = bytes[i];
                i += 1;
                while i < bytes.len() && bytes[i] != quote {
                    if bytes[i] == b'\\' {
                        i += 1;
                    }
                    i += 1;
                }
            }
            b if b == open => depth += 1,
            b if b == close => {
                depth -= 1;
                if depth == 0 {
                    return Some(i + 1);
                }
            }
            _ => {}
        }
        i += 1;
    }
    None
}

impl DocumentParser for MdxParser {
    fn markup(&self) -> Markup {
        Markup::Markdown
    }

    fn parse(&self, source: &str) -> Document {
        let layout = scan(source);
        let shadow = layout.shadow(source);
        parse_with(&shadow, source, true, layout.extras())
    }

    fn reconstruct(&self, document: &Document, translations: &TranslationMap) -> String {
        let layout = scan(&document.source);
        let mut splices = collect_splices(document, translations);
        for (range, text) in &mut splices {
            if let Some(field) = layout.fields.iter().find(|field| field.range == *range) {
                *text = field.quoting.escape(text);
            }
        }
        apply_splices(&document.source, splices)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::MarkdownParser;

    fn sources(doc: &Document) -> Vec<String> {
        doc.translatable_segments()
            .iter()
            .map(|seg| seg.source.clone())
            .collect()
    }

    /// Translate every segment with `f` and reconstruct.
    fn translate_all(source: &str, f: impl Fn(&str) -> String) -> String {
        let parser = MdxParser;
        let doc = parser.parse(source);
        let mut translations = TranslationMap::new();
        for seg in doc.translatable_segments() {
            translations.insert(seg.id.clone(), f(&seg.source));
        }
        parser.reconstruct(&doc, &translations)
    }

    #[test]
    fn jsx_block_children_are_translated() {
        let source = "<Admonition warning open title=\"Heads up\" id=\"heads-up\">\nFirst line.\nSecond line.\n\nAnother para.\n</Admonition>\n\nTail.\n";
        let doc = MdxParser.parse(source);
        assert_eq!(
            sources(&doc),
            ["Heads up", "First line.", "Second line.", "Another para.", "Tail."]
        );
        let out = translate_all(source, |s| format!("[{s}]"));
        assert_eq!(
            out,
            "<Admonition warning open title=\"[Heads up]\" id=\"heads-up\">\n[First line.] [Second line.]\n\n[Another para.]\n</Admonition>\n\n[Tail.]\n"
        );
    }

    #[test]
    fn plain_markdown_parser_still_swallows_jsx_children() {
        let doc = MarkdownParser.parse("<Admonition>\ntext\n</Admonition>\n");
        assert!(doc.translatable_segments().is_empty());
    }

    #[test]
    fn multi_line_jsx_tag_is_an_opaque_boundary() {
        let source = "<Admonition\n  warning\n  title=\"The limits of `fh init`\"\n  id=\"limits\"\n>\nBody text.\n</Admonition>\n";
        let doc = MdxParser.parse(source);
        assert_eq!(sources(&doc), ["The limits of `fh init`", "Body text."]);
        let out = translate_all(source, |s| format!("[{s}]"));
        assert_eq!(
            out,
            "<Admonition\n  warning\n  title=\"[The limits of `fh init`]\"\n  id=\"limits\"\n>\n[Body text.]\n</Admonition>\n"
        );
    }

    #[test]
    fn attribute_expressions_and_other_attributes_stay_verbatim() {
        let source = "<ExternalSources\n  showTitle={false}\n  links={[\n    { title: \"Nix Docker examples\", href: \"https://x\" },\n  ]}\n/>\n\nAfter.\n";
        let doc = MdxParser.parse(source);
        assert_eq!(sources(&doc), ["After."]);
        let out = translate_all(source, |_| "번역".to_string());
        assert_eq!(out, source.replace("After.", "번역"));
    }

    #[test]
    fn self_closing_component_line_preserved() {
        let source = "Intro.\n\n<Platforms />\n\nOutro.\n";
        let doc = MdxParser.parse(source);
        assert_eq!(sources(&doc), ["Intro.", "Outro."]);
        assert_eq!(
            translate_all(source, |s| format!("[{s}]")),
            "[Intro.]\n\n<Platforms />\n\n[Outro.]\n"
        );
    }

    #[test]
    fn jsx_tags_adjacent_to_code_fences_and_each_other() {
        let source = "<SpecificLanguage lang=\"C++\">\n```shell title=\"Explore\"\nnix develop\n```\n\nProse here.\n\n</SpecificLanguage>\n<SpecificLanguage lang=\"Go\">\nMore prose.\n</SpecificLanguage>\n";
        let doc = MdxParser.parse(source);
        assert_eq!(sources(&doc), ["Prose here.", "More prose."]);
        let out = translate_all(source, |s| format!("[{s}]"));
        assert_eq!(
            out,
            "<SpecificLanguage lang=\"C++\">\n```shell title=\"Explore\"\nnix develop\n```\n\n[Prose here.]\n\n</SpecificLanguage>\n<SpecificLanguage lang=\"Go\">\n[More prose.]\n</SpecificLanguage>\n"
        );
    }

    #[test]
    fn tags_inside_code_fences_are_not_jsx() {
        let source = "```html\n<Admonition title=\"not a heading\">\n</Admonition>\n```\n\nText.\n";
        let doc = MdxParser.parse(source);
        assert_eq!(sources(&doc), ["Text."]);
        assert_eq!(
            translate_all(source, |_| "번역".to_string()),
            source.replace("Text.", "번역")
        );
    }

    #[test]
    fn inline_jsx_kept_verbatim_inside_segment() {
        let source = "Use <Language /> here and <Link href=\"/x\">the docs</Link> now.\n";
        let doc = MdxParser.parse(source);
        assert_eq!(
            sources(&doc),
            ["Use <Language /> here and <Link href=\"/x\">the docs</Link> now."]
        );
        let out = translate_all(source, |_| {
            "<Language /> 사용, <Link href=\"/x\">문서</Link> 참고.".to_string()
        });
        assert_eq!(out, "<Language /> 사용, <Link href=\"/x\">문서</Link> 참고.\n");
    }

    #[test]
    fn indented_jsx_children_and_tags_in_lists() {
        let source = "<Admonition info title=\"Contribute\">\n  If you're interested, see the manual.\n</Admonition>\n\n1. First item.\n   <NixStorePath pkg=\"git\" />\n1. Second item.\n";
        let doc = MdxParser.parse(source);
        assert_eq!(
            sources(&doc),
            ["Contribute", "If you're interested, see the manual.", "First item.", "Second item."]
        );
        let out = translate_all(source, |s| format!("[{s}]"));
        assert_eq!(
            out,
            "<Admonition info title=\"[Contribute]\">\n  [If you're interested, see the manual.]\n</Admonition>\n\n1. [First item.]\n   <NixStorePath pkg=\"git\" />\n1. [Second item.]\n"
        );
    }

    #[test]
    fn frontmatter_title_and_snippet_translated_others_untouched() {
        let source = "---\ntitle: Nix flakes\nwip: true\nsnippet: |\n  A system for [Nix code](/concepts/nix-language)\nrelated: [\"channels\"]\nexternalSources:\n  [\n    {\n      title: \"Flakes\",\n      href: \"https://wiki.nixos.org/wiki/Flakes\",\n    },\n  ]\n---\n\nBody.\n";
        let doc = MdxParser.parse(source);
        assert_eq!(
            sources(&doc),
            ["Nix flakes", "A system for [Nix code](/concepts/nix-language)", "Body."]
        );
        let title = doc.translatable_segments()[0];
        assert_eq!(title.block_type, BlockType::Heading);
        let out = translate_all(source, |s| match s {
            "Nix flakes" => "Nix 플레이크".to_string(),
            "Body." => "본문.".to_string(),
            _ => "[Nix 코드](/concepts/nix-language)를 참조하고 공유하는 체계".to_string(),
        });
        assert_eq!(
            out,
            "---\ntitle: Nix 플레이크\nwip: true\nsnippet: |\n  [Nix 코드](/concepts/nix-language)를 참조하고 공유하는 체계\nrelated: [\"channels\"]\nexternalSources:\n  [\n    {\n      title: \"Flakes\",\n      href: \"https://wiki.nixos.org/wiki/Flakes\",\n    },\n  ]\n---\n\n본문.\n"
        );
    }

    #[test]
    fn frontmatter_block_scalar_over_several_lines_collapses() {
        let source = "---\nsnippet: |\n  First half\n  second half.\n---\n\nBody.\n";
        let doc = MdxParser.parse(source);
        assert_eq!(sources(&doc), ["First half second half.", "Body."]);
        let out = translate_all(source, |_| "한 줄".to_string());
        assert_eq!(out, "---\nsnippet: |\n  한 줄\n---\n\n한 줄\n");
    }

    #[test]
    fn frontmatter_summary_items_translated_with_escaping() {
        let source = "---\ntitle: Get Nix running\norder: 1\nsummary:\n  [\n    \"Get [Nix](/concepts/nix) running\",\n    \"Verify that it works\",\n  ]\n---\n\nBody.\n";
        let doc = MdxParser.parse(source);
        assert_eq!(
            sources(&doc),
            ["Get Nix running", "Get [Nix](/concepts/nix) running", "Verify that it works", "Body."]
        );
        let out = translate_all(source, |s| match s {
            "Verify that it works" => "\"동작\" 확인".to_string(),
            other => other.to_string(),
        });
        assert!(out.contains("    \"\\\"동작\\\" 확인\",\n"), "{out}");

        let inline = "---\nsummary: [\"Cleanly remove Nix\"]\n---\n\nBody.\n";
        assert_eq!(
            translate_all(inline, |_| "제거".to_string()),
            "---\nsummary: [\"제거\"]\n---\n\n제거\n"
        );
    }

    #[test]
    fn frontmatter_plain_scalar_is_quoted_when_translation_needs_it() {
        let source = "---\ntitle: Nix\n---\n\nBody.\n";
        let out = translate_all(source, |s| match s {
            "Nix" => "Nix: 소개".to_string(),
            other => other.to_string(),
        });
        assert_eq!(out, "---\ntitle: \"Nix: 소개\"\n---\n\nBody.\n");

        let quoted = "---\ndescription: \"What it is\"\n---\n\nBody.\n";
        let out = translate_all(quoted, |s| match s {
            "What it is" => "\"그것\"".to_string(),
            other => other.to_string(),
        });
        assert_eq!(out, "---\ndescription: \"\\\"그것\\\"\"\n---\n\nBody.\n");
    }

    #[test]
    fn frontmatter_without_body_and_without_translated_keys() {
        let source = "---\nid: nix-installer-differences\n---\n\nThe installer differs.\n";
        let doc = MdxParser.parse(source);
        assert_eq!(sources(&doc), ["The installer differs."]);
        assert_eq!(
            translate_all(source, |_| "다릅니다.".to_string()),
            "---\nid: nix-installer-differences\n---\n\n다릅니다.\n"
        );
    }

    #[test]
    fn heading_custom_id_suffix_preserved() {
        let source = "## Flake references \\{#references}\n\nText.\n\n### Plain heading\n";
        let doc = MdxParser.parse(source);
        assert_eq!(sources(&doc), ["Flake references", "Text.", "Plain heading"]);
        assert_eq!(
            translate_all(source, |s| format!("[{s}]")),
            "## [Flake references] \\{#references}\n\n[Text.]\n\n### [Plain heading]\n"
        );
    }

    #[test]
    fn heading_id_suffix_is_mdx_only() {
        let doc = MarkdownParser.parse("## Refs \\{#refs}\n");
        assert_eq!(sources(&doc), ["Refs \\{#refs}"]);
    }

    #[test]
    fn attribute_title_translation_escapes_quotes() {
        let source = "<Admonition title=\"Why?\" id=\"why\">\nBody.\n</Admonition>\n";
        let out = translate_all(source, |s| match s {
            "Why?" => "\"왜\"일까?".to_string(),
            other => other.to_string(),
        });
        assert_eq!(
            out,
            "<Admonition title=\"&quot;왜&quot;일까?\" id=\"why\">\nBody.\n</Admonition>\n"
        );
    }

    #[test]
    fn esm_lines_and_expression_lines_are_opaque() {
        let source = "import Foo from \"./foo\";\nexport const x = 1;\n\n{/* <Brief id=\"x\" /> */}\n\nBody.\n";
        let doc = MdxParser.parse(source);
        assert_eq!(sources(&doc), ["Body."]);
        assert_eq!(
            translate_all(source, |_| "본문.".to_string()),
            source.replace("Body.", "본문.")
        );
    }

    #[test]
    fn link_definitions_stay_verbatim() {
        let source = "Body [a].\n\n[a]: https://example.com\n[b]: /concepts/nix\n";
        let doc = MdxParser.parse(source);
        assert_eq!(sources(&doc), ["Body [a]."]);
        assert_eq!(
            translate_all(source, |_| "본문 [a].".to_string()),
            "본문 [a].\n\n[a]: https://example.com\n[b]: /concepts/nix\n"
        );
    }

    #[test]
    fn empty_translation_map_round_trips_a_realistic_page() {
        let source = "---\ntitle: Nix flakes\nsnippet: |\n  A system\n---\n\n<Admonition warning open title=\"Status\" id=\"s\">\nIn the [upstream][u], flakes are **experimental**.\n\n</Admonition>\n\nA Nix _flake_ is a directory.\n\n## Flake references \\{#references}\n\nRef | Desc\n:---|:----\n`path:/x` | The directory\n\n```nix title=\"flake.nix\"\n{ }\n```\n\n[u]: https://github.com/NixOS/nix\n";
        let doc = MdxParser.parse(source);
        assert_eq!(
            sources(&doc),
            [
                "Nix flakes",
                "A system",
                "Status",
                "In the [upstream][u], flakes are **experimental**.",
                "A Nix _flake_ is a directory.",
                "Flake references",
                "Ref",
                "Desc",
                "`path:/x`",
                "The directory",
            ]
        );
        assert_eq!(MdxParser.reconstruct(&doc, &TranslationMap::new()), source);
    }
}
