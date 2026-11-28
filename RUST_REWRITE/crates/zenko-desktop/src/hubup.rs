//! The hub's Upcoming page in the window: building it, refreshing it when the plan / deadlines / free hours change,
//! its buttons, the clock (tap or drag over half hours, hover info, the hands every second), the calendar cells,
//! the quick add's time menus and the IA / EE / TOK tracker's labels and entries (zenko.py dl_render and friends).

use super::*;
use zenko_desktop::uppage::{UpData, UpPage, quick_times};

/// the clock size and layout for the window now: up_row's 340 / 290 on a wide main area, Zenko's 300 in two rows
fn layout_for(x: &HubW) -> (bool, i32) {
    let main = x.hub.ui.ws[x.hub.ids.stage].w;
    let wide = main >= 1000;
    (wide, if wide { if x.size.0 >= 1180 { 340 } else { 290 } } else { 300 })
}

fn fonts_for(d: &mut UpData) {
    d.jp = ["Yu Gothic UI", "Yu Gothic", "Meiryo UI", "Meiryo", "MS Gothic", "Noto Sans CJK JP", "Noto Sans JP", "IPAGothic"]
        .iter().find(|n| crate::gdi::font_exists(n)).map(|n| n.to_string()).unwrap_or_default();
    d.serif = ["Georgia", "Cambria", "Palatino Linotype", "Book Antiqua", "Garamond", "Constantia"].iter()
        .find(|n| crate::gdi::font_exists(n)).map(|n| n.to_string()).unwrap_or_default();
    d.mincho = ["Yu Mincho", "\u{6e38}\u{660e}\u{671d}", "MS PMincho", "MS Mincho", "Noto Serif JP", "Noto Serif CJK JP"].iter()
        .find(|n| crate::gdi::font_exists(n)).map(|n| n.to_string()).unwrap_or_default();
}

pub(super) fn data() -> UpData {
    let mut d = crate::win::with_fox(|fox, _| fox.up_data()).unwrap_or_default();
    fonts_for(&mut d);
    d
}

fn fox_pic() -> std::rc::Rc<zenko_desktop::tkcanvas::Photo> {
    crate::win::with_fox(|fox, t| {
        let im = fox.small_fox(t);
        std::rc::Rc::new(zenko_desktop::tkcanvas::Photo { w: im.w, h: im.h, px: im.px })
    }).unwrap_or_else(|| std::rc::Rc::new(zenko_desktop::tkcanvas::Photo { w: 1, h: 1, px: vec![[0; 4]] }))
}

/// hub_build_deadlines(pg)
pub(super) fn build(x: &mut HubW, pg: usize) {
    let th = x.hub.t.clone();
    let cm10 = (100.0 * x.dpi / 25.4 + 0.5) as i32;
    let d = data();
    let f = faces(x);
    let (wide, s) = layout_for(x);
    let mut up = UpPage::build(&mut x.hub.ui, pg, &th, &f, cm10, &d, wide, s);
    up.set_fox(fox_pic());
    up.draw_clock(&mut x.hub.ui, &th, &f, &d);
    x.up = Some(up);
    x.up_d = d;
}

/// dl_render() with the app's data now
pub(super) fn render(x: &mut HubW) {
    let d = data();
    let (th, f) = (x.hub.t.clone(), faces(x));
    let cm10 = (100.0 * x.dpi / 25.4 + 0.5) as i32;
    let (wide, s) = layout_for(x);
    if let Some(up) = x.up.as_mut() {
        up.render(&mut x.hub.ui, &th, &f, cm10, &d, wide, s);
    }
    x.up_d = d;
    relayout(x);
}

