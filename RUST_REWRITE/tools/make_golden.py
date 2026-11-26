"""Golden-frame oracle: imports the ORIGINAL zenko.py (never modified) and dumps the exact sprite grids it draws,
so the Rust renderer can be checked cell-for-cell.  Run from the repo root:

    python RUST_REWRITE/tools/make_golden.py

Writes RUST_REWRITE/golden/sprites.jsonl.gz (one JSON object per case: the call and the rows it returned)."""
import sys
sys.dont_write_bytecode = True
import gzip, json, os, sys, tempfile, itertools, hashlib

ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
os.environ["APPDATA"] = tempfile.mkdtemp()            # keep the art cache / data dir away from the real one
sys.path.insert(0, ROOT)
import zenko as Z                                      # noqa: E402


def raw(name):
    f = getattr(Z, name)
    return getattr(f, "_zk_raw", f)


def clear_caches():
    Z._BODY.clear()
    Z._HEAD.clear()


CASES = []


def case(fn, lk=None, **args):
    CASES.append({"fn": fn, "look": lk or {"ears": "pointy", "blush": "soft"}, "args": args})


LOOKS = [{"ears": e, "blush": b} for e in ("pointy", "snowy", "floppy") for b in ("soft", "rosy", "none")]
ACCS = [[], ["glasses"], ["headphones"], ["scarf"], ["flower"], ["bow"], ["crown"], ["headband"], ["mask"],
        ["towel"], ["sweat"], ["glasses", "headphones", "scarf"], ["flower", "bow", "headband", "towel", "sweat"]] + \
       [[r] for r in Z.ROBES] + [["uniform", "glasses", "headband"], ["yukata", "flower", "scarf"]]
LOOKV = [[0, 0, 0, 0], [1, 0, 1, 0], [-1, 1, -1, 1], [1, -1, 0, 1]]

for n in range(1, 10):
    for fr in range(8):
        case("build_tails", n=n, frame=fr, frames=8, shadow=bool(fr % 2))
for st in Z.STATES:
    for fr in (0, 1):
        case("build_frame", state=st, frame=fr, look=None, acc=[], hx=0, ear=0)
