//! `yeokja-eval gate`: mechanical pass/fail per block, for the first reply
//! and for what the retry loop kept.

use crate::item::{Item, item_files, read_jsonl, write_jsonl};
use crate::run::{Output, outputs};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use yeokja_translate::evaluator::{EvaluationContext, IssueSeverity, TranslationEvaluator};
use yeokja_translate::inline::audit::Audit;
use yeokja_translate::orchestrator::evaluators_for;
use yeokja_translate::pipeline::repair_reply;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Gate {
    pub candidate: String,
    pub item: String,
    pub block: String,
    pub repeat: u32,
    /// `first` (the model's first reply) or `final` (after the retry loop).
    pub variant: String,
    pub pass: bool,
    /// Check name → passed.
    pub checks: BTreeMap<String, bool>,
    pub issues: Vec<String>,
    /// The inline audit repaired this block's markup (counted apart).
    pub repaired: bool,
}

#[derive(Deserialize)]
struct Truncation {
    min_ratio: f64,
}

const ALERTS: &[&str] = &["[!NOTE]", "[!TIP]", "[!IMPORTANT]", "[!WARNING]", "[!CAUTION]"];

fn alert_markers(text: &str) -> Vec<String> {
    let mut found = Vec::new();
    let mut rest = text;
    while let Some(at) = rest.find("[!") {
        let tail = &rest[at..];
        let end = tail.find(']').map(|e| e + 1).unwrap_or(tail.len());
        found.push(tail[..end].to_string());
        rest = &tail[end.min(tail.len())..];
        if end == 0 {
            break;
        }
    }
    found.sort();
    found
}

fn sentences(text: &str) -> usize {
    let chars: Vec<char> = text.chars().collect();
    let mut n = 0;
    for (i, c) in chars.iter().enumerate() {
        if matches!(c, '.' | '?' | '!' | '。') {
            let next = chars.get(i + 1).copied();
            let prev = if i > 0 { chars[i - 1] } else { ' ' };
            // Not a decimal point, an ellipsis or `e.g.` inside a word.
            if (next.is_none() || next.is_some_and(char::is_whitespace)) && !prev.is_ascii_digit() {
                n += 1;
            }
        }
    }
    n.max(1)
}

/// Why a translation looks cut short, if it does.
fn truncated(source: &str, translation: &str, min_ratio: f64) -> Option<String> {
    let s = source.chars().count();
    if s < 40 {
        return None;
    }
    let ratio = translation.chars().count() as f64 / s as f64;
    if ratio < min_ratio {
        return Some(format!("length ratio {ratio:.2} < {min_ratio:.2}"));
    }
    let (a, b) = (sentences(source), sentences(translation));
    if a >= 2 && b * 2 < a {
        return Some(format!("{b} sentences for {a}"));
    }
    None
}

async fn check_block(
    item: &Item,
    block: &crate::item::Block,
    translations: &HashMap<usize, Option<String>>,
    whole: &HashMap<usize, String>,
    evaluators: &[Box<dyn TranslationEvaluator>],
    min_ratio: f64,
) -> (BTreeMap<String, bool>, Vec<String>, bool) {
    let mut checks: BTreeMap<String, bool> = BTreeMap::new();
    let mut issues = Vec::new();
    let mut repaired = false;
    let fail = |checks: &mut BTreeMap<String, bool>, name: &str| {
        checks.insert(name.to_string(), false);
    };
    for name in ["present", "evaluators", "audit", "alignment", "alert-markers", "truncation"] {
        checks.insert(name.to_string(), true);
    }
    let batch: Vec<(usize, String)> = item
        .blocks
        .iter()
        .flat_map(|b| &b.segments)
        .map(|s| (s.index, s.source.clone()))
        .collect();
    let misaligned: Vec<usize> =
        yeokja_translate::alignment::misaligned(&batch, whole, &item.request.paragraphs, item.request.markup)
            .into_iter()
            .map(|m| m.idx)
            .collect();
    for seg in &block.segments {
        let Some(Some(translation)) = translations.get(&seg.index) else {
            fail(&mut checks, "present");
            issues.push(format!("[{}] no usable translation", seg.index));
            continue;
        };
        let ctx = EvaluationContext {
            source: seg.source.clone(),
            translation: translation.clone(),
            glossary: item.request.glossary.clone(),
            source_lang: item.request.source_lang.clone(),
            target_lang: item.request.target_lang.clone(),
            markup: item.request.markup,
        };
        for evaluator in evaluators {
            if let Ok(result) = evaluator.evaluate(&ctx).await
                && !result.passed
            {
                fail(&mut checks, "evaluators");
                for issue in result.issues.iter().filter(|i| i.severity == IssueSeverity::Error) {
                    issues.push(format!("[{}] {}: {}", seg.index, evaluator.name(), issue.message));
                }
            }
        }
        if let Some(batch) = &item.inline
            && let Some(tagged) = batch.tagged.get(&seg.index)
        {
            match yeokja_translate::inline::audit::audit(translation, tagged, &batch.ctx) {
                Audit::Sound => {}
                Audit::Repaired { defects, .. } => {
                    repaired = true;
                    fail(&mut checks, "audit");
                    issues.push(format!("[{}] audit repaired: {}", seg.index, defects.join(", ")));
                }
                Audit::Unrepairable { defects, .. } => {
                    fail(&mut checks, "audit");
                    issues.push(format!("[{}] audit: {}", seg.index, defects.join(", ")));
                }
            }
        }
        if misaligned.contains(&seg.index) {
            fail(&mut checks, "alignment");
            issues.push(format!("[{}] carries another segment's anchors", seg.index));
        }
        let wanted: Vec<String> = alert_markers(&seg.source).into_iter().filter(|m| ALERTS.contains(&m.as_str())).collect();
        let written: Vec<String> = alert_markers(translation);
        if written != wanted {
            fail(&mut checks, "alert-markers");
            issues.push(format!("[{}] alert markers {:?}, source has {:?}", seg.index, written, wanted));
        }
        if let Some(why) = truncated(&seg.source, translation, min_ratio) {
            fail(&mut checks, "truncation");
            issues.push(format!("[{}] truncated? {why}", seg.index));
        }
    }
    (checks, issues, repaired)
}

