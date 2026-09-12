"""Localize fixed layout labels after yeokja overlays without modifying symlink targets."""
import re
import sys
from pathlib import Path


def write_local(path: Path, text: str) -> None:
    if path.is_symlink():
        path.unlink()
    path.write_text(text, encoding="utf-8")


def prepare(tree: Path) -> None:
    path = tree / "symbols.tex"
    text = path.read_text(encoding="utf-8")
    text = text.replace(r"\addcontentsline{toc}{part}{Index of symbols}",
                        r"\addcontentsline{toc}{part}{기호 찾아보기}")
    text = re.sub(r"\\markboth\{\}\{\\textsc\{[^}]*\}\}",
                  lambda _: r"\markboth{}{\textsc{기호 찾아보기}}", text)
    write_local(path, text)
    path = tree / "induction.tex"
    if path.exists():
        # The upstream exercise uses bare \LEM even though the macro takes one
        # argument. Make its empty subscript explicit before a Korean particle.
        text = path.read_text(encoding="utf-8")
        text = re.sub(r"\\LEM(?=[가-힣])", lambda _: r"\LEM{}", text)
        write_local(path, text)
    path = tree / "introduction.tex"
    if path.exists():
        text = path.read_text(encoding="utf-8").replace(
            r"\addcontentsline{toc}{chapter}{Introduction}",
            r"\addcontentsline{toc}{chapter}{서론}")
        write_local(path, text)
    path = tree / "front.tex"
    text = path.read_text(encoding="utf-8")
    marker = r"\input{version.tex}"
    if text.count(marker) != 1:
        raise ValueError("expected one version marker on the copyright page")
    notice = r"""
\noindent\textbf{비공식 한국어 번역}\par
\noindent 원저작자: The Univalent Foundations Program.\\
이 번역은 yeokja와 Codex gpt-6-astra를 이용하여 작성했습니다.\\
원문과 번역은 CC BY-SA 3.0으로 배포합니다.\\
원저작자가 이 번역을 검토하거나 보증하지 않습니다.\\
\url{https://github.com/yeokja/yeokja/tree/main/projects/hott}\par
\medskip
"""
    write_local(path, text.replace(marker, marker + notice))


if __name__ == "__main__":
    prepare(Path(sys.argv[1]) if len(sys.argv) > 1 else Path.cwd())
