"""crates/zenko-desktop/data/garden.json: the Zen garden's data from zenko.py (fish, rods, lanterns, depths, question
bank, decor art, breathing patterns, grounding steps, orb tints)."""
import sys
sys.dont_write_bytecode = True
import json, os, tempfile
sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", ".."))
os.environ.setdefault("APPDATA", tempfile.mkdtemp())
import zenko as Z  # noqa: E402

out = {
    "FISH": [list(f) for f in Z.FISH], "FISH_RARITY": Z.FISH_RARITY, "FISH_RCOL": Z.FISH_RCOL,
    "FISH_RODS": Z.FISH_RODS, "FISH_LANTERNS": Z.FISH_LANTERNS, "FISH_DEPTHS": Z.FISH_DEPTHS,
    "FISH_Q_SUBJECTS": Z.FISH_Q_SUBJECTS, "FISH_QS": [list(q) for q in Z.FISH_QS], "FISH_SHAPE": Z.FISH_SHAPE,
    "Z22_DECOR_ART": {k: [v[0], v[1]] for k, v in Z.Z22_DECOR_ART.items()},
    "BREATH_PATTERNS": Z.BREATH_PATTERNS, "BREATH_ORDER": Z.BREATH_ORDER, "BREATH_PHASE": Z.BREATH_PHASE,
    "GROUNDING": Z.GROUNDING, "BREATH_TINT": Z.BREATH_TINT,
}
p = os.path.join(os.path.dirname(__file__), "..", "crates", "zenko-desktop", "data", "garden.json")
with open(p, "w", encoding="utf-8") as f:
    json.dump(out, f, ensure_ascii=False, separators=(",", ":"))
print("ok", os.path.getsize(p))
