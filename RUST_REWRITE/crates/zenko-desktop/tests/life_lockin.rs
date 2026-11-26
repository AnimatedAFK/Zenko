//! Roaming, the idle routines and lock-in, against zenko.py's rules.
use zenko_desktop::{Cfg, Companion, Ev};

fn fox(tag: &str) -> Companion {
    let d = std::env::temp_dir().join(format!("zk_life_{}_{tag}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    unsafe { std::env::set_var("APPDATA", &d) };
    let mut f = Companion::new(Cfg::from(serde_json::json!({"size": 4, "robe": "none"})), 1.0, 1000.0);
    f.env.work = (0, 0, 1920, 1040);
    f.env.screen_w = 1920;
    f
}

#[test]
fn roaming_walks_and_comes_home() {
    let mut f = fox("roam");
    let home = f.roam_home();
    let mut win = home;
    f.life.roam_next = 0.0;
    let mut walked = false;
    let mut moved = false;
    let mut t = 1000.0;
    for _ in 0..3000 {
        t += 0.06;
        f.win_move = None;
        let _ = f.frame(t, (300, 900), win);
        if let Some(p) = f.win_move {
            moved |= p != home;
            win = p;
        }
        walked |= f.last_state.starts_with("qwalk");
        f.events.clear();
    }
    assert!(walked && moved, "she never walked");
    // a session sends her straight home and stops roaming
    f.life.roam_next = 0.0;
    for _ in 0..40 {
        t += 0.06;
        let _ = f.frame(t, (300, 900), win);
        if let Some(p) = f.win_move.take() {
            win = p;
        }
        if f.life.roam.is_some() {
            break;
        }
    }
    f.start_session(t, "Chem", 25.0);
    let _ = f.frame(t + 0.06, (300, 900), win);
    assert!(f.life.roam.is_none());
}

#[test]
fn idle_routine_poses() {
    let mut f = fox("pose");
    let win = f.roam_home();
    f.life.roam_next = 1e12;
    let _ = f.frame(1000.0, (0, 0), win); // sets the first routine time
    f.life.z19_next = Some(1000.1);
    let _ = f.frame(1000.2, (0, 0), win);
    assert!(f.last_state.starts_with("rigL:") || f.last_state.starts_with("rigR:"), "{}", f.last_state);
    // home is on the right half of the screen, so she faces left
    assert!(f.last_state.starts_with("rigL:"));
}

#[test]
fn lockin_pounces_and_stops() {
    let mut f = fox("lock");
    f.lockin_start(2000.0, Some(1), false);
    assert_eq!(f.session.as_ref().map(|s| s.task.as_str()), Some("Lock-in"));
    assert!(f.accessories(2000.0).iter().any(|a| a == "headband"));
    let says: Vec<String> =
        f.events.drain(..).filter_map(|e| if let Ev::Say { text, .. } = e { Some(text) } else { None }).collect();
    assert!(says.iter().any(|s| s.starts_with("Lock-in mode ON for 1 min! *ties hachimaki*")));
    // a good window first, then Reddit
    assert_eq!(f.lockin_check(2000.2, (11, "Notes", "notepad.exe", 5), false, 99), None);
    let e = f.lockin_check(2000.4, (22, "Reddit - Dive into anything - Google Chrome", "chrome.exe", 6), false, 99)
        .expect("pounce");
    assert!(e.tab && e.site == "Reddit" && e.good == Some(11));
    assert!(f.lock_eating);
    assert_eq!(f.lockin.as_ref().unwrap().eaten, 1);
    // still eating: nothing more
    assert_eq!(f.lockin_check(2000.6, (22, "Reddit", "chrome.exe", 6), false, 99), None);
    f.lock_eating = false;
    // her own windows are never eaten
    assert_eq!(f.lockin_check(2001.0, (33, "Reddit", "zenkorust.exe", 99), false, 99), None);
    // hold Esc for 2 seconds
    assert!(f.lockin_check(2002.0, (11, "Notes", "notepad.exe", 5), true, 99).is_none());
    assert!(f.lockin.is_some());
    assert!(f.lockin_check(2004.1, (11, "Notes", "notepad.exe", 5), true, 99).is_none());
    assert!(f.lockin.is_none());
    let says: Vec<String> =
        f.events.drain(..).filter_map(|e| if let Ev::Say { text, .. } = e { Some(text) } else { None }).collect();
    assert!(says.iter().any(|s| s == "Lock-in off. I ate 1 distraction while it was on. Rest well, ne~"), "{says:?}");
}
