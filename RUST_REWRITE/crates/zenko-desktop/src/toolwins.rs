//! The study-tool windows on the desktop (zenko.py ZkToolWindow: open_ptable, open_french_dict, open_calc and
//! open_bible): one of each at a time (opening again brings it to the front), sized and placed as zenko.py does,
//! with their clicks, hovers, keys and wheel turns answered here. Their slow work (the translator, the Bible's
//! chapters, original words and downloads) runs on threads, as app.bg does, and comes back through DONE.

use std::cell::RefCell;
use std::sync::Mutex;
use serde_json::Value;
use zenko_desktop::biblewin::{self, BibleFonts, BibleWin, OrigWant, Want};
use zenko_desktop::casio::{self, Casio};
use zenko_desktop::frwin::{self, FrWin};
use zenko_desktop::Companion;
use zenko_desktop::ptwin::{self, PtWin};
use zenko_desktop::theme::Theme;
use zenko_desktop::toolwin::{ToolFonts, entry_text, geometry, win_title};
use zenko_desktop::ui::{Kind, Ui};

enum Done {
    Fr { q: String, res: Result<(String, String), String>, fr_in: bool },
    Chapter { want: Want, res: Result<(Vec<(i64, String)>, String), String> },
    Orig { want: OrigWant, res: Result<Vec<Value>, String> },
    Word { i: usize, res: Option<Value> },
    Prog(String, f64),
    GetAll(((i64, i64), Vec<String>)),
}

static DONE: Mutex<Vec<Done>> = Mutex::new(vec![]);

fn send(d: Done) {
    DONE.lock().unwrap_or_else(|e| e.into_inner()).push(d);
}

#[derive(Default)]
struct Tools {
    pt: Option<PtWin>,
    fr: Option<FrWin>,
    cg: Option<(Casio, Option<usize>)>,
    bible: Option<BibleWin>,
    /// the Bible's note window: (its text box, the verse key, the verse's ref)
    note: Option<(usize, String)>,
    study: Option<(usize, zenko_desktop::toolwin::Tool)>,
    next_blink: f64,
    next_prog: f64,
    th: Option<Theme>,
    dpi: f64,
    mouse: (i32, i32),
}

thread_local! {
    static T: RefCell<Tools> = RefCell::new(Tools::default());
}

const PT: &str = "ptable";
const FR: &str = "french";
const CG: &str = "casio";
const BI: &str = "bible";
const NOTE: &str = "bible_note";
const STUDY: &str = "bible_study";

fn screen() -> (i32, i32) {
    unsafe {
        use windows_sys::Win32::UI::WindowsAndMessaging::{GetSystemMetrics, SM_CXSCREEN, SM_CYSCREEN};
        (GetSystemMetrics(SM_CXSCREEN), GetSystemMetrics(SM_CYSCREEN))
    }
}

fn fonts(font_ui: &str) -> ToolFonts {
    let (mincho, serif, _jpg) = crate::hubw::tool_fonts(font_ui);
    ToolFonts { ui: font_ui.to_string(), serif, jp: mincho }
}

fn pick(names: &[&str], fallback: &str) -> String {
    names.iter().find(|n| crate::gdi::font_exists(n)).map(|n| n.to_string()).unwrap_or_else(|| fallback.to_string())
}

fn size(name: &str) -> (i32, i32) {
    crate::uiwin::with_ui(name, |ui, _| (ui.ws[0].w, ui.ws[0].h)).unwrap_or((800, 600))
}

