//! The hub's Exercise page (zenko.py ZenkoGym hub_build_exercise / gym_render and its cards, with Zenko's food card
//! and Zenko20's cached glow figures): the dusk banner, the muscle map (day tabs, the front and back figures in their
//! glow frames, a moon and an enso ring), today's card (pick the day / tick the muscle groups / the rest night), the
//! daily protein foxfire, water cups and sleep moons, the weight line with the profile and calorie guide, and the
//! food list with its meters. The looping animations (stars, foxfire, figures, the sleepy z's) step from the hub.

use crate::bodyart::{BODY_H, BODY_W, body_frames, gym_data};
use crate::hubwin::{canvas_of, hub_title_draw};
use crate::pagekit::{Faces, Scroller, WrapState, hub_card, hub_scroller, label_w, tt_btn, tt_entry, zk_fonts, zk_wrap};
use crate::rempage::{glow_art, px_photo};
use crate::shelfpage::spaced;
use crate::theme::Theme;
use crate::tkcanvas::{Canvas, Photo};
use crate::ui::{Anchor, Fill, Font, Fonts, Grid, Kind, Pack, Side, Ui};
use serde_json::Value;
use std::collections::HashMap;
use std::rc::Rc;
use zenko_art::palette::mixhex;

pub const TITLE: &str = "Exercise";
pub const SUB: &str = "\u{935b}\u{932c} \u{b7} Train slow and steady: stronger every week, lighter every month";
pub const WATER_GOAL: i64 = 8;
const ACC: (i64, i64, i64) = (236, 78, 58);

/// what the page shows: gym() (with its setdefaults), today, the fonts
#[derive(Clone, Debug, Default)]
pub struct GymData {
    pub gym: Value,
    pub today_iso: String,
    pub today: i64,
    /// _mincho_font() and _serif_font()
    pub jp: String,
    pub serif: String,
}

impl GymData {
    /// gym_today(): which split day today is (0..3), or None until you've said
    pub fn gym_today(&self) -> Option<usize> {
        self.gym["next"].as_i64().map(|n| n.rem_euclid(4) as usize)
    }
    fn day(&self, i: usize) -> &Value {
        &gym_data()["GYM_ALL"][i]
    }
    fn s(v: &Value) -> String {
        v.as_str().unwrap_or("").to_string()
    }
    /// gym_done_regions(day)
    fn done_regions(&self, i: usize) -> Vec<String> {
        let key = format!("{}|{}", self.today_iso, i);
        let got: Vec<String> = self.gym["checks"][key.as_str()].as_array().map(|a| a.iter().filter_map(|v| v.as_str()
            .map(String::from)).collect()).unwrap_or_default();
        let mut out: Vec<String> = vec![];
        for g in self.day(i)["groups"].as_array().unwrap() {
            if got.contains(&Self::s(&g[0])) {
                out.extend(g[2].as_array().unwrap().iter().map(Self::s));
            }
        }
        out.sort();
        out
    }
    fn food_today(&self) -> Vec<Value> {
        self.gym["food"][self.today_iso.as_str()].as_array().cloned().unwrap_or_default()
    }
}

/// calorie_plan(kg, cm, age, sex): (protein low, high), and (upkeep, target) once the height is known
pub fn calorie_plan(kg: Option<f64>, cm: Option<f64>, age: i64, sex: &str) -> Option<((i64, i64), Option<(i64, i64)>)> {
    let kg = kg.filter(|k| *k != 0.0)?;
    let r = |v: f64| v.round_ties_even() as i64;
    let prot = (r(kg * 1.6), r(kg * 2.0));
    let Some(cm) = cm.filter(|c| *c != 0.0) else { return Some((prot, None)) };
    let mut base = 10.0 * kg + 6.25 * cm - 5.0 * age as f64;
    base += if sex == "m" { 5.0 } else if sex == "f" { -161.0 } else { -78.0 };
    let upkeep = r(base * 1.55 / 10.0) * 10;
    let floor = if sex == "m" { 2000 } else if sex == "f" { 1800 } else { 1900 };
    let target = floor.max(upkeep - 300);
    Some((prot, Some((upkeep, r(target as f64 / 10.0) * 10))))
}

fn fmt_i(v: i64) -> String {
    crate::routes17::fmt_int(v)
}

/// a looping canvas animation the hub steps
#[derive(Clone, Debug)]
pub enum Anim {
    /// gym_banner's twinkle (420 ms)
    Twinkle { cv: usize },
    /// gym_foxfire's flicker (230 ms): (outer polygon, cx, b, fh, fw) per lit flame
    Flick { cv: usize, flames: Vec<(u32, f64, f64, f64, f64)> },
    /// the muscle map's glow frames and stars (260 ms)
    Figures { cv: usize, front: u32, back: u32, day: usize },
    /// the rest night's z's and stars (500 ms)
    Drift { cv: usize, zs: Vec<u32>, night: String },
}

pub struct GymPage {
    pub pg: usize,
    pub title: usize,
    pub sc: Scroller,
    pub body: usize,
    /// gym_view: the day the map and card show
    pub view: usize,
    banner: usize,
    banner_w: i32,
    pub clicks: Vec<(usize, String)>,
    /// the weight, height, food entries; the weight message label; the sex radios
    pub wentry: usize,
    pub hentry: usize,
    pub fentry: usize,
    pub wmsg: usize,
    pub fstatus: usize,
    pub radios: Vec<(usize, &'static str)>,
    pub anims: Vec<(Anim, f64, u64)>,
    /// the glow figures, made once a day / done set (gym_photos' cache)
    photos: HashMap<(usize, Vec<String>), (Vec<Rc<Photo>>, Vec<Rc<Photo>>)>,
    moon: Option<Rc<Photo>>,
    pub wrap: WrapState,
    seig: Option<(String, Rc<Photo>)>,
    th: Theme,
    f: Faces,
    d: GymData,
}

fn justify_left(ui: &mut Ui, l: usize, wl: i32) {
    if let Kind::Label { left, wrap, .. } = &mut ui.ws[l].kind {
        *left = true;
        *wrap = wl;
    }
}

fn wrapl(ui: &mut Ui, l: usize, wl: i32) {
    if let Kind::Label { wrap, .. } = &mut ui.ws[l].kind {
        *wrap = wl;
    }
}

fn italic(fam: &str, n: i32) -> Font {
    Font { family: fam.into(), size: n, bold: false, italic: true }
}

impl GymPage {
    /// hub_build_exercise(pg): the title, the scroller and its body, gym_view = today's day (else 0), gym_render()
    pub fn build(ui: &mut Ui, pg: usize, th: &Theme, f: &Faces, cm10: i32, d: &GymData, now: f64) -> GymPage {
        let title = ui.canvas(pg, &th.banner, cm10, 96);
        canvas_of(ui, title);
        ui.pack(title, Pack::default().fill(Fill::X));
        let sc = hub_scroller(ui, pg, th, cm10);
        let body = ui.frame(sc.inner, &th.paper);
        ui.pack(body, Pack::default().fill(Fill::X).padx(24, 24).pady(14, 14));
        let view = d.gym_today().unwrap_or(0);
        let mut p = GymPage { pg, title, sc, body, view, banner: 0, banner_w: 0, clicks: vec![], wentry: 0, hentry: 0,
                              fentry: 0, wmsg: 0, fstatus: 0, radios: vec![], anims: vec![], photos: HashMap::new(),
                              moon: None, wrap: WrapState::default(), seig: None, th: th.clone(),
                              f: Faces { ui: f.ui.clone(), title: f.title.clone() }, d: d.clone() };
        p.render(ui, cm10, d, now);
        p
    }

    fn click(&mut self, ids: &[usize], cmd: &str) {
        for &i in ids {
            self.clicks.push((i, cmd.to_string()));
        }
    }

