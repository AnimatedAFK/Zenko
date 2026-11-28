//! zenko.py's PeriodicWindow (元素 · the periodic table): the find and molar-mass boxes, the table drawn on a canvas
//! (coloured by category or by electronegativity, with the hovered or picked element large in the gap), and the
//! detail card on the right with an "Ask Zenko" box.

use crate::hubwin::canvas_of;
use crate::ptable::{Elem, by_sym, cat_of, cats, molar_mass, pt, pt_config, pt_full_config, pt_place};
use crate::theme::Theme;
use crate::toolwin::{Tool, ToolFonts, lab_w, set_btn_text, set_fg, set_text};
use crate::ui::{Anchor, Fill, Font, Fonts, Pack, Side, Ui};
use zenko_art::palette::mixhex;

pub const SIZE: (i32, i32) = (1240, 760);
pub const TITLE: &str = "Periodic table";

pub struct PtWin {
    pub t: Tool,
    pub sel: usize,
    pub mode: &'static str,
    pub hover: Option<usize>,
    pub find_e: usize,
    pub mm_e: usize,
    pub mm_out: usize,
    pub mode_btn: usize,
    pub cv: usize,
    pub det: usize,
    pub ans: usize,
    /// the element the ask box is about, when its answer comes
    pub asked: Option<usize>,
}

fn i(x: f64) -> i32 {
    x.trunc() as i32
}

impl PtWin {
    /// build(); `last` is life pt_last
    pub fn build(ui: &mut Ui, th: &Theme, f: &ToolFonts, last: &str, cm10: i32) -> PtWin {
        let t = Tool::new(th, f);
        let win = 0;
        ui.ws[win].bg = th.paper.clone();
        let sel = by_sym(last).map(|e| e.z - 1).unwrap_or(5);
        t.header(ui, win, "\u{5143}\u{7d20}", "The periodic table",
                 "Click an element. Values as in your IB data booklet (Ar to 2 d.p., Pauling electronegativity).");
        let body = ui.frame(win, &th.paper);
        ui.pack(body, Pack::default().fill(Fill::Both).expand());
        let left = ui.frame(body, &th.paper);
        ui.pack(left, Pack::default().side(Side::Left).fill(Fill::Both).expand().padx(14, 6).pady(10, 10));
        let tools = ui.frame(left, &th.paper);
        ui.pack(tools, Pack::default().fill(Fill::X));
        let l = ui.label(tools, "Find", &th.paper, &th.grey, t.fu(9, false));
        ui.pack(l, Pack::default().side(Side::Left));
        let faces = crate::pagekit::Faces { ui: f.ui.clone(), title: String::new() };
        let find_e = crate::pagekit::tt_entry(ui, tools, "", 14, th, &faces);
        ui.pack(find_e, Pack::default().side(Side::Left).padx(6, 14));
        let l = ui.label(tools, "Molar mass", &th.paper, &th.grey, t.fu(9, false));
        ui.pack(l, Pack::default().side(Side::Left));
        let mm_e = crate::pagekit::tt_entry(ui, tools, "H2SO4", 16, th, &faces);
        ui.pack(mm_e, Pack::default().side(Side::Left).padx(6, 6));
        let mm_out = ui.label(tools, "", &th.paper, &th.ink, t.fu(10, true));
        ui.pack(mm_out, Pack::default().side(Side::Left).padx(6, 6));
        let mode_btn = t.btn(ui, tools, "Colour: electronegativity", "pt_mode", false, 9);
        ui.pack(mode_btn, Pack::default().side(Side::Right));
        let (cw, ch) = crate::pagekit::cm(cm10);
        let cv = ui.canvas(left, &th.paper, cw, ch);
        canvas_of(ui, cv);
        ui.pack(cv, Pack::default().fill(Fill::Both).expand().pady(8, 0));
        let right = ui.frame(body, &th.card);
        ui.ws[right].opt_w = 330;
        ui.ws[right].propagate = false;
        ui.pack(right, Pack::default().side(Side::Right).fill(Fill::Y).padx(6, 14).pady(10, 10));
        let mut p = PtWin { t, sel, mode: "cat", hover: None, find_e, mm_e, mm_out, mode_btn, cv, det: right, ans: 0, asked: None };
        p.molar(ui, "H2SO4");
        p.show_detail(ui);
        p
    }

