//! The hub's Upcoming page: zenko.py hub_build_deadlines / dl_render (the add box), up_row (exam countdown, the
//! Japanese clock where you mark free half hours, the study plan; Zenko's two-row layout on a narrow window),
//! dl_calendar, dl_list and Zenko14's IA / EE / TOK tracker (ib_card), with Zenko21's half-hour ring and quick add
//! and Zenko23's ema lines under the plan.

use crate::ema::fmt_hm;
use crate::hubwin::{canvas_of, hub_title_draw};
use crate::pagekit::{Faces, Scroller, WrapState, glow_bar, hub_card, hub_scroller, label_w, tt_btn, tt_entry, zk_fonts, zk_wrap};
use crate::session::clip;
use crate::theme::Theme;
use crate::tkcanvas::{Canvas, Photo};
use crate::tt::{tt_done, tt_today};
use crate::ui::{Anchor, Fill, Font, Fonts, Grid, Kind, Pack, Side, Ui};
use serde_json::Value;
use std::rc::Rc;
use std::sync::OnceLock;

macro_rules! rx {
    ($p:expr) => {{
        static R: OnceLock<regex::Regex> = OnceLock::new();
        R.get_or_init(|| regex::Regex::new($p).unwrap())
    }};
}
use zenko_art::palette::mixhex;

pub const TITLE: &str = "Upcoming";
pub const SUB: &str = "Exams, deadlines and your study plan. Mark your free hours on the clock and I'll plan study sessions. \
                       Drop a file (exam schedule, syllabus) anywhere to add dates.";
const SHU: &str = "#c8372d";
const WD: [&str; 7] = ["Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday", "Sunday"];
const MON: [&str; 12] = ["January", "February", "March", "April", "May", "June", "July", "August", "September", "October",
                         "November", "December"];
const KANJI_NUM: [&str; 12] = ["十二", "一", "二", "三", "四", "五", "六", "七", "八", "九", "十", "十一"];
const STAGES: [&str; 6] = ["Topic", "Research", "Outline", "Draft 1", "Feedback", "Final"];
pub const DEFAULT_STATUS: &str = "e.g.  Renal block exam 12 Dec 9am   \u{b7}   Files: PDF, Word, Excel, PowerPoint, text, .ics \
                                  calendar, photos";

/// an exam the planner works towards (plan_targets with its hours)
#[derive(Clone, Debug, PartialEq)]
pub struct UpTarget {
    pub id: String,
    pub title: String,
    /// naive local seconds
    pub due: f64,
    pub hours: f64,
}

/// What the page shows, from the app (Companion::up_data)
#[derive(Clone, Debug, Default, PartialEq)]
pub struct UpData {
    /// today's local day number and the local wall clock now (seconds)
    pub today: i64,
    pub now_l: f64,
    pub targets: Vec<UpTarget>,
    /// life: studied, free, free_dates, plan_short, ema_short, ibtasks
    pub life: Value,
    pub tt: Vec<Value>,
    pub deadlines: Vec<Value>,
    pub planner: bool,
    pub session: bool,
    /// the emas by id: their text (for Zenko23's "didn't fit" lines)
    pub emas: Vec<(String, String)>,
    pub exam_days: i64,
    pub exam_day: i64,
    /// _jp_font, _serif_font, _mincho_font
    pub jp: String,
    pub serif: String,
    pub mincho: String,
    /// SUBJECT_DEFS order: (sid, short name)
    pub subjects: Vec<(String, String)>,
}

pub struct UpPage {
    pub pg: usize,
    pub title: usize,
    pub sc: Scroller,
    pub body: usize,
    pub entry: usize,
    pub status: usize,
    pub status_text: String,
    /// up_day (0 = Monday), up_pm; None until the first render (up_row's hasattr)
    pub day: Option<usize>,
    pub pm: bool,
    /// dl_month (year, month), dl_day
    pub month: (i64, i64),
    pub dl_day: Option<i64>,
    /// the clock canvas and its size S, the info label under it
    pub clock: usize,
    pub s: i32,
    pub info: usize,
    /// the quick add's two time menus
    pub m1: usize,
    pub m2: usize,
    /// the calendar canvas, the countdown and IA bars (canvas, frac), what the ✕ / stage labels and entries do
    pub cal: usize,
    pub bars: Vec<(usize, f64, f64)>,
    pub clicks: Vec<(usize, String)>,
    pub ib_entries: Vec<(usize, usize, usize)>,
    /// the countdown / plan / list buttons' targets (index -> what)
    pub cmds: Vec<String>,
    /// the layout the middle row was made for: (wide main area, S)
    pub made: (bool, i32),
    pub wrap: WrapState,
    drawn: Vec<i32>,
    seig: Option<(String, Rc<Photo>)>,
    fox: Option<Rc<Photo>>,
    /// the clock's painting drag: (marking free?, painting)
    pub paint: Option<bool>,
}

fn lbl(ui: &mut Ui, parent: usize, text: &str, bg: &str, fg: &str, font: Font) -> usize {
    ui.label(parent, text, bg, fg, font)
}

fn wl(ui: &mut Ui, l: usize, w: i32, left: bool) {
    if let Kind::Label { wrap, left: lf, .. } = &mut ui.ws[l].kind {
        (*wrap, *lf) = (w, left);
    }
}

fn set_text(ui: &mut Ui, id: usize, s: &str) {
    match &mut ui.ws[id].kind {
        Kind::Label { text, .. } | Kind::Menu { text, .. } => *text = s.into(),
        _ => {}
    }
}

fn truthy(v: &Value) -> bool {
    crate::plan::truthy(v)
}

fn num(v: &Value) -> f64 {
    v.as_f64().or_else(|| v.as_str().and_then(|s| s.parse().ok())).unwrap_or(0.0)
}

/// "%a %d %b"
fn a_d_b(day: i64) -> String {
    let (_, m, d) = crate::civil_from_days_pub(day);
    format!("{} {d:02} {}", &WD[crate::lifechat::weekday(day)][..3], &MON[m as usize - 1][..3])
}

/// fmt_num for an int: 1,850
fn fmt_num(v: i64) -> String {
    crate::routes17::fmt_int(v)
}

/// slot_hm(v): 16.5 -> "16:30"
pub fn slot_hm(v: f64) -> String {
    let v = if v < 24.0 { v.rem_euclid(24.0) } else { 24.0 };
    format!("{:02}:{:02}", v as i64, if v - v.trunc() >= 0.5 { 30 } else { 0 })
}

/// slots_text([16, 16.5, 17, 20.5]) -> "16:00-17:30, 20:30-21:00"
pub fn slots_text(slots: &[f64]) -> String {
    let mut ss: Vec<f64> = slots.iter().map(|s| (s * 2.0).round_ties_even() / 2.0).collect();
    ss.sort_by(|a, b| a.partial_cmp(b).unwrap());
    ss.dedup();
    if ss.is_empty() {
        return "none".into();
    }
    let mut runs = vec![];
    let (mut a, mut prev) = (ss[0], ss[0]);
    for s in ss[1..].iter().map(|s| Some(*s)).chain([None]) {
        if let Some(s) = s
            && (s - (prev + 0.5)).abs() < 1e-6 {
            prev = s;
            continue;
        }
        runs.push(format!("{}-{}", slot_hm(a), slot_hm(prev + 0.5)));
        if let Some(s) = s {
            a = s;
            prev = s;
        }
    }
    runs.join(", ")
}

/// slots_minutes(slots)
pub fn slots_minutes(slots: &[f64]) -> i64 {
    let mut k: Vec<i64> = slots.iter().map(|s| (s * 2.0).round_ties_even() as i64).collect();
    k.sort();
    k.dedup();
    k.len() as i64 * 30
}

/// the 49 times of the quick add's menus
pub fn quick_times() -> Vec<String> {
    let mut t: Vec<String> = (0..=48).map(|k| format!("{:02}:{:02}", k * 30 / 60, k * 30 % 60)).collect();
    t[48] = "24:00".into();
    t
}

