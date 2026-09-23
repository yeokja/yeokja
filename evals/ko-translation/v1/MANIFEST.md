# eval 세트 v1 MANIFEST

이 파일은 `yeokja-eval extract`가 만듭니다. 손으로 고치지 마세요. 설계는
[`../README.md`](../README.md)에 있습니다.

- yeokja 커밋: `4569949b2aaf83b82377ccc0ce38e720c0b8ddd2`
- 시드: `20260923`, 목표 요청 수: 60, 프로젝트당 2~4요청
- 다시 만들기: `cargo run -p yeokja-eval -- extract --seed 20260923 --target 60`

## 라이선스

이 디렉터리 전체에 적용되는 단일 라이선스는 없습니다. 파일마다 아래 라이선스를
따릅니다. 발췌는 원저작물의 일부이며, 각 항목의 `upstream`이 원문 위치(저장소와
고정 커밋)를, `attribution`이 저작자를 가리킵니다. 요청에 포함된 용어집과
`glossaries/`, `synthetic/`, `synthetic.jsonl`, `sources.toml`,
`truncation.toml`, 이 파일은 yeokja 도구와 같은 `MIT OR Apache-2.0`입니다.

| 파일 | 라이선스 | 조건 | 요청 | 블록 |
|---|---|---|---|---|
| `items.cc-by-4.0.jsonl` | CC BY 4.0 — https://creativecommons.org/licenses/by/4.0/ | 저작자 표시 필요. | 16 | 78 |
| `items.cc-by-nc-sa-4.0.jsonl` | CC BY-NC-SA 4.0 — https://creativecommons.org/licenses/by-nc-sa/4.0/ | 저작자 표시, 비영리, 동일조건변경허락. 상업적으로 이용할 수 없습니다. | 3 | 47 |
| `items.cc-by-sa-3.0.jsonl` | CC BY-SA 3.0 — https://creativecommons.org/licenses/by-sa/3.0/ | 저작자 표시, 동일조건변경허락. | 3 | 72 |
| `items.cc-by-sa-4.0.jsonl` | CC BY-SA 4.0 — https://creativecommons.org/licenses/by-sa/4.0/ | 저작자 표시, 동일조건변경허락. | 8 | 145 |
| `items.permissive.jsonl` | 항목마다 적힌 원문 라이선스(MIT, Apache-2.0, BSD-3-Clause, CC0-1.0, 퍼블릭 도메인) | 재배포 시 항목의 `attribution`과 `license`를 유지하세요. | 41 | 631 |
| `synthetic.jsonl` | MIT OR Apache-2.0 (직접 작성) | | 11 | 80 |

## 출처

