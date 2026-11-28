"""crates/zenko-desktop/data/gym.json: the Exercise page's tables (the split days with their muscle groups, the body
map's regions, inner lines, muscle names and kitsune-mask pixels), taken from the untouched zenko.py."""
import sys
sys.dont_write_bytecode = True
import json, os, tempfile
ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
os.environ["APPDATA"] = tempfile.mkdtemp()
sys.path.insert(0, ROOT)
import zenko as Z  # noqa: E402

data = {
    "GYM_ALL": [{"kanji": d["kanji"], "big": d["big"], "name": d["name"], "targets": d["targets"],
                 "groups": [[n, k, list(r)] for n, k, r in d["groups"]]} for d in Z.GYM_ALL],
    "BODY_W": Z.BODY_W, "BODY_H": Z.BODY_H, "BODY_OX": Z.BODY_OX, "BODY_OY": Z.BODY_OY,
    "BODY_FRONT": [[n, [list(p) for p in poly]] for n, poly in Z.BODY_FRONT],
    "BODY_BACK": [[n, [list(p) for p in poly]] for n, poly in Z.BODY_BACK],
    "BODY_LINES": {k: [[list(a), list(b)] for a, b in v] for k, v in Z.BODY_LINES.items()},
    "MUSCLE_NAMES": Z.MUSCLE_NAMES,
    "MASK_PIX": [[x, y, list(c)] for x, y, c in Z.MASK_PIX],
}
with open(os.path.join(ROOT, "RUST_REWRITE", "crates", "zenko-desktop", "data", "gym.json"), "w") as f:
    json.dump(data, f, ensure_ascii=False)
print("ok", len(data["BODY_FRONT"]), len(data["MASK_PIX"]))
