//! The Zen garden painted as pixel art (zenko.py `_Ras`, `garden_scene` = `_garden_scene_z18` + `z19_garden_season`,
//! and `_garden_tree`), bit for bit: same raster rules, same random draws in the same order, Python's int()/round().

use std::f64::consts::PI;
use zenko_art::core::hypot;
use zenko_sound::mt::PyRandom;

pub type Rgb = (u8, u8, u8);

/// `_Ras`: a tiny RGB raster
#[derive(Clone)]
pub struct Ras {
    pub w: i64,
    pub h: i64,
    pub b: Vec<u8>,
}

/// Python int() of a float
fn ti(v: f64) -> i64 {
    v as i64
}

impl Ras {
    pub fn new(w: i64, h: i64) -> Ras {
        Ras { w, h, b: vec![0; (w.max(0) * h.max(0) * 3) as usize] }
    }

    pub fn put(&mut self, x: f64, y: f64, c: Rgb, a: f64) {
        let (x, y) = (ti(x), ti(y));
        if 0 <= x && x < self.w && 0 <= y && y < self.h {
            let i = ((y * self.w + x) * 3) as usize;
            let b = &mut self.b;
            if a >= 1.0 {
                b[i] = c.0;
                b[i + 1] = c.1;
                b[i + 2] = c.2;
            } else if a > 0.0 {
                for (j, cc) in [c.0, c.1, c.2].into_iter().enumerate() {
                    let o = b[i + j] as f64;
                    b[i + j] = ti(o + (cc as f64 - o) * a) as u8;
                }
            }
        }
    }

    fn p(&mut self, x: i64, y: i64, c: Rgb) {
        self.put(x as f64, y as f64, c, 1.0)
    }

    pub fn get(&self, x: i64, y: i64) -> Rgb {
        let i = ((y * self.w + x) * 3) as usize;
        (self.b[i], self.b[i + 1], self.b[i + 2])
    }

    fn set(&mut self, x: i64, y: i64, c: Rgb) {
        let i = ((y * self.w + x) * 3) as usize;
        self.b[i] = c.0;
        self.b[i + 1] = c.1;
        self.b[i + 2] = c.2;
    }

    pub fn rect(&mut self, x0: f64, y0: f64, x1: f64, y1: f64, c: Rgb, a: f64) {
        for y in 0.max(ti(y0))..self.h.min(ti(y1)) {
            for x in 0.max(ti(x0))..self.w.min(ti(x1)) {
                self.put(x as f64, y as f64, c, a);
            }
        }
    }

    pub fn disc(&mut self, cx: f64, cy: f64, r: f64, c: Rgb, a: f64, sy: f64) {
        let r2 = r * r;
        for y in ti(cy - r * sy) - 1..ti(cy + r * sy) + 2 {
            let dy = (y as f64 + 0.5 - cy) / sy;
            for x in ti(cx - r) - 1..ti(cx + r) + 2 {
                let dx = x as f64 + 0.5 - cx;
                if dx * dx + dy * dy <= r2 {
                    self.put(x as f64, y as f64, c, a);
                }
            }
        }
    }

    pub fn poly(&mut self, pts: &[(f64, f64)], c: Rgb, a: f64) {
        let n = pts.len();
        let lo = pts.iter().map(|p| p.1).fold(f64::INFINITY, f64::min);
        let hi = pts.iter().map(|p| p.1).fold(f64::NEG_INFINITY, f64::max);
        for y in 0.max(ti(lo))..self.h.min(ti(hi) + 1) {
            let yc = y as f64 + 0.5;
            let mut xs = Vec::new();
            for i in 0..n {
                let (x0, y0) = pts[i];
                let (x1, y1) = pts[(i + 1) % n];
                if (y0 <= yc && yc < y1) || (y1 <= yc && yc < y0) {
                    xs.push(x0 + (yc - y0) * (x1 - x0) / (y1 - y0));
                }
            }
            xs.sort_by(|a, b| a.partial_cmp(b).unwrap());
            let mut k = 0;
            while k + 1 < xs.len() {
                for x in ti(xs[k].round_ties_even())..ti(xs[k + 1].round_ties_even()) {
                    self.put(x as f64, y as f64, c, a);
                }
                k += 2;
            }
        }
    }

    pub fn line(&mut self, x0: f64, y0: f64, x1: f64, y1: f64, c: Rgb, w: f64, a: f64) {
        let d = (x1 - x0).abs().max((y1 - y0).abs()).max(1.0);
        let steps = ti(d * 1.5) + 1;
        let r = w / 2.0;
        for i in 0..=steps {
            let t = i as f64 / steps as f64;
            let (x, y) = (x0 + (x1 - x0) * t, y0 + (y1 - y0) * t);
            if w <= 1.2 {
                self.put(x, y, c, a);
            } else {
                self.disc(x, y, r, c, a, 1.0);
            }
        }
    }

    /// RGBA pixels (for a Photo)
    pub fn rgba(&self) -> Vec<[u8; 4]> {
        self.b.chunks(3).map(|p| [p[0], p[1], p[2], 255]).collect()
    }
}

pub fn lerp3(a: Rgb, b: Rgb, t: f64) -> Rgb {
    let t = t.clamp(0.0, 1.0).max(0.0);
    let f = |p: u8, q: u8| ti(p as f64 + (q as f64 - p as f64) * t) as u8;
    (f(a.0, b.0), f(a.1, b.1), f(a.2, b.2))
}

pub const BAYER: [[i32; 4]; 4] = [[0, 8, 2, 10], [12, 4, 14, 6], [3, 11, 1, 9], [15, 7, 13, 5]];

fn bay(x: i64, y: i64) -> i32 {
    BAYER[(y & 3) as usize][(x & 3) as usize]
}

/// `_dith`
pub fn dith(x: i64, y: i64, a: f64) -> bool {
    a > (bay(x, y) as f64 + 0.5) / 16.0
}

/// `_shade`
pub fn shade(c: Rgb, f: f64) -> Rgb {
    let g = |v: u8| ti(v as f64 * f).clamp(0, 255) as u8;
    (g(c.0), g(c.1), g(c.2))
}

fn hex(h: &str) -> Rgb {
    let h = h.trim_start_matches('#');
    let p = |i: usize| u8::from_str_radix(&h[i..i + 2], 16).unwrap_or(0);
    (p(0), p(2), p(4))
}

