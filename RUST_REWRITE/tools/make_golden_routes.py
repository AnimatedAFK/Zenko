"""golden/routes.json: zenko.py's later version chat routes (Zenko22: fishing, koi book, passport, badges, square,
browser link; z17: periodic table, calculator, dictionary, Bible, close/keep rules; z15: screen time, map, updates;
ib_route: IA / EE / TOK words, limits, due dates, exam countdown; the garden and delay picker lines), offline, with the
clock pinned, Python's random seeded per line and some screen time, garden, focus stats and a browser link set up first.
Run: xvfb-run -a -s "-screen 0 1920x1080x24" env PYTHONDONTWRITEBYTECODE=1 /usr/bin/python3.12 tools/make_golden_routes.py"""
import sys
sys.dont_write_bytecode = True
import json, os, tempfile, types, random
os.environ["TZ"] = "UTC"
import time  # noqa: E402
time.tzset()
ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
os.environ["APPDATA"] = tempfile.mkdtemp()
os.environ["FOX_FOOD_BASE"] = "http://127.0.0.1:9"     # online food lookups fail at once (the port has none yet)
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
_sleep = time.sleep
time.time = lambda: NOW[0]

Z.Zenko.ema_tick = lambda self: None
TODAY = "2025-12-11"
os.makedirs(Z.DATA_DIR, exist_ok=True)
with open(os.path.join(Z.DATA_DIR, "stats.json"), "w") as f:
    json.dump({"2025-12-01": 30000, "2025-12-05": 9000.5}, f)
for name in ("_life_slow", "study_missed", "deadline_reminders", "check_timetable", "update_tick", "journey_check",
             "z22_badge_check"):
    setattr(Z.Zenko23, name, lambda self, *a, **k: None)      # the timers: these lines are about chat only
app = Z.Zenko23()
app.root.update()
LIFE = {"screen": {"days": {TODAY: {"apps": {"Google Chrome": 5000.5, "VS Code": 1830.2, "Zenko": 400, "Word": 75.3},
                                    "sites": {"YouTube": 3725.0, "Other websites": 900, "Reddit": 59.6},
                                    "hours": [0] * 24}}},
        "garden": {"xp": 200.0, "health": 0.9, "active": ["2025-12-08"], "log": [], "grove": [], "seed": 7,
                   "planted": "2025-11-01"},
        "badges": {"first": "2025-12-01", "spotless": "2025-12-02"},
        "fishing_book": {"koi": {"n": 1}, "carp": {"n": 2}}}
for k in ("screen", "garden", "badges"):
    app.life[k] = json.loads(json.dumps(LIFE[k]))
app.life["fishing"]["book"] = dict(LIFE["fishing_book"])
app.life_save()
BROWSER = {"time": NOW[0] + 100, "title": "Photosynthesis - Wikipedia", "host": "en.wikipedia.org", "version": "0.4.0"}
with open(os.path.join(Z.DATA_DIR, "browser_now.json"), "w") as f:
    json.dump(BROWSER, f)

