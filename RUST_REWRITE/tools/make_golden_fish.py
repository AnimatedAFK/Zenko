"""golden/fish.json: zenko.py fish_rgba for every fish (book size, pond size turned and squashed, tiny), as one
FNV-1a 64 hash per pixel row, plus fish_pattern's blob count."""
import sys
sys.dont_write_bytecode = True
import json, math, os, tempfile
sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", ".."))
os.environ.setdefault("APPDATA", tempfile.mkdtemp())
import zenko as Z  # noqa: E402


def fnv(b):
    h = 0xcbf29ce484222325
    for x in b:
        h = ((h ^ x) * 0x100000001b3) & 0xffffffffffffffff
    return "%016x" % h


out = []
for i, f in enumerate(Z.FISH):
    fid = f[0]
    BL = {"medaka": 52, "gold": 76, "oar": 150, "catfish": 104}.get(f[4], 96)
    cases = [(1, BL, 0.0, 1.0), (1 + i * 7919, 40, (i % 16) * math.pi / 8, 0.5), (999999 - i, 24, 0.0, 1.0),
             (i + 3, 16, 3 * math.pi / 8, 0.5), (12345, 130, 0.0, 1.0), (i * 31, 60, math.pi, 0.5)]
    for seed, L, ang, sq in cases:
        px = Z.fish_rgba(fid, seed, L, ang, sq)
        out.append({"fid": fid, "seed": seed, "L": L, "angle": ang, "squash": sq, "w": len(px[0]), "h": len(px),
                    "rows": [fnv(bytes(c for p in r for c in p)) for r in px],
                    "blobs": len(Z.fish_pattern(f[5], seed)["blobs"])})
with open(os.path.join(os.path.dirname(__file__), "..", "golden", "fish.json"), "w") as fo:
    json.dump(out, fo, indent=0)
print("ok", len(out))
