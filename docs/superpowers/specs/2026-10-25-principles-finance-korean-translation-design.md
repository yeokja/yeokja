# Principles of Finance 2e 한국어 번역 설계

## 목표

OpenStax(Rice University)가 펴낸 개방형 교재 *Principles of Finance 2e*(Julie
Dahlquist, Rainford Knight, CC BY-NC-SA 4.0, `github.com/openstax/osbooks-principles-finance`)를
yeokja로 한국어로 옮겨 GitHub Pages 웹 판(`principles-finance/`)으로 낸다. 파서는
`docs/superpowers/specs/2026-10-25-xml-vocabulary-profiles-design.md`의 `cnxml`이다.
한국어 제목은 "재무관리 원리"다.

완료 상태는 다음 조건을 모두 만족해야 한다.

- 원문이 `projects/principles-finance/upstream` 서브모듈로 커밋 `e3e5135a`에 고정되어 있다.
- 번역 대상 132개 문서(모듈 131개, 컬렉션 1개)의 모든 세그먼트가 번역 상태이고
  `yeokja status --check upstream`이 통과한다. `evaluate --mechanical-only`가 0건이다.
- 번역한 문서가 원문과 골격 동치다(어휘 프로필 설계 2.7).
- 자체 렌더러가 만든 사이트의 모든 내부 링크·앵커·이미지가 실재하고, 빈 상호 참조가
  없다.
- Pages 배포 배선, 랜딩 페이지 항목, README 출처 표기, 루트 `LICENSE.md` 항목이
  갖춰져 있다.

## 조사 결과

- **형식.** CNXML(`http://cnx.rice.edu/cnxml`) + MathML. `META-INF/books.xml` →
  `collections/principles-finance-2e.collection.xml`(collxml). 컬렉션은 머리말 모듈 하나와
  장(`col:subcollection`) 20개를 담는다. 모듈은 `modules/mNNNNN/index.cnxml` 132개이며
  `m00088`("Placeholder Text")은 컬렉션에 없다. 장마다 첫 모듈이 `class="introduction"`인
  도입부("Why It Matters")다. 그림 278개(`media/`, 30MB)는 모두 래스터다.
- **규모.** `cnxml` 파서로 세그먼트 15,230개(모듈 15,209 + 컬렉션 21). 저장소에서 가장
  큰 축에 든다.
- **빌드가 만드는 것.** 원문은 번호와 상호 참조 텍스트를 담지 않는다. 내용 없는
  `<link target-id document/>` 425개(대상: 그림 282, 표 143, 절 2), `<link document>` 37개,
  `note` 종류 라벨, 내용 없는 `<title/>`인 절 끝 묶음(요약, 객관식, 복습, 연습 문제,
  동영상 활동, CFA Institute)은 OpenStax 빌드(enki)가 채우고 장 끝 페이지로 모은다.
  도입부 그림(`class="splash"`)도 번호를 받는다(원문 사이트의 1.1절 첫 그림이 "Figure 1.2").
  수식은 상호 참조 대상이 없다.
- **연습문제.** `exercise` 644개 중 580개는 본문 없이 `<link class="os-embed"
  url="#exercise/FI_Ch1_Sec1_MCQ1"/>`만 담는다. 본문은 OpenStax Exercises API에 있고,
  API 응답에는 라이선스 필드가 없으며 풀이는 공개되지 않는다(`solutions_are_public:
  false`). 나머지 64개는 `think-through` 노트 안의 예제로 문제와 풀이가 원문에 있다.
- **공식 빌드.** enki는 Docker와 상용 Prince, Node·Java·Python·Ruby를 요구해 CI에서
  재현하기 어렵다.
- **원문 사이트 주소.** `openstax.org/books/principles-finance-2e/pages/` 아래 절은
  `{장}-{절}-{영어 제목 슬러그}`, 도입부는 `{장}-why-it-matters`, 머리말은 `preface`, 장 끝
  페이지는 `{장}-summary`·`key-terms`·`cfa-institute`·`multiple-choice`·`review-questions`·
  `problems`·`video-activity`다. 절 130개 주소를 모두 열어 확인했다.

