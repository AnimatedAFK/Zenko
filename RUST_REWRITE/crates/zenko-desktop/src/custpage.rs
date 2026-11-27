//! The hub's Customize page: zenko.py hub_build_customize (live preview, name and personality, her voice, coat / eyes /
//! foxfire swatches, tails, ears and cheeks, size, outfits, idle animations, Zenko19's wardrobe and poses, start over)
//! with Zenko20's look and feel and sidebar pages and Zenko22's travel outfits, speech bubble, garden decor and
//! sidebar scene, in the same order, with the same widgets, fonts and padding.
//!
//! Commands: "cz_chk:<key>", "cz_rad:<key>:<s|i|f>:<value>", "cz_outfit:<key>", "cz_btn:<what>", "cz_mood:<mood>",
//! "cz_play:<kind>", "cz_pose:<kind>", "cz_robe:<key>", "cz_bubble:<key>", "cz_decor:<key>", "cz_scene:<key>",
//! "cz_nav:<key>" (the sidebar ticks); the swatches, the accent dots and the sidebar arrows are clicked by place.

use crate::hubwin::{canvas_of, hub_title_draw};
use crate::pagekit::{Faces, Scroller, WrapState, hub_card, hub_scroller, zk_fonts, zk_wrap};
use crate::setpage::{B, py_float, s_of};
use crate::theme::Theme;
use crate::tkcanvas::{Canvas, Photo};
use crate::ui::{Anchor, Fill, Font, Fonts, Grid, Kind, Pack, Side, Ui};
use serde_json::Value;
use std::rc::Rc;
use std::sync::OnceLock;
use zenko_art::palette::{Palette, mixhex};

pub const SUB: &str = "Her coat, eyes, foxfire, outfits, voice and little habits. Changes show up right away.";

/// zenko.py's look tables (COATS, EYE_COLOURS, FIRE_COLOURS, BIB_COLOURS, OUTFITS, ACCENTS, MOTION_LEVELS, ROBES,
/// Z22_BUBBLES / DECOR / SCENES / ACCENTS, the journey's places with their hours, DEFAULTS for Start over)
pub fn cdata() -> &'static Value {
    static D: OnceLock<Value> = OnceLock::new();
    D.get_or_init(|| serde_json::from_str(include_str!("../data/customize.json")).expect("customize.json"))
}

fn rows(k: &str) -> Vec<Vec<Value>> {
    cdata()[k].as_array().map(|a| a.iter().map(|r| r.as_array().cloned().unwrap_or_default()).collect()).unwrap_or_default()
}

fn st(v: &Value) -> String {
    v.as_str().unwrap_or("").to_string()
}

/// the desktop fox's pose as the preview shows it (cz_anim): tails picture (None when she's curled), body picture,
/// and its offset in her pixels
#[derive(Clone)]
pub struct Preview {
    pub tails: Option<Rc<Photo>>,
    pub body: Rc<Photo>,
    pub dxf: i32,
    pub dyf: i32,
}

/// What the page shows, from the app
#[derive(Clone, Default)]
pub struct CustData {
    pub cfg: Value,
    pub fox_name: String,
    pub catch: String,
    /// bond_text(): hearts, its name
    pub bond: (String, String),
    /// z19_hours(), journey_state(): the hours travelled and the places reached
    pub hours: f64,
    pub jhours: f64,
    pub got: Vec<String>,
    pub preview: Option<Preview>,
    /// THEMES[theme]["acc"] for the Theme accent dot
    pub theme_acc: String,
}

pub struct CustPage {
    pub pg: usize,
    pub title: usize,
    pub sc: Scroller,
    pub preview: usize,
    pub bond: usize,
    pub entries: Vec<(String, usize)>,
    /// the swatch canvases and their labels: (widget, config key, value)
    pub swatches: Vec<(usize, String, String)>,
    pub sw_cvs: Vec<(usize, String, String, String, bool)>,
    /// the accent dots and their labels: (widget, accent key)
    pub accents: Vec<(usize, String)>,
    /// the sidebar editor's arrows: (label, index, -1 / 1), its reset link
    pub arrows: Vec<(usize, usize, i32)>,
    pub nav_reset: usize,
    pub nav_order: Vec<String>,
    pub labels: Vec<(String, usize)>,
    pub scale: usize,
    pub wrap: WrapState,
    seig: Option<(String, Rc<Photo>)>,
    zk_f: f64,
    name: String,
}

/// zenko20 z20_nav_all: every page the sidebar can hold, in the usual order
pub const NAV_ALL: [(&str, &str); 16] = [("home", "Home"), ("timetable", "Timetable"), ("deadlines", "Upcoming"),
    ("reminders", "Reminders"), ("notebook", "Study books"), ("daily", "Daily"), ("exercise", "Exercise"),
    ("garden", "Zen garden"), ("map", "World map"), ("screen", "Screen time"), ("chat", "Chat"), ("square", "Town square"),
    ("notes", "Notes"), ("you", "You"), ("customize", "Customize"), ("settings", "Settings")];

impl CustPage {
    pub fn build(ui: &mut Ui, pg: usize, th: &Theme, f: &Faces, cm10: i32, d: &CustData) -> CustPage {
        let title = ui.canvas(pg, &th.banner, cm10, 96);
        canvas_of(ui, title);
        ui.pack(title, Pack::default().fill(Fill::X));
        let sc = hub_scroller(ui, pg, th, cm10);
        let mut p = CustPage { pg, title, sc: sc.clone(), preview: 0, bond: 0, entries: vec![], swatches: vec![],
                               sw_cvs: vec![], accents: vec![], arrows: vec![], nav_reset: 0, nav_order: vec![], labels: vec![],
                               scale: 0, wrap: WrapState::default(), seig: None, zk_f: 1.0,
                               name: format!("Customize {}", d.fox_name) };
        let mut b = B { ui, th, f, cfg: &d.cfg, cm10, bg: th.card.clone() };
        let outer = b.ui.frame(sc.inner, &th.paper);
        b.ui.pack(outer, Pack::default().fill(Fill::X).padx(24, 24).pady(12, 12));
        let left = b.ui.frame(outer, &th.paper);
        b.ui.pack(left, Pack::default().side(Side::Left).fill(Fill::Y).anchor(Anchor::N).padx(0, 8));
        let right = b.ui.frame(outer, &th.paper);
        b.ui.pack(right, Pack::default().side(Side::Left).fill(Fill::Both).expand().padx(8, 0));
        let rcol = b.ui.frame(right, &th.paper);
        b.ui.pack(rcol, Pack::default().side(Side::Left).fill(Fill::Both).expand().anchor(Anchor::N));
        p.preview_card(&mut b, left, d);
        p.name_card(&mut b, left, d);
        p.voice_card(&mut b, left);
        p.look_cards(&mut b, rcol);
        p.habits(&mut b, rcol, d);
        p.z20(&mut b, sc.inner, d);
        p.z22(&mut b, sc.inner, d);
        p
    }

