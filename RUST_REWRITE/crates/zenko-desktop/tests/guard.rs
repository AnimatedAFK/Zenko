//! Stage 3g's desktop leftovers (golden/guard.json, tools/make_golden_guard.py): deadline_reminders, books_slow (sleep
//! and burnout guard) + gym_water_nudge, screen_tick + screen_tell, and zk_slow_tick's pop quiz, IB and garden nudges.
//! Every bubble (with buttons), command, life["screen"/"said"/"quiz_offers"/"cards"/"stats"], the water log, the
//! deadline note, the break and the session pause must match zenko.py.

use serde_json::{Value, json};
use zenko_desktop::{Cfg, Companion, Ev};
use zenko_sound::mt::PyRandom;

#[test]
fn guard_matches_zenko_py() {
    let g: Vec<Value> = serde_json::from_str(&std::fs::read_to_string("../../golden/guard.json").unwrap()).unwrap();
    let d = std::env::temp_dir().join(format!("zk_guard_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(d.join("FoxBuddy")).unwrap();
    unsafe { std::env::set_var("APPDATA", &d) };
    let mut now = g[0]["now"].as_f64().unwrap();
    zenko_desktop::set_fake_now(Some(now));
    std::fs::write(d.join("FoxBuddy/deadlines.json"), g[0]["deadlines"].to_string()).unwrap();
    let mut fox = Companion::new(Cfg::from(json!({"robe": "none"})), 1.0, now);
    fox.life_startup();
    fox.rewards.next_tick = Some(f64::MAX);
    fox.study.slow_next = f64::MAX;
    fox.guard.off();
    fox.env.idle = 0.0;
    let life0 = g[0]["life"].clone();
    let active = g[0]["garden_active"].clone();
    zenko_desktop::test_life_do(&mut fox, |l| {
        for (k, v) in life0.as_object().unwrap() {
            l[k.as_str()] = v.clone();
        }
        l["garden"]["active"] = active;
        l["garden"]["health"] = json!(1.0);
    });
    fox.guard.quiz_offer_t = now - 25.0 * 60.0;
    let mut last: Vec<Vec<(String, String)>> = vec![];
    let mut bad = vec![];
    for (i, e) in g.iter().enumerate().skip(1) {
        let q = e["q"].as_str().unwrap();
        fox.brain.rng = PyRandom::new(6000 + i as u64 - 1);
        fox.events.clear();
        let mut cmds: Vec<Value> = vec![];
        let taps = std::mem::take(&mut last);
        if let Some(rest) = q.strip_prefix("@fg ") {
            let p: Vec<&str> = rest.split('|').collect();
            for _ in 0..p[3].parse::<usize>().unwrap() {
                now += 1.0;
                zenko_desktop::set_fake_now(Some(now));
                fox.study.fg = Some((p[0].into(), p[1].into(), p[2].parse().unwrap(), None));
                let own = zenko_desktop::guard::screen_own_guess(p[0], p[1]);
                fox.screen_tick(now, 0, own);
            }
        } else {
            now = e["now"].as_f64().unwrap();
            zenko_desktop::set_fake_now(Some(now));
            if let Some(r) = q.strip_prefix("@tap ") {
                let v: Vec<usize> = r.split(' ').map(|x| x.parse().unwrap()).collect();
                let c = taps[v[0]][v[1]].1.clone();
                if c.starts_with("lockin:") || c.starts_with("open_hub:") {
                    cmds.push(json!(c));
                } else {
                    assert!(fox.guard_button(now, &c), "{c}");
                }
            } else if let Some(n) = q.strip_prefix("@books ") {
                let n: f64 = n.parse().unwrap();
                if n > 0.0 {
                    let (day, hour) = zenko_desktop::guard::test_day_hour();
                    zenko_desktop::test_life_do(&mut fox, |l| {
                        let h = &mut l["hourlog"][day.as_str()][hour.to_string().as_str()];
                        if !h.is_array() {
                            *h = json!([0, 0]);
                        }
                        h[0] = json!(h[0].as_f64().unwrap() as i64 + n as i64);
                    });
                }
                fox.books_slow(now);
                fox.gym_water_nudge(now);
            } else if q == "@water" {
                fox.gym_water_nudge(now);
            } else if q == "@dl" {
                fox.deadline_reminders(now);
            } else if q == "@zk" {
                fox.zk_slow(now);
            } else if let Some(n) = q.strip_prefix("@slog ") {
                let n: i64 = n.parse().unwrap();
                let (day, _) = zenko_desktop::guard::test_day_hour();
                zenko_desktop::test_life_do(&mut fox, |l| {
                    let b = l["slog"][day.as_str()]["bio"].as_i64().unwrap_or(0);
                    l["slog"][day.as_str()]["bio"] = json!(b + n);
                });
            } else if q == "@endbreak" {
                fox.end_break(now, false);
            }
        }
        assert_eq!(now, e["now"].as_f64().unwrap(), "#{i} clock");
        fox.brain.rng = PyRandom::new(6500 + i as u64 - 1); // the harness seeds screen_tell itself
        fox.pump(now + 0.3);
        let mut says = vec![];
        for ev in &fox.events {
            match ev {
                Ev::Say { text, choices, .. } => {
                    if !choices.is_empty() {
                        last.push(choices.clone());
                    }
                    says.push(json!(["say", text, choices.iter().map(|c| c.0.clone()).collect::<Vec<_>>()]));
                }
                Ev::Command(c) => cmds.push(json!(c)),
                _ => {}
            }
        }
        let l = zenko_desktop::read_json("zenko_life.json");
        let mut life = serde_json::Map::new();
        for k in ["screen", "said", "quiz_offers", "cards", "stats"] {
            life.insert(k.into(), l.get(k).cloned().unwrap_or(Value::Null));
        }
        let water = l["gym"]["water"].clone();
        let water = if water.is_null() { json!({}) } else { water };
        let note = fox.cfg.raw.get("last_deadline_note").cloned().unwrap_or(Value::Null);
        let paused = fox.session.as_ref().is_some_and(|s| s.paused);
        let checks = [("says", Value::Array(says), &e["says"]), ("cmds", Value::Array(cmds), &e["cmds"]),
                      ("life", Value::Object(life), &e["life"]), ("water", water, &e["water"]),
                      ("note", note, &e["note"]), ("brk", json!(fox.brk.is_some()), &e["brk"]),
                      ("paused", json!(paused), &e["paused"])];
        for (k, a, b) in checks {
            if a != *b {
                bad.push(format!("#{i} {q:?} {k}\n  want {b}\n  got  {a}"));
            }
        }
    }
    zenko_desktop::set_fake_now(None);
    assert!(bad.is_empty(), "{} differ:\n{}", bad.len(), bad.join("\n"));
}
