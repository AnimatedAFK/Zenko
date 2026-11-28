//! The Town square's festival street (zenko.py zenko23 z23_street, z23_person_rows / z23_person_pal / z23_look, the
//! festival calendar and sky mode) bit for bit, and the square's safety filters (square_clean, square_name_ok,
//! square_tag).

use crate::gardenart::{BAYER, Ras, Rgb, lerp3, shade};
use crate::rewards::days_from_civil;
use serde_json::{Value, json};
use std::sync::OnceLock;
use zenko_sound::mt::PyRandom;

pub fn sdata() -> &'static Value {
    static D: OnceLock<Value> = OnceLock::new();
    D.get_or_init(|| serde_json::from_str(include_str!("../data/square.json")).unwrap())
}

fn ti(v: f64) -> i64 {
    v as i64
}

pub fn rgb(h: &str) -> Rgb {
    let h = h.trim_start_matches('#');
    let p = |i: usize| u8::from_str_radix(h.get(i..i + 2).unwrap_or("00"), 16).unwrap_or(0);
    (p(0), p(2), p(4))
}

fn strs(v: &Value) -> Vec<String> {
    v.as_array().map(|a| a.iter().filter_map(|s| s.as_str().map(String::from)).collect()).unwrap_or_default()
}

/// Z23_FEST[key]: (kanji, name, lantern colours, stalls, ambient, fireworks a minute)
pub fn fest(key: &str) -> &'static Value {
    let f = &sdata()["Z23_FEST"];
    if f[key].is_array() { &f[key] } else { &f["summer"] }
}

pub fn stall(key: &str) -> &'static Value {
    &sdata()["Z23_STALLS"][key]
}

// ---- the calendar ------------------------------------------------------------------------------------------------

fn md(tab: &str, year: i64) -> Option<(i64, i64)> {
    let v = &sdata()[tab][year.to_string().as_str()];
    Some((v[0].as_i64()?, v[1].as_i64()?))
}

/// z23_fest_calendar(year): [(key, first day, last day)] as day numbers, most special first
pub fn fest_calendar(year: i64) -> Vec<(&'static str, i64, i64)> {
    let d = |m: i64, dd: i64| days_from_civil(year, m, dd);
    let mut out = vec![("omisoka", d(12, 31), d(12, 31)), ("newyear", d(1, 1), d(1, 3)), ("christmas", d(12, 20), d(12, 26)),
                       ("halloween", d(10, 25), d(10, 31)), ("setsubun", d(2, 2), d(2, 3)), ("valentine", d(2, 13), d(2, 14)),
                       ("hina", d(3, 1), d(3, 3)), ("tanabata", d(7, 5), d(7, 7)), ("kodomo", d(5, 3), d(5, 5)),
                       ("obon", d(8, 13), d(8, 16)), ("hanami", d(3, 25), d(4, 8)), ("summer", d(7, 20), d(8, 12))];
    for (key, tab, before, after) in [("diwali", "DIWALI", 1, 2), ("holi", "HOLI", 1, 0), ("tsukimi", "TSUKIMI", 1, 1)] {
        if let Some((m, dd)) = md(tab, year) {
            let x = d(m, dd);
            out.insert(0, (key, x - before, x + after));
        }
    }
    out
}

/// z23_fest_today(day): the festival the street is dressed for
pub fn fest_today(day: i64) -> &'static str {
    let (y, m, _) = crate::civil_from_days_pub(day);
    for (key, a, b) in fest_calendar(y as i64) {
        if a <= day && day <= b {
            return key;
        }
    }
    match m {
        3..=5 => "spring",
        6..=8 => "summer",
        9..=11 => "autumn",
        _ => "winter",
    }
}

/// z23_fest_next(day): the next special festival after a day
pub fn fest_next(day: i64) -> Option<(&'static str, i64)> {
    let (y, _, _) = crate::civil_from_days_pub(day);
    let mut best: Option<(&'static str, i64)> = None;
    for yy in [y as i64, y as i64 + 1] {
        for (key, a, _) in fest_calendar(yy) {
            if a > day && best.is_none_or(|b| a < b.1) {
                best = Some((key, a));
            }
        }
    }
    best
}

/// z23_sky_mode(): day 07-16, dusk 16-19, else night
pub fn sky_mode(hour: u32) -> &'static str {
    if (7..16).contains(&hour) { "day" } else if (16..19).contains(&hour) { "dusk" } else { "night" }
}

// ---- the street ----------------------------------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq)]
pub struct Stall {
    pub key: String,
    pub x0: i64,
    pub x1: i64,
    pub sign_y: i64,
    pub top: i64,
}

/// z23_street's layout
#[derive(Clone, Debug, Default)]
pub struct StreetLay {
    pub gy: i64,
    pub sky: Vec<i64>,
    /// (x, y, colour)
    pub lanterns: Vec<(f64, f64, Rgb)>,
    pub twinkles: Vec<(f64, f64, String)>,
    pub stalls: Vec<Stall>,
    pub mid: i64,
    pub trees: Vec<i64>,
    pub lanes: (i64, i64),
}

fn shuffle<T>(r: &mut PyRandom, x: &mut [T]) {
    crate::bookgen::shuffle(r, x)
}

