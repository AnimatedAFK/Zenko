"""UI-shape oracle: imports the ORIGINAL zenko.py (never modified) and records round_pts (the pixel-stepped box outline
used by the speech bubble, the timer panel, chips and buttons) for many sizes, radii, steps and shapes.
Run from the repo root:  python RUST_REWRITE/tools/make_golden_ui.py  -> RUST_REWRITE/golden/ui_shapes.json"""
import sys
sys.dont_write_bytecode = True
import json, os, tempfile

ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
os.environ["APPDATA"] = tempfile.mkdtemp()
sys.path.insert(0, ROOT)
import zenko as Z  # noqa: E402

out = []
for shape in ("round", "ema", "leaf", "pill"):
    for (w, h) in ((40, 22), (120, 60), (231, 97), (9, 7), (300, 40)):
        for r in (0, 3, 10, 16, 21, 32, 99):
            for step in (2, 3, 4):
                x1, y1 = 19, 23
                pts = Z.round_pts(x1, y1, x1 + w, y1 + h, r, step, Z.SHAPES[shape](r))
                out.append({"shape": shape, "x1": x1, "y1": y1, "x2": x1 + w, "y2": y1 + h, "r": r, "step": step,
                            "pts": pts})
p = os.path.join(ROOT, "RUST_REWRITE", "golden", "ui_shapes.json")
with open(p, "w") as f:
    json.dump(out, f)
print(len(out), "shapes ->", p)
