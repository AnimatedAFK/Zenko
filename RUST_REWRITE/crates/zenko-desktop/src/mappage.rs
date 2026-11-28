//! The hub's World map page (zenko.py Zenko22 hub_build_map / z22_region_bar / z22_flow / z22_region / map_draw /
//! map_overlay / map_hover / z22_stamp / map_panel, with the base map_place_ring / map_pick / map_select / map_anim):
//! the region buttons flowed into rows, the painting of the region with its torii or pins, the road of places
//! reached, labels, Zenko standing at the last stop, drifting clouds and the hover tip; the side panel with the
//! place's ema card (story, did-you-know, rewards), its gold-seal quiz, the region's passport and the journey list.

use crate::aa::{Sh, Shape, aa_pixels};
use crate::canvasdraw::text_bbox;
use crate::gardenpage::zoom_px;
use crate::hubside::sprite_photo;
use crate::hubwin::{canvas_of, hub_title_draw};
use crate::mapart::{Proj, mdata, region_scene};
use crate::pagekit::{Faces, Scroller, WrapState, glow_bar, hub_card, hub_scroller, label_w, tt_btn, zk_fonts, zk_wrap};
use crate::theme::Theme;
use crate::tkcanvas::Photo;
use crate::ui::{Anchor, Fill, Font, Fonts, Grid, Kind, Pack, Place, Side, Ui};
use serde_json::Value;
use std::rc::Rc;
use zenko_art::palette::{Palette, mixhex};
use zenko_sound::mt::PyRandom;

pub const TITLE: &str = "World map";
pub const SUB: &str = "\u{5730}\u{56f3} \u{b7} Zenko's journey: Japan, then Kyoto's seasons, Korea, China, the Silk Road \
and the old legends. Focus hours carry her on; tap a place for its story.";

/// what the page shows
#[derive(Clone)]
pub struct MapData {
    /// journey_hours(): focus hours plus lock-in minutes
    pub hours: f64,
    /// journey()["quiz"]: places whose gold seal you've earned
    pub quiz: Vec<String>,
    pub pal: Palette,
    pub tails: i32,
    pub hd: bool,
    /// _mincho_font or _jp_font, and _serif_font
    pub jp: String,
    pub serif: String,
    /// z22_season_now() for Kyoto
    pub season: String,
}

fn s(v: &Value) -> String {
    v.as_str().unwrap_or("").to_string()
}

fn arr(v: &Value) -> &Vec<Value> {
    static E: Vec<Value> = Vec::new();
    v.as_array().unwrap_or(&E)
}

pub fn all() -> &'static Vec<Value> {
    arr(&mdata()["JOURNEY_ALL"])
}

pub fn by_id(pid: &str) -> Option<&'static Value> {
    all().iter().find(|p| p[0] == pid)
}

pub fn region_of(pid: &str) -> String {
    mdata()["JOURNEY_REGION_OF"][pid].as_str().unwrap_or("").to_string()
}

fn hrs_of(p: &Value) -> f64 {
    p[6].as_f64().unwrap_or(0.0)
}

/// z22_region_places(region)
pub fn region_places(region: &str) -> Vec<&'static Value> {
    all().iter().filter(|p| region_of(p[0].as_str().unwrap_or("")) == region).collect()
}

impl MapData {
    /// journey_state(): (hours, places reached, the next place)
    pub fn state(&self) -> (f64, Vec<&'static Value>, Option<&'static Value>) {
        let h = self.hours;
        (h, all().iter().filter(|p| h >= hrs_of(p)).collect(), all().iter().find(|p| h < hrs_of(p)))
    }
    /// the place selected when the page opens: the last one reached
    pub fn first_sel(&self) -> String {
        self.state().1.last().map(|p| s(&p[0])).unwrap_or_else(|| "inari".into())
    }
}

fn italic(fam: &str, n: i32) -> Font {
    Font { family: fam.into(), size: n, bold: false, italic: true }
}

fn justify_left(ui: &mut Ui, l: usize, wl: i32) {
    if let Kind::Label { left, wrap, .. } = &mut ui.ws[l].kind {
        *left = true;
        *wrap = wl;
    }
}

fn wrap(ui: &mut Ui, l: usize, wl: i32) {
    if let Kind::Label { wrap, .. } = &mut ui.ws[l].kind {
        *wrap = wl;
    }
}

fn anc(a: &str) -> &'static str {
    match a {
        "e" => "e",
        "s" => "s",
        "n" => "n",
        "nw" => "nw",
        "ne" => "ne",
        "sw" => "sw",
        "se" => "se",
        _ => "w",
    }
}

fn first_name(n: &str) -> String {
    n.split(" (").next().unwrap_or("").to_string()
}

pub struct MapPage {
    pub pg: usize,
    pub title: usize,
    pub body: usize,
    pub bar: usize,
    pub cv: usize,
    pub side: usize,
    pub sc: Scroller,
    pub clicks: Vec<(usize, String)>,
    pub sel: String,
    pub region: String,
    pub hover: Option<String>,
    pub side_w: i32,
    key: Option<String>,
    img: Option<Rc<Photo>>,
    pub proj: Option<Proj>,
    pub z: i64,
    tz: Option<(i64, String)>,
    icons: Option<[Rc<Photo>; 3]>,
    fox: Option<Rc<Photo>>,
    glow: Option<Rc<Photo>>,
    cloud: Option<Rc<Photo>>,
    /// pid -> (x, y) in canvas pixels
    pub items: Vec<(String, (f64, f64))>,
    ring: u32,
    ring2: u32,
    zk: Option<u32>,
    zk_at: Option<(f64, f64)>,
    tip: u32,
    tipbg: u32,
    clouds: Vec<(u32, f64)>,
    pub anim_on: bool,
    /// the region buttons (flowed by fit) and the journey bar (drawn at its width)
    btns: Vec<usize>,
    flow_w: i32,
    fresh: bool,
    jbar: Option<(usize, f64, i32)>,
    /// the quiz's result label and the shuffled answers
    pub quiz_res: Option<usize>,
    pub quiz_opts: Vec<String>,
    pub wrap: WrapState,
    fit_sig: Option<(i64, i32, i32, usize)>,
    drawn: (i64, i64),
    seig: Option<(String, Rc<Photo>)>,
    th: Theme,
    f: Faces,
    pub d: MapData,
    pub zk_f: f64,
}

