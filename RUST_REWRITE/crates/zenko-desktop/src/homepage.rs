//! The hub's Home page (zenko.py hub_build_home, hub_build_session_card, hub_refresh_home, hub_draw_week, hub_intro,
//! hub_music_card + music_icons): the four ema stat cards, the focus card in its states, "Coming up today", the week
//! chart, Quick ask and the Music card.

use crate::aa::{Shape, Sh, aa_pixels};
use crate::ema::fmt_hm;
use crate::hubside::zk_size;
use crate::hubwin::{canvas_of, hub_title_draw};
use crate::pagekit::{Faces, Scroller, WrapState, hub_card, hub_scroller, label_w, tt_btn, tt_entry, zk_fonts, zk_wrap};
use crate::session::clip;
use crate::shapes::round_pts_corners;
use crate::theme::Theme;
use crate::tkcanvas::{Canvas, Photo};
use crate::ui::{Anchor, Fill, Font, Fonts, Grid, Kind, Pack, Side, Ui};
use serde_json::Value;
use std::rc::Rc;
use zenko_art::palette::{hex_rgb, mixhex};

const WD: [&str; 7] = ["Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday", "Sunday"];
const MO: [&str; 12] = ["January", "February", "March", "April", "May", "June", "July", "August", "September", "October",
                        "November", "December"];

/// What the page shows, gathered from the app (Companion::home_data) so the page itself is plain widgets
#[derive(Clone, Debug, Default)]
pub struct HomeData {
    /// local day number (days since 1970-01-01) and hour
    pub day: i64,
    pub hour: u32,
    /// int(time.time()) (the blinking colon, the 5-second deadline refresh)
    pub unix_s: i64,
    pub user_short: String,
    /// (task, planned, elapsed, paused, dist, closed)
    pub session: Option<(String, f64, f64, bool, f64, i64)>,
    /// (planned, elapsed)
    pub brk: Option<(f64, f64)>,
    pub focus_min: i64,
    /// today's timetable rows: (start, task, mins, done)
    pub rows: Vec<(String, String, i64, bool)>,
    /// focus_item: (start, task, mins)
    pub focus: Option<(String, String, i64)>,
    /// the next deadline: (when_text, title)
    pub due: Option<(String, String)>,
    /// the last 7 days' focus minutes, oldest first
    pub week: [i64; 7],
    pub last_task: String,
    pub last_minutes: String,
    /// the first three music presets that have a value: (column, name)
    pub presets: Vec<(usize, String)>,
    pub spotify: bool,
    pub theme: String,
}

pub struct HomePage {
    pub pg: usize,
    pub title: usize,
    pub sc: Scroller,
    pub stat: Vec<(usize, usize)>,
    pub sess: usize,
    pub next_body: usize,
    pub week: usize,
    pub music: (usize, usize),
    pub pp: usize,
    pub timer: Option<usize>,
    pub bar: Option<usize>,
    pub sess_info: Option<usize>,
    pub task_entry: Option<usize>,
    pub min_entry: Option<usize>,
    pub wrap: WrapState,
    key: Option<(bool, bool, bool, String)>,
    next_key: Option<Vec<(String, String, bool)>>,
    wk_key: Option<(i64, i64)>,
    seig: Option<(String, Rc<Photo>)>,
    greet: String,
    sub: String,
    pub icons: Rc<MusicIcons>,
}

/// music_icons(): (normal, hover) for prev / next / play / pause, and the disc
#[derive(Debug)]
pub struct MusicIcons {
    pub prev: (Rc<Photo>, Rc<Photo>),
    pub next: (Rc<Photo>, Rc<Photo>),
    pub play: (Rc<Photo>, Rc<Photo>),
    pub pause: (Rc<Photo>, Rc<Photo>),
    pub disc: Rc<Photo>,
}

fn a(c: &str, al: f64) -> Option<(String, f64)> {
    Some((c.to_string(), al))
}

