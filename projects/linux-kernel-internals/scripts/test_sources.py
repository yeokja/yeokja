import unittest

from sources import nav_markdown, normalize, parse_nav

CONFIG = '''site_name: X
nav:
  - Home: index.md
  - Foundations:
    - categories/foundations.md
    - Kernel Internals:
      - Getting Started: kernel/README.md
      - "vDSO: Fast Syscalls": mm/vdso.md
      - 'The Cost of "Asking" Too Often': sched/cost.md
  - About:
    - about.md
extra:
  x: 1
'''


class NavTests(unittest.TestCase):
    def test_labels_become_a_nested_list_without_unlabelled_pages(self):
        self.assertEqual(nav_markdown(CONFIG), (
            '- Home\n'
            '- Foundations\n'
            '    - Kernel Internals\n'
            '        - Getting Started\n'
            '        - vDSO: Fast Syscalls\n'
            '        - The Cost of "Asking" Too Often\n'
            '- About\n'))

    def test_block_ends_at_the_next_top_level_key(self):
        self.assertEqual(len(parse_nav(CONFIG)), 9)

    def test_unrecognized_line_fails(self):
        with self.assertRaises(ValueError):
            parse_nav('nav:\n  - Home: [index.md]\n')

    def test_label_with_markup_fails(self):
        with self.assertRaises(ValueError):
            nav_markdown('nav:\n  - "The `x` Story": a.md\n')

    def test_intraword_underscore_is_text(self):
        self.assertEqual(nav_markdown('nav:\n  - io_uring vs epoll: a.md\n'), '- io_uring vs epoll\n')


class NormalizeTests(unittest.TestCase):
    def test_header_gets_its_space(self):
        self.assertEqual(normalize('a\n\n!!!note\n    body\n\n!!!tip "T"\n    - x\n'),
                         'a\n\n!!! note\n    body\n\n!!! tip "T"\n    - x\n')

    def test_indented_header(self):
        self.assertEqual(normalize('- a\n\n    !!!warning\n        b\n'), '- a\n\n    !!! warning\n        b\n')

    def test_page_without_such_headers_is_not_derived(self):
        self.assertIsNone(normalize('!!! note\n    body\n\nWow!!! Really.\n'))

    def test_fenced_code_is_left_alone(self):
        self.assertIsNone(normalize('```\n!!!note\n```\n'))


if __name__ == '__main__':
    unittest.main()
