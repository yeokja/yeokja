import subprocess
import unittest

import prepare_markdeep


class PrepareMarkdeepTests(unittest.TestCase):
    def translated_source(self):
        return '\n\n'.join(f'<!-- markdeep:{key} -->\n\n번역{i}'
                             for i, key in enumerate(prepare_markdeep.KEYS)) + '\n'

    def test_mapping_retains_keys_and_quotes(self):
        mapping = prepare_markdeep.parse_labels(self.translated_source())
        self.assertEqual(mapping['name'], 'Korean')
        self.assertEqual(mapping['keyword']['contents'], '번역0')
        self.assertEqual(mapping['keyword']['&ldquo;'], '&ldquo;')
        self.assertEqual(mapping['keyword']['&rdquo;'], '&rdquo;')
        self.assertEqual(len(mapping['keyword']), len(prepare_markdeep.KEYS) + 2)

    def test_missing_and_untranslated_labels_fail(self):
        with self.assertRaisesRegex(ValueError, 'keys'):
            prepare_markdeep.parse_labels(self.translated_source().replace('markdeep:table', 'markdeep:extra'))
        with self.assertRaisesRegex(ValueError, 'Korean'):
            prepare_markdeep.parse_labels(self.translated_source().replace('번역0', 'Contents'))

    def test_script_cannot_be_closed_by_translated_label(self):
        mapping = prepare_markdeep.parse_labels(self.translated_source().replace('번역0', '번역</script>'))
        script = prepare_markdeep.language_script(mapping)
        self.assertNotIn('</script>', script)
        self.assertIn('options.onLoad=', script)
        self.assertNotIn('.lang=', script)


    def test_script_localizes_presentation_after_render_without_changing_grammar(self):
        mapping = prepare_markdeep.parse_labels(self.translated_source())
        script = prepare_markdeep.language_script(mapping)
        fixture = r"""
const assert = require('node:assert/strict');
const node = value => ({nodeType:3, nodeValue:value});
const toc = {childNodes:[node('Contents')]};
const caption = {childNodes:[node('Figure\u00a01:')]};
const link = {childNodes:[node('listing\u00a021')]};
const image = {childNodes:[node('Image 3:')]};
const nested = {childNodes:[{nodeType:1, childNodes:[node('Table 4:')]}]};
let called = 0;
const previous = function(){ called++; assert.equal(caption.childNodes[0].nodeValue, '번역3\u00a01:'); };
global.window = {markdeepOptions:{lang:'en', onLoad:previous}};
global.document = {querySelectorAll:()=>[toc,caption,link,image,nested]};
"""
        assertions = r"""
assert.equal(window.markdeepOptions.lang, 'en');
assert.equal(caption.childNodes[0].nodeValue, 'Figure\u00a01:');
window.markdeepOptions.onLoad();
assert.equal(toc.childNodes[0].nodeValue, '번역0');
assert.equal(caption.childNodes[0].nodeValue, '번역3\u00a01:');
assert.equal(link.childNodes[0].nodeValue, '번역4\u00a021');
assert.equal(image.childNodes[0].nodeValue, '번역3 3:');
assert.equal(nested.childNodes[0].childNodes[0].nodeValue, '번역8 4:');
assert.equal(called, 1);
window.markdeepOptions.onLoad();
assert.equal(called, 2);
assert.equal(caption.childNodes[0].nodeValue, '번역3\u00a01:');
"""
        result = subprocess.run(['node', '-e', fixture + script + assertions], capture_output=True, text=True)
        self.assertEqual(result.returncode, 0, result.stderr)


if __name__ == '__main__':
    unittest.main()
