//! The Spotify account (spotify.rs) against zenko.py's class Spotify with a pretend Spotify: the token refresh and
//! its save, the API's friendly errors (401 retry, NO_DEVICE, 403, 429), waking a device, now(), play_query's
//! playlist match and search, do()'s sentences, find_preset, spotify_link_to_uri, the PKCE challenge and urlencode.

use serde_json::{Value, json};
use std::sync::{Arc, Mutex};
use zenko_desktop::spotify::*;
use zenko_desktop::web;

fn no_sleep(_: f64) {}

#[test]
fn helpers_match_zenko_py() {
    // hashlib / base64: the RFC 7636 example
    assert_eq!(challenge("dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk"), "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM");
    assert_eq!(token_urlsafe(64).len(), 86);
    assert_eq!(urlencode(&[("q", "lo fi & beats".into()), ("type", "track".into())]), "q=lo+fi+%26+beats&type=track");
    assert_eq!(link_to_uri("https://open.spotify.com/intl-de/playlist/37i9dQZF1DX8Uebhn9wzrS?si=x").as_deref(),
               Some("spotify:playlist:37i9dQZF1DX8Uebhn9wzrS"));
    assert_eq!(link_to_uri("spotify:track:abc").as_deref(), Some("spotify:track:abc"));
    assert_eq!(link_to_uri("lofi"), None);
    let ps = vec![json!({"name": "Focus", "value": "lofi", "kind": "playlist"}), json!({"name": "My favorite song", "value": ""}),
                  json!({"name": "Gym", "value": "spotify:playlist:x"})];
    assert_eq!(find_preset(&ps, "focus"), Some(ps[0].clone()));
    assert_eq!(find_preset(&ps, "my favourite song"), None); // no value
    assert_eq!(find_preset(&ps, "#3"), Some(ps[2].clone()));
    assert_eq!(find_preset(&ps, "number 2"), None);
    assert_eq!(find_preset(&ps, "jazz"), None);
    assert_eq!(web::unquote_plus("a+b%20c%3F"), "a b c?");
}

