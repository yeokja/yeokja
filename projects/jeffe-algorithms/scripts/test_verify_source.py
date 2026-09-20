import unittest
from verify_source import (compare, exercise_numbers, figure_destinations, footnote_names, math_symbols,
                           pseudocode_index, refold_accents, residual_difference, words)


def text(*chunks):
    return [{'text': t, 'cls': 'text'} for t in chunks]


class WordTests(unittest.TestCase):
    def test_keeps_prose_drops_math_figures_and_running_heads(self):
        spans = [{'text': 'The algo-', 'cls': 'text'}, {'text': 'rithm runs', 'cls': 'text'},
                 {'text': 'x + y', 'cls': 'math'}, {'text': '2. BACKTRACKING', 'cls': 'header_footer'},
                 {'text': 'label', 'cls': 'figure'}, {'text': 'PlaceQueens', 'cls': 'code'}]
        self.assertEqual(words(spans), ['The', 'algorithm', 'runs', 'PlaceQueens'])

    def test_page_numbers_after_page_are_wildcards(self):
        self.assertEqual(words(text('see page 81 and pages 3–5')),
                         ['see', 'page', '#', 'and', 'pages', '#'])

    def test_span_boundaries_do_not_matter(self):
        # The original splits justified lines into per-word spans and breaks
        # spans at font changes; LuaLaTeX output splits differently.
        self.assertEqual(words(text('the classical', 'Queens Problem', ', first')),
                         words(text('the classical Queens Problem, first')))
        self.assertEqual(words(text('(“', 'Schwer', 'ist')), words(text('(“Schwer ist')))

    def test_punctuation_is_its_own_token(self):
        self.assertEqual(words(text('Problem, first.')), ['Problem', ',', 'first', '.'])

    def test_hyphenated_compounds_match_across_line_breaks(self):
        self.assertEqual(words(text('depth-', 'first search')), words(text('depth-first search')))

    def test_numbers_references_and_quotes(self):
        self.assertEqual(words(text('Figure 2.3 and ??')), ['Figure', '2.3', 'and', '??'])
        self.assertEqual(words(text('don’t ﬁnd')), words(text("don't find")))


class CompareTests(unittest.TestCase):
    def test_ratio_and_hunks(self):
        report = compare(['a', 'b', 'c', 'd'], ['a', 'x', 'c', 'd'])
        self.assertAlmostEqual(report.mismatch_ratio, 0.25)
        self.assertEqual(report.hunks, [(['b'], ['x'])])

    def test_unresolved_reference_matches_a_number(self):
        report = compare(['Figure', '1.3', 'shows'], ['Figure', '??', 'shows'])
        self.assertEqual(report.mismatch_ratio, 0)

    def test_blocks_moved_by_float_or_footnote_placement_are_matched(self):
        body1 = 'we tentatively place a queen on that square'.split()
        note = 'I do not know what this game is called'.split()
        # (the body around a float is always longer than the float itself)
        body2 = 'and recursively grope for consistent placements of the queens in later rows'.split()
        report = compare(body1 + note + body2, body1 + body2 + note)
        self.assertEqual(report.mismatch_ratio, 0)
        self.assertEqual(report.hunks, [])
        self.assertEqual([m[0] for m in report.moved], [note])

    def test_spacing_only_differences_match(self):
        # The original's text layer drops the space after an "ff" ligature
        # ("cutting offthe tree").
        report = compare(['cutting', 'offthe', 'tree'], ['cutting', 'off', 'the', 'tree'])
        self.assertEqual(report.mismatch_ratio, 0)
        self.assertEqual(report.hunks, [])

    def test_small_caps_that_the_original_extracts_as_capitals_match(self):
        # Roboto small caps (captions, comments) come out uppercase in the
        # original's text layer and lowercase from LuaTeX.
        report = compare(['or', 'NONE', 'if'], ['or', 'None', 'if'])
        self.assertEqual(report.mismatch_ratio, 0)
        # ...but ordinary capitalization errors still count.
        report = compare(['the', 'Queens', 'problem'], ['the', 'queens', 'problem'])
        self.assertAlmostEqual(report.mismatch_ratio, 1 / 3)

    def test_short_coincidental_runs_are_not_moves(self):
        report = compare(['a', 'the', 'b', 'c', 'd', 'e'], ['a', 'b', 'c', 'd', 'e', 'the'])
        self.assertAlmostEqual(report.mismatch_ratio, 1 / 6)