/// z23_street(aw, ah, ev, mode, seed): the painting at art size and its layout
pub fn street(aw: i64, ah: i64, ev: &str, mode: &str, seed: i64) -> (Ras, StreetLay) {
    let mut rnd = PyRandom::new((seed * 7919 + aw * 31 + ah) as u64);
    let mut r = Ras::new(aw, ah);
    let gy = ti(ah as f64 * 0.78);
    let night = mode == "night";
    let dusk = mode == "dusk";
    let f = fest(ev);
    let lcols: Vec<Rgb> = strs(&f[2]).iter().map(|c| rgb(c)).collect();
    let snowy = matches!(ev, "christmas" | "omisoka" | "winter");
    let (aws, ahs, gys) = (aw as f64, ah as f64, gy as f64);
    // ---- the sky
    let stops: [(f64, Rgb); 4] = if night {
        [(0.0, (10, 12, 34)), (0.55, (28, 24, 66)), (0.85, (72, 40, 78)), (1.0, (120, 62, 70))]
    } else if dusk {
        [(0.0, (42, 44, 104)), (0.45, (150, 80, 120)), (0.8, (244, 140, 96)), (1.0, (255, 190, 120))]
    } else {
        [(0.0, (96, 156, 222)), (0.6, (170, 200, 232)), (0.9, (246, 214, 176)), (1.0, (255, 226, 186))]
    };
    let (mut t, mut a, mut b) = (0.0, stops[0].1, stops[1].1);
    for y in 0..gy {
        let fy = y as f64 / 1.max(gy) as f64;
        for i in 0..stops.len() - 1 {
            if stops[i].0 <= fy && fy <= stops[i + 1].0 {
                t = (fy - stops[i].0) / 1e-6f64.max(stops[i + 1].0 - stops[i].0);
                (a, b) = (stops[i].1, stops[i + 1].1);
                break;
            }
        }
        for x in 0..aw {
            let tt = t + ((BAYER[(y & 3) as usize][(x & 3) as usize] as f64 + 0.5) / 16.0 - 0.5) * 0.18;
            let c = lerp3(a, b, tt);
            let i3 = ((y * aw + x) * 3) as usize;
            (r.b[i3], r.b[i3 + 1], r.b[i3 + 2]) = (c.0, c.1, c.2);
        }
    }
    if night || dusk {
        let nstar = ti(aws * gys / if night { 90.0 } else { 260.0 });
        if ev == "tanabata" && night {
            for _ in 0..aw * 3 {
                let x = rnd.random() * aws;
                let y = gys * 0.75 - x * 0.45 * gys / aws + rnd.gauss(0.0, gys * 0.07);
                let al = 0.35 + rnd.random() * 0.4;
                r.put(x, y, (230, 225, 255), al);
            }
        }
        for _ in 0..nstar {
            let x = rnd.randbelow(aw as u64) as f64;
            let y = rnd.randbelow(ti(gys * 0.6) as u64) as f64;
            let al = 0.3 + rnd.random() * 0.6;
            r.put(x, y, (255, 248, 230), al);
        }
    }
    if night || ev == "tsukimi" {
        let mr = 4.max(ti(ahs * if ev == "tsukimi" || ev == "halloween" { 0.12 } else { 0.06 }));
        let (mx, my) = (ti(aws * 0.84), ti(ahs * 0.16) + mr);
        let mcol: Rgb = if ev == "halloween" { (255, 170, 80) } else if ev == "tsukimi" { (255, 236, 180) } else { (250, 238, 205) };
        let (mxf, myf, mrf) = (mx as f64, my as f64, mr as f64);
        let mut rr = mr * 3;
        while rr > mr {
            r.disc(mxf, myf, rr as f64, mcol, 0.018, 1.0);
            rr -= 1;
        }
        r.disc(mxf, myf, mrf, mcol, 1.0, 1.0);
        r.disc(mxf - mrf * 0.3, myf - mrf * 0.25, mrf * 0.22, shade(mcol, 0.9), 1.0, 1.0);
        r.disc(mxf + mrf * 0.35, myf + mrf * 0.3, mrf * 0.16, shade(mcol, 0.9), 1.0, 1.0);
        if ev == "tsukimi" {
            for (dx, dy) in [(0, 0), (1, 0), (0, -1), (-1, -2), (-1, -3), (1, 1), (2, 1)] {
                r.put((mx + dx) as f64, (my + dy + 1) as f64, shade(mcol, 0.78), 1.0);
            }
        }
    } else if dusk {
        r.disc(ti(aws * 0.78) as f64, ti(gys * 0.78) as f64, 5.max(ti(ahs * 0.07)) as f64, (255, 214, 150), 0.95, 1.0);
    }
    // ---- far hills with a pagoda
    let far: Rgb = if night { (40, 34, 70) } else if dusk { (120, 70, 100) } else { (128, 150, 178) };
    let far2: Rgb = if night { (26, 24, 50) } else if dusk { (86, 50, 80) } else { (98, 120, 150) };
    let ph = rnd.random() * 6.0;
    for x in 0..aw {
        let xf = x as f64;
        let h1 = gys * (0.36 + 0.07 * (xf / (aws / 5.0) + ph).sin() + 0.03 * (xf / 9.0).sin());
        let h2 = gys * (0.24 + 0.05 * (xf / (aws / 3.3) + ph * 2.0).sin() + 0.02 * (xf / 5.0).sin());
        for y in ti(gys - h1)..gy {
            r.put(xf, y as f64, if (y as f64) < gys - h2 { far } else { far2 }, 1.0);
        }
    }
    let px = ti(aws * (0.18 + rnd.random() * 0.12));
    let pbase = ti(gys - gys * 0.33);
    let rc = |r: &mut Ras, x0: i64, y0: i64, x1: i64, y1: i64, c: Rgb| r.rect(x0 as f64, y0 as f64, x1 as f64, y1 as f64, c, 1.0);
    for k in 0..5 {
        let w = 13 - k * 2;
        let yk = pbase - k * 5;
        rc(&mut r, px - w, yk - 1, px + w + 1, yk + 1, far2);
        rc(&mut r, px - w + 3, yk + 1, px + w - 2, yk + 5, far2);
    }
    rc(&mut r, px, pbase - 30, px + 1, pbase - 24, far2);
    if night {
        for k in 0..4 {
            r.put(px as f64, (pbase - k * 5 + 3) as f64, (255, 190, 110), 0.8);
        }
    }
    // ---- the back row of machiya houses
    let mut sky_line = vec![gy; aw as usize];
    let lit: Rgb = if night || dusk { (255, 196, 110) } else { (70, 48, 36) };
    let (wood, wood2): (Rgb, Rgb) = if night { ((62, 38, 28), (44, 28, 22)) } else { ((96, 60, 40), (70, 44, 30)) };
    let plaster: Rgb = if night { (120, 104, 94) } else if dusk { (206, 182, 160) } else { (226, 214, 192) };
    let (roof, roof2): (Rgb, Rgb) = if night { ((44, 48, 62), (30, 32, 44)) } else { ((66, 72, 90), (48, 52, 66)) };
    let snow: Rgb = (236, 240, 250);
    let mut x = -rnd.randint(0, 20);
    let mut houses = vec![];
    while x < aw {
        let w = rnd.randint(46, 72);
        let hh = rnd.randint(ti(ahs * 0.36), ti(ahs * 0.46));
        let top = gy - hh;
        houses.push((x, w, top));
        rc(&mut r, x, top + 10, x + w, gy, wood);
        rc(&mut r, x + 2, top + 12, x + w - 2, top + 22, plaster);
        let mut sx = x + 5;
        while sx < x + w - 5 {
            if sx.div_euclid(3).rem_euclid(5) != 0 {
                rc(&mut r, sx, top + 14, sx + 1, top + 20, wood2);
            }
            sx += 3;
        }
        rc(&mut r, x - 2, top + 22, x + w + 2, top + 25, roof);
        rc(&mut r, x - 2, top + 25, x + w + 2, top + 26, roof2);
        rc(&mut r, x + 3, top + 28, x + w - 3, gy - 2, lit);
        let mut sx = x + 3;
        while sx < x + w - 3 {
            rc(&mut r, sx, top + 28, sx + 1, gy - 2, wood2);
            sx += 2;
        }
        if night || dusk {
            for k in 0..3 {
                r.rect((x + 3) as f64, (top + 28 + k) as f64, (x + w - 3) as f64, (top + 29 + k) as f64, (255, 220, 160), 0.15);
            }
        }
        if rnd.random() < 0.6 {
            let nc = *rnd.choice(&[(46u8, 64u8, 120u8), (150, 40, 40), (60, 90, 70), (110, 60, 120)]);
            let dx = x + rnd.randint(6, 7.max(w - 24));
            for k in 0..3 {
                rc(&mut r, dx + k * 6, top + 27, dx + k * 6 + 5, top + 37, nc);
            }
            r.disc((dx + 9) as f64, (top + 31) as f64, 2.0, (240, 232, 214), 1.0, 1.0);
        }
        let (xf, tf, wf) = (x as f64, top as f64, w as f64);
        r.poly(&[(xf - 4.0, tf + 11.0), (xf + wf + 4.0, tf + 11.0), (xf + wf - 3.0, tf), (xf + 3.0, tf)], roof, 1.0);
        let mut sx = x - 2;
        while sx < x + w + 3 {
            let sxf = sx as f64;
            r.line(sxf, tf + 10.0, sxf + if sxf > xf + wf / 2.0 { 2.0 } else { -1.0 }, tf + 2.0, roof2, 1.0, 1.0);
            sx += 3;
        }
        rc(&mut r, x + 2, top - 1, x + w - 2, top + 1, roof2);
        if snowy {
            rc(&mut r, x + 2, top - 2, x + w - 2, top, snow);
            rc(&mut r, x - 3, top + 10, x + w + 3, top + 11, snow);
            rc(&mut r, x - 2, top + 22, x + w + 2, top + 23, snow);
        }
        for sx in 0.max(x - 4)..aw.min(x + w + 4) {
            sky_line[sx as usize] = sky_line[sx as usize].min(top);
        }
        x += w + rnd.randint(0, 3);
    }
    // trees for hanami / spring (sakura) and autumn / tsukimi (momiji)
    let mut trees = vec![];
    if matches!(ev, "hanami" | "autumn" | "tsukimi" | "spring") {
        let cols: Vec<Rgb> = if ev == "hanami" || ev == "spring" {
            vec![(255, 190, 214), (255, 214, 230), (240, 150, 190)]
        } else {
            vec![(214, 70, 40), (240, 120, 40), (190, 40, 40), (250, 170, 60)]
        };
        let nt = 2.max(aw / 120);
        for k in 0..nt {
            let tx = ti((k as f64 + 0.5) * aws / nt as f64) + rnd.randint(-15, 15);
            trees.push(tx);
            let txf = tx as f64;
            rc(&mut r, tx - 1, gy - 32, tx + 2, gy, (70, 44, 34));
            r.line(txf, gys - 26.0, txf - 7.0, gys - 34.0, (70, 44, 34), 1.5, 1.0);
            r.line(txf + 1.0, gys - 28.0, txf + 8.0, gys - 36.0, (70, 44, 34), 1.5, 1.0);
            for _ in 0..70 {
                let ang = rnd.random() * 6.28;
                let rr = rnd.random().powf(0.6) * 14.0;
                let c = *rnd.choice(&cols);
                r.disc(txf + ang.cos() * rr * 1.3, gys - 40.0 + ang.sin() * rr * 0.75, 2.2, c, if night { 0.9 } else { 1.0 }, 1.0);
            }
        }
    }
    // ---- the yagura (or a Christmas tree) in the middle
    let cxm = aw / 2 + rnd.randint(-10, 10);
    let cf = cxm as f64;
    let mut lanterns: Vec<(f64, f64, Rgb)> = vec![];
    let mut twinkles: Vec<(f64, f64, String)> = vec![];
    let mut ytop = gy - 54;
    if ev == "christmas" {
        rc(&mut r, cxm - 2, gy - 10, cxm + 3, gy, (80, 50, 34));
        for k in 0..5 {
            let w = 22 - k * 4;
            let yk = gy - 10 - k * 9;
            let (wf, ykf) = (w as f64, yk as f64);
            r.poly(&[(cf - wf, ykf), (cf + wf + 1.0, ykf), (cf + 1.0, ykf - 14.0), (cf, ykf - 14.0)],
                   if k % 2 == 1 { (34, 110, 70) } else { (40, 128, 80) }, 1.0);
            rc(&mut r, cxm - w + 2, yk - 1, cxm + w - 1, yk, snow);
            for j in 0..6 {
                let c = rnd.choice(&["#ffd27a", "#ff5a5a", "#7fd8ff", "#7be07a"]).to_string();
                twinkles.push(((cxm - w + 3) as f64 + j as f64 * (2 * w - 6) as f64 / 5.0, (yk - 3 - (j % 2) * 4) as f64, c));
            }
        }
        r.disc(cf, (gy - 60) as f64, 3.0, (255, 220, 110), 1.0, 1.0);
        twinkles.push((cf, (gy - 60) as f64, "#fff2b0".into()));
        ytop = gy - 62;
    } else {
        let (red, white): (Rgb, Rgb) = ((200, 52, 40), (240, 232, 214));
        rc(&mut r, cxm - 20, gy - 30, cxm + 21, gy - 27, wood);
        for k in cxm - 19..cxm + 20 {
            rc(&mut r, k, gy - 27, k + 1, gy - 14, if (k - cxm).div_euclid(4).rem_euclid(2) == 1 { red } else { white });
        }
        for lx in [cxm - 18, cxm + 16] {
            rc(&mut r, lx, gy - 30, lx + 3, gy, wood2);
        }
        rc(&mut r, cxm - 14, gy - 46, cxm - 12, gy - 30, wood2);
        rc(&mut r, cxm + 12, gy - 46, cxm + 14, gy - 30, wood2);
        r.poly(&[(cf - 22.0, gys - 46.0), (cf + 23.0, gys - 46.0), (cf + 14.0, gys - 54.0), (cf - 13.0, gys - 54.0)], roof, 1.0);
        r.disc(cf, gys - 37.0, 5.0, (120, 60, 30), 1.0, 1.0);
        r.disc(cf, gys - 37.0, 3.5, (226, 200, 160), 1.0, 1.0);
        if ev == "newyear" || ev == "omisoka" {
            r.line(cf - 20.0, gys - 44.0, cf + 20.0, gys - 44.0, (230, 200, 120), 1.6, 1.0);
            let mut k = -15;
            while k < 16 {
                rc(&mut r, cxm + k, gy - 43, cxm + k + 2, gy - 38, (250, 250, 250));
                k += 10;
            }
        }
        if ev == "kodomo" {
            rc(&mut r, cxm + 30, gy - 74, cxm + 31, gy, (120, 90, 60));
            for (k, cc) in [(30u8, 30u8, 40u8), (210, 50, 50), (60, 110, 200)].into_iter().enumerate() {
                let k = k as i64;
                let yk = (gy - 70 + k * 9) as f64;
                let kf = k as f64;
                r.poly(&[(cf + 31.0, yk), (cf + 50.0 - kf * 2.0, yk + 1.0), (cf + 54.0 - kf * 2.0, yk + 4.0),
                         (cf + 50.0 - kf * 2.0, yk + 6.0), (cf + 31.0, yk + 6.0)], cc, 1.0);
                r.disc(cf + 34.0, yk + 3.0, 1.2, (250, 250, 250), 1.0, 1.0);
            }
        }
        if ev == "diwali" {
            for (rr, cc) in [(14.0, (255u8, 90u8, 160u8)), (11.0, (255, 210, 60)), (8.0, (80, 190, 255)), (5.0, (120, 220, 120)),
                             (2.0, (255, 140, 40))] {
                r.disc(cf, gys + (ahs - gys) * 0.5, rr, cc, 0.85, 0.3);
            }
        }
    }
    // ---- lantern ropes
    let rope_col: Rgb = if night { (30, 22, 20) } else { (60, 40, 30) };
    let rope = |r: &mut Ras, rnd: &mut PyRandom, lanterns: &mut Vec<(f64, f64, Rgb)>, x0: f64, y0: f64, x1: f64, y1: f64, sag: f64,
                every: i64| {
        let n = 2.max(ti((x1 - x0).abs()));
        let mut pts = Vec::with_capacity(n as usize + 1);
        for i in 0..=n {
            let t = i as f64 / n as f64;
            let xx = x0 + (x1 - x0) * t;
            let yy = y0 + (y1 - y0) * t + sag * 4.0 * t * (1.0 - t);
            pts.push((xx, yy));
            r.put(xx, yy, rope_col, 1.0);
        }
        let mut k = rnd.randint(2, every);
        while k < pts.len() as i64 - 2 {
            let (lx, ly) = pts[k as usize];
            let c = lcols[lanterns.len() % lcols.len()];
            lanterns.push((lx, ly + 1.0, c));
            k += every;
        }
    };
    let ya = (gy - 70 + rnd.randint(-4, 4)) as f64;
    rope(&mut r, &mut rnd, &mut lanterns, cf, (ytop + 1) as f64, -4.0, ya, 14.0, 11);
    let yb = (gy - 72 + rnd.randint(-4, 4)) as f64;
    rope(&mut r, &mut rnd, &mut lanterns, cf, (ytop + 1) as f64, aws + 4.0, yb, 14.0, 11);
    for &(hx, hw, htop) in &houses {
        if rnd.random() < 0.3 {
            rope(&mut r, &mut rnd, &mut lanterns, (hx + 4) as f64, (htop + 25) as f64, (hx + hw - 4) as f64, (htop + 25) as f64, 2.0, 12);
        }
    }
    for &(lx, ly, body) in &lanterns {
        if night || dusk {
            for rr in [6.0, 5.0, 4.0] {
                r.disc(lx, ly + 4.0, rr, body, 0.06, 1.0);
            }
        }
        r.rect(lx - 1.0, ly, lx + 2.0, ly + 1.0, (30, 24, 20), 1.0);
        r.rect(lx - 2.0, ly + 1.0, lx + 3.0, ly + 7.0, shade(body, if night { 0.85 } else { 1.0 }), 1.0);
        r.rect(lx - 2.0, ly + 3.0, lx + 3.0, ly + 4.0, shade(body, 0.7), 1.0);
        r.rect(lx - 1.0, ly + 7.0, lx + 2.0, ly + 8.0, (30, 24, 20), 1.0);
    }
    // ---- yatai stalls along the street
    let mut order = strs(&f[3]);
    shuffle(&mut rnd, &mut order);
    let n = 2.max((aw - 30).div_euclid(62));
    let slots: Vec<i64> = (0..n).map(|k| ti(15.0 + (k as f64 + 0.5) * (aw - 30) as f64 / n as f64)).filter(|s| (s - cxm).abs() > 34)
        .collect();
    let mut stalls = vec![];
    for (k, &sx) in slots.iter().enumerate() {
        let key = order[k % order.len()].clone();
        let st = stall(&key);
        let (x0, x1) = (sx - 22, sx + 22);
        let top = gy - 34;
        let aw_c: [Rgb; 2] = match k % 3 {
            0 => [(200, 52, 40), (240, 232, 214)],
            1 => [(46, 80, 150), (240, 232, 214)],
            _ => [(240, 150, 40), (240, 232, 214)],
        };
        rc(&mut r, x0, top, x0 + 2, gy, wood2);
        rc(&mut r, x1 - 2, top, x1, gy, wood2);
        rc(&mut r, x0 + 2, top + 8, x1 - 2, gy - 12, if night { (52, 36, 30) } else { (84, 60, 44) });
        let sxf = sx as f64;
        if night || dusk {
            r.rect((x0 + 3) as f64, (top + 8) as f64, (x1 - 3) as f64, (gy - 12) as f64, (255, 200, 120), 0.35);
            r.disc(sxf, (top + 10) as f64, 2.0, (255, 236, 180), 1.0, 1.0);
            for rr in [16.0, 12.0, 8.0] {
                r.disc(sxf, (gy - 4) as f64, rr, (255, 190, 110), 0.05, 0.35);
            }
        }
        let kc = *rnd.choice(&[(250u8, 220u8, 190u8), (230, 190, 150), (190, 140, 100)]);
        rc(&mut r, sx - 3, gy - 22, sx + 3, gy - 16, kc);
        rc(&mut r, sx - 4, gy - 24, sx + 4, gy - 21, (30, 24, 24));
        rc(&mut r, sx - 4, gy - 22, sx + 4, gy - 21, (240, 240, 240));
        let sc = *rnd.choice(&[(60u8, 80u8, 140u8), (150, 50, 40), (70, 110, 80)]);
        rc(&mut r, sx - 5, gy - 16, sx + 6, gy - 12, sc);
        for a in x0 - 1..x1 + 1 {
            let c = aw_c[(a - x0).div_euclid(4).rem_euclid(2) as usize];
            let m = (a - x0).rem_euclid(4);
            rc(&mut r, a, top + 1, a + 1, top + 7 + if m == 1 || m == 2 { 1 } else { 0 }, c);
        }
        rc(&mut r, x0 - 2, top - 1, x1 + 2, top + 1, wood2);
        rc(&mut r, x0 + 2, top - 9, x1 - 2, top - 1, if !night { (240, 228, 200) } else { (226, 206, 170) });
        rc(&mut r, x0 + 2, top - 10, x1 - 2, top - 9, wood2);
        rc(&mut r, x0 - 1, gy - 12, x1 + 1, gy - 6, if !night { (130, 84, 52) } else { (96, 62, 40) });
        rc(&mut r, x0 - 1, gy - 12, x1 + 1, gy - 11, if !night { (170, 116, 72) } else { (130, 86, 56) });
        let gc: Vec<Rgb> = strs(&st[3]).iter().map(|c| rgb(c)).collect();
        let kind = st[2].as_str().unwrap_or("");
        if kind == "kingyo" {
            rc(&mut r, x0 + 4, gy - 15, x1 - 4, gy - 12, gc[0]);
            for j in 0..7 {
                r.put((x0 + 6 + j * 5) as f64, (gy - 14 + j % 2) as f64, gc[1], 1.0);
            }
        } else if matches!(key.as_str(), "ringo" | "dango" | "tsukidango" | "wataame" | "takoyaki" | "pumpkin" | "mithai") {
            let stick = key == "ringo" || key == "wataame";
            for j in 0..6 {
                let bx = (x0 + 6 + j * 6) as f64;
                if stick {
                    r.line(bx, gys - 12.0, bx, gys - 17.0, (220, 200, 160), 1.0, 1.0);
                }
                r.disc(bx, gys - if stick { 18.0 } else { 14.0 }, if key == "wataame" { 2.4 } else { 1.8 }, gc[j as usize % gc.len()],
                       1.0, 1.0);
            }
        } else if key == "omen" {
            for j in 0..4 {
                let bx = x0 + 8 + j * 9;
                r.disc(bx as f64, (top + 13) as f64, 3.0, gc[0], 1.0, 1.0);
                for (dx, dy) in [(-1, 12), (1, 12), (-2, 10), (2, 10)] {
                    r.put((bx + dx) as f64, (top + dy) as f64, gc[1], 1.0);
                }
            }
        } else {
            for j in 0..7 {
                rc(&mut r, x0 + 5 + j * 5, gy - 15 - j % 2, x0 + 8 + j * 5, gy - 12, gc[j as usize % gc.len()]);
            }
        }
        if ev == "halloween" && k % 2 == 0 {
            r.disc((x1 + 3) as f64, (gy + 2) as f64, 3.0, (240, 120, 30), 1.0, 0.8);
            rc(&mut r, x1 + 2, gy - 1, x1 + 3, gy, (60, 40, 20));
            twinkles.push(((x1 + 3) as f64, (gy + 2) as f64, "#ffd060".into()));
        }
        if (ev == "newyear" || ev == "omisoka") && k % 2 == 1 {
            for (j, hh) in [14, 11, 9].into_iter().enumerate() {
                let j = j as i64;
                rc(&mut r, x1 + 1 + j * 2, gy - hh, x1 + 2 + j * 2, gy, (90, 160, 80));
            }
            rc(&mut r, x1, gy - 4, x1 + 8, gy, (180, 140, 80));
        }
        if ev == "diwali" {
            for j in 0..4 {
                twinkles.push(((x0 + 4 + j * 12) as f64, (gy + 1) as f64, "#ffcf5a".into()));
                rc(&mut r, x0 + 3 + j * 12, gy + 1, x0 + 6 + j * 12, gy + 3, (170, 80, 40));
            }
        }
        if ev == "tanabata" && k % 2 == 0 {
            rc(&mut r, x0 - 5, top - 30, x0 - 4, gy, (90, 150, 70));
            for j in 0..9 {
                let a0 = x0 - 10 + rnd.randint(0, 10);
                let a1 = x0 - 8 + rnd.randint(0, 10);
                let c = *rnd.choice(&[(255u8, 120u8, 160u8), (120, 200, 255), (255, 220, 90), (140, 230, 140)]);
                rc(&mut r, a0, top - 26 + j * 4, a1, top - 22 + j * 4, c);
            }
        }
        stalls.push(Stall { key, x0, x1, sign_y: top - 5, top });
    }
    if ev == "tsukimi" {
        for _ in 0..aw / 18 {
            let bx = rnd.randbelow(aw as u64) as i64;
            for j in 0..4 {
                r.line((bx + j) as f64, gys + 2.0, (bx + j * 2 - 2) as f64, gys - 8.0 - j as f64, (200, 180, 130), 1.0, 1.0);
            }
        }
    }
    // ---- the street
    let stone: Rgb = if night { (72, 66, 76) } else if dusk { (132, 112, 112) } else { (162, 150, 136) };
    let stone2 = shade(stone, 0.82);
    for y in gy..ah {
        let fy = (y - gy) as f64 / 1.max(ah - gy) as f64;
        let c = shade(stone, 1.0 - 0.25 * fy);
        for x in 0..aw {
            r.put(x as f64, y as f64, c, 1.0);
        }
        if (y - gy) % 6 == 5 {
            rc(&mut r, 0, y, aw, y + 1, stone2);
        }
    }
    let mut y = gy;
    while y < ah {
        let off = ((y - gy) / 6) * 7;
        let mut x = (-off).rem_euclid(14);
        while x < aw {
            rc(&mut r, x, y, x + 1, ah.min(y + 6), stone2);
            x += 14;
        }
        y += 6;
    }
    if snowy {
        for x in 0..aw {
            let top = gy + 3 + ti(2.0 * (x as f64 / 7.0).sin() + 1.0);
            for y in gy..top {
                r.put(x as f64, y as f64, (232, 236, 246), 1.0);
            }
        }
        for _ in 0..aw * 2 {
            let x = rnd.randbelow(aw as u64) as f64;
            let y = (gy + rnd.randbelow((ah - gy) as u64) as i64) as f64;
            r.put(x, y, snow, 0.6);
        }
    }
    if night || dusk {
        for s in &stalls {
            for rr in [20.0, 14.0] {
                r.disc((s.x0 + s.x1) as f64 / 2.0, (gy + 6) as f64, rr, (255, 180, 100), 0.05, 0.3);
            }
        }
    }
    let lanes = (gy + ti((ah - gy) as f64 * 0.30), gy + ti((ah - gy) as f64 * 0.78));
    (r, StreetLay { gy, sky: sky_line, lanterns, twinkles, stalls, mid: cxm, trees, lanes })
}

