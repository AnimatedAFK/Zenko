//! The sitting, front-facing Zenko: body + ema plaque, head, face, effects, accessories, robes.
//! Ported from zenko.py: _paw, _body, _ear, _head, eyes, blush, mouth, effects, effects2, glasses,
//! headphones, _lick_paw, _highfive_paw, draw_outfit, _head_ears, robe_sit, build_frame, z19_sit_extra.

use crate::core::*;
use crate::palette::{robe_motif, robe_of};
use std::f64::consts::PI;

/// Ear / blush style (zenko.py's module dict LOOK).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Look {
    pub ears: String,
    pub blush: String,
}
impl Default for Look {
    fn default() -> Self {
        Look {
            ears: "pointy".into(),
            blush: "soft".into(),
        }
    }
}

pub fn has(acc: &[&str], a: &str) -> bool {
    acc.iter().any(|x| *x == a)
}

// ---- body: sitting up, holding a little wooden ema plaque ------------------------------------------
pub fn walk_feet(pose: &str) -> Option<[(f64, f64, f64); 2]> {
    // index 0 = side -1, index 1 = side +1
    Some(match pose {
        "walk0" => [(0.0, 0.5, 1.08), (2.5, -4.0, 0.94)],
        "walk1" => [(0.0, 0.0, 1.04), (4.5, -5.5, 0.9)],
        "walk2" => [(2.5, -4.0, 0.94), (0.0, 0.5, 1.08)],
        "walk3" => [(4.5, -5.5, 0.9), (0.0, 0.0, 1.04)],
        _ => return None,
    })
}
const PLAQUE: [Pt; 6] = [
    (17.0, 55.5),
    (22.5, 51.5),
    (49.5, 51.5),
    (55.0, 55.5),
    (55.0, 71.5),
    (17.0, 71.5),
];

pub fn paw(g: &mut Grid, cx: f64, cy: f64, rx: f64, ry: f64, socks: bool) {
    let p = ell(cx, cy, rx, ry);
    paint(
        g,
        &p,
        &|x, y| {
            if socks {
                if y as f64 >= cy || (x + y) % 2 != 0 {
                    b'n'
                } else {
                    b'N'
                }
            } else if (y as f64) < cy - 0.5 {
                b'L'
            } else {
                b'O'
            }
        },
        Some(b'K'),
        None,
    );
    for tx in [cx - 1.2, cx + 0.8] {
        g.put(pint(tx), pint(cy + ry - 1.0), b'K');
    }
}

pub fn body(pose: &str, bib: bool) -> Grid {
    let mut g = Grid::blank();
    let fur = lit(CX - 6.0, 54.0, 22.0, 20.0, b"LOODd", 0.05, 3.0);
    let lift = walk_feet(pose).unwrap_or([(0.0, 0.0, 1.0), (0.0, 0.0, 1.0)]);
    for (si, sgn) in [(0usize, -1.0f64), (1, 1.0)] {
        let hx = CX + sgn * 12.5;
        let up = lift[si].0 * 0.45;
        let tone = lit(
            hx - 4.0,
            62.0 - up,
            12.0,
            12.0,
            b"LODd",
            if sgn > 0.0 { 0.08 } else { 0.0 },
            3.0,
        );
        paint(
            &mut g,
            &ell(hx, 67.5 - up, 9.0, 8.5),
            &tone,
            Some(b'K'),
            None,
        );
    }
    for (si, sgn) in [(0usize, -1.0f64), (1, 1.0)] {
        let (up, tuck, size) = lift[si];
        paw(
            &mut g,
            CX + sgn * (15.5 - tuck),
            74.4 - up,
            5.2 * size,
            2.6 * (0.9 + 0.1 * size),
            true,
        );
    }
    paint(&mut g, &ell(CX, 59.0, 15.5, 15.0), &fur, Some(b'K'), None);
    let chest = union(
        &ell(CX, 51.0, 9.5, 8.0),
        &mir(&poly(&[
            (27.0, 46.0),
            (31.0, 56.0),
            (36.0, 58.5),
            (36.0, 46.0),
        ])),
    );
    paint(
        &mut g,
        &chest,
        &|x, y| {
            dith(
                x,
                y,
                0.0f64.max(
                    pw((x as f64 + 0.5 - CX) / 10.0, 2.0) + pw((y as f64 + 0.5 - 50.0) / 9.0, 2.0)
                        - 0.45,
                ) * 1.6,
                b"WWC",
            )
        },
        None,
        None,
    );
    for (x, y) in [(30, 50), (31, 52), (30, 53)] {
        g.put2(x, y, b'C');
    }
    for x in 25..47 {
        let yb = 44 + pyround(pw((x as f64 + 0.5 - CX) / 11.0, 2.0) * 2.0);
        g.put(x, yb, b'M');
        g.put(x, yb + 1, b'm');
    }
    if bib {
        let bibc = poly(&[
            (24.5, 44.5),
            (47.5, 44.5),
            (44.5, 50.0),
            (36.0, 53.5),
            (27.5, 50.0),
        ]);
        paint(
            &mut g,
            &bibc,
            &|x, y| {
                if y >= 51 || imod(x - y, 6) == 0 {
                    b's'
                } else {
                    b'S'
                }
            },
            Some(b'K'),
            Some(b"WCLODdlKkMm"),
        );
        for x in 26..46 {
            g.put(
                x,
                45 + if x == 26 || x == 45 { 1 } else { 0 },
                if x % 3 == 0 { b'x' } else { b'S' },
            );
        }
    }
    stampd(
        &mut g,
        33,
        45,
        &[".KJJK.", "KXxXXK", "KXxXJK", "KJXXJK", ".KBBK.", "..KK.."],
    );
    for sgn in [-1.0f64, 1.0] {
        if pose == "lick" && sgn < 0.0 {
            continue;
        }
        let lx = CX + sgn * 6.5;
        let mut leg = Cells::new();
        for y in 49..55 {
            for x in pint(lx - 2.5)..pint(lx + 2.5) {
                leg.insert((x, y));
            }
        }
        paint(
            &mut g,
            &leg,
            &|x, y| {
                if y >= 53 {
                    b'n'
                } else if (x as f64 + 0.5 - CX).abs() < 6.0 {
                    b'W'
                } else {
                    b'L'
                }
            },
            Some(b'k'),
            Some(b"WCLODdl"),
        );
    }
    let pl = poly(&PLAQUE);
    paint(
        &mut g,
        &pl,
        &|x, y| {
            if y == 70 || x == 17 || x == 54 {
                b'j'
            } else if y == 69
                || x == 18
                || x == 53
                || y as f64 <= 53.0 + 0.max(21 - x) as f64 * 0.7
                || y as f64 <= 53.0 + 0.max(x - 50) as f64 * 0.7
            {
                b'o'
            } else {
                b'w'
            }
        },
        Some(b'K'),
        None,
    );
    for x in 17..55 {
        let yt = pl.iter().filter(|c| c.0 == x).map(|c| c.1).min().unwrap();
        g.put(x, yt, b'j');
        g.put(x, yt + 1, if x == 17 || x == 54 { b'j' } else { b'o' });
    }
    for x in 19..53 {
        if (x * 5) % 11 == 0 {
            g.put(x, 70, b'o');
        }
    }
    for (x, y) in [(24, 54), (47, 54)] {
        g.put(x, y, b'M');
    }
    for sgn in [-1.0f64, 1.0] {
        if pose == "lick" && sgn < 0.0 {
            continue;
        }
        paw(&mut g, CX + sgn * 6.5, 53.4, 3.6, 2.5, true);
    }
    g
}

