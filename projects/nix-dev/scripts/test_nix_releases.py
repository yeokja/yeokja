import importlib.util
import json
import tempfile
import unittest
from pathlib import Path


def load_module():
    spec = importlib.util.spec_from_file_location("nix_releases", Path(__file__).with_name("nix-releases.py"))
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


NIXPKGS_NIX = """
      nix_2_31 = addTests "nix_2_31" self.nixComponents_2_31.nix-everything;
      latest = self.nix_2_34;
      stable = addFallbackPathsCheck self.nix_2_31;
"""


class NixReleasesTest(unittest.TestCase):
    def write_upstream(self, root: Path) -> None:
        (root / "nix").mkdir()
        (root / "npins").mkdir()
        (root / "nix/nix-versions.json").write_text(json.dumps({
            "2.9": {}, "2.35": {}, "2.34": {},
        }))
        (root / "nix/sources.json").write_text(json.dumps({"pins": {
            "23.05": {"revision": "aaa"},
            "26.05": {"revision": "ccc"},
            "25.11": {"revision": "bbb"},
        }}))
        (root / "npins/sources.json").write_text(json.dumps({"pins": {
            "nixpkgs-rolling": {"revision": "rrr"},
        }}))

    def test_substitutions_follow_releases_nix(self) -> None:
        module = load_module()
        fetched = []

        def fetch(revision: str) -> str:
            fetched.append(revision)
            return NIXPKGS_NIX.replace("nix_2_31;", f"nix_2_{len(revision)};")

        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            self.write_upstream(root)
            cache = root / "cache.json"
            cache.write_text(json.dumps({"rrr": "2.34"}))

            values = module.substitutions(root, cache, fetch)

            self.assertEqual(values, {
                "nix-latest": "2.35",
                "nix-rolling": "2.34",
                "nix-stable": "2.3",
                "nix-prev-stable": "2.3",
                "nixpkgs-stable": "26.05",
                "nixpkgs-prev-stable": "25.11",
            })
            # 캐시된 rolling 리비전은 받지 않고, 새로 본 둘은 캐시에 기록됩니다.
            self.assertEqual(fetched, ["ccc", "bbb"])
            self.assertEqual(json.loads(cache.read_text()), {"rrr": "2.34", "ccc": "2.3", "bbb": "2.3"})

    def test_nix_version_in_nixpkgs_reads_stable_alias(self) -> None:
        module = load_module()
        self.assertEqual(module.nix_version_in_nixpkgs(NIXPKGS_NIX), "2.31")
        with self.assertRaises(ValueError):
            module.nix_version_in_nixpkgs("nothing here")

    def test_substitute_rejects_leftover_placeholders(self) -> None:
        module = load_module()
        text = "Nix @nix-latest@ ([single page](nix-@nix-latest@.html)) in Nixpkgs @nixpkgs-stable@"
        result = module.substitute(text, {"nix-latest": "2.35", "nixpkgs-stable": "26.05"})
        self.assertEqual(result, "Nix 2.35 ([single page](nix-2.35.html)) in Nixpkgs 26.05")
        with self.assertRaises(ValueError):
            module.substitute("@nix-rolling@", {})


if __name__ == "__main__":
    unittest.main()
