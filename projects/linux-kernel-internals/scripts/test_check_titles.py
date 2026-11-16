import json
from pathlib import Path
import tempfile
import unittest

import check_titles
from check_titles import (Site, check, destination, final_consonant, fix, heading_ids, headings, links, norm,
                          replace_link_text, slugify, unwrap, write_translation)

CONFIG = '''site_name: X
nav:
  - Home: index.md
  - Memory:
    - Getting Started: mm/README.md
    - Page Allocator: mm/page-allocator.md
    - Workqueues: mm/workqueues.md
    - War Stories: mm/war-stories.md
  - Net:
    - War Stories: net/war-stories.md
    - Contributing: contributing.md
'''

GLOSSARY = '[terms.cgroup]\ntranslation = "cgroup"\n[terms.reclaim]\ntranslation = "회수"\n'


def state(pairs):
    segments = [{'id': f'section:0/block:{index}/seg:0', 'source': source, 'source_hash': index,
                 'context_hash': 1, 'translation': translation, 'glossary_snapshot': {},
                 'translated_at': '2026-01-01T00:00:00Z', 'issues': []}
                for index, (source, translation) in enumerate(pairs)]
    return json.dumps({'version': 1, 'source_hash': 7, 'segments': segments}, ensure_ascii=False, indent=2)


class Project:
    """A project tree in a temporary directory: pages are {name: [(source, translation)]},
    the first pair of a page being its H1 and a pair whose source starts with `## ` a heading."""

    def __init__(self, test, pages, nav):
        self.directory = tempfile.TemporaryDirectory()
        test.addCleanup(self.directory.cleanup)
        self.root = Path(self.directory.name)
        (self.root / 'upstream/docs').mkdir(parents=True)
        (self.root / 'upstream/mkdocs.yml').write_text(CONFIG)
        (self.root / 'glossary.toml').write_text(GLOSSARY)
        for name, pairs in pages.items():
            source = self.root / 'upstream/docs' / name
            source.parent.mkdir(parents=True, exist_ok=True)
            lines = ['# ' + pairs[0][0], '']
            stripped = [pairs[0]]
            for text, translation in pairs[1:]:
                lines += [text, '']
                stripped.append((text[3:], translation) if text.startswith('## ') else (text, translation))
            source.write_text('\n'.join(lines))
            self.write(f'state/upstream/docs/{name}.yeokja.json', state(stripped))
        self.write('state/nav/nav.md.yeokja.json', state(nav))

    def write(self, name, text):
        path = self.root / name
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(text)

    def check(self):
        violations, stats = check(Site(self.root), self.root)
        return [(violation.rule, violation.message) for violation in violations], stats

    def translation(self, name, index):
        data = json.loads((self.root / f'state/upstream/docs/{name}.yeokja.json').read_text())
        return data['segments'][index]['translation']


NAV = [('Home', '홈'), ('Memory', '메모리'), ('Getting Started', '시작하기'), ('Page Allocator', '페이지 할당자'),
       ('Workqueues', 'Workqueue'), ('War Stories', '실전 사례'), ('Net', '네트워크'), ('War Stories', '실전 사례'),
       ('Contributing', '기여하기')]


def pages(**changes):
    site = {
        'index.md': [('Home', '홈')],
        'mm/README.md': [('Memory Management', '메모리 관리')],
        'mm/page-allocator.md': [('Page Allocator', '페이지 할당자'), ('## Free Lists', '프리 리스트')],
        'mm/workqueues.md': [('Workqueues', 'Workqueue')],
        'mm/war-stories.md': [('Memory War Stories', '메모리 실전 사례')],
        'net/war-stories.md': [('Network War Stories', '네트워크 실전 사례')],
    }
    site.update(changes)
    return site


