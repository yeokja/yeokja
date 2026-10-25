# Interactive Linear Algebra 한국어 번역 설계

## 목표

Dan Margalit·Joseph Rabinoff의 개방형 교재 *Interactive Linear Algebra*(Georgia Tech,
`https://textbooks.math.gatech.edu/ila/`, 소스 `github.com/QBobWatson/ila`)를 yeokja로
한국어로 옮겨 GitHub Pages 웹 판(`ila/`)을 낸다. 원문은 Rabinoff가 고친 MathBook XML(PreTeXt의
전신)이고, 파서는 `docs/superpowers/specs/2026-10-25-xml-vocabulary-profiles-design.md`의
`pretext`다.

완료 상태는 다음 조건을 모두 만족해야 한다.

- `projects/ila/upstream`이 서브모듈로 커밋 `a8593c2`에 고정되어 있고, 빌드에 쓰는 중첩
  서브모듈 `mathbook`(`d6dc0da`)·`mathbook-assets`(`48944c9`)가 함께 고정되어 있다.
  `mathbox`(데모 프레임워크)는 받지 않는다.
- `src/`의 번역 대상 38개 파일의 모든 세그먼트가 번역 상태이고 `yeokja status --check`와
  `yeokja evaluate --mechanical-only`(0건)가 통과한다.
- 번역한 38개 파일이 원문과 골격 동치다(어휘 프로필 설계 2.7, 아래 접근 방식 6).
- 웹 판이 GFDL 수정판 의무(라이선스 절)를 지키고, 모든 수식·그림이 렌더되며(실패 0개),
  상대 링크·knowl·이미지가 모두 실재한다.
- Pages 배포 배선, 랜딩 페이지 항목, README 출처 표기, 루트 `LICENSE.md` 항목이 갖춰져 있다.

## 조사 결과

- **구성.** 6개 장과 부록(복소수, 기호, 풀이, GFDL), 머리말 3편. `src/ila.xml`이
  `xi:include`로 39개 파일을 묶는다. 커스텀 요소 `latex-code`(TikZ, 253), `mathbox`(WebGL 데모
  iframe, 157), `specialcase`, `bluebox`, `essential`, `restrict-version`(판 구분: default·1553,
  48), 번역할 속성 `type-name`(33). `m` 7,886, `me` 1,328(그중 `p` 안 1,157), `idx` 712.
- **수식은 실제 LaTeX다.** `macros.sty`·`spalign.sty`(`\mat{1 2; 3 4}`, `\syseq`)·`jdr-tikz.sty`를
  쓰고 MathJax가 처리하지 못하는 매크로가 많다. 원 빌드는 모든 수식을 `pdflatex`로 조판한다.
- **원 빌드 체인**(SConstruct, Vagrant ubuntu1804): `versioning.xsl` → xmllint RelaxNG →
  `mathbook-html.xsl`(수식은 `<script type="text/x-latex-*">` 자리표시) → `pretex.py`
  (pdflatex + 패치된 Inkscape 0.92.3 + Python 2 `tounicode.py` + FontForge) → compass CSS,
  uglify → 데모(mako + CoffeeScript 1.x, mathbox gulp3/browserify). Nix로는 XSLT·TeX·compass는
  가능하고 Inkscape 0.92·Python 2·CoffeeScript 1.x·gulp3은 불가능하다.
- **XSLT 단계 실측.** xsltproc로 HTML 53쪽과 knowl 3,982개가 나오고 경고가 없다. 자리표시는
  inline 26,499, display 4,695, code 779, code-bare 62개이고 서로 다른 스니펫은 약 3,700개다.
- **다국어.** mathbook `xsl/localizations`에 `ko-KR`이 없다(`en-US.xsl` 129문자열).
  `src/xsl/mathbook-html.xsl`에 영어가 하드코딩된 곳이 있다(의견 링크, PDF 링크, 데모 제목,
  메뉴 단추, 아이콘 대체 텍스트).
- **라이선스.** 본문 GFDL-1.3-or-later(파일 머리 주석, `gfdl.xml`; 판권 `shortlicense`는
  "1.2 or later"), 불변 부분·표지 문구 없음. 루트 LICENSE 없음. XSLT 포크(`mathbook`)는
  GPL-2/3, `mathbox`는 MIT, `demos/`는 표기 없음.
- **원서 웹 판**(textbooks.math.gatech.edu/ila)은 옛 저장소 gt-linalg의 2019년 빌드로, 현재
  소스보다 오래됐다. 데모(`ila/demos/*.html`)는 그 사이트에서 그대로 열린다.

