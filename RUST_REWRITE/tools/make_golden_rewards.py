"""golden/rewards.json: what zenko.py's final class (Zenko23) writes to zenko_life.json and says when a session or a
lock-in ends, or the zenko22 badge tick runs, for a set of starting files and a pinned clock (UTC). The real methods
run (start-up setdefaults, end_session's reward chain, lockin_stop's chain, z22_badge_check); only the parts that need
a screen are stubbed. The Rust test (tests/rewards.rs) replays each case and must write the same bytes."""
import sys
sys.dont_write_bytecode = True
import json, os, tempfile, types
os.environ["TZ"] = "UTC"
import time  # noqa: E402
import warnings  # noqa: E402
warnings.simplefilter("ignore")
time.tzset()
ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
os.environ["APPDATA"] = tempfile.mkdtemp()
sys.path.insert(0, ROOT)
import zenko as Z  # noqa: E402
import datetime as _dt  # noqa: E402

NOW = [0.0]
time.time = lambda: NOW[0]


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
Z.random.random = lambda: 0.99          # no 50% "that session fed me" line; Rust gets no random say either
Z.random.randrange = lambda a, b=None: 4242
Z.BackgroundMusic = lambda: None
OUT = []
Z.beep = lambda kind, *a, **k: OUT.append(["beep", kind])


class Root:
    def __init__(self):
        self.jobs = []

    def after(self, ms, fn=None, *a):
        if fn is not None:
            self.jobs.append((ms, fn))
        return "job"

    def __getattr__(self, k):
        return lambda *a, **kw: None


NOOP = ("tt_refresh", "daily_refresh", "map_refresh", "up_refresh", "chat_line", "garden_refresh", "music_session_hook",
        "music_apply", "badges_refresh", "fish_refresh", "hub_refresh", "z22_badges_refresh", "zk_on_map")


def make(cfg, extra):
    H = type("H", (Z.Zenko23,), {"__init__": lambda s: None})
    for k in NOOP:
        setattr(H, k, lambda s, *a, **kw: None)
    o = H()
    o.root, o.cfg, o.extra = Root(), dict({"pet_needs": False}, **cfg), dict(extra)
    o.session = o.lockin = o.brk = o.quiz = None
    o.hub = None
    o.images = {}
    o.FONT_CLASSES = ()
    o._z22_metrics_t = 0.0
    def say(text, secs=6, chips=None, *a, **k):
        if isinstance(getattr(o, "_say_cap", None), list):
            o._say_cap.append(text)
        OUT.append(["say", text, secs] + ([[c[0] for c in chips]] if chips else []))
    o.say = say
    o.speaker = types.SimpleNamespace(say=lambda text, *a: OUT.append(["speak", text] + list(a)))
    o.tt = Z.load_tt()
    o.announced, o._tt_last = set(), 0.0
    o.set_mood = lambda m, secs=0, *a, **k: OUT.append(["mood", m, secs])
    o.save_extra = lambda: None
    o.life_init()
    o.study_init()
    o.books_init()
    o.z22_migrate()
    return o


def drain(o):
    """Run the after() jobs in time order, as the Rust side runs its Later queue."""
    said = []
    while o.root.jobs:
        jobs, o.root.jobs = sorted(o.root.jobs, key=lambda j: j[0]), []
        for ms, fn in jobs:
            n0 = len(OUT)
            fn()
            for e in OUT[n0:]:
                said.append([ms / 1000.0] + e)
            del OUT[n0:]
    return said


def read(name):
    p = os.path.join(Z.DATA_DIR, name)
    return open(p, encoding="utf-8").read() if os.path.exists(p) else None


def wipe():
    for n in ("zenko_life.json", "stats.json", "zenko_extras.json", "timetable.json"):
        p = os.path.join(Z.DATA_DIR, n)
        if os.path.exists(p):
            os.remove(p)


def day(off):
    return (_dt.datetime.utcfromtimestamp(NOW[0]).date() - _dt.timedelta(days=off)).isoformat()


