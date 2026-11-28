//! How Tk 8.6 on Windows paints a canvas (tkCanvas.c DisplayCanvas with each item type's display proc): every item
//! becomes the X drawing calls Tk makes for it (XFillRectangle, XDrawRectangle, XFillArc, XDrawArc, XFillPolygon,
//! XDrawLines, the text layout and Tk_RedrawImage), with Tk's own rounding, its bezier smoothing (tkTrig.c
//! TkMakeBezierCurve, 12 steps) and its anchor rules. The Windows side (gdi.rs) turns each call into the same GDI
//! calls tkWinDraw.c makes.

use crate::shapes::tk_coord;
use crate::tkcanvas::{CItem, CKind, Canvas, Photo};
use crate::ui::{Fonts, text_layout};
use std::rc::Rc;

/// an X graphics context's pen: colour, integer line width ((int)(width + 0.5)), cap and join
#[derive(Clone, Debug, PartialEq)]
pub struct TkPen {
    pub color: String,
    pub width: i32,
    pub cap: &'static str,
    pub join: &'static str,
    /// SetUpGraphicsPort's pen style for a dash list: 0 solid, else PS_DASH (1), PS_DOT (2), PS_DASHDOT (3),
    /// PS_DASHDOTDOT (4)
    pub style: u32,
}

#[derive(Clone, Debug, PartialEq)]
pub enum TkOp {
    FillRect { x: i32, y: i32, w: i32, h: i32, color: String },
    /// XFillRectangle with a gray50 stipple: only the pixels where (x + y) is even (in the canvas's own pixels)
    FillStipple { x: i32, y: i32, w: i32, h: i32, color: String },
    DrawRect { x: i32, y: i32, w: i32, h: i32, pen: TkPen },
    /// XFillArc of a whole ellipse; the fill GC's pen is 1 pixel of the fill colour (line width 0)
    FillArc { x: i32, y: i32, w: i32, h: i32, color: String },
    DrawArc { x: i32, y: i32, w: i32, h: i32, pen: TkPen },
    /// XDrawArc of part of an ellipse: `start` / `extent` in 64ths of a degree, counter-clockwise from 3 o'clock
    DrawArcPart { x: i32, y: i32, w: i32, h: i32, start: i32, extent: i32, pen: TkPen },
    /// XFillArc of part of an ellipse with the GC's arc mode (chord: cut by the straight line; else a pie slice)
    FillArcPart { x: i32, y: i32, w: i32, h: i32, start: i32, extent: i32, chord: bool, color: String },
    FillPoly { pts: Vec<(i32, i32)>, color: String },
    DrawLines { pts: Vec<(i32, i32)>, pen: TkPen },
    /// one run of text per line: (x, baseline y, text)
    Text { runs: Vec<(i32, i32, String)>, font: crate::ui::Font, color: String },
    Photo { x: i32, y: i32, img: Rc<Photo> },
    /// Some: draw only inside this box (x, y, w, h) until Clip(None)
    Clip(Option<[i32; 4]>),
}

fn steps(c: &[f64; 8], n: usize, out: &mut Vec<(i32, i32)>) {
    for i in 1..=n {
        let t = i as f64 / n as f64;
        let t2 = t * t;
        let t3 = t2 * t;
        let u = 1.0 - t;
        let u2 = u * u;
        let u3 = u2 * u;
        out.push((tk_coord(c[0] * u3 + 3.0 * (c[2] * t * u2 + c[4] * t2 * u) + c[6] * t3),
                  tk_coord(c[1] * u3 + 3.0 * (c[3] * t * u2 + c[5] * t2 * u) + c[7] * t3)));
    }
}

