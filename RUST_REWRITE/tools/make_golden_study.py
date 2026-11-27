"""golden/study.json: zenko.py's zenko8 study tracker (study_tick -> study_classify / study_commit / study_flush /
credit_minutes / study_tidy, the "studying Bio?" ask and its taps, study_slow with study_missed, the recap, the
weak-subject alert and the golden hour) and study_route (I'm studying X, logging hours, recap, reports, goals,
conjugate, notebook / test me), offline, with the clock (and time.monotonic) pinned, Python's random seeded per step,
and deadlines, a timetable and some study history set up first.
Steps: a chat line; "@fg title|proc|idle|site|n" = n study ticks (1 s apart) on that window; "@tap i" = press button i
of the last bubble; "@slow" = study_slow(); "@at HH:MM" / "@next HH:MM" = move the clock to that time today / tomorrow.
Run: xvfb-run -a -s "-screen 0 1920x1080x24" env PYTHONDONTWRITEBYTECODE=1 /usr/bin/python3.12 tools/make_golden_study.py"""
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


DEADLINES = [{"id": "bm1", "title": "Bio MYA", "kind": "Exam", "date": day(5)},
             {"id": "ch1", "title": "Chemistry test", "kind": "Quiz", "date": day(12), "time": "10:00"},
             {"id": "my1", "title": "MYA week", "kind": "Exam", "date": day(20)},
             {"id": "fr1", "title": "French oral", "kind": "Presentation", "date": day(70)},
             {"title": "Psych essay", "kind": "Assignment", "date": day(8)},
             {"id": "en1", "title": "English essay", "kind": "Assignment", "date": day(3), "done": True}]
TT = [{"start": "08:00", "mins": 50, "task": "Study: Bio MYA", "date": day(0), "plan": "bm1", "done": []},
      {"start": "06:00", "mins": 50, "task": "Study: Chemistry test", "date": day(0), "plan": "ch1", "done": [], "auto_s": 600},
      {"start": "09:00", "mins": 60, "task": "Homework time", "daily": True, "done": []},
      {"start": "13:00", "mins": 45, "task": "Lunch club", "date": day(1), "done": []}]
HOURS = {"10": [3000, 300], "11": [2800, 200], "16": [1300, 800], "20": [1250, 100]}
LIFE = {"slog": {day(-70): {"psych": 100}, day(-9): {"bio": 600}, day(-3): {"math": 5400}, day(-1): {"bio": 3600, "chem": 1200}},
        "dlog": {day(-8): {"YouTube": 4000}, day(-1): {"YouTube": 2400, "Reddit": 300}},
        "hourlog": {day(-2): HOURS, day(-1): {"10": [2000, 100], "16": [900, 900]}, day(-50): {"8": [5000, 0]}},
        "goals": {"math": 3}, "studied": {"bm1": 30},
        "win_subj": {"past paper # - sumatrapdf": "math"}}
os.makedirs(Z.DATA_DIR, exist_ok=True)
with open(os.path.join(Z.DATA_DIR, "deadlines.json"), "w") as f:
    json.dump(DEADLINES, f, indent=1)
with open(os.path.join(Z.DATA_DIR, "timetable.json"), "w") as f:
    json.dump(TT, f, indent=1)
for name in ("_life_slow", "deadline_reminders", "check_timetable", "update_tick", "journey_check", "z22_badge_check",
             "study_slow_tick", "screen_tick", "tick", "books_slow_tick"):
    setattr(Z.Zenko23, name, lambda self, *a, **k: None)      # the other timers: these steps call study_* themselves
app = Z.Zenko23()
app.root.update()
for k in LIFE:
    app.life[k] = json.loads(json.dumps(LIFE[k]))
app.life_save()
time.monotonic = lambda: NOW[0]
app.st["last_flush"] = NOW[0]

