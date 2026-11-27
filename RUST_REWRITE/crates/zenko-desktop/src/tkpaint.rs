//! Tk 8.6's Windows drawing (tkWinDraw.c) for the canvas calls canvasdraw.rs makes: the same pens
//! (SetUpGraphicsPort), paths (RenderObject / MakeAndStrokePath / StrokeInPieces), arcs (DrawOrFillArc), text drawn
//! on its baseline and pictures blended the way Tk's photo images are.

use crate::uiwin::GdiFonts;
use crate::gdi::rgb;
use windows_sys::Win32::Foundation::*;
use windows_sys::Win32::Graphics::Gdi::*;
use zenko_desktop::canvasdraw::{TkOp, TkPen, blend_photo};

const MAX_STROKE_POINTS: usize = 64;
const STROKE_PIECE: usize = 32;

/// SetUpGraphicsPort
unsafe fn make_pen(p: &TkPen) -> HPEN {
    unsafe {
        let c = rgb(&p.color);
        if p.width < 2 {
            return CreatePen(PS_SOLID, p.width, c);
        }
        let lb = LOGBRUSH { lbStyle: BS_SOLID, lbColor: c, lbHatch: 0 };
        let cap = match p.cap {
            "butt" => PS_ENDCAP_FLAT,
            "round" => PS_ENDCAP_ROUND,
            _ => PS_ENDCAP_SQUARE,
        };
        let join = match p.join {
            "miter" => PS_JOIN_MITER,
            "round" => PS_JOIN_ROUND,
            _ => PS_JOIN_BEVEL,
        };
        ExtCreatePen((PS_GEOMETRIC | PS_SOLID | cap | join) as u32, p.width as u32, &lb, 0, std::ptr::null())
    }
}

fn before(a: POINT, b: POINT) -> bool {
    a.x < b.x || (a.x == b.x && a.y < b.y)
}

/// ThinLineEndPoint: a 1-pixel line drops its last pixel; Tk adds it back (or reverses the line)
fn thin_end(width: i32, cap: &str, p: &mut [POINT]) -> bool {
    let n = p.len();
    if width != 1 || n < 2 || cap == "notlast" || (p[0].x == p[n - 1].x && p[0].y == p[n - 1].y) {
        return false;
    }
    let first = !before(p[1], p[0]);
    let last = before(p[n - 1], p[n - 2]);
    if !last {
        return false;
    }
    if !first {
        p.reverse();
        return false;
    }
    true
}

unsafe fn stroke(dc: HDC, p: &[POINT]) {
    unsafe {
        BeginPath(dc);
        Polyline(dc, p.as_ptr(), p.len() as i32);
        EndPath(dc);
        StrokePath(dc);
    }
}

unsafe fn end_cap(dc: HDC, p: &[POINT], start: usize, step: isize, width: u32) {
    unsafe {
        let mut seg = vec![p[start]];
        let mut len = 0.0;
        let mut n = 1usize;
        while n < p.len() && n < STROKE_PIECE {
            let pt = p[(start as isize + n as isize * step) as usize];
            let (dx, dy) = ((pt.x - seg[n - 1].x) as f64, (pt.y - seg[n - 1].y) as f64);
            let sl = dx.hypot(dy);
            if len + sl > 2.0 * width as f64 {
                let f = (2.0 * width as f64 - len) / sl;
                seg.push(POINT { x: seg[n - 1].x + (dx * f) as i32, y: seg[n - 1].y + (dy * f) as i32 });
                break;
            }
            seg.push(pt);
            len += sl;
            n += 1;
        }
        stroke(dc, &seg);
    }
}

/// StrokeInPieces: long geometric polylines are stroked 32 points at a time, with flat ends, then capped
unsafe fn stroke_pieces(dc: HDC, p: &[POINT], closed: bool, pen: &TkPen) {
    unsafe {
        let npoints = p.len();
        let flat = pen.cap != "butt" && pen.cap != "notlast";
        let mut fp: HPEN = std::ptr::null_mut();
        let old = GetCurrentObject(dc, OBJ_PEN as u32);
        if flat {
            fp = make_pen(&TkPen { cap: "butt", ..pen.clone() });
            SelectObject(dc, fp as _);
        }
        let total = if closed { npoints + 2 } else { npoints };
        let mut i = 0;
        while i + 2 < total {
            let n = (total - i).min(STROKE_PIECE);
            let piece: Vec<POINT> = (0..n).map(|j| p[if closed { (i + j) % (npoints - 1) } else { i + j }]).collect();
            stroke(dc, &piece);
            i += STROKE_PIECE - 2;
        }
        if flat {
            SelectObject(dc, old);
            DeleteObject(fp as _);
            if !closed {
                end_cap(dc, p, 0, 1, pen.width as u32);
                end_cap(dc, p, npoints - 1, -1, pen.width as u32);
            }
        }
    }
}

/// DrawOrFillArc for a whole ellipse: start and end both at three o'clock
fn arc_ends(x: i32, y: i32, w: i32, h: i32) -> (i32, i32) {
    let xr = x as f64 + w as f64 / 2.0;
    let yr = y as f64 + h as f64 / 2.0;
    ((xr + w as f64 / 2.0 + 0.5) as i32, (yr + 0.5) as i32)
}

