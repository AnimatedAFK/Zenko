//! ZenkoRust.exe: Zenko on the desktop, drawn by the Rust port.
//! A see-through, always-on-top window exactly like zenko.py's (same size, same place, same pictures).
//! This preview only READS Zenko's data in %APPDATA%\FoxBuddy; it keeps its own position in rust_preview.json.
#![cfg_attr(windows, windows_subsystem = "windows")]

#[cfg(not(windows))]
fn main() {
    eprintln!("ZenkoRust is a Windows app. On other systems run: cargo run --example preview");
}

#[cfg(windows)]
fn main() {
    win::run();
}

#[cfg(windows)]
mod win {
    use std::cell::RefCell;
    use std::time::Instant;
    use windows_sys::Win32::Foundation::*;
    use windows_sys::Win32::Graphics::Gdi::*;
    use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
    use windows_sys::Win32::System::SystemInformation::GetTickCount;
    use windows_sys::Win32::UI::HiDpi::*;
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::*;
    use windows_sys::Win32::UI::WindowsAndMessaging::*;
    use zenko_desktop::{Cfg, Companion, Frame, Session, data_dir};

    struct App {
        fox: Companion,
        t0: Instant,
        x: i32,
        y: i32,
        drag: Option<(i32, i32, i32, i32, bool)>,
        dib: Option<(HDC, HBITMAP, *mut u8, i32, i32)>,
    }

    thread_local! {
        static APP: RefCell<Option<App>> = const { RefCell::new(None) };
    }

    fn wide(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(std::iter::once(0)).collect()
    }

    fn now(app: &App) -> f64 {
        1000.0 + app.t0.elapsed().as_secs_f64()
    }

    fn idle_seconds() -> f64 {
        unsafe {
            let mut li = LASTINPUTINFO {
                cbSize: std::mem::size_of::<LASTINPUTINFO>() as u32,
                dwTime: 0,
            };
            if GetLastInputInfo(&mut li) != 0 {
                return GetTickCount().wrapping_sub(li.dwTime) as f64 / 1000.0;
            }
        }
        0.0
    }

    fn pos_path() -> std::path::PathBuf {
        data_dir().join("rust_preview.json")
    }

    fn save_pos(x: i32, y: i32) {
        let _ = std::fs::create_dir_all(data_dir());
        let _ = std::fs::write(pos_path(), format!("{{\"x\": {x}, \"y\": {y}}}"));
    }

    fn start_pos(fox: &Companion) -> (i32, i32) {
        unsafe {
            let sw = GetSystemMetrics(SM_CXSCREEN);
            let sh = GetSystemMetrics(SM_CYSCREEN);
            let own: serde_json::Value = std::fs::read_to_string(pos_path())
                .ok()
                .and_then(|s| serde_json::from_str(&s).ok())
                .unwrap_or_default();
            for v in [&own, &fox.cfg.raw] {
                if let (Some(x), Some(y)) = (
                    v.get("x").and_then(|x| x.as_f64()),
                    v.get("y").and_then(|y| y.as_f64()),
                ) {
                    let (x, y) = (x as i32, y as i32);
                    if -fox.w < x && x < sw && 0 <= y && y < sh {
                        return (x, y);
                    }
                }
            }
            let mut r = RECT {
                left: 0,
                top: 0,
                right: sw,
                bottom: sh,
            };
            SystemParametersInfoW(SPI_GETWORKAREA, 0, &mut r as *mut RECT as *mut _, 0);
            (
                r.right - fox.w - (6.0 * fox.u) as i32,
                r.bottom - fox.h - (2.0 * fox.u) as i32,
            )
        }
    }

