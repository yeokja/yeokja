//! Audit and repair of stored translations against their source's tags.
//!
//! A translation made before inline tags writes the Markdown itself, and some
//! of it no longer reads as the source's markup: a shortcut reference whose
//! text was translated (`[team repo]` → `[팀 저장소]`) is dead text, an
//! italic came back bold. The audit counts the constructs the translation
//! reads back as against the source's; where some are missing it reads the
//! translation again against the source's tags and lets the serializer write
//! it back. Nothing that already reads right is touched. See
//! `docs/superpowers/specs/2026-09-22-markup-audit-design.md`.

use super::markdown::{
    DocContext, Position, Tag, TagKind, Tagged, code_span_ranges, normalize_label, read, render, structure,
    tagify_lenient,
};
use crate::evaluator_format::commonmark_emphasis;
use std::collections::{HashMap, HashSet};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Audit {
    /// The translation reads back with every construct of the source.
    Sound,
    /// It did not; `markdown` does. `defects` says what was wrong.
    Repaired { markdown: String, defects: Vec<String> },
    /// It did not, and no repair could be confirmed.
    Unrepairable { defects: Vec<String>, reasons: Vec<String> },
}

/// Audit a stored `translation` of the segment `tagged` was made from.
pub fn audit(translation: &str, tagged: &Tagged, ctx: &DocContext) -> Audit {
    if tagged.position == Position::Plain {
        return Audit::Sound;
    }
    let defects = defects(translation, tagged, ctx);
    if defects.is_empty() {
        return Audit::Sound;
    }
    match reread(translation, tagged, ctx) {
        Ok(markdown) => {
            let left = self::defects(&markdown, tagged, ctx);
            if left.is_empty() {
                Audit::Repaired { markdown, defects }
            } else {
                Audit::Unrepairable { defects, reasons: left }
            }
        }
        Err(reasons) => Audit::Unrepairable { defects, reasons },
    }
}

/// How many of each construct `shape` (see `structure`) holds: emphasis and
/// links by kind, the opaque ones by letter. Code is left to the code rules,
/// which allow a translation to shed a plural.
fn constructs(shape: &str) -> HashMap<String, usize> {
    let mut counts: HashMap<String, usize> = HashMap::new();
    let chars: Vec<char> = shape.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        match chars[i] {
            '(' => {
                if let Some(kind) = chars.get(i + 1) {
                    *counts.entry(format!("({kind}")).or_default() += 1;
                }
                i += 2;
            }
            'c' | 'r' if chars.get(i + 1) == Some(&':') => {
                while i < chars.len() && chars[i] != '|' {
                    i += 1;
                }
                i += 1;
            }
            ')' => i += 1,
            c => {
                *counts.entry(c.to_string()).or_default() += 1;
                i += 1;
            }
        }
    }
    counts
}

fn construct_name(key: &str) -> &str {
    match key {
        "(i" => "italic",
        "(b" => "bold",
        "(s" => "strikethrough",
        "(a" => "link",
        "h" => "inline HTML",
        "f" => "footnote reference",
        "g" => "image",
        "u" => "autolink",
        "e" => "expression",
        "n" => "line break",
        other => other,
    }
}

/// What the source has that `markdown` lost.
fn defects(markdown: &str, tagged: &Tagged, ctx: &DocContext) -> Vec<String> {
    let want = constructs(&structure(&tagged.source, tagged, ctx).0);
    let got = constructs(&structure(markdown, tagged, ctx).0);
    let mut found: Vec<String> = want
        .iter()
        .filter(|(key, n)| got.get(*key).copied().unwrap_or(0) < **n)
        .map(|(key, n)| format!("{} {} of {n}", construct_name(key), got.get(key).copied().unwrap_or(0)))
        .collect();
    let stray = |text: &str| commonmark_emphasis(text).stray.len();
    if stray(markdown) > stray(&tagged.source) {
        found.push("emphasis marks printed as text".to_string());
    }
    found.sort();
    found
}

