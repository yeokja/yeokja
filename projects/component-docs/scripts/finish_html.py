"""Add visible attribution and translation provenance to each reading page."""
from pathlib import Path
import sys

footer = '''<footer class="translation-credit"><hr>
<p>원문 © 2023 The Bytecode Alliance Contributors ·
<a href="https://github.com/bytecodealliance/component-docs">WebAssembly Components Documentation</a> ·
<a href="https://creativecommons.org/licenses/by/4.0/">CC BY 4.0</a><br>
Yeokja 한국어 번역·편집 · Claude Code (claude-sonnet-5) ·
이 번역은 원문을 수정한 문서이며 CC BY 4.0으로 제공합니다.</p></footer>
'''

for page in Path(sys.argv[1]).rglob("*.html"):
    text = page.read_text()
    if "</main>" in text:
        page.write_text(text.replace("</main>", footer + "</main>", 1))
