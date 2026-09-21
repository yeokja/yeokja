use crate::evaluator::*;
use async_trait::async_trait;

pub struct FormatEvaluator;

#[async_trait]
impl TranslationEvaluator for FormatEvaluator {
    async fn evaluate(
        &self,
        context: &EvaluationContext,
    ) -> Result<EvaluationResult, EvaluationError> {
        let mut issues = Self::markup_issues(context);
        issues.extend(parenthesis_issues(context));
        let passed = !issues.iter().any(|i| i.severity == IssueSeverity::Error);
        Ok(EvaluationResult { passed, issues })
    }

    fn triggers_retranslation(&self) -> bool {
        true
    }

    fn name(&self) -> &'static str {
        "Format"
    }
}

impl FormatEvaluator {
    fn markup_issues(context: &EvaluationContext) -> Vec<EvaluationIssue> {
        if context.markup == Markup::MkDocs {
            let mut issues = mkdocs_issues(&context.source, &context.translation);
            // Formulas are copied verbatim (checked above); hide them so their
            // `_`, `*` and backticks do not count as Markdown emphasis or code.
            let masked = EvaluationContext {
                source: yeokja_parser_mkdocs::mask_math(&context.source),
                translation: yeokja_parser_mkdocs::mask_math(&context.translation),
                markup: Markup::Markdown,
                ..context.clone()
            };
            issues.extend(Self::markup_issues(&masked));
            return issues;
        }

        let mut issues = Vec::new();

        // A full reference link keeps the source text as its label so the
        // text can be translated, `[*단형화*][_monomorphized_]`. The label is
        // not rendered, and its marks are no emphasis of the translation.
        let (visible_source, visible_translation) = if context.markup == Markup::Markdown {
            (
                without_reference_labels(&context.source),
                without_reference_labels(&context.translation),
            )
        } else {
            (context.source.clone(), context.translation.clone())
        };

        // Code the translation marks up where the source wrote it as text, or
        // repeats where Korean names the subject again, is no code gained:
        // `.pem` for .pem. It is excused from the counts below and still has
        // to close.
        let source_spans = backtick_spans(&chars(&context.source), context.markup);
        let open_source = source_spans.unclosed
            || (context.markup == Markup::Asciidoc
                && !pair_up(&chars(&context.source), '`').unclosable.is_empty());
        let translation_spans = backtick_spans(&chars(&context.translation), context.markup);
        let (missing, left) = unmatched_code(&source_spans.spans, &translation_spans.spans);
        // A left-over span that holds what a role or interpreted text of the
        // source held is that span recast, not an addition.
        let mut kept: Vec<&BacktickSpan> = translation_spans
            .spans
            .iter()
            .filter(|span| !span.code)
            .collect();
        let mut roles: Vec<&BacktickSpan> = Vec::new();
        for role in source_spans.spans.iter().filter(|span| !span.code) {
            match kept.iter().position(|span| span.content == role.content) {
                Some(at) => drop(kept.remove(at)),
                None => roles.push(role),
            }
        }
        let mut recast = Vec::new();
        let mut from_source_text = 0;
        if !open_source {
            for span in &left {
                if let Some(at) = roles.iter().position(|role| role.content == span.content) {
                    recast.push(format!(
                        "{} became the literal {}",
                        roles.remove(at).written,
                        span.written
                    ));
                } else if written_as_text(&context.source, &span.content) {
                    from_source_text += 1;
                }
            }
        }
        if !recast.is_empty() {
            issues.push(EvaluationIssue {
                severity: IssueSeverity::Error,
                kind: IssueKind::FormatLost,
                message: format!(
                    "{}. A role or interpreted text renders differently from code — a :mod: \
                     role links, :math: typesets — so keep it the way the source writes it.",
                    recast.join(", "),
                ),
            });
        }

        // Counted by run, not by character: `` ``code`` `` marks the same one
        // pair as `` `code` ``, written the way an unclosable pair has to be
        // rewritten (see below). Counting characters would read that fix as two
        // markers the source never had and reject it.
        //
        // Markdown and Verso use both `_text_` and `*text*` for emphasis. `_`
        // cannot close before a Korean particle, so the prompt deliberately
        // asks the translator to switch that pair to `*`. Count the two spellings
        // together for those markups; treating the instructed rewrite as a lost
        // marker makes every retry repeat an impossible demand.
        // Backticks and asterisks are ordinary punctuation in LaTeX (not
        // Markdown-style inline markup), so applying these checks there rejects
        // valid TeX quotations such as ``term'' and mathematical `*` tokens.
        if context.markup != Markup::Latex {
            let source_code_runs = mark_runs(&context.source, '`')
                - if context.markup == Markup::Rst {
                    rst_reference_runs(&context.source)
                } else {
                    0
                };
            let translation_code_runs = (mark_runs(&context.translation, '`')
                - if context.markup == Markup::Rst {
                    rst_reference_runs(&context.translation)
                } else {
                    0
                })
            .saturating_sub(2 * from_source_text);
            let checks = [
                (
                    "Inline code markers (`)",
                    source_code_runs,
                    translation_code_runs,
                ),
                (
                    "Emphasis marker runs",
                    emphasis_runs(&visible_source, context.markup),
                    emphasis_runs(&visible_translation, context.markup),
                ),
            ];
            for (name, in_source, in_translation) in checks {
                if in_source != in_translation {
                    issues.push(EvaluationIssue {
                        severity: IssueSeverity::Error,
                        kind: IssueKind::FormatLost,
                        message: format!(
                            "{name} count mismatch: source has {in_source}, \
                             translation has {in_translation}"
                        ),
                    });
                }
            }
        }

        // Counting markers says nothing about what is between them. Code is
        // copied, not translated: every code span of the source has to come
        // back unchanged, wherever Korean word order puts it. A source that
        // leaves a span open pairs its marks differently from a translation
        // that closes it, so its spans are no measure.
        if !open_source && !missing.is_empty() {
            issues.push(EvaluationIssue {
                severity: IssueSeverity::Error,
                kind: IssueKind::FormatLost,
                message: format!(
                    "Inline code changed: {} is not in the translation{}. Copy the content \
                     of every code span byte for byte, even prose or a typo inside it; only \
                     the text around it is translated.{}",
                    missing.join(", "),
                    if left.is_empty() {
                        String::new()
                    } else {
                        let written: Vec<&str> =
                            left.iter().map(|span| span.written.as_str()).collect();
                        format!(", which writes {} instead", written.join(", "))
                    },
                    // Told to double the marks before a particle, the
                    // translator takes the passthrough pluses for marks too
                    // and drops them, retry after retry; a space it keeps.
                    if context.markup == Markup::Asciidoc
                        && missing.iter().any(|written| {
                            let content = written.trim_matches('`');
                            content.len() > 1 && content.starts_with('+') && content.ends_with('+')
                        })
                    {
                        " A `+…+` span is a passthrough: without its pluses AsciiDoc rewrites \
                         its text (-> becomes an arrow). Keep the span exactly as the source \
                         writes it and put a space after its closing mark instead of doubling \
                         the marks: `+x+` 같은, `+x+` 와."
                    } else {
                        ""
                    },
                ),
            });
        }

        // Closing somewhere is not closing where the source closed. Asciidoctor
        // does not give up on a pair whose closing mark a letter follows — it
        // keeps looking and closes on a later mark instead, so `_it_는 and
        // _other_ 사이` renders as one emphasis over `it_는 and _other`. The
        // prose in the middle is swallowed and the pairs after it are eaten, yet
        // nothing is left unclosed and the run count still matches. What gives it
        // away is how many pairs actually form.
        for &(mark, unconstrained) in constrained_marks(context.markup) {
            let source = pair_up(&chars(&context.source), mark);
            // A source that leaves a pair open has no count worth matching, and
            // demanding the translation match it would reject the rewrite that
            // fixes it. `chapters/type_system.asciidoc` writes `` `{...}`` ``.
            if !source.unclosable.is_empty() {
                continue;
            }
            let (in_source, in_translation) =
                if mark == '_' && matches!(context.markup, Markup::Markdown | Markup::Verso) {
                    (
                        markdown_emphasis_pairs(&visible_source),
                        markdown_emphasis_pairs(&visible_translation),
                    )
                } else {
                    let excused = if mark == '`' { from_source_text } else { 0 };
                    (
                        source.formed,
                        pair_up(&chars(&context.translation), mark)
                            .formed
                            .saturating_sub(excused),
                    )
                };
            if in_source != in_translation {
                issues.push(EvaluationIssue {
                    severity: IssueSeverity::Error,
                    kind: IssueKind::FormatLost,
                    message: format!(
                        "Inline pairs of {mark} do not line up: the source forms {in_source}, \
                         the translation {in_translation}. A pair whose closing mark a letter \
                         follows closes on a later mark instead, swallowing the text between \
                         and absorbing the pairs after it. Write each marked-up term a suffix \
                         follows as {unconstrained}, doubling the mark at BOTH ends."
                    ),
                });
            }
        }

        // AsciiDoc's curved quotes are constrained the same way, and there is no
        // doubled form to escape to: `"`term`"를` prints its marks as themselves.
        if context.markup == Markup::Asciidoc {
            let quotes = unclosable_quotes(&context.translation);
            if !quotes.is_empty() && unclosable_quotes(&context.source).is_empty() {
                issues.push(EvaluationIssue {
                    severity: IssueSeverity::Error,
                    kind: IssueKind::FormatLost,
                    message: format!(
                        "{} never closes: a curved-quote pair cannot close against a letter \
                         either, and it has no doubled form. Write the quotation marks \
                         themselves — “term” — or put a space or punctuation after the \
                         closing mark.",
                        quotes.join(", "),
                    ),
                });
            }
        }

        // reStructuredText recognizes an opening marker only when no word
        // character precedes it and a closing one only when none follows, and
        // there is no doubled form to escape to — the way out is a
        // backslash-escaped space, which renders as nothing.
        if context.markup == Markup::Rst {
            let source_malformed_roles = rst_malformed_role_closures(&context.source);
            let mut source_role_counts = std::collections::HashMap::<&str, usize>::new();
            for role in &source_malformed_roles {
                *source_role_counts.entry(role).or_default() += 1;
            }
            let malformed_roles: Vec<String> = rst_malformed_role_closures(&context.translation)
                .into_iter()
                .filter(|role| {
                    let Some(count) = source_role_counts.get_mut(role.as_str()) else {
                        return true;
                    };
                    if *count == 0 {
                        return true;
                    }
                    *count -= 1;
                    false
                })
                .collect();
            if !malformed_roles.is_empty() {
                issues.push(EvaluationIssue {
                    severity: IssueSeverity::Error,
                    kind: IssueKind::FormatLost,
                    message: format!(
                        "{} has an extra backtick after an RST role. Preserve the role as, for example, :pep:`649`.",
                        malformed_roles.join(", "),
                    ),
                });
            }

            // A segment that begins inside emphasis an earlier sentence opened
            // gives its stars no measure; its literals still have one.
            let marks: &[char] = if rst_starts_inside_emphasis(&context.source) {
                &['`']
            } else {
                &['`', '*']
            };
            let broken = rst_broken_pairs_of(&context.translation, marks);
            if !broken.is_empty() && rst_broken_pairs(&context.source).is_empty() {
                let shown: Vec<&str> = broken.iter().map(String::as_str).collect();
                issues.push(EvaluationIssue {
                    severity: IssueSeverity::Error,
                    kind: IssueKind::FormatLost,
                    message: format!(
                        "{} is not recognized as markup: reStructuredText requires \
                         whitespace or punctuation on the outside of each marker, and \
                         doubling the marks does not help. Separate the word from the \
                         marker with a backslash-escaped space, which renders as \
                         nothing: ``heap``\\ 에, **bold**\\ 를, 실행\\ **될**.",
                        shown.join(", "),
                    ),
                });
            }

            let broken = rst_broken_bracket_references(&context.translation);
            if !broken.is_empty() && rst_broken_bracket_references(&context.source).is_empty() {
                issues.push(EvaluationIssue {
                    severity: IssueSeverity::Error,
                    kind: IssueKind::FormatLost,
                    message: format!(
                        "{} is not recognized as a footnote or citation reference: \
                         reStructuredText requires whitespace or punctuation after the \
                         trailing underscore. Separate a Korean particle with a \
                         backslash-escaped space, which renders as nothing: [2]_\\ 에.",
                        broken.join(", "),
                    ),
                });
            }
        }

        // Verso roles carry semantic identifiers in their headers, and code
        // roles use their payload to locate checked examples. They can remain
        // superficially balanced after a translator changes or drops those
        // values, so compare the structural tokens themselves rather than only
        // counting braces and backticks. Visible `[labels]` remain free to be
        // translated; only their bracketed shape is recorded.
        if context.markup == Markup::Verso {
            let source = verso_structure(&context.source);
            let translation = verso_structure(&context.translation);
            if source != translation {
                issues.push(EvaluationIssue {
                    severity: IssueSeverity::Error,
                    kind: IssueKind::FormatLost,
                    message: format!(
                        "Verso role structure changed: preserve every `{{role arguments}}` \
                         header and every backticked code/math payload exactly. Visible \
                         text inside `[labels]` may be translated. Source tokens: {source:?}; \
                         translation tokens: {translation:?}"
                    ),
                });
            }
        }

        if context.markup == Markup::Latex {
            let mut source = latex_structure(&context.source);
            let mut translation = latex_structure(&context.translation);
            let groups_preserved =
                latex_group_balance(&source) == latex_group_balance(&translation);
            // Korean grammar routinely moves a displayed term or reference to
            // another part of the sentence. The safety property is that every
            // structural token survives byte-for-byte, including duplicates;
            // their prose-level order is not itself LaTeX syntax.
            source.sort_unstable();
            translation.sort_unstable();
            if !groups_preserved || !is_multiset_subset(&source, &translation) {
                issues.push(EvaluationIssue {
                    severity: IssueSeverity::Error,
                    kind: IssueKind::FormatLost,
                    message: format!(
                        "LaTeX structure changed: preserve every source command, brace, bracket, \
                         comment placeholder, and mathematical expression (natural sentence \
                         reordering, translated prose inside \\text{{...}}, and moved terminal \
                         punctuation are allowed). Source tokens: \
                         {source:?}; translation tokens: {translation:?}"
                    ),
                });
            }
        }

        // A constrained pair has to close next to a non-word character, and
        // Korean writes its particles straight onto the word before them.
        let unclosable = unclosable_pairs(&context.translation, context.markup);
        if !unclosable.is_empty() && unclosable_pairs(&context.source, context.markup).is_empty() {
            // Name every one of them. A segment often carries several marked-up
            // terms and only some of them take a suffix, so "a pair does not
            // close" leaves the translator guessing which — and it fixes one
            // and leaves the rest.
            let shown: Vec<&str> = unclosable.iter().map(|u| u.text.as_str()).collect();
            let unconstrained = unclosable[0].unconstrained;
            issues.push(EvaluationIssue {
                severity: IssueSeverity::Error,
                kind: IssueKind::FormatLost,
                message: format!(
                    "{} never closes: the marks print as themselves and the run swallows \
                     the text after them. Write {} as {}, doubling the mark at BOTH ends \
                     — doubling the closing one alone closes neither way — or put a space \
                     or punctuation after the closing mark.",
                    shown.join(", "),
                    if shown.len() == 1 { "it" } else { "each" },
                    unconstrained,
                ),
            });
        }

        // A segment's span starts where a line does, so the first character of
        // a translation lands where markup is read.
        if let Some(opened) = line_start_construct(&context.translation, context.markup)
            && Some(opened) != line_start_construct(&context.source, context.markup)
        {
            issues.push(EvaluationIssue {
                severity: IssueSeverity::Error,
                kind: IssueKind::FormatLost,
                message: format!(
                    "Translation begins with {opened}, which the source does not. At \
                     the start of a line that is markup, not text — begin with a word \
                     instead, or reorder the sentence."
                ),
            });
        }

        issues
    }
}

