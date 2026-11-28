//! The whole "Ask Zenko" page against zenko.py's (golden/hub_pages.json): every widget's place and size, the chat text's
//! display lines and the scrollbar's thumb, built and filled in the order zenko.py does it.
mod common;
use common::{check_items, table};
use serde_json::Value;
use zenko_desktop::chatpage::ChatPage;
use zenko_desktop::ui::{Kind, Ui};

fn cmp(ui: &Ui, id: usize, g: &Value, ox: i32, oy: i32, at: &str, bad: &mut Vec<String>) {
    let w = &ui.ws[id];
    let (ax, ay) = ui.abs(id);
    let got = (ax, ay, w.w, w.h, w.mapped);
    let gi = |k: &str| g[k].as_i64().unwrap() as i32;
    let want = (gi("x") - ox, gi("y") - oy, gi("w"), gi("h"), g["mapped"].as_bool().unwrap());
    let (got, want) = if !want.4 { ((0, 0, 0, 0, got.4), (0, 0, 0, 0, false)) } else { (got, want) };
    if got != want {
        bad.push(format!("{at} {}: {got:?} want {want:?}", g["cls"]));
    }
    let kids = g["kids"].as_array().unwrap();
    if kids.len() != w.kids.len() {
        bad.push(format!("{at}: {} kids, want {}", w.kids.len(), kids.len()));
        return;
    }
    for (k, (&c, gk)) in w.kids.iter().zip(kids).enumerate() {
        cmp(ui, c, gk, ox, oy, &format!("{at}/{k}"), bad);
    }
}