/// zk_fit for the page; zk_relayout: a new layout when the main area crosses 1000 pixels
pub(super) fn fit(x: &mut HubW, w: i32, h: i32) {
    let (th, f) = (x.hub.t.clone(), faces(x));
    let Some(up) = x.up.as_mut() else { return };
    up.fit(&mut x.hub.ui, &x.fonts, &th, &f, x.hub.zk_f, w, h);
    let lay = layout_for(x);
    if x.up.as_ref().is_some_and(|u| u.made != lay) {
        let d = x.up_d.clone();
        let cm10 = (100.0 * x.dpi / 25.4 + 0.5) as i32;
        let up = x.up.as_mut().unwrap();
        up.render(&mut x.hub.ui, &th, &f, cm10, &d, lay.0, lay.1);
        up.draw_clock(&mut x.hub.ui, &th, &f, &d);
        up.fit(&mut x.hub.ui, &x.fonts, &th, &f, x.hub.zk_f, w, h);
    }
    let up = x.up.as_mut().unwrap();
    let sc = up.sc.clone();
    let y = x.hub.ui.ws[sc.cv].yoff;
    sc.set_y(&mut x.hub.ui, y);
    x.hub.ui.layout_at(&x.fonts, w, h);
    sc.draw_bar(&mut x.hub.ui, false, &th);
}

/// up_refresh: the page again when what it shows changed (not while she's typing in one of its fields)
pub(super) fn refresh(x: &mut HubW) {
    if x.up.is_none() || x.hub.side.page.as_deref() != Some("deadlines") {
        return;
    }
    let mut d = data();
    let typing = x.hub.ui.focus.is_some_and(|f| matches!(x.hub.ui.ws[f].kind, Kind::Entry { .. }));
    // (the clock's time isn't a change: the hands tick on their own)
    d.now_l = x.up_d.now_l;
    if d != x.up_d && !typing && x.up.as_ref().is_some_and(|u| u.paint.is_none()) {
        render(x);
        let (th, f) = (x.hub.t.clone(), faces(x));
        let d = x.up_d.clone();
        if let Some(up) = x.up.as_mut() {
            up.draw_clock(&mut x.hub.ui, &th, &f, &d);
        }
    }
}

/// up_tick: the hands move every second
pub(super) fn tick(x: &mut HubW) {
    let th = x.hub.t.clone();
    let mut d = x.up_d.clone();
    let unix = zenko_desktop::unix_now();
    d.now_l = unix + zenko_desktop::local_offset_pub() as f64;
    if let Some(up) = x.up.as_mut() {
        up.tick(&mut x.hub.ui, &th, &d);
    }
}

fn redraw_clock(x: &mut HubW) {
    let (th, f) = (x.hub.t.clone(), faces(x));
    let d = data();
    if let Some(up) = x.up.as_mut() {
        up.draw_clock(&mut x.hub.ui, &th, &f, &d);
        up.info_default(&mut x.hub.ui, &d);
    }
    x.up_d = d;
}

/// up_set_hour(h, on): mark a half hour free (or not) on the clock's weekday
fn set_hour(x: &HubW, h: f64, on: bool) {
    let Some(up) = x.up.as_ref() else { return };
    let mut cur = up.up_hours(&x.up_d);
    let h = (h * 2.0).round_ties_even() / 2.0;
    if on {
        if !cur.contains(&h) {
            cur.push(h);
        }
    } else {
        cur.retain(|v| *v != h);
    }
    cur.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let key = up.day.unwrap_or(0).to_string();
    let date = zenko_desktop::plan::iso(up.up_date(&x.up_d));
    crate::win::with_fox(move |fox, _| fox.life_do(|l| {
        l["free"][&key] = serde_json::json!(cur);
        if let Some(o) = l["free_dates"].as_object_mut() {
            o.remove(&date);
        }
        l["free_set"] = serde_json::json!(true);
    }));
}

fn replan() {
    crate::win::with_fox(|fox, _| {
        fox.replan();
    });
}

fn say(s: String, secs: f64) {
    crate::win::with_fox(move |fox, _| fox.say(&s, secs));
}