pub fn music_icons(th: &Theme) -> MusicIcons {
    let card = th.card.clone();
    let (fld, hov) = (th.start.clone(), mixhex(&th.start, "#ffffff", 0.14));
    let btn = |kind: &str, big: bool, hover: bool| -> Rc<Photo> {
        let s = if big { 48 } else { 38 };
        let c = s as f64 / 2.0;
        let r = c - 2.0;
        let fill = if big { if hover { mixhex(&th.acc, "#ffffff", 0.15) } else { th.acc.clone() } }
                   else if hover { hov.clone() } else { fld.clone() };
        let ink = if big { th.onacc.clone() } else { th.ink.clone() };
        let rim = if hover { th.gold.clone() } else { mixhex(&fill, "#ffffff", 0.25) };
        let mut sh: Vec<Sh> = vec![(Shape::Circle(c, c + 1.2, r), a(&mixhex(&card, "#000000", 0.5), 0.6)),
                                   (Shape::Circle(c, c, r), a(&fill, 1.0)), (Shape::Ring(c, c, r, 1.2), a(&rim, 0.9))];
        let k = s as f64 / 38.0;
        let ik = a(&ink, 1.0);
        match kind {
            "prev" => {
                sh.push((Shape::Poly(vec![(c + 5.0 * k, c - 6.0 * k), (c + 5.0 * k, c + 6.0 * k), (c - 4.0 * k, c)]), ik.clone()));
                sh.push((Shape::RRect(c - 7.0 * k, c - 6.0 * k, c - 4.5 * k, c + 6.0 * k, 1.0), ik));
            }
            "next" => {
                sh.push((Shape::Poly(vec![(c - 5.0 * k, c - 6.0 * k), (c - 5.0 * k, c + 6.0 * k), (c + 4.0 * k, c)]), ik.clone()));
                sh.push((Shape::RRect(c + 4.5 * k, c - 6.0 * k, c + 7.0 * k, c + 6.0 * k, 1.0), ik));
            }
            "play" => sh.push((Shape::Poly(vec![(c - 3.5 * k, c - 7.0 * k), (c - 3.5 * k, c + 7.0 * k), (c + 7.0 * k, c)]), ik)),
            _ => {
                sh.push((Shape::RRect(c - 5.5 * k, c - 6.5 * k, c - 1.5 * k, c + 6.5 * k, 1.2), ik.clone()));
                sh.push((Shape::RRect(c + 1.5 * k, c - 6.5 * k, c + 5.5 * k, c + 6.5 * k, 1.2), ik));
            }
        }
        Rc::new(aa_pixels(s, s, &sh, Some(hex_rgb(&card))))
    };
    let pair = |kind: &str, big: bool| (btn(kind, big, false), btn(kind, big, true));
    let mut disc: Vec<Sh> = vec![(Shape::Circle(32.0, 33.0, 30.0), a(&mixhex(&card, "#000000", 0.5), 0.6)),
                                 (Shape::Circle(32.0, 32.0, 30.0), a("#16151c", 1.0))];
    for rr in [26.0, 22.5, 19.0, 15.5] {
        disc.push((Shape::Ring(32.0, 32.0, rr, 0.8), a("#ffffff", 0.07)));
    }
    disc.extend([(Shape::Ring(32.0, 32.0, 29.5, 1.0), a(&th.gold, 0.5)), (Shape::Circle(32.0, 32.0, 10.0), a(&th.acc, 1.0)),
                 (Shape::Circle(32.0, 32.0, 6.5), a(&mixhex(&th.acc, "#ffffff", 0.3), 0.6)),
                 (Shape::Circle(32.0, 32.0, 2.2), a(&card, 1.0)),
                 (Shape::Poly(vec![(14.0, 15.0), (22.0, 9.0), (25.0, 12.0), (17.0, 18.0)]), a("#ffffff", 0.10))]);
    MusicIcons { prev: pair("prev", false), next: pair("next", false), play: pair("play", true), pause: pair("pause", true),
                 disc: Rc::new(aa_pixels(64, 64, &disc, Some(hex_rgb(&card)))) }
}

/// date.strftime("%A, %d %B %Y")
pub fn long_day(day: i64) -> String {
    let (y, m, d) = crate::civil_from_days_pub(day);
    format!("{}, {d:02} {} {y}", WD[crate::lifechat::weekday(day)], MO[m as usize - 1])
}