impl UpPage {
    /// hub_build_deadlines(pg) and the first dl_render(); `wide` is hub main's width >= 1000, `s` the clock's size
    #[allow(clippy::too_many_arguments)]
    pub fn build(ui: &mut Ui, pg: usize, th: &Theme, f: &Faces, cm10: i32, d: &UpData, wide: bool, s: i32) -> UpPage {
        let title = ui.canvas(pg, &th.banner, cm10, 96);
        canvas_of(ui, title);
        ui.pack(title, Pack::default().fill(Fill::X));
        let sc = hub_scroller(ui, pg, th, cm10);
        let body = ui.frame(sc.inner, &th.paper);
        ui.pack(body, Pack::default().fill(Fill::X).padx(24, 24).pady(14, 14));
        let (y, m, _) = crate::civil_from_days_pub(d.today);
        let mut p = UpPage { pg, title, sc, body, entry: 0, status: 0, status_text: DEFAULT_STATUS.into(), day: None, pm: false,
                             month: (y as i64, m as i64), dl_day: None, clock: 0, s, info: 0, m1: 0, m2: 0, cal: 0,
                             bars: vec![], clicks: vec![], ib_entries: vec![], cmds: vec![], made: (wide, s),
                             wrap: WrapState::default(), drawn: vec![], seig: None, fox: None, paint: None };
        p.render(ui, th, f, cm10, d, wide, s);
        p
    }

    fn cmd(&mut self, what: String) -> String {
        self.cmds.push(what);
        format!("up:{}", self.cmds.len() - 1)
    }

    /// up_date(): the next date on the clock's weekday (today if it's today)
    pub fn up_date(&self, d: &UpData) -> i64 {
        let wd = crate::lifechat::weekday(d.today) as i64;
        d.today + (self.day.unwrap_or(0) as i64 - wd).rem_euclid(7)
    }

    /// up_hours(): the free half hours that day
    pub fn up_hours(&self, d: &UpData) -> Vec<f64> {
        let mut v: Vec<f64> = crate::Companion::free_for(&d.life, self.up_date(d)).iter()
            .map(|h| (h * 2.0).round_ties_even() / 2.0).collect();
        v.dedup();
        v
    }

    /// dl_render: everything in the body again
    #[allow(clippy::too_many_arguments)]
    pub fn render(&mut self, ui: &mut Ui, th: &Theme, f: &Faces, cm10: i32, d: &UpData, wide: bool, s: i32) {
        for k in ui.ws[self.body].kids.clone() {
            ui.destroy(k);
        }
        self.wrap = WrapState { lastw: self.wrap.lastw, ..WrapState::default() };
        self.drawn.clear();
        self.bars.clear();
        self.clicks.clear();
        self.ib_entries.clear();
        self.cmds.clear();
        self.made = (wide, s);
        let body = self.body;
        let bg = th.card.clone();
        let top = hub_card(ui, body, None, "round", th, f);
        ui.pack(top, Pack::default().fill(Fill::X));
        let r = ui.frame(top, &bg);
        ui.pack(r, Pack::default().fill(Fill::X).padx(16, 16).pady(12, 12));
        let b = tt_btn(ui, r, "Add from a file...", "up_file", Some(&th.acc), Some(&th.onacc), 10, 14, 3, th, f);
        ui.pack(b, Pack::default().side(Side::Left));
        let l = lbl(ui, r, "   or type an exam:", &bg, &th.grey, Font::new(&f.ui, 10, false));
        ui.pack(l, Pack::default().side(Side::Left));
        let e = tt_entry(ui, r, "", 38, th, f);
        ui.pack(e, Pack::default().side(Side::Left).padx(8, 8).ipady(2));
        self.entry = e;
        let b = tt_btn(ui, r, "Add", "up_add", Some(&th.start), None, 10, 12, 2, th, f);
        ui.pack(b, Pack::default().side(Side::Left));
        let st = label_w(ui, top, &self.status_text.clone(), &bg, &th.grey, &f.ui, 9, false);
        wl(ui, st, 900, true);
        ui.pack(st, Pack::default().fill(Fill::X).padx(16, 16).pady(0, 10));
        self.status = st;
        self.up_row(ui, th, f, cm10, d, wide, s);
        let row = ui.frame(body, &th.paper);
        ui.pack(row, Pack::default().fill(Fill::X).pady(12, 0));
        let cal = hub_card(ui, row, None, "round", th, f);
        ui.pack(cal, Pack::default().side(Side::Left).fill(Fill::Y));
        self.calendar(ui, th, f, d, cal);
        let lt = match self.dl_day {
            None => "Coming up".to_string(),
            Some(dd) => {
                let (_, m, day) = crate::civil_from_days_pub(dd);
                format!("On {} {day:02} {}", WD[crate::lifechat::weekday(dd)], MON[m as usize - 1])
            }
        };
        let lst = hub_card(ui, row, Some(&lt), "round", th, f);
        ui.pack(lst, Pack::default().side(Side::Left).fill(Fill::Both).expand().padx(12, 0));
        self.list(ui, th, f, d, lst);
        self.ib_card(ui, th, f, d);
    }

    /// up_row: three columns on a wide main area (Zenko: two rows when it's under 1000 pixels)
    #[allow(clippy::too_many_arguments)]
    fn up_row(&mut self, ui: &mut Ui, th: &Theme, f: &Faces, cm10: i32, d: &UpData, wide: bool, s: i32) {
        if self.day.is_none() {
            self.day = Some(crate::lifechat::weekday(d.today));
            let hour = ((d.now_l.rem_euclid(86400.0)) / 3600.0) as i64;
            self.pm = hour >= 12 || !crate::Companion::free_for(&d.life, d.today).iter().any(|h| (*h as i64) < 12);
        }
        let row = ui.frame(self.body, &th.paper);
        ui.pack(row, Pack::default().fill(Fill::X).pady(12, 0));
        let g = |row: usize, col: usize, padx: (i32, i32), pady: (i32, i32), cspan: usize| Grid {
            row, col, sticky: "nsew", padx, pady, cspan, rspan: 1 };
        let (left, mid, right);
        if wide {
            (ui.ws[row].gw_cols, ui.ws[row].gu_cols) = (vec![1, 0, 1], vec!["up", "", "up"]);
            left = hub_card(ui, row, Some("Exam countdown"), "ema", th, f);
            ui.grid(left, g(0, 0, (0, 0), (0, 0), 1));
            mid = hub_card(ui, row, Some("Your free hours"), "leaf", th, f);
            ui.grid(mid, g(0, 1, (12, 12), (0, 0), 1));
            right = hub_card(ui, row, Some("Study plan"), "round", th, f);
            ui.grid(right, g(0, 2, (0, 0), (0, 0), 1));
        } else {
            (ui.ws[row].gw_cols, ui.ws[row].gu_cols) = (vec![1, 1], vec!["up", "up"]);
            left = hub_card(ui, row, Some("Exam countdown"), "ema", th, f);
            ui.grid(left, g(0, 0, (0, 6), (0, 0), 1));
            right = hub_card(ui, row, Some("Study plan"), "round", th, f);
            ui.grid(right, g(0, 1, (6, 0), (0, 0), 1));
            mid = hub_card(ui, row, Some("Your free hours"), "leaf", th, f);
            ui.grid(mid, g(1, 0, (0, 0), (12, 0), 2));
        }
        self.countdown(ui, th, f, d, left);
        self.clock_card(ui, th, f, cm10, d, mid, s);
        self.plan(ui, th, f, d, right);
    }

