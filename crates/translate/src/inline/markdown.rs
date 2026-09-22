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
use std::ops::Range;

/// Document-level facts a segment cannot show on its own.
#[derive(Clone, Debug, Default)]
pub struct DocContext {
    /// Reference definition labels, normalized: `[text][label]` and `[label]`
    /// are links only when the document defines the label.
    reference_labels: HashSet<String>,
}

impl DocContext {
    pub fn from_markdown(source: &str) -> Self {
        let parser = Parser::new_ext(source, options());
        let reference_labels =
            parser.reference_definitions().iter().map(|(label, _)| normalize_label(label)).collect();
        Self { reference_labels }
    }
}

/// Where a segment sits, for the escapes that depend on it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Position {
    #[default]
    Inline,
    /// A table cell, where a bare `|` ends the cell.
    TableCell,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TagKind {
    Italic,
    Bold,
    Strike,
    Link,
    Html,
    Opaque,
    Math,
    Code,
}

impl TagKind {
    fn letter(self) -> char {
        match self {
            TagKind::Italic => 'i',
            TagKind::Bold => 'b',
            TagKind::Strike => 's',
            TagKind::Link => 'a',
            TagKind::Html => 'h',
            TagKind::Opaque => 'x',
            TagKind::Math => 'm',
            TagKind::Code => 'c',
        }
    }

    fn is_emphasis(self) -> bool {
        matches!(self, TagKind::Italic | TagKind::Bold | TagKind::Strike)
    }

    fn is_void(self) -> bool {
        matches!(self, TagKind::Opaque | TagKind::Math | TagKind::Code)
    }
}

#[derive(Clone, Debug)]
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
    /// How the construct reads back, for verifying a rendering.
    shape: String,
    /// The text it contributes when read back (math renders its own source).
    visible: String,
}

#[derive(Clone, Debug)]
pub struct Tagged {
    /// The segment's source string.
    pub source: String,
    /// The text the model sees.
    pub text: String,
    pub tags: Vec<Tag>,
    pub position: Position,
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

fn options() -> Options {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_STRIKETHROUGH);
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
    Parser::new_with_broken_link_callback(text, options(), Some(callback)).into_offset_iter().collect()
}

/// Byte ranges of the backtick code spans in `text`, marks included.
fn code_span_ranges(text: &str) -> Vec<Range<usize>> {
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

/// mdBook MathJax (`\\(…\\)`, `\\[…\\]`) and footnote references (`[^x]`,
/// whose definitions live elsewhere in the document) as one private-use
/// character each, so a segment read alone cannot misread them. Code spans
/// are left alone: `` `[^.,]+` `` is a pattern, not a footnote.
fn mask(source: &str) -> (String, Vec<String>) {
    let code = code_span_ranges(source);
    let mut out = String::new();
    let mut masked = Vec::new();
    let mut i = 0;
    while i < source.len() {
        if let Some(span) = code.iter().find(|r| r.start == i) {
            out.push_str(&source[span.clone()]);
            i = span.end;
            continue;
        }
        let rest = &source[i..];
        let escaped = source[..i].ends_with('\\') && !source[..i].ends_with("\\\\");
        let opening = [("\\\\(", "\\\\)"), ("\\\\[", "\\\\]"), ("[^", "]")]
            .into_iter()
            .find(|(open, _)| rest.starts_with(open) && !(escaped && *open == "[^"));
        if let Some((open, close)) = opening
            && let Some(rel) = rest[open.len()..].find(close)
        {
            let end = i + open.len() + rel + close.len();
            // A construct is never cut by a code span.
            if !code.iter().any(|r| r.start < end && r.end > i) {
                out.push(MASK_EDGE);
                out.push(char::from_u32(MASK_BASE + masked.len() as u32).expect("private use"));
                out.push(MASK_EDGE);
                masked.push(source[i..end].to_string());
                i = end;
                continue;
            }
        }
        let c = rest.chars().next().expect("char boundary");
        out.push(c);
        i += c.len_utf8();
    }
    (out, masked)
}

fn masked_index(c: char, masked: &[String]) -> Option<usize> {
    let v = c as u32;
    (MASK_BASE..MASK_BASE + masked.len() as u32).contains(&v).then(|| (v - MASK_BASE) as usize)
}

fn unmask(text: &str, masked: &[String]) -> String {
    let mut out = String::new();
    for piece in split_masked(text, masked) {
        match piece {
            Ok(c) => out.push(c),
            Err(i) => out.push_str(&masked[i]),
        }
    }
    out
}

/// The characters of `text`, with each edge-wrapped masked construct as its
/// index.
fn split_masked(text: &str, masked: &[String]) -> Vec<Result<char, usize>> {
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
    let (masked, spans) = mask(source);
    // Math is masked first: `\\(\sum_{j} x_j\\)` holds no emphasis marks.
    if !commonmark_emphasis(&masked).stray.is_empty() {
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
        tags.push(Tag { n, kind, open, close: String::new(), label: None, code: None, shape: shape.into(), visible });
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
                tags.push(Tag { n, kind, open: delim.clone(), close: delim, label: None, code: None, shape: String::new(), visible: String::new() });
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
                    shape: format!("c:{content}|"),
                    visible: String::new(),
                });
            }
            Event::InlineHtml(_) | Event::Html(_) if html_pairs.contains_key(&i) => {
                let n = next(counter);
                text.push_str(&format!("<h{n}>"));
                tags.push(Tag { n, kind: TagKind::Html, open: raw(), close: String::new(), label: None, code: None, shape: "h".into(), visible: String::new() });
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
                if raw_text
                    .char_indices()
                    .any(|(at, c)| c == '`' && !guarded[..range.start + at].ends_with('\\'))
                {
                    return Err(Untaggable("the source leaves a code span open".into()));
                }
                let t = t.strip_prefix(GUARD).map(|r| r.strip_prefix(' ').unwrap_or(r)).unwrap_or(t);
                for piece in split_masked(t, &spans) {
                    match piece {
                        Err(at) if spans[at].starts_with("[^") => {
                            let n = next(counter);
                            void(&mut text, &mut tags, n, TagKind::Opaque, spans[at].clone(), "f", String::new());
                        }
                        Err(at) => {
                            let n = next(counter);
                            let visible = spans[at].replace("\\\\", "\\");
                            void(&mut text, &mut tags, n, TagKind::Math, spans[at].clone(), "", visible);
                        }
                        Ok(c) => xml_escape(c, &mut text),
                    }
                }
            }
            _ => {}
        }
    }
    Ok(Tagged { source: source.to_string(), text, tags, position })
}

