# Write You a Haskell 한국어 번역 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** *Write You a Haskell*(mdBook, 산문 CC BY-SA 4.0)를 `markdown` 파서로 yeokja 번역하고, 수식·include·인용이 보존되고 영어 제목 앵커가 살아 있는 한국어 사이트를 Pages 배포 배선까지 갖춘다.

**Architecture:** 원문 서브모듈 저장소 전체를 조립 트리의 바탕으로 두고 `ko/book/src/` 번역 미러를 겹친다. nix devShell이 원문 `book.toml`의 전처리기 체인(include-rs 포크, katex, bib + pandoc)을 제공한다. 빌드는 마커 검사 → 영어 원본 빌드 → 한국어 빌드 → 앵커 보존 → 인쇄 페이지 → 하단 고지 → 링크 검사 순이다.

**Tech Stack:** yeokja CLI(`markdown` 파서, `claude_code` provider), mdBook 0.5.4 + mdbook-katex 0.10.0 + mdbook-bib 0.5.3(패치) + mdbook-include-rs(저자 포크) + pandoc, Python 3 표준 라이브러리 스크립트, Nix devShell, GitHub Actions Pages.

**Spec:** `docs/superpowers/specs/2026-10-17-write-you-a-haskell-korean-translation-design.md`

## Global Constraints

- 원문 서브모듈: `projects/write-you-a-haskell/upstream`, URL `https://github.com/sdiehl/write-you-a-haskell.git`, `shallow = true`, `main`의 `4333ed8c95f016470ca30c950e1744873f004bab`에 고정. `upstream/`은 수정하지 않는다.
- 번역 범위: `upstream/book/src/**/*.md`. `crates/`의 코드, `refs.bib`, 생성되는 참고문헌 페이지는 번역하지 않는다.
- `inline_tags = false`(`$` 수식 때문). provider `type = "claude_code"`, `model = "claude-opus-5-5"`, 커스텀 `prompt_template`(수식·include·인용 보존 규칙과 GFM 마커 규칙 명시).
- 수식(`$..$`, `$$..$$`), `{{#include ...}}`, `[@key]`는 바이트 그대로 보존한다. `scripts/check_markers.py`가 빌드 전에 파일별로 검사한다.
- 한국어 사이트: mdbook 로그에 버전 차이 알림 외 warn/error 없음, `preserve_anchors.py` 성공, `check_links.py` `New: 0`.
- 라이선스: CC BY-SA 4.0 개작물. 저작자 Stephen Diehl, 라이선스 파일 두 개(`LICENSE-PROSE`, `LICENSE`), 변경 고지, 비공식 고지를 유지한다.
- README 출처 표기는 AGENTS.md 규칙(yeokja 링크 `https://github.com/yeokja/yeokja`, 실제 모델, 학습 비허용 문장)을 따른다.
- `ko/`, `build/`, `dist/`, `__pycache__/`는 커밋하지 않는다.
- 번역 provider 동시 사용 제한: `yeokja translate`를 시작하기 전에 `pgrep -fl 'yeokja translate'`로 다른 번역 프로세스가 없는지 확인한다.
- 커밋 메시지: `[write-you-a-haskell] <type>: <한국어 요약>`, 마지막 줄 `Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>`. push하지 않는다.
- `$Y`는 저장소 루트의 `target/release/yeokja`다. crates는 바꾸지 않는다.

---

### Task 1: 원문 서브모듈 고정

**Files:**
- Modify: `.gitmodules`
- Create: `projects/write-you-a-haskell/upstream` (서브모듈)

- [ ] **Step 1: 서브모듈 추가**

```sh
git submodule add --depth 1 https://github.com/sdiehl/write-you-a-haskell.git projects/write-you-a-haskell/upstream
```

- [ ] **Step 2: `.gitmodules`의 새 항목에 `shallow = true`를 추가하고 커밋 해시를 확인**

```sh
git -C projects/write-you-a-haskell/upstream log -1 --format=%H
```

Expected: `4333ed8c95f016470ca30c950e1744873f004bab`.

- [ ] **Step 3: 조사 수치 확인**

```sh
cd projects/write-you-a-haskell
$Y status upstream/book/src      # Files 31, Total segments 4160
$Y coverage upstream/book/src    # 84% offered, 건너뛴 긴 구간은 모두 코드 블록
```

### Task 2: nix devShell

**Files:**
- Create: `nix/projects/write-you-a-haskell.nix`

- [ ] **Step 1: devShell 선언**

`pkgs.mdbook`, `pkgs.mdbook-katex`, `pkgs.pandoc`, `pkgs.python3`에 더해
`pkgs.mdbook-bib.overrideAttrs`로 `src/parser/hayagriva_parser.rs`의
`entry.authors()`를 `entry.authors().or_else(|| entry.editors())`로 바꾸는
postPatch를 두고, `pkgs.rustPlatform.buildRustPackage`로
`sdiehl/mdbook-include-rs` 커밋 `32e5190a5418367059851a2ce5055dceab79b510`을
빌드한다(`src` hash `sha256-Wgxe9xLFpvwcIX96rhZbqf7Q7MmM8M/SH5ARGz+jBlI=`,
`cargoHash`는 `lib.fakeHash`로 한 번 빌드해 얻는다, `doCheck = false`).

- [ ] **Step 2: 셸이 열리고 도구가 모두 있는지 확인**

