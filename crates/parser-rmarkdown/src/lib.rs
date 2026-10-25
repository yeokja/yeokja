//! R Markdown flavour of the Markdown parser: knitr chunks and inline R over
//! Pandoc Markdown, as bookdown renders a book.
//!
//! Like the MkDocs parser, this one hands pulldown-cmark a shadow of the
//! source — same length, same newlines — and cuts block text from the real
//! source, so reconstruction splices translations exactly as the plain
//! Markdown parser does. What the shadow changes:
//!
//! * Math (`$..$`, `$$..$$`, `\(..\)`, `\[..\]`, `\begin{..}..\end{..}`) is
//!   found by one forward scan that also knows code fences, code spans and
//!   HTML comments, and is filled with `x`. Nothing inside a formula — a line
//!   starting with `~~~`, `+ ` or `#` — can open a block, and a fence-like
//!   line inside a formula never hides the rest of the file.
//! * Heading attributes (`{#id}`, `{-}`, `{.cls}`, `{#id .cls key=v}`) are
//!   blanked, so they stay verbatim after the translated title.
//! * YAML front matter is blanked; `title`, `subtitle` and `description`
//!   values are offered as literal strings.
//! * Pandoc fenced-div lines (`::: {.note}`, `:::`) are blanked, and so are
//!   the fences of a bookdown prose chunk (```` ```{theorem} ````), whose body
//!   is Markdown.
//! * A knitr chunk that opens inside a raw HTML block (`<div align="center">`
//!   right above it) is blanked whole, so the block ends there instead of at
//!   the chunk's first blank line, after which CommonMark read the rest of
//!   the code as paragraphs and the closing fence as a new one.
//!
//! After pulldown-cmark has cut the blocks, each block is split into
//! sentences with its math masked, so no segment boundary falls inside a
//! formula. Sentences with no prose — only math, inline R, references,
//! citations or anchors — are left out of the spans and stay verbatim. A
//! bookdown text reference `(ref:label)` and a `(PART)`/`(APPENDIX)` heading
//! prefix stay outside the span. knitr chunk captions (`fig.cap`, `fig.alt`)
//! and configured inline-code arguments are offered as literal strings.
//!
//! A segment joins its lines with spaces, so a translation holds display math
//! on one line. Reconstruction writes each display formula back as the source
//! wrote it — its own line breaks, and the break before `$$`/`\begin` and
//! after the closer — since Pandoc reads `\begin{equation}` as math only at
//! the start of a line and bookdown numbers a formula only when `$$` opens a
//! line and `(\#eq:…)` stands on one of its own.

use regex::Regex;
use std::ops::Range;
use std::sync::LazyLock;
use yeokja_core::hash::content_hash;
use yeokja_core::model::*;
use yeokja_core::parser::{DocumentParser, Markup, TranslationMap};
use yeokja_parser_markdown_dialect::{Extra, html_block_ranges, parse_with};
use yeokja_parser_utils::{apply_splices, join_segments_with_translations, resolve_reference_links, split_sentences};

/// The R Markdown parser. `captures` are the source's
/// `inline_code_captures`: regular expressions over the content of an inline
/// code span whose capture groups are offered as literal strings.
#[derive(Default)]
pub struct RmarkdownParser {
    captures: Vec<Regex>,
}

impl RmarkdownParser {
    pub fn new(captures: Vec<Regex>) -> Self {
        Self { captures }
    }

    /// A parser for the given capture patterns, or the first pattern's error.
    pub fn with_patterns<S: AsRef<str>>(patterns: &[S]) -> Result<Self, regex::Error> {
        patterns.iter().map(|p| Regex::new(p.as_ref())).collect::<Result<Vec<_>, _>>().map(Self::new)
    }
}

impl DocumentParser for RmarkdownParser {
    fn markup(&self) -> Markup {
        Markup::RMarkdown
    }

    fn parse(&self, source: &str) -> Document {
        let layout = scan(source);
        let shadow = String::from_utf8(layout.shadow).expect("shadow edits are ASCII over whole characters");
        let doc = parse_with(&shadow, source, true, layout.extras);
        refine(doc, &layout.regions, &self.captures)
    }

    fn reconstruct(&self, document: &Document, translations: &TranslationMap) -> String {
        let translations = resolve_reference_links(document, translations);
        let front = front_matter(&document.source);
        let body = front.as_ref().map_or(0, |f| (f.end + 1).min(document.source.len()));
        let math = scan_regions(&document.source, body).math;
        let mut splices = Vec::new();
        for block in document.sections.iter().flat_map(|s| &s.blocks) {
            let Some(span) = &block.span else { continue };
            if !block.translatable || !block.segments.iter().any(|seg| translations.contains_key(&seg.id)) {
                continue;
            }
            let joined = join_segments_with_translations(&block.segments, &translations);
            let text = if block.role == BlockRole::Literal {
                quote_literal(&document.source, span.clone(), joined, front.clone())
            } else {
                restore_display_lines(&document.source, span, &math, joined)
            };
            splices.push((span.clone(), text));
        }
        apply_splices(&document.source, splices)
    }
}

// ---- regions ---------------------------------------------------------------

/// A fenced code block: the whole block and its opening line.
#[derive(Debug, Clone)]
pub struct Fence {
    pub range: Range<usize>,
    pub opener: Range<usize>,
    /// The closing line, when the fence closes.
    pub closer: Option<Range<usize>>,
    /// A bookdown prose chunk (```` ```{theorem} ````): its body is Markdown,
    /// scanned like the rest of the document.
    pub prose: bool,
}

/// What a forward scan of Pandoc Markdown finds, in document order.
#[derive(Debug, Default, Clone)]
pub struct Regions {
    pub fences: Vec<Fence>,
    pub code_spans: Vec<Range<usize>>,
    pub math: Vec<Range<usize>>,
    pub comments: Vec<Range<usize>>,
}

