import importlib.util
import sys
import unittest
from pathlib import Path

HAS_MYST = importlib.util.find_spec("myst_parser") is not None


@unittest.skipUnless(HAS_MYST, "myst-parser가 설치된 환경(build/venv)에서만 실행합니다")
class KoSlugsTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        sys.path.insert(0, str(Path(__file__).parent))
        import ko_slugs

        cls.ko_slugs = ko_slugs

    def test_heading_titles_match_myst_view(self) -> None:
        text = (
            "---\ntitle: x\n---\n\n# Ad hoc `shell` environments\n\n"
            "```sh\n# not a heading\n```\n\n## Search *for* packages\n\n#### Too deep\n"
        )
        self.assertEqual(
            self.ko_slugs.heading_titles(text),
            ["Ad hoc shell environments", "Search for packages"],
        )

    def test_pair_headings_maps_translated_titles_in_order(self) -> None:
        reference = "# Ad hoc shell environments\n\n## Search for packages\n\n## Next steps\n\n## Next steps\n"
        translated = "# 임시 셸 환경\n\n## 패키지 검색\n\n## 다음 단계\n\n## 다음 단계\n"
        mapping = self.ko_slugs.pair_headings(reference, translated)
        self.assertEqual(list(mapping["임시 셸 환경"]), ["ad-hoc-shell-environments"])
        self.assertEqual(list(mapping["패키지 검색"]), ["search-for-packages"])
        # 같은 번역 제목이 둘이면 등장 순서대로 소비하고 myst가 -1을 덧붙입니다.
        self.assertEqual(list(mapping["다음 단계"]), ["next-steps", "next-steps"])

    def test_pair_headings_refuses_structural_mismatch(self) -> None:
        self.assertIsNone(self.ko_slugs.pair_headings("# A\n\n## B\n", "# 가\n"))


if __name__ == "__main__":
    unittest.main()