for i, (st, acc) in enumerate(itertools.product(["idle", "focus", "happy", "sleep", "lick", "highfive", "walk0", "walk3", "alert"], ACCS)):
    case("build_frame", state=st, frame=i % 2, look=LOOKV[i % 4], acc=acc, hx=(i % 3) - 1, ear=(i // 3) % 2)
for i, lk in enumerate(LOOKS):
    for st in ("idle", "happy", "blink", "sad", "walk1"):
        case("build_frame", lk=lk, look=None, state=st, frame=i % 2, acc=["glasses"] if i % 2 else [], hx=i % 2, ear=(i + 1) % 2)
for fr in range(8):
    for n in (1, 3, 9):
        case("build_walk4", frame=fr, n=n, frames=8, blink=fr == 5, left=n == 9)
        case("build_walk34", frame=fr, n=n, frames=8, blink=fr == 5, left=n == 9, look="happy" if fr % 3 == 0 else "open")
POSES = ["stand", "stretch_front", "stretch_back", "playbow", "lie", "doze", "crouch", "rear", "leap", "dive", "land",
         "sniff", "shake", "walk", "bogus"]
for fr in range(8):
    case("rig_rows", state="qwalkR%d" % fr, n=3, acc=[])
    case("rig_rows", state="qwalkL%d" % fr, n=9, acc=["kimono_spring"])
for p in POSES:
    for fr in range(8):
        case("rig_rows", state="rig%s:%s:%d" % ("L" if fr % 2 else "R", p, fr), n=1 + (fr * 2) % 9,
             acc=[[], ["uniform"], ["yukata", "glasses"], ["kimono_autumn", "crown"], ["flower", "headband"],
                  ["kimono_summer"], ["kimono_winter", "mask"], ["bow", "towel"]][fr])
for lk in LOOKS[::4]:
    case("rig_rows", lk=lk, state="rigR:stand:0", n=3, acc=[])
case("rig_rows", state="nonsense", n=3, acc=[])
for fr in (0, 1):
    for n in (1, 3, 4, 9):
        for acc in ([], ["headphones"]):
            case("build_curl", frame=fr, acc=acc, n=n)
for fr in range(4):
    case("foxfire_rows", frame=fr)
for kind in ("read", "lift", "zen"):
    for fr in range(8):
        case("z19_sit_extra", kind=kind, frame=fr, state="focus" if kind == "read" else "idle", acc=[])


def run(c):
    Z.LOOK.clear()
    Z.LOOK.update(c["look"])
    clear_caches()
    a = dict(c["args"])
    f = c["fn"]
    if f == "build_tails":
        return raw(f)(a["n"], a["frame"], a["frames"], a["shadow"])
    if f == "build_frame":
        look = tuple(a["look"]) if a["look"] else None
        return raw(f)(a["state"], "up", a["frame"], look, tuple(a["acc"]), a["hx"], a["ear"])
    if f in ("build_walk4", "build_walk34"):
        extra = {"look": a["look"]} if f == "build_walk34" else {}
        return raw(f)(a["frame"], a["n"], a["frames"], a["blink"], a["left"], **extra)
    if f == "rig_rows":
        return raw(f)(a["state"], a["n"], tuple(a["acc"]))
    if f == "build_curl":
        return raw(f)(a["frame"], tuple(a["acc"]), a["n"])
    if f == "foxfire_rows":
        return raw(f)(a["frame"])
    if f == "z19_sit_extra":
        base = raw("build_frame")(a["state"], "up", a["frame"] % 2, None, tuple(a["acc"]), 0)
        return raw(f)(base, a["kind"], a["frame"])
    raise ValueError(f)


out = os.path.join(ROOT, "RUST_REWRITE", "golden", "sprites.jsonl.gz")
with gzip.open(out, "wt", encoding="utf-8", compresslevel=9) as fh:
    for c in CASES:
        rows = list(run(c))
        c["sha"] = hashlib.sha256("\n".join(rows).encode()).hexdigest()[:16]
        c["rows"] = rows
        fh.write(json.dumps(c, separators=(",", ":")) + "\n")
print(len(CASES), "cases ->", out)


# ---- pixels: palettes (make_pal) and the HD upscaler, checked on the final RGBA the window shows ----------------
import struct, zlib  # noqa: E402


def png_rgba(data):
    w, h = struct.unpack(">II", data[16:24])
    pos, idat = 8, b""
    while pos < len(data):
        n = struct.unpack(">I", data[pos:pos + 4])[0]
        t = data[pos + 4:pos + 8]
        if t == b"IDAT":
            idat += data[pos + 8:pos + 8 + n]
        pos += 12 + n
    raw = zlib.decompress(idat)
    stride = w * 4 + 1
    return w, h, b"".join(raw[y * stride + 1:(y + 1) * stride] for y in range(h))


class FakeFox:
    def __init__(self, cfg, robe=None):
        self.cfg = cfg
        self._robe_cur = robe


def make_pal(cfg, robe=None):
    me = FakeFox(cfg, robe)
    base = Z.ZenkoExtras.make_pal(me)                  # theme + coat + eyes + foxfire + bib
    if robe in Z.ROBES:                                 # Zenko19.make_pal on top
        base.update((k, v) for k, v in Z.ROBES[robe].items() if len(k) == 1)
    return base


PAL_CFGS = [({"theme": t}, None) for t in list(Z.FUR) + ["nope"]] + [
    ({"theme": "classic", "coat": "white"}, None), ({"theme": "snow", "coat": "black", "eye_color": "#ff4d6d"}, None),
    ({"theme": "mint", "coat": "golden", "foxfire_color": "#8a5cff", "scarf": "#3b6ea8"}, "yukata"),
    ({"theme": "sakura", "eye_color": "#3fd68f", "foxfire_color": "#ffb81c"}, "uniform"),
    ({"theme": "midnight", "coat": "sakura"}, "hanbok"), ({"theme": "ember", "coat": "orange", "scarf": "#ffffff"}, None)]
pix = []
for cfg, robe in PAL_CFGS:
    pal = make_pal(cfg, robe)
    pix.append({"kind": "pal", "cfg": cfg, "robe": robe or "", "pal": {k: v for k, v in pal.items() if v}})
Z.LOOK.clear()
Z.LOOK.update({"ears": "pointy", "blush": "soft"})
clear_caches()
spr = [("tails+frame", [raw("build_tails")(3, 2, 8, False), raw("build_frame")("happy", "up", 1, None, ("glasses",), 0, 0)]),
       ("rig", [raw("rig_rows")("rigR:playbow:1", 5, ("kimono_spring",))]),
       ("walk", [raw("rig_rows")("qwalkL3", 3, ())]),
       ("curl", [raw("build_curl")(1, (), 4)]),
       ("fire", [raw("foxfire_rows")(1)])]
for i, (cfg, robe) in enumerate(PAL_CFGS[:6]):
    pal = make_pal(cfg, robe)
    for name, layers in spr:
        for zoom in (1, 2, 3, 4, 5, 6, 7):
            if (i + zoom) % 3 and zoom not in (3, 5):
                continue
            if zoom >= 2:
                png = raw("sprite_png_hd")(layers, pal, zoom)
            else:
                png = raw("sprite_png")(layers, pal)
            w, h, rgba = png_rgba(png)
            pix.append({"kind": "img", "cfg": cfg, "robe": robe or "", "sprite": name, "zoom": zoom, "w": w, "h": h,
                        "sha": hashlib.sha256(rgba).hexdigest()})
out2 = os.path.join(ROOT, "RUST_REWRITE", "golden", "pixels.jsonl.gz")
with gzip.open(out2, "wt", encoding="utf-8", compresslevel=9) as fh:
    for c in pix:
        fh.write(json.dumps(c, separators=(",", ":")) + "\n")
print(len(pix), "pixel cases ->", out2)
