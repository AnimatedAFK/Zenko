//! The hub's Reminders page in the window: building it, drawing the night shrine again when the emas or today's
//! timetable change (and each minute, for "in 25 min"), the wisps' drift, the Write-an-ema card (chips, Every day,
//! Say it out loud, Hang it / Return) and the plaques' clicks and hover (zenko.py hub_build_reminders, ema_render,
//! ema_anim, ema_motion, ema_click).

use super::*;
use zenko_desktop::rempage::{RemData, RemPage};

fn fonts_for(d: &mut RemData) {
    d.jp = ["Yu Gothic UI", "Yu Gothic", "Meiryo UI", "Meiryo", "MS Gothic", "Noto Sans CJK JP", "Noto Sans JP", "IPAGothic"]
        .iter().find(|n| crate::gdi::font_exists(n)).map(|n| n.to_string()).unwrap_or_default();
    d.serif = ["Yu Mincho", "MS Mincho", "Georgia", "Times New Roman"].iter()
        .find(|n| crate::gdi::font_exists(n)).map(|n| n.to_string()).unwrap_or_default();
}

fn data() -> RemData {
    let mut d = crate::win::with_fox(|fox, _| fox.rem_data()).unwrap_or_default();
    fonts_for(&mut d);
    d
}

/// what the canvas shows changed (the minute counts: "in 25 min")
fn same(a: &RemData, b: &RemData) -> bool {
    a.act == b.act && a.done == b.done && a.tt == b.tt && (a.now as i64) / 60 == (b.now as i64) / 60 && a.today == b.today
}

/// hub_build_reminders(pg)
pub(super) fn build(x: &mut HubW, pg: usize) {
    let th = x.hub.t.clone();
    let cm10 = (100.0 * x.dpi / 25.4 + 0.5) as i32;
    let d = data();
    let f = faces(x);
    x.rem = Some(RemPage::build(&mut x.hub.ui, pg, &th, &f, cm10, &d));
    x.rem_d = d;
}

/// ema_render() with the app's data now
pub(super) fn render(x: &mut HubW) {
    let d = data();
    let (th, f) = (x.hub.t.clone(), faces(x));
    if let Some(p) = x.rem.as_mut() {
        p.render(&mut x.hub.ui, &th, &f, &x.fonts, x.hub.zk_f, &d);
    }
    x.rem_d = d;
    relayout(x);
}

/// zk_fit for the page (the canvas is drawn again when its width changed)
pub(super) fn fit(x: &mut HubW, w: i32, h: i32) {
    let (th, f) = (x.hub.t.clone(), faces(x));
    let d = x.rem_d.clone();
    let Some(p) = x.rem.as_mut() else { return };
    p.fit(&mut x.hub.ui, &x.fonts, &th, &f, x.hub.zk_f, w, h, &d);
    let sc = p.sc.clone();
    let y = x.hub.ui.ws[sc.cv].yoff;
    sc.set_y(&mut x.hub.ui, y);
    x.hub.ui.layout_at(&x.fonts, w, h);
    sc.draw_bar(&mut x.hub.ui, false, &th);
}

/// every frame while the page shows: the wisps drift (ema_anim, 90 ms), the "or at" box picks itself while it has
/// text (its <KeyRelease>), and every 2 s the canvas again if what it shows changed
pub(super) fn step(x: &mut HubW, n: f64) {
    if x.rem.is_none() || x.hub.side.page.as_deref() != Some("reminders") {
        return;
    }
    if n >= x.rem_anim {
        x.rem_anim = n + 0.09;
        let t = n;
        if let Some(p) = x.rem.as_ref() {
            p.anim(&mut x.hub.ui, t);
        }
    }
    let th = x.hub.t.clone();
    if let Some(p) = x.rem.as_mut()
        && x.hub.ui.focus == Some(p.at_entry) && !x.hub.ui.text_of(p.at_entry).trim().is_empty() && p.pick != "at" {
        p.choose(&mut x.hub.ui, &th, "at");
    }
    if n >= x.rem_chk {
        x.rem_chk = n + 2.0;
        let d = data();
        if !same(&d, &x.rem_d) {
            render(x);
        }
    }
}

