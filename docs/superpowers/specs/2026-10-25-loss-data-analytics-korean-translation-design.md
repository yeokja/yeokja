# Loss Data Analytics(2판) 한국어 번역 설계

## 목표

OpenActTexts가 펴낸 개방형 보험계리 교재 *Loss Data Analytics*, 2판
(`OpenActTexts/LDAVer2`, 편집 Edward W. Frees, 장별 저자, bookdown/R Markdown)을
yeokja로 한국어로 옮겨 "손해 데이터 분석(Loss Data Analytics)" gitbook 웹판을 GitHub
Pages의 `loss-data-analytics/`에 낸다. 파서는
`docs/superpowers/specs/2026-10-25-rmarkdown-parser-design.md`의 `rmarkdown`(Rmd)과
`csv`(용어 풀이 사전)다.

완료 상태는 다음을 모두 만족한다.

- `projects/loss-data-analytics/upstream` 서브모듈이 `67f5aab0`에 고정되어 있다.
- 빌드 대상 Rmd 28개 + child 3개, `References/GlossFct.csv`의 Definition 열,
  Gloss 표시어 사전(`gloss/GlossLabels.csv`)의 Label 열이 모두 번역 상태이고
  `yeokja status --check`가 통과한다. `evaluate --mechanical-only`가 0건이다.
- 번역한 Rmd의 청크·인라인 R·디스플레이 수식·교차 참조 토큰이 원문과 같은
  다중집합이다(`scripts/check.py source`).
- devShell에서 bookdown gitbook이 오류 없이 knit되고, 산출물의 장 수·로컬 링크·
  앵커·knitr 오류 출력을 `scripts/check.py site`가 확인한다.
- README 출처 표기, 루트 `LICENSE.md` 표, Pages 배선, 랜딩 카드가 갖춰져 있다.

## 조사 결과

- **원문.** 최신 커밋 `67f5aab0`(2026-04-03, docs 자료만 바뀜). 본문 `Chapters/`의
  마지막 변경은 2025-02-02. 영어판은 `docs/`에 렌더해 커밋하고 그 디렉터리를 Pages로
  낸다(`https://openacttexts.github.io/LDAVer2/`, bookdown 0.39).
- **빌드.** `_bookdown.yml`의 `rmd_files` 28개(`index.Rmd`, R 코드뿐인
  `References/Packages.Rmd`·`Glossary.Rmd`, 서문, 17개 장, 부록 5개, `PdfGlossary`,
  `Bibliography`)와 child 3개(`Chapters/DistributionTableSections/{Continuous,Discrete,
  LimitedExpValue}.Rmd`). 퀴즈는 `child = './Quizzes/Quiz*.html'`(SurveyJS JSON을 담은
  JS)다. R 청크 449, tikz 14, js 1, 인라인 R 1,275(Gloss 434, HideExample 143,
  SolnBegin/End 124, HideRCode 109). 원문은 R 패키지 버전을 고정하지 않는다.
- **R 패키지.** 원문 Rmd가 `library()`/`::`로 부르는 패키지는 50개이고 CASdatasets를
  뺀 49개가 nixpkgs `rPackages`에 있다. CASdatasets는 CRAN에 없고
  `https://cas.uqam.ca/pub/src/contrib/`에만 있다(1.2-0, `Depends: xts, survival`,
  `Imports: lattice`). 베이즈 장이 `sgautonb` 데이터에 쓴다.
- **용어 풀이.** `Gloss('term'[, 'section'])`은 `References/GlossFct.csv`(507레코드,
  열 Term·Definition·Section)에서 정의를 찾아 녹색 풍선 도움말로 보여 준다. 첫 인자는
  조회 키이자 화면에 보이는 글이다. 빌드 대상에 나오는 서로 다른 첫 인자는 377개다
  (18개는 사전에 없어 원문에서도 풀이가 비어 있다).
- **비결정 요소.** `index.Rmd` 서문의 `` `r format(Sys.Date(), "%d %B %Y")` ``,
  gitbook `in_header`의 Google Analytics 태그(`Format/ShowHide2024GSTag.js`),
  `_output.yml`의 편집 링크가 bookdown-demo를 가리킨다.
