//! An open study book on the Study books page (zenko.py ZenkoBooks.book_build and what it holds): the header with
//! the other books, the gold rule, the two pages with their gutter (Zenko17 adds the "Opens in its own window" bar);
//! on the left page Zenko at her blackboard (teach_build / teach_step / teach_say), the practice corner
//! (practice_bio, _drill for chem and maths, practice_french, practice_psych, practice_english, practice_diary,
//! practice_grades) and the tutor brain card; on the right page a subject's tutor (nb_subject_page with
//! sources_card, the topics and the earlier pages), the study diary (nb_overview, stacked) or the grades
//! (grades_page). book_fit / book_reflow keep everything inside the paper, nb_wrap wraps a label to its card.

use crate::bookgen::{self, Problem, PsyPrompt};
use crate::ema::fmt_hm;
use crate::hubwin::canvas_of;
use crate::pagekit::{Scroller, hub_scroller, tt_entry};
use crate::session::clip;
use crate::shelfpage::{book, practice, spaced};
use crate::theme::Theme;
use crate::tkcanvas::{Canvas, Photo};
use crate::tktext::{Text, Wrap};
use crate::ui::{Anchor, Fill, Font, Grid, Kind, Pack, Place, Side, Ui};
use serde_json::{Value, json};
use std::collections::HashMap;
use std::rc::Rc;
use zenko_art::palette::{Palette, mixhex};
use zenko_sound::mt::PyRandom;

pub const SUBJ: [(&str, &str); 6] = [("bio", "Bio"), ("chem", "Chem"), ("math", "Maths"), ("french", "French"),
                                     ("psych", "Psych"), ("english", "English")];

/// one subject: (sid, name, short name, colour)
pub type Subj = (String, String, String, String);

/// What a book shows, gathered from the app (Companion::book_data)
#[derive(Clone, Debug, Default)]
pub struct BookData {
    pub key: String,
    pub subjects: Vec<Subj>,
    /// minutes per subject this week / today
    pub week: HashMap<String, f64>,
    pub today: HashMap<String, f64>,
    /// the whole life file as it is now (book_state, fr_vocab, cards, nb_hist, grades, src_out, goals, unsorted,
    /// hourlog ...); the practice corners change their parts of it
    pub life: Value,
    pub cfg: Value,
    /// _mincho_font(), _serif_font(), _jp_font() or FONT_UI
    pub jp: String,
    pub serif: String,
    pub jpg: String,
    pub hd: bool,
    pub tails: i32,
    pub look: zenko_art::sit::Look,
    /// the flashcards due now
    pub due: Vec<Value>,
    /// exam_for(sid) and exam_for(sid, specific_only=True): (title, when_text, minutes studied, minutes needed)
    pub exam: HashMap<String, (String, String, f64, f64)>,
    pub exam_specific: HashMap<String, (String, String, f64, f64)>,
    /// the first upcoming deadline that counts for every subject
    pub gen_exam: Option<(String, String, f64, f64)>,
    /// recap_text() but its last line (chosen at random when you studied 30+ minutes today)
    pub recap: String,
    /// seconds studied today (slog)
    pub recap_tot: f64,
    pub best_time: String,
    pub best: Vec<i64>,
    pub distraction: String,
    /// late_minutes(7): (weekday "Mon", minutes)
    pub late: Vec<(String, f64)>,
    pub today_study: f64,
    /// src_load(sid)
    pub sources: Vec<Value>,
    /// time.time()
    pub now: f64,
    /// datetime.date.today().isoformat()
    pub today_iso: String,
}

/// one practice corner's widgets and state
#[derive(Clone, Debug, Default)]
pub struct Prac {
    pub labels: HashMap<&'static str, usize>,
    pub entry: usize,
    pub text: usize,
    /// bio: the fact; chem / maths: the problem; french: (theme, fr, en); psych: the prompt and the quiz
    pub fact: Option<Value>,
    pub quiz: bool,
    pub prob: Option<Problem>,
    pub done: bool,
    pub word: Option<(String, String, String)>,
    pub last: Option<String>,
    pub psy: Option<PsyPrompt>,
    pub term: Option<(String, String)>,
    pub opts: usize,
    pub radios: Vec<(usize, String, String)>,
    /// grades: the option menus (subject, kind, grade) and entries (name, score, out of, date)
    pub menus: Vec<usize>,
    pub entries: Vec<usize>,
    /// psych: when the next quiz question comes (time.time())
    pub next_at: Option<f64>,
}

/// teach_build's canvas and its state
#[derive(Clone, Debug, Default)]
pub struct Teach {
    pub cv: usize,
    pub fox: u32,
    pub ptr: u32,
    pub tip: u32,
    pub board: u32,
    pub line: usize,
    pub i: i64,
    pub talk: f64,
    pub happy: f64,
}

/// What a click asks the app to do
#[derive(Clone, Debug, PartialEq)]
pub enum Act {
    None,
    /// open another book (or "shelf")
    Open(String, bool),
    /// the life file changed (save it)
    Save,
    /// cfg_set(key, value)
    Cfg(String, Value),
    /// ask the tutor (sid, question, mode)
    Ask(String, String, String),
    /// an app command (the tool windows, the sources' files)
    Run(String),
    /// build the book again
    Render,
}

pub struct BookPage {
    pub host: usize,
    pub key: String,
    pub page: String,
    pub col: String,
    pub hdr: usize,
    pub rule: usize,
    pub inner: usize,
    pub left: usize,
    pub lsc: Scroller,
    pub rsc: Scroller,
    pub lbody: usize,
    pub rbody: usize,
    pub teach: Teach,
    pub prac: Prac,
    /// the tutor brain card's label and button row
    pub brain: (usize, usize),
    /// nb_subject_page: the question box, the answer, the status line, the mode radios, the history frame
    pub ask: usize,
    pub ans: usize,
    pub status: usize,
    pub mode: String,
    pub modes: Vec<(usize, String)>,
    pub hist: usize,
    /// labels nb_wrap keeps wrapped to their card: (label, margin)
    pub nbwrap: Vec<(usize, i32)>,
    /// the best-focus-times canvas (diary)
    pub focus_cv: usize,
    /// clickable labels: (label, what it does, colour back from GOLD)
    pub links: Vec<(usize, String, String)>,
    /// the Text widgets with a ZkScrollbar: (text, its bar)
    pub bars: Vec<(usize, usize)>,
    pub rnd: PyRandom,
    pub imgs: HashMap<String, Rc<Photo>>,
    pub wrap: crate::pagekit::WrapState,
    pub life: Value,
    pub cfg: Value,
    pub th: Theme,
    pub ui_font: String,
    pub cm10: i32,
    pub d: BookData,
}

pub fn is_subject(key: &str) -> bool {
    crate::shelfpage::SUBJECTS.contains(&key)
}

fn sv(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Null => String::new(),
        o => o.to_string(),
    }
}

/// BOARD: what's on her blackboard
fn board_of(key: &str) -> String {
    bookgen::data()["BOARD"][key].as_str().unwrap_or("").to_string()
}

/// book_pal(key): her coat for this book with the scarf in its colour
pub fn book_pal(key: &str, col: &str) -> Palette {
    let coat = bookgen::data()["BOOK_COATS"][key].as_str().unwrap_or("snow").to_string();
    let mut p = Palette::fur(&coat);
    p.set(b'S', col);
    p.set(b's', &mixhex(col, "#000000", 0.28));
    p
}

/// pick_model(names, wanted)
pub fn pick_model(names: &[String], wanted: &str) -> Option<String> {
    if names.is_empty() {
        return None;
    }
    if names.iter().any(|n| n == wanted) {
        return Some(wanted.into());
    }
    let base = wanted.split(':').next().unwrap_or("");
    let local: Vec<&String> = { let l: Vec<&String> = names.iter().filter(|n| !n.contains("cloud")).collect();
                                if l.is_empty() { names.iter().collect() } else { l } };
    let re = regex::Regex::new(r"vl|vision|llava|embed|moondream").unwrap();
    let a: Vec<&&String> = local.iter().filter(|n| n.split(':').next() == Some(base)).collect();
    if let Some(x) = a.first() {
        return Some((**x).clone());
    }
    let b: Vec<&&String> = local.iter().filter(|n| !re.is_match(n)).collect();
    Some(b.first().map(|x| (**x).clone()).unwrap_or_else(|| local[0].clone()))
}

/// subject_model(sid, names, wanted): the first installed model trained for the subject, else the normal one
pub fn subject_model(sid: &str, names: &[String], wanted: &str, on: bool) -> Option<String> {
    if names.is_empty() {
        return None;
    }
    if on {
        for p in bookgen::data()["SUBJECT_MODELS"][sid].as_array().into_iter().flatten() {
            let p = p.as_str().unwrap_or("");
            if let Some(h) = names.iter().find(|n| n.split(':').next() == Some(p) && !n.contains("cloud")) {
                return Some(h.clone());
            }
        }
    }
    pick_model(names, wanted)
}

/// "%d %b %H:%M" of a time in local time
fn dbhm(t: f64) -> String {
    const MON: [&str; 12] = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
    let s = t.floor() as i64 + crate::local_offset_pub();
    let (_, m, d) = crate::civil_from_days_pub(s.div_euclid(86400));
    let r = s.rem_euclid(86400);
    format!("{d:02} {} {:02}:{:02}", MON[m as usize - 1], r / 3600, r % 3600 / 60)
}

/// "%d %b %Y" of an ISO date (the text itself if it isn't one)
fn dby(iso: &str) -> String {
    const MON: [&str; 12] = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
    match crate::plan::parse_iso(iso) {
        Some(day) => {
            let (y, m, d) = crate::civil_from_days_pub(day);
            format!("{d:02} {} {y}", MON[m as usize - 1])
        }
        None => iso.into(),
    }
}

/// "%g"
fn pg(x: f64) -> String {
    bookgen::pyg_n(x, 6)
}

/// re.sub(r"\[.*?\]", "_____", s)
fn cloze(s: &str) -> String {
    regex::Regex::new(r"\[.*?\]").unwrap().replace_all(s, "_____").to_string()
}

impl BookPage {
    fn lbl(&mut self, ui: &mut Ui, parent: usize, text: &str, fg: &str, fam: &str, size: i32, bold: bool, italic: bool) -> usize {
        let bg = ui.ws[parent].bg.clone();
        let l = ui.label(parent, text, &bg, fg, Font { family: fam.into(), size, bold, italic });
        l
    }

    fn set_wrap(ui: &mut Ui, id: usize, wl: i32, left: bool, anchor_w: bool) {
        if let Kind::Label { wrap, left: lf, anchor, .. } = &mut ui.ws[id].kind {
            *wrap = wl;
            if left {
                *lf = true;
            }
            if anchor_w {
                *anchor = Anchor::W;
            }
        }
    }

    pub fn set_text(ui: &mut Ui, id: usize, t: &str) {
        match &mut ui.ws[id].kind {
            Kind::Label { text, .. } | Kind::Check { text, .. } | Kind::Menu { text, .. } | Kind::Button { text, .. } => *text = t.into(),
            _ => {}
        }
    }

    fn set_fg(ui: &mut Ui, id: usize, c: &str) {
        if let Kind::Label { fg, .. } = &mut ui.ws[id].kind {
            *fg = c.into();
        }
    }

    fn btn(&mut self, ui: &mut Ui, parent: usize, text: &str, cmd: &str, bg: &str, fg: &str, size: i32, padx: i32, pady: i32,
           enabled: bool) -> usize {
        let f = self.ui_font.clone();
        ui.button(parent, text, cmd, bg, fg, &f, size, padx, pady, enabled)
    }

    fn field_btn(&mut self, ui: &mut Ui, parent: usize, text: &str, cmd: &str, size: i32, padx: i32, pady: i32) -> usize {
        let (f, i) = (self.th.field.clone(), self.th.ink.clone());
        self.btn(ui, parent, text, cmd, &f, &i, size, padx, pady, true)
    }

    fn acc_btn(&mut self, ui: &mut Ui, parent: usize, text: &str, cmd: &str, size: i32, padx: i32, pady: i32) -> usize {
        let (a, o) = (self.th.acc.clone(), self.th.onacc.clone());
        self.btn(ui, parent, text, cmd, &a, &o, size, padx, pady, true)
    }

    fn frame(ui: &mut Ui, parent: usize, bg: &str, p: Pack) -> usize {
        let f = ui.frame(parent, bg);
        ui.pack(f, p);
        f
    }

    /// book_section(parent, title): a kanji + serif heading over a hairline (hub_card inside a book)
    fn section(&mut self, ui: &mut Ui, parent: usize, title: Option<&str>) -> usize {
        let page = self.page.clone();
        let f = ui.frame(parent, &page);
        if let Some(title) = title {
            let row = Self::frame(ui, f, &page, Pack::default().fill(Fill::X).padx(16, 16).pady(22, 4));
            let (kan, eng) = match title.split_once(" \u{b7} ") {
                Some((k, e)) => (k.to_string(), e.to_string()),
                None => (String::new(), title.to_string()),
            };
            if !kan.is_empty() {
                let (jp, g) = (self.d.jp.clone(), mixhex(&self.th.gold, &page, 0.1));
                let l = self.lbl(ui, row, &kan, &g, &jp, 12, false, false);
                ui.pack(l, Pack::default().side(Side::Left).padx(0, 10));
            }
            let (serif, ink) = (self.d.serif.clone(), self.th.ink.clone());
            let l = self.lbl(ui, row, &eng, &ink, &serif, 13, false, false);
            ui.set_anchor(l, Anchor::W);
            ui.pack(l, Pack::default().side(Side::Left));
            let ln = ui.frame(f, &mixhex(&self.th.line, &page, 0.25));
            ui.ws[ln].opt_h = 1;
            ui.pack(ln, Pack::default().fill(Fill::X).padx(16, 16).pady(4, 12));
        } else {
            let sp = ui.frame(f, &page);
            ui.ws[sp].opt_h = 10;
            ui.pack(sp, Pack::default().fill(Fill::X));
        }
        f
    }

    /// _p_label(c, text, fg, size, bold, wrap, pady)
    fn p_label(&mut self, ui: &mut Ui, c: usize, text: &str, fg: Option<&str>, size: i32, bold: bool, wrap: i32,
               pady: (i32, i32)) -> usize {
        let fg = fg.map(String::from).unwrap_or_else(|| self.th.ink.clone());
        let f = self.ui_font.clone();
        let l = self.lbl(ui, c, text, &fg, &f, size, bold, false);
        Self::set_wrap(ui, l, wrap, true, true);
        ui.pack(l, Pack::default().fill(Fill::X).padx(14, 14).pady(pady.0, pady.1));
        l
    }

    /// _p_entry(c, width)
    fn p_entry(&mut self, ui: &mut Ui, c: usize, width: i32) -> usize {
        let f = crate::pagekit::Faces { ui: self.ui_font.clone(), title: String::new() };
        let e = tt_entry(ui, c, "", width, &self.th, &f);
        ui.pack(e, Pack::default().anchor(Anchor::W).padx(14, 14).pady(2, 4));
        e
    }

    /// a tk.Text(height, width, font=(FONT_UI, size), relief flat, wrap word) with a 1-pixel LINE ring or none
    fn text_box(&mut self, ui: &mut Ui, parent: usize, height: i32, width: i32, size: i32, bg: &str, ring: bool, pad: (i32, i32),
                edit: bool) -> usize {
        let mut t = Text::new(Font::new(&self.ui_font, size, false), &self.th.ink, bg);
        (t.width_ch, t.height_ln, t.wrap, t.hl, t.bd) = (width, height, Wrap::Word, if ring { 1 } else { 0 }, 1);
        (t.padx, t.pady) = pad;
        if edit {
            t.edit = Some(0);
        }
        let ring_c = if ring { self.th.line.clone() } else { String::new() };
        ui.text(parent, t, &ring_c)
    }

    /// zk_text_scroll(text): the slim themed scrollbar inside a tall answer box
    fn text_bar(&mut self, ui: &mut Ui, text: usize) {
        let bg = ui.ws[text].bg.clone();
        let sb = ui.canvas(text, &bg, 9, 40);
        canvas_of(ui, sb);
        ui.ws[sb].place = Some(Place { relx: 1.0, x: -1, y: 2, relh: Some(1.0), h: Some(-4), ne: true, ..Default::default() });
        self.bars.push((text, sb));
    }

    /// md_insert(text, s, size=10) into a display-only box
    fn md_show(&mut self, ui: &mut Ui, text: usize, s: &str, size: i32) {
        let (th, f, serif) = (self.th.clone(), self.ui_font.clone(), self.d.serif.clone());
        let t = ui.text_mut(text);
        t.delete_all();
        crate::chatpage::md_tags(t, size, &th.ink, &th, &f, &serif, "Consolas");
        for (sg, tg) in crate::mdtext::md_segments(s, &[]) {
            let tags: Vec<&str> = tg.iter().map(|x| x.as_str()).collect();
            t.insert_end(&sg, &tags);
        }
    }

