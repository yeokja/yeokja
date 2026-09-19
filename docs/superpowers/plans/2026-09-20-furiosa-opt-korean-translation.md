# furiosa-opt 문서 한국어 번역 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** furiosa-ai/furiosa-opt의 mdBook 책(`docs/`)을 yeokja로 한국어 번역하고, 영어 앵커가 살아 있는 HTML을 Pages 배포 배선까지 갖춘다.

**Architecture:** `projects/furiosa-opt/`에 원문 서브모듈을 고정하고, `markdown` 파서로 `upstream/docs/src/**/*.md`를 `ko/docs/src/`로 번역한다. derive로 upstream 전체 위에 `ko/`를 겹친 트리에서 영어 원본과 한국어판을 모두 빌드하고, rustc-dev-guide/webassembly-component-docs의 스크립트로 앵커 보존·인쇄 페이지·링크 검사를 한다.

**Tech Stack:** yeokja(Rust CLI, `target/release/yeokja`), claude_code provider(`claude-sonnet-5`), mdBook 0.5.x + mdbook-mermaid, Python 3 표준 라이브러리, Nix devShell, GitHub Actions Pages.

**Spec:** `docs/superpowers/specs/2026-09-20-furiosa-opt-korean-translation-design.md`

## Global Constraints

- 원문 서브모듈: `projects/furiosa-opt/upstream`, URL `https://github.com/furiosa-ai/furiosa-opt.git`, `shallow = true`, 구현 시점의 main HEAD에 고정(설계 시점 `9b9cf0fdc78df00cdc430eae725a5ad9084a735e`).
- 번역 범위: `upstream/docs/src/**/*.md`만. `upstream/`은 절대 수정하지 않는다.
- provider: `type = "claude_code"`, `model = "claude-sonnet-5"`, 커스텀 `prompt_template` 없음.
- 하드웨어 구성 요소·제품 고유명사는 영문 그대로(Fetch Engine, Tensor Unit, …). 문체는 `~합니다/~입니다`.
- 한국어 책 제목: `Tensor Contraction Processor 프로그래밍`.
- GFM 경고 표시 마커(`[!NOTE]` 등)는 영어 그대로 남아야 한다.
- 원문에 없던 깨진 로컬 링크·앵커 0개(`check_links.py` → `New: 0`).
- 라이선스: Apache-2.0. 사이트에 `LICENSE`, `NOTICE`를 싣고, 모든 본문 페이지에 비공식·수정본 고지를 넣는다.
- README 출처 표기는 AGENTS.md 규칙(yeokja 링크 `https://github.com/yeokja/yeokja`, 실제 모델, 학습 비허용 문장)을 따른다.
- `ko/`, `build/`, `dist/`, `__pycache__/`는 커밋하지 않는다.
- 커밋 메시지 형식: `[furiosa-opt] <type>: <한국어 요약>` + 본문, 마지막 줄 `Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>`. push하지 않는다.
- yeokja 바이너리: 저장소 루트의 `target/release/yeokja`(2026-09-20에 main 기준으로 다시 빌드함). 아래 명령의 `$Y`는 `/Users/moreal/github/yeokja/yeokja/target/release/yeokja`다.

---

### Task 1: 프로젝트 골격, 원문 고정, 용어집

**Files:**
- Modify: `.gitmodules`(서브모듈 추가)
- Create: `projects/furiosa-opt/upstream`(gitlink)
- Create: `projects/furiosa-opt/yeokja.toml`
- Create: `projects/furiosa-opt/glossary.toml`
- Create: `projects/furiosa-opt/.gitignore`

**Interfaces:**
- Produces: yeokja 프로젝트 루트 `projects/furiosa-opt`, source 경로 `upstream/docs/src`, 출력 `ko/docs/src/{path}`. 이후 태스크는 이 경로를 그대로 쓴다.

- [ ] **Step 1: 서브모듈 추가와 고정**

```bash
cd /Users/moreal/github/yeokja/yeokja
git submodule add https://github.com/furiosa-ai/furiosa-opt.git projects/furiosa-opt/upstream
git config -f .gitmodules submodule.projects/furiosa-opt/upstream.shallow true
git -C projects/furiosa-opt/upstream fetch origin main
git -C projects/furiosa-opt/upstream checkout origin/main
git -C projects/furiosa-opt/upstream log -1 --format='%H %cd'   # 이 해시를 README(Task 6)에 적는다
```

