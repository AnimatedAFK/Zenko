//! The hub's Screen time page in the window (zenko.py hub_build_screen and its buttons): the range switch, Pause /
//! Resume, the two switches, the skipped words (tap to remove, type to add, Return adds too), reset, forget today and
//! delete all (each asked first), and screen_save_maybe's render each minute while the page shows.

use super::*;
use serde_json::json;
use zenko_desktop::screenpage::{ScreenData, ScreenPage, exclude_edit};

/// what the page shows now
fn data(x: &HubW) -> ScreenData {
    let (mincho, serif, _) = hubbook::fonts_pub(&x.hub.font_ui);
    crate::win::with_fox(|fox, _| {
        let unix = zenko_desktop::unix_now();
        let secs = unix.floor() as i64 + zenko_desktop::local_offset_pub();
        ScreenData {
            life: fox.life_now(), screen_time: fox.cfg.b("screen_time", true), nudges: fox.cfg.b("screen_nudges", true),
            exclude: fox.screen_exclude(), today: secs.div_euclid(86400), hour: secs.rem_euclid(86400) / 3600, now: unix,
            jp: mincho.clone(), serif: serif.clone(),
        }
    }).unwrap_or_default()
}

fn cm10(x: &HubW) -> i32 {
    (100.0 * x.dpi / 25.4 + 0.5) as i32
}

/// hub_build_screen(pg) (screen_ui["range"] kept from the last time the page was made)
pub(super) fn build(x: &mut HubW, pg: usize) {
    let (th, f) = (x.hub.t.clone(), faces(x));
    let d = data(x);
    let (range, c) = (x.screen_range, cm10(x));
    x.screen = Some(ScreenPage::build(&mut x.hub.ui, pg, &th, &f, c, range, &d));
    x.screen_next = now(x) + 60.0;
}

pub(super) fn fit(x: &mut HubW, w: i32, h: i32) {
    let (th, f) = (x.hub.t.clone(), faces(x));
    let Some(p) = x.screen.as_mut() else { return };
    p.fit(&mut x.hub.ui, &x.fonts, &th, &f, x.hub.zk_f, w, h);
    let sc = p.sc.clone();
    sc.draw_bar(&mut x.hub.ui, false, &th);
}

/// screen_render()
pub(super) fn render(x: &mut HubW) {
    let (th, f) = (x.hub.t.clone(), faces(x));
    let d = data(x);
    let c = cm10(x);
    let Some(p) = x.screen.as_mut() else { return };
    p.render(&mut x.hub.ui, &th, &f, c, &d);
    relayout(x);
}

/// screen_save_maybe's render while the page shows (not while you're typing a word in)
pub(super) fn step(x: &mut HubW, n: f64) {
    if x.screen.is_none() || n < x.screen_next {
        return;
    }
    x.screen_next = n + 60.0;
    let typing = x.screen.as_ref().is_some_and(|p| x.hub.ui.focus == Some(p.entry));
    if x.hub.side.page.as_deref() == Some("screen") && !typing {
        render(x);
    }
}

fn exclude_do(x: &mut HubW, word: &str, add: Option<bool>) {
    let w = word.to_string();
    crate::win::with_fox(move |fox, _| {
        let mut ex = fox.screen_exclude();
        exclude_edit(&mut ex, &w, add);
        zenko_desktop::update_config(&mut fox.cfg, |v| v["screen_exclude"] = json!(ex));
    });
    render(x);
}

/// the Return key in the word box: Skip this too
pub(super) fn key_return(x: &mut HubW) -> bool {
    let Some(e) = x.screen.as_ref().map(|p| p.entry) else { return false };
    if x.hub.ui.focus != Some(e) {
        return false;
    }
    cmd(x, "scr_add");
    true
}

pub(super) fn cmd(x: &mut HubW, cmd: &str) {
    let Some(p) = x.screen.as_ref() else { return };
    if let Some(n) = cmd.strip_prefix("scr_range:") {
        x.screen_range = n.parse().unwrap_or(1);
        if let Some(p) = x.screen.as_mut() {
            p.range = x.screen_range;
        }
        render(x);
    } else if cmd == "scr_pause" {
        // screen_pause(60)
        crate::win::with_fox(|fox, _| {
            let t = zenko_desktop::unix_now() + 3600.0;
            fox.life_do(|l| l["screen"]["paused_until"] = json!(t));
        });
        render(x);
        crate::win::with_fox(|fox, _| fox.say("Okay, I'm not looking for the next hour~ Your secrets are safe.", 6.0));
    } else if cmd == "scr_resume" {
        crate::win::with_fox(|fox, _| fox.life_do(|l| l["screen"]["paused_until"] = json!(0)));
        render(x);
    } else if let Some(k) = cmd.strip_prefix("scr_tog:") {
        let Some(&(id, key)) = p.toggles.iter().find(|t| t.1 == k) else { return };
        let on = matches!(x.hub.ui.ws[id].kind, Kind::Check { on: true, .. });
        crate::win::with_fox(move |fox, _| zenko_desktop::update_config(&mut fox.cfg, |v| v[key] = on.into()));
        render(x);
    } else if let Some(w) = cmd.strip_prefix("scr_rm:") {
        exclude_do(x, w, Some(false));
    } else if cmd == "scr_add" {
        let w = x.hub.ui.text_of(p.entry);
        if !w.trim().is_empty() {
            exclude_do(x, &w, Some(true));
        }
    } else if cmd == "scr_reset" {
        exclude_do(x, "", None);
    } else if let Some(which) = cmd.strip_prefix("scr_clear:") {
        let q = if which == "today" { "Forget today's screen time?" } else {
            "Delete all saved screen time? This can't be undone." };
        x.ask = Some((q.into(), "Screen time".into(), format!("scr_cleared:{which}")));
    } else if let Some(which) = cmd.strip_prefix("scr_cleared:") {
        // screen_clear(today_only) once you said yes
        let today = which == "today";
        crate::win::with_fox(move |fox, _| {
            let d = zenko_desktop::today_iso();
            fox.life_do(|l| {
                if today {
                    if let Some(o) = l["screen"]["days"].as_object_mut() {
                        o.remove(&d);
                    }
                } else {
                    l["screen"]["days"] = json!({});
                }
            });
        });
        render(x);
    }
}
