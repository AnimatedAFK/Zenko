//! The hub's About you page against zenko.py's (golden/hub_you.json): empty, filled in (small midnight window) and
//! with things she has learned; every widget's place and size, texts and fonts.
mod common;
use common::{cmp_tree, fonts_of};
use serde_json::{Value, json};
use zenko_desktop::pagekit::Faces;
use zenko_desktop::ui::Ui;
use zenko_desktop::youpage::YouPage;

#[test]
fn you_page_matches_zenko_py() {
    let g: Vec<Value> = serde_json::from_str(&std::fs::read_to_string("../../golden/hub_you.json").unwrap()).unwrap();
    let dir = std::env::temp_dir().join(format!("zk_you_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("FoxBuddy")).unwrap();
    unsafe { std::env::set_var("APPDATA", &dir) };
    let mut fails = vec![];
    for c in &g {
        let name = c["name"].as_str().unwrap();
        let learned = dir.join("FoxBuddy").join("learned.json");
        if name == "you_learned" {
            std::fs::write(&learned, json!({"replies": {"good morning": "Ohayo!", "gn": "Oyasumi~"}, "sessions": [],
                                            "style": {"aliases": {"sched": "timetable", "hw": "homework"}}}).to_string()).unwrap();
        } else {
            let _ = std::fs::remove_file(&learned);
        }
        let th = zenko_desktop::theme::theme(&zenko_desktop::Cfg::from(json!({"theme": c["cfg_all"]["theme"]})));
        let mut fonts = fonts_of(c);
        if name == "you_filled" {
            for (k, v) in fonts_of(&g[0]).0 {
                let e = fonts.0.entry(k).or_insert_with(|| v.clone());
                for (s, n) in v.2 {
                    e.2.entry(s).or_insert(n);
                }
            }
        }
        let tree = &c["tree"];
        let (pw, ph) = (tree["w"].as_i64().unwrap() as i32, tree["h"].as_i64().unwrap() as i32);
        let (ox, oy) = (tree["x"].as_i64().unwrap() as i32, tree["y"].as_i64().unwrap() as i32);
        let cm10 = tree["kids"][0]["rw"].as_i64().unwrap() as i32;
        let faces = Faces { ui: "Segoe UI".into(), title: String::new() };
        let mut ui = Ui::new(&th.paper);
        ui.x11 = true;
        let summary = zenko_desktop::chat::learned_summary();
        let mut yp = YouPage::build(&mut ui, 0, &th, &faces, cm10, &c["cfg_all"], &summary);
        let zk = c["zk_f"].as_f64().unwrap();
        if name == "you_filled" {
            let big = &g[0];
            let (bw, bh) = (big["tree"]["w"].as_i64().unwrap() as i32, big["tree"]["h"].as_i64().unwrap() as i32);
            yp.fit(&mut ui, &fonts, &th, &faces, big["zk_f"].as_f64().unwrap(), bw, bh);
            zenko_desktop::pagekit::zk_wrap(&mut ui, &fonts, 0, &mut yp.wrap, pw, ph);
        }
        yp.fit(&mut ui, &fonts, &th, &faces, zk, pw, ph);
        let mut bad = vec![];
        cmp_tree(&ui, 0, tree, ox, oy, name, &mut bad);
        for b in &bad {
            eprintln!("{b}");
        }
        if !bad.is_empty() {
            fails.push(format!("{name}: {}", bad.len()));
        }
    }
    assert!(fails.is_empty(), "{fails:?}");
}
