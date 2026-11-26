//! Curled-up sleeping Zenko and the foxfire flame, from zenko.py build_curl / foxfire_rows.

use crate::core::*;
use crate::sit::{effects, has, paw};
use std::f64::consts::PI;

pub fn build_curl(frame: i32, acc: &[&str], n: i32) -> Grid {
    let mut g = Grid::blank();
    for (x, y) in ell(46.0, 76.5, 36.0, 1.8) {
        if (x + y) % 2 == 0 {
            g.set(x, y, b'g');
        }
    }
    let b = if frame != 0 { 0.5 } else { 0.0 };
    let body = ell(52.0, 63.5 - b / 2.0, 25.0, 12.0 + b);
    paint(
        &mut g,
        &body,
        &lit(46.0, 56.0, 30.0, 16.0, b"lLODd", 0.05, 3.0),
        Some(b'K'),
        None,
    );
    for pts in [
        [(13.0, 57.0), (24.0, 53.0), (9.0, 43.5)],
        [(25.0, 52.5), (35.0, 56.0), (33.5, 42.0)],
    ] {
        paint(
            &mut g,
            &poly(&pts),
            &|_x, y| if y <= 47 { b't' } else { b'O' },
            Some(b'K'),
            None,
        );
    }
    for pts in [
        [(14.5, 55.0), (21.5, 53.0), (11.5, 47.5)],
        [(27.0, 53.0), (32.5, 54.5), (31.5, 47.0)],
    ] {
        paint_ch(&mut g, &poly(&pts), b'P', None, None);
    }
    let hd = ell(23.0, 63.0, 14.0, 11.0);
    let tone = lit(19.0, 58.0, 16.0, 13.0, b"lLOOD", 0.0, 3.0);
    paint(
        &mut g,
        &hd,
        &|x, y| {
            if y as f64 >= 64.0 + (x as f64 + 0.5 - 23.0).abs() * 0.2 {
                dith(x, y, 0.2, b"WC")
            } else {
                tone(x, y)
            }
        },
        Some(b'K'),
        None,
    );
    stampd(&mut g, 22, 53, &[".MM.", ".XX.", "XxXJ", ".JJ."]);
    for ex0 in [13, 26] {
        for (c, r) in [(0, 0), (1, 1), (2, 1), (3, 1), (4, 1), (5, 0)] {
            g.put(ex0 + c, 62 + r, b'B');
        }
    }
    g.put(12, 61, b'B');
    g.put(32, 61, b'B');
    for (x, y) in [(22, 66), (23, 66), (24, 66), (23, 67)] {
        g.put(x, y, b'B');
    }
    g.put(22, 66, b'H');
    for (x, y) in [(13, 65), (15, 65), (14, 66), (31, 65), (33, 65), (32, 66)] {
        g.put(x, y, b'P');
    }
    for (x, y) in [(22, 69), (23, 70), (24, 69)] {
        g.put(x, y, b'K');
    }
    paw(&mut g, 31.0, 72.0, 5.0, 2.6, true);
    if has(acc, "headphones") {
        let mut cup = Cells::new();
        for x in 32..38 {
            for y in 57..67 {
                cup.insert((x, y));
            }
        }
        paint(
            &mut g,
            &cup,
            &|x, y| if x == 33 && y < 61 { b'Q' } else { b'E' },
            Some(b'K'),
            None,
        );
        for x in 12..33 {
            let y = 50 - pint(3.0 * ((x - 12) as f64 / 21.0 * PI).sin());
            g.put(x, y, b'E');
            g.put(x, y - 1, b'K');
        }
    }
    let nn = 1.max(9.min(n));
    let show = nn.min(4);
    for i in 0..show {
        let k = show - 1 - i;
        let o = k as f64 * 2.4;
        let pts = [
            (74.0, 60.0 - o),
            (101.0, 70.0 - o * 0.5),
            (64.0, 81.0 - o * 0.3),
            (34.0 + (k * 3) as f64, 74.5 - o),
        ];
        tail(
            &mut g,
            &pts,
            7.4 - 0.4 * k as f64,
            k * 5 + 2,
            frame,
            4.0,
            0.72,
            b'k',
        );
    }
    effects(&mut g, "sleep", frame);
    seal(&mut g);
    g
}

/// A 9x13 flame; the frame (0..3) makes it flicker.
pub fn foxfire_rows(frame: i32) -> Grid {
    let (w, h) = (9, 13);
    let mut g = Grid::new(w, h);
    let sway = [0, 1, 0, -1][imod(frame, 4) as usize];
    for y in 0..h {
        let t = y as f64 / (h as f64 - 1.0);
        let hw = if t < 0.55 {
            4.2 * pw(t / 0.55, 0.8)
        } else {
            4.2 * 0.0f64.max(1.0 - pw((t - 0.55) / 0.45, 2.0)).sqrt() + 0.3
        };
        let cxf = 4.5 + sway as f64 * (1.0 - t) * 1.6;
        for x in 0..w {
            let d = (x as f64 + 0.5 - cxf).abs();
            if d <= hw {
                let core = d / hw.max(0.5);
                let ch = if t > 0.45 && core < 0.45 {
                    b'A'
                } else if core < 0.75 && t > 0.25 {
                    b'I'
                } else {
                    b'U'
                };
                g.set(x, y, ch);
            }
        }
    }
    if frame % 2 != 0 {
        g.set(pint(4.0 + sway as f64), 0, b'U');
    }
    g
}
