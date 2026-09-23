//! The person's share of the judging: pick blind pairs for the judging page
//! and bring its verdicts back as `human.jsonl`.

use crate::gate::Gate;
use crate::item::{Item, item_files, read_jsonl, write_jsonl};
use crate::judge::Pick;
use crate::report::Human;
use crate::run::{Candidate, Output, outputs};
use anyhow::{Context, Result, anyhow};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::{Path, PathBuf};
use yeokja_core::hash::content_hash;

/// One pair as the page shows it: no model names.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PagePair {
    pub id: String,
    pub n: usize,
    pub kind: String,
    pub project: String,
    pub parser: String,
    pub source: String,
    pub glossary: Vec<String>,
    pub a: String,
    pub b: String,
}

/// What the page's ids stand for. Kept in the repository only.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Blind {
    pub id: String,
    pub item: String,
    pub block: String,
    pub candidate: String,
    pub baseline: String,
    /// `a` or `b`: where the baseline was shown.
    pub baseline_side: String,
}

fn label(run: &Path) -> Result<String> {
    let c: Candidate = toml::from_str(&std::fs::read_to_string(run.join("run.toml"))?)?;
    Ok(c.label)
}

fn passing(run: &Path) -> Result<HashSet<(String, String)>> {
    let gates: Vec<Gate> = read_jsonl(&run.join("gates.jsonl"))?;
    Ok(gates.into_iter().filter(|g| g.variant == "final" && g.repeat == 0 && g.pass).map(|g| (g.item, g.block)).collect())
}

pub fn pairs(set: &Path, baseline: &Path, candidates: &[PathBuf], out: &Path, count: usize, seed: u64) -> Result<()> {
    let mut items: HashMap<String, Item> = HashMap::new();
    for file in item_files(set)? {
        for item in read_jsonl::<Item>(&file)? {
            items.insert(item.id.clone(), item);
        }
    }
    let picks: Vec<Pick> = read_jsonl(&set.join("judge-sample.jsonl"))?;
    let base_label = label(baseline)?;
    let base_rows: HashMap<String, Output> =
        outputs(baseline)?.into_iter().filter(|o| o.repeat == 0).map(|o| (o.item.clone(), o)).collect();
    let base_pass = passing(baseline)?;
    struct Cand {
        label: String,
        rows: HashMap<String, Output>,
        pass: HashSet<(String, String)>,
    }
    let cands: Vec<Cand> = candidates
        .iter()
        .map(|run| {
            Ok(Cand {
                label: label(run)?,
                rows: outputs(run)?.into_iter().filter(|o| o.repeat == 0).map(|o| (o.item.clone(), o)).collect(),
                pass: passing(run)?,
            })
        })
        .collect::<Result<_>>()?;

    // Blocks every candidate and the baseline passed, in seed order; about
    // a fifth from hard and synthetic blocks, the rest real.
    let mut eligible: Vec<&Pick> = picks
        .iter()
        .filter(|p| {
            let key = (p.item.clone(), p.block.clone());
            base_pass.contains(&key) && cands.iter().all(|c| c.pass.contains(&key))
        })
        .collect();
    eligible.sort_by_key(|p| content_hash(&format!("human:{seed}:{}:{}", p.item, p.block)));
    let special = count / 5;
    let mut chosen: Vec<&Pick> = eligible.iter().copied().filter(|p| p.kind != "real").take(special).collect();
    chosen.extend(eligible.iter().copied().filter(|p| p.kind == "real").take(count - chosen.len()));

    let text_of = |item: &Item, block: &crate::item::Block, row: &Output| -> Option<String> {
        let parts: Option<Vec<String>> =
            block.segments.iter().map(|s| row.finals.get(&s.index).map(|f| f.translation.clone())).collect();
        let _ = item;
        Some(parts?.join(" "))
    };
    let mut page = Vec::new();
    let mut blind = Vec::new();
    for (n, pick) in chosen.iter().enumerate() {
        let cand = &cands[n % cands.len()];
        let item = &items[&pick.item];
        let block = item.blocks.iter().find(|b| b.id == pick.block).ok_or_else(|| anyhow!("block {}", pick.block))?;
        let base = text_of(item, block, &base_rows[&pick.item]).ok_or_else(|| anyhow!("baseline text"))?;
        let other = text_of(item, block, &cand.rows[&pick.item]).ok_or_else(|| anyhow!("candidate text"))?;
        let baseline_a = content_hash(&format!("side:{seed}:{n}")).is_multiple_of(2);
        let (a, b) = if baseline_a { (base, other) } else { (other, base) };
        let source: Vec<&str> = block.segments.iter().map(|s| s.source.as_str()).collect();
        let source = source.join(" ");
        let lower = source.to_lowercase();
        let mut glossary: Vec<String> = item
            .request
            .glossary
            .iter()
            .filter(|(t, _)| lower.contains(&t.to_lowercase()))
            .map(|(t, k)| format!("{t} → {k}"))
            .collect();
        glossary.sort();
        let id = format!("p{:02}", n + 1);
        page.push(PagePair {
            id: id.clone(),
            n: n + 1,
            kind: pick.kind.clone(),
            project: item.project.clone(),
            parser: item.parser.clone(),
            source,
            glossary,
            a,
            b,
        });
        blind.push(Blind {
            id,
            item: pick.item.clone(),
            block: pick.block.clone(),
            candidate: cand.label.clone(),
            baseline: base_label.clone(),
            baseline_side: if baseline_a { "a" } else { "b" }.to_string(),
        });
    }
    std::fs::create_dir_all(out)?;
    // The page's copy stays out of the repository's diffs of judgments: it
    // repeats licensed text and is only the seed for the page's store.
    std::fs::write(out.join("page-pairs.json"), serde_json::to_string_pretty(&page)?)?;
    write_jsonl(&out.join("blind-map.jsonl"), &blind)?;
    eprintln!("{} pairs ({special} hard/synthetic)", page.len());
    Ok(())
}

