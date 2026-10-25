# R Markdown 파서 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** bookdown/R Markdown 교재를 Pandoc 의미론으로 읽는 `rmarkdown` 파서와 CSV 열을 번역하는 `csv` 파서를 추가한다.

**Architecture:** `crates/parser-rmarkdown`이 영역 스캔(`scan_regions`)으로 그림자를 만들어 `parse_with`에 주고, 블록을 수식을 가린 문장 단위로 다시 다듬는다(`refine`). `crates/parser-csv`는 필드를 표 셀 블록으로 내고 `[[tables]]` 규칙이 열을 고른다. 번역 쪽은 `Markup::RMarkdown`/`Markup::Plain`과 코덱 방언 `Dialect::RMarkdown`.

**Tech Stack:** Rust, pulldown-cmark(`parser-markdown-dialect`), `regex`.

**Spec:** `docs/superpowers/specs/2026-10-25-rmarkdown-parser-design.md`

## Global Constraints

- 다른 파서와 기존 프로젝트의 세그먼트 모양을 바꾸지 않는다(`mkdocs`의 `code_ranges`는 손대지 않는다).
- 픽스처는 LDAVer2(CC BY 4.0) 발췌만 넣는다: `Continuous.Rmd`, `Frequency2024.Rmd` 세 구간, `GlossFct.csv`.
- 커밋 메시지: `[parser-rmarkdown] <type>: <한국어 요약>`, `[parser-csv] …`, `[translate] …`, `[core] …`.

---

### Task 1: 영역 스캔과 그림자

**Files:** Create `crates/parser-rmarkdown/{Cargo.toml,src/lib.rs}`; Modify `Cargo.toml`(workspace members)

- [x] `scan_regions`: 펜스(산문 청크는 안쪽 계속 스캔), HTML 주석, 코드 스팬, Pandoc `$`/`$$`/`\(\)`/`\[\]`/`\begin…\end` 수식.
- [x] 그림자: 수식 `x`, 제목 속성(ATX·setext), front matter(`title`·`subtitle`·`description` Literal), fenced div, 산문 청크 펜스.
- [x] 청크 머리 `fig.cap`·`fig.alt`(산문 청크 `name`) Literal, `(ref:label)` 값 제외.

### Task 2: 블록 다듬기와 재구성

**Files:** Modify `crates/parser-rmarkdown/src/lib.rs`; Create `crates/parser-rmarkdown/src/tests.rs`, `crates/parser-rmarkdown/tests/fixtures/{Continuous.Rmd,Frequency-excerpt.Rmd}`

- [x] `(ref:label)`·`(PART)` 접두 제외, 수식을 가린 문장 분할, 강조 안 분할 합치기.
- [x] 산문 없는 문장 제외와 블록 나누기, 가장자리 인라인 R·디스플레이 수식 다듬기, 번호 다시 매기기.
- [x] 보존 토큰(`preserved_token_spans`, `preserved_tokens`), `@{key}` 수선(`brace_glued_citations`).
- [x] 인라인 코드 캡처(`RmarkdownParser::with_patterns`).
- [x] 재구성: Literal 번역의 R/YAML 따옴표 규칙.
- [x] 검증:

```sh
cargo test -p yeokja-parser-rmarkdown   # 17 passed
```

### Task 3: CSV 파서

**Files:** Create `crates/parser-csv/{Cargo.toml,src/lib.rs,tests/fixtures/GlossFct.csv}`

- [x] RFC 4180 레코드 판독(필드 범위·값·따옴표 여부).
- [x] 표 셀 블록(열 이름 행 비번역, 글자 없는 필드 제외, 구역당 900블록과 열 이름 되풀이).
- [x] 재구성: 따옴표 필드·따옴표가 필요한 번역은 `"…"`와 `""`.
- [x] 검증: `cargo test -p yeokja-parser-csv` (6 passed, GlossFct.csv 바이트 왕복과 Definition 열 선택).

### Task 4: 설정·레지스트리·Markup

**Files:** Modify `crates/core/{Cargo.toml,src/config.rs,src/parser.rs}`, `crates/parsers/{Cargo.toml,src/lib.rs}`

- [x] `Markup::RMarkdown`, `Markup::Plain`.
- [x] `SourceConfig.inline_code_captures`, `PARSER_NAMES`와 모르는 이름 오류, 캡처의 파서 제한·정규식 검사, `INLINE_TAG_PARSERS`에 `rmarkdown`.
- [x] 레지스트리: `rmarkdown`(캡처 포함), `csv`, `.Rmd`/`.rmd`/`.csv` 자동 감지.

### Task 5: 평가기·프롬프트·코덱

**Files:** Modify `crates/translate/{Cargo.toml,src/evaluator_format.rs,src/evaluator_glossary.rs,src/evaluator_link.rs,src/alignment.rs,src/prompt.rs,src/pipeline.rs,src/inline/markdown.rs}`

- [x] 서식 평가: `rmarkdown_issues`(수식 대조, 토큰 다중집합), 수식 가린 Markdown 검사, 괄호 검사, Plain 생략.
- [x] 용어집(수식 가림)·링크·정렬·`closing_rule`·`repair_reply`.
- [x] `Dialect::RMarkdown`: 수식·토큰·raw TeX 가림, `$`·`^`·`@` 이스케이프, `@{key}`, `\[` 대괄호 판단.
- [x] 검증:

```sh
cargo test --workspace                      # 979 passed, 0 failed
cargo clippy --workspace --all-targets      # 경고 0
YEOKJA_PROJECTS=<LDAVer2 프로젝트를 담은 디렉터리> cargo test -p yeokja-translate --test inline_roundtrip -- --ignored --nocapture
# RMarkdown: segments 9773, untaggable 22, same html 9751/9751
```

### Task 6: 실측과 문서

**Files:** Modify `docs/translation-project-checklist.md`; Create spec·plan

- [x] scratch 프로젝트에서 `yeokja status`/`coverage`(spec 4절 표).
- [x] `cargo build --release -p yeokja-cli`.
- [x] 체크리스트의 인라인 태그 안내에 `rmarkdown`, `[[tables]]` 안내에 `csv`.
