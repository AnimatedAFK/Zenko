//! The hub window's frame as zenko.py builds it (FoxBuddy.open_hub with the Zenko, Zenko19, Zenko20 and Zenko21
//! additions): the shrine border (zk_border), the 244 px sidebar canvas, the custom title bar (hub_titlebar: app
//! name, the Ctrl K search pill, minimise / maximise / close), the tip bar with its fading tips (hub_anim) and the
//! page stage, laid out by the same pack / place rules Tk uses.

use crate::hubside::{Side, zk_size, round_pts_flat};
use crate::theme::Theme;
use crate::tkcanvas::{Canvas, Photo};
use crate::ui::{Anchor, Fill, Font, Fonts, Kind, Pack, Place, Side as PSide, Ui};
use crate::canvasdraw::text_bbox;
use std::rc::Rc;
use zenko_art::palette::{hex_rgb, mixhex};
use zenko_sound::mt::PyRandom;

pub const BORDER: i32 = 11;
pub const SHU: &str = "#c8372d";
pub const SIDE_W: i32 = 244;

/// ids of the hub's widgets in its Ui
#[derive(Clone, Debug, Default)]
pub struct HubIds {
    pub border: Option<usize>,
    pub side: usize,
    pub side_cv: usize,
    pub right: usize,
    pub bar: Option<usize>,
    pub title: Option<usize>,
    pub btns: Option<usize>,
    pub bmin: Option<usize>,
    pub bmax: Option<usize>,
    pub bclose: Option<usize>,
    pub pill: Option<usize>,
    pub line: usize,
    pub tipbar: usize,
    pub tip: usize,
    pub stage: usize,
}

pub struct Hub {
    pub ui: Ui,
    pub ids: HubIds,
    pub side: Side,
    pub t: Theme,
    pub font_ui: String,
    pub zk_f: f64,
    /// the border tiles (horizontal, vertical), made once per colour scheme
    tiles: Option<(Rc<Photo>, Rc<Photo>)>,
    border_wh: (i32, i32),
    pub tip_i: usize,
    pub tip_t0: f64,
    tip_now: Option<(String, String)>,
    pub pill_hover: bool,
    /// which window button the mouse is over ("min", "max", "close")
    pub btn_hover: Option<&'static str>,
}

/// zenko.py _lerp3 (int() truncation, t clamped)
pub fn lerp3(a: [u8; 3], b: [u8; 3], t: f64) -> [u8; 3] {
    let t = t.clamp(0.0, 1.0);
    let f = |x: u8, y: u8| (x as f64 + (y as f64 - x as f64) * t) as i64 as u8;
    [f(a[0], b[0]), f(a[1], b[1]), f(a[2], b[2])]
}

/// zk_border's tile(): 96 px of lacquered vermilion wood with gold lines and studs
pub fn border_tile(horizontal: bool, night: &str, gold: &str) -> Photo {
    let (b, n) = (BORDER as usize, 96usize);
    let (wd, ht) = if horizontal { (n, b) } else { (b, n) };
    let mut rnd = PyRandom::new(11);
    let base = hex_rgb(&mixhex(SHU, night, 0.35));
    let dark = hex_rgb(&mixhex(SHU, "#000000", 0.62));
    let gold = hex_rgb(gold);
    let ph: Vec<f64> = (0..4).map(|_| rnd.uniform(0.0, 6.28)).collect();
    let mut px = vec![[0u8; 4]; wd * ht];
    for y in 0..ht {
        for x in 0..wd {
            let (u, v) = if horizontal { (x, y) } else { (y, x) };
            let (uf, vf) = (u as f64, v as f64);
            let grain = 0.5 + 0.22 * (uf * 0.21 + 3.2 * (vf * 0.9 + ph[0]).sin() + ph[1]).sin()
                + 0.12 * (uf * 0.05 + ph[2]).sin() + rnd.uniform(-0.08, 0.08);
            let mut col = lerp3(dark, base, grain.clamp(0.0, 1.0));
            if v == 0 || v == b - 1 {
                col = lerp3(col, [8, 6, 10], 0.75);
            } else if v == b - 3 {
                col = lerp3(col, gold, 0.75);
            } else if v == 2 {
                col = lerp3(col, gold, 0.28);
            }
            if v == 5 && (u % 24 == 11 || u % 24 == 12) {
                col = lerp3(col, gold, 0.85);
            }
            px[y * wd + x] = [col[0], col[1], col[2], 255];
        }
    }
    Photo { w: wd, h: ht, px }
}

