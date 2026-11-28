//! The hub's Timetable page: zenko.py open_timetable(parent=pg) (the header with the fox, the day, 急 Delay and
//! Prev / Today / Next, the progress bar, the scrolled day of cards, the add form) and tt_refresh / tt_card.

use crate::ema::fmt_hm;
use crate::pagekit::{Faces, Scroller, WrapState, glow_bar, tt_btn, tt_entry, zk_fonts, zk_wrap};
use crate::session::clip;
use crate::theme::Theme;
use crate::tkcanvas::{Canvas, Photo};
use crate::tt::{tt_done, tt_today};
use crate::ui::{Anchor, Fill, Font, Fonts, Grid, Kind, Pack, Side, Ui};
use serde_json::Value;
use std::rc::Rc;

const WD: [&str; 7] = ["Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday", "Sunday"];
const MON: [&str; 12] = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
/// the page's tt_W, tt_maxh
pub const TT_W: i32 = 700;
pub const TT_MAXH: i32 = 420;

/// What the page shows, from the app (Companion::tt_data)
#[derive(Clone, Debug, Default)]
pub struct TtData {
    /// today's local day number and the local time now (seconds since the epoch, local wall clock)
    pub today: i64,
    pub now_l: f64,
    pub tt: Vec<Value>,
    /// focus_item()'s start time (today only)
    pub next: Option<String>,
}

pub struct TtPage {
    pub pg: usize,
    pub title: usize,
    pub sub: usize,
    pub bar: usize,
    pub sc: Scroller,
    pub te: usize,
    pub ke: usize,
    pub me: usize,
    pub every: usize,
    pub msg: usize,
    /// self.tt_day: days from today
    pub day: i64,
    /// the cards' buttons: (Start, Done/Undo, Delete) -> index into tt
    pub rows: Vec<usize>,
    pub wrap: WrapState,
}

fn lbl(ui: &mut Ui, parent: usize, text: &str, bg: &str, fg: &str, font: Font) -> usize {
    ui.label(parent, text, bg, fg, font)
}

/// "%d %b" / "%A" for a local day number
fn day_mon(dn: i64) -> String {
    let (_, m, d) = crate::civil_from_days_pub(dn);
    format!("{d:02} {}", MON[(m as usize).clamp(1, 12) - 1])
}

