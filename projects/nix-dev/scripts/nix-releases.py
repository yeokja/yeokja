#!/usr/bin/env python3
"""source/reference/nix-manual.md의 @…@ 자리 표시자를 실제 버전으로 바꿉니다.

upstream은 이 파일을 nix/releases.nix로 전처리합니다. 그 계산을 Nix 없이
재현합니다.

- @nix-latest@: nix/nix-versions.json의 가장 높은 릴리스(예: 2.35)
- @nixpkgs-stable@, @nixpkgs-prev-stable@: nix/sources.json에 고정된 Nixpkgs
  릴리스 중 가장 높은 둘(예: 26.05, 25.11)
- @nix-rolling@, @nix-stable@, @nix-prev-stable@: 각 Nixpkgs 고정본(rolling은
  npins/sources.json의 nixpkgs-rolling)에서 `pkgs.nix.version`의 major.minor

마지막 셋은 Nixpkgs를 평가해야 나오는 값이라 JSON만으로는 알 수 없습니다.
Nixpkgs 저장소의 pkgs/tools/package-management/nix/default.nix에서
`stable = … self.nix_2_NN` 줄을 읽어 알아내고(all-packages.nix의
`nix = nixVersions.stable`), 결과를 프로젝트의 nixpkgs-nix-versions.json에
리비전별로 기록해 둡니다. 기록된 리비전은 다시 받지 않으므로 upstream이
고정본을 올릴 때만 네트워크가 필요하고, 그때 갱신된 JSON을 함께 커밋합니다.

사용법:
    nix-releases.py substitutions <upstream-dir> <cache-json>
    nix-releases.py substitute <upstream-dir> <cache-json> <in.md> <out.md>
"""

from __future__ import annotations

import json
import re
import sys
import urllib.request
from pathlib import Path
from typing import Callable

NIXPKGS_NIX_FILE = "pkgs/tools/package-management/nix/default.nix"
STABLE_RE = re.compile(r"^\s*stable\s*=.*?\bnix_(\d+)_(\d+)\b", re.MULTILINE)


def version_key(version: str) -> tuple[int, ...]:
    return tuple(int(part) for part in version.split("."))


def fetch_nixpkgs_file(revision: str) -> str:
    url = f"https://raw.githubusercontent.com/NixOS/nixpkgs/{revision}/{NIXPKGS_NIX_FILE}"
    with urllib.request.urlopen(url, timeout=60) as response:
        return response.read().decode("utf-8")


def nix_version_in_nixpkgs(source: str) -> str:
    """Nixpkgs의 nix/default.nix 본문에서 `pkgs.nix`의 major.minor를 읽습니다."""
    match = STABLE_RE.search(source)
    if not match:
        raise ValueError(f"{NIXPKGS_NIX_FILE}에서 `stable = … nix_X_Y` 줄을 찾지 못했습니다")
    return f"{match.group(1)}.{match.group(2)}"


def resolve_nix_versions(
    revisions: dict[str, str],
    cache_path: Path,
    fetch: Callable[[str], str] = fetch_nixpkgs_file,
) -> dict[str, str]:
    """{이름: Nixpkgs 리비전} → {이름: Nix major.minor}. 캐시에 없는 리비전만 받아 옵니다."""
    cache: dict[str, str] = {}
    if cache_path.exists():
        cache = json.loads(cache_path.read_text(encoding="utf-8"))
    changed = False
    result = {}
    for name, revision in revisions.items():
        if revision not in cache:
            print(f"nix-releases: Nixpkgs {revision[:12]}({name})의 Nix 버전을 GitHub에서 확인합니다", file=sys.stderr)
            cache[revision] = nix_version_in_nixpkgs(fetch(revision))
            changed = True
        result[name] = cache[revision]
    if changed:
        cache_path.write_text(json.dumps(cache, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    return result


def substitutions(upstream: Path, cache_path: Path, fetch: Callable[[str], str] = fetch_nixpkgs_file) -> dict[str, str]:
    nix_versions = json.loads((upstream / "nix/nix-versions.json").read_text(encoding="utf-8"))
    nixpkgs_pins = json.loads((upstream / "nix/sources.json").read_text(encoding="utf-8"))["pins"]
    main_pins = json.loads((upstream / "npins/sources.json").read_text(encoding="utf-8"))["pins"]

    nixpkgs_sorted = sorted(nixpkgs_pins, key=version_key)
    stable, prev_stable = nixpkgs_sorted[-1], nixpkgs_sorted[-2]
    resolved = resolve_nix_versions(
        {
            "rolling": main_pins["nixpkgs-rolling"]["revision"],
            "stable": nixpkgs_pins[stable]["revision"],
            "prev-stable": nixpkgs_pins[prev_stable]["revision"],
        },
        cache_path,
        fetch,
    )
    return {
        "nix-latest": max(nix_versions, key=version_key),
        "nix-rolling": resolved["rolling"],
        "nix-stable": resolved["stable"],
        "nix-prev-stable": resolved["prev-stable"],
        "nixpkgs-stable": stable,
        "nixpkgs-prev-stable": prev_stable,
    }


def substitute(text: str, values: dict[str, str]) -> str:
    for name, value in values.items():
        text = text.replace(f"@{name}@", value)
    leftover = re.findall(r"@[a-z-]+@", text)
    if leftover:
        raise ValueError(f"치환되지 않은 자리 표시자가 남았습니다: {sorted(set(leftover))}")
    return text


def main(argv: list[str]) -> int:
    if len(argv) >= 3 and argv[0] == "substitutions":
        values = substitutions(Path(argv[1]), Path(argv[2]))
        print(json.dumps(values, indent=2))
        return 0
    if len(argv) == 5 and argv[0] == "substitute":
        values = substitutions(Path(argv[1]), Path(argv[2]))
        source = Path(argv[3]).read_text(encoding="utf-8")
        Path(argv[4]).write_text(substitute(source, values), encoding="utf-8")
        return 0
    print(__doc__, file=sys.stderr)
    return 2


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
