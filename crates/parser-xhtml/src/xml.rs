//! A well-formedness-checking XML reader that keeps every construct's byte
//! range, so reconstruction can splice into the source instead of
//! re-serializing it. Works on whole documents and on fragments (a segment's
//! source), which may have several top-level nodes.

use crate::entities::ENTITIES;
use std::ops::Range;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct XmlError {
    pub offset: usize,
    pub message: String,
}

impl std::fmt::Display for XmlError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "byte {}: {}", self.offset, self.message)
    }
}

#[derive(Debug, Clone)]
pub struct Attr {
    pub name: String,
    /// The value's bytes, inside the quotes.
    pub value: Range<usize>,
}

#[derive(Debug, Clone)]
pub struct Element {
    /// The qualified name as written (`p`, `epub:switch`).
    pub name: String,
    pub attrs: Vec<Attr>,
    /// The start tag.
    pub start: Range<usize>,
    /// The end tag; `None` for an empty-element tag (`<br/>`).
    pub end: Option<Range<usize>>,
    pub children: Vec<Node>,
}

impl Element {
    /// The name without its namespace prefix.
    pub fn local(&self) -> &str {
        self.name.rsplit(':').next().unwrap_or(&self.name)
    }

    pub fn range(&self) -> Range<usize> {
        self.start.start..self.end.as_ref().map_or(self.start.end, |end| end.end)
    }

    /// The bytes between the start and end tags.
    pub fn inner(&self) -> Range<usize> {
        self.start.end..self.end.as_ref().map_or(self.start.end, |end| end.start)
    }

    pub fn attr<'a>(&self, source: &'a str, name: &str) -> Option<&'a str> {
        self.attrs
            .iter()
            .find(|a| a.name == name)
            .map(|a| &source[a.value.clone()])
    }

    pub fn has_class(&self, source: &str, class: &str) -> bool {
        self.attr(source, "class")
            .is_some_and(|c| c.split_ascii_whitespace().any(|c| c == class))
    }
}

#[derive(Debug, Clone)]
pub enum Node {
    Element(Element),
    Text(Range<usize>),
    /// A comment, processing instruction, DOCTYPE or CDATA section.
    Other(Range<usize>),
}

impl Node {
    pub fn range(&self) -> Range<usize> {
        match self {
            Node::Element(e) => e.range(),
            Node::Text(r) | Node::Other(r) => r.clone(),
        }
    }
}

fn error(offset: usize, message: impl Into<String>) -> XmlError {
    XmlError {
        offset,
        message: message.into(),
    }
}

fn is_name_byte(b: u8) -> bool {
    !(b.is_ascii_whitespace() || matches!(b, b'/' | b'>' | b'=' | b'<' | b'"' | b'\''))
}

fn find(source: &str, from: usize, needle: &str) -> Option<usize> {
    source[from..].find(needle).map(|at| from + at)
}

