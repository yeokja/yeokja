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


UPSTREAM = 'https://github.com/BrunoLevy/learn-fpga'


def page_path(path):
    return path.with_name('index.html') if path.name == 'README.md' else path.with_suffix('.html')


def pandoc(source, *args):
    return subprocess.run(['pandoc', *args], input=source, text=True,
                          stdout=subprocess.PIPE, check=True).stdout


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
        prefix = '/BrunoLevy/learn-fpga/'
        if parts.netloc != 'github.com' or not path.startswith(prefix):
            return url
        match = re.match(r'(?:blob|tree)/(?:master|main)/(.*)', path[len(prefix):])
        if not match:
            return url
        target = Path(match[1])
    elif not path:
        return url
    else:
        target = Path(posixpath.normpath(str(current.parent / path)))
        # FemtoRV/README.md uses a repository-relative image URL.
        if not (tree / target).exists() and not Path(path).is_absolute() and '..' not in Path(path).parts and (tree / path).exists():
            target = Path(path)
    if '..' in target.parts or target.is_absolute():
        return url
    if target in documents:
        target = page_path(target)
    elif (tree / target).is_dir():
        target = target / 'index.html'
    elif not (tree / target).is_file():
        # Upstream occasionally links files that are no longer in the checkout.
        return UPSTREAM + '/blob/master/' + quote(str(target), safe='/') + ('#' + parts.fragment if parts.fragment else '')
    relative = posixpath.relpath(str(target), str(current.parent))
    return urlunsplit(('', '', quote(relative, safe='/'), parts.query, parts.fragment))


def main():
    tree = Path.cwd()
    project = Path(os.environ['YEOKJA_ROOT'])
    upstream = project / 'upstream'
    documents = {p.relative_to(upstream) for p in upstream.rglob('*.md')}
    destination = tree / 'site'
    if destination.exists():
        shutil.rmtree(destination)
    shutil.copytree(tree, destination, ignore=shutil.ignore_patterns('.git', 'site'))
    for document in documents:
        if document.stem.upper() in {'LICENSE', 'COPYING'}:
            shutil.copyfile(upstream / document, destination / document.with_suffix('.original.md'))
    style = '''body{max-width:58rem;margin:2rem auto;padding:0 1.2rem;font:18px/1.8 system-ui,sans-serif;color:#242424}a{color:#155e9b}img,video{max-width:100%;height:auto}pre{overflow:auto;background:#f4f4f4;padding:1rem;line-height:1.5}code{font-size:.9em}table{display:block;overflow:auto;border-collapse:collapse}td,th{border:1px solid #ddd;padding:.4rem .7rem}nav,footer{font-size:.85rem;color:#555;border-bottom:1px solid #ddd;padding-bottom:1rem}h1,h2,h3{line-height:1.35;scroll-margin-top:1rem}'''
    for document in sorted(documents):
        original = json.loads(pandoc((upstream / document).read_text(), '-f', 'gfm', '-t', 'json'))
        translated = json.loads(pandoc((tree / document).read_text(), '-f', 'gfm', '-t', 'json'))
        old_headers, new_headers = list(headers(original)), list(headers(translated))
        if len(old_headers) != len(new_headers):
            raise ValueError(f'Heading count changed: {document}')
        for old, new in zip(old_headers, new_headers):
            new[0] = old[0]
        body = pandoc(json.dumps(translated), '-f', 'json', '-t', 'html5', '--mathml')
        body = re.sub(r'\b(href|src)="([^"]*)"',
                      lambda m: m[1] + '="' + html.escape(rewrite_url(m[2], document, documents, tree), quote=True) + '"', body)
        home = posixpath.relpath('index.html', str(document.parent))
        source = UPSTREAM + '/blob/master/' + quote(str(document), safe='/')
        license_url = posixpath.relpath('LICENSE', str(document.parent))
        title_match = re.search(r'<h[12][^>]*>(.*?)</h[12]>', body, re.S)
        title = re.sub('<[^>]*>', '', title_match[1]) if title_match else html.escape(str(document))
        output = f'''<!doctype html>
<html lang="ko"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1">
<title>{title} · learn-fpga 한국어</title><style>{style}</style></head><body>
<nav><a href="{home}">learn-fpga 한국어</a> · <a href="{source}">원문</a> · yeokja 기계 번역</nav>
<main>{body}</main><footer>Bruno Levy 및 기여자들의 learn-fpga 비공식 한국어 번역. <a href="{license_url}">원문 라이선스</a>. 코드와 기술 용어는 원문을 함께 확인하세요.</footer>
</body></html>'''
        (destination / page_path(document)).write_text(output)
    # Code directories are also reachable from the tutorials.
    for directory in [destination, *sorted(p for p in destination.rglob('*') if p.is_dir())]:
        if (directory / 'index.html').exists():
            continue
        entries = ''.join(f'<li><a href="{quote(p.name) + ("/" if p.is_dir() else "")}">{html.escape(p.name)}</a></li>'
                          for p in sorted(directory.iterdir()))
        (directory / 'index.html').write_text(f'<!doctype html><html lang="ko"><meta charset="utf-8"><title>예제 파일</title><h1>예제 파일</h1><ul>{entries}</ul></html>')
    (destination / '.nojekyll').touch()
    print(f'Built {len(documents)} Markdown pages in {destination}')


if __name__ == '__main__':
    main()
