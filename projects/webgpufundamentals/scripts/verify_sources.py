"""Check translated metadata, literal code, includes, and stored evaluation issues."""
from collections import Counter
from pathlib import Path
from html.parser import HTMLParser
import json
import re

root = Path(__file__).resolve().parents[1]
originals = root / 'upstream/webgpu/lessons'
translated = root / 'ko/webgpu/lessons/ko'
failures = []

def fences(text):
    blocks = []
    block = None
    marker = ''
    for line in text.splitlines(keepends=True):
        if block is None:
            match = re.match(r'^ {0,3}(`{3,}|~{3,})', line)
            if match:
                marker = match[1]
                block = [line]
        else:
            block.append(line)
            if re.fullmatch(r' {0,3}' + re.escape(marker[0]) + '{' + str(len(marker)) + r',}\s*', line):
                blocks.append(''.join(block))
                block = None
    if block is not None:
        blocks.append(''.join(block))
    return blocks

for source in sorted(originals.glob('*.md')):
    target = translated / source.name
    if not target.is_file():
        failures.append(f'missing translation: {source.name}')
        continue
    a, b = source.read_text(), target.read_text()
    header_a = a.partition('\n\n')[0].splitlines()
    header_b = b.partition('\n\n')[0].splitlines()
    if [line.partition(':')[0] for line in header_a] != [line.partition(':')[0] for line in header_b]:
        failures.append(f'metadata keys or line boundaries changed: {source.name}')
    if fences(a) != fences(b):
        failures.append(f'code fences changed: {source.name}')
    # These two includes intentionally select the Korean overlay.
    b = b.replace('webgpu/lessons/ko/toc.html', 'webgpu/lessons/toc.html')
    b = b.replace('webgpu/lessons/ko/webgpu-wgsl-function-reference.inc.html', 'webgpu/lessons/webgpu-wgsl-function-reference.inc.html')
    macros = lambda text: Counter(re.findall(r'\{\{\{.*?\}\}\}', text, re.S))
    if macros(a) != macros(b):
        failures.append(f'lesson-builder macros changed: {source.name}')
class HtmlStructure(HTMLParser):
    def __init__(self, text):
        super().__init__()
        self.structure = []
        self.feed(text)
    def handle_starttag(self, tag, attrs):
        self.structure.append(('start', tag, attrs))
    def handle_endtag(self, tag):
        self.structure.append(('end', tag))

for name in ('toc.html', 'webgpu-wgsl-function-reference.inc.html'):
    if not (translated / name).is_file():
        failures.append(f'missing translation: {name}')
        continue
    a, b = (originals / name).read_text(), (translated / name).read_text()
    if HtmlStructure(a).structure != HtmlStructure(b).structure:
        failures.append(f'HTML tags or attributes changed: {name}')
    for pattern in (r'\{\{#escapehtml\}\}.*?\{\{/escapehtml\}\}', r'<code\b[^>]*>.*?</code>'):
        if re.findall(pattern, a, re.S) != re.findall(pattern, b, re.S):
            failures.append(f'HTML code literals changed: {name}')

for state in sorted((root / 'state').rglob('*.yeokja.json')):
    for segment in json.loads(state.read_text())['segments']:
        issues = segment.get('issues', [])
        # The glossary matcher sees shader/texture/buffer words inside an
        # unchanged example URL. Such template-only segments contain no prose.
        literal_macro = re.fullmatch(r'\s*\{\{\{.*?\}\}\}\s*', segment['source'], re.S)
        if literal_macro and segment['source'] == segment.get('translation'):
            issues = [issue for issue in issues if not issue.startswith("Term '")]
        # The only term occurrence here is in an unchanged, empty anchor id.
        if (segment['source'] == '<a id="a-texture-helpers"></a> Simple Image Loading Functions'
                and segment.get('translation') == '<a id="a-texture-helpers"></a> 간단한 이미지 로딩 함수'):
            issues = [issue for issue in issues if issue != "Term 'texture' should be translated as '텍스처' but was not found in translation"]
        # This sentence explicitly describes renaming literal variable names.
        if (state.name == 'webgpu-vertex-buffers.md.yeokja.json'
                and segment['id'] == 'section:1/block:14/seg:1'
                and segment.get('translation') == '그 외에는 사용 용도를 `STORAGE`에서 `VERTEX`로 변경한 것뿐입니다 (그리고 모든 변수 이름도 "storage"에서 "vertex"로 바꾸었습니다).'):
            issues = [issue for issue in issues if issue != "Term 'vertex' should be translated as '정점' but was not found in translation"]
        if issues:
            failures.append(f'{state.name}:{segment["id"]}: {issues}')
if failures:
    raise SystemExit('\n'.join(failures))
print('Verified all lesson metadata, literal code, template macros, and stored evaluation issues.')
