//! Display width, and where to break a translated line so it wraps the way
//! the source did.

/// Display width in terminal columns: East Asian Wide and Fullwidth
/// characters occupy two columns, everything else one. docutils counts a
/// title's width this way when it checks the adornment under it.
pub fn display_width(text: &str) -> usize {
    text.chars().map(|c| if is_wide(c) { 2 } else { 1 }).sum()
}

/// Unicode East Asian Width W or F, over the ranges that occur in practice.
pub fn is_wide(c: char) -> bool {
    matches!(c as u32,
        0x1100..=0x115F           // Hangul Jamo
        | 0x2E80..=0x303E         // CJK Radicals .. CJK Symbols and Punctuation
        | 0x3041..=0x33FF         // Hiragana .. CJK Compatibility
        | 0x3400..=0x4DBF         // CJK Extension A
        | 0x4E00..=0x9FFF         // CJK Unified Ideographs
        | 0xA000..=0xA4CF         // Yi
        | 0xAC00..=0xD7A3         // Hangul Syllables
        | 0xF900..=0xFAFF         // CJK Compatibility Ideographs
        | 0xFE30..=0xFE4F         // CJK Compatibility Forms
        | 0xFF00..=0xFF60         // Fullwidth Forms
        | 0xFFE0..=0xFFE6
        | 0x20000..=0x2FFFD
        | 0x30000..=0x3FFFD)
}

/// The byte position of the space to break `text` at, so its first line
/// comes closest to `target_width` columns.
///
/// Only spaces `is_safe` accepts qualify — the caller rules out spaces inside
/// inline markup — and only where the rest holds a letter or digit: a second
/// line of punctuation alone would read as an adornment or a marker. `None`
/// when no space qualifies; the text then stays on one line.
pub fn split_at_safe_space(
    text: &str,
    target_width: usize,
    is_safe: impl Fn(usize) -> bool,
) -> Option<usize> {
    text.match_indices(' ')
        .map(|(at, _)| at)
        .filter(|&at| is_safe(at))
        .filter(|&at| text[at + 1..].chars().any(char::is_alphanumeric))
        .min_by_key(|&at| display_width(&text[..at]).abs_diff(target_width))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn breaks_at_the_space_nearest_the_target_width() {
        assert_eq!(split_at_safe_space("aaa bbb ccc", 4, |_| true), Some(3));
        assert_eq!(split_at_safe_space("aaa bbb ccc", 8, |_| true), Some(7));
    }

    #[test]
    fn counts_wide_characters_as_two_columns() {
        // "가나다" is 6 columns wide, so the space after it is nearest 6.
        assert_eq!(
            split_at_safe_space("가 나다 라", 6, |_| true),
            Some("가 나다".len())
        );
    }

    #[test]
    fn skips_spaces_the_caller_rules_out() {
        assert_eq!(split_at_safe_space("aaa bbb ccc", 4, |at| at != 3), Some(7));
        assert_eq!(split_at_safe_space("aaa bbb", 4, |_| false), None);
    }

    #[test]
    fn never_leaves_punctuation_alone_on_the_second_line() {
        assert_eq!(split_at_safe_space("제목 ---", 4, |_| true), None);
        assert_eq!(split_at_safe_space("a b ---", 4, |_| true), Some(1));
    }
}
