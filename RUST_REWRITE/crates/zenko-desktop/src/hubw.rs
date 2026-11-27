//! The hub window on Windows: FoxBuddy.open_hub's borderless window (make_borderless: no Windows title bar, still in
//! the taskbar; round_corners: Windows 11 rounded corners with a thin gold border), hub_fade_in, hub_make_drag,
//! hub_resize_edges, hub_minimize / hub_toggle_max / hub_close, the sidebar's hover and clicks, zk_fit after a resize
//! and the parts of hub_anim / hub_tick that live in the frame (menu pill glide, the fading tips, the status line).

use crate::gdi;
use crate::uiwin::{GdiFonts, to_ops};
use std::cell::RefCell;
use windows_sys::Win32::Foundation::*;
use windows_sys::Win32::Graphics::Dwm::DwmSetWindowAttribute;
use windows_sys::Win32::Graphics::Gdi::*;
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::*;
use windows_sys::Win32::UI::WindowsAndMessaging::*;
use zenko_desktop::hubside::SideCtx;
use zenko_desktop::hubwin::{Hub, zk_factor};
use zenko_desktop::theme::Theme;
use zenko_desktop::chatpage::ChatPage;
use zenko_desktop::ui::{Draw, Kind, Place};

/// what the app hands the window when it opens
pub struct HubInit {
    pub t: Theme,
    pub font_ui: String,
    pub navkeys: Vec<(String, String)>,
    pub ctx: SideCtx,
    pub status: String,
    pub dpi: f64,
    pub icon_px: Vec<[u8; 4]>,
    pub ui_border: bool,
    pub text_scale: f64,
    pub theme: String,
    pub living: bool,
    pub particles: bool,
    /// how long a page slide takes (SLIDE_MS, x0.75 at the "balanced" motion level); None: no slide (page_fx off, or
    /// the "saver" level)
    pub slide_s: Option<f64>,
}

struct HubW {
    hwnd: HWND,
    hub: Hub,
    fonts: GdiFonts,
    ctx: SideCtx,
    t0: std::time::Instant,
    text_scale: f64,
    zen_maru: bool,
    maxed: bool,
    restore: (i32, i32, i32, i32),
    drag: Option<(i32, i32)>,
    drag_at: (i32, i32),
    drag_moved: bool,
    rs: Option<(String, i32, i32, i32, i32, i32, i32)>,
    fit_at: Option<f64>,
    size: (i32, i32),
    min: (i32, i32),
    cursor: PCWSTR,
    faded: bool,
    anim: zenko_desktop::hubanim::HubAnim,
    theme: String,
    living: bool,
    particles: bool,
    minimized: bool,
    slide_s: Option<f64>,
    dpi: f64,
    /// the pages built so far (hub_build_<name>) and the slide between two of them
    pages: Vec<(String, usize)>,
    slide: Option<Slide>,
    /// the Ask Zenko page: its widgets, how many of her chat lines it shows, the queue and the reply being typed
    chat: Option<ChatPage>,
    chat_seen: Option<u64>,
    chat_q: std::collections::VecDeque<(String, String)>,
    typing: Option<(Vec<(String, Vec<String>)>, usize, f64)>,
    sb_drag: Option<(f64, f64)>,
}

/// z21_show's vertical slide: the new page comes in from below (or above), the old one goes out the other way
struct Slide {
    pg: usize,
    op: usize,
    d: i32,
    t0: f64,
}

const SERIF: &str = "Georgia";
const MONO: &str = "Consolas";

type PCWSTR = *const u16;

thread_local! {
    static HUB: RefCell<Option<HubW>> = const { RefCell::new(None) };
    /// what the app should do: "sound:<kind>", "closed"
    pub static OUT: RefCell<Vec<String>> = const { RefCell::new(vec![]) };
}

fn out(s: &str) {
    OUT.with(|q| q.borrow_mut().push(s.into()));
}

fn now(x: &HubW) -> f64 {
    x.t0.elapsed().as_secs_f64()
}

pub fn is_open() -> bool {
    HUB.with(|h| h.try_borrow().map(|v| v.is_some()).unwrap_or(true))
}

fn with<R>(f: impl FnOnce(&mut HubW) -> R) -> Option<R> {
    HUB.with(|h| h.try_borrow_mut().ok().and_then(|mut v| v.as_mut().map(f)))
}

