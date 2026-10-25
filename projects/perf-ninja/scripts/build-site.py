"""Render the assembled Markdown tree, keeping upstream URLs and heading IDs."""
import html
import json
import os
from pathlib import Path
import posixpath
import re
import shutil
import subprocess
from urllib.parse import quote, unquote, urlsplit, urlunsplit


UPSTREAM = 'https://github.com/dendibakh/perf-ninja'
REPOSITORY = '/dendibakh/perf-ninja/'


def page_path(path):
    return path.with_name('index.html') if path.name == 'README.md' else path.with_suffix('.html')


def pandoc(source, *args):
    return subprocess.run(['pandoc', *args], input=source, text=True,
                          stdout=subprocess.PIPE, check=True).stdout


def markdown_ast(path):
    return json.loads(pandoc(path.read_text(), '-f', 'gfm', '-t', 'json'))


def render(ast):
    return pandoc(json.dumps(ast), '-f', 'json', '-t', 'html5')


def headers(node):
    if isinstance(node, dict):
        if node.get('t') == 'Header':
            yield node['c'][1]
        for value in node.values():
            yield from headers(value)
    elif isinstance(node, list):
        for item in node:
            yield from headers(item)


def rewrite_url(url, current, documents, tree):
    parts = urlsplit(html.unescape(url))
    path = unquote(parts.path)
    if parts.scheme or parts.netloc:
        if parts.netloc != 'github.com' or not path.startswith(REPOSITORY):
            return url
        # Only `main` is the pinned checkout; `golden` holds the reference
        # solutions, which are not in the tree, so those links stay upstream.
        match = re.match(r'(?:blob|tree)/main/(.*)', path[len(REPOSITORY):])
        if not match:
            return url
        target = Path(match[1])
    elif not path:
        return url
    elif path.startswith('/'):
        # GitHub resolves a root-relative URL against the repository root; on
        # Pages the site lives under perf-ninja/, so make it document-relative.
        target = Path(path.lstrip('/'))
    else:
        target = Path(posixpath.normpath(str(current.parent / path)))
    if '..' in target.parts or target.is_absolute():
        return url
    if target in documents:
        target = page_path(target)
    elif (tree / target).is_dir():
        target = target / 'index.html'
    elif not (tree / target).is_file():
        return UPSTREAM + '/blob/main/' + quote(str(target), safe='/') + ('#' + parts.fragment if parts.fragment else '')
    relative = posixpath.relpath(str(target), str(current.parent))
    return urlunsplit(('', '', quote(relative, safe='/'), parts.query, parts.fragment))


STYLE = '''body{max-width:58rem;margin:2rem auto;padding:0 1.2rem;font:18px/1.8 system-ui,sans-serif;color:#242424;background:#fff;word-break:keep-all;overflow-wrap:anywhere}a{color:#155e9b}img,video{max-width:100%;height:auto}pre{overflow:auto;background:#f4f4f4;padding:1rem;line-height:1.5;word-break:normal;overflow-wrap:normal}code{font-size:.9em}table{display:block;overflow:auto;border-collapse:collapse}td,th{border:1px solid #ddd;padding:.4rem .7rem}nav,footer{font-size:.85rem;color:#555}nav{border-bottom:1px solid #ddd;padding-bottom:1rem}footer{border-top:1px solid #ddd;margin-top:3rem;padding-top:1rem}h1,h2,h3{line-height:1.35;scroll-margin-top:1rem}details{border:1px solid #ddd;border-radius:6px;padding:.3rem 1rem;margin:.6rem 0}details[open]{padding-bottom:.6rem}summary{cursor:pointer}
@media (prefers-color-scheme:dark){body{color:#ddd;background:#181a1b}a{color:#7cb7ef}pre{background:#24272a}nav,footer{color:#aaa}nav,footer,details,td,th{border-color:#444}}'''


def page(document, body):
    home = posixpath.relpath('index.html', str(document.parent))
    source = UPSTREAM + '/blob/main/' + quote(str(document), safe='/')
    # Lab READMEs open with a video thumbnail and no title; name them by directory.
    title_match = re.search(r'<h1[^>]*>(.*?)</h1>', body, re.S)
    fallback = document.parent.name if document.name == 'README.md' else document.stem
    title = re.sub('<[^>]*>', '', title_match[1]).strip() if title_match else html.escape(fallback)
    return f'''<!doctype html>
<html lang="ko"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1">
<title>{title} · Performance Ninja 한국어</title><style>{STYLE}</style></head><body>
<nav><a href="{home}">Performance Ninja 한국어</a> · <a href="{source}">원문</a> · yeokja 기계 번역</nav>
<main>{body}</main>
<footer>Denis Bakhvalov와 실습 과제 저자들의 <a href="{UPSTREAM}">Performance Ninja</a>를 한국어로 옮긴 비공식 번역입니다. 원문과 이 번역은 <a href="https://creativecommons.org/licenses/by/4.0/deed.ko">CC BY 4.0</a>이며, 번역 과정에서 원문을 수정했습니다. 코드와 주석, 벤치마크 출력, 영상은 원문 그대로이며 제3자 코드는 각 디렉터리의 라이선스를 따릅니다.</footer>
</body></html>'''


def main():
    tree = Path.cwd()
    project = Path(os.environ['YEOKJA_ROOT'])
    upstream = project / 'upstream'
    documents = {p.relative_to(upstream) for p in upstream.rglob('*.md')}
    destination = tree / 'site'
    if destination.exists():
        shutil.rmtree(destination)
    shutil.copytree(tree, destination, ignore=shutil.ignore_patterns('.git', 'site'))
    for document in sorted(documents):
        original = markdown_ast(upstream / document)
        translated = markdown_ast(tree / document)
        old_headers, new_headers = list(headers(original)), list(headers(translated))
        if len(old_headers) != len(new_headers):
            raise ValueError(f'Heading count changed: {document}')
        for old, new in zip(old_headers, new_headers):
            new[0] = old[0]
        body = render(translated)
        body = re.sub(r'\b(href|src)="([^"]*)"',
                      lambda m: m[1] + '="' + html.escape(rewrite_url(m[2], document, documents, tree), quote=True) + '"', body)
        (destination / page_path(document)).write_text(page(document, body))
    # Lab directories hold code that the documents link to.
    for directory in [destination, *sorted(p for p in destination.rglob('*') if p.is_dir())]:
        if (directory / 'index.html').exists():
            continue
        entries = ''.join(f'<li><a href="{quote(p.name) + ("/index.html" if p.is_dir() else "")}">{html.escape(p.name)}{"/" if p.is_dir() else ""}</a></li>'
                          for p in sorted(directory.iterdir()))
        name = html.escape(str(directory.relative_to(destination)))
        (directory / 'index.html').write_text(f'<!doctype html><html lang="ko"><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>{name} · 예제 파일</title><style>{STYLE}</style><h1>예제 파일: {name}</h1><ul>{entries}</ul></html>')
    (destination / '.nojekyll').touch()
    print(f'Built {len(documents)} Markdown pages in {destination}')


if __name__ == '__main__':
    main()
