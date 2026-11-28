//! zenko.py's BibleWindow (聖書 · The Holy Bible): the night header with the book menu, chapter arrows and box,
//! translation menu and "Go to" box; a paned window with the warm paper page (the chapter, verse numbers, notes,
//! the picked and hovered verse) on the left and the study side on the right (the verse, its original words as
//! cards that flow left to right for Greek and right to left for Hebrew, a word's lexicon entry, the buttons).
//! The work zenko.py hands to app.bg comes back from here as jobs (chapter_job / orig_job / word_job) that the
//! window's host runs off the UI thread and answers with the *_done methods.

use crate::bible::{book, morph_text, n_books, step_lookup};
use crate::biblefs::{TRANS, bible_chapter, bible_original, notice};
use crate::mdtext::clip;
use crate::theme::Theme;
use crate::tktext::{Text, Wrap};
use crate::toolwin::{Tool, ToolFonts, entry_text, lab_w, set_entry, set_text, set_wrap};
use crate::ui::{Anchor, Fill, Font, Fonts, Grid, Kind, Pack, Place, Side, Ui};
use serde_json::{Map, Value};
use zenko_art::palette::mixhex;

pub const SIZE: (i32, i32) = (1280, 820);
pub const TITLE: &str = "Bible";
pub const PARCH: &str = "#efe3c8";
pub const PINK: &str = "#2b1d14";
pub const REF_PH: &str = "Go to\u{2026} John 3:16";

/// the fonts beyond the tool window's own: the Hebrew and Greek picks
#[derive(Clone, Debug)]
pub struct BibleFonts {
    pub heb: String,
    pub grk: String,
}

/// one chapter to fetch
#[derive(Clone, Debug, PartialEq)]
pub struct Want {
    pub bi: usize,
    pub ch: i64,
    pub trans: String,
}

pub struct BibleWin {
    pub t: Tool,
    pub bf: BibleFonts,
    pub bi: usize,
    pub ch: i64,
    pub vs: i64,
    pub trans: String,
    pub used: String,
    pub verses: Vec<(i64, String)>,
    pub words: Vec<Value>,
    /// (label, fraction) while a download runs
    pub prog: Option<(String, f64)>,
    pub book_btn: usize,
    pub ch_e: usize,
    pub tr_btn: usize,
    pub ref_e: usize,
    pub pw: usize,
    pub left: usize,
    pub right: usize,
    pub title_lbl: usize,
    pub sub_lbl: usize,
    pub read: usize,
    pub foot: usize,
    pub st_title: usize,
    pub st_sub: usize,
    pub outer: usize,
    pub st_cv: usize,
    pub sb: usize,
    pub inner: usize,
    pub status: usize,
    pub flow: Option<usize>,
    pub cards: Vec<usize>,
    pub lex_box: Option<usize>,
    /// the word card picked, and the lexicon body label waiting for its entry
    pub picked: Option<usize>,
    pub body: Option<usize>,
    /// flow_now is due (after a layout)
    pub flow_due: bool,
    /// the verse under the mouse (hov_v)
    pub hover: Option<i64>,
    /// the read text's lines that hold each verse: verse -> (first index, last index)
    pub ranges: Vec<(i64, (usize, usize), (usize, usize))>,
}

