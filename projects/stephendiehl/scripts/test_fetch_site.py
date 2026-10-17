import shutil
import unittest

import fetch_site


POST = """<html><body><nav>x</nav>
<main
            class="flex-grow overflow-x-hidden"
          >
            <div class="post-meta" style="display: none">
              <div class="post-meta-row">
                <span class="post-date">July 7, 2013</span>
                <button id="reading-mode-toggle"><span>Reading Mode</span></button>
              </div>
              <div class="post-tags">
                <a href="/tags/haskell/"
                  >haskell</a
                >
                <a href="/tags/category theory/"
                  >category theory</a
                >
              </div>
            </div>
              <h1 id="monads-made-difficult" tabindex="-1">Monads Made Difficult</h1>
<p>A short introduction.<br>
It assumes Haskell. See <a href="/posts/adjunctions/">adjunctions</a>, <a href="/posts/web3/">web3</a> and <a href="/writings/">writings</a>.</p>
<p>Category $\\mathcal{C}$ and <code>f $ x</code>.</p>
<img src="/images/nat.svg" loading="lazy" decoding="async" width="150">
<pre><code class="language-haskell"><span class="token keyword">type</span> Hask <span class="token operator">=</span> (-&gt;)
</code></pre>
<h5 id="categories" tabindex="-1">Categories</h5>
<p>Energy \\(E\\) is $$E = mc^2$$</p>
          </main>
<footer>y</footer></body></html>"""


class ParsingTests(unittest.TestCase):
    def test_sitemap_paths_and_lastmod(self):
        xml = ("<urlset><url><loc>https://www.stephendiehl.com/posts/monads/</loc>"
               "<lastmod>2024-01-02</lastmod></url><url><loc>https://www.stephendiehl.com/</loc></url></urlset>")
        self.assertEqual(fetch_site.parse_sitemap(xml), {"/posts/monads/": "2024-01-02", "/": None})

    def test_post_index_keeps_order_and_parses_dates(self):
        page = ('<li><a href="/posts/b/">B &amp; C</a>\n<time>October 11, 2026</time></li>'
                '<li><a href="/posts/a/">A</a><time>April 21, 2013</time></li>')
        posts = fetch_site.parse_post_index(page)
        self.assertEqual([p["slug"] for p in posts], ["b", "a"])
        self.assertEqual(posts[0], {"path": "/posts/b/", "slug": "b", "title": "B & C", "date": "2026-10-11"})

    def test_iso_date_rejects_garbage(self):
        self.assertEqual(fetch_site.iso_date("May 1, 2018"), "2018-05-01")
        with self.assertRaises(ValueError):
            fetch_site.iso_date("Sometime")

    def test_post_meta_is_split_off(self):
        date, tags, rest = fetch_site.post_meta(fetch_site.main_content(POST))
        self.assertEqual(date, "2013-07-07")
        self.assertEqual(tags, ["haskell", "category theory"])
        self.assertNotIn("post-meta", rest)
        self.assertIn("<h1", rest)

    def test_pages_without_meta_pass_through(self):
        date, tags, rest = fetch_site.post_meta("<p>Hi</p>")
        self.assertEqual((date, tags, rest), (None, [], "<p>Hi</p>"))

    def test_cache_name(self):
        self.assertEqual(fetch_site.cache_name("/posts/monads/"), "posts_monads.html")
        self.assertEqual(fetch_site.cache_name("/"), "index.html")


