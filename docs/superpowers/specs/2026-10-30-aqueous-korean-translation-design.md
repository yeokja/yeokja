# Aqueous Archive Format 명세 한국어 번역 설계

## 목표

Kang Seonghoon이 쓰는 *Aqueous Archive Format* 명세(편집자 초안,
`github.com/lifthrasiir/aqueous`의 `spec/index.bs`, 공개 도메인/MIT-0)를 yeokja로 한국어로
옮겨 GitHub Pages(`aqueous/`)로 낸다. 파서는
`docs/superpowers/specs/2026-10-30-bikeshed-parser-design.md`의 `bikeshed`이며 이 명세가 그
첫 사용처다. 한국어 제목은 "Aqueous 아카이브 형식 명세"다(랜딩·README). 문서의 `<h1>`은
원문 "Aqueous Archive Format"을 그대로 옮긴 "Aqueous 아카이브 형식"이며 `<title>`도 같다 —
원문 제목에 "Specification"이 없으므로 본문 제목에는 "명세"를 더하지 않는다.

완료 상태는 다음 조건을 모두 만족해야 한다.

- 원문이 `projects/aqueous/upstream` 서브모듈로 커밋 `3d37dae5`(2026-10-29)에 고정되어 있다.
- `upstream/spec/index.bs`의 모든 세그먼트가 번역 상태이고 `yeokja status --check
  upstream/spec`이 통과한다. `evaluate --mechanical-only`가 0건이다.
- 원문과 번역을 같은 Bikeshed(7.2.0)로 빌드해 둘 다 `--die-on=warning`을 통과하고,
  `scripts/check.py`가 두 HTML의 요소 id 집합, 요소 이름 다중집합, `<dfn>` 수, 수준별 제목
  수, 내부 링크 해소를 같다고 판정한다.
- 빌드된 페이지가 어느 upstream 커밋의 스냅샷인지, 원문 링크, 비공식 번역임을 머리말에
  보인다.
- Pages 배포 배선, 랜딩 페이지 항목, README 출처 표기, 루트 `LICENSE.md` 항목이
  갖춰져 있다.

## 조사 결과

- **형식.** Bikeshed 소스 한 파일(`Markup Shorthands: markdown yes`), 5,298줄, 약 29k 단어.
  `bikeshed` 파서로 세그먼트 2,471개(`bikeshed:` 링크를 담은 것 825개). 그림은
  `figures/segments.svg`(pikchr 산출물이 커밋되어 있어 pikchr가 필요 없다) 하나를
  `<pre class=include-raw>`로 넣는다.
- **변경 속도.** 편집자 초안이 주 단위로 크게 바뀐다(최근 7일 +5,844/-5,407줄). 번역은
  스냅샷이며, 원문을 올릴 때마다 `ContextChanged`·`Stale` 세그먼트를 다시 번역한다.
- **빌드 도구.** nixpkgs의 `bikeshed`는 의존성(`json-home-client`)이 broken이라 쓸 수 없다.
  PyPI `bikeshed==7.2.0`(Python ≥3.12)은 원문 `index.bs`를 약 2초에 빌드하며 경고가
  없다(lint 메시지 하나: 2770행 `result` dfn 미참조). `bikeshed update`로 내려받는
  참조 데이터(anchors, biblio)가 처음 한 번 필요하다.
- **Bikeshed의 지역화 수단.** 메타데이터는 문서 어디의 `<pre class=metadata>`에서든
  합쳐지며 `Title`·`Date`·`Warning`·`Custom Warning Title` 같은 단일 값 키는 뒤의 것이
  이긴다(`joinValue`). 7.2.0에는 명령줄 메타데이터 덮어쓰기(`--md-*`)가 없다.
  `Translation: <lang> <url>, name …, native-name …`은 머리말 `Translations` 항목에
  링크를 둔다. `Warning: custom` + `Custom Warning Title/Text`는 머리말에 열린 `<details>`
  경고 상자를 둔다. `Local Boilerplate: <name> yes`로 스펙 옆의 `<name>.include`를 쓸 수
  있지만 그 선언 자체도 메타데이터이고, 코드가 만드는 제목("Table of Contents", "Index",
  "References", "Terms defined by this specification" 등)은 번역 훅(`_t`)이 항등 함수라
  include로는 바꿀 수 없다. 노트·예시·이슈의 "Note"/"Example"/"Issue" 라벨은 CSS
  `::before { content }`다.