/// datetime.now().hour < 12 / < 18
pub fn greeting(hour: u32, user_short: &str) -> String {
    let g = if hour < 12 { "Good morning" } else if hour < 18 { "Good afternoon" } else { "Good evening" };
    if user_short.is_empty() { g.into() } else { format!("{g}, {user_short}") }
}

fn lbl(ui: &mut Ui, parent: usize, text: &str, bg: &str, fg: &str, fam: &str, size: i32, bold: bool, p: Pack) -> usize {
    let l = label_w(ui, parent, text, bg, fg, fam, size, bold);
    ui.pack(l, p);
    l
}

fn set_text(ui: &mut Ui, id: usize, s: &str) {
    if let Kind::Label { text, .. } = &mut ui.ws[id].kind {
        *text = s.into();
    }
}

fn text_of(ui: &Ui, id: usize) -> String {
    match &ui.ws[id].kind {
        Kind::Label { text, .. } => text.clone(),
        _ => String::new(),
    }
}

/// Python's "%02d" of a float: int() of it
fn d2(v: f64) -> String {
    format!("{:02}", v.trunc() as i64)
}

impl HomePage {
    /// hub_build_home(pg)
    pub fn build(ui: &mut Ui, pg: usize, th: &Theme, f: &Faces, cm10: i32, d: &HomeData) -> HomePage {
        let greet = greeting(d.hour, &d.user_short);
        let sub = long_day(d.day);
        let title = ui.canvas(pg, &th.banner, cm10, 96);
        canvas_of(ui, title);
        ui.pack(title, Pack::default().fill(Fill::X));
        let sc = hub_scroller(ui, pg, th, cm10);
        let body = ui.frame(sc.inner, &th.paper);
        ui.pack(body, Pack::default().fill(Fill::X).padx(24, 24).pady(14, 14));
        let row = ui.frame(body, &th.paper);
        ui.pack(row, Pack::default().fill(Fill::X));
        let card = th.card.clone();
        let mut stat = vec![];
        for i in 0..4 {
            let c = hub_card(ui, row, None, "ema", th, f);
            ui.pack(c, Pack::default().side(Side::Left).fill(Fill::Both).expand().padx(if i == 0 { 0 } else { 10 }, 0));
            let big = lbl(ui, c, "-", &card, &th.accd, &f.ui, 20, true, Pack::default().fill(Fill::X).padx(14, 14).pady(10, 0));
            let cap = lbl(ui, c, "", &card, &th.grey, &f.ui, 9, false, Pack::default().fill(Fill::X).padx(14, 14).pady(0, 10));
            stat.push((big, cap));
        }
        let mid = ui.frame(body, &th.paper);
        ui.pack(mid, Pack::default().fill(Fill::Both).expand().pady(12, 0));
        let leftcol = ui.frame(mid, &th.paper);
        ui.pack(leftcol, Pack::default().side(Side::Left).fill(Fill::Both).expand());
        let sess = hub_card(ui, leftcol, None, "leaf", th, f);
        ui.pack(sess, Pack::default().fill(Fill::X));
        let next = hub_card(ui, leftcol, Some("Coming up today"), "round", th, f);
        ui.pack(next, Pack::default().fill(Fill::Both).expand().pady(12, 0));
        let next_body = ui.frame(next, &card);
        ui.pack(next_body, Pack::default().fill(Fill::Both).expand().padx(14, 14).pady(0, 10));
        let right = ui.frame(mid, &th.paper);
        ui.pack(right, Pack::default().side(Side::Left).fill(Fill::Y).padx(12, 0));
        let wk = hub_card(ui, right, Some("Last 7 days (focus minutes)"), "round", th, f);
        ui.pack(wk, Pack::default().fill(Fill::X));
        let week = ui.canvas(wk, &card, 270, 130);
        canvas_of(ui, week);
        ui.pack(week, Pack::default().padx(12, 12).pady(0, 10));
        let qa = hub_card(ui, right, Some("Quick ask"), "round", th, f);
        ui.pack(qa, Pack::default().fill(Fill::X).pady(12, 0));
        let grid = ui.frame(qa, &card);
        ui.pack(grid, Pack::default().padx(12, 12).pady(0, 12));
        for (i, (lab, cmd)) in [("Weather", "weather"), ("Headlines", "news"), ("Study tip", "tip"), ("Motivate me", "motivate me")]
            .iter().enumerate() {
            let b = tt_btn(ui, grid, lab, &format!("hub_ask:{cmd}"), Some(&th.start), None, 9, 10, 3, th, f);
            ui.grid(b, Grid { row: i / 2, col: i % 2, sticky: "ew", padx: (3, 3), pady: (3, 3), ..Default::default() });
        }
        // hub_music_card(right)
        let icons = Rc::new(music_icons(th));
        let c = hub_card(ui, right, Some("Music"), "round", th, f);
        ui.pack(c, Pack::default().fill(Fill::X).pady(12, 0));
        let top = ui.frame(c, &card);
        ui.pack(top, Pack::default().fill(Fill::X).padx(14, 14));
        let disc = ui.image_label(top, icons.disc.clone(), &card, 2);
        ui.pack(disc, Pack::default().side(Side::Left));
        let tx = ui.frame(top, &card);
        ui.pack(tx, Pack::default().side(Side::Left).fill(Fill::X).expand().padx(10, 0));
        let lab = lbl(ui, tx, "Spotify", &card, &th.ink, &f.ui, 11, true, Pack::default().fill(Fill::X));
        let st = if d.spotify { "..." } else { "Not connected: Settings > Spotify" };
        let sub_l = lbl(ui, tx, st, &card, &th.grey, &f.ui, 9, false, Pack::default().fill(Fill::X));
        if let Kind::Label { wrap, left, .. } = &mut ui.ws[sub_l].kind {
            (*wrap, *left) = (170, true);
        }
        let row = ui.frame(c, &card);
        ui.pack(row, Pack::default().pady(8, 10));
        let mut pp = 0;
        for (k, img) in [("prev", icons.prev.0.clone()), ("pp", icons.play.0.clone()), ("next", icons.next.0.clone())] {
            let b = ui.image_label(row, img, &card, 0);
            ui.ws[b].name = format!("music:{k}");
            ui.pack(b, Pack::default().side(Side::Left).padx(5, 5));
            if k == "pp" {
                pp = b;
            }
        }
        let pr = ui.frame(c, &card);
        ui.pack(pr, Pack::default().padx(14, 14).pady(0, 12));
        for (i, name) in d.presets.iter().map(|(i, n)| (*i, n)) {
            let b = tt_btn(ui, pr, &clip(name, 16), &format!("music_preset:{i}"), None, None, 9, 8, 1, th, f);
            ui.grid(b, Grid { row: 0, col: i, padx: (3, 3), ..Default::default() });
        }
        HomePage { pg, title, sc, stat, sess, next_body, week, music: (lab, sub_l), pp, timer: None, bar: None, sess_info: None,
                   task_entry: None, min_entry: None, wrap: WrapState::default(), key: None, next_key: None, wk_key: None,
                   seig: None, greet, sub, icons }
    }

