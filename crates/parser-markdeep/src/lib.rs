//! Markdeep's HTML documents contain Markdown with indented title matter,
//! labelled captions, and code fences whose language/highlight runs can change.
//! Mask those regions without changing byte offsets, then reuse Markdown spans.
use yeokja_core::hash::content_hash;
use yeokja_core::model::{BlockType, Document, Segment, SegmentId};
use yeokja_core::parser::{DocumentParser, Markup, TranslationMap};
use yeokja_parser_markdown_dialect::{Extra, parse_with};
use yeokja_parser_utils::{resolve_reference_links, splice_reconstruct};

pub struct MarkdeepParser;

fn blank(bytes: &mut [u8]) {
    for byte in bytes {
        if !matches!(*byte, b'\n' | b'\r') {
            *byte = b' ';
        }
    }
}

impl DocumentParser for MarkdeepParser {
    fn markup(&self) -> Markup {
        Markup::Markdown
    }

    fn parse(&self, source: &str) -> Document {
        let mut shadow = source.as_bytes().to_vec();
        let mut extras = Vec::new();
        let mut offset = 0;
        let mut code = false;
        let mut backtick_fence = 0;
        let mut equation = false;
        let mut consumed = 0;
        let mut list = false;
        let mut extra_list: Option<usize> = None;
        for line in source.split_inclusive('\n') {
            if offset < consumed {
                offset += line.len();
                continue;
            }
            let trimmed = line.trim();
            let start = offset + line.len() - line.trim_start().len();
            let end = start + trimmed.len();
            let fence_len = trimmed.bytes().take_while(|b| *b == b'~').count();
            let fence = fence_len >= 3;
            let ticks = trimmed.bytes().take_while(|b| *b == b'`').count();
            if backtick_fence > 0 || (!code && ticks >= 3) {
                if backtick_fence > 0 {
                    if ticks >= backtick_fence && trimmed[ticks..].trim().is_empty() {
                        backtick_fence = 0;
                    }
                } else {
                    backtick_fence = ticks;
                }
                blank(&mut shadow[offset..offset + line.len()]);
            } else if code || fence {
                // A labelled fence switches syntax/highlighting within a listing;
                // only the bare fence ends it, regardless of the fence length.
                code = !(code && fence && trimmed[fence_len..].trim().is_empty());
                blank(&mut shadow[offset..offset + line.len()]);
            } else if equation || trimmed.starts_with("$$") {
                let delimiters = trimmed.matches("$$").count();
                if delimiters % 2 == 1 {
                    equation = !equation;
                }
                blank(&mut shadow[offset..offset + line.len()]);
            } else if trimmed.starts_with("(insert ") && trimmed.ends_with(" here)") {
                blank(&mut shadow[offset..offset + line.len()]);
            } else if let Some((range, syntax_end, finish)) = caption(source, start) {
                extras.push(Extra {
                    range,
                    block_type: BlockType::Paragraph,
                });
                let trailing = &source[syntax_end..finish];
                let trailing_start = syntax_end + trailing.len() - trailing.trim_start().len();
                let trailing_end = syntax_end + trailing.trim_end().len();
                if trailing_start < trailing_end {
                    extras.push(Extra {
                        range: trailing_start..trailing_end,
                        block_type: BlockType::Paragraph,
                    });
                }
                blank(&mut shadow[offset..finish]);
                consumed = finish;
            } else {
                // Continuation lines belong to the same Markdown list paragraph.
                // Only standalone deeply indented Markdeep title matter is extra.
                if trimmed.is_empty() {
                    list = false;
                    extra_list = None;
                }
                let marker = trimmed.starts_with("- ")
                    || trimmed.starts_with("* ")
                    || trimmed.split_once(". ").is_some_and(|(n, _)| {
                        !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit())
                    });
                if marker {
                    list = true;
                }
                if list
                    && !marker
                    && start - offset >= 4
                    && !trimmed.is_empty()
                    && let Some(index) = extra_list
                {
                    extras[index].range.end = end;
                    blank(&mut shadow[offset..offset + line.len()]);
                }
                if start - offset >= 4
                    && !trimmed.is_empty()
                    && !trimmed.starts_with('<')
                    && (!list || marker)
                {
                    let text = if marker {
                        start + trimmed.find(' ').unwrap() + 1
                    } else {
                        start
                    };
                    extra_list = marker.then_some(extras.len());
                    extras.push(Extra {
                        range: text..end,
                        block_type: if marker {
                            BlockType::ListItem
                        } else {
                            BlockType::Paragraph
                        },
                    });
                    blank(&mut shadow[offset..offset + line.len()]);
                }
            }
            offset += line.len();
        }
        let mut document = parse_with(
            &String::from_utf8(shadow).expect("blanked UTF-8"),
            source,
            false,
            extras,
        );
        // Initials such as J.D. Foley are part of a sentence. Keep its markup
        // and author together so per-segment translation cannot move formatting
        // into a separate segment. This dialect correction leaves other parsers
        // and all unaffected source hashes/IDs unchanged.
        for block in document.sections.iter_mut().flat_map(|s| &mut s.blocks) {
            let mut joined: Vec<Segment> = Vec::new();
            for segment in std::mem::take(&mut block.segments) {
                if let Some(previous) = joined.last_mut()
                    && ends_in_initials(&previous.source)
                {
                    previous.source.push(' ');
                    previous.source.push_str(&segment.source);
                    previous.source_hash = content_hash(&previous.source);
                } else {
                    joined.push(segment);
                }
            }
            for (index, segment) in joined.iter_mut().enumerate() {
                if let Some((section, block, _)) = segment.id.position() {
                    segment.id = SegmentId::new(section, block, index);
                }
            }
            block.segments = joined;
        }
        document
    }

    fn reconstruct(&self, document: &Document, translations: &TranslationMap) -> String {
        splice_reconstruct(document, &resolve_reference_links(document, translations))
    }
}

