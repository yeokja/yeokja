//! `yeokja-eval extract`: freeze production requests into the eval set.

use crate::item::{Block, Item, Segment, write_jsonl};
use anyhow::{Context, Result, anyhow, bail};
use serde::Deserialize;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;
use yeokja_core::hash::content_hash;
use yeokja_core::project::ProjectContext;
use yeokja_core::state::StateFile;
use yeokja_translate::orchestrator::{ParserFactory, PlannedRequest, collect_files, plan_requests};

#[derive(Deserialize)]
struct Sources {
    projects: BTreeMap<String, ProjectSource>,
    hard: Hard,
}

#[derive(Deserialize)]
struct ProjectSource {
    bucket: String,
    license: String,
    attribution: String,
    root: String,
    skip_containing: Option<String>,
}

#[derive(Deserialize)]
struct Hard {
    commits: Vec<String>,
}

#[derive(Deserialize)]
struct Probes {
    block: Vec<Probe>,
}

#[derive(Deserialize)]
struct Probe {
    file: String,
    block: String,
    probes: Vec<String>,
}

pub struct Options {
    pub repo: PathBuf,
    pub out: PathBuf,
    pub seed: u64,
    pub target: usize,
    pub per_project_min: usize,
    pub per_project_max: usize,
}

/// Requests of this many numbered segments or more are preferred: they are
/// what a production run mostly sends.
const FULL_REQUEST: usize = 8;

fn git(repo: &Path, args: &[&str]) -> Result<String> {
    let out = Command::new("git").arg("-C").arg(repo).args(args).output()?;
    if !out.status.success() {
        bail!("git {}: {}", args.join(" "), String::from_utf8_lossy(&out.stderr));
    }
    Ok(String::from_utf8(out.stdout)?)
}

fn key(seed: u64, id: &str) -> u64 {
    content_hash(&format!("{seed}:{id}"))
}

/// Text a reader would read in a segment, without code, URLs and markup.
fn prose_letters(source: &str) -> usize {
    let mut in_code = false;
    let mut in_url = false;
    let mut count = 0;
    let mut prev = ' ';
    for c in source.chars() {
        match c {
            '`' => in_code = !in_code,
            '(' if prev == ']' => in_url = true,
            ')' if in_url => in_url = false,
            _ if !in_code && !in_url && c.is_alphabetic() => count += 1,
            _ => {}
        }
        prev = c;
    }
    count
}

fn trivial(segment: &str) -> bool {
    segment.chars().count() < 20 || prose_letters(segment) < 12
}

fn block_id(segment_id: &str) -> String {
    segment_id.rsplit_once("/seg:").map_or(segment_id, |(block, _)| block).to_string()
}

fn strata(raw: &str, segments: &[Segment], tags: usize) -> Vec<String> {
    let mut out = Vec::new();
    let trimmed = raw.trim_start();
    if segments.len() >= 4 {
        out.push("long".to_string());
    }
    if tags >= 3 || raw.matches('`').count() + raw.matches("](").count() + raw.matches("**").count() >= 4 {
        out.push("inline-markup".to_string());
    }
    if raw.contains('$') || raw.contains("\\(") || raw.contains(":math:") || raw.contains("\\begin{equation") {
        out.push("math".to_string());
    }
    if trimmed.starts_with("- ") || trimmed.starts_with("* ") || trimmed.starts_with("\\item") {
        out.push("list".to_string());
    }
    if trimmed.starts_with('|') {
        out.push("table".to_string());
    }
    if trimmed.starts_with('#') || trimmed.starts_with("\\section") || trimmed.starts_with("\\chapter") {
        out.push("heading".to_string());
    }
    if segments.iter().all(|s| trivial(&s.source)) {
        out.push("trivial".to_string());
    }
    if out.is_empty() || (out.len() == 1 && out[0] == "long") {
        out.push("prose".to_string());
    }
    out
}

struct Context_<'a> {
    repo: &'a Path,
    yeokja_commit: String,
}

