# Loss Data Analytics(2판) 한국어 번역 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [x]`) syntax for tracking.

**Goal:** LDAVer2의 bookdown 원고를 `rmarkdown`·`csv` 파서로 번역하고, devShell에서 knit한 한국어 gitbook을 Pages 배포 배선까지 갖춘다.

**Architecture:** 원문 서브모듈을 조립 트리의 바탕으로 두고 `ko/`(번역 Rmd·GlossFct.csv), `ko-gloss/`(번역한 Gloss 표시어), `overlay/`(한국어 bookdown 설정·CSS)를 겹친다. `prepare_book.py`가 knit 전에 GA 태그·날짜·Gloss를 손질하고 `bookdown::render_book`이 gitbook을 만든다.

**Tech Stack:** yeokja CLI(`rmarkdown`·`csv` 파서, `inline_tags`, `inline_code_captures`, `claude_code` provider, `claude-opus-5-5`), Python 3 표준 라이브러리, R + bookdown(Nix `rWrapper`, CASdatasets는 `buildRPackage`), GitHub Actions Pages.

**Spec:** `docs/superpowers/specs/2026-10-25-loss-data-analytics-korean-translation-design.md`

## Global Constraints

- `upstream/`은 `67f5aab0`에 고정한 서브모듈이며 수정하지 않는다. 원문 손질은 조립 트리에서 `prepare_book.py`가 하고, 바꿀 자리가 없으면 실패한다.
- 번역 대상은 Rmd 29개, `GlossFct.csv` Definition 열, `gloss/GlossLabels.csv` Label 열.
- `Gloss()`의 첫 인자(조회 키)와 `GlossFct.csv`의 Term 열은 번역하지 않는다.
- README 출처 표기는 AGENTS.md 규칙을 따르고, 모델명은 `state/`에 기록된 번역을 기준으로 확인한다.
- `ko/`, `ko-gloss/`, `build/`, `dist/`, `__pycache__/`는 커밋하지 않는다.
- 커밋 메시지: `[loss-data-analytics] <type>: <한국어 요약>`.

---

### Task 1: 프로젝트 골격

**Files:** Create `projects/loss-data-analytics/{yeokja.toml,glossary.toml,.gitignore,README.md,gloss/GlossLabels.csv,scripts/gloss_labels.py,overlay/{_bookdown.yml,_output.yml,Format/ko.css}}`, `nix/projects/loss-data-analytics.nix`

- [x] `yeokja.toml`: 세 소스(Rmd, GlossFct.csv, GlossLabels.csv)와 `[[tables]]` 두 개, `derive` 오버레이 세 개, `[translation] concurrency = 6`.
- [x] `gloss_labels.py extract` → 377개 표시어.
- [x] 확인: `yeokja status upstream` → 30개 파일(Rmd 29 + CSV), 9,038 세그먼트. `yeokja status gloss` → 377.

### Task 2: 시험 번역(빈도 장)

- [x] `yeokja translate upstream/Chapters/Frequency2024.Rmd`(529개) → 저장된 문제, `evaluate --mechanical-only` 확인.
- [x] `scripts/check.py source upstream ko`로 청크·인라인 R·수식·참조 토큰 보존 확인. 디스플레이 수식은 공백 런을 하나로 보고 비교한다(파서 설계 3절).
- [x] 문체·용어 검토 → `glossary.toml` 보정(insurer, Section).
- [x] 결과: 82초, 저장된 문제 0건, `evaluate` 0건, 원문 보존 검사 통과.

### Task 3: 전체 번역과 용어 기준

- [x] `yeokja translate upstream`, `yeokja translate gloss`: 9,038 + 377 세그먼트, 재시도 14회. `"R Code for Kendall's Tau"`처럼 레이블 안에 다른 따옴표가 든 캡처 5개는 캡처 정규식을 따옴표 종류별 그룹으로 고친 뒤 다시 번역.
- [x] `yeokja evaluate --mechanical-only upstream`, `gloss` → 0건.
- [x] `yeokja terms build --dry-run`(후보 2,000, 불일치 반복 세그먼트 4) → `yeokja terms build`(578개 채택, 용어 516 + 반복 세그먼트 4 추가, 기존 번역 5,557 수용) → `terms.toml` 검토(`B&uuml;hlmann`에서 잘린 `hlmann-Straub` 항목의 번역어를 용어집의 "스트라우브"로, 형용사·동사 용법과 겹치는 짧은 항목 삭제, 동사 "claim"·"sample means" 등 `except` 추가, "income inequality" 소득 불평등) → `yeokja glossary adopt --prefer HEAD` → 남은 1,416 세그먼트 `translate`(재시도 47회) → `status --check`, `evaluate --mechanical-only` 0건.

### Task 4: 빌드와 검사

**Files:** Create `projects/loss-data-analytics/scripts/{prepare_book.py,check.py}`

- [x] devShell: `pdftools`·`sf`를 `rPackages.override`로 고침.
- [x] 영어 원문을 devShell에서 knit해 재현을 확인한다: Windows 경로, VGAM `"loge"`, 닫지 않은 `<div>` 3곳·`<p>` 1곳을 `prepare_book.py --portability`로 고친 뒤 930개 청크 모두 실행, 번호 붙은 장 22개.
- [x] `prepare_book.py`: 청크 복원, 이식성, GA 태그, 서문 날짜, 참고 문헌 제목·author, 수식 줄 복원, 참조 링크 복원, Gloss 재정의, 조사 맞춤.
- [x] `check.py site`: 장 수, 로컬 링크·앵커, knitr 오류 출력, `??` 교차 참조.
- [x] 확인:

```sh
cd projects/loss-data-analytics
nix develop path:../../nix#loss-data-analytics -c ../../target/release/yeokja build html
```

- [x] 브라우저로 첫 화면, 빈도 장(Gloss 풍선 도움말, 접기 단추, 수식), 지급준비금 장(tikz 그림)을 확인한다.

### Task 5: 배포 배선

**Files:** Modify `.github/pages-projects.json`, `.github/scripts/{stage-pages.sh,rebuild-translations.sh}`, `site/index.html`, `LICENSE.md`

- [ ] Pages 항목, 스테이징 줄, 재구성 전 `gloss_labels.py extract --check`와 `status --check upstream`·`gloss` 분기.
- [ ] 랜딩 페이지 "경제·금융"(`finance`) 주제와 카드, 라이선스 표 줄.
- [ ] `bash .github/scripts/rebuild-translations.sh loss-data-analytics`가 provider 호출 없이 통과하는지 확인.
