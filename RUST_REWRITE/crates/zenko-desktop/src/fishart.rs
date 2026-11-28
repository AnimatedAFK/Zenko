//! The koi drawn as pixel art (zenko.py Zenko22 `fish_pattern`, `_fish_hw`, `fish_rgba`, `_fh`), bit for bit, and
//! the Zen garden's data (data/garden.json from tools/gen_garden_data.py).

use crate::gardenart::{Rgb, lerp3, shade};
use serde_json::Value;
use std::f64::consts::PI;
use std::sync::OnceLock;
use zenko_art::core::hypot;
use zenko_sound::mt::PyRandom;

pub fn gdata() -> &'static Value {
    static D: OnceLock<Value> = OnceLock::new();
    D.get_or_init(|| serde_json::from_str(include_str!("../data/garden.json")).unwrap())
}

/// one row of FISH: (id, kanji, name, rarity, body, pattern, cond, (lo, hi) cm, note)
pub fn fish_all() -> &'static [Value] {
    gdata()["FISH"].as_array().unwrap()
}

pub fn fish_by_id(fid: &str) -> Option<&'static Value> {
    fish_all().iter().find(|f| f[0] == fid)
}

/// `_fh(*a)`: a small stable hash in 0..1
pub fn fh(parts: &[String]) -> f64 {
    let mut h: u32 = 2166136261;
    for p in parts {
        for ch in p.chars() {
            h = (h ^ ch as u32).wrapping_mul(16777619);
        }
    }
    (h % 100003) as f64 / 100003.0
}

type Blob = (f64, f64, f64, f64, Rgb);

#[derive(Clone, Debug, Default)]
pub struct Pattern {
    pub base: Rgb,
    pub blobs: Vec<Blob>,
    pub net: f64,
    pub metal: f64,
    pub ginrin: f64,
    pub glow: Option<Rgb>,
    pub head: Option<Rgb>,
    pub spine: Option<Rgb>,
    pub sides: Option<Rgb>,
    pub long: bool,
    pub dots: Option<(Rgb, f64)>,
    pub crescent: bool,
    pub flame: bool,
    pub netcol: Option<Rgb>,
}