struct ProjectRun {
    name: String,
    dir: PathBuf,
    ctx: ProjectContext,
    factory: ParserFactory,
    upstream: String,
    glossary_file: String,
}

impl ProjectRun {
    fn load(repo: &Path, dir: &Path, name: &str) -> Result<Self> {
        std::env::set_current_dir(dir).with_context(|| format!("enter {}", dir.display()))?;
        let ctx = ProjectContext::load().with_context(|| format!("load {name}"))?;
        let upstream = upstream_of(repo, dir);
        Ok(Self {
            name: name.to_string(),
            dir: dir.to_path_buf(),
            ctx,
            factory: Arc::new(yeokja_parsers::select_parser),
            upstream,
            glossary_file: format!("glossaries/{name}.toml"),
        })
    }

    fn enter(&self) -> Result<()> {
        std::env::set_current_dir(&self.dir)?;
        Ok(())
    }

    fn plan(&self, file: &Path) -> Result<Vec<PlannedRequest>> {
        self.enter()?;
        plan_requests(file, &self.ctx.config, &self.ctx.glossary, &self.factory)
            .map_err(|e| anyhow!("{}/{}: {e}", self.name, file.display()))
    }

    fn parser_of(&self, file: &Path) -> String {
        self.ctx.config.source_for(file).map(|s| s.parser.clone()).unwrap_or_default()
    }
}

/// `<url>@<commit>` of the project's pinned upstream, or of this repository
/// when the project keeps its own source.
fn upstream_of(repo: &Path, dir: &Path) -> String {
    let rel = dir.strip_prefix(repo).unwrap_or(dir).join("upstream");
    let rel = rel.to_string_lossy().to_string();
    let url = git(repo, &["config", "-f", ".gitmodules", &format!("submodule.{rel}.url")])
        .map(|s| s.trim().to_string())
        .unwrap_or_default();
    let commit = git(repo, &["ls-tree", "HEAD", &rel])
        .ok()
        .and_then(|line| line.split_whitespace().nth(2).map(str::to_string))
        .unwrap_or_default();
    if url.is_empty() { String::new() } else { format!("{url}@{commit}") }
}

fn item_from(
    run: &ProjectRun,
    source: &ProjectSource,
    cx: &Context_,
    file: &Path,
    number: usize,
    planned: PlannedRequest,
    kind: &str,
) -> Item {
    let rel = file.strip_prefix(".").unwrap_or(file).to_string_lossy().to_string();
    let blocks = planned
        .blocks
        .iter()
        .map(|b| {
            let segments: Vec<Segment> = b
                .segments
                .iter()
                .map(|s| Segment {
                    id: s.id.clone(),
                    index: s.index,
                    source: s.source.clone(),
                    reference: None,
                    known_bad: None,
                })
                .collect();
            let tags: usize = planned
                .inline
                .as_ref()
                .map(|batch| segments.iter().filter_map(|s| batch.tagged.get(&s.index)).map(|t| t.tags.len()).sum())
                .unwrap_or(0);
            Block {
                id: block_id(&b.segments[0].id),
                strata: strata(&b.raw, &segments, tags),
                raw: b.raw.clone(),
                probes: Vec::new(),
                segments,
            }
        })
        .collect();
    Item {
        id: format!("{}/{}#{}", run.name, rel, number),
        kind: kind.to_string(),
        project: run.name.clone(),
        source_path: rel,
        parser: run.parser_of(file),
        upstream: run.upstream.clone(),
        license: source.license.clone(),
        attribution: source.attribution.clone(),
        yeokja_commit: cx.yeokja_commit.clone(),
        request: planned.request,
        inline: planned.inline,
        blocks,
        glossary: run.glossary_file.clone(),
    }
}

fn nontrivial(planned: &PlannedRequest) -> bool {
    planned.blocks.iter().flat_map(|b| &b.segments).any(|s| !trivial(&s.source))
}

