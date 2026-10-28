# Algorithmica(러시아어판) 한국어 번역 설계

이슈: yeokja/yeokja#10

## 목표

[`algorithmica-org/algorithmica`](https://github.com/algorithmica-org/algorithmica)의
러시아어판 *Алгоритмика*(<https://ru.algorithmica.org>, Sergey Slotin과 Tinkoff
Generation의 CS 강의 자료, CC BY-SA 4.0)를 고정된 원문으로 삼아 yeokja로
러시아어 → 한국어로 번역하고, 한국어판 제목 **알고리즘카**로 원문과 같은 Hugo
테마의 사이트를 GitHub Pages `algorithmica/`에 배포한다.

완료 상태는 다음 조건을 모두 만족해야 한다.

- 원문 저장소가 `projects/algorithmica/upstream` 서브모듈로
  `ed1945c3d2de8548b1c744d3161eb668703db808`에 고정되어 있다.
- 번역 범위(아래)의 모든 세그먼트가 번역 상태이고
  `yeokja status --check upstream/content/russian`이 통과한다. 이 검사의 대상이
  정확히 번역 범위와 같다(`scripts/scope.py check`).
- `yeokja evaluate --mechanical-only upstream/content/russian`에 남은 항목이 모두
  확인된 오탐이다(검사 절 참고).
- 한국어 Hugo 빌드가 오류 없이 끝나고, 같은 Hugo로 빌드한 러시아어 원문과 비교해
  쪽 목록이 같고, 원문에서 멀쩡하던 로컬 링크·그림·앵커가 깨지지 않으며, 쪽마다
  본문 제목·그림·코드 블록 개수와 수식의 종류가 같다(`scripts/check_site.py`).
- 산출물에 방문 통계 스크립트(Google Analytics, Yandex.Metrica)가 없다.
- Pages 배포 배선, 랜딩 페이지 항목, `LICENSE.md` 표, README 출처 표기가 갖춰져
  있다.

## 조사 결과

- **저장소.** Hugo 사이트 하나에 두 언어판이 있다. `content/english`(영어판, 현재
  *Algorithms for Modern Hardware* 한 권)와 `content/russian`(러시아어판 강의 자료).
  `themes/algorithmica`(MIT)와 슬라이드용 `themes/reveal-hugo` 서브모듈. 원문
  Netlify 빌드는 Hugo 0.96.0 extended. 2022-12 이후 휴면이다.
- **라이선스.** 저장소에 LICENSE 파일이 없다. 러시아어판의 사이트 하단 문구
  (`themes/algorithmica/i18n/ru.toml`의 `copyright2`, 커밋 25e9ba5 "changing
  license to share-alike")가 "자료는 CC BY-SA로 배포한다"고 밝힌다. 영어판
  (`i18n/en.toml`)은 CC BY-NC 문구가 주석 처리되어 있어 이용 허락이 없고, 저자는
  영어판 책의 전자책·인쇄판 판매 계획을 밝혔다. 테마는 MIT(`themes/algorithmica/LICENSE`).
  저자는 그림 일부의 출처를 모른다고 적었다("I stole a lot of those").
- **분량.** `content/russian`의 Markdown 386개 가운데 `draft: true`가 111개다.
  초안이 아닌 문서는 cs 152, math 53, ml 37, dl 31, 루트 `_index.md`·
  `contributing.md`로 275개. 그중 `dl/aux/`(3개)는 reveal-hugo 슬라이드
  묶음(`outputs: [Reveal]`)과 그 섹션 머리말이다. 남은 272개가 mkdocs 파서로
  세그먼트 7,718개(코드·수식·주석 제외, coverage 59%)다.
- **front matter.** `title` 366, `weight` 224, `authors` 120, `draft` 111, `date` 38
  (그중 2개는 `date: {}`), `prerequisites` 34, `published` 23, `created` 23,
  `editors` 12, `part` 8, `menuTitle` 7, `hideSidebar` 4. front matter가 없는 옛
  위키 문서가 19개(math, 파일 이름이 키릴 문자) 있고 원문 사이트에서도 제목 없이
  나온다. 약 74개 파일이 `*.ru.md`처럼 언어 접미사를 단다.
- **초안 섹션.** `math/`, `ml/`, `dl/`의 `_index.md`가 초안이다. Hugo는 그 섹션
  쪽을 만들지 않지만 아래 문서는 내놓으므로, 원문 사이트에서 이 세 분야의 글은 첫
  화면 목차에 없고 검색·직접 링크로만 닿는다.
- **문법.** Goldmark(unsafe HTML, 각주 끔). shortcode 없음. 테마의
  `partials/content.html`이 본문을 `$$`로 나누어 수식 블록을 Markdown 렌더링에서
  빼고, KaTeX auto-render가 `$`/`$$`를 그린다. `@@`(TikZ) 블록은 번역 범위에 없다.
  GFM 표 4곳, 그 안의 `$`는 칸 안에서 짝이 맞는다. coverage에서 건너뛴 긴 구간은
  모두 코드·수식·HTML 주석이다.
- **앵커.** 범위 안의 문서끼리 제목 앵커로 잇는 링크는
  `math/algebra/linear-algebra/linalg.ru.md`의 `bitset#Гаусс` 하나뿐이다.
- **Hugo 버전.** nixpkgs의 Hugo 0.165.0 extended로 원문이 빌드된다. 조건은
  `published:` 줄과 `date: {}` 줄 삭제(날짜로 읽다 실패)뿐이다.
- **방문 통계.** `partials/analytics.html`에 Google Analytics와 Yandex.Metrica가
  있다. `head.html`은 쓰지 않는 TikZJax와 lunr(영어·러시아어 형태소)를 싣는다.
- **시험 번역.** `cs/range-queries/sparse-table.md`(56세그먼트, 수식·코드·링크·
  제목이 고루 있음)를 기본 프롬프트로 번역했다. 재시도 0, 합쇼체, 용어집
  (разреженная таблица → 희소 테이블) 적용, 수식·인라인 코드·링크 보존이 의도대로였고,
  отрезок은 문맥대로 "구간"으로 옮겼다.

## 범위

- 번역: `content/russian`에서 초안과 `dl/aux/`를 뺀 272개 문서의 제목(front matter
  `title`), 본문, 목록, 표 칸, 그림 설명(alt).
- 번역하지 않음: 코드 블록과 그 안의 러시아어 주석, 인라인 코드, 수식(수식 속
  `\text{…}`의 러시아어 포함), URL, 저자·편집자 이름(원문 표기 그대로), 외부 문제
  이름. 그림 속 글자.
- 범위 밖:
  - **영어판 `content/english`.** 이용 허락이 없다(조사 결과). 빌드에서도 지운다.
  - **초안 111개.** 원문 사이트에도 나오지 않는다.
  - **`dl/aux/`.** reveal-hugo 슬라이드 묶음이고 번역 사이트는 그 테마를 쓰지 않는다.

## 접근 방식

### 1. `yeokja.toml`

```toml
[project]
source_lang = "ru"
target_lang = "ko"

[[sources]]
path = "upstream/content/russian"
pattern = "**/*.md"
exclude = ["dl/aux/**", <초안 111개>]
parser = "mkdocs"
output = "ko/content/korean/{path}"

[translation]
concurrency = 6
```

- **파서 `mkdocs`.** `$`/`$$` 수식을 원자로 가리고 front matter의 `title`만
  세그먼트로 낸다. Hugo 전용 문법(shortcode)이 없어 그대로 맞는다. `inline_tags`는
  mkdocs 파서에 아직 없으므로 끈다.
- **초안은 `exclude`에 고정한다.** 빌드 전처리로 초안을 지우는 안은 번역 범위를
  바꾸지 못한다 — `yeokja status --check upstream/content/russian`은 디렉터리의
  모든 문서를 보므로 초안의 세그먼트를 미번역으로 세어 CI가 실패하거나 provider를
  부르게 된다. 그래서 `scripts/scope.py`가 원문 front matter에서 목록을 계산하고
  (`print`), 빌드 첫 줄의 `scope.py check`가 yeokja.toml과 원문이 어긋나면 실패한다.
  원문을 올려 초안이 바뀌면 이 검사가 알려 준다.
- **기본 프롬프트.** 러시아어 원문에도 언어 이름을 설정에서 읽어 동작하고, 합쇼체를
  요구한다(시험 번역에서 확인). 커스텀 템플릿을 두지 않으므로 GFM 경고 마커 규칙과
  인라인 태그 자리표시자 문제가 없다.

### 2. 용어집

러시아어는 격변화가 있어 용어 검사(단어 경계 일치)가 원형 하나로는 다른 격의 같은
용어를 놓친다. 그래서 핵심 용어마다 말뭉치에 실제로 나오는 격·수 형태를 모두 같은
번역어의 항목으로 둔다(`дерево отрезков`, `дерева отрезков`, `дереве отрезков`, … →
세그먼트 트리). 번역어는 한국어 경시 프로그래밍·알고리즘 교재의 관용을 따른다:
세그먼트 트리, 트립(декартово дерево), 신장 트리, 최단 경로, 동적 계획법(구어
динамика와 ДП는 DP), 매칭(паросочетание), 볼록 껍질, 희소 테이블, 트라이(бор),
영속(персистентный), 해시·해싱, 누적 합, 서로소 집합(СНМ은 DSU), 이진 탐색, 펜윅
트리, 강한 연결 요소, 증가 경로, 올바른 괄호 문자열(ПСП) 등.

뜻이 장마다 갈리는 단어는 용어집에 두지 않는다: вершина(그래프의 정점/다각형의
꼭짓점), ребро(간선/변), поток(유량/입출력 스트림), перебор(완전 탐색/순회).
약어 ДО(дерево отрезков)는 용어 검사가 대소문자를 가리지 않아 전치사 до와
겹치므로 두지 않는다. 첫 번역 뒤 `yeokja terms build`로 나머지 반복 용어와 제목을
고정한다. terms build가 고른 단어 가운데 вершина·ребро의 격 형태와 компонент
(PCA의 주성분)처럼 장마다 뜻이 갈리는 것은 지우고 `rejected`에 둔다. 평면 기하의
скалярное/векторное произведение은 국내 교재 관례대로 내적/외적으로 고친다.

사이트 이름 Алгоритмика는 "알고리즘카"로 고정한다(러시아어 '알고리즘'+'-카'의 조어를
한국어 독자가 읽을 수 있게 옮긴 이름). 랜딩 페이지 등 다른 곳에서는
"알고리즘카(Алгоритмика)"처럼 원어를 함께 적는다.

### 3. 검사

- `evaluate --mechanical-only`의 남은 항목은 두 종류의 오탐이다.
  - Alignment 4건: 숫자 0·1, `sift down` 같은 짧은 토큰이 이웃 세그먼트와 겹친다.
  - Format 3세그먼트(각 2건): 원문이 `$f[l, r, k, p] = $ максимальная … $p$`처럼
    닫는 `$` 앞에 공백을 둔다. 평가기는 이를 닫는 구분자로 보지 않아 사이의 러시아어를
    수식으로 읽지만, KaTeX는 첫 `$`에서 닫으므로 그 글은 본문이다. 번역은 본문을
    옮겨야 하므로(그렇지 않으면 러시아어가 남는다) 평가기와 맞출 수 없다. 이 중 2개
    (`cs/games/nim.md`, `math/algebra/Операции_с_Матрицами.md`)는 모델이 러시아어를
    그대로 두었던 것을 state에서 직접 고쳤다.
- 용어 검사는 대소문자를 가리지 않으므로 НОД(최대공약수)는 구어 нод(노드)와
  겹쳐 용어집에서 뺐다.

### 4. 빌드

조립 트리는 `upstream` 위에 `ko/`(번역된 `content/korean`)와 `overlay/`를 겹친다.

- **`overlay/config.yaml`** 이 원문 `config.yaml`을 대신한다. 한국어 한 언어
  (`contentDir: content/korean`, `locale: ko-KR`), `baseURL:
  https://yeokja.moreal.dev/algorithmica/`, 하위 경로에서 테마의 절대 경로
  (`/icons/…`)가 동작하도록 `canonifyURLs: true`, 테마는 `[algorithmica]`만,
  출력은 HTML과 검색 색인, 분류(태그) 쪽은 끈다. Hugo 0.123부터 켜지는 내장
  링크·그림 렌더 훅은 끈다(`renderHooks.*.useEmbedded: never`). 옛 위키 문서의
  `[…](Категория:Конспект)` 같은 링크를 URL로 읽다 빌드를 멈추고, 원문이 쓰던
  Hugo 0.96에는 없던 동작이다. 비교용 원문 빌드도 같게 둔다.
- **테마는 고치지 않고 사이트 쪽 `layouts/`·`i18n/`으로 덮는다.** Hugo는 사이트의
  같은 경로 파일을 테마보다 먼저 쓴다. 테마 파일(MIT 고지 포함)은 그대로 둔다.
  - `partials/analytics.html`: 빈 파일(방문 통계 제거).
  - `partials/head.html`: TikZJax와 lunr를 빼고, 제목과 본문에서 검색어를 그대로
    찾는 부분 문자열 검색으로 바꾼다. lunr는 한국어를 낱말로 나누지 못한다.
    스크립트 속 경로(`/searchindex.json`, `/syntax-dark.css`)는 `canonifyURLs`가
    고치지 않고, `canonifyURLs`를 켜면 `relURL`도 하위 경로를 붙이지 않으므로
    `absURL`을 쓴다.
  - `partials/buttons.html`: prose.io 편집 버튼을 빼고(원문 저장소에 대한 편집이
    된다) GitHub 버튼은 원문 러시아어 파일을 가리키게 한다(`originalPath`).
  - `partials/header.html`: 날짜를 한국어 형식으로, 첫 화면에 번역 고지.
  - `partials/footer.html`: 원저작 표시·CC BY-SA 4.0·번역 고지.
  - `_default/baseof.html`(`lang='ko'`), `404.html`, `i18n/ko.toml`.
- **`scripts/prepare_site.py korean`(derive generate 단계).**
  - 영어판, reveal-hugo, `dl/aux`를 지운다.
  - `content/russian`의 그림 등 Markdown이 아닌 파일을 `content/korean`으로 복사한다.
  - 초안을 원문 그대로 복사한다. Hugo는 초안을 내놓지 않지만, 초안인 `_index.md`가
    없으면 그 섹션(math/ml/dl 등)을 디렉터리 이름 제목으로 새로 만들어 첫 화면에
    올리므로, 원문 사이트와 섹션 구조를 같게 두려면 필요하다.
  - 범위의 모든 문서에 번역본이 있는지 확인한다.
  - `*.ru.md`를 `*.md`로 바꾼다. 한국어 한 언어 사이트에서 `.ru` 접미사는 다른
    언어 문서로 읽혀 사라진다. URL은 같다.
  - `published:`·`date: {}` 줄을 지우고, `part:`를 스크립트의 사전으로 옮기고
    (파서는 `title`만 번역한다. 값은 8개), 원문 경로를 `originalPath`로 덧붙인다.
    `menuTitle`(CS, Math, ML, DL, RL, NLP, CV)은 원문도 라틴 약어이므로 그대로 둔다.
  - 조립 트리의 파일은 심볼릭 링크다. 고칠 파일은 링크를 끊고 새로 쓰고(링크를
    따라 쓰면 `ko/`·`upstream/`이 바뀐다), 마지막에 남은 링크를 모두 실제 파일로
    바꾼다. Hugo는 링크인 파일(테마·본문)을 읽지 않는다.
- **원문 비교 빌드.** `build.html`은 먼저 upstream 사본에
  `prepare_site.py russian`(같은 삭제·front matter 손질)을 하고
  `scripts/original.yaml`을 겹쳐 같은 Hugo·같은 baseURL로 러시아어 사이트를
  빌드한 뒤, 번역 사이트를 빌드하고 `scripts/check_site.py`로 비교한다.
- **수식 비교.** 쪽마다 서로 다른 수식의 집합을 비교한다. 번역은 같은 변수를 한 번
  더 쓸 수 있으므로 개수는 보지 않고, 원문 문단 속 줄바꿈이 번역에서 한 줄로
  합쳐지므로 공백을 접는다. 다를 수밖에 없는 쪽 5개는 이유와 함께
  `KNOWN_MATH`에 두고 보고만 한다: 본문에 글자 그대로의 `$`가 있어 짝이 번역문을
  끼고 맺어지는 쪽 2개, 원문 수식이 줄머리 `- 1`·`- b`·`+ 2`에서 Markdown 목록으로
  깨지는데 번역은 문단을 한 줄로 이어 바르게 그리는 쪽 3개.
- **제목 ID는 고정하지 않는다.** 한국어 제목의 Hugo 자동 ID는 러시아어 제목의
  ID와 다르지만, 범위 안에서 제목 앵커를 가리키는 문서 간 링크는 하나(`bitset#Гаусс`)
  뿐이고 원문 빌드에서도 그 대상이 없어 깨져 있다(`check_site.py`가 원문 결함으로
  보고). 원문 제목 ID를 `{#id}`로 붙이면 쪽마다 제목 수 대응과 Hugo ID 규칙 재현이
  필요해 얻는 것보다 비용이 크다. 외부에서 러시아어 앵커로 들어오는 링크는 한국어
  사이트와 무관하다.
- **툴체인.** `nix/projects/algorithmica.nix`: nixpkgs `hugo`(0.165 extended, SASS
  포함)와 `python3`(스크립트는 표준 라이브러리만 쓴다).

## 라이선스

- 원문 러시아어판 자료는 CC BY-SA 4.0이다. 번역은 개작물이므로 같은 CC BY-SA
  4.0으로 배포한다(§3(b)). 사이트 하단과 첫 화면, README에 저작자(Sergey Slotin,
  Tinkoff Generation, Algorithmica.org), 원문 링크, 라이선스 링크, 원문을 번역·수정한
  비공식 번역이라는 고지를 둔다.
- 테마(`themes/algorithmica`)는 MIT다. 테마 파일을 그대로 싣고 LICENSE를 보존하며,
  덮어 쓴 레이아웃은 프로젝트의 `overlay/`에 있다.
- 그림 일부는 저자도 출처를 모른다고 밝혔다. 원문 사이트가 같은 그림을 CC BY-SA
  아래 싣고 있으므로 수정 없이 함께 싣고, README에 이 사실을 적는다. 권리자의
  요청이 있으면 해당 그림을 뺀다.
- 영어판은 이용 허락이 없으므로 번역하지도, 산출물에 싣지도 않는다.
- README는 AGENTS.md의 출처 표기 규칙을 따른다. 루트 `LICENSE.md` 표에 CC BY-SA
  4.0을 추가한다.

## 배포

`.github/pages-projects.json`에 `project = "algorithmica"`, `target = "html"`,
`artifact_path = "projects/algorithmica/dist/site"`를 더하고 `stage-pages.sh`에
스테이징 줄을 더한다. CI 재구성(`rebuild-translations.sh`)은 재구성 전에
`scope.py check`와 `status --check upstream/content/russian`을 부른다. 랜딩
페이지는 "알고리즘" 주제에 카드를 둔다.

## 작업 순서

1. 프로젝트 골격: `yeokja.toml`(scope.py로 exclude 생성), `glossary.toml`,
   `.gitignore`, `nix/projects/algorithmica.nix`.
2. 시험 번역(`cs/range-queries/sparse-table.md`) → 문체·용어·수식 확인.
3. 전체 번역 → `evaluate --mechanical-only` → `terms build`(`--dry-run` 먼저) →
   `terms.toml` 검토 → `glossary adopt --prefer HEAD` → 남은 세그먼트 번역 →
   `status --check`.
4. overlay·스크립트·빌드 → `check_site.py` 통과 → 브라우저로 첫 화면과 본문 확인.
5. README, 공유 파일 배선 항목.

## 되짚을 수 있는 결정

- **초안 제외.** 저자가 초안을 공개 사이트에 내지 않았으므로 번역도 하지 않는다.
  원문을 올렸을 때 초안이 풀리면 `scope.py check`가 실패하고, 목록을 갱신한 뒤 새
  문서만 번역한다.
- **math/ml/dl의 고아 쪽.** 원문 사이트와 같게 첫 화면 목차에 올리지 않는다. 이
  분야를 목차에 올리려면 초안 `_index.md` 대신 번역한 섹션 머리말을 두는 안을 따로
  본다.
- **코드 주석.** 코드 블록 속 러시아어 주석은 번역하지 않는다. 독자 피드백에 따라
  주석만 옮기는 안(코드 블록 전처리)을 따로 다룬다.
- **검색.** 부분 문자열 검색은 형태소를 모르지만 한국어 조사가 붙어도 어간으로
  찾을 수 있다. 쪽 수(약 300)와 색인 크기에서 충분하다.
