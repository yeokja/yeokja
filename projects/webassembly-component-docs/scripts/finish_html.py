"""Add visible attribution and translation provenance to each reading page."""
from pathlib import Path
import sys

footer = '''<footer class="translation-credit"><hr>
<p>원문 © 2023 The Bytecode Alliance Contributors ·
<a href="https://github.com/bytecodealliance/component-docs">WebAssembly Components Documentation</a> ·
<a href="https://creativecommons.org/licenses/by/4.0/">CC BY 4.0</a><br>
Yeokja 한국어 번역·편집 · Anthropic 사의 claude-sonnet-5 모델을 활용하여
번역되었으며 학습을 모두 비허용한 상태로 작업하였습니다 ·
이 번역은 원문을 수정한 문서이며 CC BY 4.0으로 제공합니다.</p></footer>
'''

for page in Path(sys.argv[1]).rglob("*.html"):
    text = page.read_text()
    if "</main>" in text:
        page.write_text(text.replace("</main>", footer + "</main>", 1))
