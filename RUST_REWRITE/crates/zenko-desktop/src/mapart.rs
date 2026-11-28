//! The World map's paintings (zenko.py jmap_scene, asia_scene, kyoto_scene, myth_scene with JMap / RMap / SkyMap),
//! bit for bit: same raster (`_Ras`), same random draws in the same order, Python's int()/round()/floor.

use crate::gardenart::{BAYER, Ras, Rgb, lerp3, shade};
use serde_json::Value;
use std::sync::OnceLock;
use zenko_sound::mt::PyRandom;

pub fn mdata() -> &'static Value {
    static D: OnceLock<Value> = OnceLock::new();
    D.get_or_init(|| serde_json::from_str(include_str!("../data/map.json")).unwrap())
}

fn pts(v: &Value) -> Vec<(f64, f64)> {
    v.as_array().unwrap().iter().map(|p| (p[0].as_f64().unwrap(), p[1].as_f64().unwrap())).collect()
}

fn ti(v: f64) -> i64 {
    v as i64
}

/// the projection of a map: lon/lat (or fractions, for the legends) -> scene pixels
#[derive(Clone, Debug, PartialEq)]
pub enum Proj {
    /// JMap: Japan with the Okinawa inset
    Japan { w: i64, h: i64, s: f64, kx: f64, ox: f64, oy: f64, inset: (i64, i64, i64, i64) },
    /// RMap: a rectangular view (lon0, lon1, lat0, lat1)
    Rect { w: i64, h: i64, s: f64, kx: f64, ox: f64, oy: f64, view: (f64, f64, f64, f64) },
    /// SkyMap: places at fractions of the painting
    Sky { w: i64, h: i64, s: f64 },
}

impl Proj {
    pub fn japan(w: i64, h: i64) -> Proj {
        let (lon0, lon1, lat0, lat1) = (128.6, 146.4, 29.6, 46.0);
        let kx = 38.0f64.to_radians().cos();
        let (gw, gh) = ((lon1 - lon0) * kx, lat1 - lat0);
        let s = ((w - 8) as f64 / gw).min((h - 8) as f64 / gh);
        let (ox, oy) = ((w as f64 - gw * s) / 2.0, (h as f64 - gh * s) / 2.0);
        let (bw, bh) = (40.max(ti(w as f64 * 0.17)), 34.max(ti(h as f64 * 0.15)));
        Proj::Japan { w, h, s, kx, ox, oy, inset: (w - bw - 6, h - bh - 6, w - 6, h - 6) }
    }
    pub fn rect(w: i64, h: i64, view: (f64, f64, f64, f64)) -> Proj {
        let (lo0, lo1, la0, la1) = view;
        let kx = ((la0 + la1) / 2.0).to_radians().cos();
        let (gw, gh) = ((lo1 - lo0) * kx, la1 - la0);
        let s = ((w - 4) as f64 / gw).min((h - 4) as f64 / gh);
        Proj::Rect { w, h, s, kx, ox: (w as f64 - gw * s) / 2.0, oy: (h as f64 - gh * s) / 2.0, view }
    }
    pub fn sky(w: i64, h: i64) -> Proj {
        Proj::Sky { w, h, s: w.min(h) as f64 / 20.0 }
    }
    pub fn s(&self) -> f64 {
        match self {
            Proj::Japan { s, .. } | Proj::Rect { s, .. } | Proj::Sky { s, .. } => *s,
        }
    }
    pub fn kx(&self) -> f64 {
        match self {
            Proj::Japan { kx, .. } | Proj::Rect { kx, .. } => *kx,
            Proj::Sky { .. } => 1.0,
        }
    }
    pub fn inset(&self) -> (i64, i64, i64, i64) {
        match self {
            Proj::Japan { inset, .. } => *inset,
            _ => (-99, -99, -98, -98),
        }
    }
    pub fn xy(&self, lon: f64, lat: f64) -> (f64, f64) {
        match *self {
            Proj::Japan { s, kx, ox, oy, inset, .. } => {
                if lat < 28.0 {
                    let (x0, y0, x1, y1) = (inset.0 as f64, inset.1 as f64, inset.2 as f64, inset.3 as f64);
                    let u = (lon - 127.5) / 1.0;
                    let v = (lat - 25.95) / 1.05;
                    return (x0 + 6.0 + u * (x1 - x0 - 12.0), y1 - 8.0 - v * (y1 - y0 - 16.0));
                }
                (ox + (lon - 128.6) * kx * s, oy + (46.0 - lat) * s)
            }
            Proj::Rect { s, kx, ox, oy, view, .. } => (ox + (lon - view.0) * kx * s, oy + (view.3 - lat) * s),
            Proj::Sky { w, h, .. } => (lon * w as f64, lat * h as f64),
        }
    }
    /// RMap.ll(x, y)
    pub fn ll(&self, x: f64, y: f64) -> (f64, f64) {
        match *self {
            Proj::Rect { s, kx, ox, oy, view, .. } => (view.0 + (x - ox) / (kx * s), view.3 - (y - oy) / s),
            _ => (x, y),
        }
    }
}

