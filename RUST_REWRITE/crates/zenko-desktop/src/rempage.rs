//! The hub's Reminders page: zenko.py hub_build_reminders (the Write-an-ema card: the text box with its placeholder,
//! the When chips, "or at", Every day / Say it out loud, Hang it) and the shrine at night (ema_render: sky, stars,
//! moon, hills and a torii, today's planned study as paper lanterns on a rope, the ema rack with its roof, posts,
//! stone lantern and Zenko with her bubble, the plaques (ema_draw_plaque: cord, glow, wood, number, text, when,
//! 毎日 tag, fox seal, the selected plaque's pills), the fulfilled row and the foxfire wisps).

use crate::ema::{ema_left, ema_when};
use crate::hubwin::{canvas_of, hub_title_draw};
use crate::pagekit::{Faces, Scroller, WrapState, hub_card, hub_scroller, zk_fonts, zk_wrap};
use crate::session::clip;
use crate::theme::Theme;
use crate::tkcanvas::{Canvas, Photo};
use crate::tt::{tt_done, tt_today};
use crate::ui::{Fill, Font, Fonts, Kind, Pack, Side, Ui};
use serde_json::Value;
use std::collections::HashMap;
use std::f64::consts::PI;
use std::rc::Rc;
use zenko_art::palette::mixhex;
use zenko_sound::mt::PyRandom;

pub const TITLE: &str = "Reminders";
pub const SUB: &str = "\u{7d75}\u{99ac} \u{b7} Write a wish plaque and hang it on the shrine rack. Zenko says it out loud \
                       when it's time. Your planned study glows in the lanterns above.";
pub const PLACEHOLDER: &str = "What should I remind you of?  (e.g. call mum at 6pm, take vitamins every day at 8)";
pub const CHIPS: [(&str, &str); 6] = [("auto", "From my words"), ("none", "Anytime"), ("30", "In 30 min"), ("60", "In 1 hour"),
                                      ("tonight", "Tonight 8 PM"), ("tmrw", "Tomorrow 7 AM")];
const INK_E: &str = "#3a2414";
const WOOD_D: &str = "#3b2418";
const WOOD_L: &str = "#6b4329";
const VERM: &str = "#c8372d";

/// What the page shows, from the app (Companion::rem_data)
#[derive(Clone, Debug, Default)]
pub struct RemData {
    /// time.time() and the local wall clock (seconds), today's local day number
    pub now: f64,
    pub now_l: f64,
    pub today: i64,
    pub tt: Vec<Value>,
    /// ema_active(), ema_done_list()[:8]
    pub act: Vec<Value>,
    pub done: Vec<Value>,
    /// _jp_font, _font_pick(Yu Mincho, MS Mincho, Georgia, Times New Roman)
    pub jp: String,
    pub serif: String,
    pub aloud: bool,
    /// HD_ART["on"]
    pub hd: bool,
    /// small_fox(1), small_fox(2)
    pub fox: Vec<Rc<Photo>>,
}

/// what a spot on the night canvas does: ("plaque" | "done" | "edit" | "snooze" | "remove" | "undo" | "clear_done", id)
pub type Hit = ([f64; 4], String, String);

pub struct RemPage {
    pub pg: usize,
    pub title: usize,
    pub sc: Scroller,
    pub entry: usize,
    pub at_entry: usize,
    pub every: usize,
    pub aloud: usize,
    pub msg: usize,
    pub chips: Vec<(String, usize)>,
    /// pick["v"]
    pub pick: String,
    /// the night canvas
    pub cv: usize,
    /// self._ema_sel, ema_ui["hover"]
    pub sel: Option<String>,
    pub hover: Option<String>,
    pub hits: Vec<Hit>,
    /// (two image items, x, y, phase, speed)
    pub wisps: Vec<(u32, u32, f64, f64, f64, f64)>,
    /// the width the canvas was drawn for (0: never)
    pub drawn_w: i32,
    pub wrap: WrapState,
    seig: Option<(String, Rc<Photo>)>,
    imgs: HashMap<String, Rc<Photo>>,
}

fn truthy(v: &Value) -> bool {
    match v {
        Value::Null | Value::Bool(false) => false,
        Value::Number(n) => n.as_f64() != Some(0.0),
        Value::String(s) => !s.is_empty(),
        _ => true,
    }
}

fn at_of(e: &Value) -> Option<f64> {
    e["at"].as_f64().filter(|a| *a != 0.0)
}

/// Python's int() of a float (towards zero), kept as a float for coordinates
fn ti(v: f64) -> f64 {
    v.trunc()
}

fn hex_rgb(c: &str) -> (i64, i64, i64) {
    let h = c.trim_start_matches('#');
    let p = |i: usize| i64::from_str_radix(h.get(i..i + 2).unwrap_or("00"), 16).unwrap_or(0);
    (p(0), p(2), p(4))
}

/// _rgb_mix(a, b, t)
fn rgb_mix(a: (f64, f64, f64), b: (f64, f64, f64), t: f64) -> (f64, f64, f64) {
    let t = t.clamp(0.0, 1.0);
    (ti(a.0 + (b.0 - a.0) * t), ti(a.1 + (b.1 - a.1) * t), ti(a.2 + (b.2 - a.2) * t))
}

type Art = Box<dyn Fn(i32, i32) -> Option<[u8; 4]>>;

fn px(c: (f64, f64, f64)) -> Option<[u8; 4]> {
    Some([c.0 as u8, c.1 as u8, c.2 as u8, 255])
}

/// px_photo(w, h, fn, zoom): zenko21's HD upscale for zoom >= 2 (but 5 and 7), else whole-step zoom
pub fn px_photo(w: i32, h: i32, f: &Art, zoom: usize, hd: bool) -> Rc<Photo> {
    let mut v = Vec::with_capacity((w * h).max(0) as usize);
    for y in 0..h {
        for x in 0..w {
            v.push(f(x, y).unwrap_or([0, 0, 0, 0]));
        }
    }
    let img = zenko_art::raster::Image { w: w as usize, h: h as usize, px: v };
    let im = if hd && zoom >= 2 && zoom != 5 && zoom != 7 {
        zenko_art::raster::zk_upscale(&img, zoom)
    } else if zoom > 1 {
        zenko_art::raster::nearest(&img, zoom)
    } else {
        img
    };
    Rc::new(Photo { w: im.w, h: im.h, px: im.px })
}

