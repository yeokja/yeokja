# Hugo 파서와 링크 안 문장 분리 설계

Hugo 블로그(첫 사용처: microsoft/forge-lab의 `blog/content/posts/*/index.md`)를 번역하는 데
모자란 코어 세 가지를 채운다.

- R1. TOML 프런트매터(`+++ … +++`)를 본문으로 읽지 않고, 고른 키의 문자열 값만 번역한다.
- R2. 단축 코드만 있는 줄(`{{< note >}}`, `{{< /note >}}`, `{{% x %}}`)은 세그먼트가 아니다.
- R3. 문장 분리기가 인라인 링크·이미지 안에서 문장을 자르지 않는다.

## 1. 원인

- **R1.** Markdown 계열 파서는 pulldown-cmark의 `ENABLE_YAML_STYLE_METADATA_BLOCKS`만 켜고,
  `mkdocs`의 `front_matter_end`도 `---`만 본다. `+++`로 연 블록은 첫 줄이 글자인 문단이
  되어 `title = "…" date = 2026-11-11 …` 전체가 한 세그먼트로 나온다. 번역되면 TOML이
  깨진다. 프런트매터 값을 내놓는 옵션 `front_matter_keys`는 `rmarkdown`(YAML) 전용이다.
- **R2.** `{{< note >}}`는 CommonMark에서 글자 문단이다. 혼자 있는 줄은 `{{< note >}}`
  세그먼트가 되고, 빈 줄 없이 본문이 이어지면 본문 첫 문장에 붙어 번역 대상이 된다.
- **R3.** `split_sentences`는 백틱 코드 안의 구두점만 경계에서 뺀다. 그림 설명이 여러
  문장인 `![Diagram: one column of memory. The main thread's … registered.](x.svg)`는
  `![Diagram: … memory.`와 `The main thread's … registered.](x.svg)`로 갈라져 각 세그먼트의
  Markdown이 짝을 잃는다. 링크 글자 `[First step. Then more](url)`도 같다.

## 2. 결정

### 2.1 새 파서 이름 `hugo`

`parser = "hugo"`는 Hugo(goldmark, CommonMark) 사이트의 콘텐츠 파일을 읽는다. 본문은
`markdown` 파서와 똑같이 읽고(`parse_with`, 제목 `{#id}` 보존, 참조 링크 해석), 그림자에서
두 가지를 지운다.

1. 파일 첫 줄이 `+++`이면 다음 `+++` 줄까지가 TOML 프런트매터, `---`이면 다음 `---`·`...`
   줄까지가 YAML 프런트매터다. 닫는 줄까지 통째로 공백으로 지운다. 닫는 줄이 없으면
   프런트매터가 아니다.
2. 단축 코드만 있는 줄을 공백으로 지운다(2.4).

지운 바이트는 스팬 밖이므로 재구성에서 원문 그대로 남는다. Markup은 `Markdown`이고
인라인 태그 방언은 CommonMark다. `inline_tags`를 켤 수 있는 파서(`INLINE_TAG_PARSERS`)에
넣는다 — goldmark는 기본 설정에서 `$` 수식을 읽지 않는다.

**`markdown`에 옵션을 다는 대신 새 이름인 이유.**

- 단축 코드 줄 지우기를 `markdown`에 늘 켜면 Hugo가 아닌 원문의 `{{< … >}}`·`{{% … %}}`
  줄(템플릿 예시, Jinja 문서) 세그먼트가 바뀐다. 옵션으로 두면 Hugo 사이트는 옵션 두
  개(프런트매터, 단축 코드)를 늘 함께 켜야 한다. 사이트 생성기가 정하는 문법 묶음이므로
  `mkdocs`처럼 이름 하나로 고르는 편이 설정이 짧고 틀릴 곳이 없다.
- TOML 프런트매터를 Markdown 계열 공통(pulldown-cmark의
  `ENABLE_PLUSES_DELIMITED_METADATA_BLOCKS`)으로 켜지 않는다. MyST에서 `+++` 줄은 블록
  나눔 표시라서, `+++`로 시작하는 MyST 노트북은 다음 `+++`까지를 잃는다.
- 그래서 기존 프로젝트의 세그먼트는 R3(2.5) 말고는 바뀌지 않는다.