- [ ] **Step 2: `.gitignore` 작성**

`projects/furiosa-opt/.gitignore`:

```gitignore
ko/
dist/
__pycache__/
```

(`build/`는 저장소 루트 `.gitignore`가 이미 제외한다.)

- [ ] **Step 3: `yeokja.toml` 작성 (빌드 섹션은 Task 3에서 추가)**

```toml
[project]
source_lang = "en"
target_lang = "ko"
glossary = "glossary.toml"
state_dir = "state"

[[sources]]
path = "upstream/docs/src"
pattern = "**/*.md"
parser = "markdown"
output = "ko/docs/src/{path}"

[derive]
base = "upstream"

[[derive.overlay]]
path = "ko"
require_base = true

[provider]
type = "claude_code"
model = "claude-sonnet-5"

[evaluation]
auto_evaluate = true
style_evaluate = false
max_retries = 3

[translation]
concurrency = 8
batch_segments = 32
```

- [ ] **Step 4: `glossary.toml` 작성**

```toml
# Programming Tensor Contraction Processors 용어집
# 하드웨어 구성 요소·제품 고유명사는 영문 그대로 둔다(코드 API와 대응).

[terms."Tensor Contraction Processor"]
translation = "Tensor Contraction Processor"
note = "FuriosaAI 프로세서 아키텍처 고유명사"
[terms.TCP]
translation = "TCP"
note = "Tensor Contraction Processor 약어"
[terms.RNGD]
translation = "RNGD"
note = "제품명"
[terms."Tensor Unit"]
translation = "Tensor Unit"
note = "하드웨어 구성 요소 고유명사"
[terms."Fetch Engine"]
translation = "Fetch Engine"
note = "하드웨어 구성 요소 고유명사"
[terms."Commit Engine"]
translation = "Commit Engine"
note = "하드웨어 구성 요소 고유명사"
[terms."DMA Engine"]
translation = "DMA Engine"
note = "하드웨어 구성 요소 고유명사"
[terms.Sequencer]
translation = "Sequencer"
note = "하드웨어 구성 요소 고유명사(Fetch/Commit Sequencer 포함)"
[terms."Contraction Engine"]
translation = "Contraction Engine"
note = "하드웨어 구성 요소 고유명사"
[terms."Vector Engine"]
translation = "Vector Engine"
note = "하드웨어 구성 요소 고유명사"
[terms."Transpose Engine"]
translation = "Transpose Engine"
note = "하드웨어 구성 요소 고유명사"
[terms."Switch Engine"]
translation = "Switch Engine"
note = "하드웨어 구성 요소 고유명사"
[terms."Cast Engine"]
translation = "Cast Engine"
note = "하드웨어 구성 요소 고유명사"
[terms."Collect Engine"]
translation = "Collect Engine"
note = "하드웨어 구성 요소 고유명사"
[terms."Fetch Adapter"]
translation = "Fetch Adapter"
note = "하드웨어 구성 요소 고유명사"
[terms."Commit Adapter"]
translation = "Commit Adapter"
note = "하드웨어 구성 요소 고유명사"
[terms."Stream Adapter"]
translation = "Stream Adapter"
note = "하드웨어 구성 요소 고유명사"
[terms."Lane Folder"]
translation = "Lane Folder"
note = "하드웨어 구성 요소 고유명사"
[terms."Packet Reducer"]
translation = "Packet Reducer"
note = "하드웨어 구성 요소 고유명사"
[terms."Time Reducer"]
translation = "Time Reducer"
note = "하드웨어 구성 요소 고유명사"
[terms."Inter-Slice Reducer"]
translation = "Inter-Slice Reducer"
note = "하드웨어 구성 요소 고유명사"
[terms."Intra-Slice Chain"]
translation = "Intra-Slice Chain"
note = "하드웨어 구성 요소 고유명사"
[terms."Intra-Slice Reduce"]
translation = "Intra-Slice Reduce"
note = "하드웨어 구성 요소 고유명사"
[terms.VCG]
translation = "VCG"
note = "Vector Engine 구성 요소 약어"
[terms."Register File"]
translation = "Register File"
note = "하드웨어 구성 요소 고유명사(TRF/VRF 등 약어 포함)"
[terms.DM]
translation = "DM"
note = "Data Memory 약어, 하드웨어 고유명사"
[terms.Chip]
translation = "Chip"
note = "하드웨어 계층 이름(대문자로 쓰인 경우)"
[terms.Cluster]
translation = "Cluster"
note = "하드웨어 계층 이름(대문자로 쓰인 경우)"
[terms.Slice]
translation = "Slice"
note = "하드웨어 계층 이름(대문자로 쓰인 경우)"
[terms."Kernel Optimizer"]
translation = "Kernel Optimizer"
note = "도구 이름"
[terms."Schedule Viewer"]
translation = "Schedule Viewer"
note = "도구 이름"

[terms.tensor]
translation = "텐서"
[terms.kernel]
translation = "커널"
[terms.mapping]
translation = "매핑"
[terms.schedule]
translation = "스케줄"
[terms.scheduling]
translation = "스케줄링"
[terms.tiling]
translation = "타일링"
[terms.packet]
translation = "패킷"
[terms.stream]
translation = "스트림"
[terms.axis]
translation = "축"
[terms.contraction]
translation = "축약"
note = "텐서 축약. 고유명사 Tensor Contraction Processor/Contraction Engine은 제외"
[terms.reduction]
translation = "리덕션"
[terms.pipeline]
translation = "파이프라인"
[terms.throughput]
translation = "처리량"
[terms.latency]
translation = "지연 시간"
[terms."memory hierarchy"]
translation = "메모리 계층"
```

