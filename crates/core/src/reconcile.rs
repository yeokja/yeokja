use crate::change::{compute_status, SegmentStatus};
use crate::glossary::Glossary;
use crate::hash::context_hash;
use crate::model::{Document, Segment};
use crate::state::{SegmentState, StateFile};
use std::collections::HashMap;

/// Report produced by [`reconcile_accepting_context`]: a parser-migration
/// reconcile that, unlike [`reconcile_with_status`], folds `ContextChanged`
/// into the saved state instead of leaving it for `translate` to redo.
pub struct ReconcileReport {
    pub segments: Vec<SegmentState>,
    /// Segments carried over from the existing state because their source
    /// text (hash) matched, whether or not their status changed.
    pub kept: usize,
    /// Source text of segments with no counterpart in the existing state.
    pub added: Vec<String>,
    /// Source text of existing segments with no counterpart in the new document.
    pub dropped: Vec<String>,
    /// Kept segments whose only difference was `ContextChanged`; their
    /// `context_hash` was updated to the current value.
    pub context_accepted: usize,
    /// Kept segments left `GlossaryStale` — untouched, since a parser
    /// reshape does not change the glossary.
    pub glossary_stale: usize,
}

pub struct ReconcileResult {
    pub segments: Vec<SegmentState>,
}

pub struct ReconciledSegment {
    pub state: SegmentState,
    pub status: SegmentStatus,
    pub context_hash: u64,
}

/// Reconcile a document against existing state and compute status for each segment.
pub fn reconcile_with_status(
    document: &Document,
    existing: &StateFile,
    glossary: &Glossary,
) -> Vec<ReconciledSegment> {
    let result = reconcile(document, existing);
    let segments = document.translatable_segments();
    let hashes: Vec<u64> = segments.iter().map(|s| s.source_hash).collect();

    result
        .segments
        .into_iter()
        .enumerate()
        .map(|(i, seg_state)| {
            let prev = if i > 0 { Some(hashes[i - 1]) } else { None };
            let next = hashes.get(i + 1).copied();
            let ctx = context_hash(prev, next);
            let status = compute_status(&seg_state, seg_state.source_hash, ctx, glossary);
            ReconciledSegment {
                state: seg_state,
                status,
                context_hash: ctx,
            }
        })
        .collect()
}

pub fn reconcile(document: &Document, existing: &StateFile) -> ReconcileResult {
    let new_segments = document.translatable_segments();
    tracing::trace!(segments = new_segments.len(), existing = existing.segments.len(), "Reconciling");
    let new_with_context: Vec<(&Segment, u64)> = {
        let hashes: Vec<u64> = new_segments.iter().map(|s| s.source_hash).collect();
        new_segments
            .iter()
            .enumerate()
            .map(|(i, seg)| {
                let prev = if i > 0 { Some(hashes[i - 1]) } else { None };
                let next = hashes.get(i + 1).copied();
                let ctx = context_hash(prev, next);
                (*seg, ctx)
            })
            .collect()
    };

    let mut existing_by_hash: HashMap<u64, Vec<&SegmentState>> = HashMap::new();
    for seg in &existing.segments {
        existing_by_hash.entry(seg.source_hash).or_default().push(seg);
    }

    let mut matched_existing: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut result_segments = Vec::new();

    for (new_seg, ctx_hash) in &new_with_context {
        let matched = existing_by_hash
            .get(&new_seg.source_hash)
            .and_then(|candidates| {
                let new_idx = new_seg.id.flat_index();
                candidates
                    .iter()
                    .filter(|c| !matched_existing.contains(&c.id.0))
                    .min_by_key(|c| {
                        let old_idx = c.id.flat_index();
                        (new_idx as isize - old_idx as isize).unsigned_abs()
                    })
            });

        match matched {
            Some(old) => {
                matched_existing.insert(old.id.0.clone());
                result_segments.push(SegmentState {
                    id: new_seg.id.clone(),
                    source: new_seg.source.clone(),
                    source_hash: new_seg.source_hash,
                    // Preserve the old context_hash so compute_status can detect ContextChanged
                    context_hash: old.context_hash,
                    translation: old.translation.clone(),
                    glossary_snapshot: old.glossary_snapshot.clone(),
                    translated_at: old.translated_at,
                    issues: old.issues.clone(),
                });
            }
            None => {
                result_segments.push(SegmentState {
                    id: new_seg.id.clone(),
                    source: new_seg.source.clone(),
                    source_hash: new_seg.source_hash,
                    context_hash: *ctx_hash,
                    translation: None,
                    glossary_snapshot: HashMap::new(),
                    translated_at: None,
                    issues: Vec::new(),
                });
            }
        }
    }

    ReconcileResult { segments: result_segments }
}

