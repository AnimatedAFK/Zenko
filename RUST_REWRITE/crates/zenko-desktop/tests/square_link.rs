//! The Town square's link against zenko.py's SquareLink23 (golden/square_link.json): every streamed line's effect on
//! the messages (cleaned text, checked names, looks, repeats), the reports and the change count, square_visible with a
//! block list, and the exact URL and body of a post, a post with your look, and a report.
use serde_json::{Value, json};
use zenko_desktop::squarelink::{SQUARE_TOPIC, SquareLink};

#[test]
fn square_link_matches_zenko_py() {
    let g: Value = serde_json::from_str(&std::fs::read_to_string("../../golden/square_link.json").unwrap()).unwrap();
    let mut sq = SquareLink::new("https://ntfy.sh/", SQUARE_TOPIC);
    for (i, s) in g["steps"].as_array().unwrap().iter().enumerate() {
        sq.line(s["raw"].as_str().unwrap().as_bytes(), 1766000000.0);
        let msgs: Vec<Value> = sq.msgs.iter().map(|m| {
            let mut v = json!({"id": m.id, "uid": m.uid, "name": m.name, "text": m.text, "ts": m.ts});
            if let Some((y, mm, h)) = m.look {
                v["look"] = json!({"y": y, "m": mm, "h": h});
            }
            v
        }).collect();
        let want: Vec<Value> = s["msgs"].as_array().unwrap().iter().map(|m| {
            let mut m = m.clone();
            m["ts"] = json!(m["ts"].as_f64().unwrap());
            m
        }).collect();
        assert_eq!(msgs, want, "step {i} msgs");
        assert_eq!(sq.changed, s["changed"].as_i64().unwrap(), "step {i} changed");
        let reps: serde_json::Map<String, Value> = sq.reports.iter().map(|(k, v)| (k.clone(), json!(v.iter().collect::<Vec<_>>()))).collect();
        assert_eq!(Value::Object(reps), s["reports"], "step {i} reports");
    }
    let vis: Vec<&str> = sq.visible(&["u3".to_string()]).iter().map(|m| m.id.as_str()).collect::<Vec<_>>().into_iter()
        .map(|s| Box::leak(s.to_string().into_boxed_str()) as &str).collect();
    let want: Vec<&str> = g["visible"].as_array().unwrap().iter().map(|m| m["id"].as_str().unwrap()).collect();
    assert_eq!(vis, want);
    let p = g["posts"].as_array().unwrap();
    let got = |r: (String, String)| json!([r.0, r.1]);
    assert_eq!(got(sq.post_req("me1", "Kitsune", "hi", None)), p[0]);
    sq.look = Some((4, true, 0));
    assert_eq!(got(sq.post_req("me1", "Kitsune", "kon kon \u{e9}\u{2603}", None)), p[1]);
    assert_eq!(got(sq.post_req("me1", "", "", Some("a3"))), p[2]);
    assert_eq!(sq.since, "b5");
}
