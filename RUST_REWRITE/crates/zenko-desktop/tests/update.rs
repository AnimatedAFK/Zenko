//! The updater's pure parts (update.rs) against zenko.py (golden/update.json from tools/make_golden_update.py):
//! write_lnk's bytes, version_newer, update_check on GitHub / latest.json answers, the exe check, and
//! update_download with a pretend server (checksum, cut-off download, the file check).

use serde_json::{Value, json};
use std::sync::Arc;
use zenko_desktop::update::*;
use zenko_desktop::web;

fn golden() -> Value {
    serde_json::from_str(&std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../../golden/update.json")).unwrap()).unwrap()
}

#[test]
fn lnk_and_versions_match_zenko_py() {
    let g = golden();
    for c in g["lnk"].as_array().unwrap() {
        let a: Vec<&str> = c["args"].as_array().unwrap().iter().map(|v| v.as_str().unwrap()).collect();
        let want: Vec<u8> = c["bytes"].as_array().unwrap().iter().map(|v| v.as_u64().unwrap() as u8).collect();
        assert_eq!(lnk_bytes(a[0], a[1], a[2], a[3], a[4], a[0].as_bytes()), want, "{a:?}");
    }
    for c in g["newer"].as_array().unwrap() {
        assert_eq!(version_newer(c[0].as_str().unwrap(), c[1].as_str().unwrap()), c[2].as_bool().unwrap(), "{c}");
    }
    for c in g["tuple"].as_array().unwrap() {
        let want: Vec<u64> = c[1].as_array().unwrap().iter().map(|v| v.as_u64().unwrap()).collect();
        assert_eq!(version_tuple(c[0].as_str().unwrap()), want);
    }
    let dir = std::env::temp_dir().join(format!("zk_upd_ok_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    for c in g["file_ok"].as_array().unwrap() {
        let n = c[0].as_u64().unwrap() as usize;
        let mut b = c[1].as_str().unwrap().as_bytes().to_vec();
        b.resize(n, 0);
        let p = dir.join("f.exe");
        std::fs::write(&p, b).unwrap();
        let (ok, why) = exe_ok(&p);
        assert_eq!(json!([ok, why]), c[2]);
    }
}

#[test]
fn update_check_matches_zenko_py() {
    let _g = web::test_lock();
    let g = golden();
    for c in g["check"].as_array().unwrap() {
        let ans = c["ans"].clone();
        web::set_fetch(Arc::new(move |r, chunk| {
            let v = &ans[r.url.as_str()];
            assert!(!v.is_null(), "unexpected {}", r.url);
            let b = if let Some(s) = v.as_str() { s.as_bytes().to_vec() } else { v.to_string().into_bytes() };
            chunk(&b, b.len() as u64);
            Ok(200)
        }));
        let r = update_check(c["source"].as_str().unwrap(), c["prefer"].as_str(), "zenko.exe", true, 15.0);
        match c.get("err") {
            Some(e) => assert_eq!(r.unwrap_err(), e.as_str().unwrap(), "{}", c["name"]),
            None => assert_eq!(r.unwrap().unwrap_or(Value::Null), c["out"], "{}", c["name"]),
        }
    }
    // the Rust app: only its own exe, never zenko.py or zenko.py's Zenko.exe
    let rel = g["check"][0]["ans"].clone();
    web::set_fetch(Arc::new(move |r, chunk| {
        let v = &rel[r.url.as_str()];
        let b = if let Some(s) = v.as_str() { s.as_bytes().to_vec() } else { v.to_string().into_bytes() };
        chunk(&b, b.len() as u64);
        Ok(if v.is_null() { 404 } else { 200 })
    }));
    let r = update_check("AnimatedAFK/Zenko", Some("exe"), "zenkorust.exe", false, 15.0).unwrap().unwrap();
    assert_eq!((r["name"].as_str(), r["kind"].as_str(), r["url"].as_str()), (Some("ZenkoRust.exe"), Some("exe"), Some("https://dl/ZenkoRust.exe")));
    assert_eq!(update_check("someone/else", Some("exe"), "zenkorust.exe", false, 15.0).unwrap_err(),
               "I couldn't find any releases there yet. Check the name in Settings > Updates.");
}

#[test]
fn download_checks_the_file() {
    let _g = web::test_lock();
    let dir = std::env::temp_dir().join(format!("zk_upd_dl_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let mut exe = b"MZ".to_vec();
    exe.resize(3 * 1024 * 1024, 7);
    let e2 = exe.clone();
    web::set_fetch(Arc::new(move |r, chunk| {
        if r.url.ends_with("/cut") {
            chunk(&e2[..1000], e2.len() as u64);
            return Err("<urlopen error timed out>".into());
        }
        let b: &[u8] = if r.url.ends_with("/small") { b"MZ tiny" } else { &e2 };
        for part in b.chunks(65536) {
            if !chunk(part, b.len() as u64) {
                break;
            }
        }
        Ok(200)
    }));
    let mut h = web::Sha256::new();
    h.update(&exe);
    let sha = h.hex();
    let dest = dir.join("ZenkoRust.new.exe");
    let mut last = 0.0;
    update_download(&json!({"url": "https://dl/ok", "sha256": sha, "size": 0}), &dest, &mut |f| last = f).unwrap();
    assert_eq!(std::fs::read(&dest).unwrap(), exe);
    assert!((last - 1.0).abs() < 1e-9);
    assert_eq!(update_download(&json!({"url": "https://dl/ok", "sha256": "00"}), &dest, &mut |_| {}).unwrap_err(),
               "The download didn't match its checksum, so I threw it away. Try again later.");
    assert_eq!(update_download(&json!({"url": "https://dl/small", "sha256": ""}), &dest, &mut |_| {}).unwrap_err(), "That isn't a Zenko.exe.");
    assert!(update_download(&json!({"url": "https://dl/cut"}), &dest, &mut |_| {}).unwrap_err().starts_with("The download didn't finish"));
    assert!(!dir.join("ZenkoRust.new.exe.part").exists());
}
