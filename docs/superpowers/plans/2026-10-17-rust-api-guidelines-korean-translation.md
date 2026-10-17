# Rust API 가이드라인 한국어 번역 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** rust-lang/api-guidelines의 mdBook 책을 yeokja로 한국어 번역하고, 영어 앵커가 살아 있는 HTML을 Pages 배포 배선까지 갖춘다.

**Architecture:** `projects/rust-api-guidelines/`에 원문 서브모듈을 고정하고, `markdown` 파서(`inline_tags`)로 `upstream/src/**/*.md`를 `ko/src/`로 번역한다. derive로 upstream 전체 위에 `ko/`를 겹친 트리에서 영어 원본과 한국어판을 모두 빌드하고, rustc-dev-guide/rust-forge의 스크립트로 앵커 보존·인쇄 페이지·링크 검사를 한다.

**Tech Stack:** yeokja(Rust CLI, `target/release/yeokja`), claude_code provider(`claude-opus-5-5`), mdBook 0.5.x, Python 3 표준 라이브러리, Nix devShell, GitHub Actions Pages.

**Spec:** `docs/superpowers/specs/2026-10-17-rust-api-guidelines-korean-translation-design.md`

## Global Constraints

- 원문 서브모듈: `projects/rust-api-guidelines/upstream`, URL `https://github.com/rust-lang/api-guidelines.git`, `shallow = true`, 커밋 `97a0969cb07fe4cabb0eed8a56234053f47d83dc`.
- 번역 범위: `upstream/src/**/*.md`만. `upstream/`은 절대 수정하지 않는다.
- provider: `type = "claude_code"`, `model = "claude-opus-5-5"`, 커스텀 `prompt_template` 없음.
- 용어는 rustc-dev-guide·rust-forge 용어집과 맞추고(크레이트, 트레이트, 라이프타임, 빌림), 도구·언어 이름과 newtype·unsafe는 원문 그대로. 문체는 `~합니다/~입니다`.
- 한국어 책 제목: `Rust API 가이드라인`.
- 가이드라인 식별자 `(C-XXX)`와 `<a id="c-xxx">` 앵커는 그대로 남아야 한다.
- 원문에 없던 깨진 로컬 링크·앵커 0개(`check_links.py` → `New: 0`).
- 라이선스: MIT OR Apache-2.0. 사이트에 `LICENSE-MIT`, `LICENSE-APACHE`를 싣는다.
- README 출처 표기는 AGENTS.md 규칙(yeokja 링크 `https://github.com/yeokja/yeokja`, 실제 모델, 학습 비허용 문장)을 따른다.
- `ko/`, `build/`, `dist/`, `__pycache__/`는 커밋하지 않는다.
- 커밋 메시지 형식: `[rust-api-guidelines] <type>: <한국어 요약>` + 본문, 마지막 줄 `Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>`. push하지 않는다.
- yeokja 바이너리: 저장소 루트의 `target/release/yeokja`. 아래 명령의 `$Y`는 `/Users/moreal/github/yeokja/yeokja/target/release/yeokja`다.

---

### Task 1: 프로젝트 골격, 원문 고정, 용어집, devShell

**Files:**
- Modify: `.gitmodules`(서브모듈 추가)
- Create: `projects/rust-api-guidelines/upstream`(gitlink)
- Create: `projects/rust-api-guidelines/yeokja.toml`
- Create: `projects/rust-api-guidelines/glossary.toml`
- Create: `projects/rust-api-guidelines/.gitignore`
- Create: `nix/projects/rust-api-guidelines.nix`

- [ ] **Step 1: 서브모듈 추가와 고정**

```bash
cd /Users/moreal/github/yeokja/yeokja
git submodule add --depth 1 https://github.com/rust-lang/api-guidelines.git projects/rust-api-guidelines/upstream
git config -f .gitmodules submodule.projects/rust-api-guidelines/upstream.shallow true
git -C projects/rust-api-guidelines/upstream log -1 --format='%H %cd'   # 97a0969c…
```

- [ ] **Step 2: `.gitignore`, `yeokja.toml`, `glossary.toml` 작성**

spec의 "번역 설정"과 "용어집과 표기 규칙" 절대로 작성한다. `[build.html]`은 Task 2에서 채운다.

- [ ] **Step 3: devShell 작성과 원본 빌드 확인**

`nix/projects/rust-api-guidelines.nix`에 `mdbook`, `python3`를 선언한다.

```bash
cd projects/rust-api-guidelines
nix develop path:../../nix#rust-api-guidelines -c mdbook build upstream --dest-dir "$PWD/build/original-check"
rm -rf build/original-check
```

- [ ] **Step 4: 분할 확인(번역 호출 없음)**

```bash
$Y status upstream/src | tail -8     # 15개 파일, 840개 세그먼트
$Y coverage upstream/src             # 건너뛴 구간은 코드 블록·링크 참조 정의뿐
```

---

### Task 2: 빌드 스크립트와 `[build.html]`

**Files:**
- Create: `projects/rust-api-guidelines/scripts/preserve_anchors.py`, `build_print.py`, `test_anchors.py`, `test_print.py`(rustc-dev-guide 사본)
- Create: `projects/rust-api-guidelines/scripts/check_links.py`(rust-forge 사본)
- Modify: `projects/rust-api-guidelines/yeokja.toml`(`[build.html]`)

- [ ] **Step 1: 스크립트 복사와 테스트**