    /// hub_build_session_card: the focus card for no session / a session / a break
    fn session_card(&mut self, ui: &mut Ui, th: &Theme, f: &Faces, d: &HomeData) {
        let c = self.sess;
        for k in ui.ws[c].kids.clone() {
            if ui.ws[k].place.is_none() {
                ui.destroy(k);
            }
        }
        (self.timer, self.bar, self.sess_info, self.task_entry, self.min_entry) = (None, None, None, None, None);
        let bg = th.card.clone();
        let fam = f.ui.clone();
        if d.session.is_some() || d.brk.is_some() {
            let b = d.brk.is_some();
            let s = d.session.as_ref();
            let state = if b { "On a break" } else if s.unwrap().3 { "Paused" } else { "Focusing" };
            lbl(ui, c, state, &bg, &th.grey, &fam, 10, true, Pack::default().fill(Fill::X).padx(18, 18).pady(14, 0));
            let what = if b { "Break".to_string() } else { clip(&s.unwrap().0, 34) };
            lbl(ui, c, &what, &bg, &th.ink, &fam, 18, true, Pack::default().fill(Fill::X).padx(18, 18));
            let t = label_w(ui, c, "--:--", &bg, &th.gold, "Consolas", 46, true);
            ui.pack(t, Pack::default().fill(Fill::X).padx(18, 18));
            let bar = ui.canvas(c, &bg, 0, 20);
            canvas_of(ui, bar);
            let cm10 = ui.ws[self.title].opt_w;
            ui.ws[bar].opt_w = cm10;
            let info = label_w(ui, c, "", &bg, &th.grey, &fam, 9, false);
            if !b {
                ui.pack(bar, Pack::default().fill(Fill::X).padx(18, 18).pady(0, 4));
                ui.pack(info, Pack::default().fill(Fill::X).padx(18, 18));
            }
            let btns = ui.frame(c, &bg);
            ui.pack(btns, Pack::default().anchor(Anchor::W).padx(18, 18).pady(14, 14));
            if b {
                let x = tt_btn(ui, btns, "End break", "end_break", Some(&th.start), None, 11, 16, 4, th, f);
                ui.pack(x, Pack::default().side(Side::Left).padx(0, 8));
            } else {
                let paused = s.unwrap().3;
                let x = tt_btn(ui, btns, if paused { "Resume" } else { "Pause" }, "toggle_pause", Some(&th.start), None, 11, 16, 4, th, f);
                ui.pack(x, Pack::default().side(Side::Left).padx(0, 8));
                let x = tt_btn(ui, btns, "Stop", "end_session", Some(&th.delbtn), None, 11, 16, 4, th, f);
                ui.pack(x, Pack::default().side(Side::Left).padx(0, 8));
                let x = tt_btn(ui, btns, "Take a break", "start_break", Some(&th.field), None, 11, 16, 4, th, f);
                ui.pack(x, Pack::default().side(Side::Left));
            }
            (self.timer, self.bar, self.sess_info) = (Some(t), Some(bar), Some(info));
            return;
        }
        lbl(ui, c, "Start a focus session", &bg, &th.ink, &fam, 16, true, Pack::default().fill(Fill::X).padx(18, 18).pady(14, 2));
        lbl(ui, c, "I'll watch for distractions and keep you company.", &bg, &th.grey, &fam, 10, false,
            Pack::default().fill(Fill::X).padx(18, 18).pady(0, 10));
        lbl(ui, c, "What are you working on?", &bg, &th.grey, &fam, 9, false, Pack::default().fill(Fill::X).padx(18, 18));
        let te = tt_entry(ui, c, &d.last_task, 40, th, f);
        ui.pack(te, Pack::default().fill(Fill::X).padx(18, 18).pady(2, 10));
        let r = ui.frame(c, &bg);
        ui.pack(r, Pack::default().fill(Fill::X).padx(18, 18));
        let m = ui.label(r, "Minutes", &bg, &th.grey, Font::new(&fam, 9, false));
        ui.pack(m, Pack::default().side(Side::Left));
        let me = tt_entry(ui, r, &d.last_minutes, 5, th, f);
        ui.pack(me, Pack::default().side(Side::Left).padx(8, 12));
        for v in [15, 25, 45, 60, 90] {
            let b = tt_btn(ui, r, &v.to_string(), &format!("home_min:{v}"), Some(&th.field), None, 9, 8, 1, th, f);
            ui.pack(b, Pack::default().side(Side::Left).padx(2, 2));
        }
        let go = tt_btn(ui, c, "\u{2726}  Start focusing", "home_go", Some(&th.acc), Some(&th.onacc), 13, 24, 8, th, f);
        ui.pack(go, Pack::default().anchor(Anchor::W).padx(18, 18).pady(14, 6));
        if let Some((st, task, mins)) = &d.focus {
            let t = format!("On your timetable: {st} {} ({mins} min)", clip(task, 26));
            lbl(ui, c, &t, &bg, &th.grey, &fam, 9, false, Pack::default().fill(Fill::X).padx(18, 18));
            let b = tt_btn(ui, c, "Start that instead", "home_start_item", Some(&th.start), None, 9, 10, 2, th, f);
            ui.pack(b, Pack::default().anchor(Anchor::W).padx(18, 18).pady(4, 12));
        }
        (self.task_entry, self.min_entry) = (Some(te), Some(me));
    }

