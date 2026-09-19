from pathlib import Path
import tempfile
import unittest
from restore_alerts import restore_alert_lines, restore


class RestoreAlertTests(unittest.TestCase):
    def test_marker_joined_with_prose_goes_back_to_its_own_line(self):
        self.assertEqual(
            restore_alert_lines('> [!WARNING] **경고 제목**\n>\n> 본문입니다.\n'),
            '> [!WARNING]\n> **경고 제목**\n>\n> 본문입니다.\n',
        )

    def test_indented_blockquote_keeps_its_prefix_on_both_lines(self):
        self.assertEqual(
            restore_alert_lines('- 항목\n\n  > [!NOTE] 참고 내용입니다.\n'),
            '- 항목\n\n  > [!NOTE]\n  > 참고 내용입니다.\n',
        )

    def test_every_gfm_alert_kind_is_restored(self):
        for kind in ('NOTE', 'TIP', 'IMPORTANT', 'WARNING', 'CAUTION'):
            self.assertEqual(restore_alert_lines(f'> [!{kind}] 문장.'), f'> [!{kind}]\n> 문장.')

    def test_marker_already_alone_and_other_text_are_unchanged(self):
        text = '> [!NOTE]\n> 문장.\n\n[!NOTE] 인용문 밖\n\n> [!참고] 번역된 마커\n'
        self.assertEqual(restore_alert_lines(text), text)

    def test_code_fences_are_left_alone(self):
        text = '```markdown\n> [!NOTE] 예시\n```\n'
        self.assertEqual(restore_alert_lines(text), text)

    def test_restore_replaces_links_without_writing_through(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            target = root / 'ko.md'
            target.write_text('> [!TIP] 팁입니다.\n')
            tree = root / 'tree'
            tree.mkdir()
            (tree / 'page.md').symlink_to(target)
            restore(tree)
            self.assertEqual((tree / 'page.md').read_text(), '> [!TIP]\n> 팁입니다.\n')
            self.assertFalse((tree / 'page.md').is_symlink())
            self.assertEqual(target.read_text(), '> [!TIP] 팁입니다.\n')


if __name__ == '__main__':
    unittest.main()
