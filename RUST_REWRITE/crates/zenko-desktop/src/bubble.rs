//! The speech bubble (zenko.py bubble_window / bubble_click / bubble_hide and the zenko22 say / bubble_spot /
//! bubble_place): its own see-through, always-on-top window that sits on whichever side of her has room.

use crate::gdi::{self, Op, rgb};
use windows_sys::Win32::Foundation::*;
use windows_sys::Win32::Graphics::Gdi::*;
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::UI::WindowsAndMessaging::*;
use zenko_art::palette::mixhex;
use zenko_desktop::theme::{Theme, bubble_style, theme};
use zenko_desktop::{Cfg, Companion};

pub struct Bubble {
    pub hwnd: HWND,
    pub ops_text: Vec<Op>,
    pub ops_frame: Vec<Op>,
    pub fonts: Vec<HFONT>,
    pub bw: i32,
    pub bh: i32,
    pub m: i32,
    pub tail: i32,
    pub u: f64,
    pub side: Option<&'static str>,
    pub aim: Option<(i32, i32)>,
    pub pos: Option<(i32, i32)>,
    pub chips: Vec<(i32, i32, i32, i32, String)>,
    pub hide_at: f64,
    pub shown: bool,
}

pub fn clip(text: &str, n: usize) -> String {
    if text.chars().count() <= n {
        text.to_string()
    } else {
        text.chars().take(n - 1).collect::<String>() + "\u{2026}"
    }
}

/// zenko22 z22_bu: the bubble grows and shrinks with her size.
fn bub_u(cfg: &Cfg, u: f64) -> f64 {
    if !cfg.b("bubble_scale", true) {
        return u;
    }
    let k = ((cfg.f("size", 4.0) - 2.0) / 2.0).powf(0.6);
    let k = if k.is_finite() { k } else { 1.0 };
    u * k.clamp(0.8, 1.9)
}

/// monitor_area: the work area (no taskbar) of the screen holding x, y.
pub fn monitor_area(x: i32, y: i32) -> (i32, i32, i32, i32) {
    unsafe {
        let hm = MonitorFromPoint(POINT { x, y }, MONITOR_DEFAULTTONEAREST);
        let mut mi: MONITORINFO = std::mem::zeroed();
        mi.cbSize = std::mem::size_of::<MONITORINFO>() as u32;
        if !hm.is_null() && GetMonitorInfoW(hm, &mut mi) != 0 {
            let r = mi.rcWork;
            return (r.left, r.top, r.right, r.bottom);
        }
        let mut r = RECT { left: 0, top: 0, right: 0, bottom: 0 };
        SystemParametersInfoW(SPI_GETWORKAREA, 0, &mut r as *mut RECT as *mut _, 0);
        (r.left, r.top, r.right, r.bottom)
    }
}

unsafe extern "system" fn bub_proc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    unsafe {
        match msg {
            WM_PAINT => {
                let mut ps: PAINTSTRUCT = std::mem::zeroed();
                let dc = BeginPaint(hwnd, &mut ps);
                let mut rc = RECT { left: 0, top: 0, right: 0, bottom: 0 };
                GetClientRect(hwnd, &mut rc);
                let (w, h) = (rc.right.max(1), rc.bottom.max(1));
                let mdc = CreateCompatibleDC(dc);
                let bmp = CreateCompatibleBitmap(dc, w, h);
                let ob = SelectObject(mdc, bmp as _);
                let key = CreateSolidBrush(gdi::KEY);
                FillRect(mdc, &rc, key);
                DeleteObject(key as _);
                crate::with_bubble(|b| {
                    gdi::draw(mdc, &b.ops_frame);
                    gdi::draw(mdc, &b.ops_text);
                });
                BitBlt(dc, 0, 0, w, h, mdc, 0, 0, SRCCOPY);
                SelectObject(mdc, ob);
                DeleteObject(bmp as _);
                DeleteDC(mdc);
                EndPaint(hwnd, &ps);
                0
            }
            WM_LBUTTONDOWN => {
                let (x, y) = ((lp & 0xffff) as i16 as i32, ((lp >> 16) & 0xffff) as i16 as i32);
                crate::bubble_click(x, y);
                0
            }
            WM_MOUSEACTIVATE => MA_NOACTIVATE as LRESULT,
            _ => DefWindowProcW(hwnd, msg, wp, lp),
        }
    }
}