/// Isolate caption prose from Markdeep labels, nested brackets, and image URLs.
/// Scan balanced brackets because filenames and inline references also use them.
fn caption(source: &str, start: usize) -> Option<(std::ops::Range<usize>, usize, usize)> {
    let rest = &source[start..];
    let image = rest.starts_with("![");
    if !image && !rest.starts_with("[Listing [") {
        return None;
    }
    let open = start + usize::from(image);
    let mut depth = 0;
    let mut close = None;
    for (i, ch) in source[open..].char_indices() {
        match ch {
            '[' => depth += 1,
            ']' => {
                depth -= 1;
                if depth == 0 {
                    close = Some(open + i);
                    break;
                }
            }
            _ => {}
        }
    }
    let close = close?;
    let mut text = open + 1;
    if source[text..].starts_with("Listing [") || source[text..].starts_with("Figure [") {
        text += source[text..close].find("]: ")? + 3;
    }
    for (opening, closing) in [("<kbd>", "</kbd>"), ("<span ", "</span>")] {
        if source[text..close].starts_with(opening) {
            text += source[text..close].find(closing)? + closing.len();
        }
    }
    text += source[text..close].len() - source[text..close].trim_start().len();
    let end = text + source[text..close].trim_end().len();
    let syntax_end = if image {
        let tail = source[close + 1..].strip_prefix('(')?;
        let mut depth = 1;
        let mut end = None;
        for (index, ch) in tail.char_indices() {
            if ch == '(' {
                depth += 1;
            }
            if ch == ')' {
                depth -= 1;
                if depth == 0 {
                    end = Some(close + 3 + index);
                    break;
                }
            }
        }
        end?
    } else {
        close + 1
    };
    let finish = syntax_end
        + source[syntax_end..]
            .find('\n')
            .map_or(source.len() - syntax_end, |n| n + 1);
    (text < end).then_some((text..end, syntax_end, finish))
}

fn ends_in_initials(text: &str) -> bool {
    let Some(token) = text.split_whitespace().next_back() else {
        return false;
    };
    let Some(initials) = token.strip_suffix('.') else {
        return false;
    };
    !initials.is_empty()
        && initials
            .split('.')
            .all(|part| part.len() == 1 && part.as_bytes()[0].is_ascii_uppercase())
}