fn entry_set(x: &mut HubW, id: usize, s: &str) {
    if let Kind::Entry { e, .. } = &mut x.hub.ui.ws[id].kind {
        e.text = s.encode_utf16().collect();
        e.pos = e.text.len();
        e.left = 0;
        e.sel = None;
    }
}

fn check_on(x: &HubW, id: usize) -> bool {
    matches!(x.hub.ui.ws[id].kind, Kind::Check { on: true, .. })
}

/// hang(): the ema from the box, at the time the chips / "or at" say
fn hang(x: &mut HubW) {
    let Some(p) = x.rem.as_ref() else { return };
    let raw = x.hub.ui.text_of(p.entry).trim().to_string();
    if raw.is_empty() {
        x.hub.ui.focus = Some(p.entry);
        return;
    }
    let (k, atv, daily) = (p.pick.clone(), x.hub.ui.text_of(p.at_entry).trim().to_string(), check_on(x, p.every));
    let (entry, at_entry, every) = (p.entry, p.at_entry, p.every);
    let res = crate::win::with_fox(move |fox, _| {
        let now = zenko_desktop::unix_now();
        let off = zenko_desktop::local_offset_pub() as f64;
        let now_l = now + off;
        let today = now_l.div_euclid(86400.0);
        let (txt, mut at, mut rep) = zenko_desktop::ema::ema_parse(&raw, now);
        match k.as_str() {
            "none" => at = None,
            "30" | "60" => at = Some(now + k.parse::<f64>().unwrap() * 60.0),
            "tonight" => {
                let dd = today * 86400.0 + 20.0 * 3600.0;
                at = Some(if dd > now_l { dd } else { dd + 86400.0 } - off);
            }
            "tmrw" => at = Some((today + 1.0) * 86400.0 + 7.0 * 3600.0 - off),
            "at" if !atv.is_empty() => {
                let q = if regex::Regex::new("[a-z]{3}").unwrap().is_match(&atv) { format!("x {atv}") } else { format!("x at {atv}") };
                let (_, a2, r2) = zenko_desktop::ema::ema_parse(&q, now);
                if a2.is_some() {
                    at = a2;
                }
                rep = rep.or(r2);
            }
            _ => {}
        }
        let mut rep = rep.map(String::from);
        if daily {
            rep = Some("daily".into());
            if at.is_none() {
                at = Some(now + 60.0);
            }
        }
        let t = if txt.is_empty() { raw.chars().take(160).collect() } else { txt };
        let e = fox.ema_add(&t, at, rep.as_deref());
        let msg = match at {
            Some(a) => format!("Hung~ {} ({})", zenko_desktop::ema::ema_when(Some(a), now, false),
                               zenko_desktop::ema::ema_left(a, now)),
            None => "Hung~ anytime".into(),
        };
        (e["id"].as_str().unwrap_or("").to_string(), msg)
    });
    let Some((id, msg)) = res else { return };
    entry_set(x, entry, "");
    entry_set(x, at_entry, "");
    if let Kind::Check { on, .. } = &mut x.hub.ui.ws[every].kind {
        *on = false;
    }
    let th = x.hub.t.clone();
    if let Some(p) = x.rem.as_mut() {
        p.choose(&mut x.hub.ui, &th, "auto");
        p.sel = Some(id);
        p.set_msg(&mut x.hub.ui, &msg);
    }
    out("sound:hello");
    render(x);
}

/// the card's buttons: the When chips, Say it out loud, Hang it
pub(super) fn cmd(x: &mut HubW, cmd: &str) {
    let th = x.hub.t.clone();
    if let Some(k) = cmd.strip_prefix("rem_pick:") {
        if let Some(p) = x.rem.as_mut() {
            p.choose(&mut x.hub.ui, &th, k);
        }
    } else if cmd == "rem_aloud" {
        let on = x.rem.as_ref().is_some_and(|p| check_on(x, p.aloud));
        crate::win::with_fox(move |fox, _| fox.cfg_set("remind_aloud", serde_json::json!(on)));
    } else if cmd == "rem_hang" {
        hang(x);
    }
    relayout(x);
}

