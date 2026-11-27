//! The daily check-in card against zenko.py in real Tk (golden/checkin.json, tools/make_golden_checkin.py): every
//! widget's requested size and place, every canvas item, where the window goes, and what checkin_save writes.

use serde_json::{Value, json};
use std::collections::HashMap;
use zenko_desktop::ui::{Font, Fonts, Item, Kind, Ui};
use zenko_desktop::{Cfg, Companion, Ev};

static CLOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// Tk's own font numbers from the golden file
struct Table(HashMap<String, (i32, i32, HashMap<String, i32>)>);

impl Table {
    fn key(f: &Font) -> String {
        format!("{}{}", f.size, if f.bold { "b" } else { "" })
    }
}

impl Fonts for Table {
    fn width(&self, f: &Font, s: &str) -> i32 {
        let t = &self.0[&Table::key(f)].2;
        if s.is_empty() {
            return 0;
        }
        *t.get(s).unwrap_or_else(|| panic!("no width for {s:?} in {}", Table::key(f)))
    }
    fn metrics(&self, f: &Font) -> (i32, i32) {
        let e = &self.0[&Table::key(f)];
        (e.0, e.1)
    }
}

fn table(c: &Value) -> Table {
    let mut m = HashMap::new();
    for (k, v) in c["fonts"].as_object().unwrap() {
        let w = v["w"].as_object().unwrap().iter().map(|(s, n)| (s.clone(), n.as_i64().unwrap() as i32)).collect();
        m.insert(k.clone(), (v["ascent"].as_i64().unwrap() as i32, v["descent"].as_i64().unwrap() as i32, w));
    }
    Table(m)
}

fn cls(k: &Kind, id: usize) -> &'static str {
    match k {
        _ if id == 0 => "Toplevel",
        Kind::Frame => "Frame",
        Kind::Label { .. } => "Label",
        Kind::Entry { .. } => "Entry",
        Kind::Button { .. } | Kind::Canvas { .. } => "Canvas",
        Kind::Text { .. } => "Text",
        Kind::Check { radio: true, .. } => "Radiobutton",
        Kind::Check { .. } => "Checkbutton",
    }
}

fn coords(it: &Item) -> (Vec<i64>, &'static str) {
    let flat = |p: &Vec<(i32, i32)>| p.iter().flat_map(|&(x, y)| [x as i64, y as i64]).collect();
    match it {
        Item::Poly { pts, .. } => (flat(pts), "polygon"),
        Item::Line { pts, .. } => (flat(pts), "line"),
        Item::Oval { x1, y1, x2, y2, .. } => (vec![*x1 as i64, *y1 as i64, *x2 as i64, *y2 as i64], "oval"),
        Item::Text { x, y, .. } => (vec![*x as i64, *y as i64], "text"),
    }
}

fn check(u: &Ui, t: &zenko_desktop::theme::Theme, id: usize, want: &Value, path: &str) {
    let w = &u.ws[id];
    let path = format!("{path}/{}", want["cls"].as_str().unwrap());
    assert_eq!(cls(&w.kind, id), want["cls"], "{path}");
    let placed = w.place.is_some();
    if id != 0 {
        assert_eq!((w.x, w.y, w.w, w.h), (as_i(&want["x"]), as_i(&want["y"]), as_i(&want["w"]), as_i(&want["h"])), "{path} place");
    }
    if !placed {
        assert_eq!((w.rw, w.rh), (as_i(&want["rw"]), as_i(&want["rh"])), "{path} requested size");
    }
    assert_eq!(w.mapped, want["mapped"].as_bool().unwrap(), "{path} mapped");
    if let Some(items) = want.get("items") {
        let got = u.items_of(id, t);
        let items = items.as_array().unwrap();
        assert_eq!(got.len(), items.len(), "{path} item count");
        for (g, e) in got.iter().zip(items) {
            let (c, ty) = coords(g);
            assert_eq!(ty, e["type"], "{path}");
            let ec: Vec<i64> = e["coords"].as_array().unwrap().iter().map(|v| v.as_f64().unwrap().round() as i64).collect();
            assert_eq!(c, ec, "{path} {ty} coords");
            let (fill, outline) = match g {
                Item::Poly { fill, outline, .. } => (fill.clone().unwrap_or_default(), outline.clone().unwrap_or_default()),
                Item::Line { color, .. } | Item::Text { color, .. } => (color.clone(), String::new()),
                Item::Oval { fill, .. } => (fill.clone(), String::new()),
            };
            assert_eq!(fill, e["fill"], "{path} {ty} fill");
            if let Some(o) = e.get("outline") {
                assert_eq!(outline, *o, "{path} {ty} outline");
            }
            if let Item::Text { text, .. } = g {
                assert_eq!(text, &e["text"], "{path}");
            }
        }
    }
    let kids = want["kids"].as_array().unwrap();
    assert_eq!(w.kids.len(), kids.len(), "{path} children");
    for (k, e) in w.kids.iter().zip(kids) {
        check(u, t, *k, e, &path);
    }
}