    /// hub_refresh_home(force): `counting` = hub_intro's count-up is still running (time.monotonic() <= _count_until)
    pub fn refresh(&mut self, ui: &mut Ui, th: &Theme, f: &Faces, d: &HomeData, force: bool, counting: bool) {
        let key = (d.session.is_some(), d.session.as_ref().is_some_and(|s| s.3), d.brk.is_some(), d.theme.clone());
        if force || self.key.as_ref() != Some(&key) {
            self.key = Some(key);
            self.session_card(ui, th, f, d);
        }
        let done = d.rows.iter().filter(|r| r.3).count();
        if !counting {
            set_text(ui, self.stat[0].0, &format!("{} min", d.focus_min));
            set_text(ui, self.stat[1].0, &format!("{done} / {}", d.rows.len()));
        }
        set_text(ui, self.stat[0].1, "focused today");
        set_text(ui, self.stat[1].1, "timetable items done");
        let (nb, nc) = match &d.focus {
            Some((st, task, _)) => (st.clone(), clip(task, 26)),
            None => ("-".into(), "nothing left today".into()),
        };
        set_text(ui, self.stat[2].0, &nb);
        set_text(ui, self.stat[2].1, &nc);
        if d.unix_s % 5 == 0 || text_of(ui, self.stat[3].0) == "-" {
            let (db, dc) = match &d.due {
                Some((w, t)) => (capitalize(w), clip(t, 26)),
                None => ("None".into(), "no deadlines - drop a file".into()),
            };
            set_text(ui, self.stat[3].0, &db);
            set_text(ui, self.stat[3].1, &dc);
        }
        if let Some((_, planned, elapsed, _, dist, closed)) = &d.session {
            let left = (planned - elapsed).max(0.0);
            let colon = if d.unix_s % 2 == 0 { ":" } else { " " };
            if let Some(t) = self.timer {
                set_text(ui, t, &format!("{}{colon}{}", d2((left / 60.0).floor()), d2(left.rem_euclid(60.0))));
            }
            if let Some(bar) = self.bar {
                let wd = (ui.ws[bar].w - 4).max(10) as f64;
                let frac = elapsed / planned.max(1.0);
                let cv = canvas_of(ui, bar);
                *cv = Canvas::default();
                crate::pagekit::glow_bar(cv, 2.0, 5.0, wd, 15.0, frac, th);
            }
            if let Some(i) = self.sess_info {
                set_text(ui, i, &format!("Off-task so far: {} min   \u{b7}   distractions closed: {closed}",
                                         (dist / 60.0).floor() as i64));
            }
        } else if let Some((planned, elapsed)) = d.brk {
            let left = (planned - elapsed).max(0.0);
            if let Some(t) = self.timer {
                set_text(ui, t, &format!("{}:{}", d2((left / 60.0).floor()), d2(left.rem_euclid(60.0))));
            }
        }
        let nk: Vec<(String, String, bool)> = d.rows.iter().map(|r| (r.0.clone(), r.1.clone(), r.3)).collect();
        if self.next_key.as_ref() != Some(&nk) {
            self.next_key = Some(nk);
            for k in ui.ws[self.next_body].kids.clone() {
                ui.destroy(k);
            }
            let bg = th.card.clone();
            let mut shown = 0;
            for (st, task, mins, done) in &d.rows {
                if *done {
                    continue;
                }
                shown += 1;
                if shown > 5 {
                    break;
                }
                let r = ui.frame(self.next_body, &bg);
                ui.pack(r, Pack::default().fill(Fill::X).pady(2, 2));
                let t = label_w(ui, r, st, &bg, &th.accd, "Consolas", 12, true);
                ui.ws[t].opt_w = 6;
                ui.pack(t, Pack::default().side(Side::Left));
                lbl(ui, r, &clip(task, 34), &bg, &th.ink, &f.ui, 10, false, Pack::default().side(Side::Left));
                let m = ui.label(r, &fmt_hm(*mins), &bg, &th.grey, Font::new(&f.ui, 9, false));
                ui.pack(m, Pack::default().side(Side::Right));
            }
            if shown == 0 {
                let l = lbl(ui, self.next_body, "Nothing left on today's timetable. Add something in the Timetable tab.", &bg,
                            &th.grey, &f.ui, 10, false, Pack::default().fill(Fill::X).pady(6, 6));
                if let Kind::Label { wrap, left, .. } = &mut ui.ws[l].kind {
                    (*wrap, *left) = (380, true);
                }
            }
        }
        let wk = (d.focus_min, d.day);
        if self.wk_key != Some(wk) && !counting {
            self.wk_key = Some(wk);
            self.draw_week(ui, th, f, d, 1.0);
        }
    }

