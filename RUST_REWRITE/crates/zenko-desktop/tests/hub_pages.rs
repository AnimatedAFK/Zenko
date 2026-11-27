//! Hub pages against zenko.py's real ones (golden/hub_pages.json, tools/make_golden_page.py).

mod common;
use common::{check_items, table};
use serde_json::Value;

#[test]
fn page_title_banner_matches_zenko_py() {
    let g: Value = serde_json::from_str(include_str!("../../../golden/hub_pages.json")).unwrap();
    for c in g.as_array().unwrap() {
        let name = c["name"].as_str().unwrap();
        let theme = c["cfg_all"]["theme"].as_str().unwrap();
        let t = zenko_desktop::theme::theme(&zenko_desktop::Cfg::from(serde_json::json!({"theme": theme})));
        let fonts = table(c);
        let banner = &c["tree"]["kids"][0];
        let w = banner["w"].as_i64().unwrap() as i32;
        let (title, sub) = match c["page"].as_str().unwrap() {
            "chat" => ("Ask Zenko", "Commands, questions, weather, news, Wikipedia, currency... typos are fine"),
            p => panic!("no title for {p}"),
        };
        let mut cv = zenko_desktop::tkcanvas::Canvas::default();
        let mut seig = None;
        let hh = zenko_desktop::hubwin::hub_title_draw(&mut cv, w, title, sub, &t, "Segoe UI", "",
                                                        c["zk_f"].as_f64().unwrap(), &fonts, &mut seig);
        assert_eq!(hh, banner["h"].as_i64().unwrap() as i32, "{name} banner height");
        check_items(&format!("{name} banner"), &cv, banner["items"].as_array().unwrap(), &c["images"], &fonts);
    }
}
