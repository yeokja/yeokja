#!/usr/bin/env python3
"""Build the Korean web edition of Interactive Linear Algebra.

    build_html.py TREE OUT

TREE is the assembled tree (upstream with ko/ and overlay/ laid over it) and
OUT the site directory to create. The steps follow the original SCons build
(SConstruct, src/SConscript) with the parts Nix cannot provide replaced:

1. ``src/xsl/git-hash.xsl`` and ``src/xsl/theme.xsl``, which SCons generated.
2. ``xsltproc --xinclude versioning.xsl`` keeps the default variant.
3. The Korean edition's additions (overlay ``src/ko-edition.xml``: title page
   credit and date, notices, History appendix) are spliced into the versioned book, the
   document language becomes ko-KR (the localization in overlay
   ``mathbook/xsl/localizations/ko-KR.xsl``), and the two links to the
   original site's variants become absolute.
4. ``xsltproc src/xsl/ila-ko.xsl`` (the book's stylesheet with Korean
   interface text) writes the HTML pages and knowls.
5. ``render_latex.py`` turns the LaTeX placeholders into SVG images
   (cache: $YEOKJA_ROOT/build/latex-cache).
6. CSS from mathbook-assets' SCSS (compass) and the book's add-ons, the
   concatenated JavaScript, fonts and images, as SConstruct bundles them.
"""

import os
import re
import shutil
import subprocess
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
ORIGINAL_SITE = "https://textbooks.math.gatech.edu/ila/"


def sh(cmd: list, cwd: Path, env: dict | None = None) -> str:
    print("+ " + " ".join(str(c) for c in cmd), flush=True)
    proc = subprocess.run(cmd, cwd=cwd, env=env, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True)
    if proc.returncode != 0:
        print(proc.stdout, file=sys.stderr)
        raise SystemExit(f"command failed ({proc.returncode}): {cmd[0]}")
    return proc.stdout


def upstream_commit(root: Path) -> str:
    try:
        out = subprocess.run(["git", "-C", str(root / "upstream"), "rev-parse", "HEAD"],
                             stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, text=True, check=True)
        return out.stdout.strip()
    except (subprocess.CalledProcessError, FileNotFoundError):
        raise SystemExit("cannot read the upstream commit (git -C upstream rev-parse HEAD)")


def generated_xsl(tree: Path, commit: str) -> None:
    xsl = tree / "src/xsl"
    text = (xsl / "git-hash.xsl.in").read_text(encoding="utf-8")
    for key, value in {"@GIT_HASH@": commit, "@GIT_HASH_SHORT@": commit[:6],
                       "@ILA_JS@": "ko", "@ILA_CSS@": "ko", "@COVER_JS@": "ko"}.items():
        text = text.replace(key, value)
    write(xsl / "git-hash.xsl", text)
    write(xsl / "theme.xsl", (xsl / "theme-gt.xsl").read_text(encoding="utf-8"))


def write(path: Path, text: str) -> None:
    # The tree is a symlink farm; never write through a link into upstream/.
    if path.is_symlink():
        path.unlink()
    path.write_text(text, encoding="utf-8")


def fragment(edition: str, name: str) -> str:
    m = re.search(rf"<{name}>\s*(.*?)\s*</{name}>", edition, re.S)
    if not m:
        raise SystemExit(f"ko-edition.xml has no <{name}>")
    return m.group(1)


def splice_once(text: str, anchor: str, insert: str, after: bool, what: str) -> str:
    if text.count(anchor) != 1:
        raise SystemExit(f"{what}: expected one {anchor!r} in the versioned book, found {text.count(anchor)}")
    i = text.index(anchor)
    if after:
        i += len(anchor)
    return text[:i] + insert + text[i:]


def korean_book(versioned: str, edition: str, commit: str) -> str:
    edition = edition.replace("@UPSTREAM_COMMIT@", commit[:7])
    text = versioned
    if text.count('xml:lang="en-US"') != 1:
        raise SystemExit("versioned book: expected one xml:lang=\"en-US\"")
    text = text.replace('xml:lang="en-US"', 'xml:lang="ko-KR"')
    # Title page: the translator after the authors (GFDL 4.B).
    text = splice_once(text, "<date>", fragment(edition, "titlepage-credit") + "\n    ", False, "title page")
    # The original dates the title page with the build day (<today/>).
    text, n = re.subn(r"<date>\s*<today\s*/>\s*</date>", fragment(edition, "titlepage-date"), text)
    if n != 1:
        raise SystemExit("versioned book: expected one <date><today/></date> on the title page")
    # Front colophon: the original notice in English, then ours (4.D-4.F).
    m = re.search(r"<colophon>\s*<copyright>.*?</copyright>", text, re.S)
    if not m:
        raise SystemExit("versioned book: no front colophon copyright")
    text = text[:m.end()] + "\n" + fragment(edition, "colophon-notice") + text[m.end():]
    # History (4.I), right before the license appendix.
    text = splice_once(text, '<appendix xml:id="appendix-gfdl"', fragment(edition, "history") + "\n", False, "history")
    # "Variants of this textbook" links to the original site's root-relative paths.
    for path in ("/ila/1553", "/ila"):
        old = f'<url href="{path}">'
        if old not in text:
            raise SystemExit(f"versioned book: no link {old}")
        text = text.replace(old, f'<url href="{ORIGINAL_SITE}{path.removeprefix("/ila").removeprefix("/")}">')
    return text


