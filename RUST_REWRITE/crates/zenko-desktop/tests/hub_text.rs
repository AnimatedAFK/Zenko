//! The chat's Text widget against golden/hub_pages.json: the same contents and tags laid out by tktext must give every
//! display line Tk showed (start, end, dlineinfo box), the boxes of their first and last characters and the yview.
mod common;
use common::{font_key, table};
use serde_json::Value;
use zenko_desktop::tktext::{Justify, TagCfg, Text, Wrap};
use zenko_desktop::ui::Font;

fn font(s: &str) -> Font {
    let (family, size, bold, italic) = font_key(s);
    Font { family, size, bold, italic }
}

fn find<'a>(n: &'a Value, cls: &str) -> Option<&'a Value> {
    if n["cls"] == cls {
        return Some(n);
    }
    n["kids"].as_array().unwrap().iter().find_map(|k| find(k, cls))
}

fn idx(s: &str) -> (usize, usize) {
    let (a, b) = s.split_once('.').unwrap();
    (a.parse().unwrap(), b.parse().unwrap())
}

fn wrap(s: &str) -> Option<Wrap> {
    match s {
        "word" => Some(Wrap::Word),
        "char" => Some(Wrap::Char),
        "none" => Some(Wrap::None),
        _ => None,
    }
}

