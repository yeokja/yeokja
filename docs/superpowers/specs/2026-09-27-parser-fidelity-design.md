# 파서 충실도 개선 설계

배포본 렌더링 감사에서 번역문이 아니라 **파서와 재구성**이 원문의 구조를 바꾼 사례가
여섯 갈래로 드러났다. 번역을 고쳐서는 없어지지 않고, 같은 원문 모양이 새로 들어오면 다시
생긴다. 이 문서는 각 결함의 원인이 된 코드와 고치는 방법, 그리고 저장된 상태(state)에 주는
영향을 정한다. 공통 원칙은 둘이다.

- **원문 렌더러가 읽는 방식에 맞춘다.** docutils, Asciidoctor, pulldown-cmark(CommonMark),
  Markdeep이 원문을 어떻게 읽는지가 기준이다. "보기 좋은" 규칙을 새로 만들지 않는다.
- **이미 있는 번역을 다시 번역하지 않는다.** 원문 텍스트가 그대로인 세그먼트는 ID가 바뀌어도
  `source_hash`로 짝지어진다(`crates/core/src/reconcile.rs`). 문제는 이웃 세그먼트가 늘거나
  줄어 `context_hash`가 달라진 세그먼트(ContextChanged)다. 지금은 `translate`가 이것을 조용히
  다시 번역하고 `status --check`가 실패로 센다. 파서 이관용 명령(8절)으로 이것을 모델 호출
  없이 받아들인다.

## 1. AsciiDoc: 구분자 없는 `[source]` 문단

**현상.** theBeamBook `chapters/beam_loader.asciidoc`에는 `[source, C]` 속성 줄 바로 밑에
구분자(`----`) 없이 코드를 적은 곳이 10곳 있다(196, 212, 221, 227, 248, 268, 276, 282, 309,
316행). Asciidoctor는 스타일이 `source`인 문단을 목록(listing) 블록으로 읽고 줄을 그대로
둔다. `crates/parser-asciidoc/src/lib.rs`의 파싱 루프는 `[...]` 줄을 건너뛰기만 하므로 이
문단을 산문으로 읽어 한 세그먼트로 만들고, 재구성이 줄바꿈을 공백으로 합친다. 두 줄 이상인
5곳(196: 4줄, 248: 8줄, 282: 2줄, 309: 3줄, 316: 13줄)의 코드가 배포본에서 한 줄로 나온다.
예: state `section:0/block:62/seg:0`
`BeamInstr* next; PreFetch(2, next); MoveCall(Arg(0)); NextPF(2, next);`,
`dist/site/index.html`의 `<code data-lang="C">`도 한 줄이다. 나머지 5곳은 한 줄짜리라 보이는
결함은 없지만 코드가 번역 대상에 들어가 있다.

**결정.** 블록 속성 줄의 첫 위치 속성(스타일)이 문자 그대로 두어야 하는 스타일이면, 바로
다음 줄이 구분자가 아닐 때 그 문단(다음 빈 줄까지)을 불투명 `CodeBlock`으로 둔다.

- 스타일 목록: `source`, `listing`, `literal`, `pass`, `stem`, `latexmath`, `asciimath`,
  그리고 도표 스타일 `ditaa`, `graphviz`, `plantuml`, `mermaid`, `svgbob`. 이 프로젝트의
  Asciidoctor 확장이 받아들이는 도표 이름 가운데 문단 형태로 쓰일 수 있는 것들이다.
- 스타일은 `[` 뒤 첫 쉼표 앞의 토큰이다(`[source, C]`, `[source,erlang]`, `[listing]`).
  `#id`·`.role`·`%option` 줄임 표기가 붙은 토큰(`[source#ex1]`)은 `#`/`.`/`%` 앞까지를 스타일로
  본다. `[[anchor]]`, `[NOTE]`, `[cols=...]`처럼 목록에 없는 스타일은 지금처럼 다룬다.
- 속성은 바로 다음 블록에만 붙는다. 기존 코드가 빈 줄에서 `declared_columns`를 버리는 것과
  같이, 빈 줄을 만나면 대기 중인 스타일도 버린다. 연속된 속성 줄(`[[id]]` 다음 `[source]`)은
  마지막 스타일이 이긴다.
