//! The hub's Exercise page in the window (zenko.py ZenkoGym's taps and buttons): the day tabs, which day today is,
//! ticking muscle groups, Finish, water cups, sleep moons, logging a weight, height and sex, adding food (the built-in
//! table; online lookups come with the brains stage), removing a food line, fixing its grams, and the animations.

use super::*;
use serde_json::{Value, json};
use zenko_desktop::gympage::{GymData, GymPage, WATER_GOAL};

fn cm10(x: &HubW) -> i32 {
    (100.0 * x.dpi / 25.4 + 0.5) as i32
}

fn data(x: &HubW) -> GymData {
    let (mincho, serif, _) = hubbook::fonts_pub(&x.hub.font_ui);
    crate::win::with_fox(|fox, _| {
        let gym = fox.gym_do(|g| g.clone());
        let secs = zenko_desktop::unix_now().floor() as i64 + zenko_desktop::local_offset_pub();
        let today = secs.div_euclid(86400);
        GymData { gym, today_iso: zenko_desktop::plan::iso(today), today, jp: mincho.clone(), serif: serif.clone() }
    }).unwrap_or_default()
}

pub(super) fn build(x: &mut HubW, pg: usize) {
    let (th, f) = (x.hub.t.clone(), faces(x));
    let d = data(x);
    let (c, n) = (cm10(x), now(x));
    x.gym = Some(GymPage::build(&mut x.hub.ui, pg, &th, &f, c, &d, n));
}

pub(super) fn fit(x: &mut HubW, w: i32, h: i32) {
    let (th, f) = (x.hub.t.clone(), faces(x));
    let Some(p) = x.gym.as_mut() else { return };
    p.fit(&mut x.hub.ui, &x.fonts, &th, &f, x.hub.zk_f, w, h);
    let sc = p.sc.clone();
    sc.draw_bar(&mut x.hub.ui, false, &th);
}

/// gym_render()
pub(super) fn render(x: &mut HubW) {
    let d = data(x);
    let (c, n) = (cm10(x), now(x));
    let Some(p) = x.gym.as_mut() else { return };
    p.render(&mut x.hub.ui, c, &d, n);
    relayout(x);
}

/// the looping animations while the page shows
pub(super) fn step(x: &mut HubW, n: f64) -> bool {
    if x.hub.side.page.as_deref() != Some("exercise") {
        return false;
    }
    let Some(p) = x.gym.as_mut() else { return false };
    p.step(&mut x.hub.ui, n)
}

/// a click on a label, cup, moon or seal of the page
pub(super) fn click(x: &mut HubW, mx: i32, my: i32) -> bool {
    if x.hub.side.page.as_deref() != Some("exercise") {
        return false;
    }
    let Some(c) = x.gym.as_ref().and_then(|p| p.click_at(&x.hub.ui, mx, my)) else { return false };
    cmd(x, &c);
    true
}

pub(super) fn key_return(x: &mut HubW) -> bool {
    let Some(p) = x.gym.as_ref() else { return false };
    let f = x.hub.ui.focus;
    let c = if f == Some(p.wentry) { "gym_wlog" } else if f == Some(p.hentry) { "gym_prof" } else if f == Some(p.fentry) {
        "gym_food_add" } else { return false };
    cmd(x, c);
    true
}

/// the height box lost the focus: saved when it changed
pub(super) fn focus_left(x: &mut HubW, old: Option<usize>) {
    let Some(p) = x.gym.as_ref() else { return };
    if old.is_none() || old != Some(p.hentry) || x.hub.ui.focus == old {
        return;
    }
    let v = x.hub.ui.text_of(p.hentry).trim().to_string();
    let cur = data(x).gym["profile"]["height"].clone();
    let cur = match &cur {
        Value::Number(n) => n.as_i64().map(|i| i.to_string()).unwrap_or_else(|| {
            let f = n.as_f64().unwrap_or(0.0);
            if f.fract() == 0.0 { format!("{f:.1}") } else { format!("{f}") }
        }),
        Value::String(s) => s.clone(),
        _ => String::new(),
    };
    if cur != v {
        cmd(x, "gym_prof");
    }
}

fn say(t: String, secs: f64) {
    crate::win::with_fox(move |fox, _| fox.say(&t, secs));
}

fn gym_do(f: impl FnOnce(&mut Value)) {
    crate::win::with_fox(move |fox, _| fox.gym_do(f));
}

