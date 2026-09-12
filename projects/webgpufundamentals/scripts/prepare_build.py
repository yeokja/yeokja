"""Make upstream asynchronous Grunt tasks propagate their build errors."""
from pathlib import Path
p = Path('Gruntfile.js')
s = p.read_text()
old = 'buildStuff(buildSettings).finally(finish);'
assert s.count(old) == 1
s = s.replace(old, 'buildStuff(buildSettings).then(() => finish(), error => { grunt.log.error(error.stack || JSON.stringify(error)); finish(false); });')
s = s.replace("lessonGrep: 'webgpu*.md'", "lessonGrep: 'web*.md'")
p.write_text(s)

# Only English (for original links) and Korean are published by this project.
# This runs in the disposable build copy, never in the upstream submodule.
import shutil
for folder in Path('webgpu/lessons').iterdir():
    if folder.is_dir() and folder.name != 'ko' and (folder / 'langinfo.hanson').is_file():
        shutil.rmtree(folder)

# The upstream camera lesson lacks one closing fence, swallowing prose as JS.
import re
paragraph_re = re.compile(r'(<a id="a-aim-fs"></a> .*?)(?=\n\n```js\n-  const numFs)', re.S)
for language in ('en', 'ko'):
    folder = Path('webgpu/lessons') / ('ko' if language == 'ko' else '')
    camera = folder / 'webgpu-cameras.md'
    content = camera.read_text()
    match = paragraph_re.search(content)
    if not match:
        raise ValueError('camera aim paragraph changed; review the fence repair')
    paragraph = match[1] if language == 'en' else (folder / '_camera-aim-fs.md').read_text().strip()
    content = content[:match.start()] + '```\n\n' + paragraph + content[match.end():]
    camera.write_text(content)
