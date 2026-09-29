//! `yeokja-eval view`: one local HTML page that shows every candidate's
//! translation of each block side by side, with the gates and the judges'
//! reasons. The page repeats licensed source text, so it stays out of the
//! repository.

use crate::gate::Gate;
use crate::item::{Item, item_files, read_jsonl};
use crate::judge::{Pair, Pick};
use crate::run::{Candidate, Output, outputs};
use anyhow::Result;
use serde::Serialize;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::{Path, PathBuf};

pub struct Options {
    pub set: PathBuf,
    /// Run directories; empty means every directory under `<eval>/runs`.
    pub runs: Vec<PathBuf>,
    /// Judgment directories; empty means every directory under `<eval>/judgments`.
    pub judgments: Vec<PathBuf>,
    pub out: PathBuf,
}

#[derive(Serialize)]
struct Page {
    candidates: Vec<String>,
    blocks: Vec<ViewBlock>,
}

#[derive(Serialize)]
struct ViewBlock {
    item: String,
    block: String,
    kind: String,
    project: String,
    parser: String,
    strata: Vec<String>,
    probes: Vec<String>,
    sampled: bool,
    source: String,
    glossary: Vec<String>,
    reference: Option<String>,
    known_bad: Option<String>,
    translations: BTreeMap<String, Translation>,
    judgments: Vec<Judgment>,
}

#[derive(Serialize)]
struct Translation {
    text: String,
    /// The model's first reply, when the retry loop changed it.
    first: Option<String>,
    attempts: u32,
    pass: Option<bool>,
    issues: Vec<String>,
}

/// Both orders of one judge's pair, folded as the report folds them.
#[derive(Serialize)]
struct Judgment {
    judge: String,
    judge_model: String,
    baseline: String,
    candidate: String,
    /// Criterion → `baseline`, `candidate` or `tie`.
    verdicts: BTreeMap<String, String>,
    reasons: Vec<String>,
}

/// (judge, baseline, candidate) → both orders of that judge's pair.
type Groups<'a> = BTreeMap<(String, String, String), Vec<&'a Pair>>;

fn subdirs(dir: &Path) -> Result<Vec<PathBuf>> {
    let mut dirs: Vec<PathBuf> =
        std::fs::read_dir(dir)?.filter_map(|e| e.ok().map(|e| e.path())).filter(|p| p.is_dir()).collect();
    dirs.sort();
    Ok(dirs)
}

/// A block's text in a run: its segments joined, `None` if one is missing.
fn joined(segments: &[crate::item::Segment], text: impl Fn(usize) -> Option<String>) -> Option<String> {
    let parts: Option<Vec<String>> = segments.iter().map(|s| text(s.index)).collect();
    Some(parts?.join(" "))
}

fn joined_opt(
    segments: &[crate::item::Segment],
    text: impl Fn(&crate::item::Segment) -> Option<&String>,
) -> Option<String> {
    let parts: Vec<&str> = segments.iter().filter_map(|s| text(s).map(String::as_str)).collect();
    if parts.is_empty() { None } else { Some(parts.join(" ")) }
}

fn fold_pairs(pairs: &[&Pair]) -> Option<Judgment> {
    let first = pairs.first()?;
    let mut verdicts = BTreeMap::new();
    for criterion in ["overall", "accuracy", "fluency", "translationese"] {
        let mut seen = pairs.iter().filter_map(|p| p.verdicts.as_ref()).map(|v| match criterion {
            "accuracy" => v.accuracy.clone(),
            "fluency" => v.fluency.clone(),
            "translationese" => v.translationese.clone(),
            _ => v.overall.clone(),
        });
        let a = seen.next()?;
        // A single order stands alone; disagreeing orders are a tie.
        let folded = if seen.all(|b| b == a) { a } else { "tie".to_string() };
        verdicts.insert(criterion.to_string(), folded);
    }
    Some(Judgment {
        judge: first.judge.clone(),
        judge_model: first.judge_model.clone(),
        baseline: first.baseline.clone(),
        candidate: first.candidate.clone(),
        verdicts,
        reasons: pairs.iter().filter_map(|p| p.reason.clone()).collect(),
    })
}

/// JSON that is safe inside a `<script>` element.
fn script_json<T: Serialize>(value: &T) -> Result<String> {
    Ok(serde_json::to_string(value)?.replace("</", "<\\/").replace("<!--", "<\\!--"))
}

