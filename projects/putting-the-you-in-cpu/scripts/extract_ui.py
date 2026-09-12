"""Extract upstream UI strings and MDX attributes that the body parser skips."""
from pathlib import Path
import json
import re

root = Path(__file__).resolve().parents[1]
upstream = root / 'upstream'
entries = []

def add(path, source, mode='text'):
    if not source.strip():
        return
    entries.append(dict(path=str(path.relative_to(upstream)), source=source, mode=mode))

for path in sorted((upstream / 'src/content/chapters').glob('*.mdx')):
    text = path.read_text()
    for match in re.finditer(r"\balt=(['\"])(.*?)\1", text):
        add(path, match[2], 'attribute')
    for match in re.finditer(r'^shortname: (.+)$', text, re.M):
        add(path, match[1], 'shortname')

strings = [
'Navigate Between Chapters', 'Chapter Contents', 'Previous:', 'Next:',
'All Chapters', 'Edit on GitHub', 'A project by Hack Club',
'From the beginning&hellip;', '[This Space Intentionally Left Blank]',
'The bottom of every page is padded so readers can maintain a consistent eyeline.',
'Open source with ❤︎ on GitHub', 'Other editions:', 'One-Pager', 'Normal', 'Editions',
'July, 2023',
'Curious exactly what happens when you run a program on your computer? Read this article to learn how multiprocessing works, what system calls really are, how computers manage memory with hardware interrupts, and how Linux loads executables.',
'Curious exactly what happens when you run a program on your computer? Learn how multiprocessing works, what system calls really are, how computers manage memory with hardware interrupts, and how Linux loads executables.',
'Looks like you wound up accessing a memory region &mdash; errr, page &mdash; that doesn&lsquo;t exist.',
'Want to learn more about page faults?', 'Start from the beginning!',
'a rabbit hole into how your computer runs programs.'
]
for path in sorted((upstream/'src').rglob('*')):
    if path.suffix not in ('.astro', '.ts'):
        continue
    text = path.read_text()
    for source in strings:
        if source in text:
            add(path, source)

add(upstream/'src/content/chapters/4-becoming-an-elf-lord.mdx', "(Thank you to <a href='https://ncase.me/' target='_blank'>Nicky Case</a> for the adorable drawing.)", 'html')
for path in sorted((upstream/'src/content/chapters').glob('*.mdx')):
    if "name='Shell session'" in path.read_text():
        add(path, 'Shell session', 'text')
add(upstream/'src/pages/404.astro', 'Not Found', 'text')

out = root / 'ui'
out.mkdir(exist_ok=True)
for i, entry in enumerate(entries):
    entry['file'] = f'{i:03}.md'
    (out/entry['file']).write_text(entry['source']+'\n')
(root/'ui.json').write_text(json.dumps(entries, ensure_ascii=False, indent=2)+'\n')
print(f'Extracted {len(entries)} strings')
