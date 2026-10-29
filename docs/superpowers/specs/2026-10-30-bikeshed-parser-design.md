# Bikeshed 파서 설계

Bikeshed 명세(`Markup Shorthands: markdown yes`)를 번역할 수 있도록 `bikeshed` 파서를
추가하고, 그 바탕인 `parser-markdown-dialect`의 HTML 블록 안 들여쓴 산문 처리를 고친다.
첫 사용처는 lifthrasiir/aqueous의 `spec/index.bs`(Aqueous Archive Format)다.

## 1. 배경

Bikeshed 원문은 HTML 블록(`<h2 id=x>`, `<table class=data>`의 닫지 않은 `<td>`/`<th>`,
`<dl>`, `<div class=note>`, `<ol class=algorithm>`)과 Markdown 문단·목록·강조가 섞인
한 파일이며 한 줄에 한 문장을 쓴다. `parser = "markdown"`, `inline_tags = true`로 읽으면
HTML 블록 안의 산문까지 이미 제공되지만(index.bs 2,664 세그먼트), Bikeshed 고유 구문은
그대로 모델에게 가고 되돌릴 규칙이 없다.

index.bs의 Bikeshed 구문(원문 ed1945c3 기준):

| 구문 | 개수 | 뜻 |
|---|---|---|
| `[=term=]`, `[=for/term=]` | 1,373 | 정의 자동 링크. 표시 글자는 마지막 `/` 뒤 |
| `[=term\|display=]` | 42 | 표시 글자를 따로 적은 자동 링크 |
| `[[#section]]`, `[[#section\|text]]` | 206 / 41 | 절 참조 |
| `[[RFC8949]]`, `[[!AGE]]` | 55 | 참고 문헌 참조 |
| `\|var\|` | 743 | 알고리즘 변수 |
| `<dfn>`, `<dfn for=…>` | 306 | 정의. `lt=`를 가진 것은 없다 |
| `<pre class=metadata>` | 1 | `Abstract:` 줄이 문서 초록 |
| `<pre class=cddl\|biblio\|anchors\|link-defaults\|include-raw>`, `<pre>` | 14 | 리터럴 |

`[=u8=][[=block/size=]]`(배열 표기, `[` + 자동 링크 + `]`)와 `[=u=]([=meta/x=])`(함수
표기)처럼 자동 링크가 대괄호·괄호에 이웃한 꼴이 있다.

그 밖에 CommonMark가 삼키는 산문이 있다. `<div class=note>`나 `<div class=example>`
안에서 2칸 들여쓴 문단 뒤에 빈 줄을 두고 4칸 들여쓴 목록을 쓴 곳(index.bs 3366,
3906, 5166행의 세 노트, 약 310단어)이다. CommonMark에서 HTML 블록은 빈 줄에서 끝나므로
그 목록은 문서 수준의 **들여쓴 코드 블록**이 되어 제공되지 않는다. Bikeshed의
Markdown은 둘러싼 요소를 기준으로 들여쓰기를 읽으므로 목록으로 렌더링한다.

## 2. 결정

### 2.1 `parser-markdown-dialect`: HTML 블록 안 산문의 공통 들여쓰기

`push_html_block`은 HTML 블록 안의 산문 덩어리(태그 사이 텍스트)를 앞뒤 공백만 떼어
CommonMark로 다시 읽었다. 덩어리의 모든 줄이 들여쓰여 있으면 첫 줄만 공백이 벗겨지고
나머지 줄은 들여쓰기가 남아, 첫 줄보다 4칸 이상 더 들여쓴 줄이 코드 블록이나 엉뚱한
중첩 목록이 된다.

- 산문 덩어리를 다시 읽기 전에 **비어 있지 않은 줄의 공통 들여쓰기(공백·탭 바이트
  수의 최솟값)를 줄마다 벗긴다.** 첫 산문 줄은 지금처럼 앞 공백을 모두 벗기고, 앞의
  빈 줄은 건너뛴다. 공통 들여쓰기가 0이면 지금과 같은 텍스트가 읽히므로 기존
  프로젝트의 세그먼트는 바뀌지 않는다.