- 다음 줄이 여는 구분자면 지금처럼 구분자 블록 처리가 맡는다(이미 불투명).
- `[verse]`·`[quote]` 문단은 이 결정에 넣지 않는다. 산문이지만 줄 구조가 의미를 가지므로
  따로 설계가 필요하고, 현재 AsciiDoc 프로젝트(theBeamBook)에는 없다.

**영향.** theBeamBook 세그먼트 10개가 state에서 빠지고(모두 원문 그대로 둔 번역이었다) 이웃
세그먼트 약 20개가 ContextChanged가 된다. 8절 명령으로 받아들이면 다시 번역하는 세그먼트는
없다.

## 2. reStructuredText: 도메인 지시자와 graphviz 본문

### 2.1 이름에 `:`가 든 지시자의 본문이 번역되지 않음

**현상.** `parse_directive`(`crates/parser-rst/src/lib.rs:688`)는 이름에 영숫자와 `-_.+`만
허용한다. `.. py:function::`, `.. c:function::` 같은 Sphinx 도메인 지시자는 지시자로
인식되지 않고 "대상·치환·주석" 분기로 떨어져, 들여 쓴 본문 전체가 원문 그대로 남는다.
pypy `objspace.rst`의 `getexecutioncontext()` 설명("Return the currently active execution
context.") 등이 배포본에서 영어다. 원문의 도메인 지시자 수: pypy `py:function` 43,
`py:data` 14, `py:attribute` 5, `py:method` 1; peps `c:function` 27, `c:member` 8, `c:type` 3,
`c:struct` 3; cpython-devguide `c:function`·`c:member`·`c:var` 각 1.

**결정.** 지시자 이름에 `:`를 허용한다(docutils의 지시자 이름 문법과 같음: 영숫자로 시작하고
영숫자와 `-_.+:`로 이루어짐, 마지막이 `:`가 아님). 그러면 도메인 지시자는 이름 없는 지시자와
같은 길을 탄다: 표지 줄(시그니처)은 원문 그대로, 본문은 번역한다. 이미 있는
`function_directive_body_is_translatable_but_signature_is_not` 테스트가 기대하는 동작이다.
`OPAQUE_DIRECTIVES`·`INLINE_PROSE_DIRECTIVES` 비교는 전체 이름(`py:function`)으로 한다.

### 2.2 graphviz 본문이 산문으로 합쳐짐

**현상.** `.. graphviz::`(pypy 10곳)와 `.. digraph::`(rpython `translation.rst:616`) 본문이
산문으로 읽혀 한 줄로 합쳐진다. `translation.rst`의 DOT 본문에는 `// Invisible edge ...`
주석 줄이 있어서, 합쳐진 줄에서 이 주석이 뒤따르는 간선 정의
(`"Input or Output" -> "Transformation Step" [style=invis]`)까지 삼킨다. 모델이 주석을
번역한 것도 확인된다(`ko/rpython/doc/translation.rst:385`).

**결정.** `graphviz`, `graph`, `digraph`(sphinx.ext.graphviz의 세 지시자)를
`OPAQUE_DIRECTIVES`에 넣는다. 도표 안의 영어 레이블은 원문 도표의 일부로 둔다.

**영향(2절 전체).** 도메인 지시자 본문이 새 세그먼트(Pending)가 된다. 추정 규모는 pypy
약 70, peps 약 45, cpython-devguide 3 문단이며 정확한 수는 8절 명령의 미리보기로 센다.
graphviz 본문 세그먼트 11개는 빠진다. 새 세그먼트는 번역해야 하고, 이웃의 ContextChanged는
8절 명령으로 받아들인다.

## 3. reStructuredText: 재구성이 줄을 합쳐 docutils의 해석이 바뀜

재구성은 여러 줄 문단을 번역문 한 줄로 바꾼다. docutils는 대부분 이를 같은 문단으로 읽지만,
**문단의 둘째 줄이 첫 줄의 뜻을 정하는** 두 경우에는 해석이 달라진다.

### 3.1 첫 줄이 목록 표지처럼 보이는 문단

**현상.** docutils는 표지로 시작하는 줄이라도 다음 줄이 들여 쓰이지 않은 채 이어지면
목록이 아니라 문단으로 읽는다("A. Einstein was ..." 규칙). 한 줄로 합쳐지면 이 판정 근거가
사라져 목록이 된다.

| PEP | 원문 | 배포본 |
|---|---|---|
| pep-0421 `section:31/block:5,7,9` | `* (2009) Dino ... opinion:` + 다음 줄 URL, `.. [#…] (2012) ...` | `<ol start=2009>` 등, `(2009)`가 사라짐 |
| pep-0485 `section:24/block:9` | `.. [4] 1976. R. H. Lathwell. ...` + 다음 줄 | `<ol start=1976>` 안에 `R.`·`H.` 중첩 목록 |
| pep-0497 173–206행 | `a. It should ...` + 다음 줄(두 목록, 6항목) | 알파벳 목록 2개(원문은 문단) |

번역이 원문에 없던 표지로 시작해도 같은 일이 생긴다: pep-0336 `section:4/block:1/seg:0`
(`(1) 호출하기…`), pep-0513 `section:6/block:7/seg:0`(`(b) …`).

**결정 — 첫 줄 이스케이프.** 재구성한 문단(목록 항목·각주·필드 본문의 첫 문단 포함)의 첫
줄을 docutils가 블록 시작으로 읽는데, 원문 문단은 그렇게 읽히지 않았다면 번역문 맨 앞에
`\`를 붙인다. `\(2009)`는 docutils에서 `(2009)`로 보이고, 목록 판정은 일어나지 않는다.

- 판정 함수 `reads_as_block_start(first_line, next_line)`는 docutils 규칙을 따른다.
  글머리표(`-`, `+`, `*`, `•`, `‣`, `⁃` 뒤 공백), 열거 표지(아라비아 숫자·한 글자
  알파벳·로마 숫자·`#`에 `.`/`)`, 또는 `( )`로 감싸고 공백이나 줄 끝), 필드 표지
  (`:이름:` 뒤 공백), 옵션 표지(`-x`, `--xx` 뒤 공백 둘 또는 인자), 줄 블록(`| `),
  doctest(`>>> `)를 블록 시작으로 본다. 열거 표지는 `next_line`이 없거나 비었거나 더
  들여 쓰였을 때만 블록 시작이다(docutils의 두 번째 줄 검사).
- 원문 쪽은 실제 다음 줄로, 번역 쪽은 재구성 결과(한 줄이면 `next_line` 없음)로 판정한다.
  "번역만 블록 시작"일 때만 이스케이프한다. 원문도 목록으로 읽히던 모양(예: `* 1. foo`처럼
  한 줄짜리 중첩 목록)은 건드리지 않는다.
- `repair_korean_reference_boundaries`처럼 재구성의 마지막 단계에서 바이트만 더하므로
  state는 바뀌지 않는다.

### 3.2 각주·지시자 본문의 들여쓰기가 둘째 줄로 정해지는 경우

**현상.** docutils는 명시적 마크업(각주 `.. [1]`, 인용, 첫 줄에 내용이 있는 지시자)과 필드의
본문 들여쓰기를 **둘째 줄부터의 최소 들여쓰기**로 정한다. pep-3144 각주 `[1]`(159행)은 둘째
줄이 7칸이고 뒤따르는 이메일 인용이 10칸이라 인용이 `block_quote`가 된다. 첫 문단이 한 줄로
합쳐지면 최소 들여쓰기가 10칸이 되어 이메일이 평범한 각주 문단으로 바뀐다
(`ko/peps/pep-3144.rst:111–121`).

**결정 — 이어짐 줄 유지.** 이런 본문의 첫 문단이 원문에서 두 줄 이상이었고, 같은 본문 안에
그 이어짐 열(`Run::text_col`)보다 더 들여 쓴 줄이 뒤따르면, 번역문을 두 줄로 나눠 둘째 줄을
원래 이어짐 열에서 시작한다.

- 파서가 `Run`을 명시적 마크업·필드 표지에서 시작할 때 이 조건을 판정해, 블록에 새
  `BlockRole::ContinuationColumn { column }`을 단다. 역할은 재구성에만 쓰이고 세그먼트 ID와
  해시에는 들어가지 않는다.
- 나눌 위치는 번역문에서 인라인 마크업(백틱 구간, `|치환|`, `[#]_` 같은 참조) 밖의 공백 가운데
  원문 첫 줄의 표시 폭에 가장 가까운 곳이다. 둘째 줄이 문장 부호만으로 이루어지면(제목 밑줄로
  읽힘) 그 위치는 쓰지 않는다. 쓸 수 있는 공백이 없으면 나누지 않는다(지금 동작).
- 나누는 규칙은 parser-utils의 공통 도우미 `split_at_safe_space`(5절과 공유)로 둔다.

**영향(3절 전체).** 재구성만 바뀌므로 다시 번역할 세그먼트는 없다. `ko/`의 해당 줄만 바뀐다.

## 4. reStructuredText: 목록과 제목 인식

### 4.1 한 글자 알파벳·로마 숫자 열거 표지

**현상.** `parse_enumerated_item`(`lib.rs:595`)은 "J. Smith" 같은 머리글자와 헷갈릴까 봐
한 글자 알파벳 뒤 `.`를 표지로 받지 않고, 로마 숫자는 아예 모른다.

- pep-0818 682–686행: `a.`/`b.`/`c.` 아래 `i.   Set ...`, `ii.  Set ...`, `iii. Set ...`가
  한 항목으로 합쳐진다(`section:14/block:26/seg:1`이 `Set ... ii.`처럼 다음 표지를 문장 끝에
  붙인 모양). 알파벳 표지는 `a.`가 따로 세그먼트가 된다(`block:23/seg:0`).
- pep-0209 70–115행: `a.  Use a local coercion model`처럼 표지 뒤 공백이 둘이다. 우회 코드
  `indented_alpha_enumeration_edit`(`lib.rs:1407`)가 표지를 `a. `(공백 하나)로 다시 쓰면서
  본문 열(8칸)이 항목 텍스트 열(7칸)보다 깊어져, 설명 문단이 인용문(`block_quote`)으로, 여러 줄
  항목이 정의 목록으로 바뀐다.

**결정.** docutils와 같은 규칙으로 열거 표지를 받는다.

- 표지 종류: 아라비아 숫자, `#`, 한 글자 알파벳(대·소문자), 로마 숫자(대·소문자
  `ivxlcdm` 조합). `i`/`I` 한 글자는 같은 들여쓰기에서 바로 앞 표지가 `h`/`H`면 알파벳, 아니면
  로마 숫자다(docutils의 순서 판정과 같음).
- 한 줄이 표지로 시작하면, 다음 줄이 없거나 비었거나 표지 뒤 텍스트 열 이상으로 들여 쓰였거나
  **같은 들여쓰기에서 다음 순번 표지로 시작할 때만** 목록 항목이다. 그 밖(다음 줄이 표지와
  같은 열에서 텍스트로 이어짐)은 문단이다 — 3.1절의 판정과 같은 함수를 쓴다.
- 항목 텍스트는 표지 뒤 공백 전체를 건너뛴 곳에서 시작하므로 `a.  `, `i.   `의 공백은 원문
  바이트로 남는다.
- `indented_alpha_enumeration_edit`와 `alpha_enumerator`를 없앤다. 이 우회가 하던 "항목마다
  제 줄 유지"는 항목이 각자 블록이 되면서 저절로 된다.
- 기존 테스트 `an_initial_is_not_a_list_marker`는 "다음 줄이 이어지는 머리글자는 문단"으로
  바꾼다. 한 줄짜리 `J. Smith wrote this.` 뒤가 빈 줄이면 docutils도 목록으로 읽으므로
  그렇게 맞춘다.

### 4.2 한 글자 제목 밑줄

**현상.** pep-0450 175–176행 `R` / `-`는 docutils에서 절 제목이다(Text 상태에서 다음 줄이
구두점 한 글자 반복이면 밑줄). `adornment_char`(`lib.rs:561`)는 두 글자 이상만 장식으로
받으므로 `R -` 문단이 되고, 목차에서 R 절이 빠진다.

**결정.** 한 줄 텍스트 바로 밑의 장식 줄은 한 글자도 밑줄로 받는다. 길이 판정은 기존
`adornment_covers`(4 이상이거나 제목 폭 이상)를 그대로 쓴다. 윗줄 장식과 전환선(transition)
판정은 두 글자 이상 규칙을 유지한다. 저장소 전체 RST 원문에서 이 모양은 pep-0450 한 곳뿐이다.

### 4.3 glossary의 여러 줄 용어

**현상.** `.. glossary::` 안에서는 들여 쓴 정의 바로 위의 연속된 줄이 **각각** 용어다.
pep-0639 207, 217, 230행(`license expression`/`SPDX expression` 등 3쌍)이 한 세그먼트
(`section:7/block:4/seg:0` `license expression SPDX expression`)로 합쳐져 한 용어가 되고,
`:term:`license expression``이 이 문서의 용어집 대신 intersphinx로 packaging.python.org에
연결된다.

**결정.** `glossary` 지시자 본문에서 정의보다 얕은 줄은 줄마다 제목 성격의 블록(용어)으로
둔다. 이름에 `:`를 허용하는 2.1절 변경과 무관하게, `glossary`만 이 규칙을 탄다. 저장소
전체에서 여러 줄 용어는 위 3쌍뿐이다.

**영향(4절 전체).** 원문 텍스트가 바뀌는 세그먼트만 다시 번역한다: pep-0818의 열거 항목
(`block:23`, `25`, `26`, `27`의 약 12 세그먼트), pep-0209의 알파벳 항목, pep-0450의 `R`,
pep-0639의 용어 6개. 표지를 새로 인식해 텍스트가 바뀌는 다른 문서가 있는지는 8절 미리보기로
확인하고, 계획에 수를 적는다.

## 5. Markdeep: 한 줄로 합쳐진 인용문

**현상.** Markdeep은 `>`로 시작하는 줄이 **두 줄 이상** 이어질 때만 인용문으로 만든다
(정규식 `(?:\n>.*){2,}`). raytracing `RayTracingInOneWeekend.html` 37–40행의 네 줄 인용문이
한 줄 번역으로 재구성되어(`ko/books/RayTracingInOneWeekend.html:25`) 배포본에서 평범한
문단이 된다(state `section:1/block:4/seg:0..3`).

**결정.** parser-markdeep의 재구성에서, 원문이 두 줄 이상인 인용문 블록은 번역문을 원문 줄
폭(가장 긴 원문 줄의 표시 폭)에 맞춰 여러 줄로 나누고 줄마다 원문의 인용 접두어(`> `)를
붙인다. 나눌 위치는 3.2절의 `split_at_safe_space`로 고르되, 안전 판정은 pulldown-cmark가
번역문에서 `Text`로 읽은 구간 안의 공백이다(코드 스팬·링크 대상·자동 링크 안에서는 나누지
않는다). 나눌 곳이 없어 한 줄이 되면 빈 인용 줄(`>`)을 하나 덧붙여 두 줄 조건을 채운다.

**영향.** 재구성만 바뀐다. 네 책 원문에서 인용문은 이 한 곳이다.

## 6. Markdown: 인라인 강조 안의 강제 줄바꿈

**현상.** putting-the-you-in-cpu 6장 76–80행의 대화체 인용문은 줄 끝 공백 두 개로 줄을
바꾸는데, 전체가 `*...*` 강조 하나에 싸여 있다. parser-markdown-dialect는 인라인 컨테이너가
시작될 때 그 **전체 범위**로 run을 늘리므로(`Tag::Emphasis` 분기), 강제 줄바꿈에서 run을
닫아도 이미 컨테이너 끝까지 늘어난 뒤다. 결과는 한 블록 4 세그먼트(`section:1/block:5`)이고
재구성이 줄바꿈을 공백으로 합친다. 강조 밖의 강제 줄바꿈은 이미 블록을 나눠 보존된다
(`parser-markdown`의 `hard_break_preserved` 테스트).

**결정.** 강제 줄바꿈이 run 안쪽(run 끝이 줄바꿈보다 뒤)에 오면, run을 줄바꿈 시작에서 잘라
블록으로 내보내고, 잘린 뒤쪽 끝(컨테이너 끝)을 기억했다가 다음 텍스트 이벤트에서 새 run을
그 끝까지 늘린다. 줄바꿈 바이트(`  \n> ` 등 다음 줄 접두어 포함)는 어느 span에도 들지 않아
원문 그대로 남는다. 줄마다 블록이 되며, 첫 줄 블록은 여는 `*`를, 마지막 블록은 닫는 `***`를
가진다(지금도 세그먼트가 이렇게 나뉘어 번역되고 있다). 새 BlockRole이나 core 모델 변경은
없다. dialect를 쓰는 markdown·mdx·myst·mkdocs·markdeep 파서에 모두 적용된다.

**영향.** 이 인용문은 5블록이 되며 앞 세 줄은 원문이 같아 번역을 그대로 쓴다. 넷째 세그먼트
`B: Interrupting cow wh — A: **MOOOOO!***`가 `B: Interrupting cow wh —`와
`A: **MOOOOO!***` 두 세그먼트로 바뀌어 다시 번역한다. 다른 프로젝트는 원문에서 줄 끝 공백
두 개가 강조 안에 있는 곳을 8절 미리보기로 센다(대략적인 텍스트 검색으로 cp-algorithms 22,
rustc-dev-guide 3, learn-fpga 1줄이 줄 끝 공백 후보지만, 대부분 강조 밖이라 이미 보존된다).

## 7. LaTeX: `\addcontentsline`의 차례 문구

**현상.** `is_layout_command`(`crates/parser-latex/src/lib.rs:731`)가 `addcontentsline` 줄
전체를 건너뛰므로, jeffe-algorithms 서문·후기의 `\addcontentsline{toc}{...}{Preface}` 등
10곳(`frontmatter/preface.tex` 8, `backmatter/credits.tex`·`colophon.tex` 각 1)의 차례·PDF
책갈피 문구가 영어로 남는다. 본문 제목과 쪽 머리글(`\markboth`)은 이미 번역된다. HoTT는 같은
문제를 `scripts/prepare_pdf.py`의 문자열 치환(`Index of symbols` → `기호 찾아보기`,
`Introduction` → `서론`)으로 우회하고 있다.

**결정.** `\addcontentsline{…}{…}{TEXT}`의 셋째 인자를 제목(`Heading`) 블록으로 번역한다.
앞 두 인자와 명령은 원문 그대로다. 셋째 인자가 매크로 하나뿐이면(`\ExercisesName`,
`\bibname`) 번역하지 않는다. `addcontentsline`은 `is_layout_command`에서 빼고, 인자 위치를
고르는 새 표(`TOC_TEXT_COMMANDS`)로 처리한다. 주석 줄(`%\addcontentsline`)은 지금처럼 무시된다.

**영향.** jeffe 10, HoTT 2(symbols.tex, introduction.tex)의 새 세그먼트(Pending). 본문 제목과
같은 영어 원문이므로 용어가 어긋나지 않게 같은 문서의 제목 번역을 그대로 쓴다(계획의 이관
단계에서 제목 세그먼트의 번역을 복사). HoTT는 새 세그먼트의 번역을 지금 치환이 넣는 문자열로
맞춘 뒤 `prepare_pdf.py`의 두 치환과 그 테스트를 지운다.

## 8. 파서 이관 명령: `yeokja reconcile`

파서가 바뀌면 원문은 그대로여도 세그먼트 모양이 바뀐다. 지금 도구로는 `translate`가
ContextChanged 세그먼트까지 모두 다시 번역하므로, 1–7절을 적용하면 이웃 수백 세그먼트가
이유 없이 재번역된다.

**결정.** `yeokja reconcile <path> [--dry-run]`을 추가한다.

- 각 파일을 현재 파서로 다시 읽어 기존 state와 `reconcile`(해시 짝짓기)한 결과를 저장한다.
  원문이 같은 세그먼트는 새 ID로 번역·용어집 스냅숏·시각·이슈를 그대로 가져가고,
  **ContextChanged만 해당하는 세그먼트는 `context_hash`를 현재 값으로 바꾼다.** 새 세그먼트는
  Pending으로 들어가고, 짝이 없는 옛 세그먼트는 state에서 빠진다.
- Stale(원문이 바뀜)과 GlossaryStale은 건드리지 않는다. 파서 이관은 원문과 용어집을 바꾸지
  않으므로 이 둘이 나오면 이관과 무관한 기존 문제다.
- 모델을 부르지 않는다. `--dry-run`은 파일별로 짝지음·새 세그먼트·빠질 세그먼트·받아들일
  문맥 변경 수와 새·빠질 세그먼트 원문을 출력만 한다. 계획의 영향 수치는 이 출력으로 확정한다.
- 쓰임새는 파서 변경 직후 한 번이다. 원문 갱신(서브모듈 올림) 뒤에는 쓰지 않는다 — 그때의
  ContextChanged는 실제 문맥 변화일 수 있다. 이 구분을 명령의 도움말과
  `docs/translation-project-checklist.md`에 적는다.

## 9. 순서와 보류 작업과의 관계

- peps에는 보류 중인 합성(`yeokja -C projects/peps fuse upstream/peps --previous 24aa727`)이
  있다. 합성은 이전 리비전 state와 세그먼트를 짝지으므로, **peps 합성을 먼저 끝낸 뒤** 2–4절의
  peps 이관을 한다. 순서가 바뀌면 합성이 새 세그먼트 모양과 옛 state를 짝짓지 못한다.
- 커밋은 파서 변경(코드+테스트)과 프로젝트별 이관(state·`ko/`)을 나눈다. 이관 커밋은
  `yeokja reconcile` 결과와 새 세그먼트 번역을 함께 담는다.
- push 전에 `.github/pages-projects.json`의 모든 프로젝트에 대해
  `.github/scripts/rebuild-translations.sh`를 돌려 `status --check`가 통과하는지 본다
  (한 곳의 남은 세그먼트가 Pages 배포 전체를 실패시킨다).

## 10. 다루지 않는 것

- pep-0485 각주의 `1976.`/`R.`/`H.`가 따로 세그먼트가 되는 문장 분리 문제. 3.1절 이후
  렌더링은 원문과 같아지고, 번역 품질에 영향이 없다.
- AsciiDoc `[verse]`/`[quote]` 문단의 줄 구조(1절).
- Markdown 전체에서 여러 줄 문단의 줄 구조 보존. 5·6절처럼 렌더러가 줄 구조에 의미를 두는
  경우만 다룬다.
- 빌드 설정 문제(rust-forge·rustc-dev-guide 리다이렉트, PDF 레이블 현지화 등)와 번역문
  수정은 별도 작업이다.

## 11. 검증

- **단위 테스트** — 각 절의 원문 조각을 그대로 옮긴 파서 테스트: 세그먼트 목록과 재구성
  결과(번역 적용 전후)를 단언한다. 3.1절은 원문 목록/번역 목록/둘 다 목록인 세 경우, 3.2절은
  나눌 곳이 없는 경우와 둘째 줄이 구두점뿐인 경우를 포함한다. 8절은 빠짐·새로 생김·문맥만
  바뀜이 섞인 state로 저장 결과를 단언한다.
- **말뭉치 왕복** — 번역 없이 재구성하면 원문과 바이트가 같아야 한다(기존 왕복 테스트를 모든
  RST·AsciiDoc·Markdown 계열 프로젝트 원문에 돌린다).
- **렌더러 대조** — peps·pypy·cpython-devguide의 `ko/`와 원문을 docutils로 읽어 블록 노드
  종류별 개수(enumerated_list, bullet_list, list_item, block_quote, definition_list, section,
  glossary 용어)를 문서마다 비교한다. 감사에서 지적된 8개 PEP의 차이가 사라지고 새 차이가
  생기지 않아야 한다.
- **빌드 결과** — theBeamBook 배포 HTML의 `<code data-lang="C">`가 원문 줄 수를 유지,
  pypy `objspace.html`의 지시자 설명이 한국어이고 `translation.html` 도표에 범례 간선이 있음,
  raytracing HTML의 인용문이 `<blockquote>`, putting-the-you-in-cpu의 대화 인용문에 `<br>` 4개,
  jeffe PDF 차례(`pdftotext`)에 Preface 등 영어 항목이 없음, HoTT PDF 차례가 이전과 같음.
- **상태** — 이관 후 모든 Pages 프로젝트에서 `status --check` 통과.