/// `fish_pattern(pid, seed)`
pub fn fish_pattern(pid: &str, seed: i64) -> Pattern {
    let mut rnd = PyRandom::new_str(&format!("{pid}/{seed}"));
    const RED: Rgb = (214, 52, 40);
    const WHITE: Rgb = (246, 243, 236);
    const BLACK: Rgb = (30, 30, 36);
    let mut p = Pattern { base: WHITE, ..Default::default() };
    fn red_patches(rnd: &mut PyRandom, b: &mut Vec<Blob>, n0: i64, n1: i64, col: Rgb) {
        for _ in 0..rnd.randint(n0, n1) {
            let u = rnd.uniform(0.3, 0.92);
            let v = rnd.uniform(-0.45, 0.45);
            let ru = rnd.uniform(0.07, 0.15);
            let rv = rnd.uniform(0.45, 1.1);
            b.push((u, v, ru, rv, col));
        }
        if rnd.random() < 0.7 {
            let u = rnd.uniform(0.82, 0.9);
            let v = rnd.uniform(-0.15, 0.15);
            let ru = rnd.uniform(0.06, 0.09);
            let rv = rnd.uniform(0.45, 0.8);
            b.push((u, v, ru, rv, col));
        }
    }
    fn spots(rnd: &mut PyRandom, b: &mut Vec<Blob>, n0: i64, n1: i64, col: Rgb, u0: f64, u1: f64) {
        for _ in 0..rnd.randint(n0, n1) {
            let r = rnd.uniform(0.025, 0.05);
            let u = rnd.uniform(u0, u1);
            let v = rnd.uniform(-0.75, 0.75);
            b.push((u, v, r, r * 6.0, col));
        }
    }
    // blobs with a fixed rv: (u range, v range, ru range, rv)
    fn blob4(rnd: &mut PyRandom, u: (f64, f64), v: (f64, f64), ru: (f64, f64), rv: (f64, f64)) -> (f64, f64, f64, f64) {
        let a = rnd.uniform(u.0, u.1);
        let b = rnd.uniform(v.0, v.1);
        let c = rnd.uniform(ru.0, ru.1);
        let d = rnd.uniform(rv.0, rv.1);
        (a, b, c, d)
    }
    let b = &mut p.blobs;
    match pid {
        "kohaku" | "ginrin" | "butterfly" => {
            red_patches(&mut rnd, b, 2, 4, RED);
            if pid == "ginrin" {
                p.ginrin = 0.9;
            }
            if pid == "butterfly" {
                p.long = true;
                let kind = *rnd.choice(&["kohaku", "ogon", "showa"]);
                if kind == "ogon" {
                    b.clear();
                    p.base = (236, 182, 52);
                    p.metal = 0.9;
                } else if kind == "showa" {
                    p.base = BLACK;
                    red_patches(&mut rnd, b, 2, 3, RED);
                    let u = rnd.uniform(0.4, 0.7);
                    let v = rnd.uniform(-0.3, 0.3);
                    b.push((u, v, 0.09, 0.8, WHITE));
                }
            }
        }
        "sanke" => {
            red_patches(&mut rnd, b, 2, 4, RED);
            spots(&mut rnd, b, 3, 6, BLACK, 0.25, 0.72);
        }
        "showa" => {
            p.base = BLACK;
            red_patches(&mut rnd, b, 2, 3, RED);
            for _ in 0..rnd.randint(1, 3) {
                let q = blob4(&mut rnd, (0.3, 0.75), (-0.5, 0.5), (0.06, 0.11), (0.4, 0.9));
                b.push((q.0, q.1, q.2, q.3, WHITE));
            }
        }
        "bekko" => spots(&mut rnd, b, 4, 8, BLACK, 0.25, 0.78),
        "utsuri" => {
            p.base = BLACK;
            for _ in 0..rnd.randint(2, 4) {
                let q = blob4(&mut rnd, (0.3, 0.88), (-0.5, 0.5), (0.07, 0.14), (0.5, 1.0));
                b.push((q.0, q.1, q.2, q.3, WHITE));
            }
        }
        "tancho" => b.push((0.845, 0.0, 0.062, 0.7, (220, 30, 34))),
        "higoi" => p.base = (226, 92, 40),
        "chagoi" => (p.base, p.net) = ((150, 104, 62), 0.5),
        "karashi" => (p.base, p.net) = ((222, 178, 48), 0.25),
        "magoi" => (p.base, p.net) = ((72, 74, 58), 0.45),
        "asagi" => {
            (p.base, p.net, p.head, p.sides) = ((118, 150, 182), 0.65, Some((204, 216, 226)), Some((226, 104, 48)));
        }
        "shusui" => {
            (p.base, p.head, p.sides, p.spine) = ((166, 196, 218), Some((214, 226, 234)), Some((226, 104, 48)), Some((40, 70, 130)));
        }
        "goshiki" => {
            (p.base, p.net) = ((200, 206, 216), 0.85);
            red_patches(&mut rnd, b, 2, 3, (200, 46, 46));
        }
        "ochiba" => {
            p.base = (150, 164, 182);
            for _ in 0..rnd.randint(2, 4) {
                let q = blob4(&mut rnd, (0.3, 0.85), (-0.4, 0.4), (0.07, 0.13), (0.5, 1.0));
                b.push((q.0, q.1, q.2, q.3, (176, 116, 62)));
            }
        }
        "ogon" => (p.base, p.metal) = ((236, 182, 52), 1.0),
        "platinum" => (p.base, p.metal) = ((226, 230, 236), 1.0),
        "kujaku" => {
            (p.base, p.metal, p.net) = ((232, 232, 230), 0.8, 0.55);
            red_patches(&mut rnd, b, 2, 3, (236, 122, 40));
        }
        "wakin" => {
            p.base = (240, 104, 30);
            if rnd.random() < 0.5 {
                let u = rnd.uniform(0.35, 0.7);
                let v = rnd.uniform(-0.3, 0.3);
                b.push((u, v, 0.12, 0.9, WHITE));
            }
        }
        "ryukin" => {
            (p.base, p.long) = ((232, 62, 40), true);
            for _ in 0..rnd.randint(1, 3) {
                let u = rnd.uniform(0.35, 0.85);
                let v = rnd.uniform(-0.5, 0.5);
                let ru = rnd.uniform(0.08, 0.14);
                b.push((u, v, ru, 0.9, WHITE));
            }
        }
        "medaka" => p.base = (236, 196, 120),
        "yuki" => {
            (p.base, p.metal, p.dots) = ((232, 240, 255), 0.7, Some(((255, 255, 255), 0.06)));
            for _ in 0..2 {
                let u = rnd.uniform(0.35, 0.8);
                let v = rnd.uniform(-0.3, 0.3);
                b.push((u, v, 0.12, 0.9, (190, 214, 246)));
            }
        }
        "ame" => {
            (p.base, p.metal, p.dots, p.spine) = ((118, 140, 170), 0.35, Some(((214, 230, 248), 0.07)), Some((60, 78, 110)));
        }
        "sakura" => {
            p.base = (250, 222, 232);
            for _ in 0..rnd.randint(5, 8) {
                let r = rnd.uniform(0.035, 0.055);
                let u = rnd.uniform(0.3, 0.9);
                let v = rnd.uniform(-0.6, 0.6);
                b.push((u, v, r, r * 7.0, (238, 132, 172)));
            }
        }
        "momiji" => {
            p.base = (232, 108, 44);
            for _ in 0..rnd.randint(3, 5) {
                let u = rnd.uniform(0.3, 0.85);
                let v = rnd.uniform(-0.5, 0.5);
                let ru = rnd.uniform(0.05, 0.08);
                b.push((u, v, ru, 0.8, (170, 32, 26)));
            }
        }
        "hotaru" => (p.base, p.dots, p.glow) = ((24, 32, 52), Some(((220, 255, 120), 0.08)), Some((190, 255, 110))),
        "tsuki" => {
            (p.base, p.metal, p.dots) = ((36, 40, 92), 0.4, Some(((220, 226, 255), 0.04)));
            p.crescent = true;
        }
        "kitsunebi" => (p.base, p.glow, p.flame) = ((52, 28, 86), Some((110, 200, 255)), true),
        "ryu" => {
            (p.base, p.metal, p.net, p.long) = ((232, 176, 46), 0.9, 0.7, true);
            p.netcol = Some((40, 150, 110));
            p.spine = Some((200, 40, 40));
        }
        "namazu" => {
            (p.base, p.net) = ((82, 78, 60), 0.0);
            for _ in 0..rnd.randint(6, 10) {
                let u = rnd.uniform(0.2, 0.9);
                let v = rnd.uniform(-0.8, 0.8);
                let ru = rnd.uniform(0.03, 0.06);
                b.push((u, v, ru, 1.2, (52, 50, 40)));
            }
        }
        "ryugu" => (p.base, p.metal, p.spine) = ((210, 220, 232), 0.8, Some((226, 40, 52))),
        _ => {}
    }
    p
}

