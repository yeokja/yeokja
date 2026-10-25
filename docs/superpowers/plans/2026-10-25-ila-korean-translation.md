# Interactive Linear Algebra 한국어 번역 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** MathBook XML 원문(`src/*.xml`)을 `pretext` 파서로 번역하고, 원 XSLT와 새 수식 렌더러로 한국어 웹 판을 GFDL 수정판 의무와 Pages 배포 배선까지 갖춘다.

**Architecture:** `upstream`(중첩 서브모듈 `mathbook`, `mathbook-assets` 포함)을 조립 트리의 바탕으로 두고 `ko/src/` 번역 미러와 `overlay/`(ko-KR 지역화, 한국어판 XSLT 덮어쓰기, GFDL 조각)를 겹친다. `build_html.py`가 판본 XML에 한국어판 조각을 끼워 XSLT로 HTML을 만들고, `render_latex.py`가 수식·TikZ 자리표시를 latex → dvisvgm SVG로 바꾸며, compass로 원 CSS를 컴파일한다.

**Tech Stack:** yeokja CLI(`pretext` 파서, `claude_code` provider, `claude-opus-5-5`), xsltproc/xmllint, TeX Live(latex, dvisvgm, kotex, mathdesign, pgf), compass 1.0.3, Python 3 표준 라이브러리, Nix devShell, GitHub Actions Pages.

**Spec:** `docs/superpowers/specs/2026-10-25-ila-korean-translation-design.md`

## Global Constraints

- `upstream`은 `a8593c2`, `mathbook`은 `d6dc0da`, `mathbook-assets`는 `48944c9`에 고정하고 수정하지 않는다. `mathbox`는 받지 않는다.
- 번역 대상은 `upstream/src/*.xml`에서 `gfdl.xml`, `schemas.xml`을 뺀 38개 파일, 5,717 세그먼트.
- 원문을 바꾸는 일(언어 코드, 한국어판 조각, 원서 사이트 링크)은 빌드가 판본 XML에서 한다. `ko/` 번역 파일과 `upstream/`은 건드리지 않는다.
- README 출처 표기는 AGENTS.md 규칙을 따르고, 모델명은 `state/`에 기록된 번역을 기준으로 확인한다.
- `ko/`, `build/`, `dist/`, `__pycache__/`는 커밋하지 않는다.
- 커밋 메시지: `[ila] <type>: <한국어 요약>`.

---

### Task 1: 프로젝트 골격

**Files:** Create `projects/ila/{yeokja.toml,glossary.toml,.gitignore,README.md}`

- [x] `yeokja.toml`: `pretext` 소스(`exclude = ["gfdl.xml", "schemas.xml"]`, `output = "ko/src/{path}"`), `[derive]` 오버레이 두 개(`ko` require_base, `overlay`), `[translation] concurrency = 6`.
- [x] `glossary.toml`: 국내 선형대수 교재 관례의 용어와 번역판 제목.
- [x] 확인: `yeokja status upstream/src` → 38개 파일, 5,717 세그먼트.

### Task 2: 시험 번역(`vectors.xml`)

- [x] `yeokja translate upstream/src/vectors.xml`: 89 세그먼트, 재시도 2회.
- [x] 재구성 diff: `<m>` 원자 보존, `p` 안 `me`·`latex-code`가 문장 사이에 그대로 남고 한국어 어순으로 앞뒤 문장이 옮겨짐, 결과가 well-formed. `evaluate --mechanical-only` 0건.

### Task 3: 전체 번역과 용어 기준

- [x] `yeokja translate upstream/src`: 5,628 세그먼트, 재시도 51회(최대 재시도 초과 5블록), 배치 번호 어긋남 1회.
- [x] `evaluate --mechanical-only` → 용어 검사 7건, 모두 TikZ 라벨·색인·`case-by-case basis` 오탐 → 용어집 `except`로 정리해 0건.
- [x] `yeokja terms build` → 용어 242개(복수형 포함 333)와 세그먼트 7개 → 검토: 코드 속 용례 자동 `except`(prose에 나오지 않는 TikZ·색인·수식 토큰만), 번역문이 이미 쓰는 번역어로 수정(직교 분해, 항등변환, 곱셈성, 스칼라 곱, 평행육면체, "가장 잘 맞는 ~"), 어색한 결정 6개 제거(defining property 계열, two noncollinear vectors) → `glossary adopt --prefer HEAD` → 남은 323 세그먼트 `translate`(재시도 33회).
- [x] 남은 용어 검사 5건: 수식 안 `\text{}`(4건)는 `except`, 실제 결함 1건(동차 연립일차방정식)은 고쳐 저장.
- [x] `status --check upstream/src`, `evaluate --mechanical-only` 0건, `check.py skeleton` 38개 통과.

### Task 4: 오버레이, 빌드, 검사

**Files:** Create `projects/ila/overlay/mathbook/xsl/localizations/ko-KR.xsl`, `projects/ila/overlay/src/xsl/ila-ko.xsl`, `projects/ila/overlay/src/ko-edition.xml`, `projects/ila/scripts/{build_html.py,render_latex.py,check.py,ko.css,compass-shim.rb}`, `nix/projects/ila.nix`

- [x] `ko-KR.xsl`: `en-US.xsl` 129문자열 번역.
- [x] `ila-ko.xsl`: 책 스타일시트 import, 영어 하드코딩 템플릿 덮어쓰기, 데모 iframe은 원서 사이트.
- [x] `ko-edition.xml`: 제목 페이지 번역자·날짜, 판권 고지, 이력 부록.
- [x] `render_latex.py`: 인라인 papersize/디스플레이 min bbox, 단일 `\tag` HTML 라벨, 해시 캐시, 묶음 실패 시 개별 재조판, 사진 data URI.
- [x] `build_html.py`: generated XSL, versioning, 판본 XML 수정, XSLT(경고 0 강제), 렌더, compass CSS·JS·글꼴·이미지.
- [x] `check.py skeleton|site`를 빌드 앞뒤에 넣는다.
- [x] 확인:

```sh
cd projects/ila
nix develop path:../../nix#ila -c ../../target/release/yeokja build html
# skeleton: 38 files checked
# render_latex: 3615 distinct snippets ... 32107 placeholders replaced, 0 failed snippets
# site: 4041 pages, 42247 links/knowls/images checked
```

- [x] 브라우저로 제목 페이지(번역자·날짜), 판권, 이력, 본문(인라인·디스플레이 수식, TikZ, 사진 반사 그림), knowl 속 데모 iframe, 모바일 폭(가로 스크롤 없음)을 확인한다.

### Task 5: 배포 배선(호출 측 통합)

**Files:** Modify `.github/pages-projects.json`, `.github/scripts/{stage-pages.sh,rebuild-translations.sh}`, `site/index.html`, `LICENSE.md`

- [ ] Pages 항목(`nested_submodules = "mathbook mathbook-assets"`), 스테이징 줄, 재구성 전 `status --check upstream/src` 분기.
- [ ] 랜딩 페이지 "Math" 주제 카드, 라이선스 표 줄.
- [x] `bash .github/scripts/rebuild-translations.sh ila`가 provider 호출 없이 통과(기본 분기로 확인).
