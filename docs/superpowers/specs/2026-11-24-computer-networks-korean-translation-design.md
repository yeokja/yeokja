# Computer Networks: A Systems Approach 한국어 번역 설계

## 목표

Larry L. Peterson과 Bruce S. Davie의 *Computer Networks: A Systems Approach* 7판
([SystemsApproach/7E](https://github.com/SystemsApproach/7E), Sphinx/RST, CC BY 4.0)을 한국어로 옮겨
`projects/computer-networks`로 추가하고, 영어 원문 빌드와 쪽·앵커·그림·표 구조가 같은 사이트를 Pages의
`/computer-networks/`에 배포한다(이슈 #19). 책 제목은 "컴퓨터 네트워크: 시스템 접근"이며 원제를 함께 적는다.

## 조사 결과

- 원문: Sphinx 8.2 + sphinx-book-theme 1.4(`upstream/requirements.txt`). RST 107개 파일, 9,836 세그먼트(코어 변경 뒤)
  (약 20.9만 단어, 세그먼트 중앙값 121자, 최대 763자). 그림 212개, `.. table::` 18개, `:numref:` 339개,
  `:math:` 153개, `.. index::` 약어 항목 다수, 치환 참조(`|Intro|` 등 장 번호) 465개 세그먼트.
- 구조: 최상위 장 문서(`introduction.rst` 등 26개)가 하위 디렉터리의 절 파일을 `.. include::`로 엮는다.
  `conf.py`는 `*/*rst`와 `CONTRIBUTING.rst`, `*/README.rst`를 빌드에서 뺀다. 어느 장에서도 읽지 않는 RST는
  `CONTRIBUTING.rst`와 `resources/README.rst` 둘이다.
- 제목 307개에 명시적 레이블이 하나도 없다. 쪽 안 앵커(`#sliding-window`)가 모두 제목 글자에서 나오며,
  원문 안에서도 `book.systemsapproach.org/…/reliable.html#sliding-window` 같은 깊은 링크를 쓴다. `:ref:`는
  명시적 레이블 하나(`reading-VL2`)만 가리킨다.
- `conf.py`의 `setup()`은 Google Analytics 스크립트만 넣는다. 확장 `sphinxcontrib.spelling`은 PyEnchant가
  libenchant C 라이브러리를 찾지 못하면 로드 단계에서 빌드를 멈춘다(nix 셸에서 확인). 원문은 그 확장의
  `:spelling:ignore:` 역할을 6곳에서 쓴다.
- 장 번호 치환이 정의된 `chapters.rst`는 최상위 문서라 Sphinx가 독립 쪽(`chapters.html`, 내용 없음)으로도 만든다.
- 7판의 한국어 번역은 없다(이전 판의 상용 번역서만 있다).

## 범위

- 번역: `upstream/**/*.rst` 105개 파일(최상위 장 문서, 절 파일, 목차의 "About This Book"인 `README.rst`).
- 제외: `CONTRIBUTING.rst`, `resources/README.rst` — 원문 빌드가 빼므로 독자에게 보이지 않는다.
- 원문 그대로: 그림 이미지(PNG·JPG 속 글자), 코드·`literalinclude` 파일, 수식, `.. index::` 약어 항목(약어
  목록 쪽은 영어 약어 사전으로 남긴다. 항목을 옮기면 정렬과 검색 대상이 원문과 어긋난다), 더 읽을거리의
  RFC·논문·책 제목(독자가 원문 제목으로 찾는다), 사람 이름, URL.

## 접근 방식

### 1. 원문 고정

`main`의 HEAD(`889b23414b71c4cda23a2d8a7aa2d294b419bccf`, 2026-11-23)에 고정한다. 마지막 태그 `v7.0`
(`d154d20`)이 아니라 HEAD인 이유: 원문 README가 `main`을 늘 일관된 상태로 유지하고 웹판도 `main`에서
만든다고 밝히며, 태그는 수업용 스냅숏일 뿐이다. 독자가 보는 원문 웹판과 같은 내용을 옮긴다.

### 2. 소스 규칙

```toml
[[sources]]
path = "upstream"
pattern = "**/*.rst"
exclude = ["CONTRIBUTING.rst", "resources/README.rst"]
parser = "rst"
output = "ko/{path}"
```

- `inline_tags`는 `rst` 파서에 없으므로 쓰지 않는다.
- 기본 프롬프트를 쓴다. 커스텀 `prompt_template`이 필요할 만한 문체·형식 요구(합쇼체, 코드·역할 보존,
  경고 표시 마커)가 기본 프롬프트에 이미 있고, 커스텀 템플릿은 코어 프롬프트 개선(마커 보존 규칙 등)을
  자동으로 받지 못한다.
- `:numref:` 글자: ``:numref:`Figure %s <fig-x>` ``는 독자에게 ``그림 %s``로 보여야 한다. 역할 내용은 파서와
  용어 검사가 손대지 않는 원자라 번역이 일관되게 바꾸지 못하고(시험 번역에서 모두 영어로 남았고, 용어집에
  Figure를 넣어도 역할 안은 검사하지 않는다), 이 글자는 Sphinx가 번호를 채우는 고정 틀이므로 조립 단계의
  `prepare.py`가 ``Figure(s) %s``·``Table(s) %s``의 낱말만 "그림"·"표"로 바꾼다. `%s`와 `<target>`은 그대로이고,
  Sphinx 빌드의 `--fail-on-warning`이 표적 손상을 잡는다.
- 장 제목: "Chapter |TCP|: Reliable Byte Stream" → "|TCP|\ 장: 신뢰성 있는 바이트 스트림"(렌더링 "12장: …").
  "제"를 붙이지 않고 번호와 "장" 사이를 띄우지 않는다. 부 제목도 "1부: 기초"처럼 쓴다. 장 제목은 세그먼트마다
  하나뿐이라 `terms build`가 통일하지 않으므로 번역 뒤 state에서 이 꼴로 맞췄다.
- 책 제목: 표지 쪽 제목과 `html_title`은 "컴퓨터 네트워크: 시스템 접근"이고, 원제는 모든 쪽 위의 고지에 함께
  적는다(표지 제목에 원제를 붙이면 RST 제목 밑줄 폭을 넘고 쪽 제목·탭 제목이 길어진다). 본문 속 책 제목과
  README.rst의 저작자 표시 줄(Title/Authors/Copyright/Source/License)은 원문 그대로다.
- 장 번호: "Chapter |Intro|: Introduction"처럼 치환 참조가 한국어 조사·낱말과 붙는 자리는 RST가 치환으로
  읽도록 `\ `로 경계를 둔다("|Intro|\ 장"). 이 경계 복구는 `rst` 파서 코어가 한다(아래 6절).

### 3. 제공자와 규모

`claude_code` / `claude-opus-5-5`, `concurrency = 6`, `batch_segments = 32`, `auto_evaluate = true`,
`max_retries = 3`. 다른 프로젝트와 같은 제공자·모델이다. 동시 실행 수는 같은 체크아웃에서 여러 번역
프로젝트가 함께 돌 때의 한도를 고려한 값이다. 약 9.6천 세그먼트이므로 전체 번역은 백그라운드로 돌리고,
멈추면 같은 명령을 다시 실행해 남은 세그먼트만 이어서 번역한다.

### 4. 용어

`glossary.toml`에 손으로 정한 용어를 두고(`glossary_plurals = true`), 첫 번역 뒤 `terms build`로 반복 용어와
제목 번역어를 `terms.toml`에 고정한다.

| 원어 | 번역어 | 비고 |
|---|---|---|
| packet / switch / router / link / host / protocol | 패킷 / 스위치 / 라우터 / 링크 / 호스트 / 프로토콜 | |
| datacenter, data center | 데이터센터 | 원문의 두 표기를 붙여 쓴 한 표기로 |
| bandwidth / latency / throughput | 대역폭 / 지연 시간 / 처리량 | |
| round-trip time | 왕복 시간 | 원문이 (RTT)를 붙인 곳은 "왕복 시간(RTT)" |
| congestion control / flow control | 혼잡 제어 / 흐름 제어 | |
| forwarding table / routing | 포워딩 테이블 / 라우팅 | |
| best-effort | 최선형 | 처음 나올 때 원어 병기 |
| statistical multiplexing / sliding window | 통계적 다중화 / 슬라이딩 윈도 | "윈도우"가 아니라 "윈도" |
| encapsulation / end-to-end | 캡슐화 / 종단 간 | |
| network address translation | 네트워크 주소 변환 | 원문이 (NAT)를 붙인 곳은 "네트워크 주소 변환(NAT)" |
| Chapter / Section / Further Reading | 장 / 절 / 더 읽을거리 | "Section 3.2" → "3.2절" |

`routing`에는 `except = ["|Routing|"]`를 둔다. 장 번호 치환 이름 `|Routing|`은 낱말이 아니므로 그 자리에서
"라우팅"을 요구하면 "|Routing|.5.1 …" 같은 절 제목이 모두 오탐이 된다.

`terms build`가 고른 용어 중 window 계열(window, congestion window, advertised window, window size)은 "윈도"로
고친다(슬라이딩 윈도와 같은 표기). "윈도"는 "윈도우"의 부분 문자열이라 용어 검사가 "윈도우"를 거르지 못하므로
state의 "윈도우"도 "윈도"로 바꿨다. replay(→재전송, retransmission과 겹침)와 deficit(→적자, DRR 문맥에 맞지 않음)는
용어에서 빼고 `rejected`에 둔다.

번역어에는 괄호 원어를 넣지 않는다. 용어 검사는 번역어가 번역문에 있는지를 보므로, 번역어에 "(RTT)"를
넣으면 원문에 약어가 없는 곳까지 약어를 강요한다. 약어 병기는 원문을 따르고 note로 안내한다.

### 5. 조립과 빌드

- `[derive]`: `upstream` 위에 `ko/`를 얹은 링크 트리(`require_base = true`). 생성 단계
  `scripts/prepare.py .`가 트리를 한국어 빌드용으로 고친다. 링크 트리이므로 고칠 파일은 링크를 새 파일로
  바꿔(`os.replace`) 원문 서브모듈에 쓰지 않는다. 고칠 자리는 원문 글자를 그대로 고정점으로 삼아, 원문을
  올려 고정점이 사라지면 조용히 넘어가지 않고 실패한다. 두 번 실행해도 결과가 같다.
- `conf.py`에는 원문 설정 뒤에 관리 블록을 덧붙여 같은 이름을 다시 정의한다(원문 설정을 지우지 않으므로
  upstream이 다른 설정을 바꿔도 따라갈 것이 없다).
  - `language = "ko"`, `numfig_format = {"figure": "그림 %s.", "table": "표 %s."}`,
    `html_title = "컴퓨터 네트워크: 시스템 접근"`, 버전 표시 "Version 7.1-dev" → "버전 7.1-dev".
  - 확장에서 `sphinxcontrib.spelling`과 `sphinx_multiversion`을 뺀다. 맞춤법 검사는 영어 원문용이고 libenchant
    시스템 라이브러리를 요구하며, 여러 버전 빌드는 하나의 고정 커밋을 옮기는 번역에 쓰이지 않는다. 빠진
    확장이 제공하던 `:spelling:ignore:` 역할만 글자를 그대로 내보내는 작은 `spelling` 도메인으로 남긴다.
  - `setup()`을 다시 정의해 Google Analytics 스크립트를 넣지 않는다(번역 사이트에서 원문 측 추적 ID로
    방문을 보내지 않는다).
  - `html_theme_options["announcement"]`: 모든 쪽 위에 원저작자, 원문 링크, CC BY 4.0, 번역·수정 사실,
    비공식 기계 번역임을 알린다(CC BY 4.0 3(a)의 저작자 표시와 변경 표시).
  - `chapters.rst`를 `exclude_patterns`에 넣는다. 링크 트리에서는 Sphinx가 `.. include::` 관계를 알아보지
    못해 "어느 목차에도 없는 문서" 경고를 내고, 이 쪽은 장 번호 치환만 담은 빈 쪽이라 독자에게 쓸모가 없다.
    include는 그대로 된다.
  - 영어 앵커 확장 `en_anchors`(아래)와 `suppress_warnings += ["autosectionlabel.*"]`. autosectionlabel의
    레이블은 제목 글자이므로 서로 다른 원문 제목이 같은 번역어가 되면 중복 경고가 나지만, 원문의 `:ref:`는
    명시적 레이블만 가리키므로 깨진 참조가 아니다.
- 목차 레이블: `toctree`는 파서가 옮기지 않는 불투명 블록이므로 `prepare.py`가 `index.rst`의
  `:caption: Table of Contents` → "목차", `Acronym Glossary <genindex>` → "약어 목록", `_templates/layout.html`의
  같은 링크 글자, `_templates/genindex.html`의 쪽 제목과 머리말을 바꾼다(머리말에는 항목이 원문 그대로라는 한
  문장을 덧붙인다). "약어 목록"은 이 쪽이 `.. index::` 약어 항목만 모은 색인이라는 사실을 그대로 말한다.
- 영어 앵커(`scripts/en_anchors.py`): doctree-read(우선순위 400, 목차 수집기보다 먼저)에서 같은 경로의 원문
  장 문서를 별도 프로세스의 순수 docutils로 읽어 섹션 id를 순서대로 모으고, 번역 doctree의 같은 자리 섹션에
  첫 id로 올린다. 별도 프로세스인 이유: Sphinx는 docutils 전역 등록부에 자기 지시문을 올려 두므로 빌드
  프로세스 안에서 읽으면 그 지시문이 env를 찾다 실패하고, env를 넘기면 원문이 번역 문서의 env를 오염시킨다.
  docutils는 치환을 풀기 전의 제목 글자로 id를 만들므로(`chapter-intro-introduction`) 원문 사이트와 같다.
  섹션 수가 다르면 경고(빌드 실패)를 낸다. 원문 사이트로 들어오던 깊은 링크가 도메인만 바꿔 그대로 통하고,
  `check_site.py`가 앵커 집합을 원문과 직접 비교할 수 있다.
- 빌드: `sphinx-build -M html . _build --fail-on-warning --keep-going` 후 `_build/html` → `site`. `prepare.py`를
  거친 영어 원문 빌드가 경고 0건이므로 경고를 오류로 다룬다(번역이 역할·지시문·표적을 깨면 빌드가 멈춘다).
- 툴체인: `nix/projects/computer-networks.nix` — `python3`, `uv`와 공용 `python-venv.nix`로 upstream
  `requirements.txt`를 `build/venv`에 설치한다(peps와 같은 방식). spelling 확장을 빼므로 libenchant는 넣지 않는다.

### 6. 코어 의존성

`rst` 파서 코어 변경이 선행한다(별도 spec). 이 프로젝트는 그 변경이 들어간 CLI로만 번역한다.

- 그림 지시문의 캡션과 범례를 번역 세그먼트로 읽는다(그림 212개, 약 300 세그먼트).
- 치환 참조 경계 복구: 번역에서 `|name|` 바로 뒤나 앞에 한국어가 붙으면 `\ `를 넣는다(465 세그먼트).
- `.. table::` 제목을 세그먼트로 읽는다(18개).

코어가 들어오기 전에 번역하면 캡션이 영어로 남거나 장 제목 치환이 글자로 나오므로, 시험 번역도 코어 병합
뒤에 한다.

### 7. 검사

- `yeokja evaluate --mechanical-only`, `status --check` 통과. Alignment 오탐(원문의 "It"·"it"을 한국어 주어로
  풀어 이웃 세그먼트의 이름 IPv6·AS3·R2를 쓴 번역)은 하나씩 확인해 남긴다. 다시 번역해도 같은 주어를 쓰며 그것이
  옳은 번역이다.
- `scripts/check_site.py dist/site`: `prepare.py`를 거친 영어 원문 빌드(`build/reference`)와 쪽마다 비교한다 —
  쪽 목록, 섹션 앵커와 순서, h1–h6·`figure`·`table`·`img`·번호 캡션(`caption-number`)·수식 블록 수, 로컬
  링크·자산의 대상 파일과 `#앵커`, Google Analytics 잔재, 번역 고지. 두 빌드가 같은 준비 단계를 거치므로 차이는
  번역에서만 나온다.
- `scripts/test_prepare.py`: 링크 트리에서 원문 서브모듈을 건드리지 않는지, 멱등성, 관리 블록이 실제로
  실행되어 값을 덮는지, 고정점이 사라지면 실패하는지, 원문 섹션 id 계산(include·치환).
- 브라우저: 장 쪽(그림 캡션 "그림 1.1.", `:numref:` 링크, 장 제목 번호), 표가 있는 쪽, 약어 목록, 고지, 검색.

## 라이선스

원문은 CC BY 4.0이다. 저작자 표시는 원문 README의 두 줄을 따른다 — Peterson & Davie 2026
(<https://github.com/SystemsApproach/7E>)과 이전 판 내용 © Elsevier 2012(<https://github.com/SystemsApproach/book>),
둘 다 CC BY 4.0. 번역은 개작물로 같은 CC BY 4.0으로 내고, README와 사이트 고지에 변경 사실(번역, 빌드 설정
변경, 분석 스크립트 제거)을 적는다. 그림은 원문 그대로 싣는다.

## 배포

Pages 경로 `/computer-networks/`. `pages-projects.json` 항목, `stage-pages.sh` 줄, `rebuild-translations.sh`
분기, 루트 `LICENSE.md` 표 한 줄, `site/index.html` 카드는 통합 담당이 넣는다. CI는 state에서 `ko/`를
재구성하고 devShell로 빌드만 한다.

## 되짚을 수 있는 결정

- 원문을 올릴 때: `prepare.py`의 고정점(언어, 확장 목록, numfig_format, html_title, setup, 목차 레이블)이
  바뀌었으면 테스트가 실패하므로 관리 블록을 원문에 맞춰 고친다. 원문을 올린 뒤에는 `reconcile`을 쓰지 않는다.
- 원문 `virtual/datacenter.rst`의 사이드바는 강조 안에 `|Routing|`을 써서 원문 사이트에서도 치환되지 않은
  글자로 나온다. 번역도 원문 그대로 둔다.
- 약어 목록을 번역하려면 `.. index::` 항목을 세그먼트로 읽는 파서 변경과 정렬 기준 결정이 먼저 필요하다.
- 원문이 제목에 명시적 레이블을 달기 시작하면 섹션의 첫 id가 레이블 id로 바뀐다. `en_anchors`는 원문 쪽도 같은
  규칙으로 읽으므로 그대로 맞지만, 원문 갱신 때 `check_site.py`의 앵커 비교를 다시 확인한다.