## 범위

- 번역: 모듈 131개의 본문·제목·캡션·노트·각주·용어집, `media@alt`, `table@summary`,
  MathML `mtext`, 컬렉션의 책·장 제목(어휘 프로필 2.3이 정하는 범위).
- 번역하지 않음: MathML 식별자(`mi`), 엑셀 수식·R 코드(`code`), 계산기 키
  (`span.primary-function`/`secondary-function`), URL·ID, 그림 속 글자, `m00088`.
- **외부 연습문제 580개는 번역 범위에서 뺀다.** 렌더러가 연속한 os-embed 문제를 한
  상자로 묶어 "문제 N–M번은 원문 사이트의 연습문제 서비스가 제공하는 문제"라고 적고
  원문 장 끝 페이지로 링크한다. API 콘텐츠를 스냅샷으로 커밋해 번역하는 안은 라이선스
  근거가 없고(응답에 라이선스 필드가 없다), 풀이가 비공개라 문제만 옮기게 되며, 원문
  저장소 밖의 내용이라 서브모듈 고정으로 재현되지 않아 기각한다.

## 접근 방식

### 1. `yeokja.toml`

```toml
[[sources]]
path = "upstream/modules"
pattern = "*/index.cnxml"
exclude = ["m00088/index.cnxml"]
parser = "cnxml"
output = "ko/modules/{path}"

[[sources]]
path = "upstream/collections"
pattern = "*.collection.xml"
parser = "cnxml"
output = "ko/collections/{path}"

[provider]
type = "claude_code"
model = "claude-opus-5-5"

[derive]
base = "upstream"
[[derive.overlay]]
path = "ko"
require_base = true

[translation]
concurrency = 6
batch_segments = 32
```

`cnxml` 소스는 설정 없이 인라인 태그로 주고받는다. 기본 프롬프트(합쇼체)를 쓴다.
교재 본문에 맞고, 커스텀 템플릿을 두면 GFM 마커·`{inline_tags}` 규칙을 따로 챙겨야 한다.
세그먼트가 많아 장 단위(모듈 파일)로 나눠 번역하고 `status`로 진척을 본다.

### 2. 용어

`glossary.toml`은 국내 재무관리 교재(기업재무·투자론)의 관례를 따른다. 기업재무,
운전자본 관리, 자본예산, 자본구조, 화폐의 시간가치, 현재가치·미래가치, 순현재가치,
내부수익률, 가중평균자본비용, 타인자본비용·자기자본비용, 배당할인모형, 만기수익률,
표면이자율, 보통주·우선주, 재무상태표(대차대조표라고 쓰지 않는다), 손익계산서,
현금흐름표, 발생주의 회계, 자기자본이익률·총자산이익률, 매출채권·매입채무,
체계적·비체계적 위험, 자본자산가격결정모형, 효율적 시장 가설, 구매력평가 등 여러 단어로
된 전문 용어를 고정하고 약어(NPV, IRR, WACC, CAPM, ROE, GAAP …)는 그대로 둔다.

- 일상어와 겹치는 짧은 단어(*return*, *risk*, *bond*, *stock*, *option*, *dividend*,
  *beta*, *interest rate*)는 용어 검사 오탐으로 재시도를 부르므로 용어집에 두지 않고
  `terms.toml`에 맡긴다. *depreciation*도 회계(감가상각)와 환율(통화 가치 하락)에서 뜻이
  갈려 뺀다.
- 책 제목 *Principles of Finance*는 "재무관리 원리"로 고정한다.

첫 번역 뒤 `yeokja terms build`로 반복 제목("Learning Outcomes" 등)과 용어를 고정하고
`glossary adopt --prefer HEAD` 뒤 남은 세그먼트만 다시 번역한다.

