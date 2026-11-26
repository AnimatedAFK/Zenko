//! Desktop-fox behaviour against zenko.py's rules: where a click lands, sulking after 8 pokes, a finished session
//! writing stats.json, and the bubble lines she says.
use zenko_desktop::{Cfg, Companion, Ev};

fn fox() -> Companion {
    let d = std::env::temp_dir().join(format!("zk_test_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    unsafe { std::env::set_var("APPDATA", &d) };
    Companion::new(Cfg::from(serde_json::json!({"size": 4, "robe": "none"})), 1.0, 1000.0)
}

fn says(f: &mut Companion) -> Vec<String> {
    f.events.drain(..).filter_map(|e| if let Ev::Say { text, .. } = e { Some(text) } else { None }).collect()
}

#[test]
fn behaviour() {
    let mut f = fox();
    let (cx, fy, z) = (f.colw, f.fox_y, f.z);
    // nose (31..41, 32..38) = boop, head = pet, tails = tail, belly = belly
    for (x, y, kind) in [(36, 35, "boop"), (30, 20, "pet"), (70, 50, "tail"), (25, 60, "belly")] {
        f.fox_clicked(1000.0 + x as f64 * 10.0, cx + x * z, fy + y * z);
        assert_eq!(f.play.as_ref().unwrap().kind, kind);
    }
    // eight pokes inside 4 seconds: she sulks
    let mut f = fox();
    for i in 0..8 {
        f.fox_clicked(2000.0 + i as f64 * 0.3, cx + 30 * z, fy + 20 * z);
    }
    assert!(f.sulk_until > 2000.0);
    assert!(says(&mut f).iter().any(|s| s == "Mou~!! Too many pokes! Hmph! (Say sorry, or give me a treat~)"));
    // a treat while sulking forgives her
    f.interact(2003.0, "treat", Some("dango".into()));
    assert_eq!(f.sulk_until, 0.0);
    assert!(says(&mut f).iter().any(|s| s == "...*sniff sniff* Is that dango? ...Fine. Forgiven! *munch*"));
    // a one-minute session runs out and is written to stats.json
    let mut f = fox();
    f.start_session(3000.0, "Biology notes", 1.0);
    assert_eq!(says(&mut f), vec!["Let's go! 1 min on \u{201c}Biology notes\u{201d}. I'm watching.".to_string()]);
    for i in 1..=61 {
        f.tick_1s(3000.0 + i as f64, 1.0, 0.5, "Notes", "notepad.exe");
    }
    assert!(f.session.is_none());
    assert_eq!(says(&mut f), vec!["Done! 1 min on \u{201c}Biology notes\u{201d}. Off-task: 0 min. Spotless!".to_string()]);
    let st = zenko_desktop::stats();
    assert_eq!(st[zenko_desktop::today_iso()].as_i64(), Some(60));

    // the session ends, and 7 seconds later a 5-minute break starts by itself (auto_break_after_session)
    for i in 62..=69 {
        f.tick_1s(3000.0 + i as f64, 1.0, 0.5, "Notes", "notepad.exe");
    }
    assert!(f.brk.is_some());
    assert_eq!(f.current_mood(3070.0), "sleep");
    assert_eq!(f.belly_text(3070.0), "04:57");
    f.end_break(3071.0, false);
    assert!(f.brk.is_none());

    // Reddit during a session: a nag at the grace time, a warning, then the tab is closed; going back = praise
    let mut f = fox();
    f.start_session(5000.0, "Chem", 25.0);
    says(&mut f);
    let mut cmds = vec![];
    let mut all = vec![];
    for i in 1..=21 {
        f.tick_1s(5000.0 + i as f64, 1.0, 0.5, "Reddit - Dive into anything - Google Chrome", "chrome.exe");
        for e in f.events.drain(..) {
            match e {
                Ev::Command(c) => cmds.push(c),
                Ev::Say { text, .. } => all.push(text),
                _ => {}
            }
        }
    }
    assert!(f.watch.alerted);
    assert_eq!(f.current_mood(5021.0), "alert");
    assert!(all.iter().any(|s| s.starts_with("Closing Reddit in 10 seconds")), "{all:?}");
    assert_eq!(cmds, vec!["close:tab:Reddit".to_string()]);
    assert!((f.session.as_ref().unwrap().dist - 21.0).abs() < 1e-9);
    f.tick_1s(5022.0, 1.0, 0.5, "Notes", "notepad.exe");
    assert!(!f.watch.alerted);
    assert_eq!(says(&mut f).len(), 1); // a PRAISE line
}
