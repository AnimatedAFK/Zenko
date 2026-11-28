//! zenko.py's distraction decisions (study video subject, subject classifier, site rules, window key) for a list of
//! window titles, against golden/distraction.json made by tools/gen_desktop_data.py.
use serde_json::Value;
use zenko_desktop::watch::{classify_subject, data, rule_verdict, study_video_subject, window_key};

#[test]
fn distraction_decisions_match_python() {
    let g: Value = serde_json::from_str(include_str!("../../../golden/distraction.json")).unwrap();
    let rules = &g["rules"];
    let mut bad = vec![];
    for c in g["cases"].as_array().unwrap() {
        let t = c["title"].as_str().unwrap();
        let stripped = data().tail.replace(t, "").to_string();
        let sv = study_video_subject(&stripped, &[]);
        let want_sv = c["sv"].as_str().map(String::from);
        if sv != want_sv {
            bad.push(format!("sv {t:?}: {sv:?} vs {want_sv:?}"));
        }
        let (s, a, b) = classify_subject(&t.to_lowercase(), &Value::Null);
        let w = &c["cls"];
        if (s.clone(), a, b) != (w[0].as_str().map(String::from), w[1].as_i64().unwrap(), w[2].as_i64().unwrap()) {
            bad.push(format!("cls {t:?}: {:?} vs {w}", (s, a, b)));
        }
        let r = rule_verdict(rules, t, "chrome.exe");
        let (wd, wi) = (c["rule"].as_str(), c["rule_i"].as_i64().unwrap());
        let got = (r.map(|x| x.0), r.map(|x| x.1 as i64).unwrap_or(-1));
        if got != (wd, wi) {
            bad.push(format!("rule {t:?}: {got:?} vs {wd:?} {wi}"));
        }
        let k = window_key(t);
        if k != c["key"].as_str().unwrap() {
            bad.push(format!("key {t:?}: {k:?} vs {}", c["key"]));
        }
    }
    assert!(bad.is_empty(), "{} mismatches:\n{}", bad.len(), bad.join("\n"));
}
