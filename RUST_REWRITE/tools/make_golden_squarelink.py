"""golden/square_link.json: zenko.py's SquareLink23 fed a stream of ntfy lines (messages, reports, junk, repeats,
looks, rude words and links) — the messages, reports, changed count and `since` after each line, what square_visible
shows with a block list, and the exact URL and body each post sends."""
import sys
sys.dont_write_bytecode = True
import json, os, tempfile
sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", ".."))
os.environ.setdefault("APPDATA", tempfile.mkdtemp())
import zenko as Z  # noqa: E402

Z.time.time = lambda: 1766000000.0
sq = Z.SquareLink23("https://ntfy.sh/", Z.SQUARE_TOPIC)
B = lambda d: json.dumps(d)
lines = [
    B({"id": "a1", "time": 1765990000, "event": "open"}),
    B({"id": "a2", "time": 1765990001, "event": "message", "message": B({"z": 1, "u": "u1", "n": "momiji", "t": "hello   there\nfriend"})}),
    B({"id": "a3", "time": 1765990002, "event": "message", "message": B({"z": 1, "u": "u2", "n": "x", "t": "visit www.example.com or mail me a@b.co"})}),
    B({"id": "a3", "time": 1765990003, "event": "message", "message": B({"z": 1, "u": "u2", "n": "dup", "t": "again"})}),
    B({"id": "a4", "time": 1765990004, "event": "message", "message": B({"z": 2, "u": "u3", "n": "old", "t": "wrong version"})}),
    B({"id": "a5", "event": "message", "message": B({"z": 1, "u": "u3", "n": "Yuki-chan!!", "t": "what the fuck, shitty day", "a": {"y": 13, "m": 1, "h": -2}})}),
    B({"id": "a6", "time": "1765990006", "event": "message", "message": B({"z": 1, "u": "u4", "n": "tanuki_7", "t": "   "})}),
    B({"id": "a7", "time": 1765990007, "event": "message", "message": B({"z": 1, "rep": "a2", "by": "u9"})}),
    B({"id": "a8", "time": 1765990008, "event": "message", "message": B({"z": 1, "rep": "a2", "by": "u8"})}),
    B({"id": "a9", "time": 1765990009, "event": "message", "message": B({"z": 1, "rep": "a2", "by": "u8"})}),
    "not json at all",
    B({"id": "b1", "time": 1765990010, "event": "message", "message": "plain text"}),
    B({"id": "b2", "time": 1765990011, "event": "message", "message": B({"z": 1, "u": "u5" * 30, "n": "Kitsune", "t": "call +1 (555) 123-4567 ok", "a": {"y": "2", "m": 0, "h": 3}})}),
    B({"id": "b3", "time": 1765990012, "event": "message", "message": B({"z": 1, "rep": "a3", "by": "u1"})}),
    B({"id": "b4", "time": 1765990013, "event": "message", "message": B({"z": 1, "u": "u6", "n": "Ünïcode", "t": "café ☕ add me on snap: foxy99", "a": "nope"})}),
    B({"id": "b5", "time": 1765990014, "event": "message", "message": B({"z": 1, "rep": "a2", "by": "u7"})}),
]
steps = []
for raw in lines:
    try:
        ev = json.loads(raw)
    except ValueError:
        ev = None
    if ev is not None and ev.get("event") == "message":
        sq_since = ev.get("id")
        sq.take(ev)
    steps.append({"raw": raw, "msgs": json.loads(json.dumps(sq.msgs)), "changed": sq.changed,
                  "reports": {k: sorted(v) for k, v in sq.reports.items()}})


class App:
    cfg = {"square_block": ["u3"]}
    _sq = sq


vis = Z.Zenko23.square_visible(App)
posts = []


class R:
    def __enter__(self):
        return self

    def __exit__(self, *a):
        return False

    def read(self):
        return b""


def fake(req, timeout=0):
    posts.append([req.full_url, req.data.decode()])
    return R()


Z.urllib.request.urlopen = fake
sq.post("me1", "Kitsune", "hi")
sq.look = {"y": 4, "m": True, "h": 0}
sq.post("me1", "Kitsune", "kon kon é☃")
sq.post("me1", "", "", rep="a3")
out = {"steps": steps, "visible": vis, "posts": posts}
with open(os.path.join(os.path.dirname(__file__), "..", "golden", "square_link.json"), "w") as f:
    json.dump(out, f, indent=0)
print(len(steps), len(vis), posts)
