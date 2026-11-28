//! Windows host for zenko_desktop::ui windows: GDI fonts behind the `Fonts` trait, the display list turned into
//! gdi::Op, and a borderless always-on-top popup (Tk's overrideredirect + -topmost + -transparentcolor KEY) that
//! feeds the pointer and keys back into the widget tree. What its buttons ask for goes to OUT for the app loop.

use crate::gdi::{self, Op, rgb};
use std::cell::RefCell;
use std::collections::HashMap;
use windows_sys::Win32::Foundation::*;
use windows_sys::Win32::Graphics::Gdi::*;
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::*;
use windows_sys::Win32::UI::WindowsAndMessaging::*;
use zenko_desktop::theme::Theme;
use zenko_desktop::ui::{Cursor, Draw, Font, Fonts, Ui};

/// HFONTs made the way Tk makes them (points * dpi / 72), measured with GDI
pub struct GdiFonts {
    dpi: f64,
    cache: RefCell<HashMap<Font, HFONT>>,
}

impl GdiFonts {
    pub fn new(dpi: f64) -> GdiFonts {
        GdiFonts { dpi, cache: RefCell::new(HashMap::new()) }
    }
    pub fn h(&self, f: &Font) -> HFONT {
        *self.cache.borrow_mut().entry(f.clone()).or_insert_with(|| gdi::tk_font_s(&f.family, f.size, f.bold, f.italic, self.dpi))
    }
}

impl Drop for GdiFonts {
    fn drop(&mut self) {
        for (_, h) in self.cache.borrow_mut().drain() {
            unsafe { DeleteObject(h as _) };
        }
    }
}

impl Fonts for GdiFonts {
    fn width(&self, f: &Font, s: &str) -> i32 {
        let w: Vec<u16> = s.encode_utf16().collect();
        crate::tkw::text_width(self.h(f), &w)
    }
    fn pixels(&self, f: &Font) -> f64 {
        f.size as f64 * self.dpi / 72.0
    }
    fn metrics(&self, f: &Font) -> (i32, i32) {
        let m = crate::tkw::metrics(self.h(f));
        (m.ascent, m.descent)
    }
}

/// the display list as GDI ops
/// the plain drawing calls (a Tk canvas's own calls, Draw::Tk, are painted by tkpaint and left out here)
pub fn to_ops(fonts: &GdiFonts, d: &[Draw]) -> Vec<Op> {
    d.iter().filter(|x| !matches!(x, Draw::Tk { .. })).map(|x| match x {
        Draw::Rect { x, y, w, h, color } => Op::Rect { x: *x, y: *y, w: *w, h: *h, color: rgb(color) },
        Draw::Poly { pts, fill, outline } => Op::Poly { pts: pts.clone(), fill: fill.as_deref().map(rgb),
                                                        outline: outline.as_deref().map(rgb), width: 1 },
        Draw::Line { pts, color } => Op::Line { pts: pts.clone(), color: rgb(color), width: 1 },
        Draw::Oval { x1, y1, x2, y2, fill } => Op::Oval { x1: *x1, y1: *y1, x2: *x2, y2: *y2, fill: rgb(fill) },
        Draw::Text { x, y, text, font, color } => {
            let h = fonts.h(font);
            Op::Text { x: *x, y: *y, lay: gdi::layout(h, text, 0), font: h, color: rgb(color), right: false }
        }
        Draw::Clip(c) => Op::Clip(*c),
        Draw::Tk { .. } => unreachable!(),
    }).collect()
}

pub struct UiWin {
    pub name: String,
    pub hwnd: HWND,
    pub ui: Ui,
    fonts: GdiFonts,
    t: Theme,
    t0: std::time::Instant,
    /// a normal titled, resizable window (a Toplevel) rather than a borderless popup
    framed: bool,
}

/// a button press: which window, its command, and the text of every named entry
pub struct UiOut {
    pub win: String,
    pub cmd: String,
    pub values: Vec<(String, String)>,
}

thread_local! {
    static WINS: RefCell<Vec<UiWin>> = const { RefCell::new(vec![]) };
    pub static OUT: RefCell<Vec<UiOut>> = const { RefCell::new(vec![]) };
}

pub fn is_open(name: &str) -> bool {
    WINS.with(|w| w.try_borrow().map(|v| v.iter().any(|x| x.name == name)).unwrap_or(true))
}

pub fn lift(name: &str) {
    WINS.with(|w| {
        if let Ok(v) = w.try_borrow() {
            if let Some(x) = v.iter().find(|x| x.name == name) {
                unsafe { SetWindowPos(x.hwnd, HWND_TOPMOST, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE) };
            }
        }
    });
}

