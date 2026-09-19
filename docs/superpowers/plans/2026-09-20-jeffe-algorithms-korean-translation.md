# Jeff Erickson *Algorithms* 한국어 번역 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** PDF로만 공개된 Jeff Erickson의 *Algorithms*에서 영어 LaTeX 원고를 복원·검증하고, yeokja `latex` 파서로 번역해 한국어 PDF를 Pages로 배포한다.

**Architecture:** 원서 PDF를 서브모듈로 고정한다. 페이지 이미지와 폰트로 분류한 텍스트를 추출해 장별로 LLM 서브에이전트가 고정된 매크로 어휘로 LaTeX를 쓴다. 영어 모드로 다시 컴파일한 텍스트를 원서와 단어 단위로 대조해 검증한다. 검증된 `source/`를 yeokja로 번역해 같은 프리앰블의 한국어 모드(LuaLaTeX + luatexko)로 조판한다.

**Tech Stack:** Python 3 + PyMuPDF, poppler, LuaLaTeX(memoir, fontspec, unicode-math, luatexko), latexmk, yeokja(`latex` 파서, claude_code provider), Nix devShell, GitHub Actions Pages.

**Spec:** `docs/superpowers/specs/2026-09-20-jeffe-algorithms-korean-translation-design.md`

## Global Constraints

- 프로젝트: `projects/jeffe-algorithms/`. 원문 서브모듈 `projects/jeffe-algorithms/upstream` = `https://github.com/jeffgerickson/algorithms.git`, `shallow = true`, 구현 시점 기본 브랜치 HEAD. 기준 원서는 `upstream/Algorithms-JeffE.pdf`(472쪽). `upstream/`은 수정하지 않는다.
- PDF 쪽 번호 = 인쇄 쪽 번호 + 18(앞부분 로마 숫자 18쪽). 모든 스크립트와 문서는 PDF 쪽(1부터)을 쓰고, 인쇄 쪽은 명시할 때만 쓴다.
- 범위: 서문, 0~12장(연습문제·각주 포함), Image Credits, Colophon. 색인 3종과 CC BY-NC-SA 강의 노트는 제외. 그림 1.25(인쇄 61쪽, PDF 79쪽) 이미지는 싣지 않고 원서 안내 문구로 대체. 그림 안 영어 문구는 원서 그대로.
- 원고 파일: `source/main.tex`, `source/preamble.tex`, `source/frontmatter/preface.tex`, `source/chapters/NN-<name>.tex`(`00-intro`, `01-recursion`, `02-backtracking`, `03-dynprog`, `04-greedy`, `05-graphs`, `06-dfs`, `07-mst`, `08-sssp`, `09-apsp`, `10-maxflow`, `11-maxflowapps`, `12-nphard`), `source/backmatter/credits.tex`, `source/backmatter/colophon.tex`, `source/figures/`.
- 매크로 어휘는 `source/preamble.tex`와 `RECONSTRUCTION.md`가 정의하며, 복원 에이전트는 그 밖의 매크로를 만들지 않는다(필요하면 두 파일을 먼저 고친다).
- 원고 검증 기준: 장별 원서 본문 단어 중 불일치 비율 ≤ 0.5%, 모든 불일치 구간 검토, 그림·각주·연습문제 개수와 절 제목 목록 일치, `\Proc` 절차 집합이 원서 Index of Pseudocode의 해당 장 항목을 모두 포함.
- 엔진: 영어·한국어 모두 LuaLaTeX. 한국어 모드는 `\koreantrue`.
- provider: `type = "claude_code"`, `model = "claude-sonnet-5"`. 의사코드 키워드·절차 이름은 영어, 주석만 번역.
- 번역본 라이선스 CC BY 4.0, © 2019 Jeff Erickson, 원서 링크 `http://algorithms.wtf`, 변경 사항(원고 복원·번역·색인 제외·그림 1.25 제외), 비공식·저자 미승인, 복원·번역에 쓴 AI 모델 명시.
- README 출처 표기는 AGENTS.md 규칙(yeokja 링크 `https://github.com/yeokja/yeokja`, 실제 번역 모델, 학습 비허용 문장)을 따른다. 영어 원고 복원 모델도 README에 적는다(복원 서브에이전트의 실제 모델).
- 번역 provider 동시 사용 제한: `yeokja translate` 전에 `pgrep -fl 'yeokja translate'`로 다른 번역이 없는지 확인하고 있으면 기다린다.
- 커밋하지 않는 것: `ko/`, `build/`, `output/`, `dist/`, `__pycache__/`, LaTeX 중간 파일.
- 커밋 메시지: `[jeffe-algorithms] <type>: <한국어 요약>`, 마지막 줄 `Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>`. push하지 않는다.
- `$Y` = 저장소 루트 `target/release/yeokja`. `$P` = `projects/jeffe-algorithms`. 명령은 특별히 적지 않으면 `$P`에서 `nix develop path:../../nix#jeffe-algorithms -c ...`로 실행한다(아래에서는 `nd` 로 줄여 쓴다: `alias nd='nix develop path:../../nix#jeffe-algorithms -c'`).
- 설계 조사 산출물(참고용, 커밋 대상 아님): `/private/tmp/claude-501/-Users-moreal-github-yeokja-yeokja/6279d88e-ffe5-4dea-9e86-e529821b04db/scratchpad/jeffe/`(`figx.py` 그림 잘라내기 시제품, `spans.py`, `samples/p351-p91-llm-reconstruction.tex` 복원 예시, `fonts.txt`). 이 경로가 없으면 무시하고 plan의 코드를 쓴다.

---

### Task 1: 프로젝트 골격, 원문 고정, devShell

**Files:**
- Modify: `.gitmodules`; Create: `$P/upstream`(gitlink), `$P/.gitignore`, `nix/projects/jeffe-algorithms.nix`

- [ ] **Step 1: 서브모듈**

```bash
git submodule add https://github.com/jeffgerickson/algorithms.git projects/jeffe-algorithms/upstream
git config -f .gitmodules submodule.projects/jeffe-algorithms/upstream.shallow true
git -C projects/jeffe-algorithms/upstream log -1 --format='%H %cd'   # README에 적는다
python3 -c "import hashlib;print(hashlib.sha256(open('projects/jeffe-algorithms/upstream/Algorithms-JeffE.pdf','rb').read()).hexdigest())"  # README에 적는다
```

`$P/.gitignore`:

```gitignore
ko/
dist/
output/
__pycache__/
*.aux
*.log
*.out
*.toc
*.fls
*.fdb_latexmk
*.synctex.gz
```

- [ ] **Step 2: devShell**

`nix/projects/jeffe-algorithms.nix`:

```nix
# jeffe-algorithms: 원서 PDF 추출(PyMuPDF, poppler)과 LuaLaTeX 조판.
# 원서 글꼴과 같은 계열(XCharter, Roboto, Inconsolata; 수식 XCharter-Math)을
# TeX Live에서, 한국어는 나눔 글꼴을 씁니다(hott 프로젝트와 같은 글꼴 설정 방식).
{
  pkgs,
  lib,
  system,
}: let
  tex = pkgs.texlive.combine {
    inherit
      (pkgs.texlive)
      scheme-medium
      collection-luatex
      collection-langkorean
      collection-fontsrecommended
      collection-fontsextra
      collection-latexextra
      collection-mathscience
      latexmk
      ;
  };
in
  pkgs.mkShell {
    packages = [
      tex
      pkgs.nanum
      pkgs.poppler_utils
      (pkgs.python3.withPackages (ps: [ps.pymupdf]))
    ];
    FONTCONFIG_FILE = pkgs.makeFontsConf {fontDirectories = [pkgs.nanum];};
    shellHook = ''
      export OSFONTDIR="${pkgs.nanum}/share/fonts//"
    '';
  }
```

- [ ] **Step 3: 도구 확인**

```bash
cd projects/jeffe-algorithms
nix develop path:../../nix#jeffe-algorithms -c sh -c '
  python3 -c "import pymupdf; print(pymupdf.__doc__)";
  pdfinfo upstream/Algorithms-JeffE.pdf | grep Pages;
  for f in XCharter-Roman.otf XCharter-Math.otf Roboto-Regular.otf Inconsolatazi4-Regular.otf; do kpsewhich $f || echo "MISSING $f"; done;
  fc-list | grep -c Nanum'
```