/// Zenko.zk_factor with Zenko21's Zen Maru Gothic step: how much the hub's text grows with the window
pub fn zk_factor(w: i32, h: i32, text_scale: f64, zen_maru: bool) -> f64 {
    let (w, h) = if w < 200 || h < 200 { (1240, 800) } else { (w, h) };
    let mut f = 0.5 * (w as f64 / 1240.0) + 0.5 * (h as f64 / 800.0);
    f *= text_scale;
    f = f.clamp(0.85, 1.6);
    if zen_maru {
        f = 1.7f64.min(f * 1.07);
    }
    f
}

/// FoxBuddy.mix: like _mixhex, without clamping
pub fn mix(c1: &str, c2: &str, t: f64) -> String {
    let (a, b) = (hex_rgb(c1), hex_rgb(c2));
    let v: Vec<i64> = (0..3).map(|i| (a[i] as f64 * (1.0 - t) + b[i] as f64 * t) as i64).collect();
    format!("#{:02x}{:02x}{:02x}", v[0], v[1], v[2])
}

fn canvas_of(ui: &mut Ui, id: usize) -> &mut Canvas {
    let Kind::Canvas { tk, .. } = &mut ui.ws[id].kind else { panic!("not a canvas") };
    tk.get_or_insert_with(Default::default)
}