| 프로젝트 | 파일 | 원문 라이선스 | 저작자 | 원문 | 요청(실제/어려운) |
|---|---|---|---|---|---|
| chisel-book | `items.cc-by-sa-4.0.jsonl` | CC-BY-SA-4.0 | Martin Schoeberl | https://github.com/schoeberl/chisel-book.git@7fdd5b17c434a247bf7b259cf6ac24b01154b2b0 | 1/0 |
| cp-algorithms | `items.cc-by-sa-4.0.jsonl` | CC-BY-SA-4.0 | cp-algorithms contributors | https://github.com/cp-algorithms/cp-algorithms.git@f06f7d5e5630f0b811e37d9e657562ba38c41f6a | 4/0 |
| cpython-devguide | `items.permissive.jsonl` | CC0-1.0 | Python Software Foundation and devguide contributors | https://github.com/python/devguide.git@261dc2116ca81985c5c0cfc59db5a251d2c8db96 | 4/0 |
| fp-lean | `items.cc-by-4.0.jsonl` | CC-BY-4.0 | Microsoft Corporation 2023 and Lean FRO, LLC 2023–2026 | https://github.com/leanprover/fp-lean.git@0abeea545d560a324e2561f7210fbd6ed154fa02 | 3/0 |
| furiosa-opt | `items.permissive.jsonl` | Apache-2.0 | FuriosaAI, Inc. | https://github.com/furiosa-ai/furiosa-opt.git@9b9cf0fdc78df00cdc430eae725a5ad9084a735e | 3/0 |
| hott | `items.cc-by-sa-3.0.jsonl` | CC-BY-SA-3.0 | The Univalent Foundations Program | https://github.com/HoTT/book.git@578b85cc8d586b1677ec4335148adeb443057d24 | 3/0 |
| jeffe-algorithms | `items.cc-by-4.0.jsonl` | CC-BY-4.0 | Jeff Erickson (복원한 영어 원고) | https://github.com/jeffgerickson/algorithms.git@9d4f235ac54e094594e7db6332e20489ba8e830b | 3/0 |
| learn-fpga | `items.permissive.jsonl` | BSD-3-Clause | Bruno Levy and learn-fpga contributors | https://github.com/BrunoLevy/learn-fpga.git@5c08c870315c09ccd9ec64ccde20ab3375b3f273 | 3/0 |
| mil | `items.cc-by-4.0.jsonl` | CC-BY-4.0 | Jeremy Avigad and Patrick Massot | https://github.com/avigad/mathematics_in_lean_source.git@c7a19becc9b7b4e696b0746b135d8c6da1aba707 | 3/0 |
| nix-dev | `items.cc-by-sa-4.0.jsonl` | CC-BY-SA-4.0 | NixOS Foundation and nix.dev contributors | https://github.com/NixOS/nix.dev.git@34b8b09b989f3d1ebac127e20540ccce3ad736b1 | 3/0 |
| peps | `items.permissive.jsonl` | CC0-1.0 OR LicenseRef-Public-Domain | PEP authors | https://github.com/python/peps.git@0e6e554613af307fb4d2c8a1f1426ae087dd5194 | 3/0 |
| putting-the-you-in-cpu | `items.permissive.jsonl` | MIT | Lexi Mattick | https://github.com/hackclub/putting-the-you-in-cpu.git@bd4f776249a209aa30af942ea2b47c9784b64ab5 | 3/0 |
| pypy | `items.permissive.jsonl` | MIT | PyPy contributors | https://github.com/pypy/pypy.git@4eeb73ca9598d4743be0c8e452bf7850b7a17630 | 3/0 |
| raytracing | `items.permissive.jsonl` | CC0-1.0 | Peter Shirley, Trevor David Black and Steve Hollasch | https://github.com/RayTracing/raytracing.github.io.git@0ab7db4c08fe23f23c0d9d30fed166c83cefed91 | 3/0 |
| rust-forge | `items.permissive.jsonl` | MIT OR Apache-2.0 | The Rust Project Developers | https://github.com/rust-lang/rust-forge.git@fad588bac595807a782ec9d4a1d598fdeb647bb2 | 3/2 |
| rustc-dev-guide | `items.permissive.jsonl` | MIT OR Apache-2.0 | The Rust Project Developers | https://github.com/rust-lang/rustc-dev-guide@70010653e2ae78ec2a93eabe6814f46592e99496 | 3/8 |
| thebeambook | `items.cc-by-4.0.jsonl` | CC-BY-4.0 | Erik Stenman and contributors | https://github.com/happi/theBeamBook.git@7998e22e78417dbe20e5136b9aee862a1ecaa404 | 3/0 |
| tpil | `items.permissive.jsonl` | Apache-2.0 | Jeremy Avigad, Leonardo de Moura, Soonho Kong, Sebastian Ullrich and Lean Community | https://github.com/leanprover/theorem_proving_in_lean4.git@4e28129fdd58037f8f5857548d5e99fe4fb0cc57 | 3/0 |
| webassembly-component-docs | `items.cc-by-4.0.jsonl` | CC-BY-4.0 | The Bytecode Alliance | https://github.com/bytecodealliance/component-docs@0117479cf469d61ad45c80315a4d1a7132f8709a | 3/1 |
| zero-to-nix | `items.cc-by-nc-sa-4.0.jsonl` | CC-BY-NC-SA-4.0 | Determinate Systems, Inc. | https://github.com/DeterminateSystems/zero-to-nix.git@edc8bcc56f8e27142176cc48560fc13f1e8da8e5 | 3/0 |

## 구성

| 파서 | 요청 | 블록 |
|---|---|---|
| asciidoc | 3 | 3 |
| latex | 6 | 47 |
| latex-extended | 3 | 72 |
| markdeep | 3 | 62 |
| markdown | 33 | 512 |
| mdx | 6 | 83 |
| mil | 3 | 14 |
| mkdocs | 4 | 51 |
| myst | 3 | 70 |
| pep | 3 | 49 |
| rst | 9 | 84 |
| verso | 6 | 6 |

| 블록 꼬리표 | 블록 |
|---|---|
| inline-markup | 199 |
| long | 48 |
| math | 62 |
| prose | 575 |
| trivial | 230 |

## 오염 고지

이 세트와 원문은 공개 저장소에 있으므로, 이후에 학습된 모델은 이 텍스트를 보았을 수
있습니다. 결과를 해석할 때 고려하고, 새 버전은 합성 항목의 비중을 늘립니다.