기대: `Pages: 472`, 글꼴 경로가 모두 나오고 Nanum이 1개 이상. `XCharter-Math.otf`가 없으면(`xcharter-math` 패키지가 컬렉션에 없을 때) `texlive.combine`에 `xcharter-math`를 추가하고, 그래도 없으면 수식 글꼴을 `STIXTwoMath-Regular.otf`(`stix2-otf`)로 정하고 spec의 "원고 형식" 절을 고친다. `Roboto-*.otf`는 `roboto` 패키지, Inconsolata는 `inconsolata` 패키지(파일명은 `kpsewhich -all -format=opentype` 또는 `fc-list`로 확인)다.

- [ ] **Step 4: 커밋**

```bash
cd ../.. && git add .gitmodules projects/jeffe-algorithms/upstream projects/jeffe-algorithms/.gitignore nix/projects/jeffe-algorithms.nix
git commit -m "[jeffe-algorithms] feat: 원서 PDF 서브모듈과 LuaLaTeX devShell 추가" -m "(#3)" -m "Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>"
```

---

### Task 2: 추출 스크립트

**Files:**
- Create: `$P/scripts/extract.py`, `$P/scripts/test_extract.py`

**Interfaces:**
- Produces: `classify(font: str, bbox, page_height, figure_boxes) -> str`(값: `text|math|code|heading|figure|header_footer`), `normalize_glyphs(text: str) -> str`, `chapter_ranges(toc: list[list]) -> dict[str, list[int]]`(장 이름 → `[첫 PDF 쪽, 끝 PDF 쪽]`), CLI `extract.py <pdf> <outdir>` → `<outdir>/pNNN.png`, `<outdir>/pNNN.json`, `<outdir>/chapters.json`, `<outdir>/figures.json`.
- `pNNN.json` 형식: `{"page": N, "printed": "…", "spans": [{"text", "font", "size", "bbox":[x0,y0,x1,y1], "cls"}], "links": [{"bbox", "to_page", "name"}], "figures": [[x0,y0,x1,y1], …]}`.

- [ ] **Step 1: 실패하는 테스트**

`$P/scripts/test_extract.py`:

```python
import unittest
from extract import classify, normalize_glyphs, chapter_ranges


class ClassifyTests(unittest.TestCase):
    def test_fonts(self):
        box = (100, 300, 200, 310)
        self.assertEqual(classify('ABCDEF+XCharter-Roman', box, 792, []), 'text')
        self.assertEqual(classify('ABCDEF+MathDesign-Charter-Italic', box, 792, []), 'math')
        self.assertEqual(classify('ABCDEF+Inconsolata-Regular', box, 792, []), 'code')
        self.assertEqual(classify('ABCDEF+Roboto-Bold', box, 792, []), 'heading')

    def test_position_and_figure_win_over_font(self):
        self.assertEqual(classify('XCharter-Roman', (100, 20, 200, 30), 792, []), 'header_footer')
        self.assertEqual(classify('XCharter-Roman', (100, 770, 200, 780), 792, []), 'header_footer')
        self.assertEqual(classify('XCharter-Roman', (110, 310, 150, 320), 792, [(100, 300, 300, 400)]), 'figure')


class GlyphTests(unittest.TestCase):
    def test_known_glyph_problems(self):
        self.assertEqual(normalize_glyphs('u\x01v'), 'u⟨ARC⟩v')
        self.assertEqual(normalize_glyphs('ﬁnd ﬂow'), 'find flow')


class ChapterTests(unittest.TestCase):
    def test_ranges_from_outline(self):
        toc = [[1, 'Preface', 5], [1, '0 Introduction', 19], [2, '0.1 What is an algorithm?', 19],
               [1, '1 Recursion', 39], [1, 'Index', 455]]
        self.assertEqual(chapter_ranges(toc, 472), {
            'preface': [5, 18], '00': [19, 38], '01': [39, 454], 'index': [455, 472]})


if __name__ == '__main__':
    unittest.main()
```

Run: `cd projects/jeffe-algorithms/scripts && nix develop path:../../../nix#jeffe-algorithms -c python3 -m unittest test_extract`
Expected: FAIL(`No module named 'extract'`).

- [ ] **Step 2: 구현**

`$P/scripts/extract.py`:

```python
"""Extract per-page reconstruction material from the Algorithms PDF.

For every PDF page: a 150dpi PNG, and a JSON with each text span's font,
size, bbox and class (text/math/code/heading/figure/header_footer), the
page's internal link targets, and the bounding boxes of embedded figures.
Also writes chapters.json (chapter key -> [first, last] PDF page) from the
outline and figures.json (page -> figure boxes).
"""
import json
from pathlib import Path
import re
import sys

MARGIN = 45  # points; running heads and folios live in the top/bottom bands
GLYPHS = {'\x01': '⟨ARC⟩', 'ﬁ': 'fi', 'ﬂ': 'fl', 'ﬀ': 'ff', 'ﬃ': 'ffi', 'ﬄ': 'ffl'}


def classify(font, bbox, page_height, figure_boxes):
    x0, y0, x1, y1 = bbox
    if y1 < MARGIN or y0 > page_height - MARGIN:
        return 'header_footer'
    cx, cy = (x0 + x1) / 2, (y0 + y1) / 2
    if any(fx0 <= cx <= fx1 and fy0 <= cy <= fy1 for fx0, fy0, fx1, fy1 in figure_boxes):
        return 'figure'
    name = font.split('+')[-1]
    if name.startswith('MathDesign') or 'Math' in name or name.startswith(('CMSY', 'CMEX', 'stmary', 'MSAM', 'MSBM')):
        return 'math'
    if name.startswith('Inconsolata'):
        return 'code'
    if name.startswith('Roboto'):
        return 'heading'
    return 'text'


def normalize_glyphs(text):
    for bad, good in GLYPHS.items():
        text = text.replace(bad, good)
    return text


def chapter_ranges(toc, page_count):
    """Top-level outline entries -> chapter keys ('preface', '00'..'12', 'index', ...)."""
    tops = [(title, page) for level, title, page in toc if level == 1]
    ranges = {}
    for i, (title, first) in enumerate(tops):
        last = (tops[i + 1][1] - 1) if i + 1 < len(tops) else page_count
        m = re.match(r'(\d+)\s', title)
        key = f'{int(m.group(1)):02d}' if m else re.sub(r'\W+', '-', title.lower()).strip('-')
        ranges[key] = [first, last]
    return ranges


def figure_boxes(page):
    """Raster image placements; vector figure boxes come from figures.json
    (written by extract_figures.py, which traces the content stream)."""
    return [tuple(page.get_image_bbox(img)) for img in page.get_images(full=True)
            if not page.get_image_bbox(img).is_empty]


def main(pdf_path, out):
    import pymupdf
    out = Path(out)
    out.mkdir(parents=True, exist_ok=True)
    doc = pymupdf.open(pdf_path)
    (out / 'chapters.json').write_text(json.dumps(chapter_ranges(doc.get_toc(), doc.page_count), indent=1))
    figures = json.loads(Path(sys.argv[3]).read_text()) if len(sys.argv) > 3 else {}
    for index, page in enumerate(doc):
        number = index + 1
        boxes = [tuple(b) for b in figures.get(str(number), [])] + figure_boxes(page)
        spans = []
        for block in page.get_text('dict')['blocks']:
            for line in block.get('lines', []):
                for span in line['spans']:
                    if not span['text'].strip():
                        continue
                    spans.append({'text': normalize_glyphs(span['text']), 'font': span['font'],
                                  'size': round(span['size'], 1), 'bbox': [round(v, 1) for v in span['bbox']],
                                  'cls': classify(span['font'], span['bbox'], page.rect.height, boxes)})
        links = [{'bbox': [round(v, 1) for v in link['from']], 'to_page': link.get('page', -1) + 1,
                  'name': link.get('nameddest') or link.get('uri')} for link in page.get_links()]
        (out / f'p{number:03d}.json').write_text(json.dumps(
            {'page': number, 'printed': page.get_label(), 'spans': spans, 'links': links,
             'figures': [list(b) for b in boxes]}, ensure_ascii=False, indent=0))
        page.get_pixmap(dpi=150).save(out / f'p{number:03d}.png')


if __name__ == '__main__':
    main(sys.argv[1], sys.argv[2])
```

벡터 그림의 배치는 페이지 콘텐츠 스트림의 `cm`·`Do` 연산을 추적해야 정확하므로 Task 3의 `extract_figures.py`가 계산하고, `extract.py`는 그 결과(`figures.json`)를 세 번째 인자로 받는다.

