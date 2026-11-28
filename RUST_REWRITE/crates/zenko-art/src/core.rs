//! Grid primitives, ported line-for-line from zenko.py ("Pixel-art sprites" section).
//! Every picture is a W x H grid of palette letters ('.' = transparent).
//! Floating-point maths follows the Python expression order exactly so the cells come out identical.

use std::collections::{HashMap, HashSet};
use std::f64::consts::PI;

pub const W: i32 = 110;
pub const H: i32 = 78;
pub const CX: f64 = 36.0;
pub const TAIL_BASE: (f64, f64) = (50.0, 67.0);
pub const BELLY_X: i32 = 19;
pub const BELLY_Y: i32 = 58;

const BAYER: [[i32; 4]; 4] = [[0, 8, 2, 10], [12, 4, 14, 6], [3, 11, 1, 9], [15, 7, 13, 5]];
const LIGHT: (f64, f64, f64) = (-0.52, -0.62, 0.59);

pub type Cells = HashSet<(i32, i32)>;
pub type Pt = (f64, f64);

// ---- Python number semantics ------------------------------------------------------------------
/// Python's float `%` (result has the sign of the divisor).
#[inline]
pub fn pymod(a: f64, b: f64) -> f64 {
    let mut m = a % b;
    if m != 0.0 && ((b < 0.0) != (m < 0.0)) {
        m += b;
    }
    m
}
/// Python's int `%` / `//`.
#[inline]
pub fn imod(a: i32, b: i32) -> i32 {
    a.rem_euclid(b)
}
#[inline]
pub fn ifloordiv(a: i32, b: i32) -> i32 {
    a.div_euclid(b)
}
/// Python's `round(x)` (half to even) as an int.
#[inline]
pub fn pyround(x: f64) -> i32 {
    x.round_ties_even() as i32
}
/// Python's `int(x)` (truncate toward zero).
#[inline]
pub fn pint(x: f64) -> i32 {
    x as i32
}
#[inline]
pub fn radians(d: f64) -> f64 {
    d * (PI / 180.0)
}
#[inline]
pub fn pw(a: f64, b: f64) -> f64 {
    a.powf(b)
}
/// CPython's math.hypot for two finite values (vector_norm with its correction step),
/// so results match Python to the last bit.
pub fn hypot(x: f64, y: f64) -> f64 {
    let (x, y) = (x.abs(), y.abs());
    let max = x.max(y);
    if max == 0.0 {
        return 0.0;
    }
    if x.is_infinite() || y.is_infinite() {
        return f64::INFINITY;
    }
    let (_, max_e) = frexp(max);
    if max_e < -1023 {
        // subnormal: plain fallback (never happens for sprite coordinates)
        return (x * x + y * y).sqrt();
    }
    let scale = ldexp(1.0, -max_e);
    let mut csum = 1.0f64;
    let mut frac1 = 0.0f64;
    let mut frac2 = 0.0f64;
    for v in [x, y] {
        let v = v * scale;
        // dl_mul(v, v)
        let (pr_hi, pr_lo) = dl_mul(v, v);
        // dl_fast_sum(csum, pr_hi)
        let hi = csum + pr_hi;
        let lo = (csum - hi) + pr_hi;
        csum = hi;
        frac1 += pr_lo;
        frac2 += lo;
    }
    let mut h = (csum - 1.0 + (frac1 + frac2)).sqrt();
    let (pr_hi, pr_lo) = dl_mul(-h, h);
    let hi = csum + pr_hi;
    let lo = (csum - hi) + pr_hi;
    csum = hi;
    frac1 += pr_lo;
    frac2 += lo;
    let x2 = csum - 1.0 + (frac1 + frac2);
    h += x2 / (2.0 * h);
    h / scale
}
#[inline]
fn dl_mul(x: f64, y: f64) -> (f64, f64) {
    let z = x * y;
    let zz = x.mul_add(y, -z);
    (z, zz)
}
fn frexp(x: f64) -> (f64, i32) {
    if x == 0.0 {
        return (0.0, 0);
    }
    let bits = x.to_bits();
    let exp = ((bits >> 52) & 0x7ff) as i32;
    if exp == 0 {
        let (m, e) = frexp(x * f64::from_bits(0x4350000000000000)); // 2^54
        return (m, e - 54);
    }
    let e = exp - 1022;
    let m = f64::from_bits((bits & !(0x7ffu64 << 52)) | (1022u64 << 52));
    (m, e)
}
fn ldexp(x: f64, e: i32) -> f64 {
    x * 2f64.powi(e)
}

