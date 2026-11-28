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
import json
import random, os, tempfile, types
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

_pxp0 = Z.px_photo


def _px_rec(w, h, fn, zoom=1):
    """px_photo, recorded small: the picture before a plain whole-step zoom, and that zoom"""
    LAST[0] = None
    img = _pxp0(w, h, fn, zoom)
    if LAST[0] is not None:
        rows = LAST[0]
        z = int(zoom) if (len(rows) == h and int(zoom) > 1) else 1
        REG[str(img)] = {"z": z, "rows": ["".join("%02x%02x%02x%02x" % tuple(p) for p in row) for row in rows]}
    return img


Z.px_photo = _px_rec

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
    for k, v in (("*Checkbutton.borderWidth", 2), ("*Radiobutton.borderWidth", 2), ("*Checkbutton.highlightThickness", 1),
                 ("*Radiobutton.highlightThickness", 1), ("*Label.borderWidth", 2), ("*Label.padX", 1), ("*Label.padY", 1), ("*Label.highlightThickness", 0),
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


WIDE_PIECES = [False]                     # (set for the resized-window cases)


def wrap_pieces(text):
    """every run of a paragraph that starts a line when it wraps (at its start or after a space)"""
    out = set()
    for para in text.split("\n"):
        if " " not in para:
            continue
        for i in range(len(para)):
            if i == 0 or para[i - 1] == " ":
                for j in range(i + 1, len(para) + 1):
                    out.add(para[i:j])
    return out


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
                if io.get("width", "0") not in ("0", ""):          # wrapped canvas text measures runs from each line start
                    for piece in wrap_pieces(io["text"]):
                        ent["w"][piece] = f.measure(piece)
                a = f.actual()                   # the same face at other sizes (pages lay out before zk_fit scales)
                for n in range(6, 17):
                    g = tkfont.Font(family=a["family"], size=n, weight=a["weight"], slant=a["slant"])
                    key = "{%s} %d%s%s" % (a["family"], n, " bold" if a["weight"] == "bold" else "",
                                           " italic" if a["slant"] == "italic" else "")
                    e2 = fonts.setdefault(key, {"ascent": g.metrics("ascent"), "descent": g.metrics("descent"), "w": {}})
                    e2["w"][io["text"]] = g.measure(io["text"])
                    e2["w"]["0"] = g.measure("0")
                    if io.get("width", "0") not in ("0", ""):      # (a wrap that runs before the fonts are fitted)
                        for piece in wrap_pieces(io["text"]):
                            e2["w"][piece] = g.measure(piece)
            items.append(it)
        rec["items"] = items
    if rec["cls"] == "Label" and o.get("image") and o["image"] not in images:
        images[o["image"]] = REG.get(o["image"]) or photo_px(w.tk, o["image"])
    if rec["cls"] in ("Label", "Checkbutton", "Radiobutton") and o.get("font"):
        f = tkfont.Font(font=o["font"])
        ent = fonts.setdefault(o["font"], {"ascent": f.metrics("ascent"), "descent": f.metrics("descent"), "actual": f.actual(), "w": {}})
        ent["w"][o.get("text", "")] = f.measure(o.get("text", ""))
        ent["w"]["0"] = f.measure("0")
        for piece in wrap_pieces(o.get("text", "")):              # what wrapping the label measures
            ent["w"][piece] = f.measure(piece)
        f0 = getattr(w, "_zk_f0", None)          # and near its built size (a resized window wraps before zk_fit scales it)
        if f0 and WIDE_PIECES[0]:
            a0 = f.actual()
            for n0 in range(max(6, f0[1] - 1), f0[1] + 3):
                if n0 == a0["size"]:
                    continue
                g = tkfont.Font(family=a0["family"], size=n0, weight=a0["weight"], slant=a0["slant"])
                key = "{%s} %d%s%s" % (a0["family"], n0, " bold" if a0["weight"] == "bold" else "", " italic" if a0["slant"] == "italic" else "")
                e2 = fonts.setdefault(key, {"ascent": g.metrics("ascent"), "descent": g.metrics("descent"), "w": {}})
                for piece in wrap_pieces(o.get("text", "")):
                    e2["w"][piece] = g.measure(piece)
        a = f.actual()
        for n in range(6, 17):
            g = tkfont.Font(family=a["family"], size=n, weight=a["weight"], slant=a["slant"])
            key = "{%s} %d%s%s" % (a["family"], n, " bold" if a["weight"] == "bold" else "", " italic" if a["slant"] == "italic" else "")
            e2 = fonts.setdefault(key, {"ascent": g.metrics("ascent"), "descent": g.metrics("descent"), "w": {}})
            e2["w"][o.get("text", "")] = g.measure(o.get("text", ""))
            e2["w"]["0"] = g.measure("0")
        for n in range(17, 61):                                   # (big text sizes: the whole text only)
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
    ("chat_typed", 1765407600.0, {"theme": "snow"}, None, "chat"),
]
SCROLLS = [("units", -3), ("units", -3), ("pages", -1), ("moveto", 0.3), ("units", 3), ("units", 1), ("pages", 1),
           ("moveto", 0.0), ("units", -3), ("units", 3), ("pages", 1), ("moveto", 0.55), ("units", -1), ("moveto", 1.0)]


OUT, SETUP = "hub_pages.json", {}
if len(sys.argv) > 1 and sys.argv[1] == "home":
    # golden/hub_home.json: the Home page (stat cards, the focus card in its states, "Coming up today", the week
    # chart, quick ask, music) with a timetable, deadlines and focus history set up; time.time is pinned too
    OUT = "hub_home.json"
    time.time = lambda: NOW[0]
    T0 = 1765440000.0 + 9.5 * 3600          # Thursday 11 Dec 2025, 09:30
    CASES = [("home_idle", T0, {"theme": "snow", "user_name": "Anoop Jalan"}, None, "home"),
             ("home_focus", T0, {"theme": "snow"}, None, "home"),
             ("home_paused", T0 + 6 * 3600, {"theme": "midnight"}, (1120, 720), "home"),
             ("home_break", T0 + 12 * 3600, {"theme": "snow", "user_name": "Anoop"}, None, "home"),
             ("home_empty", T0 - 3 * 3600, {"theme": "snow"}, None, "home")]

    def _data(app, name):
        if name == "home_empty":
            return
        day = lambda n: (_dt.date(2025, 12, 11) + _dt.timedelta(days=n)).isoformat()
        tt = [{"start": "08:00", "mins": 50, "task": "Bio reading", "daily": True, "done": [day(0)]},
              {"start": "10:00", "mins": 45, "task": "Chemistry past paper 2 with the long mark scheme", "daily": True, "done": []},
              {"start": "13:00", "mins": 30, "task": "Lunch", "daily": True, "done": []},
              {"start": "15:00", "mins": 90, "task": "Maths AA SL", "daily": True, "done": []},
              {"start": "17:00", "mins": 25, "task": "French vocab", "daily": True, "done": []},
              {"start": "19:00", "mins": 60, "task": "English essay", "daily": True, "done": []},
              {"start": "21:00", "mins": 20, "task": "Read", "daily": True, "done": []}]
        app.tt[:] = tt
        Z.save_tt(app.tt)
        Z.save_deadlines([{"id": "d1", "title": "Biology MYA paper one", "kind": "Exam", "date": day(3)}])
        st = {day(-k): v for k, v in ((0, 2700), (1, 5400), (2, 0), (3, 1800), (4, 9000), (5, 600), (6, 3000))}
        with open(os.path.join(Z.DATA_DIR, "stats.json"), "w") as f:
            json.dump(st, f)
        if name == "home_focus":
            app.start_session("Bio reading", 45)
            app.session["elapsed"], app.session["dist"], app.session["closed"] = 900.0, 130.0, 2
        elif name == "home_paused":
            app.start_session("Chemistry past paper 2 with the long mark scheme", 30)
            app.session["elapsed"], app.session["paused"] = 1200.0, True
        elif name == "home_break":
            app.start_break(10)
            app.brk["elapsed"] = 125.0
    SETUP["data"] = _data


