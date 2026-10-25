# XML 어휘 프로필 설계

`xhtml` 파서(`docs/superpowers/specs/2026-10-22-xhtml-parser-design.md`)를 **XML 어휘
프로필**로 일반화해, 같은 판독기·문장 분할·범위 재구성·인라인 태그 전송으로 OpenStax
CNXML과 PreTeXt(MathBook XML) 책을 번역한다. 새 파서 이름은 `cnxml`과 `pretext`다.
`xhtml`의 동작은 바뀌지 않는다.

## 1. 배경

조사 표본은 두 권이다.

- *Principles of Finance 2e*(OpenStax, `github.com/openstax/osbooks-principles-finance`).
  CNXML 모듈 132개(`modules/mNNNNN/index.cnxml`)와 컬렉션 문서 1개. MathML 수식 785개,
  그중 `mtext` 462개에 영어 단어가 있다.
- *Interactive Linear Algebra*(`github.com/QBobWatson/ila`). MathBook XML(PreTeXt의 전신)
  `src/*.xml` 40개. `m` 7,886개, 디스플레이 수식 `me` 1,328개(그중 `p` 안 1,157개, 수식 뒤에
  문장이 이어지는 경우 274개), TikZ `latex-code` 253개, 색인 `idx` 712개, 판 구분
  `restrict-version` 48개(문단 묶음도, 문장 일부도 감싼다), 번역할 속성 `type-name` 33개.

기존 `xhtml` 파서로 읽으면 CNXML은 세그먼트 434개(최대 86,128자), PreTeXt는 104개다.
블록 표가 XHTML 이름뿐이라 `para`·`section`·`item`이 인라인으로 읽히고, 블록 순회가
블록이 아닌 요소 안으로 내려가지 않아 루트 아래 전체가 한 구간(run)이 되기 때문이다.

검토한 방향과 판단:

| 방향 | 판단 |
|---|---|
| A. 어휘마다 파서 크레이트를 따로 만든다 | 기각. XML 판독기, 열린 인라인 안 분할점 억제, 범위 재구성, 태그 코덱이 그대로 겹친다. |
| B. `yeokja.toml`에 요소 목록(블록·건너뛰기·코드·속성)을 적게 한다 | 기각(1단계). CNXML `emphasis`의 `effect` 속성, MathML `mtext` 섬, 마침표를 품은 디스플레이 수식, PreTeXt의 "나머지는 블록" 같은 규칙은 목록으로 쓰이지 않는다. 같은 어휘를 쓰는 프로젝트마다 표를 복사하게 된다. |
| **C. 파서 크레이트 안의 어휘 열거형 하나가 모든 분류 질문에 답하고, 파서 이름이 어휘를 고른다** | **채택.** 파서와 태그 코덱이 같은 답을 쓴다. 어휘를 더하는 일은 표와 `match` 갈래 몇 개다. |

## 2. 결정

### 2.1 구조

- `crates/parser-xhtml/src/vocab.rs`의 `Vocabulary { Xhtml, Cnxml, Pretext }`가 요소에 대해
  묻는 질문을 모두 맡는다.

  | 질문 | 메서드 |
  |---|---|
  | 구간을 끊는가 | `breaks_run` (블록 이름 또는 블록을 품은 요소) |
  | 번역하지 않는가 (`translate="no"` 포함) | `is_untranslated` |
  | 코드처럼 원자인가 | `is_code` |
  | 값을 번역하는 속성인가 | `translates_attr` |
  | 인라인 태그 종류 (i/b/a/h) | `inline_kind` |
  | 제목만 읽는 컨테이너인가 | `head_title` |
  | 건너뛰는 요소 안에서 번역할 섬 | `island` |
  | 표·행·셀·머리 구역 | `is_table`, `is_row`, `is_cell`, `is_table_head` |
  | 제목 레벨에 드는 절 요소인가 | `is_sectioning` |
  | 구간의 블록 종류 | `block_kind` |
  | 문장을 끝내는 디스플레이 수식인가 | `ends_sentence` |
  | 골격 검사에서 있는지만 보는 강조인가 | `is_emphasis` |

