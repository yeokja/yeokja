//! Fusion: translate with other translations of the same sentences in view.
//!
//! An editor model gets the ordinary translation request plus drafts — the
//! translation already in the state, one made by another model — and writes
//! each sentence anew: it may keep a draft's wording, combine drafts, or
//! ignore them. The drafts are anonymous and framed as fallible, so the one
//! already in use is not deferred to for being there; the reply goes through
//! the same pipeline (tags, evaluators, retries) as any translation.

use crate::provider::{
    CompletionRequest, LlmProvider, TranslateError, TranslateRequest, TranslateResponse, TranslationProvider,
};
use async_trait::async_trait;
use std::collections::BTreeMap;
use std::sync::Arc;

/// The system prompt for the editor.
pub const EDITOR_SYSTEM_PROMPT: &str = "You are a professional translator and editor. Translate accurately while \
preserving the original formatting, technical terms, and structure. Output only the translation.";

const FUSE_RULES: &str = "Draft translations of these sentences follow, written by different translators. They \
are shown as plain markup, not in any tag format. They are drafts, not references: any of them may \
mistranslate, drop or add meaning, lose links or markup, or read unnaturally, and a draft is not better \
for appearing first. Do not assume any draft is correct and do not average them. Write the best \
translation of each numbered sentence yourself: reuse a draft's wording where it is right, combine \
drafts, or write something new. Your answer must follow every rule above, including the [N] format";

/// Asks `llm` for a translation of each request with `drafts` in view.
pub struct Fuser {
    pub llm: Arc<dyn LlmProvider>,
    /// Draft number → segment index (as numbered in the request) → text.
    pub drafts: Vec<BTreeMap<usize, String>>,
}

/// The prompt for `request` with `drafts` appended.
pub fn fuse_prompt(request: &TranslateRequest, drafts: &[BTreeMap<usize, String>]) -> String {
    let mut prompt = crate::prompt::build_prompt(request);
    prompt.push('\n');
    prompt.push_str(FUSE_RULES);
    prompt.push_str(if request.inline_tags { " and the tag format of the sentences.\n" } else { ".\n" });
    for (n, draft) in drafts.iter().enumerate() {
        prompt.push_str(&format!("\nDraft {}:\n", n + 1));
        for (idx, _) in &request.segments {
            if let Some(text) = draft.get(idx) {
                prompt.push_str(&format!("[{idx}] {text}\n"));
            }
        }
    }
    prompt
}

#[async_trait]
impl TranslationProvider for Fuser {
    async fn translate(&self, request: TranslateRequest) -> Result<TranslateResponse, TranslateError> {
        let prompt = fuse_prompt(&request, &self.drafts);
        let response = self.llm.complete(CompletionRequest { prompt }).await?;
        let translations = crate::prompt::parse_response_for(&response.text, &request.segments)
            .map_err(TranslateError::Parse)?;
        Ok(TranslateResponse { translations, usage: response.usage })
    }
}

/// Put `drafts` in an order fixed by `key` but unrelated to where each draft
/// came from, so position says nothing about which one is already in use.
pub fn shuffle(mut drafts: Vec<BTreeMap<usize, String>>, key: &str) -> Vec<BTreeMap<usize, String>> {
    let hash = yeokja_core::hash::content_hash(key);
    let n = drafts.len();
    if n > 1 {
        drafts.rotate_left((hash % n as u64) as usize);
        if (hash / n as u64).is_multiple_of(2) {
            drafts.reverse();
        }
    }
    drafts
}

#[cfg(test)]
mod tests {
    use super::*;
    use yeokja_core::parser::Markup;

    fn request() -> TranslateRequest {
        TranslateRequest {
            segments: vec![(1, "Some topics have a venue.".into()), (2, "Use it.".into())],
            block_context: "Some topics have a venue. Use it.".into(),
            glossary: Default::default(),
            source_lang: "en".into(),
            target_lang: "ko".into(),
            markup: Markup::Markdown,
            feedback: None,
            prompt_template: None,
            paragraphs: Default::default(),
            inline_tags: false,
        }
    }

    #[test]
    fn prompt_lists_every_draft_under_the_request_numbers() {
        let drafts = vec![
            BTreeMap::from([(1, "A1".to_string()), (2, "A2".to_string())]),
            BTreeMap::from([(1, "B1".to_string())]),
        ];
        let prompt = fuse_prompt(&request(), &drafts);
        assert!(prompt.contains("[1] Some topics have a venue."));
        assert!(prompt.contains("Draft 1:\n[1] A1\n[2] A2\n"));
        assert!(prompt.contains("Draft 2:\n[1] B1\n"));
        assert!(prompt.contains("including the [N] format.\n"));
    }

    #[test]
    fn shuffle_is_stable_and_keeps_every_draft() {
        let drafts = vec![BTreeMap::from([(1, "a".to_string())]), BTreeMap::from([(1, "b".to_string())])];
        let once = shuffle(drafts.clone(), "k");
        assert_eq!(once, shuffle(drafts.clone(), "k"));
        let mut texts: Vec<&String> = once.iter().map(|d| &d[&1]).collect();
        texts.sort();
        assert_eq!(texts, ["a", "b"]);
    }
}