Run: `python3 -m unittest test_extract` → PASS. 목차 제목 형식(`'0 Introduction'`처럼 번호+공백)은 실제 `doc.get_toc()` 출력을 보고 정규식을 맞춘다: `nd python3 -c "import pymupdf;print(pymupdf.open('upstream/Algorithms-JeffE.pdf').get_toc()[:30])"`. 테스트의 `toc`도 실제 형식에 맞춰 고친다.

- [ ] **Step 3: 커밋(실제 추출 실행은 Task 3 뒤)**

```bash
git add projects/jeffe-algorithms/scripts/extract.py projects/jeffe-algorithms/scripts/test_extract.py
git commit -m "[jeffe-algorithms] feat: 원서 PDF 쪽별 추출 스크립트 추가" -m "Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>"
```

---

### Task 3: 그림 잘라내기

**Files:**
- Create: `$P/scripts/extract_figures.py`, `$P/scripts/test_extract_figures.py`
- Create(생성물, 커밋): `$P/source/figures/*.pdf`, `*.png|*.jpg`, `$P/source/figures/manifest.json`

**Interfaces:**
- Produces: `placements(doc, page) -> list[tuple[str, Rect]]`(XObject 이름과 페이지 좌표 상자), CLI `extract_figures.py <pdf> <source/figures> <build/extract/figures.json>`. 파일명: 벡터 `p<PDF쪽 3자리>-<XObject이름>.pdf`, 래스터 `p<쪽>-img<n>.<ext>`. `manifest.json`: `{"p091-Fm12": {"page": 91, "bbox": [...], "kind": "vector"}, ...}`.

- [ ] **Step 1: 테스트(작은 합성 PDF로)**

```python
import unittest
import pymupdf
from extract_figures import placements


class PlacementTests(unittest.TestCase):
    def test_form_xobject_placement_in_page_coordinates(self):
        src = pymupdf.open()
        figure = src.new_page(width=100, height=50)
        figure.draw_rect(pymupdf.Rect(0, 0, 100, 50))
        figure.insert_text((10, 30), 'label')
        doc = pymupdf.open()
        page = doc.new_page(width=612, height=792)
        page.show_pdf_page(pymupdf.Rect(200, 300, 300, 350), src, 0)
        found = placements(doc, page)
        self.assertEqual(len(found), 1)
        name, rect = found[0]
        self.assertAlmostEqual(rect.x0, 200, delta=1)
        self.assertAlmostEqual(rect.y0, 300, delta=1)
        self.assertAlmostEqual(rect.x1, 300, delta=1)
        self.assertAlmostEqual(rect.y1, 350, delta=1)


if __name__ == '__main__':
    unittest.main()
```

Run: FAIL(모듈 없음).

- [ ] **Step 2: 구현(설계 조사 `figx.py`를 일반화)**

```python
"""Cut every embedded vector figure (form XObject) and raster image out of
the Algorithms PDF as standalone files for the reconstructed source.

The book's figures are OmniGraffle PDFs placed with \\includegraphics, so each
is a form XObject drawn by a `Do` operator; its page box is the XObject's
BBox transformed by the current transformation matrix at that `Do`.
"""
import json
from pathlib import Path
import re
import sys

import pymupdf

TOKEN = re.compile(rb"/[^\s/\[\]()<>{}]+|\((?:\\.|[^\\)])*\)|<[0-9A-Fa-f\s]*>|\[|\]|[^\s/\[\]()<>]+")
SKIP_PAGES = {79}  # Figure 1.25 (printed p. 61): portrait included with the artist's permission only


def placements(doc, page):
    forms = {name: (xref, bbox) for xref, name, invoker, bbox in page.get_xobjects() if invoker == 0}
    stream = b''.join(doc.xref_stream(c) or b'' for c in page.get_contents())
    ctm, stack, operands, found = pymupdf.Matrix(1, 0, 0, 1, 0, 0), [], [], []
    height = page.mediabox.height
    for token in TOKEN.findall(stream):
        if token == b'q':
            stack.append(ctm)
        elif token == b'Q':
            ctm = stack.pop() if stack else ctm
        elif token == b'cm':
            a, b, c, d, e, f = map(float, operands[-6:])
            ctm = pymupdf.Matrix(a, b, c, d, e, f) * ctm
        elif token == b'Do':
            name = operands[-1].decode()[1:]
            if name in forms:
                r = pymupdf.Rect(forms[name][1]) * ctm
                found.append((name, pymupdf.Rect(r.x0, height - r.y1, r.x1, height - r.y0)))
        if token in (b'q', b'Q', b'cm', b'Do') or (re.match(rb"^[A-Za-z'\"*]+$", token) and not token.startswith(b'/')):
            operands = []
        else:
            operands.append(token)
    return found


def main(pdf_path, figures_dir, boxes_path):
    doc = pymupdf.open(pdf_path)
    figures_dir, manifest, boxes = Path(figures_dir), {}, {}
    figures_dir.mkdir(parents=True, exist_ok=True)
    for index, page in enumerate(doc):
        number = index + 1
        page_boxes = []
        for name, rect in placements(doc, page):
            page_boxes.append([round(v, 1) for v in rect])
            if number in SKIP_PAGES:
                continue
            key = f'p{number:03d}-{name}'
            out = pymupdf.open()
            target = out.new_page(width=rect.width, height=rect.height)
            target.show_pdf_page(target.rect, doc, index, clip=rect)
            out.save(figures_dir / f'{key}.pdf', garbage=4, deflate=True)
            manifest[key] = {'page': number, 'bbox': page_boxes[-1], 'kind': 'vector'}
        for n, image in enumerate(page.get_images(full=True)):
            rect = page.get_image_bbox(image)
            if rect.is_empty:
                continue
            page_boxes.append([round(v, 1) for v in rect])
            if number in SKIP_PAGES:
                continue
            info = doc.extract_image(image[0])
            key = f'p{number:03d}-img{n}'
            (figures_dir / f'{key}.{info["ext"]}').write_bytes(info['image'])
            manifest[key] = {'page': number, 'bbox': page_boxes[-1], 'kind': 'raster'}
        if page_boxes:
            boxes[str(number)] = page_boxes
    (figures_dir / 'manifest.json').write_text(json.dumps(manifest, indent=1))
    Path(boxes_path).parent.mkdir(parents=True, exist_ok=True)
    Path(boxes_path).write_text(json.dumps(boxes, indent=1))


if __name__ == '__main__':
    main(*sys.argv[1:4])
```

- [ ] **Step 3: 실행과 추출**

```bash
cd projects/jeffe-algorithms
nd python3 scripts/extract_figures.py upstream/Algorithms-JeffE.pdf source/figures build/extract/figures.json
nd python3 scripts/extract.py upstream/Algorithms-JeffE.pdf build/extract build/extract/figures.json
ls source/figures | wc -l; du -sh source/figures; cat build/extract/chapters.json
```

기대: 벡터 약 211개와 래스터 이미지가 생기고, 크기는 수 MB 수준이다. `chapters.json`에 preface, 00~12, 색인·credits·colophon 범위가 나온다. 표본 3개(PDF 91쪽 의사코드 그림, 51쪽, 한 래스터)를 열어 원서와 같은지 눈으로 확인한다: `nd pdftoppm -r 80 -png source/figures/<파일>.pdf /tmp/check`는 쓰지 말고 `build/check-*.png`로 저장해 Read 도구로 본다. 79쪽 그림이 없는지 확인한다.

- [ ] **Step 4: 커밋**

```bash
cd ../.. && git add projects/jeffe-algorithms/scripts/extract_figures.py projects/jeffe-algorithms/scripts/test_extract_figures.py projects/jeffe-algorithms/source/figures
git commit -m "[jeffe-algorithms] feat: 원서 그림을 독립 PDF·이미지로 잘라낸 결과 추가" -m "그림 1.25(작가 허락으로만 수록된 초상화)는 제외한다." -m "Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>"
```

---

### Task 4: 원고 골격, 프리앰블, 복원 규칙

**Files:**
- Create: `$P/source/main.tex`, `$P/source/preamble.tex`, `$P/source/frontmatter/translation-notice.tex`, `$P/RECONSTRUCTION.md`
- Create: 빈 장 파일 15개(`frontmatter/preface.tex`, `chapters/00-intro.tex` … `12-nphard.tex`, `backmatter/credits.tex`, `backmatter/colophon.tex`) — 내용은 `% TO BE RECONSTRUCTED` 주석 한 줄(복원 전 컴파일 확인용; Task 6·8에서 채운다)