- 파서는 `XhtmlParser`, `CnxmlParser`, `PretextParser` 세 단위 구조체다. 구현은 하나이고
  어휘만 다르다. `skeleton_with(vocab, source)`가 골격을 만들고, `skeleton(source)`는
  XHTML이다.
- **`Markup`은 `Xhtml`을 공유한다.** 마크업이 쓰이는 곳(`closing_rule`이 빈 문자열,
  `evaluator_format`의 보정 없음, 링크 평가기가 엔티티를 읽은 뒤 URL 추출)은 세 어휘에서
  똑같이 동작해야 한다. 변형을 따로 두면 같은 갈래를 세 번 쓰게 된다.
- 번역 크레이트는 `Dialect::{Xhtml, Cnxml, Pretext}`를 두고 `Dialect::vocabulary()`로
  어휘를 얻는다. `inline/xhtml.rs`는 하드코딩한 `CODE`·`is_untranslated` 대신 어휘에
  묻는다. 파서 크레이트가 공개하던 `CODE`·`SKIP`·`is_untranslated`는 없앤다.

### 2.2 어휘 공통 규칙

- **블록을 품은 인라인 요소는 블록처럼 내려간다.** 건너뛰는 요소가 아닌데 자손에 블록
  요소가 있으면 구간을 끊고 그 안으로 재귀한다. 이런 요소는 투명하다: 절 깊이와 표 위치를
  바꾸지 않고, 자기 바로 아래 텍스트는 `Paragraph` 구간이 된다. 1장의 "문서 전체가 한
  구간" 문제를 이 규칙과 어휘별 블록 표가 함께 고친다. technicalpm XHTML에는 이런 요소가
  없어 결과가 같다(4장).
- **건너뛰는 요소는 구간을 끊지 않는다.** 구간 안에서는 `<xN/>`로 모델에 보이고 원문
  바이트로 돌아온다. 블록 표에도 있는 요소(XHTML `pre`, CNXML `preformat`)만 블록이라
  끊고 건너뛴다.
- **표 머리 행.** 셀의 `label_row`는 표의 첫 행이거나 표 머리 구역(`thead`) 안의 행이다.
  CNXML 표는 `thead`에 두 행을 두는 경우가 있다.
- **제목 레벨.** `title`의 레벨은 둘러싼 절 요소의 수다. 부모가 절 요소가 아니면(그림·표·
  노트·예제·정의의 제목) 그 수에 1을 더해, 둘러싼 절 제목 아래에 놓인다. 레벨은
  `surroundings`의 제목 스택이 쓰므로, 그림 제목이 절 제목을 밀어내지 않아야 한다. 레벨은
  파일 안에서 상대적이다(따로 포함되는 PreTeXt 절 파일은 `section`이 1).
- **세그먼트 사이 공백.** 재구성은 한 구간의 세그먼트 사이에 원문의 공백 바이트를 그대로
  둔다(`join_segments_with_translations`의 한 칸 대신). CNXML 모듈 두 개(m00079, m00107)가
  문장 사이에 두 칸을 쓴다. 원문 공백이 한 칸이면 결과가 같다.
- **섬.** 속성 값 번역을 일반화한 `Item::Inner { host, value }`다. 호스트는 속성이면 시작
  태그, `mtext`면 둘러싼 `math` 전체다. 구간 번역이 호스트 바이트를 원문 그대로 지니면 그
  자리를 갱신한 호스트로 바꾸고, 구간이 번역되지 않았으면 호스트를 직접 갈아 끼운다. 섬
  값은 `BlockRole::Literal` 블록이고, 앞뒤 공백은 범위 밖이라 그대로 남는다.

### 2.3 CNXML 프로필

