"""golden/breath.json: zenko.py _breath_tone for every phase, length and volume used, as a SHA-256 of the .wav."""
import sys
sys.dont_write_bytecode = True
import hashlib, json, os, tempfile
sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", ".."))
os.environ.setdefault("APPDATA", tempfile.mkdtemp())
import zenko as Z  # noqa: E402

out = []
for kind, secs, vol in [("in", 4, 2), ("hold", 4, 2), ("out", 4, 2), ("rest", 4, 1), ("in", 4, 3), ("hold", 7, 2),
                        ("out", 8, 2), ("in", 5, 2), ("out", 5, 3), ("in", 2, 2), ("top", 1, 2), ("out", 6, 2)]:
    b = Z._breath_tone(kind, secs, vol)
    out.append({"kind": kind, "secs": secs, "vol": vol, "len": len(b), "sha": hashlib.sha256(b).hexdigest()})
with open(os.path.join(os.path.dirname(__file__), "..", "golden", "breath.json"), "w") as f:
    json.dump(out, f, indent=0)
print("ok", len(out))