/// _scan_fill(mask, w, h, pts, val)
fn scan_fill(mask: &mut [u8], w: i64, h: i64, pts: &[(f64, f64)], val: u8) {
    let n = pts.len();
    let lo = pts.iter().map(|p| p.1).fold(f64::INFINITY, f64::min);
    let hi = pts.iter().map(|p| p.1).fold(f64::NEG_INFINITY, f64::max);
    for y in 0.max(ti(lo))..h.min(ti(hi) + 1) {
        let yc = y as f64 + 0.5;
        let mut xs = vec![];
        for k in 0..n {
            let (x0, y0) = pts[k];
            let (x1, y1) = pts[(k + 1) % n];
            if (y0 <= yc && yc < y1) || (y1 <= yc && yc < y0) {
                xs.push(x0 + (yc - y0) * (x1 - x0) / (y1 - y0));
            }
        }
        xs.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let mut k = 0;
        while k + 1 < xs.len() {
            for x in 0.max(ti(xs[k].round_ties_even()))..w.min(ti(xs[k + 1].round_ties_even())) {
                mask[(y * w + x) as usize] = val;
            }
            k += 2;
        }
    }
}

/// the seigaiha night sea every map but the legends starts from
fn sea(r: &mut Ras, w: i64, h: i64) {
    let cell = 8.max(ti(w.min(h) as f64 / 26.0));
    let half = cell / 2;
    for y in 0..h {
        let t = y as f64 / 1.max(h - 1) as f64;
        let base = lerp3((14, 20, 54), (20, 36, 82), t);
        let row = y.div_euclid(half);
        for x in 0..w {
            let ox = (x + if row % 2 != 0 { half } else { 0 }).rem_euclid(cell) as f64 - cell as f64 / 2.0;
            let oy = y.rem_euclid(half) as f64 - cell as f64 / 2.0;
            let d = (ox * ox + oy * oy).sqrt() / (cell as f64 / 2.0);
            let ring = (d * 3.0).rem_euclid(1.0);
            let c = if d <= 1.0 && ring < 0.22 { lerp3(base, (60, 92, 150), 0.32) } else { base };
            let i = ((y * w + x) * 3) as usize;
            (r.b[i], r.b[i + 1], r.b[i + 2]) = c;
        }
    }
}

fn noise(x: i64, y: i64) -> f64 {
    let (xf, yf) = (x as f64, y as f64);
    (xf * 0.21 + yf * 0.13).sin() * (xf * 0.07 - yf * 0.19).sin() + ((xf + yf) * 0.05).sin() * 0.5
}

fn bayer(x: i64, y: i64) -> i64 {
    BAYER[y.rem_euclid(4) as usize][x.rem_euclid(4) as usize] as i64
}

/// the coastline: dark ink on land, foam on the sea, a dotted second ring
fn coast(r: &mut Ras, mask: &[u8], w: i64, h: i64) {
    for y in 1..h - 1 {
        for x in 1..w - 1 {
            let i = (y * w + x) as usize;
            let wu = w as usize;
            let m = mask[i];
            let nb = mask[i - 1] + mask[i + 1] + mask[i - wu] + mask[i + wu];
            if m != 0 && nb < 4 {
                r.put(x as f64, y as f64, (40, 46, 60), 1.0);
            } else if m == 0 && nb != 0 {
                r.put(x as f64, y as f64, (150, 196, 220), 0.75);
            } else if m == 0 && (mask[if x > 1 { i - 2 } else { i }] != 0 || mask[if x < w - 2 { i + 2 } else { i }] != 0
                || (y > 1 && mask[i - 2 * wu] != 0) || (y < h - 2 && mask[i + 2 * wu] != 0))
                && (x + y) % 2 == 0 {
                r.put(x as f64, y as f64, (110, 150, 196), 0.45);
            }
        }
    }
}

