//! Corpus round trip for inline tag transport: every segment of every
//! `markdown` source is tagged, read back untranslated and rendered, and must
//! render to the same HTML as its source.
//!
//! `YEOKJA_PROJECTS=projects cargo test -p yeokja-translate --test inline_roundtrip -- --ignored --nocapture`

use pulldown_cmark::{BrokenLink, CowStr, Options, Parser};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use yeokja_core::config::ProjectConfig;
use yeokja_core::model::BlockType;
use yeokja_translate::inline::markdown::{self, DocContext, Position};

fn html(text: &str, labels: &[String]) -> String {
    let callback = |link: BrokenLink| {
        let label = link.reference.split_whitespace().collect::<Vec<_>>().join(" ").to_lowercase();
        labels.contains(&label).then(|| (CowStr::from(""), CowStr::from("")))
    };
    let mut options = Options::empty();
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TABLES);
    // Read as inline text, the way the segment sits in its document.
    let guarded = format!("x {text}");
    let mut out = String::new();
    pulldown_cmark::html::push_html(&mut out, Parser::new_with_broken_link_callback(&guarded, options, Some(callback)));
    out
}

fn markdown_files(project: &Path, config: &ProjectConfig) -> Vec<PathBuf> {
    let mut files = Vec::new();
    for source in config.sources.iter().filter(|s| s.parser == "markdown") {
        let pattern = project.join(&source.path).join(&source.pattern);
        for entry in glob::glob(&pattern.to_string_lossy()).into_iter().flatten().flatten() {
            if let Ok(rel) = entry.strip_prefix(project) {
                if config.source_for(rel).is_some_and(|s| s.parser == "markdown") {
                    files.push(rel.to_path_buf());
                }
            }
        }
    }
    files
}

#[test]
#[ignore]
fn stored_markdown_sources_survive_the_round_trip() {
    let root = PathBuf::from(std::env::var("YEOKJA_PROJECTS").expect("set YEOKJA_PROJECTS to the projects directory"));
    let (mut total, mut untaggable, mut same) = (0usize, 0usize, 0usize);
    let mut failures = Vec::new();
    let mut untaggable_examples = Vec::new();
    let mut entries: Vec<_> = std::fs::read_dir(&root).unwrap().flatten().map(|e| e.path()).collect();
    entries.sort();
    for project in entries {
        let Ok(config) = ProjectConfig::load(&project.join("yeokja.toml")) else { continue };
        for rel in markdown_files(&project, &config) {
            let Ok(source) = std::fs::read_to_string(project.join(&rel)) else { continue };
            let doc = yeokja_parsers::select_parser(&rel, &config).parse(&source);
            let ctx = DocContext::from_markdown(&source);
            let labels: Vec<String> = {
                let parser = Parser::new_ext(&source, Options::empty());
                parser.reference_definitions().iter().map(|(l, _)| l.split_whitespace().collect::<Vec<_>>().join(" ").to_lowercase()).collect()
            };
            let positions: HashMap<String, Position> = doc
                .sections
                .iter()
                .flat_map(|s| &s.blocks)
                .flat_map(|b| {
                    let position = if b.block_type == BlockType::Table { Position::TableCell } else { Position::Inline };
                    b.segments.iter().map(move |seg| (seg.id.0.clone(), position))
                })
                .collect();
            for segment in doc.all_segments() {
                total += 1;
                let mut counter = 0;
                let position = positions.get(&segment.id.0).copied().unwrap_or_default();
                let tagged = match markdown::tagify(&segment.source, &ctx, position, &mut counter) {
                    Ok(t) => t,
                    Err(why) => {
                        untaggable += 1;
                        if untaggable_examples.len() < 25 {
                            untaggable_examples.push(format!("{} ({})", segment.source, why.0));
                        }
                        continue;
                    }
                };
                let tree = markdown::read(&tagged.text, &tagged).unwrap_or_else(|p| panic!("{}: {p:?}", segment.source));
                let rendered = markdown::render(&tree, &tagged, &ctx);
                if html(&rendered.markdown, &labels) == html(&segment.source, &labels) {
                    same += 1;
                } else if failures.len() < 40 {
                    failures.push(format!("{}\n  src: {}\n  out: {}", rel.display(), segment.source, rendered.markdown));
                }
            }
        }
    }
    let taggable = total - untaggable;
    println!("segments {total}, untaggable {untaggable}, same html {same}/{taggable}");
    for u in &untaggable_examples {
        println!("untaggable: {u}");
    }
    for f in &failures {
        println!("{f}");
    }
    assert!(same as f64 >= taggable as f64 * 0.9999, "{same}/{taggable}");
}
