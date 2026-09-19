# 경쟁 프로그래밍을 위한 알고리즘 (cp-algorithms 한국어 번역)

[cp-algorithms](https://cp-algorithms.com)([cp-algorithms/cp-algorithms](https://github.com/cp-algorithms/cp-algorithms))를
한국어로 옮긴 비공식 번역입니다. [yeokja](https://github.com/yeokja/yeokja)와 함께 Anthropic 사의
`claude-sonnet-5` 모델을 활용하여 번역되었으며 학습을 모두 비허용한 상태로 작업하였습니다.
재시도 뒤에도 기계 검사를 통과하지 못한 세그먼트 4개는 Anthropic 사의 `claude-opus-5` 모델로 교정하였습니다.

## 범위

- 원문: `upstream/` 서브모듈, 커밋 `f06f7d5e5630f0b811e37d9e657562ba38c41f6a`(2026-09-19)
- 번역 대상: `upstream/src/**/*.md`와 홈 본문 `upstream/README.md`(사이트에서는 `src/index_body`로 포함됩니다)
- 제외: `contrib.md`, `code_of_conduct.md`(원문 저장소 기여 절차), `preview.md`(원문 미리보기 도구)
- 수식, 코드, front matter의 태그는 원문 그대로입니다. 태그 "Translated"는 러시아어 e-maxx에서 옮긴 문서, "Original"은 cp-algorithms에서 새로 쓴 문서라는 원문 표시입니다. 연습 문제 목록의 외부 문제 제목도 원문 그대로 둡니다.

## 라이선스

원문은 cp-algorithms contributors가 [CC BY-SA 4.0](https://creativecommons.org/licenses/by-sa/4.0/)으로
공개했습니다. 이 번역은 원문을 수정한 개작물로서 같은 CC BY-SA 4.0으로 제공하며, 사이트 하단에
원저작자·라이선스·번역 사실을 표시합니다. cp-algorithms의 공식 사이트가 아니며, cp-algorithms의
이름이나 상표를 쓸 권리를 주지 않습니다.

## 재현

```sh
cargo build --release -p yeokja-cli
git submodule update --init projects/cp-algorithms/upstream
cd projects/cp-algorithms
../../target/release/yeokja translate upstream/src
../../target/release/yeokja translate upstream/README.md
../../target/release/yeokja status --check upstream/src
../../target/release/yeokja status --check upstream/README.md
nix develop path:../../nix#cp-algorithms -c ../../target/release/yeokja build html   # dist/site
```

`state/`는 번역의 진실의 원천이므로 커밋합니다. `ko/`, `build/`, `dist/`는 재생성되므로 커밋하지 않습니다.
CI(`.github/scripts/rebuild-translations.sh cp-algorithms`)는 미번역 세그먼트를 먼저 검사한 뒤
state에서 `ko/`를 재구성하므로 번역 provider를 부르지 않습니다.

## 구성

- 파서: `mkdocs`(`crates/parser-mkdocs`) — arithmatex 수식, admonition·details·탭, front matter, Jinja를 인식합니다.
  `scripts/check_parser_corpus.py`는 원문 전체를 항등·가짜 번역으로 재구성해 Python-Markdown 렌더링 구조가
  원문과 같은지 확인합니다.

  ```sh
  cargo run --release -p yeokja-parser-mkdocs --example roundtrip -- projects/cp-algorithms/upstream/src build/cp-roundtrip
  nix develop path:nix#cp-algorithms -c python3 projects/cp-algorithms/scripts/check_parser_corpus.py projects/cp-algorithms/upstream/src build/cp-roundtrip
  ```

- 평가: 수식(`$..$`, `$$..$$`, `\(..\)`, `\[..\]`, `\begin..\end`)이 바이트 단위로 보존되는지, 원문에 없는 수식이나
  Jinja 구분자(`{{`, `{%`, `{#`)가 생기지 않았는지 검사합니다. 재시도 뒤에도 기계 검사를 통과하지 못한 세그먼트
  4개는 state에서 번역문을 직접 고쳤습니다.
- 빌드: 영어 원본을 먼저 빌드해 제목 ID를 뽑고(`scripts/extract_heading_ids.py`), 한국어 빌드에서
  `overlay/hooks/ko_anchors.py`가 번역된 제목에 같은 ID를 입힙니다. 제목 개수가 다르면 빌드가 실패합니다.
  `scripts/prepare_site.py`가 원문 `mkdocs.yml`에서 조립 트리에서 동작하지 않는 git·rss 플러그인과
  분석·기부 배너 설정을 빼고, 태그 색인에 태그 이름의 뜻을 덧붙입니다. `scripts/check_links.py`는
  원문 빌드에 없던 깨진 로컬 링크가 생기면 실패합니다.