/// glow_art(r, col, strength): a soft round glow with real transparency
pub fn glow_art(r: i32, col: (i64, i64, i64), strength: f64) -> Art {
    let r = r as f64;
    Box::new(move |x, y| {
        let d = ((x as f64 + 0.5 - r).powi(2) + (y as f64 + 0.5 - r).powi(2)).sqrt() / r;
        if d >= 1.0 {
            return None;
        }
        let a = (255.0 * strength * (1.0 - d).powi(2)) as i64;
        if a > 2 { Some([col.0 as u8, col.1 as u8, col.2 as u8, a as u8]) } else { None }
    })
}

/// lantern_art(w, h): a red paper chochin lantern (ribbed), black lacquer caps
pub fn lantern_art(w: i32, h: i32) -> Art {
    Box::new(move |x, y| {
        let (wf, hf) = (w as f64, h as f64);
        let cap = 2.max(h / 9);
        if y < cap || y >= h - cap {
            if (x as f64 - wf / 2.0).abs() > wf * 0.36 {
                return None;
            }
            return px(if (x + y) % 5 != 0 { (30.0, 22.0, 22.0) } else { (70.0, 58.0, 50.0) });
        }
        let u = (x as f64 + 0.5 - wf / 2.0) / (wf / 2.0);
        let v = (y as f64 + 0.5 - hf / 2.0) / ((hf - 2.0 * cap as f64) / 2.0);
        let bulge = 1.0 - 0.18 * v * v;
        if u.abs() > bulge {
            return None;
        }
        let rib = (y - cap) % 4 == 0;
        let lit = 1.0 - u.abs() * 0.55;
        let mut c = rgb_mix((120.0, 16.0, 24.0), (238.0, 64.0, 52.0), lit);
        c = rgb_mix(c, (255.0, 190.0, 120.0), 0.35 * (1.0 - u.abs() * 2.0).max(0.0) * (1.0 - v.abs()));
        if rib {
            c = rgb_mix(c, (60.0, 10.0, 14.0), 0.45);
        }
        if u.abs() > bulge - 0.12 {
            c = rgb_mix(c, (40.0, 6.0, 10.0), 0.4);
        }
        px(c)
    })
}

/// ema_plaque_wood(w, h, seed): a wooden ema (pentagon top, warm hinoki grain, darker edge)
pub fn ema_plaque_wood(w: i32, h: i32, seed: i64) -> Art {
    let mut rnd = PyRandom::new(seed.unsigned_abs());
    let (wf, hf) = (w as f64, h as f64);
    let mut knots = vec![];
    for _ in 0..2 {
        let kx = rnd.uniform(0.15, 0.85) * wf;
        let ky = rnd.uniform(0.35, 0.9) * hf;
        knots.push((kx, ky));
    }
    let ph = rnd.uniform(0.0, 6.28);
    let roof = hf * 0.24;
    Box::new(move |x, y| {
        let (xf, yf) = (x as f64, y as f64);
        let cx = (xf + 0.5 - wf / 2.0).abs();
        if yf < roof * (cx / (wf / 2.0)) {
            return None;
        }
        let edge = xf.min(wf - 1.0 - xf).min(hf - 1.0 - yf).min((yf - roof * (cx / (wf / 2.0))) * 1.6);
        if edge < 1.2 {
            return px((92.0, 56.0, 30.0));
        }
        if edge < 2.6 {
            return px((176.0, 120.0, 66.0));
        }
        let g = (yf * 0.55 + (xf * 0.09 + ph).sin() * 2.2).sin() * 0.5 + 0.5;
        let mut c = rgb_mix((236.0, 204.0, 152.0), (214.0, 172.0, 112.0), g * 0.55);
        for &(kx, ky) in &knots {
            let d = ((xf - kx) * 0.8).hypot((yf - ky) * 1.6);
            if d < 6.0 {
                c = rgb_mix(c, (170.0, 116.0, 64.0), (1.0 - d / 6.0) * 0.45);
            }
        }
        let light = 1.0 - (yf / hf) * 0.12;
        px((ti(c.0 * light), ti(c.1 * light), ti(c.2 * light)))
    })
}

/// ema_stone_lantern(w, h): a tōrō stone lantern with a warm light inside
pub fn ema_stone_lantern(w: i32, h: i32) -> Art {
    Box::new(move |x, y| {
        let (wf, hf, yf) = (w as f64, h as f64, y as f64);
        let u = x as f64 + 0.5 - wf / 2.0;
        let c = if yf < hf * 0.12 {
            if u.abs() < (yf / (hf * 0.12)) * wf * 0.5 + 1.0 { (120.0, 118.0, 128.0) } else { return None }
        } else if yf < hf * 0.18 {
            if u.abs() < wf * 0.5 { (96.0, 94.0, 104.0) } else { return None }
        } else if yf < hf * 0.42 {
            if u.abs() < wf * 0.36 {
                if u.abs() < wf * 0.22 && hf * 0.22 < yf && yf < hf * 0.38 {
                    rgb_mix((255.0, 214.0, 130.0), (255.0, 150.0, 60.0), u.abs() / (wf * 0.22))
                } else {
                    (132.0, 130.0, 140.0)
                }
            } else {
                return None;
            }
        } else if yf < hf * 0.48 {
            if u.abs() < wf * 0.44 { (104.0, 102.0, 112.0) } else { return None }
        } else if yf < hf * 0.9 {
            if u.abs() < wf * 0.14 + (yf - hf * 0.48) * 0.08 { (118.0, 116.0, 126.0) } else { return None }
        } else if u.abs() < wf * 0.42 {
            (90.0, 88.0, 98.0)
        } else {
            return None;
        };
        px(c)
    })
}

/// rbox(cv, x1, y1, x2, y2, r, fill, outline, width=1, tags, shape): a pixel-rounded polygon (joinstyle miter)
#[allow(clippy::too_many_arguments)]
fn rbox(cv: &mut Canvas, x1: f64, y1: f64, x2: f64, y2: f64, r: i32, fill: &str, outline: &str, tags: &[&str],
        shape: &str) -> u32 {
    let pts = crate::hubside::round_pts_flat(x1, y1, x2, y2, r, 2, shape);
    cv.poly(pts).fill(fill).outline(outline).join("miter").tags(tags);
    cv.last_id()
}

