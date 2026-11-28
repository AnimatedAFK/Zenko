//! The hub's Zen garden page (zenko.py Zenko hub_build_garden / garden_draw / garden_overlay_setup / garden_zenko /
//! garden_anim / garden_panel / zen_cards / breath_*, with Zenko22's pond fish, decor, cast frame and fishing card):
//! the pixel-art garden with Zenko sitting by her tree (lit by the lanterns, her shadow and the grass painted into the
//! picture), the glowing lanterns, fireflies, falling petals and your own koi swimming in the pond; the side panel
//! (Breathe, Koi fishing, Grounding, the tree's ema, Recent, How it grows); the breathing guide over a dimmed garden.

use crate::aa::{Shape, Sh, aa_pixels};
use crate::fishart::{fish_all, fish_by_id, fish_rgba, gdata};
use crate::gardenart::{BAYER, Lay, Look, Ras, Rgb, garden_scene, lerp3};
use crate::gympage::{flame, foxfire_w};
use crate::hubside::zk_size;
use crate::hubwin::{canvas_of, hub_title_draw};
use crate::pagekit::{Faces, Scroller, WrapState, glow_bar, hub_card, hub_scroller, label_w, tt_btn, zk_fonts, zk_wrap};
use crate::shelfpage::spaced;
use crate::theme::Theme;
use crate::tkcanvas::Photo;
use crate::ui::{Anchor, Fill, Font, Fonts, Grid, Kind, Pack, Place, Side, Ui};
use serde_json::Value;
use std::collections::HashMap;
use std::f64::consts::PI;
use std::rc::Rc;
use zenko_art::palette::{Palette, mixhex};
use zenko_sound::mt::PyRandom;

pub const TITLE: &str = "Zen garden";
pub const SUB: &str = "\u{7985} \u{b7} Breathe here before an exam or before sleep. Your focus sessions grow the tree.";

/// what the page shows
#[derive(Clone)]
pub struct GardenData {
    /// life["garden"] and life["fishing"] (with their setdefaults)
    pub garden: Value,
    pub fishing: Value,
    pub cards: Vec<Value>,
    /// the settings it reads: breath_kind, breath_minutes, breath_sound, lockin_minutes, fish_gold, garden_decor
    pub cfg: Value,
    pub lockin: bool,
    /// local day number, its ISO date, local hour and minute (z19_daypart), the month
    pub today: i64,
    pub hour: u32,
    pub minute: u32,
    pub month: u32,
    /// Z19_LOOK's switches and weather (the part of day and season are filled in from the clock)
    pub sky: bool,
    pub seasons: bool,
    pub weather: Option<String>,
    /// the weather code (fish_now's rain and snow)
    pub wx_code: Option<i64>,
    /// journey_state: hours and the places reached
    pub hours: f64,
    pub got: Vec<String>,
    /// Zenko's colours, look and tails
    pub pal: Palette,
    pub look: zenko_art::sit::Look,
    pub tails: i32,
    pub hd: bool,
    /// _mincho_font() or _jp_font(); _jp_font(); _serif_font()
    pub jp: String,
    pub jpg: String,
    pub serif: String,
    /// the 5-4-3-2-1 step (-1 = not started)
    pub ground_i: i64,
}

impl Default for GardenData {
    fn default() -> Self {
        GardenData {
            garden: Value::Null, fishing: Value::Null, cards: vec![], cfg: Value::Null, lockin: false, today: 0, hour: 12,
            minute: 0, month: 1, sky: true, seasons: true, weather: None, wx_code: None, hours: 0.0, got: vec![],
            pal: Palette::empty(), look: zenko_art::sit::Look::default(), tails: 1, hd: true, jp: String::new(),
            jpg: String::new(), serif: String::new(), ground_i: -1,
        }
    }
}

impl GardenData {
    fn iso(&self) -> String {
        crate::plan::iso(self.today)
    }
    fn part(&self) -> &'static str {
        crate::hubside::daypart(self.hour, self.minute).0
    }
    fn season(&self) -> &'static str {
        crate::hubside::season_of(self.month)
    }
    fn look(&self) -> Look {
        Look { season: self.seasons, sky: self.sky, season_name: self.season().into(), part: self.part().into(),
               weather: self.weather.clone() }
    }
    fn stages() -> &'static [Value] {
        crate::rewards::rdata()["GARDEN_STAGES"].as_array().unwrap()
    }
    /// garden_stage(): (stage, growth 0..1, minutes for the next stage)
    pub fn stage(&self) -> (usize, f64, Option<f64>) {
        let xp = self.garden["xp"].as_f64().unwrap_or(0.0);
        let st = Self::stages();
        let mut i = 0;
        for (k, s) in st.iter().enumerate() {
            if xp >= s[0].as_f64().unwrap_or(0.0) {
                i = k;
            }
        }
        let nxt = st.get(i + 1).map(|s| s[0].as_f64().unwrap_or(0.0));
        let lo = st[i][0].as_f64().unwrap_or(0.0);
        let g = match nxt {
            Some(n) => 1.0f64.min((xp - lo) / (n - lo)),
            None => 1.0f64.min((xp - lo) / 600.0),
        };
        (i, g, nxt)
    }
    /// garden_health() on this day
    pub fn health(&self) -> f64 {
        let h = self.garden["health"].as_f64().unwrap_or(1.0);
        let Some(last) = self.garden["active"].as_array().and_then(|a| a.last()).and_then(|v| v.as_str()) else { return h };
        let Some(ld) = crate::plan::parse_iso(last) else { return h };
        let eff = (self.today - ld - 1 - 1).max(0);
        (h - 0.25 * eff as f64).clamp(0.0, 1.0)
    }
    fn c(&self, k: &str) -> &Value {
        &self.cfg[k]
    }
    /// fish_now(): night, day, rain, snow, season, part
    pub fn fish_now(&self) -> FishNow {
        let rain = matches!(self.wx_code, Some(51 | 53 | 55 | 56 | 57 | 61 | 63 | 65 | 66 | 67 | 80 | 81 | 82 | 95 | 96 | 99));
        let snow = matches!(self.wx_code, Some(71 | 73 | 75 | 77 | 85 | 86));
        let part = self.part();
        FishNow { night: part == "night", day: part == "day" || part == "dawn", rain, snow, season: self.season(), part }
    }
    /// fish_gear(): (rod, lantern, hours)
    pub fn fish_gear(&self) -> (usize, usize, f64) {
        let g = gdata();
        let best = |k: &str| g[k].as_array().unwrap().iter().enumerate()
            .filter(|(_, r)| self.hours >= r[2].as_f64().unwrap_or(0.0)).map(|(i, _)| i).max().unwrap_or(0);
        (best("FISH_RODS"), best("FISH_LANTERNS"), self.hours)
    }
    /// fish_can(f, now, depth, rod, lantern)
    pub fn fish_can(&self, f: &Value, now: &FishNow, depth: i64, rod: usize, lan: usize) -> bool {
        let c = &f[6];
        let t = |k: &str| c[k].as_bool().unwrap_or(false);
        if t("night") && !now.night || t("day") && now.night || t("rain") && !now.rain {
            return false;
        }
        if t("snow_or_winter") && !(now.snow || now.season == "winter") {
            return false;
        }
        if let Some(s) = c["season"].as_str()
            && s != now.season
        {
            return false;
        }
        let n = |k: &str, d: i64| c[k].as_i64().unwrap_or(d);
        if (lan as i64) < n("lantern", 0) || (rod as i64) < n("rod", 0) || depth < n("depth", 1) {
            return false;
        }
        if let Some(pl) = c["place"].as_str()
            && !self.got.iter().any(|g| g == pl)
        {
            return false;
        }
        true
    }
}

pub struct FishNow {
    pub night: bool,
    pub day: bool,
    pub rain: bool,
    pub snow: bool,
    pub season: &'static str,
    pub part: &'static str,
}

/// a breathing session (self.breath)
#[derive(Clone, Debug)]
pub struct Breath {
    pub key: String,
    pub steps: Vec<(String, f64)>,
    pub cycle: f64,
    pub rounds: i64,
    pub total: f64,
    pub t0: f64,
    pub phase: Option<i64>,
    pub starts: Vec<f64>,
    pub ends: Vec<f64>,
}

/// breath_sizes(steps): the orb size each step starts and ends at
pub fn breath_sizes(steps: &[(String, f64)]) -> (Vec<f64>, Vec<f64>) {
    let n = steps.len();
    let mut ends: Vec<Option<f64>> = vec![];
    for (k, (ph, _)) in steps.iter().enumerate() {
        let nxt = &steps[(k + 1) % n].0;
        ends.push(match ph.as_str() {
            "in" => Some(if nxt == "top" { 0.82 } else { 1.0 }),
            "top" => Some(1.0),
            "out" => Some(0.35),
            _ => None,
        });
    }
    for k in 0..n {
        if ends[k].is_none() {
            let prev = ends[(k + n - 1) % n];
            ends[k] = Some(prev.unwrap_or(0.35));
        }
    }
    let ends: Vec<f64> = ends.into_iter().map(|e| e.unwrap()).collect();
    let starts = (0..n).map(|k| ends[(k + n - 1) % n]).collect();
    (starts, ends)
}

/// breath_start's session for a pattern (minutes None = the saved length)
pub fn breath_new(key: &str, minutes: Option<f64>, cfg_minutes: f64, now: f64) -> Breath {
    let p = &gdata()["BREATH_PATTERNS"][key];
    let steps: Vec<(String, f64)> = p["steps"].as_array().unwrap().iter()
        .map(|s| (s[0].as_str().unwrap().to_string(), s[1].as_f64().unwrap())).collect();
    let cycle = zenko_sound::music::py_sum(steps.iter().map(|s| s.1));
    let mins = minutes.unwrap_or(if key == "sigh" { 1.0 } else { cfg_minutes });
    let mins = if mins == 0.0 { 3.0 } else { mins };
    let mut rounds = 2.max((mins * 60.0 / cycle).round_ties_even() as i64);
    if key == "478" && minutes.is_none() {
        rounds = 4.max(rounds.min(8));
    }
    let (starts, ends) = breath_sizes(&steps);
    Breath { key: key.into(), steps, cycle, rounds, total: rounds as f64 * cycle, t0: now + 2.0, phase: None, starts, ends }
}

/// the fish on the line (self._z22_fish)
#[derive(Clone, Debug)]
pub struct Fish {
    pub phase: String,
    pub t: f64,
    pub bx: f64,
    pub by: f64,
    pub depth: i64,
    pub gold: bool,
    pub q: Value,
    pub bite_at: f64,
    /// rod, rod2, line, bob, bob2, mark
    pub items: Option<[u32; 6]>,
    /// (item, x, y, t0, big)
    pub ripples: Vec<(u32, f64, f64, f64, bool)>,
    /// (t, dx, item)
    pub jump: Option<(f64, f64, u32)>,
}

#[derive(Clone, Debug)]
struct Fly {
    id: u32,
    x: f64,
    y: f64,
    ph: f64,
    sp: f64,
    a: f64,
}

#[derive(Clone, Debug)]
struct Swim {
    fid: String,
    seed: i64,
    l: i64,
    x: f64,
    y: f64,
    a: f64,
    sp: f64,
    item: Option<u32>,
    k: i64,
}

#[derive(Clone, Debug)]
struct Petal {
    id: u32,
    vy: f64,
    ph: f64,
    floor: f64,
}