impl MapPage {
    pub fn build(ui: &mut Ui, pg: usize, th: &Theme, f: &Faces, cm10: i32, d: &MapData) -> MapPage {
        let title = ui.canvas(pg, &th.banner, cm10, 96);
        canvas_of(ui, title);
        ui.pack(title, Pack::default().fill(Fill::X));
        let body = ui.frame(pg, &th.paper);
        ui.pack(body, Pack::default().fill(Fill::Both).expand().padx(20, 20).pady(10, 14));
        ui.ws[body].gw_rows = vec![0, 1];
        ui.ws[body].gw_cols = vec![1];
        let bar = ui.frame(body, &th.paper);
        ui.grid(bar, Grid { row: 0, col: 0, sticky: "ew", padx: (0, 0), pady: (0, 8), cspan: 0, rspan: 0 });
        let frame = ui.frame(body, &mixhex(&th.gold, "#000000", 0.45));
        (ui.ws[frame].padx, ui.ws[frame].pady) = (2, 2);
        ui.grid(frame, Grid { row: 1, col: 0, sticky: "nsew", padx: (0, 0), pady: (0, 0), cspan: 0, rspan: 0 });
        let cv = ui.canvas(frame, "#0e1436", 420, 300);
        canvas_of(ui, cv);
        ui.pack(cv, Pack::default().fill(Fill::Both).expand());
        let side = ui.frame(body, &th.paper);
        ui.ws[side].opt_w = 300;
        ui.ws[side].propagate = false;
        ui.grid(side, Grid { row: 0, col: 1, sticky: "ns", padx: (14, 0), pady: (0, 0), cspan: 0, rspan: 2 });
        let sc = hub_scroller(ui, side, th, cm10);
        let sel = d.first_sel();
        let region = { let r = region_of(&sel); if r.is_empty() { "japan".to_string() } else { r } };
        let mut p = MapPage {
            pg, title, body, bar, cv, side, sc, clicks: vec![], sel, region, hover: None, side_w: 300, key: None, img: None,
            proj: None, z: 2, tz: None, icons: None, fox: None, glow: None, cloud: None, items: vec![], ring: 0, ring2: 0,
            zk: None, zk_at: None, tip: 0, tipbg: 0, clouds: vec![], anim_on: false, btns: vec![], flow_w: 0, fresh: true, jbar: None,
            quiz_res: None, quiz_opts: vec![], wrap: WrapState::default(), fit_sig: None, drawn: (0, 0), seig: None, th: th.clone(),
            f: Faces { ui: f.ui.clone(), title: f.title.clone() }, d: d.clone(), zk_f: 1.0,
        };
        p.region_bar(ui);
        p.panel(ui);
        p
    }

    fn faces(&self) -> Faces {
        Faces { ui: self.f.ui.clone(), title: self.f.title.clone() }
    }

    fn click(&mut self, ids: &[usize], cmd: &str) {
        for &i in ids {
            self.clicks.push((i, cmd.to_string()));
        }
    }

    /// the command of the clickable widget under a point, if any (the region buttons carry their own)
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

    /// z22_region_bar(): one button per region, the open ones with their count, the shut ones with the hours
    pub fn region_bar(&mut self, ui: &mut Ui) {
        for k in ui.ws[self.bar].kids.clone() {
            ui.destroy(k);
        }
        let (th, f) = (self.th.clone(), self.faces());
        let (_, got, nxt) = self.d.state();
        let ids: Vec<String> = got.iter().map(|p| s(&p[0])).collect();
        self.btns.clear();
        for r in arr(&mdata()["JOURNEY_REGIONS"]) {
            let key = s(&r[0]);
            let pl = region_places(&key);
            let n = pl.iter().filter(|p| ids.contains(&s(&p[0]))).count();
            let open = n > 0 || nxt.is_some_and(|q| region_of(q[0].as_str().unwrap_or("")) == key);
            let on = self.region == key;
            let tail = if n > 0 {
                format!("  {}/{}", n, pl.len())
            } else if open {
                String::new()
            } else {
                format!("  \u{1f512} {}h", pl[0][6].as_i64().unwrap_or(0))
            };
            let lab = format!("{} {}{}", s(&r[1]), s(&r[2]), tail);
            let b = tt_btn(ui, self.bar, &lab, &format!("mp_region:{key}"), Some(if on { &th.acc } else { &th.field }),
                           Some(if on { &th.onacc } else if open { &th.ink } else { &th.grey }), 9, 9, 1, &th, &f);
            self.btns.push(b);
        }
        self.fresh = true;
    }

    /// z22_flow(bar, btns): the buttons in as many rows as the bar's width needs, at the sizes they have now
    fn flow(&mut self, ui: &mut Ui) -> bool {
        let bw = ui.ws[self.bar].w;
        let w = if bw > 60 { bw } else { 520 };
        let (mut x, mut y, mut rowh) = (0, 0, 0);
        for &b in &self.btns {
            let (rw, rh) = (ui.ws[b].rw, ui.ws[b].rh);
            if x > 0 && x + rw > w {
                (x, y) = (0, y + rowh + 5);
                rowh = 0;
            }
            ui.ws[b].place = Some(Place { x, y, ..Default::default() });
            x += rw + 5;
            rowh = rowh.max(rh);
        }
        let h = y + rowh + 2;
        let changed = ui.ws[self.bar].opt_h != h;
        ui.ws[self.bar].opt_h = h;
        changed
    }

    /// z22_region(key): another region's painting, its last place reached (or its first) selected
    pub fn set_region(&mut self, ui: &mut Ui, key: &str) -> bool {
        if self.region == key {
            return false;
        }
        self.region = key.into();
        self.key = None;
        let h = self.d.hours;
        let pl = region_places(key);
        let here: Vec<&&Value> = pl.iter().filter(|p| h >= hrs_of(p)).collect();
        self.sel = s(&here.last().map(|p| **p).unwrap_or(pl[0])[0]);
        self.region_bar(ui);
        self.panel(ui);
        true
    }

