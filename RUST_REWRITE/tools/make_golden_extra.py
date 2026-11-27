"""golden/extra.json: zenko.py's remaining chat routes, in answer_local's order after study_route: life_route (exams in
words, free time, study plan, hours for an exam, countdown, flashcards + the review quiz, facts, music, desktop
toggles, streak/badges, how are you, haiku, visitor), extra_route (lock-in, pomodoro, focus N, reminders, weekly
report, quiz, calc, define/translate with lookups off, quick notes, to-dos, omikuji, word of the day, sing, outfits,
coats, rename, treats and play, treasures, bond, sulk/sorry), app_route, deadline_route, battery and notes_route.
Offline, clock pinned (it moves 2.7 s a line, as long as each line's wait, so Tk's after() timers and the clock agree),
Python's random seeded per line (and random.Random() for the omikuji), lookups off.
Run: xvfb-run -a -s "-screen 0 1920x1080x24" env PYTHONDONTWRITEBYTECODE=1 /usr/bin/python3.12 tools/make_golden_extra.py"""
import sys
sys.dont_write_bytecode = True
import json, os, tempfile, types, random
os.environ["TZ"] = "UTC"
import time  # noqa: E402
time.tzset()
ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
os.environ["APPDATA"] = tempfile.mkdtemp()
os.environ["FOX_FOOD_BASE"] = "http://127.0.0.1:9"
os.environ["FOX_FAKE_BATTERY"] = "57,1"
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


class SeededRandom(random.Random):
    def __init__(self, x=None):
        super().__init__(777)


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


DEADLINES = [{"id": "bm1", "title": "Bio essay", "kind": "Assignment", "date": day(6)},
             {"id": "ch1", "title": "Chemistry test", "kind": "Quiz", "date": day(12), "time": "10:00"}]
NOTES = {"Biology notes": "ATP is the energy currency of the cell. Mitochondria make ATP by aerobic respiration.\n"
                          "Glycolysis happens in the cytoplasm."}
LIFE = {"cards": [{"id": "c1", "q": "Capital of France", "a": "Paris", "box": 1, "wrong": 2, "right": 1,
                   "added": day(-5), "due": day(-1)},
                  {"id": "c2", "q": "Symbol for gold", "a": "Au", "box": 0, "wrong": 0, "right": 0,
                   "added": day(-2), "due": day(3)}],
        "facts": {"favourite subject": "biology"}, "badges": {"first": day(-9)}}
os.makedirs(Z.DATA_DIR, exist_ok=True)
with open(os.path.join(Z.DATA_DIR, "deadlines.json"), "w") as f:
    json.dump(DEADLINES, f, indent=1)
os.makedirs(Z.RM_NOTES, exist_ok=True)
for k, v in NOTES.items():
    with open(os.path.join(Z.RM_NOTES, k + ".md"), "w", encoding="utf-8") as f:
        f.write(v)
with open(os.path.join(Z.DATA_DIR, "config.json"), "w") as f:
    json.dump({"web_enabled": False, "llm_enabled": False}, f)
for name in ("_life_slow", "study_missed", "deadline_reminders", "check_timetable", "update_tick", "journey_check",
             "z22_badge_check", "study_slow_tick", "books_slow_tick", "tick", "screen_tick"):
    setattr(Z.Zenko23, name, lambda self, *a, **k: None)      # the timers: these lines are about chat only
app = Z.Zenko23()
app.root.update()
for k in LIFE:
    app.life[k] = json.loads(json.dumps(LIFE[k]))
app.life_save()
Z.random.Random = SeededRandom

