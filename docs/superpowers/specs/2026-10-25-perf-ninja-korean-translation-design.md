# Performance Ninja 한국어 번역 설계

## 목표

Denis Bakhvalov의 온라인 성능 최적화 강좌 *Performance Ninja*
(`https://github.com/dendibakh/perf-ninja`, CC BY 4.0)의 Markdown 문서 전체를
yeokja로 한국어 번역해 GitHub Pages(`perf-ninja/`)에 정적 사이트로 낸다. 강좌는
저장소 README 트리(루트 안내 문서, 범주별 README, 실습 과제 README) 그 자체이며
별도 문서 빌드가 없다. 구조가 같은 `projects/learn-fpga`를 선례로 삼는다.

완료 상태는 다음 조건을 모두 만족해야 한다.

- 원문이 `projects/perf-ninja/upstream` 서브모듈로 `b2cb0c3`(2026-08-11)에 고정되어
  있다.
- 39개 Markdown 문서의 모든 세그먼트가 번역 상태이고 `yeokja status --check
  upstream`이 통과한다. `yeokja evaluate --mechanical-only upstream`이 0건이다.
- 빌드 산출물의 제목 수·`<details>` 수·각주 수가 원문과 같고, 사이트 안 상대 링크가
  모두 실재하는 파일을 가리킨다(`scripts/check_site.py`).
- Pages 배포 배선, 랜딩 페이지 항목, README 출처 표기, 루트 `LICENSE.md` 항목이
  갖춰져 있다.

## 조사 결과

- **규모.** Markdown 39개(루트 6, 범주 README 6, 실습 과제 README 26,
  `HugePagesSetupTips.md`), 26,239단어(코드 펜스 밖 19,124), 1,407 세그먼트. 가장 큰
  문서는 `false_sharing_1`(1,573단어). 코드 펜스 161개(그중 C++ 84개)와 벤치마크
  출력은 파서가 넘긴다(`yeokja coverage` 48% 제공, 건너뛴 구간 전부 코드·출력).
- **형식.** 순수 GFM이다. 표·경고 마커·수식이 없다. 각주가 두 문서에 있다
  (`mem_order_violation_1`, `compiler_intrinsics_3`). 후자는 각주 정의 안에 들여쓴
  `asm` 코드 펜스가 있다.
- **HTML.** 실습 과제마다 `<details>`/`<summary><b>…</b></summary>` 접기 상자가
  다섯 개씩 있다(라벨은 `Hint 1:`·`Hint 2:`·`Hint 3:`·`Worked Solution:`·
  `Code for Solution (Link):` 다섯 가지, 99쌍). `Worked Solution` 상자 안에는
  `## Background:` 같은 제목이 들어 있다. 그 밖에 `<br>` 79개, `<sup>` 6개, 루트
  README의 `<p align="center"><img src="/logo.jpg"></p>`(저장소 루트 절대경로)가
  있다.
- **영상.** 실습 과제 README 대부분이 YouTube 영상 썸네일(`[<img
  src="../../../img/X.png">](youtube URL)`, 29개)로 시작한다. 과제의 핵심 설명 일부는
  영상에만 있다.
- **링크.** 디렉터리 링크(`labs/core_bound/vectorization_1`) 21개 이상, 영어 앵커
  링크 3개(`GetStarted.md#how-to-build-lab-assignments`), 정답 코드가 있는 `golden`
  브랜치의 `solution.cpp` 링크 19개.
- **라이선스.** 저장소에 LICENSE 파일이 없고 GitHub API의 license도 비어 있다. 근거는
  README 마지막 줄 "Copyright © 2025 by Denis Bakhvalov under Creative Commons license
  (CC BY 4.0)."와 `Contributing.md`의 "License" 절이다. 제3자 코드가 섞여 있다:
  `labs/misc/pgo/lua`(MIT), `labs/misc/pgo/bench`·`input`(BSD), `labs/misc/pgo/md5`
  (MIT), `labs/misc/lto`의 aobench(New BSD), `compiler_intrinsics_2/inputs/
  LoopVectorize.cpp`(Apache-2.0 WITH LLVM-exception), `tools/gbench`(Google
  Benchmark).
- **원문 상태.** README가 스스로 "work-in-progress"라고 밝히며, "CPU Frontend Bound"와
  "Data-Driven optimizations" 범주는 과제 목록이 비어 있다. `compiler_intrinsics_3`
  첫 문단에 문장이 끝나지 않은 곳이 있다.

## 범위

- 번역: 39개 Markdown 문서의 본문, 제목, 목록, 각주, `<summary>` 라벨, 링크 텍스트.
- 번역하지 않음
  - **C/C++ 코드와 그 주석, 벤치마크·`perf` 출력.** 학습자는 코드를 원본 저장소에서
    내려받아 고치고 제출한다. 사이트의 코드만 주석이 한국어이면 실제로 작업하는
    파일과 어긋나고, 출력은 도구가 내는 영어 그대로여야 대조할 수 있다. 펜스 안
    주석을 번역하려면 C/C++ 주석 파서가 필요하지만 그만한 이득이 없다.
  - **YouTube 영상과 썸네일 이미지.** 썸네일은 영상 링크이며 그림 속 글자는 영상
    제목이다. 영상의 자막·번역은 이 프로젝트 범위 밖이다.
  - 사람 이름, GitHub 계정(`@dendibakh`), 후원 플랫폼 이름, 도구·명령 이름.
