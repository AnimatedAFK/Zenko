//! The hub's Notes page (zenko.py Zenko21.hub_build_notes and its nb_* methods): the Scratchpad (type anything, it is
//! kept as a draft and filed into each subject's "Quick notes" when you leave the page or press File it now, with an
//! undo), and the Notebooks: a list of the notebooks folder (subject folders first, folders open / closed, file
//! counts) beside a page that shows a folder's count, a text page to edit, or what kind of file it is; New page, New
//! folder, Open and Delete (to the notebooks bin).

use crate::chatpage::scrollbar_draw;
use crate::hubwin::{canvas_of, hub_title_draw};
use crate::notes::*;
use crate::pagekit::{Faces, WrapState, hub_card, label_w, tt_btn, zk_fonts, zk_wrap};
use crate::theme::Theme;
use crate::tkcanvas::Photo;
use crate::tktext::{Text, Wrap};
use crate::ui::{Anchor, Fill, Font, Fonts, Grid, Kind, Pack, Side, Ui};
use serde_json::{Value, json};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::rc::Rc;

pub const TITLE: &str = "Notes";
pub const SUB: &str = "Write anything. When you leave this page, I file each part into the right subject";
const INTRO: &str = "Pick a notebook on the left to open it.\n\nEverything you file from the Scratchpad lands in that \
                     subject's \u{201c}Quick notes\u{201d}. Import a folder (like your reMarkable export) and its \
                     folders show up here too: a folder called Bio, Chem, French... goes straight into that subject.";

/// what nb_flush put where, to undo (path, size before or -1)
#[derive(Clone, Debug)]
pub struct Undo {
    pub text: String,
    pub files: Vec<(PathBuf, i64)>,
}

/// what the page asks of the app
#[derive(Clone, Debug, PartialEq)]
pub enum NAct {
    /// cfg_set(key, value)
    Cfg(String, Value),
    /// life["nb_last"] = value (and life_save)
    NbLast(Value),
    /// say(text, secs)
    Say(String, f64),
    /// os.startfile(path)
    Open(PathBuf),
}

pub struct NotesPage {
    pub pg: usize,
    pub title: usize,
    pub status: usize,
    pub scratch: usize,
    pub ssb: usize,
    pub preview: usize,
    pub tree: usize,
    pub lsb: usize,
    pub page_title: usize,
    pub view: usize,
    pub vsb: usize,
    pub open: BTreeSet<String>,
    /// (depth, path, is a folder)
    pub rows: Vec<(usize, PathBuf, bool)>,
    pub cur: Option<PathBuf>,
    pub cur_dir: Option<PathBuf>,
    pub cur_text: bool,
    /// the view was typed in since it was loaded (edit_modified)
    pub modified: bool,
    pub life: Value,
    pub th: Theme,
    pub wrap: WrapState,
    seig: Option<(String, Rc<Photo>)>,
}

fn set_label(ui: &mut Ui, id: usize, s: &str) {
    if let Kind::Label { text, .. } = &mut ui.ws[id].kind {
        *text = s.into();
    }
}

fn wrapped(ui: &mut Ui, l: usize, wl: i32) {
    if let Kind::Label { wrap, left, .. } = &mut ui.ws[l].kind {
        (*wrap, *left) = (wl, true);
    }
}

fn s(p: &Path) -> String {
    p.to_string_lossy().to_string()
}

fn base(p: &Path) -> String {
    p.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default()
}

/// nb_count(path): the files under a folder (not the dot ones)
pub fn nb_count(path: &Path) -> usize {
    let mut n = 0;
    let Ok(rd) = std::fs::read_dir(path) else { return 0 };
    for e in rd.flatten() {
        let p = e.path();
        if p.is_dir() {
            n += nb_count(&p);
        } else if !base(&p).starts_with('.') {
            n += 1;
        }
    }
    n
}

