//! Hyperlink names across a reStructuredText document.
//!
//! A reference written with an embedded URI and one underscore,
//! `` `text <uri>`_ ``, also makes `text` a target name for the whole
//! document. Translated, two such links with the same visible text and
//! different URIs collide (docutils: "Duplicate explicit target name"), and a
//! plain reference `` `name`_ `` whose name the translation no longer defines
//! has no target ("Unknown target name"). Neither shows inside one segment —
//! both broke the PEP site after a retranslation — so the document is checked
//! as a whole after reconstruction.
//!
//! Duplicates are repaired: a later colliding link becomes anonymous
//! (`` `text <uri>`__ ``), which links the same way and defines no name.
//! References that lost their target are reported for a person to look at.

use std::collections::{HashMap, HashSet};

/// A name as docutils compares names: case and runs of whitespace ignored.
fn normalize(name: &str) -> String {
    name.split_whitespace().collect::<Vec<_>>().join(" ").to_lowercase()
}

/// `text` with inline literals (``…``) blanked out, so link syntax quoted as
/// code is not read as links.
fn without_literals(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(start) = rest.find("``") {
        out.push_str(&rest[..start]);
        let after = &rest[start + 2..];
        match after.find("``") {
            Some(end) => {
                out.push_str(&" ".repeat(end + 4));
                rest = &after[end + 2..];
            }
            None => {
                out.push_str(&rest[start..]);
                rest = "";
            }
        }
    }
    out.push_str(rest);
    out
}

/// One `` `text <target>`_ `` or `` `text <target>`__ `` in `text`.
#[derive(Debug, Clone, PartialEq)]
pub struct EmbeddedLink {
    /// Byte range of the whole construct in the scanned text.
    pub range: std::ops::Range<usize>,
    pub name: String,
    pub target: String,
    pub anonymous: bool,
}

/// Every embedded-target link in `text`, outside inline literals.
pub fn embedded_links(text: &str) -> Vec<EmbeddedLink> {
    let scan = without_literals(text);
    let bytes = scan.as_bytes();
    let mut links = Vec::new();
    let mut at = 0;
    while let Some(open) = scan[at..].find('`').map(|o| at + o) {
        let Some(close) = scan[open + 1..].find('`').map(|c| open + 1 + c) else { break };
        let inner = &scan[open + 1..close];
        let underscores = bytes[close + 1..].iter().take_while(|&&b| b == b'_').count();
        if (1..=2).contains(&underscores)
            && let Some(lt) = inner.rfind('<')
            && inner.ends_with('>')
            && !(open > 0 && bytes[open - 1] == b':')
        {
            let name = inner[..lt].trim();
            let target = inner[lt + 1..inner.len() - 1].trim();
            if !name.is_empty() && !target.is_empty() {
                links.push(EmbeddedLink {
                    range: open..close + 1 + underscores,
                    name: name.to_string(),
                    target: target.to_string(),
                    anonymous: underscores == 2,
                });
            }
        }
        at = close + 1;
    }
    links
}

/// Plain named references, `` `name`_ `` (not roles, not anonymous, not
/// embedded), and the names alias links point at (`` `text <name_>`_ ``).
pub fn references(text: &str) -> Vec<String> {
    let scan = without_literals(text);
    let bytes = scan.as_bytes();
    let mut names = Vec::new();
    let mut at = 0;
    while let Some(open) = scan[at..].find('`').map(|o| at + o) {
        let Some(close) = scan[open + 1..].find('`').map(|c| open + 1 + c) else { break };
        let inner = &scan[open + 1..close];
        let underscores = bytes[close + 1..].iter().take_while(|&&b| b == b'_').count();
        let role = open > 0 && bytes[open - 1] == b':';
        if !role && underscores == 1 && !inner.contains('<') {
            names.push(inner.trim().to_string());
        } else if !role
            && let Some(lt) = inner.rfind('<')
            && let Some(alias) = inner[lt + 1..].strip_suffix("_>")
        {
            names.push(alias.trim().to_string());
        }
        at = close + 1;
    }
    names
}

/// Names a document defines: explicit targets (`.. _name: uri`), section
/// titles, and named embedded-target links.
pub fn defined_names(document: &str) -> HashSet<String> {
    let mut names = HashSet::new();
    let lines: Vec<&str> = document.lines().collect();
    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim_start();
        if let Some(rest) = trimmed.strip_prefix(".. _")
            && let Some(colon) = rest.find(':')
        {
            names.insert(normalize(rest[..colon].trim_matches('`')));
        }
        let underline = lines.get(i + 1).map(|l| l.trim_end()).unwrap_or("");
        let title = line.trim();
        if !title.is_empty()
            && underline.len() >= title.chars().count().min(3)
            && underline.chars().all(|c| "=-~^\"'`#*+.:_".contains(c))
            && underline.chars().collect::<HashSet<_>>().len() == 1
            && !title.chars().all(|c| "=-~^\"'`#*+.:_".contains(c))
        {
            names.insert(normalize(title));
        }
    }
    for link in embedded_links(document) {
        if !link.anonymous && !link.target.ends_with('_') {
            names.insert(normalize(&link.name));
        }
    }
    names
}

