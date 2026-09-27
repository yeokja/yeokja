import unittest
from relativize_redirects import relativize

STUB = '''<title>Redirecting...</title>
<meta http-equiv="refresh" content="0; URL=/release/backporting.html">
<link rel="canonical" href="/release/backporting.html">
<a href="/release/backporting.html">/release/backporting.html</a>
let redirectUrl = "/release/backporting.html";'''


class RedirectTests(unittest.TestCase):
    def test_root_absolute_targets_become_relative(self):
        result = relativize(STUB, 'release/beta-backporting.html')
        self.assertNotIn('"/release', result)
        self.assertNotIn('URL=/', result)
        self.assertEqual(result.count('href="backporting.html"'), 2)
        self.assertIn('URL=backporting.html', result)

    def test_top_level_stub_keeps_directory(self):
        self.assertIn('URL=release/backporting.html', relativize(STUB, 'beta-backporting.html'))

    def test_external_and_relative_targets_are_untouched(self):
        for target in ('https://example.org/a.html', 'borrow-check.html', '//cdn/x.html'):
            stub = STUB.replace('/release/backporting.html', target)
            self.assertEqual(relativize(stub, 'sub/a.html'), stub)

    def test_ordinary_pages_are_untouched(self):
        page = '<a href="/release/backporting.html">x</a>'
        self.assertEqual(relativize(page, 'a.html'), page)


if __name__ == '__main__':
    unittest.main()
