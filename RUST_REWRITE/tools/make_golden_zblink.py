"""golden/zblink.json: zenko.py's built-in summariser (_sentences, _terms, summarise_extract, summarise_offline) and the
Zenko Browser card makers (Zenko22.z22_split_card, z22_cloze) on made-up pages."""
import sys
sys.dont_write_bytecode = True
import json, os, tempfile
sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", ".."))
os.environ.setdefault("APPDATA", tempfile.mkdtemp())
import zenko as Z  # noqa: E402

HERE = os.path.dirname(__file__)
PAGES = [
    ("Osmosis", "Osmosis is the movement of water molecules across a partially permeable membrane. It goes from a region of "
     "higher water potential to a region of lower water potential.\n\nCells in a hypotonic solution gain water and may "
     "swell. Plant cells become turgid because the Cell wall resists the pressure. Animal cells such as Erythrocytes can "
     "burst, which is called haemolysis.\nIn a hypertonic solution, cells lose water by Osmosis and plant cells become "
     "plasmolysed as the membrane pulls away from the wall. Accept all cookies to continue reading this page.\n"
     "• Isotonic solutions cause no net movement of water across the membrane at all.\n"
     "Water potential is measured in kilopascals and pure water has a water potential of zero kPa. Solutes lower the "
     "water potential of a solution, so it becomes negative! (This is why) sugary drinks can dehydrate you. "
     "Scientists like Wilhelm Pfeffer measured osmotic pressure with clay pots in 1877. The Pfeffer cell showed how "
     "pressure depends on concentration and temperature, which van 't Hoff later explained."),
    ("The French Revolution", "The French Revolution was a period of political and societal change in France that began "
     "with the Estates General of 1789 and ended with the coup of 18 Brumaire in November 1799. Its causes are generally "
     "agreed to be a combination of social, political and economic factors which the existing regime proved unable to "
     "manage. Financial crisis and widespread social distress led to the convocation of the Estates General in May 1789. "
     "The Storming of the Bastille on 14 July led to a series of radical measures by the Assembly. Many of its ideas are "
     "considered fundamental principles of liberal democracy. Subscribe to our newsletter for more history! "
     "Napoleon Bonaparte later rose to power during the Consulate and crowned himself Emperor in 1804."),
    ("short", "Too short. Nope."),
    ("", ""),
]
g = {"sentences": [], "terms": [], "extract": [], "offline": [], "cloze": [], "split": []}
for name, body in PAGES:
    g["sentences"].append([body, Z._sentences(body)])
    g["terms"].append([body, Z._terms(body)])
    g["offline"].append([name, body, Z.Zenko22.summarise_offline(Z.Zenko22, name, body)])
    g["cloze"].append([body, [list(x) for x in Z.Zenko22.z22_cloze(Z.Zenko22, body)]])
docs = [{"title": n, "text": b, "url": ""} for n, b in PAGES]
for q, n, lim in [("what is osmosis", 3, 600), ("french revolution causes", 5, 900), ("", 7, 1300), ("water", 2, 100)]:
    g["extract"].append([q, n, lim, Z.summarise_extract(q, docs, n=n, limit=lim)])
for t in ["osmosis - water moving across a membrane", "  H2O = water.  ", "Mitochondria is the powerhouse of the cell",
          "a: b", "no separator here at all", "x = ", "The cell membrane refers to   a thin layer", "Glucose: C6H12O6 – sugar",
          "ab is cd", "  ...  "]:
    r = Z.Zenko22.z22_split_card(Z.Zenko22, t)
    g["split"].append([t, list(r) if r else None])
with open(os.path.join(HERE, "..", "golden", "zblink.json"), "w", encoding="utf-8") as f:
    json.dump(g, f, ensure_ascii=False, indent=0)
print("ok", {k: len(v) for k, v in g.items()}, g["offline"][0][2][:300])