/// hub_show for the frame: the menu pill moves to the page (the pages themselves come with their own stages)
fn show(x: &mut HubW, page: &str) {
    if x.hub.side.page.as_deref() == Some(page) {
        after_show(x, page);
        return;
    }
    finish_slide(x);
    let old = x.hub.side.page.clone();
    let order: Vec<&str> = x.hub.side.navkeys.iter().map(|(k, _)| k.as_str()).collect();
    let d = match (order.iter().position(|k| *k == page), old.as_deref().and_then(|o| order.iter().position(|k| *k == o))) {
        (Some(a), Some(b)) if a < b => -1,
        _ => 1,
    };
    x.hub.side.page = Some(page.into());
    let c = x.ctx.clone();
    x.hub.side.nav(&c);
    x.hub.side_sync();
    let pg = page_frame(x, page);
    let op = old.as_deref().and_then(|o| x.pages.iter().find(|(k, _)| k == o).map(|p| p.1));
    let hh = x.hub.ui.ws[x.hub.ids.stage].h;
    let smooth = x.slide_s.is_some() && hh > 50 && op.is_some() && pg.is_some();
    if let (true, Some(pg), Some(op)) = (smooth, pg, op) {
        place(x, pg, Some(d * hh));
        x.hub.ui.ws[op].low = true;
        x.hub.ui.ws[pg].low = false;
        x.slide = Some(Slide { pg, op, d, t0: now(x) });
    } else {
        if let Some(op) = op {
            place(x, op, None);
        }
        if let Some(pg) = pg {
            place(x, pg, Some(0));
        }
    }
    relayout(x);
    after_show(x, page);
}

/// pg.place(x=0, y=.., relwidth=1, relheight=1), or place_forget with None
fn place(x: &mut HubW, id: usize, y: Option<i32>) {
    let w = &mut x.hub.ui.ws[id];
    match y {
        Some(y) => w.place = Some(Place { y, relw: Some(1.0), relh: Some(1.0), ..Default::default() }),
        None => {
            w.place = None;
            w.mapped = false;
        }
    }
}

fn finish_slide(x: &mut HubW) {
    if let Some(s) = x.slide.take() {
        place(x, s.op, None);
        place(x, s.pg, Some(0));
        x.hub.ui.ws[s.op].low = false;
        relayout(x);
    }
}

/// hub_after_show: the chat's entry takes the keyboard
fn after_show(x: &mut HubW, page: &str) {
    if page == "chat" {
        if let Some(cp) = &x.chat {
            x.hub.ui.focus = Some(cp.entry);
        }
    }
}

/// the frame of a page, built the first time it's shown (only the pages ported so far)
fn page_frame(x: &mut HubW, page: &str) -> Option<usize> {
    if let Some(p) = x.pages.iter().find(|(k, _)| k == page) {
        return Some(p.1);
    }
    let pg = match page {
        "chat" => {
            let th = x.hub.t.clone();
            let pg = x.hub.ui.frame(x.hub.ids.stage, &th.paper);
            let cm10 = (100.0 * x.dpi / 25.4 + 0.5) as i32;
            let muted = crate::win::with_fox(|fox, _| fox.cfg.b("voice_muted", false)).unwrap_or(false);
            x.chat = Some(ChatPage::build(&mut x.hub.ui, pg, &th, &x.hub.font_ui, muted, cm10));
            x.chat_seen = None;
            pg
        }
        _ => return None,
    };
    x.pages.push((page.into(), pg));
    Some(pg)
}

/// lay the window out again, with zk_fit's fonts and the chat banner for the new width
fn relayout(x: &mut HubW) {
    let (w, h) = x.size;
    x.hub.layout(&x.fonts, w, h);
    if let Some(cp) = x.chat.as_mut() {
        let zk = x.hub.zk_f;
        cp.fit(&mut x.hub.ui, &x.fonts, zk);
        x.hub.layout(&x.fonts, w, h);
        let font_ui = x.hub.font_ui.clone();
        let th = x.hub.t.clone();
        if cp.draw_title(&mut x.hub.ui, &x.fonts, &th, &font_ui, "", zk) {
            x.hub.layout(&x.fonts, w, h);
        }
        cp.sync_scroll(&mut x.hub.ui, &x.fonts, &th);
    }
}