/// the garden's layout (garden_scene's `lay` dict)
#[derive(Clone, Debug, Default)]
pub struct Lay {
    pub tree: (i64, i64),
    pub tips: Vec<(f64, f64, i64)>,
    pub lanterns: Vec<(f64, f64)>,
    pub pond: (f64, f64, f64, f64),
    pub horizon: i64,
    pub torii: (i64, i64),
    pub moon: (f64, f64, f64),
    pub wall: i64,
    pub basin: (f64, f64, f64),
    pub breath: (i64, f64),
}

/// Z19_LOOK: the season and sky looks (settings) with the season and part of day already worked out
#[derive(Clone, Debug)]
pub struct Look {
    pub season: bool,
    pub sky: bool,
    pub season_name: String,
    pub part: String,
    pub weather: Option<String>,
}

impl Default for Look {
    fn default() -> Look {
        Look { season: true, sky: true, season_name: "spring".into(), part: "night".into(), weather: None }
    }
}

/// `garden_scene(w, h, stage, growth, health, grove, seed)`
pub fn garden_scene(w: i64, h: i64, stage: i64, growth: f64, health: f64, grove: i64, seed: u64, look: &Look) -> (Ras, Lay) {
    let (mut r, lay) = scene_z18(w, h, stage, growth, health, grove, seed);
    season(&mut r, &lay, w, h, seed, look);
    (r, lay)
}

