//! The hub sidebar's living logo against zenko.py (golden/hub_logo.json, tools/make_golden_logo.py): for every page
//! and a few clock readings, her mood, what she wears, and the exact pixels of her tails and body.

use serde_json::{Value, json};
use zenko_desktop::hubanim::HubAnim;
use zenko_desktop::{Cfg, Companion};

#[test]
fn logo_matches_zenko_py() {
    let g: Value = serde_json::from_str(include_str!("../../../golden/hub_logo.json")).unwrap();
    let d = std::env::temp_dir().join(format!("zk_logo_{}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    unsafe { std::env::set_var("APPDATA", &d) };
    zenko_desktop::set_fake_now(Some(1744970400.0));
    let mut fox = Companion::new(Cfg::from(json!({})), 1.0, 1000.0);
    // one hub, one picture cache, as in the golden run
    let mut a = HubAnim::new(0.0, 7);
    for r in g.as_array().unwrap() {
        let page = r["page"].as_str().unwrap();
        let now = r["mono"].as_f64().unwrap();
        let at = format!("{page} at {now}");
        fox.hub_page = Some(page.into());
        (a.blink_at, a.blink_until) = (now + 100.0, 0.0);
        let lf = fox.hub_logo(now, Some(page), &mut a, true);
        assert_eq!(fox.current_mood(now), r["mood"].as_str().unwrap(), "{at} mood");
        let acc: Vec<String> = r["acc"].as_array().unwrap().iter().map(|v| v.as_str().unwrap().to_string()).collect();
        assert_eq!(fox.accessories(now), acc, "{at} accessories");
        let pal = fox.pal.clone();
        let cmp = |name: &str, grid: &zenko_art::core::Grid, want: &Value| {
            let im = zenko_art::raster::sprite_photo(&[grid], &pal, 1, true);
            let rows = want.as_array().unwrap();
            assert_eq!((im.w, im.h), (rows[0].as_array().unwrap().len(), rows.len()), "{at} {name} size");
            for (y, row) in rows.iter().enumerate() {
                for (x, v) in row.as_array().unwrap().iter().enumerate() {
                    let q = im.px[y * im.w + x];
                    let mine = if q[3] == 0 { "#00000000".to_string() }
                               else { format!("#{:02x}{:02x}{:02x}{:02x}", q[0], q[1], q[2], q[3]) };
                    let w = v.as_str().unwrap();
                    let w = if w.ends_with("00") && w.len() == 9 { "#00000000" } else { w };
                    assert_eq!(mine, w, "{at} {name} pixel {x},{y}");
                }
            }
        };
        match (&lf.tails, r["tails"].is_null()) {
            (None, true) => {}
            (Some((_, gr)), false) => cmp("tails", gr, &r["tails"]),
            (t, _) => panic!("{at}: tails shown {} vs {}", t.is_some(), !r["tails"].is_null()),
        }
        cmp("body", &lf.body.1, &r["body"]);
    }
    zenko_desktop::set_fake_now(None);
}