    fn card(&mut self, b: &mut B, parent: usize, title: &str, pady: (i32, i32)) -> usize {
        let c = hub_card(b.ui, parent, Some(title), "round", b.th, b.f);
        b.ui.pack(c, Pack::default().fill(Fill::X).pady(pady.0, pady.1));
        c
    }

    /// label(parent, text): GREY 9, anchor w, justify left, wraplength 330, fill x, padx 14, pady (6, 0)
    fn label(b: &mut B, parent: usize, text: &str) -> usize {
        let l = b.note(parent, text, &b.th.grey.clone(), 9, false, 330);
        b.ui.pack(l, Pack::default().fill(Fill::X).padx(14, 14).pady(6, 0));
        l
    }

    /// check(parent, text, key, default)
    fn check(b: &mut B, parent: usize, text: &str, key: &str, default: bool) -> usize {
        let on = b.flag(key, default);
        let c = b.chk(parent, text, on, &format!("cz_chk:{key}"));
        b.ui.pack(c, Pack::default().fill(Fill::X).padx(14, 14));
        c
    }

    /// radios(parent, items, key, default, cast): gridded four to a row (sticky w, padx 0 8) in a frame (fill x,
    /// padx 14, pady 0 4)
    fn radios(b: &mut B, parent: usize, items: &[(&str, &str)], key: &str, default: &str, ty: char) {
        let cur = b.get(key, default);
        let bg = b.bg.clone();
        let fr = b.ui.frame(parent, &bg);
        b.ui.pack(fr, Pack::default().fill(Fill::X).padx(14, 14).pady(0, 4));
        for (i, (lab, val)) in items.iter().enumerate() {
            let r = b.radio(fr, lab, cur == *val, &format!("cz_rad:{key}:{ty}:{val}"), 10);
            b.ui.grid(r, Grid { row: i / 4, col: i % 4, sticky: "w", padx: (0, 8), ..Default::default() });
        }
    }

    /// swatches(parent, opts, key, coat): six to a row, a 30-pixel dot over its name
    fn swatches(&mut self, b: &mut B, parent: usize, opts: &[(String, String, String)], key: &str, coat: bool) {
        let bg = b.bg.clone();
        let row = b.ui.frame(parent, &bg);
        b.ui.pack(row, Pack::default().fill(Fill::X).padx(12, 12).pady(2, 6));
        for (i, (lab, val, col)) in opts.iter().enumerate() {
            let f = b.ui.frame(row, &bg);
            b.ui.grid(f, Grid { row: i / 6, col: i % 6, padx: (2, 2), pady: (2, 2), ..Default::default() });
            let cv = b.ui.canvas(f, &bg, 30, 30);
            canvas_of(b.ui, cv);
            b.ui.pack(cv, Pack::default());
            let lb = b.lbl(f, lab, &b.th.grey.clone(), 8);
            b.ui.pack(lb, Pack::default());
            self.swatches.push((cv, key.into(), val.clone()));
            self.swatches.push((lb, key.into(), val.clone()));
            self.sw_cvs.push((cv, key.into(), val.clone(), col.clone(), coat));
        }
        let cur = b.get(key, "");
        self.draw_swatches(b.ui, b.th, key, &cur);
    }

    /// draw() of one swatch row: the gold ring round the one in use
    pub fn draw_swatches(&self, ui: &mut Ui, th: &Theme, key: &str, cur: &str) {
        let bg = th.card.clone();
        for (cv, k, val, col, coat) in &self.sw_cvs {
            if k != key {
                continue;
            }
            let t = canvas_of(ui, *cv);
            *t = Canvas::default();
            let on = cur == val;
            t.oval(2.0, 2.0, 28.0, 28.0).outline(&if on { th.gold.clone() } else { mixhex(&bg, &th.ink, 0.12) })
                .width(if on { 3.0 } else { 1.0 });
            let fur = ["snow", "classic", "ember", "fennec", "midnight", "sakura", "mint", "silver", "golden", "black"];
            if *coat && !val.is_empty() && fur.contains(&col.as_str()) {
                let fp = Palette::fur(col);
                let (o, k2, w) = (fp.hex(b'O').unwrap_or_default(), fp.hex(b'K').unwrap_or_default(), fp.hex(b'W').unwrap_or_default());
                t.oval(7.0, 7.0, 23.0, 23.0).fill(&o).outline(&k2).width(2.0);
                t.arc(7.0, 7.0, 23.0, 23.0, 200.0, 140.0, "chord").fill(&w).outline("");
                t.oval(7.0, 7.0, 23.0, 23.0).outline(&k2).width(2.0);
            } else if !col.is_empty() && !*coat {
                t.oval(7.0, 7.0, 23.0, 23.0).fill(col).outline(&th.line);
            } else {
                t.oval(7.0, 7.0, 23.0, 23.0).fill(&bg).outline(&th.grey);
                t.line(vec![10.0, 20.0, 20.0, 10.0]).fill(&th.grey).width(2.0);
            }
        }
    }

