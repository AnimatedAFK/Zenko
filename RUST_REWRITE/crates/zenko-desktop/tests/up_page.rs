//! The hub's Upcoming page against zenko.py's (golden/hub_deadlines.json): exams, a planned week, free hours and the
//! IA / EE / TOK tracker in a wide window, the same in a small midnight window (two rows), another day in the
//! afternoon with a picked calendar day, and empty; every widget's place and size, texts and fonts, and every drawn
//! canvas (the clock, plaques, bars, the calendar).
mod common;
use common::{check_items, cmp_tree, fonts_of};
use serde_json::{Value, json};
use zenko_desktop::pagekit::Faces;
use zenko_desktop::ui::{Kind, Ui};
use zenko_desktop::uppage::UpPage;

/// golden canvases and our widgets in the same tree order
fn pairs<'a>(ui: &Ui, id: usize, g: &'a Value, out: &mut Vec<(usize, &'a Value)>) {
    if !g["mapped"].as_bool().unwrap() {
        return;
    }
    if g["cls"] == "Canvas" || g["cls"] == "Menubutton" {
        out.push((id, g));
    }
    let kids = g["kids"].as_array().unwrap();
    if kids.len() == ui.ws[id].kids.len() {
        for (&c, gk) in ui.ws[id].kids.iter().zip(kids) {
            pairs(ui, c, gk, out);
        }
    }
}

#[test]
fn up_page_matches_zenko_py() {
    let g: Vec<Value> = serde_json::from_str(&std::fs::read_to_string("../../golden/hub_deadlines.json").unwrap()).unwrap();
    let dir = std::env::temp_dir().join(format!("zk_up_{}", std::process::id()));
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
        let now = c["now"].as_f64().unwrap();
        zenko_desktop::set_fake_now(Some(now));
        let fb = dir.join("FoxBuddy");
        std::fs::write(fb.join("timetable.json"), c["tt"].to_string()).unwrap();
        std::fs::write(fb.join("deadlines.json"), c["deadlines"].to_string()).unwrap();
        std::fs::write(fb.join("zenko_life.json"), c["life"].to_string()).unwrap();
        let mut cfg = c["cfg_all"].clone();
        cfg["robe"] = json!("none");
        let mut fox = zenko_desktop::Companion::new(zenko_desktop::Cfg::from(cfg), 1.0, now);
        fox.tt = c["tt"].as_array().cloned().unwrap_or_default();
        let mut d = fox.up_data();
        // (the golden keeps only the Daily page's part of life: the rest is what the recording's setup made)
        // (each case's setup added the IA / EE / TOK tasks again to the same life file; the empty case added nothing)
        {
            let n = g.iter().position(|x| x["name"] == c["name"]).unwrap().min(2) + 1;
            let day = |n: i64| zenko_desktop::plan::iso(zenko_desktop::rewards::days_from_civil(2025, 12, 11) + n);
            let wk = json!([16.0, 16.5, 17.0, 17.5, 18.0, 19.0, 19.5]);
            let we = json!([10.0, 10.5, 11.0, 14.0, 14.5]);
            let three = vec![
                json!({"id": "a1", "kind": "ee", "subject": "", "unit": "words", "target": 4000, "words": 1850,
                       "title": "Extended Essay", "due": day(20), "stages": ["Topic", "Research"]}),
                json!({"id": "a2", "kind": "ia", "subject": "bio", "unit": "words", "target": 3000, "words": 0,
                       "title": "Biology HL IA", "due": day(5), "stages": []}),
                json!({"id": "a3", "kind": "tok_essay", "subject": "", "unit": "words", "target": 1600, "words": 0,
                       "title": "TOK essay", "due": "", "stages": []})];
            let tasks: Vec<Value> = (0..n).flat_map(|_| three.clone()).collect();
            d.life = json!({"free": {"0": wk, "1": wk, "2": wk, "3": wk, "4": wk, "5": we, "6": we}, "free_dates": {},
                            "free_set": true, "studied": {"d1": 95}, "plan_hours": {}, "plan_short": {}, "ema_short": {},
                            "ibtasks": tasks});
        }
        d.targets.iter_mut().for_each(|t| t.hours = t.hours.max(0.0));
        d.jp = c["jp"].as_str().unwrap().into();
        d.serif = "Liberation Serif".into();
        // _mincho_font: whatever the IA plaques were drawn with
        let fonts_all = c["fonts"].as_object().unwrap();
        d.mincho = ["IPAMincho", "IPAPMincho", "Noto Serif CJK JP"].iter()
            .find(|f| fonts_all.keys().any(|k| k.contains(*f))).map(|s| s.to_string()).unwrap_or_else(|| d.jp.clone());
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
        let geo = c["geometry"].as_str().unwrap();
        let win_w: i32 = geo.split('x').next().unwrap().parse().unwrap();
        let cm10 = tree["kids"][0]["rw"].as_i64().unwrap() as i32;
        let faces = Faces { ui: "Segoe UI".into(), title: String::new() };
        let mut ui = Ui::new(&th.paper);
        ui.x11 = true;
        let wide = pw >= 1000;
        let s = if wide { if win_w >= 1180 { 340 } else { 290 } } else { 300 };
        let mut up = UpPage::build(&mut ui, 0, &th, &faces, cm10, &d, wide, s);
        up.set_fox(icon.clone());
        if name == "up_pm_day" {
            up.day = Some(5);
            up.pm = true;
            up.dl_day = Some(zenko_desktop::rewards::days_from_civil(2025, 12, 14));
        }
        up.render(&mut ui, &th, &faces, cm10, &d, wide, s);
        if small && std::env::var("ZK_PRE").is_ok() {
            zenko_desktop::pagekit::zk_fonts(&mut ui, &fonts, 0, g[0]["zk_f"].as_f64().unwrap());
            ui.layout_at(&fonts, pw, ph);
            up.draw_bars(&mut ui, &th);
            zenko_desktop::pagekit::zk_wrap(&mut ui, &fonts, 0, &mut up.wrap, pw, ph);
        }
        up.fit(&mut ui, &fonts, &th, &faces, c["zk_f"].as_f64().unwrap(), pw, ph);
        let mut bad = vec![];
        cmp_tree(&ui, 0, tree, ox, oy, name, &mut bad);
        let mut ps = vec![];
        pairs(&ui, 0, tree, &mut ps);
        for (k, (id, gn)) in ps.iter().enumerate() {
            match &ui.ws[*id].kind {
                Kind::Canvas { tk: Some(cv), .. } if k > 0 && gn["items"].as_array().is_some_and(|a| !a.is_empty())
                    && !(gn["w"] == 10 && gn["items"].as_array().unwrap().len() == 4) => {
                    let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                        check_items(&format!("{name} canvas {k}"), cv, gn["items"].as_array().unwrap(), &c["images"], &fonts)
                    }));
                    if r.is_err() {
                        bad.push(format!("{name}: canvas {k} ({}x{})", gn["w"], gn["h"]));
                    }
                }
                Kind::Menu { text, .. } => {
                    if gn["opts"]["text"].as_str() != Some(text.as_str()) {
                        bad.push(format!("{name}: menu {text:?} want {}", gn["opts"]["text"]));
                    }
                }
                _ => {}
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
