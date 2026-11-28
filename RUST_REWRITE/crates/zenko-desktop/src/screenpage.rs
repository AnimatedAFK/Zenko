//! The hub's Screen time page (zenko.py hub_build_screen / screen_render / screen_list): the Today / 7 / 30 days
//! switch with Pause / Resume, four tiles, the by-hour or by-day chart, the websites and apps lists with their bars,
//! and the "Never tracked" card (two switches, the skipped words as chips, add one, reset, forget today, delete all).

use crate::hubwin::{canvas_of, hub_title_draw};
use crate::pagekit::{Faces, Scroller, WrapState, hub_card, hub_scroller, label_w, tt_btn, tt_entry, zk_fonts, zk_wrap};
use crate::rempage::rbox_pub as rbox;
use crate::routes17::{screen_fmt, screen_range_at};
use crate::theme::Theme;
use crate::tkcanvas::Photo;
use crate::ui::{Anchor, Fill, Font, Fonts, Kind, Pack, Side, Ui};
use serde_json::Value;
use std::rc::Rc;
use zenko_art::palette::mixhex;

pub const TITLE: &str = "Screen time";
pub const SUB: &str = "\u{753b}\u{9762} \u{b7} Where your day went. Private and incognito windows, banking and password apps \
are never recorded, and nothing leaves this PC.";
const PRIVACY: &str = "Private windows are skipped: Edge InPrivate, Firefox and Brave private windows, and Chrome Incognito \
(Chrome only labels its first Incognito tab, so once I see it I skip that whole window). Anything matching a word below is \
skipped too, by window title or app name, and is not counted at all. Unsure? Press Pause.";

/// what the page shows: life["screen"], the switches, the skipped words, the clock
#[derive(Clone, Debug, Default)]
pub struct ScreenData {
    pub life: Value,
    pub screen_time: bool,
    pub nudges: bool,
    pub exclude: Vec<String>,
    /// today's day number (local), the hour now and time.time()
    pub today: i64,
    pub hour: i64,
    pub now: f64,
    /// _mincho_font() or _jp_font() or FONT_UI, and _serif_font()
    pub jp: String,
    pub serif: String,
}

pub struct ScreenPage {
    pub pg: usize,
    pub title: usize,
    pub sc: Scroller,
    pub body: usize,
    /// screen_ui["range"] and the range drawn last ("drawn")
    pub range: i64,
    drawn: i64,
    /// the yview top to go back to after a render of the same range
    keep: Option<f64>,
    pub entry: usize,
    pub toggles: Vec<(usize, &'static str)>,
    /// the chart: (canvas, its bars (label, seconds), the bar that's now)
    chart: (usize, Vec<(String, f64)>, usize),
    /// each list row's bar: (canvas, its share of the longest)
    bars: Vec<(usize, f64)>,
    drawn_w: Vec<i32>,
    pub wrap: WrapState,
    seig: Option<(String, Rc<Photo>)>,
}

fn justify_left(ui: &mut Ui, l: usize) {
    if let Kind::Label { left, .. } = &mut ui.ws[l].kind {
        *left = true;
    }
}

fn browsers() -> Vec<String> {
    let rd = crate::routes17::routes_data();
    let names = &rd["SCREEN_APP_NAMES"];
    rd["BROWSERS"].as_array().unwrap().iter().filter_map(|b| names.get(b.as_str().unwrap()).and_then(|v| v.as_str()))
        .map(String::from).collect()
}

impl ScreenPage {
    /// hub_build_screen(pg): the title, the scroller and its body, then screen_render()
    pub fn build(ui: &mut Ui, pg: usize, th: &Theme, f: &Faces, cm10: i32, range: i64, d: &ScreenData) -> ScreenPage {
        let title = ui.canvas(pg, &th.banner, cm10, 96);
        canvas_of(ui, title);
        ui.pack(title, Pack::default().fill(Fill::X));
        let sc = hub_scroller(ui, pg, th, cm10);
        let body = ui.frame(sc.inner, &th.paper);
        ui.pack(body, Pack::default().fill(Fill::Both).expand().padx(20, 20).pady(12, 16));
        let mut p = ScreenPage { pg, title, sc, body, range, drawn: 0, keep: None, entry: 0, toggles: vec![],
                                 chart: (0, vec![], 0), bars: vec![], drawn_w: vec![], wrap: WrapState::default(),
                                 seig: None };
        p.render(ui, th, f, cm10, d);
        p
    }

