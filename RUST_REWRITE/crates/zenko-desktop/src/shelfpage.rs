//! The Study books page's shelf (zenko.py ZenkoBooks.shelf_build / shelf_layout / shelf_cabinet / shelf_draw /
//! shelf_scroll / shelf_scroll_text / book_labels / shelf_hover / shelf_tick / shelf_key, and the pixel art it paints:
//! shelf_sky, book_art, petal_soft, washi_art): one night canvas with a lacquered cabinet holding the nine volumes,
//! lanterns and a shimenawa, petals drifting behind, the hovered book lifting and glowing, and a hanging scroll that
//! says what it is.

use crate::ema::fmt_hm;
use crate::hubwin::canvas_of;
use crate::rempage::{Art, glow_art, hex_rgb, lantern_art, px, px_photo, rgb_mix};
use crate::session::clip;
use crate::tkcanvas::{Canvas, Photo};
use crate::ui::{Fill, Font, Fonts, Pack, Ui};
use std::collections::HashMap;
use std::f64::consts::PI;
use std::rc::Rc;
use zenko_sound::mt::PyRandom;

type C3 = (f64, f64, f64);

/// BOOKS: key, kanji, reading, name (the colour is book_meta's: a subject's own, else the book's)
pub const BOOKS: [(&str, &str, &str, &str, &str); 9] = [
    ("bio", "\u{751f}\u{7269}", "seibutsu", "Biology HL", "#46b37a"),
    ("chem", "\u{5316}\u{5b66}", "kagaku", "Chemistry HL", "#4f8fe0"),
    ("math", "\u{6570}\u{5b66}", "suugaku", "Math AA SL", "#e0a33a"),
    ("french", "\u{4ecf}\u{8a9e}", "futsugo", "French ab initio SL", "#d65f8f"),
    ("psych", "\u{5fc3}\u{7406}", "shinri", "Psychology SL", "#a46ad8"),
    ("english", "\u{82f1}\u{8a9e}", "eigo", "English Lang & Lit HL", "#e05a4a"),
    ("diary", "\u{65e5}\u{8a18}", "nikki", "Study diary", "#c9a24a"),
    ("grades", "\u{6210}\u{7e3e}", "seiseki", "Grades", "#b0122b"),
    ("bible", "\u{8056}\u{66f8}", "seisho", "Bible", "#6b2a1e"),
];

pub const SUBJECTS: [&str; 6] = ["bio", "chem", "math", "french", "psych", "english"];

/// BOOK_PRACTICE
pub fn practice(key: &str) -> &'static str {
    match key {
        "bio" => "\u{4eca}\u{65e5}\u{306e}\u{8c46}\u{77e5}\u{8b58} \u{b7} Syllabus fact",
        "chem" => "\u{8a08}\u{7b97}\u{9053}\u{5834} \u{b7} Calculation dojo",
        "math" => "\u{554f}\u{984c}\u{9053}\u{5834} \u{b7} Problem dojo",
        "french" => "\u{5358}\u{8a9e}\u{7df4}\u{7fd2} \u{b7} Vocab drill",
        "psych" => "\u{6982}\u{5ff5}\u{30ec}\u{30f3}\u{30ba} \u{b7} Concept lens",
        "english" => "\u{66f8}\u{304d}\u{65b9}\u{306e}\u{5dfb}\u{7269} \u{b7} Answer-writing scroll",
        "diary" => "\u{65e9}\u{5bdd} \u{b7} Sleep and rest guard",
        "grades" => "\u{6210}\u{7e3e}\u{8868} \u{b7} Add a result",
        "bible" => "\u{539f}\u{8a9e} \u{b7} Hebrew, Aramaic and Greek, word by word",
        _ => "",
    }
}

pub fn book(key: &str) -> Option<&'static (&'static str, &'static str, &'static str, &'static str, &'static str)> {
    BOOKS.iter().find(|b| b.0 == key)
}

/// _h01(x, y, s): a cheap repeatable hash -> 0..1
pub fn h01(x: i64, y: i64, s: i64) -> f64 {
    let m = 0xFFFF_FFFFu64;
    let n = (x as u64).wrapping_mul(374761393).wrapping_add((y as u64).wrapping_mul(668265263))
        .wrapping_add((s as u64).wrapping_mul(2147483647)) & m;
    let n = ((n ^ (n >> 13)).wrapping_mul(1274126177)) & m;
    ((n ^ (n >> 16)) & 0xFFFF) as f64 / 65535.0
}

fn c3(c: (i64, i64, i64)) -> C3 {
    (c.0 as f64, c.1 as f64, c.2 as f64)
}

/// glow_art with a fractional radius (the hovered book's halo)
pub fn glow_art_f(r: f64, col: (i64, i64, i64), strength: f64) -> Art {
    Box::new(move |x, y| {
        let d = ((x as f64 + 0.5 - r).powi(2) + (y as f64 + 0.5 - r).powi(2)).sqrt() / r;
        if d >= 1.0 {
            return None;
        }
        let a = (255.0 * strength * (1.0 - d).powi(2)) as i64;
        if a > 2 { Some([col.0 as u8, col.1 as u8, col.2 as u8, a as u8]) } else { None }
    })
}

