# MkDocs 정의 목록과 수식 옵션 설계

`mkdocs` 파서가 사이트의 `markdown_extensions` 구성에 맞춰 읽도록 소스 옵션 두 개를
더하고, 수식이 없는 `mkdocs` 소스에 인라인 태그 전송을 연다. 첫 사용처는 MkDocs
Material 사이트 *Linux Kernel Internals*(`markdown_extensions`: `tables`, `admonition`,
`def_list`, `pymdownx.superfences`, `attr_list`; `pymdownx.arithmatex` 없음)다.

## 1. 배경

`mkdocs` 파서는 pulldown-cmark(CommonMark)에 원문과 길이가 같은 그림자를 주어
Python-Markdown 고유 구문을 같은 구조의 CommonMark로 읽힌다. 지금 그림자는 admonition·
탭 머리, 프런트매터, Jinja 줄, 수식을 다루며, 수식 인식은 언제나 켜져 있다.

### 1.1 정의 목록

Python-Markdown `def_list`는 용어 줄 아래의 `:   ` 줄을 정의로 읽는다.

```markdown
Landed
:   Linux, January 2026 (CVE-2026-23089)

Driver
:   `snd-usb-audio` (`sound/usb/mixer.c`)
```

CommonMark에는 이 구문이 없으므로 두 줄은 한 문단이고, 세그먼트는
`Landed :   Linux, January 2026 (CVE-2026-23089)` 하나가 된다. 번역은 한 줄로 되돌아가
`<dl>`이 일반 문단이 되고, 모델이 `:   `를 지우거나 옮길 수 있다. 이 원문에서는 54개
파일의 290줄이 이 구문이다.

### 1.2 `$`

`pymdownx.arithmatex`가 없는 사이트에서 `$`는 글자다. 파서는 `$..$`를 인라인 수식으로
읽으므로, 이웃한 두 목록 항목이 각각 `…into 10000$](…)`로 끝나면 첫 `$`부터 둘째 `$`
까지가 수식이 되어 둘째 항목의 `- `가 가려지고 두 항목이 한 세그먼트로 합쳐진다.
`Markup::MkDocs`의 Format 평가는 원문의 수식을 번역에 바이트 그대로 요구하므로,
`$5/line—$10,000` 같은 산문에서 오탐도 낸다.

### 1.3 인라인 태그

이 원문은 코드 스팬과 링크가 매우 많다. 인라인 태그 전송은 강조·링크의 문법을
직렬화기가 쓰게 하지만 `mkdocs` 소스에는 켤 수 없다. 태그 코덱이 arithmatex 수식을
모르기 때문이다. 수식이 없는 소스에는 그 이유가 없다.

## 2. 결정

### 2.1 소스 옵션은 `markdown_extensions`를 따른다

`SourceConfig`에 두 옵션을 더한다. 둘 다 `mkdocs` 전용이고, 사이트의 `mkdocs.yml`이
해당 확장을 켰는지를 그대로 옮긴다.

| 옵션 | 기본값 | 뜻 |
|---|---|---|
| `math` | `true` | `pymdownx.arithmatex`를 쓴다. `false`면 수식을 찾지 않는다 |
| `definition_lists` | `false` | `def_list`를 쓴다. `true`면 정의 목록을 읽는다 |

- 기본값은 지금 동작이다. 옵션을 쓰지 않는 소스의 세그먼트는 바뀌지 않는다.
- 정의 목록을 언제나 읽지 않고 옵션으로 둔다. 확장을 켜지 않은 사이트에서 문단 안의
  `: ` 줄은 글자이고, 들여쓴 코드 블록의 `: v(a) {}`(C++ 생성자 초기화 목록) 같은 줄을
  정의로 읽으면 코드가 세그먼트가 된다.
- 필드는 `Option<bool>`이다. 다른 파서의 소스가 값을 적으면(참·거짓 모두)
  `ProjectConfig::from_toml`이 거부한다. `inline_code_captures`·`front_matter_keys`와
  같은 규칙이다.
- `SourceConfig`에 `#[serde(deny_unknown_fields)]`를 붙인다. `definition_list = true`
  같은 오타가 조용히 버려져 기본값으로 읽히는 일을 설정 오류로 바꾼다.
- 파서 선택(`crates/parsers`)은 `mkdocs` 소스에 `MkdocsParser::configured(math,
  definition_lists)`를 준다. `MkdocsParser::default()`는 수식 켬, 정의 목록 끔이다.

### 2.2 정의 목록 읽기

Python-Markdown의 규칙(`(^|\n)[ ]{0,3}:[ ]{1,3}(.*?)(\n|$)`, 블록 처리 순서)을 줄
단위로 옮긴다. 그림자 처리는 수식을 채운 뒤에 한다.

**정의 줄.** 들여쓰기 뒤 `:`와 공백 하나 이상으로 시작하는 줄이다. admonition 본문처럼
컨테이너 안에서는 본문이 먼저 내어쓰이므로 들여쓰기 깊이는 제한하지 않는다. 펜스 안의
줄, 수식에 걸친 줄은 아니다. `:::`(mkdocstrings)은 콜론 뒤가 공백이 아니므로 해당하지
않는다.

**정의로 인정하는 조건.** 다음 중 하나다.

- 바로 위에 용어 줄이 있다.
- 빈 줄 뒤이거나(`Term`, 빈 줄, `:   def` — 느슨한 정의), 다른 정의 또는 그 본문 뒤다
  (한 용어의 여러 정의).

바로 위 줄이 다른 블록을 여는 줄(목록 항목, 인용, 표 행, HTML, 제목, 블록 머리)이면
정의가 아니다. Python-Markdown은 그 줄들을 자기 처리기에 먼저 준다.

