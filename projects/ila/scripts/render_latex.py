#!/usr/bin/env python3
"""Replace the LaTeX placeholders in the generated HTML with SVG images.

    render_latex.py HTML_DIR OUT_DIR --preamble FILE --style-dir DIR
                    --image-dir DIR --cache-dir DIR [--jobs N]

The book's stylesheet (src/xsl/mathbook-html.xsl) writes every formula and
TikZ picture as a placeholder, ``<script type="text/x-latex-KIND">CODE</script>``,
for the original build's pretex.py to compile. This script plays that part
with tools Nix can provide: each distinct snippet is typeset once with
``latex`` (DVI) and converted with ``dvisvgm``, and the placeholder becomes an
``<img>`` of the SVG, sized in ``em`` so that 12 TeX points are one em of the
surrounding text, as in the original.

KIND is one of:
    inline        inline math, ``$CODE$``
    code-inline   inline TeX in text mode
    display       a display (``equation*``, ``align*`` ...); a lone ``\\tag``
                  becomes an HTML label at the right, as in the original
    code          a TikZ picture or other TeX in text mode, shown as a display
    code-bare     TeX run for its side effects (``\\usetikzlibrary``); it
                  applies to the snippets after it in the same HTML file

Rendered snippets are cached by a hash of everything that went into them
(the TeX preamble, the bare code before them, the kind and the code), so a
rebuild only compiles what changed. The SVG files go to OUT_DIR/latex/; HTML
files are written to OUT_DIR with the same relative paths. A snippet that
fails to compile is reported and makes the run fail.
"""

import argparse
import base64
import concurrent.futures
import hashlib
import html
import json
import mimetypes
import os
import re
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

SCRIPT = re.compile(r'<script type="text/x-latex-([a-z-]+)">(.*?)</script>', re.S)
TAG = re.compile(r"\\tag\{([^{}]*)\}")
EM = 12.0  # TeX points per em: the original typeset at 12pt (amsart, 12pt)
BATCH = 40
VERSION = "4"  # bump to invalidate the cache when this script's output changes

DOCUMENT = r"""\documentclass[12pt,reqno,dvisvgm]{amsart}
\usepackage[charter,sfscaled,ttscaled,cal=cmcal]{mathdesign}
\renewcommand{\sfdefault}{phv}
\usepackage{textcomp}
\usepackage{kotex}
\usepackage{graphicx}
\graphicspath{{figure-images/}{.}}
%(preamble)s
%(context)s
\pagestyle{empty}
\hoffset=-1in \voffset=-1in \oddsidemargin=0pt \evensidemargin=0pt
\topmargin=0pt \headheight=0pt \headsep=0pt \footskip=0pt
\topskip=0pt \parindent=0pt \parskip=0pt
\paperwidth=8in \paperheight=30in \textwidth=7in \textheight=29in \hsize=7in
\newsavebox\yeokjabox
\newwrite\yeokjasizes
\immediate\openout\yeokjasizes=\jobname.sizes
\begin{document}%%
"""

# An inline snippet is a box shipped out as its own page, whose paper is the
# box: dvisvgm --bbox=papersize then keeps the baseline (depth) exactly.
INLINE = r"""\sbox\yeokjabox{%(open)s%(code)s%(close)s}%%
\immediate\write\yeokjasizes{\the\wd\yeokjabox,\the\ht\yeokjabox,\the\dp\yeokjabox}%%
\shipout\vbox{\special{papersize=\the\wd\yeokjabox,\the\dimexpr\ht\yeokjabox+\dp\yeokjabox\relax}\usebox\yeokjabox}%%
"""

# A display is set in a 7in column (the original's page width) and cropped to
# what it draws: dvisvgm --bbox=min.
DISPLAY = r"""\thispagestyle{empty}%%
\begin{minipage}{7in}%%
%(code)s%%
\end{minipage}%%
\newpage
"""


def sha(*parts: str) -> str:
    h = hashlib.sha256()
    for part in parts:
        h.update(part.encode("utf-8"))
        h.update(b"\0")
    return h.hexdigest()[:24]


class Snippet:
    def __init__(self, kind: str, code: str, context: str, preamble: str):
        self.kind = kind
        self.code = code.strip()
        self.context = context
        self.tag = None
        if kind == "display":
            tags = TAG.findall(self.code)
            if len(tags) == 1:
                # Drawn as HTML next to the image, as pretex did.
                self.tag = tags[0]
                self.code = TAG.sub("", self.code)
                # A \tag on its own line leaves a blank line, a \par in math.
                self.code = re.sub(r"\n\s*\n", "\n", self.code)
                self.code = re.sub(r"\\(begin|end)\{(equation|align|gather|multline)\}", r"\\\1{\2*}", self.code)
            elif tags:
                self.code = TAG.sub(r"\\tag*{(\1)}", self.code)
        self.key = sha(VERSION, preamble, context, kind, self.code)

    @property
    def inline(self) -> bool:
        return self.kind in ("inline", "code-inline")