#[test]
fn chat_text_matches_zenko_py() {
    let g: Value = serde_json::from_str(&std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../../golden/hub_pages.json")).unwrap()).unwrap();
    for c in g.as_array().unwrap() {
        let name = c["name"].as_str().unwrap();
        let fonts = table(c);
        let node = find(&c["tree"], "Text").unwrap();
        let o = &node["opts"];
        let num = |k: &str| o[k].as_str().unwrap().parse::<i32>().unwrap();
        let tx = &c["texts"][0];
        // built with the unscaled font (FONT_UI 11) and filled before it is mapped; zk_fit scales the font after
        let mut t = Text::new(Font::new("Segoe UI", 11, false), o["foreground"].as_str().unwrap(), o["background"].as_str().unwrap());
        (t.padx, t.pady, t.bd, t.hl) = (num("padx"), num("pady"), num("borderwidth"), num("highlightthickness"));
        (t.spacing1, t.spacing2, t.spacing3) = (num("spacing1"), num("spacing2"), num("spacing3"));
        (t.width_ch, t.height_ln) = (num("width"), num("height"));
        t.wrap = wrap(o["wrap"].as_str().unwrap()).unwrap();
        for tg in tx["tags"].as_array().unwrap() {
            let q = &tg["opts"];
            let s = |k: &str| q[k].as_str().unwrap().to_string();
            let n = |k: &str| q[k].as_str().unwrap().parse::<i32>().ok();
            let b = |k: &str| match q[k].as_str().unwrap() {
                "" => None,
                v => Some(v == "1" || v == "true"),
            };
            let cfg = TagCfg {
                name: String::new(),
                font: Some(s("font")).filter(|v| !v.is_empty()).map(|v| font(&v)),
                fg: Some(s("foreground")).filter(|v| !v.is_empty()),
                bg: Some(s("background")).filter(|v| !v.is_empty()),
                spacing1: n("spacing1"),
                spacing2: n("spacing2"),
                spacing3: n("spacing3"),
                lmargin1: n("lmargin1"),
                lmargin2: n("lmargin2"),
                rmargin: n("rmargin"),
                offset: n("offset"),
                underline: b("underline"),
                overstrike: b("overstrike"),
                wrap: wrap(&s("wrap")),
                justify: match s("justify").as_str() {
                    "right" => Some(Justify::Right),
                    "center" => Some(Justify::Center),
                    "left" => Some(Justify::Left),
                    _ => None,
                },
            };
            let name_ = tg["name"].as_str().unwrap();
            *t.tag(name_) = TagCfg { name: name_.into(), ..cfg };
        }
        // the contents, one chat message at a time, each followed by `see end` as _chat_insert does; the widget's
        // history (its size at each `see`, the font changes of zk_fit) is replayed in order
        let hist = tx["hist"].as_array().unwrap();
        let mut hi = 0;
        let step = |t: &mut Text, ev: &Value| {
            let (w, h) = if ev[0] == "see" { (&ev[2], &ev[3]) } else { (&ev[1], &ev[2]) };
            let (w, h) = (w.as_i64().unwrap() as i32, h.as_i64().unwrap() as i32);
            if (w, h) != (t.w, t.h) {
                (t.w, t.h) = (w, h);
                t.relayout(&fonts);
            }
            if ev[0] == "see" {
                let e = t.end_index();
                t.see(&fonts, e);
            } else {
                t.font = font(ev[3].as_str().unwrap());
                t.relayout(&fonts);
            }
        };
        let dump = tx["dump"].as_array().unwrap();
        let mut on: Vec<String> = vec![];
        for (k, e) in dump.iter().enumerate() {
            let (kind, val) = (e[0].as_str().unwrap(), e[1].as_str().unwrap());
            match kind {
                "tagon" => on.push(val.into()),
                "tagoff" => on.retain(|x| x != val),
                "text" => {
                    let last_line = k + 1 == dump.len() || (dump[k + 1][0] == "tagoff" && dump[k + 1][1] == "fox" && val == "\n");
                    if k + 1 == dump.len() && val == "\n" {
                        break; // the widget's own last newline
                    }
                    let tags: Vec<&str> = on.iter().map(|s| s.as_str()).collect();
                    t.insert_end(val, &tags);
                    if last_line {
                        while hi < hist.len() {
                            hi += 1;
                            step(&mut t, &hist[hi - 1]);
                            if hist[hi - 1][0] == "see" {
                                break;
                            }
                        }
                    }
                }
                _ => {}
            }
        }
        let all: String = dump.iter().filter(|e| e[0] == "text").map(|e| e[1].as_str().unwrap()).collect();
        assert_eq!(t.get(), all, "{name} contents");
        for ev in &hist[hi..] {
            step(&mut t, ev);
        }
        assert_eq!(t.font, font(o["font"].as_str().unwrap()), "{name} font");
        assert_eq!(t.req_size(&fonts), (node["rw"].as_i64().unwrap() as i32, node["rh"].as_i64().unwrap() as i32), "{name} req size");
        (t.w, t.h) = (tx["w"].as_i64().unwrap() as i32, tx["h"].as_i64().unwrap() as i32);
        t.relayout(&fonts);
        let want = tx["lines"].as_array().unwrap();
        let got: Vec<_> = t.dlines.iter().filter(|d| t.dlineinfo(&fonts, (d.line, d.ch)).is_some()).collect();
        for (k, l) in want.iter().enumerate() {
            let at = format!("{name} line {}", l["index"]);
            let d = got.get(k).unwrap_or_else(|| panic!("{at}: missing"));
            assert_eq!((d.line, d.ch), idx(l["index"].as_str().unwrap()), "{at} start");
            assert_eq!((d.line, d.ch + d.count - 1), idx(l["end"].as_str().unwrap()), "{at} end");
            let bx: Vec<i32> = l["box"].as_array().unwrap().iter().map(|v| v.as_i64().unwrap() as i32).collect();
            assert_eq!(t.dlineinfo(&fonts, (d.line, d.ch)).unwrap().to_vec(), bx, "{at} dlineinfo");
            for (key, i) in [("first_bbox", (d.line, d.ch)), ("last_bbox", (d.line, d.ch + d.count.max(2) - 2))] {
                let want_b: Vec<i32> = l[key].as_array().unwrap().iter().map(|v| v.as_i64().unwrap() as i32).collect();
                let i = if key == "last_bbox" && d.count == 1 { (d.line, d.ch) } else { i };
                assert_eq!(t.bbox(&fonts, i).map(|b| b.to_vec()).unwrap_or_default(), want_b, "{at} {key} {i:?}");
            }
        }
        assert_eq!(got.len(), want.len(), "{name} line count");
        let yv = t.yview(&fonts);
        let wy = (tx["yview"][0].as_f64().unwrap(), tx["yview"][1].as_f64().unwrap());
        assert!((yv.0 - wy.0).abs() < 1e-12 && (yv.1 - wy.1).abs() < 1e-12, "{name} yview {yv:?} vs {wy:?}");
    }
}