    /// up_countdown: an ema plaque with the days left, the title, studied so far, a bar and - / + for the hours
    fn countdown(&mut self, ui: &mut Ui, th: &Theme, f: &Faces, d: &UpData, c: usize) {
        let bg = th.card.clone();
        if d.targets.is_empty() {
            let l = label_w(ui, c, "No exams coming up~\n\nTell me in chat, like:\n\u{201c}my chemistry exam is in 2 days\u{201d}\n\
                                    \u{201c}maths test on Friday 9am\u{201d}\n\nI'll plan study sessions in your free hours.",
                            &bg, &th.grey, &f.ui, 10, false);
            wl(ui, l, 0, true);
            ui.pack(l, Pack::default().fill(Fill::X).padx(16, 16).pady(4, 16));
            return;
        }
        let jp = if d.jp.is_empty() { f.ui.clone() } else { d.jp.clone() };
        for t in d.targets.iter().take(4) {
            let dday = (t.due / 86400.0).floor() as i64;
            let days = dday - d.today;
            let r = ui.frame(c, &bg);
            ui.pack(r, Pack::default().fill(Fill::X).padx(12, 12).pady(4, 4));
            let cvw = ui.canvas(r, &bg, 58, 58);
            ui.pack(cvw, Pack::default().side(Side::Left));
            let cv = canvas_of(ui, cvw);
            let (wood, edge) = if days <= 2 {
                (mixhex(SHU, &bg, 0.15), mixhex(SHU, "#000000", 0.3))
            } else {
                (mixhex("#c98a35", &bg, 0.25), mixhex("#7a4a1c", &bg, 0.1))
            };
            cv.poly(vec![29.0, 2.0, 55.0, 14.0, 55.0, 56.0, 3.0, 56.0, 3.0, 14.0]).fill(&wood).outline(&edge).width(2.0);
            cv.line(vec![20.0, 9.0, 38.0, 9.0]).fill(&edge).width(2.0);
            let big = if days == 0 { "\u{4eca}\u{65e5}".to_string() } else { days.to_string() };
            cv.text(29.0, 31.0, &big).fill("#fff6e0").font(Font::new(&jp, if days != 0 { 15 } else { 12 }, true));
            let small = if days == 0 { "" } else if days == 1 { "day" } else { "days" };
            cv.text(29.0, 48.0, small).fill("#fff6e0").font(Font::new(&f.ui, 7, true));
            let mid = ui.frame(r, &bg);
            ui.pack(mid, Pack::default().side(Side::Left).fill(Fill::X).expand().padx(8, 0));
            let l = label_w(ui, mid, &clip(&t.title, 24), &bg, &th.ink, &f.ui, 10, true);
            ui.pack(l, Pack::default().fill(Fill::X));
            let done = num(&d.life["studied"][&t.id]) as i64;
            let ds = crate::plan::iso(d.today);
            let todays: i64 = d.tt.iter().filter(|i| i["plan"].as_str() == Some(t.id.as_str()) && i["date"].as_str() == Some(&ds))
                .map(|i| num(&i["mins"]) as i64).sum();
            let txt = format!("{} \u{b7} {} of {} studied{}", a_d_b(dday), if done != 0 { fmt_hm(done) } else { "0m".into() },
                              fmt_hm((t.hours * 60.0) as i64),
                              if todays != 0 { format!("\nToday's goal: {}", fmt_hm(todays)) } else { String::new() });
            let l = label_w(ui, mid, &txt, &bg, &th.grey, &f.ui, 8, false);
            wl(ui, l, 0, true);
            ui.pack(l, Pack::default().fill(Fill::X));
            let bar = ui.canvas(mid, &bg, 60, 10);
            canvas_of(ui, bar);
            ui.pack(bar, Pack::default().fill(Fill::X).pady(2, 0));
            let frac = (done as f64 / (t.hours * 60.0).max(1.0)).min(1.0);
            self.bars.push((bar, frac, 9.0));
            let hr = ui.frame(mid, &bg);
            ui.pack(hr, Pack::default().anchor(Anchor::W).pady(2, 0));
            let c1 = self.cmd(format!("bump:{}:{}:-1", t.id, t.hours));
            let b = tt_btn(ui, hr, "\u{2212}", &c1, Some(&th.field), None, 8, 6, 0, th, f);
            ui.pack(b, Pack::default().side(Side::Left));
            let l = lbl(ui, hr, &format!(" {} total ", fmt_hm((t.hours * 60.0) as i64)), &bg, &th.ink, Font::new(&f.ui, 8, false));
            ui.pack(l, Pack::default().side(Side::Left));
            let c2 = self.cmd(format!("bump:{}:{}:1", t.id, t.hours));
            let b = tt_btn(ui, hr, "+", &c2, Some(&th.field), None, 8, 6, 0, th, f);
            ui.pack(b, Pack::default().side(Side::Left));
        }
        let sp = ui.frame(c, &bg);
        ui.ws[sp].opt_h = 8;
        ui.pack(sp, Pack::default());
    }

    /// up_plan (with Zenko23's ema lines after it)
    fn plan(&mut self, ui: &mut Ui, th: &Theme, f: &Faces, d: &UpData, c: usize) {
        let bg = th.card.clone();
        let ds = crate::plan::iso(d.today);
        let mut rows: Vec<&Value> = d.tt.iter().filter(|i| truthy(&i["plan"]) && i["date"].as_str().unwrap_or("") >= ds.as_str())
            .collect();
        rows.sort_by(|a, b| (a["date"].as_str(), a["start"].as_str()).cmp(&(b["date"].as_str(), b["start"].as_str())));
        let p = Pack::default().fill(Fill::X).padx(16, 16);
        if !d.planner {
            let l = label_w(ui, c, "The auto planner is off (Settings > Study planner).", &bg, &th.grey, &f.ui, 10, false);
            wl(ui, l, 260, true);
            ui.pack(l, p.clone().pady(4, 10));
        } else if rows.is_empty() {
            let l = label_w(ui, c, "Nothing planned yet. Add an exam and mark your free hours on the clock, and I'll fill this in~",
                            &bg, &th.grey, &f.ui, 10, false);
            wl(ui, l, 260, true);
            ui.pack(l, p.clone().pady(4, 10));
        }
        let mut last = None;
        let mut shown = 0;
        for it in &rows {
            if shown >= 9 {
                break;
            }
            let Some(dd) = crate::plan::parse_iso(it["date"].as_str().unwrap_or("")) else { continue };
            if last != Some(dd) {
                last = Some(dd);
                let lab = if dd == d.today {
                    "Today".to_string()
                } else if dd - d.today == 1 {
                    "Tomorrow".into()
                } else {
                    let (_, m, day) = crate::civil_from_days_pub(dd);
                    format!("{} {day:02} {}", WD[crate::lifechat::weekday(dd)], &MON[m as usize - 1][..3])
                };
                let l = label_w(ui, c, &lab, &bg, &th.gold, &f.ui, 9, true);
                ui.pack(l, p.clone().pady(6, 0));
            }
            let done = tt_done(it, &crate::plan::iso(dd));
            let r = ui.frame(c, &bg);
            ui.pack(r, p.clone().pady(1, 1));
            let l = label_w(ui, r, it["start"].as_str().unwrap_or(""), &bg, if done { &th.grey } else { &th.accd }, "Consolas", 10, true);
            ui.ws[l].opt_w = 6;
            ui.pack(l, Pack::default().side(Side::Left));
            let l = label_w(ui, r, &clip(it["task"].as_str().unwrap_or(""), 22), &bg, if done { &th.grey } else { &th.ink }, &f.ui, 9, false);
            if let Kind::Label { strike, .. } = &mut ui.ws[l].kind {
                *strike = done;
            }
            ui.pack(l, Pack::default().side(Side::Left));
            if dd == d.today && !done && !d.session {
                let k = d.tt.iter().position(|x| std::ptr::eq(x, *it)).unwrap_or(0);
                let cm = self.cmd(format!("start:{k}"));
                let b = tt_btn(ui, r, "Start", &cm, Some(&th.start), None, 8, 6, 0, th, f);
                ui.pack(b, Pack::default().side(Side::Right));
            }
            shown += 1;
        }
        if rows.len() > shown {
            let l = label_w(ui, c, &format!("+ {} more on your timetable", rows.len() - shown), &bg, &th.grey, &f.ui, 8, false);
            ui.pack(l, p.clone());
        }
        if let Some(short) = d.life["plan_short"].as_object() {
            for (tid, m) in short.iter().filter(|(k, _)| !k.starts_with("ema:")) {
                let title = d.deadlines.iter().find(|x| x["id"].as_str() == Some(tid.as_str()))
                    .and_then(|x| x["title"].as_str()).unwrap_or("an exam").to_string();
                let l = label_w(ui, c, &format!("Uu~ {} more for {} didn't fit. Mark more free hours?", fmt_hm(num(m) as i64),
                                                clip(&title, 20)), &bg, &th.bad, &f.ui, 8, false);
                wl(ui, l, 260, true);
                ui.pack(l, p.clone().pady(4, 0));
            }
        }
        let b = ui.frame(c, &bg);
        ui.pack(b, p.clone().pady(8, 12));
        let bt = tt_btn(ui, b, "Re-plan now", "up_replan", Some(&th.acc), Some(&th.onacc), 9, 10, 1, th, f);
        ui.pack(bt, Pack::default().side(Side::Left));
        if let Some(short) = d.life["ema_short"].as_object() {
            for (pid, m) in short.iter().take(3) {
                let id = pid.get(4..).unwrap_or("");
                if let Some((_, text)) = d.emas.iter().find(|(i, _)| i == id) {
                    let l = label_w(ui, c, &format!("Uu~ {} for the ema \u{201c}{}\u{201d} didn't fit before it's due.",
                                                    fmt_hm(num(m) as i64), clip(text, 22)), &bg, &th.bad, &f.ui, 8, false);
                    wl(ui, l, 260, true);
                    ui.pack(l, p.clone().pady(2, 0));
                }
            }
        }
    }