    /// the command of the clickable widget at a point (a label, a cup, a seal), if any
    pub fn click_at(&self, ui: &Ui, mx: i32, my: i32) -> Option<String> {
        let mut over = ui.hit(mx, my);
        while let Some(i) = over {
            if let Some(c) = self.clicks.iter().find(|c| c.0 == i) {
                return Some(c.1.clone());
            }
            over = ui.ws[i].parent;
        }
        None
    }

    fn hand(ui: &mut Ui, ids: &[usize]) {
        for &i in ids {
            ui.ws[i].hand = true;
        }
    }

    /// gym_render(): the body made again
    pub fn render(&mut self, ui: &mut Ui, cm10: i32, d: &GymData, now: f64) {
        self.d = d.clone();
        for k in ui.ws[self.body].kids.clone() {
            ui.destroy(k);
        }
        self.wrap = WrapState { lastw: self.wrap.lastw, ..WrapState::default() };
        self.clicks.clear();
        self.radios.clear();
        self.anims.clear();
        let body = self.body;
        let th = self.th.clone();
        self.banner(ui, cm10, now);
        let top = ui.frame(body, &th.paper);
        ui.pack(top, Pack::default().fill(Fill::X));
        ui.ws[top].gw_cols = vec![0, 1];
        let left = ui.frame(top, &th.paper);
        ui.grid(left, Grid { row: 0, col: 0, sticky: "nsew", padx: (0, 12), ..Default::default() });
        let right = ui.frame(top, &th.paper);
        ui.grid(right, Grid { row: 0, col: 1, sticky: "nsew", ..Default::default() });
        self.map_card(ui, left, now);
        self.day_card(ui, right, cm10, now);
        self.daily_card(ui, body, now);
        let low = ui.frame(body, &th.paper);
        ui.pack(low, Pack::default().fill(Fill::X).pady(12, 0));
        ui.ws[low].gw_cols = vec![1, 1];
        ui.ws[low].gu_cols = vec!["g", "g"];
        let a = ui.frame(low, &th.paper);
        ui.grid(a, Grid { row: 0, col: 0, sticky: "nsew", padx: (0, 6), ..Default::default() });
        let b = ui.frame(low, &th.paper);
        ui.grid(b, Grid { row: 0, col: 1, sticky: "nsew", padx: (6, 0), ..Default::default() });
        self.weight_card(ui, a);
        self.food_card(ui, b, now);
    }

    // ---- the dusk banner ----
    fn banner(&mut self, ui: &mut Ui, cm10: i32, now: f64) {
        let cv = ui.canvas(self.body, &self.th.paper, cm10, 92);
        canvas_of(ui, cv);
        ui.pack(cv, Pack::default().fill(Fill::X).pady(0, 12));
        self.banner = cv;
        self.banner_w = 0;
        self.anims.push((Anim::Twinkle { cv }, now + 0.42, 1));
    }

    fn banner_draw(&mut self, ui: &mut Ui) {
        let wv = ui.ws[self.banner].w;
        if wv == self.banner_w {
            return;
        }
        self.banner_w = wv;
        let (jp, serif, fu) = (self.d.jp.clone(), self.d.serif.clone(), self.f.ui.clone());
        let t = self.d.gym_today();
        let name = t.map(|i| (GymData::s(&self.d.day(i)["big"]), GymData::s(&self.d.day(i)["kanji"]),
                              GymData::s(&self.d.day(i)["name"])));
        let cv = canvas_of(ui, self.banner);
        *cv = Canvas::default();
        let h = 92.0;
        let w = 300.max(wv) as f64;
        let (top, bot) = ([24.0, 22.0, 58.0], [196.0, 84.0, 70.0]);
        for y in 0..92 {
            let f = y as f64 / 91.0;
            let c: Vec<i64> = (0..3).map(|j| (top[j] + (bot[j] - top[j]) * f.powf(1.6)) as i64).collect();
            cv.line(vec![0.0, y as f64, w, y as f64]).fill(&format!("#{:02x}{:02x}{:02x}", c[0], c[1], c[2]));
        }
        for k in 0..26i64 {
            let fx = ((k * 97) % 1000) as f64 / 1000.0;
            let (y, r) = (6 + (k * 53) % 46, 1 + k % 2);
            cv.text((fx * w) as i64 as f64, y as f64, if r == 2 { "\u{2726}" } else { "\u{b7}" })
                .fill(if r == 2 { "#fff2d0" } else { "#c9c4ff" }).font(Font::new(&fu, if r == 2 { 7 } else { 10 }, false))
                .tags(&["star"]);
        }
        let mx = w - 150.0;
        cv.oval(mx - 22.0, 8.0, mx + 22.0, 52.0).fill("#ffe9b8").outline("");
        cv.oval(mx - 10.0, 2.0, mx + 32.0, 46.0).fill("#2a1d4a").outline("");
        let mut hill = vec![0.0, h];
        let wi = w as i64;
        let mut x = 0;
        while x < wi + 40 {
            let k = x / 40;
            hill.extend([x as f64, h - 16.0 - 8.0 * ((k % 3 == 1) as i64 as f64) - 5.0 * ((k % 4 == 2) as i64 as f64)]);
            x += 40;
        }
        hill.extend([w, h]);
        cv.poly(hill).fill("#3a1730").outline("").smooth();
        let tx = w - 70.0;
        cv.rect(tx - 26.0, 38.0, tx - 20.0, h - 10.0).fill("#e2432f").outline("");
        cv.rect(tx + 20.0, 38.0, tx + 26.0, h - 10.0).fill("#e2432f").outline("");
        cv.poly(vec![tx - 40.0, 30.0, tx + 40.0, 30.0, tx + 34.0, 38.0, tx - 34.0, 38.0]).fill("#e2432f").outline("");
        cv.rect(tx - 38.0, 26.0, tx + 38.0, 30.0).fill("#1d1020").outline("");
        cv.rect(tx - 30.0, 46.0, tx + 30.0, 50.0).fill("#e2432f").outline("");
        for (k, fx) in [0.42, 0.55, 0.66].into_iter().enumerate() {
            let x = (fx * w) as i64 as f64;
            let y = 40.0 + 9.0 * (k % 2) as f64;
            cv.oval(x - 7.0, y - 7.0, x + 7.0, y + 7.0).fill("#6d7dff").outline("").tags(&["ff"]);
            cv.oval(x - 3.0, y - 3.0, x + 3.0, y + 3.0).fill("#e4ecff").outline("").tags(&["ff"]);
        }
        match name {
            None => {
                cv.text(22.0, 30.0, "\u{4eca}\u{65e5}\u{306f}\u{4f55}\u{306e}\u{65e5}\u{ff1f}").fill("#ffe9b8")
                    .font(Font::new(&jp, 17, false)).anchor("w");
                cv.text(22.0, 60.0, &spaced("WHICH DAY IS TODAY?  PICK IT BELOW")).fill("#f3d6c8")
                    .font(Font::new(&serif, 8, false)).anchor("w");
            }
            Some((big, kanji, nm)) => {
                cv.text(22.0, 32.0, &format!("{big}  {kanji}\u{306e}\u{65e5}")).fill("#ffe9b8").font(Font::new(&jp, 18, false))
                    .anchor("w");
                cv.text(22.0, 62.0, &spaced(&format!("TODAY \u{b7} {}", nm.to_uppercase()))).fill("#f3d6c8")
                    .font(Font::new(&serif, 8, false)).anchor("w");
            }
        }
    }