/// The request of a file this seed picks: a full-size one when the file has
/// any, else any request with prose.
fn pick_request(seed: u64, id: &str, planned: &[PlannedRequest]) -> Option<usize> {
    let mut candidates: Vec<usize> = (0..planned.len()).filter(|&i| nontrivial(&planned[i])).collect();
    let full: Vec<usize> =
        candidates.iter().copied().filter(|&i| planned[i].request.segments.len() >= FULL_REQUEST).collect();
    if !full.is_empty() {
        candidates = full;
    }
    candidates.into_iter().min_by_key(|i| key(seed, &format!("{id}#{}", i + 1)))
}

pub fn run(opts: &Options) -> Result<()> {
    let repo = opts.repo.canonicalize()?;
    let out = if opts.out.is_absolute() { opts.out.clone() } else { repo.join(&opts.out) };
    let sources: Sources = toml::from_str(&std::fs::read_to_string(out.join("sources.toml"))?)?;
    let dirty = !git(&repo, &["status", "--porcelain", "--", "crates"])?.trim().is_empty();
    let cx = Context_ {
        repo: &repo,
        yeokja_commit: format!("{}{}", git(&repo, &["rev-parse", "HEAD"])?.trim(), if dirty { "-dirty" } else { "" }),
    };
    let tracked: HashSet<String> = git(&repo, &["ls-files", "projects"])?.lines().map(str::to_string).collect();

    let mut items: Vec<(String, Item)> = Vec::new();
    let mut glossaries: BTreeMap<String, PathBuf> = BTreeMap::new();
    let mut ratios: Vec<(String, f64)> = Vec::new();

    // Real items: files per project in seed order, one request per file.
    let mut pools: Vec<(String, Vec<Item>)> = Vec::new();
    for (name, source) in &sources.projects {
        let dir = repo.join("projects").join(name);
        let run = ProjectRun::load(&repo, &dir, name)?;
        glossaries.insert(name.clone(), dir.join(&run.ctx.config.project.glossary));
        let state_dir = run.ctx.config.state_dir().map(Path::to_path_buf);
        let mut files: Vec<PathBuf> = collect_files(Path::new(&source.root), &run.ctx.config)?
            .into_iter()
            .filter(|f| {
                let state = StateFile::state_file_path(f, state_dir.as_deref());
                let state = state.strip_prefix(".").unwrap_or(&state);
                tracked.contains(&format!("projects/{name}/{}", state.display()))
            })
            .collect();
        ratios.extend(length_ratios(&run, &files, state_dir.as_deref()));
        files.sort_by_key(|f| key(opts.seed, &format!("{name}/{}", f.display())));
        let mut pool = Vec::new();
        for file in files {
            if pool.len() >= opts.per_project_max {
                break;
            }
            if let Some(needle) = &source.skip_containing
                && std::fs::read_to_string(dir.join(&file)).is_ok_and(|text| text.contains(needle.as_str()))
            {
                continue;
            }
            let planned = run.plan(&file)?;
            let rel = file.strip_prefix(".").unwrap_or(&file).display().to_string();
            let Some(i) = pick_request(opts.seed, &format!("{name}/{rel}"), &planned) else { continue };
            let planned = planned.into_iter().nth(i).expect("picked index");
            pool.push(item_from(&run, source, &cx, &file, i + 1, planned, "real"));
        }
        eprintln!("{name}: {} candidate requests", pool.len());
        pools.push((source.bucket.clone(), pool));
    }
    // Round-robin: everyone gets `per_project_min`, then fill to `target`.
    let mut taken = vec![0usize; pools.len()];
    let mut total = 0;
    for round in 0..opts.per_project_max {
        for (p, (bucket, pool)) in pools.iter().enumerate() {
            if round >= opts.per_project_min && total >= opts.target {
                break;
            }
            if let Some(item) = pool.get(round) {
                items.push((bucket.clone(), item.clone()));
                taken[p] += 1;
                total += 1;
            }
        }
    }

    // Hard items: segments a person corrected, with the request around them.
    let hard = hard_items(&cx, &sources, &repo)?;
    let hard_ids: HashSet<String> = hard.iter().map(|(_, item)| item.id.clone()).collect();
    items.retain(|(_, item)| !hard_ids.contains(&item.id));
    items.extend(hard);

    // Synthetic items: every request of the synthetic project.
    let synthetic_dir = out.join("synthetic");
    let synthetic = synthetic_items(&cx, &synthetic_dir)?;
    glossaries.insert("synthetic".to_string(), synthetic_dir.join("glossary.toml"));

    std::env::set_current_dir(&repo)?;
    let mut by_bucket: BTreeMap<String, Vec<Item>> = BTreeMap::new();
    for (bucket, item) in items {
        by_bucket.entry(bucket).or_default().push(item);
    }
    for (bucket, list) in &mut by_bucket {
        list.sort_by(|a, b| a.id.cmp(&b.id));
        write_jsonl(&out.join(format!("items.{bucket}.jsonl")), list)?;
    }
    write_jsonl(&out.join("synthetic.jsonl"), &synthetic)?;
    let used: HashSet<&str> = by_bucket
        .values()
        .flatten()
        .map(|i| i.project.as_str())
        .chain(std::iter::once("synthetic"))
        .collect();
    std::fs::create_dir_all(out.join("glossaries"))?;
    for (name, path) in &glossaries {
        if used.contains(name.as_str()) && path.exists() {
            std::fs::copy(path, out.join("glossaries").join(format!("{name}.toml")))?;
        }
    }
    write_truncation(&out, &ratios)?;
    crate::manifest::write(&out, &sources_summary(&sources), &by_bucket, &synthetic, &cx.yeokja_commit, opts)?;
    Ok(())
}

