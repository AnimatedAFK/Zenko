//! The hub's Town square page in the window (zenko.py hub_build_square + Zenko23's festival street): the status row
//! (Go online / Leave, your nickname), the board painted every 800 ms from the link, Send with slow mode, a
//! right-click on a message to report, block or copy it, and the street (a 90 ms frame, stalls to click, a
//! right-click menu for your yukata, the fox mask and festival previews).

use super::*;
use serde_json::json;
use zenko_desktop::squarelink::StallDo;
use zenko_desktop::squarepage::{SquareData, SquarePage};
use zenko_desktop::squareart::{SQUARE_SLOW, square_clean, square_name_ok, square_tag};

fn cm10(x: &HubW) -> i32 {
    (100.0 * x.dpi / 25.4 + 0.5) as i32
}

fn cfg() -> zenko_desktop::Cfg {
    crate::win::with_fox(|fox, _| fox.cfg.clone()).unwrap_or_else(zenko_desktop::Cfg::load)
}

fn cfg_set(k: &str, v: Value) {
    let k = k.to_string();
    crate::win::with_fox(move |fox, _| zenko_desktop::update_config(&mut fox.cfg, |c| c[k.as_str()] = v.clone()));
}

fn say(t: &str, secs: f64) {
    let t = t.to_string();
    crate::win::with_fox(move |fox, _| fox.say(&t, secs));
}

/// square_uid(): made once (16 hex digits) and kept in the config
fn uid() -> String {
    crate::win::with_fox(|fox, _| {
        if let Some(u) = fox.cfg.raw.get("square_uid").and_then(|v| v.as_str()).filter(|s| !s.is_empty()) {
            return u.to_string();
        }
        let hi = (fox.rand() * 4294967296.0) as u64;
        let lo = (fox.rand() * 4294967296.0) as u64;
        let u = format!("{:016x}", (hi << 32) | lo);
        let v = u.clone();
        zenko_desktop::update_config(&mut fox.cfg, |c| c["square_uid"] = json!(v));
        u
    }).unwrap_or_else(|| "0000000000000000".into())
}

fn strs(c: &zenko_desktop::Cfg, k: &str) -> Vec<String> {
    c.list(k)
}

/// what the page shows, read from the app and the link now
fn data(x: &HubW) -> SquareData {
    let c = cfg();
    let on = c.b("square_on", false);
    let (_, _, jp) = hubbook::fonts_pub(&x.hub.font_ui);
    let now = zenko_desktop::unix_now();
    let offset = zenko_desktop::local_offset_pub();
    let secs = now.floor() as i64 + offset;
    let (mut state, mut msgs, mut changed) = ("off".to_string(), vec![], 0);
    if let Some(l) = crate::sqnet::link() {
        let l = l.lock().unwrap();
        state = l.state.clone();
        changed = l.changed;
        if on {
            let hid = strs(&c, "square_hidden");
            msgs = l.visible(&strs(&c, "square_block")).into_iter().filter(|m| !hid.contains(&m.id)).collect();
        }
    }
    let i = |k: &str, d: i64| c.raw.get(k).and_then(|v| v.as_i64().or_else(|| v.as_f64().map(|f| f as i64))).unwrap_or(d);
    SquareData {
        on, state, uid: uid(), name: c.s("square_name", ""), msgs, changed, yukata: i("z23_yukata", 3), mask: c.b("z23_mask", false),
        now, offset, day: secs.div_euclid(86400), hour: (secs.rem_euclid(86400) / 3600) as u32, preview: x.sq_preview.clone(),
        jp: if jp.is_empty() { x.hub.font_ui.clone() } else { jp },
    }
}

pub(super) fn build(x: &mut HubW, pg: usize) {
    let (th, f) = (x.hub.t.clone(), faces(x));
    let d = data(x);
    let c = cm10(x);
    let seed = (zenko_desktop::unix_now() * 1e6) as u64;
    let mut p = SquarePage::build(&mut x.hub.ui, pg, &th, &f, c, &d, x.size.1, seed);
    // (painted at once, before the page is laid out, as hub_build_square does)
    p.paint(&mut x.hub.ui, &x.fonts);
    x.square = Some(p);
    x.sq_paint_at = 0.0;
}

