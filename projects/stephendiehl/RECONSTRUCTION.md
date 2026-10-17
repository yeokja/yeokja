# 원고 복원 규칙

이 디렉터리의 `source/`는 Stephen Diehl의 블로그 <https://www.stephendiehl.com>에
게시된 HTML에서 복원한 영어 Markdown 원고입니다. 저자는 사이트의 원고 저장소를
공개하지 않았으므로(사이트 소스는 MIT라고 밝혔지만 저장소가 없음) 저자의 원고가
아니라 **게시된 페이지를 기계적으로 되돌린 파생 원고**입니다. 복원은
`scripts/fetch_site.py`가 결정적으로 수행하며, 사람이나 모델이 본문을 다시
타이핑하지 않습니다.

## 입력

- `/posts/` 색인: 번역 범위를 정하는 글 목록(제목·게시일·slug, 최신순).
- `/sitemap.xml`: URL별 `lastmod`. 색인에 없는 `/posts/<slug>/`를 찾는 데도 쓴다.
- 각 글 `/posts/<slug>/`: `<main>` 안의 본문. 사이트는 markdown-it 출력으로 보이는
  정적 HTML(`<h1 id=… tabindex="-1">`, Prism 토큰 `<span class="token …">`이 든
  `<pre><code class="language-…">`, `$…$`·`$$…$$`·`\(…\)` 수식이 본문 텍스트로
  남아 클라이언트에서 KaTeX로 렌더링)이다.
- `/`(홈 소개 문구)와 `/license/`(라이선스 전문).

내려받은 HTML은 `build/fetch/`(커밋하지 않음)에 한 번만 캐시한다. 요청 사이에
1초 지연을 두고, User-Agent에 프로젝트와 연락처를 밝히며, 실패하면 3회까지
재시도한다.

```sh
cd projects/stephendiehl
nix develop path:../../nix#stephendiehl -c python3 scripts/fetch_site.py fetch     # build/fetch/
nix develop path:../../nix#stephendiehl -c python3 scripts/fetch_site.py convert   # source/
```

`--refresh`를 주면 캐시를 무시하고 다시 내려받는다. `convert`는 캐시만 읽으므로
오프라인에서도 재현된다. 변환에 쓴 pandoc 버전은 `source/manifest.json`의
`pandoc`에 기록한다(버전이 다르면 이스케이프나 줄바꿈이 조금 달라질 수 있다).

## 변환 규칙 (`scripts/fetch_site.py`)

1. **범위는 `/posts/` 색인이다.** 사이트맵에만 있고 색인에서 빠진
   `/posts/<slug>/`는 저자가 목록에서 내린 글로 보고 가져오지 않는다. 그 URL
   목록은 `manifest.json`의 `unlisted_posts`에 적어 두어 저자가 다시 올리면 알 수
   있게 한다. `/writings/`(별도 책 링크 모음), `/hire/`, `/pgp/`, `/tags/*`는
   번역 대상이 아니다.
2. **`<main>` 안만 쓴다.** 사이트 머리글·꼬리글·내비게이션은 버린다. 글 상단의
   `post-meta` 블록(게시일, 태그, 읽기 모드 버튼)은 본문에서 떼어 내 게시일과
   태그만 front matter로 옮긴다.
3. **`<br>` + 줄바꿈은 부드러운 줄바꿈이다.** 저자의 markdown-it은 `breaks: true`로
   원고의 줄바꿈을 `<br>`로 찍는다. 문단 안의 `<br>\n`은 줄바꿈(공백)으로 되돌려
   한 문장이 한 세그먼트가 되게 한다. 줄 끝이 아닌 `<br>`(표 칸 안)은 그대로 둔다.
4. **제목 heading의 `id`·`tabindex`를 버린다.** mdBook이 제목에서 다시 id를 만들고,
   빌드가 영어 제목 id를 별칭으로 되살린다(`scripts/preserve_anchors.py`). 2016년
   이전 글은 제목이 `<h1>`이 아니라 `<h3>`로 시작하므로, `<h1>`이 없는 글에서는
   색인 제목과 같은 첫 heading을 `<h1>`으로 올리고, 그런 heading도 없으면 색인
   제목으로 `<h1>`을 만든다. 본문 소제목의 단계(`<h5>` 등)는 원문 그대로 둔다.
5. **링크.** `/posts/<slug>/`가 범위 안이면 `<slug>.md`(같은 디렉터리의 상대
   링크)로, 그 밖의 사이트 상대 경로(`/images/…`, `/writings/`, 범위 밖 글)는
   `https://www.stephendiehl.com/…` 절대 URL로 바꾼다. 조각(`#…`)만 있는 링크와
   외부 링크는 그대로 둔다. 이미지는 재배포하지 않고 원문 URL을 가리킨다.
