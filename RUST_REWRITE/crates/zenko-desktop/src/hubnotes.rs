//! The hub's Notes page in the window (zenko.py Zenko21): building it, the scratchpad's draft saved 700 ms after you
//! stop typing and its preview 500 ms after, a typed-in page saved 800 ms after, the notebooks list (click, double
//! click, wheel), File it now / Undo, New page / New folder / Open / Delete, Import a folder (and keep bringing in new
//! notes from it), and the scratchpad filed when you leave the page or close the window (hub_show, z20_back,
//! hub_close).

use super::*;
use serde_json::{Value, json};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use zenko_desktop::notes::{self, nb_dir, nb_scratch};
use zenko_desktop::notespage::{NAct, NotesPage, Undo};

thread_local! {
    /// self._nb_last: the last filing, undoable until Zenko closes
    static UNDO: RefCell<Option<Undo>> = const { RefCell::new(None) };
}

type Import = Arc<Mutex<Option<(usize, usize, Vec<(String, usize)>)>>>;

/// the page's timers and what the boxes held last frame
#[derive(Default)]
pub(super) struct NotesState {
    draft_at: Option<f64>,
    prev_at: Option<f64>,
    page_at: Option<f64>,
    scratch: String,
    view: String,
    /// a folder being brought in: (folder, ask to watch it, quiet, the result once it's done)
    import: Option<(PathBuf, bool, bool, Import)>,
}

fn life() -> Value {
    crate::win::with_fox(|fox, _| fox.life_now()).unwrap_or(Value::Null)
}

fn shown(x: &HubW) -> bool {
    x.hub.side.page.as_deref() == Some("notes") && x.notes.is_some()
}

/// hub_build_notes(pg)
pub(super) fn build(x: &mut HubW, pg: usize) {
    let th = x.hub.t.clone();
    let cm10 = (100.0 * x.dpi / 25.4 + 0.5) as i32;
    let f = faces(x);
    let open: Vec<String> = crate::win::with_fox(|fox, _| fox.cfg.raw["nb_open"].as_array().map(|a| a.iter()
        .filter_map(|v| v.as_str().map(String::from)).collect())).flatten().unwrap_or_default();
    let scratch = std::fs::read(nb_scratch()).map(|b| String::from_utf8_lossy(&b).to_string()).unwrap_or_default();
    let p = NotesPage::build(&mut x.hub.ui, pg, &th, &f, cm10, &life(), &open, &scratch);
    let view = x.hub.ui.text_mut(p.view).plain();
    x.notes_st = NotesState { scratch: scratch.clone(), view, ..Default::default() };
    x.notes = Some(p);
}

pub(super) fn fit(x: &mut HubW, w: i32, h: i32) {
    let (th, f) = (x.hub.t.clone(), faces(x));
    let Some(p) = x.notes.as_mut() else { return };
    p.fit(&mut x.hub.ui, &x.fonts, &th, &f, x.hub.zk_f, w, h);
}

fn acts(x: &mut HubW, a: Vec<NAct>) {
    for act in a {
        match act {
            NAct::Cfg(k, v) => {
                crate::win::with_fox(move |fox, _| fox.cfg_set(&k, v));
            }
            NAct::NbLast(v) => {
                crate::win::with_fox(move |fox, _| fox.life_do(|l| l["nb_last"] = v));
            }
            NAct::Say(t, s) => {
                crate::win::with_fox(move |fox, _| fox.say(&t, s));
            }
            NAct::Open(p) => open_path(&p),
        }
    }
    let _ = x;
}

/// os.startfile(path)
fn open_path(p: &Path) {
    let w: Vec<u16> = p.as_os_str().to_string_lossy().encode_utf16().chain(std::iter::once(0)).collect();
    let verb: Vec<u16> = "open\0".encode_utf16().collect();
    unsafe {
        windows_sys::Win32::UI::Shell::ShellExecuteW(std::ptr::null_mut(), verb.as_ptr(), w.as_ptr(), std::ptr::null(),
                                                     std::ptr::null(), SW_SHOWNORMAL);
    }
}