/// Read `source` into its top-level nodes.
pub fn parse(source: &str) -> Result<Vec<Node>, XmlError> {
    let b = source.as_bytes();
    let mut roots: Vec<Node> = Vec::new();
    let mut stack: Vec<Element> = Vec::new();
    let push = |node: Node, stack: &mut Vec<Element>, roots: &mut Vec<Node>| match stack.last_mut()
    {
        Some(top) => top.children.push(node),
        None => roots.push(node),
    };
    let mut i = 0;
    while i < b.len() {
        if b[i] != b'<' {
            let end = find(source, i, "<").unwrap_or(b.len());
            check_text(source, i..end)?;
            push(Node::Text(i..end), &mut stack, &mut roots);
            i = end;
            continue;
        }
        let rest = &source[i..];
        if rest.starts_with("<!--") {
            let end =
                find(source, i + 4, "-->").ok_or_else(|| error(i, "unterminated comment"))? + 3;
            push(Node::Other(i..end), &mut stack, &mut roots);
            i = end;
        } else if rest.starts_with("<![CDATA[") {
            let end = find(source, i + 9, "]]>")
                .ok_or_else(|| error(i, "unterminated CDATA section"))?
                + 3;
            push(Node::Other(i..end), &mut stack, &mut roots);
            i = end;
        } else if rest.starts_with("<?") {
            let end = find(source, i + 2, "?>")
                .ok_or_else(|| error(i, "unterminated processing instruction"))?
                + 2;
            push(Node::Other(i..end), &mut stack, &mut roots);
            i = end;
        } else if rest.starts_with("<!") {
            // DOCTYPE, possibly with an internal subset in brackets.
            let mut j = i + 2;
            let mut depth = 0usize;
            while j < b.len() && !(b[j] == b'>' && depth == 0) {
                match b[j] {
                    b'[' => depth += 1,
                    b']' => depth = depth.saturating_sub(1),
                    _ => {}
                }
                j += 1;
            }
            if j >= b.len() {
                return Err(error(i, "unterminated declaration"));
            }
            push(Node::Other(i..j + 1), &mut stack, &mut roots);
            i = j + 1;
        } else if rest.starts_with("</") {
            let close = find(source, i, ">").ok_or_else(|| error(i, "unterminated end tag"))?;
            let name = source[i + 2..close].trim();
            let mut element = stack
                .pop()
                .ok_or_else(|| error(i, format!("</{name}> closes nothing")))?;
            if element.name != name {
                return Err(error(i, format!("</{name}> closes <{}>", element.name)));
            }
            element.end = Some(i..close + 1);
            push(Node::Element(element), &mut stack, &mut roots);
            i = close + 1;
        } else {
            let (element, end, empty) = start_tag(source, i)?;
            i = end;
            if empty {
                push(Node::Element(element), &mut stack, &mut roots);
            } else {
                stack.push(element);
            }
        }
    }
    if let Some(open) = stack.last() {
        return Err(error(
            open.start.start,
            format!("<{}> is never closed", open.name),
        ));
    }
    Ok(roots)
}

/// A start tag at `at`: the element, the offset after it, and whether it is
/// an empty-element tag.
fn start_tag(source: &str, at: usize) -> Result<(Element, usize, bool), XmlError> {
    let b = source.as_bytes();
    let mut j = at + 1;
    while j < b.len() && is_name_byte(b[j]) {
        j += 1;
    }
    if j == at + 1 {
        return Err(error(at, "a bare < must be written &lt;"));
    }
    let name = source[at + 1..j].to_string();
    let mut attrs = Vec::new();
    loop {
        while j < b.len() && b[j].is_ascii_whitespace() {
            j += 1;
        }
        match b.get(j) {
            None => return Err(error(at, format!("unterminated start tag <{name}>"))),
            Some(b'>') => {
                let element = Element {
                    name,
                    attrs,
                    start: at..j + 1,
                    end: None,
                    children: Vec::new(),
                };
                return Ok((element, j + 1, false));
            }
            Some(b'/') if b.get(j + 1) == Some(&b'>') => {
                let element = Element {
                    name,
                    attrs,
                    start: at..j + 2,
                    end: None,
                    children: Vec::new(),
                };
                return Ok((element, j + 2, true));
            }
            Some(_) => {}
        }
        let name_start = j;
        while j < b.len() && is_name_byte(b[j]) {
            j += 1;
        }
        if j == name_start {
            return Err(error(j, format!("malformed attribute in <{name}>")));
        }
        let attr_name = source[name_start..j].to_string();
        while j < b.len() && b[j].is_ascii_whitespace() {
            j += 1;
        }
        if b.get(j) != Some(&b'=') {
            return Err(error(j, format!("attribute {attr_name} has no value")));
        }
        j += 1;
        while j < b.len() && b[j].is_ascii_whitespace() {
            j += 1;
        }
        let quote = match b.get(j) {
            Some(q @ (b'"' | b'\'')) => *q,
            _ => {
                return Err(error(
                    j,
                    format!("attribute {attr_name} value is not quoted"),
                ));
            }
        };
        let value_start = j + 1;
        let value_end = source[value_start..]
            .find(quote as char)
            .map(|at| value_start + at)
            .ok_or_else(|| error(j, format!("unterminated value of {attr_name}")))?;
        if source[value_start..value_end].contains('<') {
            return Err(error(
                value_start,
                format!("a < in the value of {attr_name}"),
            ));
        }
        check_text(source, value_start..value_end)?;
        attrs.push(Attr {
            name: attr_name,
            value: value_start..value_end,
        });
        j = value_end + 1;
    }
}