- [ ] **Step 5: 분할 확인(번역 호출 없음)**

```bash
cd projects/furiosa-opt
$Y status upstream/docs/src | tail -5          # 53개 파일이 미번역으로 보여야 한다
$Y coverage upstream/docs/src                   # 파서가 건너뛴 5줄 이상 구간 확인
$Y inspect upstream/docs/src | head -80         # 표와 선택 규칙 확인
```

기대: 53개 파일이 모두 대상이다. coverage가 건너뛴 구간은 코드 블록, mermaid, `{{#include}}`, front matter뿐이다. 표 중 식별자·수치만 있는 열이 번역 대상으로 잡히면 `[[tables]]` 규칙을 추가한다. 형식은 `projects/*/yeokja.toml`에서 `[[tables]]`를 쓰는 예를 `grep -l '\[\[tables\]\]' projects/*/yeokja.toml`로 찾아 따른다. 표 구조가 번역 대상이어도 문제없으면 추가하지 않는다(YAGNI).

- [ ] **Step 6: 커밋**

```bash
cd /Users/moreal/github/yeokja/yeokja
git add .gitmodules projects/furiosa-opt/upstream projects/furiosa-opt/.gitignore projects/furiosa-opt/yeokja.toml projects/furiosa-opt/glossary.toml
git commit -m "[furiosa-opt] feat: 원문 서브모듈과 yeokja 번역 설정 추가" -m "furiosa-ai/furiosa-opt의 docs/ mdBook 책을 번역 대상으로 등록하고 하드웨어 고유명사를 영문으로 유지하는 용어집을 둔다. (#1)" -m "Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>"
```

---

### Task 2: Nix devShell과 원본 빌드 확인

**Files:**
- Create: `nix/projects/furiosa-opt.nix`

**Interfaces:**
- Consumes: Task 1의 `projects/furiosa-opt/upstream`.
- Produces: devShell `furiosa-opt`(`mdbook`, `mdbook-mermaid`, `python3`). Task 3·4·6이 `nix develop path:../../nix#furiosa-opt -c ...`로 쓴다.

- [ ] **Step 1: devShell 작성**

```nix
# furiosa-opt: mdbook + mdbook-mermaid(원문 book.toml의 [preprocessor.mermaid]) +
# 빌드 스크립트가 부르는 python3(scripts/*.py — 표준 라이브러리만 사용, venv 불필요).
{
  pkgs,
  lib,
  system,
}:
pkgs.mkShell {
  packages = [
    pkgs.mdbook
    pkgs.mdbook-mermaid
    pkgs.python3
  ];
}
```

