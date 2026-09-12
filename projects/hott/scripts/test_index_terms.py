import unittest
from index_terms import calls, components, localize, restore_indices

class IndexTests(unittest.TestCase):
    def test_components_preserve_tex_and_makeindex_syntax(self):
        self.assertEqual(components(r'group!abelian@\emph{abelian}|('), ['group', r'\emph{abelian}'])
        self.assertEqual(components('operator"!'), ['operator"!'])
        self.assertEqual(localize(r'group!abelian@\emph{abelian}|(',
                                  {'group':'군', r'\emph{abelian}':r'\emph{아벨}'}),
                         r'group@군!abelian@\emph{아벨}|(')

    def test_restore_translated_keys_in_place_with_original_ranges(self):
        source = r'A\index{group|(} B\indexdef{group!abelian} C\index{group|)}'
        translated = r'가\index{그룹|(} 나\indexdef{그룹!가환} 다\index{그룹|)}'
        self.assertEqual(restore_indices(source, translated, {'group':'군','abelian':'아벨'}),
                         r'가\index{group@군|(} 나\indexdef{group@군!abelian@아벨} 다\index{group@군|)}')

    def test_see_targets_and_comments(self):
        source = '% \\index{comment}\n'+r'\indexsee{source}{domain}'
        self.assertEqual(len(list(calls(source))), 1)
        self.assertEqual(restore_indices(source,source,{'source':'시작점','domain':'정의역'}),
                         '% \\index{comment}\n'+r'\indexsee{source@시작점}{정의역}')

    def test_missing_marker_is_a_build_error(self):
        with self.assertRaises(ValueError):
            restore_indices(r'A\index{group}', '가', {'group':'군'})

if __name__ == '__main__': unittest.main()
