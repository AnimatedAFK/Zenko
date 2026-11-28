//! The hub's Daily page (zenko.py hub_build_daily / daily_render and its cards: daily_checkin_card,
//! daily_cards_card, daily_well_card, Zenko22's daily_badges_card, daily_homework_card, daily_facts_card).

use crate::ema::fmt_hm;
use crate::hubwin::{canvas_of, hub_title_draw};
use crate::pagekit::{Faces, Scroller, WrapState, hub_card, hub_scroller, label_w, tt_btn, tt_entry, zk_fonts, zk_wrap};
use crate::session::clip;
use crate::theme::Theme;
use crate::tkcanvas::{Canvas, Photo};
use crate::tktext::{Text, Wrap};
use crate::ui::{Fill, Font, Fonts, Grid, Kind, Pack, Side, Ui};
use serde_json::Value;
use std::rc::Rc;
use zenko_art::palette::mixhex;

pub const TITLE: &str = "Daily";
pub const SUB: &str = "Your streak, ema badges, check-ins, flashcards, homework help and wellbeing";
const SHU: &str = "#c8372d";
const WD: [&str; 7] = ["Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday", "Sunday"];
const MON: [&str; 12] = ["January", "February", "March", "April", "May", "June", "July", "August", "September", "October",
                         "November", "December"];

/// What the page shows, gathered from the app (Companion::daily_data)
#[derive(Clone, Debug, Default, PartialEq)]
pub struct DailyData {
    /// local day number
    pub day: i64,
    pub streak: i64,
    pub total_hours: f64,
    pub mood_word: String,
    pub fox_name: String,
    pub spirit: i64,
    pub hunger: f64,
    /// life: journal, cards, badges, btier, badge_pins, facts, notes, found
    pub life: Value,
    /// the config (badge_view, the reminders, daynight, bedtime)
    pub cfg: Value,
    /// self.well: seconds since each wellbeing reminder
    pub well: Value,
    /// _jp_font(): a Japanese font, or ""
    pub jp: String,
    /// how many flashcards are due (cards_due())
    pub due: usize,
}

pub struct DailyPage {
    pub pg: usize,
    pub title: usize,
    pub sc: Scroller,
    pub body: usize,
    pub moods_cv: usize,
    pub badges_cv: usize,
    pub tip: usize,
    pub note: usize,
    pub q: usize,
    pub a: usize,
    pub hw: usize,
    pub fact: usize,
    pub seg: Vec<(String, usize)>,
    pub mode: String,
    /// the badges as drawn: (id, tile x, tile y)
    pub tiles: Vec<(String, f64, f64)>,
    /// the facts / notes rows' × buttons: (button, "fact:<key>" or "note:<text>")
    pub forget: Vec<(usize, String)>,
    pub wrap: WrapState,
    drawn_w: (i32, i32),
    seig: Option<(String, Rc<Photo>)>,
}

fn rdata() -> &'static Value {
    crate::rewards::rdata()
}

fn moods() -> Vec<(String, String, String)> {
    rdata()["MOODS"].as_array().unwrap().iter()
        .map(|m| (m[0].as_str().unwrap().into(), m[1].as_str().unwrap().into(), m[2].as_str().unwrap().into())).collect()
}

fn arr(v: &Value) -> Vec<Value> {
    v.as_array().cloned().unwrap_or_default()
}

fn wrap_left(ui: &mut Ui, l: usize, wl: i32) {
    if let Kind::Label { wrap, left, .. } = &mut ui.ws[l].kind {
        (*wrap, *left) = (wl, true);
    }
}

#[allow(clippy::too_many_arguments)]
fn lbl(ui: &mut Ui, parent: usize, text: &str, bg: &str, fg: &str, fam: &str, size: i32, bold: bool, p: Pack) -> usize {
    let l = label_w(ui, parent, text, bg, fg, fam, size, bold);
    ui.pack(l, p);
    l
}

/// tk.Frame(c, bg=bg, height=10).pack()
fn spacer(ui: &mut Ui, c: usize, bg: &str) {
    let f = ui.frame(c, bg);
    ui.ws[f].opt_h = 10;
    ui.pack(f, Pack::default());
}

/// date.fromisoformat(d).strftime("%d %b")
fn day_mon(iso: &str) -> String {
    let m: usize = iso.get(5..7).and_then(|s| s.parse().ok()).unwrap_or(1);
    format!("{} {}", iso.get(8..10).unwrap_or(""), &MON[m.clamp(1, 12) - 1][..3])
}

/// str.title() for a one-word mood
fn title_case(s: &str) -> String {
    let mut c = s.chars();
    c.next().map(|f| f.to_uppercase().collect::<String>() + &c.as_str().to_lowercase()).unwrap_or_default()
}