/// Parentheses the translation doubles or leaves unpaired.
///
/// They are read in the raw text, where the source's math is still there to
/// be seen; MkDocs math is masked here rather than by the caller.
fn parenthesis_issues(context: &EvaluationContext) -> Vec<EvaluationIssue> {
    let (source, translation, markup) = if context.markup == Markup::MkDocs {
        (
            yeokja_parser_mkdocs::mask_math(&context.source),
            yeokja_parser_mkdocs::mask_math(&context.translation),
            Markup::Markdown,
        )
    } else {
        (context.source.clone(), context.translation.clone(), context.markup)
    };
    let mut issues = Vec::new();

    // Korean has no preposition to open a parenthetical with, so "(in
    // [`panicking.rs`])" becomes a link and a particle — and the pair around
    // it has come back written twice, `(([`panicking.rs`]에서))`. Prose
    // never needs a pair that holds nothing but another pair.
    let doubled = doubled_parentheses(&translation, markup);
    if doubled.len() > doubled_parentheses(&source, markup).len() {
        issues.push(EvaluationIssue {
            severity: IssueSeverity::Error,
            kind: IssueKind::FormatLost,
            message: format!(
                "{} wraps its text in two pairs of parentheses where the source has one. \
                 Write a single pair: (X에서), not ((X에서)).",
                doubled.join(", "),
            ),
        });
    } else if parentheses_balance(&source, markup)
        // Code or math that leaves a parenthesis open can pair it with the
        // prose — `(with length $O(\sqrt{n}$)` — and a translation that
        // reads it that way is right.
        && context.source.matches('(').count() == context.source.matches(')').count()
        && !parentheses_balance(&translation, markup)
    {
        issues.push(EvaluationIssue {
            severity: IssueSeverity::Error,
            kind: IssueKind::FormatLost,
            message: "Parentheses do not balance: the translation opens or closes a pair \
                      the source does not. Keep each ( ) pair of the source exactly once."
                .to_string(),
        });
    }

    issues
}

/// How many times `mark` opens or closes an inline pair, counting a run of
/// them once.
///
/// Curved quotes borrow the backtick — `"`term`"` is a quotation, not code — so
/// those marks are not counted. The only way to write a quotation whose closing
/// mark a Korean particle follows is to give up the construct and write “term”
/// outright, and counting the borrowed marks would read that fix as two code
/// markers dropped.
fn mark_runs(text: &str, mark: char) -> usize {
    let chars = chars(text);
    let borrowed: std::collections::HashSet<usize> = if mark == '`' {
        curved_quotes(&chars)
            .into_iter()
            .flat_map(|(open, close)| [open, close])
            .collect()
    } else {
        std::collections::HashSet::new()
    };
    let counts = |at: usize| chars[at] == mark && !borrowed.contains(&at);

    let mut runs = 0;
    let mut i = 0;
    while i < chars.len() {
        if !counts(i) {
            i += 1;
            continue;
        }
        runs += 1;
        while i < chars.len() && counts(i) {
            i += 1;
        }
    }
    runs
}

/// `text` without the labels of its full reference links: `[번역][label]`
/// reads as `[번역]`.
fn without_reference_labels(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(at) = rest.find("][") {
        out.push_str(&rest[..=at]);
        let after = &rest[at + 2..];
        rest = match after.find(']') {
            Some(close) if !after[..close].contains('[') => &after[close + 1..],
            _ => &rest[at + 1..],
        };
    }
    out.push_str(rest);
    out
}

/// Emphasis marker runs whose spelling may change without changing meaning.
fn emphasis_runs(text: &str, markup: Markup) -> usize {
    let stars = mark_runs(text, '*');
    if matches!(markup, Markup::Markdown | Markup::Verso) {
        stars + mark_runs(text, '_')
    } else {
        stars
    }
}

/// Emphasis pairs that actually form under Markdown/Verso flanking rules.
///
/// `_` needs the constrained pairing simulation, while `*` is allowed directly
/// before a Korean word character and each two marker runs form one pair.
fn markdown_emphasis_pairs(text: &str) -> usize {
    pair_up(&chars(text), '_').formed + mark_runs(text, '*') / 2
}

/// What a mark does across a stretch of text: how many pairs it actually forms,
/// and every pair it opens without being able to close.
///
/// The two answer different questions. A swallowed pair is closed — just not
/// where the source closed it — so only `formed` tells it apart from prose.
#[derive(Default)]
struct Pairing {
    formed: usize,
    unclosable: Vec<std::ops::Range<usize>>,
}

/// An inline pair that cannot close: the offending text as written, and the
/// form of the pair that survives a word character against its closing mark.
struct Unclosable {
    text: String,
    unconstrained: &'static str,
}

/// Every constrained inline pair `text` opens but cannot close.
///
/// AsciiDoc reads `` `code` `` as code only when neither mark touches a word
/// character, and Asciidoctor spells "word character" `\p{Word}` — which every
/// Hangul syllable satisfies. Korean writes its particles onto the end of the
/// word they attach to, so the natural translation of "on the `heap`" ends
/// `` `heap`에 ``: a pair that never closes. Both marks then print as
/// themselves, and the opening one keeps looking for a partner, swallowing the
/// text up to the next mark in the paragraph.
///
/// Which marks that covers is `constrained_marks`.
fn unclosable_pairs(text: &str, markup: Markup) -> Vec<Unclosable> {
    let chars = chars(text);
    let mut found = Vec::new();
    for &(mark, unconstrained) in constrained_marks(markup) {
        for span in pair_up(&chars, mark).unclosable {
            found.push(Unclosable {
                text: chars[span].iter().collect(),
                unconstrained,
            });
        }
    }
    found
}

fn chars(text: &str) -> Vec<char> {
    text.chars().collect()
}

/// A backtick-delimited span: where it sits (marks included), as written, its
/// content with whitespace runs collapsed — a source line break inside a span
/// renders as a space — and whether that content is code.
struct BacktickSpan {
    range: std::ops::Range<usize>,
    written: String,
    content: String,
    code: bool,
}

struct BacktickSpans {
    spans: Vec<BacktickSpan>,
    /// Whether a run of marks opened a span that nothing closes.
    unclosed: bool,
}

/// Every backtick-delimited span in `chars`.
///
/// A run of marks closes on the next run of the same length, except that an
/// RST inline literal closes on the last two marks of the next run of two or
/// more:
/// ``:func:`filter``` holds a role, backtick and all.
///
/// Only some spans hold code. A MyST role (`` {ref}`guide <target>` ``) and
/// RST interpreted text (`` `label`_ ``, `` :term:`…` ``) put a visible label
/// between their backticks, which a translation translates; an AsciiDoc curved
/// quote (`` "`term`" ``) borrows the backtick without being a span at all.
/// Verso compares its payloads in `verso_structure`, and in LaTeX a backtick is
/// an opening quotation mark.
fn backtick_spans(chars: &[char], markup: Markup) -> BacktickSpans {
    let mut found = BacktickSpans { spans: Vec::new(), unclosed: false };
    if markup == Markup::Latex {
        return found;
    }
    let borrowed: std::collections::HashSet<usize> = if markup == Markup::Asciidoc {
        curved_quotes(chars)
            .into_iter()
            .flat_map(|(open, close)| [open, close])
            .collect()
    } else {
        std::collections::HashSet::new()
    };
    let is_mark = |at: usize| chars[at] == '`' && !borrowed.contains(&at);
    let run_at = |at: usize| (at..chars.len()).take_while(|&k| is_mark(k)).count();
    let collapse = |text: &[char]| {
        text.iter().collect::<String>().split_whitespace().collect::<Vec<_>>().join(" ")
    };

    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '\\' {
            i += 2;
            continue;
        }
        if !is_mark(i) {
            i += 1;
            continue;
        }
        let run = run_at(i);
        let literal = markup == Markup::Rst && run >= 2;
        let open_len = if literal { 2 } else { run };
        let mut at = i + run;
        // Where the closing marks start and where the span ends.
        let close = loop {
            if at >= chars.len() {
                break None;
            }
            if !is_mark(at) {
                at += 1;
                continue;
            }
            let found = run_at(at);
            if literal && found >= 2 {
                break Some((at + found - 2, at + found));
            }
            if !literal && found == run {
                break Some((at, at + found));
            }
            at += found;
        };
        let Some((close, end)) = close else {
            found.unclosed = true;
            i += run;
            continue;
        };
        let code = match markup {
            Markup::Markdown | Markup::MkDocs => i == 0 || chars[i - 1] != '}',
            Markup::Asciidoc => true,
            Markup::Rst => literal,
            Markup::Verso | Markup::Latex => false,
        };
        // RST does not nest inline markup, but a translation that moves a
        // literal into link text keeps the literal's content; count it there.
        if markup == Markup::Rst && !literal {
            let nested = backtick_spans(&chars[i + open_len..close], markup);
            found.spans.extend(nested.spans.into_iter().map(|span| BacktickSpan {
                range: span.range.start + i + open_len..span.range.end + i + open_len,
                ..span
            }));
        }
        found.spans.push(BacktickSpan {
            range: i..end,
            written: collapse(&chars[i..end]),
            content: collapse(&chars[i + open_len..close.max(i + open_len)]),
            code,
        });
        i = end;
    }
    found
}

/// The source's code spans that no translation span stands for, as written,
/// and the translation's code spans left over.
fn unmatched_code<'a>(
    source: &[BacktickSpan],
    translation: &'a [BacktickSpan],
) -> (Vec<String>, Vec<&'a BacktickSpan>) {
    let mut left: Vec<&BacktickSpan> = translation.iter().filter(|span| span.code).collect();
    // Exact copies first, so that a looser form does not take the span an
    // exact copy needs.
    let mut pending = Vec::new();
    for span in source.iter().filter(|span| span.code) {
        match left.iter().position(|written| written.content == span.content) {
            Some(at) => drop(left.remove(at)),
            None => pending.push(span),
        }
    }
    let mut missing = Vec::new();
    for span in pending {
        match left.iter().position(|written| stands_for(&written.content, &span.content)) {
            Some(at) => drop(left.remove(at)),
            None => missing.push(span.written.clone()),
        }
    }
    (missing, left)
}

/// Whether `source` writes `content` as a word of its own — not inside a
/// longer word — so a translation that marks it up as code adds nothing.
fn written_as_text(source: &str, content: &str) -> bool {
    let is_word = |c: Option<char>| c.is_some_and(|c| c.is_alphanumeric() || c == '_');
    !content.is_empty()
        && source.match_indices(content).any(|(at, _)| {
            !is_word(source[..at].chars().next_back())
                && !is_word(source[at + content.len()..].chars().next())
        })
}

/// Whether a translation may write `written` for the source's code `content`
/// by shedding what the Korean sentence around it carries instead.
///
/// Korean does not inflect, so an English plural or the call parentheses of a
/// function named in prose may go (``ParamSpecs``, ``tp_alloc()``); and so may
/// sentence punctuation or a parenthesis the source let slip inside its end
/// marks (``MyRing.``, ``PyObject_IsTrue())``). Nothing may be added: that is
/// how PEP 818's ``({next(){}})`` gained a `)`.
fn stands_for(written: &str, content: &str) -> bool {
    let plural = content.strip_suffix('s').is_some_and(|stem| {
        stem.chars().all(|c| c.is_ascii_alphabetic())
            && !stem.ends_with('s')
            // `ORs` and `VMs`, but not `is` or `has`.
            && (stem.len() >= 3 || (stem.len() == 2 && stem.chars().all(|c| c.is_ascii_uppercase())))
    });
    (plural && written == &content[..content.len() - 1])
        || content.strip_suffix("()") == Some(written)
        || written == shed_edges(content)
}

/// `content` without the sentence punctuation and unmatched parentheses at its
/// edges.
fn shed_edges(mut content: &str) -> &str {
    loop {
        let opens = content.matches('(').count();
        let closes = content.matches(')').count();
        content = if content.ends_with(['.', ',', ';', ':', '!', '?'])
            || (content.ends_with(')') && closes > opens)
        {
            &content[..content.len() - 1]
        } else if content.starts_with('(') && opens > closes {
            &content[1..]
        } else {
            return content;
        };
    }
}

/// Char ranges of the math in a LaTeX text: `$…$`, `$$…$$`, `\(…\)`, `\[…\]`.
fn latex_math_ranges(chars: &[char]) -> Vec<std::ops::Range<usize>> {
    let run_at = |at: usize| chars[at..].iter().take_while(|c| **c == '$').count();
    let mut found = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        match chars[i] {
            '\\' if matches!(chars.get(i + 1), Some('(' | '[')) => {
                let close = if chars[i + 1] == '(' { ')' } else { ']' };
                match (i + 2..chars.len().saturating_sub(1))
                    .find(|&at| chars[at] == '\\' && chars[at + 1] == close)
                {
                    Some(at) => {
                        found.push(i..at + 2);
                        i = at + 2;
                    }
                    None => i += 2,
                }
            }
            '\\' => i += 2,
            '$' => {
                let run = run_at(i);
                let mut at = i + run;
                let mut end = None;
                while at < chars.len() {
                    match chars[at] {
                        '\\' => at += 2,
                        '$' if run_at(at) == run => {
                            end = Some(at + run);
                            break;
                        }
                        '$' => at += run_at(at),
                        _ => at += 1,
                    }
                }
                match end {
                    Some(end) => {
                        found.push(i..end);
                        i = end;
                    }
                    None => i += run,
                }
            }
            _ => i += 1,
        }
    }
    found
}

/// The parenthesis pairs of the prose in `chars` as (open, close) indices,
/// and whether every parenthesis found its partner.
///
/// Code and math are left out. They are compared on their own, and Korean word
/// order moves them, which reorders their brackets: `$[0, 1)$` holds a `)`
/// that closes nothing.
fn prose_parentheses(chars: &[char], markup: Markup) -> (Vec<(usize, usize)>, bool) {
    let mut verbatim = vec![false; chars.len()];
    let ranges = backtick_spans(chars, markup)
        .spans
        .into_iter()
        .map(|span| span.range)
        .chain(if markup == Markup::Latex { latex_math_ranges(chars) } else { Vec::new() });
    for range in ranges {
        verbatim[range].fill(true);
    }
    let mut open = Vec::new();
    let mut pairs = Vec::new();
    let mut balanced = true;
    for (at, c) in chars.iter().enumerate() {
        match c {
            _ if verbatim[at] => {}
            // A smiley closes nothing — though PEP 102 lets `:-)` close its
            // own parenthetical, which a translation may well spell `:-))`.
            ')' if at >= 2 && matches!(chars[at - 2..at], [':' | ';', '-']) => {}
            '(' => open.push(at),
            ')' => match open.pop() {
                Some(from) => pairs.push((from, at)),
                None => balanced = false,
            },
            _ => {}
        }
    }
    (pairs, balanced && open.is_empty())
}

/// Every parenthesis pair in the prose of `text` that holds nothing but
/// another pair — `((…))` — as written.
fn doubled_parentheses(text: &str, markup: Markup) -> Vec<String> {
    let chars = chars(text);
    let (mut pairs, _) = prose_parentheses(&chars, markup);
    pairs.sort_unstable();
    let close_of: std::collections::HashMap<usize, usize> = pairs.iter().copied().collect();
    pairs
        .iter()
        .filter(|&&(open, close)| close_of.get(&(open + 1)) == Some(&(close - 1)))
        .map(|&(open, close)| chars[open..=close].iter().collect())
        .collect()
}

fn parentheses_balance(text: &str, markup: Markup) -> bool {
    prose_parentheses(&chars(text), markup).1
}

/// The marks that open a constrained pair, each with the unconstrained form to
/// rewrite it as.
///
/// Markdown shares the rule only for `_`, where CommonMark rules out intraword
/// emphasis. Its code spans and `*` emphasis close against a word just fine, so
/// checking them there would fail translations that are correct. And
/// `__italic__` is bold in Markdown, so the way out is the other mark.
fn constrained_marks(markup: Markup) -> &'static [(char, &'static str)] {
    match markup {
        Markup::Asciidoc => &[('`', "``code``"), ('*', "**bold**"), ('_', "__italic__")],
        Markup::Markdown | Markup::MkDocs | Markup::Verso => &[('_', "*italic*")],
        // reStructuredText pairs are checked by `rst_broken_pairs`: every one
        // of its marker forms is constrained, so the doubled-form advice these
        // entries carry would be wrong there.
        Markup::Rst | Markup::Latex => &[],
    }
}