/// os.path.relpath(p, NB_DIR) with "  ›  " between the parts
fn rel_title(p: &Path) -> String {
    let r = p.strip_prefix(nb_dir()).unwrap_or(p);
    r.iter().map(|x| x.to_string_lossy().to_string()).collect::<Vec<_>>().join("  \u{203a}  ")
}

fn is_text(name: &str) -> bool {
    let l = name.to_lowercase();
    NB_TEXT.iter().any(|e| l.ends_with(e))
}

impl NotesPage {
    /// hub_build_notes(pg): `open` is cfg["nb_open"], `scratch` the draft the scratchpad had
    pub fn build(ui: &mut Ui, pg: usize, th: &Theme, f: &Faces, cm10: i32, life: &Value, open: &[String], scratch: &str) -> NotesPage {
        let title = ui.canvas(pg, &th.banner, cm10, 96);
        canvas_of(ui, title);
        ui.pack(title, Pack::default().fill(Fill::X));
        let outer = ui.frame(pg, &th.paper);
        ui.pack(outer, Pack::default().fill(Fill::Both).expand().padx(24, 24).pady(10, 14));
        let bg = th.card.clone();
        nb_ensure(life);
        let bar = ui.frame(outer, &th.paper);
        ui.pack(bar, Pack::default().fill(Fill::X).pady(0, 10));
        for (i, (t, c)) in [("Import a folder\u{2026}", "notes_import"), ("reMarkable\u{2026}", "notes_rm"),
                            ("Open notebooks folder", "notes_open_dir")].iter().enumerate() {
            let b = tt_btn(ui, bar, t, c, Some(&th.field), None, 9, 10, 2, th, f);
            ui.pack(b, Pack::default().side(Side::Left).padx(if i == 0 { 0 } else { 6 }, 0));
        }
        let status = ui.label(bar, "", &th.paper, &th.grey, Font::new(&f.ui, 9, false));
        ui.set_anchor(status, Anchor::E);
        ui.pack(status, Pack::default().side(Side::Right).fill(Fill::X).expand());
        let cols = ui.frame(outer, &th.paper);
        ui.pack(cols, Pack::default().fill(Fill::Both).expand());
        (ui.ws[cols].gw_cols, ui.ws[cols].gu_cols, ui.ws[cols].gw_rows) = (vec![5, 6], vec!["nb", "nb"], vec![1]);
        let left = hub_card(ui, cols, Some("Scratchpad"), "round", th, f);
        ui.grid(left, Grid { row: 0, col: 0, sticky: "nsew", padx: (0, 6), ..Default::default() });
        let right = hub_card(ui, cols, Some("Notebooks"), "round", th, f);
        ui.grid(right, Grid { row: 0, col: 1, sticky: "nsew", padx: (6, 0), ..Default::default() });
        let l = label_w(ui, left, "Type in a rush, mix subjects, no headings needed. French then Bio then Chem? I'll split it \
                                   up and put each bit in its own notebook.", &bg, &th.grey, &f.ui, 9, false);
        wrapped(ui, l, 380);
        ui.pack(l, Pack::default().fill(Fill::X).padx(14, 14));
        let tf = ui.frame(left, &bg);
        ui.pack(tf, Pack::default().fill(Fill::Both).expand().padx(14, 14).pady(6, 4));
        let mk_text = |ui: &mut Ui, parent: usize, size: i32, pad: (i32, i32), body: &str| {
            let mut t = Text::new(Font::new(&f.ui, size, false), &th.ink, &th.field);
            (t.width_ch, t.height_ln, t.wrap, t.hl, t.bd) = (30, 12, Wrap::Word, 1, 1);
            (t.padx, t.pady) = pad;
            t.insert_end(body, &[]);
            t.edit = Some(0);
            ui.text(parent, t, &th.line)
        };
        let sb_of = |ui: &mut Ui, parent: usize| {
            let sb = ui.canvas(parent, &bg, 10, 40);
            canvas_of(ui, sb);
            sb
        };
        let scratch_t = mk_text(ui, tf, 11, (10, 8), scratch);
        let ssb = sb_of(ui, tf);
        ui.pack(ssb, Pack::default().side(Side::Right).fill(Fill::Y));
        ui.pack(scratch_t, Pack::default().side(Side::Left).fill(Fill::Both).expand());
        let preview = label_w(ui, left, "", &bg, &th.grey, &f.ui, 9, false);
        wrapped(ui, preview, 380);
        ui.pack(preview, Pack::default().fill(Fill::X).padx(14, 14));
        let r = ui.frame(left, &bg);
        ui.pack(r, Pack::default().fill(Fill::X).padx(14, 14).pady(6, 12));
        let b = tt_btn(ui, r, "File it now", "notes_file", Some(&th.acc), Some(&th.onacc), 10, 14, 2, th, f);
        ui.pack(b, Pack::default().side(Side::Left));
        let b = tt_btn(ui, r, "Undo last filing", "notes_undo", Some(&th.field), None, 9, 10, 1, th, f);
        ui.pack(b, Pack::default().side(Side::Left).padx(8, 0));
        let l = label_w(ui, right, "One notebook per subject. Imported folders keep their own folders inside.", &bg, &th.grey,
                        &f.ui, 9, false);
        wrapped(ui, l, 440);
        ui.pack(l, Pack::default().fill(Fill::X).padx(14, 14));
        let nbw = ui.frame(right, &bg);
        ui.pack(nbw, Pack::default().fill(Fill::Both).expand().padx(14, 14).pady(6, 4));
        (ui.ws[nbw].gw_cols, ui.ws[nbw].gu_cols, ui.ws[nbw].gw_rows) = (vec![2, 3], vec!["nbt", "nbt"], vec![1]);
        let lf = ui.frame(nbw, &bg);
        ui.grid(lf, Grid { row: 0, col: 0, sticky: "nsew", padx: (0, 8), ..Default::default() });
        let tree = ui.listbox(lf, 18, Font::new(&f.ui, 10, false), &th.field, &th.ink, &th.line, &th.acc, &th.onacc, "notes_row");
        let lsb = sb_of(ui, lf);
        ui.pack(lsb, Pack::default().side(Side::Right).fill(Fill::Y));
        ui.pack(tree, Pack::default().side(Side::Left).fill(Fill::Both).expand());
        let rf = ui.frame(nbw, &bg);
        ui.grid(rf, Grid { row: 0, col: 1, sticky: "nsew", ..Default::default() });
        let page_title = label_w(ui, rf, "", &bg, &th.gold, &f.ui, 10, true);
        wrapped(ui, page_title, 0);
        ui.pack(page_title, Pack::default().fill(Fill::X));
        let vf = ui.frame(rf, &bg);
        ui.pack(vf, Pack::default().fill(Fill::Both).expand().pady(4, 0));
        let view = mk_text(ui, vf, 10, (8, 6), "");
        let vsb = sb_of(ui, vf);
        ui.pack(vsb, Pack::default().side(Side::Right).fill(Fill::Y));
        ui.pack(view, Pack::default().side(Side::Left).fill(Fill::Both).expand());
        let r2 = ui.frame(right, &bg);
        ui.pack(r2, Pack::default().fill(Fill::X).padx(14, 14).pady(6, 12));
        for (i, (t, c)) in [("New page", "notes_new_page"), ("New folder", "notes_new_folder"), ("Open", "notes_open"),
                            ("Delete", "notes_delete")].iter().enumerate() {
            let col = if *c == "notes_delete" { th.delbtn.clone() } else { th.field.clone() };
            let b = tt_btn(ui, r2, t, c, Some(&col), None, 9, 10, 1, th, f);
            ui.pack(b, Pack::default().side(Side::Left).padx(if i == 0 { 0 } else { 6 }, 0));
        }
        let mut p = NotesPage { pg, title, status, scratch: scratch_t, ssb, preview, tree, lsb, page_title, view, vsb,
                                open: open.iter().cloned().collect(), rows: vec![], cur: None, cur_dir: None, cur_text: false,
                                modified: false, life: life.clone(), th: th.clone(), wrap: WrapState::default(), seig: None };
        p.tree_fill(ui);
        p.preview(ui);
        p.status_last(ui, None);
        let v = ui.text_mut(view);
        v.insert_end(INTRO, &[]);
        v.edit = None;
        p
    }