/// nb_flush(quiet): the scratchpad filed (the page's, or the draft file when the page isn't there)
pub(super) fn flush(x: &mut HubW, quiet: bool) {
    let n = zenko_desktop::unix_now();
    if let Some(p) = x.notes.as_mut() {
        p.life = life();
        let (u, a) = p.flush(&mut x.hub.ui, quiet, n);
        if u.is_some() {
            UNDO.with(|v| *v.borrow_mut() = u);
            x.notes_st.scratch.clear();
        }
        loaded(x);
        acts(x, a);
        return;
    }
    let Ok(body) = std::fs::read(nb_scratch()) else { return };
    let body = String::from_utf8_lossy(&body).to_string();
    let mut l = life();
    let (u, a) = zenko_desktop::notespage::flush_text(&body, &mut l, quiet, n);
    if u.is_some() {
        let _ = std::fs::write(nb_scratch(), "");
        UNDO.with(|v| *v.borrow_mut() = u);
    }
    acts(x, a);
}

/// quit with the window closed: the draft file filed quietly (as nb_flush does on quit)
pub(super) fn quit_flush() {
    let Ok(body) = std::fs::read(nb_scratch()) else { return };
    let body = String::from_utf8_lossy(&body).to_string();
    let n = zenko_desktop::unix_now();
    let mut l = life();
    let (u, a) = zenko_desktop::notespage::flush_text(&body, &mut l, true, n);
    if u.is_some() {
        let _ = std::fs::write(nb_scratch(), "");
    }
    for act in a {
        match act {
            NAct::Cfg(k, v) => {
                crate::win::with_fox(move |fox, _| fox.cfg_set(&k, v));
            }
            NAct::NbLast(v) => {
                crate::win::with_fox(move |fox, _| fox.life_do(|l| l["nb_last"] = v));
            }
            _ => {}
        }
    }
}

/// hub_close / quit: the open page saved, the scratchpad filed quietly
pub(super) fn closing(x: &mut HubW) {
    if let Some(p) = x.notes.as_mut() {
        p.page_save(&mut x.hub.ui);
    }
    draft_save(x);
    flush(x, true);
}

fn draft_save(x: &mut HubW) {
    x.notes_st.draft_at = None;
    if let Some(p) = x.notes.as_ref() {
        let body = x.hub.ui.text_mut(p.scratch).plain();
        let _ = std::fs::create_dir_all(zenko_desktop::data_dir());
        let _ = std::fs::write(nb_scratch(), body);
    }
}

/// every frame while the page shows: typing in the boxes starts their timers (KeyRelease), which then run out
pub(super) fn step(x: &mut HubW, n: f64) {
    let done = x.notes_st.import.as_ref().and_then(|(d, ask, quiet, slot)| {
        let r = slot.lock().unwrap().take();
        r.map(|r| (d.clone(), *ask, *quiet, r))
    });
    if let Some((d, ask, quiet, res)) = done {
        x.notes_st.import = None;
        import_done(x, &d, ask, quiet, res);
    }
    if !shown(x) {
        return;
    }
    let p = x.notes.as_ref().unwrap();
    let (sc, vw) = (p.scratch, p.view);
    let st = x.hub.ui.text_mut(sc).plain();
    if st != x.notes_st.scratch {
        x.notes_st.scratch = st;
        x.notes_st.draft_at = Some(n + 0.7);
        x.notes_st.prev_at = Some(n + 0.5);
    }
    let vt = x.hub.ui.text_mut(vw).plain();
    if vt != x.notes_st.view {
        // (what a page / folder shows is set through loaded(): a change here was typed)
        x.notes_st.view = vt;
        let p = x.notes.as_mut().unwrap();
        p.modified = true;
        x.notes_st.page_at = Some(n + 0.8);
    }
    if x.notes_st.draft_at.is_some_and(|t| n >= t) {
        draft_save(x);
    }
    if x.notes_st.prev_at.is_some_and(|t| n >= t) {
        x.notes_st.prev_at = None;
        let p = x.notes.as_mut().unwrap();
        p.life = life();
        p.preview(&mut x.hub.ui);
        relayout(x);
    }
    if x.notes_st.page_at.is_some_and(|t| n >= t) {
        x.notes_st.page_at = None;
        let p = x.notes.as_mut().unwrap();
        p.page_save(&mut x.hub.ui);
    }
}