#[test]
fn account_with_a_pretend_spotify() {
    let _g = web::test_lock();
    let dir = std::env::temp_dir().join(format!("zk_spotify_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    unsafe { std::env::set_var("APPDATA", &dir) };
    std::fs::create_dir_all(dir.join("FoxBuddy")).unwrap();
    std::fs::write(token_path(), r#"{"access_token":"old","refresh_token":"r1","expires_at":0}"#).unwrap();
    let log = Arc::new(Mutex::new(Vec::<String>::new()));
    let state = Arc::new(Mutex::new(json!({"devices": [], "playing": null, "first401": true})));
    let (l2, s2) = (log.clone(), state.clone());
    web::set_fetch(Arc::new(move |r, chunk| {
        let body = String::from_utf8_lossy(&r.body).into_owned();
        l2.lock().unwrap().push(format!("{} {} {}", r.method, r.url, body));
        let mut st = s2.lock().unwrap();
        let (code, out): (u32, Value) = if r.url.ends_with("/api/token") {
            assert!(r.headers.contains("application/x-www-form-urlencoded"));
            // the first refresh uses the saved r1; after the 401 it refreshes again with the r2 it was given
            assert!(body.contains("grant_type=refresh_token&refresh_token=r1&client_id=cid")
                    || body.contains("grant_type=refresh_token&refresh_token=r2&client_id=cid"), "{body}");
            (200, json!({"access_token": "new", "expires_in": 3600, "refresh_token": "r2"}))
        } else {
            assert!(r.headers.contains("Authorization: Bearer new"), "{}", r.headers);
            let path = r.url.split("/v1").nth(1).unwrap().to_string();
            if path == "/me/player" && r.method == "GET" {
                if st["first401"] == true {
                    st["first401"] = json!(false);
                    (401, json!({"error": {"status": 401, "message": "expired"}}))
                } else if st["playing"].is_null() { (204, Value::Null) } else {
                    (200, json!({"is_playing": true, "item": {"name": "Lofi Song", "artists": [{"name": "A"}, {"name": "B"}], "uri": "spotify:track:1",
                                  "duration_ms": 1000}, "device": {"volume_percent": 40}, "shuffle_state": false, "progress_ms": 5}))
                }
            } else if path == "/me/player/devices" {
                (200, json!({"devices": st["devices"]}))
            } else if path == "/me/player" && r.method == "PUT" {
                st["active"] = json!(true);
                (204, Value::Null)
            } else if path.starts_with("/me/player/play") || path.starts_with("/me/player/next") {
                if st["active"] != true { (404, json!({"error": {"status": 404, "message": "Player command failed: No active device found", "reason": "NO_ACTIVE_DEVICE"}})) }
                else { st["playing"] = json!(true); (204, Value::Null) }
            } else if path.starts_with("/me/player/pause") {
                (403, json!({"error": {"status": 403, "message": "Player command failed: Restriction violated", "reason": "UNKNOWN"}}))
            } else if path.starts_with("/me/player/volume") {
                (429, json!({"error": {"status": 429}}))
            } else if path.starts_with("/me/playlists") {
                (200, json!({"items": [{"name": "Study Lofi", "uri": "spotify:playlist:sl"}, null, {"name": "Gym", "uri": "spotify:playlist:g"}]}))
            } else if path.starts_with("/search") {
                (200, json!({"tracks": {"items": [{"name": "Clair de Lune", "uri": "spotify:track:cdl", "artists": [{"name": "Debussy"}]}]}}))
            } else { (500, json!({"error": {"message": "boom"}})) }
        };
        let b = if out.is_null() { vec![] } else { out.to_string().into_bytes() };
        chunk(&b, b.len() as u64);
        Ok(code)
    }));
    unsafe { std::env::set_var("FOX_SPOTIFY_BASE", "http://sp.test") };
    let mut sp = Spotify::new("cid");
    sp.sleep = no_sleep;
    assert!(sp.connected());
    // the 401 makes it refresh once and ask again; nothing on: None
    assert_eq!(sp.now().unwrap(), None);
    let saved: Value = serde_json::from_str(&std::fs::read_to_string(token_path()).unwrap()).unwrap();
    assert_eq!((saved["access_token"].as_str(), saved["refresh_token"].as_str()), (Some("new"), Some("r2")));
    assert_eq!(sp.do_("now", None).unwrap(), "Nothing is playing on Spotify right now.");
    // no device anywhere (and no app to open)
    assert_eq!(sp.do_("next", None).unwrap_err(), "No Spotify device is open. Open the Spotify app on this PC (or your phone) once, then try again.");
    // a device: moved there, then played; a close playlist name wins
    state.lock().unwrap()["devices"] = json!([{"id": "phone", "type": "Smartphone", "is_active": true}, {"id": "pc", "type": "Computer", "is_active": false}]);
    assert_eq!(sp.do_("play", Some("study lofi")).unwrap(), "Playing your playlist \u{201c}Study Lofi\u{201d}.");
    assert!(log.lock().unwrap().iter().any(|l| l.contains("PUT http://sp.test/v1/me/player {\"device_ids\":[\"pc\"],\"play\":false}")));
    assert_eq!(sp.do_("play", Some("clair de lune")).unwrap(), "Playing \u{201c}Clair de Lune\u{201d} by Debussy.");
    assert!(log.lock().unwrap().iter().any(|l| l.contains("/search?q=clair+de+lune&type=track&limit=5")));
    assert_eq!(sp.do_("now", None).unwrap(), "Now playing: \u{201c}Lofi Song\u{201d} by A, B.");
    assert_eq!(sp.now().unwrap().unwrap().volume, Some(40));
    // 403 while playing: the friendly sentence
    assert_eq!(sp.do_("pause", None).unwrap_err(), "Spotify wouldn't do that right now (Player command failed: Restriction violated). If the \
                music is on another device (phone, speaker), control it there or switch it to this PC.");
    assert_eq!(sp.do_("volume", Some("140")).unwrap_err(), "Spotify asked me to slow down. Try again in a minute.");
    assert!(log.lock().unwrap().iter().any(|l| l.contains("/me/player/volume?volume_percent=100")));
    assert_eq!(sp.do_("dance", None).unwrap(), "I don't know that music command.");
    assert_eq!(sp.play_preset(&json!({"name": "Gym", "value": "https://open.spotify.com/track/abc", "kind": "playlist"})).unwrap(),
               "Playing your \u{201c}Gym\u{201d} preset.");
    assert!(log.lock().unwrap().last().unwrap().contains("{\"uris\":[\"spotify:track:abc\"]}"));
    sp.forget();
    assert!(!sp.connected() && !token_path().exists());
    assert_eq!(sp.do_("now", None).unwrap_err(), "Spotify isn't connected yet (Settings > Spotify > Connect).");
    unsafe { std::env::remove_var("FOX_SPOTIFY_BASE") };
}

#[test]
fn sign_in_catches_the_reply() {
    let _g = web::test_lock();
    unsafe { std::env::set_var("FOX_SPOTIFY_BASE", "http://sp.test") };
    web::set_fetch(Arc::new(|r, chunk| {
        let body = String::from_utf8_lossy(&r.body).into_owned();
        assert!(body.starts_with("grant_type=authorization_code&code=CODE1&redirect_uri=http%3A%2F%2F127.0.0.1%3A8765%2Fcallback&client_id=cid&code_verifier="), "{body}");
        let b = br#"{"access_token":"a","refresh_token":"r","expires_in":3600}"#;
        chunk(b, b.len() as u64);
        Ok(200)
    }));
    let dir = std::env::temp_dir().join(format!("zk_spotify_login_{}", std::process::id()));
    unsafe { std::env::set_var("APPDATA", &dir) };
    let mut sp = Spotify::new("cid");
    let opened = Arc::new(Mutex::new(String::new()));
    let o2 = opened.clone();
    let res = sp.login(&move |u| {
        *o2.lock().unwrap() = u.to_string();
        let state = u.split("state=").nth(1).unwrap().to_string();
        std::thread::spawn(move || {
            use std::io::{Read, Write};
            std::thread::sleep(std::time::Duration::from_millis(150));
            let mut s = std::net::TcpStream::connect(("127.0.0.1", PORT)).unwrap();
            s.write_all(b"GET /favicon.ico HTTP/1.1\r\n\r\n").unwrap();
            let mut s = std::net::TcpStream::connect(("127.0.0.1", PORT)).unwrap();
            s.write_all(format!("GET /callback?code=CODE1&state={state} HTTP/1.1\r\nHost: x\r\n\r\n").as_bytes()).unwrap();
            let mut out = String::new();
            let _ = s.read_to_string(&mut out);
            assert!(out.contains("Zenko is connected to Spotify!"));
        });
    }, 10.0);
    assert!(res.is_ok(), "{res:?}");
    let u = opened.lock().unwrap().clone();
    assert!(u.starts_with("http://sp.test/authorize?client_id=cid&response_type=code&redirect_uri=http%3A%2F%2F127.0.0.1%3A8765%2Fcallback&code_challenge_method=S256&code_challenge="), "{u}");
    assert!(u.contains("&scope=user-read-playback-state+user-modify-playback-state+"));
    assert!(sp.connected());
    let mut sp2 = Spotify::new("");
    assert_eq!(sp2.login(&|_| {}, 1.0).unwrap_err(), "Paste your Spotify Client ID in Settings first.");
    unsafe { std::env::remove_var("FOX_SPOTIFY_BASE") };
}
