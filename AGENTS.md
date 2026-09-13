# AGENTS.md

이 저장소는 두 종류의 코드베이스를 담고 있습니다.

- `crates/`, `web/`: yeokja 번역 도구 자체 (Rust CLI/서버, 웹 프런트엔드)
- `projects/<name>/`: yeokja로 특정 문서를 한국어로 번역하는 개별 프로젝트. 각각
  `yeokja.toml`, `README.md`, `state/`(번역 상태, 진실의 원천)를 가집니다.

## 번역 프로젝트를 추가하거나 바꿀 때

규모가 있는 변경(새 번역 프로젝트 추가, 파서 확장 등)은 `docs/superpowers/`의
spec-driven 워크플로(design spec → implementation plan)를 따릅니다. 기존 예시는
`docs/superpowers/specs/`와 `docs/superpowers/plans/`에 있습니다. 새 번역
프로젝트를 계획할 때는 먼저 `docs/translation-project-checklist.md`를 확인하세요
— 반복되는 단계(서브모듈 고정, `yeokja.toml`, 용어집, README 출처 표기, Pages
배포 배선)를 정리한 체크리스트입니다.

## 번역 프로젝트 README의 출처 표기 규칙 (필수)

`projects/*/README.md`가 "yeokja와 함께 어떤 AI 회사의 어떤 모델로 번역했다"는
내용을 언급할 때는 다음 세 가지를 모두 지킵니다.

1. **yeokja에 하이퍼링크를 건다.** 링크 대상은 항상
   `https://github.com/yeokja/yeokja`입니다.
2. **실제로 그 프로젝트를 번역한 AI 회사와 모델명을 명시한다.** 회사는 모델
   제공사 기준으로 OpenAI 또는 Anthropic(그 외 provider를 쓰면 해당 회사)이며,
   모델명은 반드시 **그 프로젝트의 `state/**/*.yeokja.json`에 실제로 기록된
   번역에 쓰인 모델**을 기준으로 합니다. `yeokja.toml`의 `[provider]`만 보고
   단정하지 마세요 — provider/model 설정이 이후 다른 프로젝트와 통일하기 위해
   바뀌었지만 기존 번역은 재번역되지 않은 경우(예: `projects/cpython-devguide`)가
   있습니다. 실제 사용 모델은 `git log -p -- projects/<name>/yeokja.toml`로
   provider 변경 이력을 확인하고, `state/**/*.yeokja.json`의 `translated_at`
   타임스탬프를 변경 커밋 시각과 비교해 어느 모델로 번역된 세그먼트인지 확인한
   뒤 적습니다. 설정과 실제 번역 이력이 다르면 실제 이력을 따릅니다.
3. **학습 비허용 고지 문장을 덧붙인다.** 형식:
   "{회사} 사의 `{모델명}` 모델을 활용하여 번역되었으며 학습을 모두 비허용한
   상태로 작업하였습니다."

수정은 해당 문장 주변만 최소로 바꾸고, README의 다른 내용은 건드리지 않습니다.

## GitHub-flavored Markdown 경고 표시(alert) 마커를 번역하지 않기 (필수)

`[!NOTE]`, `[!TIP]`, `[!WARNING]`, `[!IMPORTANT]`, `[!CAUTION]`은 GFM 문법
토큰이며 렌더러가 그대로 인식해야 하므로, 대괄호 안 단어를 한국어로 옮기거나
(`[!참고]`), 콜론 문구로 바꾸거나(`참고: ...`), 통째로 빠뜨려서는 안 됩니다.
`crates/translate/src/prompt.rs`의 기본 프롬프트가 이 규칙을 강제하지만,
프로젝트별 `yeokja.toml`에 `[provider].prompt_template`을 커스텀으로 두면
이 규칙이 자동으로 포함되지 않으므로 **커스텀 템플릿을 쓰는 프로젝트는 이
문장을 직접 명시**해야 합니다(예: `projects/webassembly-component-docs/yeokja.toml`).
새 프로젝트를 추가하거나 기존 `state/**/*.yeokja.json`을 검토할 때는 원문의
alert 마커가 번역문에도 영어 그대로 남아 있는지 확인하세요.