// ---- head ------------------------------------------------------------------------------------------
fn ear(sgn: i32, flick: f64) -> (Vec<Pt>, Vec<Pt>) {
    let a = radians(-14.0 * flick);
    let e = |x: f64, y: f64| -> Pt {
        let (x, y) = (x - 23.0, y - 22.0);
        let (x, y) = (
            23.0 + x * a.cos() - y * a.sin(),
            22.0 + x * a.sin() + y * a.cos(),
        );
        if sgn < 0 { (x, y) } else { (2.0 * CX - x, y) }
    };
    let outer = vec![
        e(15.5, 27.0),
        e(30.5, 18.0),
        e(24.5, 9.0),
        e(16.5, 1.5),
        e(13.5, 2.5),
        e(12.0, 12.0),
    ];
    let inner = vec![
        e(17.5, 23.5),
        e(27.0, 18.0),
        e(22.0, 11.5),
        e(16.0, 5.5),
        e(14.5, 12.5),
    ];
    (outer, inner)
}

pub fn head(hx: i32, earf: i32, look: &Look) -> Grid {
    let mut g = Grid::blank();
    let flop = if look.ears == "floppy" { 3.0 } else { 0.0 };
    for sgn in [-1i32, 1] {
        let fl = (if sgn > 0 { earf } else { 0 }) as f64 + flop;
        let (o, i) = ear(sgn, fl);
        let oc = shift(&poly(&o), hx, 0);
        let tipy = pint(7.0 + fl * 3.0);
        paint(
            &mut g,
            &oc,
            &|x, y| {
                if y <= tipy {
                    b't'
                } else if sgn < 0 && x <= 17 + hx {
                    b'L'
                } else {
                    b'O'
                }
            },
            Some(b'K'),
            None,
        );
        paint(
            &mut g,
            &shift(&poly(&i), hx, 0),
            &|_x, y| if y < 12 { b'p' } else { b'P' },
            None,
            None,
        );
        for (fx, fy) in [(19, 22), (20, 21), (21, 20), (18, 20), (19, 19), (22, 21)] {
            let x = if sgn < 0 { fx } else { mx(fx) };
            g.put(x + hx, fy, b'W');
        }
    }
    let hd = shift(
        &union(&ell(CX, 29.5, 19.5, 14.5), &ell(CX, 33.5, 21.0, 9.5)),
        hx,
        0,
    );
    let tone = lit(CX - 6.0 + hx as f64, 24.0, 24.0, 19.0, b"lLOOD", 0.0, 3.0);
    let head_col = |x: i32, y: i32| -> u8 {
        let dx = (x as f64 + 0.5 - CX - hx as f64).abs();
        if y as f64 + 0.5 > 35.5 - 6.0 * pw(dx / 21.0, 1.3) && !(dx < 3.0 && y < 35) {
            return dith(x, y, (y - 33) as f64 / 14.0, b"WWC");
        }
        tone(x, y)
    };
    paint(&mut g, &hd, &head_col, Some(b'K'), None);
    for sgn in [-1, 1] {
        let mut pts: Vec<Pt> = vec![
            (17.0, 33.0),
            (12.5, 37.5),
            (16.5, 38.0),
            (14.0, 41.5),
            (20.0, 41.0),
            (19.5, 35.0),
        ];
        if sgn > 0 {
            pts = pts.iter().map(|&(x, y)| (2.0 * CX - x, y)).collect();
        }
        let c = shift(&poly(&pts), hx, 0);
        let col = |x: i32, y: i32| dith(x, y, 0.25, b"WC");
        paint(&mut g, &c, &col, Some(b'K'), Some(b"."));
        paint(&mut g, &c, &col, None, None);
    }
    for (x, y, ch) in [
        (34, 15, b'L'),
        (35, 14, b'K'),
        (34, 14, b'K'),
        (33, 15, b'K'),
        (36, 15, b'L'),
        (37, 14, b'K'),
        (38, 15, b'K'),
    ] {
        g.put(x + hx, y, ch);
    }
    stampd(&mut g, 34 + hx, 18, &[".MM.", ".XX.", "XxXJ", ".JJ."]);
    for (x, y) in [(25, 23), (26, 23), (25, 22)] {
        g.put(x + hx, y, b'm');
        g.put(mx(x) + hx, y, b'm');
    }
    head_ears(&mut g, look);
    g
}

