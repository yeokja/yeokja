# The Coding Career Handbook 한국어 번역 설계

## 목표

Shawn "swyx" Wang의 *The Coding Career Handbook*(<https://learninpublic.org/>, CC BY 4.0,
Quarto book, 저장소 <https://forge.smol.ai/swyx/codingcareer>)을 yeokja로 한국어로 옮겨
GitHub Pages 웹 판(`https://yeokja.moreal.dev/learninpublic/`)으로 낸다. 프로젝트 이름과
Pages 경로는 `learninpublic`, 한국어 제목은 "코딩 커리어 핸드북"이다. 파서는 `rmarkdown`
(`docs/superpowers/specs/2026-10-25-rmarkdown-parser-design.md`)이고, 장 요약을 담은 중첩
프런트매터 키는 같은 날짜의 `rmarkdown-front-matter-keys` 설계가 더한 `front_matter_keys`
옵션으로 읽는다.

완료 상태는 다음 조건을 모두 만족해야 한다.

- 원문이 `projects/learninpublic/upstream` 서브모듈로 커밋 `a87982e3`(2026-09-18)에
  고정되어 있다.
- 번역 대상 53개 문서(`chapters/*.qmd` 49개, 루트 `index.qmd`·`quickstart.qmd`·
  `audiobook.qmd`·`reading-trail.qmd`)의 모든 세그먼트가 번역 상태이고 `yeokja status
  --check upstream`이 통과한다. `evaluate --mechanical-only`가 0건이다.
- `quarto render --profile web --to html`로 만든 사이트가 같은 Quarto로 만든 영어 원문
  사이트와 쪽 목록·쪽별 제목 수가 같고, 내부 링크·앵커가 모두 실재하며, 권리가 확인되지
  않은 그림 파일이 산출물에 없다.
- Pages 배포 배선, 랜딩 페이지 항목, README 출처 표기, 루트 `LICENSE.md` 항목이
  갖춰져 있다.

## 조사 결과

- **형식.** Quarto book(`.qmd`, Pandoc Markdown). `_quarto.yml`이 공통 설정, 프로필별
  `_quarto-web.yml`(웹: 장 목록과 부 제목), `_quarto-publication.yml`(EPUB/PDF),
  `_quarto-retailer.yml` 등이 있다. 원문은 Quarto 1.10.18을 고정하며(`.quarto-version`),
  nixpkgs(flake.lock)의 `quarto`도 1.10.18이다. 공식 빌드(`scripts/build.sh`)는 npm 스크립트로
  아이콘·삽화·소셜 카드·책 지도를 생성하고 EPUB·Typst PDF·epubcheck까지 돌리지만, 웹 HTML만
  필요하면 `quarto render --profile web --to html` 하나로 `_book/`이 나온다.
- **규모.** 53개 문서, 약 126k 단어, `rmarkdown` 파서로 약 8.5k 세그먼트. 가장 큰 파일은
  `chapters/bibliography.qmd`(링크 1,501개를 "레이블: <URL>" 목록으로 둔 부록)과
  `chapters/tactics-marketing-yourself.qmd`.
- **프런트매터.** 각 장은 `summary.sentence`(YAML 앵커 `&chapter_summary`)와
  `summary.paragraph`를 두고 `description`·`open-graph`·`twitter-card`가 별칭으로 가리킨다.
  루트 4개 문서에는 `summary`가 없고 `quickstart.qmd`만 최상위 `description`을 둔다.
  파서가 최상위 `title`·`description`은 기본으로 읽으므로 루트 문서에 따로 키를 줄 필요가
  없다(같은 소스 규칙에 두어도 해가 없다).
- **본문 문법.** `@sec-…` 교차 참조 176곳(`(@sec-x)` 괄호 안, 문장 속 모두 있음), `{#sec-x}`
  제목 속성, `{.unnumbered}`, fenced div(`::: {.callout-…}`, `{.content-visible
  when-profile=…}`), 원시 HTML(`<img>`, `<nav>`, `<a>`), 이미지 속성
  `{fig-alt="…"}`, `{{< include … >}}` 단축코드, `$FAMOUS_COMPANY`·`$5M` 같은 산문 속 `$`
  (수식 아님: 파서 설계 2.4의 Pandoc `tex_math_dollars` 규칙으로 처리). 수식은 없다.
- **사이트 구성.** 원문 사이트는 Quarto 기본 책 골격 위에 `include-in-header`·
  `include-after-body`로 JavaScript 셸(`assets/reader/magazine-shell.js`: 머리띠·검색
  단추, `styles/book-reader.js`: 장 삽화·집필 이력, `assets/audiobook/*.js`: 오디오북
  플레이어·따라 읽기·오프라인 PWA, `assets/reader/trail*.js`: 읽기 기록 노트)을 얹는다.
  이 스크립트와 포함 파일, `app.webmanifest`, `assets/generated/book-map.html`은 모두
  루트 절대 경로(`/assets/…`, `/chapters/…`)와 `location.pathname` 비교(`pathname ===
  "/"`, `startsWith("/chapters/")`)에 기대어 **사이트가 도메인 루트에 있다고 가정**한다.
  `styles/magazine.scss`의 셸 레이아웃은 `body.book-shell-enabled`(스크립트가 붙이는
  클래스)로 게이트되어 있어, 스크립트가 없으면 Quarto 기본 사이드바가 그대로 남는다.
- **책 지도.** `index.qmd`가 `{{< include assets/generated/book-map.html >}}`로 넣는
  `book-map.html`은 `scripts/generate-book-map.mjs`가 `_quarto-web.yml`의 부·장 순서, 각
  장의 `# 제목`, `summary.sentence`로 생성해 커밋해 둔 파일이다(부 5개, 번호 장 44개:
  guides 7, principles 12, strategies 10, tactics 11(finale 포함), concepts 4).
- **그림 권리.** `assets/rights.json`(46개)과 `THIRD-PARTY-NOTICES.md`가 그림을 네 부류로
  나눈다: `shawn-controlled`(CC BY 4.0) 14, `open-third-party`(CC0, CC BY 2.5/3.0,
  CC BY-SA 3.0, MIT, ISC, xkcd CC BY-NC 2.5) 8, `third-party-permission-required`(ARR) 13,
  `unresolved` 11. 발행인 확약(`assets/clearance.json`)은 "이 저장소와 그 산출물"의
  재배포만 덮고 CC BY 재허락이 아니다. ARR·미확인 24개 중 22개가 장 본문의 그림이고,
  `cover.jpg`·`published-cover.png`는 `index.qmd`의 원시 `<img>`에만 쓰인다.
- **원문 그림 주소.** 원문 사이트의 장 페이지는 `../assets/images/<dir>/<file>`을 가리키므로
  절대 주소는 `https://learninpublic.org/assets/images/<dir>/<file>`이다(장 페이지 curl로
  확인).
- **라이선스.** 원고(`index.qmd`, `chapters/`, `assets/images/` 중 Shawn 소유분)는 CC BY
  4.0(`LICENSE.md`, `LICENSE-CONTENT`, `_quarto.yml`의 `metadata.rights`), 스크립트·설정·
  스타일은 MIT(`LICENSE-CODE`). Quincy Larson의 서문과 Nat Sharpe의 편집 기여는 CC BY 4.0
  공개가 확약되어 있다. 권장 출처 표기는 "The Coding Career Handbook by Shawn Wang (swyx),
  licensed CC BY 4.0."이다.

## 범위

- 번역: `chapters/*.qmd` 49개 전부와 루트 `index.qmd`, `quickstart.qmd`, `audiobook.qmd`,
  `reading-trail.qmd`. 제목·본문·목록·인용·콜아웃·표·그림 캡션, 프런트매터의
  `description`·`summary.sentence`·`summary.paragraph`.
- 번역하지 않음: `publishing/`(소매판 전용), `docs/`, `audiobook/transcripts`(오디오북
  자막), 코드·URL·`@sec-…` 키·`{#sec-…}` 속성·사람 이름·`fig-alt`(파서가 내놓지 않는다:
  `{…}` 속성은 재구성 때 그대로 남는다)·그림 속 글자. 영어 오디오·EPUB·PDF 링크는 원문
  사이트 주소 그대로 둔다.
- 산출물: 웹 HTML만. EPUB·PDF·Kindle·인쇄판·오디오북 패키징·소셜 카드·PWA는 만들지
  않는다.

## 접근 방식

### 1. `yeokja.toml`

```toml
[[sources]]
path = "upstream"
pattern = "**/*.qmd"
exclude = ["publishing/**"]
parser = "rmarkdown"
output = "ko/{path}"
inline_tags = true
front_matter_keys = ["summary.sentence", "summary.paragraph"]
```

한 규칙이 루트 4개와 `chapters/` 49개를 함께 잡는다(`*.qmd`는 그 디렉터리만 보므로
`**/*.qmd`로 쓴다). `rmarkdown` 파서로 8,708 세그먼트다. `front_matter_keys`는 루트 문서에서 아무것도 찾지 못하고 넘어간다.

`[provider]`는 `claude_code` / `claude-opus-5-5`, `[translation] concurrency = 6`,
`[evaluation] auto_evaluate = true, style_evaluate = false, max_retries = 3`. 프롬프트는
기본 템플릿을 쓴다: `rmarkdown` 소스의 인라인 태그 전송이 강조·링크를 태그로 옮기고,
`@sec-…` 뒤에 조사가 붙은 번역은 파서의 `brace_glued_citations`가 `@{sec-x}를`로 고쳐
쓰므로 프롬프트 규칙이 필요 없다(Pandoc 2.14+의 `@{key}` 문법이며 Quarto는 이를
Cite로 읽어 교차 참조로 푼다; 시험 번역 뒤 빌드에서 `?@sec-`가 남지 않는지 확인한다).
커스텀 템플릿은 GFM 마커·`{inline_tags}`·합쇼체 규칙을 다시 적어야 하므로 두지 않는다.

### 2. 용어

`glossary.toml`은 책이 고유명사처럼 반복해 쓰는 원칙·전략·개념 이름을 고정한다.
"Learn in Public"은 책의 표어이므로 "공개적으로 배우기"로 옮기고, 장 안의 첫 언급에서
영어를 괄호로 남기는 일은 모델의 재량에 맡긴다(용어 검사는 번역어의 존재만 본다).
그 밖에 Learn in Private(비공개로 배우기), Open Source Your Knowledge(지식을 오픈
소스하기), Pick Up What They Put Down(그들이 내려놓은 것을 집어 들기), Platinum Rule
(백금률), Good Enough(충분히 좋음), First Principles Thinking(제1원리 사고),
Specialize in the New(새것에 전문화하기), Spark Joy(설렘을 주는가), Know Your Tools
(도구를 알기), Lampshading(램프셰이딩), Mise en Place Writing(미장플라스 글쓰기),
Friendcatcher(친구 잡이), Luck Surface Area(행운의 표면적), Big L Notation(Big L
표기법), Learning Gears(배움의 기어), Megatrends(메가트렌드), Profit Center·Cost Center
(이익 센터·비용 센터), Career Ladder(커리어 사다리), Strategic Awareness(전략적 인식),
Dark Matter Developers(암흑 물질 개발자), Code Newbie(코드 뉴비), Junior/Senior
Developer(주니어/시니어 개발자), T-shaped(T자형) 등 24개로 시작한다. 일상어와 겹치는
짧은 단어(tactics, strategy, principle, career)는 오탐 재시도를 부르므로 두지 않고
`terms.toml`에 맡긴다.

첫 번역 뒤 `yeokja terms build`(먼저 `--dry-run`)로 반복 제목·레이블을 고정하고,
`terms.toml`을 검토한 뒤 `glossary adopt --prefer HEAD`, 남은 세그먼트만 다시 번역한다.

### 3. 조립(derive)

`[derive] base = "upstream"`에 `ko/`(`require_base = true`)와 `overlay/`를 겹치고 생성
단계 둘과 패치 하나를 돈다. 조립 트리의 파일은 원본을 가리키는 심볼릭 링크이므로
스크립트는 링크를 끊고 새 파일을 쓴다.

**`overlay/_quarto.yml`** — 원문 파일을 통째로 대신한다(병합이 아니라 교체이므로 원문의
항목 중 필요한 것을 다시 적는다).

- `book`: 제목 "코딩 커리어 핸드북", 부제 "코드 뉴비에서 시니어 개발자까지 — 가이드,
  원칙, 전략, 전술", 저자 "Shawn Wang (swyx)", `site-url`
  `https://yeokja.moreal.dev/learninpublic/`, 설명, 검색(사이드바 오버레이), 쪽 탐색.
  `downloads`는 두지 않는다(EPUB·PDF를 만들지 않는다). `page-footer`에 원저작자·CC BY
  4.0·번역 고지(비공식 기계 번역, 변경 사실)와 원문·번역 저장소 링크를 둔다.
- `lang: ko-KR`, `language.search-text-placeholder`, `toc-title` 등 UI 문자열.
- `format.html`: `theme` light `[cosmo, styles/book.scss, styles/magazine.scss,
  styles/korean.scss]`, dark `[darkly, …]`, `respect-user-color-scheme`, `canonical-url`,
  `toc`, `number-sections`, `open-graph: true`(원문의 `chapter-social.lua` 대신 Quarto가
  `description`으로 소셜 메타를 만든다). **`include-in-header`·`include-after-body`는
  두지 않는다** — 원문의 JavaScript 셸·오디오북 플레이어·따라 읽기·오프라인 PWA·읽기
  기록은 루트 배포를 전제로 하여 `/learninpublic/` 하위 경로에서 동작하지 않고,
  `location.pathname` 비교까지 고쳐 써야 해서 재작성 비용이 번역의 가치를 넘는다.
  번역판은 Quarto 기본 책 골격(사이드바 목차, 내장 검색, 페이지 목차, 밝은/어두운
  테마)과 원문의 SCSS(홈 레이아웃·책 지도 스타일은 셸 없이도 적용된다)로 낸다.
- `project.resources`: favicon·apple-touch-icon·`assets/fonts/**`·
  `assets/illustrations-magazine/**`(책 지도 삽화, CC BY 4.0)·`assets/materials/**`·
  `LICENSE-CONTENT`·`THIRD-PARTY-NOTICES.md`. 오디오북 자막·챕터 JSON, 서비스 워커,
  웹 매니페스트, 리더 스크립트는 싣지 않는다.
- `profile.default: web`과 `metadata.rights`는 원문과 같다.

**`overlay/_quarto-web.yml`** — 장 목록은 원문과 같고 부 제목만 한국어("당신의 코딩
커리어", "원칙", "전략", "전술", "개념").

**`overlay/styles/korean.scss`** — 한글 줄바꿈(`word-break: keep-all`,
`overflow-wrap: anywhere`), 시스템 한글 글꼴을 `--book-sans`·`--book-serif` 앞에,
그림 대체 안내(`.figure-omitted`)와 표지 대체(`.book-cover-omitted`) 상자.

**`overlay/assets/ko/page-classes.html`** — 원문 셸이 `location.pathname`으로 붙이던
`body.book-home`·`book-quickstart-page`·`book-chapter-page` 클래스를 경로의 마지막
조각만 보고 붙이는 열 줄짜리 인라인 스크립트. `index.qmd`·`quickstart.qmd`의 홈·퀵스타트
레이아웃이 이 클래스에 기대므로 `format.html.include-after-body`로 모든 쪽에 넣는다.

**`scripts/prepare_site.py TREE`**(생성 단계 1) — 번역된 문서를 하위 경로·정적 사이트에
맞춘다. 원문 `assets/rights.json`을 읽어 `class`가 `third-party-permission-required`
또는 `unresolved`인 파일을 "싣지 않는 그림"으로 정하므로 upstream이 분류를 바꾸면
따라간다.

- 장 본문의 `![캡션](../assets/images/…){…}` 중 싣지 않는 그림을, 들여쓰기를 유지한
  채 한 문단의 스팬으로 바꾼다: `[그림 「{번역된 캡션}」은 저작권자의 별도 허락이
  필요하거나 출처가 확인되지 않아 번역판에 싣지 않습니다. [원문 사이트에서 그림
  보기](https://learninpublic.org/assets/images/…)]{.figure-omitted}`. 싣는 그림은
  속성(`fig-alt`)까지 그대로 둔다.
- `index.qmd`의 표지 `<img>`(`cover.jpg`, `published-cover.png`) 세 곳을 원문 사이트
  링크가 든 `<span class="book-cover-omitted">`로 바꾼다.
- 싣지 않는 그림 파일을 트리의 `assets/images/`에서 지운다(산출물에 들어갈 길을 없앤다).
- `audiobook.qmd`의 `::: {#audiobook-playlist}`(스크립트가 채우던 "Loading…")를
  `audiobook/manifest.json`에서 만든 정적 목록으로 바꾼다: 트랙 번호, 제목(영어 원제),
  길이, 원문 사이트의 MP3 링크, 번역된 장으로 가는 링크. `#audiobook-offline-manager`
  div는 오프라인 저장이 원문 사이트에서만 된다는 한 줄로 바꾼다.
- `reading-trail.qmd`의 앱 루트 div 앞에 읽기 기록 노트가 원문 사이트에서만 동작한다는
  한 줄을 둔다(단추·입력 상자는 번역된 채 남지만 동작하지 않는다).
- `index.qmd`의 "Premium member sign-in" 블록은 둔다(원문 사이트 링크).

**`scripts/book_map.py TREE`**(생성 단계 2) — `upstream/scripts/generate-book-map.mjs`를
표준 라이브러리 Python으로 옮긴 것. 조립 트리의 `_quarto-web.yml`(한국어 부 제목)에서
부별 장 목록을 읽고(finale은 원문처럼 Tactics에 붙인다), 번역된 장의 `# 제목 {#sec-…}`과
`summary.sentence`를 읽어 `assets/generated/book-map.html`을 다시 쓴다. 부 이름·동사·
설명은 스크립트 안의 한국어 표다. 출력 HTML의 요소·클래스·속성 구조는 원문 생성기와
같되 **링크와 삽화 경로는 루트 절대(`/chapters/…`)가 아니라 상대(`chapters/…`)**로
쓴다 — 이 파일은 루트의 `index.qmd`에만 포함되므로 상대 경로로 충분하고 하위 경로
배포에서도 맞는다. 부별 장 수(7/12/10/11/4)와 번호 장 44개는 원문처럼 검사해 어긋나면
실패한다.

**`patches/magazine-fonts.patch`**(패치 단계) — `styles/magazine.scss`의 `@font-face`
두 줄이 `/assets/fonts/…`를 가리키는 것을 프로젝트 기준 상대 경로 `assets/fonts/…`로
바꾼다. Quarto는 SCSS의 `url()`을 프로젝트 루트 기준으로 풀어 그 파일을 `site_libs/`에
복사하고 경로를 고쳐 쓰므로(루트 절대 경로는 손대지 않고 남긴다) 이것으로 충분하다.

**트리 정리.** `prepare_site.py`는 끝으로 렌더링에 쓰지 않는 디렉터리(`docs/`,
`publishing/`, `tests/`, `worker/`, 오디오북 자막·챕터 데이터)를 지우고 남은 심볼릭
링크를 실제 파일로 바꾼다(32MB). Quarto가 링크를 따라가며 프로젝트 밖 경로를 만들지
않도록 하는 안전장치다.

### 4. 빌드와 검사

`[build.html]`은 조립 트리에서 다음을 차례로 한다.

1. `quarto render --profile web --to html` → `_book/`.
2. `LICENSE-CONTENT`, `THIRD-PARTY-NOTICES.md`를 `_book/`에 복사한다.
3. 비교 기준으로 원문을 같은 Quarto로 빌드한다: `upstream`을
   `$YEOKJA_ROOT/build/original-source`로 복사(`.git` 제외)해 그 안에서 같은 명령을
   돌린다. 원문 트리는 커밋된 `book-map.html`과 셸 스크립트를 그대로 쓴다(비교는 HTML
   구조만 본다).
4. `scripts/check.py ORIGINAL_BOOK KO_BOOK`:
   - 두 빌드의 HTML 쪽 목록이 같다.
   - 쪽마다 본문(`main`)의 `h1`~`h6` 수가 같다(`audiobook.html`은 정적 목록으로 바뀌어
     제외 목록에 둔다).
   - 원문 쪽의 `id`(절 앵커 `sec-…`, 명시 id)가 번역 쪽에도 모두 있다. 원문이 자동
     생성한 영어 제목 id는 번역에서 한글 id가 되므로 `sec-`·명시 id만 본다.
   - 번역 빌드의 로컬 링크(`href`, `src`)가 가리키는 파일과 `#앵커`가 실재한다.
     `?@sec-`(풀리지 않은 교차 참조)가 본문에 없다.
   - `assets/rights.json`에서 싣지 않기로 한 그림 파일이 `_book/`에 없다.
   - 본문 문단 중 한글이 전혀 없고 라틴 문자가 40자 이상인 것을 세어 보고한다. 허용
     목록(참고 문헌의 "레이블: URL" 항목, 코드, 인용 출처 줄, 오디오북 목록의 영어
     원제)을 뺀 나머지가 있으면 실패한다.
5. `_book`을 `site`로 옮긴다. `outputs = ["site"]`.

빌드 툴체인은 `nix/projects/learninpublic.nix`(`pkgs.quarto` 1.10.18, `pkgs.python3`)다.
nixpkgs의 quarto는 nixpkgs의 pandoc 3.7.0.2를 `QUARTO_PANDOC`으로 묶는데, Quarto 1.10은
pandoc 3.10의 `syntax-highlighting` 옵션을 쓰므로 그 pandoc으로는 모든 렌더링이 "Unknown
option" 예외로 실패한다(`quarto check`도 3.10.0을 요구한다). pinned nixpkgs에 더 새
pandoc이 없어, pandoc 3.10 공식 릴리스 바이너리(linux amd64/arm64, macOS arm64/x86_64)를
시스템별 해시로 고정해 받는 파생물을 두고 devShell이 `QUARTO_PANDOC`을 그것으로 내보낸다.
원문의 npm·Typst·epubcheck 단계는 쓰지 않는다.

### 5. 작업 순서

1. 골격: 설정, 용어집, `.gitignore`(루트 고정 패턴 `/ko/`, `/build/`, `/dist/`,
   `/__pycache__/` — `overlay/ko/`가 가려지지 않게), 스크립트, 오버레이, nix.
2. 시험 번역(`chapters/principles-learn-in-public.qmd`): 재구성 diff, 재시도율, 문체,
   `{fig-alt}`·`@sec-`·강조·링크 보존을 본다. `chapters/bibliography.qmd`로 URL 목록
   보존을 본다.
3. 전체 번역(백그라운드, 로그는 스크래치 디렉터리) → `evaluate --mechanical-only` 0건 →
   `terms build --dry-run` → `terms build` → `terms.toml` 검토 → `glossary adopt --prefer
   HEAD` → 남은 세그먼트 번역 → `status --check`.
4. `nix develop path:../../nix#learninpublic -c ../../target/release/yeokja build html`,
   `check.py`, 브라우저 확인.
5. README, 배포 배선(호출 측 공유 파일: `pages-projects.json`, `stage-pages.sh`,
   `rebuild-translations.sh`, 루트 `LICENSE.md`, `site/index.html`).

## 라이선스

원고는 CC BY 4.0이다. 번역은 같은 CC BY 4.0 2차적 저작물로 내고, 모든 쪽 하단과 README에
"The Coding Career Handbook by Shawn Wang (swyx), licensed CC BY 4.0"과 번역·변경
사실(한국어 번역, 일부 그림 제외, JavaScript 기능 제외), 비공식 번역임을 표시한다.
원문 `LICENSE-CONTENT`와 `THIRD-PARTY-NOTICES.md`를 사이트 루트에 함께 싣는다.

- 개방형 라이선스 그림(CC0, CC BY, CC BY-SA, MIT, ISC, xkcd CC BY-NC 2.5)은 수정 없이
  싣고 `THIRD-PARTY-NOTICES.md`가 출처 표기를 담당한다. CC BY-SA 그림은 수정하지
  않으므로 ShareAlike 의무가 번역 본문에 미치지 않는다. xkcd는 비상업 조건을 유지한다.
- ARR·미확인 24개는 싣지 않는다. 발행인 확약은 원문 저장소와 그 산출물의 재배포만
  덮고 제3자 번역판까지 미친다고 볼 근거가 없다. 저자(swyx)에게 번역판 수록 허락을
  물어 받으면 다시 싣는다(사람이 할 일).
- 책 지도 삽화(`assets/illustrations-magazine/`, 매니페스트에 CC BY 4.0)는 싣는다.
- 번역 사이트는 원문의 EPUB·PDF·오디오·프리미엄 로그인·Amazon 링크를 원문 주소 그대로
  남긴다. 모두 영어판이다.

## 배포

`.github/pages-projects.json`에 `project = "learninpublic"`, `target = "html"`,
`artifact = "dist-learninpublic"`, `artifact_path = "projects/learninpublic/dist"`를
더하고 `stage-pages.sh`에 `overlay_site "dist-learninpublic" "site" "learninpublic"`을
더한다. `rebuild-translations.sh`의 분기는 `status --check upstream` 뒤 `translate
upstream`(소스 규칙이 하나이므로 `upstream` 전체를 한 번에 확인하면 upstream이 새로
더한 문서가 state 없이 빠지는 경우도 배포를 막는다). 랜딩 페이지에는 "에세이" 주제
(`essays`)에 카드를 둔다 — 커리어 에세이 모음이지 프로젝트 관리 교재가 아니다.

## 되짚을 수 있는 결정

- **JavaScript 셸 제외.** 하위 경로 배포가 아니라 전용 도메인 루트에 두게 되면 원문의
  포함 파일·스크립트를 그대로 쓸 수 있다. 그때 `include-in-header` 등을 되살리고
  `book_map.py`의 상대 경로만 남기면 된다.
- **그림 제외.** 저자 허락을 받으면 `prepare_site.py`의 제외 기준(`rights.json` 분류)을
  허락 목록으로 좁힌다.
- **오디오북·읽기 기록.** 정적 목록과 안내문으로 대신한다. 플레이어를 되살리려면 셸
  결정과 함께 본다.
- **"Learn in Public"의 영어 병기.** 모델 재량에 맡겼다. 일관성이 필요해지면 용어집
  번역어를 "공개적으로 배우기(Learn in Public)"로 바꾸고 `glossary adopt`로 다시
  받아들인다.
- **`fig-alt`.** 파서가 내놓지 않아 영어로 남는다. 파서가 Quarto 이미지 속성의 `fig-alt`를
  세그먼트로 내게 되면 재번역 없이 `reconcile`로 옮긴다.
