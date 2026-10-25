#!/usr/bin/env python3
"""조립 트리(build/tree)를 bookdown으로 knit하기 전에 다듬습니다.

    prepare_book.py <tree> <date YYYY-MM-DD> <upstream>
    prepare_book.py --portability <tree>     # 영어 원문 비교 빌드: 아래 '이식성'만

이식성(원문을 Windows·옛 패키지로 knit해 생긴 문제, 영어 원문 빌드에도 똑같이 적용):
- R 코드의 Windows 경로 "Data\\파일.csv"를 "Data/파일.csv"로 바꿉니다.
- 현재 VGAM에 없는 링크 이름 "loge"를 "loglink"로 바꿉니다.
- 닫히지 않은 접기 상자 <div> 세 곳(손실 심도 장 2, 총손실 장 1)을 닫고, 분포표의 닫지 않은
  <p> 하나를 지웁니다(Pandoc 3이 원시 HTML <div>를 Div로 읽기 때문에 생기는 목차 붕괴).

한국어판:

- Format/ShowHide2024GSTag.js(gitbook in_header)에서 Google Analytics 태그를 지웁니다.
- index.Rmd 머리말의 `r format(Sys.Date(), ...)`를 원문 커밋 날짜로 고정합니다
  (빌드 날짜에 따라 산출물이 바뀌지 않게).
- 인라인 R이 만드는 HTML 전용 장 제목 '# Bibliography {-}'와 front matter의 author 한 줄을
  한국어로 바꿉니다(둘 다 파서가 번역 단위로 내놓지 않음).
- References/Glossary.Rmd의 Gloss()가 화면에 번역한 표시어(References/GlossLabels.csv)를
  보여 주게 하고, 풍선 도움말에 원어를 괄호로 덧붙입니다. 첫 인자는 GlossFct.csv의
  조회 키라 그대로 둡니다.
- knitr 청크 본문을 원문에서 되돌립니다. 원문이 `<div>` 안에 둔 청크는 CommonMark가
  HTML 블록으로 읽어, 청크 안 빈 줄 뒤의 R 코드가 문단으로 번역 단위에 들어가고 줄이
  합쳐진 채 재구성됩니다(Simulation 장 1곳). knitr는 청크를 먼저 실행하므로 청크는
  원문과 바이트가 같아야 합니다. 청크 수나 머리가 원문과 다르면 실패합니다.
- 번역 문단에서 줄 가운데로 들어간 번호 수식 환경(\\begin{equation} 등)을 줄 머리로
  되돌립니다. rmarkdown 파서는 디스플레이 수식이 든 문단을 한 줄로 합쳐 내놓는데, Pandoc은
  줄 가운데의 \\begin{equation}을 수식으로 읽지 않아 bookdown이 식 번호와 (\\#eq:) 앵커를
  만들지 못합니다.
- `$$` 안에서 줄 가운데로 들어간 식 레이블 (\\#eq:…)을 제 줄에 둡니다(같은 이유).
- `[표 8.3][Table 8.3](#tab:8.3)`·`[표 8.3][Table 8.3]{#tab:8.3}`을 `[표 8.3](#tab:8.3)`·
  `[표 8.3]{#tab:8.3}`으로 되돌립니다. 원문의
  `[Table 8.3](#tab:8.3)`은 인라인 링크인데 파서가 `(#tab:8.3)`을 참조 토큰으로 가린 채 읽어
  같은 이름의 참조 정의(`[Table 8.3]: ./ChapSimulation#tab:8.3`)를 쓰는 참조 링크로 보고,
  직렬화기가 그렇게 써 냅니다.
- 본문의 `r Gloss('term')` 바로 뒤 조사(은/는, 이/가, 을/를, 과/와, 으로/로)를
  표시어의 받침에 맞춥니다. 번역 모델은 영어 키를 보고 조사를 고르므로 표시어와
  어긋날 수 있습니다.

바꿀 자리가 없으면 실패합니다(원문이 바뀌어 규칙이 조용히 무력해지는 것을 막음).
"""

import csv
import re
import sys
from pathlib import Path

