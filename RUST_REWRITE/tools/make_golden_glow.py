"""golden/glow.json: zenko.py's z19_glow (the focus glow in the hub window's frame and the petal burst after a
completed session), run on a stand-in canvas that records every create_* call, for several window sizes, themes,
progress, paused, and burst times. The burst's petals are given (zenko.py picks them with an unseeded Random).
Run: env PYTHONDONTWRITEBYTECODE=1 /usr/bin/python3.12 tools/make_golden_glow.py"""
import sys
sys.dont_write_bytecode = True
import json, os, tempfile, types
ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
os.environ["APPDATA"] = tempfile.mkdtemp()
sys.path.insert(0, ROOT)
import zenko as Z  # noqa: E402

MONO = [0.0]
Z.time.monotonic = lambda: MONO[0]


class Cv:
    def __init__(self, w, h):
        self.w, self.h, self.items = w, h, []

    def winfo_exists(self):
        return True

    def winfo_width(self):
        return self.w

    def winfo_height(self):
        return self.h

    def find_withtag(self, tag):
        return [i for i in self.items if tag in i.get("tags", "")]

    def delete(self, tag):
        self.items = [i for i in self.items if tag not in i.get("tags", "")]

    def _add(self, ty, a, k):
        self.items.append({"type": ty, "coords": [float(x) for x in a], **{x: (y if x != "width" else float(y))
                                                                             for x, y in k.items()}})

    def create_line(self, *a, **k):
        self._add("line", a, k)

    def create_oval(self, *a, **k):
        self._add("oval", a, k)

    def create_rectangle(self, *a, **k):
        self._add("rectangle", a, k)


PETALS = [(37.0 + 61.0 * k, -150.0 + 7.0 * k, ("#ffb7d5", "#ff8fbf", "#fff0f6", "GOLD")[k % 4], (1.5, 2.0, 2.5)[k % 3])
          for k in range(46)]
CASES = [("snow", 1120, 700, 300, 1500, False, 0.0, None), ("snow", 1120, 700, 1499, 1500, False, 3.1, None),
         ("night", 900, 640, 10, 1500, True, 1.0, None), ("sakura", 760, 520, 0.5, 1500, False, 2.0, None),
         ("matcha", 1000, 700, 3000, 1500, False, 4.4, None), ("night", 1120, 700, 700, 1500, False, 7.7, None),
         ("snow", 1120, 700, None, None, False, 0.0, 0.7), ("night", 900, 640, None, None, False, 0.0, 2.2),
         ("snow", 1000, 600, 600, 1500, False, 5.0, 1.3), ("snow", 1000, 600, None, None, False, 0.0, 2.7)]
out = []
for theme, w, h, el, pl, paused, mono, bt in CASES:
    Z.apply_theme(theme)
    fake = types.SimpleNamespace(hub={"border": Cv(w, h)}, cfg={}, SHU=Z.Zenko23.SHU, BORDER=Z.Zenko23.BORDER,
                                 session={"elapsed": el, "planned": pl, "paused": paused} if el is not None else None)
    petals = [(p, s, Z.GOLD if c == "GOLD" else c, z) for p, s, c, z in PETALS]
    fake._z19_burst = {"t0": 100.0, "petals": petals} if bt is not None else None
    MONO[0] = mono if bt is None else 100.0 + bt
    Z.Zenko23.z19_glow(fake)
    out.append({"theme": theme, "w": w, "h": h, "sess": [el, pl, paused] if el is not None else None, "now": MONO[0],
                "burst": bt, "petals": [[p / (2.0 * (w + h)), s, c, z] for p, s, c, z in petals],
                "items": fake.hub["border"].items, "burst_left": fake._z19_burst is not None})
with open(os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "golden", "glow.json"), "w") as f:
    json.dump(out, f, indent=0)
print([len(o["items"]) for o in out])