/// what the hub must do after a page event
#[derive(Clone, Debug, PartialEq)]
pub enum GAct {
    /// play a breathing cue (phase, seconds)
    BreathCue(String, f64),
    /// the session ended: (key, minutes)
    BreathDone(String, f64),
    /// a fish bit: ask its question
    FishAsk,
    Sound(&'static str),
}

pub struct GardenPage {
    pub pg: usize,
    pub title: usize,
    pub body: usize,
    pub cv: usize,
    pub side: usize,
    pub sc: Scroller,
    pub clicks: Vec<(usize, String)>,
    pub side_w: i32,
    bar: Option<(usize, f64, i32)>,
    flick: Option<(usize, Vec<(u32, f64, f64, f64, f64)>)>,
    flick_at: f64,
    flick_k: u64,
    key: Option<String>,
    pub z: i64,
    pub ras: Option<Ras>,
    pub lay: Option<Lay>,
    img: Option<Rc<Photo>>,
    img_dim: Option<Rc<Photo>>,
    dimmed: Option<String>,
    glows: Option<[Rc<Photo>; 3]>,
    lamps: Vec<u32>,
    flies: Vec<Fly>,
    petals: Vec<Petal>,
    petal_rate: f64,
    petal_col: [&'static str; 3],
    swim: Vec<Swim>,
    fish_img: HashMap<(String, i64, i64, i64, i64, u64), Rc<Photo>>,
    orbs: HashMap<(i64, i64, String), Rc<Photo>>,
    decor_img: HashMap<(String, i64), Rc<Photo>>,
    /// track, orb, count_s, count, label_s, label, sub, head
    bitems: [u32; 8],
    pub breath: Option<Breath>,
    pub fish: Option<Fish>,
    pub done_until: f64,
    pub anim_on: bool,
    pub wrap: WrapState,
    seig: Option<(String, Rc<Photo>)>,
    th: Theme,
    f: Faces,
    pub d: GardenData,
    pub zk_f: f64,
    /// the lacquer card over the picture (fish_card) and what it is: 1 the question, 2 a result with its buttons
    pub fcard: Option<usize>,
    pub fcard_kind: u8,
}

fn justify_left(ui: &mut Ui, l: usize, wl: i32) {
    if let Kind::Label { left, wrap, .. } = &mut ui.ws[l].kind {
        *left = true;
        *wrap = wl;
    }
}

fn italic(fam: &str, n: i32) -> Font {
    Font { family: fam.into(), size: n, bold: false, italic: true }
}

fn hexc(c: Rgb) -> [u8; 4] {
    [c.0, c.1, c.2, 255]
}

/// PhotoImage.zoom(z, z): every pixel made a z x z block
pub fn zoom_px(w: usize, h: usize, px: &[[u8; 4]], z: usize) -> Photo {
    let z = z.max(1);
    let mut out = Vec::with_capacity(w * h * z * z);
    for y in 0..h * z {
        for x in 0..w * z {
            out.push(px[(y / z) * w + x / z]);
        }
    }
    Photo { w: w * z, h: h * z, px: out }
}

fn pct_g(v: f64) -> String {
    // "%g" of a number rounded to one place
    if v == v.trunc() { format!("{}", v as i64) } else { format!("{v}") }
}

fn weekday_day(iso: &str) -> String {
    match crate::plan::parse_iso(iso) {
        Some(d) => {
            let wd = ["Thu", "Fri", "Sat", "Sun", "Mon", "Tue", "Wed"][d.rem_euclid(7) as usize];
            format!("{wd} {}", &iso[8..10])
        }
        None => iso.to_string(),
    }
}

/// breath_orb_photo(r, z, phase): the breathing orb as pixel art
fn orb_px(r: f64, z: i64, phase: &str) -> Photo {
    let r = 3.max(r as i64);
    let tint = match gdata()["BREATH_TINT"][phase].as_array() {
        Some(a) => (a[0].as_u64().unwrap() as u8, a[1].as_u64().unwrap() as u8, a[2].as_u64().unwrap() as u8),
        None => (210, 220, 255),
    };
    let rr = (r as f64 * 1.5) as i64 + 2;
    let n = (2 * rr + 1) as usize;
    let rf = r as f64;
    let mut px = Vec::with_capacity(n * n);
    for y in 0..n as i64 {
        for x in 0..n as i64 {
            let d = zenko_art::core::hypot((x - rr) as f64, (y - rr) as f64) / rf;
            let mut a = 0.0;
            let mut c = tint;
            if d <= 1.0 {
                if d > 1.0 - 1.6 / rf {
                    a = 0.95;
                    c = lerp3(tint, (255, 255, 255), 0.55);
                } else if d > 1.0 - 2.6 / rf {
                    a = 0.45;
                } else {
                    a = 0.10 + 0.16 * (1.0 - d).powf(2.0);
                    if d < 0.35 && crate::gardenart::dith(x, y, (0.35 - d) * 1.2) {
                        a += 0.06;
                    }
                }
            } else if d < 1.5 {
                let q = (1.0 - (d - 1.0) / 0.5).powf(2.0) * 0.30;
                a = if !crate::gardenart::dith(x, y, q * 2.6) { 0.0 } else { 0.22 };
            }
            let al = (255.0 * 1.0f64.min(a)) as u8;
            px.push(if al == 0 { [0, 0, 0, 0] } else { [c.0, c.1, c.2, al] });
        }
    }
    zoom_px(n, n, &px, z as usize)
}

impl GardenPage {
    pub fn build(ui: &mut Ui, pg: usize, th: &Theme, f: &Faces, cm10: i32, d: &GardenData) -> GardenPage {
        let title = ui.canvas(pg, &th.banner, cm10, 96);
        canvas_of(ui, title);
        ui.pack(title, Pack::default().fill(Fill::X));
        let body = ui.frame(pg, &th.paper);
        ui.pack(body, Pack::default().fill(Fill::Both).expand().padx(20, 20).pady(12, 14));
        ui.ws[body].gw_rows = vec![1];
        ui.ws[body].gw_cols = vec![1];
        let frame = ui.frame(body, &mixhex(&th.line, "#000000", 0.2));
        (ui.ws[frame].padx, ui.ws[frame].pady) = (1, 1);
        ui.grid(frame, Grid { row: 0, col: 0, sticky: "nsew", padx: (0, 0), pady: (0, 0), cspan: 0, rspan: 0 });
        let cv = ui.canvas(frame, "#0a0922", 420, 300);
        canvas_of(ui, cv);
        ui.pack(cv, Pack::default().fill(Fill::Both).expand());
        let side = ui.frame(body, &th.paper);
        ui.ws[side].opt_w = 300;
        ui.ws[side].propagate = false;
        ui.grid(side, Grid { row: 0, col: 1, sticky: "ns", padx: (14, 0), pady: (0, 0), cspan: 0, rspan: 0 });
        let sc = hub_scroller(ui, side, th, cm10);
        let mut p = GardenPage {
            pg, title, body, cv, side, sc, clicks: vec![], side_w: 300, bar: None, flick: None, flick_at: 0.0, flick_k: 1,
            key: None, z: 2, ras: None, lay: None, img: None, img_dim: None, dimmed: None, glows: None, lamps: vec![],
            flies: vec![], petals: vec![], petal_rate: 0.0, petal_col: ["", "", ""], swim: vec![], fish_img: HashMap::new(),
            orbs: HashMap::new(), decor_img: HashMap::new(), bitems: [0; 8], breath: None, fish: None, done_until: 0.0,
            anim_on: false, wrap: WrapState::default(), seig: None, th: th.clone(),
            f: Faces { ui: f.ui.clone(), title: f.title.clone() }, d: d.clone(), zk_f: 1.0, fcard: None, fcard_kind: 0,
        };
        p.panel(ui, d);
        p
    }

    fn click(&mut self, ids: &[usize], cmd: &str) {
        for &i in ids {
            self.clicks.push((i, cmd.to_string()));
            ui_hand_mark(i);
        }
    }

    /// the command of the clickable row under a point, if any
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

    // ---- the side panel ----------------------------------------------------------------------------------------

    /// garden_panel() (Zenko22: the fishing card goes in after the Breathe card)
    pub fn panel(&mut self, ui: &mut Ui, d: &GardenData) {
        self.d = d.clone();
        let p = self.sc.inner;
        for k in ui.ws[p].kids.clone() {
            ui.destroy(k);
        }
        self.clicks.clear();
        self.bar = None;
        self.flick = None;
        let (th, f) = (self.th.clone(), Faces { ui: self.f.ui.clone(), title: self.f.title.clone() });
        let bg = th.card.clone();
        let fu = f.ui.clone();
        let serif = d.serif.clone();
        let breath_card = self.zen_breathe(ui, p, d);
        let _ = breath_card;
        let ground = self.zen_ground(ui, p, d);
        let (i, g, nxt) = d.stage();
        let hp = d.health();
        let stv = &GardenData::stages()[i];
        let s = |v: &Value| v.as_str().unwrap_or("").to_string();
        let c = hub_card(ui, p, None, "ema", &th, &f);
        ui.pack(c, Pack::default().fill(Fill::X).pady(12, 0));
        let l = ui.label(c, &s(&stv[1]), &bg, &mixhex(&th.gold, &th.ink, 0.25), Font::new(&d.jp, 30, false));
        ui.pack(l, Pack::default().pady(16, 0));
        let l = ui.label(c, &spaced(&s(&stv[3]).to_uppercase()), &bg, &th.ink, Font::new(&serif, 11, false));
        ui.pack(l, Pack::default());
        let n = GardenData::stages().len();
        let l = ui.label(c, &format!("{} \u{b7} stage {} of {}", s(&stv[2]), i + 1, n), &bg, &th.grey, italic(&serif, 9));
        ui.pack(l, Pack::default().pady(0, 8));
        let r = ui.frame(c, &bg);
        ui.pack(r, Pack::default().fill(Fill::X).padx(16, 16));
        let l = label_w(ui, r, "\u{6210}\u{9577}  Growth", &bg, &th.grey, &fu, 9, false);
        ui.pack(l, Pack::default().fill(Fill::X));
        let bar = ui.canvas(r, &bg, 200, 12);
        canvas_of(ui, bar);
        ui.pack(bar, Pack::default().fill(Fill::X).pady(2, 0));
        self.bar = Some((bar, g, -1));
        let xp = d.garden["xp"].as_f64().unwrap_or(0.0);
        let t = match nxt {
            Some(nx) => {
                let ns = &GardenData::stages()[i + 1];
                format!("{} / {} min to {} {}", crate::routes17::fmt_int(xp as i64), crate::routes17::fmt_int(nx as i64),
                        s(&ns[1]), s(&ns[3]))
            }
            None => "Fully grown: a sacred tree! You can plant a new seed.".into(),
        };
        let l = label_w(ui, r, &t, &bg, &th.ink, &fu, 9, false);
        justify_left(ui, l, 0);
        ui.pack(l, Pack::default().fill(Fill::X).pady(2, 8));
        let l = label_w(ui, r, "\u{751f}\u{6c17}  Vitality", &bg, &th.grey, &fu, 9, false);
        ui.pack(l, Pack::default().fill(Fill::X));
        let (ff, flames) = foxfire_w(ui, &th, r, hp, 8, 14.0);
        ui.pack(ff, Pack::default().anchor(Anchor::W).pady(2, 0));
        if !flames.is_empty() {
            self.flick = Some((ff, flames));
        }
        let act: Vec<String> = d.garden["active"].as_array().map(|a| a.iter().filter_map(|v| v.as_str().map(String::from))
            .collect()).unwrap_or_default();
        let mood = if act.is_empty() {
            "Waiting for your first lock-in~"
        } else if hp >= 0.99 {
            "Thriving. Lovely~"
        } else if hp >= 0.75 {
            "A little thirsty"
        } else if hp >= 0.5 {
            "Drooping: a session will help"
        } else if hp > 0.0 {
            "Wilting... one lock-in brings it back"
        } else {
            "Withered, but not gone. It's waiting for you"
        };
        let l = ui.label(r, mood, &bg, &th.ink, italic(&serif, 10));
        ui.set_anchor(l, Anchor::W);
        justify_left(ui, l, 0);
        ui.pack(l, Pack::default().fill(Fill::X).pady(2, 10));
        let today = d.iso();
        let mut streak = 0;
        let mut dd = if act.contains(&today) { d.today } else { d.today - 1 };
        while act.contains(&crate::plan::iso(dd)) {
            streak += 1;
            dd -= 1;
        }
        let log: Vec<Value> = d.garden["log"].as_array().cloned().unwrap_or_default();
        let tl: Vec<&Value> = log.iter().filter(|e| e["d"].as_str() == Some(&today)).collect();
        let mins: i64 = tl.iter().map(|e| e["m"].as_i64().unwrap_or(0)).sum();
        let l = label_w(ui, c, &format!("{}-day garden streak   \u{b7}   today: {} session{}, {} min", streak, tl.len(),
                                       if tl.len() == 1 { "" } else { "s" }, mins), &bg, &th.grey, &fu, 9, false);
        justify_left(ui, l, 0);
        ui.pack(l, Pack::default().fill(Fill::X).padx(16, 16).pady(0, 8));
        let btns = ui.frame(c, &bg);
        ui.pack(btns, Pack::default().fill(Fill::X).padx(16, 16).pady(0, 14));
        if d.lockin {
            let l = label_w(ui, btns, "Lock-in running: your tree is drinking it up~", &bg, &th.gold, &fu, 9, true);
            justify_left(ui, l, 0);
            ui.pack(l, Pack::default().fill(Fill::X));
        } else {
            let lm = d.c("lockin_minutes").as_f64().filter(|v| *v != 0.0).unwrap_or(50.0) as i64;
            let b = tt_btn(ui, btns, &format!("Lock in {lm} min"), "gd_lockin", Some(&th.acc), Some(&th.onacc), 10, 12, 3, &th, &f);
            ui.pack(b, Pack::default().anchor(Anchor::W));
        }
        if nxt.is_none() {
            let b = tt_btn(ui, btns, "Plant a new seed", "gd_plant", Some(&th.start), None, 9, 10, 2, &th, &f);
            ui.pack(b, Pack::default().anchor(Anchor::W).pady(6, 0));
        }
        let c2 = hub_card(ui, p, Some("\u{8a18}\u{9332} \u{b7} Recent"), "", &th, &f);
        ui.pack(c2, Pack::default().fill(Fill::X).pady(12, 0));
        if log.is_empty() {
            let l = ui.label(c2, "Lock-ins and focus sessions show up here.", &bg, &th.grey, italic(&serif, 10));
            ui.set_anchor(l, Anchor::W);
            ui.pack(l, Pack::default().fill(Fill::X).padx(16, 16).pady(0, 12));
        }
        for e in log.iter().rev().take(6) {
            let rr = ui.frame(c2, &bg);
            ui.pack(rr, Pack::default().fill(Fill::X).padx(16, 16).pady(1, 1));
            let dd = weekday_day(e["d"].as_str().unwrap_or(""));
            let l = ui.label(rr, &format!("+{} min", e["m"].as_i64().unwrap_or(0)), &bg, &th.gold, Font::new(&fu, 9, true));
            ui.pack(l, Pack::default().side(Side::Right));
            let k = if e["k"] == "lockin" { "Lock-in" } else { "Focus" };
            let l = label_w(ui, rr, &format!("{} {} \u{b7} {}", dd, e["t"].as_str().unwrap_or(""), k), &bg, &th.ink, &fu, 9, false);
            ui.pack(l, Pack::default().side(Side::Left).fill(Fill::X).expand());
        }
        let sp = ui.frame(c2, &bg);
        ui.ws[sp].opt_h = 10;
        ui.pack(sp, Pack::default());
        let c3 = hub_card(ui, p, Some("\u{5fc3}\u{5f97} \u{b7} How it grows"), "", &th, &f);
        ui.pack(c3, Pack::default().fill(Fill::X).pady(12, 0));
        let grove = d.garden["grove"].as_array().map(|a| a.len()).unwrap_or(0);
        let l = label_w(ui, c3, &format!("A lock-in grows your tree by its minutes; a normal focus session counts at 60%. One day off is a free rest day. After that, every skipped day makes it wilt a little, and the next session brings it back. It starts in a bonsai pot and is planted in the shrine garden when it blossoms. Grown trees join the grove on the hills ({grove} so far)."), &bg, &th.grey, &fu, 9, false);
        justify_left(ui, l, 250);
        ui.pack(l, Pack::default().fill(Fill::X).padx(16, 16).pady(0, 14));
        let fc = self.fish_card(ui, p, d);
        ui.pack_before(fc, ground, Pack::default().fill(Fill::X).pady(12, 0));
    }