STEPS = ["@fg Cell respiration notes - Google Docs - Google Chrome|chrome.exe|0||70", "what am i studying",
         "@fg Cell respiration notes - Google Docs - Google Chrome|chrome.exe|30||20",
         "@fg Cell respiration notes - Google Docs - Google Chrome|chrome.exe|0||1",
         "@fg YouTube - Google Chrome|chrome.exe|0|YouTube|30",
         "@fg Cell respiration notes - Google Docs - Google Chrome|chrome.exe|400||5",
         "@fg Cell respiration notes - Google Docs - Google Chrome|chrome.exe|0||2040", "@slow",
         "@at 10:30", "@fg Untitled worksheet 3 - Word|winword.exe|0||125", "@fg Untitled worksheet 3 - Word|winword.exe|0||1",
         "@tap 1", "@fg Untitled worksheet 3 - Word|winword.exe|0||61", "what am i studying",
         "@fg Random project.docx - Word|winword.exe|0||125", "@tap 6", "@fg Random project.docx - Word|winword.exe|0||5",
         "@fg Integration past paper 12 - SumatraPDF|sumatrapdf.exe|0||65", "@fg Some page - Google Chrome|chrome.exe|0||10",
         "what am i studying", "i'm studying chem", "@fg Some page - Google Chrome|chrome.exe|0||75", "what am i studying",
         "stop studying", "@fg Some page - Google Chrome|chrome.exe|0||5", "what subject am i doing",
         "@fg Lesson notes - OneNote|onenote.exe|0||130", "what am i studying", "@tap 0",
         "i studied math for 2 hours", "log 90 min chem", "i was studying bio for the past 2 hours",
         "i revised psych for 45 minutes yesterday", "i did french for half an hour", "i spent an hour on english",
         "i'm going to study french for an hour", "let's do bio for half an hour", "@fg Some page - Google Chrome|chrome.exe|0||3",
         "i'm doing english", "switch to maths", "done studying", "i studied something for 2 hours",
         "how did i do today", "recap", "distraction report", "what distracts me", "best focus time",
         "when do i study best", "study report", "how much have i studied", "hours this week",
         "set my math goal to 4 hours", "i want to study 2 hours a week of french", "bio goal 3h", "chem goal is 2.5 hours",
         "conjugate manger", "conjugate se lever", "conjugate être", "conjugate xyzq", "conjugate appeler",
         "conjugate nettoyer", "conjugate finir", "conjugate vendre", "conjugate aller", "conjugate s'amuser",
         "conjugate préférer", "conjugate commencer", "conjugate the verb partir", "notebook", "open my notebook",
         "test me on photosynthesis", "flashcards from my notes",
         "@at 15:05", "@slow", "@tap 1", "@slow", "@at 21:05", "@slow", "@slow",
         "@next 10:04", "@slow", "@slow", "@at 16:00", "@slow", "recap", "@next 21:30", "@slow", "@next 08:00",
         "@fg Lesson notes - OneNote|onenote.exe|0||61", "@slow", "week report", "distraction report"]
caught, cmds, last = [], [], [None]
app.chat_line = lambda who, text: (caught.append(["line", who, text]), app.chat_log.append((who, text)))
app.ask_llm = lambda q: caught.append(["llm", q])


def _say(text, secs=9, choices=None):
    caught.append(["say", text, [c[0] for c in (choices or [])]])
    if choices:
        last[0] = choices


app.say = _say
app.speak = lambda *a, **k: None
app.open_hub = lambda page=None, *a, **k: cmds.append("open_hub:%s" % page)
app.nb_show = lambda sid, *a, **k: cmds.append("nb_show:%s" % sid)
app.nb_start_test = lambda sid, topic, *a, **k: cmds.append("nb_start_test:%s:%s" % (sid, topic))
app.nb_cards_from_file = lambda sid=None, *a, **k: cmds.append("nb_cards_from_file:%s" % (sid or ""))
app.nb_refresh = lambda *a, **k: None
app.start_session = lambda task, mins=25, *a, **k: cmds.append("start_session:%s:%s" % (task, mins))
out = [{"deadlines": DEADLINES, "tt": TT, "life": LIFE, "now": NOW[0]}]
for i, q in enumerate(STEPS):
    random.seed(4000 + i)
    del caught[:]
    del cmds[:]
    if q.startswith("@fg "):
        title, proc, idle, site, n = q[4:].split("|")
        for _ in range(int(n)):
            NOW[0] += 1.0
            app._last_fg = (title, proc, float(idle), site or None)
            app.study_tick()
    elif q.startswith("@tap "):
        ch, last[0] = last[0], None
        ch[int(q[5:])][1]()
    elif q == "@slow":
        NOW[0] += 30.0
        app.study_slow()
    elif q.startswith(("@at ", "@next ")):
        h, m = map(int, q.split()[1].split(":"))
        d = _dt.datetime.utcfromtimestamp(NOW[0]).date() + _dt.timedelta(days=1 if q.startswith("@next") else 0)
        NOW[0] = _dt.datetime(d.year, d.month, d.day, h, m, tzinfo=_dt.timezone.utc).timestamp()
    else:
        NOW[0] += 7.0
        app.chat_send(q)
    _sleep(0 if q.startswith(("@at", "@next")) else 2.7)   # after(150..2500) jobs (badges) belong to this step
    for _ in range(3):
        app.root.update()
    fox = [c[2] for c in caught if c[0] == "line" and c[1] == "Fox"]
    got = [c for c in caught if c[0] != "say"]
    says = [c for c in caught if c[0] == "say" and not any(f.split("\n")[0] == c[1] for f in fox)]
    L = app.life
    w = app.st.get("now")
    ov = app.st.get("override")
    out.append({"q": q, "now": NOW[0], "out": got, "says": says, "cmds": list(cmds),
                "life": json.loads(json.dumps({k: L.get(k) for k in ("slog", "dlog", "hourlog", "unsorted", "win_subj",
                                                                    "subj_words", "goals", "studied", "said")})),
                "tt": json.loads(json.dumps(app.tt)), "st_now": list(w) if w else None,
                "override": [ov[0], ov[1]] if ov else None, "credit": dict(app.st["credit"]),
                "deadline_ids": [d.get("id") for d in Z.load_deadlines()]})
with open(os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "golden", "study.json"), "w") as f:
    json.dump(out, f, ensure_ascii=False, indent=0)
for o in out[1:]:
    print(repr(o["q"]), "->", o["out"][1:] if o["out"] else "", o["says"], o["cmds"], o["st_now"])
os._exit(0)
