# Jeff Erickson *Algorithms* 한국어 번역 설계

이슈: yeokja/yeokja#3

## 목표

Jeff Erickson의 교재 *Algorithms*(1st edition, 2019,
<https://jeffe.cs.illinois.edu/teaching/algorithms/>)를 yeokja로 한국어 번역해
한국어 PDF를 GitHub Pages로 배포한다.

이 책은 다른 프로젝트와 달리 **편집 가능한 원고가 공개되어 있지 않다.** 공개된
것은 pdfLaTeX로 만든 PDF뿐이다. yeokja는 원고 마크업을 번역하는 도구라서 PDF를
직접 읽지 못한다. 그래서 먼저 PDF에서 영어 LaTeX 원고를 복원하고 검증한 뒤,
그 원고를 기존 `latex` 파서로 번역한다.

완료 상태는 다음 조건을 모두 만족해야 한다.

- 원서 PDF를 담은 `jeffgerickson/algorithms` 저장소의 고정 커밋이 서브모듈로
  등록되어 있다.
- 범위 안의 모든 장이 영어 LaTeX 원고(`source/`)로 복원되어 있다. 그 원고를
  컴파일한 PDF의 텍스트가 원서 텍스트 레이어와 장별로 검증 기준을 만족한다
  (아래 "원고 검증").
- 복원된 원고의 모든 대상 세그먼트가 번역 상태이고 `yeokja status --check`가
  통과한다.
- 한국어 PDF가 LuaLaTeX로 오류 없이 빌드된다. 정의되지 않은 참조(`??`)와
  누락 글리프 경고가 없다.
- Pages 배포 배선, 랜딩 항목, README 출처 표기가 갖춰져 있다.

## 원문 조사 결과 (2026-09-20)

- **원고:** LaTeX 원고와 그림 원본(OmniGraffle)은 공개되어 있지 않다. 원고
  공개를 요청한 저자 저장소 이슈 #248(2021)은 답이 없고, 저자의 마지막 이슈
  활동은 2019년이다. 일본어판(inzkyk, CC BY 4.0)도 원고를 받지 않고 새로
  조판한 것으로 보인다.
- **라이선스:** 교재는 CC BY 4.0이다(저장소 README: "Everything on this site is
  available under a Creative Commons Attribution 4.0 International License").
  서문의 "Steal This Book!" 절에서 저자는 출처를 밝히면 허락 없이 사용·재배포·
  개작해도 된다고 적었고, 파생물을 웹에 공개해 달라고 했다. 강의 노트(Extended
  Dance Remix, Director's Cut, Models of Computation)는 CC BY-NC-SA 4.0이다.
- **PDF:** 472쪽이다(앞부분 18쪽, 본문 13개 장 428쪽, 뒷부분 26쪽). 실제
  텍스트 레이어가 있다. 본문 약 16.5만 단어, 수식이 문자의 약 4.9%다.
  내부 하이퍼링크 2,277개와 목차 212개 항목이 있다. 캡션이 달린 그림 198개,
  임베드된 벡터 그림 211개, 연습문제 561개, 각주 210개, 이름 붙은 의사코드
  절차 약 136개가 있다. 2019-06-15 이후 바뀌지 않았다.
- **변환 도구:** `pdftotext`, `pymupdf4llm`, `docling`은 산문은 쓸 만하지만
  수식과 의사코드가 망가진다(인라인 수식 손실, 의사코드가 한 줄로 합쳐짐).
  `marker`, `nougat`은 nixpkgs에 없다.
- **추출 가능성:** 벡터 그림은 PyMuPDF로 독립 PDF로 잘라낼 수 있다. 영어
  단어가 든 그림은 약 25개뿐이다.

## 범위

**포함(CC BY 4.0 교재):**

- 앞부분: 서문(Preface)과 목차. 목차는 LaTeX가 생성한다.
- 본문: 0~12장, 연습문제와 각주 포함.
- 뒷부분: Image Credits, Colophon.

**제외:**

- 색인 3종(Index, Index of People, Index of Pseudocode). 쪽 번호가 바뀌므로
  `\index` 재태깅이 필요하다. 후속 작업으로 둔다.
- CC BY-NC-SA 강의 노트 전부. 라이선스가 달라 별도 프로젝트 대상이다.
- 그림 안의 영어 문구(약 25개 그림)는 원서 그대로 둔다. 캡션은 LaTeX 텍스트라
  번역한다.

**그림 권리 처리:**

- 그림 1.25는 "작가의 허락을 받아 수록"된 초상화라 CC BY 범위 밖이다. 이미지를
  싣지 않고, 같은 자리에 원서 해당 쪽을 안내하는 문구를 둔다.
- 그림 5.2(Yale Law Library, Creative Commons)는 Flickr의 실제 라이선스를
  확인한다. 수정 없이 싣고, 출처와 라이선스를 Image Credits에 명시한다.
- 나머지 비저자 그림은 모두 퍼블릭 도메인이다(Image Credits 기준).

**번역 라이선스:** 번역본은 CC BY 4.0으로 배포한다. 다음 내용을 판권 면과
README에 적는다.

- © 2019 Jeff Erickson, 원서 링크(<http://algorithms.wtf>).
- 변경 사항: PDF에서 원고를 복원했고, 번역했고, 색인을 제외했고, 그림 1.25를
  제외했다.
- 저자가 승인하지 않은 비공식 번역이라는 점.
- 영어 원고 복원과 번역에 쓴 AI 모델. 번역 모델은 AGENTS.md 규칙을 따른다.

## 접근 방식

### 채택: LLM 기반 LaTeX 원고 복원 → 텍스트 대조 검증 → yeokja `latex` 번역

1. 원서 PDF에서 페이지별 자료를 추출한다: 페이지 이미지(PNG), 글꼴로 분류한
   텍스트 스팬(본문/수식/코드/제목/그림 내부), 링크 대상.
2. 장 단위로 LLM(Claude Code 서브에이전트)이 이 자료를 보고 정해진 매크로
   어휘로 LaTeX 원고를 쓴다. 그림은 벡터 PDF 잘라내기로 붙인다.
3. 복원 원고를 컴파일해 텍스트를 뽑고, 원서 텍스트 레이어와 단어 단위로
   대조한다. 기준을 넘는 차이는 모두 고친다.
4. 검증된 영어 원고를 yeokja `latex` 파서로 번역하고, 같은 프리앰블의 한국어
   모드로 LuaLaTeX PDF를 만든다.

수식과 의사코드가 망가지지 않는 유일한 경로다. 텍스트 대조로 빠뜨리거나 지어낸
문장을 기계적으로 잡을 수 있다. 번역 단계는 napkin·hott 같은 기존 LaTeX 책
프로젝트와 같은 방식이다.

### 제외: 기성 변환기(docling, pymupdf4llm)의 결과를 원고로 사용

수식·의사코드 품질이 낮아 수작업 교정량이 LLM 복원보다 크다.

### 제외: LLM이 PDF에서 한국어를 바로 생성

yeokja의 `state/`(진실의 원천), 증분 번역, 기계 평가를 쓸 수 없다. 영어 원고
검증 단계도 없어져 누락을 잡을 방법이 사라진다.

### 병행(사용자 판단): 저자에게 원고 요청

저자는 서문에서 파생물이 생기면 알려 달라고 했다. 사용자가 원하면 번역 계획을
알리고 원고를 요청하는 메일을 보낼 수 있다(`jeffe@illinois.edu`). 외부 연락은
사용자가 직접 하며, 이 설계는 그 응답을 기다리지 않는다. 나중에 진짜 원고를
받으면 세그먼트가 달라 재번역이 필요하다. 그 판단은 그때 한다.

## 디렉터리와 책임

```text
projects/jeffe-algorithms/
├── upstream/        jeffgerickson/algorithms 서브모듈(원서 PDF, 읽기 전용, shallow)
├── source/          복원한 영어 LaTeX 원고(커밋, CC BY 4.0 파생물)
│   ├── main.tex         문서 클래스, 장 포함, 영어/한국어 모드 전환
│   ├── preamble.tex     글꼴·레이아웃·매크로 어휘
│   ├── frontmatter/     preface.tex
│   ├── chapters/        00-intro.tex … 12-nphard.tex
│   ├── backmatter/      credits.tex, colophon.tex
│   └── figures/         원서에서 잘라낸 벡터 PDF와 퍼블릭 도메인 래스터 이미지
├── scripts/         추출·그림 잘라내기·원고 검증·PDF 준비 스크립트와 단위 테스트
├── RECONSTRUCTION.md  복원 규칙(매크로 어휘, 라벨 규칙, 검증 절차)
├── state/           번역 상태(커밋)
├── ko/              번역된 원고(재생성, 커밋하지 않음)
├── build/           추출 자료·검증 빌드·조립 트리(커밋하지 않음)
├── output/pdf/      한국어 PDF(커밋하지 않음)
├── glossary.toml
├── yeokja.toml
└── README.md
```

## 원문 고정

- `projects/jeffe-algorithms/upstream`을
  `https://github.com/jeffgerickson/algorithms.git`, `shallow = true`로 등록한다.
- 이 저장소의 마지막 push는 2019-11-23이다. 기본 브랜치 HEAD
  `9d4f235ac54e094594e7db6332e20489ba8e830b`(커밋 날짜 2019-07-01)에 고정하고
  README에 적는다. 기준 PDF의 SHA-256은
  `00295b82ff725c52a71351a0bb1ee1bdb1584e594a62395d3fb382aefd0f4d42`이다.
- 기준 원서는 최상위 `Algorithms-JeffE.pdf`다(최신 개정, 강의 사이트 사본과
  바이트 단위로 같음). 복원 원고의 모든 쪽 번호 인용은 이 파일 기준이다.

## 추출 (`scripts/extract.py`, `scripts/extract_figures.py`)

- `extract.py upstream/Algorithms-JeffE.pdf build/extract`는 쪽마다 다음을
  만든다(재생성 가능, 커밋하지 않음).
  - `pNNN.png`(150dpi).
  - `pNNN.json`: 스팬별 텍스트, 글꼴 이름, 크기, bbox, 분류. 분류값은 `text`,
    `math`, `code`, `heading`, `figure`, `header_footer`이다. 글꼴 이름 규칙은
    XCharter=본문, MathDesign-Charter·CharterBT·stmary·Dingbats=수식,
    Inconsolata=코드, Roboto=제목이다. 그림 영역 안의 스팬은 `figure`로
    분류한다.
  - 쪽의 내부 링크 대상(쪽 번호와 목적지 이름).
  - 장별 쪽 범위: 목차 outline에서 계산해 `build/extract/chapters.json`에
    둔다.
- 알려진 글리프 문제는 추출 단계에서 표지로 바꾼다.
  - stmaryrd 화살표(`\x01`) → `⟨ARC⟩`. 글꼴을 보고 바꾼다. MathDesign-Ex도
    큰 괄호에 `\x01`을 쓰기 때문이다.
  - 연습문제 난이도 기호는 Dingbats의 `n/o/p/m`(♥/♦/♣/♠)이다.
  - 합자와 구식 숫자 각주는 정규화한다.
- `extract_figures.py`는 벡터 그림(폼 XObject)을 그 객체만 독립 PDF로 잘라
  `source/figures/`에 `pNNN-<이름>.pdf`로 저장한다. 쪽을 잘라 붙이면 쪽 전체
  글꼴이 따라 들어와 그림 하나가 약 300KB가 되므로 쓰지 않는다. 래스터
  이미지는 원본 해상도로 뽑는다. 결과는 벡터 203개와 래스터 9개(22MB)다.
  그림 1.25는 제외한다.

## 원고 형식과 매크로 어휘

- 문서 클래스는 원서와 같은 `memoir`를 쓴다. 원서 모양(Charter 본문, Roboto
  제목, Inconsolata 코드, 장 제목 스타일)을 가깝게 따르되, 픽셀 단위 재현은
  목표가 아니다.
- **엔진:** 영어·한국어 모두 LuaLaTeX다.
  - 공통: `fontspec`으로 XCharter(본문), Roboto(제목), Inconsolata(코드).
    수식은 `unicode-math` + `XCharter-Math`.
  - 한국어 모드: `luatexko`와 나눔명조/나눔고딕을 더한다.
  - `main.tex` 첫 줄의 `\newif\ifkorean`로 전환한다. 한국어 빌드는 준비
    스크립트가 `\koreantrue`를 넣는다.
- `preamble.tex`에 고정 매크로 어휘를 정의한다. 복원 에이전트는 이 밖의
  매크로를 새로 만들지 않는다. 꼭 필요하면 `RECONSTRUCTION.md`와
  `preamble.tex`에 먼저 추가한다.
  - `\arc{u}{v}`: 방향 간선 u→v.
  - 식별자: `\Proc{Name}`(절차 이름, 작은 대문자), `\Const{True}`(상수, 작은
    대문자), `\Var{name}`(여러 글자 변수, 이탤릭). 식별자에 `\textit`나
    `\textsc`를 쓰지 않는다. 파서가 그 인자를 번역하기 때문이다.
  - `pseudocode` 환경: 박스 의사코드. `\Header`(밑줄 친 절차 머리),
    `\Indent`(2em), `\Comment{…}`(주석)를 쓴다. 파서는 이 환경을 불투명하게
    다루고 `\Comment` 인자만 번역 대상으로 제시한다. 표준 `algorithm` 플로트와
    이름이 겹치지 않게 `pseudocode`로 정했다.
  - `\Str`, `\StrMark`, `\StrMarkAlt`: 코드 문자열과 표시된 글자. `\Hint{…}`:
    "[Hint: …]".
  - `problembox` 환경: 테두리 친 문제 설명. `\Segments`, `\Decisions`: 문자열
    분할·결정 순서 그림. `\[ … \]` 안에서만 쓰며, 이 부분은 번역하지 않는다.
  - `exercises` 환경과 `\difficulty{…}`: 난이도 표시는 글자 코드로 쓴다(h/H =
    작은/큰 하트, d = 다이아몬드, c = 클럽, s = 스페이드).
  - `\figref`, `\secref`, `\chapref`: 원서와 같은 참조 표기. `\omitfigure{…}`:
    싣지 않는 그림 자리.
  - 장 첫머리 인용구는 `\chapter` 앞에 `\epigraph{…}{…}`로 쓴다.
  - 굵은 수식은 `\symbfit`, 텍스트 글꼴 첨자는 `\textup{…}`로 쓴다.
- **판형:** 원서와 같게 memoir 10pt, 글꼴 배율 1.0473, `\linespread{1.125}`,
  `microtype`을 쓴다.
- **라벨:** `\label{fig:<장>.<번호>}`, `\label{sec:<장>.<번호>}`,
  `\label{ex:<장>.<번호>}` 형식이다. 원서의 번호 체계를 그대로 재현해,
  컴파일 후 그림·절·연습문제 번호가 원서와 같아야 한다.
- 원서의 하이퍼링크 목적지(2,277개)로 `\ref` 대상을 확인한다.

## 원고 복원 절차

- 장 하나를 서브에이전트 하나가 맡는다. 입력은 해당 쪽의 PNG와 JSON,
  `RECONSTRUCTION.md`, `preamble.tex`이고, 출력은 `source/chapters/NN-*.tex`다.
- 에이전트는 페이지 이미지를 기준으로 구조(절, 목록, 의사코드, 수식 정렬)를
  잡는다. 단어는 JSON 텍스트를 그대로 옮기고 새로 타이핑하지 않는다.
- 장마다 `verify_source.py`를 통과해야 완료다.
- **파일럿:** 2장(Backtracking, 26쪽)을 먼저 복원·검증·번역·빌드해 매크로
  어휘, 검증 기준, 파서 동작을 확정한다. 이 장은 의사코드, 그림, 연습문제를 두루
  포함한다. 파일럿에서 어휘나 기준이 바뀌면 `RECONSTRUCTION.md`와 이 spec을
  함께 고친 뒤 나머지 장으로 넘어간다.

## 원고 검증 (`scripts/verify_source.py`)

`verify_source.py <chapter>`는 영어 모드로 해당 장만 컴파일한다(`\includeonly`).
그다음 PyMuPDF로 텍스트를 뽑아 원서의 같은 장 텍스트와 비교한다.

- **정규화:** 머리말·꼬리말과 쪽 번호를 제거하고, 하이픈 줄바꿈을 잇고, 합자와
  유니코드를 NFKC로 맞추고, 공백을 정리한다. 그림 내부 텍스트는 양쪽에서
  제외한다(그림은 원서 PDF를 잘라 쓰므로 같다).
- **비교:** 본문(`text`, `heading`, `code`) 단어열을 `difflib.SequenceMatcher`로
  맞춘다. 원서 단어 중 일치하지 않은 비율이 **0.5% 이하**여야 한다. 일치하지
  않은 구간은 전부 목록으로 출력하고, 구간마다 원고 오류인지 추출 잡음인지
  사람이나 에이전트가 판정한다. 원고 오류는 모두 고친다.
- **정규화 세부(파일럿에서 확정):**
  - 구두점은 따로 떼어 토큰으로 센다. 하이픈과 따옴표 모양은 통일한다.
  - 띄어쓰기만 다른 경우는 일치로 본다. 원서 텍스트 레이어는 ff 합자 뒤를
    "offthe"처럼 붙여 쓴다.
  - Roboto 작은 대문자는 원서에서 대문자로 추출되므로 일치로 본다.
  - 4토큰 이상의 블록이 위치만 옮겨진 경우(플로트, 각주)는 일치로 세고 따로
    나열한다. 순서를 무시한 뒤 남는 차이(`residual after ignoring order`)는
    비어 있어야 한다.
- **수식:** 수식 스팬의 기호열(공백 제거)을 따로 비교해 불일치 구간을
  출력한다. 큰 괄호와 첨자 추출 순서 차이만 남아야 한다. 수식은 원서와 복원본
  쪽을 나란히 렌더링한 이미지(`scripts/sidebyside.py`)로 모든 쪽을 눈으로
  확인한다. 파일럿(2장)의 결과는 수식 기호열 불일치 0.52%, 내용 오류 0건이었다.
- **구조 개수:** PDF끼리 자동으로 비교하며, 다음이 원서와 같아야 한다.
  - 캡션 라벨(그림 수).
  - `Hfootnote` 링크(각주 수).
  - 연습문제 번호.
  - outline(절 제목 목록).
  - Index of Pseudocode의 해당 장 절차 집합.
- 파일럿(2장) 결과: 단어 불일치 0.1238%(9,694 대 9,696 토큰), 남은 8개
  구간은 모두 각주 재배치였다. 구조는 모두 일치했다(그림 6, 각주 14,
  연습문제 6, outline 16, 절차 8).
- 결과는 `build/verify/<chapter>.txt`에 쓰고, 기준을 넘으면 종료 코드 1이다.
- 정규화·비교 함수는 `scripts/test_verify_source.py`로 단위 테스트한다.

## 번역 설정

```toml
[[sources]]
path = "source"
pattern = "**/*.tex"
parser = "latex"
output = "ko/{path}"
```

- `preamble.tex`와 `main.tex`에는 번역할 산문이 없다. 파서가 세그먼트를
  제시하면 `[[sources]]` 패턴을 `frontmatter/**`, `chapters/**`, `backmatter/**`
  세 규칙으로 나눠 제외한다.
- **파서 확장(파일럿에서 필요 확인):** 기존 파서는 의사코드 블록 전체를 산문
  세그먼트 하나로 제시해 키워드까지 번역될 위험이 있었다. 그래서
  `crates/parser-latex`에서 `pseudocode` 환경을 불투명 환경으로, `Comment`를
  가시 텍스트 명령으로 추가했다(테스트 `pseudocode_is_opaque_except_its_comments`,
  별도 `[*]` 커밋). napkin, hott, chisel-book은 두 이름을 쓰지 않아 세그먼트가
  바뀌지 않는다. `\Proc`, `\arc` 같은 인라인 명령은 그대로 보존된다.
- `[provider]`: `claude_code` / `claude-sonnet-5`(다른 활성 프로젝트와 같음).
  커스텀 프롬프트는 없다.
- `[evaluation]`: `auto_evaluate = true`, `max_retries = 3`.
- `[translation]`: `concurrency = 8`.
- **의사코드 표기:** 키워드(if, for, return)와 절차 이름은 영어로 둔다. 주석만
  번역한다. 원서와 일본어판의 관례다.

### 용어집

알고리즘 교재 표준 용어로 시작한다.

- **기본 대역:** recursion → 재귀, backtracking → 백트래킹, dynamic
  programming → 동적 계획법, greedy algorithm → 탐욕 알고리즘, graph →
  그래프, vertex → 정점, edge → 간선, depth-first search → 깊이 우선 탐색,
  breadth-first search → 너비 우선 탐색, minimum spanning tree → 최소 신장
  트리, shortest path → 최단 경로, maximum flow → 최대 흐름, minimum cut →
  최소 컷, NP-hard → NP-난해, reduction → 환원, running time → 실행 시간,
  recurrence → 점화식, subproblem → 부분 문제, memoization → 메모이제이션,
  induction → 귀납법, invariant → 불변식.
- **영문 유지:** 사람 이름과 알고리즘 이름에 붙은 고유명사(Dijkstra,
  Bellman-Ford, Ford-Fulkerson 등)는 원어로 둔다.

## 한국어 PDF 빌드 (`[build.pdf]`)

- 한국어판은 `-usepretex='\def\KoreanEdition{1}'`로 켠다. 조립 트리에서
  `scripts/prepare_pdf.py`가 트리를 검사하고, 모델이 쓴 ASCII `"…"`를
  `“…”`로 바꾼다(링크를 교체하며 `ko/`와 `source/`는 건드리지 않음).
- `latexmk -lualatex -e '$max_repeat=10' main.tex`를 실행한다. 한국어판은
  참조가 안정되기까지 기본값 5회보다 많은 반복이 필요하다. 원서처럼 목차,
  그림, 하이퍼링크를 포함한다.
- 나눔 글꼴에는 이탤릭과 작은 대문자가 없으므로 한글 이탤릭은 가짜 기울임
  (fake slant)으로 조판한다.
- `source/luaotfload.conf`는 LuaTeX 글꼴 색인을 TeX Live와 `$OSFONTDIR`로
  제한하고, devShell은 프로젝트 전용 `TEXMFVAR`를 쓴다. 이렇게 하지 않으면
  사용자 글꼴 폴더의 거대한 TTC를 색인하느라 첫 실행에 40분 넘게 걸린다.
- 결과를 `output/pdf/Algorithms-ko.pdf`로 복사한다.
- 로그를 검사해 다음이 있으면 실패한다: `LaTeX Warning: Reference`,
  `Citation`, `There were undefined references`, `Missing character`.
- nix devShell `jeffe-algorithms`의 구성:
  - `texlive.combine`: `scheme-medium`, `collection-luatex`,
    `collection-langkorean`, `collection-fontsrecommended`,
    `collection-fontsextra`(XCharter, Roboto, Inconsolata), `collection-latexextra`,
    `collection-mathscience`, `latexmk`.
  - `pkgs.nanum`, `pkgs.poppler-utils`.
  - `python3.withPackages (ps: [ ps.pymupdf ])`.
  - 글꼴 설정은 hott의 devShell 패턴(`FONTCONFIG_FILE`, `OSFONTDIR`)을 따른다.

## 배포 배선

- `.github/pages-projects.json`: `{"project": "jeffe-algorithms", "target": "pdf",
  "artifact": "dist-jeffe-algorithms-pdf", "artifact_path":
  "projects/jeffe-algorithms/output/pdf"}`.
- `stage-pages.sh`: hott의 PDF 배치를 따라 `jeffe-algorithms/Algorithms-ko.pdf`로
  싣는다.
- `rebuild-translations.sh`: `jeffe-algorithms)` 분기에서 `translate source`와
  `status --check source`를 실행한다.
- `site/index.html`: 새 주제 "알고리즘"(`data-topic="algorithms"`) 섹션과 필터
  버튼을 추가하고 이 책을 싣는다. cp-algorithms(#2)도 같은 주제에 들어간다.
- `pr.yml`: `projects/jeffe-algorithms/scripts`의 단위 테스트 스텝을 추가한다.

## README

다음을 적는다.

- 원서, 저자, 라이선스(CC BY 4.0), 고정 커밋, 기준 PDF.
- 범위와 제외 항목(색인, 강의 노트, 그림 1.25).
- 원고 복원 방식과 검증 기준. 영어 원고가 저자 원고가 아니라 PDF에서 복원한
  파생 원고라는 점을 분명히 한다.
- 재현 명령: 추출, 검증, 번역, 빌드.
- AGENTS.md 출처 표기: [yeokja](https://github.com/yeokja/yeokja) 링크, 실제
  번역 모델, 학습 비허용 문장. 영어 원고 복원에 쓴 모델도 함께 밝힌다.
- 비공식 번역이며 저자의 승인을 받지 않았다는 고지.

## 위험과 대응

- **복원 오류(수식·의사코드):** 단어 대조, 수식 기호열 대조, 구조 개수 검사로
  잡는다. 수식 밀도가 10% 이상인 74쪽은 페이지 이미지와 컴파일 결과를 나란히
  두고 추가로 확인한다.
- **분량:** 428쪽 복원과 약 16.5만 단어 번역이 필요하다. 장별 서브에이전트로
  병렬화하되, 번역 provider 부하를 고려해 번역은 다른 프로젝트와 겹치지 않게
  돌린다.
- **파서가 의사코드 주석을 다루지 못함:** 파일럿에서 확인하고, 필요하면 작은
  파서 확장을 한다.
- **저자 반응:** CC BY로 허용된 작업이다. 그래도 AI 복원·번역 사실을 README와
  판권 면에 투명하게 밝힌다.
- **나중에 진짜 원고가 공개될 가능성:** 재번역이 필요하다. `state/`의 번역문은
  참고 자료로 재활용할 수 있다.