    /// nb_bar(parent, frac, col, w, h=10)
    fn nb_bar(&mut self, ui: &mut Ui, parent: usize, frac: f64, col: &str, w: i32) -> usize {
        let h = 10;
        let bg = ui.ws[parent].bg.clone();
        let cv = ui.canvas(parent, &bg, w, h + 2);
        let line = self.th.line.clone();
        let c = canvas_of(ui, cv);
        crate::rempage::rbox_pub(c, 1.0, 1.0, (w - 1) as f64, h as f64, 99, &mixhex(&bg, "#000000", 0.25), &line, &[], "pill");
        if frac > 0.0 {
            crate::rempage::rbox_pub(c, 1.0, 1.0, ((h + 2) as f64).max(1.0 + (w - 2) as f64 * frac.min(1.0)), h as f64, 99, col, "",
                                     &[], "pill");
        }
        cv
    }
}

/// "{:,}" of a whole number
fn commas(n: i64) -> String {
    let s = n.abs().to_string();
    let mut out = String::new();
    for (i, ch) in s.chars().enumerate() {
        if i > 0 && (s.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(ch);
    }
    if n < 0 { format!("-{out}") } else { out }
}

fn nearest(img: &zenko_art::raster::Image, z: usize) -> Photo {
    let (w, h) = (img.w * z, img.h * z);
    let mut px = vec![[0u8; 4]; w * h];
    for y in 0..h {
        for x in 0..w {
            px[y * w + x] = img.px[(y / z) * img.w + x / z];
        }
    }
    Photo { w, h, px }
}

impl BookPage {
    /// book_fox(key, state, tf): Zenko teaching, in the book's coat (glasses for the sciences, maths and grades)
    fn fox_img(&mut self, state: &str, tf: i64) -> Rc<Photo> {
        let k = format!("{}|{state}|{tf}|{}", self.key, self.d.tails);
        if let Some(p) = self.imgs.get(&k) {
            return p.clone();
        }
        if self.imgs.len() > 80 {
            self.imgs.clear();
        }
        let acc: Vec<&str> = if matches!(self.key.as_str(), "math" | "chem" | "bio" | "grades") { vec!["glasses", "scarf"] }
                             else { vec!["scarf"] };
        let t = zenko_art::core::build_tails(self.d.tails, tf as i32, 8, false);
        let b = zenko_art::sit::build_frame(state, 0, None, &acc, 0, 0, &self.d.look);
        let pal = book_pal(&self.key, &self.col);
        let img = if self.d.hd {
            let i = zenko_art::raster::sprite_hd(&[&t, &b], &pal, 2, None);
            Photo { w: i.w, h: i.h, px: i.px }
        } else {
            nearest(&zenko_art::raster::sprite_rows(&[&t, &b], &pal, None), 2)
        };
        let p = Rc::new(img);
        self.imgs.insert(k, p.clone());
        p
    }

    /// book_build(host, key) on the page `pg` (zenko17: with the bar of tool windows)
    #[allow(clippy::too_many_arguments)]
    pub fn build(ui: &mut Ui, pg: usize, th: &Theme, ui_font: &str, cm10: i32, d: BookData, rnd: PyRandom) -> BookPage {
        let key = d.key.clone();
        let host = ui.frame(pg, &th.paper);
        ui.pack(host, Pack::default().fill(Fill::Both).expand());
        let b = book(&key).copied().unwrap_or(("", "", "", "", "#c9a24a"));
        let col = d.subjects.iter().find(|s| s.0 == key).map(|s| s.3.clone()).unwrap_or_else(|| b.4.to_string());
        let mut p = BookPage {
            host, key: key.clone(), page: th.card.clone(), col: col.clone(), hdr: 0, rule: 0, inner: 0, left: 0,
            lsc: Scroller { boxf: 0, cv: 0, sb: 0, inner: 0 }, rsc: Scroller { boxf: 0, cv: 0, sb: 0, inner: 0 }, lbody: 0,
            rbody: 0, teach: Teach::default(), prac: Prac::default(), brain: (0, 0), ask: 0, ans: 0, status: 0,
            mode: "explain".into(), modes: vec![], hist: 0, nbwrap: vec![], focus_cv: 0, links: vec![], bars: vec![], rnd,
            imgs: HashMap::new(), wrap: Default::default(), life: d.life.clone(), cfg: d.cfg.clone(), th: th.clone(),
            ui_font: ui_font.into(), cm10, d,
        };
        let (paper, grey, ink, gold) = (th.paper.clone(), th.grey.clone(), th.ink.clone(), th.gold.clone());
        let (jp, serif) = (p.d.jp.clone(), p.d.serif.clone());
        let cover = mixhex(&col, "#000000", 0.45);
        let page = p.page.clone();
        // the header
        let hdr = Self::frame(ui, host, &paper, Pack::default().fill(Fill::X).padx(28, 28).pady(16, 0));
        p.hdr = hdr;
        let back = p.lbl(ui, hdr, "\u{2190}  \u{672c}\u{68da}", &grey, &jp, 10, false, false);
        ui.pack(back, Pack::default().side(Side::Left).anchor(Anchor::S).pady(0, 6));
        p.links.push((back, "open:shelf".into(), grey.clone()));
        let kc = mixhex(&col, "#ffffff", 0.25);
        let k = p.lbl(ui, hdr, b.1, &kc, &jp, 24, false, false);
        ui.pack(k, Pack::default().side(Side::Left).padx(26, 12));
        let tb = Self::frame(ui, hdr, &paper, Pack::default().side(Side::Left).anchor(Anchor::S));
        let l = p.lbl(ui, tb, &spaced(&b.2.to_uppercase()), &grey, &serif, 8, false, false);
        ui.pack(l, Pack::default().anchor(Anchor::W));
        let l = p.lbl(ui, tb, b.3, &ink, &serif, 18, false, false);
        ui.pack(l, Pack::default().anchor(Anchor::W));
        let right = Self::frame(ui, hdr, &paper, Pack::default().side(Side::Right).anchor(Anchor::S).pady(0, 6));
        if is_subject(&key) {
            let wk = p.d.week.get(&key).copied().unwrap_or(0.0);
            let td = p.d.today.get(&key).copied().unwrap_or(0.0);
            let l = p.lbl(ui, right, &format!("Today  {}     This week  {}", fmt_hm(td as i64), fmt_hm(wk as i64)), &grey, &serif,
                          9, false, true);
            ui.pack(l, Pack::default().side(Side::Right));
        }
        let others = Self::frame(ui, right, &paper, Pack::default().side(Side::Right).padx(0, 22));
        for b2 in crate::shelfpage::BOOKS.iter() {
            if b2.0 == key {
                continue;
            }
            let c2 = p.d.subjects.iter().find(|s| s.0 == b2.0).map(|s| s.3.clone()).unwrap_or_else(|| b2.4.to_string());
            let fgc = mixhex(&c2, &paper, 0.15);
            let lab = p.lbl(ui, others, b2.1, &fgc, &jp, 9, false, false);
            ui.ws[lab].padx = 3;
            ui.pack(lab, Pack::default().side(Side::Left));
            p.links.push((lab, format!("open_now:{}", b2.0), fgc));
        }
        // the gold rule
        let rule = ui.canvas(host, &paper, cm10, 10);
        canvas_of(ui, rule);
        ui.pack(rule, Pack::default().fill(Fill::X).padx(28, 28).pady(6, 10));
        p.rule = rule;
        // the book: a slim cover edge, a hairline of gold, two pages with a soft gutter shadow
        let bk = Self::frame(ui, host, &cover, Pack::default().fill(Fill::Both).expand().padx(24, 24).pady(0, 18));
        let goldf = Self::frame(ui, bk, &mixhex("#d4af37", &cover, 0.35), Pack::default().fill(Fill::Both).expand().padx(4, 4).pady(4, 4));
        let inner = Self::frame(ui, goldf, &page, Pack::default().fill(Fill::Both).expand().padx(1, 1).pady(1, 1));
        p.inner = inner;
        let left = ui.frame(inner, &page);
        ui.ws[left].opt_w = 380;
        ui.ws[left].propagate = false;
        ui.pack(left, Pack::default().side(Side::Left).fill(Fill::Y));
        p.left = left;
        for t in [0.30, 0.18, 0.08, 0.10, 0.04] {
            let g = ui.frame(inner, &mixhex(&page, "#000000", t));
            ui.ws[g].opt_w = 2;
            ui.pack(g, Pack::default().side(Side::Left).fill(Fill::Y));
        }
        let rightp = Self::frame(ui, inner, &page, Pack::default().side(Side::Left).fill(Fill::Both).expand());
        let lsc = hub_scroller(ui, left, th, cm10);
        let rsc = hub_scroller(ui, rightp, th, cm10);
        for s in [&lsc, &rsc] {
            ui.ws[s.inner].bg = page.clone();
            ui.ws[s.cv].bg = page.clone();
            ui.ws[s.cv].opt_w = 80;
            ui.ws[s.boxf].bg = page.clone();
        }
        let lbody = Self::frame(ui, lsc.inner, &page, Pack::default().fill(Fill::X).padx(18, 14).pady(16, 22));
        let rbody = Self::frame(ui, rsc.inner, &page, Pack::default().fill(Fill::X).padx(22, 22).pady(12, 26));
        (p.lsc, p.rsc, p.lbody, p.rbody) = (lsc, rsc, lbody, rbody);
        p.teach_build(ui, lbody);
        let pc = p.section(ui, lbody, Some(practice(&key)));
        ui.pack(pc, Pack::default().fill(Fill::X).pady(10, 0));
        match key.as_str() {
            "bio" => p.practice_bio(ui, pc),
            "chem" | "math" => p.drill(ui, pc),
            "french" => p.practice_french(ui, pc),
            "psych" => p.practice_psych(ui, pc),
            "english" => p.practice_english(ui, pc),
            "diary" => p.practice_diary(ui, pc),
            "grades" => p.practice_grades(ui, pc),
            _ => {}
        }
        if is_subject(&key) {
            p.brain_card(ui, lbody);
            p.subject_page(ui, rbody);
        } else if key == "diary" {
            p.overview(ui, rbody);
        } else {
            p.grades_page(ui, rbody);
        }
        // zenko17: the tool windows this book opens
        let tools: &[(&str, &str)] = match key.as_str() {
            "chem" => &[("\u{5143}\u{7d20}  Periodic table \u{2197}", "ptable")],
            "math" => &[("\u{96fb}\u{5353}  Graphing calculator \u{2197}", "calc")],
            "french" => &[("\u{8f9e}\u{66f8}  Dictionary & translator \u{2197}", "french")],
            _ => &[],
        };
        if !tools.is_empty() {
            let bar = ui.frame(host, &paper);
            ui.pack_before(bar, rule, Pack::default().fill(Fill::X).padx(28, 28).pady(0, 6));
            let f = p.ui_font.clone();
            let l = p.lbl(ui, bar, "Opens in its own window:", &grey, &f, 8, false, true);
            ui.pack(l, Pack::default().side(Side::Left));
            for (lab, cmd) in tools {
                let st = th.start.clone();
                let bt = p.btn(ui, bar, lab, &format!("bk:tool:{cmd}"), &st, &ink, 9, 10, 1, true);
                ui.pack(bt, Pack::default().side(Side::Left).padx(8, 0));
            }
        }
        let _ = gold;
        p
    }

    // ---- Zenko at her little blackboard ------------------------------------------------------------------------------

    fn teach_build(&mut self, ui: &mut Ui, parent: usize) {
        let b = book(&self.key).copied().unwrap();
        let (wc, hc) = (326.0, 196.0);
        let page = self.page.clone();
        let cv = ui.canvas(parent, &page, 326, 196);
        ui.pack(cv, Pack::default().anchor(Anchor::W));
        let jpg = self.d.jpg.clone();
        let fox = self.fox_img("happy", 0);
        let (line, f) = (self.th.line.clone(), self.ui_font.clone());
        let c = canvas_of(ui, cv);
        c.line(vec![8.0, hc - 8.0, wc - 8.0, hc - 8.0]).fill(&mixhex(&line, &page, 0.3));
        c.oval(6.0, hc - 14.0, 176.0, hc - 3.0).fill(&mixhex(&page, "#000000", 0.18)).outline("");
        let (bx1, by1, bx2, by2) = (178.0, 14.0, 320.0, 116.0);
        c.rect(bx1 - 5.0, by1 - 5.0, bx2 + 5.0, by2 + 5.0).fill("#8b5a2b").outline("#5c3a1a");
        c.rect(bx1, by1, bx2, by2).fill("#24412f").outline("");
        c.text((bx1 + bx2) / 2.0, by1 + 26.0, b.1).fill("#f2f2e8").font(Font::new(&jpg, 20, true));
        c.text((bx1 + bx2) / 2.0, by1 + 66.0, &board_of(&self.key)).fill("#e9f0c9").font(Font::new(&f, 9, true))
            .wrap(bx2 - bx1 - 12.0).justify("center");
        let board = c.last_id();
        c.rect(bx1 - 2.0, by2 + 5.0, bx2 + 2.0, by2 + 9.0).fill("#6b4226").outline("");
        c.rect(bx1 + 20.0, by2 + 2.0, bx1 + 30.0, by2 + 5.0).fill("#ffffff").outline("");
        c.rect(bx1 + 36.0, by2 + 2.0, bx1 + 44.0, by2 + 5.0).fill("#ffd1dc").outline("");
        c.image(-34.0, hc - 6.0, fox).anchor("sw");
        let fox_id = c.last_id();
        c.line(vec![140.0, 112.0, 188.0, 66.0]).fill("#8b5a2b").width(3.0).cap("round");
        let ptr = c.last_id();
        c.oval(185.0, 63.0, 191.0, 69.0).fill("#f6efdc").outline("");
        let tip = c.last_id();
        let ln = mixhex(&self.th.ink, &page, 0.15);
        let serif = self.d.serif.clone();
        let line_l = self.lbl(ui, parent, &self.teach_line(), &ln, &serif, 10, false, true);
        Self::set_wrap(ui, line_l, 300, true, true);
        ui.pack(line_l, Pack::default().fill(Fill::X).pady(8, 0));
        self.teach = Teach { cv, fox: fox_id, ptr, tip, board, line: line_l, i: 0, talk: self.d.now + 2.5, happy: 0.0 };
        self.teach_step(ui, self.d.now);
    }

    fn teach_line(&self) -> String {
        match self.key.as_str() {
            "diary" => "Kon! This is our \u{65e5}\u{8a18}: your study time, goals and how you're sleeping~".into(),
            "grades" => "Kon! Write your results in here whenever you get them back. I'll keep the trends~".into(),
            k => format!("Kon! Sensei Zenko here. Today in {}: do the practice below, then ask me anything on the right page~",
                         book(k).map(|b| b.1).unwrap_or("")),
        }
    }

    /// teach_say(text, secs, board, happy)
    pub fn teach_say(&mut self, ui: &mut Ui, text: Option<&str>, secs: f64, board: Option<&str>, happy: bool, now: f64) {
        if let Some(t) = text {
            Self::set_text(ui, self.teach.line, &clip(t, 220));
        }
        if let Some(bd) = board {
            let id = self.teach.board;
            if let Some(it) = canvas_of(ui, self.teach.cv).get(id) {
                it.text = clip(bd, 60);
            }
        }
        self.teach.talk = now + secs;
        if happy {
            self.teach.happy = now + 2.0;
        }
    }

    /// teach_step(): her next frame; how long until the next one (s)
    pub fn teach_step(&mut self, ui: &mut Ui, now: f64) -> f64 {
        self.teach.i += 1;
        let i = self.teach.i;
        let st = if now < self.teach.happy {
            if i % 2 != 0 { "giggle" } else { "happy" }
        } else if now < self.teach.talk {
            if i % 2 != 0 { "happy" } else { "idle" }
        } else if i % 17 == 0 {
            "blink"
        } else {
            "idle"
        };
        let img = self.fox_img(st, (i / 2) % 8);
        let wig = if now < self.teach.talk { 3.0 * (i as f64 * 0.9).sin() } else { 1.2 * (i as f64 * 0.3).sin() };
        let t = self.teach.clone();
        let c = canvas_of(ui, t.cv);
        if let Some(it) = c.get(t.fox) {
            it.image = Some(img);
        }
        if let Some(it) = c.get(t.ptr) {
            it.coords = vec![140.0, 112.0, 188.0 + wig, 66.0 - wig];
        }
        if let Some(it) = c.get(t.tip) {
            it.coords = vec![185.0 + wig, 63.0 - wig, 191.0 + wig, 69.0 - wig];
        }
        if now < self.teach.talk { 0.17 } else { 0.26 }
    }

    /// the rule's drawing for its width now
    pub fn draw_rule(&mut self, ui: &mut Ui) {
        let w = ui.ws[self.rule].w as f64;
        let (g, c2) = (mixhex(&self.th.gold, &self.th.paper, 0.45), mixhex(&self.col, "#ffffff", 0.1));
        let c = canvas_of(ui, self.rule);
        *c = Canvas::default();
        c.line(vec![0.0, 5.0, w, 5.0]).fill(&g);
        c.rect(0.0, 3.0, 46.0, 7.0).fill(&c2).outline("");
    }
}

// ---- the practice corners --------------------------------------------------------------------------------------------

impl BookPage {
    /// life["book_state"].setdefault(key, default)
    fn bs(&mut self, key: &str, default: Value) -> &mut Value {
        if !self.life["book_state"].is_object() {
            self.life["book_state"] = json!({});
        }
        let o = self.life["book_state"].as_object_mut().unwrap();
        o.entry(key.to_string()).or_insert(default)
    }

    fn bsi(&mut self, key: &str, k: &str) -> i64 {
        self.life["book_state"][key][k].as_i64().unwrap_or(0)
    }

    fn lab(&self, k: &str) -> usize {
        self.prac.labels[k]
    }

    // 生物 Bio: a syllabus fact, then a cloze quiz on it
    fn practice_bio(&mut self, ui: &mut Ui, c: usize) {
        self.bs("bio", json!({"seen": [], "right": 0}));
        let (gold, grey) = (self.th.gold.clone(), self.th.grey.clone());
        let code = self.p_label(ui, c, "", Some(&gold), 8, true, 320, (0, 4));
        let fact = self.p_label(ui, c, "", None, 10, false, 320, (0, 4));
        let e = self.p_entry(ui, c, 24);
        let fb = self.p_label(ui, c, "", Some(&grey), 8, false, 320, (0, 4));
        let bg = ui.ws[c].bg.clone();
        let row = Self::frame(ui, c, &bg, Pack::default().fill(Fill::X).padx(14, 14).pady(2, 12));
        self.prac.labels = HashMap::from([("code", code), ("fact", fact), ("fb", fb)]);
        self.prac.entry = e;
        let b = self.acc_btn(ui, row, "Quiz me", "bk:bio_check", 9, 10, 1);
        ui.pack(b, Pack::default().side(Side::Left));
        let b = self.field_btn(ui, row, "Next fact", "bk:bio_next", 9, 10, 1);
        ui.pack(b, Pack::default().side(Side::Left).padx(6, 6));
        let b = self.field_btn(ui, row, "Explain more", "bk:bio_explain", 9, 10, 1);
        ui.pack(b, Pack::default().side(Side::Left));
        self.p_label(ui, c, "Facts follow the topic codes of the 2023 Biology guide (your exams in 2027 use it).", Some(&grey), 7, false,
                     320, (0, 10));
        self.bio_pick(ui);
    }

    fn bio_pick(&mut self, ui: &mut Ui) {
        let facts: Vec<Value> = bookgen::data()["BIO_FACTS"].as_array().unwrap().clone();
        let seen: Vec<Value> = {
            let s = self.life["book_state"]["bio"]["seen"].as_array().cloned().unwrap_or_default();
            s[s.len().saturating_sub(30)..].to_vec()
        };
        let pool: Vec<Value> = facts.iter().filter(|f| !seen.contains(&f[2])).cloned().collect();
        let pool = if pool.is_empty() { facts } else { pool };
        let f = self.rnd.choice(&pool).clone();
        let mut ns = seen;
        ns.push(f[2].clone());
        let ns = ns[ns.len().saturating_sub(30)..].to_vec();
        self.life["book_state"]["bio"]["seen"] = Value::Array(ns);
        let (a, b, t) = (sv(&f[0]), sv(&f[1]), sv(&f[2]));
        Self::set_text(ui, self.lab("code"), &format!("{a} \u{b7} {b}"));
        Self::set_text(ui, self.lab("fact"), &t.replace(['[', ']'], ""));
        self.prac.fact = Some(f.clone());
        self.prac.quiz = false;
        ui.ws[self.prac.entry].pack = None;
        if let Kind::Entry { e, .. } = &mut ui.ws[self.prac.entry].kind {
            e.text.clear();
            e.pos = 0;
        }
        Self::set_text(ui, self.lab("fb"), "Read it, then tap 'Quiz me' and I'll hide the key word~");
        let now = self.d.now;
        self.teach_say(ui, Some(&format!("Did you know? {b}")), 3.0, Some(&format!("{a} {b}")), false, now);
    }

    // 化学 / 数学: generated problems with worked solutions
    fn drill(&mut self, ui: &mut Ui, c: usize) {
        let key = self.key.clone();
        self.bs(&key, json!({"right": 0, "tries": 0, "streak": 0, "best": 0}));
        let (gold, grey, ink) = (self.th.gold.clone(), self.th.grey.clone(), self.th.ink.clone());
        let top = self.p_label(ui, c, "", Some(&gold), 8, true, 320, (0, 4));
        let q = self.p_label(ui, c, "", None, 10, false, 320, (0, 4));
        let e = self.p_entry(ui, c, 18);
        let fb = self.p_label(ui, c, "", Some(&grey), 8, false, 320, (0, 4));
        let sol = self.p_label(ui, c, "", Some(&ink), 9, false, 320, (0, 4));
        let bg = ui.ws[c].bg.clone();
        let row = Self::frame(ui, c, &bg, Pack::default().fill(Fill::X).padx(14, 14).pady(2, 6));
        let score = self.p_label(ui, c, "", Some(&grey), 8, false, 320, (0, 4));
        let note = if key == "chem" { "Every problem is new. Answers within about 1% count (3 s.f. is fine)." }
                   else { "Every problem is new. P1 = do it without a calculator; P2 = GDC allowed." };
        self.p_label(ui, c, note, Some(&grey), 7, false, 320, (0, 10));
        self.prac.labels = HashMap::from([("top", top), ("q", q), ("fb", fb), ("sol", sol), ("score", score)]);
        self.prac.entry = e;
        let b = self.acc_btn(ui, row, "Check", "bk:drill_check", 9, 10, 1);
        ui.pack(b, Pack::default().side(Side::Left));
        let b = self.field_btn(ui, row, "Show working", "bk:drill_show", 9, 10, 1);
        ui.pack(b, Pack::default().side(Side::Left).padx(6, 6));
        let b = self.field_btn(ui, row, "New problem", "bk:drill_new", 9, 10, 1);
        ui.pack(b, Pack::default().side(Side::Left));
        self.drill_new(ui);
    }

    fn drill_score(&mut self, ui: &mut Ui) {
        let k = self.key.clone();
        let t = format!("{} / {} right \u{b7} streak {} \u{b7} best streak {}", self.bsi(&k, "right"), self.bsi(&k, "tries"),
                        self.bsi(&k, "streak"), self.bsi(&k, "best"));
        Self::set_text(ui, self.lab("score"), &t);
    }

    fn drill_new(&mut self, ui: &mut Ui) {
        let p = if self.key == "chem" { bookgen::chem_problem(&mut self.rnd) } else { bookgen::math_problem(&mut self.rnd) };
        let tag = match p.p {
            Some(1) => " \u{b7} P1 style".to_string(),
            Some(_) => " \u{b7} P2 style".to_string(),
            None => String::new(),
        };
        Self::set_text(ui, self.lab("top"), &format!("{}{tag}", p.topic));
        Self::set_text(ui, self.lab("q"), &p.q);
        if let Kind::Entry { e, .. } = &mut ui.ws[self.prac.entry].kind {
            e.text.clear();
            e.pos = 0;
        }
        let unit = match &p.unit {
            Some(u) if !u.is_empty() => format!(" (in {u})"),
            _ => String::new(),
        };
        let grey = self.th.grey.clone();
        Self::set_text(ui, self.lab("fb"), &format!("Type your answer{unit}, then Enter."));
        Self::set_fg(ui, self.lab("fb"), &grey);
        Self::set_text(ui, self.lab("sol"), "");
        ui.focus = Some(self.prac.entry);
        let board = p.topic.rsplit(':').next().unwrap_or("").trim().to_string();
        let now = self.d.now;
        self.teach_say(ui, Some("Try this one~"), 2.0, Some(&board), false, now);
        self.prac.prob = Some(p);
        self.prac.done = false;
        self.drill_score(ui);
    }

    // 仏語 French: vocab drill by theme (Leitner boxes)
    fn fr_short(n: &str) -> &str {
        match n {
            "All" => "Tout",
            "Ing\u{e9}niosit\u{e9} humaine" => "Ing\u{e9}niosit\u{e9}",
            "Organisation sociale" => "Org. sociale",
            "Partager la plan\u{e8}te" => "Plan\u{e8}te",
            o => o,
        }
    }

    fn radio_btn(&mut self, ui: &mut Ui, parent: usize, text: &str, on: bool, select: &str, cmd: &str, size: i32, bold: bool,
                 padx: i32, pady: i32) -> usize {
        let (f, field, ink) = (self.ui_font.clone(), self.th.field.clone(), self.th.ink.clone());
        let r = ui.check(parent, text, &field, &ink, Font::new(&f, size, bold), on, select, cmd, 1, 0);
        if let Kind::Check { radio, noind, .. } = &mut ui.ws[r].kind {
            *radio = true;
            *noind = true;
        }
        (ui.ws[r].padx, ui.ws[r].pady) = (padx, pady);
        r
    }

    fn practice_french(&mut self, ui: &mut Ui, c: usize) {
        self.bs("french", json!({"theme": "All", "dir": "en-fr", "right": 0}));
        let bg = ui.ws[c].bg.clone();
        let th_ = Self::frame(ui, c, &bg, Pack::default().fill(Fill::X).padx(14, 14).pady(0, 4));
        let cur = self.life["book_state"]["french"]["theme"].as_str().unwrap_or("All").to_string();
        let mut names = vec!["All".to_string()];
        names.extend(bookgen::data()["FR_VOCAB"].as_object().unwrap().keys().cloned());
        let mut r = 0;
        self.prac.radios.clear();
        for (i, n) in names.iter().enumerate() {
            if i == 0 || i == 3 {
                r = Self::frame(ui, th_, &bg, Pack::default().fill(Fill::X).pady(1, 1));
            }
            let rb = self.radio_btn(ui, r, Self::fr_short(n), *n == cur, "#d65f8f", &format!("bk:fr_theme:{n}"), 8, true, 6, 2);
            ui.pack(rb, Pack::default().side(Side::Left).padx(0, 4));
            self.prac.radios.push((rb, "theme".into(), n.clone()));
        }
        let dcur = self.life["book_state"]["french"]["dir"].as_str().unwrap_or("en-fr").to_string();
        let dr = Self::frame(ui, c, &bg, Pack::default().fill(Fill::X).padx(14, 14).pady(2, 4));
        let gold = self.th.gold.clone();
        for (val, lab) in [("en-fr", "English \u{2192} French"), ("fr-en", "French \u{2192} English")] {
            let rb = self.radio_btn(ui, dr, lab, val == dcur, &gold, &format!("bk:fr_dir:{val}"), 8, false, 6, 1);
            ui.pack(rb, Pack::default().side(Side::Left).padx(0, 4));
            self.prac.radios.push((rb, "dir".into(), val.into()));
        }
        let grey = self.th.grey.clone();
        let prompt = self.p_label(ui, c, "", None, 13, true, 320, (6, 0));
        let hint = self.p_label(ui, c, "", Some(&grey), 8, false, 320, (0, 4));
        let e = self.p_entry(ui, c, 26);
        let fb = self.p_label(ui, c, "", Some(&grey), 9, false, 320, (0, 4));
        let row = Self::frame(ui, c, &bg, Pack::default().fill(Fill::X).padx(14, 14).pady(2, 6));
        let prog = self.p_label(ui, c, "", Some(&grey), 8, false, 320, (0, 10));
        self.prac.labels = HashMap::from([("prompt", prompt), ("hint", hint), ("fb", fb), ("prog", prog)]);
        self.prac.entry = e;
        let b = self.acc_btn(ui, row, "Check", "bk:fr_check", 9, 10, 1);
        ui.pack(b, Pack::default().side(Side::Left));
        let b = self.field_btn(ui, row, "Skip", "bk:fr_new", 9, 10, 1);
        ui.pack(b, Pack::default().side(Side::Left).padx(6, 6));
        self.fr_new(ui);
    }

    fn fr_pool(&self) -> Vec<(String, String, String)> {
        let t = self.life["book_state"]["french"]["theme"].as_str().unwrap_or("All").to_string();
        let mut out = vec![];
        for (n, ws) in bookgen::data()["FR_VOCAB"].as_object().unwrap() {
            if t == "All" || t == *n {
                for w in ws.as_array().unwrap() {
                    out.push((n.clone(), sv(&w[0]), sv(&w[1])));
                }
            }
        }
        out
    }

    fn fr_v(&self, fr: &str) -> i64 {
        self.life["fr_vocab"][fr].as_i64().unwrap_or(0)
    }

    fn fr_progress(&mut self, ui: &mut Ui) {
        let p = self.fr_pool();
        let known = p.iter().filter(|x| self.fr_v(&x.1) >= 3).count();
        let t = self.life["book_state"]["french"]["theme"].as_str().unwrap_or("All").to_string();
        Self::set_text(ui, self.lab("prog"), &format!("Learned {known} / {} words in {} (a word counts after 3 right in a row)", p.len(),
                                                      Self::fr_short(&t)));
    }

    fn fr_new(&mut self, ui: &mut Ui) {
        let p = self.fr_pool();
        let p2: Vec<_> = p.iter().filter(|x| Some(&x.1) != self.prac.last.as_ref()).cloned().collect();
        let p2 = if p2.is_empty() { p } else { p2 };
        let low = p2.iter().map(|x| self.fr_v(&x.1)).min().unwrap_or(0);
        let cand: Vec<_> = p2.iter().filter(|x| self.fr_v(&x.1) <= low + 1).cloned().collect();
        let w = self.rnd.choice(&cand).clone();
        self.prac.last = Some(w.1.clone());
        if self.life["book_state"]["french"]["dir"] == "fr-en" {
            Self::set_text(ui, self.lab("prompt"), &w.1);
            Self::set_text(ui, self.lab("hint"), &format!("{} \u{b7} what does it mean in English?", w.0));
        } else {
            Self::set_text(ui, self.lab("prompt"), &w.2);
            Self::set_text(ui, self.lab("hint"), &format!("{} \u{b7} in French (with le/la if it's a noun)", w.0));
        }
        if let Kind::Entry { e, .. } = &mut ui.ws[self.prac.entry].kind {
            e.text.clear();
            e.pos = 0;
        }
        Self::set_text(ui, self.lab("fb"), "");
        ui.focus = Some(self.prac.entry);
        let now = self.d.now;
        self.teach_say(ui, None, 1.5, Some(&w.0), false, now);
        self.prac.word = Some(w);
        self.prac.done = false;
        self.fr_progress(ui);
    }

    // 心理 Psych: concept lens prompts + a research-methods quiz
    fn practice_psych(&mut self, ui: &mut Ui, c: usize) {
        let bg = ui.ws[c].bg.clone();
        let (gold, grey) = (self.th.gold.clone(), self.th.grey.clone());
        let head = self.p_label(ui, c, "", Some(&gold), 8, true, 320, (0, 4));
        let q = self.p_label(ui, c, "", None, 10, false, 320, (0, 4));
        let hint = self.p_label(ui, c, "", Some(&grey), 8, false, 320, (0, 4));
        let row = Self::frame(ui, c, &bg, Pack::default().fill(Fill::X).padx(14, 14).pady(2, 10));
        self.prac.labels = HashMap::from([("head", head), ("q", q), ("hint", hint)]);
        let b = self.field_btn(ui, row, "New prompt", "bk:psy_new", 9, 10, 1);
        ui.pack(b, Pack::default().side(Side::Left));
        let b = self.acc_btn(ui, row, "Plan it with me", "bk:psy_plan", 9, 10, 1);
        ui.pack(b, Pack::default().side(Side::Left).padx(6, 6));
        self.psy_new(ui);
        let line = self.th.line.clone();
        let hr = ui.frame(c, &line);
        ui.ws[hr].opt_h = 1;
        ui.pack(hr, Pack::default().fill(Fill::X).padx(14, 14).pady(0, 8));
        self.p_label(ui, c, "\u{7814}\u{7a76}\u{6cd5} \u{b7} Research methods quiz", Some(&gold), 9, true, 320, (0, 4));
        let mq = self.p_label(ui, c, "", None, 9, false, 320, (0, 4));
        let opts = Self::frame(ui, c, &bg, Pack::default().fill(Fill::X).padx(14, 14));
        let mfb = self.p_label(ui, c, "", Some(&grey), 8, false, 320, (4, 6));
        self.prac.labels.insert("mq", mq);
        self.prac.labels.insert("mfb", mfb);
        self.prac.opts = opts;
        self.bs("psych", json!({"right": 0, "tries": 0}));
        self.psy_mnew(ui);
        let hr = ui.frame(c, &line);
        ui.ws[hr].opt_h = 1;
        ui.pack(hr, Pack::default().fill(Fill::X).padx(14, 14).pady(4, 8));
        self.p_label(ui, c, "Class practicals (one per context)", Some(&gold), 9, true, 320, (0, 4));
        let pr: Vec<String> = bookgen::data()["PSY_PRACTICALS"].as_array().unwrap().iter()
            .map(|p| format!("\u{2022} {}: {} ({})", sv(&p[0]), sv(&p[1]), sv(&p[2]))).collect();
        self.p_label(ui, c, &pr.join("\n"), None, 8, false, 320, (0, 4));
        self.p_label(ui, c, "Psychology follows the NEW guide (first exams 2027): 6 concepts, 4 contexts, 3 approaches, research \
                             proposal IA.", Some(&grey), 7, false, 320, (2, 10));
    }

    fn psy_new(&mut self, ui: &mut Ui) {
        let p = bookgen::psy_prompt(&mut self.rnd);
        Self::set_text(ui, self.lab("head"), &format!("Concept: {} \u{b7} Context: {}", p.concept, p.context));
        Self::set_text(ui, self.lab("q"), &p.q);
        Self::set_text(ui, self.lab("hint"), &format!("{}: {}", p.concept, p.hint));
        let now = self.d.now;
        self.teach_say(ui, None, 1.5, Some(&p.concept.to_lowercase()), false, now);
        self.prac.psy = Some(p);
    }

    fn psy_mnew(&mut self, ui: &mut Ui) {
        let opts = self.prac.opts;
        for k in ui.ws[opts].kids.clone() {
            ui.destroy(k);
        }
        let (term, dfn, ch) = bookgen::psy_term_quiz(&mut self.rnd);
        Self::set_text(ui, self.lab("mq"), &format!("Which term? \u{201c}{dfn}\u{201d}"));
        let grey = self.th.grey.clone();
        let t = format!("{} / {} right", self.bsi("psych", "right"), self.bsi("psych", "tries"));
        Self::set_text(ui, self.lab("mfb"), &t);
        Self::set_fg(ui, self.lab("mfb"), &grey);
        let bg = ui.ws[opts].bg.clone();
        let mut r = 0;
        for (i, t) in ch.iter().enumerate() {
            if i % 2 == 0 {
                r = Self::frame(ui, opts, &bg, Pack::default().fill(Fill::X).pady(1, 1));
            }
            let b = self.field_btn(ui, r, t, &format!("bk:psy_pick:{t}"), 8, 6, 0);
            ui.pack(b, Pack::default().side(Side::Left).padx(0, 4));
        }
        self.prac.term = Some((term, dfn));
        self.prac.next_at = None;
    }

    // 英語 English: tips + a paragraph checker
    fn practice_english(&mut self, ui: &mut Ui, c: usize) {
        let n = bookgen::data()["ENGLISH_TIPS"].as_array().unwrap().len();
        let tip0 = bookgen::randrange(&mut self.rnd, n);       // (setdefault's default is made either way)
        self.bs("english", json!({"tip": tip0}));
        let bg = ui.ws[c].bg.clone();
        let gold = self.th.gold.clone();
        let cat = self.p_label(ui, c, "", Some(&gold), 8, true, 320, (0, 4));
        let tip = self.p_label(ui, c, "", None, 10, false, 320, (0, 4));
        let row = Self::frame(ui, c, &bg, Pack::default().fill(Fill::X).padx(14, 14).pady(2, 8));
        self.prac.labels = HashMap::from([("cat", cat), ("tip", tip)]);
        let b = self.field_btn(ui, row, "\u{2039}", "bk:en_tip:-1", 9, 8, 1);
        ui.pack(b, Pack::default().side(Side::Left));
        let b = self.field_btn(ui, row, "Next tip \u{203a}", "bk:en_tip:1", 9, 10, 1);
        ui.pack(b, Pack::default().side(Side::Left).padx(6, 6));
        self.en_show(ui, 0);
        let line = self.th.line.clone();
        let hr = ui.frame(c, &line);
        ui.ws[hr].opt_h = 1;
        ui.pack(hr, Pack::default().fill(Fill::X).padx(14, 14).pady(0, 8));
        self.p_label(ui, c, "\u{6dfb}\u{524a} \u{b7} Check my paragraph", Some(&gold), 9, true, 320, (0, 4));
        let grey = self.th.grey.clone();
        self.p_label(ui, c, "Paste one analysis paragraph. I check evidence, technique, effect and purpose.", Some(&grey), 8, false, 320,
                     (0, 4));
        let field = self.th.field.clone();
        let t = self.text_box(ui, c, 6, 80, 9, &field, true, (1, 1), true);
        ui.pack(t, Pack::default().fill(Fill::X).padx(14, 14).pady(2, 4));
        self.text_bar(ui, t);
        self.prac.text = t;
        let fb = self.p_label(ui, c, "", None, 9, false, 320, (0, 4));
        self.prac.labels.insert("fb", fb);
        let r2 = Self::frame(ui, c, &bg, Pack::default().fill(Fill::X).padx(14, 14).pady(2, 12));
        let b = self.acc_btn(ui, r2, "Quick check", "bk:en_check", 9, 10, 1);
        ui.pack(b, Pack::default().side(Side::Left));
        let b = self.field_btn(ui, r2, "Detailed feedback", "bk:en_detail", 9, 10, 1);
        ui.pack(b, Pack::default().side(Side::Left).padx(6, 6));
    }

    fn en_show(&mut self, ui: &mut Ui, d: i64) {
        let tips = bookgen::data()["ENGLISH_TIPS"].as_array().unwrap();
        let n = tips.len() as i64;
        let i = (self.bsi("english", "tip") + d).rem_euclid(n);
        self.life["book_state"]["english"]["tip"] = json!(i);
        let (k, t) = (sv(&tips[i as usize][0]), sv(&tips[i as usize][1]));
        Self::set_text(ui, self.lab("cat"), &format!("{k} \u{b7} tip {} of {n}", i + 1));
        Self::set_text(ui, self.lab("tip"), &t);
        if d != 0 {
            let now = self.d.now;
            self.teach_say(ui, None, 2.0, Some(&k), false, now);
        }
    }

    // 日記 diary: sleep + rest guard
    fn cfg_s(&self, k: &str, dflt: &str) -> String {
        match self.cfg.get(k) {
            None | Some(Value::Null) => dflt.into(),
            Some(Value::String(s)) => s.clone(),
            Some(Value::Number(n)) => match n.as_f64() {
                Some(f) if n.is_f64() => bookgen::py_repr(f),
                _ => n.to_string(),
            },
            Some(Value::Bool(b)) => if *b { "True".into() } else { "False".into() },
            Some(o) => o.to_string(),
        }
    }

    fn practice_diary(&mut self, ui: &mut Ui, c: usize) {
        let bg = ui.ws[c].bg.clone();
        let (ink, field, f, grey) = (self.th.ink.clone(), self.th.field.clone(), self.ui_font.clone(), self.th.grey.clone());
        let on = self.cfg.get("sleep_guard").map(truthy).unwrap_or(true);
        let ck = ui.check(c, "Guard my sleep and stop me burning out", &bg, &ink, Font::new(&f, 9, true), on, &field, "bk:sleep_guard",
                          1, 2);
        ui.pack(ck, Pack::default().fill(Fill::X).padx(12, 12));
        let g = Self::frame(ui, c, &bg, Pack::default().fill(Fill::X).padx(14, 14).pady(4, 4));
        let faces = crate::pagekit::Faces { ui: f.clone(), title: String::new() };
        self.prac.entries.clear();
        for (i, (lab, k, dflt)) in [("Bedtime", "bedtime", "22:30"), ("Wake up", "wake_time", "06:30"),
                                    ("Max study hours a day", "burnout_hours", "6")].iter().enumerate() {
            let l = self.lbl(ui, g, lab, &ink, &f, 9, false, false);
            ui.set_anchor(l, Anchor::W);
            ui.grid(l, Grid { row: i, col: 0, sticky: "w", pady: (2, 2), ..Default::default() });
            let v = self.cfg_s(k, dflt);
            let en = tt_entry(ui, g, &v, 7, &self.th, &faces);
            ui.grid(en, Grid { row: i, col: 1, sticky: "w", padx: (8, 8), pady: (2, 2), ..Default::default() });
            self.prac.entries.push(en);
        }
        self.p_label(ui, c, "30 min before bedtime I start the wind-down. If you're still studying after it, I'll nudge you (sleep is \
                             when your brain files today's revision). Past your daily max, I suggest a rest.", Some(&grey), 8, false, 320,
                     (2, 6));
        let late = self.d.late.clone();
        let cv = ui.canvas(c, &bg, 320, 70);
        ui.pack(cv, Pack::default().padx(14, 14).anchor(Anchor::W));
        let mx = late.iter().map(|x| x.1).fold(30.0f64, f64::max);
        let cc = canvas_of(ui, cv);
        for (i, (dname, m)) in late.iter().enumerate() {
            let x = 8.0 + i as f64 * 44.0;
            let hgt = 40.0 * m / mx;
            cc.rect(x, 46.0 - hgt, x + 26.0, 46.0).fill(if *m < 30.0 { "#7d6cae" } else { "#d9364f" }).outline("");
            cc.text(x + 13.0, 58.0, dname).fill(&grey).font(Font::new(&f, 7, false));
            if *m != 0.0 {
                cc.text(x + 13.0, 40.0 - hgt, &(m.trunc() as i64).to_string()).fill(&ink).font(Font::new(&f, 7, false));
            }
        }
        let n_late = late.iter().filter(|x| x.1 >= 30.0).count();
        let cap = self.cfg_s("burnout_hours", "6").parse::<f64>().unwrap_or(6.0) * 60.0;
        let lt = if n_late > 0 { format!("{n_late} late night{} this week.", if n_late == 1 { "" } else { "s" }) }
                 else { "No late nights~ \u{3048}\u{3089}\u{3044}!".into() };
        let t = format!("Minutes studied after bedtime, last 7 nights. {lt} Today: {} of your {} max.", fmt_hm(self.d.today_study as i64),
                        fmt_hm(cap as i64));
        self.p_label(ui, c, &t, Some(&grey), 8, false, 320, (2, 12));
    }

    // 成績 grades
    fn opt_menu(&mut self, ui: &mut Ui, parent: usize, text: &str, cmd: &str) -> usize {
        let (f, field, ink, line) = (self.ui_font.clone(), self.th.field.clone(), self.th.ink.clone(), self.th.line.clone());
        let m = ui.menubutton(parent, text, cmd, Font::new(&f, 9, false), &field, &ink, &line, 0, self.cm10);
        (ui.ws[m].hl, ui.ws[m].bd) = (0, 0);
        m
    }

    fn practice_grades(&mut self, ui: &mut Ui, c: usize) {
        let bg = ui.ws[c].bg.clone();
        let g = Self::frame(ui, c, &bg, Pack::default().fill(Fill::X).padx(14, 14).pady(0, 4));
        let subs: Vec<(String, String)> = self.d.subjects.iter().map(|s| (s.0.clone(), s.2.clone())).collect();
        let s0 = subs.first().cloned().unwrap_or(("bio".into(), "Bio".into()));
        self.bs("grades", json!({"sid": s0.0, "kind": "MYA"}));
        let bsid = self.life["book_state"]["grades"]["sid"].as_str().unwrap_or("").to_string();
        let sv_ = subs.iter().find(|s| s.0 == bsid).map(|s| s.1.clone()).unwrap_or(s0.1.clone());
        let kv = self.life["book_state"]["grades"]["kind"].as_str().unwrap_or("MYA").to_string();
        let faces = crate::pagekit::Faces { ui: self.ui_font.clone(), title: String::new() };
        let (ink, grey, f) = (self.th.ink.clone(), self.th.grey.clone(), self.ui_font.clone());
        let m1 = self.opt_menu(ui, g, &sv_, "bk:gr_menu:sid");
        let m2 = self.opt_menu(ui, g, &kv, "bk:gr_menu:kind");
        let e1 = tt_entry(ui, g, "", 16, &self.th, &faces);
        let mut r = 0;
        for (lab, w) in [("Subject", m1), ("Type", m2), ("Name (optional)", e1)] {
            let l = self.lbl(ui, g, lab, &ink, &f, 9, false, false);
            ui.set_anchor(l, Anchor::W);
            ui.grid(l, Grid { row: r, col: 0, sticky: "w", pady: (2, 2), ..Default::default() });
            ui.grid(w, Grid { row: r, col: 1, cspan: 3, sticky: "w", padx: (6, 6), pady: (2, 2), ..Default::default() });
            r += 1;
        }
        let l = self.lbl(ui, g, "Score", &ink, &f, 9, false, false);
        ui.set_anchor(l, Anchor::W);
        ui.grid(l, Grid { row: r, col: 0, sticky: "w", pady: (2, 2), ..Default::default() });
        let e2 = tt_entry(ui, g, "", 5, &self.th, &faces);
        ui.grid(e2, Grid { row: r, col: 1, sticky: "w", padx: (6, 2), ..Default::default() });
        let l = self.lbl(ui, g, "out of", &grey, &f, 9, false, false);
        ui.grid(l, Grid { row: r, col: 2, sticky: "w", ..Default::default() });
        let e3 = tt_entry(ui, g, "", 5, &self.th, &faces);
        ui.grid(e3, Grid { row: r, col: 3, sticky: "w", padx: (4, 4), ..Default::default() });
        r += 1;
        let l = self.lbl(ui, g, "IB grade (1-7)", &ink, &f, 9, false, false);
        ui.set_anchor(l, Anchor::W);
        ui.grid(l, Grid { row: r, col: 0, sticky: "w", pady: (2, 2), ..Default::default() });
        let m3 = self.opt_menu(ui, g, "-", "bk:gr_menu:grade");
        ui.grid(m3, Grid { row: r, col: 1, cspan: 3, sticky: "w", padx: (6, 6), ..Default::default() });
        r += 1;
        let l = self.lbl(ui, g, "Date", &ink, &f, 9, false, false);
        ui.set_anchor(l, Anchor::W);
        ui.grid(l, Grid { row: r, col: 0, sticky: "w", pady: (2, 2), ..Default::default() });
        let td = self.d.today_iso.clone();
        let e4 = tt_entry(ui, g, &td, 11, &self.th, &faces);
        ui.grid(e4, Grid { row: r, col: 1, cspan: 3, sticky: "w", padx: (6, 6), ..Default::default() });
        let fb = self.p_label(ui, c, "Fill in a score, an IB grade, or both.", Some(&grey), 8, false, 320, (0, 4));
        self.prac.labels = HashMap::from([("fb", fb)]);
        self.prac.menus = vec![m1, m2, m3];
        self.prac.entries = vec![e1, e2, e3, e4];
        let r2 = Self::frame(ui, c, &bg, Pack::default().fill(Fill::X).padx(14, 14).pady(2, 12));
        let b = self.acc_btn(ui, r2, "\u{8a18}\u{5165} \u{b7} Save result", "bk:gr_save", 9, 12, 1);
        ui.pack(b, Pack::default().side(Side::Left));
    }
}

/// Python truthiness of a JSON value
pub fn truthy(v: &Value) -> bool {
    match v {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::Number(n) => n.as_f64().is_some_and(|f| f != 0.0),
        Value::String(s) => !s.is_empty(),
        Value::Array(a) => !a.is_empty(),
        Value::Object(o) => !o.is_empty(),
    }
}

// ---- the tutor brain card and the right pages --------------------------------------------------------------------------

impl BookPage {
    fn subj(&self, sid: &str) -> Subj {
        self.d.subjects.iter().find(|s| s.0 == sid).cloned().unwrap_or((sid.into(), sid.into(), sid.into(), self.th.gold.clone()))
    }

    fn brain_card(&mut self, ui: &mut Ui, parent: usize) {
        let c = self.section(ui, parent, Some("\u{8ce2}\u{3055} \u{b7} Tutor brain"));
        ui.pack(c, Pack::default().fill(Fill::X).pady(10, 0));
        let (grey, f) = (self.th.grey.clone(), self.ui_font.clone());
        let lab = self.lbl(ui, c, "Checking which free brains you have...", &grey, &f, 8, false, false);
        Self::set_wrap(ui, lab, 320, true, true);
        ui.pack(lab, Pack::default().fill(Fill::X).padx(14, 14).pady(0, 6));
        let bg = ui.ws[c].bg.clone();
        let row = Self::frame(ui, c, &bg, Pack::default().fill(Fill::X).padx(14, 14).pady(0, 10));
        self.brain = (lab, row);
    }

    /// brain_card's done(names, err): which brain answers in this book
    pub fn brains_done(&mut self, ui: &mut Ui, names: &[String]) {
        let (lab, row) = self.brain;
        if lab == 0 {
            return;
        }
        if names.is_empty() {
            Self::set_text(ui, lab, "No smart brain running, so I use my IB notes + free web search + Wikipedia. For full tutor answers \
                                     install Ollama (free) from Settings > Smart brain setup, then come back here.");
            return;
        }
        let key = self.key.clone();
        let on = self.cfg.get("subject_models").map(truthy).unwrap_or(true);
        let wanted = self.cfg_s("llm_model", "");
        let m = subject_model(&key, names, &wanted, on).unwrap_or_default();
        let prefs: Vec<String> = bookgen::data()["SUBJECT_MODELS"][&key].as_array().into_iter().flatten()
            .filter_map(|v| v.as_str().map(String::from)).collect();
        let special = prefs.iter().any(|p| Some(p.as_str()) == m.split(':').next());
        let want = bookgen::data()["SUBJECT_MODEL_PULL"][&key].as_str().map(String::from);
        let mut txt = format!("Using {m}{}. ", if special { " (picked for this subject)" } else { "" });
        if !special && want.is_some() {
            txt += bookgen::data()["SUBJECT_MODEL_WHY"][&key].as_str().unwrap_or("");
        }
        Self::set_text(ui, lab, &txt);
        if let (false, Some(w)) = (special, want) {
            let st = self.th.start.clone();
            let ink = self.th.ink.clone();
            let b = self.btn(ui, row, &format!("Get {w} (free, a few GB)"), &format!("bk:pull:{w}"), &st, &ink, 8, 8, 1, true);
            ui.pack(b, Pack::default().side(Side::Left));
        }
    }

    /// nb_subject_page(body, sid) inside a book
    fn subject_page(&mut self, ui: &mut Ui, body: usize) {
        let sid = self.key.clone();
        let (_, name, short, col) = self.subj(&sid);
        let bg = self.th.card.clone();
        let (gold, grey, ink, field, f) = (self.th.gold.clone(), self.th.grey.clone(), self.th.ink.clone(), self.th.field.clone(),
                                           self.ui_font.clone());
        let serif = self.d.serif.clone();
        let head = self.section(ui, body, None);
        ui.pack(head, Pack::default().fill(Fill::X));
        let r = Self::frame(ui, head, &bg, Pack::default().fill(Fill::X).padx(16, 16).pady(12, 4));
        let l = self.lbl(ui, r, "\u{5bb6}\u{5ead}\u{6559}\u{5e2b} \u{b7} YOUR TUTOR", &mixhex(&gold, &bg, 0.15), &serif, 9, false, false);
        ui.pack(l, Pack::default().side(Side::Left));
        if let Some((title, when, got, need)) = self.d.exam.get(&sid).cloned() {
            let rr = Self::frame(ui, head, &bg, Pack::default().fill(Fill::X).padx(16, 16).pady(0, 4));
            let l = self.lbl(ui, rr, &format!("{} \u{b7} {when} \u{b7} {} of {}", clip(&title, 30), fmt_hm(got as i64), fmt_hm(need as i64)),
                             &ink, &f, 9, false, false);
            ui.set_anchor(l, Anchor::W);
            ui.pack(l, Pack::default().side(Side::Left));
            let b = self.nb_bar(ui, rr, got / need.max(1.0), &col, 200);
            ui.pack(b, Pack::default().side(Side::Right));
        }
        let hl = self.lbl(ui, head, &format!("I answer as an IB {name} tutor: the free smart brain (Ollama) when it's running, plus web \
                                              search, Wikipedia and my own IB notes. Everything stays free."), &grey, &serif, 10, false, true);
        Self::set_wrap(ui, hl, 420, true, true);
        self.nbwrap.push((hl, 34));
        ui.pack(hl, Pack::default().fill(Fill::X).padx(16, 16).pady(0, 12));
        // ask
        let c = self.section(ui, body, Some(&format!("Ask your {short} tutor")));
        ui.pack(c, Pack::default().fill(Fill::X).pady(12, 0));
        let t = self.text_box(ui, c, 4, 20, 10, &field, true, (1, 1), true);
        ui.pack(t, Pack::default().fill(Fill::X).padx(16, 16).pady(0, 6));
        self.ask = t;
        let mr = Self::frame(ui, c, &bg, Pack::default().fill(Fill::X).padx(16, 16));
        self.modes.clear();
        let mode = self.mode.clone();
        for m in bookgen::data()["NB_MODES"].as_array().unwrap() {
            let (k, lab) = (sv(&m[0]), sv(&m[1]));
            let rb = self.radio_btn(ui, mr, &lab, k == mode, &col, &format!("bk:mode:{k}"), 9, true, 10, 3);
            ui.pack(rb, Pack::default().side(Side::Left).padx(0, 6));
            self.modes.push((rb, k));
        }
        let status = self.lbl(ui, c, "", &grey, &f, 8, false, false);
        ui.set_anchor(status, Anchor::W);
        self.status = status;
        let ans = self.text_box(ui, c, 14, 20, 10, &mixhex(&bg, &field, 0.5), false, (10, 8), false);
        self.ans = ans;
        let br = Self::frame(ui, c, &bg, Pack::default().fill(Fill::X).padx(16, 16).pady(8, 4));
        let b = self.btn(ui, br, "Ask", "bk:ask", &self.th.acc.clone(), &self.th.onacc.clone(), 10, 16, 2, true);
        ui.pack(b, Pack::default().side(Side::Left));
        let b = self.field_btn(ui, br, "Clear", "bk:ask_clear", 9, 10, 1);
        ui.pack(b, Pack::default().side(Side::Left).padx(6, 6));
        let sv_row = ui.frame(c, &bg);
        let b = self.field_btn(ui, sv_row, "Save answer as flashcards", "bk:ans_cards", 8, 8, 1);
        ui.pack(b, Pack::default().side(Side::Left));
        ui.pack(status, Pack::default().fill(Fill::X).padx(16, 16));
        ui.pack(ans, Pack::default().fill(Fill::X).padx(16, 16).pady(4, 4));
        ui.pack(sv_row, Pack::default().fill(Fill::X).padx(16, 16).pady(0, 14));
        let hist = self.life["nb_hist"][&sid].as_array().cloned().unwrap_or_default();
        let first = match hist.last() {
            Some(h) => sv(&h["a"]),
            None => format!("Ask anything about {name}~ Try the topic chips below, or paste a past-paper question. Pick 'Like I'm 12' for a \
                             simple version, 'Exam answer' for a model answer with mark points, or 'Test me' for a quiz."),
        };
        self.md_show(ui, ans, &first, 10);
        self.text_bar(ui, ans);
        self.sources_card(ui, body, &sid, &short);
        // topics + tools
        let c3 = self.section(ui, body, Some("Topics"));
        ui.pack(c3, Pack::default().fill(Fill::X).pady(12, 0));
        let tf = Self::frame(ui, c3, &bg, Pack::default().fill(Fill::X).padx(16, 16).pady(0, 6));
        let mut row = 0;
        let topics: Vec<String> = bookgen::data()["SUBJECT_TOPICS"][&sid].as_array().into_iter().flatten().map(sv).collect();
        for (i, tp) in topics.iter().enumerate() {
            if i % 3 == 0 {
                row = Self::frame(ui, tf, &bg, Pack::default().fill(Fill::X).pady(2, 2));
            }
            let b = self.field_btn(ui, row, tp, &format!("bk:topic:{tp}"), 8, 8, 0);
            ui.pack(b, Pack::default().side(Side::Left).padx(0, 5));
        }
        let due = self.d.due.iter().filter(|x| x["subject"] == sid.as_str()).count();
        let allc = self.life["cards"].as_array().map(|a| a.iter().filter(|x| x["subject"] == sid.as_str()).count()).unwrap_or(0);
        let tr = Self::frame(ui, c3, &bg, Pack::default().fill(Fill::X).padx(16, 16).pady(6, 12));
        let st = self.th.start.clone();
        let b = self.btn(ui, tr, "Flashcards from my notes file...", "bk:cards_file", &st, &ink, 9, 10, 1, true);
        ui.pack(b, Pack::default().side(Side::Left));
        let b = self.btn(ui, tr, &format!("Review my {short} mistakes ({due} due)"), "bk:review", &field, &ink, 9, 10, 1, due > 0);
        ui.pack(b, Pack::default().side(Side::Left).padx(6, 6));
        let l = self.lbl(ui, tr, &format!("{allc} {short} card{} saved", if allc == 1 { "" } else { "s" }), &grey, &f, 8, false, false);
        ui.pack(l, Pack::default().side(Side::Left).padx(6, 6));
        // history
        let c4 = self.section(ui, body, Some("Earlier pages"));
        ui.pack(c4, Pack::default().fill(Fill::X).pady(12, 0));
        let hf = Self::frame(ui, c4, &bg, Pack::default().fill(Fill::X).padx(16, 16).pady(0, 12));
        self.hist = hf;
        self.hist_refresh(ui);
    }

    /// nb_hist_refresh(sid)
    pub fn hist_refresh(&mut self, ui: &mut Ui) {
        let hf = self.hist;
        for k in ui.ws[hf].kids.clone() {
            ui.destroy(k);
        }
        self.links.retain(|l| !l.1.starts_with("hist:"));
        let (grey, ink, f) = (self.th.grey.clone(), self.th.ink.clone(), self.ui_font.clone());
        let h = self.life["nb_hist"][&self.key].as_array().cloned().unwrap_or_default();
        if h.is_empty() {
            let l = self.lbl(ui, hf, "Your questions and my answers are saved here, one page per subject~", &grey, &f, 9, false, false);
            ui.set_anchor(l, Anchor::W);
            ui.pack(l, Pack::default().fill(Fill::X));
        }
        let n = h.len();
        for (i, x) in h.iter().enumerate().skip(n.saturating_sub(10)).rev() {
            let when = dbhm(x["t"].as_f64().unwrap_or(0.0));
            let l = self.lbl(ui, hf, &format!("{when} \u{b7} {}", clip(&sv(&x["q"]).replace('\n', " "), 80)), &ink, &f, 9, false, false);
            ui.set_anchor(l, Anchor::W);
            ui.pack(l, Pack::default().fill(Fill::X).pady(1, 1));
            self.links.push((l, format!("hist:{i}"), ink.clone()));
        }
    }

    fn sources_card(&mut self, ui: &mut Ui, body: usize, sid: &str, short: &str) {
        let bg = self.th.card.clone();
        let (grey, ink, field, f, gold) = (self.th.grey.clone(), self.th.ink.clone(), self.th.field.clone(), self.ui_font.clone(),
                                           self.th.gold.clone());
        let c = self.section(ui, body, Some(&format!("\u{8cc7}\u{6599} \u{b7} My {short} sources (notes from your files)")));
        ui.pack(c, Pack::default().fill(Fill::X).pady(12, 0));
        let lab = self.lbl(ui, c, "Add your class notes, slides, PDFs or Word files. I make study notes from them and answer questions using \
                                   only what's in them, with [1] citations, like NotebookLM but free and on your PC.", &grey, &f, 8, false,
                           false);
        Self::set_wrap(ui, lab, 440, true, true);
        ui.pack(lab, Pack::default().fill(Fill::X).padx(16, 16));
        self.nbwrap.push((lab, 34));
        let lf = Self::frame(ui, c, &bg, Pack::default().fill(Fill::X).padx(16, 16).pady(6, 4));
        let items = self.d.sources.clone();
        if items.is_empty() {
            let l = self.lbl(ui, lf, "No files yet~", &grey, &f, 9, false, true);
            ui.set_anchor(l, Anchor::W);
            ui.pack(l, Pack::default().fill(Fill::X));
        }
        for it in &items {
            let r = Self::frame(ui, lf, &bg, Pack::default().fill(Fill::X).pady(1, 1));
            let l = self.lbl(ui, r, &format!("\u{1f4dc} {}", clip(&sv(&it["name"]), 46)), &ink, &f, 9, false, false);
            ui.set_anchor(l, Anchor::W);
            ui.pack(l, Pack::default().side(Side::Left));
            let x = self.lbl(ui, r, "\u{2715}", &grey, &f, 8, false, false);
            ui.pack(x, Pack::default().side(Side::Right));
            self.links.push((x, format!("src_rm:{}", sv(&it["id"])), grey.clone()));
            let l = self.lbl(ui, r, &format!("{} words", commas(it["words"].as_i64().unwrap_or(0))), &grey, &f, 8, false, false);
            ui.pack(l, Pack::default().side(Side::Right).padx(8, 8));
        }
        let br = Self::frame(ui, c, &bg, Pack::default().fill(Fill::X).padx(16, 16).pady(4, 4));
        let on = !items.is_empty();
        let (st, acc, onacc) = (self.th.start.clone(), self.th.acc.clone(), self.th.onacc.clone());
        let b = self.btn(ui, br, "+ Add files...", "bk:src_add", &st, &ink, 9, 10, 1, true);
        ui.pack(b, Pack::default().side(Side::Left));
        let b = self.btn(ui, br, "Make study notes", "bk:src:notes", &acc, &onacc, 9, 10, 1, on);
        ui.pack(b, Pack::default().side(Side::Left).padx(6, 6));
        let b = self.btn(ui, br, "Briefing", "bk:src:brief", &field, &ink, 9, 8, 1, on);
        ui.pack(b, Pack::default().side(Side::Left));
        let b = self.btn(ui, br, "Flashcards", "bk:src:cards", &field, &ink, 9, 8, 1, on);
        ui.pack(b, Pack::default().side(Side::Left).padx(6, 6));
        let qr = Self::frame(ui, c, &bg, Pack::default().fill(Fill::X).padx(16, 16).pady(4, 2));
        let b = self.btn(ui, qr, "Ask my sources", "bk:src:ask", &field, &ink, 9, 8, 1, on);
        ui.pack(b, Pack::default().side(Side::Right).padx(6, 0));
        let faces = crate::pagekit::Faces { ui: f.clone(), title: String::new() };
        let qe = tt_entry(ui, qr, "", 16, &self.th, &faces);
        ui.pack(qe, Pack::default().side(Side::Left).fill(Fill::X).expand());
        let status = self.lbl(ui, c, "", &gold, &f, 8, true, false);
        ui.set_anchor(status, Anchor::W);
        ui.pack(status, Pack::default().fill(Fill::X).padx(16, 16));
        let out = self.text_box(ui, c, 12, 20, 10, &mixhex(&bg, &field, 0.5), false, (10, 8), false);
        ui.pack(out, Pack::default().fill(Fill::X).padx(16, 16).pady(4, 4));
        let last = self.life["src_out"][sid]["text"].as_str().map(String::from);
        let txt = last.clone().unwrap_or_else(|| "Add a file, then press 'Make study notes'. You can also ask things like 'what are the \
                                                  stages of mitosis?' and I'll answer from your files.".into());
        self.md_show(ui, out, &txt, 9);
        self.text_bar(ui, out);
        let sr = Self::frame(ui, c, &bg, Pack::default().fill(Fill::X).padx(16, 16).pady(0, 12));
        let b = self.btn(ui, sr, "Save as Markdown...", "bk:src_export", &field, &ink, 8, 8, 1, last.is_some());
        ui.pack(b, Pack::default().side(Side::Left));
        let b = self.btn(ui, sr, "Cards from these notes", "bk:src_cards", &field, &ink, 8, 8, 1, last.is_some());
        ui.pack(b, Pack::default().side(Side::Left).padx(6, 6));
    }

    /// nb_overview(body, stack=True): the study diary's right page
    fn overview(&mut self, ui: &mut Ui, body: usize) {
        let bg = self.th.card.clone();
        let (paper, grey, ink, field, f, gold) = (self.th.paper.clone(), self.th.grey.clone(), self.th.ink.clone(), self.th.field.clone(),
                                                  self.ui_font.clone(), self.th.gold.clone());
        let cols = Self::frame(ui, body, &paper, Pack::default().fill(Fill::X));
        ui.ws[cols].gw_cols = vec![1];
        ui.ws[cols].gu_cols = vec!["o"];
        let left = ui.frame(cols, &paper);
        ui.grid(left, Grid { row: 0, col: 0, sticky: "nsew", ..Default::default() });
        let right = ui.frame(cols, &paper);
        ui.grid(right, Grid { row: 1, col: 0, sticky: "nsew", pady: (6, 0), ..Default::default() });
        let c = self.section(ui, left, Some("This week, by subject"));
        ui.pack(c, Pack::default().fill(Fill::X));
        let faces = crate::pagekit::Faces { ui: f.clone(), title: String::new() };
        let _ = &faces;
        for (sid, name, _short, col) in self.d.subjects.clone() {
            let r = Self::frame(ui, c, &bg, Pack::default().fill(Fill::X).padx(16, 16).pady(2, 6));
            let top = Self::frame(ui, r, &bg, Pack::default().fill(Fill::X));
            let l = self.lbl(ui, top, &name, &ink, &f, 10, true, false);
            ui.set_anchor(l, Anchor::W);
            ui.pack(l, Pack::default().side(Side::Left));
            let goal = self.life["goals"][&sid].as_f64().unwrap_or(0.0);
            let m = self.d.week.get(&sid).copied().unwrap_or(0.0);
            let gt = if goal != 0.0 { format!(" / {} goal", fmt_hm((goal * 60.0) as i64)) } else { String::new() };
            let l = self.lbl(ui, top, &format!("{}{gt}", fmt_hm(m as i64)), &grey, &f, 9, false, false);
            ui.pack(l, Pack::default().side(Side::Left).padx(8, 8));
            let e = ui.entry(top, 4, Font::new(&f, 9, false), &if goal != 0.0 { pg(goal) } else { String::new() }, &field);
            ui.pack(e, Pack::default().side(Side::Right));
            self.prac.entries.push(e);
            let l = self.lbl(ui, top, "weekly goal (h)", &grey, &f, 8, false, false);
            ui.pack(l, Pack::default().side(Side::Right).padx(4, 4));
            let frac = if goal != 0.0 { m / (goal * 60.0) } else { (m / 600.0).min(1.0) };
            let b = self.nb_bar(ui, r, frac, &col, 300);
            ui.pack(b, Pack::default().anchor(Anchor::W).pady(2, 0));
            if let Some((title, when, got, need)) = self.d.exam_specific.get(&sid).cloned() {
                let l = self.lbl(ui, r, &format!("{} {when}: {} of {} studied", clip(&title, 28), fmt_hm(got as i64), fmt_hm(need as i64)), &grey,
                                 &f, 8, false, false);
                ui.set_anchor(l, Anchor::W);
                ui.pack(l, Pack::default().fill(Fill::X));
            }
        }
        if let Some((title, when, got, need)) = self.d.gen_exam.clone() {
            let l = self.lbl(ui, c, &format!("All subjects count toward \u{201c}{}\u{201d} ({when}): {} of {}", clip(&title, 26),
                                             fmt_hm(got as i64), fmt_hm(need as i64)), &gold, &f, 9, true, false);
            Self::set_wrap(ui, l, 300, true, true);
            self.nbwrap.push((l, 34));
            ui.pack(l, Pack::default().fill(Fill::X).padx(16, 16));
        }
        let l = self.lbl(ui, c, "I count your time automatically from your windows (notes, PDFs, sites). Forgot to? Say \u{201c}I studied bio for \
                                 2 hours\u{201d}.", &grey, &f, 8, false, false);
        Self::set_wrap(ui, l, 300, true, true);
        self.nbwrap.push((l, 34));
        ui.pack(l, Pack::default().fill(Fill::X).padx(16, 16).pady(4, 12));
        // unsorted time
        let mut uns: Vec<(String, Value)> = self.life["unsorted"].as_object().map(|o| o.iter()
            .filter(|(_, u)| u["secs"].as_f64().unwrap_or(0.0) >= 60.0).map(|(k, u)| (k.clone(), u.clone())).collect()).unwrap_or_default();
        let c2 = self.section(ui, left, Some("Time I wasn't sure about"));
        ui.pack(c2, Pack::default().fill(Fill::X).pady(12, 0));
        if uns.is_empty() {
            let l = self.lbl(ui, c2, "Nothing to sort~ I knew what you were studying all day!", &grey, &f, 9, false, false);
            ui.set_anchor(l, Anchor::W);
            ui.pack(l, Pack::default().fill(Fill::X).padx(16, 16).pady(0, 12));
        }
        uns.sort_by(|a, b| b.1["secs"].as_f64().unwrap_or(0.0).partial_cmp(&a.1["secs"].as_f64().unwrap_or(0.0)).unwrap());
        for (k, u) in uns.iter().take(6) {
            let r = Self::frame(ui, c2, &bg, Pack::default().fill(Fill::X).padx(16, 16).pady(0, 8));
            let secs = u["secs"].as_f64().unwrap_or(0.0) as i64;
            let l = self.lbl(ui, r, &format!("\u{201c}{}\u{201d}  \u{b7}  {}", clip(&sv(&u["title"]), 46), fmt_hm(secs / 60)), &ink, &f, 9,
                             false, false);
            ui.set_anchor(l, Anchor::W);
            ui.pack(l, Pack::default().fill(Fill::X));
            let ch = Self::frame(ui, r, &bg, Pack::default().fill(Fill::X).pady(2, 0));
            for (sid, _, short, col) in self.d.subjects.clone() {
                let bgc = if u["guess"] == sid.as_str() { col.clone() } else { field.clone() };
                let b = self.btn(ui, ch, &short, &format!("bk:sort:{k}:{sid}"), &bgc, &ink, 8, 6, 0, true);
                ui.pack(b, Pack::default().side(Side::Left).padx(0, 4));
            }
            let b = self.field_btn(ui, ch, "Not study", &format!("bk:sort:{k}:"), 8, 6, 0);
            ui.pack(b, Pack::default().side(Side::Left));
        }
        // recap
        let c3 = self.section(ui, right, Some("Today so far"));
        ui.pack(c3, Pack::default().fill(Fill::X));
        const END: [&str; 3] = ["Otsukare~ rest well, you did good today!", "Every minute counts. Proud of you~kon!",
                                "Tomorrow we go again, ne~? Sleep well!"];
        let last = if self.d.recap_tot >= 1800.0 { self.rnd.choice(&END).to_string() }
                   else { "Tomorrow's a fresh page. I'll help you start strong~".to_string() };
        let rc = format!("{}\n{last}", self.d.recap);
        let l = self.lbl(ui, c3, &rc, &ink, &f, 9, false, false);
        Self::set_wrap(ui, l, 300, true, true);
        self.nbwrap.push((l, 34));
        ui.pack(l, Pack::default().fill(Fill::X).padx(16, 16).pady(0, 12));
        // best focus times
        let c4 = self.section(ui, right, Some("Your best focus times"));
        ui.pack(c4, Pack::default().fill(Fill::X).pady(12, 0));
        let cv = ui.canvas(c4, &bg, self.cm10, 74);
        canvas_of(ui, cv);
        ui.pack(cv, Pack::default().fill(Fill::X).padx(16, 16));
        self.focus_cv = cv;
        let bt = format!("{}  (orange = study, grey = distracted)", self.d.best_time);
        let l = self.lbl(ui, c4, &bt, &grey, &f, 8, false, false);
        Self::set_wrap(ui, l, 300, true, true);
        self.nbwrap.push((l, 34));
        ui.pack(l, Pack::default().fill(Fill::X).padx(16, 16).pady(2, 12));
        // distraction report
        let c5 = self.section(ui, right, Some("Distraction report"));
        ui.pack(c5, Pack::default().fill(Fill::X).pady(12, 0));
        let dr = self.d.distraction.clone();
        let l = self.lbl(ui, c5, &dr, &ink, &f, 9, false, false);
        Self::set_wrap(ui, l, 300, true, true);
        self.nbwrap.push((l, 34));
        ui.pack(l, Pack::default().fill(Fill::X).padx(16, 16).pady(0, 8));
        let r = Self::frame(ui, c5, &bg, Pack::default().fill(Fill::X).padx(16, 16).pady(0, 12));
        let b = self.field_btn(ui, r, "Edit distracting sites...", "bk:distractions", 9, 10, 1);
        ui.pack(b, Pack::default().side(Side::Left));
        // tracking settings
        let c6 = self.section(ui, right, Some("How I track"));
        ui.pack(c6, Pack::default().fill(Fill::X).pady(12, 0));
        let recap_at = self.cfg_s("recap_time", "21:00");
        for (text, key) in [("Count study time automatically from my windows".to_string(), "auto_track"),
                            ("Ask me with one tap when I'm not sure of the subject".into(), "ask_subject"),
                            ("Weak-subject alerts".into(), "weak_alerts"), (format!("End-of-day recap (at {recap_at})"), "recap"),
                            ("Golden-hour nudges at my best focus time".into(), "golden_hour"),
                            ("Pause the focus timer while I'm away from the keyboard".into(), "idle_pause"),
                            ("Quick mistake flashcard during breaks".into(), "break_cards")] {
            let on = self.cfg.get(key).map(truthy).unwrap_or(true);
            let ck = ui.check(c6, &text, &bg, &ink, Font::new(&f, 9, false), on, &field, &format!("bk:track:{key}"), 0, 2);
            ui.pack(ck, Pack::default().fill(Fill::X).padx(14, 14));
        }
        let r = Self::frame(ui, c6, &bg, Pack::default().fill(Fill::X).padx(16, 16).pady(4, 12));
        let l = self.lbl(ui, r, "Not counted after", &grey, &f, 9, false, false);
        ui.pack(l, Pack::default().side(Side::Left));
        let iv = self.cfg_s("track_idle_minutes", "6");
        let ie = ui.entry(r, 3, Font::new(&f, 9, false), &iv, &field);
        ui.pack(ie, Pack::default().side(Side::Left).padx(4, 4));
        self.prac.entries.push(ie);
        let l = self.lbl(ui, r, "min with no keyboard/mouse (reading time is kept until then)", &grey, &f, 9, false, false);
        ui.pack(l, Pack::default().side(Side::Left));
    }

    /// the best-focus-times bars for the canvas's width now
    pub fn draw_focus(&mut self, ui: &mut Ui) {
        if self.focus_cv == 0 {
            return;
        }
        let w = (ui.ws[self.focus_cv].w as f64).max(200.0);
        let mut agg: HashMap<i64, (f64, f64)> = HashMap::new();
        if let Some(o) = self.life["hourlog"].as_object() {
            for hours in o.values() {
                for (hh, v) in hours.as_object().into_iter().flatten() {
                    let a = agg.entry(hh.parse().unwrap_or(0)).or_default();
                    a.0 += v[0].as_f64().unwrap_or(0.0);
                    a.1 += v[1].as_f64().unwrap_or(0.0);
                }
            }
        }
        let top = agg.values().map(|v| v.0 + v.1).fold(f64::NAN, f64::max);
        let top = if top.is_nan() { 1.0 } else { top };
        let bw = (w - 4.0) / 24.0;
        let (gold, acc, grey, f) = (self.th.gold.clone(), self.th.acc.clone(), self.th.grey.clone(), self.ui_font.clone());
        let best = self.d.best.clone();
        let c = canvas_of(ui, self.focus_cv);
        *c = Canvas::default();
        for h in 0..24i64 {
            let (s, x) = agg.get(&h).copied().unwrap_or((0.0, 0.0));
            let x1 = 2.0 + h as f64 * bw;
            let hs = 50.0 * s / top;
            let hx = 50.0 * x / top;
            c.rect(x1 + 1.0, 56.0 - hs, x1 + bw - 1.0, 56.0).fill(if best.contains(&h) { &gold } else { &acc }).outline("");
            if hx >= 1.0 {
                c.rect(x1 + 1.0, 56.0 - hs - hx, x1 + bw - 1.0, 56.0 - hs).fill(&grey).outline("");
            }
            if h % 3 == 0 {
                c.text(x1 + bw / 2.0, 66.0, &h.to_string()).fill(&grey).font(Font::new(&f, 7, false));
            }
        }
    }

    /// grades_page(body)
    fn grades_page(&mut self, ui: &mut Ui, body: usize) {
        let bg = self.th.card.clone();
        let (grey, ink, f, gold, line) = (self.th.grey.clone(), self.th.ink.clone(), self.ui_font.clone(), self.th.gold.clone(),
                                          self.th.line.clone());
        let jpg = self.d.jpg.clone();
        let head = self.section(ui, body, None);
        ui.pack(head, Pack::default().fill(Fill::X));
        let mut all: Vec<Value> = self.life["grades"].as_array().cloned().unwrap_or_default();
        all.sort_by(|a, b| (sv(&a["date"]), sv(&a["id"])).cmp(&(sv(&b["date"]), sv(&b["id"]))));
        let mut latest: Vec<(String, i64)> = vec![];
        for g in &all {
            if let Some(gr) = g["grade"].as_i64().filter(|&x| x != 0) {
                let sid = sv(&g["sid"]);
                match latest.iter_mut().find(|x| x.0 == sid) {
                    Some(x) => x.1 = gr,
                    None => latest.push((sid, gr)),
                }
            }
        }
        let r = Self::frame(ui, head, &bg, Pack::default().fill(Fill::X).padx(16, 16).pady(12, 12));
        let l = self.lbl(ui, r, "\u{6210}\u{7e3e}\u{8868}", &gold, &jpg, 16, true, false);
        ui.pack(l, Pack::default().side(Side::Left));
        let l = self.lbl(ui, r, "  My results", &ink, &f, 13, true, false);
        ui.pack(l, Pack::default().side(Side::Left));
        if !latest.is_empty() {
            let tot: i64 = latest.iter().map(|x| x.1).sum();
            let n = latest.len();
            let l = self.lbl(ui, r, &format!("latest IB grades: {tot} / {} from {n} subject{}", 7 * n, if n == 1 { "" } else { "s" }), &grey,
                             &f, 9, false, false);
            ui.pack(l, Pack::default().side(Side::Right));
        }
        if all.is_empty() {
            let l = self.lbl(ui, head, "No results yet. Add one on the left page, or tell me in chat: 'I got 34/40 on my bio paper 1' or 'got a \
                                        6 in chem MYA'.", &grey, &f, 9, false, false);
            Self::set_wrap(ui, l, 560, true, true);
            ui.pack(l, Pack::default().fill(Fill::X).padx(16, 16).pady(0, 12));
        }
        for (sid, name, _, col) in self.d.subjects.clone() {
            let gl: Vec<&Value> = all.iter().filter(|g| g["sid"] == sid.as_str()).collect();
            let c = self.section(ui, body, None);
            ui.pack(c, Pack::default().fill(Fill::X).pady(10, 0));
            let hr = Self::frame(ui, c, &bg, Pack::default().fill(Fill::X).padx(16, 16).pady(10, 4));
            let l = self.lbl(ui, hr, "\u{25a0}", &col, &f, 12, false, false);
            ui.pack(l, Pack::default().side(Side::Left));
            let l = self.lbl(ui, hr, &name, &ink, &f, 11, true, false);
            ui.pack(l, Pack::default().side(Side::Left).padx(6, 0));
            let pcts: Vec<f64> = gl.iter().filter(|g| g["score"].as_f64().is_some() && g["out_of"].as_f64().is_some_and(|o| o != 0.0))
                .map(|g| 100.0 * g["score"].as_f64().unwrap() / g["out_of"].as_f64().unwrap()).collect();
            if !pcts.is_empty() {
                let mut trend = String::new();
                if pcts.len() >= 2 {
                    let dd = pcts[pcts.len() - 1] - pcts[pcts.len() - 2];
                    trend = if dd > 0.5 { format!("  \u{2191} {}%", dd.round_ties_even() as i64) }
                            else if dd < -0.5 { format!("  \u{2193} {}%", (-dd).round_ties_even() as i64) }
                            else { "  \u{2192} steady".into() };
                }
                let avg = (pcts.iter().sum::<f64>() / pcts.len() as f64).round_ties_even() as i64;
                let l = self.lbl(ui, hr, &format!("avg {avg}%{trend}"), &grey, &f, 9, false, false);
                ui.pack(l, Pack::default().side(Side::Right));
            }
            if let Some(gr) = latest.iter().find(|x| x.0 == sid) {
                let l = self.lbl(ui, hr, &format!("grade {}  ", gr.1), &gold, &f, 10, true, false);
                ui.pack(l, Pack::default().side(Side::Right));
            }
            if pcts.len() >= 2 {
                let cv = ui.canvas(c, &bg, 300, 46);
                ui.pack(cv, Pack::default().anchor(Anchor::W).padx(16, 16));
                let last: Vec<f64> = pcts[pcts.len().saturating_sub(12)..].to_vec();
                let mut pts = vec![];
                for (i, p) in last.iter().enumerate() {
                    pts.push(6.0 + i as f64 * (288.0 / (last.len() as f64 - 1.0).max(1.0)));
                    pts.push(40.0 - 34.0 * p / 100.0);
                }
                let cc = canvas_of(ui, cv);
                cc.line(vec![6.0, 40.0, 294.0, 40.0]).fill(&line);
                cc.line(pts.clone()).fill(&col).width(2.0);
                for i in (0..pts.len()).step_by(2) {
                    cc.oval(pts[i] - 2.0, pts[i + 1] - 2.0, pts[i] + 2.0, pts[i + 1] + 2.0).fill(&col).outline("");
                }
            }
            if gl.is_empty() {
                let l = self.lbl(ui, c, "nothing yet", &grey, &f, 8, false, false);
                ui.set_anchor(l, Anchor::W);
                ui.pack(l, Pack::default().fill(Fill::X).padx(16, 16).pady(0, 10));
                continue;
            }
            let lf = Self::frame(ui, c, &bg, Pack::default().fill(Fill::X).padx(16, 16).pady(2, 10));
            for g in gl.iter().skip(gl.len().saturating_sub(8)).rev() {
                let row = Self::frame(ui, lf, &bg, Pack::default().fill(Fill::X).pady(1, 1));
                let sc = match (g["score"].as_f64(), g["out_of"].as_f64()) {
                    (Some(s), Some(o)) => format!("{}/{} ({}%)", pg(s), pg(o), (100.0 * s / o).round_ties_even() as i64),
                    _ => String::new(),
                };
                let gr = g["grade"].as_i64().filter(|&x| x != 0);
                let gtxt = gr.map(|x| format!("grade {x}")).unwrap_or_default();
                let when = dby(&sv(&g["date"]));
                let nm = sv(&g["name"]);
                let l = self.lbl(ui, row, &format!("{when} \u{b7} {}{}", sv(&g["kind"]), if nm.is_empty() { String::new() }
                                                                               else { format!(" \u{b7} {nm}") }), &ink, &f, 9, false, false);
                ui.set_anchor(l, Anchor::W);
                ui.pack(l, Pack::default().side(Side::Left));
                let x = self.lbl(ui, row, "\u{2715}", &grey, &f, 8, false, false);
                ui.pack(x, Pack::default().side(Side::Right).padx(8, 0));
                self.links.push((x, format!("gr_del:{}", sv(&g["id"])), grey.clone()));
                let t: Vec<&str> = [sc.as_str(), gtxt.as_str()].into_iter().filter(|s| !s.is_empty()).collect();
                let l = self.lbl(ui, row, &t.join("  "), if gr.is_some() { &gold } else { &ink }, &f, 9, true, false);
                ui.pack(l, Pack::default().side(Side::Right));
            }
        }
    }
}

// ---- keeping a book page inside its paper ------------------------------------------------------------------------------

impl BookPage {
    fn canvas_above(ui: &Ui, mut id: usize) -> Option<usize> {
        while let Some(p) = ui.ws[id].parent {
            if matches!(ui.ws[p].kind, Kind::Canvas { card: None, .. }) {
                return Some(p);
            }
            id = p;
        }
        None
    }

    /// nb_wrap(lab, margin): the label wraps to its card's width (never wider than the visible page)
    fn nb_wrap_pass(&mut self, ui: &mut Ui) {
        for &(l, margin) in &self.nbwrap {
            let Some(par) = ui.ws[l].parent else { continue };
            let mut w = ui.ws[par].w;
            if let Some(cv) = Self::canvas_above(ui, l)
                && ui.ws[cv].w > 50 {
                w = w.min(ui.ws[cv].w - 0.max(ui.abs(par).0 - ui.abs(cv).0) - 4);
            }
            if let Kind::Label { wrap, .. } = &mut ui.ws[l].kind {
                *wrap = 160.max(w - margin);
            }
        }
    }

    /// book_fit(body, cv): a row of buttons that is too wide breaks onto a second line, a long label wraps
    fn book_fit(ui: &mut Ui, body: usize, cv: usize) -> bool {
        let right = ui.abs(cv).0 + ui.ws[cv].w - 6;
        let room = |ui: &Ui, w: usize| {
            let r = right - ui.abs(w).0;
            if ui.ws[w].w > 1 { r.min(ui.ws[w].w) } else { r }
        };
        fn walk(ui: &mut Ui, w: usize, room: &dyn Fn(&Ui, usize) -> i32, changed: &mut bool) {
            for ch in ui.ws[w].kids.clone() {
                if !ui.ws[ch].mapped && ui.ws[ch].pack.is_none() && ui.ws[ch].grid.is_none() {
                    continue;
                }
                let rm = room(ui, ch);
                match &ui.ws[ch].kind {
                    Kind::Label { text, image: None, .. } => {
                        if ui.ws[ch].rw > rm + 2 && text.chars().count() > 30 {
                            if let Kind::Label { wrap, left, .. } = &mut ui.ws[ch].kind {
                                *wrap = 150.max(rm - 4);
                                *left = true;
                            }
                            *changed = true;
                        }
                    }
                    Kind::Frame => {
                        if ui.ws[ch].rw > rm + 2 {
                            let kids: Vec<usize> = ui.ws[ch].kids.iter().copied().filter(|&k| ui.ws[k].pack.is_some()).collect();
                            if kids.len() >= 2 && kids.iter().all(|&k| ui.ws[k].pack.as_ref().unwrap().side == Side::Left) {
                                BookPage::reflow(ui, kids, rm);
                                *changed = true;
                                continue;
                            }
                        }
                        walk(ui, ch, room, changed);
                    }
                    _ => {}
                }
            }
        }
        let mut changed = false;
        walk(ui, body, &room, &mut changed);
        changed
    }

    /// book_reflow(row, kids, avail): the packed row becomes a wrapped grid
    fn reflow(ui: &mut Ui, kids: Vec<usize>, avail: i32) {
        let mut info = vec![];
        let mut kids = kids;
        kids.sort_by_key(|&k| ui.ws[k].pack_seq);
        for k in kids {
            let p = ui.ws[k].pack.take().unwrap();
            info.push((k, p.padx, p.pady));
        }
        let (mut r, mut c, mut x) = (0usize, 0usize, 0);
        for (k, px, py) in info {
            let w = ui.ws[k].rw + px.0 + px.1;
            if c > 0 && x + w > avail {
                (r, c, x) = (r + 1, 0, 0);
            }
            ui.grid(k, Grid { row: r, col: c, sticky: "w", padx: px, pady: if r == 0 { py } else { (4, 0) }, ..Default::default() });
            c += 1;
            x += w;
        }
    }

    /// what Tk has settled to by the time the page is seen: the fonts fitted to the window, the left page's width
    /// from the book's, the rule and the focus bars drawn, nb_wrap, book_fit, zk_wrap and the scrollbars
    pub fn fit(&mut self, ui: &mut Ui, fonts: &dyn crate::ui::Fonts, pg: usize, zk_f: f64, pw: i32, ph: i32) {
        crate::pagekit::zk_fonts(ui, fonts, pg, zk_f);
        ui.layout_at(fonts, pw, ph);
        let iw = ui.ws[self.inner].w;
        ui.ws[self.left].opt_w = 360.max(440.min((iw as f64 * 0.42) as i32));
        ui.layout_at(fonts, pw, ph);
        self.draw_rule(ui);
        self.draw_focus(ui);
        self.nb_wrap_pass(ui);
        ui.layout_at(fonts, pw, ph);
        for _ in 0..3 {
            let a = Self::book_fit(ui, self.lbody, self.lsc.cv);
            let b = Self::book_fit(ui, self.rbody, self.rsc.cv);
            ui.layout_at(fonts, pw, ph);
            if !a && !b {
                break;
            }
        }
        crate::pagekit::zk_wrap(ui, fonts, pg, &mut self.wrap, pw, ph);
        ui.layout_at(fonts, pw, ph);
        self.nb_wrap_pass(ui);
        ui.layout_at(fonts, pw, ph);
        let a = self.lsc.sync(ui);
        let b = self.rsc.sync(ui);
        if a || b {
            ui.layout_at(fonts, pw, ph);
        }
        self.draw_focus(ui);
    }
}

// ---- what a book shows, from the app ----------------------------------------------------------------------------------

impl crate::Companion {
    pub fn book_data(&mut self, key: &str) -> BookData {
        let life = self.life_now();
        let ws = self.week_start();
        let week: HashMap<String, f64> = self.subject_minutes(1, Some(ws)).into_iter().collect();
        let today: HashMap<String, f64> = self.subject_minutes(1, None).into_iter().collect();
        let raw = crate::watch::data().raw["SUBJECTS"].as_array().cloned().unwrap_or_default();
        let subjects: Vec<Subj> = SUBJ.iter().map(|(sid, short)| {
            let col = raw.iter().find(|s| s[0] == *sid).and_then(|s| s[3].as_str()).map(String::from)
                .unwrap_or_else(|| book(sid).map(|b| b.4).unwrap_or("#c9a24a").to_string());
            (sid.to_string(), self.subject_full(sid), short.to_string(), col)
        }).collect();
        let nl = self.nl();
        let td = crate::plan::day_n(nl);
        let mut exam = HashMap::new();
        let mut exam_specific = HashMap::new();
        for (sid, ..) in &subjects {
            for (spec, m) in [(false, &mut exam), (true, &mut exam_specific)] {
                if let Some(ex) = self.exam_for(sid, spec) {
                    let (got, need) = self.exam_progress(&ex);
                    m.insert(sid.clone(), (sv(&ex["title"]), crate::study::when_text(ex["date"].as_str().unwrap_or(""), td), got, need));
                }
            }
        }
        let gen_exam = self.upcoming_deadlines(6).into_iter().find(|d| truthy(&d["id"])
            && crate::watch::classify_subject(d["title"].as_str().unwrap_or(""), &Value::Null).0.is_none()
            && crate::plan::PLAN_KINDS.contains(&d["kind"].as_str().unwrap_or("Deadline")))
            .map(|d| {
                let (got, need) = self.exam_progress(&d);
                (sv(&d["title"]), crate::study::when_text(d["date"].as_str().unwrap_or(""), td), got, need)
            });
        const WD: [&str; 7] = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];
        let late = self.late_minutes(&life, 7).into_iter().map(|(d, m)| (WD[(d + 3).rem_euclid(7) as usize].to_string(), m)).collect();
        let today_study = self.today_study_minutes(&life, None);
        let sources = match crate::read_json(&format!("sources/{key}.json")) {
            Value::Array(a) => a,
            _ => vec![],
        };
        let recap = self.recap_text(td);
        let recap = recap.rsplit_once('\n').map(|x| x.0.to_string()).unwrap_or(recap);
        let recap_tot = life["slog"][crate::plan::iso(td).as_str()].as_object()
            .map_or(0.0, |o| o.values().map(|v| v.as_f64().unwrap_or(0.0)).sum());
        BookData {
            key: key.into(), subjects, week, today, life, cfg: self.cfg.raw.clone(), jp: "Yu Mincho".into(), serif: "Yu Mincho".into(),
            jpg: "Yu Gothic UI".into(), hd: self.cfg.b("hd_art", true), tails: self.tail_count(crate::unix_now()),
            look: self.look_ref().clone(), due: self.cards_due(), exam, exam_specific, gen_exam, recap, recap_tot,
            best_time: self.best_time_text(), best: self.best_hours().iter().take(2).map(|x| x.0).collect(),
            distraction: self.distraction_report(7), late, today_study, sources, now: crate::unix_now(),
            today_iso: crate::plan::iso(td),
        }
    }
}

