//! The hub's Home page against zenko.py's (golden/hub_home.json): every widget's place and size in five states
//! (idle with a name, focusing, paused in a small midnight window, on a break, idle in the afternoon), the texts it
//! shows, the week chart's items and the music pictures.
mod common;
use common::{check_items, cmp_tree as cmp, find, fonts_of};
use serde_json::{Value, json};
use zenko_desktop::homepage::{HomeData, HomePage};
use zenko_desktop::pagekit::Faces;
use zenko_desktop::ui::{Kind, Ui};

#[test]
fn home_page_matches_zenko_py() {
    let g: Vec<Value> = serde_json::from_str(&std::fs::read_to_string("../../golden/hub_home.json").unwrap()).unwrap();
    let dir = std::env::temp_dir().join(format!("zk_home_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("FoxBuddy")).unwrap();
    unsafe { std::env::set_var("APPDATA", &dir) };
    let mut fails = vec![];
    for c in &g {
        let name = c["name"].as_str().unwrap();
        let now = c["now"].as_f64().unwrap();
        zenko_desktop::set_fake_now(Some(now));
        for (f, k) in [("timetable.json", "tt"), ("deadlines.json", "deadlines"), ("stats.json", "stats")] {
            std::fs::write(dir.join("FoxBuddy").join(f), c[k].to_string()).unwrap();
        }
        let mut cfg = c["cfg"].clone();
        cfg["robe"] = json!("none");
        let fox = zenko_desktop::Companion::new(zenko_desktop::Cfg::from(cfg.clone()), 1.0, now);
        let mut d: HomeData = fox.home_data();
        let s = &c["session"];
        d.session = s.is_object().then(|| (s["task"].as_str().unwrap().into(), s["planned"].as_f64().unwrap(),
                                           s["elapsed"].as_f64().unwrap(), s["paused"].as_bool().unwrap(),
                                           s["dist"].as_f64().unwrap(), s.get("closed").and_then(|v| v.as_i64()).unwrap_or(0)));
        let b = &c["brk"];
        d.brk = b.is_object().then(|| (b["planned"].as_f64().unwrap(), b["elapsed"].as_f64().unwrap()));
        let th = zenko_desktop::theme::theme(&zenko_desktop::Cfg::from(json!({"theme": c["cfg_all"]["theme"]})));
        let mut fonts = fonts_of(c);
        if name == "home_paused" {
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
        let mut hp = HomePage::build(&mut ui, 0, &th, &faces, cm10, &d);
        hp.refresh(&mut ui, &th, &faces, &d, true, false);
        let zk = c["zk_f"].as_f64().unwrap();
        if name == "home_paused" {
            // the window opened at 1320x900 and was made 1120x720 straight away: the page was fitted at the big size,
            // then zk_after_map's zk_fit(fonts=False) wrapped it at the new size with the old fonts (that stacks the
            // two columns), then zk_resized's full zk_fit
            let big = &g[0];
            let (bw, bh) = (big["tree"]["w"].as_i64().unwrap() as i32, big["tree"]["h"].as_i64().unwrap() as i32);
            hp.fit(&mut ui, &fonts, &th, &faces, big["zk_f"].as_f64().unwrap(), bw, bh);
            zenko_desktop::pagekit::zk_wrap(&mut ui, &fonts, 0, &mut hp.wrap, pw, ph);
        }
        hp.fit(&mut ui, &fonts, &th, &faces, zk, pw, ph);
        hp.refresh(&mut ui, &th, &faces, &d, false, false);
        ui.layout_at(&fonts, pw, ph);
        let mut bad = vec![];
        cmp(&ui, 0, tree, ox, oy, name, &mut bad);
        // the week chart and the pictures
        let mut wk = vec![];
        find(tree, &|n| n["cls"] == "Canvas" && n["w"] == 270 && n["h"] == 130, &mut wk);
        let Kind::Canvas { tk: Some(cv), .. } = &ui.ws[hp.week].kind else { panic!() };
        check_items(&format!("{name} week"), cv, wk[0]["items"].as_array().unwrap(), &c["images"], &fonts);
        let mut pics = vec![];
        find(tree, &|n| n["cls"] == "Label" && n["opts"]["image"].as_str().is_some_and(|s| !s.is_empty()), &mut pics);
        let ours = [hp.icons.disc.clone(), hp.icons.prev.0.clone(), hp.icons.play.0.clone(), hp.icons.next.0.clone()];
        for (p, o) in pics.iter().zip(ours) {
            let rows = c["images"][p["opts"]["image"].as_str().unwrap()].as_array().unwrap();
            for (y, row) in rows.iter().enumerate() {
                for (x, px) in row.as_array().unwrap().iter().enumerate() {
                    let q = o.px[y * o.w + x];
                    let got = format!("#{:02x}{:02x}{:02x}{:02x}", q[0], q[1], q[2], q[3]);
                    if px.as_str().unwrap() != got {
                        bad.push(format!("{name} picture {} ({x},{y}): {got} want {px}", p["opts"]["image"]));
                        break;
                    }
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
