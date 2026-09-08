"""번역된 제목에도 영어 원문의 앵커(#slug)를 유지하는 Sphinx 확장입니다.

nix.dev는 myst_heading_anchors=3으로 제목 텍스트에서 GitHub식 슬러그를 만들고,
문서끼리 `[…](./file.md#some-heading)` 꼴로 그 슬러그를 참조합니다. 제목을
한국어로 옮기면 슬러그가 전부 바뀌어 이런 링크와 외부에서 들어오는 깊은
링크가 깨집니다. 게다가 docutils는 ASCII가 아닌 제목의 섹션 id를 `id1`,
`id2`처럼 붙이므로 HTML의 앵커 자체도 사라집니다.

yeokja는 문서 구조를 보존하므로 번역본과 원문의 제목은 수와 순서가 같습니다.
이 확장은 두 트리의 각 문서를 myst와 같은 markdown-it 파서로 토큰화해 제목을
자리별로 짝지어 두고,

1. myst의 heading_slug_func로 번역된 제목 텍스트를 원문 슬러그로 돌려보내며
   (같은 문서 안에서 같은 번역 제목이 여럿이면 등장 순서대로 소비합니다),
2. doctree-read 단계에서 그 슬러그를 섹션의 첫 번째 id로 넣어 HTML 앵커와
   페이지 안 목차 링크도 영어 슬러그를 쓰게 합니다.

원문 디렉터리는 conf.py의 `ko_slugs_reference` 설정으로 넘깁니다. 제목 수가
다른 문서(번역이 구조를 바꾼 경우)는 경고를 내고 기본 슬러그로 돌아갑니다.
"""

from __future__ import annotations

from collections import defaultdict, deque
from pathlib import Path

from docutils import nodes
from markdown_it.renderer import RendererHTML
from myst_parser.config.main import MdParserConfig
from myst_parser.mdit_to_docutils.base import default_slugify
from myst_parser.parsers.mdit import create_md_parser
from sphinx.application import Sphinx
from sphinx.util import logging

logger = logging.getLogger(__name__)

# 슬러그가 붙는 제목 깊이. upstream conf.py의 myst_heading_anchors와 맞춥니다.
MAX_LEVEL = 3


def heading_titles(text: str, max_level: int = MAX_LEVEL) -> list[str]:
    """myst의 compute_unique_slug가 보는 것과 같은 제목 텍스트를 순서대로 돌려줍니다."""
    parser = create_md_parser(MdParserConfig(), RendererHTML)
    tokens = parser.parse(text)
    titles = []
    for index, token in enumerate(tokens):
        if token.type != "heading_open" or int(token.tag[1:]) > max_level:
            continue
        inline = tokens[index + 1]
        titles.append(
            "".join(
                child.content
                for child in (inline.children or [])
                if child.type in ("text", "code_inline")
            )
        )
    return titles


def pair_headings(reference: str, translated: str) -> dict[str, deque[str]] | None:
    """번역 제목 → 원문 슬러그 대기열. 제목 수가 다르면 None."""
    original = heading_titles(reference)
    current = heading_titles(translated)
    if len(original) != len(current):
        return None
    mapping: dict[str, deque[str]] = defaultdict(deque)
    for source, target in zip(original, current):
        mapping[target].append(default_slugify(source))
    return mapping


class SlugMap:
    def __init__(self, app: Sphinx, reference: Path) -> None:
        self.app = app
        self.reference = reference
        self.docs: dict[str, dict[str, deque[str]]] = {}

    def load(self, docname: str) -> dict[str, deque[str]]:
        if docname in self.docs:
            return self.docs[docname]
        translated = Path(self.app.env.doc2path(docname))
        original = self.reference / translated.relative_to(self.app.srcdir)
        mapping: dict[str, deque[str]] = {}
        if original.is_file():
            paired = pair_headings(
                original.read_text(encoding="utf-8"),
                translated.read_text(encoding="utf-8"),
            )
            if paired is None:
                logger.warning(
                    "ko_slugs: %s의 제목 수가 원문과 달라 영어 앵커를 유지하지 못합니다",
                    docname,
                    type="ko_slugs",
                )
            else:
                mapping = paired
        self.docs[docname] = mapping
        return mapping

    def slug(self, title: str) -> str:
        docname = self.app.env.docname
        queue = self.load(docname).get(title) if docname else None
        if queue:
            return queue.popleft()
        return default_slugify(title)

    def reset(self, docname: str) -> None:
        self.docs.pop(docname, None)


# Sphinx는 설정을 deepcopy·pickle하므로 heading_slug_func에는 app을 붙든
# 바운드 메서드 대신 myst가 임포트해 주는 모듈 경로 문자열("ko_slugs.slug")을
# 등록하고, 상태는 모듈 전역에 둡니다.
_slug_map: SlugMap | None = None


def slug(title: str) -> str:
    if _slug_map is None:
        return default_slugify(title)
    return _slug_map.slug(title)


def install_slug_func(app: Sphinx, config) -> None:
    global _slug_map
    reference = config.ko_slugs_reference
    if not reference:
        logger.warning("ko_slugs: ko_slugs_reference가 비어 있어 영어 앵커를 유지하지 않습니다", type="ko_slugs")
        return
    _slug_map = SlugMap(app, Path(reference))
    config.myst_heading_slug_func = f"{__name__}.slug"


def promote_slug_ids(app: Sphinx, doctree: nodes.document) -> None:
    """myst 슬러그를 섹션의 첫 id로 올려 HTML 앵커와 목차 링크가 영어 슬러그를 쓰게 합니다."""
    docname = app.env.docname
    if _slug_map is not None:
        _slug_map.reset(docname)
    myst_slugs = app.env.metadata.get(docname, {}).get("myst_slugs", {})
    for section in doctree.findall(nodes.section):
        slug = section.get("slug")
        if not slug or slug in section["ids"][:1]:
            continue
        owner = doctree.ids.get(slug)
        if owner is not None and owner is not section:
            continue
        if slug in section["ids"]:
            section["ids"].remove(slug)
        section["ids"].insert(0, slug)
        doctree.ids[slug] = section
        if slug in myst_slugs:
            line, _, text = myst_slugs[slug]
            myst_slugs[slug] = (line, slug, text)


def setup(app: Sphinx) -> dict:
    app.add_config_value("ko_slugs_reference", "", "env", [str])
    app.connect("config-inited", install_slug_func)
    # 환경 수집기(목차 등)보다 먼저 실행해야 목차 링크가 영어 슬러그를 씁니다.
    app.connect("doctree-read", promote_slug_ids, priority=400)
    return {"version": "1", "parallel_read_safe": True, "parallel_write_safe": True}