/// the open_* commands: true when it was one of them
pub fn open(c: &str, fox: &mut Companion, th: &Theme, font_ui: &str, dpi: f64) -> bool {
    let (kind, arg) = match c {
        "open_ptable" | "ptable" => (PT, None),
        "open_french_dict" | "french_dict" | "french" => (FR, None),
        "open_calc" | "calc" => (CG, None),
        "open_bible" | "bible" => (BI, None),
        _ => match c.strip_prefix("open_bible:") {
            Some(r) => (BI, Some(r.to_string())),
            None => return false,
        },
    };
    if crate::uiwin::is_open(kind) {
        crate::uiwin::raise(kind);
        if let (BI, Some(r)) = (kind, arg) {
            goto(&r);
        }
        return true;
    }
    let f = fonts(font_ui);
    let (sw, sh) = screen();
    let cm10 = (100.0 * dpi / 25.4 + 0.5) as i32;
    let mut ui = Ui::new(&th.paper);
    let life = fox.life_now();
    let (sz, title) = match kind {
        PT => (ptwin::SIZE, ptwin::TITLE),
        FR => (frwin::SIZE, frwin::TITLE),
        CG => (casio::SIZE, casio::TITLE),
        _ => (biblewin::SIZE, biblewin::TITLE),
    };
    let (w, h, x, y) = geometry(sz, sw, sh);
    let mut first: Option<Want> = None;
    T.with(|t| {
        let mut t = t.borrow_mut();
        t.th = Some(th.clone());
        t.dpi = dpi;
        match kind {
            PT => t.pt = Some(PtWin::build(&mut ui, th, &f, life["pt_last"].as_str().unwrap_or("C"), cm10)),
            FR => {
                let hist: Vec<String> = life["fr_hist"].as_array().into_iter().flatten().filter_map(|v| v.as_str().map(String::from)).collect();
                t.fr = Some(FrWin::build(&mut ui, th, &f, &hist));
            }
            CG => t.cg = Some((Casio::build(&mut ui, th, &f, &life["cg"]), None)),
            _ => {
                let bf = BibleFonts {
                    heb: pick(&["SBL Hebrew", "Ezra SIL", "Taamey Frank CLM", "Times New Roman", "David", "Arial", "Segoe UI"], font_ui),
                    grk: pick(&["SBL Greek", "Gentium Plus", "Gentium", "Cambria", "Times New Roman", "Palatino Linotype", "Segoe UI"], font_ui),
                };
                let mut b = BibleWin::build(&mut ui, th, &f, &bf, &life["bible_pos"], cm10);
                first = Some(b.load_chapter(&mut ui));
                t.bible = Some(b);
            }
        }
    });
    crate::uiwin::open_window(kind, &win_title(title), ui, th.clone(), dpi, x, y, w, h);
    crate::uiwin::set_min(kind, 560.min(w), 420.min(h));
    fit(kind, w, h);
    if let Some(wt) = first {
        save_pos();
        run_chapter(wt);
    }
    if let (BI, Some(r)) = (kind, arg) {
        goto(&r);
    }
    true
}

/// open_bible(ref): the window goes to the reference
fn goto(r: &str) {
    if let Some((bi, ch, vs)) = zenko_desktop::routes17::bible_parse_ref(r) {
        let w = with_bible(|b, ui| b.go(ui, bi, ch, vs.unwrap_or(1)));
        if let Some(w) = w {
            save_pos();
            run_chapter(w);
        }
    }
}

fn with_bible<R>(f: impl FnOnce(&mut BibleWin, &mut Ui) -> R) -> Option<R> {
    T.with(|t| {
        let mut t = t.borrow_mut();
        let b = t.bible.as_mut()?;
        crate::uiwin::with_ui(BI, |ui, _| f(b, ui))
    })
}

fn with_bible_f<R>(f: impl FnOnce(&mut BibleWin, &mut Ui, &dyn zenko_desktop::ui::Fonts) -> R) -> Option<R> {
    T.with(|t| {
        let mut t = t.borrow_mut();
        let b = t.bible.as_mut()?;
        crate::uiwin::with_ui(BI, |ui, fo| f(b, ui, fo))
    })
}

fn fit(name: &str, w: i32, h: i32) {
    T.with(|t| {
        let mut t = t.borrow_mut();
        let t = &mut *t;
        crate::uiwin::with_ui(name, |ui, fo| match name {
            PT => {
                if let Some(p) = t.pt.as_mut() {
                    p.fit(ui, fo, w, h);
                }
            }
            FR => {
                if let Some(p) = t.fr.as_mut() {
                    p.fit(ui, fo, w, h);
                }
            }
            CG => {
                if let Some((p, _)) = t.cg.as_mut() {
                    p.fit(ui, fo, w, h);
                }
            }
            BI => {
                if let Some(p) = t.bible.as_mut() {
                    p.fit(ui, fo, w, h);
                }
            }
            STUDY => {
                ui.layout_at(fo, w, h);
                if let Some((_, tool)) = &t.study {
                    tool.paint_bars(ui, fo);
                }
            }
            _ => ui.layout_at(fo, w, h),
        });
    });
}

fn refit(name: &str) {
    let (w, h) = size(name);
    fit(name, w, h);
}

fn life_set(fox: &mut Companion, k: &str, v: Value) {
    fox.life_do(|l| l[k] = v);
}

