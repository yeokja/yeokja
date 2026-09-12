"""Prepare a local mdBook copy for the translated Pages build."""
from pathlib import Path
import re
import posixpath
import sys


def write_local(path, text):
    # assemble links files to upstream/ko; replace the link, never its target.
    temporary = path.with_name(path.name + ".preparing")
    temporary.write_text(text)
    temporary.replace(path)


book = Path(sys.argv[1]) / "book.toml"
text = book.read_text()
# External link checking is a separate concern from producing reproducible HTML.
text = text.replace("[output.linkcheck2]", "")
# Root-relative redirects escape the component-docs subdirectory on Pages.
def relative_redirect(match):
    source, target = match.groups()
    target = posixpath.relpath(target.lstrip("/"), posixpath.dirname(source.lstrip("/")) or ".")
    return f'"{source}" = "{target}"'

text = re.sub(r'^"(/[^"\n]+)" = "(/[^"\n]+)"$', relative_redirect, text, flags=re.M)
write_local(book, text)

# This translation must not send visits to the upstream analytics account.
write_local(book.parent / "theme" / "head.hbs", "")

# Yeokja joins prose lines; mdbook-tabs requires directives on separate lines.
# Restore their block boundaries without changing directive arguments or prose.
for source in (book.parent / "src").rglob("*.md"):
    text = source.read_text()
    text = re.sub(r"(\{\{#(?:tabs|tab|endtab|endtabs)\b[^\n]*?\}\})",
                  r"\n\n\1\n\n", text)
    write_local(source, text)