/// `fish_glow(fid)`
pub fn fish_glow(fid: &str) -> Option<Rgb> {
    fish_by_id(fid).and_then(|f| fish_pattern(f[5].as_str()?, 0).glow)
}

/// `_fish_hw(body, u)`: half-width of the body at u (0 tail tip .. 1 nose)
pub fn fish_hw(body: &str, u: f64) -> f64 {
    if body == "oar" {
        if u < 0.02 {
            return 0.0;
        }
        return if u < 0.93 { 1.0 } else { 0.0f64.max(0.0f64.max(1.0 - ((u - 0.93) / 0.07).powf(2.0)).sqrt()) };
    }
    if body == "gold" {
        let a = 0.3;
        if u < a {
            return 0.0;
        }
        let t = (u - a) / (1.0 - a);
        return 0.0f64.max(1.0 - ((t - 0.48) / 0.52).powf(2.0)).sqrt() * (0.35 + 0.65 * 1.0f64.min(t * 3.0));
    }
    let a = 0.17;
    if u < a {
        return 0.0;
    }
    let t = (u - a) / (1.0 - a);
    if body == "catfish" {
        return if t < 0.8 {
            0.22 + 0.78 * (1.0f64.min(t / 0.8) * PI / 2.0).sin()
        } else {
            0.0f64.max(1.0 - ((t - 0.8) / 0.2).powf(1.6)).sqrt()
        };
    }
    if t < 0.62 {
        return 0.25 + 0.75 * (PI / 2.0 * t / 0.62).sin().powf(1.1);
    }
    let s = (t - 0.62) / 0.38;
    0.0f64.max(1.0 - s.powf(2.2)).sqrt()
}

