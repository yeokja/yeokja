"""Adapt a copied upstream tree for a Korean GitHub Pages subdirectory."""
from pathlib import Path
import re

root = Path.cwd()
base = '/putting-the-you-in-cpu'
p = root/'astro.config.mjs'
s = p.read_text().replace("import vercel from '@astrojs/vercel'", '')
s = s.replace("site: 'https://cpu.land/'", "site: 'https://yeokja.moreal.dev',\n\tbase: '/putting-the-you-in-cpu',\n\toutDir: './site'")
s = s.replace("trailingSlash: 'never'", "trailingSlash: 'always'").replace('\tadapter: vercel(),\n', '')
p.write_text(s)
for p in (root/'src').rglob('*.astro'):
    s = p.read_text().replace("lang='en'", "lang='ko'").replace("content='en_US'", "content='ko_KR'")
    s = re.sub(r"\s*<script defer data-domain='cpu.land'[^>]+/>", '', s)
    s = s.replace("new URL('/banner.png', Astro.site)", "new URL('/putting-the-you-in-cpu/banner.png', Astro.site)")
    # Relative chapter URLs break when Pages redirects /chapter to /chapter/.
    for expr in ['chapter.id', 'prevChapter.id', 'nextChapter.id', 'otherChapter.id']:
        s = s.replace('href={'+expr+'}', 'href={"'+base+'/" + '+expr+' + "/"}')
        s = s.replace("? '/' : "+expr+'}', '? "'+base+'/" : "'+base+'/" + '+expr+' + "/"}')
    s = s.replace("href='the-basics'", "href='/putting-the-you-in-cpu/the-basics/'")
    s = s.replace('Continue to Chapter 1:', '1장으로 계속:').replace('Continue to Chapter {nextChapter.data.chapter}:', '{nextChapter.data.chapter}장으로 계속:')
    s = s.replace('Chapter {chapter!.data.chapter}', '{chapter!.data.chapter}장').replace('Chapter {chapter.data.chapter}:', '{chapter.data.chapter}장:')
    s = s.replace('Ch. {chapter.data.chapter}', '{chapter.data.chapter}장')
    s = s.replace('`Chapter ${prevChapter.data.chapter}`', '`${prevChapter.data.chapter}장`').replace('`Chapter ${nextChapter.data.chapter}`', '`${nextChapter.data.chapter}장`')
    s = s.replace("'Intro'", "'소개'").replace('>Intro</a>', '>소개</a>')
    s = s.replace('Part of <a', '다음 글의 일부입니다: <a')
    s = s.replace('>PDF</a>', '>PDF (영어 원문)</a>')
    s = s.replace('\n\t\t\t\tBy\n', '\n\t\t\t\t글쓴이\n')
    s = s.replace('const newHtml = dom.serialize()', 'const newHtml = dom.window.document.body.innerHTML')
    p.write_text(s)
# The upstream robots policy and analytics proxy belong to cpu.land.
(root/'public/robots.txt').write_text('User-agent: *\nAllow: /\n')