impl Hub {
    /// the window's widgets, as open_hub builds them (borderless with the shrine frame is zenko.py's default)
    pub fn new(t: &Theme, font_ui: &str, navkeys: Vec<(String, String)>, borderless: bool, ui_border: bool,
               w: i32, h: i32, tip_i: usize, now: f64) -> Hub {
        let night = t.night.clone();
        let mut ui = Ui::new(&night);
        let mut ids = HubIds::default();
        let side = ui.frame(0, &night);
        (ui.ws[side].opt_w, ui.ws[side].propagate) = (SIDE_W, false);
        ui.pack(side, Pack::default().side(PSide::Left).fill(Fill::Y));
        let sb = ui.canvas(side, &night, 0, 0);
        ui.ws[sb].place = Some(Place { relw: Some(1.0), relh: Some(1.0), ..Default::default() });
        ids.side = side;
        ids.side_cv = sb;
        let right = ui.frame(0, &t.paper);
        ui.pack(right, Pack::default().side(PSide::Left).fill(Fill::Both).expand());
        ids.right = right;
        if borderless {
            // hub_titlebar
            let bar = ui.frame(right, &night);
            (ui.ws[bar].opt_h, ui.ws[bar].propagate) = (34, false);
            ui.pack(bar, Pack::default().side(PSide::Top).fill(Fill::X));
            let lab = ui.label(bar, "\u{2726}  ZENKO", &night, &mixhex(&night, &t.gold, 0.7), Font::new(font_ui, 9, true));
            ui.pack(lab, Pack::default().side(PSide::Left).padx(16, 16));
            let btns = ui.frame(bar, &night);
            ui.pack(btns, Pack::default().side(PSide::Right).fill(Fill::Y));
            let mk = |ui: &mut Ui| {
                let c = ui.canvas(btns, &night, 46, 34);
                ui.pack(c, Pack::default().side(PSide::Left));
                c
            };
            let (a, b, c) = (mk(&mut ui), mk(&mut ui), mk(&mut ui));
            // Zenko20: the search pill
            let pill = ui.canvas(bar, &night, 300, 26);
            ui.pack(pill, Pack::default().side(PSide::Left).padx(6, 0).pady(4, 4));
            (ids.bar, ids.title, ids.btns, ids.bmin, ids.bmax, ids.bclose, ids.pill) =
                (Some(bar), Some(lab), Some(btns), Some(a), Some(b), Some(c), Some(pill));
        }
        let line = ui.frame(right, &t.line);
        ui.ws[line].opt_h = 1;
        ui.pack(line, Pack::default().side(PSide::Bottom).fill(Fill::X));
        let tipbar = ui.frame(right, &night);
        ui.pack(tipbar, Pack::default().side(PSide::Bottom).fill(Fill::X));
        let tip = ui.label(tipbar, "", &night, &night, Font::new(font_ui, 10, false));
        ui.set_anchor(tip, Anchor::W);
        (ui.ws[tip].padx, ui.ws[tip].pady) = (18, 7);
        ui.pack(tip, Pack::default().fill(Fill::X));
        let stage = ui.frame(right, &t.paper);
        ui.pack(stage, Pack::default().fill(Fill::Both).expand());
        (ids.line, ids.tipbar, ids.tip, ids.stage) = (line, tipbar, tip, stage);
        if ui_border {
            // zk_border: the toplevel gets padx = pady = 11 and a canvas placed over all of it, lowered
            (ui.ws[0].padx, ui.ws[0].pady) = (BORDER, BORDER);
            let cv = ui.canvas(0, &mixhex(SHU, &night, 0.4), 0, 0);
            ui.ws[cv].place = Some(Place { ignore: true, relw: Some(1.0), relh: Some(1.0), ..Default::default() });
            ui.ws[cv].low = true;
            ids.border = Some(cv);
        }
        let mut hub = Hub { ui, ids, side: Side::new(navkeys, 2), t: t.clone(), font_ui: font_ui.into(), zk_f: 1.0,
                            tiles: None, border_wh: (0, 0), tip_i, tip_t0: now, tip_now: None, pill_hover: false,
                            btn_hover: None };
        hub.ui.ws[0].w = w;
        hub.ui.ws[0].h = h;
        hub.buttons_draw();
        hub
    }

    /// zk_fit's font pass over the frame: every Label's font scaled by zk_size from the size it was made with
    pub fn fit_fonts(&mut self, zk_f: f64) {
        self.zk_f = zk_f;
        let mut set = |id: Option<usize>, base: i32| {
            if let Some(id) = id
                && let Kind::Label { font, .. } = &mut self.ui.ws[id].kind {
                font.size = zk_size(zk_f, base);
            }
        };
        set(self.ids.title, 9);
        set(Some(self.ids.tip), 10);
    }

    /// lay the window out at w x h; redraws the border when its size changed (its <Configure>)
    pub fn layout(&mut self, fonts: &dyn Fonts, w: i32, h: i32) {
        self.pill_draw(fonts);
        self.ui.layout_at(fonts, w, h);
        if let Some(b) = self.ids.border {
            let (bw, bh) = (self.ui.ws[b].w, self.ui.ws[b].h);
            if (bw, bh) != self.border_wh {
                self.border_wh = (bw, bh);
                self.border_draw(bw, bh);
            }
        }
        let sc = self.ids.side_cv;
        (self.side.w, self.side.h) = (self.ui.ws[sc].w, self.ui.ws[sc].h);
    }

    /// the sidebar canvas's items go into the window (after Side::draw)
    pub fn side_sync(&mut self) {
        let sc = self.ids.side_cv;
        *canvas_of(&mut self.ui, sc) = self.side.cv.clone();
    }

