//! zenko.py's CasioWindow (電卓 · an fx-CG50-style graphing calculator): the keypad, and the screen's state machine
//! (RUN-MAT with history and memories, GRAPH with V-Window, trace and G-Solve, EQUATION with SIMUL / POLY / SOLVER,
//! STAT with 1- and 2-variable results, DIST) drawn on its 384 x 216 canvas exactly as zenko.py draws it.

use crate::bible::cg_tokens;
use crate::bookgen::py_round;
use crate::cgmath::{CgErr, Sv, calc_bincd, calc_binpd, calc_fmt, calc_invnorm, calc_linreg, calc_normcd, calc_roots_poly,
                    calc_solve_linear, calc_stats1, cg_eval, cg_solve};
use crate::hubwin::canvas_of;
use crate::mdtext::clip;
use crate::ptable::tools;
use crate::theme::Theme;
use crate::tkcanvas::Canvas;
use crate::toolwin::ToolFonts;
use crate::ui::{Fill, Font, Fonts, Grid as UGrid, Pack, Side, Ui};
use serde_json::{Value, json};
use std::collections::BTreeMap;
use zenko_art::palette::mixhex;

pub const SIZE: (i32, i32) = (470, 800);
pub const TITLE: &str = "Graphing calculator";
pub const SW: f64 = 384.0;
pub const SH: f64 = 216.0;
const BODY: &str = "#1c1f27";

/// a history value: a number, or the error shown in its place
#[derive(Clone, Debug, PartialEq)]
pub enum Hv {
    F(f64),
    S(String),
}

#[derive(Clone, Debug)]
pub struct CGrid {
    pub kind: String,
    pub labels: Vec<String>,
    pub rows: Vec<Vec<Option<f64>>>,
    pub r: usize,
    pub c: usize,
    pub title: String,
    pub top: usize,
    pub names: Option<Vec<String>>,
    pub which: String,
}

pub struct Casio {
    pub deg: bool,
    pub mem: BTreeMap<String, f64>,
    pub ans: f64,
    pub hist: Vec<(String, Hv)>,
    pub ys: Vec<String>,
    pub mode: String,
    pub inp: Vec<String>,
    pub cur: usize,
    pub shift: bool,
    pub alpha: bool,
    pub exact: bool,
    pub ysel: usize,
    pub view: [f64; 4],
    pub trace: Option<(usize, f64)>,
    pub gmsg: String,
    pub grid: Option<CGrid>,
    pub result: Option<Vec<(String, String)>>,
    pub res_top: usize,
    pub lists: [Vec<f64>; 2],
    pub eq_sub: Option<String>,
    gs: Option<(String, usize)>,
    pub blink: bool,
    /// life["cg"] as zenko.py keeps it (save() and the solver's last equation)
    pub life_cg: Value,
    /// set when life["cg"] changed (the app saves it)
    pub dirty: bool,
    /// save() has run: zenko.py's life["cg"] then holds the window's own ys / mem / lists objects, so later edits
    /// show in it before the next save
    aliased: bool,
    /// SHIFT then AC: the window closes
    pub close: bool,
    pub cv: usize,
    /// the key labels: (widget, key id, its colour)
    pub keys: Vec<(usize, String, String)>,
    serif: String,
    fu: String,
}

type R<T> = Result<T, CgErr>;

fn hv_json(v: &Hv) -> Value {
    match v {
        Hv::F(f) => json!(f),
        Hv::S(s) => json!(s),
    }
}

fn key_col(sty: &str) -> (&'static str, &'static str) {
    match sty {
        "f" => ("#3a4050", "#f0f0f0"),
        "shift" => ("#3a4050", "#e8c547"),
        "alpha" => ("#3a4050", "#e2645a"),
        "k" => ("#2c313d", "#f2f2f2"),
        "arrow" => ("#454c5d", "#f2f2f2"),
        "n" => ("#e4e1d9", "#1b1d24"),
        "del" => ("#c8743a", "#ffffff"),
        _ => ("#3b6ea8", "#ffffff"),
    }
}

fn ins(kid: &str) -> Option<String> {
    tools()["CG_INS"].get(kid).and_then(|v| v.as_str()).map(String::from)
}
fn shifted(kid: &str) -> Option<String> {
    tools()["CG_SHIFT"].get(kid).and_then(|v| v.as_str()).map(String::from)
}
fn alpha_of(kid: &str) -> Option<String> {
    for row in tools()["CG_KEYS"].as_array().unwrap() {
        for k in row.as_array().unwrap() {
            if k[0] == kid && !k[3].as_str().unwrap().is_empty() {
                return Some(k[3].as_str().unwrap().into());
            }
        }
    }
    None
}

fn views() -> Vec<(String, [f64; 4])> {
    tools()["CG_VIEWS"].as_array().unwrap().iter().map(|v| {
        (v[0].as_str().unwrap().into(), [v[1].as_f64().unwrap(), v[2].as_f64().unwrap(), v[3].as_f64().unwrap(), v[4].as_f64().unwrap()])
    }).collect()
}

fn msg(e: &CgErr) -> String {
    e.msg().to_string()
}

const DIST_NAMES: [(&str, &str, &[&str], &[f64]); 4] = [
    ("F1", "Normal C.D", &["Lower", "Upper", "\u{3c3}", "\u{3bc}"], &[-1e99, 0.0, 1.0, 0.0]),
    ("F2", "Inverse Normal (left tail)", &["Area", "\u{3c3}", "\u{3bc}"], &[0.5, 1.0, 0.0]),
    ("F3", "Binomial P.D", &["x", "Numtrial", "p"], &[0.0, 10.0, 0.5]),
    ("F4", "Binomial C.D", &["Lower", "Upper", "Numtrial", "p"], &[0.0, 5.0, 10.0, 0.5]),
];

