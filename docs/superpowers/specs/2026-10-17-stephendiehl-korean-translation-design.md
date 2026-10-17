# Stephen Diehl 블로그 한국어 번역 설계

이슈: yeokja/yeokja#8

## 목표

Stephen Diehl의 개인 블로그 <https://www.stephendiehl.com>의 글을 yeokja로
한국어 번역해 mdBook 사이트로 GitHub Pages에 배포한다.

이 블로그는 **공개된 원고 저장소가 없다.** 공개된 것은 정적 HTML뿐이다. 그래서
먼저 HTML에서 영어 Markdown 원고를 결정적으로 복원해 `source/`로 커밋하고
(jeffe-algorithms 선례), 그 원고를 Markdown 계열 파서로 번역한다.

완료 상태는 다음 조건을 모두 만족해야 한다.

- `/posts/` 색인의 모든 글이 `projects/stephendiehl/source/posts/<slug>.md`로
  복원되어 있고, 복원 스크립트가 캐시에서 같은 결과를 다시 만든다.
- 복원된 원고의 모든 대상 세그먼트가 번역 상태이고 `yeokja status --check source`가
  통과한다.
- 영어 원고와 한국어 번역이 같은 mdBook 구성으로 빌드되고, 수식(KaTeX)·코드가
  렌더링되며, 영어 제목 앵커가 보존되고, 원문에 없던 깨진 링크와 두 빌드 사이의
  수식 수 차이가 없다.
- Pages 배포 배선, 랜딩 항목, README 출처 표기와 저자가 요구한 고지가 갖춰져 있다.

## 원문 조사 결과 (2026-10-17)

- **원고:** sdiehl 계정에 사이트 저장소가 없다. 사이트는 정적 HTML이며
  markdown-it 산출로 보이는 특징(`<h1 id=… tabindex="-1">`, `breaks: true`로
  문단 안 줄바꿈이 `<br>`로 찍힘)과 Prism 토큰(`<span class="token …">`)이 든
  `<pre><code class="language-…">`, 본문 텍스트로 남은 `$…$`·`$$…$$`·`\(…\)`
  수식(클라이언트 KaTeX 렌더링)을 가진다. RSS(`feed.xml`)에도 본문이 있으나
  페이지 HTML과 같은 산출이라 쓰지 않는다.
- **글 수:** `/posts/` 색인에 111편(2013-04-21 ~ 2026-10-11). 사이트맵에는
  `/posts/<slug>/`가 136개 있어 색인에 없는 글이 25편 더 있다(암호화폐·정치
  주제가 대부분이며 모두 200으로 응답한다). 저자가 목록에서 내린 글이므로 범위에
  넣지 않는다.
- **분량:** 산문 약 19만 단어(코드 펜스 제외), 코드 약 16,000줄, 인라인 수식
  약 1,000개, 디스플레이 수식 약 240개. yeokja `mkdocs` 파서 기준 112파일
  **12,044세그먼트**(글 111편 + 홈 소개). 가장 큰 글은 `training_llms`(1,134
  세그먼트), 그다음 MLIR 연재.
- **라이선스(`/license`):** 글(에세이, 블로그 글, 노트, 서평, 산문)은 CC BY-SA
  4.0, 코드(글 속 스니펫 포함)는 MIT. 출처 표기는 "Stephen Diehl,
  stephendiehl.com"과 라이선스 링크, 변경 사실 표시. 저자는 "어떤 조직·고용주·
  단체와의 연관도 이름 붙이거나 암시하지 말 것"을 하드 룰로 요구하고, 선호
  소개 문구 "Stephen Diehl is a software engineer and writer from London."을
  제시하며, 모든 글이 개인 자격의 글이라는 "Personal views only" 문단을 둔다.
