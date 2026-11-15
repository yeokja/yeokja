# Linux 커널 내부 구조 (Linux Kernel Internals 한국어 번역)

Laveesh Bansal의 [Linux Kernel Internals](https://kernel-internals.org/)([laveeshb/linux-kernel-internals](https://github.com/laveeshb/linux-kernel-internals))를
한국어로 옮긴 비공식 번역입니다. [yeokja](https://github.com/yeokja/yeokja)와 함께 Anthropic 사의
`claude-opus-5-5` 모델을 활용하여 번역되었으며 학습을 모두 비허용한 상태로 작업하였습니다.

## 범위

- 원문: `upstream/` 서브모듈, 커밋 `6ee74a0b2e5f49c75d107ac4935369f73a004546`(2026-10-23)
- 번역 대상: `upstream/docs/**/*.md` 498쪽(홈 `index.md`, `about.md`, `changelog.md` 포함)과
  `mkdocs.yml`의 nav 레이블 580개
- 제외: `contributing.md`(원문 저장소에 기여하는 절차. 영어 그대로 싣고 제목 아래에 그 이유를 적습니다),
  루트 `README.md`(사이트에 쓰이지 않습니다)
- 원문 그대로 두는 것: 코드 블록(주석 포함), ASCII 다이어그램과 커널 로그, mermaid 다이어그램의 레이블,
  커널 식별자(함수·구조체·sysctl·Kconfig 이름), 커밋 해시, CVE 번호, 사람 이름, 인용된 커밋·패치·글의 제목

## 라이선스

원문은 Laveesh Bansal이 문서는 [CC BY-SA 4.0](https://creativecommons.org/licenses/by-sa/4.0/)으로,
코드 스니펫은 GPL-2.0으로 공개했습니다(`upstream/LICENSE`, `LICENSE-CC-BY-SA-4.0`, `LICENSE-GPL-2.0`).
이 번역은 문서를 수정한 개작물로서 같은 CC BY-SA 4.0으로 제공하며, 코드 스니펫은 고치지 않고 GPL-2.0
그대로 싣습니다. Tux 로고는 Larry Ewing(원본)과 Simon Budig(SVG)의 것입니다. 사이트의 모든 쪽 하단에
원저작자·원문 링크·라이선스·번역 사실을 표시하고, 라이선스 전문 세 파일을 사이트 루트에 함께 싣습니다.

원문은 확정적인 참고 자료가 아닌 커뮤니티 학습 자료이며 오류가 있을 수 있다고 스스로 밝힙니다. 번역은
그 오류를 그대로 옮길 수 있고 번역 자체의 오류도 있을 수 있습니다. 확실하지 않을 때는
[공식 커널 문서](https://docs.kernel.org/)와 소스 코드를 확인하십시오. 이 번역은 원저작자가 검토하거나
보증하지 않았으며, Linux Kernel Internals의 공식 사이트가 아닙니다.

## 재현

```sh
cargo build --release -p yeokja-cli
git submodule update --init projects/linux-kernel-internals/upstream
cd projects/linux-kernel-internals
python3 scripts/sources.py extract --check
../../target/release/yeokja translate upstream/docs
../../target/release/yeokja translate normalized/docs
../../target/release/yeokja translate nav
../../target/release/yeokja status --check upstream/docs
../../target/release/yeokja status --check normalized/docs
../../target/release/yeokja status --check nav
nix develop path:../../nix#linux-kernel-internals -c ../../target/release/yeokja build html   # dist/site
```

`state/`는 번역의 진실의 원천이므로 커밋합니다. `ko/`, `ko-nav/`, `build/`, `dist/`는 재생성되므로
커밋하지 않습니다. CI(`.github/scripts/rebuild-translations.sh linux-kernel-internals`)는 파생 소스와
미번역 세그먼트를 먼저 검사한 뒤 state에서 `ko/`와 `ko-nav/`를 재구성하므로 번역 provider를 부르지
않습니다.

## 구성

- 파서: `mkdocs`(`crates/parser-mkdocs`). 이 사이트의 `markdown_extensions`에 맞춰 `math = false`
  (`$`는 글자), `definition_lists = true`(war-stories 머리의 정의 목록), `inline_tags = true`(강조·링크를
  태그로 주고받음)로 읽습니다.
- 파생 소스: `scripts/sources.py extract`가 원문에서 두 가지를 만들어 커밋해 둡니다.
  `extract --check`는 커밋된 것이 고정된 원문과 맞는지 검사합니다.
  - `nav/nav.md`: `mkdocs.yml`의 nav 레이블을 nav 순서대로 적은 목록. 번역(`ko-nav/nav.md`)은
    `scripts/prepare_site.py`가 원문 nav의 같은 줄에 되돌려 넣으므로 쪽 목록과 순서는 원문 그대로입니다.
  - `normalized/docs/`: admonition 머리를 공백 없이 쓴 쪽(`!!!note`; `mm/numa-reclaim.md`,
    `mm/tlb-optimization.md`)에 공백을 넣은 사본. Python-Markdown은 두 표기를 모두 admonition으로 읽지만
    파서는 공백 있는 표기만 읽으므로, 이 두 쪽은 사본에서 번역합니다.
- 빌드: 같은 툴체인으로 영어 원문을 먼저 빌드하면서 `overlay/hooks/anchors.py`가 쪽마다 제목 ID를
  기록하고, 한국어 빌드에서 같은 hook이 번역된 제목에 그 ID를 입힙니다. 제목 수가 다르면 빌드가
  실패합니다. `scripts/prepare_site.py`가 원문 `mkdocs.yml`의 사이트 이름·언어·검색·푸터·nav 레이블을
  바꿉니다.
- 검사: `scripts/check_site.py`가 두 빌드의 쪽 목록, 쪽별 구조(제목과 제목 ID, 정의 목록, admonition,
  mermaid, 코드 블록, 표, 목록 수)를 비교하고, 원문 빌드에 없던 깨진 로컬 링크·앵커가 생기거나 nav가
  번역되지 않았으면 실패합니다.
- 스크립트 테스트: `nix develop path:../../nix#linux-kernel-internals -c python3 -m unittest discover -s scripts`
