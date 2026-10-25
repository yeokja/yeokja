#!/usr/bin/env python3
"""Loss Data Analytics 한국어판 검사.

    check.py source <upstream-dir> <ko-dir> [<GlossLabels.csv> [<tree>]]
        번역한 Rmd마다 원문과 같은 바이트여야 하는 것을 다중집합으로 비교합니다:
        R/tikz/js 청크 본문, 청크 머리(fig.cap·fig.alt 문자열 제외),
        디스플레이 수식($$…$$, \\[…\\], \\begin{…}…\\end{…}; 공백 런은 하나로 봄), 인라인 R
        (Hide*의 두 번째 인자 제외), \\@ref(…), (\\#eq:…), (#tab:…), (ref:…).
        Gloss 인자가 표시어 사전(GlossLabels.csv)에 모두 있는지도 봅니다. <tree>를 주면
        <ko-dir>의 파일 목록으로 조립 트리(prepare_book.py 뒤)의 같은 파일을 봅니다.

    check.py site <site-dir> <expected-chapters>
        gitbook 산출물의 장 수(목차에서 번호 붙은 장), 로컬 링크와 #앵커가
        모두 실재하는지, 본문에 knitr 오류 출력("## Error")이나 처리되지 않은
        `r 인라인 코드, 깨진 교차 참조(??)가 없는지 봅니다.
"""

import collections
import csv
import html.parser
import re
import sys
from pathlib import Path
from urllib.parse import unquote, urlsplit

FENCE = re.compile(r"^([ \t]*)(`{3,}|~{3,})(.*)$", re.M)
CAP_ATTR = re.compile(r"""\b(fig\.cap|fig\.alt|name)\s*=\s*(['"])(?:\\.|(?!\2).)*\2""")
HIDE_LABEL = re.compile(r"""^(r Hide\w+\(\s*['"][^'"]*['"]\s*,\s*)(['"])(?:(?!\2).)+\2""")


def split_chunks(text):
    """(산문, [(머리, 본문)]) — 펜스 코드 블록을 떼어 냅니다."""
    prose, chunks, pos = [], [], 0
    lines = text.splitlines(keepends=True)
    i = 0
    out = []
    while i < len(lines):
        m = FENCE.match(lines[i].rstrip("\n"))
        # $$ 안의 줄머리 ~~~\mathrm… 은 펜스가 아니다(정보 문자열이 \로 시작)
        if m and not m.group(3).lstrip().startswith("\\"):
            fence = m.group(2)
            j = i + 1
            while j < len(lines):
                m2 = FENCE.match(lines[j].rstrip("\n"))
                if m2 and m2.group(2)[0] == fence[0] and len(m2.group(2)) >= len(fence) and not m2.group(3).strip():
                    break
                j += 1
            head = lines[i].strip()
            body = "".join(lines[i + 1 : j])
            # bookdown 산문 청크(theorem 등)는 번역 대상이므로 본문을 비교하지 않음
            if re.match(r"`{3,}\s*\{\s*(r|tikz|js|verbatim)\b", head) or not head.startswith("```{"):
                chunks.append((CAP_ATTR.sub(r"\1=<cap>", head), body))
            else:
                out.append("".join(lines[i : j + 1]))
            i = j + 1
            continue
        out.append(lines[i])
        i += 1
    return "".join(out), chunks


INLINE_R = re.compile(r"(?<!`)`(r [^`]+)`(?!`)")
DISPLAY = re.compile(r"\$\$.+?\$\$|\\\[.+?\\\]|\\begin\{([A-Za-z*]+)\}.+?\\end\{\1\}", re.S)
TOKENS = re.compile(r"\\@ref\([^)]*\)|\(\\?#[a-z]+:[^)]*\)|\(ref:[^)]*\)")


# prepare_book.py가 일부러 바꾸는 인라인 R(원문 쪽, 바뀐 쪽).
INTENDED = {
    ("inline", 'r format(Sys.Date(), "%d %B %Y")'): None,
    ("inline", "r if (knitr:::is_html_output()) '# Bibliography {-}'"): (
        "inline",
        "r if (knitr:::is_html_output()) '# 참고 문헌 {#bibliography .unnumbered}'",
    ),
}


def source_items(text):
    # prepare_book.py의 이식성 손질(Windows 경로)은 양쪽에서 같게 봅니다.
    text = text.replace('read.csv("Data\\\\', 'read.csv("Data/')
    text = re.sub(r'\b(l(?:scale|shape))="loge"', r'\1="loglink"', text)
    prose, chunks = split_chunks(text)
    items = collections.Counter()
    for head, body in chunks:
        items[("chunk", head, body)] += 1
    for m in INLINE_R.finditer(prose):
        code = " ".join(m.group(1).split())
        items[("inline", HIDE_LABEL.sub(r"\1<label>", code))] += 1
    no_code = INLINE_R.sub("", prose)
    for m in DISPLAY.finditer(no_code):
        # 디스플레이 수식이 든 문단은 세그먼트가 줄바꿈을 공백으로 합칩니다
        # (rmarkdown 파서 설계 3절). TeX에서 같은 뜻이므로 공백 런을 하나로 봅니다.
        items[("math", " ".join(m.group(0).split()))] += 1
    for m in TOKENS.finditer(no_code):
        items[("token", m.group(0))] += 1
    return items


def gloss_args(text):
    return [m.group(2) for m in re.finditer(r"`r Gloss\(\s*(['\"])(.+?)\1", text)]