    fn preview_card(&mut self, b: &mut B, left: usize, d: &CustData) {
        let th = b.th;
        let bg = b.bg.clone();
        let c = self.card(b, left, "Live preview", (0, 0));
        let pz = 3.0;
        let (w_, h_) = (zenko_art::core::W as f64, zenko_art::core::H as f64);
        let pbg = mixhex(&th.card, &th.night, 0.55);
        let cv = b.ui.canvas(c, &pbg, (w_ * pz + 20.0) as i32, (h_ * pz + 20.0) as i32);
        b.ui.ws[cv].hl = 1;
        b.ui.ws[cv].hlc = th.line.clone();
        b.ui.pack(cv, Pack::default().padx(14, 14).pady(4, 6));
        self.preview = cv;
        {
            let t = canvas_of(b.ui, cv);
            let mut rnd = zenko_sound::mt::PyRandom::new(3);
            for _ in 0..26 {
                let x = rnd.uniform(4.0, w_ * pz + 16.0);
                let y = rnd.uniform(4.0, h_ * pz * 0.6);
                let a = rnd.uniform(0.2, 0.6);
                t.rect(x, y, x + 2.0, y + 2.0).fill(&mixhex(&pbg, &th.ink, a)).outline("");
            }
        }
        self.draw_preview(b.ui, th, &b.f.ui.clone(), d);
        Self::label(b, c, "Try a mood (or click her: head, nose, belly and tails all react differently)");
        let mf = b.ui.frame(c, &bg);
        b.ui.pack(mf, Pack::default().fill(Fill::X).padx(14, 14).pady(2, 4));
        for (lab, mood) in [("Happy", "happy"), ("Focus", "focus"), ("Sleepy", "sleep"), ("Cross", "alert"), ("Sad", "sad")] {
            let x = b.btn(mf, lab, &format!("cz_mood:{mood}"), Some(&th.field), None, 9, 8, 1);
            b.ui.pack(x, Pack::default().side(Side::Left).padx(0, 5));
        }
        Self::label(b, c, "Play with her");
        let pf = b.ui.frame(c, &bg);
        b.ui.pack(pf, Pack::default().fill(Fill::X).padx(14, 14).pady(2, 4));
        for (i, (lab, kind)) in [("Headpat", "pet"), ("Boop", "boop"), ("Tickle", "tickle"), ("Treat", "treat"), ("Yarn", "yarn"),
                                 ("High five", "highfive"), ("Hide & seek", "hide"), ("Gift", "gift"), ("Dance", "dance"),
                                 ("Brush", "brush"), ("Sing", "sing"), ("Belly rub", "belly")].iter().enumerate() {
            let x = b.btn(pf, lab, &format!("cz_play:{kind}"), Some(&th.start), None, 9, 8, 1);
            b.ui.grid(x, Grid { row: i / 4, col: i % 4, padx: (0, 5), pady: (3, 3), sticky: "w", ..Default::default() });
        }
        let bl = b.ui.label(c, &format!("Bond  {}   {}", d.bond.0, d.bond.1), &bg, &th.gold, b.font(10, true));
        b.ui.set_anchor(bl, Anchor::W);
        b.ui.pack(bl, Pack::default().fill(Fill::X).padx(14, 14).pady(4, 12));
        self.bond = bl;
    }

    /// cz_anim: the preview's pictures (the same pose as the desktop fox, three times bigger) and the name line
    pub fn draw_preview(&self, ui: &mut Ui, th: &Theme, font_ui: &str, d: &CustData) {
        let z = 3.0;
        let h_ = zenko_art::core::H as f64;
        let t = canvas_of(ui, self.preview);
        t.items.truncate(26);
        let (dx, dy) = d.preview.as_ref().map(|p| (p.dxf as f64, p.dyf.min(60) as f64)).unwrap_or((0.0, 0.0));
        let (x, y) = (10.0 + dx * z, 10.0 + dy * z);
        let tails = d.preview.as_ref().and_then(|p| p.tails.clone());
        match tails {
            Some(img) => {
                t.image(x, y, img).anchor("nw");
            }
            None => {
                let empty = Rc::new(Photo { w: 0, h: 0, px: vec![] });
                let it = t.image(x, y, empty).anchor("nw");
                it.hidden = d.preview.is_some();
                it.image = None;
            }
        }
        match d.preview.as_ref() {
            Some(p) => {
                t.image(x, y, p.body.clone()).anchor("nw");
            }
            None => {
                let empty = Rc::new(Photo { w: 0, h: 0, px: vec![] });
                t.image(10.0, 10.0, empty).anchor("nw").image = None;
            }
        }
        t.text(14.0, h_ * z + 12.0, &format!("{}  \u{b7}  {}", d.fox_name, d.catch)).anchor("w").fill(&th.grey)
            .font(Font::new(font_ui, 9, false)).tags(&["nm"]);
    }

    fn name_card(&mut self, b: &mut B, left: usize, d: &CustData) {
        let bg = b.bg.clone();
        let c = self.card(b, left, "Name and personality", (12, 0));
        Self::label(b, c, "Her name");
        let e = b.entry(c, &d.fox_name, 22);
        b.ui.pack(e, Pack::default().anchor(Anchor::W).padx(14, 14).pady(2, 2));
        self.entries.push(("fox_name".into(), e));
        Self::label(b, c, "Her catchphrase");
        let e = b.entry(c, &d.catch, 22);
        b.ui.pack(e, Pack::default().anchor(Anchor::W).padx(14, 14).pady(2, 2));
        self.entries.push(("catchphrase".into(), e));
        Self::label(b, c, "How much fox-girl talk (kon kon~, ehehe~)");
        Self::radios(b, c, &[("Calm", "0"), ("Cute", "1"), ("Extra kawaii", "2")], "voice_level", "1", 'i');
        Self::label(b, c, "How she reacts when you slack");
        Self::radios(b, c, &[("Gentle", "gentle"), ("Mixed", "mixed"), ("Savage", "savage")], "mode", "mixed", 's');
        let g = b.ui.frame(c, &bg);
        b.ui.ws[g].opt_h = 8;
        b.ui.pack(g, Pack::default());
    }

