//! The hub's Study books page in the window: the shelf (petals every 40 ms, hover lift and the hanging scroll), a book
//! clicked on it (book_anim: 9 + 12 frames, then the book), an open book (Zenko teaching every 170/260 ms, the practice
//! corner, the tutor), its links, buttons, option menus and entries, and what they change in the app (zenko.py
//! hub_build_notebook, book_open, nb_render, shelf_motion, shelf_click, book_anim, Zenko23.book_open's Bible).

use super::*;
use serde_json::{Value, json};
use std::sync::{Arc, Mutex};
use zenko_desktop::ui::{Fill, Pack};
use zenko_desktop::bookpage::{Act, BookPage};
use zenko_desktop::shelfpage::{self as shelf, ShelfData, ShelfPage};

/// the book clicked on the shelf, opening: the next frame, when, where it started, the canvas, its colour
struct Anim {
    key: String,
    next: usize,
    at: f64,
    spot: (i32, i32, i32, i32),
    wh: (i32, i32),
    col: String,
}

pub(super) struct Nb {
    host: usize,
    shelf: Option<ShelfPage>,
    sd: ShelfData,
    shelf_wh: (i32, i32),
    shelf_next: f64,
    book: Option<BookPage>,
    teach_next: f64,
    anim: Option<Anim>,
    render_at: Option<f64>,
    /// nb_refresh after a goal is saved: the page is made again unless an entry or text box has the keyboard
    refresh_at: Option<f64>,
    focus: Option<usize>,
    /// ollama_models() from a thread: the names once they're in
    brains: Option<Arc<Mutex<Option<Vec<String>>>>>,
    fonts: (String, String, String),
}

thread_local! {
    /// self.nb_sid: the book open (or "shelf"); it stays while the app runs (and starts from life["nb_last"])
    static SID: RefCell<Option<String>> = const { RefCell::new(None) };
}

fn pick(names: &[&str], dflt: &str) -> String {
    names.iter().find(|n| crate::gdi::font_exists(n)).map(|n| n.to_string()).unwrap_or_else(|| dflt.to_string())
}

pub(super) fn fonts_pub(font_ui: &str) -> (String, String, String) {
    fonts(font_ui)
}

/// _mincho_font, _serif_font and _jp_font (or FONT_UI)
fn fonts(font_ui: &str) -> (String, String, String) {
    let jp = pick(&["Yu Gothic UI", "Yu Gothic", "Meiryo UI", "Meiryo", "MS Gothic", "Noto Sans CJK JP", "Noto Sans JP", "IPAGothic"], "");
    let jpg = if jp.is_empty() { font_ui.to_string() } else { jp };
    let mincho = pick(&["Yu Mincho", "\u{6e38}\u{660e}\u{671d}", "MS PMincho", "MS Mincho", "Noto Serif JP", "Noto Serif CJK JP",
                        "IPAMincho", "IPAPMincho"], &jpg);
    let serif = pick(&["Georgia", "Cambria", "Palatino Linotype", "Book Antiqua", "Garamond", "Constantia", "Liberation Serif",
                       "DejaVu Serif", "FreeSerif"], font_ui);
    (mincho, serif, jpg)
}

fn sid() -> String {
    SID.with(|s| s.borrow().clone()).unwrap_or_else(|| {
        crate::win::with_fox(|fox, _| fox.life_now()["nb_last"].as_str().map(String::from)).flatten()
            .unwrap_or_else(|| "overview".into())
    })
}

fn set_sid(k: &str) {
    SID.with(|s| *s.borrow_mut() = Some(k.to_string()));
}

fn shown(x: &HubW) -> bool {
    x.hub.side.page.as_deref() == Some("notebook") && x.nb.is_some()
}

