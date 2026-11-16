"""Keep the name of a page the same wherever the translation writes it.

A page of this site is named in three places that are translated apart from
each other: its own H1, its label in the nav, and the text of the links other
pages point at it with. The rule is that a title the source writes as the same
string is the same string in the translation, and the page's translated H1 is
the reference:

  h1     pages whose source H1 is the same string have the same translated H1.
  nav    a nav label that is its page's source H1 is the page's translated H1.
         Any other label follows the pages whose H1 is that string when they
         agree on one translation, and otherwise the same source label has
         one translation everywhere (the most common one, the first on a tie).
  link   a link to a page of this site (a relative `.md` target, or a bare
         `#anchor`) whose source text is the heading the anchor names, the
         target page's H1, or one of its nav labels - looked up in that
         order - has the translation of that same title as its text. Where
         the source writes the title in another letter case (a lowercase
         mention inside a sentence) the translation may differ in case too.
  alias  another internal link text, one that is no title of its target but
         starts with a capital letter or a digit, has one translation for
         all the links that give the same target the same source text (the
         most common one, the first on a tie).
  latin  an internal link text is not left in the source language. A text
         the translation copied unchanged is accepted when it is the
         target's file or directory name, when it occurs in the target's
         translated H1 or nav label, or when every word of it is inline
         code, part of an identifier or path, an acronym, a term the term
         base keeps in the source language, or a lowercase name some
         translated title keeps.

Strings are compared after trimming, dropping an emphasis or code span that
wraps the whole string, removing backslash escapes, collapsing whitespace and
folding case; the wrapper the source link text has is kept around the
translated title. Links to anything outside the translation (external URLs,
the contributing page that stays in English) are not looked at.

Everything is read from the pinned upstream (headings, nav) and from state/;
nothing is built and no provider is called.

usage: check_titles.py            list the violations; exit 1 if there are any
       check_titles.py --fix      rewrite the translations in state/ that a
                                  replacement decides (nav, link, alias), then
                                  list what is left
       check_titles.py --stats    also print how many titles were compared

`--fix` only replaces a nav label or the text of a link, and adjusts a Korean
particle that directly follows the link to the new final syllable; every other
byte of the state file stays. An `h1` or `latin` violation needs a
translation: retranslate the segment or edit its translation. After a fix,
`yeokja translate` reassembles ko/ from state without calling the provider.
"""
from collections import Counter, defaultdict
import json
from pathlib import Path
import posixpath
import re
import sys
import unicodedata

from sources import parse_nav

ROOT = Path(__file__).resolve().parent.parent
DOCS = 'upstream/docs'
NAV_STATE = 'state/nav/nav.md.yeokja.json'
SUFFIX = '.yeokja.json'

FENCE = re.compile(r'^\s*(`{3,}|~{3,})')
ATX = re.compile(r'^(#{1,6})[ \t]+(.*?)[ \t]*#*[ \t]*$')
NUMBERED = re.compile(r'^(\d+\.[ \t]+)(.*)$')
SCHEME = re.compile(r'^(?:[A-Za-z][A-Za-z0-9+.-]*:|//)')
WRAPPERS = ('**', '__', '*', '_', '`')
WORD = re.compile(r"[A-Za-z][A-Za-z'’-]*")
# Particle pairs as (after a final consonant, after a vowel).
PARTICLES = (('은', '는'), ('이', '가'), ('을', '를'), ('과', '와'), ('으로', '로'))
PARTICLE = re.compile(r'^(으로|은|는|이|가|을|를|과|와|로)(?=$|[\s,.;:!?)\]]|[는도만서써]|부터)')
DIGIT_FINAL = {'0': 21, '1': 8, '3': 16, '6': 1, '7': 8, '8': 8}


# ---------------------------------------------------------------- Markdown

def headings(text):
    """The ATX headings of a page as [(level, inline source)]."""
    lines = text.split('\n')
    start = 0
    if lines and lines[0] == '---' and '---' in lines[1:]:
        start = lines.index('---', 1) + 1
    out = []
    fence = None
    for line in lines[start:]:
        mark = FENCE.match(line)
        if mark:
            if fence is None:
                fence = mark[1]
            elif mark[1][0] == fence[0] and len(mark[1]) >= len(fence):
                fence = None
            continue
        if fence is None:
            match = ATX.match(line)
            if match:
                out.append((len(match[1]), match[2]))
    return out


