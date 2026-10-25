# XHTML 파서 설계

EPUB 책을 번역할 수 있도록 XHTML 콘텐츠 문서를 읽는 `xhtml` 파서를 추가한다.
EPUB 컨테이너(ZIP, OPF, NCX)는 파서가 아니라 프로젝트 파이프라인이 다룬다.
첫 사용처는 `docs/superpowers/specs/2026-10-22-technicalpm-korean-translation-design.md`다.

## 1. 배경

yeokja의 파서는 `DocumentParser::parse(&str) -> Document`로 파일 하나를 읽는다
(`crates/core/src/parser.rs`). 상태(`state/**/*.yeokja.json`), 변경 감지, 재번역,
`status --check`, `reconcile`은 모두 파일 단위다. EPUB은 XHTML 여러 개와 패키지
문서를 담은 ZIP이다.

검토한 방향과 판단:

| 방향 | 판단 |
|---|---|
| A. EPUB 아카이브 하나를 `Document` 하나로 읽도록 트레이트를 확장 | 기각. 챕터 하나를 고쳐도 책 전체가 한 상태 파일이 되어 증분 번역·diff·리뷰가 거칠어진다. 파일 단위 도구 전부에 아카이브 분기가 생긴다. |
| B. pandoc으로 EPUB을 Markdown으로 바꿔 기존 파서로 번역 | 기각. Pressbooks의 `textbox`·캡션·각주 구조와 클래스가 손실되고, 재조립한 EPUB이 원문과 바이트 단위로 대응하지 않는다. |
| **C. EPUB을 풀어 둔 트리의 XHTML을 파일마다 `xhtml` 파서로 읽고, 빌드가 다시 묶는다** | **채택.** 기존 `[[sources]]`·`[derive]`·`[build]`·파일 단위 상태를 그대로 쓰고, 새로 만드는 것은 XHTML 파서 하나다. 같은 파서를 HTML로 된 다른 원문에도 쓸 수 있다. |

## 2. 조사 (Pressbooks EPUB, 2026-10-22)

표본은 *Technical Project Management in Living and Geometric Order*(Pressbooks,
`wisc.pb.unizin.org/technicalpm`)의 EPUB 내보내기다. 18.5MB, 콘텐츠 문서 30개,
1.35MB.

- **EPUB 2.0**이다. OPF `version="2.0"`, 목차는 `toc.ncx`, `nav.xhtml` 없음.
  콘텐츠 문서는 확장자가 `.html`이지만 media-type이 `application/xhtml+xml`이고
  `<!DOCTYPE html PUBLIC "-//W3C//DTD XHTML 1.1//EN" …>`를 선언한다.
- **DTD 엔티티.** `&nbsp;` 370개는 DTD를 읽지 않는 XML 파서에서 "Entity not
  defined" 오류가 난다. 그 밖의 엔티티는 XML 기본 엔티티(`&amp;` 89, `&quot;` 44)뿐이다.
  `&nbsp;`를 문자 참조로 바꾸면 30개 문서가 모두 well-formed다.
- **요소 분포.** `p` 2,046, `li` 1,528, `strong` 1,030, `em` 741, `div` 658, `a` 507,
  `span` 311, `h1`–`h5` 441, `img` 66, `br` 32, `td` 16(표 1개), `sup` 3, `i` 4, `b` 1,
  `code` 1. `pre`·`math`·`svg`·`script`·`iframe`은 없다.
- **혼합 콘텐츠.** 텍스트를 직접 가진 블록은 대부분 `p`·`li`·`h*`지만, `li` 34개는
  글 뒤에 하위 목록을 품고, `div.textbox` 18개와 `div.wp-caption-text` 41개는 `p` 없이
  텍스트와 `br`을 직접 담는다. `li` 하나는 `p`와 맨 텍스트를 함께 가진다. `p` 안에 블록
  요소가 든 경우는 없다.
- **속성.** `style` 171개(대부분 `span`·`div`·`p`). `img`의 `alt` 66개 중 25개가 비어
  있지 않고(그림 설명, oEmbed 썸네일 설명), `title`은 `a` 9·`img` 6·`div` 1개.
- **각주.** 3개 문서에 `<a …><sup class="footnote">[1]</sup></a>` 참조와 문서 끝
  `div.footnotes` 본문이 있다.
