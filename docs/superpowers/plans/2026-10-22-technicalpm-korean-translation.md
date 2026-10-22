# Technical Project Management in Living and Geometric Order 한국어 번역 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Pressbooks EPUB을 `xhtml` 파서로 번역하고 한국어 EPUB과 웹 판을 Pages 배포 배선까지 갖춘다.

**Architecture:** 압축을 푼 EPUB 트리(`upstream/`)를 조립 트리의 바탕으로 두고 `ko/OEBPS/` 번역 미러를 겹친다. `build_epub.py`가 목차·NCX·OPF를 한국어 제목에서 다시 만들고 고지·그림 제외를 적용해 묶으며, `build_site.py`가 같은 책을 웹 페이지로 내놓는다.

**Tech Stack:** yeokja CLI(`xhtml` 파서, `claude_code` provider, `claude-opus-5-5`), Python 3 표준 라이브러리, Nix devShell(python3), GitHub Actions Pages, 로컬 `epubcheck`.

**Spec:** `docs/superpowers/specs/2026-10-22-technicalpm-korean-translation-design.md`

## Global Constraints

- `upstream/`은 원본 EPUB(`upstream.lock`의 SHA-256)과 같은 108개 파일이며 수정하지 않는다.
- 번역 대상은 `upstream/OEBPS/*.html`에서 `front-cover.html`, `table-of-contents.html`을 뺀 28개 문서.
- 1장 TMS 그림(`attachment_343`, `assets/Figure-1-3.png`)은 빌드가 자리 표시로 바꾸고 파일과 manifest 항목을 뺀다.
- README 출처 표기는 AGENTS.md 규칙을 따르고, 모델명은 `state/`에 기록된 번역을 기준으로 확인한다.
- `ko/`, `build/`, `dist/`, `__pycache__/`는 커밋하지 않는다.
- 커밋 메시지: `[technicalpm] <type>: <한국어 요약>`. push하지 않는다.

---

### Task 1: 프로젝트 골격

**Files:** Create `projects/technicalpm/{upstream/,upstream.lock,yeokja.toml,glossary.toml,.gitignore,README.md}`

- [x] EPUB을 풀어 `upstream/`에 두고 항목 목록이 EPUB과 같은지 확인한다.
- [x] `yeokja.toml`: `state_dir = "state"`, `xhtml` 소스와 `exclude`, `derive` 오버레이, `build.epub`·`build.html`.
- [x] 확인: `yeokja status upstream/OEBPS` → 28개 파일, 8,637 세그먼트. `yeokja coverage`의 건너뛴 구간이 모두 마크업으로 시작.

### Task 2: 시험 번역(1장)

- [x] `yeokja translate upstream/OEBPS/chapter-001-…html`: 602개, 태그 판독 실패 0, 재시도 7건(모두 `Lean` 용어 오탐) → `Lean`·`Agile`을 용어집에서 뺀다.
- [x] `scripts/check.py skeleton`으로 골격 동치 확인.

### Task 3: 전체 번역과 용어 기준

- [x] `yeokja translate upstream/OEBPS`: 8,637개 전부, 태그 판독 실패 0, 재시도 16회.
- [x] `yeokja evaluate --mechanical-only upstream/OEBPS` → 0건(모델이 글자로 쓴 `&nbsp;` 1건을 직렬화기·링크 평가기 수정과 함께 바로잡음).
- [ ] `yeokja terms build` → `terms.toml` 검토 → `yeokja glossary adopt --prefer HEAD` → 남은 세그먼트 `yeokja translate` → `status --check`, `evaluate --mechanical-only` 재확인.

### Task 4: 빌드와 검사

**Files:** Create `projects/technicalpm/scripts/{build_epub.py,build_site.py,check.py}`

- [x] `build_epub.py`: 실제 파일로 스테이징, 목차·NCX·OPF·`<title>`·`xml:lang`, 판권 고지, 그림 제외, `mimetype` 무압축 첫 항목.
- [x] `build_site.py`: spine 순서 페이지, 이전·다음 탐색, 공통 하단, 랜딩 페이지, EPUB 내려받기.
- [x] `check.py skeleton|epub`을 두 빌드 대상 앞뒤에 넣는다.
- [ ] 확인:

```sh
cd projects/technicalpm
nix develop path:../../nix#technicalpm -c ../../target/release/yeokja build html
epubcheck build/tree/technicalpm-ko.epub   # RSC-006 3건, RSC-012 3건(원문과 같음)
```

- [ ] 브라우저로 랜딩, 목차, 1장(자리 표시 그림), 판권 고지, 모바일 폭을 확인한다.

### Task 5: 배포 배선

**Files:** Modify `.github/pages-projects.json`, `.github/scripts/{stage-pages.sh,rebuild-translations.sh}`, `site/index.html`, `LICENSE.md`; Create `nix/projects/technicalpm.nix`

- [x] Pages 항목, 스테이징 줄, 재구성 전 `status --check upstream/OEBPS` 분기.
- [x] 랜딩 페이지 "프로젝트 관리" 주제와 항목, 라이선스 표 줄.
- [ ] `bash .github/scripts/rebuild-translations.sh technicalpm`가 provider 호출 없이 통과하는지 확인.
