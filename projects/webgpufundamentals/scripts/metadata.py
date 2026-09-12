"""Translate lesson-builder metadata as values, keeping its keys and newlines."""
from pathlib import Path
import re
import json
import sys

root = Path(__file__).resolve().parents[1]
separator = '\n\n---\n\n'
mode = sys.argv[1]
if mode not in {'extract', 'apply'}:
    raise SystemExit('usage: metadata.py extract|apply')
for source in sorted((root / 'upstream/webgpu/lessons').glob('*.md')):
    header, source_boundary, _body = source.read_text().partition('\n\n')
    fields = []
    for line in header.splitlines():
        match = re.fullmatch(r'([A-Za-z0-9_-]+): (.*)', line)
        if not match:
            raise ValueError(f'{source}: unexpected metadata line: {line!r}')
        fields.append(match.groups())
    visible = [(key, value) for key, value in fields if key.lower() in {'title', 'description', 'toc'}]
    if mode == 'extract':
        dest = root / 'metadata' / source.name
        dest.parent.mkdir(exist_ok=True)
        dest.write_text(separator.join(value for _, value in visible) + '\n')
    else:
        values = (root / 'ko-metadata' / source.name).read_text().strip().split(separator)
        if len(values) != len(visible):
            raise ValueError(f'{source}: metadata field count changed')
        translated = dict(zip((key for key, _ in visible), values))
        target = root / 'ko/webgpu/lessons/ko' / source.name
        _, boundary, body = target.read_text().partition('\n\n')
        if not boundary and source_boundary:
            raise ValueError(f'{target}: missing header boundary')
        lines = [f'{key}: {" ".join(translated.get(key, value).splitlines())}' for key, value in fields]
        body = body.replace('include "webgpu/lessons/webgpu-wgsl-function-reference.inc.html"', 'include "webgpu/lessons/ko/webgpu-wgsl-function-reference.inc.html"')
        body = body.replace('include "webgpu/lessons/toc.html"', 'include "webgpu/lessons/ko/toc.html"')
        target.write_text('\n'.join(lines) + '\n\n' + body)

# The upstream Korean locale predates this browser-feature warning.
warning_file = '_need-new-webgpu.md'
if mode == 'extract':
    langinfo = (root / 'upstream/webgpu/lessons/langinfo.hanson').read_text()
    match = re.search(r"needNewWebGPU: '([^']*)'", langinfo)
    if not match:
        raise ValueError('missing English WebGPU feature warning')
    (root / 'metadata' / warning_file).write_text(match[1] + '\n')
else:
    warning = (root / 'ko-metadata' / warning_file).read_text().strip()
    locale = (root / 'upstream/webgpu/lessons/ko/langinfo.hanson').read_text()
    locale = locale.replace('{', '{\n  needNewWebGPU: ' + json.dumps(warning, ensure_ascii=False) + ',', 1)
    (root / 'ko/webgpu/lessons/ko/langinfo.hanson').write_text(locale)

# A missing closing code fence in upstream hides this paragraph from Markdown.
# Translate it separately; prepare_build.py repairs the fence in its copy.
camera = root / 'upstream/webgpu/lessons/webgpu-cameras.md'
if mode == 'extract' and camera.is_file():
    match = re.search(r'(<a id="a-aim-fs"></a> .*?)(?=\n\n```js\n-  const numFs)', camera.read_text(), re.S)
    if not match:
        raise ValueError('camera aim paragraph changed; review the fence repair')
    (root / 'metadata/_camera-aim-fs.md').write_text(match[1] + '\n')
