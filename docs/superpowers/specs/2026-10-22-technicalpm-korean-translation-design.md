# Technical Project Management in Living and Geometric Order 한국어 번역 설계

## 목표

University of Wisconsin–Madison이 Pressbooks로 펴낸 개방형 교재 *Technical Project
Management in Living and Geometric Order*(3판, Russell·Pferdehirt·Nelson, CC BY
4.0, `https://wisc.pb.unizin.org/technicalpm/`)의 EPUB 내보내기를 고정된 원문으로
사용한다. yeokja로 독자에게 보이는 본문 전체를 한국어로 번역해 한국어 EPUB과
GitHub Pages 웹 읽기 판을 낸다. 파서는
`docs/superpowers/specs/2026-10-22-xhtml-parser-design.md`의 `xhtml` 파서다.

완료 상태는 다음 조건을 모두 만족해야 한다.

- 원문 EPUB의 압축을 푼 트리가 `projects/technicalpm/upstream/`에 커밋되어 있다.
  출처 URL, 내려받은 날짜, 원본 EPUB의 SHA-256이 `upstream.lock`에 기록되어 있다.
- 번역 대상 콘텐츠 문서의 모든 세그먼트가 번역 상태이고 `yeokja status --check`가
  통과한다.
- 번역한 30개 콘텐츠 문서가 원문과 골격 동치다(XHTML 파서 설계 3.4).
- 한국어 EPUB이 `epubcheck`에서 원문 EPUB에 없던 오류·경고를 하나도 내지 않는다.
- 목차(`table-of-contents.html`, `toc.ncx`)의 항목이 번역된 본문 제목과 글자까지
  같다.
- Pages 배포 배선, 랜딩 페이지 항목, README 출처 표기, 루트 `LICENSE.md` 항목이
  갖춰져 있다.

## 조사 결과

- **제공 형식.** EPUB, Digital PDF(화면용), Print PDF(인쇄용), MOBI, Pressbooks
  XML(`type=wxr`, WordPress WXR)을 제공한다. 공개 소스 저장소는 없다. PDF·MOBI는
  EPUB과 같은 원본에서 만든 산출물이다. WXR은 원본에 가장 가깝지만 WordPress 블록
  마크업과 Pressbooks 단축코드를 읽어야 하고, 번역한 결과를 독자에게 낼 렌더러가 없다.
  EPUB은 렌더링이 끝난 XHTML이라 번역 결과를 그대로 책과 웹으로 낼 수 있다.
- **EPUB 구조.** EPUB 2.0. `META-INF/container.xml` → `book.opf`(루트에 있음),
  `toc.ncx`(navPoint 30개), `OEBPS/` 아래 콘텐츠 문서 30개(`.html`, XHTML 1.1),
  `jacobs.css`, `OEBPS/assets/`(PNG 57, JPEG 11, 글꼴 4, 18MB). spine은 표지 →
  제목 → 판권 → 헌사 → 목차 → 머리말 5개 → 15개 장 → 부록 4개 → 용어집 순서다.
  콘텐츠 문서는 1.35MB이고, 가장 큰 장은 약 100KB다.
- **마크업.** XHTML 파서 설계 2절 참고. Pressbooks 구조는 `div.chapter` 안의
  `chapter-title-wrap`(장 번호 `h3`, 제목 `h2`), 절 제목 `h1.section-header`,
  `div.textbox`(학습 목표, 핵심 정리, 사이드바), `div.wp-caption`(그림과 캡션),
  `div.blockquote`, 각주 3개, oEmbed 자리(유튜브 썸네일 이미지와 링크) 6개다.
- **목차.** `table-of-contents.html`의 링크 217개는 모두 실재하는 문서·앵커를
  가리킨다. 201개는 대상 제목과 텍스트가 같고, 나머지 16개는 장 번호·저자 표기가
  별도 `span`으로 붙은 경우다.
- **라이선스.** 책은 CC BY 4.0("except where otherwise noted")이다. 예외 두 건:
  부록 B의 PDCA 그림(Christoph Roser, AllAboutLean.com, CC BY-SA)과 1장의 그림 하나
  (The Minerals, Metals & Materials Society, 2003, "Used with permission"). 그 밖의
  "public domain" 표기 3건은 제약이 없다.
- 본문 그림은 영어 글자가 든 도표가 많다.

## 범위

spine의 콘텐츠 문서 30개 가운데 다음을 번역한다.

- 번역: 제목 페이지, 판권, 머리말 6개, 15개 장, 부록 4개, 용어집의 본문·제목·
  캡션·`textbox`·각주, 이미지 `alt`·요소 `title` 속성, 각 문서의 `head` `title`.
- 번역하지 않고 빌드가 만듦: `table-of-contents.html`의 항목 텍스트, `toc.ncx`의
  `navLabel`, OPF의 `dc:title`. 모두 번역된 본문 제목에서 옮긴다(접근 방식 4).
