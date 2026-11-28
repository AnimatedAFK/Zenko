//! fishart::fish_rgba / fish_pattern against zenko.py (golden/fish.json from tools/make_golden_fish.py): every fish
//! at book, pond (turned, squashed) and tiny sizes, one hash per pixel row.

use serde_json::Value;
use zenko_desktop::fishart::{fish_by_id, fish_pattern, fish_rgba};

fn fnv(b: &[u8]) -> String {
    let mut h: u64 = 0xcbf29ce484222325;
    for &x in b {
        h = (h ^ x as u64).wrapping_mul(0x100000001b3);
    }
    format!("{h:016x}")
}

#[test]
fn fish_match_python() {
    let g: Vec<Value> = serde_json::from_str(include_str!("../../../golden/fish.json")).unwrap();
    let mut bad = Vec::new();
    for c in &g {
        let fid = c["fid"].as_str().unwrap();
        let seed = c["seed"].as_i64().unwrap();
        let pid = fish_by_id(fid).unwrap()[5].as_str().unwrap();
        assert_eq!(fish_pattern(pid, seed).blobs.len() as i64, c["blobs"].as_i64().unwrap(), "blobs {fid} {seed}");
        let (w, h, px) = fish_rgba(fid, seed, c["L"].as_i64().unwrap(), c["angle"].as_f64().unwrap(), c["squash"].as_f64().unwrap());
        assert_eq!((w as i64, h as i64), (c["w"].as_i64().unwrap(), c["h"].as_i64().unwrap()), "size {fid}");
        let rows = c["rows"].as_array().unwrap();
        let diff: Vec<usize> = (0..h).filter(|&y| {
            let b: Vec<u8> = px[y * w..(y + 1) * w].iter().flatten().copied().collect();
            fnv(&b) != rows[y].as_str().unwrap()
        }).collect();
        if !diff.is_empty() {
            bad.push(format!("{fid} seed {seed} L {}: {} rows, first {:?}", c["L"], diff.len(), &diff[..diff.len().min(5)]));
        }
    }
    assert!(bad.is_empty(), "{} cases differ:\n{}", bad.len(), bad.join("\n"));
}
