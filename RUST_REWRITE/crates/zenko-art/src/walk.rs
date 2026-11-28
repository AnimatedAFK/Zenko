//! Four-legged walks: build_walk4 (side view) and build_walk34 (three-quarter view), from zenko.py.

use crate::core::*;
use crate::sit::{Look, blush, eyes, head, mouth};
use std::f64::consts::PI;

fn walk_tails(
    g: &mut Grid,
    frame: i32,
    n: i32,
    t_: f64,
    bx: f64,
    by: f64,
    lo_hi: (f64, f64),
    single: f64,
    mid: f64,
    spread: f64,
    base_len: f64,
    wave: f64,
    r_small: f64,
    r_big: f64,
) {
    let nn = 1.max(9.min(n));
    let (lo, hi) = if nn > 1 { lo_hi } else { (single, single) };
    let mut lay: Vec<(f64, [Pt; 4])> = vec![];
    for i in 0..nn {
        let fi = i as f64;
        let a = lo + (hi - lo) * fi / (1.max(nn - 1)) as f64 + wave * (t_ + fi * 0.9).sin();
        let l = base_len - 3.0 * (a - mid).abs() / spread + if i % 2 != 0 { 1.5 } else { 0.0 };
        let da = dvec(a);
        let tip = (bx + da.0 * l, by + da.1 * l);
        let (c1, c2) = (dvec(a + 26.0), dvec(a - 30.0 - 6.0 * (t_ + fi).sin()));
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
    lay.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
    for (i, (_a, pts)) in lay.iter().enumerate() {
        let i = i as i32;
        tail(
            g,
            pts,
            if nn <= 3 { r_small } else { r_big },
            i * 3 + 1,
            (frame + i) % 3,
            3.0,
            0.66,
            b'k',
        );
    }
    k_to_d(g);
}

pub fn build_walk4(frame: i32, n: i32, frames: i32, blink: bool, left: bool) -> Grid {
    let mut g = Grid::blank();
    let ph = imod(frame, frames) as f64 / frames as f64;
    let t_ = 2.0 * PI * ph;
    let bob = pyround(0.9 * (2.0 * t_).sin().abs()) as f64;
    let nod = if (2.0 * t_ + 0.9).sin() > 0.45 {
        1.0
    } else {
        0.0
    };
    for (x, y) in ell(54.0, 76.6, 28.0, 1.6) {
        if (x + y) % 2 == 0 {
            g.set(x, y, b'g');
        }
    }
    walk_tails(
        &mut g,
        frame,
        n,
        t_,
        35.0,
        56.0 - bob,
        (118.0, 166.0),
        142.0,
        142.0,
        24.0,
        27.0,
        4.0,
        7.6,
        7.0,
    );

    let leg = |g: &mut Grid, px: f64, py: f64, offset: f64, near: bool| {
        let s = pymod(ph + offset, 1.0);
        let (ang, lift) = if s < 0.6 {
            (0.42 - 0.84 * (s / 0.6), 0.0)
        } else {
            let u = (s - 0.6) / 0.4;
            (-0.42 + 0.84 * u, 3.0 * (PI * u).sin())
        };
        let fx = px + 9.0 * ang.sin();
        let fy = 73.8 - lift;
        let mut cells = Cells::new();
        for k in 0..13 {
            let t = k as f64 / 12.0;
            let (cx, cy) = (px + (fx - px) * t, py + (fy - py) * t);
            let r = 3.4 - 0.6 * t;
            cells.extend(ell(cx, cy, r, r));
        }
        if near {
            paint(
                g,
                &cells,
                &|x, y| dith(x, y, 0.3 + (y as f64 - py) / 30.0, b"LOD"),
                Some(b'K'),
                None,
            );
        } else {
            paint(g, &cells, &|x, y| dith(x, y, 0.6, b"Dd"), Some(b'K'), None);
        }
        let p = ell(fx + 1.2, fy + 0.8, 3.4, 2.0);
        paint_ch(g, &p, if near { b'n' } else { b'C' }, Some(b'K'), None);
        g.put(pint(fx + 1.2), pint(fy + 2.0), b'K');
    };
    let by0 = 62.0 - bob;
    leg(&mut g, 61.0, by0, 0.75, false);
    leg(&mut g, 38.0, by0, 0.50, false);
    let cy = 57.5 - bob;
    let body = ell(49.0, cy, 17.0, 9.5);
    paint(
        &mut g,
        &body,
        &lit(44.0, cy - 6.0, 22.0, 14.0, b"lLOODd", 0.02, 3.0),
        Some(b'K'),
        None,
    );
    let belly: Cells = ell(56.0, cy + 5.0, 11.0, 4.8)
        .into_iter()
        .filter(|c| body.contains(c))
        .collect();
    paint(
        &mut g,
        &belly,
        &|x, y| dith(x, y, 0.25 + (y as f64 - cy) / 30.0, b"WC"),
        None,
        None,
    );
    leg(&mut g, 38.0, by0, 0.0, true);
    leg(&mut g, 61.0, by0, 0.25, true);
    let hy = 43.0 - bob + nod;
    let hx = 75.0;
    let neck = poly(&[
        (55.0, 50.0 - bob),
        (66.0, hy + 2.0),
        (72.0, hy + 9.0),
        (66.0, 64.0 - bob),
        (56.0, 62.0 - bob),
    ]);
    paint(
        &mut g,
        &neck,
        &lit(62.0, hy + 2.0, 14.0, 14.0, b"LOD", 0.0, 3.0),
        Some(b'K'),
        Some(b".g"),
    );
    paint(
        &mut g,
        &ell(64.5, 57.0 - bob, 5.0, 5.5),
        &|x, y| dith(x, y, 0.2, b"WC"),
        None,
        None,
    );
    for k in 0..9 {
        g.put(
            pint(64.0 + k as f64 * 0.55),
            pint(hy + 7.0 + k as f64 * 0.9),
            b'M',
        );
    }
    stampd(
        &mut g,
        67,
        pint(hy + 14.0),
        &[".KK.", "KXxK", "KJXK", ".KK."],
    );
    let ears: [([Pt; 3], [Pt; 3]); 2] = [
        (
            [(63.0, hy - 6.0), (64.5, hy - 25.0), (75.0, hy - 11.0)],
            [(65.5, hy - 9.0), (66.0, hy - 21.0), (72.0, hy - 11.0)],
        ),
        (
            [(72.0, hy - 11.0), (81.5, hy - 27.0), (88.0, hy - 8.0)],
            [(75.0, hy - 11.0), (81.5, hy - 23.0), (85.0, hy - 10.0)],
        ),
    ];
    for (pts, inner) in ears.iter() {
        let top = hy - 19.0;
        paint(
            &mut g,
            &poly(pts),
            &|_x, y| if y as f64 <= top { b't' } else { b'O' },
            Some(b'K'),
            None,
        );
        paint_ch(&mut g, &poly(inner), b'P', None, None);
    }
    let hd = ell(hx, hy, 14.5, 12.5);
    let tone = lit(hx - 4.0, hy - 6.0, 18.0, 15.0, b"lLOOD", 0.0, 3.0);
    paint(
        &mut g,
        &hd,
        &|x, y| {
            if y as f64 > hy + 4.0 + (hx - x as f64) * 0.35 && x as f64 > hx - 9.0 {
                dith(x, y, 0.15, b"WC")
            } else {
                tone(x, y)
            }
        },
        Some(b'K'),
        Some(b".gD"),
    );
    for (x, y) in [
        (hx - 13.0, hy + 7.0),
        (hx - 12.0, hy + 9.0),
        (hx - 10.0, hy + 10.0),
    ] {
        g.put(pint(x), pint(y), b'W');
    }
    let snout = ell(hx + 14.5, hy + 4.5, 5.0, 3.4);
    paint(
        &mut g,
        &snout,
        &|x, y| dith(x, y, 0.2, b"WC"),
        Some(b'K'),
        Some(b".g"),
    );
    let (nx, ny) = (pint(hx + 19.0), pint(hy + 3.0));
    for (x, y) in [(nx, ny), (nx + 1, ny), (nx, ny + 1), (nx + 1, ny + 1)] {
        g.put(x, y, b'B');
    }
    g.put(nx, ny, b'H');
    for (x, y) in [
        (pint(hx + 14.0), pint(hy + 8.0)),
        (pint(hx + 15.0), pint(hy + 9.0)),
        (pint(hx + 16.0), pint(hy + 8.0)),
    ] {
        g.put(x, y, b'K');
    }
    let (ex, ey) = (pint(hx + 3.0), pint(hy - 5.0));
    if blink {
        for (x, y) in [
            (ex, ey + 5),
            (ex + 1, ey + 6),
            (ex + 2, ey + 6),
            (ex + 3, ey + 6),
            (ex + 4, ey + 6),
            (ex + 5, ey + 5),
        ] {
            g.put(x, y, b'B');
        }
    } else {
        let shape = [
            ".BBBB.", "BeeeeB", "BeBBeB", "BeBBeB", "BqBBqB", "BqqqqB", ".BBBB.",
        ];
        for (r, row) in shape.iter().enumerate() {
            for (c, ch) in row.bytes().enumerate() {
                if ch != b'.' {
                    g.put(ex + c as i32, ey + r as i32, ch);
                }
            }
        }
        g.put(ex + 1, ey + 1, b'H');
        g.put(ex + 2, ey + 2, b'H');
        g.put(ex + 4, ey + 4, b'H');
        g.put(ex + 5, ey - 1, b'B');
        g.put(ex + 6, ey - 1, b'B');
        g.put(ex + 6, ey - 2, b'B');
        g.put(ex + 6, ey, b'B');
    }
    for (x, y) in [
        (ex + 3, ey + 9),
        (ex + 4, ey + 9),
        (ex + 5, ey + 9),
        (ex + 4, ey + 8),
    ] {
        g.put(x, y, b'P');
    }
    seal(&mut g);
    if left {
        g.mirror_x();
    }
    g
}

pub fn build_walk34(
    frame: i32,
    n: i32,
    frames: i32,
    blink: bool,
    left: bool,
    lookk: &str,
    look: &Look,
) -> Grid {
    let mut g = Grid::blank();
    let ph = imod(frame, frames) as f64 / frames as f64;
    let t_ = 2.0 * PI * ph;
    let bob = if (2.0 * t_).sin() > 0.2 { 1 } else { 0 };
    let bobf = bob as f64;
    for (x, y) in ell(52.0, 76.2, 27.0, 1.8) {
        if (x + y) % 2 == 0 {
            g.set(x, y, b'g');
        }
    }
    walk_tails(
        &mut g,
        frame,
        n,
        t_,
        30.0,
        57.0 - bobf,
        (112.0, 168.0),
        138.0,
        140.0,
        28.0,
        25.0,
        5.0,
        7.4,
        6.8,
    );
    let leg = |g: &mut Grid, px: f64, py: f64, offset: f64, near: bool, length: f64| {
        let s = pymod(ph + offset, 1.0);
        let (dx, lift) = if s < 0.55 {
            (2.2 - 4.4 * (s / 0.55), 0.0)
        } else {
            let u = (s - 0.55) / 0.45;
            (-2.2 + 4.4 * u, 2.6 * (PI * u).sin())
        };
        let (fx, fy) = (px + dx, py + length - lift);
        let mut cells = Cells::new();
        for k in 0..10 {
            let t = k as f64 / 8.0;
            let (cx, cy) = (px + (fx - px) * t * 0.8, py + (fy - py) * t);
            let r = 3.3 - 0.5 * t;
            cells.extend(ell(cx, cy, r, r));
        }
        if near {
            paint(
                g,
                &cells,
                &|x, y| dith(x, y, 0.25 + (y as f64 - py) / 28.0, b"LOD"),
                Some(b'K'),
                None,
            );
        } else {
            paint(g, &cells, &|x, y| dith(x, y, 0.6, b"Dd"), Some(b'K'), None);
        }
        let p = ell(fx + 0.6, fy + 0.6, 3.5, 2.1);
        paint_ch(g, &p, if near { b'n' } else { b'C' }, Some(b'K'), None);
        g.put(pint(fx), pint(fy + 2.0), b'K');
        g.put(pint(fx + 2.0), pint(fy + 2.0), b'K');
    };
    let by0 = 62.0 - bobf;
    leg(&mut g, 55.0, by0, 0.5, false, 10.5);
    leg(&mut g, 31.0, by0, 0.0, false, 10.5);
    leg(&mut g, 37.0, by0, 0.5, true, 11.5);
    leg(&mut g, 61.0, by0, 0.0, true, 11.5);
    let cy = 58.5 - bobf;
    let body = ell(46.0, cy, 18.5, 8.6);
    paint(
        &mut g,
        &body,
        &lit(40.0, cy - 6.0, 24.0, 14.0, b"lLOODd", 0.02, 3.0),
        Some(b'K'),
        None,
    );
    let belly: Cells = ell(50.0, cy + 6.0, 13.0, 4.0)
        .into_iter()
        .filter(|c| body.contains(c))
        .collect();
    paint(
        &mut g,
        &belly,
        &|x, y| dith(x, y, 0.3 + (y as f64 - cy) / 30.0, b"WC"),
        None,
        None,
    );
    let chest = ell(59.5, cy + 0.5, 7.5, 7.0);
    paint(
        &mut g,
        &chest,
        &|x, y| dith(x, y, 0.15 + (y as f64 - cy) / 25.0, b"WWC"),
        Some(b'K'),
        Some(b".g"),
    );
    let mut hg = Grid::blank();
    let hx = 2;
    let earf = if imod(frame, frames) == 3 { 1 } else { 0 };
    over(&mut hg, &head(hx, earf, look));
    eyes(&mut hg, if blink { "blink" } else { lookk }, 1, 0, hx);
    blush(&mut hg, hx, lookk == "happy", look);
    mouth(
        &mut hg,
        if lookk == "happy" { "grin" } else { "flat" },
        1,
        0,
        hx,
    );
    let (ox, oy) = (23, 10 - bob);
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
    for k in -6..7 {
        let x = 59 + k;
        let y = 54 - bob + pyround(pw(k as f64 / 6.0, 2.0) * 1.5);
        if g.at(x, y) != b'.' {
            g.put(x, y, b'M');
            g.put(x, y + 1, b'm');
        }
    }
    stampd(&mut g, 57, 55 - bob, &[".KK.", "KXxK", "KJXK", ".KK."]);
    seal(&mut g);
    if left {
        g.mirror_x();
    }
    g
}