    unsafe fn present(hwnd: HWND, app: &mut App, f: &Frame) {
        unsafe {
            let (w, h) = (f.w as i32, f.h as i32);
            let screen = GetDC(std::ptr::null_mut());
            if app.dib.map(|d| (d.3, d.4)) != Some((w, h)) {
                if let Some((dc, bmp, _, _, _)) = app.dib.take() {
                    DeleteObject(bmp as _);
                    DeleteDC(dc);
                }
                let dc = CreateCompatibleDC(screen);
                let mut bi: BITMAPINFO = std::mem::zeroed();
                bi.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
                bi.bmiHeader.biWidth = w;
                bi.bmiHeader.biHeight = -h;
                bi.bmiHeader.biPlanes = 1;
                bi.bmiHeader.biBitCount = 32;
                bi.bmiHeader.biCompression = BI_RGB;
                let mut bits: *mut core::ffi::c_void = std::ptr::null_mut();
                let bmp =
                    CreateDIBSection(dc, &bi, DIB_RGB_COLORS, &mut bits, std::ptr::null_mut(), 0);
                SelectObject(dc, bmp as _);
                app.dib = Some((dc, bmp, bits as *mut u8, w, h));
            }
            let (dc, _, bits, _, _) = app.dib.unwrap();
            let buf = std::slice::from_raw_parts_mut(bits, (w * h * 4) as usize);
            for (i, p) in f.px.iter().enumerate() {
                // premultiplied BGRA; sprites are fully opaque or fully clear, like Tk's colour key
                let a = p[3] as u32;
                buf[i * 4] = (p[2] as u32 * a / 255) as u8;
                buf[i * 4 + 1] = (p[1] as u32 * a / 255) as u8;
                buf[i * 4 + 2] = (p[0] as u32 * a / 255) as u8;
                buf[i * 4 + 3] = p[3];
            }
            let pt = POINT { x: app.x, y: app.y };
            let sz = SIZE { cx: w, cy: h };
            let src = POINT { x: 0, y: 0 };
            let blend = BLENDFUNCTION {
                BlendOp: AC_SRC_OVER as u8,
                BlendFlags: 0,
                SourceConstantAlpha: 255,
                AlphaFormat: AC_SRC_ALPHA as u8,
            };
            UpdateLayeredWindow(hwnd, screen, &pt, &sz, dc, &src, 0, &blend, ULW_ALPHA);
            ReleaseDC(std::ptr::null_mut(), screen);
        }
    }

    unsafe fn tick(hwnd: HWND) {
        APP.with(|a| {
            let mut g = a.borrow_mut();
            let Some(app) = g.as_mut() else { return };
            let t = now(app);
            // zenko.py tick(): away for idle_minutes -> she falls asleep; any input wakes her
            let idle = idle_seconds();
            let away = idle >= app.fox.cfg.f("idle_minutes", 4.0) * 60.0;
            if app.fox.sleeping && !away && idle < 3.0 {
                app.fox.sleeping = false;
            }
            if away && !app.fox.sleeping {
                app.fox.sleeping = true;
            }
            if let Some(s) = &app.fox.session {
                if t - s.started >= s.planned {
                    app.fox.session = None;
                    app.fox.mood_override = Some(("happy".into(), t + 6.0));
                }
            }
            let mut p = POINT { x: 0, y: 0 };
            unsafe { GetCursorPos(&mut p) };
            let f = app.fox.frame(t, (p.x, p.y), (app.x, app.y));
            unsafe { present(hwnd, app, &f) };
        });
    }

    fn toggle_focus() {
        APP.with(|a| {
            if let Some(app) = a.borrow_mut().as_mut() {
                let t = now(app);
                if app.fox.session.is_some() {
                    app.fox.session = None;
                } else {
                    app.fox.session = Some(Session {
                        planned: 25.0 * 60.0,
                        started: t,
                    });
                }
            }
        });
    }