/// bookdown's theorem and proof engines, whose chunk body is Markdown.
const PROSE_ENGINES: &[&str] = &[
    "theorem", "lemma", "corollary", "proposition", "conjecture", "definition", "example", "exercise",
    "hypothesis", "proof", "remark", "solution",
];

fn line_end(b: &[u8], from: usize) -> usize {
    b[from..].iter().position(|&c| c == b'\n').map_or(b.len(), |p| from + p)
}

/// A fence opener: its character and length.
fn fence_open(line: &str) -> Option<(u8, usize)> {
    let t = line.trim_start();
    for &c in b"`~" {
        let n = t.bytes().take_while(|&b| b == c).count();
        if n >= 3 && !(c == b'`' && t[n..].contains('`')) {
            return Some((c, n));
        }
    }
    None
}

fn fence_closes(line: &str, (c, n): (u8, usize)) -> bool {
    let t = line.trim();
    t.len() >= n && t.bytes().all(|b| b == c)
}

/// The engine of a knitr chunk opener (```` ```{r label, …} ````) and the
/// byte range of its options within the line.
fn chunk_engine(line: &str) -> Option<(&str, Range<usize>)> {
    let indent = line.len() - line.trim_start().len();
    let t = &line[indent..];
    let ticks = t.bytes().take_while(|&b| b == b'`').count();
    let rest = &t[ticks..];
    let lead = rest.len() - rest.trim_start().len();
    let body = rest.trim_start().strip_prefix('{')?;
    let name_len = body.bytes().take_while(|b| b.is_ascii_alphanumeric() || *b == b'_').count();
    if ticks < 3 || name_len == 0 {
        return None;
    }
    let close = body.rfind('}')?;
    let start = indent + ticks + lead + 1;
    Some((&body[..name_len], start..start + close))
}

/// The fence a prose chunk opens with, if `line` opens one.
fn prose_chunk(line: &str) -> Option<(u8, usize)> {
    let (engine, _) = chunk_engine(line)?;
    PROSE_ENGINES.contains(&engine.to_ascii_lowercase().as_str()).then(|| fence_open(line)).flatten()
}

/// Whether the line after the newline at `nl` is blank.
fn blank_line_follows(b: &[u8], nl: usize) -> bool {
    b[nl + 1..].iter().take_while(|&&c| c != b'\n').all(|c| *c == b' ' || *c == b'\t' || *c == b'\r')
}

/// The end of the code span whose `n`-backtick opener ends at `from`.
fn code_span_close(b: &[u8], from: usize, n: usize) -> Option<usize> {
    let mut j = from;
    while j < b.len() {
        match b[j] {
            b'`' => {
                let m = b[j..].iter().take_while(|&&c| c == b'`').count();
                if m == n {
                    return Some(j + m);
                }
                j += m;
            }
            b'\n' if blank_line_follows(b, j) => return None,
            _ => j += 1,
        }
    }
    None
}

/// Pandoc's `tex_math_dollars`: `$$…$$` holds anything but a blank line;
/// `$…$` opens before a non-space and closes after one, not before a digit.
fn dollar_math(b: &[u8], i: usize) -> Option<Range<usize>> {
    if b.get(i + 1) == Some(&b'$') {
        let mut j = i + 2;
        while j + 1 < b.len() {
            match b[j] {
                b'\\' => j += 2,
                b'$' if b[j + 1] == b'$' => return (j > i + 2).then_some(i..j + 2),
                b'\n' if blank_line_follows(b, j) => return None,
                _ => j += 1,
            }
        }
        return None;
    }
    if b.get(i + 1).is_none_or(|c| c.is_ascii_whitespace()) {
        return None;
    }
    // The first `$` closes, or nothing does: `$20,000 and $30,000` is text.
    let mut j = i + 1;
    while j < b.len() {
        match b[j] {
            b'\\' => j += 2,
            b'$' => {
                let closes = !b[j - 1].is_ascii_whitespace() && !b.get(j + 1).is_some_and(u8::is_ascii_digit);
                return closes.then_some(i..j + 1);
            }
            b'\n' if blank_line_follows(b, j) => return None,
            _ => j += 1,
        }
    }
    None
}

/// `\(…\)`, `\[…\]` (Pandoc's `tex_math_single_backslash`, on in R Markdown)
/// and a raw `\begin{env}…\end{env}`.
fn backslash_math(text: &str, i: usize) -> Option<Range<usize>> {
    let b = text.as_bytes();
    let close = match b.get(i + 1) {
        Some(b'(') => b')',
        Some(b'[') => b']',
        Some(b'b') if text[i..].starts_with("\\begin{") => {
            let name_end = text[i + 7..].find('}')? + i + 7;
            let name = &text[i + 7..name_end];
            if name.is_empty() || !name.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'*') {
                return None;
            }
            let closing = format!("\\end{{{name}}}");
            let end = text[name_end..].find(&closing)? + name_end + closing.len();
            return Some(i..end);
        }
        _ => return None,
    };
    let mut j = i + 2;
    while j + 1 < b.len() {
        match b[j] {
            b'\\' if b[j + 1] == close => return Some(i..j + 2),
            b'\\' => j += 2,
            b'\n' if blank_line_follows(b, j) => return None,
            _ => j += 1,
        }
    }
    None
}

