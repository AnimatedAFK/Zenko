//! The hub's Settings page against zenko.py's (golden/hub_settings.json): wide in snow, small at midnight, and a
//! config with most things changed (tails, scarf, size, units, toggles, home city, text size); every widget's place
//! and size, texts and fonts, and every item of every canvas (the nav chips, the swatches, the scarves, the buttons).
mod common;
use common::{check_items, cmp_tree, fonts_of};
use serde_json::Value;
use zenko_desktop::pagekit::Faces;
use zenko_desktop::setpage::{SetData, SetPage};
use zenko_desktop::ui::{Item, Kind, Ui};

fn pairs<'a>(ui: &Ui, id: usize, g: &'a Value, out: &mut Vec<(usize, &'a Value)>) {
    if !g["mapped"].as_bool().unwrap() {
        return;
    }
    if g["cls"] == "Canvas" {
        out.push((id, g));
    }
    let kids = g["kids"].as_array().unwrap();
    if kids.len() == ui.ws[id].kids.len() {
        for (&c, gk) in ui.ws[id].kids.iter().zip(kids) {
            pairs(ui, c, gk, out);
        }
    }
}

fn walk<'a>(g: &'a Value, out: &mut Vec<&'a Value>) {
    out.push(g);
    for k in g["kids"].as_array().into_iter().flatten() {
        walk(k, out);
    }
}

/// the label text the app worked out, as zenko.py showed it (the first label that starts so)
fn said(all: &[&Value], pre: &[&str]) -> String {
    all.iter().filter_map(|n| n["opts"]["text"].as_str())
        .find(|t| pre.iter().any(|p| t.starts_with(p))).unwrap_or("").to_string()
}

/// a round_frame card or a PixelButton, item by item (type, coords, fill, outline, text)
fn simple_items(ui: &Ui, th: &zenko_desktop::theme::Theme, id: usize, want: &[Value], at: &str) -> Result<(), String> {
    let got = ui.items_of(id, th);
    if got.len() != want.len() {
        return Err(format!("{at}: {} items, want {}", got.len(), want.len()));
    }
    let flat = |p: &[(i32, i32)]| p.iter().flat_map(|&(x, y)| [x as i64, y as i64]).collect::<Vec<_>>();
    for (k, (g, e)) in got.iter().zip(want).enumerate() {
        let (c, ty, fill, outline, text) = match g {
            Item::Poly { pts, fill, outline } => (flat(pts), "polygon", fill.clone().unwrap_or_default(),
                                                  outline.clone().unwrap_or_default(), None),
            Item::Line { pts, color } => (flat(pts), "line", color.clone(), String::new(), None),
            Item::Oval { x1, y1, x2, y2, fill } => (vec![*x1 as i64, *y1 as i64, *x2 as i64, *y2 as i64], "oval", fill.clone(),
                                                    String::new(), None),
            Item::Text { x, y, text, color, .. } => (vec![*x as i64, *y as i64], "text", color.clone(), String::new(),
                                                     Some(text.clone())),
        };
        let ec: Vec<i64> = e["coords"].as_array().unwrap().iter().map(|v| v.as_f64().unwrap().round() as i64).collect();
        let o = &e["opts"];
        if ty != e["type"] || c != ec || fill != o["fill"].as_str().unwrap_or("")
            || (ty == "polygon" && outline != o["outline"].as_str().unwrap_or(""))
            || text.is_some_and(|t| t != o["text"].as_str().unwrap_or("")) {
            return Err(format!("{at} item {k}: {ty} {c:?} {fill} {outline} want {} {ec:?} {}", e["type"], o["fill"]));
        }
    }
    Ok(())
}