if len(sys.argv) > 1 and sys.argv[1] == "screen":
    # golden/hub_screen.json: the Screen time page (range switch, pause, tiles, the by-hour / by-day chart, websites
    # and apps lists, the privacy card with its switches and skipped words) for today, 7 and 30 days, paused, off, empty
    OUT = "hub_screen.json"
    time.time = lambda: NOW[0]
    T0 = 1765440000.0 + 15.5 * 3600          # Thursday 11 Dec 2025, 15:30
    CASES = [("screen_today", T0, {"theme": "snow"}, None, "screen"),
             ("screen_week_small", T0, {"theme": "midnight", "screen_nudges": False}, (1120, 720), "screen"),
             ("screen_month", T0, {"theme": "sakura", "screen_exclude": ["bank", "keepass", "my diary"]}, None, "screen"),
             ("screen_paused", T0, {"theme": "snow"}, None, "screen"),
             ("screen_off_empty", T0, {"theme": "snow", "screen_time": False}, None, "screen")]

    def _data(app, name):
        if name == "screen_off_empty":
            return
        day = lambda n: (_dt.date(2025, 12, 11) + _dt.timedelta(days=n)).isoformat()
        rnd = random.Random(5)
        names = ["Google Chrome", "VS Code", "Word", "Spotify", "Discord", "Microsoft Edge", "Notion", "Anki", "OneNote",
                 "Excel", "PowerPoint", "Zoom"]
        sites = ["YouTube", "Google Docs", "ManageBac", "Wikipedia", "Khan Academy", "Other websites", "New tab", "Reddit",
                 "Quizlet", "Desmos", "Gmail", "Save My Exams with a very long site name"]
        D = {}
        for k in range(0, 40, 1 if name != "screen_today" else 3):
            n_apps = 3 + (k * 7) % 9
            apps = {a: float(rnd.randint(30, 5400)) + rnd.random() for a in names[:n_apps]}
            st = {sv: float(rnd.randint(20, 3000)) for sv in sites[:2 + (k * 5) % 10]}
            hrs = [0.0] * 24
            for h in range(8, 23):
                hrs[h] = float(rnd.randint(0, 3600))
            D[day(-k)] = {"apps": apps, "sites": st, "hours": hrs}
        if name == "screen_today":
            D[day(0)]["apps"]["Google Chrome"] = 7200.5
            D[day(0)]["hours"][15] = 1800.0
        app.screen()["days"] = D
        if name == "screen_paused":
            app.screen()["paused_until"] = NOW[0] + 37 * 60 + 5
        app.screen_ui = {"range": {"screen_week_small": 7, "screen_month": 30}.get(name, 1)}
        app.life_save()

    def _after(app, name):
        # (the page as it settles: rendered, then zk_fit(fonts=True) 30 ms later; a slow first render can land after
        # the harness's own fit)
        app.screen_render()
        for _ in range(8):
            app.root.update()
            time.sleep(0.05)
    SETUP["data"], SETUP["after"] = _data, _after


if len(sys.argv) > 1 and sys.argv[1] == "gym":
    # golden/hub_gym.json: the Exercise page (dusk banner, muscle map with its glow figures, which-day picker / training
    # ticks / rest night, daily protein-water-sleep, weight line + profile, food today with its meters) in five states.
    # The page's animations (foxfire flicker, star twinkle, the figures' glow frames) are left out: frame 0 only.
    OUT = "hub_gym.json"
    time.time = lambda: NOW[0]
    T0 = 1765440000.0 + 15.5 * 3600          # Thursday 11 Dec 2025, 15:30
    CASES = [("gym_pick", T0, {"theme": "snow"}, None, "exercise"),
             ("gym_day1", T0, {"theme": "snow"}, None, "exercise"),
             ("gym_day2_small", T0, {"theme": "midnight"}, (1120, 720), "exercise"),
             ("gym_rest", T0, {"theme": "sakura"}, None, "exercise"),
             ("gym_view_legs", T0, {"theme": "snow"}, None, "exercise")]

    def _data(app, name):
        day = lambda n: (_dt.date(2025, 12, 11) + _dt.timedelta(days=n)).isoformat()
        G = app.gym()
        if name == "gym_pick":
            app.life_save()
            return
        G["next"] = {"gym_day1": 0, "gym_day2_small": 1, "gym_rest": 3, "gym_view_legs": 0}[name]
        if name in ("gym_day1", "gym_view_legs"):
            G["checks"][day(0) + "|0"] = ["Chest", "Biceps"]
            G["checks"][day(0) + "|2"] = ["Quads", "Hamstrings", "Glutes", "Calves"]
            G["log"] = [{"date": day(-9), "day": 1}, {"date": day(-2), "day": 2}]
            G["weights"] = [{"date": day(-k), "kg": w} for k, w in ((40, 74.2), (30, 73.6), (20, 73.1), (12, 72.4),
                                                                     (6, 72.6), (1, 71.9))]
            G["profile"] = {"age": 17, "height": 172.0, "sex": "m"}
            G["food"][day(0)] = [{"t": "2 eggs (100g)", "kcal": 143, "p": 12.6, "src": "table"},
                                 {"t": "Toast with butter", "kcal": 180, "p": 4.0, "src": "table"},
                                 {"t": "Paneer tikka (150g) · usda", "kcal": 412, "p": 27.35, "src": "web", "g": 150,
                                  "k100": 274.7, "p100": 18.2, "match": "paneer"},
                                 {"t": "A very long dish name that goes on and on with chutney and rice and more", "kcal": 655,
                                  "p": 18.0, "src": "brain"}]
            G["water"][day(0)] = 3
            G["sleep"].update({day(0): 4, day(-1): 2, day(-3): 5, day(-9): 1})
        elif name == "gym_day2_small":
            G["log"] = [{"date": day(-1), "day": 0}]
            G["water"][day(0)] = 8
            G["sleep"][day(0)] = 1
        elif name == "gym_rest":
            G["weights"] = [{"date": day(-3), "kg": 80.0}]
            G["profile"] = {"age": 17, "sex": "f"}
            G["log"] = [{"date": day(-1), "day": 2}]
            G["food"][day(0)] = [{"t": "Dal (1 bowl)", "kcal": 180, "p": 9.0, "src": "table"}]
        app.life_save()

    def _after(app, name):
        after0 = app.root.after

        def quiet(ms, fn=None, *a):           # (the page's looping animations: frame 0 only)
            if ms in (230, 260, 420, 500):
                return ""
            return after0(ms, fn, *a) if fn else after0(ms)
        app.root.after = quiet
        if name == "gym_view_legs":
            app.gym_view = 2
        app.gym_render()
        app.root.after = after0
        for _ in range(6):
            app.root.update()
            time.sleep(0.03)
    SETUP["data"], SETUP["after"] = _data, _after


if len(sys.argv) > 1 and sys.argv[1] == "you":
    # golden/hub_you.json: the About you page, empty and filled in (Text boxes, checkbuttons, what she learned)
    OUT = "hub_you.json"
    time.time = lambda: NOW[0]
    T0 = 1765440000.0 + 9.5 * 3600
    YOU = {"user_name": "Anoop Jalan", "user_role": "IB Diploma student in year two",
           "user_studying": "Biology HL internal assessment on enzyme kinetics and the chemistry of buffers",
           "user_goals": "A 7 in Bio HL\nFinish the EE draft by January\nSleep before 23:00",
           "user_notes": "Be gentle but push me a bit. I like short answers with one example. " * 3,
           "learn_style": False}
    CASES = [("you_empty", T0, {"theme": "snow"}, None, "you"),
             ("you_filled", T0, dict(YOU, theme="midnight"), (1120, 720), "you"),
             ("you_learned", T0, dict(YOU, theme="snow"), None, "you")]

    def _data(app, name):
        if name == "you_learned":
            d = Z.load_learned()
            d["replies"] = {"good morning": "Ohayo!", "gn": "Oyasumi~"}
            d.setdefault("style", {})["aliases"] = {"sched": "timetable", "hw": "homework"}
            Z.save_learned(d)
    SETUP["data"] = _data