    // ---- meters ----
    /// gym_foxfire(parent, frac, n, size): a row of foxfire flames, lit toward the goal (gold once it's reached)
    fn foxfire(&mut self, ui: &mut Ui, parent: usize, frac: f64, n: i64, size: f64, now: f64) -> usize {
        let bg = self.th.card.clone();
        let (w, h) = (size + 4.0, size + 8.0);
        let id = ui.canvas(parent, &bg, (n as f64 * w) as i32, h as i32);
        let cv = canvas_of(ui, id);
        let lit = frac.max(0.0) * n as f64;
        let over = frac >= 1.0;
        let (outer, inner) = if over { ("#ffc23d", "#fff5cc") } else { ("#5b72ff", "#dbe6ff") };
        let mut flames = vec![];
        for k in 0..n {
            let (cx, b) = (k as f64 * w + w / 2.0, h - 2.0);
            let amt = (lit - k as f64).clamp(0.0, 1.0);
            let fh = if amt > 0.0 { size * (0.55 + 0.45 * amt) } else { size * 0.75 };
            let fw = size * 0.62;
            if amt <= 0.0 {
                cv.poly(flame(cx, b, fh, fw, 0.0)).fill("").outline(&mixhex("#7a86c8", &bg, 0.55)).smooth().width(1.0);
                continue;
            }
            if over || amt >= 1.0 {
                cv.oval(cx - fw, b - fh * 0.9, cx + fw, b + 1.0).fill(&mixhex(outer, &bg, 0.82)).outline("");
            }
            cv.poly(flame(cx, b, fh, fw, 0.0)).fill(&mixhex(outer, &bg, 0.5 * (1.0 - amt))).outline("").smooth();
            let o = cv.last_id();
            cv.poly(flame(cx, b, fh * 0.55, fw * 0.5, 0.0)).fill(inner).outline("").smooth();
            flames.push((o, cx, b, fh, fw));
        }
        if !flames.is_empty() {
            self.anims.push((Anim::Flick { cv: id, flames }, now + 0.23, 1));
        }
        id
    }

    /// gym_brush_bar(parent, frac, w): an ink-brush stroke from vermilion to gold
    fn brush_bar(&self, ui: &mut Ui, parent: usize, frac: f64, w: i32) -> usize {
        let bg = self.th.card.clone();
        let h = 14.0;
        let wf = w as f64;
        let id = ui.canvas(parent, &bg, w, 14);
        let fu = self.f.ui.clone();
        let line = mixhex(&self.th.line, &bg, 0.35);
        let cv = canvas_of(ui, id);
        cv.line(vec![3.0, h / 2.0, wf - 3.0, h / 2.0]).fill(&line).width(8.0).cap("round");
        let fx = 3.0 + (wf - 6.0) * frac.clamp(0.0, 1.0);
        if fx > 5.0 {
            let (a, b) = ([236.0, 78.0, 58.0], [255.0, 200.0, 90.0]);
            let steps = 2.max((fx - 3.0) as i64);
            for x in 0..steps {
                let f = x as f64 / steps as f64;
                let c: Vec<i64> = (0..3).map(|j| (a[j] + (b[j] - a[j]) * f) as i64).collect();
                let t = 9.0 - 3.0 * f + if x % 7 == 0 { 1.0 } else { 0.0 };
                cv.line(vec![3.0 + x as f64, h / 2.0 - t / 2.0, 3.0 + x as f64, h / 2.0 + t / 2.0])
                    .fill(&format!("#{:02x}{:02x}{:02x}", c[0], c[1], c[2]));
            }
            cv.text(fx + 1.0, h / 2.0, "\u{2726}").fill("#fff0c4").font(Font::new(&fu, 7, false));
        }
        id
    }

    // ---- muscle map ----
    fn photos(&mut self, day: usize) -> (Vec<Rc<Photo>>, Vec<Rc<Photo>>) {
        let dn = self.d.done_regions(day);
        let key = (day, dn.clone());
        if let Some(p) = self.photos.get(&key) {
            return p.clone();
        }
        let tg: Vec<String> = self.d.day(day)["targets"].as_array().unwrap().iter().map(GymData::s).collect();
        let mk = |view: &str| -> Vec<Rc<Photo>> {
            body_frames(view, &tg, ACC, &dn).into_iter().map(|px| {
                let img = zenko_art::raster::Image { w: BODY_W, h: BODY_H, px };
                let im = zenko_art::raster::nearest(&img, 2);
                Rc::new(Photo { w: im.w, h: im.h, px: im.px })
            }).collect()
        };
        let v = (mk("front"), mk("back"));
        if self.photos.len() >= 6 {
            self.photos.clear();
        }
        self.photos.insert(key, v.clone());
        v
    }

    fn map_card(&mut self, ui: &mut Ui, parent: usize, now: f64) {
        let (th, f) = (self.th.clone(), Faces { ui: self.f.ui.clone(), title: self.f.title.clone() });
        let c = hub_card(ui, parent, Some("\u{7b4b}\u{8089} \u{b7} Muscle map"), "round", &th, &f);
        ui.pack(c, Pack::default().fill(Fill::Both).expand());
        let bg = th.card.clone();
        let (jp, serif) = (self.d.jp.clone(), self.d.serif.clone());
        let sel = ui.frame(c, &bg);
        ui.pack(sel, Pack::default().pady(2, 4));
        let today = self.d.gym_today();
        for i in 0..4 {
            let on = i == self.view;
            let kanji = GymData::s(&self.d.day(i)["kanji"]);
            let lab = ui.label(sel, &format!("{kanji}\u{306e}\u{65e5}"), &bg, if on { &th.gold } else { &th.grey },
                               Font::new(&jp, 12, false));
            ui.ws[lab].padx = 10;
            ui.pack(lab, Pack::default().side(Side::Left));
            Self::hand(ui, &[lab]);
            self.click(&[lab], &format!("gym_view:{i}"));
            if Some(i) == today {
                let l = ui.label(sel, "\u{4eca}\u{65e5}", &bg, &th.acc, Font::new(&jp, 8, false));
                ui.pack(l, Pack::default().side(Side::Left).padx(0, 6));
            }
        }
        let (w, h) = ((2 * BODY_W * 2 + 60) as i32, (BODY_H * 2 + 44) as i32);
        let cv = ui.canvas(c, &bg, w, h);
        canvas_of(ui, cv);
        ui.pack(cv, Pack::default().padx(14, 14));
        if self.moon.is_none() {
            self.moon = Some(px_photo(180, 180, &glow_art(90, (150, 170, 255), 0.22), 1, false));
        }
        let (front, back) = self.photos(self.view);
        let moon = self.moon.clone().unwrap();
        let fu = f.ui.clone();
        let (wf, hf) = (w as f64, h as f64);
        let k = canvas_of(ui, cv);
        k.image((w / 2) as f64, (h / 2 - 30) as f64, moon);
        let r = 150.0;
        let (cx, cy) = ((w / 2) as f64, (h / 2 - 30) as f64);
        k.arc(cx - r, cy - r, cx + r, cy + r, 110.0, 320.0, "arc").outline(&mixhex(&th.gold, &bg, 0.55)).width(2.0);
        k.arc(cx - r + 4.0, cy - r + 4.0, cx + r - 4.0, cy + r - 4.0, 120.0, 250.0, "arc").outline(&mixhex(&th.gold, &bg, 0.8))
            .width(1.0);
        for s in 0..14i32 {
            let sx = 8 + (s * 131) % (w - 16);
            let sy = 10 + (s * 71) % (h - 60);
            if (sx - w / 2).abs() < 120 && 60 < sy && sy < h - 90 {
                continue;
            }
            k.text(sx as f64, sy as f64, if s % 3 == 0 { "\u{2726}" } else { "\u{2727}" })
                .fill(&mixhex(&th.gold, &bg, 0.35 + 0.15 * (s % 3) as f64)).font(Font::new(&fu, 7 + 2 * (s % 2), false))
                .tags(&["star"]);
        }
        let (x1, x2) = (16.0, (16 + BODY_W * 2 + 28) as f64);
        k.image(x1, 8.0, front[0].clone()).anchor("nw");
        let fi = k.last_id();
        k.image(x2, 8.0, back[0].clone()).anchor("nw");
        let bi = k.last_id();
        for (x, t) in [(x1 + BODY_W as f64, "\u{524d}"), (x2 + BODY_W as f64, "\u{5f8c}")] {
            k.text(x, hf - 18.0, t).fill(&mixhex(&th.gold, &bg, 0.2)).font(Font::new(&jp, 12, false));
        }
        let _ = wf;
        let d = self.d.day(self.view);
        let names: Vec<String> = d["targets"].as_array().unwrap().iter()
            .map(|t| GymData::s(&gym_data()["MUSCLE_NAMES"][t.as_str().unwrap()])).collect();
        let names = if names.is_empty() { "Rest \u{b7} every muscle recovers and grows today".to_string() } else {
            names.join(" \u{b7} ") };
        let l = ui.label(c, &names, &bg, &th.ink, italic(&serif, 11));
        wrapl(ui, l, w);
        ui.pack(l, Pack::default().pady(2, 14));
        self.anims.push((Anim::Figures { cv, front: fi, back: bi, day: self.view }, now + 0.26, 1));
    }

