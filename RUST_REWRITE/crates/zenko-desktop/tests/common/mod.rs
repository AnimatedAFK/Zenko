//! Shared by the golden tests of Tk windows: Tk font names, a font table from the golden run, canvas item checks.
#![allow(dead_code)]

use serde_json::Value;
use std::collections::HashMap;
use zenko_desktop::tkcanvas::{CKind, Canvas};
use zenko_desktop::ui::{Font, Fonts, Kind, Ui};

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

/// font_of plus italic
pub fn font_key(s: &str) -> (String, i32, bool, bool) {
    let (f, n, b) = font_of(s);
    let rest = if s.starts_with('{') { s.split_once('}').map(|x| x.1).unwrap_or("") } else { s };
    (f, n, b, rest.contains("italic"))
}

/// (ascent, descent, widths of whole strings, widths of single characters: a string not in the first map is the sum of
/// its characters, as Tk measures runs on both X11 and GDI)
pub struct Table(pub HashMap<(String, i32, bool, bool), (i32, i32, HashMap<String, i32>, HashMap<char, i32>)>);

impl Fonts for Table {
    fn width(&self, f: &Font, s: &str) -> i32 {
        if s.is_empty() {
            return 0;
        }
        let e = self.0.get(&(f.family.clone(), f.size, f.bold, f.italic)).unwrap_or_else(|| panic!("no font {f:?}"));
        if let Some(w) = e.2.get(s) {
            return *w;
        }
        s.chars().map(|c| *e.3.get(&c).unwrap_or_else(|| panic!("no width for {s:?} ({c:?}) in {f:?}"))).sum()
    }
    fn metrics(&self, f: &Font) -> (i32, i32) {
        let e = self.0.get(&(f.family.clone(), f.size, f.bold, f.italic)).unwrap_or_else(|| panic!("no font {f:?}"));
        (e.0, e.1)
    }
}


/// the font numbers of a golden case ("{Segoe UI} 10" and "{Segoe UI} 10 normal" are one font; a named font like
/// "font12" is looked up by what Tk says it really is)
pub fn table(c: &Value) -> Table {
    let mut fonts = Table(HashMap::new());
    for (k, v) in c["fonts"].as_object().unwrap() {
        let key = match v.get("actual") {
            Some(a) if !k.contains(' ') && !k.starts_with('{') => (a["family"].as_str().unwrap().to_string(),
                a["size"].as_i64().unwrap() as i32, a["weight"] == "bold", a["slant"] == "italic"),
            _ => font_key(k),
        };
        let e = fonts.0.entry(key).or_insert((v["ascent"].as_i64().unwrap() as i32, v["descent"].as_i64().unwrap() as i32,
                                              HashMap::new(), HashMap::new()));
        e.2.extend(v["w"].as_object().unwrap().iter().map(|(s, n)| (s.clone(), n.as_i64().unwrap() as i32)));
        if let Some(ch) = v.get("ch").and_then(|c| c.as_object()) {
            e.3.extend(ch.iter().map(|(s, n)| (s.chars().next().unwrap(), n.as_i64().unwrap() as i32)));
        }
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
        // ("current" is Tk's tag for the item under the pointer: where the recording's pointer happened to be)
        let tags: Vec<&str> = wi["tags"].as_array().unwrap().iter().map(|v| v.as_str().unwrap()).filter(|t| *t != "current").collect();
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
                if std::env::var("ZK_PIXDIFF").is_ok() {
                    // (debugging: where the picture differs, in sprite pixels at the zoom given)
                    let z: usize = std::env::var("ZK_PIXDIFF").unwrap().parse().unwrap_or(1);
                    let mut bad = std::collections::BTreeSet::new();
                    for (yy, row) in rows.iter().enumerate() {
                        for (xx, v) in row.as_array().unwrap().iter().enumerate() {
                            let q = p.px[yy * p.w + xx];
                            let g = format!("#{:02x}{:02x}{:02x}{:02x}", q[0], q[1], q[2], if q[3] > 0 { 255 } else { 0 });
                            if g != v.as_str().unwrap() && !(q[3] > 0 && !v.as_str().unwrap().ends_with("ff")) {
                                bad.insert((xx / z, yy / z, g, v.as_str().unwrap().to_string()));
                            }
                        }
                    }
                    eprintln!("{at}: {} sprite pixels differ: {:?}", bad.len(), bad.iter().take(40).collect::<Vec<_>>());
                    let xs: Vec<usize> = bad.iter().map(|b| b.0).collect(); let ys: Vec<usize> = bad.iter().map(|b| b.1).collect();
                    eprintln!("  box x {:?}..{:?} y {:?}..{:?}", xs.iter().min(), xs.iter().max(), ys.iter().min(), ys.iter().max());
                }
                for (yy, row) in rows.iter().enumerate() {
                    for (xx, v) in row.as_array().unwrap().iter().enumerate() {
                        let mut q = p.px[yy * p.w + xx];
                        // (Tk 8.6's photo get / transparency get only tell see-through from not: a soft glow's
                        // partial alpha is recorded as opaque)
                        if q[3] > 0 && v.as_str().unwrap().ends_with("ff") {
                            q[3] = 255;
                        }
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
                let fs = s("font");
                if !fs.contains(' ') && !fs.starts_with('{') {
                    // a named font (tkfont.Font): what Tk says it is, under the family the app asked for
                    let a = &images["__fonts"][&fs]["actual"];
                    if a.is_object() {
                        assert_eq!((f.size, f.bold), (a["size"].as_i64().unwrap() as i32, a["weight"] == "bold"), "{at} font");
                    }
                } else {
                    assert_eq!((f.family.clone(), f.size, f.bold), font_of(&fs), "{at} font");
                }
                assert_eq!(it.underline, fs.contains("underline"), "{at} underline");
                if io.get("justify").is_some() {
                    assert_eq!(it.justify, s("justify"), "{at} justify");
                }
                assert_eq!(it.wrap, s("width").parse::<f64>().unwrap_or(0.0), "{at} wrap");
                if !bb.is_empty() {
                    assert_eq!(zenko_desktop::canvasdraw::text_bbox(it, fonts).to_vec(), bb, "{at} bbox");
                }
            }
            _ => {
                assert_eq!(it.stipple, s("stipple"), "{at} stipple");
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
                if it.kind == CKind::Arc {
                    assert!((it.start - s("start").parse::<f64>().unwrap()).abs() < 1e-6, "{at} start {}", it.start);
                    assert!((it.extent - s("extent").parse::<f64>().unwrap()).abs() < 1e-6, "{at} extent {}", it.extent);
                    assert_eq!(it.style, s("style"), "{at} style");
                }
            }
        }
    }
}