fn latex_group_balance(tokens: &[String]) -> (i64, i64) {
    let (mut depth, mut minimum) = (0, 0);
    for token in tokens {
        match token.as_str() {
            "syntax:{" => depth += 1,
            "syntax:}" => depth -= 1,
            _ => {}
        }
        minimum = minimum.min(depth);
    }
    (depth, minimum)
}

/// LaTeX syntax whose spelling is independent of the visible prose around it.
///
/// The evaluator compares the resulting tokens as a multiset: Korean sentence
/// order may move complete commands and math spans, while changing or dropping
/// even one token (including one of two duplicates) is still rejected.
fn latex_structure(text: &str) -> Vec<String> {
    let masked = mask_latex_visible_text_arguments(text);
    let text = masked.as_str();
    let mut tokens = Vec::new();
    let bytes = text.as_bytes();
    let mut at = 0usize;
    while at < bytes.len() {
        if text[at..].starts_with('⟦')
            && let Some(close) = text[at..].find('⟧')
        {
            let end = at + close + '⟧'.len_utf8();
            tokens.push(format!("comment:{}", &text[at..end]));
            at = end;
            continue;
        }

        if bytes[at] == b'$' && !is_escaped(bytes, at) {
            let delimiter = if bytes.get(at + 1) == Some(&b'$') {
                "$$"
            } else {
                "$"
            };
            let start = at;
            at += delimiter.len();
            while at < bytes.len() {
                if text[at..].starts_with(delimiter) && !is_escaped(bytes, at) {
                    at += delimiter.len();
                    break;
                }
                at += text[at..].chars().next().map_or(1, char::len_utf8);
            }
            tokens.push(format!("math:{}", normalize_math_token(&text[start..at])));
            continue;
        }

        if text[at..].starts_with("\\(") || text[at..].starts_with("\\[") {
            let (close, width) = if text[at..].starts_with("\\(") {
                ("\\)", 2)
            } else {
                ("\\]", 2)
            };
            let start = at;
            at += width;
            if let Some(offset) = text[at..].find(close) {
                at += offset + close.len();
            } else {
                at = text.len();
            }
            tokens.push(format!("math:{}", normalize_math_token(&text[start..at])));
            continue;
        }

        if bytes[at] == b'\\' {
            let start = at;
            at += 1;
            if at < bytes.len() && (bytes[at].is_ascii_alphabetic() || bytes[at] == b'@') {
                while at < bytes.len() && (bytes[at].is_ascii_alphabetic() || bytes[at] == b'@') {
                    at += 1;
                }
            } else if at < bytes.len() {
                at += text[at..].chars().next().map_or(1, char::len_utf8);
            }
            let command = &text[start + 1..at];
            // A backslash followed by a space is a typographic interword-space
            // hint (most often after i.e.), not semantic document structure.
            // Korean normally drops it with the preceding Latin abbreviation.
            if command.chars().all(char::is_whitespace) {
                continue;
            }
            // TeX accent commands and dotless i/j spell visible Latin text.
            // Transliteration into Hangul legitimately removes both the accent
            // command and its local braces.
            if matches!(command, "i" | "j") {
                continue;
            }
            if latex_accent_command(command) {
                if let Some(end) = latex_argument_end(text, at, b'{', b'}') {
                    at = end;
                } else if at < bytes.len() {
                    if bytes[at] == b'\\' {
                        at += 1;
                        while at < bytes.len()
                            && (bytes[at].is_ascii_alphabetic() || bytes[at] == b'@')
                        {
                            at += 1;
                        }
                    } else {
                        at += text[at..].chars().next().map_or(1, char::len_utf8);
                    }
                }
                continue;
            }
            tokens.push(format!("command:{}", &text[start..at]));
            if let Some(arguments) = latex_reference_arguments(command) {
                while let Some(end) = latex_argument_end(text, at, b'[', b']') {
                    tokens.push(format!("opaque:{}", &text[at..end]));
                    at = end;
                }
                // Keep a range's endpoints together: sorting independent
                // argument tokens would accept a reversed reference range.
                let arguments_start = at;
                for _ in 0..arguments {
                    if let Some(end) = latex_argument_end(text, at, b'{', b'}') {
                        at = end;
                    }
                }
                if at > arguments_start {
                    tokens.push(format!("opaque:{}", &text[arguments_start..at]));
                }
            }
            continue;
        }

        let ch = text[at..].chars().next().unwrap();
        if matches!(ch, '{' | '}' | '[' | ']' | '&' | '#' | '~') {
            tokens.push(format!("syntax:{ch}"));
        }
        at += ch.len_utf8();
    }
    tokens
}

fn mask_latex_visible_text_arguments(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut output = String::with_capacity(text.len());
    let mut at = 0usize;
    while at < bytes.len() {
        if text[at..].starts_with("\\text") {
            let command_end = at + "\\text".len();
            let mut open = command_end;
            while bytes.get(open).is_some_and(u8::is_ascii_whitespace) {
                open += 1;
            }
            if bytes.get(open) == Some(&b'{')
                && let Some(end) = latex_argument_end(text, command_end, b'{', b'}')
            {
                output.push_str(&text[at..=open]);
                output.push_str("VISIBLE_PROSE");
                output.push('}');
                at = end;
                continue;
            }
        }
        let ch = text[at..].chars().next().unwrap();
        output.push(ch);
        at += ch.len_utf8();
    }
    output
}

fn normalize_math_token(token: &str) -> String {
    let (open, close) = if token.starts_with("$$") && token.ends_with("$$") {
        ("$$", "$$")
    } else if token.starts_with('$') && token.ends_with('$') {
        ("$", "$")
    } else if token.starts_with("\\(") && token.ends_with("\\)") {
        ("\\(", "\\)")
    } else if token.starts_with("\\[") && token.ends_with("\\]") {
        ("\\[", "\\]")
    } else {
        return token.to_string();
    };
    let inner = token[open.len()..token.len() - close.len()].trim_end();
    let inner = inner
        .strip_suffix(['.', ',', ';', ':', '?', '!'])
        .unwrap_or(inner)
        .trim_end();
    format!("{open}{inner}{close}")
}

fn latex_accent_command(command: &str) -> bool {
    matches!(
        command,
        "\"" | "'" | "^" | "~" | "=" | "b" | "c" | "d" | "H" | "k" | "r" | "t" | "u" | "v"
    )
}

/// Checks only MkDocs needs: math copied verbatim, and no new Jinja
/// delimiters (the macros plugin renders every page as a template).
fn mkdocs_issues(source: &str, translation: &str) -> Vec<EvaluationIssue> {
    let spans = |text: &str| -> Vec<String> {
        let mut v: Vec<String> = yeokja_parser_mkdocs::math_spans(text).into_iter().map(|r| text[r].to_string()).collect();
        v.sort_unstable();
        v
    };
    let (required, available) = (spans(source), spans(translation));
    let mut issues = Vec::new();
    if !is_multiset_subset(&required, &available) {
        issues.push(EvaluationIssue {
            severity: IssueSeverity::Error,
            kind: IssueKind::FormatLost,
            message: format!(
                "Math changed: copy every $...$, $$...$$, \\(...\\) and \\[...\\] expression \
                 byte-for-byte and translate nothing inside it; put Korean particles after the \
                 closing delimiter. Source math: {required:?}; translation math: {available:?}"
            ),
        });
    }
    // Repeating a source formula can suit Korean word order; a formula the
    // source never had is text turned into math (`O(N)` → `$O(N)$`, which a
    // navigation label prints with its dollars) or a sentence pulled in from
    // a neighbouring segment.
    let added: Vec<&String> = available.iter().filter(|m| !required.contains(m)).collect();
    if !added.is_empty() {
        issues.push(EvaluationIssue {
            severity: IssueSeverity::Error,
            kind: IssueKind::FormatLost,
            message: format!(
                "Math added: the translation has formulas the source sentence does not have \
                 {added:?}. Do not turn plain text into $...$ math, and translate only this \
                 sentence, not its neighbours."
            ),
        });
    }
    for delimiter in ["{{", "{%", "{#"] {
        if translation.matches(delimiter).count() > source.matches(delimiter).count() {
            issues.push(EvaluationIssue {
                severity: IssueSeverity::Error,
                kind: IssueKind::FormatLost,
                message: format!(
                    "`{delimiter}` added: MkDocs renders pages as Jinja templates, so the \
                     translation must not introduce `{{{{`, `{{%` or `{{#`."
                ),
            });
        }
    }
    issues
}

fn is_multiset_subset(required: &[String], available: &[String]) -> bool {
    let mut counts = std::collections::HashMap::<&str, usize>::new();
    for token in available {
        *counts.entry(token).or_default() += 1;
    }
    required.iter().all(|token| {
        let Some(count) = counts.get_mut(token.as_str()) else {
            return false;
        };
        if *count == 0 {
            return false;
        }
        *count -= 1;
        true
    })
}

fn is_escaped(bytes: &[u8], at: usize) -> bool {
    let mut slashes = 0usize;
    let mut cursor = at;
    while cursor > 0 && bytes[cursor - 1] == b'\\' {
        slashes += 1;
        cursor -= 1;
    }
    slashes % 2 == 1
}

fn latex_reference_arguments(command: &str) -> Option<usize> {
    match command {
        "crefrange" | "Crefrange" | "cpagerefrange" | "Cpagerefrange" => Some(2),
        "hyperref" => Some(0),       // optional target, then a visible label
        "narrowequation" => Some(1), // HoTT's inline/display math wrapper
        "Cref" | "cref" | "cite" | "citeauthor" | "citep" | "citet" | "eqref" | "include"
        | "includegraphics" | "input" | "label" | "symlabel" | "pageref" | "ref" | "url"
        | "href" => Some(1),
        _ => None,
    }
}

/// Reference keys and URL destinations are identifiers, not glossary prose.
/// Keep visible link labels and all surrounding text available for checking.
pub(crate) fn without_latex_reference_arguments(text: &str) -> String {
    let mut output = String::new();
    let mut at = 0;
    while at < text.len() {
        if text.as_bytes()[at] == b'\\' {
            let start = at;
            at += 1;
            while text.as_bytes().get(at).is_some_and(u8::is_ascii_alphabetic) {
                at += 1;
            }
            let command = &text[start + 1..at];
            if let Some(arguments) = latex_reference_arguments(command).or(match command {
                "index" | "indexdef" | "indexfoot" => Some(1),
                "indexsee" => Some(2),
                _ => None,
            }) {
                while let Some(end) = latex_argument_end(text, at, b'[', b']') {
                    at = end;
                }
                for _ in 0..arguments {
                    if let Some(end) = latex_argument_end(text, at, b'{', b'}') {
                        at = end;
                    }
                }
                output.push(' ');
            } else {
                output.push_str(&text[start..at]);
            }
            continue;
        }
        let ch = text[at..].chars().next().unwrap();
        output.push(ch);
        at += ch.len_utf8();
    }
    output
}

fn latex_argument_end(text: &str, mut at: usize, open: u8, close: u8) -> Option<usize> {
    let bytes = text.as_bytes();
    while bytes.get(at).is_some_and(u8::is_ascii_whitespace) {
        at += 1;
    }
    if bytes.get(at) != Some(&open) {
        return None;
    }
    let mut depth = 0usize;
    let mut cursor = at;
    while cursor < bytes.len() {
        if bytes[cursor] == open && !is_escaped(bytes, cursor) {
            depth += 1;
        } else if bytes[cursor] == close && !is_escaped(bytes, cursor) {
            depth -= 1;
            if depth == 0 {
                return Some(cursor + 1);
            }
        }
        cursor += 1;
    }
    None
}

/// Sorted structural tokens from Verso inline roles and math.
///
/// Sorting deliberately permits a natural sentence reordering. The tokens are
/// still a multiset, so dropping either of two identical references is caught.
fn verso_structure(text: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut at = 0;
    while at < text.len() {
        if text[at..].starts_with("$$`") || text[at..].starts_with("$`") {
            let prefix_len = if text[at..].starts_with("$$`") { 3 } else { 2 };
            let payload_start = at + prefix_len;
            if let Some(close) = text[payload_start..].find('`') {
                let end = payload_start + close + 1;
                tokens.push(format!("math:{}", &text[at..end]));
                at = end;
                continue;
            }
        }

        if text[at..].starts_with('{') {
            let name_start = at + 1;
            let starts_as_role = text[name_start..]
                .chars()
                .next()
                .is_some_and(|c| c.is_ascii_alphabetic());
            if starts_as_role && let Some(close_offset) = text[name_start..].find('}') {
                let header_end = name_start + close_offset + 1;
                let header = &text[at..header_end];
                if text[header_end..].starts_with('`') {
                    let payload_start = header_end + 1;
                    if let Some(close) = text[payload_start..].find('`') {
                        let end = payload_start + close + 1;
                        tokens.push(format!("role-code:{header}{}", &text[header_end..end]));
                        at = end;
                        continue;
                    }
                }
                if text[header_end..].starts_with('[') {
                    tokens.push(format!("role-label:{header}"));
                } else {
                    tokens.push(format!("role:{header}"));
                }
                at = header_end;
                continue;
            }
        }

        let ch = text[at..].chars().next().unwrap();
        at += ch.len_utf8();
    }
    tokens.sort();
    tokens
}

#[derive(Debug)]
struct VersoCodeToken {
    range: std::ops::Range<usize>,
    header: String,
    payload: String,
    raw: String,
}

/// Restore non-breaking spaces in opaque Verso role payloads.
///
/// Models commonly turn an invisible U+00A0 into U+0020 or the literal text
/// `\u{a0}`. The payload is code, not prose. When a translated role has the
/// same header and otherwise-identical payload, put back the exact source token
/// before format evaluation and persistence. Tokens are matched as a multiset
/// so natural sentence reordering remains allowed.
pub(crate) fn restore_verso_code_whitespace(source: &str, translation: &str) -> String {
    let source_tokens = verso_code_tokens(source);
    let translation_tokens = verso_code_tokens(translation);
    let mut used = vec![false; translation_tokens.len()];
    let mut replacements = Vec::new();

    for source_token in source_tokens
        .iter()
        .filter(|token| token.payload.contains('\u{a0}'))
    {
        let normalized_source = normalize_verso_code_whitespace(&source_token.payload);
        let Some((index, translated_token)) =
            translation_tokens
                .iter()
                .enumerate()
                .find(|(index, token)| {
                    !used[*index]
                        && token.header == source_token.header
                        && normalize_verso_code_whitespace(&token.payload) == normalized_source
                })
        else {
            continue;
        };
        used[index] = true;
        replacements.push((translated_token.range.clone(), source_token.raw.clone()));
    }

    let mut restored = translation.to_string();
    replacements.sort_by_key(|(range, _)| std::cmp::Reverse(range.start));
    for (range, source_token) in replacements {
        restored.replace_range(range, &source_token);
    }
    restored
}

fn normalize_verso_code_whitespace(payload: &str) -> String {
    payload.replace("\\u{a0}", " ").replace('\u{a0}', " ")
}

fn verso_code_tokens(text: &str) -> Vec<VersoCodeToken> {
    let mut tokens = Vec::new();
    let mut at = 0;
    while at < text.len() {
        if text[at..].starts_with('{') {
            let name_start = at + 1;
            let starts_as_role = text[name_start..]
                .chars()
                .next()
                .is_some_and(|ch| ch.is_ascii_alphabetic());
            if starts_as_role && let Some(close_offset) = text[name_start..].find('}') {
                let header_end = name_start + close_offset + 1;
                if text[header_end..].starts_with('`') {
                    let payload_start = header_end + 1;
                    if let Some(close) = text[payload_start..].find('`') {
                        let payload_end = payload_start + close;
                        let end = payload_end + 1;
                        tokens.push(VersoCodeToken {
                            range: at..end,
                            header: text[at..header_end].to_string(),
                            payload: text[payload_start..payload_end].to_string(),
                            raw: text[at..end].to_string(),
                        });
                        at = end;
                        continue;
                    }
                }
            }
        }
        let ch = text[at..].chars().next().unwrap();
        at += ch.len_utf8();
    }
    tokens
}