- **빈 문단.** `<p>&nbsp;</p>`류 37개.

## 3. 결정

### 3.1 읽기

- 크레이트 `crates/parser-xhtml`, 파서 이름 `xhtml`, `Markup::Xhtml`. 확장자 자동
  감지는 `.xhtml`만 한다. `.html`은 HTML5일 수 있으므로 `[[sources]]`에서 이름으로
  지정해야 한다.
- 크레이트 안의 작은 XML 판독기(`xml.rs`)로 읽는다. 요소마다 시작·끝 태그와 속성
  값의 바이트 범위를 남기고, 짝이 맞지 않는 태그·따옴표 없는 속성·맨 `&`·`<`를
  위치와 함께 거부한다. 번역 크레이트도 세그먼트 조각을 태그화할 때 같은 판독기를
  쓴다. 재구성은 다른 파서처럼 범위를 갈아 끼우는 방식이다. 범위 밖 바이트(선언, DOCTYPE, `head`, 태그, 속성, 공백)는
  원문 그대로 남는다.
- **엔티티.** XML 기본 5개와 문자 참조 외에 **XHTML 1.1 DTD의 이름 엔티티 전체**(HTML
  4 엔티티 252개, `entities.rs`)를 내장 표로 풀어 읽는다. DTD는 내려받거나 읽지 않는다. 표에 없는 이름
  엔티티는 위치를 담은 파싱 오류다(`parse_checked`). 엔티티가 원문에서 어떻게
  쓰였는지는 범위 밖이면 그대로 남고, 범위 안이면 3.3의 직렬화 규칙을 따른다.
- **well-formed가 아니면 실패한다.** HTML5식 복구는 하지 않는다. 깨진 문서를 추측해
  읽으면 재구성 결과도 추측이 되기 때문이다.

### 3.2 번역 단위

이 장의 요소 분류(블록·건너뛰기·코드·번역 속성)는 XHTML 어휘 프로필이다. 같은 파서가
CNXML·PreTeXt 프로필도 읽으며, 프로필 구조와 어휘 공통 규칙(블록을 품은 인라인 요소,
세그먼트 사이 공백)은 `docs/superpowers/specs/2026-10-25-xml-vocabulary-profiles-design.md`가
정한다.

번역 단위는 **블록 수준 요소 안의 인라인 구간**이다. 블록 수준 요소는 XHTML의 흐름
요소(`p`, `h1`–`h6`, `li`, `dt`, `dd`, `td`, `th`, `caption`, `figcaption`,
`blockquote`, `div`, `section`, `aside`, `header`, `footer`, `nav`, `ul`, `ol`, `dl`,
`table`, `tr`, `figure`, `body` 등)이고, 그 밖은 인라인이다.

- 블록 요소의 자식 목록을 훑으며 **텍스트와 인라인 요소가 이어진 구간**을 하나로
  모은다. 블록 자식을 만나면 구간이 끊긴다. 하위 목록을 품은 `li`, 텍스트를 직접 담은
  `div.textbox`, `p`와 맨 텍스트가 섞인 `li`가 모두 같은 규칙으로 처리된다.
- 구간의 앞뒤 공백은 범위에서 뺀다. 글자(Unicode letter)가 하나도 없는 구간(`&nbsp;`만
  있는 빈 문단, 목록 표시만 있는 구간)은 블록을 만들지 않는다.
- `BlockType`은 구간을 담은 요소로 정한다. `h1`–`h6`는 `Heading`(`heading_level`은
  태그 숫자), `li`·`dt`·`dd`는 `ListItem`, `td`·`th`는 `Table`(`BlockRole::TableCell`,
  기존 표 선택 규칙과 같은 필드), `blockquote`는 `BlockQuote`, 나머지는 `Paragraph`다.
- **번역하지 않는 하위 트리:** `head`(단 `title`은 아래 예외), `pre`, `code`가 블록으로
  쓰인 경우, `script`, `style`, `math`, `svg`, `object`, `iframe`, 그리고
  `translate="no"`가 붙은 요소. 블록 하나 전체가 `code`인 경우도 번역하지 않는다.
- **`head`의 `title`** 은 리더 앱이 문서 제목으로 보여 주므로 `Heading`(레벨 없음)으로
  번역한다.
