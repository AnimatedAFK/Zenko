//! zenko19 anatomical fox rig (side view): two-bone IK legs, torso on landmarks, poses and the walk.
//! Ported from zenko.py: _ik2, _capsule, _bez_pts, _TORSO, _spine_xy, _leg_cells, _draw_leg, _rig_tails,
//! _rig_head, build_rig, robe_rig, _walk_leg, walk_rig_pose, rig_pose, rig_rows.

use crate::core::*;
use crate::palette::{is_robe, robe_motif, robe_of};
use crate::sit::{Look, blush, draw_outfit, effects, eyes, glasses, has, head, motif, mouth};
use std::f64::consts::PI;

pub const RIG_GROUND: f64 = 75.6;
pub const FORE: (f64, f64, f64) = (8.4, 10.0, 4.0);
pub const HIND: (f64, f64, f64) = (9.4, 10.4, 6.8);

pub const TORSO: [Pt; 15] = [
    (-0.24, -3.6),
    (-0.10, -7.6),
    (0.20, -9.0),
    (0.55, -8.8),
    (0.88, -9.6),
    (1.10, -10.5),
    (1.34, -5.0),
    (1.30, 2.6),
    (1.12, 10.2),
    (0.80, 11.6),
    (0.48, 11.4),
    (0.22, 9.4),
    (0.00, 8.0),
    (-0.20, 4.6),
    (-0.30, 0.0),
];
pub fn torso_lie() -> Vec<Pt> {
    TORSO
        .iter()
        .map(|&(u, v)| (u, v + if v > 0.0 { 1.6 } else { 0.0 }))
        .collect()
}

pub fn ik2(ax: f64, ay: f64, bx: f64, by: f64, l1: f64, l2: f64, bend: f64) -> Pt {
    let (mut dx, mut dy) = (bx - ax, by - ay);
    let mut d = hypot(dx, dy);
    if d == 0.0 {
        d = 0.001;
    }
    if d > l1 + l2 - 0.05 {
        let k = (l1 + l2 - 0.05) / d;
        dx *= k;
        dy *= k;
        d = l1 + l2 - 0.05;
    }
    if d < (l1 - l2).abs() + 0.05 {
        d = (l1 - l2).abs() + 0.05;
    }
    let a = (l1 * l1 + d * d - l2 * l2) / (2.0 * d);
    let h = 0.0f64.max(l1 * l1 - a * a).sqrt();
    let (ux, uy) = (dx / d, dy / d);
    (ax + ux * a + uy * h * bend, ay + uy * a - ux * h * bend)
}

pub fn capsule(p: Pt, q: Pt, r1: f64, r2: f64) -> Cells {
    let mut out = Cells::new();
    let x0 = pint((p.0 - r1).min(q.0 - r2)) - 1;
    let x1 = pint((p.0 + r1).max(q.0 + r2)) + 2;
    let y0 = pint((p.1 - r1).min(q.1 - r2)) - 1;
    let y1 = pint((p.1 + r1).max(q.1 + r2)) + 2;
    let (vx, vy) = (q.0 - p.0, q.1 - p.1);
    let mut l2 = vx * vx + vy * vy;
    if l2 == 0.0 {
        l2 = 0.001;
    }
    for y in 0.max(y0)..H.min(y1) {
        for x in 0.max(x0)..W.min(x1) {
            let (px, py) = (x as f64 + 0.5 - p.0, y as f64 + 0.5 - p.1);
            let t = 0.0f64.max(1.0f64.min((px * vx + py * vy) / l2));
            let r = r1 + (r2 - r1) * t;
            let (ex, ey) = (px - vx * t, py - vy * t);
            if ex * ex + ey * ey <= r * r {
                out.insert((x, y));
            }
        }
    }
    out
}

pub fn bez_pts(pts: &[Pt], n: usize) -> Vec<Pt> {
    let mut out = vec![];
    let nn = pts.len();
    for i in 0..nn {
        let (p0, p1, p2, p3) = (
            pts[(i + nn - 1) % nn],
            pts[i],
            pts[(i + 1) % nn],
            pts[(i + 2) % nn],
        );
        for k in 0..n {
            let t = k as f64 / n as f64;
            let (t2, t3) = (t * t, t * t * t);
            let x = 0.5
                * ((2.0 * p1.0)
                    + (-p0.0 + p2.0) * t
                    + (2.0 * p0.0 - 5.0 * p1.0 + 4.0 * p2.0 - p3.0) * t2
                    + (-p0.0 + 3.0 * p1.0 - 3.0 * p2.0 + p3.0) * t3);
            let y = 0.5
                * ((2.0 * p1.1)
                    + (-p0.1 + p2.1) * t
                    + (2.0 * p0.1 - 5.0 * p1.1 + 4.0 * p2.1 - p3.1) * t2
                    + (-p0.1 + 3.0 * p1.1 - 3.0 * p2.1 + p3.1) * t3);
            out.push((x, y));
        }
    }
    out
}

