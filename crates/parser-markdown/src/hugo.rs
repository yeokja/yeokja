//! Hugo flavour of the Markdown parser: a content file of a Hugo site.
//!
//! The body is read exactly as the Markdown parser reads it (goldmark is
//! CommonMark). Like the other flavours, this one hands pulldown-cmark a
//! shadow of the source — same length, same newlines — in which two things
//! are blanked, so they stay verbatim outside every span:
//!
//! * The front matter: TOML between `+++` lines or YAML between `---` lines
//!   at the top of the file. The string values of the configured
//!   `front_matter_keys` are offered as literal strings, and reconstruction
//!   writes each translation back as a valid string of its format.
//! * Lines holding only shortcodes (`{{< note >}}`, `{{< /note >}}`,
//!   `{{% details %}}`), and a shortcode that opens a line and closes on a
//!   later one, outside fenced code. The Markdown between paired shortcodes
//!   is translated as usual; a shortcode inside a sentence travels with it.

use std::ops::Range;
use yeokja_core::model::*;
use yeokja_core::parser::{DocumentParser, Markup, TranslationMap};
use yeokja_parser_markdown_dialect::front_matter::{
    key_paths, quote_toml, quote_yaml, toml_front_matter, toml_values, yaml_front_matter, yaml_values,
};
use yeokja_parser_markdown_dialect::{Extra, parse_with};
use yeokja_parser_utils::{apply_splices, collect_splices, resolve_reference_links};

/// The Hugo parser. `front_matter_keys` are the source's
/// `front_matter_keys`: dotted key paths whose front matter string values are
/// offered as paragraphs. With none, the front matter is kept whole.
#[derive(Default)]
pub struct HugoParser {
    front_matter_keys: Vec<Vec<String>>,
}

impl HugoParser {
    /// A parser offering the values of the given dotted key paths.
    pub fn configured<K: AsRef<str>>(keys: &[K]) -> Self {
        Self { front_matter_keys: key_paths(keys) }
    }
}

#[derive(Clone, Copy, PartialEq)]
enum Format {
    Toml,
    Yaml,
}

fn front_matter(source: &str) -> Option<(Format, Range<usize>)> {
    toml_front_matter(source)
        .map(|r| (Format::Toml, r))
        .or_else(|| yaml_front_matter(source).map(|r| (Format::Yaml, r)))
}

impl DocumentParser for HugoParser {
    fn markup(&self) -> Markup {
        Markup::Markdown
    }

    fn parse(&self, source: &str) -> Document {
        let mut shadow = source.as_bytes().to_vec();
        let mut extras = Vec::new();
        let mut body = 0;
        if let Some((format, front)) = front_matter(source) {
            let values: Vec<Range<usize>> = match format {
                Format::Toml => toml_values(source, front.clone(), &self.front_matter_keys),
                Format::Yaml => yaml_values(source, front.clone(), &[], &self.front_matter_keys)
                    .into_iter()
                    .map(|(range, _)| range)
                    .collect(),
            };
            for range in values {
                extras.push(Extra { range, block_type: BlockType::Paragraph, literal: true });
            }
            fill(&mut shadow, front.clone());
            body = front.end;
        }
        for line in shortcode_lines(source, body) {
            fill(&mut shadow, line);
        }
        let shadow = String::from_utf8(shadow).expect("shadow edits are ASCII over whole lines");
        parse_with(&shadow, source, true, extras)
    }

    fn reconstruct(&self, document: &Document, translations: &TranslationMap) -> String {
        let translations = resolve_reference_links(document, translations);
        let source = &document.source;
        let front = front_matter(source);
        let splices = collect_splices(document, &translations)
            .into_iter()
            .map(|(span, text)| match &front {
                Some((format, range)) if range.contains(&span.start) => match format {
                    Format::Toml => quote_toml(source, span, &text),
                    Format::Yaml => (span.clone(), quote_yaml(source, span, text)),
                },
                _ => (span, text),
            })
            .collect();
        apply_splices(source, splices)
    }
}

fn fill(shadow: &mut [u8], range: Range<usize>) {
    for b in &mut shadow[range] {
        if *b != b'\n' {
            *b = b' ';
        }
    }
}