fn save_pos() {
    let p = T.with(|t| t.borrow().bible.as_ref().map(|b| b.pos_value()));
    if let Some(p) = p {
        crate::win::with_fox(move |fox, _| life_set(fox, "bible_pos", p));
    }
}

fn notes() -> serde_json::Map<String, Value> {
    crate::win::with_fox(|fox, _| fox.life_now()["bible_notes"].as_object().cloned()).flatten().unwrap_or_default()
}

fn run_chapter(w: Want) {
    std::thread::spawn(move || {
        let res = biblewin::chapter_job(&w);
        send(Done::Chapter { want: w, res });
    });
}

fn run_orig(w: OrigWant) {
    std::thread::spawn(move || {
        let mut prog = |l: &str, f: f64| send(Done::Prog(l.to_string(), f));
        let res = biblewin::orig_job(&w, Some(&mut prog));
        send(Done::Orig { want: w, res });
    });
}

/// the brain isn't in the Rust port yet (stage 5b): what z17_llm says when there's none
const NO_BRAIN: &str = "For this I need a brain: open Ollama (free, on this PC) or switch on the online brain in Settings > Online brain.";

/// what a tool window sent: true when it was one of them
pub fn out(o: &crate::uiwin::UiOut, fox: &mut Companion, t: f64) -> bool {
    let name = o.win.as_str();
    if ![PT, FR, CG, BI, NOTE, STUDY].contains(&name) {
        return false;
    }
    let a: Vec<&str> = o.cmd.splitn(3, ':').collect();
    let num = |i: usize| a.get(i).and_then(|v| v.parse::<i32>().ok()).unwrap_or(0);
    match a[0] {
        "resize" => return { fit(name, num(1), num(2)); true },
        "closed" => {
            T.with(|t| {
                let mut t = t.borrow_mut();
                match name {
                    PT => t.pt = None,
                    FR => t.fr = None,
                    CG => {
                        if let Some((c, _)) = t.cg.as_mut() {
                            c.save();
                            let v = c.life_value();
                            fox.life_do(|l| l["cg"] = v);
                        }
                        t.cg = None;
                    }
                    BI => t.bible = None,
                    NOTE => t.note = None,
                    _ => t.study = None,
                }
            });
            return true;
        }
        "motion" => {
            T.with(|t| t.borrow_mut().mouse = (num(1), num(2)));
        }
        _ => {}
    }
    match name {
        PT => pt_out(o, &a, fox),
        FR => fr_out(o, &a, fox),
        CG => cg_out(&a, fox),
        BI => bible_out(&a, fox),
        NOTE => note_out(&a, fox),
        _ => {
            if a[0] == "key" && num(1) == 0x1b {
                crate::uiwin::close(STUDY);
                T.with(|t| t.borrow_mut().study = None);
            }
        }
    }
    let _ = t;
    true
}

fn pt_out(o: &crate::uiwin::UiOut, a: &[&str], fox: &mut Companion) {
    let num = |i: usize| a.get(i).and_then(|v| v.parse::<i32>().ok()).unwrap_or(0);
    let mut picked = None;
    let mut changed = false;
    T.with(|t| {
        let mut t = t.borrow_mut();
        let Some(p) = t.pt.as_mut() else { return };
        crate::uiwin::with_ui(PT, |ui, _| {
            let local = |ui: &Ui, x: i32, y: i32| {
                let (cx, cy) = ui.abs(p.cv);
                ((x - cx) as f64, (y - cy) as f64)
            };
            match a[0] {
                "motion" | "leave" => {
                    let e = if a[0] == "leave" { None } else {
                        let (x, y) = local(ui, num(1), num(2));
                        p.at(ui, x, y)
                    };
                    if p.set_hover(e) {
                        p.draw(ui);
                    }
                }
                "click" => {
                    let (x, y) = local(ui, num(1), num(2));
                    picked = p.click(ui, x, y);
                    changed = true;
                }
                "pt_mode" => {
                    p.toggle_mode(ui);
                    changed = true;
                }
                "pt_ask" => {
                    let _q = p.ask(ui);
                    p.answer(ui, &format!("I couldn't ask right now ({NO_BRAIN})."));
                    changed = true;
                }
                "key" | "char" => {
                    // the find and molar-mass boxes follow each key typed (<KeyRelease>)
                    if ui.focus == Some(p.find_e) {
                        let q = entry_text(ui, p.find_e);
                        changed = p.find(ui, &q);
                    } else if ui.focus == Some(p.mm_e) {
                        let q = entry_text(ui, p.mm_e);
                        p.molar(ui, &q);
                        changed = true;
                    }
                    if num(1) == 0x1b {
                        p.asked = None;
                    }
                }
                _ => {}
            }
        });
    });
    let _ = o;
    if let Some(s) = picked {
        life_set(fox, "pt_last", s.into());
    }
    if a[0] == "key" && num(1) == 0x1b {
        crate::uiwin::close(PT);
        T.with(|t| t.borrow_mut().pt = None);
        return;
    }
    if changed {
        refit(PT);
    }
}