/// strftime("%I:%M %p").lstrip("0") of "HH:MM"
fn clock12(hm: &str) -> String {
    let (h, m) = hm.split_once(':').unwrap_or(("0", "0"));
    let (h, m) = (h.parse::<i64>().unwrap_or(0), m.parse::<i64>().unwrap_or(0));
    let h12 = if h % 12 == 0 { 12 } else { h % 12 };
    format!("{h12}:{m:02} {}", if h < 12 { "AM" } else { "PM" })
}

fn day_of(d: &RemData, ts: f64) -> i64 {
    (ts + (d.now_l - d.now)).div_euclid(86400.0) as i64
}

/// PixelButton.config(bg, fg)
fn set_btn(ui: &mut Ui, id: usize, bg: &str, fg: &str) {
    if let Kind::Button { color, fg: f, .. } = &mut ui.ws[id].kind {
        *color = bg.into();
        *f = fg.into();
    }
}

impl RemPage {
    /// hub_build_reminders(pg)
    pub fn build(ui: &mut Ui, pg: usize, th: &Theme, f: &Faces, cm10: i32, d: &RemData) -> RemPage {
        let title = ui.canvas(pg, &th.banner, cm10, 96);
        canvas_of(ui, title);
        ui.pack(title, Pack::default().fill(Fill::X));
        let bg = th.card.clone();
        let sc = hub_scroller(ui, pg, th, cm10);
        let top = ui.frame(sc.inner, &th.paper);
        ui.pack(top, Pack::default().fill(Fill::X).padx(24, 24).pady(12, 0));
        let card = hub_card(ui, top, Some("Write an ema"), "round", th, f);
        ui.pack(card, Pack::default().fill(Fill::X));
        let r1 = ui.frame(card, &bg);
        ui.pack(r1, Pack::default().fill(Fill::X).padx(16, 16).pady(2, 6));
        let entry = ui.entry(r1, 20, Font::new(&f.ui, 12, false), "", &th.field);
        if let Kind::Entry { e, .. } = &mut ui.ws[entry].kind {
            e.ph = Some(PLACEHOLDER.into());
        }
        ui.pack(entry, Pack::default().side(Side::Left).fill(Fill::X).expand().ipady(5));
        let r2 = ui.frame(card, &bg);
        ui.pack(r2, Pack::default().fill(Fill::X).padx(16, 16).pady(0, 4));
        let l = ui.label(r2, "When", &bg, &th.gold, Font::new(&f.ui, 9, true));
        ui.pack(l, Pack::default().side(Side::Left).padx(0, 8));
        let mut chips = vec![];
        for (k, lab) in CHIPS {
            let b = ui.button(r2, lab, &format!("rem_pick:{k}"), &th.start, &th.ink, &f.ui, 9, 9, 2, true);
            ui.pack(b, Pack::default().side(Side::Left).padx(0, 5));
            chips.push((k.to_string(), b));
        }
        let l = ui.label(r2, "  or at", &bg, &th.grey, Font::new(&f.ui, 9, false));
        ui.pack(l, Pack::default().side(Side::Left));
        let at_entry = ui.entry(r2, 8, Font::new(&f.ui, 10, false), "", &th.field);
        ui.pack(at_entry, Pack::default().side(Side::Left).padx(6, 0).ipady(2));
        let r3 = ui.frame(card, &bg);
        ui.pack(r3, Pack::default().fill(Fill::X).padx(16, 16).pady(2, 12));
        let every = ui.check(r3, "Every day", &bg, &th.ink, Font::new(&f.ui, 10, false), false, &th.field, "rem_every", 0, 2);
        ui.pack(every, Pack::default().side(Side::Left));
        let aloud = ui.check(r3, "Say reminders and planned study out loud", &bg, &th.ink, Font::new(&f.ui, 10, false),
                             d.aloud, &th.field, "rem_aloud", 0, 2);
        ui.pack(aloud, Pack::default().side(Side::Left).padx(14, 0));
        let msg = ui.label(r3, "", &bg, &th.gold, Font::new(&f.ui, 9, true));
        ui.set_anchor(msg, crate::ui::Anchor::W);
        let b = ui.button(r3, "Hang it", "rem_hang", &th.acc, &th.onacc, &f.ui, 11, 18, 3, true);
        ui.pack(b, Pack::default().side(Side::Right));
        ui.pack(msg, Pack::default().side(Side::Right).padx(10, 10));
        // the shrine at night
        let mat = ui.frame(sc.inner, &mixhex(&th.line, &th.gold, 0.25));
        (ui.ws[mat].padx, ui.ws[mat].pady) = (1, 1);
        ui.pack(mat, Pack::default().fill(Fill::X).padx(24, 24).pady(12, 16));
        let cv = ui.canvas(mat, &th.night, cm10, 520);
        canvas_of(ui, cv);
        ui.pack(cv, Pack::default().fill(Fill::X));
        let mut p = RemPage { pg, title, sc, entry, at_entry, every, aloud, msg, chips, pick: String::new(), cv, sel: None,
                              hover: None, hits: vec![], wisps: vec![], drawn_w: 0, wrap: WrapState::default(), seig: None,
                              imgs: HashMap::new() };
        p.choose(ui, th, "auto");
        p
    }

    /// choose(k): the picked chip in ORANGE
    pub fn choose(&mut self, ui: &mut Ui, th: &Theme, k: &str) {
        self.pick = k.into();
        for (kk, b) in self.chips.clone() {
            if kk == k {
                set_btn(ui, b, &th.acc, &th.onacc);
            } else {
                set_btn(ui, b, &th.start, &th.ink);
            }
        }
    }

    /// set the message beside Hang it
    pub fn set_msg(&mut self, ui: &mut Ui, s: &str) {
        if let Kind::Label { text, .. } = &mut ui.ws[self.msg].kind {
            *text = s.into();
        }
    }

    fn img(&mut self, key: String, make: impl FnOnce() -> Rc<Photo>) -> Rc<Photo> {
        if !self.imgs.contains_key(&key) {
            if self.imgs.len() > 160 {
                self.imgs.clear();
            }
            self.imgs.insert(key.clone(), make());
        }
        self.imgs[&key].clone()
    }