fn head_ears(g: &mut Grid, look: &Look) {
    if look.ears == "snowy" {
        for y in 0..10 {
            for x in 0..W {
                if g.at(x, y) == b't' {
                    g.set(x, y, if (x + y) % 3 != 0 { b'W' } else { b'C' });
                }
            }
        }
    }
}

// ---- face ------------------------------------------------------------------------------------------
pub const EYE_L: (i32, i32) = (21, 27);
const EYE_OPEN: [&str; 9] = [
    "...BBBB..",
    "B.BBBBBB.",
    ".BBqqqqBB",
    "BBqHHqqqB",
    ".BqHHBBqB",
    ".BeeBBBeB",
    ".BeeeeHeB",
    "..BeeeeB.",
    "...BBBB..",
];

fn eye_cells(kind: &str) -> OrdMap<u8> {
    let mut out: OrdMap<u8> = OrdMap::new();
    let grid = |out: &mut OrdMap<u8>, rows: &[&str]| {
        for (r, row) in rows.iter().enumerate() {
            for (c, ch) in row.bytes().enumerate() {
                if ch != b'.' {
                    out.insert((c as i32, r as i32), ch);
                }
            }
        }
    };
    match kind {
        "open" | "sad" => {
            grid(&mut out, &EYE_OPEN);
            if kind == "sad" {
                out.insert((6, 6), b'H');
                out.insert((3, 6), b'H');
            }
        }
        "focus" => {
            grid(
                &mut out,
                &[
                    "",
                    "",
                    "B.BBBBBB.",
                    ".BBBBBBBB",
                    "BBqqBBqqB",
                    ".BeeBBBeB",
                    ".BeeeeHeB",
                    "..BeeeeB.",
                    "...BBBB..",
                ],
            );
            out.insert((3, 4), b'H');
        }
        "angry" => {
            grid(
                &mut out,
                &[
                    "",
                    "",
                    "..BBBB...",
                    ".BBqqqBB.",
                    "BBqHHqqB.",
                    ".BqHBBqB.",
                    ".BeeBBeB.",
                    "..BeeeB..",
                    "...BBB...",
                ],
            );
            for v in out.vals.iter_mut() {
                if *v == b'e' {
                    *v = b'V';
                }
            }
        }
        "blink" | "sleep" => {
            grid(
                &mut out,
                &[
                    "",
                    "",
                    "",
                    "",
                    "B......B.",
                    ".BB...BB.",
                    "...BBB...",
                    "",
                    "",
                ],
            );
            out.insert((0, 3), b'B');
            if kind == "sleep" {
                out.insert((1, 3), b'B');
            }
        }
        "happy" => {
            grid(
                &mut out,
                &[
                    "",
                    "",
                    "",
                    "...BBB...",
                    "..BB.BB..",
                    ".BB...BB.",
                    "B.......B",
                    "",
                    "",
                ],
            );
            out.insert((0, 4), b'B');
        }
        _ => {}
    }
    out
}

pub fn eyes(g: &mut Grid, kind: &str, lx: i32, ly: i32, hx: i32) {
    let (ex, ey) = EYE_L;
    let cells = eye_cells(kind);
    for (i, &(c, r)) in cells.keys.iter().enumerate() {
        let ch = cells.vals[i];
        g.put(ex + c + lx + hx, ey + r + ly, ch);
        if ch == b'H' {
            continue;
        }
        g.put(ex + 21 + (8 - c) + lx + hx, ey + r + ly, ch);
    }
    for (i, &(c, r)) in cells.keys.iter().enumerate() {
        if cells.vals[i] == b'H' {
            g.put(ex + 21 + c + lx + hx, ey + r + ly, b'H');
        }
    }
    if kind == "angry" {
        for (c, r) in [(2, 0), (3, 0), (4, 1), (5, 1)] {
            g.put(ex + c + hx, ey + r, b'm');
            g.put(ex + 21 + 8 - c + hx, ey + r, b'm');
        }
    }
    if kind == "sad" {
        for (c, r) in [(3, 0), (4, 0), (5, -1)] {
            g.put(ex + c + hx, ey + r - 1, b'm');
            g.put(ex + 21 + 8 - c + hx, ey + r - 1, b'm');
        }
        for (c, r) in [(1, 9), (1, 10), (0, 11), (1, 11), (2, 11), (1, 12)] {
            g.put(ex + c + lx + hx, ey + r + ly, b'T');
            g.put(ex + 21 + 8 - c + lx + hx, ey + r + ly, b'T');
        }
    }
}

