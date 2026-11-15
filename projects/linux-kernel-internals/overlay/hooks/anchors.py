"""MkDocs hook: give translated headings the ids of their English originals.

The same hook runs in both builds. In the English reference build
(ANCHOR_IDS_OUT=<file>) a treeprocessor that runs after toc records every
heading id of every page and the hook writes them out when the build ends. In
the Korean build (KO_ANCHOR_IDS=<file>) a treeprocessor that runs after
attr_list and before toc sets those ids on the translated headings, so the toc,
the search index and every `#anchor` link use the English ids. Headings are
paired by document order; a count mismatch means translation changed the
heading structure and fails the build.
"""
import json
import os
from pathlib import Path

from markdown.extensions import Extension
from markdown.treeprocessors import Treeprocessor

HEADINGS = ('h1', 'h2', 'h3', 'h4', 'h5', 'h6')
_state = {'page': None, 'ids': {}, 'recorded': {}}


def headings(root):
    return [el for el in root.iter() if el.tag in HEADINGS]


class RecordIds(Treeprocessor):
    def run(self, root):
        _state['recorded'][_state['page']] = [el.get('id') for el in headings(root)]


class EnglishIds(Treeprocessor):
    def run(self, root):
        page = _state['page']
        ids = _state['ids'].get(page)
        if ids is None:
            raise ValueError(f'{page}: no English heading ids (page missing from the English build)')
        found = headings(root)
        if len(found) != len(ids):
            titles = [''.join(el.itertext()) for el in found]
            raise ValueError(f'{page}: {len(found)} translated headings {titles} '
                             f'vs {len(ids)} English ids {ids}')
        for element, heading_id in zip(found, ids):
            element.set('id', heading_id)


class AnchorsExtension(Extension):
    def __init__(self, record):
        super().__init__()
        self.record = record

    def extendMarkdown(self, md):
        # attr_list is 8 and toc is 5 in Python-Markdown's treeprocessor registry.
        if self.record:
            md.treeprocessors.register(RecordIds(md), 'record_anchor_ids', 1)
        else:
            md.treeprocessors.register(EnglishIds(md), 'english_anchor_ids', 6)


def on_config(config):
    record = 'ANCHOR_IDS_OUT' in os.environ
    if not record:
        _state['ids'] = json.loads(Path(os.environ['KO_ANCHOR_IDS']).read_text())
    config.markdown_extensions.append(AnchorsExtension(record))
    return config


def on_page_markdown(markdown, page, config, files):
    _state['page'] = page.file.src_uri
    return markdown


def on_post_build(config):
    if 'ANCHOR_IDS_OUT' in os.environ:
        Path(os.environ['ANCHOR_IDS_OUT']).write_text(
            json.dumps(_state['recorded'], ensure_ascii=False, indent=1, sort_keys=True))