- 벗긴 텍스트의 블록 범위는 줄마다의 오프셋 표로 원문 오프셋에 되돌린다. 범위가 줄을
  넘으면 원문 범위에는 이어진 줄의 들여쓰기가 포함되며, `normalize_inline_text`가
  이어진 줄의 앞 공백을 접으므로 세그먼트 텍스트는 벗긴 텍스트로 읽은 것과 같다.
  재구성은 원문 범위를 그대로 바꾸므로 바이트가 정확하다.
- 이 처리를 `prose_extras(source, range, preserve_heading_ids) -> Vec<Extra>`로 내놓아
  방언이 HTML 밖의 영역에도 쓸 수 있게 한다(2.2). `html_opaque_ranges(source)`도 함께
  내놓는다(리터럴 요소·주석·CDATA·처리 명령의 범위).

문서 수준의 들여쓴 코드 블록은 여기서 바꾸지 않는다. mdBook 같은 CommonMark 렌더러는
열린 `<div>` 안이라도 빈 줄 뒤 4칸 들여쓴 줄을 코드로 렌더링하므로 번역하지 않는 것이
맞다. 그 규칙은 Bikeshed 파서가 가진다.

### 2.2 `bikeshed` 파서 (`crates/parser-bikeshed`)

`mkdocs`·`rmarkdown`처럼 `parse_with`에 그림자를 주는 방식이되, **그림자 전에 원문을
한 번 고쳐 쓴다.** 자동 링크와 절 참조를 예약된 URL 스킴의 Markdown 링크로 바꾸면
세그먼트·인라인 태그 코덱·링크 평가기가 모두 그것을 보통의 링크로 다루고, 재구성이
끝난 출력에서 되돌린다. `Document.source`와 state의 세그먼트 `source`는 고쳐 쓴
텍스트다(3절).

**고쳐 쓰기(`rewrite`).** 리터럴 영역 밖에서 왼쪽부터 한 번 훑는다. 리터럴 영역은
HTML 리터럴 요소·주석(`html_opaque_ranges`), 펜스 코드 블록, 요소 밖의 들여쓴 코드
블록(아래), 백틱 코드 스팬(같은 길이의 백틱 런으로 닫히는 것)이다. 토큰은 한 줄 안에서
닫혀야 한다.

| 원문 | 고쳐 쓴 꼴 | 되돌리기 |
|---|---|---|
| `[=key\|text=]` | `[text](bikeshed:=KEY\|=)` | `[=key\|번역=]` |
| `[=key=]` | `[disp](bikeshed:=KEY=)`, disp는 key의 마지막 `/` 뒤 | 번역이 disp와 같으면 `[=key=]`, 다르면 `[=key\|번역=]` |
| `[[ref\|text]]` | `[text](bikeshed:[[REF]])` | `[[ref\|번역]]` |
| `[[#id]]`, `[[RFC8949]]`, `[[!AGE]]` | 그대로 | 그대로(글자다) |
| `\|var\|` | 그대로 | 그대로 |

`KEY`·`REF`는 key를 퍼센트 인코딩한 것(영숫자와 `-._~/#:`만 그대로)이라 목적지에 공백·
괄호가 없고 CommonMark가 링크로 읽는다. `[[`의 다음 글자가 `=`이면 `[` + 자동 링크다.
표시 글자에 `[`·`]`·백틱이 있거나 줄을 넘는 토큰은 바꾸지 않는다.

되돌리기(`restore`)는 출력 전체에서 `](bikeshed:`를 찾아 앞의 짝 맞는 `[`까지를 표시
글자로, `)`까지를 목적지로 읽어 표의 규칙으로 쓴다. 번역이 없으면 고쳐 쓰기와 되돌리기가
서로 역함수라 바이트가 같다(`[=foo\|foo=]`는 `\|=`가 붙은 목적지로 구별된다).

