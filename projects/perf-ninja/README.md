# Performance Ninja 한국어 번역

Denis Bakhvalov의 온라인 성능 최적화 강좌
[dendibakh/perf-ninja](https://github.com/dendibakh/perf-ninja)(Performance Ninja
Class)의 Markdown 문서를 한국어로 옮긴 비공식 번역입니다.
[yeokja](https://github.com/yeokja/yeokja)와 함께 Anthropic 사의 `claude-opus-5-5`
모델을 활용하여 번역되었으며 학습을 모두 비허용한 상태로 작업하였습니다.

원본은 `upstream` 서브모듈에 고정하며(`b2cb0c3`), 번역 상태는 `state/`에
커밋합니다.

게시 주소: <https://yeokja.moreal.dev/perf-ninja/>

## 범위

- 번역 대상: 저장소의 Markdown 39개 전부(루트 안내 문서 6개, 범주별 README 6개,
  실습 과제 README 26개, `HugePagesSetupTips.md`). 본문·제목·각주·링크 텍스트와
  힌트·풀이 접기 상자의 라벨(`힌트 1:`, `풀이:`, `해결책 코드(링크):`)을 번역합니다.
- 번역하지 않음: C/C++ 코드와 그 주석, 벤치마크·`perf` 출력(학습자가 내려받아
  고치는 코드와 도구가 내는 출력과 그대로 대조할 수 있도록), YouTube 영상과 썸네일,
  사람 이름과 계정, 도구 이름. Top-down 분석 범주 이름(Core Bound, Memory Bound,
  Bad Speculation 등)은 perf·VTune 출력에 나오는 지표 이름이라 원문 그대로 둡니다
  (`glossary.toml`).
- **각 실습 과제의 핵심 설명 일부는 영상에만 있습니다.** 과제 README 첫머리의
  썸네일은 원 저자의 YouTube 영상(영어)으로 연결되며, 영상 내용은 번역하지
  않았습니다.
- 원문 README가 밝히듯 강좌는 작업 중인 상태이며, "CPU Frontend Bound"와
  "Data-Driven optimizations" 범주에는 아직 과제가 없습니다. 정답 코드는 원본
  저장소의 `golden` 브랜치에 있어 해당 링크는 원본 저장소로 연결됩니다.

## 라이선스

원문에는 LICENSE 파일이 없습니다. 원문 README의 다음 고지와 `Contributing.md`의
"License" 절을 근거로 강좌 전체(문서와 코드)를
[CC BY 4.0](https://creativecommons.org/licenses/by/4.0/)으로 봅니다.

> Copyright © 2025 by Denis Bakhvalov under Creative Commons license (CC BY 4.0).

이 번역은 원문을 번역·수정한 2차적 저작물로서 같은 CC BY 4.0으로 제공합니다. 웹
판의 모든 쪽 하단에 저작자(Denis Bakhvalov와 각 실습 과제 저자), 원문 링크,
라이선스, 번역·수정 사실을 표시합니다. 원문 저자의 공식 번역이 아닙니다.

다음 제3자 코드는 번역하지 않고 원본 그대로 두며 각자의 라이선스를 따릅니다.

- `labs/misc/pgo/lua`: Lua (MIT, `COPYRIGHT`)
- `labs/misc/pgo/bench`, `labs/misc/pgo/input`: Revised BSD (`license.txt`)
- `labs/misc/pgo/md5`: md5.lua (MIT, `MIT-LICENSE.txt`)
- `labs/misc/lto`: Syoyo Fujita의 aobench (New BSD, 실습 README에 표기)
- `labs/core_bound/compiler_intrinsics_2/inputs/LoopVectorize.cpp`: LLVM
  (Apache-2.0 WITH LLVM-exception)
- `tools/gbench`: Google Benchmark의 비교 도구(디렉터리에 라이선스 파일은 없으며
  원 프로젝트는 Apache-2.0)

## 번역과 빌드

번역에는 인증된 Claude Code CLI가 필요하고, HTML 빌드는 nix devShell(Python 3,
Pandoc)에서 실행합니다.

```sh
git submodule update --init projects/perf-ninja/upstream
cd projects/perf-ninja
../../target/release/yeokja translate upstream          # 누락 세그먼트 번역 및 ko/ 재구성
../../target/release/yeokja status --check upstream
nix develop path:../../nix#perf-ninja -c ../../target/release/yeokja build html   # dist/site
nix develop path:../../nix#perf-ninja -c python3 scripts/test_build_site.py
```

`state/`는 번역의 진실의 원천이므로 커밋합니다. `ko/`, `build/`, `dist/`는
`state/`와 원문에서 언제든 재생성되므로 커밋하지 않습니다.

빌드(`scripts/build-site.py`)는 `ko/`를 원본 위에 조립한 트리를 그대로 복사한 뒤
Markdown을 Pandoc(`gfm`)으로 HTML로 바꿉니다. README는 `index.html`로 바꾸고,
디렉터리 링크와 원본 저장소 `main` 브랜치 링크는 사이트 안의 쪽으로, 저장소 루트
절대경로(`/logo.jpg`)는 문서 기준 상대 경로로 고칩니다. 제목 ID는 원문에서 가져와
영어 앵커를 유지합니다. 이어서 `scripts/check_site.py`가 문서마다 제목·접기 상자·
각주 수가 원문과 같은지, 사이트 안 상대 링크와 앵커가 모두 실재하는지 검사합니다.
Deploy Pages 워크플로는 커밋된 번역 상태로 `ko/`를 재구성하고 미번역 여부를 검사한
뒤 빌드해, `dist-perf-ninja` 산출물을 Pages의 `perf-ninja/`에 배치합니다.
