//! zenko20's Ctrl+K quick search (zenko.py PAGE_WORDS, zk_match, ZkPalette, Zenko20.z20_items): a floating box over
//! the main window that finds pages, the cards on them, the study books, things to do and look choices. This is
//! the platform independent part: the scoring, the list of items, and the box as a `Ui` tree (paletwin.rs hosts it).

use crate::pagekit::walk;
use crate::theme::Theme;
use crate::ui::{Anchor, Fill, Font, Fonts, Kind, Pack, Side, Ui};
use serde_json::Value;
use zenko_art::palette::mixhex;

/// one thing the box can offer; `run` says what picking it does (see paletwin.rs `run`)
#[derive(Clone, Debug, PartialEq)]
pub struct PItem {
    pub title: String,
    pub kind: String,
    pub sub: String,
    pub words: String,
    pub boost: f64,
    pub run: String,
}

fn data() -> &'static Value {
    static D: std::sync::OnceLock<Value> = std::sync::OnceLock::new();
    D.get_or_init(|| serde_json::from_str(include_str!("../data/palette.json")).unwrap())
}

fn cust() -> &'static Value {
    static D: std::sync::OnceLock<Value> = std::sync::OnceLock::new();
    D.get_or_init(|| serde_json::from_str(include_str!("../data/customize.json")).unwrap())
}

/// PAGE_WORDS (with Zenko22's town square)
pub fn page_words(k: &str) -> String {
    if k == "square" {
        return "town square global chat online friends message everyone community talk".into();
    }
    data()["PAGE_WORDS"][k].as_str().unwrap_or("").to_string()
}

fn is_word(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// re.search(r"\b" + re.escape(part), s)
fn word_start(s: &[char], part: &[char]) -> bool {
    if part.is_empty() || part.len() > s.len() {
        return false;
    }
    (0..=s.len() - part.len()).any(|i| {
        s[i..i + part.len()] == *part && {
            let prev = if i == 0 { false } else { is_word(s[i - 1]) };
            prev != is_word(part[0])
        }
    })
}

fn contains(s: &[char], part: &[char]) -> bool {
    part.is_empty() || (part.len() <= s.len() && (0..=s.len() - part.len()).any(|i| s[i..i + part.len()] == *part))
}

/// re.findall(r"\w+", s)
fn words_of(s: &[char]) -> Vec<String> {
    let mut out = vec![];
    let mut cur = String::new();
    for &c in s {
        if is_word(c) {
            cur.push(c);
        } else if !cur.is_empty() {
            out.push(std::mem::take(&mut cur));
        }
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out
}

/// zk_match(q, text, words): how well a search fits a title (0 = not at all). Word starts beat middles beat fuzzy.
pub fn zk_match(q: &str, text: &str, words: &str) -> f64 {
    let q = q.to_lowercase();
    let q = q.trim();
    if q.is_empty() {
        return 1.0;
    }
    let t: Vec<char> = text.to_lowercase().chars().collect();
    let hay: Vec<char> = format!("{} {}", text.to_lowercase(), words.to_lowercase()).chars().collect();
    let mut score = 0.0;
    for part in q.split_whitespace() {
        let p: Vec<char> = part.chars().collect();
        if t.starts_with(&p) {
            score += 6.0;
        } else if word_start(&t, &p) {
            score += 4.5;
        } else if contains(&t, &p) {
            score += 3.0;
        } else if word_start(&hay, &p) {
            score += 2.0;
        } else if contains(&hay, &p) {
            score += 1.2;
        } else {
            if p.len() < 4 {
                return 0.0;
            }
            let best = words_of(&hay).iter().map(|w| crate::pydifflib::ratio(part, w)).fold(0.0, f64::max);
            if best < 0.78 {
                return 0.0;
            }
            score += best;
        }
    }
    score
}

/// ZkPalette.results: the best eight, then "Ask <name>" when something is typed
pub fn results(q: &str, items: &[PItem], fox: &str) -> Vec<PItem> {
    let mut out: Vec<(f64, usize, &PItem)> = vec![];
    for it in items {
        let s = zk_match(q, &it.title, &format!("{} {}", it.words, it.sub));
        if s > 0.0 {
            out.push((-(s + it.boost), out.len(), it));
        }
    }
    out.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap().then(a.1.cmp(&b.1)));
    let mut res: Vec<PItem> = out.iter().take(8).map(|t| t.2.clone()).collect();
    let qs = q.trim();
    if !qs.is_empty() {
        res.push(PItem { title: format!("Ask {fox}: \u{201c}{qs}\u{201d}"), kind: "Chat".into(),
                         sub: "Sends it to the Chat page".into(), words: String::new(), boost: 0.0,
                         run: format!("ask:{qs}") });
    }
    res
}

/// what z20_items needs from the app
pub struct ItemsIn<'a> {
    /// z20_nav_all(): every page key and label
    pub nav_all: &'a [(String, String)],
    /// the sidebar's pages, in order
    pub visible: &'a [String],
    /// _z20_cards: (page, card title) in the order they were made
    pub cards: &'a [(String, String)],
    pub session: bool,
    pub fox: &'a str,
}

