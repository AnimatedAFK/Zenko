"""golden/map.json: zenko.py's map paintings (jmap_scene, kyoto_scene in each season, myth_scene, asia_scene for
Korea, China and the Silk Road) at a spread of sizes, as one FNV-1a 64 hash per pixel row plus the projection."""
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


CASES = [("japan", 200, 165, None), ("japan", 331, 270, None), ("japan", 420, 330, None),
         ("kyoto", 200, 165, "spring"), ("kyoto", 260, 200, "summer"), ("kyoto", 300, 248, "autumn"), ("kyoto", 331, 270, "winter"),
         ("myth", 200, 165, None), ("myth", 331, 270, None),
         ("korea", 200, 165, None), ("china", 300, 248, None), ("silk", 331, 270, None), ("korea", 420, 330, None)]
out = []
for region, w, h, season in CASES:
    if region == "japan":
        R, P = Z._jmap_scene_raw(w, h) if hasattr(Z, "_jmap_scene_raw") else Z.jmap_scene(w, h)
    elif region == "kyoto":
        R, P = Z.kyoto_scene(w, h, season)
    elif region == "myth":
        R, P = Z.myth_scene(w, h)
    else:
        R, P = Z.asia_scene(w, h, region)
    row = w * 3
    pts = [list(P.xy(*p[4:6])) if region != "myth" else list(P.xy(*p[4:6])) for p in Z.JOURNEY_ALL[:6]]
    out.append({"args": [region, w, h, season], "rows": [fnv(R.b[y * row:(y + 1) * row]) for y in range(h)],
                "s": P.s, "inset": list(P.inset), "pts": pts})
with open(os.path.join(os.path.dirname(__file__), "..", "golden", "map.json"), "w") as f:
    json.dump(out, f, indent=0)
print("ok", len(out))