GLOSS_PATCH_READ = (
    'Gloss_data <- read.csv("References/GlossFct.csv", header = TRUE)\n',
    'Gloss_data <- read.csv("References/GlossFct.csv", header = TRUE, fileEncoding = "UTF-8")\n'
    'Gloss_labels <- read.csv("References/GlossLabels.csv", header = TRUE, fileEncoding = "UTF-8")\n'
    "GlossLabel <- function(term){\n"
    "  i <- match(term, Gloss_labels$Term)\n"
    "  if (is.na(i)) term else Gloss_labels$Label[[i]]\n"
    "}\n",
)
GLOSS_PATCH_CALL = (
    r"(?m)^(\s*)return\(paste\(A1,term,A2,(Glossfunction\(term\)|GlossfunctionBySection\(term, section\)),A3, sep=\"\"\)\)",
    r'\1return(paste(A1,GlossLabel(term),A2,"(",term,") ",\2,A3, sep=""))',
)
GLOSS_CALL = re.compile(r"`r Gloss\(\s*(['\"])(.+?)\1[^`]*`")
# (받침 있을 때, 받침 없을 때). 뒤에 한글이 이어지면 조사가 아닐 수 있어 건드리지 않습니다.
PARTICLES = [("은", "는"), ("이", "가"), ("을", "를"), ("과", "와")]


def has_batchim(word: str):
    """마지막 한글 음절의 받침 여부. 한글로 끝나지 않으면 None."""
    word = re.sub(r"[\s)\]*_]+$", "", word)
    if not word or not ("가" <= word[-1] <= "힣"):
        return None
    return (ord(word[-1]) - 0xAC00) % 28


def fix_particles(text: str, labels: dict) -> tuple:
    out, pos, fixed = [], 0, 0
    for m in GLOSS_CALL.finditer(text):
        out.append(text[pos : m.end()])
        pos = m.end()
        jong = has_batchim(labels.get(m.group(2), ""))
        if jong is None:
            continue
        rest = text[pos : pos + 3]
        if rest.startswith(("으로", "로")):
            word = "으로" if rest.startswith("으로") else "로"
            nxt = text[pos + len(word) : pos + len(word) + 1]
            want = "로" if jong in (0, 8) else "으로"  # ㄹ 받침(8)은 '로'
            if word != want and not ("가" <= nxt <= "힣" and nxt not in "서써는부"):
                out.append(want)
                pos += len(word)
                fixed += 1
            continue
        for with_b, without_b in PARTICLES:
            for word in (with_b, without_b):
                nxt = text[pos + 1 : pos + 2]
                if rest.startswith(word) and not ("가" <= nxt <= "힣"):
                    want = with_b if jong else without_b
                    if word != want:
                        out.append(want)
                        pos += 1
                        fixed += 1
                    break
            else:
                continue
            break
    out.append(text[pos:])
    return "".join(out), fixed


def write(path: Path, text: str) -> None:
    """조립 트리의 파일은 원문·ko/를 가리키는 심볼릭 링크일 수 있으므로 링크를 끊고 씁니다."""
    if path.is_symlink():
        path.unlink()
    path.write_text(text, encoding="utf-8")


CHUNK_OPEN = re.compile(r"^[ \t]*(`{3,})\s*\{")


def chunks(text: str):
    """knitr 청크 (머리 줄 끝 위치, 본문 끝 위치, 머리 줄) 목록."""
    out, lines, pos, i = [], text.splitlines(keepends=True), 0, 0
    offsets = []
    for line in lines:
        offsets.append(pos)
        pos += len(line)
    offsets.append(pos)
    while i < len(lines):
        m = CHUNK_OPEN.match(lines[i])
        if m:
            j = i + 1
            close = re.compile(r"^[ \t]*`{%d,}\s*$" % len(m.group(1)))
            while j < len(lines) and not close.match(lines[j]):
                j += 1
            out.append((offsets[i + 1], offsets[j], lines[i]))
            i = j + 1
            continue
        i += 1
    return out


def restore_chunks(path: Path, original: Path) -> int:
    text = path.read_text(encoding="utf-8")
    orig = original.read_text(encoding="utf-8")
    a, b = chunks(orig), chunks(text)
    if len(a) != len(b):
        sys.exit(f"prepare_book: {path}: {len(b)} chunks, original has {len(a)}")
    restored, out, pos = 0, [], 0
    for (os_, oe, ohead), (ts, te, thead) in zip(a, b):
        if ohead.split(",")[0].strip() != thead.split(",")[0].strip():
            sys.exit(f"prepare_book: {path}: chunk header {thead.strip()!r} != {ohead.strip()!r}")
        out.append(text[pos:ts])
        out.append(orig[os_:oe])
        restored += text[ts:te] != orig[os_:oe]
        pos = te
    out.append(text[pos:])
    if restored:
        write(path, "".join(out))
    return restored