    fn voice_card(&mut self, b: &mut B, left: usize) {
        let th = b.th;
        let bg = b.bg.clone();
        let c = self.card(b, left, "Her voice", (12, 0));
        Self::label(b, c, "Little voice clips on her reactions (kon kon~, ehehe~, kyuu~, mou~!)");
        Self::radios(b, c, &[("Off", "off"), ("Fox-girl chirps (built in)", "synth"), ("VOICEVOX (free app)", "voicevox")],
                     "voice_clips", "synth", 's');
        Self::label(b, c, "Chirp pitch");
        let cp = b.cfg.get("chirp_pitch").map(|v| match v {
            Value::Number(n) => py_float(n.as_f64().unwrap_or(1.0)),
            v => s_of(v),
        }).unwrap_or_else(|| "1.0".into());
        let fr = b.ui.frame(c, &bg);
        b.ui.pack(fr, Pack::default().fill(Fill::X).padx(14, 14).pady(0, 4));
        for (i, (lab, val)) in [("Soft", "0.88"), ("Cute", "1.0"), ("Squeaky", "1.15")].iter().enumerate() {
            let r = b.radio(fr, lab, cp == *val, &format!("cz_rad:chirp_pitch:f:{val}"), 10);
            b.ui.grid(r, Grid { row: i / 4, col: i % 4, sticky: "w", padx: (0, 8), ..Default::default() });
        }
        let vvrow = b.ui.frame(c, &bg);
        b.ui.pack(vvrow, Pack::default().fill(Fill::X).padx(14, 14).pady(4, 2));
        let vst = b.note(c, "", &th.grey, 9, false, 330);
        self.labels.push(("vv_status".into(), vst));
        let x = b.btn(vvrow, "Find VOICEVOX", "cz_btn:vv_find", Some(&th.start), None, 9, 8, 1);
        b.ui.pack(x, Pack::default().side(Side::Left));
        let x = b.btn(vvrow, "Hear her", "cz_btn:voice_demo", Some(&th.field), None, 9, 8, 1);
        b.ui.pack(x, Pack::default().side(Side::Left).padx(6, 0));
        b.ui.pack(vst, Pack::default().fill(Fill::X).padx(14, 14).pady(2, 2));
        Self::check(b, c, "Open VOICEVOX by itself when Zenko starts (hidden)", "voicevox_autostart", true);
        Self::check(b, c, "With VOICEVOX, she says each speech bubble out loud in Japanese (the bubble stays in English)",
                    "jp_voice", true);
        Self::label(b, c, "Her sounds are only her voice now: no drums, bells or plucks. Without VOICEVOX she uses her little \
                           built-in voice clips; pick Off above for silence.");
        let eng_row = b.ui.frame(c, &bg);
        b.ui.pack(eng_row, Pack::default().fill(Fill::X).padx(14, 14).pady(2, 0));
        let path = b.get("voicevox_engine_path", "");
        let et = if path.is_empty() {
            "Engine: not found yet (she looks in your voicevox_engine folder and the VOICEVOX app's folder)".to_string()
        } else {
            format!("Engine: {path}")
        };
        let el = b.note(c, &et, &th.grey, 8, false, 330);
        self.labels.push(("engine".into(), el));
        let x = b.btn(eng_row, "Browse for run.exe", "cz_btn:vv_browse", Some(&th.field), None, 9, 8, 1);
        b.ui.pack(x, Pack::default().side(Side::Left));
        b.ui.pack(el, Pack::default().fill(Fill::X).padx(14, 14).pady(2, 2));
        Self::label(b, c, "Talking voice (when she reads her replies out loud)");
        let v = b.cfg.get("tts_pitch").and_then(|v| v.as_f64()).unwrap_or(0.0).trunc();
        let sp = b.ui.scale(c, 0.0, 10.0, v, 220, 15, 30, &bg, &th.field, "cz_scale:tts_pitch");
        if let Kind::Scale { label, show, font, fg, .. } = &mut b.ui.ws[sp].kind {
            *label = "Cute pitch (0 = normal)".into();
            *show = true;
            *font = Font::new(&b.f.ui, 8, false);
            *fg = th.ink.clone();
        }
        b.ui.pack(sp, Pack::default().anchor(Anchor::W).padx(14, 14));
        self.scale = sp;
        let trow = b.ui.frame(c, &bg);
        b.ui.pack(trow, Pack::default().fill(Fill::X).padx(14, 14).pady(4, 2));
        let js = b.note(c, "", &th.grey, 9, false, 330);
        self.labels.push(("jp_status".into(), js));
        let x = b.btn(trow, "Use a Japanese Windows voice", "cz_btn:use_jp", Some(&th.start), None, 9, 8, 1);
        b.ui.pack(x, Pack::default().side(Side::Left));
        let x = b.btn(trow, "Test", "cz_btn:speak_test", Some(&th.field), None, 9, 8, 1);
        b.ui.pack(x, Pack::default().side(Side::Left).padx(6, 0));
        b.ui.pack(js, Pack::default().fill(Fill::X).padx(14, 14).pady(2, 2));
        Self::label(b, c, "Her chirps are made inside the app (no recordings of real people). VOICEVOX voices are free; \
                           credit 'VOICEVOX:<character>' if you share them.");
        let g = b.ui.frame(c, &bg);
        b.ui.ws[g].opt_h = 8;
        b.ui.pack(g, Pack::default());
    }

    fn gap(b: &mut B, c: usize, h: i32) {
        let bg = b.bg.clone();
        let g = b.ui.frame(c, &bg);
        b.ui.ws[g].opt_h = h;
        b.ui.pack(g, Pack::default());
    }