6. **이미지의 `loading`·`decoding` 속성은 버린다.** `width`가 있는 `<img>`는
   pandoc이 원시 HTML로 남기며 mdBook이 그대로 렌더링한다. `alt`는 번역하지
   않는다.
7. **언어가 없는 코드 블록은 `text`로 표시한다.** pandoc이 들여쓰기 블록 대신
   펜스를 쓰게 하여, 파서의 수식 스캐너와 mdbook-katex가 코드 안의 `$`를 수식으로
   읽지 않게 한다.
8. **pandoc 변환.** `-f html+tex_math_dollars+tex_math_single_backslash -t
   commonmark_x-attributes-smart-fenced_divs-bracketed_spans --wrap=none`. 본문 텍스트의 `$…$`, `$$…$$`,
   `\(…\)`를 수식으로 읽어 `$…$`/`$$…$$`로 쓰고, Prism 토큰 `<span>`은 코드
   펜스로 평탄화되며, 제목 속성(`{#id}`)과 스마트 따옴표 이스케이프(`\'`)는 끄고,
   `<div>`·`<span>`은 pandoc 전용 `:::`·`[]{}` 문법 대신 원시 HTML로 남기며, 문단은 한 줄로 쓴다. 단순한 표는 파이프 표가 되고, 칸에 목록·줄바꿈이 든
   표는 원시 HTML `<table>`로 남는다(아래 "알려진 한계").
9. **원시 HTML 블록의 `$`는 `&#36;`로 쓴다.** 원시 HTML 표의 칸에 든 가격
   (`$3.99`)이 다음 칸의 `$`와 짝지어 수식으로 읽히지 않게 한다. 원문 사이트는
   텍스트 노드 단위로 수식을 렌더링하므로 그런 짝짓기가 없다.
10. **출력.** `source/posts/<slug>.md`는 front matter(`date`, `tags`, `url`) 뒤에
    `# 제목`으로 시작하는 본문이다. `source/index.md`는 홈 소개 문구에
    `# Stephen Diehl` 제목을 붙인 것이고, `source/license.md`는 라이선스 전문이다.
    `source/manifest.json`에는 가져온 시각, pandoc 버전, 글마다 slug·제목·게시일·
    태그·URL·`lastmod`·원본 HTML과 Markdown의 SHA-256, 그리고 제외한 unlisted
    글 목록을 적는다. 색인에서 사라진 글의 `source/posts/*.md`는 지운다.

## 검증

- `python3 -m unittest discover -s scripts -p 'test_*.py'`: 색인·사이트맵 파싱,
  메타 분리, 링크 재작성, 제목 승격, `$` 이스케이프, pandoc 변환(설치된 경우)
  단위 테스트.
- 변환 결과의 자기 점검: 모든 글에 `# 제목`이 하나씩 있고, 코드 펜스 밖에 사이트
  상대 링크(`](/`, `href="/`)가 없으며, `source/posts/*.md`의 수가 색인의 글 수와
  같다.
- 원문 사이트와의 대조는 수식·코드 렌더링을 눈으로 확인하는 것으로 갈음한다.
  빌드가 영어 원고와 한국어 번역을 같은 방식으로 조판하고
  `scripts/check_math.py`가 두 빌드의 KaTeX 수식 수를 쪽마다 비교하므로, 번역이
  수식을 잃거나 더하면 빌드가 실패한다.

## 알려진 한계(원문에서 물려받은 것 포함)

- 칸에 목록이나 줄바꿈이 든 표 2개(`training_llms.md`, `typechecker_zoo.md`)는
  원시 HTML `<table>`로 남는다. 파서는 원시 HTML 블록 안의 텍스트도 번역
  세그먼트로 제시하므로 번역은 된다.
- 원문에서 이미 깨진 수식은 그대로 깨져 있다. 예: `tensor_canonicalization_rust`의
  `\( R^a{}_{bac} \)`는 원문 HTML에서 `_…_`가 강조로 렌더링되어 있고,
  `python_cuda`의 `\\(r))`는 원문의 오타이며, `mlir_memory`의
  `\text{base_address}`는 KaTeX가 원문 사이트에서도 렌더링하지 못한다.
- `$` 수식은 원문 사이트가 KaTeX auto-render로 읽는 범위와 같다는 가정 아래
  옮긴다. 산문에 `$`가 홀로 쓰인 금액 표기(`$25/hour`)는 같은 문단에 짝이 되는
  `$`가 없어 수식으로 읽히지 않는다.
- 이미지 `alt`와 원시 HTML `<img>`의 속성은 번역하지 않는다.
- 2016년 이전 글의 소제목은 원문대로 `<h5>`(`#####`)여서 mdBook 목차에는 나오지
  않는다.
