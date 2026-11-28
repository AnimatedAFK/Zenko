//! The Bible's network and disk layer (biblefs.rs / web.rs) against what zenko.py does with the same answers: a
//! chapter from bolls.life cached on disk, the KJV fallback, a STEPBible section split into chapter files, the
//! format check, and the WebError messages.

use serde_json::json;
use std::sync::{Arc, Mutex};
use zenko_desktop::biblefs::*;
use zenko_desktop::web;

#[test]
fn bible_network_and_disk() {
    let _g = web::test_lock();
    let dir = std::env::temp_dir().join(format!("zk_biblefs_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    unsafe { std::env::set_var("APPDATA", &dir) };
    reset();
    let seen = Arc::new(Mutex::new(vec![]));
    let s2 = seen.clone();
    let section = {
        let mut t = String::from("header line\n");
        for (v, w) in [(1, "Ἐν (En)\tin\tG1722=PREP\tἐν=in"), (2, "οὗτος (houtos)\tthis\tG3778=D-NSM\tοὗτος=this")] {
            t.push_str(&format!("Jhn.1.{v}#01=NKO\t{w}\n"));
        }
        t.push_str("Jhn.2.1#01=NKO\tΚαὶ (Kai)\tand\tG2532=CONJ\tκαί=and\n");
        t.push_str(&" ".repeat(100000));
        t
    };
    web::set_fetch(Arc::new(move |r, chunk| {
        s2.lock().unwrap().push(r.url.clone());
        let body: Vec<u8> = if r.url.contains("/get-text/NKJV/43/3/") {
            json!([{"verse": 16, "text": "For God so <i>loved</i> the world"}, {"verse": 1, "text": "There was a man&nbsp;of the Pharisees"}])
                .to_string().into_bytes()
        } else if r.url.contains("TAGNT%20Mat-Jhn") {
            section.clone().into_bytes()
        } else if r.url.contains("api.github.com") {
            return Err("HTTP Error 403".into());
        } else {
            return Err("<urlopen error [Errno 11001] getaddrinfo failed>".into());
        };
        let n = body.len() as u64;
        chunk(&body, n);
        Ok(200)
    }));
    // a chapter: fetched, sorted, cleaned, saved; then read from disk without the network
    let (vs, used) = bible_chapter(42, 3, "NKJV", true).unwrap();
    assert_eq!(used, "NKJV");
    assert_eq!(vs, vec![(1, "There was a man of the Pharisees".to_string()), (16, "For God so loved the world".to_string())]);
    let n = seen.lock().unwrap().len();
    assert_eq!(bible_chapter(42, 3, "NKJV", false).unwrap().0.len(), 2);
    assert_eq!(seen.lock().unwrap().len(), n, "the second read comes from disk");
    // nothing answers: NKJV falls back to the KJV, and that fails as zenko.py words it
    assert_eq!(bible_chapter(0, 1, "NKJV", true).unwrap_err(),
               "I couldn't fetch Genesis 1 (WebError). The Bible sites may be blocked on this network.");
    assert_eq!(bible_chapter(0, 1, "WEB", false).unwrap_err(), "This chapter isn't saved on this PC yet, and I'm offline.");
    assert!(seen.lock().unwrap().iter().any(|u| u == "https://bible-api.com/Genesis+1?translation=kjv"));
    // the STEPBible listing is refused: the published names are used (and not cached)
    let ix = step_index();
    assert_eq!(ix["guessed"], json!(true));
    assert_eq!(step_files_for(42), vec![guess("TAGNT")[0].clone()]);
    assert_eq!(step_files_for(19).len(), 1);
    // John 1:2 downloads the Mat-Jhn section and splits it into chapters
    let w = bible_original(42, 1, 2, true, None).unwrap();
    assert_eq!(w.len(), 1);
    assert_eq!(w[0]["w"], "οὗτος");
    assert!(orig_path(42, 2).exists());
    assert_eq!(step_have_all().0, 2);
    assert_eq!(std::fs::read_to_string(bible_dir().join("orig/format.txt")).unwrap(), "2");
    // Acts isn't in that section, and its own file can't be fetched
    let e = bible_original(43, 1, 1, true, None).unwrap_err();
    assert!(e.starts_with("The download of the original-language text didn't finish (URLError)."), "{e}");
    assert_eq!(bible_original(43, 1, 1, false, None).unwrap_err(), "The original text for this book isn't on this PC yet.");
    // web.rs's messages
    assert_eq!(web::http_get("https://api.github.com/x", 5.0).unwrap_err(),
               "The website answered with an error (403). Try again in a moment.");
    assert_eq!(web::http_get("https://nowhere/", 5.0).unwrap_err(), web::E_NET);
    let mut h = web::Sha256::new();
    h.update(b"abc");
    assert_eq!(h.hex(), "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad");
    assert_eq!(web::quote("Translators Amalgamated OT+NT/TAGNT", "/"), "Translators%20Amalgamated%20OT%2BNT/TAGNT");
    let _ = std::fs::remove_dir_all(&dir);
}
