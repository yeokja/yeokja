"""번역된 제목에도 영어 원문의 섹션 앵커를 유지하는 Sphinx 확장입니다.

원문의 307개 제목에는 명시적 레이블이 하나도 없어서 HTML 앵커(`#sliding-window`)가
전부 제목 글자에서 나옵니다. 제목을 한국어로 옮기면 docutils가 ASCII가 아닌 제목에
`id1`, `id2`를 붙이므로 원문 사이트로 들어오던 깊은 링크와 쪽 안 목차 앵커가 모두
원문과 달라집니다.

yeokja는 문서 구조를 보존하므로 번역 문서와 원문 문서의 섹션은 수와 순서가 같습니다.
이 확장은 doctree-read 단계에서 같은 경로의 원문 RST를 docutils로 따로 읽어
(`.. include::`와 `|Intro|` 같은 치환은 원문 그대로 풀립니다) 섹션 id를 등장 순서대로
모으고, 번역 doctree의 같은 자리 섹션에 그 id를 첫 id로 올립니다. 목차 수집기보다
먼저 실행하므로 쪽 안 목차와 전역 목차 링크도 영어 앵커를 씁니다.

원문 디렉터리는 conf.py의 `en_anchors_reference`로 넘깁니다. 섹션 수가 다른 문서는
경고를 내고 기본 id를 그대로 둡니다(빌드는 `--fail-on-warning`이므로 실패합니다).
"""

from __future__ import annotations

import io
import json
import subprocess
import sys
from pathlib import Path

from docutils import nodes
from docutils.core import publish_doctree

if __name__ != "__main__":
    from sphinx.application import Sphinx
    from sphinx.util import logging

    logger = logging.getLogger(__name__)

# Sphinx 전용 역할·지시문은 docutils만으로는 모르므로 오류 노드가 되지만 섹션
# 구조와 제목 글자(=id)는 그대로 남습니다. 그 진단은 버립니다.
_SETTINGS = {
    "report_level": 5,
    "halt_level": 5,
    "warning_stream": io.StringIO(),
    "file_insertion_enabled": True,
    "raw_enabled": False,
    "input_encoding": "utf-8",
    # Sphinx처럼 맨 위 섹션을 문서 제목으로 올리지 않아야 섹션 수가 같습니다.
    "doctitle_xform": False,
    "sectsubtitle_xform": False,
}


def reference_section_ids(path: Path) -> list[str]:
    """원문 RST를 docutils로 읽어 섹션의 첫 id를 문서 순서대로 돌려줍니다.

    Sphinx가 docutils 전역 등록부에 자기 지시문·역할을 올려 두므로 빌드 프로세스
    안에서 읽으면 그것들이 env를 찾다 실패하고, env를 넘기면 원문이 번역 문서의
    env를 오염시킵니다. 그래서 순수 docutils만 있는 별도 프로세스에서 읽습니다.
    """
    doctree = publish_doctree(
        path.read_text(encoding="utf-8"),
        source_path=str(path),
        settings_overrides=dict(_SETTINGS, warning_stream=io.StringIO()),
    )
    return [section["ids"][0] for section in doctree.findall(nodes.section) if section["ids"]]


_reference_ids: dict[str, list[str]] | None = None


def load_reference_ids(reference: Path) -> dict[str, list[str]]:
    """원문 최상위 *.rst(빌드 문서; 하위 디렉터리 파일은 include로만 쓰임)의 섹션 id를
    한 번에 별도 프로세스에서 모읍니다."""
    global _reference_ids
    if _reference_ids is None:
        names = sorted(path.name for path in reference.glob("*.rst"))
        result = subprocess.run(
            [sys.executable, "-I", __file__, str(reference), *names],
            check=True,
            capture_output=True,
            text=True,
        )
        _reference_ids = json.loads(result.stdout)
    return _reference_ids


def promote_id(doctree: nodes.document, section: nodes.section, new_id: str) -> bool:
    """new_id를 섹션의 첫 id로 올립니다. 다른 노드가 이미 쓰는 id면 건드리지 않습니다."""
    if section["ids"][:1] == [new_id]:
        return True
    owner = doctree.ids.get(new_id)
    if owner is not None and owner is not section:
        return False
    if new_id in section["ids"]:
        section["ids"].remove(new_id)
    section["ids"].insert(0, new_id)
    doctree.ids[new_id] = section
    return True


def promote_english_ids(app: Sphinx, doctree: nodes.document) -> None:
    reference = app.config.en_anchors_reference
    if not reference:
        logger.warning("en_anchors: en_anchors_reference가 비어 있어 영어 앵커를 유지하지 않습니다", type="en_anchors")
        return
    docname = app.env.docname
    translated = Path(app.env.doc2path(docname))
    original = Path(reference) / translated.relative_to(app.srcdir)
    if original.parent != Path(reference) or not original.is_file():
        return
    english = load_reference_ids(Path(reference))[original.name]
    sections = [section for section in doctree.findall(nodes.section)]
    if len(english) != len(sections):
        logger.warning(
            "en_anchors: %s의 섹션 수(%d)가 원문(%d)과 달라 영어 앵커를 유지하지 못합니다",
            docname,
            len(sections),
            len(english),
            type="en_anchors",
        )
        return
    for section, english_id in zip(sections, english):
        if not promote_id(doctree, section, english_id):
            logger.warning(
                "en_anchors: %s에서 영어 앵커 %r를 다른 노드가 이미 씁니다",
                docname,
                english_id,
                type="en_anchors",
            )


def setup(app: Sphinx) -> dict:
    app.add_config_value("en_anchors_reference", "", "env", [str])
    # 목차 수집기·autosectionlabel(기본 우선순위 500)보다 먼저 실행합니다.
    app.connect("doctree-read", promote_english_ids, priority=400)
    return {"version": "1", "parallel_read_safe": True, "parallel_write_safe": True}


if __name__ == "__main__":
    root = Path(sys.argv[1])
    json.dump({name: reference_section_ids(root / name) for name in sys.argv[2:]}, sys.stdout)
