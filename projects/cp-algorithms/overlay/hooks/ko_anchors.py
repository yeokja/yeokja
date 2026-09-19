"""MkDocs hook: give translated headings the ids of their English originals.

A Python-Markdown treeprocessor runs after attr_list and before toc, so the
toc, permalinks and search index all use the English ids. Headings are paired
by document order with the English build's list (extract_heading_ids.py); a
count mismatch means translation changed the heading structure and fails the
build.
"""
import json
import os
from pathlib import Path

from markdown.extensions import Extension
from markdown.treeprocessors import Treeprocessor

HEADINGS = ('h1', 'h2', 'h3', 'h4', 'h5', 'h6')
GENERATED = {'tags.md'}
_state = {'page': None, 'ids': {}}


class EnglishIds(Treeprocessor):
    def run(self, root):
        page = _state['page']
        ids = _state['ids'].get(page)
        if page in GENERATED or ids is None:
            return
        headings = [el for el in root.iter() if el.tag in HEADINGS]
        if len(headings) != len(ids):
            titles = [''.join(el.itertext()) for el in headings]
            raise ValueError(f'{page}: {len(headings)} translated headings {titles} '
                             f'vs {len(ids)} English ids {ids}')
        for element, heading_id in zip(headings, ids):
            element.set('id', heading_id)


class EnglishIdsExtension(Extension):
    def extendMarkdown(self, md):
        # attr_list is 8 and toc is 5 in Python-Markdown's treeprocessor registry.
        md.treeprocessors.register(EnglishIds(md), 'ko_anchors', 6)


def on_config(config):
    _state['ids'] = json.loads(Path(os.environ['KO_ANCHOR_IDS']).read_text())
    config.markdown_extensions.append(EnglishIdsExtension())
    return config


def on_page_markdown(markdown, page, config, files):
    _state['page'] = page.file.src_uri
    return markdown