    /// hub_intro's frame at e (0..1): the count-up numbers and the growing bars
    pub fn intro_frame(&mut self, ui: &mut Ui, th: &Theme, f: &Faces, d: &HomeData, e: f64) {
        let done = d.rows.iter().filter(|r| r.3).count() as f64;
        set_text(ui, self.stat[0].0, &format!("{} min", (d.focus_min as f64 * e).round_ties_even() as i64));
        set_text(ui, self.stat[1].0, &format!("{} / {}", (done * e).round_ties_even() as i64, d.rows.len()));
        self.draw_week(ui, th, f, d, e);
    }

    /// hub_draw_week(f)
    pub fn draw_week(&mut self, ui: &mut Ui, th: &Theme, fc: &Faces, d: &HomeData, f: f64) {
        let cv = canvas_of(ui, self.week);
        *cv = Canvas::default();
        let top = d.week.iter().copied().max().unwrap_or(0).max(30) as f64;
        cv.line(vec![4.0, 100.0, 266.0, 100.0]).fill(&th.line);
        for (i, &v) in d.week.iter().enumerate() {
            let x = 10.0 + i as f64 * 37.0;
            let hgt = if v != 0 { (85.0 * v as f64 / top * f).max(2.0) } else { 2.0 };
            let col = if i == 6 { th.acc.clone() } else { mixhex(&th.card, &th.acc, 0.38) };
            let rr = (8.0f64.min(hgt / 2.0)) as i32;
            let pts = round_pts_corners(x, 100.0 - hgt, x + 26.0, 100.0, [(false, rr), (false, rr), (false, 0), (false, 0)], 2);
            cv.poly(pts.into_iter().flat_map(|(a, b)| [a, b]).collect()).fill(&col).outline("");
            if hgt > 6.0 {
                cv.line(vec![x + rr as f64, 100.0 - hgt + 2.0, x + 26.0 - rr as f64, 100.0 - hgt + 2.0]).fill(&mixhex(&col, "#ffffff", 0.45));
            }
            if i == 6 && v != 0 {
                cv.rect(x + 10.0, 100.0 - hgt - 6.0, x + 16.0, 100.0 - hgt - 2.0).fill(&th.glow).outline("");
            }
            if v != 0 && f > 0.95 {
                cv.text(x + 13.0, 100.0 - hgt - 12.0, &v.to_string()).fill(&th.ink).font(Font::new(&fc.ui, 8, true));
            }
            let dn = d.day - 6 + i as i64;
            let wd = &WD[crate::lifechat::weekday(dn)][..2];
            cv.text(x + 13.0, 115.0, wd).fill(if i == 6 { &th.gold } else { &th.grey }).font(Font::new(&fc.ui, 8, false));
        }
    }