/// the page's buttons
pub(super) fn cmd(x: &mut HubW, cmd: &str) {
    let Some(up) = x.up.as_ref() else { return };
    if let Some(k) = cmd.strip_prefix("up:") {
        let Some(what) = k.parse::<usize>().ok().and_then(|k| up.cmds.get(k).cloned()) else { return };
        if let Some(r) = what.strip_prefix("bump:") {
            let p: Vec<&str> = r.rsplitn(3, ':').collect();
            if p.len() == 3 {
                let (k, cur, id) = (p[0].parse::<f64>().unwrap_or(0.0), p[1].parse::<f64>().unwrap_or(3.0), p[2].to_string());
                crate::win::with_fox(move |fox, _| {
                    fox.life_do(|l| l["plan_hours"][&id] = serde_json::json!((cur + k).clamp(0.5, 60.0)));
                    fox.replan();
                });
            }
        } else if let Some(i) = what.strip_prefix("start:").and_then(|k| k.parse::<usize>().ok()) {
            crate::win::with_fox(move |fox, t| {
                if fox.session.is_some() {
                    fox.say("Finish or stop the current session first.", 5.0);
                } else if i < fox.tt.len() {
                    let it = fox.tt[i].clone();
                    let task = it["task"].as_str().map(String::from).unwrap_or_default();
                    fox.start_session_item(t, &task, it["mins"].as_f64().unwrap_or(30.0), Some(i));
                }
            });
        } else if let Some((act, key)) = what.split_once(':') {
            let parts: Vec<String> = key.split('\u{1f}').map(String::from).collect();
            let del = act == "dl_del";
            crate::win::with_fox(move |fox, _| {
                let mut its = zenko_desktop::plan::load_deadlines();
                if let Some(p) = its.iter().position(|v| (!parts[0].is_empty() && v["id"].as_str() == Some(&parts[0]))
                    || (v["title"].as_str() == Some(&parts[1]) && v["date"].as_str() == Some(&parts[2]))) {
                    if del {
                        its.remove(p);
                    } else {
                        its[p]["done"] = serde_json::json!(true);
                    }
                }
                zenko_desktop::plan::save_deadlines(&mut its);
                fox.plan_maybe();
            });
        }
        render(x);
        redraw_clock(x);
        return;
    }
    match cmd {
        "up_file" => crate::win::with_fox(|fox, _| fox.events.push(zenko_desktop::Ev::Command("pick_files".into()))).unwrap_or(()),
        "up_add" => {
            let e = up.entry;
            let q = x.hub.ui.text_of(e).trim().to_string();
            if q.is_empty() {
                return;
            }
            let res = crate::win::with_fox(move |fox, t| fox.life_add_typed(t, &q)).unwrap_or_default();
            if let Kind::Entry { e, .. } = &mut x.hub.ui.ws[e].kind {
                e.text.clear();
                (e.pos, e.anchor, e.sel, e.left) = (0, 0, None, 0);
            }
            if let Some(up) = x.up.as_mut() {
                up.set_status(&mut x.hub.ui, &res);
            }
            render(x);
            redraw_clock(x);
        }
        "up_replan" => {
            crate::win::with_fox(|fox, t| {
                let r = fox.plan_reply(t);
                fox.fox_reply(t, &r);
            });
            render(x);
            redraw_clock(x);
        }
        "up_flip" | "up_copy" | "up_clear" => {
            if cmd == "up_flip" {
                if let Some(up) = x.up.as_mut() {
                    up.pm = !up.pm;
                }
            } else if cmd == "up_copy" {
                let hrs = up.up_hours(&x.up_d);
                let h2 = hrs.clone();
                crate::win::with_fox(move |fox, _| {
                    fox.life_do(|l| {
                        for k in 0..5 {
                            l["free"][k.to_string()] = serde_json::json!(h2);
                        }
                        l["free_set"] = serde_json::json!(true);
                    });
                    fox.replan();
                });
                if let Some(up) = x.up.as_mut() {
                    up.set_status(&mut x.hub.ui, &format!("Copied {} to Monday-Friday.", zenko_desktop::uppage::slots_text(&hrs)));
                }
            } else {
                let key = up.day.unwrap_or(0).to_string();
                let date = zenko_desktop::plan::iso(up.up_date(&x.up_d));
                crate::win::with_fox(move |fox, _| {
                    fox.life_do(|l| {
                        l["free"][&key] = serde_json::json!([]);
                        if let Some(o) = l["free_dates"].as_object_mut() {
                            o.remove(&date);
                        }
                        l["free_set"] = serde_json::json!(true);
                    });
                    fox.replan();
                });
            }
            render(x);
            redraw_clock(x);
        }
        _ => {
            if let Some(i) = cmd.strip_prefix("up_day:").and_then(|k| k.parse::<usize>().ok()) {
                if let Some(up) = x.up.as_mut() {
                    up.day = Some(i);
                }
            } else if let Some(k) = cmd.strip_prefix("up_month:").and_then(|k| k.parse::<i64>().ok()) {
                if let Some(up) = x.up.as_mut() {
                    let (y, m) = up.month;
                    let t = y * 12 + (m - 1) + k;
                    up.month = (t.div_euclid(12), t.rem_euclid(12) + 1);
                }
            } else if cmd == "up_pick:none" {
                if let Some(up) = x.up.as_mut() {
                    up.dl_day = None;
                }
            } else if let Some(on) = cmd.strip_prefix("up_quick:") {
                quick(x, on == "1");
                return;
            } else if let Some(r) = cmd.strip_prefix("ib_add:") {
                let (kind, sid) = r.split_once(':').unwrap_or((r, ""));
                let (kind, sid) = (kind.to_string(), sid.to_string());
                crate::win::with_fox(move |fox, _| {
                    fox.ib_add(&kind, &sid);
                });
            } else if let Some(tid) = cmd.strip_prefix("ib_words:") {
                ib_words(x, tid);
                return;
            } else if let Some(tid) = cmd.strip_prefix("ib_count:") {
                let c = format!("ib_count_file:{tid}");
                crate::win::with_fox(move |fox, _| fox.events.push(zenko_desktop::Ev::Command(c)));
                return;
            } else {
                return;
            }
            render(x);
            redraw_clock(x);
        }
    }
}

