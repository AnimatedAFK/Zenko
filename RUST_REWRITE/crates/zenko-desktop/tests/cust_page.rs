//! The hub's Customize page against zenko.py's (golden/hub_customize.json): wide in snow, small at midnight, and with
//! most of her look changed (coat, eyes, foxfire, tails, ears, blush, outfits, voice, habits, accent, motion, sidebar
//! order); every widget's place and size, texts and fonts, and every item of every canvas (the live preview's
//! pictures of her pose, the swatches, the accent dots, the buttons).
mod common;
use common::{check_items, cmp_tree, fonts_of};
use serde_json::Value;
use zenko_desktop::pagekit::Faces;
use zenko_desktop::custpage::CustPage;
use serde_json::json;
use zenko_desktop::ui::{Item, Kind, Ui};

fn pairs<'a>(ui: &Ui, id: usize, g: &'a Value, out: &mut Vec<(usize, &'a Value)>) {
    if !g["mapped"].as_bool().unwrap() {
        return;
    }
    if g["cls"] == "Canvas" {
        out.push((id, g));
    }
    let kids = g["kids"].as_array().unwrap();
    if kids.len() == ui.ws[id].kids.len() {
        for (&c, gk) in ui.ws[id].kids.iter().zip(kids) {
            pairs(ui, c, gk, out);
        }
    }
}

fn walk<'a>(g: &'a Value, out: &mut Vec<&'a Value>) {
    out.push(g);
    for k in g["kids"].as_array().into_iter().flatten() {
        walk(k, out);
    }
}

/// a round_frame card or a PixelButton, item by item (type, coords, fill, outline, text)
fn simple_items(ui: &Ui, th: &zenko_desktop::theme::Theme, id: usize, want: &[Value], at: &str) -> Result<(), String> {
    let got = ui.items_of(id, th);
    if got.len() != want.len() {
        return Err(format!("{at}: {} items, want {}", got.len(), want.len()));
    }
    let flat = |p: &[(i32, i32)]| p.iter().flat_map(|&(x, y)| [x as i64, y as i64]).collect::<Vec<_>>();
    for (k, (g, e)) in got.iter().zip(want).enumerate() {
        let (c, ty, fill, outline, text) = match g {
            Item::Poly { pts, fill, outline } => (flat(pts), "polygon", fill.clone().unwrap_or_default(),
                                                  outline.clone().unwrap_or_default(), None),
            Item::Line { pts, color } => (flat(pts), "line", color.clone(), String::new(), None),
            Item::Oval { x1, y1, x2, y2, fill } => (vec![*x1 as i64, *y1 as i64, *x2 as i64, *y2 as i64], "oval", fill.clone(),
                                                    String::new(), None),
            Item::Text { x, y, text, color, .. } => (vec![*x as i64, *y as i64], "text", color.clone(), String::new(),
                                                     Some(text.clone())),
        };
        let ec: Vec<i64> = e["coords"].as_array().unwrap().iter().map(|v| v.as_f64().unwrap().round() as i64).collect();
        let o = &e["opts"];
        if ty != e["type"] || c != ec || fill != o["fill"].as_str().unwrap_or("")
            || (ty == "polygon" && outline != o["outline"].as_str().unwrap_or(""))
            || text.is_some_and(|t| t != o["text"].as_str().unwrap_or("")) {
            return Err(format!("{at} item {k}: {ty} {c:?} {fill} {outline} want {} {ec:?} {}", e["type"], o["fill"]));
        }
    }
    Ok(())
}

