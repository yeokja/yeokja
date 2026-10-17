#!/usr/bin/env python3
"""Reconstruct the English source of stephendiehl.com as Markdown.

The blog has no public source repository, so the English manuscript under
``source/`` is rebuilt from the published HTML: ``fetch`` downloads the post
index and every listed post once into a cache, and ``convert`` turns the cached
pages into ``source/posts/<slug>.md`` with pandoc. ``source/manifest.json``
records what was fetched, with a content hash per page, so a later run shows
which posts changed upstream.

Usage (from the project directory)::

    python3 scripts/fetch_site.py fetch              # download into build/fetch/
    python3 scripts/fetch_site.py convert            # build/fetch/ -> source/
    python3 scripts/fetch_site.py all                # both

See RECONSTRUCTION.md for the conversion rules this script implements.
"""

from __future__ import annotations

import argparse
import datetime as dt
import hashlib
import html
import json
import re
import subprocess
import sys
import time
import urllib.error
import urllib.parse
import urllib.request
from pathlib import Path

SITE = "https://www.stephendiehl.com"
USER_AGENT = "yeokja-stephendiehl-fetch/1.0 (Korean translation project; dev.moreal@gmail.com)"
PROJECT = Path(__file__).resolve().parents[1]
DEFAULT_CACHE = PROJECT / "build" / "fetch"
DEFAULT_SOURCE = PROJECT / "source"
DELAY_SECONDS = 1.0

# Pages other than posts that the book carries. The license page is kept in
# English (it is not a translation source) and the home blurb becomes index.md.
EXTRA_PAGES = {"index": "/", "license": "/license/"}

PANDOC_FROM = "html+tex_math_dollars+tex_math_single_backslash"
PANDOC_TO = "commonmark_x-attributes-smart-fenced_divs-bracketed_spans"

MONTHS = {m: i for i, m in enumerate(
    ["January", "February", "March", "April", "May", "June", "July",
     "August", "September", "October", "November", "December"], 1)}


# --- fetching -----------------------------------------------------------------

def cache_name(path: str) -> str:
    """Cache file name for a site path such as ``/posts/monads/``."""
    stem = re.sub(r"[^A-Za-z0-9._-]+", "_", path.strip("/")) or "index"
    return stem + ".html"


def download(url: str, retries: int = 3) -> bytes:
    last: Exception | None = None
    for attempt in range(retries):
        try:
            request = urllib.request.Request(url, headers={"User-Agent": USER_AGENT})
            with urllib.request.urlopen(request, timeout=60) as response:
                return response.read()
        except (urllib.error.URLError, TimeoutError) as error:  # pragma: no cover - network
            last = error
            time.sleep(DELAY_SECONDS * (attempt + 2))
    raise RuntimeError(f"failed to fetch {url}: {last}")


def fetch_page(path: str, cache: Path, refresh: bool = False) -> Path:
    """Download one site path into the cache (once) and return the file."""
    target = cache / cache_name(path)
    if target.exists() and not refresh:
        return target
    cache.mkdir(parents=True, exist_ok=True)
    target.write_bytes(download(SITE + path))
    time.sleep(DELAY_SECONDS)
    return target


def parse_sitemap(xml: str) -> dict[str, str | None]:
    """Map each site path in the sitemap to its ``lastmod`` (or None)."""
    out: dict[str, str | None] = {}
    for block in re.findall(r"<url>(.*?)</url>", xml, re.S):
        loc = re.search(r"<loc>\s*(.*?)\s*</loc>", block)
        if not loc:
            continue
        path = loc.group(1).strip().replace(SITE, "") or "/"
        lastmod = re.search(r"<lastmod>\s*(.*?)\s*</lastmod>", block)
        out[path] = lastmod.group(1).strip() if lastmod else None
    return out


def parse_post_index(page: str) -> list[dict[str, str]]:
    """Posts in the order ``/posts/`` lists them (newest first)."""
    posts = []
    for match in re.finditer(r'<a href="(/posts/[^"]+/)">(.*?)</a>\s*<time>(.*?)</time>', page, re.S):
        posts.append({
            "path": match.group(1),
            "slug": match.group(1).strip("/").split("/")[-1],
            "title": html.unescape(re.sub(r"\s+", " ", match.group(2)).strip()),
            "date": iso_date(match.group(3).strip()),
        })
    return posts


