//! The hub's Customize page in the window: building it, the live preview (cz_anim, every 70 ms), its swatches,
//! radios, checkboxes, buttons, the accent dots and the sidebar editor (zenko.py hub_build_customize, cz_anim,
//! z20_customize_extra, z20_nav_editor, z22_customize_extra).

use super::*;
use serde_json::{Value, json};
use zenko_desktop::custpage::{CustData, CustPage};

fn data() -> CustData {
    crate::win::with_fox(|fox, n| {
        fox.accessories(n);
        fox.cust_data()
    }).unwrap_or_default()
}

pub(super) fn build(x: &mut HubW, pg: usize) {
    let th = x.hub.t.clone();
    let cm10 = (100.0 * x.dpi / 25.4 + 0.5) as i32;
    let d = data();
    let f = faces(x);
    x.cust = Some(CustPage::build(&mut x.hub.ui, pg, &th, &f, cm10, &d));
}

pub(super) fn fit(x: &mut HubW, w: i32, h: i32) {
    let (th, f) = (x.hub.t.clone(), faces(x));
    let Some(p) = x.cust.as_mut() else { return };
    let y = x.hub.ui.ws[p.sc.cv].yoff;
    p.fit(&mut x.hub.ui, &x.fonts, &th, &f, x.hub.zk_f, w, h);
    let sc = p.sc.clone();
    sc.set_y(&mut x.hub.ui, y);
    x.hub.ui.layout_at(&x.fonts, w, h);
    sc.draw_bar(&mut x.hub.ui, false, &th);
}

/// the page made again (zenko.py: hub_close + open_hub("customize") after Start over, or the editor's redraw)
fn rebuild(x: &mut HubW) {
    let Some(old) = x.cust.take() else { return };
    let y = x.hub.ui.ws[old.sc.cv].yoff;
    let pg = old.pg;
    let kids = x.hub.ui.ws[pg].kids.clone();
    for k in kids {
        x.hub.ui.destroy(k);
    }
    build(x, pg);
    relayout(x);
    if let Some(p) = x.cust.as_ref() {
        let sc = p.sc.clone();
        sc.set_y(&mut x.hub.ui, y);
        relayout(x);
    }
}

fn cfg_set(k: &str, v: Value) {
    let k = k.to_string();
    crate::win::with_fox(move |fox, _| fox.cfg_set(&k, v));
}

fn cfg_get(k: &str) -> Value {
    let k = k.to_string();
    crate::win::with_fox(move |fox, _| fox.cfg.raw.get(&k).cloned()).flatten().unwrap_or(Value::Null)
}

fn look() {
    crate::win::with_fox(|fox, _| fox.look_changed());
}

fn say(text: &str, secs: f64) {
    let t = text.to_string();
    crate::win::with_fox(move |fox, _| fox.say(&t, secs));
}

/// every frame while the page shows: the preview follows her pose (cz_anim, 70 ms); a box that lost the keyboard saves
pub(super) fn step(x: &mut HubW, n: f64) {
    if x.cust.is_none() || x.hub.side.page.as_deref() != Some("customize") {
        return;
    }
    let f = x.hub.ui.focus;
    if f != x.cust_focus {
        if let Some(old) = x.cust_focus {
            save_entry(x, old);
        }
        x.cust_focus = f;
    }
    if n >= x.cust_anim {
        x.cust_anim = n + 0.07;
        let d = data();
        let (th, fu) = (x.hub.t.clone(), x.hub.font_ui.clone());
        if let Some(p) = x.cust.as_ref() {
            p.draw_preview(&mut x.hub.ui, &th, &fu, &d);
            let bl = p.bond;
            if let Kind::Label { text, .. } = &mut x.hub.ui.ws[bl].kind {
                *text = format!("Bond  {}   {}", d.bond.0, d.bond.1);
            }
        }
    }
}

fn save_entry(x: &mut HubW, id: usize) {
    let Some(key) = x.cust.as_ref().and_then(|p| p.entries.iter().find(|e| e.1 == id).map(|e| e.0.clone())) else { return };
    let v = x.hub.ui.text_of(id).trim().to_string();
    let v = match key.as_str() {
        "fox_name" => {
            let s: String = v.chars().take(20).collect();
            if s.is_empty() { "Zenko".to_string() } else { s }
        }
        _ => {
            let s: String = v.chars().take(40).collect();
            if s.is_empty() { "Kon! Kon!".to_string() } else { s }
        }
    };
    cfg_set(&key, json!(v));
}

pub(super) fn key_return(x: &mut HubW) -> bool {
    let Some(f) = x.hub.ui.focus else { return false };
    if x.cust.as_ref().is_some_and(|p| p.entries.iter().any(|e| e.1 == f)) {
        save_entry(x, f);
        relayout(x);
        return true;
    }
    false
}