pub fn blush(g: &mut Grid, hx: i32, strong: bool, look: &Look) {
    let st = look.blush.as_str();
    if st == "none" {
        return;
    }
    let strong = strong || st == "rosy";
    let mut pts = vec![(20, 37), (22, 37), (24, 37), (21, 38), (23, 38)];
    if strong {
        pts.extend([(19, 38), (25, 38)]);
    }
    for (x, y) in pts {
        g.put(x + hx, y, b'P');
        g.put(mx(x) + hx, y, b'P');
    }
}

pub fn mouth(g: &mut Grid, kind: &str, sx: i32, sy: i32, hx: i32) {
    let p2 = |g: &mut Grid, pts: &[(i32, i32)], ch: u8| {
        for &(x, y) in pts {
            g.put(x + sx + hx, y + sy, ch);
            g.put(mx(x) + sx + hx, y + sy, ch);
        }
    };
    p2(g, &[(34, 35), (35, 35), (35, 36)], b'B');
    g.put(34 + sx + hx, 35 + sy, b'H');
    let smile = [(35, 37), (34, 38), (33, 38), (32, 37)];
    match kind {
        "flat" | "lick" => p2(g, &smile, b'K'),
        "open" => {
            p2(g, &smile, b'K');
            p2(g, &[(34, 39), (35, 39)], b'R');
            p2(g, &[(33, 39), (34, 40), (35, 40)], b'K');
        }
        "grin" => {
            p2(g, &smile, b'K');
            p2(g, &[(34, 39), (35, 39), (33, 39)], b'R');
            p2(g, &[(32, 39), (33, 40), (34, 40), (35, 40)], b'K');
            p2(g, &[(35, 39)], b'P');
        }
        "frown" => p2(g, &[(35, 38), (34, 38), (33, 39)], b'K'),
        "pout" => {
            p2(g, &[(35, 38), (34, 38)], b'K');
            g.put(36 + sx + hx, 39 + sy, b'K');
            g.put(35 + sx + hx, 39 + sy, b'K');
        }
        _ => {}
    }
}

pub fn glyph(g: &mut Grid, x0: i32, y0: i32, rows: &[&str], ch: u8) {
    for (dy, r) in rows.iter().enumerate() {
        for (dx, c) in r.bytes().enumerate() {
            if c == b'#' {
                g.put(x0 + dx as i32, y0 + dy as i32, ch);
            } else if c == b'o' {
                g.put(x0 + dx as i32, y0 + dy as i32, b'K');
            }
        }
    }
}

const STAR: [&str; 7] = [
    "...#...", "...#...", "..###..", "#######", "..###..", "...#...", "...#...",
];
const STAR_S: [&str; 3] = [".#.", "###", ".#."];
const HEART: [&str; 6] = [
    ".##.##.", "#######", "#######", ".#####.", "..###..", "...#...",
];

pub fn effects(g: &mut Grid, kind: &str, frame: i32) {
    match kind {
        "angry" => {
            glyph(
                g,
                60,
                9 + frame,
                &["..o..", ".o#o.", "o###o", ".o#o.", "..o.."],
                b'V',
            );
            glyph(g, 66, 5 - frame, &[".o.", "o#o", ".o."], b'V');
        }
        "sleep" => {
            glyph(
                g,
                60,
                22 - 2 * frame,
                &["######", "....#.", "...#..", "..#...", ".#....", "######"],
                b'Z',
            );
            glyph(
                g,
                70,
                12 - 2 * frame,
                &["####", "..#.", ".#..", "####"],
                b'Z',
            );
            glyph(g, 77, 5 - frame, &["###", ".#.", "###"], b'Z');
        }
        "dustL" | "dustR" => {
            let sgn = if kind == "dustL" { -1 } else { 1 };
            let x0 = pint(CX + (sgn * 23) as f64) - if sgn < 0 { 3 } else { 0 };
            for (dx, dy, ch) in [
                (0, 0, b'C'),
                (1, 0, b'W'),
                (2, 0, b'C'),
                (1, -1, b'C'),
                (3, 1, b'C'),
                (-1, 1, b'C'),
                (2, -2, b'W'),
            ] {
                let xx = x0 + if sgn > 0 { dx } else { -dx };
                if 0 <= xx && xx < W && g.at(xx, 75 + dy) == b'.' {
                    g.put(xx, 75 + dy, ch);
                }
            }
        }
        "happy" => {
            glyph(g, 3, if frame != 0 { 9 } else { 11 }, &STAR, b'Y');
            glyph(g, 61, if frame != 0 { 2 } else { 4 }, &HEART, b'V');
            glyph(g, if frame != 0 { 8 } else { 10 }, 22, &STAR_S, b'Y');
            glyph(g, 68, if frame != 0 { 14 } else { 12 }, &STAR_S, b'Y');
        }
        _ => effects2(g, kind, frame),
    }
}