### 2.2 `front_matter_keys`를 `hugo`에서도 받는다

`front_matter_keys`(점으로 이은 키 경로 목록)는 `rmarkdown`과 `hugo`가 읽는다. 다른
파서의 소스가 설정하면 설정 오류다. 경로 검사(빈 조각 거부)는 그대로다.

- `rmarkdown`은 지금처럼 기본 키(`title`·`subtitle`·`description`)에 더해 경로 값을 낸다.
- `hugo`는 기본 키가 없고 **설정한 경로만** 낸다. Hugo 프런트매터에는 `slug`, `author`,
  `artwork`, `aliases`처럼 글자처럼 보이지만 번역하면 URL이나 템플릿 조회가 깨지는 값이
  섞여 있어, 무엇을 번역하는지 설정에 드러나야 한다.
- 값은 `BlockType::Paragraph`, `literal: true`인 Extra로 내고, 문장 단위로 나뉜다.

YAML 프런트매터는 `rmarkdown`의 규칙(들여쓰기 구조로 경로를 찾고, 따옴표·평문·앵커를
읽고, 블록 스칼라와 별칭은 내지 않음; 2026-10-30 R Markdown 프런트매터 키 설계)을 그대로
쓴다. 이를 위해 YAML 도우미(`front_matter`, `yaml_key`, `yaml_scalar`, `yaml_path_line`,
`yaml_values`, 따옴표 쓰기)를 `parser-markdown-dialect`의 `front_matter` 모듈로 옮겨 두
파서가 함께 쓴다. `rmarkdown`의 동작은 바뀌지 않는다.

### 2.3 TOML 값 찾기와 쓰기

TOML을 파서로 읽지 않고 줄 단위로 찾는다. 값을 바꿔 쓸 바이트 범위가 필요하기 때문이다.

**찾기.** 프런트매터 안의 줄을 차례로 본다.

- 빈 줄과 `#` 주석 줄은 건너뛴다.
- `[a.b]` 표 머리 줄은 현재 표를 `a.b`로 바꾼다. `[[a]]` 배열 표 머리 뒤의 키는 경로로
  가리킬 수 없으므로 다음 표 머리까지 찾지 않는다.
- `key = value` 줄의 키는 점으로 이은 키다. 각 조각은 bare 키(`A-Za-z0-9_-`)이거나
  따옴표 키(`"…"`, `'…'`)이고, 점 앞뒤 공백은 무시한다. 현재 표 경로 뒤에 키 조각을 이은
  것이 설정한 경로와 같으면 그 값을 본다.
- 값이 한 줄 안에서 닫히는 basic 문자열(`"…"`, 역슬래시 이스케이프 고려)이나 literal
  문자열(`'…'`)이면 따옴표 안쪽을 낸다. 공백뿐이면 내지 않는다. 여러 줄 문자열
  (`"""`, `'''`), 배열, 인라인 표, 숫자, 날짜, 불리언은 내지 않는다.

세그먼트에는 원문 바이트가 그대로 간다(basic 문자열의 `\"`도 그대로). YAML 값과 같은
규칙이며, 번역이 이스케이프를 유지하면 그대로 쓸 수 있다.

**쓰기.** 번역은 언제나 유효한 TOML 문자열로 돌아간다.

- basic 문자열: 번역을 한 글자씩 읽으며 TOML 1.0의 유효한 이스케이프(`\b \t \n \f \r
  \" \\ \uXXXX \UXXXXXXXX`)는 그대로 두고, 그 밖의 `\`는 `\\`로, 이스케이프되지 않은
  `"`는 `\"`로, 제어 문자는 `\n`·`\t`·`\r`·`\uXXXX`로 쓴다. Hugo의 TOML 읽기(go-toml v2)는
  1.0이므로 1.1의 `\e`는 유효한 이스케이프로 치지 않는다.
