//! The hub's Zen garden against zenko.py's (golden/hub_garden.json): a bonsai at night with your pond fish, a seed in a
//! small midnight window, full bloom while box breathing, and the sacred tree with decor and a line in the water; every
//! widget's place and size, texts and fonts, and every canvas's items after one animation frame (the painting with
//! Zenko's shadow and grass, lanterns, fireflies, decor, koi, the breathing guide, the rod and bobber).
mod common;
use common::{check_items, cmp_tree, fonts_of};

use serde_json::Value;
use zenko_desktop::gardenpage::{Fish, GardenData, GardenPage, breath_new};
use zenko_desktop::pagekit::Faces;
use zenko_desktop::ui::Ui;

#[test]
fn garden_page_matches_zenko_py() {
    let g: Vec<Value> = serde_json::from_str(&std::fs::read_to_string("../../golden/hub_garden.json").unwrap()).unwrap();
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
        let secs = now as i64;
        let today = secs.div_euclid(86400);
        let iso = zenko_desktop::plan::iso(today);
        let mut pal = zenko_art::palette::Palette::empty();
        for (k, v) in c["pal"].as_object().unwrap() {
            if let Some(h) = v.as_str() && k.len() == 1 && h.starts_with('#') {
                pal.set(k.as_bytes()[0], h);
            }
        }
        let jp = c["jp"].as_str().unwrap().to_string();
        let got: Vec<String> = c["journey"][1].as_array().unwrap().iter().map(|p| p[0].as_str().unwrap().to_string()).collect();
        let d = GardenData {
            garden: c["life_all"]["garden"].clone(), fishing: c["life_all"]["fishing"].clone(), cards: vec![],
            cfg: c["cfg_full"].clone(), lockin: false, today, hour: (secs.rem_euclid(86400) / 3600) as u32,
            minute: (secs.rem_euclid(3600) / 60) as u32, month: iso[5..7].parse().unwrap(), sky: true, seasons: true,
            weather: None, wx_code: None, hours: c["journey"][0].as_f64().unwrap(), got, pal,
            look: zenko_art::sit::Look::default(), tails: c["tails"].as_i64().unwrap() as i32, hd: c["hd"].as_bool().unwrap(),
            jp: jp.clone(), jpg: jp, serif: "Liberation Serif".into(), ground_i: c["ground_i"].as_i64().unwrap(),
        };
        let mut ui = Ui::new(&th.paper);
        ui.x11 = true;
        let mut p = GardenPage::build(&mut ui, 0, &th, &faces, cm10, &d);
        let zk = c["zk_f"].as_f64().unwrap();
        // (the first layout holds widgets the recording never measured, so unknown letters count 7 px here)
        p.fit(&mut ui, &Lenient(&fonts), &th, &faces, zk, pw, ph);
        // the final frame: (breathing / a line in the water), the panel made again, one frame at the fixed clock
        let t = 5000.0;
        if name == "garden_bloom_breath" {
            let mut b = breath_new("box", None, 5.0, t);
            b.t0 = t - 5.5;
            let gb = &c["breath"];
            assert_eq!(b.rounds, gb["rounds"].as_i64().unwrap());
            assert_eq!(b.total, gb["total"].as_f64().unwrap());
            p.breath = Some(b);
        }
        if name == "garden_sacred_fish" {
            p.fish = Some(Fish { phase: "wait".into(), t: t - 1.0, bx: 0.2, by: -0.1, depth: 3, gold: true, q: Value::Null,
                                 bite_at: 1e9, items: None, ripples: vec![], jump: None });
        }
        p.panel(&mut ui, &d);
        p.fit(&mut ui, &fonts, &th, &faces, zk, pw, ph);
        p.redraw(&mut ui);
        let acts = p.frame(&mut ui, t, &mut || 0.99);
        eprintln!("acts {acts:?}");
        assert_eq!(p.side_w as i64, c["side_w"].as_i64().unwrap(), "{name} side width");
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
                check_items(&format!("{name} canvas {n}"), cv, gn["items"].as_array().unwrap(), &imgs, &fonts);
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
