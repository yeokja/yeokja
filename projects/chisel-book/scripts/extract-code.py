#!/usr/bin/env python3
"""Extract the source snippets referenced by the Chisel book."""

from __future__ import annotations

import argparse
from pathlib import Path


SOURCE_DIRECTORIES = ("src/main/scala", "src/test/scala", "src/main/vhdl")


def extract(root: Path) -> int:
    output_directory = root / "code"
    output_directory.mkdir(parents=True, exist_ok=True)
    written = 0

    for relative_directory in SOURCE_DIRECTORIES:
        for source in sorted((root / relative_directory).rglob("*")):
            if not source.is_file():
                continue

            output = None
            try:
                for line in source.read_text(encoding="utf-8").splitlines():
                    tokens = line.strip().split()
                    marker = len(tokens) >= 2 and tokens[0] in {"//-", "--/"}
                    if marker and tokens[1] == "start":
                        if len(tokens) != 3:
                            raise ValueError(f"invalid start marker in {source}: {line}")
                        # Match the upstream Scala extractor: a nested marker
                        # replaces the active output (memory.scala relies on it).
                        if output is not None:
                            output.close()
                        output = (output_directory / f"{tokens[2]}.txt").open(
                            "w", encoding="utf-8", newline="\n"
                        )
                        written += 1
                    elif marker and tokens[1] == "end":
                        if output is None:
                            raise ValueError(f"unmatched end marker in {source}: {line}")
                        output.close()
                        output = None
                    elif output is not None:
                        output.write(f"{line}\n")
            finally:
                if output is not None:
                    output.close()

    return written


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("root", type=Path)
    args = parser.parse_args()
    print(f"extracted {extract(args.root)} code snippets")


if __name__ == "__main__":
    main()