    pub fn scratch_text(&self, ui: &mut Ui) -> String {
        ui.text_mut(self.scratch).plain()
    }

    /// nb_tree_fill: the subject notebooks, then Unsorted / Imported, then any other folder; open ones show inside
    pub fn tree_fill(&mut self, ui: &mut Ui) {
        let mut rows = vec![];
        let subj: Vec<String> = nb_subjects(&self.life).into_iter().map(|s| s.1).collect();
        let d = nb_dir();
        let mut extra: Vec<String> = std::fs::read_dir(&d).map(|rd| rd.flatten().filter(|e| e.path().is_dir())
            .map(|e| e.file_name().to_string_lossy().to_string())
            .filter(|n| !subj.contains(n) && !n.starts_with('.')).collect()).unwrap_or_default();
        extra.sort();
        let mut tops = subj.clone();
        tops.extend([NB_UNSORTED, NB_IMPORTED].iter().filter(|x| extra.iter().any(|e| e == *x)).map(|x| x.to_string()));
        tops.extend(extra.iter().filter(|e| e.as_str() != NB_UNSORTED && e.as_str() != NB_IMPORTED).cloned());
        fn walk(path: &Path, depth: usize, open: &BTreeSet<String>, rows: &mut Vec<(usize, PathBuf, bool)>) {
            let Ok(rd) = std::fs::read_dir(path) else { return };
            let mut items: Vec<(bool, String, PathBuf)> = rd.flatten().map(|e| {
                let p = e.path();
                (!p.is_dir(), e.file_name().to_string_lossy().to_string(), p)
            }).collect();
            items.sort_by(|a, b| (a.0, a.1.to_lowercase()).cmp(&(b.0, b.1.to_lowercase())));
            for (file, n, p) in items {
                if n.starts_with('.') {
                    continue;
                }
                rows.push((depth, p.clone(), !file));
                if !file && open.contains(&s(&p)) {
                    walk(&p, depth + 1, open, rows);
                }
            }
        }
        for t in tops {
            let p = d.join(&t);
            rows.push((0, p.clone(), true));
            if self.open.contains(&s(&p)) {
                walk(&p, 1, &self.open, &mut rows);
            }
        }
        let gold = self.th.gold.clone();
        let mut items = vec![];
        let mut sel = None;
        for (i, (depth, p, isdir)) in rows.iter().enumerate() {
            let name = base(p);
            let pad = "   ".repeat(*depth);
            let lab = if *isdir {
                let n = nb_count(p);
                format!("{pad}{} {name}{}", if self.open.contains(&s(p)) { "\u{25be}" } else { "\u{25b8}" },
                        if n > 0 { format!("  ({n})") } else { String::new() })
            } else {
                let shown = if is_text(&name) {
                    Path::new(&name).file_stem().map(|x| x.to_string_lossy().to_string()).unwrap_or(name.clone())
                } else {
                    name.clone()
                };
                format!("{pad}   \u{b7} {shown}")
            };
            items.push((lab, if *depth == 0 { Some(gold.clone()) } else { None }));
            if self.cur.as_ref() == Some(p) {
                sel = Some(i);
            }
        }
        self.rows = rows;
        if let Kind::List { items: it, sel: s0, top, .. } = &mut ui.ws[self.tree].kind {
            *it = items;
            *s0 = sel;
            *top = (*top).min(it.len().saturating_sub(1));
        }
    }