    // ---- layout ------------------------------------------------------------------------------------------------

    /// zk_fit(): Zenko20 skips the whole fit while (factor, window size, text scale, widget count) is unchanged,
    /// so a panel made again with as many widgets keeps its unwrapped labels until the next real fit
    pub fn fit(&mut self, ui: &mut Ui, fonts: &dyn Fonts, th: &Theme, f: &Faces, zk_f: f64, pw: i32, ph: i32) {
        let sig = ((zk_f * 1000.0).round_ties_even() as i64, pw, ph, crate::pagekit::walk(ui, self.pg).len());
        let skip = self.fit_sig == Some(sig);
        self.relayout(ui, fonts, th, f, zk_f, pw, ph);
        if !skip {
            zk_wrap(ui, fonts, self.pg, &mut self.wrap, pw, ph);
            self.fit_sig = Some(sig);
            if self.sc.sync(ui) {
                ui.layout_at(fonts, pw, ph);
            }
            self.bars(ui);
        }
        self.draw(ui, fonts);
    }

    /// the layout without the fit: fonts of new widgets scaled (as their Map event does), the title, the region
    /// buttons flowed, the side panel's width (map_draw), the scroller and the journey bar
    #[allow(clippy::too_many_arguments)]
    pub fn relayout(&mut self, ui: &mut Ui, fonts: &dyn Fonts, th: &Theme, f: &Faces, zk_f: f64, pw: i32, ph: i32) {
        self.zk_f = zk_f;
        if self.fresh {
            // z22_region_bar flows its new buttons at once (update_idletasks), before their Map event scales them
            ui.layout_at(fonts, pw, ph);
            self.flow(ui);
            self.fresh = false;
        }
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
        let sw = (260.0 + 60.0 * ((zk_f - 0.85) / 0.6).clamp(0.0, 1.0)) as i32;
        if (ui.ws[self.side].opt_w - sw).abs() > 4 {
            ui.ws[self.side].opt_w = sw;
            self.side_w = sw;
            self.panel(ui);
            zk_fonts(ui, fonts, self.pg, zk_f);
            ui.layout_at(fonts, pw, ph);
        }
        // the bar's <Configure>: flowed again when its width moved more than 4 px from the last
        let bw = ui.ws[self.bar].w;
        if (bw - self.flow_w).abs() > 4 {
            self.flow_w = bw;
            self.flow(ui);
        }
        ui.layout_at(fonts, pw, ph);
        if self.sc.sync(ui) {
            ui.layout_at(fonts, pw, ph);
        }
        self.bars(ui);
    }

    fn bars(&mut self, ui: &mut Ui) {
        if let Some((bar, g, w0)) = self.jbar {
            let bw = ui.ws[bar].w;
            if bw != w0 {
                let th = self.th.clone();
                let cv = canvas_of(ui, bar);
                cv.delete("all");
                glow_bar(cv, 1.0, 1.0, (bw - 2) as f64, 10.0, g, &th);
                self.jbar = Some((bar, g, bw));
            }
        }
    }

    /// ui["key"] = None: the painting is made again at the next draw
    pub fn forget(&mut self) {
        self.key = None;
    }

    fn cv_size(&self, ui: &Ui) -> (i64, i64) {
        (ui.ws[self.cv].w as i64, ui.ws[self.cv].h as i64)
    }

    /// map_draw(): the region painted again when the size, region or (in Kyoto) the season changed; the overlay
    /// is always made afresh
    pub fn draw(&mut self, ui: &mut Ui, fonts: &dyn Fonts) {
        let (w, h) = self.cv_size(ui);
        if w < 120 || h < 120 {
            return;
        }
        let z = 2.max((w as f64 / 400.0).max(h as f64 / 330.0).round_ties_even() as i64);
        let (rw, rh) = ((w as f64 / z as f64).ceil() as i64, (h as f64 / z as f64).ceil() as i64);
        let season = if self.region == "kyoto" { self.d.season.clone() } else { String::new() };
        let key = format!("{rw}|{rh}|{z}|{}|{season}", self.region);
        if self.key.as_deref() == Some(key.as_str()) && self.drawn == (w, h) {
            return;                                     // (no Configure: the canvas kept its size)
        }
        self.drawn = (w, h);
        if self.key.as_deref() != Some(key.as_str()) {
            let (r, p) = region_scene(&self.region, rw, rh, &season);
            self.img = Some(Rc::new(zoom_px(r.w as usize, r.h as usize, &r.rgba(), z as usize)));
            self.proj = Some(p);
            self.z = z;
            self.key = Some(key);
        }
        self.overlay(ui, fonts);
        self.anim_on = true;
    }

