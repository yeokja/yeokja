from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

MODULE = Path(__file__).with_name('verify_books.py')

SOURCE = r'''English heading
===
English prose with $x + y$ and Figure [sphere], Listing [main].

    ~~~~~ C++
    int x = 1;
    ~~~~~ C++ highlight
    int y = 2;
    ~~~~~ C++
    return x + y;
    ~~~~~
    [Listing [main]: Example]

$$ E = mc^2 $$
![Figure [sphere]: Sphere](../images/sphere.png)
See [the documentation][docs].
[docs]: https://example.org/docs
<link href='../style/book.css'>
'''


class VerifyBooksTests(unittest.TestCase):
    def verify(self, target, source=SOURCE):
        self.assertTrue(MODULE.exists(), 'verification CLI must exist')
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            for name, content in [('upstream', source), ('ko', target)]:
                (root / name).mkdir()
                (root / name / 'Book.html').write_text(content)
            return subprocess.run([sys.executable, str(MODULE), '--source', str(root / 'upstream'),
                                   '--target', str(root / 'ko')], capture_output=True, text=True)

    def test_translated_prose_passes(self):
        result = self.verify(SOURCE.replace('English prose', '한국어 설명').replace('Example]', '예제]'))
        self.assertEqual(result.returncode, 0, result.stderr)

    def test_corrupted_code_after_highlight_switch_fails(self):
        result = self.verify(SOURCE.replace('int y = 2;', 'int y = 3;'))
        self.assertNotEqual(result.returncode, 0)
        self.assertIn('code', result.stderr)
        self.assertIn('Book.html:', result.stderr)

    def test_changed_highlight_switch_fails(self):
        result = self.verify(SOURCE.replace('C++ highlight', 'C++'))
        self.assertNotEqual(result.returncode, 0)
        self.assertIn('code', result.stderr)

    def test_corrupted_inline_and_display_math_fail(self):
        for original, corrupted in [('x + y', 'x - y'), ('mc^2', 'mc^3')]:
            with self.subTest(original=original):
                result = self.verify(SOURCE.replace(original, corrupted))
                self.assertNotEqual(result.returncode, 0)
                self.assertIn('math', result.stderr)

    def test_corrupted_reference_definition_fails(self):
        result = self.verify(SOURCE.replace('[docs]:', '[missing]:'))
        self.assertNotEqual(result.returncode, 0)
        self.assertIn('reference', result.stderr)

    def test_corrupted_identifier_and_reference_use_fail(self):
        for original in ['[Figure [sphere]:', 'Figure [sphere],', '[the documentation][docs]']:
            with self.subTest(original=original):
                result = self.verify(SOURCE.replace(original, original.replace('sphere', 'oops').replace('[docs]', '[oops]')))
                self.assertNotEqual(result.returncode, 0)

    def test_changed_figure_or_local_asset_fails(self):
        for original in ['../images/sphere.png', '../style/book.css']:
            with self.subTest(original=original):
                result = self.verify(SOURCE.replace(original, '../missing.png'))
                self.assertNotEqual(result.returncode, 0)
                self.assertIn('asset', result.stderr)

    def test_bare_cross_reference_and_include_corruption_fail(self):
        source = SOURCE + "See [main].\n(insert acknowledgments.md.html here)\n"
        for original, replacement in [('[main].', '[missing].'), ('acknowledgments.md.html', 'missing.html')]:
            with self.subTest(original=original):
                result = self.verify(source.replace(original, replacement), source)
                self.assertNotEqual(result.returncode, 0)

    def test_centered_title_and_indented_prose_can_be_translated(self):
        source = "                      **English title**\n\n    Paragraph continuation\n" + SOURCE
        target = source.replace('English title', '한국어 제목').replace('Paragraph continuation', '문단 이어짐')
        result = self.verify(target, source)
        self.assertEqual(result.returncode, 0, result.stderr)

    def test_korean_math_word_order_and_translated_figure_word_pass(self):
        source = SOURCE + "At $t=0$, $C$ has value $v$. See figure [sphere].\n"
        target = source.replace('At $t=0$, $C$ has value $v$. See figure [sphere].',
                                '$C$는 $t=0$에서 값 $v$를 가집니다. 그림 [sphere]를 보십시오.')
        result = self.verify(target, source)
        self.assertEqual(result.returncode, 0, result.stderr)

    def test_missing_translated_file_fails(self):
        self.assertTrue(MODULE.exists())
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / 'Book.html').write_text(SOURCE)
            result = subprocess.run([sys.executable, str(MODULE), '--source', str(root),
                                     '--target', str(root / 'missing')], capture_output=True, text=True)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn('missing translated book', result.stderr)

    def test_plain_indented_fences_are_preserved(self):
        for source in ['Text\n\n    ~~~~~\n    echo hello\n    ~~~~~\n']:
            result = self.verify(source.replace('hello', 'oops').replace('value = 1', 'value = 2'), source)
            self.assertNotEqual(result.returncode, 0)
            self.assertIn('code', result.stderr)


if __name__ == '__main__':
    unittest.main()