/// her chat lines into the page: the ones there already when it opens (or her hello), then each new one through
/// _chat_pump, her replies typed out a few letters every 14 ms. True when something changed.
fn chat_step(x: &mut HubW) -> bool {
    let t = now(x);
    let Some(cp) = x.chat.as_mut() else { return false };
    let Some((log, n, name)) = crate::win::with_fox(|fox, t| {
        if x.chat_seen.is_none() && fox.chat_log.is_empty() {
            fox.chat_line(t, "Fox", "Hi! Type 'help' to see what I can do.");
            return (vec![], fox.chat_n - 1, fox.fox_name());
        }
        (fox.chat_log.clone(), fox.chat_n, fox.fox_name())
    }) else { return false };
    let th = x.hub.t.clone();
    let font_ui = x.hub.font_ui.clone();
    let mut changed = false;
    match x.chat_seen {
        None => {
            for (who, t) in &log {
                cp.insert(&mut x.hub.ui, &x.fonts, &th, &font_ui, SERIF, MONO, &name, who == "You", t);
            }
            x.chat_seen = Some(n);
            changed = true;
        }
        Some(seen) if n > seen => {
            let k = ((n - seen) as usize).min(log.len());
            x.chat_q.extend(log[log.len() - k..].iter().cloned());
            x.chat_seen = Some(n);
        }
        _ => {}
    }
    loop {
        if let Some((chunks, i, at)) = x.typing.as_mut() {
            while *i < chunks.len() && t >= *at {
                let (s, tg) = &chunks[*i];
                cp.type_chunk(&mut x.hub.ui, &x.fonts, s, tg);
                *i += 1;
                *at += 0.014;
                changed = true;
            }
            if *i < chunks.len() {
                break;
            }
            cp.type_chunk(&mut x.hub.ui, &x.fonts, "\n\n", &["fox".to_string()]);
            x.typing = None;
            changed = true;
        }
        let Some((who, text)) = x.chat_q.pop_front() else { break };
        changed = true;
        if who == "You" || text.chars().count() < 3 {
            cp.insert(&mut x.hub.ui, &x.fonts, &th, &font_ui, SERIF, MONO, &name, who == "You", &text);
            continue;
        }
        let chunks = cp.begin_typed(&mut x.hub.ui, &th, &font_ui, SERIF, MONO, &name, &text);
        x.typing = Some((chunks, 0, t));
    }
    if changed {
        cp.sync_scroll(&mut x.hub.ui, &x.fonts, &th);
    }
    changed
}

/// what the chat page's buttons do
fn chat_cmd(x: &mut HubW, cmd: &str) {
    let Some(cp) = x.chat.as_ref() else { return };
    let (entry, status, mute) = (cp.entry, cp.status, cp.mute);
    let send = |q: &str| {
        let q = q.to_string();
        crate::win::with_fox_at(move |fox, t, win| {
            if let zenko_desktop::chat::Sent::Brain(_) = fox.chat_send(t, &q, win) {
                fox.brain_fallback(t);
            }
        });
    };
    if let Some(q) = cmd.strip_prefix("chat:") {
        send(q);
    } else if cmd == "chat_send" {
        let q = x.hub.ui.text_of(entry).trim().to_string();
        if !q.is_empty() {
            if let Kind::Entry { e, .. } = &mut x.hub.ui.ws[entry].kind {
                (e.text, e.pos, e.anchor, e.sel, e.left) = (vec![], 0, 0, None, 0);
            }
            send(&q);
        }
    } else if cmd == "chat_mute" {
        // voice_mute(): her voice off / on, and the button says which
        let on = crate::win::with_fox(|fox, _| {
            let on = !fox.cfg.b("voice_muted", false);
            zenko_desktop::update_config(&mut fox.cfg, |v| v["voice_muted"] = on.into());
            on
        }).unwrap_or(false);
        let th = x.hub.t.clone();
        if let Kind::Button { text, color, fg, .. } = &mut x.hub.ui.ws[mute].kind {
            *text = if on { "Unmute" } else { "Mute" }.into();
            *color = if on { th.acc.clone() } else { th.start.clone() };
            *fg = if on { th.onacc.clone() } else { th.ink.clone() };
        }
        relayout(x);
    } else if let Kind::Label { text, .. } = &mut x.hub.ui.ws[status].kind {
        // Mic, Talk, Image and Screen need her voice and eyes, which come in a later stage of the port
        *text = "That button isn't in the Rust preview yet.".into();
    }
}

/// z17_wheel over the chat text (3 lines a notch, only while there's somewhere to go)
fn chat_wheel(x: &mut HubW, mx: i32, my: i32, delta: i32) -> bool {
    let Some(cp) = x.chat.as_mut() else { return false };
    let tid = cp.text;
    let mut over = x.hub.ui.hit(mx, my);
    while let Some(i) = over {
        if i == tid {
            break;
        }
        over = x.hub.ui.ws[i].parent;
    }
    if over.is_none() || !x.hub.ui.shown(tid) {
        return false;
    }
    let (a, b) = match &x.hub.ui.ws[tid].kind {
        Kind::Text { t, .. } => t.yview(&x.fonts),
        _ => return false,
    };
    let d = if delta > 0 { -1 } else { 1 };
    let n = 1.max(delta.abs() / 120);
    if (a > 0.0005 || b < 0.9995) && !((d < 0 && a <= 0.0005) || (d > 0 && b >= 0.9995)) {
        cp.scroll(&mut x.hub.ui, &x.fonts, "units", (d * n * 3) as f64);
        let th = x.hub.t.clone();
        cp.sync_scroll(&mut x.hub.ui, &x.fonts, &th);
        return true;
    }
    false
}