fn effects2(g: &mut Grid, kind: &str, frame: i32) {
    let small_heart = [".#.#.", "#####", ".###.", "..#.."];
    match kind {
        "hearts" => {
            glyph(g, 60, 4 - frame * 2, &HEART, b'V');
            glyph(g, 2, 12 + frame, &small_heart, b'P');
            glyph(g, 70, 18 - frame, &small_heart, b'P');
        }
        "notes" => {
            let note = ["..##", "..#.", "..#.", "###.", "##.."];
            glyph(g, 62, 6 + frame * 2, &note, b'Y');
            glyph(g, 72, 14 - frame * 2, &note, b'I');
            glyph(g, 3, 10 + frame, &note, b'Y');
        }
        "boop" => {
            glyph(g, 62, 6, &["#", "#", "#", ".", "#"], b'V');
            glyph(g, 66, 3 + frame, &STAR_S, b'Y');
        }
        "sulk" => {
            glyph(
                g,
                58,
                4 + frame,
                &["..ooo...", ".o###oo.", "o######o", ".oo##oo.", "...oo..."],
                b'Z',
            );
            glyph(g, 60, 26, &[".o.", "o#o", "o#o", ".o."], b'T');
        }
        "crumbs" => {
            for (x, y) in [
                (44, 44 + frame),
                (47, 47 - frame),
                (41, 48),
                (49, 43 + frame * 2),
            ] {
                g.put(x, y, b'o');
            }
        }
        "spark" => {
            glyph(g, 2, 8 + frame * 2, &STAR, b'Y');
            glyph(g, 64, 4 + (1 - frame) * 2, &STAR_S, b'Y');
            glyph(g, 70, 20, &STAR_S, b'x');
        }
        _ => {}
    }
}

/// name -> (eyes, mouth, effect)
pub fn state(name: &str) -> Option<(&'static str, &'static str, Option<&'static str>)> {
    Some(match name {
        "idle" => ("open", "flat", None),
        "focus" => ("focus", "flat", None),
        "blink" => ("blink", "flat", None),
        "alert" => ("angry", "pout", Some("angry")),
        "alert2" => ("angry", "open", Some("angry")),
        "happy" => ("happy", "open", Some("happy")),
        "sleep" => ("sleep", "flat", Some("sleep")),
        "sad" => ("sad", "frown", None),
        "lick" => ("happy", "lick", None),
        "lick2" => ("happy", "flat", None),
        "munch" => ("happy", "open", Some("crumbs")),
        "munch2" => ("happy", "flat", Some("crumbs")),
        "yawn" => ("blink", "open", None),
        "sulk" => ("focus", "pout", Some("sulk")),
        "boop" => ("blink", "open", Some("boop")),
        "boop2" => ("open", "grin", None),
        "hearts" => ("happy", "grin", Some("hearts")),
        "giggle" => ("happy", "grin", Some("spark")),
        "sing" => ("happy", "open", Some("notes")),
        "sing2" => ("blink", "flat", Some("notes")),
        "highfive" => ("happy", "grin", Some("spark")),
        "chomp" => ("angry", "open", None),
        "walk0" => ("open", "grin", Some("dustL")),
        "walk1" => ("open", "grin", None),
        "walk2" => ("open", "grin", Some("dustR")),
        "walk3" => ("open", "grin", None),
        _ => return None,
    })
}
pub const STATE_NAMES: [&str; 26] = [
    "idle", "focus", "blink", "alert", "alert2", "happy", "sleep", "sad", "lick", "lick2", "munch",
    "munch2", "yawn", "sulk", "boop", "boop2", "hearts", "giggle", "sing", "sing2", "highfive",
    "chomp", "walk0", "walk1", "walk2", "walk3",
];

pub fn glasses(g: &mut Grid, lx: i32, ly: i32, hx: i32) {
    for cx in [25.5f64, 46.5] {
        for a in (0..360).step_by(5) {
            let x = pint(cx + 5.6 * radians(a as f64).cos());
            let y = pint(31.5 + 5.3 * radians(a as f64).sin());
            g.put(x + lx + hx, y + ly, b'F');
        }
        g.put(pint(cx) - 3 + lx + hx, 28 + ly, b'H');
    }
    for x in 31..41 {
        g.put(x + lx + hx, 30 + ly, b'F');
    }
}

pub fn headphones(g: &mut Grid, hx: i32) {
    for a in (196..345).step_by(2) {
        for (k, ch) in [(0, b'K'), (1, b'E'), (2, b'E'), (3, b'K')] {
            let xx = pint(CX + (19.5 - k as f64) * radians(a as f64).cos()) + hx;
            let yy = pint(31.0 + (18.0 - k as f64) * radians(a as f64).sin());
            g.put(xx, yy, ch);
        }
    }
    for sgn in [-1, 1] {
        let x0 = (if sgn < 0 { 12 } else { 54 }) + hx;
        let mut cup = Cells::new();
        for x in x0..x0 + 6 {
            for y in 26..39 {
                cup.insert((x, y));
            }
        }
        for c in [(x0, 26), (x0 + 5, 26), (x0, 38), (x0 + 5, 38)] {
            cup.remove(&c);
        }
        paint(
            g,
            &cup,
            &|x, y| {
                if (x == x0 + 1 && (28..=31).contains(&y) && sgn < 0)
                    || (x == x0 + 4 && (28..=31).contains(&y) && sgn > 0)
                {
                    b'Q'
                } else if y >= 35 {
                    b'G'
                } else {
                    b'E'
                }
            },
            Some(b'K'),
            None,
        );
    }
}