- **라이선스.** 저장소 `LICENSE`는 CC0 1.0이다. 본문은 서문(`Chapters/Preface.Rmd`의 CC BY 4.0
  배지)과 기여 안내(`GettingStarted/LICENSE.md`)에서 CC BY 4.0으로 낸다.
- **시험 번역(`Chapters/Frequency2024.Rmd`, 529세그먼트).** 82초, 저장된 문제 0건,
  `evaluate --mechanical-only` 0건. 디스플레이 수식은 줄바꿈이 공백으로 합쳐지는 것
  말고는 그대로이고(빌드 4절의 수식 줄 복원), `\@ref`·`(\#eq:)`·인라인 R·청크는 원문과 같다. 모델은 문장 속
  `` `r Gloss('Aggregate claims')` ``를 한국어 낱말("총청구액")로 읽고 그 받침에 맞춰
  조사를 붙인다.

## 범위

- 번역: 빌드 대상 Rmd 29개(`Packages.Rmd`·`Glossary.Rmd`는 R 코드뿐이라 뺀다)의 제목·
  문단·목록·표·각주, front matter의 `title`·`description`, 청크 머리의 `fig.cap`·
  `fig.alt` 문자열, `(ref:label)` 텍스트 참조 정의, `Hide*(id, label)`의 두 번째 인자
  (접기 단추의 글); `GlossFct.csv`의 Definition 열; Gloss 표시어.
- 번역하지 않음: R·tikz·js 청크 본문, 수식, 레이블·교차 참조·인용 키, 데이터셋·변수·
  패키지·함수 이름, 사람 이름, `.bib`, 퀴즈 JS(`Quizzes/`), 정적 그림 속 글자.
- 1단계에서 영어로 남는 것: R이 실행 중 만드는 표 제목(`TableGen1`·`kbl(caption=…)`),
  그래프 축·범례, tikz 그림 속 글, 퀴즈 문항과 단추. 모두 청크 본문이나 JS 안에 있어
  파서가 번역 대상으로 내놓지 않는다(파서 설계 3절). README에 적는다.
- 빌드하지 않음: PDF(`pdf_document2`, krantz 클래스)와 EPUB. 원문도 웹판이 주이고 PDF는
  일부 장만 낸다.

## 접근 방식

### 1. `yeokja.toml`

```toml
[[sources]]                       # Rmd 29개
path = "upstream"
pattern = "**/*.Rmd"
exclude = [docs/**, GettingStarted/**, IntermediateCalcs/**, Chapters/SimulationSections/**,
  옛 원고(DataResources, ExpRatingBonusMalus, ContinuousOLD), **/*Latex.Rmd,
  References/{PdfOutput,Glossary,Packages}.Rmd]
parser = "rmarkdown"
inline_tags = true
inline_code_captures = [Hide*(id, label)의 label; 큰따옴표·작은따옴표 레이블을 그룹 둘로
  나눠, "R Code for Kendall's Tau"의 '를 레이블 글자로 읽음]

[[sources]]                       # GlossFct.csv, Definition 열만
[[sources]]                       # gloss/GlossLabels.csv, Label 열만 → ko-gloss/References/

[derive]  base = upstream, overlay ko(require_base), ko-gloss, overlay
[translation]  concurrency = 6
```

기본 프롬프트를 쓴다. 파서 설계의 `closing_rule`이 수식·인라인 R·참조·`@{key}` 규칙을
모델에 알리고, 기본 프롬프트가 합쇼체를 요구하므로 교재 문체에 맞는다.

### 2. 용어 풀이(Gloss)

`Gloss()`의 첫 인자는 사전의 조회 키이므로 번역하지 않는다. `GlossFct.csv`는
Definition 열만 번역한다. 그러나 첫 인자는 화면에도 보이는 글이고, 시험 번역에서 모델은
그 자리를 한국어 낱말로 읽어 조사를 붙였다. 영어 키를 그대로 보이면 "Aggregate
claims은"처럼 읽힌다. 그래서 표시어를 따로 번역한다.