pub fn close(name: &str) {
    let gone: Vec<UiWin> = WINS.with(|w| {
        let Ok(mut v) = w.try_borrow_mut() else { return vec![] };
        let (out, keep): (Vec<UiWin>, Vec<UiWin>) = v.drain(..).partition(|x| x.name == name);
        *v = keep;
        out
    });
    for x in gone {
        unsafe { DestroyWindow(x.hwnd) };
    }
}

/// open a popup of `ui` laid out at its requested size; `place(w, h)` says where its top-left goes
pub fn open_popup(name: &str, mut ui: Ui, t: Theme, dpi: f64, place: impl FnOnce(i32, i32) -> (i32, i32)) {
    let fonts = GdiFonts::new(dpi);
    ui.layout(&fonts);
    let (w, h) = (ui.ws[0].w, ui.ws[0].h);
    let (x, y) = place(w, h);
    unsafe {
        let hinst = GetModuleHandleW(std::ptr::null());
        let cls = gdi::wide("ZenkoRustTkPopup");
        let wc = WNDCLASSEXW {
            cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
            style: CS_DBLCLKS,
            lpfnWndProc: Some(proc_),
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
        let hwnd = CreateWindowExW(WS_EX_LAYERED | WS_EX_TOPMOST | WS_EX_TOOLWINDOW, cls.as_ptr(), cls.as_ptr(), WS_POPUP,
                                   x, y, w, h, std::ptr::null_mut(), std::ptr::null_mut(), hinst, std::ptr::null());
        SetLayeredWindowAttributes(hwnd, gdi::KEY, 0, LWA_COLORKEY);
        WINS.with(|v| v.borrow_mut().push(UiWin { name: name.into(), hwnd, ui, fonts, t, t0: std::time::Instant::now(), framed: false }));
        ShowWindow(hwnd, SW_SHOWNOACTIVATE);
        SetTimer(hwnd, 1, 100, None);
    }
}

/// open a titled, resizable window (tk.Toplevel with title and geometry "WxH+X+Y": the client area is w x h at x, y).
/// Its clicks that no button takes, wheel turns, keys and size changes go to OUT as "click:x:y", "wheel:delta",
/// "key:vk", "resize:w:h" and "closed", for the app to answer with `with_ui`.
pub fn open_window(name: &str, title: &str, ui: Ui, t: Theme, dpi: f64, x: i32, y: i32, w: i32, h: i32) {
    let fonts = GdiFonts::new(dpi);
    unsafe {
        let hinst = GetModuleHandleW(std::ptr::null());
        let cls = gdi::wide("ZenkoRustTkWindow");
        let wc = WNDCLASSEXW {
            cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
            style: CS_DBLCLKS,
            lpfnWndProc: Some(proc_),
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: hinst,
            hIcon: LoadIconW(hinst, 1 as _),
            hCursor: LoadCursorW(std::ptr::null_mut(), IDC_ARROW),
            hbrBackground: std::ptr::null_mut(),
            lpszMenuName: std::ptr::null(),
            lpszClassName: cls.as_ptr(),
            hIconSm: std::ptr::null_mut(),
        };
        RegisterClassExW(&wc);
        let mut r = RECT { left: x, top: y, right: x + w, bottom: y + h };
        AdjustWindowRectEx(&mut r, WS_OVERLAPPEDWINDOW, 0, 0);
        let tw = gdi::wide(title);
        let hwnd = CreateWindowExW(0, cls.as_ptr(), tw.as_ptr(), WS_OVERLAPPEDWINDOW, x, y,
                                   r.right - r.left, r.bottom - r.top, std::ptr::null_mut(), std::ptr::null_mut(), hinst,
                                   std::ptr::null());
        let mut ui = ui;
        ui.layout_at(&fonts, w, h);
        WINS.with(|v| v.borrow_mut().push(UiWin { name: name.into(), hwnd, ui, fonts, t, t0: std::time::Instant::now(), framed: true }));
        ShowWindow(hwnd, SW_SHOW);
        SetForegroundWindow(hwnd);
        SetTimer(hwnd, 1, 100, None);
    }
}

/// change a window's widgets (and lay it out again at its size), then repaint it
pub fn with_ui<R>(name: &str, f: impl FnOnce(&mut Ui, &dyn Fonts) -> R) -> Option<R> {
    let mut hwnd = std::ptr::null_mut();
    let r = WINS.with(|w| {
        let mut v = w.try_borrow_mut().ok()?;
        let x = v.iter_mut().find(|x| x.name == name)?;
        hwnd = x.hwnd;
        Some(f(&mut x.ui, &x.fonts))
    });
    if !hwnd.is_null() {
        unsafe { InvalidateRect(hwnd, std::ptr::null(), 0) };
    }
    r
}

/// bring a window to the front (deiconify + lift)
pub fn raise(name: &str) {
    WINS.with(|w| {
        if let Ok(v) = w.try_borrow()
            && let Some(x) = v.iter().find(|x| x.name == name) {
            unsafe {
                ShowWindow(x.hwnd, SW_RESTORE);
                SetForegroundWindow(x.hwnd);
            }
        }
    });
}

fn out(hwnd: HWND, cmd: String) {
    let mut o = None;
    with(hwnd, |x| {
        if x.framed {
            o = Some(UiOut { win: x.name.clone(), cmd, values: vec![] });
        }
    });
    if let Some(o) = o {
        OUT.with(|q| q.borrow_mut().push(o));
    }
}

/// the display list painted: plain calls with GDI, a Tk canvas's own calls with tkpaint at its place
unsafe fn paint_list(mdc: HDC, fonts: &GdiFonts, d: Vec<Draw>) {
    unsafe {
        let mut run: Vec<Draw> = vec![];
        for item in d {
            if let Draw::Tk { x: ox, y: oy, ops } = item {
                gdi::draw(mdc, &to_ops(fonts, &run));
                run.clear();
                let mut old = POINT { x: 0, y: 0 };
                SetViewportOrgEx(mdc, ox, oy, &mut old);
                crate::tkpaint::paint(mdc, &ops, fonts);
                SetViewportOrgEx(mdc, old.x, old.y, std::ptr::null_mut());
            } else {
                run.push(item);
            }
        }
        gdi::draw(mdc, &to_ops(fonts, &run));
    }
}

fn with(hwnd: HWND, f: impl FnOnce(&mut UiWin)) {
    WINS.with(|w| {
        if let Ok(mut v) = w.try_borrow_mut() {
            if let Some(x) = v.iter_mut().find(|x| x.hwnd == hwnd) {
                f(x);
            }
        }
    });
}

fn now(x: &UiWin) -> f64 {
    x.t0.elapsed().as_secs_f64()
}

fn values(ui: &Ui) -> Vec<(String, String)> {
    ui.ws.iter().enumerate().filter(|(_, w)| !w.name.is_empty()).map(|(i, w)| (w.name.clone(), ui.text_of(i))).collect()
}

unsafe extern "system" fn proc_(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    unsafe {
        let pt = || ((lp & 0xffff) as i16 as i32, ((lp >> 16) & 0xffff) as i16 as i32);
        let redraw = || {
            InvalidateRect(hwnd, std::ptr::null(), 0);
        };
        match msg {
            WM_PAINT => {
                let mut ps: PAINTSTRUCT = std::mem::zeroed();
                let dc = BeginPaint(hwnd, &mut ps);
                with(hwnd, |x| {
                    let d = x.ui.draw(&x.fonts, &x.t, now(x));
                    let (w, h) = (x.ui.ws[0].w, x.ui.ws[0].h);
                    let mdc = CreateCompatibleDC(dc);
                    let bmp = CreateCompatibleBitmap(dc, w, h);
                    let ob = SelectObject(mdc, bmp as _);
                    paint_list(mdc, &x.fonts, d);
                    BitBlt(dc, 0, 0, w, h, mdc, 0, 0, SRCCOPY);
                    SelectObject(mdc, ob);
                    DeleteObject(bmp as _);
                    DeleteDC(mdc);
                });
                EndPaint(hwnd, &ps);
                0
            }
            WM_ERASEBKGND => 1,
            WM_TIMER => {
                redraw();
                0
            }
            WM_ACTIVATE => {
                let on = (wp & 0xffff) != 0;
                with(hwnd, |x| x.ui.active = on);
                redraw();
                0
            }
            WM_MOUSEACTIVATE => MA_ACTIVATE as isize,
            WM_SIZE => {
                let (w, h) = ((lp & 0xffff) as i32, ((lp >> 16) & 0xffff) as i32);
                let mut framed = false;
                with(hwnd, |x| {
                    framed = x.framed;
                    if x.framed && w > 0 && h > 0 {
                        x.ui.layout_at(&x.fonts, w, h);
                    }
                });
                if framed && w > 0 && h > 0 {
                    out(hwnd, format!("resize:{w}:{h}"));
                    redraw();
                }
                0
            }
            WM_MOUSEWHEEL => {
                let delta = ((wp >> 16) & 0xffff) as i16 as i32;
                out(hwnd, format!("wheel:{delta}"));
                redraw();
                0
            }
            WM_CLOSE => {
                let mut framed = false;
                with(hwnd, |x| framed = x.framed);
                if framed {
                    out(hwnd, "closed".into());
                    let name = WINS.with(|w| w.try_borrow().ok().and_then(|v| v.iter().find(|x| x.hwnd == hwnd).map(|x| x.name.clone())));
                    if let Some(n) = name {
                        close(&n);
                    }
                    return 0;
                }
                DefWindowProcW(hwnd, msg, wp, lp)
            }
            WM_MOUSEMOVE => {
                let (mx, my) = pt();
                let mut tme = TRACKMOUSEEVENT { cbSize: std::mem::size_of::<TRACKMOUSEEVENT>() as u32, dwFlags: TME_LEAVE,
                                                hwndTrack: hwnd, dwHoverTime: 0 };
                TrackMouseEvent(&mut tme);
                let mut cur = Cursor::Arrow;
                with(hwnd, |x| {
                    let n = now(x);
                    x.ui.motion(&x.fonts, mx, my, n);
                    cur = x.ui.cursor_at(mx, my);
                });
                let c = match cur {
                    Cursor::Hand => IDC_HAND,
                    Cursor::IBeam => IDC_IBEAM,
                    Cursor::Arrow => IDC_ARROW,
                };
                SetCursor(LoadCursorW(std::ptr::null_mut(), c));
                redraw();
                0
            }
            0x02A3 => {
                with(hwnd, |x| x.ui.leave());
                redraw();
                0
            }
            WM_SETCURSOR => {
                if (lp & 0xffff) as u32 == HTCLIENT {
                    return 1;
                }
                DefWindowProcW(hwnd, msg, wp, lp)
            }
            WM_LBUTTONDOWN | WM_LBUTTONDBLCLK => {
                let (mx, my) = pt();
                SetCapture(hwnd);
                SetForegroundWindow(hwnd);
                SetFocus(hwnd);
                with(hwnd, |x| {
                    let n = now(x);
                    x.ui.press(&x.fonts, mx, my, n, msg == WM_LBUTTONDBLCLK);
                });
                redraw();
                0
            }
            WM_LBUTTONUP => {
                ReleaseCapture();
                let (mx, my) = pt();
                let mut out = None;
                with(hwnd, |x| {
                    if let Some(cmd) = x.ui.release(mx, my) {
                        out = Some(UiOut { win: x.name.clone(), cmd, values: values(&x.ui) });
                    } else if x.framed {
                        out = Some(UiOut { win: x.name.clone(), cmd: format!("click:{mx}:{my}"), values: vec![] });
                    }
                });
                if let Some(o) = out {
                    OUT.with(|q| q.borrow_mut().push(o));
                }
                redraw();
                0
            }
            WM_KEYDOWN => {
                let vk = wp as u16;
                out(hwnd, format!("key:{vk}"));
                let shift = GetKeyState(VK_SHIFT as i32) < 0;
                let ctrl = GetKeyState(VK_CONTROL as i32) < 0;
                with(hwnd, |x| {
                    let n = now(x);
                    if ctrl && vk == b'A' as u16 {
                        x.ui.select_all(&x.fonts);
                    } else if ctrl && (vk == b'C' as u16 || vk == b'X' as u16) {
                        if let Some(s) = x.ui.selected() {
                            crate::dialog::clipboard_set(&s);
                            if vk == b'X' as u16 {
                                x.ui.insert(&x.fonts, &[], n);
                            }
                        }
                    } else if ctrl && vk == b'V' as u16 {
                        if let Some(s) = crate::dialog::clipboard_get() {
                            let s: Vec<u16> = s.into_iter().filter(|&c| c != 13 && c != 10).collect();
                            x.ui.insert(&x.fonts, &s, n);
                        }
                    } else {
                        x.ui.key(&x.fonts, vk, shift, ctrl, n);
                    }
                });
                redraw();
                0
            }
            WM_CHAR => {
                let c = wp as u16;
                if c >= 0x20 && c != 0x7f && GetKeyState(VK_CONTROL as i32) >= 0 {
                    with(hwnd, |x| {
                        let n = now(x);
                        x.ui.insert(&x.fonts, &[c], n);
                    });
                    redraw();
                }
                0
            }
            _ => DefWindowProcW(hwnd, msg, wp, lp),
        }
    }
}