// ---- the grid ---------------------------------------------------------------------------------
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Grid {
    pub w: i32,
    pub h: i32,
    pub c: Vec<u8>,
}

impl Grid {
    pub fn blank() -> Grid {
        Grid::new(W, H)
    }
    pub fn new(w: i32, h: i32) -> Grid {
        Grid {
            w,
            h,
            c: vec![b'.'; (w * h) as usize],
        }
    }
    pub fn from_rows<S: AsRef<str>>(rows: &[S]) -> Grid {
        let h = rows.len() as i32;
        let w = rows.first().map(|r| r.as_ref().len()).unwrap_or(0) as i32;
        let mut g = Grid::new(w, h);
        for (y, r) in rows.iter().enumerate() {
            for (x, ch) in r.as_ref().bytes().enumerate() {
                g.c[y * w as usize + x] = ch;
            }
        }
        g
    }
    #[inline]
    pub fn inb(&self, x: i32, y: i32) -> bool {
        0 <= x && x < self.w && 0 <= y && y < self.h
    }
    /// In-bounds read (callers guarantee bounds, like the Python code does).
    #[inline]
    pub fn at(&self, x: i32, y: i32) -> u8 {
        self.c[(y * self.w + x) as usize]
    }
    /// Python list indexing: negative indices count from the end.
    #[inline]
    pub fn pyat(&self, x: i32, y: i32) -> u8 {
        let x = if x < 0 { x + self.w } else { x };
        let y = if y < 0 { y + self.h } else { y };
        self.at(x, y)
    }
    #[inline]
    pub fn set(&mut self, x: i32, y: i32, ch: u8) {
        let i = (y * self.w + x) as usize;
        self.c[i] = ch;
    }
    #[inline]
    pub fn put(&mut self, x: i32, y: i32, ch: u8) {
        if self.inb(x, y) {
            self.set(x, y, ch);
        }
    }
    pub fn put2(&mut self, x: i32, y: i32, ch: u8) {
        self.put(x, y, ch);
        self.put(mx(x), y, ch);
    }
    pub fn rows(&self) -> Vec<String> {
        (0..self.h)
            .map(|y| {
                String::from_utf8(
                    self.c[(y * self.w) as usize..((y + 1) * self.w) as usize].to_vec(),
                )
                .unwrap()
            })
            .collect()
    }
    pub fn mirror_x(&mut self) {
        let w = self.w as usize;
        for row in self.c.chunks_mut(w) {
            row.reverse();
        }
    }
}

#[inline]
pub fn mx(x: i32) -> i32 {
    71 - x
}

#[inline]
pub fn in_set(set: &[u8], ch: u8) -> bool {
    set.contains(&ch)
}

pub fn dither(x: i32, y: i32, v: f64, tones: &[u8], soft: f64) -> u8 {
    let v = 0.999f64.min(0.0f64.max(v));
    let pos = v * (tones.len() as f64 - 1.0);
    let mut i = pos as usize;
    let f = pos - i as f64;
    let f = 1.0f64.min(0.0f64.max((f - 0.5) * soft + 0.5));
    if i + 1 < tones.len()
        && f > (BAYER[imod(y, 4) as usize][imod(x, 4) as usize] as f64 + 0.5) / 16.0
    {
        i += 1;
    }
    tones[i]
}
#[inline]
pub fn dith(x: i32, y: i32, v: f64, tones: &[u8]) -> u8 {
    dither(x, y, v, tones, 3.0)
}

pub fn ell(cx: f64, cy: f64, rx: f64, ry: f64) -> Cells {
    let mut out = Cells::new();
    for y in 0.max(pint(cy - ry) - 1)..H.min(pint(cy + ry) + 2) {
        for x in 0.max(pint(cx - rx) - 1)..W.min(pint(cx + rx) + 2) {
            if pw((x as f64 + 0.5 - cx) / rx, 2.0) + pw((y as f64 + 0.5 - cy) / ry, 2.0) <= 1.0 {
                out.insert((x, y));
            }
        }
    }
    out
}