/// TkMakeBezierCurve: `p` is x0, y0, x1, y1, ...; screen points out
pub fn bezier(p: &[f64], n: usize) -> Vec<(i32, i32)> {
    let np = p.len() / 2;
    let nc = np * 2;
    let mut out = vec![];
    let mut c = [0f64; 8];
    let closed = p[0] == p[nc - 2] && p[1] == p[nc - 1];
    if closed {
        c[0] = 0.5 * p[nc - 4] + 0.5 * p[0];
        c[1] = 0.5 * p[nc - 3] + 0.5 * p[1];
        c[2] = 0.167 * p[nc - 4] + 0.833 * p[0];
        c[3] = 0.167 * p[nc - 3] + 0.833 * p[1];
        c[4] = 0.833 * p[0] + 0.167 * p[2];
        c[5] = 0.833 * p[1] + 0.167 * p[3];
        c[6] = 0.5 * p[0] + 0.5 * p[2];
        c[7] = 0.5 * p[1] + 0.5 * p[3];
        out.push((tk_coord(c[0]), tk_coord(c[1])));
        steps(&c, n, &mut out);
    } else {
        out.push((tk_coord(p[0]), tk_coord(p[1])));
    }
    for i in 2..np {
        let q = &p[(i - 2) * 2..];
        if i == 2 && !closed {
            c[0] = q[0];
            c[1] = q[1];
            c[2] = 0.333 * q[0] + 0.667 * q[2];
            c[3] = 0.333 * q[1] + 0.667 * q[3];
        } else {
            c[0] = 0.5 * q[0] + 0.5 * q[2];
            c[1] = 0.5 * q[1] + 0.5 * q[3];
            c[2] = 0.167 * q[0] + 0.833 * q[2];
            c[3] = 0.167 * q[1] + 0.833 * q[3];
        }
        if i == np - 1 && !closed {
            c[4] = 0.667 * q[2] + 0.333 * q[4];
            c[5] = 0.667 * q[3] + 0.333 * q[5];
            c[6] = q[4];
            c[7] = q[5];
        } else {
            c[4] = 0.833 * q[2] + 0.167 * q[4];
            c[5] = 0.833 * q[3] + 0.167 * q[5];
            c[6] = 0.5 * q[2] + 0.5 * q[4];
            c[7] = 0.5 * q[3] + 0.5 * q[5];
        }
        if (q[0] == q[2] && q[1] == q[3]) || (q[2] == q[4] && q[3] == q[5]) {
            out.push((tk_coord(c[6]), tk_coord(c[7])));
            continue;
        }
        steps(&c, n, &mut out);
    }
    out
}

fn pen(it: &CItem, color: &str) -> TkPen {
    let width = ((it.width + 0.5) as i32).max(0);
    // tkWinDraw.c SetUpGraphicsPort: gc->dashes holds the first bytes of the list, ended by a 0
    let style = if it.dash.is_empty() {
        0
    } else {
        let p = |i: usize| it.dash.get(i).copied().unwrap_or(0);
        if p(1) != 0 && p(2) != 0 {
            if p(3) == 0 || p(4) != 0 { 4 } else { 3 }
        } else if p(0) as i32 > 4 * width {
            1
        } else {
            2
        }
    };
    TkPen { color: color.into(), width, cap: it.capstyle, join: it.joinstyle, style }
}

fn pts(c: &[f64]) -> Vec<(i32, i32)> {
    c.chunks(2).map(|p| (tk_coord(p[0]), tk_coord(p[1]))).collect()
}

/// tkRectOval.c DisplayRectOval: the corners, widened by a pixel when they round to the same column/row
fn rect_box(c: &[f64]) -> (i32, i32, i32, i32) {
    let b = [c[0].min(c[2]), c[1].min(c[3]), c[0].max(c[2]), c[1].max(c[3])];
    let (mut x1, mut y1, mut x2, mut y2) = (tk_coord(b[0]), tk_coord(b[1]), tk_coord(b[2]), tk_coord(b[3]));
    let widen = |a1: &mut i32, a2: &mut i32, lo: f64, hi: f64| {
        let (i1, i2) = (lo as i16 as i32, hi as i16 as i32);
        if *a2 != *a1 {
            return;
        }
        let grow_hi = if i1 == i2 {
            i1 > 0
        } else if i1 > 0 {
            hi - i2 as f64 > i2 as f64 - lo
        } else {
            hi - i1 as f64 > i1 as f64 - lo
        };
        if grow_hi { *a2 += 1 } else { *a1 -= 1 }
    };
    widen(&mut x1, &mut x2, b[0], b[2]);
    widen(&mut y1, &mut y2, b[1], b[3]);
    (x1, y1, x2, y2)
}