    /// mini_fox(zoom): Zenko at half size, 2x2 blocks keeping her dark outline
    fn mini_fox(&self, zoom: usize) -> Option<Rc<Photo>> {
        let layers = [zenko_art::core::build_tails(self.d.tails, 0, 8, false),
                      zenko_art::sit::build_frame("happy", 0, Some((0, 0, 0, 0)), &[], 0, 0, &zenko_art::sit::Look::default())];
        let (sh, sw) = (layers[1].h as usize, layers[1].w as usize);
        let mut px: Vec<Option<[u8; 3]>> = vec![None; sh * sw];
        for l in &layers {
            for y in 0..(l.h as usize).min(sh) {
                for x in 0..(l.w as usize).min(sw) {
                    let ch = l.at(x as _, y as _);
                    if ch != b'.'
                        && let Some(c) = self.d.pal.get(ch)
                    {
                        px[y * sw + x] = Some([c[0], c[1], c[2]]);
                    }
                }
            }
        }
        let at = |x: usize, y: usize| px[y * sw + x];
        let ys: Vec<usize> = (0..sh).filter(|&y| (0..sw).any(|x| at(x, y).is_some())).collect();
        let xs: Vec<usize> = (0..sw).filter(|&x| (0..sh).any(|y| at(x, y).is_some())).collect();
        if ys.is_empty() {
            return None;
        }
        let (y0, y1, x0, x1) = (ys[0], ys[ys.len() - 1] + 1, xs[0], xs[xs.len() - 1] + 1);
        let (ow, oh) = ((x1 - x0).div_ceil(2), (y1 - y0).div_ceil(2));
        let mut out = Vec::with_capacity(ow * oh);
        for y in (y0..y1).step_by(2) {
            for x in (x0..x1).step_by(2) {
                let mut blk = vec![];
                for yy in [y, y + 1] {
                    for xx in [x, x + 1] {
                        if yy < sh && xx < sw && let Some(c) = at(xx, yy) {
                            blk.push(c);
                        }
                    }
                }
                if blk.len() < 2 {
                    out.push([0, 0, 0, 0]);
                    continue;
                }
                let dark: Vec<&[u8; 3]> = blk.iter().filter(|c| 0.3 * (c[0] as f64) + 0.59 * (c[1] as f64) + 0.11 * (c[2] as f64) < 70.0)
                    .collect();
                let c = if dark.len() >= 2 {
                    *dark[0]
                } else {
                    let n = blk.len() as u32;
                    let m = |i: usize| (blk.iter().map(|c| c[i] as u32).sum::<u32>() / n) as u8;
                    [m(0), m(1), m(2)]
                };
                out.push([c[0], c[1], c[2], 255]);
            }
        }
        Some(Rc::new(zoom_px(ow, oh, &out, zoom.max(1))))
    }

