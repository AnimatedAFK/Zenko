//! The hub's Notes page against zenko.py's (golden/hub_notes.json): every widget's place and size (a fresh page in a
//! small midnight window, and the page after it was used), and the page used step by step: folders opened and closed
//! and pages picked in the notebooks list, the scratchpad filed and the filing undone, a new page written. After every
//! step: the list's rows, colours and selection, the page's title and text (and whether it can be typed in), the
//! status, the preview, the scratchpad, the open folders, life["nb_last"] and every file in the notebooks folder.
mod common;
use common::{cmp_tree, fonts_of};
use serde_json::{Value, json};
use std::path::Path;
use zenko_desktop::notes::{nb_dir, nb_scratch};
use zenko_desktop::notespage::NotesPage;
use zenko_desktop::pagekit::Faces;
use zenko_desktop::ui::{Kind, Ui};

const SCRATCH: &str = "Bonjour, je m'appelle Anoop.\nLe passé composé uses avoir or être.\nMitosis has four phases.\n\
                       The cell membrane is a phospholipid bilayer.\n\nmoles = mass / molar mass";

fn setup(life: &Value) {
    let _ = std::fs::remove_dir_all(nb_dir());
    zenko_desktop::notes::nb_ensure(life);
    let bio = nb_dir().join("Biology HL");
    std::fs::create_dir_all(bio.join("Cells")).unwrap();
    std::fs::write(bio.join("Cells").join("membranes.md"), "# Membranes\n\nPhospholipid bilayer, fluid mosaic.").unwrap();
    std::fs::write(bio.join("Genetics.txt"), "DNA, genes, alleles.").unwrap();
    std::fs::write(bio.join("diagram.png"), "x").unwrap();
    std::fs::create_dir_all(nb_dir().join("Imported").join("reMarkable")).unwrap();
    std::fs::write(nb_dir().join("Imported").join("reMarkable").join("Quick.md"), "hi").unwrap();
    std::fs::create_dir_all(nb_dir().join("Zeta extra")).unwrap();
    std::fs::write(nb_scratch(), SCRATCH).unwrap();
}

fn files() -> Vec<Value> {
    fn walk(d: &Path, out: &mut Vec<Value>) {
        let mut ents: Vec<_> = std::fs::read_dir(d).unwrap().flatten().collect();
        ents.sort_by_key(|e| e.file_name());
        let (dirs, fs): (Vec<_>, Vec<_>) = ents.into_iter().partition(|e| e.path().is_dir());
        for f in fs {
            let p = f.path();
            out.push(json!([p.strip_prefix(nb_dir()).unwrap().to_string_lossy(),
                            String::from_utf8_lossy(&std::fs::read(&p).unwrap())]));
        }
        for d in dirs {
            walk(&d.path(), out);
        }
    }
    let mut v = vec![];
    walk(&nb_dir(), &mut v);
    v
}

fn state(ui: &mut Ui, p: &NotesPage) -> Value {
    let Kind::List { items, sel, .. } = &ui.ws[p.tree].kind else { panic!() };
    let rows: Vec<&str> = items.iter().map(|i| i.0.as_str()).collect();
    let fg: Vec<&str> = items.iter().map(|i| i.1.as_deref().unwrap_or("")).collect();
    let sel: Vec<usize> = sel.iter().copied().collect();
    let lab = |id: usize| ui.text_of(id);
    let (title, status, preview) = (lab(p.page_title), lab(p.status), lab(p.preview));
    let (rows, fg) = (json!(rows), json!(fg));
    let view_state = if ui.text_mut(p.view).edit.is_some() { "normal" } else { "disabled" };
    let view = ui.text_mut(p.view).plain();
    let scratch = ui.text_mut(p.scratch).plain();
    let open: Vec<String> = p.open.iter().map(|o| Path::new(o).strip_prefix(nb_dir()).unwrap().to_string_lossy().to_string()).collect();
    json!({"rows": rows, "fg": fg, "sel": sel, "title": title, "view": view, "view_state": view_state, "status": status,
           "preview": preview, "scratch": scratch, "files": files(), "nb_open": open, "life_nb_last": p.life["nb_last"]})
}