    fn geom(&self, ui: &Ui) -> (f64, f64) {
        let (w, h) = (ui.ws[self.cv].w as f64, ui.ws[self.cv].h as f64);
        let cw = 26f64.max(70f64.min((w - 8.0) / 18.0));
        let ch = 28f64.max((cw * 1.12).min((h - 10.0) / 10.1));
        (cw, ch)
    }

    fn cell_xy(&self, ui: &Ui, z: usize) -> (f64, f64, f64, f64) {
        let (r, c) = pt_place(z);
        let (cw, ch) = self.geom(ui);
        let y = (r as f64 - 1.0) * ch + if r >= 8 { 0.95 * ch } else { 0.0 };
        (4.0 + (c as f64 - 1.0) * cw, 4.0 + y, cw, ch)
    }

    fn color(&self, e: &Elem) -> String {
        if self.mode == "en" {
            let Some(en) = e.en else { return mixhex(&self.t.th.card, &self.t.th.ink, 0.12) };
            let t = (en - 0.7) / 3.3;
            return mixhex("#3d6fb8", "#e0705a", t.clamp(0.0, 1.0));
        }
        cat_of(&e.cat).1
    }

    /// draw(): the whole table for the canvas's size now
    pub fn draw(&self, ui: &mut Ui) {
        let (cw, ch) = self.geom(ui);
        let big = cw >= 44.0;
        let th = self.t.th.clone();
        let (fu, serif) = (self.t.f.ui.clone(), self.t.f.serif.clone());
        let card = th.card.clone();
        let cells: Vec<(usize, (f64, f64, f64, f64), String)> = pt().iter().map(|e| (e.z, self.cell_xy(ui, e.z), self.color(e))).collect();
        let (sel, hov, mode) = (self.sel, self.hover, self.mode);
        let shown = &pt()[hov.unwrap_or(sel)];
        let shown_col = self.color(shown);
        let cv = canvas_of(ui, self.cv);
        *cv = Default::default();
        for (z, (x, y, w, h), col) in cells {
            let e = &pt()[z - 1];
            let (is_sel, is_hov) = (z - 1 == sel, Some(z - 1) == hov);
            let fill = mixhex(&card, &col, if is_sel || is_hov { 0.55 } else { 0.30 });
            cv.rect(x + 1.0, y + 1.0, x + w - 2.0, y + h - 2.0).fill(&fill)
                .outline(&if is_sel { th.gold.clone() } else { mixhex(&card, &col, 0.8) })
                .width(if is_sel { 2.0 } else { 1.0 }).tags(&["el", &format!("z{z}")]);
            cv.text(x + 4.0, y + 3.0, &z.to_string()).anchor("nw").fill(&mixhex(&th.ink, &fill, 0.35))
                .font(Font::new(&fu, 6.max(i(cw / 7.5)), false));
            cv.text(x + w / 2.0, y + h * if big { 0.5 } else { 0.56 }, &e.sym).fill(&th.ink)
                .font(Font::new(&serif, 8.max(i(cw / 3.3)), true));
            if big {
                cv.text(x + w / 2.0, y + h - 4.0, &e.mass).anchor("s").fill(&mixhex(&th.ink, &fill, 0.3))
                    .font(Font::new(&fu, 6.max(i(cw / 8.5)), false));
            }
        }
        for (r, l) in [(6.0, "57\u{2013}71"), (7.0, "89\u{2013}103")] {
            let (x, y) = (4.0 + 2.0 * cw, 4.0 + (r - 1.0) * ch);
            cv.rect(x + 1.0, y + 1.0, x + cw - 2.0, y + ch - 2.0).fill(&mixhex(&card, &th.paper, 0.5))
                .outline(&mixhex(&card, &th.ink, 0.15)).dash(&[2, 2]);
            cv.text(x + cw / 2.0, y + ch / 2.0, l).fill(&th.grey).font(Font::new(&fu, 6.max(i(cw / 7.0)), false));
        }
        let ly = 4.0 + 7.0 * ch + 0.18 * ch;
        let lx = 4.0 + 2.6 * cw;
        if mode == "cat" {
            for (k, (_, name, col)) in cats().iter().enumerate() {
                let xx = lx + (k % 5) as f64 * cw * 2.9;
                let yy = ly + (k / 5) as f64 * 15.0 - 2.0;
                cv.rect(xx, yy, xx + 10.0, yy + 10.0).fill(col).outline("");
                cv.text(xx + 14.0, yy + 5.0, name).anchor("w").fill(&th.grey).font(Font::new(&fu, 8, false));
            }
        } else {
            for k in 0..40 {
                let kf = k as f64;
                cv.rect(lx + kf * 6.0, ly, lx + kf * 6.0 + 6.0, ly + 10.0).outline("").fill(&mixhex("#3d6fb8", "#e0705a", kf / 39.0));
            }
            cv.text(lx, ly + 18.0, "0.7").anchor("w").fill(&th.grey).font(Font::new(&fu, 8, false));
            cv.text(lx + 240.0, ly + 18.0, "4.0  (Pauling electronegativity)").anchor("e").fill(&th.grey).font(Font::new(&fu, 8, false));
        }
        let (x0, y0) = (4.0 + 3.0 * cw, 4.0);
        cv.text(x0 + 4.0, y0 + 4.0, &shown.z.to_string()).anchor("nw").fill(&th.grey).font(Font::new(&fu, 8.max(i(cw / 5.0)), false));
        cv.text(x0 + 4.0, y0 + ch * 0.35, &shown.sym).anchor("nw").fill(&shown_col).font(Font::new(&serif, 16.max(i(cw * 0.9)), true));
        cv.text(x0 + cw * 2.2, y0 + ch * 0.5, &format!("{}\nAr {}   \u{b7}   {}", shown.name, shown.mass, cat_of(&shown.cat).0))
            .anchor("w").fill(&th.ink).font(Font::new(&serif, 9.max(i(cw / 4.0)), false));
    }