- [ ] **Step 2: 원본 빌드로 호환성 확인**

```bash
cd /Users/moreal/github/yeokja/yeokja/projects/furiosa-opt
nix develop path:../../nix#furiosa-opt -c sh -c 'mdbook --version; mdbook-mermaid --version; mdbook build upstream/docs --dest-dir "$PWD/build/original-check"'
grep -l 'class="mermaid"' -r build/original-check | head -3
git -C upstream status --short   # 비어 있어야 한다(원문 트리를 건드리지 않음)
```

기대: 빌드가 성공하고 mermaid 블록이 `<pre class="mermaid">`로 렌더링된다. upstream 작업 트리는 깨끗하다.

`mdbook-mermaid`가 mdbook 0.5와 맞지 않아 전처리기 오류가 나면, `nix/projects/webassembly-component-docs.nix`의 mdbook-tabs처럼 `pkgs.rustPlatform.buildRustPackage` + `pkgs.fetchCrate`로 호환 버전(crates.io의 최신 `mdbook-mermaid`)을 고정한다. 해시는 `lib.fakeHash`로 두고 `nix develop`이 보고하는 값으로 채운다. 그 뒤 이 스텝을 다시 실행한다.

- [ ] **Step 3: 정리와 커밋**

```bash
rm -rf build/original-check
cd /Users/moreal/github/yeokja/yeokja
git add nix/projects/furiosa-opt.nix
git commit -m "[furiosa-opt] feat: mdBook 빌드용 Nix devShell 추가" -m "Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>"
```

---

### Task 3: 빌드 스크립트와 `[build.html]`

**Files:**
- Create: `projects/furiosa-opt/scripts/preserve_anchors.py`(rustc-dev-guide 사본)
- Create: `projects/furiosa-opt/scripts/build_print.py`(rustc-dev-guide 사본)
- Create: `projects/furiosa-opt/scripts/test_anchors.py`, `test_print.py`(rustc-dev-guide 사본)
- Create: `projects/furiosa-opt/scripts/check_links.py`(webassembly-component-docs 사본)
- Create: `projects/furiosa-opt/scripts/finish_html.py`
- Test: `projects/furiosa-opt/scripts/test_finish.py`
- Modify: `projects/furiosa-opt/yeokja.toml`(`[build.html]` 추가)

**Interfaces:**
- Consumes: Task 2의 devShell.
- Produces: `finish_html.add_footer(text: str, depth: int) -> str`, `finish_html.finish(root: Path) -> None`. `yeokja build html`이 `dist/site/`를 만든다.

- [ ] **Step 1: 기존 스크립트 복사와 테스트 확인**

```bash
cd /Users/moreal/github/yeokja/yeokja/projects/furiosa-opt
mkdir -p scripts
cp ../rustc-dev-guide/scripts/{preserve_anchors.py,build_print.py,test_anchors.py,test_print.py} scripts/
cp ../webassembly-component-docs/scripts/check_links.py scripts/
(cd scripts && python3 -m unittest test_anchors test_print)
```

기대: `OK`.

- [ ] **Step 2: `test_finish.py`를 먼저 작성(실패 확인)**

```python
from pathlib import Path
import tempfile
import unittest
from finish_html import add_footer, finish


class FinishTests(unittest.TestCase):
    def test_footer_goes_inside_main_with_root_relative_license_links(self):
        result = add_footer('<main><h1>제목</h1></main>', depth=2)
        self.assertIn('<footer class="translation-credit">', result)
        self.assertLess(result.index('<footer'), result.index('</main>'))
        self.assertIn('href="../../LICENSE"', result)
        self.assertIn('href="../../NOTICE"', result)
        self.assertIn('https://github.com/yeokja/yeokja', result)
        self.assertIn('Anthropic 사의 <code>claude-sonnet-5</code> 모델을 활용하여', result)
        self.assertIn('학습을 모두 비허용한 상태로 작업하였습니다', result)
        self.assertIn('비공식 번역', result)

    def test_page_without_main_is_unchanged(self):
        self.assertEqual(add_footer('<html>redirect</html>', depth=0), '<html>redirect</html>')

    def test_finish_uses_each_page_depth(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / 'a').mkdir()
            (root / 'index.html').write_text('<main></main>')
            (root / 'a' / 'b.html').write_text('<main></main>')
            finish(root)
            self.assertIn('href="LICENSE"', (root / 'index.html').read_text())
            self.assertIn('href="../LICENSE"', (root / 'a' / 'b.html').read_text())


if __name__ == '__main__':
    unittest.main()
```

