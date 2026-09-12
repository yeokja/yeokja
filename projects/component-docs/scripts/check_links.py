from html.parser import HTMLParser
from pathlib import Path
from urllib.parse import urlsplit, unquote
from collections import Counter
import sys

class Page(HTMLParser):
 def __init__(self,p):
  super().__init__(); self.ids=set(); self.links=[]; self.feed(p.read_text())
 def handle_starttag(self,tag,attrs):
  a=dict(attrs)
  if a.get('id'): self.ids.add(a['id'])
  if tag=='a' and a.get('name'): self.ids.add(a['name'])
  for k in ('href','src'):
   if a.get(k): self.links.append(a[k])

def broken(root):
 root=Path(root).resolve(); pages={p.resolve():Page(p) for p in root.rglob('*.html')}; missing=set()
 for path,page in pages.items():
  for link in page.links:
   u=urlsplit(link)
   if u.scheme or u.netloc or u.path.startswith('/'): continue
   target=(path.parent/unquote(u.path)).resolve() if u.path else path
   if target.is_dir(): target/= 'index.html'
   if not target.is_relative_to(root): continue
   if not target.exists(): missing.add((str(path.relative_to(root)),link,'file'))
   elif u.fragment and target in pages and unquote(u.fragment) not in pages[target].ids:
    missing.add((str(path.relative_to(root)),link,'fragment'))
 return missing
base=broken(sys.argv[1]); ko=broken(sys.argv[2]); new=ko-base
print('Original unresolved local links:',len(base),'Korean:',len(ko),'New:',len(new))
for item in sorted(new)[:60]: print(item)
sys.exit(bool(new))