## 범위

- 번역: `src/*.xml` 가운데 `gfdl.xml`(라이선스 원문)과 `schemas.xml`(편집기용 스키마 연결)을
  뺀 38개. 본문·제목·캡션·목표 상자·풀이, `type-name` 속성, `restrict-version`의 두 판 모두
  (`pretext` 파서 기본값). 세그먼트 5,717개.
- 번역하지 않음(1단계): 수식 안 `\text{}`·`\sptxt{}`와 TikZ 라벨(영어로 남는다), 색인 `idx`
  (찾아보기 페이지는 영어), 데모 안의 글자, 인명·URL, `gfdl.xml`.
- 빌드: default 판 하나의 웹 판. 1553 판과 PDF는 내지 않는다.

## 접근 방식

### 1. 원문 고정

호출 측이 `upstream`을 `a8593c2`에 고정하고 중첩 서브모듈 `mathbook`·`mathbook-assets`를
초기화한다. CI는 `projects/ila/upstream`을 비재귀로 받으므로, 이 프로젝트의 Pages 항목은
`nested_submodules = "mathbook mathbook-assets"`로 두 경로만 더 받는다.

### 2. `yeokja.toml`

```toml
[[sources]]
path = "upstream/src"
pattern = "*.xml"
exclude = ["gfdl.xml", "schemas.xml"]
parser = "pretext"
output = "ko/src/{path}"

[provider]
type = "claude_code"
model = "claude-opus-5-5"

[derive]
base = "upstream"
[[derive.overlay]]   # 번역 미러
path = "ko"
require_base = true
[[derive.overlay]]   # 원문에 없는 파일: ko-KR.xsl, ila-ko.xsl, ko-edition.xml
path = "overlay"
require_base = false

[translation]
concurrency = 6
```

`pretext` 소스는 언제나 인라인 태그로 주고받는다. 기본 프롬프트를 쓴다(합쇼체, GFM 경고 마커
규칙 포함). `[build.html]`은 골격 검사 → `build_html.py` → 사이트 검사 순서다.

### 3. 용어

국내 선형대수 교재 관례를 따른다. "일차/선형"은 교재 관례대로 연립일차방정식·일차결합·
일차독립/일차종속과 선형변환을 함께 쓴다. 생성(span), 기저, 차원, 열공간/영공간, 행 축약,
(기약) 행 사다리꼴, 피벗, 자유변수, 첨가행렬, 행렬 변환, 일대일/전사, 가역, 단위행렬,
행렬식, 여인수, 고윳값/고유벡터/고유공간, 특성다항식, 대각화 가능, 회전-확대 행렬,
확률행렬/정상상태, 내적, 직교여공간, 정사영, 정규직교, 최소제곱해.

- rank는 계수(coefficient)와 겹치지 않도록 "랭크"로 쓴다.
- 번역판 제목 "대화형 선형대수 — Interactive Linear Algebra 한국어판"을 용어집에 고정한다.
  책 제목 세그먼트 전체가 이 문자열이 된다(라이선스 절, GFDL 4.A).
- 수식 매크로와 겹치는 단어(`\Span`, `\rank`, `\Nul`, `\Col`)와 일상어(onto, inverse,
  consistent)는 용어집에 두지 않고 `terms.toml`에 맡긴다. 용어 검사는 세그먼트 원문의 TikZ
  코드·색인·디스플레이 수식 바이트도 읽으므로, 1단계에서 영어로 남는 그 부분의 용례는
  `except`로 뺀다(TikZ 스타일 `\draw[vector]`, 색인 `<h>Column space</h>`, 수식 안
  `\text{...}`).

### 4. 빌드(B안: 원 XSLT + 새 수식 렌더러)

원 빌드를 그대로 재현할 수 없어 다음 안을 비교했다.

| 안 | 판단 |
|---|---|
| A. 원 빌드(SCons + pretex) 재현 | 불가. 패치된 Inkscape 0.92, Python 2, FontForge 스크립트에 묶여 있다. |
| **B. 원 XSLT로 HTML, 수식·TikZ는 새 렌더러(latex → dvisvgm)** | **채택.** XSLT와 TeX 매크로를 그대로 쓰므로 원서와 같은 조판이 나온다. 렌더러는 Python 표준 라이브러리 하나다. |
| C. MathJax로 수식 처리 | 기각. `\syseq`·`spalign`·TikZ 253개를 처리하지 못한다. |
| D. 렌더 판 HTML을 직접 번역 | 기각. well-formed가 아니고, 2019년 판이라 현재 소스와 다르며 4,000개가 넘는 파일이다. |

