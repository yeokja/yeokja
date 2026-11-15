//! Reconstruct every Markdown file under <src> twice — unchanged and with
//! each segment prefixed by `가 ` — for the corpus structure check in
//! projects/cp-algorithms/scripts/check_parser_corpus.py.
//!
//! `roundtrip <src> <out> [--no-math] [--definition-lists]`: the flags read
//! the corpus as a source with `math = false` / `definition_lists = true`.
use std::fs;
use std::path::{Path, PathBuf};
use yeokja_core::parser::{DocumentParser, TranslationMap};
use yeokja_parser_mkdocs::MkdocsParser;

fn markdown_files(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            markdown_files(&path, out);
        } else if path.extension().is_some_and(|e| e == "md") {
            out.push(path);
        }
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let (src, out) = (Path::new(&args[1]), Path::new(&args[2]));
    let flag = |name: &str| args[3..].iter().any(|a| a == name);
    let parser = MkdocsParser::configured(!flag("--no-math"), flag("--definition-lists"));
    let mut files = Vec::new();
    markdown_files(src, &mut files);
    for file in files {
        let Ok(text) = fs::read_to_string(&file) else { continue };
        let doc = parser.parse(&text);
        for (mode, prefix) in [("identity", ""), ("fake", "가 ")] {
            let mut map = TranslationMap::new();
            for seg in doc.translatable_segments() {
                map.insert(seg.id.clone(), format!("{prefix}{}", seg.source));
            }
            let target = out.join(mode).join(file.strip_prefix(src).unwrap());
            fs::create_dir_all(target.parent().unwrap()).unwrap();
            fs::write(target, parser.reconstruct(&doc, &map)).unwrap();
        }
    }
}
