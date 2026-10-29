# 코딩 커리어 핸드북 (The Coding Career Handbook 한국어 번역)

Shawn "swyx" Wang의 [*The Coding Career Handbook*](https://learninpublic.org/)(저장소
<https://forge.smol.ai/swyx/codingcareer>, 2026-08-18 v1.3.0부터 전문 공개)을 영어에서
한국어로 옮긴 비공식 번역입니다. [yeokja](https://github.com/yeokja/yeokja)와 함께
Anthropic 사의 `claude-opus-5-5` 모델을 활용하여 번역되었으며 학습을 모두 비허용한
상태로 작업하였습니다.

## 범위

- 원문: `upstream/` 서브모듈, 커밋 `a87982e3489393d895e0849ba6d182945ab7ea67`(2026-09-18,
  `main`). 영어판: <https://learninpublic.org/>
- 번역 대상(53개 문서, 8,708 세그먼트): `chapters/*.qmd` 49개(서문·머리말·변경 이력,
  가이드 7, 원칙 12, 전략 10, 전술 10, 마무리·감사의 말, 개념 4, 참고 문헌)와 루트
  `index.qmd`(홈), `quickstart.qmd`, `audiobook.qmd`, `reading-trail.qmd`. 제목·본문·목록·
  인용·콜아웃·그림 캡션과 프런트매터의 `description`·`summary.sentence`·
  `summary.paragraph`(장 카드·검색 미리보기·소셜 메타가 쓰는 요약)를 옮깁니다.
- 원문 그대로 두는 것: 코드, URL, `@sec-…` 교차 참조 키와 `{#sec-…}` 속성, 사람 이름,
  그림 속 글자, 그림의 `fig-alt`(파서가 아직 세그먼트로 내지 않음). 영어 오디오북(MP3·
  M4B·RSS), EPUB·PDF, Amazon 인쇄판, 프리미엄 로그인 링크는 원문 사이트 주소 그대로입니다.
- 번역하지 않음: `publishing/`(소매판 전용), `docs/`, 오디오북 자막.
- **원문 사이트와 다른 점.**
  - 원문의 그림 46개 중 저작권자의 별도 허락이 필요하거나(ARR 13) 출처가 확인되지 않은
    (11) 24개는 싣지 않습니다(`upstream/assets/rights.json`, `THIRD-PARTY-NOTICES.md`).
    본문에서는 번역된 캡션과 원문 사이트의 그림 링크가 든 안내 상자로, 홈의 표지는 원문
    사이트 링크로 대신합니다. 저자 소유(CC BY 4.0) 14개와 개방형 라이선스 8개(CC0, CC BY
    2.5/3.0, CC BY-SA 3.0, MIT, ISC, xkcd CC BY-NC 2.5)는 그대로 싣습니다.
  - 원문 사이트의 JavaScript 기능 — 리더 머리띠, 장 삽화·집필 이력, 오디오북 플레이어와
    따라 읽기, 오프라인(PWA) 저장, 읽기 기록 노트 — 은 사이트가 도메인 루트에 있다고
    가정한 코드(`/assets/…` 절대 경로, `location.pathname` 비교)라 `/learninpublic/` 하위
    경로에서 동작하지 않아 싣지 않습니다. 번역판은 Quarto 기본 책 골격(사이드바 목차, 내장
    검색, 페이지 목차, 밝은/어두운 테마)과 원문의 스타일시트로 냅니다. 오디오북 쪽의 챕터
    목록은 `audiobook/manifest.json`에서 만든 정적 목록(원문 사이트의 MP3 링크, 번역된 장
    링크)입니다.
  - EPUB·PDF·Kindle·인쇄판·소셜 카드는 만들지 않습니다.
- 용어는 `glossary.toml`(책의 표어·원칙·개념 이름: 공개적으로 배우기, 백금률, 램프셰이딩,
  프렌드캐처, 행운의 표면적 …)과 프로젝트 용어 기준 `terms.toml`에 있습니다.

## 라이선스

원고(`index.qmd`, `chapters/`, 저자 소유 그림)는 [CC BY 4.0](https://creativecommons.org/licenses/by/4.0/)
(`upstream/LICENSE.md`, `LICENSE-CONTENT`)이고 스크립트·설정·스타일은 MIT(`LICENSE-CODE`)
입니다. 이 번역은 같은 CC BY 4.0 2차적 저작물로 제공합니다. 출처 표기: "The Coding Career
Handbook by Shawn Wang (swyx), licensed CC BY 4.0." 원문을 한국어로 번역하고 위의 그림과
JavaScript 기능을 뺀 비공식 번역본이며 저자의 공식 번역이 아닙니다. 모든 쪽 하단에
원저작자·라이선스·변경 사실을 표시하고, 원문 `LICENSE-CONTENT`와 제3자 저작물 고지
`THIRD-PARTY-NOTICES.md`를 사이트 루트에 함께 싣습니다. CC BY-SA 그림은 수정하지 않고
그대로 싣고, xkcd 그림은 비상업 조건을 유지합니다. 싣지 않은 그림은 저자의 허락을 받으면
`scripts/prepare_site.py`의 제외 기준을 좁혀 되살립니다.

## 재현

```sh
git submodule update --init projects/learninpublic/upstream
cd projects/learninpublic
../../target/release/yeokja translate upstream               # 누락 세그먼트 번역 및 ko/ 재구성
../../target/release/yeokja status --check upstream
../../target/release/yeokja evaluate --mechanical-only upstream
nix develop path:../../nix#learninpublic -c ../../target/release/yeokja build html   # dist/site
```

번역하려면 인증된 Claude Code CLI가 필요합니다. `state/`는 번역의 진실의 원천이므로
커밋합니다. `ko/`, `build/`, `dist/`는 `state/`와 원문에서 언제든 재생성되므로 커밋하지
않습니다. CI(`.github/scripts/rebuild-translations.sh learninpublic`)는 미번역 세그먼트를
먼저 검사한 뒤 state에서 `ko/`를 재구성하므로 번역 provider를 부르지 않습니다.

## 구성

- 파서: `rmarkdown`(`inline_tags = true`). 강조·링크는 태그로 주고받고, `@sec-…` 뒤에
  조사가 붙은 번역은 파서가 `@{sec-x}를`로 고쳐 써 Quarto 교차 참조가 유지됩니다.
  `front_matter_keys = ["summary.sentence", "summary.paragraph"]`로 중첩 프런트매터 요약을
  번역합니다(설계: `docs/superpowers/specs/2026-10-30-rmarkdown-front-matter-keys-design.md`).
- 빌드 툴체인: `nix/projects/learninpublic.nix` — nixpkgs Quarto 1.10.18(원문이 고정한
  판)과 python3. nixpkgs의 quarto는 pandoc 3.7을 묶는데 Quarto 1.10은 pandoc 3.10의
  옵션을 쓰므로, pandoc 3.10 공식 릴리스 바이너리를 해시로 고정해 `QUARTO_PANDOC`으로
  넘깁니다.
- `overlay/_quarto.yml`, `overlay/_quarto-web.yml`: 한국어 제목·부제·설명·부 제목,
  `lang: ko-KR`, 검색 자리표시자, `site-url`(`https://yeokja.moreal.dev/learninpublic/`),
  하단 출처·번역 고지. 원문의 `include-in-header`·`include-after-body`(JavaScript 셸)는
  두지 않습니다.
- `overlay/styles/korean.scss`: 한글 글꼴·줄바꿈, 그림·표지 대체 안내 상자.
  `overlay/assets/ko/page-classes.html`: 원문 셸이 붙이던 쪽 클래스(`book-home` 등)를
  하위 경로에서도 맞게 붙이는 인라인 스크립트. `patches/magazine-fonts.patch`: 글꼴
  `url(/assets/fonts/…)`을 상대 경로로.
- `scripts/prepare_site.py`(조립 단계): 권리 미확인 그림 참조를 안내 상자로 바꾸고 파일을
  지우며, 표지·오디오북 목록·읽기 기록 안내를 정적으로 만듭니다.
  `scripts/book_map.py`(조립 단계): 원문 `scripts/generate-book-map.mjs`의 Python 이식.
  번역된 부 제목·장 제목·요약으로 홈의 책 지도를 다시 만듭니다(링크는 상대 경로).
- `scripts/check.py`: 같은 Quarto로 빌드한 영어 원문 사이트와 쪽 목록·쪽별 제목 수·
  `sec-` 앵커를 비교하고, 번역 빌드의 로컬 링크·앵커, 풀리지 않은 교차 참조(`?@sec-`),
  제외 그림의 부재, 영어로 남은 문단(허용 목록 밖)을 검사합니다.
- 용어 기준: `yeokja terms build`가 정한 `terms.toml` 493개(사람·회사·제품 이름은 원문 그대로, 널리
  알려진 인명 일부는 한글 표기). 용어집 검사는 번역문에 번역어가 부분 문자열로 들어 있는지만
  보므로, 동사로도 쓰이는 표어는 어간까지만 적습니다(`glossary.toml` 머리말).
- 평가: `yeokja evaluate --mechanical-only upstream`은 0건입니다. 원문이 대문자로 강조한 낱말
  (YOU, NOT, CLEAR)에 모델이 덧붙인 `**`는 재구성 때 이스케이프되어 글자로 보이므로 state에서
  지웠습니다(7세그먼트).