    /// screen_render(): the page's body made again for the range (the scroll kept when the range is the same)
    pub fn render(&mut self, ui: &mut Ui, th: &Theme, f: &Faces, cm10: i32, d: &ScreenData) {
        self.keep = Some(if self.drawn == self.range { self.sc.yview(ui).0 } else { 0.0 });
        self.drawn = self.range;
        for k in ui.ws[self.body].kids.clone() {
            ui.destroy(k);
        }
        self.wrap = WrapState { lastw: self.wrap.lastw, ..WrapState::default() };
        self.bars.clear();
        self.toggles.clear();
        self.drawn_w.clear();
        let body = self.body;
        let bg = th.card.clone();
        let days = self.range;
        let r = screen_range_at(&d.life, days, d.today);
        let paused_until = d.life["screen"]["paused_until"].as_f64().unwrap_or(0.0);
        // range switch + pause
        let bar = ui.frame(body, &th.paper);
        ui.pack(bar, Pack::default().fill(Fill::X));
        for (lab, n) in [("Today", 1), ("Last 7 days", 7), ("Last 30 days", 30)] {
            let on = n == days;
            let b = tt_btn(ui, bar, lab, &format!("scr_range:{n}"), Some(if on { &th.acc } else { &th.field }),
                           Some(if on { &th.onacc } else { &th.ink }), 9, 10, 2, th, f);
            ui.pack(b, Pack::default().side(Side::Left).padx(0, 6));
        }
        if d.now < paused_until {
            let left = ((paused_until - d.now) / 60.0) as i64 + 1;
            let b = tt_btn(ui, bar, "Resume tracking", "scr_resume", Some(&th.start), None, 9, 10, 2, th, f);
            ui.pack(b, Pack::default().side(Side::Right));
            let l = ui.label(bar, &format!("Paused for {left} more min"), &th.paper, &th.gold, Font::new(&f.ui, 9, true));
            ui.pack(l, Pack::default().side(Side::Right).padx(8, 8));
        } else if d.screen_time {
            let b = tt_btn(ui, bar, "Pause 1 hour", "scr_pause", None, None, 9, 10, 2, th, f);
            ui.pack(b, Pack::default().side(Side::Right));
        } else {
            let l = ui.label(bar, "Screen time is off (turn it on below)", &th.paper, &th.grey, Font::new(&f.ui, 9, false));
            ui.pack(l, Pack::default().side(Side::Right));
        }
        // tiles
        let tiles = ui.frame(body, &th.paper);
        ui.pack(tiles, Pack::default().fill(Fill::X).pady(12, 0));
        let span = if days == 1 { "today".to_string() } else { format!("in {days} days") };
        let top = &r.top;
        let mut sites = r.sites.clone();
        sites.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        sites.retain(|kv| kv.0 != "Other websites" && kv.0 != "New tab");
        let br = browsers();
        let mut apps = r.apps.clone();
        apps.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        let in_br = zenko_sound::music::py_sum(apps.iter().filter(|a| br.contains(&a.0)).map(|a| a.1));
        let top_app = apps.iter().find(|a| !br.contains(&a.0));
        let tile_data = [
            ("\u{753b}\u{9762}", screen_fmt(r.total), format!("screen time {span}{}", if days == 1 { String::new() } else {
                format!(" ({} a day)", screen_fmt(r.total / days as f64)) })),
            ("\u{4e00}\u{756a}", top.first().map(|t| t.0.clone()).unwrap_or_else(|| "\u{2014}".into()),
             top.first().map(|t| format!("longest: {}", screen_fmt(t.1))).unwrap_or_else(|| "longest".into())),
            ("\u{7db2}", screen_fmt(in_br), "in the browser".into()),
            ("\u{5668}", top_app.map(|a| a.0.clone()).unwrap_or_else(|| "\u{2014}".into()),
             format!("top app: {}", screen_fmt(top_app.map(|a| a.1).unwrap_or(0.0)))),
        ];
        for (kan, big, small) in tile_data {
            let t = hub_card(ui, tiles, None, "round", th, f);
            ui.pack(t, Pack::default().side(Side::Left).fill(Fill::Both).expand().padx(0, 10));
            let l = ui.label(t, kan, &bg, &mixhex(&th.gold, &bg, 0.35), Font::new(&d.jp, 11, false));
            ui.pack(l, Pack::default().anchor(Anchor::W).padx(14, 14).pady(10, 0));
            let l = label_w(ui, t, &crate::extra::clip(&big, 22), &bg, &th.ink, &f.ui, 16, true);
            ui.pack(l, Pack::default().fill(Fill::X).padx(14, 14));
            let l = label_w(ui, t, &small, &bg, &th.grey, &f.ui, 9, false);
            justify_left(ui, l);
            ui.pack(l, Pack::default().fill(Fill::X).padx(14, 14).pady(0, 12));
        }
        // chart
        let c = hub_card(ui, body, Some(if days == 1 { "\u{6642} \u{b7} By hour" } else { "\u{65e5} \u{b7} By day" }), "round",
                         th, f);
        ui.pack(c, Pack::default().fill(Fill::X).pady(12, 0));
        let cv = ui.canvas(c, &bg, cm10, 150);
        canvas_of(ui, cv);
        ui.pack(cv, Pack::default().fill(Fill::X).padx(14, 14).pady(0, 12));
        let (vals, nowi) = if days == 1 {
            (r.hours.iter().enumerate().map(|(h, &v)| (if h % 3 == 0 { h.to_string() } else { String::new() }, v)).collect(),
             d.hour as usize)
        } else {
            let n = r.days.len();
            let v: Vec<(String, f64)> = r.days.iter().enumerate().map(|(i, (_, v))| {
                let dn = d.today - (n - 1 - i) as i64;
                let lab = if days <= 7 {
                    ["Mo", "Tu", "We", "Th", "Fr", "Sa", "Su"][crate::lifechat::weekday(dn)].to_string()
                } else {
                    let dd = crate::lifechat::ymd(dn).2;
                    if dd % 5 == 0 || dd == 1 { dd.to_string() } else { String::new() }
                };
                (lab, *v)
            }).collect();
            (v, n.saturating_sub(1))
        };
        self.chart = (cv, vals, nowi);
        // websites + apps
        let cols = ui.frame(body, &th.paper);
        ui.pack(cols, Pack::default().fill(Fill::X).pady(12, 0));
        let left = ui.frame(cols, &th.paper);
        ui.pack(left, Pack::default().side(Side::Left).fill(Fill::Both).expand().padx(0, 6));
        let right = ui.frame(cols, &th.paper);
        ui.pack(right, Pack::default().side(Side::Left).fill(Fill::Both).expand().padx(6, 0));
        let other = r.sites.iter().find(|s| s.0 == "Other websites").map(|s| s.1).unwrap_or(0.0);
        let empty = format!("Sites show up here when you browse (private windows never do).{}",
                            if other == 0.0 { String::new() } else { format!("  Other pages: {}.", screen_fmt(other)) });
        self.list(ui, th, f, cm10, d, left, "\u{7db2} \u{b7} Websites", &sites, &empty);
        self.list(ui, th, f, cm10, d, right, "\u{5668} \u{b7} Apps", &apps, "Apps show up here as you use them.");
        // privacy
        let p = hub_card(ui, body, Some("\u{79d8} \u{b7} Never tracked"), "round", th, f);
        ui.pack(p, Pack::default().fill(Fill::X).pady(12, 0));
        let l = label_w(ui, p, PRIVACY, &bg, &th.grey, &f.ui, 9, false);
        justify_left(ui, l);
        ui.pack(l, Pack::default().fill(Fill::X).padx(16, 16).pady(0, 8));
        for (text, on, key) in [("Track my screen time (on this PC only)", d.screen_time, "screen_time"),
                                ("Tell me each time a site or app passes another hour today", d.nudges, "screen_nudges")] {
            let ck = ui.check(p, text, &bg, &th.ink, Font::new(&f.ui, 10, false), on, &th.field, &format!("scr_tog:{key}"),
                              0, 2);
            ui.pack(ck, Pack::default().fill(Fill::X).padx(12, 12));
            self.toggles.push((ck, key));
        }
        let l = label_w(ui, p, "Skipped words and apps (tap one to remove it)", &bg, &th.ink, &f.ui, 9, true);
        ui.pack(l, Pack::default().fill(Fill::X).padx(16, 16).pady(10, 4));
        let chips = ui.frame(p, &bg);
        ui.pack(chips, Pack::default().fill(Fill::X).padx(16, 16));
        for wd in &d.exclude {
            let b = tt_btn(ui, chips, &format!("{wd}  \u{d7}"), &format!("scr_rm:{wd}"), None, None, 8, 7, 1, th, f);
            ui.pack(b, Pack::default().side(Side::Left).padx(0, 4).pady(2, 2));
        }
        let r1 = ui.frame(p, &bg);
        ui.pack(r1, Pack::default().fill(Fill::X).padx(16, 16).pady(8, 4));
        let e = tt_entry(ui, r1, "", 22, th, f);
        ui.pack(e, Pack::default().side(Side::Left));
        self.entry = e;
        let b = tt_btn(ui, r1, "Skip this too", "scr_add", Some(&th.start), None, 9, 10, 1, th, f);
        ui.pack(b, Pack::default().side(Side::Left).padx(6, 6));
        let l = label_w(ui, p, "Examples: a bank's name, an app like 'keepass', or a word in a window title.", &bg, &th.grey,
                        &f.ui, 8, false);
        ui.pack(l, Pack::default().fill(Fill::X).padx(16, 16));
        let r2 = ui.frame(p, &bg);
        ui.pack(r2, Pack::default().fill(Fill::X).padx(16, 16).pady(10, 14));
        for (i, (lab, cmd)) in [("Reset the list", "scr_reset"), ("Forget today", "scr_clear:today"),
                                ("Delete all screen time", "scr_clear:all")].into_iter().enumerate() {
            let b = tt_btn(ui, r2, lab, cmd, None, None, 8, 8, 1, th, f);
            let pk = Pack::default().side(Side::Left);
            ui.pack(b, if i == 1 { pk.padx(6, 6) } else { pk });
        }
    }