/// Scan `text` from `start`: fenced code blocks at line starts, HTML
/// comments, code spans and math, each skipping the others' insides.
pub fn scan_regions(text: &str, start: usize) -> Regions {
    let b = text.as_bytes();
    let mut r = Regions::default();
    let mut open_prose: Option<((u8, usize), Range<usize>)> = None;
    let mut i = start;
    while i < b.len() {
        if i == 0 || b[i - 1] == b'\n' {
            let end = line_end(b, i);
            let line = &text[i..end];
            if let Some((fence, opener)) = &open_prose
                && fence_closes(line, *fence)
            {
                r.fences.push(Fence { range: opener.start..end, opener: opener.clone(), closer: Some(i..end), prose: true });
                open_prose = None;
                i = end;
                continue;
            }
            if open_prose.is_none()
                && let Some(fence) = prose_chunk(line)
            {
                open_prose = Some((fence, i..end));
                i = end;
                continue;
            }
            if let Some(fence) = fence_open(line) {
                let mut j = (end + 1).min(b.len());
                let mut closer = None;
                while j < b.len() {
                    let e = line_end(b, j);
                    if fence_closes(&text[j..e], fence) {
                        closer = Some(j..e);
                        break;
                    }
                    j = (e + 1).min(b.len());
                }
                let stop = closer.as_ref().map_or(b.len(), |c| c.end);
                r.fences.push(Fence { range: i..stop, opener: i..end, closer, prose: false });
                i = stop;
                continue;
            }
        }
        match b[i] {
            b'<' if text[i..].starts_with("<!--") => {
                let end = text[i + 4..].find("-->").map_or(b.len(), |e| i + 4 + e + 3);
                r.comments.push(i..end);
                i = end;
                continue;
            }
            b'`' => {
                let n = b[i..].iter().take_while(|&&c| c == b'`').count();
                match code_span_close(b, i + n, n) {
                    Some(end) => {
                        r.code_spans.push(i..end);
                        i = end;
                    }
                    None => i += n,
                }
                continue;
            }
            b'\\' => {
                if let Some(m) = backslash_math(text, i) {
                    i = m.end;
                    r.math.push(m);
                } else {
                    // An escaped character; a backslash before a newline is a
                    // hard break, and the next line still starts a line.
                    i += if b.get(i + 1) == Some(&b'\n') { 1 } else { 2 };
                }
                continue;
            }
            b'$' => {
                match dollar_math(b, i) {
                    Some(m) => {
                        i = m.end;
                        r.math.push(m);
                    }
                    None => i += b[i..].iter().take_while(|&&c| c == b'$').count(),
                }
                continue;
            }
            _ => i += 1,
        }
    }
    if let Some((_, opener)) = open_prose {
        r.fences.push(Fence { range: opener.start..b.len(), opener: opener.clone(), closer: None, prose: true });
    }
    r
}

/// Math spans of `text` as Pandoc reads them, skipping code.
pub fn math_spans(text: &str) -> Vec<Range<usize>> {
    scan_regions(text, 0).math
}

/// Replace every math span with the same placeholder, for checks that must
/// not read `_` or `*` inside formulas as emphasis.
pub fn mask_math(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut pos = 0;
    for r in math_spans(text) {
        out.push_str(&text[pos..r.start]);
        out.push_str("⟦M⟧");
        pos = r.end;
    }
    out.push_str(&text[pos..]);
    out
}

/// Whether a code span (backticks included) is knitr inline R, `` `r expr` ``.
pub fn is_inline_r(span: &str) -> bool {
    let inner = span.trim_matches('`');
    inner.starts_with("r ") || inner.starts_with("r#") || inner.starts_with("r\t")
}

// ---- preserved tokens ------------------------------------------------------

static TOKEN: LazyLock<Regex> = LazyLock::new(|| {
    let key = r"-?@[\w][\w:.#$%&+?<>~/-]*";
    Regex::new(&[
        r"\\@ref\([^)\s]*\)".to_string(),
        r"\(\\?#[A-Za-z]+:[^)\s]*\)".to_string(),
        r"\(ref:[^)\s]+\)".to_string(),
        r"\[\^[^\]\s]+\]".to_string(),
        r"<a\s+(?:id|name)\s*=[^>]*>\s*</a>".to_string(),
        // The attributes of a bracketed span, `[Table 7.1]{#tab:7.1}`.
        r"\]\{[#.][^{}\s][^{}]*\}".to_string(),
        format!(r"\[{key}(?:\s*;\s*{key})*\]"),
    ]
    .join("|"))
    .expect("token pattern")
});

/// The key of a bare Pandoc citation whose `@` is at `at`, as a range: a
/// letter, digit or `_`, then those and inner punctuation, or a key in braces
/// (`@{frees2009}`), which ends where the brace does.
fn bare_citation(text: &str, at: usize) -> Option<Range<usize>> {
    if text[..at].chars().next_back().is_some_and(|p| p.is_alphanumeric() || p == '\\' || p == '@') {
        return None;
    }
    if text[at + 1..].starts_with('{') {
        let close = text[at + 1..].find('}')?;
        return (close > 1).then_some(at..at + 1 + close + 1);
    }
    let chars: Vec<(usize, char)> = text[at + 1..].char_indices().collect();
    let word = |c: char| c.is_alphanumeric() || c == '_';
    if !chars.first().is_some_and(|(_, c)| word(*c)) {
        return None;
    }
    let mut end = 0;
    let mut k = 0;
    while k < chars.len() {
        let c = chars[k].1;
        if word(c) {
            end = chars[k].0 + c.len_utf8();
            k += 1;
        } else if ":.#$%&-+?<>~/".contains(c) && chars.get(k + 1).is_some_and(|(_, n)| word(*n)) {
            k += 1;
        } else {
            break;
        }
    }
    Some(at..at + 1 + end)
}

