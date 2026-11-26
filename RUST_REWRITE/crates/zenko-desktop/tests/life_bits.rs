//! The music tablet picture and its buttons, parse_clock, the weather URLs and geocode's pick, against
//! golden/life.json (made by tools/make_golden_life.py from zenko.py's own methods).
use serde_json::{Value, json};
use zenko_desktop::music::Player;
use zenko_desktop::slow::{forecast_url, geocode_pick, geocode_url, url_quote};
use zenko_desktop::tt::parse_clock;
use zenko_desktop::{Cfg, Companion};

#[test]
fn life_bits_match_zenko_py() {
    let g: Value = serde_json::from_str(include_str!("../../../golden/life.json")).unwrap();
    let d = std::env::temp_dir().join(format!("zk_lifebits_{}", std::process::id()));
    unsafe { std::env::set_var("APPDATA", &d) };
    let mut f = Companion::new(Cfg::from(json!({})), 1.0, 1000.0);
    for c in g["tablet"].as_array().unwrap() {
        let p = &c["player"];
        f.music.player = Some(Player { playing: p["playing"].as_bool().unwrap(), title: p["title"].as_str().unwrap().into(),
                                       artist: p["artist"].as_str().unwrap().into() });
        f.music.tab_flash = (c["flash"][0].as_str().unwrap().into(), c["flash"][1].as_f64().unwrap());
        let rows = f.tablet_rows(c["t"].as_f64().unwrap());
        let want: Vec<String> = c["rows"].as_array().unwrap().iter().map(|r| r.as_str().unwrap().to_string()).collect();
        assert_eq!(rows, want, "tablet for {p}");
    }
    f.music.player = Some(Player { playing: true, title: "x".into(), artist: String::new() });
    (f.z, f.colw, f.fox_y) = (4, 37, 21);
    f.sleeping = false;
    for h in g["hits"].as_array().unwrap() {
        let (x, y) = (h[0].as_i64().unwrap() as i32, h[1].as_i64().unwrap() as i32);
        assert_eq!(f.tablet_hit(1000.0, x, y), h[2].as_str(), "tablet_hit({x}, {y})");
    }
    for (s, want) in g["clocks"].as_object().unwrap() {
        assert_eq!(parse_clock(s).as_deref(), want.as_str(), "parse_clock({s:?})");
    }
    let urls: Vec<&str> = g["geo"]["urls"].as_array().unwrap().iter().map(|u| u.as_str().unwrap()).collect();
    let res = json!({"results": [
        {"name": "Paris", "country": "France", "country_code": "FR", "admin1": "Île-de-France", "latitude": 48.85341, "longitude": 2.3488},
        {"name": "Paris", "country": "United States", "country_code": "US", "admin1": "Texas", "latitude": 33.66094, "longitude": -95.55551}]});
    for (i, place) in ["Paris", "Paris, Texas", "Paris, us", "Paris, Germany"].iter().enumerate() {
        assert_eq!(geocode_url(place), urls[i]);
        assert_eq!(geocode_pick(place, &res).unwrap(), g["geo"][*place], "geocode({place})");
    }
    assert_eq!(forecast_url(&res["results"][1]), g["forecast_url"].as_str().unwrap());
    let q: Vec<String> = ["São Paulo", "New York", "a/b&c=d", "東京", "x-y_z.~"].iter().map(|s| url_quote(s)).collect();
    assert_eq!(json!(q), g["quote"]);
}

fn said(f: &mut Companion) -> Vec<String> {
    f.events.drain(..).filter_map(|e| if let zenko_desktop::Ev::Say { text, .. } = e { Some(text) } else { None }).collect()
}

#[test]
fn slow_tick_lines_and_visitor() {
    let d = std::env::temp_dir().join(format!("zk_slow_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    unsafe {
        std::env::set_var("APPDATA", &d);
        std::env::set_var("ZK_TZ_OFFSET", "0");
    }
    // 2025-11-26 07:30 UTC: morning, autumn
    zenko_desktop::set_fake_now(Some(1764142200.0));
    let mut f = Companion::new(Cfg::from(json!({"pet_needs": false, "random_events": false, "daily_checkin": false})), 1.0, 1000.0);
    f.life_startup();
    f.life_slow(1006.0);
    assert!(said(&mut f).is_empty(), "too soon after start for good morning");
    f.life_slow(1036.0);
    let s = said(&mut f);
    assert!(s.len() == 1 && s[0].contains("ood morning") || s[0].starts_with("Ohayou"), "{s:?}");
    f.life_slow(1066.0);
    assert!(said(&mut f).is_empty(), "only once a day");
    // a visiting friend: the friend badge, two lines, then she goes home after 13 s
    f.random_event(1100.0, Some("visitor"));
    assert!(f.slow.visitor.is_some());
    f.pump(1101.7);
    f.pump(1107.6);
    let s = said(&mut f);
    assert!(s.iter().any(|l| l.starts_with("Kon kon! My friend ")) && s.iter().any(|l| l.contains("has to go home")), "{s:?}");
    assert!(zenko_desktop::read_json("zenko_life.json")["badges"].get("friend").is_some());
    let _ = f.frame(1105.0, (0, 0), (0, 0));
    assert!(f.slow.visitor.is_some());
    let _ = f.frame(1113.5, (0, 0), (0, 0));
    assert!(f.slow.visitor.is_none());
    // autumn leaves fall around her
    for i in 0..400 {
        let _ = f.frame(1200.0 + i as f64 * 0.06, (0, 0), (0, 0));
    }
    assert_eq!(f.slow.wx.kind.as_deref(), Some("leaves"));
    assert!(!f.slow.wx.items.is_empty());
    f.slow.wx.code = Some(61);
    let _ = f.frame(1300.0, (0, 0), (0, 0));
    assert_eq!(f.slow.wx.kind.as_deref(), Some("rain"));
    // a haiku is remembered so she doesn't repeat it soon
    f.random_event(1400.0, Some("haiku"));
    assert_eq!(zenko_desktop::read_json("zenko_life.json")["haiku"].as_array().unwrap().len(), 1);
    zenko_desktop::set_fake_now(None);
}
