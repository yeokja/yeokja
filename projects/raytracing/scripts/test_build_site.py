import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest


BUILD_SCRIPT = Path(__file__).with_name('build_site.py')


class BuildSiteGuardTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        self.project = self.root / 'project'
        self.tree = self.root / 'tree'
        self.source = self.project / 'upstream/books/Book.html'
        self.translation = self.project / 'ko/books/Book.html'
        self.state = self.project / 'state/upstream/books/Book.html.yeokja.json'
        self.assembled = self.tree / 'books/Book.html'
        self.published = self.tree / 'site/index.html'
        self.write(self.source, 'English book')
        self.write(self.translation, '한국어 책')
        self.write(self.assembled, '한국어 책')
        self.write(self.published, 'previous completed build')
        self.write_state([{'translation': '한국어 책', 'issues': []}])

    @staticmethod
    def write(path, text):
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(text, encoding='utf-8')

    def write_state(self, segments):
        self.write(self.state, json.dumps({'segments': segments}, ensure_ascii=False))

    def assert_rejected_before_staging(self, message):
        result = subprocess.run(
            [sys.executable, str(BUILD_SCRIPT), str(self.tree), str(self.project)],
            capture_output=True, text=True,
        )
        self.assertNotEqual(result.returncode, 0, result.stdout)
        self.assertIn(message, result.stderr)
        self.assertEqual(self.published.read_text(), 'previous completed build')
        self.assertEqual(list((self.tree / 'site').iterdir()), [self.published])

    def test_missing_translation_overlay_rejects_english_base(self):
        self.translation.unlink()
        self.write(self.assembled, 'English book')
        self.assert_rejected_before_staging('Missing translation output/state for Book.html')

    def test_missing_translation_state_rejects_existing_overlay(self):
        self.state.unlink()
        self.assert_rejected_before_staging('Missing translation output/state for Book.html')

    def test_pending_segment_rejects_existing_overlay(self):
        self.write_state([{'translation': '한국어 책', 'issues': []},
                          {'translation': None, 'issues': []}])
        self.assert_rejected_before_staging('Incomplete or failed translation for Book.html')

    def test_validation_issue_rejects_translated_segment(self):
        self.write_state([{'translation': '한국어 책',
                           'issues': [{'severity': 'error', 'message': 'lost code'}]}])
        self.assert_rejected_before_staging('Incomplete or failed translation for Book.html')

    def test_empty_state_rejects_existing_overlay(self):
        self.write_state([])
        self.assert_rejected_before_staging('Incomplete or failed translation for Book.html')

    def test_stale_assembled_book_rejects_current_translation(self):
        self.write(self.assembled, '이전 번역')
        self.assert_rejected_before_staging('Assembled translation is stale for Book.html')


if __name__ == '__main__':
    unittest.main()