PORTABILITY = [
    (re.compile(r'read\.csv\("Data\\\\'), 'read.csv("Data/'),
    # nixpkgs의 VGAM(1.1-14)에는 옛 링크 이름 "loge"가 없습니다(같은 함수의 새 이름 "loglink").
    (re.compile(r'\b(l(?:scale|shape))="loge"'), r'\1="loglink"'),
    # 접기 상자(Hide*)의 <div>를 닫지 않은 곳 세 군데. Pandoc 3은 원시 HTML <div>를 Div로
    # 읽어 닫히지 않은 Div가 뒤의 모든 장을 감싸게 되고, bookdown이 5장부터 목차와 장
    # 나누기를 잃습니다. 원문의 다른 상자처럼 풀이 끝에 </div>를 넣습니다.
    (re.compile(r"(`r HideRCode\('Toy\.4f'.*?\n</div>\n)", re.S), r"\1\n</div>\n"),
    (re.compile(r"(`r HideExample\('4\.4\.3'.*?`r SolnEnd\(\)`\n)", re.S), r"\1\n</div>\n"),
    (re.compile(r"(`r HideRCode\('Ex642A\.Hide'.*?`r CodeFontLarge\(\)`\n)", re.S), r"\1\n</div>\n"),
    # 한계기대값 표 탭(levA)의 닫지 않은 <p>가 같은 이유로 탭 <div>를 닫지 못하게 해, 마지막 두
    # 장(표기법, 데이터 자료)이 분포표 장 안으로 들어갑니다.
    (re.compile(r'(<div id="levA" class="tabcontent">\n.*\n  <h3>[^<\n]*</h3>\n)  <p>\n'), r"\1"),
]


def fix_portability(tree: Path) -> None:
    hits = [0] * len(PORTABILITY)
    for rmd in sorted(tree.glob("Chapters/**/*.Rmd")):
        text = rmd.read_text(encoding="utf-8")
        new = text
        for i, (pat, repl) in enumerate(PORTABILITY):
            new, n = pat.subn(repl, new)
            hits[i] += n
        if new != text:
            write(rmd, new)
    for (pat, _), n in zip(PORTABILITY, hits):
        if not n:
            sys.exit(f"prepare_book: portability rule matched nothing: {pat.pattern}")
    print(f"prepare_book: portability fixes {hits}")


ENVS = r"(?:equation|align|eqnarray|gather|multline)\*?"
BEGIN_ENV = re.compile(r"\\begin\{" + ENVS + r"\}")
END_ENV = re.compile(r"\\end\{" + ENVS + r"\}")
REF_LINK = re.compile(r"\[([^\[\]\n]+)\]\[[^\[\]\n]+\](\(#[A-Za-z]+:[^()\s]+\)|\{#[A-Za-z]+:[^{}\s]+\})")
EQ_LABEL = re.compile(r"\(\\#eq:[^()\s]+\)")


def math_mask(line: str) -> list:
    """줄의 각 글자가 $…$ 또는 $$…$$ 안인지(그 줄 안에서만 셈)."""
    inside, out, i, mode = False, [False] * len(line), 0, None
    while i < len(line):
        if line[i] == "\\":
            out[i : i + 2] = [inside] * len(out[i : i + 2])
            i += 2
            continue
        if line[i] == "$":
            run = 2 if line.startswith("$$", i) else 1
            if not inside:
                inside, mode = True, run
            elif run == mode or (mode == 2 and run == 2):
                inside = False
            out[i : i + run] = [True] * run
            i += run
            continue
        out[i] = inside
        i += 1
    return out


def fix_prose(text: str) -> tuple:
    """청크 밖 줄에서 번호 수식 환경을 줄 머리로, 참조 링크를 인라인 링크로."""
    spans = [(s, e) for s, e, _ in chunks(text)]
    out, pos, envs, links = [], 0, 0, 0
    lines = text.splitlines(keepends=True)
    offset = 0
    for line in lines:
        start = offset
        offset += len(line)
        if any(s <= start < e for s, e in spans):
            out.append(line)
            continue
        line, n = REF_LINK.subn(r"[\1]\2", line)
        links += n
        mask = math_mask(line)
        cuts = []
        for m in BEGIN_ENV.finditer(line):
            if not mask[m.start()] and line[: m.start()].strip():
                cuts.append(m.start())
        for m in END_ENV.finditer(line):
            if not mask[m.start()] and line[m.end() :].strip():
                cuts.append(m.end())
        # $$ 안의 식 레이블은 bookdown이 제 줄에 홀로 있을 때만 \tag와 앵커로 바꿉니다.
        for m in EQ_LABEL.finditer(line):
            if mask[m.start()]:
                # bookdown은 줄 머리의 <span class="math display">만 봅니다: 여는 $$도 줄 머리로.
                open_at = line.rfind("$$", 0, m.start())
                if open_at > 0 and line[:open_at].strip():
                    cuts.append(open_at)
                if line[: m.start()].strip():
                    cuts.append(m.start())
                if line[m.end() :].strip():
                    cuts.append(m.end())
        for c in sorted(cuts, reverse=True):
            line = line[:c].rstrip(" ") + "\n" + line[c:].lstrip(" ")
            envs += 1
        out.append(line)
    return "".join(out), envs, links