impl BibleWin {
    /// build(); `pos` is life bible_pos (or null), `cm10` the screen's pixels per 10 cm
    pub fn build(ui: &mut Ui, th: &Theme, f: &ToolFonts, bf: &BibleFonts, pos: &Value, cm10: i32) -> BibleWin {
        let mut t = Tool::new(th, f);
        let win = 0;
        ui.ws[win].bg = th.paper.clone();
        let def = serde_json::json!([18, 119, 1, "NKJV"]);
        let pos = if pos.as_array().is_some_and(|a| !a.is_empty()) { pos } else { &def };
        let n = |i: usize, d: i64| pos[i].as_i64().or_else(|| pos[i].as_f64().map(|v| v as i64)).unwrap_or(d);
        let (bi, ch) = (n(0, 18).clamp(0, n_books() as i64 - 1) as usize, n(1, 119));
        let vs = match n(2, 1) { 0 => 1, v => v };
        let trans = pos[3].as_str().filter(|s| TRANS.iter().any(|(k, _)| k == s)).unwrap_or("NKJV").to_string();
        let hdr = t.header(ui, win, "\u{8056}\u{66f8}", "The Holy Bible",
                           "Click any verse to see it in the original Hebrew, Aramaic or Greek, word by word, with notes for study.");
        let nav = ui.frame(hdr, &th.night);
        ui.pack(nav, Pack::default().side(Side::Right).padx(14, 14));
        let book_btn = ui.menubutton(nav, "", "bible_books", Font::new(&f.ui, 10, true), &th.field, &th.ink, "", 0, cm10);
        {
            let w = &mut ui.ws[book_btn];
            (w.bd, w.hl, w.padx, w.pady, w.hand) = (1, 0, 10, 3, true);
            if let Kind::Menu { ind, .. } = &mut w.kind {
                *ind = (0, 0);
            }
        }
        ui.pack(book_btn, Pack::default().side(Side::Left).padx(0, 6));
        let b = t.btn(ui, nav, "\u{25c0}", "bible_prev", false, 9);
        ui.pack(b, Pack::default().side(Side::Left));
        let ch_e = ui.entry(nav, 4, Font::new(&f.ui, 11, false), "", &th.field);
        if let Kind::Entry { e, .. } = &mut ui.ws[ch_e].kind {
            e.center = true;
        }
        ui.pack(ch_e, Pack::default().side(Side::Left).padx(4, 4).ipady(2));
        let b = t.btn(ui, nav, "\u{25b6}", "bible_next", false, 9);
        ui.pack(b, Pack::default().side(Side::Left));
        let tr_btn = ui.menubutton(nav, &trans, "bible_trans", Font::new(&f.ui, 9, true), &th.field, &th.ink, "", 6, cm10);
        ui.ws[tr_btn].hl = 0;
        ui.pack(tr_btn, Pack::default().side(Side::Left).padx(10, 6));
        let ref_e = ui.entry(nav, 16, Font::new(&f.ui, 10, false), REF_PH, &th.field);
        if let Kind::Entry { e, .. } = &mut ui.ws[ref_e].kind {
            e.fg = Some(th.grey.clone());
            e.pos = e.text.len();
        }
        ui.pack(ref_e, Pack::default().side(Side::Left).ipady(2));
        // the paned window: a frame (the sash colour) with the two panes placed side by side
        let pw = ui.frame(win, &mixhex(&th.paper, &th.gold, 0.25));
        ui.pack(pw, Pack::default().fill(Fill::Both).expand());
        let left = ui.frame(pw, PARCH);
        let right = ui.frame(pw, &th.card);
        let (parch, ink) = (PARCH, PINK);
        let title_lbl = lab_w(ui, left, "", parch, ink, Font::new(&f.serif, 20, false), Anchor::W, false, 0);
        ui.pack(title_lbl, Pack::default().fill(Fill::X).padx(30, 30).pady(16, 0));
        let sub_lbl = lab_w(ui, left, "", parch, &mixhex(ink, parch, 0.45), t.fi(&f.serif, 9, false, true), Anchor::W, false, 0);
        ui.pack(sub_lbl, Pack::default().fill(Fill::X).padx(30, 30));
        let mut tx = Text::new(Font::new(&f.serif, 13, false), ink, parch);
        (tx.wrap, tx.hl, tx.bd, tx.padx, tx.pady, tx.spacing1, tx.spacing3) = (Wrap::Word, 0, 1, 30, 12, 3, 7);
        (tx.width_ch, tx.height_ln) = (80, 24);
        {
            let c = tx.tag("vn");
            c.fg = Some(mixhex(&th.gold, ink, 0.25));
            c.font = Some(Font::new(&f.ui, 8, true));
            c.offset = Some(5);
        }
        tx.tag("sel_v").bg = Some(mixhex(parch, &th.gold, 0.28));
        tx.tag("hov_v").bg = Some(mixhex(parch, &th.gold, 0.12));
        {
            let c = tx.tag("note");
            c.fg = Some(th.gold.clone());
            c.font = Some(Font::new(&f.ui, 8, false));
        }
        {
            let c = tx.tag("msg");
            c.fg = Some(mixhex(ink, parch, 0.35));
            c.font = Some(t.fi(&f.serif, 12, false, true));
        }
        let read = ui.text(left, tx, "");
        ui.ws[read].hand = false;
        ui.pack(read, Pack::default().fill(Fill::Both).expand());
        t.scroll(ui, read);
        let foot = lab_w(ui, left, "", parch, &mixhex(ink, parch, 0.5), Font::new(&f.ui, 7, false), Anchor::W, true, 560);
        ui.pack(foot, Pack::default().fill(Fill::X).padx(30, 30).pady(0, 8));
        // the study side
        let bg = th.card.clone();
        let st_title = lab_w(ui, right, "", &bg, &th.ink, Font::new(&f.serif, 15, false), Anchor::W, false, 0);
        ui.pack(st_title, Pack::default().fill(Fill::X).padx(18, 18).pady(14, 0));
        let st_sub = lab_w(ui, right, "", &bg, &th.grey, Font::new(&f.ui, 9, false), Anchor::W, true, 420);
        ui.pack(st_sub, Pack::default().fill(Fill::X).padx(18, 18));
        // zk_scroll_frame(right, bg)
        let outer = ui.frame(right, &bg);
        ui.ws[outer].gw_rows = vec![1];
        ui.ws[outer].gw_cols = vec![1];
        let (cw, chh) = crate::pagekit::cm(cm10);
        let st_cv = ui.canvas(outer, &bg, cw, chh);
        let sb = ui.canvas(outer, &bg, 10, 40);
        ui.grid(st_cv, Grid { row: 0, col: 0, sticky: "nsew", ..Default::default() });
        ui.grid(sb, Grid { row: 0, col: 1, sticky: "ns", ..Default::default() });
        let inner = ui.frame(st_cv, &bg);
        ui.ws[inner].cwin = true;
        ui.pack(outer, Pack::default().fill(Fill::Both).expand().padx(12, 4).pady(6, 0));
        let status = lab_w(ui, right, "", &bg, &th.gold, Font::new(&f.ui, 8, true), Anchor::W, true, 420);
        ui.pack(status, Pack::default().fill(Fill::X).padx(18, 18).pady(2, 0));
        let br = ui.frame(right, &bg);
        ui.pack(br, Pack::default().fill(Fill::X).padx(16, 16).pady(6, 4));
        let b = t.btn(ui, br, "Explain this verse", "bible_explain", true, 9);
        ui.pack(b, Pack::default().side(Side::Left));
        let b = t.btn(ui, br, "Interlinear \u{2197}", "bible_web:interlinear", false, 9);
        ui.pack(b, Pack::default().side(Side::Left).padx(6, 6));
        let b = t.btn(ui, br, "Commentaries \u{2197}", "bible_web:commentaries", false, 9);
        ui.pack(b, Pack::default().side(Side::Left));
        let b = t.btn(ui, br, "My note", "bible_note", false, 9);
        ui.pack(b, Pack::default().side(Side::Left).padx(6, 6));
        let br2 = ui.frame(right, &bg);
        ui.pack(br2, Pack::default().fill(Fill::X).padx(16, 16).pady(0, 12));
        let b = t.btn(ui, br2, "Save all Hebrew + Greek for offline", "bible_get_all", false, 9);
        ui.pack(b, Pack::default().side(Side::Left));
        BibleWin { t, bf: bf.clone(), bi, ch, vs, trans: trans.clone(), used: trans, verses: vec![], words: vec![], prog: None,
                   book_btn, ch_e, tr_btn, ref_e, pw, left, right, title_lbl, sub_lbl, read, foot, st_title, st_sub, outer,
                   st_cv, sb, inner, status, flow: None, cards: vec![], lex_box: None, picked: None, body: None,
                   flow_due: false, hover: None, ranges: vec![] }
    }

