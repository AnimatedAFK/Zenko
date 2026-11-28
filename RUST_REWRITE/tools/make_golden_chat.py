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
         "Hi Zenko, please stretch", "sniff around", "start 300", "time left", "stop", "HELP", "what do you know about me",
         # the ema rack (zenko18) and the timetable
         "remind me to call mum at 6pm", "remind me to drink water every day at 10", "remind me in 20 min to stretch",
         "remind me tomorrow 7am take vitamins", "don't let me forget to buy milk", "remind me", "my reminders",
         "move call mum to 7pm", "move milk to tomorrow", "rename milk to Buy oat milk", "mark call mum as done",
         "snooze stretch for 15 min", "remind me on friday to submit the essay", "remind me at 9 to read",
         "remind me after lunch to walk", "Remind me in half an hour to check the oven", "delete reminder 2",
         "clear fulfilled reminders", "plan study bio at 5pm for 1 hour", "i will revise chem at 7",
         "add 4pm study renal 45", "add 8:30 history 20 daily", "add tomorrow 6pm piano", "add math", "add 5pm",
         "timetable", "what's next", "next", "any reminders", "the oven reminder is done", "clear all reminders",
         "what are my reminders", "remind me in 5 min to stand up", "remind me to drink tea every day at 8am",
         "remind me to water plants"]
out = []
_ema_tick = Z.Zenko.ema_tick
Z.Zenko.ema_tick = lambda self: None          # the 5 s timer: replayed by hand at the end instead
app = Z.Zenko23()
app.cfg["ema_plan"] = False                   # (ema auto-planning has its own golden)
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
                "brk": None if not app.brk else {"planned": app.brk["planned"]},
                "ema": [[e.get("text"), e.get("at"), e.get("repeat"), e.get("done"), e.get("rang")] for e in app.ema_list()],
                "tt": [[i["start"], i["mins"], i["task"], i["daily"], i["date"]] for i in app.tt]})
# ema_tick a day later: what rings, what rolls on to tomorrow
tick = []
for k in range(3):
    NOW[0] += 3600 * 9
    del caught[:]
    _ema_tick(app)
    tick.append({"now": NOW[0], "out": list(caught),
                 "ema": [[e.get("text"), e.get("at"), e.get("repeat"), e.get("done"), e.get("rang")] for e in app.ema_list()]})
out.append({"tick": tick})
with open(os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "golden", "chat.json"), "w") as f:
    json.dump(out, f, ensure_ascii=False, indent=0)
for o in out:
    print(repr(o.get("q")), "->", o.get("out", o.get("tick")))
os._exit(0)
