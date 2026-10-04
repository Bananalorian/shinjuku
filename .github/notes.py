#!/usr/bin/env python3
"""Release notes: the CHANGELOG.md section for this tag, plus how to play."""
import re, sys

tag, repo = sys.argv[1], sys.argv[2]
version = tag.lstrip("v")
owner, name = repo.split("/")
text = open("CHANGELOG.md", encoding="utf-8").read()
m = re.search(rf"^## \[{re.escape(version)}\][^\n]*\n(.*?)(?=^## \[|\Z)", text, re.S | re.M)
body = m.group(1).strip() if m else "See CHANGELOG.md."
print(f"""{body}

---

### Play

- **In your browser:** https://{owner.lower()}.github.io/{name}/
- **Offline in a browser:** download `last-train-to-shinjuku-browser.html` and open it.
- **Desktop:** grab the zip for Windows, macOS or Linux below.
""")