impl TtPage {
    /// open_timetable(parent=pg)
    pub fn build(ui: &mut Ui, pg: usize, th: &Theme, f: &Faces, icon: Rc<Photo>, d: &TtData) -> TtPage {
        let hb = th.banner.clone();
        let hdr = ui.frame(pg, &hb);
        ui.pack(hdr, Pack::default().fill(Fill::X));
        let ic = ui.image_label(hdr, icon, &hb, 2);
        ui.pack(ic, Pack::default().side(Side::Left).padx(14, 8).pady(6, 6));
        let mid = ui.frame(hdr, &hb);
        ui.pack(mid, Pack::default().side(Side::Left).fill(Fill::X).expand());
        let title = lbl(ui, mid, "", &hb, &th.ink, Font::new(&f.ui, 18, true));
        ui.set_anchor(title, Anchor::W);
        ui.pack(title, Pack::default().anchor(Anchor::W));
        let sub = lbl(ui, mid, "", &hb, &th.grey, Font::new(&f.ui, 10, false));
        ui.set_anchor(sub, Anchor::W);
        ui.pack(sub, Pack::default().anchor(Anchor::W));
        let nav = ui.frame(hdr, &hb);
        ui.pack(nav, Pack::default().side(Side::Right).padx(12, 12));
        let ub = tt_btn(ui, nav, "\u{6025} Delay", "tt_delay", Some(&th.acc), Some(&th.onacc), 9, 10, 3, th, f);
        ui.pack(ub, Pack::default().side(Side::Left).padx(2, 10));
        for (t, c) in [("< Prev", "tt_nav:-1"), ("Today", "tt_nav:0"), ("Next >", "tt_nav:1")] {
            let b = tt_btn(ui, nav, t, c, None, None, 9, 10, 3, th, f);
            ui.pack(b, Pack::default().side(Side::Left).padx(2, 2));
        }
        let gl = ui.frame(pg, &th.gold);
        ui.ws[gl].opt_h = 1;
        ui.pack(gl, Pack::default().fill(Fill::X));
        let bar = ui.canvas(pg, &th.paper, TT_W + 4, 14);
        crate::hubwin::canvas_of(ui, bar);
        ui.pack(bar, Pack::default().padx(12, 12).pady(10, 2));
        let wrap = ui.frame(pg, &th.paper);
        ui.pack(wrap, Pack::default().fill(Fill::Both).expand().padx(12, 4).pady(2, 2));
        let cv = ui.canvas(wrap, &th.paper, TT_W, 200);
        let sb = ui.canvas(wrap, &th.paper, 10, 40);
        ui.grid(cv, Grid { row: 0, col: 0, ..Default::default() });
        ui.grid(sb, Grid { row: 0, col: 1, sticky: "ns", ..Default::default() });
        let inner = ui.frame(cv, &th.paper);
        ui.ws[inner].cwin = true;
        let sc = Scroller { boxf: wrap, cv, sb, inner };
        // ---- add form
        let fb = th.foot.clone();
        let ln = ui.frame(pg, &th.line);
        ui.ws[ln].opt_h = 1;
        ui.pack(ln, Pack::default().fill(Fill::X).pady(6, 0));
        let foot = ui.frame(pg, &fb);
        ui.pack(foot, Pack::default().fill(Fill::X));
        let g = |row: usize, col: usize, sticky: &'static str, padx: (i32, i32), pady: (i32, i32), cspan: usize| Grid {
            row, col, sticky, padx, pady, cspan, rspan: 1 };
        let l = lbl(ui, foot, "Add to this day", &fb, &th.ink, Font::new(&f.ui, 10, true));
        ui.grid(l, g(0, 0, "w", (12, 12), (8, 2), 3));
        let b = tt_btn(ui, foot, "Export PDF", "tt_export", Some(&th.field), None, 8, 8, 1, th, f);
        ui.grid(b, g(0, 3, "e", (8, 8), (8, 2), 2));
        let l = lbl(ui, foot, "Time (4pm or 16:30)", &fb, &th.grey, Font::new(&f.ui, 8, false));
        ui.grid(l, g(1, 0, "w", (12, 0), (0, 0), 1));
        let l = lbl(ui, foot, "What are you doing?", &fb, &th.grey, Font::new(&f.ui, 8, false));
        ui.grid(l, g(1, 1, "w", (8, 8), (0, 0), 1));
        let l = lbl(ui, foot, "Minutes", &fb, &th.grey, Font::new(&f.ui, 8, false));
        ui.grid(l, g(1, 2, "w", (0, 0), (0, 0), 1));
        let te = tt_entry(ui, foot, "", 9, th, f);
        ui.grid(te, g(2, 0, "w", (12, 0), (0, 0), 1));
        let ke = tt_entry(ui, foot, "", 30, th, f);
        ui.grid(ke, g(2, 1, "w", (8, 8), (0, 0), 1));
        let me = tt_entry(ui, foot, "30", 5, th, f);
        ui.grid(me, g(2, 2, "w", (0, 0), (0, 0), 1));
        let quick = ui.frame(foot, &fb);
        ui.grid(quick, g(3, 0, "w", (12, 12), (6, 0), 3));
        for v in [15, 30, 45, 60, 90] {
            let b = tt_btn(ui, quick, &v.to_string(), &format!("tt_quick:{v}"), Some(&th.field), None, 9, 7, 1, th, f);
            ui.pack(b, Pack::default().side(Side::Left).padx(0, 4));
        }
        let every = ui.check(foot, "every day", &fb, &th.ink, Font::new(&f.ui, 10, false), false, &th.field, "tt_every", 1, 2);
        ui.grid(every, g(3, 3, "", (8, 8), (6, 0), 1));
        let add = tt_btn(ui, foot, "+ Add", "tt_add", Some(&th.acc), Some(&th.onacc), 11, 16, 4, th, f);
        ui.grid(add, g(2, 3, "", (8, 8), (0, 0), 1));
        let msg = lbl(ui, foot, "", &fb, &th.bad, Font::new(&f.ui, 9, false));
        ui.grid(msg, g(4, 0, "w", (12, 12), (2, 8), 6));
        let mut p = TtPage { pg, title, sub, bar, sc, te, ke, me, every, msg, day: 0, rows: vec![], wrap: WrapState::default() };
        p.refresh(ui, th, f, d);
        p
    }

    /// tt_refresh: the header for the day, the bar, every card again
    pub fn refresh(&mut self, ui: &mut Ui, th: &Theme, f: &Faces, d: &TtData) {
        let day = d.today + self.day;
        let ds = crate::plan::iso(day);
        let is_today = self.day == 0;
        let rows = tt_today(&d.tt, &ds);
        let name = match self.day {
            0 => "Today".to_string(),
            1 => "Tomorrow".into(),
            -1 => "Yesterday".into(),
            _ => WD[crate::lifechat::weekday(day)].into(),
        };
        set_text(ui, self.title, &format!("{name}  \u{b7}  {}", day_mon(day)));
        let done_n = rows.iter().filter(|&&i| tt_done(&d.tt[i], &ds)).count();
        let mut sub = if rows.is_empty() {
            "Nothing planned yet".to_string()
        } else {
            let total: i64 = rows.iter().map(|&i| d.tt[i]["mins"].as_f64().unwrap_or(0.0) as i64).sum();
            format!("{done_n} of {} done  \u{b7}  {} planned", rows.len(), fmt_hm(total))
        };
        if is_today && let Some(n) = &d.next {
            sub += &format!("  \u{b7}  next: {n}");
        }
        set_text(ui, self.sub, &sub);
        let frac = if rows.is_empty() { 0.0 } else { done_n as f64 / rows.len() as f64 };
        let cv = crate::hubwin::canvas_of(ui, self.bar);
        *cv = Canvas::default();
        glow_bar(cv, 1.0, 2.0, (TT_W + 3) as f64, 12.0, frac, th);
        let inner = self.sc.inner;
        for k in ui.ws[inner].kids.clone() {
            ui.destroy(k);
        }
        self.rows.clear();
        if rows.is_empty() {
            let l = lbl(ui, inner, "A blank day!\nAdd your first item below.", &th.paper, &th.grey, Font::new(&f.ui, 12, false));
            ui.ws[l].pady = 26;
            ui.pack(l, Pack::default().fill(Fill::X));
        }
        for i in rows {
            self.card(ui, th, f, d, i, day);
        }
        // (the canvas's height and the scrollbar follow the cards: sync() after the layout)
        // the cards are new widgets: zk_fit's fonts reach them on the next fit
        self.wrap.wl0.retain(|e| ui.ws.get(e.0).is_some());
    }

    /// tt_refresh's last part: cv height = max(110, min(h, tt_maxh)), the scrollbar while h > tt_maxh. True when
    /// something changed (lay out again).
    pub fn sync(&self, ui: &mut Ui) -> bool {
        let h = ui.ws[self.sc.inner].rh;
        let ch = 110.max(h.min(TT_MAXH));
        let need = h > TT_MAXH;
        let mut ch_ = false;
        if ui.ws[self.sc.cv].opt_h != ch {
            ui.ws[self.sc.cv].opt_h = ch;
            ch_ = true;
        }
        if need != ui.ws[self.sc.sb].grid.is_some() {
            ui.ws[self.sc.sb].grid = need.then(|| Grid { row: 0, col: 1, sticky: "ns", ..Default::default() });
            ch_ = true;
        }
        ch_
    }

    /// tt_card(it, day, now)
    fn card(&mut self, ui: &mut Ui, th: &Theme, f: &Faces, d: &TtData, i: usize, day: i64) {
        let it = &d.tt[i];
        let today = day == d.today;
        let ds = crate::plan::iso(day);
        let (h, m) = it["start"].as_str().unwrap_or("00:00").split_once(':').unwrap_or(("0", "0"));
        let st = day as f64 * 86400.0 + h.parse::<f64>().unwrap_or(0.0) * 3600.0 + m.parse::<f64>().unwrap_or(0.0) * 60.0;
        let mins = it["mins"].as_f64().unwrap_or(30.0);
        let en = st + mins * 60.0;
        let done = tt_done(it, &ds);
        let now = d.now_l;
        let (status, bg, accent) = if done {
            ("done".to_string(), th.donebg.clone(), th.good.clone())
        } else if today && st <= now && now < en {
            ("NOW".into(), th.nowbg.clone(), th.acc.clone())
        } else if today && en <= now {
            ("missed".into(), th.missbg.clone(), th.bad.clone())
        } else if today && st - now <= 3600.0 {
            let n = 1.max(((st - now) / 60.0).round_ties_even() as i64);
            (format!("in {n} min"), th.soonbg.clone(), th.accd.clone())
        } else {
            (String::new(), th.card.clone(), th.edge.clone())
        };
        let edge = if status.is_empty() { th.line.clone() } else { accent.clone() };
        let card = ui.round_frame(self.sc.inner, &bg, &edge, 12, "round", 4, &th.ink);
        if let Some(cv) = ui.ws[card].kids.first().copied()
            && let Kind::Canvas { card: Some(c), .. } = &mut ui.ws[cv].kind {
            c.notch = 5;
        }
        ui.pack(card, Pack::default().fill(Fill::X).pady(4, 4).padx(0, 8));
        let a = ui.frame(card, &accent);
        ui.ws[a].opt_w = 5;
        ui.pack(a, Pack::default().side(Side::Left).fill(Fill::Y).padx(8, 0).pady(7, 7));
        let tb = ui.frame(card, &bg);
        ui.pack(tb, Pack::default().side(Side::Left).padx(10, 6).pady(8, 8));
        let gold = if done { th.grey.clone() } else { th.gold.clone() };
        let l = lbl(ui, tb, it["start"].as_str().unwrap_or(""), &bg, &gold, Font::new("Consolas", 16, true));
        ui.pack(l, Pack::default().anchor(Anchor::W));
        let enm = (en.div_euclid(60.0) as i64).rem_euclid(1440);
        let l = lbl(ui, tb, &format!("until {:02}:{:02}", enm / 60, enm % 60), &bg, &th.grey, Font::new(&f.ui, 9, false));
        ui.pack(l, Pack::default().anchor(Anchor::W));
        let mid = ui.frame(card, &bg);
        ui.pack(mid, Pack::default().side(Side::Left).fill(Fill::X).expand().padx(6, 6));
        let fg = if done { th.grey.clone() } else { th.ink.clone() };
        let l = lbl(ui, mid, &clip(it["task"].as_str().unwrap_or(""), 30), &bg, &fg, Font::new(&f.ui, 12, true));
        ui.set_anchor(l, Anchor::W);
        if let Kind::Label { strike, .. } = &mut ui.ws[l].kind {
            *strike = done;
        }
        ui.pack(l, Pack::default().anchor(Anchor::W));
        let daily = it.get("daily").is_some_and(|v| v.as_bool().unwrap_or(!v.is_null()));
        let meta = fmt_hm(mins as i64) + if daily { "   \u{b7}   repeats daily" } else { "" };
        let l = lbl(ui, mid, &meta, &bg, &th.grey, Font::new(&f.ui, 9, false));
        ui.set_anchor(l, Anchor::W);
        ui.pack(l, Pack::default().anchor(Anchor::W));
        let right = ui.frame(card, &bg);
        ui.pack(right, Pack::default().side(Side::Right).padx(8, 8).pady(6, 6));
        if !status.is_empty() {
            let l = lbl(ui, right, &status, &accent, &th.onacc, Font::new(&f.ui, 8, true));
            ui.ws[l].padx = 7;
            ui.pack(l, Pack::default().anchor(Anchor::E).pady(0, 4));
        }
        let row = ui.frame(right, &bg);
        ui.pack(row, Pack::default().anchor(Anchor::E));
        let k = self.rows.len();
        self.rows.push(i);
        let can_start = today && !done;
        let b = ui.button(row, "Start", &format!("tt_start:{k}"), &th.start, &th.ink, &f.ui, 9, 8, 1, can_start);
        ui.pack(b, Pack::default().side(Side::Left).padx(2, 2));
        let b = tt_btn(ui, row, if done { "Undo" } else { "Done" }, &format!("tt_mark:{k}"), Some(&th.donebtn), None, 9, 8, 1, th, f);
        ui.pack(b, Pack::default().side(Side::Left).padx(2, 2));
        let b = tt_btn(ui, row, "Delete", &format!("tt_delete:{k}"), Some(&th.delbtn), None, 9, 8, 1, th, f);
        ui.pack(b, Pack::default().side(Side::Left).padx(2, 2));
    }

    /// zk_fit for the page; lays out at the window's size (pw, ph)
    #[allow(clippy::too_many_arguments)]
    pub fn fit(&mut self, ui: &mut Ui, fonts: &dyn Fonts, zk_f: f64, pw: i32, ph: i32) {
        zk_fonts(ui, fonts, self.pg, zk_f);
        ui.layout_at(fonts, pw, ph);
        if self.sync(ui) {
            ui.layout_at(fonts, pw, ph);
        }
        zk_wrap(ui, fonts, self.pg, &mut self.wrap, pw, ph);
        if self.sync(ui) {
            ui.layout_at(fonts, pw, ph);
        }
    }

    pub fn set_msg(&self, ui: &mut Ui, s: &str) {
        set_text(ui, self.msg, s);
    }
}

fn set_text(ui: &mut Ui, id: usize, s: &str) {
    if let Kind::Label { text, .. } = &mut ui.ws[id].kind {
        *text = s.into();
    }
}

/// TtData from the app as it is now
impl crate::Companion {
    pub fn tt_data(&mut self) -> TtData {
        let unix = crate::unix_now();
        let now_l = unix + crate::local_offset_pub() as f64;
        let today = now_l.floor().div_euclid(86400.0) as i64;
        let next = self.focus_item().map(|i| self.tt[i]["start"].as_str().unwrap_or("").to_string());
        TtData { today, now_l, tt: self.tt.clone(), next }
    }
}
