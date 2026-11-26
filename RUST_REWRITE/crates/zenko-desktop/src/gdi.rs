//! A small stand-in for the Tk canvas on Windows: Tk draws with GDI, so this draws the same shapes with the same GDI
//! calls (Polygon with even-odd fill and a mitred geometric pen, Pie/Ellipse with no pen, TextOutW in the same fonts)
//! and keeps them as a display list that a window replays when it paints.

use windows_sys::Win32::Foundation::*;
use windows_sys::Win32::Graphics::Gdi::*;

pub fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

/// "#rrggbb" -> COLORREF
pub fn rgb(hex: &str) -> u32 {
    let h = hex.trim_start_matches('#');
    let v = u32::from_str_radix(h.get(..6).unwrap_or("000000"), 16).unwrap_or(0);
    let (r, g, b) = ((v >> 16) & 255, (v >> 8) & 255, v & 255);
    r | (g << 8) | (b << 16)
}

pub const KEY: u32 = 0x00ff00ff; // #ff00ff, the see-through colour

/// The fonts zenko.py carries (Zen Maru Gothic, Zen Old Mincho), loaded for this app only (like AddFontResourceEx
/// with FR_PRIVATE in zenko21): nothing is installed.
pub fn load_fonts() {
    static FONTS: [&[u8]; 3] = [
        include_bytes!("../../../assets/fonts/ZenMaruGothic-Regular.ttf"),
        include_bytes!("../../../assets/fonts/ZenMaruGothic-Bold.ttf"),
        include_bytes!("../../../assets/fonts/ZenOldMincho-Bold.ttf"),
    ];
    for f in FONTS {
        let mut n = 0u32;
        unsafe {
            AddFontMemResourceEx(f.as_ptr() as _, f.len() as u32, std::ptr::null(), &mut n);
        }
    }
}

pub fn font_exists(name: &str) -> bool {
    unsafe extern "system" fn cb(_: *const LOGFONTW, _: *const TEXTMETRICW, _: u32, lp: LPARAM) -> i32 {
        unsafe { *(lp as *mut bool) = true };
        0
    }
    let mut found = false;
    unsafe {
        let dc = GetDC(std::ptr::null_mut());
        let mut lf: LOGFONTW = std::mem::zeroed();
        lf.lfCharSet = DEFAULT_CHARSET as u8;
        for (i, c) in name.encode_utf16().take(31).enumerate() {
            lf.lfFaceName[i] = c;
        }
        EnumFontFamiliesExW(dc, &lf, Some(cb), &mut found as *mut bool as LPARAM, 0);
        ReleaseDC(std::ptr::null_mut(), dc);
    }
    found
}

/// zenko.py set_ui_font: the chosen font if Windows has it, else Zen Maru Gothic, else Segoe UI.
pub fn ui_font(wanted: &str) -> String {
    for n in [wanted, "Zen Maru Gothic", "Segoe UI Variable Display", "Segoe UI Variable Text", "Segoe UI"] {
        if !n.is_empty() && font_exists(n) {
            return n.to_string();
        }
    }
    "Segoe UI".into()
}

/// Tk's font size in points -> pixels (TkFontGetPixels: points * dpi / 72, rounded).
pub fn tk_font(family: &str, points: i32, bold: bool, dpi: f64) -> HFONT {
    let px = (points as f64 * dpi / 72.0 + 0.5) as i32;
    let name = wide(family);
    unsafe {
        CreateFontW(
            -px,
            0,
            0,
            0,
            if bold { FW_BOLD as i32 } else { FW_NORMAL as i32 },
            0,
            0,
            0,
            DEFAULT_CHARSET as u32,
            OUT_TT_PRECIS as u32,
            CLIP_DEFAULT_PRECIS as u32,
            DEFAULT_QUALITY as u32,
            (DEFAULT_PITCH as u32) | (FF_DONTCARE as u32),
            name.as_ptr(),
        )
    }
}

pub struct TextLayout {
    pub lines: Vec<Vec<u16>>,
    pub width: i32,
    pub height: i32,
    pub line_h: i32,
}

fn extent(dc: HDC, s: &[u16]) -> i32 {
    let mut sz = SIZE { cx: 0, cy: 0 };
    unsafe { GetTextExtentPoint32W(dc, s.as_ptr(), s.len() as i32, &mut sz) };
    sz.cx
}

/// Tk_ComputeTextLayout: words wrapped at `wrap` pixels (0 = no wrap), newlines kept, trailing spaces not counted.
pub fn layout(font: HFONT, text: &str, wrap: i32) -> TextLayout {
    unsafe {
        let dc = CreateCompatibleDC(std::ptr::null_mut());
        let old = SelectObject(dc, font as _);
        let mut tm: TEXTMETRICW = std::mem::zeroed();
        GetTextMetricsW(dc, &mut tm);
        let line_h = tm.tmAscent + tm.tmDescent;
        let mut lines: Vec<Vec<u16>> = vec![];
        let mut width = 0;
        for para in text.split('\n') {
            let u: Vec<u16> = para.encode_utf16().collect();
            if wrap <= 0 || extent(dc, &u) <= wrap {
                width = width.max(extent(dc, trim_end(&u)));
                lines.push(u);
                continue;
            }
            let mut start = 0usize;
            while start < u.len() {
                // the longest run of whole words from `start` that fits
                let mut end = start;
                let mut best = start;
                while end < u.len() {
                    let mut e = end;
                    while e < u.len() && u[e] == b' ' as u16 {
                        e += 1;
                    }
                    while e < u.len() && u[e] != b' ' as u16 {
                        e += 1;
                    }
                    if extent(dc, &u[start..e]) <= wrap {
                        best = e;
                        end = e;
                    } else {
                        break;
                    }
                }
                if best == start {
                    // not even one word fits: as many characters as fit (at least one)
                    let mut e = start + 1;
                    while e < u.len() && extent(dc, &u[start..e + 1]) <= wrap {
                        e += 1;
                    }
                    best = e;
                }
                let line = u[start..best].to_vec();
                width = width.max(extent(dc, trim_end(&line)));
                lines.push(line);
                start = best;
                while start < u.len() && u[start] == b' ' as u16 {
                    start += 1;
                }
            }
            if u.is_empty() {
                lines.push(vec![]);
            }
        }
        SelectObject(dc, old);
        DeleteDC(dc);
        let height = line_h * lines.len() as i32;
        TextLayout { lines, width, height, line_h }
    }
}

