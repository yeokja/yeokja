"""Apply yeokja's supplemental translations to the generated overlay."""
from pathlib import Path
import html
import json

root = Path(__file__).resolve().parents[1]
texts = {}
for entry in json.loads((root/'ui.json').read_text()):
    dest = root/'ko'/entry['path']
    text = texts.get(dest)
    if text is None:
        text = dest.read_text() if dest.suffix == '.mdx' else (root/'upstream'/entry['path']).read_text()
    translated = (root/'ko-ui'/entry['file']).read_text().strip()
    if not translated:
        raise ValueError(f'Empty translation: {entry}')
    source = entry['source']
    if entry['mode'] == 'shortname':
        source = 'shortname: '+source
        translated = 'shortname: '+json.dumps(translated, ensure_ascii=False)
    elif entry['mode'] == 'attribute':
        translated = html.escape(html.unescape(translated), quote=True)
    elif entry['mode'] != 'html':
        translated = translated.replace("'", '&#39;')
    if source not in text:
        raise ValueError(f'Source string missing: {entry}')
    texts[dest] = text.replace(source, translated)
for dest, text in texts.items():
    dest.parent.mkdir(parents=True, exist_ok=True)
    dest.write_text(text)