- 번역하지 않음: `front-cover.html`(이미지뿐), 그림 속 글자, URL, 인용 문헌
  제목(참고 문헌 목록의 서지 사항은 원문 그대로 두고 문장만 번역), 사람·기관 이름.

## 접근 방식

### 1. 원문 고정

git 저장소가 없으므로 서브모듈 대신 **압축을 푼 트리를 커밋**한다. 원본 `.epub`
하나를 커밋하는 안보다 낫다고 본 이유는 다음과 같다.

- `[[sources]]`가 파일을 바로 가리킨다(압축 해제 `derive` 단계가 필요 없다).
- 원문을 다시 내려받았을 때 바뀐 문서가 텍스트 diff로 보인다. Pressbooks 내보내기는
  매번 다시 만들어지므로 URL만으로는 고정되지 않는다.

`upstream.lock`(TOML)에 `url`, `retrieved`(날짜), `sha256`(원본 EPUB,
`29bfed83fde9158f87a05b5aa9465c6111f47bdd5c75d4f22d3779c2fbeefd8a`), 파일 수를
적는다. 저장소에 18MB(대부분 이미지)가 더해진다. 원문을 올릴 때는 새 EPUB을 풀어
트리를 바꾸고 lock을 갱신한다. 이때는 체크리스트 7절대로 `reconcile`을 쓰지 않는다.

### 2. `yeokja.toml`

```toml
[project]
source_lang = "en"
target_lang = "ko"
state_dir = "state"

[[sources]]
path = "upstream/OEBPS"
pattern = "*.html"
exclude = ["front-cover.html", "table-of-contents.html"]
parser = "xhtml"
output = "ko/OEBPS/{path}"

[provider]
type = "claude_code"
model = "claude-opus-5-5"

[derive]
base = "upstream"

[[derive.overlay]]
path = "ko"
require_base = true

[build.epub]   # build_epub.py . technicalpm-ko.epub
[build.html]   # build_epub.py 다음 build_site.py . site
```

빌드 명령은 조립 트리(`build/tree`) 안에서 `YEOKJA_ROOT`와 함께 돈다. 기본
프롬프트를 쓴다. 기본 프롬프트가 한국어에 합쇼체를 요구하므로 교재 문체에 맞다.
커스텀 템플릿이 필요해지면 AGENTS.md의 GFM 마커 문장과 `{inline_tags}`
자리표시자를 넣는다.

### 3. 용어

`glossary.toml`에 책의 핵심 개념을 먼저 고정한다. 장마다 반복되는 개념이라 첫 번역
전에 정해야 재번역이 적다.

- *living order* / *geometric order*: 책 제목이자 중심 개념이다. "살아 있는
  질서"/"기하학적 질서"로 옮기고, 첫 등장에만 원어를 괄호로 병기한다.
- PMBOK 용어(scope, work breakdown structure, stakeholder, critical path, earned
  value, risk register 등)는 PMBOK 지침서 한국어판 용어를 따른다. 약어(WBS, EVM,
  CPM, PERT)는 영문으로 둔다.
- Lean 용어(*muda*, pull/push, value stream)는 국내 Lean 문헌의 관용 표기를 따른다.
  *Lean*, *Agile* 자체는 용어집에 두지 않는다. 기관·방법론 이름(Lean Construction
  Institute 등)에서 영어로 남아야 하는 곳이 많아 용어 검사가 오탐으로 재시도를
  일으킨다(1장 시험 번역에서 재시도 7건이 모두 이 오탐이었다).

첫 번역 후 `yeokja terms build`로 제목·반복 용어를 고정한다(체크리스트 3절).

### 4. 빌드

`scripts/build_epub.py`는 `build/tree`(번역이 겹쳐진 트리)에서 EPUB을 만든다.
표준 라이브러리만 쓴다.

1. 번역된 콘텐츠 문서에서 장·절 제목을 읽어 `table-of-contents.html`의 항목
   텍스트(`span.toc-chapter-title`, `span.toc-subsection-title`)를 링크 대상의 제목
   텍스트로 바꾼다. `toc.ncx`의 `navLabel`도 같은 방식으로, OPF `dc:title`은 제목
   페이지에서 바꾼다. 대상을 못 찾으면 실패한다.
2. 모든 콘텐츠 문서의 `xml:lang`과 OPF `dc:language`를 `ko`로 바꾼다.
3. 판권 페이지 끝에 번역 고지를 넣는다(라이선스 절).
4. 1장의 TMS 그림을 자리 표시 상자로 바꾸고 이미지 파일과 manifest 항목을 뺀다
   (라이선스 절).
5. `mimetype`을 무압축 첫 엔트리로, 나머지를 deflate로 묶는다.

