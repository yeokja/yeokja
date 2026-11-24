"""Prepare the assembled Sphinx tree for the Korean build.

Run from `[[derive.step]]` inside the assembled tree (cwd = tree root). The tree
is a link tree whose files point back into `upstream/` or `ko/`, so every edit
replaces the link with a fresh file (`os.replace`) instead of writing through
it. Each edit is anchored on exact upstream text and fails loudly when that text
moved, so an upstream bump cannot silently ship an English or GA-enabled build.
Running the script twice on the same tree is a no-op.
"""

import os
import re
import stat
import sys
import tempfile
from pathlib import Path


BEGIN_MARKER = "# BEGIN YEOKJA KOREAN CONFIG"
END_MARKER = "# END YEOKJA KOREAN CONFIG"

BOOK_TITLE = "컴퓨터 네트워크: 시스템 접근"
ORIGINAL_URL = "https://book.systemsapproach.org/"
SOURCE_URL = "https://github.com/SystemsApproach/7E"
LICENSE_URL = "https://creativecommons.org/licenses/by/4.0/deed.ko"
ANNOUNCEMENT = (
    "이 사이트는 Larry L. Peterson과 Bruce S. Davie의 "
    f'<a href="{ORIGINAL_URL}"><i>Computer Networks: A Systems Approach</i></a> '
    f'(<a href="{SOURCE_URL}">SystemsApproach/7E</a>)를 한국어로 옮긴 비공식 기계 번역입니다. '
    f'원문과 번역 모두 <a href="{LICENSE_URL}">CC BY 4.0</a>에 따라 배포되며, '
    "본문을 번역하고 언어·제목·그림 번호 형식을 바꾸고 분석 스크립트를 뺐습니다. "
    "정확한 내용은 원문을 확인하세요."
)

MANAGED_BLOCK = f'''{BEGIN_MARKER}
# yeokja: 한국어 빌드 설정입니다. 원문 설정 뒤에 덧붙여 같은 이름을 다시 정의합니다.
language = "ko"
# chapters.rst는 장 번호 치환(|Intro| 등)만 담고 각 장이 `.. include::`로 읽습니다.
# 조립 트리는 심볼릭 링크라 Sphinx가 그 include 관계를 알아보지 못하고 "어느 목차에도
# 없는 문서" 경고를 내므로, 독립 문서로는 빌드하지 않습니다(include는 그대로 됩니다).
exclude_patterns = [*exclude_patterns, "chapters.rst"]
# 맞춤법 검사(PyEnchant → libenchant)와 여러 버전 빌드는 번역 사이트에 쓰지 않습니다.
extensions = [
    name
    for name in extensions
    if name not in ("sphinxcontrib.spelling", "sphinx_multiversion")
]
numfig_format = {{"figure": "그림 %s.", "table": "표 %s."}}
# 원문 제목에는 명시적 레이블이 없어 앵커가 모두 제목 글자에서 나옵니다.
# 번역 제목에도 원문 앵커를 유지합니다(scripts/en_anchors.py). 프로젝트 디렉터리는
# yeokja build가 주는 $YEOKJA_ROOT로 찾습니다.
sys.path.insert(0, os.path.join(os.environ["YEOKJA_ROOT"], "scripts"))
extensions.append("en_anchors")
en_anchors_reference = os.path.join(os.environ["YEOKJA_ROOT"], "upstream")
# autosectionlabel 레이블은 제목 글자이므로 서로 다른 원문 제목이 같은 번역어가 되면
# 중복 경고가 납니다. 원문의 :ref:는 명시적 레이블 하나(reading-VL2)만 가리키므로
# 이 경고는 깨진 참조를 뜻하지 않습니다.
suppress_warnings = [*globals().get("suppress_warnings", []), "autosectionlabel.*"]
version = release = version.replace("Version ", "버전 ", 1)
html_title = "{BOOK_TITLE}"
html_theme_options = {{
    **html_theme_options,
    "announcement": {ANNOUNCEMENT!r},
}}


def _spelling_ignore_role(name, rawtext, text, lineno, inliner, options=None, content=None):
    from docutils import nodes

    return [nodes.Text(text)], []


def setup(app):
    """원문 setup()은 Google Analytics 스크립트만 넣으므로 대신합니다.

    맞춤법 검사 확장을 뺐으므로 그 확장이 쓰는 `:spelling:ignore:` 역할만
    글자 그대로 내보내는 작은 `spelling` 도메인으로 남깁니다.
    """
    from sphinx.domains import Domain

    class SpellingIgnoreDomain(Domain):
        name = "spelling"
        label = "Spelling (ignored)"
        roles = {{"ignore": _spelling_ignore_role}}

        def merge_domaindata(self, docnames, otherdata):
            pass

    app.add_domain(SpellingIgnoreDomain)
{END_MARKER}'''