- 코드·이미지·제3자 디렉터리를 포함한 저장소의 나머지 파일은 사이트에 원본 그대로
  복사한다(문서의 상대 링크가 가리키므로).

## 접근 방식

### 1. `yeokja.toml`

```toml
[[sources]]
path = "upstream"
pattern = "**/*.md"
parser = "markdown"
inline_tags = true
output = "ko/{path}"

[derive]
base = "upstream"

[[derive.overlay]]
path = "ko"
require_base = true

[build.html]   # scripts/build-site.py 다음 scripts/check_site.py site

[provider]
type = "claude_code"
model = "claude-opus-5-5"
```

빌드 렌더러는 pandoc의 `gfm` 리더다. `$` 수식이 없는 CommonMark 렌더러이므로
체크리스트 2절에 따라 `inline_tags = true`를 켠다. 기본 프롬프트를 쓴다(합쇼체,
GFM 경고 마커 보존 규칙 포함).

### 2. `<summary>` 라벨

다섯 가지 라벨은 모든 과제에 되풀이되는 UI 문구라 번역하고, 첫 번역 뒤
`terms build`의 whole 항목으로 고정해 모든 문서에서 같게 한다. 번역어는
"힌트 1:"·"힌트 2:"·"힌트 3:"·"풀이:"·"해결책 코드(링크):"이다. "Worked Solution"은
단계별 풀이 해설이라 "풀이"로 옮긴다. 풀이 상자 안의 `### Solution:` 제목과 상자 안
"Link to Solution"은 "해결책"으로 옮기므로, 정답 코드 상자도 같은 말을 쓴다. 라벨을 원문 그대로 두는 안은 접기 상자를 연
독자가 마주치는 첫 문구가 영어가 되고, 같은 강좌의 다른 번역 프로젝트와도 맞지 않아
택하지 않는다.

### 3. 용어

`glossary.toml`에 강좌의 주제 용어를 먼저 고정한다. 과제 이름이 곧 기법 이름이라
(Loop Interchange, False Sharing 등) 문서마다 반복된다.

- 기법: 벡터화, 함수 인라이닝, 의존성 체인, 루프 교환, 루프 타일링, 소프트웨어
  프리페칭, 거짓 공유, 대용량 페이지, 메모리 정렬, 메모리 순서 위반, 데이터 패킹,
  룩업 테이블, 조건부 이동, 가상 함수 호출, 링크 타임 최적화(LTO), 프로파일 기반
  최적화(PGO), 컴파일러 내장 함수.
- 측정: 벤치마크, 기준 버전(baseline), 속도 향상(speedup), 핫스팟, 처리량, 지연 시간,
  캐시 미스, 캐시 라인, 분기 예측 실패.
- **Top-down 분석 범주 이름은 원문 그대로 둔다.** Core Bound, Memory Bound, Bad
  Speculation, Frontend Bound, Retiring은 perf·VTune·toplev가 그대로 출력하는 지표
  이름이고 강좌의 범주 이름이기도 하다. "코어 바운드"처럼 음차하거나 "잘못된 추측
  실행"처럼 풀어 쓰면 도구 출력과 대조할 수 없다.
- Linux의 Transparent Huge Pages, Windows의 Large Pages처럼 운영체제 기능 이름은
  원문 그대로 둔다(`huge pages`의 `except`).

### 4. 빌드

`scripts/build-site.py`는 learn-fpga의 같은 이름 스크립트를 복사해 고친 것이다.
조립 트리(`build/tree`)에서 실행한다.

1. 트리 전체를 `site/`로 복사한다(코드·이미지·제3자 디렉터리 포함).
2. 각 Markdown을 pandoc `gfm` 리더로 JSON AST로 읽고, 원문 AST의 제목 ID를 번역
   AST에 옮겨 영어 앵커를 유지한다. 제목 수가 다르면 실패한다. `<details>` 안의
   `## Background:` 같은 제목도 같은 방식으로 센다.
3. HTML로 쓴 뒤 `href`/`src`를 고친다.
   - 상대 링크: Markdown 문서는 `README.md` → `index.html`, 그 밖의 `.md` →
     `.html`, 디렉터리는 `<dir>/index.html`(디렉터리마다 문서가 없으면 파일 목록
     페이지를 만든다).
   - 저장소 루트 절대경로(`/logo.jpg`): 현재 문서에서의 상대 경로로 바꾼다.
     GitHub는 이를 저장소 루트로 읽지만 Pages의 `perf-ninja/` 아래에서는 사이트
     루트를 가리키게 되기 때문이다.
   - `github.com/dendibakh/perf-ninja/(blob|tree)/main/…`: 사이트 안 페이지로 바꾼다.
     `golden` 브랜치 링크는 고정한 커밋에 없는 정답 코드이므로 원본 저장소 링크로
     남긴다. 트리에 없는 파일은 원본 저장소의 `main` 링크로 바꾼다.
