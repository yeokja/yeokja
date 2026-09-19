from pathlib import Path
import tempfile
import unittest

from prepare_site import explain_tags, main, prepare

UPSTREAM = '''site_name: Algorithms for Competitive Programming
site_url: https://cp-algorithms.com
edit_uri: edit/main/src/
copyright: Text is available under the <a href="x">CC BY-SA 4.0</a> License
extra_javascript:
  - javascript/config.js
  - javascript/donation-banner.js
  - https://unpkg.com/mathjax@3/es5/tex-mml-chtml.js
markdown_extensions:
  - pymdownx.emoji:
      emoji_index: !!python/name:material.extensions.emoji.twemoji
plugins:
  - toggle-sidebar:
      toggle_button: all
  - mkdocs-simple-hooks:
      hooks:
          on_env: "hooks:on_env"
  - search
  - tags
  - literate-nav:
      nav_file: navigation.md
  - git-revision-date-localized:
      enabled: !ENV [MKDOCS_ENABLE_GIT_REVISION_DATE, False]
  - git-authors
  - git-committers:
      token: !ENV MKDOCS_GIT_COMMITTERS_APIKEY
  - macros
  - rss
extra:
  analytics:
    provider: google
theme:
  name: material
'''

TAGS = '''# 태그

이 파일은 모든 페이지에서 쓰인 태그의 전체 색인입니다.

<!-- material/tags -->
'''


class PrepareTests(unittest.TestCase):
    def test_korean_config(self):
        out = prepare(UPSTREAM, english=False, site_url='https://example.org/cp-algorithms/')
        for gone in ('toggle-sidebar', 'mkdocs-simple-hooks', 'git-revision', 'git-authors',
                     'git-committers', '- rss', 'analytics', 'donation-banner', 'edit_uri'):
            self.assertNotIn(gone, out)
        self.assertIn('language: ko', out)
        self.assertIn('hooks/ko_anchors.py', out)
        self.assertIn('- hooks.py', out)
        self.assertIn('site_url: https://example.org/cp-algorithms/', out)
        self.assertIn('!!python/name:material.extensions.emoji.twemoji', out)
        self.assertIn('비공식 번역', out)

    def test_english_config_has_no_korean_hook(self):
        out = prepare(UPSTREAM, english=True, site_url='https://example.org/')
        self.assertNotIn('ko_anchors', out)
        self.assertNotIn('language: ko', out)
        self.assertIn('- hooks.py', out)

    def test_missing_expected_entry_fails(self):
        with self.assertRaises(ValueError):
            prepare(UPSTREAM.replace('  - macros\n', ''), english=False, site_url='x')


class TagsTests(unittest.TestCase):
    def test_tag_names_are_explained_before_the_index(self):
        out = explain_tags(TAGS)
        self.assertIn('"Translated"', out)
        self.assertIn('"Original"', out)
        self.assertLess(out.index('"Translated"'), out.index('<!-- material/tags -->'))

    def test_missing_marker_fails(self):
        with self.assertRaises(ValueError):
            explain_tags('# 태그\n')

    def test_main_replaces_links_instead_of_writing_through(self):
        with tempfile.TemporaryDirectory() as d:
            root = Path(d)
            layer, tree = root / 'layer', root / 'tree'
            (layer / 'src').mkdir(parents=True)
            (tree / 'src').mkdir(parents=True)
            (layer / 'mkdocs.yml').write_text(UPSTREAM)
            (layer / 'src' / 'tags.md').write_text(TAGS)
            (tree / 'mkdocs.yml').symlink_to(layer / 'mkdocs.yml')
            (tree / 'src' / 'tags.md').symlink_to(layer / 'src' / 'tags.md')
            main([str(tree)])
            self.assertEqual((layer / 'mkdocs.yml').read_text(), UPSTREAM)
            self.assertEqual((layer / 'src' / 'tags.md').read_text(), TAGS)
            self.assertFalse((tree / 'mkdocs.yml').is_symlink())
            self.assertIn('language: ko', (tree / 'mkdocs.yml').read_text())
            self.assertIn('"Original"', (tree / 'src' / 'tags.md').read_text())


if __name__ == '__main__':
    unittest.main()