// ---- people --------------------------------------------------------------------------------------------------------

/// a person's look (z23_look's dict)
#[derive(Clone, Debug, PartialEq, Default)]
pub struct Look {
    pub h: String,
    pub s: String,
    pub y: (String, String),
    pub o: String,
    pub long: bool,
    pub bun: bool,
    pub kid: bool,
    pub acc: Option<String>,
    pub hat: Option<String>,
    pub ghost: bool,
    pub fox: bool,
}

impl Look {
    /// a cache key for the sprite (json.dumps(look, sort_keys=True))
    pub fn key(&self) -> String {
        format!("{:?}", self)
    }
    pub fn to_json(&self) -> Value {
        json!({"h": self.h, "s": self.s, "y": [self.y.0, self.y.1], "o": self.o, "long": self.long, "bun": self.bun,
               "kid": self.kid, "acc": self.acc, "hat": self.hat, "ghost": self.ghost, "fox": self.fox})
    }
}

fn pick_s(r: &mut PyRandom, k: &str) -> String {
    let v = strs(&sdata()[k]);
    r.choice(&v).clone()
}

pub fn yukata(i: usize) -> (String, String) {
    let y = &sdata()["YUKATA"];
    let n = y.as_array().unwrap().len();
    let p = &y[i % n];
    (p[0].as_str().unwrap().into(), p[1].as_str().unwrap().into())
}