if len(sys.argv) > 1 and sys.argv[1] == "daily":
    # golden/hub_daily.json: the Daily page (stat plaques, check-in with the 14-day dots, flashcards, wellbeing, the
    # 61 ema badges with tiers / pins / secrets, homework buddy, things she remembers) with a life set up, and empty
    OUT = "hub_daily.json"
    time.time = lambda: NOW[0]
    T0 = 1765440000.0 + 9.5 * 3600
    CASES = [("daily_snow", T0, {"theme": "snow"}, None, "daily"),
             ("daily_midnight_small", T0, {"theme": "midnight", "badge_view": "earned", "remind_water": False,
                                           "daynight": False}, (1120, 720), "daily"),
             ("daily_empty", T0, {"theme": "sakura"}, None, "daily")]

    def _data(app, name):
        L = app.life
        day = lambda n: (_dt.date(2025, 12, 11) + _dt.timedelta(days=n)).isoformat()
        if name == "daily_empty":
            for k in ("journal", "cards", "notes", "found"):
                L[k] = []
            L["badges"], L["facts"], L["btier"], L["badge_pins"] = {}, {}, {}, []
            app.life_save()
            return
        L["journal"] = [{"date": day(-k), "mood": m, "note": n, "t": 0} for k, m, n in
                        ((12, "good", ""), (9, "meh", "Tired after the mock exam, but the chemistry went fine I think"),
                         (5, "great", "Got a 7 on the bio quiz!"), (3, "rough", ""), (1, "okay", "Long day"), (0, "good", ""))]
        L["cards"] = [{"id": "c%d" % i, "q": q, "a": a, "box": b, "wrong": w, "right": 1, "due": day(dd)} for i, (q, a, b, w, dd) in
                      enumerate((("What is the function of ATP synthase in oxidative phosphorylation?", "Makes ATP from ADP + Pi", 1, 4, -1),
                                 ("Avogadro's number", "6.022e23", 0, 0, 0), ("Le Chatelier", "System opposes change", 2, 2, 3),
                                 ("pH of a buffer", "pKa + log(A/HA)", 1, 1, -2)))]
        L["badges"] = {"first": day(-30), "streak3": day(-20), "t_angler": day(-10), "night": day(-5)}
        L["btier"] = {"t_angler": 2}
        L["badge_pins"] = ["t_angler"]
        L["facts"] = {"favourite subject": "biology", "birthday": "05-14", "pet": "a cat called Miso"}
        L["notes"] = ["I have piano on Tuesdays", "Mum's birthday is in March and I should plan something nice for it"]
        L["found"] = [{"what": "a maple leaf"}, {"what": "a shiny pebble"}]
        app.well.update({"eyes": 600, "posture": 1500, "water": 100})
        app.life_save()
    SETUP["data"] = _data


def scroll_rec(t):
    """the wheel and the scrollbar: yview after each 'yview scroll' / 'yview moveto', and the top line then"""
    rec = []
    for how, v in SCROLLS:
        if how == "moveto":
            t.yview_moveto(v)
        else:
            t.yview_scroll(v, how)
        t.update_idletasks()
        top = t.index("@0,0")
        rec.append({"op": [how, v], "yview": [float(x) for x in t.yview()], "top": t.index(top + " display linestart"),
                    "top_box": list(t.dlineinfo(top) or [])})
    return rec
if os.environ.get("ZK_TRACE"):                                   # debugging: which renders / wraps run, and when
    import traceback
    for _m in ("daily_render", "zk_flow_reflow", "z21_unwrap", "zk_fit"):
        def _wrap(f, m=_m):
            def g(self, *a, **k):
                print("TRACE", m, [x.name for x in traceback.extract_stack()[-6:-1]], flush=True)
                return f(self, *a, **k)
            return g
        setattr(Z.Zenko23, _m, _wrap(getattr(Z.Zenko23, _m)))
    if os.environ.get("ZK_ONLY"):
        CASES = [c for c in CASES if c[0] == os.environ["ZK_ONLY"]]
if len(sys.argv) > 1 and sys.argv[1] == "timetable":
    # golden/hub_timetable.json: the Timetable page (header with 急 Delay / Prev / Today / Next, progress bar, the
    # day's cards in every state: done, NOW, missed, in N min, later, repeating; the add form) today, tomorrow, empty
    OUT = "hub_timetable.json"
    time.time = lambda: NOW[0]
    T0 = 1765440000.0 + 1.5 * 3600          # Thursday 11 Dec 2025, 09:30 UTC
    CASES = [("tt_today", T0, {"theme": "snow"}, None, "timetable"),
             ("tt_midnight_small", T0, {"theme": "midnight"}, (1120, 720), "timetable"),
             ("tt_tomorrow", T0, {"theme": "snow"}, None, "timetable"),
             ("tt_empty", T0, {"theme": "snow"}, None, "timetable")]

    def _data(app, name):
        if name == "tt_empty":
            return
        day = lambda n: (_dt.date(2025, 12, 11) + _dt.timedelta(days=n)).isoformat()
        tt = [{"start": "07:00", "mins": 30, "task": "Morning run", "daily": True, "done": []},
              {"start": "08:00", "mins": 50, "task": "Bio reading", "daily": True, "done": [day(0)]},
              {"start": "09:00", "mins": 60, "task": "Chemistry past paper 2 with the long mark scheme", "daily": False,
               "date": day(0), "done": []},
              {"start": "10:00", "mins": 45, "task": "Maths AA SL", "daily": True, "done": []},
              {"start": "13:00", "mins": 30, "task": "Lunch", "daily": True, "done": []},
              {"start": "15:00", "mins": 90, "task": "French vocab and the listening practice", "daily": False,
               "date": day(0), "done": []},
              {"start": "17:00", "mins": 25, "task": "Psych notes", "daily": True, "done": []},
              {"start": "19:00", "mins": 60, "task": "English essay", "daily": False, "date": day(1), "done": []},
              {"start": "21:00", "mins": 20, "task": "Read", "daily": True, "done": []}]
        app.tt[:] = tt
        Z.save_tt(app.tt)

    def _after(app, name):
        if name == "tt_tomorrow":
            app.tt_nav(1)
    SETUP["data"], SETUP["after"] = _data, _after