    /// zen_cards' Breathe card
    fn zen_breathe(&mut self, ui: &mut Ui, p: usize, d: &GardenData) -> usize {
        let (th, f) = (self.th.clone(), Faces { ui: self.f.ui.clone(), title: self.f.title.clone() });
        let bg = th.card.clone();
        let fu = f.ui.clone();
        let g = gdata();
        let cur = match &self.breath {
            Some(b) => b.key.clone(),
            None => d.c("breath_kind").as_str().unwrap_or("box").to_string(),
        };
        let cur = if g["BREATH_PATTERNS"][cur.as_str()].is_object() { cur } else { "box".to_string() };
        let c = hub_card(ui, p, Some("\u{547c}\u{5438} \u{b7} Breathe"), "", &th, &f);
        ui.pack(c, Pack::default().fill(Fill::X));
        let l = label_w(ui, c, "Before an exam, before sleep, or whenever your mind races.", &bg, &th.grey, &fu, 9, false);
        justify_left(ui, l, 0);
        ui.pack(l, Pack::default().fill(Fill::X).padx(14, 14).pady(0, 6));
        let sel_bg = mixhex(&th.gold, &bg, 0.84);
        for key in g["BREATH_ORDER"].as_array().unwrap() {
            let key = key.as_str().unwrap();
            let pp = &g["BREATH_PATTERNS"][key];
            let on = key == cur;
            let rb = if on { sel_bg.clone() } else { bg.clone() };
            let row = ui.frame(c, &rb);
            ui.ws[row].hl = 1;
            ui.ws[row].hlc = if on { mixhex(&th.gold, &bg, 0.45) } else { bg.clone() };
            ui.pack(row, Pack::default().fill(Fill::X).padx(12, 12).pady(2, 2));
            let nums = if key != "sigh" {
                pp["steps"].as_array().unwrap().iter().map(|s| s[1].as_i64().unwrap().to_string()).collect::<Vec<_>>().join("-")
            } else {
                "2 + 1, then 6".into()
            };
            let a = label_w(ui, row, pp["name"].as_str().unwrap(), &rb, &th.ink, &fu, 10, on);
            ui.pack(a, Pack::default().fill(Fill::X).padx(8, 8).pady(4, 0));
            let b = label_w(ui, row, &format!("{}   \u{b7}   {}", pp["use"].as_str().unwrap(), nums), &rb,
                            if on { &th.gold } else { &th.grey }, &fu, 9, false);
            ui.pack(b, Pack::default().fill(Fill::X).padx(8, 8).pady(0, 4));
            self.click(&[row, a, b], &format!("gd_breath_pick:{key}"));
            for i in [row, a, b] {
                ui.ws[i].hand = true;
            }
        }
        let l = ui.label(c, g["BREATH_PATTERNS"][cur.as_str()]["about"].as_str().unwrap(), &bg, &th.ink, italic(&d.serif, 10));
        ui.set_anchor(l, Anchor::W);
        justify_left(ui, l, 0);
        ui.pack(l, Pack::default().fill(Fill::X).padx(14, 14).pady(8, 4));
        if cur != "sigh" {
            let r = ui.frame(c, &bg);
            ui.pack(r, Pack::default().fill(Fill::X).padx(12, 12));
            let l = ui.label(r, "Length", &bg, &th.grey, Font::new(&fu, 9, false));
            ui.pack(l, Pack::default().side(Side::Left).padx(2, 6));
            let mv = d.c("breath_minutes").as_f64().filter(|v| *v != 0.0).unwrap_or(3.0) as i64;
            for m in [1, 3, 5] {
                let rb = ui.check(r, &format!("{m} min"), &bg, &th.ink, Font::new(&fu, 9, false), mv == m, &th.field,
                                  &format!("gd_bmin:{m}"), 0, 2);
                if let Kind::Check { radio, .. } = &mut ui.ws[rb].kind {
                    *radio = true;
                }
                ui.pack(rb, Pack::default().side(Side::Left).padx(0, 4));
            }
        }
        let snd = d.c("breath_sound").as_bool().unwrap_or(true);
        let ck = ui.check(c, "Gentle sound with each breath", &bg, &th.ink, Font::new(&fu, 9, false), snd, &th.field,
                          "gd_bsound", 0, 2);
        ui.pack(ck, Pack::default().fill(Fill::X).padx(12, 12).pady(2, 0));
        let btns = ui.frame(c, &bg);
        ui.pack(btns, Pack::default().fill(Fill::X).padx(14, 14).pady(8, 6));
        if self.breath.is_some() {
            let b = tt_btn(ui, btns, "Stop", "gd_breath_stop", None, None, 10, 14, 3, &th, &f);
            ui.pack(b, Pack::default().side(Side::Left));
            let l = ui.label(btns, "Follow the circle~", &bg, &th.gold, italic(&d.serif, 10));
            ui.pack(l, Pack::default().side(Side::Left).padx(10, 10));
        } else {
            let b = tt_btn(ui, btns, "Begin", &format!("gd_breath_start:{cur}"), Some(&th.acc), Some(&th.onacc), 10, 16, 3, &th, &f);
            ui.pack(b, Pack::default().side(Side::Left));
        }
        let calm: Vec<Value> = d.garden["calm"].as_array().cloned().unwrap_or_default();
        let today = d.iso();
        let tmin = zenko_sound::music::py_sum(calm.iter().filter(|x| x["d"].as_str() == Some(&today))
            .map(|x| x["m"].as_f64().unwrap_or(0.0)));
        let wk0 = crate::plan::iso(d.today - 6);
        let wk = calm.iter().filter(|x| x["d"].as_str().unwrap_or("") >= wk0.as_str()).count();
        let tm = if tmin != 0.0 { pct_g((tmin * 10.0).round_ties_even() / 10.0) } else { "0".into() };
        let l = label_w(ui, c, &format!("Today: {} calm min   \u{b7}   {} session{} this week", tm, wk, if wk == 1 { "" } else { "s" }),
                        &bg, &th.grey, &fu, 9, false);
        ui.pack(l, Pack::default().fill(Fill::X).padx(14, 14).pady(0, 12));
        c
    }

    /// zen_cards' Grounding card
    fn zen_ground(&mut self, ui: &mut Ui, p: usize, d: &GardenData) -> usize {
        let (th, f) = (self.th.clone(), Faces { ui: self.f.ui.clone(), title: self.f.title.clone() });
        let bg = th.card.clone();
        let fu = f.ui.clone();
        let gr = gdata()["GROUNDING"].as_array().unwrap();
        let g = hub_card(ui, p, Some("\u{4e94}\u{611f} \u{b7} Grounding 5-4-3-2-1"), "", &th, &f);
        ui.pack(g, Pack::default().fill(Fill::X).pady(12, 0));
        let i = d.ground_i;
        let nxt = if i < 0 {
            let l = label_w(ui, g, "When panic rises before a test, come back to your senses, one step at a time.", &bg, &th.grey,
                            &fu, 9, false);
            justify_left(ui, l, 0);
            ui.pack(l, Pack::default().fill(Fill::X).padx(14, 14));
            "Start"
        } else {
            let e = &gr[i as usize];
            let r = ui.frame(g, &bg);
            ui.pack(r, Pack::default().fill(Fill::X).padx(14, 14));
            let l = ui.label(r, e[0].as_str().unwrap(), &bg, &th.gold, Font::new(&d.serif, 26, false));
            ui.pack(l, Pack::default().side(Side::Left).padx(0, 10));
            let col = ui.frame(r, &bg);
            ui.pack(col, Pack::default().side(Side::Left).fill(Fill::X).expand());
            let l = label_w(ui, col, e[1].as_str().unwrap(), &bg, &th.ink, &fu, 10, true);
            ui.pack(l, Pack::default().fill(Fill::X));
            let l = label_w(ui, col, e[2].as_str().unwrap(), &bg, &th.grey, &fu, 9, false);
            justify_left(ui, l, 0);
            ui.pack(l, Pack::default().fill(Fill::X));
            if (i as usize) < gr.len() - 1 { "Next" } else { "Done" }
        };
        let b = tt_btn(ui, g, nxt, "gd_ground", Some(&th.start), None, 9, 12, 2, &th, &f);
        ui.pack(b, Pack::default().anchor(Anchor::W).padx(14, 14).pady(8, 12));
        g
    }