/// the IA's "Update" (or Return in its box): log the words typed there
fn ib_words(x: &mut HubW, tid: &str) {
    let Some(up) = x.up.as_ref() else { return };
    let tasks = x.up_d.life["ibtasks"].as_array().cloned().unwrap_or_default();
    let Some(ti) = tasks.iter().position(|t| t["id"].as_str() == Some(tid)) else { return };
    let Some(&(we, _, _)) = up.ib_entries.iter().find(|e| e.2 == ti) else { return };
    let s: String = x.hub.ui.text_of(we).chars().filter(|c| c.is_ascii_digit()).collect();
    if let Ok(n) = s.parse::<i64>() {
        let msg = crate::win::with_fox(move |fox, _| fox.ib_set_words(ti, n)).unwrap_or_default();
        say(msg, 8.0);
        render(x);
        redraw_clock(x);
    }
}

/// the due box (Return / leaving it): parse_day or 2027-03-15
fn ib_due(x: &mut HubW, de: usize, ti: usize) {
    let txt = x.hub.ui.text_of(de).trim().to_string();
    let cur = x.up_d.life["ibtasks"][ti]["due"].as_str().unwrap_or("").to_string();
    if txt.is_empty() || txt == cur {
        return;
    }
    let today = x.up_d.today;
    let d = zenko_desktop::lifechat::parse_rel_date(&txt, today).or_else(|| zenko_desktop::plan::parse_iso(&txt));
    match d {
        Some(d) => {
            let iso = zenko_desktop::plan::iso(d);
            crate::win::with_fox(move |fox, _| fox.life_do(|l| l["ibtasks"][ti]["due"] = serde_json::json!(iso)));
            render(x);
            redraw_clock(x);
        }
        None => say("Type the date like 2027-03-15~".into(), 5.0),
    }
}

/// z21_quick_add's Add / Remove: every half hour from the first menu's time to the second's
fn quick(x: &mut HubW, on: bool) {
    let Some(up) = x.up.as_ref() else { return };
    let tv = |s: String| {
        let (h, m) = s.split_once(':').unwrap_or(("0", "0"));
        h.parse::<f64>().unwrap_or(0.0) + m.parse::<f64>().unwrap_or(0.0) / 60.0
    };
    let (a, b) = (tv(x.hub.ui.text_of(up.m1)), tv(x.hub.ui.text_of(up.m2)));
    if b <= a {
        if let Some(up) = x.up.as_mut() {
            up.set_status(&mut x.hub.ui, "Pick an end time after the start time.");
        }
        relayout(x);
        return;
    }
    let mut s = a;
    while s < b - 1e-6 {
        set_hour(x, s, on);
        x.up_d = data();
        s += 0.5;
    }
    if let Some(up) = x.up.as_mut() {
        up.pm = a >= 12.0;
    }
    replan();
    render(x);
    redraw_clock(x);
}