def sub_once(path: Path, pattern: str, repl: str, flags: int = 0) -> None:
    text = path.read_text(encoding="utf-8")
    new, n = re.subn(pattern, repl, text, count=1, flags=flags)
    if n != 1:
        sys.exit(f"prepare_book: pattern not found in {path}: {pattern}")
    write(path, new)


def main() -> None:
    if sys.argv[1] == "--portability":
        fix_portability(Path(sys.argv[2]))
        return
    tree = Path(sys.argv[1])
    y, m, d = (int(x) for x in sys.argv[2].split("-"))

    # 다른 손질보다 먼저: 이 뒤의 손질(Glossary.Rmd 청크)을 되돌리지 않게
    upstream = Path(sys.argv[3])
    restored = 0
    for rmd in sorted(tree.rglob("*.Rmd")):
        rel = rmd.relative_to(tree)
        if rel.parts[0] in ("docs", "build") or not (upstream / rel).exists():
            continue
        restored += restore_chunks(rmd, upstream / rel)
    print(f"prepare_book: restored {restored} chunk bodies from the original")
    fix_portability(tree)


    sub_once(
        tree / "Format/ShowHide2024GSTag.js",
        r"<!-- Google tag \(gtag\.js\) -->.*?</script>\s*<script>.*?</script>\s*",
        "",
        re.S,
    )
    sub_once(
        tree / "index.Rmd",
        r"`r format\(Sys\.Date\(\), \"%d %B %Y\"\)`",
        f"{y}년 {m}월 {d}일",
    )
    # front matter의 author는 파서가 번역 단위로 내놓지 않는 한 줄짜리 표제 정보입니다.
    sub_once(
        tree / "index.Rmd",
        r'^author: "An open text authored by the Actuarial Community"$',
        'author: "보험계리 공동체가 함께 쓴 개방형 교재"',
        re.M,
    )
    sub_once(
        tree / "Chapters/Bibliography.Rmd",
        r"'# Bibliography \{-\}'",
        "'# 참고 문헌 {#bibliography .unnumbered}'",
    )

    glossary = tree / "References/Glossary.Rmd"
    text = glossary.read_text(encoding="utf-8")
    if GLOSS_PATCH_READ[0] not in text:
        sys.exit(f"prepare_book: Gloss_data read.csv not found in {glossary}")
    text = text.replace(GLOSS_PATCH_READ[0], GLOSS_PATCH_READ[1], 1)
    text, n = re.subn(GLOSS_PATCH_CALL[0], GLOSS_PATCH_CALL[1], text)
    if n != 2:
        sys.exit(f"prepare_book: expected 2 Gloss() return statements in {glossary}, found {n}")
    write(glossary, text)

    envs = links = 0
    for rmd in sorted(tree.glob("Chapters/**/*.Rmd")):
        text = rmd.read_text(encoding="utf-8")
        new, e, l = fix_prose(text)
        if new != text:
            write(rmd, new)
        envs, links = envs + e, links + l
    print(f"prepare_book: {envs} line breaks around equation environments and labels, fixed {links} reference links")

    with open(tree / "References/GlossLabels.csv", encoding="utf-8", newline="") as f:
        labels = {row["Term"]: row["Label"] for row in csv.DictReader(f)}
    total = 0
    for rmd in sorted(tree.glob("Chapters/**/*.Rmd")):
        text = rmd.read_text(encoding="utf-8")
        new, fixed = fix_particles(text, labels)
        if fixed:
            write(rmd, new)
            total += fixed
    print(f"prepare_book: fixed {total} particles after Gloss()")


if __name__ == "__main__":
    main()
