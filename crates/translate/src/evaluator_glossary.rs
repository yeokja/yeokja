use crate::evaluator::*;
use async_trait::async_trait;

pub struct GlossaryEvaluator;

#[async_trait]
impl TranslationEvaluator for GlossaryEvaluator {
    async fn evaluate(
        &self,
        context: &EvaluationContext,
    ) -> Result<EvaluationResult, EvaluationError> {
        let source = glossary_source(context.markup, &context.source);

        // The source is read without regard to case, so the translation is too.
        // Demanding the glossary's spelling starts a retry that cannot converge:
        // theBeamBook writes "the beam file format" and "erlang:md5/1" in its
        // own prose, and a translator that keeps the author's spelling gives the
        // same answer however many times it is asked. Case is a house-style
        // question, and this evaluator is the wrong place to have it — it costs
        // a whole block's retranslation per disagreement. `unmet_terms` holds
        // that rule, and the stricter one for whole-segment entries.
        let whole = context.glossary.whole_match(&source).map(|(text, _)| text.to_string());
        let issues: Vec<EvaluationIssue> = context
            .glossary
            .unmet_terms(&source, &context.translation)
            .into_iter()
            .map(|(term, expected)| EvaluationIssue {
                severity: IssueSeverity::Error,
                kind: IssueKind::GlossaryMismatch,
                message: if whole.as_deref() == Some(term.as_str()) {
                    format!("Segment '{term}' should be translated as exactly '{expected}'")
                } else {
                    format!(
                        "Term '{}' should be translated as '{}' but was not found in translation",
                        term, expected
                    )
                },
            })
            .collect();

        Ok(EvaluationResult {
            passed: issues.is_empty(),
            issues,
        })
    }

    fn triggers_retranslation(&self) -> bool {
        true
    }

    fn name(&self) -> &'static str {
        "Glossary"
    }
}

/// The source as glossary checks read it: without the parts of the markup
/// that are names rather than prose (Verso role headers, LaTeX reference
/// arguments). Snapshot adoption reads it the same way as this evaluator.
///
/// A Markdown segment that is nothing but one link to another site names
/// that site's page — `[UVa OJ - Continued Fractions](https://…)` in a
/// problem list — and its words are a title to copy, not prose to render
/// with the glossary.
pub fn glossary_source(markup: Markup, source: &str) -> String {
    match markup {
        Markup::Verso => without_verso_role_headers(source),
        Markup::Latex => crate::evaluator_format::without_latex_reference_arguments(source),
        Markup::Markdown | Markup::MkDocs if is_external_link_only(source) => String::new(),
        _ => source.to_string(),
    }
}

/// Whether `source` is exactly `[label](http…)`.
fn is_external_link_only(source: &str) -> bool {
    let source = source.trim();
    let Some(rest) = source.strip_prefix('[') else { return false };
    let Some(close) = rest.rfind("](") else { return false };
    let (label, target) = (&rest[..close], &rest[close + 2..]);
    let Some(target) = target.strip_suffix(')') else { return false };
    !label.contains("](")
        && (target.starts_with("http://") || target.starts_with("https://"))
        && !target.contains(char::is_whitespace)
}

