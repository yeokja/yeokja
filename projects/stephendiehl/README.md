# Stephen Diehl 블로그 (한국어 번역)

Stephen Diehl의 개인 블로그 [stephendiehl.com](https://www.stephendiehl.com)에 실린
글을 한국어로 옮긴 비공식 번역입니다. [yeokja](https://github.com/yeokja/yeokja)와
함께 Anthropic 사의 `claude-opus-5-5` 모델을 활용하여 번역되었으며 학습을 모두
비허용한 상태로 작업하였습니다. 저자가 검토하거나 승인한 번역이 아닙니다.

## 원문과 영어 원고

저자는 사이트의 원고 저장소를 공개하지 않았습니다. 이 프로젝트의 `source/`는
게시된 HTML에서 **기계적으로 복원한 영어 Markdown 원고**이며 저자의 원고가
아닙니다.

- 원문: <https://www.stephendiehl.com>의 `/posts/` 색인에 실린 글. 가져온 시각과
  글마다의 URL·게시일·태그·사이트맵 `lastmod`·원본 HTML의 SHA-256은
  `source/manifest.json`에 있습니다.
- 복원 방법: `scripts/fetch_site.py`가 색인과 각 글의 `<main>` 본문을 한 번만
  내려받아(`build/fetch/`, 요청 간 1초 지연) pandoc으로 Markdown으로 바꿉니다.
  Prism 토큰은 코드 펜스로, `$…$`·`$$…$$`·`\(…\)` 수식은 `$` 수식으로 남고,
  게시일·태그·원문 URL은 front matter로 옮깁니다. 본문을 사람이나 모델이 다시
  타이핑하지 않습니다. 규칙과 알려진 한계는 `RECONSTRUCTION.md`에 있습니다.

## 범위

- 포함: `/posts/` 색인의 글 111편(2013년 4월–2026년 10월), 홈의 소개 문구.
- 제외
  - 사이트맵에만 있고 `/posts/` 색인에서 빠진 글 25편: 저자가 목록에서 내린
    글로 보고 가져오지 않습니다(`source/manifest.json`의 `unlisted_posts`).
  - `/writings/`(별도 책들로의 링크 모음; *Write You a Haskell* 등은 별도
    프로젝트), `/hire/`, `/pgp/`, 태그 페이지.
  - `/license/`는 번역하지 않고 **영어 원문 그대로** 싣되, 번역본 사이트의
    "라이선스와 고지" 페이지 앞부분에 한국어 요약과 저자의 "개인 의견" 고지
    번역을 붙입니다.
- 코드 블록, 수식, 이미지는 원문 그대로입니다. 이미지는 재배포하지 않고 원문
  사이트의 URL을 그대로 불러옵니다. 글이 정치·시장 등 민감한 주제를 다루더라도
  원문을 그대로 옮기며 의견을 더하거나 빼지 않습니다.

## 라이선스

원문 © Stephen Diehl, [stephendiehl.com](https://www.stephendiehl.com). 글(산문)은
[CC BY-SA 4.0](https://creativecommons.org/licenses/by-sa/4.0/), 코드(글 속
스니펫 포함)는 MIT입니다. 복원한 영어 원고와 한국어 번역본은 원문을 한국어로
옮기며 **변경한** 파생물로서 같은 CC BY-SA 4.0으로 제공하고, 코드 블록은
그대로 MIT입니다.

저자의 라이선스 페이지가 요구하는 대로 번역본 사이트의 모든 페이지에 출처
"Stephen Diehl, stephendiehl.com", 라이선스 링크, 변경 사실을 표시합니다. 또한
저자가 밝힌 하드 룰에 따라, 이 글들은 저자가 **개인 자격으로** 쓴 것이며 이
번역본을 인용하거나 언급할 때 저자의 고용주·고객·단체 등 어떤 조직과의 연관도
밝히거나 암시해서는 안 됩니다. 저자가 미리 승인한 소개 문구는
"Stephen Diehl is a software engineer and writer from London."입니다.

## 재현

```sh
cd projects/stephendiehl
ND="nix develop path:../../nix#stephendiehl -c"

# 원문에서 영어 원고 복원 (build/fetch/ 캐시 → source/)
$ND python3 scripts/fetch_site.py fetch
$ND python3 scripts/fetch_site.py convert

# 번역과 한국어 사이트
../../target/release/yeokja translate source
../../target/release/yeokja status --check source
$ND ../../target/release/yeokja build html        # dist/site
```

`state/`와 `source/`는 커밋합니다. `ko/`, `build/`, `dist/`는 `state/`와
`source/`에서 언제든 재생성할 수 있으므로 커밋하지 않습니다.

빌드는 영어 원고와 한국어 번역을 같은 mdBook 구성(연도별 목차, 글 상단의
게시일·태그·원문 링크, mdbook-katex 수식 렌더링)으로 함께 만들고, 영어 제목
앵커를 한국어 페이지에 보존하며(`scripts/preserve_anchors.py`), 원문에 없던
깨진 링크(`scripts/check_links.py`)나 두 빌드 사이의 수식 수 차이
(`scripts/check_math.py`)가 생기면 실패합니다.

## 도구

| 스크립트 | 하는 일 |
|---|---|
| `fetch_site.py` | 색인·사이트맵·글을 캐시에 내려받고 pandoc으로 `source/`를 만듭니다 |
| `prepare_book.py` | `source/`(영어) 또는 조립 트리(한국어)에서 mdBook 디렉터리를 만듭니다 |
| `preserve_anchors.py` | 영어 제목 id를 한국어 페이지의 별칭으로 넣습니다 |
| `build_print.py` | 인쇄용 단일 페이지를 번역된 장에서 다시 만듭니다 |
| `finish_html.py` | 각 페이지에 출처·라이선스·변경 고지를 넣습니다 |
| `check_links.py` | 원문 빌드에 없던 깨진 로컬 링크를 찾습니다 |
| `check_math.py` | 영어·한국어 빌드의 KaTeX 수식 수를 쪽마다 비교합니다 |