/// shelf_sky(w, h): indigo sky, stars, the moon, misty mountains with a little torii, a wooden floor
pub fn shelf_sky(w: i32, h: i32) -> Art {
    let (wf, hf) = (w as f64, h as f64);
    let (top, mid, low) = ((9.0, 12.0, 34.0), (27.0, 25.0, 72.0), (58.0, 34.0, 78.0));
    let moon = (wf * 0.86, hf * 0.17, 6.0f64.max(hf * 0.075));
    let floor = (hf * 0.86) as i32;
    let ridge = |x: f64, base: f64, amp: f64, s: f64| {
        base + amp * (0.55 * (x * 0.045 + s).sin() + 0.3 * (x * 0.11 + 2.0 * s).sin() + 0.15 * (x * 0.27 + s).sin())
    };
    let tx = (wf * 0.16) as i32;
    Box::new(move |x, y| {
        let (xf, yf) = (x as f64, y as f64);
        if y >= floor {
            let plank = (y - floor) / 3;
            let g = 0.5 + 0.5 * h01((x / 9 + plank * 7) as i64, plank as i64, 3);
            let mut c = rgb_mix((44.0, 26.0, 20.0), (78.0, 48.0, 32.0),
                                g * 0.6 + if (y - floor) % 3 == 0 { 0.25 } else { 0.0 });
            if (x + plank * 5) % 23 == 0 {
                c = (34.0, 20.0, 16.0);
            }
            return px(c);
        }
        let t = yf / floor as f64;
        let mut c = if t < 0.6 { rgb_mix(top, mid, t / 0.6) } else { rgb_mix(mid, low, (t - 0.6) / 0.4) };
        let (dx, dy) = (xf - moon.0, yf - moon.1);
        let d = (dx * dx + dy * dy).sqrt();
        if d < moon.2 {
            let cr = if h01((xf / 2.0) as i64, (yf / 2.0) as i64, 5) > 0.82 { 0.10 } else { 0.0 };
            return px(rgb_mix((255.0, 244.0, 214.0), (214.0, 200.0, 168.0), cr + 0.25 * (dx + dy) / (2.0 * moon.2) + 0.1));
        }
        if d < moon.2 * 4.0 {
            c = rgb_mix(c, (120.0, 110.0, 170.0), 0.35 * (1.0 - (d - moon.2) / (moon.2 * 3.0)).powi(2));
        }
        if t < 0.62 {
            let r = h01(x as i64, y as i64, 11);
            if r > 0.9965 {
                return px(if r > 0.999 { (255.0, 250.0, 220.0) } else { (170.0, 180.0, 230.0) });
            }
        }
        if yf > ridge(xf, hf * 0.60, hf * 0.06, 1.3) {
            c = rgb_mix(c, (40.0, 34.0, 86.0), 0.75);
        }
        if yf > ridge(xf, hf * 0.70, hf * 0.05, 4.1) {
            c = rgb_mix(c, (26.0, 20.0, 52.0), 0.85);
            let ry = ridge(tx as f64, hf * 0.70, hf * 0.05, 4.1);
            let ax = (x - tx).abs();
            if ax <= 5 && ry - 8.0 <= yf && yf <= ry
                && (y == (ry - 8.0) as i32 || y == (ry - 7.0) as i32 || (y == (ry - 5.0) as i32 && ax <= 4) || ax == 3 || ax == 4) {
                return px((196.0, 44.0, 58.0));
            }
        }
        if t > 0.8 {
            c = rgb_mix(c, (92.0, 70.0, 120.0), (t - 0.8) / 0.2 * 0.5);
        }
        px(c)
    })
}

const MASK: [&str; 11] = ["K.........K", "KK.......KK", "KwK.....KwK", "KwwKKKKKwwK", "KwwwwwwwwwK", "KwrwwwwwrwK",
                          "KwKKwwwKKwK", ".KwwwwwwwK.", "..KwwrwwK..", "...KwwwK...", "....KKK...."];

/// book_art(col, cw, ch, hover): washi gradient with fibres, seigaiha waves, gold foil frame and corner guards, stab
/// binding, a golden kitsune mask crest, and the page block on the side and bottom
pub fn book_art(col: &str, cw: i32, ch: i32, hover: bool) -> Art {
    let base = c3(hex_rgb(col));
    let dark = rgb_mix(base, (0.0, 0.0, 0.0), 0.55);
    let deep = rgb_mix(base, (0.0, 0.0, 0.0), 0.30);
    let light = rgb_mix(base, (255.0, 255.0, 255.0), if hover { 0.22 } else { 0.12 });
    let (gold, gold_l, gold_d) = ((212.0, 175.0, 55.0), (255.0, 226.0, 130.0), (140.0, 104.0, 28.0));
    let (cream, line) = ((244.0, 234.0, 210.0), (214.0, 198.0, 160.0));
    let pw = 3;
    let (cx1, cy1) = (cw - pw, ch - pw);
    let sp = 5.max(cw / 10);
    let mx0 = sp + ((cx1 as f64 * 0.60 - sp as f64) / 2.0) as i32 - 5;
    let my0 = (ch as f64 * 0.15) as i32;
    let holes: Vec<i32> = [0.12, 0.37, 0.62, 0.87].iter().map(|k| (cy1 as f64 * k) as i32).collect();
    let r_w = 3.0f64.max(cw as f64 / 14.0);
    Box::new(move |x, y| {
        if x >= cx1 || y >= cy1 {
            if x < pw || y < pw || (x >= cx1 && y >= cy1 && (x - cx1) + (y - cy1) > pw) {
                return None;
            }
            if x >= cx1 && y < cy1 {
                return px(if (y + x) % 3 == 0 { line } else { cream });
            }
            if y >= cy1 && x < cx1 {
                return px(if (x + y) % 3 == 0 { line } else { cream });
            }
            return px(line);
        }
        if x == 0 || y == 0 || x == cx1 - 1 || y == cy1 - 1 {
            return px(dark);
        }
        if x < sp {
            let c = rgb_mix(deep, dark, 0.35 + 0.25 * h01(x as i64, (y / 3) as i64, 2));
            let hx = sp / 2;
            if holes.iter().any(|&hy| (y - hy).abs() <= 0) && x <= hx {
                return px((238.0, 226.0, 196.0));
            }
            if x == hx && holes[0] <= y && y <= holes[3] {
                return px(if y % 2 == 0 { (238.0, 226.0, 196.0) } else { (200.0, 186.0, 150.0) });
            }
            if x == sp - 1 {
                return px(rgb_mix(dark, (0.0, 0.0, 0.0), 0.2));
            }
            return px(c);
        }
        let (xf, yf) = (x as f64, y as f64);
        let t = xf / cx1 as f64 * 0.45 + yf / cy1 as f64 * 0.55;
        let mut c = rgb_mix(light, deep, t);
        let fib = h01((x / 3) as i64, y as i64, 7);
        if fib > 0.93 {
            c = rgb_mix(c, (255.0, 255.0, 255.0), 0.10);
        } else if fib < 0.05 {
            c = rgb_mix(c, (0.0, 0.0, 0.0), 0.08);
        }
        if yf > cy1 as f64 * 0.60 {
            let row = ((cy1 as f64 - yf) / (r_w * 0.8)) as i64;
            let ox = if row % 2 != 0 { r_w } else { 0.0 };
            let cxw = ((xf - sp as f64 + ox) / (2.0 * r_w)).round_ties_even() * 2.0 * r_w - ox + sp as f64;
            let cyw = cy1 as f64 - row as f64 * r_w * 0.8;
            let d = ((xf - cxw).powi(2) + (yf - cyw).powi(2)).sqrt();
            if yf <= cyw && [1.0, 0.62, 0.28].iter().any(|k| (d - r_w * k).abs() < 0.55) {
                c = rgb_mix(c, (255.0, 240.0, 200.0), 0.22);
            }
        }
        let (l, r, t_, b) = (sp + 2, cx1 - 3, 2, cy1 - 3);
        let on_outer = ((x == l || x == r) && t_ <= y && y <= b) || ((y == t_ || y == b) && l <= x && x <= r);
        let on_inner = ((x == l + 2 || x == r - 2) && t_ + 2 <= y && y <= b - 2)
            || ((y == t_ + 2 || y == b - 2) && l + 2 <= x && x <= r - 2);
        if on_outer {
            return px(if (x + y) % 7 == 0 { gold_l } else { gold });
        }
        if on_inner {
            return px(gold_d);
        }
        for (ccx, ccy, sx, sy) in [(r, t_, -1, 1), (r, b, -1, -1)] {
            let (ddx, ddy) = ((x - ccx) * sx, (y - ccy) * sy);
            if 0 <= ddx && 0 <= ddy && ddx + ddy <= 5 {
                return px(if ddx + ddy <= 1 { gold_l } else if ddx + ddy < 5 { gold } else { gold_d });
            }
        }
        let (mxr, myr) = (x - mx0, y - my0);
        if (0..MASK.len() as i32).contains(&myr) && (0..MASK[0].len() as i32).contains(&mxr) {
            match MASK[myr as usize].as_bytes()[mxr as usize] {
                b'K' => return px(gold_d),
                b'w' => return px(if (mxr + myr) % 5 == 0 { gold_l } else { gold }),
                b'r' => return px((200.0, 30.0, 50.0)),
                _ => {}
            }
        }
        px(c)
    })
}