pub fn spine_xy(h: Pt, s: Pt, u: f64, v: f64) -> Pt {
    let (ux, uy) = (s.0 - h.0, s.1 - h.1);
    let mut l = hypot(ux, uy);
    if l == 0.0 {
        l = 1.0;
    }
    let (mut nx, mut ny) = (-uy / l, ux / l);
    if ny < 0.0 {
        nx = -nx;
        ny = -ny;
    }
    (h.0 + ux * u + nx * v, h.1 + uy * u + ny * v)
}

pub struct Parts {
    pub upper: Cells,
    pub lower: Cells,
    pub meta: Cells,
}
#[derive(Clone, Copy)]
pub struct Joints {
    pub top: Pt,
    pub paw: Pt,
    pub fore: bool,
}

pub fn leg_cells(
    top: Pt,
    paw: Pt,
    meta_ang: f64,
    bones: (f64, f64, f64),
    fore: bool,
    near: bool,
) -> (Parts, Joints) {
    let (l1, l2, l3) = bones;
    let a = radians(meta_ang);
    let (jx, jy) = (paw.0 - a.sin() * l3, paw.1 - a.cos() * l3);
    let bend = if fore { -1.0 } else { 1.0 };
    let (kx, ky) = ik2(top.0, top.1, jx, jy, l1, l2, bend);
    let parts = if fore {
        Parts {
            upper: capsule(top, (kx, ky), if near { 5.2 } else { 4.6 }, 3.8),
            lower: capsule((kx, ky), (jx, jy), 3.5, 3.1),
            meta: capsule((jx, jy), paw, 3.0, 2.8),
        }
    } else {
        Parts {
            upper: union(
                &capsule(top, (kx, ky), if near { 7.6 } else { 6.8 }, 4.6),
                &ell(top.0 - 0.5, top.1 + 2.6, 6.4, 6.6),
            ),
            lower: capsule((kx, ky), (jx, jy), 4.0, 3.2),
            meta: capsule((jx, jy), paw, 3.0, 2.8),
        }
    };
    (parts, Joints { top, paw, fore })
}

pub fn draw_leg(
    g: &mut Grid,
    parts: &Parts,
    j: &Joints,
    near: bool,
    toe_dir: f64,
    paw_flat: bool,
    robe: Option<&str>,
) {
    let up = &parts.upper;
    let low = union(&parts.lower, &parts.meta);
    let py0 = j.paw.1;
    let ty = j.top.1;
    let span = 6.0f64.max(py0 - ty);
    let upc: Box<dyn Fn(i32, i32) -> u8>;
    let lowc: Box<dyn Fn(i32, i32) -> u8>;
    if near {
        upc = Box::new(move |x, y| dith(x, y, 0.25 + (y as f64 - ty) / 26.0, b"lLOOD"));
        lowc = Box::new(move |x, y| dith(x, y, 0.30 + 0.55 * (y as f64 - ty) / span, b"LOOD"));
    } else {
        upc = Box::new(move |x, y| dith(x, y, 0.55 + (y as f64 - ty) / 40.0, b"ODDd"));
        lowc = Box::new(move |x, y| dith(x, y, 0.62 + 0.3 * (y as f64 - ty) / span, b"ODd"));
    }
    let upc: Box<dyn Fn(i32, i32) -> u8> = if let Some(rb) = robe {
        let sl = if robe_motif(rb) == "sailor" && j.fore {
            b'h'
        } else {
            b'a'
        };
        Box::new(move |_x, _y| if near { sl } else { b'c' })
    } else {
        upc
    };
    paint(g, &low, &*lowc, Some(b'K'), Some(b".g"));
    if near {
        paint(g, up, &*upc, None, None);
        for &(x, y) in up {
            for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                let n2 = (x + dx, y + dy);
                if !up.contains(&n2) && g.inb(n2.0, n2.1) && in_set(b".g", g.at(n2.0, n2.1)) {
                    g.set(n2.0, n2.1, b'K');
                }
            }
        }
    } else {
        paint(g, up, &*upc, Some(b'K'), Some(b".gnNKd"));
    }
    let (px, py) = j.paw;
    let cells = if paw_flat {
        ell(px + toe_dir * 1.0, py - 0.9, 3.6, 2.3)
    } else {
        ell(px, py, 2.8, 2.8)
    };
    if near {
        paint(
            g,
            &cells,
            &|x, y| {
                if y as f64 > py - 1.6 || !near {
                    b'n'
                } else {
                    dith(x, y, 0.55, b"Dn")
                }
            },
            Some(b'K'),
            Some(b".gnNdDKOLl"),
        );
    } else {
        paint_ch(g, &cells, b'N', Some(b'K'), Some(b".gnNdDKOLl"));
    }
    if paw_flat {
        g.put(pint(px + toe_dir * 2.4), pint(py - 0.6), b'K');
    }
}