def scan(path: Path, preamble: str) -> list:
    """The snippets of one HTML file, in order, as (match, Snippet|None)."""
    text = path.read_text(encoding="utf-8")
    out, context = [], ""
    for m in SCRIPT.finditer(text):
        # xsltproc's HTML output writes script content raw (no entities).
        kind, code = m.group(1), m.group(2)
        if kind == "code-bare":
            context += code.strip() + "\n"
            out.append((m, None))
            continue
        if kind not in ("inline", "code-inline", "display", "code"):
            raise SystemExit(f"{path}: unknown placeholder type text/x-latex-{kind}")
        out.append((m, Snippet(kind, code, context, preamble) if code.strip() else None))
    return out


def run(cmd: list, cwd: Path, env: dict) -> subprocess.CompletedProcess:
    return subprocess.run(cmd, cwd=cwd, env=env, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True, errors="replace")


def compile_batch(snippets: list, preamble: str, env: dict, image_dir: Path) -> dict:
    """Render snippets sharing one context. Returns key -> result dict."""
    inline = snippets[0].inline
    with tempfile.TemporaryDirectory(prefix="ila-latex-") as tmp:
        tmp = Path(tmp)
        (tmp / "figure-images").symlink_to(image_dir.resolve(), target_is_directory=True)
        body = [DOCUMENT % {"preamble": preamble, "context": snippets[0].context}]
        for s in snippets:
            if inline:
                math = s.kind == "inline"
                body.append(INLINE % {"open": "$" if math else "", "close": "$" if math else "", "code": s.code})
            else:
                body.append(DISPLAY % {"code": s.code})
        body.append(r"\end{document}" + "\n")
        (tmp / "doc.tex").write_text("".join(body), encoding="utf-8")
        latex = run(["latex", "-interaction=nonstopmode", "-halt-on-error", "doc.tex"], tmp, env)
        log = (tmp / "doc.log").read_text(encoding="utf-8", errors="replace") if (tmp / "doc.log").exists() else latex.stdout
        if latex.returncode != 0 or re.search(r"^! ", log, re.M):
            return {s.key: {"error": tail(log)} for s in snippets}
        sizes = (tmp / "doc.sizes").read_text().split() if inline else []
        bbox = "papersize" if inline else "min"
        svg = run(["dvisvgm", "--no-fonts", "--embed-bitmaps", "--exact-bbox", f"--bbox={bbox}", "--page=1-", "--output=p%p.svg", "doc.dvi"], tmp, env)
        results = {}
        for i, s in enumerate(snippets, start=1):
            page = tmp / f"p{i:0{len(str(len(snippets)))}d}.svg"
            if not page.exists():
                page = tmp / f"p{i}.svg"
            if svg.returncode != 0 or not page.exists():
                results[s.key] = {"error": tail(svg.stdout)}
                continue
            data = embed_bitmaps(page.read_text(encoding="utf-8"), image_dir)
            w, h = svg_size(data)
            r = {"svg": data, "width": w, "height": h}
            if inline:
                wd, ht, dp = (float(x.removesuffix("pt")) for x in sizes[i - 1].split(","))
                r.update(width=wd, height=ht + dp, depth=dp)
            results[s.key] = r
        return results


def embed_bitmaps(data: str, image_dir: Path) -> str:
    """Inline the photos TikZ pictures include. graphicx's dvisvgm driver
    writes them as raw <image> links, which dvisvgm --embed-bitmaps does not
    touch, and an SVG shown with <img> cannot load a linked file."""
    def repl(m):
        path = image_dir / m.group(2)
        mime = mimetypes.guess_type(path.name)[0] or "application/octet-stream"
        return f"{m.group(1)}data:{mime};base64,{base64.b64encode(path.read_bytes()).decode('ascii')}{m.group(3)}"
    return re.sub(r"""(xlink:href=['"])figure-images/([^'"]+)(['"])""", repl, data)


def svg_size(data: str) -> tuple:
    """Width and height of a dvisvgm SVG in TeX points (dvisvgm writes bp)."""
    m = re.search(r'<svg[^>]*\swidth=["\']([\d.]+)pt["\'][^>]*\sheight=["\']([\d.]+)pt["\']', data)
    if not m:
        raise ValueError("SVG without a width/height in pt")
    return float(m.group(1)) * 72.27 / 72, float(m.group(2)) * 72.27 / 72


def tail(log: str) -> str:
    lines = log.splitlines()
    for i, line in enumerate(lines):
        if line.startswith("! "):
            return "\n".join(lines[i:i + 8])
    return "\n".join(lines[-15:])


