import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

SCRIPT = Path(__file__).with_name('prepare_pdf.py')

class PreparePdfTests(unittest.TestCase):
    def test_localizes_layout_after_translated_heading_without_writing_base(self):
        with tempfile.TemporaryDirectory() as d:
            root = Path(d)
            base = root / 'base'
            tree = root / 'tree'
            base.mkdir(); tree.mkdir()
            symbols = '\\markboth{}{\\textsc{Index of symbols}}\n\\addcontentsline{toc}{part}{Index of symbols}\n\\chapter*{기호 색인}\n'
            front = '\\input{version.tex}\n저작권 안내\n'
            induction = r'\LEM을 사용합니다. $\LEM\infty$'
            (base / 'induction.tex').write_text(induction)
            (base / 'symbols.tex').write_text(symbols)
            (base / 'front.tex').write_text(front)
            for name in ('symbols.tex', 'front.tex', 'induction.tex'):
                (tree / name).symlink_to(base / name)
            result = subprocess.run([sys.executable, str(SCRIPT), str(tree)], capture_output=True, text=True)
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertIn('\\chapter*{기호 색인}', (tree / 'symbols.tex').read_text())
            self.assertIn('\\addcontentsline{toc}{part}{기호 찾아보기}', (tree / 'symbols.tex').read_text())
            self.assertIn('비공식 한국어 번역', (tree / 'front.tex').read_text())
            self.assertEqual((base / 'symbols.tex').read_text(), symbols)
            self.assertEqual((base / 'front.tex').read_text(), front)
            self.assertEqual((base / 'induction.tex').read_text(), induction)
            self.assertEqual((tree / 'induction.tex').read_text(), r'\LEM{}을 사용합니다. $\LEM\infty$')

if __name__ == '__main__': unittest.main()