/// the card title cleaned the way z20_items cleans it: no leading "<emoji> · ", no trailing "(...)"
pub fn clean_title(title: &str) -> String {
    static RE: std::sync::OnceLock<(regex::Regex, regex::Regex)> = std::sync::OnceLock::new();
    let (lead, tail) = RE.get_or_init(|| (regex::Regex::new(r"^[^\x00-\x7f]+\s*\u{b7}\s*").unwrap(),
                                          regex::Regex::new(r"\s*\(.*?\)\s*$").unwrap()));
    let c = lead.replacen(title, 1, "").to_string();
    let c = tail.replacen(&c, 1, "").to_string();
    if c.is_empty() { title.to_string() } else { c }
}

/// Zenko20.z20_items
pub fn items(inp: &ItemsIn) -> Vec<PItem> {
    let it = |title: String, kind: &str, sub: String, words: String, boost: f64, run: String| PItem {
        title, kind: kind.into(), sub, words, boost, run,
    };
    let mut items = vec![];
    for (k, lab) in inp.nav_all {
        let sub = match inp.visible.iter().position(|v| v == k) {
            Some(i) if i < 9 => format!("Ctrl+{}", i + 1),
            Some(_) => String::new(),
            None => "hidden from sidebar".into(),
        };
        items.push(it(lab.clone(), "Page", sub, page_words(k), 1.5, format!("page:{k}")));
    }
    let mut seen: Vec<(String, String)> = vec![];
    for (page, title) in inp.cards {
        let clean = clean_title(title);
        let key = (page.clone(), clean.clone());
        if seen.contains(&key) || title.contains("%s") {
            continue;
        }
        seen.push(key);
        let label = inp.nav_all.iter().find(|(k, _)| k == page).map(|x| x.1.clone()).unwrap_or_else(|| page.clone());
        let kind = if page == "settings" || page == "customize" { "Setting" } else { "Section" };
        items.push(it(clean, kind, label, title.clone(), 0.0, format!("reveal:{page}\t{title}")));
    }
    let mut a = vec![];
    for b in data()["BOOKS"].as_array().unwrap() {
        let s = |i: usize| b[i].as_str().unwrap().to_string();
        a.push(it(format!("{} book", s(1)), "Book", "Study books".into(), s(2), 0.8, format!("book:{}", s(0))));
    }
    let mut act = |title: &str, run: &str, words: &str, sub: &str| {
        a.push(it(title.into(), "Do", sub.into(), words.into(), 0.5, run.into()));
    };
    if inp.session {
        act("Pause or resume the session", "do:pause", "focus timer stop pause", "");
        act("Stop the session", "do:stop", "focus timer end", "");
    } else {
        act("Start a focus session", "do:start", "focus timer pomodoro study start begin", "");
    }
    act("Lock in (no distractions)", "do:lockin", "lockin lock-in focus strict", "");
    act("Take a break", "do:break", "rest pause", "");
    act("Box breathing", "do:breath_box", "breathe calm anxious relax", "");
    act("4-7-8 breathing before sleep", "do:breath_478", "breathe sleep", "");
    act(&format!("Talk out loud with {}", inp.fox), "do:talk", "voice speak microphone talk", "");
    for (title, run, w) in [("Periodic table", "do:ptable", "chemistry elements"),
                            ("French dictionary", "do:french", "french words translate"),
                            ("Calculator (fx-CG50 style)", "do:calc", "math maths calculator casio"),
                            ("Bible", "do:bible", "scripture verse")] {
        act(title, run, w, "Tool window");
    }
    act("Export timetable as PDF", "do:export_pdf", "remarkable print", "");
    act("Check for updates", "reveal:settings\tUpdates and install", "update version new", "");
    for (val, lab) in crate::setpage::FUR_NAMES {
        a.push(it(format!("Theme: {lab}"), "Look", String::new(), format!("colour color theme fox coat {val}"), 0.0,
                  format!("theme:{val}")));
    }
    for ac in cust()["ACCENTS"].as_array().unwrap() {
        a.push(it(format!("Accent: {}", ac[1].as_str().unwrap()), "Look", String::new(),
                  "accent colour color highlight buttons".into(), 0.0, format!("accent:{}", ac[0].as_str().unwrap())));
    }
    for m in cust()["MOTION_LEVELS"].as_array().unwrap() {
        a.push(it(format!("Motion: {}", m[1].as_str().unwrap()), "Look", String::new(),
                  "motion speed animation battery performance fast".into(), 0.0,
                  format!("motion:{}", m[0].as_str().unwrap())));
    }
    a.push(it("Arrange the sidebar".into(), "Look", String::new(), "sidebar order hide pages menu navigation".into(), 0.0,
              "reveal:customize\tSidebar pages".into()));
    items.extend(a);
    items
}