// ---- clicks, keys and time --------------------------------------------------------------------------------------------

impl BookPage {
    fn entry_text(ui: &Ui, id: usize) -> String {
        match &ui.ws[id].kind {
            Kind::Entry { e, .. } => String::from_utf16_lossy(&e.text),
            _ => String::new(),
        }
    }

    fn set_entry(ui: &mut Ui, id: usize, s: &str) {
        if let Kind::Entry { e, .. } = &mut ui.ws[id].kind {
            e.text = s.encode_utf16().collect();
            e.pos = e.text.len();
        }
    }

    /// nb_mistake(card, ok, sid): a wrong answer goes into the deck (srs_update) tagged with the subject
    fn mistake(&mut self, q: &str, a: &str, sid: &str) -> Act {
        Act::Run(format!("nb_mistake\t{sid}\t{q}\t{a}"))
    }

    /// book_ask(q, mode): the question goes into the tutor's box and is asked
    fn book_ask(&mut self, ui: &mut Ui, q: &str, mode: Option<&str>) -> Vec<Act> {
        if self.ask == 0 {
            return vec![];
        }
        ui.text_mut(self.ask).delete_all();
        ui.text_mut(self.ask).insert_end(q, &[]);
        if let Some(m) = mode {
            self.set_mode(ui, m);
        }
        self.do_ask(ui)
    }