pub(super) fn fit(x: &mut HubW, w: i32, h: i32) {
    let (th, f) = (x.hub.t.clone(), faces(x));
    let Some(p) = x.square.as_mut() else { return };
    p.fit(&mut x.hub.ui, &x.fonts, &th, &f, x.hub.zk_f, w, h);
}

fn on_page(x: &HubW) -> bool {
    x.hub.side.page.as_deref() == Some("square") && x.square.is_some()
}

fn paint(x: &mut HubW) {
    let d = data(x);
    let Some(p) = x.square.as_mut() else { return };
    p.d = d;
    let ids = [p.st, p.go];
    let before: Vec<String> = ids.iter().map(|&i| x.hub.ui.text_of(i)).collect();
    p.paint(&mut x.hub.ui, &x.fonts);
    // (a label that changed its text takes a new size: the page is laid out again)
    if ids.iter().zip(&before).any(|(&i, b)| x.hub.ui.text_of(i) != *b) {
        let (w, h) = x.size;
        x.hub.layout(&x.fonts, w, h);
        fit(x, w, h);
    }
}

/// the 800 ms paint tick and the street's 90 ms frame, while the page shows
pub(super) fn step(x: &mut HubW, n: f64) -> bool {
    if !on_page(x) {
        return false;
    }
    let mut did = false;
    if n >= x.sq_paint_at {
        x.sq_paint_at = n + 0.8;
        paint(x);
        did = true;
    }
    if x.slide.is_none() && n >= x.sq_next {
        x.sq_next = n + 0.09;
        let d = x.square.as_ref().unwrap().d.clone();
        let p = x.square.as_mut().unwrap();
        if let Some(s) = p.street.as_mut()
            && s.lay.is_some()
            && s.step(&mut x.hub.ui, &x.fonts, &d)
        {
            s.build(&mut x.hub.ui, &x.fonts, &d);
        }
        did = true;
    }
    did
}

/// save_name(): a nickname that passes is kept
fn save_name(x: &mut HubW) -> Option<String> {
    let p = x.square.as_ref()?;
    let n = square_name_ok(&x.hub.ui.text_of(p.ne))?;
    cfg_set("square_name", json!(n));
    Some(n)
}

/// the nickname box lost the keyboard (<FocusOut>)
pub(super) fn focus_left(x: &mut HubW, old: Option<usize>) {
    if x.square.as_ref().is_some_and(|p| old == Some(p.ne) && x.hub.ui.focus != old) {
        save_name(x);
    }
}

/// Return in the nickname box (save, then the message box) or the message box (send)
pub(super) fn key_return(x: &mut HubW) -> bool {
    let Some(p) = x.square.as_ref() else { return false };
    let (ne, me) = (p.ne, p.me);
    if x.hub.ui.focus == Some(ne) {
        save_name(x);
        x.hub.ui.focus = Some(me);
        true
    } else if x.hub.ui.focus == Some(me) {
        cmd(x, "sq_send");
        true
    } else {
        false
    }
}

fn rules() {
    let text = "The Town square is public: anyone using Zenko can read it. Use a nickname, be kind, and never share your real \
                name, school or where you live.".to_string();
    crate::win::with_fox(move |fox, _| fox.events.push(zenko_desktop::Ev::Say {
        text, secs: 40.0, choices: vec![("I understand".into(), "sq_rules_ok".into()), ("Not now".into(), "sq_rules_no".into())],
    }));
}

fn toggle(x: &mut HubW) {
    let c = cfg();
    if c.b("square_on", false) {
        cfg_set("square_on", json!(false));
        crate::sqnet::disconnect();
    } else {
        if !c.b("square_rules_ok", false) {
            rules();
            return;
        }
        cfg_set("square_on", json!(true));
        crate::sqnet::connect(&cfg());
    }
    if let Some(p) = x.square.as_mut() {
        p.shown = -1;
    }
    paint(x);
}

