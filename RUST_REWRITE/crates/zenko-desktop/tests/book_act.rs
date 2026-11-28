//! Using an open book against zenko.py (golden/hub_bookact.json): the practice corners' buttons pressed and answers
//! typed in chemistry, maths, biology, French, psychology, the grades book and the study diary, with random seeded
//! before each step. After every step: every label's (and button's) text, every entry's text, and what the life file
//! (book_state, fr_vocab, grades, goals, the flashcard deck) and the settings hold.
use serde_json::{Value, json};
use zenko_desktop::bookpage::{Act, BookPage};
use zenko_desktop::ui::{Kind, Ui};
use zenko_sound::mt::PyRandom;

fn walk(ui: &Ui, id: usize, f: &mut dyn FnMut(usize)) {
    for &k in &ui.ws[id].kids {
        f(k);
        walk(ui, k, f);
    }
}

fn labels(ui: &Ui, host: usize) -> Vec<String> {
    let mut v = vec![];
    walk(ui, host, &mut |k| match &ui.ws[k].kind {
        Kind::Label { text, .. } | Kind::Check { text, .. } | Kind::Menu { text, .. } => v.push(text.clone()),
        Kind::Button { text, .. } => v.push(format!("[{text}]")),
        _ => {}
    });
    v
}

fn entries(ui: &Ui, host: usize) -> Vec<usize> {
    let mut v = vec![];
    walk(ui, host, &mut |k| if matches!(ui.ws[k].kind, Kind::Entry { .. }) { v.push(k) });
    v
}

fn no_ids(v: &Value) -> Value {
    let mut v = v.clone();
    for c in v.as_array_mut().into_iter().flatten() {
        if let Some(o) = c.as_object_mut() {
            o.remove("id");
        }
    }
    v
}