    /// the book menu: [(testament, [(group, [book index])])]
    pub fn book_menu() -> Vec<(&'static str, Vec<(&'static str, Vec<usize>)>)> {
        let mut out: Vec<(&'static str, Vec<(&'static str, Vec<usize>)>)> = vec![];
        for i in 0..n_books() {
            let tm = if i < 39 { "Old Testament" } else { "New Testament" };
            let g = book(i).4;
            if out.last().is_none_or(|t| t.0 != tm) {
                out.push((tm, vec![]));
            }
            let groups = &mut out.last_mut().unwrap().1;
            if groups.last().is_none_or(|x| x.0 != g) {
                groups.push((g, vec![]));
            }
            groups.last_mut().unwrap().1.push(i);
        }
        out
    }

    pub fn pos_value(&self) -> Value {
        serde_json::json!([self.bi, self.ch, self.vs, self.trans])
    }

    pub fn ref_(&self, vs: Option<i64>) -> String {
        format!("{} {}:{}", if self.bi != 18 { book(self.bi).1 } else { "Psalm" }, self.ch, vs.unwrap_or(self.vs))
    }

    pub fn key(&self, vs: Option<i64>) -> String {
        format!("{}.{}.{}", book(self.bi).0, self.ch, vs.unwrap_or(self.vs))
    }

    pub fn go(&mut self, ui: &mut Ui, bi: usize, ch: i64, vs: i64) -> Want {
        self.bi = bi;
        self.ch = ch.min(book(bi).2).max(1);
        self.vs = if vs == 0 { 1 } else { vs };
        self.load_chapter(ui)
    }

    pub fn go_chapter(&mut self, ui: &mut Ui, d: i64) -> Option<Want> {
        let (mut bi, mut ch) = (self.bi, self.ch + d);
        if ch < 1 {
            if bi == 0 {
                return None;
            }
            bi -= 1;
            ch = book(bi).2;
        } else if ch > book(bi).2 {
            if bi == n_books() - 1 {
                return None;
            }
            (bi, ch) = (bi + 1, 1);
        }
        Some(self.go(ui, bi, ch, 1))
    }

    pub fn jump_chapter(&mut self, ui: &mut Ui) -> Option<Want> {
        let s = entry_text(ui, self.ch_e);
        let n: i64 = s.trim().parse().ok()?;
        Some(self.go(ui, self.bi, n, 1))
    }

    /// the "Go to" box took the focus: its hint goes
    pub fn ref_focus(&mut self, ui: &mut Ui) {
        if entry_text(ui, self.ref_e).starts_with("Go to") {
            set_entry(ui, self.ref_e, "");
            if let Kind::Entry { e, .. } = &mut ui.ws[self.ref_e].kind {
                e.fg = None;
            }
        }
    }

    pub fn goto_ref(&mut self, ui: &mut Ui) -> Option<Want> {
        let text = entry_text(ui, self.ref_e);
        match crate::routes17::bible_parse_ref(&text) {
            None => {
                set_text(ui, self.status, "I didn't understand that reference. Try 'Psalm 119:105' or 'Rom 8'.");
                None
            }
            Some((bi, ch, vs)) => Some(self.go(ui, bi, ch, vs.unwrap_or(1))),
        }
    }

    pub fn set_trans(&mut self, ui: &mut Ui, v: &str) -> Want {
        self.trans = v.to_string();
        crate::toolwin::set_btn_text(ui, self.tr_btn, v);
        if let Kind::Menu { text, .. } = &mut ui.ws[self.tr_btn].kind {
            *text = v.to_string();
        }
        self.load_chapter(ui)
    }

    fn set_read(&mut self, ui: &mut Ui, s: &str) {
        let t = ui.text_mut(self.read);
        t.delete_all();
        t.insert_end(s, &["msg"]);
        self.ranges.clear();
        self.hover = None;
    }

    /// load_chapter(): the page says it's opening; the chapter to fetch comes back
    pub fn load_chapter(&mut self, ui: &mut Ui) -> Want {
        let b = book(self.bi);
        if let Kind::Menu { text, .. } = &mut ui.ws[self.book_btn].kind {
            *text = format!("{}  \u{25be}", b.1);
        }
        set_entry(ui, self.ch_e, &self.ch.to_string());
        set_text(ui, self.title_lbl, &format!("{} {}", if self.bi == 18 { "Psalm" } else { b.1 }, self.ch));
        set_text(ui, self.sub_lbl, &format!("{}  \u{b7}  {}", if self.bi < 39 { "Old Testament" } else { "New Testament" }, b.4));
        self.set_read(ui, &format!("Opening {} {}...", b.1, self.ch));
        Want { bi: self.bi, ch: self.ch, trans: self.trans.clone() }
    }

    /// chapter_done(res, err) with life bible_notes; the original-language job for the verse picked comes back
    pub fn chapter_done(&mut self, ui: &mut Ui, fonts: &dyn Fonts, want: &Want, res: Result<(Vec<(i64, String)>, String), String>,
                        notes: &Map<String, Value>) -> Option<OrigWant> {
        if (self.bi, self.ch, &self.trans) != (want.bi, want.ch, &want.trans) {
            return None;
        }
        let (verses, used) = match res {
            Err(e) => {
                self.set_read(ui, &format!("{e}\n\nChapters you've opened before are saved and work offline. On a network that blocks \
                                            Bible sites, try at home once and this chapter will be here next time."));
                self.verses.clear();
                return Some(self.load_original(ui));
            }
            Ok(r) => r,
        };
        self.verses = verses.clone();
        self.used = used.clone();
        let mut ranges = vec![];
        {
            let t = ui.text_mut(self.read);
            t.delete_all();
            for (n, txt) in &verses {
                let tag = format!("v{n}");
                let start = t.end_index();
                t.insert_end(&format!("{n} "), &["vn", &tag]);
                t.insert_end(txt, &[&tag]);
                if notes.get(&self.key(Some(*n))).is_some_and(truthy) {
                    t.insert_end("  \u{270e}", &["note", &tag]);
                }
                t.insert_end("\n", &[&tag]);
                ranges.push((*n, start, t.end_index()));
            }
        }
        self.ranges = ranges;
        let mut note = notice(&used).to_string();
        if used != self.trans {
            note = format!("{} couldn't be fetched here, so this is the {}. ", self.trans, used) + &note;
        }
        set_text(ui, self.foot, &note);
        let vs = if verses.iter().any(|(n, _)| *n == self.vs) { self.vs } else { verses.first().map(|v| v.0).unwrap_or(1) };
        Some(self.pick(ui, fonts, vs, true))
    }

    /// pick(n, scroll): the verse marked; its original words to look up come back
    pub fn pick(&mut self, ui: &mut Ui, fonts: &dyn Fonts, n: i64, scroll: bool) -> OrigWant {
        self.vs = n;
        let tag = format!("v{n}");
        {
            let t = ui.text_mut(self.read);
            t.tag_remove_all("sel_v");
            t.tag_add_over("sel_v", &tag);
            if scroll && let Some(first) = t.tag_first(&tag) {
                t.see(fonts, first);
            }
        }
        self.load_original(ui)
    }

    /// the verse a click or hover at this index of the read text is in
    pub fn verse_at(&self, ui: &Ui, idx: (usize, usize)) -> Option<i64> {
        let Kind::Text { t, .. } = &ui.ws[self.read].kind else { return None };
        t.tags_at(idx).iter().rev().find_map(|g| g.strip_prefix('v').and_then(|n| n.parse().ok()))
    }

    /// <Enter> / <Leave> on a verse: hov_v follows the mouse. True when it changed.
    pub fn set_hover(&mut self, ui: &mut Ui, v: Option<i64>) -> bool {
        if v == self.hover {
            return false;
        }
        self.hover = v;
        let t = ui.text_mut(self.read);
        t.tag_remove_all("hov_v");
        if let Some(n) = v {
            t.tag_add_over("hov_v", &format!("v{n}"));
        }
        true
    }

    pub fn status_prog(&self, ui: &mut Ui) {
        if let Some((l, f)) = &self.prog {
            set_text(ui, self.status, &format!("Downloading {l}... {}%  (one time, then it works offline)", (f * 100.0) as i64));
        }
    }

    /// load_original(): the study side shows the verse and waits for its words
    pub fn load_original(&mut self, ui: &mut Ui) -> OrigWant {
        let lang = if self.bi >= 39 { "Greek" } else { "Hebrew" };
        let eng = self.verses.iter().find(|v| v.0 == self.vs).map(|v| v.1.clone()).unwrap_or_default();
        set_text(ui, self.st_title, &self.ref_(None));
        set_text(ui, self.st_sub, &eng);
        self.clear_cards(ui);
        set_text(ui, self.status, &format!("Looking up the {lang}..."));
        OrigWant { bi: self.bi, ch: self.ch, vs: self.vs }
    }

    fn clear_cards(&mut self, ui: &mut Ui) {
        for k in ui.ws[self.inner].kids.clone() {
            ui.destroy(k);
        }
        self.cards.clear();
        self.flow = None;
        self.lex_box = None;
        self.picked = None;
        self.body = None;
    }

    pub fn orig_done(&mut self, ui: &mut Ui, want: &OrigWant, res: Result<Vec<Value>, String>, notes: &Map<String, Value>) {
        self.prog = None;
        if (self.bi, self.ch, self.vs) != (want.bi, want.ch, want.vs) {
            return;
        }
        let lang = if self.bi >= 39 { "Greek" } else { "Hebrew" };
        match res {
            Err(e) => {
                set_text(ui, self.status, &format!("{e}  The 'Interlinear \u{2197}' button opens this verse on Bible Hub instead."));
                self.words.clear();
                self.show_empty(ui);
            }
            Ok(w) => {
                self.words = w;
                let mut langs: Vec<String> = self.words.iter().map(|w| w["lang"].as_str().unwrap_or(lang).to_string()).collect();
                langs.sort();
                langs.dedup();
                let kind = if langs.is_empty() { lang.to_string() } else { langs.join(" and ") };
                let msg = if self.words.is_empty() {
                    "No original words found for this verse (the numbering may differ).".to_string()
                } else {
                    format!("{} words in {kind}. Click a word for its full dictionary entry.", self.words.len())
                };
                set_text(ui, self.status, &msg);
                self.show_cards(ui, notes);
            }
        }
    }

    fn show_empty(&mut self, ui: &mut Ui) {
        self.clear_cards(ui);
        let th = &self.t.th;
        let l = ui.label(self.inner, "The original text will appear here.", &th.card, &th.grey, self.t.fi(&self.t.f.serif, 11, false, true));
        ui.pack(l, Pack::default().anchor(Anchor::W).padx(8, 8).pady(8, 8));
    }

    fn word_font(&self) -> &str {
        if self.bi < 39 { &self.bf.heb } else { &self.bf.grk }
    }

    fn show_cards(&mut self, ui: &mut Ui, notes: &Map<String, Value>) {
        self.clear_cards(ui);
        let th = self.t.th.clone();
        let bg = th.card.clone();
        let heb = self.bi < 39;
        let fam = self.word_font().to_string();
        let line: Vec<&str> = self.words.iter().filter(|w| !truthy(&w["var"])).map(|w| w["w"].as_str().unwrap_or("")).collect();
        let l = lab_w(ui, self.inner, &line.join(" "), &bg, &th.ink, Font::new(&fam, 17, false),
                      if heb { Anchor::E } else { Anchor::W }, !heb, 460);
        ui.pack(l, Pack::default().fill(Fill::X).padx(8, 8).pady(6, 10));
        let flow = ui.frame(self.inner, &bg);
        ui.pack(flow, Pack::default().fill(Fill::X).padx(4, 4));
        self.flow = Some(flow);
        let cbg = mixhex(&bg, &th.field, 0.55);
        for w in self.words.clone() {
            let c = ui.frame(flow, &cbg);
            let cw = &mut ui.ws[c];
            (cw.padx, cw.pady, cw.hand, cw.hl, cw.hlc) = (8, 5, true, 1, mixhex(&bg, &th.gold, 0.25));
            let col = if w["lang"] == "Aramaic" { "#9a6a2a".to_string() } else { th.ink.clone() };
            let s = |k: &str| w[k].as_str().unwrap_or("").to_string();
            let sm = strip_chars(&format!("{} \u{b7} {}", s("s"), s("m")), &[' ', '\u{b7}']);
            let labs = [
                ui.label(c, &s("w"), &cbg, &col, Font::new(&fam, 16, false)),
                ui.label(c, &s("t"), &cbg, &th.grey, self.t.fi(&self.t.f.ui, 8, false, true)),
                ui.label(c, &clip(&s("g"), 26), &cbg, &th.ink, Font::new(&self.t.f.ui, 9, true)),
                ui.label(c, &(sm + if truthy(&w["var"]) { "  variant" } else { "" }), &cbg, &mixhex(&th.grey, &bg, 0.2),
                         Font::new(&self.t.f.ui, 7, false)),
            ];
            for lb in labs {
                ui.ws[lb].hand = true;
                ui.pack(lb, Pack::default());
            }
            self.cards.push(c);
        }
        let lb = ui.frame(self.inner, &bg);
        ui.pack(lb, Pack::default().fill(Fill::X).padx(8, 8).pady(12, 8));
        self.lex_box = Some(lb);
        if let Some(n) = notes.get(&self.key(None)).and_then(|v| v.as_str()).filter(|s| !s.is_empty()) {
            let a = lab_w(ui, lb, "\u{270e} My note", &bg, &th.gold, Font::new(&self.t.f.ui, 8, true), Anchor::W, false, 0);
            ui.pack(a, Pack::default().fill(Fill::X));
            let b = lab_w(ui, lb, n, &bg, &th.ink, self.t.fi(&self.t.f.serif, 10, false, true), Anchor::W, true, 440);
            ui.pack(b, Pack::default().fill(Fill::X));
        }
        self.flow_due = true;
    }

    /// flow_now(): the word cards laid out in lines (needs a layout first, for their sizes)
    pub fn flow_now(&mut self, ui: &mut Ui) {
        self.flow_due = false;
        let Some(flow) = self.flow else { return };
        if self.cards.is_empty() {
            return;
        }
        let w_all = 200.max(ui.ws[self.st_cv].w - 24);
        let rtl = self.bi < 39;
        let (mut row, mut col, mut used) = (0usize, 0usize, 0);
        let mut rc = vec![];
        for &c in &self.cards {
            let w = ui.ws[c].rw + 6;
            if col > 0 && used + w > w_all {
                (row, col, used) = (row + 1, 0, 0);
            }
            rc.push((row, col));
            used += w;
            col += 1;
        }
        let rows = rc.last().map(|r| r.0 + 1).unwrap_or(0);
        let per_row: Vec<usize> = (0..rows).map(|r| rc.iter().filter(|x| x.0 == r).count()).collect();
        let maxc = per_row.iter().copied().max().unwrap_or(0);
        for (k, &c) in self.cards.iter().enumerate() {
            let (r, cc) = rc[k];
            let gc = if rtl { maxc - cc } else { cc };
            ui.grid(c, Grid { row: r, col: gc, sticky: if rtl { "ne" } else { "nw" }, padx: (3, 3), pady: (3, 3), ..Default::default() });
        }
        ui.ws[flow].gw_cols = (0..=maxc).map(|k| if rtl && k == 0 { 1 } else { 0 }).collect();
        if let Some(lb) = self.lex_box {
            for c in ui.ws[lb].kids.clone() {
                if matches!(ui.ws[c].kind, Kind::Label { .. }) {
                    set_wrap(ui, c, 200.max(w_all - 20));
                }
            }
        }
    }

    /// word(i): the card marked and its entry started; the lexicon lookup (lang, strong) comes back
    pub fn word(&mut self, ui: &mut Ui, i: usize) -> Option<(String, String)> {
        let w = self.words.get(i)?.clone();
        let th = self.t.th.clone();
        for (k, &c) in self.cards.iter().enumerate() {
            let cw = &mut ui.ws[c];
            (cw.hlc, cw.hl) = if k == i { (th.gold.clone(), 2) } else { (mixhex(&th.card, &th.gold, 0.25), 1) };
        }
        self.picked = Some(i);
        let lb = self.lex_box?;
        for c in ui.ws[lb].kids.clone() {
            ui.destroy(c);
        }
        let bg = th.card.clone();
        let s = |k: &str| w[k].as_str().unwrap_or("").to_string();
        let lemma = if s("l").is_empty() { s("w") } else { s("l") };
        let a = lab_w(ui, lb, &format!("{lemma}   {}", s("t")), &bg, &th.ink, Font::new(self.word_font(), 15, false), Anchor::W, false, 0);
        ui.pack(a, Pack::default().fill(Fill::X));
        let meta = format!("{}  \u{b7}  {}  \u{b7}  {}", s("s"), s("lang"), morph_text(&s("m"), &s("lang")));
        let b = lab_w(ui, lb, &meta, &bg, &th.grey, Font::new(&self.t.f.ui, 8, false), Anchor::W, true, 440);
        ui.pack(b, Pack::default().fill(Fill::X));
        let body = lab_w(ui, lb, "Opening the lexicon...", &bg, &th.ink, Font::new(&self.t.f.ui, 9, false), Anchor::W, true, 440);
        ui.pack(body, Pack::default().fill(Fill::X).pady(4, 0));
        self.body = Some(body);
        let lang = if w["lang"].is_null() { "Hebrew".to_string() } else { s("lang") };
        Some((if lang == "Greek" { "Greek" } else { "Hebrew" }.to_string(), s("s")))
    }

    /// word's done(): the lexicon entry (lemma, translit, gloss, meaning) or nothing
    pub fn word_done(&mut self, ui: &mut Ui, i: usize, res: Option<Value>) {
        let (Some(body), Some(w)) = (self.body, self.words.get(i)) else { return };
        if self.picked != Some(i) {
            return;
        }
        let g = w["g"].as_str().filter(|s| !s.is_empty()).or(w["lg"].as_str()).unwrap_or("").to_string();
        match res {
            None => set_text(ui, body, &format!("Gloss: {g}")),
            Some(e) => {
                let part = |k: usize| e[k].as_str().unwrap_or("").to_string();
                set_text(ui, body, &format!("{}\n\n{}", part(2), part(3).trim()));
                self.flow_due = true;
            }
        }
    }

    /// biblehub.com for this verse (open_web)
    pub fn web_url(&self, kind: &str) -> String {
        format!("https://biblehub.com/{kind}/{}/{}-{}.htm", book(self.bi).3, self.ch, self.vs)
    }

    /// what explain() asks the brain: (title ref, the user message, the cache key)
    pub fn explain_ask(&self) -> (String, String, String) {
        let r = self.ref_(None);
        let eng = self.verses.iter().find(|v| v.0 == self.vs).map(|v| v.1.clone()).unwrap_or_default();
        let words: Vec<String> = self.words.iter().take(60).filter(|w| !truthy(&w["var"])).map(|w| {
            let s = |k: &str| w[k].as_str().unwrap_or("").to_string();
            format!("- {} ({}) {} \u{b7} {} \u{b7} {}", s("w"), s("t"), s("s"), s("m"), s("g"))
        }).collect();
        let ctx: Vec<String> = self.verses.iter().filter(|(n, _)| (n - self.vs).abs() <= 3 && *n != self.vs)
            .map(|(n, t)| format!("{n} {t}")).collect();
        let words = if words.is_empty() { "(not downloaded)".to_string() } else { words.join("\n") };
        let user = format!("Verse: {r} ({})\n\"{eng}\"\n\nOriginal words ({}):\n{words}\n\nNearby verses for context: {}", self.used,
                           if self.bi >= 39 { "Greek" } else { "Hebrew/Aramaic" }, clip(&ctx.join(" "), 900));
        (r, user, format!("{}|{}", self.key(None), self.used))
    }

    /// lay the window out at w x h: the sash at half the width, the panes' wraplengths from their sizes, the cards
    /// flowed once their sizes are known, then the scrollbars
    pub fn fit(&mut self, ui: &mut Ui, fonts: &dyn Fonts, w: i32, h: i32) {
        let sash = (w as f64 * 0.5) as i32;
        ui.ws[self.left].place = Some(Place { x: 0, w: Some(sash), relh: Some(1.0), ..Default::default() });
        ui.ws[self.right].place = Some(Place { x: sash + 5, w: Some(w - sash - 5), relh: Some(1.0), ..Default::default() });
        set_wrap(ui, self.foot, 200.max(sash - 60));
        let rw = w - sash - 5;
        set_wrap(ui, self.st_sub, 200.max(rw - 40));
        set_wrap(ui, self.status, 200.max(rw - 40));
        ui.layout_at(fonts, w, h);
        // (the inside's <Configure> flows the cards again whenever its size changes)
        if self.flow_due || !self.cards.is_empty() {
            self.flow_now(ui);
            ui.layout_at(fonts, w, h);
        }
        self.t.paint_bars(ui, fonts);
        let (lo, hi) = self.yview(ui);
        crate::chatpage::scrollbar_draw(ui, self.sb, lo, hi, false, &self.t.th);
    }

    /// the study side's canvas view over its inside
    pub fn yview(&self, ui: &Ui) -> (f64, f64) {
        let (rh, h) = (ui.ws[self.inner].rh.max(1) as f64, ui.ws[self.st_cv].h as f64);
        let y = ui.ws[self.st_cv].yoff as f64;
        ((y / rh).max(0.0), ((y + h) / rh).min(1.0))
    }

    /// the mouse wheel over the study side
    pub fn wheel(&self, ui: &mut Ui, delta: i32) -> bool {
        let top = (ui.ws[self.inner].rh - ui.ws[self.st_cv].h).max(0);
        if top == 0 {
            return false;
        }
        let y = (ui.ws[self.st_cv].yoff - delta / 120 * (ui.ws[self.st_cv].h / 10).max(1)).clamp(0, top);
        let ch = y != ui.ws[self.st_cv].yoff;
        ui.ws[self.st_cv].yoff = y;
        ch
    }

    /// the card (or a label in it) at this widget
    pub fn card_of(&self, ui: &Ui, mut id: usize) -> Option<usize> {
        loop {
            if let Some(i) = self.cards.iter().position(|&c| c == id) {
                return Some(i);
            }
            id = ui.ws[id].parent?;
        }
    }
}