def run(case):
    wipe()
    NOW[0] = case["now"]
    os.makedirs(Z.DATA_DIR, exist_ok=True)
    files = {}
    for name, body in case.get("files", {}).items():
        files[name] = body if isinstance(body, str) else json.dumps(body)
        with open(os.path.join(Z.DATA_DIR, name), "w", encoding="utf-8") as f:
            f.write(files[name])
    del OUT[:]
    o = make(case.get("cfg", {}), case.get("extra", {}))
    o.root.jobs = []                    # start-up jobs (music, ticks) are not part of this
    startup = read("zenko_life.json")
    del OUT[:]
    act = case["act"]
    NOW[0] += act.get("later", 0.0)
    if act["do"] == "session":
        def base_end(self, completed):  # FoxBuddy.end_session, reward part only
            s, self.session = self.session, None
            focused = max(0.0, s["elapsed"] - s["dist"] - s["away"])
            Z.add_stats(focused)
            self.life_session_done(s, completed, focused)
            if completed and s.get("item") is not None:
                self.tt_mark(s["item"], True)
        Z.FoxBuddy.end_session = base_end
        o.session = {"task": act["task"], "planned": act["planned"], "elapsed": act["elapsed"], "dist": act["dist"],
                     "away": act["away"],
                     "item": o.tt[act["item"]] if "item" in act else None}
        o.end_session(act["completed"])
    elif act["do"] == "lockin":
        o.lockin = {"until": NOW[0] + 600, "eaten": act["eaten"], "t0": NOW[0] - act["mins"] * 60.0}
        o.lockin_stop(act["done"], False)
    elif act["do"] == "tt":
        Z.IS_WIN = True
        for off in act["steps"]:
            NOW[0] = case["now"] + off
            o.check_timetable(1000.0 + off)
            OUT.append(["step", off])
    elif act["do"] == "tick":
        o.z22_badge_check()
    now_out = [[0.0] + e for e in OUT]
    del OUT[:]
    later = drain(o)
    return {"name": case["name"], "now": case["now"], "files": files, "cfg": case.get("cfg", {}),
            "act": act, "startup": startup, "life": read("zenko_life.json"), "stats": read("stats.json"),
            "timetable": read("timetable.json"),
            "out": now_out, "later": later}


T = 1764151200.25   # 2025-11-26 10:00:00.25 UTC, a Wednesday
NOW[0] = T
gl = {"xp": 240.0, "health": 0.8, "active": ["2025-11-20"], "log": [{"d": "2025-11-20", "m": 50, "k": "lockin"},
      {"d": "2025-11-21", "m": 30.0, "k": "focus"}], "grove": [], "seed": 77, "planted": "2025-10-01"}
