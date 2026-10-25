# Performance Ninja 한국어 번역 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** perf-ninja 저장소의 Markdown 39개를 번역하고 pandoc 정적 사이트를 Pages `perf-ninja/`에 배포할 수 있게 한다.

**Architecture:** `upstream` 서브모듈을 조립 트리의 바탕으로 두고 `ko/` 번역 미러를 겹친다. `build-site.py`가 Markdown을 pandoc으로 렌더링하면서 원문 제목 ID를 옮기고 링크를 사이트 안으로 고치며, `check_site.py`가 원문과의 구조 일치와 링크 대상을 검사한다.

**Tech Stack:** yeokja CLI(`markdown` 파서, `inline_tags`, `claude_code` provider, `claude-opus-5-5`), Python 3 표준 라이브러리, pandoc, Nix devShell, GitHub Actions Pages.

**Spec:** `docs/superpowers/specs/2026-10-25-perf-ninja-korean-translation-design.md`

## Global Constraints

- 서브모듈은 `b2cb0c3`에 고정하며 수정하지 않는다.
- 번역 대상은 `upstream/**/*.md` 39개 전부. 코드 펜스·코드 주석·벤치마크 출력·영상은 번역하지 않는다.
- README 출처 표기는 AGENTS.md 규칙을 따르고, 모델명은 `state/`에 기록된 번역을 기준으로 확인한다.
- `ko/`, `build/`, `dist/`, `__pycache__/`는 커밋하지 않는다.
- 커밋 메시지: `[perf-ninja] <type>: <한국어 요약>`.

---

### Task 1: 프로젝트 골격

**Files:** Create `projects/perf-ninja/{yeokja.toml,glossary.toml,.gitignore,README.md}`

- [x] `yeokja.toml`: `upstream`/`**/*.md`/`markdown`/`inline_tags = true`/`ko/{path}`, `derive` 오버레이, `build.html`.
- [x] `glossary.toml`: spec 접근 방식 3의 용어.
- [x] 확인: `yeokja status upstream` → 39개 파일, 1,407 세그먼트.

### Task 2: 시험 번역

- [x] `yeokja translate upstream/labs/memory_bound/false_sharing_1/README.md`: 42개, 재시도 0, 강조 표시가 열린 채 끝나는 원문 1건만 태그 없이 전송. 출력의 `\>64` 처리 결함 1건은 state에서 바로잡음.

### Task 3: 전체 번역과 용어 기준

- [x] `yeokja translate upstream`: 1,365개, 재시도 6회(4개 세그먼트), 재시도 초과 1건(다음 항목의 URL 세그먼트).
- [x] `yeokja evaluate --mechanical-only upstream` → 1건: 맨 URL 안의 `~`를 직렬화기가 `\~`로 이스케이프해 링크 평가기가 URL 손실로 봄(`mem_order_violation_1`). state에서 이스케이프를 지워 0건.
- [x] `yeokja terms build`: 후보 607개 중 105개 용어와 whole 2개 추가, 기존 번역 785개 수용. 검토: 잘못된 항목 10개 삭제(`Lua script` → 스크립트, `th element` 등), `prefetch`를 "프리페치"로, whole `Solution`을 "해결책"으로, 모델이 정하지 못한 `Code for Solution (Link)`와 힌트·풀이 라벨 whole 항목을 추가, `bit`에 `except = ["a bit"]`. `glossary adopt --prefer HEAD` → 남은 62개 `translate` → `status --check`, `evaluate --mechanical-only` 0건.

### Task 4: 재구성 확인

- [x] `ko/`와 `upstream/`의 줄 구조 비교: `<details>`·`</details>`·`<summary>` 줄, 제목, 코드 펜스 줄(내용까지 동일), `<br>`, 각주 정의 수가 39개 문서 모두 같음. `compiler_intrinsics_3` 각주 안 `asm` 펜스는 들여쓰기 그대로 보존.

### Task 5: 빌드와 검사

**Files:** Create `projects/perf-ninja/scripts/{build-site.py,check_site.py,test_build_site.py}`, `nix/projects/perf-ninja.nix`

- [x] `build-site.py`: learn-fpga 스크립트를 복사해 UPSTREAM URL, `main`/`golden` 링크, `/logo.jpg` 재작성, CC BY 4.0 하단, `<details>` 스타일.
- [x] `check_site.py`: 제목·`<details>`·각주 수 대조, 상대 링크·조각 대상 존재.
- [x] `test_build_site.py`: 링크 재작성과 제목 ID 보존 단위 시험.
- [x] 확인:

```sh
cd projects/perf-ninja
nix develop path:../../nix#perf-ninja -c ../../target/release/yeokja build html
nix develop path:../../nix#perf-ninja -c python3 scripts/test_build_site.py
```

- [x] 브라우저로 랜딩, 실습 과제 한 쪽(힌트 상자 열기), 각주 쪽을 확인한다.

### Task 6: 배포 배선

**Files:** Modify `.github/pages-projects.json`, `.github/scripts/{stage-pages.sh,rebuild-translations.sh}`, `site/index.html`, `LICENSE.md`

- [ ] Pages 항목, 스테이징 줄, `rebuild-translations.sh`의 `cpython-devguide|learn-fpga` 분기에 `perf-ninja` 추가.
- [ ] 랜딩 페이지 "하드웨어" 주제 항목, 라이선스 표 줄.
- [ ] `bash .github/scripts/rebuild-translations.sh perf-ninja`가 provider 호출 없이 통과하는지 확인.
