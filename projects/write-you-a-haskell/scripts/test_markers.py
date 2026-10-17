import unittest
from check_markers import compare


class MarkerTests(unittest.TestCase):
    def test_identical_markup_passes(self):
        source = 'Read $x$ as [@pierce2002].\n\n{{#include ../../crates/a.rs:ast}}\n\n```rust\nfn f() {}\n```\n'
        translated = source.replace('Read', '읽기').replace(' as ', '는 ')
        self.assertEqual(compare(source, translated), [])

    def test_lost_dollar_is_reported(self):
        self.assertEqual(compare('$x$ and $y$', '$x$와 y'), ['dollar: 4 -> 2'])

    def test_rewritten_include_and_citation_are_reported(self):
        problems = compare('{{#include a.rs:x}} [@key]', '{{#include b.rs:x}} [@키]')
        self.assertEqual(len(problems), 2)
        self.assertIn("include: lost ['{{#include a.rs:x}}'] gained ['{{#include b.rs:x}}']", problems)
        # A translated key is no citation at all, so it counts as lost, not gained.
        self.assertIn("citation: lost ['[@key]'] gained []", problems)

    def test_fence_count_change_is_reported(self):
        self.assertEqual(compare('```\nx\n```\n', 'x\n'), ['fence: 2 -> 0'])


if __name__ == '__main__':
    unittest.main()
