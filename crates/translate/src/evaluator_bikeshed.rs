//! The Bikeshed tokens a translation must carry over as they are: bare
//! references `[[#section]]`, `[[RFC8949]]`, `[[!AGE]]` and variables
//! `|name|`. The autolinks and references with display text are rewritten
//! to `bikeshed:` links by the parser, and `LinkEvaluator` already compares
//! their destinations; this check covers what stays as prose.
use crate::evaluator::*;
use async_trait::async_trait;
use std::collections::HashMap;

pub struct BikeshedEvaluator;

#[async_trait]
impl TranslationEvaluator for BikeshedEvaluator {
    async fn evaluate(
        &self,
        context: &EvaluationContext,
    ) -> Result<EvaluationResult, EvaluationError> {
        let mut issues = Vec::new();
        if context.markup == Markup::Bikeshed {
            let (source, translation) = (
                bikeshed_tokens(&context.source),
                bikeshed_tokens(&context.translation),
            );
            let missing = difference(&source, &translation);
            let added = difference(&translation, &source);
            if !missing.is_empty() || !added.is_empty() {
                let mut message = String::from(
                    "Bikeshed references and variables must survive unchanged, once each",
                );
                if !missing.is_empty() {
                    message.push_str(&format!(
                        "; missing from the translation: {}",
                        missing.join(", ")
                    ));
                }
                if !added.is_empty() {
                    message.push_str(&format!("; not in the source: {}", added.join(", ")));
                }
                message.push_str(". Keep [[…]] and |name| exactly as written and attach particles after them (|n|을).");
                issues.push(EvaluationIssue {
                    severity: IssueSeverity::Error,
                    kind: IssueKind::FormatLost,
                    message,
                });
            }
        }
        Ok(EvaluationResult {
            passed: issues.is_empty(),
            issues,
        })
    }

    fn name(&self) -> &'static str {
        "Bikeshed"
    }
}

/// The tokens of `a` not in `b`, with multiplicity.
fn difference(a: &[String], b: &[String]) -> Vec<String> {
    let mut counts: HashMap<&str, usize> = HashMap::new();
    for token in b {
        *counts.entry(token).or_default() += 1;
    }
    let mut out = Vec::new();
    for token in a {
        match counts.get_mut(token.as_str()) {
            Some(n) if *n > 0 => *n -= 1,
            _ => out.push(token.clone()),
        }
    }
    out
}

/// `text` with its backtick code spans blanked, so their content is not
/// read as a token.
fn without_code_spans(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut out = String::with_capacity(text.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'`' {
            let len = bytes[i..].iter().take_while(|&&b| b == b'`').count();
            let mut j = i + len;
            let mut close = None;
            while j < bytes.len() {
                if bytes[j] == b'`' {
                    let run = bytes[j..].iter().take_while(|&&b| b == b'`').count();
                    if run == len {
                        close = Some(j + run);
                        break;
                    }
                    j += run;
                } else {
                    j += 1;
                }
            }
            match close {
                Some(end) => {
                    out.extend(std::iter::repeat_n(' ', end - i));
                    i = end;
                }
                None => {
                    out.push_str(&text[i..i + len]);
                    i += len;
                }
            }
            continue;
        }
        let c = text[i..].chars().next().unwrap();
        out.push(c);
        i += c.len_utf8();
    }
    out
}

/// `text` with its variables `|name|` outside code spans blanked, for
/// checks that read prose: a variable name is not a term to translate.
pub fn without_variables(text: &str) -> String {
    let masked = without_code_spans(text);
    let bytes = masked.as_bytes();
    let mut out = String::with_capacity(text.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'|'
            && let Some(len) = variable_len(&masked[i + 1..])
        {
            out.extend(std::iter::repeat_n(' ', len + 2));
            i += len + 2;
            continue;
        }
        let c = text[i..].chars().next().unwrap();
        out.push(c);
        i += c.len_utf8();
    }
    out
}

