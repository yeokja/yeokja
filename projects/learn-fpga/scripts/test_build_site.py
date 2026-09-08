import os
from pathlib import Path
import subprocess
import tempfile
import unittest
import importlib.util


class BuildSiteTest(unittest.TestCase):
    def test_repository_relative_image_in_nested_document(self):
        spec = importlib.util.spec_from_file_location('build_site', Path(__file__).with_name('build-site.py'))
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)
        with tempfile.TemporaryDirectory() as tmp:
            tree = Path(tmp)
            (tree / 'FemtoRV/Images').mkdir(parents=True)
            (tree / 'FemtoRV/Images/demo.gif').write_bytes(b'GIF89a')
            result = module.rewrite_url('FemtoRV/Images/demo.gif', Path('FemtoRV/README.md'), set(), tree)
            self.assertEqual(result, 'Images/demo.gif')

    def test_translated_links_original_anchors_and_assets(self):
        script = Path(__file__).with_name('build-site.py')
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            upstream = root / 'upstream'
            tree = root / 'tree'
            upstream.mkdir()
            tree.mkdir()
            for base in (upstream, tree):
                (base / 'chapter').mkdir()
                (base / 'chapter/code.v').write_text('module test; endmodule\n')
                (base / 'LICENSE').write_text('Original copyright notice')
            (upstream / 'README.md').write_text('# Hello FPGA\n\n[Next](chapter/README.md#first-step)')
            (tree / 'README.md').write_text('# FPGA 시작\n\n[다음](chapter/README.md#first-step)')
            (upstream / 'chapter/README.md').write_text('# First step\n\nExample')
            (tree / 'chapter/README.md').write_text('# 첫 단계\n\n[코드](code.v)\n\n[처음](https://github.com/BrunoLevy/learn-fpga/blob/master/README.md)\n\n```verilog\nmodule test; endmodule\n```')
            subprocess.run(['python3', str(script)], cwd=tree,
                           env={**os.environ, 'YEOKJA_ROOT': str(root)}, check=True)
            home = (tree / 'site/index.html').read_text()
            chapter = (tree / 'site/chapter/index.html').read_text()
            self.assertIn('chapter/index.html#first-step', home)
            self.assertIn('id="first-step"', chapter)
            self.assertIn('href="../index.html"', chapter)
            self.assertIn('첫 단계', chapter)
            self.assertIn('lang="ko"', home)
            self.assertEqual((tree / 'site/chapter/code.v').read_text(), 'module test; endmodule\n')
            self.assertEqual((tree / 'site/LICENSE').read_text(), 'Original copyright notice')


if __name__ == '__main__':
    unittest.main()
