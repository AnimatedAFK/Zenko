//! The Ctrl+K palette against zenko.py's (golden/hub_palette.json): every item z20_items offers (pages with their
//! Ctrl+N, the cards of the built pages, books, things to do, look choices), what a few searches show (word starts,
//! a typo found by difflib, two words, the "Ask" row), and the box itself: every widget's place, size, text, font
//! and the highlighted row, wide in snow and in a small midnight window.
mod common;
use common::{cmp_tree, fonts_of};
use serde_json::Value;
use zenko_desktop::palette::{ItemsIn, PItem, Palette, items, results};

fn golden() -> Vec<Value> {
    serde_json::from_str(&std::fs::read_to_string("../../golden/hub_palette.json").unwrap()).unwrap()
}

fn key(v: &Value) -> (String, String, String, String, f64) {
    let s = |k: &str| v[k].as_str().unwrap_or("").to_string();
    (s("title"), s("kind"), s("sub"), s("words"), v["boost"].as_f64().unwrap_or(0.0))
}

fn pkey(p: &PItem) -> (String, String, String, String, f64) {
    (p.title.clone(), p.kind.clone(), p.sub.clone(), p.words.clone(), p.boost)
}

fn rust_items(c: &Value) -> Vec<PItem> {
    let nav: Vec<(String, String)> = zenko_desktop::custpage::NAV_ALL.iter().map(|(k, l)| (k.to_string(), l.to_string())).collect();
    let visible: Vec<String> = c["navkeys"].as_array().unwrap().iter().map(|v| v.as_str().unwrap().into()).collect();
    // the cards zenko.py had registered (their raw titles are the items' words), page by their label
    let cards: Vec<(String, String)> = c["items"].as_array().unwrap().iter()
        .filter(|v| v["kind"] == "Setting" || v["kind"] == "Section")
        .map(|v| {
            let lab = v["sub"].as_str().unwrap();
            (nav.iter().find(|n| n.1 == lab).unwrap().0.clone(), v["words"].as_str().unwrap().to_string())
        }).collect();
    items(&ItemsIn { nav_all: &nav, visible: &visible, cards: &cards, session: false, fox: c["fox_name"].as_str().unwrap() })
}

#[test]
fn palette_items_and_results_match_zenko_py() {
    for c in &golden() {
        let name = c["name"].as_str().unwrap();
        let got = rust_items(c);
        let want: Vec<_> = c["items"].as_array().unwrap().iter().map(key).collect();
        assert_eq!(got.len(), want.len(), "{name}: item count");
        for (g, w) in got.iter().zip(&want) {
            assert_eq!(&pkey(g), w, "{name}");
        }
        let q = c["query"].as_str().unwrap();
        let shown: Vec<_> = results(q, &got, c["fox_name"].as_str().unwrap()).iter().map(pkey).collect();
        let want: Vec<_> = c["shown"].as_array().unwrap().iter().map(key).collect();
        assert_eq!(shown, want, "{name}: results for {q:?}");
    }
    // more searches, scored by the same rules (zk_match by hand from zenko.py)
    use zenko_desktop::palette::zk_match;
    assert_eq!(zk_match("", "Home", ""), 1.0);
    assert_eq!(zk_match("ho", "Home", ""), 6.0);
    assert_eq!(zk_match("bre", "Box breathing", ""), 4.5);
    assert_eq!(zk_match("eat", "Box breathing", ""), 3.0);
    assert_eq!(zk_match("cal", "Garden", "zen calm"), 2.0);
    assert_eq!(zk_match("alm", "Garden", "zen calm"), 1.2);
    assert_eq!(zk_match("xyz", "Garden", "zen calm"), 0.0);
    assert_eq!(zk_match("gardn", "Zen", "garden"), 0.9090909090909091);
    assert_eq!(zenko_desktop::palette::clean_title("\u{1f338} \u{b7} Sidebar scene (new)"), "Sidebar scene");
    assert_eq!(zenko_desktop::palette::clean_title("Wardrobe (earned with study hours)"), "Wardrobe");
}

#[test]
fn palette_box_matches_zenko_py() {
    let mut fails = vec![];
    for c in &golden() {
        let name = c["name"].as_str().unwrap();
        let th = zenko_desktop::theme::theme(&zenko_desktop::Cfg::from(c["cfg_full"].clone()));
        let fonts = fonts_of(c);
        let its = rust_items(c);
        let mut p = Palette::new(&th, c["font_ui"].as_str().unwrap(), c["hub_w"].as_i64().unwrap() as i32,
                                 c["zk_f"].as_f64().unwrap(), its, c["fox_name"].as_str().unwrap(), &fonts);
        let q: Vec<u16> = c["query"].as_str().unwrap().encode_utf16().collect();
        if let zenko_desktop::ui::Kind::Entry { e, .. } = &mut p.ui.ws[p.entry].kind {
            (e.text, e.pos) = (q.clone(), q.len());      // (typed: the recording has no widths for the entry's text)
        }
        p.sync(&fonts);
        for _ in 0..c["sel"].as_i64().unwrap() {
            p.move_by(1);
        }
        p.ui.layout(&fonts);
        assert_eq!(p.w, c["W"].as_i64().unwrap() as i32, "{name}");
        let hx = c["hub_xy"][0].as_i64().unwrap() as i32;
        let hy = c["hub_xy"][1].as_i64().unwrap() as i32;
        let (x, y) = p.place(hx, hy, c["hub_w"].as_i64().unwrap() as i32, c["hub_h"].as_i64().unwrap() as i32);
        let geo = format!("{}x{}+{}+{}", p.ui.ws[0].w, p.ui.ws[0].h, x, y);
        let mut bad = vec![];
        if geo != c["pal_geo"].as_str().unwrap() {
            bad.push(format!("geometry {geo} want {}", c["pal_geo"]));
        }
        cmp_tree(&p.ui, 0, &c["tree"], 0, 0, name, &mut bad);
        // the rows' colours: the picked one in the accent's tint
        let list = &c["tree"]["kids"][0]["kids"][2]["kids"];
        for (i, (&r, g)) in p.rows.iter().zip(list.as_array().unwrap()).enumerate() {
            let want = g["opts"]["bg"].as_str().or(g["opts"]["background"].as_str()).unwrap();
            if p.ui.ws[r].bg != want {
                bad.push(format!("row {i} bg {} want {want}", p.ui.ws[r].bg));
            }
            for (&k, gk) in p.ui.ws[r].kids.iter().zip(g["kids"].as_array().unwrap()) {
                let want = gk["opts"]["bg"].as_str().or(gk["opts"]["background"].as_str()).unwrap();
                if p.ui.ws[k].bg != want {
                    bad.push(format!("row {i} child bg {} want {want}", p.ui.ws[k].bg));
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
    assert!(fails.is_empty(), "{fails:?}");
}
