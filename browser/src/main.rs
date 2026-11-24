// Zenko Browser: a small Rust shell around WebView2 (the Edge engine that is already on Windows).
// The browser frame (tabs, toolbar, command bar, animations) is a web page drawn by Chromium's GPU
// compositor, so it animates at the screen's full refresh rate. Each tab, the Claude panel and the
// frame are separate WebView2 views inside one window.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod bridge;
mod edge;
mod history;
mod updater;
#[cfg(windows)]
mod win;

use std::{
    borrow::Cow,
    cell::RefCell,
    collections::HashSet,
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

#[derive(Debug)]
pub enum Ev {
    Ui(String),
    Page(u32, String, String),
    Title(u32, String),
    Load(u32, bool, String),
    Open(String),
    Key(String),
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
}
#[derive(Serialize, Deserialize, Clone)]
struct SavedTab {
    url: String,
    title: String,
    #[serde(default)]
    pinned: bool,
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
}

// ---------------------------------------------------------------- built-in pages (zenko://...)

fn serve(path: &str) -> Response<Cow<'static, [u8]>> {
    let (body, mime): (&'static [u8], &str) = match path.trim_start_matches('/') {
        "" | "chrome" | "chrome.html" => (include_bytes!("../ui/chrome.html"), "text/html; charset=utf-8"),
        "newtab" | "newtab.html" => (include_bytes!("../ui/newtab.html"), "text/html; charset=utf-8"),
        "settings" | "settings.html" => (include_bytes!("../ui/settings.html"), "text/html; charset=utf-8"),
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

    fn build_page(&mut self, id: u32, url: &str, bounds: Rc4, visible: bool) -> Option<WebView> {
        let p1 = self.proxy.clone();
        let p2 = self.proxy.clone();
        let p3 = self.proxy.clone();
        let p4 = self.proxy.clone();
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
                    let _ = p4.send_event(Ev::Open(u));
                    NewWindowResponse::Deny
                }
            });
        #[cfg(windows)]
        let builder = builder
            .with_browser_accelerator_keys(true)
            .with_theme(if is_panel { Theme::Dark } else { Theme::Auto });
        match builder.build_as_child(&self.window) {
            Ok(wv) => {
                #[cfg(windows)]
                win::hook(&wv, id, self.proxy.clone(), self.keys.clone());
                Some(wv)
            }
            Err(e) => {
                eprintln!("could not open a page: {e}");
                None
            }
        }
    }

    fn new_tab(&mut self, url: &str, focus: bool) {
        let id = self.next_id;
        self.next_id += 1;
        let Some(wv) = self.build_page(id, url, self.content, false) else { return };
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
        };
        // new tabs open right after the current one
        let at = self.index(self.active).map(|i| i + 1).unwrap_or(self.tabs.len());
        self.tabs.insert(at.min(self.tabs.len()), tab);
        if focus || self.tabs.len() == 1 {
            self.activate(id);
        }
        self.changed();
    }

    fn index(&self, id: u32) -> Option<usize> {
        self.tabs.iter().position(|t| t.id == id)
    }

    fn activate(&mut self, id: u32) {
        let Some(i) = self.index(id) else { return };
        if self.fullscreen && id != self.active {
            let a = self.active;
            self.set_fullscreen(a, false);
        }
        let old = self.active;
        self.active = id;
        let content = self.content;
        let overlay = self.overlay;
        for t in self.tabs.iter_mut() {
            if t.id == id {
                let _ = t.wv.set_bounds(rect(content));
                let _ = t.wv.set_visible(!overlay && t.door.is_none());
                #[cfg(windows)]
                if t.level > 0 {
                    let _ = t.wv.set_memory_usage_level(MemoryUsageLevel::Normal);
                }
                t.level = 0;
                t.hidden_at = None;
            } else if t.id == old || t.hidden_at.is_none() {
                let _ = t.wv.set_visible(false);
                t.hidden_at = Some(Instant::now());
            }
        }
        if !overlay {
            let _ = self.tabs[i].wv.focus();
        }
        self.focused = id;
        self.changed();
    }

    fn close_tab(&mut self, id: u32) {
        let Some(i) = self.index(id) else { return };
        let t = self.tabs.remove(i);
        if t.url != NEWTAB {
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
        if self.active == id {
            let n = self.tabs[i.min(self.tabs.len() - 1)].id;
            self.activate(n);
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
                       "loading": t.loading, "audio": t.audio, "muted": t.muted, "asleep": t.level >= 2, "door": t.door.is_some(), "pinned": t.pinned})
            })
            .collect();
        let (back, fwd) = self
            .active_tab()
            .map(|t| (t.wv.can_go_back().unwrap_or(false), t.wv.can_go_forward().unwrap_or(false)))
            .unwrap_or((false, false));
        let st = json!({"tabs": tabs, "active": self.active, "panel": self.panel_open,
                        "back": back, "fwd": fwd, "closed": self.closed.len(), "version": VERSION,
                        "max": self.window.is_maximized()});
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
        if let Some(t) = self.active_tab() {
            let _ = t.wv.set_bounds(rect(self.content));
        }
        if let Some(p) = &self.panel {
            let _ = p.set_bounds(rect(self.panel_rect));
        }
    }

    fn set_panel(&mut self, open: bool) {
        self.panel_open = open;
        if open && self.panel.is_none() {
            self.panel = self.build_page(PANEL_ID, CLAUDE_URL, self.panel_rect, !self.overlay);
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
        let b = b.with_theme(Theme::Dark);
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
                if let Some(t) = self.active_tab() {
                    win::capture(&t.wv, "page", self.proxy.clone());
                    self.shots_wanted += 1;
                }
                if self.panel_open {
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
            if let Some(t) = self.active_tab() {
                let _ = t.wv.set_visible(t.door.is_none());
            }
            if self.panel_open {
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
        if let Some(t) = self.active_tab() {
            let r = if on { self.full_rect() } else { self.content };
            let _ = t.wv.set_bounds(rect(r));
        }
        if let Some(p) = &self.panel {
            let _ = p.set_visible(!on && self.panel_open);
        }
    }

    fn hide_pages(&self) {
        if !self.overlay {
            return;
        }
        if let Some(t) = self.active_tab() {
            let _ = t.wv.set_visible(false);
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
                .map(|t| SavedTab { url: nice_url(&t.url), title: t.title.clone(), pinned: t.pinned })
                .collect(),
            active: self.index(self.active).unwrap_or(0),
            panel: self.panel_open,
            closed: self.closed.clone(),
        };
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
                if t.level == 1 && secs > self.sleep_after * 60 {
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
                self.sleep_after = v["sleepAfter"].as_u64().unwrap_or(20);
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
                    let _ = t.wv.zoom(v["f"].as_f64().unwrap_or(1.0));
                }
            }
            "revealed" => {
                let id = n("id");
                if let Some(i) = self.index(id) {
                    self.tabs[i].door = None;
                    if id == self.active && !self.overlay {
                        let _ = self.tabs[i].wv.set_visible(true);
                        let _ = self.tabs[i].wv.focus();
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
        if is_ours(src) {
            self.handle_ui(body, target, flow);
            return;
        }
        let Ok(v) = serde_json::from_str::<Value>(body) else { return };
        if v["t"] == "icon" {
            if let (Some(i), Some(h)) = (self.index(id), v["href"].as_str()) {
                if h.starts_with("https://") || h.starts_with("data:image/") {
                    self.tabs[i].icon = h.chars().take(4000).collect();
                    self.changed();
                }
            }
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
        .with_devtools(cfg!(debug_assertions))
        .with_custom_protocol("zenko".into(), protocol)
        .with_ipc_handler(move |req: Request<String>| {
            if is_ours(&req.uri().to_string()) {
                let _ = pu.send_event(Ev::Ui(req.into_body()));
            }
        });
    #[cfg(windows)]
    let cb = cb.with_default_context_menus(false).with_browser_accelerator_keys(false);
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
    };

    // Session restore
    let saved: Session = std::fs::read_to_string(dir.join("session.json"))
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default();
    app.closed = saved.closed.clone();
    let mut ids = vec![];
    for t in saved.tabs.iter().take(60) {
        app.new_tab(&t.url, false);
        if let Some(last) = app.tabs.last_mut() {
            last.title = t.title.clone();
            last.pinned = t.pinned;
            ids.push(last.id);
        }
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
            Event::WindowEvent { event: WindowEvent::Resized(sz), window_id, .. } => {
                if window_id == main_id {
                    let l = sz.to_logical::<f64>(app.window.scale_factor());
                    let _ = app.chrome.set_bounds(rect(Rc4 { x: 0.0, y: 0.0, w: l.width, h: l.height }));
                    app.ui(&format!("zb.maximized({})", app.window.is_maximized()));
                    if app.fullscreen {
                        let r = app.full_rect();
                        if let Some(t) = app.active_tab() {
                            let _ = t.wv.set_bounds(rect(r));
                        }
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
                Ev::Open(u) => app.new_tab(&u, true),
                Ev::Key(k) => app.ui(&format!("zb.key({})", json!(k))),
                Ev::Focus(id) => {
                    if id != u32::MAX && id != app.focused {
                        app.focused = id;
                        app.tell_zenko();
                    }
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
