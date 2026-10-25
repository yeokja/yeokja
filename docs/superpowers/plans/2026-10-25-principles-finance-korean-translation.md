# Principles of Finance 2e 한국어 번역 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** OpenStax CNXML 원고를 `cnxml` 파서로 번역하고, 자체 렌더러로 한국어 웹 판을 만들어 Pages 배포 배선까지 갖춘다.

**Architecture:** `upstream` 서브모듈(e3e5135a)을 조립 트리의 바탕으로 두고 `ko/modules/`·`ko/collections/` 번역 미러를 겹친다. `build_site.py`가 번역된 컬렉션과 모듈을 읽어 번호·상호 참조·장 끝 페이지·각주·MathML을 갖춘 정적 사이트를 만들고, `check.py`가 그 앞뒤에서 골격 동치와 링크·앵커를 검사한다.

**Tech Stack:** yeokja CLI(`cnxml` 파서, `claude_code` provider, `claude-opus-5-5`), Python 3 + lxml, MathJax 3.2.2(CDN), Nix devShell, GitHub Actions Pages.

**Spec:** `docs/superpowers/specs/2026-10-25-principles-finance-korean-translation-design.md`

## Global Constraints

- `upstream/`은 수정하지 않는다. 번역 대상은 모듈 131개(`m00088` 제외)와 컬렉션 1개.
- 외부 연습문제(os-embed 580개)는 번역하지 않고 렌더러가 원문 장 끝 페이지 링크 상자로 바꾼다.
- OpenStax 로고·표지 이미지는 산출물에 넣지 않는다.
- README 출처 표기는 AGENTS.md 규칙을 따르고, 모델명은 `state/`에 기록된 번역을 기준으로 확인한다.
- `ko/`, `build/`, `dist/`, `__pycache__/`는 커밋하지 않는다.
- 커밋 메시지: `[principles-finance] <type>: <한국어 요약>`.

---

### Task 1: 프로젝트 골격

**Files:** Create `projects/principles-finance/{yeokja.toml,glossary.toml,.gitignore}`, `nix/projects/principles-finance.nix`

- [x] `yeokja.toml`: `cnxml` 소스 둘(모듈, 컬렉션), `derive` 오버레이, `build.html`, `[translation] concurrency = 6, batch_segments = 32`.
- [x] `glossary.toml`: 국내 재무관리 교재 용어(여러 단어 용어 위주, 짧은 일상어 제외).
- [x] nix devShell: `python3.withPackages (ps: [ ps.lxml ])`.
- [x] 확인: `yeokja status upstream` → 132개 파일, 15,230 세그먼트.

### Task 2: 시험 번역(`m00002`, 1.1절)

- [x] `yeokja translate upstream/modules/m00002/index.cnxml`: 202개, 재시도 2건, `evaluate --mechanical-only` 0건.
- [x] 검토: 합쇼체, 용어집 반영, 인라인 태그 보존. 각주 출처가 `Ben S.`에서 나뉘어 이웃 세그먼트 내용을 겹쳐 쓴 번역 1건을 state에서 바로잡는다.

### Task 3: 전체 번역과 용어 기준

- [x] 컬렉션 순서의 모듈 목록을 둘로 나눠 모듈 단위로 `yeokja translate`(프로세스 둘), 이어서 `upstream/collections`.
- [x] `evaluate --mechanical-only` → 출처 표기의 `Federal Reserve` 오탐 → 지역 연준·이사회 항목을 용어집에 더하고 `exchange rate`에 `except`.
- [x] `yeokja terms build`(2,000 후보 중 423개 용어, 42개 세그먼트 항목) → `terms.toml` 검토(일반어·조각 15개 삭제 후 `rejected`에 기록, `interest rate`를 "이자율"로, `financial markets`를 "금융시장"으로) → `glossary adopt` → 남은 1,815개 `translate`.
- [x] 재평가 오탐(동사 `stocks`·`contract`, `In return`, `supply and demand curves`)에 `except`, 가상 회사 이름 `Alpha`를 "알파"로 → 20개 재번역. 문장 분할이 `Yahoo! Finance`를 가른 세그먼트 1건을 state에서 바로잡는다.
- [x] 확인: `status --check upstream` 통과, `evaluate --mechanical-only upstream` 0건.

### Task 4: 렌더러와 검사

**Files:** Create `projects/principles-finance/scripts/{build_site.py,check.py}`

- [x] `build_site.py TREE OUT UPSTREAM`: 페이지 구성(머리말, 장 도입부, 절, 장 끝 페이지), 그림·표 번호, 상호 참조 텍스트, 노트 라벨, 각주, MathML + MathJax, os-embed 상자, 동영상 링크, 이미지 복사, 랜딩 목차, 이전·다음, 공통 하단, CSS.
- [x] `check.py skeleton`: 골격 동치(CNXML 블록 표, `mtext` 텍스트 제외) + 요소 id 다중집합 일치. 번역에서 `term`이 둘로 복제된 세그먼트 1건을 이 검사로 찾아 state에서 바로잡는다.
- [x] `check.py site`: `h1` 하나, 내부 링크·이미지·앵커 실재, id 중복 없음, 빈 상호 참조 없음.
- [x] 원문 사이트 주소 261개(절·도입부·장 끝 페이지·머리말·책 소개)를 한 번씩 열어 200을 확인한다.
- [x] 확인:

```sh
cd projects/principles-finance
nix develop path:../../nix#principles-finance -c ../../target/release/yeokja build html
# skeleton: 133 documents checked / site: 261 pages, 257 images, 256 figures and 134 tables numbered
# site: 261 pages, 3897 local links checked
```

- [x] 브라우저로 랜딩, 장 도입부, 절(수식, 계산기 표), 핵심 용어, 문제 상자, 모바일 폭(가로 스크롤 없음)을 확인한다.

### Task 5: README와 배포 배선

- [x] README: 범위, 라이선스(CC BY-NC-SA 4.0, 비상업 2차적 저작물), 재현, 출처 표기.
- [x] `bash .github/scripts/rebuild-translations.sh principles-finance`가 provider 호출 없이 통과.
- [ ] 호출 측: `.github/pages-projects.json`, `stage-pages.sh`, `rebuild-translations.sh` 분기, 루트 `LICENSE.md`, `site/index.html`의 `finance` 카드.
