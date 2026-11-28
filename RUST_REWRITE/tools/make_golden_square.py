"""crates/zenko-desktop/data/square.json (the festival tables, crowd colours, stall list, word filters) and
golden/square.json: zenko.py's festival street paintings (z23_street) for every festival in day, dusk and night at a
spread of sizes (one FNV-1a 64 hash per row, and the layout), people (z23_look + z23_person_rows), the festival
calendar, and the Town square's filters (square_clean, square_name_ok, square_tag)."""
import sys
sys.dont_write_bytecode = True
import datetime, json, os, random, tempfile
sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", ".."))
os.environ.setdefault("APPDATA", tempfile.mkdtemp())
import zenko as Z  # noqa: E402

L = lambda v: json.loads(json.dumps(v))
data = {"Z23_FEST": L(Z.Z23_FEST), "Z23_FEST_ORDER": list(Z.Z23_FEST), "Z23_STALLS": L(Z.Z23_STALLS),
        "Z23_FORTUNES": L(Z.Z23_FORTUNES), "DIWALI": {str(k): v for k, v in Z._Z23_DIWALI.items()},
        "HOLI": {str(k): v for k, v in Z._Z23_HOLI.items()}, "TSUKIMI": {str(k): v for k, v in Z._Z23_TSUKIMI.items()},
        "HAIR": Z._Z23_HAIR, "SKIN": Z._Z23_SKIN, "YUKATA": L(Z._Z23_YUKATA), "OBI": Z._Z23_OBI,
        "SQ_BAD": Z._SQ_BAD, "SQ_STEM": Z._SQ_STEM, "SQUARE": [Z.SQUARE_SERVER, Z.SQUARE_TOPIC, Z.SQUARE_SLOW, Z.SQUARE_MAX,
                                                              Z.SQUARE_HIDE_AT]}
with open(os.path.join(os.path.dirname(__file__), "..", "crates", "zenko-desktop", "data", "square.json"), "w",
          encoding="utf-8") as f:
    json.dump(data, f, ensure_ascii=False, separators=(",", ":"))


def fnv(b):
    h = 0xcbf29ce484222325
    for x in b:
        h = ((h ^ x) * 0x100000001b3) & 0xffffffffffffffff
    return "%016x" % h


modes = ["night", "dusk", "day"]
cases = []
for i, ev in enumerate(Z.Z23_FEST):
    cases.append((360 + (i % 4) * 57, 141 + (i % 3) * 20, ev, modes[i % 3], 739000 + i))
cases += [(500, 171, "summer", "night", 739580), (300, 111, "christmas", "day", 739581), (620, 181, "tanabata", "night", 2),
          (430, 151, "halloween", "dusk", 5), (430, 151, "diwali", "night", 6), (360, 141, "kodomo", "day", 7)]
streets = []
for aw, ah, ev, mode, seed in cases:
    R, Lay = Z.z23_street(aw, ah, ev, mode, seed=seed)
    row = aw * 3
    streets.append({"args": [aw, ah, ev, mode, seed], "rows": [fnv(R.b[y * row:(y + 1) * row]) for y in range(ah)],
                    "lay": L(Lay)})
people = []
for k in range(60):
    ev = list(Z.Z23_FEST)[k % len(Z.Z23_FEST)]
    seed = k * 37 + 1
    look = Z.z23_look(random.Random(seed), ev, fox=(k % 9 == 0))
    people.append({"seed": seed, "ev": ev, "fox": k % 9 == 0, "look": L(look),
                   "rows": [Z.z23_person_rows(look, 0), Z.z23_person_rows(look, 1)], "pal": Z.z23_person_pal(look)})
uidlook = []
for uid in ["0123456789abcdef", "ffee00112233aabb", "deadbeefcafef00d"]:
    uidlook.append({"uid": uid, "ev": "winter", "look": L(Z.z23_look(random.Random(uid), "winter", fox=True))})
days = []
d0 = datetime.date(2025, 12, 1)
for n in range(0, 400, 3):
    d = d0 + datetime.timedelta(days=n)
    nx = Z.z23_fest_next(d)
    days.append([d.isoformat(), Z.z23_fest_today(d), [nx[0], nx[1].isoformat()] if nx else None])
texts = ["hello there", "  lots   of   space  ", "visit https://example.com now", "mail me a@b.co please", "call +1 (555) 123-4567",
         "my snap: foxy99", "what the fuck", "shitty day", "FUCKING hell", "kill yourself", "a" * 300, "www.zenko.app rocks",
         "insta @me", "discord: me#1", "assassin class", "Scunthorpe", "pornhub", "nice.com", "ok 12345678 ok", "naïve café",
         "", "dick move", "retarded", "rapeseed oil"]
clean = [[t, Z.square_clean(t)] for t in texts]
names = [[n, Z.square_name_ok(n)] for n in ["fox", "ab", "Anoop_J", "x" * 30, "fuckboy", "Zen ko!", "  a.b-c  ", "名前", "kitsune99"]]
tags = [[u, Z.square_tag(u)] for u in ["", "0123456789abcdef", "abc"]]
gold = {"streets": streets, "people": people, "uidlook": uidlook, "days": days, "clean": clean, "names": names, "tags": tags}
with open(os.path.join(os.path.dirname(__file__), "..", "golden", "square.json"), "w") as f:
    json.dump(gold, f, ensure_ascii=False, indent=0)
print("ok", len(streets), len(people))