impl Casio {
    /// build(): `cg` is life["cg"]
    pub fn build(ui: &mut Ui, th: &Theme, f: &ToolFonts, cg: &Value) -> Casio {
        let st = if cg.is_object() { cg.clone() } else { json!({}) };
        let hist: Vec<(String, Hv)> = st["hist"].as_array().map(|h| {
            let n = h.len();
            h[n.saturating_sub(12)..].iter().map(|e| {
                (e[0].as_str().unwrap_or("").to_string(), match &e[1] { Value::String(s) => Hv::S(s.clone()), v => Hv::F(v.as_f64().unwrap_or(0.0)) })
            }).collect()
        }).unwrap_or_default();
        let fl = |k: &str| -> Vec<f64> { st[k].as_array().map(|a| a.iter().filter_map(|v| v.as_f64()).collect()).unwrap_or_default() };
        let ys: Vec<String> = st["ys"].as_array().filter(|a| !a.is_empty())
            .map(|a| a.iter().map(|v| v.as_str().unwrap_or("").to_string()).collect()).unwrap_or(vec!["".into(); 3]);
        let mut c = Casio {
            deg: st.get("deg").map(|v| v.as_bool().unwrap_or(true)).unwrap_or(true),
            mem: st["mem"].as_object().map(|m| m.iter().map(|(k, v)| (k.clone(), v.as_f64().unwrap_or(0.0))).collect()).unwrap_or_default(),
            ans: st["ans"].as_f64().unwrap_or(0.0),
            hist, ys, mode: "run".into(), inp: vec![], cur: 0, shift: false, alpha: false, exact: true, ysel: 0,
            view: views()[0].1, trace: None, gmsg: String::new(), grid: None, result: None, res_top: 0,
            lists: [fl("L1"), fl("L2")], eq_sub: None, gs: None, blink: true, life_cg: st, dirty: false, aliased: false, close: false,
            cv: 0, keys: vec![], serif: f.serif.clone(), fu: f.ui.clone(),
        };
        let win = 0;
        ui.ws[win].bg = BODY.into();
        let top = ui.frame(win, BODY);
        ui.pack(top, Pack::default().fill(Fill::X).padx(18, 18).pady(12, 2));
        let l = ui.label(top, "ZENKO", BODY, "#d9d4c7", Font::new(&f.serif, 11, true));
        ui.pack(l, Pack::default().side(Side::Left));
        let l = ui.label(top, "  colour graphing  \u{b7}  fx-CG50 style", BODY, "#7d8496", Font::new(&f.ui, 8, false));
        ui.pack(l, Pack::default().side(Side::Left));
        let l = ui.label(top, "\u{72d0}", BODY, &th.gold, Font::new(&f.jp, 13, false));
        ui.pack(l, Pack::default().side(Side::Right));
        let bez = ui.frame(win, "#0d0f14");
        (ui.ws[bez].padx, ui.ws[bez].pady) = (8, 8);
        ui.pack(bez, Pack::default().padx(18, 18).pady(4, 8));
        c.cv = ui.canvas(bez, "#f6f7f2", SW as i32, SH as i32);
        canvas_of(ui, c.cv);
        ui.pack(c.cv, Pack::default());
        let pad = ui.frame(win, BODY);
        ui.pack(pad, Pack::default().fill(Fill::Both).expand().padx(12, 12).pady(0, 10));
        for (r, row) in tools()["CG_KEYS"].as_array().unwrap().iter().enumerate() {
            let fr = ui.frame(pad, BODY);
            ui.pack(fr, Pack::default().fill(Fill::X).pady(0, if r > 0 { 3 } else { 8 }));
            let row = row.as_array().unwrap();
            ui.ws[fr].gw_cols = vec![1; row.len()];
            ui.ws[fr].gu_cols = vec!["key"; row.len()];
            for (n, k) in row.iter().enumerate() {
                let s = |i: usize| k[i].as_str().unwrap().to_string();
                let (kid, lab, sl, al, sty) = (s(0), s(1), s(2), s(3), s(4));
                let cell = ui.frame(fr, BODY);
                ui.grid(cell, UGrid { row: 0, col: n, sticky: "ew", padx: (2, 2), ..Default::default() });
                let tf = ui.frame(cell, BODY);
                ui.pack(tf, Pack::default().fill(Fill::X));
                let l = ui.label(tf, &sl, BODY, "#e8c547", Font::new(&f.ui, 6, true));
                ui.pack(l, Pack::default().side(Side::Left).padx(2, 0));
                let l = ui.label(tf, &al, BODY, "#e2645a", Font::new(&f.ui, 6, true));
                ui.pack(l, Pack::default().side(Side::Right).padx(0, 2));
                let (bg, fg) = key_col(&sty);
                let big = matches!(sty.as_str(), "n" | "del" | "exe");
                let kl = ui.label(cell, &lab, bg, fg, Font::new(&f.ui, if big { 11 } else { 8 }, true));
                ui.ws[kl].pady = if big { 7 } else { 4 };
                ui.ws[kl].hand = true;
                ui.pack(kl, Pack::default().fill(Fill::X));
                c.keys.push((kl, kid, bg.into()));
            }
        }
        c
    }

    /// the key under a window point (a key's label)
    pub fn key_at(&self, ui: &Ui, x: i32, y: i32) -> Option<(usize, String)> {
        let id = ui.hit(x, y)?;
        self.keys.iter().find(|k| k.0 == id).map(|k| (k.0, k.1.clone()))
    }

    /// <ButtonPress-1> on a key: darker; `up` the release (back to its colour)
    pub fn key_shade(&self, ui: &mut Ui, id: usize, down: bool) {
        if let Some(k) = self.keys.iter().find(|k| k.0 == id) {
            ui.ws[id].bg = if down { mixhex(&k.2, "#000000", 0.25) } else { k.2.clone() };
        }
    }

    /// on_key(e): the keyboard; `keysym` as Tk names it, `ch` the character typed
    pub fn on_key(&mut self, ui: &mut Ui, keysym: &str, ch: &str) -> bool {
        let m = match keysym {
            "Return" | "KP_Enter" => Some("EXE"),
            "BackSpace" => Some("DEL"),
            "Delete" => Some("AC"),
            "Escape" => Some("EXIT"),
            "Left" => Some("LEFT"),
            "Right" => Some("RIGHT"),
            "Up" => Some("UP"),
            "Down" => Some("DOWN"),
            "F1" | "F2" | "F3" | "F4" | "F5" | "F6" => Some(keysym),
            _ => None,
        };
        if let Some(k) = m {
            self.press(ui, k);
            return true;
        }
        let c0 = ch.chars().next();
        if ch.chars().count() == 1 && c0.is_some_and(|c| c.is_numeric() && c.to_digit(10).is_some()) {
            self.press(ui, ch);
            return true;
        }
        if ch == "!" {
            self.shift = true;
            self.press(ui, "INV");
            return true;
        }
        let cm = match ch {
            "+" => Some("ADD"), "-" => Some("SUB"), "*" => Some("MUL"), "/" => Some("DIV"), "^" => Some("POW"), "(" => Some("LP"),
            ")" => Some("RP"), "," => Some("COMMA"), "." => Some("DOT"), "x" => Some("XT"), _ => None,
        };
        if let Some(k) = cm {
            self.press(ui, k);
            return true;
        }
        if ch == "p" {
            self.shift = true;
            self.press(ui, "EXP");
            return true;
        }
        if ch == "=" && self.mode == "eqn" && self.eq_sub.as_deref() == Some("solver") {
            self.insert("=");
            self.draw(ui);
            return true;
        }
        if let Some(c) = c0.filter(|c| ch.chars().count() == 1 && c.is_alphabetic()) {
            let up: String = c.to_uppercase().collect();
            if up.chars().count() == 1 && "ABCDEFGHIJKLMNOPQRSTUVWYZ".contains(up.as_str()) && self.mode == "run" {
                self.insert(&up);
                self.draw(ui);
                return true;
            }
        }
        false
    }

    fn insert(&mut self, s: &str) {
        self.inp.insert(self.cur, s.to_string());
        self.cur += 1;
    }

    /// press(kid): the key's work (an error is only logged, as zenko.py does), then the screen drawn
    pub fn press(&mut self, ui: &mut Ui, kid: &str) {
        let _ = self.press_inner(kid);
        self.draw(ui);
    }

    /// the key's work without drawing (Err: what zenko.py's press() would log)
    pub fn press_inner(&mut self, kid: &str) -> R<()> {
        let (sh, al) = (self.shift, self.alpha);
        if kid == "SHIFT" {
            self.shift = !self.shift;
            self.alpha = false;
            return Ok(());
        }
        if kid == "ALPHA" {
            self.alpha = !self.alpha;
            self.shift = false;
            return Ok(());
        }
        self.shift = false;
        self.alpha = false;
        if sh && kid == "AC" {
            self.close = true;
            return Ok(());
        }
        if sh && kid == "MENU" {
            self.deg = !self.deg;
            self.save();
            return Ok(());
        }
        if kid == "MENU" {
            self.mode = "menu".into();
            return Ok(());
        }
        if self.mode == "menu" {
            let pick = match kid {
                "1" | "F1" => Some("run"), "2" | "F2" => Some("stat"), "3" | "F3" => Some("graph"), "4" | "F4" => Some("eqn"),
                "5" | "F5" => Some("dist"), _ => None,
            };
            if let Some(p) = pick {
                self.open_mode(p);
            } else if kid == "EXIT" || kid == "EXE" {
                self.mode = "run".into();
            }
            return Ok(());
        }
        let text = if al && alpha_of(kid).is_some() {
            alpha_of(kid)
        } else if sh && shifted(kid).is_some() {
            shifted(kid)
        } else if ins(kid).is_some() {
            ins(kid)
        } else if !kid.is_empty() && kid.chars().all(|c| c.is_ascii_digit()) {
            Some(kid.to_string())
        } else {
            None
        };
        match self.mode.clone().as_str() {
            "run" => self.key_run(kid, text),
            "graph" => self.key_graph(kid, text),
            "gview" => self.key_gview(kid),
            "eqn" => self.key_eqn(kid, text),
            "stat" => self.key_stat(kid, text),
            _ => self.key_dist(kid, text),
        }
    }