    // ---- today's training ----
    fn pick_card(&mut self, ui: &mut Ui, parent: usize) {
        let (th, f) = (self.th.clone(), Faces { ui: self.f.ui.clone(), title: self.f.title.clone() });
        let c = hub_card(ui, parent, Some("\u{4eca}\u{65e5} \u{b7} Which day is today?"), "round", &th, &f);
        ui.pack(c, Pack::default().fill(Fill::Both).expand());
        let bg = th.card.clone();
        let (jp, serif) = (self.d.jp.clone(), self.d.serif.clone());
        let l = ui.label(c, "Tap today's training. I'll move to the next day each time you press Finish, and you can change \
it here whenever the order slips.", &bg, &th.grey, italic(&serif, 10));
        ui.set_anchor(l, Anchor::W);
        justify_left(ui, l, 340);
        ui.pack(l, Pack::default().fill(Fill::X).padx(16, 16).pady(0, 10));
        for k in 0..4 {
            let d = self.d.day(k).clone();
            let rb = mixhex(&th.gold, &bg, 0.9);
            let row = ui.frame(c, &rb);
            ui.pack(row, Pack::default().fill(Fill::X).padx(16, 16).pady(4, 4));
            let big = ui.label(row, &GymData::s(&d["big"]), &rb, if k == 3 { "#7d8cff" } else { "#ec4e3a" },
                               Font::new(&jp, 24, false));
            ui.pack(big, Pack::default().side(Side::Left).padx(12, 12).pady(6, 6));
            let tx = ui.frame(row, &rb);
            ui.pack(tx, Pack::default().side(Side::Left).fill(Fill::X).expand());
            let a = label_w(ui, tx, &spaced(&format!("{}\u{306e}\u{65e5} \u{b7} {}", GymData::s(&d["kanji"]),
                                                     if k == 3 { "REST DAY".to_string() } else { format!("DAY {}", k + 1) })),
                            &rb, &th.grey, &serif, 8, false);
            ui.pack(a, Pack::default().anchor(Anchor::W));
            let b = label_w(ui, tx, &GymData::s(&d["name"]), &rb, &th.ink, &serif, 13, false);
            ui.pack(b, Pack::default().anchor(Anchor::W));
            Self::hand(ui, &[row, big, tx, a, b]);
            self.click(&[row, big, tx, a, b], &format!("gym_set:{k}"));
        }
        let sp = ui.frame(c, &bg);
        ui.ws[sp].opt_h = 12;
        ui.pack(sp, Pack::default());
    }

    /// gym_day_chips(c, i): 今日は [胸 Chest] [背 Back] [脚 Legs] [休 Rest]
    fn day_chips(&mut self, ui: &mut Ui, c: usize, i: usize) {
        let th = self.th.clone();
        let bg = th.card.clone();
        let (jp, serif) = (self.d.jp.clone(), self.d.serif.clone());
        let ch = ui.frame(c, &bg);
        ui.pack(ch, Pack::default().fill(Fill::X).padx(16, 16).pady(0, 6));
        let l = ui.label(ch, "\u{4eca}\u{65e5}\u{306f}", &bg, &th.grey, Font::new(&jp, 9, false));
        ui.pack(l, Pack::default().side(Side::Left).padx(0, 4));
        let today = self.d.gym_today();
        for k in 0..4 {
            let on = Some(k) == today;
            let big = GymData::s(&self.d.day(k)["big"]);
            let cbg = if on { if k == 3 { "#5b6cd8".to_string() } else { "#ec4e3a".to_string() } } else { mixhex(&th.gold, &bg, 0.88) };
            let chip = ui.label(ch, &format!(" {big} {} ", ["Chest", "Back", "Legs", "Rest"][k]), &cbg,
                                if on { "#fff6ec" } else { &th.ink }, Font::new(&serif, 9, false));
            ui.ws[chip].padx = 4;
            ui.pack(chip, Pack::default().side(Side::Left).padx(2, 2));
            Self::hand(ui, &[chip]);
            if !on {
                self.click(&[chip], &format!("gym_set:{k}"));
            }
        }
        if Some(i) != today {
            let sl = ui.label(ch, "set as today \u{2192}", &bg, &th.acc, italic(&serif, 9));
            ui.pack(sl, Pack::default().side(Side::Right));
            Self::hand(ui, &[sl]);
            self.click(&[sl], &format!("gym_set:{i}"));
        }
    }

    fn rest_card(&mut self, ui: &mut Ui, parent: usize, now: f64) {
        let (th, f) = (self.th.clone(), Faces { ui: self.f.ui.clone(), title: self.f.title.clone() });
        let c = hub_card(ui, parent, Some("\u{4f11}\u{307f} \u{b7} Rest day"), "round", &th, &f);
        ui.pack(c, Pack::default().fill(Fill::Both).expand());
        let bg = th.card.clone();
        let (jp, serif) = (self.d.jp.clone(), self.d.serif.clone());
        let hd = ui.frame(c, &bg);
        ui.pack(hd, Pack::default().fill(Fill::X).padx(16, 16).pady(4, 6));
        let l = ui.label(hd, "\u{4f11}", &bg, "#8e9bff", Font::new(&jp, 34, false));
        ui.pack(l, Pack::default().side(Side::Left).padx(0, 14));
        let tb = ui.frame(hd, &bg);
        ui.pack(tb, Pack::default().side(Side::Left).fill(Fill::X).expand());
        let today = self.d.gym_today() == Some(3);
        let l = ui.label(tb, &spaced(&format!("{}REST DAY", if today { "TODAY \u{b7} " } else { "" })), &bg, &th.grey,
                         Font::new(&serif, 8, false));
        ui.pack(l, Pack::default().anchor(Anchor::W));
        let l = label_w(ui, tb, "Rest and recover", &bg, &th.ink, &serif, 15, false);
        ui.pack(l, Pack::default().anchor(Anchor::W));
        self.day_chips(ui, c, 3);
        let (w, h) = (340.0, 96.0);
        let id = ui.canvas(c, &bg, 340, 96);
        ui.pack(id, Pack::default().padx(16, 16).pady(4, 6).anchor(Anchor::W));
        let night = mixhex("#1c1840", &bg, 0.25);
        let fu = self.f.ui.clone();
        let cv = canvas_of(ui, id);
        cv.rect(0.0, 0.0, w, h).fill(&night).outline("");
        for k in 0..16i64 {
            cv.text((10 + (k * 83) % (340 - 20)) as f64, (8 + (k * 37) % (96 - 30)) as f64,
                    if k % 4 == 0 { "\u{2726}" } else { "\u{b7}" }).fill("#d9d4ff")
                .font(Font::new(&fu, if k % 4 == 0 { 7 } else { 10 }, false)).tags(&["star"]);
        }
        cv.oval(w - 86.0, 14.0, w - 40.0, 60.0).fill("#ffe9b8").outline("");
        cv.oval(w - 74.0, 6.0, w - 30.0, 52.0).fill(&night).outline("");
        cv.poly(vec![0.0, h, 0.0, h - 18.0, 60.0, h - 26.0, 140.0, h - 16.0, 220.0, h - 28.0, w, h - 14.0, w, h])
            .fill("#2a2150").outline("").smooth();
        let mut zs = vec![];
        for k in 0..3 {
            cv.text(70.0 + 22.0 * k as f64, 52.0 - 12.0 * k as f64, "z").fill("#b9c2ff").font(italic(&serif, 10 + 3 * k));
            zs.push(cv.last_id());
        }
        self.anims.push((Anim::Drift { cv: id, zs, night }, now + 0.5, 1));
        for t in ["A gentle walk or a light stretch is plenty.", "Sleep 8\u{2013}10 hours: this is when muscle is built.",
                  "Still hit your protein and water today."] {
            let r = ui.frame(c, &bg);
            ui.pack(r, Pack::default().fill(Fill::X).padx(16, 16).pady(2, 2));
            let l = ui.label(r, "\u{263e}", &bg, "#8e9bff", Font::new(&fu, 10, false));
            ui.pack(l, Pack::default().side(Side::Left).padx(0, 8));
            let l = label_w(ui, r, t, &bg, &th.ink, &serif, 11, false);
            ui.pack(l, Pack::default().side(Side::Left));
        }
        let br = ui.frame(c, &bg);
        ui.pack(br, Pack::default().fill(Fill::X).padx(16, 16).pady(10, 14));
        let b = tt_btn(ui, br, "Rested  \u{2192}  day 1 next", "gym_finish:3", Some("#7d8cff"), Some(&th.onacc), 10, 14, 3, &th, &f);
        ui.pack(b, Pack::default().side(Side::Left));
    }