#[test]
fn set_page_matches_zenko_py() {
    let g: Vec<Value> = serde_json::from_str(&std::fs::read_to_string("../../golden/hub_settings.json").unwrap()).unwrap();
    let only = std::env::var("ZK_CASE").unwrap_or_default();
    let mut fails = vec![];
    for c in &g {
        let name = c["name"].as_str().unwrap();
        if !only.is_empty() && only != name {
            continue;
        }
        let tree = &c["tree"];
        let mut all = vec![];
        walk(tree, &mut all);
        let try_lbl = all.iter().find(|n| n["opts"]["text"].as_str().is_some_and(|t| t.starts_with("\u{2713} allowed")
            || t.starts_with("\u{2715} closed") || t.starts_with("\u{2713} kept")));
        let mut d = SetData {
            cfg: c["cfg_full"].clone(),
            is_win: false,
            autostart: false,
            font_ui: c["font_ui"].as_str().unwrap().into(),
            spotify: false,
            lockin: false,
            zb_linked: false,
            whisper: said(&all, &["Whisper isn't", "Whisper is "]),
            stt: said(&all, &["Not set up yet", "Ready"]),
            version: said(&all, &["You have v"]),
            live: None,
            install: false,
            frozen: false,
            speed: said(&all, &["This window opened"]),
            exam_iso: c["exam_iso"].as_str().unwrap().into(),
            update_source: String::new(),
            rule_try: try_lbl.map(|n| (n["opts"]["text"].as_str().unwrap().to_string(),
                                       n["opts"]["fg"].as_str().or(n["opts"]["foreground"].as_str()).unwrap_or("").to_string()))
                .unwrap_or_default(),
        };
        let th = zenko_desktop::theme::theme(&zenko_desktop::Cfg::from(c["cfg_full"].clone()));
        if d.rule_try.1.is_empty() {
            d.rule_try.1 = th.grey.clone();
        }
        let fonts = fonts_of(c);
        let (pw, ph) = (tree["w"].as_i64().unwrap() as i32, tree["h"].as_i64().unwrap() as i32);
        let (ox, oy) = (tree["x"].as_i64().unwrap() as i32, tree["y"].as_i64().unwrap() as i32);
        let cm10 = tree["kids"][0]["rw"].as_i64().unwrap() as i32;
        let faces = Faces { ui: d.font_ui.clone(), title: String::new() };
        let zk_f = c["zk_f"].as_f64().unwrap();
        let mut ui = Ui::new(&th.paper);
        ui.x11 = true;
        let mut p = SetPage::build(&mut ui, 0, &th, &faces, cm10, &d);
        p.fit(&mut ui, &fonts, &th, &faces, zk_f, pw, ph);
        let mut bad = vec![];
        cmp_tree(&ui, 0, tree, ox, oy, name, &mut bad);
        let mut ps = vec![];
        pairs(&ui, 0, tree, &mut ps);
        let mut imgs = c["images"].clone();
        imgs["__fonts"] = c["fonts"].clone();
        // (all but the scroller's own canvas, which holds only the page, and its bar)
        let mut seen = 0;
        for (k, (id, gn)) in ps.iter().enumerate() {
            if matches!(ui.ws[*id].kind, Kind::Button { .. } | Kind::Canvas { card: Some(_), .. }) {
                seen += 1;
                if let Err(e) = simple_items(&ui, &th, *id, gn["items"].as_array().unwrap(), &format!("{name} canvas {k}")) {
                    bad.push(e);
                }
                continue;
            }
            if let Kind::Canvas { tk: Some(cv), .. } = &ui.ws[*id].kind
                && gn["items"].as_array().is_some_and(|a| !a.is_empty())
                && !(gn["w"] == 10 && gn["items"].as_array().unwrap().len() == 4) {
                let want = gn["items"].as_array().unwrap().clone();
                seen += 1;
                let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    check_items(&format!("{name} canvas {k}"), cv, &want, &imgs, &fonts)
                }));
                if r.is_err() {
                    bad.push(format!("{name}: canvas {k} ({}x{} at {},{})", gn["w"], gn["h"], gn["x"], gn["y"]));
                }
            }
        }
        eprintln!("{name}: {seen} canvases checked");
        assert!(seen >= 78, "{name}: only {seen} canvases were compared");
        for b in &bad {
            eprintln!("{b}");
        }
        if !bad.is_empty() {
            fails.push(format!("{name}: {}", bad.len()));
        }
    }
    assert!(fails.is_empty(), "{fails:?}");
}