/// Ranges of the tokens a translation must copy: cross-references
/// (`\@ref(…)`, `(\#eq:…)`, `(#tab:…)`), text references `(ref:…)`, citations
/// (`@key`, `[@a; @b]`), footnote references `[^n]` and empty anchors
/// `<a id=…></a>` — outside math and code.
pub fn preserved_token_spans(text: &str) -> Vec<Range<usize>> {
    let regions = scan_regions(text, 0);
    let skip: Vec<Range<usize>> =
        regions.math.iter().chain(&regions.code_spans).chain(&regions.comments).cloned().collect();
    let outside = |r: &Range<usize>| !skip.iter().any(|s| s.start < r.end && r.start < s.end);
    let mut found: Vec<Range<usize>> = TOKEN.find_iter(text).map(|m| m.range()).filter(outside).collect();
    for (at, _) in text.match_indices('@') {
        if found.iter().any(|r| r.contains(&at)) {
            continue;
        }
        if let Some(r) = bare_citation(text, at).filter(outside) {
            found.push(r);
        }
    }
    found.sort_by_key(|r| r.start);
    found
}

/// The tokens of `text` a translation must keep (see [`preserved_token_spans`]),
/// sorted, so two texts compare as multisets.
pub fn preserved_tokens(text: &str) -> Vec<String> {
    // `@{key}` is `@key` written so that a particle may follow it.
    let unbrace = |t: &str| match t.strip_prefix("@{").and_then(|k| k.strip_suffix('}')) {
        Some(key) => format!("@{key}"),
        None => t.to_string(),
    };
    let mut tokens: Vec<String> = preserved_token_spans(text).into_iter().map(|r| unbrace(&text[r])).collect();
    tokens.sort_unstable();
    tokens
}

/// The translation with each bare citation of the source that a letter now
/// follows (`@frees2009는`, which Pandoc reads as the key `frees2009는`)
/// written in braces, `@{frees2009}는`.
pub fn brace_glued_citations(source: &str, translation: &str) -> String {
    let keys: Vec<&str> = preserved_token_spans(source)
        .into_iter()
        .map(|r| &source[r])
        .filter(|t| t.starts_with('@') && !t.starts_with("@{"))
        .collect();
    let mut out = translation.to_string();
    for key in keys {
        let mut from = 0;
        while let Some(found) = out[from..].find(key) {
            let at = from + found;
            let end = at + key.len();
            // A Korean letter is a particle; an ASCII one makes another key.
            let glued = out[end..].chars().next().is_some_and(|c| c.is_alphanumeric() && !c.is_ascii());
            if glued && bare_citation(&out, at).is_some() {
                out.replace_range(at..end, &format!("@{{{}}}", &key[1..]));
                from = end + 2;
            } else {
                from = end;
            }
        }
    }
    out
}

static HTML_TAG: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"</?[A-Za-z][^>]*>").expect("tag pattern"));

/// Whether `text` holds prose: a letter outside math, inline R, preserved
/// tokens, HTML tags and comments.
fn is_prose(text: &str) -> bool {
    let regions = scan_regions(text, 0);
    let mut cut: Vec<Range<usize>> = regions.math.clone();
    cut.extend(regions.code_spans.iter().filter(|r| is_inline_r(&text[(*r).clone()])).cloned());
    cut.extend(regions.comments.iter().cloned());
    cut.extend(preserved_token_spans(text));
    cut.extend(HTML_TAG.find_iter(text).map(|m| m.range()));
    text.char_indices().any(|(at, c)| c.is_alphabetic() && !cut.iter().any(|r| r.contains(&at)))
}

// ---- layout ----------------------------------------------------------------

struct Layout {
    shadow: Vec<u8>,
    extras: Vec<Extra>,
    regions: Regions,
}

fn fill(shadow: &mut [u8], range: Range<usize>, with: u8) {
    for b in &mut shadow[range] {
        if *b != b'\n' {
            *b = with;
        }
    }
}

/// Byte ranges of each line, without its newline.
fn lines(source: &str) -> Vec<Range<usize>> {
    let mut out = Vec::new();
    let mut start = 0;
    for (i, b) in source.bytes().enumerate() {
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

/// The YAML front matter, closing line included, when the file starts with it.
fn front_matter(source: &str) -> Option<Range<usize>> {
    let lines = lines(source);
    if lines.first().map(|l| source[l.clone()].trim_end()) != Some("---") {
        return None;
    }
    lines.iter().skip(1).find(|l| matches!(source[(*l).clone()].trim_end(), "---" | "...")).map(|l| 0..l.end)
}

/// Front matter keys whose values are visible text.
const YAML_TEXT_KEYS: &[(&str, BlockType)] =
    &[("title", BlockType::Heading), ("subtitle", BlockType::Heading), ("description", BlockType::Paragraph)];

/// The value ranges of the visible-text keys of the front matter, without
/// their quotes. A quoted value may run over several lines; a plain one takes
/// the following more-indented lines along.
fn yaml_values(source: &str, front: Range<usize>) -> Vec<(Range<usize>, BlockType)> {
    let b = source.as_bytes();
    let all = lines(&source[..front.end]);
    let mut out = Vec::new();
    for (k, line) in all.iter().enumerate() {
        let text = &source[line.clone()];
        let Some(&(key, block_type)) = YAML_TEXT_KEYS.iter().find(|(key, _)| {
            text.strip_prefix(key).is_some_and(|rest| rest.starts_with(':'))
        }) else {
            continue;
        };
        let after = line.start + key.len() + 1;
        let lead = source[after..line.end].len() - source[after..line.end].trim_start().len();
        let start = after + lead;
        if start >= line.end {
            continue;
        }
        let value = match b[start] {
            q @ (b'"' | b'\'') => {
                let mut j = start + 1;
                let mut close = None;
                while j < front.end {
                    if q == b'"' && b[j] == b'\\' {
                        j += 2;
                        continue;
                    }
                    if b[j] == q {
                        if q == b'\'' && b.get(j + 1) == Some(&b'\'') {
                            j += 2;
                            continue;
                        }
                        close = Some(j);
                        break;
                    }
                    j += 1;
                }
                let Some(close) = close else { continue };
                start + 1..close
            }
            b'|' | b'>' => continue,
            _ => {
                let mut end = line.start + text.trim_end().len();
                for next in &all[k + 1..] {
                    let t = &source[next.clone()];
                    if t.trim().is_empty() || !t.starts_with([' ', '\t']) {
                        break;
                    }
                    end = next.start + t.trim_end().len();
                }
                start..end
            }
        };
        if !source[value.clone()].trim().is_empty() {
            out.push((value, block_type));
        }
    }
    out
}

static HEADING_ATTRS: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"^\{\s*(?:(?:#[^\s{}]+|\.[^\s{}]+|-|[\w.-]+=(?:"[^"]*"|'[^']*'|[^\s"'{}]+))\s*)+\}$"#)
        .expect("attribute pattern")
});