    fn day_card(&mut self, ui: &mut Ui, parent: usize, _cm10: i32, now: f64) {
        if self.d.gym_today().is_none() {
            return self.pick_card(ui, parent);
        }
        let i = self.view;
        if i == 3 {
            return self.rest_card(ui, parent, now);
        }
        let (th, f) = (self.th.clone(), Faces { ui: self.f.ui.clone(), title: self.f.title.clone() });
        let d = self.d.day(i).clone();
        let c = hub_card(ui, parent, Some("\u{4eca}\u{65e5} \u{b7} Training"), "round", &th, &f);
        ui.pack(c, Pack::default().fill(Fill::Both).expand());
        let bg = th.card.clone();
        let (jp, serif) = (self.d.jp.clone(), self.d.serif.clone());
        let hd = ui.frame(c, &bg);
        ui.pack(hd, Pack::default().fill(Fill::X).padx(16, 16).pady(4, 6));
        let l = ui.label(hd, &GymData::s(&d["big"]), &bg, &mixhex(&th.gold, &bg, 0.05), Font::new(&jp, 34, false));
        ui.pack(l, Pack::default().side(Side::Left).padx(0, 14));
        let tb = ui.frame(hd, &bg);
        ui.pack(tb, Pack::default().side(Side::Left).fill(Fill::X).expand());
        let today = Some(i) == self.d.gym_today();
        let l = ui.label(tb, &spaced(&format!("{}DAY {} OF 3", if today { "TODAY \u{b7} " } else { "" }, i + 1)), &bg,
                         &th.grey, Font::new(&serif, 8, false));
        ui.pack(l, Pack::default().anchor(Anchor::W));
        let l = label_w(ui, tb, &GymData::s(&d["name"]), &bg, &th.ink, &serif, 15, false);
        justify_left(ui, l, 340);
        ui.pack(l, Pack::default().anchor(Anchor::W).fill(Fill::X));
        self.day_chips(ui, c, i);
        let ln = ui.frame(c, &mixhex(&th.line, &bg, 0.25));
        ui.ws[ln].opt_h = 1;
        ui.pack(ln, Pack::default().fill(Fill::X).padx(16, 16).pady(2, 8));
        let key = format!("{}|{}", self.d.today_iso, i);
        let done: Vec<String> = self.d.gym["checks"][key.as_str()].as_array().map(|a| a.iter().map(GymData::s).collect())
            .unwrap_or_default();
        let groups = d["groups"].as_array().unwrap().clone();
        for g in &groups {
            let (name, kan) = (GymData::s(&g[0]), GymData::s(&g[1]));
            let on = done.contains(&name);
            let row = ui.frame(c, &bg);
            ui.pack(row, Pack::default().fill(Fill::X).padx(16, 16).pady(3, 3));
            let bx = ui.canvas(row, &bg, 28, 28);
            ui.pack(bx, Pack::default().side(Side::Left).padx(0, 12));
            let cv = canvas_of(ui, bx);
            if on {
                cv.oval(2.0, 2.0, 26.0, 26.0).fill("#d8362a").outline(&mixhex("#d8362a", "#000000", 0.25)).width(1.0);
                cv.oval(5.0, 5.0, 23.0, 23.0).outline("#f6b4a6").width(1.0);
                cv.text(14.0, 14.0, "\u{6e08}").fill("#fff6ec").font(Font::new(&jp, 10, true));
            } else {
                cv.oval(4.0, 4.0, 24.0, 24.0).outline(&mixhex("#d8362a", &bg, 0.55)).width(2.0).dash(&[3, 2]);
            }
            let nl = label_w(ui, row, &name, &bg, if on { &th.grey } else { &th.ink }, &serif, 14, false);
            ui.pack(nl, Pack::default().side(Side::Left));
            let kl = ui.label(row, &kan, &bg, &mixhex(&th.gold, &bg, if on { 0.5 } else { 0.1 }), Font::new(&jp, 12, false));
            ui.pack(kl, Pack::default().side(Side::Right));
            Self::hand(ui, &[row, bx, nl, kl]);
            self.click(&[row, bx, nl, kl], &format!("gym_tog:{i}:{name}"));
        }
        let n_done = groups.iter().filter(|g| done.contains(&GymData::s(&g[0]))).count();
        let l = ui.label(c, &format!("{n_done} of {} trained today", groups.len()), &bg, &th.grey, italic(&serif, 10));
        ui.set_anchor(l, Anchor::W);
        ui.pack(l, Pack::default().fill(Fill::X).padx(16, 16).pady(10, 4));
        let br = ui.frame(c, &bg);
        ui.pack(br, Pack::default().fill(Fill::X).padx(16, 16).pady(4, 4));
        let b = tt_btn(ui, br, &format!("Finish day {}  \u{2192}", i + 1), &format!("gym_finish:{i}"), Some(&th.acc),
                       Some(&th.onacc), 10, 14, 3, &th, &f);
        ui.pack(b, Pack::default().side(Side::Left));
        if let Some(last) = self.d.gym["log"].as_array().and_then(|l| l.last()) {
            let ds = GymData::s(&last["date"]);
            let when = match crate::plan::parse_iso(&ds) {
                Some(dn) => {
                    let (_, m, dd) = crate::lifechat::ymd(dn);
                    const MON: [&str; 12] = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
                    format!("{} {dd:02} {}", ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"][crate::lifechat::weekday(dn)],
                            MON[(m - 1) as usize])
                }
                None => ds.clone(),
            };
            let t = if last["day"].as_i64() == Some(3) { format!("Last: rest on {when}") } else {
                format!("Last: day {} on {when}", last["day"].as_i64().unwrap_or(0) + 1) };
            let l = ui.label(br, &t, &bg, &th.grey, Font::new(&f.ui, 8, false));
            ui.pack(l, Pack::default().side(Side::Left).padx(10, 10));
        }
        let sp = ui.frame(c, &bg);
        ui.ws[sp].opt_h = 10;
        ui.pack(sp, Pack::default());
    }