- **변환 품질(pandoc 3.12, 표본 8편 → 전체 111편):** `-f
  html+tex_math_dollars+tex_math_single_backslash -t
  commonmark_x-attributes-smart-fenced_divs-bracketed_spans`로 Prism 토큰은 깨끗한 코드 펜스가 되고, 수식은
  `$…$`/`$$…$$`로 남으며, 단순한 표는 파이프 표, 각주·인용은 문제없다. 손봐야
  했던 것: 문단 안 `<br>\n`(부드러운 줄바꿈), 제목의 `id`·`tabindex`, 2016년
  이전 글의 `<h3>` 제목(h1 없음, 6편), `width`가 있는 `<img>`(원시 HTML로
  남음, 50개), 언어 없는 `<pre><code>`(18개, 들여쓰기 블록이 됨), 칸에 목록이
  든 표 2개(원시 HTML `<table>`로 남고 그 칸의 가격 `$3.99`가 이웃 칸과 짝지어
  수식으로 읽힘), 사이트 상대 링크와 글 사이 링크, 원문 heading id와 mdBook id가
  달라 깨지는 글 안 앵커 링크(4편), `<div>` 묶음이 pandoc 전용 `:::` 문법으로
  나와 mdBook이 글자 그대로 찍는 문제(7편). 원문에서 이미 깨진 수식 3건은 그대로다.
- **파서 비교(echo provider로 전체 왕복):** `markdown` 파서는 `$$…$$`만 있는
  문단 34개를 세그먼트로 제시하고 `$a_1 b_2$` 같은 수식 안의 `_`·`*`를 강조로
  읽을 수 있다. `mkdocs` 파서는 수식을 그림자에서 가려 세그먼트가 수식을 통째로
  담고, 수식만 있는 블록은 제시하지 않으며(698 → 657세그먼트, 표본 8편), 번역
  프롬프트에 수식을 그대로 복사하라는 규칙이 붙는다. 두 파서 모두 공백 외
  왕복 차이는 없다.
- **툴체인:** nixpkgs-unstable에 mdbook 0.5.4, mdbook-katex 0.10.0(mdbook
  0.5.3 대상 빌드 경고만 있고 동작함), pandoc 3.7.0.2, python3 3.14가 있다.
  mdbook-katex는 펜스·코드 스팬 안의 `$`를 건드리지 않았다(빌드 검사로 확인).

## 범위

**포함(CC BY-SA 4.0 글):**

- `/posts/` 색인의 글 111편 전부. 태그는 글 메타로 유지한다.
- 홈(`/`)의 소개 문구(`source/index.md`).
- `/license`의 영어 전문(`source/license.md`, 번역하지 않음)과 그 한국어 요약·
  "Personal views only" 고지 번역(빌드 스크립트가 넣는 고정 문안).

**제외:**

- 사이트맵에만 있는 글 25편. 저자의 편집 결정을 따른다. URL은
  `source/manifest.json`의 `unlisted_posts`에 적어 두어 저자가 다시 올리면 알 수
  있게 한다.
- `/writings/`(별도 책 링크 모음; *Write You a Haskell*은 별도 프로젝트),
  `/hire/`, `/pgp/`, `/tags/*`.
- 이미지: 재배포하지 않고 `https://www.stephendiehl.com/images/…`를 그대로
  불러온다. `alt`는 번역하지 않는다.

**번역 라이선스와 고지:** 번역본은 CC BY-SA 4.0이다. README와 사이트의 모든
페이지(꼬리글), 소개 페이지, 라이선스 페이지에 다음을 적는다.

- © Stephen Diehl, stephendiehl.com, 글 CC BY-SA 4.0·코드 MIT, 변경 사실(한국어
  번역, 사이트 구성), 비공식·저자 미승인.
- 저자의 하드 룰: 어떤 조직과의 연관도 밝히거나 암시하지 않는다. 소개 문구
  "Stephen Diehl is a software engineer and writer from London."
- "Personal views only" 문단의 한국어 번역(라이선스 페이지)과 그 요지(소개
  페이지·꼬리글).
- 번역 모델은 AGENTS.md 규칙을 따른다.
- 정치·시장 등 민감한 주제의 글도 원문 그대로 옮기고 의견을 더하지 않는다.

## 접근 방식

### 채택: HTML → pandoc Markdown 결정적 복원 → `mkdocs` 파서 번역 → mdBook + mdbook-katex