fn day_name(i: usize) -> (String, String) {
    let d = &zenko_desktop::bodyart::gym_data()["GYM_ALL"][i];
    (d["kanji"].as_str().unwrap_or("").into(), d["name"].as_str().unwrap_or("").to_lowercase())
}

/// save_prof(): the height (120..230 cm, else none) and the sex radio
fn save_prof(x: &mut HubW, sex: Option<&str>) {
    let Some(p) = x.gym.as_ref() else { return };
    let hv = x.hub.ui.text_of(p.hentry).trim().to_string();
    let hcm: f64 = if hv.is_empty() { 0.0 } else { hv.parse().unwrap_or(0.0) };
    let sx = sex.map(String::from).unwrap_or_else(|| {
        p.radios.iter().find(|r| matches!(x.hub.ui.ws[r.0].kind, Kind::Check { on: true, .. })).map(|r| r.1.to_string())
            .unwrap_or_default()
    });
    gym_do(move |g| {
        g["profile"]["height"] = if (120.0..=230.0).contains(&hcm) { json!(hcm) } else { Value::Null };
        g["profile"]["sex"] = json!(sx);
    });
    render(x);
}

pub(super) fn cmd(x: &mut HubW, cmd: &str) {
    let today = data(x).today_iso;
    let a: Vec<&str> = cmd.splitn(3, ':').collect();
    let num = |i: usize| a.get(i).and_then(|v| v.parse::<i64>().ok()).unwrap_or(0);
    match a[0] {
        "gym_view" => {
            if let Some(p) = x.gym.as_mut() {
                p.view = num(1) as usize;
            }
            render(x);
        }
        "gym_set" => {
            // gym_set_today(i)
            let i = num(1).rem_euclid(4);
            gym_do(move |g| g["next"] = json!(i));
            if let Some(p) = x.gym.as_mut() {
                p.view = i as usize;
            }
            let (kanji, name) = day_name(i as usize);
            say(if i == 3 { "Kon! A rest day~ Stretch, walk, sleep well.".into() } else {
                format!("Kon! Today is {kanji}\u{306e}\u{65e5}: {name}. Ganbatte~") }, 6.0);
            render(x);
        }
        "gym_tog" => {
            let i = num(1);
            let name = a.get(2).copied().unwrap_or("").to_string();
            let key = format!("{today}|{i}");
            let n_groups = zenko_desktop::bodyart::gym_data()["GYM_ALL"][i as usize]["groups"].as_array().map(|g| g.len()).unwrap_or(0);
            let all = crate::win::with_fox(move |fox, t| {
                let n = fox.gym_do(|g| {
                    let mut cur: Vec<String> = g["checks"][key.as_str()].as_array().map(|a| a.iter()
                        .filter_map(|v| v.as_str().map(String::from)).collect()).unwrap_or_default();
                    if let Some(p) = cur.iter().position(|c| *c == name) {
                        cur.remove(p);
                    } else {
                        cur.push(name.clone());
                    }
                    cur.sort();
                    cur.dedup();
                    let n = cur.len();
                    g["checks"][key.as_str()] = json!(cur);
                    if let Some(o) = g["checks"].as_object_mut() {
                        let ks: Vec<String> = o.keys().cloned().collect();
                        for k in &ks[..ks.len().saturating_sub(30)] {
                            o.remove(k);
                        }
                    }
                    n
                });
                if n == n_groups {
                    fox.say(&format!("Kon! Every muscle for day {} trained~ Otsukare!", i + 1), 6.0);
                    fox.set_mood(t, "happy", 3.0);
                }
            });
            let _ = all;
            render(x);
        }
        "gym_water" => {
            let n = num(1);
            let t = today.clone();
            let full = crate::win::with_fox(move |fox, _| fox.gym_do(|g| {
                let cur = g["water"][t.as_str()].as_i64().unwrap_or(0);
                let v = if cur == n + 1 { n } else { n + 1 };
                g["water"][t.as_str()] = json!(v);
                trim_days(&mut g["water"], 60);
                v
            })).unwrap_or(0);
            if full == WATER_GOAL {
                say(format!("Kon! All {WATER_GOAL} glasses~ Well hydrated!"), 5.0);
            }
            render(x);
        }
        "gym_sleep" => {
            let n = num(1);
            let t = today.clone();
            gym_do(move |g| {
                g["sleep"][t.as_str()] = json!(n);
                trim_days(&mut g["sleep"], 60);
            });
            if n <= 2 {
                say("Rough night... go easy today, and try to get to bed a little earlier~".into(), 7.0);
            }
            render(x);
        }
        "gym_finish" => {
            let i = num(1);
            let t = today.clone();
            gym_do(move |g| {
                let mut log: Vec<Value> = g["log"].as_array().cloned().unwrap_or_default();
                log.retain(|e| !(e["date"].as_str() == Some(t.as_str()) && e["day"].as_i64() == Some(i)));
                log.push(json!({"date": t, "day": i}));
                let s = log.len().saturating_sub(200);
                g["log"] = json!(log[s..].to_vec());
                g["next"] = json!((i + 1) % 4);
            });
            let next = ((i + 1) % 4) as usize;
            if let Some(p) = x.gym.as_mut() {
                p.view = next;
            }
            let msg = if i == 3 { format!("Well rested~ Next time: {}.", day_name(0).1) } else {
                format!("Otsukare! Day {} done~ Next time: {}.", i + 1, if next == 3 { "a rest day".into() } else { day_name(next).1 }) };
            crate::win::with_fox(move |fox, t| {
                fox.set_mood(t, "happy", 4.0);
                fox.say(&msg, 8.0);
                fox.beep_play("hello");
            });
            render(x);
        }
        "gym_wlog" => {
            let Some(p) = x.gym.as_ref() else { return };
            let (e, msg) = (p.wentry, p.wmsg);
            let t = x.hub.ui.text_of(e).trim().to_lowercase().replace(',', ".");
            let re = regex::Regex::new(r"^(\d{2,3}(?:\.\d)?)\s*(kg|lbs?|pounds)?$").unwrap();
            let err = match re.captures(&t) {
                None => Some("Type your weight like 68.5 (or 150 lb)"),
                Some(m) => {
                    let unit = m.get(2).map(|u| u.as_str()).unwrap_or("");
                    let kg = m[1].parse::<f64>().unwrap() * if unit.starts_with("lb") || unit.starts_with("pound") { 0.4536 } else { 1.0 };
                    if !(30.0..=250.0).contains(&kg) {
                        Some("That doesn't look right. Try again?")
                    } else {
                        let d = today.clone();
                        // gym_weight_add(kg): one weigh-in a day, sorted, the last 400
                        gym_do(move |g| {
                            let mut w: Vec<Value> = g["weights"].as_array().cloned().unwrap_or_default();
                            w.retain(|x| x["date"].as_str() != Some(d.as_str()));
                            w.push(json!({"date": d, "kg": format!("{kg:.1}").parse::<f64>().unwrap()}));
                            w.sort_by(|a, b| a["date"].as_str().cmp(&b["date"].as_str()));
                            let s = w.len().saturating_sub(400);
                            g["weights"] = json!(w[s..].to_vec());
                        });
                        None
                    }
                }
            };
            match err {
                Some(m) => {
                    if let Kind::Label { text, .. } = &mut x.hub.ui.ws[msg].kind {
                        *text = m.into();
                    }
                    relayout(x);
                }
                None => render(x),
            }
        }
        "gym_prof" => save_prof(x, None),
        "gym_sex" => {
            let v = a.get(1).copied().unwrap_or("").to_string();
            save_prof(x, Some(&v));
        }
        "gym_food_add" => {
            let Some(p) = x.gym.as_ref() else { return };
            let (e, st) = (p.fentry, p.fstatus);
            let t = x.hub.ui.text_of(e).trim().to_string();
            if t.is_empty() {
                return;
            }
            if let Kind::Entry { e, .. } = &mut x.hub.ui.ws[e].kind {
                (e.text, e.pos, e.anchor, e.sel, e.left) = (vec![], 0, 0, None, 0);
            }
            let (items, unknown) = zenko_desktop::gym::food_estimate(&t);
            let d = today.clone();
            gym_do(move |g| {
                if !g["food"][d.as_str()].is_array() {
                    g["food"][d.as_str()] = json!([]);
                }
                for (lab, k, pr) in items {
                    g["food"][d.as_str()].as_array_mut().unwrap().push(json!({"t": lab, "kcal": k, "p": pr, "src": "table"}));
                }
                trim_days(&mut g["food"], 60);
            });
            if unknown.is_empty() {
                render(x);
                return;
            }
            // (the online lookup comes with the brains and web stage: what the table doesn't know is reported now)
            let short: Vec<String> = unknown.iter().map(|u| zenko_desktop::extra::clip(u, 24)).collect();
            if let Kind::Label { text, .. } = &mut x.hub.ui.ws[st].kind {
                *text = format!("Estimating {} with your smart brain...", short.join(", "));
            }
            let miss: Vec<String> = unknown.iter().map(|u| zenko_desktop::extra::clip(u, 30)).collect();
            render(x);
            say(format!("I couldn't find {}. Try a simpler name with a weight, like '150 g rice', or tell me the calories: \
'protein bar 210 kcal 20 g protein'.", miss.join(", ")), 8.0);
        }
        "gym_food_rm" => {
            let i = num(1) as usize;
            let d = today.clone();
            gym_do(move |g| {
                if let Some(a) = g["food"][d.as_str()].as_array_mut() {
                    if i < a.len() {
                        a.remove(i);
                    }
                }
            });
            render(x);
        }
        "gym_fix" => {
            // food_fix_grams(it): ask how much it was
            let i = num(1) as usize;
            let it = data(x).gym["food"][today.as_str()][i].clone();
            let t = it["t"].as_str().unwrap_or("").to_string();
            let g0 = it["g"].as_f64().filter(|g| *g != 0.0).or_else(|| {
                regex::Regex::new(r"\((\d+)g").unwrap().captures(&t).and_then(|m| m[1].parse::<f64>().ok())
            });
            let q = format!("How many grams (or ml) was \u{201c}{}\u{201d}?{}", zenko_desktop::extra::clip(&t, 50),
                            g0.map(|g| format!("\nNow: {} g", g as i64)).unwrap_or_default());
            out(&format!("run:ask_string\tgym_fix:{i}\tHow much?\t{q}"));
        }
        _ => {}
    }
}