/// the original words to look up
#[derive(Clone, Debug, PartialEq)]
pub struct OrigWant {
    pub bi: usize,
    pub ch: i64,
    pub vs: i64,
}

fn truthy(v: &Value) -> bool {
    match v {
        Value::Bool(b) => *b,
        Value::Null => false,
        Value::String(s) => !s.is_empty(),
        Value::Number(n) => n.as_f64() != Some(0.0),
        _ => true,
    }
}

/// str.strip(chars)
fn strip_chars(s: &str, cs: &[char]) -> String {
    s.trim_matches(|c| cs.contains(&c)).to_string()
}

/// load_chapter's job: the chapter, or anything saved on this PC in another translation
pub fn chapter_job(w: &Want) -> Result<(Vec<(i64, String)>, String), String> {
    match bible_chapter(w.bi, w.ch, &w.trans, true) {
        Ok(r) => Ok(r),
        Err(e) => {
            for (alt, _) in TRANS.iter().filter(|(k, _)| *k != w.trans) {
                if let Ok(r) = bible_chapter(w.bi, w.ch, alt, false) {
                    return Ok(r);
                }
            }
            Err(e)
        }
    }
}

/// load_original's job
pub fn orig_job(w: &OrigWant, progress: Option<&mut dyn FnMut(&str, f64)>) -> Result<Vec<Value>, String> {
    bible_original(w.bi, w.ch, w.vs, true, progress)
}

/// word's job: the lexicon entry
pub fn word_job(lang: &str, strong: &str) -> Option<Value> {
    let lex = crate::biblefs::step_lexicon(lang);
    step_lookup(&lex, strong).cloned()
}