fn lick_paw(g: &mut Grid, frame: i32, hx: i32) {
    let up = if frame != 0 { 1 } else { 0 };
    let leg = poly(&[(23.5, 55.0), (31.0, 55.5), (34.0, 45.0), (26.5, 44.0)]);
    paint(
        g,
        &leg,
        &|x, y| {
            if y < 49 {
                b'n'
            } else {
                dith(x, y, (x - 23) as f64 / 9.0, b"LOD")
            }
        },
        Some(b'k'),
        Some(b"WCLODdlMmwojKSXJxB"),
    );
    let p = ell(31.0 + hx as f64, 42.5 - up as f64, 4.8, 3.6);
    paint(
        g,
        &p,
        &|x, y| {
            if y >= 42 - up || (x + y) % 2 != 0 {
                b'n'
            } else {
                b'N'
            }
        },
        Some(b'K'),
        None,
    );
    for (x, y) in [(28, 44), (30, 44), (32, 44)] {
        g.put(x + hx, y - up, b'K');
    }
    if frame != 0 {
        for (x, y) in [(34, 39), (35, 39), (34, 40), (35, 40), (33, 40)] {
            g.put(x + hx, y, if y == 39 { b'R' } else { b'P' });
        }
    }
}

fn highfive_paw(g: &mut Grid, _hx: i32) {
    let leg = poly(&[(23.5, 55.0), (30.5, 55.5), (17.5, 38.0), (11.5, 40.0)]);
    paint(
        g,
        &leg,
        &|x, y| {
            if y < 42 {
                b'n'
            } else {
                dith(x, y, (x - 12) as f64 / 14.0, b"LOD")
            }
        },
        Some(b'k'),
        Some(b"WCLODdlMmwojKSXJxB"),
    );
    let p = ell(13.0, 35.0, 4.8, 4.0);
    paint(
        g,
        &p,
        &|x, y| {
            if y >= 35 || (x + y) % 2 != 0 {
                b'n'
            } else {
                b'N'
            }
        },
        Some(b'K'),
        None,
    );
    for (x, y) in [(11, 33), (13, 32), (15, 33)] {
        g.put(x, y, b'P');
    }
    g.put(13, 35, b'P');
    g.put(12, 35, b'P');
}

const FLOWER: [&str; 7] = [
    "..2.2..", ".21212.", "2112112", ".12X21.", "2112112", ".21.12.", "..2.2..",
];
const BOW: [&str; 5] = [".KK.KK.", "KMMKMMK", "KMmXmMK", "KMMKMMK", ".KK.KK."];
const CROWN: [&str; 5] = ["x..xx..x", "X..XX..X", "XXXVVXXX", "XXXXXXXX", "JJJJJJJJ"];
const MASK: [&str; 8] = [
    "K.....K", "KK...KK", "K5KKK5K", "K55555K", "K66566K", "K55555K", ".K565K.", "..KKK..",
];

pub fn draw_outfit(g: &mut Grid, acc: &[&str], hx: i32, frame: i32) {
    if has(acc, "flower") {
        stampd(g, 11 + hx, 11, &FLOWER);
    }
    if has(acc, "bow") {
        stampd(g, 51 + hx, 12, &BOW);
    }
    if has(acc, "crown") {
        stampd(g, 32 + hx, 10, &CROWN);
    }
    if has(acc, "headband") {
        let fur = b"lLOODdWC";
        for x in 14..58 {
            if 33 + hx <= x && x <= 38 + hx {
                continue;
            }
            for (y, ch) in [(20, b'M'), (21, b'm')] {
                if 0 <= x && x < W && in_set(fur, g.at(x, y)) {
                    g.set(x, y, ch);
                }
            }
        }
        let fl = imod(frame, 2);
        for (x, y) in [
            (56, 20),
            (57, 20),
            (58, 21 + fl),
            (59, 22 + fl),
            (57, 22),
            (58, 23),
            (59, 24 - fl),
        ] {
            g.put(x + hx, y, b'M');
        }
    }
    if has(acc, "mask") {
        stampd(g, 8 + hx, 21, &MASK);
    }
    if has(acc, "towel") {
        for x in 22..51 {
            if (32..=39).contains(&x) {
                continue;
            }
            let arc = pyround((x - 36i32).abs() as f64 * 0.15);
            g.put(x, 40 + arc, b'K');
            for (k, y) in [41 + arc, 42 + arc, 43 + arc].into_iter().enumerate() {
                let mut ch = if k == 0 { b'8' } else { b'7' };
                if k == 1 && imod(x + arc, 4) == 0 {
                    ch = b'9';
                }
                g.put(x, y, ch);
            }
            g.put(x, 44 + arc, b'K');
        }
        for y in 44..52 {
            let sl = ifloordiv(y - 44, 3);
            for x in (20 - sl)..(26 - sl) {
                let edge = x == 20 - sl || x == 25 - sl;
                g.put(
                    x,
                    y,
                    if edge {
                        b'K'
                    } else if (y == 48 || y == 49) && x % 2 == 0 {
                        b'9'
                    } else if x == 21 - sl {
                        b'8'
                    } else {
                        b'7'
                    },
                );
            }
        }
        for x in 18..23 {
            g.put(x, 52, b'K');
            g.put(x, 53, if x % 2 != 0 { b'9' } else { b'K' });
        }
    }
    if has(acc, "sweat") {
        for (x, y, ch) in [
            (58, 26, b'I'),
            (58, 27, b'I'),
            (57, 28, b'I'),
            (58, 28, b'A'),
            (59, 28, b'I'),
            (58, 29, b'I'),
            (57, 27, b'K'),
            (59, 27, b'K'),
            (56, 28, b'K'),
            (60, 28, b'K'),
            (57, 29, b'K'),
            (59, 29, b'K'),
            (58, 30, b'K'),
            (58, 25, b'K'),
        ] {
            g.put(x + hx, y, ch);
        }
    }
}

// ---- zenko19 wardrobe --------------------------------------------------------------------------------
const ROBE_FUR: &[u8] = b"lLOODdWCSsk";

