from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import unittest

SCRIPT = Path(__file__).with_name('metadata.py')

class MetadataTests(unittest.TestCase):
    def test_reassembles_keys_and_includes_without_changing_body_or_code(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            (root / 'scripts').mkdir()
            shutil.copyfile(SCRIPT, root / 'scripts/metadata.py')
            upstream = root / 'upstream/webgpu/lessons'
            upstream.mkdir(parents=True)
            (upstream / 'index.md').write_text('Title: Title\nDescription: Description\nTOC: Contents\n\nEnglish.\n')
            (upstream / 'langinfo.hanson').write_text("{needNewWebGPU: 'Update your browser.'}")
            (upstream / 'ko').mkdir()
            (upstream / 'ko/langinfo.hanson').write_text("{language: '한국어'}")
            (upstream / 'tool.md').write_text('Title: Tool\nLink: webgpu/lessons/resources/tool.html\n')
            command = [sys.executable, str(root / 'scripts/metadata.py')]
            subprocess.run(command + ['extract'], check=True)
            self.assertEqual((root / 'metadata/index.md').read_text(), 'Title\n\n---\n\nDescription\n\n---\n\nContents\n')
            (root / 'ko-metadata').mkdir()
            (root / 'ko-metadata/index.md').write_text('제목\n\n---\n\n설명\n\n---\n\n목차\n')
            (root / 'ko-metadata/_need-new-webgpu.md').write_text('브라우저를 업데이트하십시오.\n')
            (root / 'ko-metadata/tool.md').write_text('도구\n')
            ko = root / 'ko/webgpu/lessons/ko'
            ko.mkdir(parents=True)
            (ko / 'tool.md').write_text('제목: 도구 링크: webgpu/lessons/resources/tool.html\n')
            body = '본문입니다.\n\n```js\nconst x = 1;\n```\n\n{{{include "webgpu/lessons/toc.html"}}}\n'
            (ko / 'index.md').write_text('모델이 바꾼 헤더\n\n' + body)
            subprocess.run(command + ['apply'], check=True)
            self.assertEqual((ko / 'tool.md').read_text(), 'Title: 도구\nLink: webgpu/lessons/resources/tool.html\n\n')
            self.assertIn('브라우저를 업데이트하십시오.', (ko / 'langinfo.hanson').read_text())
            expected = 'Title: 제목\nDescription: 설명\nTOC: 목차\n\n' + body.replace('lessons/toc.html', 'lessons/ko/toc.html')
            self.assertEqual((ko / 'index.md').read_text(), expected)
            subprocess.run(command + ['apply'], check=True)
            self.assertEqual((ko / 'index.md').read_text(), expected)

if __name__ == '__main__':
    unittest.main()