fn as_i(v: &Value) -> i32 {
    v.as_i64().unwrap() as i32
}

#[test]
fn checkin_card_matches_tk() {
    let g: Value = serde_json::from_str(include_str!("../../../golden/checkin.json")).unwrap();
    for c in g["cases"].as_array().unwrap() {
        let fonts = table(c);
        let cfg = Cfg::from(json!({"user_name": c["name"], "theme": c["theme"]}));
        let mut f = Companion::new(cfg.clone(), 1.0, 1000.0);
        (f.w, f.colw) = (437, 250);
        let t = zenko_desktop::theme::theme(&cfg);
        let mut u = f.checkin_ui("Segoe UI");
        u.layout(&fonts);
        check(&u, &t, 0, &c["tree"], "");
        let geo = c["geo"].as_array().unwrap();
        let root = (as_i(&c["root"][0]), as_i(&c["root"][1]));
        let scr = (as_i(&c["screen"][0]), as_i(&c["screen"][1]));
        assert_eq!(f.checkin_pos(root, u.ws[0].w, u.ws[0].h, scr), (as_i(&geo[0]), as_i(&geo[1])), "window position");
        // opening: beep hello, happy for 3 s
        f.events.clear();
        assert!(f.checkin_opening(1000.0, true));
        assert!(f.events.iter().any(|e| matches!(e, Ev::Beep { kind, .. } if kind == "hello")));
        assert_eq!(f.mood_override.as_ref().map(|m| m.0.as_str()), Some("happy"));
        // a click on "Great" gives its command
        let b = u.ws.iter().position(|w| matches!(&w.kind, Kind::Button { text, .. } if text == "Great")).unwrap();
        let (bx, by) = u.abs(b);
        u.press(&fonts, bx + 3, by + 3, 0.0, false);
        assert_eq!(u.release(bx + 3, by + 3).as_deref(), Some("mood:great"));
    }
}

#[test]
fn checkin_save_matches_zenko_py() {
    let _g = CLOCK.lock().unwrap_or_else(|e| e.into_inner());
    let d = std::env::temp_dir().join(format!("zk_checkin_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    unsafe {
        std::env::set_var("APPDATA", &d);
        std::env::set_var("ZK_TZ_OFFSET", "0");
    }
    let g: Value = serde_json::from_str(include_str!("../../../golden/checkin.json")).unwrap();
    let mut f = Companion::new(Cfg::from(json!({})), 1.0, 1000.0);
    zenko_desktop::set_fake_now(Some(g["saves"][0]["now"].as_f64().unwrap()));
    f.life_startup();
    for s in g["saves"].as_array().unwrap() {
        zenko_desktop::set_fake_now(Some(s["now"].as_f64().unwrap()));
        f.events.clear();
        f.checkin_save(1000.0, s["mood"].as_str().unwrap(), s["note"].as_str().unwrap());
        let l = zenko_desktop::read_json("zenko_life.json");
        assert_eq!(l["journal"], s["journal"], "{}", s["mood"]);
        assert_eq!(l["last_checkin"], s["last"]);
        let beeps: Vec<Value> = f.events.iter().filter_map(|e| match e {
            Ev::Beep { kind, .. } => Some(json!(["beep", kind])),
            _ => None,
        }).collect();
        assert_eq!(Value::Array(beeps), s["beeps"]);
        let said: Vec<String> = f.events.iter().filter_map(|e| match e {
            Ev::Say { text, .. } => Some(text.clone()),
            _ => None,
        }).collect();
        // zenko.py picks with random.choice (pinned to the first in the golden run); hers comes from the same list
        let first = s["said"][0][0].as_str().unwrap();
        assert_eq!(said.len(), 1);
        let opts = zenko_desktop::checkin::mood_reply(s["mood"].as_str().unwrap());
        assert!(opts.contains(&first) && opts.contains(&said[0].as_str()), "{said:?}");
        assert_eq!(f.chat_log.last().unwrap().1, said[0]);
        assert_eq!(f.mood_override.as_ref().map(|m| m.0.as_str()), s["moods"][0][0].as_str());
        let awarded = !s["awards"].as_array().unwrap().is_empty();
        if awarded {
            assert!(zenko_desktop::read_json("zenko_life.json")["badges"].get("journal7").is_some(), "journal7 badge");
        }
    }
    zenko_desktop::set_fake_now(None);
}
