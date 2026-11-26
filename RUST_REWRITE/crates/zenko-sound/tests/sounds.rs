//! Every sound zenko.py synthesises must come out of Rust as the very same .wav bytes.
use sha2::{Digest, Sha256};

#[test]
fn every_wav_matches_python() {
    let p = concat!(env!("CARGO_MANIFEST_DIR"), "/../../golden/sounds.json");
    let cases: Vec<serde_json::Value> = serde_json::from_str(&std::fs::read_to_string(p).expect("golden/sounds.json")).unwrap();
    let mut bad = Vec::new();
    for c in &cases {
        let kind = c["kind"].as_str().unwrap();
        let vol = c["vol"].as_i64().unwrap() as i32;
        let pitch = c["pitch"].as_f64().unwrap();
        let got = zenko_sound::wav_bytes(kind, vol, pitch);
        let sha = got.as_ref().map(|b| Sha256::digest(b).iter().map(|x| format!("{:02x}", x)).collect::<String>()).unwrap_or_default();
        let len = got.as_ref().map(|b| b.len()).unwrap_or(0) as u64;
        assert_eq!(zenko_sound::sound_file_name(kind, vol, pitch), c["file"].as_str().unwrap());
        if sha != c["sha"].as_str().unwrap() || len != c["len"].as_u64().unwrap() {
            bad.push(format!("{kind} vol {vol} pitch {pitch}: len {len} vs {}", c["len"]));
        }
    }
    println!("{} sounds checked, {} different", cases.len(), bad.len());
    assert!(bad.is_empty(), "{}", bad.join("\n"));
}