if len(sys.argv) > 1 and sys.argv[1] == "deadlines":
    # golden/hub_deadlines.json: the Upcoming page (add box, exam countdown plaques, the Japanese clock with its
    # half-hour ring, timetable band, hands and quick add, the study plan, the month calendar, Coming up, and the
    # IA/EE/TOK tracker) wide, narrow (two rows), on another day/PM with a picked date, and empty
    OUT = "hub_deadlines.json"
    time.time = lambda: NOW[0]
    T0 = 1765440000.0 + 1.5 * 3600 + 754.25   # Thursday 11 Dec 2025, 09:42:34.25 UTC
    CASES = [("up_wide", T0, {"theme": "snow"}, None, "deadlines"),
             ("up_midnight_small", T0, {"theme": "midnight"}, (1120, 720), "deadlines"),
             ("up_pm_day", T0, {"theme": "snow"}, None, "deadlines"),
             ("up_empty", T0, {"theme": "snow"}, None, "deadlines")]

    def _data(app, name):
        if name == "up_empty":
            return
        day = lambda n: (_dt.date(2025, 12, 11) + _dt.timedelta(days=n)).isoformat()
        Z.save_deadlines([
            {"id": "d1", "title": "Biology MYA paper one", "kind": "Exam", "date": day(3), "time": "09:00", "done": False},
            {"id": "d2", "title": "Chemistry quiz on equilibrium and the long buffer questions", "kind": "Quiz",
             "date": day(1), "time": "", "done": False},
            {"id": "d3", "title": "English essay draft", "kind": "Assignment", "date": day(-2), "time": "", "done": False},
            {"id": "d4", "title": "Maths homework", "kind": "Homework", "date": day(9), "time": "", "done": False},
            {"id": "d5", "title": "Old test", "kind": "Exam", "date": day(-5), "time": "", "done": True}])
        tt = [{"start": "08:00", "mins": 50, "task": "Bio reading", "daily": True, "done": []},
              {"start": "13:00", "mins": 30, "task": "Lunch", "daily": True, "done": []},
              {"start": "20:00", "mins": 60, "task": "Piano", "daily": True, "done": []}]
        app.tt[:] = tt
        Z.save_tt(app.tt)
        L = app.life
        for k in range(7):
            L["free"][str(k)] = [16.0, 16.5, 17.0, 17.5, 18.0, 19.0, 19.5] if k < 5 else [10.0, 10.5, 11.0, 14.0, 14.5]
        L["free_set"] = True
        L["studied"]["d1"] = 95
        app.life_save()
        app.replan()
        app.ib_add("ee", due=day(20))
        app.ib_tasks()[-1]["words"] = 1850
        app.ib_tasks()[-1]["stages"] = ["Topic", "Research"]
        app.ib_add("ia", "bio", due=day(5))
        app.ib_add("tok_essay")
        app.life_save()

    def _after(app, name):
        if name == "up_pm_day":
            app.up_day, app.up_pm = 5, True
            app.dl_day = _dt.date(2025, 12, 14)
            app.dl_render()
    SETUP["data"], SETUP["after"] = _data, _after


if len(sys.argv) > 1 and sys.argv[1] == "reminders":
    # golden/hub_reminders.json: the Reminders page (write an ema, the night shrine canvas: sky, lanterns for today's
    # timetable, the rack with plaques that are soon / daily / rang / later / anytime, Zenko and her bubble, the
    # fulfilled row, foxfire) wide, in a small midnight window, with a plaque selected (its pills) and one hovered,
    # and empty
    OUT = "hub_reminders.json"
    time.time = lambda: NOW[0]
    T0 = 1765440000.0 + 1.5 * 3600 + 754.25   # Thursday 11 Dec 2025, 09:42:34.25 UTC
    CASES = [("rem_wide", T0, {"theme": "snow"}, None, "reminders"),
             ("rem_midnight_small", T0, {"theme": "midnight"}, (1120, 720), "reminders"),
             ("rem_sel", T0, {"theme": "snow"}, None, "reminders"),
             ("rem_empty", T0, {"theme": "snow"}, None, "reminders")]

    def _data(app, name):
        app.life["ema"] = []
        day = lambda n: (_dt.date(2025, 12, 11) + _dt.timedelta(days=n)).isoformat()
        if name == "rem_empty":
            app.tt[:] = []
            Z.save_tt(app.tt)
            app.life_save()
            return
        at = lambda d, hh, mm: _dt.datetime(2025, 12, 11 + d, hh, mm).timestamp()
        app.ema_add("Call mum about the weekend", T0 + 40 * 60)
        app.ema_add("Take vitamins", at(0, 20, 0), "daily")
        app.ema_add("Return the library book", at(1, 16, 0))
        app.ema_add("Buy a new notebook for chemistry and the long revision cards for the mock exams")
        E = app.ema_add("Drink water", T0 - 10 * 60)
        E["rang"] = True
        app.ema_add("Email Ms Lee about the IA draft", at(5, 9, 30))
        D = app.ema_add("Finish the bio worksheet")
        D["done"] = True
        D2 = app.ema_add("Print the TOK essay")
        D2["done"] = True
        app.ema_save()
        tt = [{"start": "08:00", "mins": 50, "task": "Bio reading", "daily": True, "done": [day(0)]},
              {"start": "10:00", "mins": 45, "task": "Maths AA SL practice set with the calculator", "daily": True, "done": []},
              {"start": "13:00", "mins": 30, "task": "Lunch", "daily": True, "done": []},
              {"start": "16:30", "mins": 60, "task": "French vocab", "daily": False, "date": day(0), "done": []}]
        app.tt[:] = tt
        Z.save_tt(app.tt)
        app.life_save()

    def _after(app, name):
        if name == "rem_sel":
            act = app.ema_active()
            app._ema_sel = act[0]["id"]
            app.ema_ui["hover"] = act[1]["id"]
            app.ema_render()
    SETUP["data"], SETUP["after"] = _data, _after



if len(sys.argv) > 1 and sys.argv[1] == "settings":
    # golden/hub_settings.json: the Settings page (the section chips, every card down the two columns) wide, in a
    # small midnight window, and with other choices made (theme, tails, bib, size, units, volume, toggles off)
    OUT = "hub_settings.json"
    time.time = lambda: NOW[0]
    T0 = 1765440000.0 + 1.5 * 3600 + 754.25
    ALT = {"theme": "snow", "tails": "9", "scarf": "#3b6ea8", "size": 6, "temp_unit": "F", "sound_volume": 3,
           "glasses_in_session": False, "foxfire": False, "web_enabled": False, "home_city": "Mumbai",
           "break_after_minutes": 45, "break_minutes": 10, "speak_mode": "always", "voice_engine": "whisper",
           "text_scale": 1.15, "study_video_words": ["khan", "crashcourse"], "remind_voice": "off",
           "tts_speed": 2, "spotify_client_id": "abc123", "bubble_font": "pixel"}
    CASES = [("set_wide", T0, {"theme": "snow"}, None, "settings"),
             ("set_midnight_small", T0, {"theme": "midnight"}, (1120, 720), "settings"),
             ("set_alt", T0, ALT, None, "settings")]
    SETUP["data"], SETUP["after"] = (lambda app, name: None), (lambda app, name: None)

if len(sys.argv) > 1 and sys.argv[1] == "customize":
    # golden/hub_customize.json: the Customize page (live preview, name, voice, coat / eyes / foxfire swatches, tails,
    # ears, size, outfits, idle habits, wardrobe, poses, look and feel, sidebar pages, travel outfits, bubble, decor,
    # scene) wide, in a small midnight window, and with most of her look changed
    OUT = "hub_customize.json"
    time.time = lambda: NOW[0]
    T0 = 1765440000.0 + 1.5 * 3600 + 754.25
    ALT = {"theme": "snow", "coat": "black", "eye_color": "#a77bff", "foxfire_color": "#ff5fa8", "tails": "9",
           "ear_style": "floppy", "blush_style": "rosy", "size": 6, "outfit": ["bow", "glasses_always"],
           "scarf": "#2f8f83", "voice_level": 2, "mode": "savage", "voice_clips": "off", "chirp_pitch": 1.15,
           "tts_pitch": 4, "idle_sway": False, "idle_yawn": False, "look_at_mouse": False, "idle_lively": "lively",
           "fox_name": "Kitsu", "catchphrase": "Kon kon, let's go!", "accent": "jade", "motion_level": "smooth",
           "nav_compact": True, "nav_order": ["chat", "home", "timetable"], "nav_hidden": ["garden", "screen"],
           "jp_voice": False, "bubble_scale": False, "fox_poses": False, "lockin_headband": False}
    CASES = [("cust_wide", T0, {"theme": "snow"}, None, "customize"),
             ("cust_midnight_small", T0, {"theme": "midnight"}, (1120, 720), "customize"),
             ("cust_alt", T0, ALT, None, "customize")]
    SETUP["data"], SETUP["after"] = (lambda app, name: None), (lambda app, name: None)

