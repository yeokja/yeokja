//! Detects translations stored under the wrong segment number.
//!
//! Segments are translated in numbered batches, and the answer is filed under
//! whatever `[N]` the model wrote. When the model slips by one, every later
//! segment receives its neighbour's translation, and no per-segment evaluator
//! notices: each translation is well-formed on its own.
//!
//! A Korean translation keeps numbers, Latin-script names and identifiers,
//! inline code, URLs and math exactly as the source writes them. So when a
//! translation carries such an anchor that its own source lacks but another
//! segment of the same batch has, the translation may belong to that other
//! segment.
//!
//! Not every such anchor is a slip. Korean puts the verb last, so a sentence
//! split across two segments (by display math, a line break, a comment)
//! often has part of the second half translated into the first; that is a
//! *move*, and the neighbour's translation still carries the rest of its own
//! source. A slip leaves the neighbour with nothing of its own, and a
//! duplicate writes the same content in both places.

use std::collections::{BTreeSet, HashMap};
use yeokja_core::parser::Markup;

/// A translation that carries anchors from another segment's source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Misalignment {
    /// The segment whose translation looks foreign.
    pub idx: usize,
    /// The segment whose source those anchors come from.
    pub from: usize,
    pub anchors: Vec<String>,
}

/// Verbatim tokens a translation may keep from this source, lowercased:
/// Latin words of two or more letters, numbers, inline code, URLs and math.
///
/// A source is read generously — every form a faithful translation might
/// write its content in — so that a translation's anchor is foreign only when
/// no reading of the source explains it: "three" and "March" also give "3",
/// "31st" gives "31", "250 MHz" gives "250mhz", "int-to-float" gives its
/// parts as well as the compound, "stashing" gives "stash", a word may be set
/// as code, and a code span gives its words.
pub fn anchors(text: &str, markup: Markup) -> BTreeSet<String> {
    tokens(text, markup, Side::Source)
}

/// The anchors a translation actually writes, read literally.
fn written_anchors(text: &str, markup: Markup) -> BTreeSet<String> {
    tokens(text, markup, Side::Translation)
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Side {
    Source,
    Translation,
}

#[derive(Clone, Copy)]
struct Syntax {
    side: Side,
    /// Backticks delimit code; in LaTeX they are opening quotes.
    code: bool,
    /// `$` delimits math, rather than being a literal dollar sign.
    math: bool,
}

fn tokens(text: &str, markup: Markup, side: Side) -> BTreeSet<String> {
    let chars: Vec<char> = text.chars().collect();
    let syntax = Syntax {
        side,
        code: !matches!(markup, Markup::Latex),
        math: matches!(markup, Markup::Latex | Markup::MkDocs | Markup::Verso),
    };
    let mut found = BTreeSet::new();
    collect(&chars, syntax, &mut found);
    found
}

fn collect(chars: &[char], syntax: Syntax, found: &mut BTreeSet<String>) {
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        // A command name or an escaped character: markup, not content
        // (`\textit`, `\'e`, RST's `\ `).
        if c == '\\' {
            i += 1;
            let command = chars[i..].iter().take_while(|ch| ch.is_ascii_alphabetic()).count();
            i += command.max(1);
            continue;
        }
        if (c == '`' && syntax.code) || (c == '$' && syntax.math) {
            let run = run_length(chars, i);
            let Some(close) = closing_run(chars, i + run, c, run) else {
                i += run;
                continue;
            };
            let body = &chars[i + run..close];
            let inner: String = body.iter().collect::<String>().trim().to_string();
            // A span with spaces is prose (RST link text, say) unless it is math.
            if c == '$' || !inner.contains(char::is_whitespace) {
                if !inner.is_empty() {
                    let inner = if c == '`' { code_name(&inner) } else { inner };
                    found.insert(format!("{c}{inner}{c}"));
                }
                if c == '`' && syntax.side == Side::Source {
                    collect(body, syntax, found);
                }
            } else {
                collect(body, syntax, found);
            }
            i = close + run;
            continue;
        }
        if c.is_ascii_alphanumeric() {
            i = word(chars, i, syntax.side, found);
            continue;
        }
        i += 1;
    }
}

