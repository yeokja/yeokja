# Stephen Diehl 블로그 한국어 번역 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 공개 원고가 없는 stephendiehl.com의 글을 HTML에서 Markdown으로 결정적으로 복원해 `source/`로 커밋하고, yeokja `mkdocs` 파서로 번역해 mdBook + mdbook-katex 사이트를 Pages `stephendiehl/`로 배포한다.

**Architecture:** `scripts/fetch_site.py`가 `/posts/` 색인의 글을 캐시에 내려받아 pandoc으로 `source/posts/<slug>.md`와 `manifest.json`을 만든다. `mkdocs` 파서가 `$` 수식을 가린 채 세그먼트를 제시하고, `scripts/prepare_book.py`가 영어 원고와 ko 오버레이 조립 트리에서 같은 구성의 mdBook을 만들어 앵커 보존·링크·수식 수 검사를 거쳐 `dist/site`를 낸다.

**Tech Stack:** Python 3 표준 라이브러리, pandoc, mdBook 0.5 + mdbook-katex, yeokja(`mkdocs` 파서, claude_code provider), Nix devShell, GitHub Actions Pages.

**Spec:** `docs/superpowers/specs/2026-10-17-stephendiehl-korean-translation-design.md`

## Global Constraints

- 프로젝트: `projects/stephendiehl/`. 원문은 `https://www.stephendiehl.com`, 범위는 `/posts/` 색인의 글 전부와 홈 소개 문구. 사이트맵에만 있는 글, `/writings/`, `/hire/`, `/pgp/`, 태그 페이지는 제외.
- 사이트에 과도한 요청을 보내지 않는다: 요청 간 1초, 프로젝트와 연락처를 밝힌 User-Agent, `build/fetch/` 캐시를 한 번만 채운다.
- `source/`는 커밋한다(복원 원고). `ko/`, `build/`, `dist/`, `__pycache__/`는 커밋하지 않는다.
- 파서 `mkdocs`, `inline_tags` 없음, provider `claude_code` / `claude-opus-5-5`, 커스텀 프롬프트 없음.
- 번역본 라이선스 CC BY-SA 4.0. 모든 페이지에 "Stephen Diehl, stephendiehl.com", 라이선스 링크, 변경 사실, 소개 문구 "Stephen Diehl is a software engineer and writer from London.", 조직 연관 금지 고지를 적는다. 민감한 주제의 글도 원문 그대로 옮긴다.
- README 출처 표기는 AGENTS.md 규칙(yeokja 링크, 실제 번역 모델, 학습 비허용 문장)을 따른다.
- 번역 provider 동시 사용 제한: `yeokja translate` 전에 `pgrep -fl 'yeokja translate'`로 다른 번역이 없는지 확인한다.
- 커밋 메시지: `[stephendiehl] <type>: <한국어 요약>`, 마지막 줄 `Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>`. push하지 않는다.
- `$Y` = 저장소 루트 `target/release/yeokja`. `$P` = `projects/stephendiehl`. `nd` = `nix develop path:../../nix#stephendiehl -c`(`$P`에서).

---

### Task 1: 프로젝트 골격과 devShell

**Files:**
- Create: `$P/.gitignore`, `nix/projects/stephendiehl.nix`

- [ ] **Step 1:** `$P/.gitignore`에 `ko/`, `build/`, `dist/`, `__pycache__/`.
- [ ] **Step 2:** `nix/projects/stephendiehl.nix`에 `mdbook`, `mdbook-katex`, `pandoc`, `python3`를 선언한다.
- [ ] **Step 3:** `cd $P && nd sh -c 'mdbook --version; mdbook-katex --version; pandoc --version | head -1'`로 셸이 열리는지 확인한다.

### Task 2: 원문 복원 스크립트

**Files:**
- Create: `$P/scripts/fetch_site.py`, `$P/scripts/test_fetch_site.py`, `$P/RECONSTRUCTION.md`

- [ ] **Step 1:** `fetch_site.py`에 `fetch`(색인·사이트맵·홈·라이선스·각 글을 `build/fetch/`에 캐시)와 `convert`(캐시 → `source/`) 명령을 만든다. 색인 파싱(`parse_post_index`), 사이트맵 파싱(`parse_sitemap`), `<main>` 추출, `post-meta` 분리, 전처리(`ensure_h1`, `prepare_html`: `<br>\n` → 줄바꿈, 제목 속성 제거, `loading`/`decoding` 제거, 언어 없는 코드에 `language-text`, 링크·앵커 재작성), pandoc 호출, 후처리(`tidy_markdown`, `escape_dollars_in_html_blocks`), front matter와 `manifest.json` 작성.
- [ ] **Step 2:** `test_fetch_site.py`로 각 함수를 작은 HTML 조각으로 테스트한다. pandoc 테스트는 pandoc이 없으면 건너뛴다.
- [ ] **Step 3:** `cd $P && python3 -m unittest -q scripts/test_fetch_site.py`.
- [ ] **Step 4:** `nd python3 scripts/fetch_site.py fetch`, `nd python3 scripts/fetch_site.py convert`. `source/posts/*.md`가 색인의 글 수와 같고 모든 글에 `# 제목`이 하나씩 있는지, 코드 펜스 밖에 `](/`·`href="/`가 없는지 확인한다.
- [ ] **Step 5:** `RECONSTRUCTION.md`에 입력, 변환 규칙, 검증, 알려진 한계를 적는다.

### Task 3: 번역 설정과 파일럿

**Files:**
- Create: `$P/yeokja.toml`, `$P/glossary.toml`

