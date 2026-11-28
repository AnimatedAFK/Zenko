//! The hub's World map against zenko.py's (golden/hub_map.json): Japan with a hover tip and a quiz to answer, Kyoto
//! in winter in a small window, and Korea picked while still locked; every widget's place, size, text and font, and
//! every canvas's items after one animation frame (the painting, plaque, road, torii and pins, names, Zenko, ring,
//! clouds, tip; the passport stamps and the journey bar).
mod common;
use common::{check_items, cmp_tree, fonts_of};

use serde_json::Value;
use zenko_desktop::mappage::{MapData, MapPage};
use zenko_desktop::pagekit::Faces;
use zenko_desktop::ui::Ui;

#[test]
fn map_page_matches_zenko_py() {
    let g: Vec<Value> = serde_json::from_str(&std::fs::read_to_string("../../golden/hub_map.json").unwrap()).unwrap();
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
        let mut pal = zenko_art::palette::Palette::empty();
        for (k, v) in c["pal"].as_object().unwrap() {
            if let Some(h) = v.as_str() && k.len() == 1 && h.starts_with('#') {
                pal.set(k.as_bytes()[0], h);
            }
        }
        let quiz: Vec<String> = c["life_all"]["journey"]["quiz"].as_array().unwrap().iter().map(|v| v.as_str().unwrap().into()).collect();
        let d = MapData {
            hours: c["journey"][0].as_f64().unwrap(), quiz, pal, tails: c["tails"].as_i64().unwrap() as i32,
            hd: c["hd"].as_bool().unwrap(), jp: c["mincho"].as_str().unwrap().into(), serif: c["serif2"].as_str().unwrap().into(),
            season: c["season"].as_str().unwrap().into(),
        };
        let mut ui = Ui::new(&th.paper);
        ui.x11 = true;
        let mut p = MapPage::build(&mut ui, 0, &th, &faces, cm10, &d);
        let zk = c["zk_f"].as_f64().unwrap();
        p.fit(&mut ui, &Lenient(&fonts), &th, &faces, zk, pw, ph);
        if name == "map_korea" {
            p.set_region(&mut ui, "korea");
        }
        // the final frame: the panel made again, the painting redrawn and one 60 ms frame at the fixed clock, the panel
        // again (as map_draw's after(50) does), the hover tip, then zk_fit
        p.panel(&mut ui);
        p.relayout(&mut ui, &Lenient(&fonts), &th, &faces, zk, pw, ph);
        p.forget();
        p.draw(&mut ui, &fonts);
        p.anim(&mut ui, 5000.0);
        p.panel(&mut ui);
        p.relayout(&mut ui, &fonts, &th, &faces, zk, pw, ph);
        if name == "map_japan" {
            p.hover_at(&mut ui, &fonts, Some("nara"));
        }
        p.fit(&mut ui, &fonts, &th, &faces, zk, pw, ph);
        assert_eq!(p.sel, c["sel"].as_str().unwrap(), "{name} sel");
        assert_eq!(p.side_w as i64, c["side_w"].as_i64().unwrap(), "{name} side width");
        assert_eq!(p.z, c["z"].as_i64().unwrap(), "{name} z");
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
        common::find(tree, &|n| n["cls"] == "Frame" && n["pack"]["padx"] == "20", &mut out);
        let mut pairs = vec![];
        pair(&ui, p.body, out[0], &mut pairs);
        let mut imgs = c["images"].clone();
        common::expand_images(&mut imgs);
        let mut n = 0;
        for (id, gn) in pairs {
            if let Some(cv) = ui.ws[id].tk_canvas() {
                let items: Vec<Value> = gn["items"].as_array().unwrap().iter().filter(|i| i["type"] != "window").cloned().collect();
                check_items(&format!("{name} canvas {n}"), cv, &items, &imgs, &fonts);
                n += 1;
            }
        }
        assert!(n >= 3, "{name}: {n} canvases");
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

struct Lenient<'a>(&'a common::Table);
impl zenko_desktop::ui::Fonts for Lenient<'_> {
    fn width(&self, f: &zenko_desktop::ui::Font, s: &str) -> i32 {
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| self.0.width(f, s))).unwrap_or(7 * s.chars().count() as i32)
    }
    fn metrics(&self, f: &zenko_desktop::ui::Font) -> (i32, i32) {
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| self.0.metrics(f))).unwrap_or((14, 4))
    }
}