/// gold screen-style clouds
fn cloud(r: &mut Ras, x0: f64, y0: f64, ww: f64, top: Rgb) {
    let hh = 4.0f64.max(ww * 0.16);
    for (dx, rr) in [(0.0, 0.55), (0.32, 0.8), (0.62, 0.62), (0.88, 0.45)] {
        r.disc(x0 + dx * ww, y0, hh * rr * 1.2, (186, 150, 82), 1.0, 0.62);
    }
    for (dx, rr) in [(0.0, 0.55), (0.32, 0.8), (0.62, 0.62), (0.88, 0.45)] {
        r.disc(x0 + dx * ww, y0 - 1.0, hh * rr * 1.05, top, 1.0, 0.55);
    }
}

/// jmap_scene(w, h, seed=11)
pub fn jmap_scene(w: i64, h: i64) -> (Ras, Proj) {
    let d = mdata();
    let mut r = Ras::new(w, h);
    let p = Proj::japan(w, h);
    let mut rnd = PyRandom::new(11);
    sea(&mut r, w, h);
    let mut mask = vec![0u8; (w * h) as usize];
    for k in d["JMAP_LAND_ORDER"].as_array().unwrap() {
        let poly: Vec<(f64, f64)> = pts(&d["JMAP_LAND"][k.as_str().unwrap()]).into_iter().map(|(lo, la)| p.xy(lo, la)).collect();
        scan_fill(&mut mask, w, h, &poly, 1);
    }
    let oki: Vec<(f64, f64)> = pts(&d["JMAP_OKINAWA"]).into_iter().map(|(lo, la)| p.xy(lo, la)).collect();
    scan_fill(&mut mask, w, h, &oki, 1);
    let mut dist: Vec<u8> = mask.iter().map(|&m| if m != 0 { 255 } else { 0 }).collect();
    let (wu, hu) = (w as usize, h as usize);
    for y in 0..hu {
        for x in 0..wu {
            let i = y * wu + x;
            if dist[i] != 0 {
                let mut v = dist[i] as u16;
                if x > 0 {
                    v = v.min(dist[i - 1] as u16 + 1);
                }
                if y > 0 {
                    v = v.min(dist[i - wu] as u16 + 1);
                }
                dist[i] = v as u8;
            }
        }
    }
    for y in (0..hu).rev() {
        for x in (0..wu).rev() {
            let i = y * wu + x;
            if dist[i] != 0 {
                let mut v = dist[i] as u16;
                if x < wu - 1 {
                    v = v.min(dist[i + 1] as u16 + 1);
                }
                if y < hu - 1 {
                    v = v.min(dist[i + wu] as u16 + 1);
                }
                dist[i] = v as u8;
            }
        }
    }
    let sc = 1.0f64.max(p.s() / 22.0);
    for y in 0..h {
        for x in 0..w {
            let i = (y * w + x) as usize;
            if mask[i] == 0 {
                continue;
            }
            let dd = dist[i] as f64 / sc;
            let nz = noise(x, y);
            let mut c: Rgb = if dd <= 1.2 {
                (206, 186, 132)
            } else if dd < 3.2 + nz {
                (98, 142, 84)
            } else if dd < 6.5 + nz * 1.5 {
                (70, 116, 72)
            } else if nz > -0.4 {
                (52, 94, 66)
            } else {
                (66, 104, 70)
            };
            if dd > 2.5 && bayer(x, y) < 2 && nz > 0.3 {
                c = (40, 78, 58);
            }
            let j = i * 3;
            (r.b[j], r.b[j + 1], r.b[j + 2]) = c;
        }
    }
    coast(&mut r, &mask, w, h);
    let (bx, by) = p.xy(136.08, 35.28);
    r.disc(bx, by, 1.5f64.max(p.s() * 0.11), (44, 74, 130), 1.0, 1.8);
    let ms = 3.max(ti(p.s() * 0.16));
    let peak = |r: &mut Ras, x: f64, y: f64, s: i64, snow: f64, big: bool| {
        let (stone, dark) = if !big { ((128, 120, 132), (92, 86, 104)) } else { ((96, 108, 150), (70, 80, 120)) };
        for k in 0..s {
            let yy = y - s as f64 + k as f64;
            let half = k as f64 * if big { 1.15 } else { 0.95 };
            r.rect(x - half, yy, x, yy + 1.0, stone, 1.0);
            r.rect(x, yy, x + half + 1.0, yy + 1.0, dark, 1.0);
            if (k as f64) < s as f64 * snow {
                r.rect(x - half, yy, x + half + 1.0, yy + 1.0, (240, 244, 255), 1.0);
            }
        }
    };
    for (lo, la) in pts(&d["JMAP_PEAKS"]) {
        let (x, y) = p.xy(lo, la);
        if 0 <= ti(x) && ti(x) < w && 0 <= ti(y) && ti(y) < h && mask[(ti(y) * w + ti(x)) as usize] != 0 {
            let s = ms + rnd.randint(-1, 1);
            peak(&mut r, x, y, s, 0.35, false);
        }
    }
    let (fx, fy) = p.xy(138.73, 35.36);
    peak(&mut r, fx, fy, ti(ms as f64 * 1.8), 0.4, true);
    for (cx, cy, cw) in [(0.80, 0.30, 0.15), (0.12, 0.70, 0.13), (0.70, 0.62, 0.11), (0.30, 0.10, 0.12)] {
        let (x0, y0, ww) = (w as f64 * cx, h as f64 * cy, w as f64 * cw);
        let hh = 4.0f64.max(ww * 0.16);
        cloud(&mut r, x0, y0, ww, (232, 198, 120));
        r.rect(x0 - hh * 0.4, y0 + hh * 0.22, x0 + ww * 0.95, y0 + hh * 0.22 + 1.0, (186, 150, 82), 1.0);
    }
    let (cx, cy) = p.xy(131.2, 39.9);
    let rr = 6.0f64.max(p.s() * 0.9);
    let (g1, g2, g3) = ((232, 198, 120), (200, 170, 104), (150, 128, 90));
    for (ang, ln, col) in [(0.0, 1.0, g1), (90.0, 0.7, g2), (180.0, 0.7, g2), (270.0, 0.7, g2), (45.0, 0.42, g3),
                           (135.0, 0.42, g3), (225.0, 0.42, g3), (315.0, 0.42, g3)] {
        let a = (ang - 90.0f64).to_radians();
        let (tx, ty) = (cx + a.cos() * rr * ln, cy + a.sin() * rr * ln);
        let (px, py) = (-a.sin() * rr * 0.16, a.cos() * rr * 0.16);
        r.poly(&[(cx + px, cy + py), (tx, ty), (cx - px, cy - py)], col, 1.0);
    }
    r.disc(cx, cy, 1.5f64.max(rr * 0.12), (255, 236, 190), 1.0, 1.0);
    let (x0, y0, x1, y1) = p.inset();
    let (x0, y0, x1, y1) = (x0 as f64, y0 as f64, x1 as f64, y1 as f64);
    r.rect(x0, y0, x1, y1, (16, 24, 60), 0.6);
    for (a, b, c, dd) in [(x0, y0, x1, y0 + 1.0), (x0, y1 - 1.0, x1, y1), (x0, y0, x0 + 1.0, y1), (x1 - 1.0, y0, x1, y1)] {
        r.rect(a, b, c, dd, (200, 164, 92), 1.0);
    }
    r.poly(&oki, (98, 142, 84), 1.0);
    (r, p)
}