class MarkdownTests(unittest.TestCase):
    def test_links_skip_code_images_and_escapes(self):
        text = r'`[a](b)` ![img](i.png) \[no\](x) [one](a.md) and [t[0] `]`](b.md "T")'
        self.assertEqual([(text[start:end], dest) for start, end, dest in links(text)],
                         [('one', 'a.md'), ('t[0] `]`', 'b.md "T"')])

    def test_a_link_around_an_image_is_one_link(self):
        text = '[![logo](l.png) Home](index.md)'
        self.assertEqual([(text[start:end], dest) for start, end, dest in links(text)],
                         [('![logo](l.png) Home', 'index.md')])

    def test_destination_keeps_internal_pages_only(self):
        self.assertEqual(destination('../mm/slab.md#free "t"'), ('../mm/slab.md', 'free'))
        self.assertEqual(destination('#free'), ('', 'free'))
        self.assertIsNone(destination('https://lwn.net/Articles/1/'))
        self.assertIsNone(destination('mailto:a@b.c'))
        self.assertIsNone(destination('assets/tux.svg'))

    def test_unwrap_and_norm(self):
        self.assertEqual(unwrap(' **`kmalloc()`** '), ('**`', 'kmalloc()', '`**'))
        self.assertEqual(unwrap('`a` and `b`'), ('', '`a` and `b`', ''))
        self.assertEqual(norm('**Page  Allocator**'), norm('page allocator'))
        self.assertEqual(norm(r'Slab \(SLUB\)'), 'slab (slub)')

    def test_headings_skip_fences_and_ids_follow_python_markdown(self):
        text = '# A `b()` Title\n\n```sh\n# not a heading\n```\n\n## Why?\n\n## Why?\n\n### 한글\n'
        found = headings(text)
        self.assertEqual(found, [(1, 'A `b()` Title'), (2, 'Why?'), (2, 'Why?'), (3, '한글')])
        self.assertEqual(heading_ids(found), ['a-b-title', 'why', 'why_1', '_1'])
        self.assertEqual(slugify('CPU cgroup: v1 vs v2'), 'cpu-cgroup-v1-vs-v2')


class KoreanTests(unittest.TestCase):
    def test_final_consonant(self):
        self.assertEqual(final_consonant('페이지 할당자'), 0)
        self.assertTrue(final_consonant('커널 주소 공간'))
        self.assertEqual(final_consonant('패킷의 일생(수신 경로)'), final_consonant('일생'))
        self.assertEqual(final_consonant('`커널`'), final_consonant('커널'))
        self.assertTrue(final_consonant('cgroup v1'))
        self.assertEqual(final_consonant('cgroup v2'), 0)
        self.assertIsNone(final_consonant('Page Allocator'))

    def test_a_particle_after_the_link_follows_the_new_text(self):
        self.assertEqual(replace_link_text('[Kernel Space](k.md)를 보십시오.', 1, 13, '커널 주소 공간'),
                         ('[커널 주소 공간](k.md)을 보십시오.', 'particle 를 → 을'))
        self.assertEqual(replace_link_text('[A](a.md)과 [B](b.md)', 1, 2, '페이지')[0], '[페이지](a.md)와 [B](b.md)')
        self.assertEqual(replace_link_text('[A](a.md)으로 갑니다', 1, 2, '커널')[0], '[커널](a.md)로 갑니다')
        self.assertEqual(replace_link_text('[A](a.md)로는 부족', 1, 2, '공간')[0], '[공간](a.md)으로는 부족')

    def test_a_word_that_only_starts_like_a_particle_stays(self):
        self.assertEqual(replace_link_text('[A](a.md)이후에', 1, 2, '페이지')[0], '[페이지](a.md)이후에')
        self.assertEqual(replace_link_text('[A](a.md) 를', 1, 2, '공간')[0], '[공간](a.md) 를')

    def test_a_particle_stays_when_the_new_text_ends_in_latin(self):
        text, note = replace_link_text('[가상 메모리](a.md)를 봅니다', 1, 7, 'Direct I/O')
        self.assertEqual(text, '[Direct I/O](a.md)를 봅니다')
        self.assertIn('kept', note)


