from __future__ import annotations

import json
import tempfile
import unittest
from pathlib import Path

from check_example_pairs import mismatches


def write_state(path: Path, pairs: list[tuple[str, str]]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    segments = [{"source": s, "translation": t} for s, t in pairs]
    path.write_text(json.dumps({"segments": segments}), encoding="utf-8")


class ExamplePairTests(unittest.TestCase):
    def setUp(self) -> None:
        self.directory = tempfile.TemporaryDirectory()
        self.state = Path(self.directory.name)

    def tearDown(self) -> None:
        self.directory.cleanup()

    def test_matching_translations_pass(self) -> None:
        write_state(self.state / "upstream/examples/A.lean.yeokja.json", [("Reduce", "축약")])
        write_state(self.state / "upstream/book/B.lean.yeokja.json", [("Reduce", "축약"), ("Other", "기타")])
        self.assertEqual(mismatches(self.state), [])

    def test_book_only_fix_is_reported(self) -> None:
        write_state(self.state / "upstream/examples/A.lean.yeokja.json", [("Reduce", "축약")])
        write_state(self.state / "upstream/book/B.lean.yeokja.json", [("Reduce", "축약합니다")])
        self.assertEqual([m[0] for m in mismatches(self.state)], ["Reduce"])

    def test_example_sentence_absent_from_book_is_ignored(self) -> None:
        write_state(self.state / "upstream/examples/A.lean.yeokja.json", [("Only here", "여기만")])
        write_state(self.state / "upstream/book/B.lean.yeokja.json", [])
        self.assertEqual(mismatches(self.state), [])


if __name__ == "__main__":
    unittest.main()