fn view(region: &str) -> (f64, f64, f64, f64) {
    let v = &mdata()["Z22_VIEWS"][region];
    (v[0].as_f64().unwrap(), v[1].as_f64().unwrap(), v[2].as_f64().unwrap(), v[3].as_f64().unwrap())
}

/// asia_scene(w, h, region): Korea, China or the Silk Road
pub fn asia_scene(w: i64, h: i64, region: &str) -> (Ras, Proj) {
    let d = mdata();
    let mut r = Ras::new(w, h);
    let p = Proj::rect(w, h, view(region));
    sea(&mut r, w, h);
    let mut mask = vec![1u8; (w * h) as usize];
    let proj = |v: &Value| -> Vec<(f64, f64)> { pts(v).into_iter().map(|(lo, la)| p.xy(lo, la)).collect() };
    scan_fill(&mut mask, w, h, &proj(&d["Z22_SEA"]), 0);
    for isl in d["Z22_ISLANDS"].as_array().unwrap() {
        scan_fill(&mut mask, w, h, &proj(isl), 1);
    }
    let (s, kx) = (p.s(), p.kx());
    for l in d["Z22_LAKES"].as_array().unwrap() {
        let (lo, la, rx, ry) = (l[0].as_f64().unwrap(), l[1].as_f64().unwrap(), l[2].as_f64().unwrap(), l[3].as_f64().unwrap());
        let (cx, cy) = p.xy(lo, la);
        let (rxp, ryp) = (rx * kx * s, ry * s);
        for y in 0.max(ti(cy - ryp) - 1)..h.min(ti(cy + ryp) + 2) {
            for x in 0.max(ti(cx - rxp) - 1)..w.min(ti(cx + rxp) + 2) {
                if ((x as f64 + 0.5 - cx) / 0.6f64.max(rxp)).powi(2) + ((y as f64 + 0.5 - cy) / 0.6f64.max(ryp)).powi(2) <= 1.0 {
                    mask[(y * w + x) as usize] = 0;
                }
            }
        }
    }
    let mut zones: Vec<(bool, f64, f64, f64, f64)> = vec![];
    for (key, desert) in [("Z22_DESERTS", true), ("Z22_HIGH", false)] {
        for z in d[key].as_array().unwrap() {
            let (cx, cy) = p.xy(z[0].as_f64().unwrap(), z[1].as_f64().unwrap());
            zones.push((desert, cx, cy, z[2].as_f64().unwrap() * kx * s, z[3].as_f64().unwrap() * s));
        }
    }
    for y in 0..h {
        for x in 0..w {
            let i = (y * w + x) as usize;
            if mask[i] == 0 {
                continue;
            }
            let nz = noise(x, y);
            let mut c: Rgb = if nz < 0.2 { (98, 142, 84) } else { (84, 128, 76) };
            if bayer(x, y) < 2 && nz > 0.45 {
                c = (58, 98, 64);
            }
            for &(desert, cx, cy, rx, ry) in &zones {
                let dd = ((x as f64 - cx) / 1.0f64.max(rx)).powi(2) + ((y as f64 - cy) / 1.0f64.max(ry)).powi(2);
                let edge = 1.0 + 0.18 * (x as f64 * 0.3 + y as f64 * 0.2).sin();
                if dd < edge {
                    if desert {
                        c = if (x * 3 + y * 5) % 11 != 0 { (214, 182, 118) } else { (196, 160, 96) };
                        if dd > edge * 0.8 && bayer(x, y) < 8 {
                            c = (170, 156, 96);
                        }
                    } else {
                        c = if nz > -0.2 { (150, 140, 120) } else { (128, 122, 112) };
                        if dd > edge * 0.85 && bayer(x, y) < 8 {
                            c = (120, 132, 96);
                        }
                    }
                }
            }
            let j = i * 3;
            (r.b[j], r.b[j + 1], r.b[j + 2]) = c;
        }
    }
    coast(&mut r, &mask, w, h);
    let rw = 1.0f64.max(s * 0.12);
    for rv in d["Z22_RIVERS"].as_array().unwrap() {
        let q = proj(rv);
        for k in 0..q.len() - 1 {
            r.line(q[k].0, q[k].1, q[k + 1].0, q[k + 1].1, (70, 120, 180), 2.2f64.min(rw), 1.0);
        }
    }
    let ms = 3.max(ti(9.0f64.min(s * 0.55)));
    let peak = |r: &mut Ras, x: f64, y: f64, s: i64| {
        for k in 0..s {
            let yy = y - s as f64 + k as f64;
            let half = k as f64 * 0.95;
            r.rect(x - half, yy, x, yy + 1.0, (128, 120, 132), 1.0);
            r.rect(x, yy, x + half + 1.0, yy + 1.0, (92, 86, 104), 1.0);
            if (k as f64) < s as f64 * 0.35 {
                r.rect(x - half, yy, x + half + 1.0, yy + 1.0, (240, 244, 255), 1.0);
            }
        }
    };
    let mut rnd = PyRandom::new_str(region);
    for rg in d["Z22_RANGES"].as_array().unwrap() {
        let q = proj(rg);
        if q.len() == 1 {
            peak(&mut r, q[0].0, q[0].1, ms + 1);
            continue;
        }
        for k in 0..q.len() - 1 {
            let ((x0, y0), (x1, y1)) = (q[k], q[k + 1]);
            let n = 1.max(ti(zenko_art::core::hypot(x1 - x0, y1 - y0) / (ms as f64 * 1.3)));
            for j in 0..n {
                let t = j as f64 / n as f64;
                let x = x0 + (x1 - x0) * t;
                let y = y0 + (y1 - y0) * t + rnd.uniform(-1.0, 1.0);
                if 0 <= ti(x) && ti(x) < w && 0 <= ti(y) && ti(y) < h && mask[(ti(y) * w + ti(x)) as usize] != 0 {
                    let s2 = ms + rnd.randint(-1, 1);
                    peak(&mut r, x, y, s2);
                }
            }
        }
    }
    for (cx, cy, cw) in [(0.86, 0.12, 0.12), (0.10, 0.86, 0.13)] {
        cloud(&mut r, w as f64 * cx, h as f64 * cy, w as f64 * cw, (232, 198, 120));
    }
    (r, p)
}

