//! Shared by the golden tests of Tk windows: Tk font names, a font table from the golden run, canvas item checks.
#![allow(dead_code)]

use serde_json::Value;
use std::collections::HashMap;
use zenko_desktop::tkcanvas::{CKind, Canvas};
use zenko_desktop::ui::{Font, Fonts};

pub fn font_of(s: &str) -> (String, i32, bool) {
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

pub struct Table(HashMap<(String, i32, bool), (i32, i32, HashMap<String, i32>)>);

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


/// the font numbers of a golden case ("{Segoe UI} 10" and "{Segoe UI} 10 normal" are one font; a named font like
/// "font12" is looked up by what Tk says it really is)
pub fn table(c: &Value) -> Table {
    let mut fonts = Table(HashMap::new());
    for (k, v) in c["fonts"].as_object().unwrap() {
        let key = match v.get("actual") {
            Some(a) if !k.contains(' ') && !k.starts_with('{') => {
                (a["family"].as_str().unwrap().to_string(), a["size"].as_i64().unwrap() as i32, a["weight"] == "bold")
            }
            _ => font_of(k),
        };
        let e = fonts.0.entry(key).or_insert((v["ascent"].as_i64().unwrap() as i32, v["descent"].as_i64().unwrap() as i32,
                                              HashMap::new()));
        e.2.extend(v["w"].as_object().unwrap().iter().map(|(s, n)| (s.clone(), n.as_i64().unwrap() as i32)));
    }
    fonts
}

/// every item of a canvas against the golden dump: type, coordinates, tags, colours, widths, texts, fonts, boxes and
/// picture pixels
pub fn check_items(at: &str, cv: &Canvas, want: &[Value], images: &Value, fonts: &Table) {
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
        assert_eq!(it.hidden, s("state") == "hidden", "{at} state");
        match it.kind {
            CKind::Image => {
                assert_eq!(it.anchor, s("anchor"), "{at}");
                let rows = images[s("image")].as_array().unwrap();
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
                assert_eq!(it.wrap, s("width").parse::<f64>().unwrap_or(0.0), "{at} wrap");
                if !bb.is_empty() {
                    assert_eq!(zenko_desktop::canvasdraw::text_bbox(it, fonts).to_vec(), bb, "{at} bbox");
                }
            }
            _ => {
                assert_eq!(it.fill, s("fill"), "{at} fill");
                assert_eq!(it.outline, s("outline"), "{at} outline");
                assert_eq!(it.width, s("width").parse::<f64>().unwrap(), "{at} width");
                if it.kind == CKind::Line || it.kind == CKind::Poly {
                    assert_eq!(it.joinstyle, s("joinstyle"), "{at} joinstyle");
                    assert_eq!(it.smooth, matches!(s("smooth").as_str(), "true" | "1" | "bezier"), "{at} smooth");
                }
                if it.kind == CKind::Line {
                    assert_eq!(it.capstyle, s("capstyle"), "{at} capstyle");
                }
            }
        }
    }
}