/// Reads the word, number or URL starting at `start`; returns where it ends.
fn word(chars: &[char], start: usize, side: Side, found: &mut BTreeSet<String>) -> usize {
    let mut i = start;
    let mut parts: Vec<String> = vec![String::new()];
    while i < chars.len() {
        let ch = chars[i];
        if ch.is_ascii_alphanumeric() {
            parts.last_mut().expect("parts is never empty").push(ch);
        } else if ch == '.' && chars.get(i + 1).is_some_and(|n| n.is_ascii_digit()) {
            parts.last_mut().expect("parts is never empty").push('.');
        } else if ch == '-'
            && chars[i - 1].is_ascii_alphabetic()
            && chars.get(i + 1).is_some_and(|n| n.is_ascii_alphabetic())
        {
            parts.push(String::new());
        } else {
            break;
        }
        i += 1;
    }
    if chars[i..].starts_with(&[':', '/', '/']) {
        let end = chars[start..]
            .iter()
            .position(|ch| {
                ch.is_whitespace() || !ch.is_ascii() || matches!(ch, ')' | '>' | '<' | ']' | '{' | '}' | '`' | '"' | '\'')
            })
            .map_or(chars.len(), |n| start + n);
        let url: String = chars[start..end].iter().collect();
        found.insert(url.trim_end_matches(['.', ',', ';', ':', '!', '?']).to_lowercase());
        return end;
    }

    let compound: String = parts.concat().to_lowercase();
    insert_word(&compound, found);
    if side == Side::Source {
        // The translation may set a word from the source as code.
        found.insert(format!("`{}`", code_name(&compound)));
        if parts.len() > 1 {
            for part in &parts {
                insert_word(&part.to_lowercase(), found);
            }
        }
        for part in &parts {
            let part = part.to_lowercase();
            if let Some(n) = number_word(&part) {
                found.insert(n.to_string());
            }
            // A glossary term keeps its base form: "stashing" is "stash",
            // "fulfillment" is "fulfill", "evaluation" is "evaluate".
            for (suffix, ending) in [("ing", ""), ("ed", ""), ("ment", ""), ("ation", "ate")] {
                if let Some(stem) = part.strip_suffix(suffix).filter(|stem| stem.len() >= 3) {
                    insert_word(&format!("{stem}{ending}"), found);
                }
            }
            // "31st", "3d", "1.5TB": the number on its own.
            let digits: String = part.chars().take_while(|c| c.is_ascii_digit() || *c == '.').collect();
            if !digits.is_empty() && digits.len() < part.len() {
                insert_word(&digits, found);
            }
        }
        // "250 MHz" may be written "250MHz".
        if compound.chars().all(|c| c.is_ascii_digit() || c == '.') && chars.get(i) == Some(&' ') {
            let unit: String = chars[i + 1..].iter().take_while(|c| c.is_ascii_alphabetic()).collect();
            if (1..=3).contains(&unit.len()) {
                insert_word(&format!("{compound}{}", unit.to_lowercase()), found);
            }
        }
    }
    i
}

fn insert_word(token: &str, found: &mut BTreeSet<String>) {
    if token.chars().all(|c| c.is_ascii_digit()) && !token.is_empty() {
        // "06" and "6" are one number.
        let trimmed = token.trim_start_matches('0');
        found.insert(if trimmed.is_empty() { "0" } else { trimmed }.to_string());
    } else if token.len() >= 2 || token.chars().any(|c| c.is_ascii_digit()) {
        found.insert(fold_plural(token));
    }
}

/// Words a translation writes as a number.
fn number_word(word: &str) -> Option<u32> {
    const WORDS: &[(&str, u32)] = &[
        ("zero", 0), ("one", 1), ("two", 2), ("three", 3), ("four", 4), ("five", 5),
        ("six", 6), ("seven", 7), ("eight", 8), ("nine", 9), ("ten", 10), ("eleven", 11),
        ("twelve", 12), ("once", 1), ("twice", 2), ("first", 1), ("second", 2),
        ("third", 3), ("fourth", 4), ("fifth", 5), ("binary", 2), ("octal", 8),
        ("decimal", 10), ("hexadecimal", 16), ("january", 1), ("february", 2),
        ("march", 3), ("april", 4), ("may", 5), ("june", 6), ("july", 7), ("august", 8),
        ("september", 9), ("october", 10), ("november", 11), ("december", 12),
    ];
    WORDS.iter().find(|(w, _)| *w == word).map(|(_, n)| *n)
}