    unsafe extern "system" fn wndproc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
        unsafe {
            match msg {
                WM_TIMER => {
                    tick(hwnd);
                    0
                }
                WM_LBUTTONDOWN => {
                    let mut p = POINT { x: 0, y: 0 };
                    GetCursorPos(&mut p);
                    APP.with(|a| {
                        if let Some(app) = a.borrow_mut().as_mut() {
                            app.drag = Some((p.x - app.x, p.y - app.y, p.x, p.y, false));
                        }
                    });
                    SetCapture(hwnd);
                    0
                }
                WM_MOUSEMOVE => {
                    let mut p = POINT { x: 0, y: 0 };
                    GetCursorPos(&mut p);
                    APP.with(|a| {
                        if let Some(app) = a.borrow_mut().as_mut() {
                            if let Some((dx, dy, sx, sy, moved)) = app.drag {
                                if moved || (p.x - sx).abs() + (p.y - sy).abs() > 4 {
                                    app.x = p.x - dx;
                                    app.y = p.y - dy;
                                    app.drag = Some((dx, dy, sx, sy, true));
                                    SetWindowPos(
                                        hwnd,
                                        std::ptr::null_mut(),
                                        app.x,
                                        app.y,
                                        0,
                                        0,
                                        SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE,
                                    );
                                }
                            }
                        }
                    });
                    0
                }
                WM_LBUTTONUP => {
                    ReleaseCapture();
                    APP.with(|a| {
                        if let Some(app) = a.borrow_mut().as_mut() {
                            if let Some((_, _, _, _, true)) = app.drag {
                                save_pos(app.x, app.y);
                            }
                            app.drag = None;
                        }
                    });
                    0
                }
                WM_LBUTTONDBLCLK => {
                    toggle_focus();
                    0
                }
                WM_RBUTTONUP => {
                    let menu = CreatePopupMenu();
                    let focusing = APP.with(|a| {
                        a.borrow()
                            .as_ref()
                            .map(|x| x.fox.session.is_some())
                            .unwrap_or(false)
                    });
                    let l1 = wide(if focusing {
                        "Stop focus session"
                    } else {
                        "Start a 25-minute focus session"
                    });
                    let l2 = wide("Quit Zenko (Rust preview)");
                    AppendMenuW(menu, MF_STRING, 1, l1.as_ptr());
                    AppendMenuW(menu, MF_SEPARATOR, 0, std::ptr::null());
                    AppendMenuW(menu, MF_STRING, 2, l2.as_ptr());
                    let mut p = POINT { x: 0, y: 0 };
                    GetCursorPos(&mut p);
                    SetForegroundWindow(hwnd);
                    let cmd = TrackPopupMenu(
                        menu,
                        TPM_RETURNCMD | TPM_RIGHTBUTTON,
                        p.x,
                        p.y,
                        0,
                        hwnd,
                        std::ptr::null(),
                    );
                    DestroyMenu(menu);
                    if cmd == 1 {
                        toggle_focus();
                    } else if cmd == 2 {
                        DestroyWindow(hwnd);
                    }
                    0
                }
                WM_DESTROY => {
                    PostQuitMessage(0);
                    0
                }
                _ => DefWindowProcW(hwnd, msg, wp, lp),
            }
        }
    }

    pub fn run() {
        unsafe {
            if SetProcessDpiAwareness(PROCESS_SYSTEM_DPI_AWARE) != 0 {
                SetProcessDPIAware();
            }
            let u = (GetDpiForSystem() as f64 / 96.0).max(1.0);
            let t0 = Instant::now();
            let fox = Companion::new(Cfg::load(), u, 1000.0);
            let (x, y) = start_pos(&fox);
            let (w, h) = (fox.w, fox.h);
            let hinst = GetModuleHandleW(std::ptr::null());
            let cls = wide("ZenkoRustFox");
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
            let title = wide("Zenko");
            let hwnd = CreateWindowExW(
                WS_EX_LAYERED | WS_EX_TOPMOST | WS_EX_TOOLWINDOW,
                cls.as_ptr(),
                title.as_ptr(),
                WS_POPUP,
                x,
                y,
                w,
                h,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                hinst,
                std::ptr::null(),
            );
            APP.with(|a| {
                *a.borrow_mut() = Some(App {
                    fox,
                    t0,
                    x,
                    y,
                    drag: None,
                    dib: None,
                })
            });
            tick(hwnd);
            ShowWindow(hwnd, SW_SHOWNOACTIVATE);
            SetTimer(hwnd, 1, 60, None);
            let mut msg: MSG = std::mem::zeroed();
            while GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0) > 0 {
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        }
    }
}