#[allow(clippy::too_many_arguments)]
pub fn rig_tails(
    g: &mut Grid,
    base: Pt,
    ang: f64,
    n: i32,
    frame: i32,
    frames: i32,
    sway: f64,
    length: f64,
    lift: f64,
    r: f64,
) {
    let nn = 1.max(9.min(n));
    let t_ = 2.0 * PI * imod(frame, frames) as f64 / frames as f64;
    let (lo, hi) = if nn > 1 {
        (ang - 26.0, ang + 26.0)
    } else {
        (ang, ang)
    };
    let (bx, by) = base;
    let mut lay: Vec<(f64, [Pt; 4])> = vec![];
    for i in 0..nn {
        let fi = i as f64;
        let a = lo + (hi - lo) * fi / (1.max(nn - 1)) as f64 + sway * (t_ + fi * 0.9).sin();
        let l = length - 3.0 * (a - ang).abs() / 30.0 + if i % 2 != 0 { 1.5 } else { 0.0 };
        let da = dvec(a);
        let tip = (bx + da.0 * l, by + da.1 * l - lift);
        let (c1, c2) = (dvec(a + 14.0), dvec(a - 12.0 - 5.0 * (t_ + fi).sin()));
        lay.push((
            a,
            [
                (bx, by),
                (bx + c1.0 * l * 0.45, by + c1.1 * l * 0.45),
                (tip.0 - c2.0 * l * 0.35, tip.1 - c2.1 * l * 0.35),
                tip,
            ],
        ));
    }
    lay.sort_by(|a, b| {
        (-(a.0 - 180.0).abs())
            .partial_cmp(&(-(b.0 - 180.0).abs()))
            .unwrap()
    });
    for (i, (_a, pts)) in lay.iter().enumerate() {
        let i = i as i32;
        tail(
            g,
            pts,
            if nn <= 3 { r } else { r - 0.6 },
            i * 3 + 1,
            (frame + i) % 3,
            4.2,
            0.66,
            b'k',
        );
    }
    k_to_d(g);
}