/// what the view holds now is what was loaded (not typed): no save for it
fn loaded(x: &mut HubW) {
    if let Some(p) = x.notes.as_ref() {
        x.notes_st.view = x.hub.ui.text_mut(p.view).plain();
    }
}

pub(super) fn cmd(x: &mut HubW, c: &str) {
    let Some(p) = x.notes.as_mut() else { return };
    p.life = life();
    if let Some(i) = c.strip_prefix("notes_row:") {
        let _ = i;
        let a = p.tree_click(&mut x.hub.ui, &x.fonts);
        loaded(x);
        acts(x, a);
    } else if c.starts_with("notes_row_open:") {
        if let Some((_, path, _)) = p.sel(&x.hub.ui) {
            open_path(&path);
        }
    } else {
        match c {
            "notes_file" => flush(x, false),
            "notes_undo" => {
                let u = UNDO.with(|v| v.borrow_mut().take());
                let a = p.undo(&mut x.hub.ui, u);
                x.notes_st.scratch = x.hub.ui.text_mut(p.scratch).plain();
                acts(x, a);
            }
            "notes_new_page" => {
                p.new_page(&mut x.hub.ui);
                loaded(x);
            }
            "notes_new_folder" => out("run:notes_folder_ask"),
            "notes_open" => {
                if let Some((_, path, _)) = p.sel(&x.hub.ui) {
                    open_path(&path);
                }
            }
            "notes_open_dir" => open_path(&nb_dir()),
            "notes_delete" => {
                if let Some(q) = p.delete_ask(&mut x.hub.ui) {
                    x.ask = Some((q, "Zenko".into(), "notes_delete_yes".into()));
                }
            }
            "notes_delete_yes" => {
                let nl = zenko_desktop::plan::naive(zenko_desktop::unix_now());
                let (day, s) = (nl.div_euclid(86400.0) as i64, nl.rem_euclid(86400.0) as i64);
                let stamp = format!("{}-{:02}{:02}{:02}", zenko_desktop::plan::iso(day).replace('-', ""), s / 3600, s / 60 % 60, s % 60);
                p.delete(&mut x.hub.ui, &stamp);
                loaded(x);
            }
            "notes_import" => out("run:notes_import_ask"),
            _ => {
                // (the reMarkable window isn't ported yet)
                let s = c.to_string();
                crate::win::with_fox(move |fox, _| fox.trace.push(s));
            }
        }
    }
    relayout(x);
}

/// New folder's name, from the little dialog
pub fn new_folder(name: &str) {
    let name = name.trim().to_string();
    with(|x| {
        if let Some(p) = x.notes.as_mut() {
            p.new_folder(&mut x.hub.ui, &name);
            relayout(x);
        }
    });
}

/// nb_import_run(d, ask_watch, quiet): the folder copied in the background
pub fn import_run(d: &Path, ask_watch: bool, quiet: bool) {
    let d = d.to_path_buf();
    let l = life();
    with(|x| {
        if let Some(p) = x.notes.as_ref()
            && !quiet {
            let nm = d.file_name().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
            if let Kind::Label { text, .. } = &mut x.hub.ui.ws[p.status].kind {
                *text = format!("Bringing in \u{201c}{nm}\u{201d}\u{2026}");
            }
        }
        let slot: Import = Arc::new(Mutex::new(None));
        let s2 = slot.clone();
        let d2 = d.clone();
        std::thread::spawn(move || {
            let r = notes::nb_import(&d2, &l);
            *s2.lock().unwrap() = Some(r);
        });
        x.notes_st.import = Some((d.clone(), ask_watch, quiet, slot));
    });
}

