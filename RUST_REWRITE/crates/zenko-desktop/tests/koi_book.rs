//! The koi book window against zenko.py (golden/koi_book.json from tools/make_golden_page.py koibook): nothing caught,
//! two kinds caught (the first picked, one in the pond), and an uncaught fish's hint picked.

mod common;

use common::{check_items, cmp_tree, fonts_of};
use serde_json::Value;
use zenko_desktop::koibook::{KoiBook, geometry};
use zenko_desktop::ui::Ui;

#[test]
fn koi_book_matches_zenko_py() {
    let g: Vec<Value> = serde_json::from_str(&std::fs::read_to_string("../../golden/koi_book.json").unwrap()).unwrap();
    let mut fails = vec![];
    for c in &g {
        let name = c["name"].as_str().unwrap();
        eprintln!("case {name}");
        let th = zenko_desktop::theme::theme(&zenko_desktop::Cfg::from(serde_json::json!({"theme": c["cfg_all"]["theme"]})));
        let fonts = fonts_of(c);
        let tree = &c["tree"];
        let sc = c["screen"].as_array().unwrap();
        let (w, h, x, y) = geometry(sc[0].as_i64().unwrap() as i32, sc[1].as_i64().unwrap() as i32);
        assert_eq!(format!("{w}x{h}+{x}+{y}"), c["book_geo"].as_str().unwrap(), "{name} geometry");
        let mut ui = Ui::new(&th.paper);
        ui.x11 = true;
        let fishing = &c["life_all"]["fishing"];
        let mut k = KoiBook::build(&mut ui, &th, "Segoe UI", c["mincho"].as_str().unwrap(), c["serif2"].as_str().unwrap(), fishing, 393);
        if name == "koi_hint" {
            // (a click on its tile: picked, the grid and the panel drawn again)
            k.sel = c["pick"].as_str().map(String::from);
            k.detail(&mut ui);
        }
        k.fit(&mut ui, &fonts, w, h);
        let mut bad = vec![];
        cmp_tree(&ui, 0, tree, 0, 0, name, &mut bad);
        for b in bad.iter().take(30) {
            eprintln!("{b}");
        }
        if !bad.is_empty() {
            fails.push(format!("{name}: {}", bad.len()));
            continue;
        }
        let mut imgs = c["images"].clone();
        common::expand_images(&mut imgs);
        let gl = &tree["kids"][1]["kids"][0]["kids"];
        check_items(&format!("{name} grid"), ui.ws[k.cv].tk_canvas().unwrap(), gl[0]["items"].as_array().unwrap(), &imgs, &fonts);
        check_items(&format!("{name} bar"), ui.ws[k.sb].tk_canvas().unwrap(), gl[1]["items"].as_array().unwrap(), &imgs, &fonts);
    }
    assert!(fails.is_empty(), "{fails:?}");
}