1. `scripts/fetch_site.py fetch`가 `/posts/` 색인, 사이트맵, 홈, 라이선스, 각
   글을 `build/fetch/`에 한 번 내려받는다(지연 1초, 명시적 User-Agent, 3회
   재시도).
2. `scripts/fetch_site.py convert`가 캐시만 읽어 `source/`를 만든다. 변환은
   정규식 전처리 + pandoc + 후처리로 결정적이며, pandoc 버전을 manifest에 적는다.
3. `mkdocs` 파서로 번역한다. 빌드 구성(mdBook)과 파서 이름이 다르지만, 파서는
   세그먼트를 자르는 규칙일 뿐이고 이 원고에 mkdocs 전용 문법이 없으므로
   CommonMark 원고에 그대로 쓸 수 있다.
4. `scripts/prepare_book.py`가 영어 원고와 조립 트리(ko 오버레이)에서 각각
   mdBook 디렉터리를 만들고, 두 책을 빌드해 앵커 보존·인쇄 페이지·꼬리글·링크
   검사·수식 수 비교를 거쳐 한국어 책을 `dist/site`로 낸다.

### 제외: `feed.xml`(RSS)에서 복원

본문 HTML이 페이지와 같고, 메타(태그·`lastmod`)가 빠지며, 항목 수가 색인과
다를 수 있다. 페이지 HTML이 유일한 진실의 원천이다.

### 제외: `markdown` 파서 + 커스텀 프롬프트

`markdown` 파서는 `$` 수식을 모르므로 수식만 있는 문단이 세그먼트가 되고 수식
안의 `_`·`*`가 강조로 읽힐 수 있다. 커스텀 `prompt_template`으로 수식 규칙을
더하면 되지만, 파서가 구조적으로 수식을 가려 주는 `mkdocs`가 더 안전하고 기본
프롬프트(GFM 경고 마커 규칙 포함)를 그대로 쓴다.

### 제외: `inline_tags`

체크리스트 2절대로 `$` 수식이 있는 소스에는 켜지 않는다. 인라인 태그 코덱은
mdBook의 `\\(…\\)`만 수식으로 알아보고 `$…$`는 산문으로 보내며, `mkdocs`는
`INLINE_TAG_PARSERS`에 들어 있지도 않다.

### 제외: 클라이언트 KaTeX(auto-render)

원문 사이트와 같은 방식이지만, 빌드 시점 렌더링(mdbook-katex)이 JS 없이 읽히고
`check_math.py`로 수식 수를 기계적으로 비교할 수 있다.

## 디렉터리와 책임

```text
projects/stephendiehl/
├── source/           복원한 영어 원고(커밋, CC BY-SA 4.0 파생물)
│   ├── posts/<slug>.md   front matter(date, tags, url) + `# 제목` + 본문
│   ├── index.md          홈 소개 문구
│   ├── license.md        라이선스 전문(영어, 번역 제외)
│   └── manifest.json     가져온 시각, pandoc 버전, 글별 메타·해시, unlisted 목록
├── scripts/          fetch_site, prepare_book, preserve_anchors, build_print,
│                     finish_html, check_links, check_math와 단위 테스트
├── RECONSTRUCTION.md 복원 규칙·갱신 방법·제외 목록·알려진 한계
├── state/            번역 상태(커밋)
├── ko/               번역된 원고(재생성, 커밋하지 않음)
├── build/            fetch 캐시, book-en/, book-ko/, 조립 트리(커밋하지 않음)
├── dist/site/        한국어 사이트(커밋하지 않음)
├── glossary.toml, terms.toml, yeokja.toml, README.md, .gitignore
```

## 원문 복원 (`scripts/fetch_site.py`)

규칙의 전문은 `RECONSTRUCTION.md`다. 요지:

- 범위는 `/posts/` 색인이다. `<main>` 안만 쓰고 `post-meta` 블록(게시일·태그)은
  front matter로 옮긴다. 색인 제목은 `<h1>`이 없을 때의 대체 제목이다.
- 전처리: `<br>\n` → 줄바꿈, 제목의 `id`·`tabindex` 제거(원문 id → mdBook id
  대응표는 글 안 앵커 링크 재작성에 쓴다), `<img>`의 `loading`·`decoding` 제거,
  언어 없는 `<pre><code>`에 `language-text`, 링크 재작성(범위 안 글은
  `<slug>.md`, 그 밖의 사이트 상대 경로는 절대 URL, 글 안 앵커는 mdBook id).
- pandoc: `-f html+tex_math_dollars+tex_math_single_backslash -t
  commonmark_x-attributes-smart-fenced_divs-bracketed_spans --wrap=none`.
- 후처리: 끝 공백 제거, 원시 HTML 블록의 `$` → `&#36;`.
- `manifest.json`: `site`, `fetched_at`, `pandoc`, 글마다 `slug`·`title`·`date`·
  `tags`·`url`·`lastmod`·`html_sha256`·`markdown_sha256`, `extra_pages`,
  `unlisted_posts`. 색인에서 사라진 글의 파일은 지운다.
