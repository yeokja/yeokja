# Linux Kernel Internals 한국어 번역 설계

## 목표

Laveesh Bansal의 *Linux Kernel Internals*(<https://kernel-internals.org/>, 저장소
<https://github.com/laveeshb/linux-kernel-internals>, MkDocs Material, 문서 CC BY-SA 4.0 ·
코드 스니펫 GPL-2.0)를 yeokja로 한국어로 옮겨 GitHub Pages 웹 판
(`https://yeokja.moreal.dev/linux-kernel-internals/`)으로 낸다. 프로젝트 이름과 Pages 경로는
`linux-kernel-internals`, 한국어 제목은 "Linux 커널 내부 구조"다. 파서는 `mkdocs`이고
이 사이트의 `markdown_extensions`에 맞춘 소스 옵션(`math = false`,
`definition_lists = true`, `inline_tags = true`)은
`docs/superpowers/specs/2026-11-15-mkdocs-def-list-and-math-option-design.md`가 더한 것이다.
용어 검사·복수형·인라인 태그 재시도·번역할 글자가 없는 세그먼트의 동작은
`docs/superpowers/specs/2026-11-15-translation-robustness-core-fixes-design.md`를 따른다.

완료 상태는 다음 조건을 모두 만족해야 한다.

- 원문이 `projects/linux-kernel-internals/upstream` 서브모듈로 커밋
  `6ee74a0b2e5f49c75d107ac4935369f73a004546`(2026-10-23)에 고정되어 있다.
- 번역 대상(본문 498쪽과 nav 레이블 목록)의 모든 세그먼트가 번역 상태이고 소스 규칙
  둘(`upstream/docs`, `nav`)의 `yeokja status --check`가 통과한다.
  `evaluate --mechanical-only`가 0건이다.
- `mkdocs build --strict`로 만든 사이트가 같은 툴체인으로 만든 영어 원문 사이트와 쪽 목록,
  쪽별 블록 구조(제목·제목 ID·정의 목록·admonition·mermaid·코드 블록·표·목록 수)가 같고,
  원문 빌드에 없던 깨진 로컬 링크·앵커가 없으며, nav가 번역되어 있다.
- Pages 배포 배선, 랜딩 페이지 항목, README 출처 표기, 루트 `LICENSE.md` 항목이 갖춰져
  있다.

## 조사 결과

- **형식.** MkDocs 1.6.1 + Material(원문은 9.7.7 고정, nixpkgs는 9.7.6), Python-Markdown.
  `markdown_extensions`는 `tables`, `admonition`, `def_list`, `pymdownx.highlight`,
  `pymdownx.superfences`(mermaid custom fence), `attr_list`이고 `pymdownx.arithmatex`는
  없다. 플러그인은 `search`뿐이다. 다국어 구조와 PDF는 없다.
- **규모.** `docs/` 아래 Markdown 499개(산문 약 58만 단어, 줄 기준 57%가 코드 블록).
  `contributing.md`를 뺀 498쪽이 `mkdocs` 파서로 51,454 세그먼트, nav 레이블이 580
  세그먼트다. 세그먼트 길이는 중앙값 51자, 최대 1,648자이며 짧은 표 칸이 많다. 코드 스팬
  하나뿐이거나 글자가 없는 세그먼트가 약 3,000개 있다(provider를 부르지 않고 원문 그대로
  기록된다).
- **쓰이는 구문.** admonition 232개(warning·note·tip·info; `???`·탭 없음), 정의 목록 54쪽
  290줄(war-stories 머리의 "Landed", "Fixed in", "CVSS" 같은 메타데이터), mermaid 94개, 표
  718개, 프런트매터는 `index.md` 하나(`hide: [navigation]`). 원시 HTML 블록·각주·이미지·
  스니펫 포함·GFM alert는 없다.