/// zenko22 `_motif`: extra cloth patterns for the travel outfits, falling back to the zenko19 ones.
pub fn motif(x: i32, y: i32, motif: &str) -> Option<u8> {
    let a = imod(x * 7 + y * 13, 23);
    match motif {
        "bingata" => {
            return if a == 0 || a == 1 {
                Some(b'f')
            } else if a == 11 || a == 12 {
                Some(b'u')
            } else {
                None
            };
        }
        "fan" => {
            let k = (imod(x, 8), imod(y, 7));
            return if [(3, 2), (2, 3), (3, 3), (4, 3)].contains(&k) {
                Some(b'f')
            } else if a == 5 {
                Some(b'u')
            } else {
                None
            };
        }
        "hanbok" => {
            if y < 58 {
                return [Some(b'f'), Some(b'u'), None, Some(b'h'), None]
                    [imod(ifloordiv(y, 2), 5) as usize];
            }
            return None;
        }
        "cloud" => {
            return if imod(x + pint(3.0 * (y as f64 * 0.5).sin()), 9) == 0 && imod(y, 3) != 0 {
                Some(b'f')
            } else {
                None
            };
        }
        "ikat" => {
            let v = imod(x + (imod(y, 8) - 4).abs(), 8);
            return if v == 0 {
                Some(b'f')
            } else if v == 4 {
                Some(b'u')
            } else {
                None
            };
        }
        _ => {}
    }
    motif_z21(x, y, motif)
}

fn motif_z21(x: i32, y: i32, motif: &str) -> Option<u8> {
    if motif == "sailor" {
        return None;
    }
    let (a, b) = (imod(x * 7 + y * 13, 23), imod(x + 2 * y, 9));
    match motif {
        "sakura" => {
            if a == 0 {
                Some(b'f')
            } else if a == 1 || a == 22 {
                Some(b'u')
            } else {
                None
            }
        }
        "maple" => {
            if a == 0 {
                Some(b'f')
            } else if (a == 5 || a == 11) && b < 3 {
                Some(b'u')
            } else {
                None
            }
        }
        "snow" => {
            if (a == 0 || a == 12) && b % 2 == 0 {
                Some(b'f')
            } else {
                None
            }
        }
        "water" => {
            if imod(x + pint(2.5 * (y as f64 * 0.7).sin()), 7) == 0 {
                Some(b'f')
            } else {
                None
            }
        }
        "asagao" => {
            if a == 0 {
                Some(b'f')
            } else if a == 11 {
                Some(b'u')
            } else {
                None
            }
        }
        _ => None,
    }
}

pub fn robe_sit(g: &mut Grid, acc: &[&str]) {
    let Some(rb) = robe_of(acc) else { return };
    let mo = robe_motif(rb);
    for y in 40..76 {
        for x in 8..64 {
            let ch = g.at(x, y);
            if !in_set(ROBE_FUR, ch) || ch == b'k' {
                continue;
            }
            if mo == "sailor" {
                g.set(
                    x,
                    y,
                    if y < 56 {
                        b'h'
                    } else if (y + x) % 9 != 0 {
                        b'a'
                    } else {
                        b'c'
                    },
                );
                continue;
            }
            let side = (x as f64 + 0.5 - CX).abs() / 16.0;
            g.set(
                x,
                y,
                motif(x, y, mo).unwrap_or(if side > 0.78 || y > 72 { b'c' } else { b'a' }),
            );
        }
    }
    let long_ = mo != "sailor";
    let pts: Vec<Pt> = vec![
        (23.0, 45.0),
        (16.5, 47.0),
        (12.5, 53.0),
        (12.0, if long_ { 66.5 } else { 57.5 }),
        (14.5, if long_ { 69.0 } else { 60.0 }),
        (19.5, if long_ { 69.0 } else { 60.0 }),
        (21.0, 52.0),
    ];
    for sgn in [-1, 1] {
        let p: Vec<Pt> = if sgn < 0 {
            pts.clone()
        } else {
            pts.iter().map(|&(x, y)| (2.0 * CX - x, y)).collect()
        };
        let cells: Cells = poly(&p)
            .into_iter()
            .filter(|c| !in_set(b"wojnN", g.at(c.0, c.1)))
            .collect();
        if long_ {
            paint(
                g,
                &cells,
                &|x, y| {
                    motif(x, y, mo).unwrap_or(if y >= 67 || (x as f64 + 0.5 - CX).abs() > 21.5 {
                        b'c'
                    } else {
                        b'a'
                    })
                },
                Some(b'K'),
                Some(b".gLODdlWCk"),
            );
        } else {
            paint(
                g,
                &cells,
                &|_x, y| {
                    if y >= 58 {
                        b'a'
                    } else if y == 57 {
                        b'f'
                    } else {
                        b'h'
                    }
                },
                Some(b'K'),
                Some(b".gLODdlWCk"),
            );
        }
    }
    if mo == "sailor" {
        for x in 22..51 {
            for y in 44..50 {
                let d = (x as f64 + 0.5 - CX).abs();
                let cur = g.at(x, y);
                if (y - 44) as f64 > (d - 8.0) * 0.9 && in_set(b"hK", cur) {
                    let edge = y == 49 || d > 13.2;
                    let nv = if (y == 46 && d > 3.0) || ((d - 12.0).abs() < 0.6) {
                        b'f'
                    } else if edge && cur == b'K' {
                        b'K'
                    } else {
                        b'a'
                    };
                    g.set(x, y, nv);
                }
            }
        }
        stampd(
            g,
            32,
            50,
            &[
                "KiiiiiiK", ".KiriiK.", "..KiiK..", ".KiK.iK.", "Kii..iiK", "KK....KK",
            ],
        );
    } else {
        for t in 0..12 {
            for w in 0..3 {
                g.put(
                    pint(27.0 + t as f64 * 0.75) + w,
                    44 + t,
                    if w < 2 { b'h' } else { b'K' },
                );
                g.put(
                    pint(45.0 - t as f64 * 0.75) - w,
                    44 + t,
                    if w < 2 { b'h' } else { b'c' },
                );
            }
        }
        stampd(g, 33, 51, &["KiiiiK", "KirriK"]);
    }
}