    fn look_cards(&mut self, b: &mut B, rcol: usize) {
        let c = self.card(b, rcol, "Coat", (0, 0));
        Self::label(b, c, "Fur colour (the app theme stays the same)");
        let coats: Vec<(String, String, String)> = rows("COATS").iter()
            .map(|r| (st(&r[1]), st(&r[0]), st(&r[2]))).collect();
        self.swatches(b, c, &coats, "coat", true);
        let c = self.card(b, rcol, "Eyes", (12, 0));
        let eyes: Vec<(String, String, String)> = rows("EYE_COLOURS").iter().map(|r| (st(&r[0]), st(&r[1]), st(&r[1]))).collect();
        self.swatches(b, c, &eyes, "eye_color", false);
        let c = self.card(b, rcol, "Foxfire (kitsune-bi)", (12, 0));
        let fire: Vec<(String, String, String)> = rows("FIRE_COLOURS").iter().map(|r| (st(&r[0]), st(&r[1]), st(&r[1]))).collect();
        self.swatches(b, c, &fire, "foxfire_color", false);
        Self::check(b, c, "Foxfire wisps float around her", "foxfire", true);
        Self::gap(b, c, 6);
        let c = self.card(b, rcol, "Tails", (12, 0));
        Self::label(b, c, "A zenko earns a new tail with wisdom");
        Self::radios(b, c, &[("Grow as I focus", "grow"), ("1", "1"), ("2", "2"), ("3", "3"), ("5", "5"), ("7", "7"), ("9", "9")],
                     "tails", "grow", 's');
        let c = self.card(b, rcol, "Ears and cheeks", (12, 0));
        Self::label(b, c, "Ears");
        Self::radios(b, c, &[("Pointy", "pointy"), ("Snowy tips", "snowy"), ("Floppy", "floppy")], "ear_style", "pointy", 's');
        Self::label(b, c, "Blush");
        Self::radios(b, c, &[("None", "none"), ("Soft", "soft"), ("Rosy", "rosy")], "blush_style", "soft", 's');
        Self::gap(b, c, 6);
        let c = self.card(b, rcol, "Size on your desktop", (12, 0));
        Self::radios(b, c, &[("Tiny", "3"), ("Small", "4"), ("Medium", "5"), ("Large", "6")], "size", "4", 'i');
        Self::gap(b, c, 6);
        let c = self.card(b, rcol, "Outfits and accessories", (12, 0));
        let cur: Vec<String> = b.cfg.get("outfit").and_then(|v| v.as_array()).map(|a| a.iter().map(s_of).collect())
            .unwrap_or_default();
        for r in rows("OUTFITS") {
            let (k, lab) = (st(&r[0]), st(&r[1]));
            let cb = b.chk(c, &lab, cur.contains(&k), &format!("cz_outfit:{k}"));
            b.ui.pack(cb, Pack::default().fill(Fill::X).padx(14, 14));
        }
        Self::label(b, c, "Inari bib colour");
        let bibs: Vec<(String, String, String)> = rows("BIB_COLOURS").iter().map(|r| (st(&r[0]), st(&r[1]), st(&r[1]))).collect();
        self.swatches(b, c, &bibs, "scarf", false);
        Self::check(b, c, "Glasses during focus sessions", "glasses_in_session", true);
        Self::check(b, c, "Headphones while music plays", "headphones_with_music", true);
        Self::check(b, c, "Hachimaki headband in lock-in mode", "lockin_headband", true);
        Self::gap(b, c, 6);
    }

    fn habits(&mut self, b: &mut B, rcol: usize, d: &CustData) {
        let th = b.th;
        let bg = b.bg.clone();
        let c = self.card(b, rcol, "Idle animations", (12, 0));
        Self::check(b, c, "Gentle head sway", "idle_sway", true);
        Self::check(b, c, "Ear twitches", "idle_ears", true);
        Self::check(b, c, "Licks her paw sometimes", "idle_lick", true);
        Self::check(b, c, "Blinks", "idle_blink", true);
        Self::check(b, c, "Big sleepy yawns now and then", "idle_yawn", true);
        Self::check(b, c, "Looks at my mouse", "look_at_mouse", true);
        Self::check(b, c, "Curls up and sleeps when I'm away", "curl_when_away", true);
        Self::label(b, c, "Liveliness");
        Self::radios(b, c, &[("Calm", "calm"), ("Normal", "normal"), ("Lively", "lively")], "idle_lively", "normal", 's');
        Self::gap(b, c, 6);
        // z19_customize: the wardrobe and the poses
        let c = self.card(b, rcol, "Wardrobe (earned with study hours)", (12, 0));
        let dis = mixhex(&bg, &th.grey, 0.6);
        let l = b.ui.label(c, &format!("You've studied {:.1} hours with her so far.", d.hours), &bg, &th.grey, b.font(9, false));
        b.ui.set_anchor(l, Anchor::W);
        b.ui.pack(l, Pack::default().fill(Fill::X).padx(14, 14).pady(0, 4));
        let robe = b.get("robe", "auto");
        let rh = |k: &str| cdata()["ROBES"][k]["hours"].as_f64().unwrap_or(0.0);
        let opts = [("Dress for the season (auto)", "auto", 3.0, "Seasonal kimono, a yukata for summer festivals, the uniform when exams are near"),
                    ("Seasonal kimono", "kimono", 3.0, "Sakura, ro silk, momiji or snow, following the calendar"),
                    ("Festival yukata", "yukata", rh("yukata"), "Indigo with morning glories and a yellow obi"),
                    ("Sailor uniform", "uniform", rh("uniform"), "For exam season"),
                    ("Just her fur", "none", 0.0, "")];
        for (lab, val, need, sub) in opts {
            let ok = d.hours >= need;
            let txt = if ok { lab.to_string() } else { format!("{lab}  (unlocks at {} h)", need as i64) };
            let r = b.radio(c, &txt, robe == val, &if ok { format!("cz_robe:{val}") } else { String::new() }, 10);
            if !ok && let Kind::Check { fg, .. } = &mut b.ui.ws[r].kind {
                *fg = dis.clone();
            }
            b.ui.pack(r, Pack::default().fill(Fill::X).padx(14, 14));
            if !sub.is_empty() {
                let s = b.ui.label(c, &format!("    {sub}"), &bg, &mixhex(&bg, &th.grey, 0.8), b.font(8, false));
                b.ui.set_anchor(s, Anchor::W);
                b.ui.pack(s, Pack::default().fill(Fill::X).padx(14, 14));
            }
        }
        Self::gap(b, c, 8);
        let c2 = self.card(b, rcol, "Fox poses", (12, 0));
        let on = b.flag("fox_poses", true);
        let cb = b.chk(c2, "Stretches, play bows, naps and pounces now and then", on, "cz_chk:fox_poses");
        b.ui.pack(cb, Pack::default().fill(Fill::X).padx(14, 14));
        let row = b.ui.frame(c2, &bg);
        b.ui.pack(row, Pack::default().fill(Fill::X).padx(14, 14).pady(6, 12));
        for (lab, k) in [("Stretch", "stretch"), ("Play bow", "playbow"), ("Pounce", "pounce"), ("Lie down", "rest"), ("Sniff", "sniff")] {
            let x = b.btn(row, lab, &format!("cz_pose:{k}"), Some(&th.field), None, 9, 8, 2);
            b.ui.pack(x, Pack::default().side(Side::Left).padx(0, 6).pady(2, 2));
        }
        let c = self.card(b, rcol, "Start over", (12, 0));
        let x = b.btn(c, "Reset her look to default", "cz_btn:reset", Some(&th.field), None, 9, 10, 2);
        b.ui.pack(x, Pack::default().anchor(Anchor::W).padx(14, 14).pady(8, 12));
    }

