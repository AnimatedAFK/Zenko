"""crates/zenko-desktop/data/tools.json (the tool windows' tables: periodic table, calculator keys, Bible books),
data/lexicon.json (zenko.py's bundled STEPBible brief lexicons, decoded) and golden/tools.json: zenko.py's tool maths
(electron configurations, table places, molar masses, the CG50's prepare / eval / natural display / roots / linear
systems / SolveN / statistics / distributions), the French vocab search, Bible references, STEP rows and grammar hints."""
import sys
sys.dont_write_bytecode = True
import json, math, os, tempfile
sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", ".."))
os.environ.setdefault("APPDATA", tempfile.mkdtemp())
import zenko as Z  # noqa: E402

HERE = os.path.dirname(__file__)
L = lambda v: json.loads(json.dumps(v))
data = {"PT": [[e["sym"], e["name"], e["mass"], e["cat"], e["en"]] for e in Z.PT],
        "PT_CATS": [[k, v[0], v[1]] for k, v in Z.PT_CATS.items()], "PT_EXC": {str(k): v for k, v in Z._PT_EXC.items()},
        "CG_KEYS": L(Z._CG_KEYS), "CG_INS": Z._CG_INS, "CG_SHIFT": Z._CG_SHIFT, "CG_VIEWS": L(Z.CG_VIEWS),
        "BIBLE_BOOKS": L(Z.BIBLE_BOOKS), "BIBLE_ALIAS": Z._BIBLE_ALIAS, "BIBLE_TRANS": L(Z.BIBLE_TRANS),
        "BIBLE_NOTICE": Z.BIBLE_NOTICE, "STEP_GUESS": Z.STEP_GUESS, "BIBLE_STUDY_PROMPT": Z.BIBLE_STUDY_PROMPT}
with open(os.path.join(HERE, "..", "crates", "zenko-desktop", "data", "tools.json"), "w", encoding="utf-8") as f:
    json.dump(data, f, ensure_ascii=False, separators=(",", ":"))
Z._lex_bundled("TBESH")
with open(os.path.join(HERE, "..", "crates", "zenko-desktop", "data", "lexicon.json"), "w", encoding="utf-8") as f:
    json.dump(Z._STEP["bundle"], f, ensure_ascii=False, separators=(",", ":"))


def tryit(fn, *a, **k):
    try:
        r = fn(*a, **k)
        if isinstance(r, complex):
            return {"c": [r.real, r.imag]}
        if isinstance(r, float) and (r != r or r in (float("inf"), float("-inf"))):
            return {"f": str(r)}
        return {"ok": L(r) if not isinstance(r, tuple) else L(list(r))}
    except Exception as e:
        return {"err": type(e).__name__, "msg": str(e)}


g = {}
g["pt"] = [[Z.pt_config(z), Z.pt_full_config(z), list(Z.pt_place(z))] for z in range(1, 119)]
FORMS = ["H2SO4", "Ca(OH)2", "CuSO4·5H2O", "[Cu(NH3)4]SO4", "NaCl", "C6H12O6", "SO4^2-", "NH4+", "Fe2(SO4)3", "h2o",
         "H2O)", "(H2O", "Xx", "", "2H2O", "CH3COOH", "K4[Fe(CN)6]", "Og", "Uuo", "C 6 H 6", "MgSO4.7H2O", "Al2O3*2H2O"]
g["molar"] = [[f, tryit(Z.molar_mass, f)] for f in FORMS]
EXPRS = ["1+2", "2π", "3(4)", "2sin(30)", "sin(30)", "cos(60)+tan(45)", "sin⁻¹(0.5)", "√(2)", "³√(-27)", "2^10", "2^0.5",
         "(-8)^(1/3)", "(-8)^0.5", "(-2)^(2/3)", "5!", "3!+1", "0÷0", "1÷0", "10^(3)", "e^(1)", "ln(e)", "log(100)",
         "log(2,8)", "logab(2,8)", "2×10^3", "1ᴇ3", "−5+3", "-3²", "2⁻¹", "Ans+1", "Ans×2", "X+1", "A+B", "A×2", "2A",
         "AB", "nCr(5,2)", "nPr(5,2)", "nCr(2,5)", "fact(171)", "Int(3.7)", "Frac(3.7)", "root(3,8)", "abs(-4)", "exp(0)",
         "sinh(1)", "1/3", "(1+2", "1+", "", " ", "2**3", "2^^3", "pi", "x", "2x", "sqrt(-1)", "ln(0)", "1e400", "10^400",
         "2^100000", "0.1+0.2", "1/7", "22/7", "πr", "3.5.2", ".5", "5.", "2(3)(4)", "sin(x)", "θ+1", "1,2", "1<2", "a+1",
         "tan(90)", "cos⁻¹(2)", "2.5!", "(-1)!", "17!", "1234567890123", "123456789×1000", "1÷3×3", "√(8)", "√(12)/2",
         "√(0.5)", "π/4", "3π/2", "-π", "2^(-1)", "1.5e3", "1E3", "1.2e-15"]