pub fn poly(pts: &[Pt]) -> Cells {
    let mut out = Cells::new();
    let mut miny = f64::INFINITY;
    let mut maxy = f64::NEG_INFINITY;
    let mut minx = f64::INFINITY;
    let mut maxx = f64::NEG_INFINITY;
    for p in pts {
        minx = minx.min(p.0);
        maxx = maxx.max(p.0);
        miny = miny.min(p.1);
        maxy = maxy.max(p.1);
    }
    let n = pts.len();
    for y in 0.max(pint(miny) - 1)..H.min(pint(maxy) + 2) {
        for x in 0.max(pint(minx) - 1)..W.min(pint(maxx) + 2) {
            let (px, py) = (x as f64 + 0.5, y as f64 + 0.5);
            let mut inside = false;
            let mut j = n - 1;
            for i in 0..n {
                let (xi, yi) = pts[i];
                let (xj, yj) = pts[j];
                if (yi > py) != (yj > py) && px < (xj - xi) * (py - yi) / (yj - yi) + xi {
                    inside = !inside;
                }
                j = i;
            }
            if inside {
                out.insert((x, y));
            }
        }
    }
    out
}

pub fn mir(cells: &Cells) -> Cells {
    let mut out = cells.clone();
    for &(x, y) in cells {
        out.insert((mx(x), y));
    }
    out
}

pub fn shift(cells: &Cells, dx: i32, dy: i32) -> Cells {
    cells.iter().map(|&(x, y)| (x + dx, y + dy)).collect()
}

pub fn union(a: &Cells, b: &Cells) -> Cells {
    a.union(b).copied().collect()
}

/// Fill cells (colour = f(x, y)) and outline them on the outside.
pub fn paint(
    g: &mut Grid,
    cells: &Cells,
    colour: &dyn Fn(i32, i32) -> u8,
    outline: Option<u8>,
    over: Option<&[u8]>,
) {
    for &(x, y) in cells {
        if g.inb(x, y) {
            let c = colour(x, y);
            g.set(x, y, c);
        }
    }
    if let Some(ol) = outline {
        for &(x, y) in cells {
            for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                let n = (x + dx, y + dy);
                if !cells.contains(&n) && g.inb(n.0, n.1) {
                    if over.map_or(true, |o| in_set(o, g.at(n.0, n.1))) {
                        g.set(n.0, n.1, ol);
                    }
                }
            }
        }
    }
}

pub fn paint_ch(g: &mut Grid, cells: &Cells, ch: u8, outline: Option<u8>, over: Option<&[u8]>) {
    paint(g, cells, &|_, _| ch, outline, over)
}

pub fn lit(
    cx: f64,
    cy: f64,
    rx: f64,
    ry: f64,
    tones: &'static [u8],
    bias: f64,
    soft: f64,
) -> impl Fn(i32, i32) -> u8 {
    let (lx, ly, lz) = LIGHT;
    move |x: i32, y: i32| {
        let nx = (x as f64 + 0.5 - cx) / rx;
        let ny = (y as f64 + 0.5 - cy) / ry;
        let nz = (0.0f64.max(1.0 - 1.0f64.min(nx * nx + ny * ny))).sqrt();
        let d = nx * lx + ny * ly + nz * lz;
        dither(x, y, 1.0 - (d * 0.5 + 0.5) + bias, tones, soft)
    }
}

/// Soft inner lines ('k') that touch the outside become hard outline ('K').
pub fn seal(g: &mut Grid) {
    for y in 0..g.h {
        for x in 0..g.w {
            if g.at(x, y) == b'k' {
                for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                    let (nx, ny) = (x + dx, y + dy);
                    if !g.inb(nx, ny) || g.at(nx, ny) == b'.' {
                        g.set(x, y, b'K');
                        break;
                    }
                }
            }
        }
    }
}

/// Draw one grid on top of another (transparent '.' cells are skipped).
pub fn over(g: &mut Grid, layer: &Grid) {
    for i in 0..g.c.len() {
        if layer.c[i] != b'.' {
            g.c[i] = layer.c[i];
        }
    }
}