impl DailyPage {
    /// hub_build_daily(pg) and the first daily_render()
    pub fn build(ui: &mut Ui, pg: usize, th: &Theme, f: &Faces, cm10: i32, d: &DailyData) -> DailyPage {
        let title = ui.canvas(pg, &th.banner, cm10, 96);
        canvas_of(ui, title);
        ui.pack(title, Pack::default().fill(Fill::X));
        let sc = hub_scroller(ui, pg, th, cm10);
        let body = ui.frame(sc.inner, &th.paper);
        ui.pack(body, Pack::default().fill(Fill::X).padx(24, 24).pady(14, 14));
        let mut p = DailyPage { pg, title, sc, body, moods_cv: 0, badges_cv: 0, tip: 0, note: 0, q: 0, a: 0, hw: 0, fact: 0,
                                seg: vec![], mode: String::new(), tiles: vec![], forget: vec![], wrap: WrapState::default(),
                                drawn_w: (0, 0), seig: None };
        p.render(ui, th, f, cm10, d);
        p
    }

    /// daily_render: everything inside the body again
    pub fn render(&mut self, ui: &mut Ui, th: &Theme, f: &Faces, cm10: i32, d: &DailyData) {
        for k in ui.ws[self.body].kids.clone() {
            ui.destroy(k);
        }
        // (pg._zk_lastw lives on the page, not on the rebuilt widgets)
        self.wrap = WrapState { lastw: self.wrap.lastw, ..WrapState::default() };
        self.drawn_w = (0, 0);
        self.forget.clear();
        let body = self.body;
        let bg = th.card.clone();
        let row = ui.frame(body, &th.paper);
        ui.pack(row, Pack::default().fill(Fill::X));
        let hu = d.hunger;
        let tummy = if hu < 25.0 { "Full" } else if hu < 55.0 { "Peckish" } else if hu < 80.0 { "Hungry" } else { "Starving" };
        let nb = rdata()["BADGES"].as_array().map(|a| a.len()).unwrap_or(0);
        let earned = d.life["badges"].as_object().map(|o| o.len()).unwrap_or(0);
        let stats = [
            (format!("{} day{}", d.streak, if d.streak == 1 { "" } else { "s" }), "focus streak (\u{708e})".to_string()),
            (if d.total_hours >= 1.0 / 60.0 { fmt_hm((d.total_hours * 60.0) as i64) } else { "0m".into() }, "focused in total".into()),
            (d.mood_word.clone(), format!("{}'s mood \u{b7} foxfire {}%", d.fox_name, d.spirit)),
            (tummy.into(), "her tummy (focus feeds her)".into()),
            (format!("{earned} / {nb}"), "ema badges".into()),
        ];
        let capf = if d.jp.is_empty() { f.ui.clone() } else { d.jp.clone() };
        for (i, (big, cap)) in stats.iter().enumerate() {
            let c = hub_card(ui, row, None, "ema", th, f);
            ui.pack(c, Pack::default().side(Side::Left).fill(Fill::Both).expand().padx(if i == 0 { 0 } else { 10 }, 0));
            lbl(ui, c, big, &bg, &th.accd, &f.ui, 17, true, Pack::default().fill(Fill::X).padx(14, 14).pady(10, 0));
            lbl(ui, c, cap, &bg, &th.grey, &capf, 8, false, Pack::default().fill(Fill::X).padx(14, 14).pady(0, 10));
        }
        let cols = ui.frame(body, &th.paper);
        ui.pack(cols, Pack::default().fill(Fill::X).pady(12, 0));
        (ui.ws[cols].gw_cols, ui.ws[cols].gu_cols) = (vec![1, 1], vec!["d", "d"]);
        let left = ui.frame(cols, &th.paper);
        ui.grid(left, Grid { row: 0, col: 0, sticky: "nsew", padx: (0, 6), ..Default::default() });
        let right = ui.frame(cols, &th.paper);
        ui.grid(right, Grid { row: 0, col: 1, sticky: "nsew", padx: (6, 0), ..Default::default() });
        self.checkin_card(ui, th, f, cm10, d, left);
        self.cards_card(ui, th, f, d, left);
        self.well_card(ui, th, f, d, left);
        self.badges_card(ui, th, f, cm10, d, right);
        self.homework_card(ui, th, f, right);
        self.facts_card(ui, th, f, d, right);
    }