    /// zk_fit for the page (fonts, then zk_wrap), then the banner for the canvas's width; lays out at (pw, ph)
    pub fn fit(&mut self, ui: &mut Ui, fonts: &dyn Fonts, th: &Theme, f: &Faces, zk_f: f64, pw: i32, ph: i32) {
        zk_fonts(ui, fonts, self.pg, zk_f);
        ui.layout_at(fonts, pw, ph);
        self.banner(ui, fonts, th, f, zk_f, pw, ph);
        zk_wrap(ui, fonts, self.pg, &mut self.wrap, pw, ph);
        if self.sc.sync(ui) {
            ui.layout_at(fonts, pw, ph);
        }
    }

    /// the banner for the canvas's width now (lays out again when its height changed)
    pub fn banner(&mut self, ui: &mut Ui, fonts: &dyn Fonts, th: &Theme, f: &Faces, zk_f: f64, pw: i32, ph: i32) {
        let w = ui.ws[self.title].w;
        if w < 20 {
            return;
        }
        let mut cv = std::mem::take(canvas_of(ui, self.title));
        let hh = hub_title_draw(&mut cv, w, &self.greet, &self.sub, th, &f.ui, &f.title, zk_f, fonts, &mut self.seig);
        *canvas_of(ui, self.title) = cv;
        if ui.ws[self.title].opt_h != hh {
            ui.ws[self.title].opt_h = hh;
            ui.layout_at(fonts, pw, ph);
        }
    }