- `scripts/gloss_labels.py extract`가 빌드 대상 Rmd에서 첫 인자를 처음 나온 순서로 모아
  `gloss/GlossLabels.csv`(열 `Term`·`Label`, 처음에는 같은 값)를 만든다. 원문을 올리면
  다시 뽑고, `--check`가 CI에서 원문과 어긋남을 잡는다. `Label` 열만 `csv` 파서로
  번역한다. 표시어는 같은 용어집을 따르므로 본문 번역과 같은 번역어가 된다.
- `GlossFct.csv`의 `Term` 열을 번역하고 원문 사본을 키로 쓰는 안은 택하지 않았다. 대소문자·
  복수형이 다른 377개 인자(사전에 없는 18개 포함)에 대응하는 표시어가 나오지 않는다.
- `scripts/prepare_book.py`가 조립 트리의 `References/Glossary.Rmd`를 고쳐 `Gloss()`가
  `GlossLabels.csv`의 표시어를 보이고 풍선 도움말 앞에 원어를 괄호로 붙이게 한다.
  원문 파일을 overlay로 통째로 베끼지 않고 두 자리만 정규식으로 바꾼다(자리가 없으면
  실패한다).
- 모델이 고른 조사가 표시어의 받침과 어긋날 수 있으므로, 같은 스크립트가 본문의
  `` `r Gloss('…')` `` 바로 뒤 조사 은/는, 이/가, 을/를, 과/와, 으로/로를 표시어 끝 음절의
  받침에 맞춘다. 뒤에 한글이 이어지면(이다, 이라고, 로서 …) 조사가 아닐 수 있어 두지
  않는다.

### 3. 용어

`glossary.toml`에 국내 보험계리·손해보험·통계학 교재의 관례를 따라 먼저 고정한다:
손실(loss), 총손실(aggregate loss), 빈도/심도, 공제액, 보상한도, 공동보험, 재보험,
순보험료, 요율 산정, 위험 분류, 언더라이팅, 손해율, 보험계약자, 보험자(insurer; 상법
용어 보험자·보험계약자·피보험자에 맞춤), 신뢰도, 뷜만, 경험요율, 보너스-말러스,
지급준비금, 진전 삼각형, 체인래더, 위험가치(VaR), 꼬리조건부기대값(TVaR), 생존함수,
위험률 함수, 적률생성함수, 혼합분포, 영절단/영수정/영과잉, 코퓰라, 최대가능도, 적합도,
사전분포/사후분포, 절(Section) 등. 약어(IBNR, VaR, GLM)와 R 이름은 영문으로 둔다.
일상어와 겹치는 짧은 단어(loss, claim, risk, premium)는 오탐 재시도를 피하려고 넣지
않고 `terms build`에 맡긴다.

### 4. 빌드

조립 트리(`build/tree`)에서 `[build.html]`이 다음을 한다.

1. `scripts/gloss_labels.py extract --check`.
2. `scripts/prepare_book.py <tree> <원문 커밋 날짜> <upstream>`. 바꿀 자리가 없으면 실패한다.
   조립 트리의 파일은 원문·`ko/`를 가리키는 심볼릭 링크이므로 링크를 끊고 쓴다.
   - 청크 복원: 번역 Rmd의 knitr 청크 본문을 원문 것으로 되돌린다(청크 수·머리가 다르면
     실패). 원문이 `<div align="center">` 안에 둔 청크(Simulation 장 1곳)는 CommonMark가
     HTML 블록으로 읽어, 청크 안 빈 줄 뒤의 R 코드가 문단으로 번역 단위에 들어가 줄이
     합쳐진 채 재구성된다.
   - 이식성(영어 원문 빌드에도 같게 적용, `--portability`): Windows 경로
     `read.csv("Data\\…")`, 현재 VGAM에 없는 링크 이름 `"loge"`(→`"loglink"`), 닫지 않은
     접기 상자 `<div>` 세 곳과 분포표의 닫지 않은 `<p>` 하나. Pandoc 3은 원시 HTML `<div>`를
     Div로 읽어 닫히지 않은 Div가 뒤의 장을 모두 감싸고, bookdown이 그 뒤로 목차와 장 나누기를
     잃는다(원문 렌더 당시의 Pandoc은 그렇게 읽지 않았다).
   - GA 태그 제거, 서문 날짜를 원문 커밋 날짜(`git -C upstream log -1 --format=%cs`)로 고정,
     인라인 R이 만드는 HTML 전용 제목 `# Bibliography {-}`를 `# 참고 문헌
     {#bibliography .unnumbered}`(파일 이름 유지)로, front matter의 `author` 한 줄을 한국어로.
   - 수식 줄 복원: 파서는 디스플레이 수식이 든 문단을 한 줄로 합쳐 내놓는다(파서 설계 3절).
     Pandoc은 줄 가운데의 `\begin{equation}`을 수식으로 읽지 않고, bookdown은 줄 머리의
     `$$`와 제 줄에 홀로 있는 `(\#eq:…)`만 식 번호로 바꾸므로, 번호 수식 환경의 앞뒤와
     `$$` 안 식 레이블(과 그 여는 `$$`)에서 줄을 바꾼다.
   - 참조 링크 복원: 원문 `[Table 8.3](#tab:8.3)`·`[Table 8.3]{#tab:8.3}`은 같은 이름의 참조
     정의(`[Table 8.3]: ./ChapSimulation#tab:8.3`)가 있는데, 파서가 `(#tab:8.3)`을 보존
     토큰으로 가린 채 읽어 참조 링크로 보고 직렬화기가 `[표 8.3][Table 8.3](#tab:8.3)`으로
     쓴다(114곳). `[표 8.3](#tab:8.3)`으로 되돌린다.
   - Gloss 재정의와 조사 맞춤(접근 방식 2).
