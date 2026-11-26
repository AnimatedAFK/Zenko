"""golden/icon.json: zenko.py icon_pixels(64, theme) for every fur theme (the window icon), as hex rows."""
import sys
sys.dont_write_bytecode = True
import json, os, tempfile
sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", ".."))
os.environ.setdefault("APPDATA", tempfile.mkdtemp())
import zenko as Z  # noqa: E402

out = {}
for th in ["snow", "classic", "ember", "fennec", "midnight", "sakura", "mint", "silver"]:
    px = Z.icon_pixels(64, th)
    out[th] = ["".join("%02x%02x%02x%02x" % p for p in row) for row in px]
with open(os.path.join(os.path.dirname(__file__), "..", "golden", "icon.json"), "w") as f:
    json.dump(out, f, indent=0)
print("ok")