/// A reference link's label, normalized; `None` for an inline link.
fn link_key(tag: &Tag) -> Option<String> {
    match &tag.label {
        Some(label) => Some(normalize_label(label)),
        None => tag.close.strip_prefix("][").and_then(|r| r.strip_suffix(']')).map(normalize_label),
    }
}

/// Whether translation tag `t` stands for source tag `s`.
fn same_construct(t: &Tag, s: &Tag) -> bool {
    match (t.kind, s.kind) {
        (TagKind::Link, TagKind::Link) => match (link_key(t), link_key(s)) {
            (Some(a), Some(b)) => a == b,
            (None, None) => t.close == s.close,
            _ => false,
        },
        (TagKind::Role, TagKind::Role) => match (&t.role, &s.role) {
            (Some(a), Some(b)) => a.name == b.name && a.target == b.target,
            _ => false,
        },
        (TagKind::Html, TagKind::Html) | (TagKind::Opaque, TagKind::Opaque) | (TagKind::Math, TagKind::Math) => {
            t.open == s.open
        }
        _ => false,
    }
}

/// Read `translation` again as tag text in the source's numbering, check it
/// with [`read`] and write it back with [`render`], keeping the translation's
/// own code spans.
fn reread(translation: &str, tagged: &Tagged, ctx: &DocContext) -> Result<String, Vec<String>> {
    // Reading decodes character references and writing never makes one, so
    // `&nbsp;` would come back an invisible character.
    if translation.contains("&#") || translation.split('&').skip(1).any(|r| {
        let name = r.chars().take_while(char::is_ascii_alphanumeric).count();
        name > 0 && r[name..].starts_with(';')
    }) {
        return Err(vec!["the translation holds character references; repair it by hand".to_string()]);
    }
    let mut counter = 1_000_000;
    let t = tagify_lenient(translation, ctx, tagged.position, &mut counter).map_err(|u| vec![u.0])?;
    let mut map: HashMap<usize, usize> = HashMap::new();
    let mut used: HashSet<usize> = HashSet::new();

    for tt in t.tags.iter().filter(|tag| !tag.kind.is_emphasis() && tag.kind != TagKind::Code) {
        if let Some(s) = tagged.tags.iter().find(|s| !used.contains(&s.n) && same_construct(tt, s)) {
            map.insert(tt.n, s.n);
            used.insert(s.n);
        }
    }
    let t_em: Vec<&Tag> = t.tags.iter().filter(|tag| tag.kind.is_emphasis()).collect();
    let s_em: Vec<&Tag> = tagged.tags.iter().filter(|tag| tag.kind.is_emphasis()).collect();
    // Kind by kind in order first. What is left of the translation may be
    // the same emphasis in another kind (`*x*` came back `**x**`); that is
    // taken only when what is left of the source is of one kind, since Korean
    // word order can move a bold ahead of an italic.
    for kind in [TagKind::Italic, TagKind::Bold, TagKind::Strike] {
        let ts = t_em.iter().filter(|tag| tag.kind == kind);
        let ss = s_em.iter().filter(|tag| tag.kind == kind);
        for (tt, s) in ts.zip(ss) {
            map.insert(tt.n, s.n);
        }
    }
    let t_left: Vec<&&Tag> = t_em.iter().filter(|tag| !map.contains_key(&tag.n)).collect();
    let s_left: Vec<&&Tag> = s_em.iter().filter(|s| !map.values().any(|n| *n == s.n)).collect();
    if !t_left.is_empty() && t_left.len() == s_left.len() && s_left.iter().all(|s| s.kind == s_left[0].kind) {
        for (tt, s) in t_left.iter().zip(&s_left) {
            map.insert(tt.n, s.n);
        }
    }
    let added: Vec<String> = t
        .tags
        .iter()
        .filter(|tag| tag.kind != TagKind::Code && !map.contains_key(&tag.n))
        .map(|tag| format!("the translation has {} the source does not", describe(tag)))
        .collect();
    if !added.is_empty() {
        return Err(added);
    }

    let mut reply = renumber(&t.text, &map, tagged);
    let dead: Vec<&Tag> = tagged
        .tags
        .iter()
        .filter(|s| s.kind == TagKind::Link && link_key(s).is_some() && !map.values().any(|n| *n == s.n))
        .collect();
    if !dead.is_empty() {
        reply = revive_references(&reply, &dead, tagged)?;
    }

    // Marks the translation left printed are its emphasis: never escape them
    // into text for good.
    if commonmark_emphasis(translation).stray.len() > commonmark_emphasis(&tagged.source).stray.len() {
        let lost: Vec<&Tag> = s_em.iter().copied().filter(|s| !map.values().any(|n| *n == s.n)).collect();
        reply = revive_emphasis(&reply, &lost, &s_em)?;
    }

    let mut tree = read(&reply, tagged)?;
    // Keep the code the translation wrote: the audit repairs markup only.
    for node in &mut tree.nodes {
        if let super::markdown::Node::Code { tag, .. } = node {
            *tag = None;
        }
    }
    let rendered = render(&tree, tagged, ctx);
    if !rendered.verified {
        return Err(vec![format!("the rewritten Markdown does not read back as meant: {}", rendered.markdown)]);
    }
    Ok(rendered.markdown)
}