def links(text):
    """Inline links of a Markdown string as [(start, end, dest)]: text[start:end]
    is the link text. Images and code spans are skipped; a link inside a link
    text is not reported apart from the outer one."""
    out = []
    stack = []
    i = 0
    n = len(text)
    while i < n:
        char = text[i]
        if char == '\\':
            i += 2
            continue
        if char == '`':
            run = len(text) - len(text[i:].lstrip('`')) - i
            close = i + run
            while True:
                close = text.find('`' * run, close)
                if close == -1 or text[close:close + run + 1] != '`' * (run + 1):
                    break
                close += len(text[close:]) - len(text[close:].lstrip('`'))
            i = i + run if close == -1 else close + run
            continue
        if char == '[':
            stack.append((i, i > 0 and text[i - 1] == '!'))
        elif char == ']' and stack:
            start, image = stack.pop()
            if text[i + 1:i + 2] == '(':
                depth = 0
                j = i + 1
                while j < n:
                    if text[j] == '\\':
                        j += 2
                        continue
                    if text[j] == '(':
                        depth += 1
                    elif text[j] == ')':
                        depth -= 1
                        if depth == 0:
                            break
                    j += 1
                if j < n:
                    if not image and not any(not img for _, img in stack):
                        out.append((start + 1, i, text[i + 2:j]))
                    if not image:
                        # Markdown has no link in a link: an outer bracket
                        # that follows is plain text.
                        stack = [item for item in stack if item[1]]
                    i = j + 1
                    continue
        i += 1
    return sorted(out)


def destination(dest):
    """(path, anchor) of an internal link destination, or None."""
    dest = dest.strip()
    if dest.startswith('<') and '>' in dest:
        dest = dest[1:dest.index('>')]
    else:
        dest = dest.split()[0] if dest.split() else ''
    if not dest or SCHEME.match(dest):
        return None
    path, _, anchor = dest.partition('#')
    if path and not path.endswith('.md'):
        return None
    return path, anchor


def unwrap(text):
    """(opening, core, closing): the text without the emphasis or code span
    that wraps all of it."""
    text = text.strip()
    opening = closing = ''
    while True:
        for mark in WRAPPERS:
            inner = text[len(mark):-len(mark)]
            if (len(text) > 2 * len(mark) and text.startswith(mark) and text.endswith(mark)
                    and mark not in inner and inner == inner.strip()):
                opening += mark
                closing = mark + closing
                text = inner
                break
        else:
            return opening, text, closing


def fold(text, case=True):
    core = unwrap(text)[1]
    core = re.sub(r'\\([\\`*_{}\[\]()>#+\-.!|~<])', r'\1', core)
    core = ' '.join(core.split())
    return core.casefold() if case else core


def norm(text):
    return fold(text)


def plain(text):
    """What a heading renders as, close enough to slug it."""
    text = re.sub(r'!?\[([^\]]*)\]\([^)]*\)', r'\1', text)
    text = re.sub(r'\\(.)', r'\1', text)
    text = text.replace('`', '')
    text = re.sub(r'\*+', '', text)
    return re.sub(r'\s*\{[#.][^}]*\}\s*$', '', text)


def slugify(text):
    """Python-Markdown's default toc slug."""
    value = unicodedata.normalize('NFKD', text).encode('ascii', 'ignore').decode('ascii')
    value = re.sub(r'[^\w\s-]', '', value).strip().lower()
    return re.sub(r'[-\s]+', '-', value)


def heading_ids(page_headings):
    used = set()
    out = []
    for _, source in page_headings:
        base = slug = slugify(plain(source))
        count = 1
        while slug in used or not slug:
            slug = f'{base}_{count}'
            count += 1
        used.add(slug)
        out.append(slug)
    return out


# ------------------------------------------------------------------ Korean