    fn checkin_card(&mut self, ui: &mut Ui, th: &Theme, f: &Faces, cm10: i32, d: &DailyData, parent: usize) {
        let bg = th.card.clone();
        let c = hub_card(ui, parent, Some("How are you today?"), "round", th, f);
        ui.pack(c, Pack::default().fill(Fill::X));
        let today = crate::plan::iso(d.day);
        let journal = arr(&d.life["journal"]);
        let mine: Vec<&Value> = journal.iter().filter(|j| j["date"].as_str() == Some(&today)).collect();
        let r = ui.frame(c, &bg);
        ui.pack(r, Pack::default().fill(Fill::X).padx(14, 14).pady(0, 4));
        let note = mine.last().and_then(|j| j["note"].as_str()).unwrap_or("").to_string();
        for (key, lab, col) in moods() {
            let on = mine.last().is_some_and(|j| j["mood"].as_str() == Some(key.as_str()));
            let (b, fg) = if on { (col.clone(), th.onacc.clone()) } else { (mixhex(&bg, &col, 0.35), th.ink.clone()) };
            let x = tt_btn(ui, r, &lab, &format!("daily_mood:{key}"), Some(&b), Some(&fg), 9, 8, 1, th, f);
            ui.pack(x, Pack::default().side(Side::Left).padx(2, 2));
        }
        let e = tt_entry(ui, c, &note, 40, th, f);
        ui.pack(e, Pack::default().fill(Fill::X).padx(16, 16).pady(2, 4).ipady(2));
        self.note = e;
        lbl(ui, c, "Optional note, then tap a mood. It's private, kept only on this PC.", &bg, &th.grey, &f.ui, 8, false,
            Pack::default().fill(Fill::X).padx(16, 16));
        let cv = ui.canvas(c, &bg, cm10, 44);
        canvas_of(ui, cv);
        ui.pack(cv, Pack::default().fill(Fill::X).padx(14, 14).pady(6, 2));
        self.moods_cv = cv;
        let recent: Vec<&Value> = journal.iter().filter(|j| j["note"].as_str().is_some_and(|s| !s.is_empty())).collect();
        for j in recent.iter().rev().take(3) {
            let t = format!("{} \u{b7} {}: {}", day_mon(j["date"].as_str().unwrap_or("")), title_case(j["mood"].as_str().unwrap_or("")),
                            clip(j["note"].as_str().unwrap_or(""), 60));
            lbl(ui, c, &t, &bg, &th.grey, &f.ui, 8, false, Pack::default().fill(Fill::X).padx(16, 16));
        }
        spacer(ui, c, &bg);
    }

    /// the 14-day mood dots for the canvas's width now
    fn draw_moods(&self, ui: &mut Ui, th: &Theme, f: &Faces, d: &DailyData) {
        let bg = th.card.clone();
        let w = ui.ws[self.moods_cv].w;
        let mut by = std::collections::HashMap::new();
        for j in arr(&d.life["journal"]) {
            by.insert(j["date"].as_str().unwrap_or("").to_string(), j["mood"].as_str().unwrap_or("").to_string());
        }
        let cols: std::collections::HashMap<String, String> = moods().into_iter().map(|(k, _, c)| (k, c)).collect();
        let cv = canvas_of(ui, self.moods_cv);
        *cv = Canvas::default();
        let ww = 200.max(w) as f64;
        let step = (ww - 10.0) / 14.0;
        for i in 0..14 {
            let dn = d.day - 13 + i;
            let x = 5.0 + step * i as f64 + step / 2.0;
            let m = by.get(&crate::plan::iso(dn));
            let fill = m.and_then(|m| cols.get(m)).cloned().unwrap_or_else(|| mixhex(&bg, "#000000", 0.25));
            cv.oval(x - 7.0, 6.0, x + 7.0, 20.0).fill(&fill).outline("");
            cv.text(x, 33.0, &WD[crate::lifechat::weekday(dn)][..1]).fill(&th.grey).font(Font::new(&f.ui, 7, false));
        }
    }