- **문자 참조.** 원문이 `&lt;`·`&gt;`·`&nbsp;`·`&thinsp;`를 쓴 곳은 날것 `<<`가 Bikeshed의 CSS
  생성 규칙 `<<foo>>`로, `<public…>`이 HTML 태그로 읽히는 자리다(`|a| &lt;&lt; |b|`, IANA 절의
  `&lt;public+aqr@mearie.org&gt;`, 상수 표의 `&thinsp;`). 인라인 태그 코덱이 참조를 원문대로
  되돌려 쓰고, state의 `translation`에 적힌 참조도 그대로 출력된다. `scripts/entities.py
  check`는 번역이 참조를 잃지 않았는지 확인하는 안전망이다.
- **시험 번역**(1–399행, 203 세그먼트, 27초, 재시도 2건). `[=key=]`·`[=key|text=]`가
  바르게 되돌려지고(`[=implicitly-sized|암묵 크기=]`), 번역된 dfn에 `lt=`가 붙으며
  (`<dfn export lt="implicitly-sized">암묵 크기</dfn>`), `|var|`가 모두 남았다. 원문과
  번역의 Bikeshed 메시지 집합은 줄 번호와 dfn 글자만 다르고, id 집합과 dfn 수(38)가
  같았다. 용어집은 복수형("shorthand blocks" → 단축 블록, "formulas" → 수식, "entries" →
  항목)을 잡지 못했다.
- **빌드 검사가 요구하는 파서 성질.** 원문과 번역의 Bikeshed 출력이 요소 수까지 같으려면
  HTML 블록 요소(`<ol class=algorithm>`, `<div class=note>`) 안의 Markdown 목록이 인라인
  HTML(`C1<sub>16</sub>`, `|e|<sub>|i|</sub>`, `<dfn>`, `<a>`)이 있어도 항목 단위로
  세그먼트가 되어야 하고, Bikeshed 정의 목록 구문 `: 용어` / `:: 설명`(72줄)이 항목 단위로
  주어져야 한다. 파서가 이를 지키므로 `scripts/check.py`의 요소 다중집합 비교가 통과한다;
  어느 쪽이 깨지면 알고리즘 목록이 한 `<li>`로, 정의 목록이 한 `<p>`로 합쳐져 그 비교가
  빌드를 막는다.
- **저자의 한국어.** 원문 저장소 `docs/`는 한국어다. 아카이브, 블록, 세그먼트, 체크섬,
  재압축, 바이트열, 사전, 엔트리, 헤더, 미러, 복구, 평가를 쓰고 formula·leaf·critical·parcel은
  영어로 둔다.

## 범위

- 번역: 본문 산문, 제목, 표 머리글, 노트·예시·이슈, `<dfn>` 산문 용어, 자동 링크·절 참조의
  표시 글자, `Abstract:`.
- 번역하지 않음: 나머지 메타데이터, `<pre>`(CDDL, biblio, anchors, link-defaults, include-raw),
  16진 상수와 `<sub>16</sub>` 표기, 식별자형 정의(`u8`, `leb128`, `offset`, `HEADER_TYPE`,
  `block_start`, `aqr_send` …), 명령 이름(Nop, Literal, Swap, Slice, Apply, Concat, Pack,
  Split, Patch, Checksum — "Nop 명령"), 레지스트리 표의 이름·해시·데이터 열, 조건 키워드
  `Optional`/`If`/`Depends on`(표에서 `[=If=]`로 쓰여 영문 그대로 둔다), Bikeshed의 CC0/OWFa
  저작권 문단(원문 페이지의 법적 문구).
- 저장소의 `README.md`, `docs/`(저자의 한국어 설계 메모), 코덱·크레이트는 범위 밖이다.

## 접근 방식

### 1. `yeokja.toml`

```toml
[[sources]]
path = "upstream/spec"
pattern = "*.bs"
parser = "bikeshed"
inline_tags = true
output = "ko/spec/{path}"

[derive]
base = "upstream"
[[derive.overlay]]
path = "ko"
require_base = true

[build.html]
command = 'python3 "$YEOKJA_ROOT/scripts/build.py" "$YEOKJA_ROOT" "$PWD"'
outputs = ["site"]

[provider]
type = "claude_code"
model = "claude-opus-5-5"
prompt_template = '…'

[translation]
concurrency = 6
batch_segments = 32
```

커스텀 `prompt_template`을 둔다. 인라인 태그 모드에서는 코어의 Bikeshed `closing_rule`이
가지 않으므로 파서 설계 5절의 규칙을 프로젝트가 가진다. 템플릿은 코어의 자리표시자
`{source_lang}`·`{target_lang}`·`{inline_tags}`·`{glossary}`·`{feedback}`·`{context}`·
`{surroundings}`·`{segments}`를 모두 쓰고, 기본 템플릿의 합쇼체 규칙과 코드 스팬 규칙을
옮겨 적는다. 추가 규칙:

