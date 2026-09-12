from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

SCRIPT = Path(__file__).with_name('prepare_site.py')

class PrepareSiteTests(unittest.TestCase):
    def test_mounts_home_assets_and_examples_without_rewriting_external_urls(self):
        with tempfile.TemporaryDirectory() as tmp:
            site = Path(tmp)
            ko = site / 'webgpu/lessons/ko'
            ko.mkdir(parents=True)
            (ko / 'index.html').write_text('한국어')
            lesson = ko / 'lesson.html'
            lesson.write_text('''<a href="/">home</a>
<a href="/contributors.html">contributors</a>
<script src="/webgpu/resources/test.js"></script>
<iframe src="../example.html"></iframe>
<a href="//example.com/x">external</a>
<a href="/webgpufundamentals/webgpu/">already mounted</a>''')
            (site / 'sitemap.xml').write_text('<loc>https://webgpufundamentals.org/webgpu/lessons/ko/lesson.html</loc>')
            (site / 'CNAME').write_text('webgpufundamentals.org')
            subprocess.run([sys.executable, str(SCRIPT), tmp], check=True)
            result = lesson.read_text()
            self.assertIn('href="/webgpufundamentals/"', result)
            self.assertIn('href="/webgpufundamentals/contributors.html"', result)
            self.assertIn('src="/webgpufundamentals/webgpu/resources/test.js"', result)
            self.assertIn('src="../example.html"', result)
            self.assertIn('href="//example.com/x"', result)
            self.assertNotIn('/webgpufundamentals/webgpufundamentals/', result)
            self.assertEqual((site / 'sitemap.xml').read_text(), '<loc>https://yeokja.moreal.dev/webgpufundamentals/webgpu/lessons/ko/lesson.html</loc>')
            self.assertFalse((site / 'CNAME').exists())
            self.assertIn('webgpu/lessons/ko/', (site / 'index.html').read_text())
            subprocess.run([sys.executable, str(SCRIPT), tmp], check=True)
            self.assertEqual(result, lesson.read_text())

if __name__ == '__main__':
    unittest.main()