if len(sys.argv) > 1 and sys.argv[1] == "palette":
    # golden/hub_palette.json: the Ctrl+K palette over the Settings page: every item it offers, what a few searches
    # show (word starts, a typo matched by difflib, the "Ask" row) and the box itself, wide and in a small midnight window
    OUT = "hub_palette.json"
    time.time = lambda: NOW[0]
    T0 = 1765440000.0 + 1.5 * 3600 + 754.25
    QUERY = {"pal_empty": ("", 0), "pal_sett": ("sett", 0), "pal_typo_small": ("brething", 2), "pal_two": ("acc ja", 1)}
    CASES = [("pal_empty", T0, {"theme": "snow"}, None, "settings"),
             ("pal_sett", T0, {"theme": "snow"}, None, "settings"),
             ("pal_typo_small", T0, {"theme": "midnight"}, (1120, 720), "settings"),
             ("pal_two", T0, {"theme": "snow", "nav_hidden": ["garden"]}, None, "settings")]

    def _open(app, name):
        for pg in ("customize", "daily"):          # (pages built later, as prewarm builds them, list their cards)
            app.z20_prebuild(pg)
        app.root.update()
        app.z20_palette()
        P = app._z20_palette
        P._maybe_close = lambda *a: None           # (the test display's focus comes and goes)
        q, mv = QUERY[name]
        P.var.set(q)
        for _ in range(mv):
            P.move(1)
        for _ in range(10):                        # (zk_on_map scales each widget as the X server maps it)
            app.root.update()
            time.sleep(0.05)

    def _tree(app, images, fonts):
        _open(app, NAME[0])
        P = app._z20_palette
        P.win.update_idletasks()
        keep = ("title", "kind", "sub", "words", "boost")
        EXTRA[0] = {"items": [{k: it.get(k) for k in keep if k in it} for it in P.items],
                    "shown": [{k: it.get(k) for k in keep if k in it} for it in P.shown], "sel": P.sel, "W": P.W,
                    "query": P.var.get(), "hub_w": app.hub["win"].winfo_width(), "hub_h": app.hub["win"].winfo_height(),
                    "pal_geo": P.win.geometry(), "navkeys": [k for k, _ in app.hub["navkeys"]],
                    "hub_xy": [app.hub["win"].winfo_rootx(), app.hub["win"].winfo_rooty()]}
        return walk(P.win, P.win, set(), images, fonts)
    SETUP["data"], SETUP["tree"] = (lambda app, name: None), _tree

if len(sys.argv) > 1 and sys.argv[1] == "shelf":
    # golden/hub_shelf.json: the Study books shelf (the night, the cabinet, the nine books with their labels and
    # this week's hours, the hanging scroll), wide in snow, a book hovered (lifted, glowing, the scroll describing it),
    # and in a small midnight window
    OUT = "hub_shelf.json"
    time.time = lambda: NOW[0]
    T0 = 1765440000.0 + 1.5 * 3600 + 754.25
    CASES = [("shelf_wide", T0, {"theme": "snow"}, None, "notebook"),
             ("shelf_hover", T0, {"theme": "snow"}, None, "notebook"),
             ("shelf_midnight_small", T0, {"theme": "midnight"}, (1120, 720), "notebook")]

    def _data(app, name):
        app.nb_sid = "shelf"
        app.life["slog"] = {"2025-12-09": {"bio": 5400, "chem": 600}, "2025-12-11": {"math": 1500, "bio": 300},
                            "2025-12-01": {"french": 3000}}
        app.life["grades"] = [{"sid": "bio", "score": 6, "of": 7, "what": "Mock"}] * (3 if name == "shelf_hover" else 1)
        app.life_save()

    def _after(app, name):
        if name == "shelf_hover":
            app.shelf_hover("bio")
            for _ in range(40):
                app.root.update()
                time.sleep(0.03)

    def _tree(app, images, fonts):
        EXTRA[0] = {"mincho": Z._mincho_font(app.root), "serif_font": Z._serif_font(app.root),
                    "lift": dict(app.nb["fx"]["lift"]), "hover": app.nb.get("hover"),
                    "petals": [p[:4] for p in app.nb["fx"]["petals"]],
                    "books": [[b[0], b[1], b[2], b[3], app.book_meta(b[0])["col"]] for b in Z.BOOKS],
                    "practice": Z.BOOK_PRACTICE}
        h = app.hub
        return walk(h["pages"]["notebook"], h["win"], set(), images, fonts)
    SETUP["data"], SETUP["after"], SETUP["tree"] = _data, _after, _tree

if len(sys.argv) > 1 and sys.argv[1] == "book":
    # golden/hub_book.json: open books on the Study books page: a subject's (Zenko at her blackboard, the practice
    # card, the tutor brain, the right page's tutor, sources, topics and history) for each subject, the study diary
    # and the grades book; a small midnight window too. random is seeded just before the book is made, so the
    # practice pages (a fact, a generated problem, a word, a prompt, a quiz) can be made the same way in Rust.
    OUT = "hub_book.json"
    time.time = lambda: NOW[0]
    T0 = 1765440000.0 + 1.5 * 3600 + 754.25
    CASES = [("book_bio", T0, {"theme": "snow"}, None, "notebook"),
             ("book_chem", T0, {"theme": "snow"}, None, "notebook"),
             ("book_math_small", T0, {"theme": "midnight"}, (1120, 720), "notebook"),
             ("book_french", T0, {"theme": "snow"}, None, "notebook"),
             ("book_psych", T0, {"theme": "snow"}, None, "notebook"),
             ("book_english", T0, {"theme": "snow"}, None, "notebook"),
             ("book_diary", T0, {"theme": "snow"}, None, "notebook"),
             ("book_grades", T0, {"theme": "snow"}, None, "notebook")]
    BEFORE = {}

    def _data(app, name):
        app.nb_sid = "shelf"
        app.life["slog"] = {"2025-12-09": {"bio": 5400, "chem": 600}, "2025-12-11": {"math": 1500, "bio": 300}}
        app.life["grades"] = [{"id": "g1", "sid": "bio", "kind": "Mock", "name": "Cells", "score": 34.0, "out_of": 40.0,
                               "grade": 6, "day": "2025-11-20"},
                              {"id": "g2", "sid": "math", "kind": "Quiz", "name": "", "score": 18.0, "out_of": 25.0,
                               "grade": None, "day": "2025-12-02"}] if name == "book_grades" else []
        app.life["nb_hist"] = {"bio": [{"t": T0 - 3600, "q": "What is osmosis?", "a": "Osmosis is the **net movement** of water."}]} \
            if name == "book_bio" else {}
        app.life_save()

    def _after(app, name):
        key = name.split("_")[1]
        BEFORE["life"] = json.loads(json.dumps(app.life, default=str))
        BEFORE["cfg"] = json.loads(json.dumps(app.cfg, default=str))
        random.seed(77)
        app.book_open(key, anim=False)
        for _ in range(16):
            app.root.update()
            time.sleep(0.05)

    def _tree(app, images, fonts):
        EXTRA[0] = {"mincho": Z._mincho_font(app.root), "serif_font": Z._serif_font(app.root), "life_before": BEFORE["life"],
                    "cfg_before": BEFORE["cfg"], "jp_font": Z._jp_font(app.root), "book": app.nb_sid}
        h = app.hub
        return walk(h["pages"]["notebook"], h["win"], set(), images, fonts)
    SETUP["data"], SETUP["after"], SETUP["tree"] = _data, _after, _tree