    /// Zenko22 fish_panel_card
    fn fish_card(&mut self, ui: &mut Ui, p: usize, d: &GardenData) -> usize {
        let (th, f) = (self.th.clone(), Faces { ui: self.f.ui.clone(), title: self.f.title.clone() });
        let bg = th.card.clone();
        let fu = f.ui.clone();
        let g = gdata();
        let fs = &d.fishing;
        let c = hub_card(ui, p, Some("\u{91e3}\u{308a} \u{b7} Koi fishing"), "", &th, &f);
        let (rod, lan, hrs) = d.fish_gear();
        let today = d.iso();
        let used = if fs["free_day"].as_str() == Some(&today) { fs["free_used"].as_i64().unwrap_or(0) } else { 0 };
        let free = 3 - used;
        let wl = 180.max(self.side_w - 60);
        let l = label_w(ui, c, "Cast into the pond, answer the question when a fish bites, and it's yours. Harder water, rarer fish.",
                        &bg, &th.grey, &fu, 9, false);
        justify_left(ui, l, wl);
        ui.pack(l, Pack::default().fill(Fill::X).padx(16, 16));
        let num = |k: &str| fs[k].as_f64().unwrap_or(0.0) as i64;
        let l = label_w(ui, c, &format!("\u{1f359} {} bait  \u{b7}  \u{2728} {} golden  \u{b7}  {} free today", num("bait"), num("gold"),
                                       free.max(0)), &bg, &th.ink, &fu, 10, true);
        justify_left(ui, l, wl);
        ui.pack(l, Pack::default().fill(Fill::X).padx(16, 16).pady(6, 0));
        let rods = g["FISH_RODS"].as_array().unwrap();
        let lans = g["FISH_LANTERNS"].as_array().unwrap();
        let s = |v: &Value| v.as_str().unwrap_or("").to_string();
        let nrod = rods.iter().find(|r| r[2].as_f64().unwrap_or(0.0) > hrs);
        let nlan = lans.iter().find(|r| r[2].as_f64().unwrap_or(0.0) > hrs);
        let gear = format!("{} {}  \u{b7}  {}", s(&rods[rod][0]), s(&rods[rod][1]),
                           if lan > 0 { format!("{} {}", s(&lans[lan][0]), s(&lans[lan][1])) } else { "no lantern yet".into() });
        let l = label_w(ui, c, &gear, &bg, &th.ink, &fu, 9, false);
        ui.pack(l, Pack::default().fill(Fill::X).padx(16, 16));
        let nx: Vec<String> = [nrod, nlan].iter().flatten()
            .map(|r| format!("{} at {} h", s(&r[1]), r[2].as_f64().unwrap_or(0.0) as i64)).collect();
        if !nx.is_empty() {
            let l = label_w(ui, c, &format!("Next: {} (you have {:.1} h)", nx.join(", "), hrs), &bg, &th.grey, &fu, 8, false);
            justify_left(ui, l, wl);
            ui.pack(l, Pack::default().fill(Fill::X).padx(16, 16));
        }
        let row = ui.frame(c, &bg);
        ui.pack(row, Pack::default().fill(Fill::X).padx(14, 14).pady(8, 2));
        let dv = fs["depth"].as_i64().filter(|v| *v != 0).unwrap_or(1);
        let deps = g["FISH_DEPTHS"].as_array().unwrap();
        for (i, dp) in deps.iter().enumerate() {
            let name = s(&dp[1]);
            let t = format!("{} {}", s(&dp[0]), if i < 2 { name.split(' ').next().unwrap_or("").to_string() } else { "Falls".into() });
            let on = dv == i as i64 + 1;
            let b = tt_btn(ui, row, &t, &format!("gd_depth:{}", i + 1), Some(if on { &th.acc } else { &th.field }),
                           Some(if on { &th.onacc } else { &th.ink }), 9, 7, 1, &th, &f);
            ui.pack(b, Pack::default().side(Side::Left).padx(0, 4));
        }
        let dsel = &deps[(dv - 1).clamp(0, 2) as usize];
        let l = label_w(ui, c, &format!("{}: {}", s(&dsel[1]), s(&dsel[2])), &bg, &th.grey, &fu, 8, false);
        ui.pack(l, Pack::default().fill(Fill::X).padx(16, 16));
        let row2 = ui.frame(c, &bg);
        ui.pack(row2, Pack::default().fill(Fill::X).padx(14, 14).pady(6, 0));
        let src = fs["src"].as_str().unwrap_or("mix").to_string();
        for (k, lab) in [("mix", "Mix"), ("cards", "My cards"), ("bank", "Questions")] {
            let on = src == k;
            let b = tt_btn(ui, row2, lab, &format!("gd_src:{k}"), Some(if on { &th.acc } else { &th.field }),
                           Some(if on { &th.onacc } else { &th.ink }), 9, 7, 1, &th, &f);
            ui.pack(b, Pack::default().side(Side::Left).padx(0, 4));
        }
        let subj = fs["subj"].as_str().unwrap_or("any").to_string();
        let sname = g["FISH_Q_SUBJECTS"].as_array().unwrap().iter().find(|q| q[0] == subj.as_str())
            .map(|q| s(&q[1])).unwrap_or_else(|| "Any".into());
        let b = tt_btn(ui, c, &format!("Subject: {sname} \u{25be}"), "gd_subj_menu", None, None, 9, 8, 1, &th, &f);
        ui.pack(b, Pack::default().anchor(Anchor::W).padx(14, 14).pady(4, 0));
        if num("gold") > 0 {
            let on = d.c("fish_gold").as_bool().unwrap_or(false);
            let ck = ui.check(c, "Use golden bait (rare fish bite more)", &bg, &th.ink, Font::new(&fu, 9, false), on, &th.field,
                              "gd_gold", 0, 2);
            ui.pack(ck, Pack::default().fill(Fill::X).padx(14, 14));
        }
        let now = d.fish_now();
        let cap = |w: &str| {
            let mut cs = w.chars();
            cs.next().map(|f| f.to_uppercase().collect::<String>() + cs.as_str()).unwrap_or_default()
        };
        let mut bits = vec![if now.night { "Night".to_string() } else { cap(now.part) }, cap(now.season)];
        if now.rain {
            bits.push("rain".into());
        }
        if now.snow {
            bits.push("snow".into());
        }
        let special: Vec<String> = fish_all().iter().filter(|fi| fi[3].as_i64().unwrap_or(0) >= 4 && d.fish_can(fi, &now, 3, rod, lan))
            .map(|fi| s(&fi[2])).collect();
        let sp = if special.is_empty() { String::new() } else {
            format!("  \u{b7}  could bite: {}", special.iter().take(3).cloned().collect::<Vec<_>>().join(", "))
        };
        let l = label_w(ui, c, &format!("Now: {}{}", bits.join(" \u{b7} "), sp), &bg, &th.grey, &fu, 8, false);
        justify_left(ui, l, wl);
        ui.pack(l, Pack::default().fill(Fill::X).padx(16, 16).pady(6, 0));
        let busy = self.fish.is_some();
        let btns = ui.frame(c, &bg);
        ui.pack(btns, Pack::default().fill(Fill::X).padx(14, 14).pady(8, 0));
        let b = ui.button(btns, if busy { "Waiting for a bite\u{2026}" } else { "\u{1f3a3}  Cast line" }, "gd_cast",
                          if busy { &th.field } else { &th.acc }, if busy { &th.grey } else { &th.onacc }, &fu, 11, 14, 4, !busy);
        ui.pack(b, Pack::default().side(Side::Left));
        let b = tt_btn(ui, btns, "Koi book", "gd_koibook", None, None, 10, 10, 3, &th, &f);
        ui.pack(b, Pack::default().side(Side::Left).padx(6, 0));
        let book = fs["book"].as_object().cloned().unwrap_or_default();
        let n: i64 = book.values().map(|v| v["n"].as_i64().unwrap_or(0)).sum();
        let l = label_w(ui, c, &format!("{} caught  \u{b7}  {} of {} kinds found  \u{b7}  {}/{} right", n, book.len(), fish_all().len(),
                                       num("right"), num("asked")), &bg, &th.grey, &fu, 8, false);
        ui.pack(l, Pack::default().fill(Fill::X).padx(16, 16).pady(6, 12));
        c
    }

    // ---- layout ------------------------------------------------------------------------------------------------

    pub fn fit(&mut self, ui: &mut Ui, fonts: &dyn Fonts, th: &Theme, f: &Faces, zk_f: f64, pw: i32, ph: i32) {
        self.zk_f = zk_f;
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
        // garden_draw: the side panel's width follows the window
        let sw = (260.0 + 60.0 * ((zk_f - 0.85) / 0.6).clamp(0.0, 1.0)) as i32;
        if (ui.ws[self.side].opt_w - sw).abs() > 4 {
            ui.ws[self.side].opt_w = sw;
            self.side_w = sw;
            ui.layout_at(fonts, pw, ph);
        }
        zk_wrap(ui, fonts, self.pg, &mut self.wrap, pw, ph);
        if self.sc.sync(ui) {
            ui.layout_at(fonts, pw, ph);
        }
        if let Some((bar, g, w0)) = self.bar {
            let bw = ui.ws[bar].w;
            if bw != w0 {
                let cv = canvas_of(ui, bar);
                cv.delete("all");
                glow_bar(cv, 1.0, 1.0, (bw - 2) as f64, 10.0, g, th);
                self.bar = Some((bar, g, bw));
            }
        }
        self.draw(ui);
    }

    /// ui["key"] = None: the painting is made again at the next draw
    pub fn forget(&mut self) {
        self.key = None;
    }

    /// the painting made afresh (garden_refresh(rebuild=True) / the key forgotten)
    pub fn redraw(&mut self, ui: &mut Ui) {
        self.key = None;
        self.draw(ui);
    }

    fn cv_size(&self, ui: &Ui) -> (i64, i64) {
        (ui.ws[self.cv].w as i64, ui.ws[self.cv].h as i64)
    }

    /// garden_draw(): the painting made again when its size, stage, health or seed changed
    pub fn draw(&mut self, ui: &mut Ui) {
        let (w, h) = self.cv_size(ui);
        if w < 80 || h < 80 {
            return;
        }
        let z = 2.max((w as f64 / 420.0).max(h as f64 / 250.0).round_ties_even() as i64);
        let (rw, rh) = ((w as f64 / z as f64).ceil() as i64, (h as f64 / z as f64).ceil() as i64);
        let d = self.d.clone();
        let (i, g, _) = d.stage();
        let hp = d.health();
        let grove = d.garden["grove"].as_array().map(|a| a.len()).unwrap_or(0) as i64;
        let seed = d.garden["seed"].as_u64().unwrap_or(7);
        let look = d.look();
        let key = format!("{rw}|{rh}|{z}|{i}|{}|{}|{grove}|{seed}|{look:?}|{w}|{h}", (g * 10.0).round_ties_even(),
                          (hp * 100.0).round_ties_even());
        if self.key.as_deref() == Some(key.as_str()) {
            self.anim_on = true;
            return;
        }
        let (mut r, lay) = garden_scene(rw, rh, i as i64, g, hp, grove, seed, &look);
        let zk = self.garden_zenko(&mut r, &lay, z, i, hp);
        let img = Rc::new(zoom_px(r.w as usize, r.h as usize, &r.rgba(), z as usize));
        let cv = canvas_of(ui, self.cv);
        cv.delete("all");
        cv.image(0.0, 0.0, img.clone()).anchor("nw").tags(&["scene"]);
        self.img = Some(img);
        self.ras = Some(r);
        self.lay = Some(lay);
        self.z = z;
        self.key = Some(key);
        self.dimmed = None;
        self.overlay(ui, zk);
        self.anim_on = true;
    }

    fn glows(&mut self) -> [Rc<Photo>; 3] {
        if self.glows.is_none() {
            let c = |cx: f64, r: f64, col: &str, a: f64| -> Sh { (Shape::Circle(cx, cx, r), Some((col.into(), a))) };
            self.glows = Some([
                Rc::new(aa_pixels(40, 40, &[c(20.0, 19.0, "#ffcf7a", 0.10), c(20.0, 12.0, "#ffcf7a", 0.16), c(20.0, 6.0, "#ffe7b0", 0.5),
                                            c(20.0, 2.5, "#fff6dc", 1.0)], None)),
                Rc::new(aa_pixels(36, 36, &[c(18.0, 17.0, "#7fd8ff", 0.10), c(18.0, 10.0, "#9fe4ff", 0.2), c(18.0, 5.0, "#d6f6ff", 0.6),
                                            c(18.0, 2.0, "#ffffff", 1.0)], None)),
                Rc::new(aa_pixels(90, 90, &[c(45.0, 44.0, "#ffb35a", 0.06), c(45.0, 30.0, "#ffb35a", 0.08), c(45.0, 16.0, "#ffcf8a", 0.12)],
                                  None)),
            ]);
        }
        self.glows.clone().unwrap()
    }