#[derive(Deserialize)]
struct PageVerdict {
    accuracy: Option<String>,
    fluency: Option<String>,
    translationese: Option<String>,
    overall: Option<String>,
    note: Option<String>,
}

/// `verdicts`: a JSON object of page verdicts by pair id (the page store's
/// `verdicts` collection, as `ArtifactData list` returns it, flattened).
pub fn import(out: &Path, verdicts: &Path) -> Result<()> {
    let blind: Vec<Blind> = read_jsonl(&out.join("blind-map.jsonl"))?;
    let raw: BTreeMap<String, PageVerdict> = serde_json::from_str(
        &std::fs::read_to_string(verdicts).with_context(|| format!("read {}", verdicts.display()))?,
    )?;
    let side = |answer: &Option<String>, baseline_side: &str| -> Option<String> {
        Some(match (answer.as_deref()?, baseline_side) {
            ("A", "a") | ("B", "b") => "baseline",
            ("A", "b") | ("B", "a") => "candidate",
            _ => "tie",
        }
        .to_string())
    };
    let mut rows = Vec::new();
    for b in &blind {
        let Some(v) = raw.get(&b.id) else { continue };
        let (Some(accuracy), Some(fluency), Some(translationese), Some(overall)) = (
            side(&v.accuracy, &b.baseline_side),
            side(&v.fluency, &b.baseline_side),
            side(&v.translationese, &b.baseline_side),
            side(&v.overall, &b.baseline_side),
        ) else {
            continue;
        };
        rows.push(Human {
            candidate: b.candidate.clone(),
            item: b.item.clone(),
            block: b.block.clone(),
            accuracy,
            fluency,
            translationese,
            overall,
            note: v.note.clone().filter(|n| !n.trim().is_empty()),
        });
    }
    eprintln!("{} of {} pairs judged", rows.len(), blind.len());
    write_jsonl(&out.join("human.jsonl"), &rows)?;
    Ok(())
}
