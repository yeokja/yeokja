# R Markdown 파서 설계

bookdown/R Markdown 교재를 번역할 수 있도록 Pandoc Markdown 의미론으로 `.Rmd`를
읽는 `rmarkdown` 파서와, 용어 사전처럼 CSV로 된 원문의 열을 번역하는 `csv` 파서를
추가한다. 첫 사용처는 OpenActTexts/LDAVer2(*Loss Data Analytics*, 2판)다.

## 1. 배경

LDAVer2는 bookdown gitbook으로 빌드하는 Rmd 28개와 child 3개
(`Chapters/DistributionTableSections/{Continuous,Discrete,LimitedExpValue}.Rmd`)로
되어 있다. knitr가 R 청크(`{r}` 449, `{tikz}` 14, `{js}` 1)와 인라인 R(1,275개)을
먼저 실행하고 Pandoc이 나머지를 읽는다. 수식은 `$`/`$$` 1,902개, `\(\)`/`\[\]` 81개,
맨 `\begin{equation|align}` 285개이고, 교차 참조 `\@ref()` 725개, `(\#eq:)` 80개,
`(#tab:)` 59개, 제목 속성 `{#id}` 296개와 `{-}` 139개, 인용 `@key` 약 286개가 있다.
용어 풀이는 `Gloss('term')`이 `References/GlossFct.csv`(용어 489개)에서 찾는다.

기존 파서로 읽으면:

- `markdown`(CommonMark)은 `$$` 안의 줄머리 `~~~~\mathrm{E}[X^k]`를 물결 펜스로
  읽어 `Continuous.Rmd` L938–1411의 353줄을 코드로 삼키고, `$$` 안의 `+ `를 목록으로
  읽으며, 수식 속 `*`·`_`를 강조로 읽고, 제목 속성을 세그먼트에 남긴다.
- `mkdocs`는 수식을 마스킹하지만 수식 탐색이 코드 영역을 먼저 따로 찾는다
  (`code_ranges`). `$$` 안의 `~~~~\mathrm…` 줄을 닫히지 않은 펜스로 보아 그 뒤의
  수식을 더는 찾지 않으므로 같은 줄들이 pulldown-cmark에 그대로 가서 다시 펜스가 된다.
  `Continuous.Rmd`에서 `mkdocs`는 98세그먼트, 이 파서는 134세그먼트를 낸다.
- 두 파서 모두 R 청크 머리의 `fig.cap` 문자열, 인라인 R 인자(`HideExample`의 레이블),
  인라인 R만 있는 문단(544세그먼트), `(ref:label)` 레이블, `\@ref`·`@key`·`(\#eq:)`
  보존을 다루지 못하고, CSV를 읽는 파서가 없다.

## 2. 결정

### 2.1 구조

새 크레이트 `crates/parser-rmarkdown`. `mkdocs`와 같은 그림자 방식이다. 원문과 길이·
줄바꿈이 같은 그림자를 만들어 `parse_with`(pulldown-cmark)에 주고, 블록 범위는 원문에서
자른다. 그 뒤 블록마다 다시 다듬는다(2.4).

`mkdocs`를 고치거나 넓히지 않고 따로 둔다. 수식 규칙(`$20,000`은 수식이 아니다, 빈 줄을
넘지 않는다), 제목 속성 문법(`{-}`, `{.cls}`), knitr 청크·인라인 R·bookdown 참조는
Python-Markdown과 다르고, `mkdocs` 소스의 세그먼트 모양을 바꾸지 않아야 한다. `mkdocs`의
`code_ranges` 결함은 이 설계 범위 밖이다(cp-algorithms 등 기존 소스의 세그먼트가 바뀐다).

### 2.2 영역 스캔 (`scan_regions`)

한 번의 전진 스캔이 서로의 안쪽을 건너뛰며 다음을 찾는다. 수식 안에서는 줄머리 검사를
하지 않으므로 `$$` 안의 `~~~`, `+ `, `#` 줄은 어떤 블록도 열지 못한다.

