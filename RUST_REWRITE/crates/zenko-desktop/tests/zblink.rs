//! The Zenko Browser link (zblink.rs) against zenko.py: the built-in summariser and card makers on golden pages
//! (golden/zblink.json from tools/make_golden_zblink.py), and a browser_inbox.json run end to end in a temp data folder.
use serde_json::{Value, json};
use zenko_desktop::zblink::*;
use zenko_desktop::{Cfg, Companion, Ev};

fn golden() -> Value {
    serde_json::from_str(&std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../../golden/zblink.json")).unwrap()).unwrap()
}

#[test]
fn summariser_and_cards_match_zenko_py() {
    let g = golden();
    for c in g["sentences"].as_array().unwrap() {
        assert_eq!(json!(sentences(c[0].as_str().unwrap())), c[1]);
    }
    for c in g["terms"].as_array().unwrap() {
        assert_eq!(json!(terms(c[0].as_str().unwrap())), c[1]);
    }
    for c in g["offline"].as_array().unwrap() {
        assert_eq!(summarise_offline(c[0].as_str().unwrap(), c[1].as_str().unwrap()), c[2].as_str().unwrap());
    }
    for c in g["cloze"].as_array().unwrap() {
        let got: Vec<Value> = cloze(c[0].as_str().unwrap(), 6).into_iter().map(|(q, a)| json!([q, a])).collect();
        assert_eq!(json!(got), c[1]);
    }
    let docs: Vec<(String, String)> = g["sentences"].as_array().unwrap().iter().zip(g["offline"].as_array().unwrap())
        .map(|(s, o)| (o[0].as_str().unwrap().to_string(), s[0].as_str().unwrap().to_string())).collect();
    for c in g["extract"].as_array().unwrap() {
        let got = summarise_extract(c[0].as_str().unwrap(), &docs, c[1].as_u64().unwrap() as usize, c[2].as_u64().unwrap() as usize);
        assert_eq!(got, c[3].as_str().unwrap(), "{c}");
    }
    for c in g["split"].as_array().unwrap() {
        let got = split_card(c[0].as_str().unwrap()).map(|(q, a)| json!([q, a])).unwrap_or(Value::Null);
        assert_eq!(got, c[1], "{}", c[0]);
    }
}

fn says(f: &mut Companion) -> Vec<(String, Vec<(String, String)>)> {
    f.events.drain(..).filter_map(|e| if let Ev::Say { text, choices, .. } = e { Some((text, choices)) } else { None }).collect()
}

#[test]
fn inbox_and_link_file() {
    let _l = zenko_desktop::web::test_lock();
    let d = std::env::temp_dir().join(format!("zk_zb_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    unsafe { std::env::set_var("APPDATA", &d) };
    let data = zenko_desktop::data_dir();
    std::fs::create_dir_all(&data).unwrap();
    let mut f = Companion::new(Cfg::from(json!({"size": 4, "robe": "none"})), 1.0, 1000.0);
    f.events.clear();
    let inbox = data.join("browser_inbox.json");
    let page = golden()["sentences"][0][0].as_str().unwrap().to_string();
    std::fs::write(&inbox, serde_json::to_string(&json!([
        {"kind": "note", "title": "Osmosis notes", "text": "water moves", "url": "https://x.org/a", "subject": "Biology"},
        {"kind": "clip", "path": "C:\\clip.png"},
        {"kind": "reminder", "text": "  pack   the bag ", "when": 0},
        {"kind": "deadline", "text": "Bio IA draft", "date": "2026-12-03", "time": ""},
        {"kind": "card", "text": "osmosis - water moving across a membrane"},
        {"kind": "quiz", "title": "Osmosis", "text": page},
        "not a dict"
    ])).unwrap()).unwrap();
    let mut live_calls = 0;
    let mut live = |_b: &str| { live_calls += 1; false };
    f.zb_inbox_tick(1000.0, &mut live);
    assert!(!inbox.exists() && !data.join("browser_inbox.json.take").exists());
    let scratch = std::fs::read_to_string(zenko_desktop::notes::nb_scratch()).unwrap();
    assert_eq!(scratch, "\nBiology\nOsmosis notes\nwater moves\n(from https://x.org/a)\n\n[picture clip] C:\\clip.png\n");
    assert_eq!(live_calls, 2);
    let s = says(&mut f);
    // the quiz says its own line; the last returned line ("Added to your deck...") is said at the end with 7 s
    assert!(s[0].0.starts_with("I made ") && s[0].0.ends_with("fill-in-the-blank cards from \u{201c}Osmosis\u{201d}~ Ready?"), "{s:?}");
    assert_eq!(s[0].1, vec![("Quiz me".to_string(), "hub_ask:review flashcards".to_string()), ("Later".to_string(), String::new())]);
    assert!(s[1].0.starts_with("Added to your deck: \u{201c}osmosis\u{201d}"), "{s:?}");
    assert!(f.ema_active().iter().any(|e| e["text"] == "pack the bag"));
    let dl = zenko_desktop::plan::load_deadlines();
    assert!(dl.iter().any(|x| x["title"] == "Bio IA draft" && x["source"] == "browser" && x["date"] == "2026-12-03"));
    let cards = f.life_now()["cards"].as_array().unwrap().iter().filter(|c| c["from"] == "browser").count();
    assert!(cards >= 1);

    // deadline messages
    let r = f.zb_inbox_item(1000.0, &json!({"kind": "deadline", "text": "Chem test", "date": "2026-12-03"}), &mut |_| false).unwrap();
    assert_eq!(r.as_deref(), Some("Added to Upcoming: \u{201c}Chem test\u{201d} on Thu 3 Dec~"));
    let r = f.zb_inbox_item(1000.0, &json!({"kind": "deadline", "text": "Chem test"}), &mut |_| false).unwrap();
    assert_eq!(r.as_deref(), Some("I couldn't find a date in that. Try selecting the date too~"));
    let r = f.zb_inbox_item(1000.0, &json!({"kind": "summary", "text": "tiny"}), &mut |_| false).unwrap();
    assert_eq!(r.as_deref(), Some("There wasn't enough text there for me to summarise~"));
    let r = f.zb_inbox_item(1000.0, &json!({"kind": "card", "text": "nothing to split"}), &mut |_| false).unwrap();
    assert!(r.unwrap().starts_with("Select something like"));
    // a summary: "Reading ...", the chat line, then "Done~ ..." with Save to notes, which files it
    f.events.clear();
    let r = f.zb_inbox_item(1000.0, &json!({"kind": "summary", "title": "Osmosis", "text": page, "url": "u"}), &mut |_| false).unwrap();
    assert!(r.is_none());
    assert_eq!(says(&mut f)[0].0, "Reading \u{201c}Osmosis\u{201d}... give me a moment~");
    let id = f.zb.saves.last().unwrap().0.clone();
    std::fs::write(zenko_desktop::notes::nb_scratch(), "").unwrap();
    let mut got = String::new();
    f.zb_save(&id, &mut |b| { got = b.to_string(); true });
    assert!(got.starts_with("\nSummary: Osmosis\nKon! The key points of \u{201c}Osmosis\u{201d}:\n\u{2022} "), "{got}");
    assert!(got.ends_with("\n(from u)\n"));

    // the link file
    assert!(!f.z22_link_tick(1000.0, 0.0, false));
    let link: Value = serde_json::from_str(&std::fs::read_to_string(data.join("zenko_link.json")).unwrap()).unwrap();
    assert_eq!(link["app"], "zenko");
    assert_eq!(link["session"], json!({"on": false}));
    let keys: Vec<&str> = link.as_object().unwrap().keys().map(|k| k.as_str()).collect();
    assert_eq!(keys, ["app", "version", "time", "pid", "name", "deadlines", "today", "ema", "session", "ema_plan"]);
}
