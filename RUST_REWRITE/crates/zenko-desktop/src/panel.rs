//! The timer panel beside her during a session (zenko.py draw_panel + pixel_box), drawn with the same GDI calls Tk
//! uses, onto her window.

use crate::gdi::{self, Op, rgb};
use windows_sys::Win32::Graphics::Gdi::*;
use zenko_art::palette::mixhex;
use zenko_desktop::Companion;
use zenko_desktop::session::clip;
use zenko_desktop::shapes::{round_pts_f, tk_coord};
use zenko_desktop::theme::Theme;

/// Fonts for the panel, made once per (family, dpi).
pub struct Fonts {
    key: (String, i32),
    pub task: HFONT,
    pub time: HFONT,
    pub small: HFONT,
}

impl Fonts {
    pub fn get<'a>(slot: &'a mut Option<Fonts>, family: &str, dpi: f64) -> &'a Fonts {
        let key = (family.to_string(), dpi as i32);
        if slot.as_ref().map(|f| &f.key) != Some(&key) {
            if let Some(f) = slot.take() {
                unsafe {
                    DeleteObject(f.task as _);
                    DeleteObject(f.time as _);
                    DeleteObject(f.small as _);
                }
            }
            *slot = Some(Fonts {
                key,
                task: gdi::tk_font(family, 9, true, dpi),
                time: gdi::tk_font(family, 15, true, dpi),
                small: gdi::tk_font(family, 8, false, dpi),
            });
        }
        slot.as_ref().unwrap()
    }
}

fn oval(ops: &mut Vec<Op>, x1: f64, y1: f64, x2: f64, y2: f64, fill: u32) {
    let (a, b, mut c, d) = (tk_coord(x1), tk_coord(y1), tk_coord(x2), tk_coord(y2));
    if c <= a {
        c = a + 1;
    }
    ops.push(Op::Oval { x1: a, y1: b, x2: c, y2: d.max(b + 1), fill });
}

#[allow(clippy::too_many_arguments)]
fn rbox_f(ops: &mut Vec<Op>, x1: f64, y1: f64, x2: f64, y2: f64, r: i32, fill: Option<u32>, outline: Option<u32>,
          width: i32, shape: &str, step: i32) {
    let pts = round_pts_f(x1, y1, x2, y2, r, step, shape);
    ops.push(Op::Poly { pts, fill, outline, width });
}

/// pixel_box: a lacquered box with round pixel-stepped corners, a double gold rim and a little gold knot on top.
pub fn pixel_box(ops: &mut Vec<Op>, u: f64, t: &Theme, x1: i32, y1: i32, x2: i32, y2: i32, fill: &str, shape: &str) {
    let r = (16.0 * u) as i32;
    let st = 2.max((2.0 * u) as i32);
    let w = 2.max((2.0 * u) as i32);
    gdi::rbox(ops, x1, y1, x2, y2, r, Some(rgb(fill)), Some(rgb(&t.gold)), w, shape, st);
    let i = (5.0 * u) as i32;
    gdi::rbox(ops, x1 + i, y1 + i, x2 - i, y2 - i, 2.max(r - i), None, Some(rgb(&mixhex(fill, &t.gold, 0.35))), 1,
              shape, st);
    let j = 2.max((2.0 * u) as i32);
    let mxx = (x1 + x2).div_euclid(2);
    for (cx, k) in [(mxx - 3 * j, 1), (mxx + 3 * j, 1), (mxx, 2)] {
        let f = if k == 2 { t.gold.clone() } else { mixhex(&t.gold, fill, 0.3) };
        oval(ops, (cx - k * j) as f64, (y1 - k * j + 1) as f64, (cx + k * j) as f64, (y1 + k * j + 1) as f64, rgb(&f));
    }
}

fn text(ops: &mut Vec<Op>, font: HFONT, x: i32, y: i32, s: &str, color: &str, anchor_ne: bool) {
    let lay = gdi::layout(font, s, 0);
    let x = if anchor_ne { x - lay.width } else { x };
    ops.push(Op::Text { x, y, lay, font, color: rgb(color), right: anchor_ne });
}

/// draw_panel. Returns nothing to draw when there is no session.
pub fn ops(fox: &Companion, t: &Theme, fonts: &Fonts) -> Vec<Op> {
    let mut ops = vec![];
    let Some(s) = &fox.session else { return ops };
    let u = fox.u;
    let x2 = fox.colw - (14.0 * u) as i32;
    let x1 = (6.0 * u) as i32;
    let y2 = fox.h - (8.0 * u) as i32;
    let ph = (58.0 * u) as i32;
    let y1 = y2 - ph;
    pixel_box(&mut ops, u, t, x1, y1, x2, y2, &t.bub, "leaf");
    let left = (s.planned - s.elapsed).max(0.0) as i64;
    let (m, sec) = (left.div_euclid(60), left.rem_euclid(60));
    let pad = (13.0 * u) as i32;
    text(&mut ops, fonts.task, x1 + pad, y1 + (7.0 * u) as i32, &clip(&s.task, 26), &t.bubink, false);
    let label = format!("{m:02}:{sec:02}{}", if s.paused { "  paused" } else if s.idle_paused { "  away" } else { "" });
    text(&mut ops, fonts.time, x1 + pad, y1 + (19.0 * u) as i32, &label, if s.paused { &t.grey } else { &t.gold },
         false);
    let on = (s.elapsed - s.dist - s.away).max(0.0);
    let right = format!("on task {}m\noff task {}m", (on / 60.0).floor() as i64, (s.dist / 60.0).floor() as i64);
    text(&mut ops, fonts.small, x2 - pad, y1 + (14.0 * u) as i32, &right, &t.grey, true);
    let (bx1, bx2) = (x1 + pad, x2 - pad);
    let (by1, by2) = (y2 - (14.0 * u) as i32, y2 - (8.0 * u) as i32);
    let frac = (s.elapsed / s.planned).min(1.0);
    let st = 2.max(u as i32);
    gdi::rbox(&mut ops, bx1, by1, bx2, by2, 99, Some(rgb(&mixhex(&t.bub, "#000000", 0.3))),
              Some(rgb(&mixhex(&t.bub, &t.gold, 0.5))), 1, "pill", st);
    if frac > 0.0 {
        let fx = bx1 as f64 + (bx2 - bx1) as f64 * frac;
        if fx - bx1 as f64 > (2 * st) as f64 {
            rbox_f(&mut ops, (bx1 + 1) as f64, (by1 + 1) as f64, fx, (by2 - 1) as f64, 99, Some(rgb(&t.acc)), None, 1,
                   "pill", st);
        }
        let k = 2.max((2.0 * u) as i32) as f64;
        oval(&mut ops, fx - k - 1.0, by1 as f64 - k, fx + k + 1.0, by2 as f64 + k, rgb(&t.glow));
        let k2 = (k as i64).div_euclid(2) as f64;
        oval(&mut ops, fx - k2, (by1 + 1) as f64, fx + k2, (by2 - 1) as f64, 0xffffff);
    }
    ops
}