    fn set_mode(&mut self, ui: &mut Ui, m: &str) {
        self.mode = m.into();
        for (rb, k) in &self.modes {
            if let Kind::Check { on, .. } = &mut ui.ws[*rb].kind {
                *on = k == m;
            }
        }
    }

    fn do_ask(&mut self, ui: &mut Ui) -> Vec<Act> {
        let q = ui.text_mut(self.ask).get().trim().to_string();
        if q.is_empty() {
            Self::set_text(ui, self.status, "Type a question first~");
            return vec![];
        }
        let now = self.d.now.max(crate::unix_now());
        self.teach_say(ui, Some("Fumu fumu~ let me think about that..."), 30.0, Some(&clip(&q, 50)), false, now);
        vec![Act::Ask(self.key.clone(), q, self.mode.clone())]
    }

    /// the tutor's answer arrived (nb_ask's done): it shows, the history list grows, she reads out the first sentence
    pub fn answer(&mut self, ui: &mut Ui, txt: &str, now: f64) {
        if self.ans != 0 {
            self.md_show(ui, self.ans, txt, 10);
            Self::set_text(ui, self.status, "");
        }
        self.hist_refresh(ui);
        let flat = txt.replace('\n', " ").replace("Kon! ", "");
        let first = fancy_regex::Regex::new(r"(?<=[.!?])\s").unwrap().split(&flat).next().and_then(|r| r.ok()).unwrap_or("").to_string();
        let secs = 9.0f64.min(2.5 + txt.chars().count() as f64 / 220.0);
        self.teach_say(ui, Some(if first.is_empty() { "There~" } else { &first }), secs, None, false, now);
    }