fn run_length(chars: &[char], at: usize) -> usize {
    chars[at..].iter().take_while(|&&ch| ch == chars[at]).count()
}

/// The start of the next run of exactly `len` `mark`s at or after `from`.
fn closing_run(chars: &[char], from: usize, mark: char, len: usize) -> Option<usize> {
    let mut i = from;
    while i < chars.len() {
        if chars[i] == '\\' && mark == '$' {
            i += 2;
            continue;
        }
        if chars[i] == mark {
            let run = run_length(chars, i);
            if run == len {
                return Some(i);
            }
            i += run;
            continue;
        }
        i += 1;
    }
    None
}

/// "slices" and "slice" are one anchor: the glossary keeps a hardware name in
/// the singular where the source may have written the plural.
fn fold_plural(word: &str) -> String {
    match word.strip_suffix('s') {
        Some(stem) if word.len() >= 4 && !stem.ends_with('s') && word.chars().all(|c| c.is_ascii_alphabetic()) => {
            stem.to_string()
        }
        _ => word.to_string(),
    }
}

/// A code span's content as an anchor: `tp_alloc()` and `tp_alloc` name one
/// function, and ``ParamSpecs`` is the plural of ``ParamSpec``.
fn code_name(inner: &str) -> String {
    fold_plural(&inner.strip_suffix("()").unwrap_or(inner).to_lowercase())
}

/// Whether consecutive segments are one sentence that markup split (display
/// math, an inline comment, a line of a list item): the first breaks off on
/// a lowercase word with no closing punctuation, or the second picks up with
/// a lowercase word. Korean word order moves content across such a split.
fn one_sentence(first: &str, second: &str) -> bool {
    let lowercase = |word: &str| !word.is_empty() && word.chars().all(|c| c.is_ascii_lowercase());
    let breaks_off = first.split_whitespace().last().is_some_and(|w| {
        lowercase(w.trim_start_matches(|c: char| !c.is_ascii_alphanumeric()))
    });
    let picks_up = second.split_whitespace().next().is_some_and(|w| {
        lowercase(w.trim_matches(|c: char| !c.is_ascii_alphanumeric()))
    });
    breaks_off || picks_up
}

/// Latin words the translation writes capitalised, lowercased.
fn names(translation: &str) -> BTreeSet<String> {
    let mut found = BTreeSet::new();
    let mut word = String::new();
    for ch in translation.chars().chain(std::iter::once(' ')) {
        if ch.is_ascii_alphabetic() {
            word.push(ch);
        } else {
            if word.starts_with(|c: char| c.is_ascii_uppercase()) {
                found.insert(fold_plural(&word.to_lowercase()));
            }
            word.clear();
        }
    }
    found
}

/// One anchor that proves the content came from elsewhere: inline code of
/// three or more characters, math beyond a single symbol, a URL, or a token
/// mixing letters and digits ("1.5TB", "x86").
fn is_strong(anchor: &str) -> bool {
    if let Some(inner) = anchor.strip_prefix('`').and_then(|a| a.strip_suffix('`')) {
        return inner.chars().count() > 2;
    }
    if let Some(inner) = anchor.strip_prefix('$').and_then(|a| a.strip_suffix('$')) {
        return !is_single_symbol(inner);
    }
    anchor.contains("://")
        || (anchor.chars().any(|c| c.is_ascii_digit()) && anchor.chars().any(|c| c.is_ascii_alphabetic()))
}

/// `$I$`, `$X'$`, `$\alpha$`, `$N_m$`: a symbol a translation often restates
/// to fit Korean word order.
fn is_single_symbol(math: &str) -> bool {
    let math = math.trim();
    let rest = if let Some(command) = math.strip_prefix('\\') {
        let len = command.chars().take_while(|c| c.is_ascii_alphabetic()).count();
        if len == 0 {
            return false;
        }
        &command[len..]
    } else {
        let mut chars = math.chars();
        match chars.next() {
            Some(c) if c.is_ascii_alphanumeric() => chars.as_str(),
            _ => return false,
        }
    };
    let rest = rest.trim_matches('\'');
    rest.is_empty()
        || rest
            .strip_prefix('_')
            .is_some_and(|s| s.chars().count() == 1 && s.chars().all(|c| c.is_ascii_alphanumeric()))
}