fn fr_out(o: &crate::uiwin::UiOut, a: &[&str], fox: &mut Companion) {
    let num = |i: usize| a.get(i).and_then(|v| v.parse::<i32>().ok()).unwrap_or(0);
    let mut hist: Vec<String> = fox.life_now()["fr_hist"].as_array().into_iter().flatten()
        .filter_map(|v| v.as_str().map(String::from)).collect();
    let mut job = None;
    let mut card = None;
    let mut hist_changed = false;
    T.with(|t| {
        let mut t = t.borrow_mut();
        let Some(p) = t.fr.as_mut() else { return };
        crate::uiwin::with_ui(FR, |ui, _| {
            let mut look = |p: &mut FrWin, ui: &mut Ui, hist: &mut Vec<String>| {
                job = p.look(ui, hist);
                hist_changed = true;
                p.draw_hist(ui, hist);
            };
            match a[0] {
                "fr_look" => look(p, ui, &mut hist),
                "key" if num(1) == 13 && ui.focus == Some(p.q) => look(p, ui, &mut hist),
                "fr_explain" | "fr_examples" => {
                    if p.explain(ui, a[0] == "fr_examples").is_some() {
                        p.card_added(ui, &format!("Couldn't ask ({NO_BRAIN})."));
                    }
                }
                "fr_card" => card = p.add_card(ui),
                "click" => {
                    // a recent look-up, clicked
                    if let Some(id) = ui.hit(num(1), num(2)) {
                        if let Some(h) = p.hist_labels.iter().find(|(l, _)| *l == id).map(|x| x.1.clone()) {
                            zenko_desktop::toolwin::set_entry(ui, p.q, &h);
                            look(p, ui, &mut hist);
                        }
                    }
                }
                c if c.starts_with("fr_dir") => {
                    let d = o.cmd.strip_prefix("fr_dir:").unwrap_or("auto").to_string();
                    p.set_dir(ui, &d);
                }
                _ => {}
            }
        });
    });
    if hist_changed {
        life_set(fox, "fr_hist", serde_json::json!(hist));
    }
    if let Some((q, d, fr_in)) = job {
        std::thread::spawn(move || {
            let mut tr = |q: &str, tgt: &str, src: &str| zenko_desktop::web::translate_text(q, tgt, src);
            let (res, fr2) = frwin::look_job(&q, &d, fr_in, &mut tr);
            send(Done::Fr { q, res, fr_in: fr2 });
        });
    }
    if let Some((q, ans)) = card {
        let msg = fox.card_add(&q, &ans);
        T.with(|t| {
            if let Some(p) = t.borrow_mut().fr.as_mut() {
                crate::uiwin::with_ui(FR, |ui, _| p.card_added(ui, &msg));
            }
        });
    }
    if a[0] == "key" && num(1) == 0x1b {
        crate::uiwin::close(FR);
        T.with(|t| t.borrow_mut().fr = None);
        return;
    }
    refit(FR);
}

/// a virtual key as Tk's keysym (the ones the calculator reads)
fn keysym(vk: i32) -> &'static str {
    match vk {
        0x0d => "Return",
        0x08 => "BackSpace",
        0x2e => "Delete",
        0x1b => "Escape",
        0x25 => "Left",
        0x27 => "Right",
        0x26 => "Up",
        0x28 => "Down",
        0x70 => "F1",
        0x71 => "F2",
        0x72 => "F3",
        0x73 => "F4",
        0x74 => "F5",
        0x75 => "F6",
        _ => "",
    }
}