CONVO = [
    # extra_route
    "lock in 25", "lock in", "unlock", "unlock",
    "pomodoro", "pomodoro 50/10 3", "stop pomodoro", "pomo 30-5 x2", "stop the pomodoro",
    "focus 45 on essay writing", "study 20", "stop", "work for 2 h on lab report", "stop",
    "remind me to stretch in 20 min", "remind me in 2 hours to drink water", "remind me to call grandma at 7pm",
    "my reminders", "clear my reminders",
    "how was my week", "weekly report",
    "stop quiz", "quiz me on photosynthesis", "flashcards on the krebs cycle",
    "calc 15% of 240", "2^10", "what is 12 times 7", "calc (3+4)*2^3", "= 10/4", "calc 1/0", "calc sqrt 81 + 3",
    "calc banana", "3 x 4", "calc 2**300", "calc -2**2", "calc 7//2", "calc 7.5 % 2", "calc 20% off 50",
    "calc sin(30)", "calc pi*2", "calc round(2.5)", "calc 1e3+1", "calc 0.1+0.2", "what's 100 divided by 8",
    "calc 10 mod 3", "calc 2 squared", "calc ln(e)", "calc 1,000,000 * 3", "calc 2^-2", "calc 5 - -3",
    "calc abs(-4.5)", "calc floor(7.8)", "calc 10/3", "calc 123456789*987654321", "calc 2^100",
    "calc square root of 2", "calc 3 cubed", "calc log(1000)", "calc 0x1f + 1", "calc 1_000 + 1", "calc 2 ** 0.5",
    "calc (-8) ** (1/3)", "calc 4 % 0", "calc 1e308*10", "calc ceil(-0.5)", "calc exp(1)", "calc tan(45)",
    "calc 1/3*3", "calc .5+.25", "calc 7 % -3", "calc -7 // 2", "calc 2^3^2", "calc round(-2.5)", "calc 100!",
    "define serendipity", "what does ephemeral mean", "meaning of kitsune",
    "translate good luck on your exam to japanese", "translate hello", "translate hi to klingon",
    "note: buy more aburaage", "note to self, call grandma", "note pick up library books", "take a note - revise krebs",
    "my notes", "delete note 2", "delete note 9", "my notes", "clear my notes", "my notes",
    "todo finish essay", "add todo: read chapter 3", "task email teacher", "to-do: buy pens", "my todos", "done 1",
    "done email", "done xyzzy", "what's on my to do", "clear done todos", "todo list", "clear my todos", "todo list",
    "omikuji", "omikuji", "tie it", "my fortune",
    "word of the day", "another japanese word", "nihongo",
    "sing", "sing me a song",
    "outfits", "outfit crown", "wear a bow", "take off the crown", "outfit wings", "bib red", "bib purple",
    "take off bib", "outfit none",
    "coat golden", "fur pink", "coat rainbow", "be dark", "coat default",
    "your name is mochi", "who are you", "your name is zenko",
    "give you a treat", "here's some mochi", "feed you", "eat dango", "have an onigiri",
    "pet", "*boop*", "tickle you", "yarn", "high five", "gift", "dance", "brush", "fluff your tails", "good fox", "yawn",
    "hide and seek", "belly rub",
    "treasures", "our bond", "do you like me",
    "shut up", "pet", "sorry", "sorry",
    # life_route
    "my chemistry exam is in 2 days", "exam on friday at 9am", "i have a maths test next monday", "when is my exam?",
    "i'm free 4-7pm on weekdays", "i am free 3 to 6 tomorrow", "free 9am-12 on saturdays",
    "plan my study", "replan",
    "i need 4 hours for my chemistry exam", "give me 2 hours of study for bio essay", "i need 3 hours for unicorn",
    "countdown", "how many days until my exams",
    "add flashcard mitochondria = powerhouse of the cell", "flashcard capital of japan = tokyo", "new card H2O -> water",
    "my flashcards",
    "review flashcards", "paris", "hint", "skip", "stop quiz", "review my mistakes", "flashcards", "Paris", "tokio",
    "watr", "the powerhouse of the cell",
    "remember that i play piano", "remember my favourite colour is blue", "my birthday is 14 may",
    "my best friend is aiko", "what do you remember about me", "forget my best friend", "forget piano",
    "what do you remember about me",
    "study music", "play bamboo rain music", "background music off",
    "roaming off", "seasons on", "day night off", "stop roaming", "start walking around", "go for a walk",
    "my streak", "badges",
    "how are you", "are you hungry",
    "haiku", "write me a haiku",
    "invite a friend",
    # app_route, deadline_route, battery, notes_route
    "open spotify", "open timetable", "open the calendar", "open zenko", "open my outlook calendar", "open notes",
    "my deadlines", "what's due", "upcoming exams", "add deadline physics lab report on 20 dec 5pm",
    "add exam renal block 12/01", "add test french vocab", "add quiz",
    "battery", "how's my battery", "battery level",
    "remarkable notes", "summarize my notes on biology", "read my notes on biology", "ask my notes: what is atp",
    "quiz me on my biology notes", "summarise that file",
]
out = [{"now": NOW[0], "life": LIFE, "deadlines": DEADLINES, "notes": NOTES, "cfg": {"web_enabled": False, "llm_enabled": False}}]
caught, cmds = [], []
app.chat_line = lambda who, text: (caught.append(["line", who, text]), app.chat_log.append((who, text)))
app.ask_llm = lambda q: caught.append(["llm", q])
app.say = lambda text, secs=9, choices=None: caught.append(["say", text, [c[0] for c in (choices or [])]])
app.speak = lambda *a, **k: None
app.open_hub = lambda page=None, *a, **k: cmds.append("open_hub:%s" % page)
app.open_breathing = lambda *a, **k: cmds.append("open_breathing")
app.visitor_start = (lambda f: lambda *a, **k: (cmds.append("visitor_start"), f(*a, **k))[1])(app.visitor_start)
app.apply_look = lambda *a, **k: cmds.append("apply_look")
app.music_apply = lambda *a, **k: cmds.append("music_apply")
app.export_pdf = lambda *a, **k: cmds.append("export_pdf")
CFG_KEYS = ("outfit", "scarf", "coat", "fox_name", "bgm", "bgm_track", "roam", "seasons", "daynight")
for i, q in enumerate(CONVO):
    random.seed(5000 + i)
    NOW[0] += 2.7
    del caught[:]
    del cmds[:]
    app.chat_send(q)
    _sleep(2.7)
    for _ in range(3):
        app.root.update()
    fox = [c[2] for c in caught if c[0] == "line" and c[1] == "Fox"]
    got = [c for c in caught if c[0] != "say"]
    says = [c for c in caught if c[0] == "say" and not any(f.split("\n")[0] == c[1] for f in fox)]
    E, L = app.extra, app.life
    q_ = app.quiz
    out.append({"q": q, "now": NOW[0], "out": got, "says": says, "cmds": list(cmds),
                "extra": json.loads(json.dumps({k: E.get(k) for k in ("notes", "todos", "reminders", "omikuji", "bond",
                                                                       "treasures", "treats")})),
                "cfg": {k: app.cfg.get(k) for k in CFG_KEYS},
                "life": json.loads(json.dumps({k: L.get(k) for k in ("cards", "facts", "notes", "free", "free_dates",
                                                                    "free_set", "plan_hours", "plan_short")})),
                "badges": sorted(L.get("badges", {})), "ema": [[e.get("text"), e.get("at")] for e in L.get("ema", [])],
                "tt": json.loads(json.dumps(app.tt)), "deadlines": Z.load_deadlines(),
                "session": None if not app.session else [app.session["task"], app.session["planned"], app.session["paused"]],
                "pomo": json.loads(json.dumps(app.pomo)), "lockin": bool(app.lockin),
                "quiz": None if not q_ else json.loads(json.dumps({k: q_.get(k) for k in ("cards", "i", "score", "topic", "srs", "saved")})),
                "sulk": bool(app.sulk_until)})
with open(os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "golden", "extra.json"), "w") as f:
    json.dump(out, f, ensure_ascii=False, indent=0)
for o in out[1:]:
    print(repr(o["q"]), "->", [c[2][:120] if c[0] == "line" else c for c in o["out"][1:]], o["says"][:2], o["cmds"])
os._exit(0)