def iso_date(text: str) -> str:
    """``July 7, 2013`` -> ``2013-07-07``; ISO input passes through."""
    if re.fullmatch(r"\d{4}-\d{2}-\d{2}", text):
        return text
    match = re.fullmatch(r"([A-Z][a-z]+) (\d{1,2}), (\d{4})", text)
    if not match:
        raise ValueError(f"unrecognised date: {text!r}")
    return f"{int(match.group(3)):04d}-{MONTHS[match.group(1)]:02d}-{int(match.group(2)):02d}"


# --- HTML preparation ---------------------------------------------------------

def main_content(page: str) -> str:
    match = re.search(r"<main\b[^>]*>(.*?)</main>", page, re.S)
    if not match:
        raise ValueError("page has no <main> element")
    return match.group(1)


def post_meta(main: str) -> tuple[str | None, list[str], str]:
    """Split the post-meta block off: (date, tags, remaining HTML)."""
    match = re.search(r'<div class="post-meta"[^>]*>(.*?)</div>\s*</div>\s*', main, re.S)
    if not match:
        return None, [], main
    block = match.group(1)
    date = re.search(r'<span class="post-date">(.*?)</span>', block, re.S)
    tags = [html.unescape(re.sub(r"\s+", " ", t).strip())
            for t in re.findall(r'<a href="/tags/[^"]*"\s*>(.*?)</a', block, re.S)]
    remaining = main[:match.start()] + main[match.end():]
    return (iso_date(date.group(1).strip()) if date else None), tags, remaining


def mdbook_id(heading: str) -> str:
    """The id mdBook gives a heading: lowercase, spaces to ``-``, only
    alphanumerics, ``-`` and ``_`` kept (``Fréchet`` keeps its accent)."""
    text = html.unescape(re.sub(r"<[^>]+>", "", heading)).strip()
    out = []
    for ch in text:
        if ch.isalnum() or ch in "_-":
            out.append(ch.lower())
        elif ch.isspace():
            out.append("-")
    return "".join(out)


def heading_ids(main: str) -> dict[str, str]:
    """Map each original heading id (markdown-it-anchor, percent-encoded) to
    the id mdBook will give the same heading."""
    ids = {}
    for match in re.finditer(r'<h[1-6]\b[^>]*\bid="([^"]*)"[^>]*>(.*?)</h[1-6]>', main, re.S):
        original = urllib.parse.unquote(html.unescape(match.group(1)))
        ids.setdefault(original, mdbook_id(match.group(2)))
    return ids


def rewrite_links(fragment: str, slugs: set[str], anchors: dict[str, str] | None = None) -> str:
    """Make site links usable from ``source/posts/``.

    Links to posts in scope become relative Markdown links (``slug.md``), other
    site-relative links and image sources become absolute URLs on the original
    site. A fragment-only link to one of the page's own headings is renamed to
    mdBook's id for that heading; other fragment links are untouched.
    """
    anchors = anchors or {}

    def href(match: re.Match) -> str:
        attr, value = match.group(1), match.group(2)
        post = re.fullmatch(r"/posts/([^/#?]+)/?(#.*)?", value)
        if attr == "href" and post and post.group(1) in slugs:
            return f'{attr}="{post.group(1)}.md{post.group(2) or ""}"'
        if value.startswith("/"):
            return f'{attr}="{SITE}{value}"'
        if attr == "href" and value.startswith("#"):
            target = urllib.parse.unquote(html.unescape(value[1:]))
            if target in anchors:
                return f'{attr}="#{anchors[target]}"'
        return match.group(0)

    return re.sub(r'\b(href|src)="([^"]*)"', href, fragment)