    /// zk_border's draw(): tiles along all four edges, gold corner fittings with rivets, the fox-mask crest and the
    /// flame
    pub fn border_draw(&mut self, w: i32, h: i32) {
        let Some(id) = self.ids.border else { return };
        if w < 50 || h < 50 {
            return;
        }
        let t = self.t.clone();
        let (th, tv) = self.tiles.get_or_insert_with(|| {
            (Rc::new(border_tile(true, &t.night, &t.gold)), Rc::new(border_tile(false, &t.night, &t.gold)))
        }).clone();
        let cv = canvas_of(&mut self.ui, id);
        let glow: Vec<_> = cv.items.iter().filter(|i| i.has("z19glow")).cloned().collect();
        *cv = Canvas::default();
        let b = BORDER;
        let f = |v: i32| v as f64;
        let mut x = 0;
        while x < w {
            cv.image(f(x), 0.0, th.clone()).anchor("nw");
            cv.image(f(x), f(h - b), th.clone()).anchor("nw");
            x += 96;
        }
        let mut y = 0;
        while y < h {
            cv.image(0.0, f(y), tv.clone()).anchor("nw");
            cv.image(f(w - b), f(y), tv.clone()).anchor("nw");
            y += 96;
        }
        let (g, gd) = (t.gold.clone(), mixhex(&t.gold, "#000000", 0.45));
        let hi = mixhex(&t.gold, "#ffffff", 0.45);
        let rivet = mixhex(&t.gold, "#000000", 0.25);
        let l = 30;
        for (cx, cy, sx, sy) in [(0, 0, 1, 1), (w, 0, -1, 1), (0, h, 1, -1), (w, h, -1, -1)] {
            let pts = [cx, cy, cx + sx * l, cy, cx + sx * l, cy + sy * (b - 2), cx + sx * (b + 2), cy + sy * (b - 2),
                       cx + sx * (b - 2), cy + sy * (b + 2), cx + sx * (b - 2), cy + sy * l, cx, cy + sy * l];
            cv.poly(pts.iter().map(|&v| f(v)).collect()).fill(&g).outline(&gd);
            cv.line(vec![f(cx + sx * 2), f(cy + sy * 2), f(cx + sx * (l - 2)), f(cy + sy * 2)]).fill(&hi);
            cv.line(vec![f(cx + sx * 2), f(cy + sy * 2), f(cx + sx * 2), f(cy + sy * (l - 2))]).fill(&hi);
            for (rx, ry) in [(l - 6, b / 2), (b / 2, l - 6), (b / 2 + 1, b / 2 + 1)] {
                let (x_, y_) = (cx + sx * rx, cy + sy * ry);
                cv.oval(f(x_ - 2), f(y_ - 2), f(x_ + 2), f(y_ + 2)).fill(&rivet).outline(&gd);
                cv.rect(f(x_ - 1), f(y_ - 1), f(x_), f(y_)).fill(&hi).outline("");
            }
        }
        let m = w / 2;
        cv.poly([m - 18, 0, m + 18, 0, m + 13, b, m - 13, b].iter().map(|&v| f(v)).collect())
            .fill(&mixhex(&t.night, "#000000", 0.3)).outline(&gd);
        let face = [m - 7, 1, m - 5, 3, m - 2, 2, m + 2, 2, m + 5, 3, m + 7, 1, m + 6, 6, m + 2, 9, m, 10, m - 2, 9, m - 6, 6];
        cv.poly(face.iter().map(|&v| f(v)).collect()).fill("#fff6ea").outline(&gd);
        cv.line(vec![f(m - 4), 5.0, f(m - 2), 6.0]).fill(SHU).width(1.0);
        cv.line(vec![f(m + 4), 5.0, f(m + 2), 6.0]).fill(SHU).width(1.0);
        cv.rect(f(m - 1), 3.0, f(m), 4.0).fill(SHU).outline("");
        let fl = [m, h - b + 1, m + 4, h - 4, m + 2, h - 1, m - 2, h - 1, m - 4, h - 4];
        cv.poly(fl.iter().map(|&v| f(v)).collect()).fill(&t.glow).outline(&mixhex(&t.glow, "#000000", 0.4));
        cv.items.extend(glow);
    }