    /// at(x, y): the element under a canvas point
    pub fn at(&self, ui: &Ui, x: f64, y: f64) -> Option<usize> {
        pt().iter().position(|e| {
            let (ex, ey, w, h) = self.cell_xy(ui, e.z);
            ex <= x && x < ex + w && ey <= y && y < ey + h
        })
    }

    /// set_hover(e): true when it changed (then the table is drawn again)
    pub fn set_hover(&mut self, e: Option<usize>) -> bool {
        if e != self.hover {
            self.hover = e;
            return true;
        }
        false
    }

    /// click(): Some(symbol for life pt_last) when an element was picked
    pub fn click(&mut self, ui: &mut Ui, x: f64, y: f64) -> Option<String> {
        let e = self.at(ui, x, y)?;
        self.sel = e;
        self.show_detail(ui);
        Some(pt()[e].sym.clone())
    }

    /// find(): the find box's text, typed
    pub fn find(&mut self, ui: &mut Ui, q: &str) -> bool {
        let q = q.trim().to_lowercase();
        if q.is_empty() {
            return false;
        }
        let hit = pt().iter().position(|e| q == e.sym.to_lowercase() || q == e.z.to_string())
            .or_else(|| pt().iter().position(|e| e.name.to_lowercase().starts_with(&q)));
        if let Some(h) = hit {
            self.sel = h;
            self.show_detail(ui);
            return true;
        }
        false
    }

    /// molar(): the molar-mass box's text, typed
    pub fn molar(&mut self, ui: &mut Ui, f: &str) {
        let f = f.trim();
        if f.is_empty() {
            set_text(ui, self.mm_out, "");
            return;
        }
        match molar_mass(f) {
            Ok((mm, _)) => {
                set_text(ui, self.mm_out, &format!("= {mm:.2} g mol\u{207b}\u{b9}"));
                set_fg(ui, self.mm_out, &self.t.th.ink.clone());
            }
            Err(e) => {
                set_text(ui, self.mm_out, &e.chars().take(60).collect::<String>());
                set_fg(ui, self.mm_out, &self.t.th.grey.clone());
            }
        }
    }

    pub fn toggle_mode(&mut self, ui: &mut Ui) {
        self.mode = if self.mode == "cat" { "en" } else { "cat" };
        set_btn_text(ui, self.mode_btn, if self.mode == "en" { "Colour: categories" } else { "Colour: electronegativity" });
    }

