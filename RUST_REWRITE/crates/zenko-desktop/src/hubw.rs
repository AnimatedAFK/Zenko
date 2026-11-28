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
use zenko_desktop::homepage::HomePage;
use zenko_desktop::youpage::YouPage;
use zenko_desktop::dailypage::{DailyData, DailyPage};
use zenko_desktop::ttpage::{TtData, TtPage};
use zenko_desktop::uppage::{UpData, UpPage};

#[path = "hubup.rs"]
mod hubup;
#[path = "hubrem.rs"]
mod hubrem;
#[path = "hubset.rs"]
mod hubset;
#[path = "hubcust.rs"]
mod hubcust;
#[path = "hubbook.rs"]
mod hubbook;
#[path = "hubnotes.rs"]
mod hubnotes;
#[path = "hubscreen.rs"]
mod hubscreen;
#[path = "hubgym.rs"]
mod hubgym;
#[path = "hubgarden.rs"]
mod hubgarden;
#[path = "paletwin.rs"]
mod paletwin;
use serde_json::Value;
use zenko_desktop::pagekit::Faces;
use zenko_desktop::ui::{Draw, Kind, Place};

/// what the app hands the window when it opens
pub struct HubInit {
    pub t: Theme,
    /// seconds since the app started
    pub opened_s: f64,
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
    /// z19_glow: next redraw (every 120 ms) and the petal burst
    glow_next: f64,
    burst: Option<zenko_desktop::hubwin::GlowBurst>,
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
    /// the Home page, its next once-a-second refresh and hub_intro's count-up (when it started)
    home: Option<HomePage>,
    home_next: f64,
    home_intro: Option<f64>,
    /// music_playing: what the play / pause button shows
    music_playing: bool,
    you: Option<YouPage>,
    daily: Option<DailyPage>,
    /// what the Daily page was last rendered from
    daily_d: DailyData,
    daily_next: f64,
    tt: Option<TtPage>,
    /// what the Timetable page was last drawn from, and when (it redraws every 30 s, like its after(30000))
    tt_d: TtData,
    tt_next: f64,
    /// the Upcoming page, what it was last rendered from, and its hands' next tick / next check for changes
    up: Option<UpPage>,
    up_d: UpData,
    up_next: f64,
    up_chk: f64,
    /// a time menu to pop up once the click is handled
    up_menu: Option<usize>,
    rem: Option<zenko_desktop::rempage::RemPage>,
    /// the Settings page, its try-a-title text, the box that had the keyboard, the new rule's Keep / Close
    set: Option<zenko_desktop::setpage::SetPage>,
    set_try: String,
    set_focus: Option<usize>,
    set_rule_do: String,
    /// the Customize page, the box that had the keyboard, the next preview frame
    cust: Option<zenko_desktop::custpage::CustPage>,
    cust_focus: Option<usize>,
    cust_anim: f64,
    /// the Study books page: the shelf or the open book
    nb: Option<hubbook::Nb>,
    /// the Notes page and its timers
    notes: Option<zenko_desktop::notespage::NotesPage>,
    notes_st: hubnotes::NotesState,
    screen: Option<zenko_desktop::screenpage::ScreenPage>,
    gym: Option<zenko_desktop::gympage::GymPage>,
    /// the Zen garden page; its subject menu to pop, a command waiting for the picture, a catch's card (at, fish, seed,
    /// cm, new) and the badge check after it, the next 50 ms frame, the panel 400 ms after Lock in
    garden: Option<zenko_desktop::gardenpage::GardenPage>,
    gd_menu: bool,
    gd_pending: Option<String>,
    gd_catch: Option<(f64, String, i64, f64, bool)>,
    gd_badge_at: f64,
    gd_next: f64,
    gd_panel_at: f64,
    /// screen_ui["range"] (kept while the window is open) and the next minute render
    screen_range: i64,
    screen_next: f64,
    /// seconds from the app's start to this window opening
    opened_s: f64,
    /// Ctrl+K: a card to glide to (page, title, when), its glow (card, its own items, start) and the next prebuild
    reveal: Option<(String, String, f64)>,
    flash: Option<(usize, usize, f64)>,
    prebuild_at: f64,
    rem_d: zenko_desktop::rempage::RemData,
    rem_anim: f64,
    rem_chk: f64,
    /// a Yes/No question to ask once the click is handled: (text, title, command on Yes)
    ask: Option<(String, String, String)>,
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
    if old.as_deref() == Some("notes") {
        // hub_show: leaving Notes files the scratchpad
        hubnotes::flush(x, false);
    }
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
fn faces(x: &HubW) -> Faces {
    Faces { ui: x.hub.font_ui.clone(), title: String::new() }
}

/// hub_refresh_home with what the app has now; `force` rebuilds the focus card
fn home_refresh(x: &mut HubW, force: bool) {
    let Some(d) = crate::win::with_fox(|fox, _| fox.home_data()) else { return };
    let n = now(x);
    let counting = x.home_intro.is_some_and(|t0| n <= t0 + 0.7);
    let (th, f) = (x.hub.t.clone(), faces(x));
    if let Some(hp) = x.home.as_mut() {
        hp.refresh(&mut x.hub.ui, &th, &f, &d, force, counting);
        if counting {
            let t = ((n - x.home_intro.unwrap()) / 0.65).min(1.0);
            hp.intro_frame(&mut x.hub.ui, &th, &f, &d, 1.0 - (1.0 - t).powi(3));
        }
    }
    relayout(x);
}

/// what the Home page's buttons do
/// hub_ask(q): to the chat page, and ask her
fn hub_ask(x: &mut HubW, q: &str) {
    show(x, "chat");
    let q = q.to_string();
    crate::win::with_fox_at(move |fox, t, win| {
        if let zenko_desktop::chat::Sent::Brain(_) = fox.chat_send(t, &q, win) {
            fox.brain_fallback(t);
        }
    });
}

fn home_cmd(x: &mut HubW, cmd: &str) {
    let Some(hp) = x.home.as_ref() else { return };
    let (te, me) = (hp.task_entry, hp.min_entry);
    let get = |x: &HubW, e: Option<usize>| e.map(|e| x.hub.ui.text_of(e)).unwrap_or_default();
    if let Some(q) = cmd.strip_prefix("hub_ask:") {
        hub_ask(x, q);
        return;
    }
    if let Some(v) = cmd.strip_prefix("home_min:") {
        if let Some(e) = me
            && let Kind::Entry { e, .. } = &mut x.hub.ui.ws[e].kind {
            e.text = v.encode_utf16().collect();
            (e.pos, e.anchor, e.sel, e.left) = (e.text.len(), e.text.len(), None, 0);
        }
        return;
    }
    let (task, mins) = (get(x, te), get(x, me));
    let done = crate::win::with_fox(|fox, t| match cmd {
        "home_go" => {
            // go(): 1..240 minutes (25 when it isn't a number), "Focus session" when nothing was typed
            let n = mins.trim().parse::<f64>().ok().filter(|v| v.is_finite()).map(|v| (v.trunc() as i64).clamp(1, 240)).unwrap_or(25);
            let name = if task.trim().is_empty() { "Focus session".to_string() } else { task.trim().to_string() };
            let tt = task.trim().to_string();
            zenko_desktop::update_config(&mut fox.cfg, |v| {
                v["last_task"] = tt.clone().into();
                v["last_minutes"] = n.into();
            });
            fox.start_session(t, &name, n as f64);
            true
        }
        "home_start_item" => {
            if let Some(i) = fox.focus_item() {
                let (task, m) = (fox.tt[i]["task"].as_str().unwrap_or("").to_string(), fox.tt[i]["mins"].as_f64().unwrap_or(30.0));
                fox.start_session_item(t, &task, m, Some(i));
            }
            true
        }
        "toggle_pause" => {
            fox.toggle_pause();
            true
        }
        "end_session" => {
            fox.end_session(t, false);
            true
        }
        "start_break" => {
            fox.start_break(t, None);
            true
        }
        "end_break" => {
            fox.end_break(t, false);
            true
        }
        _ => false,
    }).unwrap_or(false);
    if done {
        home_refresh(x, false);
    }
}

/// the About you page: save (the button, Return in a field, or leaving a field), the two switches, Forget everything
fn you_cmd(x: &mut HubW, cmd: &str) {
    let Some(yp) = x.you.as_ref() else { return };
    match cmd {
        "you_save" => {
            let vals = yp.values(&x.hub.ui);
            let short = crate::win::with_fox(|fox, _| {
                zenko_desktop::update_config(&mut fox.cfg, |v| {
                    for (k, s) in &vals {
                        v[*k] = s.clone().into();
                    }
                });
                fox.cfg.raw.get("user_name").and_then(|v| v.as_str()).unwrap_or("").trim().split(' ').next()
                    .unwrap_or("").to_string()
            }).unwrap_or_default();
            let msg = if short.is_empty() { "Saved. ".to_string() } else { format!("Saved. Nice to meet you, {short}!") };
            yp.set_status(&mut x.hub.ui, &msg);
        }
        "you_learn" | "you_style" => {
            let (id, key) = if cmd == "you_learn" { (yp.learn, "learn_habits") } else { (yp.style, "learn_style") };
            let on = YouPage::checked(&x.hub.ui, id);
            crate::win::with_fox(|fox, _| zenko_desktop::update_config(&mut fox.cfg, |v| v[key] = on.into()));
        }
        "you_forget" => {
            zenko_desktop::chat::forget_reply("everything");
            yp.set_info(&mut x.hub.ui, &zenko_desktop::chat::learned_summary());
            crate::win::with_fox(|fox, _| fox.say("Okay, I've forgotten everything I learned. Fresh start!", 5.0));
        }
        _ => return,
    }
    relayout(x);
}

/// daily_render() with the app's data now
fn daily_render(x: &mut HubW) {
    let d = crate::win::with_fox(|fox, _| fox.daily_data()).unwrap_or_default();
    let (th, f) = (x.hub.t.clone(), faces(x));
    let cm10 = (100.0 * x.dpi / 25.4 + 0.5) as i32;
    if let Some(dp) = x.daily.as_mut() {
        dp.render(&mut x.hub.ui, &th, &f, cm10, &d);
    }
    x.daily_d = d;
    relayout(x);
}

/// daily_refresh(): the page shows what changed, unless she's typing in one of its fields (then on the next show)
fn daily_refresh(x: &mut HubW) {
    if x.daily.is_none() || x.hub.side.page.as_deref() != Some("daily") {
        return;
    }
    let d = crate::win::with_fox(|fox, _| fox.daily_data()).unwrap_or_default();
    let o = &x.daily_d;
    if (&d.life, d.streak, d.due, &d.cfg) == (&o.life, o.streak, o.due, &o.cfg) {
        return;
    }
    let typing = x.hub.ui.focus.is_some_and(|f| matches!(x.hub.ui.ws[f].kind, Kind::Entry { .. } | Kind::Text { .. }));
    if !typing {
        daily_render(x);
    }
}

/// the Daily page's buttons
fn daily_cmd(x: &mut HubW, cmd: &str) {
    let Some(dp) = x.daily.as_ref() else { return };
    let ui = &x.hub.ui;
    let text_of = |id: usize| match &ui.ws[id].kind {
        Kind::Text { t, .. } => t.get(),
        _ => ui.text_of(id),
    };
    let (note, q, a, hw, fact) = (text_of(dp.note), text_of(dp.q), text_of(dp.a), text_of(dp.hw), text_of(dp.fact));
    if let Some(m) = cmd.strip_prefix("daily_mood:") {
        let m = m.to_string();
        crate::win::with_fox(move |fox, t| fox.checkin_save(t, &m, &note));
        daily_render(x);
    } else if cmd == "daily_card_add" {
        if !q.trim().is_empty() && !a.trim().is_empty() {
            crate::win::with_fox(move |fox, _| {
                let s = fox.card_add(&q, &a);
                fox.say(&s, 6.0);
            });
            daily_refresh(x);
        }
    } else if cmd == "daily_breathe" {
        crate::win::with_fox(|fox, _| fox.events.push(zenko_desktop::Ev::Command("open_breathing".into())));
    } else if cmd == "daily_checkin" {
        crate::win::with_fox(|fox, _| fox.events.push(zenko_desktop::Ev::Command("open_checkin_force".into())));
    } else if cmd == "daily_homework" {
        let q = hw.trim().to_string();
        if !q.is_empty() {
            hub_ask(x, &format!("homework: {q}"));
        }
    } else if cmd == "daily_fact_add" {
        let s = fact.trim().to_string();
        if !s.is_empty() {
            let low = s.to_lowercase();
            let s = if low.starts_with("my ") || low.starts_with("remember") { s } else { format!("remember {s}") };
            crate::win::with_fox(move |fox, t| {
                let r = fox.life_route(t, &s).unwrap_or_else(|| "Noted~".into());
                fox.say(&r, 6.0);
            });
            daily_render(x);
        }
    } else if let Some(i) = cmd.strip_prefix("daily_forget:") {
        let Some(what) = i.parse::<usize>().ok().and_then(|i| dp.forget.get(i)).map(|f| f.1.clone()) else { return };
        crate::win::with_fox(move |fox, _| fox.life_do(|l| {
            if let Some(k) = what.strip_prefix("fact:") {
                if let Some(o) = l["facts"].as_object_mut() {
                    o.remove(k);
                }
            } else if let Some(n) = what.strip_prefix("note:")
                && let Some(v) = l["notes"].as_array_mut()
                && let Some(p) = v.iter().position(|e| e.as_str() == Some(n)) {
                v.remove(p);
            }
        }));
        daily_render(x);
    } else if let Some(v) = cmd.strip_prefix("daily_badges:") {
        let v = v.to_string();
        let v2 = v.clone();
        crate::win::with_fox(move |fox, _| zenko_desktop::update_config(&mut fox.cfg, |c| c["badge_view"] = v2.clone().into()));
        x.daily_d.cfg["badge_view"] = v.clone().into();
        let (th, f) = (x.hub.t.clone(), faces(x));
        let d = x.daily_d.clone();
        if let Some(dp) = x.daily.as_mut() {
            dp.set_mode(&mut x.hub.ui, &th, &f, &d, &v);
        }
        relayout(x);
    } else if cmd == "open_hub:notebook" {
        show(x, "notebook");
    }
}

/// tt_refresh() with the app's timetable now
fn tt_refresh(x: &mut HubW) {
    let d = crate::win::with_fox(|fox, _| fox.tt_data()).unwrap_or_default();
    let (th, f) = (x.hub.t.clone(), faces(x));
    if let Some(tp) = x.tt.as_mut() {
        tp.refresh(&mut x.hub.ui, &th, &f, &d);
    }
    x.tt_d = d;
    x.tt_next = now(x) + 30.0;
    relayout(x);
}

/// the Timetable page's buttons and Return keys
fn tt_cmd(x: &mut HubW, cmd: &str) {
    let Some(tp) = x.tt.as_ref() else { return };
    let item = |p: &str| cmd.strip_prefix(p).map(|k| k.trim_end_matches(":yes")).and_then(|k| k.parse::<usize>().ok())
        .and_then(|k| tp.rows.get(k).copied());
    let day = zenko_desktop::plan::iso(x.tt_d.today + tp.day);
    if let Some(d) = cmd.strip_prefix("tt_nav:") {
        let d: i64 = d.parse().unwrap_or(0);
        if let Some(tp) = x.tt.as_mut() {
            tp.day = if d == 0 { 0 } else { tp.day + d };
        }
        tt_refresh(x);
    } else if let Some(i) = item("tt_start:") {
        crate::win::with_fox(move |fox, t| {
            if fox.session.is_some() {
                fox.say("Finish or stop the current session first.", 5.0);
            } else if i < fox.tt.len() {
                let it = fox.tt[i].clone();
                let task = it["task"].as_str().map(String::from).unwrap_or_else(|| it["task"].to_string());
                fox.start_session_item(t, &task, it["mins"].as_f64().unwrap_or(30.0), Some(i));
            }
        });
        tt_refresh(x);
    } else if let Some(i) = item("tt_mark:") {
        let done = x.tt_d.tt.get(i).is_some_and(|it| zenko_desktop::tt::tt_done(it, &day));
        crate::win::with_fox(move |fox, t| fox.tt_mark_day(t, i, !done, &day));
        tt_refresh(x);
    } else if let Some(i) = item("tt_delete:") {
        let daily = x.tt_d.tt.get(i).and_then(|it| it.get("daily")).is_some_and(|v| v.as_bool().unwrap_or(!v.is_null()));
        if daily && !cmd.ends_with(":yes") {
            let k = cmd.strip_prefix("tt_delete:").unwrap_or("").to_string();
            x.ask = Some(("This item repeats every day.\nDelete it from all days?".into(), "Delete".into(), format!("tt_delete:{k}:yes")));
            return;
        }
        let it = x.tt_d.tt.get(i).cloned();
        crate::win::with_fox(move |fox, _| {
            if let Some(it) = it
                && let Some(p) = fox.tt.iter().position(|v| *v == it) {
                fox.tt.remove(p);
            }
            zenko_desktop::tt::save_tt(&fox.tt);
        });
        tt_refresh(x);
    } else if let Some(v) = cmd.strip_prefix("tt_quick:") {
        let me = tp.me;
        if let Kind::Entry { e, .. } = &mut x.hub.ui.ws[me].kind {
            e.text = v.encode_utf16().collect();
            (e.pos, e.anchor, e.sel, e.left) = (e.text.len(), e.text.len(), None, 0);
        }
    } else if cmd == "tt_add" {
        let ui = &x.hub.ui;
        let (t, k, m) = (ui.text_of(tp.te), ui.text_of(tp.ke), ui.text_of(tp.me));
        let every = matches!(ui.ws[tp.every].kind, Kind::Check { on: true, .. });
        let st = zenko_desktop::tt::parse_clock(&t);
        if st.is_none() || k.trim().is_empty() {
            tp.set_msg(&mut x.hub.ui, "Enter a time like 2pm or 14:30, and what you're doing.");
            relayout(x);
            return;
        }
        // int(float(m)), 1..480; 30 when it isn't a number
        let ln = m.trim().parse::<f64>().ok().filter(|v| v.is_finite()).map(|v| (v.trunc() as i64).clamp(1, 480)).unwrap_or(30);
        let item = serde_json::json!({"start": st.unwrap(), "mins": ln, "task": k.trim(), "daily": every, "date": day, "done": []});
        crate::win::with_fox(move |fox, _| {
            fox.tt.push(item);
            zenko_desktop::tt::save_tt(&fox.tt);
        });
        let (te, ke, msg) = (tp.te, tp.ke, tp.msg);
        for e in [te, ke] {
            if let Kind::Entry { e, .. } = &mut x.hub.ui.ws[e].kind {
                e.text.clear();
                (e.pos, e.anchor, e.sel, e.left) = (0, 0, None, 0);
            }
        }
        let _ = msg;
        tp.set_msg(&mut x.hub.ui, "");
        tt_refresh(x);
        x.hub.ui.focus = Some(te);
    } else if cmd == "tt_delay" {
        crate::win::with_fox(|fox, _| fox.events.push(zenko_desktop::Ev::Command("delay_picker".into())));
    } else if cmd == "tt_export" {
        crate::win::with_fox(|fox, _| fox.events.push(zenko_desktop::Ev::Command("export_pdf".into())));
    }
}

/// the badge shelf under the mouse: the tip line says how to earn the plaque (a hand over earned ones)
fn daily_hover(x: &mut HubW, mx: i32, my: i32) -> bool {
    let Some(dp) = x.daily.as_ref() else { return false };
    if x.hub.side.page.as_deref() != Some("daily") {
        return false;
    }
    let cv = dp.badges_cv;
    let (cx, cy) = x.hub.ui.abs(cv);
    let inside = mx >= cx && my >= cy && mx < cx + x.hub.ui.ws[cv].w && my < cy + x.hub.ui.ws[cv].h;
    let bid = if inside { dp.badge_at((mx - cx) as f64, (my - cy) as f64) } else { None };
    let msg = match &bid {
        Some(b) => {
            let m = zenko_desktop::rewards::metrics(&x.daily_d.life);
            DailyPage::badge_info(&x.daily_d, b, &m)
        }
        None => "Hover a plaque to see how to earn it.".into(),
    };
    let ch = dp.set_tip(&mut x.hub.ui, &msg);
    if bid.as_ref().is_some_and(|b| x.daily_d.life["badges"].get(b).is_some()) {
        x.cursor = IDC_HAND;
    }
    if ch {
        relayout(x);
    }
    ch
}

/// a click on an earned plaque pins it to the sidebar (three at most) or unpins it
fn daily_badge_click(x: &mut HubW, mx: i32, my: i32) {
    let Some(dp) = x.daily.as_ref() else { return };
    let cv = dp.badges_cv;
    let (cx, cy) = x.hub.ui.abs(cv);
    if mx < cx || my < cy || mx >= cx + x.hub.ui.ws[cv].w || my >= cy + x.hub.ui.ws[cv].h {
        return;
    }
    let Some(bid) = dp.badge_at((mx - cx) as f64, (my - cy) as f64) else { return };
    if x.daily_d.life["badges"].get(&bid).is_none() {
        return;
    }
    let b2 = bid.clone();
    let pins = crate::win::with_fox(move |fox, _| fox.life_do(|l| {
        let mut p: Vec<Value> = l["badge_pins"].as_array().cloned().unwrap_or_default();
        if let Some(i) = p.iter().position(|v| v.as_str() == Some(b2.as_str())) {
            p.remove(i);
        } else {
            if p.len() >= 3 {
                p.remove(0);
            }
            p.push(b2.clone().into());
        }
        l["badge_pins"] = Value::Array(p.clone());
        p
    })).unwrap_or_default();
    out("sound:tap");
    x.daily_d.life["badge_pins"] = Value::Array(pins);
    let (th, f) = (x.hub.t.clone(), faces(x));
    let d = x.daily_d.clone();
    if let Some(dp) = x.daily.as_mut() {
        dp.draw_badges(&mut x.hub.ui, &th, &f, &d);
    }
    relayout(x);
}

/// <FocusOut> on a profile field saves it
fn you_focus_left(x: &mut HubW, old: Option<usize>) {
    let Some(yp) = x.you.as_ref() else { return };
    if old.is_some_and(|o| x.hub.ui.focus != Some(o) && yp.fields.iter().any(|f| f.1 == o)) {
        you_cmd(x, "you_save");
    }
}

/// the music card's round buttons (labels with pictures): previous, play / pause, next
fn home_music_click(x: &mut HubW, mx: i32, my: i32) -> bool {
    let Some(id) = x.hub.ui.hit(mx, my) else { return false };
    let Some(k) = x.hub.ui.ws[id].name.strip_prefix("music:").map(String::from) else { return false };
    out("sound:tap");
    let key = match k.as_str() {
        "prev" => "previous",
        "next" => "next",
        _ => "playpause",
    };
    crate::win::with_fox(|fox, _| fox.events.push(zenko_desktop::Ev::Media(key.into())));
    if k == "pp" {
        x.music_playing = !x.music_playing;
    }
    let playing = x.music_playing;
    if let Some(hp) = x.home.as_ref() {
        let img = if playing { hp.icons.pause.0.clone() } else { hp.icons.play.0.clone() };
        if let Kind::Label { image, .. } = &mut x.hub.ui.ws[hp.pp].kind {
            *image = Some(img);
        }
    }
    true
}

fn after_show(x: &mut HubW, page: &str) {
    if page == "home" {
        x.home_intro = Some(now(x));
        home_refresh(x, true);
    }
    if page == "timetable" && x.tt.is_some() {
        // hub_after_show: tt_refresh()
        tt_refresh(x);
    }
    if page == "deadlines" && x.up.is_some() {
        hubup::refresh(x);
    }
    if page == "reminders" && x.rem.is_some() {
        hubrem::render(x);
    }
    if page == "daily" && x.daily.is_some() {
        // (zenko.py renders on show when something changed while it was hidden)
        let d = crate::win::with_fox(|fox, _| fox.daily_data()).unwrap_or_default();
        if d.life != x.daily_d.life || d.streak != x.daily_d.streak || d.due != x.daily_d.due {
            daily_render(x);
        }
    }
    if page == "screen" && x.screen.is_some() {
        // hub_after_show: screen_render() once the page has slid in
        hubscreen::render(x);
    }
    if page == "garden" && x.garden.is_some() {
        hubgarden::shown(x);
    }
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
        "home" => {
            let th = x.hub.t.clone();
            let pg = x.hub.ui.frame(x.hub.ids.stage, &th.paper);
            let cm10 = (100.0 * x.dpi / 25.4 + 0.5) as i32;
            let d = crate::win::with_fox(|fox, _| fox.home_data()).unwrap_or_default();
            let f = faces(x);
            x.home = Some(HomePage::build(&mut x.hub.ui, pg, &th, &f, cm10, &d));
            pg
        }
        "timetable" => {
            let th = x.hub.t.clone();
            let pg = x.hub.ui.frame(x.hub.ids.stage, &th.paper);
            let (d, icon) = crate::win::with_fox(|fox, t| {
                let im = fox.small_fox(t);
                (fox.tt_data(), std::rc::Rc::new(zenko_desktop::tkcanvas::Photo { w: im.w, h: im.h, px: im.px }))
            }).unwrap_or_else(|| (TtData::default(), std::rc::Rc::new(zenko_desktop::tkcanvas::Photo { w: 1, h: 1, px: vec![[0; 4]] })));
            let f = faces(x);
            x.tt = Some(TtPage::build(&mut x.hub.ui, pg, &th, &f, icon, &d));
            x.tt_d = d;
            x.tt_next = now(x) + 30.0;
            pg
        }
        "deadlines" => {
            let th = x.hub.t.clone();
            let pg = x.hub.ui.frame(x.hub.ids.stage, &th.paper);
            hubup::build(x, pg);
            pg
        }
        "reminders" => {
            let th = x.hub.t.clone();
            let pg = x.hub.ui.frame(x.hub.ids.stage, &th.paper);
            hubrem::build(x, pg);
            pg
        }
        "settings" => {
            let th = x.hub.t.clone();
            let pg = x.hub.ui.frame(x.hub.ids.stage, &th.paper);
            hubset::build(x, pg);
            pg
        }
        "customize" => {
            let th = x.hub.t.clone();
            let pg = x.hub.ui.frame(x.hub.ids.stage, &th.paper);
            hubcust::build(x, pg);
            pg
        }
        "notes" => {
            let th = x.hub.t.clone();
            let pg = x.hub.ui.frame(x.hub.ids.stage, &th.paper);
            hubnotes::build(x, pg);
            pg
        }
        "exercise" => {
            let th = x.hub.t.clone();
            let pg = x.hub.ui.frame(x.hub.ids.stage, &th.paper);
            hubgym::build(x, pg);
            pg
        }
        "garden" => {
            let th = x.hub.t.clone();
            let pg = x.hub.ui.frame(x.hub.ids.stage, &th.paper);
            hubgarden::build(x, pg);
            pg
        }
        "screen" => {
            let th = x.hub.t.clone();
            let pg = x.hub.ui.frame(x.hub.ids.stage, &th.paper);
            hubscreen::build(x, pg);
            pg
        }
        "notebook" => {
            let th = x.hub.t.clone();
            let pg = x.hub.ui.frame(x.hub.ids.stage, &th.paper);
            hubbook::build(x, pg);
            pg
        }
        "daily" => {
            let th = x.hub.t.clone();
            let pg = x.hub.ui.frame(x.hub.ids.stage, &th.paper);
            let cm10 = (100.0 * x.dpi / 25.4 + 0.5) as i32;
            let d = crate::win::with_fox(|fox, _| fox.daily_data()).unwrap_or_default();
            let f = faces(x);
            x.daily = Some(DailyPage::build(&mut x.hub.ui, pg, &th, &f, cm10, &d));
            x.daily_d = d;
            pg
        }
        "you" => {
            let th = x.hub.t.clone();
            let pg = x.hub.ui.frame(x.hub.ids.stage, &th.paper);
            let cm10 = (100.0 * x.dpi / 25.4 + 0.5) as i32;
            let cfg = crate::win::with_fox(|fox, _| fox.cfg.raw.clone()).unwrap_or_default();
            let f = faces(x);
            let learned = zenko_desktop::chat::learned_summary();
            x.you = Some(YouPage::build(&mut x.hub.ui, pg, &th, &f, cm10, &cfg, &learned));
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
    if let Some(yp) = x.you.as_mut() {
        let (th, f) = (x.hub.t.clone(), Faces { ui: x.hub.font_ui.clone(), title: String::new() });
        yp.fit(&mut x.hub.ui, &x.fonts, &th, &f, x.hub.zk_f, w, h);
        let sc = yp.sc.clone();
        let y = x.hub.ui.ws[sc.cv].yoff;
        sc.set_y(&mut x.hub.ui, y);
        x.hub.ui.layout_at(&x.fonts, w, h);
        sc.draw_bar(&mut x.hub.ui, false, &th);
    }
    if let Some(tp) = x.tt.as_mut() {
        tp.fit(&mut x.hub.ui, &x.fonts, x.hub.zk_f, w, h);
        let sc = tp.sc.clone();
        let y = x.hub.ui.ws[sc.cv].yoff;
        sc.set_y(&mut x.hub.ui, y);
        x.hub.ui.layout_at(&x.fonts, w, h);
        let th = x.hub.t.clone();
        sc.draw_bar(&mut x.hub.ui, false, &th);
    }
    hubup::fit(x, w, h);
    hubrem::fit(x, w, h);
    hubset::fit(x, w, h);
    hubcust::fit(x, w, h);
    hubbook::fit(x, w, h);
    hubnotes::fit(x, w, h);
    hubscreen::fit(x, w, h);
    hubgym::fit(x, w, h);
    hubgarden::fit(x, w, h);
    if let Some(dp) = x.daily.as_mut() {
        let (th, f) = (x.hub.t.clone(), Faces { ui: x.hub.font_ui.clone(), title: String::new() });
        let d = x.daily_d.clone();
        dp.fit(&mut x.hub.ui, &x.fonts, &th, &f, &d, x.hub.zk_f, w, h);
        let sc = dp.sc.clone();
        let y = x.hub.ui.ws[sc.cv].yoff;
        sc.set_y(&mut x.hub.ui, y);
        x.hub.ui.layout_at(&x.fonts, w, h);
        sc.draw_bar(&mut x.hub.ui, false, &th);
    }
    if let Some(hp) = x.home.as_mut() {
        let (th, f) = (x.hub.t.clone(), Faces { ui: x.hub.font_ui.clone(), title: String::new() });
        hp.fit(&mut x.hub.ui, &x.fonts, &th, &f, x.hub.zk_f, w, h);
        let sc = hp.sc.clone();
        let y = x.hub.ui.ws[sc.cv].yoff;
        sc.set_y(&mut x.hub.ui, y);
        x.hub.ui.layout_at(&x.fonts, w, h);
        sc.draw_bar(&mut x.hub.ui, false, &th);
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
/// the wheel over the Home page scrolls it
/// a page's scroller
fn page_sc(x: &HubW, page: Option<&str>) -> Option<zenko_desktop::pagekit::Scroller> {
    match page {
        Some("home") => x.home.as_ref().map(|p| p.sc.clone()),
        Some("you") => x.you.as_ref().map(|p| p.sc.clone()),
        Some("daily") => x.daily.as_ref().map(|p| p.sc.clone()),
        Some("timetable") => x.tt.as_ref().map(|p| p.sc.clone()),
        Some("deadlines") => x.up.as_ref().map(|p| p.sc.clone()),
        Some("reminders") => x.rem.as_ref().map(|p| p.sc.clone()),
        Some("settings") => x.set.as_ref().map(|p| p.sc.clone()),
        Some("customize") => x.cust.as_ref().map(|p| p.sc.clone()),
        Some("screen") => x.screen.as_ref().map(|p| p.sc.clone()),
        Some("exercise") => x.gym.as_ref().map(|p| p.sc.clone()),
        Some("garden") => x.garden.as_ref().map(|p| p.sc.clone()),
        _ => None,
    }
}

fn page_wheel(x: &mut HubW, mx: i32, my: i32, delta: i32) -> bool {
    if x.hub.side.page.as_deref() == Some("notes") {
        return hubnotes::wheel(x, mx, my, delta);
    }
    if x.hub.side.page.as_deref() == Some("notebook") {
        return hubbook::scrollers(x).into_iter().any(|sc| sc_wheel(x, sc, mx, my, delta));
    }
    let Some(sc) = page_sc(x, x.hub.side.page.as_deref()) else { return false };
    sc_wheel(x, sc, mx, my, delta)
}

fn sc_wheel(x: &mut HubW, sc: zenko_desktop::pagekit::Scroller, mx: i32, my: i32, delta: i32) -> bool {
    let mut over = x.hub.ui.hit(mx, my);
    while let Some(i) = over {
        if i == sc.boxf {
            break;
        }
        over = x.hub.ui.ws[i].parent;
    }
    if over.is_none() {
        return false;
    }
    if sc.wheel(&mut x.hub.ui, delta) {
        let (w, h) = x.size;
        x.hub.layout(&x.fonts, w, h);
        let th = x.hub.t.clone();
        sc.draw_bar(&mut x.hub.ui, false, &th);
        return true;
    }
    false
}

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
            theme: init.theme.clone(), living: init.living, glow_next: 0.0, burst: None, particles: init.particles, minimized: false,
            slide_s: init.slide_s, dpi: init.dpi, pages: vec![], slide: None, chat: None, chat_seen: None, home: None, home_next: 0.0, home_intro: None, music_playing: false, you: None, daily: None, daily_d: DailyData::default(), daily_next: 0.0, tt: None, tt_d: TtData::default(), tt_next: 0.0, up: None, up_d: UpData::default(), up_next: 0.0, up_chk: 0.0, up_menu: None, rem: None, rem_d: Default::default(), rem_anim: 0.0, rem_chk: 0.0, set: None, set_try: String::new(), set_focus: None, set_rule_do: String::new(), cust: None, cust_focus: None, cust_anim: 0.0, nb: None, notes: None, notes_st: Default::default(), screen: None, gym: None, garden: None, gd_menu: false, gd_pending: None, gd_catch: None, gd_badge_at: 0.0, gd_next: 0.0, gd_panel_at: 0.0, screen_range: 1, screen_next: 0.0, opened_s: init.opened_s, reveal: None, flash: None, prebuild_at: 2.5, ask: None,
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
    paletwin::close();
    with(hubnotes::closing);
    let h = HUB.with(|v| v.borrow_mut().take());
    if let Some(x) = h {
        unsafe { DestroyWindow(x.hwnd) };
        crate::win::with_fox(|fox, _| fox.hub_page = None);
        out("closed");
    }
}

/// quit: the window closed (its Notes page saved and filed), or the scratchpad's draft filed on its own
pub fn quit() {
    if is_open() {
        close();
    } else {
        hubnotes::quit_flush();
    }
}

/// the app's garden commands (chat, the menu): breath:k, grounding, fish_cast, koi_book
pub fn garden_cmd(c: &str) {
    with(|x| hubgarden::app_cmd(x, c));
}

/// the koi book changed who swims in the pond: the garden paints it again
pub fn garden_pond_changed() {
    with(|x| {
        if let Some(p) = x.garden.as_mut() {
            p.forget();
        }
        let (w, h) = x.size;
        hubgarden::fit(x, w, h);
    });
}

/// food_fix_grams' answer (from its little dialog)
pub fn gym_fix(i: usize, ans: &str) {
    with(|x| hubgym::fix_grams(x, i, ans));
}

/// the Notes page's New folder name (from its little dialog)
pub fn notes_new_folder(name: &str) {
    hubnotes::new_folder(name);
}

/// Import a folder: the folder picked, brought in (then asked about keeping it in step)
pub fn notes_import_ask() {
    let hwnd = with(|x| x.hwnd).unwrap_or(std::ptr::null_mut());
    if let Some(d) = unsafe { hubnotes::pick_folder(hwnd) } {
        hubnotes::import_run(&d, true, false);
    }
}

/// nb_watch_tick's import of a kept folder while the window is open
pub fn notes_import(d: &std::path::Path, quiet: bool) {
    hubnotes::import_run(d, false, quiet);
}

pub fn notes_watch_add(d: &str) {
    hubnotes::watch_add(d);
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
    if palette_tick(x, n) {
        unsafe { InvalidateRect(x.hwnd, std::ptr::null(), 0) };
    }
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
    if n >= x.glow_next {
        // z19_hub_loop's z19_glow, every 120 ms
        x.glow_next = n + 0.12;
        let gold = x.ctx.t.gold.clone();
        if let Some((sess, b0, t)) = crate::win::with_fox(|fox, t| {
            let sess = fox.session.as_ref().filter(|_| fox.cfg.b("focus_glow", true)).map(|s| (s.elapsed, s.planned, s.paused));
            let b0 = fox.glow_burst.take().map(|t0| {
                let mut r: Vec<f64> = (0..46 * 4).map(|_| fox.rand()).collect();
                zenko_desktop::hubwin::GlowBurst::new(t0, &gold, &mut || r.pop().unwrap_or(0.5))
            });
            (sess, b0, t)
        }) {
            if b0.is_some() {
                x.burst = b0;
            }
            let (w, h) = x.size;
            if !x.hub.glow_draw(w, h, t, sess, x.burst.as_ref()) {
                x.burst = None;
            }
        }
    }
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
    // the Timetable page: its 30 s redraw, and tt_changed when the timetable changes elsewhere (chat, sessions)
    if x.hub.side.page.as_deref() == Some("timetable") && x.tt.is_some() && x.slide.is_none() {
        let changed = n >= x.daily_next && crate::win::with_fox(|fox, _| fox.tt != x.tt_d.tt).unwrap_or(false);
        if n >= x.tt_next || changed {
            tt_refresh(x);
        }
    }
    // the Upcoming page: the clock's hands every second, and up_refresh when the plan or deadlines change
    if x.hub.side.page.as_deref() == Some("deadlines") && x.up.is_some() && x.slide.is_none() {
        if n >= x.up_next {
            x.up_next = n + 1.0;
            hubup::tick(x);
        }
        if n >= x.up_chk {
            x.up_chk = n + 2.0;
            hubup::refresh(x);
        }
    }
    // the Reminders page: the foxfire drifts, ema_render when the emas or today's plan change
    if x.slide.is_none() {
        hubrem::step(x, n);
        hubset::step(x);
        hubcust::step(x, n);
        hubbook::step(x, n);
    }
    hubnotes::step(x, n);
    hubscreen::step(x, n);
    hubgym::step(x, n);
    hubgarden::tick(x, n);
    hubgarden::step(x, n);
    // daily_refresh: what changed (a badge, a check-in, a card) shows while the page is open
    if x.hub.side.page.as_deref() == Some("daily") && x.slide.is_none() && n >= x.daily_next {
        x.daily_next = n + 1.0;
        daily_refresh(x);
    }
    // hub_tick (Home: once a second) and hub_intro's count-up (every 28 ms for 0.65 s)
    if x.hub.side.page.as_deref() == Some("home") && x.home.is_some() && x.slide.is_none() {
        let counting = x.home_intro.is_some_and(|t0| n <= t0 + 0.75);
        if n >= x.home_next || counting {
            x.home_next = n + 1.0;
            home_refresh(x, false);
        }
    }
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
                    daily_hover(x, mx, my);
                    hubup::motion(x, mx, my);
                    hubrem::motion(x, mx, my);
                    hubbook::motion(x, mx, my);
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
                            let old = x.hub.ui.focus;
                            x.hub.ui.press(&x.fonts, mx, my, n, msg == WM_LBUTTONDBLCLK);
                            you_focus_left(x, old);
                            hubgym::focus_left(x, old);
                            hubup::focus_left(x, old);
                            hubup::press(x, mx, my);
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
                if let Some(m) = with(|x| x.up_menu.take()).flatten() {
                    ReleaseCapture();
                    hubup::popup(hwnd, m);
                    redraw();
                }
                if let Some(m) = with(|x| {
                    // a Settings menu button under the pointer pops its choices (the OptionMenu)
                    if x.hub.side.page.as_deref() != Some("settings") {
                        return None;
                    }
                    let (mx, my) = pt();
                    let hit = x.hub.ui.hit(mx, my)?;
                    matches!(&x.hub.ui.ws[hit].kind, Kind::Menu { cmd, .. } if cmd.starts_with("set_menu:")).then_some(hit)
                }).flatten() {
                    ReleaseCapture();
                    hubset::popup(hwnd, m);
                    redraw();
                }
                let (mx, my) = pt();
                if let Some(m) = with(|x| hubbook::menu_at(x, mx, my)).flatten() {
                    ReleaseCapture();
                    hubbook::popup(hwnd, m);
                    redraw();
                }
                0
            }
            WM_LBUTTONUP => {
                ReleaseCapture();
                let (mx, my) = pt();
                let mut act: Option<&'static str> = None;
                with(|x| {
                    sb_event(x, mx, my, "release");
                    hubup::release(x);
                    if let Some(cmd) = x.hub.ui.release(mx, my) {
                        out("sound:click");
                        if cmd.starts_with("you_") {
                            you_cmd(x, &cmd);
                        } else if cmd.starts_with("daily_") || cmd == "open_hub:notebook" {
                            daily_cmd(x, &cmd);
                        } else if cmd.starts_with("tt_") {
                            tt_cmd(x, &cmd);
                        } else if cmd.starts_with("up") || cmd.starts_with("ib_") {
                            hubup::cmd(x, &cmd);
                        } else if cmd.starts_with("rem_") {
                            hubrem::cmd(x, &cmd);
                        } else if cmd.starts_with("cz_") {
                            hubcust::cmd(x, &cmd);
                        } else if cmd.starts_with("bk:") {
                            hubbook::cmd(x, &cmd);
                        } else if cmd.starts_with("notes_") {
                            hubnotes::cmd(x, &cmd);
                        } else if cmd.starts_with("scr_") {
                            hubscreen::cmd(x, &cmd);
                        } else if cmd.starts_with("gym_") {
                            hubgym::cmd(x, &cmd);
                        } else if cmd.starts_with("gd_") {
                            hubgarden::cmd(x, &cmd);
                        } else if cmd.starts_with("set_") && !cmd.starts_with("set_menu:") {
                            hubset::cmd(x, &cmd);
                        } else if cmd.starts_with("hub_ask:") || cmd.starts_with("home_") || matches!(cmd.as_str(),
                            "toggle_pause" | "end_session" | "start_break" | "end_break") {
                            home_cmd(x, &cmd);
                        } else {
                            chat_cmd(x, &cmd);
                        }
                    } else if hubrem::click(x, mx, my) {
                    } else if hubset::click(x, mx, my) {
                    } else if hubcust::click(x, mx, my) {
                    } else if hubbook::click(x, mx, my) {
                    } else if hubgym::click(x, mx, my) {
                    } else if hubgarden::click(x, mx, my) {
                    } else if x.hub.side.page.as_deref() == Some("home") {
                        home_music_click(x, mx, my);
                    } else if x.hub.side.page.as_deref() == Some("daily") {
                        daily_badge_click(x, mx, my);
                    }
                    let moved = x.drag.is_some() && x.drag_moved;
                    x.rs = None;
                    x.drag = None;
                    let part = x.hub.part_at(mx, my);
                    match part {
                        "min" | "max" | "close" => act = Some(part),
                        "pill" if !moved => palette_open(x),
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
                // a question a button asked (messagebox.askyesno), once the click is done with
                if with(|x| std::mem::take(&mut x.gd_menu)).unwrap_or(false) {
                    hubgarden::subj_popup(hwnd);
                    redraw();
                }
                if let Some((text, title, cmd)) = with(|x| x.ask.take()).flatten() {
                    let w = |s: &str| s.encode_utf16().chain(std::iter::once(0)).collect::<Vec<u16>>();
                    let (t, c) = (w(&text), w(&title));
                    if MessageBoxW(hwnd, t.as_ptr(), c.as_ptr(), MB_YESNO | MB_ICONQUESTION) == IDYES {
                        with(|x| if cmd.starts_with("notes_") {
                            hubnotes::cmd(x, &cmd)
                        } else if cmd.starts_with("scr_") {
                            hubscreen::cmd(x, &cmd)
                        } else {
                            tt_cmd(x, &cmd)
                        });
                    }
                    redraw();
                }
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
                if with(|x| chat_wheel(x, p.x, p.y, delta) || page_wheel(x, p.x, p.y, delta)) == Some(true) {
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
                    let home_task = x.home.as_ref().and_then(|h| h.task_entry);
                    let you_entry = x.you.as_ref().is_some_and(|y| y.fields.iter().any(|f| !f.2 && Some(f.1) == x.hub.ui.focus));
                    if ctrl && vk == b'K' as u16 {
                        palette_open(x);
                    } else if ctrl && (b'1' as u16..=b'9' as u16).contains(&vk) {
                        nav_jump(x, (vk - b'1' as u16) as usize);
                    } else if !ctrl && hubgarden::key(x, vk) {
                    } else if vk == VK_RETURN && entry.is_some() && x.hub.ui.focus == entry {
                        chat_cmd(x, "chat_send");
                    } else if vk == VK_RETURN && home_task.is_some() && x.hub.ui.focus == home_task {
                        home_cmd(x, "home_go");
                    } else if vk == VK_RETURN && you_entry {
                        you_cmd(x, "you_save");
                    } else if vk == VK_RETURN && x.tt.as_ref().is_some_and(|t| x.hub.ui.focus == Some(t.ke)) {
                        tt_cmd(x, "tt_add");
                    } else if vk == VK_RETURN && x.tt.as_ref().is_some_and(|t| x.hub.ui.focus == Some(t.te)) {
                        x.hub.ui.focus = x.tt.as_ref().map(|t| t.ke);
                    } else if vk == VK_RETURN && x.hub.side.page.as_deref() == Some("deadlines") && hubup::key_return(x) {
                    } else if vk == VK_RETURN && x.hub.side.page.as_deref() == Some("reminders") && hubrem::key_return(x) {
                    } else if vk == VK_RETURN && x.hub.side.page.as_deref() == Some("settings") && hubset::key_return(x) {
                    } else if vk == VK_RETURN && x.hub.side.page.as_deref() == Some("customize") && hubcust::key_return(x) {
                    } else if vk == VK_RETURN && hubbook::key_return(x) {
                    } else if vk == VK_RETURN && hubscreen::key_return(x) {
                    } else if vk == VK_RETURN && hubgym::key_return(x) {
                    } else if vk == VK_RETURN && x.daily.as_ref().is_some_and(|d| x.hub.ui.focus == Some(d.a)) {
                        daily_cmd(x, "daily_card_add");
                    } else if vk == VK_RETURN && x.daily.as_ref().is_some_and(|d| x.hub.ui.focus == Some(d.fact)) {
                        daily_cmd(x, "daily_fact_add");
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

/// Zenko20.PREBUILD: the pages made in the background, one at a time, once the window has settled (z20_prewarm)
const PREBUILD: [&str; 14] = ["timetable", "deadlines", "reminders", "daily", "notebook", "customize", "settings", "you",
                              "garden", "exercise", "screen", "map", "chat", "notes"];

/// z20_nav_jump: Ctrl+1..9, the sidebar's pages in order
fn nav_jump(x: &mut HubW, i: usize) {
    if let Some(k) = x.hub.side.navkeys.get(i).map(|n| n.0.clone()) {
        if x.hub.side.page.as_deref() != Some(k.as_str()) {
            out("sound:nav");
        }
        show(x, &k);
    }
}

/// z20_palette: Ctrl+K or the title bar's search pill (z20_items from what the window has now)
fn palette_open(x: &mut HubW) {
    use zenko_desktop::palette::{ItemsIn, Palette, card_titles, items};
    let nav: Vec<(String, String)> = zenko_desktop::custpage::NAV_ALL.iter().map(|(k, l)| (k.to_string(), l.to_string())).collect();
    let visible: Vec<String> = x.hub.side.navkeys.iter().map(|n| n.0.clone()).collect();
    let mut cards = vec![];
    for (k, pg) in &x.pages {
        for t in card_titles(&x.hub.ui, *pg) {
            cards.push((k.clone(), t));
        }
    }
    let (fox, session) = crate::win::with_fox(|f, _| (f.fox_name(), f.session.is_some()))
        .unwrap_or_else(|| ("Zenko".into(), false));
    let its = items(&ItemsIn { nav_all: &nav, visible: &visible, cards: &cards, session, fox: &fox });
    let fonts = GdiFonts::new(x.dpi);
    let p = Palette::new(&x.hub.t, &x.hub.font_ui, x.size.0, x.hub.zk_f, its, &fox, &fonts);
    paletwin::open(p, fonts, x.hub.t.clone(), x.hwnd, window_rect(x.hwnd));
}

/// what a picked palette row does (ZkPalette.run: the nav sound, then its run)
fn palette_do(x: &mut HubW, cmd: &str) {
    out("sound:nav");
    let page_now = x.hub.side.page.clone().unwrap_or_else(|| "home".into());
    if let Some(k) = cmd.strip_prefix("page:") {
        show(x, k);
    } else if let Some(r) = cmd.strip_prefix("reveal:") {
        let (page, title) = r.split_once('\t').unwrap_or((r, ""));
        show(x, page);
        x.reveal = Some((page.into(), title.into(), now(x) + 0.16));
    } else if cmd.starts_with("book:") {
        // book_open(k): the Study books page (its books aren't in this build yet)
        show(x, "notebook");
        out(&format!("run:{cmd}"));
    } else if let Some(v) = cmd.strip_prefix("theme:") {
        out(&format!("run:theme:{v}"));
    } else if let Some(k) = cmd.strip_prefix("accent:") {
        // z20_set_accent: set_theme(theme, announce=False) opens the window again on the same page
        let k = k.to_string();
        crate::win::with_fox(move |fox, _| fox.cfg_set("accent", Value::from(k)));
        out(&format!("run:reopen:{page_now}"));
    } else if let Some(k) = cmd.strip_prefix("motion:") {
        let k = k.to_string();
        crate::win::with_fox(move |fox, _| fox.cfg_set("motion_level", Value::from(k)));
    } else if let Some(q) = cmd.strip_prefix("ask:") {
        hub_ask(x, q);
    } else if let Some(d) = cmd.strip_prefix("do:") {
        match d {
            "start" => out("run:cmd:focus"),
            "pause" | "stop" | "lockin" | "break" => out(&format!("run:cmd:{d}")),
            "breath_box" | "breath_478" => hubgarden::app_cmd(x, &format!("breath:{}", &d[7..])),
            _ => out(&format!("run:{d}")),
        }
    }
}

/// the palette's picks, a revealed card's glide and glow, and the pages made in the background
fn palette_tick(x: &mut HubW, n: f64) -> bool {
    let mut dirty = false;
    for c in paletwin::PICKED.with(|q| std::mem::take(&mut *q.borrow_mut())) {
        palette_do(x, &c);
        dirty = true;
    }
    if let Some((page, title, at)) = x.reveal.clone()
        && n >= at {
        x.reveal = None;
        reveal(x, &page, &title, n);
        dirty = true;
    }
    if let Some((card, base, t0)) = x.flash {
        // z20_flash: 14 steps of 70 ms, the border glowing up and back down
        let i = ((n - t0) / 0.07) as i32;
        if let Kind::Canvas { items, .. } = &mut x.hub.ui.ws[card].kind {
            items.truncate(base);
        }
        if i >= 14 {
            x.flash = None;
        } else {
            let a = 1.0 - (i as f64 - 7.0).abs() / 7.0;
            let col = zenko_art::palette::mixhex(&x.hub.t.card, &x.hub.t.gold, 0.25 + 0.75 * a);
            let (w, h) = (x.hub.ui.ws[card].w, x.hub.ui.ws[card].h);
            if let Kind::Canvas { items, .. } = &mut x.hub.ui.ws[card].kind {
                // rbox(1, 1, W-2, H-2, 12, outline, width=2)
                for k in 0..2 {
                    items.push(zenko_desktop::ui::Item::Poly {
                        pts: zenko_desktop::shapes::round_pts(1 + k, 1 + k, w - 2 - k, h - 2 - k, 12 - k, 2, "round"),
                        fill: None, outline: Some(col.clone()) });
                }
            }
        }
        dirty = true;
    }
    if n >= x.prebuild_at && x.slide.is_none() && !x.minimized {
        x.prebuild_at = n + 0.35;
        if let Some(p) = PREBUILD.iter().find(|p| !x.pages.iter().any(|(k, _)| k == *p) && page_built(p)) {
            page_frame(x, p);
        }
    }
    dirty
}

/// the pages this build can make
fn page_built(p: &str) -> bool {
    matches!(p, "chat" | "home" | "timetable" | "deadlines" | "reminders" | "settings" | "customize" | "daily" | "you" | "notebook"
             | "notes" | "screen" | "exercise")
}

/// z20_reveal's find: the card titled so (else the first whose title holds it), scrolled to 12 px from the top,
/// then its glow
fn reveal(x: &mut HubW, page: &str, title: &str, n: f64) {
    let Some(pg) = x.pages.iter().find(|(k, _)| k == page).map(|p| p.1) else { return };
    let mut ids = zenko_desktop::pagekit::walk(&x.hub.ui, pg);
    ids.sort();
    let row_title = |ui: &zenko_desktop::ui::Ui, id: usize| -> Option<String> {
        let k = &ui.ws[id].kids;
        if k.len() >= 2 && matches!(ui.ws[id].kind, Kind::Frame)
            && matches!(&ui.ws[k[0]].kind, Kind::Label { text, .. } if text == "\u{25c6}")
            && let Kind::Label { text, .. } = &ui.ws[k[1]].kind {
            return Some(text.clone());
        }
        None
    };
    let rows: Vec<(usize, String)> = ids.iter().filter_map(|&i| row_title(&x.hub.ui, i).map(|t| (i, t))).collect();
    let tl = title.to_lowercase();
    let Some(row) = rows.iter().find(|r| r.1 == title).or_else(|| rows.iter().find(|r| r.1.to_lowercase().contains(&tl)))
        .map(|r| r.0) else { return };
    let Some(card) = x.hub.ui.ws[row].parent else { return };
    let mut inner = x.hub.ui.ws[card].parent;
    while let Some(i) = inner {
        if x.hub.ui.ws[i].cwin {
            break;
        }
        inner = x.hub.ui.ws[i].parent;
    }
    if let (Some(inner), Some(sc)) = (inner, page_sc(x, Some(page)))
        && sc.inner == inner {
        let y = x.hub.ui.abs(card).1 - x.hub.ui.abs(inner).1;
        sc.set_y(&mut x.hub.ui, y - 12);
        let (w, h) = x.size;
        x.hub.layout(&x.fonts, w, h);
        let th = x.hub.t.clone();
        sc.draw_bar(&mut x.hub.ui, false, &th);
    }
    if let Kind::Canvas { items, card: Some(_), .. } = &x.hub.ui.ws[card].kind {
        x.flash = Some((card, items.len(), n));
    }
}