    /// up_clock (and Zenko21's quick add under it)
    #[allow(clippy::too_many_arguments)]
    fn clock_card(&mut self, ui: &mut Ui, th: &Theme, f: &Faces, cm10: i32, d: &UpData, c: usize, s: i32) {
        let bg = th.card.clone();
        let chips = ui.frame(c, &bg);
        ui.pack(chips, Pack::default().pady(0, 4));
        let day = self.day.unwrap_or(0);
        for (i, nm) in ["Mo", "Tu", "We", "Th", "Fr", "Sa", "Su"].iter().enumerate() {
            let on = i == day;
            let b = tt_btn(ui, chips, nm, &format!("up_day:{i}"), Some(if on { &th.acc } else { &th.field }),
                           Some(if on { &th.onacc } else { &th.ink }), 8, 5, 0, th, f);
            ui.pack(b, Pack::default().side(Side::Left).padx(1, 1));
        }
        let cv = ui.canvas(c, &bg, s, s);
        canvas_of(ui, cv);
        ui.pack(cv, Pack::default().padx(10, 10));
        (self.clock, self.s) = (cv, s);
        let info = lbl(ui, c, "", &bg, &th.grey, Font::new(&f.ui, 9, false));
        wl(ui, info, s + 20, false);
        ui.pack(info, Pack::default().pady(2, 0));
        self.info = info;
        let bb = ui.frame(c, &bg);
        ui.pack(bb, Pack::default().pady(4, 10));
        let ampm = if self.pm { "\u{5348}\u{524d} AM" } else { "\u{5348}\u{5f8c} PM" };
        for (t, cmd, col) in [(ampm, "up_flip", th.start.clone()), ("Copy to weekdays", "up_copy", th.field.clone()),
                              ("Clear", "up_clear", th.delbtn.clone())] {
            let b = tt_btn(ui, bb, t, cmd, Some(&col), None, 9, 8, 1, th, f);
            ui.pack(b, Pack::default().side(Side::Left).padx(2, 2));
        }
        self.draw_clock(ui, th, f, d);
        self.info_default(ui, d);
        // z21_quick_add
        let row = ui.frame(c, &bg);
        ui.pack(row, Pack::default().pady(0, 4));
        let times = quick_times();
        let hour = (d.now_l.rem_euclid(86400.0) / 3600.0) as i64;
        let start0 = if day == crate::lifechat::weekday(d.today) { format!("{:02}:00", 23.min(hour + 1)) } else { "16:00".into() };
        let i0 = times.iter().position(|t| *t == start0).unwrap_or(32);
        let end0 = times[(i0 + 2).min(times.len() - 1)].clone();
        let l = lbl(ui, row, "Free from", &bg, &th.grey, Font::new(&f.ui, 9, false));
        ui.pack(l, Pack::default().side(Side::Left));
        let m1 = ui.menubutton(row, &start0, "up_menu:1", Font::new(&f.ui, 9, false), &th.field, &th.ink, &th.line, 5, cm10);
        ui.pack(m1, Pack::default().side(Side::Left).padx(4, 4));
        let l = lbl(ui, row, "to", &bg, &th.grey, Font::new(&f.ui, 9, false));
        ui.pack(l, Pack::default().side(Side::Left));
        let m2 = ui.menubutton(row, &end0, "up_menu:2", Font::new(&f.ui, 9, false), &th.field, &th.ink, &th.line, 5, cm10);
        ui.pack(m2, Pack::default().side(Side::Left).padx(4, 0));
        (self.m1, self.m2) = (m1, m2);
        let row2 = ui.frame(c, &bg);
        ui.pack(row2, Pack::default().pady(0, 10));
        let b = tt_btn(ui, row2, "Add as free time", "up_quick:1", Some(&th.acc), Some(&th.onacc), 9, 10, 1, th, f);
        ui.pack(b, Pack::default().side(Side::Left));
        let b = tt_btn(ui, row2, "Remove", "up_quick:0", Some(&th.field), None, 9, 8, 1, th, f);
        ui.pack(b, Pack::default().side(Side::Left).padx(4, 0));
    }

    /// up_info_default (Zenko21: half hours and their total)
    pub fn info_default(&self, ui: &mut Ui, d: &UpData) {
        let hrs = self.up_hours(d);
        let mins = slots_minutes(&hrs);
        let t = format!("Free on {}s: {}{}\nTap half an hour to mark it, or drag.", WD[self.day.unwrap_or(0)],
                        if hrs.is_empty() { "none yet".to_string() } else { slots_text(&hrs) },
                        if mins != 0 { format!("  ({})", fmt_hm(mins)) } else { String::new() });
        set_text(ui, self.info, &t);
    }

    fn geom(&self) -> (f64, f64, f64) {
        let s = self.s as f64;
        (s / 2.0, s / 2.0, s / 2.0 - 6.0)
    }

    /// up_hit (Zenko21): the half hour under (x, y) on the ring, in canvas coordinates
    pub fn hit(&self, x: f64, y: f64) -> Option<f64> {
        let (cx, cy, r) = self.geom();
        let dd = (x - cx).hypot(y - cy);
        if !(r - 36.0 <= dd && dd <= r + 2.0) {
            return None;
        }
        let a = ((x - cx).atan2(cy - y).to_degrees() + 360.0).rem_euclid(360.0);
        Some(23.5f64.min((a / 15.0).floor() / 2.0 + if self.pm { 12.0 } else { 0.0 }))
    }

    /// up_hover: what that half hour is
    pub fn hover(&self, ui: &mut Ui, d: &UpData, x: f64, y: f64) {
        let Some(h) = self.hit(x, y) else { return self.info_default(ui, d) };
        let day = self.up_date(d);
        let a = (h * 60.0) as i64;
        let ds = crate::plan::iso(day);
        let mut what = vec![];
        for i in tt_today(&d.tt, &ds) {
            let it = &d.tt[i];
            if let Some((hh, mm)) = it["start"].as_str().and_then(|s| s.split_once(':'))
                && let (Ok(hh), Ok(mm)) = (hh.parse::<i64>(), mm.parse::<i64>()) {
                let s0 = hh * 60 + mm;
                if s0 < a + 30 && a < s0 + num(&it["mins"]) as i64 {
                    what.push(it["task"].as_str().unwrap_or("").to_string());
                }
            }
        }
        let on = self.up_hours(d).contains(&h);
        let t = format!("{}  {}-{}  \u{b7}  {}{}\nClick to {}.", &WD[crate::lifechat::weekday(day)][..3], slot_hm(h), slot_hm(h + 0.5),
                        if on { "free for study" } else { "not free" },
                        if what.is_empty() { String::new() } else { format!("  \u{b7}  {}", clip(&what.join(", "), 40)) },
                        if on { "unmark" } else { "mark it free" });
        set_text(ui, self.info, &t);
    }

