import importlib.util
import os
from pathlib import Path
import subprocess
import tempfile
import unittest


def load(name):
    spec = importlib.util.spec_from_file_location(name.replace('-', '_'), Path(__file__).with_name(name))
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


build_site = load('build-site.py')


class RewriteUrlTest(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.tree = Path(self.tmp.name)
        (self.tree / 'labs/core_bound/vectorization_1').mkdir(parents=True)
        (self.tree / 'labs/core_bound/vectorization_1/solution.cpp').write_text('')
        (self.tree / 'logo.jpg').write_bytes(b'')
        self.documents = {Path('README.md'), Path('GetStarted.md'), Path('labs/core_bound/vectorization_1/README.md')}

    def tearDown(self):
        self.tmp.cleanup()

    def rewrite(self, url, current='README.md'):
        return build_site.rewrite_url(url, Path(current), self.documents, self.tree)

    def test_directory_link_becomes_its_readme_page(self):
        self.assertEqual(self.rewrite('labs/core_bound/vectorization_1'), 'labs/core_bound/vectorization_1/index.html')

    def test_root_absolute_path_becomes_document_relative(self):
        self.assertEqual(self.rewrite('/logo.jpg'), 'logo.jpg')
        self.assertEqual(self.rewrite('/logo.jpg', 'labs/core_bound/vectorization_1/README.md'), '../../../logo.jpg')

    def test_markdown_link_keeps_anchor(self):
        self.assertEqual(self.rewrite('../../../GetStarted.md#how-to-build-lab-assignments', 'labs/core_bound/vectorization_1/README.md'),
                         '../../../GetStarted.html#how-to-build-lab-assignments')

    def test_main_branch_link_stays_in_site_and_golden_stays_upstream(self):
        main = 'https://github.com/dendibakh/perf-ninja/blob/main/labs/core_bound/vectorization_1/solution.cpp'
        golden = 'https://github.com/dendibakh/perf-ninja/blob/golden/labs/core_bound/vectorization_1/solution.cpp'
        self.assertEqual(self.rewrite(main), 'labs/core_bound/vectorization_1/solution.cpp')
        self.assertEqual(self.rewrite(golden), golden)

    def test_missing_file_links_upstream(self):
        self.assertEqual(self.rewrite('labs/gone.cpp'), 'https://github.com/dendibakh/perf-ninja/blob/main/labs/gone.cpp')


class BuildTest(unittest.TestCase):
    def test_translated_page_keeps_original_heading_ids(self):
        script = Path(__file__).with_name('build-site.py')
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            upstream, tree = root / 'upstream', root / 'tree'
            for base in (upstream, tree):
                (base / 'lab').mkdir(parents=True)
                (base / 'lab/solution.cpp').write_text('int main() {}\n')
            (upstream / 'README.md').write_text('# Performance Ninja Class\n\n[Lab](lab)\n')
            (tree / 'README.md').write_text('# 퍼포먼스 닌자\n\n[실습](lab)\n')
            (upstream / 'lab/README.md').write_text('Intro\n\n<details>\n<summary><b>Hint 1:</b></summary>\n\n## Background:\n\nText\n\n</details>\n')
            (tree / 'lab/README.md').write_text('소개\n\n<details>\n<summary><b>힌트 1:</b></summary>\n\n## 배경:\n\n본문\n\n</details>\n')
            subprocess.run(['python3', str(script)], cwd=tree, env={**os.environ, 'YEOKJA_ROOT': str(root)}, check=True)
            home = (tree / 'site/index.html').read_text()
            lab = (tree / 'site/lab/index.html').read_text()
            self.assertIn('href="lab/index.html"', home)
            self.assertIn('id="background"', lab)
            self.assertIn('<summary><b>힌트 1:</b></summary>', lab)
            self.assertIn('<title>lab · ', lab)
            self.assertEqual((tree / 'site/lab/solution.cpp').read_text(), 'int main() {}\n')
            check = Path(__file__).with_name('check_site.py')
            subprocess.run(['python3', str(check), 'site'], cwd=tree, env={**os.environ, 'YEOKJA_ROOT': str(root)}, check=True)


if __name__ == '__main__':
    unittest.main()