**Interfaces:**
- Produces: 매크로 어휘(아래), `\ifkorean` 전환, `main.tex`의 `\include` 목록. `verify_source.py`(Task 5)와 복원 에이전트(Task 6·8)가 이 파일들을 쓴다.

- [ ] **Step 1: `main.tex`**

```latex
\providecommand{\KoreanEdition}{0}
\newif\ifkorean
\ifnum\KoreanEdition=1 \koreantrue\fi
\documentclass[11pt,twoside,openany]{memoir}
\input{preamble}
\includeonly{\IncludeOnly}
\begin{document}
\frontmatter
\ifkorean\include{frontmatter/translation-notice}\fi
\include{frontmatter/preface}
\tableofcontents*
\mainmatter
\setcounter{chapter}{-1}
\include{chapters/00-intro}
\include{chapters/01-recursion}
\include{chapters/02-backtracking}
\include{chapters/03-dynprog}
\include{chapters/04-greedy}
\include{chapters/05-graphs}
\include{chapters/06-dfs}
\include{chapters/07-mst}
\include{chapters/08-sssp}
\include{chapters/09-apsp}
\include{chapters/10-maxflow}
\include{chapters/11-maxflowapps}
\include{chapters/12-nphard}
\backmatter
\include{backmatter/credits}
\include{backmatter/colophon}
\end{document}
```

`\IncludeOnly`는 프리앰블에서 기본값(전체 목록)을 정의하고, `verify_source.py`가 `-usepretex='\def\IncludeOnly{chapters/02-backtracking}'`로 바꾼다. 한국어판은 `-usepretex='\def\KoreanEdition{1}'`로 켠다. 원서의 장 번호가 0부터 시작하므로 `\mainmatter` 다음에 `\setcounter{chapter}{-1}`을 둔다.

- [ ] **Step 2: `preamble.tex`**

```latex
% Reconstructed layout for Jeff Erickson, Algorithms (2019).
% Close to the original (Charter text, Roboto headings, Inconsolata code),
% not a pixel copy. \ifkorean switches to the Korean edition.
\providecommand{\IncludeOnly}{frontmatter/translation-notice,frontmatter/preface,%
chapters/00-intro,chapters/01-recursion,chapters/02-backtracking,chapters/03-dynprog,%
chapters/04-greedy,chapters/05-graphs,chapters/06-dfs,chapters/07-mst,chapters/08-sssp,%
chapters/09-apsp,chapters/10-maxflow,chapters/11-maxflowapps,chapters/12-nphard,%
backmatter/credits,backmatter/colophon}
\usepackage{amsmath}
\usepackage{fontspec}
\usepackage{unicode-math}
\setmainfont{XCharter}[Extension=.otf, UprightFont=*-Roman, ItalicFont=*-Italic,
  BoldFont=*-Bold, BoldItalicFont=*-BoldItalic, Numbers=OldStyle]
\setsansfont{Roboto}[Extension=.otf, UprightFont=*-Regular, BoldFont=*-Bold,
  ItalicFont=*-Italic, BoldItalicFont=*-BoldItalic]
\setmonofont{Inconsolatazi4}[Extension=.otf, UprightFont=*-Regular, BoldFont=*-Bold]
\setmathfont{XCharter-Math.otf}
\ifkorean
  \usepackage[hangul]{luatexko}
  \setmainhangulfont{NanumMyeongjo}[BoldFont=NanumMyeongjoBold]
  \setsanshangulfont{NanumGothic}[BoldFont=NanumGothicBold]
  \setmonohangulfont{NanumGothicCoding}
\fi
\usepackage{graphicx}
\graphicspath{{figures/}}
\usepackage{enumitem}
\usepackage{xcolor}
\usepackage[hidelinks,bookmarksnumbered]{hyperref}

% Headings in Roboto, as in the original.
\setsecheadstyle{\Large\sffamily\bfseries}
\setsubsecheadstyle{\large\sffamily\bfseries}
\renewcommand{\chaptitlefont}{\Huge\sffamily\bfseries}
\renewcommand{\chapnumfont}{\Huge\sffamily\bfseries}
\setsecnumdepth{subsection}
\counterwithin{figure}{chapter}

% ---- Fixed macro vocabulary (see RECONSTRUCTION.md) ----
\newcommand{\arc}[2]{#1\mathord{\rightarrow}#2}   % directed edge u→v
\newcommand{\Proc}[1]{\textsc{#1}}                % pseudocode procedure name
\newcommand{\Var}[1]{\mathit{#1}}                 % multi-letter variable
\newcommand{\Comment}[1]{\quad$\langle\!\langle$\,\textit{#1}\,$\rangle\!\rangle$}
\newcommand{\Indent}{\hspace*{1.5em}}
\newenvironment{algorithm}
  {\par\medskip\noindent\begin{minipage}{\linewidth}\setlength{\parindent}{0pt}\obeylines\raggedright}
  {\end{minipage}\par\medskip}
\newcommand{\difficulty}[1]{\ifcase#1\or$\heartsuit$\or$\clubsuit$\or$\spadesuit$\fi\ }
\newenvironment{exercises}{\section*{\ExercisesName}\begin{enumerate}[label=\arabic*.,leftmargin=*]}{\end{enumerate}}
\ifkorean
  \newcommand{\FigureName}{그림}\newcommand{\SectionName}{절}\newcommand{\ChapterName}{장}
  \newcommand{\PageName}{쪽}\newcommand{\ExercisesName}{연습문제}
  \renewcommand{\contentsname}{차례}\renewcommand{\figurename}{그림}
\else
  \newcommand{\FigureName}{Figure}\newcommand{\SectionName}{Section}\newcommand{\ChapterName}{Chapter}
  \newcommand{\PageName}{page}\newcommand{\ExercisesName}{Exercises}
\fi
\newcommand{\figref}[1]{\FigureName~\ref{#1}}
\newcommand{\secref}[1]{\ifkorean\ref{#1}\SectionName\else\SectionName~\ref{#1}\fi}
\newcommand{\chapref}[1]{\ifkorean\ref{#1}\ChapterName\else\ChapterName~\ref{#1}\fi}
\newcommand{\omitfigure}[1]{\par\medskip\noindent\fbox{\parbox{\linewidth}{\centering\small #1}}\par\medskip}
```

원서의 연습문제 난이도 기호, 의사코드 상자 모양, 각주 모양은 Task 6 파일럿에서 페이지 이미지와 비교해 조정한다(조정 내용은 이 파일과 `RECONSTRUCTION.md`에 함께 반영). `\figref` 등의 한국어 어순은 번역문에서 모델이 `\figref{fig:2.3}`을 그대로 두기만 하면 "그림 2.3"으로 조판되게 한 것이다.

- [ ] **Step 3: `translation-notice.tex`(한국어판 판권 면, 번역 대상 아님)**

```latex
\thispagestyle{empty}
\vspace*{\fill}
\noindent\textbf{비공식 한국어 번역}\par\medskip
\noindent 원서: Jeff Erickson, \textit{Algorithms}, 1st edition (2019), \url{http://algorithms.wtf}.
© 2019 Jeff Erickson. 원서는 Creative Commons Attribution 4.0 International(CC BY 4.0)로 공개되었습니다.\par\medskip
\noindent 이 번역본은 원서 PDF에서 영어 원고를 복원한 뒤 한국어로 옮긴 개작물이며, 같은 CC BY 4.0으로 제공합니다.
원서의 색인 3종과 그림 1.25는 싣지 않았고, 그림 안의 영어 문구는 원서 그대로 두었습니다.
저자가 검토하거나 승인한 번역이 아닙니다.\par\medskip
\noindent 영어 원고는 AI 모델의 도움으로 복원했으며, 번역은 \href{https://github.com/yeokja/yeokja}{yeokja}와 함께
Anthropic 사의 \texttt{claude-sonnet-5} 모델을 활용하여 번역되었으며 학습을 모두 비허용한 상태로 작업하였습니다.
\vspace*{\fill}
\clearpage
```

(모델명은 Task 11에서 state 이력으로 다시 확인한다.)

- [ ] **Step 4: `RECONSTRUCTION.md`**

````markdown
# 원고 복원 규칙

이 디렉터리의 `source/`는 Jeff Erickson의 *Algorithms* PDF(`upstream/Algorithms-JeffE.pdf`)에서
복원한 영어 LaTeX 원고입니다. 저자의 원고가 아닙니다.

## 입력
- `build/extract/pNNN.png`: 쪽 이미지(구조의 기준)
- `build/extract/pNNN.json`: 스팬별 텍스트와 분류(`text`/`math`/`code`/`heading`/`figure`/`header_footer`), 링크
- `build/extract/chapters.json`: 장별 PDF 쪽 범위
- `source/figures/manifest.json`: 쪽별 그림 파일

