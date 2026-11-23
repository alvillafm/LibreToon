"""Check local links, images, downloads and anchors in a built MkDocs site."""
import argparse
from html.parser import HTMLParser
from pathlib import Path
from urllib.parse import unquote, urlsplit


class Page(HTMLParser):
    def __init__(self, path):
        super().__init__()
        self.links = []
        self.ids = set()
        self.feed(path.read_text(encoding="utf-8"))

    def handle_starttag(self, tag, attrs):
        attrs = dict(attrs)
        if "id" in attrs:
            self.ids.add(attrs["id"])
        for key in ("href", "src"):
            if attrs.get(key):
                self.links.append(attrs[key])


def check(root):
    pages = {path.resolve(): Page(path) for path in root.rglob("*.html")}
    errors = []
    for source, page in pages.items():
        for link in page.links:
            url = urlsplit(link)
            if url.scheme or url.netloc or url.path.startswith("/"):
                continue
            target = (source.parent / unquote(url.path)).resolve() if url.path else source
            if target.is_dir():
                target /= "index.html"
            if not target.is_relative_to(root):
                errors.append(f"{source.relative_to(root)}: path outside site: {link}")
            elif not target.is_file():
                errors.append(f"{source.relative_to(root)}: missing file: {link}")
            elif url.fragment and target in pages and unquote(url.fragment) not in pages[target].ids:
                errors.append(f"{source.relative_to(root)}: missing anchor: {link}")
    return len(pages), errors


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("site", nargs="?", type=Path, default=Path("site"))
    args = parser.parse_args()
    root = args.site.resolve()
    if not (root / "index.html").is_file():
        parser.error("build the MkDocs site first")
    count, errors = check(root)
    for error in errors:
        print(error)
    print(f"Checked {count} pages: {len(errors)} broken local links, images, downloads or anchors.")
    raise SystemExit(bool(errors))