/// hub_build_notebook(pg)
pub(super) fn build(x: &mut HubW, pg: usize) {
    let th = x.hub.t.clone();
    let host = x.hub.ui.frame(pg, &th.paper);
    x.hub.ui.pack(host, Pack::default().fill(Fill::Both).expand());
    let f = fonts(&x.hub.font_ui);
    x.nb = Some(Nb { host, shelf: None, sd: ShelfData::default(), shelf_wh: (0, 0), shelf_next: 0.0, book: None, teach_next: 0.0,
                     anim: None, render_at: None, refresh_at: None, focus: None, brains: None, fonts: f });
    let s = sid();
    if s == "bible" || shelf::book(&s).is_none() {
        set_sid("shelf");
    }
    render(x);
}

/// nb_render: the shelf, or the open book, made again
fn render(x: &mut HubW) {
    let Some(nb) = x.nb.as_mut() else { return };
    let kids = x.hub.ui.ws[nb.host].kids.clone();
    for k in kids {
        x.hub.ui.destroy(k);
    }
    nb.shelf = None;
    nb.book = None;
    nb.anim = None;
    nb.render_at = None;
    nb.refresh_at = None;
    nb.brains = None;
    nb.focus = None;
    let host = nb.host;
    let key = sid();
    let th = x.hub.t.clone();
    let (mincho, serif, jpg) = nb.fonts.clone();
    if shelf::book(&key).is_none() || key == "bible" {
        let mut sd = crate::win::with_fox(|fox, _| fox.shelf_data()).unwrap_or_default();
        (sd.jp, sd.serif) = (mincho, serif);
        let seed = (zenko_desktop::unix_now() * 1000.0) as u64;
        let sp = ShelfPage::build(&mut x.hub.ui, host, &th.paper, seed);
        let nb = x.nb.as_mut().unwrap();
        nb.sd = sd;
        nb.shelf = Some(sp);
        nb.shelf_wh = (0, 0);
    } else {
        let Some(mut d) = crate::win::with_fox(|fox, _| fox.book_data(&key)) else { return };
        (d.jp, d.serif, d.jpg) = (mincho, serif, jpg);
        d.now = zenko_desktop::unix_now();
        let cm10 = (100.0 * x.dpi / 25.4 + 0.5) as i32;
        let seed = (d.now * 1000.0) as u64;
        let fu = x.hub.font_ui.clone();
        let bp = BookPage::build(&mut x.hub.ui, host, &th, &fu, cm10, d, zenko_mt(seed));
        let nb = x.nb.as_mut().unwrap();
        nb.book = Some(bp);
        nb.teach_next = 0.0;
        // brain_card: which free brains are there (ollama_models, in the background)
        let slot = Arc::new(Mutex::new(None));
        let s2 = slot.clone();
        std::thread::spawn(move || {
            let names = ollama_models();
            *s2.lock().unwrap() = Some(names);
        });
        nb.brains = Some(slot);
    }
    relayout(x);
}

fn zenko_mt(seed: u64) -> zenko_sound::mt::PyRandom {
    zenko_sound::mt::PyRandom::new(seed)
}

/// ollama_models(): GET http://127.0.0.1:11434/api/tags, the models' names ([] when Ollama isn't running)
fn ollama_models() -> Vec<String> {
    use std::io::{Read, Write};
    let addr: std::net::SocketAddr = "127.0.0.1:11434".parse().unwrap();
    let Ok(mut s) = std::net::TcpStream::connect_timeout(&addr, std::time::Duration::from_millis(800)) else { return vec![] };
    let _ = s.set_read_timeout(Some(std::time::Duration::from_secs(3)));
    if s.write_all(b"GET /api/tags HTTP/1.0\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n").is_err() {
        return vec![];
    }
    let mut buf = vec![];
    let _ = s.read_to_end(&mut buf);
    let txt = String::from_utf8_lossy(&buf);
    let body = txt.split_once("\r\n\r\n").map(|b| b.1).unwrap_or("");
    let v: Value = serde_json::from_str(body).unwrap_or(Value::Null);
    v["models"].as_array().map(|a| a.iter().filter_map(|m| m["name"].as_str().map(String::from)).collect()).unwrap_or_default()
}