fn radio_group(x: &mut HubW, pre: &str, val: &str) {
    for w in x.hub.ui.ws.iter_mut() {
        if let Kind::Check { on, cmd, radio: true, .. } = &mut w.kind
            && let Some(r) = cmd.strip_prefix(pre) {
            *on = r.rsplit(':').next() == Some(val);
        }
    }
}

fn check_on(x: &HubW, cmd: &str) -> bool {
    x.hub.ui.ws.iter().any(|w| matches!(&w.kind, Kind::Check { on: true, cmd: c, .. } if c == cmd))
}

pub(super) fn cmd(x: &mut HubW, c: &str) {
    if let Some(key) = c.strip_prefix("cz_chk:") {
        let on = check_on(x, c);
        cfg_set(key, json!(on));
        match key {
            "glasses_in_session" | "headphones_with_music" | "lockin_headband" | "foxfire" => look(),
            "nav_compact" => out("run:reopen:customize"),
            "bubble_scale" => say(&format!("My bubble is {} now~", if on { "as big as me" } else { "always the same size" }), 4.0),
            _ => {}
        }
    } else if let Some(r) = c.strip_prefix("cz_rad:") {
        let mut it = r.splitn(3, ':');
        let (key, ty, val) = (it.next().unwrap_or(""), it.next().unwrap_or("s"), it.next().unwrap_or(""));
        radio_group(x, &format!("cz_rad:{key}:"), val);
        let v = match ty {
            "i" => json!(val.parse::<i64>().unwrap_or(0)),
            "f" => json!(val.parse::<f64>().unwrap_or(1.0)),
            _ => json!(val),
        };
        match key {
            "size" => out(&format!("run:size:{val}")),
            "voice_level" => {
                cfg_set(key, v);
                look();
                crate::win::with_fox(|fox, _| {
                    let lvl = fox.cfg.f("voice_level", 1.0) as i64;
                    let t = zenko_desktop::chat::fox_voice("This is how I talk now.", "happy", true, lvl, &mut fox.brain.rng);
                    fox.say(&t, 5.0);
                });
            }
            "voice_clips" | "chirp_pitch" => {
                cfg_set(key, v);
                look();
                out("sound:hello");
            }
            "tails" | "ear_style" | "blush_style" => {
                cfg_set(key, v);
                look();
            }
            _ => cfg_set(key, v),
        }
    } else if let Some(k) = c.strip_prefix("cz_outfit:") {
        let on = check_on(x, c);
        let k = k.to_string();
        crate::win::with_fox(move |fox, _| fox.set_outfit(&k, Some(on)));
    } else if let Some(k) = c.strip_prefix("cz_robe:") {
        radio_group(x, "cz_robe:", k);
        cfg_set("robe", json!(k));
    } else if let Some(k) = c.strip_prefix("cz_bubble:") {
        radio_group(x, "cz_bubble:", k);
        cfg_set("bubble_style", json!(k));
        say("Like this~? Kon!", 4.0);
    } else if let Some(k) = c.strip_prefix("cz_scene:") {
        radio_group(x, "cz_scene:", k);
        cfg_set("side_scene", json!(k));
    } else if let Some(k) = c.strip_prefix("cz_decor:") {
        let on = check_on(x, c);
        let d = crate::win::with_fox(|fox, _| fox.cust_data()).unwrap_or_default();
        let rewards = zenko_desktop::rewards::rdata()["MAP_REWARDS"].as_array().cloned().unwrap_or_default();
        let hd: Vec<String> = rewards.iter().filter(|r| r[1] == "decor" && d.got.iter().any(|g| r[0] == g.as_str()))
            .filter_map(|r| r[2].as_str().map(String::from)).collect();
        let mut s: Vec<String> = match cfg_get("garden_decor") {
            Value::Array(a) => a.iter().filter_map(|v| v.as_str().map(String::from)).collect(),
            _ => hd,
        };
        s.retain(|e| e != k);
        if on {
            s.push(k.into());
        }
        s.sort();
        cfg_set("garden_decor", json!(s));
    } else if let Some(k) = c.strip_prefix("cz_nav:") {
        let on = check_on(x, c);
        let mut hs: Vec<String> = cfg_get("nav_hidden").as_array().map(|a| a.iter().filter_map(|v| v.as_str().map(String::from)).collect())
            .unwrap_or_default();
        hs.retain(|h| h != k);
        if !on {
            hs.push(k.into());
        }
        hs.sort();
        cfg_set("nav_hidden", json!(hs));
        nav_apply(x);
        rebuild(x);
    } else if let Some(m) = c.strip_prefix("cz_mood:") {
        let m = m.to_string();
        crate::win::with_fox(move |fox, n| {
            fox.set_mood(n, &m, 4.0);
            fox.play = None;
        });
    } else if let Some(k) = c.strip_prefix("cz_play:") {
        let k = k.to_string();
        crate::win::with_fox(move |fox, n| fox.interact(n, &k, None));
    } else if let Some(k) = c.strip_prefix("cz_pose:") {
        out(&format!("run:pose:{k}"));
    } else if let Some(k) = c.strip_prefix("cz_scale:") {
        let v = x.cust.as_ref().map(|p| p.scale).and_then(|s| match x.hub.ui.ws[s].kind {
            Kind::Scale { val, .. } => Some(val),
            _ => None,
        }).unwrap_or(0.0);
        cfg_set(k, json!(v as i64));
    } else if let Some(b) = c.strip_prefix("cz_btn:") {
        match b {
            "reset" => {
                crate::win::with_fox(|fox, _| {
                    let defs = zenko_desktop::custpage::cdata()["DEFAULTS"].clone();
                    for (k, v) in defs.as_object().into_iter().flatten() {
                        fox.cfg_set(k, v.clone());
                    }
                    fox.look_changed();
                });
                out("run:reopen:customize");
            }
            "speak_test" => out("run:speak_test"),
            "voice_demo" => out("sound:hello"),
            _ => {
                // (VOICEVOX's finder and the run.exe picker, and the Japanese voice search, aren't ported yet)
                let s = format!("cz_btn:{b}");
                crate::win::with_fox(move |fox, _| fox.trace.push(s));
            }
        }
    }
    relayout(x);
}