fn cg_out(a: &[&str], fox: &mut Companion) {
    let num = |i: usize| a.get(i).and_then(|v| v.parse::<i32>().ok()).unwrap_or(0);
    let mut close = false;
    let mut save = None;
    T.with(|t| {
        let mut t = t.borrow_mut();
        let Some((c, down)) = t.cg.as_mut() else { return };
        crate::uiwin::with_ui(CG, |ui, _| {
            match a[0] {
                "down" => {
                    if let Some((id, _)) = c.key_at(ui, num(1), num(2)) {
                        c.key_shade(ui, id, true);
                        *down = Some(id);
                    }
                }
                "click" => {
                    if let Some(id) = down.take() {
                        c.key_shade(ui, id, false);
                        if let Some((id2, kid)) = c.key_at(ui, num(1), num(2)) {
                            if id2 == id {
                                c.press(ui, &kid);
                            }
                        }
                    }
                }
                "key" => {
                    let k = keysym(num(1));
                    if !k.is_empty() {
                        c.on_key(ui, k, "");
                    }
                }
                "char" => {
                    let ch = a.get(1).copied().unwrap_or("");
                    c.on_key(ui, "", ch);
                }
                _ => {}
            }
            if c.dirty {
                c.dirty = false;
                save = Some(c.life_value());
            }
            close = c.close;
        });
    });
    if let Some(v) = save {
        fox.life_do(|l| l["cg"] = v);
    }
    if close {
        crate::uiwin::close(CG);
        T.with(|t| t.borrow_mut().cg = None);
    }
}

fn bible_out(a: &[&str], fox: &mut Companion) {
    let num = |i: usize| a.get(i).and_then(|v| v.parse::<i32>().ok()).unwrap_or(0);
    let mut want: Option<Want> = None;
    let mut orig: Option<OrigWant> = None;
    let mut word: Option<(usize, String, String)> = None;
    let mut refit_now = true;
    // the two menu buttons open their menus on a click
    let menu = if a[0] == "click" {
        with_bible(|b, ui| match ui.hit(num(1), num(2)) {
            Some(h) if h == b.book_btn => "bible_books",
            Some(h) if h == b.tr_btn => "bible_trans",
            _ => "",
        }).unwrap_or("")
    } else {
        ""
    };
    let cmd = if menu.is_empty() { a[0] } else { menu };
    match cmd {
        "bible_books" => {
            let items: Vec<crate::menu::Item> = BibleWin::book_menu().into_iter().map(|(tm, groups)| crate::menu::Item::Sub {
                label: tm.into(),
                items: groups.into_iter().map(|(g, idx)| crate::menu::Item::Sub {
                    label: g.into(),
                    items: idx.into_iter().map(|i| crate::menu::cmd(zenko_desktop::bible::book(i).1, &format!("{i}"))).collect(),
                }).collect(),
            }).collect();
            let Some(th) = T.with(|t| t.borrow().th.clone()) else { return };
            let c = crate::menu::popup(crate::uiwin::hwnd_of(BI), &items, &th);
            if let Ok(i) = c.parse::<usize>() {
                want = with_bible(|b, ui| b.go(ui, i, 1, 1));
            }
        }
        "bible_trans" => {
            let cur = T.with(|t| t.borrow().bible.as_ref().map(|b| b.trans.clone())).unwrap_or_default();
            let items: Vec<crate::menu::Item> = zenko_desktop::biblefs::TRANS.iter()
                .map(|(k, _)| crate::menu::Item::Radio { label: k.to_string(), cmd: k.to_string(), on: *k == cur }).collect();
            let Some(th) = T.with(|t| t.borrow().th.clone()) else { return };
            let c = crate::menu::popup(crate::uiwin::hwnd_of(BI), &items, &th);
            if !c.is_empty() {
                want = with_bible(|b, ui| b.set_trans(ui, &c));
            }
        }
        "bible_prev" => want = with_bible(|b, ui| b.go_chapter(ui, -1)).flatten(),
        "bible_next" => want = with_bible(|b, ui| b.go_chapter(ui, 1)).flatten(),
        "bible_web" => {
            if let Some(u) = T.with(|t| t.borrow().bible.as_ref().map(|b| b.web_url(a.get(1).copied().unwrap_or("interlinear")))) {
                open_url(&u);
            }
        }
        "bible_explain" => open_study(fox),
        "bible_note" => open_note(fox),
        "bible_get_all" => {
            let (have, total) = zenko_desktop::biblefs::step_have_all();
            with_bible(|b, ui| {
                let s = if have >= total {
                    format!("All {total} chapters of the Hebrew, Aramaic and Greek are already saved on this PC.")
                } else {
                    "Saving the original text of the whole Bible (about 110 MB, one time). You can keep reading while it downloads.".into()
                };
                zenko_desktop::toolwin::set_text(ui, b.status, &s);
            });
            if have < total {
                std::thread::spawn(|| {
                    let mut prog = |l: &str, f: f64| send(Done::Prog(l.to_string(), f));
                    let r = zenko_desktop::biblefs::step_download_all(Some(&mut prog));
                    send(Done::GetAll(r));
                });
            }
        }
        "key" => {
            let vk = num(1);
            let foc = with_bible(|b, ui| (ui.focus == Some(b.ch_e), ui.focus == Some(b.ref_e))).unwrap_or((false, false));
            if vk == 13 && foc.0 {
                want = with_bible(|b, ui| b.jump_chapter(ui)).flatten();
            } else if vk == 13 && foc.1 {
                want = with_bible(|b, ui| b.goto_ref(ui)).flatten();
            } else if vk == 0x1b {
                crate::uiwin::close(BI);
                T.with(|t| t.borrow_mut().bible = None);
                return;
            }
        }
        "down" => {
            // the "Go to" box's <FocusIn>
            with_bible(|b, ui| {
                if ui.focus == Some(b.ref_e) {
                    b.ref_focus(ui);
                }
            });
        }
        "motion" | "leave" => {
            let (x, y) = (num(1), num(2));
            refit_now = with_bible_f(|b, ui, fo| {
                let v = if a[0] == "leave" { None } else { verse_under(b, ui, fo, x, y) };
                b.set_hover(ui, v)
            }).unwrap_or(false);
        }
        "wheel" => {
            let d = num(1);
            let (mx, my) = T.with(|t| t.borrow().mouse);
            with_bible_f(|b, ui, fo| {
                let over_read = ui.hit(mx, my).is_some_and(|h| h == b.read || ui.ws[h].parent == Some(b.read));
                if over_read {
                    ui.text_mut(b.read).scroll_units(fo, -(d / 120).max(-10).min(10) * 4);
                } else {
                    b.wheel(ui, d);
                }
            });
        }
        "click" => {
            let (x, y) = (num(1), num(2));
            let hit = with_bible_f(|b, ui, fo| {
                if let Some(v) = verse_under(b, ui, fo, x, y) {
                    return (Some(b.pick(ui, fo, v, false)), None);
                }
                let id = ui.hit(x, y);
                if let Some(i) = id.and_then(|id| b.card_of(ui, id)) {
                    if let Some((l, s)) = b.word(ui, i) {
                        return (None, Some((i, l, s)));
                    }
                }
                (None, None)
            });
            if let Some((o, w)) = hit {
                orig = o;
                word = w;
            }
            if orig.is_some() {
                save_pos();
            }
        }
        _ => {}
    }
    if let Some(w) = want {
        save_pos();
        run_chapter(w);
    }
    if let Some(o) = orig {
        run_orig(o);
    }
    if let Some((i, lang, s)) = word {
        std::thread::spawn(move || send(Done::Word { i, res: biblewin::word_job(&lang, &s) }));
    }
    if refit_now {
        refit(BI);
    }
}

