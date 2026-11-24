// Zenko Browser: a small Rust shell around WebView2 (the Edge engine that is already on Windows).
// The browser frame (tabs, toolbar, command bar, animations) is a web page drawn by Chromium's GPU
// compositor, so it animates at the screen's full refresh rate. Each tab, the Claude panel and the
// frame are separate WebView2 views inside one window.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod bridge;
mod edge;
mod guard;
mod history;
mod updater;
#[cfg(windows)]
mod win;

use std::{
    borrow::Cow,
    cell::RefCell,
    collections::{HashMap, HashSet},
    path::PathBuf,
    rc::Rc,
    time::{Duration, Instant},
};

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tao::{
    dpi::{LogicalPosition, LogicalSize},
    event::{Event, StartCause, WindowEvent},
    event_loop::{ControlFlow, EventLoopBuilder, EventLoopProxy, EventLoopWindowTarget},
    window::{ResizeDirection, Window, WindowBuilder, WindowId},
};
use wry::{
    http::{header::CONTENT_TYPE, Request, Response},
    NewWindowResponse, PageLoadEvent, Rect, WebContext, WebView, WebViewBuilder,
};
#[cfg(windows)]
use wry::{MemoryUsageLevel, Theme, WebViewBuilderExtWindows, WebViewExtWindows};

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
const PANEL_ID: u32 = 0;
const CLAUDE_URL: &str = "https://claude.ai/new";
const NEWTAB: &str = "zenko://newtab";
const PEEK_ID: u32 = u32::MAX - 1;
/// Browser start-up switches. Every view in the window must use exactly the same ones.
static ARGS: std::sync::OnceLock<String> = std::sync::OnceLock::new();
fn browser_args() -> String {
    ARGS.get().cloned().unwrap_or_else(|| "--disable-features=msWebOOUI,msPdfOOUI,msSmartScreenProtection".into())
}
fn unix_now() -> u64 {
    history::now()
}

#[derive(Debug)]
pub enum Ev {
    Ui(String),
    Page(u32, String, String),
    Title(u32, String),
    Load(u32, bool, String),
    Open(u32, String),
    Key(String),
    Blocked(u32, String),
    Crashed(u32),
    Menu(u32, String, String),
    Redirect(u32, String),
    Snapshot(u32, String),
    DlStart(String, String),
    DlDone(String, String, bool),
    Focus(u32),
    Shot(String, String),
    Audio(u32, bool),
    Fullscreen(u32, bool),
    Update(Value),
}

struct Tab {
    id: u32,
    wv: WebView,
    url: String,
    title: String,
    icon: String,
    loading: bool,
    audio: bool,
    muted: bool,
    hidden_at: Option<Instant>,
    level: u8, // 0 awake, 1 low memory, 2 asleep
    /// New tab still behind the closed shoji door (page hidden until it has loaded, so no white flash).
    door: Option<Instant>,
    pinned: bool,
    space: String,
    group: String,
    parent: u32,
    last_used: u64,
    profile: String,
    blocked: u32,
    blocked_hosts: Vec<String>,
    text: String,
    scroll: f64,
    restore_scroll: f64,
}

#[derive(Clone, Copy, Default)]
struct Rc4 {
    x: f64,
    y: f64,
    w: f64,
    h: f64,
}

fn rect(r: Rc4) -> Rect {
    Rect {
        position: LogicalPosition::new(r.x, r.y).into(),
        size: LogicalSize::new(r.w.max(1.0), r.h.max(1.0)).into(),
    }
}

#[derive(Serialize, Deserialize, Default)]
struct Session {
    tabs: Vec<SavedTab>,
    active: usize,
    panel: bool,
    #[serde(default)]
    closed: Vec<String>,
    #[serde(default)]
    space: String,
    #[serde(default)]
    archived: Vec<SavedTab>,
}
#[derive(Serialize, Deserialize, Clone)]
struct SavedTab {
    url: String,
    title: String,
    #[serde(default)]
    pinned: bool,
    #[serde(default)]
    space: String,
    #[serde(default)]
    group: String,
    #[serde(default)]
    profile: String,
    #[serde(default)]
    last_used: u64,
    #[serde(default)]
    scroll: f64,
}

struct App {
    window: Window,
    chrome: WebView,
    ctx: WebContext,
    proxy: EventLoopProxy<Ev>,
    keys: Rc<RefCell<HashSet<String>>>,
    tabs: Vec<Tab>,
    active: u32,
    next_id: u32,
    panel: Option<WebView>,
    panel_title: String,
    panel_open: bool,
    content: Rc4,
    panel_rect: Rc4,
    focused: u32,
    overlay: bool,
    shots_wanted: usize,
    closed: Vec<String>,
    popouts: Vec<(Window, WebView)>,
    data_dir: PathBuf,
    dirty: bool,
    last_save: Instant,
    last_title: String,
    sleep_after: u64,
    fullscreen: bool,
    hist: std::collections::HashMap<String, history::Entry>,
    hist_dirty: bool,
    guard: Rc<RefCell<guard::Guard>>,
    space: String,
    split: Option<(u32, u32)>,
    peek: Option<WebView>,
    rp: Value,
    downloads: Vec<Value>,
    dl_subject: Rc<RefCell<(bool, String)>>,
    time: HashMap<String, u64>,
    day: String,
    archived: Vec<SavedTab>,
    panel_mode: String,
    battery: bool,
    win_focused: bool,
    ext_tried: bool,
    focus_pause: Option<Instant>,
}

// ---------------------------------------------------------------- built-in pages (zenko://...)

fn serve(path: &str) -> Response<Cow<'static, [u8]>> {
    let (body, mime): (&'static [u8], &str) = match path.trim_start_matches('/') {
        "" | "chrome" | "chrome.html" => (include_bytes!("../ui/chrome.html"), "text/html; charset=utf-8"),
        "newtab" | "newtab.html" => (include_bytes!("../ui/newtab.html"), "text/html; charset=utf-8"),
        "settings" | "settings.html" => (include_bytes!("../ui/settings.html"), "text/html; charset=utf-8"),
        "blocked" | "blocked.html" => (include_bytes!("../ui/blocked.html"), "text/html; charset=utf-8"),
        "theme.css" => (include_bytes!("../ui/theme.css"), "text/css; charset=utf-8"),
        "prefs.js" => (include_bytes!("../ui/prefs.js"), "text/javascript; charset=utf-8"),
        "zenko.png" => (include_bytes!("../ui/zenko.png"), "image/png"),
        "zenko_sleep.png" => (include_bytes!("../ui/zenko_sleep.png"), "image/png"),
        "fonts/ZenMaruGothic-Regular.ttf" => (include_bytes!("../ui/fonts/ZenMaruGothic-Regular.ttf"), "font/ttf"),
        "fonts/ZenMaruGothic-Bold.ttf" => (include_bytes!("../ui/fonts/ZenMaruGothic-Bold.ttf"), "font/ttf"),
        "fonts/Outfit.ttf" => (include_bytes!("../ui/fonts/Outfit.ttf"), "font/ttf"),
        "fonts/PixelifySans.ttf" => (include_bytes!("../ui/fonts/PixelifySans.ttf"), "font/ttf"),
        "fonts/DotGothic16.ttf" => (include_bytes!("../ui/fonts/DotGothic16.ttf"), "font/ttf"),
        "fonts/ZenOldMincho-Bold.ttf" => (include_bytes!("../ui/fonts/ZenOldMincho-Bold.ttf"), "font/ttf"),
        _ => (b"Not found", "text/plain"),
    };
    Response::builder()
        .header(CONTENT_TYPE, mime)
        .body(Cow::Borrowed(body))
        .unwrap()
}

fn protocol(_id: wry::WebViewId, req: Request<Vec<u8>>) -> Response<Cow<'static, [u8]>> {
    serve(req.uri().path())
}

/// zenko://newtab -> the address WebView2 actually uses for our custom scheme.
fn real_url(u: &str) -> String {
    if let Some(rest) = u.strip_prefix("zenko://") {
        format!("http://zenko.localhost/{}", rest)
    } else {
        u.to_string()
    }
}
/// The other way round, so the address bar shows zenko://newtab.
fn nice_url(u: &str) -> String {
    for p in ["http://zenko.localhost/", "https://zenko.localhost/"] {
        if let Some(rest) = u.strip_prefix(p) {
            return format!("zenko://{}", rest.trim_end_matches(".html"));
        }
    }
    u.to_string()
}
fn is_ours(u: &str) -> bool {
    u.starts_with("http://zenko.localhost/") || u.starts_with("https://zenko.localhost/")
}

/// Sign-in pop-ups must stay real pop-ups (the site waits for them to report back).
fn keep_as_popup(u: &str) -> bool {
    let l = u.to_ascii_lowercase();
    [
        "accounts.google.",
        "login.microsoftonline.",
        "login.live.com",
        "appleid.apple.com",
        "github.com/login",
        "oauth",
        "/auth",
        "signin",
        "sso",
        "about:blank",
    ]
    .iter()
    .any(|k| l.contains(k))
}

const CONTENT_JS: &str = include_str!("../ui/content.js");
const PANEL_JS: &str = include_str!("../ui/panel.js");

impl App {
    fn data_file(&self, name: &str) -> PathBuf {
        self.data_dir.join(name)
    }