    fn open_mode(&mut self, m: &str) {
        self.mode = m.into();
        self.result = None;
        self.res_top = 0;
        match m {
            "graph" => self.edit_line(&self.ys[self.ysel].clone()),
            "run" => {
                self.inp.clear();
                self.cur = 0;
            }
            "eqn" => {
                self.grid = None;
                self.eq_sub = None;
            }
            "stat" => self.grid_stat(),
            "dist" => self.grid = None,
            _ => {}
        }
    }

    fn edit_line(&mut self, s: &str) {
        self.inp = cg_tokens(s);
        self.cur = self.inp.len();
    }

    fn edit_keys(&mut self, kid: &str, text: Option<String>) -> bool {
        if let Some(t) = text {
            self.insert(&t);
            return true;
        }
        match kid {
            "DEL" => {
                if self.cur > 0 {
                    self.inp.remove(self.cur - 1);
                    self.cur -= 1;
                }
                true
            }
            "LEFT" => {
                self.cur = self.cur.saturating_sub(1);
                true
            }
            "RIGHT" => {
                self.cur = self.inp.len().min(self.cur + 1);
                true
            }
            _ => false,
        }
    }

    fn joined(&self) -> String {
        self.inp.concat()
    }

    fn key_run(&mut self, kid: &str, text: Option<String>) -> R<()> {
        if kid == "AC" {
            self.inp.clear();
            self.cur = 0;
            return Ok(());
        }
        if kid == "F1" {
            self.deg = !self.deg;
            self.save();
            return Ok(());
        }
        if kid == "SD" && text.is_none() {
            if !self.hist.is_empty() && self.inp.is_empty() {
                self.exact = !self.exact;
            }
            return Ok(());
        }
        if kid == "F2" {
            self.exact = !self.exact;
            return Ok(());
        }
        if let Some(m) = match kid { "F3" => Some("graph"), "F4" => Some("eqn"), "F5" => Some("stat"), "F6" => Some("dist"), _ => None } {
            self.open_mode(m);
            return Ok(());
        }
        if kid == "UP" && !self.hist.is_empty() && self.inp.is_empty() {
            let s = self.hist.last().unwrap().0.clone();
            self.edit_line(&s);
            return Ok(());
        }
        if kid == "EXE" {
            let mut s = self.joined().trim().to_string();
            if s.is_empty() {
                match self.hist.last() {
                    Some(h) => s = h.0.clone(),
                    None => return Ok(()),
                }
            }
            let mut target: Option<String> = None;
            if let Some((a, b)) = s.clone().split_once('\u{2192}') {
                s = a.to_string();
                target = Some(b.trim().to_uppercase().chars().take(1).collect());
            }
            match cg_eval(&s, self.ans, self.deg, None, Some(&self.mem)) {
                Ok(v) => {
                    self.ans = v;
                    let t = target.clone().unwrap_or_default();
                    if !t.is_empty() && t.chars().all(|c| c.is_alphabetic()) {
                        self.mem.insert(t.clone(), v);
                    }
                    let label = if t.is_empty() { s.clone() } else { format!("{s}\u{2192}{t}") };
                    self.hist.push((label, Hv::F(v)));
                }
                Err(CgErr::Calc(m)) => self.hist.push((s.clone(), Hv::S(m.into()))),
                Err(e) => return Err(e),
            }
            let n = self.hist.len();
            if n > 12 {
                self.hist.drain(..n - 12);
            }
            self.inp.clear();
            self.cur = 0;
            self.exact = true;
            self.save();
            return Ok(());
        }
        let ops = ["+", "\u{2212}", "\u{d7}", "\u{f7}", "^", "\u{b2}", "\u{207b}\u{b9}", "!"];
        if self.inp.is_empty() && text.as_deref().is_some_and(|t| ops.contains(&t)) && !self.hist.is_empty() {
            self.inp = vec!["Ans".into()];
            self.cur = 1;
        }
        self.edit_keys(kid, text);
        Ok(())
    }

    fn key_graph(&mut self, kid: &str, text: Option<String>) -> R<()> {
        match kid {
            "UP" | "DOWN" => {
                self.ys[self.ysel] = self.joined();
                self.ysel = (self.ysel + if kid == "DOWN" { 1 } else { 2 }) % 3;
                let s = self.ys[self.ysel].clone();
                self.edit_line(&s);
            }
            "EXE" => {
                self.ys[self.ysel] = self.joined();
                self.ysel = (self.ysel + 1) % 3;
                let s = self.ys[self.ysel].clone();
                self.edit_line(&s);
                self.save();
            }
            "AC" => {
                self.inp.clear();
                self.cur = 0;
            }
            "F2" => {
                self.inp.clear();
                self.cur = 0;
                self.ys[self.ysel] = String::new();
                self.save();
            }
            "F3" => {
                let vs = views();
                let cur = vs.iter().position(|v| v.1 == self.view).map(|i| i as i64).unwrap_or(-1);
                let nx = ((cur + 1) as usize) % vs.len();
                self.view = vs[nx].1;
                self.gmsg = format!("V-Window: {}", vs[nx].0);
            }
            "F6" => {
                self.ys[self.ysel] = self.joined();
                self.save();
                self.mode = "gview".into();
                self.trace = None;
                self.gmsg = String::new();
            }
            "EXIT" => self.open_mode("run"),
            _ => {
                self.edit_keys(kid, text);
            }
        }
        Ok(())
    }

    /// fy(i, x): graphs always use radians; a CalcError is a gap
    pub fn fy(&self, i: usize, x: f64) -> R<Option<f64>> {
        match cg_eval(&self.ys[i], self.ans, false, Some(x), Some(&self.mem)) {
            Ok(v) => Ok(Some(v)),
            Err(CgErr::Calc(_)) => Ok(None),
            Err(e) => Err(e),
        }
    }

    fn key_gview(&mut self, kid: &str) -> R<()> {
        let [xmin, xmax, ymin, ymax] = self.view;
        if kid == "EXIT" {
            self.mode = "graph".into();
            return Ok(());
        }
        if kid == "ADD" || kid == "SUB" {
            let f = if kid == "ADD" { 0.5 } else { 2.0 };
            let (cx, cy) = ((xmin + xmax) / 2.0, (ymin + ymax) / 2.0);
            self.view = [cx - (cx - xmin) * f, cx + (xmax - cx) * f, cy - (cy - ymin) * f, cy + (ymax - cy) * f];
            return Ok(());
        }
        let active: Vec<usize> = (0..3).filter(|&i| !self.ys[i].trim().is_empty()).collect();
        if active.is_empty() {
            self.gmsg = "Type a function in Y1 first".into();
            return Ok(());
        }
        if kid == "LEFT" || kid == "RIGHT" {
            let tr = self.trace.get_or_insert((active[0], (xmin + xmax) / 2.0));
            let step = (xmax - xmin) / 94.0;
            if kid == "RIGHT" { tr.1 += step } else { tr.1 -= step }
            return Ok(());
        }
        if (kid == "UP" || kid == "DOWN") && self.trace.is_some() {
            let t0 = self.trace.unwrap().0;
            let k = active.iter().position(|&a| a == t0).unwrap_or(0) as i64;
            let n = active.len() as i64;
            let nk = (k + if kid == "DOWN" { 1 } else { -1 }).rem_euclid(n) as usize;
            self.trace.as_mut().unwrap().0 = active[nk];
            return Ok(());
        }
        if kid == "F6" {
            self.trace = if self.trace.is_some() { None } else { Some((active[0], (xmin + xmax) / 2.0)) };
            return Ok(());
        }
        let i = self.trace.map(|t| t.0).unwrap_or(active[0]);
        let Some(pts) = self.solve_points(kid, i, &active)? else { return Ok(()) };
        if pts.is_empty() {
            self.gmsg = "Not found in this window".into();
            return Ok(());
        }
        let (x, y) = match &self.gs {
            Some((g, k)) if g == kid => pts[(k + 1) % pts.len()],
            _ => pts[0],
        };
        let k = pts.iter().position(|p| p.0 == x && p.1 == y).unwrap_or(0);
        self.gs = Some((kid.to_string(), k));
        self.trace = Some((i, x));
        let name = match kid { "F1" => "ROOT", "F2" => "MAX", "F3" => "MIN", "F4" => "Y-ICEPT", _ => "ISCT" };
        let Some(y) = y else { return Err(CgErr::Py("TypeError")) };
        self.gmsg = format!("{name}  x={}  y={}   ({}/{})", calc_fmt(py_round(x, 10), false)?, calc_fmt(py_round(y, 10), false)?,
                            k + 1, pts.len());
        Ok(())
    }

