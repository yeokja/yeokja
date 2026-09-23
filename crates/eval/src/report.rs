//! `yeokja-eval report`: gate rates, blind win rates and the verdict of the
//! design's decision rule, as Markdown.

use crate::gate::Gate;
use crate::item::{Item, item_files, read_jsonl};
use crate::judge::{Pair, Pick};
use crate::run::{Candidate, outputs};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::fmt::Write;
use std::path::{Path, PathBuf};

/// A person's verdict on one blind pair, mapped to systems.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Human {
    pub candidate: String,
    pub item: String,
    pub block: String,
    pub accuracy: String,
    pub fluency: String,
    pub translationese: String,
    pub overall: String,
    pub note: Option<String>,
}

pub struct Options {
    pub set: PathBuf,
    pub baseline: PathBuf,
    pub candidates: Vec<PathBuf>,
    pub judgments: PathBuf,
    pub out: PathBuf,
}

#[derive(Default, Clone, Copy)]
struct Tally {
    wins: usize,
    losses: usize,
    ties: usize,
}

impl Tally {
    fn add(&mut self, side: &str) {
        match side {
            "candidate" => self.wins += 1,
            "baseline" => self.losses += 1,
            _ => self.ties += 1,
        }
    }
    fn decided(&self) -> usize {
        self.wins + self.losses
    }
    fn rate(&self) -> Option<f64> {
        (self.decided() > 0).then(|| self.wins as f64 / self.decided() as f64)
    }
    /// Wilson score interval, 95%.
    fn wilson(&self) -> Option<(f64, f64)> {
        let n = self.decided() as f64;
        if n == 0.0 {
            return None;
        }
        let z = 1.96;
        let p = self.wins as f64 / n;
        let centre = p + z * z / (2.0 * n);
        let spread = z * ((p * (1.0 - p) + z * z / (4.0 * n)) / n).sqrt();
        let denom = 1.0 + z * z / n;
        Some(((centre - spread) / denom, (centre + spread) / denom))
    }
    fn cell(&self) -> String {
        match (self.rate(), self.wilson()) {
            (Some(r), Some((lo, hi))) => format!(
                "{:.0}% [{:.0}–{:.0}] ({}승 {}패 {}무)",
                r * 100.0,
                lo * 100.0,
                hi * 100.0,
                self.wins,
                self.losses,
                self.ties
            ),
            _ => format!("— ({}무)", self.ties),
        }
    }
}

/// Both orders of a pair folded into one verdict: agreeing orders count,
/// disagreeing orders are a tie.
fn fold(a: &str, b: &str) -> String {
    if a == b { a.to_string() } else { "tie".to_string() }
}

/// Fewer human verdicts than this cannot vouch for a judge.
const MIN_HUMAN: usize = 20;

/// (judge, candidate, item, block).
type PairKey = (String, String, String, String);

const CRITERIA: [&str; 4] = ["overall", "accuracy", "fluency", "translationese"];

fn criterion<'a>(v: &'a crate::judge::Verdicts, name: &str) -> &'a str {
    match name {
        "accuracy" => &v.accuracy,
        "fluency" => &v.fluency,
        "translationese" => &v.translationese,
        _ => &v.overall,
    }
}

fn pct(n: usize, d: usize) -> String {
    if d == 0 { "—".to_string() } else { format!("{:.1}% ({n}/{d})", n as f64 * 100.0 / d as f64) }
}

