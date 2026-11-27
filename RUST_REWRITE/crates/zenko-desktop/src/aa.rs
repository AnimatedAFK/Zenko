//! zenko.py aa_pixels / _shape_d / _poly_d / _bbox / _cov: simple shapes drawn with smooth (anti-aliased) edges into
//! an RGBA picture (the moon, the sun, glows). Python's float maths step by step, so every byte matches.

use crate::tkcanvas::Photo;
use zenko_art::palette::hex_rgb;

#[derive(Clone, Debug)]
pub enum Shape {
    /// ("circle", cx, cy, r, colour)
    Circle(f64, f64, f64),
    /// ("ring", cx, cy, r, width, colour)
    Ring(f64, f64, f64, f64),
    /// ("poly", [(x, y)...], colour)
    Poly(Vec<(f64, f64)>),
    /// ("rrect", x1, y1, x2, y2, r, colour)
    RRect(f64, f64, f64, f64, f64),
}

/// a shape with its colour: Some((hex, alpha)) or None (erase)
pub type Sh = (Shape, Option<(String, f64)>);

pub fn cov(d: f64) -> f64 {
    if d <= -0.5 {
        0.0
    } else if d >= 0.5 {
        1.0
    } else {
        d + 0.5
    }
}

fn hypot(x: f64, y: f64) -> f64 {
    x.hypot(y)
}

pub fn poly_d(pts: &[(f64, f64)], x: f64, y: f64) -> f64 {
    let mut inside = false;
    let mut best = 1e9f64;
    let n = pts.len();
    let mut j = n - 1;
    for i in 0..n {
        let ((xi, yi), (xj, yj)) = (pts[i], pts[j]);
        let den = if yj - yi != 0.0 { yj - yi } else { 1e-9 };
        if ((yi > y) != (yj > y)) && x < (xj - xi) * (y - yi) / den + xi {
            inside = !inside;
        }
        let (dx, dy) = (xj - xi, yj - yi);
        let l = dx * dx + dy * dy;
        let l = if l != 0.0 { l } else { 1e-9 };
        let t = 0f64.max(1f64.min(((x - xi) * dx + (y - yi) * dy) / l));
        best = best.min(hypot(x - xi - t * dx, y - yi - t * dy));
        j = i;
    }
    if inside { best } else { -best }
}

pub fn shape_d(sh: &Shape, x: f64, y: f64) -> f64 {
    match sh {
        Shape::Circle(cx, cy, r) => r - hypot(x - cx, y - cy),
        Shape::Ring(cx, cy, r, w) => {
            let d = hypot(x - cx, y - cy);
            (r - d).min(d - (r - w))
        }
        Shape::Poly(p) => poly_d(p, x, y),
        Shape::RRect(x1, y1, x2, y2, r) => {
            let (cx, cy) = ((x1 + x2) / 2.0, (y1 + y2) / 2.0);
            let qx = (x - cx).abs() - ((x2 - x1) / 2.0 - r);
            let qy = (y - cy).abs() - ((y2 - y1) / 2.0 - r);
            let out = hypot(qx.max(0.0), qy.max(0.0)) + qx.max(qy).min(0.0);
            r - out
        }
    }
}

fn bbox(sh: &Shape) -> (f64, f64, f64, f64) {
    match sh {
        Shape::Circle(cx, cy, r) | Shape::Ring(cx, cy, r, _) => (cx - r - 1.0, cy - r - 1.0, cx + r + 1.0, cy + r + 1.0),
        Shape::Poly(p) => {
            let xs = p.iter().map(|q| q.0);
            let ys = p.iter().map(|q| q.1);
            (xs.clone().fold(f64::INFINITY, f64::min) - 1.0, ys.clone().fold(f64::INFINITY, f64::min) - 1.0,
             xs.fold(f64::NEG_INFINITY, f64::max) + 1.0, ys.fold(f64::NEG_INFINITY, f64::max) + 1.0)
        }
        Shape::RRect(x1, y1, x2, y2, _) => (x1 - 1.0, y1 - 1.0, x2 + 1.0, y2 + 1.0),
    }
}

/// int() of a Python float: toward zero
fn pint(v: f64) -> i64 {
    v.trunc() as i64
}

pub fn aa_pixels(w: usize, h: usize, shapes: &[Sh], bg: Option<[u8; 3]>) -> Photo {
    let mut px: Vec<(f64, f64, f64, f64)> = vec![match bg {
        Some(c) => (c[0] as f64, c[1] as f64, c[2] as f64, 1.0),
        None => (0.0, 0.0, 0.0, 0.0),
    }; w * h];
    for (sh, col) in shapes {
        let (rgb, a0) = match col {
            Some((c, a)) => (Some(hex_rgb(c)), *a),
            None => (None, 1.0),
        };
        let (bx1, by1, bx2, by2) = bbox(sh);
        let ys = 0.max(pint(by1))..(h as i64).min(pint(by2) + 2);
        for y in ys {
            for x in 0.max(pint(bx1))..(w as i64).min(pint(bx2) + 2) {
                let c = cov(shape_d(sh, x as f64 + 0.5, y as f64 + 0.5)) * a0;
                if c <= 0.0 {
                    continue;
                }
                let p = &mut px[y as usize * w + x as usize];
                let (r, g, b, a) = *p;
                let Some(rgb) = rgb else {
                    *p = (r, g, b, a * (1.0 - c));
                    continue;
                };
                let na = c + a * (1.0 - c);
                if na <= 0.0 {
                    continue;
                }
                *p = ((rgb[0] as f64 * c + r * a * (1.0 - c)) / na, (rgb[1] as f64 * c + g * a * (1.0 - c)) / na,
                      (rgb[2] as f64 * c + b * a * (1.0 - c)) / na, na);
            }
        }
    }
    let px = px.iter().map(|&(r, g, b, a)| [r as u8, g as u8, b as u8, (a * 255.0).round_ties_even() as u8]).collect();
    Photo { w, h, px }
}