    /// nb_preview: which notebooks the scratchpad would go to
    pub fn preview(&mut self, ui: &mut Ui) {
        let body = self.scratch_text(ui);
        if body.trim().is_empty() {
            set_label(ui, self.preview, "When you leave this page (or press File it now) each part goes to its notebook.");
            return;
        }
        let names: Vec<String> = nb_split(&body, &self.life).iter().map(|(sid, t)| {
            let n = sid.as_ref().map(|s| subject_name(s, &self.life, true)).unwrap_or_else(|| NB_UNSORTED.into());
            let nl = t.split('\n').filter(|l| !l.trim().is_empty()).count();
            format!("{n} ({nl} line{})", if nl == 1 { "" } else { "s" })
        }).collect();
        set_label(ui, self.preview, &format!("Will file: {}", names.join(", ")));
    }

    /// nb_status_last: "Last filed 09:42 → French, Bio" (`today` is today's day number, for the time or the date)
    pub fn status_last(&mut self, ui: &mut Ui, today: Option<i64>) {
        let last = &self.life["nb_last"];
        let mut txt = String::new();
        if let Some(to) = last["to"].as_array().filter(|a| !a.is_empty()) {
            let w = last["when"].as_str().unwrap_or("");
            let when = match (crate::plan::parse_iso(w.get(..10).unwrap_or("")), w.get(11..16)) {
                (Some(day), Some(hm)) => {
                    let td = today.unwrap_or_else(|| crate::plan::naive(crate::unix_now()).div_euclid(86400.0) as i64);
                    if day == td {
                        hm.to_string()
                    } else {
                        let (_, m, d) = crate::lifechat::ymd(day);
                        format!("{d:02} {}", ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"]
                            [m as usize - 1])
                    }
                }
                _ => String::new(),
            };
            let mut uniq: Vec<String> = vec![];
            for f in to.iter().filter_map(|v| v.as_str()) {
                if !uniq.iter().any(|u| u == f) {
                    uniq.push(f.into());
                }
            }
            txt = format!("Last filed {when} \u{2192} {}", uniq.join(", "));
        }
        set_label(ui, self.status, &txt);
    }

