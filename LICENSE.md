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
| `chisel-book` | CC BY-SA 4.0 |
| `cp-algorithms` | CC BY-SA 4.0 |
| `cpython-devguide` | CC0 1.0 |
| `fp-lean` | CC BY 4.0 |
| `furiosa-opt` | Apache-2.0 |
| `hott` | CC BY-SA 3.0 |
| `jeffe-algorithms` | CC BY 4.0 (CC BY-NC-SA 4.0인 강의 노트와 허락받은 그림은 번역하지 않음) |
| `learn-fpga` | BSD-3-Clause (하위 디렉터리별 별도 라이선스 유지) |
| `mil` | 교재 텍스트 CC BY 4.0, 소스 Apache-2.0 |
| `napkin` | 본문·PDF CC BY-SA 4.0, LaTeX 소스 GPL-3.0 |
| `nix-dev` | CC BY-SA 4.0 |
| `peps` | PEP마다 다름: 퍼블릭 도메인/CC0 1.0 또는 OPUBL-1.0만 번역 |
| `putting-the-you-in-cpu` | MIT |
| `pypy` | MIT |
| `raytracing` | CC0 1.0 |
| `rust-forge` | MIT OR Apache-2.0 |
| `rustc-dev-guide` | MIT OR Apache-2.0 |
| `thebeambook` | CC BY 4.0 |
| `tpil` | Apache-2.0 |
| `webassembly-component-docs` | CC BY 4.0 |
| `zero-to-nix` | CC BY-NC-SA 4.0 (비상업) |

원문 라이선스 전문은 각 `projects/<name>/upstream/`의 라이선스 파일에 있습니다.