pub fn yukata_n() -> usize {
    sdata()["YUKATA"].as_array().unwrap().len()
}

pub fn hair(i: usize) -> String {
    let h = strs(&sdata()["HAIR"]);
    h[i % h.len()].clone()
}

pub fn hair_n() -> usize {
    sdata()["HAIR"].as_array().unwrap().len()
}

/// z23_look(rnd, ev, fox)
pub fn look(r: &mut PyRandom, ev: &str, fox: bool) -> Look {
    let h = pick_s(r, "HAIR");
    let s = pick_s(r, "SKIN");
    let yi = r.randbelow(yukata_n() as u64) as usize;
    let y = yukata(yi);
    let o = pick_s(r, "OBI");
    let long = r.random() < 0.45;
    let bun = r.random() < 0.2;
    let kid = r.random() < 0.16;
    let accs = [None, None, Some("mask"), Some("apple"), Some("fan"), Some("fish"), Some("balloon")];
    let acc = r.choice(&accs).map(String::from);
    let mut l = Look { h, s, y, o, long, bun, kid, acc, hat: None, ghost: false, fox: false };
    if ev == "christmas" {
        if r.random() < 0.4 {
            l.hat = Some("santa".into());
        }
    } else if ev == "halloween" {
        let q = r.random();
        if q < 0.18 {
            l.ghost = true;
        } else if q < 0.45 {
            l.hat = Some("witch".into());
        }
    } else if ev == "setsubun" {
        if r.random() < 0.3 {
            l.hat = Some("horns".into());
        }
    } else if (ev == "summer" || ev == "obon") && r.random() < 0.15 {
        l.hat = Some("band".into());
    }
    if ev == "holi" {
        let a = r.choice(&["#f4f0e6", "#ffffff"]).to_string();
        let b = r.choice(&["#ff5fa2", "#ffd23a", "#4fc3ff", "#7be07a"]).to_string();
        l.y = (a, b);
    }
    if fox {
        l.h = "#e2843f".into();
        (l.fox, l.kid, l.ghost, l.long, l.bun) = (true, false, false, false, false);
    }
    l
}

