# Rust API 가이드라인 한국어 번역 설계

이슈: yeokja/yeokja#6

## 목표

[`rust-lang/api-guidelines`](https://github.com/rust-lang/api-guidelines)
(<https://rust-lang.github.io/api-guidelines/>)의 mdBook 책 *Rust API
Guidelines*를 고정된 원문으로 삼아 yeokja로 전체를 한국어로 번역하고, 영어
앵커가 살아 있는 HTML을 GitHub Pages에 배포한다.

완료 상태는 다음 조건을 모두 만족해야 한다.

- 원문의 고정 커밋이 읽기 전용 서브모듈로 등록되어 있다.
- `upstream/src/**/*.md`(15개 파일, `SUMMARY.md` 포함)의 모든 대상 세그먼트가
  번역 상태이고 `yeokja status --check`가 통과한다.
- 한국어 mdBook HTML 빌드가 성공하고, 원문 빌드에 없던 깨진 로컬 링크·앵커가
  하나도 없다(`check_links.py`가 `New: 0`).
- 가이드라인 식별자 앵커(`#c-case` 등 54개)와 원문의 영어 제목 앵커가 한국어
  페이지에서도 그대로 동작한다.
- Pages 배포 배선과 랜딩 페이지 항목, `LICENSE.md` 표, README 출처 표기가
  갖춰져 있다.

## 조사 결과

- **원문:** 기본 브랜치 `master`, 커밋
  `97a0969cb07fe4cabb0eed8a56234053f47d83dc`(2025-07-08). `book.toml`은
  `title`/`description`만 있고 전처리기·수식·리다이렉트 설정이 없다.
- **분량:** `src/` 아래 Markdown 15개(약 92KB, 약 1.2만 단어), 세그먼트 840개.
  `yeokja coverage` 기준 텍스트 1,722줄 중 63%가 번역 대상이며, 파서가 건너뛴
  5줄 이상 구간은 모두 코드 블록(71개)과 링크 참조 정의 묶음이다.
- **구조:** 제목 117개. 각 가이드라인은 `<a id="c-xxx"></a>` HTML 블록(54개)
  뒤에 `## 제목 (C-XXX)` 형식의 제목이 따른다. `checklist.md`의 항목과 본문
  상호 참조는 `[C-XXX]: naming.html#c-xxx` 같은 링크 참조 정의(58개)로 이
  HTML 앵커를 가리키므로, 제목이 번역되어도 주된 앵커는 깨지지 않는다. 같은
  페이지 안의 `#c-xxx` 참조 4개도 같다.
- **표:** `naming.md`에만 GFM 표 3개(23행)가 있다. 셀은 `UpperCamelCase`,
  `snake_case` 같은 코드 스팬과 짧은 산문이며 식별자만 담은 열이 없어
  `[[tables]]` 제외 규칙이 필요 없다. `<br>` 2개가 셀 안에 있다.
- **수식·경고 표시:** `$`는 매크로 예제 코드 블록과 셸 명령 한 줄에만 있어
  수식이 아니다. GFM 경고 표시 마커는 없다. HTML 주석 1개(`checklist.md`)는
  파서가 그대로 보존한다.
- **리다이렉트:** `[output.html.redirect]`가 없어 리다이렉트 스텁이 생기지
  않는다. rustc-dev-guide의 `relativize_redirects.py`는 필요 없다.
- **툴체인:** 원문 CI는 mdbook 버전을 고정하지 않는다. nixpkgs의 mdbook 0.5.4로
  원문 빌드가 성공한다.
- **라이선스:** MIT OR Apache-2.0(`LICENSE-MIT`, `LICENSE-APACHE`).
- **파일럿:** `naming.md` 165개 세그먼트를 번역한 결과, 표·앵커·링크 참조
  정의·`<br>`가 모두 보존되었고 링크 검사 재시도 1회로 통과했다. 용어집에
  `lifetime`이 있어도 표의 복수형 `Lifetimes`는 "수명"으로 번역되어, 용어
  기준(`terms.toml`) 단계가 필요하다.

## 범위

`upstream/src/**/*.md` 전체를 번역한다. 저장소 루트의 `README.md`,
`CONTRIBUTING.md`는 책 사이트에 나오지 않으므로 제외한다.

코드 블록(Rust 예제, `Cargo.toml`, 셸 명령), 인라인 코드, URL, 링크 참조
정의, `<a id>` 앵커, HTML 주석은 원문을 보존한다. 가이드라인 식별자
(`C-CASE`, `C-CONV` 등)는 제목과 체크리스트에서 그대로 둔다.

원문은 MIT OR Apache-2.0이다. 번역문은 같은 이중 라이선스로 배포하며, 사이트
루트에 원문 `LICENSE-MIT`, `LICENSE-APACHE`를 함께 싣는다(rustc-dev-guide와
같은 방식).

## 접근 방식

### 채택: rustc-dev-guide의 mdBook 오버레이

원문 서브모듈 위에 `ko/` 번역 미러를 겹쳐 빌드 트리를 조립하고, 영어 원본과
한국어판을 모두 빌드한 뒤 렌더링된 원본 HTML을 기준으로 영어 제목 앵커를
한국어 페이지에 덧붙인다. 필요한 스크립트는 rustc-dev-guide와 rust-forge에
같은 사본으로 있으며 새 제품 기능이 필요 없다.

이 책은 주된 앵커가 `<a id="c-xxx">`라서 제목 번역만으로는 상호 참조가
깨지지 않지만, 외부에서 `naming.html#casing-conforms-to-rfc-430-c-case` 같은
mdBook 제목 ID로 들어오는 링크를 살리기 위해 `preserve_anchors.py`를
그대로 쓴다. `print.html`은 `build_print.py`로 장마다 ID 접두사를 붙여
`#c-xxx` 앵커가 장 사이에서 충돌하지 않게 한다.

### 제외: 단일 `mdbook build`

제목 ID만 바뀌는 작은 책이라 단일 빌드로도 상호 참조는 동작하지만,
`check_links.py`가 원문 빌드와 비교해야 "원문에 없던" 깨진 링크를 가려낼 수
있다. 원본 빌드는 수 초면 끝나므로 두 번 빌드하는 비용이 작다.

## 디렉터리와 책임

```text
projects/rust-api-guidelines/
├── upstream/       rust-lang/api-guidelines 원문 서브모듈(읽기 전용, shallow)
├── state/          번역 상태(*.yeokja.json, 진실의 원천, 커밋 대상)
├── ko/             state에서 재생성되는 src 미러(커밋하지 않음)
├── scripts/        앵커 보존·인쇄 페이지·링크 검사 스크립트와 테스트
├── build/          원본 빌드와 조립 트리(커밋하지 않음)
├── dist/site/      완성된 한국어 HTML(커밋하지 않음)
├── glossary.toml   용어집
├── terms.toml      프로젝트 용어 기준(`yeokja terms build` 결과)
├── yeokja.toml     번역·조립·빌드 설정
└── README.md       범위, 라이선스, 재현 명령, 출처 표기
```

`.gitignore`에 `ko/`, `dist/`, `__pycache__/`를 둔다(`build/`는 저장소 루트
`.gitignore`가 제외한다). `upstream/`은 수정하지 않는다.

## 원문 고정

- `.gitmodules`에 `projects/rust-api-guidelines/upstream`을
  `https://github.com/rust-lang/api-guidelines.git`, `shallow = true`로
  등록하고 `97a0969cb07fe4cabb0eed8a56234053f47d83dc`에 고정한다.
- mdBook은 git 이력을 쓰지 않으므로 얕은 클론으로 충분하다(`unshallow`
  불필요).

## 번역 설정 (`yeokja.toml`)

```toml
[[sources]]
path = "upstream/src"
pattern = "**/*.md"
parser = "markdown"
inline_tags = true
output = "ko/src/{path}"

[derive]
base = "upstream"

[[derive.overlay]]
path = "ko"
require_base = true
```

- `inline_tags = true`: mdBook은 `$` 수식을 쓰지 않는 CommonMark 렌더러이고
  `book.toml`에 `mathjax-support`가 없으므로 켤 수 있다. 조사 앞 강조나 코드
  스팬 변경이 구조적으로 생기지 않는다.
- `[provider]`: `type = "claude_code"`, `model = "claude-opus-5-5"`. 다른 활성
  프로젝트와 같다. 커스텀 `prompt_template`은 두지 않는다.
- `[evaluation]`: `auto_evaluate = true`, `style_evaluate = false`,
  `max_retries = 3`.
- `[translation]`: `concurrency = 8`, `batch_segments = 32`.
- `[[tables]]`는 두지 않는다(조사 결과 참고).

## 용어집과 표기 규칙

`glossary.toml`은 rustc-dev-guide·rust-forge 용어집과 번역어를 맞춘다.

- **공통 번역어:** crate → 크레이트, trait → 트레이트, lifetime → 라이프타임,
  borrow → 빌림, standard library → 표준 라이브러리, closure → 클로저,
  generic → 제네릭, type parameter → 타입 매개변수, struct → 구조체,
  enum variant → 열거형 배리언트, attribute → 속성, constructor → 생성자,
  destructor → 소멸자, panic → 패닉, wrapper → 래퍼, feature → 기능,
  breaking change → 호환성을 깨는 변경, release notes → 릴리스 노트.
- **이 책에서 새로 정하는 번역어:** ownership → 소유권, builder → 빌더,
  getter → 게터, setter → 세터, iterator → 이터레이터, associated type →
  연관 타입, trait bound → 트레이트 경계, sealed trait → 봉인된 트레이트,
  smart pointer → 스마트 포인터, conversion → 변환, guideline → 가이드라인,
  checklist → 체크리스트.
- **원문 그대로:** Rust, Cargo, crates.io, rustdoc, Serde, Clippy, RFC 같은
  이름과 Rust 관용구 이름 newtype, unsafe. 번역값을 원문과 같게 두고 `note`에
  이유를 적는다.
- 코드 식별자(`Deref`, `as_`, `into_iter`, `FromIterator`)와 가이드라인
  식별자(`C-NEWTYPE`)는 용어집 대상이 아니다.
- 문체는 다른 프로젝트와 같이 `~합니다/~입니다` 격식체로 통일한다. 제목의
  가이드라인 문장("Casing conforms to RFC 430")도 "대소문자 표기는 RFC 430을
  따릅니다"처럼 서술형으로 옮기고 `(C-CASE)` 식별자를 그대로 남긴다.
- 책 제목은 `Rust API 가이드라인`으로 한다.

첫 번역 뒤 `yeokja terms build`로 `terms.toml`을 만들어 반복 용어와 제목의
번역어를 고정하고, `glossary adopt --prefer HEAD` 후 남은 세그먼트만 다시
번역한다(체크리스트 3절).

## 빌드 (`[build.html]`)

rustc-dev-guide와 같은 순서로 구성한다. 책 루트가 저장소 루트라서 조립
트리의 `book/`에 결과가 나온다.

1. 영어 원본 빌드: `mdbook build "$YEOKJA_ROOT/upstream" --dest-dir
   "$YEOKJA_ROOT/build/original"`.
2. 한국어 빌드: 조립 트리에서 `MDBOOK_BOOK__LANGUAGE=ko
   MDBOOK_BOOK__TITLE='Rust API 가이드라인' mdbook build`.
3. `preserve_anchors.py build/original book`: 영어 제목 ID를 한국어 페이지에
   덧붙인다.
4. `build_print.py`를 한국어판과 원본 양쪽에 적용해 `print.html`의 ID 체계를
   맞춘다.
5. `check_links.py build/original book`: 원문에 없던 깨진 로컬 링크나 앵커가
   있으면 빌드를 실패시킨다.
6. `book`을 `site`로 옮기고 원문 `LICENSE-*`를 복사한다. `outputs = ["site"]`.

스크립트는 저장소 관례대로 프로젝트 안에 복사해 둔다. `preserve_anchors.py`,
`build_print.py`, `test_anchors.py`, `test_print.py`는 rustc-dev-guide에서,
`check_links.py`는 rust-forge에서 가져온다. `relativize_redirects.py`는
리다이렉트가 없어 가져오지 않는다.

툴체인은 `nix/projects/rust-api-guidelines.nix` devShell에 `mdbook`,
`python3`로 선언한다.

## 배포 배선

- `.github/pages-projects.json`: `{"project": "rust-api-guidelines",
  "target": "html", "artifact": "dist-rust-api-guidelines", "artifact_path":
  "projects/rust-api-guidelines/dist"}`를 배열 끝에 둔다.
- `.github/scripts/stage-pages.sh`: 마지막 `overlay_site` 호출 뒤에
  `overlay_site "dist-rust-api-guidelines" "site" "rust-api-guidelines"`.
- `.github/scripts/rebuild-translations.sh`: 기본 분기(`*`)는 state 파일 하나당
  원본 하나를 확인하므로 upstream이 새 파일을 추가해도 잡지 못한다.
  rustc-dev-guide처럼 `rust-api-guidelines)` 분기를 두어 `translate upstream/src`
  뒤에 `status --check upstream/src`를 디렉터리 단위로 실행한다.
- `.github/workflows/pr.yml`: `projects/rust-api-guidelines/scripts`의
  `test_*.py` 단위 테스트 스텝을 rustc-dev-guide 스텝 뒤에 둔다.
- `LICENSE.md`의 번역 프로젝트 표에 `rust-api-guidelines` | `MIT OR
  Apache-2.0`을 알파벳 순으로 넣는다.
- `site/index.html`: `data-topics="rust"` 항목들 옆에 제목 `Rust API
  가이드라인`, 원문 링크 `rust-lang/api-guidelines`, 태그 `Apache 2.0`·`MIT`·
  `기계 번역`으로 항목을 추가한다.

## README

`projects/rust-api-guidelines/README.md`에는 다음을 적는다.

- 번역 범위(`src/` 책 전체)와 고정 커밋.
- MIT OR Apache-2.0 라이선스 보존 방식(사이트에 `LICENSE-*` 복사).
- 재현 명령: `translate`, `status --check`, nix devShell 안에서 `build html`.
- `state/`는 커밋 대상이고 `ko/`·`build/`·`dist/`는 재생성 가능한 산출물이라는
  점.
- AGENTS.md의 출처 표기 규칙: [yeokja](https://github.com/yeokja/yeokja) 링크,
  실제 번역 모델, "Anthropic 사의 `claude-opus-5-5` 모델을 활용하여
  번역되었으며 학습을 모두 비허용한 상태로 작업하였습니다." 모델명은
  `state/**/*.yeokja.json` 이력으로 확인한다.

## 검증

- `yeokja status --check upstream/src`: 미번역 세그먼트 0.
- `yeokja evaluate --mechanical-only upstream/src`: Alignment/Format/Link
  항목은 체크리스트 6절대로 하나씩 확인하고, 실제 문제는 state에서 번역을
  지우고 다시 번역한다.
- 원문과 `ko/`의 파일별 `<a id=`, 코드 펜스, 링크 참조 정의(`]: `) 개수가
  같다.
- nix devShell에서 `yeokja build html`이 성공하고 `check_links.py`가 `New: 0`을
  낸다. `dist/site/naming.html`에 `id="c-case"`와 영어 제목 ID
  `id="casing-conforms-to-rfc-430-c-case"`가 모두 있다.
- `scripts/`의 단위 테스트가 통과한다.
- `.github/scripts/rebuild-translations.sh rust-api-guidelines`로 CI 재구성
  경로를 로컬에서 재현한다.
- 번역 품질: 몇 파일을 직접 읽어 격식체, 용어 일관성, 가이드라인 식별자
  보존을 확인한다.

## 위험과 대응

- **제목 안의 가이드라인 식별자:** 모델이 `(C-CASE)`를 빠뜨리거나 번역하면
  체크리스트와 제목의 대응이 끊긴다. 용어 기준 단계와 직접 확인으로 잡고,
  필요하면 해당 세그먼트를 다시 번역한다.
- **표 셀 안의 `<br>`과 `-\>`:** 직렬화기가 `-\>`를 `->`로 쓰는 것은
  CommonMark 렌더링이 같아 무해하다. `<br>` 손실은 `check` 단계의 개수 비교로
  확인한다.
- **upstream 갱신:** 이 번역은 고정 커밋 기준이다. 갱신은 서브모듈을 올린 뒤
  증분 번역으로 처리하며, `rebuild_daily`는 쓰지 않는다.