/// the page laid out for the window's size: the book's fit (its scroll places kept), the shelf drawn again when its
/// canvas changed size (its <Configure>)
pub(super) fn fit(x: &mut HubW, w: i32, h: i32) {
    let Some(nb) = x.nb.as_mut() else { return };
    let host = nb.host;
    if let Some(p) = nb.book.as_mut() {
        let (yl, yr) = (x.hub.ui.ws[p.lsc.cv].yoff, x.hub.ui.ws[p.rsc.cv].yoff);
        p.fit(&mut x.hub.ui, &x.fonts, host, x.hub.zk_f, w, h);
        let (l, r) = (p.lsc.clone(), p.rsc.clone());
        l.set_y(&mut x.hub.ui, yl);
        r.set_y(&mut x.hub.ui, yr);
        x.hub.ui.layout_at(&x.fonts, w, h);
        let th = x.hub.t.clone();
        l.draw_bar(&mut x.hub.ui, false, &th);
        r.draw_bar(&mut x.hub.ui, false, &th);
    }
    if let Some(sp) = nb.shelf.as_mut() {
        let wh = (x.hub.ui.ws[sp.cv].w, x.hub.ui.ws[sp.cv].h);
        if wh != nb.shelf_wh && wh.0 > 1 && nb.anim.is_none() {
            nb.shelf_wh = wh;
            sp.draw(&mut x.hub.ui, &x.fonts, &nb.sd);
        }
    }
}

/// the book's two pages' scrollers (the one under the pointer gets the wheel)
pub(super) fn scrollers(x: &HubW) -> Vec<zenko_desktop::pagekit::Scroller> {
    x.nb.as_ref().and_then(|n| n.book.as_ref()).map(|p| vec![p.lsc.clone(), p.rsc.clone()]).unwrap_or_default()
}

/// every frame while the page shows: the shelf's tick, the opening animation, her teaching, the brains' answer
pub(super) fn step(x: &mut HubW, n: f64) {
    if !shown(x) {
        return;
    }
    let un = zenko_desktop::unix_now();
    let f = x.hub.ui.focus;
    let nb = x.nb.as_mut().unwrap();
    if f != nb.focus {
        let old = nb.focus;
        nb.focus = f;
        if let Some(old) = old
            && let Some(p) = nb.book.as_mut()
            && p.prac.entries.contains(&old) {
            let acts = p.entry_done(&mut x.hub.ui, old);
            apply(x, acts);
        }
    }
    let nb = x.nb.as_mut().unwrap();
    if nb.refresh_at.is_some_and(|t| n >= t) {
        nb.refresh_at = None;
        let typing = f.is_some_and(|i| matches!(x.hub.ui.ws[i].kind, Kind::Entry { .. } | Kind::Text { .. }));
        if !typing {
            render(x);
            return;
        }
    }
    let nb = x.nb.as_mut().unwrap();
    if nb.render_at.is_some_and(|t| n >= t) {
        nb.render_at = None;
        render(x);
        return;
    }
    if let Some(a) = nb.anim.as_mut() {
        if n >= a.at {
            if a.next < shelf::ANIM_A + shelf::ANIM_B {
                let sp = nb.shelf.as_ref().unwrap();
                let cv = zenko_desktop::hubwin::canvas_of_pub(&mut x.hub.ui, sp.cv);
                let jp = nb.fonts.2.clone();
                shelf::anim_frame(cv, a.next, a.spot, a.wh.0, a.wh.1, &a.key, &a.col, &jp, &x.hub.font_ui);
                a.at = n + shelf::anim_delay(a.next);
                a.next += 1;
            } else {
                let key = a.key.clone();
                finish_open(x, &key);
            }
        }
        return;
    }
    if let Some(sp) = nb.shelf.as_mut()
        && n >= nb.shelf_next {
        nb.shelf_next = n + 0.04;
        sp.tick(&mut x.hub.ui);
    }
    if let Some(p) = nb.book.as_mut() {
        let names = nb.brains.as_ref().and_then(|b| b.lock().unwrap().take());
        if let Some(names) = names {
            nb.brains = None;
            p.brains_done(&mut x.hub.ui, &names);
            relayout(x);
            return;
        }
        let nb = x.nb.as_mut().unwrap();
        let p = nb.book.as_mut().unwrap();
        let waiting = p.prac.next_at.is_some();
        p.tick(&mut x.hub.ui, un, &mut nb.teach_next);
        if waiting && p.prac.next_at.is_none() {
            // (the psychology corner's next question: the page's words changed)
            relayout(x);
        }
    }
}