/// Byte ranges of each line from `from`, without its newline.
fn lines_from(source: &str, from: usize) -> Vec<Range<usize>> {
    let mut out = Vec::new();
    let mut start = from;
    for (i, b) in source.bytes().enumerate().skip(from) {
        if b == b'\n' {
            out.push(start..i);
            start = i + 1;
        }
    }
    if start < source.len() {
        out.push(start..source.len());
    }
    out
}

/// A fence opener: its character and run length.
fn fence_open(line: &str) -> Option<(u8, usize)> {
    let indent = line.len() - line.trim_start_matches(' ').len();
    if indent > 3 {
        return None;
    }
    let rest = &line[indent..];
    let c = *rest.as_bytes().first()?;
    if c != b'`' && c != b'~' {
        return None;
    }
    let n = rest.bytes().take_while(|&b| b == c).count();
    (n >= 3 && !(c == b'`' && rest[n..].contains('`'))).then_some((c, n))
}

fn fence_closes(line: &str, (c, n): (u8, usize)) -> bool {
    let t = line.trim();
    line.len() - line.trim_start_matches(' ').len() <= 3 && t.len() >= n && t.bytes().all(|b| b == c)
}

/// The end (exclusive) of the shortcode that opens at `at` in `text`
/// (`{{<` … `>}}` or `{{%` … `%}}`), when it closes in `text`.
fn shortcode_end(text: &str, at: usize) -> Option<usize> {
    let close = match text.get(at..at + 3)? {
        "{{<" => ">}}",
        "{{%" => "%}}",
        _ => return None,
    };
    text[at + 3..].find(close).map(|n| at + 3 + n + 3)
}

/// Whether `text` from `at` holds only shortcodes and whitespace.
fn only_shortcodes(text: &str, mut at: usize) -> bool {
    loop {
        at += text[at..].len() - text[at..].trim_start().len();
        if at == text.len() {
            return true;
        }
        match shortcode_end(text, at) {
            Some(end) => at = end,
            None => return false,
        }
    }
}