fn check_text(source: &str, range: Range<usize>) -> Result<(), XmlError> {
    decode_each(source, range, |_, _| {})
}

/// The character an entity name (`nbsp`, `#160`, `#xA0`) stands for: XML's
/// own five, character references, and the XHTML 1.1 DTD's names.
pub fn entity(name: &str) -> Option<char> {
    match name {
        "lt" => Some('<'),
        "gt" => Some('>'),
        "amp" => Some('&'),
        "quot" => Some('"'),
        "apos" => Some('\''),
        _ => {
            if let Some(hex) = name.strip_prefix("#x").or_else(|| name.strip_prefix("#X")) {
                return u32::from_str_radix(hex, 16).ok().and_then(char::from_u32);
            }
            if let Some(dec) = name.strip_prefix('#') {
                return dec.parse::<u32>().ok().and_then(char::from_u32);
            }
            ENTITIES
                .binary_search_by(|(n, _)| (*n).cmp(name))
                .ok()
                .map(|at| ENTITIES[at].1)
        }
    }
}

/// Call `each` with every character `range` of `source` stands for and the
/// bytes it was written with (one byte range per entity).
pub fn decode_each(
    source: &str,
    range: Range<usize>,
    mut each: impl FnMut(char, Range<usize>),
) -> Result<(), XmlError> {
    let mut i = range.start;
    while i < range.end {
        let c = source[i..].chars().next().expect("char boundary");
        if c == '&' {
            let semi = source[i..range.end]
                .find(';')
                .map(|at| i + at)
                .ok_or_else(|| error(i, "a bare & must be written &amp;"))?;
            let name = &source[i + 1..semi];
            let decoded =
                entity(name).ok_or_else(|| error(i, format!("unknown entity &{name};")))?;
            each(decoded, i..semi + 1);
            i = semi + 1;
        } else {
            each(c, i..i + c.len_utf8());
            i += c.len_utf8();
        }
    }
    Ok(())
}

/// The text `range` of `source` stands for.
pub fn decode(source: &str, range: Range<usize>) -> Result<String, XmlError> {
    let mut out = String::new();
    decode_each(source, range, |c, _| out.push(c))?;
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_ranges_and_nesting() {
        let src = r#"<p class="a">x <em>y</em><br/></p>"#;
        let nodes = parse(src).unwrap();
        let Node::Element(p) = &nodes[0] else {
            panic!()
        };
        assert_eq!(p.name, "p");
        assert_eq!(p.attr(src, "class"), Some("a"));
        assert_eq!(&src[p.start.clone()], r#"<p class="a">"#);
        assert_eq!(p.children.len(), 3);
        let Node::Element(br) = &p.children[2] else {
            panic!()
        };
        assert!(br.end.is_none());
    }

    #[test]
    fn decodes_dtd_entities() {
        let src = "a&nbsp;b&#160;&amp;&eacute;";
        assert_eq!(decode(src, 0..src.len()).unwrap(), "a\u{a0}b\u{a0}&é");
        assert!(decode("&bogus;", 0..7).is_err());
        assert!(decode("a & b", 0..5).is_err());
    }

    #[test]
    fn rejects_mismatched_tags() {
        assert!(parse("<p><em>x</p></em>").is_err());
        assert!(parse("<p>x").is_err());
    }

    #[test]
    fn reads_prolog() {
        let src = "<?xml version=\"1.0\"?>\n<!DOCTYPE html PUBLIC \"x\" \"y\">\n<html/>";
        let nodes = parse(src).unwrap();
        assert!(matches!(nodes.last(), Some(Node::Element(_))));
    }
}