| 항목 | 내용 |
|---|---|
| 블록 | `document` `content` `section` `para` `list` `item` `title` `caption` `figure` `subfigure` `table` `tgroup` `thead` `tbody` `tfoot` `row` `entry` `note` `exercise` `problem` `solution` `commentary` `example` `rule` `statement` `proof` `glossary` `definition` `meaning` `seealso` `equation` `footnote` `quote` `label` `preformat` `metadata` `collection` `subcollection` |
| 건너뛰기 | `math` `preformat` `iframe` `object` `embed` |
| 코드(원자) | `code`, 그리고 `span.primary-function`·`span.secondary-function`(계산기 키 `2ND`, `ENTER` 376개: 기기에 인쇄된 이름) |
| 번역 속성 | `alt`(`media`), `summary`(`table`) |
| 제목 컨테이너 | `metadata` → `md:title`만 `Heading`(레벨 없음). `md:content-id`·`md:uuid`·`md:license` 등은 번역하지 않는다. |
| 섬 | `math` 안의 `mtext` |
| 인라인 종류 | `emphasis`: `effect="italics"` → i, `bold` 또는 없음(CNXML 기본값) → b, 그 밖 → h. `term` → b, `foreign`·`cite-title` → i, `link` → a(내용 없는 `link`는 `<xN/>`), 나머지 → h |
| 표 | `table`/`row`/`entry`, 머리 구역 `thead` |
| 절 요소 | `document` `section` `collection` `subcollection` |
| 블록 종류 | `title` → `Heading`, `item` → `ListItem`, `entry` → `Table`(셀), `quote` → `BlockQuote`, 나머지 → `Paragraph` |

- **`footnote`는 블록이다.** 113개 모두 문장 끝에 붙은 출처라, 각주 앞뒤의 문장이 다른
  블록이 되어도 어순이 섞일 일이 없다. 각주 본문이 제 구간이 된다.
- **`media`는 블록이 아니다.** 문단 안의 그림일 수 있다. `alt`만 번역하고 안의 `image`는
  빈 요소, `iframe`은 건너뛴다.
- `mi` 안의 단어(`<mi>Revenue</mi>`)는 수식의 식별자로 보고 번역하지 않는다.

### 2.4 PreTeXt 프로필

"인라인 화이트리스트, 나머지는 블록"이다. PreTeXt의 구조 요소(`definition`,
`statement`, `objectives`, `bluebox`, `specialcase` 같은 책별 요소 포함)는 목록 없이
블록이 된다.

| 항목 | 내용 |
|---|---|
| 인라인 | 텍스트 표시 `em` `term` `alert` `q` `sq` `dq` `c` `m` `xref` `url` `fn` `sup` `sub` `foreign` `pubtitle` `articletitle` `booktitle` `abbr` `acro` `init` `email` `var` `taxon` `personname` `quantity` `kbd` `tag` `tage` `attr` `insert` `delete` `stale` `icon` `fillin`; 문자·생성어 `rsq` `lsq` `ldq` `rdq` `ndash` `mdash` `nbsp` `ellipsis` `today` `trademark` `registered` `copyright` `less` `greater` `lt` `gt` 등; 판 구분 `restrict-version` |
| 빈 요소 | 목록에 없어도 블록이 아니다. 산문을 담지 않으므로 구간을 끊을 이유가 없다(`<xi:include/>`, 목록에 없는 문자 요소). |
| 코드(원자) | `m`, `c`. 백틱 스팬으로 모델에 보이고 바이트 그대로 돌아온다. |
| 건너뛰기 | `me` `men` `md` `mdn` `latex-code` `latex-image-code` `mathbox` `image` `macros` `docinfo` `idx` `usage` `pre` `cd` `program` `sage` `console` `input` `output` `sageplot` `asymptote` `video` `interactive` |
| 번역 속성 | `type-name` |
| 인라인 종류 | `em` `foreign` `pubtitle` `articletitle` `booktitle` → i, `term` `alert` → b, `xref` `url` → a(내용 없는 `xref`는 `<xN/>`), 나머지 → h |
| 표 | `tabular`/`row`/`cell` |
| 절 요소 | `book` `article` `part` `chapter` `section` `subsection` `subsubsection` `appendix` `preface` `acknowledgement` `colophon` `foreword` `biography` `exercises` `worksheet` `reading-questions` `references` `glossary` `solutions` `index` `paragraphs` `frontmatter` `backmatter` |
| 블록 종류 | `title`·`subtitle`·`shorttitle` → `Heading`, `li` → `ListItem`, `cell` → `Table`(셀), `blockquote` → `BlockQuote`, 나머지(`caption` 포함) → `Paragraph` |

