//! The Study books shelf against zenko.py's (golden/hub_shelf.json): the night sky, petals, the title block, the
//! lacquered cabinet with its shimenawa, shide and lanterns, the nine books (glow, shadow, ribbon, cover art, slip,
//! hanko, name plate, this week's hours), the hanging scroll; a book hovered (lifted 7 px, glowing, the scroll
//! describing it) and a small midnight window. Every canvas item and every pixel of every picture.
mod common;
use common::{check_items, expand_images, fonts_of};
use serde_json::{Value, json};
use zenko_desktop::shelfpage::ShelfPage;
use zenko_desktop::ui::{Kind, Ui};

#[test]
fn shelf_matches_zenko_py() {
    let g: Vec<Value> = serde_json::from_str(&std::fs::read_to_string("../../golden/hub_shelf.json").unwrap()).unwrap();
    let dir = std::env::temp_dir().join(format!("zk_shelf_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("FoxBuddy")).unwrap();
    unsafe { std::env::set_var("APPDATA", &dir) };
    let only = std::env::var("ZK_CASE").unwrap_or_default();
    for c in &g {
        let name = c["name"].as_str().unwrap();
        if !only.is_empty() && only != name {
            continue;
        }
        let now = c["now"].as_f64().unwrap();
        zenko_desktop::set_fake_now(Some(now));
        std::fs::write(dir.join("FoxBuddy").join("zenko_life.json"), c["life_all"].to_string()).unwrap();
        let fox = zenko_desktop::Companion::new(zenko_desktop::Cfg::from(c["cfg_all"].clone()), 1.0, now);
        let mut d = fox.shelf_data();
        d.jp = c["mincho"].as_str().unwrap().into();
        d.serif = c["serif_font"].as_str().unwrap().into();
        let th = zenko_desktop::theme::theme(&zenko_desktop::Cfg::from(json!({"theme": c["cfg_all"]["theme"]})));
        let fonts = fonts_of(c);
        let tree = &c["tree"];
        let (pw, ph) = (tree["w"].as_i64().unwrap() as i32, tree["h"].as_i64().unwrap() as i32);
        let mut ui = Ui::new(&th.paper);
        ui.x11 = true;
        let mut p = ShelfPage::build(&mut ui, 0, &th.paper, 1);
        ui.layout_at(&fonts, pw, ph);
        let gcv = &tree["kids"][0]["kids"][0];
        assert_eq!((ui.ws[p.cv].w, ui.ws[p.cv].h), (gcv["w"].as_i64().unwrap() as i32, gcv["h"].as_i64().unwrap() as i32),
                   "{name}: canvas size");
        // (the petals start where Python's random put them, and drift as they did)
        let gp: Vec<[f64; 4]> = c["petals"].as_array().unwrap().iter()
            .map(|v| { let a: Vec<f64> = v.as_array().unwrap().iter().map(|x| x.as_f64().unwrap()).collect(); [a[0], a[1], a[2], a[3]] })
            .collect();
        p.petals = gp.iter().map(|q| (*q, 0)).collect();
        // (hovered before the drawing: the recording measured only the words the scroll ended up with)
        p.hover = c["hover"].as_str().map(|s| s.to_string());
        p.draw(&mut ui, &fonts, &d);
        if let Some(k) = c["hover"].as_str() {
            for _ in 0..40 {
                p.tick(&mut ui);
            }
            assert_eq!(p.lift.get(k).copied(), c["lift"][k].as_f64(), "{name}: lift");
            let ids: Vec<u32> = p.petals.iter().map(|x| x.1).collect();
            let Kind::Canvas { tk: Some(cv), .. } = &mut ui.ws[p.cv].kind else { panic!() };
            for (q, id) in gp.iter().zip(ids) {
                cv.get(id).unwrap().coords = vec![q[0], q[1]];
            }
        }
        let mut imgs = c["images"].clone();
        expand_images(&mut imgs);
        imgs["__fonts"] = c["fonts"].clone();
        let Kind::Canvas { tk: Some(cv), .. } = &ui.ws[p.cv].kind else { panic!() };
        check_items(name, cv, gcv["items"].as_array().unwrap(), &imgs, &fonts);
        eprintln!("{name}: {} items", cv.items.len());
    }
}