| 구성 | 규칙 |
|---|---|
| 펜스 코드 블록 | 줄머리(들여쓰기 무관)의 ```` ``` ````·`~~~` 3개 이상, 같은 문자·길이 이상의 줄로 닫힘. 닫히지 않으면 파일 끝까지 |
| bookdown 산문 청크 | ```` ```{theorem|lemma|corollary|proposition|conjecture|definition|example|exercise|hypothesis|proof|remark|solution …} ````. 본문은 Markdown이므로 안쪽도 계속 스캔 |
| HTML 주석 | `<!--`…`-->` |
| 코드 스팬 | 같은 길이의 백틱 런으로 닫힘, 빈 줄을 넘지 않음 |
| `$$…$$` | 빈 줄을 넘지 않음, 내용이 비지 않음 |
| `$…$` | 여는 `$` 뒤가 공백이 아니고, **처음 만나는** `$`가 닫는다. 그 `$` 앞이 공백이거나 뒤가 숫자면 수식이 아니다(`$20,000 and $30,000`). 빈 줄을 넘지 않음 |
| `\(…\)`, `\[…\]` | R Markdown이 켜는 `tex_math_single_backslash`. 빈 줄을 넘지 않음 |
| `\begin{env}…\end{env}` | env 이름이 영숫자·`*`. 빈 줄을 넘어도 된다(raw LaTeX 블록) |
| 이스케이프 | `\` 다음 한 글자를 건너뜀(`\$`는 글자). 줄 끝의 `\`는 다음 줄머리 검사를 막지 않음 |

### 2.3 그림자

- 수식은 `x`로 채운다(줄바꿈 유지). Pandoc에서 `$$`는 인라인 요소이므로 앞뒤 줄과 같은
  문단에 남는다. 이는 Pandoc 의미론과 같다.
- 제목 속성: ATX 제목 줄, 또는 setext 밑줄 바로 앞 줄 끝의 공백 뒤 `{…}`가 `#id`,
  `.cls`, `-`, `key=value`(따옴표 값 포함)로만 되어 있으면 공백으로 지운다. 제목 범위
  밖에 그대로 남는다. `## Sets {a, b}`처럼 속성 문법이 아닌 중괄호는 제목 글이다.
- YAML front matter는 지우고 `title`·`subtitle`(Heading)·`description`(Paragraph)
  값만 Literal로 offer한다. 따옴표 값은 여러 줄이어도 되고 범위는 따옴표 안쪽이다.
  따옴표 없는 값은 더 들여 쓴 다음 줄들까지다. 블록 스칼라(`|`, `>`)는 건너뛴다.
- Pandoc fenced div 줄(`::: {.note}`, `::: name`, `:::`)과 산문 청크의 여는·닫는
  펜스 줄은 지운다. 본문은 일반 Markdown으로 읽힌다.

### 2.4 블록 다듬기 (`refine`)

pulldown-cmark가 자른 번역 대상 블록마다:

1. **접두 레이블.** 문단 맨 앞의 bookdown 텍스트 참조 정의 `(ref:label)`, 제목 맨 앞의
   `(PART)`·`(PART*)`·`(APPENDIX)`는 범위 밖으로 뺀다. 본문만 번역된다.
2. **수식을 가린 문장 분할.** 정규화한 텍스트에서 수식 글자를 `x`로 바꾼 뒤
   `split_sentences`로 나누고, 원문 바이트 범위로 되돌린다. 분할점이 수식 안에 생기지
   않는다(`$x. Y$`).
3. **강조 안 분할 합치기.** 나눈 두 문장이 같은 쪽의 `*`(홀수 길이 런)나 `**`(2 이상
   런)를 열어 둔 채 끝나고 시작하면 합친다(`**Example 7.2.1. Actuarial Exam
   Question.**`). 실측에서 태그화할 수 없는 세그먼트가 464개에서 22개로 줄었다.
4. **산문 없는 문장 제외.** 수식, 인라인 R, 보존 토큰(2.6), HTML 태그, 주석을 빼고
   글자가 하나도 없는 문장은 범위에서 뺀다. 그런 문장 앞뒤로 블록을 나눈다. 빠진
   바이트는 그대로 남는다. 인라인 R만 있는 문단(`` `r HideExample(...)` ``,
   `` `r SolnEnd()` ``), 수식만 있는 문단, `<a id=tab:1.5></a>`만 있는 문단이 여기에
   든다. 블록 하나에서 아무 문장도 남지 않으면 번역하지 않는 블록이 된다.
