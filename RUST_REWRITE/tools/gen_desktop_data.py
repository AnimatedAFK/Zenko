"""Copies the desktop fox's word lists out of the ORIGINAL zenko.py (never modified) into
RUST_REWRITE/crates/zenko-desktop/data/desktop.json: nag/praise/pep lines, distraction names, browsers, the zenko17
rule sites, the subject words used to recognise study videos, and the regular expressions that go with them.
Also writes golden/distraction.json: what zenko.py decides for a list of window titles (study video or not, rules).
Run from the repo root:  python RUST_REWRITE/tools/gen_desktop_data.py"""
import sys
sys.dont_write_bytecode = True
import json, os, tempfile

ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
os.environ["APPDATA"] = tempfile.mkdtemp()
sys.path.insert(0, ROOT)
import zenko as Z  # noqa: E402

terms = {sid: sorted([[w, bool(st)] for _p, st, w in lst]) for sid, lst in Z._sv_terms().items()}
data = {
    "NAGS": Z.NAGS, "PRAISE": Z.PRAISE, "SUGGEST": Z.SUGGEST, "MOTIV": Z.MOTIV, "HALFWAY": Z.HALFWAY, "FINAL": Z.FINAL,
    "PRETTY": Z.PRETTY, "BROWSERS": list(Z.BROWSERS), "LOCK_LINES": Z.LOCK_LINES, "BREAK_NUDGE": Z.BREAK_NUDGE,
    "DEFAULTS": {k: Z.DEFAULTS[k] for k in ("distractions", "distraction_apps", "allow", "idle_minutes", "grace_seconds",
                                             "nag_every_seconds", "passive_minutes", "watch_without_session")},
    "RULE_ALIASES": Z._RULE_ALIASES,
    "SUBJECTS": [[sid, name, short, aliases, topics, sites] for (sid, name, short, col, aliases, topics, sites) in Z.SUBJECT_DEFS],
    "SV_TERMS": terms,
    "RE": {"cues": Z.STUDY_VIDEO_CUES.pattern, "nope": Z.STUDY_VIDEO_NOPE.pattern, "browser_tail": Z._SCREEN_BROWSER_TAIL.pattern,
           "phrase_junk": Z._SV_PHRASE_JUNK.pattern, "title_tails": Z.TITLE_TAILS.pattern},
}
out = os.path.join(ROOT, "RUST_REWRITE", "crates", "zenko-desktop", "data")
os.makedirs(out, exist_ok=True)
with open(os.path.join(out, "desktop.json"), "w", encoding="utf-8") as f:
    json.dump(data, f, ensure_ascii=False, indent=0)

TITLES = [
    "Mitosis and Meiosis explained - IB Biology - YouTube - Google Chrome",
    "(3) Le Chatelier's principle | Equilibrium | Chemistry - YouTube - Microsoft​ Edge",
    "funny cat memes compilation - YouTube - Google Chrome",
    "YouTube - Google Chrome", "Home - YouTube", "Minecraft let's play episode 4 - YouTube",
    "Integration by parts - worked example - YouTube - Mozilla Firefox",
    "Passé composé vs imparfait | French lesson - YouTube", "Hamlet analysis - Act 1 - YouTube - Brave",
    "Milgram obedience study - IB Psychology - YouTube", "lofi hip hop radio - beats to relax/study to - YouTube",
    "Krebs cycles in 5 minutes - YouTube", "The heart - YouTube", "heart and blood explained - YouTube",
    "Taylor Swift - Anti-Hero (Official Music Video) - YouTube", "Cells cells cells lesson - YouTube",
    "Organic chemistry tutor: SN1 vs SN2 - YouTube", "dna's structure - YouTube", "x-ray of the lungs - YouTube",
    "Reddit - Dive into anything - Google Chrome", "Netflix", "Untitled document - Google Docs - Google Chrome",
]
RULES = [{"site": "YouTube", "words": "bio, cell, khan", "do": "keep"},
         {"site": "Reddit", "words": "", "do": "close"},
         {"site": "Any site or app", "words": "chess", "do": "close"},
         {"site": "X / Twitter", "words": "", "do": "close", "on": False}]
cases = []
for t in TITLES + ["Chess.com - Play Chess Online", "Home / X", "khan academy cell biology - YouTube"]:
    low = t.lower()
    sv = Z.study_video_subject(Z._SCREEN_BROWSER_TAIL.sub("", t))
    cls = Z.classify_subject(low, None)
    do, r = Z.rule_verdict(RULES, t, "chrome.exe")
    cases.append({"title": t, "sv": sv[0], "cls": [cls[0], cls[1], cls[2]], "rule": do,
                  "rule_i": RULES.index(r) if r else -1, "key": Z.window_key(t)})
with open(os.path.join(ROOT, "RUST_REWRITE", "golden", "distraction.json"), "w", encoding="utf-8") as f:
    json.dump({"rules": RULES, "cases": cases}, f, ensure_ascii=False, indent=0)
print(len(cases), "titles; data", sum(len(v) for v in terms.values()), "study words")