`scripts/build_html.py`(조립 트리에서 실행):

1. SCons가 만들던 `src/xsl/git-hash.xsl`(원문 커밋)과 `theme.xsl`(gt 테마)을 만든다.
2. `xsltproc --xinclude --stringparam version default versioning.xsl ila.xml`.
3. 판본 XML에 한국어판 조각(`overlay/src/ko-edition.xml`)을 끼운다: 제목 페이지의 번역자
   표시와 날짜, 판권의 원 고지 영어 원문·번역판 저작권·라이선스 고지·투명 사본 위치, GFDL
   부록 앞의 "이력 (History)" 부록. 문서 언어를 `ko-KR`로 바꾸고, "Variants" 머리말이 원서
   사이트의 루트 상대 경로(`/ila`, `/ila/1553`)로 거는 링크를 원서 사이트 절대 주소로 바꾼다.
   이 변경들은 번역된 `ko/` 파일이 아니라 판본 XML에 적용하므로 번역 상태와 무관하다.
4. `xsltproc overlay/src/xsl/ila-ko.xsl`: 책의 `mathbook-html.xsl`을 import하고 영어가
   하드코딩된 템플릿만 덮어쓴다(메뉴 단추, 아이콘 대체 텍스트, 의견 링크는 yeokja 이슈로, PDF
   링크와 표지 애니메이션은 비움, 묶음 CSS/JS 경로, 데모 iframe). mathbook 경고·오류가 하나라도
   있으면 실패한다(예: `ko-KR.xsl`에 없는 문자열).
5. `scripts/render_latex.py`가 자리표시를 SVG로 바꾼다(5절).
6. CSS: `mathbook-assets`의 SCSS를 compass로 컴파일하고 SConstruct의 순서대로
   `mathbook-add-on.css`, `ila-add-on*.css`, `knowlstyle.css`와 `scripts/ko.css`를 붙인다.
   nixpkgs의 compass 1.0.3은 Ruby 3.4에서 사라진 `File.exists?`를 불러 멈추므로
   `scripts/compass-shim.rb`를 `RUBYOPT=-r`로 싣는다. 렌더 판 CSS를 내려받아 고정하는 대안은
   필요 없어졌다. JS는 SConstruct처럼 jQuery·sticky·knowl·`Mathbook.js`를 이어 붙인다.
   글꼴·이미지·`manifest.json`을 복사하고, `index2.html`(앞부분)을 `index.html`로도 둔다(원
   빌드와 같다).

**지역화.** `overlay/mathbook/xsl/localizations/ko-KR.xsl`은 `en-US.xsl`의 129문자열을 모두
옮긴 것이다(GPL, 원 파일과 같은 라이선스). mathbook이 `localizations/{xml:lang}.xsl`을
스타일시트 기준 상대 경로로 읽으므로, 조립 트리(심볼릭 링크 농장)에 겹친 파일이 그대로 쓰인다.

**데모.** 157개 MathBox 데모는 1단계에서 다시 빌드하지 않고 iframe이 원서 사이트
`https://textbooks.math.gatech.edu/ila/demos/…`를 가리킨다(`loading="lazy"`). 데모 안의 글자는
영어다. 원서 사이트가 사라지면 데모가 빈 틀이 된다(되짚을 수 있는 결정).

**툴체인**(`nix/projects/ila.nix`): `texliveBasic.withPackages`(collection-latexrecommended,
-pictures, -langkorean, -fontsrecommended, amscls, mathdesign, charter, helvetic, dvisvgm),
libxslt, libxml2, python3, compass, ghostscript.

### 5. 수식 렌더러

`scripts/render_latex.py`는 pretex.py의 역할을 Nix가 제공하는 도구로 대신한다.

- 자리표시 종류: `inline`(`$…$`), `code-inline`(텍스트 모드), `display`(`equation*` 등),
  `code`(TikZ 등, 디스플레이로 보임), `code-bare`(`\usetikzlibrary` 같은 부수 효과; 같은 HTML
  파일에서 뒤따르는 스니펫의 문맥이 된다 — pretex가 HTML 파일마다 문서 하나를 조판한 것과 같다).