    /// nb_flush: the scratchpad filed into each subject's Quick notes and emptied; the undo for it
    pub fn flush(&mut self, ui: &mut Ui, quiet: bool, now: f64) -> (Option<Undo>, Vec<NAct>) {
        let body = self.scratch_text(ui);
        let (u, acts) = flush_text(&body, &mut self.life, quiet, now);
        if u.is_none() {
            return (None, acts);
        }
        let _ = std::fs::write(nb_scratch(), "");
        ui.text_mut(self.scratch).delete_all();
        self.tree_fill(ui);
        self.preview(ui);
        self.status_last(ui, None);
        if let Some(c) = self.cur.clone()
            && base(&c) == "Quick notes.md" {
            self.show_file(ui, &c);
        }
        (u, acts)
    }

    /// nb_undo: the filed notes taken back out of the notebooks and into the scratchpad
    pub fn undo(&mut self, ui: &mut Ui, u: Option<Undo>) -> Vec<NAct> {
        let Some(u) = u else {
            set_label(ui, self.status, "Nothing to undo (undo works until Zenko closes).");
            return vec![];
        };
        undo_files(&u);
        self.life["nb_last"] = json!({});
        let t = ui.text_mut(self.scratch);
        t.delete_all();
        t.insert_end(&u.text, &[]);
        set_label(ui, self.status, "Undone: your notes are back in the scratchpad.");
        self.tree_fill(ui);
        self.preview(ui);
        let _ = std::fs::write(nb_scratch(), &u.text);
        vec![NAct::NbLast(json!({}))]
    }

