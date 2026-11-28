//! The four study-tool windows against zenko.py (golden/tool_windows.json from tools/make_golden_page.py tools): the
//! periodic table, the French dictionary, the CG50 calculator (its keypad, then its screen after every group of key
//! presses through all its modes) and the Bible.

mod common;

use common::{check_items, cmp_tree, fonts_of};
use serde_json::Value;
use zenko_desktop::theme::Theme;
use zenko_desktop::toolwin::{ToolFonts, geometry};
use zenko_desktop::ui::Ui;

fn cases() -> Vec<Value> {
    serde_json::from_str(&std::fs::read_to_string("../../golden/tool_windows.json").unwrap()).unwrap()
}

fn setup(c: &Value) -> (Theme, ToolFonts, common::Table, Ui) {
    let th = zenko_desktop::theme::theme(&zenko_desktop::Cfg::from(serde_json::json!({"theme": c["cfg_all"]["theme"]})));
    let f = ToolFonts { ui: c["font_ui"].as_str().unwrap().into(), serif: c["serif2"].as_str().unwrap().into(),
                        jp: c["mincho"].as_str().unwrap().into() };
    let mut ui = Ui::new(&th.paper);
    ui.x11 = true;
    (th, f, fonts_of(c), ui)
}

fn merge_fonts(c: &mut Value, from: &Value) {
    for (k, v) in from["fonts"].as_object().unwrap() {
        let e = c["fonts"].as_object_mut().unwrap().entry(k.clone()).or_insert_with(|| v.clone());
        for (t, wv) in v["w"].as_object().unwrap() {
            e["w"].as_object_mut().unwrap().entry(t.clone()).or_insert_with(|| wv.clone());
        }
    }
}

fn geo(c: &Value, size: (i32, i32)) -> (i32, i32) {
    let sc = c["screen"].as_array().unwrap();
    let (w, h, x, y) = geometry(size, sc[0].as_i64().unwrap() as i32, sc[1].as_i64().unwrap() as i32);
    assert_eq!(format!("{w}x{h}+{x}+{y}"), c["geo"].as_str().unwrap(), "{} geometry", c["name"]);
    (w, h)
}

fn tree_ok(ui: &Ui, c: &Value, fails: &mut Vec<String>) -> bool {
    let name = c["name"].as_str().unwrap();
    let mut bad = vec![];
    cmp_tree(ui, 0, &c["tree"], 0, 0, name, &mut bad);
    for b in bad.iter().take(30) {
        eprintln!("{b}");
    }
    if !bad.is_empty() {
        fails.push(format!("{name}: {}", bad.len()));
        return false;
    }
    true
}

fn find_canvas<'a>(g: &'a Value, out: &mut Vec<&'a Value>) {
    if g["cls"] == "Canvas" {
        out.push(g);
    }
    for k in g["kids"].as_array().into_iter().flatten() {
        find_canvas(k, out);
    }
}

#[test]
fn periodic_window_matches_zenko_py() {
    use zenko_desktop::ptwin::{PtWin, SIZE};
    let mut fails = vec![];
    let all = cases();
    for c in all.iter().filter(|c| c["name"].as_str().unwrap().starts_with("pt_")) {
        let name = c["name"].as_str().unwrap();
        eprintln!("case {name}");
        // (the colour button keeps the size of its first text: those widths come from the first case)
        let mut c2 = c.clone();
        merge_fonts(&mut c2, &all[0]);
        let (th, f, fonts, mut ui) = setup(&c2);
        let (w, h) = geo(c, SIZE);
        let mut p = PtWin::build(&mut ui, &th, &f, "C", 393);
        if name == "pt_en" {
            p.toggle_mode(&mut ui);
            p.find(&mut ui, "fe");
            p.molar(&mut ui, "Ca(OH)2");
            zenko_desktop::toolwin::set_entry(&mut ui, p.find_e, "fe");
            zenko_desktop::toolwin::set_entry(&mut ui, p.mm_e, "Ca(OH)2");
            p.set_hover(zenko_desktop::ptable::by_sym("Na").map(|e| e.z - 1));
        }
        p.fit(&mut ui, &fonts, w, h);
        if !tree_ok(&ui, c, &mut fails) {
            continue;
        }
        let mut cvs = vec![];
        find_canvas(&c["tree"], &mut cvs);
        let mut imgs = c["images"].clone();
        common::expand_images(&mut imgs);
        check_items(&format!("{name} table"), ui.ws[p.cv].tk_canvas().unwrap(), cvs[1]["items"].as_array().unwrap(), &imgs, &fonts);
    }
    assert!(fails.is_empty(), "{fails:?}");
}

