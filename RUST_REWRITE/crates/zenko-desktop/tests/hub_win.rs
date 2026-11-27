//! The hub window's frame against zenko.py's real one (golden/hub_win.json, tools/make_golden_hubwin.py): every
//! widget's place and size, colours, fonts and texts, every item on the border, window-button and search-pill
//! canvases, and the border's wood pixels.

use serde_json::Value;
use std::collections::HashMap;
use zenko_desktop::hubwin::{Hub, zk_factor};
use zenko_desktop::tkcanvas::CKind;
use zenko_desktop::ui::{Font, Fonts, Kind};

fn font_of(s: &str) -> (String, i32, bool) {
    let (fam, rest) = if let Some(r) = s.strip_prefix('{') {
        let i = r.find('}').unwrap();
        (r[..i].to_string(), r[i + 1..].trim().to_string())
    } else {
        let mut it = s.splitn(2, ' ');
        (it.next().unwrap().to_string(), it.next().unwrap_or("").to_string())
    };
    let size = rest.split_whitespace().next().unwrap().parse().unwrap();
    (fam, size, rest.contains("bold"))
}

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
        let e = self.0.get(&(f.family.clone(), f.size, f.bold)).unwrap_or_else(|| panic!("no font {f:?}"));
        (e.0, e.1)
    }
}

fn check(c: &Value, hub: &Hub, fonts: &Table, g: &Value, id: usize, path: &str) {
    let name = c["name"].as_str().unwrap();
    let w = &hub.ui.ws[id];
    let cls = match &w.kind {
        Kind::Frame => if id == 0 { "Toplevel" } else { "Frame" },
        Kind::Label { .. } => "Label",
        _ => "Canvas",
    };
    let at = format!("{name} {path}/{cls}");
    assert_eq!(g["cls"], cls, "{at}");
    let (x, y) = hub.ui.abs(id);
    let gi = |k: &str| g[k].as_i64().unwrap() as i32;
    assert_eq!((x, y, w.w, w.h), (gi("x"), gi("y"), gi("w"), gi("h")), "{at} place");
    if g["manager"] != "place" && id != 0 {
        assert_eq!((w.rw, w.rh), (gi("rw"), gi("rh")), "{at} requested size");
    }
    let o = &g["opts"];
    let st = |k: &str| o[k].as_str().unwrap_or("").to_string();
    assert_eq!(w.bg, st("background"), "{at} bg");
    if let Kind::Label { text, font, fg, .. } = &w.kind {
        assert_eq!(*text, st("text"), "{at} text");
        assert_eq!((font.family.clone(), font.size, font.bold), font_of(&st("font")), "{at} font");
        if Some(id) != Some(hub.ids.tip) {
            assert_eq!(*fg, st("foreground"), "{at} fg");
        }
        assert_eq!((w.padx, w.pady), (st("padx").parse().unwrap(), st("pady").parse().unwrap()), "{at} pad");
    }
    if let Kind::Canvas { tk: Some(cv), .. } = &w.kind
        && let Some(want) = g.get("items").and_then(|v| v.as_array()) {
        assert_eq!(cv.items.len(), want.len(), "{at} item count");
        for (k, (it, wi)) in cv.items.iter().zip(want).enumerate() {
            let at = format!("{at} item {k} {}", wi["type"]);
            assert_eq!(it.kind.tk_name(), wi["type"], "{at}");
            let wc: Vec<f64> = wi["coords"].as_array().unwrap().iter().map(|v| v.as_f64().unwrap()).collect();
            assert_eq!(it.coords.len(), wc.len(), "{at} coords {:?} {:?}", it.coords, wc);
            for (a, b) in it.coords.iter().zip(&wc) {
                assert!((a - b).abs() < 1e-6, "{at} coords {:?} vs {:?}", it.coords, wc);
            }
            let tags: Vec<&str> = wi["tags"].as_array().unwrap().iter().map(|v| v.as_str().unwrap()).collect();
            assert_eq!(it.tags, tags, "{at} tags");
            let io = &wi["opts"];
            let s = |k: &str| io.get(k).and_then(|v| v.as_str()).unwrap_or("").to_string();
            let bb: Vec<i32> = wi["bbox"].as_array().unwrap().iter().map(|v| v.as_i64().unwrap() as i32).collect();
            match it.kind {
                CKind::Image => {
                    assert_eq!(it.anchor, s("anchor"), "{at}");
                    let rows = c["images"][s("image")].as_array().unwrap();
                    let p = it.image.as_ref().unwrap();
                    assert_eq!((p.w, p.h), (rows[0].as_array().unwrap().len(), rows.len()), "{at} size");
                    for (yy, row) in rows.iter().enumerate() {
                        for (xx, v) in row.as_array().unwrap().iter().enumerate() {
                            let q = p.px[yy * p.w + xx];
                            assert_eq!(format!("#{:02x}{:02x}{:02x}{:02x}", q[0], q[1], q[2], q[3]), v.as_str().unwrap(),
                                       "{at} pixel {xx},{yy}");
                        }
                    }
                }
                CKind::Text => {
                    assert_eq!(it.text, s("text"), "{at}");
                    assert_eq!(it.fill, s("fill"), "{at} fill");
                    assert_eq!(it.anchor, s("anchor"), "{at} anchor");
                    let f = it.font.as_ref().unwrap();
                    assert_eq!((f.family.clone(), f.size, f.bold), font_of(&s("font")), "{at} font");
                    assert_eq!(zenko_desktop::canvasdraw::text_bbox(it, fonts).to_vec(), bb, "{at} bbox");
                }
                _ => {
                    assert_eq!(it.fill, s("fill"), "{at} fill");
                    assert_eq!(it.outline, s("outline"), "{at} outline");
                    assert_eq!(it.width, s("width").parse::<f64>().unwrap(), "{at} width");
                    if it.kind == CKind::Line || it.kind == CKind::Poly {
                        assert_eq!(it.joinstyle, s("joinstyle"), "{at} joinstyle");
                    }
                    if it.kind == CKind::Line {
                        assert_eq!(it.capstyle, s("capstyle"), "{at} capstyle");
                    }
                }
            }
        }
    }
    let kids = hub.ui.stack(id);
    let gk = g["kids"].as_array().unwrap();
    if id == hub.ids.side_cv || id == hub.ids.stage {
        return;
    }
    assert_eq!(kids.len(), gk.len(), "{at} children");
    for (i, (k, gv)) in kids.iter().zip(gk).enumerate() {
        check(c, hub, fonts, gv, *k, &format!("{path}/{i}"));
    }
}

