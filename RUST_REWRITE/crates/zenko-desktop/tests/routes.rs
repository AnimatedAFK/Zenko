//! zenko.py's later version chat routes (golden/routes.json, tools/make_golden_routes.py): Zenko22, z17, z15, ib_route,
//! the garden and delay picker. Every reply (fox voice included), what she opens, the IB tasks, close/keep rules and
//! the screen-time pause must match.

use serde_json::{Value, json};
use zenko_desktop::{Cfg, Companion, Ev};
use zenko_sound::mt::PyRandom;

#[test]
fn routes_match_zenko_py() {
    let g: Vec<Value> =
        serde_json::from_str(&std::fs::read_to_string("../../golden/routes.json").unwrap()).unwrap();
    let d = std::env::temp_dir().join(format!("zk_routes_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(d.join("FoxBuddy")).unwrap();
    unsafe { std::env::set_var("APPDATA", &d) };
    zenko_desktop::set_fake_now(Some(g[0]["now"].as_f64().unwrap()));
    std::fs::write(d.join("FoxBuddy/stats.json"), g[0]["stats"].to_string()).unwrap();
    std::fs::write(d.join("FoxBuddy/browser_now.json"), g[0]["browser"].to_string()).unwrap();
    let mut fox = Companion::new(Cfg::from(json!({"robe": "none"})), 1.0, 1000.0);
    fox.life_startup();
    fox.rewards.next_tick = Some(f64::MAX); // the 3-minute badge tick (stubbed on the Python side too)
    let life0 = g[0]["life"].clone();
    zenko_desktop::test_life_do(&mut fox, |l| {
        for k in ["screen", "garden", "badges"] {
            l[k] = life0[k].clone();
        }
        l["fishing"]["book"] = life0["fishing_book"].clone();
    });
    let mut bad = vec![];
    // zenko.py makes the gym dict on some early tick; it's compared once the Exercise lines start
    let gym_from = g.iter().position(|e| e["q"] == "i ate 2 eggs and toast").unwrap(); // (and fills today's food list)
    for (i, e) in g.iter().enumerate().skip(1) {
        let q = e["q"].as_str().unwrap();
        let now = e["now"].as_f64().unwrap();
        zenko_desktop::set_fake_now(Some(now));
        fox.brain.rng = PyRandom::new(3000 + i as u64 - 1);
        fox.chat_log.clear();
        fox.events.clear();
        let r = fox.chat_send(now, q, (0, 0));
        fox.pump(now + 0.7);
        let mut got: Vec<Value> = fox.chat_log.iter().map(|(w, t)| json!(["line", w, t])).collect();
        if let zenko_desktop::chat::Sent::Brain(b) = r {
            got.push(json!(["llm", b]));
        }
        let first: Vec<String> = fox.chat_log.iter().filter(|(w, _)| w == "Fox")
            .map(|(_, t)| t.split('\n').next().unwrap().to_string()).collect();
        let mut cmds = vec![];
        for ev in &fox.events {
            match ev {
                Ev::Say { text, choices, .. } if !first.contains(text) => {
                    got.push(json!(["say", text, choices.iter().map(|c| c.0.clone()).collect::<Vec<_>>()]))
                }
                Ev::Command(c) => cmds.push(json!(c)),
                _ => {}
            }
        }
        let life: Value = serde_json::from_str(&std::fs::read_to_string(d.join("FoxBuddy/zenko_life.json")).unwrap()).unwrap();
        let ib: Vec<Value> = life["ibtasks"].as_array().cloned().unwrap_or_default().iter()
            .map(|t| json!([t["kind"], t["subject"], t["unit"], t["target"], t["words"], t["title"], t["due"], t["hist"]])).collect();
        let rules = fox.cfg.raw.get("zk_rules").cloned().unwrap_or(json!([]));
        let paused = life["screen"]["paused_until"].as_f64().unwrap_or(0.0) > 0.0;
        let gym = if i >= gym_from { life.get("gym").cloned().unwrap_or(Value::Null) } else { e["gym"].clone() };
        let checks = [("gym", gym, e["gym"].clone()), ("out", Value::Array(got), e["out"].clone()), ("cmds", Value::Array(cmds), e["cmds"].clone()),
                      ("ib", Value::Array(ib), e["ib"].clone()), ("rules", rules, e["rules"].clone()),
                      ("paused", json!(paused), e["paused"].clone())];
        for (k, a, b) in checks {
            if a != b {
                bad.push(format!("#{i} {q:?} {k}\n  want {b}\n  got  {a}"));
            }
        }
    }
    zenko_desktop::set_fake_now(None);
    assert!(bad.is_empty(), "{} differ:\n{}", bad.len(), bad.join("\n"));
}
