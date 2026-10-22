//! XHTML codec for inline tag transport.
//!
//! An inline element becomes a numbered tag the model moves with the words it
//! wraps; the serializer writes the element's own start and end tags back, so
//! there is no delimiter rule to satisfy. Code shows as a backtick span the
//! model can read and comes back as the source bytes. Reading the reply is
//! shared with the Markdown codec (`markdown::read`).

use super::markdown::{Dialect, Node, Position, Rendered, Tag, TagKind, Tagged, Tree, Untaggable};
use yeokja_parser_xhtml::xml::{self, Element, Node as XmlNode};
use yeokja_parser_xhtml::{CODE, is_untranslated};

const ITALIC: &[&str] = &["em", "i", "cite", "dfn", "var"];
const BOLD: &[&str] = &["strong", "b"];

fn escape(c: char, out: &mut String) {
    match c {
        '&' => out.push_str("&amp;"),
        '<' => out.push_str("&lt;"),
        '>' => out.push_str("&gt;"),
        _ => out.push(c),
    }
}

/// Text as the model sees it: entities read, whitespace runs folded (XHTML
/// renders them as one space), `<`, `>` and `&` escaped.
fn shown_text(source: &str, range: std::ops::Range<usize>, out: &mut String) -> Result<(), Untaggable> {
    let mut space = false;
    let mut backtick = false;
    xml::decode_each(source, range, |c, _| {
        // A backtick in prose would read back as a code span.
        backtick |= c == '`';
        if c.is_ascii_whitespace() {
            space = true;
            return;
        }
        if space {
            out.push(' ');
            space = false;
        }
        escape(c, out);
    })
    .map_err(|e| Untaggable(e.to_string()))?;
    if backtick {
        return Err(Untaggable("a backtick in the text".into()));
    }
    if space {
        out.push(' ');
    }
    Ok(())
}

fn has_letter(source: &str, nodes: &[XmlNode]) -> bool {
    nodes.iter().any(|node| match node {
        XmlNode::Text(range) => xml::decode(source, range.clone()).is_ok_and(|t| t.chars().any(char::is_alphabetic)),
        XmlNode::Element(e) => !is_untranslated(e, source) && has_letter(source, &e.children),
        XmlNode::Other(_) => false,
    })
}

/// A footnote reference: a link whose only content is `<sup class="footnote">`.
fn is_footnote_ref(source: &str, e: &Element) -> bool {
    let elements: Vec<&Element> = e
        .children
        .iter()
        .filter_map(|n| match n {
            XmlNode::Element(c) => Some(c),
            _ => None,
        })
        .collect();
    e.local() == "a"
        && elements.len() == 1
        && elements[0].local() == "sup"
        && elements[0].has_class(source, "footnote")
        && e.children.iter().all(|n| !matches!(n, XmlNode::Text(r) if !source[r.clone()].trim().is_empty()))
}

struct Tagger<'a> {
    source: &'a str,
    text: String,
    tags: Vec<Tag>,
    counter: &'a mut usize,
}