// ---- reading ---------------------------------------------------------------

enum Piece {
    Text(String),
    Code { written: String, content: String },
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
                pieces.push(Piece::Code { written, content });
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
    Code { written: String, content: String },
}

fn tokenize(pieces: Vec<Piece>) -> Vec<Token> {
    let mut tokens = Vec::new();
    for piece in pieces {
        let text = match piece {
            Piece::Code { written, content } => {
                tokens.push(Token::Code { written, content });
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
fn prose_of(source: &str) -> String {
    split_code_spans(source)
        .into_iter()
        .map(|piece| match piece {
            Piece::Text(t) => t,
            Piece::Code { .. } => " ".into(),
        })
        .collect()
}

pub fn read(reply: &str, tagged: &Tagged) -> Result<Tree, Vec<String>> {
    let by_n: HashMap<usize, &Tag> = tagged.tags.iter().map(|t| (t.n, t)).collect();
    let mut notes = Vec::new();
    let mut problems: Vec<String> = Vec::new();
    let name = |n: usize| format!("<{}{n}>", by_n[&n].kind.letter());

    // Lenient reading: the number is the tag's identity.
    let mut nodes: Vec<Node> = Vec::new();
    let mut open: Vec<usize> = Vec::new();
    let mut codes: Vec<(usize, String)> = Vec::new(); // node index, content
    for token in tokenize(split_code_spans(reply)) {
        match token {
            Token::Text(t) => nodes.push(Node::Text(t)),
            Token::Code { written, content } => {
                codes.push((nodes.len(), content));
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
    // span's bytes over a span the model changed.
    let source_codes: Vec<&Tag> = tagged.tags.iter().filter(|t| t.kind == TagKind::Code).collect();
    let mut unused: Vec<usize> = (0..source_codes.len()).collect();
    let mut assigned: HashMap<usize, usize> = HashMap::new(); // node index -> source code index
    for (node, content) in &codes {
        if let Some(pos) = unused.iter().position(|&c| source_codes[c].code.as_deref() == Some(content.as_str())) {
            assigned.insert(*node, unused.remove(pos));
        }
    }
    for (node, content) in &codes {
        if assigned.contains_key(node) {
            continue;
        }
        if let Some(pos) = unused.iter().position(|&c| stands_for(content, source_codes[c].code.as_deref().unwrap_or(""))) {
            assigned.insert(*node, unused.remove(pos));
        }
    }
    let prose = prose_of(&tagged.source);
    for (node, content) in &codes {
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
            TagKind::Link | TagKind::Html => o == 1 && c == 1,
            _ => v == 1,
        };
        if !ok {
            let what = if o + c + v == 0 { "is missing" } else { "appears more than once" };
            problems.push(format!("{} {what} — keep every tag exactly once", name(tag.n)));
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
    /// Markdown to write and the content CommonMark will read.
    Code(String, String),
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
                '_' => !(prev.is_some_and(|p| p.is_ascii_alphanumeric()) && next.is_some_and(|n| n.is_ascii_alphanumeric())),
                '~' => prev == Some('~') || next == Some('~'),
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
        for (i, tok) in toks.iter().enumerate() {
            match tok {
                Tok::Text(t) => {
                    let at_start = line_start && out.is_empty();
                    self.escape(t, at_start, &mut out);
                }
                Tok::Void(n) => out.push_str(&self.tags[n].open),
                Tok::Code(markdown, _) => out.push_str(markdown),
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
        let (masked, spans) = mask(markdown);
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
                            Err(at) if spans[at].starts_with("[^") => shape.push('f'),
                            Err(at) => text.push_str(&spans[at].replace("\\\\", "\\")),
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
                Tok::Code(_, content) => shape.push_str(&format!("c:{content}|")),
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
                Tok::Code(tag.open.clone(), tag.code.clone().unwrap_or_default())
            }
            Node::Code { tag: None, written } => {
                let run = written.chars().take_while(|c| *c == '`').count();
                let inner: String = written.chars().skip(run).take(written.chars().count().saturating_sub(2 * run)).collect();
                Tok::Code(written.clone(), code_content(&inner))
            }
        })
        .collect();
    let mut emphasis: Vec<usize> =
        tagged.tags.iter().filter(|t| t.kind.is_emphasis()).map(|t| t.n).collect();
    emphasis.sort_unstable();

    let mut strategies: HashMap<usize, Strategy> = HashMap::new();
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
        DocContext { reference_labels: labels.iter().map(|l| normalize_label(l)).collect() }
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

    #[test]
    fn code_comes_back_as_the_source_wrote_it() {
        assert_eq!(
            translate("Remove the `Makefiles` now.", "`Makefile`을 지금 제거하십시오."),
            "`Makefiles`을 지금 제거하십시오."
        );
        assert_eq!(translate("the Foo type", "`Foo` 타입"), "`Foo` 타입");
    }
}