fn set_entry(x: &mut HubW, id: usize, s: &str) {
    if let Kind::Entry { e, .. } = &mut x.hub.ui.ws[id].kind {
        e.text = s.encode_utf16().collect();
        e.pos = e.text.len();
        e.left = 0;
        e.sel = None;
    }
}

/// a post finished (on its thread): put the text back and say so if it didn't go
static SENT: std::sync::Mutex<Vec<(String, Option<String>)>> = std::sync::Mutex::new(vec![]);

fn send(x: &mut HubW) {
    let c = cfg();
    if !c.b("square_on", false) || crate::sqnet::link().is_none() {
        if !c.b("square_rules_ok", false) {
            rules();
        } else {
            toggle(x);
        }
        return;
    }
    let Some(name) = save_name(x) else {
        say("Pick a nickname first (3 to 18 letters or numbers, nothing rude)~", 8.0);
        x.hub.ui.focus = x.square.as_ref().map(|p| p.ne);
        return;
    };
    let now = zenko_desktop::unix_now();
    let p = x.square.as_mut().unwrap();
    if SQUARE_SLOW - (now - p.last) > 0.0 {
        return;
    }
    let me = p.me;
    let t = square_clean(&x.hub.ui.text_of(me));
    if t.is_empty() {
        return;
    }
    x.square.as_mut().unwrap().last = now;
    set_entry(x, me, "");
    let t2 = t.clone();
    crate::sqnet::post(uid(), name, t, None, move |err| SENT.lock().unwrap().push((t2, err)));
}

/// what came back from posts since the last frame
pub(super) fn posted(x: &mut HubW) {
    let done: Vec<_> = std::mem::take(&mut *SENT.lock().unwrap());
    for (t, err) in done {
        match err {
            Some(e) => {
                if let Some(p) = x.square.as_mut() {
                    p.last = 0.0;
                    let me = p.me;
                    set_entry(x, me, &t);
                }
                say(&format!("That didn't send. Are you online? ({})", zenko_desktop::extra::clip(&e, 60)), 8.0);
            }
            None => crate::win::with_fox(|fox, _| fox.events.push(zenko_desktop::Ev::Ui("send".into()))).unwrap_or(()),
        }
    }
}

pub(super) fn cmd(x: &mut HubW, c: &str) {
    let (a, arg) = c.split_once(':').unwrap_or((c, ""));
    match a {
        "sq_toggle" => toggle(x),
        "sq_send" => send(x),
        "sq_rules_ok" => {
            cfg_set("square_rules_ok", json!(true));
            toggle(x);
        }
        "sq_report" | "sq_block" | "sq_copy" => {
            let Some(m) = x.sq_menu_msg.take() else { return };
            match a {
                "sq_report" => {
                    crate::sqnet::post(uid(), String::new(), String::new(), Some(m.id.clone()), |_| {});
                    say("Thank you for reporting. It's hidden for you now~", 6.0);
                    let mut hid = cfg().list("square_hidden");
                    if !hid.contains(&m.id) {
                        hid.push(m.id.clone());
                    }
                    hid.sort();
                    let from = hid.len().saturating_sub(300);
                    cfg_set("square_hidden", json!(hid[from..].to_vec()));
                }
                "sq_block" => {
                    let mut b = cfg().list("square_block");
                    if !b.contains(&m.uid) {
                        b.push(m.uid.clone());
                    }
                    let from = b.len().saturating_sub(200);
                    cfg_set("square_block", json!(b[from..].to_vec()));
                }
                _ => {
                    crate::dialog::clipboard_set(&m.text.encode_utf16().collect::<Vec<u16>>());
                    return;
                }
            }
            if let Some(p) = x.square.as_mut() {
                p.shown = -1;
            }
            paint(x);
        }
        "sq_yukata" => set_look(x, Some(arg.parse().unwrap_or(3)), None),
        "sq_mask" => {
            let on = !cfg().b("z23_mask", false);
            set_look(x, None, Some(on));
        }
        "sq_preview" => {
            x.sq_preview = if arg.is_empty() { None } else { Some(arg.to_string()) };
            let d = data(x);
            if let Some(s) = x.square.as_mut().and_then(|p| p.street.as_mut()) {
                s.aw = 0;
                s.build(&mut x.hub.ui, &x.fonts, &d);
            }
            if let Some(p) = x.square.as_mut() {
                p.d = d;
            }
        }
        _ => {}
    }
}