/// tkCanvText.c ComputeTextBbox: the laid-out lines, width, height and the anchor offsets (dx, dy)
fn text_geom(it: &CItem, fonts: &dyn Fonts) -> (Vec<String>, i32, i32, f64, f64) {
    let f = it.font.as_ref().expect("text item without a font");
    let (lines, mut w) = text_layout(fonts, f, &it.text, it.wrap as i32);
    let mut h = fonts.linespace(f) * lines.len() as i32;
    if it.hidden || it.fill.is_empty() {
        w = 0;
        h = 0;
    }
    let dy = match it.anchor {
        "nw" | "n" | "ne" => 0,
        "sw" | "s" | "se" => -h,
        _ => -h / 2,
    };
    let dx = match it.anchor {
        "nw" | "w" | "sw" => 0,
        "ne" | "e" | "se" => -w,
        _ => -w / 2,
    };
    (lines, w, h, dx as f64, dy as f64)
}

/// Tk's bbox for a text item (the layout box plus the insert-cursor fudge of 1 on each side)
pub fn text_bbox(it: &CItem, fonts: &dyn Fonts) -> [i32; 4] {
    let (_, w, h, dx, dy) = text_geom(it, fonts);
    let r = |v: f64| (v + 0.5).floor() as i32;
    let (x, y) = (it.coords[0], it.coords[1]);
    [r(x + dx - 1.0), r(y + dy), r(x + dx + w as f64 + 1.0), r(y + dy + h as f64)]
}

/// an item's bbox as `cv.bbox(id)` gives it, for text and (round-joined) polygons and lines: tkCanvPoly.c
/// ComputePolygonBbox includes every control point (smooth or not), grows by (width+1.5)/2 for the outline and one
/// more pixel of fudge
pub fn item_bbox(cv: &crate::tkcanvas::Canvas, id: u32, fonts: &dyn Fonts) -> Option<[i32; 4]> {
    let it = cv.items.iter().find(|i| i.id == id)?;
    if it.hidden {
        return None;
    }
    match it.kind {
        crate::tkcanvas::CKind::Text => Some(text_bbox(it, fonts)),
        crate::tkcanvas::CKind::Poly | crate::tkcanvas::CKind::Line if it.coords.len() >= 2 => {
            let c = &it.coords;
            let (mut x1, mut y1) = (c[0] as i32, c[1] as i32);
            let (mut x2, mut y2) = (x1, y1);
            let n = c.len() / 2;
            let closed = it.kind == crate::tkcanvas::CKind::Poly && (c[0] != c[2 * n - 2] || c[1] != c[2 * n - 1]);
            let np = if closed { n + 1 } else { n };
            let last = if it.kind == crate::tkcanvas::CKind::Poly { np - 1 } else { n };
            for i in 1..last.min(n) {
                let (px, py) = ((c[2 * i] + 0.5) as i32, (c[2 * i + 1] + 0.5) as i32);
                x1 = x1.min(px);
                x2 = x2.max(px);
                y1 = y1.min(py);
                y2 = y2.max(py);
            }
            if !it.outline.is_empty() || it.kind == crate::tkcanvas::CKind::Line {
                let w = it.width.max(1.0);
                let i = ((w + 1.5) / 2.0) as i32;
                (x1, y1, x2, y2) = (x1 - i, y1 - i, x2 + i, y2 + i);
            }
            Some([x1 - 1, y1 - 1, x2 + 1, y2 + 1])
        }
        _ => None,
    }
}

/// `cv.bbox(tag)`: the union of the boxes of the items with the tag (empty ones left out)
pub fn tag_bbox(cv: &crate::tkcanvas::Canvas, tag: &str, fonts: &dyn Fonts) -> Option<[i32; 4]> {
    let mut out: Option<[i32; 4]> = None;
    for it in cv.items.iter().filter(|i| i.tags.iter().any(|t| t == tag) || i.id.to_string() == tag) {
        let Some(b) = item_bbox(cv, it.id, fonts) else { continue };
        if b[0] >= b[2] || b[1] >= b[3] {
            continue;
        }
        out = Some(match out {
            None => b,
            Some(o) => [o[0].min(b[0]), o[1].min(b[1]), o[2].max(b[2]), o[3].max(b[3])],
        });
    }
    out
}