- [ ] **Step 1:** `yeokja.toml`에 `[[sources]]` 두 규칙(`posts/*.md`, `index.md`, 둘 다 `mkdocs`), `[derive]`(base `source`, overlay `ko`), `[build.html]`(Task 4의 명령), `[provider]`, `[evaluation]`, `[translation]`을 쓴다.
- [ ] **Step 2:** `glossary.toml`에 함수형·범주론·컴파일러·AI·수학·금융 용어를 쓰고, 일반어와 겹치는 용어에 `except`를 둔다.
- [ ] **Step 3:** `$Y coverage source --min-lines 6`으로 파서가 건너뛴 긴 구간이 코드·수식뿐인지 확인한다.
- [ ] **Step 4:** 파일럿 5편(`monads`, `mlir_gpu_target`, `calculation_of_volume`, `slippage`, `buddhism_summary`)과 `index.md`를 `$Y translate source/posts/<slug>.md`로 번역하고 `ko/`를 읽어 수식·코드·링크가 보존되는지, 어조가 합쇼체인지 본다.

### Task 4: mdBook 빌드

**Files:**
- Create: `$P/scripts/prepare_book.py`, `test_prepare_book.py`, `finish_html.py`, `test_finish.py`, `check_math.py`; Copy from `projects/furiosa-opt/scripts/`: `preserve_anchors.py`(`--skip` 옵션 추가), `test_anchors.py`, `build_print.py`, `test_print.py`, `check_links.py`

- [ ] **Step 1:** `prepare_book.py <root> <out> --lang en|ko [--manifest]`: `book.toml`, `custom.css`, `src/SUMMARY.md`(소개 → 연도별 part 최신순 → 라이선스), `src/posts/*.md`(front matter → `<p class="post-meta">` 줄), `src/index.md`(소개 + 사이트 고지), `src/license.md`(ko는 요약·고지 번역 + 영어 전문).
- [ ] **Step 2:** `finish_html.py`의 꼬리글에 출처·라이선스·변경·소개 문구·조직 연관 금지·yeokja·모델·학습 비허용 문장을 넣는다.
- [ ] **Step 3:** `check_math.py <en> <ko>`: 쪽별 `<span class="katex">` 수 비교, `<code>` 안의 `&lt;span class="katex` 검출.
- [ ] **Step 4:** `[build.html]` 명령: 두 책 준비 → `mdbook build` 둘 → `preserve_anchors.py --skip index.html --skip license.html` → `build_print.py` 둘 → `finish_html.py` → `check_links.py` → `check_math.py` → `site/`.
- [ ] **Step 5:** `python3 -m unittest discover -s $P/scripts -p 'test_*.py'`.
- [ ] **Step 6:** `cd $P && nd $Y build html`이 통과하고 `dist/site/posts/monads.html`에 KaTeX 수식과 하이라이트된 코드, 상단 메타 줄, 꼬리글이 있는지 본다.

### Task 5: 전체 번역과 용어 기준

- [ ] **Step 1:** `pgrep -fl 'yeokja translate'` 확인 뒤 `cd $P && $Y translate source`를 백그라운드로 돌리고 `$Y status source`로 지켜본다.
- [ ] **Step 2:** `$Y terms build --dry-run` → `$Y terms build` → `terms.toml` 검토(잘못된 번역어 수정·삭제, 형용사 용법 `except`) → `$Y glossary adopt --prefer HEAD` → `$Y translate source`.
- [ ] **Step 3:** `$Y status --check source`, `$Y evaluate --mechanical-only source`. Alignment/Format/Link 항목은 체크리스트 6절대로 확인해 실제 문제는 state에서 지우고 다시 번역한다.
- [ ] **Step 4:** `nd $Y build html` 통과. 수식이 많은 글(`taylor`, `slippage`, `particles`)과 코드가 많은 글(`mlir_gpu_target`), 원시 HTML 표가 있는 글(`training_llms`)을 브라우저로 확인한다.
- [ ] **Step 5:** 저장소 루트에서 `.github/scripts/rebuild-translations.sh stephendiehl target/release/yeokja` 통과.

### Task 6: 배포 배선과 README

**Files:**
- Modify: `.github/pages-projects.json`, `.github/scripts/stage-pages.sh`, `.github/scripts/rebuild-translations.sh`, `.github/workflows/pr.yml`, `LICENSE.md`, `site/index.html`; Create: `$P/README.md`

- [ ] **Step 1:** `pages-projects.json` 끝에 `stephendiehl`/`html`/`dist-stephendiehl`/`projects/stephendiehl/dist` 항목.
- [ ] **Step 2:** `stage-pages.sh`의 마지막 `overlay_site` 뒤에 `overlay_site "dist-stephendiehl" "site" "stephendiehl"`.
- [ ] **Step 3:** `rebuild-translations.sh`에 `stephendiehl)` 분기(`status --check source` → `translate source`).
- [ ] **Step 4:** `pr.yml`에 `projects/stephendiehl/scripts` 단위 테스트 스텝.
- [ ] **Step 5:** `LICENSE.md` 표에 알파벳 순으로 `stephendiehl` 한 줄.
- [ ] **Step 6:** `site/index.html`에 `essays`("에세이") 필터 버튼과 "에세이" 섹션의 "Stephen Diehl 블로그" 항목.
- [ ] **Step 7:** `README.md`: 범위, 복원 방식, 라이선스와 저자 고지, 재현 명령, state/ko/dist 설명, 도구 표, AGENTS.md 출처 표기.

### Task 7: 설계 문서와 커밋

- [ ] **Step 1:** spec과 이 plan을 `docs/superpowers/`에 둔다.
- [ ] **Step 2:** 커밋 순서: 원문 복원 스크립트와 source → 번역 설정·빌드 → 전체 한국어 번역(state) → Pages 배포 배선과 README → 설계 문서.