Run: `(cd scripts && python3 -m unittest test_finish)`
Expected: FAIL(`ModuleNotFoundError: No module named 'finish_html'`).

- [ ] **Step 3: `finish_html.py` 구현**

```python
"""Add visible attribution, license, and modification notice to each reading page."""
from pathlib import Path
import sys

FOOTER = '''<footer class="translation-credit"><hr>
<p>원문 © FuriosaAI ·
<a href="https://github.com/furiosa-ai/furiosa-opt">Programming Tensor Contraction Processors</a> ·
<a href="{root}LICENSE">Apache License 2.0</a> · <a href="{root}NOTICE">NOTICE</a><br>
<a href="https://github.com/yeokja/yeokja">yeokja</a> 한국어 번역 ·
Anthropic 사의 <code>claude-sonnet-5</code> 모델을 활용하여 번역되었으며
학습을 모두 비허용한 상태로 작업하였습니다 ·
이 문서는 원문을 수정한 비공식 번역본이며 Apache License 2.0으로 제공합니다.</p></footer>
'''


def add_footer(text, depth):
    if '</main>' not in text:
        return text
    return text.replace('</main>', FOOTER.format(root='../' * depth) + '</main>', 1)


def finish(root):
    root = Path(root)
    for page in root.rglob('*.html'):
        depth = len(page.relative_to(root).parts) - 1
        page.write_text(add_footer(page.read_text(), depth))


if __name__ == '__main__':
    finish(sys.argv[1])
```

Run: `(cd scripts && python3 -m unittest test_anchors test_print test_finish)`
Expected: `OK`.

- [ ] **Step 4: `[build.html]` 추가**

`projects/furiosa-opt/yeokja.toml`의 `[derive.overlay]` 블록 뒤, `[provider]` 앞에 넣는다.

```toml
[build.html]
command = '''
set -e
mkdir -p "$YEOKJA_ROOT/build"
rm -rf "$YEOKJA_ROOT/build/original"
mdbook build "$YEOKJA_ROOT/upstream/docs" --dest-dir "$YEOKJA_ROOT/build/original"
MDBOOK_BOOK__LANGUAGE=ko MDBOOK_BOOK__TITLE='Tensor Contraction Processor 프로그래밍' mdbook build docs
python3 "$YEOKJA_ROOT/scripts/preserve_anchors.py" "$YEOKJA_ROOT/build/original" docs/book
python3 "$YEOKJA_ROOT/scripts/build_print.py" docs/book
python3 "$YEOKJA_ROOT/scripts/build_print.py" "$YEOKJA_ROOT/build/original"
cp "$YEOKJA_ROOT/upstream/LICENSE" "$YEOKJA_ROOT/upstream/NOTICE" docs/book/
python3 "$YEOKJA_ROOT/scripts/finish_html.py" docs/book
python3 "$YEOKJA_ROOT/scripts/check_links.py" "$YEOKJA_ROOT/build/original" docs/book
mv docs/book site
'''
outputs = ["site"]
```

`LICENSE`/`NOTICE`를 `check_links.py`보다 먼저 복사해야 하단 고지의 링크가 깨진 것으로 잡히지 않는다.

- [ ] **Step 5: 커밋(빌드 검증은 Task 4에서 번역 일부가 생긴 뒤)**

```bash
cd /Users/moreal/github/yeokja/yeokja
git add projects/furiosa-opt/scripts projects/furiosa-opt/yeokja.toml
git commit -m "[furiosa-opt] feat: 앵커 보존·인쇄 페이지·링크 검사를 포함한 HTML 빌드 추가" -m "rustc-dev-guide와 webassembly-component-docs의 스크립트를 복사해 영어 앵커를 보존하고, 원문에 없던 깨진 링크가 생기면 빌드를 실패시킨다. 하단 고지로 Apache-2.0 변경 고지와 번역 출처를 싣는다." -m "Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>"
```