/// The attribute block Pandoc takes from the end of a heading line
/// (`header_attributes`), as a range within `line`.
fn heading_attrs(line: &str) -> Option<Range<usize>> {
    let t = line.trim_end();
    if !t.ends_with('}') {
        return None;
    }
    let open = t.rfind('{')?;
    if open == 0 || !t[..open].ends_with([' ', '\t']) || t[..open].trim().is_empty() {
        return None;
    }
    HEADING_ATTRS.is_match(&t[open..]).then_some(open..t.len())
}

fn is_atx_heading(line: &str) -> bool {
    let rest = line.trim_start_matches(' ');
    if line.len() - rest.len() > 3 {
        return false;
    }
    let hashes = rest.bytes().take_while(|&b| b == b'#').count();
    (1..=6).contains(&hashes) && rest[hashes..].chars().next().is_none_or(char::is_whitespace)
}

fn is_setext_underline(line: &str) -> bool {
    let t = line.trim();
    !t.is_empty() && line.len() - line.trim_start().len() <= 3 && (t.bytes().all(|b| b == b'=') || t.bytes().all(|b| b == b'-'))
}

/// A Pandoc fenced-div line: `::: {.cls}`, `::: name`, or a closing `:::`.
fn is_div_fence(line: &str) -> bool {
    let t = line.trim();
    let colons = t.bytes().take_while(|&b| b == b':').count();
    if colons < 3 {
        return false;
    }
    let rest = t[colons..].trim();
    let rest = rest.trim_end_matches(':').trim_end();
    rest.is_empty()
        || (rest.starts_with('{') && rest.ends_with('}'))
        || rest.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
}

/// knitr chunk options whose string value is visible text, by engine.
fn caption_options(engine: &str) -> &'static [&'static str] {
    if PROSE_ENGINES.contains(&engine.to_ascii_lowercase().as_str()) { &["name"] } else { &["fig.cap", "fig.alt"] }
}

/// The string literal values of a chunk's visible-text options, without
/// their quotes. A bookdown text reference (`'(ref:label)'`) is a label, not
/// text, and is left alone.
fn chunk_captions(source: &str, opener: Range<usize>) -> Vec<Range<usize>> {
    let line = &source[opener.clone()];
    let Some((engine, options)) = chunk_engine(line) else { return Vec::new() };
    let mut out = Vec::new();
    let b = line.as_bytes();
    for option in caption_options(engine) {
        for (at, _) in line[options.clone()].match_indices(option) {
            let at = options.start + at;
            let before = line[..at].chars().next_back();
            if before.is_some_and(|c| c.is_alphanumeric() || c == '.' || c == '_') {
                continue;
            }
            let rest = &line[at + option.len()..];
            let lead = rest.len() - rest.trim_start().len();
            let Some(after_eq) = rest.trim_start().strip_prefix('=') else { continue };
            let lead2 = after_eq.len() - after_eq.trim_start().len();
            let q_at = at + option.len() + lead + 1 + lead2;
            let Some(&q @ (b'"' | b'\'')) = b.get(q_at) else { continue };
            let mut j = q_at + 1;
            while j < b.len() && b[j] != q {
                j += if b[j] == b'\\' { 2 } else { 1 };
            }
            if j >= b.len() {
                continue;
            }
            let value = &line[q_at + 1..j];
            if value.trim().is_empty() || REF_LABEL.find(value.trim()).is_some_and(|m| m.end() == value.trim().len()) {
                continue;
            }
            out.push(opener.start + q_at + 1..opener.start + j);
        }
    }
    out
}