    /// hub_titlebar's mk(): the three window buttons (their background turns to the hover colour under the mouse)
    pub fn buttons_draw(&mut self) {
        let col = mixhex(&self.t.night, &self.t.ink, 0.8);
        for (k, id) in [("min", self.ids.bmin), ("max", self.ids.bmax), ("close", self.ids.bclose)] {
            let Some(id) = id else { continue };
            let hover = if k == "close" { "#e5484d".to_string() } else { mixhex(&self.t.night, &self.t.ink, 0.12) };
            self.ui.ws[id].bg = if self.btn_hover == Some(k) { hover } else { self.t.night.clone() };
            let cv = canvas_of(&mut self.ui, id);
            *cv = Canvas::default();
            match k {
                "min" => {
                    cv.line(vec![18.0, 17.0, 28.0, 17.0]).fill(&col).width(1.4);
                }
                "max" => {
                    cv.rect(18.0, 12.0, 28.0, 22.0).outline(&col).width(1.4);
                }
                _ => {
                    cv.line(vec![18.0, 12.0, 28.0, 22.0]).fill(&col).width(1.4);
                    cv.line(vec![28.0, 12.0, 18.0, 22.0]).fill(&col).width(1.4);
                }
            }
        }
    }

    /// Zenko20's search pill: "Search pages, settings, actions", then "Ctrl K", the canvas as wide as the words
    pub fn pill_draw(&mut self, fonts: &dyn Fonts) {
        let Some(id) = self.ids.pill else { return };
        let (night, ink, gold) = (self.t.night.clone(), self.t.ink.clone(), self.t.gold.clone());
        let hover = self.pill_hover;
        let fu = self.font_ui.clone();
        let cv = canvas_of(&mut self.ui, id);
        *cv = Canvas::default();
        cv.text(32.0, 13.0, "Search pages, settings, actions").anchor("w")
            .fill(&mixhex(&night, &ink, if hover { 0.72 } else { 0.62 })).font(Font::new(&fu, 9, false)).tags(&["t"]);
        let t1 = cv.last_id();
        cv.text(0.0, 13.0, "Ctrl K").anchor("w").fill(&mixhex(&night, &ink, 0.42)).font(Font::new(&fu, 8, false))
            .tags(&["t"]);
        let t2 = cv.last_id();
        let b1 = text_bbox(cv.get(t1).unwrap(), fonts);
        cv.get(t2).unwrap().coords = vec![(b1[2] + 14) as f64, 13.0];
        let w = text_bbox(cv.get(t2).unwrap(), fonts)[2] + 14;
        let base = mixhex(&night, &ink, if hover { 0.10 } else { 0.06 });
        let pts = round_pts_flat(1.0, 1.0, (w - 2) as f64, 24.0, 99, 2, "pill");
        cv.poly(pts).fill(&base).outline(&mixhex(&night, &gold, if hover { 0.35 } else { 0.18 })).join("miter");
        cv.text(16.0, 13.0, "\u{2315}").fill(&gold).font(Font::new(&fu, 10, false)).anchor("w");
        cv.tag_raise("t");
        self.ui.ws[id].opt_w = w;
    }

    /// hub_anim's rotating tip: a new one every 9 s, fading in over 0.6 s and out over the last second
    pub fn tip_update(&mut self, now: f64) -> bool {
        let mut e = now - self.tip_t0;
        if e > 9.0 {
            self.tip_t0 = now;
            self.tip_i = (self.tip_i + 1) % crate::tips::TIPS.len();
            e = 0.0;
        }
        let a = if e < 8.0 { (e / 0.6).min(1.0) } else { ((9.0 - e) / 1.0).max(0.0) };
        // round(0.85 * a, 2): Python rounds the exact value, as Rust's formatting does
        let a: f64 = format!("{:.2}", 0.85 * a).parse().unwrap();
        let tip = (format!("\u{2726}  {}", crate::tips::TIPS[self.tip_i]),
                   mix(&self.t.night, &mixhex(&self.t.ink, &self.t.gold, 0.25), a));
        if self.tip_now.as_ref() == Some(&tip) {
            return false;
        }
        if let Kind::Label { text, fg, .. } = &mut self.ui.ws[self.ids.tip].kind {
            *text = tip.0.clone();
            *fg = tip.1.clone();
        }
        self.tip_now = Some(tip);
        true
    }
}