    /// nb_sel: the selected row
    pub fn sel(&self, ui: &Ui) -> Option<(usize, PathBuf, bool)> {
        let Kind::List { sel: Some(i), .. } = &ui.ws[self.tree].kind else { return None };
        self.rows.get(*i).cloned()
    }

    /// nb_tree_click: a folder opens or closes (and shows its count), a file shows
    pub fn tree_click(&mut self, ui: &mut Ui, fonts: &dyn Fonts) -> Vec<NAct> {
        let Some((_, p, isdir)) = self.sel(ui) else { return vec![] };
        if isdir {
            let k = s(&p);
            if !self.open.remove(&k) {
                self.open.insert(k);
            }
            let v: Vec<String> = self.open.iter().cloned().collect();
            let v = v[v.len().saturating_sub(60)..].to_vec();
            self.cur_dir = Some(p.clone());
            self.tree_fill(ui);
            if let Some(i) = self.rows.iter().position(|r| r.1 == p) {
                if let Kind::List { sel, .. } = &mut ui.ws[self.tree].kind {
                    *sel = Some(i);
                }
                ui.list_see(fonts, self.tree, i);
            }
            let inside = self.cur.as_ref().is_some_and(|c| s(c.parent().unwrap_or(Path::new(""))).starts_with(&s(&p)));
            if !inside {
                self.show_folder(ui, &p);
            }
            return vec![NAct::Cfg("nb_open".into(), json!(v))];
        }
        self.cur_dir = p.parent().map(|x| x.to_path_buf());
        self.show_file(ui, &p);
        vec![]
    }

    fn view_set(&mut self, ui: &mut Ui, body: &str, edit: bool) {
        let v = ui.text_mut(self.view);
        v.delete_all();
        v.insert_end(body, &[]);
        v.top = (1, 0);
        v.top_offset = 0;
        v.edit = if edit { Some(0) } else { None };
    }

    /// nb_show_folder: how many files, and what to do
    pub fn show_folder(&mut self, ui: &mut Ui, p: &Path) {
        self.page_save(ui);
        self.cur = None;
        let n = nb_count(p);
        set_label(ui, self.page_title, &rel_title(p));
        self.view_set(ui, &format!("{n} file{} here.\n\nPick a page on the left to read or edit it. New page adds one here; \
                                    notes you write in the Scratchpad about this subject go into \u{201c}Quick notes\u{201d}.",
                                   if n == 1 { "" } else { "s" }), false);
    }

    /// nb_show_file: a text page to read and edit, or what kind of file it is
    pub fn show_file(&mut self, ui: &mut Ui, p: &Path) {
        self.page_save(ui);
        self.cur = Some(p.to_path_buf());
        set_label(ui, self.page_title, &rel_title(p));
        let ext = p.extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default();
        self.modified = false;
        if is_text(&base(p)) {
            let t = nb_file_text(p, 200000);
            self.view_set(ui, &t, true);
            self.cur_text = true;
            return;
        }
        self.cur_text = false;
        let up = if ext.is_empty() { "plain".to_string() } else { ext.to_uppercase() };
        // (a PDF's typed text isn't read yet: it shows like any other file)
        self.view_set(ui, &format!("This is a {up} file. Press Open file to open it in its own app."), false);
    }

    /// nb_page_save: a typed-in page written back
    pub fn page_save(&mut self, ui: &mut Ui) {
        if !self.modified || !self.cur_text {
            return;
        }
        let Some(c) = self.cur.clone() else { return };
        let body = ui.text_mut(self.view).plain();
        if std::fs::write(&c, body).is_ok() {
            self.modified = false;
        }
    }

    /// nb_target_dir: the selected folder (or the selected file's), else the folder last opened, else Unsorted
    pub fn target_dir(&self, ui: &Ui) -> PathBuf {
        if let Some((_, p, isdir)) = self.sel(ui) {
            return if isdir { p } else { p.parent().map(|x| x.to_path_buf()).unwrap_or(p) };
        }
        self.cur_dir.clone().unwrap_or_else(|| nb_dir().join(NB_UNSORTED))
    }