def ensure_h1(main: str, title: str) -> str:
    """Give every post a level-1 title heading.

    Posts from 2016 and earlier open with an ``<h3>`` carrying the title
    instead of an ``<h1>``; that heading is promoted. A post with no title
    heading at all gets one from the post index.
    """
    if re.search(r"<h1\b", main):
        return main
    first = re.search(r"<(h[2-6])\b[^>]*>(.*?)</\1>", main, re.S)
    if first and html.unescape(re.sub(r"<[^>]+>|\s+", " ", first.group(2))).strip() == title:
        return main[:first.start()] + f"<h1>{first.group(2)}</h1>" + main[first.end():]
    return f"<h1>{html.escape(title)}</h1>\n" + main


def prepare_html(main: str, slugs: set[str]) -> str:
    """Normalise the article HTML before pandoc sees it.

    * ``<br>`` followed by a newline is markdown-it's rendering of a soft line
      break in the author's paragraphs (``breaks: true``), not a hard break, so
      it goes back to a newline.
    * Heading ``id``/``tabindex`` attributes are dropped: mdBook assigns its own
      ids and the build restores the English ones as aliases.
    * ``loading``/``decoding`` image hints are presentation only.
    * A code block without a language is marked ``text`` so pandoc writes a
      fence rather than an indented block, which every tool downstream
      (the parser's math scanner, mdbook-katex) recognises as code.
    """
    anchors = heading_ids(main)
    out = main.replace("<br>\n", "\n").replace("<br />\n", "\n")
    out = re.sub(r'<(h[1-6])\b[^>]*>', r'<\1>', out)
    out = re.sub(r'\s+(?:loading|decoding)="[^"]*"', "", out)
    out = out.replace("<pre><code>", '<pre><code class="language-text">')
    return rewrite_links(out, slugs, anchors)


HTML_BLOCK_START = re.compile(r"<(?:table|div|colgroup|tr|td|th|p|ul|ol|dl|details|figure|section|blockquote)\b", re.I)


def escape_dollars_in_html_blocks(markdown: str) -> str:
    """Write ``$`` as ``&#36;`` inside raw HTML blocks.

    pandoc keeps a table whose cells hold lists or line breaks as raw HTML.
    A dollar price in such a cell would pair with the next cell's dollar and
    read as math to mdbook-katex and to the parser's math scanner; the original
    site renders math per text node, so no such pairing exists there.
    """
    out = []
    in_fence = in_block = False
    for line in markdown.splitlines():
        if re.match(r"^\s*(```|~~~)", line):
            in_fence = not in_fence
            in_block = False
        elif not in_fence:
            if not line.strip():
                in_block = False
            elif not in_block and HTML_BLOCK_START.match(line):
                in_block = True
        out.append(line.replace("$", "&#36;") if in_block else line)
    return "\n".join(out) + ("\n" if markdown.endswith("\n") else "")


# --- pandoc -------------------------------------------------------------------

def pandoc_version() -> str:
    first = subprocess.run(["pandoc", "--version"], check=True, capture_output=True, text=True).stdout.splitlines()[0]
    return first.split()[-1]


def to_markdown(fragment: str) -> str:
    result = subprocess.run(
        ["pandoc", "-f", PANDOC_FROM, "-t", PANDOC_TO, "--wrap=none"],
        input=fragment, check=True, capture_output=True, text=True,
    )
    return escape_dollars_in_html_blocks(tidy_markdown(result.stdout))


def tidy_markdown(text: str) -> str:
    """Normalise pandoc output: single trailing newline, no trailing spaces."""
    lines = [line.rstrip() for line in text.splitlines()]
    return "\n".join(lines).strip("\n") + "\n"


def front_matter(fields: dict[str, object]) -> str:
    lines = ["---"]
    for key, value in fields.items():
        if value is None:
            continue
        if isinstance(value, list):
            items = ", ".join(json.dumps(v, ensure_ascii=False) for v in value)
            lines.append(f"{key}: [{items}]")
        else:
            lines.append(f"{key}: {json.dumps(value, ensure_ascii=False)}")
    lines.append("---")
    return "\n".join(lines) + "\n\n"


