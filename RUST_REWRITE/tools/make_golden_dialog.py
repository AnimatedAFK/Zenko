"""golden/dialog_layout.json: where Tk puts every widget of zenko.py's new-session dialog (open_dialog), with the
size each widget asked for. The Rust grid/pack maths (tkgrid.rs) is checked against it: given the same requested
sizes it must put everything in the same place. Needs a display (run under xvfb-run) and Python with tkinter."""
import sys
sys.dont_write_bytecode = True
import json, os, tempfile
ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
os.environ["APPDATA"] = tempfile.mkdtemp()
sys.path.insert(0, ROOT)
import zenko as Z  # noqa: E402
import tkinter as tk  # noqa: E402

out = []
for case, (task, hint) in enumerate([("Biology notes", ""), ("", "You focus best in the morning; 25 min sessions suit you."),
                                      ("A much longer task name than usual for the box", "Short hint.")]):
    root = tk.Tk()
    root.withdraw()

    class D:
        pass
    d = D()
    d.root, d.dialog, d.session = root, None, None
    d.cfg = {"last_task": task, "last_minutes": 25, "learn_habits": bool(hint)}
    d.say = lambda *a, **k: None
    d.style_titlebar = lambda w: None
    d.small_fox = lambda zoom, bg=None: tk.PhotoImage(width=84, height=80)
    d.tt_entry = lambda *a: Z.FoxBuddy.tt_entry(d, *a)
    d.tt_btn = lambda *a, **k: Z.FoxBuddy.tt_btn(d, *a, **k)
    Z.learned_hint = (lambda h=hint: h)
    Z.ui_sound = lambda *a: None
    Z.FoxBuddy.open_dialog(d)
    dlg = d.dialog
    dlg.update_idletasks()

    def walk(w, depth=0):
        rec = {"cls": w.winfo_class(), "x": w.winfo_x(), "y": w.winfo_y(), "w": w.winfo_width(), "h": w.winfo_height(),
               "rw": w.winfo_reqwidth(), "rh": w.winfo_reqheight(), "kids": []}
        for k in w.winfo_children():
            rec["kids"].append(walk(k, depth + 1))
        return rec
    out.append(walk(dlg))
    root.destroy()
with open(os.path.join(os.path.dirname(__file__), "..", "golden", "dialog_layout.json"), "w") as f:
    json.dump(out, f, indent=1)
print(json.dumps(out[0])[:1500])