    /// garden_overlay_setup() with Zenko22's decor and pond fish
    fn overlay(&mut self, ui: &mut Ui, zk: Option<ZkSprite>) {
        let (w, h) = self.cv_size(ui);
        let (wf, hf) = (w as f64, h as f64);
        let lay = self.lay.clone().unwrap();
        let z = self.z as f64;
        let mut rnd = PyRandom::new(3);
        let (i, g, _) = self.d.stage();
        let hp = self.d.health();
        let [glow, fox, lglow] = self.glows();
        let cv = canvas_of(ui, self.cv);
        self.lamps.clear();
        for &(lx, ly) in &lay.lanterns {
            cv.image(lx * z, ly * z, lglow.clone()).tags(&["fx"]);
            self.lamps.push(cv.last_id());
        }
        self.flies.clear();
        for k in 0..9 {
            let x = rnd.uniform(0.05, 0.95) * wf;
            let y = rnd.uniform(lay.horizon as f64 * z * 0.9, hf * 0.95);
            cv.image(x, y, if k < 3 { fox.clone() } else { glow.clone() }).tags(&["fx"]);
            let id = cv.last_id();
            let ph = rnd.uniform(0.0, 6.28);
            let sp = rnd.uniform(0.4, 1.0);
            self.flies.push(Fly { id, x, y, ph, sp, a: ph });
        }
        self.petals.clear();
        let blossom = i >= 6 || (i == 5 && g > 0.4);
        self.petal_rate = if blossom { 0.6 + 1.6 * hp } else if hp < 0.6 && i >= 2 { 0.5 } else { 0.0 };
        self.petal_col = if blossom && hp >= 0.4 { ["#ffd1e1", "#ffb3cd", "#ffe4ee"] } else { ["#c4955a", "#a8794a", "#d8b07a"] };
        if let Some(zk) = zk {
            cv.image(zk.x as f64, zk.y as f64, zk.photo).anchor("nw").tags(&["fx"]);
            if let Some(front) = zk.front {
                cv.image(zk.fx as f64, zk.fy as f64, front).anchor("nw").tags(&["fx"]);
            }
        }
        // breath_items()
        cv.delete("breath");
        let serif = self.d.serif.clone();
        let fu = self.f.ui.clone();
        cv.oval(0.0, 0.0, 0.0, 0.0).outline("#7d82b8").width(1.0).dash(&[2, 5]).hide(true).tags(&["breath", "fx"]);
        let track = cv.last_id();
        cv.image_none(0.0, 0.0).hide(true).tags(&["breath", "fx"]);
        let orb = cv.last_id();
        let mut ids = [track, orb, 0, 0, 0, 0, 0, 0];
        for (j, (fill, fam, n)) in [("#0a0c1e", serif.as_str(), 26), ("#f4f5ff", serif.as_str(), 26), ("#05060f", serif.as_str(), 18),
                                    ("#eef0ff", serif.as_str(), 18), ("#b8bce0", fu.as_str(), 10)].into_iter().enumerate() {
            cv.text(0.0, 0.0, "").fill(fill).font(Font::new(fam, n, false)).hide(true).tags(&["breath", "fx"]);
            ids[2 + j] = cv.last_id();
        }
        cv.text(0.0, 0.0, "").fill("#c9cdf0").font(Font::new(&fu, 9, false)).anchor("nw").hide(true).tags(&["breath", "fx"]);
        ids[7] = cv.last_id();
        self.bitems = ids;
        cv.tag_raise("fx");
        // Zenko22: decor and your fish
        self.decor(ui);
        self.swim.clear();
        let fs = &self.d.fishing;
        let mut pond: Vec<(String, i64)> = fs["pond"].as_array().map(|a| a.iter().filter_map(|p| {
            let fid = p[0].as_str()?;
            fish_by_id(fid)?;
            Some((fid.to_string(), p[1].as_i64().unwrap_or(1)))
        }).take(6).collect()).unwrap_or_default();
        if pond.is_empty() {
            pond = vec![("kohaku".into(), 1), ("higoi".into(), 2), ("chagoi".into(), 3)];
        }
        let prx = lay.pond.2;
        let mut rnd = PyRandom::new(11);
        for (fid, seed) in pond {
            let body = fish_by_id(&fid).unwrap()[4].as_str().unwrap().to_string();
            let mut l = 9.max((prx * if body != "oar" { 0.34 } else { 0.7 }) as i64);
            if body == "medaka" {
                l = 6.max(l.div_euclid(2));
            }
            let x = rnd.uniform(-0.5, 0.5);
            let y = rnd.uniform(-0.4, 0.4);
            let a = rnd.uniform(0.0, 6.28);
            let sp = rnd.uniform(0.0025, 0.0045);
            self.swim.push(Swim { fid, seed, l, x, y, a, sp, item: None, k: -1 });
        }
        if let Some(f) = self.fish.as_mut() {
            f.items = None;
            f.ripples.clear();
            f.jump = None;
        }
        canvas_of(ui, self.cv).tag_raise("fx");
    }

    /// z22_decor(): the garden decor you earned on the world map
    fn decor(&mut self, ui: &mut Ui) {
        let lay = self.lay.clone().unwrap();
        let (rw, rh) = {
            let r = self.ras.as_ref().unwrap();
            (r.w as f64, r.h as f64)
        };
        let z = self.z;
        canvas_of(ui, self.cv).delete("decor");
        let rewards = crate::rewards::rdata()["MAP_REWARDS"].as_array().cloned().unwrap_or_default();
        let have: Vec<String> = rewards.iter().filter(|r| r[1] == "decor" && self.d.got.iter().any(|g| r[0] == g.as_str()))
            .filter_map(|r| r[2].as_str().map(String::from)).collect();
        let want: Vec<String> = match self.d.c("garden_decor").as_array() {
            Some(a) if !a.is_empty() => a.iter().filter_map(|v| v.as_str().map(String::from)).collect(),
            _ => have.clone(),
        };
        let on: Vec<String> = want.into_iter().filter(|k| have.contains(k)).collect();
        let (pcx, pcy, prx, pry) = lay.pond;
        let hz = lay.horizon as f64;
        for k in on {
            let (x, y) = match k.as_str() {
                "deer" => (rw * 0.12, rh * 0.93),
                "seatorii" => (pcx - prx * 0.45, pcy + pry * 0.2),
                "momiji" => (rw * 0.06, hz + rh * 0.08),
                "cheong" => (rw * 0.62, hz + rh * 0.06),
                "pine" => (rw * 0.95, hz + rh * 0.07),
                "camel" => (rw * 0.52, rh * 0.95),
                "rabbit" => (rw * 0.25, rh * 0.96),
                "kitsune" => (rw * 0.69, rh * 0.93),
                _ => continue,
            };
            let dz = 1.max(z);
            let img = self.decor_img.entry((k.clone(), dz)).or_insert_with(|| {
                let art = &gdata()["Z22_DECOR_ART"][k.as_str()];
                let rows: Vec<&str> = art[0].as_array().unwrap().iter().map(|r| r.as_str().unwrap()).collect();
                let mut pal = Palette::empty();
                for (ch, hx) in art[1].as_object().unwrap() {
                    pal.set(ch.as_bytes()[0], hx.as_str().unwrap());
                }
                Rc::new(crate::hubside::sprite_photo(&rows, &pal, dz as usize, self.d.hd))
            }).clone();
            canvas_of(ui, self.cv).image(x * z as f64, y * z as f64, img).anchor("s").tags(&["decor", "fx"]);
        }
    }