/// z22_side_extras' pins: the pinned badges you have (at most 3) as (id, kanji, name, tier)
pub fn side_pins(life: &serde_json::Value) -> Vec<(String, String, String, i64)> {
    let d = crate::rewards::rdata();
    let empty = vec![];
    let badges = d["BADGES"].as_array().unwrap_or(&empty);
    let tiered = d["BADGE_TIERED"].as_array().unwrap_or(&empty);
    life["badge_pins"].as_array().unwrap_or(&empty).iter().filter_map(|b| b.as_str())
        .filter(|b| life["badges"].get(*b).is_some()).take(3)
        .filter_map(|bid| {
            let b = badges.iter().find(|b| b[0] == bid)?;
            let tier = if tiered.iter().any(|t| t[0] == bid) { life["btier"][bid].as_i64().unwrap_or(0) } else { 0 };
            Some((bid.to_string(), b[1].as_str()?.to_string(), b[2].as_str()?.to_string(), tier))
        }).collect()
}

/// zenko20 z20_nav_list (with Zenko22's Town square after Chat): the menu in your order, minus the hidden ones
pub fn nav_list(cfg: &crate::Cfg) -> Vec<(String, String)> {
    let all = [("home", "Home"), ("timetable", "Timetable"), ("deadlines", "Upcoming"), ("reminders", "Reminders"),
               ("notebook", "Study books"), ("daily", "Daily"), ("exercise", "Exercise"), ("garden", "Zen garden"),
               ("map", "World map"), ("screen", "Screen time"), ("chat", "Chat"), ("square", "Town square"),
               ("notes", "Notes"), ("you", "You"), ("customize", "Customize"), ("settings", "Settings")];
    let strs = |k: &str| -> Vec<String> {
        cfg.raw.get(k).and_then(|v| v.as_array()).map(|a| a.iter().filter_map(|x| x.as_str().map(String::from)).collect())
            .unwrap_or_default()
    };
    let mut order: Vec<String> = strs("nav_order").into_iter().filter(|k| all.iter().any(|a| a.0 == k)).collect();
    for (k, _) in all {
        if !order.iter().any(|o| o == k) {
            order.push(k.into());
        }
    }
    let hidden: Vec<String> = strs("nav_hidden").into_iter().filter(|k| k != "home" && k != "settings").collect();
    order.into_iter().filter(|k| !hidden.contains(k))
        .map(|k| { let l = all.iter().find(|a| a.0 == k).unwrap().1; (k, l.to_string()) }).collect()
}

impl Hub {
    /// hub_anim's menu pill glide: a third of the way to the chosen item each frame
    pub fn glide(&mut self, c: &crate::hubside::SideCtx) -> bool {
        let (Some(to), Some(y)) = (self.side.pill_to, self.side.pill_y) else { return false };
        if (to - y).abs() <= 0.5 {
            return false;
        }
        let dy = (to - y) * 0.32;
        self.side.pill_y = Some(y + if dy.abs() > 0.5 { dy } else { to - y });
        self.side.nav(c);
        true
    }

