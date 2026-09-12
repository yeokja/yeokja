#!/usr/bin/env python3
"""Localize rendered Markdeep labels using translations produced by Yeokja.

The English translation input is ui/markdeep.md. The build imports load_labels
and language_script to install a presentation callback before the renderer.
The renderer keeps its original input grammar so protected caption IDs resolve.
"""

import argparse
import json
from pathlib import Path
import re

ROOT = Path(__file__).resolve().parents[1]
KEYS = ('contents', 'chapter', 'diagram', 'figure', 'listing', 'sec', 'section', 'subsection', 'table')
MARKER = re.compile(r'<!-- markdeep:([a-z]+) -->\s*\n(.*?)(?=\n<!-- markdeep:|\Z)', re.S)


def parse_labels(source):
    matches = list(MARKER.finditer(source))
    keys = tuple(match[1] for match in matches)
    if keys != KEYS:
        raise ValueError(f'Expected ordered Markdeep label keys {KEYS}; got {keys}')
    if source[:matches[0].start()].strip():
        raise ValueError('Unexpected text before first Markdeep label')
    keyword = {}
    for match in matches:
        value = match[2].strip()
        if not re.search('[가-힣]', value):
            raise ValueError(f'Markdeep label {match[1]} has no Korean translation')
        if '\n' in value or '<!-- markdeep:' in value:
            raise ValueError(f'Markdeep label {match[1]} must be a single translated line')
        keyword[match[1]] = value
    # These are punctuation entities used by the pinned renderer, not prose.
    keyword.update({'&ldquo;': '&ldquo;', '&rdquo;': '&rdquo;'})
    return {'name': 'Korean', 'keyword': keyword}


def load_labels(path=ROOT / 'ko-ui/markdeep.md'):
    return parse_labels(Path(path).read_text(encoding='utf-8'))


def language_script(mapping):
    """Translate generated DOM text after rendering; never change input grammar."""
    payload = json.dumps(mapping['keyword'], ensure_ascii=False, separators=(',', ':'))
    payload = payload.replace('<', '\\u003c').replace('\u2028', '\\u2028').replace('\u2029', '\\u2029')
    # Markdeep's lang.keyword also drives its caption and cross-reference parser.
    # Setting it to Korean before parsing breaks English identifiers intentionally
    # protected by Yeokja. Only generated display text is changed in onLoad.
    return r"""(function(){
const labels=PAYLOAD;
const options=window.markdeepOptions=window.markdeepOptions||{};
const previous=options.onLoad;
function localize(element){
    for(const child of element.childNodes){
        if(child.nodeType===3){
            if(!child.nodeValue.trim()) continue;
            child.nodeValue=child.nodeValue.replace(
                /^(\s*)(Contents|Chapter|Diagram|Figure|Image|Listing|Sec|Section|Subsection|Table)(?=[\s\d.:]|$)/i,
                function(all,space,word){return space+labels[word.toLowerCase()==='image'?'figure':word.toLowerCase()];});
            return true;
        }
        if(child.nodeType===1 && localize(child)) return true;
    }
    return false;
}
options.onLoad=function(){
    document.querySelectorAll([
        '.tocHeader', '.mediumTOC > center > b',
        '.imagecaption > b:first-child', '.listingcaption > b:first-child',
        '.tablecaption > b:first-child', '.imagecaption .num',
        'a[href^="#figure_"]', 'a[href^="#image_"]', 'a[href^="#listing_"]',
        'a[href^="#table_"]', 'a[href^="#diagram_"]', 'a[href^="#chapter_"]',
        'a[href^="#section_"]', 'a[href^="#subsection_"]'
    ].join(',')).forEach(localize);
    if(typeof previous==='function') return previous.apply(this,arguments);
};
})();""".replace('PAYLOAD', payload)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--translated', type=Path, default=ROOT / 'ko-ui/markdeep.md')
    parser.add_argument('--output', type=Path, help='Write JavaScript here (otherwise print it)')
    args = parser.parse_args()
    script = language_script(load_labels(args.translated)) + '\n'
    if args.output:
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(script, encoding='utf-8')
    else:
        print(script, end='')


if __name__ == '__main__':
    main()