    /// map_overlay(): the title plaque, the road, the pins and names, Zenko, the ring, the hover tip and clouds
    fn overlay(&mut self, ui: &mut Ui, fonts: &dyn Fonts) {
        let th = self.th.clone();
        let (w, h) = self.cv_size(ui);
        let (wf, hf) = (w as f64, h as f64);
        let p = self.proj.clone().unwrap();
        let z = self.z;
        let zf = z as f64;
        let region = self.region.clone();
        let (hrs, got, nxt) = self.d.state();
        let gotids: Vec<String> = got.iter().map(|q| s(&q[0])).collect();
        let places = region_places(&region);
        let here: Vec<&Value> = places.iter().copied().filter(|q| gotids.contains(&s(&q[0]))).collect();
        let jp = self.d.jp.clone();
        let serif = self.d.serif.clone();
        let fu = self.f.ui.clone();
        let reg = arr(&mdata()["JOURNEY_REGIONS"]).iter().find(|r| r[0] == region.as_str()).unwrap().clone();
        if self.tz.as_ref() != Some(&(z, region.clone())) {
            self.tz = Some((z, region.clone()));
            let zz = 2.max(z) as usize;
            let icon_v = if region == "japan" || region == "kyoto" { &mdata()["JMAP_ICON_TORII"] } else { &mdata()["Z22_PIN"] };
            let icon: Vec<String> = arr(icon_v).iter().map(s).collect();
            let rows: Vec<&str> = icon.iter().map(|r| r.as_str()).collect();
            let mk = |a: &str, b: &str| {
                let mut pal = Palette::empty();
                pal.set(b'#', a);
                pal.set(b'+', b);
                Rc::new(sprite_photo(&rows, &pal, zz, self.d.hd))
            };
            self.icons = Some([mk("#e0442e", "#fff1d0"), mk("#6d7290", "#9aa0c0"), mk("#e8c06a", "#fff6dc")]);
            self.fox = self.mini_fox(1.max(z / 2) as usize);
            let c = |r: f64, col: &str, a: f64| -> Sh { (Shape::Circle(22.0, 22.0, r), Some((col.into(), a))) };
            self.glow = Some(Rc::new(aa_pixels(44, 44, &[c(21.0, "#ffcf7a", 0.10), c(13.0, "#ffcf7a", 0.18), c(6.0, "#ffe7b0", 0.35)],
                                               None)));
        }
        if self.cloud.is_none() {
            let c = |x: f64, y: f64, r: f64, a: f64| -> Sh { (Shape::Circle(x, y, r), Some(("#ffffff".into(), a))) };
            self.cloud = Some(Rc::new(aa_pixels(160, 50, &[c(40.0, 30.0, 20.0, 0.07), c(70.0, 22.0, 24.0, 0.08),
                                                          c(104.0, 28.0, 20.0, 0.07), c(130.0, 32.0, 14.0, 0.06)], None)));
        }
        let [t_on, t_off, t_next] = self.icons.clone().unwrap();
        let glow = self.glow.clone().unwrap();
        let fox = self.fox.clone();
        let cloud = self.cloud.clone().unwrap();
        let img = self.img.clone().unwrap();
        let cv = canvas_of(ui, self.cv);
        cv.delete("all");
        cv.image(0.0, 0.0, img).anchor("nw").tags(&["scene"]);
        let (tx, ty) = (14.0, 12.0);
        cv.text(tx + 12.0, ty + 10.0, &format!("{} \u{b7} {}", s(&reg[1]), s(&reg[2]))).fill(&th.gold).anchor("nw")
            .font(Font::new(&jp, 11.max(h as i32 / 40), false));
        let t1 = cv.last_id();
        let bb = text_bbox(cv.get(t1).unwrap(), fonts);
        cv.text(tx + 12.0, (bb[3] + 2) as f64, &format!("{:.1} focus hours \u{b7} {} of {} places here", hrs, here.len(), places.len()))
            .fill(&mixhex(&th.ink, "#000000", 0.1)).anchor("nw").font(italic(&serif, 8.max(h as i32 / 64)));
        let t2 = cv.last_id();
        let bb2 = text_bbox(cv.get(t2).unwrap(), fonts);
        let right = bb[2].max(bb2[2]) as f64;
        cv.rect(tx, ty, right + 14.0, (bb2[3] + 10) as f64).fill("#0b1030").outline(&th.gold);
        cv.rect(tx + 3.0, ty + 3.0, right + 11.0, (bb2[3] + 7) as f64).outline(&mixhex(&th.gold, "#0b1030", 0.5));
        cv.tag_raise(&t1.to_string());
        cv.tag_raise(&t2.to_string());
        if region == "japan" {
            let (x0, y0, _, _) = p.inset();
            cv.text(((x0 + 4) * z) as f64, ((y0 + 3) * z) as f64, "\u{6c96}\u{7e04} Okinawa").anchor("nw").fill(&th.gold)
                .font(Font::new(&fu, 7.max(h as i32 / 80), false));
        }
        let pts: Vec<(f64, f64)> = here.iter().map(|q| p.xy(q[4].as_f64().unwrap(), q[5].as_f64().unwrap())).collect();
        if pts.len() >= 2 && region != "kyoto" && region != "myth" {
            let flat: Vec<f64> = pts.iter().flat_map(|&(x, y)| [x * zf, y * zf]).collect();
            let sm = region == "japan";
            let l = cv.line(flat.clone()).fill(&mixhex(&th.gold, "#000000", 0.55)).width(3.max(z + 1) as f64).cap("round")
                .join("round");
            if sm {
                l.smooth();
            }
            let l = cv.line(flat).fill(&th.gold).width(1.max(z - 1) as f64).dash(&[6, 5]).cap("round");
            if sm {
                l.smooth();
            }
        }
        if let Some(nx) = nxt
            && let Some(a) = pts.last()
            && region_of(nx[0].as_str().unwrap_or("")) == region
            && !(region == "japan" && nx[5].as_f64().unwrap_or(0.0) < 28.0)
        {
            let b = p.xy(nx[4].as_f64().unwrap(), nx[5].as_f64().unwrap());
            cv.line(vec![a.0 * zf, a.1 * zf, b.0 * zf, b.1 * zf]).fill(&mixhex(&th.grey, "#000000", 0.2)).width(1.max(z - 1) as f64)
                .dash(&[2, 6]);
        }
        self.items.clear();
        let quiz = self.d.quiz.clone();
        let show_names = w >= if region == "japan" { 600 } else { 420 };
        let fs = 8.max(h as i32 / 70);
        let nxt_id = nxt.map(|q| s(&q[0]));
        for q in &places {
            let pid = s(&q[0]);
            let (x, y) = p.xy(q[4].as_f64().unwrap(), q[5].as_f64().unwrap());
            let (x, y) = (x * zf, y * zf);
            let on = gotids.contains(&pid);
            let is_next = nxt_id.as_deref() == Some(pid.as_str());
            let im = if on { t_on.clone() } else if is_next { t_next.clone() } else { t_off.clone() };
            if on {
                cv.image(x, y, glow.clone()).tags(&["fx"]);
            }
            let ptag = format!("p_{pid}");
            cv.image(x, y, im).anchor("s").tags(&["place", &ptag]);
            if quiz.contains(&pid) {
                cv.oval(x - 3.0 * zf - 2.0, y - 9.0 * zf - 4.0, x - 3.0 * zf + 4.0, y - 9.0 * zf + 2.0).fill(&th.gold)
                    .outline("#3a2414").tags(&["fx"]);
            }
            self.items.push((pid.clone(), (x, y)));
            if show_names && (on || is_next || region == "myth") {
                let name = first_name(&s(&q[2]));
                let lab = &mdata()["JMAP_LABEL"][pid.as_str()];
                let (mut an, lx0, ly0) = if lab.is_array() {
                    (anc(lab[0].as_str().unwrap()), lab[1].as_f64().unwrap(), lab[2].as_f64().unwrap())
                } else {
                    ("w", 11.0, -6.0)
                };
                let k = 1.0f64.max(zf / 2.0);
                let (mut lx, ly) = (x + lx0 * k, y - 7.0 + ly0 * k);
                if region != "japan" && x > wf * 0.78 {
                    (an, lx) = ("e", x - 11.0 * k);
                }
                let font = Font::new(&fu, fs, true);
                for (dx, dy) in [(1.0, 1.0), (-1.0, 1.0), (1.0, -1.0), (-1.0, -1.0)] {
                    cv.text(lx + dx, ly + dy, &name).anchor(an).fill("#05081c").font(font.clone()).tags(&["lab"]);
                }
                cv.text(lx, ly, &name).anchor(an).fill(if on { &th.ink } else { &th.gold }).font(font).tags(&["lab"]);
            }
        }
        cv.oval(0.0, 0.0, 0.0, 0.0).outline(&th.gold).width(2.0).tags(&["fx"]);
        self.ring = cv.last_id();
        cv.oval(0.0, 0.0, 0.0, 0.0).outline(&mixhex(&th.gold, "#ffffff", 0.4)).width(1.0).tags(&["fx"]);
        self.ring2 = cv.last_id();
        let places0 = arr(&mdata()["JMAP_PLACES"]);
        let mut cur: Option<&Value> = Some(got.last().copied().unwrap_or(&places0[0]));
        if region_of(cur.unwrap()[0].as_str().unwrap_or("")) != region {
            cur = here.last().copied();
        }
        self.zk_at = cur.and_then(|c| self.items.iter().find(|i| i.0 == c[0].as_str().unwrap_or("")).map(|i| i.1));
        self.zk = match (&fox, cur) {
            (Some(fx), Some(_)) => {
                cv.image(-99.0, -99.0, fx.clone()).anchor("s").tags(&["fx"]);
                Some(cv.last_id())
            }
            _ => None,
        };
        cv.text(0.0, 0.0, "").anchor("sw").fill(&th.ink).font(Font::new(&fu, fs, true)).hide(true);
        self.tip = cv.last_id();
        cv.rect(0.0, 0.0, 0.0, 0.0).fill("#0b1030").outline(&th.gold).hide(true);
        self.tipbg = cv.last_id();
        cv.tag_raise(&self.tip.to_string());
        let mut rnd = PyRandom::new(5);
        self.clouds.clear();
        for _ in 0..4 {
            let x = rnd.uniform(0.0, wf);
            let y = rnd.uniform(0.1, 0.9) * hf;
            cv.image(x, y, cloud.clone()).tags(&["fx"]);
            let id = cv.last_id();
            self.clouds.push((id, rnd.uniform(0.15, 0.4)));
        }
        self.place_ring(ui);
    }

