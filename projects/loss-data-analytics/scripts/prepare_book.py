#!/usr/bin/env python3
"""조립 트리(build/tree)를 bookdown으로 knit하기 전에 다듬습니다.

    prepare_book.py <tree> <date YYYY-MM-DD>
    prepare_book.py --portability <tree>     # 영어 원문 비교 빌드: 아래 '이식성'만

이식성(원문을 Windows·옛 패키지로 knit해 생긴 문제, 영어 원문 빌드에도 똑같이 적용):
- R 코드의 Windows 경로 "Data\\파일.csv"를 "Data/파일.csv"로 바꿉니다.
- 현재 VGAM에 없는 링크 이름 "loge"를 "loglink"로 바꿉니다.
- 실제 파일명이 .PNG인 그림 참조의 확장자 대소문자를 맞춥니다(Linux용).
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


PORTABILITY = [
    (re.compile(r'read\.csv\("Data\\\\'), 'read.csv("Data/'),
    # nixpkgs의 VGAM(1.1-14)에는 옛 링크 이름 "loge"가 없습니다(같은 함수의 새 이름 "loglink").
    (re.compile(r'\b(l(?:scale|shape))="loge"'), r'\1="loglink"'),
    # 원문은 Figures/F5Iterative.PNG, FigureEvolution.PNG를 소문자 .png로 참조합니다. macOS의
    # 대소문자 무시 파일 시스템에서는 지나가지만 Linux(CI)에서는 include_graphics()가 파일을
    # 못 찾아 빌드가 멈춥니다.
    (re.compile(r'Figures/(F5Iterative|FigureEvolution)\.png'), r'Figures/\1.PNG'),
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