/// Every AsciiDoc curved-quote pair `text` opens but cannot close.
///
/// `"`term`"` is constrained like the rest, so a Korean particle against its
/// closing mark holds it open. Unlike `` `code` `` it has no doubled form to
/// escape to — the way out is to write “term” with the quotation marks
/// themselves, which is what the pair would have rendered as.
fn unclosable_quotes(text: &str) -> Vec<String> {
    let chars = chars(text);
    curved_quotes(&chars)
        .into_iter()
        .filter(|&(_, close)| chars.get(close + 2).is_some_and(|c| is_word(*c)))
        .map(|(open, close)| chars[open - 1..close + 2].iter().collect())
        .collect()
}

/// Every AsciiDoc curved-quote pair in `chars`, as the indices of the two
/// backticks it borrows.
///
/// The pair opens on `` "` `` and closes on the mirrored `` `" ``, and only
/// there — `'` behaves the same way for single quotes.
fn curved_quotes(chars: &[char]) -> Vec<(usize, usize)> {
    let mut found = Vec::new();
    let mut i = 0;
    while i + 1 < chars.len() {
        let quote = chars[i];
        if (quote != '"' && quote != '\'') || chars[i + 1] != '`' {
            i += 1;
            continue;
        }
        let mut at = i + 2;
        let mut close = None;
        while at + 1 < chars.len() {
            if chars[at] == '`' && chars[at + 1] == quote {
                close = Some(at);
                break;
            }
            at += 1;
        }
        match close {
            Some(close) => {
                found.push((i + 1, close));
                i = close + 2;
            }
            None => i += 1,
        }
    }
    found
}

/// Asciidoctor's `\p{Word}`, which every Hangul syllable satisfies.
fn is_word(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// Pair up runs of `mark` the way AsciiDoc reads them, and report each pair
/// that opens without being able to close — as the span of the pair plus the
/// character holding it open.
///
/// A run of two marks is the unconstrained form and pairs with another run of
/// two, anywhere. A lone mark is the constrained form and pairs with another
/// lone mark, but only when nothing sits against either end: not a word
/// character, not a quote, not another mark. So half-doubling a pair —
/// `` `Atom``이라는 `` — closes neither way, and that is exactly what a
/// translator reaches for first on being told to double the marks.
fn pair_up(chars: &[char], mark: char) -> Pairing {
    // Asciidoctor's own flanking test, which is why `'` is in here: it turns
    // `` `Atom`'s `` into a curly quote and no code span at all.
    let blocked = |c: char| c.is_alphanumeric() || c == '_' || c == '"' || c == '\'' || c == mark;
    let run_len = |at: usize| chars[at..].iter().take_while(|c| **c == mark).count();

    let mut pairing = Pairing::default();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] != mark {
            i += 1;
            continue;
        }
        let open_len = run_len(i);
        let after_open = i + open_len;
        // A lone mark with a word against its left opens nothing: the
        // underscores in `min_heap_size` are text, not markup.
        let opens = open_len > 1
            || ((i == 0 || !blocked(chars[i - 1]))
                && chars.get(after_open).is_some_and(|c| !c.is_whitespace()));
        if !opens {
            i = after_open;
            continue;
        }
        // Look for a run that can close this one. AsciiDoc keeps looking past
        // a candidate it cannot use, which is why `_temp_alloc_ is` closes on
        // the trailing underscore rather than giving up on the one inside the
        // identifier. Failing to look past it called the English source broken
        // too, and a check that rejects the source rejects nothing.
        let mut from = after_open;
        let mut first_candidate = None;
        let closes_at = loop {
            let Some(offset) = chars[from..].iter().position(|c| *c == mark) else {
                break None;
            };
            let close = from + offset;
            let close_len = run_len(close);
            first_candidate.get_or_insert(close + close_len);
            let usable = close_len == open_len
                && (open_len > 1
                    || (!chars[close - 1].is_whitespace()
                        && !chars.get(close + 1).is_some_and(|c| blocked(*c))));
            if usable {
                break Some(close + close_len);
            }
            from = close + close_len;
        };
        match (closes_at, first_candidate) {
            (Some(end), _) => {
                pairing.formed += 1;
                i = end;
            }
            // A mark with nothing after it to pair with is text, not markup.
            (None, None) => break,
            (None, Some(end)) => {
                pairing.unclosable.push(i..(end + 1).min(chars.len()));
                i = end;
            }
        }
    }
    pairing
}

/// Every reStructuredText inline pair `text` writes against a word character.
///
/// Docutils recognizes an opening marker only when whitespace or punctuation
/// precedes it, and a closing one only when whitespace or punctuation follows
/// it — and a Hangul syllable is a letter. The natural translation of "on the
/// ``heap``" ends `` ``heap``에 ``, which renders with both marker runs
/// printed as themselves. Unlike AsciiDoc there is no unconstrained form:
/// doubling is a different construct under the same rule. The way out is a
/// backslash-escaped space (`` ``heap``\ 에 ``), which renders as nothing.
///
/// Reference suffixes (`` `name`_ ``, `` `name`__ ``) belong to the closing
/// marker, so the rule applies after the underscores.
fn rst_literal_role_spans(text: &str) -> Vec<std::ops::Range<usize>> {
    let chars = chars(text);
    let mut spans = Vec::new();
    let mut at = 2usize;
    while at < chars.len() {
        if chars[at] != ':' || chars[at - 2..at] != ['`', '`'] {
            at += 1;
            continue;
        }
        let Some(open_offset) = chars[at + 1..]
            .iter()
            .position(|ch| *ch == '`' || ch.is_whitespace())
        else {
            break;
        };
        let open = at + 1 + open_offset;
        if chars[open] != '`' || chars.get(open.wrapping_sub(1)) != Some(&':') {
            at = open + 1;
            continue;
        }
        let Some(close_offset) = chars[open + 1..].iter().position(|ch| *ch == '`') else {
            break;
        };
        let close = open + 1 + close_offset;
        if chars.get(close + 1) == Some(&'`') && chars.get(close + 2) == Some(&'`') {
            spans.push(at - 2..close + 3);
            at = close + 3;
        } else {
            at = close + 1;
        }
    }
    spans
}

/// Whether `text` begins inside emphasis an earlier segment opened, as in
/// `It* is *true…`: its first `*` run closes — text against its left, none
/// against its right — and a later one opens again. A lone star, `PyObject*`,
/// `(*)` or an escaped `\*`, is not that.
fn rst_starts_inside_emphasis(text: &str) -> bool {
    let chars = chars(text);
    let backtick_mask = rst_backtick_mask(&chars);
    let star =
        |at: usize| chars[at] == '*' && !backtick_mask[at] && (at == 0 || chars[at - 1] != '\\');
    let Some(at) = (0..chars.len()).find(|&at| star(at)) else {
        return false;
    };
    let end = at + chars[at..].iter().take_while(|c| **c == '*').count();
    let closes =
        at > 0 && !chars[at - 1].is_whitespace() && chars.get(end).is_none_or(|c| !is_word(*c));
    let opens_later = (end..chars.len()).any(|later| {
        star(later)
            && chars[later - 1].is_whitespace()
            && chars
                .get(later + 1)
                .is_some_and(|c| !c.is_whitespace() && *c != '*')
    });
    closes && opens_later
}

fn rst_broken_pairs(text: &str) -> Vec<String> {
    rst_broken_pairs_of(text, &['`', '*'])
}

/// Every reStructuredText pair of the given marks `text` writes against a word
/// character.
fn rst_broken_pairs_of(text: &str, marks: &[char]) -> Vec<String> {
    let chars = chars(text);
    let backtick_mask = rst_backtick_mask(&chars);
    let run_len = |at: usize, mark: char| chars[at..].iter().take_while(|c| **c == mark).count();
    let mut found = Vec::new();

    for &mark in marks {
        // (start index, run length) of the currently open marker, if any.
        let mut open: Option<(usize, usize)> = None;
        let mut i = 0;
        while i < chars.len() {
            if chars[i] != mark || (mark == '*' && backtick_mask[i]) {
                i += 1;
                continue;
            }
            let len = run_len(i, mark);
            match open {
                None => {
                    // An opener needs text right after it; a run followed by
                    // whitespace is prose (`2 * 3`).
                    if !chars.get(i + len).is_some_and(|c| !c.is_whitespace()) {
                        i += len;
                        continue;
                    }
                    let blocked = i > 0 && is_word(chars[i - 1]);
                    if !blocked {
                        open = Some((i, len));
                    } else if chars[i + len..].contains(&mark) {
                        // A word glued to the front (`실행**될 수 있는**`)
                        // keeps the pair from opening — but only call it a
                        // pair when a partner run exists; `2*3` is arithmetic.
                        let end = (i + len + 1).min(chars.len());
                        found.push(chars[i.saturating_sub(1)..end].iter().collect());
                    }
                    i += len;
                }
                Some((oi, olen)) => {
                    // Only a run of the opener's own length closes it: `` and
                    // ` are different constructs in reStructuredText.
                    if len != olen {
                        i += len;
                        continue;
                    }
                    if chars[i - 1].is_whitespace() {
                        found.push(chars[oi..i + len].iter().collect());
                        open = None;
                        i += len;
                        continue;
                    }
                    // `_` and `__` after a closing backtick are the reference
                    // suffix; the rule applies to the character after them.
                    let mut end = i + len;
                    if mark == '`' {
                        while chars.get(end).is_some_and(|c| *c == '_') {
                            end += 1;
                        }
                    }
                    // Docutils accepts whitespace or closing punctuation after
                    // an end-string; a word character or an opening bracket
                    // (`` ``x``(y) ``, dropped space and all) blocks it.
                    if chars
                        .get(end)
                        .is_some_and(|c| is_word(*c) || matches!(c, '(' | '[' | '{' | '<'))
                    {
                        found.push(chars[oi..end + 1].iter().collect());
                    }
                    open = None;
                    i += len;
                }
            }
        }
    }
    for span in rst_literal_role_spans(text) {
        if chars
            .get(span.end)
            .is_some_and(|ch| is_word(*ch) || matches!(ch, '(' | '[' | '{' | '<'))
        {
            found.push(chars[span.start..=span.end].iter().collect());
        }
    }
    found
}

fn rst_backtick_mask(chars: &[char]) -> Vec<bool> {
    let mut mask = vec![false; chars.len()];
    let mut at = 0usize;
    while at < chars.len() {
        if chars[at] != '`' {
            at += 1;
            continue;
        }
        let run = chars[at..].iter().take_while(|ch| **ch == '`').count();
        let mut scan = at + run;
        let close = loop {
            let Some(offset) = chars[scan..].iter().position(|ch| *ch == '`') else {
                break None;
            };
            let candidate = scan + offset;
            let candidate_run = chars[candidate..]
                .iter()
                .take_while(|ch| **ch == '`')
                .count();
            if candidate_run == run {
                break Some(candidate + run);
            }
            scan = candidate + candidate_run;
        };
        let Some(end) = close else {
            at += run;
            continue;
        };
        mask[at..end].fill(true);
        at = end;
    }
    mask
}

/// Footnote and citation references whose trailing underscore touches a word.
///
/// A Korean particle naturally produces ``[2]_에서`` or ``[RFC]_는``. As with
/// other RST inline markup, the adjacent word character prevents docutils from
/// recognizing the reference. A backslash-escaped space keeps the source valid
/// without adding visible whitespace: ``[2]_\ 에서``.
fn rst_broken_bracket_references(text: &str) -> Vec<String> {
    let chars = chars(text);
    let mut found = Vec::new();
    let mut at = 0usize;

    while at < chars.len() {
        if chars[at] != '[' {
            at += 1;
            continue;
        }
        let Some(close_offset) = chars[at + 1..]
            .iter()
            .position(|ch| *ch == ']' || *ch == '\n')
        else {
            break;
        };
        let close = at + 1 + close_offset;
        if chars[close] == '\n' {
            at = close + 1;
            continue;
        }
        let underscore = close + 1;
        let after = underscore + 1;
        if chars.get(underscore) == Some(&'_') {
            if at > 0 && is_word(chars[at - 1]) {
                found.push(chars[at - 1..=underscore].iter().collect());
            }
            if chars
                .get(after)
                .is_some_and(|ch| is_word(*ch) || matches!(ch, '(' | '[' | '{' | '<'))
            {
                found.push(chars[at..=after].iter().collect());
            }
        }
        at = close + 1;
    }
    found
}

fn rst_malformed_role_closures(text: &str) -> Vec<String> {
    let chars = chars(text);
    let mut found = Vec::new();
    let mut at = 0usize;
    while at < chars.len() {
        if chars[at] != ':' {
            at += 1;
            continue;
        }
        let Some(open_offset) = chars[at + 1..]
            .iter()
            .position(|ch| *ch == '`' || ch.is_whitespace())
        else {
            break;
        };
        let open = at + 1 + open_offset;
        if chars[open] != '`' || chars.get(open.wrapping_sub(1)) != Some(&':') {
            at = open + 1;
            continue;
        }
        let Some(close_offset) = chars[open + 1..].iter().position(|ch| *ch == '`') else {
            break;
        };
        let close = open + 1 + close_offset;
        if chars.get(close + 1) == Some(&'`') {
            found.push(chars[at..=close + 1].iter().collect());
        }
        at = close + 1;
    }
    found
}

/// Insert invisible RST boundaries where Korean particles touch inline markup.
///
/// This is a deterministic typography repair, not a translation decision. It
/// runs before evaluation so a model does not spend retries rediscovering the
/// same ``\\ `` escape for every literal, role, emphasis span, footnote, and
/// citation. If the source segment itself looks structurally incomplete (for
/// example because a literal spans two parser segments), that class of repair
/// is skipped rather than guessing at a segment boundary.
pub(crate) fn repair_rst_boundaries(source: &str, translation: &str) -> String {
    let repaired_line_start;
    let translation = if line_start_construct(source, Markup::Rst).is_none()
        && line_start_construct(translation, Markup::Rst) == Some("an attribute entry (`:name:`)")
    {
        // A leading field marker opens a field list; a backslash keeps it
        // prose and renders as nothing.
        let trimmed = translation.trim_start();
        let indent_len = translation.len() - trimmed.len();
        repaired_line_start = format!("{}\\{trimmed}", &translation[..indent_len]);
        repaired_line_start.as_str()
    } else {
        translation
    };

    let chars = chars(translation);
    let backtick_mask = rst_backtick_mask(&chars);
    let mut insertions = std::collections::BTreeSet::new();
    let mut removals = std::collections::BTreeSet::new();
    let mut spaces = std::collections::BTreeSet::new();

    for token in source.split_whitespace() {
        let url = token
            .trim_start_matches(['(', '[', '{', '<', '\'', '"'])
            .trim_end_matches(['.', ',', ';', ':', ')', ']', '}', '>', '\'', '"']);
        if !url.starts_with("http://") && !url.starts_with("https://") {
            continue;
        }
        for (byte_at, _) in translation.match_indices(url) {
            let byte_end = byte_at + url.len();
            // A particle is Korean; an ASCII letter goes on a longer URL that
            // starts the same way (`…/decimal/` in `…/decimal/decarith.html`).
            if translation[byte_end..]
                .chars()
                .next()
                .is_some_and(|c| is_word(c) && !c.is_ascii())
            {
                insertions.insert(translation[..byte_end].chars().count());
            }
        }
    }

    if rst_broken_bracket_references(source).is_empty() {
        let mut at = 0usize;
        while at < chars.len() {
            if chars[at] != '[' {
                at += 1;
                continue;
            }
            let Some(close_offset) = chars[at + 1..]
                .iter()
                .position(|ch| *ch == ']' || *ch == '\n')
            else {
                break;
            };
            let close = at + 1 + close_offset;
            if chars[close] == '\n' {
                at = close + 1;
                continue;
            }
            let underscore = close + 1;
            let after = underscore + 1;
            if chars.get(underscore) == Some(&'_') {
                if at > 0 && is_word(chars[at - 1]) {
                    spaces.insert(at);
                }
                if chars
                    .get(after)
                    .is_some_and(|ch| is_word(*ch) || matches!(ch, '(' | '[' | '{' | '<'))
                {
                    insertions.insert(after);
                }
            }
            at = close + 1;
        }
    }

    if rst_broken_pairs(source).is_empty() {
        for span in rst_literal_role_spans(translation) {
            if chars
                .get(span.end)
                .is_some_and(|ch| is_word(*ch) || matches!(ch, '(' | '[' | '{' | '<'))
            {
                insertions.insert(span.end);
            }
        }
        let run_len =
            |at: usize, mark: char| chars[at..].iter().take_while(|c| **c == mark).count();
        // A closer the segment never saw open is not an opener to repair.
        let marks: &[char] = if rst_starts_inside_emphasis(source) {
            &['`']
        } else {
            &['`', '*']
        };
        for &mark in marks {
            let mut open: Option<(usize, usize)> = None;
            let mut at = 0usize;
            while at < chars.len() {
                if chars[at] != mark || (mark == '*' && backtick_mask[at]) {
                    at += 1;
                    continue;
                }
                let len = run_len(at, mark);
                match open {
                    None => {
                        if !chars.get(at + len).is_some_and(|ch| !ch.is_whitespace()) {
                            at += len;
                            continue;
                        }
                        let blocked = at > 0 && is_word(chars[at - 1]);
                        if !blocked {
                            open = Some((at, len));
                        } else if chars[at + len..].contains(&mark) {
                            insertions.insert(at);
                            open = Some((at, len));
                        }
                        at += len;
                    }
                    Some((_, open_len)) => {
                        if len != open_len {
                            at += len;
                            continue;
                        }
                        if chars[at - 1].is_whitespace() {
                            let mut before = at;
                            while before > 0
                                && chars[before - 1].is_whitespace()
                                && chars[before - 1] != '\n'
                            {
                                before -= 1;
                                removals.insert(before);
                            }
                            open = None;
                            at += len;
                            continue;
                        }
                        let mut end = at + len;
                        if mark == '`' {
                            while chars.get(end).is_some_and(|ch| *ch == '_') {
                                end += 1;
                            }
                        }
                        if chars
                            .get(end)
                            .is_some_and(|ch| is_word(*ch) || matches!(ch, '(' | '[' | '{' | '<'))
                        {
                            insertions.insert(end);
                        }
                        open = None;
                        at += len;
                    }
                }
            }
        }
    }

    if insertions.is_empty() && removals.is_empty() && spaces.is_empty() {
        return translation.to_string();
    }
    let mut repaired =
        String::with_capacity(translation.len() + insertions.len() * 2 + spaces.len());
    for boundary in 0..=chars.len() {
        if spaces.contains(&boundary) {
            repaired.push(' ');
        } else if insertions.contains(&boundary) {
            repaired.push_str("\\ ");
        }
        if let Some(ch) = chars.get(boundary)
            && !removals.contains(&boundary)
        {
            repaired.push(*ch);
        }
    }
    repaired
}