- `|var|`는 변수 이름이라 그대로 한 번 두고 조사는 닫는 `|` 뒤에 붙인다(`|n|을`).
- `[[#section]]`, `[[RFC8949]]`, `[[!AGE]]` 같은 이중 대괄호 참조는 그대로 둔다.
- 정의 용어인 링크 글자는 용어집 번역을 쓰고 정의와 참조가 같은 글자로 읽히게 한다.
- 식별자형 이름·상수·명령 이름·레지스트리 이름·16진수·CDDL은 영문 그대로 둔다.
- RFC 2119 키워드는 고정 어구로 옮긴다: MUST → ~해야 합니다, MUST NOT → ~해서는 안
  됩니다, SHOULD → ~하는 것이 좋습니다, SHOULD NOT → ~하지 않는 것이 좋습니다, MAY → ~할
  수 있습니다, REQUIRED → 필수, OPTIONAL → 선택. 괄호로 영문 키워드를 덧붙이지 않는다 —
  명세의 적합성 절(`Boilerplate: omit conformance`)이 없고 300곳이 넘어 가독성을 해친다.
  README가 이 대응을 적는다.
- HTML 문자 참조(`&lt;` `&gt;` `&amp;` `&nbsp;` `&thinsp;`)는 참조 그대로 둔다.
- 문단 머리의 Bikeshed 단축 구문 `Issue:`·`Note:`·`Advisement:`는 영문 접두사를 그대로
  둔다(번역하면 보통 문단이 되어 이슈 색인에서 빠진다).

GFM 경고 마커 규칙은 Bikeshed에 경고 마커가 없으므로 두지 않는다. 원문은 한 줄에 한
문장을 쓰지만(semantic linefeeds) 파서는 문단을 한 세그먼트로 주고 응답은 한 줄이어야
하므로, 번역 문단은 한 줄이 된다. Bikeshed 출력은 같다.

### 2. 용어

`glossary.toml`은 저자가 `docs/`에서 쓰는 한국어를 우선하고, 저자가 영어로 둔 용어는
다음과 같이 정한다: formula 공식, parcel 파셀, leaf/non-leaf block 리프/비리프 블록, critical
필수(모르면 미지원 실패가 나는 종류), non-critical 비필수, implicitly-sized 암묵 크기,
shorthand block 약칭 블록, base 베이스, import 임포트, stack machine 스택 기계. byte
sequence는 저자 표기대로 "바이트열"(띄어쓰기 없음)이다. 실패 종류(invalid archive 잘못된
아카이브, resource failure 리소스 실패, decryption failure 복호화 실패, base failure 베이스
실패, unsupported failure 미지원 실패), 노드 종류, 블록 종류, 등록 해시·사전·모듈도 고정한다.

- 용어 검사는 낱말 그대로 대조하므로 복수형(blocks, formulas, entries, shorthand blocks …)을
  따로 둔다.
- 짧은 일상어와 겹치는 정의(type, size, data, kind, result, root, space, target, match,
  content, extra …)는 오탐으로 재시도를 부르므로 두지 않고 `terms.toml`에 맡긴다.
- 구조 표의 필드 이름(`<td><dfn>size</dfn>` 131개)은 세그먼트 전체가 그 낱말인 식별자다.
  용어와 같은 낱말인 것(blocks, entries, imports, instructions)은 `[whole]` 항목으로 영문을
  고정한다. `checksum` 단수는 필드 이름이면서 표 머리글(`<th>Checksum` → 체크섬)이기도 해
  용어집으로 가를 수 없으므로 두지 않는다. `terms build`가 만든 `[whole]` 항목이 필드
  이름과 같은 낱말(`Type`)이면 지운다.
- 알고리즘 변수 `|node|`·`|entry|`·`|bytes|`는 용어가 아니므로 그 꼴을 `except`에 둔다.
- 문서 제목 "Aqueous Archive Format" → "Aqueous 아카이브 형식"을 고정한다. `<h1>`이
  세그먼트이고 Bikeshed가 `<h1>`을 `Title`로 쓰므로 이 항목이 `<title>`도 정한다.

첫 번역 뒤 `yeokja terms build`로 반복 제목·용어를 고정하고 `glossary adopt --prefer HEAD`
뒤 남은 세그먼트만 다시 번역한다. 정의(dfn) 글자와 `[=term=]` 링크 글자가 같은 번역어를
쓰는지는 파서가 `lt=`로 링크를 지키므로 빌드가 깨지진 않지만, 독자에게는 같은 글자여야
하므로 `terms build` 결과를 dfn 목록과 대조한다.