### 3. 빌드: 자체 렌더러

enki를 재현하지 않고 Python 3 + lxml 렌더러 `scripts/build_site.py TREE OUT UPSTREAM`을
둔다. 조립 트리(원문 위에 `ko/`를 겹친 `build/tree`)의 번역된 컬렉션과 모듈을 읽고,
원문 영어 제목은 원문 사이트 주소를 만드는 데만 쓴다.

- **페이지.** 머리말, 장마다 도입부(`{n}-intro.html`, 장 제목과 "이 장의 구성"), 절
  (`{n}-{k}.html`), 장 끝 페이지(`{n}-summary.html`, `key-terms`, `cfa-institute`,
  `multiple-choice`, `review-questions`, `problems`, `video-activity`; 원문 사이트 순서)와
  랜딩 `index.html`(책 소개, 전체 목차). 모든 페이지에 이전·다음 탐색과 공통 하단.
- **번호.** 장은 컬렉션 순서, 절은 도입부를 뺀 장 안 순서. 그림·표는 장 안에서 문서
  순서로 번호를 매기고 `class`에 `unnumbered`가 있으면 건너뛴다. 머리말의 그림·표는 장
  번호 없이 센다. 수식에는 번호를 매기지 않는다(원문이 참조하지 않는다).
- **상호 참조.** 내용 없는 `link[target-id]`는 "그림 3.2"·"표 3.1"이나 대상 절 제목을,
  내용 없는 `link[document]`는 대상 페이지 제목을 쓴다. 대상을 찾지 못하면 빌드가
  실패한다. 장 끝 페이지로 옮긴 요소도 그 페이지로 연결된다.
- **노트 라벨.** `concepts-practice` "개념 적용", `excel-spreadsheet` "엑셀 활용",
  `link-to-learning` "더 알아보기", `think-through` "생각해 보기". 라벨이 없는 종류가
  나오면 빌드가 실패한다. `learning-objectives` 절은 상자로 보인다.
- **장 끝 묶음.** `section-summary` 등 여섯 종류의 절은 본문에서 빼고 장 끝 페이지에
  절 번호·제목 아래로 모은다. 용어집(`glossary`)은 "핵심 용어" 페이지에 번역어 순으로
  정렬한다. os-embed 문제 상자는 페이지 안에서 번호를 이어 매기고 원문 장 끝 페이지로
  링크한다. 동영상 활동의 `iframe`은 "동영상 보기" 링크(외부 사이트)로 바꾼다.
- **각주.** 페이지마다 번호를 매겨 본문에 위첨자 링크, 페이지 끝 "각주"에 되돌아가기
  링크를 둔다. 그림 제목 안의 각주(22개)도 같다.
- **수식.** MathML을 그대로 내보내고(`display="block"`은 `equation` 안), 수식이 있는
  페이지만 MathJax 3.2.2(`mml-chtml`)를 불러 MathML을 그리지 못하는 브라우저를 돕는다.
- **그 밖의 매핑.** `emphasis` italics/bold/underline/기본값 → `em`/`strong`/`u`/`strong`,
  `term` → `dfn`(`no-emphasis`는 강조 없음), 계산기 키 → `kbd`, 블록 `code`와
  `centered-code` → `pre`, `table`(`colspec` 이름으로 `colspan`, `morerows`로 `rowspan`,
  `summary`는 `aria-label`), 보기 문제의 `solution` → `details`("풀이").
  모르는 요소가 나오면 빌드가 실패한다.
- **이미지.** 본문이 참조하는 그림만 `media/`로 복사한다. 표지(`cover/`)는 쓰지 않는다.
- **CSS.** 읽기 폭 46rem, 16px 여백, 한국어 줄바꿈(`keep-all`), 넓은 표의 가로 스크롤,
  다크 모드.

PDF는 범위 밖이다.

### 4. 검사

