//! Her offline chat against zenko.py (golden/chat.json, tools/make_golden_chat.py): 100 scripted lines through
//! Zenko23.chat_send with Python's random seeded per line. Every reply (fox voice included), what goes to the smart
//! brain, the check-in she opens, and the session / break state after each line must match.

use serde_json::{Value, json};
use zenko_desktop::chat::Sent;
use zenko_desktop::{Cfg, Companion, Ev};
use zenko_sound::mt::PyRandom;

#[test]
fn chat_matches_zenko_py() {
    let g: Vec<Value> =
        serde_json::from_str(&std::fs::read_to_string("../../golden/chat.json").unwrap()).unwrap();
    let d = std::env::temp_dir().join(format!("zk_chat_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    unsafe { std::env::set_var("APPDATA", &d) };
    zenko_desktop::set_fake_now(Some(g[0]["now"].as_f64().unwrap()));
    let mut fox = Companion::new(Cfg::from(json!({"robe": "none"})), 1.0, 1000.0);
    let mut bad = vec![];
    for (i, e) in g.iter().enumerate() {
        let q = e["q"].as_str().unwrap();
        let now = e["now"].as_f64().unwrap();
        zenko_desktop::set_fake_now(Some(now));
        fox.brain.rng = PyRandom::new(1000 + i as u64);
        if let (Some(s), Some(p)) = (fox.session.as_mut(), e["pre"].as_f64()) {
            s.elapsed = p;
        }
        fox.chat_log.clear();
        let n0 = 0;
        fox.events.clear();
        let r = fox.chat_send(now, q, (0, 0));
        fox.pump(now + 0.3);
        let mut got: Vec<Value> =
            fox.chat_log[n0..].iter().map(|(w, t)| json!(["line", w, t])).collect();
        if let Sent::Brain(b) = r {
            got.push(json!(["llm", b]));
        }
        for ev in &fox.events {
            if let Ev::Command(c) = ev {
                match c.as_str() {
                    "open_checkin_force" => got.push(json!(["checkin", true])),
                    "open_breathing" => got.push(json!(["breathing"])),
                    c if c.starts_with("breath:") => got.push(json!(["breath", &c[7..]])),
                    _ => {}
                }
            }
        }
        let sess = fox.session.as_ref().map(|s| json!({"task": s.task, "planned": s.planned, "paused": s.paused}));
        let brk = fox.brk.as_ref().map(|b| json!({"planned": b.planned}));
        let want_out = e["out"].clone();
        let ok = Value::Array(got.clone()) == want_out
            && sess.unwrap_or(Value::Null) == e["session"]
            && brk.unwrap_or(Value::Null) == e["brk"];
        if !ok {
            bad.push(format!("#{i} {q:?}\n  want {want_out} {} {}\n  got  {} {:?} {:?}", e["session"], e["brk"],
                             Value::Array(got), fox.session.as_ref().map(|s| (&s.task, s.planned, s.paused)),
                             fox.brk.as_ref().map(|b| b.planned)));
        }
    }
    zenko_desktop::set_fake_now(None);
    assert!(bad.is_empty(), "{} of {} differ:\n{}", bad.len(), g.len(), bad.join("\n"));
}