/// webbrowser.open(url)
pub fn open_url(u: &str) {
    let w: Vec<u16> = u.encode_utf16().chain(std::iter::once(0)).collect();
    let verb: Vec<u16> = "open\0".encode_utf16().collect();
    unsafe {
        windows_sys::Win32::UI::Shell::ShellExecuteW(std::ptr::null_mut(), verb.as_ptr(), w.as_ptr(), std::ptr::null(),
                                                     std::ptr::null(), windows_sys::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL);
    }
}

/// the verse under a window point of the read text
fn verse_under(b: &BibleWin, ui: &Ui, fo: &dyn zenko_desktop::ui::Fonts, x: i32, y: i32) -> Option<i64> {
    let (rx, ry) = ui.abs(b.read);
    let w = &ui.ws[b.read];
    if !w.mapped || x < rx || y < ry || x >= rx + w.w || y >= ry + w.h {
        return None;
    }
    let Kind::Text { t, .. } = &w.kind else { return None };
    let idx = t.index_at(fo, x - rx, y - ry);
    // (only where there's text: below the last line is nothing)
    let d = t.dlineinfo(fo, idx)?;
    if y - ry > d[1] + d[3] {
        return None;
    }
    b.verse_at(ui, idx)
}

/// edit_note(): a small window with the note on this verse
fn open_note(fox: &mut Companion) {
    if crate::uiwin::is_open(NOTE) {
        crate::uiwin::raise(NOTE);
        return;
    }
    let Some((r, k, th, f, at)) = T.with(|t| {
        let t = t.borrow();
        let b = t.bible.as_ref()?;
        Some((b.ref_(None), b.key(None), b.t.th.clone(), b.t.f.clone(), t.dpi))
    }) else { return };
    let existing = fox.life_now()["bible_notes"][&k].as_str().unwrap_or("").to_string();
    let mut ui = Ui::new(&th.paper);
    let l = zenko_desktop::toolwin::lab_w(&mut ui, 0, &format!("My note on {r}"), &th.paper, &th.ink,
                                         zenko_desktop::ui::Font::new(&f.serif, 13, false), zenko_desktop::ui::Anchor::W, false, 0);
    ui.pack(l, zenko_desktop::ui::Pack::default().fill(zenko_desktop::ui::Fill::X).padx(14, 14).pady(12, 4));
    let mut tx = zenko_desktop::tktext::Text::new(zenko_desktop::ui::Font::new(&f.ui, 10, false), &th.ink, &th.field);
    (tx.height_ln, tx.wrap, tx.padx, tx.pady, tx.hl) = (8, zenko_desktop::tktext::Wrap::Word, 8, 6, 0);
    tx.insert_end(&existing, &[]);
    tx.edit = Some(existing.chars().count());
    let text = ui.text(0, tx, "");
    ui.pack(text, zenko_desktop::ui::Pack::default().fill(zenko_desktop::ui::Fill::Both).expand().padx(14, 14));
    ui.focus = Some(text);
    let br = ui.frame(0, &th.paper);
    ui.pack(br, zenko_desktop::ui::Pack::default().fill(zenko_desktop::ui::Fill::X).padx(14, 14).pady(10, 10));
    let tool = zenko_desktop::toolwin::Tool::new(&th, &f);
    let b = tool.btn(&mut ui, br, "Save", "note_save", true, 9);
    ui.pack(b, zenko_desktop::ui::Pack::default().side(zenko_desktop::ui::Side::Left));
    let b = tool.btn(&mut ui, br, "Cancel", "note_cancel", false, 9);
    ui.pack(b, zenko_desktop::ui::Pack::default().side(zenko_desktop::ui::Side::Left).padx(6, 6));
    let (bx, by) = crate::uiwin::origin(BI);
    crate::uiwin::open_window(NOTE, &format!("Note on {r}"), ui, th, at, bx + 140, by + 120, 440, 300);
    T.with(|t| t.borrow_mut().note = Some((text, k)));
}