---

### Task 4: 파일럿 번역과 빌드 검증

**Files:**
- Create: `projects/furiosa-opt/state/upstream/docs/src/introduction.md.yeokja.json` 외 파일럿 대상 state
- Modify: `projects/furiosa-opt/glossary.toml`(필요할 때만)

**Interfaces:**
- Consumes: Task 1~3 전부.
- Produces: 검증된 파이프라인. Task 5는 같은 명령으로 전체를 번역한다.

- [ ] **Step 1: 파일럿 대상 번역**

경고 표시가 있는 파일 하나를 함께 고른다.

```bash
cd /Users/moreal/github/yeokja/yeokja/projects/furiosa-opt
grep -rlE '\[!(NOTE|TIP|WARNING)\]' upstream/docs/src | head -3
$Y translate upstream/docs/src/introduction.md
$Y translate upstream/docs/src/moving-tensors/fetch-engine.md
$Y translate <위 grep 결과 중 하나>
```

- [ ] **Step 2: 파일럿 품질 확인**

```bash
for f in introduction.md moving-tensors/fetch-engine.md <경고 표시 파일>; do
  for tok in '[!NOTE]' '[!TIP]' '[!WARNING]' '\\(' '\\)' '$$' '{{#include'; do
    a=$(grep -oF "$tok" "upstream/docs/src/$f" | wc -l); b=$(grep -oF "$tok" "ko/docs/src/$f" | wc -l)
    [ "$a" = "$b" ] || echo "MISMATCH $f $tok upstream=$a ko=$b"
  done
done
$Y evaluate --mechanical-only upstream/docs/src/moving-tensors/fetch-engine.md
```

기대: MISMATCH 출력이 없다. 번역문을 직접 읽고 다음을 확인한다.

- 하드웨어 이름(Fetch Engine, Tensor Unit, DM 등)이 영문 그대로다.
- 격식체를 쓴다.
- 링크 URL과 `#fragment`가 원문과 같다.

용어가 어긋나면 `glossary.toml`을 보강하고, 해당 state 파일을 지운 뒤 다시 번역한다.

- [ ] **Step 3: 전체 빌드와 앵커 검증**

```bash
nix develop path:../../nix#furiosa-opt -c $Y build html 2>&1 | tail -5
python3 - <<'EOF'
from pathlib import Path
page = Path('dist/site/moving-tensors/fetch-engine.html').read_text()
for anchor in ('constraints', 'axis-lifting', 'optimizations'):
    assert f'id="{anchor}"' in page, anchor
print('anchors ok')
EOF
```

기대: 빌드 로그에 `Original unresolved local links: N Korean: N New: 0`이 나오고, `anchors ok`가 출력된다. 번역되지 않은 페이지는 영어 원문으로 빌드되므로(derive base) 이 단계에서도 전체 사이트가 나와야 한다.

- [ ] **Step 4: 커밋**

```bash
cd /Users/moreal/github/yeokja/yeokja
git add projects/furiosa-opt/state projects/furiosa-opt/glossary.toml
git commit -m "[furiosa-opt] feat: 파일럿 문서 번역" -m "Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>"
```

---

### Task 5: 전체 번역

**Files:**
- Create/Modify: `projects/furiosa-opt/state/**/*.yeokja.json`

- [ ] **Step 1: 전체 번역 실행(장시간, 백그라운드 권장)**

```bash
cd /Users/moreal/github/yeokja/yeokja/projects/furiosa-opt
$Y translate upstream/docs/src 2>&1 | tee build/translate.log | tail -20
```

중단되면 같은 명령을 다시 실행한다. 이미 번역된 세그먼트는 건너뛴다.

- [ ] **Step 2: 완결성과 기계 검사**

```bash
$Y status --check upstream/docs/src
$Y evaluate --mechanical-only upstream/docs/src 2>&1 | tail -30
for f in $(cd upstream/docs/src && find . -name '*.md'); do
  for tok in '[!NOTE]' '[!TIP]' '[!WARNING]' '[!IMPORTANT]' '[!CAUTION]' '\\(' '\\)' '$$' '{{#include' '```'; do
    a=$(grep -oF "$tok" "upstream/docs/src/$f" | wc -l); b=$(grep -oF "$tok" "ko/docs/src/$f" | wc -l)
    [ "$a" = "$b" ] || echo "MISMATCH $f $tok upstream=$a ko=$b"
  done