    /// screen_list(parent, title, rows, empty)
    #[allow(clippy::too_many_arguments)]
    fn list(&mut self, ui: &mut Ui, th: &Theme, f: &Faces, cm10: i32, d: &ScreenData, parent: usize, title: &str,
            rows: &[(String, f64)], empty: &str) {
        let bg = th.card.clone();
        let c = hub_card(ui, parent, Some(title), "round", th, f);
        ui.pack(c, Pack::default().fill(Fill::Both).expand());
        if rows.is_empty() {
            let l = ui.label(c, empty, &bg, &th.grey, Font { family: d.serif.clone(), size: 10, bold: false, italic: true });
            ui.set_anchor(l, Anchor::W);
            justify_left(ui, l);
            ui.pack(l, Pack::default().fill(Fill::X).padx(16, 16).pady(0, 14));
            return;
        }
        let mx = rows.iter().map(|r| r.1).fold(f64::NEG_INFINITY, f64::max);
        let mx = if mx == 0.0 { 1.0 } else { mx };
        for (name, v) in rows.iter().take(9) {
            let rr = ui.frame(c, &bg);
            ui.pack(rr, Pack::default().fill(Fill::X).padx(16, 16).pady(2, 0));
            let l = ui.label(rr, &screen_fmt(*v), &bg, &th.gold, Font::new(&f.ui, 9, true));
            ui.pack(l, Pack::default().side(Side::Right));
            let l = label_w(ui, rr, &crate::extra::clip(name, 30), &bg, &th.ink, &f.ui, 10, false);
            ui.pack(l, Pack::default().side(Side::Left).fill(Fill::X).expand());
            let b = ui.canvas(c, &bg, cm10, 6);
            canvas_of(ui, b);
            ui.pack(b, Pack::default().fill(Fill::X).padx(16, 16).pady(1, 4));
            self.bars.push((b, v / mx));
        }
        if rows.len() > 9 {
            let rest = zenko_sound::music::py_sum(rows[9..].iter().map(|r| r.1));
            let l = label_w(ui, c, &format!("+ {} more ({})", rows.len() - 9, screen_fmt(rest)), &bg, &th.grey, &f.ui, 9,
                            false);
            ui.pack(l, Pack::default().fill(Fill::X).padx(16, 16));
        }
        let sp = ui.frame(c, &bg);
        ui.ws[sp].opt_h = 10;
        ui.pack(sp, Pack::default());
    }