    /// a click on one of the page's labels (the shelf, another book, an earlier page, a ✕)
    pub fn link(&mut self, ui: &mut Ui, id: usize) -> Vec<Act> {
        let Some((_, what, _)) = self.links.iter().find(|l| l.0 == id).cloned() else { return vec![] };
        if what == "open:shelf" {
            return vec![Act::Open("shelf".into(), true)];
        }
        if let Some(k) = what.strip_prefix("open_now:") {
            return vec![Act::Open(k.into(), false)];
        }
        if let Some(i) = what.strip_prefix("hist:") {
            let a = self.life["nb_hist"][&self.key][i.parse::<usize>().unwrap_or(0)]["a"].as_str().unwrap_or("").to_string();
            self.md_show(ui, self.ans, &a, 10);
            Self::set_text(ui, self.status, "");
            return vec![];
        }
        if let Some(gid) = what.strip_prefix("gr_del:") {
            if let Some(a) = self.life["grades"].as_array_mut() {
                a.retain(|g| g["id"] != gid);
            }
            return vec![Act::Save, Act::Render];
        }
        if let Some(sid) = what.strip_prefix("src_rm:") {
            return vec![Act::Run(format!("src_remove\t{}\t{sid}", self.key)), Act::Render];
        }
        vec![]
    }