/// ZkScrollbar's press (a drag on the thumb, a page on the trough), drag and hover
fn sb_event(x: &mut HubW, mx: i32, my: i32, what: &str) -> bool {
    let Some(cp) = x.chat.as_mut() else { return false };
    let sb = cp.sb;
    let (ax, ay) = x.hub.ui.abs(sb);
    let (w, h) = (x.hub.ui.ws[sb].w, x.hub.ui.ws[sb].h);
    let inside = x.hub.ui.shown(sb) && mx >= ax && mx < ax + w && my >= ay && my < ay + h;
    let pos = (my - ay) as f64 / h.max(1) as f64;
    let (lo, hi) = match &x.hub.ui.ws[cp.text].kind {
        Kind::Text { t, .. } => t.yview(&x.fonts),
        _ => (0.0, 1.0),
    };
    let th = x.hub.t.clone();
    match what {
        "press" if inside => {
            if lo <= pos && pos <= hi {
                x.sb_drag = Some((pos, lo));
            } else {
                cp.scroll(&mut x.hub.ui, &x.fonts, "pages", if pos < lo { -1.0 } else { 1.0 });
            }
        }
        "move" => {
            if let Some((p0, lo0)) = x.sb_drag {
                cp.scroll(&mut x.hub.ui, &x.fonts, "moveto", (lo0 + pos - p0).clamp(0.0, 1.0));
            } else if inside != cp.sb_hover {
                cp.sb_hover = inside;
            } else {
                return false;
            }
        }
        "release" => {
            if x.sb_drag.take().is_none() {
                return false;
            }
        }
        _ => return false,
    }
    cp.sb_hover = inside || x.sb_drag.is_some();
    cp.sync_scroll(&mut x.hub.ui, &x.fonts, &th);
    true
}

/// open_hub(page): the window comes up (or to the front, on the page asked for)
pub fn open(page: Option<&str>, init: HubInit) {
    if is_open() {
        with(|x| unsafe {
            ShowWindow(x.hwnd, SW_RESTORE);
            SetForegroundWindow(x.hwnd);
            if let Some(p) = page {
                show(x, p);
            }
            InvalidateRect(x.hwnd, std::ptr::null(), 0);
        });
        return;
    }
    unsafe {
        let (sw, sh) = (GetSystemMetrics(SM_CXSCREEN), GetSystemMetrics(SM_CYSCREEN));
        let (w, h) = (((sw as f64 * 0.88) as i32).min(1320), ((sh as f64 * 0.88) as i32).min(900));
        // Zenko21's bigger minimum size, and its re-centring when the window starts out smaller
        let min = (1120.min(sw - 40), 700.min(sh - 60));
        let (w, h) = (w.max(min.0), h.max(min.1));
        let (x0, y0) = (((sw - w) / 2).max(0), ((sh - h) / 3).max(0));
        let fonts = GdiFonts::new(init.dpi);
        let zen_maru = init.font_ui == "Zen Maru Gothic";
        let mut hub = Hub::new(&init.t, &init.font_ui, init.navkeys.clone(), true, init.ui_border, w, h,
                               (std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()
                                % zenko_desktop::tips::TIPS.len() as u128) as usize, 0.0);
        // the first layout runs while the sidebar canvas is still 1 x 1, so the logo stays at zoom 1 (as in zenko.py)
        (hub.side.w, hub.side.h) = (1, 1);
        hub.side.layout(init.ctx.nav_compact);
        hub.side.status_text = init.status.clone();
        hub.fit_fonts(zk_factor(w, h, init.text_scale, zen_maru));
        hub.layout(&fonts, w, h);
        let mut ctx = init.ctx.clone();
        ctx.zk_f = hub.zk_f;
        hub.side.page = Some(page.unwrap_or("home").into());
        hub.side.draw(&ctx);
        hub.side_sync();
        hub.tip_update(0.0);

        let hinst = GetModuleHandleW(std::ptr::null());
        let cls = gdi::wide("ZenkoRustHub");
        let icon = crate::dialog::make_icon(&init.icon_px);
        let wc = WNDCLASSEXW {
            cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
            style: CS_DBLCLKS,
            lpfnWndProc: Some(proc_),
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: hinst,
            hIcon: icon,
            hCursor: LoadCursorW(std::ptr::null_mut(), IDC_ARROW),
            hbrBackground: std::ptr::null_mut(),
            lpszMenuName: std::ptr::null(),
            lpszClassName: cls.as_ptr(),
            hIconSm: icon,
        };
        RegisterClassExW(&wc);
        let title = gdi::wide("Zenko");
        let hwnd = CreateWindowExW(WS_EX_APPWINDOW | WS_EX_LAYERED, cls.as_ptr(), title.as_ptr(),
                                   WS_POPUP | WS_MINIMIZEBOX | WS_SYSMENU, x0, y0, w, h, std::ptr::null_mut(),
                                   std::ptr::null_mut(), hinst, std::ptr::null());
        SetLayeredWindowAttributes(hwnd, 0, 0, LWA_ALPHA);
        // round_corners: DWMWCP_ROUND, a thin gold border, dark mode
        let put = |attr: u32, v: i32| {
            DwmSetWindowAttribute(hwnd, attr, &v as *const i32 as *const _, 4);
        };
        put(33, 2);
        put(34, gdi::rgb(&zenko_art::palette::mixhex(&init.t.night, &init.t.gold, 0.45)) as i32);
        put(20, 1);
        HUB.with(|v| *v.borrow_mut() = Some(HubW {
            hwnd, hub, fonts, ctx, t0: std::time::Instant::now(), text_scale: init.text_scale, zen_maru, maxed: false,
            restore: (w, h, x0, y0), drag: None, drag_at: (0, 0), drag_moved: false, rs: None, fit_at: None,
            size: (w, h), min, cursor: IDC_ARROW, faded: false,
            anim: zenko_desktop::hubanim::HubAnim::new(0.0, (x0 as u64) << 20 ^ 0x9e37_79b9),
            theme: init.theme.clone(), living: init.living, particles: init.particles, minimized: false,
            slide_s: init.slide_s, dpi: init.dpi, pages: vec![], slide: None, chat: None, chat_seen: None,
            chat_q: Default::default(), typing: None, sb_drag: None,
        }));
        with(|x| {
            let p = x.hub.side.page.clone().unwrap_or_default();
            if let Some(pg) = page_frame(x, &p) {
                place(x, pg, Some(0));
                relayout(x);
            }
            after_show(x, &p);
        });
        ShowWindow(hwnd, SW_SHOW);
        SetForegroundWindow(hwnd);
        SetTimer(hwnd, 1, 45, None);
        out("sound:open");
    }
}