if len(sys.argv) > 1 and sys.argv[1] == "bookanim":
    # golden/hub_bookanim.json: a book clicked on the shelf: every frame of book_anim (the book gliding to the middle,
    # then its cover swinging open), recorded as the canvas's "anim" items just before each next frame is scheduled,
    # and the book that ends up open.
    OUT = "hub_bookanim.json"
    time.time = lambda: NOW[0]
    T0 = 1765440000.0 + 1.5 * 3600 + 754.25
    CASES = [("anim_chem", T0, {"theme": "snow"}, None, "notebook"),
             ("anim_math_small", T0, {"theme": "midnight"}, (1120, 720), "notebook"),
             ("anim_diary", T0, {"theme": "snow"}, None, "notebook")]
    FRAMES = []

    def _data(app, name):
        app.nb_sid = "shelf"
        app.life_save()

    def _after(app, name):
        key = name.split("_")[1]
        cv = app.nb["shelf"]
        real = cv.after
        FRAMES.clear()

        def rec(ms, fn=None, *a):
            items = []
            for i in cv.find_withtag("anim"):
                io = {k: v[-1] for k, v in cv.itemconfigure(i).items() if k != "tags"}
                items.append({"type": cv.type(i), "coords": cv.coords(i), "tags": list(cv.gettags(i)), "opts": io})
            if items:
                FRAMES.append({"ms": ms, "items": items})
            return real(ms, fn, *a)
        cv.after = rec
        app.book_open(key)
        for _ in range(60):
            app.root.update()
            time.sleep(0.03)
            if not (app.nb or {}).get("anim") and app.nb_sid == key:
                break

    def _tree(app, images, fonts):
        EXTRA[0] = {"frames": list(FRAMES), "book": app.nb_sid, "nb_last": app.life.get("nb_last"),
                    "jp_font": Z._jp_font(app.root), "font_ui_name": Z.FONT_UI}
        return {"w": 0, "h": 0, "kids": []}
    SETUP["data"], SETUP["after"], SETUP["tree"] = _data, _after, _tree

if len(sys.argv) > 1 and sys.argv[1] == "bookact":
    # golden/hub_bookact.json: an open book used: its practice buttons pressed and answers typed (random seeded before
    # each step), recording every label's text and what the life file / settings hold after each step.
    OUT = "hub_bookact.json"
    time.time = lambda: NOW[0]
    T0 = 1765440000.0 + 1.5 * 3600 + 754.25
    SCRIPTS = {
        "chem": [("entry", 0, "12345"), ("btn", "Check"), ("btn", "New problem"), ("btn", "Show working"), ("entry", 0, "x"),
                 ("btn", "Check")],
        "math": [("entry", 0, "-7.5"), ("btn", "Check"), ("btn", "Check"), ("entry", 0, "3/4"), ("btn", "Check")],
        "bio": [("btn", "Next fact"), ("btn", "Quiz me"), ("entry", 0, "mitochondria"), ("btn", "Quiz me"), ("btn", "Next fact")],
        "french": [("entry", 0, "bonjour"), ("btn", "Check"), ("btn", "Check"), ("btn", "Skip"), ("entry", 0, "le chat"),
                   ("btn", "Check")],
        "psych": [("btn", "New prompt"), ("btn", "New prompt")],
        "grades": [("entry", 1, "31"), ("entry", 2, "40"), ("btn", "\u8a18\u5165 \u00b7 Save result"),
                   ("entry", 1, "abc"), ("btn", "\u8a18\u5165 \u00b7 Save result")],
        "diary": [("entry", 0, "2315"), ("ret", 0), ("entry", 1, "7:05"), ("ret", 1), ("entry", 2, "40"), ("ret", 2),
                  ("entry", 3, "5"), ("ret", 3)],
    }
    CASES = [("act_" + k, T0, {"theme": "snow"}, None, "notebook") for k in SCRIPTS]
    STEPS = []

    def _data(app, name):
        app.nb_sid = "shelf"
        app.life["slog"] = {"2025-12-09": {"bio": 5400, "chem": 600}, "2025-12-11": {"math": 1500, "bio": 300}}
        app.life["grades"], app.life["nb_hist"] = [], {}
        app.life_save()

    def _labels(w, acc):
        for k in w.winfo_children():
            c = k.winfo_class()
            if c in ("Label", "Checkbutton", "Radiobutton", "Menubutton"):
                acc.append(k.cget("text"))
            elif c == "Canvas" and hasattr(k, "_text"):
                acc.append("[" + k._text + "]")
            _labels(k, acc)
        return acc

    def _find(w, pred, acc):
        for k in w.winfo_children():
            if pred(k):
                acc.append(k)
            _find(k, pred, acc)
        return acc

    def _after(app, name):
        key = name.split("_")[1]
        BEFORE_L = json.loads(json.dumps(app.life, default=str))
        EXTRA[0] = {"life_before": BEFORE_L, "cfg_before": json.loads(json.dumps(app.cfg, default=str))}
        random.seed(77)
        app.book_open(key, anim=False)
        for _ in range(10):
            app.root.update()
            time.sleep(0.03)
        STEPS.clear()
        host = app.nb["host"]
        for i, st in enumerate(SCRIPTS[key]):
            random.seed(100 + i)
            ents = _find(host, lambda k: k.winfo_class() == "Entry", [])
            if st[0] == "entry":
                e = ents[st[1]]
                e.delete(0, "end")
                e.insert(0, st[2])
                if key != "grades" and key != "diary":
                    e.focus_force()
                    app.root.update()
                    e.event_generate("<Return>")
            elif st[0] == "ret":
                e = ents[st[1]]
                e.focus_force()
                app.root.update()
                e.event_generate("<Return>")
            else:
                bs = _find(host, lambda k: getattr(k, "_text", None) == st[1] and hasattr(k, "_cmd"), [])
                bs[0]._cmd()
            for _ in range(4):
                app.root.update()
                time.sleep(0.02)
            host = app.nb["host"]
            STEPS.append({"step": list(st), "labels": _labels(host, []),
                          "entries": [k.get() for k in _find(host, lambda k: k.winfo_class() == "Entry", [])],
                          "life": json.loads(json.dumps({k: app.life.get(k) for k in ("book_state", "fr_vocab", "grades", "goals",
                                                                                         "cards")}, default=str)),
                          "cfg": {k: app.cfg.get(k) for k in ("bedtime", "wake_time", "burnout_hours", "track_idle_minutes")}})

    def _tree(app, images, fonts):
        EXTRA[0].update({"steps": list(STEPS), "mincho": Z._mincho_font(app.root), "serif_font": Z._serif_font(app.root),
                         "jp_font": Z._jp_font(app.root)})
        return {"w": 0, "h": 0, "kids": []}
    SETUP["data"], SETUP["after"], SETUP["tree"] = _data, _after, _tree

