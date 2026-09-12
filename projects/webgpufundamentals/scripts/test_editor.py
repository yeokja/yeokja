import json
import subprocess
import unittest
from patch_editor import OLD_PREFIX, patch_editor

class EditorTests(unittest.TestCase):
    def test_helper_root_follows_the_editor_asset_not_the_example_depth(self):
        for example in ('webgpu/example.html', 'webgpu/resources/example.html'):
            text = "(function(){'use strict';function getSourceBlob(){const dname=" + json.dumps('https://site.test/prefix/' + example.rsplit('/', 1)[0]) + ';' + OLD_PREFIX + "return prefix;}globalThis.actual=getSourceBlob();globalThis.monacoPath=`${window.location.origin}/monaco-editor/min/vs`;})();"
            patched = patch_editor(text)
            script = "globalThis.document={currentScript:{src:'https://site.test/prefix/webgpu/resources/editor.js'}};" + patched + "console.log(actual);console.log(monacoPath);"
            result = subprocess.run(['node', '-e', script], check=True, capture_output=True, text=True)
            self.assertEqual(result.stdout.strip(), 'https://site.test/prefix/webgpu\nhttps://site.test/prefix/monaco-editor/min/vs')
            self.assertEqual(patched, patch_editor(patched))

if __name__ == '__main__':
    unittest.main()
