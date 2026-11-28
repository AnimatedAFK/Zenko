//! The music tablet picture and its buttons, parse_clock, the weather URLs and geocode's pick, against
//! golden/life.json (made by tools/make_golden_life.py from zenko.py's own methods).
use serde_json::{Value, json};
use zenko_desktop::music::Player;
use zenko_desktop::slow::{forecast_url, geocode_pick, geocode_url, url_quote};
use zenko_desktop::tt::parse_clock;
use zenko_desktop::{Cfg, Companion};

/// the fake clock and APPDATA are process-wide, so these tests take turns
static CLOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[test]
fn life_bits_match_zenko_py() {
    let _g = CLOCK.lock().unwrap_or_else(|e| e.into_inner());
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
    let _g = CLOCK.lock().unwrap_or_else(|e| e.into_inner());
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
    let s = said(&mut f);
    assert!(s.is_empty(), "only once a day {s:?}");
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

#[test]
fn battery_matches_zenko_py() {
    let _g = CLOCK.lock().unwrap_or_else(|e| e.into_inner());
    use zenko_desktop::battery::Shape;
    let g: Value = serde_json::from_str(include_str!("../../../golden/life.json")).unwrap();
    let mut f = Companion::new(Cfg::from(json!({"theme": "snow"})), 1.0, 1000.0);
    let r = |v: &Value| zenko_desktop::shapes::tk_coord(v.as_f64().unwrap());
    for c in g["battery"].as_array().unwrap() {
        f.battery = Some((c["st"][0].as_i64().unwrap(), c["st"][1].as_bool().unwrap()));
        (f.z, f.colw, f.fox_y) = (c["z"].as_i64().unwrap() as i32, 37, 21);
        f.pal.set(b'K', "#262b4a");
        let got = f.battery_shapes(c["dx"].as_i64().unwrap() as i32, c["dy"].as_i64().unwrap() as i32, c["curled"].as_bool().unwrap());
        let want: Vec<Shape> = c["ops"].as_array().unwrap().iter().map(|o| {
            let a = o[1].as_array().unwrap();
            let fill = o[2].as_str().unwrap().to_string();
            if o[0] == "rect" {
                let ol = o[3].as_str().unwrap();
                Shape::Rect { x1: r(&a[0]), y1: r(&a[1]), x2: r(&a[2]), y2: r(&a[3]), fill,
                              outline: (!ol.is_empty()).then(|| (ol.to_string(), o[4].as_i64().unwrap() as i32)) }
            } else {
                Shape::Poly { pts: a.chunks(2).map(|p| (r(&p[0]), r(&p[1]))).collect(), fill, outline: o[3].as_str().unwrap().into() }
            }
        }).collect();
        assert_eq!(got, want, "battery {c}");
    }
}

#[test]
fn ema_countdown_on_the_plaque() {
    let _g = CLOCK.lock().unwrap_or_else(|e| e.into_inner());
    unsafe { std::env::set_var("ZK_TZ_OFFSET", "0") };
    // 2025-11-26 10:00:01 UTC; exams on 2026-04-27 = 152 days
    zenko_desktop::set_fake_now(Some(1764151201.0));
    let mut f = Companion::new(Cfg::from(json!({"exam_date": "2026-04-27"})), 1.0, 1000.0);
    assert_eq!(f.exam_days(), 152);
    assert_eq!(f.belly_text(1100.0), "D-152 TO EXAMS");
    assert_eq!(f.belly_text(1105.0), "D-152 TO EXAMS", "it keeps scrolling until it has gone past once");
    let dur = (14.0 * 4.0 + 8.0) * 0.11 + 0.3;
    assert_eq!(f.belly_text(1100.0 + dur + 0.1), "10:00");
    let mut g = Companion::new(Cfg::from(json!({"exam_date": "2026-04-27", "ema_countdown": false})), 1.0, 1000.0);
    assert_eq!(g.belly_text(1100.0), "10:00");
    zenko_desktop::set_fake_now(None);
}