pub fn create_window() -> HWND {
    unsafe {
        let hinst = GetModuleHandleW(std::ptr::null());
        let cls = gdi::wide("ZenkoRustBubble");
        let wc = WNDCLASSEXW {
            cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
            style: 0,
            lpfnWndProc: Some(bub_proc),
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: hinst,
            hIcon: std::ptr::null_mut(),
            hCursor: LoadCursorW(std::ptr::null_mut(), IDC_ARROW),
            hbrBackground: std::ptr::null_mut(),
            lpszMenuName: std::ptr::null(),
            lpszClassName: cls.as_ptr(),
            hIconSm: std::ptr::null_mut(),
        };
        RegisterClassExW(&wc);
        let hwnd = CreateWindowExW(
            WS_EX_LAYERED | WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE,
            cls.as_ptr(),
            cls.as_ptr(),
            WS_POPUP,
            0,
            0,
            10,
            10,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            hinst,
            std::ptr::null(),
        );
        SetLayeredWindowAttributes(hwnd, gdi::KEY, 0, LWA_COLORKEY); // Tk's -transparentcolor
        hwnd
    }
}

impl Bubble {
    pub fn new() -> Bubble {
        Bubble {
            hwnd: create_window(),
            ops_text: vec![],
            ops_frame: vec![],
            fonts: vec![],
            bw: 0,
            bh: 0,
            m: 0,
            tail: 0,
            u: 1.0,
            side: None,
            aim: None,
            pos: None,
            chips: vec![],
            hide_at: 0.0,
            shown: false,
        }
    }

    pub fn hide(&mut self) {
        self.shown = false;
        self.chips.clear();
        unsafe { ShowWindow(self.hwnd, SW_HIDE) };
    }

    fn free_fonts(&mut self) {
        for f in self.fonts.drain(..) {
            unsafe { DeleteObject(f as _) };
        }
    }