fn scan(source: &str) -> Layout {
    let mut shadow = source.as_bytes().to_vec();
    let mut extras = Vec::new();
    let mut body = 0;
    if let Some(front) = front_matter(source) {
        for (value, block_type) in yaml_values(source, front.clone()) {
            extras.push(Extra { range: value, block_type, literal: true });
        }
        fill(&mut shadow, front.clone(), b' ');
        body = (front.end + 1).min(source.len());
    }
    let regions = scan_regions(source, body);
    for fence in &regions.fences {
        for caption in chunk_captions(source, fence.opener.clone()) {
            let block_type = if fence.prose { BlockType::Heading } else { BlockType::Paragraph };
            extras.push(Extra { range: caption, block_type, literal: true });
        }
        if fence.prose {
            fill(&mut shadow, fence.opener.clone(), b' ');
            if let Some(closer) = &fence.closer {
                fill(&mut shadow, closer.clone(), b' ');
            }
        }
    }
    let hidden = |at: usize| {
        regions.fences.iter().any(|f| !f.prose && f.range.contains(&at))
            || regions.comments.iter().chain(&regions.math).any(|r| r.contains(&at))
    };
    let all = lines(source);
    for (k, line) in all.iter().enumerate() {
        if line.start < body || hidden(line.start) {
            continue;
        }
        let text = &source[line.clone()];
        if is_div_fence(text) {
            fill(&mut shadow, line.clone(), b' ');
            continue;
        }
        let setext = all.get(k + 1).is_some_and(|next| is_setext_underline(&source[next.clone()]))
            && !text.trim().is_empty();
        if (is_atx_heading(text) || setext)
            && let Some(attrs) = heading_attrs(text)
        {
            let start = line.start + attrs.start;
            if !regions.math.iter().any(|m| m.contains(&start)) {
                fill(&mut shadow, start..line.start + attrs.end, b' ');
            }
        }
    }
    for m in &regions.math {
        fill(&mut shadow, m.clone(), b'x');
    }
    // A knitr chunk inside a raw HTML block: blank it, so the block ends at
    // the chunk instead of at the chunk's first blank line, after which the
    // rest of the code read as paragraphs and the closing fence as a new one.
    let html = html_block_ranges(std::str::from_utf8(&shadow).expect("shadow edits are ASCII over whole characters"));
    for fence in regions.fences.iter().filter(|f| !f.prose) {
        if html.iter().any(|h| h.contains(&fence.opener.start)) {
            fill(&mut shadow, fence.range.clone(), b' ');
        }
    }
    extras.sort_by_key(|e| e.range.start);
    Layout { shadow, extras, regions }
}

// ---- refining blocks -------------------------------------------------------

static REF_LABEL: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^\(ref:[^)\s]+\)").expect("ref pattern"));
static PART_LABEL: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\((?:PART\*?|APPENDIX)\)").expect("part pattern"));

/// The length of a label that opens a block and is not text: a bookdown text
/// reference definition `(ref:label)` before a paragraph, a `(PART)` or
/// `(APPENDIX)` before a heading.
fn label_prefix(raw: &str, block_type: BlockType) -> usize {
    let pattern = match block_type {
        BlockType::Paragraph => &REF_LABEL,
        BlockType::Heading => &PART_LABEL,
        _ => return 0,
    };
    match pattern.find(raw) {
        Some(m) => m.end() + (raw[m.end()..].len() - raw[m.end()..].trim_start().len()),
        None => 0,
    }
}

/// `normalize_inline_text` over `source[range]`, with the source offset of
/// every byte of the result.
fn normalize_with_map(source: &str, range: Range<usize>) -> (String, Vec<usize>) {
    let raw = &source[range.clone()];
    let mut out = String::new();
    let mut map = Vec::new();
    let push = |out: &mut String, map: &mut Vec<usize>, text: &str, at: usize| {
        out.push_str(text);
        map.extend(at..at + text.len());
    };
    let mut pos = 0;
    for (k, line) in raw.split('\n').enumerate() {
        let at = range.start + pos;
        pos += line.len() + 1;
        if k == 0 {
            push(&mut out, &mut map, line.trim_end(), at);
            continue;
        }
        let lead = line.len() - line.trim_start_matches(|c: char| c == '>' || c.is_whitespace()).len();
        let stripped = line[lead..].trim_end();
        if stripped.is_empty() {
            continue;
        }
        if !out.is_empty() {
            out.push(' ');
            map.push(at + lead);
        }
        push(&mut out, &mut map, stripped, at + lead);
    }
    (out, map)
}

/// One sentence of a block: its source range and normalized text.
#[derive(Clone)]
struct Piece {
    range: Range<usize>,
    text: String,
}

/// The sentences of `source[range]`, split with math masked so that no
/// boundary falls inside a formula.
fn pieces(source: &str, range: Range<usize>, math: &[Range<usize>]) -> Vec<Piece> {
    let (norm, map) = normalize_with_map(source, range);
    let inside_math = |at: usize| math.iter().any(|m| m.contains(&at));
    let mut masked = String::with_capacity(norm.len());
    let mut masked_at = Vec::new();
    let mut norm_at = Vec::new();
    for (at, c) in norm.char_indices() {
        masked_at.push(masked.len());
        norm_at.push(at);
        masked.push(if inside_math(map[at]) { 'x' } else { c });
    }
    masked_at.push(masked.len());
    norm_at.push(norm.len());
    let mut out = Vec::new();
    let mut cursor = 0;
    for sentence in split_sentences(&masked) {
        let Some(found) = masked[cursor..].find(&sentence) else { continue };
        let (mb, me) = (cursor + found, cursor + found + sentence.len());
        cursor = me;
        let (Ok(cb), Ok(ce)) = (masked_at.binary_search(&mb), masked_at.binary_search(&me)) else { continue };
        let (nb, ne) = (norm_at[cb], norm_at[ce]);
        if nb >= ne {
            continue;
        }
        out.push(Piece { range: map[nb]..map[ne - 1] + 1, text: norm[nb..ne].to_string() });
    }
    // A sentence boundary inside emphasis (`**Example 7.2.1. Exam
    // Question.**`, `*a. Claim Frequency.*`) leaves both halves with a mark
    // open: keep them together.
    let mut merged: Vec<Piece> = Vec::new();
    for piece in out {
        match merged.last_mut() {
            Some(last)
                if open_marks(&source[last.range.clone()]) != (false, false)
                    && open_marks(&source[last.range.clone()]) == open_marks(&source[piece.range.clone()]) =>
            {
                last.range.end = piece.range.end;
                last.text = yeokja_parser_utils::normalize_inline_text(&source[last.range.clone()]);
            }
            _ => merged.push(piece),
        }
    }
    merged
}