impl Tagger<'_> {
    fn next(&mut self) -> usize {
        *self.counter += 1;
        *self.counter
    }

    fn opaque(&mut self, range: std::ops::Range<usize>) {
        let n = self.next();
        self.text.push_str(&format!("<x{n}/>"));
        self.tags.push(Tag::verbatim(n, TagKind::Opaque, self.source[range].to_string(), String::new(), None));
    }

    fn nodes(&mut self, nodes: &[XmlNode]) -> Result<(), Untaggable> {
        for node in nodes {
            match node {
                XmlNode::Text(range) => shown_text(self.source, range.clone(), &mut self.text)?,
                XmlNode::Other(range) => self.opaque(range.clone()),
                XmlNode::Element(e) => self.element(e)?,
            }
        }
        Ok(())
    }

    fn element(&mut self, e: &Element) -> Result<(), Untaggable> {
        let source = self.source;
        let name = e.local();
        if e.end.is_none()
            || is_untranslated(e, source)
            || is_footnote_ref(source, e)
            || !has_letter(source, &e.children)
        {
            self.opaque(e.range());
            return Ok(());
        }
        if CODE.contains(&name) {
            if e.children.iter().any(|n| !matches!(n, XmlNode::Text(_))) {
                self.opaque(e.range());
                return Ok(());
            }
            let mut content = String::new();
            xml::decode_each(source, e.inner(), |c, _| content.push(if c.is_ascii_whitespace() { ' ' } else { c }))
                .map_err(|err| Untaggable(err.to_string()))?;
            let n = self.next();
            let fence = "`".repeat(longest_backtick_run(&content) + 1);
            let pad = if content.starts_with('`') || content.ends_with('`') { " " } else { "" };
            let mut shown = String::new();
            for c in content.chars() {
                escape(c, &mut shown);
            }
            self.text.push_str(&format!("{fence}{pad}{shown}{pad}{fence}"));
            self.tags.push(Tag::verbatim(n, TagKind::Code, source[e.range()].to_string(), String::new(), Some(content)));
            return Ok(());
        }
        let kind = if ITALIC.contains(&name) {
            TagKind::Italic
        } else if BOLD.contains(&name) {
            TagKind::Bold
        } else if name == "a" {
            TagKind::Link
        } else {
            TagKind::Html
        };
        let n = self.next();
        let letter = match kind {
            TagKind::Italic => 'i',
            TagKind::Bold => 'b',
            TagKind::Link => 'a',
            _ => 'h',
        };
        let end = e.end.clone().expect("checked above");
        self.tags.push(Tag::verbatim(n, kind, source[e.start.clone()].to_string(), source[end].to_string(), None));
        self.text.push_str(&format!("<{letter}{n}>"));
        self.nodes(&e.children)?;
        self.text.push_str(&format!("</{letter}{n}>"));
        Ok(())
    }
}

fn longest_backtick_run(text: &str) -> usize {
    text.split(|c| c != '`').map(str::len).max().unwrap_or(0)
}

/// A segment's source as tag text. A plain segment (an attribute value) goes
/// as its text.
pub fn tagify(source: &str, position: Position, counter: &mut usize) -> Result<Tagged, Untaggable> {
    let mut text = String::new();
    if position == Position::Plain {
        let decoded = xml::decode(source, 0..source.len()).map_err(|e| Untaggable(e.to_string()))?;
        for c in decoded.chars() {
            escape(c, &mut text);
        }
        return Ok(Tagged { source: source.to_string(), text, tags: Vec::new(), position, dialect: Dialect::Xhtml });
    }
    let nodes = xml::parse(source).map_err(|e| Untaggable(e.to_string()))?;
    let mut tagger = Tagger { source, text, tags: Vec::new(), counter };
    tagger.nodes(&nodes)?;
    Ok(Tagged { source: source.to_string(), text: tagger.text, tags: tagger.tags, position, dialect: Dialect::Xhtml })
}

/// Text written into the document: `&`, `<`, `>` escaped, a no-break space
/// as a character reference (named entities need the DTD), and in an
/// attribute value both quotes. An entity the model wrote out (`&nbsp;`,
/// `&mdash;`) is read as the character it names: the model sees XHTML and
/// answers in it, and printing the entity as text is never what it meant.
fn write_text(text: &str, attribute: bool, out: &mut String) {
    for c in read_entities(text).chars() {
        match c {
            '\u{a0}' => out.push_str("&#160;"),
            '"' if attribute => out.push_str("&quot;"),
            '\'' if attribute => out.push_str("&#39;"),
            c => escape(c, out),
        }
    }
}

/// `text` with every entity it writes (`&nbsp;`, `&#160;`) read as its
/// character; anything else after a `&` stays as written.
pub(crate) fn read_entities(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(at) = rest.find('&') {
        out.push_str(&rest[..at]);
        let tail = &rest[at + 1..];
        let named = tail.find(';').filter(|&end| end <= 10).and_then(|end| xml::entity(&tail[..end]).map(|c| (c, end)));
        match named {
            Some((c, end)) => {
                out.push(c);
                rest = &tail[end + 1..];
            }
            None => {
                out.push('&');
                rest = tail;
            }
        }
    }
    out.push_str(rest);
    out
}

/// The reply as XHTML: each tag's own markup restored around the words.
pub fn render(tree: &Tree, tagged: &Tagged) -> Rendered {
    let attribute = tagged.position == Position::Plain;
    let tag = |n: usize| tagged.tags.iter().find(|t| t.n == n).expect("read only yields known tags");
    let mut out = String::new();
    for node in &tree.nodes {
        match node {
            Node::Text(t) => write_text(t, attribute, &mut out),
            Node::Open(n) => out.push_str(&tag(*n).open),
            Node::Close(n) => out.push_str(&tag(*n).close),
            Node::Void(n) | Node::Code { tag: Some(n), .. } => out.push_str(&tag(*n).open),
            Node::Code { tag: None, written } => write_text(written.trim_matches('`'), attribute, &mut out),
        }
    }
    Rendered { markdown: out, strategies: Vec::new(), verified: true }
}