/// Remove opaque `{role arguments}` headers while leaving visible labels and
/// surrounding prose available to glossary matching. Backticked role payloads
/// are already masked by `find_terms_in_text` itself.
fn without_verso_role_headers(text: &str) -> String {
    let mut output = String::with_capacity(text.len());
    let mut copied_through = 0;
    let mut at = 0;
    while at < text.len() {
        if text[at..].starts_with('{') {
            let name_start = at + 1;
            let starts_as_role = text[name_start..]
                .chars()
                .next()
                .is_some_and(|ch| ch.is_ascii_alphabetic());
            if starts_as_role && let Some(close_offset) = text[name_start..].find('}') {
                let end = name_start + close_offset + 1;
                output.push_str(&text[copied_through..at]);
                copied_through = end;
                at = end;
                continue;
            }
        }
        let ch = text[at..].chars().next().unwrap();
        at += ch.len_utf8();
    }
    output.push_str(&text[copied_through..]);
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_context(
        source: &str,
        translation: &str,
        glossary: Vec<(&str, &str)>,
    ) -> EvaluationContext {
        EvaluationContext {
            source: source.to_string(),
            translation: translation.to_string(),
            glossary: glossary
                .into_iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect(),
            source_lang: "en".to_string(),
            target_lang: "ko".to_string(),
            markup: Markup::Asciidoc,
        }
    }

    #[tokio::test]
    async fn except_phrases_do_not_demand_the_term() {
        let glossary = yeokja_core::glossary::Glossary::from_toml(
            r#"
[terms.future]
translation = "퓨처"
except = ["in the future"]
"#,
        )
        .unwrap();
        let ctx = EvaluationContext {
            glossary,
            ..make_context(
                "More generators may be added in the future.",
                "앞으로 생성기가 더 추가될 수 있습니다.",
                vec![],
            )
        };
        let result = GlossaryEvaluator.evaluate(&ctx).await.unwrap();
        assert!(result.passed, "{:?}", result.issues);
    }

    #[tokio::test]
    async fn passes_when_glossary_terms_used_correctly() {
        let ctx = make_context(
            "The repository is ready.",
            "저장소가 준비되었습니다.",
            vec![("repository", "저장소")],
        );
        let result = GlossaryEvaluator.evaluate(&ctx).await.unwrap();
        assert!(result.passed);
        assert!(result.issues.is_empty());
    }

    #[tokio::test]
    async fn fails_when_glossary_term_not_used() {
        let ctx = make_context(
            "The repository is ready.",
            "레포지토리가 준비되었습니다.",
            vec![("repository", "저장소")],
        );
        let result = GlossaryEvaluator.evaluate(&ctx).await.unwrap();
        assert!(!result.passed);
        assert_eq!(result.issues.len(), 1);
        assert_eq!(result.issues[0].kind, IssueKind::GlossaryMismatch);
    }

    /// A term the source itself spells in another case. The glossary says to
    /// leave "Erlang" alone, and a translation that left `erlang:md5/1` alone
    /// did exactly that.
    #[tokio::test]
    async fn passes_when_the_case_is_the_source_author_s() {
        let ctx = make_context(
            "The key is scrambled using erlang:md5/1.",
            "키는 erlang:md5/1을 사용하여 스크램블됩니다.",
            vec![("Erlang", "Erlang")],
        );
        assert!(GlossaryEvaluator.evaluate(&ctx).await.unwrap().passed);
    }

    #[tokio::test]
    async fn passes_when_term_not_in_source() {
        let ctx = make_context("Hello world.", "안녕 세계.", vec![("repository", "저장소")]);
        let result = GlossaryEvaluator.evaluate(&ctx).await.unwrap();
        assert!(result.passed);
    }

    #[tokio::test]
    async fn verso_role_names_and_code_payloads_are_not_prose_terms() {
        let mut ctx = make_context(
            "Use {tactic}`simp` and {anchorName Monad}`Monad`.",
            "{tactic}`simp`와 {anchorName Monad}`Monad`를 사용합니다.",
            vec![("tactic", "택틱"), ("monad", "모나드")],
        );
        ctx.markup = Markup::Verso;
        let result = GlossaryEvaluator.evaluate(&ctx).await.unwrap();
        assert!(result.passed, "{:?}", result.issues);
    }

    #[tokio::test]
    async fn verso_still_checks_the_same_term_in_visible_prose() {
        let mut ctx = make_context(
            "This monad uses {anchorName Monad}`Monad`.",
            "이 모나드는 {anchorName Monad}`Monad`를 사용합니다.",
            vec![("monad", "모나드")],
        );
        ctx.markup = Markup::Verso;
        assert!(GlossaryEvaluator.evaluate(&ctx).await.unwrap().passed);

        ctx.translation = "이 추상화는 {anchorName Monad}`Monad`를 사용합니다.".to_string();
        assert!(!GlossaryEvaluator.evaluate(&ctx).await.unwrap().passed);
    }
    #[tokio::test]
    async fn latex_reference_keys_are_not_glossary_prose() {
        let mut ctx = make_context(
            r"See \cref{sec:compute-universe}, \crefrange{sec:path}{sec:universe}, and \href{https://example.org/universe}{this chapter}.",
            r"\cref{sec:compute-universe}, \crefrange{sec:path}{sec:universe} 및 \href{https://example.org/universe}{이 장}을 보십시오.",
            vec![("universe", "유니버스"), ("path", "경로")],
        );
        ctx.markup = Markup::Latex;
        assert!(GlossaryEvaluator.evaluate(&ctx).await.unwrap().passed);
        ctx.source.push_str(" The universe contains this path.");
        assert!(!GlossaryEvaluator.evaluate(&ctx).await.unwrap().passed);
    }

    #[tokio::test]
    async fn whole_segment_entries_need_the_exact_rendering() {
        let glossary = yeokja_core::glossary::Glossary::from_toml(
            r#"
[whole."Example:"]
translation = "예"
"#,
        )
        .unwrap();
        let context = |translation: &str| EvaluationContext {
            source: "Example:".to_string(),
            translation: translation.to_string(),
            glossary: glossary.clone(),
            source_lang: "en".to_string(),
            target_lang: "ko".to_string(),
            markup: Markup::Markdown,
        };
        assert!(GlossaryEvaluator.evaluate(&context("예:")).await.unwrap().passed);
        let result = GlossaryEvaluator.evaluate(&context("예시는 다음과 같습니다:")).await.unwrap();
        assert!(!result.passed);
        assert!(result.issues[0].message.contains("exactly '예'"));
    }

    #[tokio::test]
    async fn a_segment_that_is_only_an_external_link_is_a_title() {
        let glossary = yeokja_core::glossary::Glossary::from_toml("[terms.\"continued fractions\"]\ntranslation = \"연분수\"\n").unwrap();
        let context = |source: &str, translation: &str| EvaluationContext {
            source: source.to_string(),
            translation: translation.to_string(),
            glossary: glossary.clone(),
            source_lang: "en".to_string(),
            target_lang: "ko".to_string(),
            markup: Markup::MkDocs,
        };
        let title = "[UVa OJ - Continued Fractions](https://onlinejudge.org/x)";
        assert!(GlossaryEvaluator.evaluate(&context(title, title)).await.unwrap().passed);
        let prose = "See [continued fractions](https://example.org) here.";
        assert!(!GlossaryEvaluator.evaluate(&context(prose, "여기를 보십시오.")).await.unwrap().passed);
        let local = "[Continued fractions](continued-fractions.md)";
        assert!(!GlossaryEvaluator.evaluate(&context(local, local)).await.unwrap().passed);
    }
}