fn describe(tag: &Tag) -> String {
    match tag.kind {
        TagKind::Italic => "an italic".into(),
        TagKind::Bold => "a bold".into(),
        TagKind::Strike => "a strikethrough".into(),
        TagKind::Link | TagKind::Role => format!("a link ({})", tag.close.trim_start_matches(']')),
        _ => format!("`{}`", tag.open),
    }
}

/// `text` with every tag renamed by `map` into the source's numbering.
fn renumber(text: &str, map: &HashMap<usize, usize>, tagged: &Tagged) -> String {
    let kinds: HashMap<usize, char> = tagged.tags.iter().map(|t| (t.n, t.kind.letter())).collect();
    let mut out = String::new();
    let mut rest = text;
    while let Some(at) = rest.find('<') {
        out.push_str(&rest[..at]);
        rest = &rest[at..];
        let end = rest.find('>').map_or(rest.len(), |e| e + 1);
        let token = &rest[..end];
        let closing = token.starts_with("</");
        let void = token.ends_with("/>");
        let digits: String = token.chars().filter(char::is_ascii_digit).collect();
        match digits.parse::<usize>().ok().and_then(|n| map.get(&n)) {
            Some(n) => {
                let letter = kinds[n];
                out.push_str(&match (closing, void) {
                    (true, _) => format!("</{letter}{n}>"),
                    (_, true) => format!("<{letter}{n}/>"),
                    _ => format!("<{letter}{n}>"),
                });
            }
            None => out.push_str(token),
        }
        rest = &rest[end..];
    }
    out.push_str(rest);
    out
}

/// Byte ranges and texts of the bracketed text in tag text `text` that did
/// not form a link: `[x]`, or `[x][y]` whose label the document does not
/// define. Code spans are skipped.
fn bracketed(text: &str) -> Vec<(std::ops::Range<usize>, String)> {
    let code = code_span_ranges(text);
    let b = text.as_bytes();
    let mut found = Vec::new();
    let mut i = 0;
    while i < b.len() {
        if let Some(span) = code.iter().find(|r| r.start == i) {
            i = span.end;
            continue;
        }
        if b[i] == b'[' && (i == 0 || b[i - 1] != b'\\') {
            let close = text[i + 1..].find([']', '[']).map(|at| i + 1 + at);
            if let Some(close) = close
                && b[close] == b']'
            {
                let mut end = close + 1;
                if b.get(end) == Some(&b'[')
                    && let Some(second) = text[end + 1..].find([']', '[']).map(|at| end + 1 + at)
                    && b[second] == b']'
                {
                    end = second + 1;
                }
                // `[!NOTE]` is an alert marker, never a link.
                if close > i + 1 && b[i + 1] != b'!' {
                    found.push((i..end, text[i + 1..close].to_string()));
                    i = end;
                    continue;
                }
            }
        }
        i += 1;
    }
    found
}

