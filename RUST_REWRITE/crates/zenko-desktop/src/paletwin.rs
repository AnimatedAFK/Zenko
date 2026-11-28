//! The window of zenko20's Ctrl+K palette (zenko.py ZkPalette as a Toplevel: overrideredirect, -topmost, over the
//! main window): typing refreshes the rows, ↑ ↓ Tab move, Enter or a click opens, Esc closes, and it goes away when
//! another app takes the focus (checked twice, focus can blink) but stays when the main window does. What was
//! picked waits in PICKED for the main window's next tick.

use crate::gdi;
use crate::uiwin::{GdiFonts, to_ops};
use std::cell::RefCell;
use windows_sys::Win32::Foundation::*;
use windows_sys::Win32::Graphics::Gdi::*;
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::*;
use windows_sys::Win32::UI::WindowsAndMessaging::*;
use zenko_desktop::palette::Palette;
use zenko_desktop::theme::Theme;
use zenko_desktop::ui::Cursor;

struct PalW {
    hwnd: HWND,
    owner: HWND,
    p: Palette,
    fonts: GdiFonts,
    t: Theme,
    t0: std::time::Instant,
    had_focus: bool,
    /// _maybe_close's second look (after 300 ms) is due
    again: bool,
}

thread_local! {
    static PAL: RefCell<Option<PalW>> = const { RefCell::new(None) };
    /// the picked item's `run`
    pub static PICKED: RefCell<Vec<String>> = const { RefCell::new(vec![]) };
}

fn with<R>(f: impl FnOnce(&mut PalW) -> R) -> Option<R> {
    PAL.with(|h| h.try_borrow_mut().ok().and_then(|mut v| v.as_mut().map(f)))
}

pub fn close() {
    let w = PAL.with(|p| p.try_borrow_mut().ok().and_then(|mut v| v.take()));
    if let Some(w) = w {
        unsafe { DestroyWindow(w.hwnd) };
    }
}

