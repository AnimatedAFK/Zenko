//! "Zenko - new focus session" (zenko.py open_dialog): the same widgets, fonts, colours and Tk grid layout, drawn
//! with GDI in a normal window with the coloured title bar style_titlebar gives it.

use crate::gdi::{self, Op, rgb};
use crate::tkw::{Entry, Label, PixelButton};
use std::cell::RefCell;
use std::rc::Rc;
use windows_sys::Win32::Foundation::*;
use windows_sys::Win32::Graphics::Dwm::DwmSetWindowAttribute;
use windows_sys::Win32::Graphics::Gdi::*;
use windows_sys::Win32::System::DataExchange::*;
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::System::Memory::*;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::*;
use windows_sys::Win32::UI::WindowsAndMessaging::*;
use zenko_art::palette::mixhex;
use zenko_desktop::theme::Theme;
use zenko_desktop::tkgrid::{Cell, grid, pack_left, pack_top_w};

pub struct Dialog {
    pub hwnd: HWND,
    t: Theme,
    fonts: Vec<HFONT>,
    statics: Vec<Op>,
    entries: [Entry; 2],
    focus: usize,
    active: bool,
    presets: Vec<PixelButton>,
    start: PixelButton,
    pressed: Option<usize>,
    dragging: bool,
    size: (i32, i32),
    t0: std::time::Instant,
    icon: HICON,
}

thread_local! {
    pub static DLG: RefCell<Option<Dialog>> = const { RefCell::new(None) };
    /// what the dialog asks the app to do: Some(task, minutes) to start, or a ui sound to play
    pub static OUT: RefCell<Vec<Out>> = const { RefCell::new(vec![]) };
}