    fn cards_card(&mut self, ui: &mut Ui, th: &Theme, f: &Faces, d: &DailyData, parent: usize) {
        let bg = th.card.clone();
        let c = hub_card(ui, parent, Some("Flashcards (they remember what you got wrong)"), "round", th, f);
        ui.pack(c, Pack::default().fill(Fill::X).pady(12, 0));
        let cards = arr(&d.life["cards"]);
        let r = ui.frame(c, &bg);
        ui.pack(r, Pack::default().fill(Fill::X).padx(16, 16));
        let t = format!("{} due today  \u{b7}  {} in your deck", d.due, cards.len());
        let l = ui.label(r, &t, &bg, &th.ink, Font::new(&f.ui, 10, true));
        ui.pack(l, Pack::default().side(Side::Left));
        let (b, fg) = if d.due > 0 { (th.acc.clone(), th.onacc.clone()) } else { (th.field.clone(), th.ink.clone()) };
        let x = tt_btn(ui, r, "Review now", "hub_ask:review flashcards", Some(&b), Some(&fg), 9, 10, 1, th, f);
        ui.pack(x, Pack::default().side(Side::Right));
        let fr = ui.frame(c, &bg);
        ui.pack(fr, Pack::default().fill(Fill::X).padx(16, 16).pady(8, 2));
        let q = ui.label(fr, "Q", &bg, &th.gold, Font::new(&f.ui, 9, true));
        ui.pack(q, Pack::default().side(Side::Left));
        let qe = tt_entry(ui, fr, "", 18, th, f);
        ui.pack(qe, Pack::default().side(Side::Left).padx(4, 6).fill(Fill::X).expand());
        let a = ui.label(fr, "A", &bg, &th.gold, Font::new(&f.ui, 9, true));
        ui.pack(a, Pack::default().side(Side::Left));
        let ae = tt_entry(ui, fr, "", 14, th, f);
        ui.pack(ae, Pack::default().side(Side::Left).padx(4, 6).fill(Fill::X).expand());
        let add = tt_btn(ui, fr, "Add", "daily_card_add", Some(&th.start), None, 9, 8, 0, th, f);
        ui.pack(add, Pack::default().side(Side::Left));
        (self.q, self.a) = (qe, ae);
        let tip = lbl(ui, c, "Tip: 'quiz me on <topic>' in chat makes cards from the web, and the ones you miss are saved here.",
                      &bg, &th.grey, &f.ui, 8, false, Pack::default().fill(Fill::X).padx(16, 16).pady(4, 0));
        wrap_left(ui, tip, 440);
        let mut hard: Vec<&Value> = cards.iter().collect();
        hard.sort_by_key(|x| -x["wrong"].as_i64().unwrap_or(0));
        for x in hard.into_iter().filter(|x| x["wrong"].as_i64().unwrap_or(0) != 0).take(3) {
            let t = format!("\u{2022} {} \u{2192} {}  (missed {}x)", clip(x["q"].as_str().unwrap_or(""), 40),
                            clip(x["a"].as_str().unwrap_or(""), 20), x["wrong"].as_i64().unwrap_or(0));
            lbl(ui, c, &t, &bg, &th.grey, &f.ui, 8, false, Pack::default().fill(Fill::X).padx(16, 16));
        }
        spacer(ui, c, &bg);
    }

    fn well_card(&mut self, ui: &mut Ui, th: &Theme, f: &Faces, d: &DailyData, parent: usize) {
        let bg = th.card.clone();
        let c = hub_card(ui, parent, Some("Wellbeing"), "round", th, f);
        ui.pack(c, Pack::default().fill(Fill::X).pady(12, 0));
        let r = ui.frame(c, &bg);
        ui.pack(r, Pack::default().fill(Fill::X).padx(16, 16).pady(0, 6));
        let b = tt_btn(ui, r, "Breathing exercise", "daily_breathe", Some(&th.start), None, 9, 10, 1, th, f);
        ui.pack(b, Pack::default().side(Side::Left));
        let b = tt_btn(ui, r, "Daily check-in", "daily_checkin", Some(&th.field), None, 9, 10, 1, th, f);
        ui.pack(b, Pack::default().side(Side::Left).padx(6, 6));
        let on = |k: &str| d.cfg.get(k).map(|v| v.as_bool().unwrap_or(!v.is_null())).unwrap_or(true);
        let mut bits = vec![];
        for (k, key, dflt, lab) in [("eyes", "remind_eyes", 20, "20-20-20 eye break"), ("posture", "remind_posture", 40, "posture"),
                                    ("water", "remind_water", 60, "water")] {
            if on(key) {
                let every = d.cfg.get(format!("{key}_every")).and_then(|v| v.as_f64()).filter(|v| *v != 0.0)
                    .map(|v| v as i64).unwrap_or(dflt);
                let secs = d.well.get(k).and_then(|v| v.as_f64()).unwrap_or(0.0);
                let left = (every - (secs / 60.0).floor() as i64).max(0);
                bits.push(format!("{lab} in ~{left} min"));
            }
        }
        let t = if bits.is_empty() { "Reminders are off (Settings > Wellbeing).".to_string() }
                else { format!("Next reminders: {}", bits.join(", ")) };
        let l = lbl(ui, c, &t, &bg, &th.grey, &f.ui, 8, false, Pack::default().fill(Fill::X).padx(16, 16));
        wrap_left(ui, l, 440);
        if on("daynight") {
            let bed = d.cfg.get("bedtime").and_then(|v| v.as_str()).unwrap_or("23:00");
            lbl(ui, c, &format!("Bedtime: {bed} (I'll nudge you to sleep after that)."), &bg, &th.grey, &f.ui, 8, false,
                Pack::default().fill(Fill::X).padx(16, 16));
        }
        spacer(ui, c, &bg);
    }