    /// up_draw (Zenko21's 24 half-hour pieces over the 12) and up_hands
    pub fn draw_clock(&mut self, ui: &mut Ui, th: &Theme, f: &Faces, d: &UpData) {
        let mut cv = std::mem::take(canvas_of(ui, self.clock));
        cv.delete("face");
        let (cx, cy, r) = self.geom();
        let bg = th.card.clone();
        let jp = d.jp.clone();
        let free = self.up_hours(d);
        let day = self.up_date(d);
        let base = if self.pm { 12.0 } else { 0.0 };
        cv.oval(cx - r, cy - r, cx + r, cy + r).fill(&mixhex(SHU, &th.night, 0.45)).outline(&th.gold).width(2.0).tags(&["face"]);
        let rm = r - 21.0;
        for k in 0..12 {
            let h = k as f64 + base;
            let on = free.contains(&h);
            let col = if on { mixhex(&bg, &th.gold, 0.62) } else { mixhex(&bg, "#000000", 0.28) };
            cv.arc(cx - rm, cy - rm, cx + rm, cy + rm, 90.0 - (k + 1) as f64 * 30.0 + 1.2, 27.6, "arc").outline(&col).width(24.0)
                .tags(&["face", "seg"]);
            let a = ((k * 30 + 15) as f64).to_radians();
            cv.text(cx + rm * a.sin(), cy - rm * a.cos(), &format!("{:02}", h as i64)).fill(if on { &th.onacc } else { &th.grey })
                .font(Font::new(&f.ui, 8, true)).tags(&["face"]);
        }
        let rb = r - 40.0;
        cv.oval(cx - rb, cy - rb, cx + rb, cy + rb).outline(&mixhex(&bg, "#000000", 0.35)).width(6.0).tags(&["face"]);
        let ds = crate::plan::iso(day);
        for i in tt_today(&d.tt, &ds) {
            let it = &d.tt[i];
            let Some((h0, m0)) = it["start"].as_str().and_then(|s| s.split_once(':')) else { continue };
            let a0 = h0.parse::<i64>().unwrap_or(0) * 60 + m0.parse::<i64>().unwrap_or(0);
            let a1 = a0 + num(&it["mins"]) as i64;
            let b = if self.pm { 720 } else { 0 };
            let (s0, s1) = (a0.max(b), a1.min(b + 720));
            if s1 <= s0 {
                continue;
            }
            let st = (s0 - b) as f64 / 2.0;
            let ext = (s1 - s0) as f64 / 2.0;
            let col = if truthy(&it["plan"]) { th.acc.clone() } else { mixhex(&bg, &th.ink, 0.45) };
            cv.arc(cx - rb, cy - rb, cx + rb, cy + rb, 90.0 - st - ext, 1.0f64.max(ext - 0.6), "arc").outline(&col).width(6.0)
                .tags(&["face"]);
        }
        let rf = r - 45.0;
        for i in (1..=10).rev() {
            let r_ = rf * i as f64 / 10.0;
            let c = mixhex(&mixhex(&bg, &th.night, 0.45), &mixhex(&bg, &th.ink, 0.06), 1.0 - i as f64 / 10.0);
            cv.oval(cx - r_, cy - r_, cx + r_, cy + r_).fill(&c).outline("").tags(&["face"]);
        }
        cv.oval(cx - rf, cy - rf, cx + rf, cy + rf).outline(&th.gold).width(1.0).tags(&["face"]);
        for mnt in 0..60 {
            let a = ((mnt * 6) as f64).to_radians();
            let r1 = rf - if mnt % 5 == 0 { 9.0 } else { 5.0 };
            let col = if mnt % 5 == 0 { th.gold.clone() } else { mixhex(&bg, &th.ink, 0.35) };
            cv.line(vec![cx + (rf - 3.0) * a.sin(), cy - (rf - 3.0) * a.cos(), cx + r1 * a.sin(), cy - r1 * a.cos()]).fill(&col)
                .width(if mnt % 5 == 0 { 2.0 } else { 1.0 }).tags(&["face"]);
        }
        let rn = rf - 24.0;
        let jf = if jp.is_empty() { f.ui.clone() } else { jp.clone() };
        for k in 0..12 {
            let a = ((k * 30) as f64).to_radians();
            let t = if jp.is_empty() { (if k == 0 { 12 } else { k }).to_string() } else { KANJI_NUM[k as usize].to_string() };
            cv.text(cx + rn * a.sin(), cy - rn * a.cos(), &t).fill(&th.ink).font(Font::new(&jf, if jp.is_empty() { 10 } else { 11 }, true))
                .tags(&["face"]);
        }
        let wd = crate::lifechat::weekday(day);
        let wk = ["月", "火", "水", "木", "金", "土", "日"][wd];
        let ap = if jp.is_empty() { if self.pm { "PM" } else { "AM" } } else if self.pm { "\u{5348}\u{5f8c}" } else { "\u{5348}\u{524d}" };
        cv.text(cx, cy - rf * 0.42, ap).fill(&th.gold).font(Font::new(&jf, 12, true)).tags(&["face"]);
        let (_, mo, dm) = crate::civil_from_days_pub(day);
        let dt = if jp.is_empty() { a_d_b(day) } else { format!("{mo}月{dm}日 ({wk})") };
        cv.text(cx, cy - rf * 0.24, &dt).fill(&th.grey).font(Font::new(&jf, 8, false)).tags(&["face"]);
        if let Some(fx) = &self.fox {
            cv.image(cx, cy + rf * 0.12, fx.clone()).anchor("n").tags(&["face"]);
        }
        cv.tag_lower("face", None);
        Self::hands(&mut cv, th, cx, cy, r, self.up_date(d) == d.today && ((d.now_l.rem_euclid(86400.0) / 3600.0) as i64 >= 12) == self.pm,
                    d.now_l);
        // Zenko21: the 24 half-hour pieces and the hour numbers back on top
        cv.delete("seg");
        for k in 0..24 {
            let s = k as f64 / 2.0 + base;
            let on = free.contains(&s);
            let col = if on { mixhex(&bg, &th.gold, 0.62) } else { mixhex(&bg, "#000000", 0.28) };
            let gap = if k % 2 == 0 { 1.2 } else { 0.5 };
            cv.arc(cx - rm, cy - rm, cx + rm, cy + rm, 90.0 - (k + 1) as f64 * 15.0 + 0.5, 15.0 - 0.5 - gap, "arc").outline(&col)
                .width(24.0).tags(&["face", "seg"]);
        }
        for k in 0..12 {
            let h = k as f64 + base;
            let (a_, b_) = (free.contains(&h), free.contains(&(h + 0.5)));
            let a = ((k * 30 + 15) as f64).to_radians();
            let col = if a_ && b_ { &th.onacc } else if a_ || b_ { &th.ink } else { &th.grey };
            cv.text(cx + rm * a.sin(), cy - rm * a.cos(), &format!("{:02}", h as i64)).fill(col).font(Font::new(&f.ui, 8, true))
                .tags(&["face", "seglab"]);
        }
        cv.tag_raise("seg");
        cv.tag_raise("seglab");
        cv.tag_raise("hands");
        *canvas_of(ui, self.clock) = cv;
    }

    /// up_hands for the clock now (up_tick redraws them every second)
    pub fn tick(&mut self, ui: &mut Ui, th: &Theme, d: &UpData) {
        if self.clock == 0 || ui.ws.get(self.clock).is_none() {
            return;
        }
        let (cx, cy, r) = self.geom();
        let cv = canvas_of(ui, self.clock);
        cv.delete("hands");
        let marker = self.up_date(d) == d.today && ((d.now_l.rem_euclid(86400.0) / 3600.0) as i64 >= 12) == self.pm;
        Self::hands(cv, th, cx, cy, r, marker, d.now_l);
    }

    #[allow(clippy::too_many_arguments)]
    fn hands(cv: &mut Canvas, th: &Theme, cx: f64, cy: f64, r: f64, marker: bool, now_l: f64) {
        let rf = r - 45.0;
        let t = now_l.rem_euclid(86400.0);
        let (hour, minute) = ((t / 3600.0) as i64, ((t / 60.0) as i64) % 60);
        let sec = t - (t / 60.0).floor() * 60.0;
        let ah = (((hour % 12) as f64 + minute as f64 / 60.0 + sec / 3600.0) * 30.0).to_radians();
        let am = ((minute as f64 + sec / 60.0) * 6.0).to_radians();
        let asec = ((sec as i64 * 6) as f64).to_radians();
        let hand = |cv: &mut Canvas, a: f64, length: f64, w: f64, tail: f64, fill: &str, outline: &str| {
            let (sx, sy) = (a.sin(), -a.cos());
            let (px, py) = (-sy, sx);
            let pts = vec![cx - sx * tail + px * w * 0.6, cy - sy * tail + py * w * 0.6,
                           cx + px * w, cy + py * w,
                           cx + sx * length * 0.82 + px * w * 0.55, cy + sy * length * 0.82 + py * w * 0.55,
                           cx + sx * length, cy + sy * length,
                           cx + sx * length * 0.82 - px * w * 0.55, cy + sy * length * 0.82 - py * w * 0.55,
                           cx - px * w, cy - py * w,
                           cx - sx * tail - px * w * 0.6, cy - sy * tail - py * w * 0.6];
            cv.poly(pts).fill(fill).outline(outline).width(1.0).tags(&["hands"]);
        };
        if marker {
            let a = (((hour % 12) as f64 + minute as f64 / 60.0) * 30.0).to_radians();
            let rr = r - 3.0;
            let (x, y) = (cx + rr * a.sin(), cy - rr * a.cos());
            cv.oval(x - 5.0, y - 5.0, x + 5.0, y + 5.0).fill(&th.glow).outline("#ffffff").width(1.0).tags(&["hands"]);
        }
        hand(cv, ah, rf * 0.55, 4.5, 12.0, &th.gold, &mixhex(&th.gold, "#000000", 0.5));
        hand(cv, am, rf * 0.84, 3.2, 14.0, &th.ink, &mixhex(&th.ink, "#000000", 0.5));
        let (sx, sy) = (asec.sin(), -asec.cos());
        cv.line(vec![cx - sx * 20.0, cy - sy * 20.0, cx + sx * rf * 0.9, cy + sy * rf * 0.9]).fill(SHU).width(2.0).tags(&["hands"]);
        let (fx, fy) = (cx - sx * 20.0, cy - sy * 20.0);
        cv.oval(fx - 4.0, fy - 4.0, fx + 4.0, fy + 4.0).fill(&th.glow).outline(SHU).tags(&["hands"]);
        cv.oval(cx - 7.0, cy - 7.0, cx + 7.0, cy + 7.0).fill(&th.gold).outline(&mixhex(&th.gold, "#000000", 0.5)).tags(&["hands"]);
        cv.oval(cx - 2.5, cy - 2.5, cx + 2.5, cy + 2.5).fill(SHU).outline("").tags(&["hands"]);
    }

