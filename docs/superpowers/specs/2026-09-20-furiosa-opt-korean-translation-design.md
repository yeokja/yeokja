# furiosa-opt 문서 한국어 번역 설계

이슈: yeokja/yeokja#1

## 목표

[`furiosa-ai/furiosa-opt`](https://github.com/furiosa-ai/furiosa-opt)의 mdBook
문서 *Programming Tensor Contraction Processors*(`docs/`)를 고정된 원문으로
사용하고, yeokja로 독자에게 보이는 Markdown 전체를 한국어로 번역해 GitHub
Pages에 배포한다.

완료 상태는 다음 조건을 모두 만족해야 한다.

- `furiosa-ai/furiosa-opt`의 고정 커밋이 읽기 전용 서브모듈로 등록되어 있다.
- `upstream/docs/src/**/*.md`(53개 파일, `SUMMARY.md` 포함)의 모든 대상
  세그먼트가 번역 상태이고 `yeokja status --check`가 통과한다.
- 한국어 mdBook HTML 빌드가 성공하고, 원문 빌드에 없던 깨진 로컬 링크·앵커가
  하나도 없다(`check_links.py`가 `New: 0`).
- 원문의 영어 제목 앵커(`#constraints` 등, 약 165개 링크가 사용)가 한국어
  페이지에서도 그대로 동작한다.
- Pages 배포 배선과 랜딩 페이지 항목, README 출처 표기가 갖춰져 있다.

## 범위

`docs/` 아래 mdBook 책만 번역한다. 저장소 루트와 크레이트의 README,
`CHANGES.md`, `skills/**/SKILL.md` 같은 흩어진 문서는 책 사이트에 나오지 않으므로
범위에서 제외한다.

코드 블록(Rust 예제, 숨김 줄 `# ...` 포함), mermaid 다이어그램 블록,
`{{#include ...}}` 지시자, 인라인 코드, URL, 수식(`\\( ... \\)`, `$$ ... $$`)은
원문을 보존한다. `{{#include}}` 71개는 모두 코드 블록 안에 있으며
`../../../furiosa-opt-std/src/...` 같은 `docs/` 바깥 크레이트 소스를 가리킨다.

원문은 Apache-2.0이다. 번역문은 원문을 수정한 파생 저작물로서 Apache-2.0으로
배포하며, 다음을 지킨다.

- 사이트 루트에 원문 `LICENSE`와 `NOTICE`를 함께 싣는다(§4(a), §4(d)).
- 모든 본문 페이지 하단에 "원문을 수정한 번역본"임을 알리는 고지를
  넣는다(§4(b)).
- Apache-2.0은 상표 사용권을 주지 않으므로(§6) 비공식 번역임을 README와
  하단 고지에 밝히고, FuriosaAI 로고나 브랜딩을 추가하지 않는다.

## 접근 방식

### 채택: rustc-dev-guide / webassembly-component-docs의 mdBook 오버레이

두 프로젝트가 쓰는 구조를 그대로 따른다. 원문 서브모듈 위에 `ko/` 번역
미러를 겹쳐 빌드 트리를 조립하고, 영어 원본과 한국어판을 모두 빌드한 뒤
렌더링된 원본 HTML을 기준으로 영어 앵커를 한국어 페이지에 덧붙인다. 필요한
스크립트는 이미 두 프로젝트에 같은 사본으로 존재하며 새 제품 기능이 필요
없다. rustc-dev-guide에서 이 방식은 번역 때문에 깨진 앵커가 0개였다(깨진
링크 44개는 모두 원문에서부터 깨진 것).

### 제외: rust-forge식 단일 `mdbook build`

제목이 번역되면 mdBook이 한국어 앵커 ID를 만들어 영어 앵커 링크가 깨진다.
rust-forge에서 실제로 일반 페이지 104개, `print.html` 104개의 링크가 이렇게
깨져 있었고 별도 수정 중이다. 이 책은 앵커 링크가 약 165개로 촘촘해서 받아들일
수 없다.

## 디렉터리와 책임

```text
projects/furiosa-opt/
├── upstream/       furiosa-ai/furiosa-opt 원문 서브모듈(읽기 전용, shallow)
├── state/          번역 상태(*.yeokja.json, 진실의 원천, 커밋 대상)
├── ko/             state에서 재생성되는 docs/src 미러(커밋하지 않음)
├── scripts/        앵커 보존·인쇄 페이지·링크 검사·하단 고지 스크립트와 테스트
├── build/          원본 빌드와 조립 트리(커밋하지 않음)
├── dist/site/      완성된 한국어 HTML(커밋하지 않음)
├── glossary.toml   용어집
├── yeokja.toml     번역·조립·빌드 설정
└── README.md       범위, 라이선스, 재현 명령, 출처 표기
```

`.gitignore`에 `ko/`, `build/`, `dist/`를 추가한다. `upstream/`은 수정하지
않는다.

## 원문 고정

- `.gitmodules`에 `projects/furiosa-opt/upstream`을
  `https://github.com/furiosa-ai/furiosa-opt.git`, `shallow = true`로 등록한다.
- 커밋은 구현 시점의 main HEAD에 고정하고 README에 적는다. 설계 시점의
  HEAD는 `9b9cf0fdc78df00cdc430eae725a5ad9084a735e`(2026-09-11)다.
- mdBook은 git 이력을 쓰지 않으므로 얕은 클론으로 충분하다(`unshallow`
  불필요).

## 번역 설정 (`yeokja.toml`)

```toml
[[sources]]
path = "upstream/docs/src"
pattern = "**/*.md"
parser = "markdown"
output = "ko/docs/src/{path}"

[derive]
base = "upstream"

[[derive.overlay]]
path = "ko"
require_base = true
```

`base = "upstream"`으로 저장소 전체를 빌드 트리의 바탕으로 두어야
`{{#include ../../../furiosa-opt-std/...}}` 상대 경로가 조립 트리에서도 풀린다.

- `[provider]`: `type = "claude_code"`, `model = "claude-sonnet-5"`. 다른 활성
  프로젝트와 같다. 커스텀 `prompt_template`은 두지 않는다. 기본 프롬프트
  (`crates/translate/src/prompt.rs`)가 GFM 경고 표시 마커 보존 규칙을 이미
  강제하므로 원문의 `[!NOTE]` 9개, `[!TIP]` 1개, `[!WARNING]` 2개가 영어
  그대로 남는다.
- `[evaluation]`: `auto_evaluate = true`, `style_evaluate = false`,
  `max_retries = 3`.
- `[translation]`: `concurrency = 8`, `batch_segments = 32`.

번역을 시작하기 전에 `yeokja inspect upstream/docs/src`와 `yeokja coverage`로
분할을 확인한다. 특히 표(약 683행), 원시 HTML 줄(37개), 경고 표시, mermaid
블록(6개)을 본다. 식별자나 수치만 담은 표 열은 필요할 때만 `[[tables]]`로
번역에서 제외한다.

## 용어집과 표기 규칙

`glossary.toml`은 다음 원칙으로 만든다.

- **하드웨어 구성 요소와 제품 고유명사는 영문 그대로 둔다**(사용자 결정).
  번역값을 원문과 같게 두고 `note`에 이유를 적는다(fp-lean의 `Lean` 항목과
  같은 형식). 대상: Tensor Contraction Processor, TCP, RNGD, Tensor Unit,
  Fetch Engine, Commit Engine, DMA Engine, Sequencer(Fetch/Commit Sequencer),
  Contraction Engine, Vector Engine, Transpose Engine, Switch Engine,
  Cast Engine, Collect Engine, Fetch Adapter, Commit Adapter, Stream Adapter,
  Lane Folder, Packet Reducer, Time Reducer, Inter-Slice Reducer,
  Intra-Slice Chain/Reduce, VCG, Register File과 그 약어, DM, 하드웨어
  계층 이름(Chip, Cluster, Slice), 도구 이름(Kernel Optimizer,
  Schedule Viewer).
- **일반 개념은 한국어로 옮긴다.** 기본값: tensor → 텐서, kernel → 커널,
  mapping → 매핑, schedule → 스케줄, scheduling → 스케줄링, tiling → 타일링,
  packet → 패킷, stream → 스트림, axis → 축, contraction → 축약,
  reduction → 리덕션, pipeline → 파이프라인, throughput → 처리량,
  latency → 지연 시간, memory hierarchy → 메모리 계층.
  구현 때 첫 번역 배치 결과를 보고 빈도 높은 용어를 보강한다.
- 코드 식별자(`FetchTensor`, `BeginTensor::fetch()`, `m![...]`)는 용어집
  대상이 아니며 인라인 코드 그대로 둔다.
- 문체는 다른 프로젝트와 같이 `~합니다/~입니다` 격식체로 통일한다.
- 책 제목은 `Tensor Contraction Processor 프로그래밍`으로 한다. 제품 아키텍처
  이름을 영문으로 두는 규칙과 맞춘 것이다.

## 빌드 (`[build.html]`)

webassembly-component-docs와 같은 순서로 구성한다.

1. 영어 원본 빌드: `mdbook build "$YEOKJA_ROOT/upstream/docs" --dest-dir
   "$YEOKJA_ROOT/build/original"`.
2. 한국어 빌드: 조립 트리에서 `MDBOOK_BOOK__LANGUAGE=ko
   MDBOOK_BOOK__TITLE='Tensor Contraction Processor 프로그래밍' mdbook build
   docs`.
3. `preserve_anchors.py build/original docs/book`: 영어 앵커 ID를 한국어
   페이지에 덧붙인다.
4. `build_print.py`를 한국어판과 원본 양쪽에 적용해 `print.html`의 ID 체계를
   맞춘다.
5. `finish_html.py docs/book`: 본문 하단에 출처·라이선스·변경 고지를 넣는다.
   문구는 원문 © FuriosaAI · *Programming Tensor Contraction Processors* ·
   Apache 2.0 링크, yeokja 한국어 번역, "Anthropic 사의 `claude-sonnet-5`
   모델을 활용하여 번역되었으며 학습을 모두 비허용한 상태로
   작업하였습니다", 원문을 수정한 비공식 번역이라는 고지로 한다. 모델명은
   AGENTS.md 규칙대로 배포 전에 `state/`의 실제 이력으로 확인한다.
6. `check_links.py build/original docs/book`: 원문에 없던 깨진 로컬 링크나
   앵커가 있으면 빌드를 실패시킨다.
7. `docs/book`을 `site`로 옮기고 원문 `LICENSE`, `NOTICE`를 복사한다.
   `outputs = ["site"]`로 둔다.

스크립트는 저장소 관례대로 프로젝트 안에 복사해 둔다.

- `preserve_anchors.py`, `build_print.py`, `test_anchors.py`, `test_print.py`는
  rustc-dev-guide에서 가져온다.
- `check_links.py`는 webassembly-component-docs에서 가져온다.
- `finish_html.py`는 webassembly-component-docs 것을 문구만 바꿔 쓴다.

이렇게 하면 같은 스크립트 사본이 여러 벌이 되지만, 공용화는 이 작업 범위
밖의 별도 정리로 둔다.

원문 `book.toml`의 `[preprocessor.mermaid]`(`mdbook-mermaid`)와
`mathjax-support = true`는 그대로 쓴다. 분석 스크립트나 외부 링크 검사 같은
제거할 설정은 없다. `[rust] edition`은 `mdbook test`용이라 빌드에 영향이 없다.

툴체인은 `nix/projects/furiosa-opt.nix` devShell에 `mdbook`, `mdbook-mermaid`,
`python3`로 선언한다. 구현 첫 단계에서 nixpkgs의 mdbook(0.5.x)과
mdbook-mermaid 버전이 호환되는지 원본 빌드로 먼저 확인한다. 호환되지 않으면
webassembly-component-docs의 mdbook-tabs처럼 `buildRustPackage`로 맞는 버전을
고정한다.

## 배포 배선

- `.github/pages-projects.json`: `{"project": "furiosa-opt", "target": "html",
  "artifact": "dist-furiosa-opt", "artifact_path": "projects/furiosa-opt/dist"}`.
- `.github/scripts/stage-pages.sh`: `overlay_site "dist-furiosa-opt" "site"
  "furiosa-opt"`.
- `.github/scripts/rebuild-translations.sh`: rustc-dev-guide와 같은 형태로
  `furiosa-opt)` 분기를 둔다. `translate upstream/docs/src` 뒤에 `status --check
  upstream/docs/src`를 실행해, upstream이 새로 추가한 문서가 state 없이 빠지는
  경우도 배포를 막는다.
- `.github/workflows/pr.yml`: rustc-dev-guide처럼
  `projects/furiosa-opt/scripts`의 `test_*.py` 단위 테스트 스텝을 추가한다.
- `site/index.html`: 기존 `hardware` 주제 섹션에 항목을 추가한다. 제목은
  `Tensor Contraction Processor 프로그래밍`, 원문 링크는
  `furiosa-ai/furiosa-opt`, 태그는 `Apache 2.0`으로 한다. 주제 태그 형식은
  기존 항목을 따른다.

## README

`projects/furiosa-opt/README.md`에는 다음을 적는다.

- 번역 범위(`docs/` 책만)와 고정 커밋.
- 비공식 번역이라는 고지와 Apache-2.0 라이선스 보존 방식(`LICENSE`,
  `NOTICE`, 변경 고지).
- 재현 명령: `translate`, `status --check`, nix devShell 안에서 `build html`.
- `state/`는 커밋 대상이고 `ko/`·`build/`·`dist/`는 재생성 가능한 산출물이라는
  점.
- AGENTS.md의 출처 표기 규칙: [yeokja](https://github.com/yeokja/yeokja) 링크,
  실제 번역 모델, "Anthropic 사의 `claude-sonnet-5` 모델을 활용하여
  번역되었으며 학습을 모두 비허용한 상태로 작업하였습니다." 모델명은 커밋 전에
  `state/**/*.yeokja.json` 이력으로 확인한다.

## 검증

- `yeokja status --check upstream/docs/src`: 미번역 세그먼트 0.
- `yeokja evaluate --mechanical-only`: 서식·링크·용어집 기계 검사 통과.
- GFM 경고 표시: 원문과 번역문의 `[!NOTE]`/`[!TIP]`/`[!WARNING]` 개수가
  파일별로 같다.
- 수식: 원문과 `ko/`의 `\\(`, `\\)`, `$$` 개수가 파일별로 같다. 모델이
  `\\(`를 `\(`로 바꾸면 MathJax 렌더링이 깨지므로 따로 센다.
- nix devShell에서 `yeokja build html`이 성공하고 `check_links.py`가
  `New: 0`을 낸다.
- `scripts/`의 단위 테스트가 통과한다.
- 표본 확인: `moving-tensors/fetch-engine.html`의 `#constraints`,
  `#axis-lifting` 링크가 해당 제목으로 이동하고, mermaid 다이어그램과 수식이
  렌더링된다.
- `.github/scripts/rebuild-translations.sh furiosa-opt`로 CI 재구성 경로를
  로컬에서 재현한다.

## 위험과 대응

- **mdbook-mermaid 호환성**: 빌드 첫 단계에서 확인한다. 안 맞으면 버전을
  고정한다.
- **upstream이 활발히 개발 중**: 이 번역은 고정 커밋 기준이다. 갱신은
  서브모듈을 올린 뒤 yeokja 증분 번역으로 처리하며, `rebuild_daily`는 쓰지
  않는다.
- **표 안의 식별자**: 번역이 표 구조를 흔들면 `[[tables]]`로 열을 제외한다.