/// tkCanvImg.c ComputeImageBbox: the top-left corner of an image item
pub fn image_xy(it: &CItem) -> (i32, i32) {
    let half = |v: f64| (v + if v >= 0.0 { 0.5 } else { -0.5 }) as i32;
    let (mut x, mut y) = (half(it.coords[0]), half(it.coords[1]));
    let Some(p) = &it.image else { return (x, y) };
    let (w, h) = (p.w as i32, p.h as i32);
    match it.anchor {
        "n" => x -= w / 2,
        "ne" => x -= w,
        "e" => {
            x -= w;
            y -= h / 2
        }
        "se" => {
            x -= w;
            y -= h
        }
        "s" => {
            x -= w / 2;
            y -= h
        }
        "sw" => y -= h,
        "w" => y -= h / 2,
        "nw" => {}
        _ => {
            x -= w / 2;
            y -= h / 2
        }
    }
    (x, y)
}

/// the drawing calls for one item
pub fn item_ops(it: &CItem, fonts: &dyn Fonts, out: &mut Vec<TkOp>) {
    if it.hidden {
        return;
    }
    match it.kind {
        CKind::Rect | CKind::Oval => {
            let (x1, y1, x2, y2) = rect_box(&it.coords);
            let (w, h) = (x2 - x1, y2 - y1);
            if !it.fill.is_empty() {
                out.push(if it.kind == CKind::Rect && it.stipple == "gray50" {
                    TkOp::FillStipple { x: x1, y: y1, w, h, color: it.fill.clone() }
                } else if it.kind == CKind::Rect {
                    TkOp::FillRect { x: x1, y: y1, w, h, color: it.fill.clone() }
                } else {
                    TkOp::FillArc { x: x1, y: y1, w, h, color: it.fill.clone() }
                });
            }
            if !it.outline.is_empty() {
                let p = pen(it, &it.outline);
                out.push(if it.kind == CKind::Rect {
                    TkOp::DrawRect { x: x1, y: y1, w, h, pen: p }
                } else {
                    TkOp::DrawArc { x: x1, y: y1, w, h, pen: p }
                });
            }
        }
        CKind::Line => {
            if it.coords.len() < 2 || it.fill.is_empty() {
                return;
            }
            let p = if it.smooth && it.coords.len() > 4 { bezier(&it.coords, 12) } else { pts(&it.coords) };
            let pn = pen(it, &it.fill);
            if p.len() > 1 {
                out.push(TkOp::DrawLines { pts: p, pen: pn });
            } else {
                // a single point: XFillArc of a dot the line's width
                let iw = pn.width.max(1);
                out.push(TkOp::FillArc { x: p[0].0 - iw / 2, y: p[0].1 - iw / 2, w: iw + 1, h: iw + 1, color: pn.color });
            }
        }
        CKind::Poly => {
            let mut c = it.coords.clone();
            let n = c.len();
            if n > 2 && (c[n - 2] != c[0] || c[n - 1] != c[1]) {
                c.push(c[0]);
                c.push(c[1]);
            }
            let np = c.len() / 2;
            if np < 1 || (it.fill.is_empty() && it.outline.is_empty()) || (np < 3 && it.outline.is_empty()) {
                return;
            }
            if np < 3 {
                let iw = pen(it, &it.outline).width.max(1);
                let (x, y) = (tk_coord(c[0]), tk_coord(c[1]));
                out.push(TkOp::FillArc { x: x - iw / 2, y: y - iw / 2, w: iw + 1, h: iw + 1, color: it.outline.clone() });
                return;
            }
            let (p, min_fill) = if !it.smooth || np < 4 { (pts(&c), 4) } else { (bezier(&c, 12), 0) };
            if !it.fill.is_empty() && p.len() >= min_fill {
                out.push(TkOp::FillPoly { pts: p.clone(), color: it.fill.clone() });
            }
            if !it.outline.is_empty() {
                out.push(TkOp::DrawLines { pts: p, pen: pen(it, &it.outline) });
            }
        }
        CKind::Text => {
            if it.fill.is_empty() || it.text.is_empty() {
                return;
            }
            let f = it.font.clone().unwrap();
            let (lines, _, _, dx, dy) = text_geom(it, fonts);
            let (x, y) = (tk_coord(it.coords[0] + dx), tk_coord(it.coords[1] + dy));
            let (asc, _) = fonts.metrics(&f);
            let ls = fonts.linespace(&f);
            // -justify: each line's place in the layout's width (Tk_ComputeTextLayout's justification)
            let (_, lw, _, _, _) = text_geom(it, fonts);
            let lx = |l: &str| match it.justify {
                "center" => (lw - fonts.width(&f, l)) / 2,
                "right" => lw - fonts.width(&f, l),
                _ => 0,
            };
            let runs: Vec<(i32, i32, String)> = lines.into_iter().enumerate().filter(|(_, l)| !l.is_empty())
                .map(|(i, l)| (x + lx(&l), y + asc + i as i32 * ls, l)).collect();
            let unders: Vec<TkOp> = if it.underline {
                // Tk_UnderlineChars: desc / 2 under the baseline, a tenth of the size high (at least 1)
                let desc = fonts.metrics(&f).1;
                let mut pos = desc / 2;
                let mut uh = ((fonts.pixels(&f) / 10.0 + 0.5) as i32).max(1);
                if pos + uh > desc {
                    uh = desc - pos;
                    if uh == 0 {
                        pos -= 1;
                        uh = 1;
                    }
                }
                runs.iter().map(|(rx, by, l)| TkOp::FillRect { x: *rx, y: by + pos, w: fonts.width(&f, l), h: uh,
                                                              color: it.fill.clone() }).collect()
            } else {
                vec![]
            };
            out.push(TkOp::Text { runs, font: f, color: it.fill.clone() });
            out.extend(unders);
        }
        CKind::Arc => {
            // tkCanvArc.c DisplayArc (style arc: the outline only; pieslice / chord fills aren't used by zenko.py)
            let (x1, y1) = (tk_coord(it.coords[0].min(it.coords[2])), tk_coord(it.coords[1].min(it.coords[3])));
            let (mut x2, mut y2) = (tk_coord(it.coords[0].max(it.coords[2])), tk_coord(it.coords[1].max(it.coords[3])));
            if x2 <= x1 {
                x2 = x1 + 1;
            }
            if y2 <= y1 {
                y2 = y1 + 1;
            }
            let start = (64.0 * it.start + 0.5) as i32;
            let extent = (64.0 * it.extent + 0.5) as i32;
            if !it.fill.is_empty() && it.style != "arc" && extent != 0 {
                out.push(TkOp::FillArcPart { x: x1, y: y1, w: x2 - x1, h: y2 - y1, start, extent, chord: it.style == "chord",
                                             color: it.fill.clone() });
            }
            if !it.outline.is_empty() && extent != 0 && it.style == "arc" {
                out.push(TkOp::DrawArcPart { x: x1, y: y1, w: x2 - x1, h: y2 - y1, start, extent, pen: pen(it, &it.outline) });
            }
        }
        CKind::Image => {
            if let Some(img) = &it.image {
                let (x, y) = image_xy(it);
                out.push(TkOp::Photo { x, y, img: img.clone() });
            }
        }
    }
}

/// the whole canvas, bottom item first
pub fn render(cv: &Canvas, fonts: &dyn Fonts) -> Vec<TkOp> {
    let mut out = vec![];
    for it in &cv.items {
        item_ops(it, fonts, &mut out);
    }
    out
}

/// Tk's photo blending on Windows (tkWinImage BlendComplexAlpha): (bg * (255 - a) + img * a) / 255, see-through
/// pixels skipped. `dst` is RGB rows `dw` wide.
pub fn blend_photo(dst: &mut [[u8; 3]], dw: usize, dh: usize, x: i32, y: i32, img: &Photo) {
    for py in 0..img.h {
        let ty = y + py as i32;
        if ty < 0 || ty >= dh as i32 {
            continue;
        }
        for px in 0..img.w {
            let tx = x + px as i32;
            if tx < 0 || tx >= dw as i32 {
                continue;
            }
            let s = img.px[py * img.w + px];
            let a = s[3] as u32;
            if a == 0 {
                continue;
            }
            let d = &mut dst[ty as usize * dw + tx as usize];
            for k in 0..3 {
                d[k] = ((d[k] as u32 * (255 - a) + s[k] as u32 * a) / 255) as u8;
            }
        }
    }
}