/// Turn emphasis marks the translation left printed (`**외적(outer
/// product)**에서` never closes) back into the source's emphasis: the runs
/// pair up in order, one pair for each lost emphasis. With none lost, the
/// translation split an emphasis in two and printed one half; a pair of `**`
/// is then the source's one bold, a pair of `*` its one italic.
fn revive_emphasis(reply: &str, lost: &[&Tag], all: &[&Tag]) -> Result<String, Vec<String>> {
    let code = code_span_ranges(reply);
    let chars: Vec<(usize, char)> = reply.char_indices().collect();
    let mut runs: Vec<(std::ops::Range<usize>, char)> = Vec::new();
    let mut in_tag = false;
    let mut i = 0;
    while i < chars.len() {
        let (at, c) = chars[i];
        if code.iter().any(|r| r.contains(&at)) {
            i += 1;
            continue;
        }
        match c {
            '<' => in_tag = true,
            '>' => in_tag = false,
            '*' | '_' if !in_tag => {
                let start = i;
                while chars.get(i).is_some_and(|(_, d)| *d == c) {
                    i += 1;
                }
                let before = start.checked_sub(1).map(|b| chars[b].1);
                let after = chars.get(i).map(|(_, d)| *d);
                let space = |x: Option<char>| x.is_none_or(char::is_whitespace);
                let intraword = c == '_' && before.is_some_and(char::is_alphanumeric) && after.is_some_and(char::is_alphanumeric);
                if !(space(before) && space(after)) && !intraword {
                    let end = chars.get(i).map_or(reply.len(), |(b, _)| *b);
                    runs.push((at..end, c));
                }
                continue;
            }
            _ => {}
        }
        i += 1;
    }
    let unpaired = runs.len() % 2 == 1
        || runs.chunks(2).any(|pair| pair[0].1 != pair[1].1 || pair[0].0.len() != pair[1].0.len());
    let owners: Option<Vec<&Tag>> = if unpaired {
        None
    } else if runs.len() == lost.len() * 2 {
        Some(lost.to_vec())
    } else if lost.is_empty() {
        runs.chunks(2)
            .map(|pair| {
                let kind = if pair[0].0.len() == 2 { TagKind::Bold } else { TagKind::Italic };
                let mut of_kind = all.iter().filter(|t| t.kind == kind);
                match (of_kind.next(), of_kind.next()) {
                    (Some(only), None) => Some(*only),
                    _ => None,
                }
            })
            .collect()
    } else {
        None
    };
    let Some(owners) = owners else {
        return Err(vec![format!("{} emphasis lost and {} printed mark(s) to take them back", lost.len(), runs.len())]);
    };
    let mut out = String::new();
    let mut last = 0;
    for (k, (range, _)) in runs.iter().enumerate() {
        let tag = owners[k / 2];
        out.push_str(&reply[last..range.start]);
        let letter = tag.kind.letter();
        out.push_str(&if k % 2 == 0 { format!("<{letter}{}>", tag.n) } else { format!("</{letter}{}>", tag.n) });
        last = range.end;
    }
    out.push_str(&reply[last..]);
    Ok(out)
}

/// Lowercase ASCII words of two letters or more: what a translated label
/// usually keeps of its source (`[keybase.io에서 확인할 수 있는]`).
fn ascii_words(text: &str) -> HashSet<String> {
    text.split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|w| w.len() >= 2)
        .map(str::to_ascii_lowercase)
        .collect()
}

