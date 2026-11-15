//! Segments with nothing to translate: a lone code span, a dash, a size.
//!
//! Sent to a model, such a segment comes back unchanged at best; at worst
//! with a word added or the code "corrected", and then the evaluators and
//! the retry loop run over text nobody could translate. The orchestrator
//! records these as their own translation without a request.
//!
//! The test is deliberately narrow. Whatever holds a letter outside the
//! constructs below goes to the model: an identifier, a name, a version
//! string, an HTML tag, a formula, a MyST role (`{ref}` stands outside its
//! backticks), an MDX expression.

use crate::evaluator::Markup;

/// Units a Korean text writes as the source does, after a number. Seconds,
/// minutes, hours and metres are left out: they may well become 초, 분, 시간.
const UNITS: [&str; 22] = [
    "B", "KB", "MB", "GB", "TB", "PB", "kB", "KiB", "MiB", "GiB", "TiB", "PiB", "Hz", "kHz", "MHz", "GHz", "ms", "us",
    "µs", "μs", "ns", "ps",
];

/// Whether backticks are code and `](…)` a destination in `markup`. In
/// reStructuredText a backtick pair holds a link's text, in AsciiDoc a
/// passthrough may: there only the letters decide.
fn reads_as_markdown(markup: Markup) -> bool {
    matches!(
        markup,
        Markup::Markdown | Markup::MkDocs | Markup::MkDocsNoMath | Markup::RMarkdown | Markup::Bikeshed
    )
}

/// Whether `source` holds nothing a translator could change: no letter is
/// left once — in a Markdown markup — its code spans, URLs, link
/// destinations and the units after its numbers are taken out. Digits,
/// punctuation and symbols were never letters.
pub fn nothing_to_translate(markup: Markup, source: &str) -> bool {
    if !reads_as_markdown(markup) {
        return !source.chars().any(char::is_alphabetic);
    }
    let chars: Vec<char> = source.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c == '`' {
            match code_span_end(&chars, i) {
                Some(end) => i = end,
                // An unclosed backtick is text, and so is what follows.
                None => i += 1,
            }
        } else if c == ']' && chars.get(i + 1) == Some(&'(') {
            match chars[i + 2..].iter().position(|c| *c == ')') {
                Some(close) => i += 2 + close + 1,
                None => i += 1,
            }
        } else if let Some(end) = url_end(&chars, i) {
            i = end;
        } else if c.is_ascii_digit() {
            i = unit_end(&chars, i);
        } else if c.is_alphabetic() {
            return false;
        } else {
            i += 1;
        }
    }
    true
}

/// Where the code span opening at `at` ends: after the next run of as many
/// backticks.
fn code_span_end(chars: &[char], at: usize) -> Option<usize> {
    let run = |from: usize| chars[from..].iter().take_while(|c| **c == '`').count();
    let open = run(at);
    let mut from = at + open;
    loop {
        let close = from + chars[from..].iter().position(|c| *c == '`')?;
        let len = run(close);
        if len == open {
            return Some(close + len);
        }
        from = close + len;
    }
}

/// Where the bare URL starting at `at` ends, if one starts there.
fn url_end(chars: &[char], at: usize) -> Option<usize> {
    if at > 0 && chars[at - 1].is_alphanumeric() {
        return None;
    }
    let starts = |prefix: &str| chars[at..].iter().copied().take(prefix.chars().count()).eq(prefix.chars());
    ["http://", "https://", "www."].iter().any(|p| starts(p)).then(|| {
        at + chars[at..].iter().position(|c| c.is_whitespace() || matches!(c, '<' | '>' | ')' | ']')).unwrap_or(chars.len() - at)
    })
}

/// The end of the number starting at `at`, with a unit from [`UNITS`] taken
/// in when one follows it (after at most one space) and ends there.
fn unit_end(chars: &[char], at: usize) -> usize {
    let mut end = at;
    while chars.get(end).is_some_and(|c| c.is_ascii_digit() || matches!(c, '.' | ',')) {
        end += 1;
    }
    let unit_start = if matches!(chars.get(end), Some(' ' | '\u{a0}')) { end + 1 } else { end };
    let unit_len = chars[unit_start..].iter().take_while(|c| c.is_alphabetic()).count();
    let unit: String = chars[unit_start..unit_start + unit_len].iter().collect();
    let bounded = !chars.get(unit_start + unit_len).is_some_and(|c| c.is_alphanumeric() || *c == '_');
    if unit_len > 0 && bounded && UNITS.contains(&unit.as_str()) { unit_start + unit_len } else { end }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn code_dashes_numbers_units_and_urls_have_nothing_to_translate() {
        for source in [
            "`foo()`",
            "``a ` b``",
            "`kmalloc()` → `kfree()`",
            "—",
            "-",
            "…",
            "",
            "4 KB",
            "~10 ms",
            "2.6.32",
            "1,024",
            "64 MiB / 2 GiB",
            "100%",
            "$5",
            "https://lwn.net/Articles/1/",
            "<https://lwn.net/Articles/1/>",
            "[`mm/slub.c`](https://git.kernel.org/x/mm/slub.c)",
            "[`x`](y.md)",
            "**`CONFIG_NUMA`**",
            "`a`, `b` (4 KB)",
        ] {
            assert!(nothing_to_translate(Markup::MkDocsNoMath, source), "{source}");
            assert!(nothing_to_translate(Markup::Markdown, source), "{source}");
        }
    }

    #[test]
    fn anything_with_a_letter_of_its_own_goes_to_the_model() {
        for source in [
            "foo",
            "Yes",
            "N/A",
            "v6.1",
            "O(1)",
            "10 s",
            "5 min",
            "4 KBs",
            "x86",
            "`foo()` returns",
            "[docs](y.md)",
            "[`x`](y.md) and more",
            "<br>",
            "&nbsp;",
            "$x$",
            "{ref}`label <target>`",
            "`unclosed",
            "예",
            "Linus Torvalds",
            "CVE-2024-1086",
        ] {
            assert!(!nothing_to_translate(Markup::MkDocsNoMath, source), "{source}");
        }
    }

    #[test]
    fn other_markups_decide_by_letters_alone() {
        // A reStructuredText backtick pair is a link's text, not code.
        assert!(!nothing_to_translate(Markup::Rst, "`Python`_"));
        assert!(!nothing_to_translate(Markup::Asciidoc, "`code`"));
        assert!(!nothing_to_translate(Markup::Latex, "4 KB"));
        assert!(!nothing_to_translate(Markup::Rst, "https://example.com"));
        for markup in [Markup::Rst, Markup::Asciidoc, Markup::Latex, Markup::Xhtml, Markup::Plain, Markup::Verso] {
            assert!(nothing_to_translate(markup, "—"));
            assert!(nothing_to_translate(markup, "1.2"));
            assert!(nothing_to_translate(markup, "§ 3"));
        }
    }
}
