# FORGE Lab 블로그 한국어 번역 설계

이슈: yeokja/yeokja#20

## 목표

[`microsoft/forge-lab`](https://github.com/microsoft/forge-lab)의 블로그
(<https://forge-blog.github.io>, Microsoft Security FORGE Lab의 연구 노트, 글은 CC BY 4.0)를
고정된 원문으로 삼아 yeokja로 영어 → 한국어로 번역하고, 원문과 같은 Hugo 템플릿을 한국어로
옮긴 비공식 번역 사이트를 GitHub Pages `forge-lab/`(<https://yeokja.moreal.dev/forge-lab/>)에
배포한다.

완료 상태는 다음 조건을 모두 만족해야 한다.

- 원문 저장소가 `projects/forge-lab/upstream` 서브모듈로
  `48f83fcbe88c164e7a075aec301dc22b18e907e2`에 고정되어 있다.
- 번역 범위(아래)의 모든 세그먼트가 번역 상태이고
  `yeokja status --check upstream/blog/content/posts`가 통과한다.
- `yeokja evaluate --mechanical-only upstream/blog/content/posts`에 남은 항목이 모두 확인된
  오탐이다.
- 한국어 Hugo 빌드가 오류 없이 끝나고, 같은 Hugo로 빌드한 영어 원문과 비교해 쪽 목록이 같고,
  원문에서 멀쩡하던 로컬 링크·그림·앵커가 깨지지 않으며, 쪽마다 본문의 제목·그림·코드 블록·
  표·목록 항목·인용·알림 상자·목차 항목 개수가 같다(`scripts/check_site.py`).
- 산출물에 방문 통계, Microsoft 로고가 없고, 모든 쪽에 비공식 번역 고지가 있다.
- 원문 템플릿이 바뀌면 빌드가 실패한다(`scripts/check_upstream.py`).
- Pages 배포 배선, 랜딩 페이지 항목, `LICENSE.md` 표, README 출처 표기가 갖춰져 있다.

## 조사 결과

- **저장소.** 루트에 CVE 목록(`cve/cve.json`), 발표 자료(`slides/*.pdf`), `blog/`(Hugo
  사이트)가 있다. 블로그 글은 `blog/content/posts/<slug>/index.md` 글 묶음 2개
  (`linux-kernel-3-month-lookback`, `rust-std-memory-safety`)이고, 묶음 안에 SVG 도표,
  `cves.txt`, 머리 그림 `cover*.webp`가 있다. 그 밖의 쪽은 개인정보 처리 방침
  (`content/privacy/index.md`) 하나다. 원문은 활발히 갱신된다.
- **라이선스.** `LICENSE`는 CC BY 4.0("Copyright (c) Microsoft Corporation"), `LICENSE-CODE`는
  MIT다. README의 License 절은 코드·스크립트에 MIT를, "Documentation and other written
  content"에 CC BY 4.0을 적용하고 CVE 식별자는 어느 쪽도 아니라고 밝힌다. 그림(장식 회화)은
  어느 쪽에도 언급되지 않는다.
- **상표.** README의 Trademarks 절은 수정본에서 Microsoft 상표·로고를 쓸 때 혼동을 일으키거나
  Microsoft의 후원을 암시하지 않아야 한다고 요구한다. 템플릿은 머리글 워드마크와 첫 화면에
  Microsoft 로고(`partials/microsoft-logo.html`)를 싣고, `js/brand-wordmarks.js`가 본문과
  글 목록 설명의 "Microsoft" 낱말마다 로고를 붙인다.
- **방문 통계.** `hugo.toml`의 `params.analytics`가 켜져 있으면 운영 빌드의 첫 화면·글 목록·
  글 쪽에 Azure Application Insights로 쪽 보기를 보내는 스크립트(`partials/analytics.html`,
  `js/analytics.js`)가 실린다. 개인정보 처리 방침 쪽은 이 수집을 설명한다.
- **문법.** Goldmark. front matter는 TOML(`+++`)이고 `title`, `description`이 화면에
  나온다(나머지는 `date`, `slug`, `author`, `author_url`, `toc`, `artwork`, `aliases`).
  본문에 `{{< note >}}`/`{{< /note >}}` shortcode 줄이 한 쌍 있다(Rust 글). 수식·각주는 없고,
  표가 하나(커널 글), 참조식 링크 정의가 하나, 본문 HTML은 없다.
- **템플릿 문구.** 영어 문구는 모두 사이트 쪽 `layouts/`에 있다(테마 없음, i18n 없음):
  `_default/baseof.html`(건너뛰기 링크, 메뉴, 하단), `_default/single.html`("All writings",
  "By", 날짜 형식 `January 2, 2006`, "min read", "Contents"), `_default/list.html`,
  `index.html`(첫 화면 문구와 그림 alt), `partials/post-list.html`(날짜 형식 `Jan 2, 2006`),
  `partials/social-meta.html`과 머리 그림 partial(그림 alt).
- **Hugo 버전.** 원문은 Hugo extended 0.158.0 이상을 요구한다. nixpkgs의 Hugo 0.165.0
  extended로 원문과 번역 템플릿이 모두 빌드된다.
- **분량.** 글 2개, 세그먼트 약 308개.

## 범위

- 번역: `blog/content/posts/**/*.md`(글 2개)의 front matter `title`·`description`, 본문, 목록,
  표 칸, 알림 상자, 그림 설명(alt). 템플릿 문구는 overlay에서 손으로 옮긴다.
- 번역하지 않음: 코드 블록, 인라인 코드, CVE 식별자, 이슈·PR 번호, 커밋 해시, 사람 이름,
  회사·연구실·제품·모델 이름(Microsoft, FORGE Lab, codename MDASH, Hyenas, Sonnet 4.6,
  GPT-5.5, Opus 4.6 등), 인용한 이슈·패치 제목, URL.
- 범위 밖:
  - **개인정보 처리 방침 쪽.** 원문 사이트의 Azure Application Insights 수집을 설명하는데
    번역 사이트는 방문 통계를 싣지 않으므로 그 설명이 이 사이트에 대해 사실이 아니다. 쪽과
    하단의 링크를 함께 뺀다.
  - **SVG 도표 속 글자.** 그림 파일의 글자는 영어로 둔다. 도표마다 alt가 그림 내용을 문장으로
    모두 풀어 쓰므로(원문 작성 지침) alt 번역이 한국어 독자에게 그림 내용을 전한다.
  - **발표 자료 PDF(`slides/`)와 `cve/cve.json`.** 블로그가 아니고, CVE 목록은 번역할
    글이 없는 데이터다. 머리 메뉴의 CVE 링크는 원문 저장소의 파일을 그대로 가리킨다.

## 접근 방식

### 1. `yeokja.toml`

- 소스 하나: `upstream/blog/content/posts`, `**/*.md`, `parser = "hugo"`, 출력
  `ko/content/posts/{path}`, `front_matter_keys = ["title", "description"]`. `hugo` 파서는
  TOML front matter에서 지정한 키만 번역 세그먼트로 내고(나머지 키는 원문 그대로),
  shortcode만 있는 줄(`{{< note >}}`)을 원문 그대로 두며, 짝 shortcode 사이의 글은 번역한다.
  세그먼트는 301개다.
- 그림만 있는 문단(본문 그림 4개, 모두 긴 설명 alt)은 alt가 번역되어야 한다. 인라인 태그
  모드에서 그림 alt를 번역 글자로 내는 코어 지원을 따른다.
- **`inline_tags = true`.** 수식이 없는 Goldmark 본문이라 켤 수 있다. 강조·링크·코드 스팬은
  태그로 주고받아 조사 앞 강조·링크 손실 같은 결함이 생기지 않는다.
- **커스텀 `prompt_template`.** 기본 프롬프트에 없는 이 블로그의 규칙이 필요하다: CVE
  식별자·이슈 번호·식별자와 모델·제품 이름을 그대로 두고, 인용한 이슈·패치·기사 제목을
  고유 이름으로 옮기지 않는다. 커스텀 템플릿이므로 `{inline_tags}` 자리표시자와 GFM 경고
  마커 문장을 직접 둔다(AGENTS.md).
- provider `claude_code`/`claude-opus-5-5`, `concurrency = 6`, `glossary_plurals = true`,
  `auto_evaluate`, `max_retries = 3`.

### 2. 용어집

메모리 안전성(memory safety, memory-safety), 건전성(soundness), 미정의 동작(undefined
behavior), 불변식(invariant), 패닉(panic), 언와인딩(unwinding), 소멸자(destructor),
할당자(allocator), 시그널 핸들러(signal handler), 가드 페이지(guard page), 참조
카운트(refcount, reference count), 권한 상승(privilege escalation), 익스플로잇(exploit),
취약점(vulnerability), 업스트림(upstream), 메인라인(mainline), 메인테이너(maintainer),
패치(patch), 서브시스템(subsystem). use-after-free와 PoC는 원어로 둔다. 번역어는 국내
보안·시스템 프로그래밍 문서의 관용을 따른다. 나머지 반복 용어와 제목은 첫 번역 뒤
`yeokja terms build`로 `terms.toml`에 정한다. 연구실 이름 `FORGE`·`FORGE Lab`·`Microsoft`·
`codename MDASH`는 원어 그대로 두는 항목이고, 소문자 일반 명사 lab("Beyond the lab")은
연구실 이름이 아니므로 `Lab` 항목을 두지 않으며, 동사 forge("safe code can't forge")는
`FORGE`의 `except`로 뺀다.

### 3. 빌드

조립 트리는 `upstream/blog` 위에 `ko/`(번역된 `content/posts`, `require_base`)와
`overlay/`를 겹친다.

- **`overlay/hugo.toml`** 이 원문 설정을 대신한다. `baseURL =
  "https://yeokja.moreal.dev/forge-lab/"`, `locale = "ko-kr"`, 한국어 읽기 시간과 요약
  길이를 글자 수로 세는 `hasCJKLanguage = true`, `params.analytics.enabled = false`(측정 키와
  수집 주소도 지운다), 한국어 사이트 설명, 원문 사이트·저장소·라이선스 주소. 사이트 제목은
  "FORGE Lab 비공식 한국어 번역"으로, 쪽 제목·공유 미리보기에도 비공식 번역임이 드러난다.
- **템플릿은 원문 사본을 손으로 옮긴다.** 문구가 적고(약 20개) i18n이 없으므로 i18n을
  새로 들이지 않고 `overlay/layouts/`에 원문 파일과 같은 경로의 사본을 둔다:
  `_default/{baseof,single,list}.html`, `index.html`, `partials/{post-list,social-meta}.html`.
  날짜는 `2006년 1월 2일` 형식, 읽기 시간은 "읽는 데 N분", 글쓴이는 "글쓴이 X". 글 목록
  제목은 `overlay/content/posts/_index.md`(`title = "글"`)가 정한다.
- **상표.** Microsoft 로고 partial과 본문에 로고를 붙이는 스크립트를 싣지 않는다. 이름
  "FORGE Lab"과 머리글 워드마크(글자)는 원문 연구실을 가리키는 이름이므로 둔다. 모든 쪽의
  머리 메뉴 아래에 "Microsoft Security FORGE Lab 블로그의 원문을 한국어로 옮긴 비공식
  번역이며 Microsoft가 만들거나 후원하거나 검토한 사이트가 아님"을 밝히는 고지를 두고, 그
  쪽의 원문(`https://forge-blog.github.io/` + 같은 경로)으로 잇는다. 하단은 원저작 표시
  (© Microsoft Corporation), 개작 사실, 원문·CC BY 4.0·yeokja·RSS 링크다.
- **그림.** 첫 화면과 글 머리의 장식 회화(`assets/images/**`, 글 묶음의 `cover*.webp`),
  공유 미리보기 그림(`static/images/forge.png`), 파비콘은 싣지 않는다. 원문의 이용 허락이
  "Documentation and other written content"(CC BY 4.0)와 코드(MIT)만 다루고 그림은 어느
  쪽에도 들지 않아 개작물에 실을 근거가 분명하지 않고, 장식이라 빼도 글의 내용이 줄지
  않는다. 그림에 딸린 움직임 스크립트·스타일, `artwork` front matter 처리, 목록 썸네일도 함께
  뺀다. 본문의 SVG 도표는 글의 일부인 문서로 보아 싣는다. 첫 화면은 그림 단이 없어지므로
  소개 글을 한 단으로 둔다.
- **글꼴.** 원문 웹 글꼴(Literata, Inter, JetBrains Mono, OFL)은 라틴 부분 집합뿐이라 한글이
  시스템 기본 글꼴로 떨어진다. `overlay/assets/css/translation.css`가 `main.css` 다음에
  실려 글꼴 변수(`--serif`, `--sans`, `--mono`)에 한글 고딕 글꼴(Apple SD Gothic Neo,
  Noto Sans KR, Malgun Gothic; 고정폭은 D2Coding 먼저)을 덧붙이고, 제목·본문에
  `word-break: keep-all`을 준다. 세리프 변수에도 고딕 한글을 두는 것은 화면에서 명조 계열
  한글의 가독성이 떨어지고 시스템마다 품질 차이가 크기 때문이다. `main.css`는 고치지 않는다.
- **`scripts/prepare_site.py korean`(derive generate 단계).** 개인정보 처리 방침 쪽, 로고,
  방문 통계, 장식 그림과 그 스크립트·스타일, 쓰이지 않는 partial, `tests/`를 지우고, 번역
  범위의 글이 모두 `ko/`의 번역본인지 확인한 뒤(없으면 영어 글이 조용히 섞이므로 실패), 남은
  심볼릭 링크를 실제 파일로 바꾼다(Hugo는 링크인 파일을 읽지 않는다). 지울 파일이 원문에서
  사라지면 실패해 스크립트를 고치게 한다.
- **원문 드리프트 검사(`scripts/check_upstream.py`).** overlay 사본의 원본(`hugo.toml`, 템플릿
  6개, `main.css`)의 SHA-256을 적어 두고, 원문 서브모듈을 올려 하나라도 바뀌면 실패한다.
  `upstream/blog/layouts`의 파일 목록도 고정해, 새 템플릿이 생기면(영어 문구가 섞이거나
  지운 partial을 부를 수 있음) 실패한다. 실패하면 바뀐 내용을 overlay 사본에 옮기고 해시를
  갱신한다.
- **원문 비교 빌드.** `build.html`은 `check_upstream.py` 다음에 `upstream/blog` 사본에
  `prepare_site.py original`(개인정보 처리 방침 쪽만 지움)을 하고 `scripts/original.toml`
  (같은 baseURL, 방문 통계 끔)을 겹쳐 영어 사이트를 빌드한 뒤, 번역 사이트를 빌드하고
  `scripts/check_site.py`로 비교한다. 비교는 본문(`<div class="prose">`) 안으로 한정한다.
  글 머리 그림이 번역 사이트에 없으므로 `<article>` 전체를 세면 그림 수가 어긋난다.
  원문 빌드의 하단 Privacy 링크는 지운 쪽을 가리키므로 원문에서만 깨진 링크로 보고된다.
  마지막에 원문 `LICENSE`, `LICENSE-CODE`를 사이트 뿌리에 복사한다.

### 4. 검사(`scripts/check_site.py`)

실패 조건: 쪽 목록 차이, 원문에서 멀쩡하던 로컬 링크·그림·스타일·앵커가 번역에서 깨짐(한국어
제목의 Hugo 자동 ID 때문에 제목 앵커 링크가 여기서 걸린다), 쪽별 본문 구조 개수 차이, 본문
그림 alt에 한글이 없음, 본문에 한글이 없음, 방문 통계·Microsoft 로고·로고 스크립트 잔재,
별칭(넘김) 쪽을 뺀 쪽의 `<html lang="ko">`나 비공식 번역 고지 누락.