    /// z20_customize_extra: look and feel, the sidebar's pages
    fn z20(&mut self, b: &mut B, inner: usize, d: &CustData) {
        let th = b.th;
        let bg = b.bg.clone();
        let row = b.ui.frame(inner, &th.paper);
        b.ui.pack(row, Pack::default().fill(Fill::X).padx(24, 24).pady(4, 18));
        let left = b.ui.frame(row, &th.paper);
        b.ui.pack(left, Pack::default().side(Side::Left).fill(Fill::Both).expand().padx(0, 8).anchor(Anchor::N));
        let right = b.ui.frame(row, &th.paper);
        b.ui.pack(right, Pack::default().side(Side::Left).fill(Fill::Both).expand().padx(8, 0).anchor(Anchor::N));
        let c = self.card(b, left, "Look and feel", (0, 0));
        let l = b.lbl(c, "Accent colour (buttons, highlights, the glow)", &th.grey, 9);
        b.ui.set_anchor(l, Anchor::W);
        b.ui.pack(l, Pack::default().fill(Fill::X).padx(16, 16).pady(2, 2));
        let sw = b.ui.frame(c, &bg);
        b.ui.pack(sw, Pack::default().fill(Fill::X).padx(12, 12).pady(0, 6));
        let cur = b.get("accent", "theme");
        for (i, r) in accent_list(&d.got).iter().enumerate() {
            let (k, lab, col) = (&r.0, &r.1, &r.2);
            let f = b.ui.frame(sw, &bg);
            b.ui.grid(f, Grid { row: 0, col: i, padx: (3, 3), ..Default::default() });
            let cv = b.ui.canvas(f, &bg, 30, 30);
            b.ui.pack(cv, Pack::default());
            let colv = col.clone().unwrap_or_else(|| d.theme_acc.clone());
            let on = *k == cur;
            {
                let t = canvas_of(b.ui, cv);
                t.oval(2.0, 2.0, 28.0, 28.0).outline(&if on { th.gold.clone() } else { mixhex(&bg, &th.ink, 0.14) })
                    .width(if on { 3.0 } else { 1.0 });
                t.oval(8.0, 8.0, 22.0, 22.0).fill(&colv).outline("");
                if col.is_none() {
                    t.text(15.0, 15.0, "T").fill(&th.onacc).font(Font::new(&b.f.ui, 7, true));
                }
            }
            let tl = b.lbl(f, lab, &th.grey, 8);
            b.ui.pack(tl, Pack::default());
            self.accents.push((cv, k.clone()));
            self.accents.push((tl, k.clone()));
        }
        let l = b.lbl(c, "Motion (how lively the window is)", &th.grey, 9);
        b.ui.set_anchor(l, Anchor::W);
        b.ui.pack(l, Pack::default().fill(Fill::X).padx(16, 16).pady(4, 0));
        let mv = b.get("motion_level", "auto");
        let mf = b.ui.frame(c, &bg);
        b.ui.pack(mf, Pack::default().fill(Fill::X).padx(14, 14));
        for (i, r) in rows("MOTION_LEVELS").iter().enumerate() {
            let (k, lab) = (st(&r[0]), st(&r[1]));
            let rb = b.radio(mf, &lab, mv == k, &format!("cz_rad:motion_level:s:{k}"), 10);
            b.ui.grid(rb, Grid { row: i / 2, col: i % 2, sticky: "w", padx: (0, 10), ..Default::default() });
        }
        let on = b.flag("nav_compact", false);
        let cb = b.chk(c, "Compact sidebar (smaller rows)", on, "cz_chk:nav_compact");
        b.ui.pack(cb, Pack::default().fill(Fill::X).padx(14, 14).pady(4, 12));
        let s = self.card(b, right, "Sidebar pages", (0, 0));
        let l = b.note(s, "Tick what shows in the sidebar and use the arrows to reorder. Hidden pages are still one Ctrl+K \
                           search away.", &th.grey, 9, false, 360);
        b.ui.pack(l, Pack::default().fill(Fill::X).padx(16, 16).pady(0, 4));
        let bx = b.ui.frame(s, &bg);
        b.ui.pack(bx, Pack::default().fill(Fill::X).padx(12, 12).pady(0, 12));
        self.nav_editor(b, bx);
    }

