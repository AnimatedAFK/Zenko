"""golden/hub_pages.json: hub pages exactly as zenko.py builds them, dumped like golden/hub_win.json (see that
script): the page frame's whole widget tree with every option, canvas item and picture, and for Text widgets every
display line (where it starts, its box and baseline). Old header of the shared part: the hub window's frame exactly as zenko.py builds it (the shrine border, title bar with the
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

HIST = {}            # Text widget -> what happened to it: ["see", w, h] / ["font", w, h, new font] (the size at that moment)
_see0, _tcfg0 = tk.Text.see, tk.Text.configure


def _see_rec(self, idx):
    HIST.setdefault(str(self), []).append(["see", idx, self.winfo_width(), self.winfo_height()])
    return _see0(self, idx)


def _tcfg_rec(self, cnf=None, **kw):
    if "font" in kw:
        HIST.setdefault(str(self), []).append(["font", self.winfo_width(), self.winfo_height(), str(tkfont.Font(font=kw["font"]))
                                               if False else " ".join("{%s}" % v if " " in str(v) else str(v) for v in kw["font"])])
    return _tcfg0(self, cnf, **kw)


tk.Text.see = _see_rec
tk.Text.configure = tk.Text.config = _tcfg_rec


def windows_defaults(root):
    for k, v in (("*Label.borderWidth", 2), ("*Label.padX", 1), ("*Label.padY", 1), ("*Label.highlightThickness", 0),
                 ("*Entry.borderWidth", 1), ("*Frame.borderWidth", 0), ("*Frame.highlightThickness", 0),
                 ("*Canvas.borderWidth", 0)):
        root.option_add(k, v)


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


def walk(w, top, skip, images, fonts):
    o = {}
    for ent in w.tk.splitlist(w.tk.call(w._w, "configure")):      # (PixelButton overrides .configure)
        v = w.tk.splitlist(ent)
        if len(v) > 2:
            o[str(v[0]).lstrip("-")] = str(v[-1])
    rec = {"cls": w.winfo_class(), "x": w.winfo_rootx() - top.winfo_rootx(), "y": w.winfo_rooty() - top.winfo_rooty(),
           "w": w.winfo_width(), "h": w.winfo_height(), "rw": w.winfo_reqwidth(), "rh": w.winfo_reqheight(),
           "mapped": bool(w.winfo_ismapped()), "manager": w.winfo_manager(), "opts": o, "kids": []}
    if w.winfo_manager() == "pack":
        rec["pack"] = {k: str(v) for k, v in w.pack_info().items() if k != "in"}
    if rec["cls"] == "Canvas" and w not in skip:
        items = []
        for i in w.find_all():
            ty = w.type(i)
            io = {k: v[-1] for k, v in w.itemconfigure(i).items() if k != "tags"}
            it = {"type": ty, "coords": w.coords(i), "tags": list(w.gettags(i)), "opts": io, "bbox": list(w.bbox(i) or [])}
            if ty == "image" and io.get("image") and io["image"] not in images:
                images[io["image"]] = REG.get(io["image"]) or photo_px(w.tk, io["image"])
            if ty == "text":
                f = tkfont.Font(font=io["font"])
                ent = fonts.setdefault(io["font"], {"ascent": f.metrics("ascent"), "descent": f.metrics("descent"), "actual": f.actual(), "w": {}})
                for line in io["text"].split("\n"):
                    for k in range(len(line) + 1):
                        ent["w"][line[:k]] = f.measure(line[:k])
                a = f.actual()                   # the same face at other sizes (pages lay out before zk_fit scales)
                for n in range(6, 17):
                    g = tkfont.Font(family=a["family"], size=n, weight=a["weight"], slant=a["slant"])
                    key = "{%s} %d%s%s" % (a["family"], n, " bold" if a["weight"] == "bold" else "",
                                           " italic" if a["slant"] == "italic" else "")
                    e2 = fonts.setdefault(key, {"ascent": g.metrics("ascent"), "descent": g.metrics("descent"), "w": {}})
                    e2["w"][io["text"]] = g.measure(io["text"])
                    e2["w"]["0"] = g.measure("0")
            items.append(it)
        rec["items"] = items
    if rec["cls"] == "Label" and o.get("font"):
        f = tkfont.Font(font=o["font"])
        ent = fonts.setdefault(o["font"], {"ascent": f.metrics("ascent"), "descent": f.metrics("descent"), "actual": f.actual(), "w": {}})
        ent["w"][o.get("text", "")] = f.measure(o.get("text", ""))
        ent["w"]["0"] = f.measure("0")
        a = f.actual()
        for n in range(6, 17):
            g = tkfont.Font(family=a["family"], size=n, weight=a["weight"], slant=a["slant"])
            key = "{%s} %d%s%s" % (a["family"], n, " bold" if a["weight"] == "bold" else "", " italic" if a["slant"] == "italic" else "")
            e2 = fonts.setdefault(key, {"ascent": g.metrics("ascent"), "descent": g.metrics("descent"), "w": {}})
            e2["w"][o.get("text", "")] = g.measure(o.get("text", ""))
            e2["w"]["0"] = g.measure("0")
    if w not in skip:
        for k in w.winfo_children():
            if k.winfo_class() == "Toplevel":
                continue
            rec["kids"].append(walk(k, top, skip, images, fonts))
    return rec



def text_lines(t):
    t.update_idletasks()
    seen, lines = set(), []
    for y in range(-2, t.winfo_height() + 2):
        idx = t.index(t.index("@0,%d" % y) + " display linestart")
        if idx in seen:
            continue
        seen.add(idx)
        info = t.dlineinfo(idx)
        if info:
            end = t.index(idx + " display lineend")
            last = t.index(end + " -1c") if end != idx else idx
            lines.append({"index": idx, "box": list(info), "end": end, "first_bbox": list(t.bbox(idx) or []),
                          "last_bbox": list(t.bbox(last) or [])})
    return lines


TAG_OPTS = ("font", "foreground", "background", "spacing1", "spacing2", "spacing3", "lmargin1", "lmargin2", "rmargin",
            "offset", "underline", "overstrike", "wrap", "justify", "elide")


def text_rec(t, fonts):
    """a Text widget: display lines, contents with tags, every tag's options (lowest priority first), and the numbers
    of every font it uses (per-character widths: Tk measures a run as the sum of its characters here)"""
    body = t.get("1.0", "end")
    used = {t.cget("font"), "{%s} 11" % Z.FONT_UI}          # (the font it was built and filled with)
    used |= {e[3] for e in HIST.get(str(t), []) if e[0] == "font"}
    tags = []
    for nm in t.tk.splitlist(t.tk.call(t._w, "tag", "names")):
        o = {k: str(t.tag_cget(nm, k)) for k in TAG_OPTS}
        tags.append({"name": str(nm), "opts": o})
        if o["font"]:
            used.add(o["font"])
    for fn in used:
        f = tkfont.Font(font=fn)
        ent = fonts.setdefault(fn, {"ascent": f.metrics("ascent"), "descent": f.metrics("descent"), "actual": f.actual(), "w": {}})
        ent.setdefault("ch", {})
        for ch in set(body + "0") - {"\n"}:
            ent["ch"][ch] = f.measure(ch)
        for s_ in ("Hello there", "Zenko:  ", "mitochondria"):
            assert f.measure(s_) == sum(f.measure(c) for c in s_), (fn, s_)
    return {"path": str(t), "hist": HIST.get(str(t), []), "lines": text_lines(t), "dump": t.dump("1.0", "end", tag=True, text=True), "tags": tags,
            "yview": [float(v) for v in t.yview()], "w": t.winfo_width(), "h": t.winfo_height()}


LONG_LOG = [
    ("You", "explain osmosis pls"),
    ("Fox", "## Osmosis\nOsmosis is the **net movement** of *water molecules* across a **partially permeable membrane** from a "
            "region of higher water potential to a region of lower water potential. It is a *passive* process, so no ATP is used.\n"
            "- Water moves down its water potential gradient\n- Solutes cannot cross the membrane\n  - nested point here\n"
            "1. First numbered step\n2. Second numbered step that is long enough to wrap onto another display line for sure\n"
            "> A quote about cells that is also rather long so that the second margin shows up when it wraps around\n"
            "Use `inline code` and a [link](https://example.com) plus H~2~O and x^2^ too.\n"
            "```\ndef f(x):\n    return x * 2\n```\n"
            "| a | b |\n|---|---|\n| 1 | 2 |\n"
            "Pneumonoultramicroscopicsilicovolcanoconiosisisaverylongwordthatcannotfitononelineatallnomatterwhatpneumonoultramicroscopicsilicovolcanoconiosisagainandagainandagain end."),
    ("You", "thanks!! what is $x^2 + \\frac{1}{2}$ and H2O + O2 -> H2O2"),
    ("Fox", "The value is $x^2 + \\frac{1}{2}$, and ~~wrong~~ right: ***both*** work.\n### Small heading\n---\nDone~"),
]

CASES = [
    # (name, unix time, cfg changes, window size, page)
    ("chat_snow", 1765407600.0, {"theme": "snow"}, None, "chat"),
    ("chat_midnight_small", 1760461200.0, {"theme": "midnight"}, (1120, 720), "chat"),
    ("chat_long", 1765407600.0, {"theme": "snow"}, None, "chat"),
]
out = []
for name, t, cfg, size, page in CASES:
    NOW[0] = t
    HIST.clear()
    app = Z.Zenko23()
    windows_defaults(app.root)
    app.cfg.update(cfg)
    Z.apply_theme(app.cfg.get("theme", "snow"))
    app.root.update()
    if name == "chat_long":
        app.chat_log = list(LONG_LOG)
    app.open_hub(page)
    h = app.hub
    w = h["win"]
    if size:
        w.geometry("%dx%d+0+0" % size)
    for _ in range(12):
        app.root.update()
        time.sleep(0.05)
    images, fonts = {}, {}
    pg = h["pages"][page]
    tree = walk(pg, w, set(), images, fonts)
    texts = []

    def find_texts(wd):
        for k in wd.winfo_children():
            if k.winfo_class() == "Text":
                texts.append(text_rec(k, fonts))
            find_texts(k)
    find_texts(pg)
    out.append({"name": name, "now": t, "cfg_all": cfg, "page": page, "geometry": w.geometry(), "zk_f": app.zk_f,
                "chat_log": list(app.chat_log), "tree": tree, "texts": texts, "images": images, "fonts": fonts})
    try:
        app.hub_close()
        app.root.destroy()
    except Exception:
        pass
with open(os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "golden", "hub_pages.json"), "w") as f:
    json.dump(out, f)
for c in out:
    print(c["name"], c["geometry"], len(c["images"]), list(c["fonts"]), [len(x["lines"]) for x in c["texts"]])
os._exit(0)
