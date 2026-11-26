"""Sound oracle: imports the ORIGINAL zenko.py (never modified) and records the sha256 of every .wav it synthesises
(`_wav_bytes(kind, vol)`), for every sound at every volume, plus her voice at a second pitch and the "a+b" mixes.
Run from the repo root:  python RUST_REWRITE/tools/make_golden_sounds.py   -> RUST_REWRITE/golden/sounds.json"""
import sys
sys.dont_write_bytecode = True
import hashlib, json, os, tempfile

ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
os.environ["APPDATA"] = tempfile.mkdtemp()
sys.path.insert(0, ROOT)
import zenko as Z  # noqa: E402

kinds = list(Z.SOUND_KINDS) + [k for k in Z.EXTRA_SOUNDS if k not in Z.SOUND_KINDS]
mixes = sorted({k + "+" + v for k, v in Z.VOICE_FOR.items() if k in kinds})
out = []
for pitch in (1.0, 1.15):
    Z.SOUND["pitch"] = pitch
    for kind in (kinds + mixes if pitch == 1.0 else [k for k in kinds if k.startswith("v_")] + ["pet+v_kyu"]):
        for vol in (1, 2, 3):
            b = Z._wav_bytes(kind, vol)
            out.append({"kind": kind, "vol": vol, "pitch": pitch, "len": len(b) if b else 0,
                        "sha": hashlib.sha256(b).hexdigest() if b else "", "file": ("%s_v%d_%s.wav" % (kind.replace("+", "_"), vol,
                                             "k1" + (("p%d" % int(pitch * 100)) if "v_" in kind else "")))})
        print(kind, pitch, flush=True)
p = os.path.join(ROOT, "RUST_REWRITE", "golden", "sounds.json")
with open(p, "w") as f:
    json.dump(out, f, indent=0)
print(len(out), "sounds ->", p)
