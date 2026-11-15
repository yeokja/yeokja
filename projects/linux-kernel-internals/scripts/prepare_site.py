"""Turn upstream mkdocs.yml into the config of the Korean build (or of the
English reference build), inside an assembled tree.

Upstream's YAML carries a Python tag (!!python/name), so it is edited as text,
entry by entry, and every edit must find what it expects: a changed upstream
config fails the build instead of shipping a half-applied one.

The Korean build takes its nav labels from the translated nav list
(nav/nav.md → ko-nav/nav.md): the nav stays upstream's, line for line, and only
the labels change, in order. The untranslated contributing page gets a line
saying why it is in English.

usage: prepare_site.py <tree> <ko-nav.md>
       prepare_site.py --english <tree>
"""
import argparse
import json
from pathlib import Path
import re

from sources import parse_nav

SITE_URL = 'https://yeokja.moreal.dev/linux-kernel-internals/'
UPSTREAM = 'https://github.com/laveeshb/linux-kernel-internals'
PIN = '6ee74a0b2e5f49c75d107ac4935369f73a004546'
COPYRIGHT = (
    f'원문 <a href="https://kernel-internals.org/">Linux Kernel Internals</a>'
    f'(<a href="{UPSTREAM}">laveeshb/linux-kernel-internals</a>)의 저작자는 Laveesh Bansal이며, '
    f'문서는 <a href="{UPSTREAM}/blob/{PIN}/LICENSE-CC-BY-SA-4.0">CC BY-SA 4.0</a>, '
    f'코드 스니펫은 <a href="{UPSTREAM}/blob/{PIN}/LICENSE-GPL-2.0">GPL-2.0</a>으로 제공됩니다. '
    'Tux 로고: Larry Ewing(원본), Simon Budig(SVG).'
    '<br>이 사이트는 원문을 <a href="https://github.com/yeokja/yeokja">yeokja</a>로 한국어로 옮긴 '
    '비공식 기계 번역(변경본)이며 원저작자가 검토하거나 보증하지 않았습니다. '
    '번역문은 같은 CC BY-SA 4.0으로, 코드 스니펫은 GPL-2.0으로 제공합니다.'
    '<br>원문은 확정적인 참고 자료가 아닌 커뮤니티 학습 자료이며 오류가 있을 수 있습니다. '
    '확실하지 않을 때는 <a href="https://docs.kernel.org/">공식 커널 문서</a>와 소스 코드를 확인하십시오.'
)
SOCIAL = f'''  social:
    - icon: fontawesome/brands/github
      link: {UPSTREAM}
      name: 원문 저장소
    - icon: fontawesome/solid/language
      link: https://github.com/yeokja/yeokja
      name: 번역 저장소 (yeokja)
'''
CONTRIBUTING_NOTE = ('*이 쪽은 원문 저장소에 기여하는 절차를 설명하므로 번역하지 않고 영어 원문 그대로 둡니다. '
                     '번역의 오류는 [yeokja 저장소](https://github.com/yeokja/yeokja/issues)에 알려 주십시오.*')
# What Python-Markdown unescapes; the inline-tag serializer may escape these.
ESCAPED = re.compile(r'\\([\\`*_{}\[\]()>#+\-.!|])')
ITEM = re.compile(r'^( *)- (.+)$')


def replace_once(text, old, new):
    if text.count(old) != 1:
        raise ValueError(f'expected exactly one {old!r} in mkdocs.yml, found {text.count(old)}')
    return text.replace(old, new)


def replace_block(text, pattern, replacement):
    new, count = re.subn(pattern, lambda _: replacement, text, flags=re.M)
    if count != 1:
        raise ValueError(f'expected exactly one block matching {pattern!r}, found {count}')
    return new