fn scene_z18(w: i64, h: i64, stage: i64, growth: f64, health: f64, grove: i64, seed: u64) -> (Ras, Lay) {
    let mut r = Ras::new(w, h);
    let mut rnd = PyRandom::new(seed);
    let (wf, hf) = (w as f64, h as f64);
    let hy = ti(hf * 0.50);
    let hyf = hy as f64;
    // ---- sky ----
    let stops: [(f64, Rgb); 5] = [(0.0, (11, 13, 36)), (0.40, (24, 30, 70)), (0.72, (58, 64, 116)), (0.90, (104, 104, 152)),
        (1.0, (142, 136, 176))];
    let sky_h = hy;
    for y in 0..sky_h {
        let t = y as f64 / (1.max(sky_h - 1)) as f64;
        let (mut f, mut c0, mut c1) = (0.0, stops[0].1, stops[0].1);
        for k in 0..stops.len() - 1 {
            if stops[k].0 <= t && t <= stops[k + 1].0 {
                let (a0, a1) = (stops[k].0, stops[k + 1].0);
                c0 = stops[k].1;
                c1 = stops[k + 1].1;
                f = (t - a0) / 1e-6f64.max(a1 - a0);
                break;
            }
        }
        let q = f * 5.0;
        for x in 0..w {
            let ff = if (q - ti(q) as f64) > (bay(x, y) as f64 + 0.5) / 16.0 { 1 } else { 0 };
            let col = lerp3(c0, c1, (ti(q) + ff) as f64 / 5.0);
            r.set(x, y, col);
        }
    }
    for _ in 0..ti((w * h) as f64 / 330.0) {
        let x = rnd.randbelow(w as u64) as f64;
        let y = ti(rnd.random().powf(1.8) * hyf * 0.62) as f64;
        let a = rnd.uniform(0.25, 0.85);
        r.put(x, y, (236, 236, 255), a);
        if a > 0.78 && rnd.random() < 0.35 {
            for (dx, dy) in [(1.0, 0.0), (-1.0, 0.0), (0.0, 1.0), (0.0, -1.0)] {
                r.put(x + dx, y + dy, (200, 206, 255), 0.35);
            }
        }
    }
    let (mx, my, mr) = (wf * 0.79, hf * 0.15, 5.0f64.max(hf * 0.062));
    for y in ti(my - mr * 3.4)..ti(my + mr * 3.4) {
        for x in ti(mx - mr * 3.4)..ti(mx + mr * 3.4) {
            let d = hypot(x as f64 + 0.5 - mx, y as f64 + 0.5 - my) / mr;
            if 1.0 < d && d < 3.4 && 0 <= x && x < w && 0 <= y && y < h {
                let a = (1.0 - (d - 1.0) / 2.4).powf(2.0);
                r.put(x as f64, y as f64, (150, 152, 204), a * 0.22);
            }
        }
    }
    r.disc(mx, my, mr, (238, 234, 216), 1.0, 1.0);
    r.disc(mx - mr * 0.12, my - mr * 0.12, mr * 0.86, (248, 246, 232), 1.0, 1.0);
    for (cx_, cy_, cr) in [(-0.32, -0.18, 0.2), (0.22, 0.30, 0.15), (0.34, -0.36, 0.09), (-0.05, 0.12, 0.07)] {
        r.disc(mx + cx_ * mr, my + cy_ * mr, mr * cr, (220, 216, 200), 0.8, 1.0);
    }
    for k in 0..4 {
        let cy = my + (k as f64 - 1.5) * mr * 1.1 + rnd.uniform(-1.5, 1.5);
        let x0 = rnd.uniform(wf * 0.48, wf * 0.72);
        let l = rnd.uniform(wf * 0.16, wf * 0.34);
        for x in ti(x0)..ti(wf.min(x0 + l)) {
            let t = (x as f64 - x0) / l;
            let s = (PI * t).sin();
            r.put(x as f64, cy, (150, 150, 200), 0.40 * s);
            if s > 0.45 {
                r.put(x as f64, cy + 1.0, (44, 48, 96), 0.40 * s);
            }
        }
    }
    // ---- mountains ----
    let ridge = |r: &mut Ras, base: f64, amp: f64, s: f64, col: Rgb, edge: Rgb| {
        for x in 0..w {
            let xf = x as f64;
            let top = base - amp * (0.55 * (xf * 0.019 + s).sin() + 0.30 * (xf * 0.051 + 2.1 * s).sin()
                + 0.15 * (xf * 0.13 + s * 0.7).sin());
            for y in 0.max(ti(top))..hy + 1 {
                let c = if (y as f64) < top + 1.5 { edge } else { col };
                r.p(x, y, c);
            }
        }
    };
    let mist = |r: &mut Ras, yc: f64, hh: f64, a0: f64| {
        for y in ti(yc - hh)..ti(yc + hh) + 1 {
            if !(0 <= y && y < h) {
                continue;
            }
            let a = a0 * (1.0 - ((y as f64 - yc) / 1.0f64.max(hh)).abs());
            for x in 0..w {
                let aa = a * (0.75 + 0.25 * (x as f64 * 0.045 + y as f64 * 0.6).sin());
                if dith(x, y, aa * 1.6) {
                    r.put(x as f64, y as f64, (150, 150, 196), 0.55);
                }
            }
        }
    };
    let (pk_x, pk_top) = (wf * 0.27, hyf - hf * 0.33);
    for y in ti(pk_top)..hy {
        let t = (y as f64 - pk_top) / (hyf - pk_top);
        let half = (0.02 + t * 0.9) * wf * 0.25;
        for x in ti(pk_x - half)..ti(pk_x + half) + 1 {
            let xf = x as f64;
            let mut col = lerp3((104, 104, 156), (82, 84, 136), t);
            if xf < pk_x - half + 1.2 || xf > pk_x + half - 1.2 {
                col = lerp3(col, (130, 130, 178), 0.5);
            }
            if t < 0.20 + 0.05 * (xf * 0.9).sin() {
                col = lerp3((222, 222, 240), (168, 168, 206), t * 3.5);
                if xf > pk_x + 1.0 {
                    col = shade(col, 0.86);
                }
            }
            r.p(x, y, col);
        }
    }
    mist(&mut r, hyf - hf * 0.17, hf * 0.05, 0.35);
    ridge(&mut r, hyf - hf * 0.13, hf * 0.05, 1.3, (74, 78, 128), (96, 98, 150));
    mist(&mut r, hyf - hf * 0.09, hf * 0.04, 0.30);
    ridge(&mut r, hyf - hf * 0.07, hf * 0.035, 3.1, (52, 58, 102), (70, 76, 122));
    let px_ = ti(wf * 0.90);
    let pxf = px_ as f64;
    let pb = hyf - hf * 0.085;
    for k in 0..4i64 {
        let ww = 3.max(ti(wf * (0.028 - k as f64 * 0.0045)));
        let wwf = ww as f64;
        let yb = pb - (k * h) as f64 * 0.03;
        r.rect((px_ - ww.div_euclid(2) + 1) as f64, yb - hf * 0.022, (px_ + ww.div_euclid(2)) as f64, yb, (40, 44, 78), 1.0);
        r.poly(&[(pxf - wwf * 0.8, yb - hf * 0.022), (pxf + wwf * 0.8, yb - hf * 0.022), (pxf + wwf * 0.35, yb - hf * 0.034),
            (pxf - wwf * 0.35, yb - hf * 0.034)], (34, 38, 70), 1.0);
    }
    r.rect(pxf, pb - hf * 0.17, pxf + 1.0, pb - hf * 0.12, (34, 38, 70), 1.0);

    // ---- pines and bamboo behind the wall ----
    let wall_top = ti(hyf - hf * 0.085);
    let wtf = wall_top as f64;
    let (pine_d, pine_m, pine_l): (Rgb, Rgb, Rgb) = ((22, 36, 46), (30, 50, 58), (44, 70, 74));
    let cloud_pad = |r: &mut Ras, x: f64, y: f64, rr: f64| {
        let sy = 0.45;
        r.disc(x, y + rr * 0.18, rr * 1.05, pine_d, 1.0, sy);
        r.disc(x - rr * 0.1, y, rr, pine_m, 1.0, sy);
        r.disc(x - rr * 0.35, y - rr * sy * 0.35, rr * 0.55, pine_l, 1.0, sy);
    };
    for (px0, ht) in [(0.06, 0.20), (0.40, 0.16), (0.63, 0.19)] {
        let bx = wf * px0 + rnd.uniform(-4.0, 4.0);
        let top = wtf - hf * ht;
        r.line(bx, (wall_top + 2) as f64, bx + wf * 0.01, top + hf * 0.03, (34, 28, 34), 1.5f64.max(hf * 0.012), 1.0);
        for j in 0..4i64 {
            let yy = top + (j * h) as f64 * ht * 0.24;
            let xx = bx + (if j % 2 == 1 { wf * 0.035 } else { -wf * 0.03 }) * (0.6 + j as f64 * 0.15);
            r.line(bx + wf * 0.005, yy + hf * 0.02, xx, yy, (34, 28, 34), 1.0f64.max(hf * 0.006), 1.0);
            cloud_pad(&mut r, xx, yy, 4.0f64.max(wf * (0.035 - j as f64 * 0.003)));
        }
    }
    for k in 0..9i64 {
        let bx = ti(wf * 0.75 + (k * w) as f64 * 0.024 + rnd.uniform(-2.0, 2.0));
        let top = wtf - hf * rnd.uniform(0.14, 0.26);
        let col = *rnd.choice(&[(40u8, 70u8, 62u8), (48, 82, 70), (34, 60, 56)]);
        let step = 5.max(ti(hf * 0.04));
        for y in ti(top)..wall_top + 2 {
            r.p(bx, y, col);
            r.p(bx + 1, y, shade(col, 0.8));
            if (y - ti(top)).rem_euclid(step) == 0 {
                r.p(bx, y, shade(col, 1.35));
                r.p(bx + 1, y, shade(col, 1.1));
            }
        }
        for j in 0..3i64 {
            let ly = top + (j * h) as f64 * 0.04 + rnd.uniform(0.0, 3.0);
            let sg = if (k + j) % 2 == 1 { 1 } else { -1 };
            for q in 0..5i64 {
                let xq = (bx + sg * (q + 1)) as f64;
                r.put(xq, ly + q as f64 * 0.45, (58, 96, 80), 1.0);
                r.put(xq, ly + q as f64 * 0.45 + 1.0, (36, 64, 58), 0.8);
            }
        }
    }

    // ---- the temple wall ----
    let roof_h = 3.max(ti(hf * 0.022));
    for y in wall_top..hy {
        let t = (y - wall_top) as f64 / 1.max(hy - wall_top) as f64;
        for x in 0..w {
            let mut c = lerp3((112, 112, 138), (86, 86, 112), t);
            if bay(x, y) > 13 {
                c = shade(c, 1.05);
            }
            r.p(x, y, c);
        }
    }
    for k in 0..5 {
        let yy = wall_top + roof_h + 2 + k * 2;
        if yy < hy - 2 {
            for x in 0..w {
                r.put(x as f64, yy as f64, (150, 150, 172), 0.55);
            }
        }
    }
    let post = 6.max(ti(wf * 0.11)) as usize;
    for x in (0..w).step_by(post) {
        r.rect(x as f64, (wall_top + roof_h) as f64, (x + 1) as f64, hyf, (70, 68, 92), 0.7);
    }
    r.rect(0.0, (wall_top - 1) as f64, wf, (wall_top + roof_h) as f64, (36, 38, 58), 1.0);
    for x in 0..w {
        r.p(x, wall_top - 2, (64, 68, 100));
        if x % 3 == 0 {
            r.p(x, wall_top + roof_h - 1, (24, 26, 42));
        }
    }
    for x in (0..w).step_by(3) {
        r.p(x, wall_top + 1, (52, 56, 84));
    }
    for y in hy - 3..hy {
        for x in 0..w {
            r.put(x as f64, y as f64, (40, 40, 64), 0.25 + 0.15 * (y - hy + 3) as f64);
        }
    }

    // ---- raked gravel ----
    let cx = ti(wf * 0.50);
    let cxf = cx as f64;
    let tb = ti(hf * 0.79);
    let isl = (cxf, tb as f64 + hf * 0.012, wf * 0.16, hf * 0.055);
    let rocks = [(wf * 0.29, hf * 0.66, wf * 0.050, hf * 0.030, 1.0), (wf * 0.335, hf * 0.675, wf * 0.024, hf * 0.016, 0.55),
        (wf * 0.70, hf * 0.64, wf * 0.040, hf * 0.024, 0.85), (wf * 0.115, hf * 0.935, wf * 0.030, hf * 0.020, 0.7)];
    let mut rings = vec![isl];
    rings.extend(rocks.iter().map(|q| (q.0, q.1, q.2 * 1.15, q.3 * 1.15)));
    let pond = (wf * 0.82, hf * 0.86, wf * 0.13, hf * 0.058);
    let (sand_l, sand_m, sand_d): (Rgb, Rgb, Rgb) = ((164, 168, 198), (132, 136, 170), (100, 104, 140));
    let gh = (h - hy) as f64;
    for y in hy..h {
        let yf = y as f64;
        let u = (y - hy) as f64 / gh;
        let sp = 3.0 + 4.2 * u;
        let par = u.powf(0.9) * gh / sp;
        let shade_f = 0.80 + 0.20 * u;
        for x in 0..w {
            let xf = x as f64;
            let mut v = par + 0.18 * (xf * 0.05).sin();
            let mut best: Option<f64> = None;
            for &(ex, ey, erx, ery) in &rings {
                let e = hypot((xf + 0.5 - ex) / erx, (yf + 0.5 - ey) / ery);
                if e < 1.62 {
                    let dd = (e - 1.0) * ery * 1.5 / (3.0 + 4.2 * 0.0f64.max(1.0f64.min((ey - hyf) / gh)));
                    if best.is_none_or(|b| dd < b) {
                        best = Some(dd);
                    }
                }
            }
            if let Some(b) = best {
                if b >= 0.0 {
                    v = b;
                }
            }
            let f = v - v.floor();
            let c = if f < 0.26 { sand_d } else if f < 0.40 { sand_l } else { sand_m };
            r.set(x, y, shade(c, shade_f));
        }
    }
    for x in 0..w {
        let xf = x as f64;
        let top = hf - hf * (0.035 + 0.012 * (xf * 0.07).sin() + 0.006 * (xf * 0.23).sin());
        for y in ti(top)..h {
            let c = if y as f64 > top + 1.0 { (38, 66, 56) } else { (60, 96, 74) };
            r.p(x, y, c);
            if bay(x, y) > 12 && y as f64 > top + 1.0 {
                r.p(x, y, (50, 84, 66));
            }
        }
    }

    // ---- moss ----
    let moss = |r: &mut Ras, ex: f64, ey: f64, rx: f64, ry: f64| {
        for y in ti(ey - ry) - 1..ti(ey + ry) + 2 {
            for x in ti(ex - rx) - 1..ti(ex + rx) + 2 {
                if !(0 <= x && x < w && 0 <= y && y < h) {
                    continue;
                }
                let (xf, yf) = (x as f64, y as f64);
                let e = hypot((xf + 0.5 - ex) / rx, (yf + 0.5 - ey) / ry);
                let wob = 0.035 * (xf * 0.7 + yf * 1.1).sin();
                if e < 1.0 + wob {
                    let t = (yf - (ey - ry)) / (2.0 * ry);
                    let mut c = lerp3((70, 112, 82), (34, 62, 52), t);
                    if e > 0.86 + wob {
                        c = if t < 0.5 { (52, 88, 66) } else { (30, 52, 46) };
                    } else if bay(x, y) > 11 {
                        c = shade(c, 1.18);
                    }
                    r.p(x, y, c);
                }
            }
        }
    };
    moss(&mut r, isl.0, isl.1, isl.2, isl.3);
    for &(rx_, ry_, rw_, rh_, _) in &rocks {
        moss(&mut r, rx_, ry_ + rh_ * 0.35, rw_ * 1.05, rh_ * 0.75);
    }
    // ---- rocks ----
    for &(rx_, ry_, rw_, rh_, sc) in &rocks {
        let top = ry_ - rh_ * 2.4 * sc;
        let pts = [(rx_ - rw_, ry_), (rx_ - rw_ * 0.85, ry_ - rh_ * 1.2 * sc), (rx_ - rw_ * 0.35, top),
            (rx_ + rw_ * 0.25, top + rh_ * 0.3), (rx_ + rw_ * 0.9, ry_ - rh_ * 1.0 * sc), (rx_ + rw_, ry_)];
        r.poly(&pts, (78, 80, 98), 1.0);
        r.poly(&[(rx_ + rw_ * 0.05, top + rh_ * 0.2), (rx_ + rw_ * 0.25, top + rh_ * 0.3), (rx_ + rw_ * 0.9, ry_ - rh_ * 1.0 * sc),
            (rx_ + rw_, ry_), (rx_ + rw_ * 0.2, ry_)], (56, 58, 76), 1.0);
        r.poly(&[(rx_ - rw_ * 0.85, ry_ - rh_ * 1.2 * sc), (rx_ - rw_ * 0.35, top), (rx_ - rw_ * 0.1, top + rh_ * 0.5),
            (rx_ - rw_ * 0.6, ry_ - rh_ * 0.9 * sc)], (112, 114, 134), 1.0);
        r.line(rx_ - rw_ * 0.85, ry_ - rh_ * 1.2 * sc, rx_ - rw_ * 0.35, top, (150, 152, 172), 1.0, 1.0);
        r.line(rx_ - rw_ * 0.35, top, rx_ + rw_ * 0.25, top + rh_ * 0.3, (134, 136, 158), 1.0, 1.0);
        for _ in 0..ti(rw_ * 1.2) {
            let mx2 = rx_ + rnd.uniform(-rw_ * 0.6, rw_ * 0.2);
            let my2 = top + rh_ * rnd.uniform(0.15, 0.6);
            let c = *rnd.choice(&[(72u8, 116u8, 84u8), (96, 140, 96), (58, 96, 72)]);
            r.put(mx2, my2, c, 1.0);
        }
        for k in 0..ti(rw_ * 2.0) {
            r.put(rx_ - rw_ + k as f64, ry_ + 1.0, (24, 40, 36), 0.6);
        }
    }

    // ---- stepping stones ----
    for k in 0..4i64 {
        let sx = cxf - wf * 0.10 + (k * w) as f64 * 0.032 + ((k % 2) * 2) as f64;
        let sy = hf * 0.975 - (k * h) as f64 * 0.035;
        if sy < tb as f64 + hf * 0.05 {
            break;
        }
        r.disc(sx, sy + 1.0, 3.0f64.max(wf * 0.016), (70, 72, 92), 1.0, 0.42);
        r.disc(sx, sy, 3.0f64.max(wf * 0.015), (124, 126, 146), 1.0, 0.40);
        r.disc(sx - 1.0, sy - 1.0, 2.0f64.max(wf * 0.010), (150, 152, 170), 1.0, 0.36);
    }

    // ---- the pond ----
    let (pcx, pcy, prx, pry) = pond;
    for y in ti(pcy - pry) - 2..ti(pcy + pry) + 3 {
        for x in ti(pcx - prx) - 2..ti(pcx + prx) + 3 {
            if !(0 <= x && x < w && 0 <= y && y < h) {
                continue;
            }
            let (xf, yf) = (x as f64, y as f64);
            let d = hypot((xf + 0.5 - pcx) / prx, (yf + 0.5 - pcy) / pry);
            if d <= 1.0 {
                let t = (yf - pcy + pry) / (2.0 * pry);
                let mut col = lerp3((26, 32, 64), (16, 20, 44), t);
                if d > 0.84 {
                    col = if yf < pcy { (94, 96, 116) } else { (66, 68, 88) };
                    if bay(x, y) > 12 {
                        col = shade(col, 1.2);
                    }
                } else if yf < pcy - pry * 0.55 && dith(x, y, 0.25) {
                    col = (36, 44, 82);
                }
                r.p(x, y, col);
            }
        }
    }
    for k in 0..5i64 {
        let yy = pcy - pry * 0.35 + (k * 2) as f64;
        let ww = prx * (0.16 - k as f64 * 0.025);
        r.rect(pcx + prx * 0.25 - ww, yy, pcx + prx * 0.25 + ww, yy + 1.0, (220, 220, 236), 0.55 - k as f64 * 0.08);
    }
    for (lx, ly) in [(-0.45, 0.25), (-0.15, 0.48), (0.55, 0.30)] {
        r.disc(pcx + lx * prx, pcy + ly * pry, 2.0f64.max(pry * 0.20), (46, 96, 72), 1.0, 0.55);
        r.disc(pcx + lx * prx - 0.6, pcy + ly * pry - 0.5, 1.2f64.max(pry * 0.12), (70, 128, 90), 1.0, 0.5);
    }
    r.put(pcx - 0.45 * prx, pcy + 0.25 * pry - 1.0, (238, 220, 232), 1.0);
    r.put(pcx - 0.45 * prx + 1.0, pcy + 0.25 * pry - 1.0, (214, 190, 210), 1.0);

    // ---- stone lantern ----
    let mut lanterns = Vec::new();
    let (lx, ly) = (wf * 0.19, hf * 0.80);
    let s = 1.0f64.max(hf / 190.0);
    let (st, st_l, st_d): (Rgb, Rgb, Rgb) = ((100, 102, 120), (140, 142, 160), (66, 68, 86));
    r.disc(lx, ly + 1.0, 9.0 * s, (24, 40, 36), 0.6, 0.3);
    r.rect(lx - 5.0 * s, ly - 2.0 * s, lx + 5.0 * s, ly, st_d, 1.0);
    r.rect(lx - 5.0 * s, ly - 2.0 * s, lx - 3.5 * s, ly, st, 1.0);
    r.rect(lx - 1.6 * s, ly - 12.0 * s, lx + 1.6 * s, ly - 2.0 * s, st, 1.0);
    r.rect(lx + 0.6 * s, ly - 12.0 * s, lx + 1.6 * s, ly - 2.0 * s, st_d, 1.0);
    r.rect(lx - 5.0 * s, ly - 14.0 * s, lx + 5.0 * s, ly - 12.0 * s, st, 1.0);
    r.rect(lx - 5.0 * s, ly - 14.0 * s, lx + 5.0 * s, ly - 13.5 * s, st_l, 1.0);
    r.rect(lx - 3.6 * s, ly - 20.0 * s, lx + 3.6 * s, ly - 14.0 * s, st, 1.0);
    r.rect(lx - 2.2 * s, ly - 19.0 * s, lx + 2.2 * s, ly - 15.0 * s, (255, 206, 130), 1.0);
    r.rect(lx - 1.0 * s, ly - 18.0 * s, lx + 1.0 * s, ly - 16.0 * s, (255, 236, 186), 1.0);
    r.poly(&[(lx - 7.5 * s, ly - 20.0 * s), (lx + 7.5 * s, ly - 20.0 * s), (lx + 2.2 * s, ly - 25.0 * s),
        (lx - 2.2 * s, ly - 25.0 * s)], st, 1.0);
    r.line(lx - 7.5 * s, ly - 20.0 * s, lx - 2.2 * s, ly - 25.0 * s, st_l, 1.0, 1.0);
    r.rect(lx - 7.5 * s, ly - 20.0 * s, lx + 7.5 * s, ly - 19.4 * s, st_d, 1.0);
    r.rect(lx - s, ly - 27.5 * s, lx + s, ly - 25.0 * s, st, 1.0);
    lanterns.push((lx, ly - 17.0 * s));
    let (rxl, ryl) = (wf * 0.11, hf * 0.06);
    for y in ti(ly - ryl)..ti(ly + ryl) + 1 {
        for x in ti(lx - rxl)..ti(lx + rxl) + 1 {
            if 0 <= x && x < w && hy <= y && y < h {
                let d = ((x as f64 + 0.5 - lx) / rxl).powf(2.0) + ((y as f64 + 0.5 - ly) / ryl).powf(2.0);
                if d < 1.0 && dith(x, y, (1.0 - d) * 0.9) {
                    r.put(x as f64, y as f64, (255, 196, 128), 0.16 * (1.0 - d) + 0.04);
                }
            }
        }
    }

    // ---- water basin ----
    let (bx, by) = (wf * 0.065, hf * 0.90);
    let br = 5.0f64.max(wf * 0.034);
    r.disc(bx, by + br * 0.5, br * 1.15, (24, 40, 36), 0.6, 0.35);
    r.disc(bx, by, br, (92, 94, 112), 1.0, 0.62);
    r.disc(bx - br * 0.2, by - br * 0.12, br * 0.8, (118, 120, 138), 1.0, 0.55);
    r.disc(bx, by - br * 0.22, br * 0.6, (22, 28, 56), 1.0, 0.32);
    r.disc(bx + br * 0.15, by - br * 0.28, br * 0.16, (200, 204, 226), 0.6, 0.4);
    let sx0 = bx - br * 2.2;
    let sy0 = by - br * 1.25;
    r.line(sx0, sy0, bx - br * 0.25, sy0 + br * 0.2, (124, 140, 96), 1.5f64.max(br * 0.22), 1.0);
    r.line(sx0, sy0 - br * 0.08, bx - br * 0.25, sy0 + br * 0.1, (168, 182, 120), 1.0, 1.0);
    r.rect(sx0 + br * 0.4, sy0 - 1.0, sx0 + br * 0.55, by + br * 0.3, (84, 98, 70), 1.0);
    for k in 0..ti(br * 0.9) {
        r.put(bx - br * 0.22, sy0 + br * 0.25 + k as f64 * 0.9, (190, 210, 240), 0.55);
    }

    // ---- the grove ----
    for k in 0..grove.min(6) {
        let gx = if k < 3 { ti(wf * (0.20 + 0.11 * k as f64)) } else { ti(wf * (0.58 + 0.07 * (k - 3) as f64)) };
        let gb = wtf;
        r.rect(gx as f64, gb - hf * 0.03, (gx + 1) as f64, gb, (40, 30, 40), 1.0);
        for _ in 0..5 {
            let dx = rnd.uniform(-3.0, 3.0);
            let dy = rnd.uniform(-2.0, 2.0);
            let c = *rnd.choice(&[(214u8, 168u8, 196u8), (232, 188, 210), (190, 140, 176)]);
            r.disc(gx as f64 + dx, gb - hf * 0.04 + dy, 2.0f64.max(hf * 0.014), c, 1.0, 1.0);
        }
    }

    let tips = garden_tree(&mut r, cx, tb, h, stage, growth, health, seed);
    let lay = Lay {
        tree: (cx, tb), tips, lanterns, pond, horizon: hy, torii: (cx, wall_top), moon: (mx, my, mr), wall: wall_top,
        basin: (bx, by, br), breath: (cx, (wtf - hf * 0.30 + hyf - hf * 0.08) / 2.0),
    };
    (r, lay)
}