```bash
cd projects/rust-api-guidelines
mkdir -p scripts
cp ../rustc-dev-guide/scripts/{preserve_anchors.py,build_print.py,test_anchors.py,test_print.py} scripts/
cp ../rust-forge/scripts/check_links.py scripts/
(cd scripts && python3 -m unittest test_anchors test_print)   # OK
```

- [ ] **Step 2: `[build.html]` 추가**

spec의 "빌드" 절 순서대로 작성한다(원본 빌드 → 한국어 빌드 → preserve_anchors → build_print ×2 → check_links → `mv book site` → `LICENSE-*` 복사).

---

### Task 3: 파일럿 번역과 빌드 검증

- [ ] **Step 1: 파일럿 번역**

```bash
$Y translate upstream/src/naming.md     # 표·앵커·링크 참조 정의가 모두 있는 파일
```

- [ ] **Step 2: 품질 확인**

`ko/src/naming.md`를 읽어 표 구조, `<a id>` 앵커, `(C-XXX)` 식별자, 링크 참조 정의, 격식체를 확인한다. 용어가 어긋나면 `glossary.toml`을 보강한다.

- [ ] **Step 3: 빌드**

```bash
nix develop path:../../nix#rust-api-guidelines -c $Y build html 2>&1 | tail -5   # New: 0
grep -c 'id="c-case"' dist/site/naming.html
```

---

### Task 4: 전체 번역과 용어 기준

- [ ] **Step 1: 전체 번역(백그라운드)**

```bash
$Y translate upstream/src 2>&1 | tee build/translate.log | tail -5
$Y status --check upstream/src
```

- [ ] **Step 2: 용어 기준(체크리스트 3절)**

```bash
$Y terms build upstream/src --dry-run
$Y terms build upstream/src
# terms.toml을 검토해 잘못 정해진 번역어를 고치거나 지우고 필요한 except를 둔다
$Y glossary adopt --prefer HEAD upstream/src
$Y translate upstream/src          # 남은 세그먼트만
$Y status --check upstream/src
```

- [ ] **Step 3: 기계 검사**

```bash
$Y evaluate --mechanical-only upstream/src 2>&1 | tail -30
for f in $(cd upstream/src && ls *.md); do
  for tok in '<a id=' '```' ']: ' '<br>'; do
    a=$(grep -oF "$tok" "upstream/src/$f" | wc -l); b=$(grep -oF "$tok" "ko/src/$f" | wc -l)
    [ "$a" = "$b" ] || echo "MISMATCH $f $tok upstream=$a ko=$b"
  done
done
```

Alignment/Format/Link 항목은 하나씩 확인해 실제 문제는 state에서 번역을 지우고 다시 번역한다.

- [ ] **Step 4: 빌드 재검증**

```bash
nix develop path:../../nix#rust-api-guidelines -c $Y build html 2>&1 | tail -3   # New: 0
```

---

### Task 5: 배포 배선, 랜딩, README

**Files:**
- Modify: `.github/pages-projects.json`(배열 끝)
- Modify: `.github/scripts/stage-pages.sh`(마지막 `overlay_site` 뒤)
- Modify: `.github/scripts/rebuild-translations.sh`(`rustc-dev-guide)` 분기 뒤)
- Modify: `.github/workflows/pr.yml`(rustc-dev-guide 테스트 스텝 뒤)
- Modify: `LICENSE.md`(표, 알파벳 순)
- Modify: `site/index.html`(`data-topics="rust"` 항목 뒤)
- Create: `projects/rust-api-guidelines/README.md`

- [ ] **Step 1: Pages 배선**

spec의 "배포 배선" 절대로 각 파일에 자기 항목만 추가한다.

- [ ] **Step 2: README 작성 전에 실제 모델 확인(AGENTS.md 규칙)**

```bash
grep -h '"model"' projects/rust-api-guidelines/state -r | sort | uniq -c
```

- [ ] **Step 3: README 작성**

spec의 "README" 절대로 작성한다.

- [ ] **Step 4: 검증**

```bash
cd /Users/moreal/github/yeokja/yeokja
python3 -m json.tool .github/pages-projects.json > /dev/null
python3 -m unittest discover -s .github/scripts -p 'test_*.py'
python3 -m unittest discover -s projects/rust-api-guidelines/scripts -p 'test_*.py'
rm -rf projects/rust-api-guidelines/ko && .github/scripts/rebuild-translations.sh rust-api-guidelines
(cd projects/rust-api-guidelines && nix develop path:../../nix#rust-api-guidelines -c ../../target/release/yeokja build html 2>&1 | tail -3)
git status --short   # ko/, dist/, build/, __pycache__가 보이지 않아야 한다
```

---

### 커밋 순서

1. `[rust-api-guidelines] feat: 원문 서브모듈·번역 설정·빌드 추가` — `.gitmodules`, `upstream`, `.gitignore`, `yeokja.toml`, `glossary.toml`, `scripts/`, `nix/projects/rust-api-guidelines.nix`
2. `[rust-api-guidelines] feat: 전체 한국어 번역` — `state/`, `terms.toml`, `glossary.toml` 변경분
3. `[rust-api-guidelines] feat: Pages 배포 배선과 README 추가` — `.github/`, `LICENSE.md`, `site/index.html`, `README.md`
4. `[rust-api-guidelines] docs: 번역 설계 spec과 plan 추가` — `docs/superpowers/`