    fn plan(&self) -> Option<((i64, i64), Option<(i64, i64)>)> {
        let g = &self.d.gym;
        let kg = g["weights"].as_array().and_then(|w| w.last()).and_then(|w| w["kg"].as_f64());
        let p = &g["profile"];
        let age = p["age"].as_i64().or_else(|| p["age"].as_f64().map(|v| v as i64)).filter(|a| *a != 0).unwrap_or(17);
        calorie_plan(kg, p["height"].as_f64(), age, p["sex"].as_str().unwrap_or(""))
    }

    // ---- protein, water, sleep ----
    fn daily_card(&mut self, ui: &mut Ui, parent: usize, now: f64) {
        let (th, f) = (self.th.clone(), Faces { ui: self.f.ui.clone(), title: self.f.title.clone() });
        let g = self.d.gym.clone();
        let today = self.d.today_iso.clone();
        let c = hub_card(ui, parent, Some("\u{65e5}\u{8ab2} \u{b7} Daily"), "round", &th, &f);
        ui.pack(c, Pack::default().fill(Fill::X).pady(12, 0));
        let bg = th.card.clone();
        let (jp, serif) = (self.d.jp.clone(), self.d.serif.clone());
        let row = ui.frame(c, &bg);
        ui.pack(row, Pack::default().fill(Fill::X).padx(16, 16).pady(0, 14));
        ui.ws[row].gw_cols = vec![1, 1, 1];
        ui.ws[row].gu_cols = vec!["d", "d", "d"];
        let col = |ui: &mut Ui, k: usize, kan: &str, title: &str| -> usize {
            let fr = ui.frame(row, &bg);
            ui.grid(fr, Grid { row: 0, col: k, sticky: "nsew", padx: (if k == 0 { 0 } else { 12 }, 0), ..Default::default() });
            let hd = ui.frame(fr, &bg);
            ui.pack(hd, Pack::default().fill(Fill::X));
            let l = ui.label(hd, kan, &bg, &mixhex(&th.gold, &bg, 0.1), Font::new(&jp, 12, false));
            ui.pack(l, Pack::default().side(Side::Left).padx(0, 8));
            let l = ui.label(hd, title, &bg, &th.ink, Font::new(&serif, 12, false));
            ui.pack(l, Pack::default().side(Side::Left));
            fr
        };
        // protein
        let fr = col(ui, 0, "\u{86cb}\u{767d}", "Protein");
        let plan = self.plan();
        let got = zenko_sound::music::py_sum(self.d.food_today().iter().map(|i| i["p"].as_f64().unwrap_or(0.0)));
        let goal = plan.map(|p| p.0.0).unwrap_or(0);
        let ff = self.foxfire(ui, fr, if goal != 0 { got / goal as f64 } else { 0.0 }, 10, 22.0, now);
        ui.pack(ff, Pack::default().anchor(Anchor::W).pady(4, 2));
        let gr = fmt_i(got.round_ties_even() as i64);
        let t = if goal != 0 {
            format!("{gr} / {goal} g{}", if got >= goal as f64 { " \u{b7} every flame lit~".to_string() } else {
                format!(" \u{b7} each flame is {} g", 1.max((goal as f64 / 10.0).round_ties_even() as i64)) })
        } else {
            format!("{gr} g today \u{b7} log your weight for a goal")
        };
        let l = ui.label(fr, &t, &bg, &th.grey, italic(&serif, 10));
        ui.pack(l, Pack::default().anchor(Anchor::W));
        // water
        let fr = col(ui, 1, "\u{6c34}", "Water");
        let cups = g["water"][today.as_str()].as_i64().or_else(|| g["water"][today.as_str()].as_f64().map(|v| v as i64)).unwrap_or(0);
        let cr = ui.frame(fr, &bg);
        ui.pack(cr, Pack::default().anchor(Anchor::W).pady(6, 2));
        for n in 0..WATER_GOAL {
            let cc = ui.canvas(cr, &bg, 20, 24);
            ui.pack(cc, Pack::default().side(Side::Left).padx(1, 1));
            let full = n < cups;
            let cv = canvas_of(ui, cc);
            cv.poly(vec![3.0, 4.0, 17.0, 4.0, 15.0, 21.0, 5.0, 21.0]).fill(if full { "#5fb6ff" } else { &th.field })
                .outline(&if full { mixhex("#9fd4ff", &bg, 0.2) } else { mixhex(&th.line, &bg, 0.1) });
            if full {
                cv.line(vec![6.0, 8.0, 6.0, 17.0]).fill("#cfeaff");
            }
            Self::hand(ui, &[cc]);
            self.click(&[cc], &format!("gym_water:{n}"));
        }
        let l = ui.label(fr, &format!("{cups} / {WATER_GOAL} glasses \u{b7} tap one per drink"), &bg, &th.grey, italic(&serif, 10));
        ui.pack(l, Pack::default().anchor(Anchor::W));
        // sleep
        let fr = col(ui, 2, "\u{7720}", "Sleep");
        let sl = g["sleep"][today.as_str()].as_i64().unwrap_or(0);
        let sr = ui.frame(fr, &bg);
        ui.pack(sr, Pack::default().anchor(Anchor::W).pady(6, 2));
        for n in 1..6 {
            let mc = ui.canvas(sr, &bg, 24, 24);
            ui.pack(mc, Pack::default().side(Side::Left).padx(1, 1));
            let on = n <= sl;
            let cv = canvas_of(ui, mc);
            cv.oval(3.0, 3.0, 21.0, 21.0).fill(if on { &th.gold } else { &th.field })
                .outline(&mixhex(&th.gold, &bg, if on { 0.3 } else { 0.6 }));
            if on {
                cv.oval(9.0, 1.0, 25.0, 17.0).fill(&bg).outline("");
            }
            Self::hand(ui, &[mc]);
            self.click(&[mc], &format!("gym_sleep:{n}"));
        }
        let week: Vec<i64> = g["sleep"].as_object().map(|o| o.iter().filter_map(|(d, v)| {
            let dn = crate::plan::parse_iso(d)?;
            let v = v.as_i64().unwrap_or(0);
            (self.d.today - dn < 7 && v != 0).then_some(v)
        }).collect()).unwrap_or_default();
        let mut txt = if sl != 0 { format!("Last night: {}", ["", "poor", "meh", "okay", "good", "great"][sl as usize]) } else {
            "How did you sleep last night?".to_string() };
        if week.len() >= 3 {
            txt += &format!(" \u{b7} week avg {:.1}", week.iter().sum::<i64>() as f64 / week.len() as f64);
        }
        let l = ui.label(fr, &txt, &bg, &th.grey, italic(&serif, 10));
        ui.pack(l, Pack::default().anchor(Anchor::W));
    }