class MathTests(unittest.TestCase):
    def test_symbol_stream_ignores_fonts_and_spacing(self):
        original = [{'text': 'Q', 'cls': 'math'}, {'text': '[', 'cls': 'math'}, {'text': '1..', 'cls': 'math'},
                    {'text': ' n', 'cls': 'math'}, {'text': ']', 'cls': 'math'}, {'text': 'word', 'cls': 'text'},
                    {'text': '〈〈', 'cls': 'math'}, {'text': 'u⟨ARC⟩v', 'cls': 'math'}]
        rebuilt = [{'text': '𝑄[1..𝑛]', 'cls': 'math'}, {'text': '⟨⟨', 'cls': 'math'},
                   {'text': '𝑢→𝑣', 'cls': 'math'}]
        self.assertEqual(math_symbols(original), math_symbols(rebuilt))

    def test_mathdesign_extension_glyphs_and_unicode_math_variants(self):
        ex = 'MathDesign-CH-Regular-Ex'
        original = [{'text': 'X', 'font': ex, 'cls': 'math'}, {'text': 'f[i]', 'font': 'CharterBT-Italic', 'cls': 'math'},
                    {'text': 'W', 'font': ex, 'cls': 'math'}, {'text': 'p', 'font': ex, 'cls': 'math'},
                    {'text': '\x00', 'font': ex, 'cls': 'math'},
                    {'text': 'X\\{x}·i̸=j···=⇒', 'font': 'MathDesign-CH-Regular-Sy', 'cls': 'math'}]
        rebuilt = [{'text': '∑𝑓[𝑖]⋁√⎧⎨⎩', 'font': 'XCharter-Math', 'cls': 'math'},
                   {'text': '𝑋∖{𝑥}⋅𝑖≠𝑗⋯⟹', 'font': 'XCharter-Math', 'cls': 'math'}]
        self.assertEqual(math_symbols(original), math_symbols(rebuilt))


class ResidualTests(unittest.TestCase):
    def test_residual_multiset_difference(self):
        hunks = [(['3', 'If', 'you', 'have', ','], []), ([], ['3', ',']), ([], ['If', 'you', 'have']),
                 (['word'], ['ward'])]
        self.assertEqual(residual_difference(hunks), (['word'], ['ward']))


class StructureTests(unittest.TestCase):
    def test_figure_destinations_of_one_chapter(self):
        class Doc:
            def resolve_names(self):
                return {'figure.2.2': {}, 'figure.2.10': {}, 'figure.12.1': {}, 'section.2.3': {},
                        'figure.1.4': {}}

        self.assertEqual(figure_destinations(Doc(), '02'), ['2.10', '2.2'])
        self.assertEqual(figure_destinations(Doc(), '12'), ['12.1'])
        self.assertEqual(figure_destinations(Doc(), 'preface'), [])

    def test_footnote_names(self):
        links = [{'name': 'Hfootnote.48'}, {'name': 'figure.2.2'}, {'name': 'Hfootnote.49'},
                 {'name': 'Hfootnote.48'}, {'name': None}]
        self.assertEqual(footnote_names(links), ['Hfootnote.48', 'Hfootnote.49'])

    def test_exercise_numbers_after_exercises_heading(self):
        spans = [{'text': '1.', 'cls': 'text', 'bbox': [76, 100, 83, 110]},
                 {'text': 'Exercises', 'cls': 'heading', 'bbox': [72, 200, 127, 210]},
                 {'text': '1.', 'cls': 'text', 'bbox': [76, 300, 83, 310]},
                 {'text': 'in row 2.', 'cls': 'text', 'bbox': [200, 320, 260, 330]},
                 {'text': '2.', 'cls': 'text', 'bbox': [57, 400, 65, 410]},
                 {'text': '3. (a)', 'cls': 'text', 'bbox': [57, 500, 82, 510]}]
        self.assertEqual(exercise_numbers(spans), [1, 2, 3])

    def test_pseudocode_index(self):
        lines = ['AddAllSafeEdges, 262', 'BeAMillionaireAndNeverPayTaxes,', '10',
                 'BellmanFord, 291, 292', 'Borůvka, 262, 272']
        self.assertEqual(pseudocode_index(lines), {
            'AddAllSafeEdges': [262], 'BeAMillionaireAndNeverPayTaxes': [10],
            'BellmanFord': [291, 292], 'Borůvka': [262, 272]})


class AccentTests(unittest.TestCase):
    def test_accents_fold_to_the_original_layer_order(self):
        self.assertEqual(refold_accents('m\u0101tr\u0101v\u1e5btta'), 'm\u0304atr\u0304avr.tta')
        self.assertEqual(refold_accents('Pi\u1e45gala'), 'Pi\u0307ngala')
        self.assertEqual(refold_accents('Chanda\u1e25\u015b\u0101stra'), 'Chandah.\u015b\u0304astra')
        self.assertEqual(refold_accents('plain text'), 'plain text')

    def test_words_splits_the_refolded_dot(self):
        spans = [{'text': 'the ak\u1e63ara count', 'cls': 'text'}]
        self.assertEqual(words(spans), ['the', 'aks', '.', 'ara', 'count'])


if __name__ == '__main__':
    unittest.main()