/// Turn the dead bracketed text of the translation back into the source's
/// reference links. One of each: by position. Several: by the words each
/// kept of its source, when that pairs them one way only.
fn revive_references(reply: &str, dead: &[&Tag], tagged: &Tagged) -> Result<String, Vec<String>> {
    let literal: HashSet<String> = bracketed(&tagged.text).into_iter().map(|(_, t)| t).collect();
    let candidates: Vec<_> = bracketed(reply).into_iter().filter(|(_, t)| !literal.contains(t)).collect();
    if candidates.len() != dead.len() {
        return Err(vec![format!(
            "{} reference link(s) lost and {} bracketed text(s) to take them back",
            dead.len(),
            candidates.len()
        )]);
    }
    let order: Vec<usize> = if dead.len() == 1 {
        vec![0]
    } else {
        let label_words: Vec<HashSet<String>> =
            dead.iter().map(|d| ascii_words(&format!("{} {}", d.label.as_deref().unwrap_or(""), d.close))).collect();
        let text_words: Vec<HashSet<String>> = candidates.iter().map(|(_, t)| ascii_words(t)).collect();
        let score = |c: usize, d: usize| text_words[c].intersection(&label_words[d]).count();
        // The pairing that keeps the most words, when no other keeps as
        // many: one label that kept `AST` settles the other one too.
        let mut best: Vec<(usize, Vec<usize>)> = permutations(dead.len())
            .into_iter()
            .map(|perm| (perm.iter().enumerate().map(|(c, d)| score(c, *d)).sum(), perm))
            .collect();
        best.sort_by_key(|b| std::cmp::Reverse(b.0));
        match best.as_slice() {
            [first, second, ..] if first.0 > 0 && first.0 > second.0 => first.1.clone(),
            _ => return Err(vec!["several reference links lost and their order is unclear".to_string()]),
        }
    };
    let mut out = String::new();
    let mut last = 0;
    for (c, (range, content)) in candidates.iter().enumerate() {
        let tag = dead[order[c]];
        let n = tag.n;
        out.push_str(&reply[last..range.start]);
        out.push_str(&format!("<a{n}>{content}</a{n}>"));
        last = range.end;
        // `[beta로 백포트](backported to beta)`: the label went where a
        // destination would, and the link it meant is the reference.
        if let Some(inner) = reply[last..].strip_prefix('(').and_then(|r| r.split_once(')')).map(|(inner, _)| inner)
            && Some(normalize_label(inner)) == link_key(tag)
        {
            last += inner.len() + 2;
        }
    }
    out.push_str(&reply[last..]);
    Ok(out)
}