3. `scripts/check.py source`: 원문과 조립 트리의 청크·인라인 R·수식(공백 런은 하나로)·참조
   토큰 다중집합 비교(위 손질 가운데 의도한 것은 같게 봄), 모든 Gloss 인자에 표시어가 있는지.
4. `Rscript -e "bookdown::render_book('index.Rmd', 'bookdown::gitbook')"`. overlay의
   `_bookdown.yml`이 `output_dir: _book`, 한국어 UI·레이블(`chapter_name: ["제", "장"]`,
   그림/표/식 …)을 정하고, `_output.yml`이 gitbook 설정에서 목차 머리·꼬리 링크를
   한국어판에 맞추고 편집 링크·공유 단추를 뺀다. `Format/ko.css`가 어절 단위 줄바꿈과
   한글 글꼴을 더한다. 로케일은 `LANG=en_US.UTF-8`로 고정한다(CSV에 한글이 들어간다).
5. 원문이 원시 HTML `<img>`(Windows 경로)로 넣어 bookdown이 복사하지 않는 기여자 사진과
   정적 그림을 복사하고, `scripts/check.py site _book 22`로 번호 붙은 장 22개, 로컬 링크·
   앵커, knitr 오류 출력, `??` 교차 참조를 본다. 영어 원문 빌드에도 있는 원문의 결함(없는
   레이블 참조, 저장소 밖 `../docs/` 링크 등 6건)은 `ORIGINAL_DEFECTS`로 세되 실패로 보지
   않는다. 그 뒤 `_book`을 `site`로 옮기고 원문 `LICENSE`를 싣는다.

`nix/projects/loss-data-analytics.nix`는 `rWrapper.override { packages = […] }`에 위
49개 패키지와 `rmarkdown`·`png`, 그리고 `rPackages.buildRPackage` + `fetchurl`(SHA-256
고정)로 만든 CASdatasets 1.2-0을 담는다. 그 밖에 pandoc, TeX Live(`pgf`, `preview`,
`standalone`; knitr tikz 엔진), ImageMagick·Ghostscript·poppler(tikz PDF → PNG),
gifski(gganimate), python3. C++ 컴파일러(nimble)는 `mkShell`의 stdenv가 준다.

nixpkgs 기본 빌드가 이 시스템에서 실패하는 두 패키지는 `rPackages.override`로 고친다:
`pdftools`(configure가 찾는 `poppler-data.pc`를 위해 `poppler_data`), `sf`(transformr가
의존; PROJ 링크 검사가 찾는 libtiff·libcurl과 그 의존·sqlite).

