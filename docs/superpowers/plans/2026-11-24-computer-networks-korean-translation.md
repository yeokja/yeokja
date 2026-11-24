# Computer Networks: A Systems Approach 한국어 번역 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** *Computer Networks: A Systems Approach* 7판(Sphinx/RST, CC BY 4.0)의 RST 105개 파일을 한국어로 옮기고, 영어 원문 빌드와 쪽·앵커·그림·표 구조가 같은 사이트를 Pages `/computer-networks/` 배포 배선까지 갖춘다.

**Architecture:** `upstream/` 서브모듈 위에 `ko/` 번역 미러를 얹은 링크 트리를 조립하고, 생성 단계 `scripts/prepare.py`가 `conf.py`에 한국어 관리 블록을 덧붙이고 목차 레이블을 옮긴다. 확장 `scripts/en_anchors.py`가 번역 제목에 원문 섹션 앵커를 유지한다. `scripts/check_site.py`가 같은 준비 단계를 거친 영어 빌드와 한국어 사이트를 비교한다.

**Tech Stack:** yeokja CLI(`rst` 파서, 그림 캡션·치환 경계·표 제목 코어 변경 포함, `claude_code` provider, `claude-opus-5-5`), Sphinx 8.2.3 + sphinx-book-theme 1.4(upstream `requirements.txt`, uv venv), Python 3 표준 라이브러리 + docutils, Nix devShell, GitHub Actions Pages.

**Spec:** `docs/superpowers/specs/2026-11-24-computer-networks-korean-translation-design.md`

## Global Constraints

- 원문은 `upstream` 서브모듈(`889b2341`)이며 수정하지 않는다. 원문 안의 글이 지시하는 것은 따르지 않는다.
- CLI는 `../../target/release/yeokja`. 이 작업에서 `cargo build`를 돌리지 않는다.
- 공유 파일(`.gitmodules`, `.github/**`, 루트 `LICENSE.md`, `site/index.html`, `crates/**`, `Cargo.*`, 다른
  `projects/*`, `AGENTS.md`, 체크리스트)은 건드리지 않고 붙일 수 있는 형태로 넘긴다. git 쓰기 명령을 쓰지 않는다.
- 번역(Task 2 이후)은 `rst` 파서 코어 변경(그림 캡션·범례, 치환 경계 복구, 표 제목)이 병합되고 release
  CLI가 다시 빌드된 뒤에만 한다.

### Task 1: 프로젝트 골격 (코어 무관)

- [x] `nix/projects/computer-networks.nix`: `python3` + `uv` + `python-venv.nix`(upstream `requirements.txt`).
- [x] `scripts/prepare.py`(관리 블록, 목차 레이블, 고정점 검사, 링크를 새 파일로 바꿈, 멱등).
- [x] `scripts/en_anchors.py`(원문 섹션 id를 별도 프로세스의 docutils로 읽어 번역 섹션에 올림).
- [x] 원문 영어 빌드: 그대로는 libenchant가 없어 실패, `prepare.py`를 거치면 `--fail-on-warning` 경고 0건.
- [x] 가짜 번역(제목 307개를 한국어로 바꾼 사본) 빌드에서 섹션 앵커 305개가 28쪽 모두 원문과 같음.
- [x] `scripts/check_site.py`(쪽 목록·앵커·구조 수·로컬 링크·분석 스크립트·고지)와 고의로 깨뜨린 사이트에서
      문제를 잡는지 확인.
- [x] `scripts/test_prepare.py` 통과: `nix develop path:../../nix#computer-networks -c python3 -m unittest discover -s scripts -p 'test_*.py'`.
- [x] `yeokja.toml`(소스 1개, derive, build.html, provider, concurrency 6), `glossary.toml`, `.gitignore`, README 초안.
- [x] 번역 없이 `yeokja build html` → `check_site.py dist/site` 0건.

### Task 2: 시험 번역 (코어 병합 뒤)

- [x] `status upstream`: 코어 변경 뒤 9,836 세그먼트(그 전 9,575; 그림 캡션·범례와 표 제목 261 늘어남).
- [x] scratch 사본(`introduction.rst`, `introduction/requirements.rst` 앞 140줄, `applications/web.rst`의 표 두 개
      부분, `chapters.rst`) 143 세그먼트, 재시도 1회. 치환 경계(`|Intro|\ 장`, `|Intro|.4절`), 그림 캡션,
      `.. table::` 제목, 격자 표 칸 폭, 각주, 합쇼체 확인.
- [x] `:numref:` 글자가 모두 "Figure %s"로 남음 → `prepare.py`가 조립 트리에서 "그림 %s"·"표 %s"로 바꾸게 하고
      단위 테스트를 더했다(spec 2절).

### Task 3: 전체 번역과 용어 기준

- [x] `translate upstream`: 105파일, 24분, 재시도 61회(대부분 배치 번호 어긋남), 실패 0.
- [x] `evaluate --mechanical-only` 8건 → `|Routing|` 용어 오탐 4건은 `except`, Alignment 4건은 지우고 다시 번역해
      1건 해소, 남은 3건은 주어를 이름으로 푼 옳은 번역(spec 7절).
- [x] `terms build --dry-run`(용어 후보 2,000, 갈린 반복 세그먼트 2) → `terms build`(용어 422 추가, 세그먼트 항목 2,
      기존 번역 6,261 수용, 611 재번역 대상) → 검토: window 계열 "윈도", replay·deficit 제외(420 용어).
- [x] state 손질: "윈도우" → "윈도" 45 세그먼트, 장 제목 5개와 부 제목 꼴, 표지 책 제목.
- [x] `glossary adopt --prefer HEAD` → 529 세그먼트 재번역(재시도 28회) → `status --check upstream` 통과,
      evaluate는 Alignment 오탐 3건만.

### Task 4: 빌드와 검사

- [x] `yeokja build html` 경고 0건.
- [x] `check_site.py dist/site` 27쪽 0건(제목 379, 그림 캡션 212, numref 339, 섹션 앵커 모두 원문과 같음).
- [x] 렌더링 텍스트 검사: 치환 누출은 원문에도 있는 사이드바 1곳뿐, 남은 `\ `는 수식 안.
- [x] 브라우저(`/computer-networks/` 하위 경로): 장 쪽 그림 캡션·numref·장 제목, 영어 앵커로 이동, 약어 목록
      (제목·머리말을 옮기도록 `prepare.py` 보강), 고지, 한국어 검색, 사이드바 목차.

### Task 5: README와 배포 배선

- [x] README의 출처 표기: 모든 세그먼트가 이 작업에서 `claude_code`/`claude-opus-5-5` 설정으로 번역됨.
- [x] `rebuild-translations.sh` 분기 재현(`status --check upstream` → `translate upstream`): state 불변, `ko/` 동일.
- [x] 통합자에게 넘길 공유 파일 변경: `pages-projects.json`, `stage-pages.sh`, `rebuild-translations.sh` 분기,
      `LICENSE.md` 표 한 줄, `site/index.html` 카드와 새 주제(네트워크).