## 규칙
1. **단어는 JSON에서 복사한다.** 산문 단어를 이미지에서 새로 타이핑하지 않는다. `⟨ARC⟩`는 `\arc{u}{v}`로, 하이픈 줄바꿈은 잇는다.
2. **구조는 이미지를 따른다.** 절·소절, 목록, 인용구(`\epigraph{인용}{출처}`), 의사코드, 수식 정렬, 각주.
3. **매크로 어휘만 쓴다:** `\arc`, `\Proc`, `\Var`, `\Comment`, `\Indent`, `algorithm` 환경, `exercises` 환경과 `\difficulty{1..3}`, `\figref`, `\secref`, `\chapref`, `\omitfigure`. 새 매크로가 필요하면 먼저 `preamble.tex`와 이 문서를 고친다.
4. **라벨:** `\label{fig:<장>.<번호>}`, `\label{sec:<장>.<번호>}`, `\label{ex:<장>.<번호>}`, `\label{eq:<장>.<번호>}`. 원서 번호와 같아야 한다. 본문 참조는 JSON 링크 대상으로 확인한다.
5. **그림:** `\begin{figure}[ht]\centering\includegraphics{p091-Fm12}\caption{…}\label{fig:2.3}\end{figure}`. 한 그림이 XObject 여러 개면 원서 배치대로 나란히 둔다. 의사코드가 그림 안에 있으면 `algorithm` 환경을 `figure` 안에 둔다.
6. **그림 1.25**는 `\omitfigure{그림 1.25는 원서 61쪽을 참고하십시오.}` 대신 영어 모드 문구 `\omitfigure{Figure 1.25 (a portrait of the author) is omitted; see page 61 of the original.}`를 쓰고 캡션과 라벨은 유지한다.
7. **각주**는 `\footnote{…}`. **여백 인용구**가 아닌 장 첫머리 인용구는 `\epigraph`.
8. **연습문제**는 장 끝 `\begin{exercises} \item … \end{exercises}`, 하위 문항은 `enumerate`(`label=(\alph*)`).
9. 쪽 나눔·줄 나눔을 흉내 내지 않는다(`\newpage`, `\\` 금지 — 의사코드 안은 예외).
10. 한 장이 끝나면 `scripts/verify_source.py <장 키>`를 통과시킨다.
````

- [ ] **Step 5: 골격 컴파일 확인**

```bash
cd projects/jeffe-algorithms/source
nd latexmk -lualatex -interaction=nonstopmode -halt-on-error -outdir=../build/skeleton main.tex
nd latexmk -lualatex -interaction=nonstopmode -halt-on-error -outdir=../build/skeleton-ko -usepretex='\koreantrue' main.tex
```

두 번째 명령은 `-usepretex='\def\KoreanEdition{1}'`로 바꿔 실행한다(`\koreantrue`는 `main.tex`에서 `\newif` 뒤에야 정의되므로 pretex로 쓸 수 없다). 두 빌드 모두 오류 없이 PDF가 나와야 한다(내용은 빈 장). 글꼴 누락이면 Task 1 Step 3으로 돌아간다.

- [ ] **Step 6: 커밋**

```bash
cd ../../.. && git add projects/jeffe-algorithms/source projects/jeffe-algorithms/RECONSTRUCTION.md
git commit -m "[jeffe-algorithms] feat: 복원 원고 골격, 프리앰블 매크로 어휘, 복원 규칙 추가" -m "Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>"
```

---

### Task 5: 원고 검증 스크립트

**Files:**
- Create: `$P/scripts/verify_source.py`, `$P/scripts/test_verify_source.py`

**Interfaces:**
- Consumes: `extract.classify`, `extract.normalize_glyphs`, `build/extract/*`, `source/`.
- Produces: `words(spans) -> list[str]`, `compare(original: list[str], rebuilt: list[str]) -> Report`(`mismatch_ratio: float`, `hunks: list[tuple[list[str], list[str]]]`), CLI `verify_source.py <chapter-key>`(종료 코드 0/1, `build/verify/<key>.txt` 보고서).

- [ ] **Step 1: 테스트**

```python
import unittest
from verify_source import words, compare


class WordTests(unittest.TestCase):
    def test_keeps_prose_drops_math_figures_and_running_heads(self):
        spans = [{'text': 'The algo-', 'cls': 'text'}, {'text': 'rithm runs', 'cls': 'text'},
                 {'text': 'x + y', 'cls': 'math'}, {'text': '2. BACKTRACKING', 'cls': 'header_footer'},
                 {'text': 'label', 'cls': 'figure'}, {'text': 'PlaceQueens', 'cls': 'code'}]
        self.assertEqual(words(spans), ['The', 'algorithm', 'runs', 'PlaceQueens'])

    def test_page_numbers_after_page_are_wildcards(self):
        self.assertEqual(words([{'text': 'see page 81 and pages 3–5', 'cls': 'text'}]),
                         ['see', 'page', '#', 'and', 'pages', '#'])


class CompareTests(unittest.TestCase):
    def test_ratio_and_hunks(self):
        report = compare(['a', 'b', 'c', 'd'], ['a', 'x', 'c', 'd'])
        self.assertAlmostEqual(report.mismatch_ratio, 0.25)
        self.assertEqual(report.hunks, [(['b'], ['x'])])

    def test_unresolved_reference_matches_a_number(self):
        report = compare(['Figure', '1.3', 'shows'], ['Figure', '??', 'shows'])
        self.assertEqual(report.mismatch_ratio, 0)


if __name__ == '__main__':
    unittest.main()
```

- [ ] **Step 2: 구현**

```python
"""Verify a reconstructed chapter against the original PDF's text layer.

Compiles the English source for one chapter, extracts its text with the same
span classification as the original, and compares prose word sequences.
Writes build/verify/<key>.txt and exits 1 when the chapter fails.
"""
from dataclasses import dataclass, field
import difflib
import json
from pathlib import Path
import re
import subprocess
import sys

from extract import classify, normalize_glyphs

ROOT = Path(__file__).resolve().parent.parent
KEEP = {'text', 'heading', 'code'}
MAX_RATIO = 0.005
NUMBER = re.compile(r'^[\dA-Z]+(\.\d+)*[.,;:)]*$')
FILES = {'preface': 'frontmatter/preface', 'image-credits': 'backmatter/credits', 'colophon': 'backmatter/colophon',
         **{f'{n:02d}': f'chapters/{n:02d}-{name}' for n, name in enumerate(
             ['intro', 'recursion', 'backtracking', 'dynprog', 'greedy', 'graphs', 'dfs', 'mst',
              'sssp', 'apsp', 'maxflow', 'maxflowapps', 'nphard'])}}


def words(spans):
    text = ' '.join(s['text'] for s in spans if s['cls'] in KEEP)
    text = re.sub(r'(\w)-\s+(\w)', r'\1\2', normalize_glyphs(text))
    text = re.sub(r'\b(pages?)\s+[\dxivlc]+(?:[–-][\dxivlc]+)?', r'\1 #', text)
    return text.split()


@dataclass
class Report:
    mismatch_ratio: float
    hunks: list = field(default_factory=list)


def compare(original, rebuilt):
    matcher = difflib.SequenceMatcher(a=original, b=rebuilt, autojunk=False)
    missing, hunks = 0, []
    for tag, i1, i2, j1, j2 in matcher.get_opcodes():
        if tag == 'equal':
            continue
        a, b = original[i1:i2], rebuilt[j1:j2]
        if tag == 'replace' and all(t.startswith('??') for t in b) and all(NUMBER.match(t) for t in a):
            continue
        missing += len(a)
        hunks.append((a, b))
    return Report(missing / max(len(original), 1), hunks)


def original_spans(key):
    first, last = json.loads((ROOT / 'build/extract/chapters.json').read_text())[key]
    spans = []
    for page in range(first, last + 1):
        spans += json.loads((ROOT / f'build/extract/p{page:03d}.json').read_text())['spans']
    return spans


def rebuilt_spans(key):
    import pymupdf
    out = ROOT / 'build/verify' / key
    subprocess.run(['latexmk', '-lualatex', '-interaction=nonstopmode', '-halt-on-error', f'-outdir={out}',
                    f'-usepretex=\\def\\IncludeOnly{{{FILES[key]}}}', 'main.tex'],
                   cwd=ROOT / 'source', check=True, stdout=subprocess.DEVNULL)
    from extract_figures import placements
    doc = pymupdf.open(out / 'main.pdf')
    spans = []
    for page in doc:
        boxes = [tuple(page.get_image_bbox(i)) for i in page.get_images(full=True)]
        boxes += [tuple(rect) for _, rect in placements(doc, page)]
        for block in page.get_text('dict')['blocks']:
            for line in block.get('lines', []):
                for span in line['spans']:
                    font = span['font'].replace('XCharter-Math', 'MathDesign-Charter')
                    spans.append({'text': span['text'], 'cls': classify(font, span['bbox'], page.rect.height, boxes)})
    return spans


def structure(key):
    text = (ROOT / 'source' / f'{FILES[key]}.tex').read_text()
    return {'figures': len(re.findall(r'\\caption\{', text)), 'footnotes': len(re.findall(r'\\footnote\{', text)),
            'exercises': len(re.findall(r'\\item\b', text.split('\\begin{exercises}')[-1])) if '\\begin{exercises}' in text else 0,
            'procs': sorted(set(re.findall(r'\\Proc\{([^}]*)\}', text)))}


def main(key):
    report = compare(words(original_spans(key)), words(rebuilt_spans(key)))
    lines = [f'{key}: mismatch {report.mismatch_ratio:.4%} (limit {MAX_RATIO:.2%})',
             f'structure: {json.dumps(structure(key))}']
    lines += [f'- {" ".join(a)!r} -> {" ".join(b)!r}' for a, b in report.hunks]
    out = ROOT / 'build/verify' / f'{key}.txt'
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text('\n'.join(lines) + '\n')
    print('\n'.join(lines[:40]))
    return 0 if report.mismatch_ratio <= MAX_RATIO else 1


if __name__ == '__main__':
    sys.exit(main(sys.argv[1]))
```

