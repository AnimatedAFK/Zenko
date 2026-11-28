//! Open study books against zenko.py's (golden/hub_book.json): each subject's book (Zenko at her blackboard, the
//! practice corner made with the same random numbers, the tutor brain, the tutor's right page with sources, topics
//! and earlier pages), maths in a small midnight window, the study diary and the grades book. Every widget's place,
//! size, text and font, the blackboard canvas, and what the practice corners wrote back to the life file.
mod common;
use common::{check_items, cmp_tree, expand_images, fonts_of};
use serde_json::{Value, json};
use zenko_desktop::bookpage::BookPage;
use zenko_desktop::ui::{Kind, Ui};
use zenko_sound::mt::PyRandom;

#[test]
fn books_match_zenko_py() {
    let g: Vec<Value> = serde_json::from_str(&std::fs::read_to_string("../../golden/hub_book.json").unwrap()).unwrap();
    let dir = std::env::temp_dir().join(format!("zk_book_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("FoxBuddy")).unwrap();
    unsafe { std::env::set_var("APPDATA", &dir) };
    let only = std::env::var("ZK_CASE").unwrap_or_default();
    let mut fails = vec![];
    for c in &g {
        let name = c["name"].as_str().unwrap();
        if !only.is_empty() && only != name {
            continue;
        }
        eprintln!("== {name}");
        let now = c["now"].as_f64().unwrap();
        zenko_desktop::set_fake_now(Some(now));
        std::fs::write(dir.join("FoxBuddy").join("zenko_life.json"), c["life_before"].to_string()).unwrap();
        let mut fox = zenko_desktop::Companion::new(zenko_desktop::Cfg::from(c["cfg_before"].clone()), 1.0, now);
        let key = c["book"].as_str().unwrap();
        let mut d = fox.book_data(key);
        d.jp = c["mincho"].as_str().unwrap().into();
        d.serif = c["serif_font"].as_str().unwrap().into();
        d.jpg = c["jp_font"].as_str().filter(|s| !s.is_empty()).unwrap_or("Segoe UI").into();
        d.now = now;
        let th = zenko_desktop::theme::theme(&zenko_desktop::Cfg::from(json!({"theme": c["cfg_all"]["theme"]})));
        let mut fonts = fonts_of(c);
        let tree = &c["tree"];
        let (pw, ph) = (tree["w"].as_i64().unwrap() as i32, tree["h"].as_i64().unwrap() as i32);
        let (ox, oy) = (tree["x"].as_i64().unwrap() as i32, tree["y"].as_i64().unwrap() as i32);
        let cm10: i32 = tree["kids"][0]["kids"][1]["opts"]["width"].as_str().unwrap().parse().unwrap();
        let mut ui = Ui::new(&th.paper);
        ui.x11 = true;
        let mut p = BookPage::build(&mut ui, 0, &th, "Segoe UI", cm10, d, PyRandom::new(77));
        p.brains_done(&mut ui, &[]);
        // (the recording has no text widths for option menus: their requested width gives them)
        let mut gm = vec![];
        common::find(tree, &|n| n["cls"] == "Menubutton", &mut gm);
        for w in ui.ws.iter() {
            if let Kind::Menu { text, font, ind, .. } = &w.kind
                && let Some(g) = gm.iter().find(|g| g["opts"]["text"] == text.as_str()) {
                let tw = g["rw"].as_i64().unwrap() as i32 - ind.0 - 2 * w.padx - 2 * (w.bd + w.hl);
                let mut f = font.clone();
                f.size = zenko_desktop::hubside::zk_size(c["zk_f"].as_f64().unwrap(), font.size);
                fonts.0.get_mut(&(f.family.clone(), f.size, f.bold, f.italic)).unwrap().2.insert(text.clone(), tw);
            }
        }
        p.fit(&mut ui, &fonts, 0, c["zk_f"].as_f64().unwrap(), pw, ph);
        let mut bad = vec![];
        cmp_tree(&ui, 0, tree, ox, oy, name, &mut bad);
        // the blackboard: her pointer tells how many steps she has taken
        let mut cvs = vec![];
        common::find(tree, &|n| n["cls"] == "Canvas" && n["opts"]["width"] == "326" && n["opts"]["height"] == "196", &mut cvs);
        let gi = cvs[0]["items"].as_array().unwrap();
        let wig = gi[10]["coords"][2].as_f64().unwrap() - 188.0;
        let i = (1..40).find(|&i| (3.0 * (i as f64 * 0.9).sin() - wig).abs() < 1e-6).expect("teach step");
        p.teach.i = i - 1;
        p.teach_step(&mut ui, now);
        let mut imgs = c["images"].clone();
        expand_images(&mut imgs);
        imgs["__fonts"] = c["fonts"].clone();
        let Kind::Canvas { tk: Some(cv), .. } = &ui.ws[p.teach.cv].kind else { panic!() };
        let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| check_items(&format!("{name} teach"), cv, gi, &imgs, &fonts)));
        if r.is_err() {
            bad.push("teach canvas".into());
        }
        // what the practice corner remembered
        for k in ["book_state", "fr_vocab"] {
            if p.life[k] != c["life_all"][k] {
                bad.push(format!("life {k}: {} want {}", p.life[k], c["life_all"][k]));
            }
        }
        for b in &bad {
            eprintln!("{b}");
        }
        if !bad.is_empty() {
            fails.push(format!("{name}: {}", bad.len()));
        }
    }
    assert!(fails.is_empty(), "{fails:?}");
}
