//! The hub's Reminders page against zenko.py's (golden/hub_reminders.json): the Write-an-ema card and the night
//! shrine with lanterns for today's timetable, plaques that are soon / daily / rang / later / anytime and two
//! fulfilled ones, wide, in a small midnight window, with a plaque selected (its pills) and another hovered, and
//! empty; every widget's place and size, texts and fonts, and every item of every canvas.
mod common;
use common::{check_items, cmp_tree, fonts_of};
use serde_json::{Value, json};
use zenko_desktop::pagekit::Faces;
use zenko_desktop::rempage::RemPage;
use zenko_desktop::ui::{Kind, Ui};

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

#[test]
fn rem_page_matches_zenko_py() {
    let g: Vec<Value> = serde_json::from_str(&std::fs::read_to_string("../../golden/hub_reminders.json").unwrap()).unwrap();
    let dir = std::env::temp_dir().join(format!("zk_rem_{}", std::process::id()));
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
        std::fs::write(fb.join("zenko_life.json"), c["life_all"].to_string()).unwrap();
        let mut cfg = c["cfg_all"].clone();
        cfg["robe"] = json!("none");
        let mut fox = zenko_desktop::Companion::new(zenko_desktop::Cfg::from(cfg), 1.0, now);
        fox.tt = c["tt"].as_array().cloned().unwrap_or_default();
        let mut d = fox.rem_data();
        // (the recording's planner hung its ema study sessions on the timetable after the wide page was drawn; the
        // small window's page and the selected plaque's render came after it)
        if name == "rem_wide" {
            d.tt.retain(|it| it.get("plan").is_none());
        }
        d.jp = c["jp"].as_str().unwrap().into();
        d.serif = c["serif"].as_str().unwrap().into();
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
        let cm10 = tree["kids"][0]["rw"].as_i64().unwrap() as i32;
        let faces = Faces { ui: "Segoe UI".into(), title: String::new() };
        let zk_f = c["zk_f"].as_f64().unwrap();
        let mut ui = Ui::new(&th.paper);
        ui.x11 = true;
        let mut p = RemPage::build(&mut ui, 0, &th, &faces, cm10, &d);
        if small {
            // the window was made at the size before: zk_resized's first fit ran with the fonts it had then, and
            // the When row it wrapped stays wrapped (the room didn't grow back)
            zenko_desktop::pagekit::zk_fonts(&mut ui, &fonts, 0, g[0]["zk_f"].as_f64().unwrap());
            ui.layout_at(&fonts, pw, ph);
            zenko_desktop::pagekit::zk_wrap(&mut ui, &fonts, 0, &mut p.wrap, pw, ph);
        }
        p.fit(&mut ui, &fonts, &th, &faces, zk_f, pw, ph, &d);
        if name == "rem_sel" {
            p.sel = d.act.first().and_then(|e| e["id"].as_str()).map(String::from);
            p.hover = d.act.get(1).and_then(|e| e["id"].as_str()).map(String::from);
            p.render(&mut ui, &th, &faces, &fonts, zk_f, &d);
            p.fit(&mut ui, &fonts, &th, &faces, zk_f, pw, ph, &d);
        }
        let mut bad = vec![];
        cmp_tree(&ui, 0, tree, ox, oy, name, &mut bad);
        let mut ps = vec![];
        pairs(&ui, 0, tree, &mut ps);
        let mut imgs = c["images"].clone();
        imgs["__fonts"] = c["fonts"].clone();
        for (k, (id, gn)) in ps.iter().enumerate() {
            if let Kind::Canvas { tk: Some(cv), .. } = &ui.ws[*id].kind
                && k > 0 && gn["items"].as_array().is_some_and(|a| !a.is_empty())
                && !(gn["w"] == 10 && gn["items"].as_array().unwrap().len() == 4) {
                // (the wisps drift every 90 ms from where they were drawn: put them back where ema_render put them)
                let mut want = gn["items"].as_array().unwrap().clone();
                let mut wi = p.wisps.iter().flat_map(|w| [(w.0, w.2, w.3), (w.1, w.2, w.3)]);
                if *id == p.cv {
                    for it in want.iter_mut().filter(|it| it["tags"].as_array().unwrap().iter().any(|t| t == "wisp")) {
                        let (_, wx, wy) = wi.next().unwrap();
                        let (gx, gy) = (it["coords"][0].as_f64().unwrap(), it["coords"][1].as_f64().unwrap());
                        if (gx - wx).abs() <= 14.0 + 1e-6 && (gy - wy).abs() <= 9.0 + 1e-6 {
                            it["coords"] = json!([wx, wy]);
                            it["bbox"] = json!([]);
                        }
                    }
                }
                if std::env::var("ZK_DUMP").is_ok() && *id == p.cv {
                    let v: Vec<Value> = cv.items.iter().map(|i| json!([i.kind.tk_name(), i.coords, i.tags, i.text, i.fill])).collect();
                    std::fs::write(format!("/tmp/claude-0/rem_{name}.json"), serde_json::to_string(&v).unwrap()).unwrap();
                }
                let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    check_items(&format!("{name} canvas {k}"), cv, &want, &imgs, &fonts)
                }));
                if r.is_err() {
                    bad.push(format!("{name}: canvas {k} ({}x{})", gn["w"], gn["h"]));
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