mem = {"A": 2.0, "B": 3.0, "X": 5.0}
g["prep"] = [[e, Z.calc_prepare(e)] for e in EXPRS]
g["eval"] = [[e, deg, tryit(Z.cg_eval, e, 7.0, deg, None, mem)] for e in EXPRS for deg in (True, False)]
g["eval_x"] = [[e, x, tryit(Z.cg_eval, e, 0.0, False, x, mem)] for e in ["x^2", "sin(x)", "1/x", "√(x)", "x!", "2x+1"]
               for x in (-2.0, 0.0, 0.5, 3.0)]
VALS = [0.0, 1.0, -1.0, 0.5, 1 / 3, 2 / 3, -0.25, math.pi, math.pi / 2, -math.pi, 2 * math.pi / 3, math.sqrt(2),
        math.sqrt(3) * 2, math.sqrt(2) / 2, math.sqrt(12), 1.23456789012, 123456789012.0, 1e-10, 1e10, 9999999999.0,
        0.1 + 0.2, 1e15, -1e-12, 7 / 9, 1 / 1001, 355 / 113, 2.5, 1e300, 1e-300, 12345.678, math.e, math.sqrt(5) / 3,
        -math.sqrt(7), 0.333333333333, 1e7 + 0.5, 3.0000000001, 1 / 997, 6 / 7 * math.pi, math.sqrt(999999),
        math.sqrt(1000001), 5e-324, -0.0, 100.0, 1e9, 1.5e-9]
g["fmt"] = [[v, tryit(Z.calc_fmt, v, True), tryit(Z.calc_fmt, v, False)] for v in VALS]
POLYS = [[1, -3, 2], [1, 0, 1], [1, 2, 1], [2, -4], [0, 1, -1], [1, 0, 0, -1], [1, -6, 11, -6], [1, 0, 0, 0, -16],
         [1, 2, 3, 4, 5], [1, 0, -2, 0, 1], [3, 0, -1, 0, 0, 2], [1, -1, 1, -1, 1, -1, 1], [0, 0, 5], [1]]
g["roots"] = [[p, tryit(lambda p=p: [[r.real, r.imag] for r in Z.calc_roots_poly(p)])] for p in POLYS]
LIN = [[[1, 1, 3], [1, -1, 1]], [[2, 1, -1, 8], [-3, -1, 2, -11], [-2, 1, 2, -3]], [[1, 2, 3], [2, 4, 6]],
       [[0, 1, 2], [1, 0, 3]], [[1, 1, 1, 1, 10], [1, -1, 1, -1, 0], [2, 1, 0, 1, 9], [0, 0, 1, 1, 7]]]
g["linear"] = [[m, tryit(Z.calc_solve_linear, m)] for m in LIN]
EQS = ["x^2=2^x", "sin(x)=0.5", "x³−2x−5", "x^2", "x^2+1", "1/x", "tan(x)", "x=x", "2=3", "x^2-2=0", "(x-1)^2",
       "x+", "x=1=2", "", "3", "√(x)=2", "ln(x)=1", "x^3=x"]
g["solve"] = [[e, deg, tryit(Z.cg_solve, e, 0.0, deg, None)] for e in EQS for deg in (False, True)]
g["solve_mem"] = [["x^2=A", tryit(Z.cg_solve, "x^2=A", 0.0, False, {"A": 9.0})]]
DATA = [[1, 2, 3, 4, 5], [2.5, 3.5, 1, 7, 7, 9], [4], [], [1, 1, 1, 1]]
g["stats"] = [[d, tryit(lambda d=d: [[k, v if not (isinstance(v, float) and v != v) else "nan"] for k, v in Z.calc_stats1(d)])]
              for d in DATA]
REG = [[[1, 2, 3, 4], [2, 4.1, 5.9, 8.2]], [[1, 1, 1], [1, 2, 3]], [[1, 2], [3, 3]], [[1], [2]], [[1, 2, 3], [1, 2]]]
g["linreg"] = [[x, y, tryit(lambda x=x, y=y: [[k, v if not (isinstance(v, float) and v != v) else "nan"] for k, v in Z.calc_linreg(x, y)])]
               for x, y in REG]
g["normcd"] = [[a, tryit(Z.calc_normcd, *a)] for a in [[-1, 1, 1, 0], [-1e99, 0, 1, 0], [0, 1e99, 1, 0], [90, 110, 10, 100],
                                                         [1, 2, 0, 0], [-1.96, 1.96, 1, 0]]]
g["invnorm"] = [[a, tryit(lambda a=a: list(Z.calc_invnorm(*a)) if a[-1] == "center" else Z.calc_invnorm(*a))]
                for a in [[0.5, 1, 0, "left"], [0.975, 1, 0, "left"], [0.05, 2, 10, "right"], [0.95, 1, 0, "center"],
                          [0, 1, 0, "left"], [1, 1, 0, "left"], [0.3, 0, 0, "left"]]]
