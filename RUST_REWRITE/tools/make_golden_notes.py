"""golden/notes.json: zenko.py's note filing: nb_line_scores and nb_split on mixed free notes (French running into
Biology, maths, headings, learned words, blank lines), nb_safe, nb_match_subject, looks_french, fr_vocab_search,
and nb_import_plan on a little folder tree.
Run: /usr/bin/python3.12 tools/make_golden_notes.py"""
import sys
sys.dont_write_bytecode = True
import json, os, tempfile
os.environ["APPDATA"] = tempfile.mkdtemp()
ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
sys.path.insert(0, ROOT)
import zenko as Z  # noqa: E402

TEXTS = [
    "Bonjour, je m'appelle Anoop et j'habite à Mumbai.\nLe passé composé uses avoir or être.\nMitosis has four phases: prophase, metaphase, anaphase, telophase.\nThe cell membrane is a phospholipid bilayer.",
    "Chem\nmoles = mass / molar mass\nAvogadro's number is 6.02 x 10^23\n\nMath\ndy/dx of sin x is cos x\nintegrate 3x^2 + 2",
    "random thought about lunch\nbuy milk\n\nphotosynthesis happens in the chloroplast\nlight dependent reactions make ATP",
    "psych: the bystander effect and conformity\nAsch's line study\nmilgram obedience\nenglish paper 1 tips\nanalyse the tone of the extract",
    "la maison est grande\nnous allons au cinéma\nenzymes lower activation energy\ndenaturation at high temperature",
    "",
    "   \n\n",
    "no subject here at all\njust words",
    "Bio HL\nDNA replication is semi-conservative\nhelicase unzips\nFrench\nles verbes irréguliers\naller, être, avoir",
    "equilibrium constant Kc\nLe Chatelier's principle\nshift to the right\nlog(x) + ln(y) = 2",
    "chat\nmerci beaucoup\nmanger",
]
LIVES = [{}, {"subj_words": {"glycolysis": "bio", "stoichiometry": "chem", "mumbai": "english"}},
         {"subject_names": {"bio": "Biology (HL)", "psych": "Psychology/SL"}}]
out = {"split": [], "scores": [], "safe": [], "match": [], "french": [], "vocab": [], "plan": []}
for li, life in enumerate(LIVES):
    for t in TEXTS:
        out["split"].append({"life": li, "text": t, "parts": Z.nb_split(t, life)})
        for ln in t.split("\n"):
            if ln.strip():
                sc, al = Z.nb_line_scores(ln, life)
                out["scores"].append({"life": li, "line": ln, "sc": sc, "alias": al})
out["lives"] = LIVES
for n in ["Bio", "  my: notes? ", "...", "a/b\\c|d", "x" * 100, "", "IB Chemistry", "French_notes", "maths-hw", "psych.SL",
          "Eng Lit", "Random"]:
    out["safe"].append([n, Z.nb_safe(n)])
    out["match"].append([n, Z.nb_match_subject(n, {}), Z.nb_match_subject(n, LIVES[2])])
for q in ["bonjour", "chat", "la maison", "the house", "manger", "hello world", "aller", "je suis là", "maison",
          "le chien est noir", "dog", "house", "travailler", "école"]:
    out["french"].append([q, Z.looks_french(q)])
    out["vocab"].append([q, [list(x) for x in Z.fr_vocab_search(q, 30)]])
src = tempfile.mkdtemp()
for rel in ["Bio/cells.md", "Bio/sub/dna.txt", "Chemistry notes/acids.md", "misc/todo.txt", "French.md", "photo.png",
            ".hidden/x.md", "desktop.ini", "Maths/calc/limits.md", "readme.txt"]:
    p = os.path.join(src, "Export", rel)
    os.makedirs(os.path.dirname(p), exist_ok=True)
    with open(p, "w") as f:
        f.write(rel)
for top in ["Export", "Export/Bio"]:
    plan = Z.nb_import_plan(os.path.join(src, top), {})
    out["plan"].append({"top": top, "plan": sorted([os.path.relpath(a, src), os.path.relpath(b, Z.NB_DIR)] for a, b in plan)})
with open(os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "golden", "notes.json"), "w") as f:
    json.dump(out, f, ensure_ascii=False, indent=0)
print(len(out["split"]), len(out["scores"]))