fn trim_end(s: &[u16]) -> &[u16] {
    let mut e = s.len();
    while e > 0 && s[e - 1] == b' ' as u16 {
        e -= 1;
    }
    &s[..e]
}

pub enum Op {
    /// create_polygon(points, fill, outline, width, joinstyle="miter")
    Poly { pts: Vec<(i32, i32)>, fill: Option<u32>, outline: Option<u32>, width: i32 },
    /// create_oval(x1, y1, x2, y2, fill, outline="")
    Oval { x1: i32, y1: i32, x2: i32, y2: i32, fill: u32 },
    /// create_line(points, fill, width)
    Line { pts: Vec<(i32, i32)>, color: u32, width: i32 },
    /// create_text (already laid out); x, y = top left; justify: 0 left, 1 right
    Text { x: i32, y: i32, lay: TextLayout, font: HFONT, color: u32, right: bool },
}

pub fn pen(color: u32, width: i32) -> HPEN {
    unsafe {
        if width <= 1 {
            CreatePen(PS_SOLID, 1, color)
        } else {
            let lb = LOGBRUSH { lbStyle: BS_SOLID, lbColor: color, lbHatch: 0 };
            ExtCreatePen((PS_GEOMETRIC | PS_SOLID | PS_ENDCAP_FLAT | PS_JOIN_MITER) as u32, width as u32, &lb, 0, std::ptr::null())
        }
    }
}

pub unsafe fn draw(dc: HDC, ops: &[Op]) {
    unsafe {
        SetPolyFillMode(dc, ALTERNATE);
        SetBkMode(dc, TRANSPARENT as i32);
        for op in ops {
            match op {
                Op::Poly { pts, fill, outline, width } => {
                    let p: Vec<POINT> = pts.iter().map(|&(x, y)| POINT { x, y }).collect();
                    if let Some(f) = fill {
                        let br = CreateSolidBrush(*f);
                        let ob = SelectObject(dc, br as _);
                        let op_ = SelectObject(dc, GetStockObject(NULL_PEN));
                        Polygon(dc, p.as_ptr(), p.len() as i32);
                        SelectObject(dc, op_);
                        SelectObject(dc, ob);
                        DeleteObject(br as _);
                    }
                    if let Some(o) = outline {
                        let pn = pen(*o, *width);
                        let opn = SelectObject(dc, pn as _);
                        let mut q = p.clone();
                        q.push(p[0]);
                        Polyline(dc, q.as_ptr(), q.len() as i32);
                        SelectObject(dc, opn);
                        DeleteObject(pn as _);
                    }
                }
                Op::Oval { x1, y1, x2, y2, fill } => {
                    let br = CreateSolidBrush(*fill);
                    let ob = SelectObject(dc, br as _);
                    let op_ = SelectObject(dc, GetStockObject(NULL_PEN));
                    Ellipse(dc, *x1, *y1, *x2 + 1, *y2 + 1);
                    SelectObject(dc, op_);
                    SelectObject(dc, ob);
                    DeleteObject(br as _);
                }
                Op::Line { pts, color, width } => {
                    let p: Vec<POINT> = pts.iter().map(|&(x, y)| POINT { x, y }).collect();
                    let pn = pen(*color, *width);
                    let opn = SelectObject(dc, pn as _);
                    Polyline(dc, p.as_ptr(), p.len() as i32);
                    SelectObject(dc, opn);
                    DeleteObject(pn as _);
                }
                Op::Text { x, y, lay, font, color, right } => {
                    let of = SelectObject(dc, *font as _);
                    SetTextColor(dc, *color);
                    for (i, l) in lay.lines.iter().enumerate() {
                        let lx = if *right { x + lay.width - extent(dc, trim_end(l)) } else { *x };
                        TextOutW(dc, lx, y + i as i32 * lay.line_h, l.as_ptr(), l.len() as i32);
                    }
                    SelectObject(dc, of);
                }
            }
        }
    }
}

pub use zenko_desktop::shapes::round_pts;

pub fn rbox(ops: &mut Vec<Op>, x1: i32, y1: i32, x2: i32, y2: i32, r: i32, fill: Option<u32>, outline: Option<u32>,
            width: i32, shape: &str, step: i32) {
    let pts = round_pts(x1, y1, x2, y2, r, step, shape);
    ops.push(Op::Poly { pts, fill, outline, width });
}
