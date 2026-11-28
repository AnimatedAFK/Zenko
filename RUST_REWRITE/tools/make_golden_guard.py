"""golden/guard.json: zenko.py's desktop leftovers (stage 3g): deadline_reminders, books_slow (wind-down, bedtime
nag + snooze/goodnight, burnout, rest day, late nights) with gym_water_nudge, screen_tick + screen_tell (screen time
recording: apps, sites, YouTube study videos, private and excluded windows, idle, the 1-hour nudges) and zk_slow_tick's
idle_quiz_check (pop quiz + its taps), ib_nudge_check, garden_check, screen_save_maybe and journey_check.
The clock (and time.monotonic) is pinned and Python's random seeded per step.
Steps: "@fg title|proc|idle|n" = n screen ticks 1 s apart; "@books N" = N more seconds in this hour's hourlog, then
books_slow + gym_water_nudge; "@water" = gym_water_nudge; "@dl" = deadline_reminders; "@zk" = zk_slow_tick's checks;
"@slog N" = N more seconds of bio today; "@tap k i" = press button i of the step's k-th bubble with buttons;
"@endbreak"; "@at HH:MM" / "@next HH:MM" = move the clock to that time today / tomorrow.
Run: xvfb-run -a -s "-screen 0 1920x1080x24" env PYTHONDONTWRITEBYTECODE=1 /usr/bin/python3.12 tools/make_golden_guard.py"""
import sys
sys.dont_write_bytecode = True
import json, os, tempfile, types, random
os.environ["TZ"] = "UTC"
import time  # noqa: E402
time.tzset()
ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
os.environ["APPDATA"] = tempfile.mkdtemp()
sys.path.insert(0, ROOT)
import zenko as Z  # noqa: E402
import datetime as _dt  # noqa: E402

NOW = [1765440000.0]          # Thursday 11 Dec 2025, 08:00


class FDT(_dt.datetime):
    @classmethod
    def now(cls, tz=None):
        return cls.utcfromtimestamp(NOW[0]) if tz is None else _dt.datetime.fromtimestamp(NOW[0], tz)

    @classmethod
    def fromtimestamp(cls, t, tz=None):
        return cls.utcfromtimestamp(t) if tz is None else _dt.datetime.fromtimestamp(t, tz)


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
Z.idle_seconds = lambda: 0.0
_sleep = time.sleep
time.time = lambda: NOW[0]


def day(n):
    return (_dt.date(2025, 12, 11) + _dt.timedelta(days=n)).isoformat()


DEADLINES = [{"id": "bm1", "title": "Bio MYA", "kind": "Exam", "date": day(2)},
             {"id": "ch1", "title": "Chemistry test", "kind": "Quiz", "date": day(3), "time": "10:00"},
             {"id": "fr1", "title": "French oral", "kind": "Presentation", "date": day(10)},
             {"id": "en1", "title": "English essay", "kind": "Assignment", "date": day(1), "done": True}]
LIFE = {"slog": dict([(day(-k), {"bio": 7300 + 60 * k}) for k in range(1, 8)]),
        "hourlog": {day(-1): {"23": [2400, 0]}, day(-2): {"23": [1900, 0]}, day(-3): {"22": [2500, 0], "23": [2000, 0]},
                    day(-4): {"1": [3000, 0]}, day(-5): {"23": [1000, 0]}},
        "cards": [{"id": "c1", "q": "What is ATP?", "a": "The cell's energy currency", "box": 1, "wrong": 2, "right": 1,
                   "due": day(-1), "subject": "bio"},
                  {"id": "c2", "q": "Avogadro's number?", "a": "6.022e23", "box": 0, "wrong": 0, "right": 0, "due": day(0)},
                  {"id": "c3", "q": "Not due yet", "a": "later", "box": 3, "wrong": 0, "right": 3, "due": day(5)}],
        "ibtasks": [{"id": "ia1", "kind": "ia", "subject": "bio", "unit": "words", "target": 2200, "words": 800,
                     "title": "Bio IA", "due": day(2), "stages": ["Topic"], "updated": day(-1), "hist": []},
                    {"id": "ee1", "kind": "ee", "subject": "", "unit": "words", "target": 4000, "words": 0,
                     "title": "Extended Essay", "due": day(10), "stages": ["Topic", "Research"], "updated": day(-6),
                     "hist": []}]}
os.makedirs(Z.DATA_DIR, exist_ok=True)
with open(os.path.join(Z.DATA_DIR, "deadlines.json"), "w") as f:
    json.dump(DEADLINES, f, indent=1)
ORIG = {n: getattr(Z.Zenko23, n) for n in ("deadline_reminders", "journey_check")}
for name in ("_life_slow", "deadline_reminders", "check_timetable", "update_tick", "journey_check", "z22_badge_check",
             "study_slow_tick", "tick", "books_slow_tick", "zk_slow_tick"):
    setattr(Z.Zenko23, name, lambda self, *a, **k: None)      # the timers: these steps call the checks themselves
app = Z.Zenko23()
app.root.update()
for k in LIFE:
    app.life[k] = json.loads(json.dumps(LIFE[k]))
app.life["garden"]["active"] = [day(-4)]
app.life["garden"]["health"] = 1.0
app.life_save()
time.monotonic = lambda: NOW[0]
app._quiz_offer_t = NOW[0] - 25 * 60
if app.bubble_job:                                            # the start-up greeting's bubble: gone
    app.root.after_cancel(app.bubble_job)
    app.bubble_job = None

