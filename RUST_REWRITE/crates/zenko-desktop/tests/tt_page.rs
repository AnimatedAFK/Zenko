//! The hub's Timetable page against zenko.py's (golden/hub_timetable.json): a day with cards in every state (missed,
//! done, NOW, in N min, later, repeating), the same in a small midnight window, tomorrow, and an empty day; every
//! widget's place and size, texts and fonts, the progress bar and the fox picture.
mod common;
use common::{check_items, cmp_tree, find, fonts_of};
use serde_json::{Value, json};
use zenko_desktop::pagekit::Faces;
use zenko_desktop::ttpage::TtPage;
use zenko_desktop::ui::{Kind, Ui};

#[test]
fn tt_page_matches_zenko_py() {
    let g: Vec<Value> = serde_json::from_str(&std::fs::read_to_string("../../golden/hub_timetable.json").unwrap()).unwrap();
    let dir = std::env::temp_dir().join(format!("zk_tt_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("FoxBuddy")).unwrap();
    unsafe { std::env::set_var("APPDATA", &dir) };
    let mut fails = vec![];
    for c in &g {
        let name = c["name"].as_str().unwrap();
        let now = c["now"].as_f64().unwrap();
        zenko_desktop::set_fake_now(Some(now));
        std::fs::write(dir.join("FoxBuddy").join("timetable.json"), c["tt"].to_string()).unwrap();
        let mut cfg = c["cfg_all"].clone();
        cfg["robe"] = json!("none");
        let mut fox = zenko_desktop::Companion::new(zenko_desktop::Cfg::from(cfg), 1.0, now);
        fox.tt = c["tt"].as_array().cloned().unwrap_or_default();
        let d = fox.tt_data();
        let im = fox.small_fox(now);
        let icon = std::rc::Rc::new(zenko_desktop::tkcanvas::Photo { w: im.w, h: im.h, px: im.px });
        let th = zenko_desktop::theme::theme(&zenko_desktop::Cfg::from(json!({"theme": c["cfg_all"]["theme"]})));
        let mut fonts = fonts_of(c);
        let small = name.contains("small");
        if small {
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
        let faces = Faces { ui: "Segoe UI".into(), title: String::new() };
        let mut ui = Ui::new(&th.paper);
        ui.x11 = true;
        let mut tp = TtPage::build(&mut ui, 0, &th, &faces, icon.clone(), &d);
        if name == "tt_tomorrow" {
            tp.day = 1;
            tp.refresh(&mut ui, &th, &faces, &d);
        }
        if small {
            // as on the Daily page: fonts scaled at the first size, the first wrap at the new size
            zenko_desktop::pagekit::zk_fonts(&mut ui, &fonts, 0, g[0]["zk_f"].as_f64().unwrap());
            ui.layout_at(&fonts, pw, ph);
            tp.sync(&mut ui);
            zenko_desktop::pagekit::zk_wrap(&mut ui, &fonts, 0, &mut tp.wrap, pw, ph);
        }
        tp.fit(&mut ui, &fonts, c["zk_f"].as_f64().unwrap(), pw, ph);
        let mut bad = vec![];
        cmp_tree(&ui, 0, tree, ox, oy, name, &mut bad);
        // the progress bar
        let mut bars = vec![];
        find(tree, &|n| n["cls"] == "Canvas" && n["h"] == 14, &mut bars);
        let Kind::Canvas { tk: Some(cv), .. } = &ui.ws[tp.bar].kind else { panic!() };
        let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            check_items(&format!("{name} bar"), cv, bars[0]["items"].as_array().unwrap(), &c["images"], &fonts)
        }));
        if r.is_err() {
            bad.push(format!("{name}: bar"));
        }
        // the fox
        let mut pics = vec![];
        find(tree, &|n| n["cls"] == "Label" && n["opts"]["image"].as_str().is_some_and(|s| !s.is_empty()), &mut pics);
        let rows = c["images"][pics[0]["opts"]["image"].as_str().unwrap()].as_array().unwrap();
        'px: for (y, row) in rows.iter().enumerate() {
            for (x, px) in row.as_array().unwrap().iter().enumerate() {
                let q = icon.px[y * icon.w + x];
                let got = format!("#{:02x}{:02x}{:02x}{:02x}", q[0], q[1], q[2], q[3]);
                if px.as_str().unwrap() != got {
                    bad.push(format!("{name} fox ({x},{y}): {got} want {px}"));
                    break 'px;
                }
            }
        }
        for b in &bad {
            eprintln!("{b}");
        }
        if !bad.is_empty() {
            fails.push(format!("{name}: {}", bad.len()));
        }
    }
    zenko_desktop::set_fake_now(None);
    assert!(fails.is_empty(), "{fails:?}");
}