fn permutations(n: usize) -> Vec<Vec<usize>> {
    if n > 6 {
        return Vec::new();
    }
    let mut all: Vec<Vec<usize>> = vec![Vec::new()];
    for _ in 0..n {
        let mut next = Vec::new();
        for p in &all {
            for i in (0..n).filter(|i| !p.contains(i)) {
                let mut q = p.clone();
                q.push(i);
                next.push(q);
            }
        }
        all = next;
    }
    all
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::inline::markdown::{Dialect, tagify};

    fn check(source: &str, translation: &str, doc: &str) -> Audit {
        let ctx = DocContext::new(doc, Dialect::CommonMark);
        let mut counter = 0;
        let tagged = tagify(source, &ctx, Position::Inline, &mut counter).unwrap();
        audit(translation, &tagged, &ctx)
    }

    fn repaired(audit: Audit) -> String {
        match audit {
            Audit::Repaired { markdown, .. } => markdown,
            other => panic!("{other:?}"),
        }
    }

    const REFS: &str = "\n\n[team repo]: https://t\n[Rust signing key]: https://k\n[available on keybase.io]: https://b\n[GPG]: https://g\n";

    #[test]
    fn a_sound_translation_is_left_alone() {
        assert_eq!(check("See the [team repo] and **this**.", "[team repo]와 **이것**을 보십시오.", REFS), Audit::Sound);
        // A split bold and a shed plural are no defect.
        assert_eq!(check("Set **`b` in `c`**.", "`c`에서 **`b` 값**을 **설정**합니다.", ""), Audit::Sound);
        assert_eq!(check("Remove the `Makefiles` now.", "`Makefile`을 지금 제거하십시오.", ""), Audit::Sound);
    }

    #[test]
    fn half_of_a_split_emphasis_left_printed_is_closed() {
        assert_eq!(
            repaired(check("Set **`b` in `c`**.", "`c`에서 **`b`**를 **설정**합니다.", "")),
            "`c`에서 **`b`를** **설정**합니다."
        );
    }

    #[test]
    fn a_translated_shortcut_reference_is_revived() {
        assert_eq!(
            repaired(check("See the [team repo] now.", "지금 [팀 저장소]를 보십시오.", REFS)),
            "지금 [팀 저장소][team repo]를 보십시오."
        );
    }

    #[test]
    fn several_dead_references_pair_by_the_words_they_kept() {
        assert_eq!(
            repaired(check(
                "Signed with the [Rust signing key], which is [available on keybase.io], with [GPG].",
                "[GPG]를 사용하여 [keybase.io에서 확인할 수 있는] [Rust 서명 키]로 서명됩니다.",
                REFS
            )),
            "[GPG]를 사용하여 [keybase.io에서 확인할 수 있는][available on keybase.io] [Rust 서명 키][Rust signing key]로 서명됩니다."
        );
        // One kept word settles both.
        assert_eq!(
            repaired(check("See [team repo] and [GPG].", "[지피지]와 [팀 repo]를 보십시오.", REFS)),
            "[지피지][GPG]와 [팀 repo][team repo]를 보십시오."
        );
        // Nothing to tell them apart: left for a person.
        assert!(matches!(
            check("See [team repo] and [GPG].", "[팀 저장소]와 [지피지]를 보십시오.", REFS),
            Audit::Unrepairable { .. }
        ));
    }

    #[test]
    fn a_label_written_as_a_destination_goes_back_to_the_reference() {
        let doc = "\n\n[backported to beta]: https://b\n";
        assert_eq!(
            repaired(check("Tracks changes [backported to beta].", "[beta로 백포트](backported to beta)된 변경 사항을 추적합니다.", doc)),
            "[beta로 백포트][backported to beta]된 변경 사항을 추적합니다."
        );
    }

    #[test]
    fn literal_brackets_of_the_source_are_not_taken_for_links() {
        assert_eq!(
            repaired(check("Use [x, y] with the [team repo].", "[x, y]를 [팀 저장소]와 씁니다.", REFS)),
            "[x, y]를 [팀 저장소][team repo]와 씁니다."
        );
    }

    #[test]
    fn an_emphasis_of_another_kind_is_turned_back() {
        assert_eq!(
            repaired(check("**NOTE**: This is for *reviewing* PRs.", "**참고**: 이것은 PR을 **리뷰**하기 위한 것입니다.", "")),
            "**참고**: 이것은 PR을 *리뷰*하기 위한 것입니다."
        );
    }

    #[test]
    fn an_emphasis_left_open_before_a_particle_is_closed() {
        assert_eq!(
            repaired(check("From the **outer product** here.", "여기의 **외적(outer product)**에서 옵니다.", "")),
            "여기의 **외적**(outer product)에서 옵니다."
        );
    }

    #[test]
    fn a_reordered_kind_flip_keeps_each_emphasis_with_its_words() {
        assert_eq!(
            repaired(check("This is *only* for **nightly**.", "**나이틀리**에서 **오직** 쓰입니다.", "")),
            "**나이틀리**에서 *오직* 쓰입니다."
        );
    }

    #[test]
    fn a_bracket_holding_tags_and_a_gloss_is_revived() {
        let doc = "\n\n[*type inference*]: https://t\n";
        assert_eq!(
            repaired(check("[*type inference*] (the process)", "[*타입 추론*](표현식의 타입을 정하는 과정)", doc)),
            "[*타입 추론*][*type inference*](표현식의 타입을 정하는 과정)"
        );
    }

    #[test]
    fn a_translation_with_character_references_is_left_alone() {
        assert!(matches!(
            check("See the [team repo] now.", "지금&nbsp;[팀 저장소]를 보십시오.", REFS),
            Audit::Unrepairable { .. }
        ));
    }

    #[test]
    fn added_or_lost_markup_is_left_for_a_person() {
        assert!(matches!(check("Plain *word* here.", "여기 평범한 낱말입니다.", ""), Audit::Unrepairable { .. }));
        assert!(matches!(
            check("See [team repo].", "**중요**: [팀 저장소]를 보십시오.", REFS),
            Audit::Unrepairable { .. }
        ));
    }
}