    pub fn show_detail(&mut self, ui: &mut Ui) {
        for k in ui.ws[self.det].kids.clone() {
            ui.destroy(k);
        }
        self.t.bars.clear();
        let e = pt()[self.sel].clone();
        let d = self.det;
        let th = self.t.th.clone();
        let bg = th.card.clone();
        let col = cat_of(&e.cat).1;
        let (fu, serif) = (self.t.f.ui.clone(), self.t.f.serif.clone());
        let top = ui.frame(d, &bg);
        ui.pack(top, Pack::default().fill(Fill::X).padx(16, 16).pady(14, 4));
        let l = ui.label(top, &e.sym, &bg, &col, Font::new(&serif, 34, true));
        ui.pack(l, Pack::default().side(Side::Left));
        let c2 = ui.frame(top, &bg);
        ui.pack(c2, Pack::default().side(Side::Left).padx(12, 12));
        let l = ui.label(c2, &e.name, &bg, &th.ink, Font::new(&serif, 16, false));
        ui.pack(l, Pack::default().anchor(Anchor::W));
        let l = ui.label(c2, &cat_of(&e.cat).0, &bg, &th.grey, Font { family: fu.clone(), size: 9, bold: false, italic: true });
        ui.pack(l, Pack::default().anchor(Anchor::W));
        let (r, c) = pt_place(e.z);
        let period = match r { 8 => 6, 9 => 7, x => x };
        let block = if r >= 8 { "f" } else if c <= 2 || e.z == 2 { "s" } else if c >= 13 { "p" } else { "d" };
        let rows = [("Atomic number", e.z.to_string()),
                    ("Relative atomic mass", e.mass.clone() + if e.mass.contains('[') { "  (mass number of the most stable isotope)" } else { "" }),
                    ("Period \u{b7} group \u{b7} block", format!("{period} \u{b7} {} \u{b7} {block}", if r >= 8 { "\u{2014}".to_string() } else { c.to_string() })),
                    ("Electronegativity", e.en.map(|v| format!("{v:.1}")).unwrap_or("\u{2014}".into())),
                    ("Electron configuration", pt_config(e.z)), ("Full configuration", pt_full_config(e.z))];
        for (k, v) in rows {
            let l = lab_w(ui, d, &k.to_uppercase(), &bg, &th.grey, Font::new(&fu, 7, true), Anchor::W, false, 0);
            ui.pack(l, Pack::default().fill(Fill::X).padx(16, 16).pady(6, 0));
            let l = lab_w(ui, d, &v, &bg, &th.ink, Font::new(&fu, 10, false), Anchor::W, true, 290);
            ui.pack(l, Pack::default().fill(Fill::X).padx(16, 16));
        }
        let exc = crate::ptable::tools()["PT_EXC"].get(e.z.to_string()).is_some();
        if exc && e.z <= 79 {
            let l = lab_w(ui, d, "An exception to the Aufbau order (a half-full or full sub-level is more stable).", &bg, &th.gold,
                          Font { family: fu.clone(), size: 8, bold: false, italic: true }, Anchor::W, true, 290);
            ui.pack(l, Pack::default().fill(Fill::X).padx(16, 16).pady(4, 0));
        }
        let br = ui.frame(d, &bg);
        ui.pack(br, Pack::default().fill(Fill::X).padx(16, 16).pady(12, 6));
        let b = self.t.btn(ui, br, &format!("Ask Zenko about {}", e.name), "pt_ask", true, 9);
        ui.pack(b, Pack::default().side(Side::Left));
        self.ans = self.t.text_box(ui, d, 8, 9);
        ui.pack(self.ans, Pack::default().fill(Fill::Both).expand().padx(12, 12).pady(0, 12));
        self.t.show_md(ui, self.ans, "Ask me about its reactions, trends, ions or uses (IB Chemistry level).", 9);
    }

    /// ask(): the question for nb_ask("chem", q, "explain"); the box shows "Thinking~ kon..." until the answer
    pub fn ask(&mut self, ui: &mut Ui) -> String {
        let e = &pt()[self.sel];
        self.asked = Some(self.sel);
        let ans = self.ans;
        self.t.show_md(ui, ans, "Thinking~ kon...", 9);
        format!("Give the IB DP Chemistry facts about {} ({}, Z={}): its electron configuration and what it explains, typical \
                 ions/oxidation states, key reactions or uses that come up in IB Chemistry, and where it sits in periodic \
                 trends (atomic radius, ionisation energy, electronegativity). Short sections.", e.name, e.sym, e.z)
    }

    pub fn answer(&mut self, ui: &mut Ui, txt: &str) {
        let ans = self.ans;
        if ans < ui.ws.len() {
            self.t.show_md(ui, ans, txt, 9);
        }
    }

    /// what Tk has settled to: the layout for the window's size, the table drawn for the canvas's size, the bars
    pub fn fit(&mut self, ui: &mut Ui, fonts: &dyn Fonts, w: i32, h: i32) {
        ui.layout_at(fonts, w, h);
        self.draw(ui);
        self.t.paint_bars(ui, fonts);
    }
}