// ---- hub pages: the widget tree against a golden page dump ----

pub fn cmp_tree(ui: &Ui, id: usize, g: &Value, ox: i32, oy: i32, at: &str, bad: &mut Vec<String>) {
    let w = &ui.ws[id];
    let (ax, ay) = ui.abs(id);
    let gi = |k: &str| g[k].as_i64().unwrap() as i32;
    let want = (gi("x") - ox, gi("y") - oy, gi("w"), gi("h"), g["mapped"].as_bool().unwrap());
    if !want.4 {
        if w.mapped {
            bad.push(format!("{at} {}: mapped, want unmapped", g["cls"]));
        }
        return;
    }
    let got = (ax, ay, w.w, w.h, w.mapped);
    if got != want {
        bad.push(format!("{at} {}: {got:?} want {want:?} (req {}x{} want {}x{})", g["cls"], w.rw, w.rh, gi("rw"), gi("rh")));
    }
    let o = &g["opts"];
    if g["cls"] == "Label" || g["cls"] == "Checkbutton" {
        if let Kind::Label { text, font, image: None, .. } | Kind::Check { text, font, .. } = &w.kind {
            let wt = o["text"].as_str().unwrap_or("");
            if text != wt {
                bad.push(format!("{at}: text {text:?} want {wt:?}"));
            }
            let (fam, n, b) = font_of(o["font"].as_str().unwrap());
            let fam = if fam == "Segoe UI" { "Segoe UI".to_string() } else { fam };
            if (font.size, font.bold) != (n, b) || (fam != font.family && fam != "Consolas") {
                bad.push(format!("{at}: font {font:?} want {}", o["font"]));
            }
        }
    }
    let kids = g["kids"].as_array().unwrap();
    if kids.len() != w.kids.len() {
        bad.push(format!("{at}: {} kids, want {}", w.kids.len(), kids.len()));
        return;
    }
    for (k, (&c, gk)) in w.kids.iter().zip(kids).enumerate() {
        cmp_tree(ui, c, gk, ox, oy, &format!("{at}/{k}"), bad);
    }
}

pub fn find<'a>(g: &'a Value, pred: &dyn Fn(&Value) -> bool, out: &mut Vec<&'a Value>) {
    if pred(g) {
        out.push(g);
    }
    for k in g["kids"].as_array().unwrap() {
        find(k, pred, out);
    }
}

pub fn fonts_of(c: &Value) -> Table {
    let mut fonts = table(c);
    let alias: Vec<_> = fonts.0.iter().filter(|(k, _)| k.0 == "Inter").map(|(k, v)| (k.clone(), v.clone())).collect();
    for (k, v) in alias {
        let e = fonts.0.entry(("Segoe UI".into(), k.1, k.2, k.3)).or_insert_with(|| v.clone());
        for (s, n) in v.2 {
            e.2.entry(s).or_insert(n);
        }
    }
    let dv: Vec<_> = fonts.0.iter().filter(|(k, _)| k.0 == "DejaVu Sans Mono").map(|(k, v)| (k.clone(), v.clone())).collect();
    for (k, v) in dv {
        let e = fonts.0.entry(("Consolas".into(), k.1, k.2, k.3)).or_insert_with(|| v.clone());
        for (s, n) in v.2 {
            e.2.entry(s).or_insert(n);
        }
    }
    fonts
}

