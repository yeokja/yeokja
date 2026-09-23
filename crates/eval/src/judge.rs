//! `yeokja-eval judge`: blind pairwise judging of baseline against candidates.

use crate::gate::Gate;
use crate::item::{Block, Item, item_files, read_jsonl, write_jsonl};
use crate::run::{Candidate, Output, outputs};
use anyhow::{Context, Result, anyhow};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::Semaphore;
use yeokja_core::hash::content_hash;
use yeokja_translate::factory::create_llm_provider;
use yeokja_translate::provider::{CompletionRequest, LlmProvider};

/// A block chosen for judging.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Pick {
    pub item: String,
    pub block: String,
    /// Why it was chosen: `real`, `hard` or `synthetic`.
    pub kind: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Verdicts {
    pub accuracy: String,
    pub fluency: String,
    pub translationese: String,
    pub overall: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pair {
    pub judge: String,
    pub judge_model: String,
    pub baseline: String,
    pub candidate: String,
    pub item: String,
    pub block: String,
    /// `AB`: the baseline was shown as A. `BA`: as B.
    pub order: String,
    /// Each criterion as `baseline`, `candidate` or `tie`.
    pub verdicts: Option<Verdicts>,
    pub reason: Option<String>,
    pub error: Option<String>,
}

const SYSTEM: &str = "You are an expert Korean technical editor judging translations. You answer with JSON only.";

const CLAUDISH: &str = "\
- long relative clauses that follow English word order; overuse of \"~하는 것은 ~이다\"
- needless passives (\"~되어진다\", \"~에 의해 ~된다\")
- \"당신\"/\"여러분\" carried over from an English \"you\" subject
- English-style dashes (—) and parenthetical asides where Korean would restructure
- intensifiers or adverbs the source does not have (\"매우\", \"정말로\", \"효과적으로\")
- noun-phrase headings or list items turned into sentences, or the reverse
- endings or terms that waver within one passage";

fn prompt(item: &Item, block: &Block, a: &str, b: &str) -> String {
    let source: Vec<&str> = block.segments.iter().map(|s| s.source.as_str()).collect();
    let source = source.join(" ");
    let mut glossary: Vec<(&String, &String)> = item
        .request
        .glossary
        .iter()
        .filter(|(term, _)| source.to_lowercase().contains(&term.to_lowercase()))
        .collect();
    glossary.sort();
    let glossary = if glossary.is_empty() {
        "(none)".to_string()
    } else {
        glossary.iter().map(|(t, k)| format!("- {t} → {k}")).collect::<Vec<_>>().join("\n")
    };
    format!(
        "Two Korean translations of the same English technical-documentation passage follow. \
You do not know which system wrote which; judge only the text. Markup (Markdown, reST, LaTeX, \
inline HTML) is part of the text and must survive translation.

Criteria:
1. accuracy — meaning kept: nothing added, dropped or distorted; numbers, names, code, links \
and markup preserved; glossary terms rendered as required.
2. fluency — reads as natural, well-edited Korean technical prose in the formal 합쇼체 register.
3. translationese — which has LESS 번역투. Signs:
{CLAUDISH}
4. overall — which translation you would publish.

For each criterion answer \"A\", \"B\" or \"tie\". Decide whenever you can; use \"tie\" only when \
the two are equally good on that criterion.

Glossary (required renderings):
{glossary}

Source (English):
<<<
{source}
>>>

Translation A:
<<<
{a}
>>>

Translation B:
<<<
{b}
>>>

Reply with one JSON object and nothing else:
{{\"accuracy\": \"A|B|tie\", \"fluency\": \"A|B|tie\", \"translationese\": \"A|B|tie\", \"overall\": \"A|B|tie\", \"reason\": \"<one sentence in Korean>\"}}"
    )
}

#[derive(Deserialize)]
struct Reply {
    accuracy: String,
    fluency: String,
    translationese: String,
    overall: String,
    reason: Option<String>,
}

fn parse(text: &str) -> Option<Reply> {
    let start = text.find('{')?;
    let end = text.rfind('}')?;
    serde_json::from_str(&text[start..=end]).ok()
}

/// A letter answer in terms of the two systems, given which letter was the baseline.
fn side(answer: &str, baseline_is_a: bool) -> String {
    match (answer.trim().to_uppercase().as_str(), baseline_is_a) {
        ("A", true) | ("B", false) => "baseline",
        ("A", false) | ("B", true) => "candidate",
        _ => "tie",
    }
    .to_string()
}

pub fn company(provider_type: &str) -> &'static str {
    match provider_type {
        "claude_code" | "claude-code" | "anthropic" => "anthropic",
        "codex" | "openai" | "openai_compatible" => "openai",
        "gemini" => "google",
        _ => "other",
    }
}

/// The newest stable Flash model the Gemini API lists.
async fn latest_flash(key_env: &str) -> Result<String> {
    let key = std::env::var(key_env).map_err(|_| anyhow!("{key_env} is not set"))?;
    let body: serde_json::Value = reqwest::Client::new()
        .get("https://generativelanguage.googleapis.com/v1beta/models?pageSize=1000")
        .header("x-goog-api-key", key)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    let mut best: Option<(Vec<u32>, String)> = None;
    for model in body["models"].as_array().into_iter().flatten() {
        let Some(name) = model["name"].as_str() else { continue };
        let Some(rest) = name.strip_prefix("models/gemini-") else { continue };
        let Some(version) = rest.strip_suffix("-flash") else { continue };
        let Ok(parts) = version.split('.').map(str::parse::<u32>).collect::<Result<Vec<_>, _>>() else { continue };
        if best.as_ref().is_none_or(|(v, _)| parts > *v) {
            best = Some((parts, rest.to_string()));
        }
    }
    best.map(|(_, name)| format!("gemini-{name}")).ok_or_else(|| anyhow!("no stable Flash model listed"))
}

fn text_of(item: &Item, block: &Block, output: &Output) -> Option<String> {
    let _ = item;
    let parts: Option<Vec<String>> =
        block.segments.iter().map(|s| output.finals.get(&s.index).map(|f| f.translation.clone())).collect();
    Some(parts?.join(" "))
}

/// The frozen judging sample: every hard block, every synthetic block with
/// a probe, and real blocks by seed, a third of them from the harder strata.
pub fn sample(items: &[Item], seed: u64, real: usize) -> Vec<Pick> {
    let mut picks = Vec::new();
    let mut real_pool = Vec::new();
    for item in items {
        for block in &item.blocks {
            let pick = Pick { item: item.id.clone(), block: block.id.clone(), kind: item.kind.clone() };
            match item.kind.as_str() {
                "hard" if block.segments.iter().any(|s| s.reference.is_some()) => picks.push(pick),
                "synthetic"
                    if block.probes.iter().any(|p| !matches!(p.as_str(), "latex-aux" | "noun-heading")) =>
                {
                    picks.push(pick)
                }
                "real"
                    if !block.strata.iter().any(|s| s == "trivial" || s == "heading")
                        && block.segments.iter().map(|s| s.source.chars().count()).sum::<usize>() >= 60 =>
                {
                    let hard = block.strata.iter().any(|s| matches!(s.as_str(), "inline-markup" | "long" | "math"));
                    real_pool.push((hard, content_hash(&format!("{seed}:{}:{}", item.id, block.id)), pick));
                }
                _ => {}
            }
        }
    }
    real_pool.sort_by_key(|(_, k, _)| *k);
    let mut per_item: HashMap<String, usize> = HashMap::new();
    let mut chosen: Vec<Pick> = Vec::new();
    let mut take = |want_hard: Option<bool>, limit: usize, chosen: &mut Vec<Pick>| {
        for (hard, _, pick) in &real_pool {
            if chosen.len() >= limit {
                break;
            }
            if want_hard.is_some_and(|w| w != *hard) || chosen.contains(pick) {
                continue;
            }
            let n = per_item.entry(pick.item.clone()).or_default();
            if *n >= 4 {
                continue;
            }
            *n += 1;
            chosen.push(pick.clone());
        }
    };
    take(Some(true), real / 3, &mut chosen);
    take(None, real, &mut chosen);
    picks.extend(chosen);
    picks.sort();
    picks
}

pub struct Options {
    pub set: PathBuf,
    pub baseline: PathBuf,
    pub candidates: Vec<PathBuf>,
    pub judges: Vec<PathBuf>,
    pub out: PathBuf,
    pub concurrency: usize,
    pub seed: u64,
    pub real: usize,
}

fn candidate_of(run: &Path) -> Result<Candidate> {
    let text = std::fs::read_to_string(run.join("run.toml")).with_context(|| format!("{}/run.toml", run.display()))?;
    Ok(toml::from_str(&text)?)
}

fn passing(run: &Path) -> Result<HashSet<(String, String)>> {
    let gates: Vec<Gate> = read_jsonl(&run.join("gates.jsonl"))
        .with_context(|| format!("run `yeokja-eval gate {}` first", run.display()))?;
    Ok(gates
        .into_iter()
        .filter(|g| g.variant == "final" && g.repeat == 0 && g.pass)
        .map(|g| (g.item, g.block))
        .collect())
}

pub async fn run(opts: Options) -> Result<()> {
    let mut items: Vec<Item> = Vec::new();
    for file in item_files(&opts.set)? {
        items.extend(read_jsonl::<Item>(&file)?);
    }
    let sample_path = opts.set.join("judge-sample.jsonl");
    let picks: Vec<Pick> = if sample_path.exists() {
        read_jsonl(&sample_path)?
    } else {
        let picks = sample(&items, opts.seed, opts.real);
        write_jsonl(&sample_path, &picks)?;
        picks
    };
    let items: HashMap<String, Item> = items.into_iter().map(|i| (i.id.clone(), i)).collect();
    std::fs::create_dir_all(&opts.out)?;

    let baseline = candidate_of(&opts.baseline)?;
    let baseline_rows: HashMap<String, Output> =
        outputs(&opts.baseline)?.into_iter().filter(|o| o.repeat == 0).map(|o| (o.item.clone(), o)).collect();
    let baseline_pass = passing(&opts.baseline)?;

    struct Judge {
        label: String,
        model: String,
        company: &'static str,
        llm: Arc<dyn LlmProvider>,
    }
    let mut judges = Vec::new();
    for path in &opts.judges {
        let mut judge: Candidate = toml::from_str(&std::fs::read_to_string(path)?)?;
        if judge.provider.model == "auto" && judge.provider.provider_type == "gemini" {
            let env = judge.provider.api_key_env.clone().unwrap_or_else(|| "GEMINI_API_KEY".to_string());
            judge.provider.model = latest_flash(&env).await?;
        }
        eprintln!("judge {}: {} {}", judge.label, judge.provider.provider_type, judge.provider.model);
        let llm = create_llm_provider(&judge.provider, SYSTEM).map_err(|e| anyhow!("{e}"))?;
        judges.push(Arc::new(Judge {
            company: company(&judge.provider.provider_type),
            label: judge.label,
            model: judge.provider.model,
            llm,
        }));
    }
    let judges_toml: BTreeMap<String, String> = judges.iter().map(|j| (j.label.clone(), j.model.clone())).collect();
    std::fs::write(
        opts.out.join("judges.toml"),
        toml::to_string(&serde_json::json!({ "judges": judges_toml, "baseline": baseline.label }))?,
    )?;

    let pairs_path = opts.out.join("pairs.jsonl");
    let done: HashSet<(String, String, String, String, String)> = if pairs_path.exists() {
        read_jsonl::<Pair>(&pairs_path)?
            .into_iter()
            .filter(|p| p.error.is_none())
            .map(|p| (p.judge, p.candidate, p.item, p.block, p.order))
            .collect()
    } else {
        HashSet::new()
    };

    let semaphore = Arc::new(Semaphore::new(opts.concurrency.max(1)));
    let writer = Arc::new(tokio::sync::Mutex::new(()));
    let mut handles = Vec::new();
    let mut skipped: BTreeMap<String, usize> = BTreeMap::new();
    for run in &opts.candidates {
        let candidate = candidate_of(run)?;
        let rows: HashMap<String, Output> =
            outputs(run)?.into_iter().filter(|o| o.repeat == 0).map(|o| (o.item.clone(), o)).collect();
        let pass = passing(run)?;
        for pick in &picks {
            let key = (pick.item.clone(), pick.block.clone());
            if !baseline_pass.contains(&key) || !pass.contains(&key) {
                *skipped.entry(candidate.label.clone()).or_default() += 1;
                continue;
            }
            let (Some(item), Some(base), Some(cand)) =
                (items.get(&pick.item), baseline_rows.get(&pick.item), rows.get(&pick.item))
            else {
                continue;
            };
            let Some(block) = item.blocks.iter().find(|b| b.id == pick.block) else { continue };
            let (Some(base_text), Some(cand_text)) = (text_of(item, block, base), text_of(item, block, cand)) else {
                continue;
            };
            for judge in &judges {
                if judge.company == company(&candidate.provider.provider_type)
                    || judge.company == company(&baseline.provider.provider_type)
                {
                    continue;
                }
                for order in ["AB", "BA"] {
                    let done_key = (
                        judge.label.clone(),
                        candidate.label.clone(),
                        pick.item.clone(),
                        pick.block.clone(),
                        order.to_string(),
                    );
                    if done.contains(&done_key) {
                        continue;
                    }
                    let baseline_is_a = order == "AB";
                    let (a, b) = if baseline_is_a { (&base_text, &cand_text) } else { (&cand_text, &base_text) };
                    let text = prompt(item, block, a, b);
                    let judge = judge.clone();
                    let semaphore = semaphore.clone();
                    let writer = writer.clone();
                    let pairs_path = pairs_path.clone();
                    let baseline_label = baseline.label.clone();
                    let candidate_label = candidate.label.clone();
                    let pick = pick.clone();
                    handles.push(tokio::spawn(async move {
                        let _permit = semaphore.acquire().await.expect("semaphore");
                        let mut pair = Pair {
                            judge: judge.label.clone(),
                            judge_model: judge.model.clone(),
                            baseline: baseline_label,
                            candidate: candidate_label,
                            item: pick.item.clone(),
                            block: pick.block.clone(),
                            order: order.to_string(),
                            verdicts: None,
                            reason: None,
                            error: None,
                        };
                        for attempt in 0..3 {
                            match judge.llm.complete(CompletionRequest { prompt: text.clone() }).await {
                                Ok(reply) => match parse(&reply.text) {
                                    Some(r) => {
                                        pair.verdicts = Some(Verdicts {
                                            accuracy: side(&r.accuracy, baseline_is_a),
                                            fluency: side(&r.fluency, baseline_is_a),
                                            translationese: side(&r.translationese, baseline_is_a),
                                            overall: side(&r.overall, baseline_is_a),
                                        });
                                        pair.reason = r.reason;
                                        pair.error = None;
                                        break;
                                    }
                                    None => pair.error = Some(format!("unparsable reply: {}", reply.text.chars().take(200).collect::<String>())),
                                },
                                Err(e) => {
                                    pair.error = Some(e.to_string());
                                    tokio::time::sleep(std::time::Duration::from_secs(10 * (attempt + 1))).await;
                                }
                            }
                        }
                        let _guard = writer.lock().await;
                        let line = crate::item::canonical(&pair).expect("serialize pair");
                        let mut file =
                            std::fs::OpenOptions::new().create(true).append(true).open(&pairs_path).expect("open pairs");
                        use std::io::Write;
                        writeln!(file, "{line}").expect("write pairs");
                    }));
                }
            }
        }
    }
    eprintln!("{} judge calls to make; blocks skipped for failing a gate: {skipped:?}", handles.len());
    let total = handles.len();
    for (n, handle) in handles.into_iter().enumerate() {
        handle.await?;
        if (n + 1) % 50 == 0 || n + 1 == total {
            eprintln!("[{}/{total}] judged", n + 1);
        }
    }

    // One row per key, the latest successful one, sorted.
    let mut rows: BTreeMap<(String, String, String, String, String), Pair> = BTreeMap::new();
    if pairs_path.exists() {
        for pair in read_jsonl::<Pair>(&pairs_path)? {
            let key = (pair.judge.clone(), pair.candidate.clone(), pair.item.clone(), pair.block.clone(), pair.order.clone());
            let keep_old = rows.get(&key).is_some_and(|old| old.error.is_none() && pair.error.is_some());
            if !keep_old {
                rows.insert(key, pair);
            }
        }
    }
    write_jsonl(&pairs_path, &rows.into_values().collect::<Vec<_>>())?;
    Ok(())
}