**인라인 태그.** `bikeshed`를 `INLINE_TAG_PARSERS`에 넣고 코덱 방언은 CommonMark다.
고쳐 쓴 링크는 `<aN>text</aN>`이 되고 꼬리 `](bikeshed:…)`는 원문 바이트로 돌아온다.
`<dfn>…</dfn>`은 짝 맞는 인라인 HTML(`<hN>`), `|var|`와 그대로 둔 `[[…]]`는 글자다.

**들여쓴 산문.** 그림자를 만들기 전에 원문의 HTML 블록 태그로 열린 요소의 스택을
따라간다(닫는 태그는 같은 이름까지 꺼내고, `li`·`dt`/`dd`·`td`/`th`·`tr`·`p`는 HTML의
생략 규칙대로 같은 종류가 열리면 닫힌다; `<pre>` 등 리터럴 요소 안은 보지 않는다).
스택이 비어 있지 않은 곳의 들여쓴 코드 블록은 산문이다: 그림자에서 공백으로 지우고
`prose_extras`로 읽어 extras로 준다. 스택이 빈 곳의 들여쓴 코드 블록은 CommonMark
그대로 코드다.

**메타데이터.** `<pre class=metadata>`는 그림자에서 공백으로 지우고(그러면 앞의
`<h1>` 줄만 HTML 블록이 된다), `Abstract:` 줄의 값만 `Extra { literal: true, block_type:
Paragraph }`로 준다. 나머지 메타데이터는 범위 밖 바이트라 그대로 남는다.

**`<dfn>` 보존.** 재구성은 세그먼트마다 번역을 끼우기 전에 원문과 번역의
`<dfn …>…</dfn>`을 차례로 짝지어(개수가 다르면 건너뛴다) 안 글자가 달라졌고 여는
태그에 `lt=` 속성이 없으면 ` lt="원문 글자"`를 붙인다. 원문 글자는 고쳐 쓴 링크를
되돌리고 태그를 벗긴 텍스트 내용이며 `&`·`<`·`>`·`"`를 HTML 이스케이프한다. 그래야
`[=원문 글자=]` 자동 링크가 계속 풀린다. `lt=`가 이미 있으면 두지 않는다.

**재구성 순서.** dfn `lt=` 보강 → `resolve_reference_links` → `splice_reconstruct` →
`restore`.

### 2.3 등록과 Markup

- `Markup::Bikeshed`. 번역 쪽에서는 Markdown 계열로 다룬다: 서식 평가기(강조 쌍,
  참조 레이블 제외), 괄호 검사, 용어집 원문, 링크 평가기(`markdown_destinations`가
  `bikeshed:` 목적지를 센다), 강조 수선. 원문 Markdown 모드의 `closing_rule`에 Bikeshed
  규칙(자동 링크는 글자만, `[[…]]`·`|var|` 그대로, 조사는 뒤에)을 적는다. 인라인 태그
  모드에서는 태그 규칙이 대신 가므로 프로젝트가 `|var|` 조사 규칙을 프롬프트에 둔다(5절).
- `PARSER_NAMES`에 `bikeshed`, 확장자 `.bs` 자동 감지.
- **Bikeshed 평가기**(`BikeshedEvaluator`, `evaluators_for`에 등록, 기계적 검사라
  `evaluate --mechanical-only`에서도 돈다): `Markup::Bikeshed` 세그먼트에서 코드 스팬
  밖의 그대로 둔 참조 `[[…]]`(`|` 없는 것)와 변수 `|var|`(영숫자·`_`와 한 칸 공백)의
  다중집합이 원문과 다르면 빠진 것과 더한 것을 알려 재시도하게 한다. 고쳐 쓴 자동
  링크·절 참조의 키는 링크 평가기가 이미 목적지로 대조하므로 다시 세지 않는다.

## 3. 한계

- state의 세그먼트 `source`와 `Document.source`는 고쳐 쓴 텍스트라 `[=u8=]`가
  `[u8](bikeshed:=u8=)`로 보인다. 원문 파일과 출력은 Bikeshed 구문이다.