done
```

기대: `status --check` 종료 코드 0, MISMATCH 없음, evaluate 이슈 없음 또는 검토 후 무해 판정. 문제가 있는 세그먼트는 원인을 확인한 뒤 해당 state 파일의 `translation`을 직접 고치거나, 파일을 지우고 다시 번역한다. 직접 고쳤으면 `$Y translate <파일>`로 `ko/`를 재구성한다.

- [ ] **Step 3: 빌드 재검증**

```bash
nix develop path:../../nix#furiosa-opt -c $Y build html 2>&1 | tail -3   # New: 0
```

- [ ] **Step 4: 커밋**

```bash
cd /Users/moreal/github/yeokja/yeokja
git add projects/furiosa-opt/state projects/furiosa-opt/glossary.toml
git commit -m "[furiosa-opt] feat: docs/ 전체 한국어 번역" -m "Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>"
```

---

### Task 6: 배포 배선, 랜딩, README

**Files:**
- Modify: `.github/pages-projects.json`
- Modify: `.github/scripts/stage-pages.sh`(rustc-dev-guide `overlay_site` 줄 근처)
- Modify: `.github/scripts/rebuild-translations.sh`(`rustc-dev-guide)` 분기 뒤)
- Modify: `.github/workflows/pr.yml`(`Test rustc-dev-guide heading anchors` 스텝 뒤)
- Modify: `site/index.html`(`id="hardware"` 섹션의 `<ul class="works">` 끝)
- Create: `projects/furiosa-opt/README.md`

- [ ] **Step 1: Pages 배선**

`.github/pages-projects.json` 배열 끝(`raytracing` 항목 뒤)에 추가:

```json
  {
    "project": "furiosa-opt",
    "target": "html",
    "artifact": "dist-furiosa-opt",
    "artifact_path": "projects/furiosa-opt/dist"
  }
```

`.github/scripts/stage-pages.sh`의 `overlay_site "dist-rustc-dev-guide" "site" "rustc-dev-guide"` 다음 줄:

```bash
overlay_site "dist-furiosa-opt" "site" "furiosa-opt"
```

`.github/scripts/rebuild-translations.sh`의 `rustc-dev-guide)` 분기 뒤:

```bash
  furiosa-opt)
    # docs/ mdBook 책 전체를 한 번에 확인해 upstream이 새로 추가한 문서가
    # state 없이 빠지는 경우도 배포를 막습니다.
    "$yeokja" translate upstream/docs/src
    "$yeokja" status --check upstream/docs/src
    ;;
```

`.github/workflows/pr.yml`의 rustc-dev-guide 테스트 스텝 뒤:

```yaml
      - name: Test furiosa-opt heading anchors and footer
        run: python3 -m unittest discover -s projects/furiosa-opt/scripts -p 'test_*.py'
```

- [ ] **Step 2: 랜딩 항목**

`site/index.html`의 `id="hardware"` 섹션 `<ul class="works">` 마지막 `</li>` 뒤에 추가:

```html
      <li class="work" data-topics="hardware">
        <div class="subjects"><span>하드웨어</span></div>
        <h4><a class="work-title" href="furiosa-opt/">Tensor Contraction Processor 프로그래밍</a></h4>
        <p class="work-desc">FuriosaAI의 Tensor Contraction Processor에서 텐서를 매핑·이동·계산하는 커널을 Rust로 작성하고 스케줄링하는 방법을 설명합니다.</p>
        <p class="work-meta">원문 <a href="https://github.com/furiosa-ai/furiosa-opt">furiosa-ai/furiosa-opt</a><span class="tag">Apache 2.0</span><span class="tag">기계 번역</span></p>
      </li>
