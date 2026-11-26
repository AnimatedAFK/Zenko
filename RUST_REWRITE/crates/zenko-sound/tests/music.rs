//! The three background-music loops and their volume-3 copies against golden/music.json (zenko.py's own bytes).
use sha2::{Digest, Sha256};
use zenko_sound::music::{music_wav, music_wav_vol};

fn sha(b: &[u8]) -> String {
    Sha256::digest(b).iter().map(|x| format!("{:02x}", x)).collect()
}

#[test]
fn music_matches_zenko_py() {
    let g: serde_json::Value = serde_json::from_str(include_str!("../../../golden/music.json")).unwrap();
    for tr in ["garden", "bamboo", "temple"] {
        let w = &g[tr];
        let b = music_wav(tr);
        assert_eq!(b.len() as u64, w["bytes"].as_u64().unwrap(), "{tr} length");
        let head: Vec<u64> = b[44..84].iter().map(|&x| x as u64).collect();
        let want: Vec<u64> = w["head"].as_array().unwrap().iter().map(|x| x.as_u64().unwrap()).collect();
        assert_eq!(head, want, "{tr} first samples");
        assert_eq!(sha(&b), w["sha256"].as_str().unwrap(), "{tr} bytes");
        assert_eq!(sha(&music_wav_vol(&b, 3)), w["vol3_sha256"].as_str().unwrap(), "{tr} volume 3");
    }
}