    fn at_of(&self, pid: &str) -> Option<(f64, f64)> {
        self.items.iter().find(|i| i.0 == pid).map(|i| i.1)
    }

    /// map_place_ring(): the gold ring under the selected place
    fn place_ring(&mut self, ui: &mut Ui) {
        let Some((x, y)) = self.at_of(&self.sel) else { return };
        let r = 9.0 + self.z as f64 * 3.0;
        if let Some(it) = canvas_of(ui, self.cv).get(self.ring) {
            it.coords = vec![x - r, y - r * 0.55 - 8.0, x + r, y + r * 0.55 - 8.0];
        }
    }

    /// map_anim(): one 60 ms frame (the second ring breathing, Zenko bobbing, the clouds drifting)
    pub fn anim(&mut self, ui: &mut Ui, t: f64) {
        if self.items.is_empty() {
            self.anim_on = false;
            return;
        }
        let (w, _) = self.cv_size(ui);
        let z = self.z as f64;
        let at = self.at_of(&self.sel);
        let cv = canvas_of(ui, self.cv);
        if let Some((x, y)) = at {
            let r = 9.0 + z * 3.0 + 2.0 * (t * 3.0).sin();
            if let Some(it) = cv.get(self.ring2) {
                it.coords = vec![x - r - 4.0, y - (r + 4.0) * 0.55 - 8.0, x + r + 4.0, y + (r + 4.0) * 0.55 - 8.0];
            }
        }
        if let (Some(id), Some((x, y))) = (self.zk, self.zk_at)
            && let Some(it) = cv.get(id)
        {
            it.coords = vec![x - 2.0 * z, y - 9.0 * z + (t * 2.4).sin() * z * 1.2];
        }
        for &(id, sp) in &self.clouds {
            let tag = id.to_string();
            cv.move_items(&tag, sp, 0.0);
            if let Some(it) = cv.get(id)
                && it.coords[0] > w as f64 + 90.0
            {
                cv.move_items(&tag, -(w as f64) - 180.0, 0.0);
            }
        }
    }

    /// map_pick(x, y): the nearest place within reach of a point on the painting
    pub fn pick(&self, x: f64, y: f64) -> Option<String> {
        let mut best: Option<&str> = None;
        let mut bd = 1e9;
        for (pid, (px, py)) in &self.items {
            let d = (px - x).powi(2) + (py - 8.0 - y).powi(2);
            if d < bd {
                (best, bd) = (Some(pid.as_str()), d);
            }
        }
        let lim = (14.0 + self.z as f64 * 4.0).powi(2);
        if bd <= lim { best.map(String::from) } else { None }
    }

    /// map_hover(pid): the tip over a place (its names, and the hours still to go)
    pub fn hover_at(&mut self, ui: &mut Ui, fonts: &dyn Fonts, pid: Option<&str>) {
        if self.tip == 0 {
            return;
        }
        self.hover = pid.map(String::from);
        let (tip, tipbg) = (self.tip, self.tipbg);
        let p = pid.and_then(by_id);
        let Some(p) = p else {
            let cv = canvas_of(ui, self.cv);
            for id in [tip, tipbg] {
                if let Some(it) = cv.get(id) {
                    it.hidden = true;
                }
            }
            return;
        };
        let hrs = self.d.hours;
        let Some((x, y)) = self.at_of(&s(&p[0])) else { return };
        let mut txt = format!("{}  {}", s(&p[1]), s(&p[2]));
        if hrs < hrs_of(p) {
            txt += &format!("  \u{b7}  {:.1} h to go", hrs_of(p) - hrs);
        }
        let (w, _) = self.cv_size(ui);
        let z = self.z as f64;
        let cv = canvas_of(ui, self.cv);
        let it = cv.get(tip).unwrap();
        it.text = txt;
        it.hidden = false;
        it.coords = vec![x + 12.0, y - 18.0 - z * 4.0];
        let mut bb = text_bbox(it, fonts);
        if bb[2] > w as i32 - 6 {
            cv.move_items(&tip.to_string(), -((bb[2] - w as i32 + 10) as f64) - 24.0, 0.0);
            bb = text_bbox(cv.get(tip).unwrap(), fonts);
        }
        let bg = cv.get(tipbg).unwrap();
        bg.coords = vec![(bb[0] - 6) as f64, (bb[1] - 3) as f64, (bb[2] + 6) as f64, (bb[3] + 3) as f64];
        bg.hidden = false;
        cv.tag_raise(&tipbg.to_string());
        cv.tag_raise(&tip.to_string());
    }

    /// map_select(pid)
    pub fn select(&mut self, ui: &mut Ui, pid: &str) {
        self.sel = pid.into();
        self.place_ring(ui);
        self.panel(ui);
    }

    // ---- the side panel ----------------------------------------------------------------------------------------

    /// z22_stamp(parent, p, size, state, bg): a round hanko for the passport
    fn stamp(&self, ui: &mut Ui, parent: usize, p: &Value, size: i32, state: u8, bg: &str) -> usize {
        let th = &self.th;
        let c = ui.canvas(parent, bg, size, size);
        let jp = self.d.jp.clone();
        let cv = canvas_of(ui, c);
        let sz = size as f64;
        let r = sz / 2.0 - 3.0;
        let (cx, cy) = (sz / 2.0, sz / 2.0);
        let kanji: Vec<char> = s(&p[1]).chars().collect();
        if state == 0 {
            cv.oval(cx - r, cy - r, cx + r, cy + r).outline(&th.line).dash(&[2, 3]);
            cv.text(cx, cy, &kanji[..1.min(kanji.len())].iter().collect::<String>()).fill(&mixhex(bg, &th.grey, 0.5))
                .font(Font::new(&jp, (sz * 0.36) as i32, false));
        } else {
            let red = "#c8372d";
            if state == 2 {
                cv.oval(cx - r - 2.0, cy - r - 2.0, cx + r + 2.0, cy + r + 2.0).outline(&th.gold).width(2.0);
            }
            cv.oval(cx - r, cy - r, cx + r, cy + r).outline(red).width(2.0);
            cv.oval(cx - r + 4.0, cy - r + 4.0, cx + r - 4.0, cy + r - 4.0).outline(&mixhex(red, bg, 0.4)).width(1.0);
            let two: String = kanji.iter().take(2).collect();
            let k = if two.chars().count() > 1 { 0.3 } else { 0.38 };
            cv.text(cx, cy - 1.0, &two).fill(red).font(Font::new(&jp, (sz * k) as i32, true));
        }
        c
    }