/// open the box over the main window `owner` (an old one closes first); `hub` is the main window's screen rect
pub fn open(mut p: Palette, fonts: GdiFonts, t: Theme, owner: HWND, hub: RECT) {
    close();
    p.ui.layout(&fonts);
    let (w, h) = (p.ui.ws[0].w, p.ui.ws[0].h);
    let (x, y) = p.place(hub.left, hub.top, hub.right - hub.left, hub.bottom - hub.top);
    unsafe {
        let hinst = GetModuleHandleW(std::ptr::null());
        let cls = gdi::wide("ZenkoRustPalette");
        let wc = WNDCLASSEXW {
            cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
            style: 0,
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
        let hwnd = CreateWindowExW(WS_EX_TOPMOST | WS_EX_TOOLWINDOW, cls.as_ptr(), cls.as_ptr(), WS_POPUP, x, y, w, h,
                                   owner, std::ptr::null_mut(), hinst, std::ptr::null());
        PAL.with(|v| *v.borrow_mut() = Some(PalW { hwnd, owner, p, fonts, t, t0: std::time::Instant::now(),
                                                   had_focus: false, again: false }));
        ShowWindow(hwnd, SW_SHOW);
        SetForegroundWindow(hwnd);
        SetFocus(hwnd);
        SetTimer(hwnd, 1, 100, None);
    }
}

/// after typing: new rows, and the box as tall as they need
fn resync(x: &mut PalW) {
    if x.p.sync(&x.fonts) {
        x.p.ui.layout(&x.fonts);
        let (w, h) = (x.p.ui.ws[0].w, x.p.ui.ws[0].h);
        unsafe { SetWindowPos(x.hwnd, std::ptr::null_mut(), 0, 0, w, h, SWP_NOMOVE | SWP_NOZORDER | SWP_NOACTIVATE) };
    }
}

/// run(i): close, the nav sound, then what the row does (on the main window's next tick)
fn run(i: usize) {
    let Some(cmd) = with(|x| x.p.pick(i)).flatten() else { return };
    close();
    PICKED.with(|q| q.borrow_mut().push(cmd));
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
                with(|x| {
                    let n = x.t0.elapsed().as_secs_f64();
                    let d = x.p.ui.draw(&x.fonts, &x.t, n);
                    let ops = to_ops(&x.fonts, &d);
                    let (w, h) = (x.p.ui.ws[0].w, x.p.ui.ws[0].h);
                    let mdc = CreateCompatibleDC(dc);
                    let bmp = CreateCompatibleBitmap(dc, w, h);
                    let ob = SelectObject(mdc, bmp as _);
                    gdi::draw(mdc, &ops);
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
                if wp == 2 {
                    // _maybe_close: another app has the focus (looked at twice), or the main window took it back
                    KillTimer(hwnd, 2);
                    let fg = GetForegroundWindow();
                    let act = with(|x| {
                        if !x.had_focus || fg == x.hwnd {
                            return 0;
                        }
                        if fg == x.owner {
                            return 1;
                        }
                        if fg.is_null() && !x.again {
                            x.again = true;
                            return 2;
                        }
                        3
                    }).unwrap_or(0);
                    match act {
                        1 => {
                            SetForegroundWindow(hwnd);
                            SetFocus(hwnd);
                        }
                        2 => {
                            SetTimer(hwnd, 2, 300, None);
                        }
                        3 => close(),
                        _ => {}
                    }
                    return 0;
                }
                redraw();
                0
            }
            WM_ACTIVATE => {
                let on = (wp & 0xffff) != 0;
                with(|x| {
                    x.p.ui.active = on;
                    if on {
                        x.had_focus = true;
                        x.again = false;
                    }
                });
                if !on {
                    SetTimer(hwnd, 2, 150, None);
                }
                redraw();
                0
            }
            WM_MOUSEACTIVATE => MA_ACTIVATE as isize,
            WM_MOUSEMOVE => {
                let (mx, my) = pt();
                let cur = with(|x| {
                    let n = x.t0.elapsed().as_secs_f64();
                    x.p.ui.motion(&x.fonts, mx, my, n);
                    if let Some(i) = x.p.row_at(mx, my) {
                        if i != x.p.sel {
                            x.p.select(i);
                        }
                        return Cursor::Hand;
                    }
                    x.p.ui.cursor_at(mx, my)
                }).unwrap_or(Cursor::Arrow);
                let c = match cur {
                    Cursor::Hand => IDC_HAND,
                    Cursor::IBeam => IDC_IBEAM,
                    Cursor::Arrow => IDC_ARROW,
                };
                SetCursor(LoadCursorW(std::ptr::null_mut(), c));
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
                let row = with(|x| x.p.row_at(mx, my)).flatten();
                if let Some(i) = row {
                    run(i);
                    return 0;
                }
                with(|x| {
                    let n = x.t0.elapsed().as_secs_f64();
                    x.p.ui.press(&x.fonts, mx, my, n, msg == WM_LBUTTONDBLCLK);
                    x.p.ui.focus = Some(x.p.entry);
                });
                redraw();
                0
            }
            WM_LBUTTONUP => {
                let (mx, my) = pt();
                with(|x| x.p.ui.release(mx, my));
                0
            }
            WM_KEYDOWN => {
                let vk = wp as u16;
                let shift = GetKeyState(VK_SHIFT as i32) < 0;
                let ctrl = GetKeyState(VK_CONTROL as i32) < 0;
                if vk == VK_ESCAPE {
                    close();
                    return 0;
                }
                if vk == VK_RETURN {
                    if let Some(i) = with(|x| x.p.sel) {
                        run(i);
                    }
                    return 0;
                }
                with(|x| {
                    let n = x.t0.elapsed().as_secs_f64();
                    if vk == VK_DOWN || vk == VK_TAB {
                        x.p.move_by(1);
                    } else if vk == VK_UP {
                        x.p.move_by(-1);
                    } else if ctrl && vk == b'A' as u16 {
                        x.p.ui.select_all(&x.fonts);
                    } else if ctrl && (vk == b'C' as u16 || vk == b'X' as u16) {
                        if let Some(s) = x.p.ui.selected() {
                            crate::dialog::clipboard_set(&s);
                            if vk == b'X' as u16 {
                                x.p.ui.insert(&x.fonts, &[], n);
                            }
                        }
                    } else if ctrl && vk == b'V' as u16 {
                        if let Some(s) = crate::dialog::clipboard_get() {
                            let s: Vec<u16> = s.into_iter().filter(|&c| c != 13 && c != 10).collect();
                            x.p.ui.insert(&x.fonts, &s, n);
                        }
                    } else {
                        x.p.ui.key(&x.fonts, vk, shift, ctrl, n);
                    }
                    resync(x);
                });
                redraw();
                0
            }
            WM_CHAR => {
                let c = wp as u16;
                if c >= 0x20 && c != 0x7f && GetKeyState(VK_CONTROL as i32) >= 0 {
                    with(|x| {
                        let n = x.t0.elapsed().as_secs_f64();
                        x.p.ui.insert(&x.fonts, &[c], n);
                        resync(x);
                    });
                    redraw();
                }
                0
            }
            _ => DefWindowProcW(hwnd, msg, wp, lp),
        }
    }
}
