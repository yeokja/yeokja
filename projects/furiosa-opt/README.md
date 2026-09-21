# Tensor Contraction Processor 프로그래밍 (한국어 번역)

[furiosa-ai/furiosa-opt](https://github.com/furiosa-ai/furiosa-opt)의 mdBook 문서
*Programming Tensor Contraction Processors*(`docs/`)를 한국어로 옮긴 비공식
번역입니다. [yeokja](https://github.com/yeokja/yeokja)와 함께 Anthropic 사의
`claude-sonnet-5` 모델을 활용하여 번역되었으며 학습을 모두 비허용한 상태로
작업하였습니다. 번역 뒤 검수 과정에서 전체 세그먼트의 약 2%(표 머리글, 그림
대체 텍스트, 조사, 링크 텍스트 등)는 Anthropic 사의 `claude-opus-5` 모델로
교정하였습니다.

## 범위

- 원문: `upstream/` 서브모듈, 커밋 `9b9cf0fdc78df00cdc430eae725a5ad9084a735e`
- 번역 대상: `upstream/docs/src/**/*.md` (코드 블록, mermaid, 수식, `{{#include}}`는 원문 유지)
- 저장소의 README, `CHANGES.md`, `skills/` 등 책 밖 문서는 제외합니다.
- Fetch Engine, Tensor Unit 같은 하드웨어 구성 요소 이름은 코드 API와의 대응을 위해 영문으로 둡니다.

## 라이선스

원문은 Apache License 2.0입니다. 이 번역은 원문을 수정한 파생 저작물로서 같은
Apache License 2.0으로 제공하며, 배포 사이트에 원문의 `LICENSE`와 `NOTICE`를
함께 싣고 모든 페이지 하단에 번역·수정 사실을 표시합니다. FuriosaAI의
공식 문서가 아니며 상표에 대한 권리를 주장하지 않습니다.

## 재현

```sh
cd projects/furiosa-opt
../../target/release/yeokja translate upstream/docs/src      # 누락 세그먼트 번역 및 ko/ 재구성
../../target/release/yeokja status --check upstream/docs/src
nix develop path:../../nix#furiosa-opt -c ../../target/release/yeokja build html   # dist/site
```

`state/`는 번역의 진실의 원천이므로 커밋합니다. `ko/`, `build/`, `dist/`는
`state/`와 원문에서 언제든 재생성되므로 커밋하지 않습니다.

빌드는 영어 원본과 한국어판을 함께 만들고, 원문의 영어 제목 앵커를 한국어
페이지에 보존합니다(`scripts/preserve_anchors.py`). 원문에 없던 깨진 로컬
링크가 생기면 `scripts/check_links.py`가 빌드를 실패시킵니다.
