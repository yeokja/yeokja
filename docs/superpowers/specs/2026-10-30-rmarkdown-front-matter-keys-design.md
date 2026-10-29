# R Markdown 프런트매터 키 옵션 설계

`rmarkdown` 파서가 YAML 프런트매터에서 내놓는 값을 소스 옵션으로 넓힌다. 첫
사용처는 Quarto 책 *Learn in Public*(`projects/learninpublic`, `*.qmd`)이다.

## 1. 배경

이 책의 각 장은 요약을 프런트매터의 중첩 키에 두고, `description`과 소셜 메타는
YAML 앵커로 그 값을 가리킨다.

```yaml
---
summary:
  sentence: &chapter_summary "The fastest way to learn is to learn in public."
  paragraph: "A longer paragraph.
    It may continue over several lines inside the quotes."
description: *chapter_summary
---
```

`summary.sentence`와 `summary.paragraph`는 사이트의 장 카드, 검색 미리보기, 소셜
메타에 쓰이므로 번역되어야 한다. 지금 파서는 최상위 `title`·`subtitle`·`description`만
읽고(`YAML_TEXT_KEYS`, `yaml_values`), 중첩 키는 보지 않으며, `description`은 별칭
(`*chapter_summary`)이라 아무것도 내놓지 못한다. 별칭은 번역할 것이 없으므로 지금
동작이 맞고, 앵커가 붙은 원본 값만 번역하면 별칭은 번역된 값을 따라간다.

## 2. 결정

### 2.1 소스 옵션 `front_matter_keys`

`SourceConfig`에 `front_matter_keys: Vec<String>`(serde 기본값 빈 목록)을 둔다.
`inline_code_captures`처럼 `rmarkdown` 전용이며, 다른 파서의 소스가 설정하면
`ProjectConfig::from_toml`이 거부한다. 각 항목은 점으로 이은 키 경로
(`summary.sentence`)다. 빈 항목이나 빈 조각(`summary.`, `.sentence`)도 거부한다.

파서 선택(`crates/parsers`)은 `inline_code_captures`를 넘기는 자리에서
`front_matter_keys`도 함께 넘긴다. `RmarkdownParser::with_patterns`는 그대로 두고,
두 옵션을 받는 `RmarkdownParser::configured(patterns, keys)`를 더한다.

### 2.2 값의 위치 찾기

YAML을 파싱하지 않고 들여쓰기 구조로 찾는다. 경로의 첫 조각은 들여쓰기 없는
`key:` 줄이고, 다음 조각은 그 줄 뒤에 오는, 더 깊이 들여쓴 `key:` 줄 중 앞 조각보다
깊고 같은 깊이의 첫 일치다. 중간에 부모 깊이 이하의 줄이 나오면 그 매핑은 끝난다.
경로가 가리키는 줄의 값은 기존 최상위 키와 같은 규칙(2.3)으로 자른다.

옵션으로 찾은 값은 `BlockType::Paragraph` Literal이다. 옵션이 비어 있으면 기존
`title`·`subtitle`·`description` 동작만 남고, 옵션이 있어도 그 셋은 그대로 내놓는다.
한 값이 두 경로에 걸리더라도(예: `description`과 옵션 `description`) 한 번만 낸다.

### 2.3 값 읽기

기존 `yaml_values`의 값 자르기를 한 함수(`yaml_scalar`)로 떼어 최상위 키와 경로 키가
같이 쓴다. 규칙:

- `&anchor ` 접두는 값 밖이다. 앵커 이름은 YAML 앵커 문자(공백·흐름 지시자 제외)로
  읽고, 그 뒤 공백을 건너뛴 자리부터 값이다.
- 큰따옴표·작은따옴표 값은 닫는 따옴표까지, 여러 줄이어도 된다(기존 동작).
- 평문 값은 줄 끝까지에 더 들여쓴 이어지는 줄을 붙인다(기존 동작).
- `|`·`>` 블록 스칼라와 별칭(`*name`)은 내놓지 않는다.
- 값이 없거나 공백뿐이면 내놓지 않는다.

재구성은 바뀌지 않는다. `quote_literal`이 프런트매터 안 Literal의 따옴표 규칙을 이미
처리하며, 앵커는 스팬 밖이므로 그대로 남는다.

### 2.4 검증 범위

같은 원문에서 확인하는 두 가지는 코드 변경 없이 회귀 테스트만 둔다.

- `$…$` 인라인 수식은 Pandoc `tex_math_dollars` 규칙(여는 `$` 바로 뒤 비공백, 닫는
  `$` 바로 앞 비공백이고 뒤에 숫자 없음)을 따른다. `work at $FAMOUS_COMPANY on
  $HYPED_TECHNOLOGY`와 `raised at $60m then $800m`은 산문이다.
- Quarto 속성이 붙은 이미지 `![A caption](x.png){fig-alt="…"}`는 캡션만 내놓고
  `{…}`는 재구성 때 그대로 남는다. `fig-alt` 값은 이 설계의 범위 밖이다.

## 3. 설정 예

```toml
[[sources]]
path = "upstream/chapters"
pattern = "*.qmd"
parser = "rmarkdown"
output = "ko/{path}"
inline_tags = true
front_matter_keys = ["summary.sentence", "summary.paragraph"]
```
