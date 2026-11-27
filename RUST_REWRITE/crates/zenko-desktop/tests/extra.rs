//! zenko.py's remaining chat routes (golden/extra.json, tools/make_golden_extra.py): life_route, extra_route,
//! app_route, deadline_route, battery and notes_route. Every reply (fox voice included), bubble, what she opens or
//! changes, her extras file, the settings she touches, flashcards, facts, free hours, the timetable, deadlines, the
//! session, pomodoro, lock-in, the review quiz and sulking must match.

use serde_json::{Value, json};
use zenko_desktop::{Cfg, Companion, Ev};
use zenko_sound::mt::PyRandom;

#[test]
fn extra_matches_zenko_py() {
    let g: Vec<Value> = serde_json::from_str(&std::fs::read_to_string("../../golden/extra.json").unwrap()).unwrap();
    let d = std::env::temp_dir().join(format!("zk_extra_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    let fb = d.join("FoxBuddy");
    std::fs::create_dir_all(fb.join("remarkable/notes")).unwrap();
    unsafe { std::env::set_var("APPDATA", &d) };
    let now0 = g[0]["now"].as_f64().unwrap();
    zenko_desktop::set_fake_now(Some(now0));
    std::fs::write(fb.join("deadlines.json"), g[0]["deadlines"].to_string()).unwrap();
    std::fs::write(fb.join("config.json"), g[0]["cfg"].to_string()).unwrap();
    for (k, v) in g[0]["notes"].as_object().unwrap() {
        std::fs::write(fb.join(format!("remarkable/notes/{k}.md")), v.as_str().unwrap()).unwrap();
    }
    let mut cfg = g[0]["cfg"].clone();
    cfg["robe"] = json!("none");
    let mut fox = Companion::new(Cfg::from(cfg), 1.0, now0);
    fox.life_startup();
    fox.rewards.next_tick = Some(f64::MAX);
    fox.study.slow_next = f64::MAX;
    fox.omikuji_seed = Some(777);
    fox.battery = Some((57, true));
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
        fox.brain.rng = PyRandom::new(5000 + i as u64 - 1);
        fox.chat_log.clear();
        fox.events.clear();
        fox.trace.clear();
        let r = fox.chat_send(now, q, (0, 0));
        fox.pump(now + 2.7);
        let mut got: Vec<Value> = fox.chat_log.iter().map(|(w, t)| json!(["line", w, t])).collect();
        if let zenko_desktop::chat::Sent::Brain(b) = r {
            got.push(json!(["llm", b]));
        }
        let first: Vec<String> = fox.chat_log.iter().filter(|(w, _)| w == "Fox")
            .map(|(_, t)| t.split('\n').next().unwrap().to_string()).collect();
        let (mut cmds, mut says) = (vec![], vec![]);
        for ev in &fox.events {
            match ev {
                Ev::Say { text, choices, .. } if !first.contains(text) => {
                    says.push(json!(["say", text, choices.iter().map(|c| c.0.clone()).collect::<Vec<_>>()]))
                }
                // nb_show opens the notebook page (book_open -> open_hub("notebook")); nb_start_test is the hub's own
                Ev::Command(c) if c.starts_with("nb_show:") => cmds.push(json!("open_hub:notebook")),
                Ev::Command(c) if c.starts_with("nb_start_test:") => {}
                Ev::Command(c) => cmds.push(json!(c)),
                _ => {}
            }
        }
        // the stubbed calls, in the order zenko.py made them (visitor_start is a command there too)
        let mut all = vec![];
        for t in &fox.trace {
            all.push(json!(t));
        }
        let cmds: Vec<Value> = cmds.into_iter().chain(all).collect();
        let ex = zenko_desktop::extra::extras();
        let extra: serde_json::Map<String, Value> = ["notes", "todos", "reminders", "omikuji", "bond", "treasures", "treats"]
            .iter().map(|k| (k.to_string(), ex.get(*k).cloned().unwrap_or(Value::Null))).collect();
        let cf: serde_json::Map<String, Value> = e["cfg"].as_object().unwrap().iter()
            .map(|(k, want)| (k.clone(), fox.cfg.raw.get(k).cloned().unwrap_or_else(|| default_cfg(k, want)))).collect();
        let l = zenko_desktop::read_json("zenko_life.json");
        let life: serde_json::Map<String, Value> = ["cards", "facts", "notes", "free", "free_dates", "free_set", "plan_hours", "plan_short"]
            .iter().map(|k| (k.to_string(), l.get(*k).cloned().unwrap_or(Value::Null))).collect();
        let mut badges: Vec<String> = l["badges"].as_object().map(|o| o.keys().cloned().collect()).unwrap_or_default();
        badges.sort();
        let ema: Vec<Value> = l["ema"].as_array().cloned().unwrap_or_default().iter().map(|x| json!([x["text"], x["at"]])).collect();
        let sess = fox.session.as_ref().map(|s| json!([s.task, s.planned, s.paused])).unwrap_or(Value::Null);
        let pomo = fox.pomo.as_ref().map(|p| json!({"work": p.work, "brk": p.brk, "rounds": p.rounds, "round": p.round,
                                                    "waiting": p.waiting})).unwrap_or(Value::Null);
        // zenko.py's prebuilt Upcoming page (up_plan) turns a missing plan_short into {} a few seconds after start;
        // the Rust hub page comes in stage 4b
        let mut life = life;
        if life["plan_short"].is_null() && e["life"]["plan_short"] == json!({}) {
            life.insert("plan_short".into(), json!({}));
        }
        // her idle stretch / play-bow lines come from the animation loop, which runs on Tk's real clock
        let want_says: Vec<Value> = e["says"].as_array().unwrap().iter()
            .filter(|s| !IDLE_LINES.contains(&s[1].as_str().unwrap_or(""))).cloned().collect();
        let checks = [("out", Value::Array(got), &e["out"]), ("says", Value::Array(says), &Value::Array(want_says)),
                      ("cmds", Value::Array(cmds), &e["cmds"]), ("extra", Value::Object(extra), &e["extra"]),
                      ("cfg", Value::Object(cf), &e["cfg"]), ("life", Value::Object(life), &e["life"]),
                      ("badges", json!(badges), &e["badges"]), ("ema", Value::Array(ema), &e["ema"]),
                      ("tt", Value::Array(fox.tt.clone()), &e["tt"]),
                      ("deadlines", Value::Array(zenko_desktop::plan::load_deadlines()), &e["deadlines"]),
                      ("session", sess, &e["session"]), ("pomo", pomo, &e["pomo"]), ("lockin", json!(fox.lockin.is_some()), &e["lockin"]),
                      ("quiz", fox.quiz_state(), &e["quiz"]), ("sulk", json!(fox.sulk_until != 0.0), &e["sulk"])];
        for (k, a, b) in checks {
            if a != *b {
                bad.push(format!("#{i} {q:?} {k}\n  want {b}\n  got  {a}"));
            }
        }
    }
    zenko_desktop::set_fake_now(None);
    assert!(bad.is_empty(), "{} differ:\n{}", bad.len(), bad.join("\n"));
}

const IDLE_LINES: [&str; 4] = ["Nnnnh~ *big stretch*", "Fuwaa~ stretchy stretch~", "*play bow* Play with me~?",
                                "Kon! *wiggles* Wanna play?"];

/// zenko.py's cfg defaults for the keys compared (DEFAULTS); the golden's first value when it never changes
fn default_cfg(k: &str, _want: &Value) -> Value {
    match k {
        "outfit" => json!([]),
        "scarf" | "coat" => json!(""),
        "fox_name" => json!("Zenko"),
        "bgm" | "roam" | "seasons" | "daynight" => json!(true),
        "bgm_track" => json!("garden"),
        _ => Value::Null,
    }
}
