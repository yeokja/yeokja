# cp-algorithms 한국어 번역 설계

이슈: yeokja/yeokja#2

## 목표

[`cp-algorithms/cp-algorithms`](https://github.com/cp-algorithms/cp-algorithms)
(<https://cp-algorithms.com>)를 고정된 원문으로 삼아 yeokja로 한국어 번역하고,
원문과 같은 구성의 MkDocs Material 사이트를 GitHub Pages에 배포한다.

원문은 Python-Markdown 확장 문법을 많이 쓴다. 수식은 arithmatex의 `$`/`$$`,
경고 표시는 `!!!`/`???` admonition, 탭은 `===`다. 현재 `markdown` 파서로는 이
구조가 깨진다(아래 "조사 결과"). 그래서 이 프로젝트는 **새 `mkdocs` 파서**를
포함한다.

완료 상태는 다음 조건을 모두 만족해야 한다.

- 원문의 고정 커밋이 서브모듈로 등록되어 있다.
- 새 `mkdocs` 파서가 cp-algorithms 전체(170개 파일)를 항등 번역으로 재구성했을
  때, Python-Markdown 렌더링 구조가 원문과 같다. `markdown-dialect`의
  이스케이프 백슬래시 버그도 수정되어 있다.
- 범위 안의 모든 대상 세그먼트가 번역 상태이고 `yeokja status --check`가
  통과한다.
- 한국어 사이트가 `mkdocs build --strict`로 빌드된다. 모든 제목 ID가 영어
  원문 ID와 같다(앵커 훅이 개수 불일치 시 실패). 원문에 없던 깨진 로컬 링크가
  없다.
- Pages 배포 배선, 랜딩 항목, README 출처 표기가 갖춰져 있다.

## 조사 결과 (2026-09-20, 설계 조사 스파이크)

- **분량:** 170개 파일, 12,997개 세그먼트, 수식 제외 약 110만 자(약 18.5만
  단어)다. rustc-dev-guide의 약 0.8배다.
- **현재 `markdown` 파서의 문제**(Python-Markdown 렌더링 구조 비교 기준):
  - **admonition:** `!!!`/`???` 96개와 `===` 탭 23개가 11+2개 파일에 있다.
    제목 줄과 4칸 들여쓴 본문이 한 문단으로 합쳐져, 번역하지 않아도(항등)
    구조가 깨진다. 빈 줄 뒤 본문 61개는 들여쓴 코드로 인식되어 번역되지 않는다.
    탭 안의 C++/Python 코드 1,352자가 산문 세그먼트에 섞인다.
  - **수식:** 인라인 11,613개, 디스플레이 700개(여러 줄 175개)가 있다.
    - 수식만 있는 문단 695개가 세그먼트로 제시되어 모델이 건드릴 수 있다.
    - 세그먼트 24개는 `$$` 한쪽만 담는다. 원인은 줄 끝 `\\`, 수식 안의 setext
      `=` 줄, `0. ` 목록 줄이다.
    - `*`, `_`는 바이트 범위 방식이라 무해하다.
  - **이스케이프 백슬래시 버그(모든 Markdown 계열 파서 공통):** `\[`나 `\(`로
    시작하는 문단·셀 23개에서 세그먼트 범위가 백슬래시를 빠뜨린다. 그 결과
    번역문이 `\` 뒤에 붙는다. 원인은
    `crates/parser-markdown-dialect/src/lib.rs`의 `extend_run`이다.
    pulldown-cmark가 이스케이프 문자의 범위에서 백슬래시를 뺀다.
  - **front matter와 제목:** `---\t`로 닫는 front matter(1개)와 공백 없는
    `##Implementation`(1개)이 있다. attr_list 제목 ID `{ #id }` 9개가
    제목 세그먼트 안에 남는다.
  - **Jinja:** `index.md`의 `{% include 'index_body' %}`가 번역 세그먼트로
    제시된다.
- **제목 앵커:** 수작업 앵커 링크 16개가 있다. MkDocs toc는 한글 제목의 ID를
  `_1`, `_2`…로 만든다. 한글 제목으로 전체 빌드했을 때 ID 1,289개 중 1,257개가
  `_N`이 되었다. 링크 2개가 깨졌고 검색 색인 1,094개가 `#_N`을 가리켰다.
  - 영어 빌드의 ID를 제목 순서대로 입히는 Python-Markdown treeprocessor 훅
    시제품으로 1,288/1,289개를 맞췄고 깨진 링크는 0개였다(나머지 1개는 생성
    페이지 `tags.md`).
- **툴체인:** 원문 CI는 버전 고정 없이 pip로 설치한다.
  - nixpkgs에 있는 것: mkdocs 1.6.1, mkdocs-material 9.7.6,
    pymdown-extensions 11.0.1, mkdocs-macros 1.3.9, mkdocs-literate-nav 0.6.2.
    git·rss 플러그인도 있다.
  - nixpkgs에 없는 것: `mkdocs-simple-hooks`(MkDocs 기본 `hooks:`로 대체
    가능), `mkdocs-toggle-sidebar-plugin`(제거 가능).
  - git 계열 플러그인은 yeokja 조립 트리(심볼릭 링크, `.git` 없음)에서 실패한다.
  - 이 설정으로 영어 빌드 19초, 한국어형 트리 빌드 5초가 걸렸다.
- **내비게이션:** `navigation.md`(literate-nav)는 일반 목록이다. 항목에 링크
  밖 텍스트가 한 글자라도 생기면 빌드가 실패한다("Expected no more elements").

## 범위

**번역 대상:**

- `upstream/src/**/*.md`: 알고리즘 문서, `navigation.md`, `index.md`, `tags.md`.
- `upstream/README.md`: 홈페이지 본문. `src/index_body` 심볼릭 링크로
  포함되므로 `ko/src/index_body`로 출력한다.

**제외(`exclude`):**

- `contrib.md`, `code_of_conduct.md`: 원문 기여 절차 문서의 심볼릭 링크다.
  기여는 영어 원문 저장소로 해야 하므로 영어로 둔다.
- `preview.md`: 원문 클라우드 함수로 전송하는 미리보기 도구다.
- `sequences/longest_increasing_subsequence.md`: 내비게이션에 없는
  리디렉션 스텁이다. 산문이 없으면 파서가 세그먼트를 내지 않으므로 규칙에
  두어도 무해하다. 따로 제외하지 않는다.

**보존:**

- 코드 블록.
- 모든 수식(`$..$`, `$$..$$`, `\(..\)`, `\[..\]`, `\begin..\end`).
- front matter의 `tags`, `e_maxx_link`. 태그 이름 "Translated"/"Original"은
  "e-maxx에서 옮김/새로 씀"이라는 원문 메타데이터라 그대로 두고, 번역된
  `tags.md`와 README에서 뜻을 설명한다.
- 링크 URL, 원시 HTML 구조.

**번역:**

- front matter의 `title:` 값(16개): 페이지 `<title>`과 헤더에 쓰인다.
- admonition과 탭의 따옴표 제목.

**라이선스:** 원문은 CC BY-SA 4.0이다. 번역본은 개작물이므로 CC BY-SA 4.0으로
배포하고 다음을 지킨다.

- 저작자 "cp-algorithms contributors"와 저작권 고지, 라이선스 링크, 원문
  링크를 유지한다.
- 번역(수정) 사실을 밝힌다.
- 원문 `mkdocs.yml`의 `copyright` 문구를 한국어로 옮기면서 "한국어 번역본,
  수정됨"을 덧붙인다.
- 비공식 번역임을 밝힌다. 상표 사용권은 주어지지 않는다.

## 접근 방식

### 채택: 새 `mkdocs` 파서 + 영어 ID 주입 훅 + 오버레이 MkDocs 빌드

`parser-myst`처럼 `parser-markdown-dialect`를 재사용하는 방언 파서를 만든다.
원문과 길이가 같은 그림자(shadow) 사본을 만들어, MkDocs 전용 구문은 가리고
본문만 pulldown-cmark가 보게 한다. 재구성은 기존처럼 원문 바이트 범위에
번역을 끼워 넣으므로, 번역 대상 밖의 바이트는 그대로 남는다.

### 제외: 기존 `markdown` 파서 그대로 사용

항등 번역만으로도 7개 파일이 깨지고, 가짜 번역 기준으로는 27개 이상이 깨진다.

### 제외: 빌드 전처리로 admonition을 다른 문법으로 변환

원문과 번역 미러의 문법이 달라져 upstream 갱신 시 증분 번역의 경계가
흐려진다. 파서가 문법을 알면 원문 그대로 쓸 수 있다.

## 파서: `crates/parser-mkdocs`

`crates/parsers/src/lib.rs`의 `parser_by_name`에 `"mkdocs"`로 등록하고 워크스페이스
멤버에 추가한다. `markup()`은 새 변형 `Markup::MkDocs`를 반환한다.

**그림자 사본 규칙:** 원문과 바이트 길이가 같고 줄바꿈 위치가 같아야 한다. 가릴
부분은 줄바꿈을 제외한 모든 바이트를 공백으로 바꾼다. 필요하면 pulldown-cmark가
구조로 읽지 않는 문자를 쓴다.

**`parse_with(&shadow, source, true, extras)` 호출 전 처리:**

1. **front matter:** `---`로 여닫는 블록을 인식한다(닫는 줄 뒤 공백·탭 허용).
   블록 전체를 가리고, `title:` 스칼라 값(따옴표 유무 모두)만 `Extra`
   세그먼트(Heading 성격)로 제시한다.
2. **수식:** `$$..$$`(여러 줄 포함), `$..$`(arithmatex 규칙: 여는 `$` 뒤와 닫는
   `$` 앞에 공백 없음, `\$`는 제외), `\(..\)`, `\[..\]`,
   `\begin{env}..\end{env}`를 찾는다.
   - 그림자에서는 수식 내부를 불활성 문자로 채운다. 그래서 수식 속 `\\`,
     `=` 줄, `0. `, `*`, `_`가 Markdown 구조로 해석되지 않는다.
   - 수식만으로 이루어진 문단(앞뒤 공백 제외)은 번역하지 않는 블록으로 둔다.
   - 인라인 수식은 세그먼트의 원문 텍스트(바이트 범위)에 그대로 남는다.
     그래서 모델이 문맥을 보고, 평가기가 보존을 검사할 수 있다.
3. **admonition과 탭:** 줄 첫머리의 `!!! type "title"`, `??? type "title"`,
   `???+ type "title"`, `=== "title"`를 찾는다.
   - 따옴표 안 제목은 `Extra` 세그먼트(Heading)로 제시한다.
   - 그림자에서는 머리 줄의 들여쓰기 뒤를 `-   ***`와 공백으로 채운다. 이렇게
     하면 pulldown-cmark가 이 줄을 내용 열이 들여쓰기+4인 글머리표 목록 항목으로
     읽는다. 항목의 첫 내용은 수평선이다. `-`와 `*`가 섞여 있어 줄 전체가
     수평선이 되지는 않는다.
   - 그 결과 4칸이나 탭으로 들여 쓴 본문은 빈 줄을 사이에 두어도 이 항목의
     연속 내용이 되고, Python-Markdown이 들여쓰기를 걷어낸 것과 같은 구조로
     읽힌다. 문단, 목록, 코드 펜스 모두 그렇다.
   - 원문 바이트는 바꾸지 않으므로 재구성은 기존 바이트 범위 방식 그대로다.
   - 글머리표 항목은 비어 있지 않으면 문단을 끊을 수 있으므로, 빈 줄 없이
     이어진 머리 줄(16개)도 처리된다.
   - 중첩 admonition은 본문 안의 머리 줄에 같은 규칙을 적용하면 된다.
   - 머리 줄이 7바이트보다 짧으면 `- ***`를 쓴다. 이때 내용 열이 +2라서 상대
     들여쓰기 2칸 이하만 정확하다. 원문의 최소 머리 줄은 9바이트다.
   - 들여쓰기를 걷어내 다시 파싱하고 오프셋 대응표를 두는 방식은 쓰지 않는다.
     그림자만으로 같은 결과를 내며, dialect 크레이트에 새 공개 API가 필요 없다.
4. **공백 없는 ATX 제목:** 줄 첫머리 `#{1,6}` 바로 뒤가 공백이 아닌 글자인
   경우를 찾는다. 그림자에서 `#` 뒤 첫 글자 자리에 공백을 넣어 제목으로 읽게
   한다. 원문 바이트는 그대로다.
5. **Jinja:** `{% ... %}`가 있는 줄과 `{{ ... }}`, `{# ... #}` 구간을 가린다.
6. **attr_list 제목 ID:** `{ #id }`, `{: #id }`, `{#id}` 형식을 처리한다.
   `preserve_heading_ids = true`로 호출하고, `strip_heading_id`를 이 형식까지
   일반화해 제목 세그먼트에서 뺀다(`parser-markdown-dialect` 수정). HTML 블록
   (`<div id=…></div>`) 바로 아래 붙은 제목도 중첩 파싱에서 같은 규칙을 따른다.
   제목의 attr_list에 `data-toc-label="…"`가 있으면(43개) 속성 목록은 가리고,
   라벨 값만 따로 세그먼트로 제시한다.

**재구성:** `parser-markdown`과 같이 `resolve_reference_links` 뒤에
`splice_reconstruct`를 호출한다.

**공통 수정 — `parser-markdown-dialect`:**

- `extend_run`이 이스케이프 문자로 시작하는 범위를 받으면, 원문에서 바로 앞
  바이트가 이스케이프용 `\`인지 확인해 범위에 포함한다. `\\`는 이스케이프된
  백슬래시다.
- 기존 Markdown 계열 프로젝트(markdown, mdx, myst 파서를 쓰는 모든
  `projects/*`)에서 이 수정으로 원문이 바뀌는 세그먼트를 찾는다. 해당
  세그먼트는 지금까지 잘못 재구성되고 있던 것이므로 다시 번역한다.
  재번역 대상이 20개를 넘으면 목록을 기록하고 프로젝트별 별도 커밋으로
  처리한다.

## 평가기와 프롬프트

- **`Markup::MkDocs`:** `FormatEvaluator`는 MkDocs 전용 검사(아래)를 먼저 한다.
  그다음 원문과 번역문의 수식 구간을 같은 자리표시자로 가리고, markup을
  `Markdown`으로 바꿔 기존 검사를 그대로 재사용한다. 수식 속 `_`, `*`, `` ` ``가
  강조·코드 검사에서 오탐을 내지 않게 하려는 것이다. `prompt.rs`의
  `closing_rule`은 Markdown 규칙에 수식 규칙을 더한다. `Markup`을 모든 경우로
  나눠 매칭하는 곳(`constrained_marks` 등)에는 `MkDocs`를 `Markdown`과 같이
  넣는다.
- **수식 보존 검사**(`MkDocs` 전용): 원문과 번역문에서 위 수식 구간을
  추출한다. 원문 수식의 다중집합이 번역문 수식의 다중집합에 바이트 단위로
  포함되어야 한다. 반대로 원문에 없는 수식이 번역문에 생겨도 오류다(원문
  수식을 반복하는 것은 허용). 파일럿에서 모델이 내비게이션 라벨의 평문
  `O(N)`을 `$O(N)$`으로 바꿔 사이드바에 달러 기호가 찍힌 일이 있었다.
  - 일반 `Markdown`에는 적용하지 않는다. "$5 and $10" 같은 산문의 `$`에서
    오탐이 난다.
- **Jinja 구분자 검사**(`MkDocs` 전용): 번역문에 원문보다 많은 `{{`, `{%`,
  `{#`가 나오면 오류다. macros 플러그인이 페이지를 Jinja로 렌더링하기 때문이다.
- **프롬프트:** `Markup::MkDocs`는 Markdown 규칙에 다음을 더한다.
  - "`$...$`, `$$...$$`, `\(...\)` 안의 수식은 바이트 그대로 복사하고 수식 안
    텍스트를 번역하지 말라."
  - "`{{`, `{%`, `{#`를 새로 쓰지 말라."
  - "제목만으로 된 목록 항목이 링크 하나(`[label](target)`)면 결과도 링크
    하나여야 한다."
  - "원문에 없는 수식을 만들지 말라", "외부 문제 제목은 원문 그대로 둔다",
    "인라인 `<br>` 뒤의 텍스트도 같은 줄에 둔다". 마지막 규칙은 응답 파서가
    `[N]`으로 시작하지 않는 줄을 버리기 때문에 필요하다. 이 동작은 모든
    프로젝트에 잠재한 문제라 별도로 고쳐야 한다.

## 제목 앵커: `hooks/ko_anchors.py`

- 설계 조사의 시제품을 다듬어 `projects/cp-algorithms/overlay/hooks/ko_anchors.py`로
  둔다. 한국어 빌드의 `mkdocs.yml`에 `hooks:`로 등록한다.
- 훅은 `on_config`에서 Python-Markdown treeprocessor를 등록한다. attr_list 뒤,
  toc 앞(우선순위 지정)에 실행된다.
- 영어 빌드가 만든 `en-heading-ids.json`에서 페이지별 제목 ID 목록을 읽어
  문서 순서대로 제목에 `id`를 입힌다. JSON은 `{페이지 경로: [id, ...]}` 형식이다.
- 제목 개수가 다르면 페이지 이름과 두 목록을 출력하고 빌드를 실패시킨다.
  번역이 제목 구조를 망가뜨린 경우를 잡는 역할도 한다.
- `tags.md`처럼 내용이 생성되는 페이지는 건너뛴다.
- 영어 ID 목록은 `scripts/extract_heading_ids.py`가 영어 빌드 HTML의
  `<article>` 안 `h1`~`h6`에서 뽑는다.
- 훅과 추출 스크립트는 단위 테스트(`scripts/test_*.py`)를 둔다.

## 빌드

**오버레이와 조립 단계:**

- `[derive]`: `base = "upstream"`, 오버레이는 `ko`(require_base)와
  `overlay`(커밋된 훅 등)다.
- `[[derive.step]]`(`generate`)로 `scripts/prepare_site.py`를 실행한다.
  이 스크립트는 조립 트리의 `mkdocs.yml`과 `src/overrides/partials/content.html`을
  한국어 빌드용으로 바꾼다.
  - `mkdocs.yml`은 알 수 없는 태그(`!!python/name`, `!ENV`)를 보존하는 로더로
    읽어 명시한 항목만 고친다.
  - 기대한 키나 패턴이 없으면 실패한다. upstream이 바뀐 것을 조용히 넘기지 않기
    위해서다.

**`mkdocs.yml` 변경:**

- 제거: `toggle-sidebar`, `mkdocs-simple-hooks`(원문 `hooks.py`는 기본
  `hooks:`로 옮긴다), `git-revision-date-localized`, `git-authors`,
  `git-committers`, `rss`, `extra.analytics`, 기부 배너 JS.
- 추가: `hooks:`에 원문 `hooks.py`(있을 때)와 `hooks/ko_anchors.py`.
- 설정: `theme.language: ko`, `plugins.search.lang: [ko, en]`,
  `site_url: https://yeokja.github.io/yeokja/cp-algorithms/`(실제 Pages 기본
  주소는 구현 시 `pages.yml`에서 확인), `site_name`은 한국어 제목,
  `copyright`는 번역 고지 포함, `edit_uri` 제거.

**`content.html`:** git 정보 블록(`git_info`)을 제거하고, 영어 UI 문구를
한국어로 바꾼다.

**`[build.html]` 명령 순서:**

1. **영어 빌드:** `prepare_site.py --english`로 영어용 설정을 만든다. 한국어
   설정과 같은 플러그인 구성이고, `ko_anchors` 훅과 언어 설정만 없다.
   원문 트리를 복사한 `build/original-source`에서 `mkdocs build --strict`로
   `build/original`을 만든다.
2. `extract_heading_ids.py build/original > build/en-heading-ids.json`.
3. **한국어 빌드:** 조립 트리에서 `mkdocs build --strict`로 `public/`을 만든다.
4. `check_links.py build/original public`: 원문에 없던 깨진 로컬 링크나
   앵커가 있으면 실패한다. webassembly-component-docs의 스크립트를 복사한다.
5. `public/`을 `site`로 옮긴다. `outputs = ["site"]`.

**툴체인:** `nix/projects/cp-algorithms.nix`에
`python3.withPackages (ps: [ mkdocs mkdocs-material pymdown-extensions
mkdocs-macros mkdocs-literate-nav ps.pyyaml ])`를 둔다. MkDocs 2.0 경고
배너는 `NO_MKDOCS_2_WARNING=true`로 끈다(mkdocs-material 9.7.6의
`material/templates/__init__.py` 기준).

## 번역 설정

```toml
[[sources]]
path = "upstream/src"
pattern = "**/*.md"
exclude = ["contrib.md", "code_of_conduct.md", "preview.md"]
parser = "mkdocs"
output = "ko/src/{path}"

[[sources]]
path = "upstream"
pattern = "README.md"
parser = "mkdocs"
output = "ko/src/index_body"
```

- 두 번째 규칙은 고정 출력 경로를 쓴다. yeokja가 `{path}` 없는 출력 템플릿을
  지원하지 않으면 다음 순서로 처리한다.
  1. `README.md`를 `ko/README.md`로 출력한다.
  2. `prepare_site.py`가 조립 트리의 `src/index_body`를 `README.md` 사본으로
     바꾼다.
- `[provider]`: `claude_code` / `claude-sonnet-5`.
- `[evaluation]`: `auto_evaluate = true`, `max_retries = 3`.
- `[translation]`: `concurrency = 8`, `batch_segments = 32`.

**용어집:** 경쟁 프로그래밍·알고리즘 표준 용어로 시작한다. #3과 공통 부분이
많다.

- 대역: segment tree → 세그먼트 트리, Fenwick tree → 펜윅 트리, disjoint set
  union → 서로소 집합 유니온, suffix array → 접미사 배열, convex hull → 볼록
  껍질, modular inverse → 모듈러 역원, sieve → 체, dynamic programming → 동적
  계획법, shortest path → 최단 경로, minimum spanning tree → 최소 신장 트리,
  strongly connected component → 강한 연결 요소, bipartite graph → 이분
  그래프, flow → 흐름, matching → 매칭, complexity → 복잡도.
- 인명과 알고리즘 고유명사(Dijkstra, Kruskal, Tarjan 등)는 원어로 둔다.

## 배포 배선

- `.github/pages-projects.json`: `{"project": "cp-algorithms", "target": "html",
  "artifact": "dist-cp-algorithms", "artifact_path":
  "projects/cp-algorithms/dist"}`.
- `stage-pages.sh`: `overlay_site "dist-cp-algorithms" "site" "cp-algorithms"`.
- `rebuild-translations.sh`: `cp-algorithms)` 분기에서 두 source 모두 먼저
  `status --check`한 뒤 `translate`로 `ko/`를 재구성한다. hott,
  putting-the-you-in-cpu와 같은 순서라 CI가 번역 provider를 부르지 않는다.
- `pr.yml`: `projects/cp-algorithms/scripts` 단위 테스트 스텝을 추가한다.
  파서 테스트는 `cargo test --workspace`에 포함된다.
- `site/index.html`: "알고리즘" 주제(#3과 공유)에 항목을 추가한다.

## README

다음을 적는다.

- 범위와 제외.
- 라이선스(CC BY-SA 4.0, 개작물 고지).
- 태그 "Translated"/"Original"의 뜻.
- 재현 명령.
- AGENTS.md 출처 표기: [yeokja](https://github.com/yeokja/yeokja) 링크, 실제
  모델, 학습 비허용 문장.
- 비공식 번역 고지.

## 검증

- **파서 단위 테스트:** 각 구문(front matter 탭 종료, `$`/`$$` 여러 줄,
  `\(`로 시작하는 문단, admonition 머리·본문·중첩, 빈 줄 뒤 본문, 탭, `##X`,
  Jinja, attr_list 제목 ID)마다 세그먼트와 재구성을 검사한다.
  `extend_run` 수정은 `parser-markdown` 테스트로 검사한다.
- **말뭉치 검사:** cp-algorithms 전체를 항등 재구성하고 "가 " 접두 가짜
  번역으로 재구성한 뒤, Python-Markdown(원문 확장 구성) 렌더링 구조를 원문과
  비교한다. 이 스크립트는 설계 조사 스파이크의 `validate.py`를 다듬어
  `scripts/check_parser_corpus.py`로 둔다.
  - 항등: 차이 0개.
  - 가짜 번역: 산문 외 구조 차이 0개.
- `yeokja status --check`: 두 source 모두.
- `yeokja evaluate --mechanical-only`: 수식 보존과 Jinja 검사 포함.
- `mkdocs build --strict`가 영어·한국어 모두 성공한다. 앵커 훅의 개수 불일치가
  없고, `check_links.py`가 `New: 0`이다.
- **표본 확인:** 다익스트라 문서의 수식 렌더링, admonition 접기(`???`), C++/Python
  탭, 검색에서 한국어 검색어, `sieve-of-eratosthenes.html#segmented-sieve`
  링크 이동.
- `.github/scripts/rebuild-translations.sh cp-algorithms`로 CI 재구성 경로를
  재현한다.

## 위험과 대응

- **세그먼트의 45%가 수식 포함:** 수식 보존 평가기와 재시도로 처리한다.
  재시도 뒤에도 남는 오류는 state를 직접 고친다.
- **literate-nav 실패:** 빌드가 즉시 잡는다. 프롬프트 규칙과 평가기의
  링크 검사로 줄인다.
- **파서 복잡도:** admonition 본문 재파싱과 오프셋 대응이 핵심이다. 말뭉치
  검사를 통과하기 전에는 번역을 시작하지 않는다.
- **upstream이 활발함:** 고정 커밋 기준으로 번역한다. 갱신은 서브모듈을 올리고
  증분 번역한다.
- **MkDocs 2.0 전환:** nixpkgs 고정 버전을 쓴다.