- **속성.** 비어 있지 않은 `alt`(`img`, `area`, `input`)와 `title`(모든 요소)의 값을
  각각 `BlockRole::Literal` 블록으로 번역한다. 범위는 따옴표 안쪽이다. 직렬화는 `&`,
  `<`, 감싼 따옴표를 문자 참조로 쓴다. 그 밖의 속성은 번역하지 않는다.
- **문장 분할.** 구간의 문장 분할(`split_sentences`)은 태그화한 텍스트(3.3)에 대해
  하되, **열린 인라인 요소 안의 분할점은 버린다.** 그래서 모든 세그먼트는 짝이 맞는
  인라인 요소만 담고, 태그화가 언제나 성공한다. Markdown 코덱의 "태그화할 수 없는
  세그먼트" 우회 경로는 이 파서에 필요 없다.

### 3.3 인라인 태그 전송

XHTML은 인라인 구조가 명시적이므로 태그 전송(`docs/superpowers/specs/2026-09-22-inline-tag-transport-design.md`)이
Markdown보다 잘 맞는다. **`xhtml` 소스는 항상 태그 전송을 쓴다.** 원시 XHTML을
모델에 그대로 주는 경로는 두지 않는다. `SourceConfig::uses_inline_tags`가 `xhtml`
소스에서 언제나 참이므로 `inline_tags` 값은 이 파서에 의미가 없다.

| 원문 | 태그 텍스트 | 되돌리기 |
|---|---|---|
| `em`, `i`, `cite`, `dfn`, `var` | `<iN>x</iN>` | 원문의 여는·닫는 태그 |
| `strong`, `b` | `<bN>x</bN>` | 〃 |
| `a`(내용 있음) | `<aN>x</aN>` | 〃 (`href` 등 속성은 원문 그대로) |
| `code`, `kbd`, `samp`, `tt` | `` `x` `` | 원문 바이트(내용 불변) |
| 그 밖의 짝 있는 인라인 요소(`span`, `sub`, `sup`, `abbr`, `q`, `small`, `u`, `s`, `mark`, …) | `<hN>x</hN>` | 원문의 여는·닫는 태그 |
| 빈 요소(`br`, `img`, `wbr`), 각주 참조(`a`가 `sup.footnote` 하나만 담음), 번역하지 않는 하위 트리 | `<xN/>` | 원문 바이트 |

- 태그 문법의 판독(번호 검증, 느슨한 판독, 코드 대조)은 Markdown 코덱의
  `markdown::read`를 그대로 쓴다. 판독은 `Tagged`만 보므로 마크업에 묶여 있지 않다.
  `Dialect::Xhtml`을 두어 `markdown::tagify`·`render`·`prose_of`가 `inline/xhtml.rs`로
  넘기게 하고, 오케스트레이터와 파이프라인은 바꾸지 않는다. Markdown 전용인 번역
  감사(`audit_file`)는 `xhtml`을 건너뛴다.
- 산문에 백틱이 있으면 응답에서 코드 스팬으로 읽히므로 그 세그먼트는 태그화하지
  않는다(`Untaggable`). 표본에는 없다.
- **직렬화는 원문 태그를 그대로 되살린다.** CommonMark의 강조 측면 규칙 같은 제약이
  없으므로 `<i1>외적</i1>에서`도 `<em>외적</em>에서`로 쓰면 된다. Markdown 직렬화기가
  하는 조사 이동, 괄호 밖으로 빼기 같은 보정은 하지 않는다.
- 텍스트는 `&`, `<`, `>`만 문자 참조로 쓰고 나머지는 UTF-8 그대로 쓴다. U+00A0은
  `&#160;`로 쓴다. 직렬화기는 이름 엔티티를 쓰지 않는다. 원문 바이트로 되살리는
  부분(태그, 불투명 요소, 번역 범위 밖)은 원문 표기(`&nbsp;` 등)를 그대로 지닌다.
- 모델이 태그를 빠뜨리거나 지어내면 기존 판독 오류 경로(재시도)를 탄다.

### 3.4 프롬프트와 검사

- 프롬프트는 기존 `INLINE_TAG_RULES`(마크업 중립)를 그대로 쓴다. `closing_rule`의
  `Markup::Xhtml` 분기는 빈 문자열이다(태그 모드에서는 쓰이지 않는다).