- **`p` 안의 디스플레이 요소**(`me`, `md`, `men`, `mdn`, `latex-code`)는 건너뛰는 요소라
  구간을 끊지 않고 구간 안의 `<xN/>`가 된다. 수식 뒤 문장이 같은 구간에 남아 한국어 어순으로
  수식 앞뒤를 옮길 수 있다.
- **마침표를 품은 디스플레이 수식은 문장 끝이다.** PreTeXt는 문장 부호를 수식 안에 둔다
  (`… one column: <me>v = \vec{1 3}.</me> We will also write …`). 수식 `me`·`men`·`md`·`mdn`의
  마지막 텍스트가 `.`·`?`·`!`로 끝나면 문장 분할에 그 부호 하나로 보인다. 다음 문장은 제
  세그먼트가 되고, 수식은 자기가 끝내는 문장에 붙는다. 부호가 없으면(`z = 3,</me> which …`)
  문장이 이어진다. 이 규칙이 없으면 ILA 전체 세그먼트가 5,326개, 있으면 5,865개다.
- **`p` 안의 `ol`·`ul`은 구간을 끊는다.** 목록 앞 글, 목록 항목들, 목록 뒤 글이 각각 구간이다.
- **`restrict-version`은 투명하다.** 인라인으로 쓰이면 `<hN>` 태그로 보존되고 안의 텍스트는
  구간에 든다. 문단이나 목록 항목을 감싸면 블록을 품은 요소로 내려간다(2.2). 두 판(default,
  1553)의 텍스트를 모두 번역한다.
- **색인 `idx`는 1단계에서 번역하지 않는다.** 색인어 1,414개는 한국어 정렬 키(`sortby`)와
  함께 정해야 하고, 그 방식은 빌드(색인 생성)와 묶인 프로젝트 결정이다. 문단 안에서는
  `<xN/>`로 남는다.
- **`notation`의 `usage`는 수식이다.** `<usage>{1\choose 2}</usage>`처럼 LaTeX를 텍스트로
  담는다. `description`은 번역한다.

### 2.5 인라인 태그 전송

- 세 어휘 소스는 **언제나 태그로 주고받는다.** `yeokja_core::config::XML_PARSERS`
  (`xhtml`, `cnxml`, `pretext`)에 들면 `SourceConfig::uses_inline_tags`가 참이고,
  `INLINE_TAG_PARSERS`에도 든다. 커스텀 `prompt_template`에 `{inline_tags}` 자리표시자가
  없으면 설정 오류다.
- 코덱은 어휘에 묻는다. 코드 원자(`<code>`, CNXML 계산기 키, PreTeXt `m`·`c`)는 백틱
  스팬, 건너뛰는 요소·빈 요소·글자 없는 요소는 `<xN/>`, 그 밖은 `inline_kind`대로
  `<iN>`·`<bN>`·`<aN>`·`<hN>`이다. XHTML의 각주 참조(`a > sup.footnote`)는 XHTML 프로필에만
  있는 불투명 규칙이다. 섬 값(`mtext`)은 속성 값처럼 평문(`Position::Plain`)으로 간다.
- Markdown 번역 감사(`audit_file`)는 세 어휘를 모두 건너뛴다.

### 2.6 등록과 설정

- `parser_by_name`에 `cnxml`, `pretext`. 확장자 자동 감지는 `.cnxml`을 더한다. `.xml`은
  어휘를 알 수 없으므로(컬렉션 문서, PreTeXt) `[[sources]]`에서 이름으로 지정한다.

```toml
# OpenStax CNXML
[[sources]]
path = "upstream/modules"
pattern = "*/index.cnxml"
parser = "cnxml"
output = "ko/modules/{path}"

[[sources]]
path = "upstream/collections"
pattern = "*.collection.xml"
parser = "cnxml"
output = "ko/collections/{path}"

# PreTeXt / MathBook XML
[[sources]]
path = "upstream/src"
pattern = "*.xml"
exclude = ["gfdl.xml", "schemas.xml"]
parser = "pretext"
output = "ko/src/{path}"
```