/// petal_soft(r, alpha): a blurred sakura petal
pub fn petal_soft(r: f64, alpha: f64) -> Art {
    Box::new(move |x, y| {
        let dx = (x as f64 + 0.5 - r) / r;
        let dy = (y as f64 + 0.5 - r) / (r * 0.75);
        let d = (dx * dx + dy * dy).sqrt();
        if d >= 1.0 {
            return None;
        }
        let a = (255.0 * alpha * (1.0 - d).powf(1.6)) as i64;
        if a > 3 { Some([246, 178, 200, a as u8]) } else { None }
    })
}

/// washi_art(w, h): warm washi paper for the hanging scroll
pub fn washi_art(w: i32, h: i32) -> Art {
    Box::new(move |x, y| {
        let mut c = (238.0, 226.0, 198.0);
        let e = x.min(w - 1 - x).min(y).min(h - 1 - y);
        if e < 3 {
            c = rgb_mix(c, (200.0, 182.0, 146.0), (3 - e) as f64 / 4.0);
        }
        let r = h01((x / 2) as i64, y as i64, 21);
        if r > 0.92 {
            c = rgb_mix(c, (255.0, 250.0, 236.0), 0.6);
        } else if r < 0.06 {
            c = rgb_mix(c, (196.0, 178.0, 140.0), 0.35);
        }
        px(c)
    })
}

/// shelf_cabinet(w, h, rows_y, bh): pixel tansu (w / h in 2x pixels)
pub fn shelf_cabinet(w: i32, h: i32, rows_y: Vec<i32>, bh: i32) -> Art {
    let (wood_l, wood, wood_d) = ((128.0, 78.0, 46.0), (92.0, 52.0, 30.0), (52.0, 28.0, 18.0));
    let (back, back_d) = ((30.0, 20.0, 28.0), (22.0, 14.0, 20.0));
    let (gold, gold_l) = ((196.0, 160.0, 72.0), (236.0, 208.0, 132.0));
    let planks: Vec<(i32, i32)> = rows_y.iter().map(|y| ((y + bh).div_euclid(2), (y + bh).div_euclid(2) + 5)).collect();
    let fr = 4;
    let grain = move |x: i32, y: i32, base: C3| {
        let (xf, yf) = (x as f64, y as f64);
        let g = 0.5 + 0.5 * (yf * 0.9 + 3.0 * (xf * 0.07 + yf * 0.05).sin()).sin() * 0.5 + 0.15 * h01((x / 4) as i64, y as i64, 9);
        rgb_mix(base, wood_l, g * 0.4)
    };
    Box::new(move |x, y| {
        let edge = x < fr || x >= w - fr || y < fr || y >= h - fr;
        for &(p1, p2) in &planks {
            if p1 <= y && y < p2 && !edge {
                if y == p1 {
                    return px(wood_l);
                }
                if y == p2 - 1 {
                    return px(wood_d);
                }
                return px(grain(x, y * 3, wood));
            }
        }
        if edge {
            let side = x < fr || x >= w - fr;
            let c = grain(if side { y } else { x }, if side { x } else { y }, wood);
            if x == 0 || x == w - 1 || y == 0 || y == h - 1 {
                return px(wood_d);
            }
            for (cx, cy) in [(0, 0), (w - 1, 0), (0, h - 1), (w - 1, h - 1)] {
                if (x - cx).abs() < 7 && (y - cy).abs() < 7 {
                    return px(if (x + y) % 6 == 0 { gold_l } else { gold });
                }
            }
            return px(c);
        }
        let mut c = if (x / 16) % 2 == 0 { back } else { rgb_mix(back, back_d, 0.5) };
        if x % 16 == 0 {
            c = back_d;
        }
        for &(_, p2) in planks.iter().chain([(fr, fr)].iter()) {
            if (0..12).contains(&(y - p2)) {
                c = rgb_mix(c, (6.0, 3.0, 6.0), 0.55 * (1.0 - (y - p2) as f64 / 12.0));
            }
        }
        px(c)
    })
}

