"""crates/zenko-desktop/data/routes.json: the tables zenko.py's later chat routes read (Bible books and aliases, close or
keep rule sites, screen-time app names and browsers, IB task kinds, fish count), taken from the untouched zenko.py."""
import sys
sys.dont_write_bytecode = True
import json, os, tempfile
ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
os.environ["APPDATA"] = tempfile.mkdtemp()
sys.path.insert(0, ROOT)
import zenko as Z  # noqa: E402

data = {
    "BIBLE_BOOKS": [list(b[:4]) for b in Z.BIBLE_BOOKS],
    "BIBLE_ALIAS": Z._BIBLE_ALIAS,
    "RULE_SITES": [[n, list(a)] for n, a in Z.RULE_SITES],
    "SCREEN_APP_NAMES": Z.SCREEN_APP_NAMES,
    "BROWSERS": list(Z.BROWSERS),
    "IB_TASK_KINDS": [list(k[:4]) for k in Z.IB_TASK_KINDS],
    "IB_IA_DEFAULTS": {k: list(v) for k, v in Z.IB_IA_DEFAULTS.items()},
    "FISH_N": len(Z.FISH),
    "UPDATE_SOURCE_DEFAULT": Z.UPDATE_SOURCE_DEFAULT,
    "EXAM_DATE_DEFAULT": Z.EXAM_DATE_DEFAULT,
    "FOODS": [[list(e[0]), e[1], e[2], e[3], e[4]] for e in Z.FOODS],
    "GYM_ALL": [[d["kanji"], d["name"]] for d in Z.GYM_ALL],
}
out = os.path.join(os.path.dirname(__file__), "..", "crates", "zenko-desktop", "data", "routes.json")
with open(out, "w", encoding="utf-8") as f:
    json.dump(data, f, ensure_ascii=False, separators=(",", ":"))
print("wrote", out, {k: (len(v) if hasattr(v, "__len__") else v) for k, v in data.items()})
os._exit(0)