/// The lines from `from` that hold only shortcodes, outside fenced code. A
/// shortcode that opens a line and closes on a later one covers every line
/// up to the closing one, when nothing but shortcodes follows its close.
fn shortcode_lines(source: &str, from: usize) -> Vec<Range<usize>> {
    let lines = lines_from(source, from);
    let mut out = Vec::new();
    let mut fence = None;
    let mut k = 0;
    while k < lines.len() {
        let text = &source[lines[k].clone()];
        if let Some(open) = fence {
            if fence_closes(text, open) {
                fence = None;
            }
            k += 1;
            continue;
        }
        if let Some(open) = fence_open(text) {
            fence = Some(open);
            k += 1;
            continue;
        }
        let lead = text.len() - text.trim_start().len();
        if !text[lead..].starts_with("{{<") && !text[lead..].starts_with("{{%") {
            k += 1;
            continue;
        }
        if only_shortcodes(text, lead) {
            out.push(lines[k].clone());
            k += 1;
            continue;
        }
        if shortcode_end(text, lead).is_none() {
            // Opens here and closes on a later line.
            let start = lines[k].start + lead;
            let block = &source[start..];
            if let Some(end) = shortcode_end(block, 0).map(|end| start + end)
                && let Some(last) = lines[k..].iter().position(|l| l.contains(&(end - 1)))
            {
                let last = k + last;
                if only_shortcodes(&source[..lines[last].end], end) {
                    out.extend(lines[k..=last].iter().cloned());
                    k = last + 1;
                    continue;
                }
            }
        }
        k += 1;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hugo() -> HugoParser {
        HugoParser::configured(&["title", "description"])
    }

    fn sources(parser: &HugoParser, source: &str) -> Vec<String> {
        parser.parse(source).translatable_segments().iter().map(|s| s.source.clone()).collect()
    }

    fn translate_all(parser: &HugoParser, source: &str, f: impl Fn(&str) -> String) -> String {
        let doc = parser.parse(source);
        let map = doc.translatable_segments().iter().map(|s| (s.id.clone(), f(&s.source))).collect();
        parser.reconstruct(&doc, &map)
    }

    const POST: &str = "+++\ntitle = \"Memory Safety Bugs\"\ndate = 2026-11-09\ndescription = \"Knowing What You Trust. Three patterns.\"\nslug = \"rust-std-memory-safety\"\nauthor = \"Yechan Bae\"\ntoml = true\n+++\n\nMicrosoft's lab found issues. Rust matters.\n";

    #[test]
    fn toml_front_matter_is_blanked_and_chosen_keys_offered() {
        assert_eq!(
            sources(&hugo(), POST),
            ["Memory Safety Bugs", "Knowing What You Trust.", "Three patterns.", "Microsoft's lab found issues.", "Rust matters."]
        );
        assert_eq!(sources(&HugoParser::default(), POST), ["Microsoft's lab found issues.", "Rust matters."]);
        let doc = hugo().parse(POST);
        assert_eq!(hugo().reconstruct(&doc, &Default::default()), POST);
        let roles: Vec<_> = doc.sections.iter().flat_map(|s| &s.blocks).filter(|b| b.span.is_some()).map(|b| b.role.clone()).collect();
        assert_eq!(roles, [BlockRole::Literal, BlockRole::Literal, BlockRole::None]);
    }

    #[test]
    fn translated_toml_values_stay_valid_strings() {
        let out = translate_all(&hugo(), POST, |s| match s {
            "Memory Safety Bugs" => "\"메모리\" 안전 C:\\버그".to_string(),
            other => format!("[{other}]"),
        });
        assert!(out.starts_with("+++\ntitle = \"\\\"메모리\\\" 안전 C:\\\\버그\"\ndate = 2026-11-09\ndescription = \"[Knowing What You Trust.] [Three patterns.]\"\nslug = \"rust-std-memory-safety\"\n"), "{out}");
        assert!(out.ends_with("+++\n\n[Microsoft's lab found issues.] [Rust matters.]\n"), "{out}");
    }

    #[test]
    fn literal_toml_value_with_an_apostrophe_becomes_a_basic_string() {
        let source = "+++\ntitle = 'Plain'\n+++\nBody.\n";
        let out = translate_all(&hugo(), source, |s| if s == "Plain" { "it's".into() } else { s.into() });
        assert_eq!(out, "+++\ntitle = \"it's\"\n+++\nBody.\n");
    }

    #[test]
    fn yaml_front_matter_offers_the_chosen_keys() {
        let source = "---\ntitle: Plain: title\ndescription: 'It''s here'\nslug: s\n---\n\nBody.\n";
        assert_eq!(sources(&hugo(), source), ["Plain: title", "It''s here", "Body."]);
        let out = translate_all(&hugo(), source, |s| format!("{s}!"));
        assert_eq!(out, "---\ntitle: \"Plain: title!\"\ndescription: 'It''s here!'\nslug: s\n---\n\nBody.!\n");
    }

    #[test]
    fn shortcode_only_lines_are_not_segments() {
        let source = "{{< note >}}\n**A note.** Body text\nstill here.\n{{< /note >}}\n\n{{% details title=\"x\" %}} {{< y >}}\nInside.\n{{% /details %}}\n\nSee {{< ref \"x.md\" >}} here.\n\n```\n{{< inside-code >}}\n```\n\n{{< figure\n  src=\"a.png\"\n>}}\n\nAfter.\n";
        assert_eq!(
            sources(&hugo(), source),
            ["**A note.** Body text still here.", "Inside.", "See {{< ref \"x.md\" >}} here.", "After."]
        );
        let doc = hugo().parse(source);
        assert!(doc.sections.iter().flat_map(|s| &s.blocks).any(|b| b.raw_content.contains("{{< inside-code >}}")));
        let out = translate_all(&hugo(), source, |s| format!("<{s}>"));
        assert!(out.starts_with("{{< note >}}\n<**A note.** Body text still here.>\n{{< /note >}}\n"), "{out}");
        assert!(out.contains("{{< figure\n  src=\"a.png\"\n>}}\n\n<After.>\n"), "{out}");
    }

    #[test]
    fn markdown_parser_still_reads_shortcodes_and_toml_as_text() {
        let doc = crate::MarkdownParser.parse("+++\ntitle = \"x\"\n+++\n\n{{< note >}}\n");
        assert_eq!(doc.translatable_segments().len(), 2);
    }
}