/// what the shelf shows of the app
#[derive(Clone, Debug, Default)]
pub struct ShelfData {
    /// minutes per subject since Monday
    pub week: HashMap<String, f64>,
    /// cards due per subject
    pub due: HashMap<String, i64>,
    pub grades: usize,
    /// _mincho_font / _serif_font
    pub jp: String,
    pub serif: String,
    pub hd: bool,
}

/// the shelf on the Study books page
pub struct ShelfPage {
    pub host: usize,
    pub cv: usize,
    pub spots: Vec<(String, i32, i32, i32, i32)>,
    pub hover: Option<String>,
    pub lift: HashMap<String, f64>,
    /// [x, y, speed, phase] and the item
    pub petals: Vec<([f64; 4], u32)>,
    pub glow: Vec<(String, u32)>,
    /// the scroll's kanji, title and description items and its left edge
    scroll: Option<(u32, u32, u32, f64)>,
    pub boxwh: Option<(i32, i32)>,
    imgs: HashMap<String, Rc<Photo>>,
    rnd: PyRandom,
}

/// shelf_layout(W, H): each book's (key, x, y, w, h), x0, the row's width, the two rows' tops, w, h
pub fn shelf_layout(wd: i32, ht: i32) -> (Vec<(String, i32, i32, i32, i32)>, i32, i32, [i32; 2], i32, i32) {
    let n_row = (BOOKS.len() as i32 + 1) / 2;
    let (wf, hf) = (wd as f64, ht as f64);
    let mut bw = 84.0f64.max(132.0f64.min((wf - 300.0) / n_row as f64 - 30.0)) as i32;
    bw = 72.0f64.max((bw as f64).min((hf - 262.0) / 2.0 / 1.42)) as i32;
    bw -= bw.rem_euclid(2);
    let mut bh = (bw as f64 * 1.42) as i32;
    bh -= bh.rem_euclid(2);
    let gap = 24.0f64.max(48.0f64.min((wf - 320.0 - (n_row * bw) as f64) / (n_row + 1) as f64)) as i32;
    let total = n_row * bw + (n_row - 1) * gap;
    let x0 = (wd - total).div_euclid(2);
    let top = 124;
    let rows = [top + 18, top + 18 + bh + 46];
    let spots = BOOKS.iter().enumerate().map(|(i, b)| {
        let (r, c) = (i as i32 / n_row, i as i32 % n_row);
        (b.0.to_string(), x0 + c * (bw + gap), rows[r as usize], bw, bh)
    }).collect();
    (spots, x0, total, rows, bw, bh)
}

impl ShelfPage {
    /// shelf_build: the night canvas filling the page
    pub fn build(ui: &mut Ui, pg: usize, paper: &str, seed: u64) -> ShelfPage {
        let host = ui.frame(pg, paper);
        ui.pack(host, Pack::default().fill(Fill::Both).expand());
        let cv = ui.canvas(host, "#0b0f2a", 1, 1);
        (ui.ws[cv].opt_w, ui.ws[cv].opt_h) = (0, 0);
        canvas_of(ui, cv);
        ui.pack(cv, Pack::default().fill(Fill::Both).expand());
        ShelfPage { host, cv, spots: vec![], hover: None, lift: HashMap::new(), petals: vec![], glow: vec![], scroll: None,
                    boxwh: None, imgs: HashMap::new(), rnd: PyRandom::new(seed) }
    }

    fn img(&mut self, key: String, make: impl FnOnce() -> Rc<Photo>) -> Rc<Photo> {
        if !self.imgs.contains_key(&key) {
            if self.imgs.len() > 60 {
                self.imgs.clear();
            }
            self.imgs.insert(key.clone(), make());
        }
        self.imgs[&key].clone()
    }

