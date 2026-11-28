//! The hub sidebar against zenko.py's real one (golden/hub_side.json, tools/make_golden_hub.py): every canvas item in
//! stacking order (type, coordinates, colours, widths, tags, fonts, hidden or not) and every picture's RGBA bytes.

use serde_json::Value;
use zenko_desktop::hubside::{Side, SideCtx, daypart, season_of};
use zenko_desktop::tkcanvas::CKind;
use zenko_desktop::ui::{Font, Fonts};
use std::collections::HashMap;

/// the font numbers Tk measured in the golden run, keyed by (family, size, bold)
struct Table(HashMap<(String, i32, bool), (i32, i32, HashMap<String, i32>)>);

impl Fonts for Table {
    fn width(&self, f: &Font, s: &str) -> i32 {
        if s.is_empty() {
            return 0;
        }
        let e = &self.0[&(f.family.clone(), f.size, f.bold)];
        *e.2.get(s).unwrap_or_else(|| panic!("no width for {s:?} in {f:?}"))
    }
    fn metrics(&self, f: &Font) -> (i32, i32) {
        let e = &self.0[&(f.family.clone(), f.size, f.bold)];
        (e.0, e.1)
    }
}

fn font_of(s: &str) -> (String, i32, bool) {
    // Tk font description: "{Segoe UI} 12 bold" or "IPAGothic 9"
    let (fam, rest) = if let Some(r) = s.strip_prefix('{') {
        let i = r.find('}').unwrap();
        (r[..i].to_string(), r[i + 1..].trim().to_string())
    } else {
        let mut it = s.splitn(2, ' ');
        (it.next().unwrap().to_string(), it.next().unwrap_or("").to_string())
    };
    let mut p = rest.split_whitespace();
    let size = p.next().unwrap().parse().unwrap();
    (fam, size, rest.contains("bold"))
}

