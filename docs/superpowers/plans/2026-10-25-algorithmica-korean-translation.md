# Algorithmica(러시아어판) 한국어 번역 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [x]`) syntax for tracking.

**Goal:** Algorithmica 러시아어판(`content/russian`)의 공개 문서 272개를 러시아어 → 한국어로 번역하고, 원문과 같은 Hugo 테마의 한국어 사이트를 Pages 배포 배선까지 갖춘다.

**Architecture:** `upstream/` 서브모듈을 조립 트리의 바탕으로 두고 `ko/content/korean/` 번역 미러와 `overlay/`(한국어 한 언어 config, 사이트 쪽 layouts·i18n)를 겹친다. `prepare_site.py korean`이 영어판·슬라이드를 지우고 그림·초안을 옮기고 front matter를 손질하며, `build.html`이 같은 Hugo로 원문 러시아어 사이트를 함께 빌드해 `check_site.py`로 비교한다.

**Tech Stack:** yeokja CLI(`mkdocs` 파서, `claude_code` provider, `claude-opus-5-5`), Hugo(nixpkgs 0.165 extended), Python 3 표준 라이브러리, Nix devShell, GitHub Actions Pages.

**Spec:** `docs/superpowers/specs/2026-10-25-algorithmica-korean-translation-design.md`

## Global Constraints

- 원문은 `upstream` 서브모듈(`ed1945c3`)이며 수정하지 않는다. 영어판(`content/english`)은 번역·배포하지 않는다.
- 번역 범위는 `scripts/scope.py`가 계산한 exclude(초안 111개 + `dl/aux/**`)를 뺀 272개 문서이며, `yeokja.toml`의 exclude와 같아야 한다.
- 테마 파일은 고치지 않는다. 바꿀 레이아웃은 `overlay/layouts/`, 문구는 `overlay/i18n/ko.toml`에 둔다.
- 산출물에 방문 통계 스크립트가 없어야 한다.
- README 출처 표기는 AGENTS.md 규칙을 따르고, 모델명은 `state/`에 기록된 번역을 기준으로 확인한다.
- `ko/`, `build/`, `dist/`, `__pycache__/`는 커밋하지 않는다.
- 커밋 메시지: `[algorithmica] <type>: <한국어 요약>`.

---

### Task 1: 프로젝트 골격

**Files:** Create `projects/algorithmica/{yeokja.toml,glossary.toml,.gitignore,scripts/scope.py}`, `nix/projects/algorithmica.nix`

- [x] `scripts/scope.py print`로 exclude 배열을 만들어 `yeokja.toml`에 넣는다. `scope.py check` 통과.
- [x] 용어집: 핵심 용어의 격·수 형태를 같은 번역어로(설계 2절).
- [x] 확인: `yeokja status upstream/content/russian` → 272개 파일, 7,718 세그먼트. `yeokja coverage`의 건너뛴 긴 구간이 모두 코드·수식·HTML 주석.

### Task 2: 시험 번역

- [x] `yeokja translate upstream/content/russian/cs/range-queries/sparse-table.md`: 56세그먼트, 재시도 0. 합쇼체, 수식·코드·링크 보존, 용어(희소 테이블) 적용 확인.

### Task 3: 전체 번역과 용어 기준

- [x] `yeokja translate upstream/content/russian`: 7,718개 전부, 재시도 24회, 최대 재시도 초과 2건(평가에서 오탐으로 확인).
- [x] `yeokja evaluate --mechanical-only upstream/content/russian`: 남은 10건은 모두 오탐(설계 3절). Alignment 4건(숫자 0·1, `sift down`), Format 3세그먼트×2(원문 `$ ` 공백). НОД 용어 오탐 1건은 용어집에서 НОД를 빼서 해소. nim·행렬 문서의 러시아어가 남은 2세그먼트는 state에서 직접 고침.
- [x] `yeokja terms build`: 후보 2,000개 중 169개 용어·5개 세그먼트 항목. 검토에서 вершина·ребро 격 형태와 компонент 9개를 지워 `rejected`로, Фарея→페리·скалярное/векторное произведение→내적/외적·левой доли→왼쪽 부분으로 수정 → `yeokja glossary adopt`(state가 아직 커밋 전이라 `--prefer HEAD` 없이) → 남은 123개 `yeokja translate` → `status --check` 통과.

### Task 4: 빌드와 검사

**Files:** Create `projects/algorithmica/overlay/{config.yaml,i18n/ko.toml,layouts/404.html,layouts/_default/baseof.html,layouts/partials/{analytics,buttons,footer,head,header}.html}`, `projects/algorithmica/scripts/{prepare_site.py,check_site.py,original.yaml}`

- [x] `prepare_site.py korean|russian`: 영어판·reveal-hugo·`dl/aux` 삭제, 그림·초안 복사, 번역 누락 검사, `.ru.md` 이름 바꾸기, `published`/`date: {}` 삭제, `part` 사전, `originalPath`.
- [x] `check_site.py`: 쪽 목록, 원문에 없던 깨진 링크·앵커, 쪽별 제목·그림·코드 블록·`$` 개수, 방문 통계 스크립트.
- [x] 확인:

```sh
cd projects/algorithmica
nix develop path:../../nix#algorithmica -c ../../target/release/yeokja build html
```

- [x] 브라우저로 첫 화면, 세그먼트 트리 목차, 희소 테이블·다익스트라 본문(수식·코드·그림), 한국어 검색, 모바일 폭(원문처럼 viewport 메타 없이 축소 표시)을 확인했다.

### Task 5: README와 배포 배선

**Files:** Create `projects/algorithmica/README.md`; Modify `.github/pages-projects.json`, `.github/scripts/{stage-pages.sh,rebuild-translations.sh}`, `site/index.html`, `LICENSE.md`

- [x] README: 범위, 영어판 제외 이유, 라이선스(CC BY-SA 4.0, 테마 MIT, 출처 불명 그림), 재현 명령, 출처 표기.
- [ ] Pages 항목, 스테이징 줄, 재구성 전 `scope.py check`와 `status --check upstream/content/russian` 분기.
- [ ] 랜딩 페이지 "알고리즘" 주제 카드, 라이선스 표 줄.
- [x] `bash .github/scripts/rebuild-translations.sh algorithmica`가 provider 호출 없이 통과하는지 확인.
