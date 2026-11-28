//! The hub's Daily page against zenko.py's (golden/hub_daily.json): with cards, badges and a snowy day, the same in a
//! small midnight window, and empty; every widget's place and size, texts and fonts, and the mood and badge canvases.
mod common;
use common::{check_items, cmp_tree, find, fonts_of};
use serde_json::{Value, json};
use zenko_desktop::dailypage::{DailyData, DailyPage};
use zenko_desktop::pagekit::Faces;
use zenko_desktop::ui::{Kind, Ui};

#[test]
fn daily_page_matches_zenko_py() {
    let g: Vec<Value> = serde_json::from_str(&std::fs::read_to_string("../../golden/hub_daily.json").unwrap()).unwrap();
    let mut fails = vec![];
    for c in &g {
        let name = c["name"].as_str().unwrap();
        let now = c["now"].as_f64().unwrap();
        zenko_desktop::set_fake_now(Some(now));
        let day = (now.floor() as i64 + zenko_desktop::local_offset_pub()).div_euclid(86400);
        let today = zenko_desktop::plan::iso(day);
        let cards = c["life"]["cards"].as_array().cloned().unwrap_or_default();
        let due = cards.iter().filter(|k| k["due"].as_str().unwrap_or("") <= today.as_str()).count();
        let d = DailyData {
            day, streak: c["streak"].as_i64().unwrap(), total_hours: c["total_hours"].as_f64().unwrap(),
            mood_word: c["mood_word"].as_str().unwrap().into(),
            fox_name: c["cfg"]["fox_name"].as_str().unwrap_or("Zenko").into(), spirit: c["spirit"].as_i64().unwrap(),
            hunger: c["hunger"].as_f64().unwrap(), life: c["life"].clone(), cfg: c["cfg_all"].clone(),
            well: c["well"].clone(), jp: c["jp"].as_str().unwrap().into(), due,
        };
        let th = zenko_desktop::theme::theme(&zenko_desktop::Cfg::from(json!({"theme": c["cfg_all"]["theme"]})));
        let mut fonts = fonts_of(c);
        let small = name == "daily_midnight_small";
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
        let mut ui = Ui::new(&th.paper);
        ui.x11 = true;
        let mut dp = DailyPage::build(&mut ui, 0, &th, &faces, cm10, &d);
        assert_eq!(zenko_desktop::palette::card_titles(&ui, 0), common::palette_cards("Daily"), "{name}: Ctrl+K cards");
        let zk = c["zk_f"].as_f64().unwrap();
        if small {
            // zenko.py: the window opened at 1320x900, where the first zk_fits scaled the fonts (zk_f for that size) but
            // didn't wrap yet; it was made 1120x720 before the first zk_wrap (the lambda after zk_after_map), so that
            // wrap ran at the small size with the big fonts; then zk_resized's full zk_fit
            zenko_desktop::pagekit::zk_fonts(&mut ui, &fonts, 0, g[0]["zk_f"].as_f64().unwrap());
            dp.draw(&mut ui, &fonts, &th, &faces, &d, pw, ph);
            zenko_desktop::pagekit::zk_wrap(&mut ui, &fonts, 0, &mut dp.wrap, pw, ph);
        }
        dp.fit(&mut ui, &fonts, &th, &faces, &d, zk, pw, ph);
        let mut bad = vec![];
        cmp_tree(&ui, 0, tree, ox, oy, name, &mut bad);
        for b in &bad {
            eprintln!("{b}");
        }
        if !bad.is_empty() {
            fails.push(format!("{name}: {}", bad.len()));
        }
        // the mood row and the badge shelf
        for (id, what) in [(dp.moods_cv, "moods"), (dp.badges_cv, "badges")] {
            let mut cvs = vec![];
            let jp20 = |n: &Value| n["items"].as_array().is_some_and(|i| i.iter().any(|t| t["opts"]["font"].as_str()
                .is_some_and(|f| f.contains(" 20 bold") && !f.contains("Segoe"))));
            if what == "moods" {
                find(tree, &|n| n["cls"] == "Canvas" && n["items"].as_array().is_some_and(|i| i.len() == 28), &mut cvs);
            } else {
                find(tree, &|n| n["cls"] == "Canvas" && jp20(n), &mut cvs);
            }
            if cvs.is_empty() {
                continue;
            }
            let Kind::Canvas { tk: Some(cv), .. } = &ui.ws[id].kind else { panic!("{what}") };
            let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                check_items(&format!("{name} {what}"), cv, cvs[0]["items"].as_array().unwrap(), &c["images"], &fonts)
            }));
            if r.is_err() {
                fails.push(format!("{name}: {what} canvas"));
            }
        }
    }
    assert!(fails.is_empty(), "{fails:?}");
}