/// a Text box against zenko.py's: its contents and every display line's box
fn check_text(at: &str, ui: &Ui, id: usize, tx: &Value, fonts: &common::Table) {
    let zenko_desktop::ui::Kind::Text { t, .. } = &ui.ws[id].kind else { panic!("{at}: not a text") };
    let want: String = tx["dump"].as_array().unwrap().iter().filter(|d| d[0] == "text").map(|d| d[1].as_str().unwrap()).collect();
    assert_eq!(t.get(), want, "{at}: contents");
    for l in tx["lines"].as_array().unwrap() {
        let (a, b) = l["index"].as_str().unwrap().split_once('.').unwrap();
        let i = (a.parse().unwrap(), b.parse().unwrap());
        let bx: Vec<i32> = l["box"].as_array().unwrap().iter().map(|v| v.as_i64().unwrap() as i32).collect();
        let got = t.dlineinfo(fonts, i).unwrap_or_else(|| panic!("{at} line {i:?} not shown"));
        assert_eq!(got.to_vec(), bx, "{at} line {i:?}");
    }
}

#[test]
fn french_window_matches_zenko_py() {
    use zenko_desktop::frwin::{FrWin, SIZE, look_job};
    let mut fails = vec![];
    for c in cases().iter().filter(|c| c["name"].as_str().unwrap().starts_with("fr_")) {
        let name = c["name"].as_str().unwrap();
        eprintln!("case {name}");
        let (th, f, fonts, mut ui) = setup(c);
        let (w, h) = geo(c, SIZE);
        let mut hist: Vec<String> = if name == "fr_start" {
            ["bonjour", "le chat", "I would like a coffee please, thank you", "manger"].iter().map(|s| s.to_string()).collect()
        } else {
            vec![]
        };
        let mut fw = FrWin::build(&mut ui, &th, &f, &hist);
        let q = match name { "fr_look" => "manger", "fr_off" => "I would like a coffee", _ => "" };
        if !q.is_empty() {
            zenko_desktop::toolwin::set_entry(&mut ui, fw.q, q);
            if name == "fr_off" {
                fw.set_dir(&mut ui, "en");
            }
            let (q, d, fr_in) = fw.look(&mut ui, &mut hist).unwrap();
            let off = name == "fr_off";
            let mut tr = |_q: &str, tgt: &str, src: &str| -> Result<(String, String, String), String> {
                if off { Err("offline".into()) } else { Ok((if tgt == "en" { "to eat" } else { "manger" }.into(), src.into(), "MyMemory".into())) }
            };
            let (res, fr2) = look_job(&q, &d, fr_in, &mut tr);
            fw.look_done(&mut ui, &q, res, fr2);
        }
        fw.fit(&mut ui, &fonts, w, h);
        assert_eq!(serde_json::json!(hist), c["life_all"]["fr_hist"].clone().as_array().cloned().map(Value::Array).unwrap_or(serde_json::json!([])),
                   "{name} history");
        if !tree_ok(&ui, c, &mut fails) {
            continue;
        }
        check_text(name, &ui, fw.out, &c["texts"][0], &fonts);
    }
    assert!(fails.is_empty(), "{fails:?}");
}

#[test]
fn casio_window_matches_zenko_py() {
    use zenko_desktop::casio::{Casio, SIZE};
    let all = cases();
    let c = all.iter().find(|c| c["name"] == "cg").unwrap();
    let (th, f, fonts, mut ui) = setup(c);
    let (w, h) = geo(c, SIZE);
    let cg = serde_json::json!({"deg": true, "mem": {"A": 2.0}, "ans": 0.0, "hist": [], "ys": ["", "", ""], "L1": [], "L2": []});
    let mut k = Casio::build(&mut ui, &th, &f, &cg);
    k.fit(&mut ui, &fonts, w, h);
    let mut imgs = c["images"].clone();
    common::expand_images(&mut imgs);
    let mut fails = vec![];
    // the keypad and the first screen
    let mut bad = vec![];
    cmp_tree(&ui, 0, &c["tree"], 0, 0, "cg", &mut bad);
    for b in bad.iter().take(30) {
        eprintln!("{b}");
    }
    assert!(bad.is_empty(), "cg tree: {}", bad.len());
    for (n, st) in c["steps"].as_array().unwrap().iter().enumerate() {
        let keys: Vec<&str> = st["keys"].as_array().unwrap().iter().map(|v| v.as_str().unwrap()).collect();
        for kid in &keys {
            k.press(&mut ui, kid);
        }
        k.blink = true;
        k.draw(&mut ui);
        let at = format!("step {n} {keys:?}");
        let state = serde_json::json!({"mode": k.mode, "hist": k.hist_json(), "ans": k.ans, "inp": k.inp, "cur": k.cur, "exact": k.exact,
                                      "deg": k.deg, "life_cg": k.life_value()});
        let want = serde_json::json!({"mode": st["mode"], "hist": st["hist"], "ans": st["ans"], "inp": st["inp"], "cur": st["cur"],
                                     "exact": st["exact"], "deg": st["deg"], "life_cg": st["life_cg"]});
        if state != want {
            eprintln!("{at}: state {state}\n   want {want}");
            fails.push(at.clone());
        }
        let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            check_items(&at, ui.ws[k.cv].tk_canvas().unwrap(), st["items"].as_array().unwrap(), &imgs, &fonts);
        }));
        if r.is_err() {
            fails.push(at);
        }
    }
    assert!(fails.is_empty(), "{fails:?}");
}