/// z20_nav_apply: the sidebar's pages in the new order, minus the hidden ones
fn nav_apply(x: &mut HubW) {
    let keys = crate::win::with_fox(|fox, _| zenko_desktop::hubwin::nav_list(&fox.cfg)).unwrap_or_default();
    x.hub.side.navkeys = keys;
    let c = x.ctx.clone();
    x.hub.side.nav(&c);
    x.hub.side_sync();
}

/// a press that isn't a button: a swatch, an accent dot, a sidebar arrow, the reset link, the preview (she reacts)
pub(super) fn click(x: &mut HubW, mx: i32, my: i32) -> bool {
    if x.cust.is_none() || x.hub.side.page.as_deref() != Some("customize") {
        return false;
    }
    let inside = |x: &HubW, id: usize| {
        let (ax, ay) = x.hub.ui.abs(id);
        let w = &x.hub.ui.ws[id];
        w.mapped && mx >= ax && my >= ay && mx < ax + w.w && my < ay + w.h
    };
    let p = x.cust.as_ref().unwrap();
    if let Some((_, key, val)) = p.swatches.iter().find(|(id, _, _)| inside(x, *id)).cloned() {
        cfg_set(&key, json!(val));
        out("sound:toggle");
        look();
        let th = x.hub.t.clone();
        x.cust.as_ref().unwrap().draw_swatches(&mut x.hub.ui, &th, &key, &val);
        return true;
    }
    if let Some((_, k)) = p.accents.iter().find(|(id, _)| inside(x, *id)).cloned() {
        cfg_set("accent", json!(k));
        out("run:reopen:customize");
        return true;
    }
    if let Some(&(_, i, d)) = p.arrows.iter().find(|(id, _, _)| inside(x, *id)) {
        let mut o = p.nav_order.clone();
        let j = i as i64 + d as i64;
        if j >= 0 && (j as usize) < o.len() {
            o.swap(i, j as usize);
            cfg_set("nav_order", json!(o));
            nav_apply(x);
            rebuild(x);
        }
        return true;
    }
    if p.nav_reset != 0 && inside(x, p.nav_reset) {
        cfg_set("nav_order", json!([]));
        cfg_set("nav_hidden", json!([]));
        nav_apply(x);
        rebuild(x);
        return true;
    }
    if inside(x, p.preview) {
        // the click lands on her as if on the desktop: (e.x - 10) / pz of her pixels from her corner
        let (ax, ay) = x.hub.ui.abs(p.preview);
        let (ex, ey) = ((mx - ax - 10) as f64 / 3.0, (my - ay - 10) as f64 / 3.0);
        crate::win::with_fox(move |fox, n| {
            let (cx, cy) = (fox.colw as f64 + ex * fox.z as f64, fox.fox_y as f64 + ey * fox.z as f64);
            fox.fox_clicked(n, cx as i32, cy as i32);
        });
        return true;
    }
    false
}