    #[allow(clippy::type_complexity)]
    fn solve_points(&mut self, kid: &str, i: usize, active: &[usize]) -> R<Option<Vec<(f64, Option<f64>)>>> {
        let (xmin, xmax) = (self.view[0], self.view[1]);
        let n = 600;
        let xs: Vec<f64> = (0..=n).map(|k| xmin + (xmax - xmin) * k as f64 / n as f64).collect();
        if kid == "F4" {
            let y = self.fy(i, 0.0)?;
            return Ok(Some(if y.is_some() && xmin <= 0.0 && 0.0 <= xmax { vec![(0.0, y)] } else { vec![] }));
        }
        if kid == "F5" {
            if active.len() < 2 {
                self.gmsg = "ISCT needs two graphs".into();
                return Ok(None);
            }
            let j = *active.iter().find(|&&a| a != i).unwrap();
            let g = |x: f64| -> R<Option<f64>> {
                let a = self.fy(i, x)?;
                let b = self.fy(j, x)?;
                Ok(match (a, b) { (Some(a), Some(b)) => Some(a - b), _ => None })
            };
            let rs = roots(&g, &xs)?;
            let mut out = vec![];
            for x in rs {
                out.push((x, self.fy(i, x)?));
            }
            return Ok(Some(out));
        }
        if kid == "F1" {
            let f = |x: f64| self.fy(i, x);
            return Ok(Some(roots(&f, &xs)?.into_iter().map(|x| (x, Some(0.0))).collect()));
        }
        if kid == "F2" || kid == "F3" {
            let sgn = if kid == "F2" { 1.0 } else { -1.0 };
            let mut out = vec![];
            let mut ys = vec![];
            for &x in &xs {
                ys.push(self.fy(i, x)?);
            }
            for k in 1..n {
                let (Some(a), Some(b), Some(c)) = (ys[k - 1], ys[k], ys[k + 1]) else { continue };
                if sgn * b >= sgn * a && sgn * b > sgn * c {
                    let (mut lo, mut hi) = (xs[k - 1], xs[k + 1]);
                    for _ in 0..80 {
                        let (m1, m2) = (lo + (hi - lo) / 3.0, hi - (hi - lo) / 3.0);
                        let (Some(f1), Some(f2)) = (self.fy(i, m1)?, self.fy(i, m2)?) else { break };
                        if sgn * f1 < sgn * f2 {
                            lo = m1;
                        } else {
                            hi = m2;
                        }
                    }
                    let x = (lo + hi) / 2.0;
                    if let Some(y) = self.fy(i, x)? {
                        out.push((x, Some(y)));
                    }
                }
            }
            return Ok(Some(out));
        }
        Ok(None)
    }

    fn make_grid(&mut self, kind: &str, labels: Vec<String>, rows: Vec<Vec<Option<f64>>>, title: &str) {
        self.grid = Some(CGrid { kind: kind.into(), labels, rows, r: 0, c: 0, title: title.into(), top: 0, names: None, which: String::new() });
        self.inp.clear();
        self.cur = 0;
        self.result = None;
    }

    fn grid_commit(&mut self) -> R<()> {
        let s = self.joined().trim().to_string();
        if !s.is_empty() {
            match cg_eval(&s, self.ans, self.deg, None, Some(&self.mem)) {
                Ok(v) => {
                    let g = self.grid.as_mut().unwrap();
                    g.rows[g.r][g.c] = Some(v);
                }
                Err(CgErr::Calc(m)) => self.gmsg = m.into(),
                Err(e) => return Err(e),
            }
        }
        self.inp.clear();
        self.cur = 0;
        Ok(())
    }

    fn grid_keys(&mut self, kid: &str, text: Option<String>) -> R<bool> {
        let mut kid = kid.to_string();
        let nlab = self.grid.as_ref().unwrap().labels.len();
        if ["UP", "DOWN", "LEFT", "RIGHT", "EXE"].contains(&kid.as_str()) && !self.inp.is_empty() {
            self.grid_commit()?;
            if kid == "EXE" {
                kid = if self.grid.as_ref().unwrap().c < nlab - 1 { "RIGHT".into() } else { "NEXT".into() };
            }
        }
        if kid == "EXE" {
            kid = if self.grid.as_ref().unwrap().c < nlab - 1 { "RIGHT".into() } else { "NEXT".into() };
        }
        let has_inp = !self.inp.is_empty();
        let g = self.grid.as_mut().unwrap();
        match kid.as_str() {
            "LEFT" => g.c = g.c.saturating_sub(1),
            "RIGHT" => g.c = (nlab - 1).min(g.c + 1),
            "DOWN" | "NEXT" => {
                if kid == "NEXT" {
                    g.c = 0;
                }
                if g.r + 1 < g.rows.len() {
                    g.r += 1;
                } else if g.kind == "stat" {
                    g.rows.push(vec![None, None]);
                    g.r += 1;
                }
            }
            "UP" => g.r = g.r.saturating_sub(1),
            "DEL" if !has_inp => g.rows[g.r][g.c] = None,
            "AC" => {
                self.inp.clear();
                self.cur = 0;
            }
            _ => {
                if text.is_some() || kid == "DEL" {
                    self.edit_keys(&kid, text);
                } else {
                    return Ok(false);
                }
            }
        }
        let g = self.grid.as_mut().unwrap();
        g.top = 0.max(g.top.min(g.r)).max(g.r.saturating_sub(5));
        Ok(true)
    }

    fn key_eqn(&mut self, kid: &str, text: Option<String>) -> R<()> {
        let sub = self.eq_sub.clone();
        if let Some(res) = &self.result {
            if ["EXIT", "EXE", "AC"].contains(&kid) {
                self.result = None;
            } else if kid == "UP" || kid == "DOWN" {
                let n = res.len() as i64;
                self.res_top = 0.max((n - 1).min(self.res_top as i64 + if kid == "DOWN" { 1 } else { -1 })) as usize;
            }
            return Ok(());
        }
        if sub.as_deref() == Some("solver") {
            return self.key_solver(kid, text);
        }
        if self.grid.is_none() {
            let Some(sub) = sub else {
                let pick = match kid { "F1" | "1" => Some("simul"), "F2" | "2" => Some("poly"), "F3" | "3" => Some("solver"), _ => None };
                if kid == "EXIT" {
                    self.open_mode("run");
                    return Ok(());
                }
                if pick == Some("solver") {
                    self.eq_sub = Some("solver".into());
                    let s = self.life_cg["solver"].as_str().unwrap_or("").to_string();
                    self.edit_line(&s);
                } else if let Some(p) = pick {
                    self.eq_sub = Some(p.into());
                }
                return Ok(());
            };
            if kid == "EXIT" {
                self.eq_sub = None;
                return Ok(());
            }
            let n = match kid { "F1" | "2" => Some(2), "F2" | "3" => Some(3), "F3" | "4" => Some(4), "F4" | "5" => Some(5), "F5" | "6" => Some(6), _ => None };
            if let Some(n) = n {
                if sub == "simul" {
                    let mut labels: Vec<String> = if n <= 3 { ["x", "y", "z"].iter().map(|s| s.to_string()).collect() } else { (0..n).map(|i| format!("x{}", i + 1)).collect() };
                    labels.truncate(n);
                    labels.push("=".into());
                    self.make_grid("simul", labels, vec![vec![None; n + 1]; n], &format!("SIMUL  {n} unknowns"));
                } else {
                    let labels: Vec<String> = "abcdefg".chars().take(n + 1).map(|c| c.to_string()).collect();
                    self.make_grid("poly", labels, vec![vec![None; n + 1]], &format!("POLY  degree {n}   ({})", crate::bible::poly_form(n)));
                }
            }
            return Ok(());
        }
        if kid == "EXIT" {
            self.grid = None;
            return Ok(());
        }
        if kid == "F1" || kid == "F6" {
            if !self.inp.is_empty() {
                self.grid_commit()?;
            }
            return self.solve_eqn();
        }
        if kid == "F3" {
            for row in &mut self.grid.as_mut().unwrap().rows {
                row.iter_mut().for_each(|v| *v = None);
            }
            return Ok(());
        }
        self.grid_keys(kid, text)?;
        Ok(())
    }