    // ---- weight ----
    fn weight_card(&mut self, ui: &mut Ui, parent: usize) {
        let (th, f) = (self.th.clone(), Faces { ui: self.f.ui.clone(), title: self.f.title.clone() });
        let g = self.d.gym.clone();
        let p = &g["profile"];
        let c = hub_card(ui, parent, Some("\u{4f53}\u{91cd} \u{b7} Weight"), "round", &th, &f);
        ui.pack(c, Pack::default().fill(Fill::Both).expand());
        let bg = th.card.clone();
        let serif = self.d.serif.clone();
        let fu = f.ui.clone();
        let row = ui.frame(c, &bg);
        ui.pack(row, Pack::default().fill(Fill::X).padx(16, 16).pady(2, 6));
        let e = tt_entry(ui, row, "", 7, &th, &f);
        ui.pack(e, Pack::default().side(Side::Left));
        self.wentry = e;
        let l = ui.label(row, "kg", &bg, &th.grey, Font::new(&fu, 9, false));
        ui.pack(l, Pack::default().side(Side::Left).padx(4, 8));
        let msg = ui.label(c, "", &bg, &th.grey, italic(&serif, 10));
        ui.set_anchor(msg, Anchor::W);
        justify_left(ui, msg, 320);
        self.wmsg = msg;
        let b = tt_btn(ui, row, "Log", "gym_wlog", Some(&th.start), None, 9, 10, 1, &th, &f);
        ui.pack(b, Pack::default().side(Side::Left));
        let all = g["weights"].as_array().cloned().unwrap_or_default();
        let w: Vec<Value> = all[all.len().saturating_sub(40)..].to_vec();
        let (cw, ch) = (330.0, 110.0);
        let id = ui.canvas(c, &bg, 330, 110);
        ui.pack(id, Pack::default().padx(16, 16).pady(2, 2).anchor(Anchor::W));
        let cv = canvas_of(ui, id);
        let fj = mixhex("#5b6fb8", &bg, 0.84);
        cv.poly(vec![30.0, ch - 14.0, cw * 0.42, 22.0, cw * 0.58, 22.0, cw - 30.0, ch - 14.0]).fill(&fj).outline("");
        cv.poly(vec![cw * 0.42, 22.0, cw * 0.58, 22.0, cw * 0.63, 36.0, cw * 0.56, 32.0, cw * 0.5, 40.0, cw * 0.45, 32.0,
                     cw * 0.37, 36.0]).fill(&mixhex("#ffffff", &bg, 0.4)).outline("");
        cv.oval(cw * 0.78 - 9.0, 12.0, cw * 0.78 + 9.0, 30.0).fill(&mixhex("#ec4e3a", &bg, 0.6)).outline("");
        let mut mtext = String::new();
        if w.len() >= 2 {
            let vals: Vec<f64> = w.iter().map(|x| x["kg"].as_f64().unwrap_or(0.0)).collect();
            let lo = vals.iter().copied().fold(f64::INFINITY, f64::min) - 0.5;
            let hi = vals.iter().copied().fold(f64::NEG_INFINITY, f64::max) + 0.5;
            let pts: Vec<(f64, f64)> = vals.iter().enumerate().map(|(k, v)| {
                (14.0 + (cw - 28.0) * k as f64 / (w.len() - 1) as f64, 10.0 + (ch - 30.0) * (1.0 - (v - lo) / (hi - lo)))
            }).collect();
            for gy in 0..3 {
                let yy = 10.0 + (ch - 30.0) * gy as f64 / 2.0;
                cv.line(vec![10.0, yy, cw - 10.0, yy]).fill(&mixhex(&th.line, &bg, 0.5)).dash(&[2, 4]);
                cv.text(cw - 8.0, yy - 6.0, &format!("{:.1}", hi - (hi - lo) * gy as f64 / 2.0)).fill(&th.grey)
                    .font(Font::new(&fu, 7, false)).anchor("e");
            }
            cv.line(pts.iter().flat_map(|p| [p.0, p.1]).collect()).fill(&th.gold).width(2.0).smooth();
            for &(x, y) in &pts[pts.len().saturating_sub(12)..] {
                cv.oval(x - 2.0, y - 2.0, x + 2.0, y + 2.0).fill(&th.acc).outline("");
            }
            let dt = |v: &Value| GymData::s(&v["date"]).chars().skip(5).collect::<String>();
            cv.text(14.0, ch - 8.0, &dt(&w[0])).fill(&th.grey).font(Font::new(&fu, 7, false)).anchor("w");
            cv.text(cw - 14.0, ch - 8.0, &dt(&w[w.len() - 1])).fill(&th.grey).font(Font::new(&fu, 7, false)).anchor("e");
            mtext = crate::Companion::gym_trend_text(&all);
        } else {
            cv.text((330 / 2) as f64, ch - 8.0, "Your weight line appears after two weigh-ins~").fill(&th.grey)
                .font(italic(&serif, 9)).wrap(cw - 20.0);
        }
        if let Kind::Label { text, .. } = &mut ui.ws[msg].kind {
            *text = mtext;
        }
        ui.pack(msg, Pack::default().fill(Fill::X).padx(16, 16).pady(4, 6));
        let ln = ui.frame(c, &mixhex(&th.line, &bg, 0.25));
        ui.ws[ln].opt_h = 1;
        ui.pack(ln, Pack::default().fill(Fill::X).padx(16, 16).pady(4, 8));
        let pr = ui.frame(c, &bg);
        ui.pack(pr, Pack::default().fill(Fill::X).padx(16, 16));
        let l = ui.label(pr, "Height", &bg, &th.grey, Font::new(&fu, 9, false));
        ui.pack(l, Pack::default().side(Side::Left));
        let hv = match &p["height"] {
            Value::Number(n) => match n.as_i64() {
                Some(i) => i.to_string(),
                None => {
                    let v = n.as_f64().unwrap_or(0.0);
                    if v.fract() == 0.0 && v.abs() < 1e16 { format!("{v:.1}") } else { format!("{v}") }
                }
            },
            Value::String(s) => s.clone(),
            _ => String::new(),
        };
        let he = tt_entry(ui, pr, &hv, 5, &th, &f);
        ui.pack(he, Pack::default().side(Side::Left).padx(6, 2));
        self.hentry = he;
        let l = ui.label(pr, "cm", &bg, &th.grey, Font::new(&fu, 9, false));
        ui.pack(l, Pack::default().side(Side::Left).padx(0, 12));
        let sex = p["sex"].as_str().unwrap_or("").to_string();
        for (lab, val) in [("M", "m"), ("F", "f"), ("\u{2013}", "")] {
            let r = ui.check(pr, lab, &bg, &th.ink, Font::new(&fu, 9, false), sex == val, &th.field, &format!("gym_sex:{val}"),
                             0, 2);
            if let Kind::Check { radio, .. } = &mut ui.ws[r].kind {
                *radio = true;
            }
            ui.pack(r, Pack::default().side(Side::Left));
            self.radios.push((r, val));
        }
        let t = match self.plan() {
            None => "Log your weight to see a protein goal (and add your height for a calorie guide).".to_string(),
            Some((pr, None)) => format!("Protein: about {}\u{2013}{} g a day. Add your height for a calorie guide.", pr.0, pr.1),
            Some((pr, Some((up, tg)))) => format!("Aim for about {} kcal a day: a gentle 300 below your ~{} upkeep. Protein \
{}\u{2013}{} g. At 17 you're still growing, so never crash diet; slow and steady keeps the muscle.", fmt_i(tg), fmt_i(up), pr.0, pr.1),
        };
        let l = label_w(ui, c, &t, &bg, &th.ink, &fu, 9, false);
        justify_left(ui, l, 320);
        ui.pack(l, Pack::default().fill(Fill::X).padx(16, 16).pady(8, 14));
    }

