//! CommonMark (mdBook) codec for inline tag transport.
//!
//! A segment's inline markup becomes numbered tags the model moves along with
//! the words they wrap — `<b1>…</b1>` bold, `<i2>…</i2>` italic, `<s3>…</s3>`
//! strikethrough, `<a4>…</a4>` a link's visible text, `<h5>…</h5>` balanced
//! inline HTML, `<x6/>` anything copied as is, `<m7/>` math — while code stays
//! a backtick span the model can read: Korean picks its particle by the sound
//! the code ends in, and hiding the code made that a coin flip. The serializer
//! writes the Markdown back, choosing for each pair a form CommonMark closes
//! where Korean put it.

use crate::evaluator_format::{commonmark_emphasis, stands_for, written_as_text};
use pulldown_cmark::{BrokenLink, CowStr, Event, LinkType, Options, Parser, Tag as MdTag, TagEnd};
use std::collections::{HashMap, HashSet};
use serde::{Deserialize, Serialize};
use std::ops::Range;

/// A set written in a fixed order, so a serialized request is reproducible.
fn sorted<S: serde::Serializer>(set: &HashSet<String>, serializer: S) -> Result<S::Ok, S::Error> {
    let mut items: Vec<&String> = set.iter().collect();
    items.sort();
    serializer.collect_seq(items)
}

/// The Markdown a source is written in. They share CommonMark's inline
/// rules — markdown-it (MyST) and micromark (MDX) implement the same
/// delimiter algorithm as pulldown-cmark — and differ in what they add.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Dialect {
    /// mdBook: CommonMark with strikethrough, footnotes and MathJax.
    #[default]
    CommonMark,
    /// MyST for Sphinx: roles (`` {ref}`text <label>` ``); no strikethrough.
    Myst,
    /// MDX: `{…}` expressions and JSX; a `{` or a `<` that could open one
    /// must be escaped in text.
    Mdx,
}

impl Dialect {
    /// The dialect of a source read by `parser`.
    pub fn for_parser(parser: &str) -> Self {
        match parser {
            "myst" => Dialect::Myst,
            "mdx" => Dialect::Mdx,
            _ => Dialect::CommonMark,
        }
    }
}

/// Document-level facts a segment cannot show on its own.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct DocContext {
    /// Reference definition labels, normalized: `[text][label]` and `[label]`
    /// are links only when the document defines the label.
    #[serde(serialize_with = "sorted")]
    reference_labels: HashSet<String>,
    dialect: Dialect,
}

impl DocContext {
    pub fn new(source: &str, dialect: Dialect) -> Self {
        let parser = Parser::new_ext(source, options(dialect));
        let reference_labels =
            parser.reference_definitions().iter().map(|(label, _)| normalize_label(label)).collect();
        Self { reference_labels, dialect }
    }

    pub fn from_markdown(source: &str) -> Self {
        Self::new(source, Dialect::CommonMark)
    }

    pub fn dialect(&self) -> Dialect {
        self.dialect
    }
}

/// Where a segment sits, for the escapes that depend on it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Position {
    #[default]
    Inline,
    /// A table cell, where a bare `|` ends the cell.
    TableCell,
    /// A string its container quotes rather than parses — a front matter
    /// value, a JSX attribute, a toctree entry title. No markup: it goes to
    /// the model as text and comes back as text.
    Plain,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum TagKind {
    Italic,
    Bold,
    Strike,
    Link,
    Html,
    Opaque,
    Math,
    Code,
    /// A MyST role's visible label (`` {ref}`label <target>` ``).
    Role,
}

impl TagKind {
    pub(super) fn letter(self) -> char {
        match self {
            TagKind::Italic => 'i',
            TagKind::Bold => 'b',
            TagKind::Strike => 's',
            TagKind::Link => 'a',
            TagKind::Html => 'h',
            TagKind::Opaque => 'x',
            TagKind::Math => 'm',
            TagKind::Code => 'c',
            // A cross-reference's label reads to the model as a link's text.
            TagKind::Role => 'a',
        }
    }

    pub(super) fn is_emphasis(self) -> bool {
        matches!(self, TagKind::Italic | TagKind::Bold | TagKind::Strike)
    }

    fn is_void(self) -> bool {
        matches!(self, TagKind::Opaque | TagKind::Math | TagKind::Code)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Tag {
    pub n: usize,
    pub kind: TagKind,
    /// Paired: the opening mark as the source wrote it (`**`, `_`, `[`,
    /// `<kbd>`). Void and code: the construct's source text.
    pub open: String,
    /// Paired: the closing mark, a link's tail (`](url)`, `][ref]`, `]`), or
    /// the closing HTML tag.
    pub close: String,
    /// A shortcut or collapsed reference link's label, as written.
    pub label: Option<String>,
    /// Code: the span's content as CommonMark reads it.
    pub code: Option<String>,
    /// A MyST role: its name and, for a label role, what it points at.
    pub role: Option<RoleRef>,
    /// How the construct reads back, for verifying a rendering.
    shape: String,
    /// The text it contributes when read back (math renders its own source).
    visible: String,
}

/// A MyST role, apart from what the model sees of it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoleRef {
    pub name: String,
    /// A label role's target; empty for a role shown as written.
    pub target: String,
    /// The source wrote no explicit label: `` {term}`Nix` `` shows its
    /// target.
    pub implicit: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Tagged {
    /// The segment's source string.
    pub source: String,
    /// The text the model sees.
    pub text: String,
    pub tags: Vec<Tag>,
    pub position: Position,
    pub dialect: Dialect,
}

/// A segment the tags cannot represent: its source leaves a mark open, as
/// when the sentence splitter cut through an emphasis or a code span.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Untaggable(pub String);

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Node {
    Text(String),
    Open(usize),
    Close(usize),
    Void(usize),
    /// A code span of the reply: the source code it was matched to, or `None`
    /// for plain source text the translation marked up as code.
    Code { tag: Option<usize>, written: String },
}

#[derive(Clone, Debug)]
pub struct Tree {
    pub nodes: Vec<Node>,
    /// What the lenient reading had to correct.
    pub notes: Vec<String>,
}

#[derive(Clone, Debug)]
pub struct Rendered {
    pub markdown: String,
    /// The form chosen for each emphasis that could not keep its own.
    pub strategies: Vec<(usize, &'static str)>,
    /// Whether the Markdown reads back as the tags meant.
    pub verified: bool,
}

/// Parsed alone, a segment such as `1.` or `- same -` reads as a block
/// marker; in its document it is inline text. A letter-like guard and a space
/// in front keep it inline without changing how its first mark flanks.
const GUARD: char = '\u{E0FF}';
const GUARD_PREFIX: &str = "\u{E0FF} ";
const MASK_BASE: u32 = 0xE000;
/// Wraps each masked construct so the marks around it flank as they did
/// against the construct's own punctuation (`**…**[^core]` still closes).
const MASK_EDGE: char = '\u{2E3A}';

fn options(dialect: Dialect) -> Options {
    let mut options = Options::empty();
    if dialect != Dialect::Myst {
        options.insert(Options::ENABLE_STRIKETHROUGH);
    }
    options.insert(Options::ENABLE_FOOTNOTES);
    options
}

pub(crate) fn normalize_label(label: &str) -> String {
    label.split_whitespace().collect::<Vec<_>>().join(" ").to_lowercase()
}

fn events<'a>(text: &'a str, ctx: &DocContext) -> Vec<(Event<'a>, Range<usize>)> {
    let labels = &ctx.reference_labels;
    let callback = |link: BrokenLink<'a>| {
        labels.contains(&normalize_label(&link.reference)).then(|| (CowStr::from(""), CowStr::from("")))
    };
    Parser::new_with_broken_link_callback(text, options(ctx.dialect), Some(callback)).into_offset_iter().collect()
}

/// Byte ranges of the backtick code spans in `text`, marks included.
pub(super) fn code_span_ranges(text: &str) -> Vec<Range<usize>> {
    let b = text.as_bytes();
    let mut ranges = Vec::new();
    let mut i = 0;
    while i < b.len() {
        if b[i] != b'`' || (i > 0 && b[i - 1] == b'\\') {
            i += 1;
            continue;
        }
        let run = b[i..].iter().take_while(|c| **c == b'`').count();
        let mut j = i + run;
        let mut close = None;
        while j < b.len() {
            if b[j] == b'`' {
                let r = b[j..].iter().take_while(|c| **c == b'`').count();
                if r == run {
                    close = Some(j);
                    break;
                }
                j += r;
            } else {
                j += 1;
            }
        }
        match close {
            Some(end) => {
                ranges.push(i..end + run);
                i = end + run;
            }
            None => i += run,
        }
    }
    ranges
}

/// What a masked construct is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Masked {
    /// mdBook MathJax, `\\(…\\)` or `\\[…\\]`.
    Math,
    /// `[^x]`, whose definition lives elsewhere in the document.
    Footnote,
    /// A MyST role: `{name}` and the backtick span right after it.
    Role,
    /// An MDX expression, `{…}`.
    Expression,
}

/// A segment with each construct it cannot read alone as one private-use
/// character (see [`mask`]).
struct Masking {
    text: String,
    spans: Vec<(Masked, String)>,
    /// An MDX `{` that no `}` closes: the segment cannot be tagged.
    open_expression: bool,
}

/// MyST role names, as myst-parser reads them.
fn is_role_name_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || matches!(c, '_' | '+' | ':' | '-')
}

/// The length of a `{name}` role head at the start of `text`.
fn role_head(text: &str) -> Option<usize> {
    let rest = text.strip_prefix('{')?;
    let name = rest.chars().take_while(|c| is_role_name_char(*c)).count();
    (name > 0 && rest[name..].starts_with('}')).then_some(name + 2)
}

/// Roles that take an explicit label, `` {ref}`label <target>` ``, the way
/// Sphinx's cross-reference roles do. A domain role (`py:func`) does too.
const LABEL_ROLES: &[&str] =
    &["ref", "doc", "term", "numref", "any", "download", "keyword", "option", "envvar", "token", "pep", "rfc"];

/// A masked role's name, its backtick span and the span's content.
fn split_role(raw: &str) -> (&str, &str, String) {
    let head = role_head(raw).unwrap_or(0);
    let span = &raw[head..];
    let run = span.chars().take_while(|c| *c == '`').count();
    let inner = span.get(run..span.len().saturating_sub(run)).unwrap_or("");
    (raw.get(1..head.saturating_sub(1)).unwrap_or(""), span, code_content(inner))
}

