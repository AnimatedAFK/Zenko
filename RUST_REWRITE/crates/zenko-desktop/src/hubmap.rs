//! The hub's World map page in the window (zenko.py Zenko22): the region buttons, the 60 ms map_anim, the hover tip
//! and hand cursor over a place, a click to read it, the passport and journey rows, and the gold-seal quiz.

use super::*;
use serde_json::{Value, json};
use zenko_desktop::mappage::{MapData, MapPage, by_id};

fn cm10(x: &HubW) -> i32 {
    (100.0 * x.dpi / 25.4 + 0.5) as i32
}

/// what the page shows, read from the app now
fn data(x: &HubW) -> MapData {
    let (mincho, serif, _) = hubbook::fonts_pub(&x.hub.font_ui);
    let secs = zenko_desktop::unix_now().floor() as i64 + zenko_desktop::local_offset_pub();
    let iso = zenko_desktop::plan::iso(secs.div_euclid(86400));
    let season = zenko_desktop::hubside::season_of(iso[5..7].parse().unwrap_or(1)).to_string();
    let none = MapData { hours: 0.0, quiz: vec![], pal: zenko_art::palette::Palette::empty(), tails: 3, hd: true, jp: mincho.clone(),
                         serif: serif.clone(), season: season.clone() };
    crate::win::with_fox(|fox, t| {
        let life = zenko_desktop::read_json("zenko_life.json");
        let (hours, _) = zenko_desktop::rewards::journey_state(&life);
        let quiz = life["journey"]["quiz"].as_array().map(|a| a.iter().filter_map(|v| v.as_str().map(String::from)).collect())
            .unwrap_or_default();
        MapData { hours, quiz, pal: fox.pal.clone(), tails: fox.tail_count(t), hd: fox.cfg.b("hd_art", true), jp: mincho.clone(),
                  serif: serif.clone(), season: season.clone() }
    }).unwrap_or(none)
}

pub(super) fn build(x: &mut HubW, pg: usize) {
    let (th, f) = (x.hub.t.clone(), faces(x));
    let (d, c) = (data(x), cm10(x));
    x.map = Some(MapPage::build(&mut x.hub.ui, pg, &th, &f, c, &d));
}

pub(super) fn fit(x: &mut HubW, w: i32, h: i32) {
    let (th, f) = (x.hub.t.clone(), faces(x));
    let Some(p) = x.map.as_mut() else { return };
    p.fit(&mut x.hub.ui, &x.fonts, &th, &f, x.hub.zk_f, w, h);
    let sc = p.sc.clone();
    sc.draw_bar(&mut x.hub.ui, false, &th);
}

fn relayout(x: &mut HubW) {
    let (th, f) = (x.hub.t.clone(), faces(x));
    let (w, h) = x.size;
    let Some(p) = x.map.as_mut() else { return };
    p.relayout(&mut x.hub.ui, &x.fonts, &th, &f, x.hub.zk_f, w, h);
    p.draw(&mut x.hub.ui, &x.fonts);
    let sc = p.sc.clone();
    sc.draw_bar(&mut x.hub.ui, false, &th);
}

fn on_page(x: &HubW) -> bool {
    x.hub.side.page.as_deref() == Some("map") && x.map.is_some()
}

/// hub_after_show("map"): what changed while it was hidden (more focus hours, a seal) shows
pub(super) fn shown(x: &mut HubW) {
    let d = data(x);
    if x.map.as_ref().is_some_and(|p| p.d.hours != d.hours || p.d.quiz != d.quiz || p.d.tails != d.tails) {
        refresh(x);
    }
}

/// map_refresh(): the journey moved on (the region bar, the painting and the panel made again)
pub(super) fn refresh(x: &mut HubW) {
    let d = data(x);
    let Some(p) = x.map.as_mut() else { return };
    p.d = d;
    p.forget();
    p.region_bar(&mut x.hub.ui);
    p.panel(&mut x.hub.ui);
    relayout(x);
}

