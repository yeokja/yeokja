from pathlib import Path
import tempfile
import unittest

from prepare_site import ko_labels, main, note_contributing, prepare

UPSTREAM = '''site_name: Linux Kernel Internals
site_description: Documentation about Linux kernel internals
site_url: https://kernel-internals.org/

theme:
  name: material
  palette:
    - scheme: default
      toggle:
        name: Switch to dark mode
    - scheme: slate
      toggle:
        name: Switch to light mode

markdown_extensions:
  - pymdownx.superfences:
      custom_fences:
        - name: mermaid
          format: !!python/name:pymdownx.superfences.fence_code_format

plugins:
  - search

extra:
  generator: false
  social:
    - icon: fontawesome/brands/github
      link: https://github.com/laveeshb/linux-kernel-internals
    - icon: fontawesome/solid/bug
      link: https://github.com/laveeshb/linux-kernel-internals/issues
      name: Issues

extra_css:
  - stylesheets/extra.css

nav:
  - Home: index.md
  - Foundations:
    - categories/foundations.md
    - "vDSO: Fast Syscalls": mm/vdso.md
'''
NAV = '- 홈\n- 기초\n    - vDSO: 빠른 "시스템 콜"\n'


class PrepareTests(unittest.TestCase):
    def test_korean_config(self):
        out = prepare(UPSTREAM, NAV)
        self.assertIn('site_name: Linux 커널 내부 구조\n', out)
        self.assertIn('site_url: https://yeokja.moreal.dev/linux-kernel-internals/\n', out)
        self.assertIn('  language: ko\n', out)
        self.assertIn('      lang:\n        - ko\n        - en\n', out)
        self.assertIn('  - stylesheets/ko.css\n', out)
        self.assertIn('hooks:\n  - hooks/anchors.py\n', out)
        self.assertIn('!!python/name:pymdownx.superfences.fence_code_format', out)
        for kept in ('Laveesh Bansal', 'CC BY-SA 4.0', 'GPL-2.0', 'Larry Ewing', 'Simon Budig', '비공식 기계 번역'):
            self.assertIn(kept, out)
        self.assertNotIn('/issues', out)
        self.assertNotIn('Switch to', out)

    def test_nav_labels_are_replaced_in_place(self):
        out = prepare(UPSTREAM, NAV)
        self.assertIn('  - "홈": index.md\n', out)
        self.assertIn('  - "기초":\n    - categories/foundations.md\n', out)
        self.assertIn('    - "vDSO: 빠른 \\"시스템 콜\\"": mm/vdso.md\n', out)

    def test_label_count_mismatch_fails(self):
        with self.assertRaises(ValueError):
            prepare(UPSTREAM, '- 홈\n- 기초\n')

    def test_depth_mismatch_fails(self):
        with self.assertRaises(ValueError):
            prepare(UPSTREAM, '- 홈\n- 기초\n- vDSO\n')

    def test_changed_upstream_config_fails(self):
        with self.assertRaises(ValueError):
            prepare(UPSTREAM.replace('  - search\n', '  - search:\n      lang: en\n'), NAV)

    def test_english_config_only_gets_the_hook(self):
        out = prepare(UPSTREAM)
        self.assertEqual(out, UPSTREAM + '\nhooks:\n  - hooks/anchors.py\n')

    def test_labels_are_unescaped_plain_text(self):
        self.assertEqual(ko_labels('- 1\\. `fork()`가 하는 일\n    - io\\_uring\n'),
                         [(0, '1. fork()가 하는 일'), (1, 'io_uring')])

    def test_contributing_note_goes_under_the_title(self):
        out = note_contributing('# Contributing\n\nThanks.\n')
        self.assertTrue(out.startswith('# Contributing\n\n*이 쪽은'))
        self.assertTrue(out.endswith('\n\nThanks.\n'))

    def test_main_replaces_links_instead_of_writing_through(self):
        with tempfile.TemporaryDirectory() as d:
            root = Path(d)
            layer, tree = root / 'layer', root / 'tree'
            (layer / 'docs').mkdir(parents=True)
            (tree / 'docs').mkdir(parents=True)
            (layer / 'mkdocs.yml').write_text(UPSTREAM)
            (layer / 'docs' / 'contributing.md').write_text('# Contributing\n')
            (root / 'nav.md').write_text(NAV)
            (tree / 'mkdocs.yml').symlink_to(layer / 'mkdocs.yml')
            (tree / 'docs' / 'contributing.md').symlink_to(layer / 'docs' / 'contributing.md')
            main([str(tree), str(root / 'nav.md')])
            self.assertEqual((layer / 'mkdocs.yml').read_text(), UPSTREAM)
            self.assertEqual((layer / 'docs' / 'contributing.md').read_text(), '# Contributing\n')
            self.assertFalse((tree / 'mkdocs.yml').is_symlink())
            self.assertIn('language: ko', (tree / 'mkdocs.yml').read_text())
            self.assertIn('yeokja', (tree / 'docs' / 'contributing.md').read_text())


if __name__ == '__main__':
    unittest.main()