    /// dl_calendar: the month, < >, the grid of days with dots for what's due
    fn calendar(&mut self, ui: &mut Ui, th: &Theme, f: &Faces, d: &UpData, c: usize) {
        let bg = th.card.clone();
        let head = ui.frame(c, &bg);
        ui.pack(head, Pack::default().fill(Fill::X).padx(14, 14).pady(12, 6));
        let b = tt_btn(ui, head, "<", "up_month:-1", Some(&th.field), None, 10, 8, 0, th, f);
        ui.pack(b, Pack::default().side(Side::Left));
        let (y, m) = self.month;
        let l = lbl(ui, head, &format!("{} {y}", MON[m as usize - 1]), &bg, &th.ink, Font::new(&f.ui, 13, true));
        ui.ws[l].opt_w = 15;
        ui.pack(l, Pack::default().side(Side::Left).padx(6, 6));
        let b = tt_btn(ui, head, ">", "up_month:1", Some(&th.field), None, 10, 8, 0, th, f);
        ui.pack(b, Pack::default().side(Side::Left));
        let (cell, gap) = (46.0, 4.0);
        let cvw = ui.canvas(c, &bg, (7.0 * (cell + gap)) as i32, (7.0 * (cell + gap) - 10.0) as i32);
        ui.pack(cvw, Pack::default().padx(14, 14).pady(0, 12));
        self.cal = cvw;
        let cv = canvas_of(ui, cvw);
        for (i, dn) in ["Mo", "Tu", "We", "Th", "Fr", "Sa", "Su"].iter().enumerate() {
            cv.text(i as f64 * (cell + gap) + cell / 2.0, 10.0, dn).fill(&th.grey).font(Font::new(&f.ui, 9, true));
        }
        let m0 = crate::rewards::days_from_civil(y, m, 1);
        let start = m0 - crate::lifechat::weekday(m0) as i64;
        for k in 0..42 {
            let day = start + k;
            let (col, rowi) = ((k % 7) as f64, (k / 7) as f64);
            let (x, yy) = (col * (cell + gap), 24.0 + rowi * (cell + gap));
            let (_, dm, dd) = crate::civil_from_days_pub(day);
            let inmonth = dm as i64 == m;
            let iso = crate::plan::iso(day);
            let items: Vec<&Value> = d.deadlines.iter().filter(|x| !truthy(&x["done"]) && x["date"].as_str() == Some(&iso)).collect();
            let mut fill = if inmonth { th.field.clone() } else { th.card.clone() };
            if day == d.today {
                fill = th.nowbg.clone();
            }
            if self.dl_day == Some(day) {
                fill = th.start.clone();
            }
            let outline = if day == d.today { th.gold.clone() } else if !items.is_empty() { th.acc.clone() } else { th.line.clone() };
            cv.rect(x, yy, x + cell, yy + cell).fill(&fill).outline(&outline)
                .width(if !items.is_empty() || day == d.today { 2.0 } else { 1.0 });
            cv.text(x + 6.0, yy + 5.0, &dd.to_string()).anchor("nw").fill(if inmonth { &th.ink } else { &th.grey })
                .font(Font::new(&f.ui, 9, day == d.today));
            if !items.is_empty() {
                let urgent = items.iter().any(|i| matches!(i["kind"].as_str(), Some("Exam") | Some("Quiz")));
                for j in 0..items.len().min(3) {
                    let jx = j as f64 * 10.0;
                    cv.oval(x + 7.0 + jx, yy + cell - 13.0, x + 14.0 + jx, yy + cell - 6.0)
                        .fill(if urgent { &th.bad } else { &th.acc }).outline("");
                }
            }
        }
        if self.dl_day.is_some() {
            let b = tt_btn(ui, c, "Show all", "up_pick:none", Some(&th.field), None, 9, 10, 1, th, f);
            ui.pack(b, Pack::default().pady(0, 12));
        }
    }

    /// the day of the calendar cell at (x, y) in the canvas
    pub fn cal_day(&self, d: &UpData, x: f64, y: f64) -> Option<i64> {
        let _ = d;
        let (cell, gap) = (46.0, 4.0);
        let col = (x / (cell + gap)).floor();
        let row = ((y - 24.0) / (cell + gap)).floor();
        if !(0.0..7.0).contains(&col) || !(0.0..6.0).contains(&row) {
            return None;
        }
        if x - col * (cell + gap) > cell || y - 24.0 - row * (cell + gap) > cell {
            return None;
        }
        let m0 = crate::rewards::days_from_civil(self.month.0, self.month.1, 1);
        Some(m0 - crate::lifechat::weekday(m0) as i64 + (row * 7.0 + col) as i64)
    }

    /// dl_list: what's due (or the picked day's)
    fn list(&mut self, ui: &mut Ui, th: &Theme, f: &Faces, d: &UpData, c: usize) {
        let bg = th.card.clone();
        let today = crate::plan::iso(d.today);
        let show: Vec<&Value> = match self.dl_day {
            Some(dd) => {
                let k = crate::plan::iso(dd);
                d.deadlines.iter().filter(|x| x["date"].as_str() == Some(&k)).collect()
            }
            None => {
                let over: Vec<&Value> = d.deadlines.iter().filter(|x| x["date"].as_str().unwrap_or("") < today.as_str() && !truthy(&x["done"]))
                    .collect();
                let skip = over.len().saturating_sub(5);
                over.into_iter().skip(skip).chain(d.deadlines.iter()
                    .filter(|x| x["date"].as_str().unwrap_or("") >= today.as_str() && !truthy(&x["done"]))).collect()
            }
        };
        if show.is_empty() {
            let l = label_w(ui, c, "Nothing here yet. Drop your exam schedule on this window or on the fox.", &bg, &th.grey, &f.ui, 10, false);
            ui.pack(l, Pack::default().fill(Fill::X).padx(16, 16).pady(4, 16));
            return;
        }
        for x in show.iter().take(30) {
            let date = x["date"].as_str().unwrap_or("");
            let late = date < today.as_str();
            let Some(dt) = crate::plan::parse_iso(date) else { continue };
            let r = ui.frame(c, &bg);
            ui.pack(r, Pack::default().fill(Fill::X).padx(16, 16).pady(3, 3));
            let kind = x["kind"].as_str().unwrap_or("");
            let exam = matches!(kind, "Exam" | "Quiz");
            let bc = if late { th.bad.clone() } else if exam { th.acc.clone() } else { th.start.clone() };
            let badge = ui.frame(r, &bc);
            (ui.ws[badge].opt_w, ui.ws[badge].opt_h, ui.ws[badge].propagate) = (52, 44, false);
            ui.pack(badge, Pack::default().side(Side::Left));
            let fgc = if late || exam { th.onacc.clone() } else { th.ink.clone() };
            let (_, m, dd) = crate::civil_from_days_pub(dt);
            let l = lbl(ui, badge, &format!("{dd:02}"), &bc, &fgc, Font::new(&f.ui, 13, true));
            ui.pack(l, Pack::default().pady(2, 0));
            let l = lbl(ui, badge, &MON[m as usize - 1][..3].to_uppercase(), &bc, &fgc, Font::new(&f.ui, 7, true));
            ui.pack(l, Pack::default());
            let mid = ui.frame(r, &bg);
            ui.pack(mid, Pack::default().side(Side::Left).fill(Fill::X).expand().padx(10, 10));
            let l = label_w(ui, mid, x["title"].as_str().unwrap_or(""), &bg, &th.ink, &f.ui, 11, true);
            ui.pack(l, Pack::default().fill(Fill::X));
            let tm = x["time"].as_str().unwrap_or("");
            let sub = format!("{}  \u{b7}  {}{}{}", kind, if late { "overdue - " } else { "" }, crate::study::when_text(date, d.today),
                              if tm.is_empty() { String::new() } else { format!("  \u{b7}  {tm}") });
            let l = label_w(ui, mid, &sub, &bg, if late { &th.bad } else { &th.grey }, &f.ui, 9, false);
            ui.pack(l, Pack::default().fill(Fill::X));
            let key = format!("{}\u{1f}{}\u{1f}{}", x["id"].as_str().unwrap_or(""), x["title"].as_str().unwrap_or(""), date);
            let c1 = self.cmd(format!("dl_done:{key}"));
            let b = tt_btn(ui, r, "Done", &c1, Some(&th.donebtn), None, 9, 8, 1, th, f);
            ui.pack(b, Pack::default().side(Side::Right).padx(2, 2));
            let c2 = self.cmd(format!("dl_del:{key}"));
            let b = tt_btn(ui, r, "Delete", &c2, Some(&th.delbtn), None, 9, 8, 1, th, f);
            ui.pack(b, Pack::default().side(Side::Right).padx(2, 2));
        }
        let sp = ui.frame(c, &bg);
        ui.ws[sp].opt_h = 10;
        ui.pack(sp, Pack::default());
    }