/// Return in the ema box hangs it
pub(super) fn key_return(x: &mut HubW) -> bool {
    if x.rem.as_ref().is_some_and(|p| x.hub.ui.focus == Some(p.entry)) {
        hang(x);
        return true;
    }
    false
}

/// the spot of the night canvas under the pointer, in its own pixels
fn spot(x: &HubW, mx: i32, my: i32) -> Option<(String, String)> {
    if x.hub.side.page.as_deref() != Some("reminders") {
        return None;
    }
    let p = x.rem.as_ref()?;
    let cv = p.cv;
    let (ax, ay) = x.hub.ui.abs(cv);
    let w = &x.hub.ui.ws[cv];
    if !w.mapped || mx < ax || my < ay || mx >= ax + w.w || my >= ay + w.h {
        return None;
    }
    p.hit((mx - ax) as f64, (my - ay) as f64)
}

/// ema_motion: the plaque under the pointer lifts, the hand cursor over anything that does something
pub(super) fn motion(x: &mut HubW, mx: i32, my: i32) {
    if x.rem.is_none() || x.hub.side.page.as_deref() != Some("reminders") {
        return;
    }
    let h = spot(x, mx, my);
    if h.is_some() {
        x.cursor = IDC_HAND;
    }
    let hov = h.filter(|(a, _)| a == "plaque").map(|(_, i)| i);
    if x.rem.as_ref().is_some_and(|p| p.hover != hov) {
        x.rem.as_mut().unwrap().hover = hov;
        render(x);
    }
}

/// ema_click (on release, like <ButtonRelease-1>)
pub(super) fn click(x: &mut HubW, mx: i32, my: i32) -> bool {
    if x.rem.is_none() || x.hub.side.page.as_deref() != Some("reminders") {
        return false;
    }
    let (ax, ay) = x.hub.ui.abs(x.rem.as_ref().unwrap().cv);
    let cvw = &x.hub.ui.ws[x.rem.as_ref().unwrap().cv];
    if !cvw.mapped || mx < ax || my < ay || mx >= ax + cvw.w || my >= ay + cvw.h {
        return false;
    }
    let Some((what, eid)) = spot(x, mx, my) else {
        if x.rem.as_ref().is_some_and(|p| p.sel.is_some()) {
            x.rem.as_mut().unwrap().sel = None;
            render(x);
        }
        return true;
    };
    match what.as_str() {
        "clear_done" => {
            crate::win::with_fox(|fox, _| fox.ema_clear_done());
        }
        "plaque" => {
            let p = x.rem.as_mut().unwrap();
            p.sel = if p.sel.as_deref() == Some(eid.as_str()) { None } else { Some(eid) };
        }
        "done" => {
            x.rem.as_mut().unwrap().sel = None;
            crate::win::with_fox(move |fox, _| fox.ema_mark(&eid, true));
            out("sound:happy");
        }
        "undo" => {
            crate::win::with_fox(move |fox, _| fox.ema_mark(&eid, false));
        }
        "remove" => {
            let p = x.rem.as_mut().unwrap();
            if p.sel.as_deref() == Some(eid.as_str()) {
                p.sel = None;
            }
            crate::win::with_fox(move |fox, _| fox.ema_remove(&eid));
        }
        "snooze" => {
            crate::win::with_fox(move |fox, _| fox.ema_snooze(&eid, 10));
        }
        "edit" => {
            // (zenko.py's "Rewrite this ema" dialog isn't ported yet: the app is asked for it)
            crate::win::with_fox(move |fox, _| fox.trace.push(format!("ema_edit:{eid}")));
        }
        _ => {}
    }
    render(x);
    true
}
