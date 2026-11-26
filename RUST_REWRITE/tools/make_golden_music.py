"""golden/music.json: sha256 of zenko.py's three background-music loops (music_file) and of the volume-3 copy
(music_file_vol), made by zenko.py's own code. tests/music.rs checks the Rust port makes the same bytes."""
import sys
sys.dont_write_bytecode = True
import hashlib, json, os, tempfile, time
ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
os.environ["APPDATA"] = tempfile.mkdtemp()
sys.path.insert(0, ROOT)
import zenko as Z  # noqa: E402

out = {"python": sys.version.split()[0]}
for tr in ("garden", "bamboo", "temple"):
    t0 = time.time()
    p = Z.music_file(tr)
    b = open(p, "rb").read()
    q = open(Z.music_file_vol(tr, 3), "rb").read()
    out[tr] = {"sha256": hashlib.sha256(b).hexdigest(), "bytes": len(b), "head": list(b[44:84]),
               "vol3_sha256": hashlib.sha256(q).hexdigest()}
    print(tr, round(time.time() - t0, 1), "s", flush=True)
dst = os.path.join(os.path.dirname(__file__), "..", "golden", "music.json")
with open(dst, "w") as f:
    json.dump(out, f, indent=1)