/// a click on the page that isn't a button: the clock ring, a calendar day, the ✕ and milestone labels, a menu
pub(super) fn press(x: &mut HubW, mx: i32, my: i32) -> bool {
    if x.hub.side.page.as_deref() != Some("deadlines") {
        return false;
    }
    let Some(up) = x.up.as_ref() else { return false };
    let inside = |id: usize| {
        let (ax, ay) = x.hub.ui.abs(id);
        x.hub.ui.ws[id].mapped && mx >= ax && my >= ay && mx < ax + x.hub.ui.ws[id].w && my < ay + x.hub.ui.ws[id].h
    };
    if up.clock != 0 && inside(up.clock) {
        let (ax, ay) = x.hub.ui.abs(up.clock);
        if let Some(h) = up.hit((mx - ax) as f64, (my - ay) as f64) {
            let on = !up.up_hours(&x.up_d).contains(&h);
            set_hour(x, h, on);
            if let Some(up) = x.up.as_mut() {
                up.paint = Some(on);
            }
            out("sound:toggle");
            redraw_clock(x);
            relayout(x);
        }
        return true;
    }
    if up.cal != 0 && inside(up.cal) {
        let (ax, ay) = x.hub.ui.abs(up.cal);
        if let Some(day) = up.cal_day(&x.up_d, (mx - ax) as f64, (my - ay) as f64) {
            if let Some(up) = x.up.as_mut() {
                up.dl_day = if up.dl_day == Some(day) { None } else { Some(day) };
            }
            render(x);
            redraw_clock(x);
        }
        return true;
    }
    for (m, which) in [(up.m1, 1), (up.m2, 2)] {
        if inside(m) {
            let _ = which;
            x.up_menu = Some(m);
            return true;
        }
    }
    if let Some((_, what)) = up.clicks.iter().find(|(l, _)| inside(*l)).cloned() {
        let tasks = x.up_d.life["ibtasks"].as_array().cloned().unwrap_or_default();
        if let Some(tid) = what.strip_prefix("ib_del:") {
            if let Some(ti) = tasks.iter().position(|t| t["id"].as_str() == Some(tid)) {
                crate::win::with_fox(move |fox, _| fox.life_do(|l| {
                    if let Some(a) = l["ibtasks"].as_array_mut()
                        && ti < a.len() {
                        a.remove(ti);
                    }
                }));
            }
        } else if let Some(r) = what.strip_prefix("ib_stage:") {
            let (tid, st) = r.split_once(':').unwrap_or((r, ""));
            if let Some(ti) = tasks.iter().position(|t| t["id"].as_str() == Some(tid)) {
                let (st, title) = (st.to_string(), tasks[ti]["title"].as_str().unwrap_or("").to_string());
                let today = zenko_desktop::plan::iso(x.up_d.today);
                let added = crate::win::with_fox(move |fox, _| fox.life_do(|l| {
                    let t = &mut l["ibtasks"][ti];
                    if !t["stages"].is_array() {
                        t["stages"] = serde_json::json!([]);
                    }
                    let a = t["stages"].as_array_mut().unwrap();
                    let had = a.iter().position(|v| v.as_str() == Some(&st));
                    match had {
                        Some(p) => {
                            a.remove(p);
                        }
                        None => a.push(serde_json::json!(st)),
                    }
                    t["updated"] = serde_json::json!(today);
                    had.is_none().then_some(st)
                })).flatten();
                if let Some(st) = added {
                    out("sound:toggle");
                    if st == "Final" {
                        say(format!("{title} done?! Sugoi~ omedetou! *happy tail swish*"), 8.0);
                        crate::win::with_fox(|fox, t| fox.set_mood(t, "happy", 6.0));
                    }
                }
            }
        }
        render(x);
        redraw_clock(x);
        return true;
    }
    false
}