#[test]
fn books_work_like_zenko_py() {
    let g: Vec<Value> = serde_json::from_str(&std::fs::read_to_string("../../golden/hub_bookact.json").unwrap()).unwrap();
    let dir = std::env::temp_dir().join(format!("zk_bact_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("FoxBuddy")).unwrap();
    unsafe { std::env::set_var("APPDATA", &dir) };
    let only = std::env::var("ZK_CASE").unwrap_or_default();
    let mut fails = vec![];
    for c in &g {
        let name = c["name"].as_str().unwrap();
        if !only.is_empty() && only != name {
            continue;
        }
        let key = name.strip_prefix("act_").unwrap();
        let now = c["now"].as_f64().unwrap();
        zenko_desktop::set_fake_now(Some(now));
        std::fs::write(dir.join("FoxBuddy").join("zenko_life.json"), c["life_before"].to_string()).unwrap();
        let mut fox = zenko_desktop::Companion::new(zenko_desktop::Cfg::from(c["cfg_before"].clone()), 1.0, now);
        let th = zenko_desktop::theme::theme(&zenko_desktop::Cfg::from(json!({"theme": "snow"})));
        let mk = |fox: &mut zenko_desktop::Companion, ui: &mut Ui, rnd: PyRandom| {
            let mut d = fox.book_data(key);
            d.jp = c["mincho"].as_str().unwrap().into();
            d.serif = c["serif_font"].as_str().unwrap().into();
            d.jpg = c["jp_font"].as_str().filter(|s| !s.is_empty()).unwrap_or("Segoe UI").into();
            d.now = now;
            let mut p = BookPage::build(ui, 0, &th, "Segoe UI", 378, d, rnd);
            p.brains_done(ui, &[]);
            p
        };
        let mut ui = Ui::new(&th.paper);
        let mut p = mk(&mut fox, &mut ui, PyRandom::new(77));
        let mut cfg = c["cfg_before"].clone();
        for (i, st) in c["steps"].as_array().unwrap().iter().enumerate() {
            let s = &st["step"];
            p.rnd = PyRandom::new(100 + i as u64);
            let host = p.host;
            let acts = match s[0].as_str().unwrap() {
                "entry" | "ret" => {
                    let id = entries(&ui, host)[s[1].as_u64().unwrap() as usize];
                    if s[0] == "entry" {
                        if let Kind::Entry { e, .. } = &mut ui.ws[id].kind {
                            e.text = s[2].as_str().unwrap().encode_utf16().collect();
                            e.pos = e.text.len();
                        }
                    }
                    if s[0] == "ret" || !matches!(key, "grades" | "diary") {
                        p.key_return(&mut ui, id).unwrap_or_default()
                    } else {
                        vec![]
                    }
                }
                _ => {
                    let want = s[1].as_str().unwrap();
                    let cmd = ui.ws.iter().find_map(|w| match &w.kind {
                        Kind::Button { text, cmd, .. } if text == want => Some(cmd.clone()),
                        _ => None,
                    }).unwrap_or_else(|| panic!("{name}: no button {want}"));
                    p.command(&mut ui, &cmd)
                }
            };
            let mut render = false;
            for a in acts {
                match a {
                    Act::Save => {
                        let parts = p.life.clone();
                        fox.life_do(|l| {
                            for k in ["book_state", "fr_vocab", "goals", "grades"] {
                                if !parts[k].is_null() {
                                    l[k] = parts[k].clone();
                                }
                            }
                        });
                    }
                    Act::Cfg(k, v) => {
                        fox.cfg_set(&k, v.clone());
                        cfg[k.as_str()] = v;
                    }
                    Act::Run(r) if r.starts_with("nb_mistake\t") => {
                        let q: Vec<&str> = r.split('\t').collect();
                        fox.nb_mistake(now, q[1], q[2], q[3]);
                        p.life["cards"] = fox.life_now()["cards"].clone();
                    }
                    Act::Render => render = true,
                    _ => {}
                }
            }
            if render {
                let kids = ui.ws[0].kids.clone();
                for k in kids {
                    ui.destroy(k);
                }
                let rnd = std::mem::replace(&mut p.rnd, PyRandom::new(0));
                p = mk(&mut fox, &mut ui, rnd);
            }
            let at = format!("{name} step {i} {s}");
            let gl: Vec<String> = st["labels"].as_array().unwrap().iter().map(|v| v.as_str().unwrap().to_string()).collect();
            let rl = labels(&ui, p.host);
            if gl != rl {
                let k = gl.iter().zip(&rl).position(|(a, b)| a != b).unwrap_or(gl.len().min(rl.len()));
                fails.push(format!("{at}: labels differ at {k}: {:?} vs {:?} ({} vs {})", gl.get(k), rl.get(k), gl.len(), rl.len()));
            }
            let ge: Vec<String> = st["entries"].as_array().unwrap().iter().map(|v| v.as_str().unwrap().to_string()).collect();
            let re: Vec<String> = entries(&ui, p.host).iter().map(|&e| match &ui.ws[e].kind {
                Kind::Entry { e, .. } => String::from_utf16_lossy(&e.text),
                _ => String::new(),
            }).collect();
            if ge != re {
                fails.push(format!("{at}: entries {ge:?} vs {re:?}"));
            }
            let life = fox.life_now();
            for k in ["book_state", "fr_vocab", "grades", "goals", "cards"] {
                let mine = if k == "cards" { p.life[k].clone() } else if p.life[k].is_null() { life[k].clone() } else { p.life[k].clone() };
                let (a, b) = if matches!(k, "cards" | "grades") { (no_ids(&st["life"][k]), no_ids(&mine)) } else { (st["life"][k].clone(), mine) };
                let a = if a.is_null() { json!({}) } else { a };
                let b = if b.is_null() { json!({}) } else { b };
                if a != b && !(a.as_array().is_some_and(|x| x.is_empty()) && b.as_object().is_some_and(|x| x.is_empty())) {
                    fails.push(format!("{at}: life {k}: {a} vs {b}"));
                }
            }
            for (k, v) in st["cfg"].as_object().unwrap() {
                if !v.is_null() && cfg[k.as_str()] != *v {
                    fails.push(format!("{at}: cfg {k}: {v} vs {}", cfg[k.as_str()]));
                }
            }
        }
    }
    for f in &fails {
        eprintln!("{f}");
    }
    assert!(fails.is_empty(), "{} differences", fails.len());
}