### 3. 빌드: 같은 Bikeshed로 원문과 번역을 둘 다

`scripts/build.py ROOT TREE`(표준 라이브러리만)가 조립 트리 안에서 다음을 한다.

1. **번역 소스 준비.** 트리의 `spec/index.bs`(번역)에 `<pre class=metadata>` 블록을 덧붙여
   `spec/index.ko.bs`로 쓴다. Bikeshed가 메타데이터 블록을 합치므로 원문을 복사해 고칠
   필요가 없고(원문이 매주 바뀌어 오버레이 사본은 쓸 수 없다), 덧붙이는 키는 뒤의 값이
   이긴다.
   - `Title: Aqueous 아카이브 형식`(번역된 `<h1>`과 같아야 하며 다르면 빌드가 실패한다),
   - `Text Macro: LONGSTATUS 편집자 초안`,
   - `Date:` = `git -C ROOT/upstream log -1 --format=%cs`(스냅샷 커밋 날짜; 빌드 날짜가
     아니다),
   - `Warning: custom`, `Custom Warning Title: 비공식 한국어 번역`, `Custom Warning Text:`
     스냅샷 커밋(해시·날짜·GitHub 링크), 원문 링크, 기계 번역·비공식 고지, 라이선스,
   - `Translation: en https://lifthrasiir.github.io/aqueous/spec/, name 영어 원문,
     native-name English`.
2. **두 번 빌드.** `bikeshed --die-on=warning --die-when=late --no-update spec`으로
   `ROOT/upstream/spec/index.bs` → `build/en.html`, `spec/index.ko.bs` → `build/ko.html`.
   `--no-update`는 CI에서 네트워크 갱신을 막는다(venv 설치 때 `bikeshed update`를 한 번
   한다). 2770행 lint 메시지는 warning이 아니라 통과한다.
3. **`scripts/check.py EN KO`.** 요소 id 집합 동일, 요소 이름 다중집합 동일(번역이 `<public…>`
   같은 태그를 만들면 잡힌다), `<dfn>` 수 동일, 수준별 `<h1>`–`<h6>` 수 동일, 모든 내부
   `href="#…"`가 id로 해소, `bikeshed:` href 없음, `<<`로 시작하는 텍스트 없음. 머리말의
   `Translations`·경고 상자가 번역 쪽에만 있으므로 머리말(`div.head`)은 비교에서 뺀다.
4. **보일러플레이트 지역화.** `build/ko.html`에서 Bikeshed가 영어로 만드는 문자열만 바꾼다.
   각 치환은 정확한 문자열을 정해진 횟수만큼 찾아야 하며, 못 찾으면 빌드가 실패한다 —
   Bikeshed가 올라 문자열이 바뀌면 조용히 영어가 남는 대신 빌드가 멈춘다.
   - 제목: Abstract → 요약, Table of Contents → 목차, Index → 색인, Terms defined by this
     specification → 이 명세가 정의하는 용어, Terms defined by reference → 참조로 정의된
     용어, References → 참고 문헌, Normative References → 규범적 참고 문헌, Informative
     References → 비규범적 참고 문헌, CDDL Index → CDDL 색인, Issues Index → 이슈 색인.
   - 머리말: `Editor:` → 편집자:, `Translations (non-normative):` → 다른 언어 판(비규범적):,
     `<time>` 글자 → `datetime`에서 만든 "2026년 10월 29일".
   - 스크립트 문자열: Jump to Table of Contents, Collapse Sidebar, Pop Out Sidebar, 색인
     패널의 "Referenced in:".
   - CSS 라벨: `</head>` 앞에 `.note::before`/`.example::before`/`.issue::before`의 `content`를
     "참고 "/"예시 "/"이슈 "로 덮는 스타일을 넣는다(카운터는 유지).
   - 경고 상자(`details.annoying-warning[open]`)는 Bikeshed 기본값이 화면 아래에 고정되므로
     머리말 안에 그대로 두는 스타일을 함께 넣는다.
   - `<html lang="en">` → `lang="ko"`.
   저작권 문단(CC0/OWFa)은 법적 문구라 바꾸지 않는다.
5. **산출.** `site/index.html`, `site/LICENSE.txt`(원문 `LICENSE.txt`).

`scripts/entities.py check|fix`는 state에서 원문이 `&lt;`·`&gt;`·`&amp;`를 쓴 세그먼트의
번역이 날것 `<`·`>`·`&`를 쓰면 보고하거나 고친다(원문에 있는 인라인 태그는 보호한다). 코덱이
참조를 되돌려 쓰므로 평소에는 `check`가 0건이며, 빌드는 3단계의 요소 이름 비교와 Bikeshed
경고로 어차피 막히므로 CI에는 넣지 않는다.