    fn key_solver(&mut self, kid: &str, text: Option<String>) -> R<()> {
        match kid {
            "EXIT" => {
                self.eq_sub = None;
                self.inp.clear();
                self.cur = 0;
            }
            "AC" => {
                self.inp.clear();
                self.cur = 0;
            }
            "EXE" | "F1" | "F6" => {
                let eq = self.joined().trim().to_string();
                if eq.is_empty() {
                    return Ok(());
                }
                if !self.life_cg.is_object() {
                    self.life_cg = json!({});
                }
                self.life_cg["solver"] = json!(eq);
                self.dirty = true;
                self.result = Some(vec![("Solving...".into(), String::new())]);
                self.res_top = 0;
                let res = cg_solve(&eq, self.ans, self.deg, Some(&self.mem.clone()), -1000.0, 1000.0, 12);
                self.solve_done(res);
            }
            "F3" => self.insert("="),
            _ => {
                self.edit_keys(kid, text);
            }
        }
        Ok(())
    }

    /// the solver's job came back
    pub fn solve_done(&mut self, res: R<Vec<f64>>) {
        if self.eq_sub.as_deref() != Some("solver") || self.result != Some(vec![("Solving...".into(), String::new())]) {
            return;
        }
        match res {
            Err(e) => self.result = Some(vec![(if matches!(e, CgErr::Calc(_)) { msg(&e) } else { "Syntax ERROR".into() }, String::new())]),
            Ok(r) if r.is_empty() => self.result = Some(vec![("No real solution".into(), String::new()),
                                                          ("(searched".into(), "\u{2212}1000 \u{2264} x \u{2264} 1000)".into())]),
            Ok(r) => {
                let many = r.len() > 1;
                let mut out = vec![];
                for (i, v) in r.iter().enumerate() {
                    let Ok(s) = calc_fmt(*v, true) else { return };
                    out.push((format!("x{} =", if many { (i + 1).to_string() } else { String::new() }), s));
                }
                self.ans = r[0];
                out.push(("Lft \u{2212} Rgt".into(), "0".into()));
                self.result = Some(out);
            }
        }
    }

    fn solve_eqn(&mut self) -> R<()> {
        let g = self.grid.clone().unwrap();
        let vals: Vec<Vec<f64>> = g.rows.iter().map(|r| r.iter().map(|v| v.unwrap_or(0.0)).collect()).collect();
        let r: R<Vec<(String, String)>> = (|| {
            if g.kind == "simul" {
                let sol = calc_solve_linear(&vals)?;
                let mut out = vec![];
                for (i, v) in sol.iter().enumerate() {
                    out.push((format!("{} =", g.labels[i]), calc_fmt(*v, self.exact)?));
                }
                Ok(out)
            } else {
                let co = &vals[0];
                let mut rts = calc_roots_poly(co)?;
                rts.sort_by(|a, b| {
                    let ka = (a.1.abs() > 1e-10, -a.0, a.1);
                    let kb = (b.1.abs() > 1e-10, -b.0, b.1);
                    ka.0.cmp(&kb.0).then(ka.1.partial_cmp(&kb.1).unwrap_or(std::cmp::Ordering::Equal))
                        .then(ka.2.partial_cmp(&kb.2).unwrap_or(std::cmp::Ordering::Equal))
                });
                let mut res = vec![];
                for (i, r) in rts.iter().enumerate() {
                    if r.1.abs() < 1e-10 {
                        res.push((format!("x{} =", i + 1), calc_fmt(r.0, true)?));
                    } else {
                        res.push((format!("x{} =", i + 1), format!("{} {} {}i", calc_fmt(py_round(r.0, 10), false)?,
                                                                   if r.1 > 0.0 { "+" } else { "\u{2212}" }, calc_fmt(py_round(r.1.abs(), 10), false)?)));
                    }
                }
                if co.len() == 3 && co[0] != 0.0 {
                    let (a, b, c) = (co[0], co[1], co[2]);
                    res.push(("\u{394} =".into(), calc_fmt(b * b - 4.0 * a * c, true)?));
                    let xv = -b / (2.0 * a);
                    res.push(("vertex".into(), format!("({}, {})", calc_fmt(xv, true)?, calc_fmt(a * xv * xv + b * xv + c, true)?)));
                }
                Ok(res)
            }
        })();
        match r {
            Ok(v) => self.result = Some(v),
            Err(CgErr::Calc(m)) => self.result = Some(vec![(m.into(), String::new())]),
            Err(e) => return Err(e),
        }
        self.res_top = 0;
        Ok(())
    }

    fn grid_stat(&mut self) {
        let [l1, l2] = self.lists.clone();
        let n = l1.len().max(l2.len()).max(1);
        let rows = (0..n).map(|i| vec![l1.get(i).copied(), l2.get(i).copied()]).collect();
        self.make_grid("stat", vec!["List 1".into(), "List 2".into()], rows, "STAT");
    }

    fn stat_lists(&mut self) -> (Vec<f64>, Vec<f64>, Vec<(f64, f64)>) {
        let rows = &self.grid.as_ref().unwrap().rows;
        let l1: Vec<f64> = rows.iter().filter_map(|r| r[0]).collect();
        let l2: Vec<f64> = rows.iter().filter_map(|r| r[1]).collect();
        let pairs: Vec<(f64, f64)> = rows.iter().filter_map(|r| Some((r[0]?, r[1]?))).collect();
        self.lists = [l1.clone(), l2.clone()];
        self.save();
        (l1, l2, pairs)
    }

    fn key_stat(&mut self, kid: &str, text: Option<String>) -> R<()> {
        if let Some(res) = &self.result {
            if ["EXIT", "EXE", "AC"].contains(&kid) {
                self.result = None;
            } else if kid == "UP" || kid == "DOWN" {
                let n = res.len() as i64;
                self.res_top = 0.max((n - 1).min(self.res_top as i64 + if kid == "DOWN" { 1 } else { -1 })) as usize;
            }
            return Ok(());
        }
        if kid == "EXIT" {
            self.stat_lists();
            self.open_mode("run");
            return Ok(());
        }
        if kid == "F1" || kid == "F2" {
            if !self.inp.is_empty() {
                self.grid_commit()?;
            }
            let (l1, _l2, pairs) = self.stat_lists();
            let r: R<Vec<(String, String)>> = (|| {
                let mut out = vec![];
                if kid == "F1" {
                    for (k, v) in calc_stats1(&l1)? {
                        out.push((k.to_string(), if k == "n" { format!("{}", v as i64) } else { calc_fmt(v, false)? }));
                    }
                } else {
                    let xs: Vec<f64> = pairs.iter().map(|p| p.0).collect();
                    let ys: Vec<f64> = pairs.iter().map(|p| p.1).collect();
                    for (k, v) in calc_linreg(&xs, &ys)? {
                        out.push((k.to_string(), match v { Sv::F(f) => calc_fmt(f, false)?, Sv::S(s) => s.to_string() }));
                    }
                }
                Ok(out)
            })();
            match r {
                Ok(v) => self.result = Some(v),
                Err(CgErr::Calc(m)) => self.result = Some(vec![(m.into(), String::new())]),
                Err(e) => return Err(e),
            }
            self.res_top = 0;
            return Ok(());
        }
        if kid == "F3" {
            let g = self.grid.as_mut().unwrap();
            if g.rows.len() > 1 {
                g.rows.remove(g.r);
                g.r = g.r.min(g.rows.len() - 1);
            } else {
                g.rows[0] = vec![None, None];
            }
            return Ok(());
        }
        if kid == "F4" {
            let g = self.grid.as_mut().unwrap();
            g.rows = vec![vec![None, None]];
            g.r = 0;
            g.c = 0;
            return Ok(());
        }
        self.grid_keys(kid, text)?;
        Ok(())
    }

