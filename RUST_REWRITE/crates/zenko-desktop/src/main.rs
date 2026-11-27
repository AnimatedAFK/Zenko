//! ZenkoRust.exe: Zenko on the desktop, drawn by the Rust port.
//! A see-through, always-on-top window exactly like zenko.py's (same size, same place, same pictures), with her
//! speech bubble, the right-click menu, play reactions, focus sessions and her sounds. It uses the same files in
//! %APPDATA%\FoxBuddy as zenko.py (config.json, stats.json, zenko_extras.json, zenko_life.json, sounds\).
#![cfg_attr(windows, windows_subsystem = "windows")]

#[cfg(not(windows))]
fn main() {
    eprintln!("ZenkoRust is a Windows app. On other systems run: cargo run --example preview");
}

#[cfg(windows)]
mod bubble;
#[cfg(windows)]
mod dialog;
#[cfg(windows)]
mod eat;
#[cfg(windows)]
mod panel;
#[cfg(windows)]
mod tkw;
#[cfg(windows)]
mod gdi;
#[cfg(windows)]
mod menu;
#[cfg(windows)]
mod speak;
#[cfg(windows)]
mod bgm;
#[cfg(windows)]
mod net;
#[cfg(windows)]
mod uiwin;
#[cfg(windows)]
mod tkpaint;
#[cfg(windows)]
mod hubw;

#[cfg(windows)]
fn main() {
    win::run();
}

#[cfg(windows)]
pub fn with_bubble(f: impl FnOnce(&bubble::Bubble)) {
    win::APP.with(|a| {
        if let Ok(g) = a.try_borrow() {
            if let Some(app) = g.as_ref() {
                if let Some(b) = &app.bubble {
                    f(b);
                }
            }
        }
    });
}

#[cfg(windows)]
pub fn bubble_click(x: i32, y: i32) {
    win::bubble_click(x, y);
}

#[cfg(windows)]
mod win {
    use crate::bubble::Bubble;
    use crate::menu::{self, Item, check, cmd, info, later, radio, sub};
    use std::cell::RefCell;
    use std::time::Instant;
    use windows_sys::Win32::Foundation::*;
    use windows_sys::Win32::Graphics::Gdi::*;
    use windows_sys::Win32::Media::Audio::*;
    use windows_sys::Win32::UI::Controls::{DRAWITEMSTRUCT, MEASUREITEMSTRUCT};
    use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
    use windows_sys::Win32::System::SystemInformation::GetTickCount;
    use windows_sys::Win32::UI::HiDpi::*;
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::*;
    use windows_sys::Win32::UI::WindowsAndMessaging::*;
    use zenko_desktop::sound::{SoundCfg, Variety, beep_file, ui_file};
    use zenko_desktop::theme::theme;
    use zenko_desktop::{Cfg, Companion, Ev, Frame, update_config};

    pub struct App {
        pub fox: Companion,
        pub bubble: Option<Bubble>,
        variety: Variety,
        t0: Instant,
        x: i32,
        y: i32,
        drag: Option<(i32, i32, i32, i32, bool)>,
        dib: Option<(HDC, HBITMAP, *mut u8, i32, i32)>,
        dpi: f64,
        family: String,
        last_tick: f64,
        next_tick: f64,
        panel_fonts: Option<crate::panel::Fonts>,
        panel_dib: Option<(HDC, HBITMAP, *mut u8, i32, i32)>,
        fg: (String, String),
        next_lock: f64,
        /// player_poll (1.5 s), music_watch (4 s), life_slow_tick (30 s)
        next_player: f64,
        next_watch: f64,
        next_slow: f64,
        next_wx: f64,
        next_batt: f64,
        eating: Option<zenko_desktop::lockin::Eat>,
        next_top: f64,
        /// hub_tick / z19_hub_loop: the hub's sidebar news, once a second
        next_hub: f64,
        hwnd: HWND,
    }

    thread_local! {
        pub static APP: RefCell<Option<App>> = const { RefCell::new(None) };
    }

    fn wide(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(std::iter::once(0)).collect()
    }

    fn now(app: &App) -> f64 {
        1000.0 + app.t0.elapsed().as_secs_f64()
    }

    /// the fox and the app clock, for the windows that need her (None while the app is busy)
    pub fn with_fox<R>(f: impl FnOnce(&mut Companion, f64) -> R) -> Option<R> {
        APP.with(|a| {
            let Ok(mut g) = a.try_borrow_mut() else { return None };
            let app = g.as_mut()?;
            let t = now(app);
            Some(f(&mut app.fox, t))
        })
    }

    /// with_fox, and where her window is (for her poses)
    pub fn with_fox_at<R>(f: impl FnOnce(&mut Companion, f64, (i32, i32)) -> R) -> Option<R> {
        APP.with(|a| {
            let Ok(mut g) = a.try_borrow_mut() else { return None };
            let app = g.as_mut()?;
            let t = now(app);
            let (x, y) = (app.x, app.y);
            Some(f(&mut app.fox, t, (x, y)))
        })
    }

    fn idle_seconds() -> f64 {
        unsafe {
            let mut li = LASTINPUTINFO { cbSize: std::mem::size_of::<LASTINPUTINFO>() as u32, dwTime: 0 };
            if GetLastInputInfo(&mut li) != 0 {
                return GetTickCount().wrapping_sub(li.dwTime) as f64 / 1000.0;
            }
        }
        0.0
    }