#[allow(clippy::too_many_arguments)]
fn rig_head(
    g: &mut Grid,
    ox: f64,
    oy: f64,
    lookk: &str,
    mouth_k: &str,
    hx: i32,
    earf: i32,
    acc: &[&str],
    frame: i32,
    ly: i32,
    look: &Look,
) {
    let mut hg = Grid::blank();
    over(&mut hg, &head(hx, earf, look));
    eyes(&mut hg, lookk, 1, ly, hx);
    blush(&mut hg, hx, lookk == "happy", look);
    mouth(&mut hg, mouth_k, 1, 0, hx);
    if has(acc, "glasses") {
        glasses(&mut hg, 1, ly, hx);
    }
    let acc2: Vec<&str> = acc.iter().copied().filter(|a| !is_robe(a)).collect();
    draw_outfit(&mut hg, &acc2, hx, frame);
    let (ox, oy) = (pyround(ox), pyround(oy));
    for y in 0..H {
        let sy = y - oy;
        if !(0 <= sy && sy < H) {
            continue;
        }
        for x in 0..W {
            let sx = x - ox;
            if 0 <= sx && sx < W && hg.at(sx, sy) != b'.' {
                g.set(x, y, hg.at(sx, sy));
            }
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Paw {
    pub x: f64,
    pub y: f64,
    pub ang: f64,
    pub flat: bool,
}
#[derive(Clone, Debug)]
pub struct HeadP {
    pub dx: f64,
    pub dy: f64,
    pub look: &'static str,
    pub mouth: &'static str,
    pub ear: i32,
    pub ly: i32,
}
impl Default for HeadP {
    fn default() -> Self {
        HeadP {
            dx: 0.0,
            dy: 0.0,
            look: "open",
            mouth: "flat",
            ear: 0,
            ly: 0,
        }
    }
}
#[derive(Clone, Debug)]
pub struct Pose {
    pub h: Pt,
    pub s: Pt,
    pub lh: Paw,
    pub lf: Paw,
    pub rh: Paw,
    pub rf: Paw,
    pub tail: (f64, f64, f64, f64),
    pub head: HeadP,
    pub torso: Option<Vec<Pt>>,
    pub shadow: bool,
    pub paws_front: bool,
    pub fx: Option<&'static str>,
}

fn robe_motif_of(rb: Option<&str>) -> &'static str {
    rb.map(robe_motif).unwrap_or("")
}

pub fn robe_rig(g: &mut Grid, body: &Cells, h: Pt, s: Pt, rb: &str) {
    let mo = robe_motif(rb);
    let mut l = hypot(s.0 - h.0, s.1 - h.1);
    if l == 0.0 {
        l = 1.0;
    }
    for &(x, y) in body {
        if !in_set(b"lLOODdWCSsk", g.at(x, y)) {
            continue;
        }
        let (bx, by_) = (x as f64 + 0.5 - h.0, y as f64 + 0.5 - h.1);
        let u = (bx * (s.0 - h.0) + by_ * (s.1 - h.1)) / (l * l);
        let v = (-bx * (s.1 - h.1) + by_ * (s.0 - h.0)) / l;
        let ch;
        if mo == "sailor" {
            let mut c = if u > 0.70 && v < -1.0 {
                if (v + 3.0).abs() < 0.5 { b'f' } else { b'a' }
            } else if u < 0.36 {
                if (x + y) % 9 != 0 { b'a' } else { b'c' }
            } else {
                b'h'
            };
            if u > 1.0 && -1.5 < v && v < 3.5 {
                c = b'i';
            }
            ch = c;
        } else if 0.40 < u && u < 0.60 {
            ch = if (u - 0.50).abs() < 0.025 { b'r' } else { b'i' };
        } else if u > 1.05 && v < 2.5 {
            ch = if v > -1.5 { b'h' } else { b'a' };
        } else {
            ch = motif(x, y, mo).unwrap_or(if v > 5.5 { b'c' } else { b'a' });
        }
        g.set(x, y, ch);
    }
}

pub fn build_rig(
    p: &Pose,
    n: i32,
    frame: i32,
    frames: i32,
    acc: &[&str],
    left: bool,
    look: &Look,
) -> Grid {
    let mut g = Grid::blank();
    let (hh, ss) = (p.h, p.s);
    let rb = robe_of(acc);
    if p.shadow {
        let cx = (hh.0 + ss.0) / 2.0;
        for (x, y) in ell(cx + 2.0, 76.4, 26.0, 1.6) {
            if (x + y) % 2 == 0 {
                g.set(x, y, b'g');
            }
        }
    }
    let root = spine_xy(hh, ss, -0.16, -3.6);
    let (ta, sway, tl, tlift) = p.tail;
    rig_tails(&mut g, root, ta, n, frame, frames, sway, tl, tlift, 8.6);
    let sh = spine_xy(hh, ss, 1.0, 0.0);
    let hip = hh;
    for (paw, hind) in [(p.lh, true), (p.lf, false)] {
        let top = if hind { hip } else { sh };
        let top = (top.0 + if hind { 0.8 } else { -0.6 }, top.1 - 0.6);
        let (parts, j) = leg_cells(
            top,
            (paw.x, paw.y),
            paw.ang,
            if hind { HIND } else { FORE },
            !hind,
            false,
        );
        draw_leg(&mut g, &parts, &j, false, 1.0, paw.flat, rb);
    }
    let tpts: Vec<Pt> = p.torso.clone().unwrap_or_else(|| TORSO.to_vec());
    let pts: Vec<Pt> = tpts.iter().map(|&(u, v)| spine_xy(hh, ss, u, v)).collect();
    let outline = bez_pts(&pts, 6);
    let mut body = poly(&outline);
    for (i, &(x, y)) in outline.iter().enumerate() {
        if i % 3 != 0 {
            continue;
        }
        let (bx, by_) = (x - hh.0, y - hh.1);
        let mut lb = hypot(ss.0 - hh.0, ss.1 - hh.1);
        if lb == 0.0 {
            lb = 1.0;
        }
        let v = (-bx * (ss.1 - hh.1) + by_ * (ss.0 - hh.0)) / lb;
        body.extend(ell(
            x,
            y + if v > 5.0 { 0.9 } else { 0.0 },
            2.1,
            if v > 5.0 { 2.9 } else { 2.1 },
        ));
    }
    let ground = pint(RIG_GROUND);
    let body: Cells = body.into_iter().filter(|c| c.1 <= ground).collect();
    let fur = |x: i32, y: i32| -> u8 {
        let bx = x as f64 + 0.5 - hh.0;
        let by_ = y as f64 + 0.5 - hh.1;
        let mut l = hypot(ss.0 - hh.0, ss.1 - hh.1);
        if l == 0.0 {
            l = 1.0;
        }
        let u = (bx * (ss.0 - hh.0) + by_ * (ss.1 - hh.1)) / (l * l);
        let v = (-bx * (ss.1 - hh.1) + by_ * (ss.0 - hh.0)) / l;
        let under = 4.0 - 3.5 * 0.0f64.max(u - 0.55);
        if v > under && u > 0.25 {
            return dith(x, y, 0.2 + (v - under) / 10.0, b"WWC");
        }
        dith(x, y, 0.20 + (v + 7.0) / 15.0, b"lLOOD")
    };
    paint(&mut g, &body, &fur, Some(b'K'), None);
    for (u, v) in [(0.18, -4.0), (0.42, -5.5), (0.66, -5.0)] {
        let (x, y) = spine_xy(hh, ss, u, v);
        for k in 0..3 {
            let c = (pint(x) - k, pint(y) + k / 2);
            if body.contains(&c) && in_set(b"lLO", g.at(c.0, c.1)) {
                g.put(c.0, c.1, if k != 0 { b'L' } else { b'l' });
            }
        }
    }
    let front = p.paws_front;
    for (paw, hind) in [(p.rh, true), (p.rf, false)] {
        if front && !hind {
            continue;
        }
        let top = if hind { hip } else { sh };
        let top = (top.0 + if hind { -0.5 } else { 0.8 }, top.1 + 0.4);
        let (parts, j) = leg_cells(
            top,
            (paw.x, paw.y),
            paw.ang,
            if hind { HIND } else { FORE },
            !hind,
            true,
        );
        draw_leg(&mut g, &parts, &j, true, 1.0, paw.flat, rb);
    }
    if let Some(rb) = rb {
        robe_rig(&mut g, &body, hh, ss, rb);
    }
    let hd = &p.head;
    let (hx_, hy_) = spine_xy(hh, ss, 1.18, -6.5);
    let ox = hx_ - 38.0 + hd.dx - 1.0;
    let oy = hy_ - 40.0 + hd.dy;
    let nb = spine_xy(hh, ss, 0.95, -2.0);
    let neck = capsule(nb, (ox + 38.0, oy + 38.0), 8.5, 9.5);
    paint(
        &mut g,
        &neck,
        &|x, y| dith(x, y, 0.25 + (y as f64 - oy - 30.0) / 30.0, b"LOOD"),
        Some(b'K'),
        Some(b".gD"),
    );
    if rb.is_none() || robe_motif_of(rb) != "sailor" {
        let rc = (ox + 38.0 + 1.0, oy + 46.0);
        let mut ruff = ell(rc.0, rc.1, 7.5, 6.5);
        for k in 0..7 {
            let a = radians(20.0 + k as f64 * 23.0);
            ruff.extend(ell(rc.0 + a.cos() * 6.8, rc.1 + a.sin() * 5.6, 2.2, 2.6));
        }
        let ruff: Cells = ruff
            .into_iter()
            .filter(|c| c.1 <= ground && !in_set(b"nN", g.at(c.0, c.1)))
            .collect();
        paint(
            &mut g,
            &ruff,
            &|x, y| dith(x, y, 0.15 + (y as f64 - rc.1 + 6.0) / 22.0, b"WWWC"),
            Some(b'K'),
            Some(b".gK"),
        );
    }
    rig_head(
        &mut g, ox, oy, hd.look, hd.mouth, 2, hd.ear, acc, frame, hd.ly, look,
    );
    let (cx_, cy_) = (ox + 38.0, oy + 44.0);
    for k in -6..7 {
        let x = pint(cx_ + k as f64);
        let y = pint(cy_ + pyround(pw(k as f64 / 6.0, 2.0) * 1.5) as f64);
        if 0 <= y && y < H - 1 && 0 <= x && x < W && !in_set(b".g", g.at(x, y)) {
            g.put(x, y, b'M');
            g.put(x, y + 1, b'm');
        }
    }
    stampd(
        &mut g,
        pint(cx_) - 2,
        pint(cy_) + 1,
        &[".KK.", "KXxK", "KJXK", ".KK."],
    );
    if front {
        for (paw, near) in [(p.lf, false), (p.rf, true)] {
            let top = (
                sh.0 + if near { 0.8 } else { -0.6 },
                sh.1 + if near { 0.4 } else { -0.6 },
            );
            let (mut parts, j) = leg_cells(top, (paw.x, paw.y), paw.ang, FORE, true, near);
            parts.upper = if near {
                parts
                    .upper
                    .iter()
                    .copied()
                    .filter(|c| in_set(b".gKnNdD", g.at(c.0, c.1)))
                    .collect()
            } else {
                Cells::new()
            };
            draw_leg(&mut g, &parts, &j, true, 1.0, paw.flat, None);
        }
    }
    if let Some(fx) = p.fx {
        effects(&mut g, fx, frame);
    }
    seal(&mut g);
    if left {
        g.mirror_x();
    }
    g
}

// ---- the walk ---------------------------------------------------------------------------------------
fn walk_leg(
    ph: f64,
    offset: f64,
    base_x: f64,
    stride: f64,
    duty: f64,
    lift_h: f64,
    fore: bool,
) -> Paw {
    let s = pymod(ph - offset, 1.0);
    if s < duty {
        let u = s / duty;
        let x = base_x + stride * (0.5 - u);
        let ang = (if fore { 14.0 } else { 8.0 }) - (if fore { 28.0 } else { 24.0 }) * u * u;
        return Paw {
            x,
            y: RIG_GROUND - 0.6,
            ang,
            flat: true,
        };
    }
    let u = (s - duty) / (1.0 - duty);
    let x = base_x + stride * (-0.5 + u * u * (3.0 - 2.0 * u));
    let lift = lift_h * pw((PI * u).sin(), 0.8);
    let ang = if fore {
        -70.0 * (PI * 1.0f64.min(u * 1.25)).sin()
    } else {
        -45.0 * (PI * 1.0f64.min(u * 1.15)).sin()
    };
    Paw {
        x,
        y: RIG_GROUND - 0.6 - lift,
        ang,
        flat: u > 0.88,
    }
}

pub fn walk_rig_pose(frame: i32, frames: i32, hop: f64, look: &'static str, xo: f64) -> Pose {
    let ph = imod(frame, frames) as f64 / frames as f64;
    let duty = 0.62;
    let bounce = hop * 1.6 * 0.0f64.max((2.0 * PI * (2.0 * ph - 0.10)).sin());
    let roll = 0.6 * (2.0 * PI * 2.0 * ph + 1.2).sin();
    let h = (44.0 + xo, 50.2 - bounce + roll * 0.4);
    let s = (71.0 + xo, 50.8 - bounce - roll * 0.4);
    let nod = 0.8 * (2.0 * PI * 2.0 * ph + 2.2).sin();
    Pose {
        h,
        s,
        lh: walk_leg(ph, 0.00, 45.5 + xo, 12.0, duty, 3.6 + hop, false),
        lf: walk_leg(ph, 0.25, 72.5 + xo, 12.0, duty, 3.6 + hop, true),
        rh: walk_leg(ph, 0.50, 44.0 + xo, 12.0, duty, 3.6 + hop, false),
        rf: walk_leg(ph, 0.75, 71.0 + xo, 12.0, duty, 3.6 + hop, true),
        tail: (166.0 + 5.0 * (2.0 * PI * ph).sin(), 4.0, 32.0, 0.0),
        head: HeadP {
            dx: 0.0,
            dy: nod - bounce * 0.3,
            look,
            mouth: if look == "happy" { "grin" } else { "flat" },
            ear: 0,
            ly: 0,
        },
        torso: None,
        shadow: true,
        paws_front: false,
        fx: None,
    }
}

fn pl(x: f64, y: Option<f64>, ang: f64, flat: bool) -> Paw {
    Paw {
        x,
        y: y.unwrap_or(RIG_GROUND - 0.6),
        ang,
        flat,
    }
}
fn pa(x: f64, y: f64, ang: f64, flat: bool) -> Paw {
    Paw { x, y, ang, flat }
}
fn hp(dx: f64, dy: f64, look: &'static str, mouth: &'static str, ear: i32, ly: i32) -> HeadP {
    HeadP {
        dx,
        dy,
        look,
        mouth,
        ear,
        ly,
    }
}
fn pose(
    h: Pt,
    s: Pt,
    lh: Paw,
    lf: Paw,
    rh: Paw,
    rf: Paw,
    tail: (f64, f64, f64, f64),
    head: HeadP,
) -> Pose {
    Pose {
        h,
        s,
        lh,
        lf,
        rh,
        rf,
        tail,
        head,
        torso: None,
        shadow: true,
        paws_front: false,
        fx: None,
    }
}

pub fn rig_pose(name: &str, frame: i32, frames: i32) -> Pose {
    let ph = imod(frame, frames) as f64 / frames as f64;
    let w = (2.0 * PI * ph).sin();
    let g8 = Some(RIG_GROUND - 0.6);
    let _ = g8;
    match name {
        "stand" => pose(
            (44.0, 50.2),
            (71.0, 50.8),
            pl(46.5, None, 8.0, true),
            pl(74.0, None, 10.0, true),
            pl(43.0, None, 8.0, true),
            pl(71.0, None, 10.0, true),
            (170.0, 3.0, 31.0, 0.0),
            HeadP::default(),
        ),
        "stretch_front" => {
            let mut p = pose(
                (42.0, 47.0),
                (70.0, 62.0),
                pl(45.0, None, 4.0, true),
                pl(92.0, None, 88.0, true),
                pl(42.5, None, 4.0, true),
                pl(86.0, None, 88.0, true),
                (138.0, 2.0, 30.0, 0.0),
                hp(9.0, 3.0, "blink", "open", 1, 0),
            );
            p.paws_front = true;
            p
        }
        "playbow" => {
            let mut p = pose(
                (42.0, 47.0 - 0.5 * w),
                (70.0, 61.0),
                pl(45.0, None, 4.0, true),
                pl(92.0, None, 88.0, true),
                pl(42.5, None, 4.0, true),
                pl(86.0, None, 88.0, true),
                (128.0, 14.0, 30.0, 0.0),
                hp(8.0, 0.0, "happy", "grin", 0, 0),
            );
            p.paws_front = true;
            p.fx = if imod(frame, 4) < 2 {
                Some("spark")
            } else {
                None
            };
            p
        }
        "stretch_back" => pose(
            (42.0, 53.0),
            (70.0, 49.0),
            pl(41.0, None, 2.0, true),
            pl(70.0, None, -6.0, true),
            pa(22.0, 71.5, -78.0, false),
            pl(67.5, None, -6.0, true),
            (176.0, 2.0, 31.0, 0.0),
            hp(0.0, 1.0, "blink", "flat", 0, 0),
        ),
        "lie" => {
            let b = 0.4 * w;
            let mut p = pose(
                (40.0, 63.5 - b),
                (68.0, 62.0 - b),
                pl(53.0, None, 84.0, true),
                pl(92.0, None, 88.0, true),
                pl(51.0, None, 84.0, true),
                pl(86.0, None, 88.0, true),
                (186.0, 2.0, 30.0, -3.0),
                hp(
                    9.0,
                    2.0,
                    if imod(frame, 8) != 0 { "open" } else { "blink" },
                    "flat",
                    0,
                    0,
                ),
            );
            p.torso = Some(torso_lie());
            p.paws_front = true;
            p
        }
        "doze" => {
            let mut p = rig_pose("lie", frame, frames);
            p.head.look = "sleep";
            p
        }
        "crouch" => {
            let k = 0.6 * w;
            pose(
                (42.0, 55.5 + k * 0.3),
                (70.0, 57.5),
                pl(47.0, None, 40.0, true),
                pl(76.0, None, 30.0, true),
                pl(45.0, None, 40.0, true),
                pl(74.0, None, 30.0, true),
                (182.0 + 4.0 * w, 1.0, 31.0, -2.0),
                hp(2.0, 2.0, "focus", "flat", 0, 1),
            )
        }
        "rear" => pose(
            (44.0, 54.0),
            (66.0, 43.0),
            pl(46.0, None, 30.0, true),
            pa(70.0, 58.0, -60.0, false),
            pl(44.0, None, 30.0, true),
            pa(68.0, 59.0, -60.0, false),
            (196.0, 2.0, 31.0, 0.0),
            hp(-2.0, 2.0, "focus", "flat", 0, 1),
        ),
        "leap" => {
            let mut p = pose(
                (42.0, 47.0),
                (70.0, 50.0),
                pa(37.0, 66.0, -50.0, false),
                pa(78.0, 63.0, -40.0, false),
                pa(35.0, 66.0, -50.0, false),
                pa(76.0, 64.0, -40.0, false),
                (172.0, 2.0, 31.0, 4.0),
                hp(0.0, 2.0, "focus", "flat", 0, 1),
            );
            p.shadow = false;
            p
        }
        "dive" => {
            let mut p = pose(
                (40.0, 44.0),
                (66.0, 56.0),
                pa(28.0, 52.0, -70.0, false),
                pl(80.0, Some(72.0), 30.0, true),
                pa(26.0, 53.0, -70.0, false),
                pl(78.0, Some(72.0), 30.0, true),
                (150.0, 2.0, 31.0, 0.0),
                hp(7.0, 2.0, "angry", "open", 0, 0),
            );
            p.shadow = false;
            p.paws_front = true;
            p
        }
        "land" => {
            let mut p = pose(
                (42.0, 48.0),
                (70.0, 61.0),
                pl(46.0, None, 6.0, true),
                pl(90.0, None, 60.0, true),
                pl(44.0, None, 6.0, true),
                pl(85.0, None, 60.0, true),
                (140.0, 6.0, 30.0, 0.0),
                hp(8.0, 3.0, "happy", "grin", 0, 0),
            );
            p.paws_front = true;
            p
        }
        "sniff" => {
            let mut p = walk_rig_pose(frame, frames, 0.0, "open", 0.0);
            p.head.dy += 4.0;
            p.head.dx = 2.0;
            p.head.ly = 1;
            p.head.look = "open";
            p.tail = (182.0, 3.0, 31.0, -2.0);
            p
        }
        "shake" => {
            let mut p = rig_pose("stand", 0, 8);
            let s = if frame % 2 != 0 { 1.0 } else { -1.0 };
            p.head = hp(1.5 * s, 0.0, "happy", "flat", 0, 0);
            p.tail = (165.0 + 12.0 * s, 0.0, 31.0, 0.0);
            p.fx = Some("spark");
            p
        }
        _ => rig_pose("stand", 0, 8),
    }
}

/// 'qwalkR3' (walking, frame 3) or 'rigL:stretch_front:0' (a pose, facing left, frame 0) -> sprite rows.
pub fn rig_rows(st: &str, n: i32, acc: &[&str], look: &Look) -> Grid {
    let acc: Vec<&str> = acc
        .iter()
        .copied()
        .filter(|a| {
            is_robe(a) || ["flower", "bow", "crown", "headband", "mask", "glasses"].contains(a)
        })
        .collect();
    if let Some(rest) = st.strip_prefix("qwalk") {
        let left = rest.as_bytes().first() == Some(&b'L');
        let fr: i32 = rest
            .get(1..)
            .and_then(|s| s.trim().parse().ok())
            .unwrap_or(0);
        return build_rig(
            &walk_rig_pose(fr, 8, 1.0, "open", 0.0),
            n,
            fr,
            8,
            &acc,
            left,
            look,
        );
    }
    let parts: Vec<&str> = st.split(':').collect();
    let (head_, name, fr) = match (parts.len() == 3)
        .then(|| parts[2].trim().parse::<i32>().ok())
        .flatten()
    {
        Some(fr) => (parts[0], parts[1], fr),
        None => ("rigR", "stand", 0),
    };
    let left = head_.ends_with('L');
    if name == "walk" {
        return build_rig(
            &walk_rig_pose(fr, 8, 1.0, "open", 0.0),
            n,
            fr,
            8,
            &acc,
            left,
            look,
        );
    }
    build_rig(&rig_pose(name, fr, 8), n, fr, 8, &acc, left, look)
}