/// the app's news for the sidebar (once a second, like hub_tick and z19_hub_loop): the sky, the status line
pub fn update(ctx: SideCtx, status: &str) {
    with(|x| {
        let sig = |c: &SideCtx| (c.part.clone(), c.season.clone(), c.sky, c.seasons, c.weather.clone(), c.pins.clone(),
                                  c.linked, c.scene.clone(), c.fox_name.clone(), c.catchphrase.clone());
        let redraw = sig(&ctx) != sig(&x.ctx);
        let zk = x.ctx.zk_f;
        x.ctx = SideCtx { zk_f: zk, ..ctx };
        let mut ch = x.hub.set_status(status);
        if redraw {
            let c = x.ctx.clone();
            x.hub.side.draw(&c);
            ch = true;
        }
        if ch {
            x.hub.side_sync();
            unsafe { InvalidateRect(x.hwnd, std::ptr::null(), 0) };
        }
    });
}

pub fn close() {
    let h = HUB.with(|v| v.borrow_mut().take());
    if let Some(x) = h {
        unsafe { DestroyWindow(x.hwnd) };
        crate::win::with_fox(|fox, _| fox.hub_page = None);
        out("closed");
    }
}

/// hub_toggle_max: the work area, or back to where it was
fn toggle_max(x: &mut HubW) {
    unsafe {
        if x.maxed {
            let (w, h, gx, gy) = x.restore;
            SetWindowPos(x.hwnd, std::ptr::null_mut(), gx, gy, w, h, SWP_NOZORDER | SWP_NOACTIVATE);
            x.maxed = false;
        } else {
            let mut r: RECT = std::mem::zeroed();
            GetWindowRect(x.hwnd, &mut r);
            x.restore = (r.right - r.left, r.bottom - r.top, r.left, r.top);
            let mut wa: RECT = std::mem::zeroed();
            SystemParametersInfoW(SPI_GETWORKAREA, 0, &mut wa as *mut RECT as *mut _, 0);
            SetWindowPos(x.hwnd, std::ptr::null_mut(), wa.left, wa.top, wa.right - wa.left, wa.bottom - wa.top,
                         SWP_NOZORDER | SWP_NOACTIVATE);
            x.maxed = true;
        }
    }
}

fn window_rect(hwnd: HWND) -> RECT {
    let mut r: RECT = unsafe { std::mem::zeroed() };
    unsafe { GetWindowRect(hwnd, &mut r) };
    r
}

fn screen_pt() -> (i32, i32) {
    let mut p = POINT { x: 0, y: 0 };
    unsafe { GetCursorPos(&mut p) };
    (p.x, p.y)
}

unsafe fn paint(x: &mut HubW, dc: HDC) {
    unsafe {
        let n = now(x);
        let d = x.hub.ui.draw(&x.fonts, &x.hub.t, n);
        let (w, h) = (x.hub.ui.ws[0].w, x.hub.ui.ws[0].h);
        let mdc = CreateCompatibleDC(dc);
        let bmp = CreateCompatibleBitmap(dc, w, h);
        let ob = SelectObject(mdc, bmp as _);
        let mut run: Vec<Draw> = vec![];
        for item in d {
            if let Draw::Tk { x: ox, y: oy, ops } = item {
                gdi::draw(mdc, &to_ops(&x.fonts, &run));
                run.clear();
                let mut old = POINT { x: 0, y: 0 };
                SetViewportOrgEx(mdc, ox, oy, &mut old);
                crate::tkpaint::paint(mdc, &ops, &x.fonts);
                SetViewportOrgEx(mdc, old.x, old.y, std::ptr::null_mut());
            } else {
                run.push(item);
            }
        }
        gdi::draw(mdc, &to_ops(&x.fonts, &run));
        BitBlt(dc, 0, 0, w, h, mdc, 0, 0, SRCCOPY);
        SelectObject(mdc, ob);
        DeleteObject(bmp as _);
        DeleteDC(mdc);
    }
}

