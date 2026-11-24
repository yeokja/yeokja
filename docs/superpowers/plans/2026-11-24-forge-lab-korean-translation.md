# FORGE Lab 블로그 한국어 번역 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** FORGE Lab 블로그(`microsoft/forge-lab`의 `blog/`) 글 2개를 영어 → 한국어로 번역하고, 원문 템플릿을 한국어로 옮긴 비공식 번역 사이트를 Pages 배포 배선까지 갖춘다.

**Architecture:** `upstream/blog`를 조립 트리의 바탕으로 두고 `ko/content/posts/` 번역 미러와 `overlay/`(한국어 `hugo.toml`, 손으로 옮긴 템플릿 사본, 한글 글꼴 CSS, 글 목록 `_index.md`)를 겹친다. `prepare_site.py korean`이 개인정보 처리 방침·로고·방문 통계·장식 그림을 지우고 번역 누락을 검사하며, `build.html`이 원문 템플릿 드리프트를 검사하고 같은 Hugo로 영어 원문 사이트를 함께 빌드해 `check_site.py`로 비교한다.

**Tech Stack:** yeokja CLI(`markdown` 파서, 인라인 태그, `claude_code` provider, `claude-opus-5-5`), Hugo(nixpkgs 0.165 extended), Python 3 표준 라이브러리, Nix devShell, GitHub Actions Pages.

**Spec:** `docs/superpowers/specs/2026-11-24-forge-lab-korean-translation-design.md`

## Global Constraints

- 원문은 `upstream` 서브모듈(`48f83fc`)이며 수정하지 않는다.
- 번역 범위는 `upstream/blog/content/posts/**/*.md`다. 개인정보 처리 방침 쪽, SVG 도표 속 글자, `slides/`, `cve/cve.json`은 범위 밖이다.
- 원문 템플릿은 고치지 않는다. 바꾼 템플릿은 `overlay/layouts/`에 같은 경로의 사본으로 두고 `scripts/check_upstream.py`에 원본 해시를 적는다.
- 산출물에 방문 통계와 Microsoft 로고가 없고, 모든 쪽에 비공식 번역 고지가 있다.
- README 출처 표기는 AGENTS.md 규칙을 따르고, 모델명은 `state/`에 기록된 번역을 기준으로 확인한다.
- `ko/`, `build/`, `dist/`, `__pycache__/`는 커밋하지 않는다.
- 커밋 메시지: `[forge-lab] <type>: <한국어 요약>`.

---

### Task 1: 프로젝트 골격과 빌드 준비

**Files:** Create `projects/forge-lab/{yeokja.toml,glossary.toml,.gitignore,README.md}`, `projects/forge-lab/overlay/{hugo.toml,content/posts/_index.md,assets/css/translation.css,layouts/index.html,layouts/_default/{baseof,single,list}.html,layouts/partials/{post-list,social-meta}.html}`, `projects/forge-lab/scripts/{prepare_site.py,check_site.py,check_upstream.py,original.toml}`, `nix/projects/forge-lab.nix`

- [x] devShell: `nix develop path:../../nix#forge-lab -c hugo version` → 0.165.0 extended.
- [x] overlay 템플릿·설정·CSS, `prepare_site.py`, `check_upstream.py`, `check_site.py`, `original.toml`.
- [x] 확인: 영어 글을 `ko/` 자리에 둔 scratch 사본에서 `yeokja build html`이 조립·두 빌드·검사까지 돌고, 실패가 "그림 alt·본문에 한글 없음"뿐이다(쪽 5개, 깨진 링크 0, 원문에서만 깨진 Privacy 링크 4).
- [x] 브라우저로 첫 화면(데스크톱·모바일)과 글 쪽의 번역 고지·날짜 형식·목차를 확인했다.

### Task 2: 코어 지원 반영

- [x] 코어의 `hugo` 파서(TOML front matter, shortcode 줄, 링크 안 문장 나누기)로 소스를 `parser = "hugo"`, `front_matter_keys = ["title", "description"]`로 둔다.
- [x] `yeokja status upstream/blog/content/posts` → 2개 파일, 301 세그먼트. `yeokja coverage`의 건너뛴 긴 구간은 front matter의 번역하지 않는 키와 코드 블록뿐이다.

### Task 3: 시험 번역

- [x] Rust 글 앞 125줄(front matter, note 상자, 그림 하나 포함)을 scratch 사본으로 번역: 51세그먼트, 재시도 0, 합쇼체, front matter·shortcode 줄·코드 블록 보존, codename MDASH·Hyenas·이슈 번호 보존. 그림만 있는 문단의 alt가 옮겨지지 않음을 확인(인라인 태그 모드에서 그림이 불투명 태그) → 코어 수정 대상.

### Task 4: 전체 번역과 용어 기준

- [x] `yeokja translate upstream/blog/content/posts`: 297개 번역, 재시도 0. 그림만 있는 문단 4개는 코어 수정 전까지 Pending으로 둔다.
- [x] `yeokja evaluate --mechanical-only upstream/blog/content/posts` 0건.
- [x] `yeokja terms build`: 후보 209개 중 42개 용어. 검토에서 `Lab`을 지워 `rejected`에 `lab`을 두고("Beyond the lab"은 연구실 이름이 아님), `FORGE`에 동사 용법 `except = ["can't forge"]`를 둔다 → `glossary adopt --prefer HEAD` → 남은 14개 `translate` → evaluate 0건.
- [x] 코어의 그림 alt 지원이 들어오면 그림만 있는 문단 4개를 `yeokja translate`로 번역하고 `status --check` 통과.

### Task 5: 빌드와 검사

- [x] `nix develop path:../../nix#forge-lab -c ../../target/release/yeokja build html`이 `check_upstream.py`·`check_site.py`를 통과한다.
- [x] 브라우저로 첫 화면, 글 목록, 두 글(목차 앵커, 표, 코드, note 상자, SVG 도표), 모바일 폭을 확인했다.

### Task 6: README와 배포 배선

**Files:** Modify `projects/forge-lab/README.md`; (호출 측) `.github/pages-projects.json`, `.github/scripts/{stage-pages.sh,rebuild-translations.sh}`, `site/index.html`, `LICENSE.md`

- [x] README 출처 표기의 모델명: provider 설정은 처음부터 `claude_code`/`claude-opus-5-5` 하나뿐이고 state의 모든 번역이 그 설정으로 만들어졌다.
- [x] `bash .github/scripts/rebuild-translations.sh forge-lab` 통과.