영어 원문을 같은 셸에서 `prepare_book.py --portability`만 거쳐 knit하면 930개 청크가 모두
실행되고 번호 붙은 장 22개가 나온다(약 1분 30초). 한국어판은 식 번호(`\tag`) 80, 표·그림
앵커 99·99, 풍선 도움말 435, 접기 상자 286, 이미지 163개가 영어 빌드와 같다.

## 라이선스

본문은 CC BY 4.0, 저장소 `LICENSE`는 CC0 1.0이다. 본문 쪽 조건이 더 엄격하므로 번역본은
CC BY 4.0 개작물로 내고 README와 루트 `LICENSE.md` 표에 "본문 CC BY 4.0, 저장소
LICENSE CC0 1.0"을 함께 적는다. 저작자(편집 Edward W. Frees와 장별 저자, OpenActTexts),
원문 링크, 라이선스, "원문을 번역한 비공식 번역본" 고지를 README에 두고, 서문의 기여자
목록과 CC BY 배지는 번역본에 그대로 남는다. 사이트 루트에 원문 `LICENSE`를 싣는다.
`Data/*.csv`의 데이터셋은 각자 출처가 있으며 원문과 같이 그대로 싣는다.

## 배포

`.github/pages-projects.json`에 `project = "loss-data-analytics"`, `target = "html"`,
`stage-pages.sh`에 `overlay_site "dist-loss-data-analytics" "site" "loss-data-analytics"`를
더한다. `rebuild-translations.sh`는 재구성 전에 `gloss_labels.py extract --check`와
세 소스의 `status --check`를 돌려 CI가 provider를 부르지 않게 한다. 랜딩 페이지에는 새
주제 "경제·금융"(`finance`)에 카드를 둔다.

## 작업 순서

1. 파서(별도 spec): `rmarkdown`, `csv`, `inline_code_captures`.
2. 골격: `yeokja.toml`, `glossary.toml`, `.gitignore`(`ko/`, `ko-gloss/`, `build/`,
   `dist/`, `__pycache__/`), `gloss/GlossLabels.csv`, overlay, devShell.
3. 시험 번역(`Frequency2024.Rmd`) → 원문 보존 검사 → 용어집 보정.
4. 전체 번역 → `terms build` → `glossary adopt --prefer HEAD` → 남은 세그먼트 번역 →
   `status --check`, `evaluate --mechanical-only` 0건.
5. 영어 원문 knit 확인 → 한국어 빌드와 검사 → 브라우저 확인.
6. README, Pages 배선.

## 되짚을 수 있는 결정

- **R 생성 글자.** 표 제목·축 레이블·tikz 글을 번역하려면 청크 본문의 문자열을 캡처하는
  규칙이 필요하다(파서 설계 3절). 표 제목(`TableGen1` 38, `kbl` 21)부터 따로 설계한다.
- **퀴즈.** SurveyJS JSON 문자열을 읽는 파서가 생기면 `Quizzes/QuizJavaScript/*.js`
  (25,164단어)를 번역 대상에 넣는다.
- **패키지 버전.** nixpkgs의 R 패키지 버전이 원문 렌더(bookdown 0.39 시절)와 다르므로
  수치 출력이 영어판과 조금 다를 수 있다. 원문이 버전을 고정하지 않아 맞출 기준이 없다.
- **CASdatasets 출처.** cas.uqam.ca가 1.2-0 tarball을 옮기면(새 버전은 `Archive/`로 가는
  관례가 없다) fetch가 실패한다. 그때는 해시가 같은 사본을 찾아 URL만 바꾼다.
- **파서 결함 우회.** 청크 복원·수식 줄 복원·참조 링크 복원은 `rmarkdown` 파서가 (1) HTML
  블록 안의 knitr 청크를 펜스로 보지 않고, (2) 디스플레이 수식 문단을 한 줄로 합치고, (3)
  인라인 링크 목적지 `(#tab:…)`을 보존 토큰으로 가리는 데서 온다. 파서가 고쳐지면 해당
  손질을 지우고 `reconcile`로 state를 옮긴다.
- **PDF.** 원문 PDF 빌드는 krantz 클래스와 pdflatex를 쓰므로 한국어에는 XeLaTeX·kotex
  설정이 따로 필요하다. 웹판이 안정된 뒤 다룬다.