    fn build_page(&mut self, id: u32, url: &str, bounds: Rc4, visible: bool, profile: &str) -> Option<WebView> {
        let try_ext = !self.ext_tried && id != PANEL_ID && id != PEEK_ID;
        if try_ext {
            self.ext_tried = true;
            let dir = self.data_file("extensions");
            let has = std::fs::read_dir(&dir).map(|d| d.flatten().any(|e| e.path().is_dir())).unwrap_or(false);
            if has {
                if let Some(w) = self.build_page_with(id, url, bounds, visible, profile, Some(dir)) {
                    return Some(w);
                }
            }
        }
        self.build_page_with(id, url, bounds, visible, profile, None)
    }

    fn build_page_with(&mut self, id: u32, url: &str, bounds: Rc4, visible: bool, profile: &str, ext: Option<PathBuf>) -> Option<WebView> {
        let p1 = self.proxy.clone();
        let p2 = self.proxy.clone();
        let p3 = self.proxy.clone();
        let p4 = self.proxy.clone();
        let p5 = self.proxy.clone();
        let p6 = self.proxy.clone();
        let subj = self.dl_subject.clone();
        let is_panel = id == PANEL_ID;
        let builder = WebViewBuilder::new_with_web_context(&mut self.ctx)
            .with_url(real_url(url))
            .with_bounds(rect(bounds))
            .with_visible(visible)
            .with_devtools(true)
            .with_back_forward_navigation_gestures(true)
            .with_general_autofill_enabled(true)
            .with_clipboard(true)
            .with_custom_protocol("zenko".into(), protocol)
            .with_initialization_script(if is_panel { PANEL_JS } else { CONTENT_JS })
            .with_ipc_handler(move |req: Request<String>| {
                let src = req.uri().to_string();
                let _ = p1.send_event(Ev::Page(id, src, req.into_body()));
            })
            .with_document_title_changed_handler(move |t| {
                let _ = p2.send_event(Ev::Title(id, t));
            })
            .with_on_page_load_handler(move |e, u| {
                let _ = p3.send_event(Ev::Load(id, matches!(e, PageLoadEvent::Started), u));
            })
            .with_new_window_req_handler(move |u, f| {
                if f.size.is_some() || keep_as_popup(&u) {
                    NewWindowResponse::Allow
                } else {
                    let _ = p4.send_event(Ev::Open(id, u));
                    NewWindowResponse::Deny
                }
            })
            .with_download_started_handler(move |u, path| {
                let (on, sub) = subj.borrow().clone();
                if on && !sub.is_empty() {
                    if let (Some(dir), Some(name)) = (path.parent().map(|p| p.to_path_buf()), path.file_name().map(|n| n.to_owned())) {
                        let to = dir.join("Zenko").join(&sub);
                        if std::fs::create_dir_all(&to).is_ok() {
                            *path = to.join(name);
                        }
                    }
                }
                let _ = p5.send_event(Ev::DlStart(u, path.to_string_lossy().to_string()));
                true
            })
            .with_download_completed_handler(move |u, path, ok| {
                let _ = p6.send_event(Ev::DlDone(u, path.map(|p| p.to_string_lossy().to_string()).unwrap_or_default(), ok));
            });
        #[cfg(windows)]
        let builder = {
            let mut b = builder
                .with_browser_accelerator_keys(true)
                .with_additional_browser_args(browser_args())
                .with_browser_extensions_enabled(true)
                .with_theme(if is_panel { Theme::Dark } else { Theme::Auto });
            if !profile.is_empty() {
                b = b.with_profile_name(profile);
            }
            if let Some(e) = ext {
                b = b.with_extensions_path(e);
            }
            b
        };
        #[cfg(not(windows))]
        let _ = (profile, ext);
        match builder.build_as_child(&self.window) {
            Ok(wv) => {
                #[cfg(windows)]
                {
                    win::hook(&wv, id, self.proxy.clone(), self.keys.clone());
                    win::extras(&wv, id, self.proxy.clone(), self.guard.clone());
                }
                Some(wv)
            }
            Err(e) => {
                eprintln!("could not open a page: {e}");
                None
            }
        }
    }

    fn new_tab(&mut self, url: &str, focus: bool) {
        let profile = self.active_tab().map(|t| t.profile.clone()).unwrap_or_default();
        self.new_tab_in(url, focus, &profile, 0);
    }

    fn new_tab_in(&mut self, url: &str, focus: bool, profile: &str, parent: u32) -> u32 {
        let id = self.next_id;
        self.next_id += 1;
        let Some(wv) = self.build_page(id, url, self.content, false, profile) else { return 0 };
        let tab = Tab {
            id,
            wv,
            url: url.to_string(),
            title: if url == NEWTAB { "New tab".into() } else { url.to_string() },
            icon: String::new(),
            loading: true,
            audio: false,
            muted: false,
            hidden_at: Some(Instant::now()),
            level: 0,
            door: if focus { Some(Instant::now()) } else { None },
            pinned: false,
            space: self.space.clone(),
            group: String::new(),
            parent,
            last_used: unix_now(),
            profile: profile.to_string(),
            blocked: 0,
            blocked_hosts: vec![],
            text: String::new(),
            scroll: 0.0,
            restore_scroll: 0.0,
        };
        // new tabs open right after the current one (after its children when opened from a page)
        let mut at = self.index(self.active).map(|i| i + 1).unwrap_or(self.tabs.len());
        if parent != 0 {
            if let Some(pi) = self.index(parent) {
                at = pi + 1;
                while at < self.tabs.len() && self.tabs[at].parent == parent {
                    at += 1;
                }
            }
        }
        self.tabs.insert(at.min(self.tabs.len()), tab);
        if focus || self.tabs.len() == 1 {
            self.activate(id);
        }
        self.changed();
        id
    }

    fn index(&self, id: u32) -> Option<usize> {
        self.tabs.iter().position(|t| t.id == id)
    }

    /// Which tabs are on screen right now (two in split view).
    fn shown_ids(&self) -> Vec<u32> {
        if self.fullscreen {
            return vec![self.active];
        }
        match self.split {
            Some((a, b)) => vec![a, b],
            None => vec![self.active],
        }
    }

    fn rect_for(&self, id: u32) -> Rc4 {
        if self.fullscreen && id == self.active {
            return self.full_rect();
        }
        let c = self.content;
        match self.split {
            Some((a, b)) if id == a || id == b => {
                let gap = 8.0;
                let w = ((c.w - gap) / 2.0).max(1.0);
                if id == a {
                    Rc4 { x: c.x, y: c.y, w, h: c.h }
                } else {
                    Rc4 { x: c.x + w + gap, y: c.y, w, h: c.h }
                }
            }
            _ => c,
        }
    }

    /// Put every page where it belongs: shown tabs visible in their place, the rest hidden.
    fn place(&mut self) {
        let shown = self.shown_ids();
        let rects: Vec<(u32, Rc4)> = shown.iter().map(|&i| (i, self.rect_for(i))).collect();
        let overlay = self.overlay;
        for t in self.tabs.iter_mut() {
            if let Some((_, r)) = rects.iter().find(|(i, _)| *i == t.id) {
                let _ = t.wv.set_bounds(rect(*r));
                let _ = t.wv.set_visible(!overlay && t.door.is_none());
                #[cfg(windows)]
                if t.level > 0 {
                    let _ = t.wv.set_memory_usage_level(MemoryUsageLevel::Normal);
                }
                t.level = 0;
                t.hidden_at = None;
            } else if t.hidden_at.is_none() {
                let _ = t.wv.set_visible(false);
                t.hidden_at = Some(Instant::now());
            }
        }
        if let Some(p) = &self.peek {
            let c = self.content;
            let _ = p.set_bounds(rect(Rc4 { x: c.x + c.w * 0.08, y: c.y + c.h * 0.05, w: c.w * 0.84, h: c.h * 0.9 }));
            let _ = p.set_visible(!overlay);
        }
    }

    fn activate(&mut self, id: u32) {
        let Some(i) = self.index(id) else { return };
        if self.fullscreen && id != self.active {
            let a = self.active;
            self.set_fullscreen(a, false);
        }
        if let Some((a, b)) = self.split {
            if id != a && id != b {
                self.split = None;
            }
        }
        if self.tabs[i].space != self.space {
            self.space = self.tabs[i].space.clone();
        }
        self.active = id;
        self.tabs[i].last_used = unix_now();
        self.place();
        if !self.overlay {
            if let Some(i) = self.index(id) {
                let _ = self.tabs[i].wv.focus();
            }
        }
        self.focused = id;
        self.changed();
    }

    fn close_tab(&mut self, id: u32) {
        self.close_tab_as(id, true)
    }

    fn close_tab_as(&mut self, id: u32, remember: bool) {
        let Some(i) = self.index(id) else { return };
        if let Some((a, b)) = self.split {
            if id == a || id == b {
                self.split = None;
                let other = if id == a { b } else { a };
                if self.active == id {
                    self.active = other;
                }
            }
        }
        let t = self.tabs.remove(i);
        for c in self.tabs.iter_mut() {
            if c.parent == id {
                c.parent = t.parent;
            }
        }
        if remember && t.url != NEWTAB {
            self.closed.push(t.url.clone());
            if self.closed.len() > 30 {
                self.closed.remove(0);
            }
        }
        drop(t);
        if self.tabs.is_empty() {
            self.new_tab(NEWTAB, true);
            return;
        }
        if self.active == id || self.index(self.active).is_none() {
            // stay in the same space when we can
            let n = self.tabs.iter().skip(i.min(self.tabs.len() - 1)).chain(self.tabs.iter().rev()).find(|t| t.space == self.space).map(|t| t.id);
            let n = n.unwrap_or(self.tabs[i.min(self.tabs.len() - 1)].id);
            self.activate(n);
        } else {
            self.place();
        }
        self.changed();
    }

