"""golden/garden.json: zenko.py garden_scene (the z18 painter + z19 season/sky) for a spread of sizes, stages, health,
grove, seeds, seasons, parts of day and weather, as one FNV-1a 64 hash per pixel row plus the layout."""
import sys
sys.dont_write_bytecode = True
import json, os, tempfile
sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", ".."))
os.environ.setdefault("APPDATA", tempfile.mkdtemp())
import zenko as Z  # noqa: E402


def fnv(b):
    h = 0xcbf29ce484222325
    for x in b:
        h = ((h ^ x) * 0x100000001b3) & 0xffffffffffffffff
    return "%016x" % h


CASES = [
    # w, h, stage, growth, health, grove, seed, season, part, weather, look season, look sky
    (210, 125, 0, 0.0, 1.0, 0, 7, "spring", "night", None, True, True),
    (210, 125, 1, 0.5, 0.6, 0, 11, "summer", "day", None, True, True),
    (240, 143, 2, 0.3, 1.0, 1, 123456, "autumn", "dusk", None, True, True),
    (300, 179, 3, 0.8, 0.4, 2, 99, "winter", "dawn", None, True, True),
    (280, 167, 4, 0.1, 0.9, 0, 5, "spring", "night", "rain", True, True),
    (320, 190, 5, 0.2, 1.0, 3, 42, "summer", "night", None, True, True),
    (320, 190, 5, 0.7, 0.8, 3, 42, "spring", "day", None, True, True),
    (420, 250, 6, 0.5, 1.0, 4, 2024, "spring", "night", None, True, True),
    (360, 214, 7, 0.9, 0.3, 5, 777, "autumn", "day", "snow", True, True),
    (420, 250, 8, 1.0, 1.0, 7, 31337, "winter", "night", None, True, True),
    (400, 238, 8, 0.4, 0.7, 6, 8, "summer", "dusk", None, False, True),
    (260, 155, 6, 0.0, 0.0, 0, 3, "spring", "day", None, False, False),
]
out = []
for (w, h, st, g, hp, gr, seed, sn, part, wx, ls, lk) in CASES:
    Z.Z19_LOOK.update({"season": ls, "sky": lk, "season_name": sn, "part": part, "weather": wx})
    R, lay = Z.garden_scene(w, h, st, g, hp, grove=gr, seed=seed)
    row = w * 3
    out.append({"args": [w, h, st, g, hp, gr, seed, sn, part, wx, ls, lk],
                "rows": [fnv(R.b[y * row:(y + 1) * row]) for y in range(h)],
                "tips": [list(t) for t in lay["tips"]], "lanterns": [list(t) for t in lay["lanterns"]],
                "pond": list(lay["pond"]), "moon": list(lay["moon"]), "basin": list(lay["basin"]),
                "wall": lay["wall"], "horizon": lay["horizon"], "tree": list(lay["tree"]), "breath": list(lay["breath"])})
with open(os.path.join(os.path.dirname(__file__), "..", "golden", "garden.json"), "w") as f:
    json.dump(out, f, indent=0)
print("ok", len(out))