    /// z20_nav_editor(box): two columns (top to bottom) of ticks with up / down arrows, and the reset link
    fn nav_editor(&mut self, b: &mut B, bx: usize) {
        let th = b.th;
        let bg = b.bg.clone();
        let strs = |k: &str| -> Vec<String> {
            b.cfg.get(k).and_then(|v| v.as_array()).map(|a| a.iter().filter_map(|x| x.as_str().map(String::from)).collect())
                .unwrap_or_default()
        };
        let mut order: Vec<String> = strs("nav_order").into_iter().filter(|k| NAV_ALL.iter().any(|a| a.0 == k)).collect();
        for (k, _) in NAV_ALL {
            if !order.iter().any(|o| o == k) {
                order.push(k.into());
            }
        }
        let hidden = strs("nav_hidden");
        let half = order.len().div_ceil(2);
        for (i, k) in order.iter().enumerate() {
            let r = b.ui.frame(bx, &bg);
            b.ui.grid(r, Grid { row: i % half, col: i / half, sticky: "we", padx: (4, 4), pady: (1, 1), ..Default::default() });
            let locked = k == "home" || k == "settings";
            let on = locked || !hidden.contains(k);
            let lab = NAV_ALL.iter().find(|a| a.0 == k).unwrap().1;
            let cb = b.chk(r, lab, on, &if locked { String::new() } else { format!("cz_nav:{k}") });
            if let Kind::Check { fg, .. } = &mut b.ui.ws[cb].kind {
                *fg = if locked { mixhex(&bg, &th.grey, 0.6).to_string() } else if on { th.ink.clone() } else { th.grey.clone() };
                if locked {
                    // (state disabled: Tk's disabledforeground for a checkbutton is its default, #a3a3a3)
                    *fg = "#a3a3a3".into();
                }
            }
            b.ui.ws[cb].opt_w = 13;
            b.ui.pack(cb, Pack::default().side(Side::Left));
            for (sym, dd) in [("\u{25b2}", -1), ("\u{25bc}", 1)] {
                let a = b.lbl(r, sym, &mixhex(&bg, &th.ink, 0.5), 8);
                b.ui.ws[a].padx = 3;
                b.ui.pack(a, Pack::default().side(Side::Left));
                self.arrows.push((a, i, dd));
            }
        }
        b.ui.ws[bx].gw_cols = vec![1, 1];
        let rs = b.ui.label(bx, "Reset to the usual order", &bg, &mixhex(&bg, &th.gold, 0.8), b.font(9, false));
        if let Kind::Label { under, .. } = &mut b.ui.ws[rs].kind {
            *under = true;
        }
        b.ui.grid(rs, Grid { row: half + 1, col: 0, sticky: "w", padx: (6, 6), pady: (6, 0), ..Default::default() });
        self.nav_reset = rs;
        self.nav_order = order;
    }

    /// z22_customize_extra: travel outfits, the speech bubble, garden decor, the sidebar scene
    fn z22(&mut self, b: &mut B, inner: usize, d: &CustData) {
        let th = b.th;
        let bg = b.bg.clone();
        let row = b.ui.frame(inner, &th.paper);
        b.ui.pack(row, Pack::default().fill(Fill::X).padx(24, 24).pady(0, 18));
        let left = b.ui.frame(row, &th.paper);
        b.ui.pack(left, Pack::default().side(Side::Left).fill(Fill::Both).expand().padx(0, 8).anchor(Anchor::N));
        let right = b.ui.frame(row, &th.paper);
        b.ui.pack(right, Pack::default().side(Side::Left).fill(Fill::Both).expand().padx(8, 0).anchor(Anchor::N));
        let dis = mixhex(&bg, &th.grey, 0.6);
        let rewards = crate::rewards::rdata()["MAP_REWARDS"].as_array().cloned().unwrap_or_default();
        let unlocked = |kind: &str| -> Vec<String> {
            rewards.iter().filter(|r| r[1] == kind && d.got.iter().any(|g| r[0] == g.as_str()))
                .filter_map(|r| r[2].as_str().map(String::from)).collect()
        };
        let need = |key: &str| -> String {
            let pid = rewards.iter().find(|r| r[2] == key).and_then(|r| r[0].as_str()).unwrap_or("");
            match rows("JOURNEY").into_iter().find(|p| p[0] == pid) {
                Some(p) => format!("  (reach {}, {} h)", st(&p[1]), p[2].as_f64().unwrap_or(0.0) as i64),
                None => String::new(),
            }
        };
        let radio = |b: &mut B, c: usize, text: &str, on: bool, ok: bool, cmd: &str| {
            let r = b.radio(c, text, on, if ok { cmd } else { "" }, 10);
            if !ok && let Kind::Check { fg, .. } = &mut b.ui.ws[r].kind {
                *fg = dis.clone();
            }
            r
        };
        let c = self.card(b, left, "Travel outfits (from the world map)", (0, 0));
        let robe = b.get("robe", "auto");
        let have = unlocked("outfit");
        for r in rewards.iter().filter(|r| r[1] == "outfit") {
            let key = st(&r[2]);
            let ok = have.contains(&key);
            let name = st(&cdata()["ROBES"][&key]["name"]);
            let t = format!("{name}{}", if ok { String::new() } else { need(&key) });
            let rb = radio(b, c, &t, robe == key, ok, &format!("cz_robe:{key}"));
            b.ui.pack(rb, Pack::default().fill(Fill::X).padx(14, 14));
        }
        let rb = radio(b, c, "Back to the seasonal wardrobe", robe == "auto", true, "cz_robe:auto");
        b.ui.pack(rb, Pack::default().fill(Fill::X).padx(14, 14).pady(4, 0));
        Self::gap(b, c, 10);
        let c2 = self.card(b, left, "Speech bubble", (12, 0));
        let bs = b.get("bubble_style", "");
        let hb = unlocked("bubble");
        if let Some(m) = cdata()["Z22_BUBBLES"].as_object() {
            for (key, v) in m {
                let ok = key.is_empty() || hb.contains(key);
                let t = format!("{}{}", st(&v[0]), if ok { String::new() } else { need(key) });
                let rb = radio(b, c2, &t, bs == *key, ok, &format!("cz_bubble:{key}"));
                b.ui.pack(rb, Pack::default().fill(Fill::X).padx(14, 14));
            }
        }
        let on = b.flag("bubble_scale", true);
        let cb = b.chk(c2, "Bubble grows and shrinks with her size", on, "cz_chk:bubble_scale");
        b.ui.pack(cb, Pack::default().fill(Fill::X).padx(14, 14).pady(6, 10));
        let c3 = self.card(b, right, "Garden decor", (0, 0));
        let hd = unlocked("decor");
        let on_set: Vec<String> = match b.cfg.get("garden_decor") {
            Some(Value::Array(a)) => a.iter().map(s_of).collect(),
            _ => hd.clone(),
        };
        if let Some(m) = cdata()["Z22_DECOR"].as_object() {
            for (key, name) in m {
                let ok = hd.contains(key);
                let t = format!("{}{}", st(name), if ok { String::new() } else { need(key) });
                let cb = b.chk(c3, &t, ok && on_set.contains(key), &if ok { format!("cz_decor:{key}") } else { String::new() });
                if !ok && let Kind::Check { fg, .. } = &mut b.ui.ws[cb].kind {
                    *fg = dis.clone();
                }
                b.ui.pack(cb, Pack::default().fill(Fill::X).padx(14, 14));
            }
        }
        Self::gap(b, c3, 10);
        let c4 = self.card(b, right, "Sidebar scene", (12, 0));
        let sv = b.get("side_scene", "");
        let hs = unlocked("scene");
        if let Some(m) = cdata()["Z22_SCENES"].as_object() {
            for (key, name) in m {
                let ok = key.is_empty() || hs.contains(key);
                let t = format!("{}{}", st(name), if ok { String::new() } else { need(key) });
                let rb = radio(b, c4, &t, sv == *key, ok, &format!("cz_scene:{key}"));
                b.ui.pack(rb, Pack::default().fill(Fill::X).padx(14, 14));
            }
        }
        let ha = unlocked("accent");
        let names: Vec<String> = rows("Z22_ACCENTS").iter().filter(|r| ha.iter().any(|h| r[0] == h.as_str()))
            .map(|r| st(&r[1])).collect();
        let t = format!("Accent colours from the map ({}) appear under Look and feel.",
                        if names.is_empty() { "none yet".to_string() } else { names.join(", ") });
        let l = b.note(c4, &t, &th.grey, 9, false, 440);
        b.ui.pack(l, Pack::default().fill(Fill::X).padx(16, 16).pady(6, 12));
    }