#[test]
fn cust_page_matches_zenko_py() {
    let g: Vec<Value> = serde_json::from_str(&std::fs::read_to_string("../../golden/hub_customize.json").unwrap()).unwrap();
    let only = std::env::var("ZK_CASE").unwrap_or_default();
    let mut fails = vec![];
    for c in &g {
        let name = c["name"].as_str().unwrap();
        if !only.is_empty() && only != name {
            continue;
        }
        let tree = &c["tree"];
        let mut all = vec![];
        walk(tree, &mut all);
        let dir = std::env::temp_dir().join(format!("zk_cust_{}_{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("FoxBuddy")).unwrap();
        unsafe { std::env::set_var("APPDATA", &dir) };
        std::fs::write(dir.join("FoxBuddy").join("zenko_life.json"), c["life_all"].to_string()).unwrap();
        let now = c["now"].as_f64().unwrap();
        zenko_desktop::set_fake_now(Some(now));
        let cfg = c["cfg_full"].clone();
        let mut fox = zenko_desktop::Companion::new(zenko_desktop::Cfg::from(cfg.clone()), 1.0, now);
        let ps = &c["pose"];
        let i = |k: &str| ps[k].as_i64().unwrap_or(0) as i32;
        let lk = ps["look"].as_array().unwrap();
        let l = |k: usize| lk[k].as_i64().unwrap() as i32;
        // (cz_anim draws every 70 ms and the pose was read after its last draw: in cust_alt she had since begun a blink,
        // her tails had swung on a frame and an ear twitch had ended; the picture is the tick before, which is checked
        // pixel for pixel)
        let late = name == "cust_alt";
        let state = match ps["state"].as_str().unwrap() { "blink" if late => "idle", s => s };
        fox.pose = Some(zenko_desktop::play::Pose {
            state: state.into(), hx: i("hx"), ear: if late { 1 } else { i("ear") }, look: (l(0), l(1), l(2), l(3)),
            frame: i("frame"), acc: ps["acc"].as_array().unwrap().iter().map(|a| a.as_str().unwrap().to_string()).collect(),
            curled: ps["curled"].as_bool().unwrap(),
            tframe: i("tframe") - late as i32, n: i("n"), dxf: i("dxf"), dyf: i("dyf"),
            hide_fire: false });
        let mut d = fox.cust_data();
        if std::env::var("ZK_DUMP").is_ok()
            && let Some(pv) = d.preview.as_ref() {
            let px: Vec<String> = pv.body.px.iter().map(|q| format!("{:02x}{:02x}{:02x}{:02x}", q[0], q[1], q[2], q[3])).collect();
            std::fs::write(format!("/tmp/claude-0/rs_{name}.json"), serde_json::to_string(&json!({"w": pv.body.w, "px": px})).unwrap()).unwrap();
        }
        d.fox_name = c["fox_name"].as_str().unwrap().into();
        d.catch = c["catch"].as_str().unwrap().into();
        d.bond = (c["bond"][0].as_str().unwrap().into(), c["bond"][1].as_str().unwrap().into());
        d.hours = c["z19_hours"].as_f64().unwrap();
        // (the recording set the accent in the config after the theme was applied, so the page is in the theme's own
        // accent; the app applies the accent with the theme)
        let mut tcfg = cfg.clone();
        tcfg.as_object_mut().unwrap().remove("accent");
        let th = zenko_desktop::theme::theme(&zenko_desktop::Cfg::from(tcfg));
        let _ = json!({});
        let mut fonts = fonts_of(c);
        let small = name.contains("small");
        if small {
            for (k, v) in fonts_of(&g[0]).0 {
                let e = fonts.0.entry(k).or_insert_with(|| v.clone());
                for (s, n) in v.2 {
                    e.2.entry(s).or_insert(n);
                }
                for (ch, n) in v.3 {
                    e.3.entry(ch).or_insert(n);
                }
            }
        }
        let (pw, ph) = (tree["w"].as_i64().unwrap() as i32, tree["h"].as_i64().unwrap() as i32);
        let (ox, oy) = (tree["x"].as_i64().unwrap() as i32, tree["y"].as_i64().unwrap() as i32);
        let cm10 = tree["kids"][0]["rw"].as_i64().unwrap() as i32;
        let faces = Faces { ui: c["font_ui"].as_str().unwrap().into(), title: String::new() };
        let zk_f = c["zk_f"].as_f64().unwrap();
        let mut ui = Ui::new(&th.paper);
        ui.x11 = true;
        let mut p = CustPage::build(&mut ui, 0, &th, &faces, cm10, &d);
        // (the window's own fit when it opened, then the harness's zk_fit on the settled page: the second one's
        // z21_unwrap lets the labels the first pass wrapped while the columns were side by side go again)
        if small {
            // (the window opened at the usual size first: its first fit ran there, with that size's fonts)
            let t0 = &g[0]["tree"];
            p.fit(&mut ui, &fonts, &th, &faces, g[0]["zk_f"].as_f64().unwrap(), t0["w"].as_i64().unwrap() as i32,
                  t0["h"].as_i64().unwrap() as i32);
        }
        p.fit(&mut ui, &fonts, &th, &faces, zk_f, pw, ph);
        p.fit(&mut ui, &fonts, &th, &faces, zk_f, pw, ph);
        let mut bad = vec![];
        cmp_tree(&ui, 0, tree, ox, oy, name, &mut bad);
        let mut ps = vec![];
        pairs(&ui, 0, tree, &mut ps);
        let mut imgs = c["images"].clone();
        imgs["__fonts"] = c["fonts"].clone();
        // (all but the scroller's own canvas, which holds only the page, and its bar)
        let mut seen = 0;
        for (k, (id, gn)) in ps.iter().enumerate() {
            if matches!(ui.ws[*id].kind, Kind::Button { .. } | Kind::Canvas { card: Some(_), .. }) {
                seen += 1;
                if let Err(e) = simple_items(&ui, &th, *id, gn["items"].as_array().unwrap(), &format!("{name} canvas {k}")) {
                    bad.push(e);
                }
                continue;
            }
            if let Kind::Canvas { tk: Some(cv), .. } = &ui.ws[*id].kind
                && gn["items"].as_array().is_some_and(|a| !a.is_empty())
                && !(gn["w"] == 10 && gn["items"].as_array().unwrap().len() == 4) {
                let want = gn["items"].as_array().unwrap().clone();
                seen += 1;
                let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    check_items(&format!("{name} canvas {k}"), cv, &want, &imgs, &fonts)
                }));
                if r.is_err() {
                    bad.push(format!("{name}: canvas {k} ({}x{} at {},{})", gn["w"], gn["h"], gn["x"], gn["y"]));
                }
            }
        }
        eprintln!("{name}: {seen} canvases checked");
        assert!(seen >= 78, "{name}: only {seen} canvases were compared");
        for b in &bad {
            eprintln!("{b}");
        }
        if !bad.is_empty() {
            fails.push(format!("{name}: {}", bad.len()));
        }
    }
    zenko_desktop::set_fake_now(None);
    assert!(fails.is_empty(), "{fails:?}");
}
