//! The opening animation of a book clicked on the shelf against zenko.py's book_anim (golden/hub_bookanim.json):
//! all 21 frames' canvas items (the book gliding and growing with its seigaiha cover, then the cover swinging open
//! over the ruled pages), their delays, and the book that ends up open.
use serde_json::Value;
use zenko_desktop::shelfpage::{ANIM_A, ANIM_B, anim_delay, anim_frame, shelf_layout};
use zenko_desktop::tkcanvas::{CKind, Canvas};

fn font_of(s: &str) -> (String, i32, bool) {
    let (fam, rest) = if let Some(r) = s.strip_prefix('{') {
        let e = r.find('}').unwrap();
        (r[..e].to_string(), r[e + 1..].trim().to_string())
    } else {
        let mut it = s.splitn(2, ' ');
        (it.next().unwrap().to_string(), it.next().unwrap_or("").to_string())
    };
    let mut p = rest.split_whitespace();
    let size = p.next().unwrap().parse().unwrap();
    (fam, size, rest.contains("bold"))
}

#[test]
fn book_anim_matches_zenko_py() {
    let g: Vec<Value> = serde_json::from_str(&std::fs::read_to_string("../../golden/hub_bookanim.json").unwrap()).unwrap();
    let dir = std::env::temp_dir().join(format!("zk_banim_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("FoxBuddy")).unwrap();
    unsafe { std::env::set_var("APPDATA", &dir) };
    for c in &g {
        let name = c["name"].as_str().unwrap();
        let key = c["book"].as_str().unwrap();
        assert_eq!(c["nb_last"].as_str(), Some(key), "{name}: nb_last");
        let now = c["now"].as_f64().unwrap();
        zenko_desktop::set_fake_now(Some(now));
        std::fs::write(dir.join("FoxBuddy").join("zenko_life.json"), c["life_all"].to_string()).unwrap();
        let mut fox = zenko_desktop::Companion::new(zenko_desktop::Cfg::from(c["cfg_full"].clone()), 1.0, now);
        let d = fox.book_data(key);
        let col = zenko_desktop::bookpage::book_col(&d);
        // (the timed frames: the shelf's own 40 ms tick records the same frame again in between)
        let frames: Vec<&Value> = c["frames"].as_array().unwrap().iter().filter(|f| f["ms"].as_i64() != Some(40)).collect();
        assert_eq!(frames.len(), ANIM_A + ANIM_B, "{name}: frames");
        let bg = &frames[0]["items"][0]["coords"];
        let (wd, ht) = (bg[2].as_f64().unwrap() as i32, bg[3].as_f64().unwrap() as i32);
        let (spots, ..) = shelf_layout(560.max(wd), 460.max(ht));
        let s = spots.iter().find(|s| s.0 == key).unwrap();
        let jp = c["jp_font"].as_str().unwrap();
        let fu = c["font_ui_name"].as_str().unwrap();
        let mut cv = Canvas::default();
        for (i, f) in frames.iter().enumerate() {
            assert_eq!((anim_delay(i) * 1000.0).round() as i64, f["ms"].as_i64().unwrap(), "{name} frame {i}: delay");
            anim_frame(&mut cv, i, (s.1, s.2, s.3, s.4), wd, ht, key, &col, jp, fu);
            let gi = f["items"].as_array().unwrap();
            assert_eq!(cv.items.len(), gi.len(), "{name} frame {i}: item count");
            for (k, (it, gt)) in cv.items.iter().zip(gi).enumerate() {
                let at = format!("{name} frame {i} item {k}");
                assert_eq!(it.kind.tk_name(), gt["type"].as_str().unwrap(), "{at}: type");
                let gc: Vec<f64> = gt["coords"].as_array().unwrap().iter().map(|v| v.as_f64().unwrap()).collect();
                assert_eq!(it.coords.len(), gc.len(), "{at}: coords");
                for (a, b) in it.coords.iter().zip(&gc) {
                    assert!((a - b).abs() < 1e-6, "{at}: coords {:?} vs {:?}", it.coords, gc);
                }
                let o = &gt["opts"];
                let tags: Vec<&str> = gt["tags"].as_array().unwrap().iter().map(|v| v.as_str().unwrap()).collect();
                assert_eq!(it.tags, tags, "{at}: tags");
                if it.kind != CKind::Image {
                    assert_eq!(it.fill, o["fill"].as_str().unwrap(), "{at}: fill");
                }
                if let Some(ol) = o.get("outline").and_then(|v| v.as_str()) {
                    assert_eq!(it.outline, ol, "{at}: outline");
                }
                if let Some(w) = o.get("width").and_then(|v| v.as_str()) && it.kind != CKind::Text {
                    assert_eq!(it.width, w.parse::<f64>().unwrap(), "{at}: width");
                }
                if let Some(st) = o.get("stipple").and_then(|v| v.as_str()) {
                    assert_eq!(it.stipple, st, "{at}: stipple");
                }
                if it.kind == CKind::Arc {
                    assert_eq!((it.start, it.extent, it.style), (o["start"].as_str().unwrap().parse().unwrap(),
                               o["extent"].as_str().unwrap().parse().unwrap(), o["style"].as_str().unwrap()), "{at}: arc");
                }
                if it.kind == CKind::Text {
                    assert_eq!(it.text, o["text"].as_str().unwrap(), "{at}: text");
                    assert_eq!(it.justify, o["justify"].as_str().unwrap(), "{at}: justify");
                    let f = it.font.as_ref().unwrap();
                    assert_eq!((f.family.clone(), f.size, f.bold), font_of(o["font"].as_str().unwrap()), "{at}: font");
                }
            }
        }
        eprintln!("{name}: {} frames ok", frames.len());
    }
}