class PrepareTests(unittest.TestCase):
    def test_links_in_scope_become_relative_and_others_absolute(self):
        _, _, rest = fetch_site.post_meta(fetch_site.main_content(POST))
        out = fetch_site.prepare_html(rest, {"monads", "adjunctions"})
        self.assertIn('href="adjunctions.md"', out)
        self.assertIn('href="https://www.stephendiehl.com/posts/web3/"', out)
        self.assertIn('href="https://www.stephendiehl.com/writings/"', out)
        self.assertIn('src="https://www.stephendiehl.com/images/nat.svg"', out)
        self.assertNotIn("loading=", out)
        self.assertNotIn("tabindex", out)
        self.assertIn("<h1>Monads Made Difficult</h1>", out)
        self.assertIn("A short introduction.\nIt assumes", out)

    def test_fragment_links_and_external_links_are_untouched(self):
        html = '<a href="#top">t</a> <a href="https://example.com/x">e</a> <a href="/posts/a/#sec">s</a>'
        out = fetch_site.rewrite_links(html, {"a"})
        self.assertEqual(out, '<a href="#top">t</a> <a href="https://example.com/x">e</a> <a href="a.md#sec">s</a>')

    def test_in_page_links_follow_mdbook_heading_ids(self):
        main = ('<h2 id="from-homebrew-(macos)" tabindex="-1">From Homebrew (macOS)</h2>'
                '<h2 id="fr%C3%A9chet-derivative">Fr&eacute;chet <code>Derivative</code></h2>'
                '<p><a href="#from-homebrew-(macos)">a</a> <a href="#fr%C3%A9chet-derivative">b</a> '
                '<a href="#frechet-derivative">c</a></p>')
        self.assertEqual(fetch_site.mdbook_id("Using <code>llvm.sh</code> &amp; Co_2"), "using-llvmsh--co_2")
        out = fetch_site.prepare_html(main, set())
        self.assertIn('href="#from-homebrew-macos"', out)
        self.assertIn('href="#fréchet-derivative"', out)
        self.assertIn('href="#frechet-derivative"', out)  # broken on the original site too

    def test_old_posts_get_their_h3_title_promoted(self):
        main = '<h3 id="x" tabindex="-1">Dive into GHC: Pipeline</h3>\n<p>Body</p><h3>Later</h3>'
        self.assertEqual(fetch_site.ensure_h1(main, "Dive into GHC: Pipeline"),
                         "<h1>Dive into GHC: Pipeline</h1>\n<p>Body</p><h3>Later</h3>")
        self.assertEqual(fetch_site.ensure_h1("<p>Body</p>", "A & B"), "<h1>A &amp; B</h1>\n<p>Body</p>")
        self.assertEqual(fetch_site.ensure_h1("<h1>T</h1><h3>T</h3>", "T"), "<h1>T</h1><h3>T</h3>")

    def test_unlabelled_code_blocks_become_text_fences(self):
        out = fetch_site.prepare_html("<pre><code>a $ b\n</code></pre>", set())
        self.assertEqual(out, '<pre><code class="language-text">a $ b\n</code></pre>')

    def test_dollars_in_raw_html_blocks_are_entities(self):
        text = "Costs $5 and $x$.\n\n<table>\n<tr><td>$3.99</td></tr>\n<tr><td>$2.99</td></tr>\n</table>\n\n``` text\n$ echo\n```\n<img src=\"a\">\n"
        out = fetch_site.escape_dollars_in_html_blocks(text)
        self.assertIn("Costs $5 and $x$.", out)
        self.assertIn("<tr><td>&#36;3.99</td></tr>\n<tr><td>&#36;2.99</td></tr>", out)
        self.assertIn("``` text\n$ echo\n```", out)
        self.assertTrue(out.endswith('<img src="a">\n'))

    def test_front_matter(self):
        text = fetch_site.front_matter({"date": "2013-07-07", "tags": ["a b", "c"], "url": "u", "lastmod": None})
        self.assertEqual(text, '---\ndate: "2013-07-07"\ntags: ["a b", "c"]\nurl: "u"\n---\n\n')


@unittest.skipUnless(shutil.which("pandoc"), "pandoc is not installed")
class PandocTests(unittest.TestCase):
    def test_post_converts_to_markdown_with_math_and_fences(self):
        post = {"path": "/posts/monads/", "slug": "monads", "title": "x", "date": "2000-01-01"}
        text, meta = fetch_site.convert_post(post, POST.encode(), {"monads", "adjunctions"}, "2024-01-02")
        self.assertTrue(text.startswith('---\ndate: "2013-07-07"\ntags: ["haskell", "category theory"]\n'))
        self.assertIn("# Monads Made Difficult\n", text)
        self.assertIn("A short introduction. It assumes Haskell. See [adjunctions](adjunctions.md)", text)
        self.assertIn("Category $\\mathcal{C}$ and `f $ x`.", text)
        self.assertIn("``` haskell\ntype Hask = (->)\n```", text)
        self.assertIn('<img src="https://www.stephendiehl.com/images/nat.svg" width="150" />', text)
        self.assertIn("Energy $E$ is $$E = mc^2$$", text)
        self.assertNotIn("Reading Mode", text)
        self.assertEqual(meta["title"], "Monads Made Difficult")
        self.assertEqual(meta["lastmod"], "2024-01-02")
        self.assertEqual(meta["tags"], ["haskell", "category theory"])
        self.assertEqual(len(meta["html_sha256"]), 64)
        self.assertTrue(text.endswith("\n") and not text.endswith("\n\n"))


if __name__ == "__main__":
    unittest.main()