/// Bare references `[[…]]` (no display text) and variables `|name|` (a
/// name of letters, digits and `_`, words separated by single spaces), in
/// order, outside code spans.
pub fn bikeshed_tokens(text: &str) -> Vec<String> {
    let text = without_code_spans(text);
    let bytes = text.as_bytes();
    let mut tokens = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        if !text.is_char_boundary(i) {
            i += 1;
            continue;
        }
        if text[i..].starts_with("[[") && !text[i..].starts_with("[[=") {
            let line_end = text[i..].find('\n').map_or(text.len(), |n| i + n);
            if let Some(close) = text[i + 2..line_end].find("]]") {
                let inner = &text[i + 2..i + 2 + close];
                if !inner.is_empty() && !inner.contains(['|', '[', ']']) {
                    tokens.push(format!("[[{inner}]]"));
                    i += close + 4;
                    continue;
                }
            }
            i += 1;
            continue;
        }
        if bytes[i] == b'|'
            && let Some(len) = variable_len(&text[i + 1..])
        {
            tokens.push(text[i..i + 1 + len + 1].to_string());
            i += len + 2;
            continue;
        }
        i += 1;
    }
    tokens
}

/// The length of a variable name at the start of `rest` when a closing bar
/// follows it.
fn variable_len(rest: &str) -> Option<usize> {
    let bytes = rest.as_bytes();
    if !bytes
        .first()
        .is_some_and(|b| b.is_ascii_alphabetic() || *b == b'_')
    {
        return None;
    }
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'|' => return (i > 0 && bytes[i - 1] != b' ').then_some(i),
            b' ' if bytes
                .get(i + 1)
                .is_some_and(|b| b.is_ascii_alphanumeric() || *b == b'_') =>
            {
                i += 1
            }
            b if b.is_ascii_alphanumeric() || b == b'_' => i += 1,
            _ => return None,
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use yeokja_core::glossary::Glossary;

    fn context(source: &str, translation: &str) -> EvaluationContext {
        EvaluationContext {
            source: source.to_string(),
            translation: translation.to_string(),
            glossary: Glossary::empty(),
            source_lang: "en".into(),
            target_lang: "ko".into(),
            markup: Markup::Bikeshed,
        }
    }

    #[test]
    fn variables_are_blanked_outside_code_spans() {
        assert_eq!(
            without_variables("Let |node| be `|x|` of |entry name|, not a | b."),
            "Let        be `|x|` of             , not a | b."
        );
    }

    #[test]
    fn tokens_are_bare_references_and_variables_outside_code() {
        assert_eq!(
            bikeshed_tokens(
                "Let |n| be |metadata block| of [[#block]] and [[!AGE]], not [[#x|text]], `|code|` or [[=a/b=]]; a | b."
            ),
            ["|n|", "|metadata block|", "[[#block]]", "[[!AGE]]"]
        );
    }

    #[tokio::test]
    async fn particles_after_the_token_pass_and_changes_fail() {
        let ok = BikeshedEvaluator
            .evaluate(&context(
                "Let |n| be [[#block]].",
                "|n|을 [[#block]]에서 둔다.",
            ))
            .await
            .unwrap();
        assert!(ok.passed);
        let bad = BikeshedEvaluator
            .evaluate(&context(
                "Let |n| be [[#block]] in |file|.",
                "|파일| 안의 [[#block]]에 |n|과 |n|을 둔다.",
            ))
            .await
            .unwrap();
        assert!(!bad.passed);
        let message = &bad.issues[0].message;
        assert!(
            message.contains("missing from the translation: |file|"),
            "{message}"
        );
        assert!(
            message.contains("not in the source: |n|, |파일|")
                || message.contains("not in the source: |n|"),
            "{message}"
        );
    }

    #[tokio::test]
    async fn other_markups_are_not_checked() {
        let ctx = EvaluationContext {
            markup: Markup::Markdown,
            ..context("a |b| c", "a c")
        };
        assert!(BikeshedEvaluator.evaluate(&ctx).await.unwrap().passed);
    }
}