CONVO = ["go fishing", "cast a line", "catch a koi", "my koi book", "fish dex", "my passport", "badges", "global chat",
         "square", "browser", "are you linked?", "periodic table", "open the calculator", "cg50", "french dictionary",
         "bible", "open bible at john 3:16", "read psalm 23", "read harry 5", "bible at nonsense words",
         "read 1 cor 13", "keep youtube if it has biology, chem", "always close reddit",
         "block tiktok when the title has dance", "never close notion", "my rules", "close rules", "screen time",
         "open my screen time", "where did my day go", "how long have i been on youtube today",
         "how long was i on netflix", "how long did i spend on code", "world map", "where are we", "journey",
         "check for updates", "any updates?", "my bio ia is 1,200 words", "bio ia at 3400 words",
         "my ee is 2500 words", "tok essay is 900 words", "tok exhibition 400 words", "ee limit 4000",
         "ee is now 4,100 words", "my maths ia is at 12 pages", "math ia is 1 page", "math ia is due on 12th march",
         "ee is due by 2026-01-20", "tok essay due 5 dec", "chem ia due march 3 2026", "psych ia due 3/2/2026",
         "tok exhibition due tomorrow", "ia tracker", "ib tasks", "exam countdown", "how many days until my exams",
         "garden", "how's my tree", "pick what to delay", "delay some things", "pause screen time", "screen time",
         "Zenko, please go fishing", "hey zenko can you open the square",
         # ZenkoGym: the Exercise tab by chat
         "what's my workout", "i weigh 68 kg", "i am 150 lbs", "weight is 67.5 kg", "i ate 2 eggs and toast",
         "i ate 150 g rice with dal", "had a large pizza", "log food: protein bar 210 kcal 20 g protein",
         "ate 3 bryani", "food: half a chicken biryani, 2 rotis and a small banana", "i ate glorp and 2 eggs",
         "i just ate two slices of bread with butter for breakfast", "i drank 2 glasses of water", "water",
         "+1 water", "drank a bottle of water", "today is back day", "what's my workout", "today is day 3",
         "i'm resting today", "gym today", "chest day today", "it's my pull day", "slept badly", "i slept 4/5",
         "slept really well", "open the gym", "i weigh 20 kg", "ate 1/2 apple", "ate 250ml milk and 2 bananas each",
         # ZenkoBooks: the 急 delay, grades, books, bedtime, downloads, facts and tips
         "add 9am history 30 daily", "add 4pm study renal 45", "add 11:50pm late reading 30", "start reading 25",
         "something came up", "i need to talk to my parents for 30 min", "im back", "delay 20 min", "undo delay",
         "undo delay", "resume", "push back everything by 1 hour", "i got 34/40 on bio paper 1",
         "got a 6 in chem mya", "i scored 72% on my psych test", "bio mid year got 30 out of 40",
         "i got 50/40 in maths", "biology result 36/40 mock", "my grades", "vocab drill", "open my bio book",
         "open study book", "my bookshelf", "bedtime 22:30", "bedtime 11pm", "bedtime 25", "download status",
         "get my eyes", "bio fact", "give me an english tip", "paper 1 tip"]
out = [{"now": NOW[0], "life": LIFE, "browser": BROWSER, "stats": {"2025-12-01": 30000, "2025-12-05": 9000.5}}]
caught, cmds = [], []
app.chat_line = lambda who, text: (caught.append(["line", who, text]), app.chat_log.append((who, text)))
app.ask_llm = lambda q: caught.append(["llm", q])
app.say = lambda text, secs=9, choices=None: caught.append(["say", text, [c[0] for c in (choices or [])]])
app.speak = lambda *a, **k: None
app.open_hub = lambda page=None, *a, **k: cmds.append("open_hub:%s" % page)
app.book_open = lambda k, anim=True: cmds.append("book_open:%s" % k)
app.brain_pull = lambda m: cmds.append("brain_pull:%s" % m)
for name in ("fish_cast", "koi_book", "open_ptable", "open_calc", "open_french_dict", "update_check_now",
             "delay_picker"):
    setattr(app, name, (lambda n: lambda *a, **k: cmds.append(n))(name))
app.open_bible = lambda ref=None: cmds.append("open_bible:%s" % (ref or ""))
for i, q in enumerate(CONVO):
    random.seed(3000 + i)
    NOW[0] += 7.0
    del caught[:]
    del cmds[:]
    app.chat_send(q)
    _sleep(0.8)
    for _ in range(3):
        app.root.update()
    fox = [c[2] for c in caught if c[0] == "line" and c[1] == "Fox"]
    got = [c for c in caught if c[0] != "say"]
    says = [c for c in caught if c[0] == "say" and not any(f.split("\n")[0] == c[1] for f in fox)]
    out.append({"q": q, "now": NOW[0], "out": got, "says": says, "cmds": list(cmds),
                "ib": [[t["kind"], t["subject"], t["unit"], t["target"], t["words"], t["title"], t["due"], [list(h) for h in t["hist"]]]
                       for t in app.life.get("ibtasks", [])],
                "rules": app.cfg.get("zk_rules") or [], "gym": json.loads(json.dumps(app.life.get("gym"))),
                "tt": json.loads(json.dumps(app.tt)), "grades": json.loads(json.dumps(app.life.get("grades"))),
                "session": None if not app.session else [app.session["task"], app.session["paused"]],
                "bedtime": [app.cfg.get("bedtime"), app.cfg.get("sleep_guard")], "paused": app.life["screen"].get("paused_until", 0) > 0})
with open(os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "golden", "routes.json"), "w") as f:
    json.dump(out, f, ensure_ascii=False, indent=0)
for o in out[1:]:
    print(repr(o["q"]), "->", o["out"][1:], o["cmds"])
os._exit(0)