/// `_garden_tree`: seed -> sprout -> bonsai in a pot -> a great sakura with a shimenawa rope
fn garden_tree(r: &mut Ras, cx: i64, tb: i64, h: i64, stage: i64, growth: f64, health: f64, seed: u64) -> Vec<(f64, f64, i64)> {
    let mut rnd = PyRandom::new(seed * 31 + stage as u64);
    let s = h as f64 / 260.0;
    let (cx, tbf) = (cx as f64, tb as f64);
    let wilt = 1.0 - health.clamp(0.0, 1.0).max(0.0);
    let bark = lerp3((92, 62, 52), (70, 66, 64), wilt * 0.6);
    let bark_d = lerp3((58, 36, 34), (44, 42, 42), wilt * 0.6);
    let leaf = lerp3((86, 168, 104), (150, 128, 70), wilt * 0.9);
    let leaf_d = lerp3((44, 108, 74), (104, 82, 50), wilt * 0.9);
    let pink: Vec<Rgb> = [(255, 196, 214), (246, 160, 192), (255, 226, 236), (226, 124, 168)].iter()
        .map(|&c| lerp3(c, (170, 140, 120), wilt * 0.85)).collect();
    let mut tips = Vec::new();
    let potted = (1..=5).contains(&stage);
    if stage == 0 {
        r.disc(cx, tbf, 14.0 * s, (56, 40, 34), 1.0, 0.35);
        r.disc(cx, tbf - 2.0 * s, 9.0 * s, (76, 54, 44), 1.0, 0.3);
        for (k, a) in [(5.0, 0.12), (3.2, 0.2), (2.0, 0.4)] {
            r.disc(cx, tbf - 4.0 * s, k * s, (255, 216, 120), a * (0.4 + 0.6 * health), 1.0);
        }
        r.disc(cx, tbf - 4.0 * s, 1.4 * s, (255, 244, 190), 1.0, 1.0);
        return vec![(cx, tbf - 4.0 * s, 0)];
    }
    let base;
    if potted {
        let st = stage as f64;
        let (pw, ph_) = ((16 + stage * 7) as f64 * s, (6.0 + st * 1.6) * s);
        r.rect(cx - pw * 0.7, tbf + 2.0 * s, cx + pw * 0.7, tbf + 6.0 * s, (84, 82, 98), 1.0);
        r.rect(cx - pw * 0.6, tbf + 6.0 * s, cx - pw * 0.45, tbf + 12.0 * s, (70, 68, 84), 1.0);
        r.rect(cx + pw * 0.45, tbf + 6.0 * s, cx + pw * 0.6, tbf + 12.0 * s, (70, 68, 84), 1.0);
        r.poly(&[(cx - pw / 2.0, tbf - ph_), (cx + pw / 2.0, tbf - ph_), (cx + pw * 0.42, tbf + 2.0 * s), (cx - pw * 0.42, tbf + 2.0 * s)],
            (46, 66, 120), 1.0);
        r.rect(cx - pw / 2.0 - 1.0, tbf - ph_ - 1.5 * s, cx + pw / 2.0 + 1.0, tbf - ph_ + 0.5 * s, (210, 172, 92), 1.0);
        r.rect(cx - pw * 0.45, tbf - ph_ + 2.0 * s, cx + pw * 0.45, tbf - ph_ + 3.0 * s, (74, 98, 160), 1.0);
        r.rect(cx - pw / 2.0, tbf - ph_ - 0.5 * s - 1.0, cx + pw / 2.0, tbf - ph_ + 0.5 * s, (60, 46, 40), 1.0);
        base = tbf - ph_;
    } else {
        r.disc(cx, tbf + 2.0 * s, 46.0 * s, (30, 56, 48), 1.0, 0.22);
        for k in 0..7 {
            let a = PI * (0.05 + 0.9 * k as f64 / 6.0);
            let len = a.cos() * 34.0 * s * (0.6 + rnd.random() * 0.5);
            r.line(cx, tbf, cx + len, tbf + a.sin() * 4.0 * s, bark_d, 2.4 * s, 1.0);
        }
        base = tbf;
    }
    if stage == 1 {
        let ht = (8.0 + growth * 6.0) * s;
        r.line(cx, base, cx, base - ht, (110, 170, 90), 1.0f64.max(1.2 * s), 1.0);
        let droop = wilt * 4.0 * s;
        for sg in [-1.0, 1.0] {
            r.disc(cx + sg * 3.2 * s, base - ht + droop * 0.5, 2.6 * s, leaf, 1.0, 0.55);
            r.disc(cx + sg * 3.6 * s, base - ht + droop * 0.5 - 0.6 * s, 1.4 * s, lerp3(leaf, (220, 255, 200), 0.3), 1.0, 0.5);
        }
        return vec![(cx, base - ht, 0)];
    }
    let g = (stage as f64 - 2.0 + growth) / 6.0;
    let trunk_len = (16.0 + 64.0 * g.powf(0.8)) * s;
    let thick = (1.6 + 10.0 * g) * s;
    let depth = 2 + 5.min(ti(stage as f64 * 0.7));
    let droop = wilt * 0.35;
    let mut clusters: Vec<(f64, f64, i64)> = Vec::new();

    struct B<'a> {
        r: &'a mut Ras,
        rnd: &'a mut PyRandom,
        clusters: &'a mut Vec<(f64, f64, i64)>,
        droop: f64,
        depth: i64,
        bark: Rgb,
        bark_d: Rgb,
        s: f64,
    }
    fn branch(b: &mut B, x: f64, y: f64, ang: f64, l: f64, th: f64, d: i64) {
        let bend = b.droop * 0.6 * (1.0 - d as f64 / (b.depth + 1) as f64);
        let ang2 = if ang.cos() >= 0.0 { ang + bend } else { ang - bend };
        let (x1, y1) = (x + ang2.cos() * l, y + ang2.sin() * l);
        b.r.line(x, y, x1, y1, b.bark, 1.0f64.max(th), 1.0);
        if th > 2.5 {
            b.r.line(x + th * 0.25, y, x1 + th * 0.2, y1, b.bark_d, 1.0f64.max(th * 0.35), 1.0);
        }
        if d <= 0 || l < 3.0 * b.s {
            b.clusters.push((x1, y1, d));
            return;
        }
        let n = if b.rnd.random() < 0.7 { 2 } else { 3 };
        for k in 0..n {
            let mut spread = b.rnd.uniform(0.32, 0.62) * (if k % 2 == 1 { 1.0 } else { -1.0 });
            spread += if n == 3 && k == 2 { b.rnd.uniform(-0.2, 0.2) } else { 0.0 };
            let l2 = l * b.rnd.uniform(0.66, 0.8);
            branch(b, x1, y1, ang2 + spread, l2, th * 0.66, d - 1);
        }
        if d <= 2 {
            b.clusters.push((x1, y1, d));
        }
    }
    {
        let mut b = B { r: &mut *r, rnd: &mut rnd, clusters: &mut clusters, droop, depth, bark, bark_d, s };
        if potted {
            let mut pts = vec![(cx, base)];
            for k in 1..5 {
                let t = k as f64 / 4.0;
                pts.push((cx + (t * 3.2 + 0.4).sin() * trunk_len * 0.22, base - trunk_len * t));
            }
            for i in 0..pts.len() - 1 {
                let ((x0, y0), (x1, y1)) = (pts[i], pts[i + 1]);
                b.r.line(x0, y0, x1, y1, bark, thick * (1.0 - 0.18 * (y0 - base) / -trunk_len), 1.0);
            }
            let last = pts[pts.len() - 1];
            b.r.line(pts[0].0 + thick * 0.25, pts[0].1, last.0 + thick * 0.1, last.1, bark_d, 1.0f64.max(thick * 0.3), 1.0);
            for (k, &(x, y)) in pts.iter().enumerate().skip(1) {
                let sgs: &[f64] = if k >= 2 { &[-1.0, 1.0] } else { &[1.0] };
                for &sg in sgs {
                    let ang = -PI / 2.0 + sg * b.rnd.uniform(0.9, 1.3);
                    branch(&mut b, x, y, ang, trunk_len * 0.38 * (1.1 - k as f64 * 0.12), thick * 0.5, 1.max(depth - 2));
                }
            }
            let ang = -PI / 2.0 + b.rnd.uniform(-0.2, 0.2);
            branch(&mut b, last.0, last.1, ang, trunk_len * 0.25, thick * 0.45, 1.max(depth - 2));
        } else {
            let ang = -PI / 2.0 + b.rnd.uniform(-0.06, 0.06);
            branch(&mut b, cx, base, ang, trunk_len * 0.55, thick, depth);
        }
    }
    let keep = 0.35 + 0.65 * health;
    let blossom = stage >= 6 || (stage == 5 && growth > 0.4);
    let rr0 = (2.4 + 5.0 * g) * s * (0.55 + 0.45 * health);
    if blossom && stage >= 6 && !clusters.is_empty() {
        let xs: Vec<f64> = clusters.iter().map(|c| c.0).collect();
        let lo = xs.iter().cloned().fold(f64::INFINITY, f64::min);
        let hi = xs.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let mid = (lo + hi) / 2.0;
        for _ in 0..clusters.len() / 2 {
            let a = *rnd.choice(&clusters);
            let t = rnd.uniform(0.25, 0.8);
            clusters.push((mid + (a.0 - mid) * t, a.1 + (base - trunk_len * 0.75 - a.1) * t * 0.5, 3));
        }
        clusters.sort_by(|p, q| (p.1, p.0).partial_cmp(&(q.1, q.0)).unwrap());
    }
    for &(x, y, d) in &clusters {
        let rr = rr0 * rnd.uniform(0.65, 1.15);
        if rnd.random() > keep {
            continue;
        }
        if potted && !blossom {
            r.disc(x, y + 1.0, rr * 1.5, leaf_d, 1.0, 0.5);
            r.disc(x - rr * 0.2, y - rr * 0.15, rr * 1.25, leaf, 1.0, 0.45);
            r.disc(x - rr * 0.5, y - rr * 0.35, rr * 0.6, lerp3(leaf, (210, 240, 190), 0.35), 1.0, 0.4);
            tips.push((x, y, d));
            continue;
        }
        let cols: Vec<Rgb> = if blossom { pink.clone() } else { vec![leaf, leaf_d, lerp3(leaf, (210, 240, 190), 0.3)] };
        if stage == 5 && blossom {
            r.disc(x, y, rr, leaf_d, 1.0, 1.0);
            for _ in 0..4 {
                let dx = rnd.uniform(-rr, rr);
                let dy = rnd.uniform(-rr, rr);
                r.disc(x + dx, y + dy, 1.0f64.max(rr * 0.25), pink[1], 1.0, 1.0);
            }
            tips.push((x, y, d));
            continue;
        }
        r.disc(x + rr * 0.25, y + rr * 0.25, rr * 1.05, if blossom { cols[cols.len() - 1] } else { leaf_d }, 1.0, 1.0);
        r.disc(x, y, rr, cols[0], 1.0, 1.0);
        for _ in 0..3 {
            let dx = rnd.uniform(-rr, rr) * 0.7;
            let dy = rnd.uniform(-rr, rr) * 0.7;
            let c = *rnd.choice(&cols[..3]);
            r.disc(x + dx, y + dy, rr * 0.45, c, 1.0, 1.0);
        }
        r.disc(x - rr * 0.35, y - rr * 0.35, rr * 0.35, if blossom { cols[2] } else { lerp3(leaf, (220, 255, 210), 0.4) }, 1.0, 1.0);
        if blossom {
            for _ in 0..5 {
                let a = rnd.uniform(0.0, 6.283);
                r.put(x + a.cos() * rr * 1.25, y + a.sin() * rr * 1.2, cols[2], 1.0);
            }
            let dk = lerp3(cols[3], (60, 30, 50), 0.4);
            r.put(x + rr * 0.3, y + rr * 0.1, dk, 1.0);
            r.put(x + rr * 0.32, y + rr * 0.12 + 1.0, dk, 1.0);
        }
        tips.push((x, y, d));
    }
    if blossom && stage >= 6 {
        for _ in 0..ti(40.0 * health) + 6 {
            let gx = cx + rnd.gauss(0.0, 30.0 * s);
            let gy = tbf + rnd.uniform(-3.0, 9.0) * s;
            let c = *rnd.choice(&pink[..3]);
            r.put(gx, gy, c, 0.85);
        }
    }
    if stage >= 8 {
        let ry = base - trunk_len * 0.22;
        let rw = thick * 0.75;
        for x in ti(cx - rw)..ti(cx + rw) + 1 {
            let t = (x as f64 - cx + rw) / (2.0 * rw);
            let yy = ry + (PI * t).sin() * 2.0 * s;
            r.rect(x as f64, yy, (x + 1) as f64, yy + 2.4 * s, (226, 196, 120), 1.0);
            if x.rem_euclid(3) == 0 {
                r.put(x as f64, yy + 1.0, (170, 140, 80), 1.0);
            }
        }
        for k in [-0.5, 0.0, 0.5] {
            let sx = cx + k * rw;
            for j in 0..4i64 {
                let o = ((j % 2) as f64 * 1.5) * s;
                let jy = (j * 2) as f64 * s;
                r.rect(sx + o, ry + 2.5 * s + jy, sx + o + 2.0 * s, ry + 4.0 * s + jy, (250, 250, 244), 1.0);
            }
        }
    }
    tips
}