/// z23_person_rows(look, frame): 13 x 26 palette rows
pub fn person_rows(look: &Look, frame: i32) -> Vec<String> {
    let base = [
        ".............", ".............", ".............", ".............", ".............",
        "....hhhhh....", "...hhhhhhh...", "...hffffffh..", "...ffkffkf...", "....fffff....", ".....fff.....",
        "....yywyy....", "...yyywyyy...", "..fyyyyyyyf..", "...ooooooo...", "...yyyyyyy...", "...yypyyyy...",
        "...yyyyypy...", "...yyyyyyy...", "....yyyyy....", "....yy.yy....", "....ff.ff....", "....gg.gg....",
        ".............", ".............", "............."];
    let mut rows: Vec<Vec<u8>> = base.iter().map(|r| r.as_bytes().to_vec()).collect();
    let set = |rows: &mut Vec<Vec<u8>>, r: usize, s: &str| rows[r] = s.as_bytes().to_vec();
    if frame == 1 {
        set(&mut rows, 19, "....yyyyy....");
        set(&mut rows, 20, "...yy...yy...");
        set(&mut rows, 21, "...ff...ff...");
        set(&mut rows, 22, "...gg...gg...");
    }
    if look.long {
        for r in [8, 9, 10, 11] {
            rows[r][3] = b'h';
            rows[r][9] = b'h';
        }
    }
    if look.bun {
        (rows[4][5], rows[4][6], rows[4][7]) = (b'h', b'h', b'h');
        rows[3][6] = b'h';
    }
    if look.fox {
        for (r, s) in [(3, "...h.....h..."), (4, "...hh...hh...")] {
            for (i, ch) in s.bytes().enumerate() {
                if ch == b'h' {
                    rows[r][i] = b'h';
                }
            }
        }
    }
    match look.acc.as_deref() {
        Some("mask") => {
            for (r, c, ch) in [(5, 9, b'm'), (5, 10, b'm'), (6, 9, b'm'), (6, 10, b'm'), (6, 11, b'm'), (7, 10, b'r'), (4, 9, b'm'),
                               (4, 11, b'm')] {
                rows[r][c] = ch;
            }
        }
        Some("apple") => {
            (rows[12][1], rows[11][1], rows[10][1], rows[9][1]) = (b'f', b'l', b'a', b'a');
            rows[9][0] = b'a';
        }
        Some("fan") => {
            for (r, c) in [(10, 11), (10, 12), (11, 11), (11, 12), (12, 11), (9, 12)] {
                rows[r][c] = b'u';
            }
            rows[13][10] = b'f';
        }
        Some("fish") => {
            (rows[14][11], rows[15][11], rows[15][12], rows[16][11], rows[16][12]) = (b'l', b'b', b'b', b'b', b'b');
            rows[16][12] = b'q';
        }
        Some("balloon") => {
            for (r, c) in [(1, 0), (1, 1), (0, 0), (0, 1), (2, 0), (2, 1)] {
                rows[r][c] = b'q';
            }
            for r in 3..13 {
                rows[r][1] = if r % 2 == 1 { b'l' } else { b'.' };
            }
            rows[13][1] = b'f';
        }
        _ => {}
    }
    match look.hat.as_deref() {
        Some("santa") => {
            for (r, s) in [(2, ".....rrr....."), (3, "....rrrrr...."), (4, "...wwwwwww..."), (1, "......rr....."), (0, ".......w.....")] {
                for (i, ch) in s.bytes().enumerate() {
                    if ch != b'.' {
                        rows[r][i] = if ch == b'r' { b'R' } else { b'W' };
                    }
                }
            }
        }
        Some("witch") => {
            for (r, s) in [(0, "......k......"), (1, ".....kk......"), (2, ".....kkk....."), (3, "....kkkk....."), (4, "..kkkkkkkkk..")] {
                for (i, ch) in s.bytes().enumerate() {
                    if ch != b'.' {
                        rows[r][i] = b'K';
                    }
                }
            }
        }
        Some("horns") => {
            (rows[4][4], rows[4][8], rows[3][4], rows[3][8]) = (b'Y', b'Y', b'Y', b'Y');
        }
        Some("band") => {
            for c in 3..10 {
                rows[6][c] = b'B';
            }
        }
        _ => {}
    }
    if look.ghost {
        let mut g = vec![
            ".............", ".............", ".............", ".............", ".............",
            "....WWWWW....", "...WWWWWWW...", "..WWWWWWWWW..", "..WWkWWWkWW..", "..WWWWWWWWW..", "..WWWWkWWWW..",
            "..WWWWWWWWW..", "..WWWWWWWWW..", "..WWWWWWWWW..", "..WWWWWWWWW..", "..WWWWWWWWW..", "..WWWWWWWWW..",
            "..WWWWWWWWW..", "..WWWWWWWWW..", "..WWWWWWWWW..", "..W.WWW.WW...", ".............", ".............",
            ".............", ".............", "............."];
        if frame != 0 {
            g[20] = "...WW.WWW.W..";
        }
        rows = g.iter().map(|r| r.as_bytes().to_vec()).collect();
    }
    if look.kid {
        let mut k: Vec<Vec<u8>> = vec![b".............".to_vec(); 4];
        k.extend(rows.iter().enumerate().filter(|(i, _)| ![12, 16, 17, 18].contains(i)).map(|(_, r)| r.clone()));
        k.truncate(26);
        rows = k;
    }
    rows.into_iter().map(|r| String::from_utf8(r).unwrap()).collect()
}