def final_consonant(text):
    """The final consonant index (0 for none) of how `text` ends when read,
    or None when that cannot be told (it ends in Latin letters)."""
    text = text.rstrip()
    while text:
        stripped = text.rstrip('`*_"\'”’ ')
        if stripped.endswith(')') and '(' in stripped:
            # A particle agrees with the word before a trailing parenthesis.
            stripped = stripped[:stripped.rindex('(')]
        if stripped == text:
            break
        text = stripped
    if not text:
        return None
    last = text[-1]
    if '가' <= last <= '힣':
        return (ord(last) - 0xAC00) % 28
    if last.isdigit():
        return DIGIT_FINAL.get(last, 0)
    return None


def particle_for(particle, final):
    for closed, open_ in PARTICLES:
        if particle in (closed, open_):
            if closed == '으로':
                return open_ if final in (0, 8) else closed
            return closed if final else open_
    return particle


def replace_link_text(translation, start, end, new):
    """`translation` with the link text at [start, end) replaced, the particle
    after the link adjusted. Returns (text, particle note or None)."""
    depth = 0
    for close in range(end + 1, len(translation)):
        if translation[close] == '(':
            depth += 1
        elif translation[close] == ')':
            depth -= 1
            if depth == 0:
                break
    tail = translation[close + 1:]
    note = None
    match = PARTICLE.match(tail)
    if match:
        final = final_consonant(new)
        if final is None:
            if final_consonant(translation[start:end]) is not None:
                note = f'particle {match[1]!r} kept: cannot tell how {new!r} ends'
        else:
            particle = particle_for(match[1], final)
            if particle != match[1]:
                tail = particle + tail[len(match[1]):]
                note = f'particle {match[1]} → {particle}'
    return translation[:start] + new + translation[end:close + 1] + tail, note


# ------------------------------------------------------------------- model

class Segment:
    def __init__(self, file, data):
        self.file = file
        self.id = data['id']
        self.source = data['source']
        self.translation = data.get('translation') or ''
        self.original = self.translation


class Title:
    """One title: its source string and the segment that translates it."""

    def __init__(self, source, segment, prefix=''):
        self.source = source
        self.segment = segment
        self.prefix = prefix

    @property
    def translation(self):
        return self.prefix + self.segment.translation


class Page:
    def __init__(self, name, segments, page_headings):
        self.name = name
        self.segments = segments
        self.h1 = None
        self.anchors = {}
        self.labels = []
        position = 0
        for (level, source), slug in zip(page_headings, heading_ids(page_headings)):
            found = self._find(source, position)
            if found is None:
                continue
            index, prefix = found
            position = index + 1
            title = Title(source, segments[index], prefix)
            self.anchors[slug] = title
            if level == 1 and self.h1 is None:
                self.h1 = title

    def _find(self, source, position):
        numbered = NUMBERED.match(source)
        for index in range(position, len(self.segments)):
            if self.segments[index].source == source:
                return index, ''
            if numbered and self.segments[index].source == numbered[2]:
                return index, numbered[1]
        return None


class Site:
    def __init__(self, root):
        self.root = Path(root)
        self.pages = {}
        state = self.root / 'state' / DOCS
        for item in sorted(state.rglob('*' + SUFFIX)):
            name = item.relative_to(state).as_posix()[:-len(SUFFIX)]
            file = item.relative_to(self.root).as_posix()
            segments = [Segment(file, data) for data in json.loads(item.read_text())['segments']]
            source = self.root / DOCS / name
            self.pages[name] = Page(name, segments, headings(source.read_text()))
        nav_file = self.root / NAV_STATE
        self.nav = [Segment(NAV_STATE, data) for data in json.loads(nav_file.read_text())['segments']]
        entries = [(label, match.groupdict().get('target'))
                   for _, _, label, match in parse_nav((self.root / 'upstream/mkdocs.yml').read_text())
                   if label is not None]
        if [label for label, _ in entries] != [segment.source for segment in self.nav]:
            raise ValueError('state/nav does not hold the labels of upstream mkdocs.yml in order '
                             '(run sources.py extract --check and yeokja status --check nav)')
        self.labels = []
        for (label, target), segment in zip(entries, self.nav):
            page = self.pages.get(target.strip()) if target else None
            title = Title(label, segment)
            self.labels.append((title, page))
            if page:
                page.labels.append(title)

    def segments(self):
        yield from self.nav
        for page in self.pages.values():
            yield from page.segments


