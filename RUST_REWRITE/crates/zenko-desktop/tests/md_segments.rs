//! zenko.py's Markdown / LaTeX / chemistry formatter against golden/md_segments.json
use serde_json::Value;
use zenko_desktop::mdtext::{chem_segments, md_plain, md_segments, tex_segments, Seg};

fn want(v: &Value) -> Vec<Seg> {
    v.as_array().unwrap().iter().map(|s| (s[0].as_str().unwrap().to_string(),
        s[1].as_array().unwrap().iter().map(|t| t.as_str().unwrap().to_string()).collect())).collect()
}

#[test]
fn md_segments_match_zenko_py() {
    let g: Value = serde_json::from_str(&std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../../golden/md_segments.json")).unwrap()).unwrap();
    let mut bad = 0;
    for c in g["md"].as_array().unwrap() {
        let t = c["text"].as_str().unwrap();
        let got = md_segments(t, &["fox"]);
        if got != want(&c["segs"]) {
            bad += 1;
            eprintln!("md {t:?}\n got  {got:?}\n want {:?}", want(&c["segs"]));
        }
        if md_plain(t) != c["plain"].as_str().unwrap() {
            bad += 1;
            eprintln!("plain {t:?}\n got  {:?}\n want {:?}", md_plain(t), c["plain"]);
        }
    }
    for c in g["tex"].as_array().unwrap() {
        let t = c["text"].as_str().unwrap();
        let got = tex_segments(t, &["md_math".to_string()]);
        if got != want(&c["segs"]) {
            bad += 1;
            eprintln!("tex {t:?}\n got  {got:?}\n want {:?}", want(&c["segs"]));
        }
    }
    for c in g["chem"].as_array().unwrap() {
        let t = c["text"].as_str().unwrap();
        let got = chem_segments(t, &[]);
        if got != want(&c["segs"]) {
            bad += 1;
            eprintln!("chem {t:?}\n got  {got:?}\n want {:?}", want(&c["segs"]));
        }
    }
    assert_eq!(bad, 0);
}