fn finish_open(x: &mut HubW, key: &str) {
    set_sid(key);
    let k = key.to_string();
    crate::win::with_fox(move |fox, _| fox.life_do(|l| l["nb_last"] = json!(k)));
    render(x);
}

/// book_open(key, anim): the shelf, a book (gliding open when it was clicked on the shelf), or the Bible's own window
fn open(x: &mut HubW, key: &str, anim: bool) {
    if key == "bible" {
        out("sound:open");
        out("run:open_bible");
        return;
    }
    if key != "shelf" && shelf::book(key).is_none() {
        return;
    }
    let n = now(x);
    if anim && key != "shelf"
        && let Some(nb) = x.nb.as_mut()
        && let Some(sp) = nb.shelf.as_ref() {
        let Some(s) = sp.spots.iter().find(|s| s.0 == key).cloned() else {
            finish_open(x, key);
            return;
        };
        let col = crate::win::with_fox(|fox, _| zenko_desktop::bookpage::book_col(&fox.book_data(key))).unwrap_or_default();
        let wh = (x.hub.ui.ws[sp.cv].w, x.hub.ui.ws[sp.cv].h);
        out("sound:nav");
        let cv = zenko_desktop::hubwin::canvas_of_pub(&mut x.hub.ui, sp.cv);
        let jp = nb.fonts.2.clone();
        shelf::anim_frame(cv, 0, (s.1, s.2, s.3, s.4), wh.0, wh.1, key, &col, &jp, &x.hub.font_ui);
        nb.anim = Some(Anim { key: key.into(), next: 1, at: n + shelf::anim_delay(0), spot: (s.1, s.2, s.3, s.4), wh, col });
        return;
    }
    if key == "shelf" {
        set_sid("shelf");
        render(x);
    } else {
        finish_open(x, key);
    }
}

/// what the book asked for
fn apply(x: &mut HubW, acts: Vec<Act>) {
    let mut again = false;
    for a in acts {
        match a {
            Act::None => {}
            Act::Open(k, anim) => {
                open(x, &k, anim);
                return;
            }
            Act::Save => {
                let Some(p) = x.nb.as_ref().and_then(|n| n.book.as_ref()) else { continue };
                let parts: Vec<(String, Value)> = ["book_state", "fr_vocab", "goals", "grades"].iter()
                    .filter(|k| !p.life[**k].is_null()).map(|k| (k.to_string(), p.life[*k].clone())).collect();
                crate::win::with_fox(move |fox, _| fox.life_do(|l| {
                    for (k, v) in parts {
                        l[k.as_str()] = v;
                    }
                }));
            }
            Act::Cfg(k, v) => {
                crate::win::with_fox(move |fox, _| fox.cfg_set(&k, v));
            }
            Act::Ask(sid, q, mode) => {
                let t = crate::win::with_fox(|fox, _| fox.nb_ask_chat(zenko_desktop::unix_now(), Some(sid), &q, &mode))
                    .unwrap_or_default();
                if let Some(p) = x.nb.as_ref().and_then(|n| n.book.as_ref()) {
                    BookPage::set_text(&mut x.hub.ui, p.status, &t);
                }
            }
            Act::Run(c) => run(x, &c),
            Act::Render => again = true,
        }
    }
    if again {
        render(x);
    } else {
        relayout(x);
    }
}

