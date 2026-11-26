//! Every sprite frame drawn by Rust must match the frame zenko.py draws, cell for cell.
use flate2::read::GzDecoder;
use serde_json::Value;
use std::collections::BTreeMap;
use std::io::{BufRead, BufReader};
use zenko_art::{Grid, Look, curl, rig, sit, walk};

fn golden() -> Vec<Value> {
    let p = concat!(env!("CARGO_MANIFEST_DIR"), "/../../golden/sprites.jsonl.gz");
    let f = std::fs::File::open(p).expect("golden file (run tools/make_golden.py)");
    BufReader::new(GzDecoder::new(f))
        .lines()
        .map(|l| serde_json::from_str(&l.unwrap()).unwrap())
        .collect()
}

fn strs(v: &Value) -> Vec<String> {
    v.as_array()
        .unwrap()
        .iter()
        .map(|s| s.as_str().unwrap().to_string())
        .collect()
}

pub fn render(c: &Value) -> Grid {
    let a = &c["args"];
    let lk = Look {
        ears: c["look"]["ears"].as_str().unwrap().into(),
        blush: c["look"]["blush"].as_str().unwrap().into(),
    };
    let i = |k: &str| a[k].as_i64().unwrap() as i32;
    let b = |k: &str| a[k].as_bool().unwrap();
    let s = |k: &str| a[k].as_str().unwrap().to_string();
    let acc_s = if a.get("acc").is_some() {
        strs(&a["acc"])
    } else {
        vec![]
    };
    let acc: Vec<&str> = acc_s.iter().map(|x| x.as_str()).collect();
    match c["fn"].as_str().unwrap() {
        "build_tails" => zenko_art::core::build_tails(i("n"), i("frame"), i("frames"), b("shadow")),
        "build_frame" => {
            let lv = a["look"].as_array().map(|v| {
                let q: Vec<i32> = v.iter().map(|x| x.as_i64().unwrap() as i32).collect();
                (q[0], q[1], q[2], q[3])
            });
            sit::build_frame(&s("state"), i("frame"), lv, &acc, i("hx"), i("ear"), &lk)
        }
        "build_walk4" => walk::build_walk4(i("frame"), i("n"), i("frames"), b("blink"), b("left")),
        "build_walk34" => walk::build_walk34(
            i("frame"),
            i("n"),
            i("frames"),
            b("blink"),
            b("left"),
            &s("look"),
            &lk,
        ),
        "rig_rows" => rig::rig_rows(&s("state"), i("n"), &acc, &lk),
        "build_curl" => curl::build_curl(i("frame"), &acc, i("n")),
        "foxfire_rows" => curl::foxfire_rows(i("frame")),
        "z19_sit_extra" => {
            let base = sit::build_frame(&s("state"), i("frame") % 2, None, &acc, 0, 0, &lk);
            sit::sit_extra(&base, &s("kind"), i("frame"))
        }
        f => panic!("unknown fn {f}"),
    }
}

#[test]
fn every_frame_matches_python() {
    let cases = golden();
    let mut per: BTreeMap<String, (usize, usize)> = BTreeMap::new();
    let mut first_bad: Vec<String> = vec![];
    for c in &cases {
        let want = strs(&c["rows"]);
        let got = render(c).rows();
        let e = per
            .entry(c["fn"].as_str().unwrap().to_string())
            .or_default();
        e.0 += 1;
        if got != want {
            e.1 += 1;
            let mut diff = 0;
            let mut where_ = None;
            for (y, (g, w)) in got.iter().zip(want.iter()).enumerate() {
                for (x, (a, b)) in g.bytes().zip(w.bytes()).enumerate() {
                    if a != b {
                        diff += 1;
                        if where_.is_none() {
                            where_ = Some((x, y, a as char, b as char));
                        }
                    }
                }
            }
            if first_bad.len() < 12 {
                first_bad.push(format!(
                    "{} {} look={}: {} cells differ, first at {:?} (rust, python)",
                    c["fn"], c["args"], c["look"], diff, where_
                ));
            }
        }
    }
    for (k, (n, bad)) in &per {
        println!("{k:16} {n:4} frames, {bad} differ");
    }
    for l in &first_bad {
        println!("  {l}");
    }
    assert!(first_bad.is_empty(), "some frames differ from zenko.py");
}