STEPS = ["@dl", "@dl", "@water", "@at 13:00", "@water", "@water", "@zk", "@tap 0 0", "@tap 0 0",
         "@fg Lofi beats to study to - YouTube - Google Chrome|chrome.exe|0|3700",
         "@fg Cell respiration explained - biology - YouTube - Google Chrome|chrome.exe|0|6",
         "@fg Instagram - Google Chrome|chrome.exe|0|30",
         "@fg New Incognito Tab - Google Chrome|chrome.exe|0|10",
         "@fg HDFC NetBanking - Google Chrome|chrome.exe|0|10",
         "@fg main.rs - zenko - Visual Studio Code|code.exe|0|3605",
         "@fg main.rs - zenko - Visual Studio Code|code.exe|200|10",
         "@fg Zenko|python.exe|0|3601",
         "@fg Reddit - Dive into anything - Google Chrome|chrome.exe|0|20",
         "@fg Some doc - Personal - Microsoft Edge|msedge.exe|0|5",
         "@fg Wikipedia, the free encyclopedia - Mozilla Firefox|firefox.exe|0|5",
         "@fg |notepad.exe|0|3", "@fg |my_tool-v2.exe|0|3", "@fg Untitled|unknown|0|3",
         "@zk", "@tap 0 1", "@zk", "@zk",
         "@books 0", "@books 60", "@books 60", "@books 60", "@slog 24000", "@books 60", "@tap 0 0", "@books 60",
         "@endbreak", "@at 22:05", "@books 0", "@books 30", "@at 22:35", "@books 30", "@books 30", "@at 23:10",
         "@books 100", "@tap 0 0", "@books 30", "@at 23:22", "@books 30", "@at 23:35", "@books 30", "@tap 0 1",
         "@next 00:30", "@books 30",
         "@next 08:00", "@dl", "@zk", "@at 11:00", "@zk", "@tap 0 0", "@tap 0 1"]
caught, cmds, last = [], [], []
app.chat_line = lambda who, text: (caught.append(["line", who, text]), app.chat_log.append((who, text)))


def _say(text, secs=9, choices=None):
    caught.append(["say", text, [c[0] for c in (choices or [])]])
    if choices:
        last.append(choices)


app.say = _say
_tell = app.screen_tell
app.screen_tell = lambda *a: (random.seed(6500 + STEP[0]), _tell(*a))   # other after() jobs may use random first
STEP = [0]
app.speak = lambda *a, **k: None
app.open_hub = lambda page=None, *a, **k: cmds.append("open_hub:%s" % page)
app.lockin_start = lambda mins=None, *a, **k: cmds.append("lockin:%s" % mins)
out = [{"deadlines": DEADLINES, "life": LIFE, "now": NOW[0], "garden_active": [day(-4)]}]
for i, q in enumerate(STEPS):
    random.seed(6000 + i)
    STEP[0] = i
    del caught[:]
    del cmds[:]
    if q.startswith("@tap "):
        k, j = map(int, q.split()[1:])
        if k >= len(last):
            print("NO BUTTONS at", i, q, file=sys.stderr); os._exit(1)
        ch = last[k]
        del last[:]
        ch[j][1]()
    else:
        del last[:]
    if q.startswith("@fg "):
        title, proc, idle, n = q[4:].split("|")
        for _ in range(int(n)):
            NOW[0] += 1.0
            app._last_fg = (title, proc, float(idle), None)
            app.screen_tick()
    elif q.startswith("@books "):
        NOW[0] += 10.0
        n = int(q.split()[1])
        if n:
            now = _dt.datetime.utcfromtimestamp(NOW[0])
            h = app.life["hourlog"].setdefault(now.date().isoformat(), {}).setdefault(str(now.hour), [0, 0])
            h[0] += n
        app.books_slow()
        app.gym_water_nudge()
    elif q == "@water":
        NOW[0] += 10.0
        app.gym_water_nudge()
    elif q == "@dl":
        NOW[0] += 10.0
        ORIG["deadline_reminders"](app)
    elif q == "@zk":
        NOW[0] += 60.0
        for f in (app.idle_quiz_check, app.ib_nudge_check, app.garden_check, app.screen_save_maybe,
                  lambda: ORIG["journey_check"](app)):
            f()
    elif q.startswith("@slog "):
        d = app.life["slog"].setdefault(day(0), {})
        d["bio"] = d.get("bio", 0) + int(q.split()[1])
    elif q == "@endbreak":
        app.end_break(False)
    elif q.startswith(("@at ", "@next ")):
        h, m = map(int, q.split()[1].split(":"))
        d = _dt.datetime.utcfromtimestamp(NOW[0]).date() + _dt.timedelta(days=1 if q.startswith("@next") else 0)
        NOW[0] = _dt.datetime(d.year, d.month, d.day, h, m, tzinfo=_dt.timezone.utc).timestamp()
    _sleep(0 if q.startswith(("@at", "@next", "@slog")) else 0.3)   # screen_tell's after(10) belongs to this step
    for _ in range(3):
        app.root.update()
    L = app.life
    out.append({"q": q, "now": NOW[0], "says": [c for c in caught if c[0] == "say"],
                "cmds": list(cmds),
                "life": json.loads(json.dumps({k: L.get(k) for k in ("screen", "said", "quiz_offers", "cards", "stats")})),
                "water": json.loads(json.dumps(L.get("gym", {}).get("water", {}))),
                "note": app.cfg.get("last_deadline_note"), "brk": bool(app.brk),
                "paused": bool(app.session and app.session.get("paused"))})
with open(os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "golden", "guard.json"), "w") as f:
    json.dump(out, f, ensure_ascii=False, indent=0)
for o in out[1:]:
    print(repr(o["q"]), "->", o["says"], o["cmds"], o["brk"])
os._exit(0)