/// `z19_garden_season`: the time-of-day sky and the season's touches
fn season(r: &mut Ras, lay: &Lay, w: i64, h: i64, seed: u64, look: &Look) {
    if !(look.season || look.sky) {
        return;
    }
    let mut rnd = PyRandom::new(seed + 19);
    let (hy, wall) = (lay.horizon, lay.wall);
    let part = look.part.as_str();
    if look.sky && part != "night" {
        let (top, bot, k) = match part {
            "dawn" => ("#3a3466", "#a8707a", 0.30),
            "day" => ("#2c4a74", "#5f8a8c", 0.30),
            "dusk" => ("#3d2e5c", "#b8704a", 0.32),
            _ => ("", "", 0.0),
        };
        if !top.is_empty() {
            let (t3, b3) = (hex(top), hex(bot));
            for y in 0..1.max(wall) {
                let f = y as f64 / 1.max(wall) as f64;
                let col = lerp3(t3, b3, f);
                for x in 0..w {
                    let (r_, g_, b_) = r.get(x, y);
                    if b_ >= r_ && (r_ as i32 + g_ as i32 + b_ as i32) < 430 {
                        r.put(x as f64, y as f64, col, k * 0.9);
                    }
                }
            }
        }
    }
    if !look.season {
        return;
    }
    let weather = look.weather.as_deref();
    let (wf, hf, hyf) = (w as f64, h as f64, hy as f64);
    match look.season_name.as_str() {
        sn if sn == "winter" || weather == Some("snow") => {
            let snow = (232, 238, 255);
            for x in 0..w {
                for y in hy..h {
                    if rnd.random() < 0.035 {
                        r.put(x as f64, y as f64, snow, 0.55);
                    }
                }
            }
            for x in 0..w {
                for (yy, a) in [(wall - 1, 0.65), (wall - 2, 0.35)] {
                    let (r_, g_, b_) = r.get(x, yy);
                    let (r_, g_, b_) = (r_ as i32, g_ as i32, b_ as i32);
                    if (r_ - b_).abs() < 40 && (g_ - b_).abs() < 40 {
                        r.put(x as f64, yy as f64, snow, a);
                    }
                }
            }
        }
        "spring" => {
            for _ in 0..ti((w * h) as f64 / 900.0) {
                let x = rnd.uniform(0.0, wf);
                let y = rnd.uniform(hyf + 2.0, hf - 2.0);
                r.put(x, y, (255, 183, 213), 0.85);
                if rnd.random() < 0.5 {
                    r.put(x + 1.0, y, (255, 143, 191), 0.7);
                }
            }
        }
        "autumn" => {
            for _ in 0..ti((w * h) as f64 / 1300.0) {
                let x = rnd.uniform(0.0, wf);
                let y = rnd.uniform(hyf + 2.0, hf - 2.0);
                let c = *rnd.choice(&[(214u8, 82u8, 44u8), (232, 132, 52), (190, 54, 40), (222, 160, 60)]);
                r.put(x, y, c, 0.9);
                r.put(x + 1.0, y, c, 0.7);
                r.put(x, y + 1.0, c, 0.6);
            }
        }
        "summer" => {
            for y in hy..h {
                for x in 0..w {
                    let (r_, g_, b_) = r.get(x, y);
                    if g_ as i32 > r_ as i32 + 12 && g_ > b_ {
                        r.put(x as f64, y as f64, (r_, g_.saturating_add(10), b_), 1.0);
                    }
                }
            }
        }
        _ => {}
    }
    if weather == Some("rain") {
        for y in hy..h {
            if y % 3 != 0 {
                continue;
            }
            for x in 0..w {
                if rnd.random() < 0.02 {
                    r.put(x as f64, y as f64, (150, 180, 230), 0.5);
                }
            }
        }
    }
}