pub fn run(opts: Options) -> Result<()> {
    let root = opts.set.parent().unwrap_or(Path::new("."));
    let runs = if opts.runs.is_empty() { subdirs(&root.join("runs"))? } else { opts.runs.clone() };
    let judgment_dirs =
        if opts.judgments.is_empty() { subdirs(&root.join("judgments"))? } else { opts.judgments.clone() };

    let mut items: Vec<Item> = Vec::new();
    for file in item_files(&opts.set)? {
        items.extend(read_jsonl::<Item>(&file)?);
    }
    let sampled: HashSet<(String, String)> =
        read_jsonl::<Pick>(&opts.set.join("judge-sample.jsonl"))?.into_iter().map(|p| (p.item, p.block)).collect();

    struct Run {
        label: String,
        rows: HashMap<String, Output>,
        gates: HashMap<(String, String), Gate>,
    }
    let mut loaded = Vec::new();
    for dir in &runs {
        let candidate: Candidate = toml::from_str(&std::fs::read_to_string(dir.join("run.toml"))?)?;
        let rows = outputs(dir)?.into_iter().filter(|o| o.repeat == 0).map(|o| (o.item.clone(), o)).collect();
        let gates_path = dir.join("gates.jsonl");
        let gates = if gates_path.exists() {
            read_jsonl::<Gate>(&gates_path)?
                .into_iter()
                .filter(|g| g.variant == "final" && g.repeat == 0)
                .map(|g| ((g.item.clone(), g.block.clone()), g))
                .collect()
        } else {
            HashMap::new()
        };
        // Runs can share a label (a contaminated fusion run keeps its
        // candidate's); such a run is shown under its directory name.
        let label = if loaded.iter().any(|r: &Run| r.label == candidate.label) {
            dir.file_name().and_then(|n| n.to_str()).unwrap_or(&candidate.label).to_string()
        } else {
            candidate.label
        };
        loaded.push(Run { label, rows, gates });
    }

    let mut pairs: Vec<Pair> = Vec::new();
    for dir in &judgment_dirs {
        let path = dir.join("pairs.jsonl");
        if path.exists() {
            pairs.extend(read_jsonl::<Pair>(&path)?.into_iter().filter(|p| p.error.is_none()));
        }
    }
    // (item, block) → (judge, baseline, candidate) → both orders.
    let mut by_block: HashMap<(String, String), Groups> = HashMap::new();
    for pair in &pairs {
        by_block
            .entry((pair.item.clone(), pair.block.clone()))
            .or_default()
            .entry((pair.judge.clone(), pair.baseline.clone(), pair.candidate.clone()))
            .or_default()
            .push(pair);
    }

    let mut blocks = Vec::new();
    for item in &items {
        for block in &item.blocks {
            let key = (item.id.clone(), block.id.clone());
            let mut translations = BTreeMap::new();
            for run in &loaded {
                let Some(row) = run.rows.get(&item.id) else {
                    continue;
                };
                let Some(text) = joined(&block.segments, |i| row.finals.get(&i).map(|f| f.translation.clone())) else {
                    continue;
                };
                let first = joined(&block.segments, |i| row.first.get(&i).cloned().flatten()).filter(|f| *f != text);
                let attempts = block
                    .segments
                    .iter()
                    .filter_map(|s| row.finals.get(&s.index))
                    .map(|f| f.attempts)
                    .max()
                    .unwrap_or(0);
                let gate = run.gates.get(&key);
                translations.insert(
                    run.label.clone(),
                    Translation {
                        text,
                        first,
                        attempts,
                        pass: gate.map(|g| g.pass),
                        issues: gate.map(|g| g.issues.clone()).unwrap_or_default(),
                    },
                );
            }
            if translations.is_empty() {
                continue;
            }
            let source = block.segments.iter().map(|s| s.source.as_str()).collect::<Vec<_>>().join(" ");
            let lower = source.to_lowercase();
            let mut glossary: Vec<String> = item
                .request
                .glossary
                .iter()
                .filter(|(t, _)| lower.contains(&t.to_lowercase()))
                .map(|(t, k)| format!("{t} → {k}"))
                .collect();
            glossary.sort();
            let judgments = by_block
                .get(&key)
                .map(|groups| groups.values().filter_map(|ps| fold_pairs(ps)).collect())
                .unwrap_or_default();
            blocks.push(ViewBlock {
                item: item.id.clone(),
                block: block.id.clone(),
                kind: item.kind.clone(),
                project: item.project.clone(),
                parser: item.parser.clone(),
                strata: block.strata.clone(),
                probes: block.probes.clone(),
                sampled: sampled.contains(&key),
                source,
                glossary,
                reference: joined_opt(&block.segments, |s| s.reference.as_ref()),
                known_bad: joined_opt(&block.segments, |s| s.known_bad.as_ref()),
                translations,
                judgments,
            });
        }
    }

    let page = Page { candidates: loaded.iter().map(|r| r.label.clone()).collect(), blocks };
    let html = TEMPLATE.replace("/*DATA*/null", &script_json(&page)?);
    if let Some(parent) = opts.out.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&opts.out, html)?;
    eprintln!("wrote {} ({} blocks, {} runs)", opts.out.display(), page.blocks.len(), page.candidates.len());
    Ok(())
}

const TEMPLATE: &str = include_str!("view.html");

#[cfg(test)]
mod tests {
    use super::*;
    use crate::judge::Verdicts;

    fn pair(order: &str, overall: &str, accuracy: &str) -> Pair {
        Pair {
            judge: "j".into(),
            judge_model: "m".into(),
            baseline: "b".into(),
            candidate: "c".into(),
            item: "i".into(),
            block: "k".into(),
            order: order.into(),
            verdicts: Some(Verdicts {
                accuracy: accuracy.into(),
                fluency: "tie".into(),
                translationese: "tie".into(),
                overall: overall.into(),
            }),
            reason: Some(order.into()),
            error: None,
        }
    }

    #[test]
    fn folds_disagreeing_orders_into_a_tie() {
        let (a, b) = (pair("AB", "candidate", "baseline"), pair("BA", "candidate", "candidate"));
        let j = fold_pairs(&[&a, &b]).unwrap();
        assert_eq!(j.verdicts["overall"], "candidate");
        assert_eq!(j.verdicts["accuracy"], "tie");
        assert_eq!(j.reasons, ["AB", "BA"]);
    }

    #[test]
    fn script_json_cannot_close_the_script() {
        let s = script_json(&"</script><!--").unwrap();
        assert!(!s.contains("</script>") && !s.contains("<!--"));
    }
}