/// z23_set_look(yukata=…, mask=…): saved, sent with your next message, and your person in the crowd restyled
fn set_look(x: &mut HubW, yukata: Option<i64>, mask: Option<bool>) {
    if let Some(y) = yukata {
        cfg_set("z23_yukata", json!(y));
    }
    if let Some(m) = mask {
        cfg_set("z23_mask", json!(m));
    }
    restyled(x);
}

fn restyled(x: &mut HubW) {
    crate::sqnet::set_look(&cfg());
    let d = data(x);
    if let Some(p) = x.square.as_mut() {
        p.d = d.clone();
        if let Some(s) = p.street.as_mut() {
            s.restyle_me(&d);
        }
    }
}

fn street_local(x: &HubW, mx: i32, my: i32) -> Option<(f64, f64)> {
    let p = x.square.as_ref()?;
    let cv = p.street.as_ref()?.cv;
    let (ax, ay) = x.hub.ui.abs(cv);
    let w = &x.hub.ui.ws[cv];
    let (lx, ly) = (mx - ax, my - ay);
    (lx >= 0 && ly >= 0 && lx < w.w && ly < w.h && x.hub.ui.shown(cv)).then_some((lx as f64, ly as f64))
}

/// <Motion> over the street: the hand over a stall
pub(super) fn motion(x: &mut HubW, mx: i32, my: i32) -> bool {
    if !on_page(x) {
        return false;
    }
    street_local(x, mx, my).and_then(|(lx, ly)| x.square.as_ref()?.street.as_ref()?.at(lx, ly)).is_some()
}

/// a click on a stall
pub(super) fn click(x: &mut HubW, mx: i32, my: i32) -> bool {
    if !on_page(x) {
        return false;
    }
    let Some(key) = street_local(x, mx, my).and_then(|(lx, ly)| x.square.as_ref()?.street.as_ref()?.at(lx, ly)) else { return false };
    let r = crate::win::with_fox(move |fox, _| fox.z23_stall(&key, zenko_desktop::unix_now())).unwrap_or(StallDo::Nothing);
    match r {
        StallDo::Look => restyled(x),
        StallDo::Puffs => {
            if let Some(s) = x.square.as_mut().and_then(|p| p.street.as_mut()) {
                s.puffs(&mut x.hub.ui);
            }
        }
        StallDo::Nothing => {}
    }
    true
}

/// which right-click menu a point gets: the board's message menu (not for your own) or the street's
pub(super) fn rclick_at(x: &mut HubW, mx: i32, my: i32) -> Option<&'static str> {
    if !on_page(x) {
        return None;
    }
    if street_local(x, mx, my).is_some() {
        return Some("street");
    }
    let p = x.square.as_ref()?;
    let (ax, ay) = x.hub.ui.abs(p.txt);
    let w = &x.hub.ui.ws[p.txt];
    if mx < ax || my < ay || mx >= ax + w.w || my >= ay + w.h {
        return None;
    }
    let m = p.msg_at(&x.hub.ui, &x.fonts, mx, my)?;
    if m.uid == uid() {
        return None;
    }
    x.sq_menu_msg = Some(m);
    Some("board")
}

const YUKATA: [&str; 10] = ["Indigo", "Crimson", "Sakura pink", "White and blue", "Wisteria", "Pine green", "Persimmon", "Night and gold",
                            "Sky blue", "Plum"];
