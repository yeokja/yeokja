"""Add the attribution, license and modification notice to each Korean page.

The license page of the original site requires the credit "Stephen Diehl,
stephendiehl.com", a link to CC BY-SA 4.0, a note that the work was changed,
and that no organisation is named or implied; its pre-approved byline is
"Stephen Diehl is a software engineer and writer from London."
"""
from pathlib import Path
import sys

FOOTER = '''<footer class="translation-credit"><hr>
<p>원문 © Stephen Diehl, <a href="https://www.stephendiehl.com">stephendiehl.com</a> ·
글 <a href="https://creativecommons.org/licenses/by-sa/4.0/">CC BY-SA 4.0</a>, 코드 MIT ·
Stephen Diehl은 런던의 소프트웨어 엔지니어이자 작가입니다(저자가 정한 소개 문구).
이 글은 저자가 개인 자격으로 쓴 것이며 어떤 조직과의 연관도 밝히거나 암시해서는 안 됩니다
(<a href="{root}license.html">라이선스와 고지</a>).<br>
이 페이지는 원문을 한국어로 옮기며 변경한 비공식 번역이며 같은 CC BY-SA 4.0으로 제공합니다 ·
<a href="https://github.com/yeokja/yeokja">yeokja</a> 한국어 번역 ·
Anthropic 사의 <code>claude-opus-5-5</code> 모델을 활용하여 번역되었으며
학습을 모두 비허용한 상태로 작업하였습니다.</p></footer>
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