/// Whether `text` leaves `*` emphasis and `**` strong emphasis open, counted
/// over its runs of `*` outside code and math: a run of three is both.
fn open_marks(text: &str) -> (bool, bool) {
    let regions = scan_regions(text, 0);
    let hidden: Vec<Range<usize>> = regions.code_spans.iter().chain(&regions.math).cloned().collect();
    let b = text.as_bytes();
    let (mut single, mut double) = (0, 0);
    let mut i = 0;
    while i < b.len() {
        if b[i] != b'*' || hidden.iter().any(|r| r.contains(&i)) || (i > 0 && b[i - 1] == b'\\') {
            i += 1;
            continue;
        }
        let run = b[i..].iter().take_while(|&&c| c == b'*').count();
        single += run % 2;
        double += usize::from(run >= 2);
        i += run;
    }
    (single % 2 == 1, double % 2 == 1)
}

/// Whether a formula is display math: `$$…$$`, `\[…\]` or a `\begin{…}`
/// environment.
fn is_display(formula: &str) -> bool {
    ["$$", "\\[", "\\begin"].iter().any(|open| formula.starts_with(open))
}

/// Inline R code (`` `r SolnBegin()` ``) and display math, which a block
/// may open or close with and which are no words of its sentences.
fn edge_atoms(source: &str, regions: &Regions) -> Vec<Range<usize>> {
    let mut atoms: Vec<Range<usize>> =
        regions.code_spans.iter().filter(|s| is_inline_r(&source[(*s).clone()])).cloned().collect();
    atoms.extend(regions.math.iter().filter(|m| is_display(&source[(*m).clone()])).cloned());
    atoms
}

/// `piece` without the inline R code or display math that opens or closes
/// it (`` `r SolnBegin()` Since… ``, `` …holds. `r SolnEnd()` ``,
/// `$$…$$ **Commands**`): they stay verbatim outside the span.
fn trim_edges(source: &str, piece: Piece, atoms: &[Range<usize>], lead: bool, tail: bool) -> Piece {
    let mut range = piece.range.clone();
    let leading = |range: &Range<usize>| {
        let space = source[range.clone()].len() - source[range.clone()].trim_start().len();
        atoms.iter().find(|s| s.start == range.start + space && s.end < range.end).cloned()
    };
    while let Some(span) = leading(&range).filter(|_| lead) {
        range.start = span.end;
    }
    let trailing = |range: &Range<usize>| {
        let space = source[range.clone()].len() - source[range.clone()].trim_end().len();
        atoms.iter().find(|s| s.end == range.end - space && s.start > range.start).cloned()
    };
    while let Some(span) = trailing(&range).filter(|_| tail) {
        range.end = span.start;
    }
    let text = &source[range.clone()];
    let range = range.start + (text.len() - text.trim_start().len())..range.start + text.trim_end().len();
    if range == piece.range {
        return piece;
    }
    let text = yeokja_parser_utils::normalize_inline_text(&source[range.clone()]);
    Piece { range, text }
}

fn block_of(template: &Block, run: &[Piece], source: &str, role: BlockRole, atoms: &[Range<usize>]) -> Block {
    let mut run = run.to_vec();
    let last = run.len() - 1;
    if role != BlockRole::Literal {
        run[0] = trim_edges(source, run[0].clone(), atoms, true, last == 0);
        if last > 0 {
            run[last] = trim_edges(source, run[last].clone(), atoms, false, true);
        }
    }
    let range = run[0].range.start..run[run.len() - 1].range.end;
    let segments = run
        .iter()
        .map(|p| Segment {
            id: SegmentId::new(0, 0, 0),
            source: p.text.clone(),
            source_hash: content_hash(&p.text),
            block_type: template.block_type,
        })
        .collect();
    Block {
        block_type: template.block_type,
        segments,
        raw_content: source[range.clone()].to_string(),
        heading_level: template.heading_level,
        span: Some(range),
        translatable: true,
        role,
    }
}

/// Capture groups of `captures` within the inline code spans of `range`.
fn captured(source: &str, range: Range<usize>, code_spans: &[Range<usize>], captures: &[Regex]) -> Vec<Range<usize>> {
    let mut out = Vec::new();
    if captures.is_empty() {
        return out;
    }
    for span in code_spans.iter().filter(|s| s.start >= range.start && s.end <= range.end) {
        let run = source[span.clone()].bytes().take_while(|&b| b == b'`').count();
        let content = span.start + run..span.end - run;
        for pattern in captures {
            for caps in pattern.captures_iter(&source[content.clone()]) {
                for group in caps.iter().skip(1).flatten() {
                    if !group.as_str().trim().is_empty() {
                        out.push(content.start + group.start()..content.start + group.end());
                    }
                }
            }
        }
    }
    out.sort_by_key(|r| r.start);
    out.dedup();
    out
}

/// Re-cut every translatable block into sentences, keep sentences without
/// prose out of the spans, and offer configured inline-code captures.
fn refine(doc: Document, regions: &Regions, captures: &[Regex]) -> Document {
    let Document { sections, source } = doc;
    let atoms = edge_atoms(&source, regions);
    let mut out = Vec::new();
    for section in sections {
        let mut blocks = Vec::new();
        for block in section.blocks {
            let Some(span) = block.span.clone().filter(|_| block.translatable) else {
                blocks.push(block);
                continue;
            };
            let start = if block.role == BlockRole::None {
                span.start + label_prefix(&source[span.clone()], block.block_type)
            } else {
                span.start
            };
            let mut emitted = false;
            let mut run: Vec<Piece> = Vec::new();
            for piece in pieces(&source, start..span.end, &regions.math) {
                if is_prose(&source[piece.range.clone()]) {
                    run.push(piece);
                    continue;
                }
                if !run.is_empty() {
                    blocks.push(block_of(&block, &run, &source, block.role.clone(), &atoms));
                    run.clear();
                    emitted = true;
                }
                if block.role == BlockRole::Literal {
                    continue;
                }
                for capture in captured(&source, piece.range.clone(), &regions.code_spans, captures) {
                    let text = source[capture.clone()].to_string();
                    let literal = Block {
                        block_type: BlockType::Paragraph,
                        heading_level: None,
                        ..block.clone()
                    };
                    let run = [Piece { range: capture, text }];
                    blocks.push(block_of(&literal, &run, &source, BlockRole::Literal, &atoms));
                }
            }
            if !run.is_empty() {
                blocks.push(block_of(&block, &run, &source, block.role.clone(), &atoms));
                emitted = true;
            }
            if !emitted {
                blocks.push(Block { segments: Vec::new(), span: None, translatable: false, ..block });
            }
        }
        out.push(Section { blocks });
    }
    for (s, section) in out.iter_mut().enumerate() {
        for (b, block) in section.blocks.iter_mut().enumerate() {
            for (k, segment) in block.segments.iter_mut().enumerate() {
                segment.id = SegmentId::new(s, b, k);
            }
        }
    }
    Document { sections: out, source }
}