### 2.7 골격 동치 검사

`skeleton_with`는 파서와 같은 `breaks_run`으로 블록을 가르고, 블록은 순서와 중첩까지,
블록 안 인라인 요소는 개수까지(multiset) 본다. `is_emphasis`(인라인 종류 i·b)인 요소는
있는지만 본다. 번역 속성 값은 지우고, 텍스트(`mtext` 포함)는 보지 않는다. 쓰는 프로젝트는
테스트에서 이 함수를 쓰거나 같은 비교를 검사 스크립트로 한다
(`projects/technicalpm/scripts/check.py`가 선례).

## 3. 범위 밖

- OpenStax 연습문제 580개의 본문(`link class="os-embed"`가 가리키는 Exercises API), 각주·
  그림·표 번호, 렌더러. 프로젝트 빌드가 맡는다.
- 수식 안의 영어: CNXML `mi`, PreTeXt `m` 안의 `\text{}`·`\sptxt{}`(293곳), TikZ 라벨.
- 색인(`idx`) 번역.
- 어휘 정의의 프로젝트별 덮어쓰기(건너뛸 요소·클래스, 번역 속성 목록).
- 없는 파서 이름이 경고 없이 Markdown으로 떨어지는 레지스트리 동작.

## 4. 검증

- **XHTML 불변.** technicalpm 원문 30개 문서: 블록 4,073개, 세그먼트 8,858개, 무변환 왕복
  바이트 동일, 골격 동치(변경 전과 같은 수). `yeokja status upstream/OEBPS`는 문서 28개, 세그먼트
  8,637개이고 `status --check`가 통과한다(저장된 세그먼트 id·해시가 그대로다). 저장된 번역으로
  다시 쓴 `ko/OEBPS` 28개 문서가 변경 전 바이너리의 출력과 바이트 단위로 같다.
- **픽스처.** `crates/parser-xhtml/tests/fixtures/`에 두 사본의 발췌를 둔다(머리 주석에 출처와
  라이선스). 세그먼트 수: CNXML `m00002` 73, `m00011`(표) 25, `m00026`(`mtext` 수식) 75,
  `m00050`(문단 안 `mtext`) 33, 컬렉션 2. PreTeXt `ila.xml` 32, `vectors.xml` 38,
  `cplx-eigenvals.xml`(`restrict-version`) 20. `tests/vocabularies.rs`가 확인하는 것:
  - 무변환 왕복이 바이트 동일하고, 번역이 없어도 원문 그대로다.
  - 모든 텍스트를 한국어로 바꾼 번역이 골격 동치이고 다시 읽으면 블록 수가 같다.
  - CNXML: `md:title`만 번역, 제목 레벨, 각주가 문단을 끊음, 용어집, `thead` 두 행의 머리 행,
    `summary=" "` 제외, `mtext` 번역(구간이 번역되지 않았을 때와 번역됐을 때 모두),
    `mi` 보존, 컬렉션의 책·장 제목.
  - PreTeXt: `restrict-version` 안의 두 판 제목, `type-name` 번역, `p` 안 목록이 구간을 끊음,
    `p` 안 `me`가 문장에 붙음, TikZ가 구간 안에 남음, `idx`·`usage` 제외, 투명한
    `restrict-version`.
  - 단위 테스트(`src/tests.rs`): 블록을 품은 인라인 요소, 세그먼트 사이 원문 공백, 문단 안
    `restrict-version`, 마침표를 품은 `md`와 품지 않은 `me`, 수식·코드만 있는 구간 제외,
    `mtext` 앞뒤 공백, CNXML 강조 분할 골격. 코덱 테스트(`inline/xhtml.rs`): CNXML 의미 태그
    매핑, 계산기 키와 `math`, PreTeXt `m`·`me`·`xref`.
