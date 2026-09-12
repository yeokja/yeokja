import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

import prepare_index


class PrepareIndexTests(unittest.TestCase):
    def test_roundtrip_keeps_inline_links_and_omitted_paragraph_ends(self):
        source = ('<!DOCTYPE html>\n<title>Home</title><div><h1>Book<br>Series</h1>'
                  '<img src="cover.jpg" alt="Book cover">'
                  '<p>Read <a href="https://example.org">the book</a> now.'
                  '<p>Next paragraph.<ul><li>ZIP format<li>Tar format</ul></div>')
        units = prepare_index.extract_units(source)
        self.assertEqual(len(units), 7)
        self.assertIn('Read `HTML_000`the book`HTML_001` now.', [u.text for u in units])
        translated = prepare_index.markdown(units)
        for unit in units:
            translated = translated.replace(unit.text, '번역 ' + unit.text)
        rendered = prepare_index.render(source, translated)
        self.assertIn('<p>번역 Read <a href="https://example.org">the book</a> now.', rendered)
        self.assertIn('alt="번역 Book cover"', rendered)
        self.assertIn('<h1>번역 Book<br>Series</h1>', rendered)

    def test_missing_or_unchanged_translation_fails(self):
        source = '<title>Home</title><p>Read this.'
        md = prepare_index.markdown(prepare_index.extract_units(source))
        with self.assertRaisesRegex(ValueError, 'Korean'):
            prepare_index.render(source, md)
        with self.assertRaisesRegex(ValueError, 'unit'):
            prepare_index.render(source, '')

    def test_missing_inline_token_fails(self):
        source = '<p>Read <em>this</em> now.'
        md = prepare_index.markdown(prepare_index.extract_units(source))
        md = md.replace('Read', '번역').replace('`HTML_000`', '')
        with self.assertRaisesRegex(ValueError, 'token'):
            prepare_index.render(source, md)

    def test_html_in_translation_is_escaped(self):
        source = '<title>Home</title><img alt="Cover">'
        md = prepare_index.markdown(prepare_index.extract_units(source))
        md = md.replace('Home', '번역 <script>').replace('Cover', '번역 " &')
        rendered = prepare_index.render(source, md)
        self.assertIn('번역 &lt;script&gt;', rendered)
        self.assertIn('alt="번역 &quot; &amp;"', rendered)

    def test_extract_check_detects_stale_without_writing(self):
        script = Path(prepare_index.__file__).resolve()
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory) / 'index.md'
            output.write_text('stale')
            result = subprocess.run(
                [sys.executable, str(script), 'extract', '--check', '--markdown', str(output)],
                capture_output=True, text=True,
            )
            self.assertNotEqual(result.returncode, 0)
            self.assertIn('stale', result.stderr)
            self.assertEqual(output.read_text(), 'stale')
            source = (script.parents[1] / 'upstream/index.html').read_text()
            output.write_text(prepare_index.markdown(prepare_index.extract_units(source)))
            result = subprocess.run(
                [sys.executable, str(script), 'extract', '--check', '--markdown', str(output)],
                capture_output=True, text=True,
            )
            self.assertEqual(result.returncode, 0, result.stderr)

    def test_upstream_has_complete_coverage(self):
        source = (Path(__file__).resolve().parents[1] / 'upstream/index.html').read_text()
        units = prepare_index.extract_units(source)
        self.assertEqual(len(units), 21)
        self.assertEqual(sum(u.kind == 'alt' for u in units), 3)
        self.assertEqual(sum(u.kind == 'title' for u in units), 1)


if __name__ == '__main__':
    unittest.main()