/// Reconcile a document against existing state for a parser migration: same
/// rule as [`reconcile`] (hash-match, nearest position wins ties). There is no
/// id fallback — a parser reshape can change a segment's text or shift ids
/// onto an unrelated segment, so a source text with no hash match goes in as
/// Pending (listed in `added`) and every unmatched old segment is `dropped`.
///
/// Only `ContextChanged` segments get their `context_hash` moved to the
/// current value here — `GlossaryStale` is left untouched, since a parser
/// reshape does not change the glossary.
pub fn reconcile_accepting_context(
    document: &Document,
    existing: &StateFile,
    glossary: &Glossary,
) -> ReconcileReport {
    let new_segments = document.translatable_segments();
    let hashes: Vec<u64> = new_segments.iter().map(|s| s.source_hash).collect();
    let new_with_context: Vec<(&Segment, u64)> = new_segments
        .iter()
        .enumerate()
        .map(|(i, seg)| {
            let prev = if i > 0 { Some(hashes[i - 1]) } else { None };
            let next = hashes.get(i + 1).copied();
            (*seg, context_hash(prev, next))
        })
        .collect();

    let mut existing_by_hash: HashMap<u64, Vec<&SegmentState>> = HashMap::new();
    for seg in &existing.segments {
        existing_by_hash.entry(seg.source_hash).or_default().push(seg);
    }

    let mut matched_existing: std::collections::HashSet<String> = std::collections::HashSet::new();

    let mut by_hash_match: Vec<Option<&SegmentState>> = Vec::with_capacity(new_with_context.len());
    for (new_seg, _) in &new_with_context {
        let matched = existing_by_hash.get(&new_seg.source_hash).and_then(|candidates| {
            let new_idx = new_seg.id.flat_index();
            candidates
                .iter()
                .filter(|c| !matched_existing.contains(&c.id.0))
                .min_by_key(|c| {
                    let old_idx = c.id.flat_index();
                    (new_idx as isize - old_idx as isize).unsigned_abs()
                })
                .copied()
        });
        if let Some(old) = matched {
            matched_existing.insert(old.id.0.clone());
        }
        by_hash_match.push(matched);
    }

    let mut result_segments = Vec::new();
    let mut kept = 0usize;
    let mut added = Vec::new();
    let mut context_accepted = 0usize;
    let mut glossary_stale = 0usize;

    for ((new_seg, ctx_hash), by_hash) in new_with_context.iter().zip(by_hash_match.iter()) {
        match by_hash {
            Some(old) => {
                kept += 1;
                let status = compute_status(old, new_seg.source_hash, *ctx_hash, glossary);
                let (source_hash, context_hash_val) = match status {
                    SegmentStatus::ContextChanged => {
                        context_accepted += 1;
                        (new_seg.source_hash, *ctx_hash)
                    }
                    SegmentStatus::GlossaryStale => {
                        glossary_stale += 1;
                        (old.source_hash, old.context_hash)
                    }
                    // Stale cannot occur: the hashes matched.
                    SegmentStatus::Stale | SegmentStatus::Translated | SegmentStatus::Pending => {
                        (old.source_hash, old.context_hash)
                    }
                };
                result_segments.push(SegmentState {
                    id: new_seg.id.clone(),
                    source: new_seg.source.clone(),
                    source_hash,
                    context_hash: context_hash_val,
                    translation: old.translation.clone(),
                    glossary_snapshot: old.glossary_snapshot.clone(),
                    translated_at: old.translated_at,
                    issues: old.issues.clone(),
                });
            }
            None => {
                added.push(new_seg.source.clone());
                result_segments.push(SegmentState {
                    id: new_seg.id.clone(),
                    source: new_seg.source.clone(),
                    source_hash: new_seg.source_hash,
                    context_hash: *ctx_hash,
                    translation: None,
                    glossary_snapshot: HashMap::new(),
                    translated_at: None,
                    issues: Vec::new(),
                });
            }
        }
    }

    let dropped: Vec<String> = existing
        .segments
        .iter()
        .filter(|s| !matched_existing.contains(&s.id.0))
        .map(|s| s.source.clone())
        .collect();

    ReconcileReport {
        segments: result_segments,
        kept,
        added,
        dropped,
        context_accepted,
        glossary_stale,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hash::content_hash;
    use crate::model::*;
    use chrono::Utc;

    fn make_segment(section: usize, block: usize, seg: usize, text: &str) -> Segment {
        Segment {
            id: SegmentId::new(section, block, seg),
            source: text.to_string(),
            source_hash: content_hash(text),
            block_type: BlockType::Paragraph,
        }
    }

    fn make_document(segments: Vec<Segment>) -> Document {
        Document {
            sections: vec![Section {
                blocks: vec![Block {
                    block_type: BlockType::Paragraph,
                    segments,
                    raw_content: String::new(),
                    heading_level: None,
                    span: None,
                    translatable: true,
                    role: BlockRole::None,
                }],
            }],
            source: String::new(),
        }
    }

    fn make_state(entries: Vec<(&str, &str)>) -> StateFile {
        let mut state = StateFile::new(0);
        for (i, (source, translation)) in entries.iter().enumerate() {
            state.segments.push(SegmentState {
                id: SegmentId::new(0, 0, i),
                source: source.to_string(),
                source_hash: content_hash(source),
                context_hash: 0,
                translation: Some(translation.to_string()),
                glossary_snapshot: HashMap::new(),
                translated_at: Some(Utc::now()),
                issues: Vec::new(),
            });
        }
        state
    }

    #[test]
    fn new_segments_get_no_translation() {
        let doc = make_document(vec![make_segment(0, 0, 0, "Hello.")]);
        let state = StateFile::new(0);
        let result = reconcile(&doc, &state);
        assert_eq!(result.segments.len(), 1);
        assert!(result.segments[0].translation.is_none());
    }

    #[test]
    fn unchanged_segments_keep_translation() {
        let doc = make_document(vec![make_segment(0, 0, 0, "Hello.")]);
        let state = make_state(vec![("Hello.", "안녕하세요.")]);
        let result = reconcile(&doc, &state);
        assert_eq!(result.segments[0].translation.as_deref(), Some("안녕하세요."));
    }

    #[test]
    fn moved_segment_keeps_translation() {
        let doc = make_document(vec![
            make_segment(0, 0, 0, "New sentence."),
            make_segment(0, 0, 1, "World."),
        ]);
        let state = make_state(vec![("World.", "세계.")]);
        let result = reconcile(&doc, &state);
        assert_eq!(result.segments.len(), 2);
        assert!(result.segments[0].translation.is_none());
        assert_eq!(result.segments[1].translation.as_deref(), Some("세계."));
    }

    #[test]
    fn deleted_segments_are_dropped() {
        let doc = make_document(vec![make_segment(0, 0, 0, "Hello.")]);
        let state = make_state(vec![("Hello.", "안녕."), ("Goodbye.", "잘가.")]);
        let result = reconcile(&doc, &state);
        assert_eq!(result.segments.len(), 1);
        assert_eq!(result.segments[0].source, "Hello.");
    }

    #[test]
    fn duplicate_segments_matched_by_position() {
        let doc = make_document(vec![
            make_segment(0, 0, 0, "Same."),
            make_segment(0, 0, 1, "Same."),
        ]);
        let state = make_state(vec![("Same.", "같다1."), ("Same.", "같다2.")]);
        let result = reconcile(&doc, &state);
        assert_eq!(result.segments.len(), 2);
        assert!(result.segments[0].translation.is_some());
        assert!(result.segments[1].translation.is_some());
        assert_ne!(result.segments[0].translation, result.segments[1].translation);
    }

    // Build a StateFile whose entries carry an explicit context_hash, so tests
    // can tell "kept as-is" apart from "updated to the current value".
    fn make_state_with_context(entries: Vec<(&str, &str, u64)>) -> StateFile {
        let mut state = StateFile::new(0);
        for (i, (source, translation, ctx)) in entries.iter().enumerate() {
            state.segments.push(SegmentState {
                id: SegmentId::new(0, 0, i),
                source: source.to_string(),
                source_hash: content_hash(source),
                context_hash: *ctx,
                translation: Some(translation.to_string()),
                glossary_snapshot: HashMap::new(),
                translated_at: Some(Utc::now()),
                issues: Vec::new(),
            });
        }
        state
    }

    #[test]
    fn accepting_context_carries_translations_across_a_parser_reshape() {
        // Old shape: A, B, C. New shape: D, A, C (B dropped, D inserted before A).
        // A and C keep their old translation but their neighbours changed, so
        // reconcile_accepting_context must fold that into their context_hash.
        let old_ctx = 111u64; // any stale placeholder distinct from the recomputed value
        let state = make_state_with_context(vec![
            ("A.", "번역A.", old_ctx),
            ("B.", "번역B.", old_ctx),
            ("C.", "번역C.", old_ctx),
        ]);
        let doc = make_document(vec![
            make_segment(0, 0, 0, "D."),
            make_segment(0, 0, 1, "A."),
            make_segment(0, 0, 2, "C."),
        ]);
        let glossary = Glossary::empty();

        let report = reconcile_accepting_context(&doc, &state, &glossary);

        assert_eq!(report.segments.len(), 3);

        let d = &report.segments[0];
        assert_eq!(d.source, "D.");
        assert!(d.translation.is_none());

        let a = &report.segments[1];
        assert_eq!(a.source, "A.");
        assert_eq!(a.translation.as_deref(), Some("번역A."));
        assert_ne!(a.context_hash, old_ctx, "A's context_hash must move to the current value");

        let c = &report.segments[2];
        assert_eq!(c.source, "C.");
        assert_eq!(c.translation.as_deref(), Some("번역C."));
        assert_ne!(c.context_hash, old_ctx, "C's context_hash must move to the current value");

        assert_eq!(report.context_accepted, 2);
        assert_eq!(report.kept, 2);
        assert_eq!(report.added, vec!["D.".to_string()]);
        assert_eq!(report.dropped, vec!["B.".to_string()]);
        assert_eq!(report.glossary_stale, 0);
    }

    #[test]
    fn accepting_context_adds_a_changed_source_and_drops_the_old_one() {
        // Same position/id, but the source text differs. A parser reshape can
        // change a segment's text (or shift ids onto an unrelated segment), so
        // reconcile must not pair them by id: the new text goes in as Pending
        // and shows up in `added`, the old one in `dropped`.
        let state = make_state_with_context(vec![("Old text.", "옛 번역.", 999)]);
        let doc = make_document(vec![make_segment(0, 0, 0, "New text.")]);
        let glossary = Glossary::empty();

        let report = reconcile_accepting_context(&doc, &state, &glossary);

        assert_eq!(report.segments.len(), 1);
        let seg = &report.segments[0];
        assert_eq!(seg.source, "New text.");
        assert!(seg.translation.is_none(), "the old translation must not be carried over by id");
        assert_eq!(seg.source_hash, content_hash("New text."));
        assert!(seg.translated_at.is_none());

        assert_eq!(report.kept, 0);
        assert_eq!(report.glossary_stale, 0);
        assert_eq!(report.context_accepted, 0);
        assert_eq!(report.added, vec!["New text.".to_string()]);
        assert_eq!(report.dropped, vec!["Old text.".to_string()]);
    }
}