/// one animation step (hub_anim's frame parts at the "smooth" motion level), the fade-in and a pending zk_fit
fn step(x: &mut HubW) -> bool {
    let n = now(x);
    // hub_fade_in: 0.22 s, easing out
    if !x.faded {
        let a = (n / 0.22).min(1.0);
        x.faded = a >= 1.0;
        let al = 1.0 - (1.0 - a) * (1.0 - a);
        unsafe { SetLayeredWindowAttributes(x.hwnd, 0, (al * 255.0).round() as u8, LWA_ALPHA) };
    }
    if let Some(at) = x.fit_at
        && n >= at {
        x.fit_at = None;
        let (w, h) = x.size;
        let f = zk_factor(w, h, x.text_scale, x.zen_maru);
        if (f - x.hub.zk_f).abs() > 1e-12 {
            x.hub.fit_fonts(f);
            x.ctx.zk_f = f;
            let _ = (w, h);
            relayout(x);
            let c = x.ctx.clone();
            x.hub.side.nav(&c);
            x.hub.side_sync();
        }
    }
    let c = x.ctx.clone();
    x.hub.glide(&c);
    // hub_anim: stars, particles and the little fox (she needs the app's fox for her mood and clothes)
    let page = x.hub.side.page.clone();
    let shown = if x.minimized { None } else { page.clone() };
    let living = x.living;
    let anim = &mut x.anim;
    if let Some((lf, pal, t)) = crate::win::with_fox(|fox, t| {
        fox.hub_page = shown;
        (fox.hub_logo(t, page.as_deref(), anim, living), fox.pal.clone(), t)
    }) {
        let th = &x.ctx.t;
        let (style, col) = zenko_desktop::hubanim::particle_style(&x.theme, x.ctx.seasons, x.ctx.weather.as_deref(),
                                                                  &x.ctx.season, &th.gold, &th.glow, &th.acc);
        let (night, ink) = (th.night.clone(), th.ink.clone());
        x.anim.frame(&mut x.hub.side, t, &night, &ink, &style, &col, x.particles, Some(lf), &pal, x.ctx.hd);
    }
    x.hub.side_sync();
    x.hub.tip_update(n);
    // z21_show's slide: ease out over SLIDE_MS
    if let Some(sl) = &x.slide {
        let t = ((n - sl.t0) / x.slide_s.unwrap_or(0.23)).min(1.0);
        let e = 1.0 - (1.0 - t).powi(3);
        let hh = x.hub.ui.ws[x.hub.ids.stage].h;
        let y = (sl.d as f64 * hh as f64 * (1.0 - e)).round() as i32;
        let (pg, op, d) = (sl.pg, sl.op, sl.d);
        if t >= 1.0 {
            finish_slide(x);
        } else {
            place(x, pg, Some(y));
            place(x, op, Some(y - d * hh));
            let (w, h) = x.size;
            x.hub.layout(&x.fonts, w, h);
        }
    }
    chat_step(x);
    true
}

fn resized(x: &mut HubW, w: i32, h: i32) {
    if (w, h) == x.size || w <= 0 || h <= 0 {
        return;
    }
    x.size = (w, h);
    relayout(x);
    let c = x.ctx.clone();
    x.hub.side.draw(&c);
    x.hub.side_sync();
    // zk_resized: the text is re-fitted 170 ms after the size settles
    x.fit_at = Some(now(x) + 0.17);
}