/// the hub_card titles of a built page, in the order they were made (the gold diamond row's label)
pub fn card_titles(ui: &Ui, pg: usize) -> Vec<String> {
    let mut out = vec![];
    let mut ids = walk(ui, pg);
    ids.sort();                                          // (ids are handed out as widgets are made)
    for id in ids {
        let k = &ui.ws[id].kids;
        if k.len() >= 2 && matches!(ui.ws[id].kind, Kind::Frame)
            && matches!(&ui.ws[k[0]].kind, Kind::Label { text, .. } if text == "\u{25c6}")
            && let Kind::Label { text, .. } = &ui.ws[k[1]].kind {
            out.push(text.clone());
        }
    }
    out
}

/// the box: ZkPalette.__init__ / refresh / paint
pub struct Palette {
    pub ui: Ui,
    pub entry: usize,
    pub list: usize,
    pub rows: Vec<usize>,
    pub shown: Vec<PItem>,
    pub sel: usize,
    pub w: i32,
    pub items: Vec<PItem>,
    pub fox: String,
    pub q: String,
    fu: String,
    th: Theme,
    zk_f: f64,
}

impl Palette {
    /// the box for a main window `hub_w` wide; fonts as zk_on_map gives them (zk_size of the hub's zk_f)
    pub fn new(th: &Theme, font_ui: &str, hub_w: i32, zk_f: f64, items: Vec<PItem>, fox: &str, fonts: &dyn Fonts)
               -> Palette {
        let w = 640.min(420.max(hub_w - 160));
        let card = th.card.clone();
        let edge = mixhex(&card, &th.gold, 0.45);
        let mut ui = Ui::new(&edge);
        let fnt = |n: i32, b: bool| Font::new(font_ui, n, b);
        let body = ui.frame(0, &card);
        ui.pack(body, Pack::default().fill(Fill::Both).expand().padx(1, 1).pady(1, 1));
        let top = ui.frame(body, &card);
        ui.pack(top, Pack::default().fill(Fill::X).padx(14, 14).pady(12, 6));
        let s = ui.label(top, "\u{2315}", &card, &th.gold, fnt(15, false));
        ui.pack(s, Pack::default().side(Side::Left).padx(2, 8));
        let entry = ui.entry(top, 20, fnt(14, false), "", &card);
        {
            let e = &mut ui.ws[entry];
            (e.bd, e.hl) = (0, 0);
            if let Kind::Entry { e, .. } = &mut e.kind {
                e.flat = Some((th.ink.clone(), th.gold.clone()));
            }
        }
        ui.pack(entry, Pack::default().side(Side::Left).fill(Fill::X).expand().ipady(4));
        let esc = ui.label(top, "esc", &mixhex(&card, &th.ink, 0.08), &th.grey, fnt(8, false));
        ui.ws[esc].padx = 6;
        ui.pack(esc, Pack::default().side(Side::Right));
        let sep = ui.frame(body, &th.line);
        ui.ws[sep].opt_h = 1;
        ui.pack(sep, Pack::default().fill(Fill::X));
        let list = ui.frame(body, &card);
        ui.pack(list, Pack::default().fill(Fill::Both).expand().pady(4, 4));
        let sp = ui.frame(body, &card);
        (ui.ws[sp].opt_w, ui.ws[sp].opt_h) = (w - 2, 1);
        ui.pack(sp, Pack::default());
        let foot = ui.label(body, "\u{2191}\u{2193} to move  \u{b7}  Enter to open  \u{b7}  Ctrl+1\u{2026}9 jump to sidebar pages",
                            &mixhex(&card, &th.night, 0.35), &th.grey, fnt(8, false));
        ui.set_anchor(foot, Anchor::W);
        (ui.ws[foot].padx, ui.ws[foot].pady) = (14, 5);
        ui.pack(foot, Pack::default().fill(Fill::X).side(Side::Bottom));
        ui.focus = Some(entry);
        ui.active = true;
        let mut p = Palette { ui, entry, list, rows: vec![], shown: vec![], sel: 0, w, items, fox: fox.into(),
                              q: String::new(), fu: font_ui.into(), th: th.clone(), zk_f };
        p.refresh(fonts);
        p
    }