const PREVIEW: [&str; 18] = ["newyear", "setsubun", "valentine", "hina", "holi", "hanami", "kodomo", "tanabata", "summer", "obon",
                             "tsukimi", "halloween", "diwali", "christmas", "omisoka", "spring", "autumn", "winter"];

/// when_text(key): "(now)" or "(12 Mar)" for the festival's next days, from today
fn when_text(day: i64, key: &str) -> String {
    let (y, _, _) = zenko_desktop::civil_from_days_pub(day);
    for yy in [y as i64, y as i64 + 1] {
        for (k, a, b) in zenko_desktop::squareart::fest_calendar(yy) {
            if k == key && b >= day {
                if a <= day {
                    return "(now)".into();
                }
                let (_, m, d) = zenko_desktop::civil_from_days_pub(a);
                let mon = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"][(m - 1) as usize];
                return format!("({d} {mon})");
            }
        }
    }
    String::new()
}

/// the menus a right-click opens (tk.Menu is the Windows menu there too)
pub(super) unsafe fn popup(hwnd: HWND, which: &str) {
    let w = |s: &str| s.encode_utf16().chain(std::iter::once(0)).collect::<Vec<u16>>();
    let mut cmds: Vec<String> = vec![];
    let mut add = |pm: HMENU, label: &str, c: String, flags: u32| unsafe {
        cmds.push(c);
        let t = w(label);
        AppendMenuW(pm, MF_STRING | flags, cmds.len(), t.as_ptr());
    };
    let pm = unsafe { CreatePopupMenu() };
    if which == "board" {
        let Some(m) = with(|x| x.sq_menu_msg.clone()).flatten() else { return };
        add(pm, "Report this message", "sq_report".into(), 0);
        add(pm, &format!("Block {}#{}", m.name, square_tag(&m.uid)), "sq_block".into(), 0);
        add(pm, "Copy text", "sq_copy".into(), 0);
    } else {
        let c = cfg();
        let y = c.raw.get("z23_yukata").and_then(|v| v.as_i64()).unwrap_or(3);
        let ym = unsafe { CreatePopupMenu() };
        for (i, n) in YUKATA.iter().enumerate() {
            add(ym, n, format!("sq_yukata:{i}"), if i as i64 == y { MF_CHECKED } else { 0 });
        }
        let t = w("My yukata");
        unsafe { AppendMenuW(pm, MF_POPUP, ym as usize, t.as_ptr()) };
        add(pm, if c.b("z23_mask", false) { "Take off my fox mask" } else { "Wear a fox mask" }, "sq_mask".into(), 0);
        let day = (zenko_desktop::unix_now().floor() as i64 + zenko_desktop::local_offset_pub()).div_euclid(86400);
        let fm = unsafe { CreatePopupMenu() };
        for k in PREVIEW {
            let name = zenko_desktop::squareart::fest(k)[1].as_str().unwrap_or(k).to_string();
            add(fm, &format!("{name}  {}", when_text(day, k)), format!("sq_preview:{k}"), 0);
        }
        let t = w("Preview a festival");
        unsafe { AppendMenuW(pm, MF_POPUP, fm as usize, t.as_ptr()) };
        if with(|x| x.sq_preview.is_some()).unwrap_or(false) {
            add(pm, "Back to today's street", "sq_preview:".into(), 0);
        }
    }
    let r = unsafe {
        let mut p = POINT { x: 0, y: 0 };
        GetCursorPos(&mut p);
        let r = TrackPopupMenu(pm, TPM_RETURNCMD | TPM_LEFTALIGN | TPM_TOPALIGN, p.x, p.y, 0, hwnd, std::ptr::null());
        DestroyMenu(pm);
        r
    };
    with(|x| {
        if r > 0
            && let Some(c) = cmds.get(r as usize - 1)
        {
            cmd(x, c);
        } else {
            x.sq_menu_msg = None;
        }
    });
}