    fn active_tab(&self) -> Option<&Tab> {
        self.index(self.active).map(|i| &self.tabs[i])
    }

    fn ui(&self, js: &str) {
        let _ = self.chrome.evaluate_script(js);
    }

    fn changed(&mut self) {
        self.dirty = true;
        let tabs: Vec<Value> = self
            .tabs
            .iter()
            .map(|t| {
                json!({"id": t.id, "title": t.title, "url": nice_url(&t.url), "icon": t.icon,
                       "loading": t.loading, "audio": t.audio, "muted": t.muted, "asleep": t.level >= 2, "door": t.door.is_some(), "pinned": t.pinned,
                       "space": t.space, "group": t.group, "subject": guard::subject(&t.title, &t.url), "parent": t.parent,
                       "profile": t.profile, "blocked": t.blocked})
            })
            .collect();
        let (back, fwd) = self
            .active_tab()
            .map(|t| (t.wv.can_go_back().unwrap_or(false), t.wv.can_go_forward().unwrap_or(false)))
            .unwrap_or((false, false));
        let split = self.split.map(|(a, b)| json!([a, b])).unwrap_or(Value::Null);
        let (blocked, bhosts) = self.active_tab().map(|t| (t.blocked, t.blocked_hosts.clone())).unwrap_or_default();
        let g = self.guard.borrow();
        let site = self.active_tab().map(|t| guard::host(&t.url)).unwrap_or_default();
        let ads_off = g.ads_off.iter().any(|x| guard::host_matches(&site, x));
        let focus = g.focus && self.focus_pause.is_none();
        drop(g);
        if let Some(t) = self.active_tab() {
            let sub = guard::subject(&t.title, &t.url).to_string();
            self.dl_subject.borrow_mut().1 = sub;
        }
        let st = json!({"tabs": tabs, "active": self.active, "panel": self.panel_open, "panelMode": self.panel_mode,
                        "back": back, "fwd": fwd, "closed": self.closed.len(), "version": VERSION,
                        "max": self.window.is_maximized(), "space": self.space, "split": split, "peek": self.peek.is_some(),
                        "blocked": blocked, "blockedHosts": bhosts, "adsOff": ads_off, "focus": focus,
                        "archived": self.archived.len(), "battery": self.battery});
        self.ui(&format!("zb.state({})", st));
        self.tell_zenko();
    }

    /// Window title + bridge file, so Zenko knows what you are reading (and which subject it is).
    fn tell_zenko(&mut self) {
        let (title, url) = if self.focused == PANEL_ID && self.panel_open {
            (self.panel_title.clone(), self.panel.as_ref().and_then(|p| p.url().ok()).unwrap_or_default())
        } else {
            self.active_tab().map(|t| (t.title.clone(), t.url.clone())).unwrap_or_default()
        };
        let host = bridge::host_of(&url);
        let shown = if host.is_empty() || is_ours(&url) {
            format!("{} - Zenko Browser", if title.is_empty() { "New tab" } else { &title })
        } else {
            format!("{} - {} - Zenko Browser", title, host)
        };
        if shown != self.last_title {
            self.window.set_title(&shown);
            self.last_title = shown;
            bridge::write_now(&title, &nice_url(&url), &host, self.focused == PANEL_ID);
        }
    }

    fn layout(&mut self, v: &Value) {
        let g = |k: &str| v.get(k).and_then(|x| x.as_f64()).unwrap_or(0.0);
        self.content = Rc4 { x: g("cx"), y: g("cy"), w: g("cw"), h: g("ch") };
        self.panel_rect = Rc4 { x: g("px"), y: g("py"), w: g("pw"), h: g("ph") };
        if self.fullscreen {
            return;
        }
        self.place();
        if let Some(p) = &self.panel {
            let _ = p.set_bounds(rect(self.panel_rect));
        }
    }

    fn set_panel(&mut self, open: bool) {
        self.panel_open = open;
        if self.panel_mode != "claude" {
            if let Some(p) = &self.panel {
                let _ = p.set_visible(false);
            }
            if !open {
                self.focused = self.active;
            }
            self.changed();
            return;
        }
        if open && self.panel.is_none() {
            self.panel = self.build_page(PANEL_ID, CLAUDE_URL, self.panel_rect, !self.overlay, "");
        }
        if let Some(p) = &self.panel {
            let _ = p.set_visible(open && !self.overlay);
            if open {
                let _ = p.focus();
                self.focused = PANEL_ID;
            }
        }
        if !open {
            self.focused = self.active;
        }
        self.changed();
    }

    fn popout(&mut self, target: &EventLoopWindowTarget<Ev>) {
        let url = self
            .panel
            .as_ref()
            .and_then(|p| p.url().ok())
            .filter(|u| u.starts_with("https://claude.ai"))
            .unwrap_or_else(|| CLAUDE_URL.to_string());
        let Ok(w) = WindowBuilder::new()
            .with_title("Claude - Zenko Browser")
            .with_inner_size(LogicalSize::new(560.0, 820.0))
            .with_window_icon(win_icon())
            .build(target)
        else {
            return;
        };
        let b = WebViewBuilder::new_with_web_context(&mut self.ctx)
            .with_url(url)
            .with_devtools(true)
            .with_initialization_script(PANEL_JS);
        #[cfg(windows)]
        let b = b.with_theme(Theme::Dark).with_additional_browser_args(browser_args()).with_browser_extensions_enabled(true);
        if let Ok(wv) = b.build(&w) {
            self.popouts.push((w, wv));
        }
        self.panel = None; // the pop-out takes over; a fresh panel opens next time
        self.set_panel(false);
    }

    /// Command bar / menus are drawn by the frame page. The pages are separate windows, so we take a
    /// picture of them, hide them, and the frame shows the picture (dimmed) under the command bar.
    fn set_overlay(&mut self, on: bool) {
        if on == self.overlay {
            return;
        }
        self.overlay = on;
        if on {
            self.shots_wanted = 0;
            #[cfg(windows)]
            {
                for id in self.shown_ids() {
                    let r = self.rect_for(id);
                    if let Some(i) = self.index(id) {
                        if self.tabs[i].door.is_none() {
                            win::capture(&self.tabs[i].wv, &format!("page|{}|{}|{}|{}", r.x, r.y, r.w, r.h), self.proxy.clone());
                            self.shots_wanted += 1;
                        }
                    }
                }
                if let Some(p) = &self.peek {
                    let c = self.content;
                    let r = Rc4 { x: c.x + c.w * 0.08, y: c.y + c.h * 0.05, w: c.w * 0.84, h: c.h * 0.9 };
                    win::capture(p, &format!("peek|{}|{}|{}|{}", r.x, r.y, r.w, r.h), self.proxy.clone());
                    self.shots_wanted += 1;
                }
                if self.panel_open && self.panel_mode == "claude" {
                    if let Some(p) = &self.panel {
                        win::capture(p, "panel", self.proxy.clone());
                        self.shots_wanted += 1;
                    }
                }
            }
            if self.shots_wanted == 0 {
                self.hide_pages();
            }
        } else {
            self.place();
            if self.panel_open && self.panel_mode == "claude" {
                if let Some(p) = &self.panel {
                    let _ = p.set_visible(true);
                }
            }
            let f = self.focused;
            if f == PANEL_ID && self.panel_open {
                if let Some(p) = &self.panel {
                    let _ = p.focus();
                }
            } else if let Some(t) = self.active_tab() {
                let _ = t.wv.focus();
            }
        }
    }

    fn full_rect(&self) -> Rc4 {
        let l = self.window.inner_size().to_logical::<f64>(self.window.scale_factor());
        Rc4 { x: 0.0, y: 0.0, w: l.width, h: l.height }
    }

    fn set_fullscreen(&mut self, id: u32, on: bool) {
        if id != self.active && on {
            return;
        }
        self.fullscreen = on;
        self.window.set_fullscreen(if on { Some(tao::window::Fullscreen::Borderless(None)) } else { None });
        self.place();
        if let Some(p) = &self.panel {
            let _ = p.set_visible(!on && self.panel_open && self.panel_mode == "claude");
        }
    }

    fn hide_pages(&self) {
        if !self.overlay {
            return;
        }
        for id in self.shown_ids() {
            if let Some(i) = self.index(id) {
                let _ = self.tabs[i].wv.set_visible(false);
            }
        }
        if let Some(p) = &self.peek {
            let _ = p.set_visible(false);
        }
        if let Some(p) = &self.panel {
            let _ = p.set_visible(false);
        }
        let _ = self.chrome.focus();
    }

    fn save(&mut self) {
        let s = Session {
            tabs: self
                .tabs
                .iter()
                .map(|t| SavedTab {
                    url: nice_url(&t.url),
                    title: t.title.clone(),
                    pinned: t.pinned,
                    space: t.space.clone(),
                    group: t.group.clone(),
                    profile: t.profile.clone(),
                    last_used: t.last_used,
                    scroll: t.scroll,
                })
                .collect(),
            active: self.index(self.active).unwrap_or(0),
            panel: self.panel_open,
            closed: self.closed.clone(),
            space: self.space.clone(),
            archived: self.archived.clone(),
        };
        if !self.day.is_empty() {
            let _ = std::fs::write(self.data_file("time.json"), json!({"day": self.day, "hosts": self.time}).to_string());
        }
        if let Ok(txt) = serde_json::to_string_pretty(&s) {
            let _ = std::fs::create_dir_all(&self.data_dir);
            let tmp = self.data_file("session.tmp");
            if std::fs::write(&tmp, txt).is_ok() {
                let _ = std::fs::rename(&tmp, self.data_file("session.json"));
            }
        }
        if self.hist_dirty {
            history::save(&self.data_file("history.json"), &self.hist);
            self.hist_dirty = false;
        }
        self.dirty = false;
        self.last_save = Instant::now();
    }