/// the drag over the ring keeps marking (or unmarking); the info line says what's under the pointer
pub(super) fn motion(x: &mut HubW, mx: i32, my: i32) -> bool {
    if x.hub.side.page.as_deref() != Some("deadlines") {
        return false;
    }
    let Some(up) = x.up.as_ref() else { return false };
    if up.clock == 0 || !x.hub.ui.ws[up.clock].mapped {
        return false;
    }
    let (ax, ay) = x.hub.ui.abs(up.clock);
    let (lx, ly) = ((mx - ax) as f64, (my - ay) as f64);
    let s = up.s as f64;
    let over = lx >= 0.0 && ly >= 0.0 && lx < s && ly < s;
    if let Some(on) = up.paint
        && let Some(h) = up.hit(lx, ly)
        && up.up_hours(&x.up_d).contains(&h) != on {
        set_hour(x, h, on);
        redraw_clock(x);
    }
    let d = x.up_d.clone();
    let Some(up) = x.up.as_ref() else { return false };
    let before = x.hub.ui.text_of(up.info);
    if over {
        up.hover(&mut x.hub.ui, &d, lx, ly);
        x.cursor = IDC_HAND;
    } else {
        up.info_default(&mut x.hub.ui, &d);
    }
    let ch = x.hub.ui.text_of(up.info) != before;
    if ch {
        relayout(x);
    }
    ch
}

/// up_release: the painting ends, the plan follows
pub(super) fn release(x: &mut HubW) {
    let Some(up) = x.up.as_mut() else { return };
    if up.paint.take().is_some() {
        replan();
        render(x);
        redraw_clock(x);
    }
}

/// the OptionMenu's menu (run after the click is handled, outside the window state: TrackPopupMenu's loop sends
/// messages of its own): the 49 half-hour times, the chosen one becomes the button's text
pub(super) unsafe fn popup(hwnd: HWND, m: usize) {
    let Some((ax, ay, h, cur)) = with(|x| {
        let (ax, ay) = x.hub.ui.abs(m);
        (ax, ay, x.hub.ui.ws[m].h, x.hub.ui.text_of(m))
    }) else { return };
    let times = quick_times();
    let mut p = POINT { x: ax, y: ay + h };
    let r = unsafe {
        ClientToScreen(hwnd, &mut p);
        let pm = CreatePopupMenu();
        for (i, t) in times.iter().enumerate() {
            let w: Vec<u16> = t.encode_utf16().chain(std::iter::once(0)).collect();
            let fl = MF_STRING | if *t == cur { MF_CHECKED } else { 0 } | if i > 0 && i % 16 == 0 { MF_MENUBARBREAK } else { 0 };
            AppendMenuW(pm, fl, i + 1, w.as_ptr());
        }
        let r = TrackPopupMenu(pm, TPM_RETURNCMD | TPM_LEFTALIGN | TPM_TOPALIGN, p.x, p.y, 0, hwnd, std::ptr::null());
        DestroyMenu(pm);
        r
    };
    with(|x| {
        if r > 0
            && let Some(t) = times.get(r as usize - 1)
            && let Kind::Menu { text, .. } = &mut x.hub.ui.ws[m].kind {
            *text = t.clone();
        }
        relayout(x);
    });
}

/// Return in one of the page's boxes
pub(super) fn key_return(x: &mut HubW) -> bool {
    let Some(up) = x.up.as_ref() else { return false };
    let f = x.hub.ui.focus;
    if f == Some(up.entry) {
        cmd(x, "up_add");
        return true;
    }
    if let Some(&(we, de, ti)) = up.ib_entries.iter().find(|e| Some(e.0) == f || Some(e.1) == f) {
        let tid = x.up_d.life["ibtasks"][ti]["id"].as_str().unwrap_or("").to_string();
        if f == Some(we) {
            ib_words(x, &tid);
        } else {
            ib_due(x, de, ti);
        }
        return true;
    }
    false
}

/// leaving a due box saves it (its <FocusOut>)
pub(super) fn focus_left(x: &mut HubW, old: Option<usize>) {
    let Some(up) = x.up.as_ref() else { return };
    if old == x.hub.ui.focus {
        return;
    }
    if let Some(&(_, de, ti)) = up.ib_entries.iter().find(|e| Some(e.1) == old) {
        ib_due(x, de, ti);
    }
}