def render_all(snippets: dict, preamble: str, cache: Path, env: dict, image_dir: Path, jobs: int) -> dict:
    results = {}
    todo = []
    for key, s in snippets.items():
        meta = cache / f"{key}.json"
        if meta.exists():
            results[key] = json.loads(meta.read_text(encoding="utf-8"))
        else:
            todo.append(s)
    groups = {}
    for s in todo:
        groups.setdefault((s.context, s.inline), []).append(s)
    batches = [g[i:i + BATCH] for g in groups.values() for i in range(0, len(g), BATCH)]
    print(f"render_latex: {len(snippets)} distinct snippets, {len(snippets) - len(todo)} cached, {len(todo)} to render in {len(batches)} batches")

    def work(batch: list) -> dict:
        out = compile_batch(batch, preamble, env, image_dir)
        if len(batch) > 1 and any("error" in r for r in out.values()):
            # One bad snippet spoils its batch: render them one by one.
            out = {}
            for s in batch:
                out.update(compile_batch([s], preamble, env, image_dir))
        return out

    with concurrent.futures.ThreadPoolExecutor(max_workers=jobs) as pool:
        for out in pool.map(work, batches):
            for key, r in out.items():
                if "error" not in r:
                    (cache / f"{key}.svg").write_text(r.pop("svg"), encoding="utf-8")
                    (cache / f"{key}.json").write_text(json.dumps(r), encoding="utf-8")
                results[key] = r
    return results


def em(pt: float) -> str:
    return f"{pt / EM:.4f}".rstrip("0").rstrip(".") + "em"


def image(s: Snippet, r: dict) -> str:
    alt = html.escape(s.code, quote=True)
    src = f"latex/{s.key}.svg"
    if s.inline:
        style = f"width:{em(r['width'])};height:{em(r['height'])};vertical-align:-{em(r['depth'])}"
        return f'<img class="pretex-inline" src="{src}" alt="{alt}" style="{style}">'
    style = f"width:{em(r['width'])};height:{em(r['height'])}"
    tag = f'<span class="tag">({html.escape(s.tag)})</span>' if s.tag else ""
    return f'<div class="pretex-display"><img src="{src}" alt="{alt}" style="{style}">{tag}</div>'


# Korean particles and punctuation stuck to a formula ("<m>A</m>의") must not
# be broken from it, as pretex's .pretex-bind did.
BIND = re.compile(r'([^\s<>]*)(<img class="pretex-inline"[^>]*>)([^\s<>]*)')


def bind(text: str) -> str:
    def repl(m):
        if not m.group(1) and not m.group(3):
            return m.group(0)
        return f'<span class="pretex-bind">{m.group(1)}{m.group(2)}{m.group(3)}</span>'
    return BIND.sub(repl, text)


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("html_dir", type=Path)
    ap.add_argument("out_dir", type=Path)
    ap.add_argument("--preamble", type=Path, required=True)
    ap.add_argument("--style-dir", type=Path, required=True)
    ap.add_argument("--image-dir", type=Path, required=True)
    ap.add_argument("--cache-dir", type=Path, required=True)
    ap.add_argument("--jobs", type=int, default=os.cpu_count() or 4)
    args = ap.parse_args()

    preamble = args.preamble.read_text(encoding="utf-8")
    for sty in sorted(args.style_dir.glob("*")):
        preamble += f"%% {sty.name} {hashlib.sha256(sty.read_bytes()).hexdigest()[:12]}\n"
    args.cache_dir.mkdir(parents=True, exist_ok=True)
    env = dict(os.environ)
    env["TEXINPUTS"] = f".:{args.style_dir.resolve()}//:" + env.get("TEXINPUTS", "")

    files = sorted(p for p in args.html_dir.rglob("*.html"))
    scanned = {p: scan(p, preamble) for p in files}
    snippets = {s.key: s for found in scanned.values() for _, s in found if s is not None}
    results = render_all(snippets, preamble, args.cache_dir, env, args.image_dir, args.jobs)

    failed = {k: r for k, r in results.items() if "error" in r}
    for key, r in sorted(failed.items()):
        print(f"render_latex: snippet {key} failed:\n{snippets[key].code}\n--\n{r['error']}\n", file=sys.stderr)

    latex_out = args.out_dir / "latex"
    latex_out.mkdir(parents=True, exist_ok=True)
    for key in snippets:
        if key not in failed:
            shutil.copyfile(args.cache_dir / f"{key}.svg", latex_out / f"{key}.svg")
    count = 0
    for path, found in scanned.items():
        text = path.read_text(encoding="utf-8")
        parts, last = [], 0
        for m, s in found:
            parts.append(text[last:m.start()])
            if s is not None and s.key not in failed:
                parts.append(image(s, results[s.key]))
                count += 1
            last = m.end()
        parts.append(text[last:])
        dest = args.out_dir / path.relative_to(args.html_dir)
        dest.parent.mkdir(parents=True, exist_ok=True)
        dest.write_text(bind("".join(parts)), encoding="utf-8")
    print(f"render_latex: {len(files)} HTML files, {count} placeholders replaced, {len(failed)} failed snippets")
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main())
