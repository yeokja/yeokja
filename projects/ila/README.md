# 대화형 선형대수 — Interactive Linear Algebra 한국어판

Dan Margalit, Joseph Rabinoff의 개방형 교재
[*Interactive Linear Algebra*](https://textbooks.math.gatech.edu/ila/)(Georgia Institute of
Technology, 소스 [QBobWatson/ila](https://github.com/QBobWatson/ila))를 한국어로 옮긴
비공식 번역입니다. [yeokja](https://github.com/yeokja/yeokja)와 함께 Anthropic 사의
`claude-opus-5-5` 모델을 활용하여 번역되었으며 학습을 모두 비허용한 상태로
작업하였습니다.

## 범위

- 원문: `upstream/` 서브모듈(커밋 `a8593c2`). 원서는 Rabinoff가 고친 MathBook XML(PreTeXt의
  전신)이고, 빌드에 쓰는 XSLT(`mathbook`)와 스타일(`mathbook-assets`)은 upstream 안의 중첩
  서브모듈입니다(`mathbox`는 받지 않습니다).
- 번역 대상: `upstream/src/*.xml` 가운데 라이선스 원문 `gfdl.xml`과 편집기용 `schemas.xml`을
  뺀 38개 파일(`pretext` 파서, 5,717 세그먼트). 본문·제목·캡션·목표 상자·풀이와 `type-name`
  속성을 번역하고, default 판과 1553 판의 텍스트를 모두 번역합니다.
- 용어는 국내 선형대수 교재 관례를 따릅니다(연립일차방정식, 일차결합, 생성, 기저, 고윳값 …;
  `glossary.toml`, `terms.toml`). rank는 "랭크"로 씁니다.
- 1단계에서 영어로 남는 것:
  - 수식 안의 영어(`\text{}`·`\sptxt{}`)와 TikZ 그림 속 라벨. 수식은 원문 LaTeX 그대로 조판합니다.
  - 찾아보기(색인어 `idx`).
  - 대화형 데모 157개. 다시 빌드하지 않고 원서 사이트
    `https://textbooks.math.gatech.edu/ila/demos/…`의 데모를 iframe으로 보여 줍니다. 데모 안의
    글자는 영어이고, 원서 사이트가 사라지면 데모가 빈 틀로 보입니다.
- 웹 판은 default 판만 냅니다. PDF는 만들지 않습니다.

## 라이선스

원서 본문은 [GNU Free Documentation License](https://www.gnu.org/licenses/fdl-1.3.html)
1.3 이상(불변 부분·앞표지·뒤표지 문구 없음)이고, 빌드에 쓰는 XSLT 포크(MathBook XML)는 GPL,
데모 프레임워크 MathBox는 MIT입니다. 이 번역은 GFDL 4절의 수정판(Modified Version)으로 같은
GFDL로 제공하며, 웹 판이 다음을 갖춥니다.

- 원제와 구분되는 제목 "대화형 선형대수 — Interactive Linear Algebra 한국어판"과 제목
  페이지의 번역자 표시(yeokja 프로젝트).
- 판권 페이지: 원 저작권 고지와 라이선스 고지(번역문과 영어 원문), 번역판의 저작권·라이선스
  고지, 투명 사본의 위치.
- "이력 (History)" 부록: 원서 항목과 이 번역판 항목(원문 커밋, 번역 방법, 바뀐 점).
- 라이선스 부록(`gfdl.xml`)은 번역하지 않고 영어 원문 그대로 둡니다.
- 투명 사본: 번역의 진실의 원천인 `state/`, 오버레이, 빌드 스크립트가 이 저장소에 있습니다.

`overlay/`의 한국어 지역화(`mathbook/xsl/localizations/ko-KR.xsl`)와 XSLT 덮어쓰기
(`src/xsl/ila-ko.xsl`)는 원 파일과 같은 GPL입니다. 저자의 공식 번역이 아닙니다.

## 재현

```sh
git -C projects/ila/upstream submodule update --init mathbook mathbook-assets
cd projects/ila
../../target/release/yeokja translate upstream/src        # 누락 세그먼트 번역 및 ko/ 재구성
../../target/release/yeokja status --check upstream/src
nix develop path:../../nix#ila -c ../../target/release/yeokja build html   # dist/site
```

번역하려면 인증된 Claude Code CLI가 필요합니다. `state/`는 번역의 진실의 원천이므로
커밋합니다. `ko/`, `build/`, `dist/`는 `state/`와 원문에서 언제든 재생성되므로 커밋하지
않습니다.

원 빌드(SCons + `pretex.py` + 패치된 Inkscape 0.92 + Python 2)는 재현할 수 없어, 빌드
(`scripts/build_html.py`)는 다음과 같이 대신합니다.

1. 원 XSLT: `versioning.xsl`(default 판) → 판본 XML에 한국어판 고지·이력을 끼우고 문서 언어를
   `ko-KR`로 → `overlay/src/xsl/ila-ko.xsl`(책의 `mathbook-html.xsl` + 한국어 화면 문구).
2. `scripts/render_latex.py`: XSLT가 남긴 수식·TikZ 자리표시(서로 다른 스니펫 약 3,600개)를
   `latex`(원서의 `macros.sty`·`spalign.sty`·`jdr-tikz.sty`, mathdesign Charter, kotex)와
   `dvisvgm`으로 SVG화합니다. 결과는 `build/latex-cache/`에 해시로 캐시합니다.
3. CSS는 `mathbook-assets`의 SCSS를 compass로 컴파일하고(`scripts/compass-shim.rb`가 Ruby 3.4
   호환을 메웁니다) 원 빌드처럼 add-on CSS·JS를 묶습니다.

검증(빌드 명령에 들어 있습니다):

```sh
python3 scripts/check.py skeleton upstream/src ko/src   # 번역문의 요소 구조·수식이 원문과 같은지
python3 scripts/check.py site dist/site                 # 링크·knowl·이미지·앵커, 렌더 안 된 수식, lang
```

Deploy Pages 워크플로는 upstream과 그 안의 `mathbook`, `mathbook-assets`를 받고, 커밋된 번역
상태로 `ko/`를 재구성하고 미번역 여부를 검사한 뒤 빌드해, `dist-ila` 산출물을 Pages의
`ila/`에 배치합니다.