    /// Tabs you have not looked at for a while use less memory, then fall asleep (they wake when shown).
    fn sleep_tick(&mut self) {
        if self.sleep_after == 0 {
            return;
        }
        let sleep_after = if self.battery { self.sleep_after.min(5) } else { self.sleep_after };
        let mut changed = false;
        for t in self.tabs.iter_mut() {
            let Some(h) = t.hidden_at else { continue };
            let secs = h.elapsed().as_secs();
            if t.audio {
                continue;
            }
            #[cfg(windows)]
            {
                if t.level == 0 && secs > 120 {
                    let _ = t.wv.set_memory_usage_level(MemoryUsageLevel::Low);
                    t.level = 1;
                }
                if t.level == 1 && secs > sleep_after * 60 {
                    win::suspend(&t.wv);
                    t.level = 2;
                    changed = true;
                }
            }
            #[cfg(not(windows))]
            let _ = secs;
        }
        if changed {
            self.changed();
        }
    }

    /// Tabs nobody has opened for a while move to "Archived tabs" (Ctrl+K finds them again).
    fn archive_tick(&mut self) {
        let days = self.rp["archiveDays"].as_u64().unwrap_or(7);
        if days == 0 {
            return;
        }
        let now = unix_now();
        let shown = self.shown_ids();
        let old: Vec<u32> = self
            .tabs
            .iter()
            .filter(|t| !t.pinned && !t.audio && !shown.contains(&t.id) && t.last_used > 0 && now.saturating_sub(t.last_used) > days * 86400)
            .map(|t| t.id)
            .collect();
        for id in old {
            if let Some(i) = self.index(id) {
                let t = &self.tabs[i];
                if t.url != NEWTAB && !is_ours(&t.url) {
                    self.archived.insert(
                        0,
                        SavedTab {
                            url: nice_url(&t.url),
                            title: t.title.clone(),
                            pinned: false,
                            space: t.space.clone(),
                            group: t.group.clone(),
                            profile: t.profile.clone(),
                            last_used: now,
                            scroll: t.scroll,
                        },
                    );
                }
                self.close_tab_as(id, false);
            }
        }
        self.archived.truncate(200);
    }

    /// Time on each site today (only while this window is in front).
    fn time_tick(&mut self) {
        if !self.win_focused || self.day.is_empty() {
            return;
        }
        let h = self.active_tab().map(|t| guard::host(&t.url)).unwrap_or_default();
        if !h.is_empty() {
            *self.time.entry(h.trim_start_matches("www.").to_string()).or_insert(0) += 1;
        }
    }

    fn recap(&self) -> Value {
        let mut v: Vec<(&String, &u64)> = self.time.iter().collect();
        v.sort_by(|a, b| b.1.cmp(a.1));
        Value::Array(v.into_iter().take(8).map(|(h, s)| json!({"h": h, "s": s})).collect())
    }

    /// Settings that the Rust side needs (sent by the frame whenever they change).
    fn apply_prefs(&mut self, v: &Value) {
        self.rp = v.clone();
        let list = |k: &str| -> Vec<String> {
            v[k].as_array().map(|a| a.iter().filter_map(|x| x.as_str()).map(|x| x.trim().to_lowercase()).filter(|x| !x.is_empty()).collect()).unwrap_or_default()
        };
        {
            let mut g = self.guard.borrow_mut();
            g.ads = v["ads"].as_bool().unwrap_or(true);
            g.ads_off = list("adsOff").into_iter().collect();
            g.clean = v["clean"].as_bool().unwrap_or(true);
            g.focus = v["focus"].as_bool().unwrap_or(false);
            g.focus_block = list("focusBlock");
            g.focus_only = if v["focusOnlyMode"].as_bool().unwrap_or(false) { list("focusOnly") } else { vec![] };
            g.focus_allow = list("focusAllow");
        }
        self.dl_subject.borrow_mut().0 = v["sortDownloads"].as_bool().unwrap_or(true);
        self.sleep_after = v["sleepAfter"].as_u64().unwrap_or(20);
        if let Some(d) = v["day"].as_str() {
            self.new_day(d);
        }
        let _ = std::fs::write(self.data_file("rprefs.json"), v.to_string());
    }

    fn new_day(&mut self, d: &str) {
        if d != self.day {
            if !self.day.is_empty() {
                self.time.clear();
            }
            self.day = d.to_string();
        }
    }

    /// Per-site look and helpers for a page that just loaded.
    fn tweak(&self, id: u32) {
        let Some(i) = self.index(id) else { return };
        let t = &self.tabs[i];
        let h = guard::host(&t.url);
        let site = self.rp["sites"].as_object().and_then(|m| m.iter().find(|(k, _)| guard::host_matches(&h, k)).map(|(_, v)| v.clone())).unwrap_or(json!({}));
        let cfg = json!({
            "site": site,
            "gestures": self.rp["gestures"].as_bool().unwrap_or(true),
            "cookies": self.rp["cookies"].as_bool().unwrap_or(true),
            "ytAds": self.rp["ytAds"].as_bool().unwrap_or(true),
            "sponsor": self.rp["sponsor"].as_bool().unwrap_or(true),
            "noAutoplay": self.rp["noAutoplay"].as_bool().unwrap_or(false),
            "preload": self.rp["preload"].as_bool().unwrap_or(true),
            "drafts": self.rp["drafts"].as_bool().unwrap_or(true),
            "inthinking": h.ends_with("inthinking.net") || h.ends_with("inthinking.co.uk"),
            "outlook": self.rp["outlookSound"].as_bool().unwrap_or(true) && (h.starts_with("outlook.") || h.contains("outlook.office")),
            "scroll": t.restore_scroll,
        });
        let _ = t.wv.evaluate_script(&format!("window.__zb&&__zb.tweak({})", cfg));
        #[cfg(windows)]
        {
            if let Some(z) = site["zoom"].as_f64() {
                win::zoom(&t.wv, z);
            }
            if cfg["outlook"] == true && !t.muted {
                win::mute(&t.wv, true);
            }
        }
    }

    fn page_js(&self, id: u32, js: &str) {
        if let Some(i) = self.index(id) {
            let _ = self.tabs[i].wv.evaluate_script(js);
        }
    }

    fn open_peek(&mut self, url: &str) {
        self.close_peek();
        let c = self.content;
        let r = Rc4 { x: c.x + c.w * 0.08, y: c.y + c.h * 0.05, w: c.w * 0.84, h: c.h * 0.9 };
        let profile = self.active_tab().map(|t| t.profile.clone()).unwrap_or_default();
        self.peek = self.build_page(PEEK_ID, url, r, !self.overlay, &profile);
        if let Some(p) = &self.peek {
            let _ = p.focus();
        }
        {
            let mut k = self.keys.borrow_mut();
            k.insert("27".into());
            k.insert("C+13".into());
        }
        self.changed();
    }

    fn close_peek(&mut self) {
        if self.peek.take().is_some() {
            {
                let mut k = self.keys.borrow_mut();
                k.remove("27");
                k.remove("C+13");
            }
            self.ui("zb.keys()");
            if let Some(t) = self.active_tab() {
                let _ = t.wv.focus();
            }
            self.changed();
        }
    }

    fn peek_to_tab(&mut self) {
        let u = self.peek.as_ref().and_then(|p| p.url().ok()).unwrap_or_default();
        self.close_peek();
        if !u.is_empty() {
            let a = self.active;
            let pr = self.active_tab().map(|t| t.profile.clone()).unwrap_or_default();
            self.new_tab_in(&u, true, &pr, a);
        }
    }

    fn split_with(&mut self, other: u32) {
        let a = self.active;
        if a == other || self.index(other).is_none() {
            return;
        }
        if let Some(i) = self.index(other) {
            self.tabs[i].space = self.space.clone();
            self.tabs[i].door = None;
        }
        self.split = Some((a, other));
        self.place();
        self.changed();
    }

    /// Hand something to Zenko: %APPDATA%\FoxBuddy\browser_inbox.json (Zenko picks it up within a few seconds).
    fn to_zenko(&self, item: Value) -> bool {
        let Some(dir) = bridge::foxbuddy_dir() else { return false };
        let f = dir.join("browser_inbox.json");
        let mut list: Vec<Value> = std::fs::read_to_string(&f).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default();
        let mut item = item;
        item["at"] = json!(unix_now());
        list.push(item);
        let tmp = f.with_extension("tmp");
        std::fs::write(&tmp, Value::Array(list).to_string()).is_ok() && std::fs::rename(&tmp, &f).is_ok()
    }

    fn page_info(&self, id: u32) -> (String, String, String) {
        self.index(id)
            .map(|i| {
                let t = &self.tabs[i];
                (t.title.clone(), nice_url(&t.url), guard::subject(&t.title, &t.url).to_string())
            })
            .unwrap_or_default()
    }

