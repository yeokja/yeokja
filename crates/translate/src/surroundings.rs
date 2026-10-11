//! What surrounds a request: the headings it sits under and the blocks just
//! before and after it, with their existing translations.
//!
//! A request carries the source of its own blocks, but a lone answer
//! ("Almost!") needs the question in the heading above it, a list item needs
//! the sentence that introduced the list, and a sentence retranslated on its
//! own needs its neighbours' wording to stay in step with them. The context
//! is read-only: nothing in it is translated.
//!
//! See `docs/superpowers/specs/2026-10-11-request-context-design.md`.

use std::collections::{HashMap, HashSet};
use yeokja_core::model::{BlockType, Document};
use yeokja_core::state::SegmentState;

/// Characters of source, and of translation, shown per item.
const SHOWN: usize = 300;

struct Entry {
    source: String,
    /// The block's translation when every segment has a current one.
    translation: Option<String>,
    /// Indices of the headings above this block, outermost first.
    headings: Vec<usize>,
}

/// The translatable blocks of one document in order, for describing what
/// surrounds any group of them.
pub struct Surroundings {
    entries: Vec<Entry>,
    by_segment: HashMap<String, usize>,
}

fn shorten(text: &str) -> String {
    let flat = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if flat.chars().count() <= SHOWN {
        flat
    } else {
        format!("{}…", flat.chars().take(SHOWN).collect::<String>())
    }
}

impl Surroundings {
    /// `states` follows `Document::translatable_segments`. A saved
    /// translation counts only while its source text is the current one.
    pub fn new(doc: &Document, states: &[SegmentState]) -> Self {
        let mut entries = Vec::new();
        let mut by_segment = HashMap::new();
        let mut stack: Vec<(u8, usize)> = Vec::new();
        let mut flat = 0usize;
        for block in doc.sections.iter().flat_map(|section| &section.blocks) {
            if !block.translatable {
                continue;
            }
            let index = entries.len();
            let mut parts = Vec::new();
            let mut complete = !block.segments.is_empty();
            for segment in &block.segments {
                by_segment.insert(segment.id.0.clone(), index);
                match states.get(flat) {
                    Some(state) if state.source_hash == segment.source_hash && state.translation.is_some() => {
                        parts.push(state.translation.clone().unwrap_or_default());
                    }
                    _ => complete = false,
                }
                flat += 1;
            }
            let is_heading = block.block_type == BlockType::Heading;
            let level = block.heading_level.unwrap_or(1);
            if is_heading {
                while stack.last().is_some_and(|(top, _)| *top >= level) {
                    stack.pop();
                }
            }
            entries.push(Entry {
                source: block.segments.iter().map(|s| s.source.as_str()).collect::<Vec<_>>().join(" "),
                translation: complete.then(|| parts.join(" ")),
                headings: stack.iter().map(|(_, at)| *at).collect(),
            });
            if is_heading {
                stack.push((level, index));
            }
        }
        Self { entries, by_segment }
    }

    fn line(&self, label: &str, index: usize) -> String {
        let entry = &self.entries[index];
        match &entry.translation {
            Some(translation) => format!("{label}: {} (translated as: {})\n", shorten(&entry.source), shorten(translation)),
            None => format!("{label}: {}\n", shorten(&entry.source)),
        }
    }