- 갱신: `fetch --refresh` 뒤 `convert`. `html_sha256`이 바뀐 글만 세그먼트가
  바뀌므로 `yeokja translate`가 그 글만 다시 번역한다.
- 테스트(`scripts/test_fetch_site.py`): 색인·사이트맵 파싱, 메타 분리, 링크·앵커
  재작성, 제목 승격, `$` 이스케이프, pandoc 변환(설치된 경우).

## 번역 설정

```toml
[[sources]]
path = "source"
pattern = "posts/*.md"
parser = "mkdocs"
output = "ko/{path}"

[[sources]]
path = "source"
pattern = "index.md"
parser = "mkdocs"
output = "ko/{path}"
```

- `license.md`와 `manifest.json`은 소스 규칙에 넣지 않는다.
- front matter는 `mkdocs` 파서가 비우며 `title:`을 두지 않으므로 제목은 본문
  `# 제목`에서 한 번만 번역된다. SUMMARY의 제목은 번역된 첫 H1에서 읽는다.
- `[provider]`: `claude_code` / `claude-opus-5-5`. 커스텀 프롬프트 없음.
- `[evaluation]`: `auto_evaluate = true`, `style_evaluate = false`,
  `max_retries = 3`. `[translation]`: `concurrency = 8`, `batch_segments = 32`.
- 용어집(`glossary.toml`): Haskell·범주론·타입 이론(함자, 자연 변환, 수반,
  의존 타입, 타입 클래스, 카인드), 컴파일러·MLIR·GHC(다이얼렉트, 로어링, 버퍼화,
  Core, 디슈거링), AI(트랜스포머, 어텐션, 임베딩), 수학·물리(테일러 급수,
  기하 대수, 큐비트), 금융(슬리피지, 변동성, 스프레드). 언어·도구·라이브러리
  이름과 Haskell 타입클래스 이름은 영문. 일반어와 겹치는 용어(kind, universe,
  attention, spread, derivative, integral)는 `except`로 예외를 두고, 문맥마다 번역어가
  다른 동사(lower, pass)는 용어집에 넣지 않는다.
- 첫 번역 뒤 체크리스트 3절대로 `terms build --dry-run` → `terms build` →
  `terms.toml` 검토 → `glossary adopt --prefer HEAD` → 남은 세그먼트 재번역.

## 빌드 (`[build.html]`, `scripts/prepare_book.py`)

