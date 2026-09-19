"""Turn upstream mkdocs.yml into the config the translated (or English
reference) build uses, inside an assembled tree.

Upstream's YAML carries Python tags (!!python/name, !ENV), so it is edited as
text, entry by entry, and every edit must find what it expects: a changed
upstream config fails the build instead of shipping a half-applied one.

The Korean build also explains the upstream tag names "Translated" and
"Original", which stay in English as page metadata, on the tag index page.

usage: prepare_site.py [--english] [--site-url URL] <tree>
"""
import argparse
from pathlib import Path
import re

PLUGIN_BLOCKS = ['toggle-sidebar', 'mkdocs-simple-hooks', 'git-revision-date-localized',
                 'git-authors', 'git-committers', 'rss']
REQUIRED_PLUGINS = ['search', 'tags', 'literate-nav', 'macros']
COPYRIGHT_KO = ('copyright: 원문은 <a href="https://github.com/cp-algorithms/cp-algorithms/blob/main/LICENSE">'
                'CC BY-SA 4.0</a>으로 제공되며 © 2014-2025 '
                '<a href="https://github.com/cp-algorithms/cp-algorithms/graphs/contributors">cp-algorithms contributors</a>입니다.'
                '<br/>이 사이트는 원문을 한국어로 옮긴 비공식 번역(수정본)이며 같은 CC BY-SA 4.0으로 제공합니다.')
TAGS_MARKER = '<!-- material/tags -->'
TAGS_NOTE = ('태그 "Translated"는 러시아어 사이트 [e-maxx](https://e-maxx.ru/algo/)의 글을 옮긴 문서, '
             '"Original"은 cp-algorithms에서 새로 쓴 문서라는 원문 표시로, 원문 그대로 둡니다.\n\n')


def remove_block(text, name):
    """Remove a `  - name` list entry together with its indented options."""
    pattern = re.compile(r'^  - ' + re.escape(name) + r'(:[^\n]*)?\n(?:    [^\n]*\n|      [^\n]*\n)*', re.M)
    new, count = pattern.subn('', text)
    if count != 1:
        raise ValueError(f'expected one plugin entry {name!r}, found {count}')
    return new


def replace_line(text, pattern, replacement):
    new, count = re.subn(pattern, replacement, text, count=1, flags=re.M)
    if count != 1:
        raise ValueError(f'expected a line matching {pattern!r}')
    return new


def prepare(text, english, site_url):
    for name in REQUIRED_PLUGINS:
        if not re.search(r'^  - ' + re.escape(name) + r'\b', text, re.M):
            raise ValueError(f'plugin {name!r} missing from upstream config')
    for name in PLUGIN_BLOCKS:
        text = remove_block(text, name)
    text = replace_line(text, r'^  - javascript/donation-banner\.js\n', '')
    text = replace_line(text, r'^edit_uri:.*\n', '')
    text = replace_line(text, r'^site_url:.*$', f'site_url: {site_url}')
    text = re.sub(r'^  analytics:\n(?:    [^\n]*\n)*', '', text, flags=re.M)
    hooks = ['hooks.py'] if english else ['hooks.py', 'hooks/ko_anchors.py']
    text += '\nhooks:\n' + ''.join(f'  - {h}\n' for h in hooks)
    if not english:
        text = replace_line(text, r'^copyright:.*$', COPYRIGHT_KO)
        text = replace_line(text, r'^site_name:.*$', 'site_name: 경쟁 프로그래밍을 위한 알고리즘')
        text = replace_line(text, r'^theme:\n', 'theme:\n  language: ko\n')
        text = replace_line(text, r'^  - search\n', '  - search:\n      lang:\n        - ko\n        - en\n')
    return text


def explain_tags(text):
    """Put the meaning of the English tag names above the tag index."""
    if TAGS_MARKER not in text:
        raise ValueError(f'{TAGS_MARKER} missing from tags.md')
    return text.replace(TAGS_MARKER, TAGS_NOTE + TAGS_MARKER, 1)


def rewrite(path, transform):
    """Replace `path` with transform(content). The assembled tree links its
    files into the layers, so the link is replaced, never written through."""
    text = path.read_text()
    if path.is_symlink():
        path.unlink()
    path.write_text(transform(text))


def main(argv=None):
    parser = argparse.ArgumentParser()
    parser.add_argument('--english', action='store_true')
    parser.add_argument('--site-url', default='https://yeokja.github.io/yeokja/cp-algorithms/')
    parser.add_argument('tree')
    args = parser.parse_args(argv)
    tree = Path(args.tree)
    rewrite(tree / 'mkdocs.yml', lambda text: prepare(text, args.english, args.site_url))
    if not args.english:
        rewrite(tree / 'src' / 'tags.md', explain_tags)


if __name__ == '__main__':
    main()