- `evaluator_format`의 마크업별 보정(`repair_*`)은 `Xhtml`에 두지 않는다. 구조는
  태그 판독이 보장한다.
- **골격 동치 검사.** 재구성한 문서와 원문을 각각 읽어 텍스트·번역 대상 속성 값을 지운
  구조가 같은지 본다. 블록 요소는 순서와 중첩까지 같아야 한다. 블록 안의 인라인 요소는
  순서와 중첩을 보지 않고 개수까지 같은지(multiset)만 본다. 태그 전송은 한국어 어순에
  맞춰 인라인 요소를 옮기게 하기 때문이다(4장에서 서식만 다른 `span` 두 개의 순서가
  바뀐 사례가 있다). 강조(`em`, `strong` 등)는 둘로 나뉠 수 있으므로 있는지만 본다. 파서 크레이트의 `skeleton()`이
  테스트에서 쓰고, 프로젝트는 같은 비교를 검사 스크립트로 한다. CLI 명령은 두지
  않는다. 쓰는 프로젝트가 늘면 `status --check`에 넣는 안을 다시 본다.

### 3.5 설정

```toml
[[sources]]
path = "upstream/OEBPS"
pattern = "*.html"
parser = "xhtml"
output = "{dir}/{stem}{ext}"
```

`INLINE_TAG_PARSERS`에 `xhtml`을 더한다. 커스텀 `prompt_template`을 쓰면
`{inline_tags}` 자리표시자가 있어야 한다는 설정 검사도 `xhtml` 소스에 적용된다.

## 4. 검증

- **무손실 왕복.** 표본 30개 문서를 번역 없이(원문을 번역으로) 재구성하면 바이트까지
  같다. 태그화 후 판독·직렬화한 세그먼트가 원문 세그먼트와 골격 동치이고, 텍스트 차이는
  엔티티 표기(`&nbsp;`→`&#160;`)뿐이다.
- **커버리지.** `yeokja coverage`에서 번역 대상이 아닌 긴 구간이 모두 마크업으로
  시작한다(`head`, 닫는 `div` 등). 표본 30개 문서에서 블록 4,073개, 세그먼트
  8,858개가 나온다(번역 대상 28개 문서는 8,637개).
- **골격 동치.** 실제 번역 후 30개 문서 모두 골격 동치다.
- **XML 유효성.** 재구성한 문서를 XHTML 1.1 이름 엔티티를 아는 판독기로 읽어 오류가
  없다. `epubcheck`는 원문 EPUB과 같은 항목만 낸다.
- 단위 테스트: 혼합 콘텐츠(`li` + 하위 목록, `div` + 텍스트 + `br`), 열린 인라인 안의
  문장 분할점 억제, 엔티티 표, 속성 값의 따옴표 이스케이프, 각주 참조의 불투명 처리,
  `translate="no"`.

## 5. 범위 밖

- HTML5(비 XML) 문서. 필요해지면 html5ever 기반 별도 파서로 다룬다.
- EPUB 컨테이너 처리(압축 해제, OPF 메타데이터, NCX 목차, 재압축). 프로젝트 빌드
  스크립트가 맡는다. 같은 형식의 프로젝트가 늘면 `yeokja` 하위 명령으로 올린다.
- `ruby`, 양방향 텍스트, 수직 쓰기.

## 6. 되짚을 수 있는 결정

- **`xhtml`은 항상 태그 전송.** 태그 판독 실패율이 높게 나오면 원시 XHTML 경로를 두는
  안을 다시 본다. 판단 기준은 Markdown 롤아웃과 같은 재시도율이다.
- **속성 번역.** `alt`·`title`만 번역한다. `aria-label` 등이 필요한 원문이 생기면
  목록을 설정으로 연다.
- **문장 분할점 억제.** 긴 강조(인용문 전체가 `em`)가 한 세그먼트로 커질 수 있다.
  표본의 세그먼트 길이는 중앙값 120자, 90분위 230자, 99분위 397자이고 가장 긴 것은
  1,461자다. 400자를 넘는 것은 약 1%로, 대부분 Pressbooks가 붙여 넣은
  `<span style=…>`이 문단 전체를 감싼 경우다. 이 정도는 문단 단위로 두고, 길이가
  번역 품질을 해치는 원문이 생기면 서식만 있는 `span`을 쪼개 닫았다 다시 여는 분할을
  검토한다.