- 프리앰블은 pretex와 같은 `amsart` 12pt + mathdesign Charter에 `kotex`를 더하고, XSLT가 만든
  `preamble.tex`(`\usepackage{macros}` 등)와 `src/latex`(TEXINPUTS)를 쓴다. 문서 클래스에
  `dvisvgm` 옵션을 주어 graphicx·pgf가 dvisvgm 드라이버를 쓴다.
- 인라인은 상자를 그대로 한 쪽으로 내보내고(`\shipout`) 쪽 크기를 상자로 정해(`papersize`
  special) `dvisvgm --bbox=papersize`로 기준선을 정확히 지킨다. 디스플레이는 원서 쪽 너비 7in의
  minipage에 조판해 `--bbox=min --exact-bbox`로 잘라 낸다. 디스플레이의 `\tag`가 하나면 이미지
  밖 오른쪽 HTML 라벨로 그린다(pretex와 같다).
- 결과는 `<img src="latex/<해시>.svg">`이고 크기는 em(12 TeX pt = 1em)으로 준다. 대체 텍스트는
  LaTeX 원문이다. 수식에 붙은 조사·문장 부호는 `span.pretex-bind`(줄바꿈 금지)로 묶는다
  ("<m>A</m>의"가 갈라지지 않게). 같은 스니펫은 한 파일을 공유하고, SVG를 `<img>`로 보이므로
  여러 SVG의 glyph id가 충돌하지 않는다. TikZ가 포함하는 사진(`figure-images/*.jpg`)은 SVG 안에
  data URI로 넣는다(`<img>`로 보이는 SVG는 외부 파일을 읽지 않는다).
- 캐시: 스니펫마다 (렌더러 버전, 프리앰블, `src/latex` 파일 해시, bare 문맥, 종류, 코드)의 해시로
  `$YEOKJA_ROOT/build/latex-cache/`에 SVG와 크기를 둔다. 40개씩 한 문서로 묶어 병렬로 조판하고,
  묶음이 실패하면 하나씩 다시 조판해 실패한 스니펫만 보고한다. 실패가 하나라도 있으면 빌드가
  실패한다.

### 6. 검사

`scripts/check.py`:

- `skeleton UPSTREAM_SRC KO_SRC`: 코어 `skeleton_with`를 쓰는 CLI가 없으므로
  `projects/technicalpm/scripts/check.py` 방식으로 Python에서 같은 비교를 한다. 블록(인라인
  화이트리스트 밖의 요소와 블록을 품은 요소)은 순서와 중첩까지, 블록 안 인라인 요소는
  multiset으로, 강조(`em`·`term` 등)는 있는지만 본다. 인라인 수식 `m`·`c`도 있는지만 본다 — 한국어
  문장은 영어 대명사 자리에 수식을 되풀이한다("<m>A</m>의 열 … <m>A</m>의 행"). 번역하지 않는
  요소(`m`, `me`, `latex-code`, `mathbox`, `idx`, `docinfo` …)는 내용까지 바이트로 같아야 하고,
  속성은 `type-name` 값만 다를 수 있다.
- `site SITE`: 모든 상대 링크·knowl·이미지가 파일로 있고, `#조각`이 대상 쪽의 id로 있으며,
  자리표시가 남지 않았고, 모든 쪽이 `lang="ko-KR"`이다. 책의 스타일시트가 knowl 안에서 태어나는
  요소에 id를 주지 않으면서 "본문에서 보기"로 거는 링크와 찾아보기의 빈 글자 링크(1,263개)는
  영어 빌드에도 똑같이 있으므로 세기만 하고 문제로 보지 않는다.

## 라이선스

본문은 GFDL 1.3 이상이다. 번역판은 GFDL 4절의 수정판(Modified Version)으로 같은 라이선스로
배포하고, 다음을 지킨다.

- **4.A 제목.** 원제와 다른 제목 "대화형 선형대수 — Interactive Linear Algebra 한국어판"을 쓴다
  (용어집에 고정해 책 제목 세그먼트가 이 문자열이 된다). 원제는 부제처럼 함께 적어 출처를
  드러낸다.
- **4.B·4.C 저자와 발행자.** 제목 페이지에 원저자 두 사람 뒤에 "한국어판 번역: yeokja
  프로젝트"를 둔다.
- **4.D–4.F 고지.** 원 저작권 고지(2017 Georgia Institute of Technology)와 그 라이선스 고지의
  번역문을 그대로 두고, 바로 뒤에 영어 원문, 번역판 저작권(2026 yeokja 프로젝트)과 라이선스
  고지를 둔다. GFDL 8절에 따라 라이선스 고지의 번역은 원문과 함께 싣는다.