def sha256(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


# --- commands -----------------------------------------------------------------

def cmd_fetch(cache: Path, refresh: bool) -> None:
    index = fetch_page("/posts/", cache, refresh).read_text()
    posts = parse_post_index(index)
    fetch_page("/sitemap.xml", cache, refresh)
    for path in EXTRA_PAGES.values():
        fetch_page(path, cache, refresh)
    for i, post in enumerate(posts, 1):
        fetch_page(post["path"], cache, refresh)
        print(f"[{i}/{len(posts)}] {post['path']}", file=sys.stderr)
    print(f"fetched {len(posts)} posts into {cache}", file=sys.stderr)


def convert_post(post: dict[str, str], raw: bytes, slugs: set[str], lastmod: str | None) -> tuple[str, dict[str, object]]:
    page = raw.decode("utf-8")
    date, tags, body = post_meta(main_content(page))
    body = prepare_html(ensure_h1(body, post["title"]), slugs)
    markdown = to_markdown(body)
    heading = re.match(r"# (.+)\n", markdown)
    title = html.unescape(heading.group(1)).strip() if heading else post["title"]
    meta = {
        "slug": post["slug"],
        "title": title,
        "date": date or post["date"],
        "tags": tags,
        "url": SITE + post["path"],
        "lastmod": lastmod,
        "html_sha256": sha256(raw),
    }
    text = front_matter({"date": meta["date"], "tags": tags, "url": meta["url"]}) + markdown
    meta["markdown_sha256"] = sha256(text.encode("utf-8"))
    return text, meta


def convert_extra(name: str, raw: bytes, slugs: set[str]) -> str:
    body = prepare_html(main_content(raw.decode("utf-8")), slugs)
    markdown = to_markdown(body)
    if name == "index":
        markdown = "# Stephen Diehl\n\n" + markdown
    return markdown


def cmd_convert(cache: Path, source: Path) -> None:
    index = (cache / cache_name("/posts/")).read_text()
    posts = parse_post_index(index)
    sitemap = parse_sitemap((cache / cache_name("/sitemap.xml")).read_text())
    slugs = {p["slug"] for p in posts}
    (source / "posts").mkdir(parents=True, exist_ok=True)
    for stale in (source / "posts").glob("*.md"):
        if stale.stem not in slugs:
            stale.unlink()
    manifest_posts = []
    for post in posts:
        raw = (cache / cache_name(post["path"])).read_bytes()
        text, meta = convert_post(post, raw, slugs, sitemap.get(post["path"]))
        (source / "posts" / f"{post['slug']}.md").write_text(text)
        manifest_posts.append(meta)
    for name, path in EXTRA_PAGES.items():
        raw = (cache / cache_name(path)).read_bytes()
        (source / f"{name}.md").write_text(convert_extra(name, raw, slugs))
    unlisted = sorted(p for p in sitemap if p.startswith("/posts/") and p != "/posts/"
                      and p.strip("/").split("/")[-1] not in slugs)
    manifest = {
        "site": SITE,
        "fetched_at": dt.datetime.fromtimestamp(
            (cache / cache_name("/posts/")).stat().st_mtime, dt.timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
        "pandoc": pandoc_version(),
        "pandoc_from": PANDOC_FROM,
        "pandoc_to": PANDOC_TO,
        "posts": manifest_posts,
        "extra_pages": {name: SITE + path for name, path in EXTRA_PAGES.items()},
        "unlisted_posts": [SITE + p for p in unlisted],
    }
    (source / "manifest.json").write_text(json.dumps(manifest, indent=2, ensure_ascii=False) + "\n")
    print(f"wrote {len(manifest_posts)} posts, {len(EXTRA_PAGES)} extra pages, "
          f"{len(unlisted)} unlisted posts skipped", file=sys.stderr)


def main(argv: list[str] | None = None) -> None:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("command", choices=["fetch", "convert", "all"])
    parser.add_argument("--cache", type=Path, default=DEFAULT_CACHE)
    parser.add_argument("--source", type=Path, default=DEFAULT_SOURCE)
    parser.add_argument("--refresh", action="store_true", help="re-download pages already in the cache")
    args = parser.parse_args(argv)
    if args.command in ("fetch", "all"):
        cmd_fetch(args.cache, args.refresh)
    if args.command in ("convert", "all"):
        cmd_convert(args.cache, args.source)


if __name__ == "__main__":
    main()