    /// garden_zenko(): Zenko lit by the scene, her contact shadow painted onto the moss, grass in front of her paws
    fn garden_zenko(&self, r: &mut Ras, lay: &Lay, z: i64, stage: usize, hp: f64) -> Option<ZkSprite> {
        let pose = if hp < 0.35 { "sleep" } else if hp >= 0.95 && stage >= 6 { "happy" } else { "idle" };
        let rows = zenko_art::sit::build_frame(pose, 0, Some((0, 0, 0, 0)), &[], 0, 0, &self.d.look);
        let tails = zenko_art::core::build_tails(self.d.tails, 0, 8, false);
        let (sh, sw) = (rows.h as i64, rows.w as i64);
        let mut px: Vec<Option<Rgb>> = vec![None; (sh * sw) as usize];
        for layer in [&tails, &rows] {
            for y in 0..layer.h.min(rows.h) {
                for x in 0..layer.w.min(rows.w) {
                    let ch = layer.at(x, y);
                    if ch != b'.'
                        && let Some(c) = self.d.pal.get(ch)
                    {
                        px[(y as i64 * sw + x as i64) as usize] = Some((c[0], c[1], c[2]));
                    }
                }
            }
        }
        let at = |x: i64, y: i64| px[(y * sw + x) as usize];
        let ys: Vec<i64> = (0..sh).filter(|&y| (0..sw).any(|x| at(x, y).is_some())).collect();
        let xs: Vec<i64> = (0..sw).filter(|&x| (0..sh).any(|y| at(x, y).is_some())).collect();
        if ys.is_empty() || xs.is_empty() {
            return None;
        }
        let (y0, y1, x0, x1) = (ys[0], *ys.last().unwrap(), xs[0], *xs.last().unwrap());
        let fh = y1 - y0 + 1;
        let (rw, rh) = (r.w as f64, r.h as f64);
        let zf = z as f64;
        let tb = lay.tree.1 as f64;
        let (feet_x, feet_y) = (rw * 0.355, tb + rh * 0.045);
        let hz = lay.horizon as f64;
        let depth = (feet_y - hz) / 1.0f64.max(rh - hz);
        let target = rh * zf * (0.17 + 0.10 * depth);
        let zs = 1.max((target / fh as f64).round_ties_even() as i64);
        let lamps = if lay.lanterns.is_empty() { vec![(rw * 0.16, rh * 0.7)] } else { lay.lanterns.clone() };
        let mut best = lamps[0];
        for &p in &lamps[1..] {
            if (p.0 - feet_x).abs() < (best.0 - feet_x).abs() {
                best = p;
            }
        }
        let lx = best.0;
        let warm_left = lx < feet_x;
        let dist = (lx - feet_x).abs() / 1.0f64.max(rw);
        let warm_a = 0.18f64.max(0.55 - dist * 1.2);
        let amb = (0.86, 0.84, 0.99);
        let (warm, cool) = ((255.0, 186.0, 112.0), (205.0, 216.0, 255.0));
        let solid = |x: i64, y: i64| 0 <= x && x < sw && 0 <= y && y < sh && at(x, y).is_some();
        let mut out = Vec::with_capacity(px.len());
        for y in 0..sh {
            let t = (y - y0) as f64 / 1.max(fh - 1) as f64;
            let ao = 1.0 - 0.30 * 0.0f64.max((t - 0.62) / 0.38);
            for x in 0..sw {
                let Some(c) = at(x, y) else {
                    out.push([0, 0, 0, 0]);
                    continue;
                };
                let (c0, c1, c2) = (c.0 as f64, c.1 as f64, c.2 as f64);
                let (mut rr, mut gg, mut bb) = (c0 * amb.0 * ao, c1 * amb.1 * ao, c2 * amb.2 * ao);
                let lum = 0.3 * c0 + 0.59 * c1 + 0.11 * c2;
                if lum > 60.0 {
                    let side = if warm_left { -1 } else { 1 };
                    if !solid(x + side, y) || !solid(x + 2 * side, y) {
                        let a = if !solid(x + side, y) { warm_a } else { warm_a * 0.5 };
                        (rr, gg, bb) = (rr + (warm.0 - rr) * a, gg + (warm.1 - gg) * a, bb + (warm.2 - bb) * a);
                    } else if !solid(x, y - 1) || !solid(x - side, y) {
                        let a = 0.28;
                        (rr, gg, bb) = (rr + (cool.0 - rr) * a, gg + (cool.1 - gg) * a, bb + (cool.2 - bb) * a);
                    }
                }
                out.push([(rr as i64).min(255) as u8, (gg as i64).min(255) as u8, (bb as i64).min(255) as u8, 255]);
            }
        }
        let photo = Rc::new(zoom_px(sw as usize, sh as usize, &out, zs as usize));
        let cxs = (x0 + x1) as f64 / 2.0;
        let sx = (feet_x * zf - cxs * zs as f64) as i64;
        let sy = (feet_y * zf - ((y1 + 1) * zs) as f64) as i64;
        let fw = ((x1 - x0 + 1) * zs) as f64 / zf;
        let (scx, scy) = (feet_x, feet_y - 0.5);
        let push = (fw * 0.18) * if warm_left { 1.0 } else { -1.0 };
        for yy in (scy - fw * 0.18) as i64 - 1..(scy + fw * 0.18) as i64 + 2 {
            for xx in (scx - fw * 0.75 + 0.0f64.min(push)) as i64 - 1..(scx + fw * 0.75 + 0.0f64.max(push)) as i64 + 2 {
                let dx = (xx as f64 + 0.5 - (scx + push * 0.5)) / (fw * 0.62 + push.abs() * 0.5);
                let dy = (yy as f64 + 0.5 - scy) / (fw * 0.15);
                let dd = dx * dx + dy * dy;
                if dd < 1.0 && 0 <= xx && (xx as f64) < rw && 0 <= yy && (yy as f64) < rh {
                    let mut a = (1.0 - dd).powf(0.7) * 0.78;
                    if dd < 0.3 {
                        a = a.max(0.7);
                    }
                    if a < 0.25 && BAYER[(yy % 4) as usize][(xx % 4) as usize] as f64 / 16.0 > a * 3.0 {
                        continue;
                    }
                    r.put(xx as f64, yy as f64, (6, 8, 22), a);
                }
            }
        }
        let gw = (fw * 1.0) as i64 + 6;
        let gh = 5.max((rh * 0.032) as i64);
        let mut front = None;
        if gw > 4 {
            let mut rnd = PyRandom::new((feet_x * 7.0 + rh) as u64);
            let mut gp = vec![[0u8; 4]; (gw * gh) as usize];
            let base = r.get(((rw - 1.0) as i64).min((feet_x + fw * 0.6) as i64), ((rh - 1.0) as i64).min((feet_y + 2.0) as i64));
            let greens: Vec<Rgb> = [(1.15, 6.0), (1.35, 10.0), (0.9, 0.0), (1.6, 14.0)].iter().map(|&(f, o)| {
                let q = |c: u8| ((c as f64 * f + o) as i64).clamp(0, 255) as u8;
                (q(base.0), q(base.1), q(base.2))
            }).collect();
            let clumps: Vec<f64> = (0..3.max(gw / 9)).map(|_| rnd.uniform(0.05, 0.95) * gw as f64).collect();
            for cxp in clumps {
                let n = rnd.randint(3, 6);
                for k in 0..n {
                    let fan = (k as f64 - (n - 1) as f64 / 2.0) * 0.55;
                    let hgt = (gh as f64 * rnd.uniform(0.45, 1.0)) as i64;
                    let col = *rnd.choice(&greens);
                    for j in 0..hgt {
                        let yy = gh - 1 - j;
                        let xx = (cxp + fan * j as f64 * 0.6).round_ties_even() as i64;
                        if 0 <= xx && xx < gw {
                            let c2 = if j >= hgt - 1 { (col.0.saturating_add(22), col.1.saturating_add(22), col.2.saturating_add(22)) }
                                     else { col };
                            gp[(yy * gw + xx) as usize] = hexc(c2);
                        }
                    }
                }
            }
            front = Some(Rc::new(zoom_px(gw as usize, gh as usize, &gp, z as usize)));
        }
        let fxp = ((feet_x - gw as f64 / 2.0) * zf) as i64;
        let fyp = ((feet_y + 1.0) * zf - (gh * z) as f64) as i64;
        Some(ZkSprite { photo, x: sx, y: sy, front, fx: fxp, fy: fyp })
    }

    // ---- the living garden -------------------------------------------------------------------------------------

    fn fish_photo(&mut self, fid: &str, seed: i64, l: i64, zoom: i64, angle: f64, squash: f64) -> Rc<Photo> {
        let key = (fid.to_string(), seed, l, zoom, (angle * 1000.0).round_ties_even() as i64, squash.to_bits());
        if let Some(p) = self.fish_img.get(&key) {
            return p.clone();
        }
        let (w, h, px) = fish_rgba(fid, seed, l, angle, squash);
        let p = Rc::new(zoom_px(w, h, &px, zoom as usize));
        if self.fish_img.len() > 600 {
            self.fish_img.clear();
        }
        self.fish_img.insert(key, p.clone());
        p
    }

    /// one frame of garden_anim (every 50 ms while the page shows); rnd is random.random
    pub fn frame(&mut self, ui: &mut Ui, t: f64, rnd: &mut dyn FnMut() -> f64) -> Vec<GAct> {
        let mut acts = vec![];
        if self.lay.is_none() {
            return acts;
        }
        let (w, h) = self.cv_size(ui);
        let (wf, hf) = (w as f64, h as f64);
        let z = self.z as f64;
        let lay = self.lay.clone().unwrap();
        self.breath_frame(ui, t, &mut acts);
        let cv = canvas_of(ui, self.cv);
        for (k, &it) in self.lamps.iter().enumerate() {
            let kf = k as f64;
            let f = 0.5 + 0.5 * (t * 3.1 + kf * 1.7).sin() * (t * 1.3 + kf).sin();
            cv.get(it).unwrap().hidden = f <= -0.6;
        }
        for fl in self.flies.iter_mut() {
            fl.a += 0.02 * fl.sp;
            let nx = fl.x + (fl.a * 1.3).sin() * 26.0 * fl.sp;
            let ny = fl.y + (fl.a * 2.1 + 1.0).sin() * 14.0 * fl.sp;
            let it = cv.get(fl.id).unwrap();
            it.coords = vec![nx, ny];
            it.hidden = (t * 1.7 + fl.ph * 3.0).sin() <= -0.55;
        }
        let rate = self.petal_rate;
        if rate != 0.0 && rnd() < rate * 0.12 && self.petals.len() < 46 {
            let tips = if lay.tips.is_empty() { vec![(lay.tree.0 as f64, lay.tree.1 as f64 - 40.0, 0)] } else { lay.tips.clone() };
            let (tx, ty, _) = tips[((rnd() * tips.len() as f64) as usize).min(tips.len() - 1)];
            let x = tx * z + (rnd() * 12.0 - 6.0) * z;
            let y = ty * z;
            let s = (1.4 + rnd()) * 1.0f64.max(z * 0.6);
            let col = self.petal_col[((rnd() * 3.0) as usize).min(2)];
            cv.poly(vec![x, y, x + s, y - s * 0.6, x + s * 1.8, y, x + s, y + s * 0.5]).fill(col).outline("").tags(&["fx"]);
            let id = cv.last_id();
            let vy = (0.25 + rnd() * 0.35) * z;
            let ph = rnd() * 6.28;
            let floor = lay.tree.1 as f64 * z + rnd() * hf * 0.18;
            self.petals.push(Petal { id, vy, ph, floor });
        }
        let mut keep = vec![];
        for mut p in std::mem::take(&mut self.petals) {
            p.ph += 0.08;
            cv.move_items(&p.id.to_string(), p.ph.sin() * 0.9 * z * 0.5 + 0.25 * z, p.vy);
            let y = cv.get(p.id).map(|i| i.coords[1]).unwrap_or(f64::MAX);
            if y < p.floor {
                keep.push(p);
            } else {
                cv.delete(&p.id.to_string());
            }
        }
        self.petals = keep;
        let _ = wf;
        // Zenko22: your fish swim, and the cast
        self.pond_frame(ui, t, rnd, &mut acts);
        acts
    }

    /// z22_pond_frame
    fn pond_frame(&mut self, ui: &mut Ui, t: f64, rnd: &mut dyn FnMut() -> f64, acts: &mut Vec<GAct>) {
        let lay = self.lay.clone().unwrap();
        let (pcx, pcy, prx, pry) = lay.pond;
        let z = self.z;
        let bait = self.fish.as_ref().filter(|f| f.phase == "wait" || f.phase == "bite").map(|f| (f.bx, f.by));
        let mut swim = std::mem::take(&mut self.swim);
        for s in swim.iter_mut() {
            let mut want = match bait {
                Some((bx, by)) => (by - s.y).atan2(bx - s.x),
                None => s.a + (t * 0.3 + s.seed as f64).sin() * 0.6,
            };
            let r = zenko_art::core::hypot(s.x, s.y / 0.85);
            if r > 0.72 {
                want = (-s.y).atan2(-s.x);
            }
            let d = (want - s.a + PI).rem_euclid(2.0 * PI) - PI;
            s.a += (d * 0.06).clamp(-0.05, 0.05);
            s.x += s.a.cos() * s.sp;
            s.y += s.a.sin() * s.sp;
            let k = ((s.a.rem_euclid(2.0 * PI)) / (2.0 * PI) * 16.0).round_ties_even() as i64 % 16;
            let (x, y) = ((pcx + s.x * prx) * z as f64, (pcy + s.y * pry) * z as f64);
            if k != s.k {
                let img = self.fish_photo(&s.fid, s.seed, s.l, z, k as f64 * PI / 8.0, 0.5);
                s.k = k;
                let cv = canvas_of(ui, self.cv);
                match s.item {
                    None => {
                        cv.image(x, y, img).tags(&["fx"]);
                        s.item = Some(cv.last_id());
                    }
                    Some(it) => cv.get(it).unwrap().image = Some(img),
                }
            }
            canvas_of(ui, self.cv).get(s.item.unwrap()).unwrap().coords = vec![x, y];
        }
        self.swim = swim;
        if self.fish.is_some() {
            self.cast_frame(ui, t, rnd, acts);
        }
    }

    pub fn ripple(&mut self, ui: &mut Ui, x: f64, y: f64, small: bool, delay: f64, t: f64) {
        if self.fish.is_none() {
            return;
        }
        let cv = canvas_of(ui, self.cv);
        cv.oval(x, y, x, y).outline("#cfe2ff").width(1.0).tags(&["fx"]);
        let it = cv.last_id();
        self.fish.as_mut().unwrap().ripples.push((it, x, y, t + delay, !small));
    }

