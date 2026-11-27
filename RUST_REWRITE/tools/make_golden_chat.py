"""golden/chat.json: what zenko.py's chat answers, offline (FoxBuddy.chat_send -> answer_local, expand_shorthand,
fox_voice, learning), for a scripted conversation. Python's random is seeded before every message (the Rust port uses
the same Mersenne Twister), the clock is pinned and the data folder is empty. Lines that would go to the smart brain are
recorded as ["llm", question].
Run: xvfb-run -a -s "-screen 0 1920x1080x24" /usr/bin/python3.12 tools/make_golden_chat.py"""
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

NOW = [1765407600.0]


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
_sleep = time.sleep
time.time = lambda: NOW[0]

CONVO = ["help", "?", "commands", "hlep", "Help me", "teach me hi => hello there", "hi", "HI!", "forget hi", "forget hi",
         "when i say sched i mean timetable", "my shorthand", "what shorthand", "forget my shorthand", "my shorthand",
         "what did you look up", "what have you learned", "home city London", "unit f", "temperature unit c",
         "time left", "start reading 25", "time left", "how long", "pause", "pause", "resume", "start maths", "stop",
         "stop", "stats", "how am i doing", "motivate me", "inspire me", "tips", "focus tip", "tell me a joke", "funny",
         "take a break", "break time", "end break", "back to work", "i'm tired", "rest", "who are you", "what can you do",
         "tell me about the legend", "zenko?", "hello", "hey", "good morning", "yo", "thanks", "ty", "thank you so much",
         "check in", "check-in", "daily checkin", "mood journal", "how am i feeling today", "strat studying 30",
         "start 10", "stop", "start", "stop", "Start Biology revision 45 min", "time left", "end", "asdfgh qwerty",
         "wat is the tim left", "teach you bye = see you later~", "bye", "byee", "forget everything", "bye",
         "remember gm -> good morning sunshine", "gm", "Gm!!", "start  history essay, 50", "pause", "time left",
         "resume", "quit session", "motivate", "i give up", "procrastinating again", "laugh", "need a break",
         "break", "stop break", "blorp me", "motivate me", "blorp me", "what's my shorthand",
         "teach goodnight => sleep well, kon~", "good night", "goodnight!", "wear a yukata", "wear nothing",
         "put on your kimono", "breathe", "i'm so stressed", "forget my favourite color", "stretch",
         "Hi Zenko, please stretch", "sniff around", "start 300", "time left", "stop", "HELP", "what do you know about me"]
out = []
app = Z.Zenko23()
app.root.update()
caught = []
app.chat_line = lambda who, text: (caught.append(["line", who, text]), app.chat_log.append((who, text)))
app.ask_llm = lambda q: caught.append(["llm", q])
app.say = lambda *a, **k: None
app.speak = lambda *a, **k: None
app.open_checkin = lambda force=False: caught.append(["checkin", force])
app.open_breathing = lambda: caught.append(["breathing"])
app.breath_start = lambda k="box": caught.append(["breath", k])
for i, q in enumerate(CONVO):
    random.seed(1000 + i)
    NOW[0] += 7.0
    del caught[:]
    pre = app.session["elapsed"] if app.session else None
    app.chat_send(q)
    _sleep(0.25)                # her after(200) actions (check-in, breathing) belong to this message
    for _ in range(3):
        app.root.update()
    s = app.session
    out.append({"q": q, "now": NOW[0], "pre": pre, "out": list(caught),
                "session": None if not s else {"task": s["task"], "planned": s["planned"], "paused": s["paused"]},
                "brk": None if not app.brk else {"planned": app.brk["planned"]}})
with open(os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "golden", "chat.json"), "w") as f:
    json.dump(out, f, ensure_ascii=False, indent=0)
for o in out:
    print(repr(o["q"]), "->", o["out"])
os._exit(0)