    fn badges_card(&mut self, ui: &mut Ui, th: &Theme, f: &Faces, cm10: i32, d: &DailyData, parent: usize) {
        let bg = th.card.clone();
        let c = hub_card(ui, parent, Some("Ema badges"), "round", th, f);
        ui.pack(c, Pack::default().fill(Fill::X));
        let earned = d.life["badges"].as_object().cloned().unwrap_or_default();
        let all = arr(&rdata()["BADGES"]);
        let top = ui.frame(c, &bg);
        ui.pack(top, Pack::default().fill(Fill::X).padx(16, 16).pady(0, 4));
        let n = all.iter().filter(|b| earned.contains_key(b[0].as_str().unwrap_or(""))).count();
        let t = format!("{n} of {} earned  \u{b7}  click an earned plaque to pin it to the sidebar (up to 3)", all.len());
        let l = label_w(ui, top, &t, &bg, &th.grey, &f.ui, 9, false);
        if let Kind::Label { left, .. } = &mut ui.ws[l].kind {
            *left = true;
        }
        ui.pack(l, Pack::default().side(Side::Left).fill(Fill::X).expand());
        let cv = ui.canvas(c, &bg, cm10, 200);
        canvas_of(ui, cv);
        ui.pack(cv, Pack::default().fill(Fill::X).padx(12, 12).pady(2, 4));
        let tip = lbl(ui, c, "Hover a plaque to see how to earn it.", &bg, &th.grey, &f.ui, 9, false,
                      Pack::default().fill(Fill::X).padx(16, 16).pady(0, 6));
        wrap_left(ui, tip, 520);
        let seg = ui.frame(c, &bg);
        ui.pack(seg, Pack::default().fill(Fill::X).padx(14, 14).pady(0, 12));
        self.mode = d.cfg.get("badge_view").and_then(|v| v.as_str()).unwrap_or("all").to_string();
        self.seg.clear();
        for (k, lab) in [("all", "All"), ("earned", "Earned"), ("tiers", "Bronze \u{b7} Silver \u{b7} Gold"), ("secret", "Secret")] {
            let on = self.mode == k;
            let (b, fg) = if on { (th.acc.clone(), th.onacc.clone()) } else { (th.field.clone(), th.ink.clone()) };
            let x = tt_btn(ui, seg, lab, &format!("daily_badges:{k}"), Some(&b), Some(&fg), 9, 9, 1, th, f);
            ui.pack(x, Pack::default().side(Side::Left).padx(0, 6));
            self.seg.push((k.into(), x));
        }
        (self.badges_cv, self.tip) = (cv, tip);
    }