    /// The context for a request made of `blocks` (each block's segment IDs,
    /// in document order), or an empty string when there is none.
    pub fn describe(&self, blocks: &[Vec<String>]) -> String {
        let positions: Vec<usize> =
            blocks.iter().filter_map(|ids| ids.first().and_then(|id| self.by_segment.get(id)).copied()).collect();
        let (Some(&first), Some(&last)) = (positions.iter().min(), positions.iter().max()) else {
            return String::new();
        };
        let inside: HashSet<usize> = positions.iter().copied().collect();
        let mut out = String::new();
        for &heading in &self.entries[first].headings {
            if !inside.contains(&heading) {
                out.push_str(&self.line("Under heading", heading));
            }
        }
        if first > 0 && !inside.contains(&(first - 1)) {
            out.push_str(&self.line("Before", first - 1));
        }
        if last + 1 < self.entries.len() && !inside.contains(&(last + 1)) {
            out.push_str(&self.line("After", last + 1));
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use yeokja_core::model::{Block, BlockRole, Section, Segment, SegmentId};

    fn block(section: usize, index: usize, kind: BlockType, level: Option<u8>, texts: &[&str]) -> Block {
        Block {
            block_type: kind,
            segments: texts
                .iter()
                .enumerate()
                .map(|(i, t)| Segment {
                    id: SegmentId::new(section, index, i),
                    source: t.to_string(),
                    source_hash: yeokja_core::hash::content_hash(t),
                    block_type: kind,
                })
                .collect(),
            raw_content: texts.join(" "),
            heading_level: level,
            span: None,
            role: BlockRole::None,
            translatable: true,
        }
    }

    fn state(segment: &Segment, translation: Option<&str>) -> SegmentState {
        SegmentState {
            id: segment.id.clone(),
            source: segment.source.clone(),
            source_hash: segment.source_hash,
            context_hash: 0,
            translation: translation.map(String::from),
            glossary_snapshot: HashMap::new(),
            translated_at: None,
            issues: vec![],
        }
    }

    fn doc() -> Document {
        Document {
            sections: vec![
                Section { blocks: vec![block(0, 0, BlockType::Heading, Some(1), &["FAQ"])] },
                Section {
                    blocks: vec![
                        block(1, 0, BlockType::Heading, Some(2), &["Is PyPy a drop in replacement for CPython?"]),
                        block(1, 1, BlockType::Paragraph, None, &["Almost!"]),
                        block(1, 2, BlockType::Paragraph, None, &["Most modules work.", "Some do not."]),
                    ],
                },
            ],
            source: String::new(),
        }
    }

    #[test]
    fn describes_headings_and_neighbours_with_current_translations() {
        let doc = doc();
        let segments = doc.translatable_segments();
        let mut states: Vec<SegmentState> = segments.iter().map(|s| state(s, None)).collect();
        states[0].translation = Some("자주 묻는 질문".into());
        states[1].translation = Some("PyPy는 CPython을 그대로 대체할 수 있습니까?".into());
        states[3].translation = Some("대부분의 모듈은 동작합니다.".into());
        states[4].translation = Some("일부는 그렇지 않습니다.".into());
        let around = Surroundings::new(&doc, &states).describe(&[vec!["section:1/block:1/seg:0".into()]]);
        assert!(around.contains("Under heading: FAQ (translated as: 자주 묻는 질문)"));
        assert!(around.contains("Under heading: Is PyPy a drop in replacement for CPython? (translated as: PyPy는"));
        // The question heading is also the block right before.
        assert!(around.contains("Before: Is PyPy"));
        assert!(around.contains("After: Most modules work. Some do not. (translated as: 대부분의 모듈은 동작합니다. 일부는 그렇지 않습니다.)"));
    }

    #[test]
    fn a_stale_or_partial_translation_is_not_shown() {
        let doc = doc();
        let segments = doc.translatable_segments();
        let mut states: Vec<SegmentState> = segments.iter().map(|s| state(s, Some("번역"))).collect();
        states[4].translation = None;
        states[1].source_hash ^= 1;
        let around = Surroundings::new(&doc, &states).describe(&[vec!["section:1/block:1/seg:0".into()]]);
        assert!(around.contains("Under heading: Is PyPy a drop in replacement for CPython?\n"));
        assert!(around.contains("After: Most modules work. Some do not.\n"));
    }

    #[test]
    fn a_request_covering_its_neighbours_shows_nothing_more() {
        let doc = doc();
        let segments = doc.translatable_segments();
        let states: Vec<SegmentState> = segments.iter().map(|s| state(s, None)).collect();
        let all: Vec<Vec<String>> = vec![
            vec!["section:0/block:0/seg:0".into()],
            vec!["section:1/block:0/seg:0".into()],
            vec!["section:1/block:1/seg:0".into()],
            vec!["section:1/block:2/seg:0".into(), "section:1/block:2/seg:1".into()],
        ];
        assert_eq!(Surroundings::new(&doc, &states).describe(&all), "");
    }
}