- **공백 없는 admonition 머리.** `mm/numa-reclaim.md`(4곳)와 `mm/tlb-optimization.md`
  (6곳)는 `!!!note`, `!!!tip "Performance tip"`처럼 `!!!` 뒤에 공백 없이 쓴다.
  Python-Markdown의 admonition 정규식은 `!!! ?`이므로 원문 사이트에서 이 10곳은 정상
  admonition으로 렌더링된다(영어 빌드에서 두 쪽의 admonition이 8개·12개). `mkdocs` 파서도
  같은 범위를 머리로 읽고 재구성 때 머리 줄을 원문 그대로 남기므로 따로 다룰 것이 없다.
- **nav.** `mkdocs.yml`의 `nav:`에 589줄이 영어로 하드코딩되어 있다(레이블 있는 줄 580,
  레이블 없이 쪽만 적은 줄 9). 일부 레이블은 YAML 따옴표로 묶여 있다(`"vDSO: Fast …"`,
  `'The Cost of Asking "Is There …" Too Often'`). nav에 없는 쪽은 없다.
- **앵커.** 빌드는 `mkdocs build --strict`이고 `validation.links.anchors: warn`이라 깨진
  `#앵커`가 빌드 실패다. 앵커 링크는 126개다. 제목 ID는 `toc` 기본 slug(영어 제목)이며
  permalink는 켜져 있지 않다. URL은 디렉터리 꼴(`mm/numa-reclaim/index.html`)이다.
- **스타일.** `docs/stylesheets/extra.css`가 `html { font-size: 150% }`(넓은 화면에서
  165%·180%)로 UI 전체를 키우고, 코드 글꼴을 시스템 monospace로, 정의 목록을 2열 격자로
  바꾼다. `docs/javascripts/external-links.js`는 외부 링크를 새 탭으로 연다.
- **라이선스.** `LICENSE`가 "Documentation: CC BY-SA 4.0 / Code snippets: GPL-2.0"으로
  나눈다(전문은 `LICENSE-CC-BY-SA-4.0`, `LICENSE-GPL-2.0`). "GPL-2.0"에 `-only`·
  `-or-later` 구분은 없다. `docs/index.md`가 "Tux logo: Larry Ewing (original), Simon Budig
  (SVG)"와 Disclaimer(확정적 참고 자료가 아닌 커뮤니티 학습 자료이며 오류가 있을 수 있다)를
  밝힌다. `mkdocs.yml`에 `copyright:` 푸터는 없다.

## 범위

- 번역: `docs/**/*.md` 중 `contributing.md`를 뺀 498쪽(`index.md`, `about.md`,
  `changelog.md` 포함)의 제목·본문·목록·표·인용·admonition 제목과 본문·정의 목록,
  그리고 nav 레이블 580개.
- 번역하지 않음: `contributing.md`(원문 저장소에 기여하는 절차; 쪽은 영어 그대로 싣고 제목
  아래에 번역하지 않은 이유 한 줄을 넣는다), 루트 `README.md`(사이트에 쓰이지 않는다),
  코드 블록(주석 포함), 언어 없는 펜스의 ASCII 다이어그램·커널 로그, mermaid 다이어그램의
  레이블, 커널 식별자·커밋 해시·CVE 번호·사람 이름, 인용된 커밋·패치·글의 제목.
- 산출물: 웹 HTML만.

## 접근 방식

### 1. 소스 규칙

```toml
[[sources]]
path = "upstream/docs"
pattern = "**/*.md"
exclude = ["contributing.md"]
parser = "mkdocs"
output = "ko/docs/{path}"
math = false
definition_lists = true
inline_tags = true

[[sources]]
path = "nav"
pattern = "nav.md"
parser = "mkdocs"
output = "ko-nav/{path}"
math = false
inline_tags = true
```

- `math = false`: `pymdownx.arithmatex`가 없으므로 `$`는 글자다(`$1 billion`,
  `/proc/irq/$N/…`).
- `definition_lists = true`: 용어 줄과 `:   ` 정의가 따로 세그먼트가 되고 표시와 줄 구조는
  원문 그대로 남는다.