    /// the pointer over a link: GOLD, back to its colour when it leaves
    pub fn hover(&mut self, ui: &mut Ui, over: Option<usize>) -> bool {
        let mut changed = false;
        let gold = self.th.gold.clone();
        for (id, _, col) in self.links.clone() {
            let want = if Some(id) == over { gold.as_str() } else { col.as_str() };
            if let Kind::Label { fg, .. } = &mut ui.ws[id].kind
                && fg != want {
                *fg = want.into();
                changed = true;
            }
        }
        changed
    }

    pub fn is_link(&self, id: usize) -> bool {
        self.links.iter().any(|l| l.0 == id)
    }

    /// Return in an entry of the page (the practice answers, the diary's times, the goals); true when it was ours
    pub fn key_return(&mut self, ui: &mut Ui, id: usize) -> Option<Vec<Act>> {
        if id == self.prac.entry && id != 0 {
            let c = match self.key.as_str() {
                "bio" => "bk:bio_check",
                "chem" | "math" => "bk:drill_check",
                "french" => "bk:fr_check",
                _ => return None,
            };
            return Some(self.command(ui, c));
        }
        if self.prac.entries.contains(&id) {
            return Some(self.entry_done(ui, id));
        }
        None
    }

    /// an entry's Return / FocusOut: the diary's times and daily max, a weekly goal, the idle minutes
    pub fn entry_done(&mut self, ui: &mut Ui, id: usize) -> Vec<Act> {
        let Some(i) = self.prac.entries.iter().position(|&e| e == id) else { return vec![] };
        let val = Self::entry_text(ui, id).trim().to_string();
        if self.key == "diary" && i < 3 {
            let (k, dflt) = [("bedtime", "22:30"), ("wake_time", "06:30"), ("burnout_hours", "6")][i];
            let back = self.cfg_s(k, dflt);
            if k == "burnout_hours" {
                let Ok(v) = val.parse::<f64>() else {
                    Self::set_entry(ui, id, &back);
                    return vec![];
                };
                let v = v.clamp(1.0, 16.0);
                Self::set_entry(ui, id, &bookgen::py_repr(v));
                self.cfg[k] = json!(v);
                return vec![Act::Cfg(k.into(), json!(v))];
            }
            let m = regex::Regex::new(r"^(\d{1,2})[:.]?(\d{2})$").unwrap().captures(&val)
                .map(|m| (m[1].parse::<i64>().unwrap(), m[2].parse::<i64>().unwrap()));
            let Some((h, mm)) = m.filter(|&(h, mm)| h <= 23 && mm <= 59) else {
                Self::set_entry(ui, id, &back);
                return vec![];
            };
            let v = format!("{h:02}:{mm:02}");
            Self::set_entry(ui, id, &v);
            self.cfg[k] = json!(v);
            return vec![Act::Cfg(k.into(), json!(v))];
        }
        if self.key == "diary" {
            let j = i - 3;
            if j < self.d.subjects.len() {
                let v = if val.is_empty() { Some(0.0) } else { val.parse::<f64>().ok() };
                let Some(v) = v else { return vec![] };
                let sid = self.d.subjects[j].0.clone();
                if !self.life["goals"].is_object() {
                    self.life["goals"] = json!({});
                }
                self.life["goals"][&sid] = json!(v.clamp(0.0, 80.0));
                return vec![Act::Save, Act::Render];
            }
            if let Ok(v) = val.parse::<i64>() {
                let v = v.clamp(2, 30);
                return vec![Act::Cfg("track_idle_minutes".into(), json!(v))];
            }
        }
        vec![]
    }