fn run(x: &mut HubW, c: &str) {
    let parts: Vec<String> = c.split('\t').map(String::from).collect();
    let un = zenko_desktop::unix_now();
    match parts[0].as_str() {
        "nb_mistake" if parts.len() == 4 => {
            let (sid, q, a) = (parts[1].clone(), parts[2].clone(), parts[3].clone());
            crate::win::with_fox(move |fox, _| fox.nb_mistake(un, &sid, &q, &a));
            // (the book's own copy of the deck follows)
            let cards = crate::win::with_fox(|fox, _| fox.life_now()["cards"].clone()).unwrap_or(Value::Null);
            if let Some(p) = x.nb.as_mut().and_then(|n| n.book.as_mut()) {
                p.life["cards"] = cards;
            }
        }
        "study_sort" if parts.len() == 3 => {
            let (k, s) = (parts[1].clone(), parts[2].clone());
            crate::win::with_fox(move |fox, _| fox.study_sort(un, &k, if s.is_empty() { None } else { Some(&s) }));
        }
        _ => {
            if let Some(k) = c.strip_prefix("sound:") {
                out(&format!("sound:{k}"));
            } else if let Some(t) = c.strip_prefix("render_in:") {
                let n = now(x);
                if let Some(nb) = x.nb.as_mut() {
                    nb.render_at = Some(n + t.parse::<f64>().unwrap_or(0.9));
                }
            } else if c == "refresh" {
                let n = now(x);
                if let Some(nb) = x.nb.as_mut() {
                    nb.refresh_at = Some(n + 0.05);
                }
            } else if let Some(t) = c.strip_prefix("tool:") {
                out(&format!("run:{t}"));
            } else {
                // (the sources' files, a brain download and the distraction list's editor aren't ported yet)
                let s = c.replace('\t', " ");
                crate::win::with_fox(move |fox, _| fox.trace.push(format!("nb:{s}")));
            }
        }
    }
}

/// a button, radio or checkbox of the book
pub(super) fn cmd(x: &mut HubW, c: &str) {
    if c.starts_with("bk:gr_menu:") {
        return;
    }
    let Some(p) = x.nb.as_mut().and_then(|n| n.book.as_mut()) else { return };
    let acts = p.command(&mut x.hub.ui, c);
    apply(x, acts);
}

fn inside(x: &HubW, id: usize, mx: i32, my: i32) -> bool {
    let (ax, ay) = x.hub.ui.abs(id);
    let w = &x.hub.ui.ws[id];
    w.mapped && mx >= ax && my >= ay && mx < ax + w.w && my < ay + w.h
}

/// the link label under the pointer (a label, or a widget inside one)
fn link_at(x: &HubW, mx: i32, my: i32) -> Option<usize> {
    let p = x.nb.as_ref()?.book.as_ref()?;
    let mut over = x.hub.ui.hit(mx, my);
    while let Some(i) = over {
        if p.is_link(i) {
            return Some(i);
        }
        over = x.hub.ui.ws[i].parent;
    }
    None
}

/// shelf_click / a link's <Button-1>
pub(super) fn click(x: &mut HubW, mx: i32, my: i32) -> bool {
    if !shown(x) {
        return false;
    }
    let nb = x.nb.as_ref().unwrap();
    if nb.anim.is_some() {
        return true;
    }
    if let Some(sp) = nb.shelf.as_ref() {
        if !inside(x, sp.cv, mx, my) {
            return false;
        }
        let (ax, ay) = x.hub.ui.abs(sp.cv);
        if let Some(k) = sp.key_at(mx - ax, my - ay) {
            out("sound:nav");
            open(x, &k, true);
        }
        return true;
    }
    if let Some(id) = link_at(x, mx, my) {
        let p = x.nb.as_mut().unwrap().book.as_mut().unwrap();
        let acts = p.link(&mut x.hub.ui, id);
        apply(x, acts);
        return true;
    }
    false
}

