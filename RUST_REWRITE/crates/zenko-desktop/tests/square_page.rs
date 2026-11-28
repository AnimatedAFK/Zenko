//! The hub's Town square against zenko.py's (golden/hub_square.json): offline on a winter night, live on Christmas Eve
//! at dusk with five messages in a small window, and on Holi morning. Every widget's place, size, text and font, and
//! every canvas's items after the street is built and stepped once with its random stream seeded (the street, its
//! lanterns, the banner, the crowd and named people, the speech bubble; the status dot; the scrollbar).
mod common;
use common::{check_items, cmp_tree, fonts_of};

use serde_json::Value;
use zenko_desktop::pagekit::Faces;
use zenko_desktop::squarepage::{Msg, SquareData, SquarePage};
use zenko_desktop::ui::Ui;

#[test]
fn square_page_matches_zenko_py() {
    let g: Vec<Value> = serde_json::from_str(&std::fs::read_to_string("../../golden/hub_square.json").unwrap()).unwrap();
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
        let on = name != "square_off";
        let msgs = c["msgs"].as_array().unwrap().iter().map(|m| Msg {
            id: m["id"].as_str().unwrap().into(), uid: m["uid"].as_str().unwrap().into(), name: m["name"].as_str().unwrap().into(),
            text: m["text"].as_str().unwrap().into(), ts: m["ts"].as_f64().unwrap(),
            look: m.get("look").map(|a| (a["y"].as_i64().unwrap(), a["m"].as_bool().unwrap(), a["h"].as_i64().unwrap())),
        }).collect();
        let secs = now.floor() as i64;
        let d = SquareData {
            on, state: if on { "live".into() } else { "off".into() }, uid: c["uid"].as_str().unwrap().into(), name: "Kitsune".into(),
            msgs, changed: if name == "square_live" { 5 } else { 0 }, yukata: 4, mask: name == "square_holi", now, offset: 0,
            day: secs.div_euclid(86400), hour: (secs.rem_euclid(86400) / 3600) as u32, preview: None,
            jp: c["jp2"].as_str().unwrap().into(),
        };
        assert_eq!(d.day + 719163, c["seed"].as_i64().unwrap(), "{name} day");
        let st_h = c["st_h"].as_f64().unwrap();
        let mut ui = Ui::new(&th.paper);
        ui.x11 = true;
        let mut p = SquarePage::build(&mut ui, 0, &th, &faces, cm10, &d, (st_h / 0.4).ceil() as i32, 7);
        let zk = c["zk_f"].as_f64().unwrap();
        // hub_build_square paints at once, before the page is mapped (so `see("end")` leaves the board at the top)
        p.paint(&mut ui, &Lenient(&fonts));
        p.fit(&mut ui, &Lenient(&fonts), &th, &faces, zk, pw, ph);
        // _final: the street made again with its stream seeded and stepped once, the board painted afresh, then zk_fit
        {
            let s = p.street.as_mut().unwrap();
            s.reset(7);
            s.build(&mut ui, &fonts, &d);
            s.step(&mut ui, &fonts, &d);
        }
        // shown = -1 (the 800 ms paint that would show it again is stopped)
        p.shown = -1;
        p.fit(&mut ui, &fonts, &th, &faces, zk, pw, ph);
        let s = p.street.as_ref().unwrap();
        let gs = &c["street"];
        assert_eq!((s.aw, s.ah, s.ev.as_str(), s.mode.as_str()),
                   (gs[0].as_i64().unwrap(), gs[1].as_i64().unwrap(), gs[2].as_str().unwrap(), gs[3].as_str().unwrap()), "{name} street");
        let gp = c["people"].as_array().unwrap();
        assert_eq!(s.people.len(), gp.len(), "{name} people");
        for (a, b) in s.people.iter().zip(gp) {
            assert!((a.x - b["x"].as_f64().unwrap()).abs() < 1e-9 && a.lane as i64 == b["lane"].as_i64().unwrap()
                    && a.dir == b["dir"].as_i64().unwrap() && (a.y - b["y"].as_f64().unwrap()).abs() < 1e-9
                    && a.stop == b["stop"].as_i64().unwrap() && a.name.as_deref() == b["name"].as_str(),
                    "{name} person {a:?} vs {b}");
        }
        let mut bad = vec![];
        cmp_tree(&ui, 0, tree, ox, oy, name, &mut bad);
        for b in bad.iter().take(30) {
            eprintln!("{b}");
        }
        if !bad.is_empty() {
            fails.push(format!("{name}: {}", bad.len()));
            continue;
        }
        let mut pairs = vec![];
        pair(&ui, 0, tree, &mut pairs);
        let mut imgs = c["images"].clone();
        common::expand_images(&mut imgs);
        let mut n = 0;
        // a speech bubble's tag carries its person's canvas id, which counts every item the canvas ever had (the street
        // was built more often in the app before the recording): both sides' bubN tags are compared as bub#
        let norm = |t: &str| if t.starts_with("bub") && t.len() > 3 && t[3..].bytes().all(|b| b.is_ascii_digit()) { "bub#".to_string() } else { t.to_string() };
        for &(id, _) in &pairs {
            if let Some(cv) = ui.ws[id].tk_canvas_mut() {
                for it in cv.items.iter_mut() {
                    it.tags = it.tags.iter().map(|t| norm(t)).collect();
                }
            }
        }
        for (id, gn) in pairs {
            if let Some(cv) = ui.ws[id].tk_canvas() {
                let mut items: Vec<Value> = gn["items"].as_array().unwrap().iter().filter(|i| i["type"] != "window").cloned().collect();
                for it in items.iter_mut() {
                    if let Some(a) = it["tags"].as_array_mut() {
                        for t in a.iter_mut() {
                            *t = Value::from(norm(t.as_str().unwrap_or("")));
                        }
                    }
                }
                check_items(&format!("{name} canvas {n}"), cv, &items, &imgs, &fonts);
                n += 1;
            }
        }
        assert!(n >= 3, "{name}: {n} canvases");
    }
    assert!(fails.is_empty(), "{fails:?}");
}

fn pair<'a>(ui: &Ui, id: usize, g: &'a Value, out: &mut Vec<(usize, &'a Value)>) {
    if g["cls"] == "Canvas" && g["items"].is_array() && !g["items"].as_array().unwrap().is_empty() && ui.ws[id].tk_canvas().is_some() {
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