/// z23_person_pal(look)
pub fn person_pal(look: &Look) -> Vec<(u8, String)> {
    let mut p: Vec<(u8, String)> = vec![(b'h', look.h.clone()), (b'f', look.s.clone()), (b'k', "#1a1414".into()), (b'y', look.y.0.clone()),
                                        (b'w', "#f4f0e6".into()), (b'p', look.y.1.clone()), (b'o', look.o.clone())];
    for (k, v) in [(b'g', "#5a3a24"), (b'm', "#f6f2ea"), (b'r', "#d9472b"), (b'a', "#e0312a"), (b'l', "#d8c8a8"), (b'u', "#4f8fe0"),
                   (b'b', "#bfe4ff"), (b'q', "#ff7a2a"), (b'R', "#d93a3a"), (b'W', "#f6f4ee"), (b'K', "#2a1e36"), (b'Y', "#ffd23a"),
                   (b'B', "#f6f4ee")] {
        p.push((k, v.into()));
    }
    p
}

/// z23_sprite(look, frame, Z): the person at zoom Z (plain pixels, no smoothing); unknown letters are magenta
pub fn person_px(look: &Look, frame: i32) -> Vec<[u8; 4]> {
    let rows = person_rows(look, frame);
    let pal = person_pal(look);
    let mut out = Vec::with_capacity(13 * 26);
    for y in 0..26 {
        for x in 0..13 {
            let ch = rows.get(y).and_then(|r| r.as_bytes().get(x).copied()).unwrap_or(b'.');
            if ch == b'.' {
                out.push([0, 0, 0, 0]);
            } else {
                let c = pal.iter().find(|p| p.0 == ch).map(|p| rgb(&p.1)).unwrap_or((255, 0, 255));
                out.push([c.0, c.1, c.2, 255]);
            }
        }
    }
    out
}

