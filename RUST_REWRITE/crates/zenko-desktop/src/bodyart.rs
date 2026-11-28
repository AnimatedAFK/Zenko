//! The Exercise page's muscle map (zenko.py _body_grid / body_frames / gym_pngs): a broad pixel figure, front or
//! back, its regions found by point-in-polygon on the widened drawing, round dithered shading, grooves between
//! muscles, the target muscles glowing in the accent (and pulsing over 4 frames with a soft aura), done ones gold,
//! the kitsune mask on the head.

use serde_json::Value;
use std::sync::OnceLock;

pub fn gym_data() -> &'static Value {
    static D: OnceLock<Value> = OnceLock::new();
    D.get_or_init(|| serde_json::from_str(include_str!("../data/gym.json")).expect("gym.json"))
}

pub const BODY_W: usize = 88;
pub const BODY_H: usize = 178;
const OX: f64 = 8.0;
const OY: f64 = 4.0;
const ARM: [&str; 6] = ["delts_lo", "biceps", "triceps", "forearms", "elbow", "hand"];
const LEGS: [&str; 7] = ["legbase", "quads", "adductors", "knee", "calves", "tabi", "hamstrings"];
const HIPS: [&str; 2] = ["shorts", "glutes"];
const SKINLIKE: [&str; 5] = ["neck", "elbow", "hand", "knee", "head"];

type C3 = (i64, i64, i64);

fn pt_in(poly: &[(f64, f64)], x: f64, y: f64) -> bool {
    let mut inside = false;
    let n = poly.len();
    let mut j = n - 1;
    for i in 0..n {
        let (xi, yi) = poly[i];
        let (xj, yj) = poly[j];
        let d = if yj - yi != 0.0 { yj - yi } else { 1e-9 };
        if (yi > y) != (yj > y) && x < (xj - xi) * (y - yi) / d + xi {
            inside = !inside;
        }
        j = i;
    }
    inside
}

fn bulk_k(y: f64) -> f64 {
    if y < 22.0 {
        1.0
    } else if y < 31.0 {
        1.0 + (y - 22.0) / 9.0 * 0.38
    } else if y < 52.0 {
        if y < 46.0 { 1.38 } else { 1.38 - (y - 46.0) / 6.0 * 0.04 }
    } else if y < 82.0 {
        1.34 - (y - 52.0) / 30.0 * 0.22
    } else {
        1.12
    }
}

fn regions(view: &str) -> Vec<(String, Vec<(f64, f64)>)> {
    let k = if view == "front" { "BODY_FRONT" } else { "BODY_BACK" };
    gym_data()[k].as_array().unwrap().iter().map(|r| {
        (r[0].as_str().unwrap().to_string(),
         r[1].as_array().unwrap().iter().map(|p| (p[0].as_f64().unwrap(), p[1].as_f64().unwrap())).collect())
    }).collect()
}

/// _body_grid(view): each pixel's region name (None outside)
fn body_grid(view: &str) -> Vec<Vec<Option<String>>> {
    let regs = regions(view);
    let mut g = vec![vec![None; BODY_W]; BODY_H];
    for (y, row) in g.iter_mut().enumerate() {
        for (x, cell) in row.iter_mut().enumerate() {
            let mut px = x as f64 + 0.5 - OX;
            if px > 36.0 {
                px = 72.0 - px;
            }
            let py = y as f64 + 0.5 - OY;
            let tx = 36.0 - (36.0 - px) / bulk_k(py);
            let hx = 36.0 - (36.0 - px) / 1.12;
            let lx = 27.5 - (27.5 - (px + 1.6)) / (if py < 133.0 { 1.24 } else if py < 142.0 { 1.1 } else { 1.26 });
            let mut ax = px + 4.6 + (py - 44.0).max(0.0) * 0.07;
            ax = 15.2 - (15.2 - ax) / (if py < 66.0 { 1.48 } else { 1.3 });
            for (name, poly) in &regs {
                let n = name.as_str();
                let q = if ARM.contains(&n) { ax } else if LEGS.contains(&n) { lx } else if HIPS.contains(&n) { hx } else { tx };
                if pt_in(poly, q, py) {
                    *cell = Some(name.clone());
                }
            }
        }
    }
    for (y, row) in g.iter_mut().enumerate().take(32) {
        for (x, cell) in row.iter_mut().enumerate() {
            if ((x as f64 + 0.5 - 36.0 - OX) / 8.8).powi(2) + ((y as f64 + 0.5 - 15.5) / 11.2).powi(2) <= 1.0 {
                *cell = Some("head".into());
            }
        }
    }
    for ex in [29.5 + OX, 42.5 + OX] {
        for (y, row) in g.iter_mut().enumerate().take(10) {
            let w = (y as f64 + 1.0) * 0.62;
            for (x, cell) in row.iter_mut().enumerate() {
                if (x as f64 + 0.5 - ex).abs() <= w * 0.5 && cell.is_none() {
                    *cell = Some("ear".into());
                }
            }
        }
    }
    g
}

