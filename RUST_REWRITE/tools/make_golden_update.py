"""golden/update.json: zenko.py's write_lnk bytes, version_tuple / version_newer, update_check on made-up GitHub and
latest.json answers (FROZEN on, as Zenko.exe), update_file_ok's verdicts and zk_log_error's line."""
import sys
sys.dont_write_bytecode = True
import json, os, tempfile
sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", ".."))
os.environ.setdefault("APPDATA", tempfile.mkdtemp())
import zenko as Z  # noqa: E402

HERE = os.path.dirname(__file__)
g = {}
d = tempfile.mkdtemp()
import ntpath
_dn = os.path.dirname
os.path.dirname = ntpath.dirname   # Zenko runs on Windows: "C:\\Z\\a.exe" -> "C:\\Z"
lnks = []
for args in [("C:\\Users\\kid\\AppData\\Local\\Programs\\Zenko\\Zenko.exe", "", "C:\\Users\\kid\\AppData\\Local\\Programs\\Zenko",
              "C:\\Users\\kid\\AppData\\Local\\Programs\\Zenko\\Zenko.exe", "Zenko, your fox focus buddy"),
             ("C:\\Z\\ZenkoRust.exe", "--updated", "", "", "")]:
    p = os.path.join(d, "a.lnk")
    Z.write_lnk(p, *args)
    lnks.append({"args": list(args), "bytes": list(open(p, "rb").read())})
g["lnk"] = lnks
os.path.dirname = _dn
V = ["v2026.11.22-zenko15.1", "v2026.12.01-zenko16", "2026.11.22", "", "abc", "v2026.11.22-zenko15", "v10", "v9.9.9"]
g["newer"] = [[a, b, Z.version_newer(a, b)] for a in V for b in V]
g["tuple"] = [[v, list(Z.version_tuple(v))] for v in V]
# update_check with a pretend network
Z.FROZEN = True
answers = {}
def fake_api(url, timeout=15):
    return answers[url]
def fake_json(url, timeout=15):
    return answers[url]
def fake_get(url, timeout=15):
    return answers[url].encode()
Z._github_api, Z.http_json, Z.http_get = fake_api, fake_json, fake_get
API = "https://api.github.com/repos/AnimatedAFK/Zenko/releases/latest"
cases = []
def run(name, source, ans, prefer=None):
    answers.clear()
    answers.update(ans)
    try:
        r = Z.update_check(source, prefer=prefer)
        cases.append({"name": name, "source": source, "prefer": prefer, "ans": ans, "out": r})
    except Exception as e:
        cases.append({"name": name, "source": source, "prefer": prefer, "ans": ans, "err": str(e)})
rel = {"tag_name": "v2026.12.01-zenko16", "body": "x" * 500, "assets": [
    {"name": "Zenko.exe", "browser_download_url": "https://dl/Zenko.exe", "size": 9000000, "digest": "sha256:ABCDEF"},
    {"name": "zenko.py", "browser_download_url": "https://dl/zenko.py", "size": 100},
    {"name": "zenko.py.sha256", "browser_download_url": "https://dl/zenko.py.sha256"},
    {"name": "ZenkoRust.exe", "browser_download_url": "https://dl/ZenkoRust.exe", "size": 7000000}]}
run("gh_py", "AnimatedAFK/Zenko", {API: rel, "https://dl/zenko.py.sha256": "FFEE  zenko.py\n"})
run("gh_exe", "https://github.com/AnimatedAFK/Zenko", {API: rel}, prefer="exe")
run("gh_none", "AnimatedAFK/Zenko", {API: {"tag_name": "v1", "assets": []}})
run("gh_bad", "not a repo!", {})
run("json", "https://site/latest.json", {"https://site/latest.json": {"version": "v2026.12.02", "exe": "https://site/Zenko.exe?x=1",
                                                                         "exe_sha256": "AA", "notes": "n"}})
run("empty", "", {})
g["check"] = cases
# update_file_ok
f1 = os.path.join(d, "small.exe"); open(f1, "wb").write(b"MZ" + b"\0" * 100)
f2 = os.path.join(d, "big.exe"); open(f2, "wb").write(b"MZ" + b"\0" * (2 * 1024 * 1024))
f3 = os.path.join(d, "no.exe"); open(f3, "wb").write(b"PK" + b"\0" * (2 * 1024 * 1024))
g["file_ok"] = [[os.path.getsize(p), open(p, "rb").read(2).decode("latin-1"), list(Z.update_file_ok(p, "exe"))] for p in (f1, f2, f3)]
with open(os.path.join(HERE, "..", "golden", "update.json"), "w", encoding="utf-8") as f:
    json.dump(g, f, ensure_ascii=False, indent=0)
print("ok", len(cases), [c.get("out") or c.get("err") for c in cases])