    // ---- food ----
    fn food_card(&mut self, ui: &mut Ui, parent: usize, now: f64) {
        let (th, f) = (self.th.clone(), Faces { ui: self.f.ui.clone(), title: self.f.title.clone() });
        let c = hub_card(ui, parent, Some("\u{98df}\u{4e8b} \u{b7} Food today"), "round", &th, &f);
        ui.pack(c, Pack::default().fill(Fill::Both).expand());
        let bg = th.card.clone();
        let serif = self.d.serif.clone();
        let fu = f.ui.clone();
        let row = ui.frame(c, &bg);
        ui.pack(row, Pack::default().fill(Fill::X).padx(16, 16).pady(2, 4));
        let e = tt_entry(ui, row, "", 24, &th, &f);
        ui.pack(e, Pack::default().side(Side::Left).fill(Fill::X).expand());
        self.fentry = e;
        let food = self.d.food_today();
        // (Zenko's gym_food_card: with something logged, the hint names the Indian dishes and the online lookup)
        let hint = if food.is_empty() { "Type what you ate, like '2 eggs, toast with butter, a banana'" } else {
            "Type what you ate, like '2 rotis, 1 bowl dal, 150 g paneer'. Indian dishes are built in; anything else I look up \
online (USDA, Open Food Facts). Tap an item to fix the grams." };
        let status = ui.label(c, hint, &bg, &th.grey, italic(&serif, 9));
        ui.set_anchor(status, Anchor::W);
        justify_left(ui, status, 320);
        self.fstatus = status;
        let b = tt_btn(ui, row, "Add", "gym_food_add", Some(&th.start), None, 9, 10, 1, &th, &f);
        ui.pack(b, Pack::default().side(Side::Left).padx(6, 0));
        ui.pack(status, Pack::default().fill(Fill::X).padx(16, 16).pady(0, 6));
        let lst = ui.frame(c, &bg);
        ui.pack(lst, Pack::default().fill(Fill::X).padx(16, 16));
        let n = food.len();
        let start = n.saturating_sub(14);
        for (k, it) in food[start..].iter().enumerate() {
            let idx = start + k;
            let r = ui.frame(lst, &bg);
            ui.pack(r, Pack::default().fill(Fill::X).pady(1, 1));
            let l = label_w(ui, r, &crate::extra::clip(&GymData::s(&it["t"]), 46), &bg, &th.ink, &fu, 9, false);
            ui.pack(l, Pack::default().side(Side::Left));
            Self::hand(ui, &[l]);
            self.click(&[l], &format!("gym_fix:{idx}"));
            let x = ui.label(r, "\u{2715}", &bg, &th.grey, Font::new(&fu, 8, false));
            ui.pack(x, Pack::default().side(Side::Right).padx(6, 0));
            Self::hand(ui, &[x]);
            self.click(&[x], &format!("gym_food_rm:{idx}"));
            let kc = it["kcal"].as_f64().unwrap_or(0.0) as i64;
            let l = ui.label(r, &format!("{kc} kcal \u{b7} {:.1} g", it["p"].as_f64().unwrap_or(0.0)), &bg, &th.gold,
                             italic(&serif, 9));
            ui.pack(l, Pack::default().side(Side::Right));
        }
        if food.is_empty() {
            let l = ui.label(lst, "Nothing logged yet today.", &bg, &th.grey, italic(&serif, 10));
            ui.pack(l, Pack::default().anchor(Anchor::W));
        }
        let tot = zenko_sound::music::py_sum(food.iter().map(|i| i["kcal"].as_f64().unwrap_or(0.0)));
        let prot = zenko_sound::music::py_sum(food.iter().map(|i| i["p"].as_f64().unwrap_or(0.0)));
        let plan = self.plan();
        let ln = ui.frame(c, &mixhex(&th.line, &bg, 0.25));
        ui.ws[ln].opt_h = 1;
        ui.pack(ln, Pack::default().fill(Fill::X).padx(16, 16).pady(8, 6));
        for (lab, val, goal) in [("Energy", tot, plan.and_then(|p| p.1.map(|x| x.1))), ("Protein", prot, plan.map(|p| p.0.0))] {
            let r = ui.frame(c, &bg);
            ui.pack(r, Pack::default().fill(Fill::X).padx(16, 16).pady(2, 2));
            let unit = if lab == "Energy" { "kcal" } else { "g" };
            let l = label_w(ui, r, lab, &bg, &th.grey, &fu, 9, false);
            ui.ws[l].opt_w = 7;
            ui.pack(l, Pack::default().side(Side::Left));
            let goal = goal.filter(|g| *g != 0);
            if let (Some(gv), "Protein") = (goal, lab) {
                let ff = self.foxfire(ui, r, val / gv as f64, 8, 14.0, now);
                ui.pack(ff, Pack::default().side(Side::Left).padx(0, 8));
            } else if let Some(gv) = goal {
                let bb = self.brush_bar(ui, r, val / gv as f64, 150);
                ui.pack(bb, Pack::default().side(Side::Left).padx(0, 8));
            }
            let t = match goal {
                Some(gv) => format!("{} / {} {unit}", fmt_i(val as i64), fmt_i(gv)),
                None => format!("{} {unit}", fmt_i(val as i64)),
            };
            let l = ui.label(r, &t, &bg, &th.ink, Font::new(&fu, 9, false));
            ui.pack(l, Pack::default().side(Side::Left));
        }
        let tip = if !food.is_empty() { "Estimates are rough (within about 20%). What matters is the weekly trend, not one perfect day." }
            else { "Protein at every meal (eggs, dal, chicken, paneer, yogurt) helps you keep muscle while losing fat." };
        let l = label_w(ui, c, tip, &bg, &th.grey, &fu, 8, false);
        justify_left(ui, l, 320);
        ui.pack(l, Pack::default().fill(Fill::X).padx(16, 16).pady(8, 14));
    }

    /// zk_fit (fonts, zk_wrap), the title for the width, the banner drawn for its width
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
        self.banner_draw(ui);
    }

    /// the animations that are due: true when a canvas changed
    pub fn step(&mut self, ui: &mut Ui, now: f64) -> bool {
        let mut changed = false;
        let gold = self.th.gold.clone();
        let bg = self.th.card.clone();
        let mut anims = std::mem::take(&mut self.anims);
        for (a, at, k) in anims.iter_mut() {
            if now < *at {
                continue;
            }
            let kk = *k;
            match a {
                Anim::Twinkle { cv } => {
                    *at = now + 0.42;
                    let c = canvas_of(ui, *cv);
                    for (n, id) in c.find_withtag("star").into_iter().enumerate() {
                        if (n as u64 + kk) % 5 == 0 {
                            c.get(id).unwrap().fill(if kk % 2 != 0 { "#ffffff" } else { "#8f88c8" });
                        }
                    }
                    c.move_items("ff", 0.0, if kk % 4 < 2 { -1.0 } else { 1.0 });
                }
                Anim::Flick { cv, flames } => {
                    *at = now + 0.23;
                    let c = canvas_of(ui, *cv);
                    for (j, &(o, cx, b, fh, fw)) in flames.iter().enumerate() {
                        let m = (kk + j as u64) % 3;
                        let lean = if m == 0 { 0.18 } else if m == 1 { -0.14 } else { 0.0 };
                        c.get(o).unwrap().coords = flame(cx, b, fh, fw, lean);
                    }
                }
                Anim::Figures { cv, front, back, day } => {
                    *at = now + 0.26;
                    let (fr, bk) = self.photos(*day);
                    let c = canvas_of(ui, *cv);
                    c.get(*front).unwrap().image = Some(fr[(kk % 4) as usize].clone());
                    c.get(*back).unwrap().image = Some(bk[(kk % 4) as usize].clone());
                    for (n, id) in c.find_withtag("star").into_iter().enumerate() {
                        if (n as u64 + kk) % 4 == 0 {
                            c.get(id).unwrap().fill(&if kk % 2 != 0 { gold.clone() } else { mixhex(&gold, &bg, 0.6) });
                        }
                    }
                }
                Anim::Drift { cv, zs, night } => {
                    *at = now + 0.5;
                    let c = canvas_of(ui, *cv);
                    for (j, &z) in zs.iter().enumerate() {
                        c.get(z).unwrap().fill(if (kk + j as u64) % 3 != 0 { "#b9c2ff" } else { night.as_str() });
                    }
                    for (n, id) in c.find_withtag("star").into_iter().enumerate() {
                        if (n as u64 + kk) % 5 == 0 {
                            c.get(id).unwrap().fill(if kk % 2 != 0 { "#ffffff" } else { "#8f88c8" });
                        }
                    }
                }
            }
            *k += 1;
            changed = true;
        }
        self.anims = anims;
        changed
    }
}

/// a flame's six points (gym_foxfire's pts)
fn flame(cx: f64, b: f64, fh: f64, fw: f64, lean: f64) -> Vec<f64> {
    vec![cx + lean * fw, b - fh, cx + fw * 0.5, b - fh * 0.45, cx + fw * 0.45, b - fh * 0.12, cx, b, cx - fw * 0.45,
         b - fh * 0.12, cx - fw * 0.5, b - fh * 0.45]
}
