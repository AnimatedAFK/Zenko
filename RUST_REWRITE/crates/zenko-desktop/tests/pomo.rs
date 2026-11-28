//! Pomodoro rounds (zenko.py pomo_start / pomo_session_done / pomo_break_done) and the timetable menu item.
use zenko_desktop::{Cfg, Companion, Ev};

fn fox(tag: &str) -> Companion {
    let d = std::env::temp_dir().join(format!("zk_pomo_{}_{tag}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    unsafe { std::env::set_var("APPDATA", &d) };
    Companion::new(Cfg::from(serde_json::json!({"pet_needs": false})), 1.0, 1000.0)
}

fn said(f: &mut Companion) -> Vec<String> {
    f.events.drain(..).filter_map(|e| if let Ev::Say { text, .. } = e { Some(text) } else { None }).collect()
}

#[test]
fn pomodoro_rounds() {
    let mut f = fox("rounds");
    let mut t = 1000.0;
    let r = f.pomo_start(t, 25, 5, 2);
    assert_eq!(r, "Pomodoro on! 2 rounds of 25 min focus + 5 min break. Round 1, go~kon!");
    assert_eq!(f.session.as_ref().unwrap().task, "Pomodoro 1/2");
    assert_eq!(f.pomo_start(t, 25, 5, 2), "A session is already running. Say 'stop' first, then try again~");
    f.end_session(t, true);
    assert!(f.watch.auto_break.is_none(), "the pomodoro decides the break, not auto_break_after_session");
    assert!(f.pomo.as_ref().unwrap().waiting);
    f.pump(t + 3.9);
    assert!(f.brk.is_none());
    t += 4.0;
    f.pump(t);
    assert_eq!(f.brk.as_ref().unwrap().planned, 300.0);
    said(&mut f);
    f.end_break(t, true);
    assert_eq!(f.pomo.as_ref().unwrap().round, 2);
    f.pump(t + 2.4);
    assert!(f.session.is_none());
    t += 2.5;
    f.pump(t);
    assert_eq!(f.session.as_ref().unwrap().task, "Pomodoro 2/2");
    f.end_session(t, true);
    assert!(f.pomo.is_none());
    said(&mut f);
    f.pump(t + 2.5);
    assert!(said(&mut f).contains(&"All 2 pomodoros done! Sugoi~! Take a long break, you earned it~kon!".to_string()));
}

#[test]
fn stopping_ends_the_pomodoro() {
    let mut f = fox("stop");
    f.pomo_start(1000.0, 25, 5, 4);
    f.end_session(1000.0, false);
    assert!(f.pomo.is_none());
    f.pump(1010.0);
    assert!(f.brk.is_none());
}
