#!/usr/bin/env python3
"""Gloss('term') 표시어 사전(gloss/GlossLabels.csv)을 만듭니다.

    gloss_labels.py extract [--check]

원문의 `r Gloss('term'[, 'section'])`은 문장 속에서 용어를 녹색 풍선 도움말로
보여 줍니다. 첫 인자는 References/GlossFct.csv의 조회 키이자 화면에 보이는
글이므로 번역하면 조회가 깨집니다. 그래서 빌드 대상 Rmd에 나오는 첫 인자를
처음 나온 순서대로 모아 `Term`(키, 번역하지 않음)과 `Label`(표시어, 번역함)
두 열의 CSV로 두고, yeokja가 `Label` 열만 번역합니다. 한국어판의
References/Glossary.Rmd(overlay)가 이 사전으로 표시어를 바꿉니다.

--check는 파일을 쓰지 않고, 원문에서 다시 뽑은 내용과 다르면 실패합니다.
"""

import csv
import io
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
UPSTREAM = ROOT / "upstream"
OUT = ROOT / "gloss" / "GlossLabels.csv"
CHILDREN = [
    "Chapters/DistributionTableSections/Continuous.Rmd",
    "Chapters/DistributionTableSections/Discrete.Rmd",
    "Chapters/DistributionTableSections/LimitedExpValue.Rmd",
]
GLOSS = re.compile(r"`r Gloss\(\s*(['\"])(.+?)\1")


def build_files():
    text = (UPSTREAM / "_bookdown.yml").read_text(encoding="utf-8")
    return re.findall(r'"([^"]+\.Rmd)"', text) + CHILDREN


def render():
    seen = []
    for rel in build_files():
        for m in GLOSS.finditer((UPSTREAM / rel).read_text(encoding="utf-8")):
            if m.group(2) not in seen:
                seen.append(m.group(2))
    buf = io.StringIO()
    w = csv.writer(buf, lineterminator="\n")
    w.writerow(["Term", "Label"])
    for term in seen:
        w.writerow([term, term])
    return buf.getvalue(), len(seen)


def main():
    if sys.argv[1:2] != ["extract"]:
        sys.exit(__doc__)
    text, n = render()
    if "--check" in sys.argv[2:]:
        if not OUT.exists() or OUT.read_text(encoding="utf-8") != text:
            sys.exit(f"{OUT.relative_to(ROOT)} is out of date; run gloss_labels.py extract")
        print(f"{OUT.relative_to(ROOT)}: {n} terms, up to date")
        return
    OUT.parent.mkdir(exist_ok=True)
    OUT.write_text(text, encoding="utf-8")
    print(f"wrote {OUT.relative_to(ROOT)}: {n} terms")


if __name__ == "__main__":
    main()
