"""crates/zenko-desktop/data/map.json: the World map's data from zenko.py (Japan's coast, peaks, places and labels,
the Asia views, sea, islands, lakes, deserts, plateau, ranges and rivers, the journey's regions, places, quizzes and
rewards)."""
import sys
sys.dont_write_bytecode = True
import json, os, tempfile
sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", ".."))
os.environ.setdefault("APPDATA", tempfile.mkdtemp())
import zenko as Z  # noqa: E402

L = lambda v: json.loads(json.dumps(v))
out = {
    "JMAP_LAND": {k: L(v) for k, v in Z.JMAP_LAND.items()}, "JMAP_LAND_ORDER": list(Z.JMAP_LAND.keys()),
    "JMAP_OKINAWA": L(Z.JMAP_OKINAWA), "JMAP_PEAKS": L(Z.JMAP_PEAKS), "JMAP_PLACES": L(Z.JMAP_PLACES),
    "JMAP_LABEL": L(Z.JMAP_LABEL), "JMAP_ICON_TORII": Z.JMAP_ICON_TORII,
    "Z22_VIEWS": L(Z.Z22_VIEWS), "Z22_SEA": L(Z.Z22_SEA), "Z22_ISLANDS": L(Z.Z22_ISLANDS), "Z22_LAKES": L(Z.Z22_LAKES),
    "Z22_DESERTS": L(Z.Z22_DESERTS), "Z22_HIGH": L(Z.Z22_HIGH), "Z22_RANGES": L(Z.Z22_RANGES), "Z22_RIVERS": L(Z.Z22_RIVERS),
    "JOURNEY_REGIONS": L(Z.JOURNEY_REGIONS), "JOURNEY_ALL": L(Z.JOURNEY_ALL), "JOURNEY_REGION_OF": L(Z.JOURNEY_REGION_OF),
    "JOURNEY_QUIZ": L(Z.JOURNEY_QUIZ), "MAP_REWARDS": L(Z.MAP_REWARDS),
}
for k in dir(Z):
    if (k.startswith("Z22_") or k.startswith("JMAP_") or k.startswith("MAP_") or k.startswith("JOURNEY_")) and k not in out:
        v = getattr(Z, k)
        try:
            out[k] = L(v)
        except (TypeError, ValueError):
            pass
p = os.path.join(os.path.dirname(__file__), "..", "crates", "zenko-desktop", "data", "map.json")
with open(p, "w", encoding="utf-8") as f:
    json.dump(out, f, ensure_ascii=False, separators=(",", ":"))
print("ok", os.path.getsize(p), sorted(out))