def ko_labels(nav_text):
    """The translated nav list as [(depth, label)]."""
    labels = []
    for number, line in enumerate(nav_text.split('\n'), 1):
        if not line.strip():
            continue
        match = ITEM.match(line)
        if not match or len(match[1]) % 4:
            raise ValueError(f'ko nav line {number}: not a list item: {line!r}')
        label = ESCAPED.sub(r'\1', match[2]).replace('`', '').strip()
        if not label:
            raise ValueError(f'ko nav line {number}: empty label')
        labels.append((len(match[1]) // 4, label))
    return labels


def translate_nav(text, nav_text):
    """Put the translated labels on the labelled nav lines, in order."""
    labels = ko_labels(nav_text)
    entries = [entry for entry in parse_nav(text) if entry[2] is not None]
    if len(entries) != len(labels):
        raise ValueError(f'{len(entries)} nav labels in mkdocs.yml vs {len(labels)} translated labels')
    lines = text.split('\n')
    for (number, depth, _, match), (ko_depth, label) in zip(entries, labels):
        if depth != ko_depth:
            raise ValueError(f'mkdocs.yml:{number + 1}: nav depth {depth} vs translated depth {ko_depth} ({label!r})')
        lines[number] = f'{match["indent"]}- {json.dumps(label, ensure_ascii=False)}:{match["target"] or ""}'
    return '\n'.join(lines)


def prepare(text, nav_text=None):
    """nav_text None: the English reference build, which only gets the hook."""
    if not text.endswith('\n'):
        text += '\n'
    if re.search(r'^(hooks|copyright):', text, re.M):
        raise ValueError('upstream mkdocs.yml now sets hooks or copyright')
    if nav_text is None:
        return text + '\nhooks:\n  - hooks/anchors.py\n'
    text = replace_once(text, 'site_name: Linux Kernel Internals\n', 'site_name: Linux 커널 내부 구조\n')
    text = replace_block(text, r'^site_description:.*\n',
                         'site_description: Linux 커널의 내부 구조와 설계 결정, 그리고 기여의 여정을 다루는 문서 (비공식 한국어 번역)\n')
    text = replace_block(text, r'^site_url:.*\n', f'site_url: {SITE_URL}\n')
    text = replace_once(text, 'theme:\n  name: material\n', 'theme:\n  name: material\n  language: ko\n')
    text = replace_once(text, 'name: Switch to dark mode\n', 'name: 어두운 화면으로 전환\n')
    text = replace_once(text, 'name: Switch to light mode\n', 'name: 밝은 화면으로 전환\n')
    text = replace_once(text, 'plugins:\n  - search\n',
                        'plugins:\n  - search:\n      lang:\n        - ko\n        - en\n')
    text = replace_block(text, r'^  social:\n(?:    [^\n]*\n)+', SOCIAL)
    text = replace_once(text, '  - stylesheets/extra.css\n', '  - stylesheets/extra.css\n  - stylesheets/ko.css\n')
    text = translate_nav(text, nav_text)
    return text + f'\ncopyright: {json.dumps(COPYRIGHT, ensure_ascii=False)}\n\nhooks:\n  - hooks/anchors.py\n'


def note_contributing(text):
    """Say, under the title, why this page is in English."""
    lines = text.split('\n')
    if not lines[0].startswith('# '):
        raise ValueError('contributing.md does not start with its title')
    return '\n'.join([lines[0], '', CONTRIBUTING_NOTE] + lines[1:])


def rewrite(target, transform):
    """Replace `target` with transform(content). The assembled tree links its
    files into the layers, so the link is replaced, never written through."""
    text = target.read_text()
    if target.is_symlink():
        target.unlink()
    target.write_text(transform(text))


def main(argv=None):
    parser = argparse.ArgumentParser()
    parser.add_argument('--english', action='store_true')
    parser.add_argument('tree')
    parser.add_argument('nav', nargs='?')
    args = parser.parse_args(argv)
    tree = Path(args.tree)
    if args.english:
        rewrite(tree / 'mkdocs.yml', prepare)
        return
    if not args.nav:
        parser.error('the Korean build needs the translated nav list')
    nav_text = Path(args.nav).read_text()
    rewrite(tree / 'mkdocs.yml', lambda text: prepare(text, nav_text))
    rewrite(tree / 'docs' / 'contributing.md', note_contributing)


if __name__ == '__main__':
    main()