    /// shelf_draw for the canvas's size now
    pub fn draw(&mut self, ui: &mut Ui, fonts: &dyn Fonts, d: &ShelfData) {
        let (wd, ht) = (560.max(ui.ws[self.cv].w), 460.max(ui.ws[self.cv].h));
        let mut cv = Canvas::default();
        let (jp, serif) = (d.jp.clone(), d.serif.clone());
        let hd = d.hd;
        let (spots, x0, total, rows, _bw, bh) = shelf_layout(wd, ht);
        self.spots = spots.clone();
        self.boxwh = Some((wd, ht));
        self.glow.clear();
        let (wf, hf) = (wd as f64, ht as f64);
        // 1. the night, painted at a quarter size
        let (w4, h4) = (wd / 4 + 1, ht / 4 + 1);
        let sky = self.img(format!("sky{w4}x{h4}"), || px_photo(w4, h4, &shelf_sky(w4, h4), 4, hd));
        cv.image(0.0, 0.0, sky).anchor("nw");
        // 2. petals, behind everything
        if self.petals.is_empty() {
            for _ in 0..9 {
                let a = self.rnd.uniform(0.0, wf);
                let b = self.rnd.uniform(0.0, hf);
                let c = self.rnd.uniform(0.35, 1.0);
                let e = self.rnd.uniform(0.0, 6.28);
                self.petals.push(([a, b, c, e], 0));
            }
        }
        let soft: Vec<Rc<Photo>> = (0..2).map(|i| {
            self.img(format!("petal{i}"), || px_photo(9, 9, &petal_soft(4.5, 0.55 - 0.2 * i as f64), 2, hd))
        }).collect();
        for p in self.petals.iter_mut() {
            cv.image(p.0[0], p.0[1], soft[if p.0[2] > 0.7 { 0 } else { 1 }].clone()).tags(&["petal"]);
            p.1 = cv.last_id();
        }
        // 3. the title block
        cv.text(44.0, 26.0, "\u{672c}\n\u{68da}").anchor("n").fill("#e9cf8a").font(Font::new(&jp, 22, false)).justify("center");
        cv.line(vec![70.0, 30.0, 70.0, 92.0]).fill("#8f7a45");
        cv.text(86.0, 34.0, &spaced("THE LIBRARY")).anchor("w").fill("#c8ad6a").font(Font::new(&serif, 9, false));
        cv.text(86.0, 58.0, "Zenko's study books").anchor("w").fill("#f3ead6").font(Font::new(&serif, 19, false));
        let mut it = Font::new(&serif, 10, false);
        it.italic = true;
        cv.text(86.0, 84.0, "Choose a volume, and I'll open it for you.").anchor("w").fill("#a9a2c4").font(it);
        // 4. the cabinet
        let (l, r) = (x0 - 36, x0 + total + 36);
        let t_ = rows[0] - 28;
        let b_ = rows[1] + bh + 20;
        let (cw2, ch2) = ((r - l).div_euclid(2), (b_ - t_).div_euclid(2));
        let rel: Vec<i32> = rows.iter().map(|y| y - t_).collect();
        let relk = rel.clone();
        let cab = self.img(format!("cab{cw2}x{ch2}:{relk:?}:{bh}"), || px_photo(cw2, ch2, &shelf_cabinet(cw2, ch2, rel, bh), 2, hd));
        cv.image(l as f64, t_ as f64, cab).anchor("nw");
        // a slim shimenawa along the top, three paper shide
        let ry = (t_ - 3) as f64;
        let mut x = l - 4;
        let mut i = 0;
        while x < r + 6 {
            let sag = 5.0 * (PI * (x - l + 4) as f64 / (r - l + 8) as f64).sin();
            let xf = x as f64;
            cv.oval(xf - 4.0, ry + sag - 3.0, xf + 4.0, ry + sag + 3.0)
                .fill(if i % 2 != 0 { "#cdb67c" } else { "#a88d55" }).outline("");
            x += 6;
            i += 1;
        }
        for k in 1..=3 {
            let sx = l as f64 + (r - l) as f64 * k as f64 / 4.0;
            let y1 = ry + 5.0 * (PI * k as f64 / 4.0).sin() + 3.0;
            let pts = vec![sx - 4.0, y1, sx + 4.0, y1, sx + 4.0, y1 + 6.0, sx, y1 + 6.0, sx, y1 + 12.0, sx + 4.0, y1 + 12.0,
                           sx + 4.0, y1 + 18.0, sx - 4.0, y1 + 18.0, sx - 4.0, y1 + 12.0, sx, y1 + 12.0, sx, y1 + 6.0,
                           sx - 4.0, y1 + 6.0];
            cv.poly(pts).fill("#f4efe3").outline("");
        }
        // lanterns on little wooden arms
        let (lw, lh) = (12, 18);
        for (ax, lx) in [(l, l - 30), (r, r + 30)] {
            if lx < 18 || lx > wd - 18 {
                continue;
            }
            let ty = (t_ + 6) as f64;
            cv.line(vec![ax as f64, ty, lx as f64, ty]).fill("#4a2c18").width(4.0);
            let ly = t_ + 6 + 12 + lh * 3 / 2;
            cv.line(vec![lx as f64, ty, lx as f64, (ly - lh * 3 / 2) as f64]).fill("#2a1a12").width(1.0);
            let g = self.img("glow60".into(), || px_photo(60, 60, &glow_art(30, (255, 170, 80), 0.35), 2, hd));
            cv.image(lx as f64, ly as f64, g);
            let lan = self.img(format!("lan{lw}x{lh}"), || px_photo(lw, lh, &lantern_art(lw, lh), 3, hd));
            cv.image(lx as f64, ly as f64, lan);
        }
        // 5. the books
        for (key, x, y, w, h) in spots {
            let b = book(&key).unwrap();
            let tag = format!("bk_{key}");
            let lift = self.lift.get(&key).copied().unwrap_or(0.0);
            let (xf, yf, wf_, hf_) = (x as f64, y as f64, w as f64, h as f64);
            let yy = yf - lift;
            let gd = w / 2 + 26;
            let gimg = self.img(format!("bglow{w}"), || px_photo(gd, gd, &glow_art_f(gd as f64 / 2.0, (255, 214, 140), 0.55), 3, hd));
            cv.image(xf + wf_ / 2.0, yy + hf_ / 2.0, gimg).tags(&[&tag]).hide(self.hover.as_deref() != Some(key.as_str()));
            let gid = cv.last_id();
            cv.rect(xf + 4.0, yf + 3.0, xf + wf_ + 4.0, yf + hf_ + 1.0).fill("#000000").outline("").stipple("gray50");
            let rx = xf + wf_ * 0.72;
            cv.poly(vec![rx, yy + hf_ - 4.0, rx + 5.0, yy + hf_ - 4.0, rx + 5.0, yy + hf_ + 12.0, rx + 2.5, yy + hf_ + 9.0, rx,
                         yy + hf_ + 12.0]).fill(if key != "grades" { "#b0122b" } else { "#c9a24a" }).outline("").tags(&[&tag]);
            let (cw_, ch_) = (w / 2, h / 2);
            let col = b.4;
            let cover = self.img(format!("cover{key}{cw_}x{ch_}"), || px_photo(cw_, ch_, &book_art(col, cw_, ch_, false), 2, hd));
            cv.image(xf, yy, cover).anchor("nw").tags(&[&tag]);
            book_labels(&mut cv, xf, yy, w - 6, h - 6, &key, &tag, &jp, &serif);
            let sub = if SUBJECTS.contains(&key.as_str()) {
                format!("{} this week", fmt_hm(d.week.get(&key).copied().unwrap_or(0.0) as i64))
            } else if key == "grades" {
                format!("{} result{}", d.grades, if d.grades == 1 { "" } else { "s" })
            } else if key == "bible" {
                "Hebrew \u{b7} Greek \u{b7} notes".into()
            } else {
                "time \u{b7} goals \u{b7} sleep".into()
            };
            let mut f8 = Font::new(&serif, 8, false);
            f8.italic = true;
            cv.text(xf + wf_ / 2.0, yf + hf_ + 21.0, &sub).fill("#bfae8a").font(f8);
            self.glow.push((key.clone(), gid));
        }
        // 6. the legend: a hanging scroll under the shelf
        let cy = (hf - 40.0).min((b_ + 46) as f64);
        let sw = (wf - 120.0).min(600.0);
        self.scroll_draw(&mut cv, wf / 2.0, cy, sw, &jp, &serif, hd);
        *canvas_of(ui, self.cv) = cv;
        self.scroll_text(ui, fonts, d);
    }