    /// the badge plaques for the canvas's width now; true when the canvas's height changed (lay out again)
    pub fn draw_badges(&mut self, ui: &mut Ui, th: &Theme, f: &Faces, d: &DailyData) -> bool {
        let w = ui.ws[self.badges_cv].w;
        if w < 60 {
            return false;
        }
        let bg = th.card.clone();
        let earned = d.life["badges"].as_object().cloned().unwrap_or_default();
        let pins: Vec<String> = arr(&d.life["badge_pins"]).iter().filter_map(|v| v.as_str().map(String::from)).collect();
        let tiered: Vec<String> = arr(&rdata()["BADGE_TIERED"]).iter().map(|b| b[0].as_str().unwrap_or("").to_string()).collect();
        let secret: Vec<String> = arr(&rdata()["BADGE_SECRET"]).iter().filter_map(|v| v.as_str().map(String::from)).collect();
        let want = self.mode.clone();
        let items: Vec<Value> = arr(&rdata()["BADGES"]).into_iter().filter(|b| {
            let id = b[0].as_str().unwrap_or("");
            want == "all" || (want == "earned" && earned.contains_key(id)) || (want == "tiers" && tiered.iter().any(|t| t == id))
                || (want == "secret" && secret.iter().any(|t| t == id))
        }).collect();
        let (cw, chh) = (74.0, 94.0);
        let per = 3.max(((w - 4) as f64 / cw).floor() as usize);
        let rows = 1.max(items.len().div_ceil(per));
        let tier_col = crate::hubside::BADGE_TIER_COL;
        let jp = d.jp.clone();
        let tier_of = |id: &str| d.life["btier"].get(id).and_then(|v| v.as_i64()).unwrap_or(0);
        self.tiles.clear();
        let cv = canvas_of(ui, self.badges_cv);
        *cv = Canvas::default();
        for (i, b) in items.iter().enumerate() {
            let (bid, kanji, name) = (b[0].as_str().unwrap_or(""), b[1].as_str().unwrap_or(""), b[2].as_str().unwrap_or(""));
            let x = (i % per) as f64 * cw + 6.0;
            let y = (i / per) as f64 * chh + 4.0;
            let got = earned.contains_key(bid);
            let sec = secret.iter().any(|s| s == bid) && !got;
            let tier = if got && tiered.iter().any(|t| t == bid) { tier_of(bid) } else { 0 };
            let wood = if got { "#c98a35".to_string() } else { mixhex(&bg, "#000000", 0.25) };
            let edge = if tier > 0 { tier_col[tier.min(3) as usize].to_string() } else if got { "#7a4a1c".into() } else { th.line.clone() };
            let tg = format!("b{i}");
            cv.poly(vec![x + 31.0, y, x + 62.0, y + 14.0, x + 62.0, y + 60.0, x, y + 60.0, x, y + 14.0]).fill(&wood).outline(&edge)
                .width(if tier > 0 { 3.0 } else { 2.0 }).tags(&[&tg]);
            cv.line(vec![x + 22.0, y + 7.0, x + 40.0, y + 7.0]).fill(if got { SHU } else { &th.line }).width(2.0).tags(&[&tg]);
            for k in 0..tier {
                let cx = x + 31.0 + (k as f64 - (tier - 1) as f64 / 2.0) * 9.0;
                cv.oval(cx - 3.0, y + 50.0, cx + 3.0, y + 56.0).fill(tier_col[tier.min(3) as usize]).outline("#3a2414").tags(&[&tg]);
            }
            if pins.iter().any(|p| p == bid) {
                cv.text(x + 56.0, y + 20.0, if jp.is_empty() { "*" } else { "\u{1f4cc}" }).font(Font::new(&f.ui, 8, false))
                    .fill(&th.ink).tags(&[&tg]);
            }
            let label = if sec { "?".to_string() } else if jp.is_empty() { name.chars().take(1).collect() } else { kanji.to_string() };
            let lf = if jp.is_empty() { Font::new(&f.ui, 16, true) } else { Font::new(&jp, 20, true) };
            cv.text(x + 31.0, y + 34.0, &label).fill(if got { "#3a2414" } else { &th.grey }).font(lf).tags(&[&tg]);
            cv.text(x + 31.0, y + 76.0, if sec { "???" } else { name }).fill(if got { &th.ink } else { &th.grey })
                .font(Font::new(&f.ui, 7, got)).wrap(cw - 4.0).tags(&[&tg]);
            self.tiles.push((bid.to_string(), x, y));
        }
        let h = (rows as f64 * chh + 4.0) as i32;
        let changed = ui.ws[self.badges_cv].opt_h != h;
        ui.ws[self.badges_cv].opt_h = h;
        changed
    }

    /// info(bid): what the tip line says over a plaque (`metrics` = z22_metrics, for the tiered ones)
    pub fn badge_info(d: &DailyData, bid: &str, metrics: &serde_json::Map<String, Value>) -> String {
        let b = arr(&rdata()["BADGES"]).into_iter().find(|b| b[0] == bid).unwrap_or_default();
        let (name, how) = (b[2].as_str().unwrap_or(""), b[3].as_str().unwrap_or(""));
        let earned = &d.life["badges"];
        let got = earned.get(bid).is_some();
        if !got && arr(&rdata()["BADGE_SECRET"]).iter().any(|s| s == bid) {
            return "A secret badge. Keep exploring and studying, and it might find you~".into();
        }
        if let Some(tb) = arr(&rdata()["BADGE_TIERED"]).into_iter().find(|t| t[0] == bid) {
            let names = arr(&rdata()["BADGE_TIER_NAME"]);
            let v = metrics.get(tb[3].as_str().unwrap_or("")).cloned().unwrap_or(Value::from(0));
            let tier = if got { d.life["btier"].get(bid).and_then(|v| v.as_i64()).unwrap_or(0) } else { 0 } as usize;
            let steps: Vec<i64> = arr(&tb[4]).iter().map(|x| x.as_i64().unwrap_or(0)).collect();
            let prog = if v.is_i64() || v.is_u64() { format!("{}", v.as_i64().unwrap_or(0)) } else { format!("{:.1}", v.as_f64().unwrap_or(0.0)) };
            let tn = |t: usize| names.get(t).and_then(|v| v.as_str()).unwrap_or("").to_string();
            let rule = tb[5].as_str().unwrap_or("").replace("{n}", &steps.iter().map(|x| x.to_string()).collect::<Vec<_>>().join("/"));
            let s = format!("{name}{}: {rule}.", if tier > 0 { format!(" ({})", tn(tier)) } else { String::new() });
            return if tier < 3 { format!("{s}  You're at {prog}; {} at {}.", tn(tier + 1), steps[tier]) }
                   else { format!("{s}  Gold! The very top~") };
        }
        if got {
            format!("{name} - earned {}. {how}", earned[bid].as_str().unwrap_or(""))
        } else {
            format!("{name} (locked): {how}")
        }
    }

