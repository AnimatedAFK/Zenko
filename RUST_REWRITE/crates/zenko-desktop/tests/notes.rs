//! The Notes page's filing against zenko.py (golden/notes.json): every line's subject scores, nb_split on mixed notes
//! with three different life files (learned words, renamed subjects), nb_safe, nb_match_subject, looks_french,
//! fr_vocab_search and nb_import_plan on a little folder tree.
use serde_json::{Value, json};
use zenko_desktop::notes::*;

#[test]
fn notes_filing_matches_zenko_py() {
    let g: Value = serde_json::from_str(&std::fs::read_to_string("../../golden/notes.json").unwrap()).unwrap();
    let dir = std::env::temp_dir().join(format!("zk_notes_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("FoxBuddy")).unwrap();
    unsafe { std::env::set_var("APPDATA", &dir) };
    let lives = g["lives"].as_array().unwrap();
    let mut bad = vec![];
    for s in g["scores"].as_array().unwrap() {
        let life = &lives[s["life"].as_u64().unwrap() as usize];
        let (sc, al) = nb_line_scores(s["line"].as_str().unwrap(), life);
        let mine: serde_json::Map<String, Value> = sc.iter().map(|(k, v)| (k.clone(), json!(v))).collect();
        if Value::Object(mine.clone()) != s["sc"] {
            bad.push(format!("scores {:?}: {} vs {}", s["line"], s["sc"], Value::Object(mine)));
        }
        for (k, v) in s["alias"].as_object().unwrap() {
            if al.get(k).copied() != v.as_bool() {
                bad.push(format!("alias {:?} {k}", s["line"]));
            }
        }
    }
    for s in g["split"].as_array().unwrap() {
        let life = &lives[s["life"].as_u64().unwrap() as usize];
        let parts = nb_split(s["text"].as_str().unwrap(), life);
        let mine = json!(parts.iter().map(|(a, b)| json!([a, b])).collect::<Vec<_>>());
        if mine != s["parts"] {
            bad.push(format!("split {:?} (life {}): {} vs {}", s["text"], s["life"], s["parts"], mine));
        }
    }
    for s in g["safe"].as_array().unwrap() {
        assert_eq!(nb_safe(s[0].as_str().unwrap()), s[1].as_str().unwrap(), "nb_safe {}", s[0]);
    }
    for s in g["match"].as_array().unwrap() {
        let n = s[0].as_str().unwrap();
        assert_eq!(json!(nb_match_subject(n, &json!({}))), s[1], "match {n}");
        assert_eq!(json!(nb_match_subject(n, &lives[2])), s[2], "match {n} (renamed)");
    }
    for s in g["french"].as_array().unwrap() {
        assert_eq!(looks_french(s[0].as_str().unwrap()), s[1].as_bool().unwrap(), "looks_french {}", s[0]);
    }
    for s in g["vocab"].as_array().unwrap() {
        let v = fr_vocab_search(s[0].as_str().unwrap(), 30);
        assert_eq!(json!(v.iter().map(|x| json!([x.0, x.1, x.2])).collect::<Vec<_>>()), s[1], "fr_vocab_search {}", s[0]);
    }
    let src = std::env::temp_dir().join(format!("zk_notes_src_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&src);
    for rel in ["Bio/cells.md", "Bio/sub/dna.txt", "Chemistry notes/acids.md", "misc/todo.txt", "French.md", "photo.png",
                ".hidden/x.md", "desktop.ini", "Maths/calc/limits.md", "readme.txt"] {
        let p = src.join("Export").join(rel);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(&p, rel).unwrap();
    }
    for c in g["plan"].as_array().unwrap() {
        let top = c["top"].as_str().unwrap();
        let mut plan: Vec<Value> = nb_import_plan(&src.join(top), &json!({})).into_iter().map(|(a, b)| {
            json!([a.strip_prefix(&src).unwrap().to_string_lossy(), b.strip_prefix(nb_dir()).unwrap().to_string_lossy()])
        }).collect();
        plan.sort_by_key(|v| v.to_string());
        let mut gp = c["plan"].as_array().unwrap().clone();
        gp.sort_by_key(|v| v.to_string());
        assert_eq!(plan, gp, "plan {top}");
    }
    for b in &bad {
        eprintln!("{b}");
    }
    assert!(bad.is_empty(), "{} differences", bad.len());
}