/// paint the ops onto `dc` (a memory DC: pictures read back what is under them)
pub unsafe fn paint(dc: HDC, ops: &[TkOp], fonts: &GdiFonts) {
    unsafe {
        SetBkMode(dc, TRANSPARENT as i32);
        SetPolyFillMode(dc, ALTERNATE);
        for op in ops {
            match op {
                TkOp::FillRect { x, y, w, h, color } => {
                    let br = CreateSolidBrush(rgb(color));
                    FillRect(dc, &RECT { left: *x, top: *y, right: x + w, bottom: y + h }, br);
                    DeleteObject(br as _);
                }
                TkOp::DrawRect { x, y, w, h, pen } => {
                    let pn = make_pen(pen);
                    let op_ = SelectObject(dc, pn as _);
                    let ob = SelectObject(dc, GetStockObject(NULL_BRUSH));
                    Rectangle(dc, *x, *y, x + w + 1, y + h + 1);
                    SelectObject(dc, ob);
                    DeleteObject(SelectObject(dc, op_));
                }
                TkOp::FillArc { x, y, w, h, color } => {
                    let c = rgb(color);
                    let pn = CreatePen(PS_SOLID, 0, c);
                    let br = CreateSolidBrush(c);
                    let op_ = SelectObject(dc, pn as _);
                    let ob = SelectObject(dc, br as _);
                    let (xs, ys) = arc_ends(*x, *y, *w, *h);
                    Pie(dc, *x, *y, x + w + 1, y + h + 1, xs, ys, xs, ys);
                    DeleteObject(SelectObject(dc, ob));
                    DeleteObject(SelectObject(dc, op_));
                }
                TkOp::DrawArc { x, y, w, h, pen } => {
                    let pn = make_pen(pen);
                    let op_ = SelectObject(dc, pn as _);
                    let (xs, ys) = arc_ends(*x, *y, *w, *h);
                    Arc(dc, *x, *y, x + w + 1, y + h + 1, xs, ys, xs, ys);
                    DeleteObject(SelectObject(dc, op_));
                }
                TkOp::FillPoly { pts, color } => {
                    let p: Vec<POINT> = pts.iter().map(|&(x, y)| POINT { x, y }).collect();
                    let br = CreateSolidBrush(rgb(color));
                    let op_ = SelectObject(dc, GetStockObject(NULL_PEN));
                    let ob = SelectObject(dc, br as _);
                    BeginPath(dc);
                    Polygon(dc, p.as_ptr(), p.len() as i32);
                    EndPath(dc);
                    StrokeAndFillPath(dc);
                    SelectObject(dc, op_);
                    DeleteObject(SelectObject(dc, ob));
                }
                TkOp::DrawLines { pts, pen } => {
                    let mut p: Vec<POINT> = pts.iter().map(|&(x, y)| POINT { x, y }).collect();
                    let end = thin_end(pen.width, pen.cap, &mut p);
                    let pn = make_pen(pen);
                    let op_ = SelectObject(dc, pn as _);
                    let n = p.len();
                    let closed = p[0].x == p[n - 1].x && p[0].y == p[n - 1].y;
                    if n > MAX_STROKE_POINTS && pen.width >= 2 {
                        stroke_pieces(dc, &p, closed, pen);
                    } else {
                        BeginPath(dc);
                        Polyline(dc, p.as_ptr(), n as i32);
                        if closed {
                            CloseFigure(dc);
                        }
                        EndPath(dc);
                        StrokePath(dc);
                    }
                    if end {
                        MoveToEx(dc, p[n - 1].x, p[n - 1].y, std::ptr::null_mut());
                        LineTo(dc, p[n - 1].x + 1, p[n - 1].y);
                    }
                    DeleteObject(SelectObject(dc, op_));
                }
                TkOp::Text { runs, font, color } => {
                    let of = SelectObject(dc, fonts.h(font) as _);
                    SetTextColor(dc, rgb(color));
                    let oa = SetTextAlign(dc, TA_LEFT | TA_BASELINE);
                    for (x, y, s) in runs {
                        let w: Vec<u16> = s.encode_utf16().collect();
                        TextOutW(dc, *x, *y, w.as_ptr(), w.len() as i32);
                    }
                    SetTextAlign(dc, oa);
                    SelectObject(dc, of);
                }
                TkOp::Photo { x, y, img } => {
                    // read what is under the picture, blend on the CPU as Tk does, put it back
                    let (w, h) = (img.w as i32, img.h as i32);
                    let mut bi: BITMAPINFO = std::mem::zeroed();
                    bi.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
                    bi.bmiHeader.biWidth = w;
                    bi.bmiHeader.biHeight = -h;
                    bi.bmiHeader.biPlanes = 1;
                    bi.bmiHeader.biBitCount = 32;
                    bi.bmiHeader.biCompression = BI_RGB;
                    let mut bits: *mut core::ffi::c_void = std::ptr::null_mut();
                    let mdc = CreateCompatibleDC(dc);
                    let bmp = CreateDIBSection(mdc, &bi, DIB_RGB_COLORS, &mut bits, std::ptr::null_mut(), 0);
                    let ob = SelectObject(mdc, bmp as _);
                    BitBlt(mdc, 0, 0, w, h, dc, *x, *y, SRCCOPY);
                    GdiFlush();
                    let buf = std::slice::from_raw_parts_mut(bits as *mut u8, (w * h * 4) as usize);
                    let mut rgbs: Vec<[u8; 3]> = buf.chunks(4).map(|q| [q[2], q[1], q[0]]).collect();
                    blend_photo(&mut rgbs, img.w, img.h, 0, 0, img);
                    for (q, c) in buf.chunks_mut(4).zip(&rgbs) {
                        q[0] = c[2];
                        q[1] = c[1];
                        q[2] = c[0];
                    }
                    BitBlt(dc, *x, *y, w, h, mdc, 0, 0, SRCCOPY);
                    SelectObject(mdc, ob);
                    DeleteObject(bmp as _);
                    DeleteDC(mdc);
                }
            }
        }
    }
}