- `inline_tags = true`: 이 원문은 코드 스팬과 링크가 매우 많다. 강조·링크를 태그로
  주고받아 조사 앞 강조, 링크 목적지 변경·손실이 구조적으로 생기지 않게 한다. 직렬화기가
  쓰는 이스케이프(`### 1\. 선제적 회수`)는 Python-Markdown에서 글자로 보인다. 직렬화기가
  확인하지 못한 응답(글자 대괄호가 링크가 되는 `[59:56](…)` 꼴, 태그 잔재)은 저장되지 않고
  재시도되며, 저장된 번역에 원문보다 많은 링크·인라인 HTML이 있으면
  `evaluate --mechanical-only`의 `InlineStructure`가 낸다.

**파생 소스(`scripts/sources.py extract [--check]`).** upstream 파일 그대로가 아닌 소스
하나(nav 레이블 목록)를 원문에서 만들어 커밋해 둔다. `--check`는 커밋된 파일이 고정된
원문에서 만든 것과 같은지 검사한다. 빌드와 CI 재구성이 가장 먼저 이 검사를 돌리므로, 원문
nav가 바뀌면 배포가 멈춘다. 스크립트는 표준 라이브러리만 쓴다(CI의 재구성 단계는 nix 셸
밖에서 돈다).

- `nav/nav.md` — `mkdocs.yml`의 nav 레이블을 nav 순서대로, nav 깊이만큼 들여쓴 Markdown
  목록으로 적은 것. 레이블 없는 줄은 번역할 것이 없어 뺀다. 레이블에 Markdown 문법이 될
  글자(`` \ ` * [ ] < > ``, 낱말 경계의 `_`)가 있으면 실패한다(지금 원문에는 없다).

**nav를 literate-nav로 하지 않는 이유.** 번역된 nav 목록을 `mkdocs-literate-nav`에 그대로
주는 방법을 먼저 검토했다. 다음 이유로 번역된 레이블을 `mkdocs.yml`의 nav에 되돌려 넣는
방법을 택한다.

- literate-nav는 절 제목을 `<li>`의 첫 텍스트에서 그대로 읽는다. 이스케이프(`\_`)를 풀지
  않고, 제목에 인라인 문법이 생기면(코드 스팬·강조) 구문 오류다. 인라인 태그 직렬화기가
  쓰는 이스케이프와 모델이 붙이는 백틱을 우리 쪽에서 풀어야 한다.
- nav 파일이 `docs/` 안의 쪽이 되어 빌드에서 따로 빼야 하고, 영어 빌드와 쪽 목록이
  달라진다.
- 구조(쪽 목록·순서·절 중첩)가 원문 `mkdocs.yml` 그대로이므로 "번역된 nav가 원문 nav와
  같은 구조인가"를 검사할 필요가 없다. 레이블 수와 줄별 깊이만 맞으면 된다.
- 플러그인과 nix 패키지가 하나 준다.

### 2. 프롬프트

`[provider]`는 `claude_code` / `claude-opus-5-5`, `[translation] concurrency = 6`,
`batch_segments = 32`, `[evaluation] auto_evaluate = true, style_evaluate = false,
max_retries = 3`. 커스텀 `prompt_template`을 쓴다. 기본 템플릿에 없는 이 원문만의 규칙이
여럿이기 때문이다. 템플릿은 `{inline_tags}` 자리표시자와 AGENTS.md의 GFM alert 마커 보존
문장, 합쇼체 규칙을 직접 담고, 다음을 더한다.

- 식별자·버전·경로·명령·해시·CVE 번호·사람 이름뿐인 세그먼트는 원문 그대로 낸다. 글자가
  있어 provider로 가는 세그먼트이기 때문에 규칙으로 둔다. 코드 스팬·숫자와 단위·문장부호
  뿐인 세그먼트는 코어가 provider 없이 원문 그대로 기록하므로 프롬프트에 적지 않는다.
- 코드 스팬 안은 바이트 그대로이고, 원문에 없는 백틱을 붙이거나 있는 백틱을 떼지 않는다.
- 커널 식별자, 사람·회사·제품 이름은 원문 그대로다.
- 커밋·패치·패치 시리즈의 제목과 글·발표·메일링 리스트 스레드의 제목은 고유명사로 보고
  옮기지 않는다(`("mm: fix …")`, `[LWN: The …](url)`). 독자는 이 문자열로 커밋과 글을
  찾는다. 메일링 리스트·커밋 메시지·기사에서 인용한 **문장**은 뜻을 옮기되 따옴표와
  식별자를 유지한다.
- `$`는 달러 기호다. `{{`, `{%`, `{#`를 넣지 않는다. 링크뿐인 목록 항목·표 칸은 링크
  하나로 남는다. 인라인 `<br>`과 HTML 문자 참조는 그대로 둔다.
- 짧은 표 칸·정의 목록 용어는 짧게(Yes → 예, No → 아니요), 값·단위는 그대로.
- 제목·표 칸·nav 레이블은 문장이 아닌 명사구로 쓴다.
- huge page와 transparent huge page는 옮기지 않고 영어 단수형으로 쓴다(3절).

### 3. 용어

`glossary.toml`은 번역어가 갈리기 쉬운 커널 용어를 고정한다: user space(사용자 공간),
system call·syscall(시스템 콜), scheduler(스케줄러), preemption(선점), interrupt
(인터럽트), context switch(컨텍스트 스위치), runqueue(런큐), load balancing(부하 분산),
page table·page fault·page cache(페이지 테이블·폴트·캐시), folio(폴리오), reclaim(회수),
allocator(할당자), memory leak(메모리 누수), OOM killer(OOM
킬러), spinlock(스핀락), mutex(뮤텍스), race condition(경쟁 상태), deadlock(교착 상태),
reference count·refcount(참조 카운트), file descriptor(파일 디스크립터), block layer
(블록 계층), device driver(디바이스 드라이버), namespace(네임스페이스), mainline
(메인라인), upstream(업스트림), patch series(패치 시리즈), regression(회귀),
vulnerability(취약점), privilege escalation(권한 상승), war story(실전 사례), trade-off
(트레이드오프), mailing list(메일링 리스트), virtualization(가상화), hypervisor
(하이퍼바이저), tracepoint(트레이스포인트), fast·slow path(빠른·느린 경로), hot path
(핫 패스).

- 한국 커널 개발자가 영어로 쓰는 말은 번역어를 원어와 같게 두어 음차를 막는다:
  use-after-free, copy-on-write, softirq, tasklet, workqueue, cgroup.
- **huge page는 옮기지 않는다.** `huge page`·`huge-page`의 번역어는 `huge page`,
  `transparent huge page`의 번역어는 `transparent huge page`다. 근거:
  - 한국어 커널 문서와 개발자 글은 "huge page"를 그대로 쓰고, 번역어("대형 페이지", "거대
    페이지", "큰 페이지")는 하나로 굳지 않았다. 같은 원문이 `large page`(x86 PS 비트가
    매핑하는 페이지, "큰 페이지")와 `large folio`("대형 폴리오")를 따로 쓰므로, 번역어를
    쓰면 서로 다른 세 개념이 비슷한 말로 겹친다.
  - 독자가 마주치는 이름(`HugePages_Total`, `nr_hugepages`, `MAP_HUGETLB`, hugetlbfs,
    khugepaged, THP)이 모두 이 낱말에서 나왔다. 본문이 같은 낱말을 써야 식별자와 설명이
    이어진다.
  - `transparent huge page`는 약어 THP와 한 쌍으로 쓰이는 기능 이름이다. "투명"만 옮기면
    약어와 풀이가 어긋나므로 통째로 원어로 둔다.
  - 한국어 문장 안에서는 단수형으로 쓰고(huge page를 할당합니다), 대소문자는 원문을
    따른다(제목의 "Transparent Huge Page(THP)"). `glossary_plurals`가 복수형 출현을 같은
    항목으로 읽고, 검사는 대소문자를 가리지 않는다.
  - 붙여 쓴 `hugepage`(커널 문서·커밋 제목의 철자)와 `hugetlb`·`hugetlbfs`는 식별자에
    가까운 이름이라 용어로 강제하지 않고 원문 철자 그대로 둔다. `huge`가 페이지가 아닌
    것을 꾸미는 경우(huge machines, huge vmalloc buffers)는 보통 낱말로 옮긴다.
- **복수형.** `[project] glossary_plurals = true`로 영어 규칙 복수형을 단수형 항목이 함께
  맞게 한다. 용어집과 `terms.toml`에는 단수형만 둔다. 번역어가 단수형과 다른 복수형이
  생기면 그 복수형만 따로 항목으로 둔다(긴 항목이 먼저 맞는다).
- "War Stories"는 버그·CVE·성능 회귀의 경위를 다루는 글 묶음이다. 장애가 아닌 설계 논쟁과
  회귀 사례도 있어 "장애 사례"보다 넓은 "실전 사례"로 옮긴다.
- 세그먼트 전체가 그 글자인 경우의 번역(`[whole]`)으로 반복되는 제목·레이블·표 칸을
  고정한다: Further reading(더 읽을거리), War Stories(실전 사례), Getting Started
  (시작하기), Yes(예), No(아니요). nav 레이블과 쪽 제목이 같은 말을 쓰게 된다.
- lock, swap, slab, kernel처럼 동사·복합어로도 흔한 짧은 단어는 두지 않는다. 용어 검사는
  번역어가 들어 있는지만 보므로 "swap two pointers" 같은 문장에서 수렴하지 않는 재시도를
  부른다.
- **원문 그대로 두는 제목과 용어 검사.** 옮기지 않는 커밋·글·책 제목 속의 용어
  (`"…: fix deadlock in …"`, `[Per-node reclaim and kswapd](url)`)는 따로 다루지 않는다.
  용어의 출현이 모두 링크 글자·큰따옴표·강조 구간 안에 있고 용어보다 긴 그 구간이 번역에
  그대로 있으면 용어 검사가 번역어를 요구하지 않는다.
- **`except`.** 구분 기호가 없어 위 규칙이 닿지 않는 자리에만 쓴다(`terms.toml`).
  - 괄호 안의 약어 풀이와 고유한 이름: `GIC (Generic Interrupt Controller)`, `KASLR
    (Kernel Address Space Layout Randomization)`, `SRAT (System Resource Affinity Table)`,
    `BDI (backing device info)`, `Linux Kernel Memory Model`, `Memory Latency Checker`,
    `Device Mapper`. 약어가 어느 낱말에서 왔는지 보이도록 풀이는 영어로 둔다.
  - 괄호 안에 영어 이름을 나열한 곳: `(hash, array, ring buffer…)`, `(descriptor table,
    avail ring, used ring)`, `(SQE, CQE, SQPOLL, io-wq, fixed buffer, multishot)`. 맵
    종류·영역·용어집 표제어의 이름이다.
  - 명령 이름: `perf sched latency`.
  - 따옴표 짝이 세그먼트 경계에서 어긋나는 문장 속의 커밋 제목 한 곳
    (`drm/etnaviv: hook up DRM GPU scheduler`).
  - 동사이거나 다른 낱말인 경우: `entry points to`·`entry points directly`(entry point),
    `Nothing in the kernel objects to this`(kernel object), `(VFs) from a single physical
    function`(VFS).
  - 용어를 영어로 남긴 번역을 `except`로 덮지 않는다. 쪽 제목·nav 레이블·그 쪽을 가리키는
    링크 글자가 같은 용어를 다르게 옮겼으면 번역을 고친다.

첫 번역 뒤 `yeokja terms build`(먼저 `--dry-run`)로 반복 용어와 제목·레이블을 고정하고,
`terms.toml`을 검토한 뒤 `glossary adopt --prefer HEAD`, 남은 세그먼트만 다시 번역한다.
`terms.toml` 검토 기준은 다음과 같다.

- 번역어가 하나뿐인 흔한 낱말(kernel, memory, driver, module, cache, bug, hardware,
  timer, policy, mapping …)은 뺀다. 일관성에 보탬이 없고 제목 속에서 검사에만 걸린다.
- 동사·다른 뜻으로도 쓰이는 낱말(address, process, function, bit, impact, analysis,
  struct, crypto)은 뺀다.
- device는 "디바이스"로 통일한다(`glossary.toml`의 "디바이스 드라이버"와 같은 말을 쓴다):
  device, block device, device model.
- 문맥마다 뜻이 다른 표 칸·제목의 `[whole]` 항목(Full, Original, Control, Why?)은 뺀다.
  용어와 어긋나는 `[whole]` 번역어는 용어에 맞춘다(Parameter → 파라미터, Pages scanned by
  direct reclaim → 직접 회수로 스캔된 페이지).
- 뺀 항목은 `rejected`에 넣어 다음 `terms build`가 다시 묻지 않게 한다.

### 4. 조립(derive)

`[derive] base = "upstream"`에 `ko/`(`require_base = true`)와 `overlay/`를 겹치고 생성
단계 `scripts/prepare_site.py . ko-nav/nav.md`를 돈다. `ko-nav/`는 트리에 겹치지 않고 이
스크립트의 입력으로만 쓴다.

**`scripts/prepare_site.py`** — 원문 `mkdocs.yml`은 Python 태그(`!!python/name`)를 담고
있어 텍스트로 고친다. 고칠 자리를 정확히 하나씩 찾지 못하면 실패하므로, 원문 설정이 바뀌면
반쯤 적용된 설정으로 빌드되지 않는다.

- `site_name`("Linux 커널 내부 구조"), `site_description`, `site_url`.
- `theme.language: ko`, 밝은·어두운 화면 전환 단추 이름, `search` 플러그인의
  `lang: [ko, en]`.
- nav: 원문 nav의 레이블 있는 줄에 번역된 레이블을 순서대로 넣는다. 레이블 수와 줄별
  깊이가 다르면 실패한다. 번역 목록의 Python-Markdown 이스케이프는 풀고 백틱은 뗀다
  (nav 레이블은 글자다).
- `extra.social`: 원문 저장소 링크는 두고, 원문의 Discussions·Issues 링크는 뺀다(번역의
  오류가 원문 저장소로 신고되지 않게). yeokja 저장소 링크를 더한다.
- `extra_css`에 `stylesheets/ko.css`를 더한다.
- `copyright:` 푸터(라이선스 절 참고)와 `hooks: [hooks/anchors.py]`.
- `docs/contributing.md`의 제목 아래에 "원문 저장소 기여 절차라 번역하지 않았다. 번역
  오류는 yeokja 저장소에 알려 달라"는 한 줄을 넣는다.
- `--english`: 영어 기준 빌드용. hook만 더한다.

**`overlay/hooks/anchors.py`** — 번역된 제목은 slug가 달라지므로 영어 제목의 ID를 그대로
입힌다. 한 hook이 두 빌드에서 돈다. 영어 기준 빌드(`ANCHOR_IDS_OUT`)에서는 `toc` 뒤에 도는
treeprocessor가 쪽(`src_uri`)마다 제목 ID를 문서 순서대로 기록해 빌드 끝에 JSON으로 쓰고,
한국어 빌드(`KO_ANCHOR_IDS`)에서는 `attr_list` 뒤 `toc` 앞에 도는 treeprocessor가 그 ID를
번역된 제목에 순서대로 입힌다. 제목 수가 다르면 빌드가 실패한다. cp-algorithms는 영어
빌드의 HTML에서 permalink가 달린 제목을 뽑지만, 이 사이트는 permalink가 없고 URL이
디렉터리 꼴이라 HTML 경로에서 `src_uri`를 되찾을 수 없으므로 빌드 안에서 기록한다.

**`overlay/docs/stylesheets/ko.css`** — 한글 줄바꿈(`word-break: keep-all`,
`overflow-wrap: anywhere`)만 둔다. 원문 CSS가 키운 글자 크기(`html { font-size: 150% }`)는
한국어 본문에서도 읽기에 무리가 없어 원문 그대로 둔다.

### 5. 빌드와 검사

`[build.html]`은 조립 트리에서 다음을 차례로 한다(`outputs = ["site"]`).

1. `scripts/sources.py extract --check`.
2. 영어 기준 빌드: `upstream/docs`·`mkdocs.yml`과 hook을 `build/original-source`로 복사하고
   `prepare_site.py --english` 뒤 `ANCHOR_IDS_OUT=build/en-heading-ids.json mkdocs build
   --strict -d build/original`.
3. 한국어 빌드: `KO_ANCHOR_IDS=build/en-heading-ids.json mkdocs build --strict -d site`.
4. `scripts/check_site.py build/original site`:
   - 두 빌드의 쪽 목록이 같다.
   - 쪽마다 본문(`article`)의 제목 수와 제목 ID 순서, 정의 목록(`dl`·`dt`·`dd`),
     종류별 admonition, mermaid, 코드 블록, 표·행·칸, 목록·항목, 인용 수가 같다.
   - 한국어 빌드에 영어 빌드에 없던 깨진 로컬 링크·`#앵커`가 없다. 파일은 디렉터리 목록의
     정확한 철자로 찾으므로, 대소문자를 가리지 않는 파일 시스템에서만 풀리는 링크도
     깨진 것으로 본다(Linux CI와 같은 판정).
   - nav 레이블 중 한글이 없는 것을 나열하고(식별자·약어뿐인 레이블은 정상), 서로 다른
     레이블의 25%를 넘으면 실패한다.
5. 원문 `LICENSE`, `LICENSE-CC-BY-SA-4.0`, `LICENSE-GPL-2.0`을 `site/`에 복사한다.

빌드 툴체인은 `nix/projects/linux-kernel-internals.nix`(`mkdocs`, `mkdocs-material`,
`pymdown-extensions`, `markdown`)다. 원문의 `requirements.txt` 고정(Material 9.7.7)이
아니라 nixpkgs 버전(9.7.6)을 쓴다. mermaid는 Material이 런타임 JS로 그리므로 패키지가
필요 없다.

### 6. 작업 순서

1. 골격: 설정, 용어집, 파생 소스, 스크립트, 오버레이, nix 셸, `.gitignore`(루트 고정 패턴
   `/ko/`, `/ko-nav/`, `/build/`, `/dist/` — `overlay/` 아래가 가려지지 않게).
2. 시험 번역: 정의 목록이 있는 war-stories 한 편, 표·admonition이 많은 한 편, 공백 없는
   admonition 머리가 있는 쪽, nav. 재구성 diff, 재시도율, 문체, 정의 목록이 `<dl>`로 렌더링되는지, 인라인 태그
   직렬화기의 이스케이프(`1\.` 등)가 Python-Markdown에서 글자로 보이는지 확인한다.
3. 전체 번역(백그라운드) → `evaluate --mechanical-only` 0건 → `terms build --dry-run` →
   `terms build` → `terms.toml` 검토 → `glossary adopt --prefer HEAD` → 남은 세그먼트 번역
   → `status --check`.
4. `nix develop path:../../nix#linux-kernel-internals -c ../../target/release/yeokja build
   html`, `check_site.py`, 브라우저 확인(홈, war-stories 한 편, 표가 많은 쪽, nav, 푸터,
   한국어 검색, 글자 크기).
5. README, 배포 배선(공유 파일: `pages-projects.json`, `stage-pages.sh`,
   `rebuild-translations.sh`, 루트 `LICENSE.md`, `site/index.html`).

## 라이선스

문서는 CC BY-SA 4.0, 코드 스니펫은 GPL-2.0이다. 번역은 문서의 개작물이므로 같은 CC BY-SA
4.0으로 내고, 코드 스니펫은 고치지 않고 GPL-2.0 그대로 싣는다. 식별자는 원문 표기 그대로
"GPL-2.0"으로 적는다(`-only`·`-or-later`를 우리가 정하지 않는다).

- 모든 쪽 하단(`copyright:` 푸터)과 README에 원저작자(Laveesh Bansal), 원문 사이트와
  저장소 링크, 두 라이선스(고정 커밋의 전문 링크), Tux 로고 표기(Larry Ewing 원본, Simon
  Budig SVG), 번역·변경 사실, 원저작자가 검토·보증하지 않은 비공식 기계 번역임을 표시한다.
- 원문 Disclaimer(확정적 참고 자료가 아닌 커뮤니티 학습 자료, 오류 가능)는 `index.md`
  본문에서 번역되어 남고, 푸터에도 한 문장으로 둔다. 번역이 원문의 오류를 그대로 옮길 수
  있기 때문이다.
- 라이선스 전문 세 파일을 사이트 루트에 함께 싣는다.
- 메일링 리스트·기사에서 인용한 문장은 원문이 인용으로 실은 범위 그대로 번역한다.

## 배포

`.github/pages-projects.json`에 `project = "linux-kernel-internals"`, `target = "html"`,
`artifact = "dist-linux-kernel-internals"`, `artifact_path =
"projects/linux-kernel-internals/dist"`를 더하고 `stage-pages.sh`에 `overlay_site
"dist-linux-kernel-internals" "site" "linux-kernel-internals"`을 더한다.
`rebuild-translations.sh`의 분기는 `scripts/sources.py extract --check` → 두 소스의
`status --check` → 두 소스의 `translate`다. 재구성 전에 확인하므로 CI가 provider를 부르지
않고, 원문이 새로 더한 쪽이 state 없이 빠지는 경우도 배포를 막는다.

## 되짚을 수 있는 결정

- **huge page를 옮기지 않음.** 번역어를 쓰려면 `glossary.toml`의 `huge page`·`huge-page`와
  `terms.toml`의 `transparent huge page`를 고치고 프롬프트의 해당 규칙을 뺀 뒤
  `glossary adopt` → `translate`를 돌린다.
- **쪽 사이 링크의 글자.** 다른 쪽을 가리키는 링크의 글자가 그 쪽의 영어 제목 그대로 남은
  곳이 있다(프롬프트의 "글 제목은 옮기지 않는다" 규칙이 사이트 안의 쪽 제목에도 적용된
  경우). 용어 검사는 이를 원문 그대로 둔 구간으로 읽어 지나간다. 번역된 쪽 제목으로
  맞추려면 프롬프트에 "같은 사이트의 `.md` 쪽을 가리키는 링크의 글자는 번역한다"를 더하고
  해당 세그먼트를 다시 번역한다.
- **커밋·글 제목을 옮기지 않음.** 제목까지 한국어로 읽히길 원하면 프롬프트 규칙을 빼고
  해당 세그먼트를 다시 번역한다.
- **"War Stories" = "실전 사례".** 바꾸려면 `glossary.toml`의 `terms`·`whole` 두 항목을
  고치고 `glossary adopt`로 다시 받아들인다.
- **mermaid·코드 주석·ASCII 다이어그램.** 영어로 남는다. 파서가 mermaid 레이블이나 코드
  주석을 세그먼트로 내게 되면 `reconcile`로 옮긴 뒤 새 세그먼트만 번역한다.
- **`contributing.md`.** 영어 그대로 싣는다. 번역하려면 `exclude`에서 빼고
  `prepare_site.py`의 안내 줄을 없앤다.
- **Material 9.7.6.** 원문 고정(9.7.7)과 패치 버전이 다르다. 영어 기준 빌드도 같은
  툴체인으로 만들므로 비교에는 영향이 없다.
- **nav 레이블과 쪽 제목.** 따로 번역되므로 같은 쪽의 nav 레이블과 `<h1>`이 다른 말로
  옮겨질 수 있다(원문도 둘이 다른 쪽이 많다). 원문에서 둘이 같은 쪽만 번역된 `<h1>`을
  nav에 쓰게 하려면 `prepare_site.py`의 nav 주입에서 맞춘다.
- **사이트 크기.** 쪽마다 전체 nav를 담아 산출물이 약 165MB다(영어 빌드 159MB). 줄여야
  하면 `prepare_site.py`에서 Material의 `navigation.prune`을 켠다.