/// Enough shared anchors to say a translation carries another segment's
/// content: one strong anchor, or several words or numbers that are not
/// merely a restated name.
fn is_evidence(shared: &[String], translation: &str, markup: Markup) -> bool {
    shared.iter().any(|a| is_strong(a)) || (shared.len() >= 2 && !is_restated_name(shared, translation, markup))
}

/// Translations name a subject the source left implicit ("It" becomes
/// "Argument Clinic", "Zero to Nix"): capitalised words and short function
/// words forming one phrase, which is not content from elsewhere.
fn is_restated_name(shared: &[String], translation: &str, markup: Markup) -> bool {
    let names = names(translation);
    let name_like = shared
        .iter()
        .all(|a| names.contains(a) || (a.len() <= 3 && a.chars().all(|c| c.is_ascii_alphabetic())));
    name_like
        && translation.split(|c: char| !c.is_ascii()).any(|run| {
            let phrase = written_anchors(run, markup);
            shared.iter().all(|a| phrase.contains(a))
        })
}

/// Translations in `translations` that carry anchors absent from their own
/// source but present in another source of the same `batch`.
///
/// `paragraphs` maps a segment to the paragraph it belongs to (indices left
/// out belong to none). Content may move between segments of one paragraph,
/// or between neighbours, when the other segment's translation still carries
/// the rest of its own source; it may not appear twice, nor leave the other
/// segment with nothing of its own.
pub fn misaligned(
    batch: &[(usize, String)],
    translations: &HashMap<usize, String>,
    paragraphs: &HashMap<usize, String>,
    markup: Markup,
) -> Vec<Misalignment> {
    let texts: HashMap<usize, &str> = batch.iter().map(|(idx, source)| (*idx, source.as_str())).collect();
    let sources: HashMap<usize, BTreeSet<String>> =
        batch.iter().map(|(idx, source)| (*idx, anchors(source, markup))).collect();
    let written: HashMap<usize, BTreeSet<String>> =
        translations.iter().map(|(idx, text)| (*idx, written_anchors(text, markup))).collect();

    let mut found = Vec::new();
    for (idx, _) in batch {
        let (Some(own), Some(mine)) = (sources.get(idx), written.get(idx)) else { continue };
        let mut foreign: BTreeSet<String> = mine.difference(own).cloned().collect();
        if foreign.is_empty() {
            continue;
        }
        // Content that moved here from a neighbour, rather than being copied,
        // is word order; the neighbour's translation must still be its own:
        // it kept anchors of its own, or it is the rest of this sentence
        // ("As of" | "November 2024, …" becomes "2024년 11월" | "기준, …").
        for (other, _) in batch {
            let adjacent = idx.abs_diff(*other) == 1;
            let same_paragraph =
                matches!((paragraphs.get(idx), paragraphs.get(other)), (Some(a), Some(b)) if a == b);
            let Some(rendered) = written.get(other).filter(|_| other != idx && (adjacent || same_paragraph)) else {
                continue;
            };
            let theirs = &sources[other];
            let moved: Vec<&String> = foreign.intersection(theirs).collect();
            let copied = moved.iter().filter(|a| rendered.contains(**a)).count();
            let kept_own = theirs.iter().any(|a| !moved.contains(&a) && rendered.contains(a));
            let (first, second) = if idx < other { (idx, other) } else { (other, idx) };
            let split_sentence = adjacent && one_sentence(texts[first], texts[second]);
            if !moved.is_empty() && copied * 2 <= moved.len() && (kept_own || split_sentence) {
                let moved: Vec<String> = moved.into_iter().cloned().collect();
                foreign.retain(|a| !moved.contains(a));
            }
        }
        let best = batch
            .iter()
            .filter(|(other, _)| other != idx)
            .map(|(other, _)| (*other, foreign.intersection(&sources[other]).cloned().collect::<Vec<_>>()))
            .filter(|(_, shared)| is_evidence(shared, &translations[idx], markup))
            .max_by_key(|(_, shared)| shared.len());
        if let Some((from, shared)) = best {
            found.push(Misalignment { idx: *idx, from, anchors: shared });
        }
    }
    found
}