fn hover(x: &mut HubW, mx: i32, my: i32) {
    let part = x.hub.part_at(mx, my);
    let btn = match part {
        "min" => Some("min"),
        "max" => Some("max"),
        "close" => Some("close"),
        _ => None,
    };
    if btn != x.hub.btn_hover {
        x.hub.btn_hover = btn;
        x.hub.buttons_draw();
    }
    let ph = part == "pill";
    if ph != x.hub.pill_hover {
        x.hub.pill_hover = ph;
        x.hub.pill_draw(&x.fonts);
    }
    let key = if part == "side" {
        let (_, ay) = x.hub.ui.abs(x.hub.ids.side_cv);
        x.hub.side.key_at(my - ay, x.ctx.nav_compact)
    } else {
        None
    };
    if key != x.hub.side.hover {
        x.hub.side.hover = key.clone();
        let c = x.ctx.clone();
        x.hub.side.nav(&c);
        x.hub.side_sync();
    }
    let zone = x.hub.zone(mx, my, x.maxed);
    x.cursor = match zone.as_str() {
        "l" | "r" => IDC_SIZEWE,
        "b" => IDC_SIZENS,
        "rb" => IDC_SIZENWSE,
        "lb" => IDC_SIZENESW,
        _ if btn.is_some() || ph || key.is_some() => IDC_HAND,
        _ => IDC_ARROW,
    };
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
                with(|x| paint(x, dc));
                EndPaint(hwnd, &ps);
                0
            }
            WM_ERASEBKGND => 1,
            WM_TIMER => {
                if with(step) == Some(true) {
                    redraw();
                }
                0
            }
            WM_SIZE => {
                let (w, h) = ((lp & 0xffff) as i32, ((lp >> 16) & 0xffff) as i32);
                with(|x| x.minimized = wp as u32 == SIZE_MINIMIZED);
                if wp as u32 != SIZE_MINIMIZED {
                    with(|x| resized(x, w, h));
                    redraw();
                }
                0
            }
            WM_GETMINMAXINFO => {
                let mi = &mut *(lp as *mut MINMAXINFO);
                if let Some((mw, mh)) = with(|x| x.min) {
                    mi.ptMinTrackSize = POINT { x: mw, y: mh };
                }
                0
            }
            WM_SETCURSOR => {
                if (lp & 0xffff) as u32 == HTCLIENT {
                    let c = with(|x| x.cursor).unwrap_or(IDC_ARROW);
                    SetCursor(LoadCursorW(std::ptr::null_mut(), c));
                    return 1;
                }
                DefWindowProcW(hwnd, msg, wp, lp)
            }
            WM_MOUSEMOVE => {
                let (mx, my) = pt();
                let mut tme = TRACKMOUSEEVENT { cbSize: std::mem::size_of::<TRACKMOUSEEVENT>() as u32, dwFlags: TME_LEAVE,
                                                hwndTrack: hwnd, dwHoverTime: 0 };
                TrackMouseEvent(&mut tme);
                let (sx, sy) = screen_pt();
                with(|x| {
                    if let Some((z, x0, y0, w0, h0, gx0, gy0)) = x.rs.clone() {
                        // hub_resize_edges' drag
                        let (dx, dy) = (sx - x0, sy - y0);
                        let (mut w1, mut h1, mut gx1) = (w0, h0, gx0);
                        if z.contains('r') {
                            w1 = (w0 + dx).max(900);
                        }
                        if z.contains('l') {
                            w1 = (w0 - dx).max(900);
                            gx1 = gx0 + (w0 - w1);
                        }
                        if z.contains('b') {
                            h1 = (h0 + dy).max(600);
                        }
                        // wm minsize wins, as Tk's geometry does
                        let (w1, h1) = (w1.max(x.min.0), h1.max(x.min.1));
                        SetWindowPos(x.hwnd, std::ptr::null_mut(), gx1, gy0, w1, h1, SWP_NOZORDER | SWP_NOACTIVATE);
                        return;
                    }
                    if let Some((dx, dy)) = x.drag {
                        if !x.drag_moved {
                            if (sx - x.drag_at.0).abs() + (sy - x.drag_at.1).abs() < 5 {
                                return;
                            }
                            x.drag_moved = true;
                        }
                        SetWindowPos(x.hwnd, std::ptr::null_mut(), sx - dx, (sy - dy).max(0), 0, 0,
                                     SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE);
                        return;
                    }
                    hover(x, mx, my);
                    let n = now(x);
                    x.hub.ui.motion(&x.fonts, mx, my, n);
                    sb_event(x, mx, my, "move");
                    if x.cursor == IDC_ARROW && x.hub.part_at(mx, my) == "page" {
                        x.cursor = match x.hub.ui.cursor_at(mx, my) {
                            zenko_desktop::ui::Cursor::Hand => IDC_HAND,
                            zenko_desktop::ui::Cursor::IBeam => IDC_IBEAM,
                            zenko_desktop::ui::Cursor::Arrow => IDC_ARROW,
                        };
                    }
                    SetCursor(LoadCursorW(std::ptr::null_mut(), x.cursor));
                });
                redraw();
                0
            }
            0x02A3 => {
                // WM_MOUSELEAVE: hub_hover(None) and the buttons go back to their own colour
                with(|x| {
                    if x.rs.is_none() && x.drag.is_none() {
                        hover(x, -100, -100);
                        x.hub.ui.leave();
                    }
                });
                redraw();
                0
            }
            WM_LBUTTONDOWN | WM_LBUTTONDBLCLK => {
                let (mx, my) = pt();
                let (sx, sy) = screen_pt();
                SetCapture(hwnd);
                SetFocus(hwnd);
                with(|x| {
                    let zone = x.hub.zone(mx, my, x.maxed);
                    let part = x.hub.part_at(mx, my);
                    if zone.is_empty() && part == "page" {
                        if !sb_event(x, mx, my, "press") {
                            let n = now(x);
                            x.hub.ui.press(&x.fonts, mx, my, n, msg == WM_LBUTTONDBLCLK);
                        }
                        return;
                    }
                    if msg == WM_LBUTTONDBLCLK && part == "bar" {
                        toggle_max(x);
                        return;
                    }
                    if !zone.is_empty() {
                        let r = window_rect(x.hwnd);
                        x.rs = Some((zone, sx, sy, r.right - r.left, r.bottom - r.top, r.left, r.top));
                        x.drag = None;
                        return;
                    }
                    // hub_make_drag: the sidebar, the title bar and its name, and the shrine frame move the window
                    if matches!(part, "side" | "bar" | "title" | "border") {
                        if x.maxed {
                            let (gw, gh, _, _) = x.restore;
                            x.maxed = false;
                            SetWindowPos(x.hwnd, std::ptr::null_mut(), sx - gw / 2, (sy - 15).max(0), gw, gh,
                                         SWP_NOZORDER | SWP_NOACTIVATE);
                        }
                        let r = window_rect(x.hwnd);
                        x.drag = Some((sx - r.left, sy - r.top));
                        x.drag_moved = false;
                        x.drag_at = (sx, sy);
                    }
                });
                0
            }
            WM_LBUTTONUP => {
                ReleaseCapture();
                let (mx, my) = pt();
                let mut act: Option<&'static str> = None;
                with(|x| {
                    sb_event(x, mx, my, "release");
                    if let Some(cmd) = x.hub.ui.release(mx, my) {
                        out("sound:click");
                        chat_cmd(x, &cmd);
                    }
                    let moved = x.drag.is_some() && x.drag_moved;
                    x.rs = None;
                    x.drag = None;
                    let part = x.hub.part_at(mx, my);
                    match part {
                        "min" | "max" | "close" => act = Some(part),
                        "side" if !moved => {
                            let (_, ay) = x.hub.ui.abs(x.hub.ids.side_cv);
                            if let Some(k) = x.hub.side.key_at(my - ay, x.ctx.nav_compact) {
                                if x.hub.side.page.as_deref() != Some(k.as_str()) {
                                    out("sound:nav");
                                }
                                show(x, &k);
                            }
                        }
                        _ => {}
                    }
                    x.drag_moved = false;
                });
                match act {
                    Some("min") => {
                        ShowWindow(hwnd, SW_MINIMIZE);
                    }
                    Some("max") => {
                        with(toggle_max);
                    }
                    Some("close") => close(),
                    _ => {}
                }
                redraw();
                0
            }
            WM_CLOSE => {
                close();
                0
            }
            WM_ACTIVATE => {
                let on = (wp & 0xffff) != 0;
                with(|x| x.hub.ui.active = on);
                redraw();
                0
            }
            WM_MOUSEWHEEL => {
                let delta = ((wp >> 16) & 0xffff) as i16 as i32;
                let mut p = POINT { x: (lp & 0xffff) as i16 as i32, y: ((lp >> 16) & 0xffff) as i16 as i32 };
                ScreenToClient(hwnd, &mut p);
                if with(|x| chat_wheel(x, p.x, p.y, delta)) == Some(true) {
                    redraw();
                }
                0
            }
            WM_KEYDOWN => {
                let vk = wp as u16;
                let shift = GetKeyState(VK_SHIFT as i32) < 0;
                let ctrl = GetKeyState(VK_CONTROL as i32) < 0;
                with(|x| {
                    let n = now(x);
                    let entry = x.chat.as_ref().map(|c| c.entry);
                    if vk == VK_RETURN && entry.is_some() && x.hub.ui.focus == entry {
                        chat_cmd(x, "chat_send");
                    } else if ctrl && vk == b'A' as u16 {
                        x.hub.ui.select_all(&x.fonts);
                    } else if ctrl && (vk == b'C' as u16 || vk == b'X' as u16) {
                        if let Some(s) = x.hub.ui.selected() {
                            crate::dialog::clipboard_set(&s);
                            if vk == b'X' as u16 {
                                x.hub.ui.insert(&x.fonts, &[], n);
                            }
                        }
                    } else if ctrl && vk == b'V' as u16 {
                        if let Some(s) = crate::dialog::clipboard_get() {
                            let s: Vec<u16> = s.into_iter().filter(|&c| c != 13 && c != 10).collect();
                            x.hub.ui.insert(&x.fonts, &s, n);
                        }
                    } else {
                        x.hub.ui.key(&x.fonts, vk, shift, ctrl, n);
                    }
                });
                redraw();
                0
            }
            WM_CHAR => {
                let c = wp as u16;
                if c >= 0x20 && c != 0x7f && GetKeyState(VK_CONTROL as i32) >= 0 {
                    with(|x| {
                        let n = now(x);
                        x.hub.ui.insert(&x.fonts, &[c], n);
                    });
                    redraw();
                }
                0
            }
            _ => DefWindowProcW(hwnd, msg, wp, lp),
        }
    }
}