/// The sitting fox's body layer (tails are a separate layer, see build_tails).
pub fn build_frame(
    st: &str,
    frame: i32,
    lookv: Option<(i32, i32, i32, i32)>,
    acc: &[&str],
    hx: i32,
    earf: i32,
    look: &Look,
) -> Grid {
    let (lx, ly, sx, sy) = lookv.unwrap_or((0, 0, 0, 0));
    let (eyes_k, mouth_k, fx) = state(st).expect("unknown state");
    let lick = st == "lick" || st == "lick2";
    let five = st == "highfive";
    let walk = walk_feet(st).map(|_| st);
    let mut g = body(
        if lick || five {
            "lick"
        } else {
            walk.unwrap_or("sit")
        },
        has(acc, "scarf"),
    );
    robe_sit(&mut g, acc);
    over(&mut g, &head(hx, earf, look));
    eyes(
        &mut g,
        eyes_k,
        if lick { 0 } else { lx },
        if lick { 0 } else { ly },
        hx,
    );
    blush(&mut g, hx, eyes_k == "happy", look);
    mouth(
        &mut g,
        mouth_k,
        if lick { 0 } else { sx },
        if lick { 0 } else { sy },
        hx,
    );
    if has(acc, "glasses") {
        glasses(&mut g, lx, ly, hx);
    }
    draw_outfit(&mut g, acc, hx, frame);
    if has(acc, "headphones") {
        headphones(&mut g, hx);
    }
    if lick {
        lick_paw(&mut g, if st == "lick" { 1 } else { 0 }, hx);
    }
    if five {
        highfive_paw(&mut g, hx);
    }
    if let Some(fx) = fx {
        effects(&mut g, fx, frame);
    }
    seal(&mut g);
    g
}

const Z19_BOOK: [&str; 10] = [
    "..KKKKKKKKK..KKKKKKKKK..",
    ".KwwwwwwwwwKKwwwwwwwwwK.",
    "KwwbbbbbwwwKwwbbbbbwwwwK",
    "KwwwwwwwwwwKwwwwwwwwwwwK",
    "KwwbbbbwwwwKwwbbbbbbwwwK",
    "KwwwwwwwwwwKwwwwwwwwwwwK",
    "KwwbbbbbbwwKwwbbbwwwwwwK",
    "KwwwwwwwwwwKwwwwwwwwwwwK",
    "KoooooooooooKoooooooooooK",
    ".KKKKKKKKKKK.KKKKKKKKKKK.",
];
const Z19_BOOK2: [&str; 10] = [
    "..KKKKKKKKK..KKKKKK.....",
    ".KwwwwwwwwwKKwwwwwwKK...",
    "KwwbbbbbwwwKwwbbbwwwwK..",
    "KwwwwwwwwwwKwwwwwwwwwwK.",
    "KwwbbbbwwwwKwwbbbbbwwwwK",
    "KwwwwwwwwwwKwwwwwwwwwwwK",
    "KwwbbbbbbwwKwwbbbwwwwwwK",
    "KwwwwwwwwwwKwwwwwwwwwwwK",
    "KoooooooooooKoooooooooooK",
    ".KKKKKKKKKKK.KKKKKKKKKKK.",
];
const Z19_BELL: [&str; 6] = [
    "KKK.....KKK",
    "KGK.....KGK",
    "KEKKKKKKKEK",
    "KEGGGGGGGEK",
    "KGK.....KGK",
    "KKK.....KKK",
];

/// zenko19 sidebar extras drawn over a sitting frame: reading a book, lifting dumbbells, zen sparkles.
pub fn sit_extra(g: &Grid, kind: &str, frame: i32) -> Grid {
    let mut g = g.clone();
    match kind {
        "read" => {
            let y = 50 + if imod(frame, 2) == 0 { 1 } else { 0 };
            stampd(
                &mut g,
                24,
                y,
                if imod(frame, 4) != 0 {
                    &Z19_BOOK
                } else {
                    &Z19_BOOK2
                },
            );
        }
        "lift" => {
            let up = imod(frame, 2);
            stampd(&mut g, 25, 49 - 5 * up, &Z19_BELL);
            stampd(&mut g, 36, 49 - 5 * (1 - up), &Z19_BELL);
        }
        "zen" => {
            for (i, (x, y)) in [(10, 30), (62, 26), (66, 44), (6, 46)]
                .into_iter()
                .enumerate()
            {
                let i = i as i32;
                let yy = y - imod(frame + i * 3, 8);
                if imod(frame + i, 3) != 0 {
                    g.put(x, yy, b'A');
                    g.put(x + 1, yy, b'I');
                }
            }
        }
        _ => {}
    }
    g
}

#[allow(dead_code)]
fn _unused() -> f64 {
    PI
}