구조 개수(그림·각주·연습문제·절차)는 보고서에 출력하고, 원서 쪽 개수와의 대조는 복원 에이전트가 보고서와 원서 이미지(캡션 "Figure N.M" 수, 각주 링크 수 — JSON `links` 중 `name`이 `Hfootnote`로 시작하는 것, 연습문제 번호 최대값, Index of Pseudocode의 해당 장 절차)로 확인해 보고서 끝에 "structure: ok" 또는 차이를 적는다. 파일럿(Task 6)에서 이 대조를 자동화할 수 있는 형태가 확인되면 `structure()`와 원서 쪽 추출을 함께 확장하고 테스트를 추가한다.

Run: `cd $P/scripts && nd python3 -m unittest test_verify_source` → PASS.

- [ ] **Step 3: 커밋**

```bash
git add projects/jeffe-algorithms/scripts/verify_source.py projects/jeffe-algorithms/scripts/test_verify_source.py
git commit -m "[jeffe-algorithms] feat: 복원 원고를 원서 텍스트와 대조하는 검증 스크립트 추가" -m "Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>"
```

---

### Task 6: 파일럿 복원 — 2장 Backtracking

**Files:** `$P/source/chapters/02-backtracking.tex`, 필요 시 `preamble.tex`, `RECONSTRUCTION.md`, `scripts/verify_source.py`

- [ ] **Step 1: 복원**

`build/extract/chapters.json`의 `"02"` 범위(약 26쪽)를 쪽마다 처리한다. 각 쪽에 대해 `pNNN.png`를 Read 도구로 보고, `pNNN.json`의 스팬 텍스트를 옮겨 `RECONSTRUCTION.md` 규칙대로 LaTeX를 쓴다. 쪽 경계에서 문단이 이어지면 한 문단으로 잇는다. 그림은 `source/figures/manifest.json`에서 해당 쪽 파일을 찾아 넣는다.

- [ ] **Step 2: 검증과 수정 반복**

```bash
cd projects/jeffe-algorithms && nd python3 scripts/verify_source.py 02
```

기대: `mismatch ≤ 0.50%`. 보고서의 각 불일치 구간을 원서 이미지와 대조해 원고 오류면 고친다. 추출 잡음(글리프, 머리말 누락 등)이면 `extract.py`/`verify_source.py`의 정규화를 고치고 테스트를 추가한다. 수식 스팬은 이 파일럿에서 원서와 복원본 PDF를 나란히 렌더링해(`nd pdftoppm -r 100 -f <쪽> -l <쪽> -png …` 결과를 `build/`에 저장해 Read로 비교) 쪽마다 눈으로 확인한다. 의사코드 모양, 연습문제 난이도 기호, 각주 모양이 원서와 크게 다르면 `preamble.tex`를 조정한다.

- [ ] **Step 3: 파일럿 결론 기록**

`RECONSTRUCTION.md` 끝에 "파일럿(2장) 결과" 절을 추가한다: 불일치 비율, 남은 불일치의 종류, 어휘 조정, 쪽당 소요 시간, 수식 검토에서 찾은 오류 수. 어휘나 기준이 바뀌었으면 spec의 해당 절도 고친다.

- [ ] **Step 4: 커밋**

```bash
cd ../.. && git add projects/jeffe-algorithms docs/superpowers/specs/2026-09-20-jeffe-algorithms-korean-translation-design.md
git commit -m "[jeffe-algorithms] feat: 파일럿으로 2장 Backtracking 원고 복원" -m "Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>"
```

---

### Task 7: yeokja 설정과 파일럿 번역·한국어 빌드

**Files:**
- Create: `$P/yeokja.toml`, `$P/glossary.toml`, `$P/scripts/prepare_pdf.py`, `$P/scripts/test_prepare_pdf.py`
- Create: `$P/state/source/chapters/02-backtracking.tex.yeokja.json`

- [ ] **Step 1: 설정**

`$P/yeokja.toml`:

```toml
[project]
source_lang = "en"
target_lang = "ko"
glossary = "glossary.toml"
state_dir = "state"

[[sources]]
path = "source"
pattern = "chapters/*.tex"
parser = "latex"
output = "ko/{path}"

[[sources]]
path = "source"
pattern = "frontmatter/preface.tex"
parser = "latex"
output = "ko/{path}"

[[sources]]
path = "source"
pattern = "backmatter/*.tex"
parser = "latex"
output = "ko/{path}"

[derive]
base = "source"

[[derive.overlay]]
path = "ko"
require_base = true

[build]
dist = "output/pdf"

[build.pdf]
command = '''
set -eu
python3 "$YEOKJA_ROOT/scripts/prepare_pdf.py" .
latexmk -lualatex -interaction=nonstopmode -halt-on-error -usepretex='\def\KoreanEdition{1}' main.tex
if grep -Eq 'Missing character|There were undefined references|multiply defined|Reference .* undefined' main.log; then
  echo "PDF validation failed: missing glyphs or unresolved references" >&2
  exit 1
fi
cp main.pdf Algorithms-ko.pdf
'''
outputs = ["Algorithms-ko.pdf"]

[provider]
type = "claude_code"
model = "claude-sonnet-5"

[evaluation]
auto_evaluate = true
style_evaluate = false
max_retries = 3

[translation]
concurrency = 8
```

`{path}`는 source `path` 기준 상대 경로다(webassembly-component-docs가 `output = "ko/component-model/src/{path}"`로 쓰는 것과 같은 규칙). 그래서 번역문은 `ko/chapters/…`에 생기고 `source/` 위에 `ko/`를 겹친다. `[build] dist` 키는 `projects/hott/yeokja.toml`의 `dist = "output/pdf"`와 같은 위치에 둔다.

`$P/glossary.toml`(spec "용어집" 절의 목록):

```toml
# Algorithms (Jeff Erickson) 용어집. 인명과 알고리즘 고유명사는 원어로 둔다.
[terms.recursion]
translation = "재귀"
[terms.backtracking]
translation = "백트래킹"
[terms."dynamic programming"]
translation = "동적 계획법"
[terms."greedy algorithm"]
translation = "탐욕 알고리즘"
[terms.graph]
translation = "그래프"
[terms.vertex]
translation = "정점"
[terms.edge]
translation = "간선"
[terms."depth-first search"]
translation = "깊이 우선 탐색"
[terms."breadth-first search"]
translation = "너비 우선 탐색"
[terms."minimum spanning tree"]
translation = "최소 신장 트리"
[terms."shortest path"]
translation = "최단 경로"
[terms."maximum flow"]
translation = "최대 흐름"
[terms."minimum cut"]
translation = "최소 컷"
[terms."NP-hard"]
translation = "NP-난해"
[terms.reduction]
translation = "환원"
[terms."running time"]
translation = "실행 시간"
[terms.recurrence]
translation = "점화식"
[terms.subproblem]
translation = "부분 문제"
[terms.memoization]
translation = "메모이제이션"
[terms.induction]
translation = "귀납법"
[terms.invariant]
translation = "불변식"
```

