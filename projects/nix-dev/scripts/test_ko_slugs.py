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
            [(1, "Ad hoc shell environments"), (2, "Search for packages"), (4, "Too deep")],
        )

    def test_pair_headings_maps_translated_titles_in_order(self) -> None:
        reference = "# Ad hoc shell environments\n\n## Search for packages\n\n## Next steps\n\n## Next steps\n"
        translated = "# 임시 셸 환경\n\n## 패키지 검색\n\n## 다음 단계\n\n## 다음 단계\n"
        mapping = self.ko_slugs.pair_headings(reference, translated)
        self.assertEqual(list(mapping.slugs["임시 셸 환경"]), ["ad-hoc-shell-environments"])
        self.assertEqual(list(mapping.slugs["패키지 검색"]), ["search-for-packages"])
        # 같은 번역 제목이 둘이면 등장 순서대로 소비하고 myst가 -1을 덧붙입니다.
        self.assertEqual(list(mapping.slugs["다음 단계"]), ["next-steps", "next-steps"])
        self.assertEqual(list(mapping.ids["다음 단계"]), ["next-steps", "next-steps"])

    def test_pair_headings_uses_docutils_ids_below_anchor_level(self) -> None:
        reference = "# Nix language\n\n#### Interactive evaluation\n\n#### Recursive attribute set `rec { ... }`\n"
        translated = "# Nix 언어\n\n#### 대화형 평가\n\n#### 재귀 속성 집합 `rec { ... }`\n"
        mapping = self.ko_slugs.pair_headings(reference, translated, anchor_level=3)
        self.assertEqual(list(mapping.slugs["Nix 언어"]), ["nix-language"])
        # h4는 슬러그가 없고 docutils make_id만 있습니다 — upstream 빌드의 섹션 id.
        self.assertNotIn("대화형 평가", mapping.slugs)
        self.assertEqual(list(mapping.ids["대화형 평가"]), ["interactive-evaluation"])
        self.assertEqual(
            list(mapping.ids["재귀 속성 집합 rec { ... }"]),
            ["recursive-attribute-set-rec"],
        )

    def test_html_anchor_is_docutils_id_not_slug(self) -> None:
        # h3 "Attribute set `{ ... }`": 슬러그는 attribute-set---지만 upstream HTML의
        # 앵커는 docutils id attribute-set입니다. 둘 다 기록해 링크와 앵커를 각각 맞춥니다.
        mapping = self.ko_slugs.pair_headings("### Attribute set `{ ... }`\n", "### 속성 집합 `{ ... }`\n")
        self.assertEqual(list(mapping.slugs["속성 집합 { ... }"]), ["attribute-set---"])
        self.assertEqual(list(mapping.ids["속성 집합 { ... }"]), ["attribute-set"])

    def test_pair_headings_refuses_structural_mismatch(self) -> None:
        self.assertIsNone(self.ko_slugs.pair_headings("# A\n\n## B\n", "# 가\n"))


if __name__ == "__main__":
    unittest.main()
