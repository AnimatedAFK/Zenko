//! The hub's Screen time page against zenko.py's (golden/hub_screen.json): today, the last 7 days (small midnight
//! window), the last 30 days with its own skipped words, paused, and off with nothing recorded; every widget's place
//! and size, texts and fonts, and the chart's and bars' canvas items.
mod common;
use common::{check_items, cmp_tree, fonts_of};
use serde_json::Value;
use zenko_desktop::pagekit::Faces;
use zenko_desktop::screenpage::{ScreenData, ScreenPage};
use zenko_desktop::ui::Ui;

#[test]
fn screen_page_matches_zenko_py() {
    let g: Vec<Value> = serde_json::from_str(&std::fs::read_to_string("../../golden/hub_screen.json").unwrap()).unwrap();
    let mut fails = vec![];
    for c in &g {
        let name = c["name"].as_str().unwrap();
        let th = zenko_desktop::theme::theme(&zenko_desktop::Cfg::from(serde_json::json!({"theme": c["cfg_all"]["theme"]})));
        let fonts = fonts_of(c);
        let tree = &c["tree"];
        let (pw, ph) = (tree["w"].as_i64().unwrap() as i32, tree["h"].as_i64().unwrap() as i32);
        let (ox, oy) = (tree["x"].as_i64().unwrap() as i32, tree["y"].as_i64().unwrap() as i32);
        let cm10 = tree["kids"][0]["rw"].as_i64().unwrap() as i32;
        let faces = Faces { ui: "Segoe UI".into(), title: String::new() };
        let cfg = &c["cfg_full"];
        let now = c["now"].as_f64().unwrap();
        let ex = cfg["screen_exclude"].as_array().map(|a| a.iter().filter_map(|v| v.as_str().map(String::from)).collect())
            .unwrap_or_else(|| zenko_desktop::routes17::routes_data()["SCREEN_EXCLUDE_DEFAULT"].as_array().unwrap().iter()
                .filter_map(|v| v.as_str().map(String::from)).collect());
        let d = ScreenData {
            life: c["life_all"].clone(), screen_time: cfg["screen_time"].as_bool().unwrap_or(true),
            nudges: cfg["screen_nudges"].as_bool().unwrap_or(true), exclude: ex, today: (now / 86400.0).floor() as i64,
            hour: ((now % 86400.0) / 3600.0) as i64, now, jp: c["jp"].as_str().unwrap().into(),
            serif: c["serif"].as_str().unwrap().into(),
        };
        let range = match name { "screen_week_small" => 7, "screen_month" => 30, _ => 1 };
        let mut ui = Ui::new(&th.paper);
        ui.x11 = true;
        eprintln!("case {name}");
        let mut p = ScreenPage::build(&mut ui, 0, &th, &faces, cm10, range, &d);
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
        // every canvas of the body (the chart, the list bars, the buttons' own canvases are widgets of theirs)
        let mut pairs = vec![];
        let gb = body_of(tree);
        pair(&ui, p.body, gb, &mut pairs);
        let mut n = 0;
        for (id, gn) in pairs {
            if let Some(cv) = ui.ws[id].tk_canvas() {
                check_items(&format!("{name} canvas {n}"), cv, gn["items"].as_array().unwrap(), &c["images"], &fonts);
                n += 1;
            }
        }
        assert!(n >= 2, "{name}: {n} canvases");
    }
    assert!(fails.is_empty(), "{fails:?}");
}

/// the golden page body: the scroller's inside frame's one kid
fn body_of(tree: &Value) -> &Value {
    let mut out = vec![];
    common::find(tree, &|n| n["cls"] == "Frame" && n["pack"]["padx"] == "20" , &mut out);
    out[0]
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