`scripts/check.py`는 빌드 명령의 앞뒤에 놓인다.

- `skeleton UPSTREAM TREE`: 모듈 132개와 컬렉션의 골격 동치. 코어의 `skeleton_with`를 부르는
  CLI가 없어 `projects/technicalpm/scripts/check.py` 방식대로 같은 비교를 Python으로 한다
  (CNXML 블록 표, 건너뛰는 요소는 통째로 비교하되 `mtext` 텍스트는 제외, 번역 속성 값은
  지움, 강조는 있는지만). 강조를 있는지만 보면 모델이 `term`을 복제해도 통과하므로, 문서의
  요소 id 다중집합이 원문과 같은지도 본다.
- `site SITE`: 페이지마다 `h1` 하나, 내부 링크·이미지가 가리키는 파일과 `#앵커` 실재, 페이지
  안 id 중복 없음, 빈 상호 참조 없음.

빌드 툴체인은 `nix/projects/principles-finance.nix`의
`python3.withPackages (ps: [ ps.lxml ])`다.

## 라이선스

원문은 CC BY-NC-SA 4.0이다(`upstream/LICENSE`, 컬렉션 `md:license`). 번역본은 같은
라이선스의 비상업적 2차적 저작물로 낸다(`zero-to-nix` 선례).

- 모든 페이지 하단에 저작자(Julie Dahlquist, Rainford Knight, OpenStax/Rice University),
  그 페이지의 원문 링크, 라이선스 링크, 번역·수정 사실(연습문제 서비스 문제를 링크로
  바꿈)과 공식 번역이 아니고 보증받지 않았다는 고지, 번역 고지를 넣는다.
- OpenStax 로고·표지 이미지(`cover/`)와 상표를 제호로 쓰지 않는다. 저작자 표시에 이름만
  적는다.
- 연습문제 서비스의 문제는 싣지 않는다(범위 절).
- README는 AGENTS.md 출처 표기 규칙을 따르고, 루트 `LICENSE.md` 표에 한 줄을 더한다.

## 배포

`.github/pages-projects.json`에 `project = "principles-finance"`, `target = "html"`을
더하고, `stage-pages.sh`에 `site` → `principles-finance` 스테이징을 더한다. CI는
`status --check upstream` 뒤 `translate upstream`으로 `ko/`를 재구성한다(provider를 부르지
않는다). 랜딩 페이지의 "경제·금융" 주제(`finance`)에 카드를 둔다.

## 작업 순서

1. 골격: `yeokja.toml`, `glossary.toml`, `.gitignore`, nix devShell.
2. 시험 번역(`m00002`, 1.1절)으로 재시도율·문체·용어를 본다.
3. 모듈 단위 전체 번역 → `evaluate --mechanical-only` → `terms build` → `glossary adopt` →
   남은 세그먼트 번역 → `status --check`.
4. `build_site.py`, `check.py`, 빌드와 브라우저 확인.
5. README, 배포 배선(호출 측 공유 파일).

## 되짚을 수 있는 결정

- **자체 렌더러.** 원문 사이트와 모양이 다르다(장 끝 페이지 순서·노트 라벨은 원문을
  따르지만 디자인은 단순하다). OpenStax가 공개 빌드를 컨테이너로 내놓으면 다시 본다.
- **외부 연습문제 제외.** Exercises API 콘텐츠의 라이선스가 명시되면 스냅샷을 커밋해
  번역하는 안을 다시 본다.
- **수식 안의 영어.** `mi`로 쓴 단어(`Revenue`, `Assets`)는 식별자로 보고 그대로 둔다.
- **문장 분할기의 약어**(`Ben S. Bernanke`)로 각주 출처가 이상하게 나뉜 세그먼트는
  번역이 이웃 내용을 겹쳐 쓰지 않았는지 따로 살핀다.
- **그림 속 영어**(엑셀·재무제표 캡처 117개 포함)는 번역하지 않는다.