    /// the font size a widget of the page has now (zk_size of what it was made with)
    pub fn size_of(ui: &Ui, id: usize, zk_f: f64) -> i32 {
        zk_size(zk_f, ui.ws[id].f0)
    }
}

/// str.capitalize()
fn capitalize(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        Some(f) => f.to_uppercase().collect::<String>() + &c.as_str().to_lowercase(),
        None => String::new(),
    }
}

/// HomeData from the app as it is now
impl crate::Companion {
    pub fn home_data(&self) -> HomeData {
        let unix = crate::unix_now();
        let secs = unix.floor() as i64 + crate::local_offset_pub();
        let day = secs.div_euclid(86400);
        let today = crate::today_iso();
        let rows: Vec<(String, String, i64, bool)> = crate::tt::tt_today(&self.tt, &today).into_iter().map(|i| {
            let it = &self.tt[i];
            (it["start"].as_str().unwrap_or("").into(), it["task"].as_str().unwrap_or("").into(),
             it["mins"].as_f64().unwrap_or(0.0) as i64, crate::tt::tt_done(it, &today))
        }).collect();
        let focus = self.focus_item().map(|i| {
            let it = &self.tt[i];
            (it["start"].as_str().unwrap_or("").into(), it["task"].as_str().unwrap_or("").into(),
             it["mins"].as_f64().unwrap_or(0.0) as i64)
        });
        let due = self.upcoming_deadlines(1).first().map(|d| {
            (crate::study::when_text(d["date"].as_str().unwrap_or(""), day), d["title"].as_str().unwrap_or("").to_string())
        });
        let st = crate::stats();
        let mut week = [0i64; 7];
        for (k, w) in week.iter_mut().enumerate() {
            let iso = crate::plan::iso(day - 6 + k as i64);
            *w = (st.get(&iso).and_then(|v| v.as_f64()).unwrap_or(0.0) / 60.0).floor() as i64;
        }
        let raw = &self.cfg.raw;
        let user = raw.get("user_name").and_then(|v| v.as_str()).unwrap_or("").trim().split(' ').next().unwrap_or("").to_string();
        let presets = raw.get("music_presets").and_then(|v| v.as_array()).map(|a| {
            a.iter().take(3).enumerate().filter(|(_, p)| p.get("value").and_then(|v| v.as_str()).is_some_and(|s| !s.is_empty()))
                .map(|(i, p)| (i, p.get("name").and_then(|v| v.as_str()).filter(|s| !s.is_empty()).map(String::from)
                    .unwrap_or_else(|| format!("Preset {}", i + 1))))
                .collect()
        }).unwrap_or_default();
        let last_minutes = match raw.get("last_minutes") {
            Some(Value::Number(n)) => n.to_string(),
            Some(Value::String(s)) => s.clone(),
            _ => "25".into(),
        };
        HomeData {
            day, hour: (secs.rem_euclid(86400) / 3600) as u32, unix_s: unix.floor() as i64, user_short: user,
            session: self.session.as_ref().map(|s| (s.task.clone(), s.planned, s.elapsed, s.paused, s.dist, s.closed)),
            brk: self.brk.as_ref().map(|b| (b.planned, b.elapsed)),
            focus_min: crate::slow::today_focus_minutes(), rows, focus, due, week,
            last_task: raw.get("last_task").and_then(|v| v.as_str()).unwrap_or("").into(), last_minutes, presets,
            spotify: self.spotify_connected(), theme: raw.get("theme").and_then(|v| v.as_str()).unwrap_or("snow").into(),
        }
    }
}