    fn key_dist(&mut self, kid: &str, text: Option<String>) -> R<()> {
        if self.result.is_some() {
            if ["EXIT", "EXE", "AC"].contains(&kid) {
                self.result = None;
            }
            return Ok(());
        }
        if self.grid.is_none() {
            let k = if ["1", "2", "3", "4"].contains(&kid) { format!("F{kid}") } else { kid.to_string() };
            if let Some((w, name, labels, defaults)) = DIST_NAMES.iter().find(|d| d.0 == k) {
                self.make_grid("dist", vec!["value".into()], defaults.iter().map(|v| vec![Some(*v)]).collect(), name);
                let g = self.grid.as_mut().unwrap();
                g.names = Some(labels.iter().map(|s| s.to_string()).collect());
                g.which = w.to_string();
            } else if k == "EXIT" {
                self.open_mode("run");
            }
            return Ok(());
        }
        if kid == "EXIT" {
            self.grid = None;
            return Ok(());
        }
        let last_row = { let g = self.grid.as_ref().unwrap(); g.r + 1 == g.rows.len() };
        if kid == "F1" || (kid == "EXE" && self.inp.is_empty() && last_row) {
            if !self.inp.is_empty() {
                self.grid_commit()?;
            }
            let g = self.grid.as_ref().unwrap();
            let v: Vec<f64> = g.rows.iter().map(|r| r[0].unwrap_or(0.0)).collect();
            let w = g.which.clone();
            let r: R<Vec<(String, String)>> = (|| {
                Ok(match w.as_str() {
                    "F1" => {
                        let p = calc_normcd(v[0], v[1], v[2], v[3])?;
                        let zl = if v[0] > -1e98 { (v[0] - v[3]) / v[2] } else { f64::NEG_INFINITY };
                        let zu = if v[1] < 1e98 { (v[1] - v[3]) / v[2] } else { f64::INFINITY };
                        vec![("p =".into(), calc_fmt(p, false)?),
                             ("z:Low =".into(), if zl == f64::NEG_INFINITY { "-\u{221e}".into() } else { calc_fmt(zl, false)? }),
                             ("z:Up =".into(), if zu == f64::INFINITY { "\u{221e}".into() } else { calc_fmt(zu, false)? })]
                    }
                    "F2" => vec![("x =".into(), calc_fmt(calc_invnorm(v[0], v[1], v[2], "left")?.0, false)?)],
                    "F3" => vec![("p =".into(), calc_fmt(calc_binpd(v[0], if v.len() > 3 { v[2] } else { v[1] }, v[v.len() - 1])?, false)?)],
                    _ => vec![("p =".into(), calc_fmt(calc_bincd(v[1], v[2], v[3], v[0])?, false)?)],
                })
            })();
            match r {
                Ok(x) => self.result = Some(x),
                Err(CgErr::Calc(m)) => self.result = Some(vec![(m.into(), String::new())]),
                Err(e) => return Err(e),
            }
            return Ok(());
        }
        self.grid_keys(kid, text)?;
        Ok(())
    }

    /// tick(): the cursor blinks every 530 ms; true when the screen is drawn again
    pub fn tick(&mut self, ui: &mut Ui) -> bool {
        self.blink = !self.blink;
        let m = self.mode.as_str();
        if m == "run" || m == "graph" || (self.grid.is_some() && ["eqn", "stat", "dist"].contains(&m))
            || (m == "eqn" && self.eq_sub.as_deref() == Some("solver") && self.result.is_none()) {
            self.draw(ui);
            return true;
        }
        false
    }

    pub fn save(&mut self) {
        if !self.life_cg.is_object() {
            self.life_cg = json!({});
        }
        let st = self.life_cg.as_object_mut().unwrap();
        let n = self.hist.len();
        st.insert("deg".into(), json!(self.deg));
        st.insert("mem".into(), json!(self.mem));
        st.insert("ans".into(), json!(self.ans));
        st.insert("hist".into(), Value::Array(self.hist[n.saturating_sub(12)..].iter().map(|(a, b)| json!([a, hv_json(b)])).collect()));
        st.insert("ys".into(), json!(self.ys));
        st.insert("L1".into(), json!(self.lists[0]));
        st.insert("L2".into(), json!(self.lists[1]));
        self.dirty = true;
        self.aliased = true;
    }

    /// life["cg"] as zenko.py would write it now
    pub fn life_value(&self) -> Value {
        let mut v = self.life_cg.clone();
        if self.aliased {
            v["ys"] = json!(self.ys);
            v["mem"] = json!(self.mem);
            v["L1"] = json!(self.lists[0]);
            v["L2"] = json!(self.lists[1]);
        }
        v
    }

    // ---- drawing ------------------------------------------------------------------------------------------------
    fn f(&self, size: i32, bold: bool) -> Font {
        Font::new(&self.fu, size, bold)
    }

    pub fn draw(&self, ui: &mut Ui) {
        let cv = canvas_of(ui, self.cv);
        *cv = Default::default();
        let _ = self.draw_on(cv);
    }

    /// everything on the screen; Err where zenko.py's draw would stop with an exception (what was drawn stays)
    pub fn draw_on(&self, cv: &mut Canvas) -> R<()> {
        let (ink, dim) = ("#15171c", "#6b7080");
        cv.rect(0.0, 0.0, SW, 16.0).fill("#dfe3ea").outline("");
        let mode_name = match self.mode.as_str() {
            "run" => "Run-Matrix", "graph" | "gview" => "Graph", "eqn" => "Equation", "stat" => "Statistics", "dist" => "Distribution",
            _ => "Main Menu",
        };
        cv.text(4.0, 8.0, mode_name).anchor("w").fill(ink).font(self.f(8, true));
        let mut flags = format!("{}  Math  {}", if self.deg && self.mode != "graph" && self.mode != "gview" { "Deg" } else { "Rad" },
                                if self.exact { "d/c" } else { "Dec" });
        if self.shift {
            flags = format!("S  {flags}");
        }
        if self.alpha {
            flags = format!("A  {flags}");
        }
        cv.text(SW - 4.0, 8.0, &flags).anchor("e").fill(if !(self.shift || self.alpha) { "#3b6ea8" } else { "#c8743a" }).font(self.f(8, true));
        match self.mode.as_str() {
            "eqn" | "stat" | "dist" => self.draw_grid(cv, ink, dim),
            "menu" => {
                self.draw_menu(cv, ink, dim);
                Ok(())
            }
            "run" => self.draw_run(cv, ink, dim),
            "graph" => {
                self.draw_graph(cv, ink, dim);
                Ok(())
            }
            _ => self.draw_gview(cv, ink),
        }
    }

    fn softkeys(&self, cv: &mut Canvas, labels: &[&str]) {
        let w = SW / 6.0;
        for (i, lab) in labels.iter().enumerate() {
            if lab.is_empty() {
                continue;
            }
            let x = i as f64 * w;
            cv.rect(x + 2.0, SH - 15.0, x + w - 2.0, SH - 1.0).fill("#2b4f7f").outline("");
            cv.text(x + w / 2.0, SH - 8.0, lab).fill("#ffffff").font(self.f(7, true));
        }
    }