    fn menu(&mut self, id: u32, cmd: &str, data: &str) {
        let enc = |s: &str| -> String {
            s.bytes().map(|b| if b.is_ascii_alphanumeric() || b"-_.~".contains(&b) { (b as char).to_string() } else { format!("%{:02X}", b) }).collect()
        };
        let frenchy = |s: &str| s.chars().any(|c| "éèêàâçùûôîïëœÉÈÀÇ".contains(c));
        match cmd {
            "lookup" => {
                let w = data.trim().to_lowercase();
                let fr = frenchy(&w) || self.page_info(id).1.contains(".fr");
                let u = if fr {
                    format!("https://www.wordreference.com/fren/{}", enc(&w))
                } else {
                    format!("https://dictionary.cambridge.org/dictionary/english/{}", enc(&w))
                };
                self.open_peek(&u);
            }
            "translateSel" => {
                let tl = if frenchy(data) || self.page_info(id).1.contains(".fr") { "en" } else { self.rp["translateTo"].as_str().unwrap_or("en") };
                self.open_peek(&format!("https://translate.google.com/?sl=auto&tl={}&text={}&op=translate", tl, enc(data)));
            }
            "translatePage" | "translatePageFr" => {
                let tl = if cmd == "translatePageFr" { "fr" } else { "en" };
                self.page_js(id, &format!("location.href={}", json!(format!("https://translate.google.com/translate?sl=auto&tl={}&u={}", tl, enc(data)))));
            }
            "toZenko" => {
                let (title, url, subject) = self.page_info(id);
                let ok = self.to_zenko(json!({"kind": "note", "text": data, "title": title, "url": url, "subject": subject}));
                self.ui(&format!("zb.toast({})", json!(if ok { "Sent to Zenko's notes" } else { "Zenko isn't set up on this PC yet" })));
            }
            "remind" => self.ui(&format!("zb.remind({})", json!(data))),
            "peek" => self.open_peek(data),
            "splitLink" => {
                let pr = self.index(id).map(|i| self.tabs[i].profile.clone()).unwrap_or_default();
                let nid = self.new_tab_in(data, false, &pr, id);
                if nid != 0 {
                    self.split_with(nid);
                }
            }
            "otherProfile" => {
                let cur = self.index(id).map(|i| self.tabs[i].profile.clone()).unwrap_or_default();
                let profiles: Vec<String> = self.rp["profiles"].as_array().map(|a| a.iter().filter_map(|x| x.as_str().map(String::from)).collect()).unwrap_or_else(|| vec!["School".into()]);
                let other = if cur.is_empty() { profiles.first().cloned().unwrap_or_else(|| "School".into()) } else { String::new() };
                let nid = self.new_tab_in(data, false, &other, id);
                if nid != 0 {
                    self.split_with(nid);
                }
            }
            "cleanLink" => {
                let c = guard::clean_url(data).unwrap_or_else(|| data.to_string());
                self.ui(&format!("zb.copy({},\"Clean link copied\")", json!(c)));
            }
            "cite" => self.page_js(id, "window.__zb&&__zb.cite()"),
            "clip" => self.ui("zb.clip()"),
            "savePage" => {
                #[cfg(windows)]
                if let Some(i) = self.index(id) {
                    win::snapshot(&self.tabs[i].wv, id, self.proxy.clone());
                }
            }
            _ => {}
        }
    }

    fn save_snapshot(&mut self, id: u32, json_txt: &str) {
        let data = serde_json::from_str::<Value>(json_txt).ok().and_then(|v| v["data"].as_str().map(String::from)).unwrap_or_default();
        if data.is_empty() {
            self.ui("zb.toast(\"Could not save this page\")");
            return;
        }
        let (title, _, _) = self.page_info(id);
        let name: String = title.chars().map(|c| if c.is_alphanumeric() || c == ' ' || c == '-' { c } else { '_' }).take(60).collect();
        let dir = self.data_file("saved");
        let _ = std::fs::create_dir_all(&dir);
        let f = dir.join(format!("{} {}.mhtml", name.trim(), unix_now()));
        if std::fs::write(&f, data).is_ok() {
            self.ui(&format!("zb.toast({})", json!(format!("Saved for offline: {}", title))));
            self.ui(&format!("zb.saved({})", self.saved_list()));
        }
    }

    fn saved_list(&self) -> Value {
        let mut out = vec![];
        if let Ok(d) = std::fs::read_dir(self.data_file("saved")) {
            for e in d.flatten() {
                let p = e.path();
                if p.extension().map(|x| x == "mhtml").unwrap_or(false) {
                    let name = p.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
                    let t = name.rsplit_once(' ').map(|(a, _)| a.to_string()).unwrap_or(name);
                    out.push(json!({"t": t, "u": format!("file:///{}", p.to_string_lossy().replace('\\', "/"))}));
                }
            }
        }
        Value::Array(out)
    }