fn note_out(a: &[&str], fox: &mut Companion) {
    match a[0] {
        "note_save" => {
            let Some((text, k)) = T.with(|t| t.borrow().note.clone()) else { return };
            let v = crate::uiwin::with_ui(NOTE, |ui, _| match &ui.ws[text].kind {
                Kind::Text { t, .. } => t.plain().trim().to_string(),
                _ => String::new(),
            }).unwrap_or_default();
            fox.life_do(|l| {
                if !l["bible_notes"].is_object() {
                    l["bible_notes"] = serde_json::json!({});
                }
                let n = l["bible_notes"].as_object_mut().unwrap();
                if v.is_empty() {
                    n.remove(&k);
                } else {
                    n.insert(k.clone(), v.clone().into());
                }
            });
            crate::uiwin::close(NOTE);
            T.with(|t| t.borrow_mut().note = None);
            // load_chapter(), keeping the verse
            let w = with_bible(|b, ui| {
                let keep = b.vs;
                let w = b.load_chapter(ui);
                b.vs = keep;
                w
            });
            if let Some(w) = w {
                run_chapter(w);
            }
        }
        "note_cancel" => {
            crate::uiwin::close(NOTE);
            T.with(|t| t.borrow_mut().note = None);
        }
        _ => {}
    }
}