    fn input_line(&self, cv: &mut Canvas, y: f64, prefix: &str) {
        let s = self.inp[..self.cur].concat();
        let rest = self.inp[self.cur..].concat();
        let mut txt = format!("{prefix}{s}{}{rest}", if self.blink { "|" } else { " " });
        let n = txt.chars().count();
        if n > 46 {
            txt = format!("\u{2026}{}", txt.chars().skip(n - 45).collect::<String>());
        }
        cv.text(6.0, y, &txt).anchor("w").fill("#15171c").font(Font::new("Consolas", 12, false));
    }

    fn draw_menu(&self, cv: &mut Canvas, ink: &str, dim: &str) {
        let apps = [("1", "Run-Matrix", "#e0705a", "1+2"), ("2", "Statistics", "#5fae9e", "x\u{304} \u{3c3}"), ("3", "Graph", "#3b6ea8", "y=x\u{b2}"),
                    ("4", "Equation", "#c9a24a", "ax+b"), ("5", "Distribution", "#8f86d6", "N(\u{3bc},\u{3c3})")];
        for (i, (k, name, col, ic)) in apps.iter().enumerate() {
            let (x, y) = (14.0 + (i % 3) as f64 * 124.0, 26.0 + (i / 3) as f64 * 92.0);
            cv.rect(x, y, x + 110.0, y + 62.0).fill(col).outline("");
            cv.text(x + 55.0, y + 31.0, ic).fill("#ffffff").font(Font::new(&self.serif, 15, true));
            cv.text(x + 4.0, y + 4.0, k).anchor("nw").fill("#ffffff").font(self.f(8, true));
            cv.text(x + 55.0, y + 72.0, name).fill(ink).font(self.f(8, false));
        }
        cv.text(SW - 6.0, SH - 6.0, "press 1-5").anchor("se").fill(dim).font(self.f(7, false));
    }

    fn draw_run(&self, cv: &mut Canvas, ink: &str, dim: &str) -> R<()> {
        let mut y = SH - 34.0;
        self.input_line(cv, y, "");
        y -= 22.0;
        let last = self.hist.last().cloned();
        for (s, v) in self.hist.iter().rev() {
            if y < 26.0 {
                break;
            }
            let out = match v {
                Hv::S(m) => m.clone(),
                Hv::F(f) => calc_fmt(*f, if Some((s.clone(), v.clone())) == last { self.exact } else { true })?,
            };
            cv.text(SW - 6.0, y, &out).anchor("e").fill(if matches!(v, Hv::S(_)) { "#c0392b" } else { ink })
                .font(Font::new("Consolas", 12, true));
            y -= 16.0;
            cv.text(6.0, y, &clip(s, 44)).anchor("w").fill(dim).font(Font::new("Consolas", 10, false));
            y -= 18.0;
        }
        self.softkeys(cv, &["Deg/Rad", "S\u{21d4}D", "GRAPH", "EQUA", "STAT", "DIST"]);
        Ok(())
    }

    fn draw_graph(&self, cv: &mut Canvas, ink: &str, dim: &str) {
        let cols = ["#3b6ea8", "#c0392b", "#2e8b57"];
        for i in 0..3 {
            let y = 30.0 + i as f64 * 30.0;
            let sel = i == self.ysel;
            if sel {
                cv.rect(2.0, y - 12.0, SW - 2.0, y + 12.0).fill("#e3ebf6").outline("");
            }
            cv.text(8.0, y, &format!("Y{}=", i + 1)).anchor("w").fill(cols[i]).font(Font::new("Consolas", 12, true));
            if sel {
                self.input_line(cv, y, "    ");
            } else {
                cv.text(48.0, y, &clip(&self.ys[i], 40)).anchor("w").fill(ink).font(Font::new("Consolas", 12, false));
            }
        }
        let m = if self.gmsg.is_empty() { "Type with x (X,\u{3b8},T key). \u{25b2}\u{25bc} choose, F6 draws." } else { &self.gmsg };
        cv.text(6.0, SH - 26.0, m).anchor("w").fill(dim).font(self.f(8, false));
        self.softkeys(cv, &["", "DELETE", "V-WIN", "", "", "DRAW"]);
    }

    fn draw_gview(&self, cv: &mut Canvas, ink: &str) -> R<()> {
        let [xmin, xmax, ymin, ymax] = self.view;
        let (top, bot) = (17.0, SH - 17.0);
        let gh = bot - top;
        let sx = |x: f64| (x - xmin) / (xmax - xmin) * SW;
        let sy = |y: f64| top + (ymax - y) / (ymax - ymin) * gh;
        if xmin < 0.0 && 0.0 < xmax {
            cv.line(vec![sx(0.0), top, sx(0.0), bot]).fill("#9aa0ad");
        }
        if ymin < 0.0 && 0.0 < ymax {
            cv.line(vec![0.0, sy(0.0), SW, sy(0.0)]).fill("#9aa0ad");
        }
        let e = (1e-9f64.max((xmax - xmin) / 8.0)).log10().floor() as i32;
        let step = if e >= 0 { 10f64.powi(e) } else { 10f64.powf(e as f64) };
        let mut x = (xmin / step).ceil() * step;
        while x <= xmax && ymin < 0.0 && 0.0 < ymax {
            cv.line(vec![sx(x), sy(0.0) - 2.0, sx(x), sy(0.0) + 2.0]).fill("#9aa0ad");
            x += step;
        }
        let cols = ["#3b6ea8", "#c0392b", "#2e8b57"];
        for i in 0..3 {
            if self.ys[i].trim().is_empty() {
                continue;
            }
            let (mut pts, mut prev): (Vec<f64>, Option<f64>) = (vec![], None);
            for px in (0..=SW as i32).step_by(2) {
                let xv = xmin + (xmax - xmin) * px as f64 / SW;
                let yv = self.fy(i, xv)?;
                let Some(yv) = yv.filter(|y| y.abs() <= 1e6) else {
                    if pts.len() >= 4 {
                        cv.line(pts.clone()).fill(cols[i]).width(2.0);
                    }
                    pts.clear();
                    prev = None;
                    continue;
                };
                let yy = sy(yv);
                if let Some(p) = prev {
                    if (yy - p).abs() > gh * 1.5 {
                        if pts.len() >= 4 {
                            cv.line(pts.clone()).fill(cols[i]).width(2.0);
                        }
                        pts.clear();
                    }
                }
                pts.push(px as f64);
                pts.push((-gh).max((2.0 * SH).min(yy)));
                prev = Some(yy);
            }
            if pts.len() >= 4 {
                cv.line(pts).fill(cols[i]).width(2.0);
            }
        }
        if let Some((i, xv)) = self.trace {
            if let Some(yv) = self.fy(i, xv)? {
                let (xx, yy) = (sx(xv), sy(yv));
                cv.line(vec![xx - 6.0, yy, xx + 6.0, yy]).fill("#c8743a").width(2.0);
                cv.line(vec![xx, yy - 6.0, xx, yy + 6.0]).fill("#c8743a").width(2.0);
                if self.gmsg.is_empty() {
                    let s = format!("Y{}  x={}  y={}", i + 1, calc_fmt(py_round(xv, 8), false)?, calc_fmt(py_round(yv, 8), false)?);
                    cv.text(4.0, top + 2.0, &s).anchor("nw").fill(ink).font(Font::new("Consolas", 9, false));
                }
            }
        }
        if !self.gmsg.is_empty() {
            cv.rect(0.0, top, SW, top + 14.0).fill("#f6f7f2").outline("");
            cv.text(4.0, top + 1.0, &self.gmsg).anchor("nw").fill(ink).font(Font::new("Consolas", 9, false));
        }
        self.softkeys(cv, &["ROOT", "MAX", "MIN", "Y-ICEPT", "ISCT", "TRACE"]);
        Ok(())
    }