class Violation:
    def __init__(self, rule, segment, message, fix=None):
        self.rule = rule
        self.segment = segment
        self.message = message
        self.fix = fix          # callable returning a particle note, or None

    def __str__(self):
        return f'{self.segment.file} {self.segment.id}: [{self.rule}] {self.message}'


def identity_terms(root):
    """Source words the term base keeps in the source language, lowercased."""
    words = set()
    for name in ('glossary.toml', 'terms.toml'):
        path = Path(root) / name
        if not path.exists():
            continue
        term = None
        for line in path.read_text().split('\n'):
            head = re.match(r'^\[terms\.(?:"((?:[^"\\]|\\.)*)"|([^\]]+))\]$', line.strip())
            if head:
                term = head[1] if head[1] is not None else head[2]
                continue
            if line.startswith('['):
                term = None
            value = re.match(r'^translation\s*=\s*"((?:[^"\\]|\\.)*)"', line.strip())
            if value and term is not None and value[1] == term:
                words.update(word.casefold() for word in WORD.findall(term))
    return words


def is_source_language(text, kept):
    """Whether a link text still in the source language has prose in it: a
    word that is neither inline code, nor part of an identifier or path
    (`sched_ext`, `mm/`, `page-tables.md`, `fork()`), nor an acronym, nor a
    word the term base or the translated titles keep."""
    text = re.sub(r'`+[^`]*`+', ' ', text)
    for token in text.split():
        token = token.strip('.,:;!?"\'“”‘’*')
        if re.search(r'[_/.()\d=<>\\]', token):
            continue
        for word in WORD.findall(token):
            if len(word) >= 3 and not word.isupper() and word.casefold() not in kept:
                return True
    return False


def names_file(text, target):
    """Whether a link text is the target's file or directory name."""
    key = norm(text)
    parts = target.casefold().split('/')
    return key in (parts[-1], parts[-1][:-len('.md')], *parts[:-1])


def in_titles(page, text):
    """Whether the translated H1 or a translated nav label of the page has
    this text in it: the titles keep these words in the source language."""
    key = ' '.join(text.split()).casefold()
    titles = ([page.h1] if page.h1 else []) + page.labels
    return any(key in ' '.join(title.translation.split()).casefold() for title in titles)


def internal_links(page, segment_text):
    """[(start, end, target page name, anchor, raw dest)] of a segment."""
    out = []
    for start, end, dest in links(segment_text):
        parsed = destination(dest)
        if parsed is None:
            continue
        path, anchor = parsed
        target = posixpath.normpath(posixpath.join(posixpath.dirname(page.name), path)) if path else page.name
        out.append((start, end, target, anchor, dest.strip()))
    return out


def paired_links(page, segment):
    """Source and translated internal links of a segment matched by
    destination, in order: [((start, end, target, anchor), (start, end))]."""
    translated = defaultdict(list)
    for start, end, _, _, dest in internal_links(page, segment.translation):
        translated[dest].append((start, end))
    pairs = []
    seen = Counter()
    for start, end, target, anchor, dest in internal_links(page, segment.source):
        index = seen[dest]
        seen[dest] += 1
        if index < len(translated[dest]):
            pairs.append(((start, end, target, anchor), translated[dest][index]))
    return pairs


def expected_text(source_text, title_translation):
    opening, _, closing = unwrap(source_text)
    return opening + unwrap(title_translation)[1] + closing


def title_of_link(site, text, target, anchor):
    """The title a source link text names, or None."""
    page = site.pages.get(target)
    if page is None:
        return None
    key = norm(text)
    candidates = []
    if anchor and anchor in page.anchors:
        candidates.append(page.anchors[anchor])
    if page.h1:
        candidates.append(page.h1)
    candidates.extend(page.labels)
    for title in candidates:
        if norm(title.source) == key:
            return title
    return None