    /// a button, radio or checkbox of the book ("bk:...")
    pub fn command(&mut self, ui: &mut Ui, c: &str) -> Vec<Act> {
        let now = self.d.now.max(crate::unix_now());
        self.d.now = now;
        let Some(c) = c.strip_prefix("bk:") else { return vec![] };
        let (gold, grey) = (self.th.gold.clone(), self.th.grey.clone());
        let key = self.key.clone();
        match c {
            "bio_next" => {
                self.bio_pick(ui);
                vec![Act::Save]
            }
            "bio_check" => {
                let Some(f) = self.prac.fact.clone() else { return vec![] };
                let txt = sv(&f[2]);
                if !self.prac.quiz {
                    self.prac.quiz = true;
                    Self::set_text(ui, self.lab("fact"), &cloze(&txt));
                    let e = self.prac.entry;
                    ui.pack_before(e, self.lab("fb"), Pack::default().anchor(Anchor::W).padx(14, 14).pady(2, 4));
                    ui.focus = Some(e);
                    Self::set_text(ui, self.lab("fb"), "Type the missing word(s), then Enter.");
                    return vec![];
                }
                let want = regex::Regex::new(r"\[(.*?)\]").unwrap().captures(&txt).map(|m| m[1].to_string()).unwrap_or_default();
                let got = Self::entry_text(ui, self.prac.entry).trim().to_string();
                let (g_, w_) = (bookgen::bio_norm(&got), bookgen::bio_norm(&want));
                if g_.is_empty() {
                    return vec![];
                }
                let ok = g_ == w_ || (g_.chars().count() > 3 && w_.starts_with(&g_))
                    || (w_.chars().count() > 5 && crate::pydifflib::ratio(&g_, &w_) > 0.85);
                let mut acts = vec![];
                if ok {
                    let r = self.bsi("bio", "right") + 1;
                    self.life["book_state"]["bio"]["right"] = json!(r);
                    Self::set_text(ui, self.lab("fb"), &format!("\u{6b63}\u{89e3}! Correct~ ({r} right so far)"));
                    let good = if self.th.good.is_empty() { gold.clone() } else { self.th.good.clone() };
                    Self::set_fg(ui, self.lab("fb"), &good);
                    self.teach_say(ui, Some(&format!("Sugoi! That's straight from {}.", sv(&f[0]))), 2.5, None, true, now);
                    if self.cfg.get("book_sounds").map(truthy).unwrap_or(false) {
                        acts.push(Act::Run("sound:happy".into()));
                    }
                } else {
                    Self::set_text(ui, self.lab("fb"), &format!("Not quite: it's \u{201c}{want}\u{201d}. I saved it to your Bio deck~"));
                    Self::set_fg(ui, self.lab("fb"), &grey);
                    acts.push(self.mistake(&format!("{} {}: {}", sv(&f[0]), sv(&f[1]), cloze(&txt)), &want, "bio"));
                    self.teach_say(ui, Some(&format!("Close! The word was {want}.")), 3.0, None, false, now);
                }
                Self::set_text(ui, self.lab("fact"), &txt.replace(['[', ']'], ""));
                self.prac.quiz = false;
                acts.push(Act::Save);
                acts
            }
            "bio_explain" => {
                let Some(f) = self.prac.fact.clone() else { return vec![] };
                self.book_ask(ui, &format!("Explain {} ({})", sv(&f[1]), sv(&f[0])), None)
            }
            "drill_new" => {
                self.drill_new(ui);
                vec![]
            }
            "drill_show" => {
                if let Some(p) = &self.prac.prob {
                    let s = p.sol.clone();
                    Self::set_text(ui, self.lab("sol"), &s);
                }
                vec![]
            }
            "drill_check" => {
                if self.prac.done {
                    self.drill_new(ui);
                    return vec![];
                }
                let Some(p) = self.prac.prob.clone() else { return vec![] };
                let Some(got) = bookgen::num_answer(&Self::entry_text(ui, self.prac.entry)) else {
                    Self::set_text(ui, self.lab("fb"), "Type a number (like 0.25, 3/4, 2.5e-3)~");
                    return vec![];
                };
                self.prac.done = true;
                macro_rules! bs { ($k:expr, $v:expr) => {{ let v = $v; self.life["book_state"][&key][$k] = json!(v); }} }
                bs!("tries", self.bsi(&key, "tries") + 1);
                let mut acts = vec![];
                if bookgen::close_enough(Some(got), p.a) {
                    let st = self.bsi(&key, "streak") + 1;
                    bs!("right", self.bsi(&key, "right") + 1);
                    bs!("streak", st);
                    bs!("best", self.bsi(&key, "best").max(st));
                    Self::set_text(ui, self.lab("fb"), "\u{6b63}\u{89e3}! Correct~ (Enter for the next one)");
                    Self::set_fg(ui, self.lab("fb"), &gold);
                    let t = if st > 1 { format!("Yatta! {st} in a row~") } else { "Yatta! Correct~".into() };
                    self.teach_say(ui, Some(&t), 2.5, None, true, now);
                } else {
                    bs!("streak", 0);
                    let a = crate::calc::fmt_num(&crate::calc::Num::Float(bookgen::py_round(p.a, 4)));
                    Self::set_text(ui, self.lab("fb"), &format!("Not quite: the answer is {a} {}. Working below. (Enter for a new one)",
                                                                p.unit.clone().unwrap_or_default()));
                    Self::set_fg(ui, self.lab("fb"), &grey);
                    Self::set_text(ui, self.lab("sol"), &p.sol);
                    acts.push(self.mistake(&p.q, &p.sol, &key));
                    self.teach_say(ui, Some("Let's look at the working together~"), 3.0, None, false, now);
                }
                acts.push(Act::Save);
                self.drill_score(ui);
                acts
            }
            "fr_new" => {
                self.fr_new(ui);
                vec![]
            }
            "fr_check" => {
                if self.prac.done {
                    self.fr_new(ui);
                    return vec![];
                }
                let Some((_, fr, en)) = self.prac.word.clone() else { return vec![] };
                let v = Self::entry_text(ui, self.prac.entry);
                if v.trim().is_empty() {
                    return vec![];
                }
                self.prac.done = true;
                let (ok, perfect) = if self.life["book_state"]["french"]["dir"] == "fr-en" {
                    let en2 = regex::Regex::new(r"\(.*?\)").unwrap().replace_all(&en, "").to_string();
                    let ok = en2.split([',', '/']).any(|a| bookgen::fr_norm(&v) == bookgen::fr_norm(a))
                        || bookgen::fr_norm(&v) == bookgen::fr_norm(&en);
                    (ok, ok)
                } else {
                    bookgen::fr_match(&v, &fr)
                };
                if !self.life["fr_vocab"].is_object() {
                    self.life["fr_vocab"] = json!({});
                }
                let mut acts = vec![];
                if ok {
                    self.life["fr_vocab"][&fr] = json!(self.fr_v(&fr) + 1);
                    self.life["book_state"]["french"]["right"] = json!(self.bsi("french", "right") + 1);
                    Self::set_text(ui, self.lab("fb"), &format!("{}{fr} = {en}", if perfect { "Tr\u{e8}s bien~ " } else { "Oui! Watch the accents: " }));
                    Self::set_fg(ui, self.lab("fb"), &gold);
                    self.teach_say(ui, Some("Tr\u{e8}s bien!"), 2.0, None, true, now);
                } else {
                    self.life["fr_vocab"][&fr] = json!(0);
                    Self::set_text(ui, self.lab("fb"), &format!("Presque~ {fr} = {en}"));
                    Self::set_fg(ui, self.lab("fb"), &grey);
                    acts.push(self.mistake(&format!("French: {en}"), &fr, "french"));
                    self.teach_say(ui, Some(&format!("{en}... say it with me: {fr}")), 3.0, None, false, now);
                }
                acts.push(Act::Save);
                self.fr_progress(ui);
                acts
            }
            "psy_new" => {
                self.psy_new(ui);
                vec![]
            }
            "psy_plan" => {
                let Some(p) = self.prac.psy.clone() else { return vec![] };
                self.book_ask(ui, &format!("Help me plan an answer (not write it for me): {}", p.q), Some("exam"))
            }
            "en_check" => {
                let t = ui.text_mut(self.prac.text).get();
                let res = bookgen::english_check(&t);
                Self::set_text(ui, self.lab("fb"), &res);
                let first = res.split('\n').next().unwrap_or("").to_string();
                self.teach_say(ui, Some(&first), 3.0, None, res.starts_with("Strong") || res.starts_with("Excellent"), now);
                vec![]
            }
            "en_detail" => {
                let t = ui.text_mut(self.prac.text).get().trim().to_string();
                if t.is_empty() {
                    return vec![];
                }
                self.book_ask(ui, &format!("Give me IB English Lang & Lit feedback on this analysis paragraph against the Paper 1 criteria (A \
                    understanding and interpretation, B analysis and evaluation, C focus and organisation, D language). Say what works, \
                    then 3 specific improvements, and rewrite ONE sentence as an example:\n\n{t}"), Some("exam"))
            }
            "sleep_guard" => {
                let on = ui.ws.iter().any(|w| matches!(&w.kind, Kind::Check { on: true, cmd, .. } if cmd == "bk:sleep_guard"));
                vec![Act::Cfg("sleep_guard".into(), json!(on))]
            }
            "gr_save" => self.grade_save(ui),
            "ask" => self.do_ask(ui),
            "ask_clear" => {
                ui.text_mut(self.ask).delete_all();
                vec![]
            }
            "distractions" => vec![Act::Run("edit_distractions".into())],
            _ => {
                if let Some(n) = c.strip_prefix("fr_theme:") {
                    self.life["book_state"]["french"]["theme"] = json!(n);
                    for (rb, k, v) in &self.prac.radios {
                        if let Kind::Check { on, .. } = &mut ui.ws[*rb].kind
                            && k == "theme" {
                            *on = v == n;
                        }
                    }
                    self.fr_new(ui);
                    return vec![Act::Save];
                }
                if let Some(n) = c.strip_prefix("fr_dir:") {
                    self.life["book_state"]["french"]["dir"] = json!(n);
                    for (rb, k, v) in &self.prac.radios {
                        if let Kind::Check { on, .. } = &mut ui.ws[*rb].kind
                            && k == "dir" {
                            *on = v == n;
                        }
                    }
                    self.fr_new(ui);
                    return vec![Act::Save];
                }
                if let Some(t) = c.strip_prefix("psy_pick:") {
                    if self.prac.next_at.is_some() {
                        return vec![];
                    }
                    let Some((term, dfn)) = self.prac.term.clone() else { return vec![] };
                    let tries = self.bsi("psych", "tries") + 1;
                    self.life["book_state"]["psych"]["tries"] = json!(tries);
                    let mut acts = vec![];
                    if t == term {
                        let r = self.bsi("psych", "right") + 1;
                        self.life["book_state"]["psych"]["right"] = json!(r);
                        Self::set_text(ui, self.lab("mfb"), &format!("\u{6b63}\u{89e3}! {term}. ({r} / {tries}) Next one coming~"));
                        Self::set_fg(ui, self.lab("mfb"), &gold);
                        self.teach_say(ui, Some("Sugoi~"), 1.5, None, true, now);
                    } else {
                        let r = self.bsi("psych", "right");
                        Self::set_text(ui, self.lab("mfb"), &format!("That's {t}; the answer was {term}. ({r} / {tries})"));
                        Self::set_fg(ui, self.lab("mfb"), &grey);
                        acts.push(self.mistake(&format!("Psych methods: {dfn}"), &term, "psych"));
                    }
                    self.prac.next_at = Some(now + 1.6);
                    acts.push(Act::Save);
                    return acts;
                }
                if let Some(d) = c.strip_prefix("en_tip:") {
                    self.en_show(ui, d.parse().unwrap_or(0));
                    return vec![Act::Save];
                }
                if let Some(m) = c.strip_prefix("mode:") {
                    self.set_mode(ui, m);
                    return vec![];
                }
                if let Some(tp) = c.strip_prefix("topic:") {
                    let q = format!("{}{tp}", if self.mode == "test" { "Test me on " } else { "Explain " });
                    return self.book_ask(ui, &q, None);
                }
                if let Some(k) = c.strip_prefix("track:") {
                    let on = ui.ws.iter().any(|w| matches!(&w.kind, Kind::Check { on: true, cmd, .. } if cmd == &format!("bk:track:{k}")));
                    return vec![Act::Cfg(k.into(), json!(on))];
                }
                if let Some(rest) = c.strip_prefix("sort:") {
                    let (k, sid) = rest.rsplit_once(':').unwrap_or((rest, ""));
                    return vec![Act::Run(format!("study_sort\t{k}\t{sid}")), Act::Render];
                }
                if let Some(t) = c.strip_prefix("tool:") {
                    return vec![Act::Run(format!("tool:{t}"))];
                }
                if let Some(w) = c.strip_prefix("pull:") {
                    return vec![Act::Run(format!("brain_pull:{w}"))];
                }
                if let Some(rest) = c.strip_prefix("gr_set:") {
                    let (which, v) = rest.split_once(':').unwrap_or((rest, ""));
                    let i = match which { "sid" => 0, "kind" => 1, _ => 2 };
                    let m = self.prac.menus[i];
                    Self::set_text(ui, m, v);
                    return vec![];
                }
                vec![Act::Run(format!("book:{c}"))]
            }
        }
    }