#[test]
fn notes_page_matches_zenko_py() {
    let g: Vec<Value> = serde_json::from_str(&std::fs::read_to_string("../../golden/hub_notes.json").unwrap()).unwrap();
    let dir = std::env::temp_dir().join(format!("zk_notes_pg_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("FoxBuddy")).unwrap();
    unsafe { std::env::set_var("APPDATA", &dir) };
    let mut fails = vec![];
    for c in &g {
        let name = c["name"].as_str().unwrap();
        let now = c["now"].as_f64().unwrap();
        zenko_desktop::set_fake_now(Some(now));
        let gdir = c["nb_dir"].as_str().unwrap();
        let mut life = c["life_all"].clone();
        if name == "notes" {
            life["nb_last"] = json!({"when": "2025-12-10T09:05", "to": ["Bio", "Chem", "Bio"]});
        }
        setup(&life);
        let open: Vec<String> = c["cfg_full"]["nb_open"].as_array().map(|a| a.iter().filter_map(|v| v.as_str())
            .map(|s| nb_dir().join(Path::new(s).strip_prefix(gdir).unwrap()).to_string_lossy().to_string()).collect())
            .unwrap_or_default();
        let open = if name == "notes" { vec![nb_dir().join("Biology HL").to_string_lossy().to_string()] } else { open };
        let th = zenko_desktop::theme::theme(&zenko_desktop::Cfg::from(json!({"theme": c["cfg_all"]["theme"]})));
        let fonts = fonts_of(c);
        let tree = &c["tree"];
        let (pw, ph) = (tree["w"].as_i64().unwrap() as i32, tree["h"].as_i64().unwrap() as i32);
        let (ox, oy) = (tree["x"].as_i64().unwrap() as i32, tree["y"].as_i64().unwrap() as i32);
        let cm10 = tree["kids"][0]["rw"].as_i64().unwrap() as i32;
        let faces = Faces { ui: "Segoe UI".into(), title: String::new() };
        let mut ui = Ui::new(&th.paper);
        ui.x11 = true;
        let mut p = NotesPage::build(&mut ui, 0, &th, &faces, cm10, &life, &open, SCRATCH);
        let zk = c["zk_f"].as_f64().unwrap();
        p.fit(&mut ui, &fonts, &th, &faces, zk, pw, ph);
        let mut undo = None;
        let today = Some(zenko_desktop::plan::naive(now).div_euclid(86400.0) as i64);
        p.status_last(&mut ui, today);
        for (i, st) in c["steps"].as_array().unwrap().iter().enumerate() {
            let s = &st["step"];
            match s[0].as_str().unwrap() {
                "start" => {}
                "pick" | "new_page" => {
                    let lab = if s[0] == "pick" { s[1].as_str().unwrap() } else { "Chemistry HL" };
                    let k = p.rows.iter().position(|r| r.1.file_name().unwrap().to_string_lossy() == lab).unwrap();
                    if let Kind::List { sel, .. } = &mut ui.ws[p.tree].kind {
                        *sel = Some(k);
                    }
                    p.tree_click(&mut ui, &fonts);
                    if s[0] == "new_page" {
                        p.new_page(&mut ui);
                    }
                }
                "flush" => undo = p.flush(&mut ui, false, now).0,
                "undo" => {
                    p.undo(&mut ui, undo.take());
                }
                "page_save" => {
                    ui.text_mut(p.view).insert_end("Acids donate protons.", &[]);
                    p.modified = true;
                    p.page_save(&mut ui);
                }
                _ => unreachable!(),
            }
            let mine = state(&mut ui, &p);
            for (k, v) in st.as_object().unwrap() {
                if k == "step" {
                    continue;
                }
                let mut m = mine[k.as_str()].clone();
                let mut want = v.clone();
                if k == "nb_open" {
                    let mut a: Vec<String> = serde_json::from_value(want).unwrap();
                    a.sort();
                    want = json!(a);
                    let mut b: Vec<String> = serde_json::from_value(m).unwrap();
                    b.sort();
                    m = json!(b);
                }
                if k == "files" {
                    let mut a = want.as_array().unwrap().clone();
                    a.sort_by_key(|x| x.to_string());
                    want = json!(a);
                    let mut b = m.as_array().unwrap().clone();
                    b.sort_by_key(|x| x.to_string());
                    m = json!(b);
                }
                if m != want {
                    fails.push(format!("{name} step {i} {s}: {k}: {want} vs {m}"));
                }
            }
        }
        p.fit(&mut ui, &fonts, &th, &faces, zk, pw, ph);
        let mut bad = vec![];
        cmp_tree(&ui, 0, tree, ox, oy, name, &mut bad);
        for b in &bad {
            eprintln!("{b}");
        }
        if !bad.is_empty() {
            fails.push(format!("{name}: {} layout differences", bad.len()));
        }
    }
    for f in &fails {
        eprintln!("{f}");
    }
    assert!(fails.is_empty(), "{} differences", fails.len());
}
