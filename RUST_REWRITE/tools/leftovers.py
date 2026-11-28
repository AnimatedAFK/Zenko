"""Stage 6 check: which zenko.py functions/methods the Rust code never names (in code or in its /// zenko.py notes).
A name that appears is not proof of a full port, and a missing one may be folded into another function; this is a
map of where to look. Run from anywhere: python3 RUST_REWRITE/tools/leftovers.py -> RUST_REWRITE/docs/LEFTOVERS.md"""
import ast, os, re, sys
sys.dont_write_bytecode = True
HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(os.path.dirname(HERE))
src = open(os.path.join(ROOT, "zenko.py"), encoding="utf-8").read()
rs = ""
for d, _, fs in os.walk(os.path.join(ROOT, "RUST_REWRITE", "crates")):
    if "/target" in d:
        continue
    for f in fs:
        if f.endswith(".rs"):
            rs += open(os.path.join(d, f), encoding="utf-8").read() + "\n"
words = set(re.findall(r"[A-Za-z_][A-Za-z0-9_]*", rs))
items = []
for n in ast.parse(src).body:
    if isinstance(n, ast.FunctionDef):
        items.append(("", n))
    elif isinstance(n, ast.ClassDef):
        for m in n.body:
            if isinstance(m, ast.FunctionDef):
                items.append((n.name, m))
miss, seen = {}, 0
for cls, f in items:
    nm = f.name
    if nm.startswith("__") or nm.lstrip("_") in words or nm in words:
        seen += 1
        continue
    miss.setdefault(cls or "(module)", []).append("%s (%d lines, L%d)" % (nm, f.end_lineno - f.lineno + 1, f.lineno))
tot = sum(len(v) for v in miss.values())
lines = ["# What the Rust code never names", "",
         "Made by `tools/leftovers.py`: %d of %d zenko.py functions and methods are named somewhere in the Rust code; "
         "these %d are not." % (seen, len(items), tot), "",
         "Triage (2026-11-28, after 5d):",
         "- **Stage 5b (brains and web, deferred by the user):** web search and summaries (search_*, wiki_*, page_text, "
         "research_text, web_route, start_web/web_worker), Ollama and the online brain (llm_*, online_*, parse_brain, "
         "ib_recheck, nb_answer, src_llm), the food lookup online, screenshot/vision reading (ollama_vision, grab_screen_png, "
         "clipboard_image).",
         "- **Not ported, outside 5b:** the reMarkable link (rm_*: pairing, downloads, page reading, which also needs the "
         "vision model), the timetable PDF export (make_timetable_pdf; its menu item says \"later\"), files dropped on "
         "her or attached in chat (drop_files_hook, read_any_file, extract_deadlines, _ics_items, _xlsx_text, pdf_text), "
         "zenko.py's own code updater (code_*, run_code_update: the Rust exe updates itself instead).",
         "- **The rest** are mostly ported under other names (Tk event plumbing, cards built inside a page module, "
         "helpers folded into callers); tools/leftovers.py only checks names.", ""]
for cls, v in sorted(miss.items(), key=lambda kv: -len(kv[1])):
    lines.append("- **%s** (%d): %s" % (cls, len(v), ", ".join(v)))
open(os.path.join(ROOT, "RUST_REWRITE", "docs", "LEFTOVERS.md"), "w", encoding="utf-8").write("\n".join(lines) + "\n")
print(seen, len(items), tot)
for cls, v in sorted(miss.items(), key=lambda kv: -len(kv[1]))[:30]:
    print(cls, len(v), sum(int(x.split("(")[1].split()[0]) for x in v))