`scripts/build_site.py`는 같은 트리의 `OEBPS/`를 정적 사이트로 내놓는다. 순서는
`index.html`(랜딩: 책 소개, 목차 링크, EPUB 내려받기) → `table-of-contents.html` →
spine 순서다. 각 문서에 이전·다음 링크와 공통 하단(저작자 표시, 번역 고지)을 넣는다.
문서는 XHTML이지만 `.html`로 내놓아 브라우저가 HTML로 읽는다. 엔티티를 문자 참조로
쓰므로(XHTML 파서 설계 3.3) 어느 쪽으로 읽어도 같다. 각 문서는 원서 `jacobs.css`
뒤에 `site.css`를 얹는다. `site.css`는 읽기 폭과 한국어 줄바꿈을 정하고, 리더용으로
`small`인 본문 글자 크기를 `1rem`으로 되돌리며(학습 상자와 표는 원서 비율대로 그보다
작다), 원서 규칙의 잘못된 색 값 때문에 사라지는 표 칸 테두리를 그리고, 다크 모드
색을 정한다.

빌드 툴체인은 `nix/projects/technicalpm.nix`에 `python3` 하나를 선언한다. 빌드와
검사 스크립트는 표준 라이브러리만 쓴다. `epubcheck` 비교는 JDK를 끌어오므로 CI
셸에 넣지 않고 README의 로컬 검증 절차로 둔다. 원문 EPUB의 결과는 RSC-006(원격
리소스)과 RSC-012(없는 조각 링크) 각 3건이다.

### 5. 검사

`scripts/check.py`는 다음을 본다.

- 30개 문서의 골격 동치. CLI 검사가 생기면 그것으로 바꾼다.
- 목차·NCX 항목과 본문 제목의 일치, 목차 링크 217개의 대상 존재.
- `epubcheck` 결과를 원문 EPUB의 결과와 비교한 새 오류·경고 0개.

## 라이선스

원문은 CC BY 4.0이다. 번역본은 개작물로서 CC BY 4.0으로 배포하고, §3(a)에 따라
다음을 지킨다.

- 판권 페이지와 웹 판 공통 하단에 저작자(Jeffrey S. Russell, Wayne P. Pferdehirt,
  John S. Nelson, Board of Regents of the University of Wisconsin System), 원문 링크,
  라이선스 링크, "원문을 번역·수정한 비공식 번역본" 고지를 넣는다.
- 원문 판권 페이지의 문장은 번역하되, 원문 라이선스 문장은 영어 원문을 함께 남긴다.
- CC BY-SA 그림(PDCA)은 수정 없이 싣고 캡션의 출처·라이선스 표기를 유지한다.
  그림을 수정하지 않으므로 책 전체에 BY-SA가 전파되지 않는다.
- **"Used with permission" 그림(1장, TMS)** 은 원 출판사가 받은 허락이라 번역본에
  미치는지 알 수 없으므로 싣지 않는다. 빌드가 그 `img`를 빼고, 번역한 캡션과 원 출처
  링크를 담은 자리 표시 상자로 바꾼다. 이미지 파일도 EPUB·사이트 산출물에서 빼고
  OPF manifest 항목을 지운다. 골격 동치 검사는 이 한 곳을 예외로 둔다.
- README는 AGENTS.md의 출처 표기 규칙을 따른다. 루트 `LICENSE.md` 표에 CC BY 4.0을
  추가한다.

## 배포

`.github/pages-projects.json`에 `project = "technicalpm"`, `target = "html"`을
더하고, `stage-pages.sh`에 스테이징 경로를 더한다. EPUB 파일은 사이트 산출물에 함께
넣어 랜딩 페이지에서 내려받게 한다(`build.html`이 `build_epub.py`를 먼저 부른다).

## 작업 순서

1. XHTML 파서(별도 spec): 판독기 분리, `parser-xhtml`, `inline/xhtml.rs`,
   `Markup::Xhtml`, 골격 동치 검사, 이 책 30개 문서로 무손실 왕복 테스트.
2. 프로젝트 골격: `upstream/`, `upstream.lock`, `yeokja.toml`, `glossary.toml`,
   README, `.gitignore`(`ko/`, `build/`, `dist/`).
3. 한 장(1장)만 번역해 태그 판독 재시도율, 문체, 용어를 검토하고 프롬프트·용어집을
   고친다.
4. 나머지 번역 → `terms build` → `glossary adopt` → 남은 세그먼트 번역.
5. 빌드 스크립트, 검사, nix devShell, Pages 배선.

## 되짚을 수 있는 결정

- **EPUB을 원문으로 삼음.** 원 저자 측이 Pressbooks 원본을 고치면 다시 내려받아야
  하고, 내보내기 HTML이 Pressbooks 버전에 따라 달라질 수 있다. 그런 일이 잦아지면
  WXR을 원문으로 삼고 Pressbooks 렌더링을 흉내 내는 안을 다시 본다.
- **목차를 빌드가 만듦.** 목차를 따로 번역하면 본문 제목과 어긋날 수 있어 결정적으로
  옮긴다. 목차 고유의 문구가 생기면 그 부분만 번역 대상으로 연다.
- **그림 속 영어.** 그림은 번역하지 않는다. 독자 피드백에 따라 주요 도표만 다시
  그리는 안을 별도로 다룬다.