```

- [ ] **Step 3: README 작성 전에 실제 모델 확인(AGENTS.md 규칙)**

```bash
cd /Users/moreal/github/yeokja/yeokja
git log -p --format='%h %cd' -- projects/furiosa-opt/yeokja.toml | grep -E '^[0-9a-f]{7} |model|type ='
```

provider가 처음부터 `claude_code`/`claude-sonnet-5` 하나였다면 모든 세그먼트가 그 모델로 번역된 것이다. 도중에 바뀌었다면 `state/**/*.yeokja.json`의 `translated_at`을 변경 커밋 시각과 비교해 실제 모델을 적는다.

- [ ] **Step 4: README 작성**

`projects/furiosa-opt/README.md`:

````markdown
# Tensor Contraction Processor 프로그래밍 (한국어 번역)

[furiosa-ai/furiosa-opt](https://github.com/furiosa-ai/furiosa-opt)의 mdBook 문서
*Programming Tensor Contraction Processors*(`docs/`)를 한국어로 옮긴 비공식
번역입니다. [yeokja](https://github.com/yeokja/yeokja)와 함께 Anthropic 사의
`claude-sonnet-5` 모델을 활용하여 번역되었으며 학습을 모두 비허용한 상태로
작업하였습니다.

## 범위

- 원문: `upstream/` 서브모듈, 커밋 `<Task 1 Step 1의 해시>`
- 번역 대상: `upstream/docs/src/**/*.md` (코드 블록, mermaid, 수식, `{{#include}}`는 원문 유지)
- 저장소의 README, `CHANGES.md`, `skills/` 등 책 밖 문서는 제외합니다.
- Fetch Engine, Tensor Unit 같은 하드웨어 구성 요소 이름은 코드 API와의 대응을 위해 영문으로 둡니다.

## 라이선스

원문은 Apache License 2.0입니다. 이 번역은 원문을 수정한 파생 저작물로서 같은
Apache License 2.0으로 제공하며, 배포 사이트에 원문의 `LICENSE`와 `NOTICE`를
함께 싣고 모든 페이지 하단에 번역·수정 사실을 표시합니다. FuriosaAI의
공식 문서가 아니며 상표에 대한 권리를 주장하지 않습니다.

## 재현

```sh
cd projects/furiosa-opt
../../target/release/yeokja translate upstream/docs/src      # 누락 세그먼트 번역 및 ko/ 재구성
../../target/release/yeokja status --check upstream/docs/src
nix develop path:../../nix#furiosa-opt -c ../../target/release/yeokja build html   # dist/site
```

`state/`는 번역의 진실의 원천이므로 커밋합니다. `ko/`, `build/`, `dist/`는
`state/`와 원문에서 언제든 재생성되므로 커밋하지 않습니다.

빌드는 영어 원본과 한국어판을 함께 만들고, 원문의 영어 제목 앵커를 한국어
페이지에 보존합니다(`scripts/preserve_anchors.py`). 원문에 없던 깨진 로컬
링크가 생기면 `scripts/check_links.py`가 빌드를 실패시킵니다.
````

`<Task 1 Step 1의 해시>`는 실제 고정 커밋 해시로 바꾼다.

- [ ] **Step 5: 검증**

```bash
cd /Users/moreal/github/yeokja/yeokja
python3 -m json.tool .github/pages-projects.json > /dev/null
python3 -m unittest discover -s .github/scripts -p 'test_*.py'
python3 -m unittest discover -s projects/furiosa-opt/scripts -p 'test_*.py'
rm -rf projects/furiosa-opt/ko && .github/scripts/rebuild-translations.sh furiosa-opt
(cd projects/furiosa-opt && nix develop path:../../nix#furiosa-opt -c ../../target/release/yeokja build html 2>&1 | tail -3)
git status --short   # ko/, dist/, build/, __pycache__가 보이지 않아야 한다
```

기대: 모든 테스트가 통과하고, `rebuild-translations.sh`가 state만으로 `ko/`를 재구성해 `status --check`를 통과하며, 빌드는 `New: 0`이다.

- [ ] **Step 6: 커밋**

```bash
git add .github/pages-projects.json .github/scripts/stage-pages.sh .github/scripts/rebuild-translations.sh .github/workflows/pr.yml site/index.html projects/furiosa-opt/README.md
git commit -m "[furiosa-opt] feat: Pages 배포 배선과 README 추가" -m "Pages 빌드 매트릭스·스테이징·CI 재구성 분기·PR 테스트·랜딩 항목을 추가하고 README에 범위, 라이선스, 재현 방법, 출처를 적는다. (#1)" -m "Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>"
```