class RuleTests(unittest.TestCase):
    def test_a_consistent_site_passes(self):
        project = Project(self, pages(**{'net/war-stories.md': [
            ('Network War Stories', '네트워크 실전 사례'),
            ('See [Page Allocator](../mm/page-allocator.md), its [Free Lists](../mm/page-allocator.md#free-lists), '
             '[LWN: The Page Allocator](https://lwn.net/Articles/1/) and [page-allocator.md](../mm/page-allocator.md).',
             '[페이지 할당자](../mm/page-allocator.md), 그 [프리 리스트](../mm/page-allocator.md#free-lists), '
             '[LWN: The Page Allocator](https://lwn.net/Articles/1/), [page-allocator.md](../mm/page-allocator.md)를 '
             '참고하십시오.')]}), NAV)
        violations, stats = project.check()
        self.assertEqual(violations, [])
        self.assertEqual(stats['title links'], 2)
        self.assertEqual(stats['other internal links'], 1)

    def test_link_text_that_is_a_title_follows_the_h1_the_nav_label_or_the_heading(self):
        project = Project(self, pages(**{'index.md': [
            ('Home', '홈'),
            ('[Page Allocator](mm/page-allocator.md)', '[Page Allocator](mm/page-allocator.md)'),
            ('**[Memory Management](mm/README.md)** and [Getting Started](mm/README.md)',
             '**[메모리 관리 개요](mm/README.md)**와 [시작](mm/README.md)'),
            ('[Free Lists](mm/page-allocator.md#free-lists)', '[자유 목록](mm/page-allocator.md#free-lists)')]}), NAV)
        violations, _ = project.check()
        self.assertEqual([rule for rule, _ in violations], ['link'] * 4)
        self.assertIn("should read '페이지 할당자'", violations[0][1])
        self.assertIn("should read '메모리 관리'", violations[1][1])
        self.assertIn("should read '시작하기'", violations[2][1])
        self.assertIn("should read '프리 리스트'", violations[3][1])

    def test_a_title_in_another_letter_case_may_keep_its_case(self):
        mention = ('Use [workqueues](mm/workqueues.md) or [Workqueues](mm/workqueues.md).',
                   '[workqueue](mm/workqueues.md)나 [workqueue](mm/workqueues.md)를 쓰십시오.')
        project = Project(self, pages(**{'index.md': [('Home', '홈'), mention]}), NAV)
        violations, _ = project.check()
        self.assertEqual(len(violations), 1)
        self.assertIn("should read 'Workqueue'", violations[0][1])

    def test_nav_label_that_is_the_h1_follows_it_and_a_repeated_label_has_one_translation(self):
        nav = list(NAV)
        nav[3] = ('Page Allocator', '페이지 얼로케이터')
        nav[7] = ('War Stories', '사례 모음')
        violations, _ = Project(self, pages(), nav).check()
        self.assertEqual([rule for rule, _ in violations], ['nav', 'nav'])
        self.assertIn("should be '페이지 할당자' (the H1 of mm/page-allocator.md)", violations[0][1])
        self.assertIn("should be '실전 사례' (the same label elsewhere in the nav)", violations[1][1])

    def test_the_same_source_h1_has_one_translation(self):
        project = Project(self, pages(**{'mm/war-stories.md': [('War Stories', '실전 사례')],
                                   'net/war-stories.md': [('War Stories', '전쟁 이야기')]}), NAV)
        self.assertEqual([rule for rule, _ in project.check()[0]], ['h1', 'h1', 'nav'])

    def test_an_alias_has_one_translation_and_an_untranslated_one_follows_it(self):
        project = Project(self, pages(**{'index.md': [
            ('Home', '홈'),
            ('[Memory Overview](mm/README.md)', '[메모리 개요](mm/README.md)'),
            ('[Memory Overview](mm/README.md)', '[메모리 개요](mm/README.md)'),
            ('[Memory Overview](mm/README.md)', '[메모리 훑어보기](mm/README.md)'),
            ('[Memory Overview](mm/README.md)', '[Memory Overview](mm/README.md)'),
            ('see [the allocator page](mm/page-allocator.md)', '[할당자 쪽](mm/page-allocator.md) 참고'),
            ('see [the allocator page](mm/page-allocator.md)', '[할당자 페이지](mm/page-allocator.md) 참고')]}), NAV)
        violations, _ = project.check()
        self.assertEqual([rule for rule, _ in violations], ['alias', 'alias'])
        self.assertTrue(all("should read '메모리 개요'" in message for _, message in violations))

    def test_a_link_text_left_in_the_source_language(self):
        project = Project(self, pages(**{'index.md': [
            ('Home', '홈'),
            ('[Tuning the allocator](mm/page-allocator.md)', '[Tuning the allocator](mm/page-allocator.md)'),
            ('[cgroup](mm/README.md), [`alloc_pages()`](mm/page-allocator.md), [mm/](mm/README.md), '
             '[NUMA](mm/README.md), [mmap](mm/README.md)',
             '[cgroup](mm/README.md), [`alloc_pages()`](mm/page-allocator.md), [mm/](mm/README.md), '
             '[NUMA](mm/README.md), [mmap](mm/README.md)')],
            'mm/war-stories.md': [('Memory War Stories', 'mmap 실전 사례')]}), NAV)
        violations, _ = project.check()
        self.assertEqual(violations, [('latin', '[Tuning the allocator](mm/page-allocator.md) is left in the '
                                                'source language')])

    def test_links_to_pages_outside_the_translation_are_not_looked_at(self):
        project = Project(self, pages(**{'index.md': [
            ('Home', '홈'), ('[Contributing](contributing.md)', '[Contributing guide](contributing.md)')]}), NAV)
        violations, stats = project.check()
        self.assertEqual(violations, [])
        self.assertEqual(stats['links to untranslated pages'], 1)

    def test_a_dropped_link_is_reported(self):
        project = Project(self, pages(**{'index.md': [
            ('Home', '홈'), ('[Page Allocator](mm/page-allocator.md)', '페이지 할당자')]}), NAV)
        self.assertEqual([rule for rule, _ in project.check()[0]], ['link'])

    def test_nav_state_that_is_not_the_upstream_nav_is_an_error(self):
        project = Project(self, pages(), NAV[:-1])
        with self.assertRaises(ValueError):
            Site(project.root)