- `prepare_book.py <root> <out> --lang en|ko`가 `<root>/posts/*.md`,
  `index.md`, `license.md`, `manifest.json`에서 `book.toml`(제목, 언어,
  `custom.css`, `[preprocessor.katex] after = ["links"]`), `src/SUMMARY.md`
  (소개 → 연도별 part, 최신 글 우선 → 라이선스), `src/posts/*.md`(front
  matter를 제목 아래의 `<p class="post-meta">` 게시일·태그·원문 링크·"한국어
  번역(원문에서 변경됨)" 줄로 바꿈), `src/index.md`(소개 문구 + 사이트 고지),
  `src/license.md`(한국어 책은 요약·고지 번역 + 영어 전문)를 쓴다. 두 언어의
  구성은 같아 제목 수가 일치한다.
- 빌드 명령: 영어·한국어 책 준비 → `mdbook build` 둘 다 → `preserve_anchors.py
  --skip index.html --skip license.html`(두 쪽은 한국어판에 고지 절이 더 있음) →
  `build_print.py` 둘 다 → `finish_html.py`(꼬리글: 출처, 라이선스, 변경 사실,
  소개 문구, 조직 연관 금지, yeokja·모델·학습 비허용 문장) → `check_links.py`
  → `check_math.py` → `site/`.
- `check_math.py`는 쪽마다 `<span class="katex">` 수를 영어·한국어 빌드에서
  비교하고, `<code>` 안에 새어 든 KaTeX가 있으면 실패한다.
- `nix/projects/stephendiehl.nix`: `mdbook`, `mdbook-katex`, `pandoc`(재변환용),
  `python3`.
- 원문에서 물려받은 깨진 링크·수식은 `check_links.py`가 영어 빌드와의 차이만
  보므로 실패 원인이 아니다(영어 빌드의 미해결 앵커 22개는 저자가 손으로 쓴
  `#frechet-derivative` 같은 링크).

## 배포 배선

- `.github/pages-projects.json`: `{"project": "stephendiehl", "target": "html",
  "artifact": "dist-stephendiehl", "artifact_path": "projects/stephendiehl/dist"}`.
- `stage-pages.sh`: `overlay_site "dist-stephendiehl" "site" "stephendiehl"`.
- `rebuild-translations.sh`: `stephendiehl)` 분기에서 `status --check source` 뒤
  `translate source`.
- `LICENSE.md` 표: `stephendiehl` | 글 CC BY-SA 4.0, 코드 MIT.
- `site/index.html`: 필터 `essays`("에세이") 버튼과 "에세이" 주제 섹션에
  "Stephen Diehl 블로그" 항목.
- `pr.yml`: `projects/stephendiehl/scripts`의 단위 테스트 스텝.

## README

- 원문, 저자, 라이선스(글 CC BY-SA 4.0·코드 MIT), 가져온 시각과 manifest.
- 범위와 제외 항목(unlisted 글 25편, writings/hire/pgp/tags, license 처리).
- 영어 원고가 저자 원고가 아니라 HTML에서 복원한 파생 원고라는 점.
- 재현 명령: fetch, convert, translate, status --check, build.
- AGENTS.md 출처 표기 세 가지와 저자의 고지(조직 연관 금지, 소개 문구).

## 위험과 대응

- **원문 갱신:** 사이트는 자주 바뀐다(2026년에도 글이 늘어남). manifest의
  해시로 바뀐 글만 다시 번역한다. 저자가 글을 내리면 convert가 파일을 지우고
  `yeokja orphans`로 state를 정리한다.
- **변환 결정성:** pandoc 버전이 바뀌면 이스케이프가 달라질 수 있다. manifest에
  버전을 적고, 재변환 뒤에는 diff를 보고 세그먼트 변화가 의미 있는지 확인한다.
- **수식 훼손:** `mkdocs` 파서가 수식을 가리고, 프롬프트가 복사를 지시하며,
  `check_math.py`가 쪽별 수식 수를 비교한다. 수식 안의 `\text{…}` 산문은
  번역될 수 있으며 그 경우에도 수식 수는 같다.
- **민감한 주제:** 글 가운데 정치·시장 비평이 있다. 원문 그대로 옮기고, 저자의
  "Personal views only" 고지를 번역해 싣는다.
- **저자 반응:** CC BY-SA로 허용된 작업이다. 출처·변경·조직 연관 금지·소개
  문구를 모든 페이지에 밝히고, 비공식 번역임을 적는다.
- **분량:** 12,044세그먼트. 파일럿 5편(501세그먼트)으로 설정을 확정한 뒤 전체를
  돌린다.
