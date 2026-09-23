//! The frozen eval item: one production request and what a judge reads of it.

use serde::{Deserialize, Serialize};
use std::io::{BufRead, Write};
use std::path::Path;
use yeokja_translate::pipeline::InlineBatch;
use yeokja_translate::provider::TranslateRequest;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Item {
    /// `<project>/<source path>#<request number in the file>`.
    pub id: String,
    /// `real`, `hard` or `synthetic`.
    pub kind: String,
    pub project: String,
    pub source_path: String,
    pub parser: String,
    /// Where the text comes from, pinned: `<url>@<commit>`.
    pub upstream: String,
    pub license: String,
    pub attribution: String,
    pub yeokja_commit: String,
    /// The request exactly as yeokja sends it.
    pub request: TranslateRequest,
    pub inline: Option<InlineBatch>,
    pub blocks: Vec<Block>,
    /// The project's glossary, copied under `glossaries/`.
    pub glossary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Block {
    /// `section:N/block:N`, unique within the item.
    pub id: String,
    pub raw: String,
    /// Tags for sampling and breakdowns (`math`, `inline-markup`, `long`, …).
    pub strata: Vec<String>,
    /// What a synthetic block is written to test.
    pub probes: Vec<String>,
    pub segments: Vec<Segment>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Segment {
    pub id: String,
    /// The segment's number in `request.segments`.
    pub index: usize,
    /// The source as a reader sees it (not tag text).
    pub source: String,
    /// A translation a person corrected (hard items only).
    pub reference: Option<String>,
    /// The translation that needed correcting (hard items only).
    pub known_bad: Option<String>,
}

/// Serialize with sorted keys, so the same value is always the same bytes.
pub fn canonical<T: Serialize>(value: &T) -> anyhow::Result<String> {
    Ok(serde_json::to_string(&serde_json::to_value(value)?)?)
}

pub fn write_jsonl<T: Serialize>(path: &Path, rows: &[T]) -> anyhow::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut out = std::io::BufWriter::new(std::fs::File::create(path)?);
    for row in rows {
        writeln!(out, "{}", canonical(row)?)?;
    }
    out.flush()?;
    Ok(())
}

pub fn read_jsonl<T: for<'de> Deserialize<'de>>(path: &Path) -> anyhow::Result<Vec<T>> {
    let file = std::fs::File::open(path).map_err(|e| anyhow::anyhow!("{}: {e}", path.display()))?;
    let mut rows = Vec::new();
    for (n, line) in std::io::BufReader::new(file).lines().enumerate() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        rows.push(
            serde_json::from_str(&line).map_err(|e| anyhow::anyhow!("{}:{}: {e}", path.display(), n + 1))?,
        );
    }
    Ok(rows)
}

/// Every item file of a set directory (`items.*.jsonl`, `synthetic.jsonl`).
pub fn item_files(set: &Path) -> anyhow::Result<Vec<std::path::PathBuf>> {
    let mut files: Vec<_> = std::fs::read_dir(set)?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| {
            let name = p.file_name().and_then(|n| n.to_str()).unwrap_or_default();
            (name.starts_with("items.") || name == "synthetic.jsonl") && name.ends_with(".jsonl")
        })
        .collect();
    files.sort();
    Ok(files)
}

/// The bucket an item file holds (`items.cc-by-4.0.jsonl` → `cc-by-4.0`).
pub fn bucket_of(file: &Path) -> String {
    let name = file.file_name().and_then(|n| n.to_str()).unwrap_or_default();
    name.trim_start_matches("items.").trim_end_matches(".jsonl").to_string()
}