- literal 문자열: 번역에 `'`이나 제어 문자가 없으면 그대로 쓴다. 있으면 literal 문자열로
  쓸 수 없으므로 따옴표까지 바꿔 basic 문자열로 쓴다(모든 `\`와 `"`를 이스케이프). 이
  경우만 스플라이스가 따옴표 두 바이트를 넓게 덮는다.

### 2.4 단축 코드 줄

펜스 코드 블록(``` ``` ```·`~~~`, 세 개 이상, 들여쓰기 세 칸 이하, 같은 글자로 같은 길이
이상이 닫음) 밖에서 다음 줄을 공백으로 지운다.

- 앞뒤 공백을 뺀 줄이 단축 코드 토큰(`{{<` … `>}}`, `{{%` … `%}}`) 하나 이상과 그 사이의
  공백으로만 이루어진 줄. 여는 꼴과 닫는 꼴(`{{< /note >}}`), 주석 꼴(`{{</* x */>}}`)
  모두 해당한다.
- 줄이 `{{<`·`{{%`로 시작하고 그 줄에서 닫히지 않는 여러 줄 단축 코드는, 닫는 `>}}`·`%}}`
  뒤에 공백(또는 다른 단축 코드)만 남는 줄까지 지운다. 닫는 줄을 찾지 못하면 지우지
  않는다.

지운 줄은 빈 줄이 되므로 `{{< note >}}` 바로 아래에 붙은 본문도 따로 문단으로 읽힌다.
짝을 이룬 단축 코드 사이의 본문은 Markdown으로 그대로 번역된다. 문장 안의 단축 코드
(`see {{< ref "x" >}} for`)는 지우지 않고 세그먼트의 글자로 함께 간다.

### 2.5 링크·이미지 안에서 자르지 않기 (R3)

`parser-utils`에 `split_markdown_sentences`와 `make_markdown_segments`를 더한다.
`split_sentences`와 같되 다음 범위의 구두점을 경계로 보지 않는다. 범위는 코드 스팬 밖에서
`[`(또는 `![`)로 시작한다.

- 인라인 링크·이미지: `[` 글자 `]` 바로 뒤 `(` … 짝 맞는 `)`.
- 전체·축약 참조 링크: `[` 글자 `]` 바로 뒤 `[` 레이블 `]`.

글자 부분의 `]`는 대괄호 짝을 맞춰 찾고, 역슬래시 이스케이프와 백틱 코드 스팬 안의
대괄호는 세지 않는다. 목적지의 `)`는 괄호 짝을 맞춰 찾고 `<…>` 목적지와 따옴표 제목 안의
괄호는 세지 않는다. 닫히지 않으면 링크가 아니고 지금처럼 자른다. `]` 뒤에 공백이 있으면
CommonMark 링크가 아니므로 가리지 않는다. 단축 참조(`[label]`)와 각주(`[^1]`)는 대괄호
글자와 구별할 수 없어 가리지 않는다.

`parser-markdown-dialect`가 블록과 Extra를 문장으로 나눌 때 `make_markdown_segments`를
쓴다. `rmarkdown`이 수식을 가린 뒤 다시 나누는 `pieces`도 `split_markdown_sentences`를
쓴다. 그래서 `markdown`, `hugo`, `mkdocs`, `mdx`, `myst`, `markdeep`, `bikeshed`,
`rmarkdown`에 적용된다. reST·AsciiDoc·LaTeX 등 다른 문법의 `make_segments`는 바꾸지
않는다(`[x](y)`가 링크가 아니다).

**기존 프로젝트에 미치는 영향.** 링크 글자나 그림 설명 안에 문장 경계가 있던 세그먼트만
바뀐다: 둘 이상의 조각이 하나로 합쳐진다. 그 조각들은 Markdown 짝이 깨진 세그먼트였으므로
바뀌는 것이 고침이다. 합쳐진 세그먼트는 새 원문이라 `reconcile` 뒤 다시 번역해야 한다.
프로젝트별 수는 구현 계획의 검증 단계에서 `reconcile --dry-run`으로 센다.

## 3. 설정 예

```toml
[[sources]]
path = "upstream/blog/content"
pattern = "posts/**/*.md"
parser = "hugo"
output = "ko/{path}"
front_matter_keys = ["title", "description"]
```

`front_matter_keys`를 비우면 프런트매터는 통째로 원문 그대로 남는다.

## 4. 범위 밖

- JSON 프런트매터(`{ … }`). forge-lab은 쓰지 않는다.
- 단축 코드 인자의 글자(`{{< figure caption="…" >}}`). 필요해지면 `inline_code_captures`
  같은 정규식 옵션으로 다룬다.
- 단축 참조 링크 `[label]` 안의 문장 경계.