    /// shelf_scroll: a makimono between two lacquered rollers with gold caps
    #[allow(clippy::too_many_arguments)]
    fn scroll_draw(&mut self, cv: &mut Canvas, cx: f64, cy: f64, sw: f64, jp: &str, serif: &str, hd: bool) {
        let ph = 52.0;
        let (x1, x2) = (cx - sw / 2.0, cx + sw / 2.0);
        let pw = (sw as i64 / 2) as i32;
        cv.rect(x1 + 4.0, cy - ph / 2.0 + 5.0, x2 + 4.0, cy + ph / 2.0 + 5.0).fill("#000000").outline("").stipple("gray50");
        let washi = self.img(format!("washi{pw}"), || px_photo(pw, 26, &washi_art(pw, 26), 2, hd));
        cv.image(x1, cy - ph / 2.0, washi).anchor("nw");
        for yb in [cy - ph / 2.0, cy + ph / 2.0 - 5.0] {
            cv.rect(x1, yb, x2, yb + 5.0).fill("#27305e").outline("");
            let ly = yb + if yb < cy { 5.0 } else { 0.0 };
            cv.line(vec![x1, ly, x2, ly]).fill("#b8975a");
        }
        for xr in [x1 - 7.0, x2 - 7.0] {
            cv.rect(xr, cy - ph / 2.0 - 6.0, xr + 14.0, cy + ph / 2.0 + 6.0).fill("#2a160d").outline("#120905");
            cv.line(vec![xr + 4.0, cy - ph / 2.0 - 4.0, xr + 4.0, cy + ph / 2.0 + 4.0]).fill("#5b3420");
            for yy in [cy - ph / 2.0 - 10.0, cy + ph / 2.0 + 6.0] {
                cv.rect(xr + 1.0, yy, xr + 13.0, yy + 4.0).fill("#c9a24a").outline("#7a5a1a");
            }
        }
        cv.text(x1 + 28.0, cy - 8.0, "").anchor("w").fill("#3a1f12").font(Font::new(jp, 13, false));
        let k = cv.last_id();
        cv.text(x1 + 28.0, cy - 8.0, "").anchor("w").fill("#2a1a12").font(Font::new(serif, 12, false));
        let t = cv.last_id();
        let mut f9 = Font::new(serif, 9, false);
        f9.italic = true;
        cv.text(x1 + 28.0, cy + 12.0, "").anchor("w").fill("#6b5236").font(f9);
        let dd = cv.last_id();
        self.scroll = Some((k, t, dd, x1));
    }

    /// shelf_scroll_text: what the hovered book is, or how to use the library
    pub fn scroll_text(&mut self, ui: &mut Ui, fonts: &dyn Fonts, d: &ShelfData) {
        let Some((ki, ti, di, x1)) = self.scroll else { return };
        let (k, t, dd) = match self.hover.as_deref().and_then(book) {
            Some(b) => {
                let last = practice(b.0).rsplit('\u{b7}').next().unwrap_or("").trim().to_string();
                let dd = if SUBJECTS.contains(&b.0) {
                    let wk = d.week.get(b.0).copied().unwrap_or(0.0) as i64;
                    let due = d.due.get(b.0).copied().unwrap_or(0);
                    format!("{} this week   \u{b7}   {} card{} due   \u{b7}   {}", fmt_hm(wk), due,
                            if due == 1 { "" } else { "s" }, last)
                } else {
                    last
                };
                (b.1.to_string(), b.3.to_string(), dd)
            }
            None => ("\u{6848}\u{5185}".into(), "How to use the library".into(),
                     "Click a volume to open it   \u{b7}   or say 'open my bio book', 'vocab drill', 'my grades'".into()),
        };
        let cv = canvas_of(ui, self.cv);
        if let Some(it) = cv.get(ki) {
            it.text = k;
        }
        let kx = cv.items.iter().find(|i| i.id == ki).map(|i| crate::canvasdraw::text_bbox(i, fonts));
        let ky = cv.items.iter().find(|i| i.id == ki).map(|i| i.coords[1]).unwrap_or(0.0);
        if let Some(it) = cv.get(ti) {
            it.coords = vec![kx.map(|b| b[2] as f64 + 10.0).unwrap_or(x1 + 28.0), ky];
            it.text = t;
        }
        if let Some(it) = cv.get(di) {
            it.text = dd;
        }
    }

    /// shelf_hover(k): the glow on the book under the pointer, the scroll's words
    pub fn set_hover(&mut self, ui: &mut Ui, fonts: &dyn Fonts, d: &ShelfData, k: Option<String>) -> bool {
        if k == self.hover {
            return false;
        }
        self.hover = k;
        let cv = canvas_of(ui, self.cv);
        for (kk, gid) in &self.glow {
            if let Some(it) = cv.get(*gid) {
                it.hidden = self.hover.as_deref() != Some(kk.as_str());
            }
        }
        self.scroll_text(ui, fonts, d);
        true
    }

    /// one shelf_tick (every 40 ms): petals drift, books glide to their hover height
    pub fn tick(&mut self, ui: &mut Ui) {
        let Some((wd, ht)) = self.boxwh else { return };
        let (wf, hf) = (wd as f64, ht as f64);
        let mut moves = vec![];
        for p in self.petals.iter_mut() {
            let q = &mut p.0;
            q[3] += 0.025;
            q[0] += 0.28 * q[2] + 0.22 * q[3].sin();
            q[1] += 0.22 * q[2];
            if q[1] > hf + 12.0 || q[0] > wf + 12.0 {
                q[0] = self.rnd.uniform(-wf * 0.2, wf * 0.9);
                q[1] = -12.0;
            }
            moves.push((p.1, q[0], q[1]));
        }
        let mut lifts = vec![];
        for (key, ..) in &self.spots {
            let cur = self.lift.get(key).copied().unwrap_or(0.0);
            let goal = if self.hover.as_deref() == Some(key.as_str()) { 7.0 } else { 0.0 };
            if (goal - cur).abs() > 0.05 {
                let mut nxt = cur + (goal - cur) * 0.35;
                if (goal - nxt).abs() < 0.3 {
                    nxt = goal;
                }
                lifts.push((key.clone(), cur, nxt));
            }
        }
        let cv = canvas_of(ui, self.cv);
        for (id, x, y) in moves {
            if let Some(it) = cv.get(id) {
                it.coords = vec![x, y];
            }
        }
        for (key, cur, nxt) in lifts {
            cv.move_items(&format!("bk_{key}"), 0.0, -(nxt - cur));
            self.lift.insert(key, nxt);
        }
    }

