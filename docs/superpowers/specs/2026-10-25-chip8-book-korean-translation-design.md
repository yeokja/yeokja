# Chip-8 Emulation in Rust 한국어 번역 설계

이슈: yeokja/yeokja#13

## 목표

[`aquova/chip8-book`](https://github.com/aquova/chip8-book)의 *An Introduction to
Chip-8 Emulation using the Rust Programming Language*(aquova, CC0 1.0)를 고정된
원문으로 삼아 yeokja로 본문 전체를 한국어로 번역하고, 한국어판 제목
**Rust로 배우는 Chip-8 에뮬레이션 입문**으로 mdBook HTML을 GitHub Pages
`chip8-book/`에 배포한다.

완료 상태는 다음 조건을 모두 만족해야 한다.

- 원문 저장소가 `projects/chip8-book/upstream` 서브모듈로
  `e0b3888bed29d5e7201e26c8a8a7cbc3d6becc8f`에 고정되어 있다.
- `upstream/src/*.md` 9개 장의 모든 세그먼트가 번역 상태이고
  `yeokja status --check upstream/src`가 통과한다.
- `yeokja evaluate --mechanical-only upstream/src`가 0건이다.
- opcode 표와 opcode 제목의 기계 토큰이 원문과 같다(`check_opcodes.py`).
- 한국어 mdBook 빌드가 경고 없이 성공하고, 같은 전처리를 거친 영어 빌드에
  없던 깨진 로컬 링크·앵커가 없다(`check_links.py`가 `New: 0`).
- Pages 배포 배선, 랜딩 페이지 항목, `LICENSE.md` 표, README 출처 표기가
  갖춰져 있다.

## 조사 결과

- **원문 형식.** mdBook 책이 아니다. `book.toml`은 있지만 `src/SUMMARY.md`가
  없고(그 파일을 더하는 업스트림 PR #1이 2021년부터 열려 있다), 실제 빌드는
  `Makefile`의 pandoc이 `src/1-intro.md`…`9-changes.md`와 `metadata.yaml`을
  이어 붙여 PDF/EPUB을 만드는 방식이다. 저장소는 2023-08-13 이후 휴면이다.
- **분량.** Markdown 9개, 19,270단어(산문 약 14,700, 코드 약 4,300),
  세그먼트 928개. 코드 펜스 112개(Rust 86개). 수식이 없다.
- **pandoc 관용구.**
  - 단독 줄 `\newpage` 10곳(PDF 쪽 나눔). mdBook에서는 글자로 보인다.
  - 헤딩 속성 `{#eb}`(2-basics), `{#ot}`(8-opcodes), `{#dfe}`(6-frontend)와 이를
    가리키는 파일 간 `](#id)` 링크 4개(2-basics → `#ot`, 4-methods → `#eb`,
    5-instr → `#ot`, `#dfe`). pandoc은 장을 한 문서로 합치므로 동작하지만
    mdBook처럼 장마다 페이지를 나누면 현재 페이지를 가리켜 깨진다.
- **표.** 8-opcodes의 opcode 표(35행)와 5-instr의 글꼴 주소 표(9행). 셀은
  `00E0`, `VX += VY`, `0xNNN` 같은 기계 토큰과 짧은 산문이 섞여 있다. markdown
  파서는 표 칸에 TableCell 역할을 달지 않아 `[[tables]]` 열 제외를 쓸 수 없다.
  opcode가 든 3단계 제목도 36개(`### 8XY4 - VX += VY`) 있다.
- **각주.** 2-basics의 `[^1]:`·`[^2]:`는 정상이다. 4-methods 33행의
  `[^1] *Underflow* …`는 콜론이 빠져 정의로 인식되지 않는 원문 결함이다.
- **이미지.** `src/img/` PNG 5개와 SVG 1개. 번역할 글자가 없다.
- **라이선스.** 저장소 전체가 CC0 1.0(`LICENSE`)이다. 단 `roms/`는 Zophar's
  Domain에서 받은 저작자 불명의 제3자 ROM이다(원문 README "Original author
  unknown").
- **파일럿.** 5-instr(210세그먼트, 표·opcode 제목·코드가 모두 있는 가장
  대표적인 장)를 기본 프롬프트로 번역했다. 합쇼체, 용어집 적용, 코드 블록·
  인라인 코드·`{#id}`·`](#id)` 보존이 모두 의도대로였다.

## 범위

- 번역: `upstream/src/*.md` 9개 장 전체(제목, 본문, 목록, 표 칸, 각주). 이미지
  대체 텍스트는 markdown 파서가 세그먼트로 내지 않아 원문 영어로 남는다.
- 번역하지 않음: 코드 블록과 그 안의 주석, 인라인 코드, URL, opcode 표기
  (`00E0`, `DXYN`), 레지스터 이름(V0–VF, VX, VY, I, PC, SP, DT, ST), 16진수
  리터럴, 크레이트·도구·파일 이름, 시스템 이름(Chip-8, Super Chip-8, XO-Chip,
  NES, Game Boy), 변경 기록의 링크 대상.
- 번역 대상이 아닌 손 작성 파일: 한국어·영어 `SUMMARY.md`와 `book.toml`, 한국어
  `metadata.yaml`(접근 방식 2).
- 저장소의 `README.md`, `code/`, `roms/`는 책에 나오지 않으므로 번역·배포하지
  않는다.

## 접근 방식

### 1. 주 산출물은 mdBook HTML

원문이 공식으로 내는 것은 pandoc PDF/EPUB이지만, 이 저장소의 배포 경로는
Pages 웹 판이고 mdBook 책 다수(rust-api-guidelines, write-you-a-haskell 등)와
같은 툴체인·검사 스크립트를 그대로 쓸 수 있다. 원문 저장소에도 `book.toml`이
있어 저자가 mdBook을 염두에 둔 흔적이 있다. PDF 타깃은 만들지 않는다.
pandoc+xelatex+한글 글꼴을 devShell에 넣는 비용에 비해 Pages 웹 판으로 충분하고,
한국어 `metadata.yaml`을 overlay에 두므로 나중에 PDF 타깃을 더할 길은 열려 있다.

### 2. `yeokja.toml`과 overlay

```toml
[[sources]]
path = "upstream/src"
pattern = "*.md"
parser = "markdown"
inline_tags = true
output = "ko/src/{path}"

[derive]
base = "upstream"

[[derive.overlay]]
path = "ko"
require_base = true

[[derive.overlay]]
path = "overlay/ko"
```

- `inline_tags = true`: 수식이 없고 mdBook(CommonMark)으로 렌더링하므로
  체크리스트 2절 조건을 만족한다. 기본 프롬프트를 쓴다(합쇼체, GFM 경고 마커
  보존 규칙 포함). 커스텀 `prompt_template`은 두지 않는다.
- markdown 파서는 헤딩의 `{#id}` 속성을 세그먼트 밖에 두고 재구성 때 원문
  그대로 붙인다. 앵커 ID가 번역·누락될 위험이 구조적으로 없다.
- `overlay/ko/src/SUMMARY.md`: 원문에 없는 목차. 각 항목은 번역된 장의 첫 제목과
  글자까지 같게 손으로 쓴다. `overlay/en/src/SUMMARY.md`는 비교용 영어 빌드에만
  쓰며 `derive`에 넣지 않는다. 장이 9개뿐이고 원문이 갱신되지 않으므로 목차
  생성 스크립트를 두지 않는다.
- `overlay/{ko,en}/book.toml`: 원문 `book.toml`의 `multilingual = false`를
  mdBook 0.5가 모르는 키로 거부하므로 그 키를 뺀 사본을 둔다. 한국어판은
  `language = "ko"`와 한국어 제목을 쓰고, 두 판 모두 원문 저장소를
  `git-repository-url`로 건다. 언어·제목 말고는 두 파일이 같아 링크 비교 조건이
  같다.
- `overlay/ko/src/metadata.yaml`: pandoc 표제 정보의 한국어판(제목, `lang: ko`,
  `toc-title: 목차`). 메타데이터는 9줄이고 대부분 LaTeX 설정이라 번역 소스로
  두지 않는다. mdBook 빌드는 이 파일을 쓰지 않으므로 사이트에서는 지운다.
- `[provider]`: `type = "claude_code"`, `model = "claude-opus-5-5"`.
  `[evaluation]`: `auto_evaluate = true`, `style_evaluate = false`,
  `max_retries = 3`. `[translation]`: `concurrency = 8`, `batch_segments = 32`.

### 3. 빌드 전처리(`scripts/prepare_book.py`)

pandoc 관용구를 mdBook에 맞추는 일은 원문·`ko/`를 고치지 않고 빌드 직전 사본에서
한다. 같은 스크립트를 영어 사본(`build/original-src`)과 조립 트리 `src/` 양쪽에
적용하므로 두 빌드가 같은 조건이 되어 `check_links.py`의 비교가 의미를 가진다.

- 코드 펜스 밖의 단독 줄 `\newpage`를 지운다. yeokja의 Markdown 직렬화기는
  번역 트리에 백슬래시를 이스케이프한 `\\newpage`로 쓰므로(CommonMark 렌더링은
  같다) 두 표기를 모두 지운다. 줄 가운데에 남은 `\newpage`가 있으면 실패한다.
- 모든 장의 헤딩에서 `{#id}`를 모아 ID → 파일 매핑을 만들고, 다른 장을
  가리키는 `](#id)`를 `](file.md#id)`로 바꾼다. 매핑은 트리에서 직접 읽으므로
  한국어 트리에서도 같은 결과가 된다. 정의되지 않은 ID를 가리키는 링크와 두
  장에 중복 정의된 ID는 오류다.

### 4. opcode 토큰 검사(`scripts/check_opcodes.py`)

표 칸과 opcode 제목은 산문 세그먼트로 번역되므로, 모델이 `0xNNN`을 `NNN`으로
줄이거나 `VX != VY`의 연산자를 고치면 참조표가 조용히 틀린다. 검사는 원문과
번역의 같은 순서의 표 행·opcode 제목마다 opcode, 레지스터, 16진수, 숫자,
`N`/`NN`/`NNN` 피연산자, 대입·비교 연산자, `rand()`의 다중 집합이 같은지 본다.
한글이 `\w`에 들므로 경계는 ASCII 문자로만 판정한다. 빌드 첫 단계에서 돌려
어긋나면 빌드를 실패시킨다. 위반 세그먼트는 state에서 지우고 다시 번역한다.

### 5. 원문 결함

4-methods의 콜론 빠진 각주(`[^1] *Underflow* …`)는 원문대로 둔다. 번역 대상은
원문의 렌더링 결과이고, 원문 mdBook·pandoc 빌드 모두 이 줄을 각주 정의가 아닌
일반 문단으로 보여 주므로 번역판도 같게 보이는 것이 원문 충실성에 맞다. 고치면
영어 빌드와의 링크 비교에서 각주 앵커가 한쪽에만 생긴다. 이 문단은 평범한
문단으로 번역되며 `[^1]`은 글자 그대로 남는다.

### 6. HTML 빌드(`[build.html]`)

rust-api-guidelines의 틀을 따른다.

1. `check_opcodes.py upstream/src src`.
2. 원문 `src/`를 `build/original-src`로 복사하고 `overlay/en`(`book.toml`,
   `src/SUMMARY.md`)을 얹는다. 양쪽에 `prepare_book.py`를 돌리고
   `metadata.yaml`을 지운다.
3. 영어 빌드 `mdbook build build/original-src --dest-dir build/original`, 한국어
   빌드는 조립 트리에서 `mdbook build`(제목·언어는 overlay의 `book.toml`). 어느
   쪽이든 로그에 warn/error가 있으면 실패한다.
4. `preserve_anchors.py`: opcode 제목 36개 등 mdBook 자동 제목 ID가 번역으로
   바뀌므로, 렌더링된 영어 페이지의 ID를 한국어 페이지 제목에 덧붙인다.
5. `build_print.py`를 양쪽에 적용해 `print.html`을 장별 ID 접두사로 다시 합친다.
6. `check_links.py build/original book`: 영어 빌드에 없던 깨진 로컬 링크가
   있으면 실패한다.
7. `book`을 `site`로 옮기고 원문 `LICENSE`를 복사한다. `outputs = ["site"]`.

`preserve_anchors.py`, `build_print.py`, `check_links.py`는 rust-api-guidelines의
사본이다. 툴체인은 `nix/projects/chip8-book.nix`에 `mdbook`, `python3`로
선언한다. 스크립트는 표준 라이브러리만 쓴다.

## 용어

`glossary.toml`은 rustc-dev-guide·rust-api-guidelines와 번역어를 맞춘다
(크레이트, 구조체, 클로저, 패턴 매칭).

- 에뮬레이션 용어: emulator → 에뮬레이터, emulation → 에뮬레이션, opcode →
  옵코드, instruction → 명령어, register → 레지스터, program counter → 프로그램
  카운터, stack pointer → 스택 포인터, subroutine → 서브루틴, delay timer → 지연
  타이머, sound timer → 사운드 타이머, sprite → 스프라이트, carry flag → 캐리
  플래그, underflow/overflow → 언더플로/오버플로, binary-coded decimal → 이진화
  십진법, frontend/backend → 프런트엔드/백엔드, canvas → 캔버스, font → 폰트
  (내장 숫자 스프라이트 `FONTSET`과 맞춤), guide → 가이드(책 자신을 가리키는
  말). 마지막 두 항목이 없으면 "글꼴"/"폰트", "가이드"/"안내서"가 장마다 섞인다.
- 원문 그대로: Chip-8, Rust, WebAssembly(webassembly-component-docs와 같음).
- *tick*, *fetch*, *decode*, *execute*처럼 동사로도 쓰이는 짧은 단어는 용어집에
  두지 않는다. 용어 검사가 동사 용법에서 오탐을 내어 재시도를 일으킨다. 반복
  용어의 일관성은 `terms build`로 맞춘다.

첫 번역 뒤 `yeokja terms build`로 `terms.toml`을 만들고 검토한 다음
`glossary adopt --prefer HEAD`, 남은 세그먼트 번역 순서로 진행한다(체크리스트
3절).

## 라이선스

원문은 CC0 1.0이므로 저작자 표시 의무가 없다. 그래도 README와 랜딩 항목에
원저자 aquova(Austin Bricker), 원문 저장소, 고정 커밋을 적는다. 번역물도
CC0 1.0으로 배포하고 사이트 루트에 원문 `LICENSE`를 싣는다. `roms/`는 CC0
적용 여부가 불분명한 제3자 ROM이라 배포하지 않는다. mdBook은 `src/`만 내놓으므로
조립 트리에 `roms/`가 남아도 산출물에 들어가지 않는다. 루트 `LICENSE.md`
표에 `chip8-book | CC0-1.0`을 더한다.

## 배포

- `.github/pages-projects.json`: `project = "chip8-book"`, `target = "html"`,
  `artifact = "dist-chip8-book"`, `artifact_path = "projects/chip8-book/dist"`.
- `.github/scripts/stage-pages.sh`:
  `overlay_site "dist-chip8-book" "site" "chip8-book"`.
- `.github/scripts/rebuild-translations.sh`: `chip8-book)` 분기에서 재구성 전에
  `status --check upstream/src`를 디렉터리 단위로 실행한 뒤
  `translate upstream/src`를 돌린다. 새 장이 state 없이 빠지는 경우도 막고 CI가
  provider를 부르지 않는다.
- `site/index.html`: 하드웨어 주제 아래 항목(원문 링크, `CC0 1.0`, `기계 번역`
  태그).

## 작업 순서

1. 프로젝트 골격: `yeokja.toml`, `glossary.toml`, `.gitignore`(`ko/`, `build/`,
   `dist/`, `__pycache__/`), overlay, scripts.
2. 5-instr 시험 번역 → 문체·용어·opcode 토큰 검토 → 용어집 보정.
3. 나머지 번역 → `evaluate --mechanical-only` 0건 → `terms build` →
   `glossary adopt --prefer HEAD` → 남은 세그먼트 번역 → `status --check`.
4. 한국어 `SUMMARY.md`, nix devShell, HTML 빌드와 검사, 브라우저 확인.
5. README, 배포 배선(호출 측 통합), `rebuild-translations.sh chip8-book` 확인.

## 되짚을 수 있는 결정

- **mdBook으로 내놓음.** 원문의 공식 산출물(PDF/EPUB)과 모양이 다르다. 한국어
  PDF가 필요해지면 `build.pdf`에서 overlay의 한국어 `metadata.yaml`과 원문
  `Makefile`의 pandoc 명령을 쓰고 devShell에 pandoc·xelatex·한글 글꼴을 더한다.
- **SUMMARY.md를 손으로 씀.** 업스트림이 PR #1을 받아 `SUMMARY.md`를 갖게 되면
  overlay를 지우고 원문 파일을 번역 소스로 돌린다.
- **콜론 빠진 각주를 그대로 둠.** 업스트림이 고치면 번역도 원문을 따라 바뀐다.
- **opcode 표를 산문 세그먼트로 번역함.** markdown 파서가 표 칸 역할을 달게
  되면 Opcode 열을 `[[tables]]`로 번역에서 빼고 `check_opcodes.py`는 opcode
  제목만 보게 줄인다.
