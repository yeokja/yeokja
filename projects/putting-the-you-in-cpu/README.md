# Putting the "You" in CPU 한국어판

[Hack Club · Lexi Mattick의 원문](https://github.com/hackclub/putting-the-you-in-cpu)을
[yeokja](https://github.com/yeokja/yeokja)의 `claude_code` provider와 `claude-sonnet-5` 모델로 번역합니다. Anthropic 사의 `claude-sonnet-5` 모델을 활용하여 번역되었으며 학습을 모두 비허용한 상태로 작업하였습니다.
원문과 이미지의 MIT 라이선스는 upstream/LICENSE 및 배포 사이트의 LICENSE에 보존합니다.

## 번역과 빌드

저장소 루트에서 다음 명령을 실행합니다. 빌드 툴체인(Bun, Node.js 24,
Python 3)은 `nix develop path:nix#putting-the-you-in-cpu`가 제공합니다.

```sh
git submodule update --init projects/putting-the-you-in-cpu/upstream
cargo build --release -p yeokja-cli
# 커밋된 state에서 복원하므로 provider 호출 없이 실행됩니다.
.github/scripts/rebuild-translations.sh putting-the-you-in-cpu
cd projects/putting-the-you-in-cpu && nix develop path:../../nix#putting-the-you-in-cpu -c ../../target/release/yeokja build html
```

결과는 `dist/site/`이며 Pages 파이프라인은 이를 `/putting-the-you-in-cpu/`에 배치합니다.
8개 장, 한 페이지 판, 404 페이지를 포함합니다. 원본 그림을 유지하고 대체 텍스트를
번역했습니다. PDF는 upstream의 영어 원문이며 링크에 명시합니다.

원문을 갱신한 경우 프로젝트 디렉터리에서:

```sh
python3 scripts/extract_ui.py
yeokja translate upstream/src/content/chapters
yeokja translate ui
python3 scripts/prepare_ui.py
yeokja build html
```

`ui/`와 `ui.json`은 본문 파서가 제외하는 이미지 대체 텍스트, 메뉴, 짧은 장 이름,
그림 감사 문구를 추출한 입력입니다. `prepare_ui.py`는 번역 결과를 `ko/`에 적용합니다.
반드시 본문 `translate` 뒤에 실행해야 하며, `ko/`에 모두 반영하므로 기존 Pages
빌드 지문에 보충 번역의 변경도 포함됩니다. `state/`와 추출 입력을 함께 커밋합니다.

`prepare_site.py`는 복사한 빌드 트리에서 정적 Astro 설정, 한국어 언어 표기와
장 이동 경로를 적용합니다. `rewrite-base.mjs`는 빌드된 HTML의 이미지·링크에
Pages 하위 경로를 붙입니다. upstream의 분석 스크립트는 배포본에서 제거합니다.

자동 용어 평가의 `kernel/fork.c` 경고는 코드 경로를 번역하지 않아 발생합니다.
원문 경로는 그대로 보존합니다.