5. **가장자리 원자 다듬기.** 블록(나눈 뒤의 연속 문장)이 인라인 R 코드나 디스플레이
   수식(`$$`, `\[`, `\begin`)으로 시작하거나 끝나면 그 부분을 범위 밖으로 뺀다
   (`` `r SolnBegin()` Since… `` → `Since…`, `$$…$$\n**`R` Commands**` →
   `**`R` Commands**`). 문장 가운데의 디스플레이 수식은 문장에 남는다. 한국어 어순은
   수식 앞뒤 구절을 바꿔야 하므로 수식에서 문장을 끊지 않는다.
6. **인라인 코드 캡처.** 4에서 뺀 문장 안의 인라인 코드 스팬에 설정한 정규식(2.7)을
   맞춰, 일치한 캡처 그룹마다 Literal Paragraph 블록을 만든다. 번역 대상 문장 안의
   코드 스팬은 캡처하지 않는다(범위가 겹칠 수 없다). 그 코드는 문장과 함께 모델에게
   간다.
7. 블록·세그먼트 번호를 다시 매긴다(구역 안 블록 위치, 블록 안 문장 위치).

### 2.5 Literal 문자열과 재구성

- knitr 청크 머리(```` ```{r label, …} ````)의 `fig.cap`·`fig.alt`, 산문 청크의
  `name` 문자열 리터럴 값을 Literal로 offer한다. 값이 `(ref:label)` 하나뿐이면 레이블이므로
  offer하지 않는다. `fig.cap = paste(…)`처럼 문자열 리터럴이 아닌 값은 건너뛴다.
- 재구성은 `splice_reconstruct`와 같되 Literal 블록의 번역을 감싼 따옴표에 맞춰 쓴다.
  범위 앞뒤 바이트가 같은 따옴표면 R 문자열과 YAML 큰따옴표 문자열은 역슬래시가 앞서지
  않은 그 따옴표 앞에 `\`를 넣고, YAML 작은따옴표 문자열은 따옴표를 겹친다. 따옴표 없는
  YAML 값은 `: `, ` #`, 끝의 `:`, YAML 지시 문자로 시작하면 큰따옴표로 감싼다.
- 인라인 코드 캡처도 같은 규칙을 따른다(`'Show Example Solution'` 안의 `'`).

### 2.6 보존 토큰과 서식 평가

세그먼트 안에 그대로 두고 번역도 정확히 같은 다중집합으로 가져야 하는 토큰
(`preserved_token_spans`, 수식·코드·주석 밖):

- `\@ref(label)`, `(\#eq:label)`·`(#tab:label)`·`(#fig:label)` 등 `(\?#종류:레이블)`,
  `(ref:label)`, 각주 참조 `[^n]`, 빈 앵커 `<a id=…></a>`·`<a name=…></a>`,
  bracketed span 속성 `]{#tab:7.1}`
- 인용: 대괄호 묶음 `[@a; @b]`(인용만 든 것), 맨 인용 `@key`. 맨 인용은 앞 글자가
  영숫자·`\`·`@`가 아니고, 키는 영숫자·`_`로 시작해 영숫자·`_`와 그 사이의
  `:.#$%&-+?<>~/`로 이어진다(Pandoc 규칙, 전자 우편 주소는 제외된다). 중괄호 키
  `@{key}`는 `@key`와 같은 토큰으로 센다.

Pandoc은 유니코드 글자도 키로 읽으므로 `@frees2009는`은 키 `frees2009는`이다. 그래서

- `Markup::RMarkdown`의 기계적 수선(`repair_reply`)이 원문의 맨 인용 뒤에 한글 같은
  비 ASCII 글자가 붙은 것을 `@{frees2009}는`으로 고친다. ASCII 글자가 붙은 것은 다른
  키이므로 고치지 않는다.
- 서식 평가기(`rmarkdown_issues`)는 수식을 원문과 바이트 단위로 대조하고(빠짐·추가),
  토큰 다중집합이 다르면 빠진 것과 더한 것을 알려 재시도하게 한다. 그다음 수식을 가린
  텍스트를 Markdown으로 검사하되, Pandoc은 `*`를 뒤 글자와 상관없이 닫으므로 MkDocs처럼
  CommonMark 측면 규칙은 적용하지 않는다. 괄호 검사도 수식을 가린 텍스트로 한다.
- 프롬프트 `closing_rule`에 수식·인라인 R·토큰 보존 규칙과 `@{key}` 표기를 적는다.
- 용어집 평가와 스냅샷 채택은 수식을 가린 원문으로 용어를 찾는다(`\text{loss}`는 수식).
- 정렬 검사는 `$`를 수식 구분자로, 링크 검사는 Markdown 링크로 읽는다.

### 2.7 인라인 코드 캡처 설정

- `[[sources]]`의 `inline_code_captures: Vec<String>`, 기본값 없음. 정규식(Rust
  `regex`)은 코드 스팬의 백틱 안쪽 내용(`r HideExample('3.3.1', 'Show Example
  Solution')`)에 맞춘다. 번호 0이 아닌 모든 캡처 그룹 가운데 일치한 것이 각각 Literal
  세그먼트가 된다. 나머지 바이트는 보존한다. 번역하지 않을 그룹은 `(?:…)`로 쓴다.
  LDAVer2의 `Hide*(id, label)` 108종 레이블을 잡는 예는 5절에 있다.
- 소스 설정에 두는 까닭: 함수 이름과 인자 위치는 책마다 다르고, 같은 프로젝트의 다른
  소스(CSV)와 무관하다. 파서는 설정을 읽지 않는다는 원칙(`select.rs`)과 달리 이 규칙은
  블록 모양 자체를 바꾸므로 사후 선택 규칙으로 표현할 수 없다. 레지스트리
  (`select_parser`)가 소스 설정에서 파서를 만들 때 넘긴다.
- 설정 검사: `rmarkdown`이 아닌 소스에 두면 오류, 컴파일되지 않는 정규식은 오류.

### 2.8 CSV 파서

새 크레이트 `crates/parser-csv`, 이름 `csv`, 확장자 `.csv` 자동 감지, `Markup::Plain`.

- RFC 4180: `,` 구분, 레코드 끝 `\n` 또는 `\r\n`, 큰따옴표 필드 안의 `""`는 따옴표
  하나, 따옴표 필드는 줄바꿈을 담을 수 있다. 따옴표 없는 필드 안의 `"`는 글자다.
- 첫 레코드가 열 이름이다. 필드마다 `BlockRole::TableCell { table: 0, column,
  label_row, header }` 블록이므로 기존 `[[tables]]` 규칙으로 번역할 열을 고른다
  (`translate = ["Definition"]`). 열 이름 행은 번역하지 않는다(코드가 이름으로 읽는다).
  글자가 없는 필드(숫자, 빈 필드)는 블록을 만들지 않는다.
- 세그먼트 번호의 블록 자리(<1000)를 지키려고 구역마다 블록을 900개 이하로 나누고,
  구역마다 열 이름 블록을 범위 없이 되풀이해 표 규칙이 각 구역에서 열 이름을 찾게 한다.
  재구성에는 영향이 없다.
- 범위는 필드 전체(따옴표 포함)다. 번역은 원문이 따옴표 필드였거나 번역에 `,`·`"`·
  줄바꿈이 있으면 큰따옴표로 감싸고 `"`를 겹쳐 쓴다. 그 밖의 바이트는 그대로다.
- `Term` 열을 번역하면 `Gloss('term')` 조회 키가 바뀐다. 키를 바꾸려면 프로젝트가
  `Gloss` 재정의(overlay)를 함께 둬야 한다. 파서는 열 선택만 제공한다.
- Markup::Plain: 서식 평가 없음, `closing_rule` 없음.

### 2.9 인라인 태그

`rmarkdown`을 `INLINE_TAG_PARSERS`에 넣는다(기본 꺼짐, 소스마다 `inline_tags = true`).
코덱 방언 `Dialect::RMarkdown`:

- 가림: 수식(이 파서의 `math_spans`)은 `<mN/>`, 보존 토큰과 raw TeX 명령
  (`\newline`, `\index`)은 `<xN/>`. raw TeX을 가리지 않으면 직렬화기가 `\`를 이스케이프해
  글자로 만든다.
- 이스케이프: 텍스트의 `$`(수식), `^`(위첨자), 앞이 영숫자가 아니고 뒤가 키 글자인
  `@`(인용)를 이스케이프한다. `~`는 CommonMark와 같다(아래첨자·취소선).
- 맨 인용 토큰 바로 뒤에 글자가 오면 `@{key}`로 쓴다.
- `\[…\]`는 수식이므로 대괄호 이스케이프 여부를 정할 때 수식을 가린 원문을 본다.
- 판독·검증은 pulldown-cmark(CommonMark)로 다시 읽는다. CommonMark가 닫는 강조는
  Pandoc도 닫으므로 검증은 보수적이다(Pandoc이 더 받아 주는 꼴을 HTML 강조로 바꿀 수는
  있다).

실측(LDAVer2 29파일 + CSV 전체 필드, 태그화→무번역 응답→직렬화, `inline_roundtrip`
테스트): 세그먼트 9,773개 가운데 태그화
불가 22개(원문이 `*n-*th`처럼 강조를 닫지 못함 등, 기존 경로로 간다), 나머지 9,751개
모두 원문과 같은 HTML. 바이트가 바뀐 71개는 기존 코덱과 같은 엔티티·이스케이프 표기
차이(`&uuml;`→`ü`)다.

### 2.10 등록과 Markup

- `Markup::RMarkdown`, `Markup::Plain` 추가. 수식 보존 검사가 `RMarkdown`에 적용된다.
- 레지스트리: 이름 `rmarkdown`(소스의 `inline_code_captures`로 생성), `csv`. 확장자
  `.Rmd`/`.rmd`, `.csv` 자동 감지.
- `PARSER_NAMES`(`config.rs`)에 모든 파서 이름을 두고, 모르는 이름이면 설정 오류로
  알린다(지금까지는 조용히 `markdown`으로 읽었다).

## 3. 한계

- 블록 구조는 CommonMark다. Pandoc은 빈 줄 없이 목록·제목·펜스가 문단을 끊지 못하지만
  pulldown-cmark는 끊는다. 바이트는 보존되고 번역 단위만 나뉜다.
- 디스플레이 수식이 든 문단의 세그먼트는 줄바꿈을 공백으로 합친 수식을 담는다(정규화).
  재구성은 번역 안의 디스플레이 수식(`$$`, `\[`, `\begin{…}`)을 공백 차이를 무시해 찾아
  원문의 줄(앞뒤 줄바꿈과 들여쓰기 포함)로 되돌리므로 Pandoc의 줄 머리 `\begin{equation}`과
  bookdown의 `$$`·`(\#eq:)` 줄 규칙이 지켜진다. 번역이 수식을 바꿔 찾지 못하면 한 줄로 남는다
  (평가기가 수식 변경을 오류로 잡는다). 수식 안 `%` 주석은 LDAVer2 수식에는 없다.
- `[text](#tab:x)`의 목적지는 보존 토큰이 아니라 링크로 다루고(인라인 태그 코덱),
  `[text]{#tab:x}` 같은 Pandoc 대괄호 span은 참조 링크로 보지 않는다(참조 링크 수선). 둘 다
  같은 이름의 참조 정의가 있어도 `[표][Table]…`로 바뀌지 않는다.
- 원시 HTML 블록(`<div align="center">`) 안에서 열린 knitr 청크는 그림자에서 통째로 비워
  코드로 남긴다. HTML 블록 안의 산문은 블록 안에서만 번역 단위가 되므로, 청크 뒤 `</div>`
  앞에 둔 산문은 제공되지 않는다. LDAVer2에는 없다.
- 문장 가운데의 인라인 R 전용 문장 조각은 문장과 함께 간다. 문장 단위 분리는 4·5단계만큼만
  한다.
- 청크 안 YAML식 옵션(`#| fig.cap: "…"`), 문자열 리터럴이 아닌 캡션, `kable(caption=…)`,
  `TableGen1(…)` 인자는 번역하지 않는다(필요하면 인라인 코드 캡처처럼 청크 본문 캡처를
  따로 설계한다).
- 캡처 번역에 백틱이 들어가면 인라인 코드가 깨진다. 평가기는 이를 보지 않는다.
- Pandoc의 `^[인라인 각주]`, 위첨자·아래첨자는 세그먼트 텍스트에 그대로 남는다.

## 4. 검증

테스트(`cargo test --workspace` 통과, `cargo clippy --workspace --all-targets` 경고 0):

- `crates/parser-rmarkdown/tests/fixtures/`의 LDAVer2 발췌(`Continuous.Rmd` 전체,
  `Frequency2024.Rmd` 세 구간, `Simulation2024.Rmd`의 `<div>` 청크·표 8.3 구간,
  `Frequency2024.Rmd`·`DataAnalytics.Rmd`의 번호 수식, CC BY 4.0)로 무번역 왕복 바이트
  동일, 전부 번역해도 코드 블록·디스플레이 수식(줄 포함) 보존.
- `<div>` 안 청크가 코드로 남고 뒤의 제목이 제공됨, 디스플레이 수식의 줄 복원(공백이 다른
  번역, 목록 안 들여쓰기), `[Table 8.3](#tab:8.3)`·`[Table 8.3]{#tab:8.3}`이 참조 정의 옆에서도
  참조 링크로 바뀌지 않음(파서·`reflinks`·인라인 태그 코덱 각각).
- `$$` 안 `~~~~`·`+ `·`#` 줄 보호(Continuous.Rmd의 마지막 제목과 `**`R` Commands**`가
  offer되고 수식은 세그먼트에 없음), Pandoc 수식 규칙, 제목 속성 5종, `fig.cap`/`fig.alt`
  Literal과 따옴표 이스케이프, `(ref:)` 정의, 인라인 R 전용 문단·가장자리 제외, 수식·
  참조 전용 문단 제외, 수식·강조 안 분할 방지, 캡처 규칙, front matter, 산문 청크·div,
  `(PART)`, 보존 토큰, `@{key}` 수선.
- CSV: RFC 4180 따옴표·줄바꿈, `GlossFct.csv`(507레코드×4열) 무번역 왕복 바이트 동일,
  열 이름 비번역, 표 규칙으로 Definition 열 선택, 따옴표가 필요한 번역의 재구성.
- 평가기: 토큰 다중집합(바뀐 레이블, 빠진 `@`, 되풀이, `\#`→`#`, 붙은 조사), Pandoc
  수식, Plain.
- 코덱: 수식·참조·raw TeX 원자, `$`·`^`·`@` 이스케이프, `@{key}`.
- 설정: 모르는 파서 이름, 캡처의 파서 제한·정규식 오류. 레지스트리: 확장자·이름·캡처.

실측(`yeokja status`/`coverage`, 원문 커밋 67f5aab0):

| 구성 | 파일 | 세그먼트 |
|---|---|---|
| 조사 설정 그대로(`mkdocs`, exclude 9개) | 34 | 9,168 |
| 같은 설정, `rmarkdown` | 34 | 8,647 |
| 빌드 대상(Rmd 28 − Packages·Glossary + child 3), `mkdocs` | 29 | 9,069 |
| 빌드 대상, `rmarkdown` + 캡처 | 29 | 8,523 |
| `GlossFct.csv` Definition 열 | 1 | 515 |

빌드 대상 `rmarkdown` 8,523개 가운데 `Hide*` 레이블 캡처 305개, `\@ref`를 담은 것
634개, `$$`를 담은 것 513개. 29파일 모두 무번역 왕복이 바이트 동일하다. `mkdocs`보다
적은 것은 인라인 R 전용 문단(544)·수식/참조 전용 문단 제외와 강조 안 분할 합치기 때문이고,
`Continuous.Rmd`는 98 → 134로 늘었다. coverage는 18,265줄 중 50%(코드 청크 포함)다.

## 5. 설정 예시

```toml
[[sources]]
path = "upstream"
pattern = "**/*.Rmd"
exclude = ["docs/**", "GettingStarted/**", "IntermediateCalcs/**", "Chapters/SimulationSections/**",
  "Chapters/DataResources.Rmd", "Chapters/ExpRatingBonusMalus.Rmd",
  "Chapters/DistributionTableSections/ContinuousOLD.Rmd", "**/*Latex.Rmd",
  "References/PdfOutput.Rmd", "References/Glossary.Rmd", "References/Packages.Rmd"]
parser = "rmarkdown"
output = "ko/{path}"
inline_tags = true
inline_code_captures = ["^r Hide\\w+\\(\\s*['\"][^'\"]*['\"]\\s*,\\s*['\"](?P<label>[^'\"]+)['\"]"]

[[sources]]
path = "upstream/References"
pattern = "GlossFct.csv"
parser = "csv"
output = "ko/References/{path}"

[[tables]]
files = "upstream/References/GlossFct.csv"
headers = ["Term", "Definition"]
translate = ["Definition"]
```