    /// ema_render: the night canvas for its width now; true when its height changed (lay the page out again)
    pub fn render(&mut self, ui: &mut Ui, th: &Theme, f: &Faces, fonts: &dyn Fonts, zk_f: f64, d: &RemData) -> bool {
        let w_px = ui.ws[self.cv].w;
        if w_px < 120 {
            return false;
        }
        self.drawn_w = w_px;
        let wf = w_px as f64;
        let fk = zk_f.clamp(0.85, 1.35);
        let fi = |v: f64| ti(v * fk);
        let (nts, now_l) = (d.now, d.now_l);
        let hd = d.hd;
        let mut cv = Canvas::default();
        self.hits.clear();
        let (ink, verm) = (INK_E, VERM);
        let act = &d.act;
        let done = &d.done;
        let (pw, ph) = (fi(176.0), fi(150.0));
        let gap = fi(22.0);
        let (rack_x0, rack_x1) = (fi(70.0), wf - fi(70.0));
        let cols = 1.0f64.max(((rack_x1 - rack_x0 + gap - fi(28.0)) / (pw + gap)).floor());
        let rows = if act.is_empty() { 1.0 } else { 1.0f64.max((act.len() as f64 / cols).ceil()) };
        let lan_top = fi(18.0);
        let lan_h = fi(178.0);
        let rack_top = lan_top + lan_h + fi(30.0);
        let beam_gap = ph + fi(74.0);
        let rack_bot = rack_top + fi(34.0) + rows * beam_gap + fi(56.0);
        let done_top = rack_bot + fi(40.0);
        let (dw, dh) = (fi(112.0), fi(96.0));
        let dcols = 1.0f64.max(((wf - fi(80.0)) / (dw + fi(14.0))).floor());
        let done_h = if done.is_empty() { 0.0 } else { fi(40.0) + (done.len() as f64 / dcols).ceil() * (dh + fi(26.0)) };
        let total = fi(380.0).max(done_top + done_h + fi(40.0));
        let changed = (ui.ws[self.cv].opt_h as f64 - total).abs() > 1.0;
        if changed {
            ui.ws[self.cv].opt_h = total as i32;
        }
        let vh = total;
        // ---- the night sky, mountains and a torii in the mist
        let top_c = mixhex(&th.night, "#000000", 0.25);
        let low_c = mixhex(&th.night, &th.banner, 0.9);
        let mut y = 0.0;
        while y < total {
            cv.rect(0.0, y, wf, y + 6.0).fill(&mixhex(&top_c, &low_c, (y / vh.max(1.0)).min(1.0))).outline("");
            y += 6.0;
        }
        let mut rnd = PyRandom::new(7);
        for _ in 0..(wf * total / 9000.0) as i64 {
            let sx = rnd.uniform(0.0, wf);
            let sy = rnd.uniform(0.0, total * 0.9);
            let s_ = *rnd.choice(&[1.0, 1.0, 2.0]);
            let t = rnd.uniform(0.25, 0.75);
            cv.rect(sx, sy, sx + s_, sy + s_).fill(&mixhex(&th.night, &th.ink, t)).outline("");
        }
        let (mx, my, mr) = (wf - fi(90.0), fi(46.0), fi(26.0));
        let moon = self.img(format!("moon {mr}"), || px_photo(mr as i32 * 2, mr as i32 * 2,
                                                                &glow_art(mr as i32, (255, 236, 190), 0.35), 1, hd));
        cv.image(mx, my, moon);
        cv.oval(mx - mr * 0.55, my - mr * 0.55, mx + mr * 0.55, my + mr * 0.55).fill("#f6e7c1").outline("");
        cv.oval(mx - mr * 0.25, my - mr * 0.6, mx + mr * 0.75, my + mr * 0.45).fill(&top_c).outline("");
        let hill = mixhex(&th.night, "#000000", 0.35);
        let mut pts = vec![0.0, vh];
        let mut x = 0;
        while x < w_px + 20 {
            let xf = x as f64;
            let wave = 0.5 + 0.5 * (xf * 0.011 + 1.3).sin() + 0.3 * (xf * 0.037).sin();
            pts.extend([xf, vh - fi(70.0) - ti(26.0 * fk * wave)]);
            x += 20;
        }
        pts.extend([wf, vh]);
        cv.poly(pts).fill(&mixhex(&hill, &low_c, 0.35)).outline("");
        let (tx, ty) = (ti(wf * 0.12), vh - fi(84.0));
        let tc = mixhex(verm, &th.night, 0.45);
        for (a, b, c, e) in [(-24.0, -34.0, 24.0, -30.0), (-19.0, -25.0, 19.0, -23.0), (-15.0, -30.0, -12.0, 0.0),
                             (12.0, -30.0, 15.0, 0.0)] {
            cv.rect(tx + a * fk, ty + b * fk, tx + c * fk, ty + e * fk).fill(&tc).outline("");
        }
        // ---- 今日の予定: planned study as paper lanterns on a rope
        let today_s = crate::plan::iso(d.today);
        let items: Vec<&Value> = tt_today(&d.tt, &today_s).into_iter().map(|i| &d.tt[i]).collect();
        let jp = if d.jp.is_empty() { f.ui.clone() } else { d.jp.clone() };
        let ui_f = |n: f64, b: bool| Font::new(&f.ui, n as i32, b);
        cv.text(fi(28.0), lan_top, "\u{4eca}\u{65e5}\u{306e}\u{4e88}\u{5b9a}").anchor("nw").fill(&th.gold)
            .font(Font::new(&jp, fi(11.0) as i32, true));
        cv.text(fi(28.0) + fi(92.0), lan_top + fi(2.0), "Planned study today  \u{b7}  from your timetable").anchor("nw")
            .fill(&th.grey).font(ui_f(fi(9.0), false));
        let ry = lan_top + fi(34.0);
        let mut rope = vec![];
        for i in 0..41 {
            let i = i as f64;
            rope.extend([fi(16.0) + (wf - fi(32.0)) * i / 40.0, ry + (PI * i / 40.0).sin() * fi(14.0)]);
        }
        cv.line(rope).fill("#7a2a20").width(2.0f64.max(fi(2.0))).smooth();
        let (lw_, lh_) = (14.0, 20.0);
        let lz = 2.0f64.max((3.0 * fk).round_ties_even());
        let lan = self.img(format!("lan {lz}"), || px_photo(14, 20, &lantern_art(14, 20), lz as usize, hd));
        let glow = self.img(format!("lglow {lz}"), || px_photo(30, 30, &glow_art(15, (255, 170, 80), 0.45), lz as usize, hd));
        let nshow = (((wf - fi(60.0)) / fi(132.0)).floor() as i64).min(8).max(1) as usize;
        let shown = &items[..items.len().min(nshow)];
        let win = |it: &Value| {
            let (h, m) = it["start"].as_str().unwrap_or("0:0").split_once(':').unwrap_or(("0", "0"));
            let st = d.today as f64 * 86400.0 + h.parse::<f64>().unwrap_or(0.0) * 3600.0 + m.parse::<f64>().unwrap_or(0.0) * 60.0;
            (st, st + it["mins"].as_f64().unwrap_or(0.0) * 60.0)
        };
        let nxt = items.iter().position(|it| !tt_done(it, &today_s) && win(it).1 > now_l);
        if shown.is_empty() {
            let x = (w_px / 2) as f64;
            let yy = ry + fi(14.0);
            cv.line(vec![x, yy, x, yy + fi(10.0)]).fill("#3a2a20").width(2.0);
            cv.image(x, yy + fi(10.0), lan.clone()).anchor("n");
            cv.rect(x - lw_ * lz / 2.0, yy + fi(10.0), x + lw_ * lz / 2.0, yy + fi(10.0) + lh_ * lz).fill(&th.night)
                .outline("").stipple("gray50");
            cv.text(x, yy + fi(10.0) + lh_ * lz + fi(10.0), "No study planned yet today").fill(&th.grey)
                .font(ui_f(fi(9.0), true)).anchor("n");
            cv.text(x, yy + fi(10.0) + lh_ * lz + fi(28.0), "Say \u{201c}plan bio at 4pm for 45 min\u{201d}")
                .fill(&mixhex(&th.grey, &th.night, 0.3)).font(ui_f(fi(8.0), false)).anchor("n");
        }
        for (k, it) in shown.iter().enumerate() {
            let x = ti(wf * (k + 1) as f64 / (shown.len() + 1) as f64);
            let sag = (PI * (x - 16.0 * fk) / (wf - 32.0 * fk)).sin() * fi(14.0);
            let yy = ry + sag;
            cv.line(vec![x, yy, x, yy + fi(10.0)]).fill("#3a2a20").width(2.0);
            let ly = yy + fi(10.0);
            let isdone = tt_done(it, &today_s);
            let is_next = nxt == Some(k);
            if is_next {
                cv.image(x, ly + lh_ * lz / 2.0, glow.clone());
            }
            cv.image(x, ly, lan.clone()).anchor("n");
            if isdone {
                cv.rect(x - lw_ * lz / 2.0, ly, x + lw_ * lz / 2.0, ly + lh_ * lz).fill(&th.night).outline("").stipple("gray50");
            }
            let st = clock12(it["start"].as_str().unwrap_or("0:00"));
            cv.text(x, ly + lh_ * lz + fi(8.0), &st).fill(if isdone { &th.grey } else { &th.gold })
                .font(ui_f(fi(10.0), true)).anchor("n");
            let t = clip(it["task"].as_str().unwrap_or(""), 22) + if isdone { "  \u{2713}" } else { "" };
            cv.text(x, ly + lh_ * lz + fi(26.0), &t).fill(if isdone { &th.grey } else { &th.ink }).wrap(fi(124.0))
                .justify("center").font(ui_f(fi(9.0), false)).anchor("n");
            if is_next {
                let (s0, _) = win(it);
                let lab = if s0 <= now_l { "now".to_string() } else { ema_left(s0, now_l) };
                cv.text(x, ly + lh_ * lz + fi(44.0), &format!("next \u{b7} {lab}")).fill(&mixhex(&th.gold, "#ffffff", 0.3))
                    .font(ui_f(fi(8.0), true)).anchor("n");
            }
        }
        if items.len() > shown.len() {
            cv.text(wf - fi(24.0), ry + fi(30.0), &format!("+{} more", items.len() - shown.len())).anchor("e")
                .fill(&th.grey).font(ui_f(fi(8.0), true));
        }
        // ---- 絵馬掛け: the ema rack
        cv.text(fi(28.0), rack_top - fi(28.0), "\u{7d75}\u{99ac}\u{639b}\u{3051}").anchor("nw").fill(&th.gold)
            .font(Font::new(&jp, fi(11.0) as i32, true));
        let n_due = act.iter().filter(|e| at_of(e).is_some_and(|a| day_of(d, a) == d.today)).count();
        let sub = format!("Your reminders  \u{b7}  {} hanging{}", act.len(),
                          if n_due > 0 { format!("  \u{b7}  {n_due} today") } else { String::new() });
        cv.text(fi(28.0) + fi(78.0), rack_top - fi(26.0), &sub).anchor("nw").fill(&th.grey).font(ui_f(fi(9.0), false));
        let (rx0, rx1) = (rack_x0 - fi(26.0), rack_x1 + fi(26.0));
        let roof_y = rack_top;
        cv.poly(vec![rx0 - fi(18.0), roof_y + fi(16.0), rx0 + fi(20.0), roof_y - fi(6.0), rx1 - fi(20.0), roof_y - fi(6.0),
                     rx1 + fi(18.0), roof_y + fi(16.0)]).fill("#2a2230").outline("");
        let (span_r, stp) = (ti(rx1 - rx0) as i64, fi(10.0) as i64);
        let mut k = 0;
        while k < span_r {
            let kf = k as f64;
            cv.line(vec![rx0 + kf, roof_y - fi(4.0), rx0 + kf - fi(4.0), roof_y + fi(14.0)]).fill("#3a3142");
            k += stp.max(1);
        }
        cv.line(vec![rx0 + fi(20.0), roof_y - fi(6.0), rx1 - fi(20.0), roof_y - fi(6.0)]).fill(&th.gold)
            .width(2.0f64.max(fi(2.0)));
        cv.line(vec![rx0 - fi(18.0), roof_y + fi(16.0), rx1 + fi(18.0), roof_y + fi(16.0)]).fill("#15101a")
            .width(2.0f64.max(fi(3.0)));
        let post_w = fi(16.0);
        for px_ in [rack_x0 - fi(16.0), rack_x1 + fi(16.0)] {
            cv.rect(px_ - post_w / 2.0, roof_y + fi(16.0), px_ + post_w / 2.0, rack_bot + fi(18.0)).fill(WOOD_D).outline("");
            cv.rect(px_ - post_w / 2.0, roof_y + fi(16.0), px_ - post_w / 2.0 + fi(4.0), rack_bot + fi(18.0)).fill(WOOD_L)
                .outline("");
            cv.rect(px_ - post_w * 0.8, rack_bot + fi(12.0), px_ + post_w * 0.8, rack_bot + fi(22.0)).fill("#6e6a75").outline("");
        }
        // stone lantern + Zenko at the foot of the rack
        let sl = self.img(format!("toro {lz}"), || px_photo(16, 30, &ema_stone_lantern(16, 30), lz as usize, hd));
        cv.image(rack_x1 + fi(48.0), rack_bot + fi(22.0), sl).anchor("s");
        let tg = self.img(format!("tglow {lz}"), || px_photo(24, 24, &glow_art(12, (255, 190, 110), 0.4), lz as usize, hd));
        cv.image(rack_x1 + fi(48.0), rack_bot + fi(22.0) - 30.0 * lz * 0.7, tg);
        let fz = if fk < 1.25 { 0 } else { 1 };
        if let Some(fox) = d.fox.get(fz).or(d.fox.first()).cloned() {
            let (fw, fh) = (fox.w as f64, fox.h as f64);
            let (fx_, fy_) = (rack_x1 - fi(30.0) - fw / 2.0, rack_bot + fi(16.0));
            cv.image(fx_, fy_, fox).anchor("s");
            let n_soon = act.iter().filter(|e| at_of(e).is_some_and(|a| 0.0 < a - nts && a - nts < 3.0 * 3600.0)).count();
            let n_rang = act.iter().filter(|e| truthy(&e["rang"])).count();
            let line = if n_rang > 0 {
                format!("{n_rang} rang! Tap it when it's done~")
            } else if n_soon > 0 {
                format!("{n_soon} coming up soon~ I'll say it out loud.")
            } else if !act.is_empty() {
                "I'm watching your ema~".into()
            } else {
                "Write me an ema~".into()
            };
            cv.text(fx_ - fw / 2.0 - fi(16.0), fy_ - fh * 0.62, &line).anchor("e").fill(&th.ink).font(ui_f(fi(9.0), true));
            let tb = cv.last_id();
            let b_ = crate::canvasdraw::text_bbox(cv.items.last().unwrap(), fonts).map(|v| v as f64);
            let bub = rbox(&mut cv, b_[0] - 10.0 * fk, b_[1] - 6.0 * fk, b_[2] + 10.0 * fk, b_[3] + 6.0 * fk, 10, &th.bub,
                           &mixhex(&th.gold, &th.night, 0.4), &[], "round");
            let mid = (b_[1] + b_[3]) / 2.0;
            cv.poly(vec![b_[2] + 10.0 * fk, mid - 4.0 * fk, b_[2] + 18.0 * fk, mid, b_[2] + 10.0 * fk, mid + 4.0 * fk])
                .fill(&th.bub).outline("");
            cv.tag_raise_above(&tb.to_string(), &bub.to_string());
        }
        // beams + plaques
        for r in 0..rows as i64 {
            let by = rack_top + fi(34.0) + r as f64 * beam_gap;
            cv.rect(rack_x0 - fi(26.0), by, rack_x1 + fi(26.0), by + fi(10.0)).fill(WOOD_D).outline("");
            cv.line(vec![rack_x0 - fi(26.0), by + 1.0, rack_x1 + fi(26.0), by + 1.0]).fill(WOOD_L).width(2.0);
        }
        if act.is_empty() {
            let cy = rack_top + fi(34.0) + fi(30.0);
            cv.text((rack_x0 + rack_x1) / 2.0, cy + ph / 2.0, "No ema hanging yet.\nWrite one above, or tell me in chat: \
                    \u{201c}remind me to drink water in 30 min\u{201d}.").fill(&th.grey).justify("center")
                .font(ui_f(fi(10.0), false)).wrap(fi(360.0));
        }
        let span = cols * pw + (cols - 1.0) * gap;
        let x_start = rack_x0 + (rack_x1 - rack_x0 - span) / 2.0;
        let serif = if d.serif.is_empty() { f.ui.clone() } else { d.serif.clone() };
        let fnts = (f.ui.clone(), serif, jp.clone());
        for (i, e) in act.iter().enumerate() {
            let (r, c) = ((i as f64 / cols).floor(), (i as f64 % cols));
            let by = rack_top + fi(34.0) + r * beam_gap;
            let x = x_start + c * (pw + gap);
            let y = by + fi(24.0);
            self.plaque(&mut cv, th, fonts, &fnts, d, e, x, y, pw, ph, fk, ink, verm, Some(by + fi(10.0)), Some(i + 1), false);
        }
        // fulfilled row
        if !done.is_empty() {
            cv.text(fi(28.0), done_top, "\u{53f6}\u{3063}\u{305f}").anchor("nw").fill(&th.gold)
                .font(Font::new(&jp, fi(11.0) as i32, true));
            cv.text(fi(28.0) + fi(64.0), done_top + fi(2.0), "Fulfilled  \u{b7}  click one to bring it back").anchor("nw")
                .fill(&th.grey).font(ui_f(fi(9.0), false));
            let bx = wf - fi(30.0);
            cv.text(bx, done_top + fi(2.0), "Clear fulfilled").anchor("ne").fill(&th.gold).font(ui_f(fi(9.0), true))
                .underline();
            let bb = crate::canvasdraw::text_bbox(cv.items.last().unwrap(), fonts).map(|v| v as f64);
            self.hits.push((bb, "clear_done".into(), String::new()));
            for (i, e) in done.iter().enumerate() {
                let (r, c) = ((i as f64 / dcols).floor(), (i as f64 % dcols));
                let x = fi(40.0) + c * (dw + fi(14.0));
                let y = done_top + fi(34.0) + r * (dh + fi(26.0));
                self.plaque(&mut cv, th, fonts, &fnts, d, e, x, y, dw, dh, fk * 0.72, ink, verm, None, None, true);
            }
        }
        // foxfire
        self.wisps.clear();
        let mut rw = PyRandom::new(((nts as i64).div_euclid(600)).unsigned_abs());
        let gcol = hex_rgb(&th.glow);
        for _ in 0..7 {
            let wx = rw.uniform(30.0, wf - 30.0);
            let wy = rw.uniform(lan_top + lan_h * 0.6, total.min(vh) - 40.0);
            let gr = ti(*rw.choice(&[7.0, 9.0, 11.0]) * fk) as i32;
            let g1 = self.img(format!("wisp {gr} {}", th.glow), || px_photo(gr * 2, gr * 2, &glow_art(gr, gcol, 0.75), 2, hd));
            cv.image(wx, wy, g1).tags(&["wisp"]);
            let o1 = cv.last_id();
            let gc2 = ((gcol.0 + 90).min(255), (gcol.1 + 90).min(255), (gcol.2 + 90).min(255));
            let g2 = self.img(format!("wispc {}", th.glow), || px_photo(4, 4, &glow_art(2, gc2, 1.0), 2, hd));
            cv.image(wx, wy, g2).tags(&["wisp"]);
            let o2 = cv.last_id();
            let ph_ = rw.uniform(0.0, 6.28);
            let sp = rw.uniform(0.4, 1.0);
            self.wisps.push((o1, o2, wx, wy, ph_, sp));
        }
        cv.tag_raise("wisp");
        cv.tag_raise("plaque");
        cv.tag_raise("pill");
        *canvas_of(ui, self.cv) = cv;
        changed
    }

