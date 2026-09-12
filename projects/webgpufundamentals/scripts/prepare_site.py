"""Prepare the upstream build for the /webgpufundamentals/ Pages mount."""
from pathlib import Path
import re
import sys
from preserve_anchors import preserve_anchors
from repair_links import repair_lesson_html
from patch_editor import patch_editor

site = Path(sys.argv[1])
base = "/webgpufundamentals"
origin = "https://yeokja.moreal.dev"
# Preserve relative URLs used by examples, while moving upstream-root URLs
# in HTML, CSS, JS, and metadata under this project's Pages directory.
roots = r"(?:webgpu|3rdparty|monaco-editor|types|favicon\.ico)(?=[/\"'?#\s]|$)"
root_url = re.compile(r"(?P<quote>[\"'`(])/(?P<path>" + roots + r"[^\"'`\s)]*)")
html_url = re.compile(r'''\b(href|src|poster|action)=("|')(/[^"']*)\2''')
def mount_attribute(match):
    name, quote, url = match.groups()
    if url.startswith('//') or url == base or url.startswith(base + '/'):
        return match[0]
    return f'{name}={quote}{base}{url}{quote}'

for folder, language in ((site / 'webgpu/lessons', 'en'), (site / 'webgpu/lessons/ko', 'ko')):
    for page in folder.glob('*.html'):
        page.write_text(repair_lesson_html(page.read_text(), page.name, language))

for source in (site / 'webgpu/lessons').glob('*.html'):
    target = source.parent / 'ko' / source.name
    if target.is_file():
        target.write_text(preserve_anchors(source.read_text(), target.read_text()))

for path in site.rglob("*"):
    if path.is_file() and path.suffix in {".html", ".css", ".js", ".json", ".xml"}:
        old = path.read_text()
        public_base = origin + base if path.suffix == ".xml" else base
        new = old.replace("https://webgpufundamentals.org/", public_base + "/")
        new = root_url.sub(lambda m: m['quote'] + base + '/' + m['path'], new)
        if path.suffix == ".html":
            new = html_url.sub(mount_attribute, new)
            new = new.replace('content="' + base + '/', 'content="' + origin + base + '/')
        if new != old:
            path.write_text(new)
editor = site / 'webgpu/resources/editor.js'
if editor.is_file():
    editor.write_text(patch_editor(editor.read_text()))

for filename in ("CNAME", "package.json", "package-lock.json", "Gruntfile.js"):
    (site / filename).unlink(missing_ok=True)
(site / 'index.html').write_text('''<!doctype html>
<html lang="ko"><meta charset="utf-8">
<meta http-equiv="refresh" content="0;url=webgpu/lessons/ko/">
<title>WebGPU 기초</title><a href="webgpu/lessons/ko/">WebGPU 기초 한국어 번역</a></html>
''')
assert (site / 'webgpu/lessons/ko/index.html').is_file()