    /// place_window: the saved spot if it is on screen, else the bottom-right corner of the work area.
    fn start_pos(fox: &Companion) -> (i32, i32) {
        unsafe {
            let sw = GetSystemMetrics(SM_CXSCREEN);
            let sh = GetSystemMetrics(SM_CYSCREEN);
            let v = &fox.cfg.raw;
            if let (Some(x), Some(y)) = (v.get("x").and_then(|x| x.as_f64()), v.get("y").and_then(|y| y.as_f64())) {
                let (x, y) = (x as i32, y as i32);
                if -fox.w < x && x < sw && 0 <= y && y < sh {
                    return (x, y);
                }
            }
            let mut r = RECT { left: 0, top: 0, right: sw, bottom: sh };
            SystemParametersInfoW(SPI_GETWORKAREA, 0, &mut r as *mut RECT as *mut _, 0);
            (r.right - fox.w - (6.0 * fox.u) as i32, r.bottom - fox.h - (2.0 * fox.u) as i32)
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
                let bmp = CreateDIBSection(dc, &bi, DIB_RGB_COLORS, &mut bits, std::ptr::null_mut(), 0);
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
            let (bdx, bdy, bcurl) = app.fox.batt_at;
            let batt = app.fox.battery_shapes(bdx, bdy, bcurl);
            if app.fox.session.is_some() || !batt.is_empty() {
                draw_panel(app, w, h, buf, &batt);
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

    /// draw_panel: the timer panel, drawn with GDI on a KEY-coloured bitmap and copied onto her frame (Tk draws it
    /// on the same see-through canvas as her).
    unsafe fn draw_panel(app: &mut App, w: i32, h: i32, buf: &mut [u8], batt: &[zenko_desktop::battery::Shape]) {
        unsafe {
            let screen = GetDC(std::ptr::null_mut());
            if app.panel_dib.map(|d| (d.3, d.4)) != Some((w, h)) {
                if let Some((dc, bmp, _, _, _)) = app.panel_dib.take() {
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
                let bmp = CreateDIBSection(dc, &bi, DIB_RGB_COLORS, &mut bits, std::ptr::null_mut(), 0);
                SelectObject(dc, bmp as _);
                app.panel_dib = Some((dc, bmp, bits as *mut u8, w, h));
            }
            ReleaseDC(std::ptr::null_mut(), screen);
            let (dc, _, bits, _, _) = app.panel_dib.unwrap();
            let pw = if batt.is_empty() { app.fox.colw.min(w) } else { w };
            let br = CreateSolidBrush(crate::gdi::KEY);
            let r = RECT { left: 0, top: 0, right: pw, bottom: h };
            FillRect(dc, &r, br);
            DeleteObject(br as _);
            let th = theme(&app.fox.cfg);
            let fam = app.family.clone();
            let fonts = crate::panel::Fonts::get(&mut app.panel_fonts, &fam, app.dpi);
            let mut ops = if app.fox.session.is_some() { crate::panel::ops(&app.fox, &th, fonts) } else { vec![] };
            use crate::gdi::{Op, rgb};
            use zenko_desktop::battery::Shape;
            for b in batt {
                ops.push(match b {
                    Shape::Rect { x1, y1, x2, y2, fill, outline } => Op::RectO { x1: *x1, y1: *y1, x2: *x2, y2: *y2,
                        fill: rgb(fill), outline: outline.as_ref().map(|(c, w)| (rgb(c), *w)) },
                    Shape::Poly { pts, fill, outline } => Op::Poly { pts: pts.clone(), fill: Some(rgb(fill)),
                        outline: Some(rgb(outline)), width: 1 },
                });
            }
            crate::gdi::draw(dc, &ops);
            GdiFlush();
            let src = std::slice::from_raw_parts(bits, (w * h * 4) as usize);
            for y in 0..h as usize {
                for x in 0..pw as usize {
                    let i = (y * w as usize + x) * 4;
                    // KEY (#ff00ff) is see-through, like Tk's -transparentcolor
                    if !(src[i] == 0xff && src[i + 1] == 0 && src[i + 2] == 0xff) {
                        buf[i] = src[i];
                        buf[i + 1] = src[i + 1];
                        buf[i + 2] = src[i + 2];
                        buf[i + 3] = 255;
                    }
                }
            }
        }
    }

    /// battery_status(): (percent, charging), or None on a desktop PC
    fn battery_status() -> Option<(i64, bool)> {
        use windows_sys::Win32::System::Power::{GetSystemPowerStatus, SYSTEM_POWER_STATUS};
        unsafe {
            let mut s: SYSTEM_POWER_STATUS = std::mem::zeroed();
            if GetSystemPowerStatus(&mut s) == 0 || s.BatteryFlag == 128 || s.BatteryLifePercent == 255 {
                return None;
            }
            Some((s.BatteryLifePercent as i64, s.ACLineStatus == 1))
        }
    }

    fn work_area() -> (i32, i32, i32, i32) {
        unsafe {
            let mut r = RECT { left: 0, top: 0, right: GetSystemMetrics(SM_CXSCREEN), bottom: GetSystemMetrics(SM_CYSCREEN) };
            SystemParametersInfoW(SPI_GETWORKAREA, 0, &mut r as *mut RECT as *mut _, 0);
            (r.left, r.top, r.right, r.bottom)
        }
    }

    /// _lockin_check with what's on screen; a pounce closes the tab now and runs the overlay.
    fn lock_poll(app: &mut App, t: f64) {
        use windows_sys::Win32::System::Threading::GetCurrentProcessId;
        let (hwnd, pid) = unsafe {
            let h = GetForegroundWindow();
            let mut pid = 0u32;
            if !h.is_null() {
                GetWindowThreadProcessId(h, &mut pid);
            }
            (h as isize, pid)
        };
        let (title, proc_) = foreground_info();
        let esc = unsafe { GetAsyncKeyState(VK_ESCAPE as i32) as u16 & 0x8000 != 0 };
        let me = unsafe { GetCurrentProcessId() };
        let Some(e) = app.fox.lockin_check(t, (hwnd, &title, &proc_, pid), esc, me) else { return };
        if e.tab {
            close_distraction("tab"); // the browser is still in front: close just that tab now
        }
        let rect = unsafe {
            let mut r = RECT { left: 0, top: 0, right: 0, bottom: 0 };
            if GetWindowRect(e.hwnd as HWND, &mut r) != 0 { (r.left, r.top, r.right, r.bottom) } else { (100, 100, 900, 600) }
        };
        if e.animate && !crate::eat::is_running() {
            let th = theme(&app.fox.cfg);
            let fam = crate::gdi::ui_font(&app.fox.cfg.s("app_font", ""));
            let fox = &mut app.fox;
            crate::eat::start(rect, &e.site, &th, &fam, app.dpi, |s, p, f, z| fox.eat_image(t, s, p, f, z));
            app.eating = Some(e);
        } else {
            eat_finish(&e);
            app.fox.lock_eating = false;
        }
    }

    /// finish(): minimise or close the window if it wasn't a tab, then bring back the window you were working in.
    fn eat_finish(e: &zenko_desktop::lockin::Eat) {
        unsafe {
            let h = e.hwnd as HWND;
            if !e.tab {
                if e.action == "close" {
                    PostMessageW(h, WM_CLOSE, 0, 0);
                } else {
                    ShowWindow(h, SW_MINIMIZE);
                }
            }
            if let Some(g) = e.good {
                if g != e.hwnd {
                    let g = g as HWND;
                    if IsWindow(g) != 0 {
                        if IsIconic(g) != 0 {
                            ShowWindow(g, SW_RESTORE);
                        }
                        SetForegroundWindow(g);
                    }
                }
            }
        }
    }

    /// foreground_info(): (window title, exe name in lower case) of the window you are using.
    fn foreground_info() -> (String, String) {
        use windows_sys::Win32::System::Threading::*;
        unsafe {
            let hwnd = GetForegroundWindow();
            if hwnd.is_null() {
                return (String::new(), String::new());
            }
            let n = GetWindowTextLengthW(hwnd);
            let mut buf = vec![0u16; n as usize + 1];
            let got = GetWindowTextW(hwnd, buf.as_mut_ptr(), n + 1);
            let title = String::from_utf16_lossy(&buf[..got.max(0) as usize]);
            let mut pid = 0u32;
            GetWindowThreadProcessId(hwnd, &mut pid);
            let mut proc_ = String::new();
            let h = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
            if !h.is_null() {
                let mut pb = vec![0u16; 520];
                let mut size = 520u32;
                if QueryFullProcessImageNameW(h, PROCESS_NAME_WIN32, pb.as_mut_ptr(), &mut size) != 0 {
                    let full = String::from_utf16_lossy(&pb[..size as usize]);
                    proc_ = full.rsplit(['\\', '/']).next().unwrap_or("").to_lowercase();
                }
                CloseHandle(h);
            }
            (title, proc_)
        }
    }

    /// close_distraction(kind): Ctrl+W for a browser tab, a polite WM_CLOSE for an app.
    fn close_distraction(kind: &str) -> bool {
        unsafe {
            let hwnd = GetForegroundWindow();
            if hwnd.is_null() {
                return false;
            }
            if kind == "tab" {
                for (vk, flag) in [(0x11u8, 0u32), (0x57, 0), (0x57, KEYEVENTF_KEYUP), (0x11, KEYEVENTF_KEYUP)] {
                    keybd_event(vk, 0, flag, 0);
                }
            } else {
                PostMessageW(hwnd, WM_CLOSE, 0, 0);
            }
            true
        }
    }

    /// winsound.PlaySound(file, SND_FILENAME | SND_ASYNC | SND_NODEFAULT), optionally after a short wait.
    fn play_file(p: std::path::PathBuf, delay: f64) {
        std::thread::spawn(move || {
            if delay > 0.0 {
                std::thread::sleep(std::time::Duration::from_secs_f64(delay));
            }
            let w: Vec<u16> = p.as_os_str().encode_wide_z();
            unsafe { PlaySoundW(w.as_ptr(), std::ptr::null_mut(), SND_FILENAME | SND_ASYNC | SND_NODEFAULT) };
        });
    }

    trait WideZ {
        fn encode_wide_z(&self) -> Vec<u16>;
    }
    impl WideZ for std::ffi::OsStr {
        fn encode_wide_z(&self) -> Vec<u16> {
            use std::os::windows::ffi::OsStrExt;
            self.encode_wide().chain(std::iter::once(0)).collect()
        }
    }

    fn handle_events(app: &mut App, t: f64) {
        let due: Vec<Ev> = {
            let (now_, later_): (Vec<_>, Vec<_>) = app.fox.delayed.drain(..).partition(|(at, _)| *at <= t);
            app.fox.delayed = later_;
            now_.into_iter().map(|(_, e)| e).collect()
        };
        app.fox.events.extend(due);
        let evs: Vec<Ev> = app.fox.events.drain(..).collect();
        let sc = SoundCfg::from(&app.fox.cfg);
        for e in evs {
            match e {
                Ev::Say { text, secs, choices } => {
                    if app.bubble.is_none() {
                        app.bubble = Some(Bubble::new());
                    }
                    let (dpi, fam, root) = (app.dpi, app.family.clone(), (app.x, app.y));
                    if let Some(b) = app.bubble.as_mut() {
                        b.say(&app.fox, root, t, &text, secs, &choices, dpi, &fam);
                    }
                }
                Ev::Beep { kind, variety } => {
                    let r = app.fox.rand();
                    if let Some(p) = beep_file(&kind, variety, &mut app.variety, t, &sc, r) {
                        play_file(p, 0.3); // give a matching speech bubble a moment to start
                    }
                }
                Ev::Ui(kind) => {
                    if let Some(p) = ui_file(&kind, &sc) {
                        play_file(p, 0.0);
                    }
                }
                Ev::Command(c) => run_command(app, &c),
                Ev::Bgm(Some((track, vol))) => crate::bgm::play(&track, vol),
                Ev::Bgm(None) => crate::bgm::stop(),
                Ev::Media(k) => crate::bgm::media_key(&k),
                Ev::Speak(text) => {
                    let c = &app.fox.cfg;
                    crate::speak::say(&text, &c.s("tts_voice", ""), c.f("tts_speed", 0.0) as i64, c.f("tts_pitch", 0.0) as i64);
                }
            }
        }
    }

    unsafe fn tick(hwnd: HWND) {
        APP.with(|a| {
            let Ok(mut g) = a.try_borrow_mut() else { return };
            let Some(app) = g.as_mut() else { return };
            let t = now(app);
            if t >= app.next_tick {
                // zenko.py tick(), once a second
                let dt = (t - app.last_tick).clamp(0.0, 5.0);
                app.last_tick = t;
                app.next_tick = t + 1.0;
                app.fg = foreground_info();
                let (title, proc_) = app.fg.clone();
                app.fox.tick_1s(t, dt, idle_seconds(), &title, &proc_);
                app.fox.study_tick(t);
                let (hwnd, own) = unsafe {
                    use windows_sys::Win32::System::Threading::GetCurrentProcessId;
                    let h = GetForegroundWindow();
                    let mut pid = 0u32;
                    if !h.is_null() {
                        GetWindowThreadProcessId(h, &mut pid);
                    }
                    (h as isize, pid == GetCurrentProcessId())
                };
                app.fox.screen_tick(t, hwnd, own);
                app.fox.study_timer(t);
                app.fox.guard_timers(t);
            }
            for o in crate::dialog::OUT.with(|q| std::mem::take(&mut *q.borrow_mut())) {
                match o {
                    crate::dialog::Out::Go(name, raw, minutes) => {
                        update_config(&mut app.fox.cfg, |v| {
                            v["last_task"] = raw.clone().into();
                            v["last_minutes"] = minutes.into();
                        });
                        app.fox.start_session(t, &name, minutes as f64);
                    }
                    crate::dialog::Out::Sound(k) => app.fox.events.push(Ev::Ui(k.into())),
                }
            }
            for o in crate::uiwin::OUT.with(|q| std::mem::take(&mut *q.borrow_mut())) {
                app.fox.events.push(Ev::Ui("tap".into())); // PixelButton._release: ui_sound("tap")
                if o.win == "checkin" {
                    if let Some(m) = o.cmd.strip_prefix("mood:") {
                        let note = o.values.iter().find(|v| v.0 == "note").map(|v| v.1.clone()).unwrap_or_default();
                        app.fox.checkin_save(t, m, &note);
                    }
                    crate::uiwin::close("checkin");
                }
            }
            for o in crate::hubw::OUT.with(|q| std::mem::take(&mut *q.borrow_mut())) {
                if let Some(k) = o.strip_prefix("sound:") {
                    app.fox.events.push(Ev::Ui(k.into()));
                } else if let Some(c) = o.strip_prefix("run:") {
                    hub_run(app, c);
                }
            }
            if t >= app.next_hub && crate::hubw::is_open() {
                app.next_hub = t + 1.0;
                crate::hubw::update(side_ctx(app), &hub_status(app));
            }
            if t >= app.next_top {
                app.next_top = t + 2.5; // keep_on_top
                unsafe {
                    SetWindowPos(hwnd, HWND_TOPMOST, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE);
                }
            }
            let mut p = POINT { x: 0, y: 0 };
            unsafe { GetCursorPos(&mut p) };
            app.fox.env = zenko_desktop::life::Env {
                work: work_area(),
                screen_w: unsafe { GetSystemMetrics(SM_CXSCREEN) },
                idle: idle_seconds(),
                dragging: app.drag.is_some(),
                bubble: app.bubble.as_ref().is_some_and(|b| b.shown),
            };
            if t >= app.next_lock && app.fox.lockin.is_some() {
                app.next_lock = t + 0.2; // lockin_poll
                lock_poll(app, t);
            }
            if crate::eat::take_finished() {
                if let Some(e) = app.eating.take() {
                    eat_finish(&e);
                }
            }
            if crate::eat::take_closed() {
                app.fox.lock_eating = false;
                app.eating = None;
            }
            if t >= app.next_player {
                app.next_player = t + 1.5;
                app.fox.player_poll(crate::bgm::spotify_window());
            }
            if t >= app.next_watch {
                app.next_watch = t + 4.0;
                if app.fox.music.bgm_state.as_ref().is_some_and(|s| s.0) {
                    crate::bgm::keep_going();
                }
            }
            if crate::bgm::FAILED.swap(false, std::sync::atomic::Ordering::Relaxed) {
                app.fox.say("Uu~ Windows wouldn't play the background music. (It still works for her other sounds.)", 8.0);
            }
            if t >= app.next_batt {
                app.next_batt = t + 30.0; // battery_tick
                app.fox.battery_tick(t, battery_status());
            }
            if t >= app.next_wx {
                app.next_wx = t + 45.0 * 60.0; // weather_poll
                if let Some(city) = app.fox.weather_wanted() {
                    crate::net::weather_poll(city);
                }
            }
            if let Some(code) = crate::net::CODE.lock().unwrap().take() {
                app.fox.slow.wx.code = Some(code);
            }
            if t >= app.next_slow {
                app.next_slow = t + 30.0;
                app.fox.life_slow(t);
            }
            app.fox.pump(t);
            app.fox.win_move = None;
            let f = app.fox.frame(t, (p.x, p.y), (app.x, app.y));
            if let Some((nx, ny)) = app.fox.win_move.take() {
                if !app.drag.is_some() {
                    app.x = nx;
                    app.y = ny;
                }
            }
            unsafe { present(hwnd, app, &f) };
            handle_events(app, t);
            let root = (app.x, app.y);
            if let Some(b) = app.bubble.as_mut() {
                if b.shown && t >= b.hide_at {
                    b.hide();
                } else {
                    b.place(&app.fox, root, false);
                }
            }
        });
    }

    /// what the hub's Settings page asks of the app: the fox's size and colours (as the menu does), the font (the hub
    /// opens again with it), a voice test
    fn hub_run(app: &mut App, c: &str) {
        if c.starts_with("size:") {
            run_command(app, c);
        } else if c.starts_with("theme:") {
            run_command(app, c);
            if crate::hubw::is_open() {
                crate::hubw::close();
                open_hub(app, Some("settings"));
            }
        } else if c == "app_font" {
            if crate::hubw::is_open() {
                crate::hubw::close();
                open_hub(app, Some("settings"));
            }
        } else if let Some(page) = c.strip_prefix("reopen:") {
            // (the accent, the compact sidebar, Start over: the window is made again with them, on the same page)
            app.fox.apply_look();
            if crate::hubw::is_open() {
                crate::hubw::close();
                open_hub(app, Some(page));
            }
        } else if let Some(k) = c.strip_prefix("pose:") {
            let t = now(app);
            let win = (app.x, app.y);
            app.fox.z19_pose_start(t, k, true, win);
        } else if c == "speak_test" {
            // speak_test: the voice says hello with the chosen voice and speed
            let c = &app.fox.cfg;
            let name = app.fox.user_short();
            let text = format!("Kon kon! Hi {}! This is how I sound. Ready to focus?", if name.is_empty() { "there" } else { &name });
            crate::speak::say(&text, &c.s("tts_voice", ""), c.f("tts_speed", 0.0) as i64, c.f("tts_pitch", 0.0) as i64);
        } else {
            app.fox.trace.push(format!("hub:{c}"));
        }
    }

    fn run_command(app: &mut App, c: &str) {
        let t = now(app);
        if c == "focus" {
            open_dialog(app, t);
            return;
        }
        let fox = &mut app.fox;
        if let Some(k) = c.strip_prefix("play:") {
            fox.interact(t, k, None);
        } else if let Some(k) = c.strip_prefix("treat:") {
            fox.interact(t, "treat", Some(k.to_string()));
        } else if c == "sorry" {
            fox.forgive(t, false, "sorry", None);
        } else if c == "pause" {
            fox.toggle_pause();
        } else if c == "stop" {
            fox.end_session(t, false);
        } else if let Some(n) = c.strip_prefix("size:") {
            let n: i64 = n.parse().unwrap_or(4);
            let (ow, oh) = (fox.w, fox.h);
            update_config(&mut fox.cfg, |v| v["size"] = n.into());
            fox.compute_dims();
            fox.apply_look();
            let (nx, ny) = (app.x + ow - fox.w, app.y + oh - fox.h);
            app.x = nx;
            app.y = ny;
            update_config(&mut fox.cfg, |v| {
                v["x"] = nx.into();
                v["y"] = ny.into();
            });
        } else if let Some(n) = c.strip_prefix("theme:") {
            update_config(&mut fox.cfg, |v| v["theme"] = n.into());
            fox.apply_look();
        } else if let Some(n) = c.strip_prefix("mode:") {
            update_config(&mut fox.cfg, |v| v["mode"] = n.into());
        } else if let Some(k) = c.strip_prefix("toggle:") {
            let on = !fox.cfg.b(k, true);
            update_config(&mut fox.cfg, |v| v[k] = on.into());
        } else if let Some(rest) = c.strip_prefix("close:") {
            let (kind, site) = rest.split_once(':').unwrap_or((rest, ""));
            let ok = close_distraction(kind);
            fox.closed_distraction(t, site, ok);
        } else if let Some(i) = c.strip_prefix("tt_start:") {
            if let Some(i) = i.parse::<usize>().ok().filter(|&i| i < fox.tt.len()) {
                let it = fox.tt[i].clone();
                let task = it["task"].as_str().map(String::from).unwrap_or_else(|| it["task"].to_string());
                fox.start_session_item(t, &task, it["mins"].as_f64().unwrap_or(30.0), Some(i));
            }
        } else if fox.ema_ring_button(c) || fox.books_button(t, c) || fox.study_button(t, c) || fox.guard_button(t, c) {
        } else if let Some(id) = c.strip_prefix("ema_done:") {
            fox.ema_set_done(id);
        } else if c == "music_apply" {
            fox.music_apply();
        } else if c == "open_hub" || c.starts_with("open_hub:") || c.starts_with("hub:") {
            let page = c.strip_prefix("open_hub:").or_else(|| c.strip_prefix("hub:"));
            open_hub(app, page.filter(|p| *p != "None"));
            return;
        } else if c == "open_checkin" || c == "open_checkin_force" {
            open_checkin(app, t, c == "open_checkin_force");
            return;
        } else if c == "lockin" {
            fox.lockin_start(t, None, false);
        } else if let Some(m) = c.strip_prefix("lockin:") {
            fox.lockin_start(t, m.parse().ok(), false);
        } else if c == "lockin_resume" {
            if fox.lockin.is_none() {
                fox.lockin_start(t, None, true);
            }
        } else if c == "lockout" {
            fox.lockin_stop(t, false, false);
        } else if c == "break" {
            fox.start_break(t, None);
        } else if c == "endbreak" {
            fox.end_break(t, false);
        } else if c == "quit" {
            unsafe { DestroyWindow(app.hwnd) };
        }
    }

    /// _jp_font: a Japanese-capable font if Windows has one, else ""
    fn jp_font() -> String {
        ["Yu Gothic UI", "Yu Gothic", "Meiryo UI", "Meiryo", "MS Gothic", "Noto Sans CJK JP", "Noto Sans JP", "IPAGothic"]
            .iter().find(|n| crate::gdi::font_exists(n)).map(|n| n.to_string()).unwrap_or_default()
    }

    /// what the hub's sidebar shows about the app right now
    fn side_ctx(app: &App) -> zenko_desktop::hubside::SideCtx {
        let fox = &app.fox;
        let c = &fox.cfg;
        let (_, month, _, hh, mm, _) = zenko_desktop::local_now();
        let fname = c.s("fox_name", "");
        let fname: String = fname.trim().chars().take(20).collect();
        let cp = c.s("catchphrase", "");
        let cp: String = cp.trim().chars().take(40).collect();
        zenko_desktop::hubside::SideCtx {
            t: theme(c),
            font_ui: crate::gdi::ui_font(&c.s("app_font", "")),
            jp: jp_font(),
            fox_name: if fname.is_empty() { "Zenko".into() } else { fname },
            catchphrase: if cp.is_empty() { "Kon! Kon!".into() } else { cp },
            part: zenko_desktop::hubside::daypart(hh, mm).0.into(),
            season: zenko_desktop::hubside::season_of(month).into(),
            sky: c.b("sky_daynight", true),
            seasons: c.b("sky_seasons", true),
            weather: fox.slow.wx.code.map(|_| fox.wx_kind().to_string()),
            nav_compact: c.b("nav_compact", false),
            zk_f: 1.0,
            hd: c.b("hd_art", true),
            pins: zenko_desktop::hubwin::side_pins(&zenko_desktop::read_json("zenko_life.json")),
            linked: false,
            scene: String::new(),
        }
    }

    fn hub_status(app: &App) -> String {
        let fox = &app.fox;
        zenko_desktop::hubwin::status_line(fox.session.as_ref().map(|s| (s.task.as_str(), s.planned, s.elapsed)),
                                           fox.brk.is_some(), zenko_desktop::slow::today_focus_minutes())
    }

    /// open_hub(page): the main window
    fn open_hub(app: &mut App, page: Option<&str>) {
        let c = &app.fox.cfg;
        let t = theme(c);
        let init = crate::hubw::HubInit {
            t,
            opened_s: app.t0.elapsed().as_secs_f64(),
            font_ui: crate::gdi::ui_font(&c.s("app_font", "")),
            navkeys: zenko_desktop::hubwin::nav_list(c),
            ctx: side_ctx(app),
            status: hub_status(app),
            dpi: app.dpi,
            icon_px: zenko_desktop::icon::icon_pixels(&c.s("theme", "snow")),
            ui_border: c.b("ui_border", true),
            text_scale: c.f("text_scale", 1.0),
            theme: c.s("theme", "snow"),
            living: c.b("sidebar_living", true),
            particles: c.b("sidebar_particles", true),
            slide_s: {
                // z20_motion: Auto follows the battery
                let mut lvl = c.s("motion_level", "auto");
                if lvl == "auto" {
                    lvl = match app.fox.battery {
                        Some((p, false)) => if p <= 20 { "saver" } else { "balanced" }.into(),
                        _ => "smooth".into(),
                    };
                }
                (c.b("page_fx", true) && lvl != "saver").then_some(if lvl == "smooth" { 0.230 } else { 0.230 * 0.75 })
            },
        };
        crate::hubw::open(page, init);
    }

    /// open_checkin(force): the daily "how are you feeling?" card beside her
    fn open_checkin(app: &mut App, t: f64, force: bool) {
        if crate::uiwin::is_open("checkin") {
            crate::uiwin::lift("checkin");
            return;
        }
        let fox = &mut app.fox;
        if !fox.checkin_opening(t, force) {
            return;
        }
        let ui = fox.checkin_ui(&crate::gdi::ui_font(&fox.cfg.s("app_font", "")));
        let scr = unsafe { (GetSystemMetrics(SM_CXSCREEN), GetSystemMetrics(SM_CYSCREEN)) };
        let root = (app.x, app.y);
        let fox = &app.fox;
        crate::uiwin::open_popup("checkin", ui, theme(&fox.cfg), app.dpi, |w, h| fox.checkin_pos(root, w, h, scr));
    }

    /// open_dialog (zenko.py double-click / "Start focus session..."): the new focus session window.
    fn open_dialog(app: &mut App, t: f64) {
        if crate::dialog::is_open() {
            crate::dialog::lift();
            return;
        }
        let fox = &mut app.fox;
        if fox.session.is_some() {
            fox.say("We're already in a session. Right-click me to pause or stop it.", 6.0);
            return;
        }
        fox.events.push(Ev::Ui("open".into()));
        let im = fox.small_fox(t);
        let c = &fox.cfg;
        let setup = crate::dialog::Setup {
            theme: theme(c),
            font_ui: crate::gdi::ui_font(&c.s("app_font", "")),
            dpi: app.dpi,
            last_task: c.s("last_task", ""),
            last_minutes: crate::dialog::py_str(c.raw.get("last_minutes")),
            hint: if c.b("learn_habits", true) { zenko_desktop::watch::learned_hint() } else { String::new() },
            fox: (im.w as i32, im.h as i32, im.px.clone()),
            icon_px: zenko_desktop::icon::icon_pixels(&c.s("theme", "snow")),
        };
        crate::dialog::open(setup);
    }

    pub fn bubble_click(x: i32, y: i32) {
        APP.with(|a| {
            let Ok(mut g) = a.try_borrow_mut() else { return };
            let Some(app) = g.as_mut() else { return };
            let Some(b) = app.bubble.as_mut() else { return };
            let hit = b.chips.iter().find(|c| c.0 <= x && x <= c.2 && c.1 <= y && y <= c.3).map(|c| c.4.clone());
            let plain = b.chips.is_empty();
            if let Some(c) = hit {
                app.fox.events.push(Ev::Ui("tap".into()));
                b.hide();
                run_command(app, &c);
            } else if plain {
                b.hide(); // tap a plain bubble to dismiss it
            }
        });
    }

    fn today_minutes() -> i64 {
        (zenko_desktop::stats().get(zenko_desktop::today_iso()).and_then(|v| v.as_f64()).unwrap_or(0.0) / 60.0)
            .floor() as i64
    }

    /// on_menu + menu_extras, in the same order with the same words.
    fn build_menu(app: &App) -> Vec<Item> {
        let fox = &app.fox;
        let c = &fox.cfg;
        let name = {
            let n = c.s("fox_name", "Zenko");
            let n: String = n.trim().chars().take(20).collect();
            if n.is_empty() { "Zenko".to_string() } else { n }
        };
        let mut m = vec![info(format!("Today: {} min focused", today_minutes())), Item::Sep];
        if let Some(s) = &fox.session {
            m.push(cmd(if s.paused { "Resume" } else { "Pause" }, "pause"));
            m.push(cmd("Stop session", "stop"));
        } else {
            m.push(cmd("Start focus session...", "focus"));
        }
        if let (Some(i), None) = (fox.focus_item(), &fox.session) {
            let it = &fox.tt[i];
            let task = it["task"].as_str().map(String::from).unwrap_or_else(|| it["task"].to_string());
            m.push(cmd(&format!("Start timetable item: {} ({} min)", zenko_desktop::session::clip(&task, 22),
                                it["mins"].as_i64().unwrap_or(30)), &format!("tt_start:{i}")));
        }
        m.push(Item::Sep);
        m.push(cmd("Open Zenko window", "open_hub"));
        m.push(later("Chat with the fox..."));
        m.push(later("Talk with her out loud"));
        m.push(later("Breathe with me (Zen garden)"));
        m.push(later("Today's timetable..."));
        m.push(later("\u{6025} Something urgent (push my day back)"));
        if fox.brk.is_some() {
            m.push(cmd("End break", "endbreak"));
        } else {
            m.push(cmd(format!("Take a {}-min break", c.f("break_minutes", 5.0) as i64), "break"));
        }
        m.push(later("Export timetable as PDF (reMarkable)..."));
        m.push(Item::Sep);
        // menu_extras (ZenkoExtras)
        let mut pm: Vec<Item> = [
            ("Headpat", "pet"), ("Belly rub", "belly"), ("Boop her nose", "boop"), ("Tickle", "tickle"),
            ("Fluff her tails", "tail"), ("Ball of yarn", "yarn"), ("Hide and seek", "hide"), ("High five", "highfive"),
            ("Give her a gift", "gift"), ("Dance", "dance"), ("Brush her fur", "brush"), ("Sing a song", "sing"),
        ]
        .iter()
        .map(|(l, k)| cmd(*l, &format!("play:{k}")))
        .collect();
        let fm: Vec<Item> = zenko_desktop::play::TREATS
            .iter()
            .map(|t| {
                let title: String = t.split(' ').map(|w| {
                    let mut ch = w.chars();
                    ch.next().map(|f| f.to_uppercase().collect::<String>() + ch.as_str()).unwrap_or_default()
                }).collect::<Vec<_>>().join(" ");
                cmd(title, &format!("treat:{t}"))
            })
            .collect();
        pm.push(sub("Feed her a treat", fm));
        if fox.sulk_until > 0.0 {
            pm.push(cmd("Say sorry", "sorry"));
        }
        pm.push(Item::Sep);
        pm.push(later("Draw an omikuji fortune"));
        pm.push(later("Japanese word of the day"));
        m.push(sub(format!("Play with {name}"), pm));
        if fox.lockin.is_some() {
            m.push(cmd(format!("Exit lock-in ({} min left)", fox.lockin_left_min()), "lockout"));
        } else {
            let lm = c.f("lockin_minutes", 50.0) as i64;
            m.push(cmd(format!("Lock-in mode ({} min)", if lm == 0 { 50 } else { lm }), "lockin"));
        }
        m.push(later(format!("Customize {name}...")));
        // menu_extras (ZenkoLife)
        let mut lm = vec![
            later("Breathing exercise"),
            cmd("Daily check-in", "open_checkin_force"),
            later("Review flashcards"),
            later("My study plan"),
            later("Streak and badges"),
            Item::Sep,
        ];
        for (lab, key) in [("Roaming", "roam"), ("Seasons and weather", "seasons"), ("Day and night", "daynight")] {
            lm.push(check(lab, &format!("toggle:{key}"), c.b(key, true)));
        }
        lm.push(check("Background music", "toggle:bgm", c.b("bgm", true)));
        m.push(sub("Daily life", lm));
        m.push(Item::Sep);
        let size = c.f("size", 4.0) as i64;
        m.push(sub("Fox size", [("Tiny", 3), ("Small", 4), ("Medium", 5), ("Large", 6)]
            .iter().map(|(l, v)| radio(*l, &format!("size:{v}"), size == *v)).collect()));
        let th = c.s("theme", "snow");
        m.push(sub("Fox colours", [("snow", "Byakko"), ("classic", "Inari"), ("ember", "Ember"), ("fennec", "Desert"),
                                    ("midnight", "Midnight"), ("sakura", "Sakura"), ("mint", "Jade"), ("silver", "Moonlight")]
            .iter().map(|(v, l)| radio(*l, &format!("theme:{v}"), th == *v)).collect()));
        let mode = c.s("mode", "mixed");
        m.push(sub("Fox mood", [("Gentle", "gentle"), ("Mixed (kind, then savage)", "mixed"), ("Savage", "savage")]
            .iter().map(|(l, v)| radio(*l, &format!("mode:{v}"), mode == *v)).collect()));
        m.push(check("Sound", "toggle:sound", c.b("sound", true)));
        m.push(check("Nag me even without a session", "toggle:watch_without_session", c.b("watch_without_session", true)));
        m.push(check("Auto-close distractions (in a session)", "toggle:auto_close", c.b("auto_close", true)));
        m.push(check("Fox looks at my mouse", "toggle:look_at_mouse", c.b("look_at_mouse", true)));
        m.push(Item::Check { label: "Start with Windows".into(), cmd: String::new(), on: false, enabled: false });
        m.push(Item::Sep);
        m.push(check("Internet lookups (weather, search...)", "toggle:web_enabled", c.b("web_enabled", true)));
        m.push(later("Smart brain (offline AI) setup..."));
        m.push(later("Edit settings (distraction list)..."));
        m.push(later("Reload settings"));
        m.push(Item::Sep);
        m.push(cmd("Quit", "quit"));
        m
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
                        if let Some(app) = a.try_borrow_mut().ok().as_mut().and_then(|g| g.as_mut()) {
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
                        if let Some(app) = a.try_borrow_mut().ok().as_mut().and_then(|g| g.as_mut()) {
                            if let Some((dx, dy, sx, sy, moved)) = app.drag {
                                if moved || (p.x - sx).abs() + (p.y - sy).abs() > 4 {
                                    app.x = p.x - dx;
                                    app.y = p.y - dy;
                                    app.drag = Some((dx, dy, sx, sy, true));
                                    SetWindowPos(hwnd, std::ptr::null_mut(), app.x, app.y, 0, 0,
                                                 SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE);
                                }
                            }
                        }
                    });
                    0
                }
                WM_LBUTTONUP => {
                    ReleaseCapture();
                    let (cx, cy) = ((lp & 0xffff) as i16 as i32, ((lp >> 16) & 0xffff) as i16 as i32);
                    APP.with(|a| {
                        if let Some(app) = a.try_borrow_mut().ok().as_mut().and_then(|g| g.as_mut()) {
                            let moved = app.drag.map(|d| d.4);
                            app.drag = None;
                            match moved {
                                Some(true) => {
                                    let (x, y) = (app.x, app.y);
                                    let t = now(app);
                                    app.fox.dragged(t);
                                    update_config(&mut app.fox.cfg, |v| {
                                        v["x"] = x.into();
                                        v["y"] = y.into();
                                    });
                                }
                                Some(false) if app.fox.tablet_hit(now(app), cx, cy).is_some() => {
                                    let t = now(app);
                                    let n = app.fox.tablet_hit(t, cx, cy).unwrap_or("screen");
                                    app.fox.tablet_press(t, n);
                                }
                                Some(false) if cx >= app.fox.colw => {
                                    // a click on the fox itself (head, nose, belly, tails react differently)
                                    let t = now(app);
                                    app.fox.fox_clicked(t, cx, cy);
                                }
                                _ => {}
                            }
                        }
                    });
                    0
                }
                WM_LBUTTONDBLCLK => {
                    APP.with(|a| {
                        if let Some(app) = a.try_borrow_mut().ok().as_mut().and_then(|g| g.as_mut()) {
                            let t = now(app);
                            open_dialog(app, t);
                        }
                    });
                    0
                }
                WM_RBUTTONUP => {
                    let built = APP.with(|a| a.try_borrow().ok().and_then(|g| g.as_ref().map(|app| (build_menu(app), theme(&app.fox.cfg)))));
                    if let Some((items, th)) = built {
                        let chosen = menu::popup(hwnd, &items, &th);
                        if !chosen.is_empty() {
                            APP.with(|a| {
                                if let Some(app) = a.try_borrow_mut().ok().as_mut().and_then(|g| g.as_mut()) {
                                    run_command(app, &chosen);
                                }
                            });
                        }
                    }
                    0
                }
                WM_MEASUREITEM => {
                    menu::measure(lp as *mut MEASUREITEMSTRUCT);
                    1
                }
                WM_DRAWITEM => {
                    menu::draw(lp as *const DRAWITEMSTRUCT);
                    1
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
            let dpi = GetDpiForSystem() as f64;
            let u = (dpi / 96.0).max(1.0);
            crate::gdi::load_fonts();
            let t0 = Instant::now();
            let cfg = Cfg::load();
            let family = if cfg.s("bubble_font", "app") == "app" {
                crate::gdi::ui_font(&cfg.s("app_font", ""))
            } else {
                "Consolas".to_string()
            };
            let mut fox = Companion::new(cfg, u, 1000.0);
            fox.life_startup();
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
                    bubble: None,
                    variety: Variety::default(),
                    t0,
                    x,
                    y,
                    drag: None,
                    dib: None,
                    dpi,
                    family,
                    last_tick: 1000.0,
                    next_tick: 1001.0,
                    panel_fonts: None,
                    panel_dib: None,
                    fg: (String::new(), String::new()),
                    next_lock: 0.0,
                    next_player: 1001.2,
                    next_watch: 1008.0,
                    next_slow: 1006.0,
                    next_wx: 1009.0,
                    next_batt: 1001.5,
                    eating: None,
                    next_top: 1002.5,
                    next_hub: 1001.0,
                    hwnd,
                })
            });
            APP.with(|a| {
                if let Some(app) = a.borrow_mut().as_mut() {
                    app.fox.delayed.push((1003.5, Ev::Command("music_apply".into())));
                }
            });
            // open_hub_on_start: the main window comes up 250 ms after the fox (zenko20)
            APP.with(|a| {
                if let Some(app) = a.borrow_mut().as_mut()
                    && app.fox.cfg.b("open_hub_on_start", true) {
                    app.fox.delayed.push((1000.25, Ev::Command("open_hub".into())));
                }
            });
            if Companion::lockin_pending() {
                APP.with(|a| {
                    if let Some(app) = a.borrow_mut().as_mut() {
                        app.fox.delayed.push((1002.5, Ev::Command("lockin_resume".into())));
                    }
                });
            }
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
