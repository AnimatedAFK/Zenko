//! The study planner and zenko23's emas that plan themselves against zenko.py (golden/plan.json,
//! tools/make_golden_plan.py): deadlines.json + free time set up, then chat lines and day changes. Every reply, what
//! she says in her bubble (with buttons), the whole timetable, plan_short / ema_short and the deadline ids must match.

use serde_json::{Value, json};
use zenko_desktop::{Cfg, Companion, Ev};
use zenko_sound::mt::PyRandom;

#[test]
fn planner_matches_zenko_py() {
    let g: Vec<Value> =
        serde_json::from_str(&std::fs::read_to_string("../../golden/plan.json").unwrap()).unwrap();
    let d = std::env::temp_dir().join(format!("zk_plan_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(d.join("FoxBuddy")).unwrap();
    unsafe { std::env::set_var("APPDATA", &d) };
    zenko_desktop::set_fake_now(Some(g[0]["now"].as_f64().unwrap()));
    std::fs::write(d.join("FoxBuddy/deadlines.json"), serde_json::to_string_pretty(&g[0]["deadlines"]).unwrap()).unwrap();
    let mut fox = Companion::new(Cfg::from(json!({"robe": "none"})), 1.0, 1000.0);
    fox.life_startup();
    let life0 = g[0]["life"].clone();
    zenko_desktop::test_life_do(&mut fox, |l| {
        for (k, v) in life0.as_object().unwrap() {
            l[k.as_str()] = v.clone();
        }
    });
    let mut bad = vec![];
    for (i, e) in g.iter().enumerate().skip(1) {
        let q = e["q"].as_str().unwrap();
        let now = e["now"].as_f64().unwrap();
        zenko_desktop::set_fake_now(Some(now));
        fox.brain.rng = PyRandom::new(2000 + i as u64 - 1);
        fox.chat_log.clear();
        fox.events.clear();
        if q == "@day" {
            fox.plan_maybe();
        } else {
            let _ = fox.chat_send(now, q, (0, 0));
        }
        fox.pump(now + 0.3);
        fox.pump(now + 1.5);
        let mut got: Vec<Value> = fox.chat_log.iter().map(|(w, t)| json!(["line", w, t])).collect();
        let first: Vec<String> = fox.chat_log.iter().filter(|(w, _)| w == "Fox")
            .map(|(_, t)| t.split('\n').next().unwrap().to_string()).collect();
        for ev in &fox.events {
            match ev {
                Ev::Say { text, choices, .. } if !first.contains(text) && !text.starts_with("Heads up:") => {
                    got.push(json!(["say", text, choices.iter().map(|c| c.0.clone()).collect::<Vec<_>>()]))
                }
                Ev::Command(c) if c.starts_with("open_hub:") => got.insert(1.min(got.len()), json!(["hub", &c[9..]])),
                _ => {}
            }
        }
        let life: Value = serde_json::from_str(&std::fs::read_to_string(d.join("FoxBuddy/zenko_life.json")).unwrap()).unwrap();
        let dl: Value = serde_json::from_str(&std::fs::read_to_string(d.join("FoxBuddy/deadlines.json")).unwrap()).unwrap();
        let ids: Vec<Value> = dl.as_array().unwrap().iter().map(|x| x["id"].clone()).collect();
        let checks = [("out", Value::Array(got.clone()), e["out"].clone()),
                      ("tt", Value::Array(fox.tt.clone()), e["tt"].clone()),
                      ("plan_short", life["plan_short"].clone(), e["plan_short"].clone()),
                      ("ema_short", life["ema_short"].clone(), e["ema_short"].clone()),
                      ("deadline_ids", Value::Array(ids), e["deadline_ids"].clone())];
        for (k, a, b) in checks {
            if a != b {
                bad.push(format!("#{i} {q:?} {k}\n  want {b}\n  got  {a}"));
            }
        }
    }
    zenko_desktop::set_fake_now(None);
    assert!(bad.is_empty(), "{} differ:\n{}", bad.len(), bad.join("\n"));
}