    /// the chart's and the bars' <Configure> draws, for the widths they have now
    fn draw(&mut self, ui: &mut Ui, th: &Theme, f: &Faces) {
        let bg = th.card.clone();
        let ws: Vec<i32> = std::iter::once(self.chart.0).chain(self.bars.iter().map(|b| b.0))
            .map(|id| ui.ws[id].w * 10000 + ui.ws[id].h).collect();
        if ws == self.drawn_w {
            return;
        }
        self.drawn_w = ws;
        let (id, vals, nowi) = (self.chart.0, self.chart.1.clone(), self.chart.2);
        let (w, h) = (ui.ws[id].w as f64, ui.ws[id].h as f64);
        let cv = canvas_of(ui, id);
        *cv = Default::default();
        if w >= 50.0 {
            let mx = vals.iter().map(|v| v.1).fold(60.0f64, f64::max);
            let n = vals.len() as f64;
            let gap = (2i64).max((w / n * 0.25) as i64) as f64;
            let bw = (w - 30.0) / n;
            let base = h - 20.0;
            for frac in [0.5, 1.0] {
                let y = base - (base - 12.0) * frac;
                cv.line(vec![30.0, y, w, y]).fill(&mixhex(&th.line, &bg, 0.4)).dash(&[2, 4]);
                cv.text(26.0, y, &screen_fmt(mx * frac)).fill(&th.grey).font(Font::new(&f.ui, 7, false)).anchor("e");
            }
            for (i, (lab, v)) in vals.iter().enumerate() {
                let x0 = 30.0 + i as f64 * bw + gap / 2.0;
                let x1 = 30.0 + (i + 1) as f64 * bw - gap / 2.0;
                let hgt = (base - 12.0) * (v / mx);
                let col = if i == nowi { th.acc.clone() } else { mixhex(&th.acc, &bg, 0.45) };
                if *v > 0.0 {
                    rbox(cv, x0, base - hgt.max(3.0), x1, base, 3, &col, "", &[], "round");
                }
                if !lab.is_empty() {
                    cv.text((x0 + x1) / 2.0, base + 10.0, lab).fill(&th.grey).font(Font::new(&f.ui, 8, false));
                }
            }
        }
        for &(b, fr) in &self.bars.clone() {
            let w = ui.ws[b].w as f64;
            let cv = canvas_of(ui, b);
            *cv = Default::default();
            rbox(cv, 0.0, 0.0, 6f64.max((w - 1.0) * fr), 5.0, 99, &mixhex(&th.acc, &bg, 0.2), "", &[], "pill");
        }
    }

