"""golden/checkin.json: zenko.py's daily check-in card (open_checkin) built by the real method in real Tk, with the
widget options Windows Tk would use by default (labels: borderwidth 2, padx/pady 1; entries: borderwidth 1). For
each case it records every widget's requested size and place, every item on every canvas (the card and the pixel
buttons), the font numbers Tk used, and what checkin_save writes. The Rust test (tests/checkin.rs) builds the same
window with the same font numbers and must match all of it.
Run: xvfb-run -a /usr/bin/python3.12 tools/make_golden_checkin.py"""
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

NOW = [1764151201.0]           # 2025-11-26 10:00:01 UTC
time.time = lambda: NOW[0]


class FD(_dt.date):
    @classmethod
    def today(cls):
        return _dt.datetime.utcfromtimestamp(NOW[0]).date()


fake = types.ModuleType("datetime")
fake.__dict__.update(_dt.__dict__)
fake.date = FD
Z.datetime = fake
OUT = []
Z.beep = lambda kind, *a, **k: OUT.append(["beep", kind])
Z.ui_sound = lambda *a, **k: None
Z.random.choice = lambda seq: seq[0]


def windows_defaults(root):
    for k, v in (("*Label.borderWidth", 2), ("*Label.padX", 1), ("*Label.padY", 1), ("*Label.highlightThickness", 0),
                 ("*Entry.borderWidth", 1), ("*Frame.borderWidth", 0), ("*Frame.highlightThickness", 0)):
        root.option_add(k, v)


def walk(w):
    rec = {"cls": w.winfo_class(), "x": w.winfo_x(), "y": w.winfo_y(), "w": w.winfo_width(), "h": w.winfo_height(),
           "rw": w.winfo_reqwidth(), "rh": w.winfo_reqheight(), "mapped": bool(w.winfo_ismapped()), "kids": []}
    if rec["cls"] == "Canvas":
        items = []
        for i in w.find_all():
            t = w.type(i)
            it = {"type": t, "coords": [round(c, 3) for c in w.coords(i)]}
            for opt in ("fill", "outline", "text"):
                try:
                    it[opt] = w.itemcget(i, opt)
                except tk.TclError:
                    pass
            items.append(it)
        rec["items"] = items
    for k in w.winfo_children():
        rec["kids"].append(walk(k))
    return rec


cases = []
for theme, name in (("snow", ""), ("sakura", "Anoop Kumar"), ("midnight", "Mio")):
    Z.apply_theme(theme)
    root = tk.Tk()
    windows_defaults(root)
    root.geometry("+300+500")
    root.update_idletasks()
    o = Z.Zenko23.__new__(Z.Zenko23)
    o.root, o.checkin_win, o.session, o.lockin = root, None, None, None
    o.cfg = {"user_name": name, "theme": theme}
    o.W, o.COLW, o.U = 437, 250, 1.0
    moods = []
    o.set_mood = lambda m, s=0, moods=moods: moods.append([m, s])
    o.open_checkin(force=True)
    w = o.checkin_win
    w.update_idletasks()
    w.update()
    fonts = {}
    texts = ["0", "Kon kon~ how are you feeling today%s?" % ((", " + name.split(" ")[0]) if name else ""),
             "(optional note: what's on your mind?)", "Not now"] + [m[1] for m in Z.MOODS]
    for size, bold in ((11, True), (11, False), (8, False), (9, True), (8, True)):
        f = tkfont.Font(family=Z.FONT_UI, size=size, weight="bold" if bold else "normal")
        fonts["%d%s" % (size, "b" if bold else "")] = {"ascent": f.metrics("ascent"), "descent": f.metrics("descent"),
                                                        "w": {t: f.measure(t) for t in texts}}
    tree = walk(w)
    geo = [w.winfo_x(), w.winfo_y(), w.winfo_reqwidth(), w.winfo_reqheight()]
    case = {"theme": theme, "name": name, "fonts": fonts, "tree": tree, "geo": geo, "screen": [w.winfo_screenwidth(), w.winfo_screenheight()],
            "root": [root.winfo_x(), root.winfo_y()], "opened": list(OUT) + [["mood"] + m for m in moods]}
    OUT.clear()
    moods.clear()
    cases.append(case)
    root.destroy()

# checkin_save: what lands in zenko_life.json and what she does
saves = []
Z.apply_theme("snow")
root = tk.Tk()
root.withdraw()
o = Z.Zenko23.__new__(Z.Zenko23)
o.root, o.checkin_win, o.session, o.lockin = root, None, None, None
o.cfg = {}
said, awards, moods = [], [], []
o.say = lambda text, secs=0, *a, **k: said.append([text, secs])
o.chat_line = lambda who, text: said.append([who, text])
o.set_mood = lambda m, s=0: moods.append([m, s])
o.award = lambda bid: awards.append(bid)
o.daily_refresh = lambda: None
o.life = {"journal": []}
o.life_save = lambda: None
for day, (mood, note) in enumerate([("great", "  exams soon  "), ("good", ""), ("okay", "x" * 350), ("meh", "tired"),
                                    ("rough", "\tbad day\n"), ("good", "again"), ("great", ""), ("okay", "same day")]):
    NOW[0] = 1764151201.0 + 86400 * min(day, 6)
    o.checkin_save(mood, note)
    saves.append({"now": NOW[0], "mood": mood, "note": note, "journal": json.loads(json.dumps(o.life["journal"])),
                  "last": o.life["last_checkin"], "said": list(said), "beeps": list(OUT), "moods": list(moods),
                  "awards": list(awards)})
    for v in (said, OUT, moods, awards):
        v.clear()
root.destroy()
with open(os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "golden", "checkin.json"), "w") as f:
    json.dump({"cases": cases, "saves": saves}, f, indent=1)
print(json.dumps(cases[0]["tree"])[:1200])
print(cases[0]["geo"], cases[0]["fonts"]["11b"]["ascent"], cases[0]["fonts"]["11b"]["descent"])