// ---- the square's filters ------------------------------------------------------------------------------------------

fn bad_re() -> &'static regex::Regex {
    static R: OnceLock<regex::Regex> = OnceLock::new();
    R.get_or_init(|| {
        let bad: Vec<String> = strs(&sdata()["SQ_BAD"]).iter().map(|w| regex::escape(w)).collect();
        let stem = strs(&sdata()["SQ_STEM"]).join("|");
        regex::Regex::new(&format!(r"(?i)\b(?:{})(?:s|es|ed|ing|er|ers|y)?\b|\b(?:{})\w*", bad.join("|"), stem)).unwrap()
    })
}

fn contact_re() -> &'static regex::Regex {
    static R: OnceLock<regex::Regex> = OnceLock::new();
    R.get_or_init(|| {
        regex::Regex::new(concat!(r"(?i)(https?://\S+|www\.\S+|\b\S+\.(?:com|net|org|io|gg|me|ly|co|xyz|app)\b\S*|",
                                  r"\b[\w.+-]+@[\w-]+\.[\w.]+\b|\+?\d[\d\s().-]{7,}\d|\b(?:snap(?:chat)?|insta(?:gram)?|discord|",
                                  r"whatsapp|telegram|tiktok)\b[:\s@]*\S*)")).unwrap()
    })
}