/// The block construct `text` would open if it sat at the start of a line, or
/// `None` for ordinary prose.
///
/// Translations are spliced in at the position their source occupied, which for
/// a paragraph or a list item is the first non-space character of a line. A
/// translation that opens a construct its source did not restructures the
/// document silently: the paragraph becomes a block title, a heading, a table
/// cell. Nothing downstream can tell that apart from markup an author wrote.
///
/// Most names hold in every markup; a leading `.Word` is a block title in
/// AsciiDoc alone.
fn line_start_construct(text: &str, markup: Markup) -> Option<&'static str> {
    let text = text.trim_start();
    // `..` opens a comment, directive, or hyperlink target in
    // reStructuredText, and a nested ordered list in AsciiDoc; `...` is an
    // ellipsis and reads as prose.
    if text == ".." || text.starts_with(".. ") {
        return Some("an explicit-markup start (`..`)");
    }
    let mut chars = text.chars();
    let first = chars.next()?;
    let rest = chars.as_str();
    let spaced = rest.starts_with([' ', '\t']);
    match first {
        // `.Title` names the block below it; `...` is an ellipsis.
        '.' if markup == Markup::Asciidoc
            && !rest.is_empty()
            && !rest.starts_with(['.', ' ', '\t']) =>
        {
            Some("a block title (`.`)")
        }
        '*' | '-' | '+' if spaced => Some("a list item"),
        '=' if spaced => Some("a section title (`=`)"),
        '#' if spaced => Some("a heading (`#`)"),
        '>' if spaced => Some("a block quote (`>`)"),
        '|' => Some("a table cell (`|`)"),
        '/' if rest.starts_with('/') => Some("a comment (`//`)"),
        '[' if text.ends_with(']') => Some("an attribute line (`[...]`)"),
        // A space or the line end follows the colon closing an attribute
        // entry or a field name; a backtick there makes it a role, `:pep:`8``.
        ':' => {
            for (at, _) in rest.match_indices(':') {
                let after = &rest[at + 1..];
                if after.starts_with('`') {
                    return None;
                }
                if at > 0 && (after.is_empty() || after.starts_with([' ', '\t'])) {
                    return Some("an attribute entry (`:name:`)");
                }
            }
            None
        }
        _ => None,
    }
}

