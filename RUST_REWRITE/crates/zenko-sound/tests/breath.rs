//! The breathing cues against golden/breath.json (zenko.py _breath_tone's bytes): every phase, length and volume.
use sha2::{Digest, Sha256};
use zenko_sound::breath_tone;

#[test]
fn breath_tones_match_zenko_py() {
    let g: Vec<serde_json::Value> = serde_json::from_str(include_str!("../../../golden/breath.json")).unwrap();
    let mut bad = vec![];
    for c in &g {
        let (k, s, v) = (c["kind"].as_str().unwrap(), c["secs"].as_f64().unwrap(), c["vol"].as_i64().unwrap() as i32);
        let b = breath_tone(k, s, v);
        let h: String = Sha256::digest(&b).iter().map(|x| format!("{:02x}", x)).collect();
        if b.len() as u64 != c["len"].as_u64().unwrap() || h != c["sha"].as_str().unwrap() {
            bad.push(format!("{k} {s} {v}"));
        }
    }
    assert!(bad.is_empty(), "{bad:?}");
}