    fn handle_ui(&mut self, msg: &str, target: &EventLoopWindowTarget<Ev>, flow: &mut ControlFlow) {
        let Ok(v) = serde_json::from_str::<Value>(msg) else { return };
        let s = |k: &str| v.get(k).and_then(|x| x.as_str()).unwrap_or("").to_string();
        let n = |k: &str| v.get(k).and_then(|x| x.as_u64()).unwrap_or(0) as u32;
        match s("c").as_str() {
            "ready" => {
                self.changed();
                let f = self.data_file("import.json");
                let mut rep = std::fs::read_to_string(&f).ok().and_then(|t| serde_json::from_str::<Value>(&t).ok()).unwrap_or(Value::Null);
                if rep.is_object() && rep["shown"] != true {
                    self.ui(&format!("zb.imported({})", rep));
                    rep["shown"] = json!(true);
                    let _ = std::fs::write(&f, rep.to_string());
                }
                let bm = edge::bookmarks(&self.data_dir.join("web"));
                self.ui(&format!("zb.bookmarks({})", bm));
                self.ui(&format!("zb.history({})", history::for_ui(&self.hist)));
                self.ui(&format!("zb.saved({})", self.saved_list()));
                self.ui(&format!("zb.archivedList({})", json!(self.archived)));
                self.ui(&format!("zb.downloads({})", Value::Array(self.downloads.clone())));
            }
            "space" => {
                let name = s("name");
                self.space = name.clone();
                self.split = None;
                let best = self.tabs.iter().filter(|t| t.space == name).max_by_key(|t| t.last_used).map(|t| t.id);
                match best {
                    Some(id) => self.activate(id),
                    None => self.new_tab(NEWTAB, true),
                }
            }
            "moveSpace" => {
                if let Some(i) = self.index(n("id")) {
                    self.tabs[i].space = s("name");
                    let id = self.tabs[i].id;
                    if id == self.active {
                        let other = self.tabs.iter().filter(|t| t.space == self.space && t.id != id).max_by_key(|t| t.last_used).map(|t| t.id);
                        match other {
                            Some(o) => self.activate(o),
                            None => self.new_tab(NEWTAB, true),
                        }
                    }
                    self.changed();
                }
            }
            "group" => {
                if let Some(i) = self.index(n("id")) {
                    self.tabs[i].group = s("name");
                    self.changed();
                }
            }
            "split" => self.split_with(n("id")),
            "unsplit" => {
                self.split = None;
                self.place();
                self.changed();
            }
            "peekClose" => self.close_peek(),
            "peekOpen" => self.peek_to_tab(),
            "search" => {
                let q = s("q").to_lowercase();
                let mut out = vec![];
                if q.len() >= 2 {
                    for t in &self.tabs {
                        let low = t.text.to_lowercase();
                        if let Some(at) = low.find(&q) {
                            let st = low[..at].char_indices().rev().nth(40).map(|(i, _)| i).unwrap_or(0);
                            let snip: String = t.text[st..].chars().take(110).collect();
                            out.push(json!({"id": t.id, "snip": snip.replace('\n', " ")}));
                        }
                    }
                }
                self.ui(&format!("zb.found({},{})", json!(s("q")), Value::Array(out)));
            }
            "unarchive" => {
                let i = n("i") as usize;
                if i < self.archived.len() {
                    let t = self.archived.remove(i);
                    let id = self.new_tab_in(&t.url, true, &t.profile, 0);
                    if let Some(x) = self.index(id) {
                        self.tabs[x].space = self.space.clone();
                    }
                    self.ui(&format!("zb.archivedList({})", json!(self.archived)));
                }
            }
            "panelMode" => {
                self.panel_mode = s("m");
                if self.panel_mode.is_empty() {
                    self.panel_mode = "claude".into();
                }
                if self.panel_mode == "claude" {
                    let open = self.panel_open;
                    self.set_panel(open);
                } else {
                    if let Some(p) = &self.panel {
                        let _ = p.set_visible(false);
                    }
                    self.changed();
                }
            }
            "openPath" | "showPath" => {
                let p = s("p");
                if self.downloads.iter().any(|d| d["path"] == p) {
                    let mut c = std::process::Command::new("explorer");
                    if s("c") == "showPath" {
                        c.arg(format!("/select,{}", p));
                    } else {
                        c.arg(&p);
                    }
                    let _ = c.spawn();
                }
            }
            "openSaved" => {
                let _ = std::process::Command::new("explorer").arg(self.data_file("saved")).spawn();
            }
            "openExtensions" => {
                let d = self.data_file("extensions");
                let _ = std::fs::create_dir_all(&d);
                let _ = std::process::Command::new("explorer").arg(d).spawn();
            }
            "video" => {
                let id = if n("id") > 0 { n("id") } else { self.active };
                self.page_js(id, &format!("window.__zb&&__zb.video({},{})", json!(s("a")), v["v"]));
            }
            "hints" => {
                let a = self.active;
                self.page_js(a, "window.__zb&&__zb.hints()");
                if let Some(t) = self.active_tab() {
                    let _ = t.wv.focus();
                }
            }
            "toZenko" => {
                let ok = self.to_zenko(v["item"].clone());
                self.ui(&format!("zb.toast({})", json!(if ok { "Sent to Zenko" } else { "Zenko isn't set up on this PC yet" })));
            }
            "clip" => {
                let data = s("data");
                let b64 = data.split_once(",").map(|x| x.1).unwrap_or("");
                use base64::Engine;
                let ok = match (bridge::foxbuddy_dir(), base64::engine::general_purpose::STANDARD.decode(b64)) {
                    (Some(dir), Ok(bytes)) => {
                        let cd = dir.join("clips");
                        let _ = std::fs::create_dir_all(&cd);
                        let f = cd.join(format!("clip-{}.png", unix_now()));
                        let (title, url, subject) = self.page_info(self.active);
                        std::fs::write(&f, bytes).is_ok()
                            && self.to_zenko(json!({"kind": "clip", "path": f.to_string_lossy(), "title": title, "url": url, "subject": subject, "text": s("note")}))
                    }
                    _ => false,
                };
                self.ui(&format!("zb.toast({})", json!(if ok { "Clip sent to Zenko's notes" } else { "Zenko isn't set up on this PC yet" })));
            }
            "makeDefault" => {
                #[cfg(windows)]
                {
                    let ok = win::register_default();
                    self.ui(&format!("zb.toast({})", json!(if ok { "Pick Zenko Browser in the Windows settings that just opened" } else { "Windows did not let us register" })));
                }
            }
            "newIn" => {
                let u = s("url");
                let a = self.active;
                self.new_tab_in(if u.is_empty() { NEWTAB } else { &u }, true, &s("profile"), if u.is_empty() { 0 } else { a });
            }
            "focusPause" => {
                let mins = v["min"].as_u64().unwrap_or(5);
                self.focus_pause = Some(Instant::now() + Duration::from_secs(mins * 60));
                self.guard.borrow_mut().focus = false;
                if let Some(u) = v["url"].as_str() {
                    let a = self.active;
                    self.page_js(a, &format!("location.href={}", json!(u)));
                }
                self.changed();
            }
            "ntData" => {
                let from = n("_from");
                let d = json!({"recap": self.recap(), "downloads": self.downloads.iter().rev().take(4).cloned().collect::<Vec<_>>(),
                               "saved": self.saved_list(), "zenko": bridge::foxbuddy_dir().is_some()});
                self.page_js(from, &format!("window.zbNT&&zbNT({})", d));
            }
            "recap" => {
                let r = self.recap();
                self.ui(&format!("zb.recap({})", r));
            }
            "day" => {
                let d = s("d");
                self.new_day(&d);
            }
            "page" => {
                let a = self.active;
                let url = self.active_tab().map(|t| t.url.clone()).unwrap_or_default();
                let cmd = s("a");
                self.menu(a, &cmd, &url);
            }
            "importEdge" => {
                self.save();
                let mut a = vec![format!("--wait-parent={}", std::process::id()), "--import-edge".to_string()];
                if v["close"].as_bool().unwrap_or(false) {
                    a.push("--close-edge".into());
                }
                if let Ok(exe) = std::env::current_exe() {
                    let _ = std::process::Command::new(exe).args(a).spawn();
                }
                *flow = ControlFlow::Exit;
            }
            "layout" => self.layout(&v),
            "new" => {
                let u = s("url");
                self.new_tab(if u.is_empty() { NEWTAB } else { &u }, !v["bg"].as_bool().unwrap_or(false))
            }
            "close" => self.close_tab(n("id")),
            "activate" => self.activate(n("id")),
            "go" => {
                let u = s("url");
                if let Some(t) = self.active_tab() {
                    let _ = t.wv.load_url(&real_url(&u));
                    let _ = t.wv.focus();
                } else {
                    self.new_tab(&u, true);
                }
            }
            "back" => {
                if let Some(t) = self.active_tab() {
                    let _ = t.wv.go_back();
                }
            }
            "fwd" => {
                if let Some(t) = self.active_tab() {
                    let _ = t.wv.go_forward();
                }
            }
            "reload" => {
                let id = if n("id") > 0 { n("id") } else { self.active };
                if let Some(i) = self.index(id) {
                    let t = &mut self.tabs[i];
                    if t.level >= 2 {
                        t.level = 0;
                    }
                    let _ = t.wv.reload();
                }
            }
            "dup" => {
                if let Some(i) = self.index(n("id")) {
                    let u = self.tabs[i].url.clone();
                    let id = self.tabs[i].id;
                    self.activate(id);
                    self.new_tab(&u, true);
                }
            }
            "pin" => {
                if let Some(i) = self.index(n("id")) {
                    let mut t = self.tabs.remove(i);
                    t.pinned = !t.pinned;
                    let pins = self.tabs.iter().filter(|x| x.pinned).count();
                    self.tabs.insert(pins.min(self.tabs.len()), t);
                    self.changed();
                }
            }
            "closeOthers" | "closeRight" => {
                let id = n("id");
                let Some(at) = self.index(id) else { return };
                let right = s("c") == "closeRight";
                let gone: Vec<u32> =
                    self.tabs.iter().enumerate().filter(|(i, t)| t.id != id && !t.pinned && (!right || *i > at)).map(|(_, t)| t.id).collect();
                if !gone.is_empty() {
                    self.activate(id);
                }
                for g in gone {
                    self.close_tab(g);
                }
            }
            "stop" => {
                #[cfg(windows)]
                if let Some(t) = self.active_tab() {
                    win::stop(&t.wv);
                }
            }
            "move" => {
                if let Some(i) = self.index(n("id")) {
                    let t = self.tabs.remove(i);
                    let pins = self.tabs.iter().filter(|x| x.pinned).count();
                    let to = (n("to") as usize).min(self.tabs.len());
                    let to = if t.pinned { to.min(pins) } else { to.max(pins) };
                    self.tabs.insert(to, t);
                    self.changed();
                }
            }
            "mute" => {
                let id = n("id");
                if let Some(i) = self.index(id) {
                    let t = &mut self.tabs[i];
                    t.muted = !t.muted;
                    #[cfg(windows)]
                    win::mute(&t.wv, t.muted);
                    self.changed();
                }
            }
            "reopen" => {
                if let Some(u) = self.closed.pop() {
                    self.new_tab(&u, true);
                }
            }
            "panel" => {
                let open = v["open"].as_bool().unwrap_or(!self.panel_open);
                self.set_panel(open);
            }
            "panelHome" => {
                if let Some(p) = &self.panel {
                    let _ = p.load_url(CLAUDE_URL);
                }
            }
            "popout" => self.popout(target),
            "overlay" => self.set_overlay(v["on"].as_bool().unwrap_or(false)),
            "shown" => self.hide_pages(),
            "keys" => {
                if let Some(a) = v["list"].as_array() {
                    let mut k = self.keys.borrow_mut();
                    k.clear();
                    k.extend(a.iter().filter_map(|x| x.as_str().map(String::from)));
                }
            }
            "prefs" => {
                let p = v["p"].clone();
                self.apply_prefs(&p);
                self.changed();
            }
            "devtools" => {
                if self.focused == PANEL_ID {
                    if let Some(p) = &self.panel {
                        p.open_devtools();
                    }
                } else if let Some(t) = self.active_tab() {
                    t.wv.open_devtools();
                }
            }
            "print" => {
                if let Some(t) = self.active_tab() {
                    let _ = t.wv.print();
                }
            }
            "zoom" => {
                if let Some(t) = self.active_tab() {
                    #[cfg(windows)]
                    win::zoom(&t.wv, v["f"].as_f64().unwrap_or(1.0));
                    #[cfg(not(windows))]
                    let _ = t.wv.zoom(v["f"].as_f64().unwrap_or(1.0));
                }
            }
            "revealed" => {
                let id = n("id");
                if let Some(i) = self.index(id) {
                    self.tabs[i].door = None;
                    if self.shown_ids().contains(&id) && !self.overlay {
                        let _ = self.tabs[i].wv.set_visible(true);
                        if id == self.active {
                            let _ = self.tabs[i].wv.focus();
                        }
                    }
                    self.changed();
                }
            }
            "focusPage" => {
                if let Some(t) = self.active_tab() {
                    let _ = t.wv.focus();
                }
            }
            "win" => match s("a").as_str() {
                "min" => self.window.set_minimized(true),
                "max" => {
                    self.window.set_maximized(!self.window.is_maximized());
                    self.changed();
                }
                "close" => {
                    self.save();
                    *flow = ControlFlow::Exit;
                }
                "drag" => {
                    let _ = self.window.drag_window();
                }
                "resize" => {
                    let d = match s("d").as_str() {
                        "n" => ResizeDirection::North,
                        "s" => ResizeDirection::South,
                        "e" => ResizeDirection::East,
                        "w" => ResizeDirection::West,
                        "ne" => ResizeDirection::NorthEast,
                        "nw" => ResizeDirection::NorthWest,
                        "se" => ResizeDirection::SouthEast,
                        _ => ResizeDirection::SouthWest,
                    };
                    let _ = self.window.drag_resize_window(d);
                }
                _ => {}
            },
            "update" => updater::start(self.proxy.clone(), true),
            "restart" => {
                self.save();
                if let Ok(exe) = std::env::current_exe() {
                    let _ = std::process::Command::new(exe).arg(format!("--wait-parent={}", std::process::id())).spawn();
                }
                *flow = ControlFlow::Exit;
            }
            "openData" => {
                let _ = std::process::Command::new("explorer").arg(&self.data_dir).spawn();
            }
            _ => {}
        }
    }