    /// map_panel()
    pub fn panel(&mut self, ui: &mut Ui) {
        let p = self.sc.inner;
        for k in ui.ws[p].kids.clone() {
            ui.destroy(k);
        }
        self.clicks.retain(|_| false);
        self.jbar = None;
        self.quiz_res = None;
        self.quiz_opts.clear();
        let (th, f) = (self.th.clone(), self.faces());
        let bg = th.card.clone();
        let serif = self.d.serif.clone();
        let jp = self.d.jp.clone();
        let fu = f.ui.clone();
        let (hrs, got, nxt) = self.d.state();
        let gotids: Vec<String> = got.iter().map(|q| s(&q[0])).collect();
        let quiz = self.d.quiz.clone();
        let places0 = arr(&mdata()["JMAP_PLACES"]);
        let pl: &Value = by_id(&self.sel).unwrap_or(&places0[0]);
        let pid = s(&pl[0]);
        let on = hrs >= hrs_of(pl);
        let wl = 160.max(ui.ws[self.side].opt_w - 64);
        let c = hub_card(ui, p, None, "ema", &th, &f);
        ui.pack(c, Pack::default().fill(Fill::X));
        let kan = s(&pl[1]);
        let l = ui.label(c, &kan, &bg, &mixhex(&th.gold, &th.ink, 0.25), Font::new(&jp, if kan.chars().count() <= 4 { 24 } else { 18 }, false));
        ui.pack(l, Pack::default().pady(16, 0));
        let l = ui.label(c, &s(&pl[2]), &bg, &th.ink, Font::new(&serif, 12, false));
        wrap(ui, l, wl);
        ui.pack(l, Pack::default());
        let l = ui.label(c, &s(&pl[3]), &bg, &th.grey, italic(&serif, 9));
        ui.pack(l, Pack::default().pady(0, 6));
        let chip = if on {
            if quiz.contains(&pid) { "Visited \u{2713}  \u{b7}  gold seal \u{2713}".to_string() } else { "Visited \u{2713}".to_string() }
        } else {
            format!("Zenko arrives after {:.1} more focus hours", hrs_of(pl) - hrs)
        };
        let l = ui.label(c, &chip, &bg, if on { &th.gold } else { &th.grey }, Font::new(&fu, 9, true));
        ui.pack(l, Pack::default().pady(0, 8));
        let head = |ui: &mut Ui, t: &str| {
            let l = label_w(ui, c, t, &bg, &th.gold, &fu, 9, true);
            ui.pack(l, Pack::default().fill(Fill::X).padx(16, 16));
        };
        let body = |ui: &mut Ui, t: &str, fg: &str, font: Font, pady: (i32, i32)| {
            let l = ui.label(c, t, &bg, fg, font);
            ui.set_anchor(l, Anchor::W);
            justify_left(ui, l, wl);
            ui.pack(l, Pack::default().fill(Fill::X).padx(16, 16).pady(pady.0, pady.1));
        };
        head(ui, "\u{7269}\u{8a9e}  Story");
        body(ui, &s(&pl[7]), &th.ink, Font::new(&fu, 10, false), (2, 10));
        head(ui, "\u{8c46}\u{77e5}\u{8b58}  Did you know?");
        body(ui, &if on { s(&pl[8]) } else { "Reach it to read the rest of the story~".into() }, if on { &th.ink } else { &th.grey },
             Font::new(&fu, 9, false), (2, 10));
        body(ui, &format!("\u{201c}{}\u{201d}", if on { s(&pl[9]) } else { "I'll tell you all about it when we get there~".into() }),
             &th.grey, italic(&serif, 10), (0, 10));
        let rew: Vec<String> = arr(&mdata()["MAP_REWARDS"]).iter().filter(|r| r[0] == pid.as_str()).map(|r| s(&r[3])).collect();
        if !rew.is_empty() {
            body(ui, &format!("{}{}", if on { "Unlocked: " } else { "Unlocks: " }, rew.join(", ")), if on { &th.gold } else { &th.grey },
                 Font::new(&fu, 9, true), (0, 10));
        }
        let qz = &mdata()["JOURNEY_QUIZ"][pid.as_str()];
        if on && qz.is_array() && !quiz.contains(&pid) {
            let qc = hub_card(ui, p, Some("\u{554f} \u{b7} Earn the gold seal"), "", &th, &f);
            ui.pack(qc, Pack::default().fill(Fill::X).pady(12, 0));
            let l = ui.label(qc, &s(&qz[0]), &bg, &th.ink, Font::new(&fu, 10, true));
            ui.set_anchor(l, Anchor::W);
            justify_left(ui, l, wl);
            ui.pack(l, Pack::default().fill(Fill::X).padx(16, 16).pady(0, 6));
            let mut opts: Vec<String> = vec![s(&qz[1])];
            opts.extend(arr(&qz[2]).iter().map(s));
            crate::bookgen::shuffle(&mut PyRandom::new_str(&pid), &mut opts);
            let res = ui.label(qc, "", &bg, &th.grey, Font::new(&fu, 9, false));
            ui.set_anchor(res, Anchor::W);
            justify_left(ui, res, wl);
            for (i, o) in opts.iter().enumerate() {
                let b = tt_btn(ui, qc, o, &format!("mp_ans:{i}"), Some(&th.field), None, 9, 10, 2, &th, &f);
                ui.pack(b, Pack::default().fill(Fill::X).padx(14, 14).pady(2, 2));
            }
            ui.pack(res, Pack::default().fill(Fill::X).padx(16, 16).pady(4, 12));
            self.quiz_res = Some(res);
            self.quiz_opts = opts;
        }
        // the passport for this region
        let region = self.region.clone();
        let reg = arr(&mdata()["JOURNEY_REGIONS"]).iter().find(|r| r[0] == region.as_str()).unwrap().clone();
        let c3 = hub_card(ui, p, Some(&format!("\u{5370} \u{b7} Passport: {}", s(&reg[2]))), "", &th, &f);
        ui.pack(c3, Pack::default().fill(Fill::X).pady(12, 0));
        let grid = ui.frame(c3, &bg);
        ui.pack(grid, Pack::default().fill(Fill::X).padx(12, 12).pady(0, 4));
        let places = region_places(&region);
        let per = 3.max(wl / 52) as usize;
        for (i, q) in places.iter().enumerate() {
            let qid = s(&q[0]);
            let st = if !gotids.contains(&qid) { 0 } else if quiz.contains(&qid) { 2 } else { 1 };
            let sp = self.stamp(ui, grid, q, 46, st, &bg);
            ui.grid(sp, Grid { row: i / per, col: i % per, sticky: "", padx: (2, 2), pady: (2, 2), cspan: 0, rspan: 0 });
            self.click(&[sp], &format!("mp_sel:{qid}"));
        }
        let nq = places.iter().filter(|q| quiz.contains(&s(&q[0]))).count();
        let ns = places.iter().filter(|q| gotids.contains(&s(&q[0]))).count();
        let l = label_w(ui, c3, &format!("{ns} stamps \u{b7} {nq} gold seals (pass a place's quiz for its seal)"), &bg, &th.grey, &fu, 8,
                        false);
        justify_left(ui, l, wl);
        ui.pack(l, Pack::default().fill(Fill::X).padx(16, 16).pady(0, 12));
        let c2 = hub_card(ui, p, Some("\u{65c5} \u{b7} Journey"), "", &th, &f);
        ui.pack(c2, Pack::default().fill(Fill::X).pady(12, 0));
        let r = ui.frame(c2, &bg);
        ui.pack(r, Pack::default().fill(Fill::X).padx(16, 16));
        if let Some(nx) = nxt {
            let prev = got.last().map(|q| hrs_of(q)).unwrap_or(0.0);
            let frac = (hrs - prev) / 0.01f64.max(hrs_of(nx) - prev);
            let l = label_w(ui, r, &format!("Next: {} {} in {:.1} h", s(&nx[1]), first_name(&s(&nx[2])), hrs_of(nx) - hrs), &bg, &th.ink,
                            &fu, 9, false);
            justify_left(ui, l, wl);
            ui.pack(l, Pack::default().fill(Fill::X));
            let bar = ui.canvas(r, &bg, 200, 12);
            canvas_of(ui, bar);
            ui.pack(bar, Pack::default().fill(Fill::X).pady(3, 4));
            self.jbar = Some((bar, frac, -1));
        } else {
            let l = label_w(ui, r, "Every place visited, even the legends. You walked the whole world with me!", &bg, &th.gold, &fu, 9,
                            true);
            justify_left(ui, l, wl);
            ui.pack(l, Pack::default().fill(Fill::X));
        }
        let l = label_w(ui, r, "Focus sessions and lock-ins both count.", &bg, &th.grey, &fu, 8, false);
        ui.pack(l, Pack::default().fill(Fill::X).pady(0, 6));
        let lst = ui.frame(c2, &bg);
        ui.pack(lst, Pack::default().fill(Fill::X).padx(12, 12).pady(0, 12));
        for q in &places {
            let qid = s(&q[0]);
            let done = hrs >= hrs_of(q);
            let rbg = if qid == pid { mixhex(&th.acc, &bg, 0.82) } else { bg.clone() };
            let row = ui.frame(lst, &rbg);
            ui.pack(row, Pack::default().fill(Fill::X));
            let mark = if done && (region == "japan" || region == "kyoto") { "\u{26e9}" } else if done { "\u{25cf}" } else { "\u{b7}" };
            let a = ui.label(row, mark, &rbg, if done { "#e0442e" } else { &th.grey }, Font::new(&fu, 9, false));
            ui.ws[a].opt_w = 2;
            ui.pack(a, Pack::default().side(Side::Left));
            let b = label_w(ui, row, &format!("{}  {}", s(&q[1]), first_name(&s(&q[2]))), &rbg, if done { &th.ink } else { &th.grey },
                            &fu, 9, false);
            ui.pack(b, Pack::default().side(Side::Left).fill(Fill::X).expand());
            let t = if done { if quiz.contains(&qid) { "\u{2605}".to_string() } else { String::new() } } else {
                format!("{}h", q[6].as_i64().unwrap_or(0))
            };
            let cc = ui.label(row, &t, &rbg, if done { &th.gold } else { &th.grey }, Font::new(&fu, 8, false));
            ui.pack(cc, Pack::default().side(Side::Right).padx(4, 4));
            self.click(&[row, a, b, cc], &format!("mp_sel:{qid}"));
        }
    }

    /// answer(o): the gold seal for the right answer, or the gentle nudge. Returns true when it was right.
    pub fn answer(&mut self, ui: &mut Ui, i: usize) -> bool {
        let qz = &mdata()["JOURNEY_QUIZ"][self.sel.as_str()];
        if self.quiz_opts.get(i).map(|o| o.as_str()) == qz[1].as_str() {
            if !self.d.quiz.contains(&self.sel) {
                self.d.quiz.push(self.sel.clone());
            }
            return true;
        }
        if let Some(res) = self.quiz_res
            && let Kind::Label { text, fg, .. } = &mut ui.ws[res].kind
        {
            *text = "Not quite~ Read the story again and try once more.".into();
            *fg = self.th.gold.clone();
        }
        false
    }
}