#[test]
fn hub_frame_matches_zenko_py() {
    let g: Value = serde_json::from_str(include_str!("../../../golden/hub_win.json")).unwrap();
    for c in g.as_array().unwrap() {
        let theme = c["cfg_all"]["theme"].as_str().unwrap();
        let cfg = zenko_desktop::Cfg::from(serde_json::json!({"theme": theme}));
        let t = zenko_desktop::theme::theme(&cfg);
        let mut fonts = Table(HashMap::new());
        for (k, v) in c["fonts"].as_object().unwrap() {
            let e = fonts.0.entry(font_of(k)).or_insert((v["ascent"].as_i64().unwrap() as i32,
                                                         v["descent"].as_i64().unwrap() as i32, HashMap::new()));
            e.2.extend(v["w"].as_object().unwrap().iter().map(|(s, n)| (s.clone(), n.as_i64().unwrap() as i32)));
        }
        let geo = c["geometry"].as_str().unwrap();
        let (wh, _) = geo.split_once('+').unwrap();
        let (w, h) = wh.split_once('x').unwrap();
        let (w, h): (i32, i32) = (w.parse().unwrap(), h.parse().unwrap());
        let mut hub = Hub::new(&t, "Segoe UI", vec![], true, true, w, h, c["tip_i"].as_u64().unwrap() as usize, 0.0);
        let f = zk_factor(w, h, 1.0, false);
        assert!((f - c["zk_f"].as_f64().unwrap()).abs() < 1e-12, "zk_f");
        hub.fit_fonts(f);
        hub.tip_update(2.0);
        hub.layout(&fonts, w, h);
        check(c, &hub, &fonts, &c["tree"], 0, "");
    }
}
