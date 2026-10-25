# 손해 데이터 분석(Loss Data Analytics) 2판 (한국어 번역)

[OpenActTexts/LDAVer2](https://github.com/OpenActTexts/LDAVer2)의 *Loss Data
Analytics*, 2판(편집 Edward W. Frees, 장별 저자, 보험계리 학계·업계 공동 집필)을
한국어로 옮긴 비공식 번역입니다. [yeokja](https://github.com/yeokja/yeokja)와 함께
Anthropic 사의 `claude-opus-5-5` 모델을 활용하여 번역되었으며 학습을 모두 비허용한
상태로 작업하였습니다.

## 범위

- 원문: `upstream/` 서브모듈, 커밋 `67f5aab0a70b95033249de583c862b0d8dcdd71a`
  (2026-04-03, 본문 `Chapters/`의 마지막 변경은 2025-02-02). 영어판:
  <https://openacttexts.github.io/LDAVer2/>
- 번역 대상(9,415 세그먼트):
  - bookdown gitbook이 읽는 R Markdown 29개(`_bookdown.yml`의 `rmd_files`에서 R 코드뿐인
    `References/Packages.Rmd`·`Glossary.Rmd`를 빼고 분포표 child 3개를 더함). 제목·
    문단·목록·표·각주, front matter의 제목·설명, 그림 캡션(`fig.cap`), 예제·코드 접기
    단추의 글(`HideExample('…', 'Show Example Solution')`의 두 번째 인자)을 옮깁니다
    (8,523 세그먼트).
  - 용어 풀이 사전 `References/GlossFct.csv`의 Definition 열(515).
  - 용어 풀이 표시어 사전 `gloss/GlossLabels.csv`의 Label 열(377, 아래 "용어 풀이").
- 원문 그대로 두는 것: R·tikz·JavaScript 코드, 수식, 교차 참조 레이블(`\@ref(…)`)과
  인용 키, 데이터셋·변수·패키지·함수 이름, 사람 이름, 참고 문헌(`.bib`).
- **영어로 남는 것.** R이 실행 중에 만드는 표 제목과 그래프의 축·범례, tikz 그림
  (지급준비금 장의 시간선·진전 삼각형) 속 글, 장마다 있는 퀴즈(`Quizzes/`, SurveyJS)의
  문항과 단추, 정적 그림 속 글자는 이번 판에서 번역하지 않습니다. 모두 코드 청크나
  JavaScript 안에 있어 번역 단위로 꺼낼 수 없습니다.
- PDF·EPUB 판은 만들지 않습니다(원문도 웹판이 주이며 PDF는 일부만 냅니다).
- 용어는 국내 보험계리·손해보험·통계학 교재의 관례를 따릅니다(`glossary.toml`,
  프로젝트 용어 기준 `terms.toml`).

### 용어 풀이(Gloss)

원문은 문장 속 용어를 `` `r Gloss('aggregate loss')` ``로 써서 녹색 풍선 도움말로
풀이를 보여 줍니다. 인자는 `GlossFct.csv`의 조회 키이자 화면에 보이는 글이라 번역할 수
없으므로, `scripts/gloss_labels.py extract`가 빌드 대상에 나오는 인자를 모아
`gloss/GlossLabels.csv`(`Term`은 키, `Label`은 번역할 표시어)를 만들고 yeokja가
`Label`만 번역합니다. 빌드할 때 `scripts/prepare_book.py`가 `Gloss()`를 고쳐 한국어
표시어를 보이고 풍선 도움말에 원어를 괄호로 붙이며, 표시어 뒤 조사(은/는, 이/가, 을/를,
과/와, 으로/로)를 받침에 맞춥니다. 원문을 올린 뒤에는 `gloss_labels.py extract`를 다시
돌립니다.

## 라이선스

원문 본문은 CC BY 4.0(서문의 라이선스 표기, `GettingStarted/LICENSE.md`)이고 저장소
`LICENSE`는 CC0 1.0입니다. 이 번역은 CC BY 4.0 개작물로 제공하며, 원 저작자는 *Loss
Data Analytics* 편집자 Edward W. Frees와 각 장의 저자들(서문의 기여자 목록), 원문은
<https://github.com/OpenActTexts/LDAVer2>입니다. 원문을 한국어로 번역한 비공식
번역본이며 저자들의 공식 번역이 아닙니다. 배포 사이트 루트에 원문 `LICENSE`를 함께
싣습니다. `Data/`의 데이터셋은 각자의 출처 표기를 따르며 원문과 같이 그대로 둡니다.

## 재현

```sh
cd projects/loss-data-analytics
python3 scripts/gloss_labels.py extract --check          # 표시어 사전이 원문과 맞는지
../../target/release/yeokja translate upstream            # Rmd와 GlossFct.csv
../../target/release/yeokja translate gloss               # Gloss 표시어
../../target/release/yeokja status --check upstream
../../target/release/yeokja status --check gloss
../../target/release/yeokja evaluate --mechanical-only upstream
nix develop path:../../nix#loss-data-analytics -c ../../target/release/yeokja build html   # dist/site
```

번역하려면 인증된 Claude Code CLI가 필요합니다. `state/`는 번역의 진실의 원천이므로
커밋합니다. `ko/`, `ko-gloss/`, `build/`, `dist/`는 `state/`와 원문에서 언제든
재생성되므로 커밋하지 않습니다.

Nix devShell(`nix/projects/loss-data-analytics.nix`)이 R과 원문이 쓰는 R 패키지
50개(CRAN에 없는 CASdatasets는 cas.uqam.ca 소스 tarball을 해시로 고정해 빌드),
pandoc, TeX Live(tikz), ImageMagick·Ghostscript·poppler, gifski, python3를 제공합니다.
빌드는 조립 트리(`build/tree`: 원문 + `ko/` + `ko-gloss/` + `overlay/`)에서 다음 순서로
진행합니다.

1. `scripts/gloss_labels.py extract --check`: 표시어 사전이 원문과 맞는지 확인합니다.
2. `scripts/prepare_book.py`: knitr를 돌리기 전에 조립 트리를 손질합니다.
   - 원문의 이식성 문제(Windows 파일 경로, 현재 VGAM에 없는 링크 이름 `"loge"`, 닫지 않은
     접기 상자 `<div>` 세 곳과 `<p>` 하나)를 고칩니다. 비교용 영어 원문 빌드에도
     `prepare_book.py --portability`로 똑같이 적용합니다.
   - Google Analytics 태그를 지우고, 서문의 날짜를 원문 커밋 날짜로 고정하고, 참고 문헌 장
     제목·저자 줄과 `Gloss()`를 한국어판에 맞춥니다.
3. `scripts/check.py source`: 손질한 Rmd의 코드 청크, 인라인 R, 디스플레이 수식,
   교차 참조 토큰이 원문과 같은지, 모든 `Gloss()` 인자에 표시어가 있는지 확인합니다.
4. `bookdown::render_book()`으로 gitbook을 만듭니다. `overlay/_bookdown.yml`이 한국어
   UI 문자열과 그림·표·식 레이블을, `overlay/_output.yml`이 목차 링크를,
   `overlay/Format/ko.css`가 한글 줄바꿈과 글꼴을 정합니다.
5. 기여자 사진과 정적 그림을 복사하고, `scripts/check.py site`로 번호 붙은 장 22개,
   로컬 링크와 앵커, knitr 오류 출력과 깨진 교차 참조를 확인합니다. 영어 원문 빌드에도
   있는 원문의 결함(없는 레이블 참조 등)은 따로 세고 실패로 보지 않습니다.

R 청크 930개를 모두 실행하지만 MCMC·애니메이션은 원문이 미리 계산해 둔 결과를 읽으므로
knit 자체는 몇 분이면 끝납니다. R 패키지 일부(`pdftools`, `sf`)는 devShell이 nixpkgs 빌드를
고쳐 쓰고, 바이너리 캐시에 없는 패키지는 처음 한 번 소스에서 빌드합니다.

Deploy Pages 워크플로는 커밋된 번역 상태로 `ko/`를 재구성하고 미번역 여부를 검사한 뒤
빌드해, `dist-loss-data-analytics` 산출물을 Pages의 `loss-data-analytics/`에
배치합니다.
