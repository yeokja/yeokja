//! Keeping CommonMark reference links resolvable after translation.
//!
//! A shortcut reference (`[standard structure]`) or a collapsed one
//! (`[nix command][]`) names its definition by its own text. The definition
//! (`[standard structure]: https://…`) lives elsewhere in the file and is not
//! part of any segment, so once the link text is translated the label no
//! longer matches and the brackets render literally. Before splicing, every
//! such reference in a translation is rewritten to the full form
//! `[translated text][original label]`, which keeps the definition reachable.

use std::collections::HashSet;
use std::ops::Range;
use yeokja_core::model::Document;
use yeokja_core::parser::TranslationMap;

/// The translations with reference links rewritten to their full form where
/// the source segment's references can be paired with the translation's.
pub fn resolve_reference_links(
    document: &Document,
    translations: &TranslationMap,
) -> TranslationMap {
    let labels = definition_labels(&document.source);
    if labels.is_empty() {
        return translations.clone();
    }
    let mut resolved = translations.clone();
    for section in &document.sections {
        for block in &section.blocks {
            if !block.translatable {
                continue;
            }
            for segment in &block.segments {
                let Some(translation) = translations.get(&segment.id) else {
                    continue;
                };
                if let Some(rewritten) = rewrite_references(&segment.source, translation, &labels) {
                    resolved.insert(segment.id.clone(), rewritten);
                }
            }
        }
    }
    resolved
}

/// The normalized labels of every link reference definition in `source`,
/// outside fenced code.
pub fn definition_labels(source: &str) -> HashSet<String> {
    let mut labels = HashSet::new();
    let mut fence: Option<(u8, usize)> = None;
    for line in source.lines() {
        let trimmed = line.trim_start();
        if let Some((marker, len)) = fence {
            let count = trimmed.bytes().take_while(|&b| b == marker).count();
            if count >= len && trimmed[count..].trim().is_empty() {
                fence = None;
            }
            continue;
        }
        if let Some(&marker) = trimmed.as_bytes().first()
            && matches!(marker, b'`' | b'~')
        {
            let len = trimmed.bytes().take_while(|&b| b == marker).count();
            if len >= 3 {
                fence = Some((marker, len));
                continue;
            }
        }
        if line.len() - trimmed.len() > 3 || !trimmed.starts_with('[') || trimmed.starts_with("[^")
        {
            continue;
        }
        let Some(close) = bracket_end(trimmed, 0) else {
            continue;
        };
        if trimmed[close..].starts_with("]:") {
            labels.insert(normalize_label(&trimmed[1..close]));
        }
    }
    labels
}

/// CommonMark label matching: case-folded, inner whitespace collapsed.
fn normalize_label(label: &str) -> String {
    label
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

/// A bracketed link reference found in inline text.
#[derive(Debug, PartialEq, Eq)]
struct Reference {
    /// The whole reference, brackets included.
    range: Range<usize>,
    /// The link text between the first pair of brackets.
    text: Range<usize>,
    /// The label it resolves by: the text itself for shortcut and collapsed
    /// references, the second bracket's content for a full one.
    label: String,
    full: bool,
}

/// Every reference-style link in `text`: shortcut, collapsed and full, but
/// not inline links, images, footnotes or anything inside a code span.
fn find_references(text: &str) -> Vec<Reference> {
    let bytes = text.as_bytes();
    let mut found = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'`' => i = skip_code_span(text, i),
            b'\\' => i += 2,
            b'[' => {
                if i > 0 && bytes[i - 1] == b'!' || bytes.get(i + 1) == Some(&b'^') {
                    i += 1;
                    continue;
                }
                let Some(close) = bracket_end(text, i) else {
                    i += 1;
                    continue;
                };
                let text_range = i + 1..close;
                let after = close + 1;
                match bytes.get(after) {
                    Some(b'(') => i = after,
                    Some(b'[') => match bracket_end(text, after) {
                        Some(label_close) if label_close == after + 1 => {
                            found.push(Reference {
                                range: i..label_close + 1,
                                text: text_range.clone(),
                                label: normalize_label(&text[text_range]),
                                full: false,
                            });
                            i = label_close + 1;
                        }
                        Some(label_close) => {
                            found.push(Reference {
                                range: i..label_close + 1,
                                text: text_range,
                                label: normalize_label(&text[after + 1..label_close]),
                                full: true,
                            });
                            i = label_close + 1;
                        }
                        None => i = after,
                    },
                    _ => {
                        found.push(Reference {
                            range: i..after,
                            text: text_range.clone(),
                            label: normalize_label(&text[text_range]),
                            full: false,
                        });
                        i = after;
                    }
                }
            }
            _ => i += 1,
        }
    }
    found
}

/// Offset of the `]` matching the `[` at `open`, skipping nested brackets,
/// escapes and code spans.
fn bracket_end(text: &str, open: usize) -> Option<usize> {
    let bytes = text.as_bytes();
    let mut depth = 0usize;
    let mut i = open;
    while i < bytes.len() {
        match bytes[i] {
            b'\\' => i += 1,
            b'`' => {
                i = skip_code_span(text, i);
                continue;
            }
            b'[' => depth += 1,
            b']' => {
                depth -= 1;
                if depth == 0 {
                    return Some(i);
                }
            }
            _ => {}
        }
        i += 1;
    }
    None
}

/// Offset just past the code span opening at `start`, or past the backtick
/// run when it never closes.
fn skip_code_span(text: &str, start: usize) -> usize {
    let bytes = text.as_bytes();
    let len = bytes[start..].iter().take_while(|&&b| b == b'`').count();
    let mut i = start + len;
    while i < bytes.len() {
        if bytes[i] == b'`' {
            let run = bytes[i..].iter().take_while(|&&b| b == b'`').count();
            if run == len {
                return i + run;
            }
            i += run;
        } else {
            i += 1;
        }
    }
    start + len
}

