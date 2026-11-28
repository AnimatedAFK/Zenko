"""golden/md_segments.json: zenko.py's Markdown / LaTeX / chemistry formatter (md_segments, tex_segments,
chem_segments, md_plain) on a corpus of tricky replies. Run: python3 tools/make_golden_md.py"""
import sys
sys.dont_write_bytecode = True
import json, os
ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
sys.path.insert(0, ROOT)
import zenko as Z  # noqa: E402

MD = [
    "Hi! Type 'help' to see what I can do.",
    "## Osmosis\nOsmosis is the **net movement** of *water molecules* across a **partially permeable membrane**.\n"
    "- Water moves down its gradient\n- Solutes cannot cross\n  - nested point here\n\t- tab nested\n      - deep\n        - deeper\n"
    "1. First\n2) Second\na. letter item\n> A quote with **bold** inside\n>no space quote\n"
    "Use `inline code` and ``double `tick` code`` and a [link](https://example.com/a?b=1) plus H~2~O and x^2^ too.\n"
    "```python\ndef f(x):\n    return x * 2\n```\n| a | b | long header |\n|:--|--:|---|\n| 1 | **2** | `c` |\n| only one |\nend",
    "The value is $x^2 + \\frac{1}{2}$, and ~~wrong~~ right: ***both*** work.\n### Small heading ###\n---\n* * *\n___\nDone~",
    "$$\n\\int_0^1 x^2\\,dx = \\frac{1}{3}\n$$\nand $$E = mc^2$$ inline, \\(a+b\\) and \\[c^{2}\\] forms.",
    "$$ \\sum_{n=1}^{\\infty} \\frac{1}{n^2} = \\frac{\\pi^2}{6}",
    "Chemistry: \\ce{H2SO4 + 2NaOH -> Na2SO4 + 2H2O} and \\ce{Fe^{3+}} and \\ce{CO3^2-} and \\ce{N2 + 3H2 <=> 2NH3}.",
    "Bare LaTeX: \\frac{a+b}{c} and \\sqrt[3]{x} and \\sqrt{2} with \\alpha \\beta \\Delta H = -286 kJ",
    "Matrix $\\begin{pmatrix} 1 & 2 \\\\ 3 & 4 \\end{pmatrix}$ and cases $f(x) = \\begin{cases} x & x>0 \\\\ -x & x\\le 0 \\end{cases}$",
    "Binomial $\\binom{n}{k}$, colour $\\color{red}{x+1}$, text $\\text{if } x \\in \\mathbb{R}$, $\\vec{v}$ and $\\hat{x}$, $\\bar{x}$",
    "Angles: $30^\\circ$ and $x^o$ and $\\dot{x}$, $\\left( \\frac{1}{2} \\right)$, $\\left. x \\right|$ and $a \\, b \\; c \\! d$",
    "Escapes: $\\{1,2\\}$ and $50\\%$ and $\\$5$ cost $5 and $10 later; price is $3.50 not math",
    "Subs: x_{n+1} = x_n^2 and a_1 + CO_2 and e^{i\\pi} + 1 = 0 and 10^{-3} and 2^10 and k^+ ions and n^2",
    "Stray marks: ****word**** and a lone ** here and *unclosed and __under__ and _it_ and snake_case_name ok",
    "HTML: <sup>2</sup> and <sub>3</sub> and <b>bold</b> and <i>it</i> line<br>break<br/>again<br />end",
    "JSON mangled: \x0crac{1}{2} and \times 3 and \theta and \rightarrow and \x08eta",
    "Nested: **bold with *italic* inside** and *italic with **bold** inside* and # not heading\n#also not\n####### seven",
    "Table only\n| x | y |\n| - | - |\n| 1 | 2 |",
    "| unicode | ✓ |\n|---|---|\n| " + "w" * 50 + " | short |",
    "Code unclosed\n```\nprint(1)\nstill code",
    "trailing newlines\n\n\n",
    "Text with \\\\ newline in math $a \\\\ b$ and \\begin{aligned} x &= 1 \\\\ y &= 2 \\end{aligned}",
    "Greek spacing: $\\Delta H$ and $\\Delta  T$ and $\\pi r^2$ and $\\lambda x$ and $\\sigma_x$",
    "Specials: $\\underbrace{x+y}$ $\\overrightarrow{AB}$ $\\tilde{a}$ $\\mathbf{v}$ $\\boxed{42}$ $\\hline$ $\\quad$ $\\to$ $\\unknowncmd$",
    "Units \\pu{25 kJ mol^-1} and $\\ce{H2O}$ in math and $\\frac{dy}{dx}$ and $\\frac{\\sqrt{x}}{2}$ and $\\dfrac{a}{b+c}$",
    "Emoji 🦊 and 日本語 **太字** and *イタリック*",
    "  indented paragraph\n   * star bullet\n+ plus bullet\n• bullet char",
    "10. ten\n  3) nested three\nb) bee",
    "Edge $ x$ and $x $ and $$ and $5$ and a$b$c and \\$ escaped",
    "Links [one](http://a.b) [two](https://c.d/e_f) [bad](ftp://x) [](https://empty)",
    "## Heading with **bold** and $x^2$\nPara ~~strike *it*~~ end",
]
TEX = ["x^2", "\\frac{a}{b}", "H_2O", "\\sqrt[n]{x+1}", "\\begin{bmatrix}1&0\\\\0&1\\end{bmatrix}", "a\\\\b", "\\Delta x",
       "\\ce{Cu^{2+}}", "\\binom{5}{2}", "{x}^{\\circ}", "\\textcolor{blue}{z}", "\\phantom{x}y", "\\left\\{ x \\right\\}"]
CHEM = ["H2SO4", "Fe^{3+}", "CO3^2-", "2H2 + O2 -> 2H2O", "A <-> B", "Ca(OH)2", "[Cu(NH3)4]^2+", "C6H12O6", "e^-"]
out = {"md": [{"text": t, "segs": [[s, list(g)] for s, g in Z.md_segments(t, ("fox",))], "plain": Z.md_plain(t)} for t in MD],
       "tex": [{"text": t, "segs": [[s, list(g)] for s, g in Z.tex_segments(t, ("md_math",))]} for t in TEX],
       "chem": [{"text": t, "segs": [[s, list(g)] for s, g in Z.chem_segments(t, ())]} for t in CHEM]}
with open(os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "golden", "md_segments.json"), "w") as f:
    json.dump(out, f, ensure_ascii=False, indent=0)
print(len(out["md"]), len(out["tex"]), len(out["chem"]))