class Link:
    """One internal link of a segment: its source text and where it points."""

    def __init__(self, segment, page, target, anchor, source_text, ordinal, text):
        self.segment = segment
        self.page = page
        self.target = target
        self.anchor = anchor
        self.source_text = source_text
        self.ordinal = ordinal      # among the segment's links with this target and text
        self.text = text            # the translated text

    def shown(self):
        return f'[{self.text}]({self.target}{"#" + self.anchor if self.anchor else ""})'

    def setter(self, new):
        """A fix that makes `new` the translated text of this link."""
        def apply():
            count = 0
            for (start, end, target, anchor), (t_start, t_end) in paired_links(self.page, self.segment):
                if (target, anchor, self.segment.source[start:end]) != (self.target, self.anchor, self.source_text):
                    continue
                if count == self.ordinal:
                    self.segment.translation, note = replace_link_text(self.segment.translation, t_start, t_end, new)
                    return note
                count += 1
            raise AssertionError('link vanished')
        return apply


def segment_links(site, page, segment):
    """The paired internal links of a segment whose target is translated."""
    ordinals = Counter()
    out = []
    for (start, end, target, anchor), (t_start, t_end) in paired_links(page, segment):
        source_text = segment.source[start:end]
        key = (target, anchor, source_text)
        out.append(Link(segment, page, target, anchor, source_text, ordinals[key],
                        segment.translation[t_start:t_end]))
        ordinals[key] += 1
    return out


def check(site, root=ROOT):
    """(violations, stats)."""
    violations = []
    stats = Counter()

    # h1: the same source H1 has one translation.
    by_source = defaultdict(list)
    for page in site.pages.values():
        if page.h1 is None:
            violations.append(Violation('h1', page.segments[0], f'{page.name} has no H1 in state'))
            continue
        by_source[norm(page.h1.source)].append(page)
    h1_translation = {}
    for key, pages in by_source.items():
        translations = Counter(page.h1.translation for page in pages)
        stats['h1 pages'] += len(pages)
        if len(translations) == 1:
            h1_translation[key] = pages[0].h1.translation
            continue
        for page in pages:
            others = ', '.join(sorted(set(translations) - {page.h1.translation}))
            violations.append(Violation(
                'h1', page.h1.segment,
                f'H1 {page.h1.source!r} is {page.h1.translation!r} here and {others} on other pages'))

    # nav.
    free = defaultdict(list)
    for title, page in site.labels:
        if page and page.h1 and norm(title.source) == norm(page.h1.source):
            stats['nav labels that are the H1'] += 1
            expect = unwrap(page.h1.translation)[1]
            if title.translation != expect:
                violations.append(nav_violation(title, expect, f'the H1 of {page.name}'))
        else:
            free[norm(title.source)].append(title)
    for key, titles in free.items():
        stats['nav labels of their own'] += len(titles)
        if key in h1_translation:
            expect, why = unwrap(h1_translation[key])[1], 'the page whose H1 is this string'
        else:
            expect, why = most_common(title.translation for title in titles), 'the same label elsewhere in the nav'
        for title in titles:
            if title.translation != expect:
                violations.append(nav_violation(title, expect, why))

    # link.
    aliases = defaultdict(list)
    for page in site.pages.values():
        for segment in page.segments:
            found = segment_links(site, page, segment)
            if segment.translation and len(found) != len(internal_links(page, segment.source)):
                violations.append(Violation('link', segment,
                                            'the translation does not keep every internal link of the source'))
            for link in found:
                if link.target not in site.pages:
                    stats['links to untranslated pages'] += 1
                    continue
                title = title_of_link(site, link.source_text, link.target, link.anchor)
                if title is None:
                    stats['other internal links'] += 1
                    aliases[link.target, norm(link.source_text)].append(link)
                    continue
                stats['title links'] += 1
                if title.segment is segment:
                    continue
                expect = expected_text(link.source_text, title.translation)
                same_case = fold(link.source_text, case=False) == fold(title.source, case=False)
                if link.text != expect and (same_case or link.text.casefold() != expect.casefold()):
                    violations.append(Violation(
                        'link', segment,
                        f'{link.shown()} should read {expect!r} ({title.segment.file} {title.segment.id})',
                        link.setter(expect)))

    # alias, latin.
    kept = identity_terms(root)
    for title in [page.h1 for page in site.pages.values() if page.h1] + [title for title, _ in site.labels]:
        kept.update(word for word in WORD.findall(title.translation) if word.islower())
    for (target, _), uses in aliases.items():
        # A text still in the source language is no candidate for the others.
        translated = [unwrap(link.text)[1] for link in uses
                      if unwrap(link.text)[1] != unwrap(link.source_text)[1]]
        expect_core = most_common(translated) if translated else None
        for link in uses:
            opening, core, closing = unwrap(link.text)
            first = unwrap(link.source_text)[1][:1]
            titled = (first.isupper() or first.isdigit()) and len(uses) > 1
            stats['alias links'] += titled
            untranslated = (core == unwrap(link.source_text)[1]
                            and not names_file(link.source_text, target)
                            and not in_titles(site.pages[target], core)
                            and is_source_language(link.source_text, kept))
            if untranslated and expect_core is None:
                violations.append(Violation('latin', link.segment,
                                            f'{link.shown()} is left in the source language'))
            elif expect_core is not None and core != expect_core and (untranslated or titled):
                expect = opening + expect_core + closing
                violations.append(Violation(
                    'alias', link.segment,
                    f'{link.shown()} should read {expect!r} like the other links that call it '
                    f'{link.source_text!r}',
                    link.setter(expect)))
    return violations, stats