/// Stored translations in document order that look like a neighbour's.
///
/// Batches were contiguous runs of segments, so a slipped batch leaves each
/// translation next to the source it belongs to. Each segment is compared
/// with the `radius` segments on either side; `paragraphs` labels each entry
/// with its paragraph. Returns `(position, from)` pairs of positions in
/// `entries` together with the shared anchors.
pub fn misaligned_in_sequence(
    entries: &[(String, String)],
    paragraphs: &[String],
    markup: Markup,
    radius: usize,
) -> Vec<(usize, usize, Vec<String>)> {
    let labels: HashMap<usize, String> = paragraphs.iter().cloned().enumerate().collect();
    let mut found = Vec::new();
    for position in 0..entries.len() {
        let window = position.saturating_sub(radius)..(position + radius + 1).min(entries.len());
        let batch: Vec<(usize, String)> = window.clone().map(|i| (i, entries[i].0.clone())).collect();
        let translations: HashMap<usize, String> = window.map(|i| (i, entries[i].1.clone())).collect();
        if let Some(m) = misaligned(&batch, &translations, &labels, markup).into_iter().find(|m| m.idx == position) {
            found.push((m.idx, m.from, m.anchors));
        }
    }
    found
}

#[cfg(test)]
mod tests {
    use super::*;

    fn batch(sources: &[&str]) -> Vec<(usize, String)> {
        sources.iter().enumerate().map(|(i, s)| (i + 1, s.to_string())).collect()
    }

    fn answers(translations: &[&str]) -> HashMap<usize, String> {
        translations.iter().enumerate().map(|(i, t)| (i + 1, t.to_string())).collect()
    }

    fn check(markup: Markup, sources: &[&str], translations: &[&str]) -> Vec<(usize, usize)> {
        misaligned(&batch(sources), &answers(translations), &HashMap::new(), markup)
            .iter()
            .map(|m| (m.idx, m.from))
            .collect()
    }

