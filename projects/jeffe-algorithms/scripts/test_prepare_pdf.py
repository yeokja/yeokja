from pathlib import Path
import tempfile
import unittest
from prepare_pdf import check, fix_quotes, smart_quotes


class CheckTests(unittest.TestCase):
    def test_complete_tree_passes(self):
        with tempfile.TemporaryDirectory() as d:
            t = Path(d)
            (t / 'frontmatter').mkdir()
            (t / 'chapters').mkdir()
            for p in ('main.tex', 'preamble.tex', 'frontmatter/translation-notice.tex', 'chapters/02.tex'):
                (t / p).write_text('x')
            self.assertEqual(check(t), [])

    def test_missing_notice_fails(self):
        with tempfile.TemporaryDirectory() as d:
            (Path(d) / 'main.tex').write_text('x')
            self.assertIn('missing frontmatter/translation-notice.tex', check(d))

    def test_edition_switch_inside_a_chapter_fails(self):
        with tempfile.TemporaryDirectory() as d:
            t = Path(d)
            (t / 'frontmatter').mkdir()
            (t / 'chapters').mkdir()
            for p in ('main.tex', 'preamble.tex', 'frontmatter/translation-notice.tex'):
                (t / p).write_text('x')
            (t / 'chapters/02.tex').write_text('\\koreantrue\n')
            self.assertEqual(check(t), ['02.tex: edition switch inside a chapter'])


class QuoteTests(unittest.TestCase):
    def test_ascii_double_quotes_become_curly_pairs(self):
        self.assertEqual(smart_quotes('필명 "Schachfreund"로, 혹은 "명백히" 좋은'),
                         '필명 “Schachfreund”로, 혹은 “명백히” 좋은')

    def test_math_escapes_and_existing_curly_quotes_are_kept(self):
        line = '$w$가 "단어" \\" $a"b$ “이미”'
        self.assertEqual(smart_quotes(line), '$w$가 “단어” \\" $a"b$ “이미”')

    def test_unpaired_quote_is_left_alone(self):
        self.assertEqual(smart_quotes('a "b" c "d'), 'a “b” c "d')

    def test_fix_replaces_a_linked_file_without_touching_its_target(self):
        with tempfile.TemporaryDirectory() as d:
            t = Path(d)
            original = t / 'ko.tex'
            original.write_text('say "hi"\n')
            (t / 'chapters').mkdir()
            link = t / 'chapters' / '02.tex'
            link.symlink_to(original)
            self.assertEqual(fix_quotes(t), ['02.tex'])
            self.assertEqual(original.read_text(), 'say "hi"\n')
            self.assertFalse(link.is_symlink())
            self.assertEqual(link.read_text(), 'say “hi”\n')


if __name__ == '__main__':
    unittest.main()