# (relative path, exact upstream text, replacement). Each upstream text must
# occur exactly once; after replacement the Korean text must occur exactly once.
TEXT_EDITS = [
    (
        "_templates/layout.html",
        "href=\"{{ pathto('genindex') }}\">Acronym Glossary</a>",
        "href=\"{{ pathto('genindex') }}\">약어 목록</a>",
    ),
    (
        "_templates/genindex.html",
        "{% set title = 'Acronym Glossary' %}",
        "{% set title = '약어 목록' %}",
    ),
    (
        "_templates/genindex.html",
        '<h1 id="index">Acronym Glossary</h1>',
        '<h1 id="index">약어 목록</h1>',
    ),
    (
        "_templates/genindex.html",
        "This glossary expands the acronyms (and other network jargon) used\n"
        "throughout the book. It's not an exhaustive index, but instead focuses\n"
        "on terms you are likely to encounter in multiple contexts. Each item\n"
        "is linked to the section that gives the best introduction to the\n"
        'term. Use the "Search" facility to find other occurrences.\n',
        "이 목록은 책 전체에서 쓰는 약어(와 그 밖의 네트워크 전문 용어)를 풀어 줍니다.\n"
        "빠짐없는 색인이 아니라 여러 맥락에서 마주칠 만한 용어에 집중하며, 각 항목은\n"
        "그 용어를 가장 잘 소개하는 절로 이어집니다. 다른 곳에 나오는 용례는 검색으로\n"
        "찾으십시오. 항목은 원문의 영어 약어와 풀이 그대로입니다.\n",
    ),
    ("index.rst", "   :caption: Table of Contents\n", "   :caption: 목차\n"),
    ("index.rst", "   Acronym Glossary <genindex>\n", "   약어 목록 <genindex>\n"),
]

# conf.py text the managed block overrides; if any of it moved, the override
# may no longer mean what it says, so stop.
CONF_ANCHORS = [
    "language = 'en'\n",
    "    'sphinxcontrib.spelling',\n",
    '    "sphinx_multiversion",\n',
    "numfig_format = {\n    'figure': 'Figure %s.',\n    'table':  'Table %s.'\n    }\n",
    "version = get_version()\n",
    'html_title = "Computer Networks: A Systems Approach"\n',
    "html_theme_options = {\n",
    "def setup(app):\n",
    "googletagmanager.com/gtag/js",
    "spelling_word_list_filename",
    "import os\nimport sys\n",
    "exclude_patterns = [",
]


def _atomic_write(path: Path, text: str) -> None:
    mode = stat.S_IMODE(path.stat().st_mode)
    temporary_path = None
    try:
        with tempfile.NamedTemporaryFile(
            mode="w",
            encoding="utf-8",
            dir=path.parent,
            prefix=f".{path.name}.",
            suffix=".tmp",
            delete=False,
        ) as temporary:
            temporary_path = Path(temporary.name)
            temporary.write(text)
        os.chmod(temporary_path, mode | stat.S_IWUSR)
        os.replace(temporary_path, path)
        temporary_path = None
    finally:
        if temporary_path is not None:
            temporary_path.unlink(missing_ok=True)


def prepare_conf(path: Path) -> None:
    text = path.read_text(encoding="utf-8")
    begins, ends = text.count(BEGIN_MARKER), text.count(END_MARKER)
    if begins or ends:
        if begins != 1 or ends != 1 or MANAGED_BLOCK not in text:
            raise ValueError(f"{path}: managed block is damaged")
        return
    for anchor in CONF_ANCHORS:
        if text.count(anchor) != 1:
            raise ValueError(f"{path}: expected exactly one {anchor!r}")
    separator = "\n" if text.endswith("\n") else "\n\n"
    _atomic_write(path, text + separator + MANAGED_BLOCK + "\n")


def apply_text_edits(root: Path) -> None:
    for relative in dict.fromkeys(name for name, _, _ in TEXT_EDITS):
        path = root / relative
        original = text = path.read_text(encoding="utf-8")
        for name, before, after in TEXT_EDITS:
            if name != relative:
                continue
            if text.count(before) == 1 and after not in text:
                text = text.replace(before, after)
            elif not (before not in text and text.count(after) == 1):
                raise ValueError(f"{path}: expected exactly one {before!r}")
        if text != original:
            _atomic_write(path, text)


# `:numref:` 글자의 "Figure %s"·"Table %s"는 Sphinx가 번호를 채우는 고정 틀입니다.
# 역할 내용은 파서와 용어 검사가 손대지 않는 원자라 번역으로는 일관되게 맞출 수
# 없으므로, 조립 트리에서 낱말만 "그림"·"표"로 바꿉니다(`%s`와 `<표적>`은 그대로).
NUMREF_LABELS = [
    (re.compile(r"(:numref:`\s*)Figures?(?=\s+%s)"), r"\1그림"),
    (re.compile(r"(:numref:`\s*)Tables?(?=\s+%s)"), r"\1표"),
]


def localize_numref_labels(root: Path) -> int:
    changed = 0
    for path in sorted(root.rglob("*.rst")):
        if any(part.startswith(("_build", ".")) for part in path.relative_to(root).parts):
            continue
        original = text = path.read_text(encoding="utf-8")
        for pattern, replacement in NUMREF_LABELS:
            text = pattern.sub(replacement, text)
        if text != original:
            _atomic_write(path, text)
            changed += 1
    return changed


def prepare(root: Path) -> None:
    prepare_conf(root / "conf.py")
    apply_text_edits(root)
    localize_numref_labels(root)


def main(argv: list[str] | None = None) -> int:
    args = sys.argv[1:] if argv is None else argv
    if len(args) != 1:
        print("usage: prepare.py <tree>", file=sys.stderr)
        return 2
    try:
        prepare(Path(args[0]))
    except (OSError, ValueError) as error:
        print(error, file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