    /// Zenko14 ib_card: the IA / EE / TOK tracker
    fn ib_card(&mut self, ui: &mut Ui, th: &Theme, f: &Faces, d: &UpData) {
        let bg = th.card.clone();
        let serif = if d.serif.is_empty() { f.ui.clone() } else { d.serif.clone() };
        let jp = if !d.mincho.is_empty() { d.mincho.clone() } else if !d.jp.is_empty() { d.jp.clone() } else { f.ui.clone() };
        let c = hub_card(ui, self.body, Some("\u{8ab2}\u{984c} \u{b7} IA \u{b7} EE \u{b7} TOK tracker"), "leaf", th, f);
        ui.pack(c, Pack::default().fill(Fill::X).pady(12, 0));
        let top = ui.frame(c, &bg);
        ui.pack(top, Pack::default().fill(Fill::X).padx(16, 16).pady(0, 6));
        let t = if d.exam_days >= 0 {
            let (y, m, dd) = crate::civil_from_days_pub(d.exam_day);
            format!("{} days until your first exam ({dd:02} {} {y}). It scrolls on her ema plaque too.", d.exam_days,
                    &MON[m as usize - 1][..3])
        } else {
            "Exams are done. Otsukaresama~".into()
        };
        let mut font = Font::new(&serif, 10, false);
        font.italic = true;
        let l = lbl(ui, top, &t, &bg, &th.grey, font);
        ui.set_anchor(l, Anchor::W);
        wl(ui, l, 0, true);
        ui.pack(l, Pack::default().side(Side::Left).fill(Fill::X));
        let tasks: Vec<Value> = d.life["ibtasks"].as_array().cloned().unwrap_or_default();
        let add = ui.frame(c, &bg);
        ui.pack(add, Pack::default().fill(Fill::X).padx(16, 16).pady(2, 8));
        let l = lbl(ui, add, "Add:", &bg, &th.ink, Font::new(&f.ui, 10, false));
        ui.pack(l, Pack::default().side(Side::Left).padx(0, 6));
        let mut n = 1;
        for (key, label) in [("ee", "Extended Essay"), ("tok_essay", "TOK essay"), ("tok_exh", "TOK exhibition")] {
            if tasks.iter().any(|t| t["kind"] == key) {
                continue;
            }
            let b = tt_btn(ui, add, label, &format!("ib_add:{key}:"), Some(&th.field), None, 9, 8, 1, th, f);
            ui.pack(b, Pack::default().side(Side::Left).padx(0, 6));
            n += 1;
        }
        if n <= 1 {
            ui.destroy(add);
        }
        let left_ia: Vec<&(String, String)> = d.subjects.iter()
            .filter(|(sid, _)| !tasks.iter().any(|t| t["kind"] == "ia" && t["subject"].as_str() == Some(sid))).collect();
        if !left_ia.is_empty() {
            let a2 = ui.frame(c, &bg);
            ui.pack(a2, Pack::default().fill(Fill::X).padx(16, 16).pady(0, 8));
            let l = lbl(ui, a2, "Add an IA:", &bg, &th.ink, Font::new(&f.ui, 10, false));
            ui.pack(l, Pack::default().side(Side::Left).padx(0, 6));
            for (sid, short) in left_ia {
                let b = tt_btn(ui, a2, short, &format!("ib_add:ia:{sid}"), Some(&th.field), None, 9, 8, 1, th, f);
                ui.pack(b, Pack::default().side(Side::Left).padx(0, 6));
            }
        }
        let mut order: Vec<usize> = (0..tasks.len()).collect();
        let due_of = |t: &Value| t["due"].as_str().filter(|s| !s.is_empty()).unwrap_or("9999").to_string();
        order.sort_by(|&a, &b| (due_of(&tasks[a]), tasks[a]["title"].as_str().unwrap_or(""))
            .cmp(&(due_of(&tasks[b]), tasks[b]["title"].as_str().unwrap_or(""))));
        if order.is_empty() {
            let l = label_w(ui, c, "Add your Extended Essay, TOK pieces and IAs. I'll track word counts, deadlines and milestones, \
                                    and nudge you when one goes quiet. You can also say 'my bio IA is 1450 words' or 'EE due 15 March' in chat.",
                            &bg, &th.grey, &f.ui, 9, false);
            wl(ui, l, 0, true);
            ui.pack(l, Pack::default().fill(Fill::X).padx(16, 16).pady(0, 14));
            return;
        }
        let rbg = mixhex(&bg, &th.ink, 0.03);
        for ti in order {
            let t = &tasks[ti];
            let row = ui.frame(c, &rbg);
            (ui.ws[row].hl, ui.ws[row].hlc) = (1, th.line.clone());
            ui.ws[row].gw_cols = vec![0, 1];
            ui.pack(row, Pack::default().fill(Fill::X).padx(16, 16).pady(4, 4));
            let cvw = ui.canvas(row, &rbg, 50, 56);
            ui.grid(cvw, Grid { row: 0, col: 0, rspan: 3, padx: (10, 10), pady: (8, 8), sticky: "n", ..Default::default() });
            let due = t["due"].as_str().unwrap_or("");
            let days = if due.is_empty() { None } else { crate::plan::parse_iso(due).map(|x| x - d.today) };
            let urgent = days.is_some_and(|x| x <= 7);
            let (wood, edge) = if urgent {
                (mixhex(SHU, &bg, 0.15), mixhex(SHU, "#000000", 0.3))
            } else {
                (mixhex("#c98a35", &bg, 0.25), mixhex("#7a4a1c", &bg, 0.1))
            };
            // IB_TASK_KINDS' kanji
            let kan = match t["kind"].as_str().unwrap_or("") {
                "ee" => "\u{8ad6}",
                "tok_essay" => "\u{77e5}",
                "tok_exh" => "\u{5c55}",
                "ia" => "\u{7814}",
                _ => "\u{8ab2}",
            };
            {
                let cv = canvas_of(ui, cvw);
                cv.poly(vec![25.0, 2.0, 47.0, 13.0, 47.0, 54.0, 3.0, 54.0, 3.0, 13.0]).fill(&wood).outline(&edge).width(2.0);
                cv.line(vec![17.0, 8.0, 33.0, 8.0]).fill(&edge).width(2.0);
                cv.text(25.0, 33.0, kan).fill("#fff6e0").font(Font::new(&jp, 16, true));
            }
            let head = ui.frame(row, &rbg);
            ui.grid(head, Grid { row: 0, col: 1, sticky: "ew", pady: (8, 0), ..Default::default() });
            let l = lbl(ui, head, t["title"].as_str().unwrap_or(""), &rbg, &th.ink, Font::new(&serif, 12, true));
            ui.set_anchor(l, Anchor::W);
            ui.pack(l, Pack::default().side(Side::Left));
            let dtxt = match days {
                Some(n) => format!("due {} \u{b7} {}", a_d_b(d.today + n), if n == 0 { "today!".to_string() } else if n >= 0 {
                    format!("{n} days") } else { format!("{} days late", -n) }),
                None => "no deadline yet".into(),
            };
            let col = if urgent { mixhex(SHU, &th.ink, 0.3) } else { th.grey.clone() };
            let l = lbl(ui, head, &format!("   {dtxt}"), &rbg, &col, Font::new(&f.ui, 9, false));
            ui.set_anchor(l, Anchor::W);
            ui.pack(l, Pack::default().side(Side::Left));
            let x = lbl(ui, head, "\u{2715}", &rbg, &th.grey, Font::new(&f.ui, 9, false));
            ui.pack(x, Pack::default().side(Side::Right).padx(10, 10));
            let tid = t["id"].as_str().unwrap_or("").to_string();
            self.clicks.push((x, format!("ib_del:{tid}")));
            let pr = ui.frame(row, &rbg);
            ui.grid(pr, Grid { row: 1, col: 1, sticky: "ew", pady: (4, 0), ..Default::default() });
            let tg = num(&t["target"]) as i64;
            let unit = t["unit"].as_str().unwrap_or("words").to_string();
            let words = num(&t["words"]) as i64;
            let frac = if tg != 0 { words as f64 / tg as f64 } else { 0.0 };
            let bar = ui.canvas(pr, &rbg, 180, 12);
            canvas_of(ui, bar);
            ui.pack(bar, Pack::default().side(Side::Left));
            self.bars.push((bar, frac, 10.0));
            let over = tg != 0 && words > tg;
            let txt = format!("  {}{} {}{}", fmt_num(words), if tg != 0 { format!(" / {}", fmt_num(tg)) } else { String::new() }, unit,
                              if over { "  (over the limit!)" } else { "" });
            let l = lbl(ui, pr, &txt, &rbg, &if over { mixhex(SHU, &th.ink, 0.25) } else { th.ink.clone() }, Font::new(&f.ui, 9, false));
            ui.pack(l, Pack::default().side(Side::Left));
            let we = ui.entry(pr, 7, Font::new(&f.ui, 10, false), "", &th.field);
            ui.pack(we, Pack::default().side(Side::Left).padx(10, 4));
            let b = tt_btn(ui, pr, "Update", &format!("ib_words:{tid}"), Some(&th.start), None, 8, 8, 0, th, f);
            ui.pack(b, Pack::default().side(Side::Left).padx(0, 4));
            if unit == "words" {
                let b = tt_btn(ui, pr, "Count my file\u{2026}", &format!("ib_count:{tid}"), Some(&th.field), None, 8, 8, 0, th, f);
                ui.pack(b, Pack::default().side(Side::Left).padx(0, 4));
            }
            let st = ui.frame(row, &rbg);
            ui.grid(st, Grid { row: 2, col: 1, sticky: "ew", pady: (6, 8), ..Default::default() });
            let have: Vec<String> = t["stages"].as_array().map(|a| a.iter().filter_map(|x| x.as_str().map(String::from)).collect())
                .unwrap_or_default();
            for s_ in STAGES {
                let on = have.iter().any(|h| h == s_);
                let l = lbl(ui, st, &format!("{}{s_}", if on { "\u{2713} " } else { "" }),
                            &if on { mixhex(&bg, &th.gold, 0.28) } else { th.field.clone() }, if on { &th.ink } else { &th.grey },
                            Font::new(&f.ui, 8, on));
                (ui.ws[l].padx, ui.ws[l].pady) = (7, 1);
                ui.pack(l, Pack::default().side(Side::Left).padx(0, 4));
                self.clicks.push((l, format!("ib_stage:{tid}:{s_}")));
            }
            let l = lbl(ui, st, "   due", &rbg, &th.grey, Font::new(&f.ui, 8, false));
            ui.pack(l, Pack::default().side(Side::Left));
            let de = ui.entry(st, 11, Font::new(&f.ui, 9, false), due, &th.field);
            ui.pack(de, Pack::default().side(Side::Left).padx(4, 0));
            self.ib_entries.push((we, de, ti));
        }
        let l = label_w(ui, c, "Tip: type a number and press Enter to log words. Tap a milestone when you finish it. Word limits follow \
                                the IB guides; set your own by saying 'my EE limit is 4000'.", &bg, &th.grey, &f.ui, 8, false);
        wl(ui, l, 0, true);
        ui.pack(l, Pack::default().fill(Fill::X).padx(16, 16).pady(6, 14));
    }