/// The source's text without its markup, for telling whether a code span of
/// the reply marks up words the source wrote as text.
pub fn prose(source: &str) -> String {
    fn walk(source: &str, nodes: &[XmlNode], out: &mut String) {
        for node in nodes {
            match node {
                XmlNode::Text(r) => out.push_str(&xml::decode(source, r.clone()).unwrap_or_default()),
                XmlNode::Element(e) if CODE.contains(&e.local()) => out.push(' '),
                XmlNode::Element(e) => walk(source, &e.children, out),
                XmlNode::Other(_) => {}
            }
        }
    }
    let mut out = String::new();
    if let Ok(nodes) = xml::parse(source) {
        walk(source, &nodes, &mut out);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::inline::markdown::read;

    fn tag(source: &str) -> Tagged {
        tagify(source, Position::Inline, &mut 0).unwrap()
    }

    fn translate(source: &str, reply: &str) -> String {
        let tagged = tag(source);
        render(&read(reply, &tagged).unwrap(), &tagged).markdown
    }

    #[test]
    fn inline_elements_become_numbered_tags() {
        let t = tag("A <em>b</em> and <strong class=\"k\">c</strong> <a href=\"x.html#y\">d</a> <span style=\"z\">e</span>.");
        assert_eq!(t.text, "A <i1>b</i1> and <b2>c</b2> <a3>d</a3> <h4>e</h4>.");
    }

    #[test]
    fn void_footnote_and_letterless_elements_are_opaque() {
        let t = tag("x<br />y<a class=\"footnote\" href=\"#n1\"><sup class=\"footnote\">[1]</sup></a> <span>  </span><img src=\"a\" alt=\"b\" />");
        assert_eq!(t.text, "x<x1/>y<x2/> <x3/><x4/>");
    }

    #[test]
    fn code_shows_its_content_and_comes_back_verbatim() {
        let t = tag("Call <code>a&lt;b</code> now.");
        assert_eq!(t.text, "Call `a&lt;b` now.");
        assert_eq!(translate("Call <code>a&lt;b</code> now.", "지금 `a&lt;b`를 호출합니다."), "지금 <code>a&lt;b</code>를 호출합니다.");
    }

    #[test]
    fn whitespace_folds_and_entities_read() {
        let t = tag("One&nbsp;two\n   three &amp; four");
        assert_eq!(t.text, "One\u{a0}two three &amp; four");
    }

    #[test]
    fn render_restores_source_tags_where_korean_puts_them() {
        let source = "The <em>outer product</em> of <a href=\"#v\">vectors</a>.";
        assert_eq!(
            translate(source, "<a2>벡터</a2>의 <i1>외적</i1>에서"),
            "<a href=\"#v\">벡터</a>의 <em>외적</em>에서"
        );
        assert_eq!(translate("A&nbsp;b", "가\u{a0}나 & 다"), "가&#160;나 &amp; 다");
    }

    #[test]
    fn entities_the_model_writes_out_are_read() {
        assert_eq!(translate("A b", "가&nbsp;나 &mdash; 다 &#8212; R&D &bogus;"), "가&#160;나 \u{2014} 다 \u{2014} R&amp;D &amp;bogus;");
    }

    #[test]
    fn plain_values_escape_quotes() {
        let tagged = tagify("A &quot;nice&quot; chart", Position::Plain, &mut 0).unwrap();
        assert_eq!(tagged.text, "A \"nice\" chart");
        let tree = read("\"멋진\" 도표 & 'x'", &tagged).unwrap();
        assert_eq!(render(&tree, &tagged).markdown, "&quot;멋진&quot; 도표 &amp; &#39;x&#39;");
    }

    #[test]
    fn missing_tags_are_problems() {
        let tagged = tag("A <em>b</em> c.");
        assert!(read("가 나 다.", &tagged).is_err());
    }

    #[test]
    fn backtick_in_prose_is_untaggable() {
        assert!(tagify("a `b` c", Position::Inline, &mut 0).is_err());
    }
}
