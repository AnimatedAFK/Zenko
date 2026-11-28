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
    "SCREEN_SITES": [list(x) for x in Z.SCREEN_SITES],
    "SCREEN_MEDIA": sorted(Z.SCREEN_MEDIA),
    "SCREEN_EXCLUDE_DEFAULT": Z.SCREEN_EXCLUDE_DEFAULT,
    "SCREEN_PRIVATE": Z._SCREEN_PRIVATE.pattern,
    "SCREEN_BROWSER_TAIL": Z._SCREEN_BROWSER_TAIL.pattern,
    "SCREEN_PROFILE": Z._SCREEN_PROFILE.pattern,
    "IB_STAGES": Z.IB_STAGES,
    "IB_TASK_KINDS": [list(k[:4]) for k in Z.IB_TASK_KINDS],
    "IB_IA_DEFAULTS": {k: list(v) for k, v in Z.IB_IA_DEFAULTS.items()},
    "FISH_N": len(Z.FISH),
    "UPDATE_SOURCE_DEFAULT": Z.UPDATE_SOURCE_DEFAULT,
    "EXAM_DATE_DEFAULT": Z.EXAM_DATE_DEFAULT,
    "FOODS": [[list(e[0]), e[1], e[2], e[3], e[4]] for e in Z.FOODS],
    "GYM_ALL": [[d["kanji"], d["name"]] for d in Z.GYM_ALL],
    "BOOKS": [[b[0], b[1]] for b in Z.BOOKS],
    "BIO_FACTS": [list(f) for f in Z.BIO_FACTS],
    "ENGLISH_TIPS": [list(t) for t in Z.ENGLISH_TIPS],
    "GRADE_KINDS": Z.GRADE_KINDS,
    "VISION_PULL": Z.VISION_PULL,
    "STUDY_HINTS": Z.STUDY_HINTS,
    "STUDY_APPS": list(Z.STUDY_APPS),
    "SUBJ_STOP": sorted(Z._SUBJ_STOP),
    "FR_IRREG": {k: [v[0], v[1], v[2], v[3], v[4]] for k, v in Z.FR_IRREG.items()},
    "FR_ETRE_VERBS": sorted(Z.FR_ETRE_VERBS),
    "FR_PRONOUNS": Z.FR_PRONOUNS,
    "OUTFITS": [list(o) for o in Z.OUTFITS],
    "OUTFIT_WORDS": Z.OUTFIT_WORDS,
    "BIB_COLOURS": [list(b) for b in Z.BIB_COLOURS],
    "COATS": [[c[0], c[1]] for c in Z.COATS],
    "TREAT_WORDS": Z.TREAT_WORDS,
    "JP_WORDS": [list(w) for w in Z.JP_WORDS],
    "OMIKUJI": [list(o) for o in Z.OMIKUJI],
    "OMI_LINES": Z.OMI_LINES,
    "OMI_LUCKY": Z.OMI_LUCKY,
    "OMI_COLOURS": Z.OMI_COLOURS,
    "LANGS": Z.LANGS,
    "FACT_KEYS": list(Z.FACT_KEYS),
    "DEADLINE_WORDS": Z.DEADLINE_WORDS,
}
out = os.path.join(os.path.dirname(__file__), "..", "crates", "zenko-desktop", "data", "routes.json")
with open(out, "w", encoding="utf-8") as f:
    json.dump(data, f, ensure_ascii=False, separators=(",", ":"))
print("wrote", out, {k: (len(v) if hasattr(v, "__len__") else v) for k, v in data.items()})
os._exit(0)