    /// the bars for their widths now (their <Configure> draws)
    pub fn draw_bars(&mut self, ui: &mut Ui, th: &Theme) {
        let ws: Vec<i32> = self.bars.iter().map(|(b, _, _)| ui.ws[*b].w).collect();
        if ws == self.drawn {
            return;
        }
        self.drawn = ws.clone();
        for ((b, frac, y2), w) in self.bars.clone().into_iter().zip(ws) {
            let cv = canvas_of(ui, b);
            *cv = Canvas::default();
            glow_bar(cv, 1.0, 1.0, (w - 2) as f64, y2, frac, th);
        }
    }

    /// set the fox picture on the clock face (small_fox(1)) before drawing
    pub fn set_fox(&mut self, fox: Rc<Photo>) {
        self.fox = Some(fox);
    }

    /// zk_fit for the page, the banner and the bars; lays out at the window's size (pw, ph)
    #[allow(clippy::too_many_arguments)]
    pub fn fit(&mut self, ui: &mut Ui, fonts: &dyn Fonts, th: &Theme, f: &Faces, zk_f: f64, pw: i32, ph: i32) {
        zk_fonts(ui, fonts, self.pg, zk_f);
        ui.layout_at(fonts, pw, ph);
        self.draw_bars(ui, th);
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
        self.draw_bars(ui, th);
        if self.sc.sync(ui) {
            ui.layout_at(fonts, pw, ph);
            self.draw_bars(ui, th);
        }
    }

    /// dl_status(text)
    pub fn set_status(&mut self, ui: &mut Ui, s: &str) {
        self.status_text = s.into();
        if ui.ws.get(self.status).is_some() {
            set_text(ui, self.status, s);
        }
    }
}

/// UpData from the app as it is now
impl crate::Companion {
    pub fn up_data(&mut self) -> UpData {
        let unix = crate::unix_now();
        let now_l = unix + crate::local_offset_pub() as f64;
        let today = now_l.floor().div_euclid(86400.0) as i64;
        let life = self.life_now();
        let deadlines = crate::plan::load_deadlines();
        let targets = self.plan_targets(&life, self.now_naive()).into_iter().map(|t| UpTarget { id: t.id, title: t.title, due: t.due, hours: t.hours })
            .collect();
        let emas = self.ema_list().into_iter().map(|e| (e["id"].as_str().unwrap_or("").to_string(), e["text"].as_str().unwrap_or("").to_string()))
            .collect();
        let exam_days = self.exam_days();
        UpData {
            today, now_l, targets, life, tt: self.tt.clone(), deadlines, planner: self.cfg.b("planner", true),
            session: self.session.is_some(), emas, exam_days, exam_day: today + exam_days,
            jp: String::new(), serif: String::new(), mincho: String::new(),
            subjects: crate::watch::data().subjects.iter().map(|s| (s.sid.clone(), crate::study::sname(&s.sid))).collect(),
        }
    }

    /// life_add_typed(q): the Upcoming page's box ("chemistry exam in 2 days" too)
    pub fn life_add_typed(&mut self, now: f64, q: &str) -> String {
        let res = self.deadline_route(now, &format!("add deadline {q}"));
        if let Some(r) = &res
            && !r.starts_with("When is it") {
            self.plan_maybe();
            return r.clone();
        }
        let today = (now.floor() as i64 + crate::local_offset_pub()).div_euclid(86400);
        let Some(d) = crate::lifechat::parse_rel_date(q, today) else { return res.unwrap_or_default() };
        if rx!(r"(?i)\b(exams?|tests?|quiz|midterm|final|assessment|mock)\b").is_match(q) {
            return self.exam_from_text(now, q, d).split('\n').next().unwrap_or("").to_string();
        }
        let title = rx!(r"(?i)\b(?:in\s+\S+\s+(?:days?|weeks?)|(?:the )?day after tomorrow|tomorrow|today|tonight|(?:on |this |next )?(?:mon|tue|tues|wed|wednes|thu|thur|thurs|fri|sat|satur|sun)(?:day)?)\b")
            .replace_all(q, " ").to_string();
        let (t, span) = crate::lifechat::find_time(&title);
        let mut title = title;
        if let Some((a, b)) = span {
            title = format!("{} {}", &title[..a], &title[b..]);
        }
        let title = rx!(r"(?i)\b(on|by|at|due)\s*$").replace(title.trim(), "").to_string();
        let title = rx!(r"\s+").replace_all(&title, " ").trim().trim_matches(|c| c == ' ' || c == '-' || c == ',').to_string();
        let title = if title.is_empty() { "Deadline".to_string() } else { title };
        let mut cs = title.chars();
        let title = cs.next().map(|c| c.to_uppercase().collect::<String>() + cs.as_str()).unwrap_or_default();
        let iso = crate::plan::iso(d);
        self.add_deadline(now, &title, &iso, &t, None, "");
        self.plan_maybe();
        let (y, _, _) = crate::civil_from_days_pub(d);
        format!("Added: {title} on {} {y} ({}).", a_d_b(d), crate::study::when_text(&iso, today))
    }
}