g["binpd"] = [[a, tryit(Z.calc_binpd, *a)] for a in [[3, 10, 0.5], [0, 5, 0.2], [2.5, 10, 0.5], [11, 10, 0.5], [3, 10, 1.5],
                                                       [3, 10.5, 0.5], [0, 0, 0.3], [5, 5, 1.0]]]
g["bincd"] = [[a, tryit(Z.calc_bincd, *a)] for a in [[5, 10, 0.5, 0], [3, 10, 0.3, 1], [10, 10, 0.5, 0], [2.7, 10, 0.5, 0],
                                                       [5, 10, 0.5, 6]]]
QS = ["manger", "eat", "chat", "le chat", "maison", "house", "être", "etre", "dog", "rouge", "Bonjour", "", "xyz", "a",
      "pomme", "école", "to go", "la", "bien"]
g["fr"] = [[q, L(Z.fr_vocab_search(q)), Z.looks_french(q)] for q in QS + ["je suis content", "I am happy", "très bien",
                                                                          "the cat is on the table", "fais", "allons"]]
REFS = ["Psalm 119:105", "ps 23", "John 3:16-18", "1 Cor 13", "Gen 1", "genesis 50:26", "Rev 22:21", "Song of Songs 2",
        "1jn 4:8", "iii john 1", "ii Kings 2", "Jude 3", "phil 4:13", "Mk 1.1", "Eze 37", "jn3:16", "Hello", "Matt 99",
        "obadiah 1", "Isa 53:5", "2 tim 3:16", "deut 6:4", "Ecc 3", "Heb 11:1", "Acts 2:38", "Romans 8:28", ""]
g["refs"] = [[r, L(Z.bible_parse_ref(r))] for r in REFS]
g["books"] = [[s, Z.bible_book(s)] for s in ["Psa", "psalms", "Ps", "1 John", "1jn", "i samuel", "ii sam", "song", "job",
                                             "jo", "j", "1 co", "phm", "", "Revelation", "rv", "canticles"]]
g["strip"] = [[t, Z._bible_strip(t)] for t in ["In the <S>7225</S> beginning", "a<sup>1</sup>b", "x<br/>y", "<i>God</i> &amp; man",
                                              "  many   spaces ", "&quot;Hi&quot;<br>"]]
OT = "Gen.1.1#01=L\tבְּ/רֵאשִׁ֖ית\tbe./re.Shit\tin/ beginning\tH9003/{H7225G}\tHR/Ncfsa\tH7225G=רֵאשִׁית=first\tx"
OT2 = "Dan.2.4#03=L\tמַלְכָּ֖א\tmal.Ka'\tO king\t{H4430}\tA/Ncmsd\tH4430=מֶלֶךְ=king"
NT = "Jhn.3.16#01=NKO\tΟὕτως (Houtōs)\tthus\tG3779=ADV\tοὕτω=thus\tx\ty"
NT2 = "Mat.1.1(1.1)#02=K\tγενέσεως (geneseōs)\tof [the] genealogy\tG1078=N-GSF\tγένεσις=origin"
NT3 = "Mrk.16.9#01=O\tἈναστὰς (Anastas)\tHaving risen\tG0450=V-2AAP-NSM\tἀνίστημι=to arise"
g["step"] = [[ln, L(Z.step_parse_line(ln))] for ln in [OT, OT2, NT, NT2, NT3, "junk", "Gen.1.1\tx\ty", "Xyz.1.1#01=L\ta\tb\tc"]]
MS = [("V-AAI-3S", "Greek"), ("N-NSM", "Greek"), ("T-NSM", "Greek"), ("V-PMP-NPM", "Greek"), ("PREP", "Greek"), ("", "Greek"),
      ("HVqp3ms", "Hebrew"), ("HR/Ncfsa", "Hebrew"), ("HC/Vqw3ms", "Hebrew"), ("A/Ncmsd", "Aramaic"), ("HNcmpc/Sp3ms", "Hebrew"),
      ("HVhi1cs", "Hebrew"), ("X", "Hebrew")]
g["morph"] = [[m, lang, Z.BibleWindow.morph_text(m, lang)] for m, lang in MS]
lex = Z.step_lexicon("Greek")
g["lookup"] = [[s, L(Z.step_lookup(lex, s))] for s in ["G3779", "G0450", "G1078", "G1078a", "G9999", "", "G25"]]
g["poly_form"] = [Z.CasioWindow.poly_form(n) for n in range(2, 7)]
g["tokens"] = [[s, Z.CasioWindow.tokens(s)] for s in ["sin(x)+√(2)", "Ans×10^(2)", "x²+3x−4", "sin⁻¹(0.5)", "root(3,8)", "abc"]]
with open(os.path.join(HERE, "..", "golden", "tools.json"), "w", encoding="utf-8") as f:
    json.dump(g, f, ensure_ascii=False)
print({k: len(v) for k, v in g.items()})