#[test]
fn chat_page_matches_zenko_py() {
    let g: Value = serde_json::from_str(include_str!("../../../golden/hub_pages.json")).unwrap();
    for c in g.as_array().unwrap() {
        let name = c["name"].as_str().unwrap();
        let th = zenko_desktop::theme::theme(&zenko_desktop::Cfg::from(serde_json::json!({"theme": c["cfg_all"]["theme"]})));
        let mut fonts = table(c);
        // the named fonts of the buttons are known by what Tk made of "Segoe UI" here
        let alias: Vec<_> = fonts.0.iter().filter(|(k, _)| k.0 == "Inter").map(|(k, v)| (k.clone(), v.clone())).collect();
        for (k, v) in alias {
            let e = fonts.0.entry(("Segoe UI".into(), k.1, k.2, k.3)).or_insert_with(|| v.clone());
            for (s, n) in v.2 {
                e.2.entry(s).or_insert(n);
            }
        }
        let tree = &c["tree"];
        let (pw, ph) = (tree["w"].as_i64().unwrap() as i32, tree["h"].as_i64().unwrap() as i32);
        let (ox, oy) = (tree["x"].as_i64().unwrap() as i32, tree["y"].as_i64().unwrap() as i32);
        let mut ui = Ui::new(&th.paper);
        let cm10 = tree["kids"][0]["rw"].as_i64().unwrap() as i32;
        let mut cp = ChatPage::build(&mut ui, 0, &th, "Segoe UI", false, cm10);
        let log = c["chat_log"].as_array().unwrap();
        for (k, e) in log.iter().enumerate().filter(|_| name != "chat_midnight_small") {
            let (who, text) = (e[0].as_str().unwrap(), e[1].as_str().unwrap());
            if name == "chat_typed" && k + 1 == log.len() {
                continue; // (typed below, once the page is up)
            }
            cp.insert(&mut ui, &fonts, &th, "Segoe UI", "Liberation Serif", "Courier", "Zenko", who == "You", text);
        }
        let zk = c["zk_f"].as_f64().unwrap();
        ui.layout_at(&fonts, pw, ph);
        cp.fit(&mut ui, &fonts, zk);
        ui.layout_at(&fonts, pw, ph);
        if cp.draw_title(&mut ui, &fonts, &th, "Segoe UI", "", zk) {
            ui.layout_at(&fonts, pw, ph);
        }
        if name == "chat_typed" {
            // her reply to a page that's open: _chat_pump types it out chunk by chunk, `see end` after each
            let text = log.last().unwrap()[1].as_str().unwrap();
            for (s, tg) in cp.begin_typed(&mut ui, &th, "Segoe UI", "Liberation Serif", "Courier", "Zenko", text) {
                cp.type_chunk(&mut ui, &fonts, &s, &tg);
            }
            cp.type_chunk(&mut ui, &fonts, "\n\n", &["fox".to_string()]);
        }
        cp.sync_scroll(&mut ui, &fonts, &th);
        let mut bad = vec![];
        cmp(&ui, 0, tree, ox, oy, name, &mut bad);
        for b in &bad {
            eprintln!("{b}");
        }
        assert!(bad.is_empty(), "{name}: {} differences", bad.len());
        if name != "chat_midnight_small" {
            // (that one is still typing its greeting in the golden run)
            let tx = &c["texts"][0];
            let Kind::Text { t, .. } = &ui.ws[cp.text].kind.clone() else { panic!() };
            // zenko.py's very first layout of the page is a transient one (the status line and entry row are not
            // placed yet) that can leave a restored long chat scrolled 1 pixel differently; the lines must agree
            // otherwise
            let lines = tx["lines"].as_array().unwrap();
            let mut shift = None;
            for (k, l) in lines.iter().enumerate() {
                let (a, b) = l["index"].as_str().unwrap().split_once('.').unwrap();
                let i = (a.parse().unwrap(), b.parse().unwrap());
                let bx: Vec<i32> = l["box"].as_array().unwrap().iter().map(|v| v.as_i64().unwrap() as i32).collect();
                let got = t.dlineinfo(&fonts, i).unwrap_or_else(|| panic!("{name} line {i:?} not shown"));
                let dy = got[1] - bx[1];
                assert!(dy.abs() <= 1 && *shift.get_or_insert(dy) == dy, "{name} line {i:?}: {got:?} vs {bx:?}");
                let last = k + 1 == lines.len();
                assert_eq!((got[0], got[2], got[4]), (bx[0], bx[2], bx[4]), "{name} line {i:?}");
                assert!(got[3] == bx[3] || last && (got[3] - bx[3]).abs() <= 1, "{name} line {i:?}: {got:?} vs {bx:?}");
            }
            // the wheel and the scrollbar, from zenko.py's own starting view
            if let Some(sc) = tx.get("scrolls").and_then(|v| v.as_array()) {
                let l0 = &lines[0];
                let (a, b) = l0["index"].as_str().unwrap().split_once('.').unwrap();
                let y0 = l0["box"][1].as_i64().unwrap() as i32;
                let mut t2 = t.clone();
                t2.top = (a.parse().unwrap(), b.parse().unwrap());
                t2.top_offset = (14 - y0).max(0);
                t2.update(&fonts);
                assert!((t2.yview(&fonts).0 - tx["yview"][0].as_f64().unwrap()).abs() < 1e-9, "{name}: start view");
                for st in sc {
                    let (how, v) = (st["op"][0].as_str().unwrap(), st["op"][1].as_f64().unwrap());
                    match how {
                        "units" => t2.scroll_units(&fonts, v as i32),
                        "pages" => t2.scroll_pages(&fonts, v as i32),
                        _ => t2.moveto(&fonts, v),
                    }
                    let yv = t2.yview(&fonts);
                    let (a, b) = st["top"].as_str().unwrap().split_once('.').unwrap();
                    let top: (usize, usize) = (a.parse().unwrap(), b.parse().unwrap());
                    let want_box: Vec<i32> = st["top_box"].as_array().unwrap().iter().map(|v| v.as_i64().unwrap() as i32).collect();
                    let got_box = t2.dlineinfo(&fonts, top).map(|b| b.to_vec()).unwrap_or_default();
                    let wy = (st["yview"][0].as_f64().unwrap(), st["yview"][1].as_f64().unwrap());
                    assert!((yv.0 - wy.0).abs() < 1e-9 && (yv.1 - wy.1).abs() < 1e-9 && got_box == want_box,
                            "{name} {how} {v}: yview {yv:?} top {:?}+{} box {got_box:?}, want {wy:?} {top:?} {want_box:?}",
                            t2.top, t2.top_offset);
                }
            }
            // the thumb for zenko.py's own yview (see above for why ours may be a pixel apart)
            let yv = &tx["yview"];
            zenko_desktop::chatpage::scrollbar_draw(&mut ui, cp.sb, yv[0].as_f64().unwrap(), yv[1].as_f64().unwrap(), false, &th);
            let sbg = &tree["kids"][1]["kids"][0];
            let Kind::Canvas { tk: Some(cv), .. } = &ui.ws[cp.sb].kind else { panic!() };
            check_items(&format!("{name} scrollbar"), cv, sbg["items"].as_array().unwrap(), &c["images"], &fonts);
        }
    }
}