/// How a role reads to the model: a label to translate, with its target,
/// or `None` for a role shown as written. Sphinx splits `label <target>` at
/// the last `<` (`split_explicit_title`); `{term}` shows its target when it
/// has no label. A label holding a backtick stays as written: the model
/// would read it as code, which a label cannot hold.
fn role_label(name: &str, content: &str) -> Option<(String, String, bool)> {
    if content.contains('`') {
        return None;
    }
    if (LABEL_ROLES.contains(&name) || name.contains(':'))
        && let Some(body) = content.strip_suffix('>')
        && let Some(open) = body.rfind('<')
        && !body[..open].ends_with('\\')
        && !body[..open].trim_end().is_empty()
    {
        return Some((body[..open].trim_end().to_string(), body[open + 1..].to_string(), false));
    }
    (name == "term").then(|| (content.to_string(), content.to_string(), true))
}

/// A label role written back with `label`: as the source wrote it when the
/// label did not change, else in the explicit form.
fn write_role(source: &str, role: &RoleRef, original: &str, label: &str) -> String {
    if label == original {
        return source.to_string();
    }
    let body = if role.implicit && label == role.target { label.to_string() } else { format!("{label} <{}>", role.target) };
    let longest = body.split(|c| c != '`').map(str::len).max().unwrap_or(0);
    let marks = "`".repeat(longest + 1);
    let pad = if body.starts_with('`') || body.ends_with('`') { " " } else { "" };
    format!("{{{}}}{marks}{pad}{body}{pad}{marks}", role.name)
}

/// The end of the MDX expression whose `{` is at `start`, past the `}` that
/// balances it.
fn expression_end(source: &str, start: usize) -> Option<usize> {
    let mut depth = 0usize;
    for (at, c) in source[start..].char_indices() {
        match c {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(start + at + 1);
                }
            }
            _ => {}
        }
    }
    None
}

/// The constructs a segment read alone would misread, each as one private-use
/// character: footnote references everywhere, mdBook MathJax in CommonMark,
/// MyST roles, MDX expressions. Code spans are left alone: `` `[^.,]+` `` is
/// a pattern, not a footnote.
fn mask(source: &str, dialect: Dialect) -> Masking {
    let code = code_span_ranges(source);
    let mut out = String::new();
    let mut spans: Vec<(Masked, String)> = Vec::new();
    let mut open_expression = false;
    let mut push = |out: &mut String, kind: Masked, text: &str| {
        out.push(MASK_EDGE);
        out.push(char::from_u32(MASK_BASE + spans.len() as u32).expect("private use"));
        out.push(MASK_EDGE);
        spans.push((kind, text.to_string()));
    };
    let mut i = 0;
    while i < source.len() {
        if let Some(span) = code.iter().find(|r| r.start == i) {
            out.push_str(&source[span.clone()]);
            i = span.end;
            continue;
        }
        let rest = &source[i..];
        let escaped = source[..i].ends_with('\\') && !source[..i].ends_with("\\\\");
        if dialect == Dialect::Myst
            && !escaped
            && let Some(head) = role_head(rest)
            && let Some(span) = code.iter().find(|r| r.start == i + head)
        {
            push(&mut out, Masked::Role, &source[i..span.end]);
            i = span.end;
            continue;
        }
        if dialect == Dialect::Mdx && !escaped && rest.starts_with('{') {
            match expression_end(source, i) {
                Some(end) => {
                    push(&mut out, Masked::Expression, &source[i..end]);
                    i = end;
                    continue;
                }
                None => open_expression = true,
            }
        }
        let mut openings = vec![("[^", "]", Masked::Footnote)];
        if dialect == Dialect::CommonMark {
            openings.extend([("\\\\(", "\\\\)", Masked::Math), ("\\\\[", "\\\\]", Masked::Math)]);
        }
        let opening = openings
            .into_iter()
            .find(|(open, _, kind)| rest.starts_with(open) && !(escaped && *kind == Masked::Footnote));
        if let Some((open, close, kind)) = opening
            && let Some(rel) = rest[open.len()..].find(close)
        {
            let end = i + open.len() + rel + close.len();
            // A construct is never cut by a code span.
            if !code.iter().any(|r| r.start < end && r.end > i) {
                push(&mut out, kind, &source[i..end]);
                i = end;
                continue;
            }
        }
        let c = rest.chars().next().expect("char boundary");
        out.push(c);
        i += c.len_utf8();
    }
    Masking { text: out, spans, open_expression }
}

fn masked_index(c: char, masked: &[(Masked, String)]) -> Option<usize> {
    let v = c as u32;
    (MASK_BASE..MASK_BASE + masked.len() as u32).contains(&v).then(|| (v - MASK_BASE) as usize)
}

fn unmask(text: &str, masked: &[(Masked, String)]) -> String {
    let mut out = String::new();
    for piece in split_masked(text, masked) {
        match piece {
            Ok(c) => out.push(c),
            Err(i) => out.push_str(&masked[i].1),
        }
    }
    out
}

/// The characters of `text`, with each edge-wrapped masked construct as its
/// index.
fn split_masked(text: &str, masked: &[(Masked, String)]) -> Vec<Result<char, usize>> {
    let chars: Vec<char> = text.chars().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == MASK_EDGE
            && let Some(at) = chars.get(i + 1).and_then(|c| masked_index(*c, masked))
            && chars.get(i + 2) == Some(&MASK_EDGE)
        {
            out.push(Err(at));
            i += 3;
            continue;
        }
        out.push(Ok(chars[i]));
        i += 1;
    }
    out
}

fn xml_escape(c: char, out: &mut String) {
    match c {
        '&' => out.push_str("&amp;"),
        '<' => out.push_str("&lt;"),
        '>' => out.push_str("&gt;"),
        _ => out.push(c),
    }
}

fn decode(text: &str) -> String {
    text.replace("&lt;", "<").replace("&gt;", ">").replace("&amp;", "&")
}

/// The element name of an inline HTML tag, and whether it closes.
fn html_tag(html: &str) -> Option<(String, bool)> {
    let body = html.strip_prefix('<')?;
    let (closing, body) = match body.strip_prefix('/') {
        Some(rest) => (true, rest),
        None => (false, body),
    };
    let name: String = body.chars().take_while(|c| c.is_ascii_alphanumeric() || *c == '-').collect();
    if name.is_empty() || html.ends_with("/>") {
        return None;
    }
    Some((name.to_ascii_lowercase(), closing))
}

/// Opening and closing inline HTML events that form a pair at the same depth.
fn pair_inline_html(events: &[(Event, Range<usize>)]) -> HashMap<usize, usize> {
    let mut pairs = HashMap::new();
    let mut open: Vec<(String, usize, usize)> = Vec::new();
    let mut depth = 0usize;
    for (i, (event, _)) in events.iter().enumerate() {
        match event {
            Event::Start(_) => depth += 1,
            Event::End(_) => depth = depth.saturating_sub(1),
            Event::InlineHtml(html) => match html_tag(html) {
                Some((name, false)) => open.push((name, i, depth)),
                Some((name, true)) => {
                    if let Some(pos) = open.iter().rposition(|(n, _, d)| *n == name && *d == depth) {
                        pairs.insert(open[pos].1, i);
                        open.truncate(pos);
                    }
                }
                None => {}
            },
            _ => {}
        }
    }
    pairs
}

pub fn tagify(source: &str, ctx: &DocContext, position: Position, counter: &mut usize) -> Result<Tagged, Untaggable> {
    tagify_with(source, ctx, position, counter, false)
}

/// [`tagify`] for Markdown a translation wrote: marks it left open stay
/// text instead of refusing the whole segment, so that an audit can still
/// read the rest of it.
pub(super) fn tagify_lenient(markdown: &str, ctx: &DocContext, position: Position, counter: &mut usize) -> Result<Tagged, Untaggable> {
    tagify_with(markdown, ctx, position, counter, true)
}