#[test]
fn sidebar_matches_zenko_py() {
    let g: Value = serde_json::from_str(include_str!("../../../golden/hub_side.json")).unwrap();
    for c in g.as_array().unwrap() {
        let name = c["name"].as_str().unwrap();
        let theme = c["cfg_all"]["theme"].as_str().unwrap();
        let cfg = zenko_desktop::Cfg::from(serde_json::json!({"theme": theme}));
        let t = zenko_desktop::theme::theme(&cfg);
        let secs = c["now"].as_f64().unwrap() as i64;
        let days = secs.div_euclid(86400);
        let sod = secs.rem_euclid(86400);
        let (_, month, _) = zenko_desktop::civil_from_days_pub(days);
        let (part, _) = daypart((sod / 3600) as u32, ((sod % 3600) / 60) as u32);
        let status = c["items"].as_array().unwrap().iter().find(|i| i["tags"][0] == "status").unwrap()["opts"]["text"]
            .as_str().unwrap().to_string();
        let ctx = SideCtx {
            t,
            font_ui: "Segoe UI".into(),
            jp: c["jp_font"].as_str().unwrap().into(),
            fox_name: c["fox_name"].as_str().unwrap().into(),
            catchphrase: c["catchphrase"].as_str().unwrap().into(),
            part: part.into(),
            season: season_of(month).into(),
            sky: c["look"]["sky"].as_bool().unwrap(),
            seasons: c["look"]["season"].as_bool().unwrap(),
            weather: None,
            nav_compact: c["cfg_all"].get("nav_compact").and_then(|v| v.as_bool()).unwrap_or(false),
            zk_f: c["zk_f"].as_f64().unwrap(),
            hd: true,
            pins: vec![],
            linked: false,
            scene: String::new(),
        };
        let navkeys = c["navkeys"].as_array().unwrap().iter()
            .map(|k| (k[0].as_str().unwrap().to_string(), k[1].as_str().unwrap().to_string())).collect();
        let mut s = Side::new(navkeys, 2);
        // the first layout runs while the canvas is still 1 x 1, so the logo zoom drops to 1 for good (as in zenko.py)
        (s.w, s.h) = (1, 1);
        s.layout(ctx.nav_compact);
        (s.w, s.h) = (c["W"].as_i64().unwrap() as i32, c["H"].as_i64().unwrap() as i32);
        s.page = c["page"].as_str().map(String::from);
        s.status_text = status;
        s.draw(&ctx);
        s.hover = c["hover"].as_str().map(String::from);
        s.draw(&ctx);
        let lay = s.layout(ctx.nav_compact);
        assert_eq!(serde_json::json!([lay.0, lay.1, lay.2, lay.3, lay.4]), c["layout"], "{name} layout");
        // hub_anim re-colours every third star as it twinkles, so those fills move with the clock
        let twinkle: Vec<u32> = s.stars.iter().step_by(3).map(|x| x.0).collect();
        let got: Vec<_> = s.cv.items.iter().filter(|i| !i.has("flake") && !i.has("logo")).collect();
        let want = c["items"].as_array().unwrap();
        for (k, (gi, wi)) in got.iter().zip(want).enumerate() {
            let at = format!("{name} item {k} {:?} {:?}", wi["type"], wi["tags"]);
            assert_eq!(gi.kind.tk_name(), wi["type"], "{at}");
            let tags: Vec<&str> = wi["tags"].as_array().unwrap().iter().map(|v| v.as_str().unwrap()).collect();
            assert_eq!(gi.tags, tags, "{at}");
            let wc: Vec<f64> = wi["coords"].as_array().unwrap().iter().map(|v| v.as_f64().unwrap()).collect();
            assert_eq!(gi.coords.len(), wc.len(), "{at} coords");
            for (a, b) in gi.coords.iter().zip(&wc) {
                assert!((a - b).abs() < 1e-6, "{at} coords {:?} vs {:?}", gi.coords, wc);
            }
            let o = &wi["opts"];
            let st = |k: &str| o.get(k).and_then(|v| v.as_str()).unwrap_or("").to_string();
            assert_eq!(gi.hidden, st("state") == "hidden", "{at} state");
            match gi.kind {
                CKind::Image => {
                    assert_eq!(gi.anchor, st("anchor"), "{at}");
                    let img = &c["images"][st("image")];
                    let rows = img["rgba"].as_array().unwrap_or_else(|| panic!("{at}: no rgba for picture"));
                    let p = gi.image.as_ref().unwrap();
                    assert_eq!((p.w, p.h), (rows[0].as_array().unwrap().len(), rows.len()), "{at} picture size");
                    for (y, row) in rows.iter().enumerate() {
                        for (x, v) in row.as_array().unwrap().iter().enumerate() {
                            let q = p.px[y * p.w + x];
                            let mine = format!("#{:02x}{:02x}{:02x}{:02x}", q[0], q[1], q[2], q[3]);
                            assert_eq!(mine, v.as_str().unwrap(), "{at} pixel {x},{y}");
                        }
                    }
                }
                CKind::Text => {
                    assert_eq!(gi.text, st("text"), "{at}");
                    assert_eq!(gi.fill, st("fill"), "{at} fill");
                    assert_eq!(gi.anchor, st("anchor"), "{at} anchor");
                    let f = gi.font.as_ref().unwrap();
                    assert_eq!((f.family.clone(), f.size, f.bold), font_of(&st("font")), "{at} font");
                    let w: f64 = st("width").parse().unwrap_or(0.0);
                    assert_eq!(gi.wrap, w, "{at} width");
                }
                _ => {
                    if !twinkle.contains(&gi.id) {
                        assert_eq!(gi.fill, st("fill"), "{at} fill");
                    }
                    assert_eq!(gi.outline, st("outline"), "{at} outline");
                    assert_eq!(gi.width, st("width").parse::<f64>().unwrap(), "{at} width");
                    if gi.kind == CKind::Line || gi.kind == CKind::Poly {
                        assert_eq!(gi.smooth, matches!(st("smooth").as_str(), "true" | "1" | "bezier"), "{at} smooth");
                        assert_eq!(gi.joinstyle, st("joinstyle"), "{at} joinstyle");
                    }
                    if gi.kind == CKind::Line {
                        assert_eq!(gi.capstyle, st("capstyle"), "{at} capstyle");
                    }
                }
            }
        }
        assert_eq!(got.len(), want.len(), "{name}: item count");
        // where Tk puts each text and picture (its bbox), from the font numbers it measured
        // ("{Segoe UI} 10" and "{Segoe UI} 10 normal" are one font: their widths are merged)
        let mut fonts = Table(HashMap::new());
        for (k, v) in c["fonts"].as_object().unwrap() {
            let e = fonts.0.entry(font_of(k)).or_insert((v["ascent"].as_i64().unwrap() as i32,
                                                         v["descent"].as_i64().unwrap() as i32, HashMap::new()));
            e.2.extend(v["w"].as_object().unwrap().iter().map(|(s, n)| (s.clone(), n.as_i64().unwrap() as i32)));
        }
        for (gi, wi) in got.iter().zip(want) {
            let bb: Vec<i32> = wi["bbox"].as_array().unwrap().iter().map(|v| v.as_i64().unwrap() as i32).collect();
            if gi.hidden || bb.is_empty() {
                continue;
            }
            match gi.kind {
                CKind::Text => assert_eq!(zenko_desktop::canvasdraw::text_bbox(gi, &fonts).to_vec(), bb,
                                          "{name} text {:?} bbox", gi.text),
                CKind::Image => {
                    let (x, y) = zenko_desktop::canvasdraw::image_xy(gi);
                    let p = gi.image.as_ref().unwrap();
                    assert_eq!(vec![x, y, x + p.w as i32, y + p.h as i32], bb, "{name} picture {:?} bbox", gi.tags);
                }
                _ => {}
            }
        }
    }
}