    /// shelf_key(x, y): the book under a canvas point
    pub fn key_at(&self, x: i32, y: i32) -> Option<String> {
        self.spots.iter().find(|(_, bx, by, w, h)| *bx <= x && x <= bx + w && by - 10 <= y && y <= by + h).map(|s| s.0.clone())
    }
}

/// spaced(t): L E T T E R - S P A C E D
pub fn spaced(t: &str) -> String {
    t.chars().map(|c| c.to_string()).collect::<Vec<_>>().join(" ")
}

/// book_labels: the daisen slip with vertical kanji, the hanko, the name plate
#[allow(clippy::too_many_arguments)]
pub fn book_labels(cv: &mut Canvas, x: f64, y: f64, w: i32, h: i32, key: &str, tag: &str, jp: &str, serif: &str) {
    let b = book(key).unwrap();
    let tags: Vec<&str> = if tag.is_empty() { vec!["book"] } else { vec!["book", tag] };
    let (wf, hf) = (w as f64, h as f64);
    let sp = 10.max(w / 10);
    let dw = 18.max((wf / 4.0) as i32);
    let dx = x + (w - dw - 8.max(w / 9)) as f64;
    let dy = y + 9.max(h / 12) as f64;
    let dh = hf * 0.47;
    let dwf = dw as f64;
    cv.rect(dx, dy, dx + dwf, dy + dh).fill("#f6efdf").outline("#a98d55").tags(&tags);
    cv.rect(dx + 2.0, dy + 2.0, dx + dwf - 2.0, dy + dh - 2.0).outline("#e0d0a6").tags(&tags);
    let fs = 9.0f64.max(19.0f64.min(dwf * 0.6)) as i32;
    let kan: Vec<String> = b.1.chars().map(|c| c.to_string()).collect();
    cv.text(dx + dwf / 2.0, dy + dh / 2.0, &kan.join("\n")).fill("#1e1010").font(Font::new(jp, fs, false)).justify("center")
        .tags(&tags);
    let hs = 9.max(w / 8);
    let hsf = hs as f64;
    let (hx, hy) = (dx - hsf - 4.0, dy + dh - hsf);
    cv.rect(hx, hy, hx + hsf, hy + hsf).fill("#b8122a").outline("").tags(&tags);
    cv.text(hx + hsf / 2.0, hy + hsf / 2.0 + 1.0, "\u{72d0}").fill("#fbe9e9").font(Font::new(jp, 6.max((hsf * 0.52) as i32), false))
        .tags(&tags);
    let py = y + hf - 21.0;
    cv.rect(x + (sp + 6) as f64, py, x + wf - 8.0, py + 14.0).fill("#1a0e09").outline("#a98d55").tags(&tags);
    cv.text(x + sp as f64 + (w - sp) as f64 / 2.0 + 1.0, py + 7.0, &clip(b.3, 22)).fill("#f0deb4")
        .font(Font::new(serif, if w < 112 { 7 } else { 8 }, false)).tags(&tags);
}

impl crate::Companion {
    /// what the shelf shows: this week's minutes per subject, cards due per subject, how many grades
    pub fn shelf_data(&self) -> ShelfData {
        let ws = self.week_start();
        let week = self.subject_minutes(1, Some(ws)).into_iter().collect();
        let mut due: HashMap<String, i64> = HashMap::new();
        for c in self.cards_due() {
            if let Some(s) = c["subject"].as_str() {
                *due.entry(s.to_string()).or_default() += 1;
            }
        }
        let grades = self.life_now()["grades"].as_array().map(|a| a.len()).unwrap_or(0);
        ShelfData { week, due, grades, jp: String::new(), serif: String::new(), hd: self.cfg.b("hd_art", true) }
    }
}

/// Python's float floor division a // b
fn fdiv(a: f64, b: f64) -> f64 {
    (a / b).floor()
}