// ---- reconstruction ---------------------------------------------------------

/// Characters that lead a line inside a block: indentation and blockquote
/// markers, which `normalize_inline_text` drops from continuation lines.
const LINE_LEAD: [char; 3] = [' ', '\t', '>'];

/// `text`, a block's translation, with every display formula of its source
/// span written as the source wrote it: its own line breaks, and the break
/// (with the next line's lead) before its opener and after its closer when
/// the source had them. The formula is found in the translation however it
/// is spaced, since a sentence joins the source's lines with spaces.
fn restore_display_lines(source: &str, span: &Range<usize>, math: &[Range<usize>], mut text: String) -> String {
    let mut cursor = 0;
    for m in math.iter().filter(|m| span.start <= m.start && m.end <= span.end) {
        let formula = &source[m.clone()];
        let before = &source[span.start..m.start];
        let after = &source[m.end..span.end];
        let opens_line = before.trim_end_matches(LINE_LEAD).ends_with('\n');
        let closes_line = after.trim_start_matches(LINE_LEAD).starts_with('\n');
        if !is_display(formula) || !(formula.contains('\n') || opens_line || closes_line) {
            continue;
        }
        let Some((start, end)) = find_respaced(&text, formula, cursor) else { continue };
        let mut range = start..end;
        let mut written = formula.to_string();
        if opens_line {
            let lead = text[..start].trim_end().len();
            if lead > 0 {
                range.start = lead;
                written.insert_str(0, &before[before.rfind('\n').map_or(0, |nl| nl + 1)..]);
                written.insert(0, '\n');
            }
        }
        if closes_line {
            let tail = text.len() - text[end..].trim_start().len();
            if tail < text.len() {
                range.end = tail;
                let next = &after.trim_start_matches(LINE_LEAD)[1..];
                written.push('\n');
                written.push_str(&next[..next.len() - next.trim_start_matches(LINE_LEAD).len()]);
            }
        }
        text.replace_range(range.clone(), &written);
        cursor = range.start + written.len();
    }
    text
}

/// Where `needle` occurs in `hay` at or after `from`, with any whitespace,
/// none included, wherever `needle` has some.
fn find_respaced(hay: &str, needle: &str, from: usize) -> Option<(usize, usize)> {
    let tokens: Vec<&str> = needle.split_whitespace().collect();
    let first = tokens.first()?;
    let mut at = from;
    while let Some(found) = hay[at..].find(first) {
        let start = at + found;
        let mut pos = start + first.len();
        let matched = tokens[1..].iter().all(|token| {
            let skipped = hay.len() - hay[pos..].trim_start().len();
            let hit = hay[skipped..].starts_with(token);
            if hit {
                pos = skipped + token.len();
            }
            hit
        });
        if matched {
            return Some((start, pos));
        }
        at = start + first.chars().next().map_or(1, char::len_utf8);
    }
    None
}

/// Put `\` before each `quote` the translation leaves unescaped.
fn escape_quote(text: &str, quote: char) -> String {
    let mut out = String::with_capacity(text.len());
    let mut backslashes = 0;
    for c in text.chars() {
        if c == quote && backslashes % 2 == 0 {
            out.push('\\');
        }
        backslashes = if c == '\\' { backslashes + 1 } else { 0 };
        out.push(c);
    }
    out
}

/// A plain YAML scalar the translation would end or change: quote it.
fn yaml_plain(text: String) -> String {
    let risky = text.contains(": ")
        || text.contains(" #")
        || text.ends_with(':')
        || text.starts_with(['-', '?', ':', ',', '[', ']', '{', '}', '#', '&', '*', '!', '|', '>', '\'', '"', '%', '@', '`']);
    if risky { format!("\"{}\"", text.replace('\\', "\\\\").replace('"', "\\\"")) } else { text }
}

/// A literal string's translation, written the way its quotes need: an R or
/// YAML double-quoted string escapes its quote with `\`, a YAML single-quoted
/// one doubles it, and a plain YAML value is quoted when it has to be.
fn quote_literal(source: &str, span: Range<usize>, text: String, front: Option<Range<usize>>) -> String {
    let b = source.as_bytes();
    let in_yaml = front.is_some_and(|f| f.contains(&span.start));
    let before = span.start.checked_sub(1).map(|i| b[i]);
    match (before, b.get(span.end).copied()) {
        (Some(q @ (b'"' | b'\'')), Some(after)) if after == q => {
            if in_yaml && q == b'\'' {
                text.replace("''", "'").replace('\'', "''")
            } else {
                escape_quote(&text, q as char)
            }
        }
        _ if in_yaml => yaml_plain(text),
        _ => text,
    }
}

#[cfg(test)]
mod tests;