def check_source(up_dir, ko_dir, labels_csv=None, tree=None):
    up_dir, ko_dir = Path(up_dir), Path(ko_dir)
    bad = 0
    files = sorted(ko_dir.rglob("*.Rmd"))
    if tree:
        # prepare_book.py가 손질한 조립 트리의 같은 파일을 봅니다.
        files = [Path(tree) / f.relative_to(ko_dir) for f in files]
        ko_dir = Path(tree)
    for ko in files:
        rel = ko.relative_to(ko_dir)
        up = up_dir / rel
        a = source_items(up.read_text(encoding="utf-8"))
        b = source_items(ko.read_text(encoding="utf-8"))
        if tree:
            for orig, new in INTENDED.items():
                if orig in a and (new is None or new in b):
                    a = a - collections.Counter({orig: a[orig]})
                    if new:
                        b = b - collections.Counter({new: a.get(new, 0) + b[new]})
        if a != b:
            bad += 1
            missing = a - b
            extra = b - a
            print(f"{rel}: {sum(missing.values())} missing, {sum(extra.values())} extra")
            for k in list(missing)[:5]:
                print("  -", repr(k)[:300])
            for k in list(extra)[:5]:
                print("  +", repr(k)[:300])
    if labels_csv:
        with open(labels_csv, encoding="utf-8", newline="") as f:
            known = {row["Term"] for row in csv.DictReader(f)}
        for ko in files:
            for arg in gloss_args(ko.read_text(encoding="utf-8")):
                if arg not in known:
                    bad += 1
                    print(f"{ko.relative_to(ko_dir)}: Gloss('{arg}') has no label")
    print(f"source: {len(files)} files, {bad} problems")
    return bad == 0


# 영어 원문 빌드(같은 devShell, prepare_book.py --portability)에도 있는 원문의 결함.
# 원문 Rmd의 없는 레이블 참조, 저장소 밖(../docs/) 링크 같은 것이라 번역본에서도 그대로입니다.
ORIGINAL_DEFECTS = {
    "CAppB.html: missing anchor #S:AggLoss:Tweedie",
    "CAppB.html: 1 x broken cross-reference",
    "ChapAggLossModels.html: missing anchor ChapAggLossModels.html#eq:case2",
    "ChapFrequency-Modeling.html: missing ../docs/ChapIntro.html#Exer:1.1",
    "ChapSimulation.html: missing anchor #Ex13.4.6",
    "ChapSimulation.html: missing ../docs/ChapClaimSeverity.html#Ex:53.2",
}


class Links(html.parser.HTMLParser):
    def __init__(self):
        super().__init__()
        self.ids, self.hrefs = set(), []

    def handle_starttag(self, tag, attrs):
        d = dict(attrs)
        for k in ("id", "name"):
            if d.get(k):
                self.ids.add(d[k])
        for k in ("href", "src"):
            if d.get(k) and not (tag == "script" and k == "href"):
                self.hrefs.append((tag, d[k]))


def check_site(site, expected_chapters):
    site = Path(site)
    pages = sorted(site.glob("*.html"))
    parsed = {}
    for p in pages:
        lp = Links()
        lp.feed(p.read_text(encoding="utf-8"))
        parsed[p.name] = lp
    problems = []
    index = (site / "index.html").read_text(encoding="utf-8")
    # 번호 붙은 장(서문·참고 문헌처럼 번호 없는 항목과 절은 세지 않음)
    chapters = len(re.findall(r'<li class="chapter" data-level="\d+" ', index))
    if chapters != expected_chapters:
        problems.append(f"chapter entries in TOC: {chapters} != {expected_chapters}")
    for name, lp in parsed.items():
        for tag, href in lp.hrefs:
            # 원문은 Windows에서 쓴 ContributorPics\x.jpg 같은 경로를 쓰고, 브라우저는
            # http(s) URL의 \를 /로 읽습니다.
            u = urlsplit(href.replace("\\", "/"))
            if u.scheme or u.netloc or href.startswith(("javascript:", "mailto:", "data:")):
                continue
            target = name if not u.path else unquote(u.path)
            if target.endswith("/"):
                target += "index.html"
            tpath = site / target
            if not tpath.exists():
                problems.append(f"{name}: missing {href}")
                continue
            if u.fragment and target in parsed:
                frag = unquote(u.fragment)
                if frag not in parsed[target].ids:
                    problems.append(f"{name}: missing anchor {href}")
        text = (site / name).read_text(encoding="utf-8")
        body = text.split('<div class="body-inner">', 1)[-1]
        for pat, what in (
            (r"## Error", "knitr error output"),
            (r"`r ", "unevaluated inline R"),
            (r"<strong>\?\?</strong>", "broken cross-reference"),
        ):
            n = len(re.findall(pat, body))
            if n:
                problems.append(f"{name}: {n} x {what}")
    known = [p for p in problems if p in ORIGINAL_DEFECTS]
    new = [p for p in problems if p not in ORIGINAL_DEFECTS]
    for p in new:
        print(p)
    print(
        f"site: {len(pages)} pages, {chapters} chapters, {len(new)} problems "
        f"({len(known)} also in the English original)"
    )
    return not new


def main():
    cmd = sys.argv[1]
    if cmd == "source":
        ok = check_source(*sys.argv[2:6])
    elif cmd == "site":
        ok = check_site(sys.argv[2], int(sys.argv[3]))
    else:
        sys.exit(__doc__)
    sys.exit(0 if ok else 1)


if __name__ == "__main__":
    main()