    fn draw_grid(&self, cv: &mut Canvas, ink: &str, dim: &str) -> R<()> {
        let solver = self.eq_sub.as_deref() == Some("solver");
        if let Some(res) = &self.result {
            let title = match &self.grid {
                Some(g) => g.title.clone(),
                None if solver => format!("SOLVER   {}", self.joined()),
                None => String::new(),
            };
            cv.text(6.0, 26.0, &title).anchor("w").fill(dim).font(self.f(8, true));
            let mut y = 46.0;
            for (k, v) in res.iter().skip(self.res_top).take(9) {
                cv.text(10.0, y, k).anchor("w").fill(ink).font(Font::new("Consolas", 12, true));
                cv.text(SW - 10.0, y, v).anchor("e").fill(ink).font(Font::new("Consolas", 12, false));
                y += 19.0;
            }
            self.softkeys(cv, &["", "", "", "", "", ""]);
            cv.text(SW - 6.0, SH - 24.0, "EXE / EXIT to go back").anchor("e").fill(dim).font(self.f(7, false));
            return Ok(());
        }
        if self.mode == "eqn" && solver {
            cv.text(6.0, 26.0, "SOLVER  \u{b7}  type an equation in x, then EXE").anchor("w").fill(dim).font(self.f(8, true));
            cv.text(10.0, 50.0, "Eq:").anchor("w").fill("#3b6ea8").font(self.f(9, true));
            self.input_line(cv, 72.0, "");
            for (k, ln) in ["e.g.  x^2=2^x    sin(x)=0.5    x\u{b3}\u{2212}2x\u{2212}5", "No '=' means '= 0'.  SHIFT then . types '='",
                            "Finds every real root from \u{2212}1000 to 1000"].iter().enumerate() {
                cv.text(10.0, 104.0 + k as f64 * 18.0, ln).anchor("w").fill(dim).font(self.f(8, false));
            }
            self.softkeys(cv, &["SOLVE", "", "=", "", "", ""]);
            return Ok(());
        }
        let sub = self.eq_sub.as_deref();
        if self.mode == "eqn" && self.grid.is_none() && (sub == Some("simul") || sub == Some("poly")) {
            let simul = sub == Some("simul");
            cv.text(6.0, 26.0, if simul { "SIMULTANEOUS: how many unknowns?" } else { "POLYNOMIAL: what degree?" }).anchor("w").fill(dim)
                .font(self.f(8, true));
            for (k, n) in (2..7).enumerate() {
                let ln = if simul { format!("F{}  {n} unknowns", k + 1) } else { format!("F{}  {}", k + 1, crate::bible::poly_form(n)) };
                cv.text(10.0, 46.0 + k as f64 * 22.0, &ln).anchor("w").fill(ink).font(self.f(10, false));
            }
            self.softkeys(cv, &["2", "3", "4", "5", "6", ""]);
            return Ok(());
        }
        let Some(g) = &self.grid else {
            let (menus, lines): (&[&str], &[&str]) = if self.mode == "eqn" {
                (&["SIMUL", "POLY", "SOLVER", "", "", ""], &["F1  Simultaneous equations  (2 to 6 unknowns)", "F2  Polynomial  (degree 2 to 6)",
                                                            "F3  Solver  (any equation, e.g. x^2=2^x)", "Then fill the table and press F1 SOLVE"])
            } else {
                (&["NORM-CD", "INV-N", "BIN-PD", "BIN-CD", "", ""], &["F1  Normal C.D  P(a < X < b)", "F2  Inverse Normal  (left tail)",
                                                                      "F3  Binomial P.D  P(X = x)", "F4  Binomial C.D  P(a \u{2264} X \u{2264} b)"])
            };
            for (k, ln) in lines.iter().enumerate() {
                cv.text(10.0, 34.0 + k as f64 * 24.0, ln).anchor("w").fill(ink).font(self.f(10, false));
            }
            self.softkeys(cv, menus);
            return Ok(());
        };
        cv.text(6.0, 26.0, &g.title).anchor("w").fill(dim).font(self.f(8, true));
        let names = g.names.as_ref();
        let ncol = g.labels.len();
        let x0 = if names.is_some() { 120.0 } else { 34.0 };
        let cw = (SW - x0 - 6.0) / ncol as f64;
        for (c, lab) in g.labels.iter().enumerate() {
            if names.is_none() {
                cv.text(x0 + c as f64 * cw + cw / 2.0, 40.0, lab).fill("#3b6ea8").font(self.f(9, true));
            }
        }
        for (k, r) in (g.top..g.rows.len().min(g.top + 6)).enumerate() {
            let y = 58.0 + k as f64 * 22.0;
            let lab = match names { Some(n) => n[r].clone(), None => (r + 1).to_string() };
            cv.text(if names.is_some() { 6.0 } else { 24.0 }, y, &lab).anchor(if names.is_some() { "w" } else { "e" }).fill(dim).font(self.f(9, false));
            for c in 0..ncol {
                let x = x0 + c as f64 * cw;
                let sel = r == g.r && c == g.c;
                cv.rect(x + 1.0, y - 10.0, x + cw - 1.0, y + 10.0).fill(if sel { "#e3ebf6" } else { "#ffffff" })
                    .outline(if sel { "#3b6ea8" } else { "#d3d7df" });
                let v = g.rows[r][c];
                let s = if sel && !self.inp.is_empty() {
                    format!("{}{}", self.joined(), if self.blink { "|" } else { "" })
                } else {
                    match v {
                        None => String::new(),
                        Some(v) if v <= -1e98 => "-\u{221e}".into(),
                        Some(v) if v >= 1e98 => "\u{221e}".into(),
                        Some(v) => calc_fmt(v, false)?,
                    }
                };
                cv.text(x + cw - 4.0, y, &clip(&s, 4.max((cw / 8.0) as i64) as usize)).anchor("e").fill(ink)
                    .font(Font::new("Consolas", if ncol <= 5 { 11 } else { 9 }, false));
            }
        }
        match self.mode.as_str() {
            "eqn" => self.softkeys(cv, &["SOLVE", "", "CLEAR", "", "", ""]),
            "stat" => self.softkeys(cv, &["1-VAR", "2-VAR", "DEL", "DEL-ALL", "", ""]),
            _ => self.softkeys(cv, &["CALC", "", "", "", "", ""]),
        }
        Ok(())
    }

    pub fn fit(&mut self, ui: &mut Ui, fonts: &dyn Fonts, w: i32, h: i32) {
        ui.layout_at(fonts, w, h);
    }

    /// the state as the golden script records it
    pub fn hist_json(&self) -> Value {
        Value::Array(self.hist.iter().map(|(a, b)| json!([a, hv_json(b)])).collect())
    }
}

/// CasioWindow.roots(g, xs): every sign change (or exact zero) refined by bisection
fn roots(g: &dyn Fn(f64) -> R<Option<f64>>, xs: &[f64]) -> R<Vec<f64>> {
    let mut out = vec![];
    let mut prev: Option<(f64, f64)> = None;
    for &x in xs {
        let Some(v) = g(x)? else {
            prev = None;
            continue;
        };
        if v == 0.0 {
            out.push(x);
        } else if let Some(p) = prev.filter(|p| p.1 * v < 0.0) {
            let (mut lo, mut hi, mut flo) = (p.0, x, p.1);
            for _ in 0..100 {
                let mid = (lo + hi) / 2.0;
                let Some(fm) = g(mid)? else { break };
                if flo * fm <= 0.0 {
                    hi = mid;
                } else {
                    lo = mid;
                    flo = fm;
                }
            }
            let xr = (lo + hi) / 2.0;
            if let Some(fr) = g(xr)? {
                if fr.abs() < 1e-6 * 1f64.max(p.1.abs() + v.abs()) {
                    out.push(xr);
                }
            }
        }
        prev = Some((x, v));
    }
    let mut clean: Vec<f64> = vec![];
    for r in out {
        if clean.last().is_none_or(|l| (r - l).abs() > 1e-7) {
            clean.push(if r.abs() > 1e-12 { py_round(r, 12) } else { 0.0 });
        }
    }
    Ok(clean)
}