4. 각 쪽에 탐색 줄(한국어 홈, 원문 링크, "yeokja 기계 번역")과 하단(저작자 Denis
   Bakhvalov, CC BY 4.0 링크, 원문을 번역·수정했다는 고지, 코드는 원문 그대로라는
   안내)을 붙인다. `<details>`가 접힌 상자로 보이도록 최소한의 스타일을 둔다.

`scripts/check_site.py`는 빌드 직후 다음을 검사하고 어긋나면 실패한다.

- 각 문서의 제목(`<h1>`–`<h6>`) 수, `<details>` 수, 각주 수가 원문을 같은 방식으로
  렌더링한 결과와 같다(검사 스크립트가 원문을 같은 pandoc 리더로 렌더링해 비교한다).
- 사이트 안의 모든 상대 `href`/`src`가 실재하는 파일을 가리키고, 조각이 있으면 그
  페이지에 같은 `id`가 있다.

`scripts/test_build_site.py`는 링크 재작성(디렉터리, `/logo.jpg`, `main`/`golden`)과
제목 ID 보존을 단위 시험한다.

빌드 툴체인은 `nix/projects/perf-ninja.nix`에 `python3`, `pandoc`을 선언한다.

### 5. 재구성 확인

첫 번역 뒤 재구성된 `ko/` 문서를 원문과 diff로 비교해 다음이 보존되는지 본다. 깨지면
파서 문제로 따로 다룬다.

- `<details>` 다음 줄의 `<summary><b>…</b></summary>`(HTML 블록과 인라인 태그가
  한 줄에 섞인 형태).
- `compiler_intrinsics_3` 각주 정의 안의 들여쓴 `asm` 코드 펜스.
- `<details>` 안 `## Background:` 제목 수(빌드 검사로도 확인).

## 라이선스

원문은 CC BY 4.0이다. 번역본은 개작물로서 CC BY 4.0으로 배포하고, §3(a)에 따라
README와 사이트의 모든 쪽 하단에 저작자(Denis Bakhvalov 및 실습 과제 저자들), 원문
링크, 라이선스 링크, 번역·수정 사실을 표시한다. LICENSE 파일이 없으므로 README에
원문 고지 문장을 그대로 인용해 근거를 남긴다.

제3자 코드(위 조사 결과의 목록)는 번역하지 않고 원본 그대로 복사하며 각 디렉터리의
라이선스 고지를 유지한다. README에 그 목록을 적는다. 루트 `LICENSE.md` 표에는
"CC BY 4.0 (LICENSE 파일 없음, README 고지 기준; 제3자 코드는 각 라이선스 유지)"를
더한다.

## 배포

`.github/pages-projects.json`에 `project = "perf-ninja"`, `target = "html"`,
`artifact = "dist-perf-ninja"`를 더하고, `stage-pages.sh`에
`overlay_site "dist-perf-ninja" "site" "perf-ninja"`를 더한다.
`rebuild-translations.sh`는 learn-fpga와 같은 분기(`upstream` 전체를 재구성한 뒤
`status --check upstream`)를 쓴다. 랜딩 페이지에는 "하드웨어" 주제에 둔다. 강좌가
CPU 마이크로아키텍처 수준의 성능 문제를 다루므로 소프트웨어 주제보다 가깝다.

## 작업 순서

1. 프로젝트 골격: `yeokja.toml`, `glossary.toml`, `.gitignore`, README.
2. 시험 번역(`false_sharing_1`: 가장 크고 힌트·풀이·코드·출력을 모두 가짐) → 재시도율,
   문체, 용어 검토 → 용어집 보정.
3. 나머지 번역 → `evaluate --mechanical-only` → `terms build` → `glossary adopt
   --prefer HEAD` → 남은 세그먼트 번역 → `status --check`.
4. 재구성 diff 확인(접근 방식 5).
5. 빌드 스크립트·검사·시험, nix devShell, 브라우저 확인.
6. 배포 배선, 랜딩 페이지, `LICENSE.md`.

## 되짚을 수 있는 결정

- **코드 주석을 번역하지 않음.** 학습자 피드백으로 주석 번역이 필요해지면 C/C++
  주석 파서를 별도 spec으로 다루고, 사이트의 코드와 다운로드할 코드가 어긋난다는
  점을 함께 해결한다.
- **원문이 work-in-progress임.** 새 과제가 추가되면 서브모듈을 올리고 새 문서만
  번역한다. 이때 체크리스트 7절대로 `reconcile`을 쓰지 않는다.
- **pandoc 정적 사이트.** 원문에 문서 빌드가 없어 GitHub의 README 렌더링에 가깝게
  흉내 낸다. learn-fpga와 같은 스크립트가 세 번째로 필요해지면 공용 스크립트로 옮긴다.