    /// Messages from pages. Only our own zenko:// pages may drive the browser; other sites can only
    /// tell us their icon.
    fn handle_page(&mut self, id: u32, src: &str, body: &str, target: &EventLoopWindowTarget<Ev>, flow: &mut ControlFlow) {
        let Ok(mut v) = serde_json::from_str::<Value>(body) else { return };
        if is_ours(src) {
            v["_from"] = json!(id);
            self.handle_ui(&v.to_string(), target, flow);
            return;
        }
        let Some(i) = self.index(id) else {
            if id == PEEK_ID && v["t"] == "key" && v["k"] == "Escape" {
                self.close_peek();
            }
            return;
        };
        match v["t"].as_str().unwrap_or("") {
            "icon" => {
                if let Some(h) = v["href"].as_str() {
                    if h.starts_with("https://") || h.starts_with("data:image/") {
                        self.tabs[i].icon = h.chars().take(4000).collect();
                        self.changed();
                    }
                }
            }
            "hello" => self.tweak(id),
            "text" => {
                self.tabs[i].text = v["s"].as_str().unwrap_or("").chars().take(30000).collect();
            }
            "scroll" => {
                self.tabs[i].scroll = v["y"].as_f64().unwrap_or(0.0);
                self.tabs[i].restore_scroll = 0.0;
                self.dirty = true;
            }
            "cite" => self.ui(&format!("zb.cite({})", v)),
            "mail" => self.ui("zb.mail()"),
            "notes" => {
                // InThinking: key terms of the page for Zenko's notes
                let (title, url, subject) = self.page_info(id);
                let ok = self.to_zenko(json!({"kind": "note", "text": v["s"], "title": title, "url": url, "subject": if subject.is_empty() { "".to_string() } else { subject }}));
                self.ui(&format!("zb.toast({})", json!(if ok { "Key terms sent to Zenko's notes" } else { "Zenko isn't set up on this PC yet" })));
            }
            "gesture" => match v["g"].as_str().unwrap_or("") {
                "back" => {
                    let _ = self.tabs[i].wv.go_back();
                }
                "fwd" => {
                    let _ = self.tabs[i].wv.go_forward();
                }
                "reload" => {
                    let _ = self.tabs[i].wv.reload();
                }
                "close" => self.close_tab(id),
                "new" => self.new_tab(NEWTAB, true),
                "reopen" => {
                    if let Some(u) = self.closed.pop() {
                        self.new_tab(&u, true);
                    }
                }
                _ => {}
            },
            _ => {}
        }
    }
}

fn win_icon() -> Option<tao::window::Icon> {
    #[cfg(windows)]
    {
        use tao::platform::windows::IconExtWindows;
        tao::window::Icon::from_resource(1, None).ok()
    }
    #[cfg(not(windows))]
    {
        None
    }
}

fn data_dir() -> PathBuf {
    let base = std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::temp_dir());
    base.join("ZenkoBrowser")
}

