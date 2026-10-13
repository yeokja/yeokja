//! `yeokja-eval surround`: derive a set whose requests carry what a request
//! now carries around it (headings, the blocks before and after) and only
//! the glossary entries its sentences use.
//!
//! Each item's file is planned again with `plan_requests` (no saved state,
//! so neighbours show as source only) and the request with the same
//! segments lends its surroundings. Everything else stays as frozen, so a
//! run on the derived set differs from a run on the original only by this.

use crate::extract::ProjectRun;
use crate::item::{Item, item_files, read_jsonl, write_jsonl};
use anyhow::{Context, Result};
use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use yeokja_core::glossary::Glossary;
use yeokja_translate::orchestrator::PlannedRequest;

pub struct Options {
    pub repo: PathBuf,
    pub set: PathBuf,
    pub out: PathBuf,
}

/// The planned request with the item's segments: same IDs and sources, or
/// failing that (the upstream moved) the same sources.
fn matching<'a>(item: &Item, planned: &'a [PlannedRequest]) -> Option<&'a PlannedRequest> {
    let want: Vec<(&str, &str)> =
        item.blocks.iter().flat_map(|b| &b.segments).map(|s| (s.id.as_str(), s.source.as_str())).collect();
    let of = |p: &'a PlannedRequest| -> Vec<(&'a str, &'a str)> {
        p.blocks.iter().flat_map(|b| &b.segments).map(|s| (s.id.as_str(), s.source.as_str())).collect()
    };
    planned.iter().find(|p| of(p) == want).or_else(|| {
        let texts: Vec<&str> = want.iter().map(|(_, s)| *s).collect();
        planned.iter().find(|p| of(p).into_iter().map(|(_, s)| s).collect::<Vec<_>>() == texts)
    })
}

pub fn run(opts: Options) -> Result<()> {
    let repo = std::fs::canonicalize(&opts.repo)?;
    let set = std::fs::canonicalize(&opts.set)?;
    std::fs::create_dir_all(&opts.out)?;
    let out = std::fs::canonicalize(&opts.out)?;
    let cwd = std::env::current_dir()?;

    let mut runs: HashMap<String, ProjectRun> = HashMap::new();
    let mut glossaries: HashMap<String, Glossary> = HashMap::new();
    let mut unmatched = Vec::new();
    let mut counts: BTreeMap<&str, usize> = BTreeMap::new();
    for file in item_files(&set)? {
        let mut items: Vec<Item> = read_jsonl(&file)?;
        for item in &mut items {
            if !runs.contains_key(&item.project) {
                let dir = if item.project == "synthetic" {
                    set.join("synthetic")
                } else {
                    repo.join("projects").join(&item.project)
                };
                runs.insert(item.project.clone(), ProjectRun::load(&repo, &dir, &item.project)?);
            }
            let planned = runs[&item.project].plan(Path::new(&item.source_path))?;
            match matching(item, &planned) {
                Some(p) => {
                    item.request.surroundings = p.request.surroundings.clone();
                    *counts.entry(if p.request.surroundings.is_empty() { "empty" } else { "surrounded" }).or_default() += 1;
                }
                None => unmatched.push(item.id.clone()),
            }
            if !glossaries.contains_key(&item.glossary) {
                let glossary = Glossary::load(&set.join(&item.glossary))
                    .map_err(|e| anyhow::anyhow!("{}: {e}", item.glossary))?;
                glossaries.insert(item.glossary.clone(), glossary);
            }
            let glossary = &glossaries[&item.glossary];
            let mut used = HashMap::new();
            for seg in item.blocks.iter().flat_map(|b| &b.segments) {
                used.extend(glossary.find_matching_terms(&seg.source));
            }
            item.request.glossary = used;
        }
        write_jsonl(&out.join(file.file_name().context("item file name")?), &items)?;
    }
    std::env::set_current_dir(cwd)?;

    for name in ["judge-sample.jsonl", "truncation.toml"] {
        std::fs::copy(set.join(name), out.join(name))?;
    }
    let glossary_dir = out.join("glossaries");
    std::fs::create_dir_all(&glossary_dir)?;
    for entry in std::fs::read_dir(set.join("glossaries"))? {
        let path = entry?.path();
        std::fs::copy(&path, glossary_dir.join(path.file_name().context("glossary name")?))?;
    }
    eprintln!("surroundings: {counts:?}; unmatched: {}", unmatched.len());
    for id in &unmatched {
        eprintln!("  unmatched: {id}");
    }
    Ok(())
}