fn tagify_with(source: &str, ctx: &DocContext, position: Position, counter: &mut usize, lenient: bool) -> Result<Tagged, Untaggable> {
    if position == Position::Plain {
        let mut text = String::new();
        for c in source.chars() {
            xml_escape(c, &mut text);
        }
        return Ok(Tagged { source: source.to_string(), text, tags: Vec::new(), position, dialect: ctx.dialect });
    }
    let Masking { text: masked, spans, open_expression } = mask(source, ctx.dialect);
    if open_expression {
        return Err(Untaggable("the source leaves an MDX expression open".into()));
    }
    // Math is masked first: `\\(\sum_{j} x_j\\)` holds no emphasis marks.
    if !lenient && !commonmark_emphasis(&masked).stray.is_empty() {
        return Err(Untaggable("the source leaves an emphasis mark open".into()));
    }
    let guarded = format!("{GUARD_PREFIX}{masked}");
    let events = events(&guarded, ctx);
    let html_pairs = pair_inline_html(&events);
    let html_closes: HashMap<usize, usize> = html_pairs.iter().map(|(o, c)| (*c, *o)).collect();

    struct Open {
        tag: usize,
        content_start: usize,
        max_end: usize,
        end: usize,
    }
    let mut text = String::new();
    let mut tags: Vec<Tag> = Vec::new();
    let mut stack: Vec<Open> = Vec::new();
    let mut html_tag_of: HashMap<usize, usize> = HashMap::new();
    let mut skip = 0usize;
    let next = |counter: &mut usize| {
        *counter += 1;
        *counter
    };
    let void = |text: &mut String, tags: &mut Vec<Tag>, n: usize, kind: TagKind, open: String, shape: &str, visible: String| {
        text.push_str(&format!("<{}{n}/>", kind.letter()));
        tags.push(Tag { n, kind, open, close: String::new(), label: None, code: None, role: None, shape: shape.into(), visible });
    };

    for (i, (event, range)) in events.iter().enumerate() {
        if skip > 0 {
            match event {
                Event::Start(_) => skip += 1,
                Event::End(_) => skip -= 1,
                _ => {}
            }
            continue;
        }
        if !matches!(event, Event::End(_))
            && let Some(top) = stack.last_mut()
        {
            top.max_end = top.max_end.max(range.end);
        }
        let raw = || unmask(&guarded[range.clone()], &spans);
        match event {
            Event::Start(MdTag::Emphasis | MdTag::Strong | MdTag::Strikethrough) => {
                let kind = match event {
                    Event::Start(MdTag::Emphasis) => TagKind::Italic,
                    Event::Start(MdTag::Strong) => TagKind::Bold,
                    _ => TagKind::Strike,
                };
                let mark = guarded[range.start..].chars().next().unwrap_or('*');
                let len = match kind {
                    TagKind::Bold => 2,
                    TagKind::Strike => guarded[range.start..].chars().take_while(|c| *c == '~').count(),
                    _ => 1,
                };
                let delim: String = std::iter::repeat_n(mark, len).collect();
                let n = next(counter);
                text.push_str(&format!("<{}{n}>", kind.letter()));
                tags.push(Tag { n, kind, open: delim.clone(), close: delim, label: None, code: None, role: None, shape: String::new(), visible: String::new() });
                stack.push(Open { tag: tags.len() - 1, content_start: range.start + len, max_end: range.start + len, end: range.end });
            }
            Event::Start(MdTag::Link { link_type: LinkType::Autolink | LinkType::Email, dest_url, .. }) => {
                let v = raw();
                let visible = dest_url.to_string();
                let n = next(counter);
                void(&mut text, &mut tags, n, TagKind::Opaque, v, "u", visible);
                skip = 1;
            }
            Event::Start(MdTag::Link { link_type, .. }) => {
                let n = next(counter);
                text.push_str(&format!("<a{n}>"));
                let shortcut = matches!(
                    link_type,
                    LinkType::Shortcut | LinkType::ShortcutUnknown | LinkType::Collapsed | LinkType::CollapsedUnknown
                );
                tags.push(Tag {
                    n,
                    kind: TagKind::Link,
                    open: "[".into(),
                    close: String::new(),
                    label: shortcut.then(String::new),
                    code: None,
                    role: None,
                    shape: String::new(),
                    visible: String::new(),
                });
                stack.push(Open { tag: tags.len() - 1, content_start: range.start + 1, max_end: range.start + 1, end: range.end });
            }
            Event::Start(MdTag::Image { .. }) => {
                let n = next(counter);
                void(&mut text, &mut tags, n, TagKind::Opaque, raw(), "g", String::new());
                skip = 1;
            }
            Event::End(TagEnd::Emphasis | TagEnd::Strong | TagEnd::Strikethrough | TagEnd::Link) => {
                let Some(open) = stack.pop() else { continue };
                let tag = &mut tags[open.tag];
                if tag.kind == TagKind::Link {
                    tag.close = unmask(&guarded[open.max_end..open.end], &spans);
                    if tag.label.is_some() {
                        tag.label = Some(unmask(&guarded[open.content_start..open.max_end], &spans));
                    }
                }
                text.push_str(&format!("</{}{}>", tag.kind.letter(), tag.n));
            }
            Event::Code(content) => {
                let span = raw();
                for c in span.chars() {
                    xml_escape(c, &mut text);
                }
                let n = next(counter);
                tags.push(Tag {
                    n,
                    kind: TagKind::Code,
                    open: span,
                    close: String::new(),
                    label: None,
                    code: Some(content.to_string()),
                    role: None,
                    shape: format!("c:{content}|"),
                    visible: String::new(),
                });
            }
            Event::InlineHtml(_) | Event::Html(_) if html_pairs.contains_key(&i) => {
                let n = next(counter);
                text.push_str(&format!("<h{n}>"));
                tags.push(Tag { n, kind: TagKind::Html, open: raw(), close: String::new(), label: None, code: None, role: None, shape: "h".into(), visible: String::new() });
                html_tag_of.insert(i, tags.len() - 1);
            }
            Event::InlineHtml(_) | Event::Html(_) if html_closes.contains_key(&i) => {
                let tag = &mut tags[html_tag_of[&html_closes[&i]]];
                tag.close = raw();
                text.push_str(&format!("</h{}>", tag.n));
            }
            Event::InlineHtml(_) | Event::Html(_) => {
                let n = next(counter);
                void(&mut text, &mut tags, n, TagKind::Opaque, raw(), "h", String::new());
            }
            Event::FootnoteReference(_) => {
                let n = next(counter);
                void(&mut text, &mut tags, n, TagKind::Opaque, raw(), "f", String::new());
            }
            Event::HardBreak => {
                let n = next(counter);
                void(&mut text, &mut tags, n, TagKind::Opaque, raw(), "n", String::new());
            }
            Event::SoftBreak => text.push(' '),
            Event::Text(t) => {
                let raw_text = &guarded[range.clone()];
                // The escape can sit just before the event's range.
                if !lenient
                    && raw_text
                        .char_indices()
                        .any(|(at, c)| c == '`' && !guarded[..range.start + at].ends_with('\\'))
                {
                    return Err(Untaggable("the source leaves a code span open".into()));
                }
                let t = t.strip_prefix(GUARD).map(|r| r.strip_prefix(' ').unwrap_or(r)).unwrap_or(t);
                for piece in split_masked(t, &spans) {
                    let Err(at) = piece else {
                        xml_escape(piece.unwrap_or_default(), &mut text);
                        continue;
                    };
                    let (kind, raw) = (spans[at].0, spans[at].1.clone());
                    let n = next(counter);
                    match kind {
                        Masked::Footnote => void(&mut text, &mut tags, n, TagKind::Opaque, raw, "f", String::new()),
                        Masked::Expression => void(&mut text, &mut tags, n, TagKind::Opaque, raw, "e", String::new()),
                        Masked::Math => {
                            let visible = raw.replace("\\\\", "\\");
                            void(&mut text, &mut tags, n, TagKind::Math, raw, "", visible);
                        }
                        Masked::Role => {
                            let (name, _, content) = split_role(&raw);
                            let name = name.to_string();
                            match role_label(&name, &content) {
                                Some((label, target, implicit)) => {
                                    text.push_str(&format!("<a{n}>"));
                                    for c in label.chars() {
                                        xml_escape(c, &mut text);
                                    }
                                    text.push_str(&format!("</a{n}>"));
                                    tags.push(Tag {
                                        n,
                                        kind: TagKind::Role,
                                        open: raw,
                                        close: String::new(),
                                        label: Some(label),
                                        code: None,
                                        role: Some(RoleRef { name, target, implicit }),
                                        shape: String::new(),
                                        visible: String::new(),
                                    });
                                }
                                // Shown as written, and held to it like code.
                                None => {
                                    for c in raw.chars() {
                                        xml_escape(c, &mut text);
                                    }
                                    tags.push(Tag {
                                        n,
                                        kind: TagKind::Code,
                                        open: raw,
                                        close: String::new(),
                                        label: None,
                                        shape: format!("r:{content}|"),
                                        code: Some(content),
                                        role: Some(RoleRef { name, target: String::new(), implicit: false }),
                                        visible: String::new(),
                                    });
                                }
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }
    Ok(Tagged { source: source.to_string(), text, tags, position, dialect: ctx.dialect })
}

// ---- reading ---------------------------------------------------------------

enum Piece {
    Text(String),
    /// A code span; in MyST, with the `{name}` written right before it.
    Code { written: String, content: String, role: Option<String> },
}

/// CommonMark's code span content: one space stripped from each end when both
/// have one and the rest is not all spaces.
fn code_content(inner: &str) -> String {
    let t = inner.replace('\n', " ");
    if t.len() >= 2 && t.starts_with(' ') && t.ends_with(' ') && !t.trim().is_empty() {
        t[1..t.len() - 1].to_string()
    } else {
        t
    }
}

/// Backtick spans first, so that `Vec<i32>` inside code is never read as a tag.
fn split_code_spans(text: &str) -> Vec<Piece> {
    let chars: Vec<char> = text.chars().collect();
    let mut pieces = Vec::new();
    let mut buf = String::new();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] != '`' {
            buf.push(chars[i]);
            i += 1;
            continue;
        }
        let run = chars[i..].iter().take_while(|c| **c == '`').count();
        let mut j = i + run;
        let mut close = None;
        while j < chars.len() {
            if chars[j] == '`' {
                let r = chars[j..].iter().take_while(|c| **c == '`').count();
                if r == run {
                    close = Some(j);
                    break;
                }
                j += r;
            } else {
                j += 1;
            }
        }
        match close {
            Some(end) => {
                if !buf.is_empty() {
                    pieces.push(Piece::Text(std::mem::take(&mut buf)));
                }
                let written = decode(&chars[i..end + run].iter().collect::<String>());
                let content = code_content(&decode(&chars[i + run..end].iter().collect::<String>()));
                pieces.push(Piece::Code { written, content, role: None });
                i = end + run;
            }
            None => {
                buf.extend(&chars[i..i + run]);
                i += run;
            }
        }
    }
    if !buf.is_empty() {
        pieces.push(Piece::Text(buf));
    }
    pieces
}

enum Token {
    Text(String),
    Open(char, Option<usize>),
    Close(char, Option<usize>),
    Void(char, usize),
    Code { written: String, content: String, role: Option<String> },
}

fn tokenize(pieces: Vec<Piece>) -> Vec<Token> {
    let mut tokens = Vec::new();
    for piece in pieces {
        let text = match piece {
            Piece::Code { written, content, role } => {
                tokens.push(Token::Code { written, content, role });
                continue;
            }
            Piece::Text(text) => text,
        };
        let b = text.as_bytes();
        let mut buf = String::new();
        let mut i = 0;
        while i < text.len() {
            if b[i] == b'<' {
                let mut j = i + 1;
                let closing = b.get(j) == Some(&b'/');
                if closing {
                    j += 1;
                }
                if let Some(&letter) = b.get(j).filter(|c| c.is_ascii_lowercase()) {
                    j += 1;
                    let digits = j;
                    while b.get(j).is_some_and(u8::is_ascii_digit) {
                        j += 1;
                    }
                    let number = (j > digits).then(|| text[digits..j].parse::<usize>().ok()).flatten();
                    let selfclose = b.get(j) == Some(&b'/');
                    let end = if selfclose { j + 1 } else { j };
                    let letter = letter as char;
                    let token = match (b.get(end), closing, selfclose, number) {
                        (Some(b'>'), false, false, Some(n)) => Some(Token::Open(letter, Some(n))),
                        (Some(b'>'), true, false, n) => Some(Token::Close(letter, n)),
                        (Some(b'>'), false, true, Some(n)) => Some(Token::Void(letter, n)),
                        _ => None,
                    };
                    if let Some(token) = token {
                        if !buf.is_empty() {
                            tokens.push(Token::Text(decode(&std::mem::take(&mut buf))));
                        }
                        tokens.push(token);
                        i = end + 1;
                        continue;
                    }
                }
            }
            let c = text[i..].chars().next().expect("char boundary");
            buf.push(c);
            i += c.len_utf8();
        }
        if !buf.is_empty() {
            tokens.push(Token::Text(decode(&buf)));
        }
    }
    tokens
}

/// Source text outside its code spans.
/// A MyST reply's `{name}` written right before a code span, taken into the
/// span: the role it names is restored from the source either way.
fn absorb_role_names(pieces: Vec<Piece>) -> Vec<Piece> {
    let mut out: Vec<Piece> = Vec::new();
    for piece in pieces {
        if let Piece::Code { written, content, .. } = &piece
            && let Some(Piece::Text(before)) = out.last_mut()
            && let Some(at) = before.rfind('{')
            && role_head(&before[at..]) == Some(before.len() - at)
        {
            let head = before.split_off(at);
            let name = head[1..head.len() - 1].to_string();
            if before.is_empty() {
                out.pop();
            }
            out.push(Piece::Code { written: format!("{head}{written}"), content: content.clone(), role: Some(name) });
            continue;
        }
        out.push(piece);
    }
    out
}

/// Source text outside its code spans, for telling whether a code span of
/// the reply marks up words the source wrote as text. A MyST role counts as
/// the text it shows: `` {term}`Nix` `` shows `Nix`.
fn prose_of(tagged: &Tagged) -> String {
    let Masking { text, spans, .. } = mask(&tagged.source, tagged.dialect);
    let shown: String = split_masked(&text, &spans)
        .into_iter()
        .map(|piece| match piece {
            Ok(c) => c.to_string(),
            Err(at) if spans[at].0 == Masked::Role => {
                let (name, _, content) = split_role(&spans[at].1);
                format!(" {} ", role_label(name, &content).map_or(content, |(label, _, _)| label))
            }
            Err(at) => spans[at].1.clone(),
        })
        .collect();
    split_code_spans(&shown)
        .into_iter()
        .map(|piece| match piece {
            Piece::Text(t) => t,
            Piece::Code { .. } => " ".into(),
        })
        .collect()
}

pub fn read(reply: &str, tagged: &Tagged) -> Result<Tree, Vec<String>> {
    let mut notes = Vec::new();
    if tagged.position == Position::Plain {
        let mut text = String::new();
        for token in tokenize(vec![Piece::Text(reply.to_string())]) {
            match token {
                Token::Text(t) => text.push_str(&t),
                _ => notes.push("dropped a tag from plain text".to_string()),
            }
        }
        return Ok(Tree { nodes: vec![Node::Text(text)], notes });
    }
    let by_n: HashMap<usize, &Tag> = tagged.tags.iter().map(|t| (t.n, t)).collect();
    let mut problems: Vec<String> = Vec::new();
    let name = |n: usize| format!("<{}{n}>", by_n[&n].kind.letter());

    // Lenient reading: the number is the tag's identity.
    let mut nodes: Vec<Node> = Vec::new();
    let mut open: Vec<usize> = Vec::new();
    let mut codes: Vec<(usize, String, Option<String>)> = Vec::new(); // node index, content, role name
    let mut pieces = split_code_spans(reply);
    if tagged.dialect == Dialect::Myst {
        pieces = absorb_role_names(pieces);
    }
    for token in tokenize(pieces) {
        match token {
            Token::Text(t) => nodes.push(Node::Text(t)),
            Token::Code { written, content, role } => {
                codes.push((nodes.len(), content, role));
                nodes.push(Node::Code { tag: None, written });
            }
            Token::Close(letter, None) => {
                match open.iter().rposition(|n| by_n[n].kind.letter() == letter) {
                    Some(pos) => {
                        let n = open[pos];
                        open.truncate(pos);
                        notes.push(format!("</{letter}> read as </{letter}{n}>"));
                        nodes.push(Node::Close(n));
                    }
                    None => notes.push(format!("dropped a stray </{letter}>")),
                }
            }
            Token::Open(letter, Some(n)) | Token::Close(letter, Some(n)) | Token::Void(letter, n)
                if !by_n.contains_key(&n) || by_n[&n].kind == TagKind::Code =>
            {
                notes.push(format!("dropped unknown tag {letter}{n}"));
            }
            Token::Open(letter, Some(n)) => {
                if by_n[&n].kind.letter() != letter {
                    notes.push(format!("<{letter}{n}> read as {}", name(n)));
                }
                if by_n[&n].kind.is_void() {
                    nodes.push(Node::Void(n));
                } else {
                    open.push(n);
                    nodes.push(Node::Open(n));
                }
            }
            Token::Close(letter, Some(n)) => {
                if by_n[&n].kind.letter() != letter {
                    notes.push(format!("</{letter}{n}> read as a close of {}", name(n)));
                }
                if let Some(pos) = open.iter().rposition(|m| *m == n) {
                    open.truncate(pos);
                }
                if !by_n[&n].kind.is_void() {
                    nodes.push(Node::Close(n));
                }
            }
            Token::Void(_, n) => {
                if by_n[&n].kind.is_void() {
                    nodes.push(Node::Void(n));
                } else {
                    problems.push(format!("{} must wrap text: write {}…</{}{n}>", name(n), name(n), by_n[&n].kind.letter()));
                }
            }
            Token::Open(_, None) => {}
        }
    }

    // Code: strictly by content. An order-based match would restore another
    // span's bytes over a span the model changed. A MyST role shown as written
    // is held to the same rule; a span the reply names a role for goes to a
    // source role first, a bare span to bare code, so that code and a role
    // with the same content keep their places.
    let source_codes: Vec<&Tag> = tagged.tags.iter().filter(|t| t.kind == TagKind::Code).collect();
    let mut unused: Vec<usize> = (0..source_codes.len()).collect();
    let mut assigned: HashMap<usize, usize> = HashMap::new(); // node index -> source code index
    let role_of = |c: usize| source_codes[c].role.as_ref().map(|r| r.name.as_str());
    for same_role in [true, false] {
        for (node, content, role) in &codes {
            if assigned.contains_key(node) {
                continue;
            }
            if let Some(pos) = unused.iter().position(|&c| {
                source_codes[c].code.as_deref() == Some(content.as_str())
                    && (!same_role || role_of(c) == role.as_deref())
            }) {
                assigned.insert(*node, unused.remove(pos));
            }
        }
    }
    for (node, content, role) in &codes {
        if assigned.contains_key(node) || role.is_some() {
            continue;
        }
        if let Some(pos) = unused
            .iter()
            .position(|&c| role_of(c).is_none() && stands_for(content, source_codes[c].code.as_deref().unwrap_or("")))
        {
            assigned.insert(*node, unused.remove(pos));
        }
    }
    let prose = prose_of(tagged);
    for (node, content, _) in &codes {
        let tag = match assigned.get(node) {
            Some(&c) => Some(source_codes[c].n),
            None => {
                let repeat = source_codes.iter().find(|t| {
                    t.code.as_deref() == Some(content.as_str()) && !unused.iter().any(|&u| source_codes[u].n == t.n)
                });
                match repeat {
                    Some(t) => Some(t.n),
                    None if written_as_text(&prose, content) => None,
                    None => {
                        problems.push(format!(
                            "code `{content}` is not in the source — copy every code span exactly as written"
                        ));
                        None
                    }
                }
            }
        };
        if let Node::Code { tag: slot, .. } = &mut nodes[*node] {
            *slot = tag;
        }
    }
    for c in unused {
        problems.push(format!("code {} is missing — keep every code span", source_codes[c].open));
    }

    // Every tag in place, nested.
    let mut opens: HashMap<usize, usize> = HashMap::new();
    let mut closes: HashMap<usize, usize> = HashMap::new();
    let mut voids: HashMap<usize, usize> = HashMap::new();
    let mut stack: Vec<usize> = Vec::new();
    for node in &nodes {
        match node {
            Node::Open(n) => {
                *opens.entry(*n).or_default() += 1;
                stack.push(*n);
            }
            Node::Close(n) => {
                *closes.entry(*n).or_default() += 1;
                match stack.pop() {
                    Some(top) if top == *n => {}
                    Some(top) => problems.push(format!("{} closes inside {}", name(*n), name(top))),
                    None => problems.push(format!("{} closes without opening", name(*n))),
                }
            }
            Node::Void(n) => *voids.entry(*n).or_default() += 1,
            _ => {}
        }
    }
    for n in stack {
        problems.push(format!("{} is never closed", name(n)));
    }
    for tag in tagged.tags.iter().filter(|t| t.kind != TagKind::Code) {
        let (o, c, v) = (
            opens.get(&tag.n).copied().unwrap_or(0),
            closes.get(&tag.n).copied().unwrap_or(0),
            voids.get(&tag.n).copied().unwrap_or(0),
        );
        let ok = match tag.kind {
            // Korean word order may split a bold phrase in two.
            k if k.is_emphasis() => o >= 1 && o == c,
            TagKind::Link | TagKind::Html | TagKind::Role => o == 1 && c == 1,
            _ => v == 1,
        };
        if !ok {
            let what = if o + c + v == 0 { "is missing" } else { "appears more than once" };
            problems.push(format!("{} {what} — keep every tag exactly once", name(tag.n)));
        }
    }
    // A role's label is text inside the role's backticks: nothing nests there.
    let mut in_role: Option<(usize, bool)> = None; // the role, whether it has text
    for node in &nodes {
        match node {
            Node::Open(n) if by_n[n].kind == TagKind::Role && in_role.is_none() => in_role = Some((*n, false)),
            Node::Close(n) if in_role.is_some_and(|(open, _)| open == *n) => {
                if in_role.is_some_and(|(_, text)| !text) {
                    problems.push(format!("{} is empty — write its translated label inside it", name(*n)));
                }
                in_role = None;
            }
            Node::Text(t) => {
                if let Some((_, text)) = &mut in_role {
                    *text |= !t.trim().is_empty();
                }
            }
            Node::Open(m) | Node::Close(m) | Node::Void(m) if let Some((open, _)) = in_role => {
                problems.push(format!("{} holds plain text only — move {} outside it", name(open), name(*m)))
            }
            Node::Code { written, .. } if let Some((open, _)) = in_role => {
                problems.push(format!("{} holds plain text only — write {written} outside it", name(open)))
            }
            _ => {}
        }
    }
    problems.dedup();
    if problems.is_empty() { Ok(Tree { nodes, notes }) } else { Err(problems) }
}

// ---- rendering -------------------------------------------------------------

#[derive(Clone, Debug, PartialEq)]
enum Tok {
    Text(String),
    Open(usize),
    Close(usize),
    Void(usize),
    /// Markdown to write, the shape it reads back as (`c:` code, `r:` a role
    /// shown as written, `(a)` a label role) and the text it shows.
    Code(String, String, String),
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Strategy {
    Star,
    Shrink,
    SwapLink,
    QuotesOut,
    ParticleIn,
    Html,
}

impl Strategy {
    fn name(self) -> &'static str {
        match self {
            Strategy::Star => "star",
            Strategy::Shrink => "shrink",
            Strategy::SwapLink => "swap-link",
            Strategy::QuotesOut => "quotes-out",
            Strategy::ParticleIn => "particle-in",
            Strategy::Html => "html",
        }
    }

    fn moves_text(self) -> bool {
        matches!(self, Strategy::Shrink | Strategy::SwapLink | Strategy::QuotesOut | Strategy::ParticleIn)
    }
}

struct Renderer<'a> {
    tags: HashMap<usize, &'a Tag>,
    position: Position,
    ctx: &'a DocContext,
    /// The source escaped a bracket, so a bracket in the translation might
    /// otherwise resolve as a reference link the source kept from forming.
    escape_brackets: bool,
}

impl Renderer<'_> {
    fn element(toks: &[Tok], n: usize) -> Option<(usize, usize)> {
        let o = toks.iter().position(|t| *t == Tok::Open(n))?;
        let c = toks.iter().position(|t| *t == Tok::Close(n))?;
        (c > o + 1).then_some((o, c))
    }

    fn is_emphasis_boundary(&self, tok: &Tok) -> bool {
        matches!(tok, Tok::Open(n) | Tok::Close(n) if self.tags[n].kind.is_emphasis())
    }

    /// The token rewrites a strategy makes. Each keeps the text and moves
    /// only where a pair opens or closes.
    fn transform(&self, toks: &[Tok], n: usize, strategy: Strategy) -> Option<Vec<Tok>> {
        let (o, c) = Self::element(toks, n)?;
        match strategy {
            // `<b1>외적(outer product)</b1>에서` -> `<b1>외적</b1>(outer product)에서`
            Strategy::Shrink => {
                let Tok::Text(last) = &toks[c - 1] else { return None };
                if !last.ends_with(')') {
                    return None;
                }
                let mut depth = 0i32;
                for i in (o + 1..c).rev() {
                    match &toks[i] {
                        Tok::Text(t) => {
                            for (at, ch) in t.char_indices().collect::<Vec<_>>().into_iter().rev() {
                                match ch {
                                    ')' => depth += 1,
                                    '(' => {
                                        depth -= 1;
                                        if depth == 0 {
                                            let (mut head, mut gloss) = (t[..at].to_string(), t[at..].to_string());
                                            if head.ends_with(' ') {
                                                head.pop();
                                                gloss.insert(0, ' ');
                                            }
                                            if head.is_empty() && i == o + 1 {
                                                return None;
                                            }
                                            let mut v = toks[..i].to_vec();
                                            if !head.is_empty() {
                                                v.push(Tok::Text(head));
                                            }
                                            v.push(toks[c].clone());
                                            v.push(Tok::Text(gloss));
                                            v.extend_from_slice(&toks[i + 1..c]);
                                            v.extend_from_slice(&toks[c + 1..]);
                                            return Some(v);
                                        }
                                    }
                                    _ => {}
                                }
                            }
                        }
                        tok if self.is_emphasis_boundary(tok) => return None,
                        _ => {}
                    }
                }
                None
            }
            // `<b1><a2>Fetch</a2></b1>는` -> `<a2><b1>Fetch</b1></a2>는`
            Strategy::SwapLink => {
                let Tok::Open(a) = toks[o + 1] else { return None };
                if self.tags[&a].kind != TagKind::Link || toks[c - 1] != Tok::Close(a) {
                    return None;
                }
                let mut v = toks[..o].to_vec();
                v.push(toks[o + 1].clone());
                v.push(toks[o].clone());
                v.extend_from_slice(&toks[o + 2..c - 1]);
                v.push(toks[c].clone());
                v.push(toks[c - 1].clone());
                v.extend_from_slice(&toks[c + 1..]);
                Some(v)
            }
            // `<i1>"권한"</i1>은` -> `"<i1>권한</i1>"은`
            Strategy::QuotesOut => {
                let (Tok::Text(first), Tok::Text(last)) = (&toks[o + 1], &toks[c - 1]) else { return None };
                let q1 = first.chars().next()?;
                let q2 = last.chars().last()?;
                if !matches!((q1, q2), ('"', '"') | ('“', '”') | ('\'', '\'') | ('‘', '’') | ('「', '」')) {
                    return None;
                }
                let mut v = toks[..o].to_vec();
                v.push(Tok::Text(q1.to_string()));
                v.push(toks[o].clone());
                if o + 1 == c - 1 {
                    let inner: Vec<char> = first.chars().collect();
                    if inner.len() < 3 {
                        return None;
                    }
                    v.push(Tok::Text(inner[1..inner.len() - 1].iter().collect()));
                } else {
                    v.push(Tok::Text(first.chars().skip(1).collect()));
                    v.extend_from_slice(&toks[o + 2..c - 1]);
                    let l: Vec<char> = last.chars().collect();
                    v.push(Tok::Text(l[..l.len() - 1].iter().collect()));
                }
                v.push(toks[c].clone());
                v.push(Tok::Text(q2.to_string()));
                v.extend_from_slice(&toks[c + 1..]);
                Some(v)
            }
            // `<i1>… `impl Trait`</i1>의` -> `<i1>… `impl Trait`의</i1>`. Only a
            // particle: a predicate (`입니다`) is not the term's, and taking it
            // in would lean it too; HTML is left for that.
            Strategy::ParticleIn => {
                let Some(Tok::Text(next)) = toks.get(c + 1) else { return None };
                let run = PARTICLES.iter().find(|p| next.starts_with(**p))?.to_string();
                let rest = next[run.len()..].to_string();
                let mut v = toks[..c].to_vec();
                v.push(Tok::Text(run));
                v.push(toks[c].clone());
                if !rest.is_empty() {
                    v.push(Tok::Text(rest));
                }
                v.extend_from_slice(&toks[c + 2..]);
                Some(v)
            }
            _ => None,
        }
    }

    fn transformed(&self, toks: &[Tok], strategies: &HashMap<usize, Strategy>) -> Vec<Tok> {
        let mut keys: Vec<_> = strategies.iter().filter(|(_, s)| s.moves_text()).collect();
        keys.sort_by_key(|(n, _)| **n);
        let mut t = toks.to_vec();
        for (n, s) in keys {
            if let Some(v) = self.transform(&t, *n, *s) {
                t = v;
            }
        }
        t
    }

    fn escape(&self, text: &str, line_start: bool, out: &mut String) {
        let dialect = self.ctx.dialect;
        let chars: Vec<char> = text.chars().collect();
        // `1. ` at the start of a line opens a list.
        let digits = chars.iter().take_while(|c| c.is_ascii_digit()).count();
        let list_marker = line_start
            && digits > 0
            && matches!(chars.get(digits), Some('.' | ')'))
            && matches!(chars.get(digits + 1), Some(' ') | None);
        for (i, &c) in chars.iter().enumerate() {
            let prev = if i > 0 { Some(chars[i - 1]) } else { out.chars().last() };
            let next = chars.get(i + 1).copied();
            let at_start = line_start && i == 0;
            let escape = match c {
                '\\' | '`' | '*' => true,
                // MDX reads `{` as an expression and `<` as JSX wherever
                // they stand.
                '{' | '<' if dialect == Dialect::Mdx => true,
                '_' => !(prev.is_some_and(|p| p.is_ascii_alphanumeric()) && next.is_some_and(|n| n.is_ascii_alphanumeric())),
                // GFM strikes through with one tilde as well as two.
                '~' => dialect != Dialect::Myst && !(prev.is_none_or(char::is_whitespace) && next.is_none_or(char::is_whitespace)),
                '<' => next.is_some_and(|n| n.is_ascii_alphabetic() || matches!(n, '/' | '!' | '?')),
                '&' => {
                    next == Some('#')
                        || (chars[i + 1..].iter().take_while(|c| c.is_ascii_alphanumeric()).count() > 0
                            && chars[i + 1..].iter().find(|c| !c.is_ascii_alphanumeric()) == Some(&';'))
                }
                '|' => self.position == Position::TableCell,
                '[' | ']' => self.escape_brackets,
                '#' | '-' | '+' => at_start && matches!(next, Some(' ') | None),
                '>' => at_start,
                '.' | ')' => list_marker && chars[..i].iter().all(|c| c.is_ascii_digit()),
                _ => false,
            };
            if escape {
                out.push('\\');
            }
            out.push(c);
        }
    }

    fn render(&self, toks: &[Tok], strategies: &HashMap<usize, Strategy>, line_start: bool) -> String {
        let mut out = String::new();
        let mut role_open: Option<usize> = None;
        for (i, tok) in toks.iter().enumerate() {
            // A role's label is written whole when the role closes.
            if let Some(start) = role_open {
                if let Tok::Close(n) = tok
                    && toks[start] == Tok::Open(*n)
                {
                    let tag = self.tags[n];
                    let label: String = toks[start + 1..i]
                        .iter()
                        .filter_map(|t| match t {
                            Tok::Text(t) => Some(t.as_str()),
                            _ => None,
                        })
                        .collect();
                    if let Some(role) = &tag.role {
                        out.push_str(&write_role(&tag.open, role, tag.label.as_deref().unwrap_or(""), &label));
                    }
                    role_open = None;
                }
                continue;
            }
            match tok {
                Tok::Text(t) => {
                    let at_start = line_start && out.is_empty();
                    self.escape(t, at_start, &mut out);
                }
                Tok::Open(n) if self.tags[n].kind == TagKind::Role => role_open = Some(i),
                Tok::Void(n) => out.push_str(&self.tags[n].open),
                Tok::Code(markdown, _, _) => out.push_str(markdown),
                Tok::Open(n) | Tok::Close(n) => {
                    let tag = self.tags[n];
                    let opening = matches!(tok, Tok::Open(_));
                    match tag.kind {
                        TagKind::Link if opening => out.push('['),
                        TagKind::Link => match &tag.label {
                            Some(label) => {
                                let start = toks[..i].iter().rposition(|t| *t == Tok::Open(*n)).unwrap_or(i);
                                let inner = self.render(&toks[start + 1..i], strategies, false);
                                // `[label](GCI)` would read as an inline link.
                                let next_opens = matches!(toks.get(i + 1), Some(Tok::Text(t)) if t.starts_with(['(', '[']));
                                if normalize_label(&inner) == normalize_label(label) && !next_opens {
                                    out.push_str(&tag.close);
                                } else {
                                    out.push_str(&format!("][{label}]"));
                                }
                            }
                            None => out.push_str(&tag.close),
                        },
                        TagKind::Html => out.push_str(if opening { &tag.open } else { &tag.close }),
                        _ => match strategies.get(n) {
                            Some(Strategy::Html) => {
                                let name = match tag.kind {
                                    TagKind::Italic => "em",
                                    TagKind::Bold => "strong",
                                    _ => "del",
                                };
                                out.push_str(&if opening { format!("<{name}>") } else { format!("</{name}>") });
                            }
                            Some(Strategy::Star) if tag.kind != TagKind::Strike => {
                                out.push_str(&"*".repeat(tag.open.chars().count()))
                            }
                            _ => out.push_str(if opening { &tag.open } else { &tag.close }),
                        },
                    }
                }
            }
        }
        out
    }

    /// How `markdown` reads back: a shape of constructs and the text.
    fn read_back(&self, markdown: &str) -> (String, String) {
        let Masking { text: masked, spans, .. } = mask(markdown, self.ctx.dialect);
        let guarded = format!("{GUARD_PREFIX}{masked}");
        let mut shape = String::new();
        let mut text = String::new();
        let mut skip = 0;
        for (event, _) in events(&guarded, self.ctx) {
            if skip > 0 {
                match event {
                    Event::Start(_) => skip += 1,
                    Event::End(_) => skip -= 1,
                    _ => {}
                }
                continue;
            }
            match event {
                Event::Start(MdTag::Emphasis) => shape.push_str("(i"),
                Event::Start(MdTag::Strong) => shape.push_str("(b"),
                Event::Start(MdTag::Strikethrough) => shape.push_str("(s"),
                Event::Start(MdTag::Link { link_type: LinkType::Autolink | LinkType::Email, dest_url, .. }) => {
                    shape.push('u');
                    text.push_str(&dest_url);
                    skip = 1;
                }
                Event::Start(MdTag::Link { .. }) => shape.push_str("(a"),
                Event::Start(MdTag::Image { .. }) => {
                    shape.push('g');
                    skip = 1;
                }
                Event::End(TagEnd::Emphasis | TagEnd::Strong | TagEnd::Strikethrough | TagEnd::Link) => shape.push(')'),
                Event::Code(c) => shape.push_str(&format!("c:{c}|")),
                Event::Html(h) | Event::InlineHtml(h) => match h.as_ref() {
                    "<em>" => shape.push_str("(i"),
                    "<strong>" => shape.push_str("(b"),
                    "<del>" => shape.push_str("(s"),
                    "</em>" | "</strong>" | "</del>" => shape.push(')'),
                    _ => shape.push('h'),
                },
                Event::FootnoteReference(_) => shape.push('f'),
                Event::HardBreak => shape.push('n'),
                Event::SoftBreak => text.push(' '),
                Event::Text(t) => {
                    for piece in split_masked(&t, &spans) {
                        match piece {
                            Err(at) => match spans[at].0 {
                                Masked::Footnote => shape.push('f'),
                                Masked::Expression => shape.push('e'),
                                Masked::Math => text.push_str(&spans[at].1.replace("\\\\", "\\")),
                                Masked::Role => {
                                    let (name, _, content) = split_role(&spans[at].1);
                                    match role_label(name, &content) {
                                        Some((label, _, _)) => {
                                            shape.push_str("(a");
                                            text.push_str(&label);
                                            shape.push(')');
                                        }
                                        None => shape.push_str(&format!("r:{content}|")),
                                    }
                                }
                            },
                            Ok(c) if c != GUARD => text.push(c),
                            Ok(_) => {}
                        }
                    }
                }
                _ => {}
            }
        }
        (shape, text)
    }

    fn intended(&self, toks: &[Tok]) -> (String, String) {
        let mut shape = String::new();
        let mut text = String::new();
        for tok in toks {
            match tok {
                Tok::Text(t) => text.push_str(t),
                Tok::Open(n) => match (self.tags[n].kind, emphasis_html(&self.tags[n].open)) {
                    (TagKind::Html, Some(kind)) => shape.push_str(&format!("({kind}")),
                    (TagKind::Html, None) => shape.push('h'),
                    (kind, _) => shape.push_str(&format!("({}", kind.letter())),
                },
                Tok::Close(n) => match (self.tags[n].kind, emphasis_html(&self.tags[n].open)) {
                    (TagKind::Html, None) => shape.push('h'),
                    _ => shape.push(')'),
                },
                Tok::Void(n) => {
                    shape.push_str(&self.tags[n].shape);
                    text.push_str(&self.tags[n].visible);
                }
                Tok::Code(_, code, shown) => {
                    shape.push_str(code);
                    text.push_str(shown);
                }
            }
        }
        (shape, text)
    }

    fn score(&self, toks: &[Tok], strategies: &HashMap<usize, Strategy>) -> usize {
        let moved = self.transformed(toks, strategies);
        let markdown = self.render(&moved, strategies, true);
        let (got_shape, got_text) = self.read_back(&markdown);
        let (want_shape, want_text) = self.intended(&moved);
        let norm = |s: &str| s.split_whitespace().collect::<Vec<_>>().join(" ");
        levenshtein(&got_shape, &want_shape) + usize::from(norm(&got_text) != norm(&want_text)) * 3
    }
}

/// Korean particles, longest first, that may follow a marked-up term.
const PARTICLES: &[&str] = &[
    "으로부터", "에서부터", "으로서", "으로써", "에서는", "에서도", "에게서", "이라는", "이라고", "이라면",
    "으로는", "으로도", "까지", "부터", "에서", "에게", "한테", "으로", "처럼", "보다", "마다", "이나", "이란",
    "라는", "라고", "라면", "와는", "과는", "로는", "로도", "에는", "에도", "은", "는", "이", "가", "을", "를",
    "의", "에", "로", "와", "과", "도", "만", "나", "란", "야",
];

/// The emphasis an HTML element reads back as, the way `read_back` sees it.
fn emphasis_html(open: &str) -> Option<char> {
    match open {
        "<em>" => Some('i'),
        "<strong>" => Some('b'),
        "<del>" => Some('s'),
        _ => None,
    }
}

fn levenshtein(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    for i in 1..=a.len() {
        let mut cur = vec![i; b.len() + 1];
        for j in 1..=b.len() {
            cur[j] = (prev[j] + 1).min(cur[j - 1] + 1).min(prev[j - 1] + usize::from(a[i - 1] != b[j - 1]));
        }
        prev = cur;
    }
    prev[b.len()]
}

/// How `markdown` reads back in `tagged`'s dialect and position: the shape
/// of its constructs (`(b…)`, `(a…)`, `c:code|`, `h`, …) and its text.
pub(super) fn structure(markdown: &str, tagged: &Tagged, ctx: &DocContext) -> (String, String) {
    let renderer = Renderer {
        tags: tagged.tags.iter().map(|t| (t.n, t)).collect(),
        position: tagged.position,
        ctx,
        escape_brackets: false,
    };
    renderer.read_back(markdown)
}

/// Write the tree as Markdown, choosing for each emphasis a form CommonMark
/// closes where the translation put it.
///
/// The source's own marks come first; when the Markdown does not read back as
/// the tags meant, each emphasis tries in turn `_` → `*`, moving a trailing
/// parenthesised gloss out, moving itself inside a link it wraps, moving
/// wrapping quotation marks out, taking the following particle in, and
/// finally HTML — never an invisible character, which would linger in search,
/// copy and diffs.
pub fn render(tree: &Tree, tagged: &Tagged, ctx: &DocContext) -> Rendered {
    if tagged.position == Position::Plain {
        let markdown = tree
            .nodes
            .iter()
            .filter_map(|node| match node {
                Node::Text(t) => Some(t.as_str()),
                _ => None,
            })
            .collect();
        return Rendered { markdown, strategies: Vec::new(), verified: true };
    }
    let renderer = Renderer {
        tags: tagged.tags.iter().map(|t| (t.n, t)).collect(),
        position: tagged.position,
        ctx,
        escape_brackets: tagged.source.contains("\\[") || tagged.source.contains("\\]"),
    };
    let toks: Vec<Tok> = tree
        .nodes
        .iter()
        .map(|node| match node {
            Node::Text(t) => Tok::Text(t.clone()),
            Node::Open(n) => Tok::Open(*n),
            Node::Close(n) => Tok::Close(*n),
            Node::Void(n) => Tok::Void(*n),
            Node::Code { tag: Some(n), .. } => {
                let tag = renderer.tags[n];
                Tok::Code(tag.open.clone(), tag.shape.clone(), String::new())
            }
            Node::Code { tag: None, written } if role_head(written).is_some() => {
                // A role the reply wrote around words of the source: it reads
                // back the way `read_back` reads any role.
                let (name, _, content) = split_role(written);
                match role_label(name, &content) {
                    Some((label, _, _)) => Tok::Code(written.clone(), "(a)".into(), label),
                    None => Tok::Code(written.clone(), format!("r:{content}|"), String::new()),
                }
            }
            Node::Code { tag: None, written } => {
                let run = written.chars().take_while(|c| *c == '`').count();
                let inner: String = written.chars().skip(run).take(written.chars().count().saturating_sub(2 * run)).collect();
                Tok::Code(written.clone(), format!("c:{}|", code_content(&inner)), String::new())
            }
        })
        .collect();
    let mut emphasis: Vec<usize> =
        tagged.tags.iter().filter(|t| t.kind.is_emphasis()).map(|t| t.n).collect();
    emphasis.sort_unstable();

    let search = |mut strategies: HashMap<usize, Strategy>| {
        let mut best = renderer.score(&toks, &strategies);
        let mut improved = true;
        while best > 0 && improved {
            improved = false;
            for &n in &emphasis {
                for s in [Strategy::Star, Strategy::Shrink, Strategy::SwapLink, Strategy::QuotesOut, Strategy::ParticleIn, Strategy::Html] {
                    let mut trial = strategies.clone();
                    trial.insert(n, s);
                    let score = renderer.score(&toks, &trial);
                    if score < best {
                        best = score;
                        strategies = trial;
                        improved = true;
                        break;
                    }
                }
                if best == 0 {
                    break;
                }
            }
        }
        (strategies, best)
    };
    let (mut strategies, mut best) = search(HashMap::new());
    // Two `_` of different pairs can pair with each other instead, and then
    // no single `_` → `*` reads better on its own: start again from `*` for
    // every emphasis.
    if best > 0 {
        let stars = emphasis.iter().map(|n| (*n, Strategy::Star)).collect();
        let (again, score) = search(stars);
        if score < best {
            (strategies, best) = (again, score);
        }
    }
    if best > 0 {
        strategies = emphasis.iter().map(|n| (*n, Strategy::Html)).collect();
        best = renderer.score(&toks, &strategies);
    }
    let moved = renderer.transformed(&toks, &strategies);
    let markdown = renderer.render(&moved, &strategies, true);
    let mut chosen: Vec<(usize, &'static str)> = strategies.iter().map(|(n, s)| (*n, s.name())).collect();
    chosen.sort_unstable();
    Rendered { markdown, strategies: chosen, verified: best == 0 }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx(labels: &[&str]) -> DocContext {
        DocContext { reference_labels: labels.iter().map(|l| normalize_label(l)).collect(), dialect: Dialect::CommonMark }
    }

    fn tag(source: &str) -> Tagged {
        let mut counter = 0;
        tagify(source, &ctx(&["arena", "mcp", "clang-asan"]), Position::Inline, &mut counter).unwrap()
    }

    /// Tag `source`, read `reply` against it and render the result.
    fn translate(source: &str, reply: &str) -> String {
        let tagged = tag(source);
        let tree = read(reply, &tagged).unwrap_or_else(|p| panic!("{reply}: {p:?}"));
        let rendered = render(&tree, &tagged, &ctx(&["arena", "mcp", "clang-asan"]));
        assert!(rendered.verified, "{}", rendered.markdown);
        rendered.markdown
    }

    // --- tagify ----------------------------------------------------------

    #[test]
    fn reference_labels_come_from_the_whole_document() {
        let doc = DocContext::from_markdown("See [arena].\n\n[arena]: https://example.com\n[Other Label]: x\n");
        assert!(doc.reference_labels.contains("arena"));
        assert!(doc.reference_labels.contains("other label"));
    }

    #[test]
    fn inline_constructs_become_tags_and_code_stays_in_backticks() {
        assert_eq!(
            tag("The **outer product** of _u_ and `v` is ~~wrong~~.").text,
            "The <b1>outer product</b1> of <i2>u</i2> and `v` is <s4>wrong</s4>."
        );
        assert_eq!(
            tag("See [the book](https://x.org/a) and [arena], or [ASan][clang-asan].").text,
            "See <a1>the book</a1> and <a2>arena</a2>, or <a3>ASan</a3>."
        );
    }

    #[test]
    fn opaque_constructs_become_empty_tags() {
        assert_eq!(
            tag("Use <https://x.org>, see ![fig](a.png), note[^1], and \\\\(T\\\\) here.").text,
            "Use <x1/>, see <x2/>, note<x3/>, and <m4/> here."
        );
    }

    #[test]
    fn balanced_inline_html_is_a_pair_and_a_lone_tag_is_opaque() {
        assert_eq!(tag("Press <kbd>Ctrl</kbd> now.").text, "Press <h1>Ctrl</h1> now.");
        assert_eq!(tag("A <br> break.").text, "A <x1/> break.");
    }

    #[test]
    fn a_segment_that_looks_like_a_block_marker_is_inline() {
        assert_eq!(tag("1. Generate a reproducer").text, "1. Generate a reproducer");
        assert_eq!(tag("- same -").text, "- same -");
    }

    #[test]
    fn code_that_looks_like_a_footnote_or_math_stays_code() {
        assert_eq!(tag("Match `[^.,:!?;]+` or `\\\\(x\\\\)` here.").text, "Match `[^.,:!?;]+` or `\\\\(x\\\\)` here.");
    }

    #[test]
    fn math_marks_do_not_make_a_segment_untaggable() {
        assert_eq!(
            tag("It computes \\\\(y_i = \\sum_j A_{ij} x_j\\\\) here.").text,
            "It computes <m1/> here."
        );
    }

    #[test]
    fn an_undefined_reference_is_text() {
        assert_eq!(tag("See [unknown] here.").text, "See [unknown] here.");
    }

    #[test]
    fn text_escapes_angle_brackets_and_ampersands() {
        assert_eq!(tag("Use a < b & friends.").text, "Use a &lt; b &amp; friends.");
        // `<T>` is raw HTML to CommonMark, so it is copied as is.
        assert_eq!(tag("Use Vec<T> here.").text, "Use Vec<x1/> here.");
    }

    #[test]
    fn numbers_are_unique_across_a_request() {
        let mut counter = 0;
        let a = tagify("One *a*.", &ctx(&[]), Position::Inline, &mut counter).unwrap();
        let b = tagify("Two *b*.", &ctx(&[]), Position::Inline, &mut counter).unwrap();
        assert_eq!(a.text, "One <i1>a</i1>.");
        assert_eq!(b.text, "Two <i2>b</i2>.");
    }

    #[test]
    fn a_source_that_leaves_a_mark_open_is_untaggable() {
        let mut counter = 0;
        assert!(tagify("_Applied to `Span` fields.", &ctx(&[]), Position::Inline, &mut counter).is_err());
        assert!(tagify("It* is *true", &ctx(&[]), Position::Inline, &mut counter).is_err());
        assert!(tagify("an unclosed `span", &ctx(&[]), Position::Inline, &mut counter).is_err());
    }

    // --- read --------------------------------------------------------------

    #[test]
    fn a_bare_close_closes_the_innermost_open_tag_of_its_kind() {
        let tagged = tag("It *holds* a tensor.");
        let tree = read("텐서를 <i1>보유</i>합니다.", &tagged).unwrap();
        assert!(tree.nodes.contains(&Node::Close(1)));
    }

    #[test]
    fn a_wrong_kind_letter_is_corrected_and_unknown_tags_are_dropped() {
        let tagged = tag("An *inherent type*, defined.");
        let tree = read("<b1>고유 타입</b1>은 <b0></b0>정의됩니다.", &tagged).unwrap();
        assert_eq!(tree.nodes[0], Node::Open(1));
        assert!(!tree.nodes.iter().any(|n| matches!(n, Node::Open(0) | Node::Close(0))));
    }

    #[test]
    fn tag_like_text_inside_code_is_code() {
        let tagged = tag("*TraitRefs* like `WellFormed(Vec<i32>: Clone)`.");
        assert_eq!(tagged.text, "<i1>TraitRefs</i1> like `WellFormed(Vec&lt;i32&gt;: Clone)`.");
        let tree = read("`WellFormed(Vec<i32>: Clone)`와 같은 <i1>TraitRefs</i1>입니다.", &tagged).unwrap();
        assert!(matches!(tree.nodes[0], Node::Code { tag: Some(_), .. }));
    }

    #[test]
    fn code_matching_follows_five_cases() {
        let src = "Set `fold=0` for `Makefiles` and `x`; the Foo type.";
        // 1 exact, 2 shed plural, 3 repeat, 4 plain text as code.
        let tree = read("`Makefile`과 `x`에 `fold=0`을 두고 `x`를 씁니다; `Foo` 타입.", &tag(src)).unwrap();
        let codes: Vec<_> = tree.nodes.iter().filter_map(|n| match n { Node::Code { tag, .. } => Some(*tag), _ => None }).collect();
        assert!(codes[..4].iter().all(|t| t.is_some()), "{codes:?}");
        assert_eq!(codes[4], None);
        // 5: a changed span is a problem, not silently replaced.
        let problems = read("`fold`와 `Makefiles`와 `x`를 설정합니다.", &tag(src)).unwrap_err();
        assert!(problems.iter().any(|p| p.contains("`fold`")), "{problems:?}");
        // A span changed after its exact copy was already matched is still case 5.
        let problems = read("`fold=0`, `Makefiles`, `x`, 그리고 `fold`.", &tag(src)).unwrap_err();
        assert!(problems.iter().any(|p| p.contains("`fold`")), "{problems:?}");
        // A source span nothing matched is missing.
        let problems = read("`fold=0`와 `x`를 설정합니다.", &tag(src)).unwrap_err();
        assert!(problems.iter().any(|p| p.contains("Makefiles") && p.contains("missing")), "{problems:?}");
    }

    #[test]
    fn emphasis_may_split_but_links_may_not_repeat() {
        let src = "Set **`b = true` in `c.toml`**.";
        assert!(read("`c.toml`에서 <b1>`b = true`</b1>를 <b1>설정</b1>합니다.", &tag(src)).is_ok());
        let src = "See [the book](u).";
        assert!(read("<a1>책</a1>과 <a1>책</a1>을 보십시오.", &tag(src)).is_err());
    }

    #[test]
    fn tags_must_nest_and_appear() {
        let src = "A **bold *and italic* text**.";
        assert!(read("<b1>굵고 <i2>기울임</b1></i2>.", &tag(src)).is_err());
        assert!(read("굵고 기울임.", &tag(src)).is_err());
        let src = "Use <https://x.org> now.";
        assert!(read("지금 사용하십시오.", &tag(src)).is_err());
    }

    // --- render ------------------------------------------------------------

    #[test]
    fn render_restores_the_source_when_nothing_moved() {
        for src in [
            "The **outer product** of _u_ and `v` is ~~wrong~~.",
            "See [the book](https://x.org/a) and [arena], or [ASan][clang-asan].",
            "Press <kbd>Ctrl</kbd>, see ![fig](a.png), note[^1], and \\\\(T\\\\).",
        ] {
            let tagged = tag(src);
            let tree = read(&tagged.text, &tagged).unwrap();
            assert_eq!(render(&tree, &tagged, &ctx(&["arena", "clang-asan"])).markdown, src);
        }
    }

    #[test]
    fn render_moves_marks_where_commonmark_closes_them() {
        assert_eq!(
            translate("from the **outer product** of it.", "<b1>외적(outer product)</b1>에서 유래합니다."),
            "**외적**(outer product)에서 유래합니다."
        );
        assert_eq!(translate("The _arity_ is it.", "<i1>arity</i1>는 그것입니다."), "*arity*는 그것입니다.");
        assert_eq!(
            translate("**[Fetch](./f.md)** reads.", "<b1><a2>Fetch</a2></b1>는 읽습니다."),
            "[**Fetch**](./f.md)는 읽습니다."
        );
        assert_eq!(
            translate("No _\"formal power\"_: none.", "<i1>\"공식 권한\"</i1>은 없습니다: 없습니다."),
            "\"_공식 권한_\"은 없습니다: 없습니다."
        );
        assert_eq!(
            translate("Short for _argument-position `impl Trait`_.", "<i1>인자 위치 `impl Trait`</i1>의 줄임말입니다."),
            "_인자 위치 `impl Trait`의_ 줄임말입니다."
        );
    }

    #[test]
    fn marks_that_pair_across_emphasis_all_turn_to_stars() {
        assert_eq!(
            translate("commands that _use_ it but _from_ here.", "그것을 <i1>사용</i1>하면서도 여기<i2>에서</i2> 쓰는 명령입니다."),
            "그것을 *사용*하면서도 여기*에서* 쓰는 명령입니다."
        );
    }

    #[test]
    fn an_escaped_backtick_in_prose_is_taggable() {
        let tagged = tag("Use \\` to quote things.");
        let tree = read(&tagged.text, &tagged).unwrap();
        assert_eq!(render(&tree, &tagged, &ctx(&[])).markdown, "Use \\` to quote things.");
    }

    #[test]
    fn source_emphasis_html_and_mailto_links_verify() {
        assert_eq!(translate("a <em>b</em> c", "가 <h1>나</h1> 다"), "가 <em>나</em> 다");
        assert_eq!(translate("Mail <mailto:x@y.z> now.", "지금 <x1/>로 메일을 보내십시오."), "지금 <mailto:x@y.z>로 메일을 보내십시오.");
    }

    #[test]
    fn brackets_the_source_escaped_stay_escaped() {
        let doc = ctx(&["n"]);
        let mut counter = 0;
        let tagged = tagify("Use \\[n\\] here.", &doc, Position::Inline, &mut counter).unwrap();
        let tree = read("여기서 [n]을 씁니다.", &tagged).unwrap();
        let rendered = render(&tree, &tagged, &doc);
        assert!(rendered.verified);
        assert_eq!(rendered.markdown, "여기서 \\[n\\]을 씁니다.");
    }

    #[test]
    fn only_a_particle_is_taken_into_emphasis() {
        assert_eq!(
            translate("The _mid-level [IR](#ir)_.", "<i1>중간 수준 <a2>IR</a2></i1>입니다."),
            "<em>중간 수준 [IR](#ir)</em>입니다."
        );
    }

    #[test]
    fn a_changed_shortcut_reference_becomes_a_full_reference() {
        assert_eq!(translate("A [arena] allocator.", "<a1>아레나</a1> 할당자입니다."), "[아레나][arena] 할당자입니다.");
        assert_eq!(
            translate("Part of [MCP] (GCI).", "<a1>MCP</a1>(GCI)의 일부입니다."),
            "[MCP][MCP](GCI)의 일부입니다."
        );
    }

    #[test]
    fn escapes_are_minimal_and_position_aware() {
        assert_eq!(translate("Keep min_heap_size and 2 * 3.", "min_heap_size와 2 * 3을 유지합니다."), "min_heap_size와 2 \\* 3을 유지합니다.");
        assert_eq!(translate("A [not a link] here.", "[링크 아님]이 여기 있습니다."), "[링크 아님]이 여기 있습니다.");
        assert_eq!(translate("Hash - sign.", "# 기호입니다."), "\\# 기호입니다.");
        let mut counter = 0;
        let tagged = tagify("a or b", &ctx(&[]), Position::TableCell, &mut counter).unwrap();
        let tree = read("a | b", &tagged).unwrap();
        assert_eq!(render(&tree, &tagged, &ctx(&[])).markdown, "a \\| b");
    }

    // --- MyST, MDX, plain text --------------------------------------------------

    fn tag_in(dialect: Dialect, source: &str) -> Tagged {
        let mut counter = 0;
        tagify(source, &DocContext::new("", dialect), Position::Inline, &mut counter).unwrap()
    }

    fn translate_in(dialect: Dialect, source: &str, reply: &str) -> String {
        let ctx = DocContext::new("", dialect);
        let tagged = tag_in(dialect, source);
        let tree = read(reply, &tagged).unwrap_or_else(|p| panic!("{reply}: {p:?}"));
        let rendered = render(&tree, &tagged, &ctx);
        assert!(rendered.verified, "{}", rendered.markdown);
        rendered.markdown
    }

    #[test]
    fn myst_label_roles_are_links_and_other_roles_stay_as_written() {
        assert_eq!(
            tag_in(Dialect::Myst, "See {ref}`the guide <install-nix>`, {term}`Nix language` and {py:func}`f <m.f>`.").text,
            "See <a1>the guide</a1>, <a2>Nix language</a2> and <a3>f</a3>."
        );
        assert_eq!(
            tag_in(Dialect::Myst, "Run {ref}`install-nix` and press {kbd}`Ctrl`.").text,
            "Run {ref}`install-nix` and press {kbd}`Ctrl`."
        );
        // CommonMark has no roles: the same bytes are text and code.
        let common = tag("Run {ref}`install-nix` now.");
        assert_eq!(common.text, "Run {ref}`install-nix` now.");
        assert!(common.tags.iter().all(|t| t.role.is_none()));
    }

    #[test]
    fn myst_has_no_strikethrough() {
        assert_eq!(tag_in(Dialect::Myst, "It ~~was~~ is.").text, "It ~~was~~ is.");
        assert_eq!(translate_in(Dialect::Myst, "It ~~was~~ is.", "~~였~~입니다."), "~~였~~입니다.");
    }

    #[test]
    fn a_myst_role_comes_back_whether_or_not_the_reply_names_it() {
        let src = "First {ref}`install-nix`, then `nix run`.";
        for reply in ["먼저 {ref}`install-nix`을, 그다음 `nix run`을 실행합니다.", "먼저 `install-nix`을, 그다음 `nix run`을 실행합니다."] {
            assert_eq!(translate_in(Dialect::Myst, src, reply), "먼저 {ref}`install-nix`을, 그다음 `nix run`을 실행합니다.");
        }
    }

    #[test]
    fn code_and_a_role_with_the_same_content_keep_their_places() {
        let src = "Use `x` or {ref}`x`.";
        assert_eq!(translate_in(Dialect::Myst, src, "{ref}`x`나 `x`를 쓰십시오."), "{ref}`x`나 `x`를 쓰십시오.");
    }

    #[test]
    fn a_label_role_holds_plain_text_only() {
        let tagged = tag_in(Dialect::Myst, "See {ref}`the **big** guide <g>` and **this**.");
        assert_eq!(tagged.text, "See <a1>the **big** guide</a1> and <b2>this</b2>.");
        let problems = read("<a1>큰 <b2>안내서</b2></a1>를 보십시오.", &tagged).unwrap_err();
        assert!(problems.iter().any(|p| p.contains("<a1> holds plain text only")), "{problems:?}");
    }

    #[test]
    fn a_translated_label_role_is_written_explicitly() {
        let src = "A {term}`Nix language` file, see {ref}`the guide <install-nix>`.";
        assert_eq!(
            translate_in(Dialect::Myst, src, "<a1>Nix 언어</a1> 파일이며 <a2>안내서</a2>를 보십시오."),
            "{term}`Nix 언어 <Nix language>` 파일이며 {ref}`안내서 <install-nix>`를 보십시오."
        );
        assert_eq!(
            translate_in(Dialect::Myst, src, "<a1>Nix language</a1> 파일이며 <a2>the guide</a2>를 보십시오."),
            "{term}`Nix language` 파일이며 {ref}`the guide <install-nix>`를 보십시오."
        );
    }

    #[test]
    fn emphasis_around_a_role_closes_before_a_particle() {
        assert_eq!(
            translate_in(Dialect::Myst, "The **{term}`Nix`** tool.", "<b1><a2>Nix</a2></b1>는 도구입니다."),
            "**{term}`Nix`는** 도구입니다."
        );
    }

    #[test]
    fn a_code_span_that_repeats_a_role_label_is_text_the_source_wrote() {
        let tagged = tag_in(Dialect::Myst, "Install {term}`Nixpkgs` now.");
        assert!(read("<a1>Nixpkgs</a1>를 지금 설치하십시오. `Nixpkgs`는 큽니다.", &tagged).is_ok());
    }

    #[test]
    fn a_role_opened_inside_a_label_role_or_left_empty_is_a_problem() {
        let tagged = tag_in(Dialect::Myst, "See {ref}`a <x>` and {ref}`b <y>`.");
        let problems = read("<a1><a2>b</a2></a1>를 보십시오.", &tagged).unwrap_err();
        assert!(problems.iter().any(|p| p.contains("<a1> holds plain text only")), "{problems:?}");
        let problems = read("<a1></a1>와 <a2>b</a2>를 보십시오.", &tagged).unwrap_err();
        assert!(problems.iter().any(|p| p.contains("<a1> is empty")), "{problems:?}");
    }

    #[test]
    fn a_role_the_reply_writes_around_source_words_verifies() {
        assert_eq!(
            translate_in(
                Dialect::Myst,
                "Install {term}`Nixpkgs` now. It is **big**.",
                "<a1>Nixpkgs</a1>를 지금 설치하십시오. {term}`Nixpkgs`는 <b2>큽니다</b2>."
            ),
            "{term}`Nixpkgs`를 지금 설치하십시오. {term}`Nixpkgs`는 **큽니다**."
        );
    }

    #[test]
    fn a_label_with_a_backtick_stays_as_written() {
        assert_eq!(
            tag_in(Dialect::Myst, "See {ref}`` the `foo` docs <x> `` now.").text,
            "See {ref}`` the `foo` docs &lt;x&gt; `` now."
        );
    }

    #[test]
    fn mdx_expressions_and_jsx_are_tags() {
        assert_eq!(
            tag_in(Dialect::Mdx, "For <Language />, see *<cite>[x](u)</cite>* in {year}.").text,
            "For <x1/>, see <i2><h3><a4>x</a4></h3></i2> in <x5/>."
        );
        assert_eq!(tag_in(Dialect::Mdx, "Write `{x}` or \\{x\\}.").text, "Write `{x}` or {x}.");
        let mut counter = 0;
        assert!(tagify("An open { brace.", &DocContext::new("", Dialect::Mdx), Position::Inline, &mut counter).is_err());
    }

    #[test]
    fn mdx_text_escapes_braces_and_angle_brackets() {
        assert_eq!(
            translate_in(Dialect::Mdx, "Use \\{x\\} if a \\< b, in {year}.", "a &lt; b이면 {x}를 <x1/>에 씁니다."),
            "a \\< b이면 \\{x}를 {year}에 씁니다."
        );
    }

    #[test]
    fn plain_text_carries_no_tags_and_no_escapes() {
        let mut counter = 0;
        let ctx = DocContext::new("", Dialect::Mdx);
        let tagged = tagify("2 * 3 <b> `x`", &ctx, Position::Plain, &mut counter).unwrap();
        assert_eq!(tagged.text, "2 * 3 &lt;b&gt; `x`");
        assert!(tagged.tags.is_empty());
        let tree = read("2 * 3 &lt;b&gt; <i1>`x`</i1>", &tagged).unwrap();
        let rendered = render(&tree, &tagged, &ctx);
        assert_eq!(rendered.markdown, "2 * 3 <b> `x`");
    }

    #[test]
    fn a_tilde_that_could_strike_through_is_escaped() {
        assert_eq!(translate_in(Dialect::Mdx, "ooze \\~(++)\\~ out", "~(++)~ 흘러나옵니다"), "\\~(++)\\~ 흘러나옵니다");
        assert_eq!(translate("About ~ 10.", "약 ~ 10입니다."), "약 ~ 10입니다.");
    }

    #[test]
    fn code_comes_back_as_the_source_wrote_it() {
        assert_eq!(
            translate("Remove the `Makefiles` now.", "`Makefile`을 지금 제거하십시오."),
            "`Makefiles`을 지금 제거하십시오."
        );
        assert_eq!(translate("the Foo type", "`Foo` 타입"), "`Foo` 타입");
    }
}