/// shelf_motion (the book under the pointer lifts and the scroll says what it is) and the links' hover colour
pub(super) fn motion(x: &mut HubW, mx: i32, my: i32) {
    if !shown(x) {
        return;
    }
    let over = link_at(x, mx, my);
    let nb = x.nb.as_mut().unwrap();
    if nb.anim.is_some() {
        return;
    }
    if let Some(sp) = nb.shelf.as_mut() {
        let (ax, ay) = x.hub.ui.abs(sp.cv);
        let w = &x.hub.ui.ws[sp.cv];
        let k = if mx >= ax && my >= ay && mx < ax + w.w && my < ay + w.h { sp.key_at(mx - ax, my - ay) } else { None };
        if k.is_some() {
            x.cursor = IDC_HAND;
        }
        sp.set_hover(&mut x.hub.ui, &x.fonts, &nb.sd, k);
        return;
    }
    if let Some(p) = nb.book.as_mut() {
        p.hover(&mut x.hub.ui, over);
        if over.is_some() {
            x.cursor = IDC_HAND;
        }
    }
}

/// Return in one of the book's entries
pub(super) fn key_return(x: &mut HubW) -> bool {
    if !shown(x) {
        return false;
    }
    let Some(f) = x.hub.ui.focus else { return false };
    let Some(p) = x.nb.as_mut().and_then(|n| n.book.as_mut()) else { return false };
    match p.key_return(&mut x.hub.ui, f) {
        Some(acts) => {
            apply(x, acts);
            true
        }
        None => false,
    }
}

/// the grades book's option menus: their choices pop up under them
pub(super) fn menu_at(x: &HubW, mx: i32, my: i32) -> Option<usize> {
    if !shown(x) {
        return None;
    }
    let hit = x.hub.ui.hit(mx, my)?;
    matches!(&x.hub.ui.ws[hit].kind, Kind::Menu { cmd, .. } if cmd.starts_with("bk:gr_menu:")).then_some(hit)
}

pub(super) unsafe fn popup(hwnd: HWND, m: usize) {
    let Some((ax, ay, h, cur, which, items)) = with(|x| {
        let (ax, ay) = x.hub.ui.abs(m);
        let which = match &x.hub.ui.ws[m].kind {
            Kind::Menu { cmd, .. } => cmd.strip_prefix("bk:gr_menu:").unwrap_or("").to_string(),
            _ => String::new(),
        };
        let items = x.nb.as_ref().and_then(|n| n.book.as_ref()).map(|p| p.menu_items(&which)).unwrap_or_default();
        (ax, ay, x.hub.ui.ws[m].h, x.hub.ui.text_of(m), which, items)
    }) else { return };
    if items.is_empty() {
        return;
    }
    let mut p = POINT { x: ax, y: ay + h };
    let r = unsafe {
        ClientToScreen(hwnd, &mut p);
        let pm = CreatePopupMenu();
        for (i, t) in items.iter().enumerate() {
            let w: Vec<u16> = t.encode_utf16().chain(std::iter::once(0)).collect();
            AppendMenuW(pm, MF_STRING | if *t == cur { MF_CHECKED } else { 0 }, i + 1, w.as_ptr());
        }
        let r = TrackPopupMenu(pm, TPM_RETURNCMD | TPM_LEFTALIGN | TPM_TOPALIGN, p.x, p.y, 0, hwnd, std::ptr::null());
        DestroyMenu(pm);
        r
    };
    with(|x| {
        if r > 0
            && let Some(t) = items.get(r as usize - 1) {
            cmd(x, &format!("bk:gr_set:{which}:{t}"));
        }
        relayout(x);
    });
}
