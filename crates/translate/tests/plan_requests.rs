//! `plan_requests` must build exactly the requests a translation from scratch
//! sends, and a planned request must survive serialization unchanged — the
//! eval set replays them from JSON.

use async_trait::async_trait;
use std::sync::{Arc, Mutex};
use yeokja_core::config::ProjectConfig;
use yeokja_core::glossary::Glossary;
use yeokja_translate::orchestrator::{
    CancelToken, Orchestrator, ParserFactory, PlannedRequest, TranslateOptions, plan_requests,
};
use yeokja_translate::provider::{TranslateError, TranslateRequest, TranslateResponse, TranslationProvider};

const SOURCE: &str = "# Title

A sentence with **bold** and a [reference][docs]. Another one with `code`.

- An item with *emphasis*.
- A second item that links to [the book](https://example.com/book).

> [!NOTE]
> A note that spans
> two lines.

Closing paragraph with $x^2$ math and a [shortcut].

[docs]: https://example.com/docs
[shortcut]: https://example.com/shortcut
";

fn config(dir: &std::path::Path, batch: usize) -> ProjectConfig {
    ProjectConfig::from_toml(&format!(
        r#"
[project]
source_lang = "en"
target_lang = "ko"

[[sources]]
path = "{}"
pattern = "**/*.md"
parser = "markdown"
output = "{{dir}}/{{stem}}.ko{{ext}}"
inline_tags = true

[provider]
type = "openai_compatible"
model = "test"

[translation]
batch_segments = {batch}
"#,
        dir.display()
    ))
    .unwrap()
}

/// Records every request and echoes its segments back.
struct Recording(Arc<Mutex<Vec<TranslateRequest>>>);

#[async_trait]
impl TranslationProvider for Recording {
    async fn translate(&self, request: TranslateRequest) -> Result<TranslateResponse, TranslateError> {
        self.0.lock().unwrap().push(request.clone());
        Ok(TranslateResponse {
            translations: request.segments.iter().map(|(i, t)| (*i, t.clone())).collect(),
            usage: None,
        })
    }
}

fn value<T: serde::Serialize>(item: &T) -> serde_json::Value {
    serde_json::to_value(item).unwrap()
}

/// Requests in a stable order: files translate blocks concurrently.
fn sorted(mut requests: Vec<serde_json::Value>) -> Vec<serde_json::Value> {
    requests.sort_by_key(|r| r.to_string());
    requests
}

async fn check(batch: usize) {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("ch1.md");
    std::fs::write(&file, SOURCE).unwrap();
    let config = config(dir.path(), batch);
    let glossary = Glossary::empty();
    let factory: ParserFactory = Arc::new(yeokja_parsers::select_parser);

    let planned = plan_requests(&file, &config, &glossary, &factory).unwrap();
    assert!(!planned.is_empty());

    let sent = Arc::new(Mutex::new(Vec::new()));
    let orchestrator = Orchestrator {
        config: Arc::new(config.clone()),
        glossary: Arc::new(glossary),
        provider: Arc::new(Recording(sent.clone())),
        eval_provider: None,
        parser_factory: factory,
        options: TranslateOptions { auto_evaluate: false, style_evaluate: false, max_retries: 0, concurrency: 1 },
        cancel: CancelToken::default(),
    };
    orchestrator.translate_path(dir.path(), None).await.unwrap();

    let sent: Vec<serde_json::Value> = sent.lock().unwrap().iter().map(value).collect();
    let planned_requests: Vec<serde_json::Value> = planned.iter().map(|p| value(&p.request)).collect();
    assert_eq!(sorted(planned_requests), sorted(sent));
}

#[tokio::test]
async fn planned_requests_match_sent_requests_one_block_each() {
    check(1).await;
}

#[tokio::test]
async fn planned_requests_match_sent_requests_batched() {
    check(32).await;
}

#[test]
fn planned_request_round_trips_through_json() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("ch1.md");
    std::fs::write(&file, SOURCE).unwrap();
    let config = config(dir.path(), 32);
    let factory: ParserFactory = Arc::new(yeokja_parsers::select_parser);
    let planned = plan_requests(&file, &config, &Glossary::empty(), &factory).unwrap();

    for request in &planned {
        assert!(request.inline.is_some(), "the fixture is taggable");
        let json = serde_json::to_string(request).unwrap();
        let back: PlannedRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(value(&back), value(request));
        // Serializing twice gives the same bytes (sets are written sorted).
        assert_eq!(serde_json::to_string(&value(request)).unwrap(), serde_json::to_string(&value(&back)).unwrap());

        // The replayed tags read an echoed reply the way the originals do.
        let inline = request.inline.as_ref().unwrap();
        let inline_back = back.inline.as_ref().unwrap();
        for (idx, text) in &request.request.segments {
            let original = &inline.tagged[idx];
            let replayed = &inline_back.tagged[idx];
            let a = yeokja_translate::inline::markdown::read(text, original).unwrap();
            let b = yeokja_translate::inline::markdown::read(text, replayed).unwrap();
            let ra = yeokja_translate::inline::markdown::render(&a, original, &inline.ctx);
            let rb = yeokja_translate::inline::markdown::render(&b, replayed, &inline_back.ctx);
            assert_eq!(ra.markdown, rb.markdown);
            assert_eq!(ra.markdown.trim(), original.source.trim());
        }
    }
    // Every segment of every block appears once, numbered in its request.
    let segments: usize = planned.iter().flat_map(|p| &p.blocks).map(|b| b.segments.len()).sum();
    let numbered: usize = planned.iter().map(|p| p.request.segments.len()).sum();
    assert_eq!(segments, numbered);
}

#[test]
fn requests_carry_their_surroundings_and_only_the_glossary_entries_they_use() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("ch1.md");
    std::fs::write(&file, SOURCE).unwrap();
    let config = config(dir.path(), 1);
    let factory: ParserFactory = Arc::new(yeokja_parsers::select_parser);
    let glossary = Glossary::from_toml("[terms.sentence]\ntranslation = \"문장\"\n[terms.zebra]\ntranslation = \"얼룩말\"\n").unwrap();
    let planned = plan_requests(&file, &config, &glossary, &factory).unwrap();

    let first_item = planned
        .iter()
        .find(|p| p.request.segments.iter().any(|(_, t)| t.contains("An item with")))
        .expect("a request for the first list item");
    let around = &first_item.request.surroundings;
    assert!(around.contains("Under heading: Title"), "{around}");
    assert!(around.contains("Before: A sentence with"), "{around}");
    assert!(around.contains("After: A second item"), "{around}");

    let paragraph = planned
        .iter()
        .find(|p| p.request.segments.iter().any(|(_, t)| t.starts_with("A sentence with")))
        .unwrap();
    assert_eq!(paragraph.request.glossary.keys().collect::<Vec<_>>(), vec!["sentence"]);
    assert!(first_item.request.glossary.is_empty());
}
