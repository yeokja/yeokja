#!/usr/bin/env python3
"""Stage translated Markdeep documents and the pinned upstream renderer."""
from pathlib import Path
import json
import shutil
import subprocess
import sys

from prepare_markdeep import language_script, load_labels


def build(tree: Path, project: Path) -> None:
    for source in (project / 'upstream/books').glob('*.html'):
        translated = project / 'ko/books' / source.name
        state = project / 'state/upstream/books' / (source.name + '.yeokja.json')
        if not translated.is_file() or not state.is_file():
            raise ValueError(f'Missing translation output/state for {source.name}')
        segments = json.loads(state.read_text())['segments']
        if not segments or any(s.get('translation') is None or s.get('issues') for s in segments):
            raise ValueError(f'Incomplete or failed translation for {source.name}')
        if (tree / 'books' / source.name).read_bytes() != translated.read_bytes():
            raise ValueError(f'Assembled translation is stale for {source.name}')
    site = tree / 'site'
    if site.exists():
        shutil.rmtree(site)
    site.mkdir()
    for name in ('books', 'images', 'style'):
        shutil.copytree(tree / name, site / name, symlinks=False)
    for name in ('favicon.png', 'COPYING.txt', 'README.md', 'PRINTING.md', 'CONTRIBUTING.md', 'CHANGELOG.md'):
        shutil.copyfile(tree / name, site / name)
    subprocess.run([sys.executable, str(project / 'scripts/verify_books.py'),
                    '--source', str(project / 'upstream/books'),
                    '--target', str(site / 'books')], check=True)
    subprocess.run([sys.executable, str(project / 'scripts/prepare_index.py'),
                    'render', '--output', str(site / 'index.html')], cwd=project, check=True)
    (site / '.nojekyll').touch()
    language = language_script(load_labels(project / 'ko-ui/markdeep.md'))
    for path in (site / 'books').glob('*.html'):
        # Source documents use an implicit HTML element. Explicit language helps
        # screen readers select the Korean voice without changing Markdeep syntax.
        text = path.read_text()
        text = text.replace('<meta charset="utf-8">', '<meta charset="utf-8">\n<script>document.documentElement.lang="ko";' + language + '</script>', 1)
        path.write_text(text)
    print(f'Staged {site}')


if __name__ == '__main__':
    build(Path(sys.argv[1]).resolve(), Path(sys.argv[2]).resolve())
