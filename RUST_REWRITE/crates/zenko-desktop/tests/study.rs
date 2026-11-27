//! zenko8's study tracker and study chat (golden/study.json, tools/make_golden_study.py): study ticks on windows
//! (subjects, distractions, unsure windows, idle), the "studying Bio?" ask and its taps, exam credit and the planned
//! session tick-off, study_slow (missed sessions + re-plan, recap, weak-subject alert, golden hour) and study_route.
//! Every line, bubble (with buttons), command, the study logs, the timetable and self.st must match.

use serde_json::{Value, json};
use zenko_desktop::{Cfg, Companion, Ev};
use zenko_sound::mt::PyRandom;

#[test]
fn study_matches_zenko_py() {
    let g: Vec<Value> = serde_json::from_str(&std::fs::read_to_string("../../golden/study.json").unwrap()).unwrap();
    let d = std::env::temp_dir().join(format!("zk_study_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(d.join("FoxBuddy")).unwrap();
    unsafe { std::env::set_var("APPDATA", &d) };
    let mut now = g[0]["now"].as_f64().unwrap();
    zenko_desktop::set_fake_now(Some(now));
    std::fs::write(d.join("FoxBuddy/deadlines.json"), g[0]["deadlines"].to_string()).unwrap();
    std::fs::write(d.join("FoxBuddy/timetable.json"), g[0]["tt"].to_string()).unwrap();
    let mut fox = Companion::new(Cfg::from(json!({"robe": "none"})), 1.0, now);
    fox.life_startup();
    fox.rewards.next_tick = Some(f64::MAX);
    fox.study.slow_next = f64::MAX;
    fox.env.idle = 0.0;
    let life0 = g[0]["life"].clone();
    zenko_desktop::test_life_do(&mut fox, |l| {
        for (k, v) in life0.as_object().unwrap() {
            l[k.as_str()] = v.clone();
        }
    });
    let mut last: Option<Vec<(String, String)>> = None;
    let mut bad = vec![];
    for (i, e) in g.iter().enumerate().skip(1) {
        let q = e["q"].as_str().unwrap();
        fox.brain.rng = PyRandom::new(4000 + i as u64 - 1);
        fox.chat_log.clear();
        fox.events.clear();
        let mut brain = None;
        if let Some(rest) = q.strip_prefix("@fg ") {
            let p: Vec<&str> = rest.split('|').collect();
            let site = (!p[3].is_empty()).then(|| p[3].to_string());
            for _ in 0..p[4].parse::<usize>().unwrap() {
                now += 1.0;
                zenko_desktop::set_fake_now(Some(now));
                fox.study.fg = Some((p[0].into(), p[1].into(), p[2].parse().unwrap(), site.clone()));
                fox.study_tick(now);
            }
        } else {
            now = e["now"].as_f64().unwrap();
            zenko_desktop::set_fake_now(Some(now));
            if let Some(k) = q.strip_prefix("@tap ") {
                let ch = last.take().unwrap();
                let c = ch[k.parse::<usize>().unwrap()].1.clone();
                if !c.is_empty() {
                    assert!(fox.study_button(now, &c), "{c}");
                }
            } else if q == "@slow" {
                fox.study_slow(now);
            } else if !q.starts_with('@') {
                if let zenko_desktop::chat::Sent::Brain(b) = fox.chat_send(now, q, (0, 0)) {
                    brain = Some(b);
                }
            }
        }
        assert_eq!(now, e["now"].as_f64().unwrap(), "#{i} clock");
        fox.pump(now + 2.7);
        let mut got: Vec<Value> = fox.chat_log.iter().map(|(w, t)| json!(["line", w, t])).collect();
        if let Some(b) = brain {
            got.push(json!(["llm", b]));
        }
        let first: Vec<String> = fox.chat_log.iter().filter(|(w, _)| w == "Fox")
            .map(|(_, t)| t.split('\n').next().unwrap().to_string()).collect();
        let (mut cmds, mut says) = (vec![], vec![]);
        for ev in &fox.events {
            match ev {
                Ev::Say { text, choices, .. } => {
                    if !choices.is_empty() {
                        last = Some(choices.clone());
                    }
                    if !first.contains(text) {
                        says.push(json!(["say", text, choices.iter().map(|c| c.0.clone()).collect::<Vec<_>>()]));
                    }
                }
                Ev::Command(c) => cmds.push(json!(c)),
                _ => {}
            }
        }
        let l = zenko_desktop::read_json("zenko_life.json");
        let mut life = serde_json::Map::new();
        for k in ["slog", "dlog", "hourlog", "unsorted", "win_subj", "subj_words", "goals", "studied", "said"] {
            life.insert(k.into(), l.get(k).cloned().unwrap_or(Value::Null));
        }
        let st = fox.study_state();
        let ids: Vec<Value> = zenko_desktop::plan::load_deadlines().iter().map(|d| d.get("id").cloned().unwrap_or(Value::Null)).collect();
        let checks = [("out", Value::Array(got), &e["out"]), ("says", Value::Array(says), &e["says"]),
                      ("cmds", Value::Array(cmds), &e["cmds"]), ("life", Value::Object(life), &e["life"]),
                      ("tt", Value::Array(fox.tt.clone()), &e["tt"]), ("st_now", st["now"].clone(), &e["st_now"]),
                      ("override", st["override"].clone(), &e["override"]), ("credit", st["credit"].clone(), &e["credit"]),
                      ("deadline_ids", Value::Array(ids), &e["deadline_ids"])];
        for (k, a, b) in checks {
            if a != *b {
                bad.push(format!("#{i} {q:?} {k}\n  want {b}\n  got  {a}"));
            }
        }
    }
    zenko_desktop::set_fake_now(None);
    assert!(bad.is_empty(), "{} differ:\n{}", bad.len(), bad.join("\n"));
}