if len(sys.argv) > 1 and sys.argv[1] == "notes":
    # golden/hub_notes.json: the Notes page (scratchpad with its preview, the notebooks list with one subject open, the
    # page view), then used: a folder and a page picked in the list, the scratchpad filed, the filing undone, a new
    # page; after each step the list's rows and colours, the title, the page, the status, the preview, the scratchpad
    # and the files in the notebooks folder.
    OUT = "hub_notes.json"
    time.time = lambda: NOW[0]
    T0 = 1765440000.0 + 1.5 * 3600 + 754.25
    CASES = [("notes", T0, {"theme": "snow"}, None, "notes"), ("notes_small", T0, {"theme": "midnight"}, (1120, 720), "notes")]
    STEPS = []
    SCRATCH = ("Bonjour, je m'appelle Anoop.\nLe passé composé uses avoir or être.\nMitosis has four phases.\n"
               "The cell membrane is a phospholipid bilayer.\n\nmoles = mass / molar mass")

    def _data(app, name):
        Z.nb_ensure(app.life)
        bio = os.path.join(Z.NB_DIR, "Biology HL")
        os.makedirs(os.path.join(bio, "Cells"), exist_ok=True)
        with open(os.path.join(bio, "Cells", "membranes.md"), "w", encoding="utf-8") as f:
            f.write("# Membranes\n\nPhospholipid bilayer, fluid mosaic.")
        with open(os.path.join(bio, "Genetics.txt"), "w", encoding="utf-8") as f:
            f.write("DNA, genes, alleles.")
        with open(os.path.join(bio, "diagram.png"), "wb") as f:
            f.write(b"x")
        os.makedirs(os.path.join(Z.NB_DIR, "Imported", "reMarkable"), exist_ok=True)
        with open(os.path.join(Z.NB_DIR, "Imported", "reMarkable", "Quick.md"), "w", encoding="utf-8") as f:
            f.write("hi")
        os.makedirs(os.path.join(Z.NB_DIR, "Zeta extra"), exist_ok=True)
        with open(Z.NB_SCRATCH, "w", encoding="utf-8") as f:
            f.write(SCRATCH)
        app.cfg["nb_open"] = [bio]
        app.life["nb_last"] = {"when": "2025-12-10T09:05", "to": ["Bio", "Chem", "Bio"]}
        app.life_save()

    def _files():
        out = []
        for dp, dn, fs in os.walk(Z.NB_DIR):
            dn.sort()
            for fn in sorted(fs):
                p = os.path.join(dp, fn)
                with open(p, "rb") as f:
                    out.append([os.path.relpath(p, Z.NB_DIR), f.read().decode("utf-8", "replace")])
        return out

    MEAS = []

    def _state(app, step):
        ui = app._nb_ui
        lb = ui["tree"]
        for k in ("status", "preview", "page_title"):
            MEAS.append((ui[k].cget("font"), ui[k].cget("text")))
        for k in ("view", "scratch"):
            MEAS.append(("ch:" + str(ui[k].cget("font")), ui[k].get("1.0", "end-1c") + Z.Zenko21.__dict__.get("_x", "")))
        return {"step": step, "rows": list(lb.get(0, "end")),
                "fg": [lb.itemcget(i, "fg") for i in range(lb.size())], "sel": list(lb.curselection()),
                "title": ui["page_title"].cget("text"), "view": ui["view"].get("1.0", "end-1c"),
                "view_state": str(ui["view"].cget("state")), "status": ui["status"].cget("text"),
                "preview": ui["preview"].cget("text"), "scratch": ui["scratch"].get("1.0", "end-1c"), "files": _files(),
                "nb_open": sorted(os.path.relpath(p, Z.NB_DIR) for p in ui["open"]),
                "life_nb_last": json.loads(json.dumps(app.life.get("nb_last"), default=str))}

    def _after(app, name):
        STEPS.clear()
        if name != "notes":
            return
        ui = app._nb_ui
        STEPS.append(_state(app, ["start"]))

        def pick(label):
            rows = [os.path.basename(r[1]) for r in ui["rows"]]
            i = rows.index(label)
            ui["tree"].selection_clear(0, "end")
            ui["tree"].selection_set(i)
            app.nb_tree_click()
            for _ in range(3):
                app.root.update()
        for lab in ["Cells", "membranes.md", "diagram.png", "Imported", "reMarkable", "Biology HL", "Biology HL"]:
            pick(lab)
            STEPS.append(_state(app, ["pick", lab]))
        app.nb_flush(quiet=False)
        app.root.update()
        STEPS.append(_state(app, ["flush"]))
        app.nb_undo()
        app.root.update()
        STEPS.append(_state(app, ["undo"]))
        pick("Chemistry HL")
        app.nb_new_page()
        app.root.update()
        STEPS.append(_state(app, ["new_page"]))
        ui["view"].insert("end", "Acids donate protons.")
        ui["view"].edit_modified(True)
        app.nb_page_save()
        STEPS.append(_state(app, ["page_save"]))

    def _tree(app, images, fonts):
        EXTRA[0] = {"steps": list(STEPS), "state": _state(app, ["final"]), "nb_dir": Z.NB_DIR}
        h = app.hub
        tr = walk(h["pages"]["notes"], h["win"], set(), images, fonts)
        intro = ("Pick a notebook on the left to open it.\n\nEverything you file from the Scratchpad lands in that subject\u2019s "
                 "\u201cQuick notes\u201d. Import a folder (like your reMarkable export) and its folders show up here too: a folder "
                 "called Bio, Chem, French... goes straight into that subject.")
        MEAS.append(("ch:" + str(h["pages"]["notes"].nametowidget(app._nb_ui["view"]).cget("font")), intro))
        for fnt, txt in MEAS:                    # (the labels' words at every step, for laying them out)
            if fnt.startswith("ch:"):
                f = tkfont.Font(font=fnt[3:])
                ent = fonts.setdefault(fnt[3:], {"ascent": f.metrics("ascent"), "descent": f.metrics("descent"), "actual": f.actual(), "w": {}})
                for ch in set(txt):
                    ent.setdefault("ch", {})[ch] = f.measure(ch)
                continue
            f = tkfont.Font(font=fnt)
            ent = fonts.setdefault(fnt, {"ascent": f.metrics("ascent"), "descent": f.metrics("descent"), "actual": f.actual(), "w": {}})
            ent["w"][txt] = f.measure(txt)
            for piece in wrap_pieces(txt):
                ent["w"][piece] = f.measure(piece)
        MEAS.clear()
        return tr
    SETUP["data"], SETUP["after"], SETUP["tree"] = _data, _after, _tree

