# 라이선스 안내

This repository contains works under several different licenses. There is no
single license for the whole repository; see the table below for each path.

이 저장소에는 라이선스가 서로 다른 저작물이 함께 들어 있습니다. 저장소 전체에
적용되는 단일 라이선스는 없으며, 경로별로 아래 표를 따릅니다.

## yeokja 도구

`crates/`, `web/`, `scripts/`, `nix/`, `site/`, `docs/`, `.github/`와 저장소
루트의 파일, 그리고 `projects/<name>/` 안에서 이 저장소가 직접 작성한 설정과
스크립트(`yeokja.toml`, `glossary.toml`, `scripts/` 등)는 다음 두 라이선스 중
하나를 골라 이용할 수 있습니다 (SPDX: `MIT OR Apache-2.0`).

- MIT License — [`LICENSES/MIT.txt`](LICENSES/MIT.txt)
- Apache License 2.0 — [`LICENSES/Apache-2.0.txt`](LICENSES/Apache-2.0.txt)

yeokja로 만든 번역 결과물에는 이 라이선스가 적용되지 않습니다. 번역 결과물은
원문의 라이선스를 따릅니다.

## 번역 프로젝트

`projects/<name>/`의 원문 서브모듈(`upstream/`), 번역 상태(`state/`), 번역문과
빌드 산출물은 원문의 2차적 저작물이며 원문 라이선스를 따릅니다. 원저작자,
세부 조건, 번역하지 않은 예외 항목은 각 프로젝트의 `README.md`에 있습니다.

| 프로젝트 | 원문 라이선스 |
|---|---|
| `algorithmica` | CC BY-SA 4.0 (러시아어판만; 이용 허락이 없는 영어판은 번역하지 않음), 테마 MIT |
| `aqueous` | 공개 도메인 헌정(불가능한 곳에서는 MIT-0) |
| `chip8-book` | CC0 1.0 |
| `chisel-book` | CC BY-SA 4.0 |
| `cp-algorithms` | CC BY-SA 4.0 |
| `cpython-devguide` | CC0 1.0 |
| `fp-lean` | CC BY 4.0 |
| `furiosa-opt` | Apache-2.0 |
| `hott` | CC BY-SA 3.0 |
| `ila` | 본문 GFDL-1.3-or-later(불변 부분 없음, 수정판 고지·이력은 웹 판에), 빌드용 XSLT 포크 GPL-2.0/3.0, MathBox MIT(데모는 원서 사이트를 가리킴) |
| `jeffe-algorithms` | CC BY 4.0 (CC BY-NC-SA 4.0인 강의 노트와 허락받은 그림은 번역하지 않음) |
| `learn-fpga` | BSD-3-Clause (하위 디렉터리별 별도 라이선스 유지) |
| `learninpublic` | 원고 CC BY 4.0(`LICENSE-CONTENT`), 코드 MIT; 권리 미확인 그림 24점은 싣지 않음, 번역물 CC BY 4.0 |
| `loss-data-analytics` | 본문 CC BY 4.0(서문 라이선스 표기, GettingStarted/LICENSE.md), 저장소 LICENSE CC0 1.0; 번역물 CC BY 4.0 |
| `mil` | 교재 텍스트 CC BY 4.0, 소스 Apache-2.0 |
| `napkin` | 본문·PDF CC BY-SA 4.0, LaTeX 소스 GPL-3.0 |
| `nix-dev` | CC BY-SA 4.0 |
| `perf-ninja` | CC BY 4.0 (LICENSE 파일 없음, README 고지 기준; 제3자 코드는 각 라이선스 유지) |
| `peps` | PEP마다 다름: 퍼블릭 도메인/CC0 1.0 또는 OPUBL-1.0만 번역 |
| `principles-finance` | CC BY-NC-SA 4.0 (비상업; OpenStax 연습문제 서비스의 문제·표지·로고는 싣지 않음) |
| `putting-the-you-in-cpu` | MIT |
| `pypy` | MIT |
| `raytracing` | CC0 1.0 |
| `rust-api-guidelines` | MIT OR Apache-2.0 |
| `rust-forge` | MIT OR Apache-2.0 |
| `rustc-dev-guide` | MIT OR Apache-2.0 |
| `stephendiehl` | 글 CC BY-SA 4.0, 코드 MIT (`source/`는 공개 원고가 없어 게시된 HTML에서 복원한 파생 원고) |
| `technicalpm` | CC BY 4.0 (원서에 허락받아 실린 1장 그림 한 점은 싣지 않음, 라이선스 문구는 `upstream/OEBPS/copyright.html`) |
| `thebeambook` | CC BY 4.0 |
| `tpil` | Apache-2.0 |
| `webassembly-component-docs` | CC BY 4.0 |
| `write-you-a-haskell` | 산문 CC BY-SA 4.0, 코드 MIT (코드는 번역하지 않음) |
| `zero-to-nix` | CC BY-NC-SA 4.0 (비상업) |

원문 라이선스 전문은 각 `projects/<name>/upstream/`의 라이선스 파일에 있습니다.

## 번역 eval 세트

`evals/`의 발췌 파일은 원문 라이선스별로 나뉘어 있으며, 파일마다 적용되는
라이선스는 `evals/ko-translation/v1/MANIFEST.md`에 있습니다(`v1`에서 파생한 세트도 같은
파일 이름에 같은 라이선스를 따릅니다). eval 세트를 위해 직접
작성한 텍스트와 도구·설정 파일은 yeokja 도구와 같은 `MIT OR Apache-2.0`입니다.