streak_stats = {day(i): 1500 for i in range(1, 8)}
big_stats = {day(i): 36000 for i in range(1, 12)}
CASES = [
    {"name": "fresh_session_done", "now": T,
     "act": {"do": "session", "task": "Biology notes", "planned": 1500, "elapsed": 1500, "dist": 2, "away": 0,
             "completed": True}},
    {"name": "fresh_session_stopped_short", "now": T,
     "act": {"do": "session", "task": "Maths", "planned": 3000, "elapsed": 420, "dist": 30, "away": 0,
             "completed": False}},
    {"name": "stopped_long_grows_garden", "now": T, "files": {"zenko_life.json": {"garden": gl, "hunger": 70.0,
                                                                                    "hunger_t": T - 7200}},
     "act": {"do": "session", "task": "Chem", "planned": 3000, "elapsed": 1900, "dist": 100, "away": 60,
             "completed": False}},
    {"name": "early_marathon_streak", "now": 1764136800.0, "files": {"stats.json": streak_stats},
     "act": {"do": "session", "task": "Long", "planned": 5400, "elapsed": 5400, "dist": 0, "away": 0,
             "completed": True}},
    {"name": "garden_stage_up_and_journey", "now": T,
     "files": {"stats.json": big_stats, "zenko_life.json": {"garden": dict(gl, xp=590.0), "badges": {"first": "x"}}},
     "act": {"do": "session", "task": "Psych", "planned": 3000, "elapsed": 3000, "dist": 0, "away": 0,
             "completed": True}},
    {"name": "lockin_done", "now": T, "act": {"do": "lockin", "eaten": 3, "mins": 50.0, "done": True}},
    {"name": "lockin_off_early", "now": T, "files": {"zenko_life.json": {"garden": gl}},
     "act": {"do": "lockin", "eaten": 1, "mins": 12.5, "done": False}},
    {"name": "lockin_off_too_short", "now": T, "act": {"do": "lockin", "eaten": 0, "mins": 2.0, "done": False}},
    {"name": "fri13_lockin", "now": 1771012800.0, "act": {"do": "lockin", "eaten": 2, "mins": 25.0, "done": True}},
    {"name": "fullmoon_lockin", "now": 1767470400.0, "act": {"do": "lockin", "eaten": 1, "mins": 60.0, "done": True}},
    {"name": "newyear_3am_session", "now": 1767236400.0,
     "act": {"do": "session", "task": "Night", "planned": 1500, "elapsed": 1500, "dist": 0, "away": 0,
             "completed": True}},
    {"name": "birthday_session", "now": T, "files": {"zenko_life.json": {"facts": {"birthday": "11-26"}}},
     "act": {"do": "session", "task": "Bday", "planned": 1500, "elapsed": 1600, "dist": 10, "away": 5,
             "completed": True}},
    {"name": "timetable_alerts", "now": T, "cfg": {"tts_voice": "win:Zira", "tts_speed": 1},
     "files": {"timetable.json": json.dumps([
         {"start": "10:04", "task": "Bio *flashcards* ~ \"cells\"", "mins": 45, "daily": True},
         {"start": "9:55", "task": "French", "mins": "20", "date": "2025-11-26"},
         {"start": "10:20", "task": "Done already", "mins": 30, "daily": True, "done": ["2025-11-26"]},
         {"start": "25:00", "task": "bad"}, {"start": "11am", "task": ""},
         {"start": "11am", "task": "A really long timetable task name that gets clipped", "mins": 10,
          "date": "2025-11-26"}])},
     "act": {"do": "tt", "steps": [0, 5, 12, 25, 40, 3600 - 300, 3600 - 290, 3600 - 280, 3605]}},
    {"name": "timetable_english_voice", "now": T, "cfg": {"remind_voice": "english", "tts_voice": "win:Zira", "tts_speed": 1},
     "files": {"timetable.json": json.dumps([{"start": "10:03", "task": "**Maths** ~ \"vectors\"", "mins": 25,
                                              "daily": True}])},
     "act": {"do": "tt", "steps": [0, 200]}},
    {"name": "timetable_quiet", "now": T, "cfg": {"remind_aloud": False},
     "files": {"timetable.json": json.dumps([{"start": "10:03", "task": "Chem", "mins": 25, "daily": True}])},
     "act": {"do": "tt", "steps": [0, 200]}},
    {"name": "timetable_item_done_prepared", "now": T,
     "files": {"timetable.json": json.dumps([{"start": "09:30", "task": "Psych revision", "mins": 30, "daily": False,
                                              "date": "2025-11-26", "plan": "p1", "done": []},
                                             {"start": "08:00", "task": "Psych 2", "mins": 20, "plan": "p1",
                                              "date": "2025-11-25", "done": ["2025-11-25"]}])},
     "act": {"do": "session", "task": "Psych revision", "planned": 1800, "elapsed": 1800, "dist": 0, "away": 0,
             "completed": True, "item": 0}},
    {"name": "timetable_ema_last_bit", "now": T,
     "files": {"timetable.json": json.dumps([{"start": "09:00", "task": "IA draft", "mins": 50, "plan": "ema:e7",
                                              "date": "2025-11-26"},
                                             {"start": "09:00", "task": "IA draft", "mins": 50, "plan": "ema:e7",
                                              "date": "2025-11-20"}]),
               "zenko_life.json": {"ema": [{"id": "e7", "text": "Finish the maths IA draft before Friday", "at": None,
                                            "made": 1.0, "done": False}]}},
     "act": {"do": "session", "task": "IA draft", "planned": 3000, "elapsed": 3000, "dist": 0, "away": 0,
             "completed": True, "item": 0}},
    {"name": "badge_tick_tiers", "now": T,
     "files": {"stats.json": big_stats, "zenko_life.json": {"stats": {"sessions": 26, "lockins": 11}, "badges": {}}},
     "act": {"do": "tick"}},
]
out = [run(c) for c in CASES]
dst = os.path.join(os.path.dirname(__file__), "..", "golden", "rewards.json")
os.makedirs(os.path.dirname(dst), exist_ok=True)
with open(dst, "w", encoding="utf-8") as f:
    json.dump(out, f, ensure_ascii=False, indent=1)
for r in out:
    print(r["name"], len(r["life"] or ""), [e[1:3] for e in r["out"] + r["later"] if e[1] == "say"][:4])