```sh
nix develop path:nix#write-you-a-haskell -c sh -c 'which mdbook mdbook-katex mdbook-bib mdbook-include-rs pandoc python3'
```

- [ ] **Step 3: 원본 빌드로 전처리기 체인 확인**

```sh
nix develop path:nix#write-you-a-haskell -c mdbook build projects/write-you-a-haskell/upstream/book --dest-dir "$PWD/projects/write-you-a-haskell/build/original"
```

Expected: `build/original/html/ch06-hindley-milner.html`에 `class="katex"`가
있고, `ch01-introduction.html`의 `bibliography.html#haskell2010` 링크에
편집자 이름이 보이며, `ch02-parsing.html`에 `struct Expr`가 들어 있다. 로그의
warn은 mdbook-bib/katex의 "built against" 버전 차이 알림뿐이다.

### Task 3: 프로젝트 설정과 스크립트

**Files:**
- Create: `projects/write-you-a-haskell/yeokja.toml`, `glossary.toml`, `.gitignore`
- Create: `projects/write-you-a-haskell/scripts/{preserve_anchors,build_print,check_links,test_anchors,test_print}.py` (furiosa-opt 사본)
- Create: `projects/write-you-a-haskell/scripts/finish_html.py`, `test_finish.py` (문구만 바꿈)
- Create: `projects/write-you-a-haskell/scripts/check_markers.py`, `test_markers.py`

- [ ] **Step 1: `yeokja.toml`을 spec의 "번역 설정"과 "빌드"대로 작성**

- [ ] **Step 2: `glossary.toml`을 spec의 "용어집과 표기 규칙"대로 작성**

- [ ] **Step 3: `check_markers.py` 테스트 작성 후 구현**

`compare(original, translated)`가 `dollar`(개수), `include`·`citation`
(Counter), `fence`(개수) 차이를 문자열 목록으로 돌려준다.

```sh
python3 -m unittest discover -s projects/write-you-a-haskell/scripts -p 'test_*.py'
```

Expected: OK (12 tests).

### Task 4: 파일럿 번역과 빌드 확인

- [ ] **Step 1: 한 장만 번역**

```sh
cd projects/write-you-a-haskell
$Y translate upstream/book/src/ch03-lambda.md
python3 scripts/check_markers.py upstream/book/src ko/book/src | grep ch03
```

Expected: `ch03-lambda.md`에 mismatch 없음. `ko/book/src/ch03-lambda.md`의
제목이 `# 3장: 람다 대수`이고 `[@pierce2002]`가 그대로 있다.

- [ ] **Step 2: 번역문 품질 점검** — 문체(합쇼체), 1인칭, 용어집 적용, 수식 조사 위치.

### Task 5: 전체 번역과 용어 기준

- [ ] **Step 1: 전체 번역(백그라운드, 로그 모니터링)**

```sh
$Y translate upstream/book/src
$Y status --check upstream/book/src
```

- [ ] **Step 2: 용어 기준 만들기**

```sh
$Y terms build upstream/book/src --dry-run
$Y terms build upstream/book/src
```

`terms.toml`을 검토해 잘못 정해진 번역어는 고치거나 지우고 예외는 `except`로
둔다.

- [ ] **Step 3: 받아들이기와 재번역**

```sh
$Y glossary adopt upstream/book/src --prefer HEAD
$Y translate upstream/book/src
$Y status --check upstream/book/src
```

### Task 6: 검증

- [ ] **Step 1: 마커 검사와 기계 평가**

```sh
python3 scripts/check_markers.py upstream/book/src ko/book/src
$Y evaluate --mechanical-only upstream/book/src
```

Alignment/Format/Link 항목은 체크리스트 6절대로 하나씩 확인해, 실제 문제는
state에서 지우고 다시 번역한다.

- [ ] **Step 2: HTML 빌드**

```sh
nix develop path:../../nix#write-you-a-haskell -c ../../target/release/yeokja build html
```

Expected: `dist/site/`에 한국어 HTML, `check_links.py` `New: 0`,
`preserve_anchors.py` 31 pages, `ch06-hindley-milner.html`에 KaTeX,
`ch02-parsing.html`의 include 코드가 원본과 같은 `<pre>` 수.

- [ ] **Step 3: CI 재구성 경로 재현**

```sh
.github/scripts/rebuild-translations.sh write-you-a-haskell target/release/yeokja
```

### Task 7: 배포 배선과 문서

**Files:**
- Modify: `.github/pages-projects.json`, `.github/scripts/stage-pages.sh`, `.github/scripts/rebuild-translations.sh`, `.github/workflows/pr.yml`, `LICENSE.md`, `site/index.html`
- Create: `projects/write-you-a-haskell/README.md`

- [ ] **Step 1: spec의 "배포 배선"대로 각 파일에 자기 항목만 최소 diff로 추가**
- [ ] **Step 2: README 작성** — furiosa-opt 형태, AGENTS.md 출처 표기 규칙 세 가지.
- [ ] **Step 3: 모델명 확인**

```sh
grep -ho '"model": *"[^"]*"' state/upstream/book/src/*.yeokja.json | sort | uniq -c
```

Expected: `claude-opus-5-5`뿐.

### Task 8: 커밋

순서: 서브모듈·nix devShell → 설정·스크립트·spec/plan → state(번역) → terms.toml → 배포 배선·README.