    /// nb_new_page: "Page N.md" in the target folder, opened
    pub fn new_page(&mut self, ui: &mut Ui) {
        let d = self.target_dir(ui);
        let _ = std::fs::create_dir_all(&d);
        let mut i = 1;
        while d.join(format!("Page {i}.md")).exists() {
            i += 1;
        }
        let p = d.join(format!("Page {i}.md"));
        let _ = std::fs::write(&p, "");
        self.open.insert(s(&d));
        self.tree_fill(ui);
        self.show_file(ui, &p);
        ui.focus = Some(self.view);
    }

    /// nb_new_folder(name)
    pub fn new_folder(&mut self, ui: &mut Ui, name: &str) {
        if name.is_empty() {
            return;
        }
        let d = self.target_dir(ui);
        let p = d.join(nb_safe(name));
        let _ = std::fs::create_dir_all(&p);
        self.open.insert(s(&d));
        self.open.insert(s(&p));
        self.tree_fill(ui);
    }

    /// nb_delete's question (None: nothing to delete, or a subject notebook, which stays)
    pub fn delete_ask(&self, ui: &mut Ui) -> Option<String> {
        let (depth, p, _) = self.sel(ui)?;
        let subj: Vec<String> = nb_subjects(&self.life).into_iter().map(|s| s.1).collect();
        if depth == 0 && subj.contains(&base(&p)) {
            set_label(ui, self.status, "Subject notebooks stay. Delete the pages inside instead.");
            return None;
        }
        Some(format!("Move \u{201c}{}\u{201d} to the notebooks bin?\n(It goes to the .bin folder inside your notebooks folder, \
                      so you can still get it back.)", base(&p)))
    }

    /// nb_delete once you said yes: into NB_DIR/.bin/<time>/
    pub fn delete(&mut self, ui: &mut Ui, stamp: &str) {
        let Some((_, p, _)) = self.sel(ui) else { return };
        let binp = nb_dir().join(".bin").join(stamp);
        let ok = std::fs::create_dir_all(&binp).is_ok() && std::fs::rename(&p, binp.join(base(&p))).is_ok();
        if !ok {
            set_label(ui, self.status, "Couldn't move it.");
            return;
        }
        if self.cur.as_ref().is_some_and(|c| s(c).starts_with(&s(&p))) {
            self.cur = None;
            self.view_set(ui, "", false);
            set_label(ui, self.page_title, "");
        }
        self.tree_fill(ui);
    }

    /// the three slim scrollbars for their boxes' views now
    pub fn draw_bars(&self, ui: &mut Ui, fonts: &dyn Fonts) {
        let th = self.th.clone();
        for (t, sb) in [(self.scratch, self.ssb), (self.view, self.vsb)] {
            let (lo, hi) = ui.text_mut(t).yview(fonts);
            scrollbar_draw(ui, sb, lo, hi, false, &th);
        }
        let (lo, hi) = ui.list_yview(fonts, self.tree);
        scrollbar_draw(ui, self.lsb, lo, hi, false, &th);
    }

    /// zk_fit for the page and the banner for its width; lays out at the window's size (pw, ph)
    pub fn fit(&mut self, ui: &mut Ui, fonts: &dyn Fonts, th: &Theme, f: &Faces, zk_f: f64, pw: i32, ph: i32) {
        zk_fonts(ui, fonts, self.pg, zk_f);
        ui.layout_at(fonts, pw, ph);
        let w = ui.ws[self.title].w;
        if w >= 20 {
            let mut cv = std::mem::take(canvas_of(ui, self.title));
            let hh = hub_title_draw(&mut cv, w, TITLE, SUB, th, &f.ui, &f.title, zk_f, fonts, &mut self.seig);
            *canvas_of(ui, self.title) = cv;
            if ui.ws[self.title].opt_h != hh {
                ui.ws[self.title].opt_h = hh;
                ui.layout_at(fonts, pw, ph);
            }
        }
        zk_wrap(ui, fonts, self.pg, &mut self.wrap, pw, ph);
        ui.layout_at(fonts, pw, ph);
        self.draw_bars(ui, fonts);
    }
}