    /// what the search box holds now (refresh when it changed)
    pub fn sync(&mut self, fonts: &dyn Fonts) -> bool {
        let q = self.ui.text_of(self.entry);
        if q == self.q {
            return false;
        }
        self.q = q;
        self.refresh(fonts);
        true
    }

    /// the rows for what is typed (the host lays the box out again: `ui.layout`)
    pub fn refresh(&mut self, fonts: &dyn Fonts) {
        self.shown = results(&self.q, &self.items, &self.fox);
        self.sel = 0;
        for r in std::mem::take(&mut self.rows) {
            self.ui.destroy(r);
        }
        let (card, th) = (self.th.card.clone(), self.th.clone());
        let fnt = |n: i32, b: bool| Font::new(&self.fu, n, b);
        if self.shown.is_empty() {
            let lab = self.ui.label(self.list, "Nothing found", &card, &th.grey, fnt(10, false));
            self.ui.ws[lab].pady = 10;
            self.ui.pack(lab, Pack::default().fill(Fill::X));
            self.rows.push(lab);
        }
        for i in 0..self.shown.len() {
            let it = self.shown[i].clone();
            let row = self.ui.frame(self.list, &card);
            self.ui.pack(row, Pack::default().fill(Fill::X).padx(8, 8).pady(1, 1));
            let kind = self.ui.label(row, &it.kind, &card, &mixhex(&card, &th.gold, 0.8), fnt(8, true));
            self.ui.ws[kind].opt_w = 8;
            self.ui.set_anchor(kind, Anchor::W);
            self.ui.pack(kind, Pack::default().side(Side::Left).padx(8, 4).pady(6, 6));
            let tl = self.ui.label(row, &it.title, &card, &th.ink, fnt(11, false));
            self.ui.set_anchor(tl, Anchor::W);
            self.ui.pack(tl, Pack::default().side(Side::Left));
            let sub = self.ui.label(row, &it.sub, &card, &th.grey, fnt(9, false));
            self.ui.set_anchor(sub, Anchor::E);
            self.ui.pack(sub, Pack::default().side(Side::Right).padx(10, 10));
            self.rows.push(row);
        }
        crate::pagekit::zk_fonts(&mut self.ui, fonts, 0, self.zk_f);
        self.paint();
    }

    pub fn paint(&mut self) {
        let card = self.th.card.clone();
        let hi = mixhex(&card, &self.th.acc, 0.22);
        for (i, &row) in self.rows.iter().enumerate() {
            if !matches!(self.ui.ws[row].kind, Kind::Frame) {
                continue;
            }
            let bg = if i == self.sel { hi.clone() } else { card.clone() };
            self.ui.ws[row].bg = bg.clone();
            for c in self.ui.ws[row].kids.clone() {
                self.ui.ws[c].bg = bg.clone();
            }
        }
    }

    pub fn select(&mut self, i: usize) {
        self.sel = i;
        self.paint();
    }

    pub fn move_by(&mut self, d: i32) {
        if !self.shown.is_empty() {
            let n = self.shown.len() as i32;
            self.sel = ((self.sel as i32 + d) % n + n) as usize % n as usize;
            self.paint();
        }
    }

    /// the row under (x, y), if any
    pub fn row_at(&self, x: i32, y: i32) -> Option<usize> {
        if self.shown.is_empty() {
            return None;
        }
        self.rows.iter().position(|&r| {
            let (ax, ay) = self.ui.abs(r);
            let w = &self.ui.ws[r];
            x >= ax && x < ax + w.w && y >= ay && y < ay + w.h
        })
    }

    /// what picking row `i` does
    pub fn pick(&self, i: usize) -> Option<String> {
        self.shown.get(i).map(|it| it.run.clone())
    }

    /// where the box goes over a main window at (hx, hy), hw x hh
    pub fn place(&self, hx: i32, hy: i32, hw: i32, hh: i32) -> (i32, i32) {
        (hx + (hw - self.w).div_euclid(2), hy + 40.max(hh.div_euclid(7)))
    }
}