**용어 줄.** 정의 줄 바로 위에 빈 줄 없이 이어진 일반 텍스트 줄들이며, 들여쓰기가 정의
줄과 3칸 이내로 같아야 한다(같은 블록 수준). 다른 블록을 여는 줄, 수식에 걸친 줄에서
멈춘다. 줄마다 용어 하나다(`<dt>` 하나).

**그림자.**

- 정의 줄의 `:`를 `-`로 바꾼다. 정의는 목록 항목이 되어 본문이 제 블록이 되고, 4칸
  들여쓴 이어지는 줄·문단·펜스는 항목의 내용이 된다(Python-Markdown이 내어써서 읽는
  구조와 같다). 콜론 뒤 공백이 4칸을 넘으면 본문이 들여쓴 코드가 되므로 그 사이를
  두 칸 간격의 중첩 목록 표시로 메운다.
- 용어 줄은 그림자에서 비우고, 줄의 앞뒤 공백을 뗀 범위를 `BlockType::Paragraph`
  `Extra`로 낸다. 비우지 않으면 정의 뒤에 내어쓰기 없이 오는 다음 용어 줄이 앞 항목의
  게으른 이어쓰기가 되어 정의 본문에 붙고, 여러 줄 용어는 한 문단으로 합쳐진다.

**재구성.** `:   ` 표시와 줄 구조는 어느 범위에도 들지 않으므로 원문 그대로 남는다.
용어 줄은 줄마다 제자리에서, 정의 본문은 표시 뒤에서 바뀐다. 여러 줄에 걸친 정의
본문은 다른 문단과 같이 한 줄로 이어 쓴다.

**다루지 않는 것.** 빈 줄로 떨어진 여러 줄 용어 문단(`A`, `B`, 빈 줄, `:   def`)은
한 문단으로 읽는다. Python-Markdown은 줄마다 용어로 보지만 번역이 한 줄로 이어 쓰면
용어가 하나가 된다. 원문에 이 꼴은 없다.

### 2.3 수식 끄기

`math = false`면 수식 범위를 찾지 않는다. 그림자에 채울 것도, 수식뿐인 블록을 빼는
일도 없으며 `$`·`\(`·`\[`는 CommonMark가 읽는 대로다.

평가와 프롬프트가 같은 사실을 알도록 파서는 `Markup::MkDocsNoMath`를 알린다(수식을
켠 소스는 지금처럼 `Markup::MkDocs`).

| 자리 | `MkDocs` | `MkDocsNoMath` |
|---|---|---|
| Format: 수식 바이트 보존, 수식 추가 금지 | 검사 | 검사하지 않음 |
| Format: 강조·코드·괄호 검사 전 수식 가리기 | 가림 | 가리지 않음 |
| Format: Jinja 구분자 추가 금지, Python-Markdown식 느슨한 강조 | 같음 | 같음 |
| 정렬 검사의 `$` 수식 토큰 | 읽음 | 읽지 않음 |
| 용어집: 외부 링크뿐·그림뿐인 세그먼트 제외 | 같음(+`$$…$$`뿐) | 같음 |
| 프롬프트 닫는 규칙 | 수식을 바이트 그대로 복사 | `$`는 달러 기호이며 `$...$`로 감싸지 않는다 |

새 변형을 두는 이유: `Markup::Markdown`으로 알리면 Python-Markdown의 느슨한 강조
규칙, Jinja 검사, MkDocs 프롬프트 규칙을 잃는다.

### 2.4 인라인 태그

`mkdocs` 소스는 `math = false`일 때에만 `inline_tags = true`를 쓸 수 있다
(`SourceConfig::supports_inline_tags`). 수식을 켠 `mkdocs` 소스가 켜면 설정 오류이며,
메시지는 `math = false`를 말한다. `INLINE_TAG_PARSERS` 목록으로 묻던 자리(설정 검증,
`inline_tags_for`, `audit_file`, 코퍼스 왕복 테스트)는 이 메서드로 묻는다.

코덱 방언은 `Dialect::MkDocs`다. CommonMark의 인라인 규칙으로 읽고 쓰되 다음이 다르다.

- 취소선이 없다(`~~x~~`는 글자). MathJax `\\(…\\)` 가리기도 없다.
- Python-Markdown은 자기 문장부호(``\ ` * _ { } [ ] ( ) > # + - . !``, 표의 `|`)만
  역슬래시를 벗긴다. `\~`, `\<`, `\&`는 역슬래시가 그대로 찍히므로 쓰지 않는다. 원문
  세그먼트가 그 글자를 역슬래시와 함께 썼을 때에만 그대로 되돌린다.

태그 모드에서는 Format 평가 대신 괄호 평가만 돌므로(기존 규칙) Jinja 구분자 검사는
빠진다. macros 플러그인을 쓰는 사이트는 태그를 켜기 전에 이를 감안한다.

### 2.5 검증

- 옵션을 쓰지 않는 기존 프로젝트는 `status --check` 결과가 변경 전과 같아야 한다
  (`mkdocs`: algorithmica, cp-algorithms, stephendiehl. 그 밖의 프로젝트도 `SourceConfig`
  검증이 바뀌므로 함께 본다).
- `crates/parser-mkdocs/examples/roundtrip.rs`는 `--no-math`, `--definition-lists`를
  받아 원문 전체의 무번역·가짜 번역 재구성을 낸다.
- `crates/translate/tests/inline_roundtrip.rs`(무시 테스트)로 원문 전체의 태그 왕복을
  본다.

## 3. 설정 예

```toml
[[sources]]
path = "upstream/docs"
pattern = "**/*.md"
parser = "mkdocs"
output = "ko/docs/{path}"
math = false
definition_lists = true
inline_tags = true
```