class FixTests(unittest.TestCase):
    def test_fix_rewrites_what_a_replacement_decides_and_nothing_else(self):
        nav = list(NAV)
        nav[3] = ('Page Allocator', '페이지 얼로케이터')
        project = Project(self, pages(**{'index.md': [
            ('Home', '홈'),
            ('See [Page Allocator](mm/page-allocator.md) and **[Page Allocator](mm/page-allocator.md)**.',
             '[Page Allocator](mm/page-allocator.md)와 **[페이지 얼로케이터](mm/page-allocator.md)**를 보십시오.'),
            ('[The Kernel](mm/README.md) left', '[The Kernel](mm/README.md) 남음')]}), nav)
        index = project.root / 'state/upstream/docs/index.md.yeokja.json'
        before = index.read_text()
        log = []
        site = Site(project.root)
        self.assertEqual(fix(site, project.root, log=log.append), 2)
        self.assertEqual(project.translation('index.md', 1),
                         '[페이지 할당자](mm/page-allocator.md)와 **[페이지 할당자](mm/page-allocator.md)**를 보십시오.')
        after = index.read_text()
        old = json.dumps('[Page Allocator](mm/page-allocator.md)와 **[페이지 얼로케이터](mm/page-allocator.md)**를 보십시오.',
                         ensure_ascii=False)
        new = json.dumps(project.translation('index.md', 1), ensure_ascii=False)
        self.assertEqual(after, before.replace(old, new))
        self.assertEqual(len(log), 3)
        self.assertEqual(project.check()[0], [('latin', '[The Kernel](mm/README.md) is left in the source language')])

    def test_write_translation_touches_one_segment(self):
        text = state([('A', '가'), ('B', '가')])
        changed = write_translation(text, 'section:0/block:1/seg:0', '가', '나 "따옴표"')
        segments = json.loads(changed)['segments']
        self.assertEqual([segment['translation'] for segment in segments], ['가', '나 "따옴표"'])
        self.assertEqual(changed.replace('"나 \\"따옴표\\""', '"가"'), text)

    def test_main_reads_the_project_the_script_is_in(self):
        self.assertEqual(check_titles.ROOT, Path(check_titles.__file__).resolve().parent.parent)


if __name__ == '__main__':
    unittest.main()