pub const SQUARE_MAX: usize = 240;
pub const SQUARE_SLOW: f64 = 15.0;
pub const SQUARE_HIDE_AT: usize = 3;

/// square_clean(text): one line, no links / numbers / emails / handles, rude words starred out
pub fn square_clean(text: &str) -> String {
    let ws = text.split_whitespace().collect::<Vec<_>>().join(" ");
    let t: String = ws.chars().take(SQUARE_MAX).collect();
    let t = contact_re().replace_all(&t, "[removed]").to_string();
    bad_re().replace_all(&t, |c: &regex::Captures| {
        let m = c.get(0).unwrap().as_str();
        let mut cs = m.chars();
        let first = cs.next().map(String::from).unwrap_or_default();
        first + &"*".repeat(cs.count())
    }).to_string()
}

/// square_name_ok(name): 3 to 18 of A-Z a-z 0-9 _ - . space, nothing the filters would change
pub fn square_name_ok(name: &str) -> Option<String> {
    let n: String = name.chars().filter(|c| c.is_ascii_alphanumeric() || "_-. ".contains(*c)).collect();
    let n: String = n.trim().chars().take(18).collect();
    if n.chars().count() < 3 || square_clean(&n) != n {
        return None;
    }
    Some(n)
}

/// square_tag(uid): the first 4 hex digits of sha1("zenko-square/" + uid)
pub fn square_tag(uid: &str) -> String {
    sha1_hex(format!("zenko-square/{uid}").as_bytes())[..4].to_string()
}

pub fn sha1_hex(data: &[u8]) -> String {
    let mut h: [u32; 5] = [0x67452301, 0xEFCDAB89, 0x98BADCFE, 0x10325476, 0xC3D2E1F0];
    let mut msg = data.to_vec();
    let bits = (data.len() as u64) * 8;
    msg.push(0x80);
    while msg.len() % 64 != 56 {
        msg.push(0);
    }
    msg.extend_from_slice(&bits.to_be_bytes());
    for chunk in msg.chunks(64) {
        let mut w = [0u32; 80];
        for i in 0..16 {
            w[i] = u32::from_be_bytes([chunk[4 * i], chunk[4 * i + 1], chunk[4 * i + 2], chunk[4 * i + 3]]);
        }
        for i in 16..80 {
            w[i] = (w[i - 3] ^ w[i - 8] ^ w[i - 14] ^ w[i - 16]).rotate_left(1);
        }
        let (mut a, mut b, mut c, mut d, mut e) = (h[0], h[1], h[2], h[3], h[4]);
        for (i, wi) in w.iter().enumerate() {
            let (f, k) = match i {
                0..=19 => ((b & c) | (!b & d), 0x5A827999),
                20..=39 => (b ^ c ^ d, 0x6ED9EBA1),
                40..=59 => ((b & c) | (b & d) | (c & d), 0x8F1BBCDC),
                _ => (b ^ c ^ d, 0xCA62C1D6),
            };
            let t = a.rotate_left(5).wrapping_add(f).wrapping_add(e).wrapping_add(k).wrapping_add(*wi);
            (e, d, c, b, a) = (d, c, b.rotate_left(30), a, t);
        }
        for (x, v) in h.iter_mut().zip([a, b, c, d, e]) {
            *x = x.wrapping_add(v);
        }
    }
    h.iter().map(|v| format!("{v:08x}")).collect()
}