    /// FoxBuddy.hub_zone: which window edge the mouse is on ("l", "r", "b", "lb", "rb" or ""), with Zenko's wider
    /// grab inside the 11 px shrine frame
    pub fn zone(&self, x: i32, y: i32, maxed: bool) -> String {
        if maxed {
            return String::new();
        }
        let (w, h) = (self.ui.ws[0].w, self.ui.ws[0].h);
        let z = format!("{}{}", if x <= 5 { "l" } else if x >= w - 6 { "r" } else { "" }, if y >= h - 6 { "b" } else { "" });
        if !z.is_empty() || self.ids.border.is_none() {
            return z;
        }
        let b = BORDER;
        format!("{}{}", if x <= b { "l" } else if x >= w - b - 1 { "r" } else { "" }, if y >= h - b - 1 { "b" } else { "" })
    }
}

impl Hub {
    /// hub_tick's self.hub_status.config(text=...): the sidebar's status line
    pub fn set_status(&mut self, text: &str) -> bool {
        if self.side.status_text == text {
            return false;
        }
        self.side.status_text = text.into();
        if let Some(id) = self.side.status_item
            && let Some(it) = self.side.cv.get(id) {
            it.text = text.into();
        }
        true
    }

    /// which window part is at (x, y) in window pixels
    pub fn part_at(&self, x: i32, y: i32) -> &'static str {
        let inside = |id: Option<usize>| id.is_some_and(|id| {
            let w = &self.ui.ws[id];
            let (ax, ay) = self.ui.abs(id);
            w.mapped && x >= ax && y >= ay && x < ax + w.w && y < ay + w.h
        });
        if inside(self.ids.bmin) { return "min"; }
        if inside(self.ids.bmax) { return "max"; }
        if inside(self.ids.bclose) { return "close"; }
        if inside(self.ids.pill) { return "pill"; }
        if inside(self.ids.title) { return "title"; }
        if inside(self.ids.bar) { return "bar"; }
        if inside(Some(self.ids.side_cv)) { return "side"; }
        if inside(Some(self.ids.stage)) || inside(Some(self.ids.tipbar)) || inside(Some(self.ids.line)) { return "page"; }
        if self.ids.border.is_some() { "border" } else { "" }
    }
}

/// hub_tick's status line: the session's task and time left, a break, or today's focus minutes
pub fn status_line(session: Option<(&str, f64, f64)>, on_break: bool, today_min: i64) -> String {
    if let Some((task, planned, elapsed)) = session {
        let left = (planned - elapsed).max(0.0);
        format!("{}\n{:02}:{:02} left", crate::session::clip(task, 22), (left / 60.0).floor() as i64,
                (left % 60.0) as i64)
    } else if on_break {
        "On a break".into()
    } else {
        format!("Idle  \u{b7}  {today_min} min focused today")
    }
}

/// seigaiha_png: blue-ocean waves over a vertical gradient, fading out to the left
pub fn seigaiha(w: usize, h: usize, top: &str, bottom: &str, line: &str, r: i64, fade: bool) -> Photo {
    let (t, b, l) = (hex_rgb(top), hex_rgb(bottom), hex_rgb(line));
    let dy = r / 2;
    let mut px = Vec::with_capacity(w * h);
    for y in 0..h {
        let tt = y as f64 / (1.max(h as i64 - 1)) as f64;
        let base: [f64; 3] = std::array::from_fn(|k| t[k] as f64 + (b[k] as f64 - t[k] as f64) * tt);
        for x in 0..w {
            let mut col = base;
            let (fx, fy) = (x as f64 + 0.5, y as f64 + 0.5);
            let mut best: Option<f64> = None;
            let k0 = (fy / dy as f64).floor() as i64;
            for row in k0..k0 + r / dy + 2 {
                let cy = (row * dy) as f64;
                if cy < fy - 0.5 {
                    continue;
                }
                let off = if row % 2 != 0 { r } else { 0 };
                let cx = ((fx - off as f64) / (2.0 * r as f64)).round_ties_even() as i64 * 2 * r + off;
                let d = (fx - cx as f64).hypot(fy - cy);
                if d <= r as f64 + 0.5 {
                    best = Some(d);
                }
            }
            if let Some(d) = best {
                let mut a: f64 = 0.0;
                for rr in [r as f64 - 1.0, r as f64 - 5.0, r as f64 - 9.0] {
                    a = a.max(crate::aa::cov(0.75 - (d - rr).abs()));
                }
                let k = a * if fade { 1.0f64.min(x as f64 / (w as f64 * 0.55)) } else { 1.0 };
                col = std::array::from_fn(|i| col[i] + (l[i] as f64 - col[i]) * k);
            }
            px.push([col[0] as u8, col[1] as u8, col[2] as u8, 255]);
        }
    }
    Photo { w, h, px }
}

