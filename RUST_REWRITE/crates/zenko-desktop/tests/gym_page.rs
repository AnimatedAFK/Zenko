//! The hub's Exercise page against zenko.py's (golden/hub_gym.json): no day picked yet, day 1 with ticks / weights /
//! food, day 2 in a small midnight window, the rest day, and day 1 while looking at the legs day; every widget's
//! place and size, texts and fonts, and every canvas's items (the banner, the glow figures' pixels, the meters).
mod common;
use common::{check_items, cmp_tree, fonts_of};

use serde_json::Value;
use zenko_desktop::gympage::{GymData, GymPage};
use zenko_desktop::pagekit::Faces;
use zenko_desktop::ui::Ui;

#[test]
fn gym_page_matches_zenko_py() {
    let g: Vec<Value> = serde_json::from_str(&std::fs::read_to_string("../../golden/hub_gym.json").unwrap()).unwrap();
    let mut fails = vec![];
    for c in &g {
        let name = c["name"].as_str().unwrap();
        eprintln!("case {name}");
        let th = zenko_desktop::theme::theme(&zenko_desktop::Cfg::from(serde_json::json!({"theme": c["cfg_all"]["theme"]})));
        let fonts = fonts_of(c);
        let tree = &c["tree"];
        let (pw, ph) = (tree["w"].as_i64().unwrap() as i32, tree["h"].as_i64().unwrap() as i32);
        let (ox, oy) = (tree["x"].as_i64().unwrap() as i32, tree["y"].as_i64().unwrap() as i32);
        let cm10 = tree["kids"][0]["rw"].as_i64().unwrap() as i32;
        let faces = Faces { ui: "Segoe UI".into(), title: String::new() };
        let now = c["now"].as_f64().unwrap();
        let today = (now / 86400.0).floor() as i64;
        let d = GymData { gym: c["life_all"]["gym"].clone(), today_iso: zenko_desktop::plan::iso(today), today,
                          jp: c["jp"].as_str().unwrap().into(), serif: "Liberation Serif".into() };
        let mut ui = Ui::new(&th.paper);
        ui.x11 = true;
        let mut p = GymPage::build(&mut ui, 0, &th, &faces, cm10, &d, 0.0);
        if name == "gym_view_legs" {
            p.view = 2;
            p.render(&mut ui, cm10, &d, 0.0);
        }
        p.fit(&mut ui, &fonts, &th, &faces, c["zk_f"].as_f64().unwrap(), pw, ph);
        let mut bad = vec![];
        cmp_tree(&ui, 0, tree, ox, oy, name, &mut bad);
        for b in bad.iter().take(30) {
            eprintln!("{b}");
        }
        if !bad.is_empty() {
            fails.push(format!("{name}: {}", bad.len()));
            continue;
        }
        let mut out = vec![];
        common::find(tree, &|n| n["cls"] == "Frame" && n["pack"]["padx"] == "24", &mut out);
        let mut pairs = vec![];
        pair(&ui, p.body, out[0], &mut pairs);
        let mut imgs = c["images"].clone();
        common::expand_images(&mut imgs);
        let mut n = 0;
        for (id, gn) in pairs {
            if let Some(cv) = ui.ws[id].tk_canvas() {
                check_items(&format!("{name} canvas {n}"), cv, gn["items"].as_array().unwrap(), &imgs, &fonts);
                n += 1;
            }
        }
        assert!(n >= 4, "{name}: {n} canvases");
    }
    assert!(fails.is_empty(), "{fails:?}");
}

fn pair<'a>(ui: &Ui, id: usize, g: &'a Value, out: &mut Vec<(usize, &'a Value)>) {
    if g["cls"] == "Canvas" && g["items"].is_array() && !g["items"].as_array().unwrap().is_empty()
        && ui.ws[id].tk_canvas().is_some() {
        out.push((id, g));
    }
    for (&k, gk) in ui.ws[id].kids.iter().zip(g["kids"].as_array().unwrap()) {
        pair(ui, k, gk, out);
    }
}