    /// ema_anim: the wisps drift (t = time.monotonic())
    pub fn anim(&self, ui: &mut Ui, t: f64) {
        let cv = canvas_of(ui, self.cv);
        for &(o1, o2, wx, wy, ph_, sp) in &self.wisps {
            let dx = (t * 0.6 * sp + ph_).sin() * 14.0;
            let dy = (t * 0.45 * sp + ph_ * 1.7).cos() * 9.0;
            for o in [o1, o2] {
                if let Some(it) = cv.get(o) {
                    it.coords = vec![wx + dx, wy + dy];
                }
            }
        }
    }

    /// ema_draw_plaque
    #[allow(clippy::too_many_arguments)]
    fn plaque(&mut self, cv: &mut Canvas, th: &Theme, fonts: &dyn Fonts, fnts: &(String, String, String), d: &RemData,
              e: &Value, x: f64, y: f64, w: f64, h: f64, f: f64, ink: &str, verm: &str, beam_y: Option<f64>,
              num: Option<usize>, small: bool) {
        let (font_ui, serif, jp) = fnts;
        let nts = d.now;
        let hd = d.hd;
        let eid = e["id"].as_str().unwrap_or("").to_string();
        let tag = format!("ema_{eid}");
        let tags = ["plaque", tag.as_str()];
        let sel = self.sel.as_deref() == Some(eid.as_str()) && !small;
        let hov = self.hover.as_deref() == Some(eid.as_str());
        let lift = if hov || sel { ti(3.0 * f) } else { 0.0 };
        let y = y - lift;
        let at = at_of(e);
        let isdone = truthy(&e["done"]);
        let rang = truthy(&e["rang"]) && !isdone;
        let soon = at.is_some_and(|a| !isdone && 0.0 < a - nts && a - nts < 3600.0);
        if let Some(by) = beam_y {
            cv.line(vec![x + w / 2.0 - 10.0 * f, by, x + w / 2.0, y + h * 0.13, x + w / 2.0 + 10.0 * f, by]).fill("#c0392b")
                .width(2.0f64.max(ti(2.0 * f))).tags(&tags);
            cv.oval(x + w / 2.0 - 3.0 * f, by - 2.0 * f, x + w / 2.0 + 3.0 * f, by + 4.0 * f).fill("#8a6a3a").outline("")
                .tags(&tags);
        }
        if rang || soon || sel {
            let g = self.img(format!("pglow {w} {h} {rang}"), || {
                px_photo(ti(w / 2.0) as i32 + 10, ti(h / 2.0) as i32 + 10,
                         &glow_art(ti(w / 4.0) as i32 + 5, if rang { (255, 90, 70) } else { (255, 190, 100) }, 0.45), 2, hd)
            });
            cv.image(x + w / 2.0, y + h / 2.0, g).tags(&tags);
        }
        cv.poly(vec![x + 3.0, y + h * 0.24 + 4.0, x + w / 2.0 + 3.0, y + 4.0, x + w + 3.0, y + h * 0.24 + 4.0, x + w + 3.0,
                     y + h + 4.0, x + 3.0, y + h + 4.0]).fill("#0a0710").outline("").tags(&tags);
        let z = if w >= 140.0 { 2.0 } else { 1.0 };
        let seed = e["seed"].as_i64().unwrap_or(1).rem_euclid(6);
        let (ww, wh) = (ti(w / z) as i32, ti(h / z) as i32);
        let wood = self.img(format!("wood {ww} {wh} {seed} {z}"), || px_photo(ww, wh, &ema_plaque_wood(ww, wh, seed), z as usize, hd));
        cv.image(x, y, wood).anchor("nw").tags(&tags);
        if sel || hov {
            cv.poly(vec![x, y + h * 0.24, x + w / 2.0, y, x + w, y + h * 0.24, x + w, y + h, x, y + h]).fill("")
                .outline(&th.gold).width(2.0).tags(&tags);
        }
        cv.oval(x + w / 2.0 - 3.0 * f, y + h * 0.13 - 3.0 * f, x + w / 2.0 + 3.0 * f, y + h * 0.13 + 3.0 * f).fill("#5a3a1e")
            .outline("").tags(&tags);
        if let Some(n) = num.filter(|_| !small) {
            cv.text(x + 12.0 * f, y + h * 0.3, &n.to_string()).anchor("w").fill(&mixhex(ink, "#e6c391", 0.45))
                .font(Font::new(font_ui, ti(8.0 * f) as i32, true)).tags(&tags);
        }
        let txt = e["text"].as_str().unwrap_or("");
        let fs = if small { 7.0f64.max(ti(10.0 * f)) } else { ti(if txt.chars().count() < 50 { 10.5 } else { 9.5 } * f) };
        cv.text(x + w / 2.0, y + h * 0.29, &clip(txt, if small { 40 } else { 90 })).anchor("n").fill(ink)
            .wrap(w - if small { 16.0 } else { 48.0 } * f).justify("center").font(Font::new(serif, fs as i32, true)).tags(&tags);
        if !small {
            let when = if isdone {
                "Fulfilled".to_string()
            } else if rang {
                format!("Rang {}", ema_when(at, nts, true))
            } else if let Some(a) = at {
                let today = day_of(d, a) == d.today;
                if a - nts < 86400.0 * 2.0 {
                    format!("{} \u{b7} {}", ema_when(at, nts, today), ema_left(a, nts))
                } else {
                    ema_when(at, nts, false)
                }
            } else {
                "Anytime".to_string()
            };
            let col = if rang { "#b3261e" } else if soon { "#8a3b12" } else { "#6b4a2a" };
            cv.line(vec![x + 16.0 * f, y + h - 30.0 * f, x + w - 16.0 * f, y + h - 30.0 * f])
                .fill(&mixhex("#e6c391", ink, 0.25)).tags(&tags);
            cv.text(x + w / 2.0, y + h - 17.0 * f, &when).fill(col).font(Font::new(font_ui, ti(8.5 * f) as i32, true)).tags(&tags);
            let t_ = cv.last_id();
            let bb_ = crate::canvasdraw::text_bbox(cv.items.last().unwrap(), fonts);
            if (bb_[2] - bb_[0]) as f64 > w - 20.0 * f {
                cv.items.last_mut().unwrap().font = Some(Font::new(font_ui, 6.0f64.max(ti(7.5 * f)) as i32, true));
            }
            if e["repeat"].as_str() == Some("daily") && !isdone {
                let bw_ = 28.0 * f;
                cv.move_items(&t_.to_string(), bw_ / 2.0 + 2.0, 0.0);
                let bb_ = crate::canvasdraw::text_bbox(cv.items.iter().find(|i| i.id == t_).unwrap(), fonts);
                let (bx_, by_) = (bb_[0] as f64 - bw_ - 3.0, y + h - 17.0 * f);
                rbox(cv, bx_, by_ - 7.0 * f, bx_ + bw_, by_ + 7.0 * f, 99, verm, "", &tags, "pill");
                cv.text(bx_ + bw_ / 2.0, by_, "\u{6bce}\u{65e5}").fill("#fbe9d0")
                    .font(Font::new(jp, 6.0f64.max(ti(7.0 * f)) as i32, true)).tags(&tags);
            }
            let (sx, sy) = (x + w - 20.0 * f, y + h * 0.27);
            cv.rect(sx - 7.0 * f, sy - 7.0 * f, sx + 7.0 * f, sy + 7.0 * f).fill(verm).outline("").tags(&tags);
            cv.text(sx, sy, "\u{72d0}").fill("#fbe9d0").font(Font::new(jp, 6.0f64.max(ti(7.0 * f)) as i32, true)).tags(&tags);
        }
        if isdone {
            let (cx, cy, rr) = (x + w * 0.62, y + h * 0.62, w.min(h) * 0.26);
            cv.oval(cx - rr, cy - rr, cx + rr, cy + rr).outline(verm).width(2.0f64.max(ti(3.0 * f))).tags(&tags);
            cv.text(cx, cy, "\u{6e08}").fill(verm).font(Font::new(jp, ti(rr * 0.95) as i32, true)).tags(&tags);
        }
        self.hits.push(([x, y, x + w, y + h], "plaque".into(), eid.clone()));
        if sel {
            let labels: &[(&str, &str)] = if isdone {
                &[("Undo", "undo"), ("Remove", "remove")]
            } else {
                &[("Done", "done"), ("Edit", "edit"), ("Snooze", "snooze"), ("Remove", "remove")]
            };
            let fnt = Font::new(font_ui, ti(8.5 * f) as i32, true);
            let widths: Vec<f64> = labels.iter().map(|(l, _)| (fonts.width(&fnt, l) as f64) + ti(18.0 * f)).collect();
            let gap_ = ti(4.0 * f);
            let tot = widths.iter().sum::<f64>() + gap_ * (labels.len() - 1) as f64;
            let mut px0 = x + w / 2.0 - tot / 2.0;
            let py = y + h + 8.0 * f;
            for ((lab, a), pw_) in labels.iter().zip(widths) {
                let isd = *a == "done";
                rbox(cv, px0, py, px0 + pw_, py + 22.0 * f, 99, if isd { &th.acc } else { &th.start },
                     &mixhex(&th.gold, &th.night, 0.3), &["pill"], "pill");
                cv.text(px0 + pw_ / 2.0, py + 11.0 * f, lab).fill(if isd { &th.onacc } else { &th.ink }).font(fnt.clone())
                    .tags(&["pill"]);
                self.hits.insert(0, ([px0, py, px0 + pw_, py + 22.0 * f], a.to_string(), eid.clone()));
                px0 += pw_ + gap_;
            }
        }
    }