pub fn stamp(g: &mut Grid, x0: i32, y0: i32, rows: &[&str], keep: &[u8]) {
    for (r, row) in rows.iter().enumerate() {
        for (c, ch) in row.bytes().enumerate() {
            if !in_set(keep, ch) {
                g.put(x0 + c as i32, y0 + r as i32, ch);
            }
        }
    }
}
pub fn stampd(g: &mut Grid, x0: i32, y0: i32, rows: &[&str]) {
    stamp(g, x0, y0, rows, b".")
}

pub fn k_to_d(g: &mut Grid) {
    for c in g.c.iter_mut() {
        if *c == b'k' {
            *c = b'D';
        }
    }
}

// ---- tails ------------------------------------------------------------------------------------
pub fn cubic(p: &[Pt; 4], t: f64) -> Pt {
    let a = pw(1.0 - t, 3.0);
    let b = 3.0 * pw(1.0 - t, 2.0) * t;
    let c = 3.0 * (1.0 - t) * t * t;
    let d = pw(t, 3.0);
    (
        a * p[0].0 + b * p[1].0 + c * p[2].0 + d * p[3].0,
        a * p[0].1 + b * p[1].1 + c * p[2].1 + d * p[3].1,
    )
}

/// Insertion-ordered map, like a Python dict.
pub struct OrdMap<V> {
    pub keys: Vec<(i32, i32)>,
    pub vals: Vec<V>,
    idx: HashMap<(i32, i32), usize>,
}
impl<V> OrdMap<V> {
    pub fn new() -> Self {
        OrdMap {
            keys: vec![],
            vals: vec![],
            idx: HashMap::new(),
        }
    }
    pub fn get(&self, k: &(i32, i32)) -> Option<&V> {
        self.idx.get(k).map(|&i| &self.vals[i])
    }
    pub fn insert(&mut self, k: (i32, i32), v: V) {
        if let Some(&i) = self.idx.get(&k) {
            self.vals[i] = v;
        } else {
            self.idx.insert(k, self.keys.len());
            self.keys.push(k);
            self.vals.push(v);
        }
    }
    pub fn contains(&self, k: &(i32, i32)) -> bool {
        self.idx.contains_key(k)
    }
}

/// One plump, fluffy tail along a cubic curve: fur coloured, a white brush tip, a little foxfire at the end.
pub fn tail(
    g: &mut Grid,
    pts: &[Pt; 4],
    rmax: f64,
    tid: i32,
    flame: i32,
    base_r: f64,
    tip_from: f64,
    line: u8,
) -> Cells {
    let mut cells: OrdMap<(f64, f64)> = OrdMap::new();
    let steps = 140;
    let mut prev = cubic(pts, 0.0);
    for i in 1..=steps {
        let t = i as f64 / steps as f64;
        let (cx, cy) = cubic(pts, t);
        let (tx, ty) = (cx - prev.0, cy - prev.1);
        prev = (cx, cy);
        let mut ln = hypot(tx, ty);
        if ln == 0.0 {
            ln = 1.0;
        }
        let (mut nx, mut ny) = (ty / ln, -tx / ln);
        if nx + ny > 0.0 {
            nx = -nx;
            ny = -ny;
        }
        let mut r = if t < 0.55 {
            base_r + (rmax - base_r) * (PI / 2.0 * t / 0.55).sin()
        } else {
            rmax * 0.0f64.max(1.0 - pw((t - 0.55) / 0.45, 2.2)) + 0.7
        };
        r *= 1.0 + 0.07 * (t * 26.0 + tid as f64 * 1.7).sin();
        for yy in (pint(cy - r) - 1)..(pint(cy + r) + 2) {
            for xx in (pint(cx - r) - 1)..(pint(cx + r) + 2) {
                let (dx, dy) = (xx as f64 + 0.5 - cx, yy as f64 + 0.5 - cy);
                if dx * dx + dy * dy <= r * r {
                    let k = (xx, yy);
                    let upd = match cells.get(&k) {
                        None => true,
                        Some(old) => t >= old.0,
                    };
                    if upd {
                        cells.insert(k, (t, (dx * nx + dy * ny) / r.max(0.8)));
                    }
                }
            }
        }
    }
    let tip0 = tip_from;
    let sel: Cells = cells.keys.iter().copied().collect();
    {
        let colour = |x: i32, y: i32| -> u8 {
            let &(t, ofs) = cells.get(&(x, y)).unwrap();
            let jit = 0.025 * (imod(x * 7 + y * 13 + tid, 5) - 2) as f64;
            if t > 0.95 - 0.02 * flame as f64 + jit {
                return b'U';
            }
            if t > 0.90 - 0.015 * flame as f64 + jit {
                return if ofs.abs() < 0.45 { b'A' } else { b'I' };
            }
            if t > tip0 + jit {
                return dith(x, y, 0.45 - ofs * 0.5, b"WWC");
            }
            let v = 0.42 - ofs * 0.55 + if t < 0.10 { 0.3 } else { 0.0 };
            dither(x, y, v, b"lLOD", 2.2)
        };
        paint(g, &sel, &colour, None, None);
    }
    for (i, &(x, y)) in cells.keys.iter().enumerate() {
        let t = cells.vals[i].0;
        for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
            let n = (x + dx, y + dy);
            if !cells.contains(&n) && g.inb(n.0, n.1) {
                let cur = g.at(n.0, n.1);
                if cur == b'.' || t > 0.5 {
                    g.set(n.0, n.1, if cur != b'.' { line } else { b'K' });
                }
            }
        }
    }
    sel
}