    /// zk_fit (fonts, zk_wrap), the title for the width, the chart and bars drawn, the kept scroll put back
    #[allow(clippy::too_many_arguments)]
    pub fn fit(&mut self, ui: &mut Ui, fonts: &dyn Fonts, th: &Theme, f: &Faces, zk_f: f64, pw: i32, ph: i32) {
        zk_fonts(ui, fonts, self.pg, zk_f);
        ui.layout_at(fonts, pw, ph);
        let w = ui.ws[self.title].w;
        if w >= 20 {
            let mut cv = std::mem::take(canvas_of(ui, self.title));
            let hh = hub_title_draw(&mut cv, w, TITLE, SUB, th, &f.ui, &f.title, zk_f, fonts, &mut self.seig);
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
        if let Some(k) = self.keep.take() {
            let rh = ui.ws[self.sc.inner].rh as f64;
            self.sc.set_y(ui, (k * rh + 0.5) as i32);
            ui.layout_at(fonts, pw, ph);
        }
        self.draw(ui, th, f);
    }
}

/// screen_exclude_edit(word, add): None resets to the defaults, Some(true) adds the words (split at , and ;, lower
/// case, ".exe" cut), Some(false) removes the word
pub fn exclude_edit(ex: &mut Vec<String>, word: &str, add: Option<bool>) {
    match add {
        None => {
            *ex = crate::routes17::routes_data()["SCREEN_EXCLUDE_DEFAULT"].as_array().unwrap().iter()
                .filter_map(|v| v.as_str().map(String::from)).collect();
        }
        Some(true) => {
            for w in word.split([',', ';']) {
                let mut w = w.trim().to_lowercase();
                if w.ends_with(".exe") {
                    w.truncate(w.len() - 4);
                }
                if !w.is_empty() && !ex.contains(&w) {
                    ex.push(w);
                }
            }
        }
        Some(false) => ex.retain(|w| w != word),
    }
}