    /// ema_hit(x, y) in canvas pixels
    pub fn hit(&self, x: f64, y: f64) -> Option<(String, String)> {
        self.hits.iter().find(|(b, _, _)| b[0] <= x && x <= b[2] && b[1] <= y && y <= b[3]).map(|(_, a, i)| (a.clone(), i.clone()))
    }

    /// zk_fit for the page and its banner; lays out at the window's size (pw, ph), then the canvas for its width
    #[allow(clippy::too_many_arguments)]
    pub fn fit(&mut self, ui: &mut Ui, fonts: &dyn Fonts, th: &Theme, f: &Faces, zk_f: f64, pw: i32, ph: i32, d: &RemData) {
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
        if ui.ws[self.cv].w != self.drawn_w && self.render(ui, th, f, fonts, zk_f, d) {
            ui.layout_at(fonts, pw, ph);
        }
        if self.sc.sync(ui) {
            ui.layout_at(fonts, pw, ph);
        }
    }
}

/// RemData from the app as it is now
impl crate::Companion {
    /// "Clear fulfilled": the done plaques go
    pub fn ema_clear_done(&mut self) {
        self.life_do(|l| {
            if let Some(a) = l["ema"].as_array_mut() {
                a.retain(|e| !truthy(&e["done"]));
            }
        });
    }

    pub fn rem_data(&mut self) -> RemData {
        let now = crate::unix_now();
        let now_l = now + crate::local_offset_pub() as f64;
        let mut done = self.ema_done_list();
        done.truncate(8);
        let fox = [1usize, 2].iter().map(|&z| {
            let im = self.small_fox_z(now, z);
            Rc::new(Photo { w: im.w, h: im.h, px: im.px })
        }).collect();
        RemData { now, now_l, today: now_l.div_euclid(86400.0) as i64, tt: self.tt.clone(), act: self.ema_active(), done,
                  jp: String::new(), serif: String::new(), aloud: self.cfg.b("remind_aloud", true),
                  hd: self.cfg.b("hd_art", true), fox }
    }
}
