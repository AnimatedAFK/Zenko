"""golden/hub_side.json: the hub's night-sky sidebar exactly as zenko.py draws it. The real app (Zenko23) runs in real
Tk under Xvfb with a pinned date and time; for each case it opens the hub, and the sidebar canvas is dumped item by
item (type, coordinates, every option, tags, stacking order), with the pixels of every picture on it and the font
numbers for every text. Falling snow/petals ("flake") and the living logo move every frame, so they are left out
here. The Rust test (tests/hub_side.rs) draws the same sidebar and must match all of it.
Run: xvfb-run -a -s "-screen 0 1920x1080x24" /usr/bin/python3.12 tools/make_golden_hub.py"""
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
# every picture made with aa_photo / sprite_photo is kept with its exact RGBA rows (Tk 8.6 can't read alpha back)
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

CASES = [
    # (name, unix time, cfg changes, window size, page, hover)
    ("winter_night_snow", 1765407600.0, {"theme": "snow"}, None, "home", None),                 # 2025-12-10 23:00
    ("spring_day_sakura", 1744970400.0, {"theme": "sakura"}, None, "chat", "timetable"),        # 2025-04-18 10:00
    ("autumn_dusk_midnight", 1760461200.0, {"theme": "midnight", "nav_compact": True}, (1100, 720), "settings", None),  # 2025-10-14 17:00
    ("summer_dawn_small", 1751346000.0, {"theme": "ember", "user_name": "Mio"}, (1000, 660), "garden", "home"),  # 2025-07-01 05:00
]


def opts(cv, i):
    o = {}
    for k, v in cv.itemconfigure(i).items():
        if k in ("tags",):
            continue
        o[k] = v[-1]
    return o


def photo_px(tkapp, nm):
    w, h = int(tkapp.call("image", "width", nm)), int(tkapp.call("image", "height", nm))
    px = []
    for y in range(h):
        row = []
        for x in range(w):
            if tkapp.getboolean(tkapp.call(nm, "transparency", "get", x, y)):
                row.append(None)
            else:
                r, g, b = (int(v) for v in tkapp.splitlist(tkapp.call(nm, "get", x, y)))
                row.append("#%02x%02x%02x" % (r, g, b))
        px.append(row)
    return {"w": w, "h": h, "px": px}


out = []
for name, t, cfg, size, page, hover in CASES:
    NOW[0] = t
    app = Z.Zenko23()
    app.cfg.update(cfg)
    Z.apply_theme(app.cfg.get("theme", "snow"))
    app.root.update()
    app.open_hub(page)
    h = app.hub
    w = h["win"]
    if size:
        w.geometry("%dx%d+0+0" % size)
    for _ in range(6):
        app.root.update()
        time.sleep(0.05)
    if hover:
        app.hub_hover(hover, True)
    for _ in range(100):                      # let the menu pill finish gliding to the page
        if h.get("pill_to") is None or h.get("pill_y") is None or abs(h["pill_to"] - h["pill_y"]) <= 0.5:
            break
        app.root.update()
        time.sleep(0.03)
    app.hub_side_draw()                       # dumped straight away: no animation step in between
    cv = h["snow"]
    items, images, fonts = [], {}, {}
    for i in cv.find_all():
        tags = list(cv.gettags(i))
        if "flake" in tags or "logo" in tags:
            continue
        ty = cv.type(i)
        o = opts(cv, i)
        it = {"type": ty, "coords": cv.coords(i), "tags": tags, "opts": o, "bbox": list(cv.bbox(i) or [])}
        if ty == "image" and o.get("image"):
            nm = o["image"]
            if nm not in images:
                images[nm] = {"rgba": REG[nm]} if nm in REG else photo_px(cv.tk, nm)
        if ty == "text":
            f = tkfont.Font(font=o["font"])
            key = o["font"]
            ent = fonts.setdefault(key, {"ascent": f.metrics("ascent"), "descent": f.metrics("descent"),
                                         "actual": f.actual(), "w": {}})
            for line in o["text"].split("\n"):
                ent["w"][line] = f.measure(line)
                for k in range(len(line) + 1):
                    ent["w"][line[:k]] = f.measure(line[:k])
                    if float(o.get("width") or 0) > 0:        # wrapped text: every piece a wrapped line can be
                        for j in range(k):
                            ent["w"].setdefault(line[j:k], f.measure(line[j:k]))
        items.append(it)
    st = Z.Z19_LOOK
    out.append({"name": name, "now": t, "cfg": app.cfg.get("theme"), "cfg_all": cfg, "page": h.get("page"), "hover": hover,
                "W": cv.winfo_width(), "H": cv.winfo_height(), "layout": list(app.hub_side_layout()),
                "look": {k: st.get(k) for k in ("season", "sky", "weather")}, "zk_f": app.zk_f, "fox_name": app.fox_name(),
                "catchphrase": app.catchphrase(), "jp_font": Z._jp_font(app.root), "navkeys": h["navkeys"],
                "items": items, "images": images, "fonts": fonts})
    try:
        app.hub_close()
        app.root.destroy()
    except Exception:
        pass
with open(os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "golden", "hub_side.json"), "w") as f:
    json.dump(out, f)
for c in out:
    print(c["name"], c["W"], c["H"], c["layout"], len(c["items"]), len(c["images"]), c["look"], c["page"])
os._exit(0)