fn shape(body: &str) -> (f64, f64) {
    match body {
        "koi" => (0.17, 0.2),
        "gold" => (0.22, 0.3),
        "medaka" => (0.11, 0.18),
        "catfish" => (0.16, 0.17),
        _ => (0.045, 0.04),
    }
}

fn mul(c: Rgb, f: f64) -> Rgb {
    let g = |v: u8| ((v as f64 * f) as i64).clamp(0, 255) as u8;
    (g(c.0), g(c.1), g(c.2))
}

/// `fish_rgba(fid, seed, L, angle, squash)`: (width, height, RGBA pixels), pointing right
pub fn fish_rgba(fid: &str, seed: i64, l: i64, angle: f64, squash: f64) -> (usize, usize, Vec<[u8; 4]>) {
    let f = fish_by_id(fid).or_else(|| fish_by_id("kohaku")).unwrap();
    let (body, pid) = (f[4].as_str().unwrap(), f[5].as_str().unwrap());
    let p = fish_pattern(pid, seed);
    let (wr, mut tail) = shape(body);
    let lf = l as f64;
    let w = 1.5f64.max(lf * wr);
    let long_ = p.long;
    if long_ {
        tail *= 1.45;
    }
    let pad = 2i64;
    let s = (lf * if body == "catfish" { 1.2 } else { 1.08 }) as i64 + 2 * pad;
    let hh = if angle != 0.0 || squash != 1.0 {
        4.max((s as f64 * squash).ceil() as i64 + 2)
    } else {
        (w * if long_ { 3.4 } else { 2.8 }) as i64 + 2 * pad
    };
    let (ca, sa) = (angle.cos(), angle.sin());
    let base = p.base;
    let small = l < 34;
    let tiny = l < 18;
    let sq = if squash == 0.0 { 1.0 } else { squash };
    let sid = seed.to_string();
    let mut out = vec![[0u8; 4]; (s * hh).max(0) as usize];
    let mut solid = vec![0u8; out.len()];
    for py in 0..hh {
        for px in 0..s {
            let dx = px as f64 + 0.5 - s as f64 / 2.0;
            let dy = (py as f64 + 0.5 - hh as f64 / 2.0) / sq;
            let lx = dx * ca + dy * sa;
            let ly = -dx * sa + dy * ca;
            let u = lx / lf + 0.5;
            let v = ly / w;
            let (mut c, mut a, mut kind): (Option<Rgb>, f64, u8) = (None, 0.0, 0);
            let mut hw = 0.0;
            let av = v.abs();
            if (0.0..=1.02).contains(&u) {
                hw = fish_hw(body, 1.0f64.min(u));
                if hw > 0.0 && av <= hw {
                    kind = 1;
                } else if u < tail + 0.02 {
                    let s_ = 0.0f64.max((tail + 0.02 - u) / (tail + 0.02));
                    let mut span = 0.25 + (if long_ { 1.5 } else { 1.05 }) * s_;
                    if body == "oar" {
                        span = 0.9;
                    }
                    let wav = if long_ { 0.12 * (u * 40.0 + v * 3.0).sin() } else { 0.0 };
                    if av <= span + wav && !(s_ > 0.6 && av < (s_ - 0.6) * if long_ { 0.9 } else { 1.3 }) {
                        kind = 2;
                    }
                } else if body != "oar" {
                    let (mut pu0, mut pu1) = if long_ { (0.5, 0.74) } else { (0.56, 0.71) };
                    if body == "gold" {
                        (pu0, pu1) = (0.55, 0.7);
                    }
                    if pu0 <= u && u <= pu1 {
                        let k = (pu1 - u) / (pu1 - pu0);
                        let ext = (if long_ { 1.25 } else { 0.72 }) * (PI * k).sin().powf(0.8);
                        if hw < av && av <= hw + ext {
                            kind = 2;
                        }
                    }
                    if (0.37..=0.45).contains(&u) {
                        let k = (0.45 - u) / 0.08;
                        if hw < av && av <= hw + 0.34 * (PI * k).sin() {
                            kind = 2;
                        }
                    }
                }
            }
            if kind == 1 {
                let mut cc = base;
                if let Some(hd) = p.head
                    && u > 0.8
                {
                    cc = lerp3(cc, hd, 1.0f64.min((u - 0.8) / 0.06));
                }
                if let Some(sd) = p.sides
                    && av / 0.01f64.max(hw) > 0.62
                    && u > 0.25
                {
                    cc = lerp3(cc, sd, 1.0f64.min((av / hw - 0.62) * 3.2));
                }
                for &(bu, bv, ru, rv, col) in &p.blobs {
                    let (du, dv) = ((u - bu) / ru, (v - bv) / rv);
                    let wob = 1.0 + 0.16 * (dv.atan2(du) * 3.0 + bu * 17.0).sin();
                    if du * du + dv * dv < wob * wob {
                        cc = col;
                    }
                }
                if p.crescent && 0.8 < u && u < 0.95 {
                    let d1 = hypot((u - 0.87) / 0.06, v / 0.6);
                    let d2 = hypot((u - 0.89) / 0.06, (v - 0.12) / 0.6);
                    if d1 < 1.0 && d2 >= 1.0 {
                        cc = (236, 238, 250);
                    }
                }
                if p.flame && u > 0.2 {
                    let fl = v.abs() / 0.01f64.max(hw);
                    let tongue = 0.55 + 0.3 * (u * 34.0 + if v > 0.0 { 1.0 } else { 0.0 }).sin() + 0.1 * (u * 71.0).sin();
                    if fl > tongue {
                        cc = lerp3((70, 150, 255), (190, 240, 255), (fl - tongue) * 2.0);
                    }
                }
                if let Some(sp) = p.spine
                    && av < (if pid == "ryu" { 0.08 } else { 0.13 })
                    && 0.3 < u
                    && u < 0.93
                {
                    if pid == "shusui" {
                        if ((u * 30.0) as i64).rem_euclid(2) == 0 {
                            cc = sp;
                        }
                    } else {
                        cc = sp;
                    }
                }
                if p.net != 0.0 && !tiny && u < 0.8 {
                    let cu = u * if !small { 12.0 } else { 9.0 };
                    let cvv = v * (if !small { 1.9 } else { 1.3 }) + if (cu as i64).rem_euclid(2) == 1 { 0.5 } else { 0.0 };
                    let (fu, fv) = (cu - (cu as i64) as f64, cvv - cvv.floor());
                    let xe = 0.18 + 0.6 * (2.0 * fv - 1.0).powf(2.0);
                    if xe - 0.3 < fu && fu < xe {
                        let to = p.netcol.unwrap_or_else(|| shade(cc, 0.55));
                        cc = lerp3(cc, to, p.net * if p.netcol.is_some() { 0.75 } else { 0.6 });
                    }
                }
                if let Some((dc, dp)) = p.dots
                    && fh(&[fid.to_string(), sid.clone(), ((u * 60.0) as i64).to_string(), ((v * 6.0) as i64).to_string()]) < dp
                {
                    cc = dc;
                }
                if p.ginrin != 0.0 && !tiny {
                    let cu = u * 22.0;
                    let cvv = v * 3.2 + 0.5 * ((u * 22.0) as i64).rem_euclid(2) as f64;
                    if fh(&[sid.clone(), (cu as i64).to_string(), (cvv as i64).to_string()]) < 0.4
                        && (cu - (cu as i64) as f64 - 0.5).abs() < 0.2
                        && (cvv - cvv.floor() - 0.5).abs() < 0.22
                    {
                        cc = lerp3(cc, (255, 255, 255), 0.75);
                    }
                }
                let rel = av / 0.01f64.max(hw);
                let mut lit = 1.0 - 0.3 * rel.powf(3.0) + 0.1 * 0.0f64.max(1.0 - (v + 0.35).abs() / 0.35);
                if p.metal != 0.0 {
                    lit += p.metal * (0.14 * 0.0f64.max((u * 13.0 + v * 2.5).sin()) + 0.16 * 0.0f64.max(1.0 - (v + 0.3).abs() / 0.22));
                }
                cc = mul(cc, lit);
                if (v.abs() - 0.74 * fish_hw(body, 0.9)).abs() < (if small { 0.2 } else { 0.11 })
                    && (u - 0.9).abs() < (if l > 50 { 0.012 } else { 0.03 })
                {
                    cc = (16, 16, 22);
                }
                if body == "catfish" && u > 0.97 {
                    cc = shade(cc, 0.6);
                }
                c = Some(cc);
                a = 1.0;
            } else if kind == 2 {
                let mut fb = lerp3(base, (255, 255, 255), 0.25);
                if let Some(g) = p.glow {
                    fb = lerp3(fb, g, 0.45);
                }
                let ray = 0.82 + 0.18 * if ((u * 50.0 + v * 4.0) as i64).rem_euclid(3) != 0 { 1.0 } else { 0.0 };
                c = Some(mul(fb, ray));
                a = if !long_ { 0.78 } else { 0.7 };
            } else if (body == "koi" || body == "catfish") && !small && 0.985 < u
                && u < (if body == "catfish" { 1.07 } else { 1.03 })
            {
                let bv = if body == "koi" { 0.32 } else { 0.42 };
                let curl = (u - 0.985) * if body == "catfish" { 9.0 } else { 3.0 };
                if (v.abs() - bv - curl).abs() < 0.09 {
                    c = Some(shade(base, 0.7));
                    a = 1.0;
                    kind = 3;
                }
            }
            let i = (py * s + px) as usize;
            if let Some(cc) = c {
                out[i] = [cc.0, cc.1, cc.2, (255.0 * a) as u8];
                solid[i] = kind;
            }
        }
    }
    let (h2, w2) = (hh, s);
    let at = |x: i64, y: i64| solid[(y * w2 + x) as usize];
    let mut edits = Vec::new();
    for y in 0..h2 {
        for x in 0..w2 {
            let k = at(x, y);
            if k == 1 || k == 2 {
                let edge = x == 0 || at(x - 1, y) == 0 || x == w2 - 1 || at(x + 1, y) == 0 || y == 0 || at(x, y - 1) == 0
                    || y == h2 - 1 || at(x, y + 1) == 0;
                if edge {
                    edits.push(((y * w2 + x) as usize, if k == 1 { 0.55 } else { 0.72 }));
                }
            }
        }
    }
    for (i, f_) in edits {
        let [r, g, b, a] = out[i];
        out[i] = [(r as f64 * f_) as u8, (g as f64 * f_) as u8, (b as f64 * f_) as u8, a.max(230)];
    }
    (s as usize, hh as usize, out)
}