if len(sys.argv) > 1 and sys.argv[1] == "garden":
    # golden/hub_garden.json: the Zen garden (the painting with Zenko, lanterns, fireflies, pond koi, decor; the side
    # panel) in four states. Its 50 ms animation is held still and run for exactly one frame at a fixed clock (after the
    # page has its final size), with random.random() fixed so no petal falls.
    OUT = "hub_garden.json"
    time.time = lambda: NOW[0]
    _mono = [5000.0]
    _mono0 = time.monotonic
    CASES = [("garden_bonsai", 1765488600.0, {"theme": "snow"}, None, "garden"),
             ("garden_seed_small", 1765440000.0 + 8 * 3600, {"theme": "midnight"}, (1120, 720), "garden"),
             ("garden_bloom_breath", 1765450800.0, {"theme": "sakura", "breath_minutes": 5}, None, "garden"),
             ("garden_sacred_fish", 1765476000.0, {"theme": "snow", "fish_gold": True, "garden_decor": ["deer", "momiji"]},
              None, "garden")]

    def _data(app, name):
        day = lambda n: (_dt.date.fromtimestamp(NOW[0]) + _dt.timedelta(days=n)).isoformat()
        G, F = app.garden(), app.fishing()
        app.garden_anim = lambda: None                 # (held still until the final frame)
        if name == "garden_bonsai":
            G.update(xp=400.0, health=0.8, seed=4321, active=[day(-3), day(-2)],
                     log=[{"d": day(-3), "t": "09:10", "m": 50, "k": "lockin"}, {"d": day(-2), "t": "16:45", "m": 30, "k": "focus"},
                          {"d": day(-2), "t": "20:05", "m": 25, "k": "lockin"}],
                     calm=[{"d": day(0), "t": "08:00", "k": "box", "m": 3.2}, {"d": day(-2), "t": "22:00", "k": "478", "m": 1.5}])
            F.update(bait=7.4, gold=0, pond=[["sanke", 77], ["ogon", 5], ["medaka", 9]], right=4, asked=6, depth=2, src="cards",
                     book={"sanke": {"n": 2, "best": 40.5, "first": day(-2), "seed": 77}, "ogon": {"n": 1, "best": 33.0, "first": day(-1), "seed": 5}})
        elif name == "garden_bloom_breath":
            G.update(xp=1500.0, health=1.0, seed=99, active=[day(-1), day(0)], grove=[{"done": day(-40)}, {"done": day(-9)}],
                     log=[{"d": day(0), "t": "10:00", "m": 40, "k": "lockin"}])
            app._ground_i = 2
        elif name == "garden_sacred_fish":
            G.update(xp=2000.0, health=1.0, seed=2024, active=[day(0)], grove=[{"done": day(-90)}] * 3,
                     log=[{"d": day(0), "t": "07:30", "m": 90, "k": "lockin"}])
            app.journey()["lock_min"] = 60 * 140
            F.update(bait=0.0, gold=3, free_day=day(0), free_used=3, pond=[["ryugu", 3], ["kitsunebi", 8], ["namazu", 4],
                     ["ryukin", 2], ["kohaku", 6], ["tsuki", 1], ["higoi", 5]], subj="chem")
        app.life_save()
        app.save_config() if hasattr(app, "save_config") else Z.save_config(app.cfg)

    def _after(app, name):
        pass

    def _final(app, name):
        after0 = app.root.after

        def quiet(ms, fn=None, *a):
            if ms == 50 or (ms == 60 and fn == app.garden_panel):   # (breath_start's second panel: made once below)
                return ""
            return after0(ms, fn, *a) if fn else after0(ms)
        app.root.after = quiet
        del app.garden_anim
        time.monotonic, rr0 = (lambda: _mono[0]), random.random
        random.random = lambda: 0.99
        ui = app.garden_ui
        if name == "garden_bloom_breath":
            app.breath_start("box")
            app.breath["t0"] = _mono[0] - 5.5
        if name == "garden_sacred_fish":
            app._z22_fish = {"phase": "wait", "t": _mono[0] - 1.0, "bx": 0.2, "by": -0.1, "depth": 3, "gold": True,
                             "q": {"q": "?", "a": "a", "opts": ["a"], "card": None, "subj": "chem", "diff": 3},
                             "items": {}, "ripples": [], "bite_at": 1e9}
        app.garden_panel()
        for _ in range(4):
            app.root.update()
        ui["key"], ui["anim_on"] = None, False
        app.garden_draw()
        for _ in range(3):
            app.root.update()
        app.root.after = after0
        time.monotonic, random.random = _mono0, rr0
        EXTRA[0] = {"tails": app.tail_count(), "pal": {k: v for k, v in app.pal.items() if isinstance(k, str)},
                    "hd": bool(Z.HD_ART["on"]), "breath": json.loads(json.dumps(getattr(app, "breath", None), default=str)),
                    "ground_i": int(getattr(app, "_ground_i", -1)), "side_w": int(ui["side"].cget("width")),
                    "cv_wh": [ui["cv"].winfo_width(), ui["cv"].winfo_height()]}
    SETUP["data"], SETUP["after"], SETUP["final"] = _data, _after, _final

EXTRA = [{}]
NAME = [""]
out = []
for name, t, cfg, size, page in CASES:
    NAME[0] = name
    NOW[0] = t
    HIST.clear()
    app = Z.Zenko23()
    windows_defaults(app.root)
    app.cfg.update(cfg)
    Z.apply_theme(app.cfg.get("theme", "snow"))
    app.pal, app.images = app.make_pal(), {}    # (as set_theme does: her colours follow the theme)
    app.root.update()
    if "data" in SETUP:
        SETUP["data"](app, name)
    if name in ("chat_long", "chat_typed"):
        app.chat_log = list(LONG_LOG[:3] if name == "chat_typed" else LONG_LOG)
    app.open_hub(page)
    h = app.hub
    w = h["win"]
    if size:
        w.geometry("%dx%d+0+0" % size)
    for _ in range(12):
        app.root.update()
        time.sleep(0.05)
    while time.monotonic() < getattr(app, "_count_until", 0) + 0.15:      # Home's count-up has finished
        app.root.update()
        time.sleep(0.02)
    if "after" in SETUP:
        SETUP["after"](app, name)
        for _ in range(6):
            app.root.update()
            time.sleep(0.05)
    if page != "chat":
        app.zk_fit()                       # the settled page: widgets rebuilt by a refresh get their fitted fonts too
        for _ in range(3):
            app.root.update()
    if "final" in SETUP:
        SETUP["final"](app, name)
    if name == "chat_typed":
        # her reply typed out by _chat_pump, a few letters every 14 ms
        app.chat_line("Fox", LONG_LOG[3][1])
        for _ in range(400):
            app.root.update()
            time.sleep(0.01)
            if not app._chat_typing:
                break
        for _ in range(5):
            app.root.update()
            time.sleep(0.02)
    images, fonts = {}, {}
    WIDE_PIECES[0] = "small" in name
    pg = h["pages"][page]
    tree = SETUP["tree"](app, images, fonts) if "tree" in SETUP else walk(pg, w, set(), images, fonts)
    texts = []

    def find_texts(wd):
        for k in wd.winfo_children():
            if k.winfo_class() == "Text":
                texts.append(text_rec(k, fonts))
            find_texts(k)
    find_texts(pg)
    if name == "chat_long":
        texts[0]["scrolls"] = scroll_rec(pg.nametowidget(texts[0]["path"]))
    out.append({"name": name, "now": t, "cfg_all": cfg, "page": page, "geometry": w.geometry(), "zk_f": app.zk_f,
                "chat_log": list(app.chat_log), "session": json.loads(json.dumps(app.session, default=str)),
                "brk": json.loads(json.dumps(app.brk, default=str)),
                "tt": json.loads(json.dumps(app.tt, default=str)), "deadlines": Z.load_deadlines(), "stats": Z.load_stats(),
                "cfg": {k: app.cfg.get(k) for k in ("last_task", "last_minutes", "music_presets", "user_name", "theme", "badge_view",
                                                     "remind_eyes", "remind_posture", "remind_water", "daynight", "bedtime",
                                                     "fox_name")},
                "life": json.loads(json.dumps({k: app.life.get(k) for k in ("journal", "cards", "badges", "btier", "badge_pins",
                                                                           "facts", "notes", "found", "stats", "fishing")}, default=str)),
                "life_all": json.loads(json.dumps(app.life, default=str)),
                "cfg_full": json.loads(json.dumps(app.cfg, default=str)),
                "font_ui": Z.FONT_UI, "exam_iso": app.exam_date().isoformat() if hasattr(app, "exam_date") else "",
                "ema_sel": getattr(app, "_ema_sel", None),
                "serif": Z._font_pick(app.root, ["Yu Mincho", "MS Mincho", "Georgia", "Times New Roman"], Z.FONT_UI),
                "well": dict(getattr(app, "well", {}) or {}), "jp": Z._jp_font(app.root),
                "mood_word": app.mood_word() if hasattr(app, "mood_word") else "", "hunger": app.hunger() if hasattr(app, "hunger") else 0,
                "spirit": app.spirit() if hasattr(app, "spirit") else 0, "streak": app.streak() if hasattr(app, "streak") else 0,
                "total_hours": app.total_hours() if hasattr(app, "total_hours") else 0,
                "pose": json.loads(json.dumps(getattr(app, "_pose", None) or {}, default=str)),
                "bond": list(app.bond_text()) if hasattr(app, "bond_text") else [],
                "fox_name": app.fox_name() if hasattr(app, "fox_name") else "",
                "catch": app.catchphrase() if hasattr(app, "catchphrase") else "",
                "z19_hours": app.z19_hours() if hasattr(app, "z19_hours") else 0,
                "journey": json.loads(json.dumps(app.journey_state(), default=str)) if hasattr(app, "journey_state") else [],
                "tree": tree, "texts": texts, "images": images, "fonts": fonts, **EXTRA[0]})
    EXTRA[0] = {}
    try:
        app.hub_close()
        app.root.destroy()
    except Exception:
        pass
with open(os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "golden", OUT), "w") as f:
    json.dump(out, f)
for c in out:
    print(c["name"], c["geometry"], len(c["images"]), list(c["fonts"]), [len(x["lines"]) for x in c["texts"]])
os._exit(0)
