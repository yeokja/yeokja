---
name: translating-issues-in-batch
description: Use when the yeokja repo has one or more open "Translate <url>" GitHub issues to turn into deployed Korean translation projects, especially several at once, or when a translation project must go from survey through core parser gaps, shared Pages wiring, signed commits, push and deploy verification to closing the issue
---

# 번역 이슈 일괄 처리

## 개요

이슈 하나를 프로젝트로 만드는 절차는 `AGENTS.md`와 `docs/translation-project-checklist.md`에
있다. 이 스킬은 그 **위**의 오케스트레이션이다: 여러 이슈를 병렬로 조사해 코어 결함을 먼저
고치고, 프로젝트 에이전트를 병렬로 돌리고, 공유 파일을 한 곳에서 통합해 배포·검증·이슈
닫기까지 끝낸다. 핵심 원칙: **프로젝트 디렉터리는 에이전트가, 공유 파일과 git은 호출 측이.**

## 단계와 필수 산출물

| 단계 | 누가 | 필수 산출물 |
|---|---|---|
| 1 조사 | 이슈마다 서브에이전트 1개(scratch clone, 저장소 불변, translate 금지) | 라이선스 근거(파일·README·테마 i18n·커밋 이력), 형식·빌드 체인, 규모, 임시 `yeokja.toml`로 잰 `status`/`coverage`, **코어 개선 필수/있으면 좋음** 목록 |
| 2 취합 | 호출 측 | 이슈별 판정(진행/보류/다른 판 번역; 기준은 `references/survey-brief.md` 끝), 코어 항목 합집합, 착수 순서 |
| 3 코어 | worktree 서브에이전트(spec+plan) | 테스트·clippy 통과 브랜치, 기존 프로젝트 세그먼트 불변 확인. 필수 항목이 0건이면 이 단계와 통합의 cargo 검증을 생략한다 |
| 4 프로젝트 | 프로젝트마다 서브에이전트 1개 | `references/project-brief.md`의 보고 형식(공유 파일 변경을 붙일 수 있는 형태로) |
| 5 통합 | 호출 측 | `references/integration-checklist.md` 전부 |
| 6 검증·배포 | 호출 측 | 전 프로젝트 재구성, nix 빌드 재현, 서명 커밋, push, Deploy Pages watch, 경로 curl, 이슈 처리(완료는 닫기, 보류는 근거 보고 후 사용자 확인) |

브리프 템플릿: `references/survey-brief.md`, `references/project-brief.md`.
CI에서만 드러나는 결함: `references/ci-pitfalls.md`.

## 순서 규칙

- 코어가 필요 없는 프로젝트는 코어 작업과 **동시에** 시작한다. 코어가 필요한 프로젝트는
  브랜치를 머지하고 release 바이너리를 다시 빌드한 뒤 시작한다.
- `git submodule add`는 호출 측이 **직렬로** 한다(`.gitmodules`·index 충돌). zsh에서 변수
  이름 `path`를 쓰지 않는다(PATH가 덮인다).
- 서브에이전트는 보고서 파일을 쓸 수 없다. 보고 전문을 마지막 메시지로 받아 호출 측이
  scratch에 저장한다.
- 라이선스가 없는 원문은 번역하지 않는다. 다국어 원문은 판마다 라이선스가 다를 수 있다.

## 코어 결함 판별

`coverage` 비율만 믿지 않는다. XML 계열 원문은 문서 전체가 세그먼트 하나가 되어도 99%로
나온다. `status`의 세그먼트 수와 가장 긴 세그먼트 길이를 함께 본다. 파서 이름 오타는
설정 오류로 거절되는지 확인한다.

## 흔한 실수

- 에이전트 보고의 공유 파일 변경을 통합하지 않고 push → 배포에서 프로젝트가 빠진다(404).
- 로컬(macOS)에서만 빌드 확인 → Linux CI에서 파일명 대소문자로 실패한다.
- 프로젝트 `.gitignore`의 `ko/`가 `overlay/ko/`까지 가려 오버레이가 커밋에서 빠진다.
- 코어를 고친 뒤 영향받는 프로젝트의 state를 `translate`로 밀어붙인다 → `reconcile`을 쓴다.
- 번역이 수 시간 느리면 provider가 아니라 호스트 부하를 의심한다(`claude -p` 한 번의
  벽시계 시간과 응답의 `duration_api_ms`를 비교).