    /// setm(v): the badge view buttons and the shelf for a new view (the config is saved by the caller)
    pub fn set_mode(&mut self, ui: &mut Ui, th: &Theme, f: &Faces, d: &DailyData, v: &str) -> bool {
        self.mode = v.to_string();
        for (k, b) in &self.seg {
            let on = k == v;
            if let Kind::Button { color, fg, .. } = &mut ui.ws[*b].kind {
                (*color, *fg) = if on { (th.acc.clone(), th.onacc.clone()) } else { (th.field.clone(), th.ink.clone()) };
            }
        }
        self.draw_badges(ui, th, f, d)
    }

    /// the tip line under the shelf (a plaque's <Enter> / <Leave>)
    pub fn set_tip(&self, ui: &mut Ui, s: &str) -> bool {
        if let Kind::Label { text, .. } = &mut ui.ws[self.tip].kind
            && text != s {
            *text = s.into();
            return true;
        }
        false
    }

    /// the plaque under a point of the badges canvas (in its own coordinates)
    pub fn badge_at(&self, x: f64, y: f64) -> Option<String> {
        self.tiles.iter().find(|(_, tx, ty)| x >= *tx && x <= tx + 62.0 && y >= *ty && y <= ty + 86.0).map(|t| t.0.clone())
    }

    fn homework_card(&mut self, ui: &mut Ui, th: &Theme, f: &Faces, parent: usize) {
        let bg = th.card.clone();
        let c = hub_card(ui, parent, Some("Homework buddy"), "round", th, f);
        ui.pack(c, Pack::default().fill(Fill::X).pady(12, 0));
        let b = tt_btn(ui, c, "Open the subject Notebook  \u{2192}", "open_hub:notebook", Some(&th.start), None, 9, 10, 1, th, f);
        ui.pack(b, Pack::default().anchor(crate::ui::Anchor::W).padx(16, 16).pady(0, 6));
        let l = lbl(ui, c, "The Notebook has a tutor page for each of your subjects. Or paste a question here and I'll guess the subject.",
                    &bg, &th.grey, &f.ui, 8, false, Pack::default().fill(Fill::X).padx(16, 16));
        wrap_left(ui, l, 440);
        let mut t = Text::new(Font::new(&f.ui, 10, false), &th.ink, &th.field);
        (t.height_ln, t.wrap, t.hl) = (4, Wrap::Word, 1);
        t.edit = Some(0);
        let id = ui.text(c, t, &th.line);
        ui.pack(id, Pack::default().fill(Fill::X).padx(16, 16).pady(6, 6));
        self.hw = id;
        let b = tt_btn(ui, c, "Explain step by step", "daily_homework", Some(&th.acc), Some(&th.onacc), 9, 10, 1, th, f);
        ui.pack(b, Pack::default().anchor(crate::ui::Anchor::W).padx(16, 16).pady(0, 12));
    }