pub enum Out {
    Go(String, String, i64),
    Sound(&'static str),
}

pub fn is_open() -> bool {
    DLG.with(|d| d.try_borrow().map(|g| g.is_some()).unwrap_or(true))
}

pub fn lift() {
    DLG.with(|d| {
        if let Ok(g) = d.try_borrow() {
            if let Some(dl) = g.as_ref() {
                unsafe {
                    SetForegroundWindow(dl.hwnd);
                }
            }
        }
    });
}

/// str(cfg.get("last_minutes", 25)) as Python prints it
pub fn py_str(v: Option<&serde_json::Value>) -> String {
    match v {
        None => "25".into(),
        Some(serde_json::Value::Number(n)) => {
            if let Some(i) = n.as_i64() {
                i.to_string()
            } else {
                let f = n.as_f64().unwrap_or(25.0);
                if f.fract() == 0.0 && f.abs() < 1e16 { format!("{f:.1}") } else { format!("{f}") }
            }
        }
        Some(serde_json::Value::String(s)) => s.clone(),
        Some(serde_json::Value::Bool(b)) => if *b { "True".into() } else { "False".into() },
        Some(other) => other.to_string(),
    }
}

pub struct Setup {
    pub theme: Theme,
    pub font_ui: String,
    pub dpi: f64,
    pub last_task: String,
    pub last_minutes: String,
    pub hint: String,
    pub fox: (i32, i32, Vec<[u8; 4]>),
    pub icon_px: Vec<[u8; 4]>,
}

fn make_icon(px: &[[u8; 4]]) -> HICON {
    unsafe {
        let mut bi: BITMAPINFO = std::mem::zeroed();
        bi.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
        bi.bmiHeader.biWidth = 64;
        bi.bmiHeader.biHeight = -64;
        bi.bmiHeader.biPlanes = 1;
        bi.bmiHeader.biBitCount = 32;
        bi.bmiHeader.biCompression = BI_RGB;
        let mut bits: *mut core::ffi::c_void = std::ptr::null_mut();
        let dc = GetDC(std::ptr::null_mut());
        let color = CreateDIBSection(dc, &bi, DIB_RGB_COLORS, &mut bits, std::ptr::null_mut(), 0);
        ReleaseDC(std::ptr::null_mut(), dc);
        let buf = std::slice::from_raw_parts_mut(bits as *mut u8, 64 * 64 * 4);
        for (i, p) in px.iter().enumerate() {
            buf[i * 4] = p[2];
            buf[i * 4 + 1] = p[1];
            buf[i * 4 + 2] = p[0];
            buf[i * 4 + 3] = p[3];
        }
        let mask = CreateBitmap(64, 64, 1, 1, std::ptr::null());
        let ii = ICONINFO { fIcon: 1, xHotspot: 0, yHotspot: 0, hbmMask: mask, hbmColor: color };
        let h = CreateIconIndirect(&ii);
        DeleteObject(mask as _);
        DeleteObject(color as _);
        h
    }
}

pub fn open(s: Setup) {
    unsafe {
        let t = s.theme.clone();
        let f = |pt: i32, bold: bool, italic: bool| gdi::tk_font_s(&s.font_ui, pt, bold, italic, s.dpi);
        let (f15b, f10, f9i, f11, f9b, f12b) = (f(15, true, false), f(10, false, false), f(9, false, true),
                                                f(11, false, false), f(9, true, false), f(12, true, false));
        let fonts = vec![f15b, f10, f9i, f11, f9b, f12b];
        // header: the fox picture, then the title lines packed under each other
        let (fw, fh, fpx) = s.fox;
        let icon_lab = (fw + 4, fh + 4);
        let mut lines = vec![Label::new(f15b, "Begin a focus ritual", &t.gold, 0),
                             Label::new(f10, "I'll guard you from distractions.", &t.grey, 0)];
        if !s.hint.is_empty() {
            lines.push(Label::new(f9i, &s.hint, &mixhex(&t.grey, &t.gold, 0.4), 330));
        }
        let sizes: Vec<(i32, i32)> = lines.iter().map(|l| l.size()).collect();
        let (tt, tt_pos) = pack_top_w(&sizes);
        let (hdr, hdr_pos) = pack_left(&[(icon_lab.0, icon_lab.1, 0, 10), (tt.0, tt.1, 0, 0)]);
        let lab_task = Label::new(f10, "What are you working on?", &t.ink, 0);
        let lab_min = Label::new(f10, "Minutes", &t.ink, 0);
        let mut e_task = Entry::new(f11, &s.last_task, 42);
        let mut e_min = Entry::new(f11, &s.last_minutes, 6);
        let mut presets: Vec<PixelButton> =
            ["15", "25", "45", "60"].iter().map(|v| PixelButton::new(v, f9b, &t.field, &t.ink, 8, 1)).collect();
        let pre_items: Vec<(i32, i32, i32, i32)> = presets.iter().map(|b| { let r = b.req(); (r.0, r.1, 2, 2) }).collect();
        let (pre, pre_pos) = pack_left(&pre_items);
        let mut start = PixelButton::new("Start", f12b, &t.acc, &t.onacc, 20, 5);
        let (ew, eh) = e_task.req();
        let (mw, mh) = e_min.req();
        let cells = vec![
            Cell::new(0, 0, hdr.0, hdr.1).span(4).sticky("w").pady(0, 10),
            Cell::new(1, 0, lab_task.size().0, lab_task.size().1).span(4).sticky("w"),
            Cell::new(2, 0, ew, eh).span(4).pady(4, 12).sticky("ew").ipad(0, 3),
            Cell::new(3, 0, lab_min.size().0, lab_min.size().1).sticky("w"),
            Cell::new(3, 1, mw, mh).sticky("w").ipad(0, 2),
            Cell::new(3, 2, pre.0, pre.1).span(2).sticky("e"),
            Cell::new(4, 0, start.req().0, start.req().1).span(4).pady(16, 0).sticky("ew"),
        ];
        let (bx, by) = (1 + 20, 1 + 16);
        let ((gw, gh), at) = grid(&cells, bx, by);
        let (cw, ch) = (gw + 40 + 2, gh + 32 + 2);
        // the parts that never change
        let mut ops = vec![Op::Rect { x: 0, y: 0, w: cw, h: ch, color: rgb(&t.gold) },
                           Op::Rect { x: 1, y: 1, w: cw - 2, h: ch - 2, color: rgb(&t.paper) }];
        let (hx, hy) = (at[0].0, at[0].1);
        ops.push(Op::Img { x: hx + hdr_pos[0].0 + 2, y: hy + hdr_pos[0].1 + 2, w: fw, h: fh, px: Rc::new(fpx) });
        let (ttx, tty) = (hx + hdr_pos[1].0, hy + hdr_pos[1].1);
        for (l, p) in lines.into_iter().zip(tt_pos) {
            l.draw(&mut ops, ttx + p.0, tty + p.1);
        }
        lab_task.draw(&mut ops, at[1].0, at[1].1);
        lab_min.draw(&mut ops, at[3].0, at[3].1);
        e_task.rect = at[2];
        e_min.rect = at[4];
        for (b, p) in presets.iter_mut().zip(pre_pos) {
            let r = b.req();
            b.rect = (at[5].0 + p.0, at[5].1 + p.1, r.0, r.1);
        }
        start.rect = (at[6].0, at[6].1, at[6].2, start.req().1);
        e_task.focus = true;
        // the window: a Tk toplevel that can't be resized, always on top
        let hinst = GetModuleHandleW(std::ptr::null());
        let cls = gdi::wide("ZenkoRustDialog");
        let wc = WNDCLASSEXW {
            cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
            style: CS_DBLCLKS,
            lpfnWndProc: Some(wndproc),
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
        let style = WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_MINIMIZEBOX;
        let mut r = RECT { left: 0, top: 0, right: cw, bottom: ch };
        AdjustWindowRectEx(&mut r, style, 0, WS_EX_TOPMOST);
        let (sw, sh) = (GetSystemMetrics(SM_CXSCREEN), GetSystemMetrics(SM_CYSCREEN));
        let (x, y) = ((sw - cw).div_euclid(2), (sh - ch).div_euclid(3));
        let title = gdi::wide("Zenko - new focus session");
        let hwnd = CreateWindowExW(WS_EX_TOPMOST, cls.as_ptr(), title.as_ptr(), style, x, y, r.right - r.left,
                                   r.bottom - r.top, std::ptr::null_mut(), std::ptr::null_mut(), hinst,
                                   std::ptr::null());
        let icon = make_icon(&s.icon_px);
        SendMessageW(hwnd, WM_SETICON, ICON_BIG as usize, icon as isize);
        SendMessageW(hwnd, WM_SETICON, ICON_SMALL as usize, icon as isize);
        // style_titlebar: dark title bar, NIGHT caption, INK text, GOLD border, rounded corners
        let bgr = |h: &str| rgb(h);
        for (attr, val) in [(20u32, 1u32), (35, bgr(&t.night)), (36, bgr(&t.ink)), (34, bgr(&t.gold)), (33, 2)] {
            DwmSetWindowAttribute(hwnd, attr, &val as *const u32 as *const _, 4);
        }
        DLG.with(|d| {
            *d.borrow_mut() = Some(Dialog {
                hwnd,
                t,
                fonts,
                statics: ops,
                entries: [e_task, e_min],
                focus: 0,
                active: true,
                presets,
                start,
                pressed: None,
                dragging: false,
                size: (cw, ch),
                t0: std::time::Instant::now(),
                icon,
            })
        });
        ShowWindow(hwnd, SW_SHOW);
        SetForegroundWindow(hwnd);
        SetFocus(hwnd);
        SetTimer(hwnd, 1, 100, None);
    }
}

fn now(d: &Dialog) -> f64 {
    d.t0.elapsed().as_secs_f64()
}

fn paint(d: &Dialog, dc: HDC) {
    let mut ops: Vec<Op> = vec![];
    let n = now(d);
    for (i, e) in d.entries.iter().enumerate() {
        e.draw(&mut ops, &d.t, n, d.active && d.focus == i);
    }
    for b in &d.presets {
        b.draw(&mut ops, &d.t.paper, &d.t);
    }
    d.start.draw(&mut ops, &d.t.paper, &d.t);
    unsafe {
        let (w, h) = d.size;
        let mdc = CreateCompatibleDC(dc);
        let bmp = CreateCompatibleBitmap(dc, w, h);
        let ob = SelectObject(mdc, bmp as _);
        gdi::draw(mdc, &d.statics);
        gdi::draw(mdc, &ops);
        BitBlt(dc, 0, 0, w, h, mdc, 0, 0, SRCCOPY);
        SelectObject(mdc, ob);
        DeleteObject(bmp as _);
        DeleteDC(mdc);
    }
}

pub(crate) fn clipboard_get() -> Option<Vec<u16>> {
    unsafe {
        if OpenClipboard(std::ptr::null_mut()) == 0 {
            return None;
        }
        let h = GetClipboardData(13); // CF_UNICODETEXT
        let mut out = None;
        if !h.is_null() {
            let p = GlobalLock(h as _) as *const u16;
            if !p.is_null() {
                let mut n = 0;
                while *p.add(n) != 0 {
                    n += 1;
                }
                out = Some(std::slice::from_raw_parts(p, n).to_vec());
                GlobalUnlock(h as _);
            }
        }
        CloseClipboard();
        out
    }
}

pub(crate) fn clipboard_set(s: &[u16]) {
    unsafe {
        if OpenClipboard(std::ptr::null_mut()) == 0 {
            return;
        }
        EmptyClipboard();
        let h = GlobalAlloc(GMEM_MOVEABLE, (s.len() + 1) * 2);
        if !h.is_null() {
            let p = GlobalLock(h) as *mut u16;
            std::ptr::copy_nonoverlapping(s.as_ptr(), p, s.len());
            *p.add(s.len()) = 0;
            GlobalUnlock(h);
            SetClipboardData(13, h as _);
        }
        CloseClipboard();
    }
}

fn close(hwnd: HWND) {
    let old = DLG.with(|d| d.try_borrow_mut().ok().and_then(|mut g| g.take()));
    if let Some(d) = old {
        unsafe {
            for f in d.fonts {
                DeleteObject(f as _);
            }
            DestroyIcon(d.icon);
        }
    }
    unsafe {
        DestroyWindow(hwnd);
    }
}

/// go(): minutes = max(1, min(240, int(float(m)))) (25 if it isn't a number), task stripped or "Focus session".
fn go(d: &Dialog) -> Option<Out> {
    let raw_task = d.entries[0].value();
    let m = d.entries[1].value();
    let minutes = match m.trim().parse::<f64>() {
        Ok(v) if v.is_finite() => (v.trunc() as i64).clamp(1, 240),
        Ok(v) if v.is_nan() => 25,
        Ok(_) => return None, // int(inf) raises in Python: nothing happens
        Err(_) => 25,
    };
    let t = raw_task.trim().to_string();
    let name = if t.is_empty() { "Focus session".to_string() } else { t.clone() };
    Some(Out::Go(name, t, minutes))
}

fn push(o: Out) {
    OUT.with(|q| q.borrow_mut().push(o));
}

unsafe extern "system" fn wndproc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    unsafe {
        let pt = || ((lp & 0xffff) as i16 as i32, ((lp >> 16) & 0xffff) as i16 as i32);
        let with = |f: &mut dyn FnMut(&mut Dialog)| {
            DLG.with(|d| {
                if let Ok(mut g) = d.try_borrow_mut() {
                    if let Some(dl) = g.as_mut() {
                        if dl.hwnd == hwnd {
                            f(dl);
                        }
                    }
                }
            })
        };
        match msg {
            WM_PAINT => {
                let mut ps: PAINTSTRUCT = std::mem::zeroed();
                let dc = BeginPaint(hwnd, &mut ps);
                DLG.with(|d| {
                    if let Ok(g) = d.try_borrow() {
                        if let Some(dl) = g.as_ref() {
                            paint(dl, dc);
                        }
                    }
                });
                EndPaint(hwnd, &ps);
                0
            }
            WM_ERASEBKGND => 1,
            WM_TIMER => {
                InvalidateRect(hwnd, std::ptr::null(), 0);
                0
            }
            WM_ACTIVATE => {
                let on = (wp & 0xffff) != 0;
                with(&mut |d| d.active = on);
                InvalidateRect(hwnd, std::ptr::null(), 0);
                0
            }
            WM_MOUSEMOVE => {
                let (x, y) = pt();
                let mut tme = TRACKMOUSEEVENT { cbSize: std::mem::size_of::<TRACKMOUSEEVENT>() as u32,
                                                dwFlags: TME_LEAVE, hwndTrack: hwnd, dwHoverTime: 0 };
                TrackMouseEvent(&mut tme);
                let mut over_entry = false;
                with(&mut |d| {
                    for (i, b) in d.presets.iter_mut().chain(std::iter::once(&mut d.start)).enumerate() {
                        b.hover = b.hit(x, y);
                        b.down = b.hover && d.pressed == Some(i);
                    }
                    over_entry = d.entries.iter().any(|e| {
                        let (ex, ey, ew, eh) = e.rect;
                        ex <= x && x < ex + ew && ey <= y && y < ey + eh
                    });
                    if d.dragging {
                        let n = now(d);
                        let e = &mut d.entries[d.focus];
                        let i = e.index_at(x);
                        e.pos = i;
                        let (a, b) = (e.anchor.min(i), e.anchor.max(i));
                        e.sel = if a == b { None } else { Some((a, b)) };
                        e.blink_t0 = n;
                    }
                });
                SetCursor(LoadCursorW(std::ptr::null_mut(), if over_entry { IDC_IBEAM } else { IDC_HAND_OR_ARROW(x, y) }));
                InvalidateRect(hwnd, std::ptr::null(), 0);
                0
            }
            0x02A3 => {
                // WM_MOUSELEAVE: <Leave> = hover off, down off
                with(&mut |d| {
                    for b in d.presets.iter_mut().chain(std::iter::once(&mut d.start)) {
                        b.hover = false;
                        b.down = false;
                    }
                });
                InvalidateRect(hwnd, std::ptr::null(), 0);
                0
            }
            WM_SETCURSOR => {
                if (lp & 0xffff) as u32 == HTCLIENT {
                    return 1; // set in WM_MOUSEMOVE
                }
                DefWindowProcW(hwnd, msg, wp, lp)
            }
            WM_LBUTTONDOWN | WM_LBUTTONDBLCLK => {
                let (x, y) = pt();
                SetCapture(hwnd);
                let dbl = msg == WM_LBUTTONDBLCLK;
                with(&mut |d| {
                    let n = now(d);
                    for (i, b) in d.presets.iter_mut().chain(std::iter::once(&mut d.start)).enumerate() {
                        if b.hit(x, y) {
                            b.down = true;
                            d.pressed = Some(i);
                        }
                    }
                    for i in 0..2 {
                        let (ex, ey, ew, eh) = d.entries[i].rect;
                        if ex <= x && x < ex + ew && ey <= y && y < ey + eh {
                            d.focus = i;
                            let e = &mut d.entries[i];
                            let k = e.index_at(x);
                            if dbl {
                                // double-click selects the word
                                let (mut a, mut b) = (k, k);
                                let sp = |c: u16| c == ' ' as u16;
                                while a > 0 && !sp(e.text[a - 1]) {
                                    a -= 1;
                                }
                                while b < e.text.len() && !sp(e.text[b]) {
                                    b += 1;
                                }
                                e.sel = if a < b { Some((a, b)) } else { None };
                                e.anchor = a;
                                e.pos = b;
                            } else {
                                e.pos = k;
                                e.anchor = k;
                                e.sel = None;
                                d.dragging = true;
                            }
                            e.blink_t0 = n;
                        }
                    }
                });
                InvalidateRect(hwnd, std::ptr::null(), 0);
                0
            }
            WM_LBUTTONUP => {
                ReleaseCapture();
                let (x, y) = pt();
                let mut act: Option<usize> = None;
                with(&mut |d| {
                    d.dragging = false;
                    if let Some(i) = d.pressed.take() {
                        let all: Vec<&mut PixelButton> =
                            d.presets.iter_mut().chain(std::iter::once(&mut d.start)).collect();
                        let was = all[i].down;
                        for b in all {
                            b.down = false;
                        }
                        let b = if i < 4 { &d.presets[i] } else { &d.start };
                        if was && b.hit(x, y) {
                            act = Some(i);
                        }
                    }
                });
                if let Some(i) = act {
                    push(Out::Sound("tap"));
                    if i < 4 {
                        let v = ["15", "25", "45", "60"][i];
                        with(&mut |d| {
                            let e = &mut d.entries[1];
                            e.text = v.encode_utf16().collect();
                            e.pos = e.pos.min(e.text.len());
                            e.sel = None;
                        });
                    } else {
                        submit(hwnd);
                        return 0;
                    }
                }
                InvalidateRect(hwnd, std::ptr::null(), 0);
                0
            }
            WM_KEYDOWN => {
                let vk = wp as u16;
                let shift = GetKeyState(VK_SHIFT as i32) < 0;
                let ctrl = GetKeyState(VK_CONTROL as i32) < 0;
                if vk == VK_RETURN {
                    submit(hwnd);
                    return 0;
                }
                if vk == VK_ESCAPE {
                    close(hwnd);
                    return 0;
                }
                with(&mut |d| {
                    let n = now(d);
                    if vk == VK_TAB {
                        // tk_focusNext / tk_focusPrev: the other entry, with all its text selected
                        d.focus = if shift { (d.focus + 1) % 2 } else { (d.focus + 1) % 2 };
                        d.entries[d.focus].select_all();
                    } else if ctrl && (vk == b'A' as u16 || vk == 0xBF) {
                        d.entries[d.focus].select_all();
                    } else if ctrl && (vk == b'C' as u16 || vk == b'X' as u16) {
                        if let Some(s) = d.entries[d.focus].selected() {
                            clipboard_set(&s);
                            if vk == b'X' as u16 {
                                d.entries[d.focus].insert(&[]);
                            }
                        }
                    } else if ctrl && vk == b'V' as u16 {
                        if let Some(s) = clipboard_get() {
                            let s: Vec<u16> = s.into_iter().filter(|&c| c != 13 && c != 10).collect();
                            d.entries[d.focus].insert(&s);
                        }
                    } else {
                        d.entries[d.focus].key(vk, shift, ctrl);
                    }
                    d.entries[d.focus].blink_t0 = n;
                });
                InvalidateRect(hwnd, std::ptr::null(), 0);
                0
            }
            WM_CHAR => {
                let c = wp as u16;
                if c >= 0x20 && c != 0x7f && GetKeyState(VK_CONTROL as i32) >= 0 {
                    with(&mut |d| {
                        let n = now(d);
                        d.entries[d.focus].insert(&[c]);
                        d.entries[d.focus].blink_t0 = n;
                    });
                    InvalidateRect(hwnd, std::ptr::null(), 0);
                }
                0
            }
            WM_CLOSE => {
                close(hwnd);
                0
            }
            _ => DefWindowProcW(hwnd, msg, wp, lp),
        }
    }
}

#[allow(non_snake_case)]
fn IDC_HAND_OR_ARROW(x: i32, y: i32) -> windows_sys::core::PCWSTR {
    // PixelButton: cursor="hand2" over a button
    let over = DLG.with(|d| {
        d.try_borrow()
            .ok()
            .and_then(|g| g.as_ref().map(|dl| dl.presets.iter().chain(std::iter::once(&dl.start)).any(|b| b.hit(x, y))))
            .unwrap_or(false)
    });
    if over { IDC_HAND } else { IDC_ARROW }
}

fn submit(hwnd: HWND) {
    let r = DLG.with(|d| d.try_borrow().ok().and_then(|g| g.as_ref().and_then(go)));
    if let Some(o) = r {
        push(o);
        close(hwnd);
    }
}
