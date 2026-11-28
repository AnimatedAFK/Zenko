//! Session and lock-in rewards (hunger, badges, garden, journey, counters) against golden/rewards.json, which
//! tools/make_golden_rewards.py makes by running zenko.py's own methods with the same files and clock. The
//! zenko_life.json and stats.json Rust writes must be byte-for-byte what zenko.py writes, and the delayed lines
//! (badges, garden, journey) must match in time, text and chips.
use serde_json::{Value, json};
use zenko_desktop::lockin::Lockin;
use zenko_desktop::rewards::Later;
use zenko_desktop::{Cfg, Companion, Ev, set_fake_now};

fn read(dir: &std::path::Path, name: &str) -> Option<String> {
    std::fs::read_to_string(dir.join(name)).ok()
}

/// the garden seed comes from each side's own random numbers when the case starts without a garden
fn no_seed(s: &Option<String>) -> Option<String> {
    s.as_ref().map(|s| {
        s.lines()
            .map(|l| if l.trim_start().starts_with("\"seed\": ") { "  \"seed\": N" } else { l })
            .collect::<Vec<_>>()
            .join("\n")
    })
}

#[test]
fn rewards_match_zenko_py() {
    let g: Value = serde_json::from_str(include_str!("../../../golden/rewards.json")).unwrap();
    let root = std::env::temp_dir().join(format!("zk_rewards_{}", std::process::id()));
    unsafe {
        std::env::set_var("ZK_TZ_OFFSET", "0");
    }
    let mut bad = vec![];
    for case in g.as_array().unwrap() {
        let name = case["name"].as_str().unwrap();
        let dir = root.join(name);
        let _ = std::fs::remove_dir_all(&dir);
        unsafe { std::env::set_var("APPDATA", &dir) };
        let data = zenko_desktop::data_dir();
        std::fs::create_dir_all(&data).unwrap();
        for (f, body) in case["files"].as_object().unwrap() {
            let text = match body {
                Value::String(s) => s.clone(),
                v => serde_json::to_string(v).unwrap(),
            };
            std::fs::write(data.join(f), text).unwrap();
        }
        let now0 = case["now"].as_f64().unwrap();
        set_fake_now(Some(now0));
        let mut cfg = json!({"pet_needs": false});
        for (k, v) in case["cfg"].as_object().unwrap() {
            cfg[k] = v.clone();
        }
        let mut f = Companion::new(Cfg::from(cfg), 1.0, 1000.0);
        f.life_startup();
        let seeded = case["files"].get("zenko_life.json").and_then(|l| l.as_str()).is_some_and(|l| l.contains("\"garden\""));
        let fix = |s: Option<String>| if seeded { s } else { no_seed(&s) };
        let startup = read(&data, "zenko_life.json");
        let want_startup = case["startup"].as_str().map(String::from);
        if fix(startup.clone()) != fix(want_startup) {
            bad.push(format!("{name}: start-up file differs:\n{}", startup.unwrap_or_default()));
        }
        let act = &case["act"];
        let t = 1000.0;
        f.events.clear();
        f.rewards.later.clear();
        set_fake_now(Some(now0 + act["later"].as_f64().unwrap_or(0.0)));
        match act["do"].as_str().unwrap() {
            "session" => {
                let item = act.get("item").and_then(|v| v.as_u64()).map(|v| v as usize);
                f.start_session_item(t, act["task"].as_str().unwrap(), act["planned"].as_f64().unwrap() / 60.0, item);
                let s = f.session.as_mut().unwrap();
                s.elapsed = act["elapsed"].as_f64().unwrap();
                s.dist = act["dist"].as_f64().unwrap();
                s.away = act["away"].as_f64().unwrap();
                f.rewards.later.clear();
                f.end_session(t, act["completed"].as_bool().unwrap());
            }
            "lockin" => {
                let wall = zenko_desktop::unix_now();
                f.lockin = Some(Lockin {
                    until: wall + 600.0,
                    eaten: act["eaten"].as_i64().unwrap(),
                    good: None,
                    last_eat: 0.0,
                    last_hwnd: None,
                    t0: wall - act["mins"].as_f64().unwrap() * 60.0,
                });
                f.lockin_stop(t, act["done"].as_bool().unwrap(), false);
            }
            "tt" => {
                // check_timetable at each step; what she says, beeps and reads aloud must match
                let mut got = vec![];
                for off in act["steps"].as_array().unwrap() {
                    let off = off.as_f64().unwrap();
                    set_fake_now(Some(now0 + off));
                    f.check_timetable(t + off);
                    for e in f.events.drain(..) {
                        match e {
                            Ev::Say { text, secs, .. } => got.push(json!(["say", text, secs])),
                            Ev::Beep { kind, .. } => got.push(json!(["beep", kind])),
                            Ev::Speak(text) | Ev::SayAloud(text) => got.push(json!(["speak", text])),
                            _ => {}
                        }
                    }
                    got.push(json!(["step", off]));
                }
                let want: Vec<Value> = case["out"].as_array().unwrap().iter()
                    .filter(|e| e[1] != "mood")
                    .map(|e| {
                        let mut a = e.as_array().unwrap()[1..].to_vec();
                        if a[0] == "say" {
                            a[2] = json!(a[2].as_f64().unwrap());
                        }
                        if a[0] == "speak" {
                            a.truncate(2);
                        }
                        if a[0] == "step" {
                            a[1] = json!(a[1].as_f64().unwrap());
                        }
                        Value::Array(a)
                    })
                    .collect();
                if got != want {
                    bad.push(format!("{name}: timetable alerts\n got  {}\n want {}", json!(got), json!(want)));
                }
            }
            _ => f.z22_badge_check(t),
        }
        // the delayed lines, run in time order like Tk's after() queue
        let mut got: Vec<Value> = vec![];
        let mut later = std::mem::take(&mut f.rewards.later);
        later.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
        for (at, job) in later {
            if let Later::Say { text, secs, choices, beep, .. } = job {
                let mut e = json!([at - t, "say", text, secs]);
                if !choices.is_empty() {
                    e.as_array_mut().unwrap().push(json!(choices.iter().map(|c| c.0.clone()).collect::<Vec<_>>()));
                }
                got.push(e);
                if let Some(b) = beep {
                    got.push(json!([at - t, "beep", b]));
                }
            } else if let Later::SayChat { text, secs } = job {
                got.push(json!([at - t, "say", text, secs]));
                got.push(json!([at - t, "beep", "gift"]));
            }
        }
        let want: Vec<Value> = case["later"].as_array().unwrap().iter()
            .filter(|e| e[1] == "say" || e[1] == "beep")
            .map(|e| {
                let mut e = e.clone();
                if e[1] == "say" {
                    e[3] = json!(e[3].as_f64().unwrap());
                }
                e
            })
            .collect();
        if got != want {
            bad.push(format!("{name}: delayed lines\n got  {}\n want {}", json!(got), json!(want)));
        }
        let life = read(&data, "zenko_life.json");
        if fix(life.clone()) != fix(case["life"].as_str().map(String::from)) {
            bad.push(format!("{name}: zenko_life.json differs:\n{}", life.unwrap_or_default()));
        }
        let tt = read(&data, "timetable.json");
        if tt.as_deref() != case["timetable"].as_str() {
            bad.push(format!("{name}: timetable.json {:?}\n vs {:?}", tt, case["timetable"]));
        }
        let stats = read(&data, "stats.json");
        if stats.as_deref() != case["stats"].as_str() {
            bad.push(format!("{name}: stats.json {:?} vs {:?}", stats, case["stats"]));
        }
    }
    set_fake_now(None);
    assert!(bad.is_empty(), "{} problem(s):\n{}", bad.len(), bad.join("\n\n"));
}