    fn facts_card(&mut self, ui: &mut Ui, th: &Theme, f: &Faces, d: &DailyData, parent: usize) {
        let bg = th.card.clone();
        let c = hub_card(ui, parent, Some("Things I remember about you"), "round", th, f);
        ui.pack(c, Pack::default().fill(Fill::X).pady(12, 0));
        let facts = d.life["facts"].as_object().cloned().unwrap_or_default();
        let notes = arr(&d.life["notes"]);
        if facts.is_empty() && notes.is_empty() {
            let l = lbl(ui, c, "Tell me things in chat, like \u{201c}my favourite subject is biology\u{201d} or \u{201c}my birthday is 14 May\u{201d}.",
                        &bg, &th.grey, &f.ui, 9, false, Pack::default().fill(Fill::X).padx(16, 16));
            wrap_left(ui, l, 440);
        }
        for (k, v) in facts.iter().take(12) {
            let r = ui.frame(c, &bg);
            ui.pack(r, Pack::default().fill(Fill::X).padx(16, 16).pady(1, 1));
            let mut v = match v {
                Value::String(s) => s.clone(),
                other => other.to_string(),
            };
            if k == "birthday" {
                // date(2000, MM, DD).strftime("%d %B")
                let m: Option<usize> = v.get(..2).and_then(|s| s.parse().ok());
                let dd: Option<u32> = v.get(3..5).and_then(|s| s.parse().ok());
                if let (Some(m), Some(dd)) = (m, dd)
                    && (1..=12).contains(&m) && (1..=31).contains(&dd) {
                    v = format!("{dd:02} {}", MON[m - 1]);
                }
            }
            let l = ui.label(r, &format!("your {k}: {v}"), &bg, &th.ink, Font::new(&f.ui, 9, false));
            ui.set_anchor(l, crate::ui::Anchor::W);
            ui.pack(l, Pack::default().side(Side::Left));
            let b = tt_btn(ui, r, "\u{d7}", &format!("daily_forget:{}", self.forget.len()), Some(&th.field), None, 8, 5, 0, th, f);
            ui.pack(b, Pack::default().side(Side::Right));
            self.forget.push((b, format!("fact:{k}")));
        }
        let skip = notes.len().saturating_sub(6);
        for n in notes.iter().skip(skip) {
            let n = n.as_str().unwrap_or("");
            let r = ui.frame(c, &bg);
            ui.pack(r, Pack::default().fill(Fill::X).padx(16, 16).pady(1, 1));
            let l = ui.label(r, &format!("\u{2022} {}", clip(n, 60)), &bg, &th.ink, Font::new(&f.ui, 9, false));
            ui.set_anchor(l, crate::ui::Anchor::W);
            ui.pack(l, Pack::default().side(Side::Left));
            let b = tt_btn(ui, r, "\u{d7}", &format!("daily_forget:{}", self.forget.len()), Some(&th.field), None, 8, 5, 0, th, f);
            ui.pack(b, Pack::default().side(Side::Right));
            self.forget.push((b, format!("note:{n}")));
        }
        let e = tt_entry(ui, c, "", 40, th, f);
        ui.pack(e, Pack::default().fill(Fill::X).padx(16, 16).pady(6, 2).ipady(2));
        self.fact = e;
        lbl(ui, c, "e.g. my favourite food is ramen  \u{b7}  remember I have piano on Tuesdays", &bg, &th.grey, &f.ui, 8, false,
            Pack::default().fill(Fill::X).padx(16, 16));
        let found = arr(&d.life["found"]);
        if !found.is_empty() {
            let skip = found.len().saturating_sub(5);
            let names: Vec<String> = found.iter().skip(skip).rev().map(|x| x["what"].as_str().unwrap_or("").to_string()).collect();
            let l = lbl(ui, c, &format!("Treasures I found for you: {}", names.join(", ")), &bg, &th.gold, &f.ui, 8, false,
                        Pack::default().fill(Fill::X).padx(16, 16).pady(6, 0));
            wrap_left(ui, l, 440);
        }
        spacer(ui, c, &bg);
    }

    /// the canvases for their widths now (their <Configure> draws); lays out again while a height changes
    pub fn draw(&mut self, ui: &mut Ui, fonts: &dyn Fonts, th: &Theme, f: &Faces, d: &DailyData, pw: i32, ph: i32) {
        for _ in 0..3 {
            let wh = (ui.ws[self.moods_cv].w, ui.ws[self.badges_cv].w);
            if wh == self.drawn_w {
                break;
            }
            self.drawn_w = wh;
            self.draw_moods(ui, th, f, d);
            if self.draw_badges(ui, th, f, d) {
                ui.layout_at(fonts, pw, ph);
            }
        }
    }

    /// zk_fit for the page, the banner and the canvases; lays out at the window's size (pw, ph)
    #[allow(clippy::too_many_arguments)]
    pub fn fit(&mut self, ui: &mut Ui, fonts: &dyn Fonts, th: &Theme, f: &Faces, d: &DailyData, zk_f: f64, pw: i32, ph: i32) {
        zk_fonts(ui, fonts, self.pg, zk_f);
        ui.layout_at(fonts, pw, ph);
        self.draw(ui, fonts, th, f, d, pw, ph);
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
        self.draw(ui, fonts, th, f, d, pw, ph);
        if self.sc.sync(ui) {
            ui.layout_at(fonts, pw, ph);
            self.draw(ui, fonts, th, f, d, pw, ph);
        }
    }
}

/// DailyData from the app as it is now
impl crate::Companion {
    pub fn daily_data(&mut self) -> DailyData {
        let unix = crate::unix_now();
        let day = (unix.floor() as i64 + crate::local_offset_pub()).div_euclid(86400);
        let l = self.life_now();
        let mut life = serde_json::Map::new();
        for k in ["journal", "cards", "badges", "btier", "badge_pins", "facts", "notes", "found"] {
            life.insert(k.into(), l.get(k).cloned().unwrap_or(Value::Null));
        }
        DailyData {
            day, streak: crate::rewards::streak(), total_hours: crate::rewards::total_hours(),
            mood_word: self.mood_word().to_string(), fox_name: self.fox_name(), spirit: self.spirit(), hunger: self.hunger_now(),
            life: Value::Object(life), cfg: self.cfg.raw.clone(), well: Value::Object(Default::default()), jp: "Yu Gothic UI".into(),
            due: self.cards_due().len(),
        }
    }
}