/// d(deg) helper used by every tail layout: (cos, -sin).
#[inline]
pub fn dvec(deg: f64) -> Pt {
    let r = radians(deg);
    (r.cos(), -r.sin())
}

pub fn tail_layout(n: i32, phase: f64, sway: f64) -> Vec<(f64, [Pt; 4])> {
    let n = 1.max(9.min(n));
    let (mut lo, mut hi) = if n > 1 { (6.0, 80.0) } else { (34.0, 34.0) };
    if n == 2 {
        lo = 18.0;
        hi = 58.0;
    }
    let (bx, by) = TAIL_BASE;
    let mut out = vec![];
    for i in 0..n {
        let fi = i as f64;
        let mut a = lo + (hi - lo) * fi / (1.max(n - 1)) as f64;
        a += 3.5 * sway * (phase * 2.0 * PI + fi * 0.9).sin();
        let l = (if n > 5 { 40.0 } else { 38.0 }) - 4.0 * pw(a / 90.0, 2.0)
            + if i % 2 != 0 { 2.0 } else { 0.0 };
        let da = dvec(a);
        let tip = (bx + da.0 * l, by + da.1 * l);
        let c1 = dvec(a - 30.0);
        let c2 = dvec(a + 30.0 + 7.0 * (phase * 2.0 * PI + fi).sin());
        let p1 = (bx + c1.0 * l * 0.45, by + c1.1 * l * 0.45);
        let p2 = (tip.0 - c2.0 * l * 0.35, tip.1 - c2.1 * l * 0.35);
        out.push((a, [(bx, by), p1, p2, tip]));
    }
    out
}

/// The tails (a separate layer drawn behind the body).
pub fn build_tails(n: i32, frame: i32, frames: i32, shadow: bool) -> Grid {
    let mut g = Grid::blank();
    if shadow {
        for (x, y) in ell(42.0, 76.6, 32.0, 1.6) {
            if (x + y) % 2 == 0 {
                g.set(x, y, b'g');
            }
        }
    }
    let phase = frame as f64 / frames as f64;
    let nn = 1.max(9.min(n));
    let mut lay = tail_layout(nn, phase, 1.0);
    lay.sort_by(|a, b| (-a.0).partial_cmp(&(-b.0)).unwrap());
    let r = if nn <= 3 {
        9.4
    } else if nn <= 6 {
        9.0
    } else {
        8.8
    };
    for (i, (_a, pts)) in lay.iter().enumerate() {
        let i = i as i32;
        tail(&mut g, pts, r, i * 3 + 1, (frame + i) % 3, 3.0, 0.66, b'k');
    }
    seal(&mut g);
    k_to_d(&mut g);
    g
}