fn import_done(x: &mut HubW, d: &Path, ask_watch: bool, quiet: bool, res: (usize, usize, Vec<(String, usize)>)) {
    let (copied, skipped, mut where_) = res;
    if let Some(p) = x.notes.as_mut() {
        for (top, _) in &where_ {
            p.open.insert(nb_dir().join(top).to_string_lossy().to_string());
        }
        p.tree_fill(&mut x.hub.ui);
        if !quiet || copied > 0 {
            where_.sort();
            let into = if where_.is_empty() {
                String::new()
            } else {
                format!(" into {}", where_.iter().map(|(k, v)| format!("{k} ({v})")).collect::<Vec<_>>().join(", "))
            };
            let t = format!("Imported {copied} file{}{into}{}.", if copied == 1 { "" } else { "s" },
                            if skipped > 0 { format!("  \u{b7}  {skipped} already here") } else { String::new() });
            if let Kind::Label { text, .. } = &mut x.hub.ui.ws[p.status].kind {
                *text = t;
            }
        }
        relayout(x);
    }
    if ask_watch {
        let nd = d.to_string_lossy().to_string();
        let watch: Vec<String> = crate::win::with_fox(|fox, _| fox.cfg.raw["nb_watch"].as_array().map(|a| a.iter()
            .filter_map(|v| v.as_str().map(String::from)).collect())).flatten().unwrap_or_default();
        if !watch.contains(&nd) {
            out(&format!("run:notes_watch_ask\t{nd}"));
        }
    }
}

/// the folder kept: cfg["nb_watch"] (the last 8)
pub fn watch_add(d: &str) {
    let d = d.to_string();
    crate::win::with_fox(move |fox, _| {
        let mut w: Vec<String> = fox.cfg.raw["nb_watch"].as_array().map(|a| a.iter().filter_map(|v| v.as_str().map(String::from))
            .collect()).unwrap_or_default();
        w.push(d);
        let w = w[w.len().saturating_sub(8)..].to_vec();
        fox.cfg_set("nb_watch", json!(w));
    });
}

/// the wheel over the notebooks list (4 rows a notch) or one of the two boxes (3 lines)
pub(super) fn wheel(x: &mut HubW, mx: i32, my: i32, delta: i32) -> bool {
    if !shown(x) {
        return false;
    }
    let p = x.notes.as_ref().unwrap();
    let (tree, boxes) = (p.tree, [p.scratch, p.view]);
    let mut over = x.hub.ui.hit(mx, my);
    while let Some(i) = over {
        if i == tree {
            let n = -(delta / 120) * 4;
            let ch = x.hub.ui.list_scroll(&x.fonts, tree, if n == 0 { -delta.signum() } else { n });
            p.draw_bars(&mut x.hub.ui, &x.fonts);
            return ch;
        }
        if boxes.contains(&i) {
            let d = if delta > 0 { -1 } else { 1 };
            let k = 1.max(delta.abs() / 120);
            x.hub.ui.text_mut(i).scroll_units(&x.fonts, d * k * 3);
            p.draw_bars(&mut x.hub.ui, &x.fonts);
            return true;
        }
        over = x.hub.ui.ws[i].parent;
    }
    false
}

/// filedialog.askdirectory: the folder to bring in
pub(super) unsafe fn pick_folder(hwnd: HWND) -> Option<PathBuf> {
    use windows_sys::Win32::UI::Shell::*;
    let title: Vec<u16> = "Pick the folder to bring into your notebooks\0".encode_utf16().collect();
    let mut bi: BROWSEINFOW = unsafe { std::mem::zeroed() };
    bi.hwndOwner = hwnd;
    bi.lpszTitle = title.as_ptr();
    bi.ulFlags = BIF_RETURNONLYFSDIRS | BIF_NEWDIALOGSTYLE;
    let pidl = unsafe { SHBrowseForFolderW(&bi) };
    if pidl.is_null() {
        return None;
    }
    let mut buf = [0u16; 1024];
    let ok = unsafe { SHGetPathFromIDListW(pidl, buf.as_mut_ptr()) } != 0;
    unsafe { windows_sys::Win32::System::Com::CoTaskMemFree(pidl as _) };
    if !ok {
        return None;
    }
    let n = buf.iter().position(|&c| c == 0).unwrap_or(0);
    Some(PathBuf::from(String::from_utf16_lossy(&buf[..n])))
}