- **4.H 라이선스 원문.** `gfdl.xml`은 번역하지 않고 영어 그대로 부록에 남는다.
- **4.I 이력.** 원서에 History 절이 없으므로 "이력 (History)" 부록을 새로 두고, 원서 제목
  페이지의 사항(제목, 2017년, 저자, Georgia Institute of Technology)으로 첫 항목을, 이 판의
  제목·연도·번역자·원문 커밋·바뀐 점으로 둘째 항목을 적는다.
- **4.J 네트워크 위치.** 이력에 원문 소스와 원서 웹 판 주소를 적는다.
- **투명 사본(3절).** 번역 상태(`state/`), 오버레이, 빌드 스크립트가 공개 저장소에 있고 판권에
  그 위치를 적는다.
- `mathbook`(XSLT, GPL)을 고친 부분(`ko-KR.xsl`, `ila-ko.xsl`)은 원 파일과 같은 GPL이다.
  데모는 원서 사이트의 것을 가리킬 뿐 배포하지 않는다.
- README는 AGENTS.md의 출처 표기 규칙을 따르고, 루트 `LICENSE.md` 표에
  GFDL-1.3-or-later(XSLT 포크 GPL, mathbox MIT)를 더한다.

## 배포

`.github/pages-projects.json`에 `project = "ila"`, `target = "html"`,
`nested_submodules = "mathbook mathbook-assets"`를 더하고 `stage-pages.sh`에서
`projects/ila/dist/site`를 Pages의 `ila/`에 둔다. `rebuild-translations.sh`는 `upstream/src`
전체를 `status --check`한 뒤 `translate`로 `ko/`를 재구성한다. 랜딩 페이지 카드는 "Math"
주제에 둔다. 사이트는 약 70MB(수식 SVG 49MB, knowl 19MB)이고 캐시 없이 빌드에 1분쯤 걸린다.

## 작업 순서

1. 프로젝트 골격: `yeokja.toml`, `glossary.toml`, `.gitignore`, README.
2. 시험 번역(`vectors.xml`): 수식 `<m>` 원자, `p` 안 디스플레이 수식 자리표시, 재구성 diff 확인.
3. 전체 번역 → `evaluate --mechanical-only` → `terms build` → `terms.toml` 검토(코드 속 용례
   `except`, 어색한 결정 수정·제거) → `glossary adopt --prefer HEAD` → 남은 세그먼트 번역.
4. 오버레이(`ko-KR.xsl`, `ila-ko.xsl`, `ko-edition.xml`), 빌드·렌더·검사 스크립트, Nix 셸.
5. 빌드와 브라우저 확인, Pages 배선(호출 측 통합).

## 되짚을 수 있는 결정

- **데모를 원서 사이트에 기댐.** 사이트가 사라지거나 바뀌면 157개 데모가 빈다. 그때는
  `demos/`(mako + CoffeeScript 1.x + mathbox gulp3)를 현대 도구로 다시 빌드하거나, 원서 사이트의
  데모 산출물을 라이선스를 확인한 뒤 고정해 함께 배포하는 안을 본다.
- **수식 안 영어와 TikZ 라벨.** `\text{}`·`\sptxt{}` 293곳과 TikZ 라벨은 영어로 남는다.
  `pretext` 어휘에 수식 안 `\text{}` 섬과 TikZ 라벨 번역(LaTeX 확장 파서)을 더하면 다시 본다.
  렌더러는 kotex를 이미 싣고 있어 한글 라벨을 그릴 수 있다.
- **색인.** `idx`(색인어 1,414개)를 번역하지 않아 찾아보기가 영어다. 한국어 정렬 키(`sortby`)와
  함께 정해야 한다.
- **default 판만 빌드.** 1553 판 전용 텍스트도 번역되어 있으므로 판을 더하는 것은 빌드 대상
  추가뿐이다.
- **compass 고정.** nixpkgs에서 compass가 빠지면 같은 SCSS를 dart-sass와 compass 믹스인
  대체 파일로 컴파일하거나, 그때의 산출 CSS를 고정한다.
- **골격 검사의 인라인 수식 규칙.** `m`을 있는지만 보므로 번역이 같은 수식을 되풀이하는 것은
  허용되고, 수식을 빠뜨리거나 바꾸는 것은 잡힌다.