/// explain(): the study notes window (cached notes, else what the brain says)
fn open_study(fox: &mut Companion) {
    let Some((r, _user, key, th, f, at)) = T.with(|t| {
        let t = t.borrow();
        let b = t.bible.as_ref()?;
        let (r, u, k) = b.explain_ask();
        Some((r, u, k, b.t.th.clone(), b.t.f.clone(), t.dpi))
    }) else { return };
    crate::uiwin::close(STUDY);
    let cached = fox.life_now()["bible_study"][&key].as_str().map(String::from);
    let mut ui = Ui::new(&th.paper);
    use zenko_desktop::ui::{Anchor, Fill, Font, Pack};
    let l = zenko_desktop::toolwin::lab_w(&mut ui, 0, &format!("{r}  \u{b7}  study notes"), &th.paper, &th.ink,
                                         Font::new(&f.serif, 15, false), Anchor::W, false, 0);
    ui.pack(l, Pack::default().fill(Fill::X).padx(16, 16).pady(12, 0));
    let l = zenko_desktop::toolwin::lab_w(&mut ui, 0, "Written by an AI from the original-language data. Test it against Scripture and good \
                                                       commentaries (Acts 17:11).", &th.paper, &th.grey,
                                         Font { family: f.ui.clone(), size: 8, bold: false, italic: true }, Anchor::W, false, 0);
    ui.pack(l, Pack::default().fill(Fill::X).padx(16, 16));
    let mut tool = zenko_desktop::toolwin::Tool::new(&th, &f);
    let out = tool.text_box(&mut ui, 0, 20, 10);
    ui.pack(out, Pack::default().fill(Fill::Both).expand().padx(12, 12).pady(10, 10));
    let msg = match cached {
        Some(c) => c,
        None => format!("I couldn't write study notes right now ({NO_BRAIN}).\n\nYou can still click each word for its \
                         lexicon entry, or open **Commentaries \u{2197}** for Matthew Henry, Barnes, Ellicott, \
                         Pulpit and more on this verse."),
    };
    tool.show_md(&mut ui, out, &msg, 10);
    let (bx, by) = crate::uiwin::origin(BI);
    crate::uiwin::open_window(STUDY, &format!("{r} \u{b7} study notes"), ui, th, at, bx + 80, by + 40, 720, 700);
    T.with(|t| t.borrow_mut().study = Some((out, tool)));
    fit(STUDY, 720, 700);
}

/// the windows' clocks (the calculator's blinking cursor, the Bible's download progress) and the threads' answers
pub fn tick(fox: &mut Companion, t: f64) {
    let blink = T.with(|x| {
        let mut x = x.borrow_mut();
        if t >= x.next_blink && x.cg.is_some() {
            x.next_blink = t + 0.53;
            return true;
        }
        false
    });
    if blink {
        T.with(|x| {
            if let Some((c, _)) = x.borrow_mut().cg.as_mut() {
                crate::uiwin::with_ui(CG, |ui, _| c.tick(ui));
            }
        });
    }
    let prog = T.with(|x| {
        let mut x = x.borrow_mut();
        if t >= x.next_prog && x.bible.is_some() {
            x.next_prog = t + 0.4;
            return true;
        }
        false
    });
    if prog {
        with_bible(|b, ui| b.status_prog(ui));
    }
    let done: Vec<Done> = std::mem::take(&mut *DONE.lock().unwrap_or_else(|e| e.into_inner()));
    for d in done {
        match d {
            Done::Fr { q, res, fr_in } => {
                T.with(|x| {
                    if let Some(p) = x.borrow_mut().fr.as_mut() {
                        crate::uiwin::with_ui(FR, |ui, _| p.look_done(ui, &q, res, fr_in));
                    }
                });
                refit(FR);
            }
            Done::Chapter { want, res } => {
                let notes = notes();
                let o = with_bible_f(|b, ui, fo| b.chapter_done(ui, fo, &want, res, &notes)).flatten();
                if let Some(o) = o {
                    save_pos();
                    run_orig(o);
                }
                refit(BI);
            }
            Done::Orig { want, res } => {
                let notes = notes();
                with_bible(|b, ui| b.orig_done(ui, &want, res, &notes));
                refit(BI);
            }
            Done::Word { i, res } => {
                with_bible(|b, ui| b.word_done(ui, i, res));
                refit(BI);
            }
            Done::Prog(l, f) => {
                with_bible(|b, _| b.prog = Some((l, f)));
            }
            Done::GetAll(((h, tot), errs)) => {
                let msg = if h >= tot {
                    format!("Saved {h} of {tot} chapters in the original languages. Every verse now works offline.")
                } else {
                    format!("Saved {h} of {tot} chapters. Some parts didn't download: {}", errs.iter().take(2).cloned().collect::<Vec<_>>().join("; "))
                };
                let again = with_bible(|b, ui| {
                    b.prog = None;
                    zenko_desktop::toolwin::set_text(ui, b.status, &msg);
                    if h > 0 && b.words.is_empty() { Some(b.load_original(ui)) } else { None }
                }).flatten();
                if let Some(o) = again {
                    run_orig(o);
                    // (zenko.py puts the summary back 1.5 s later; the lookup's own status shows meanwhile)
                }
                refit(BI);
            }
        }
    }
    let _ = fox;
}