/// book_cover(cv, x, y, w, h, key, tag, pattern): a closed book drawn with canvas shapes (the opening animation's)
#[allow(clippy::too_many_arguments)]
pub fn book_cover(cv: &mut Canvas, x: f64, y: f64, w: f64, h: f64, key: &str, col: &str, jp: &str, font_ui: &str, tag: &str,
                  pattern: bool) {
    let Some(b) = book(key) else { return };
    let mix = crate::hubwin::mix;
    let cover = mix(col, "#000000", 0.28);
    let lite = mix(col, "#ffffff", 0.18);
    let tags: Vec<&str> = if tag.is_empty() { vec!["book"] } else { vec!["book", tag] };
    cv.rect(x + 4.0, y + 5.0, x + w + 4.0, y + h + 5.0).fill("#000000").outline("").stipple("gray50").tags(&tags);
    cv.rect(x, y, x + w, y + h).fill(&cover).outline(&mix(&cover, "#000000", 0.4)).tags(&tags);
    let sp = 6.0f64.max(fdiv(w, 11.0));
    if pattern && w > 50.0 {
        // seigaiha waves on the lower half of the cover
        let r = 6.0f64.max(fdiv(w, 9.0));
        let wave = mix(&cover, &lite, 0.45);
        let mut yy = y + h - 4.0;
        let mut row = 0;
        while yy > y + h * 0.52 {
            let off = if row % 2 == 1 { r } else { 0.0 };
            let mut xx = x - off;
            while xx < x + w + r {
                for k in [1.0, 0.66, 0.33] {
                    let rr = r * k;
                    if xx - rr >= x + sp + 1.0 && xx + rr <= x + w - 1.0 {
                        cv.arc(xx - rr, yy - rr, xx + rr, yy + rr, 0.0, 180.0, "arc").outline(&wave).tags(&tags);
                    }
                }
                xx += 2.0 * r;
            }
            yy -= r * 0.75;
            row += 1;
        }
        cv.rect(x, y + h - 2.0, x + w, y + h).fill(&cover).outline("").tags(&tags);
    }
    // stab binding (yotsume toji) along the spine
    cv.rect(x, y, x + sp, y + h).fill(&mix(&cover, "#000000", 0.3)).outline("").tags(&tags);
    let sx = x + sp / 2.0 + 1.0;
    for i in 0..4 {
        let sy = y + h * (0.12 + 0.253 * i as f64);
        cv.line(vec![x, sy, sx + 4.0, sy]).fill("#f0e6cc").width(1.0).tags(&tags);
        cv.oval(sx + 2.0, sy - 2.0, sx + 6.0, sy + 2.0).fill("#f0e6cc").outline("").tags(&tags);
    }
    if w > 50.0 {
        // daisen: the white title slip with vertical kanji
        let dw = 18.0f64.max(fdiv(w, 4.2));
        let dx = x + w - dw - 6.0f64.max(fdiv(w, 10.0));
        let dy = y + 8.0f64.max(fdiv(h, 14.0));
        let dh = h * 0.5;
        cv.rect(dx, dy, dx + dw, dy + dh).fill("#f6efdc").outline("#c9b98e").tags(&tags);
        cv.rect(dx + 2.0, dy + 2.0, dx + dw - 2.0, dy + dh - 2.0).outline("#d9cba3").tags(&tags);
        let fs = 9.0f64.max(19.0f64.min(dw * 0.62)) as i32;
        let kan: Vec<String> = b.1.chars().map(|c| c.to_string()).collect();
        cv.text(dx + dw / 2.0, dy + dh / 2.0, &kan.join("\n")).fill("#2a1717").font(Font::new(jp, fs, true)).justify("center")
            .tags(&tags);
        // hanko seal
        let hs = 10.0f64.max(fdiv(w, 7.0));
        let (hx, hy) = (dx - hs - 4.0, dy + dh - hs);
        cv.rect(hx, hy, hx + hs, hy + hs).fill("#c8102e").outline("#8a0b20").tags(&tags);
        cv.text(hx + hs / 2.0, hy + hs / 2.0 + 1.0, "\u{72d0}").fill("#ffffff").font(Font::new(jp, 6.max((hs * 0.5) as i32), true))
            .tags(&tags);
        // the english name on the cover's foot
        cv.rect(x + sp + 4.0, y + h - 26.0, x + w - 6.0, y + h - 8.0).fill(&mix(&cover, "#000000", 0.35)).outline("").tags(&tags);
        cv.text(x + sp + (w - sp) / 2.0, y + h - 17.0, &clip(b.3, 20)).fill("#f6efdc")
            .font(Font::new(font_ui, if w < 110.0 { 7 } else { 8 }, true)).tags(&tags);
    }
}

/// book_anim's frames: 9 of the book gliding to the middle and growing, then 12 of its cover swinging open
pub const ANIM_A: usize = 9;
pub const ANIM_B: usize = 12;

/// book_anim's frame(i) for the book at `spot` (x, y, w, h) on a W x H canvas: the canvas's "anim" items made again
#[allow(clippy::too_many_arguments)]
pub fn anim_frame(cv: &mut Canvas, i: usize, spot: (i32, i32, i32, i32), wd: i32, ht: i32, key: &str, col: &str, jp: &str,
                  font_ui: &str) {
    let mix = crate::hubwin::mix;
    let (x, y, w, h) = (spot.0 as f64, spot.1 as f64, spot.2 as f64, spot.3 as f64);
    let (wf, hf) = (wd as f64, ht as f64);
    let w2 = (wf * 0.3).min((hf - 60.0) / 1.4) as i64 as f64;
    let h2 = (w2 * 1.4) as i64 as f64;
    let (cx, cy) = (wf / 2.0, (hf - h2) / 2.0);
    let cover = mix(col, "#000000", 0.28);
    let inside = mix(col, "#ffffff", 0.55);
    let paper = "#f6efdc";
    cv.delete("anim");
    cv.rect(0.0, 0.0, wf, hf).fill("#000000").outline("").stipple("gray50").tags(&["anim"]);
    if i < ANIM_A {
        let t = (i + 1) as f64 / ANIM_A as f64;
        let t = 1.0 - (1.0 - t).powi(3);
        let bx = x + (cx - w2 / 2.0 - x) * t;
        let by = y + (cy - y) * t;
        let bw = w + (w2 - w) * t;
        let bh = h + (h2 - h) * t;
        book_cover(cv, bx, by, bw, bh, key, col, jp, font_ui, "anim", i == ANIM_A - 1);
    } else {
        let t = (i - ANIM_A + 1) as f64 / ANIM_B as f64;
        let spine = cx - w2 / 2.0 + (w2 / 2.0) * (t * 1.4).min(1.0);
        cv.rect(spine, cy, spine + w2, cy + h2).fill(paper).outline("#c9b98e").tags(&["anim"]);
        for k in 1..9 {
            let ly = cy + h2 * k as f64 / 9.0;
            cv.line(vec![spine + 12.0, ly, spine + w2 - 12.0, ly]).fill("#e3d6b4").tags(&["anim"]);
        }
        let ang = PI * t;
        let cw = w2 * ang.cos();
        let skew = h2 * 0.06 * ang.sin();
        if cw >= 0.0 {
            let pts = vec![spine, cy, spine + cw, cy - skew, spine + cw, cy + h2 + skew, spine, cy + h2];
            cv.poly(pts).fill(&cover).outline(&mix(&cover, "#000000", 0.4)).tags(&["anim"]);
        } else {
            let lw = -cw;
            cv.rect(spine - w2, cy, spine, cy + h2).fill(paper).outline("#c9b98e").tags(&["anim"]);
            let pts = vec![spine, cy, spine - lw, cy - skew, spine - lw, cy + h2 + skew, spine, cy + h2];
            cv.poly(pts).fill(&inside).outline(&mix(&inside, "#000000", 0.3)).tags(&["anim"]);
        }
        cv.line(vec![spine, cy, spine, cy + h2]).fill(&mix(&cover, "#000000", 0.3)).width(3.0).tags(&["anim"]);
    }
}

/// the delay before frame i + 1 (or, after the last, before the book opens)
pub fn anim_delay(i: usize) -> f64 {
    if i + 1 < ANIM_A + ANIM_B {
        if i < ANIM_A { 0.022 } else { 0.026 }
    } else {
        0.09
    }
}