- 번역이 표시 글자에 `]`를 넣으면 되돌리기가 그 앞의 `[`에서 끊어 링크가 깨진다.
  인라인 태그 모드에서는 직렬화기가 꼬리를 보장하므로 표시 글자만 문제다.
- `<dfn>` 짝짓기는 차례이며, 번역이 dfn 개수를 바꾸면 `lt=`를 더하지 않는다
  (인라인 태그 모드에서는 `<hN>` 검증이 개수를 지킨다).
- 블록 구조는 CommonMark다. Bikeshed가 더 받아 주는 꼴(요소 안 들여쓰기 외)은 다루지
  않는다. 요소 스택은 HTML 블록 태그만 보므로 문단 안의 인라인 HTML로 연 요소는
  세지 않는다.
- `|var|`를 `||`로 쓴 절댓값 표기는 없다고 본다.

## 4. 검증

- `parser-markdown-dialect`: 2칸 들여쓴 노트 안의 4칸 목록이 제공되고 재구성이 바이트
  단위로 같음, 공통 들여쓰기 0인 덩어리의 세그먼트가 이전과 같음(기존 테스트).
- `parser-bikeshed`: index.bs 발췌 픽스처(메타데이터, 표, dl, 노트, dfn, 자동 링크,
  참조, 변수, cddl pre, 요소 안 들여쓴 목록)로 무번역 왕복 바이트 동일, 고쳐 쓰기의
  각 꼴과 이웃한 대괄호·괄호, 리터럴 영역 건너뛰기, 되돌리기의 세 규칙, Abstract
  Literal, 요소 안·밖의 들여쓴 코드, `<dfn>` `lt=` 보강(이스케이프, 이미 있는 lt).
- 코덱: 고쳐 쓴 링크의 태그화와 직렬화 왕복.
- 평가기: 참조·변수 다중집합(빠짐, 추가, 조사 붙음, 코드 스팬 안 제외).
- 실측: `projects/aqueous/upstream/spec/index.bs`를 `status`/`coverage`로 읽어 세그먼트
  수와 세 노트의 제공을 확인하고, 기존 Markdown 계열 프로젝트의 `status --check`가
  변경 0임을 확인한다.

실측(index.bs, 원문 ed1945c3):

| 구성 | 세그먼트 |
|---|---|
| `markdown`, 공통 들여쓰기 전 | 2,664 |
| `markdown`, 공통 들여쓰기 후 | 2,674 (`<ol class=algorithm>`의 중첩 번호 `1.`이 산문에서 빠진다) |
| `bikeshed` | 2,471 (원문 3d37dae5; 세 노트의 목록과 Abstract, `: `/`:: ` 정의 목록 항목이 더해지고, HTML 블록 안 산문의 인라인 태그(`<sub>`·`<dfn>`·`<a>`)가 세그먼트를 끊지 않아 하나로 남는다; `bikeshed:` 링크를 담은 것 825개) |

coverage는 글자가 있는 4,591줄의 71%이며 남은 긴 구간은 `<pre class=cddl>`·`biblio`·
메타데이터다. index.bs 전체가 무번역 왕복에서 바이트 동일하고, 모두 번역해도 `bikeshed:`
링크가 남지 않는다(`YEOKJA_BIKESHED_FILE`로 돌리는 무시 테스트). Markdown 계열 17개
프로젝트(`markdown`·`myst`·`mdx`·`mkdocs`·`rmarkdown`)의 `status --check`는 이전 바이너리와
같은 세그먼트 수에 변경 0이다.

## 5. 설정 예시

```toml
[[sources]]
path = "upstream/spec"
pattern = "*.bs"
parser = "bikeshed"
output = "ko/spec/{path}"
inline_tags = true
```

프롬프트에 둘 규칙(인라인 태그 모드에서는 `closing_rule`이 가지 않는다): "`|var|`는
변수 이름이라 글자 그대로 두고 조사는 닫는 `|` 뒤에 붙인다(`|n|을`). `[[#section]]`,
`[[RFC8949]]` 같은 이중 대괄호 참조는 그대로 둔다. 링크 글자가 정의 용어이면 용어집의
번역을 쓴다."