/// kyoto's tile hash
fn hsh(a: i64, b: i64, c: i64) -> f64 {
    let v: i128 = (a as i128 * 73856093) ^ (b as i128 * 19349663) ^ (c as i128 * 83492791);
    ((((v ^ (v >> 13)) * 1274126177) & 0xffffffff) as f64) / 4294967296.0
}

/// kyoto_scene(w, h, season)
pub fn kyoto_scene(w: i64, h: i64, season: &str) -> (Ras, Proj) {
    let mut r = Ras::new(w, h);
    let p = Proj::rect(w, h, view("kyoto"));
    let win = season == "winter";
    let tree: [Rgb; 3] = match season {
        "spring" => [(246, 178, 206), (230, 140, 180), (252, 214, 228)],
        "summer" => [(70, 132, 74), (52, 110, 62), (96, 150, 84)],
        "autumn" => [(214, 96, 46), (188, 58, 38), (232, 150, 60)],
        _ => [(236, 240, 250), (206, 214, 232), (250, 252, 255)],
    };
    let flat: Rgb = if !win { (176, 168, 138) } else { (214, 218, 228) };
    let hill: Rgb = if !win { (72, 110, 74) } else { (170, 182, 200) };
    let roofs: [Rgb; 4] = if !win { [(128, 112, 104), (112, 100, 96), (142, 122, 106), (98, 92, 96)] } else {
        [(226, 230, 240), (210, 216, 230), (238, 240, 248), (200, 206, 222)]
    };
    let street = if !win { lerp3(flat, (236, 228, 206), 0.45) } else { (240, 242, 248) };
    let park: Rgb = if !win { (96, 140, 86) } else { (222, 228, 240) };
    for y in 0..h {
        for x in 0..w {
            let (lon, lat) = p.ll(x as f64, y as f64);
            let mut e = 0.0f64.max((lon - 135.786) / 0.02) + 0.0f64.max((135.69 - lon) / 0.02) + 0.0f64.max((lat - 35.052) / 0.012);
            e += 0.25 * (x as f64 * 0.15).sin() * (y as f64 * 0.11).sin();
            let c: Rgb;
            if e > 0.55 {
                let mut cc = lerp3(hill, shade(hill, 0.72), 1.0f64.min((e - 0.55) * 0.6));
                let (cx, cy) = (x.div_euclid(5), y.div_euclid(5));
                if hsh(cx, cy, 0) < 0.13 {
                    let (dx, dy) = (x - cx * 5 - 2, y - cy * 5 - 2);
                    if dx * dx + dy * dy <= 4 {
                        cc = lerp3(tree[(hsh(cx, cy, 1) * 3.0) as usize], hill, 0.35);
                        if dy < 0 && dx < 0 {
                            cc = shade(cc, 1.12);
                        }
                    }
                }
                c = cc;
            } else {
                let (gx, gy) = ((lon - 135.64) / 0.0075, (lat - 34.945) / 0.0062);
                let (fx, fy) = (gx - gx.floor(), gy - gy.floor());
                if fx.min(1.0 - fx) < 0.06 || fy.min(1.0 - fy) < 0.06 {
                    c = street;
                } else {
                    let (bx, by) = (gx.floor() as i64, gy.floor() as i64);
                    let kind = hsh(bx, by, 0);
                    if kind < 0.12 {
                        let mut cc = park;
                        if hsh(x.div_euclid(3), y.div_euclid(3), 7) < 0.3 {
                            cc = lerp3(tree[(hsh(x.div_euclid(3), y.div_euclid(3), 8) * 3.0) as usize], park, 0.3);
                        }
                        c = cc;
                    } else {
                        let sub = (fy * 5.0) as i64;
                        let rc = roofs[(hsh(bx, by, sub) * 4.0) as usize];
                        let m = (fy * 5.0).rem_euclid(1.0);
                        let mut cc = if m < 0.18 { shade(rc, 0.86) } else { rc };
                        if m > 0.82 {
                            cc = shade(rc, 1.1);
                        }
                        if !win && hsh(x.div_euclid(2), y.div_euclid(2), 3) < 0.012 {
                            cc = lerp3(tree[1], rc, 0.4);
                        }
                        c = cc;
                    }
                }
            }
            let i = ((y * w + x) * 3) as usize;
            (r.b[i], r.b[i + 1], r.b[i + 2]) = c;
        }
    }
    let water: Rgb = if !win { (70, 120, 180) } else { (110, 140, 190) };
    let kamo = [(135.762, 35.075), (135.768, 35.05), (135.772, 35.03), (135.771, 35.0), (135.768, 34.975), (135.762, 34.945)];
    let katsura = [(135.66, 35.03), (135.675, 35.012), (135.69, 34.99), (135.705, 34.96), (135.715, 34.945)];
    for (rv, wd) in [(&kamo[..], 1.5f64.max(w as f64 / 170.0)), (&katsura[..], 2.0f64.max(w as f64 / 130.0))] {
        let q: Vec<(f64, f64)> = rv.iter().map(|&(lo, la)| p.xy(lo, la)).collect();
        for k in 0..q.len() - 1 {
            r.line(q[k].0, q[k].1, q[k + 1].0, q[k + 1].1, water, wd, 1.0);
        }
    }
    let (x0, y0) = p.xy(135.758, 35.03);
    let (x1, y1) = p.xy(135.765, 35.018);
    r.rect(x0, y0, x1, y1, park, 1.0);
    let mut rnd = PyRandom::new(7);
    if win || season == "spring" {
        let col: Rgb = if win { (255, 255, 255) } else { (255, 206, 226) };
        for _ in 0..ti((w * h) as f64 / 90.0) {
            let x = rnd.randbelow(w as u64) as f64;
            let y = rnd.randbelow(h as u64) as f64;
            r.put(x, y, col, 0.7);
        }
    }
    (r, p)
}