/// Explicit target names and their URIs (`.. _name: uri`).
fn explicit_targets(document: &str) -> HashMap<String, String> {
    document
        .lines()
        .filter_map(|line| {
            let rest = line.trim_start().strip_prefix(".. _")?;
            let colon = rest.find(':')?;
            Some((normalize(rest[..colon].trim_matches('`')), rest[colon + 1..].trim().to_string()))
        })
        .collect()
}

/// Make later colliding named links anonymous, in document order.
///
/// `translations` are the segments' translations in document order, and
/// `source` the source document (its explicit targets keep their names in
/// the output). Returns the indices of the translations changed.
pub fn repair_duplicate_names(source: &str, translations: &mut [Option<String>]) -> Vec<usize> {
    let mut taken: HashMap<String, String> = explicit_targets(source);
    let mut changed = Vec::new();
    for (index, slot) in translations.iter_mut().enumerate() {
        let Some(text) = slot else { continue };
        let mut repaired = text.clone();
        // Edit from the end so earlier ranges stay valid.
        let mut edits = Vec::new();
        for link in embedded_links(text) {
            // `text <name_>`_ points at another target by name; it is not a
            // second definition with its own URI.
            if link.anonymous || link.target.ends_with('_') {
                continue;
            }
            let key = normalize(&link.name);
            match taken.get(&key) {
                Some(target) if target != &link.target => edits.push(link.range.end),
                Some(_) => {}
                None => {
                    taken.insert(key, link.target.clone());
                }
            }
        }
        for end in edits.into_iter().rev() {
            repaired.insert(end, '_');
        }
        if &repaired != text {
            *text = repaired;
            changed.push(index);
        }
    }
    changed
}

/// Reference names `output` uses but no longer defines, though `source`
/// defined them: the translation broke the link. Names undefined in the
/// source too (targets from included files, say) are not reported.
pub fn lost_targets(source: &str, output: &str) -> Vec<String> {
    let in_source = defined_names(source);
    let in_output = defined_names(output);
    let mut lost: Vec<String> = references(output)
        .into_iter()
        .filter(|name| {
            let key = normalize(name);
            !in_output.contains(&key) && in_source.contains(&key)
        })
        .collect();
    lost.sort();
    lost.dedup();
    lost
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_embedded_links_outside_literals() {
        let links = embedded_links("See `저장소 <https://a>`_ and `x <https://b>`__, not ```y <c>`_``.");
        assert_eq!(links.len(), 2);
        assert_eq!((links[0].name.as_str(), links[0].anonymous), ("저장소", false));
        assert!(links[1].anonymous);
        assert!(embedded_links(":pep:`PEP 8 <8>`").is_empty());
    }

    #[test]
    fn later_colliding_links_become_anonymous() {
        let mut translations = vec![
            Some("`git-cola <https://g>`_ (`저장소 <https://github.com/git-cola>`_)".to_string()),
            None,
            Some("`GitAlias <http://ga>`_ (`저장소 <https://github.com/GitAlias>`_)".to_string()),
            Some("`저장소 <https://github.com/git-cola>`_ again".to_string()),
        ];
        assert_eq!(repair_duplicate_names("", &mut translations), vec![2]);
        assert!(translations[2].as_deref().unwrap().contains("<https://github.com/GitAlias>`__)"));
        // An alias to a named target is not a definition.
        let mut aliases = vec![Some("`mypy <mypy_>`_".to_string())];
        assert!(repair_duplicate_names(".. _mypy: http://mypy-lang.org\n", &mut aliases).is_empty());
        // The same name and target twice is not a collision.
        assert!(translations[3].as_deref().unwrap().ends_with(">`_ again"));
    }

    #[test]
    fn explicit_targets_in_the_source_keep_their_names() {
        let source = ".. _저장소: https://x\n";
        let mut translations = vec![Some("`저장소 <https://y>`_".to_string())];
        assert_eq!(repair_duplicate_names(source, &mut translations), vec![0]);
    }

    #[test]
    fn reports_references_whose_target_the_translation_dropped() {
        let source = "Intro\n=====\n\nSee `Add an atfork module <http://b>`_ and `Add an atfork module`_.\n";
        let output = "소개\n====\n\n`atfork 모듈 추가 <http://b>`_ 및 `Add an atfork module`_ 참조.\n";
        assert_eq!(lost_targets(source, output), vec!["Add an atfork module".to_string()]);
        let fixed = "소개\n====\n\n`atfork 모듈 추가 <http://b>`_ 및 `atfork 모듈 추가`_ 참조.\n";
        assert!(lost_targets(source, fixed).is_empty());
        // A name the source never defined either (an include's target) is not reported.
        assert!(lost_targets("See `Elsewhere`_.\n", "`Elsewhere`_ 참조.\n").is_empty());
    }

    #[test]
    fn section_titles_are_names() {
        let names = defined_names("How to Teach This\n=================\n\ntext\n");
        assert!(names.contains("how to teach this"));
    }
}