def most_common(values):
    """The most common value, the first one on a tie."""
    values = list(values)
    counts = Counter(values)
    best = max(counts.values())
    return next(value for value in values if counts[value] == best)


def nav_violation(title, expect, why):
    def apply():
        title.segment.translation = expect
    return Violation('nav', title.segment,
                     f'label {title.source!r} is {title.translation!r}, should be {expect!r} ({why})', apply)


# ------------------------------------------------------------------- state

def write_translation(text, segment_id, old, new):
    """The state file's text with one segment's translation replaced, every
    other byte kept."""
    at = text.index('"id": ' + json.dumps(segment_id, ensure_ascii=False))
    following = text.find('"id": ', at + 1)
    field = '"translation": ' + json.dumps(old, ensure_ascii=False)
    position = text.find(field, at, following if following != -1 else len(text))
    if position == -1:
        raise ValueError(f'{segment_id}: cannot find the translation to replace')
    return text[:position] + '"translation": ' + json.dumps(new, ensure_ascii=False) + text[position + len(field):]


def save(site):
    changed = defaultdict(list)
    for segment in site.segments():
        if segment.translation != segment.original:
            changed[segment.file].append(segment)
    for file, segments in changed.items():
        path = site.root / file
        text = path.read_text()
        for segment in segments:
            text = write_translation(text, segment.id, segment.original, segment.translation)
            segment.original = segment.translation
        path.write_text(text)
    return sum(len(segments) for segments in changed.values())


def fix(site, root=ROOT, log=print):
    """Apply every decidable fix until none is left. Returns the number of
    segments rewritten."""
    for _ in range(10):
        violations, _ = check(site, root)
        fixable = [violation for violation in violations if violation.fix]
        if not fixable:
            break
        # One fix per segment and pass: a replacement moves the other links.
        done = set()
        for violation in fixable:
            key = (violation.segment.file, violation.segment.id)
            if key in done:
                continue
            done.add(key)
            before = violation.segment.translation
            note = violation.fix()
            log(f'fixed {violation}' + (f' ({note})' if note else ''))
            assert violation.segment.translation != before
    return save(site)


def main(argv):
    options = set(argv)
    if options - {'--fix', '--stats'}:
        print(__doc__)
        return 2
    site = Site(ROOT)
    if '--fix' in options:
        print(f'rewrote {fix(site)} segment(s)')
    violations, stats = check(site)
    for violation in violations:
        print(violation)
    if '--stats' in options:
        for name, count in sorted(stats.items()):
            print(f'{name}: {count}')
    by_rule = Counter(violation.rule for violation in violations)
    detail = ', '.join(f'{rule} {count}' for rule, count in sorted(by_rule.items()))
    print(f'titles: {len(site.pages)} page(s), {len(violations)} violation(s)' + (f' ({detail})' if detail else ''))
    return 1 if violations else 0


if __name__ == '__main__':
    sys.exit(main(sys.argv[1:]))