pub struct ProjectSummary {
    pub bucket: String,
    pub license: String,
    pub attribution: String,
}

fn sources_summary(sources: &Sources) -> BTreeMap<String, ProjectSummary> {
    sources
        .projects
        .iter()
        .map(|(name, s)| {
            (
                name.clone(),
                ProjectSummary { bucket: s.bucket.clone(), license: s.license.clone(), attribution: s.attribution.clone() },
            )
        })
        .collect()
}

/// Translation length over source length, per stored segment long enough
/// for the ratio to mean something.
fn length_ratios(run: &ProjectRun, files: &[PathBuf], state_dir: Option<&Path>) -> Vec<(String, f64)> {
    let _ = run.enter();
    let mut out = Vec::new();
    for file in files {
        let path = StateFile::state_file_path(file, state_dir);
        let Ok(state) = StateFile::load(&path) else { continue };
        for seg in state.segments {
            let Some(t) = seg.translation else { continue };
            let s = seg.source.chars().count();
            if s >= 40 {
                out.push((run.name.clone(), t.chars().count() as f64 / s as f64));
            }
        }
    }
    out
}

fn percentile(sorted: &[f64], p: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    let rank = ((p / 100.0) * (sorted.len() - 1) as f64).round() as usize;
    sorted[rank.min(sorted.len() - 1)]
}

fn write_truncation(out: &Path, ratios: &[(String, f64)]) -> Result<()> {
    let mut all: Vec<f64> = ratios.iter().map(|(_, r)| *r).collect();
    all.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let mut text = String::from(
        "# 저장된 번역의 (번역 글자 수 / 원문 글자 수) 분포. 원문 40자 이상 세그먼트만.\n\
         # gate의 잘림 검사는 min_ratio 미만을 실패로 봅니다(전체 분포의 0.01 백분위).\n\n",
    );
    text.push_str(&format!("segments = {}\n", all.len()));
    text.push_str(&format!("min_ratio = {:.4}\np0_5 = {:.4}\n", percentile(&all, 0.01), percentile(&all, 0.5)));
    text.push_str(&format!("p1 = {:.4}\np5 = {:.4}\np50 = {:.4}\n", percentile(&all, 1.0), percentile(&all, 5.0), percentile(&all, 50.0)));
    std::fs::write(out.join("truncation.toml"), text)?;
    Ok(())
}

