//! Text spans inside CommonMark raw HTML blocks. Tags, attributes, comments,
//! and literal elements stay outside translation spans.
use pulldown_cmark::{Event, Options, Parser};
use std::ops::Range;

const LITERAL: &[&str] = &["pre", "code", "script", "style", "textarea", "svg", "math"];

/// End and name of a syntactically complete tag. Respect quoted `>` in
/// attributes, and reject autolinks such as `<https://example.com>`.
fn tag(source: &str, start: usize) -> Option<(usize, &str, bool)> {
    let bytes = source.as_bytes();
    let mut i = start + 1;
    let closing = bytes.get(i) == Some(&b'/');
    if closing {
        i += 1;
    }
    let name_start = i;
    if !bytes.get(i)?.is_ascii_alphabetic() {
        return None;
    }
    while bytes
        .get(i)
        .is_some_and(|b| b.is_ascii_alphanumeric() || *b == b'-')
    {
        i += 1;
    }
    let name = &source[name_start..i];
    if !bytes
        .get(i)
        .is_some_and(|b| b.is_ascii_whitespace() || matches!(b, b'/' | b'>'))
    {
        return None;
    }
    let mut quote = None;
    while let Some(&b) = bytes.get(i) {
        match (quote, b) {
            (Some(q), b) if b == q => quote = None,
            (None, b'\'' | b'"') => quote = Some(b),
            (None, b'>') => return Some((i + 1, name, closing)),
            _ => {}
        }
        i += 1;
    }
    None
}

pub(super) fn prose_ranges(source: &str) -> Vec<Range<usize>> {
    scan(source, None).0
}

/// Literal elements may span several CommonMark HTML blocks (blank lines end
/// some HTML blocks). Only recognize openers that Markdown identifies as HTML,
/// so tags in indented/fenced code and escaped text cannot hide later prose.
pub(super) fn opaque_ranges(source: &str, options: Options) -> Vec<Range<usize>> {
    let candidates: Vec<_> = Parser::new_ext(source, options)
        .into_offset_iter()
        .filter_map(|(event, range)| {
            matches!(event, Event::Html(_) | Event::InlineHtml(_)).then_some(range)
        })
        .collect();
    scan(source, Some(&candidates)).1
}

fn scan(
    source: &str,
    candidates: Option<&[Range<usize>]>,
) -> (Vec<Range<usize>>, Vec<Range<usize>>) {
    let bytes = source.as_bytes();
    let lower = source.to_ascii_lowercase();
    let mut ranges = Vec::new();
    let mut opaque = Vec::new();
    let mut candidate_index = 0;
    let (mut start, mut i) = (0, 0);
    while i < bytes.len() {
        // Markup-looking text inside Markdown code is not an HTML tag. Keep
        // the entire code span/fence together for the Markdown parser below.
        if candidates.is_none()
            && (bytes[i] == b'`' || (bytes[i] == b'~' && bytes[i..].starts_with(b"~~~")))
        {
            let marker = bytes[i];
            let len = bytes[i..].iter().take_while(|&&b| b == marker).count();
            let mut end = i + len;
            let mut closed = false;
            while end < bytes.len() {
                if bytes[end] == marker {
                    let run = bytes[end..].iter().take_while(|&&b| b == marker).count();
                    if run == len {
                        end += run;
                        closed = true;
                        break;
                    }
                    end += run;
                } else {
                    end += 1;
                }
            }
            i = if closed || len >= 3 { end } else { i + len };
            continue;
        }
        if bytes[i] != b'<' {
            i += 1;
            continue;
        }
        if let Some(candidates) = candidates {
            while candidates
                .get(candidate_index)
                .is_some_and(|range| range.end <= i)
            {
                candidate_index += 1;
            }
            if !candidates
                .get(candidate_index)
                .is_some_and(|range| range.contains(&i))
            {
                i += 1;
                continue;
            }
        }
        let mut is_opaque = true;
        let end = if source[i..].starts_with("<![CDATA[") {
            Some(
                source[i + 9..]
                    .find("]]>")
                    .map_or(bytes.len(), |end| i + 9 + end + 3),
            )
        } else if source[i..].starts_with("<?") {
            Some(
                source[i + 2..]
                    .find("?>")
                    .map_or(bytes.len(), |end| i + 2 + end + 2),
            )
        } else if source[i..].starts_with("<!--") {
            Some(
                source[i + 4..]
                    .find("-->")
                    .map_or(bytes.len(), |end| i + 4 + end + 3),
            )
        } else if let Some((end, name, closing)) = tag(source, i) {
            if !closing && LITERAL.contains(&name.to_ascii_lowercase().as_str()) {
                let needle = format!("</{}", name.to_ascii_lowercase());
                let mut search = end;
                let mut close = None;
                while let Some(offset) = lower[search..].find(&needle) {
                    let at = search + offset;
                    if let Some((tag_end, close_name, true)) = tag(source, at)
                        && close_name.eq_ignore_ascii_case(name)
                    {
                        close = Some(tag_end);
                        break;
                    }
                    search = at + needle.len();
                }
                Some(close.unwrap_or(bytes.len()))
            } else {
                is_opaque = false;
                Some(end)
            }
        } else if source[i..].starts_with("<!") || source[i..].starts_with("<?") {
            Some(source[i..].find('>').map_or(bytes.len(), |end| i + end + 1))
        } else {
            None
        };
        if let Some(end) = end {
            if start < i {
                ranges.push(start..i);
            }
            if is_opaque {
                opaque.push(i..end);
            }
            i = end;
            start = end;
        } else {
            i += 1;
        }
    }
    if start < bytes.len() {
        ranges.push(start..bytes.len());
    }
    (ranges, opaque)
}
