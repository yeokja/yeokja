//! `yeokja-eval run`: replay every frozen request through a candidate model.

use crate::item::{Item, bucket_of, item_files, read_jsonl, write_jsonl};
use anyhow::{Context, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Instant;
use tokio::sync::Semaphore;
use yeokja_core::config::ProviderConfig;
use yeokja_core::hash::content_hash;
use yeokja_translate::evaluator::TranslationEvaluator;
use yeokja_translate::factory::create_provider;
use yeokja_translate::fuse::{EDITOR_SYSTEM_PROMPT, Fuser};
use yeokja_translate::orchestrator::evaluators_for;
use yeokja_translate::pipeline::translate_with_evaluation_inline;
use yeokja_translate::provider::{
    TokenUsage, TranslateError, TranslateRequest, TranslateResponse, TranslationProvider,
};

/// A candidate: a label and the `[provider]` it translates with.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Candidate {
    pub label: String,
    pub provider: ProviderConfig,
}

/// One call to the model, as the pipeline made it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Call {
    pub duration_ms: u128,
    pub feedback: Option<String>,
    pub requested: Vec<usize>,
    /// The reply per segment as the model wrote it (tag text for inline requests).
    pub replies: BTreeMap<usize, String>,
    pub usage: Option<TokenUsage>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Final {
    pub translation: String,
    pub attempts: u32,
    pub issues: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Output {
    pub item: String,
    pub candidate: String,
    /// Repetition number, 0 unless `--repeat` asked for more.
    pub repeat: u32,
    pub calls: Vec<Call>,
    /// The first reply per segment, as Markdown/markup (inline tags rendered);
    /// `None` when the model's tags did not hold.
    pub first: BTreeMap<usize, Option<String>>,
    /// What the pipeline kept after its retries.
    pub finals: BTreeMap<usize, Final>,
    /// The request failed as a whole (after infrastructure retries).
    pub error: Option<String>,
    pub duration_ms: u128,
}

/// Wraps the candidate's provider to keep every call.
struct Recorder {
    inner: Arc<dyn TranslationProvider>,
    calls: Mutex<Vec<Call>>,
}

#[async_trait]
impl TranslationProvider for Recorder {
    async fn translate(&self, request: TranslateRequest) -> Result<TranslateResponse, TranslateError> {
        let started = Instant::now();
        let result = self.inner.translate(request.clone()).await;
        let call = Call {
            duration_ms: started.elapsed().as_millis(),
            feedback: request.feedback.clone(),
            requested: request.segments.iter().map(|(i, _)| *i).collect(),
            replies: result
                .as_ref()
                .map(|r| r.translations.iter().map(|(i, t)| (*i, t.clone())).collect())
                .unwrap_or_default(),
            usage: result.as_ref().ok().and_then(|r| r.usage.clone()),
            error: result.as_ref().err().map(|e| e.to_string()),
        };
        self.calls.lock().unwrap().push(call);
        result
    }
}

pub struct Options {
    pub set: PathBuf,
    /// Runs whose final translations are the drafts (fusion when non-empty).
    pub drafts: Vec<PathBuf>,
    /// Replace the first draft with the corrected-away translation where the
    /// set has one, and run only those items: does a bad draft leak through?
    pub contaminate: bool,
    pub candidate: PathBuf,
    pub out: PathBuf,
    pub concurrency: usize,
    pub max_retries: u32,
    pub repeat: u32,
    pub subset: Option<usize>,
    pub only: Option<String>,
}

fn first_attempt(item: &Item, calls: &[Call]) -> BTreeMap<usize, Option<String>> {
    let Some(first) = calls.iter().find(|c| c.error.is_none()) else { return BTreeMap::new() };
    first
        .replies
        .iter()
        .map(|(idx, reply)| {
            let rendered = match &item.inline {
                Some(batch) => batch.tagged.get(idx).and_then(|tagged| {
                    let tree = yeokja_translate::inline::markdown::read(reply, tagged).ok()?;
                    Some(yeokja_translate::inline::markdown::render(&tree, tagged, &batch.ctx).markdown)
                }),
                None => Some(reply.clone()),
            };
            (*idx, rendered)
        })
        .collect()
}

fn transient(error: &TranslateError) -> bool {
    matches!(error, TranslateError::Http(_) | TranslateError::Api { .. } | TranslateError::RateLimited { .. })
}

async fn translate_item(
    item: &Item,
    provider: Arc<dyn TranslationProvider>,
    candidate: &str,
    repeat: u32,
    max_retries: u32,
) -> Output {
    let started = Instant::now();
    let evaluators = evaluators_for(None, &item.request.target_lang, item.inline.is_some());
    let evaluator_refs: Vec<&dyn TranslationEvaluator> = evaluators.iter().map(|e| e.as_ref()).collect();
    let markup = item.request.markup;
    let mut all_calls = Vec::new();
    let mut outcome = Err(String::new());
    for infra_try in 0..3u64 {
        let recorder = Arc::new(Recorder { inner: provider.clone(), calls: Mutex::new(Vec::new()) });
        let result = translate_with_evaluation_inline(
            recorder.as_ref(),
            &evaluator_refs,
            item.request.clone(),
            &item.request.glossary,
            &item.request.source_lang,
            &item.request.target_lang,
            markup,
            max_retries,
            item.inline.as_ref(),
        )
        .await;
        all_calls.extend(recorder.calls.lock().unwrap().drain(..));
        match result {
            Ok(results) => {
                outcome = Ok(results);
                break;
            }
            Err(e) if transient(&e) && infra_try < 2 => {
                tracing::warn!(item = %item.id, error = %e, "Transient failure; retrying the request");
                tokio::time::sleep(std::time::Duration::from_secs(20 * (infra_try + 1))).await;
            }
            Err(e) => {
                outcome = Err(e.to_string());
                break;
            }
        }
    }
    let (finals, error) = match outcome {
        Ok(results) => (
            results
                .into_iter()
                .map(|(idx, r)| {
                    let issues = r.evaluation.map(|e| e.issues.iter().map(|i| i.message.clone()).collect()).unwrap_or_default();
                    (idx, Final { translation: r.translation, attempts: r.attempts, issues })
                })
                .collect(),
            None,
        ),
        Err(e) => (BTreeMap::new(), Some(e)),
    };
    Output {
        item: item.id.clone(),
        candidate: candidate.to_string(),
        repeat,
        first: first_attempt(item, &all_calls),
        calls: all_calls,
        finals,
        error,
        duration_ms: started.elapsed().as_millis(),
    }
}

/// TOML has no null: leave absent values out.
pub fn without_nulls(value: serde_json::Value) -> serde_json::Value {
    match value {
        serde_json::Value::Object(map) => serde_json::Value::Object(
            map.into_iter().filter(|(_, v)| !v.is_null()).map(|(k, v)| (k, without_nulls(v))).collect(),
        ),
        serde_json::Value::Array(items) => serde_json::Value::Array(items.into_iter().map(without_nulls).collect()),
        other => other,
    }
}

fn tool_version(provider: &str) -> Option<String> {
    let bin = match provider {
        "claude_code" => "claude",
        "codex" => "codex",
        _ => return None,
    };
    let out = std::process::Command::new(bin).arg("--version").output().ok()?;
    Some(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

/// Items in the order this seed picks for a subset (see `--subset`).
fn subset(items: Vec<(String, Item)>, n: usize) -> Vec<(String, Item)> {
    let mut items = items;
    items.sort_by_key(|(_, i)| content_hash(&format!("subset:{}", i.id)));
    items.truncate(n);
    items
}

pub async fn run(opts: Options) -> Result<()> {
    let candidate: Candidate = toml::from_str(
        &std::fs::read_to_string(&opts.candidate).with_context(|| format!("read {}", opts.candidate.display()))?,
    )?;
    let provider = create_provider(&candidate.provider).map_err(|e| anyhow::anyhow!("{e}"))?;
    let editor = if opts.drafts.is_empty() {
        None
    } else {
        Some(
            yeokja_translate::factory::create_llm_provider(&candidate.provider, EDITOR_SYSTEM_PROMPT)
                .map_err(|e| anyhow::anyhow!("{e}"))?,
        )
    };
    let mut draft_rows: Vec<std::collections::HashMap<String, Output>> = Vec::new();
    for run in &opts.drafts {
        draft_rows.push(outputs(run)?.into_iter().filter(|o| o.repeat == 0).map(|o| (o.item.clone(), o)).collect());
    }
    std::fs::create_dir_all(&opts.out)?;

    let mut items: Vec<(String, Item)> = Vec::new();
    for file in item_files(&opts.set)? {
        let bucket = bucket_of(&file);
        for item in read_jsonl::<Item>(&file)? {
            if opts.only.as_deref().is_none_or(|p| item.id.contains(p)) {
                items.push((bucket.clone(), item));
            }
        }
    }
    if opts.contaminate {
        items.retain(|(_, i)| i.blocks.iter().flat_map(|b| &b.segments).any(|s| s.known_bad.is_some()));
    }
    if let Some(n) = opts.subset {
        items = subset(items, n);
    }
    let head = std::process::Command::new("git").args(["rev-parse", "HEAD"]).output()?;
    let head = String::from_utf8_lossy(&head.stdout).trim().to_string();
    let prompts: String = items.iter().map(|(_, i)| yeokja_translate::prompt::build_prompt(&i.request)).collect();
    let set_commits: HashSet<&str> = items.iter().map(|(_, i)| i.yeokja_commit.as_str()).collect();
    let mut provider_record = candidate.provider.clone();
    provider_record.api_key_env = provider_record.api_key_env.map(|name| format!("{name} (value not recorded)"));
    let run_toml = toml::to_string(&without_nulls(serde_json::json!({
        "label": candidate.label,
        "provider": provider_record,
        "tool_version": tool_version(&candidate.provider.provider_type),
        "yeokja_commit": head,
        "set": opts.set.display().to_string(),
        "set_yeokja_commits": set_commits.iter().collect::<Vec<_>>(),
        "prompt_hash": format!("{:016x}", content_hash(&prompts)),
        "items": items.len(),
        "repeat": opts.repeat,
        "max_retries": opts.max_retries,
        "style_evaluate": false,
        "drafts": opts.drafts.iter().map(|d| d.display().to_string()).collect::<Vec<_>>(),
        "contaminate": opts.contaminate,
        "started_at": chrono::Utc::now().to_rfc3339(),
    })))?;
    if !set_commits.contains(head.as_str()) {
        eprintln!("note: running yeokja {head} on a set frozen at {set_commits:?}");
    }
    std::fs::write(opts.out.join("run.toml"), run_toml)?;

    // Resume: rows already written stay.
    let path_of = |bucket: &str| opts.out.join(format!("outputs.{bucket}.jsonl"));
    let mut done: HashSet<(String, u32)> = HashSet::new();
    let buckets: HashSet<String> = items.iter().map(|(b, _)| b.clone()).collect();
    for bucket in &buckets {
        if path_of(bucket).exists() {
            for row in read_jsonl::<Output>(&path_of(bucket))? {
                if row.error.is_none() {
                    done.insert((row.item, row.repeat));
                }
            }
        }
    }

    let semaphore = Arc::new(Semaphore::new(opts.concurrency.max(1)));
    let writer = Arc::new(tokio::sync::Mutex::new(()));
    let total = items.len() as u32 * opts.repeat;
    let finished = Arc::new(std::sync::atomic::AtomicU32::new(done.len() as u32));
    let mut handles = Vec::new();
    for repeat in 0..opts.repeat {
        for (bucket, item) in items.iter().cloned() {
            if done.contains(&(item.id.clone(), repeat)) {
                continue;
            }
            let provider: Arc<dyn TranslationProvider> = match &editor {
                None => provider.clone(),
                Some(llm) => {
                    let mut drafts: Vec<BTreeMap<usize, String>> = draft_rows
                        .iter()
                        .map(|rows| {
                            rows.get(&item.id)
                                .map(|o| o.finals.iter().map(|(i, f)| (*i, f.translation.clone())).collect())
                                .unwrap_or_default()
                        })
                        .collect();
                    if opts.contaminate && let Some(first) = drafts.first_mut() {
                        for seg in item.blocks.iter().flat_map(|b| &b.segments) {
                            if let Some(bad) = &seg.known_bad {
                                first.insert(seg.index, bad.clone());
                            }
                        }
                    }
                    // Anonymous and shuffled per item, the same way every run.
                    if content_hash(&format!("drafts:{}", item.id)).is_multiple_of(2) {
                        drafts.reverse();
                    }
                    Arc::new(Fuser { llm: llm.clone(), drafts })
                }
            };
            let semaphore = semaphore.clone();
            let writer = writer.clone();
            let label = candidate.label.clone();
            let path = path_of(&bucket);
            let finished = finished.clone();
            let max_retries = opts.max_retries;
            handles.push(tokio::spawn(async move {
                let _permit = semaphore.acquire().await.expect("semaphore");
                let output = translate_item(&item, provider, &label, repeat, max_retries).await;
                let _guard = writer.lock().await;
                let line = crate::item::canonical(&output).expect("serialize output");
                let mut file = std::fs::OpenOptions::new().create(true).append(true).open(&path).expect("open outputs");
                use std::io::Write;
                writeln!(file, "{line}").expect("write outputs");
                let n = finished.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1;
                eprintln!(
                    "[{n}/{total}] {} {} ({:.0}s{})",
                    label,
                    item.id,
                    output.duration_ms as f64 / 1000.0,
                    output.error.as_deref().map(|e| format!(", error: {e}")).unwrap_or_default()
                );
            }));
        }
    }
    for handle in handles {
        handle.await?;
    }

    // Keep one row per (item, repeat), the latest, sorted, so a finished run
    // is a stable file.
    for bucket in &buckets {
        let path = path_of(bucket);
        if !path.exists() {
            continue;
        }
        let mut rows: BTreeMap<(String, u32), Output> = BTreeMap::new();
        for row in read_jsonl::<Output>(&path)? {
            let key = (row.item.clone(), row.repeat);
            let keep_old = rows.get(&key).is_some_and(|old| old.error.is_none() && row.error.is_some());
            if !keep_old {
                rows.insert(key, row);
            }
        }
        write_jsonl(&path, &rows.into_values().collect::<Vec<_>>())?;
    }
    Ok(())
}

pub fn outputs(dir: &Path) -> Result<Vec<Output>> {
    let mut rows = Vec::new();
    for entry in std::fs::read_dir(dir)? {
        let path = entry?.path();
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or_default().to_string();
        if name.starts_with("outputs.") && name.ends_with(".jsonl") {
            rows.extend(read_jsonl::<Output>(&path)?);
        }
    }
    Ok(rows)
}