fn main() {
    updater::cleanup_old();
    let dir = data_dir();
    let _ = std::fs::create_dir_all(&dir);
    let all: Vec<String> = std::env::args().skip(1).collect();
    let args: Vec<String> = all.iter().filter(|a| !a.starts_with("--")).cloned().collect();
    if let Some(pid) = all.iter().find_map(|a| a.strip_prefix("--wait-parent=").and_then(|p| p.parse::<u32>().ok())) {
        edge::wait_for_pid(pid);
    }
    // One window: a second start (for example a link clicked in another app) hands its links to the first.
    #[cfg(windows)]
    if win::already_running() {
        let od = dir.join("open");
        let _ = std::fs::create_dir_all(&od);
        let nanos = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0);
        let _ = std::fs::write(od.join(format!("{}.txt", nanos)), if args.is_empty() { "focus".to_string() } else { args.join("\n") });
        return;
    }
    let rp: Value = std::fs::read_to_string(dir.join("rprefs.json")).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or(json!({}));
    let mut bargs = String::from("--disable-features=msWebOOUI,msPdfOOUI,msSmartScreenProtection");
    if rp["noAutoplay"].as_bool().unwrap_or(false) {
        bargs.push_str(" --autoplay-policy=document-user-activation-required");
    }
    let _ = ARGS.set(bargs);
    // Import from Edge: automatically on the very first start, or when asked from the browser.
    let web = dir.join("web");
    let first = !web.join("EBWebView").exists();
    let asked = all.iter().any(|a| a == "--import-edge");
    if (first || asked) && edge::has_edge() {
        if all.iter().any(|a| a == "--close-edge") {
            edge::close_edge();
        }
        let mut rep = edge::run(&web);
        rep["shown"] = json!(false);
        rep["first"] = json!(first);
        let _ = std::fs::write(dir.join("import.json"), rep.to_string());
    }
    // Address-bar history: our own visits plus Edge's (read once after each import).
    let mut hist = history::load(&dir.join("history.json"));
    let edge_hist = web.join("EBWebView").join("Default").join("History");
    let mark = dir.join("history-edge.done");
    if edge_hist.exists() && (!mark.exists() || asked) {
        if history::merge_edge(&mut hist, &edge_hist) > 0 {
            history::save(&dir.join("history.json"), &hist);
            let _ = std::fs::write(&mark, "1");
        }
    }

    let event_loop = EventLoopBuilder::<Ev>::with_user_event().build();
    let proxy = event_loop.create_proxy();

    let mut wb = WindowBuilder::new()
        .with_title("Zenko Browser")
        .with_inner_size(LogicalSize::new(1280.0, 820.0))
        .with_min_inner_size(LogicalSize::new(520.0, 360.0))
        .with_decorations(false)
        .with_window_icon(win_icon());
    #[cfg(windows)]
    {
        use tao::platform::windows::WindowBuilderExtWindows;
        wb = wb.with_undecorated_shadow(true);
    }
    let window = wb.build(&event_loop).expect("could not open a window");

    let mut ctx = WebContext::new(Some(dir.join("web")));
    let keys: Rc<RefCell<HashSet<String>>> = Rc::new(RefCell::new(HashSet::new()));
    let size = window.inner_size().to_logical::<f64>(window.scale_factor());
    let pu = proxy.clone();
    let cb = WebViewBuilder::new_with_web_context(&mut ctx)
        .with_url("http://zenko.localhost/chrome.html")
        .with_bounds(rect(Rc4 { x: 0.0, y: 0.0, w: size.width, h: size.height }))
        .with_background_color((11, 15, 35, 255))
        .with_clipboard(true)
        .with_devtools(cfg!(debug_assertions))
        .with_custom_protocol("zenko".into(), protocol)
        .with_ipc_handler(move |req: Request<String>| {
            if is_ours(&req.uri().to_string()) {
                let _ = pu.send_event(Ev::Ui(req.into_body()));
            }
        });
    #[cfg(windows)]
    let cb = cb
        .with_default_context_menus(false)
        .with_browser_accelerator_keys(false)
        .with_additional_browser_args(browser_args())
        .with_browser_extensions_enabled(true);
    let chrome = cb.build_as_child(&window).expect("WebView2 is missing. Install the Microsoft Edge WebView2 Runtime.");
    #[cfg(windows)]
    win::hook(&chrome, u32::MAX, proxy.clone(), keys.clone());

    // A first guess at the page area until the frame reports its real layout.
    let content = Rc4 { x: 8.0, y: 84.0, w: (size.width - 16.0).max(1.0), h: (size.height - 92.0).max(1.0) };
    let mut app = App {
        window,
        chrome,
        ctx,
        proxy: proxy.clone(),
        keys,
        tabs: vec![],
        active: 0,
        next_id: 1,
        panel: None,
        panel_title: "Claude".into(),
        panel_open: false,
        content,
        panel_rect: Rc4 { x: size.width - 420.0, y: 84.0, w: 412.0, h: size.height - 92.0 },
        focused: 1,
        overlay: false,
        shots_wanted: 0,
        closed: vec![],
        popouts: vec![],
        data_dir: dir.clone(),
        dirty: false,
        last_save: Instant::now(),
        last_title: String::new(),
        sleep_after: 20,
        fullscreen: false,
        hist,
        hist_dirty: false,
        guard: Rc::new(RefCell::new(guard::Guard::new())),
        space: String::new(),
        split: None,
        peek: None,
        rp: json!({}),
        downloads: vec![],
        dl_subject: Rc::new(RefCell::new((true, String::new()))),
        time: HashMap::new(),
        day: String::new(),
        archived: vec![],
        panel_mode: "claude".into(),
        battery: false,
        win_focused: true,
        ext_tried: false,
        focus_pause: None,
    };
    app.apply_prefs(&rp);
    if let Some(t) = std::fs::read_to_string(dir.join("time.json")).ok().and_then(|t| serde_json::from_str::<Value>(&t).ok()) {
        app.day = t["day"].as_str().unwrap_or("").to_string();
        if let Some(m) = t["hosts"].as_object() {
            app.time = m.iter().map(|(k, v)| (k.clone(), v.as_u64().unwrap_or(0))).collect();
        }
    }

    // Session restore
    let saved: Session = std::fs::read_to_string(dir.join("session.json"))
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default();
    app.closed = saved.closed.clone();
    app.archived = saved.archived.clone();
    app.space = saved.space.clone();
    let mut ids = vec![];
    for t in saved.tabs.iter().take(80) {
        let id = app.new_tab_in(&t.url, false, &t.profile, 0);
        if let Some(i) = app.index(id) {
            let x = &mut app.tabs[i];
            x.title = t.title.clone();
            x.pinned = t.pinned;
            x.space = t.space.clone();
            x.group = t.group.clone();
            x.last_used = if t.last_used == 0 { unix_now() } else { t.last_used };
            x.scroll = t.scroll;
            x.restore_scroll = t.scroll;
        }
        ids.push(id);
    }
    for a in &args {
        let u = if a.contains("://") { a.clone() } else { format!("file:///{}", a.replace('\\', "/")) };
        app.new_tab(&u, false);
        ids.push(app.tabs.last().map(|t| t.id).unwrap_or(0));
    }
    if app.tabs.is_empty() {
        app.new_tab(NEWTAB, true);
    } else {
        let pick = if !args.is_empty() { *ids.last().unwrap() } else { ids.get(saved.active).copied().unwrap_or(ids[0]) };
        app.activate(pick);
    }
    if saved.panel {
        app.set_panel(true);
        app.focused = app.active;
    }
    updater::start(proxy.clone(), false);

    let main_id: WindowId = app.window.id();
    let mut last_tick = Instant::now();
    let mut ticks: u64 = 0;
    let mut ui_dirty = false;
    event_loop.run(move |event, target, flow| {
        *flow = ControlFlow::WaitUntil(Instant::now() + Duration::from_secs(1));
        match event {
            Event::NewEvents(StartCause::Init) => {}
            Event::WindowEvent { event: WindowEvent::CloseRequested, window_id, .. } => {
                if window_id == main_id {
                    app.save();
                    *flow = ControlFlow::Exit;
                } else {
                    app.popouts.retain(|(w, _)| w.id() != window_id);
                }
            }
            Event::WindowEvent { event: WindowEvent::Focused(f), window_id, .. } if window_id == main_id => {
                app.win_focused = f;
            }
            Event::WindowEvent { event: WindowEvent::Resized(sz), window_id, .. } => {
                if window_id == main_id {
                    let l = sz.to_logical::<f64>(app.window.scale_factor());
                    let _ = app.chrome.set_bounds(rect(Rc4 { x: 0.0, y: 0.0, w: l.width, h: l.height }));
                    app.ui(&format!("zb.maximized({})", app.window.is_maximized()));
                    if app.fullscreen {
                        app.place();
                    }
                } else if let Some((w, wv)) = app.popouts.iter().find(|(w, _)| w.id() == window_id) {
                    let l = sz.to_logical::<f64>(w.scale_factor());
                    let _ = wv.set_bounds(rect(Rc4 { x: 0.0, y: 0.0, w: l.width, h: l.height }));
                }
            }
            Event::UserEvent(ev) => match ev {
                Ev::Ui(m) => app.handle_ui(&m, target, flow),
                Ev::Page(id, src, body) => app.handle_page(id, &src, &body, target, flow),
                Ev::Title(id, t) => {
                    if id == PANEL_ID {
                        app.panel_title = t;
                        app.tell_zenko();
                    } else if let Some(i) = app.index(id) {
                        if let Ok(u) = app.tabs[i].wv.url() {
                            app.tabs[i].url = u;
                        }
                        history::set_title(&mut app.hist, &app.tabs[i].url, &t);
                        app.hist_dirty = true;
                        app.tabs[i].title = t;
                        app.changed();
                    }
                }
                Ev::Load(id, started, url) => {
                    if let Some(i) = app.index(id) {
                        if !started && app.tabs[i].door.is_some() {
                            app.ui(&format!("zb.openDoor({})", id));
                        }
                        if !started {
                            let title = app.tabs[i].title.clone();
                            history::visit(&mut app.hist, &url, if title.starts_with("http") { "" } else { &title });
                            app.hist_dirty = true;
                        }
                        app.tabs[i].loading = started;
                        app.tabs[i].url = url;
                        if started {
                            app.tabs[i].icon.clear();
                        }
                        app.changed();
                    }
                }
                Ev::Open(src, u) => {
                    let (pr, parent) = if src == PEEK_ID || src == PANEL_ID {
                        (String::new(), 0)
                    } else {
                        (app.index(src).map(|i| app.tabs[i].profile.clone()).unwrap_or_default(), src)
                    };
                    app.new_tab_in(&u, true, &pr, parent);
                }
                Ev::Key(k) => {
                    if app.peek.is_some() && k == "27" {
                        app.close_peek();
                    } else if app.peek.is_some() && k == "C+13" {
                        app.peek_to_tab();
                    } else {
                        app.ui(&format!("zb.key({})", json!(k)));
                    }
                }
                Ev::Focus(id) => {
                    if app.peek.is_some() && id != PEEK_ID && id != u32::MAX {
                        app.close_peek();
                    }
                    if let Some((a, b)) = app.split {
                        if (id == a || id == b) && id != app.active {
                            app.active = id;
                            app.changed();
                        }
                    }
                    if id != u32::MAX && id != app.focused {
                        app.focused = id;
                        app.tell_zenko();
                    }
                }
                Ev::Blocked(id, h) => {
                    if let Some(i) = app.index(id) {
                        let t = &mut app.tabs[i];
                        t.blocked += 1;
                        if !t.blocked_hosts.contains(&h) && t.blocked_hosts.len() < 40 {
                            t.blocked_hosts.push(h);
                        }
                        if app.shown_ids().contains(&id) {
                            ui_dirty = true;
                        }
                    }
                }
                Ev::Crashed(id) => {
                    if let Some(i) = app.index(id) {
                        let _ = app.tabs[i].wv.reload();
                        app.ui("zb.toast(\"A page stopped working, so it was reloaded\")");
                    }
                }
                Ev::Menu(id, cmd, data) => app.menu(id, &cmd, &data),
                Ev::Redirect(id, u) => {
                    if id == PEEK_ID {
                        if let Some(p) = &app.peek {
                            let _ = p.load_url(&real_url(&u));
                        }
                    } else if let Some(i) = app.index(id) {
                        let _ = app.tabs[i].wv.load_url(&real_url(&u));
                    }
                }
                Ev::Snapshot(id, j) => app.save_snapshot(id, &j),
                Ev::DlStart(u, path) => {
                    let name = std::path::Path::new(&path).file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
                    app.downloads.push(json!({"url": u, "path": path, "name": name, "done": false, "ok": false, "at": unix_now()}));
                    if app.downloads.len() > 60 {
                        app.downloads.remove(0);
                    }
                    app.ui(&format!("zb.downloads({})", Value::Array(app.downloads.clone())));
                }
                Ev::DlDone(u, path, ok) => {
                    if let Some(d) = app.downloads.iter_mut().rev().find(|d| d["url"] == u.as_str() && d["done"] == false) {
                        d["done"] = json!(true);
                        d["ok"] = json!(ok);
                        if !path.is_empty() {
                            d["path"] = json!(path);
                        }
                    }
                    app.ui(&format!("zb.downloads({})", Value::Array(app.downloads.clone())));
                }
                Ev::Shot(k, data) => {
                    app.ui(&format!("zb.shot({},{})", json!(k), json!(data)));
                    app.shots_wanted = app.shots_wanted.saturating_sub(1);
                    if app.shots_wanted == 0 {
                        app.ui("zb.shotsDone()");
                    }
                }
                Ev::Audio(id, on) => {
                    if let Some(i) = app.index(id) {
                        app.tabs[i].audio = on;
                        app.changed();
                    }
                }
                Ev::Fullscreen(id, on) => app.set_fullscreen(id, on),
                Ev::Update(v) => app.ui(&format!("zb.update({})", v)),
            },
            _ => {}
        }
        if last_tick.elapsed() >= Duration::from_secs(1) {
            last_tick = Instant::now();
            ticks += 1;
            if ui_dirty {
                ui_dirty = false;
                app.changed();
            }
            app.time_tick();
            // links handed over by a second start
            if let Ok(d) = std::fs::read_dir(app.data_file("open")) {
                for e in d.flatten() {
                    let txt = std::fs::read_to_string(e.path()).unwrap_or_default();
                    let _ = std::fs::remove_file(e.path());
                    for l in txt.lines().map(|l| l.trim()).filter(|l| !l.is_empty() && *l != "focus") {
                        let u = if l.contains("://") { l.to_string() } else { format!("file:///{}", l.replace('\\', "/")) };
                        app.new_tab(&u, true);
                    }
                    app.window.set_minimized(false);
                    app.window.set_focus();
                }
            }
            if let Some(until) = app.focus_pause {
                if Instant::now() >= until {
                    app.focus_pause = None;
                    let on = app.rp["focus"].as_bool().unwrap_or(false);
                    app.guard.borrow_mut().focus = on;
                    app.changed();
                }
            }
            if ticks % 60 == 1 {
                app.archive_tick();
                #[cfg(windows)]
                {
                    let mode = app.rp["battery"].as_str().unwrap_or("auto").to_string();
                    let b = mode == "on" || (mode == "auto" && win::on_battery());
                    if b != app.battery {
                        app.battery = b;
                        app.changed();
                    }
                }
            }
            if app.dirty && app.last_save.elapsed() > Duration::from_secs(4) {
                app.save();
            }
            app.sleep_tick();
            let slow: Vec<u32> = app
                .tabs
                .iter()
                .filter(|t| t.door.map(|d| d.elapsed() > Duration::from_secs(6)).unwrap_or(false))
                .map(|t| t.id)
                .collect();
            for id in slow {
                if let Some(i) = app.index(id) {
                    app.tabs[i].door = Some(Instant::now() + Duration::from_secs(60));
                }
                app.ui(&format!("zb.openDoor({})", id));
            }
        }
    });
}
