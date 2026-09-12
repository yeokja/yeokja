# 번역 프로젝트 추가 체크리스트

새 `projects/<name>/` 번역 프로젝트를 추가할 때 기존 프로젝트들에서 반복적으로
나타난 단계를 정리한 것입니다. 이 문서는 새 프로젝트를 만들지는 않습니다 —
규모가 있는 추가 작업은 `docs/superpowers/`의 design spec → implementation plan
워크플로를 따르고(예: `docs/superpowers/specs/2026-08-30-devguide-korean-translation-design.md`
와 그 짝인 `plans/` 문서 참고), 이 체크리스트는 그 spec/plan을 쓸 때 빠뜨리기
쉬운 항목을 확인하는 용도로 씁니다.

## 1. 원문 고정

- 원문을 `projects/<name>/upstream` 서브모듈로 추가하고 특정 커밋에 고정합니다
  (`.gitmodules`에 등록).
- 얕은 클론이 링크 검증이나 `sphinx-last-updated-by-git` 같은 도구와 충돌하지
  않는지 확인합니다.

## 2. `yeokja.toml`

- `[[sources]]`로 파서·패턴·출력 경로를 지정합니다. 필요하면 `[[tables]]`로
  번역 제외 컬럼을 지정합니다 (`yeokja inspect`, `yeokja coverage`로 먼저
  확인).
- `[provider]`에 실제로 쓸 provider/model을 명시합니다. 나중에 provider를
  다른 프로젝트와 통일하려고 바꾸더라도, **이미 번역된 세그먼트는 재번역되지
  않는 한 원래 모델의 결과물로 남는다**는 점을 감안해 README 출처 표기 시점에
  실제 이력을 다시 확인해야 합니다 (AGENTS.md 참고).
- `[derive]`로 upstream 오버레이와 `ko/` 오버레이를 구성합니다.
- `[build.html]`(또는 pdf/epub)에 실제 빌드 명령을 정의합니다.

## 3. 용어집과 평가

- `glossary.toml`로 도메인 용어를 통일합니다.
- `[evaluation]`에서 `auto_evaluate`를 켜고, 필요하면 `style_evaluate`,
  `max_retries`를 조정합니다.

## 4. README

- 범위, 라이선스(원문 라이선스 보존 방식 포함), 재현 명령(`translate`,
  `status --check`, `build`)을 적습니다.
- yeokja와 실제 번역 모델을 언급할 때는 **AGENTS.md의 "번역 프로젝트 README의
  출처 표기 규칙"을 그대로 따릅니다** — yeokja 하이퍼링크, 실제 사용 모델명,
  학습 비허용 고지 문장.
- `state/`는 커밋 대상(진실의 원천), `ko/`·`build/`·`dist/`는 재구성
  가능한 산출물이므로 `.gitignore`에 추가한다는 점을 명시합니다.

## 5. CI/Pages 배포 배선

배포 대상에 포함하려면 다음 두 곳에 항목을 추가합니다 (`.github/workflows/pages.yml`
상단 주석 참고):

- `.github/pages-projects.json`: `project`, `target`, `toolchain`,
  `artifact`, `artifact_path` (필요하면 `unshallow`) 항목을 추가합니다.
- `.github/scripts/stage-pages.sh`: 새 프로젝트의 스테이징 경로를 추가합니다.
- 새 툴체인이 필요하면 `pages.yml`의 `rebuild` 잡에 설치 스텝을 추가합니다.
- CI는 번역을 하지 않고 커밋된 `state/`에서 `ko/`를 재구성만 하며,
  `status --check`가 미번역 세그먼트를 발견하면 실패합니다. 배포 전 로컬에서
  `.github/scripts/rebuild-translations.sh <project>`로 같은 과정을 재현해
  검증할 수 있습니다.

## 6. 검증

- `yeokja status --check`로 미번역 세그먼트가 없는지 확인합니다.
- `yeokja evaluate --mechanical-only`(또는 전체 평가)로 기계적 품질을 확인합니다.
- 빌드 산출물(HTML/PDF/EPUB)의 링크·앵커·경고를 프로젝트별 검증 스크립트나
  빌드 도구의 엄격 모드(예: `--fail-on-warning`)로 확인합니다.