/// map_anim every 60 ms while the page shows and isn't sliding
pub(super) fn step(x: &mut HubW, n: f64) -> bool {
    if x.mp_badge_at > 0.0 && n >= x.mp_badge_at {
        x.mp_badge_at = 0.0;
        crate::win::with_fox(|fox, t| fox.z22_badge_check(t));
    }
    if !on_page(x) || x.slide.is_some() || n < x.mp_next {
        if !on_page(x) && let Some(p) = x.map.as_mut() {
            p.anim_on = false;
        }
        return false;
    }
    x.mp_next = n + 0.06;
    let t = n;
    let Some(p) = x.map.as_mut() else { return false };
    if !p.anim_on {
        return false;
    }
    p.anim(&mut x.hub.ui, t);
    true
}

fn cv_local(x: &HubW, mx: i32, my: i32) -> Option<(f64, f64)> {
    let p = x.map.as_ref()?;
    let (ax, ay) = x.hub.ui.abs(p.cv);
    let w = &x.hub.ui.ws[p.cv];
    let (lx, ly) = (mx - ax, my - ay);
    (lx >= 0 && ly >= 0 && lx < w.w && ly < w.h && x.hub.ui.shown(p.cv)).then_some((lx as f64, ly as f64))
}

/// map_motion / <Leave>: the tip over the place under the pointer; true while over one (the hand cursor)
pub(super) fn motion(x: &mut HubW, mx: i32, my: i32) -> bool {
    if !on_page(x) {
        return false;
    }
    let pid = cv_local(x, mx, my).and_then(|(lx, ly)| x.map.as_ref().unwrap().pick(lx, ly));
    let p = x.map.as_mut().unwrap();
    if p.hover != pid {
        p.hover_at(&mut x.hub.ui, &x.fonts, pid.as_deref());
    }
    pid.is_some()
}

/// a click on a place on the painting, a passport stamp or a journey row
pub(super) fn click(x: &mut HubW, mx: i32, my: i32) -> bool {
    if !on_page(x) {
        return false;
    }
    if let Some(c) = x.map.as_ref().and_then(|p| p.click_at(&x.hub.ui, mx, my)) {
        cmd(x, &c);
        return true;
    }
    if let Some(pid) = cv_local(x, mx, my).and_then(|(lx, ly)| x.map.as_ref().unwrap().pick(lx, ly)) {
        cmd(x, &format!("mp_sel:{pid}"));
        return true;
    }
    false
}

pub(super) fn cmd(x: &mut HubW, c: &str) {
    let (a, arg) = c.split_once(':').unwrap_or((c, ""));
    match a {
        "mp_region" => {
            let Some(p) = x.map.as_mut() else { return };
            if p.set_region(&mut x.hub.ui, arg) {
                crate::win::with_fox(|fox, _| fox.events.push(zenko_desktop::Ev::Ui("nav".into())));
                relayout(x);
            }
        }
        "mp_sel" => {
            let Some(p) = x.map.as_mut() else { return };
            p.select(&mut x.hub.ui, arg);
            relayout(x);
        }
        "mp_ans" => {
            let i: usize = arg.parse().unwrap_or(0);
            let Some(p) = x.map.as_mut() else { return };
            let pid = p.sel.clone();
            if p.answer(&mut x.hub.ui, i) {
                crate::win::with_fox(move |fox, _| {
                    fox.life_do(|l: &mut Value| {
                        let q = &mut l["journey"]["quiz"];
                        if !q.is_array() {
                            *q = json!([]);
                        }
                        if !q.as_array().unwrap().iter().any(|v| v == pid.as_str()) {
                            q.as_array_mut().unwrap().push(json!(pid));
                        }
                    });
                    fox.events.push(zenko_desktop::Ev::Beep { kind: "gift".into(), variety: false });
                    let name = by_id(&pid).and_then(|p| p[2].as_str()).unwrap_or("").to_string();
                    fox.say(&format!("Sugoi~! Gold seal for {name} in your passport!"), 6.0);
                });
                x.mp_badge_at = now(x) + 0.8;
                refresh(x);
            } else {
                relayout(x);
            }
        }
        _ => {}
    }
}