pub fn run(opts: Options) -> Result<()> {
    let mut items: HashMap<String, Item> = HashMap::new();
    for file in item_files(&opts.set)? {
        for item in read_jsonl::<Item>(&file)? {
            items.insert(item.id.clone(), item);
        }
    }
    let picks: Vec<Pick> = read_jsonl(&opts.set.join("judge-sample.jsonl"))?;
    let label_of = |run: &Path| -> Result<String> {
        let c: Candidate = toml::from_str(&std::fs::read_to_string(run.join("run.toml"))?)?;
        Ok(c.label)
    };
    let baseline = label_of(&opts.baseline)?;
    let mut md = String::new();
    writeln!(md, "# 한국어 번역 eval 보고서\n")?;
    writeln!(md, "- 세트: `{}`", opts.set.display())?;
    writeln!(md, "- 기준선: `{baseline}`")?;
    writeln!(md, "- 채점: `{}`\n", opts.judgments.display())?;

    // 1. Gates and costs, per run.
    writeln!(md, "## 1. 자동 관문\n")?;
    writeln!(md, "블록 단위 통과율. 첫 시도는 모델의 첫 응답(운영과 같은 기계 수선 후), 최종은 재시도 루프를 거친 결과입니다.\n")?;
    writeln!(md, "| 후보 | 최종 통과 | 첫 시도 통과 | 판정 표본 최종 통과 | 마크업 감사 수선 | 요청 실패 | 모델 호출 | 출력 토큰 | 소요 시간(합) |")?;
    writeln!(md, "|---|---|---|---|---|---|---|---|---|")?;
    let mut runs: Vec<PathBuf> = vec![opts.baseline.clone()];
    runs.extend(opts.candidates.iter().cloned());
    let sample: std::collections::HashSet<(String, String)> =
        picks.iter().map(|p| (p.item.clone(), p.block.clone())).collect();
    let mut gate_rates: BTreeMap<String, (f64, f64)> = BTreeMap::new();
    let mut check_fail: BTreeMap<String, BTreeMap<String, usize>> = BTreeMap::new();
    for run in &runs {
        let label = label_of(run)?;
        let gates: Vec<Gate> = read_jsonl(&run.join("gates.jsonl"))?;
        let gates: Vec<&Gate> = gates.iter().filter(|g| g.repeat == 0).collect();
        let finals: Vec<&&Gate> = gates.iter().filter(|g| g.variant == "final").collect();
        let firsts: Vec<&&Gate> = gates.iter().filter(|g| g.variant == "first").collect();
        let fp = finals.iter().filter(|g| g.pass).count();
        let ip = firsts.iter().filter(|g| g.pass).count();
        let in_sample: Vec<&&&Gate> =
            finals.iter().filter(|g| sample.contains(&(g.item.clone(), g.block.clone()))).collect();
        let sp = in_sample.iter().filter(|g| g.pass).count();
        let repaired = finals.iter().filter(|g| g.repaired).count();
        for g in &finals {
            for (name, ok) in &g.checks {
                if !ok {
                    *check_fail.entry(label.clone()).or_default().entry(name.clone()).or_default() += 1;
                }
            }
        }
        let rows = outputs(run)?;
        let rows: Vec<_> = rows.iter().filter(|o| o.repeat == 0).collect();
        let failed = rows.iter().filter(|o| o.error.is_some()).count();
        let calls: usize = rows.iter().map(|o| o.calls.len()).sum();
        let tokens: u64 =
            rows.iter().flat_map(|o| &o.calls).filter_map(|c| c.usage.as_ref()).map(|u| u.output_tokens).sum();
        let seconds: u128 = rows.iter().map(|o| o.duration_ms).sum::<u128>() / 1000;
        gate_rates.insert(
            label.clone(),
            (fp as f64 / finals.len().max(1) as f64, ip as f64 / firsts.len().max(1) as f64),
        );
        writeln!(
            md,
            "| {label} | {} | {} | {} | {repaired} | {failed}/{} | {calls} | {tokens} | {}분 |",
            pct(fp, finals.len()),
            pct(ip, firsts.len()),
            pct(sp, in_sample.len()),
            rows.len(),
            seconds / 60
        )?;
    }
    writeln!(md, "\n실패한 검사(최종, 블록 수):\n")?;
    let names = ["present", "evaluators", "audit", "alignment", "alert-markers", "truncation"];
    writeln!(md, "| 후보 | {} |", names.join(" | "))?;
    writeln!(md, "|---|{}", "---|".repeat(names.len()))?;
    for (label, fails) in &check_fail {
        let cells: Vec<String> = names.iter().map(|n| fails.get(*n).copied().unwrap_or(0).to_string()).collect();
        writeln!(md, "| {label} | {} |", cells.join(" | "))?;
    }

    // 2. Blind pairwise judging.
    let pairs: Vec<Pair> = read_jsonl(&opts.judgments.join("pairs.jsonl")).unwrap_or_default();
    // (judge, candidate, item, block) → per criterion, folded over orders.
    let mut by_key: BTreeMap<PairKey, BTreeMap<String, Vec<String>>> = BTreeMap::new();
    let mut errors = 0;
    for pair in &pairs {
        let Some(v) = &pair.verdicts else {
            errors += 1;
            continue;
        };
        let entry = by_key
            .entry((pair.judge.clone(), pair.candidate.clone(), pair.item.clone(), pair.block.clone()))
            .or_default();
        for c in CRITERIA {
            entry.entry(c.to_string()).or_default().push(criterion(v, c).to_string());
        }
    }
    let folded: BTreeMap<PairKey, BTreeMap<String, String>> = by_key
        .into_iter()
        .filter(|(_, v)| v.values().all(|sides| sides.len() == 2))
        .map(|(k, v)| (k, v.into_iter().map(|(c, sides)| (c, fold(&sides[0], &sides[1]))).collect()))
        .collect();
    // Too few human verdicts say nothing about the judges; leave them out.
    let humans: Vec<Human> = read_jsonl(&opts.judgments.join("human.jsonl")).unwrap_or_default();
    let humans: Vec<Human> = if humans.len() >= MIN_HUMAN { humans } else { Vec::new() };

    writeln!(md, "\n## 2. 익명 쌍대 비교 (기준선 `{baseline}` 대비 후보 승률)\n")?;
    writeln!(
        md,
        "두 관문을 모두 통과한 판정 표본 블록만 비교합니다. A/B 순서를 바꾼 두 판정이 같을 때만 승패로 세고, 엇갈리면 무승부입니다. 괄호 안은 윌슨 95% 신뢰 구간, 승률은 무승부를 뺀 값입니다. 판정 오류 {errors}건.\n"
    )?;
    let judges: Vec<String> = {
        let mut j: Vec<String> = folded.keys().map(|k| k.0.clone()).collect();
        j.sort();
        j.dedup();
        j
    };
    let candidates: Vec<String> = opts.candidates.iter().map(|r| label_of(r)).collect::<Result<_>>()?;
    let mut tallies: BTreeMap<(String, String, String), Tally> = BTreeMap::new();
    for ((judge, cand, _, _), verdicts) in &folded {
        for (c, side) in verdicts {
            tallies.entry((judge.clone(), cand.clone(), c.clone())).or_default().add(side);
        }
    }
    for (c, side) in humans.iter().flat_map(|h| {
        [("overall", &h.overall), ("accuracy", &h.accuracy), ("fluency", &h.fluency), ("translationese", &h.translationese)]
            .into_iter()
            .map(move |(c, s)| ((h.candidate.clone(), c), s))
    }) {
        tallies.entry(("사람".to_string(), c.0, c.1.to_string())).or_default().add(side);
    }
    let mut all_judges = judges.clone();
    if !humans.is_empty() {
        all_judges.push("사람".to_string());
    }
    for criterion_name in CRITERIA {
        let title = match criterion_name {
            "overall" => "종합",
            "accuracy" => "정확성",
            "fluency" => "자연스러움",
            _ => "번역투(적을수록 승)",
        };
        writeln!(md, "### {title}\n")?;
        writeln!(md, "| 채점자 | {} |", candidates.join(" | "))?;
        writeln!(md, "|---|{}", "---|".repeat(candidates.len()))?;
        for judge in &all_judges {
            let cells: Vec<String> = candidates
                .iter()
                .map(|c| {
                    tallies
                        .get(&(judge.clone(), c.clone(), criterion_name.to_string()))
                        .map(Tally::cell)
                        .unwrap_or_else(|| "—".to_string())
                })
                .collect();
            writeln!(md, "| {judge} | {} |", cells.join(" | "))?;
        }
        writeln!(md)?;
    }

    // 3. Agreement with the person.
    let mut agreement: BTreeMap<String, (usize, usize)> = BTreeMap::new();
    if !humans.is_empty() {
        writeln!(md, "## 3. 사람 판정과의 일치율 (종합)\n")?;
        for h in &humans {
            for judge in &judges {
                if let Some(v) = folded.get(&(judge.clone(), h.candidate.clone(), h.item.clone(), h.block.clone())) {
                    let e = agreement.entry(judge.clone()).or_default();
                    e.1 += 1;
                    if v.get("overall").map(String::as_str) == Some(h.overall.as_str()) {
                        e.0 += 1;
                    }
                }
            }
        }
        writeln!(md, "| 채점자 | 일치율 | 결론에 사용 |\n|---|---|---|")?;
        for (judge, (hit, n)) in &agreement {
            let ok = *n > 0 && *hit as f64 / *n as f64 >= 0.7;
            writeln!(md, "| {judge} | {} | {} |", pct(*hit, *n), if ok { "예" } else { "아니요 (70% 미만)" })?;
        }
        writeln!(md)?;
    }

    // Judges against each other, where both judged the same pair.
    let mut inter: (usize, usize) = (0, 0);
    if judges.len() >= 2 {
        writeln!(md, "## 3-1. 채점자 간 일치율 (종합)\n")?;
        for ((judge, cand, item, block), v) in &folded {
            if judge != &judges[0] {
                continue;
            }
            for other in &judges[1..] {
                if let Some(w) = folded.get(&(other.clone(), cand.clone(), item.clone(), block.clone())) {
                    inter.1 += 1;
                    if v.get("overall") == w.get("overall") {
                        inter.0 += 1;
                    }
                }
            }
        }
        writeln!(md, "`{}`와 `{}`가 모두 판정한 쌍에서 종합 판정(순서를 바꾼 두 판정을 합친 것)이 같은 비율: {}\n", judges[0], judges[1..].join("`, `"), pct(inter.0, inter.1))?;
    }

    // 4. Breakdown by project and parser (overall, per judge folded).
    writeln!(md, "## 4. 프로젝트·파서별 종합 승률\n")?;
    for judge in &judges {
        writeln!(md, "### {judge}\n")?;
        let mut by_group: BTreeMap<(String, String), Tally> = BTreeMap::new();
        for ((j, cand, item, _), verdicts) in &folded {
            if j != judge {
                continue;
            }
            let Some(it) = items.get(item) else { continue };
            let side = verdicts.get("overall").map(String::as_str).unwrap_or("tie");
            by_group.entry((format!("{} ({})", it.project, it.parser), cand.clone())).or_default().add(side);
        }
        let groups: Vec<String> = {
            let mut g: Vec<String> = by_group.keys().map(|k| k.0.clone()).collect();
            g.dedup();
            g
        };
        writeln!(md, "| 프로젝트 (파서) | {} |", candidates.join(" | "))?;
        writeln!(md, "|---|{}", "---|".repeat(candidates.len()))?;
        for g in groups {
            let cells: Vec<String> = candidates
                .iter()
                .map(|c| {
                    by_group
                        .get(&(g.clone(), c.clone()))
                        .map(|t| format!("{}승 {}패 {}무", t.wins, t.losses, t.ties))
                        .unwrap_or_else(|| "—".to_string())
                })
                .collect();
            writeln!(md, "| {g} | {} |", cells.join(" | "))?;
        }
        writeln!(md)?;
    }

    // 5. Decision rule (design §7).
    writeln!(md, "## 5. 판정 기준 적용\n")?;
    let enough_humans = humans.len() >= MIN_HUMAN;
    let inter_ok = inter.1 > 0 && inter.0 as f64 / inter.1 as f64 >= 0.7;
    let trusted: Vec<&String> = if enough_humans {
        judges
            .iter()
            .filter(|j| agreement.get(*j).is_some_and(|(hit, n)| *n > 0 && *hit as f64 / *n as f64 >= 0.7))
            .collect()
    } else if inter_ok {
        writeln!(md, "사람 판정이 {MIN_HUMAN}쌍 미만이라, 채점자 간 일치율(70% 이상)을 신뢰도의 근거로 씁니다. 이 결론은 **LLM 채점자 근거만** 있습니다. 두 채점자가 공통으로 가진 취향은 이 방법으로 걸러지지 않습니다.\n")?;
        judges.iter().collect()
    } else {
        writeln!(md, "사람 판정이 {MIN_HUMAN}쌍 미만이고 채점자 간 일치율도 70%에 못 미쳐, 믿을 만한 채점자가 없습니다.\n")?;
        Vec::new()
    };
    let (base_final, base_first) = gate_rates.get(&baseline).copied().unwrap_or((0.0, 0.0));
    for cand in &candidates {
        let (f, i) = gate_rates.get(cand).copied().unwrap_or((0.0, 0.0));
        let gate_ok = f >= base_final && i >= base_first;
        // A judge from the candidate's own company never judged it.
        let trusted: Vec<&String> = trusted
            .iter()
            .copied()
            .filter(|j| tallies.contains_key(&((*j).clone(), cand.clone(), "overall".to_string())))
            .collect();
        let judge_ok: Vec<String> = trusted
            .iter()
            .map(|j| {
                let t = tallies.get(&((*j).clone(), cand.clone(), "overall".to_string())).copied().unwrap_or_default();
                let ok = t.wilson().is_some_and(|(lo, _)| lo > 0.5);
                format!("{j}: {}", if ok { "통과" } else { "미달" })
            })
            .collect();
        let judges_pass = !trusted.is_empty()
            && trusted.iter().all(|j| {
                tallies
                    .get(&((*j).clone(), cand.clone(), "overall".to_string()))
                    .and_then(Tally::wilson)
                    .is_some_and(|(lo, _)| lo > 0.5)
            });
        let human = tallies.get(&("사람".to_string(), cand.clone(), "overall".to_string())).and_then(Tally::rate);
        let human_ok = human.is_some_and(|r| r > 0.5);
        let verdict = if gate_ok && judges_pass && enough_humans && human_ok {
            "기준선을 대체할 근거가 있음"
        } else if !enough_humans && gate_ok && judges_pass {
            "LLM 채점자 기준으로 우세 (사람 검증 없음)"
        } else {
            "기준선을 대체할 근거가 부족함"
        };
        writeln!(md, "- **{cand}**: {verdict}")?;
        writeln!(
            md,
            "  - 관문: 최종 {:.1}% (기준선 {:.1}%), 첫 시도 {:.1}% (기준선 {:.1}%) → {}",
            f * 100.0,
            base_final * 100.0,
            i * 100.0,
            base_first * 100.0,
            if gate_ok { "통과" } else { "미달" }
        )?;
        writeln!(md, "  - 채점자 종합 승률 하한 > 50%: {}", if judge_ok.is_empty() { "믿을 만한 채점자 없음".to_string() } else { judge_ok.join(", ") })?;
        writeln!(
            md,
            "  - 사람 종합 승률 > 50%: {}",
            match human {
                Some(r) => format!("{:.0}% → {}", r * 100.0, if human_ok { "통과" } else { "미달" }),
                None => "판정 없음".to_string(),
            }
        )?;
    }
    if let Some(parent) = opts.out.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&opts.out, md)?;
    eprintln!("wrote {}", opts.out.display());
    Ok(())
}