    /// the choices of an option menu in the grades book
    pub fn menu_items(&self, which: &str) -> Vec<String> {
        match which {
            "sid" => self.d.subjects.iter().map(|s| s.2.clone()).collect(),
            "kind" => bookgen::data()["GRADE_KINDS"].as_array().unwrap().iter().map(sv).collect(),
            _ => ["-", "7", "6", "5", "4", "3", "2", "1"].iter().map(|s| s.to_string()).collect(),
        }
    }

    /// practice_grades' save(): a result goes into the grades (grade_add), the page is made again after 0.9 s
    fn grade_save(&mut self, ui: &mut Ui) -> Vec<Act> {
        let fb = self.lab("fb");
        let (gold, grey) = (self.th.gold.clone(), self.th.grey.clone());
        let [m1, m2, m3] = [self.prac.menus[0], self.prac.menus[1], self.prac.menus[2]];
        let [e1, e2, e3, e4] = [self.prac.entries[0], self.prac.entries[1], self.prac.entries[2], self.prac.entries[3]];
        let sv_ = ui.text_of(m1);
        let sid = self.d.subjects.iter().find(|s| s.2 == sv_).map(|s| s.0.clone());
        let (st, ot) = (Self::entry_text(ui, e2), Self::entry_text(ui, e3));
        let mut sc = if st.trim().is_empty() { None } else { bookgen::num_answer(&st) };
        let mut of = if ot.trim().is_empty() { None } else { bookgen::num_answer(&ot) };
        let gv = ui.text_of(m3);
        let gr = gv.parse::<i64>().ok().filter(|_| gv.chars().all(|c| c.is_ascii_digit()));
        if let (Some(s), None) = (sc, of)
            && (0.0..=100.0).contains(&s) && st.contains('%') {
            of = Some(100.0);
        }
        if sc.is_none() && gr.is_none() {
            Self::set_text(ui, fb, "Add a score (like 34 out of 40) or an IB grade~");
            Self::set_fg(ui, fb, &grey);
            return vec![];
        }
        if let Some(s) = sc
            && (of.is_none_or(|o| o == 0.0) || s < 0.0 || s > of.unwrap()) {
            Self::set_text(ui, fb, "The score has to be between 0 and 'out of'~");
            Self::set_fg(ui, fb, &grey);
            return vec![];
        }
        let Some(day) = crate::plan::parse_iso(Self::entry_text(ui, e4).trim()) else {
            Self::set_text(ui, fb, "Date as YYYY-MM-DD please~");
            Self::set_fg(ui, fb, &grey);
            return vec![];
        };
        let kind = ui.text_of(m2);
        let sid = sid.unwrap_or_default();
        self.life["book_state"]["grades"]["sid"] = json!(sid);
        self.life["book_state"]["grades"]["kind"] = json!(kind);
        if sc.is_none() {
            of = None;
        }
        let now = crate::unix_now();
        let name: String = Self::entry_text(ui, e1).trim().chars().take(40).collect();
        let e = json!({"id": format!("{:x}", (now * 1000.0) as i64), "sid": sid, "kind": if kind.is_empty() { "Other".to_string() } else { kind.clone() },
                       "name": name, "score": sc, "out_of": of, "grade": gr, "date": crate::plan::iso(day)});
        if !self.life["grades"].is_array() {
            self.life["grades"] = json!([]);
        }
        self.life["grades"].as_array_mut().unwrap().push(e.clone());
        let pct = match (sc, of) {
            (Some(s), Some(o)) => format!(" ({}%)", (100.0 * s / o).round_ties_even() as i64),
            _ => String::new(),
        };
        let what = format!("{} {}", crate::study::sname(&sid), e["kind"].as_str().unwrap_or(""));
        let mut prev: Vec<Value> = self.life["grades"].as_array().unwrap().iter()
            .filter(|x| x["sid"] == sid.as_str() && x["id"] != e["id"] && x["score"].is_number()).cloned().collect();
        prev.sort_by(|a, b| (sv(&a["date"]), sv(&a["id"])).cmp(&(sv(&b["date"]), sv(&b["id"]))));
        let mut tail = String::new();
        if let (Some(last), Some(s), Some(o)) = (prev.last(), sc, of) {
            let d = 100.0 * s / o - 100.0 * last["score"].as_f64().unwrap_or(0.0) / last["out_of"].as_f64().unwrap_or(1.0);
            tail = format!(" That's {}{}% vs last time{}", if d >= 0.0 { "+" } else { "" }, d.round_ties_even() as i64,
                           if d > 0.0 { " \u{2191} sugoi!" } else { "." });
        }
        let mut parts = vec![];
        if let (Some(s), Some(o)) = (sc, of) {
            parts.push(format!("{}/{}{pct}", pg(s), pg(o)));
        }
        if let Some(g) = gr.filter(|&g| g != 0) {
            parts.push(format!("grade {g}"));
        }
        let msg = format!("Saved {what}: {}.{tail}", parts.join(", "));
        sc = None;
        let _ = sc;
        Self::set_text(ui, fb, &msg);
        Self::set_fg(ui, fb, &gold);
        let now2 = self.d.now;
        self.teach_say(ui, Some(&msg), 3.0, None, true, now2);
        vec![Act::Save, Act::Run("render_in:0.9".into())]
    }

    /// time passes: her next frame on the blackboard and the next quiz question; Some(true) when something changed
    pub fn tick(&mut self, ui: &mut Ui, now: f64, next_teach: &mut f64) -> bool {
        let mut changed = false;
        self.d.now = now;
        if now >= *next_teach {
            *next_teach = now + self.teach_step(ui, now);
            changed = true;
        }
        if self.prac.next_at.is_some_and(|t| now >= t) {
            self.psy_mnew(ui);
            changed = true;
        }
        changed
    }
}