pub async fn gate_run(set: &Path, run: &Path) -> Result<Vec<Gate>> {
    let mut items: HashMap<String, Item> = HashMap::new();
    for file in item_files(set)? {
        for item in read_jsonl::<Item>(&file)? {
            items.insert(item.id.clone(), item);
        }
    }
    let truncation: Truncation = toml::from_str(&std::fs::read_to_string(set.join("truncation.toml"))?)?;
    let mut gates = Vec::new();
    let mut rows: Vec<Output> = outputs(run)?;
    rows.sort_by(|a, b| (&a.item, a.repeat).cmp(&(&b.item, b.repeat)));
    for row in rows {
        let Some(item) = items.get(&row.item) else { continue };
        let evaluators = evaluators_for(None, &item.request.target_lang, item.inline.is_some());
        let source_of: HashMap<usize, &str> =
            item.blocks.iter().flat_map(|b| &b.segments).map(|s| (s.index, s.source.as_str())).collect();
        let first: HashMap<usize, Option<String>> = row
            .first
            .iter()
            .map(|(idx, t)| {
                let t = t.as_ref().map(|t| match &item.inline {
                    Some(_) => t.clone(),
                    None => repair_reply(item.request.markup, source_of.get(idx).copied().unwrap_or(""), t),
                });
                (*idx, t)
            })
            .collect();
        let finals: HashMap<usize, Option<String>> =
            row.finals.iter().map(|(idx, f)| (*idx, Some(f.translation.clone()))).collect();
        for (variant, translations) in [("first", &first), ("final", &finals)] {
            let whole: HashMap<usize, String> =
                translations.iter().filter_map(|(i, t)| t.clone().map(|t| (*i, t))).collect();
            for block in &item.blocks {
                let (checks, mut issues, repaired) =
                    check_block(item, block, translations, &whole, &evaluators, truncation.min_ratio).await;
                if let Some(e) = &row.error {
                    issues.insert(0, format!("request failed: {e}"));
                }
                gates.push(Gate {
                    candidate: row.candidate.clone(),
                    item: row.item.clone(),
                    block: block.id.clone(),
                    repeat: row.repeat,
                    variant: variant.to_string(),
                    pass: row.error.is_none() && checks.values().all(|ok| *ok),
                    checks,
                    issues,
                    repaired,
                });
            }
        }
    }
    Ok(gates)
}

pub async fn run(set: PathBuf, runs: Vec<PathBuf>) -> Result<()> {
    for run in runs {
        let gates = gate_run(&set, &run).await?;
        let total = gates.iter().filter(|g| g.variant == "final").count();
        let passed = gates.iter().filter(|g| g.variant == "final" && g.pass).count();
        let first_passed = gates.iter().filter(|g| g.variant == "first" && g.pass).count();
        eprintln!(
            "{}: final {passed}/{total}, first {first_passed}/{total}",
            run.display()
        );
        write_jsonl(&run.join("gates.jsonl"), &gates)?;
    }
    Ok(())
}