- [ ] **Step 2: `prepare_pdf.py`(테스트 먼저)**

조립 트리에서 할 일은 한국어 판권 면이 번역 대상 파일이 아니라 base(`source/`)에 그대로 있는지 확인하는 것뿐이다(`\KoreanEdition`은 latexmk 인자로 켠다). 번역된 파일에서 LuaTeX 한글 경계 문제는 `latex` 파서가 이미 처리한다. 따라서 `prepare_pdf.py`는 트리의 `main.tex`와 `frontmatter/translation-notice.tex`가 있는지, 번역된 장 파일에 `\koreantrue` 같은 전환 명령이 섞이지 않았는지 검사하고 실패 시 종료한다.

```python
"""Sanity-check the assembled tree before the Korean LuaLaTeX build."""
from pathlib import Path
import sys


def check(tree):
    tree = Path(tree)
    problems = [f'missing {p}' for p in ('main.tex', 'preamble.tex', 'frontmatter/translation-notice.tex')
                if not (tree / p).exists()]
    for tex in (tree / 'chapters').glob('*.tex'):
        if '\\koreantrue' in tex.read_text():
            problems.append(f'{tex.name}: edition switch inside a chapter')
    return problems


if __name__ == '__main__':
    problems = check(sys.argv[1])
    for p in problems:
        print(p, file=sys.stderr)
    sys.exit(1 if problems else 0)
```

`test_prepare_pdf.py`:

```python
from pathlib import Path
import tempfile
import unittest
from prepare_pdf import check


class CheckTests(unittest.TestCase):
    def test_complete_tree_passes(self):
        with tempfile.TemporaryDirectory() as d:
            t = Path(d)
            (t / 'frontmatter').mkdir()
            (t / 'chapters').mkdir()
            for p in ('main.tex', 'preamble.tex', 'frontmatter/translation-notice.tex', 'chapters/02.tex'):
                (t / p).write_text('x')
            self.assertEqual(check(t), [])

    def test_missing_notice_fails(self):
        with tempfile.TemporaryDirectory() as d:
            (Path(d) / 'main.tex').write_text('x')
            self.assertIn('missing frontmatter/translation-notice.tex', check(d))


if __name__ == '__main__':
    unittest.main()
```

- [ ] **Step 3: 분할 확인 — 의사코드 주석**

```bash
cd projects/jeffe-algorithms
$Y status source | tail -3
$Y coverage source/chapters/02-backtracking.tex --min-lines 3
```

`algorithm` 환경 안의 `\Comment{…}` 텍스트와 캡션이 세그먼트로 제시되는지 확인한다(`$Y translate` 전에 `status`의 세그먼트 수와 `coverage`가 건너뛴 구간으로 판단). 주석이 건너뛰어지면 다음 중 하나를 한다(작은 것부터):
1. `parser = "latex-extended"`로 바꿔 다시 확인한다.
2. `crates/parser-latex/src/lib.rs`의 가시 텍스트 명령 목록(78행 근처 "Commands whose arguments are ordinary visible text")에 `Comment`를 추가하고, `\begin{algorithm}…\Comment{Recursion!}…\end{algorithm}`에서 `Recursion!`만 보조 세그먼트로 나오는 단위 테스트를 먼저 쓴다. `[*] feat: latex 파서가 의사코드 \Comment 인자를 번역 대상으로 제시` 커밋으로 분리한다. 기존 latex 프로젝트(`napkin`, `hott` 등)에 `\Comment`가 쓰이는지 `grep -rl '\\Comment{' projects/*/upstream`로 확인하고, 쓰이면 그 프로젝트의 `status`에서 새 세그먼트가 생기는지 보고 번역한다.

- [ ] **Step 4: 파일럿 번역과 빌드**

```bash
pgrep -fl 'yeokja translate' || echo free
$Y translate source/chapters/02-backtracking.tex
$Y evaluate --mechanical-only source/chapters/02-backtracking.tex | tail -10
nd $Y build pdf 2>&1 | tail -5
```

`output/pdf/Algorithms-ko.pdf`(아직 다른 장은 영어)를 열어 2장 쪽들을 이미지로 확인한다(`nd pdftoppm -r 80 -f N -l N -png output/pdf/Algorithms-ko.pdf build/ko-check`). 한글 조판, 의사코드 주석 번역, 그림, `\figref`의 "그림 2.3" 표기, 연습문제를 본다. 문제가 있으면 프리앰블·용어집을 고치고 재번역한다.

- [ ] **Step 5: 커밋**

```bash
cd ../.. && git add projects/jeffe-algorithms/yeokja.toml projects/jeffe-algorithms/glossary.toml projects/jeffe-algorithms/scripts/prepare_pdf.py projects/jeffe-algorithms/scripts/test_prepare_pdf.py projects/jeffe-algorithms/state
git commit -m "[jeffe-algorithms] feat: yeokja 번역 설정과 2장 파일럿 번역" -m "Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>"
```

---

### Task 8: 나머지 원고 복원

**Files:** `$P/source/frontmatter/preface.tex`, `chapters/00…12`(02 제외), `backmatter/credits.tex`, `backmatter/colophon.tex`

- [ ] **Step 1: 장별 병렬 복원**

장마다 서브에이전트 하나를 보낸다(동시 3~4개). 각 서브에이전트 지시: "`projects/jeffe-algorithms/RECONSTRUCTION.md`(파일럿 결과 절 포함)와 `source/preamble.tex`, 완성된 `source/chapters/02-backtracking.tex`(형식 예시)를 읽고, `build/extract/chapters.json`의 `<키>` 범위를 쪽마다 복원해 `source/<파일>.tex`에 쓴 뒤 `scripts/verify_source.py <키>`가 통과할 때까지 고쳐라. 다른 파일은 건드리지 마라. 커밋하지 마라. 보고: 불일치 비율, 남은 불일치 구간과 판정, 구조 대조 결과." 서브에이전트는 자기 장 파일만 쓰므로 충돌하지 않는다. `preamble.tex` 수정이 필요하면 직접 고치지 말고 보고하게 하고, 조정은 이 태스크를 실행하는 사람이 한 번에 반영한 뒤 영향받는 장을 다시 검증한다.

키: `preface`, `00`, `01`, `03`~`12`, `image-credits`, `colophon`(`chapters.json`의 실제 키 이름을 쓴다). Image Credits는 그림 1.25 항목을 "생략함"으로 표시하는 문장을 영어로 덧붙이지 말고 원문 그대로 복원한다(한국어판 판권 면이 생략을 밝힌다). 그림 5.2의 Flickr 라이선스를 확인해(`https://www.flickr.com/photos/yalelawlibrary/albums/72157621954683764`) CC BY 계열이 아니어서 수정 없는 재수록도 허용되지 않는 조건(예: 없음)이 있으면 그림 1.25처럼 제외하고 이 태스크에 기록한다.

- [ ] **Step 2: 장별 결과 검토**

각 장의 `build/verify/<키>.txt`를 읽고, 수식 밀도가 높은 쪽(`pNNN.json`에서 `math` 스팬 문자 비율 ≥ 10%인 74쪽)을 원서·복원본 렌더링으로 나란히 확인한다. 오류는 고치고 재검증한다.

- [ ] **Step 3: 전체 영어 빌드**

```bash
cd projects/jeffe-algorithms/source
nd latexmk -lualatex -interaction=nonstopmode -halt-on-error -outdir=../build/en-full main.tex
grep -E 'undefined|Missing character|multiply defined' ../build/en-full/main.log || echo clean
```

기대: `clean`. 장 사이 참조(`\figref{fig:1.3}` 등)가 모두 풀린다. 목차가 원서 목차(212항목 중 범위 안 항목)와 같은 순서·번호인지 `nd python3 -c` 로 두 PDF의 `get_toc()`를 비교한다(색인 항목 제외).

- [ ] **Step 4: 커밋(장 단위로 나눠도 된다)**

```bash
cd ../../.. && git add projects/jeffe-algorithms/source projects/jeffe-algorithms/RECONSTRUCTION.md
git commit -m "[jeffe-algorithms] feat: 전체 영어 원고 복원" -m "모든 장이 원서 텍스트 대조 기준(불일치 0.5% 이하)을 통과했다." -m "Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>"
```

---

