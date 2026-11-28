"""golden/plan.json: zenko.py's study planner and zenko23's emas that plan themselves (replan, plan_study_slots,
z23_plan_emas, z23_announce, z23_noplan, the Zenko23 chat route), offline, with the clock pinned, Python's random seeded
per step and a deadlines.json + free time set up first. Each step records the chat lines, what she says in her bubble
(with buttons), the whole timetable and life's plan_short / ema_short.
Run: xvfb-run -a -s "-screen 0 1920x1080x24" env PYTHONDONTWRITEBYTECODE=1 /usr/bin/python3.12 tools/make_golden_plan.py"""
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
_sleep = time.sleep
time.time = lambda: NOW[0]

Z.Zenko.ema_tick = lambda self: None
app = Z.Zenko23()
app._life_slow = lambda: None                 # the 30 s tick's plan_maybe: steps call it themselves
app.study_missed = lambda: None               # (missed-session replanning and the other alerts have their own goldens)
app.deadline_reminders = lambda: None
app.check_timetable = lambda: None
app.root.update()


def day(n):
    return (_dt.date(2025, 12, 11) + _dt.timedelta(days=n)).isoformat()


DEADLINES = [{"id": "aa01", "title": "Chemistry exam", "kind": "Exam", "date": day(3), "time": "09:00"},
             {"title": "Bio essay", "kind": "Assignment", "date": day(5)},
             {"id": "d0", "title": "Far exam", "kind": "Exam", "date": day(60)},
             {"id": "x1", "title": "Done quiz", "kind": "Quiz", "date": day(2), "done": True},
             {"id": "q1", "title": "Maths quiz", "kind": "Quiz", "date": day(1), "time": "14:30"},
             {"id": "c1", "title": "Club", "kind": "Event", "date": day(2)}]
LIFE = {"free_dates": {day(1): [15.5, 16, 16.5, 17, 19]}, "plan_hours": {"q1": 1.5}, "studied": {"aa01": 60}}
with open(os.path.join(Z.DATA_DIR, "deadlines.json"), "w") as f:
    json.dump(DEADLINES, f, indent=1)
app.life.update(json.loads(json.dumps(LIFE)))
app.life_save()

STEPS = ["plan my emas", "remind me to finish bio IA by saturday", "remind me to revise chem tomorrow",
         "remind me to call mum at 6pm", "remind me to study french at 5pm", "remind me to do math homework",
         "remind me to read chapter 4 for 2 hours", "plan my emas", "bio IA takes 3 hours", "don't plan revise chem",
         "when should i do my homework", "math homework takes half an hour", "town square", "festival",
         "mark math homework as done", "chapter 4 takes about 45 min", "@day", "plan my emas", "please plan my tasks",
         "remind me to write the TOK essay", "@day", "@day", "plan my emas"]
caught = []
app.chat_line = lambda who, text: (caught.append(["line", who, text]), app.chat_log.append((who, text)))
app.ask_llm = lambda q: caught.append(["llm", q])
app.say = lambda text, secs=9, choices=None: caught.append(["say", text, [c[0] for c in (choices or [])]])
app.speak = lambda *a, **k: None
app.open_hub = lambda page=None, *a, **k: caught.append(["hub", page])
out = [{"deadlines": DEADLINES, "life": LIFE, "now": NOW[0]}]
for i, q in enumerate(STEPS):
    random.seed(2000 + i)
    NOW[0] += 600.0
    del caught[:]
    if q == "@day":
        NOW[0] += 86400 - 600
        app.plan_maybe()
    else:
        app.chat_send(q)
    _sleep(1.2)                 # after(600) plan_maybe and after(900) z23_announce belong to this step
    for _ in range(3):
        app.root.update()
    fox = [c[2] for c in caught if c[0] == "line" and c[1] == "Fox"]
    got = [c for c in caught if not (c[0] == "say" and (any(f.split("\n")[0] == c[1] for f in fox)
                                                          or c[1].startswith("Heads up:")))]
    out.append({"q": q, "now": NOW[0], "out": got, "tt": json.loads(json.dumps(app.tt)),
                "plan_short": app.life.get("plan_short"), "ema_short": app.life.get("ema_short"),
                "deadline_ids": [d.get("id") for d in Z.load_deadlines()]})
with open(os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "golden", "plan.json"), "w") as f:
    json.dump(out, f, ensure_ascii=False, indent=0)
for o in out[1:]:
    print(repr(o["q"]), "->", o["out"], len(o["tt"]), o["ema_short"])
os._exit(0)