fn mix(a: C3, b: C3, t: f64) -> C3 {
    let t = t.clamp(0.0, 1.0);
    let m = |a: i64, b: i64| (a as f64 + (b - a) as f64 * t) as i64;
    (m(a.0, b.0), m(a.1, b.1), m(a.2, b.2))
}

/// body_frames(view, targets, accent, frames=4, done): each frame's pixels (rgba, row by row)
pub fn body_frames(view: &str, targets: &[String], accent: C3, done: &[String]) -> Vec<Vec<[u8; 4]>> {
    let g = body_grid(view);
    let mut lines = std::collections::HashSet::new();
    for l in gym_data()["BODY_LINES"][view].as_array().unwrap() {
        let p = |i: usize, j: usize| l[i][j].as_f64().unwrap();
        let (x0, y0, x1, y1) = (p(0, 0), p(0, 1), p(1, 0), p(1, 1));
        let n = ((x1 - x0).abs().max((y1 - y0).abs()) * 2.0) as i64 + 1;
        for k in 0..=n {
            let t = k as f64 / n as f64;
            let (x, y) = (x0 + (x1 - x0) * t, y0 + (y1 - y0) * t + OY);
            let f = if y < 90.0 + OY { 1.3 } else { 1.2 };
            for xx in [(36.0 - (36.0 - x) * f) as i64 + OX as i64, (36.0 + (36.0 - x) * f) as i64 + OX as i64] {
                if (0..BODY_W as i64).contains(&xx) && y >= 0.0 && (y as i64) < BODY_H as i64 {
                    lines.insert((xx, y as i64));
                }
            }
        }
    }
    let done: Vec<&str> = done.iter().map(|s| s.as_str()).collect();
    let targets: Vec<&str> = targets.iter().map(|s| s.as_str()).filter(|t| !done.contains(t)).collect();
    let gold: [C3; 5] = [(104, 72, 22), (176, 130, 50), (232, 188, 92), (250, 220, 146), (255, 244, 212)];
    let stone: [C3; 5] = [(78, 72, 128), (138, 130, 182), (186, 180, 220), (226, 222, 246), (246, 244, 255)];
    let skin: [C3; 5] = [(84, 76, 118), (150, 138, 176), (198, 188, 216), (230, 224, 242), (248, 244, 255)];
    let hot: [C3; 5] = [(110, 26, 40), mix(accent, (40, 0, 20), 0.35), accent, mix(accent, (255, 214, 160), 0.45),
                        mix(accent, (255, 248, 226), 0.75)];
    let shorts: [C3; 5] = [(12, 14, 38), (26, 34, 84), (40, 54, 118), (62, 84, 160), (90, 116, 196)];
    let tabi: [C3; 5] = [(110, 108, 136), (190, 190, 212), (232, 232, 244), (250, 250, 255), (255, 255, 255)];
    let outline: C3 = (18, 14, 40);
    let reg = |x: i64, y: i64| -> Option<&str> {
        if (0..BODY_W as i64).contains(&x) && (0..BODY_H as i64).contains(&y) { g[y as usize][x as usize].as_deref() } else { None }
    };
    let mut hu = vec![vec![0.5f64; BODY_W]; BODY_H];
    let mut vu = vec![vec![0.5f64; BODY_W]; BODY_H];
    for y in 0..BODY_H {
        let mut x = 0;
        while x < BODY_W {
            let r = &g[y][x];
            let mut x2 = x;
            while x2 + 1 < BODY_W && g[y][x2 + 1] == *r {
                x2 += 1;
            }
            if r.is_some() {
                for k in x..=x2 {
                    hu[y][k] = (k as f64 - x as f64 + 0.5) / (x2 - x + 1) as f64;
                }
            }
            x = x2 + 1;
        }
    }
    for x in 0..BODY_W {
        let mut y = 0;
        while y < BODY_H {
            let r = &g[y][x];
            let mut y2 = y;
            while y2 + 1 < BODY_H && g[y2 + 1][x] == *r {
                y2 += 1;
            }
            if r.is_some() {
                for k in y..=y2 {
                    vu[k][x] = (k as f64 - y as f64 + 0.5) / (y2 - y + 1) as f64;
                }
            }
            y = y2 + 1;
        }
    }
    let mut aura = std::collections::HashMap::new();
    if !targets.is_empty() {
        for y in 0..BODY_H as i64 {
            for x in 0..BODY_W as i64 {
                if reg(x, y).is_none() {
                    let mut best = 9;
                    for dy in -3..4i64 {
                        for dx in -3..4i64 {
                            if reg(x + dx, y + dy).is_some_and(|r| targets.contains(&r)) {
                                best = best.min(dx.abs() + dy.abs());
                            }
                        }
                    }
                    if best <= 4 {
                        aura.insert((x, y), best);
                    }
                }
            }
        }
    }
    let mask: Vec<(i64, i64, C3)> = gym_data()["MASK_PIX"].as_array().unwrap().iter().map(|m| {
        let c = &m[2];
        (m[0].as_i64().unwrap(), m[1].as_i64().unwrap(), (c[0].as_i64().unwrap(), c[1].as_i64().unwrap(), c[2].as_i64().unwrap()))
    }).collect();
    let mut out = vec![];
    for fk in 0..4 {
        let phase = fk as f64 / 4.0;
        let pulse = 0.5 + 0.5 * (phase * 2.0 * std::f64::consts::PI).sin();
        let mut px = Vec::with_capacity(BODY_W * BODY_H);
        for y in 0..BODY_H as i64 {
            for x in 0..BODY_W as i64 {
                let (xu, yu) = (x as usize, y as usize);
                let Some(r) = reg(x, y) else {
                    let v = aura.get(&(x, y)).map(|&d| ((60.0 + 80.0 * pulse) * (1.0 - (d as f64 - 1.0) / 4.0)) as i64);
                    px.push(match v {
                        Some(a) if a > 4 => [(accent.0 + 20).min(255) as u8, (accent.1 + 80).min(255) as u8,
                                             (accent.2 + 60).min(255) as u8, a as u8],
                        _ => [0, 0, 0, 0],
                    });
                    continue;
                };
                let nb = [reg(x - 1, y), reg(x + 1, y), reg(x, y - 1), reg(x, y + 1)];
                let rgb = |c: C3| [c.0 as u8, c.1 as u8, c.2 as u8, 255];
                if nb.contains(&None) {
                    px.push(rgb(outline));
                    continue;
                }
                if r == "ear" {
                    let inner = (x as f64 + 0.5 - OX - if (x as f64) < 36.0 + OX { 29.5 } else { 42.5 }).abs() < 1.2 && y > 3;
                    px.push(rgb(if inner { (226, 64, 82) } else { (240, 236, 250) }));
                    continue;
                }
                let tg = targets.contains(&r);
                let dn = done.contains(&r);
                let pal: [C3; 5] = if SKINLIKE.contains(&r) {
                    skin
                } else if r == "shorts" {
                    shorts
                } else if r == "glutes" {
                    if tg {
                        std::array::from_fn(|i| mix(shorts[i], hot[i], 0.82 + 0.15 * pulse))
                    } else if dn {
                        std::array::from_fn(|i| mix(shorts[i], gold[i], 0.75))
                    } else {
                        shorts
                    }
                } else if r == "tabi" {
                    tabi
                } else if tg {
                    hot
                } else if dn {
                    gold
                } else {
                    stone
                };
                let u = hu[yu][xu] * 0.7 + vu[yu][xu] * 0.3;
                let dith = (x + y) % 2 != 0;
                let mut k = if u < 0.16 {
                    if vu[yu][xu] < 0.5 { 4 } else { 3 }
                } else if u < 0.34 {
                    if dith || u < 0.25 { 3 } else { 2 }
                } else if u < 0.62 {
                    2
                } else if u < 0.78 {
                    if dith { 1 } else { 2 }
                } else {
                    1
                };
                let sep = nb.iter().any(|n| n.is_some_and(|n| n != r));
                if sep && !["shorts", "tabi", "glutes", "torso", "legbase"].contains(&r) {
                    let a = reg(x + 1, y);
                    let b = reg(x, y + 1);
                    if a.is_some_and(|a| a != r) || b.is_some_and(|b| b != r) {
                        k = 0;
                    }
                }
                if lines.contains(&(x, y)) && !["shorts", "tabi"].contains(&r) && !SKINLIKE.contains(&r) {
                    k = if tg { 0 } else { 1 };
                }
                let mut c = pal[k];
                if tg && r != "glutes" && k >= 3 {
                    c = mix(c, (255, 250, 232), 0.3 * pulse);
                }
                let yy = y - OY as i64;
                let xo = x - OX as i64;
                if r == "shorts" && (yy == 80 || yy == 81) {
                    c = if yy == 80 { (238, 200, 110) } else { (168, 128, 56) };
                }
                if r == "shorts" && view == "front" && (33..=38).contains(&xo) && (84..=92).contains(&yy) && (x + yy) % 3 == 0 {
                    c = shorts[3];
                }
                if r == "tabi" && (xo == 29 || xo == 42) && yy >= 168 {
                    c = tabi[0];
                }
                if r == "head" {
                    c = mix((248, 246, 255), (176, 170, 214), (hu[yu][xu] - 0.55).max(0.0) * 1.6);
                    if view == "front" {
                        for &(ax, ay, col) in &mask {
                            if (x == ax + OX as i64 || x == 71 - ax + OX as i64) && yy == ay {
                                c = col;
                            }
                        }
                    } else {
                        if yy == 14 || yy == 15 {
                            c = if yy == 14 { (222, 46, 64) } else { (150, 24, 40) };
                        }
                        if (34..=37).contains(&xo) && (15..=21).contains(&yy) && (x + yy) % 2 == 0 {
                            c = (222, 46, 64);
                        }
                    }
                }
                px.push(rgb(c));
            }
        }
        out.push(px);
    }
    out
}