/// myth_scene(w, h): the legends under a night sky
pub fn myth_scene(w: i64, h: i64) -> (Ras, Proj) {
    let mut r = Ras::new(w, h);
    let p = Proj::sky(w, h);
    let mut rnd = PyRandom::new(42);
    let (wf, hf) = (w as f64, h as f64);
    let sea_y = ti(hf * 0.74);
    for y in 0..h {
        let t = y as f64 / 1.max(h - 1) as f64;
        for x in 0..w {
            let c: Rgb = if y < sea_y {
                let mut c = lerp3((10, 12, 38), (44, 40, 96), t / 0.74);
                if bayer(x, y) < 2 && (x as f64 * 0.05 + y as f64 * 0.09).sin() > 0.6 {
                    c = shade(c, 1.12);
                }
                c
            } else {
                let tt = (y - sea_y) as f64 / 1.max(h - sea_y) as f64;
                let c = lerp3((26, 52, 104), (8, 18, 52), tt);
                if (x + ti(4.0 * (y as f64 * 0.6).sin())).rem_euclid(14) == 0 { lerp3(c, (90, 140, 200), 0.4) } else { c }
            };
            let i = ((y * w + x) * 3) as usize;
            (r.b[i], r.b[i + 1], r.b[i + 2]) = c;
        }
    }
    for _ in 0..ti((w * h) as f64 / 160.0) {
        let x = rnd.randbelow(w as u64) as f64;
        let y = rnd.randbelow(ti(sea_y as f64 * 0.9) as u64) as f64;
        let a = rnd.uniform(0.4, 1.0);
        r.put(x, y, (255, 246, 220), a);
    }
    let (mx, my, mr) = (wf * 0.86, hf * 0.13, 8.0f64.max(w.min(h) as f64 * 0.1));
    r.disc(mx, my, mr * 1.9, (120, 110, 160), 0.12, 1.0);
    r.disc(mx, my, mr, (244, 236, 206), 1.0, 1.0);
    for (dx, dy, rr) in [(-0.3, -0.2, 0.18), (0.25, 0.15, 0.14), (-0.05, 0.35, 0.1)] {
        r.disc(mx + dx * mr, my + dy * mr, rr * mr, (220, 208, 176), 1.0, 1.0);
    }
    for (px, py, s) in [(0.3, 0.42, 0.22), (0.38, 0.47, 0.16), (0.22, 0.5, 0.15)] {
        let (cx, cy, ss) = (px * wf, py * hf, s * hf);
        r.poly(&[(cx - ss * 0.8, cy + ss * 0.6), (cx, cy - ss * 0.6), (cx + ss * 0.8, cy + ss * 0.6)], (70, 74, 120), 1.0);
        r.poly(&[(cx - ss * 0.25, cy - ss * 0.2), (cx, cy - ss * 0.6), (cx + ss * 0.25, cy - ss * 0.2)], (230, 236, 255), 1.0);
    }
    for _ in 0..9 {
        let cx = rnd.uniform(0.0, 1.0) * wf;
        let cy = rnd.uniform(0.5, 0.7) * hf;
        let cw = rnd.uniform(0.12, 0.22) * wf;
        let hh = 4.0f64.max(cw * 0.18);
        for (dx, rr) in [(0.0, 0.55), (0.32, 0.8), (0.62, 0.62), (0.88, 0.45)] {
            r.disc(cx + dx * cw, cy, hh * rr * 1.2, (186, 150, 82), 1.0, 0.62);
        }
        for (dx, rr) in [(0.0, 0.55), (0.32, 0.8), (0.62, 0.62), (0.88, 0.45)] {
            r.disc(cx + dx * cw, cy - 1.0, hh * rr * 1.05, (232, 214, 170), 1.0, 0.55);
        }
    }
    for k in 0..6 {
        r.disc(wf * (0.56 + k as f64 * 0.04), hf * 0.27, hf * 0.045, (210, 206, 236), 1.0, 0.5);
    }
    let (ix, iy) = (wf * 0.17, hf * 0.66);
    r.disc(ix, iy, wf * 0.07, (52, 96, 86), 1.0, 0.45);
    r.poly(&[(ix - wf * 0.03, iy - 2.0), (ix, iy - hf * 0.12), (ix + wf * 0.03, iy - 2.0)], (70, 120, 96), 1.0);
    r.disc(ix, iy - hf * 0.02, wf * 0.09, (220, 230, 250), 0.18, 0.4);
    let (gx, gy) = (wf * 0.52, hf * 0.86);
    for (rr, a) in [(0.12, 0.10), (0.08, 0.16), (0.045, 0.3)] {
        r.disc(gx, gy, wf * rr, (120, 230, 220), a, 0.45);
    }
    r.rect(gx - wf * 0.03, gy - hf * 0.03, gx + wf * 0.03, gy, (180, 40, 40), 1.0);
    r.poly(&[(gx - wf * 0.045, gy - hf * 0.03), (gx, gy - hf * 0.06), (gx + wf * 0.045, gy - hf * 0.03)], (40, 120, 110), 1.0);
    for k in 0..9 {
        let kf = k as f64;
        let (fx, fy) = (wf * (0.84 + 0.11 * (kf * 0.7).cos()), hf * (0.48 + 0.08 * (kf * 1.3).sin()));
        r.disc(fx, fy, 3.2, (110, 200, 255), 0.35, 1.0);
        r.disc(fx, fy, 1.4, (220, 246, 255), 1.0, 1.0);
    }
    (r, p)
}

/// map_draw's painting for a region (Kyoto in the given season)
pub fn region_scene(region: &str, w: i64, h: i64, season: &str) -> (Ras, Proj) {
    match region {
        "japan" => jmap_scene(w, h),
        "kyoto" => kyoto_scene(w, h, season),
        "myth" => myth_scene(w, h),
        _ => asia_scene(w, h, region),
    }
}