fn hard_items(cx: &Context_, sources: &Sources, repo: &Path) -> Result<Vec<(String, Item)>> {
    // (project, state path) → segment id → (fixed, bad)
    let mut fixes: BTreeMap<(String, String), BTreeMap<String, (String, String)>> = BTreeMap::new();
    for commit in &sources.hard.commits {
        let changed = git(repo, &["diff-tree", "--no-commit-id", "-r", "--name-only", commit])?;
        for path in changed.lines().filter(|p| p.ends_with(".yeokja.json") && p.starts_with("projects/")) {
            let project = path.split('/').nth(1).unwrap_or_default().to_string();
            if !sources.projects.contains_key(&project) {
                continue;
            }
            let before: StateFile = serde_json::from_str(&git(repo, &["show", &format!("{commit}^:{path}")])?)?;
            let after: StateFile = serde_json::from_str(&git(repo, &["show", &format!("{commit}:{path}")])?)?;
            let old: HashMap<String, Option<String>> =
                before.segments.into_iter().map(|s| (s.id.0, s.translation)).collect();
            for seg in after.segments {
                let (Some(fixed), Some(Some(bad))) = (seg.translation, old.get(&seg.id.0)) else { continue };
                if &fixed != bad {
                    fixes.entry((project.clone(), path.to_string())).or_default().insert(seg.id.0, (fixed, bad.clone()));
                }
            }
        }
    }
    let mut items = Vec::new();
    for ((project, state_path), segs) in fixes {
        let source = &sources.projects[&project];
        let dir = repo.join("projects").join(&project);
        let run = ProjectRun::load(repo, &dir, &project)?;
        let state_dir = run.ctx.config.state_dir().map(|p| p.to_string_lossy().to_string()).unwrap_or_default();
        let prefix = format!("projects/{project}/{}/", state_dir.trim_start_matches("./").trim_end_matches('/'));
        let rel = state_path.strip_prefix(&prefix).unwrap_or(&state_path).trim_end_matches(".yeokja.json");
        let file = PathBuf::from(rel);
        let planned = run.plan(&file)?;
        for (i, request) in planned.into_iter().enumerate() {
            if !request.blocks.iter().flat_map(|b| &b.segments).any(|s| segs.contains_key(&s.id)) {
                continue;
            }
            let mut item = item_from(&run, source, cx, &file, i + 1, request, "hard");
            for seg in item.blocks.iter_mut().flat_map(|b| b.segments.iter_mut()) {
                if let Some((fixed, bad)) = segs.get(&seg.id) {
                    seg.reference = Some(fixed.clone());
                    seg.known_bad = Some(bad.clone());
                }
            }
            items.push((source.bucket.clone(), item));
        }
    }
    eprintln!("hard: {} requests", items.len());
    Ok(items)
}

fn synthetic_items(cx: &Context_, dir: &Path) -> Result<Vec<Item>> {
    let repo = cx.repo;
    let run = ProjectRun::load(repo, dir, "synthetic")?;
    let probes: Probes = toml::from_str(&std::fs::read_to_string(dir.join("probes.toml"))?)?;
    let by_block: HashMap<(String, String), Vec<String>> =
        probes.block.into_iter().map(|p| ((p.file, p.block), p.probes)).collect();
    let source = ProjectSource {
        bucket: "synthetic".to_string(),
        license: "MIT OR Apache-2.0".to_string(),
        attribution: "yeokja contributors (original writing for this eval)".to_string(),
        root: "src".to_string(),
        skip_containing: None,
    };
    let files = collect_files(Path::new(&source.root), &run.ctx.config)?;
    let mut items = Vec::new();
    for file in files {
        for (i, planned) in run.plan(&file)?.into_iter().enumerate() {
            let mut item = item_from(&run, &source, cx, &file, i + 1, planned, "synthetic");
            item.upstream = format!("https://github.com/yeokja/yeokja@{}", cx.yeokja_commit);
            for block in &mut item.blocks {
                if let Some(p) = by_block.get(&(item.source_path.clone(), block.id.clone())) {
                    block.probes = p.clone();
                }
            }
            items.push(item);
        }
    }
    items.sort_by(|a, b| a.id.cmp(&b.id));
    eprintln!("synthetic: {} requests", items.len());
    Ok(items)
}
