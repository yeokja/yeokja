# Write You a Haskell 한국어 번역 설계

이슈: yeokja/yeokja#7

## 목표

[`sdiehl/write-you-a-haskell`](https://github.com/sdiehl/write-you-a-haskell)의
mdBook 책 *Write You a Haskell*(`book/`)을 고정된 원문으로 사용하고, yeokja로
독자에게 보이는 Markdown 전체를 한국어로 번역해 GitHub Pages에 배포한다.

완료 상태는 다음 조건을 모두 만족해야 한다.

- `sdiehl/write-you-a-haskell`의 고정 커밋이 읽기 전용 서브모듈로 등록되어
  있다.
- `upstream/book/src/**/*.md`(31개 파일, `SUMMARY.md` 포함, 4,160 세그먼트,
  약 45,000 단어)의 모든 대상 세그먼트가 번역 상태이고
  `yeokja status --check upstream/book/src`가 통과한다.
- 한국어 mdBook HTML 빌드가 nix devShell 안에서 성공하고, mdbook 로그에
  warn/error가 없으며, 원문 빌드에 없던 깨진 로컬 링크·앵커가 하나도
  없다(`check_links.py`가 `New: 0`).
- 수식 50개 블록과 인라인 `$` 549개, `{{#include}}` 76개, `[@인용]` 135개가
  번역문에 파일별로 같은 수로 남아 있다(`check_markers.py`가 0 mismatch).
- 원문의 영어 제목 앵커가 한국어 페이지에서도 동작한다.
- Pages 배포 배선, 랜딩 페이지 항목, README 출처 표기가 갖춰져 있다.

## 조사 결과

- 원문은 2026년에 전면 개정된 판이다. Rust로 구현한 컴파일러 스냅샷
  (`crates/ch02_calc` … `crates/ch24_gc`)과 mdBook 책(`book/book.toml`,
  `book/src/`, `book/theme/`, `book/refs.bib`)으로 구성되며, 책은 25개 장과
  부록 5개다. 기본 브랜치 `main`, 설계 시점 HEAD는
  `4333ed8c95f016470ca30c950e1744873f004bab`(2026-10-13).
- 책 툴체인(`BUILDING.md`, `scripts/bootstrap.py`): mdBook 0.5.3, mdbook-katex
  0.10.0, mdbook-bib 0.5.3(저자 패치: 저자 없이 편집자만 있는 항목을 편집자로
  렌더링), 저자의 `mdbook-include-rs` 포크(커밋
  `32e5190a5418367059851a2ce5055dceab79b510`), pandoc 3.12.1, Typst 0.15.1.
  `just book`은 `cargo xtask syntax --check`와 `scripts/check_book.py`를 돌린
  뒤 `mdbook build book`을 실행하고 로그에 warn/error가 있으면 실패한다.
- `book.toml`의 전처리기 체인: `include-rs`(`python3 ../scripts/include-rs.py`가
  mdBook 0.4 API의 포크를 0.5 JSON 봉투로 감싸서 호출), `fun-spacing`,
  `rust-dedent`(둘 다 코드 블록만 손봄), `katex`, `bib`(`citation-syntax =
  "pandoc"`, CSL chicago-author-date), `bibliography`(`scripts/bibliography.py`가
  참고문헌 페이지를 `pandoc --citeproc`로 다시 렌더링 — **HTML 빌드도 pandoc이
  필요**하다). PDF 빌드(pandoc→typst)는 별도 레시피이며 범위 밖이다.
- 책 본문에 쓰인 문법: `$...$`·`$$...$$` 수식(인라인 `$` 549개, 디스플레이
  블록 50개), `{{#include ../../crates/<ch>/src/<file>.rs:<anchor>}}` 76개(모두
  제 줄에 홀로 있으며 `crates/` 아래 Rust 소스를 가리킴), `[@key]` 인용
  135개(23개 참고문헌 항목), 코드 블록 404개(`rust` 104, `fun` 43, `text` 49,
  `sh` 6, 언어 없음 202), 표 245행(주로 `appendix-e-diagnostics.md`의 진단 코드
  표와 ch06의 표기법 표), 그림 1개(`img/pipeline.svg`), GFM 경고 표시 0개,
  원시 HTML 0줄(자동 링크 `<https://...>` 2줄뿐), 제목 369개, 같은 책 안의
  앵커 링크 1개.
- `yeokja coverage upstream/book/src`: 본문 5,119줄 중 84%가 번역 대상이고,
  파서가 건너뛴 긴 구간은 모두 코드 블록이다. `yeokja inspect`는 선택 규칙이
  필요한 표를 찾지 못했다(진단 코드 표의 첫 열은 인라인 코드라 번역되지
  않는다).
- `markdown` 파서(`parser-markdown-dialect`)는 pulldown-cmark의 `ENABLE_MATH`를
  켜지 않으므로 `$...$`는 본문 텍스트의 일부로 세그먼트에 들어가고, 제 줄에
  홀로 있는 `$$` 블록과 `{{#include}}` 줄은 그 자체가 한 문단 세그먼트가 된다.
  보존은 프롬프트 규칙과 사후 검사에 맡겨야 한다.
- 기본 프롬프트(`crates/translate/src/prompt.rs`)의 `Markup::Markdown` 규칙에는
  수식 보존 규칙이 없다(`Markup::MkDocs` 규칙에만 있다).
- 라이선스: 산문과 원본 그림은 CC BY-SA 4.0(`LICENSE-PROSE`), 코드는
  MIT(`LICENSE`).
- nixpkgs(`nix/flake.lock`의 nixpkgs-unstable 핀)에는 mdbook 0.5.4,
  mdbook-katex 0.10.0, mdbook-bib 0.5.3, pandoc 3.7.0.2가 있고
  mdbook-include-rs는 없다.

## 범위

`book/src/**/*.md` 전체를 번역한다. `crates/`의 Rust 소스(`{{#include}}`로 책에
끼워 넣어지는 코드)와 그 주석, `book/refs.bib`, `book/theme/`, `README.md`,
`BUILDING.md`, `CHANGELOG.md` 등 책 밖 문서는 번역하지 않는다. mdbook-bib가
생성하는 참고문헌 페이지(`bibliography.md`)는 원문 소스에 없는 생성물이므로
영어 그대로 둔다.

코드 블록, `{{#include}}` 지시자, 인라인 코드, URL, 수식, 인용 키는 원문을
보존한다. 책의 언어 이름 `Fun`, IR 이름 `Core`/`ANF`/`Flat`, 모듈 이름
`Prelude`, 도구 이름(LLVM, JIT, REPL, Boehm GC)은 영문으로 둔다.

원문 산문은 CC BY-SA 4.0이다. 번역문은 원문을 개작한 2차적 저작물로서 같은
CC BY-SA 4.0으로 배포하며, 다음을 지킨다(§3(a), §3(b)).

- 사이트 루트에 원문 `LICENSE-PROSE`(CC BY-SA 4.0)와 `LICENSE`(MIT)를 함께
  싣는다.
- 모든 본문 페이지 하단에 저작자(Stephen Diehl), 원문 링크, 라이선스 링크,
  "원문을 수정한 비공식 번역본"이라는 변경 고지를 넣는다.
- 책에 끼워 넣어진 코드는 MIT이며 번역하지 않으므로 라이선스가 바뀌지 않는다.

## 접근 방식

### 채택: rustc-dev-guide / furiosa-opt의 mdBook 오버레이

원문 서브모듈 **저장소 전체**를 조립 트리의 바탕(`base = "upstream"`)으로 두고
`ko/book/src/` 번역 미러를 겹친다. `{{#include ../../crates/...}}`가 책 밖의
Rust 소스를 가리키고 `book.toml`의 전처리기가 `../scripts/*.py`를 부르므로
`book/`만 바탕으로 두면 빌드가 되지 않는다. 영어 원본과 한국어판을 모두 빌드한
뒤 렌더링된 원본 HTML을 기준으로 영어 제목 앵커를 한국어 페이지에 덧붙이고,
인쇄 페이지를 다시 조립하며, 하단 고지를 넣고, 깨진 링크를 검사한다. 필요한
스크립트는 furiosa-opt에 같은 사본이 있고 새 제품 기능이 필요 없다.

### 제외: `inline_tags = true`

체크리스트 2절은 `$` 수식이 있는 `markdown` 소스에 인라인 태그 전송을 켜지
말라고 한다. 이 책은 mdbook-katex로 `$...$`를 렌더링하므로 끄고, 대신 수식
보존을 프롬프트 규칙과 `check_markers.py`로 지킨다. 조사 앞 강조 처리 같은
태그 모드의 이점은 프롬프트의 Markdown 닫는 규칙(기본 프롬프트와 같은 문장)으로
대신한다.

### 제외: 파서에 수식 마스킹 추가

`parser-mkdocs`처럼 수식을 가린 그림자 사본을 쓰는 `markdown` 변형을 만들면
구조적으로 안전하지만 crates 변경이 필요하다. 이 책의 수식은 대부분 짧은
인라인 기호(`$\tau$`, `$\Gamma$`)이고 디스플레이 블록은 세그먼트 전체가
수식이라 모델이 그대로 돌려주면 되므로, 사후 검사로 충분하다고 본다. 검사가
실패하는 세그먼트가 많으면 그때 파서 변경을 별도 작업으로 검토한다.

## 디렉터리와 책임

```text
projects/write-you-a-haskell/
├── upstream/       sdiehl/write-you-a-haskell 원문 서브모듈(읽기 전용, shallow)
├── state/          번역 상태(*.yeokja.json, 진실의 원천, 커밋 대상)
├── ko/             state에서 재생성되는 book/src 미러(커밋하지 않음)
├── scripts/        앵커 보존·인쇄 페이지·링크 검사·하단 고지·마커 검사 스크립트와 테스트
├── build/          원본 빌드 결과와 mdbook 로그(커밋하지 않음)
├── dist/site/      완성된 한국어 HTML(커밋하지 않음)
├── glossary.toml   용어집
├── terms.toml      `yeokja terms build`가 만드는 프로젝트 용어 기준
├── yeokja.toml     번역·조립·빌드 설정
└── README.md       범위, 라이선스, 재현 명령, 출처 표기
```

`.gitignore`에 `ko/`, `build/`, `dist/`, `__pycache__/`를 둔다. `upstream/`은
수정하지 않는다.

## 원문 고정

- `.gitmodules`에 `projects/write-you-a-haskell/upstream`을
  `https://github.com/sdiehl/write-you-a-haskell.git`, `shallow = true`로
  등록한다.
- 커밋은 `main`의 `4333ed8c95f016470ca30c950e1744873f004bab`(2026-10-13)에
  고정하고 README에 적는다.
- mdBook과 전처리기는 git 이력을 쓰지 않으므로 얕은 클론으로 충분하다
  (`unshallow` 불필요). `include-rs`가 `crates/`를 읽으므로 서브모듈은 책
  디렉터리만이 아니라 저장소 전체가 필요하다(sparse checkout 없음).

## 번역 설정 (`yeokja.toml`)

```toml
[[sources]]
path = "upstream/book/src"
pattern = "**/*.md"
parser = "markdown"
inline_tags = false
output = "ko/book/src/{path}"

[derive]
base = "upstream"

[[derive.overlay]]
path = "ko"
require_base = true
```

- `[provider]`: `type = "claude_code"`, `model = "claude-opus-5-5"`. 커스텀
  `prompt_template`을 둔다. 기본 프롬프트의 규칙(마크업 보존, 인라인 코드
  바이트 보존, GFM 경고 표시 마커 보존, Markdown 닫는 규칙, 합쇼체)을 그대로
  옮기고 다음을 더한다: `$...$`/`$$...$$` 바이트 보존과 조사 위치, 수식만 있는
  세그먼트는 그대로 반환, 수식 추가 금지, `{{#include}}`와 `[@key]` 보존,
  고유명사 유지, "Chapter N:" → "N장:", "Appendix X:" → "부록 X:", 1인칭
  문체(저/우리) 유지. AGENTS.md대로 커스텀 템플릿에는 GFM 마커 규칙을 직접
  명시한다(이 책에는 경고 표시가 없지만 안전망으로 둔다). `{inline_tags}`
  자리표시자를 두어 나중에 태그 모드를 켤 수 있게 한다.
- `[evaluation]`: `auto_evaluate = true`, `style_evaluate = false`,
  `max_retries = 3`.
- `[translation]`: `concurrency = 8`, `batch_segments = 32`.

## 용어집과 표기 규칙

`glossary.toml`은 다음 원칙으로 만든다.

- **일반 개념은 한국어 관례 번역어로 옮긴다.** lambda calculus → 람다 대수,
  type inference → 타입 추론, unification → 단일화, substitution → 치환,
  generalization → 일반화, closure conversion → 클로저 변환, A-normal form →
  A-정규형, join point → 조인 포인트, pattern matching → 패턴 매칭,
  type class → 타입 클래스, kind checking → 카인드 검사, renamer → 리네이머,
  desugaring → 디슈거링, garbage collector → 가비지 컬렉터, copying collector →
  복사 컬렉터, dictionary passing → 사전 전달, occurs check → 발생 검사,
  typing rule → 타입 규칙, typing judgment → 타입 판단, thunk → 썽크 등.
- **고유명사는 영문 그대로 둔다**: Fun, Haskell, Prelude, LLVM, Pratt(파서),
  Boehm, Inkwell. 사람 이름이 붙은 알고리즘 이름은 한글 표기로 굳어진 관례를
  따른다(Hindley–Milner → 힌들리–밀너, De Bruijn → 드 브루인).
- **일상어와 겹치는 짧은 단어**(kind, record, root, environment, layout)는
  용어집에 두지 않는다. 용어집은 세그먼트 안의 어구 일치로 적용·검증되므로
  "kind of"나 동사 "record"에 오탐을 낸다. 이런 용어는 프롬프트와
  `terms.toml`(첫 번역 뒤 `yeokja terms build`)에 맡긴다.
- 영문 병기는 번역어 자체에 넣지 않고, 모델이 장에서 처음 나올 때 괄호로
  덧붙이는 것을 허용한다.
- 문체는 다른 프로젝트와 같이 `~합니다/~입니다` 격식체로 통일하되, 저자의
  1인칭 서술은 저/우리로 살린다.
- 책 제목은 `Write You a Haskell`을 그대로 쓰고 사이트 제목은
  `Write You a Haskell 한국어판`으로 한다. 원제가 *Write Yourself a Scheme*을
  잇는 말장난이라 번역하면 뜻이 사라진다.

## 빌드 (`[build.html]`)

furiosa-opt와 같은 순서로 구성한다. 조립 트리의 cwd는 저장소 전체이므로 책은
`book/`이다.

1. `check_markers.py upstream/book/src book/src`: 수식 `$` 개수,
   `{{#include}}` 집합, `[@key]` 집합, 코드 펜스 수가 파일별로 원문과 같은지
   확인한다. 틀리면 mdBook이 경고 없이 잘못 렌더링하므로 빌드 전에 막는다.
2. 영어 원본 빌드: `mdbook build upstream/book --dest-dir build/original`
   (stderr를 `build/mdbook-original.log`로).
3. 한국어 빌드: `MDBOOK_BOOK__LANGUAGE=ko MDBOOK_BOOK__TITLE='Write You a
   Haskell 한국어판' mdbook build book`(stderr를 `build/mdbook-ko.log`로).
   `just book`처럼 두 로그에 warn/error가 있으면 실패한다.
4. `preserve_anchors.py build/original/html book/build/html`: 영어 제목 앵커
   ID를 한국어 페이지에 덧붙인다.
5. `build_print.py`를 한국어판과 원본 양쪽에 적용한다.
6. 원문 `LICENSE`, `LICENSE-PROSE`를 사이트 루트에 복사한다.
7. `finish_html.py book/build/html`: 본문 하단에 출처·라이선스·변경 고지를
   넣는다. 문구는 원문 © Stephen Diehl · *Write You a Haskell* · 산문 CC BY-SA
   4.0 · 코드 MIT, yeokja 한국어 번역, "Anthropic 사의 `claude-opus-5-5` 모델을
   활용하여 번역되었으며 학습을 모두 비허용한 상태로 작업하였습니다", 원문을
   수정한 비공식 번역이라는 고지로 한다.
8. `check_links.py build/original/html book/build/html`: 원문에 없던 깨진
   로컬 링크나 앵커가 있으면 실패한다.
9. `book/build/html`을 `site`로 옮긴다. `outputs = ["site"]`.

`cargo xtask syntax --check`와 `scripts/check_book.py`(인용 키·앵커 존재
확인, 전처리기 자체 검사, `error-index.py --check`)는 원문 저장소 자체의
검사이며 번역이 바꾸지 않는 입력(`crates/`, `refs.bib`)을 보므로 빌드
명령에 넣지 않는다. 번역이 바꿀 수 있는 부분(인용 키, include 경로)은
`check_markers.py`가 같은 기준으로 확인한다.

mdbook-katex는 `[output.html]`에 KaTeX CSS/JS를 자동으로 넣고, 한국어 빌드도
같은 `book.toml`을 쓰므로 추가 설정이 없다.

### 툴체인 (`nix/projects/write-you-a-haskell.nix`)

| 도구 | 원문 고정(`bootstrap.py`) | devShell | 비고 |
|---|---|---|---|
| mdbook | 0.5.3 | nixpkgs 0.5.4 | 다른 mdBook 프로젝트와 같음 |
| mdbook-katex | 0.10.0 | nixpkgs 0.10.0 | 동일 |
| mdbook-bib | 0.5.3 + 저자 패치 | nixpkgs 0.5.3 `overrideAttrs`로 같은 한 줄 패치 | 패치 없이는 편집자만 있는 `haskell2010` 인용이 비어 렌더링됨 |
| mdbook-include-rs | 저자 포크 `32e5190a` | `buildRustPackage`로 같은 커밋 빌드(`cargoHash` 고정) | nixpkgs에 없음; 네트워크 없이 재현 가능 |
| pandoc | 3.12.1 | nixpkgs 3.7.0.2 | 참고문헌 페이지 citeproc 렌더링에만 쓰임 |
| python3 | 시스템 | nixpkgs | 전처리기와 빌드 스크립트, 표준 라이브러리만 사용 |

`cargo install --git`으로 빌드 시점에 내려받는 대안은 네트워크 없이 재현되지
않고 CI 캐시에 맞지 않아 택하지 않는다. mdbook-bib는 저자의 `Cargo.lock`
(`scripts/locks/mdbook-bib.lock`)을 쓰지 않고 nixpkgs의 잠금을 그대로 쓴다 —
저자 잠금은 mdbook-core 0.5.3을 맞추기 위한 것인데 mdBook 0.5.x 전처리기
프로토콜은 patch 버전 사이에 호환되며, nixpkgs 0.5.3이 mdbook 0.5.4와 함께
원본을 경고 없이 빌드하는지는 구현 첫 단계에서 확인한다.

## 배포 배선

- `.github/pages-projects.json`: `{"project": "write-you-a-haskell", "target":
  "html", "artifact": "dist-write-you-a-haskell", "artifact_path":
  "projects/write-you-a-haskell/dist"}`.
- `.github/scripts/stage-pages.sh`: 마지막 overlay 호출 뒤에
  `overlay_site "dist-write-you-a-haskell" "site" "write-you-a-haskell"`.
- `.github/scripts/rebuild-translations.sh`: `write-you-a-haskell)` 분기에서
  `translate upstream/book/src` 뒤 `status --check upstream/book/src`를
  실행해, upstream이 새로 추가한 장이 state 없이 빠지는 경우도 배포를 막는다.
- `.github/workflows/pr.yml`: `projects/write-you-a-haskell/scripts`의
  `test_*.py` 단위 테스트 스텝을 추가한다.
- `LICENSE.md` 번역 프로젝트 표: `write-you-a-haskell` | 산문 CC BY-SA 4.0,
  코드 MIT(번역하지 않음).
- `site/index.html`: 필터 버튼 목록의 `algorithms` 버튼 뒤에
  `data-topic="haskell"`("Haskell") 버튼을 두고, 마지막 주제 섹션 뒤에
  `haskell` 섹션과 `<li class="work" data-topics="haskell">` 항목을 둔다.
  제목 "Write You a Haskell", 원문 링크 `sdiehl/write-you-a-haskell`, 태그
  "CC BY-SA 4.0"·"기계 번역".

## README

`projects/write-you-a-haskell/README.md`에는 다음을 적는다(furiosa-opt 형태).

- 번역 범위(`book/src/**/*.md`)와 고정 커밋, 번역하지 않는 것(`crates/`
  코드, 참고문헌 페이지).
- 비공식 번역이라는 고지와 CC BY-SA 4.0 보존 방식(저작자 표기, 라이선스 파일,
  변경 고지), 코드는 MIT.
- 재현 명령: `translate`, `status --check`, nix devShell 안에서 `build html`.
- `state/`는 커밋 대상이고 `ko/`·`build/`·`dist/`는 재생성 가능한 산출물이라는
  점.
- AGENTS.md의 출처 표기 규칙: [yeokja](https://github.com/yeokja/yeokja) 링크,
  실제 번역 모델, "Anthropic 사의 `claude-opus-5-5` 모델을 활용하여
  번역되었으며 학습을 모두 비허용한 상태로 작업하였습니다." 모델명은
  `state/**/*.yeokja.json` 이력으로 확인한다.

## 검증

- `yeokja status --check upstream/book/src`: 미번역 세그먼트 0.
- `yeokja evaluate --mechanical-only upstream/book/src`: Alignment/Format/Link
  항목은 체크리스트 6절대로 하나씩 확인해 실제 문제는 state에서 지우고 다시
  번역한다.
- `yeokja terms build`(`--dry-run` 먼저) → `terms.toml` 검토 →
  `glossary adopt --prefer HEAD` → 남은 세그먼트 재번역.
- `check_markers.py`: 파일별 `$` 개수, include·인용 집합, 펜스 수 일치.
- nix devShell에서 `yeokja build html`이 성공하고 mdbook 로그에 warn/error가
  없으며 `check_links.py`가 `New: 0`을 낸다.
- HTML 표본 확인: `ch06-hindley-milner.html`의 타입 규칙이 KaTeX
  (`<span class="katex">`)로 렌더링되고, `ch02-parsing.html`에
  `{{#include}}`로 끼워 넣은 Rust 코드가 원문과 같은 줄 수로 들어가 있으며,
  `[@pierce2002]` 인용이 `bibliography.html#pierce2002` 링크로 렌더링된다.
- `scripts/`의 단위 테스트가 통과한다.
- `.github/scripts/rebuild-translations.sh write-you-a-haskell`로 CI 재구성
  경로를 로컬에서 재현한다.
- 번역 품질: 서론·람다 대수·힌들리-밀너·클로저 변환 장을 직접 읽어 용어와
  문체를 점검한다.

## 위험과 대응

- **수식·include·인용 손상**: 모델이 `$`를 빠뜨리거나 `[@key]`를 옮겨 쓰면
  mdBook은 경고 없이 잘못 렌더링한다. `check_markers.py`가 빌드 전에 막고,
  실패한 파일은 해당 세그먼트를 state에서 지우고 다시 번역한다.
- **mdbook-bib 0.5.3과 mdbook 0.5.4 호환**: 원본 빌드 로그로 확인한다.
  경고가 나면 저자 잠금(`scripts/locks/mdbook-bib.lock`)으로 `cargoLock`을
  바꾼다.
- **include-rs 포크의 Rust edition 2024**: nixpkgs의 rustc(1.97)로 빌드된다.
  nixpkgs 핀을 내릴 일이 생기면 1.85 이상이어야 한다.
- **upstream이 활발히 개정 중**: 이 번역은 고정 커밋 기준이다. 갱신은
  서브모듈을 올린 뒤 yeokja 증분 번역으로 처리하며, `rebuild_daily`는 쓰지
  않는다.
- **용어집 오탐**: 일상어와 겹치는 용어를 뺐으므로 남은 항목이 `glossary
  stale`을 과하게 내지는 않지만, `terms build` 뒤 `terms.toml`의 잘못된
  항목은 `except`로 다스린다.