/// `translation` with each reference paired to the source's rewritten as
/// `[text][label]`, or `None` when nothing needs to change or the two sides
/// cannot be paired.
///
/// A translated reference that already resolves — a full one, or one whose
/// text is itself a label — is paired with the source reference of the same
/// label, wherever the translation moved it. The rest are paired in order,
/// and only when their counts agree.
fn rewrite_references(source: &str, translation: &str, labels: &HashSet<String>) -> Option<String> {
    let resolving: Vec<Reference> = find_references(source)
        .into_iter()
        .filter(|reference| labels.contains(&reference.label))
        .collect();
    if resolving.is_empty() {
        return None;
    }
    let translated = find_references(translation);
    let mut unpaired: Vec<Option<&Reference>> = resolving.iter().map(Some).collect();
    for reference in &translated {
        if !labels.contains(&reference.label) {
            continue;
        }
        if let Some(slot) = unpaired
            .iter_mut()
            .find(|slot| slot.is_some_and(|original| original.label == reference.label))
        {
            *slot = None;
        }
    }
    let unresolved: Vec<&Reference> = translated
        .iter()
        .filter(|reference| !labels.contains(&reference.label))
        .collect();
    let originals: Vec<&Reference> = unpaired.into_iter().flatten().collect();
    if unresolved.is_empty() || unresolved.len() != originals.len() {
        return None;
    }
    let mut out = String::with_capacity(translation.len());
    let mut pos = 0;
    for (original, reference) in originals.iter().zip(&unresolved) {
        out.push_str(&translation[pos..reference.range.start]);
        out.push('[');
        out.push_str(&translation[reference.text.clone()]);
        out.push_str("][");
        out.push_str(&source[original.text.clone()]);
        out.push(']');
        pos = reference.range.end;
    }
    out.push_str(&translation[pos..]);
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn labels(defs: &[&str]) -> HashSet<String> {
        defs.iter().map(|label| normalize_label(label)).collect()
    }

    #[test]
    fn shortcut_reference_is_expanded() {
        let labels = labels(&["standard structure"]);
        assert_eq!(
            rewrite_references(
                "A file with a [standard structure].",
                "[표준 구조]를 가진 파일.",
                &labels
            )
            .as_deref(),
            Some("[표준 구조][standard structure]를 가진 파일.")
        );
    }

    #[test]
    fn collapsed_reference_is_expanded() {
        let labels = labels(&["`nix` command"]);
        assert_eq!(
            rewrite_references(
                "Use the [`nix` command][].",
                "[`nix` 명령][]을 쓰세요.",
                &labels
            )
            .as_deref(),
            Some("[`nix` 명령][`nix` command]을 쓰세요.")
        );
    }

    #[test]
    fn inline_links_images_and_footnotes_are_not_references() {
        let labels = labels(&["docs"]);
        let source = "See [docs] and [inline](https://x) and ![docs](img.png) and [^1].";
        let translation = "[문서]와 [인라인](https://x)과 ![docs](img.png)과 [^1] 참고.";
        assert_eq!(
            rewrite_references(source, translation, &labels).as_deref(),
            Some("[문서][docs]와 [인라인](https://x)과 ![docs](img.png)과 [^1] 참고.")
        );
    }

    #[test]
    fn label_inside_code_span_is_untouched() {
        let labels = labels(&["docs"]);
        assert_eq!(
            rewrite_references(
                "Type `[docs]` literally.",
                "`[docs]`를 그대로 입력.",
                &labels
            ),
            None
        );
    }

    #[test]
    fn count_mismatch_leaves_translation_alone() {
        let labels = labels(&["a", "b"]);
        assert_eq!(rewrite_references("[a] and [b].", "[가]만.", &labels), None);
    }

    #[test]
    fn label_match_is_case_insensitive_and_unchanged_text_is_kept() {
        let labels = labels(&["Experimental"]);
        assert_eq!(
            rewrite_references(
                "Flakes are [experimental].",
                "플레이크는 [실험적]입니다.",
                &labels
            )
            .as_deref(),
            Some("플레이크는 [실험적][experimental]입니다.")
        );
        assert_eq!(
            rewrite_references(
                "Flakes are [experimental].",
                "플레이크는 [Experimental]입니다.",
                &labels
            ),
            None
        );
        assert_eq!(
            rewrite_references(
                "Flakes are [experimental].",
                "플레이크는 [실험적][experimental]입니다.",
                &labels
            ),
            None
        );
    }

    #[test]
    fn references_kept_verbatim_pair_by_label_even_when_reordered() {
        let labels = labels(&["`nix` command", "`flake.lock`"]);
        assert_eq!(
            rewrite_references(
                "Nix creates a [`flake.lock`] once you run a [`nix` command].",
                "[`nix` 명령]을 실행하면 Nix가 [`flake.lock`]을 생성합니다.",
                &labels
            )
            .as_deref(),
            Some("[`nix` 명령][`nix` command]을 실행하면 Nix가 [`flake.lock`]을 생성합니다.")
        );
    }

    #[test]
    fn definitions_are_collected_outside_code_fences() {
        let source = "Text.\n\n[Nix 2.4]: https://a\n  [`nix` command]: https://b\n```\n[not one]: https://c\n```\n[^note]: footnote\n";
        let labels = definition_labels(source);
        assert_eq!(
            labels,
            HashSet::from(["nix 2.4".to_string(), "`nix` command".to_string()])
        );
    }
}