- **전체 사본** (`YEOKJA_XHTML_CORPUS=… YEOKJA_XHTML_CORPUS_PARSER=cnxml|pretext cargo test -p
  yeokja-parser-xhtml --test corpus -- --nocapture`, 그리고 임시 프로젝트의 `yeokja status`·
  `coverage`):

  | | CNXML 모듈 132개 | PreTeXt `src` 40개 |
  |---|---|---|
  | 무변환 왕복·골격 동치 | 132/132 | 40/40 |
  | 블록 | 8,741 | 3,192 |
  | 세그먼트 | 15,212 | 5,865 |
  | 블록 종류별 세그먼트 | Paragraph 10,505, Table 1,566, Heading 1,391, ListItem 941, Literal 809 | Paragraph 4,466, ListItem 824, Heading 542, Literal 33 |
  | 원문 길이(자) 중앙값 / p90 / p99 / 최대 | 90 / 202 / 380 / 2,225 | 102 / 273 / 1,400 / 6,432 |
  | `yeokja status` | 132개(모듈 131 + 컬렉션), 15,230 (모듈만 15,209) | 38개(`gfdl.xml`·`schemas.xml` 제외), 5,717 |
  | `yeokja coverage` 텍스트 줄 중 번역 대상 | 36% | 61% |

  - CNXML `coverage`에서 번역 대상이 아닌 5줄 이상 구간은 모두 마크업으로 시작한다
    (`</section>`, `</row>`, `<tgroup>`, `<iframe>`, `<code display="block">`, 본문 없는
    `exercise`, 숫자만 있는 `entry`). 줄 비율이 낮은 것은 표와 MathML이 줄을 많이 차지하기
    때문이다.
  - PreTeXt에서 남는 구간은 수식이 이어지는 `</p>`, XML 선언과 라이선스 주석, `mathbox`,
    `idx`다. p99가 큰 것은 세그먼트 원문이 TikZ·디스플레이 수식 바이트를 지니기 때문이고
    모델에는 `<xN/>` 하나로 보인다.
  - 조사 보고서의 PreTeXt 추정(7천~9천)보다 적은 것은 디스플레이 수식과 TikZ에서 문단을
    끊지 않고, 색인과 `usage`를 번역하지 않기 때문이다.
  - **태그화.** 두 사본으로 `inline_roundtrip`(무시 표시된 말뭉치 테스트)을 돌리면 태그화할 수
    없는 세그먼트가 0개다(CNXML 15,233, PreTeXt 5,865). 태그화 → 판독 → 직렬화 결과가 원문과
    바이트로 다른 1,519개는 공백 접기와 엔티티 표기(`&#160;`, `&#39;`, `&gt;`) 차이뿐이다.

## 5. 한계와 되짚을 수 있는 결정

- **어휘는 코드에 있다.** 프로젝트가 특정 클래스·요소를 건너뛰거나 속성을 더 번역해야 하면
  설정으로 여는 안(방향 B)을 다시 본다.
- **같은 구간 안의 같은 호스트.** 한 구간에 바이트까지 같은 `math`(또는 같은 시작 태그)가
  둘 있고 섬 번역이 서로 다르면 첫 번역이 둘 다에 쓰인다. 속성 번역에서 물려받은 한계다.
- **세그먼트 원문에 수식 바이트가 든다.** 원문 TikZ나 디스플레이 수식이 바뀌면 그 문장
  세그먼트가 stale이 된다.
- **표 열 번호**는 CNXML `namest`/`nameend` 병합을 세지 않는다.
- **문장 분할기의 약어 처리**(`Ben S. Bernanke`, `etc.`)는 공용 `split_sentences`를 따른다.
- **문장 끝 수식 규칙**은 수식의 마지막 글자만 본다. `\rlap.`처럼 끝에 쓴 마침표는 문장
  끝으로 읽히지만 `\text{.}`처럼 감싼 마침표는 읽히지 않는다. 마침표를 수식 밖에 쓰는
  원문에서는 이 규칙이 쓰일 일이 없다.
- **CNXML `footnote` 블록.** 문장 중간 각주가 나오는 책이면 인라인 불투명 요소로 바꾸는 안을
  본다.