### Task 9: 전체 번역과 한국어 PDF

- [ ] **Step 1: 전체 번역**

```bash
pgrep -fl 'yeokja translate' || echo free
cd projects/jeffe-algorithms
$Y translate source > build/translate.log 2>&1
$Y status --check source
$Y evaluate --mechanical-only source 2>&1 | tail -30
```

중단되면 다시 실행한다. 평가 오류는 state를 고치거나 재번역한다.

- [ ] **Step 2: 빌드와 검사**

```bash
nd $Y build pdf 2>&1 | tail -5
nd pdfinfo output/pdf/Algorithms-ko.pdf | grep Pages
```

기대: 빌드 성공(검사 grep 통과). 무작위 10쪽과 수식·의사코드가 많은 5쪽을 이미지로 확인한다(`pdftoppm`으로 `build/`에 저장 후 Read). 판권 면, 차례, 그림 번호·참조, 각주, 연습문제를 확인한다.

- [ ] **Step 3: 커밋**

```bash
cd ../.. && git add projects/jeffe-algorithms/state projects/jeffe-algorithms/glossary.toml
git commit -m "[jeffe-algorithms] feat: 전체 한국어 번역" -m "Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>"
```

---

### Task 10: 배포 배선, 랜딩, README

**Files:** `.github/pages-projects.json`, `.github/scripts/stage-pages.sh`, `.github/scripts/rebuild-translations.sh`, `.github/workflows/pr.yml`, `site/index.html`, `$P/README.md`

- [ ] **Step 1: 배선**

`.github/pages-projects.json` 끝에:

```json
  {
    "project": "jeffe-algorithms",
    "target": "pdf",
    "artifact": "dist-jeffe-algorithms-pdf",
    "artifact_path": "projects/jeffe-algorithms/output/pdf"
  }
```

`stage-pages.sh`: `overlay_download "dist-hott-pdf" "HoTT-ko.pdf" "hott"` 다음 줄에 `overlay_download "dist-jeffe-algorithms-pdf" "Algorithms-ko.pdf" "jeffe-algorithms"`. 필수 산출물 검사 목록(`test -s "$site_dir/hott/HoTT-ko.pdf"` 근처)에 같은 형식으로 `jeffe-algorithms/Algorithms-ko.pdf`를 추가한다.

`rebuild-translations.sh`:

```bash
  jeffe-algorithms)
    # 복원한 영어 원고(source/)의 모든 번역 대상 파일을 확인합니다.
    "$yeokja" translate source
    "$yeokja" status --check source
    ;;
```

`pr.yml`: rustc-dev-guide 테스트 스텝 뒤에

```yaml
      - name: Test Algorithms (Jeff Erickson) reconstruction scripts
        run: python3 -m unittest discover -s projects/jeffe-algorithms/scripts -p 'test_*.py'
```

PyMuPDF가 필요한 테스트(`test_extract_figures`, `test_extract` 일부, `test_verify_source`의 import)는 CI 러너에 PyMuPDF가 없으면 실패한다. `pr.yml`이 Python 의존성을 설치하는 방식을 확인해 `pip install pymupdf`를 추가하거나, 해당 테스트 모듈 첫머리에서 `pymupdf`가 없으면 `unittest.SkipTest`를 던지게 한다(순수 함수 테스트는 항상 실행되도록 import를 함수 안으로 옮긴다 — `extract.py`는 이미 `main` 안에서 import한다).

- [ ] **Step 2: 랜딩**

`site/index.html`에 "알고리즘" 주제 섹션이 없으면 추가한다(cp-algorithms plan Task 9 Step 2와 같은 섹션·필터 버튼). 항목:

```html
      <li class="work" data-topics="algorithms">
        <div class="subjects"><span>알고리즘</span></div>
        <h4><a class="work-title" href="jeffe-algorithms/Algorithms-ko.pdf">알고리즘 (Jeff Erickson)</a></h4>
        <p class="work-desc">재귀, 백트래킹, 동적 계획법, 탐욕 알고리즘부터 그래프 탐색, 최단 경로, 최대 흐름, NP-난해성까지 다루는 일리노이 대학교 알고리즘 교재입니다. 원서 PDF에서 원고를 복원해 번역했습니다.</p>
        <p class="work-meta">원문 <a href="http://algorithms.wtf">Jeff Erickson</a><span class="tag">CC BY 4.0</span><span class="tag">기계 번역</span></p>
      </li>
```

- [ ] **Step 3: 실제 모델 확인과 README**

```bash
git log -p --format='%h %cd' -- projects/jeffe-algorithms/yeokja.toml | grep -E '^[0-9a-f]{7} |model|type ='
```

`$P/README.md`:

````markdown
# 알고리즘 (Jeff Erickson, *Algorithms* 한국어 번역)

Jeff Erickson의 교재 [*Algorithms*](http://algorithms.wtf)(1st edition, 2019)를 한국어로 옮긴
비공식 번역입니다. [yeokja](https://github.com/yeokja/yeokja)와 함께 Anthropic 사의 `claude-sonnet-5`
모델을 활용하여 번역되었으며 학습을 모두 비허용한 상태로 작업하였습니다. 저자가 검토하거나 승인한
번역이 아닙니다.

## 원서와 영어 원고

저자는 LaTeX 원고를 공개하지 않았습니다. 이 프로젝트의 `source/`는 원서 PDF
(`upstream/Algorithms-JeffE.pdf`, 저장소 커밋 `<해시>`, SHA-256 `<해시>`)에서 **복원한 영어 원고**이며
저자의 원고가 아닙니다. 복원은 Anthropic 사의 `<복원 서브에이전트 모델>` 모델이 쪽 이미지와 PDF
텍스트 레이어를 보고 LaTeX를 쓰는 방식으로 했고, 다시 컴파일한 텍스트를 원서와 단어 단위로 대조해
장마다 불일치 0.5% 이하와 그림·각주·연습문제 수 일치를 확인했습니다(`scripts/verify_source.py`,
절차는 `RECONSTRUCTION.md`).

## 범위

- 포함: 서문, 0~12장(연습문제·각주 포함), Image Credits, Colophon
- 제외: 색인 3종, 강의 노트(CC BY-NC-SA 4.0으로 라이선스가 다름), 그림 1.25(작가의 허락으로만 수록된 초상화)
- 그림은 원서 PDF에서 잘라낸 것이며 그림 안의 영어 문구는 번역하지 않았습니다.

## 라이선스

원서 © 2019 Jeff Erickson, [CC BY 4.0](https://creativecommons.org/licenses/by/4.0/).
복원 원고와 번역본은 원서를 수정한 개작물로서 같은 CC BY 4.0으로 제공합니다.
비저자 그림의 출처와 라이선스는 번역본의 Image Credits에 원서대로 적혀 있습니다.

## 재현

```sh
cd projects/jeffe-algorithms
nix develop path:../../nix#jeffe-algorithms -c python3 scripts/extract_figures.py upstream/Algorithms-JeffE.pdf source/figures build/extract/figures.json
nix develop path:../../nix#jeffe-algorithms -c python3 scripts/extract.py upstream/Algorithms-JeffE.pdf build/extract build/extract/figures.json
nix develop path:../../nix#jeffe-algorithms -c python3 scripts/verify_source.py 02     # 장별 원고 검증
../../target/release/yeokja translate source
../../target/release/yeokja status --check source
nix develop path:../../nix#jeffe-algorithms -c ../../target/release/yeokja build pdf    # output/pdf/Algorithms-ko.pdf
```

`state/`와 `source/`는 커밋합니다. `ko/`, `build/`, `output/`은 재생성되므로 커밋하지 않습니다.
````

`<…>`는 실제 값으로 채운다(복원 모델은 Task 6·8 서브에이전트가 실제로 쓴 모델).

- [ ] **Step 4: 검증과 커밋**

```bash
python3 -m json.tool .github/pages-projects.json > /dev/null
python3 -m unittest discover -s .github/scripts -p 'test_*.py'
rm -rf projects/jeffe-algorithms/ko && .github/scripts/rebuild-translations.sh jeffe-algorithms
(cd projects/jeffe-algorithms && nix develop path:../../nix#jeffe-algorithms -c ../../target/release/yeokja build pdf 2>&1 | tail -3)
git status --short
git add .github site/index.html projects/jeffe-algorithms/README.md projects/jeffe-algorithms/source/frontmatter/translation-notice.tex
git commit -m "[jeffe-algorithms] feat: Pages 배포 배선과 README 추가" -m "(#3)" -m "Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>"
```
