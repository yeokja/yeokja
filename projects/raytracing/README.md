# Ray Tracing 한국어 번역

[Ray Tracing in One Weekend](https://github.com/RayTracing/raytracing.github.io)의
세 권, 감사의 글, 소개 페이지를 한국어로 번역합니다.
원본은 `upstream` submodule의 `release` 리비전으로 고정합니다.
문서·이미지·코드의 라이선스는 원본 `COPYING.txt`의 CC0입니다.

번역은 [yeokja](https://github.com/yeokja/yeokja)의 `claude_code` provider와 `claude-sonnet-5` 모델로 수행합니다. Anthropic 사의 `claude-sonnet-5` 모델을 활용하여 번역되었으며 학습을 모두 비허용한 상태로 작업하였습니다.
`state/`에는 번역과 평가 기록을 커밋하고, `ko/`·`ko-ui/`는 재구성 결과라
커밋하지 않습니다. 제목과 그래픽스 용어는 `glossary.toml`로 관리합니다.

## 재현

저장소 루트에서 CLI를 빌드한 뒤 실행합니다.

```sh
cargo build --release -p yeokja-cli
git submodule update --init projects/raytracing/upstream
python3 projects/raytracing/scripts/prepare_index.py extract --check
.github/scripts/rebuild-translations.sh raytracing
target/release/yeokja -C projects/raytracing build html
```

산출물은 `dist/site/`이며 Pages에서 `/raytracing/`으로 배포됩니다.
빌드는 원본 Markdeep 렌더러, 그림, 스타일을 보존하고 한국어 UI 라벨을
`markdeepOptions.onLoad`에서 표시 문구만 변경해 제공합니다. 입력 문법은 원본대로 유지합니다. 배포 중 LLM 호출은 필요하지 않습니다.
CI는 미번역·변경된 세그먼트를 먼저 검사하므로 완역 state가 없으면 실패합니다.

원본이 변경됐을 때는 소개 페이지 입력을 갱신하고 로컬에서 증분 번역합니다.

```sh
python3 projects/raytracing/scripts/prepare_index.py extract
target/release/yeokja -C projects/raytracing translate upstream/books
target/release/yeokja -C projects/raytracing translate ui
```

## 파서와 검증

책은 HTML 확장자를 쓰는 Markdeep 문서입니다. `parser-markdeep`은 독립 crate이며,
`parser-markdown-dialect`의 공통 span 추출을 사용합니다. Markdown·MDX·MyST도
각각 별도 crate로 분리되어 있습니다.

Markdeep 파서는 제목·여러 줄 캡션·목록을 번역 대상으로 추출하고, 코드·수식
블록·식별자·삽입 지시문을 보존합니다. 문장 안의 수식과 참조는 문맥을 유지해
번역한 뒤 독립 검증기로 보존 여부를 검사합니다.

```sh
cargo test -p yeokja-parser-markdeep
cargo run -p yeokja-parser-markdeep --example audit -- projects/raytracing/upstream/books/*.html
python3 projects/raytracing/scripts/verify_books.py
python3 -m unittest discover -s projects/raytracing/scripts -p 'test_*.py'
```

`verify_books.py`는 원문과 번역문의 코드, 수식, 캡션 식별자, 참조, 문서 삽입,
그림 경로를 비교합니다. 한국어 어순 때문에 수식·참조가 이동하는 것은 허용하지만
반복 횟수 변경·누락은 실패합니다. 빌드는 빠진 번역 파일이나 평가 오류도 거부합니다.
소개 페이지는 HTML의 본문·제목·대체 텍스트를 추출하고, 링크와 태그는 보호 토큰으로
유지합니다. 빌드 지문에는 `ko/`와 `ko-ui/`를 모두 포함합니다.