    pub fn fit(&mut self, ui: &mut Ui, fonts: &dyn Fonts, th: &Theme, f: &Faces, zk_f: f64, pw: i32, ph: i32) {
        self.zk_f = zk_f;
        zk_fonts(ui, fonts, self.pg, zk_f);
        ui.layout_at(fonts, pw, ph);
        let w = ui.ws[self.title].w;
        if w >= 20 {
            let mut cv = std::mem::take(canvas_of(ui, self.title));
            let hh = hub_title_draw(&mut cv, w, &self.name, SUB, th, &f.ui, &f.title, zk_f, fonts, &mut self.seig);
            *canvas_of(ui, self.title) = cv;
            if ui.ws[self.title].opt_h != hh {
                ui.ws[self.title].opt_h = hh;
                ui.layout_at(fonts, pw, ph);
            }
        }
        zk_wrap(ui, fonts, self.pg, &mut self.wrap, pw, ph);
        if self.sc.sync(ui) {
            ui.layout_at(fonts, pw, ph);
        }
    }
}

/// ACCENTS as z22_apply_rewards leaves it: the base ones, then the map's that you've travelled far enough for
pub fn accent_list(got: &[String]) -> Vec<(String, String, Option<String>)> {
    let mut v: Vec<(String, String, Option<String>)> = rows("ACCENTS").iter()
        .map(|r| (st(&r[0]), st(&r[1]), r[2].as_array().and_then(|a| a.first()).and_then(|c| c.as_str()).map(String::from)))
        .collect();
    let rewards = crate::rewards::rdata()["MAP_REWARDS"].as_array().cloned().unwrap_or_default();
    let have: Vec<String> = rewards.iter().filter(|r| r[1] == "accent" && got.iter().any(|g| r[0] == g.as_str()))
        .filter_map(|r| r[2].as_str().map(String::from)).collect();
    for r in crate::rewards::rdata()["Z22_ACCENTS"].as_array().into_iter().flatten() {
        if let Some(k) = r[0].as_str()
            && have.iter().any(|h| h == k) {
            v.push((k.into(), st(&r[1]), r[2].as_array().and_then(|a| a.first()).and_then(|c| c.as_str()).map(String::from)));
        }
    }
    v
}

impl crate::Companion {
    /// what the Customize page shows, from the app: her name and catchphrase, the bond, the study and travel hours,
    /// and the desktop fox's pose drawn three times bigger (cz_anim)
    pub fn cust_data(&mut self) -> CustData {
        let cfg = self.cfg.raw.clone();
        let (hs, nm, _) = self.bond_text();
        let life = crate::read_json("zenko_life.json");
        let (jh, got) = crate::rewards::journey_state(&life);
        let theme_acc = crate::theme::theme(&crate::Cfg::from(serde_json::json!({"theme": self.cfg.s("theme", "snow")}))).acc;
        let hd = self.cfg.b("hd_art", true);
        let preview = self.pose.clone().map(|p| {
            let walking = p.state.starts_with("qwalk") || p.state.starts_with("rig");
            let (state, dxf, dyf) = if walking { ("idle".to_string(), 0, 0) } else { (p.state.clone(), p.dxf, p.dyf) };
            let acc: Vec<&str> = p.acc.iter().map(|s| s.as_str()).collect();
            let photo = |g: &zenko_art::core::Grid| {
                let im = zenko_art::raster::sprite_photo(&[g], &self.pal, 3, hd);
                Rc::new(Photo { w: im.w, h: im.h, px: im.px })
            };
            if p.curled {
                Preview { tails: None, body: photo(&zenko_art::curl::build_curl(p.frame, &acc, p.n)), dxf, dyf }
            } else {
                let t = photo(&zenko_art::core::build_tails(p.n, p.tframe, 8, false));
                let bd = photo(&zenko_art::sit::build_frame(&state, p.frame, Some(p.look), &acc, p.hx, p.ear, self.look_ref()));
                Preview { tails: Some(t), body: bd, dxf, dyf }
            }
        });
        CustData { fox_name: self.fox_name(), catch: self.catchphrase(), bond: (hs, nm.to_string()),
                   hours: crate::rewards::total_hours(), jhours: jh, got, preview, theme_acc, cfg }
    }
}