/// nb_flush's filing, on any text (the page's scratchpad, or the draft file when the page isn't open): each part
/// appended to its notebook's Quick notes under a time stamp, life["nb_last"] set
pub fn flush_text(body: &str, life: &mut Value, quiet: bool, now: f64) -> (Option<Undo>, Vec<NAct>) {
    if body.trim().is_empty() {
        return (None, vec![]);
    }
    let parts = nb_split(body, life);
    if parts.is_empty() {
        return (None, vec![]);
    }
    let nl = crate::plan::naive(now);
    let day = nl.div_euclid(86400.0) as i64;
    let secs = nl.rem_euclid(86400.0) as i64;
    let (y, m, d) = crate::lifechat::ymd(day);
    const MON: [&str; 12] = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
    const WD: [&str; 7] = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];
    let stamp = format!("{} {d:02} {} {y}, {:02}:{:02}", WD[crate::lifechat::weekday(day)], MON[m as usize - 1], secs / 3600,
                        secs / 60 % 60);
    let mut undo = Undo { text: body.to_string(), files: vec![] };
    let mut filed: Vec<String> = vec![];
    for (sid, t) in &parts {
        let folder = sid.as_ref().map(|s| nb_folder_for(s, life)).unwrap_or_else(|| NB_UNSORTED.into());
        let path = nb_dir().join(&folder).join("Quick notes.md");
        if std::fs::create_dir_all(path.parent().unwrap()).is_err() {
            continue;
        }
        let before = std::fs::metadata(&path).map(|m| m.len() as i64).unwrap_or(-1);
        let mut add = String::new();
        if before <= 0 {
            let nm = sid.as_ref().map(|s| subject_name(s, life, false)).unwrap_or_else(|| NB_UNSORTED.into());
            add.push_str(&format!("# {nm} \u{b7} quick notes\n"));
        }
        add.push_str(&format!("\n## {stamp}\n\n{}\n", t.trim_matches('\n')));
        use std::io::Write;
        let ok = std::fs::OpenOptions::new().create(true).append(true).open(&path).and_then(|mut f| f.write_all(add.as_bytes()));
        if ok.is_ok() {
            undo.files.push((path, before));
            filed.push(sid.as_ref().map(|s| subject_name(s, life, true)).unwrap_or_else(|| NB_UNSORTED.into()));
        }
    }
    let when = format!("{}T{:02}:{:02}", crate::plan::iso(day), secs / 3600, secs / 60 % 60);
    let last = json!({"when": when, "to": filed});
    life["nb_last"] = last.clone();
    let mut acts = vec![NAct::NbLast(last)];
    if !quiet && !filed.is_empty() {
        let mut uniq: Vec<String> = vec![];
        for f in &filed {
            if !uniq.contains(f) {
                uniq.push(f.clone());
            }
        }
        let where_ = if uniq.len() > 1 {
            format!("{} and {}", uniq[..uniq.len() - 1].join(", "), uniq[uniq.len() - 1])
        } else {
            uniq[0].clone()
        };
        acts.push(NAct::Say(format!("Filed your notes into {where_}~"), 5.0));
    }
    (Some(undo), acts)
}

/// nb_undo's files: each notebook cut back to its size before (or removed when it was new)
pub fn undo_files(u: &Undo) {
    for (path, before) in &u.files {
        if *before <= 0 {
            let _ = std::fs::remove_file(path);
        } else if let Ok(f) = std::fs::OpenOptions::new().write(true).open(path) {
            let _ = f.set_len(*before as u64);
        }
    }
}
