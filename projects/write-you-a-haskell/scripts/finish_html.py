"""Add visible attribution, license, and modification notice to each reading page.

CC BY-SA 4.0 §3(a) asks an adapted work to keep the creator's name, the license
notice and a statement that the work was modified; the footer carries all three
on every chapter, with the license texts copied next to the site root.
"""
from pathlib import Path
import sys

FOOTER = '''<footer class="translation-credit"><hr>
<p>원문 © Stephen Diehl ·
<a href="https://github.com/sdiehl/write-you-a-haskell">Write You a Haskell</a> ·
산문 <a href="{root}LICENSE-PROSE">CC BY-SA 4.0</a> · 코드 <a href="{root}LICENSE">MIT</a><br>
<a href="https://github.com/yeokja/yeokja">yeokja</a> 한국어 번역 ·
Anthropic 사의 <code>claude-opus-5-5</code> 모델을 활용하여 번역되었으며
학습을 모두 비허용한 상태로 작업하였습니다 ·
이 문서는 원문을 수정한 비공식 번역본이며 산문은 같은 CC BY-SA 4.0으로 제공합니다.</p></footer>
'''


def add_footer(text, depth):
    if '</main>' not in text:
        return text
    return text.replace('</main>', FOOTER.format(root='../' * depth) + '</main>', 1)


def finish(root):
    root = Path(root)
    for page in root.rglob('*.html'):
        depth = len(page.relative_to(root).parts) - 1
        page.write_text(add_footer(page.read_text(), depth))


if __name__ == '__main__':
    finish(sys.argv[1])