/// Backtick runs used by an RST named reference are hyperlink syntax, not
/// inline code markers. This covers both `label`_ and a translated visible
/// label written as `번역 <label_>`_.
fn rst_reference_runs(text: &str) -> usize {
    let chars = chars(text);
    let mut runs = 0;
    let mut at = 0;
    while at < chars.len() {
        if chars[at] != '`'
            || chars.get(at.wrapping_sub(1)) == Some(&'`')
            || chars.get(at + 1) == Some(&'`')
        {
            at += 1;
            continue;
        }
        let mut scan = at + 1;
        let mut close = None;
        while scan < chars.len() {
            if chars[scan] != '`' {
                scan += 1;
                continue;
            }
            let run = chars[scan..].iter().take_while(|ch| **ch == '`').count();
            if run > 1 {
                scan += run;
                continue;
            }
            if chars.get(scan + 1) == Some(&'_') {
                close = Some(scan);
            }
            scan += 1;
            break;
        }
        if close.is_some() {
            runs += 2;
        }
        at = scan;
    }
    runs
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn make_context(source: &str, translation: &str) -> EvaluationContext {
        context_in(Markup::Asciidoc, source, translation)
    }

    fn context_in(markup: Markup, source: &str, translation: &str) -> EvaluationContext {
        EvaluationContext {
            source: source.to_string(),
            translation: translation.to_string(),
            glossary: HashMap::new(),
            source_lang: "en".to_string(),
            target_lang: "ko".to_string(),
            markup,
        }
    }

    #[tokio::test]
    async fn passes_when_formatting_preserved() {
        let ctx = make_context(
            "This is **bold** and `code`.",
            "이것은 **굵게** 그리고 `code`.",
        );
        let result = FormatEvaluator.evaluate(&ctx).await.unwrap();
        assert!(result.passed);
    }

    #[tokio::test]
    async fn fails_when_bold_lost() {
        let ctx = make_context("This is **bold**.", "이것은 굵게.");
        let result = FormatEvaluator.evaluate(&ctx).await.unwrap();
        assert!(!result.passed);
    }

    /// The case this was written for: "The .erlang.crypt file should contain…"
    /// came back starting with the filename, and asciidoctor read the whole
    /// paragraph as the title of the block below it.
    #[tokio::test]
    async fn fails_when_the_translation_opens_a_block_title() {
        let ctx = make_context(
            "The .erlang.crypt file should contain a list of tuples.",
            ".erlang.crypt 파일은 튜플 목록을 포함해야 합니다.",
        );
        let result = FormatEvaluator.evaluate(&ctx).await.unwrap();
        assert!(!result.passed);
    }

    #[tokio::test]
    async fn allows_a_construct_the_source_already_had() {
        // A list item's span excludes its marker, so both sides start with one
        // only when the text itself does.
        let ctx = make_context("| Instruction", "| 명령어");
        assert!(FormatEvaluator.evaluate(&ctx).await.unwrap().passed);
    }

    #[tokio::test]
    async fn ordinary_prose_is_not_markup() {
        for (source, translation) in [
            ("Wait for it...", "기다려 보세요..."),
            ("It costs -5 dollars.", "-5달러입니다."),
            ("Ratio 3:1 applies.", "비율 3:1이 적용됩니다."),
        ] {
            let ctx = make_context(source, translation);
            let result = FormatEvaluator.evaluate(&ctx).await.unwrap();
            assert!(result.passed, "{translation:?} should read as prose");
        }
    }

    /// Only AsciiDoc reads `.Word` as a block title; elsewhere `.NET` at the
    /// start of a line is prose.
    #[tokio::test]
    async fn a_leading_dot_is_prose_outside_asciidoc() {
        for markup in [Markup::Markdown, Markup::Rst] {
            let ctx = context_in(
                markup,
                "Once you have the .NET SDK installed, create a new project:",
                ".NET SDK 설치를 완료했다면, 다음과 같이 새 프로젝트를 생성하십시오:",
            );
            let result = FormatEvaluator.evaluate(&ctx).await.unwrap();
            assert!(result.passed, "{markup:?}: {:?}", result.issues);
        }
    }

    /// A role is not a field: the colon closing its name is followed by a
    /// backtick, where a field or attribute entry needs a space or the line end.
    #[tokio::test]
    async fn a_role_at_line_start_is_prose() {
        let ctx = context_in(
            Markup::Rst,
            "During the discussion of :pep:`340`, I maintained drafts of this PEP.",
            ":pep:`340` 논의 중에 저는 이 PEP의 초안을 관리했습니다.",
        );
        let result = FormatEvaluator.evaluate(&ctx).await.unwrap();
        assert!(result.passed, "{:?}", result.issues);
    }

    #[test]
    fn constructs_are_recognised_at_line_start() {
        assert_eq!(
            line_start_construct("= Title", Markup::Asciidoc),
            Some("a section title (`=`)")
        );
        assert_eq!(
            line_start_construct("* item", Markup::Asciidoc),
            Some("a list item")
        );
        assert_eq!(
            line_start_construct("[source,erlang]", Markup::Asciidoc),
            Some("an attribute line (`[...]`)")
        );
        assert_eq!(
            line_start_construct(":toc: left", Markup::Asciidoc),
            Some("an attribute entry (`:name:`)")
        );
        assert_eq!(
            line_start_construct(":Contact person:", Markup::Asciidoc),
            Some("an attribute entry (`:name:`)")
        );
        assert_eq!(
            line_start_construct("// note", Markup::Asciidoc),
            Some("a comment (`//`)")
        );
        assert_eq!(
            line_start_construct("보통 문장입니다.", Markup::Asciidoc),
            None
        );
        assert_eq!(line_start_construct("3.14 입니다.", Markup::Asciidoc), None);
    }

    /// The case this was written for. 858 of theBeamBook's 1974 code spans came
    /// back with a particle against the closing backtick and stopped rendering.
    #[tokio::test]
    async fn fails_when_a_particle_closes_a_code_span() {
        let ctx = make_context(
            "Perform `is_integer` on x0 and jump to the label on failure.",
            "x0에 대해 `is_integer`를 수행하고, 실패하면 레이블로 점프합니다.",
        );
        let result = FormatEvaluator.evaluate(&ctx).await.unwrap();
        assert!(!result.passed);
        assert!(
            result.issues.iter().any(|i| i.message.contains("``code``")),
            "the message should name the form that works: {:?}",
            result.issues
        );
    }

    #[tokio::test]
    async fn passes_when_the_pair_is_doubled() {
        let ctx = make_context(
            "Perform `is_integer` on x0.",
            "x0에 대해 ``is_integer``를 수행합니다.",
        );
        assert!(FormatEvaluator.evaluate(&ctx).await.unwrap().passed);
    }

    #[tokio::test]
    async fn a_markdown_code_span_closes_against_a_particle() {
        // CommonMark has no flanking rule for code spans, so this renders.
        let ctx = context_in(
            Markup::Markdown,
            "Perform `is_integer` on x0.",
            "x0에 대해 `is_integer`를 수행합니다.",
        );
        assert!(FormatEvaluator.evaluate(&ctx).await.unwrap().passed);
    }

    #[tokio::test]
    async fn a_markdown_underscore_still_has_to_close() {
        // CommonMark does rule out intraword `_` emphasis, and points elsewhere.
        let ctx = context_in(
            Markup::Markdown,
            "The _arity_ is the argument count.",
            "_arity_는 인자의 개수입니다.",
        );
        let result = FormatEvaluator.evaluate(&ctx).await.unwrap();
        assert!(!result.passed);
        assert!(result.issues.iter().any(|i| i.message.contains("*italic*")));
    }

    #[tokio::test]
    async fn markdown_allows_underscore_emphasis_to_become_stars_before_a_particle() {
        let ctx = context_in(
            Markup::Markdown,
            "The _arity_ is the argument count.",
            "인자 개수는 *arity*입니다.",
        );
        let result = FormatEvaluator.evaluate(&ctx).await.unwrap();
        assert!(result.passed, "{:?}", result.issues);
    }

    #[tokio::test]
    async fn verso_allows_underscore_emphasis_to_become_stars_before_a_particle() {
        let ctx = context_in(
            Markup::Verso,
            "It serves as _data_ for the lookup.",
            "조회에 사용할 *데이터*로서 역할을 합니다.",
        );
        let result = FormatEvaluator.evaluate(&ctx).await.unwrap();
        assert!(result.passed, "{:?}", result.issues);
    }

    #[test]
    fn an_identifier_is_not_an_unclosed_italic() {
        for text in [
            "off_heap 할당을 사용합니다.",
            "https://example.com/a_b_c 를 보세요.",
            "인자 수는 2*3 입니다.",
            "함수 인자는 `Arity` 입니다",
            "여는 백틱만 있는 `문장",
        ] {
            assert!(
                unclosable_pairs(text, Markup::Asciidoc).is_empty(),
                "{text:?} carries no unclosable pair"
            );
        }
    }

    /// A segment usually carries several marked-up terms and only some of them
    /// take a suffix. Naming one and stopping got the first fixed and the rest
    /// left alone, so every offender is reported.
    #[test]
    fn every_unclosable_pair_is_named() {
        let text = "이 명령어는 `allocate`와 같지만 스택 슬롯을 `NIL`로 지웁니다.";
        let found = unclosable_pairs(text, Markup::Asciidoc);
        let shown: Vec<&str> = found.iter().map(|u| u.text.as_str()).collect();
        assert_eq!(shown, vec!["`allocate`와", "`NIL`로"]);
    }

    /// Doubling only one end is the first thing a translator tries on being
    /// told to double the marks, and it renders as nothing either way.
    #[test]
    fn a_half_doubled_pair_closes_neither_way() {
        for text in [
            "청크 `Atom``이라는 이름을 씁니다.",
            "청크 ``Atom`이라는 이름을 씁니다.",
        ] {
            assert!(
                !unclosable_pairs(text, Markup::Asciidoc).is_empty(),
                "{text:?} closes neither as a constrained nor an unconstrained pair"
            );
        }
    }

    /// AsciiDoc looks past a closer it cannot use. `_temp_alloc_` closes on
    /// its trailing underscore, not on the one inside the identifier — so the
    /// English reads fine and only the Korean, which glues 은 to the end, does
    /// not. Calling both broken would have switched the check off for the pair.
    #[test]
    fn a_closer_is_sought_past_the_one_that_cannot_close() {
        assert!(
            unclosable_pairs("The allocator _temp_alloc_ is used.", Markup::Asciidoc).is_empty()
        );
        assert!(
            !unclosable_pairs(
                "할당자 _temp_alloc_은 임시 할당에 씁니다.",
                Markup::Asciidoc
            )
            .is_empty()
        );
    }

    /// Asciidoctor blocks a closing mark on a quote as well as on a letter.
    #[test]
    fn a_quote_against_the_closing_mark_blocks_it() {
        assert!(!unclosable_pairs("the `Atom`\"quoted\" chunk", Markup::Asciidoc).is_empty());
        assert!(unclosable_pairs("the `Atom`. chunk", Markup::Asciidoc).is_empty());
    }

    #[tokio::test]
    async fn the_message_quotes_the_offending_text() {
        let ctx = make_context(
            "It works as `allocate` but clears the slots to `NIL`.",
            "`allocate`와 같지만 슬롯을 `NIL`로 지웁니다.",
        );
        let result = FormatEvaluator.evaluate(&ctx).await.unwrap();
        let named = result
            .issues
            .iter()
            .find(|i| i.message.contains("never closes"))
            .expect("the unclosable pair should be reported");
        assert!(named.message.contains("`allocate`와"), "{}", named.message);
        assert!(named.message.contains("`NIL`로"), "{}", named.message);
    }

    #[tokio::test]
    async fn fails_when_code_lost() {
        let ctx = make_context("Use `func()` here.", "여기서 func()를 사용하세요.");
        let result = FormatEvaluator.evaluate(&ctx).await.unwrap();
        assert!(!result.passed);
    }

    /// Shipped in theBeamBook and read as correct by every earlier check: the
    /// first pair cannot close against 의, so Asciidoctor closes it on the mark
    /// that should have opened the second one. One emphasis covers
    /// `erl_process.c_의 _check_balance`, and both marks stay unclosed and
    /// nothing is missing, so neither the run count nor the unclosable check
    /// sees it.
    #[tokio::test]
    async fn fails_when_a_pair_closes_on_a_later_mark_and_swallows_the_prose() {
        let ctx = make_context(
            "This is done by the function _check_balance_ in _erl_process.c_.",
            "이는 _erl_process.c_의 _check_balance_ 함수에 의해 수행됩니다.",
        );
        let result = FormatEvaluator.evaluate(&ctx).await.unwrap();
        assert!(!result.passed);
        assert!(
            result
                .issues
                .iter()
                .any(|i| i.message.contains("do not line up")),
            "{:?}",
            result.issues
        );
    }

    /// The same thing happens to code spans, and costs more: this one shipped
    /// in theBeamBook as a single span reading `io:format`은 … `recon_trace`,
    /// with the prose between it set as code.
    #[tokio::test]
    async fn fails_when_a_code_span_closes_on_a_later_mark() {
        let ctx = make_context(
            "`io:format` offers a quick method, whereas `erl_tracer` and `recon_trace` \
             provide deeper insights.",
            "`io:format`은 빠른 방법을 제공하는 반면, `erl_tracer`와 `recon_trace` 같은 \
             도구는 더 깊은 통찰을 제공합니다.",
        );
        let result = FormatEvaluator.evaluate(&ctx).await.unwrap();
        assert!(!result.passed);
        assert!(
            result
                .issues
                .iter()
                .any(|i| i.message.contains("do not line up")),
            "{:?}",
            result.issues
        );
    }

    /// The same segment written the way it has to be written passes.
    #[tokio::test]
    async fn passes_when_both_ends_are_doubled() {
        let ctx = make_context(
            "This is done by the function _check_balance_ in _erl_process.c_.",
            "이는 __erl_process.c__의 __check_balance__ 함수에 의해 수행됩니다.",
        );
        let result = FormatEvaluator.evaluate(&ctx).await.unwrap();
        assert!(result.passed, "{:?}", result.issues);
    }

    /// A mark with a word against its left opens nothing, so this emphasis is
    /// never opened rather than never closed — the asterisks print as themselves.
    #[tokio::test]
    async fn fails_when_a_suffix_swallows_the_opening_mark() {
        let ctx = make_context(
            "two or more processes that *can* execute independently",
            "서로 독립적으로 실행*될 수 있는* 둘 이상의 프로세스",
        );
        let result = FormatEvaluator.evaluate(&ctx).await.unwrap();
        assert!(!result.passed, "{:?}", result.issues);
    }

    /// Curved quotes are constrained too, and have no doubled form to escape to.
    #[tokio::test]
    async fn fails_when_a_curved_quote_cannot_close() {
        let ctx = make_context(
            "BEAM uses the GCC extension \"`labels as values`\".",
            "BEAM은 GCC 확장 기능인 \"`labels as values`\"를 사용합니다.",
        );
        let result = FormatEvaluator.evaluate(&ctx).await.unwrap();
        assert!(!result.passed);
        assert!(
            result
                .issues
                .iter()
                .any(|i| i.message.contains("curved-quote")),
            "{:?}",
            result.issues
        );
    }

    /// Writing the quotation marks outright is the only way out, so it must not
    /// read as two code markers dropped.
    #[tokio::test]
    async fn passes_when_a_curved_quote_becomes_the_quotation_marks() {
        let ctx = make_context(
            "BEAM uses the GCC extension \"`labels as values`\".",
            "BEAM은 GCC 확장 기능인 “labels as values”를 사용합니다.",
        );
        let result = FormatEvaluator.evaluate(&ctx).await.unwrap();
        assert!(result.passed, "{:?}", result.issues);
    }

    /// A quotation the source itself leaves open is the source's business.
    #[tokio::test]
    async fn passes_when_the_source_quote_cannot_close_either() {
        let ctx = make_context("the \"`term`\"s here", "여기 \"`term`\"의");
        let result = FormatEvaluator.evaluate(&ctx).await.unwrap();
        assert!(result.passed, "{:?}", result.issues);
    }

    /// The RST counterpart of the particle problem: docutils wants whitespace
    /// or punctuation after a closing marker, and a Hangul particle is neither.
    /// A segment can begin inside emphasis an earlier sentence opened
    /// (`*…whole truth.  It* is *true…`); its first mark closes a pair the
    /// segment never saw open, so the translation's `그것*\\ 은` is that closer.
    #[tokio::test]
    async fn rst_a_source_that_starts_inside_a_pair_is_no_measure() {
        let ctx = context_in(
            Markup::Rst,
            "It* is *true that there are cases where RPython gives you better speed.",
            "그것*\\ 은 *RPython이 더 나은 속도를 내는 경우가 있다는 점에서 사실입니다.",
        );
        let result = FormatEvaluator.evaluate(&ctx).await.unwrap();
        assert!(
            !result
                .issues
                .iter()
                .any(|i| i.message.contains("is not recognized as markup")),
            "{:?}",
            result.issues
        );
    }

    /// The boundary repair must not "fix" that closer into an opener either.
    #[test]
    fn rst_boundary_repair_leaves_a_closer_from_an_earlier_segment_alone() {
        let translation =
            "그것*\\ 은 *RPython이 더 나은 속도를 내는 경우가 있다는 점에서 사실입니다.";
        assert_eq!(
            repair_rst_boundaries(
                "It* is *true that there are cases where RPython gives you better speed.",
                translation,
            ),
            translation
        );
    }

    /// A pointer, a glob or an escaped star is not a closer, and in any case a
    /// star says nothing about the literals around it.
    #[tokio::test]
    async fn rst_a_lone_star_does_not_switch_off_the_literal_check() {
        for (source, translation) in [
            (
                "Pass a PyObject* to ``foo`` here.",
                "PyObject*를 ``foo``에 전달합니다.",
            ),
            (
                "Items marked (*) need ``foo`` set.",
                "(*) 표시가 있는 항목은 ``foo``가 설정되어야 합니다.",
            ),
            (
                "Match \\*.py and ``foo`` together.",
                "\\*.py와 ``foo``를 함께 맞춥니다.",
            ),
        ] {
            let result = FormatEvaluator
                .evaluate(&context_in(Markup::Rst, source, translation))
                .await
                .unwrap();
            assert!(
                result
                    .issues
                    .iter()
                    .any(|i| i.message.contains("is not recognized as markup")),
                "{translation:?}: {:?}",
                result.issues
            );
        }
    }

    #[tokio::test]
    async fn rst_fails_when_a_particle_follows_a_closing_marker() {
        let ctx = context_in(
            Markup::Rst,
            "Objects are allocated on the ``heap``.",
            "객체는 ``heap``에 할당됩니다.",
        );
        let result = FormatEvaluator.evaluate(&ctx).await.unwrap();
        assert!(!result.passed);
        assert!(
            result
                .issues
                .iter()
                .any(|i| i.message.contains("backslash-escaped")),
            "{:?}",
            result.issues
        );
    }

    /// The documented escape — a backslash-escaped space — must pass, and so
    /// must the marker counts it leaves behind.
    #[tokio::test]
    async fn rst_passes_with_a_backslash_escaped_space() {
        let ctx = context_in(
            Markup::Rst,
            "Objects are allocated on the ``heap``.",
            "객체는 ``heap``\\ 에 할당됩니다.",
        );
        let result = FormatEvaluator.evaluate(&ctx).await.unwrap();
        assert!(result.passed, "{:?}", result.issues);
    }

    /// Doubling is not an escape in RST — `` and ` are different constructs
    /// under the same recognition rule.
    #[tokio::test]
    async fn rst_doubled_markers_do_not_escape() {
        let ctx = context_in(
            Markup::Rst,
            "two or more processes that **can** execute independently",
            "서로 독립적으로 실행**될 수 있는** 둘 이상의 프로세스",
        );
        let result = FormatEvaluator.evaluate(&ctx).await.unwrap();
        assert!(!result.passed, "{:?}", result.issues);
    }

    /// `` `name`_ `` carries its reference suffix outside the backticks; the
    /// recognition rule applies after the underscores.
    #[test]
    fn rst_reference_suffix_is_part_of_the_marker() {
        assert!(rst_broken_pairs("consult the `PyPy website`_ for details").is_empty());
        assert_eq!(
            rst_broken_pairs("`PyPy website`_를 참고하십시오"),
            vec!["`PyPy website`_를"]
        );
        assert!(rst_broken_pairs("`PyPy website`_\\ 를 참고하십시오").is_empty());
    }

    #[tokio::test]
    async fn rst_footnote_reference_requires_a_boundary_before_a_particle() {
        let ctx = context_in(
            Markup::Rst,
            "Follow the procedure [2]_ carefully.",
            "절차 [2]_에서 설명한 대로 주의해서 진행합니다.",
        );
        let result = FormatEvaluator.evaluate(&ctx).await.unwrap();
        assert!(!result.passed, "{:?}", result.issues);
        assert!(
            result
                .issues
                .iter()
                .any(|issue| issue.message.contains("[2]_에")),
            "{:?}",
            result.issues
        );
    }

    #[tokio::test]
    async fn rst_footnote_reference_requires_a_boundary_after_the_previous_word() {
        let ctx = context_in(
            Markup::Rst,
            "Discussed by Hye-Shik Chang [1]_.",
            "Hye-Shik Chang[1]_\\ 에 의해 논의되었습니다.",
        );
        let result = FormatEvaluator.evaluate(&ctx).await.unwrap();
        assert!(!result.passed, "{:?}", result.issues);
    }

    #[tokio::test]
    async fn rst_footnote_reference_accepts_a_backslash_escaped_space() {
        let ctx = context_in(
            Markup::Rst,
            "Follow the procedure [2]_ carefully.",
            "절차 [2]_\\ 에서 설명한 대로 주의해서 진행합니다.",
        );
        let result = FormatEvaluator.evaluate(&ctx).await.unwrap();
        assert!(result.passed, "{:?}", result.issues);
    }

    #[test]
    fn rst_boundary_repair_handles_particles_on_inline_constructs() {
        assert_eq!(
            repair_rst_boundaries(
                "See ``heap``, *meaning*, :pep:`310`, and [2]_ for details.",
                "``heap``에, *의미*를, :pep:`310`뿐 아니라 Chang[2]_에서도 확인합니다.",
            ),
            "``heap``\\ 에, *의미*\\ 를, :pep:`310`\\ 뿐 아니라 Chang [2]_\\ 에서도 확인합니다."
        );
        assert_eq!(
            repair_rst_boundaries("can execute independently", "실행**될 수 있는** 작업"),
            "실행\\ **될 수 있는** 작업"
        );
        assert_eq!(
            repair_rst_boundaries(
                "The ``**kwargs: Unpack[K]`` allows *inferring* a TypedDict.",
                "``**kwargs: Unpack[K]``\\ 는 TypedDict를 *추론*할 수 있게 합니다.",
            ),
            "``**kwargs: Unpack[K]``\\ 는 TypedDict를 *추론*\\ 할 수 있게 합니다."
        );
    }

    #[test]
    fn rst_boundary_repair_handles_literal_roles_and_space_before_a_closer() {
        assert_eq!(
            repair_rst_boundaries(
                "Use ``:role:`target``` as an example.",
                "예제로 ``:role:`target```을 사용합니다.",
            ),
            "예제로 ``:role:`target```\\ 을 사용합니다."
        );
        assert_eq!(
            repair_rst_boundaries(
                "**Important:** use the generated file.",
                "**중요: ** ``file`` 대상을 사용합니다.",
            ),
            "**중요:** ``file`` 대상을 사용합니다."
        );
    }

    /// At the start of a line a role and a dotted name are prose; only a
    /// field marker — the closing colon followed by a space — opens a field
    /// list, and a backslash, which renders as nothing, keeps it prose.
    #[test]
    fn rst_boundary_repair_escapes_only_a_field_marker_at_column_zero() {
        assert_eq!(
            repair_rst_boundaries(
                "As explained in :pep:`252`, descriptors have a get method.",
                ":pep:`252`\\ 에서 설명한 것처럼 디스크립터에는 get 메서드가 있습니다.",
            ),
            ":pep:`252`\\ 에서 설명한 것처럼 디스크립터에는 get 메서드가 있습니다."
        );
        assert_eq!(
            repair_rst_boundaries(
                "The .NET platform is supported.",
                ".NET 플랫폼을 지원합니다."
            ),
            ".NET 플랫폼을 지원합니다."
        );
        assert_eq!(
            repair_rst_boundaries(
                "The value of the :class: option is a string.",
                ":class: 옵션의 값은 문자열입니다."
            ),
            "\\:class: 옵션의 값은 문자열입니다."
        );
    }

    #[test]
    fn rst_boundary_repair_separates_a_url_from_a_korean_particle() {
        assert_eq!(
            repair_rst_boundaries(
                "Results are published on http://docs.python.org.",
                "결과는 http://docs.python.org에서 공개됩니다.",
            ),
            "결과는 http://docs.python.org\\ 에서 공개됩니다."
        );
    }

    /// A shorter URL the source also cites is no boundary inside a longer one.
    #[test]
    fn rst_boundary_repair_leaves_a_longer_url_whole() {
        let translation = "규격: http://speleotrove.com/decimal/decarith.html (관련 문서는 \
                           http://speleotrove.com/decimal/ 에 있음)";
        assert_eq!(
            repair_rst_boundaries(
                "Specification: http://speleotrove.com/decimal/decarith.html (related documents \
                 at http://speleotrove.com/decimal/)",
                translation,
            ),
            translation
        );
    }

    #[tokio::test]
    async fn rst_translated_named_reference_alias_is_not_inline_code() {
        let ctx = context_in(
            Markup::Rst,
            "Package docutils.parsers: markup parsers_.",
            "docutils.parsers 패키지: 마크업 `파서 <parsers_>`_.",
        );
        let result = FormatEvaluator.evaluate(&ctx).await.unwrap();
        assert!(result.passed, "{:?}", result.issues);

        let ctx = context_in(
            Markup::Rst,
            "Read the `strong arguments`_ in ``python-dev``.",
            "``python-dev``\\ 에서 `강력한 주장 <strong arguments_>`_\\ 을 읽으십시오.",
        );
        let result = FormatEvaluator.evaluate(&ctx).await.unwrap();
        assert!(result.passed, "{:?}", result.issues);

        let ctx = context_in(
            Markup::Rst,
            "See `the definitions <https://example.com>`__ of an ``.add_note()`` method.",
            "`여러 ``.add_note()`` 메서드 정의 <https://example.com>`__\\ 를 보십시오.",
        );
        let result = FormatEvaluator.evaluate(&ctx).await.unwrap();
        assert!(result.passed, "{:?}", result.issues);

        let ctx = context_in(
            Markup::Rst,
            "Use ``value`` as described by the `reference`_.",
            "`참조 <reference_>`_\\ 에 설명된 대로 ``value``\\ 를 사용합니다.",
        );
        let result = FormatEvaluator.evaluate(&ctx).await.unwrap();
        assert!(result.passed, "{:?}", result.issues);
    }

    #[test]
    fn rst_boundary_repair_respects_a_source_split_inside_markup() {
        let source = "RHS`` would roughly be ``fallback``";
        let translation = "RHS`` 는 대략 ``fallback``\\ 과 같습니다";
        assert_eq!(repair_rst_boundaries(source, translation), translation);
    }

    #[test]
    fn rst_bracket_reference_detection_covers_citations_and_named_footnotes() {
        assert_eq!(
            rst_broken_bracket_references("[RFC]_는 표준입니다. [#named]_에서 계속됩니다."),
            vec!["[RFC]_는", "[#named]_에"]
        );
        assert!(rst_broken_bracket_references("[RFC]_\\ 는 표준입니다.").is_empty());
    }

    #[test]
    fn rst_prose_is_not_markup() {
        for text in [
            "결과는 2 * 3 입니다.",
            "the value 2*3 appears once",
            ":ref:`contact` 부분을 보십시오.",
            "``code`` 다음에 공백이 있습니다.",
            "인용 부호 “안”의 텍스트.",
        ] {
            assert!(
                rst_broken_pairs(text).is_empty(),
                "{text:?} carries no broken pair"
            );
        }
    }

    #[test]
    fn rst_role_content_against_a_particle_is_broken() {
        assert_eq!(
            rst_broken_pairs(":ref:`contact`를 보십시오"),
            vec!["`contact`를"]
        );
    }

    #[tokio::test]
    async fn rst_role_rejects_an_extra_closing_backtick() {
        let ctx = context_in(
            Markup::Rst,
            "PEP :pep:`649` defines the behavior.",
            ":pep:`649``\\ 에서 동작을 정의합니다.",
        );
        let result = FormatEvaluator.evaluate(&ctx).await.unwrap();
        assert!(!result.passed, "{:?}", result.issues);
        assert!(
            result
                .issues
                .iter()
                .any(|issue| issue.message.contains("extra backtick")),
            "{:?}",
            result.issues
        );
    }

    #[tokio::test]
    async fn rst_source_preserved_literal_role_accepts_valid_outer_boundaries() {
        for translation in [
            "예제: ``:role:`target```.",
            "예제로 ``:role:`target```\\ 을 사용합니다.",
        ] {
            let ctx = context_in(
                Markup::Rst,
                "Use ``:role:`target``` as an example.",
                translation,
            );

            let result = FormatEvaluator.evaluate(&ctx).await.unwrap();

            assert!(result.passed, "{translation:?}: {:?}", result.issues);
        }
    }

    #[tokio::test]
    async fn rst_source_preserved_literal_role_rejects_a_hangul_outer_boundary() {
        let ctx = context_in(
            Markup::Rst,
            "Use ``:role:`target``` as an example.",
            "예제로 ``:role:`target```을 사용합니다.",
        );

        let result = FormatEvaluator.evaluate(&ctx).await.unwrap();

        assert!(!result.passed, "{:?}", result.issues);
    }

    #[tokio::test]
    async fn rst_strong_emphasis_rejects_whitespace_before_its_closer() {
        let ctx = context_in(
            Markup::Rst,
            "**Important:** use the generated file.",
            "**중요: ** 생성된 파일을 사용합니다.",
        );

        let result = FormatEvaluator.evaluate(&ctx).await.unwrap();

        assert!(!result.passed, "{:?}", result.issues);
    }

    #[tokio::test]
    async fn rst_new_extra_role_backtick_is_rejected_beside_a_preserved_literal_role() {
        let ctx = context_in(
            Markup::Rst,
            "Keep ``:role:`source``` and valid :pep:`649` here.",
            "Keep ``:role:`source``` but corrupt :pep:`649`` here.",
        );

        let result = FormatEvaluator.evaluate(&ctx).await.unwrap();

        assert!(!result.passed, "{:?}", result.issues);
        assert!(
            result
                .issues
                .iter()
                .any(|issue| issue.message.contains(":pep:`649``")),
            "{:?}",
            result.issues
        );
    }

    /// A translation must not begin with `..`: at the start of a line it opens
    /// a comment and swallows what follows.
    #[test]
    fn explicit_markup_start_is_recognised() {
        assert_eq!(
            line_start_construct(".. 참고하십시오", Markup::Rst),
            Some("an explicit-markup start (`..`)")
        );
        assert_eq!(
            line_start_construct("... 그리고 계속됩니다", Markup::Rst),
            None
        );
    }

    /// Curved quotes borrow the backtick but are not code, so a quotation the
    /// translation keeps as a quotation is not a code span gained.
    #[test]
    fn curved_quote_marks_are_not_counted_as_code_markers() {
        assert_eq!(mark_runs("the \"`term`\" here", '`'), 0);
        assert_eq!(mark_runs("the `code` here", '`'), 2);
        assert_eq!(mark_runs("\"`quoted`\" and `code`", '`'), 2);
    }

    #[tokio::test]
    async fn verso_allows_visible_labels_to_be_translated() {
        let ctx = context_in(
            Markup::Verso,
            "See {ref \"getting-started\"}[the previous chapter].",
            "{ref \"getting-started\"}[이전 장]을 참고하십시오.",
        );
        let result = FormatEvaluator.evaluate(&ctx).await.unwrap();
        assert!(result.passed, "{:?}", result.issues);
    }

    #[tokio::test]
    async fn verso_rejects_changed_role_headers_and_code_payloads() {
        for translation in [
            "{anchorName wrong}`List.map`을 사용합니다.",
            "{anchorName map}`List.filter`를 사용합니다.",
            "`List.map`을 사용합니다.",
        ] {
            let ctx = context_in(
                Markup::Verso,
                "Use {anchorName map}`List.map`.",
                translation,
            );
            let result = FormatEvaluator.evaluate(&ctx).await.unwrap();
            assert!(!result.passed, "{translation:?} should fail");
            assert!(
                result
                    .issues
                    .iter()
                    .any(|issue| issue.message.contains("Verso role")),
                "{:?}",
                result.issues
            );
        }
    }

    #[tokio::test]
    async fn verso_allows_roles_to_move_with_the_translated_sentence() {
        let ctx = context_in(
            Markup::Verso,
            "{lit}`lake` invokes {lit}`lean`.",
            "{lit}`lean`은 {lit}`lake`가 호출합니다.",
        );
        let result = FormatEvaluator.evaluate(&ctx).await.unwrap();
        assert!(result.passed, "{:?}", result.issues);
    }

    #[test]
    fn verso_code_nonbreaking_spaces_are_restored_before_evaluation() {
        let nbsp = '\u{a0}';
        let source = format!("Use {{lit}}`{nbsp}... ` after {{kw}}`in` and {{lit}}`{nbsp}= `.");
        let translation = "{kw}`in` 뒤에 {lit}`\\u{a0}... `와 {lit}` = `을 사용합니다.";
        let restored = restore_verso_code_whitespace(&source, translation);
        assert_eq!(
            restored,
            format!("{{kw}}`in` 뒤에 {{lit}}`{nbsp}... `와 {{lit}}`{nbsp}= `을 사용합니다.")
        );
    }

    #[test]
    fn verso_code_repair_does_not_hide_a_changed_payload() {
        let source = "Use {lit}`\u{a0}...`.";
        let translation = "{lit}`other`를 사용합니다.";
        assert_eq!(
            restore_verso_code_whitespace(source, translation),
            translation
        );
    }

    #[tokio::test]
    async fn latex_preserves_commands_math_references_and_comment_placeholders() {
        let ctx = context_in(
            Markup::Latex,
            "See \\Cref{thm:main}: a \\emph{group} $G$ works. ⟦YKTEXC0⟧",
            "\\Cref{thm:main}을 보십시오. \\emph{군} $G$는 작동합니다. ⟦YKTEXC0⟧",
        );
        let result = FormatEvaluator.evaluate(&ctx).await.unwrap();
        assert!(result.passed, "{:?}", result.issues);
    }

    #[tokio::test]
    async fn latex_rejects_changed_math_and_reference_targets() {
        for translation in [
            "\\Cref{thm:other}에 따르면 $G$가 작동합니다.",
            "\\Cref{thm:main}에 따르면 $H$가 작동합니다.",
        ] {
            let ctx = context_in(
                Markup::Latex,
                "By \\Cref{thm:main}, $G$ works.",
                translation,
            );
            let result = FormatEvaluator.evaluate(&ctx).await.unwrap();
            assert!(!result.passed, "{translation:?} should fail");
        }
    }

    #[tokio::test]
    async fn latex_allows_complete_tokens_to_move_with_korean_word_order() {
        let ctx = context_in(
            Markup::Latex,
            r"For $x \in X$, see \Cref{thm:main} and use \emph{compactness}.",
            r"\emph{콤팩트성}을 사용하고 \Cref{thm:main}을 보십시오. 단, $x \in X$입니다.",
        );
        let result = FormatEvaluator.evaluate(&ctx).await.unwrap();
        assert!(result.passed, "{:?}", result.issues);
    }

    #[tokio::test]
    async fn latex_rejects_a_dropped_duplicate_token() {
        let ctx = context_in(Markup::Latex, "$G$ acts on $G$.", "$G$가 작용합니다.");
        let result = FormatEvaluator.evaluate(&ctx).await.unwrap();
        assert!(!result.passed);
    }

    #[tokio::test]
    async fn latex_allows_additional_valid_math_notation() {
        let ctx = context_in(
            Markup::Latex,
            "The characteristic is zero and its submodules stabilize.",
            "표수는 $0$이고 $M$의 부분가군은 안정화됩니다.",
        );
        let result = FormatEvaluator.evaluate(&ctx).await.unwrap();
        assert!(result.passed, "{:?}", result.issues);
    }

    #[tokio::test]
    async fn latex_ignores_a_dropped_interword_spacing_hint() {
        let ctx = context_in(
            Markup::Latex,
            r"The value is fixed, i.e.\ it cannot move.",
            "그 값은 고정되어 있습니다. 즉, 움직일 수 없습니다.",
        );
        let result = FormatEvaluator.evaluate(&ctx).await.unwrap();
        assert!(result.passed, "{:?}", result.issues);
    }

    #[tokio::test]
    async fn latex_allows_visible_prose_inside_math_to_be_translated() {
        let ctx = context_in(
            Markup::Latex,
            r"The value is $\sup\{x \mid x \text{ compact}\}$.",
            r"그 값은 $\sup\{x \mid x \text{ 콤팩트}\}$입니다.",
        );
        let result = FormatEvaluator.evaluate(&ctx).await.unwrap();
        assert!(result.passed, "{:?}", result.issues);
    }

    #[tokio::test]
    async fn latex_allows_terminal_punctuation_to_move_out_of_math() {
        let ctx = context_in(
            Markup::Latex,
            r"Show that \[ T^\dagger = p(T). \]",
            r"다음을 보이십시오. \[ T^\dagger = p(T) \]",
        );
        let result = FormatEvaluator.evaluate(&ctx).await.unwrap();
        assert!(result.passed, "{:?}", result.issues);
    }

    #[tokio::test]
    async fn latex_allows_tex_accents_to_be_transliterated() {
        for source in [r#"G\"{o}del"#, r"\v{C}ech", r"\^{e}tre"] {
            let ctx = context_in(Markup::Latex, source, "한글 음역");
            let result = FormatEvaluator.evaluate(&ctx).await.unwrap();
            assert!(result.passed, "{source:?}: {:?}", result.issues);
        }
    }

    #[tokio::test]
    async fn latex_does_not_treat_tex_quotes_as_inline_code() {
        let ctx = context_in(Markup::Latex, "A ``group''.", "어떤 ‘군’입니다.");
        let result = FormatEvaluator.evaluate(&ctx).await.unwrap();
        assert!(result.passed, "{:?}", result.issues);
    }
    #[tokio::test]
    async fn latex_range_references_preserve_both_targets() {
        let ctx = context_in(
            Markup::Latex,
            r"See \crefrange{sec:first}{sec:last}.",
            r"\crefrange{sec:first}{sec:changed}를 보십시오.",
        );
        assert!(!FormatEvaluator.evaluate(&ctx).await.unwrap().passed);
    }
    #[tokio::test]
    async fn latex_range_references_preserve_endpoint_order() {
        let ctx = context_in(
            Markup::Latex,
            r"See \crefrange{sec:first}{sec:last}.",
            r"\crefrange{sec:last}{sec:first}를 보십시오.",
        );
        assert!(!FormatEvaluator.evaluate(&ctx).await.unwrap().passed);
    }
    #[tokio::test]
    async fn latex_math_macros_cannot_absorb_translated_prose() {
        let ctx = context_in(
            Markup::Latex,
            r"The pair \narrowequation{(a,b):A} has a second component.",
            r"쌍 \narrowequation{(a,b):A의 두 번째 성분}이 있습니다.",
        );
        assert!(!FormatEvaluator.evaluate(&ctx).await.unwrap().passed);
    }
    #[tokio::test]
    async fn latex_rejects_extra_or_reversed_group_delimiters() {
        for translation in [
            r"\cref{thm:main}}에 의해 성립합니다.",
            r"}\cref{thm:main}{에 의해 성립합니다.",
        ] {
            let ctx = context_in(Markup::Latex, r"By \cref{thm:main}, it holds.", translation);
            assert!(!FormatEvaluator.evaluate(&ctx).await.unwrap().passed);
        }
    }

    fn mkdocs_ctx(source: &str, translation: &str) -> EvaluationContext {
        context_in(Markup::MkDocs, source, translation)
    }

    #[tokio::test]
    async fn mkdocs_math_must_survive_byte_for_byte() {
        let ok = FormatEvaluator.evaluate(&mkdocs_ctx("Let $a_i$ be $O(n)$.", "$a_i$를 $O(n)$이라 하자.")).await.unwrap();
        assert!(ok.passed, "{:?}", ok.issues);
        let changed = FormatEvaluator.evaluate(&mkdocs_ctx("Let $a_i$ be $O(n)$.", "$a_{i}$를 $O(n)$이라 하자.")).await.unwrap();
        assert!(!changed.passed);
        let dropped = FormatEvaluator.evaluate(&mkdocs_ctx("Let $a_i$ be x.", "a_i를 x라 하자.")).await.unwrap();
        assert!(!dropped.passed);
    }

    #[tokio::test]
    async fn mkdocs_underscore_inside_math_is_not_emphasis() {
        let result = FormatEvaluator.evaluate(&mkdocs_ctx("If $x_1 < y_1$ then $z_2$.", "$x_1 < y_1$이면 $z_2$입니다.")).await.unwrap();
        assert!(result.passed, "{:?}", result.issues);
    }

    /// Observed in the cp-algorithms pilot: plain "O(N)" in a navigation
    /// label came back as `$O(N)$`, which the sidebar prints with its dollars,
    /// and a sentence pulled in from the next segment brought its formula.
    #[tokio::test]
    async fn mkdocs_formula_absent_from_the_source_is_an_error() {
        let added = FormatEvaluator.evaluate(&mkdocs_ctx("[Finding Bridges in O(N+M)](graph/bridge-searching.md)", "[$O(N+M)$에 다리 찾기](graph/bridge-searching.md)")).await.unwrap();
        assert!(!added.passed);
        let repeated = FormatEvaluator.evaluate(&mkdocs_ctx("For each vertex $v$ in $V$.", "$V$의 각 정점 $v$에 대해 $v$를 봅니다.")).await.unwrap();
        assert!(repeated.passed, "{:?}", repeated.issues);
    }

    #[tokio::test]
    async fn mkdocs_added_jinja_delimiter_is_an_error() {
        let result = FormatEvaluator.evaluate(&mkdocs_ctx("Use a set.", "{# 집합 #}을 사용합니다.")).await.unwrap();
        assert!(!result.passed);
    }

    /// PEP 818 shipped ``type(run_js("({next(){}})"))`` with one more `)`
    /// inside the literal: the marker counts matched, and the reader was shown
    /// code that does not run.
    #[tokio::test]
    async fn fails_when_the_code_inside_a_span_changes() {
        let ctx = context_in(
            Markup::Rst,
            r#"This is ``type(run_js("({next(){}})"))``."#,
            r#"이는 ``type(run_js("({next(){}}))"))``\ 입니다."#,
        );
        let result = FormatEvaluator.evaluate(&ctx).await.unwrap();
        assert!(!result.passed);
        assert!(
            result.issues.iter().any(|i| i.message.contains(r#"``type(run_js("({next(){}}))"))``"#)),
            "the message should quote the changed span: {:?}",
            result.issues
        );

        let ctx = context_in(Markup::Markdown, "Call `panic_fmt()` here.", "여기서 `panic_impl()`을 호출합니다.");
        assert!(!FormatEvaluator.evaluate(&ctx).await.unwrap().passed);
    }

    /// thebeambook's `` `+receive [] -> ok end+` `` came back as
    /// `` ``receive [] -> ok end``와 `` on every retry, which renders `->` as
    /// an arrow.
    #[tokio::test]
    async fn an_asciidoc_passthrough_keeps_its_pluses_when_the_marks_double() {
        let source = "For a selective receive like e.g. `+receive [] -> ok end+` we loop.";
        let dropped = FormatEvaluator
            .evaluate(&make_context(source, "``receive [] -> ok end``와 같은 선택적 receive의 경우 순회합니다."))
            .await
            .unwrap();
        assert!(!dropped.passed);
        assert!(dropped.issues.iter().any(|i| i.message.contains("`+x+` 같은")), "{:?}", dropped.issues);
        for kept in [
            "`+receive [] -> ok end+` 와 같은 선택적 receive의 경우 순회합니다.",
            "``+receive [] -> ok end+``와 같은 선택적 receive의 경우 순회합니다.",
        ] {
            let result = FormatEvaluator.evaluate(&make_context(source, kept)).await.unwrap();
            assert!(result.passed, "{kept:?}: {:?}", result.issues);
        }
    }

    #[tokio::test]
    async fn code_may_move_and_change_its_marks_but_not_its_content() {
        for (markup, source, translation) in [
            (Markup::Markdown, "Call `a` before `b`.", "`b` 앞에서 `a`를 호출합니다."),
            // A source line break inside a span is a space.
            (Markup::Markdown, "Set `max\n  depth` first.", "먼저 `max depth`를 설정합니다."),
            (Markup::Asciidoc, "Allocate on the `heap`.", "``heap``에 할당합니다."),
            (Markup::Rst, "Use ``x`` here.", "여기서 ``x``\\ 를 사용합니다."),
        ] {
            let result = FormatEvaluator.evaluate(&context_in(markup, source, translation)).await.unwrap();
            assert!(result.passed, "{translation:?}: {:?}", result.issues);
        }
    }

    /// Backticks that do not open code carry prose a translation translates.
    #[tokio::test]
    async fn backticked_labels_are_not_code() {
        for (markup, source, translation) in [
            // MyST roles and RST interpreted text name a target with a visible label.
            (Markup::Markdown, "See {ref}`the guide <guide>`.", "{ref}`안내서 <guide>`를 참고하십시오."),
            (Markup::Rst, "See `the guide`_ and :term:`garbage collection`.", "`안내서 <the guide_>`_\\ 와 :term:`쓰레기 수집 <garbage collection>`\\ 을 참고하십시오."),
            // An AsciiDoc curved quote borrows the backtick.
            (Markup::Asciidoc, "It is called \"`the heap`\" here.", "여기서는 “힙”이라고 부릅니다."),
        ] {
            let result = FormatEvaluator.evaluate(&context_in(markup, source, translation)).await.unwrap();
            assert!(result.passed, "{translation:?}: {:?}", result.issues);
        }
    }

    /// The English preposition that opens a parenthetical disappears into a
    /// Korean particle, and the pair around the link was written twice:
    /// furiosa-opt's `dma-engine.md` and rustc-dev-guide's
    /// `panic-implementation.md`.
    #[tokio::test]
    async fn fails_when_a_parenthetical_is_wrapped_twice() {
        for (source, translation) in [
            (
                "The Tensor Unit (via [Fetch](./fetch-engine.md) and [Commit](./commit-engine.md) Engines) is often more efficient.",
                "Tensor Unit(([Fetch](./fetch-engine.md) 및 [Commit](./commit-engine.md) Engine을 통해))은 더 효율적인 경우가 많습니다.",
            ),
            (
                "The `core` `panic!` macro eventually makes the following call (in [`library/core/src/panicking.rs`]):",
                "`core`의 `panic!` 매크로는 결국 다음과 같은 호출을 수행합니다(([`library/core/src/panicking.rs`]에서)):",
            ),
        ] {
            let result = FormatEvaluator.evaluate(&context_in(Markup::Markdown, source, translation)).await.unwrap();
            assert!(!result.passed, "{translation:?}");
            assert!(
                result.issues.iter().any(|i| i.message.contains("((")),
                "the message should quote the doubled pair: {:?}",
                result.issues
            );
        }
    }

    #[tokio::test]
    async fn parentheses_the_source_has_are_kept() {
        for (source, translation) in [
            (
                "The Tensor Unit (via [Fetch](./fetch-engine.md) Engines) is efficient.",
                "Tensor Unit([Fetch](./fetch-engine.md) Engine을 통해)은 효율적입니다.",
            ),
            // A gloss inside a parenthetical nests; it does not wrap twice.
            ("Sort it (e.g. insertion sort).", "정렬합니다(예: 삽입 정렬(insertion sort))."),
            ("Apply f((x)) twice.", "f((x))를 두 번 적용합니다."),
            // Code is compared as code, not as prose.
            ("Write `f((x))` here.", "여기에 `f((x))`를 씁니다."),
        ] {
            let result = FormatEvaluator.evaluate(&context_in(Markup::Markdown, source, translation)).await.unwrap();
            assert!(result.passed, "{translation:?}: {:?}", result.issues);
        }
    }

    #[tokio::test]
    async fn fails_when_the_translation_leaves_a_parenthesis_open() {
        let ctx = context_in(
            Markup::Markdown,
            "Transfer it (see [PCIe DMA](#pcie-dma)).",
            "전송합니다(([PCIe DMA](#pcie-dma) 참고).",
        );
        assert!(!FormatEvaluator.evaluate(&ctx).await.unwrap().passed);
    }

    #[tokio::test]
    async fn an_rst_literal_closes_on_the_last_two_marks_of_a_longer_run() {
        let ctx = context_in(
            Markup::Rst,
            "``:func:`filter``` could refer to a function named ``filter``.",
            "``:func:`filter```\\ 는 ``filter``\\ 라는 함수를 가리킬 수 있습니다.",
        );
        let result = FormatEvaluator.evaluate(&ctx).await.unwrap();
        assert!(result.passed, "{:?}", result.issues);
    }

    /// Korean carries plurals, call parentheses and sentence punctuation
    /// outside the span, and a translation that moves what the source let slip
    /// inside the marks is right.
    #[tokio::test]
    async fn a_translation_may_shed_what_the_sentence_carries() {
        for (markup, source, translation) in [
            (Markup::Markdown, "Edit the `Makefiles`.", "`Makefile`을 수정합니다."),
            (Markup::Markdown, "Chain the `ORs`.", "`OR`를 잇습니다."),
            (Markup::Rst, "Call ``tp_alloc()`` first.", "먼저 ``tp_alloc``\\ 을 호출합니다."),
            (Markup::Rst, "a new namespace called ``MyRing.``", "``MyRing``\\ 이라는 새 네임스페이스"),
            (
                Markup::Rst,
                "It exists (it calls the C API ``PyObject_IsTrue())``.",
                "존재합니다(C API인 ``PyObject_IsTrue()``\\ 를 호출합니다).",
            ),
        ] {
            let result = FormatEvaluator.evaluate(&context_in(markup, source, translation)).await.unwrap();
            assert!(result.passed, "{translation:?}: {:?}", result.issues);
        }
    }

    #[tokio::test]
    async fn nothing_may_be_added_to_code() {
        for (source, translation) in [
            ("It returns `f(x)`.", "`f(x))`를 반환합니다."),
            ("Always has ``fold=0``.", "``fold``\\ 는 항상 0입니다."),
            ("Use ``reapply()``.", "``.reapply()``\\ 를 사용합니다."),
            // A single backtick is interpreted text in RST, not a literal.
            ("Files ending ``.py`` count.", "`.py`\\ 로 끝나는 파일을 셉니다."),
            ("It ``is`` there.", "그것은 거기 ``있다``."),
        ] {
            let markup = if source.contains("``") { Markup::Rst } else { Markup::Markdown };
            let result = FormatEvaluator.evaluate(&context_in(markup, source, translation)).await.unwrap();
            assert!(!result.passed, "{translation:?}");
        }
    }

    /// Marking up as code what the source writes as plain text, or repeating
    /// one of its code spans where Korean names the subject again, adds no code
    /// the source lacks.
    #[tokio::test]
    async fn source_text_may_become_code() {
        for (markup, source, translation) in [
            (
                Markup::Markdown,
                "copy the .pem file into the same folder as the `gen_temp_access_token.py`",
                "`.pem` 파일을 `gen_temp_access_token.py`와 같은 폴더로 복사합니다",
            ),
            (
                Markup::Markdown,
                "The default rules for auto traits say that `Foo` is `Send` if the types of its \
                 fields are `Send`.",
                "오토 트레이트의 기본 규칙은 `Foo`의 필드 타입이 모두 `Send`이면 `Foo`도 \
                 `Send`라는 것입니다.",
            ),
            (
                Markup::Asciidoc,
                "The .erlang.crypt file should contain a list of tuples in the format \
                 {debug_info, Mode, Module, Key}.",
                "`.erlang.crypt` 파일은 `{debug_info, Mode, Module, Key}` 형식의 튜플 목록을 \
                 포함해야 합니다.",
            ),
            (
                Markup::Rst,
                "On UNIX since Python 3.2, subprocess.Popen() closes all file descriptors by \
                 default: ``close_fds=True``.",
                "Python 3.2부터 UNIX에서, ``subprocess.Popen()``\\ 은 기본적으로 모든 파일 \
                 디스크립터를 닫습니다: ``close_fds=True``.",
            ),
        ] {
            let result = FormatEvaluator
                .evaluate(&context_in(markup, source, translation))
                .await
                .unwrap();
            assert!(result.passed, "{translation:?}: {:?}", result.issues);
        }
    }

    /// A reference link keeps its source text as the label, `[번역][label]`,
    /// so the translated text can change while the link still resolves. The
    /// label is not rendered; its markup is no markup of the translation.
    #[tokio::test]
    async fn a_reference_label_is_not_rendered_markup() {
        for (source, translation) in [
            (
                "Rust code is also [_monomorphized_] during code generation.",
                "러스트 코드는 코드 생성 중에 [*단형화*][_monomorphized_]되기도 합니다.",
            ),
            (
                "[`nix` command]s natively integrate with flakes by default.",
                "[`nix` 명령][`nix` command]은 기본적으로 플레이크와 통합됩니다.",
            ),
        ] {
            let result = FormatEvaluator
                .evaluate(&context_in(Markup::Markdown, source, translation))
                .await
                .unwrap();
            assert!(result.passed, "{translation:?}: {:?}", result.issues);
        }
    }

    /// A role or interpreted text rewritten as a literal keeps its text but not
    /// what it does: `:mod:` links, `:math:` typesets.
    #[tokio::test]
    async fn a_role_recast_as_a_literal_is_reported() {
        for (source, translation) in [
            (
                "The :mod:`string` module will be converted into a package.",
                "``string`` 모듈은 패키지로 변환됩니다.",
            ),
            (
                "tends to :math:`x` while remaining in :math:`A` holds.",
                "``A``\\ 에 머무르면서 :math:`x`\\ 로 수렴합니다.",
            ),
        ] {
            let result = FormatEvaluator
                .evaluate(&context_in(Markup::Rst, source, translation))
                .await
                .unwrap();
            assert!(
                result
                    .issues
                    .iter()
                    .any(|i| i.message.contains("became the literal")),
                "{translation:?}: {:?}",
                result.issues
            );
            assert!(
                !result
                    .issues
                    .iter()
                    .any(|i| i.message.contains("count mismatch")),
                "{translation:?}: {:?}",
                result.issues
            );
        }
    }

    /// A role the translation keeps is no role recast, and the literal beside
    /// it is source text marked up as code.
    #[tokio::test]
    async fn a_kept_role_beside_an_added_literal_is_not_recast() {
        for (source, translation) in [
            (
                "Use :func:`repr` to get it; repr is safe.",
                ":func:`repr`\\ 로 얻으십시오. ``repr``\\ 은 안전합니다.",
            ),
            (
                "`asyncio`_ is a library. asyncio is great.",
                "`asyncio`_\\ 는 라이브러리입니다. ``asyncio``\\ 는 훌륭합니다.",
            ),
        ] {
            let result = FormatEvaluator
                .evaluate(&context_in(Markup::Rst, source, translation))
                .await
                .unwrap();
            assert!(result.passed, "{translation:?}: {:?}", result.issues);
        }
    }

    /// Code the source never wrote, as code or as text, is still an addition.
    #[tokio::test]
    async fn code_the_source_never_wrote_is_added() {
        for (markup, source, translation) in [
            (
                Markup::Markdown,
                "Contributors often forget to tag things.",
                "기여자들은 종종 `rollup=never` 태그를 잊습니다.",
            ),
            (
                Markup::Asciidoc,
                "The term reduction is old.",
                "`리덕션`이라는 텀은 오래되었습니다.",
            ),
            // A word inside another word is not that word written as text.
            (
                Markup::Markdown,
                "Then they ended.",
                "그러면 `the` 끝났습니다.",
            ),
            // Excused as an addition, the span still has to close.
            (
                Markup::Asciidoc,
                "A deep copy is used for binary_to_term and message passing.",
                "깊은 복사는 `binary_to_term`과 메시지 전달에 사용됩니다.",
            ),
        ] {
            let result = FormatEvaluator
                .evaluate(&context_in(markup, source, translation))
                .await
                .unwrap();
            assert!(!result.passed, "{translation:?}");
        }
    }

    /// A segment that starts inside a literal pairs its marks one off.
    #[tokio::test]
    async fn a_source_that_leaves_a_span_open_is_no_measure() {
        let ctx = context_in(
            Markup::Rst,
            "or issubclass(type(x), B)``.  (It is possible ``type(x)`` and",
            "또는 issubclass(type(x), B)``\\ 입니다. (``type(x)``\\ 도 가능하며",
        );
        let result = FormatEvaluator.evaluate(&ctx).await.unwrap();
        assert!(!result.issues.iter().any(|i| i.message.contains("Inline code changed")), "{:?}", result.issues);
    }

    #[tokio::test]
    async fn parentheses_that_are_not_prose_pairs_are_left_alone() {
        for (markup, source, translation) in [
            // PEP 102 lets a smiley close its parenthetical.
            (
                Markup::Rst,
                "Type a subject (e.g. \"Python 2.2c1 released\" :-) in the box.",
                "상자에 제목(예: \"Python 2.2c1 released\" :-))을 입력하십시오.",
            ),
            // The source's math leaves a parenthesis for the prose to close.
            (
                Markup::MkDocs,
                "only nodes on layer $1$ (with length $O(\\sqrt{n}$) can be lazy.",
                "레이어 $1$(길이가 $O(\\sqrt{n}$)인)의 노드만 lazy가 될 수 있습니다.",
            ),
            // Math moves with Korean word order, brackets and all.
            (
                Markup::Latex,
                "Take $[0, 1)$ (a half-open interval) here.",
                "여기서 (반열린 구간인) $[0, 1)$을 택합니다.",
            ),
        ] {
            let result = FormatEvaluator.evaluate(&context_in(markup, source, translation)).await.unwrap();
            assert!(result.passed, "{translation:?}: {:?}", result.issues);
        }
    }

    #[tokio::test]
    async fn latex_prose_parentheses_are_checked_too() {
        let ctx = context_in(
            Markup::Latex,
            "Consider the map (in $\\OO_K$) given by $x \\mapsto x^p$.",
            "($\\OO_K$에서의) 사상 $x \\mapsto x^p$를 생각합시다.",
        );
        assert!(FormatEvaluator.evaluate(&ctx).await.unwrap().passed);
        let ctx = context_in(
            Markup::Latex,
            "Consider the map (in $\\OO_K$) given by $x \\mapsto x^p$.",
            "(($\\OO_K$에서의)) 사상 $x \\mapsto x^p$를 생각합시다.",
        );
        assert!(!FormatEvaluator.evaluate(&ctx).await.unwrap().passed);
    }
}