    /// zenko22 say(text, secs, choices)
    #[allow(clippy::too_many_arguments)]
    pub fn say(&mut self, fox: &Companion, root: (i32, i32), now: f64, text: &str, secs: f64,
               choices: &[(String, String)], dpi: f64, family: &str) {
        let cfg = &fox.cfg;
        let u0 = fox.u;
        let u = bub_u(cfg, u0);
        let t: Theme = theme(cfg);
        let (fill, ink, rim, _inner) = bubble_style(cfg, &t);
        self.ops_text.clear();
        self.ops_frame.clear();
        self.free_fonts();
        let text = clip(text, if choices.is_empty() { 240 } else { 160 });
        let k = u / u0.max(0.5);
        let pad = (13.0 * u) as i32;
        let maxw = (206.0 * u) as i32;
        let tail = (15.0 * u) as i32;
        let m = tail + (4.0 * u) as i32;
        let fs = 8.max((10.0 * k).round_ties_even() as i32);
        let cs = 7.max((9.0 * k).round_ties_even() as i32);
        let f_txt = gdi::tk_font(family, fs, true, dpi);
        let f_chip = gdi::tk_font(family, cs, true, dpi);
        self.fonts.push(f_txt);
        self.fonts.push(f_chip);
        let lay = gdi::layout(f_txt, &text, maxw);
        let (tw, th) = (lay.width, lay.height);
        let ch = (22.0 * u) as i32;
        let (mut cx, mut cy) = (0, 0);
        let mut chips = vec![];
        for (label, cmd) in choices {
            let l = gdi::layout(f_chip, label, 0);
            let w = l.width + (18.0 * u) as i32;
            if cx != 0 && cx + w > maxw {
                cx = 0;
                cy += ch + (6.0 * u) as i32;
            }
            chips.push((cx, cy, w, l, cmd.clone()));
            cx += w + (6.0 * u) as i32;
        }
        let chips_h = if chips.is_empty() { 0 } else { cy + ch + (8.0 * u) as i32 };
        let chips_w = chips.iter().map(|c| c.0 + c.2).max().unwrap_or(0);
        let (bw, bh) = (tw.max(chips_w) + 2 * pad, th + 2 * pad + chips_h);
        self.ops_text.push(Op::Text { x: m + pad, y: m + pad, lay, font: f_txt, color: rgb(&ink), right: false });
        self.chips.clear();
        for (ox, oy, w, l, cmd) in chips {
            let (x1, y1) = (m + pad + ox, m + pad + th + (8.0 * u) as i32 + oy);
            gdi::rbox(&mut self.ops_text, x1, y1, x1 + w, y1 + ch, 99, Some(rgb(&t.acc)), Some(rgb(&rim)), 1, "pill",
                      2.max(u as i32));
            let (lw, lh) = (l.width, l.height);
            self.ops_text.push(Op::Text { x: x1 + w / 2 - lw / 2, y: y1 + ch / 2 - lh / 2, lay: l, font: f_chip,
                                          color: rgb(&t.onacc), right: false });
            self.chips.push((x1, y1, x1 + w, y1 + ch, cmd));
        }
        let _ = fill;
        self.bw = bw;
        self.bh = bh;
        self.m = m;
        self.tail = tail;
        self.u = u;
        self.side = None;
        self.aim = None;
        self.pos = None;
        self.shown = true;
        self.place(fox, root, true);
        unsafe {
            ShowWindow(self.hwnd, SW_SHOWNOACTIVATE);
            SetWindowPos(self.hwnd, HWND_TOPMOST, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE);
        }
        self.hide_at = now + secs;
    }