빌드 툴체인은 `nix/projects/aqueous.nix`의 `python3 + uv`와 `pythonVenv { requirements =
"requirements.txt"; }`, 프로젝트 `requirements.txt`의 `bikeshed==7.2.0`이다. 셸 훅이
`build/venv`를 만들고 `requirements.txt`가 바뀌지 않으면 재설치하지 않는다.

### 4. 검사

- `status --check upstream/spec`, `evaluate --mechanical-only upstream/spec`(Bikeshed 평가기가
  `[[…]]`·`|var|` 다중집합을, 링크 평가기가 `bikeshed:` 목적지를 본다).
- 빌드가 원문·번역 둘 다 `--die-on=warning`을 통과하고 `check.py`가 통과.
- 브라우저: 머리말(제목·상태·경고 상자·Translations), 표(블록 구조), 노트 안 들여쓴
  목록(3366·3906·5166행), dfn과 자동 링크(패널에 "Referenced in" 지역화), 다크 모드.

색인("이 명세가 정의하는 용어")은 Bikeshed가 dfn의 `lt`(파서가 붙인 원문 용어)로 만들므로
영어 용어로 보인다. 링크는 번역된 정의로 간다. 파서가 `lt="원문|번역"`을 붙이면 번역어로
보일 수 있다(코어 쪽 개선 후보).

## 라이선스

원문은 공개 도메인 헌정이며 그것이 불가능한 곳에서는 MIT-0이다(`upstream/LICENSE.txt`,
2026-10-28). 번역도 같은 조건으로 내고 `site/LICENSE.txt`에 원문 라이선스 파일을 싣는다
(`chip8-book`의 CC0 선례). 머리말 경고 상자가 저작자(Kang Seonghoon), 원문 링크, 스냅샷
커밋, 비공식·기계 번역 고지를 적는다. Bikeshed가 만드는 CC0/OWFa 저작권 문단은 원문 페이지와
같이 둔다. 루트 `LICENSE.md` 표에 한 줄을 더한다.

## 배포

`.github/pages-projects.json`에 `project = "aqueous"`, `target = "html"`을 더하고
`stage-pages.sh`에 `site` → `aqueous` 스테이징을 더한다. `rebuild-translations.sh`는
`status --check upstream/spec` 뒤 `translate upstream/spec`(provider를 부르지 않는다).
랜딩 페이지에는 파일 형식·압축 명세에 맞는 주제가 없어 새 주제 "데이터 형식"(`formats`)을
두고 카드를 놓는다.

## 작업 순서

1. 설계·계획 문서, `yeokja.toml`, `glossary.toml`, `requirements.txt`, nix devShell,
   `.gitignore`.
2. 시험 번역(1–399행 발췌)으로 되돌리기·`lt=`·`|var|`·재시도율·문체를 보고 발췌를
   Bikeshed로 빌드해 원문과 메시지 집합을 비교.
3. 전체 번역 → `entities.py check` → `evaluate --mechanical-only` → `terms build --dry-run` →
   `terms build` → 검토 → `glossary adopt --prefer HEAD` → 남은 세그먼트 번역 →
   `status --check`.
4. `build.py`·`check.py`, `nix develop … build html`, 브라우저 확인.
5. README, 배포 배선(호출 측 공유 파일).

## 되짚을 수 있는 결정

- **RFC 2119 키워드를 한국어 고정 어구로만 옮기는 것.** 독자가 규범성을 되살리려면 README의
  대응표를 봐야 한다. 괄호 병기를 원하면 프롬프트 규칙 한 줄과 재번역으로 바꿀 수 있다.
- **조건 키워드 `Optional`/`If`/`Depends on`을 영문으로 두는 것.** 표의 `[=If=]` 셀과
  정의가 같은 글자여야 하고 식별자처럼 쓰여 영문을 택했다.
- **보일러플레이트를 후처리로 바꾸는 것.** Bikeshed가 번역 훅을 실제로 구현하면 그쪽으로
  옮긴다. 치환은 실패 시 빌드를 멈추므로 Bikeshed를 올릴 때 드러난다.
- **문단이 한 줄로 합쳐지는 것.** 파서가 문장 단위 세그먼트를 주면 원문의 semantic
  linefeeds를 유지할 수 있다.
- **`bikeshed==7.2.0` 고정.** 참조 데이터는 `bikeshed update`로 받으므로 Bikeshed 자체
  버전만 고정한다. 올릴 때는 원문 빌드가 `--die-on=warning`을 통과하는지 먼저 본다.
