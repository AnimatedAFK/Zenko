"""crates/zenko-desktop/data/rewards.json: zenko.py's ema badges (plain, tiered, secret), garden stages, journey places
and regions, map rewards, the zenko22 accent colours and the subject ids, taken from the untouched zenko.py."""
import sys
sys.dont_write_bytecode = True
import json, os, tempfile
ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
os.environ["APPDATA"] = tempfile.mkdtemp()
sys.path.insert(0, ROOT)
import zenko as Z  # noqa: E402

data = {
    "BADGES": [list(b[:4]) for b in Z.BADGES],
    "BADGE_TIERED": [list(b) for b in Z.BADGE_TIERED],
    "BADGE_TIER_NAME": Z.BADGE_TIER_NAME,
    "BADGE_SECRET": sorted(Z.BADGE_SECRET),
    "MOODS": [list(m) for m in Z.MOODS],
    "GARDEN_STAGES": [list(s[:4]) for s in Z.GARDEN_STAGES],
    "JOURNEY_ALL": [[p[0], p[1], p[2], p[6], p[9]] for p in Z.JOURNEY_ALL],
    "JMAP_PLACES": [p[0] for p in Z.JMAP_PLACES],
    "JOURNEY_REGIONS": [r[0] for r in Z.JOURNEY_REGIONS],
    "JOURNEY_REGION_OF": Z.JOURNEY_REGION_OF,
    "MAP_REWARDS": [list(r[:4]) for r in Z.MAP_REWARDS],
    "Z22_ACCENTS": [[a[0], a[1], list(a[2])] for a in Z.Z22_ACCENTS],
    "SUBJECT_IDS": [d[0] for d in Z.SUBJECT_DEFS],
    "DEFAULT_FREE": Z.DEFAULT_FREE,
}
out = os.path.join(os.path.dirname(__file__), "..", "crates", "zenko-desktop", "data", "rewards.json")
with open(out, "w", encoding="utf-8") as f:
    json.dump(data, f, ensure_ascii=False, indent=0)
print({k: len(v) for k, v in data.items()})
print(data["MAP_REWARDS"][:3], data["GARDEN_STAGES"][:2], data["JOURNEY_ALL"][:2])