def bundle(tree: Path, out: Path, root: Path) -> None:
    env = dict(os.environ)
    # compass 1.0.3 calls File.exists?, which Ruby 3.2 removed.
    env["RUBYOPT"] = f"-r{HERE / 'compass-shim.rb'} " + env.get("RUBYOPT", "")
    assets = tree / "mathbook-assets"
    sh(["compass", "compile", "--css-dir=build", "--force", "--quiet", "scss/mathbook-gt.scss"], assets, env)
    css = [assets / "build/mathbook-gt.css", tree / "mathbook/css/mathbook-add-on.css",
           tree / "static/css/ila-add-on.css", tree / "static/css/ila-add-on-gt.css",
           tree / "vendor/knowlstyle.css", HERE / "ko.css"]
    js = [tree / "vendor/jquery.min.js", tree / "vendor/jquery.sticky.js", tree / "vendor/knowl.js",
          tree / "static/js/Mathbook.js"]
    (out / "css/fonts").mkdir(parents=True, exist_ok=True)
    (out / "js").mkdir(parents=True, exist_ok=True)
    (out / "images").mkdir(parents=True, exist_ok=True)
    (out / "css/ila.css").write_text("\n".join(p.read_text(encoding="utf-8") for p in css), encoding="utf-8")
    (out / "js/ila.js").write_text("\n;\n".join(p.read_text(encoding="utf-8") for p in js), encoding="utf-8")
    fonts = list((assets / "stylesheets/fonts/ionicons/fonts").iterdir()) + list((tree / "static/fonts").iterdir())
    for font in fonts:
        shutil.copyfile(font, out / "css/fonts" / font.name)
    for image in list((tree / "static/images").iterdir()) + list((tree / "static/theme-gt").iterdir()):
        shutil.copyfile(image, out / "images" / image.name)
    shutil.copyfile(tree / "static/manifest.json", out / "manifest.json")


def main() -> int:
    if len(sys.argv) != 3:
        print(__doc__, file=sys.stderr)
        return 2
    tree, out = Path(sys.argv[1]).resolve(), Path(sys.argv[2])
    out = (tree / out).resolve() if not out.is_absolute() else out
    root = Path(os.environ.get("YEOKJA_ROOT", HERE.parent)).resolve()
    commit = upstream_commit(root)
    work = tree / "_work"
    shutil.rmtree(work, ignore_errors=True)
    shutil.rmtree(out, ignore_errors=True)
    (work / "html").mkdir(parents=True)
    out.mkdir(parents=True)

    generated_xsl(tree, commit)
    sh(["xsltproc", "--xinclude", "--stringparam", "version", "default", "-o", str(work / "default.xml"),
        "src/xsl/versioning.xsl", "src/ila.xml"], tree)
    edition = (tree / "src/ko-edition.xml").read_text(encoding="utf-8")
    book = korean_book((work / "default.xml").read_text(encoding="utf-8"), edition, commit)
    (work / "ko.xml").write_text(book, encoding="utf-8")
    sh(["xmllint", "--noout", str(work / "ko.xml")], tree)

    log = sh(["xsltproc", "--xinclude", "--stringparam", "pdf.online", "", "-o", str(work / "html") + "/",
              "src/xsl/ila-ko.xsl", str(work / "ko.xml")], tree)
    warnings = [line for line in log.splitlines() if "WARNING" in line or "ERROR" in line]
    for line in warnings:
        print(line, file=sys.stderr)
    if warnings:
        raise SystemExit(f"xsltproc: {len(warnings)} warnings")

    shutil.copyfile(work / "html/index2.html", work / "html/index.html")
    status = subprocess.run([sys.executable, "-I", str(HERE / "render_latex.py"), str(work / "html"), str(out),
                             "--preamble", str(work / "html/preamble.tex"),
                             "--style-dir", str(tree / "src/latex"),
                             "--image-dir", str(tree / "src/figure-images"),
                             "--cache-dir", str(root / "build/latex-cache")]).returncode
    if status != 0:
        return status
    bundle(tree, out, root)
    print(f"build_html: site in {out}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
