//! `MANIFEST.md`: what each file of the set holds and under which license.

use crate::extract::{Options, ProjectSummary};
use crate::item::Item;
use anyhow::Result;
use std::collections::BTreeMap;
use std::fmt::Write;
use std::path::Path;

const BUCKET_LICENSE: &[(&str, &str, &str)] = &[
    (
        "permissive",
        "항목마다 적힌 원문 라이선스(MIT, Apache-2.0, BSD-3-Clause, CC0-1.0, 퍼블릭 도메인)",
        "재배포 시 항목의 `attribution`과 `license`를 유지하세요.",
    ),
    ("cc-by-4.0", "CC BY 4.0 — https://creativecommons.org/licenses/by/4.0/", "저작자 표시 필요."),
    (
        "cc-by-sa-3.0",
        "CC BY-SA 3.0 — https://creativecommons.org/licenses/by-sa/3.0/",
        "저작자 표시, 동일조건변경허락.",
    ),
    (
        "cc-by-sa-4.0",
        "CC BY-SA 4.0 — https://creativecommons.org/licenses/by-sa/4.0/",
        "저작자 표시, 동일조건변경허락.",
    ),
    (
        "cc-by-nc-sa-4.0",
        "CC BY-NC-SA 4.0 — https://creativecommons.org/licenses/by-nc-sa/4.0/",
        "저작자 표시, 비영리, 동일조건변경허락. 상업적으로 이용할 수 없습니다.",
    ),
];

pub fn write(
    out: &Path,
    projects: &BTreeMap<String, ProjectSummary>,
    by_bucket: &BTreeMap<String, Vec<Item>>,
    synthetic: &[Item],
    yeokja_commit: &str,
    opts: &Options,
) -> Result<()> {
    let mut md = String::new();
    writeln!(md, "# eval 세트 v1 MANIFEST\n")?;
    writeln!(
        md,
        "이 파일은 `yeokja-eval extract`가 만듭니다. 손으로 고치지 마세요. 설계는\n\
         [`../README.md`](../README.md)에 있습니다.\n"
    )?;
    writeln!(md, "- yeokja 커밋: `{yeokja_commit}`")?;
    writeln!(md, "- 시드: `{}`, 목표 요청 수: {}, 프로젝트당 {}~{}요청", opts.seed, opts.target, opts.per_project_min, opts.per_project_max)?;
    writeln!(
        md,
        "- 다시 만들기: `cargo run -p yeokja-eval -- extract --seed {} --target {}`\n",
        opts.seed, opts.target
    )?;

    writeln!(md, "## 라이선스\n")?;
    writeln!(
        md,
        "이 디렉터리 전체에 적용되는 단일 라이선스는 없습니다. 파일마다 아래 라이선스를\n\
         따릅니다. 발췌는 원저작물의 일부이며, 각 항목의 `upstream`이 원문 위치(저장소와\n\
         고정 커밋)를, `attribution`이 저작자를 가리킵니다. 요청에 포함된 용어집과\n\
         `glossaries/`, `synthetic/`, `synthetic.jsonl`, `sources.toml`,\n\
         `truncation.toml`, 이 파일은 yeokja 도구와 같은 `MIT OR Apache-2.0`입니다.\n"
    )?;
    writeln!(md, "| 파일 | 라이선스 | 조건 | 요청 | 블록 |")?;
    writeln!(md, "|---|---|---|---|---|")?;
    for (bucket, items) in by_bucket {
        let (license, terms) = BUCKET_LICENSE
            .iter()
            .find(|(b, _, _)| b == bucket)
            .map(|(_, l, t)| (*l, *t))
            .unwrap_or(("항목별 `license` 참조", ""));
        let blocks: usize = items.iter().map(|i| i.blocks.len()).sum();
        writeln!(md, "| `items.{bucket}.jsonl` | {license} | {terms} | {} | {blocks} |", items.len())?;
    }
    let blocks: usize = synthetic.iter().map(|i| i.blocks.len()).sum();
    writeln!(md, "| `synthetic.jsonl` | MIT OR Apache-2.0 (직접 작성) | | {} | {blocks} |\n", synthetic.len())?;

    writeln!(md, "## 출처\n")?;
    writeln!(md, "| 프로젝트 | 파일 | 원문 라이선스 | 저작자 | 원문 | 요청(실제/어려운) |")?;
    writeln!(md, "|---|---|---|---|---|---|")?;
    for (name, p) in projects {
        let items: Vec<&Item> = by_bucket.values().flatten().filter(|i| &i.project == name).collect();
        if items.is_empty() {
            continue;
        }
        let real = items.iter().filter(|i| i.kind == "real").count();
        let hard = items.iter().filter(|i| i.kind == "hard").count();
        writeln!(
            md,
            "| {name} | `items.{}.jsonl` | {} | {} | {} | {real}/{hard} |",
            p.bucket, p.license, p.attribution, items[0].upstream
        )?;
    }

    writeln!(md, "\n## 구성\n")?;
    let mut parsers: BTreeMap<&str, (usize, usize)> = BTreeMap::new();
    let mut strata: BTreeMap<&str, usize> = BTreeMap::new();
    for item in by_bucket.values().flatten().chain(synthetic) {
        let e = parsers.entry(item.parser.as_str()).or_default();
        e.0 += 1;
        e.1 += item.blocks.len();
        for s in item.blocks.iter().flat_map(|b| &b.strata) {
            *strata.entry(s.as_str()).or_default() += 1;
        }
    }
    writeln!(md, "| 파서 | 요청 | 블록 |\n|---|---|---|")?;
    for (parser, (r, b)) in &parsers {
        writeln!(md, "| {parser} | {r} | {b} |")?;
    }
    writeln!(md, "\n| 블록 꼬리표 | 블록 |\n|---|---|")?;
    for (s, n) in &strata {
        writeln!(md, "| {s} | {n} |")?;
    }

    writeln!(md, "\n## 오염 고지\n")?;
    writeln!(
        md,
        "이 세트와 원문은 공개 저장소에 있으므로, 이후에 학습된 모델은 이 텍스트를 보았을 수\n\
         있습니다. 결과를 해석할 때 고려하고, 새 버전은 합성 항목의 비중을 늘립니다."
    )?;
    std::fs::write(out.join("MANIFEST.md"), md)?;
    Ok(())
}
