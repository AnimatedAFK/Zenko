//! The z19 focus glow in the hub window's frame and the petal burst (golden/glow.json, tools/make_golden_glow.py):
//! every item zenko.py's z19_glow draws (type, coords, colour, width, caps) for several sizes, themes, progress,
//! paused and burst times.

use serde_json::Value;
use zenko_desktop::hubwin::{GlowBurst, Hub};
use zenko_desktop::tkcanvas::CKind;

#[test]
fn glow_matches_zenko_py() {
    let g: Vec<Value> = serde_json::from_str(&std::fs::read_to_string("../../golden/glow.json").unwrap()).unwrap();
    for (n, c) in g.iter().enumerate() {
        let cfg = zenko_desktop::Cfg::from(serde_json::json!({"theme": c["theme"]}));
        let t = zenko_desktop::theme::theme(&cfg);
        let (w, h) = (c["w"].as_i64().unwrap() as i32, c["h"].as_i64().unwrap() as i32);
        let mut hub = Hub::new(&t, "Segoe UI", vec![], true, true, w, h, 0, 0.0);
        let sess = c["sess"].as_array().map(|s| (s[0].as_f64().unwrap(), s[1].as_f64().unwrap(), s[2].as_bool().unwrap()));
        let burst = c["burst"].as_f64().map(|_| GlowBurst {
            t0: 100.0,
            petals: c["petals"].as_array().unwrap().iter()
                .map(|p| (p[0].as_f64().unwrap(), p[1].as_f64().unwrap(), p[2].as_str().unwrap().to_string(), p[3].as_f64().unwrap()))
                .collect(),
        });
        let alive = hub.glow_draw(w, h, c["now"].as_f64().unwrap(), sess, burst.as_ref());
        if burst.is_some() {
            assert_eq!(alive, c["burst_left"].as_bool().unwrap(), "case {n} burst");
        }
        let id = hub.ids.border.unwrap();
        let zenko_desktop::ui::Kind::Canvas { tk, .. } = &hub.ui.ws[id].kind else { panic!() };
        let got: Vec<_> = tk.as_ref().map(|cv| cv.items.iter().filter(|i| i.has("z19glow")).cloned().collect()).unwrap_or_default();
        let want = c["items"].as_array().unwrap();
        assert_eq!(got.len(), want.len(), "case {n} count");
        for (k, (it, wi)) in got.iter().zip(want).enumerate() {
            let ty = match it.kind {
                CKind::Line => "line",
                CKind::Oval => "oval",
                CKind::Rect => "rectangle",
                _ => "?",
            };
            assert_eq!(ty, wi["type"], "case {n} item {k}");
            let wc: Vec<f64> = wi["coords"].as_array().unwrap().iter().map(|v| v.as_f64().unwrap()).collect();
            assert_eq!(it.coords.len(), wc.len(), "case {n} item {k} coords");
            for (a, b) in it.coords.iter().zip(&wc) {
                assert!((a - b).abs() < 1e-9, "case {n} item {k} coords {:?} vs {wc:?}", it.coords);
            }
            assert_eq!(it.fill, wi["fill"].as_str().unwrap(), "case {n} item {k} fill");
            if let Some(o) = wi.get("outline") {
                assert_eq!(it.outline, o.as_str().unwrap(), "case {n} item {k} outline");
            }
            if let Some(wd) = wi.get("width") {
                assert_eq!(it.width, wd.as_f64().unwrap(), "case {n} item {k} width");
                assert_eq!(it.capstyle, wi["capstyle"].as_str().unwrap(), "case {n} item {k} cap");
                assert_eq!(it.joinstyle, wi["joinstyle"].as_str().unwrap(), "case {n} item {k} join");
            }
        }
    }
}
