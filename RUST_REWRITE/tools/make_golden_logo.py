"""golden/hub_logo.json: the hub sidebar's living logo exactly as zenko.py draws it (hub_anim with Zenko19's page poses):
for each page and a few clock readings, the mood and the pictures of the tails and body (exact RGBA).
Run: xvfb-run -a -s "-screen 0 1920x1080x24" /usr/bin/python3.12 tools/make_golden_logo.py
Old header: golden/hub_win.json: the hub window's frame exactly as zenko.py builds it (the shrine border, title bar with the
search pill and window buttons, sidebar canvas, tip bar and the page stage). The real app (Zenko23) runs in real Tk
under Xvfb with Windows' widget defaults and a pinned clock; the whole widget tree is dumped (class, place in the
window, requested size, every option) with every canvas item (sidebar items are in golden/hub_side.json) and the
pixels of every picture. The page inside the stage is left out (each page has its own golden file).
Run: xvfb-run -a -s "-screen 0 1920x1080x24" /usr/bin/python3.12 tools/make_golden_hubwin.py"""
import sys
sys.dont_write_bytecode = True
import json, os, tempfile, types
os.environ["TZ"] = "UTC"
import time  # noqa: E402
time.tzset()
ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
os.environ["APPDATA"] = tempfile.mkdtemp()
sys.path.insert(0, ROOT)
import zenko as Z  # noqa: E402
import tkinter as tk  # noqa: E402
import tkinter.font as tkfont  # noqa: E402
import datetime as _dt  # noqa: E402

NOW = [0.0]


class FDT(_dt.datetime):
    @classmethod
    def now(cls, tz=None):
        return cls.utcfromtimestamp(NOW[0]) if tz is None else _dt.datetime.fromtimestamp(NOW[0], tz)


class FD(_dt.date):
    @classmethod
    def today(cls):
        return _dt.datetime.utcfromtimestamp(NOW[0]).date()


fake = types.ModuleType("datetime")
fake.__dict__.update(_dt.__dict__)
fake.datetime, fake.date = FDT, FD
Z.datetime = fake
Z.ui_sound = lambda *a, **k: None
Z.beep = lambda *a, **k: None
REG, LAST = {}, [None]
_png0 = Z._png


def _png_rec(px, *a, **k):
    LAST[0] = px
    return _png0(px, *a, **k)


Z._png = _png_rec
for _fn in ("aa_photo", "sprite_photo"):
    def _wrap(*a, _orig=getattr(Z, _fn), **k):
        LAST[0] = None
        img = _orig(*a, **k)
        if LAST[0] is not None:
            REG[str(img)] = [["#%02x%02x%02x%02x" % tuple(p) for p in row] for row in LAST[0]]
        return img
    setattr(Z, _fn, _wrap)


def photo_px(tkapp, nm):
    w, h = int(tkapp.call("image", "width", nm)), int(tkapp.call("image", "height", nm))
    rows = []
    for y in range(h):
        row = []
        for x in range(w):
            if tkapp.getboolean(tkapp.call(nm, "transparency", "get", x, y)):
                row.append("#00000000")
            else:
                r, g, b = (int(v) for v in tkapp.splitlist(tkapp.call(nm, "get", x, y)))
                row.append("#%02x%02x%02xff" % (r, g, b))
        rows.append(row)
    return rows


def pic(cv, nm):
    return REG.get(nm) or photo_px(cv.tk, nm)     # a picture made earlier without _png: Tk's own copy (alpha 0 / 255)


MONO = [0.0]
time.monotonic = lambda: MONO[0]
out = []
NOW[0] = 1744970400.0                     # 2025-04-18 10:00 (spring)
app = Z.Zenko23()
app.root.update()
app.open_hub("home")
for _ in range(6):
    app.root.update()
h = app.hub
for page in ["home", "notebook", "exercise", "garden", "map", "settings", "customize", "you", "chat", "screen",
             "deadlines", "timetable", "reminders", "daily"]:
    h["page"] = page
    for t in (5000.0, 5000.37, 5001.9, 5003.3):
        MONO[0] = t
        h["blink_at"], h["blink_until"] = t + 100, 0.0
        app._z20_last_anim = -10.0
        h["logo_key"] = None
        Z.Zenko19.hub_anim(app)
        ti, bi = h["logo_items"]
        cv = h["snow"]
        rec = {"page": page, "now": t, "mono": t, "mood": app.current_mood(), "acc": list(app.accessories()),
               "tails": None if cv.itemcget(ti, "state") == "hidden" else pic(cv, cv.itemcget(ti, "image")),
               "body": pic(cv, cv.itemcget(bi, "image")), "coords": [cv.coords(ti), cv.coords(bi)],
               "tail_count": app.tail_count(), "session": bool(app.session)}
        out.append(rec)
with open(os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "golden", "hub_logo.json"), "w") as f:
    json.dump(out, f)
for r in out:
    print(r["page"], r["now"], r["mood"], r["acc"], r["tails"] is not None, r["body"] is not None and len(r["body"]))
os._exit(0)
