"""Make mdBook redirect pages work below a path prefix.

The book is published under a subpath, not at the site root, so a redirect
target such as "/release/backporting.html" would leave the book. Rewrite
root-absolute targets in the generated redirect stubs relative to each stub.
"""
import argparse
import posixpath
import re
from pathlib import Path

MARKER = '<title>Redirecting...</title>'
URL = re.compile(r'(URL=|href="|redirectUrl = ")(/[^/"][^"]*)')


def relativize(text, page):
    """Return the stub `text` of the page at site-relative `page` rewritten."""
    if MARKER not in text:
        return text
    start = posixpath.dirname(page) or '.'

    def replace(match):
        target = match.group(2)[1:]
        path, sep, fragment = target.partition('#')
        return match.group(1) + posixpath.relpath(path, start) + sep + fragment

    return URL.sub(replace, text)


def relativize_site(root):
    for file in root.rglob('*.html'):
        text = file.read_text()
        result = relativize(text, file.relative_to(root).as_posix())
        if result != text:
            file.write_text(result)


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('site', type=Path)
    relativize_site(parser.parse_args().site)
