//! Corpus round trip for inline tag transport: every segment of every
//! `markdown`, `myst` and `mdx` source is tagged, read back untranslated and
//! rendered, and must render to the same HTML as its source (plain strings,
//! to the same text).
//!
//! `YEOKJA_PROJECTS=projects cargo test -p yeokja-translate --test inline_roundtrip -- --ignored --nocapture`

use pulldown_cmark::{BrokenLink, CowStr, Options, Parser};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use yeokja_core::config::ProjectConfig;
use yeokja_core::config::INLINE_TAG_PARSERS;
use yeokja_core::model::{BlockRole, BlockType};
use yeokja_translate::inline::markdown::{self, Dialect, DocContext, Position};

fn html(text: &str, labels: &[String], dialect: Dialect) -> String {
    let callback = |link: BrokenLink| {
        let label = link.reference.split_whitespace().collect::<Vec<_>>().join(" ").to_lowercase();
        labels.contains(&label).then(|| (CowStr::from(""), CowStr::from("")))
    };
    let mut options = Options::empty();
    if dialect != Dialect::Myst {
        options.insert(Options::ENABLE_STRIKETHROUGH);
    }
    options.insert(Options::ENABLE_TABLES);
    // Read as inline text, the way the segment sits in its document.
    let guarded = format!("x {text}");
    let mut out = String::new();
    pulldown_cmark::html::push_html(&mut out, Parser::new_with_broken_link_callback(&guarded, options, Some(callback)));
    out
}

fn markdown_files(project: &Path, config: &ProjectConfig) -> Vec<(PathBuf, Dialect)> {
    let mut files = Vec::new();
    for source in config.sources.iter().filter(|s| INLINE_TAG_PARSERS.contains(&s.parser.as_str())) {
        let pattern = project.join(&source.path).join(&source.pattern);
        for entry in glob::glob(&pattern.to_string_lossy()).into_iter().flatten().flatten() {
            if let Ok(rel) = entry.strip_prefix(project)
                && let Some(chosen) = config.source_for(rel).filter(|s| INLINE_TAG_PARSERS.contains(&s.parser.as_str()))
            {
                files.push((rel.to_path_buf(), Dialect::for_parser(&chosen.parser)));
            }
        }
    }
    files
}

#[test]
#[ignore]
fn stored_markdown_sources_survive_the_round_trip() {
    let root = PathBuf::from(std::env::var("YEOKJA_PROJECTS").expect("set YEOKJA_PROJECTS to the projects directory"));
    // Per dialect: segments, untaggable, taggable rendering the same.
    let mut counts: HashMap<String, (usize, usize, usize)> = HashMap::new();
    let mut failures = Vec::new();
    let mut untaggable_examples = Vec::new();
    // Pairs whose bytes changed, for checking with each dialect's own renderer.
    let mut dump = std::env::var("YEOKJA_ROUNDTRIP_DUMP").ok().map(|path| std::fs::File::create(path).unwrap());
    let mut entries: Vec<_> = std::fs::read_dir(&root).unwrap().flatten().map(|e| e.path()).collect();
    entries.sort();
    for project in entries {
        let Ok(config) = ProjectConfig::load(&project.join("yeokja.toml")) else { continue };
        for (rel, dialect) in markdown_files(&project, &config) {
            let Ok(source) = std::fs::read_to_string(project.join(&rel)) else { continue };
            let doc = yeokja_parsers::select_parser(&rel, &config).parse(&source);
            let ctx = DocContext::new(&source, dialect);
            let labels: Vec<String> = {
                let parser = Parser::new_ext(&source, Options::empty());
                parser.reference_definitions().iter().map(|(l, _)| l.split_whitespace().collect::<Vec<_>>().join(" ").to_lowercase()).collect()
            };
            let positions: HashMap<String, Position> = doc
                .sections
                .iter()
                .flat_map(|s| &s.blocks)
                .flat_map(|b| {
                    let position = if b.role == BlockRole::Literal {
                        Position::Plain
                    } else if b.block_type == BlockType::Table {
                        Position::TableCell
                    } else {
                        Position::Inline
                    };
                    b.segments.iter().map(move |seg| (seg.id.0.clone(), position))
                })
                .collect();
            let count = counts.entry(format!("{dialect:?}")).or_default();
            for segment in doc.all_segments() {
                count.0 += 1;
                let mut counter = 0;
                let position = positions.get(&segment.id.0).copied().unwrap_or_default();
                let tagged = match markdown::tagify(&segment.source, &ctx, position, &mut counter) {
                    Ok(t) => t,
                    Err(why) => {
                        count.1 += 1;
                        if untaggable_examples.iter().filter(|e: &&String| e.starts_with(&format!("{dialect:?} "))).count() < 15 {
                            untaggable_examples.push(format!("{:?} {} ({})", dialect, segment.source, why.0));
                        }
                        continue;
                    }
                };
                let tree = markdown::read(&tagged.text, &tagged).unwrap_or_else(|p| panic!("{}: {p:?}", segment.source));
                let rendered = markdown::render(&tree, &tagged, &ctx);
                let same = if position == Position::Plain {
                    rendered.markdown == segment.source
                } else {
                    html(&rendered.markdown, &labels, dialect) == html(&segment.source, &labels, dialect)
                };
                if let Some(dump) = &mut dump
                    && rendered.markdown != segment.source
                    && position != Position::Plain
                {
                    use std::io::Write;
                    let line = serde_json::json!({
                        "dialect": format!("{dialect:?}"),
                        "id": format!("{}|{}", rel.display(), segment.id.0),
                        "source": segment.source,
                        "output": rendered.markdown,
                    });
                    writeln!(dump, "{line}").unwrap();
                }
                if same {
                    count.2 += 1;
                } else if failures.len() < 40 {
                    failures.push(format!("{}\n  src: {}\n  out: {}", rel.display(), segment.source, rendered.markdown));
                }
            }
        }
    }
    let mut dialects: Vec<_> = counts.iter().collect();
    dialects.sort();
    for (dialect, (total, untaggable, same)) in &dialects {
        println!("{dialect}: segments {total}, untaggable {untaggable}, same html {same}/{}", total - untaggable);
    }
    for u in &untaggable_examples {
        println!("untaggable: {u}");
    }
    for f in &failures {
        println!("{f}");
    }
    for (dialect, (total, untaggable, same)) in dialects {
        let taggable = total - untaggable;
        assert!(*same as f64 >= taggable as f64 * 0.9999, "{dialect}: {same}/{taggable}");
    }
}
