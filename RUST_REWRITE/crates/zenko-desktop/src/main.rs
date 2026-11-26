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
mod gdi;
#[cfg(windows)]
mod menu;

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
        next_top: f64,
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
            }
        }
    }

    unsafe fn tick(hwnd: HWND) {
        APP.with(|a| {
            let mut g = a.borrow_mut();
            let Some(app) = g.as_mut() else { return };
            let t = now(app);
            if t >= app.next_tick {
                // zenko.py tick(), once a second
                let dt = (t - app.last_tick).clamp(0.0, 5.0);
                app.last_tick = t;
                app.next_tick = t + 1.0;
                app.fox.tick_1s(t, dt, idle_seconds());
            }
            if t >= app.next_top {
                app.next_top = t + 2.5; // keep_on_top
                unsafe {
                    SetWindowPos(hwnd, HWND_TOPMOST, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE);
                }
            }
            let mut p = POINT { x: 0, y: 0 };
            unsafe { GetCursorPos(&mut p) };
            let f = app.fox.frame(t, (p.x, p.y), (app.x, app.y));
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
        } else if c == "quit" {
            unsafe { DestroyWindow(app.hwnd) };
        }
    }

    /// open_dialog (zenko.py double-click): the session dialog itself is the next stage, so for now this starts a
    /// session straight away with the task and minutes you used last time (the dialog's own defaults).
    fn open_dialog(app: &mut App, t: f64) {
        let fox = &mut app.fox;
        if fox.session.is_some() {
            fox.say("We're already in a session. Right-click me to pause or stop it.", 6.0);
            return;
        }
        let task = fox.cfg.s("last_task", "");
        let task = if task.trim().is_empty() { "Focus session".to_string() } else { task.trim().to_string() };
        let m = fox.cfg.f("last_minutes", 25.0).clamp(1.0, 240.0).floor();
        fox.start_session(t, &task, m);
    }

    pub fn bubble_click(x: i32, y: i32) {
        APP.with(|a| {
            let mut g = a.borrow_mut();
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
        m.push(Item::Sep);
        m.push(later("Open Zenko window"));
        m.push(later("Chat with the fox..."));
        m.push(later("Talk with her out loud"));
        m.push(later("Breathe with me (Zen garden)"));
        m.push(later("Today's timetable..."));
        m.push(later("\u{6025} Something urgent (push my day back)"));
        m.push(later(format!("Take a {}-min break", c.f("break_minutes", 5.0) as i64)));
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
        m.push(later(format!("Lock-in mode ({} min)", c.f("lockin_minutes", 50.0) as i64)));
        m.push(later(format!("Customize {name}...")));
        // menu_extras (ZenkoLife)
        let mut lm = vec![
            later("Breathing exercise"),
            later("Daily check-in"),
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
                        if let Some(app) = a.borrow_mut().as_mut() {
                            let moved = app.drag.map(|d| d.4);
                            app.drag = None;
                            match moved {
                                Some(true) => {
                                    let (x, y) = (app.x, app.y);
                                    update_config(&mut app.fox.cfg, |v| {
                                        v["x"] = x.into();
                                        v["y"] = y.into();
                                    });
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
                        if let Some(app) = a.borrow_mut().as_mut() {
                            let t = now(app);
                            open_dialog(app, t);
                        }
                    });
                    0
                }
                WM_RBUTTONUP => {
                    let built = APP.with(|a| a.borrow().as_ref().map(|app| (build_menu(app), theme(&app.fox.cfg))));
                    if let Some((items, th)) = built {
                        let chosen = menu::popup(hwnd, &items, &th);
                        if !chosen.is_empty() {
                            APP.with(|a| {
                                if let Some(app) = a.borrow_mut().as_mut() {
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
            let fox = Companion::new(cfg, u, 1000.0);
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
                    next_top: 1002.5,
                    hwnd,
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