    #[test]
    fn a_shifted_batch_is_detected_by_numbers_and_names() {
        // The furiosa-opt failure: "Tier" received the next row's sentence.
        let sources = batch(&["Tier", "1.5TB/s per chip over HBM.", "Quick Start"]);
        let shifted = answers(&["칩당 HBM 1.5TB/s입니다.", "빠른 시작", "빠른 시작"]);
        let found = misaligned(&sources, &shifted, &HashMap::new(), Markup::Markdown);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].idx, 1);
        assert_eq!(found[0].from, 2);
        assert!(found[0].anchors.contains(&"hbm".to_string()));
        assert!(found[0].anchors.contains(&"1.5tb".to_string()));
    }

    #[test]
    fn an_aligned_batch_passes() {
        let found = check(
            Markup::Markdown,
            &["Run `cargo build` first.", "The Tensor Unit reads 32 bytes.", "See https://example.com/docs for more."],
            &["먼저 `cargo build`를 실행합니다.", "Tensor Unit은 32바이트를 읽습니다.", "자세한 내용은 https://example.com/docs를 참고하십시오."],
        );
        assert!(found.is_empty(), "{found:?}");
    }

    #[test]
    fn markup_commands_and_added_glosses_are_not_anchors() {
        let found = check(
            Markup::Latex,
            &["The \\emph{reduction} step.", "Use \\emph{recursion} here."],
            &["\\emph{리덕션}(reduction) 단계입니다.", "여기서는 \\emph{재귀}를 씁니다."],
        );
        assert!(found.is_empty(), "{found:?}");
    }

    #[test]
    fn inline_code_and_math_count_as_anchors() {
        let found = check(
            Markup::MkDocs,
            &["Call `fetch()` now.", "Then $x^2$ grows."],
            &["그다음 $x^2$가 커집니다.", "그다음 $x^2$가 커집니다."],
        );
        assert_eq!(found, [(1, 2)]);
    }

    #[test]
    fn a_single_shared_word_or_digit_is_not_evidence() {
        // The glossary keeps "Slice" in English where the source said "slices";
        // a model writes "2" for "two". Neither means the numbering slipped.
        let found = check(
            Markup::Markdown,
            &["Data moves across slices twice.", "Each slice has 2 lanes."],
            &["데이터는 Slice 사이를 2번 이동합니다.", "각 slice에는 lane이 2개 있습니다."],
        );
        assert!(found.is_empty(), "{found:?}");
    }

    #[test]
    fn stored_translations_are_audited_against_their_neighbours() {
        let entries: Vec<(String, String)> = [
            ("Intro.", "소개입니다."),
            ("Tier", "칩당 HBM 1.5TB/s입니다."),
            ("1.5TB/s per chip over HBM.", "칩당 HBM 1.5TB/s입니다."),
            ("Done.", "끝입니다."),
        ]
        .iter()
        .map(|(s, t)| (s.to_string(), t.to_string()))
        .collect();
        let paragraphs: Vec<String> = (0..entries.len()).map(|i| format!("p{i}")).collect();
        let found = misaligned_in_sequence(&entries, &paragraphs, Markup::Markdown, 3);
        assert_eq!(found.iter().map(|(p, f, _)| (*p, *f)).collect::<Vec<_>>(), [(1, 2)]);
    }

    #[test]
    fn numbers_written_as_words_match_their_digits() {
        // "December" becomes "12월", "zero or one" becomes "0 또는 1".
        let found = check(
            Markup::Markdown,
            &["(8 December 2023) Hungarian Algorithm", "(12 July 2023) Planar faces, 0 or 1 of them."],
            &["(2023년 12월 8일) Hungarian Algorithm", "(2023년 7월 12일) Planar faces, 0개 또는 1개."],
        );
        assert!(found.is_empty(), "{found:?}");
        let found = check(
            Markup::Markdown,
            &["A bit is set if it is one and cleared if it is zero, from the 6th to the 9th.", "Only 0 and 1, 6-9 July."],
            &["비트가 1이면 설정, 0이면 해제된 것이며 6일부터 9일까지입니다.", "0과 1만, 7월 6-9일."],
        );
        assert!(found.is_empty(), "{found:?}");
    }

    #[test]
    fn a_sentence_duplicated_from_another_paragraph_is_caught() {
        // cp-algorithms: the next paragraph's heading was also glued on here.
        let found = check(
            Markup::Markdown,
            &[
                "Find the smallest number greater or equal to a specified number.",
                "We will improve the time complexity using the technique \"fractional cascading\".",
            ],
            &[
                "지정된 수보다 크거나 같은 가장 작은 수를 찾으십시오. \"fractional cascading\"을 이용한 가속화입니다.",
                "\"fractional cascading\"이라는 기법으로 시간 복잡도를 개선합니다.",
            ],
        );
        assert_eq!(found, [(1, 2)]);
    }

    #[test]
    fn a_sentence_duplicated_within_its_paragraph_is_caught() {
        // rustc-dev-guide: the list item's second sentence was written twice.
        let sources = batch(&[
            "Enum: Needed for support for enum types.",
            "The Rust compiler writes the information about enum into DWARF, and GDB reads the DWARF.",
        ]);
        let doubled = answers(&[
            "Enum: 열거형 타입 지원을 위해 필요합니다. Rust 컴파일러는 enum 정보를 DWARF에 기록하고, GDB는 DWARF를 읽습니다.",
            "Rust 컴파일러는 enum 정보를 DWARF에 기록하고, GDB는 DWARF를 읽습니다.",
        ]);
        let paragraphs: HashMap<usize, String> = [(1, "p".to_string()), (2, "p".to_string())].into();
        let found = misaligned(&sources, &doubled, &paragraphs, Markup::Markdown);
        assert_eq!(found.iter().map(|m| (m.idx, m.from)).collect::<Vec<_>>(), [(1, 2)]);
    }

    #[test]
    fn content_moved_into_a_neighbour_is_word_order() {
        // A sentence split across segments; Korean puts the verb last, so the
        // first half's translation takes the name from the second.
        let found = check(
            Markup::Rst,
            &[
                "06-May-2007 - Updated by Tim Delaney to reflect discussions on the python-3000",
                "and python-dev mailing lists.",
            ],
            &["2007년 5월 6일 - python-3000", "및 python-dev 메일링 리스트에서의 논의를 반영하도록 Tim Delaney가 업데이트했습니다."],
        );
        assert!(found.is_empty(), "{found:?}");
    }

    #[test]
    fn a_neighbour_left_with_nothing_of_its_own_is_a_slip() {
        let found = check(
            Markup::Markdown,
            &["Tier", "Bandwidth is 1.5TB/s over HBM", "per chip."],
            &["HBM 대역폭은 1.5TB/s입니다", "칩당입니다.", "칩당입니다."],
        );
        assert_eq!(found, [(1, 2)]);
    }

    #[test]
    fn a_restated_subject_is_not_evidence() {
        let found = check(
            Markup::Rst,
            &["It imposes further limitations.", "Argument Clinic supports optional groups."],
            &["Argument Clinic은 추가 제한 사항을 적용합니다.", "Argument Clinic은 선택적 그룹을 지원합니다."],
        );
        assert!(found.is_empty(), "{found:?}");
    }

    #[test]
    fn a_restated_symbol_is_not_evidence() {
        let found = check(
            Markup::Latex,
            &["Suppose there is a subset $X' \\subseteq X$.", "The elements of $X'$ sum to $T$."],
            &["부분집합 $X' \\subseteq X$인 $X'$가 있다고 합시다.", "$X'$의 원소의 합은 $T$입니다."],
        );
        assert!(found.is_empty(), "{found:?}");
    }

    #[test]
    fn a_double_backtick_span_closes_on_its_own_run() {
        // RST: `` ``repr()`` `` must not pair its second backtick with the
        // role that follows, or the words in between vanish from the source.
        let found = check(
            Markup::Rst,
            &[
                "It helps the ``repr()`` of objects in the :py:mod:`typing` and :py:mod:`collections.abc` modules.",
                "A new function is added to the :py:mod:`!typing` module, ``typing.evaluate_forward_ref``.",
            ],
            &[
                ":py:mod:`typing` 및 :py:mod:`collections.abc` 모듈의 객체에 대한 ``repr()``\\ 을 돕습니다.",
                ":py:mod:`!typing` 모듈에 ``typing.evaluate_forward_ref`` 함수가 추가됩니다.",
            ],
        );
        assert!(found.is_empty(), "{found:?}");
        let spans = anchors("See ``repr()`` and `draft of PEP-728 <https://example.com/728>`_.", Markup::Rst);
        assert!(spans.contains("`repr`"));
        assert!(spans.contains("pep") && spans.contains("728") && spans.contains("https://example.com/728"));
    }

    #[test]
    fn a_role_target_made_explicit_is_its_own() {
        let found = check(
            Markup::Rst,
            &["Non-final versions get a qualifier: :ref:`alpha`, :ref:`beta`.", "Each :ref:`alpha` or :ref:`beta` release is tagged."],
            &["비최종 버전에는 :ref:`알파 <alpha>`, :ref:`베타 <beta>` 같은 한정자가 붙습니다.", "각 :ref:`alpha`\\ 나 :ref:`beta` 릴리스에 태그가 붙습니다."],
        );
        assert!(found.is_empty(), "{found:?}");
    }

    #[test]
    fn latex_quotes_are_not_code() {
        let found = check(
            Markup::Latex,
            &[
                "Any CW complex is obtained by ``coning off'' spheres, as in \\cref{sec:hubs-spokes}.",
                "See \\cref{sec:hubs-spokes} for ``hubs'' and ``spokes''.",
            ],
            &[
                "\\cref{sec:hubs-spokes}에서처럼 구면을 ``뿔로 막아'' 임의의 CW 복합체를 얻습니다.",
                "``허브''와 ``바큇살''은 \\cref{sec:hubs-spokes}를 보십시오.",
            ],
        );
        assert!(found.is_empty(), "{found:?}");
    }

    #[test]
    fn spelling_variants_match_their_source() {
        // Plural numbers, hyphenated compounds either way, units with or
        // without a space, a plural inside code.
        let found = check(
            Markup::Rst,
            &[
                "A code assigns 0s and 1s to a halfword; both int-to-float and int-to-Decimal work at 250 MHz with ``ParamSpecs``.",
                "Codes of 0 and 1 load half-words, float and Decimal, 250MHz, ``ParamSpec``.",
            ],
            &[
                "코드는 half-word에 0과 1을 할당하며, int에서 float, int에서 Decimal로의 변환 모두 250MHz에서 ``ParamSpec``\\ s로 동작합니다.",
                "0과 1의 코드는 half-word, float와 Decimal, 250MHz, ``ParamSpec``\\ 을 적재합니다.",
            ],
        );
        assert!(found.is_empty(), "{found:?}");
    }

    #[test]
    fn a_sentence_split_by_a_comment_moves_its_date() {
        // rustc-dev-guide: "As of <!-- date-check --> November 2024, …".
        let found = check(
            Markup::Markdown,
            &["As of", "November 2024, most of the rust compiler is now parallelized."],
            &["2024년 11월", "기준, 러스트 컴파일러의 대부분이 병렬화되었습니다."],
        );
        assert!(found.is_empty(), "{found:?}");
        let found = check(
            Markup::Markdown,
            &["As of", "January 2021, this input data consists mainly of the HIR map."],
            &["2021년 1월 기준으로,", "2021년 1월 기준으로, 이 입력 데이터는 주로 HIR 맵으로 구성됩니다."],
        );
        assert_eq!(found, [(1, 2)]);
    }

    #[test]
    fn inflections_code_forms_and_urls_match_their_source() {
        let found = check(
            Markup::Markdown,
            &[
                "A value is reached by reinterpreting first and stashing after; the \"make\" step calls `tp_alloc()`.",
                "A reinterpret decides what the stash writes; see `make` and `tp_alloc`.",
            ],
            &[
                "값은 먼저 reinterpret한 다음 stash하여 도달하며, \"`make`\" 단계는 `tp_alloc`을 호출합니다.",
                "reinterpret은 stash가 무엇을 쓸지 결정합니다. `make`와 `tp_alloc`을 보십시오.",
            ],
        );
        assert!(found.is_empty(), "{found:?}");
        let found = check(
            Markup::Latex,
            &["Install it from \\myref{https://adoptopenjdk.net/}{AdoptOpenJDK}.", "See \\myref{https://adoptopenjdk.net/}{AdoptOpenJDK} too."],
            &["\\myref{https://adoptopenjdk.net/}{AdoptOpenJDK}에서 설치하십시오.", "\\myref{https://adoptopenjdk.net/}{AdoptOpenJDK}도 보십시오."],
        );
        assert!(found.is_empty(), "{found:?}");
    }

    #[test]
    fn a_duplicate_with_one_word_translated_away_is_still_a_duplicate() {
        // peps: "name: str" received the next item, whose own translation
        // renders "variant" in Korean.
        let found = check(
            Markup::Rst,
            &[
                "``name: str`` specifying the feature name.",
                "``multi_value: bool`` specifying whether values vary within a single variant wheel.",
            ],
            &[
                "단일 variant 휠 내에서 값이 달라지는지 지정하는 ``multi_value: bool``\\ 입니다.",
                "``multi_value: bool``\\ 은 단일 변형 휠 내에서 값이 달라지는지 지정합니다.",
            ],
        );
        assert_eq!(found, [(1, 2)]);
    }

    #[test]
    fn a_move_is_not_blamed_on_a_farther_segment() {
        // The date moved in from the next segment; an earlier segment happens
        // to carry the same date.
        let found = check(
            Markup::Markdown,
            &[
                "November 2024, the front-end is changing.",
                "Some text.",
                "As of",
                "November 2024, most of the compiler is parallel.",
            ],
            &["2024년 11월 기준, 프런트엔드가 바뀌고 있습니다.", "어떤 글.", "2024년 11월", "기준, 컴파일러 대부분이 병렬화되었습니다."],
        );
        assert!(found.is_empty(), "{found:?}");
    }
}