    /// z22_cast_frame
    fn cast_frame(&mut self, ui: &mut Ui, t: f64, rnd: &mut dyn FnMut() -> f64, acts: &mut Vec<GAct>) {
        let (w, h) = self.cv_size(ui);
        let (wf, hf) = (w as f64, h as f64);
        let lay = self.lay.clone().unwrap();
        let (pcx, pcy, prx, pry) = lay.pond;
        let z = self.z as f64;
        let tip = (wf - 14.0, hf * 0.45);
        let mut fish = self.fish.clone().unwrap();
        let (bx, by) = ((pcx + fish.bx * prx) * z, (pcy + fish.by * pry) * z);
        let (ph, el) = (fish.phase.clone(), t - fish.t);
        let (x, mut y);
        let mut ripples: Vec<(bool, f64)> = vec![];
        if ph == "cast" {
            let k = 1.0f64.min(el / 0.7);
            let (sx, sy) = tip;
            x = sx + (bx - sx) * k;
            y = sy + (by - sy) * k - (k * PI).sin() * hf * 0.18;
            if k >= 1.0 {
                fish.phase = "wait".into();
                fish.t = t;
                fish.bite_at = t + 1.6 + rnd() * 2.6;
                ripples.push((false, 0.0));
                acts.push(GAct::Sound("tap"));
            }
        } else {
            x = bx;
            y = by;
            if ph == "wait" {
                y += (t * 3.2).sin() * z * 0.6;
                if rnd() < 0.012 {
                    y += z * 1.2;
                    ripples.push((true, 0.0));
                }
                if t >= fish.bite_at {
                    fish.phase = "bite".into();
                    fish.t = t;
                    ripples.push((false, 0.0));
                    ripples.push((false, 0.2));
                    acts.push(GAct::Sound("tap"));
                    acts.push(GAct::FishAsk);
                }
            } else if ph == "bite" {
                y += z * 2.4 + (t * 18.0).sin() * z;
            } else if ph == "asking" {
                y += z * 1.5 + (t * 9.0).sin() * z * 0.6;
            } else if ph == "caught" || ph == "lost" {
                y += -z * 2.0 * 0.0f64.max(1.0 - el * 3.0);
            }
        }
        self.fish = Some(fish);
        for (small, delay) in ripples {
            self.ripple(ui, bx, by, small, delay, t);
        }
        let gold = self.th.gold.clone();
        let fu = self.f.ui.clone();
        let mut fish = self.fish.take().unwrap();
        let cv = canvas_of(ui, self.cv);
        let it = match fish.items {
            Some(it) => it,
            None => {
                let mut ids = [0u32; 6];
                cv.line(vec![0.0, 0.0, 0.0, 0.0]).fill("#6a4a2a").width(3.0f64.max(z + 1.0)).cap("round").tags(&["fx"]);
                ids[0] = cv.last_id();
                cv.line(vec![0.0, 0.0, 0.0, 0.0]).fill("#c9a26a").width(1.0f64.max(z - 1.0)).cap("round").tags(&["fx"]);
                ids[1] = cv.last_id();
                cv.line(vec![0.0; 6]).fill("#e8eef8").width(1.0).smooth().tags(&["fx"]);
                ids[2] = cv.last_id();
                cv.oval(0.0, 0.0, 0.0, 0.0).fill("#e0442e").outline("#fff6e8").width(1.0).tags(&["fx"]);
                ids[3] = cv.last_id();
                cv.rect(0.0, 0.0, 0.0, 0.0).fill("#fff6e8").outline("").tags(&["fx"]);
                ids[4] = cv.last_id();
                cv.text(0.0, 0.0, "!").fill(&gold).font(Font::new(&fu, 12.max(5 * z as i32), true)).hide(true).tags(&["fx"]);
                ids[5] = cv.last_id();
                fish.items = Some(ids);
                ids
            }
        };
        let ph = fish.phase.clone();
        cv.get(it[0]).unwrap().coords = vec![wf + 6.0, hf + 6.0, tip.0, tip.1];
        cv.get(it[1]).unwrap().coords = vec![wf + 6.0, hf + 6.0, tip.0, tip.1];
        let sag = if ph == "cast" { 0.0 } else if ph == "wait" { hf * 0.05 } else { hf * 0.01 };
        cv.get(it[2]).unwrap().coords = vec![tip.0, tip.1, (tip.0 + x) / 2.0, (tip.1 + y) / 2.0 + sag, x, y];
        let r = 2.5f64.max(z * 1.6);
        let under = ph == "bite" && ((t * 6.0) as i64) % 2 == 0;
        cv.get(it[3]).unwrap().coords = vec![x - r, y - r, x + r, y + r];
        cv.get(it[4]).unwrap().coords = vec![x - r * 0.9, y - r * 0.2, x + r * 0.9, y + r * 0.2];
        cv.get(it[3]).unwrap().hidden = under;
        cv.get(it[4]).unwrap().hidden = under;
        cv.get(it[5]).unwrap().coords = vec![x, y - r - 6.0 * z];
        cv.get(it[5]).unwrap().hidden = !(ph == "bite" || ph == "asking");
        let mut keep = vec![];
        for rp in std::mem::take(&mut fish.ripples) {
            let (item, x0, y0, t0, big) = rp;
            let a = t - t0;
            if a < 0.0 {
                keep.push(rp);
                continue;
            }
            if a > 1.2 {
                cv.delete(&item.to_string());
                continue;
            }
            let rr = (3.0 + a * if big { 16.0 } else { 7.0 }) * z;
            let ci = cv.get(item).unwrap();
            ci.coords = vec![x0 - rr, y0 - rr * 0.35, x0 + rr, y0 + rr * 0.35];
            ci.outline = mixhex("#cfe2ff", "#0e1436", 1.0f64.min(a / 1.2));
            keep.push(rp);
        }
        fish.ripples = keep;
        if let Some((jt, dx, item)) = fish.jump {
            let a = t - jt;
            if a > 1.1 {
                cv.delete(&item.to_string());
                fish.jump = None;
            } else {
                let k = a / 1.1;
                cv.get(item).unwrap().coords = vec![bx + dx * k, by - (k * PI).sin() * hf * 0.22];
                cv.tag_raise(&item.to_string());
            }
        }
        self.fish = Some(fish);
    }

    /// the foxfire flicker and the growth bar (230 ms)
    pub fn flicker(&mut self, ui: &mut Ui, now: f64) -> bool {
        if now < self.flick_at {
            return false;
        }
        self.flick_at = now + 0.23;
        let kk = self.flick_k;
        self.flick_k += 1;
        let Some((cv, flames)) = &self.flick else { return false };
        let c = canvas_of(ui, *cv);
        for (j, &(o, cx, b, fh, fw)) in flames.iter().enumerate() {
            let m = (kk + j as u64) % 3;
            let lean = if m == 0 { 0.18 } else if m == 1 { -0.14 } else { 0.0 };
            c.get(o).unwrap().coords = flame(cx, b, fh, fw, lean);
        }
        true
    }

    // ---- breathing ---------------------------------------------------------------------------------------------

    /// breath_dim(): the painting at half light while you breathe
    fn dim(&mut self, ui: &mut Ui) {
        if self.dimmed == self.key || self.ras.is_none() {
            return;
        }
        let r = self.ras.as_ref().unwrap();
        let px: Vec<[u8; 4]> = r.b.chunks(3).map(|p| {
            let q = |v: u8| (v as f64 * 0.52) as u8;
            [q(p[0]), q(p[1]), q(p[2]), 255]
        }).collect();
        let img = Rc::new(zoom_px(r.w as usize, r.h as usize, &px, self.z as usize));
        let cv = canvas_of(ui, self.cv);
        for id in cv.find_withtag("scene") {
            cv.get(id).unwrap().image = Some(img.clone());
        }
        self.img_dim = Some(img);
        self.dimmed = self.key.clone();
    }

    /// breath_stop(): the guide hides and the painting lights up again
    pub fn breath_stop(&mut self, ui: &mut Ui) {
        self.breath = None;
        let cv = canvas_of(ui, self.cv);
        for id in cv.find_withtag("breath") {
            cv.get(id).unwrap().hidden = true;
        }
        if let Some(img) = self.img.clone() {
            for id in cv.find_withtag("scene") {
                cv.get(id).unwrap().image = Some(img.clone());
            }
        }
        self.dimmed = None;
    }

    /// breath_finish's "Well done~" over the garden
    pub fn well_done(&mut self, ui: &mut Ui, now: f64) {
        let (w, h) = self.cv_size(ui);
        let size = zk_size(self.zk_f, 22);
        let serif = self.d.serif.clone();
        let cv = canvas_of(ui, self.cv);
        cv.delete("bdone");
        for (dx, col) in [(2.0, "#05060f"), (0.0, "#eef0ff")] {
            cv.text(w as f64 / 2.0 + dx, h as f64 * 0.30 + dx, "Well done~").fill(col).font(Font::new(&serif, size, false))
                .tags(&["bdone", "fx"]);
        }
        self.done_until = now + 3.5;
    }

    pub fn done_clear(&mut self, ui: &mut Ui, now: f64) -> bool {
        if self.done_until > 0.0 && now >= self.done_until {
            self.done_until = 0.0;
            canvas_of(ui, self.cv).delete("bdone");
            return true;
        }
        false
    }

    /// breath_frame()
    fn breath_frame(&mut self, ui: &mut Ui, now: f64, acts: &mut Vec<GAct>) {
        let Some(mut b) = self.breath.clone() else { return };
        self.dim(ui);
        let z = self.z;
        let (w, h) = self.cv_size(ui);
        let (wf, hf) = (w as f64, h as f64);
        let rh = hf / z as f64;
        let e = now - b.t0;
        if e >= b.total {
            acts.push(GAct::BreathDone(b.key.clone(), (b.total / 60.0 * 10.0).round_ties_even() / 10.0));
            return;
        }
        let (k, ph, s, cyc, acc, rnd_i, size);
        if e < 0.0 {
            (k, ph, s, cyc, acc, rnd_i) = (-1i64, "rest".to_string(), 2.0, 0.0, 0.0, 0i64);
            size = 0.35;
            let _ = k;
        } else {
            cyc = e.rem_euclid(b.cycle);
            rnd_i = (e / b.cycle).floor() as i64;
            let mut a = 0.0;
            let mut kk = 0usize;
            for (j, (_, st)) in b.steps.iter().enumerate() {
                kk = j;
                if cyc < a + st {
                    break;
                }
                a += st;
            }
            acc = a;
            ph = b.steps[kk].0.clone();
            s = b.steps[kk].1;
            let p = (cyc - acc) / s;
            let ease = 0.5 - 0.5 * (PI * p).cos();
            let mut sz = b.starts[kk] + (b.ends[kk] - b.starts[kk]) * ease;
            if ph == "hold" || ph == "rest" {
                sz += 0.012 * (now * 4.0).sin();
            }
            size = sz;
            let gid = rnd_i * b.steps.len() as i64 + kk as i64;
            if Some(gid) != b.phase {
                b.phase = Some(gid);
                if self.d.c("breath_sound").as_bool().unwrap_or(true) {
                    acts.push(GAct::BreathCue(ph.clone(), s));
                }
            }
            k = kk as i64;
            let _ = k;
        }
        self.breath = Some(b.clone());
        let bx = wf / 2.0;
        let (rmin, rmax) = (rh * 0.065, rh * 0.155);
        let by = hf * 0.30;
        let r = rmin + (rmax - rmin) * 0.0f64.max((size - 0.35) / 0.65);
        let okey = (3.max(r as i64), z, ph.clone());
        let orb = match self.orbs.get(&okey) {
            Some(o) => o.clone(),
            None => {
                let o = Rc::new(orb_px(r, z, &ph));
                if self.orbs.len() > 160 {
                    self.orbs.clear();
                }
                self.orbs.insert(okey, o.clone());
                o
            }
        };
        let r1 = rmax * z as f64;
        let (label, sub, count) = if e < 0.0 {
            ("Get comfy".to_string(), "sit back, let your shoulders drop".to_string(), String::new())
        } else {
            let lp = &gdata()["BREATH_PHASE"][ph.as_str()];
            (lp[0].as_str().unwrap_or("").to_string(), lp[1].as_str().unwrap_or("").to_string(),
             1.max((s - (cyc - acc) - 1e-6).ceil() as i64).to_string())
        };
        let fs = zk_size(self.zk_f, 17);
        let serif = self.d.serif.clone();
        let fu = self.f.ui.clone();
        let ly = by + r1 + 18.0f64.max(fs as f64 * 1.6);
        let cs = zk_size(self.zk_f, 24);
        let left = 0.max((b.total - 0.0f64.max(e)) as i64);
        let name = gdata()["BREATH_PATTERNS"][b.key.as_str()]["name"].as_str().unwrap_or("").to_string();
        let head = format!("{}   \u{b7}   round {} of {}   \u{b7}   {}:{:02} left", name, b.rounds.min(rnd_i + 1), b.rounds,
                           left / 60, left % 60);
        let hs = zk_size(self.zk_f, 9);
        let ss = zk_size(self.zk_f, 10);
        let [track, orb_id, cnt_s, cnt, lab_s, lab, sub_id, head_id] = self.bitems;
        let cv = canvas_of(ui, self.cv);
        let it = cv.get(orb_id).unwrap();
        it.image = Some(orb);
        it.hidden = false;
        it.coords = vec![bx, by];
        let it = cv.get(track).unwrap();
        it.coords = vec![bx - r1, by - r1, bx + r1, by + r1];
        it.hidden = false;
        for (id, dx) in [(lab_s, 2.0), (lab, 0.0)] {
            let it = cv.get(id).unwrap();
            it.text = label.clone();
            it.hidden = false;
            it.font = Some(Font::new(&serif, fs, false));
            it.coords = vec![bx + dx, ly + dx];
        }
        let it = cv.get(sub_id).unwrap();
        it.text = sub;
        it.hidden = false;
        it.font = Some(Font::new(&fu, ss, false));
        it.coords = vec![bx, ly + fs as f64 * 1.5];
        for (id, dx) in [(cnt_s, 2.0), (cnt, 0.0)] {
            let it = cv.get(id).unwrap();
            it.text = count.clone();
            it.hidden = false;
            it.font = Some(Font::new(&serif, cs, false));
            it.coords = vec![bx + dx, by + dx];
        }
        let it = cv.get(head_id).unwrap();
        it.text = head;
        it.hidden = false;
        it.font = Some(Font::new(&fu, hs, false));
        it.coords = vec![12.0, 10.0];
        cv.tag_raise("breath");
    }
}

/// Zenko's sprite in the garden and the grass in front of her paws
pub struct ZkSprite {
    pub photo: Rc<Photo>,
    pub x: i64,
    pub y: i64,
    pub front: Option<Rc<Photo>>,
    pub fx: i64,
    pub fy: i64,
}

fn ui_hand_mark(_i: usize) {}

// ---- fishing cards (Zenko22 fish_card / fish_ask / fish_answer / fish_show_catch / z22_after_buttons) ---------------

/// fish_book_L(fid): how long the fish is drawn in the koi book and on its catch card
pub fn fish_book_l(fid: &str) -> i64 {
    match fish_by_id(fid).map(|f| &f[4]).and_then(|v| v.as_str()).unwrap_or("") {
        "medaka" => 52,
        "gold" => 76,
        "oar" => 150,
        "catfish" => 104,
        _ => 96,
    }
}

/// ("%.1f" % cm).rstrip("0").rstrip(".")
pub fn cm_txt(cm: f64) -> String {
    let s = format!("{cm:.1}");
    s.trim_end_matches('0').trim_end_matches('.').to_string()
}

impl GardenPage {
    fn title_face(&self) -> String {
        if self.f.title.is_empty() { self.f.ui.clone() } else { self.f.title.clone() }
    }