    /// zenko22 bubble_spot: left of her (the usual), right, above or below, wherever there's room.
    fn spot(&self, fox: &Companion, root: (i32, i32)) -> (&'static str, i32, i32, i32, i32) {
        let (z, u) = (fox.z, self.u);
        let (rx, ry) = root;
        let (fx, fy) = (rx + fox.colw, ry + fox.fox_y);
        let head_x = fx + 36 * z;
        let (l, t, r, bt) = monitor_area(head_x, fy + 40 * z);
        let (bw, bh, m, gap) = (self.bw, self.bh, self.m, (4.0 * u) as i32);
        let ph = (58.0 * fox.u) as i32;
        let cy2 = ry + fox.h - ph - (16.0 * fox.u) as i32;
        let fh = 78 * z;
        let mut c: Vec<(&'static str, i32, i32, bool)> = vec![];
        let x2 = fx - (14.0 * u) as i32;
        let y2 = (bt - gap).min((t + gap + bh).max(cy2));
        c.push(("left", x2 - bw, y2 - bh, x2 - bw >= l + gap));
        let x1 = fx + 68 * z + (10.0 * u) as i32;
        let y2r = (bt - gap).min((t + gap + bh).max(fy + 34 * z));
        c.push(("right", x1, y2r - bh, x1 + bw <= r - gap));
        let ax1 = (l + gap).max((r - gap - bw).min(head_x - bw.div_euclid(2)));
        let ay = fy + 2 * z - (14.0 * u) as i32 - bh;
        c.push(("above", ax1, ay, ay >= t + gap));
        let by_ = fy + fh + (14.0 * u) as i32;
        c.push(("below", ax1, by_, by_ + bh <= bt - gap));
        let (side, bx, by) = c.iter().find(|x| x.3).map(|x| (x.0, x.1, x.2)).unwrap_or((c[0].0, c[0].1, c[0].2));
        let bx = (l + gap).max((r - gap - bw).min(bx));
        let by = (t + gap).max((bt - gap - bh).min(by));
        (side, bx - m, by - m, head_x, fy + 26 * z)
    }

    /// zenko22 bubble_place: draw the frame for this side and move the window.
    pub fn place(&mut self, fox: &Companion, root: (i32, i32), force: bool) {
        if !self.shown {
            return;
        }
        let (side, wx, wy, hx, hy) = self.spot(fox, root);
        if !force && self.pos == Some((wx, wy)) && self.side == Some(side) && self.aim == Some((hx, hy)) {
            return;
        }
        let u = self.u;
        let t = theme(&fox.cfg);
        let (fill, _ink, rim, inner) = bubble_style(&fox.cfg, &t);
        let (bw, bh, m, tail) = (self.bw, self.bh, self.m, self.tail);
        if force || self.side != Some(side) || self.aim != Some((hx, hy)) {
            let ops = &mut self.ops_frame;
            ops.clear();
            let (x1, y1, x2, y2) = (m, m, m + bw, m + bh);
            let r = (16.0 * u) as i32;
            let st = 2.max((2.0 * u) as i32);
            let w = 2.max((2.0 * u) as i32);
            let (fc, rc) = (rgb(&fill), rgb(&rim));
            gdi::rbox(ops, x1, y1, x2, y2, r, Some(fc), Some(rc), w, "round", st);
            let i = (5.0 * u) as i32;
            let inner = inner.unwrap_or_else(|| mixhex(&fill, &rim, 0.35));
            gdi::rbox(ops, x1 + i, y1 + i, x2 - i, y2 - i, 2.max(r - i), None, Some(rgb(&inner)), 1, "round", st);
            let j = 2.max((2.0 * u) as i32);
            let mxx = (x1 + x2).div_euclid(2);
            for (cx, k) in [(mxx - 3 * j, 1), (mxx + 3 * j, 1), (mxx, 2)] {
                let col = if k == 2 { rc } else { rgb(&mixhex(&rim, &fill, 0.3)) };
                ops.push(Op::Oval { x1: cx - k * j, y1: y1 - k * j + 1, x2: cx + k * j, y2: y1 + k * j + 1, fill: col });
            }
            let (ax, ay) = (hx - wx, hy - wy);
            let iu = |v: f64| (v * u) as i32;
            if side == "left" || side == "right" {
                let py = (y1 + iu(16.0)).max((y2 - iu(16.0)).min(if side == "right" { ay } else { y2 - iu(18.0) }));
                let ex = if side == "left" { x2 } else { x1 };
                let tip = ex + if side == "left" { tail - 2 } else { -(tail - 2) };
                ops.push(Op::Poly { pts: vec![(ex, py - iu(8.0)), (tip, py + iu(4.0)), (ex, py + iu(9.0))], fill: Some(fc),
                                    outline: Some(rc), width: w });
                ops.push(Op::Line { pts: vec![(ex, py - iu(8.0) + w), (ex, py + iu(9.0) - w)], color: fc, width: w + 1 });
            } else {
                let px = (x1 + iu(18.0)).max((x2 - iu(18.0)).min(ax));
                let ey = if side == "above" { y2 } else { y1 };
                let tip = ey + if side == "above" { tail - 2 } else { -(tail - 2) };
                ops.push(Op::Poly { pts: vec![(px - iu(8.0), ey), (px + iu(3.0), tip), (px + iu(9.0), ey)], fill: Some(fc),
                                    outline: Some(rc), width: w });
                ops.push(Op::Line { pts: vec![(px - iu(8.0) + w, ey), (px + iu(9.0) - w, ey)], color: fc, width: w + 1 });
            }
            self.side = Some(side);
            self.aim = Some((hx, hy));
            unsafe { InvalidateRect(self.hwnd, std::ptr::null(), 0) };
        }
        if self.pos != Some((wx, wy)) {
            unsafe {
                SetWindowPos(self.hwnd, HWND_TOPMOST, wx, wy, bw + 2 * m, bh + 2 * m, SWP_NOACTIVATE);
            }
            self.pos = Some((wx, wy));
        }
    }
}