/// Zenko.hub_title's banner on a page-wide canvas: gradient, seigaiha, a few stars, the title, the subtitle, gold
/// rule and diamond. Returns the canvas height (it grows with the window's text size and a long subtitle).
#[allow(clippy::too_many_arguments)]
pub fn hub_title_draw(cv: &mut Canvas, w: i32, title: &str, sub: &str, t: &Theme, font_ui: &str, font_title: &str,
                      zk_f: f64, fonts: &dyn Fonts, seig: &mut Option<(String, Rc<Photo>)>) -> i32 {
    let (ts, ss) = (zk_size(zk_f, 21), zk_size(zk_f, 10));
    let fsub = Font::new(font_ui, ss, false);
    let wtxt = fonts.width(&fsub, if sub.is_empty() { " " } else { sub });
    let lines = 1.max((wtxt as f64 / 100.0f64.max(w as f64 - 60.0)).ceil() as i32);
    let hh = (20.0 + ts as f64 * 1.9 + 14.0 + (lines * fonts.linespace(&fsub)) as f64 + 12.0) as i32;
    *cv = Canvas::default();
    let mut y = 0;
    while y < hh {
        cv.rect(0.0, y as f64, w as f64, (y + 2) as f64).fill(&mixhex(&t.night, &t.banner, y as f64 / hh as f64)).outline("");
        y += 2;
    }
    let key = format!("{}|{}|{}|{hh}", t.night, t.banner, t.acc);
    if seig.as_ref().map(|s| &s.0) != Some(&key) {
        *seig = Some((key, Rc::new(seigaiha(340, hh as usize, &t.night, &t.banner, &mixhex(&t.banner, &t.acc, 0.30), 14, true))));
    }
    cv.image(0.max(w - 340) as f64, 0.0, seig.as_ref().unwrap().1.clone()).anchor("nw");
    let mut rnd = PyRandom::new(title.chars().count() as u64);
    for _ in 0..18 {
        let sx = rnd.uniform(0.0, w as f64 * 0.75);
        let sy = rnd.uniform(4.0, hh as f64 * 0.6);
        let f = mixhex(&t.night, &t.ink, rnd.uniform(0.2, 0.6));
        cv.rect(sx, sy, sx + 2.0, sy + 2.0).fill(&f).outline("");
    }
    let tf = if font_title.is_empty() { font_ui } else { font_title };
    cv.text(28.0, 18.0, title).anchor("nw").fill(&t.ink).font(Font::new(tf, ts, true));
    cv.text(30.0, (18.0 + ts as f64 * 1.9 + 8.0) as i32 as f64, sub).anchor("nw").wrap((w - 60) as f64).fill(&t.grey)
        .font(Font::new(font_ui, ss, false)).tags(&["status"]);
    let hf = hh as f64;
    cv.line(vec![0.0, hf - 2.0, w as f64, hf - 2.0]).fill(&mixhex(&t.banner, &t.gold, 0.6));
    cv.line(vec![0.0, hf - 1.0, w as f64, hf - 1.0]).fill(&mixhex(&t.paper, "#000000", 0.2));
    cv.poly(vec![28.0, hf - 6.0, 32.0, hf - 2.0, 28.0, hf + 2.0, 24.0, hf - 2.0]).fill(&t.gold).outline("");
    hh
}