/// for k_ in sorted(D)[:-n]: D.pop(k_)
fn trim_days(v: &mut Value, n: usize) {
    if let Some(o) = v.as_object_mut() {
        let mut ks: Vec<String> = o.keys().cloned().collect();
        ks.sort();
        for k in &ks[..ks.len().saturating_sub(n)] {
            o.remove(k);
        }
    }
}

/// food_fix_grams' answer: the line's numbers follow the new amount
pub(super) fn fix_grams(x: &mut HubW, i: usize, ans: &str) {
    let today = data(x).today_iso;
    let first = zenko_desktop::gym::clean_num(ans).trim().split_whitespace().next().unwrap_or("").to_string();
    let Ok(g1) = first.trim_end_matches(['g', 'm', 'l']).parse::<f64>() else { return };
    if !(1.0..=3000.0).contains(&g1) {
        return;
    }
    gym_do(move |g| {
        let it = &mut g["food"][today.as_str()][i];
        if !it.is_object() {
            return;
        }
        let t = it["t"].as_str().unwrap_or("").to_string();
        let g0 = it["g"].as_f64().filter(|g| *g != 0.0).or_else(|| {
            regex::Regex::new(r"\((\d+)g").unwrap().captures(&t).and_then(|m| m[1].parse::<f64>().ok())
        });
        let (k, p) = if let (Some(k100), Some(p100)) = (it["k100"].as_f64(), it["p100"].as_f64()) {
            (k100 * g1 / 100.0, p100 * g1 / 100.0)
        } else if let Some(g0) = g0 {
            (it["kcal"].as_f64().unwrap_or(0.0) * g1 / g0, it["p"].as_f64().unwrap_or(0.0) * g1 / g0)
        } else {
            let Some((e, _)) = zenko_desktop::gym::food_find(&t) else { return };
            let fr = zenko_desktop::gym::food_row(e);
            (fr.kcal * g1 / fr.g, fr.prot * g1 / fr.g)
        };
        let (k, p) = zenko_desktop::gym::food_sane(k, p);
        it["kcal"] = json!(k);
        it["p"] = json!(p);
        it["g"] = json!(g1.round_ties_even() as i64);
        let base = regex::Regex::new(r"\s*\([^)]*\)\s*$").unwrap().replace(&t, "").to_string();
        let tail = if t.contains(" \u{b7} ") {
            format!(" \u{b7} {}", t.rsplit(" \u{b7} ").next().unwrap_or("").trim_end_matches(')'))
        } else { String::new() };
        it["t"] = json!(format!("{base} ({}g{tail})", g1 as i64));
    });
    render(x);
}
