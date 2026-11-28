"""Copies the fonts zenko.py carries in _FONT_BLOB (Zen Maru Gothic + Zen Old Mincho, SIL Open Font License) into
RUST_REWRITE/assets/fonts, so ZenkoRust.exe can carry the very same fonts. Reads zenko.py as text; never imports it.
Run from the repo root:  python RUST_REWRITE/tools/extract_fonts.py"""
import base64, json, lzma, os

ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
src = open(os.path.join(ROOT, "zenko.py"), encoding="utf-8").read()
i = src.index("_FONT_BLOB = (")
j = src.index("\n)", i)
blob = eval(src[i + len("_FONT_BLOB = "):j + 2], {})
raw = lzma.decompress(base64.b85decode("".join(blob)))
n = int.from_bytes(raw[:4], "big")
idx = json.loads(raw[4:4 + n].decode("utf-8"))
off = 4 + n
out = os.path.join(ROOT, "RUST_REWRITE", "assets", "fonts")
os.makedirs(out, exist_ok=True)
for name, size in idx:
    with open(os.path.join(out, name), "wb") as f:
        f.write(raw[off:off + size])
    off += size
    print(name, size)