    /// fish_card_close()
    pub fn fish_card_close(&mut self, ui: &mut Ui) {
        if let Some(o) = self.fcard.take() {
            ui.destroy(o);
        }
        self.fcard_kind = 0;
    }

    /// fish_clear(): the line, bobber, ripples and the jumping fish go, and the card closes
    pub fn fish_clear(&mut self, ui: &mut Ui) {
        if let Some(fish) = self.fish.take() {
            let cv = canvas_of(ui, self.cv);
            for id in fish.items.iter().flatten() {
                cv.delete(&id.to_string());
            }
            for rp in &fish.ripples {
                cv.delete(&rp.0.to_string());
            }
            if let Some(j) = fish.jump {
                cv.delete(&j.2.to_string());
            }
        }
        self.fish_card_close(ui);
    }

    /// fish_card(build): a gold-edged card (padx/pady 2) with the card's colour inside (padx 18, pady 14), as a canvas
    /// window at (W/2, H*0.46), anchor center. Returns (outer, inside).
    fn lacquer_card(&mut self, ui: &mut Ui, kind: u8) -> (usize, usize) {
        self.fish_card_close(ui);
        let th = self.th.clone();
        let outer = ui.frame(self.cv, &th.gold);
        (ui.ws[outer].padx, ui.ws[outer].pady) = (2, 2);
        let f = ui.frame(outer, &th.card);
        (ui.ws[f].padx, ui.ws[f].pady) = (18, 14);
        ui.pack(f, Pack::default().fill(Fill::Both).expand());
        let (w, h) = self.cv_size(ui);
        // (tkCanvWind.c: the item's point rounded half up, then x -= width / 2, y -= height / 2)
        let (px, py) = ((w as f64 / 2.0 + 0.5).floor() as i32, (h as f64 * 0.46 + 0.5).floor() as i32);
        ui.ws[outer].place = Some(Place { x: px, y: py, center: true, ..Default::default() });
        self.fcard = Some(outer);
        self.fcard_kind = kind;
        (outer, f)
    }

    /// fish_ask(): the question card. `sub` is the subject's name (or "Your flashcards").
    pub fn fish_ask_card(&mut self, ui: &mut Ui, q: &Value, sub: &str) {
        let (th, fu, tf) = (self.th.clone(), self.f.ui.clone(), self.title_face());
        let ff = Faces { ui: fu.clone(), title: self.f.title.clone() };
        let (_, f) = self.lacquer_card(ui, 1);
        let bg = th.card.clone();
        let top = ui.frame(f, &bg);
        ui.pack(top, Pack::default().fill(Fill::X));
        let l = ui.label(top, "Bite! \u{639b}\u{304b}\u{3063}\u{305f}", &bg, &th.gold, Font::new(&tf, 13, true));
        ui.pack(l, Pack::default().side(Side::Left));
        let diff = q["diff"].as_i64().unwrap_or(1).clamp(0, 3) as usize;
        let stars = "\u{2605}".repeat(diff) + &"\u{2606}".repeat(3 - diff);
        let l = ui.label(top, &format!("{sub}  {stars}"), &bg, &th.grey, Font::new(&fu, 9, false));
        ui.pack(l, Pack::default().side(Side::Right));
        let serif = self.d.serif.clone();
        let l = ui.label(f, "Answer right to reel it in:", &bg, &th.grey, italic(&serif, 10));
        ui.set_anchor(l, Anchor::W);
        ui.pack(l, Pack::default().fill(Fill::X).pady(6, 2));
        let l = label_w(ui, f, q["q"].as_str().unwrap_or(""), &bg, &th.ink, &fu, 12, true);
        justify_left(ui, l, 380);
        ui.pack(l, Pack::default().fill(Fill::X).pady(0, 10));
        for (i, o) in q["opts"].as_array().cloned().unwrap_or_default().iter().enumerate() {
            let b = tt_btn(ui, f, &format!("{}   {}", i + 1, crate::extra::clip(o.as_str().unwrap_or(""), 60)),
                           &format!("gd_ans:{i}"), None, None, 10, 12, 3, &th, &ff);
            ui.pack(b, Pack::default().fill(Fill::X).pady(2, 2));
        }
        let l = ui.label(f, "Keys 1-4 work too  \u{b7}  Esc lets it go", &bg, &th.grey, Font::new(&fu, 8, false));
        ui.pack(l, Pack::default().pady(8, 0));
    }

    /// z22_after_buttons(f, bg): Cast again (only when a cast is left), Koi book, Done
    fn after_buttons(&mut self, ui: &mut Ui, f: usize, can: bool) {
        let th = self.th.clone();
        let ff = Faces { ui: self.f.ui.clone(), title: self.f.title.clone() };
        let r = ui.frame(f, &th.card);
        ui.pack(r, Pack::default().pady(12, 0));
        let b = ui.button(r, "Cast again", "gd_again", if can { &th.acc } else { &th.field }, if can { &th.onacc } else { &th.grey },
                          &ff.ui, 10, 12, 2, can);
        ui.pack(b, Pack::default().side(Side::Left).padx(4, 4));
        let b = tt_btn(ui, r, "Koi book", "gd_book2", None, None, 10, 12, 2, &th, &ff);
        ui.pack(b, Pack::default().side(Side::Left).padx(4, 4));
        let b = tt_btn(ui, r, "Done", "gd_done", None, None, 10, 12, 2, &th, &ff);
        ui.pack(b, Pack::default().side(Side::Left).padx(4, 4));
    }

    /// fish_answer's escape card: the question, its answer and (when it was saved) the flashcard note
    pub fn fish_lost_card(&mut self, ui: &mut Ui, q: &Value, saved: bool, can: bool) {
        let (th, fu, tf) = (self.th.clone(), self.f.ui.clone(), self.title_face());
        let (_, f) = self.lacquer_card(ui, 2);
        let bg = th.card.clone();
        let l = ui.label(f, "It got away\u{2026} \u{9003}\u{3052}\u{305f}", &bg, &th.gold, Font::new(&tf, 13, true));
        ui.pack(l, Pack::default());
        let l = ui.label(f, q["q"].as_str().unwrap_or(""), &bg, &th.grey, Font::new(&fu, 9, false));
        if let Kind::Label { wrap, .. } = &mut ui.ws[l].kind {
            *wrap = 360;
        }
        ui.pack(l, Pack::default().pady(8, 2));
        let l = ui.label(f, &format!("Answer: {}", q["a"].as_str().unwrap_or("")), &bg, &th.ink, Font::new(&fu, 11, true));
        if let Kind::Label { wrap, .. } = &mut ui.ws[l].kind {
            *wrap = 360;
        }
        ui.pack(l, Pack::default().pady(0, 4));
        let l = ui.label(f, if saved { "I saved it to your flashcards so it comes back~" } else { "" }, &bg, &th.grey,
                         Font::new(&fu, 9, false));
        ui.pack(l, Pack::default());
        self.after_buttons(ui, f, can);
    }

    /// fish_show_catch(fid, seed, cm, new): the fish's picture, names, rarity and size, its note and the buttons
    pub fn fish_catch_card(&mut self, ui: &mut Ui, fid: &str, seed: i64, cm: f64, new: bool, can: bool) {
        let (th, fu) = (self.th.clone(), self.f.ui.clone());
        let fi = fish_by_id(fid).cloned().unwrap_or(Value::Null);
        let g = gdata();
        let l_ = fish_book_l(fid);
        let img = self.fish_photo(fid, seed, l_, if l_ <= 110 { 2 } else { 1 }, 0.0, 1.0);
        let (_, f) = self.lacquer_card(ui, 2);
        let bg = th.card.clone();
        if new {
            let l = ui.label(f, "\u{2726} NEW for your koi book \u{2726}", &bg, &th.gold, Font::new(&fu, 9, true));
            ui.pack(l, Pack::default());
        }
        let il = ui.image_label(f, img, &mixhex(&bg, "#0e1a3a", 0.5), 2);
        (ui.ws[il].padx, ui.ws[il].pady) = (14, 10);
        ui.pack(il, Pack::default().pady(6, 8));
        let s = |v: &Value| v.as_str().unwrap_or("").to_string();
        let l = ui.label(f, &s(&fi[1]), &bg, &mixhex(&th.gold, &th.ink, 0.25), Font::new(&self.d.jp, 20, false));
        ui.pack(l, Pack::default());
        let l = ui.label(f, &s(&fi[2]), &bg, &th.ink, Font::new(&self.d.serif, 13, false));
        ui.pack(l, Pack::default());
        let r = fi[3].as_i64().unwrap_or(1) as usize;
        let l = ui.label(f, &format!("{}  \u{b7}  {} cm", s(&g["FISH_RARITY"][r]), cm_txt(cm)), &bg, &s(&g["FISH_RCOL"][r]),
                         Font::new(&fu, 10, true));
        ui.pack(l, Pack::default().pady(2, 4));
        let l = ui.label(f, &s(&fi[8]), &bg, &th.grey, Font::new(&fu, 9, false));
        if let Kind::Label { wrap, .. } = &mut ui.ws[l].kind {
            *wrap = 340;
        }
        ui.pack(l, Pack::default());
        self.after_buttons(ui, f, can);
    }

    /// fish_answer's leap: the caught fish jumps out of the water at the bobber
    pub fn fish_jump(&mut self, ui: &mut Ui, fid: &str, seed: i64, t: f64, dx: f64) {
        let Some(lay) = self.lay.clone() else { return };
        let l = 12.max((lay.pond.2 * 0.5) as i64);
        let img = self.fish_photo(fid, seed, l, self.z, 0.0, 1.0);
        let cv = canvas_of(ui, self.cv);
        cv.image(0.0, -100.0, img).tags(&["fx"]);
        let id = cv.last_id();
        if let Some(f) = self.fish.as_mut() {
            f.jump = Some((t, dx, id));
        }
        let z = self.z as f64;
        let Some(f) = self.fish.as_ref() else { return };
        let (x, y) = ((lay.pond.0 + f.bx * lay.pond.2) * z, (lay.pond.1 + f.by * lay.pond.3) * z);
        self.ripple(ui, x, y, false, 0.0, t);
    }
}
