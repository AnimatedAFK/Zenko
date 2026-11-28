//! The hub's Settings page in the window: building it, its checkboxes, radios, menus, boxes and buttons, the spirit
//! colour and bib swatches, the speed slider, the section chips that follow the scroll (zenko.py hub_build_settings,
//! its zenko13..23 cards and z19_settings_nav).

use super::*;
use serde_json::{Value, json};
use zenko_desktop::setpage::{HOTKEYS, PIPER_VOICES, RULE_SITES, SetData, SetPage};

fn data(x: &HubW) -> SetData {
    let mut d = crate::win::with_fox(|fox, _| fox.set_data()).unwrap_or_default();
    d.font_ui = x.hub.font_ui.clone();
    let pages = x.pages.len();
    // (zenko.py also counts its pre-drawn picture cache here; the port draws the art as it goes and keeps none)
    d.speed = format!("This window opened {:.1} s after start.  Pages ready: {}.", x.opened_s, pages);
    d.rule_try = try_title(&d.cfg, "Cell biology: mitosis explained - YouTube", &x.hub.t);
    d
}

/// test(): what the rules (then the built-in list) do with a window title
fn try_title(cfg: &Value, title: &str, th: &zenko_desktop::theme::Theme) -> (String, String) {
    let rules = cfg.get("zk_rules").cloned().unwrap_or(Value::Null);
    match zenko_desktop::watch::rule_verdict(&rules, title, "") {
        Some(("keep", _)) => ("\u{2713} kept open (your rule)".into(), "#5fae7e".into()),
        Some(("close", _)) => ("\u{2715} closed (your rule)".into(), "#e5484d".into()),
        _ => {
            let t = title.to_string();
            let d = crate::win::with_fox(move |fox, n| fox.is_distracting(n, &t, "")).flatten();
            match d {
                Some(d) => (format!("\u{2715} closed (built-in list: {d})"), "#e5484d".into()),
                None => ("\u{2713} allowed (no rule needed)".into(), th.grey.clone()),
            }
        }
    }
}

/// hub_build_settings(pg)
pub(super) fn build(x: &mut HubW, pg: usize) {
    let th = x.hub.t.clone();
    let cm10 = (100.0 * x.dpi / 25.4 + 0.5) as i32;
    let d = data(x);
    let f = faces(x);
    x.set = Some(SetPage::build(&mut x.hub.ui, pg, &th, &f, cm10, &d));
    x.set_try = String::new();
}

pub(super) fn fit(x: &mut HubW, w: i32, h: i32) {
    let (th, f) = (x.hub.t.clone(), faces(x));
    let Some(p) = x.set.as_mut() else { return };
    let y = x.hub.ui.ws[p.sc.cv].yoff;
    p.fit(&mut x.hub.ui, &x.fonts, &th, &f, x.hub.zk_f, w, h);
    let sc = p.sc.clone();
    sc.set_y(&mut x.hub.ui, y);
    x.hub.ui.layout_at(&x.fonts, w, h);
    sc.draw_bar(&mut x.hub.ui, false, &th);
}

/// the page made again from the config (zenko.py: hub_close + open_hub("settings"), or a card's redraw()); the scroll
/// stays where it was
fn rebuild(x: &mut HubW) {
    let Some(old) = x.set.take() else { return };
    let y = x.hub.ui.ws[old.sc.cv].yoff;
    let pg = old.pg;
    let kids = x.hub.ui.ws[pg].kids.clone();
    for k in kids {
        x.hub.ui.destroy(k);
    }
    build(x, pg);
    relayout(x);
    if let Some(p) = x.set.as_ref() {
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

fn say(text: &str, secs: f64) {
    let t = text.to_string();
    crate::win::with_fox(move |fox, _| fox.say(&t, secs));
}

fn label(x: &HubW, name: &str) -> Option<usize> {
    x.set.as_ref()?.labels.iter().find(|(k, _)| k == name).map(|p| p.1)
}

fn set_label(x: &mut HubW, name: &str, text: &str, fg: Option<&str>) {
    if let Some(l) = label(x, name)
        && let Kind::Label { text: t, fg: f, .. } = &mut x.hub.ui.ws[l].kind {
        *t = text.into();
        if let Some(c) = fg {
            *f = c.into();
        }
    }
}

fn entry_text(x: &HubW, key: &str) -> String {
    x.set.as_ref().and_then(|p| p.entries.iter().find(|(k, _)| k == key).map(|e| e.1))
        .map(|e| x.hub.ui.text_of(e)).unwrap_or_default()
}

fn entry_put(x: &mut HubW, key: &str, s: &str) {
    let Some(id) = x.set.as_ref().and_then(|p| p.entries.iter().find(|(k, _)| k == key).map(|e| e.1)) else { return };
    if let Kind::Entry { e, .. } = &mut x.hub.ui.ws[id].kind {
        e.text = s.encode_utf16().collect();
        e.pos = e.text.len();
        e.left = 0;
        e.sel = None;
    }
}

fn check_on(x: &HubW, cmd: &str) -> bool {
    x.hub.ui.ws.iter().any(|w| matches!(&w.kind, Kind::Check { on: true, cmd: c, .. } if c == cmd))
}

/// a box's value saved when it loses the keyboard or gets Return (save_num and the plain text boxes)
fn save_entry(x: &mut HubW, key: &str) {
    let v = entry_text(x, key);
    match key {
        "break_after_minutes" | "break_minutes" => match v.trim().parse::<f64>() {
            Ok(n) => cfg_set(key, json!((n as i64).clamp(0, 600))),
            Err(_) => {
                let d = if key == "break_minutes" { "5" } else { "50" };
                let cur = cfg_get(key);
                let s = if cur.is_null() { d.to_string() } else { cur.to_string().trim_matches('"').to_string() };
                entry_put(x, key, &s);
            }
        },
        "lockin_minutes" => {
            let n = v.trim().parse::<f64>().map(|n| (n as i64).clamp(1, 600)).unwrap_or(50);
            cfg_set(key, json!(n));
        }
        "square_server" => {
            // setsrv: a server that isn't http(s)://host[:port][/path] goes back to ntfy.sh; the square reconnects
            let mut v = v.trim().trim_end_matches('/').to_string();
            let ok = (|| {
                let rest = v.strip_prefix("https://").or_else(|| v.strip_prefix("http://"))?;
                let (hp, _) = rest.split_once('/').unwrap_or((rest, ""));
                let (h, port) = hp.split_once(':').map(|(h, p)| (h, Some(p))).unwrap_or((hp, None));
                let good = !h.is_empty() && h.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '.' || c == '-')
                    && port.is_none_or(|p| !p.is_empty() && p.bytes().all(|b| b.is_ascii_digit()));
                good.then_some(())
            })().is_some();
            if !ok {
                v = zenko_desktop::squarelink::SQUARE_SERVER.into();
                entry_put(x, key, &v);
            }
            cfg_set(key, json!(v));
            crate::win::with_fox(|fox, _| crate::sqnet::connect(&fox.cfg));
        }
        "home_city" | "spotify_focus_playlist" | "vision_model" | "spotify_client_id" => {
            cfg_set(key, json!(v.trim()));
        }
        "online_key" => {
            let prov = cfg_get("online_provider").as_str().unwrap_or("gemini").to_string();
            let mut ks = cfg_get("online_keys");
            if !ks.is_object() {
                ks = json!({});
            }
            ks[prov] = json!(v.trim());
            cfg_set("online_keys", ks);
        }
        k if k.starts_with("preset_") => save_presets(x),
        _ => {}
    }
}

/// save_p(): the music presets from their rows
fn save_presets(x: &mut HubW) {
    let mut ps = cfg_get("music_presets").as_array().cloned().unwrap_or_default();
    let kinds = [("Playlist", "playlist"), ("Song", "track"), ("Album", "album"), ("Artist", "artist")];
    for (i, p) in ps.iter_mut().enumerate() {
        let name = entry_text(x, &format!("preset_name:{i}"));
        let kind = x.hub.ui.ws.iter().find_map(|w| match &w.kind {
            Kind::Menu { text, cmd, .. } if *cmd == format!("set_menu:preset_kind:{i}") => Some(text.clone()),
            _ => None,
        }).unwrap_or_default();
        p["name"] = json!(if name.trim().is_empty() { format!("Preset {}", i + 1) } else { name.trim().to_string() });
        p["kind"] = json!(kinds.iter().find(|(k, _)| *k == kind).map(|(_, v)| *v).unwrap_or("playlist"));
        p["value"] = json!(entry_text(x, &format!("preset_value:{i}")).trim());
    }
    cfg_set("music_presets", Value::Array(ps));
}

/// every frame while the page shows: a box that lost the keyboard saves (<FocusOut>), the try-a-title result follows
/// its box (<KeyRelease>), the chips follow the scroll
pub(super) fn step(x: &mut HubW) {
    if x.set.is_none() || x.hub.side.page.as_deref() != Some("settings") {
        return;
    }
    let f = x.hub.ui.focus;
    if f != x.set_focus {
        if let Some(old) = x.set_focus
            && let Some(key) = x.set.as_ref().and_then(|p| p.entries.iter().find(|e| e.1 == old).map(|e| e.0.clone())) {
            save_entry(x, &key);
        }
        x.set_focus = f;
    }
    let t = entry_text(x, "rule_try");
    if t != x.set_try {
        x.set_try = t.clone();
        let cfg = crate::win::with_fox(|fox, _| fox.cfg.raw.clone()).unwrap_or_default();
        let (s, c) = try_title(&cfg, &t, &x.hub.t);
        set_label(x, "rule_res", &s, Some(&c));
        relayout(x);
    }
    let (th, f) = (x.hub.t.clone(), faces(x));
    if let Some(p) = x.set.as_mut() {
        let cur = p.cur;
        p.follow(&mut x.hub.ui, &th, &f, &x.fonts);
        if p.cur != cur {
            relayout(x);
        }
    }
}

/// Return in one of the page's boxes
pub(super) fn key_return(x: &mut HubW) -> bool {
    let Some(f) = x.hub.ui.focus else { return false };
    let Some(key) = x.set.as_ref().and_then(|p| p.entries.iter().find(|e| e.1 == f).map(|e| e.0.clone())) else {
        return false;
    };
    match key.as_str() {
        "exam_date" => cmd(x, "set_btn:save_exam"),
        "study_video_words" => cmd(x, "set_btn:save_words"),
        "update_source" => cmd(x, "set_btn:update_save"),
        "rule_words" | "rule_other" => cmd(x, "set_btn:rule_add"),
        _ => {
            save_entry(x, &key);
            relayout(x);
        }
    }
    true
}

/// the radios of a group: the chosen one on, the rest off (the shared variable)
fn radio_group(x: &mut HubW, key: &str, val: &str) {
    let pre = format!("set_rad:{key}:");
    for w in x.hub.ui.ws.iter_mut() {
        if let Kind::Check { on, cmd, radio: true, .. } = &mut w.kind
            && let Some(r) = cmd.strip_prefix(&pre) {
            *on = r.split_once(':').map(|(_, v)| v) == Some(val);
        }
    }
}

/// the checkboxes, radios, menus' picks and the buttons
pub(super) fn cmd(x: &mut HubW, c: &str) {
    if let Some(key) = c.strip_prefix("set_chk:") {
        let on = check_on(x, c);
        match key {
            "bubble_font" => {
                cfg_set(key, json!(if on { "pixel" } else { "app" }));
                say("Like this?", 3.0);
            }
            "web_enabled" => {
                cfg_set(key, json!(on));
                say(&format!("Internet lookups are {}.", if on { "on" } else { "off" }), 4.0);
            }
            "autostart" => {
                // (the preview leaves Start with Windows to the Zenko you use: it would take over its Run entry)
                if let Kind::Check { on, .. } = &mut x.hub.ui.ws.iter_mut().find(|w| matches!(&w.kind,
                    Kind::Check { cmd, .. } if cmd == c)).unwrap().kind {
                    *on = false;
                }
                say("This preview leaves Start with Windows to your normal Zenko.", 4.0);
            }
            "square_on" => {
                cfg_set(key, json!(on));
                crate::win::with_fox(|fox, _| if on { crate::sqnet::connect(&fox.cfg) } else { crate::sqnet::disconnect() });
            }
            _ => {
                cfg_set(key, json!(on));
                if matches!(key, "foxfire" | "glasses_in_session" | "headphones_with_music" | "belly_music" | "hd_art") {
                    crate::win::with_fox(|fox, _| fox.look_changed());
                }
            }
        }
    } else if let Some(r) = c.strip_prefix("set_rad:") {
        let mut it = r.splitn(3, ':');
        let (key, ty, val) = (it.next().unwrap_or(""), it.next().unwrap_or("s"), it.next().unwrap_or(""));
        radio_group(x, key, val);
        if key == "rule_do" {
            x.set_rule_do = val.into();
        } else {
            let v = match ty {
                "i" => json!(val.parse::<i64>().unwrap_or(0)),
                "f" => json!(val.parse::<f64>().unwrap_or(1.0)),
                _ => json!(val),
            };
            match key {
                "size" => out(&format!("run:size:{val}")),
                "tails" => {
                    cfg_set(key, v);
                    crate::win::with_fox(|fox, _| fox.look_changed());
                }
                "text_scale" => {
                    cfg_set(key, v);
                    rebuild(x);
                    return;
                }
                "speak_mode" => {
                    cfg_set(key, v);
                }
                _ => cfg_set(key, v),
            }
        }
    } else if let Some(r) = c.strip_prefix("set_scale:") {
        let v = x.set.as_ref().map(|p| p.scale).and_then(|s| match x.hub.ui.ws[s].kind {
            Kind::Scale { val, .. } => Some(val),
            _ => None,
        }).unwrap_or(0.0);
        cfg_set(r, json!(v as i64));
    } else if let Some(b) = c.strip_prefix("set_btn:") {
        button(x, b);
    }
    relayout(x);
}

fn button(x: &mut HubW, b: &str) {
    let th = x.hub.t.clone();
    match b {
        "save_exam" => {
            let v = entry_text(x, "exam_date").trim().to_string();
            let msg = crate::win::with_fox(move |fox, n| fox.exam_from_text(n, &v, 0)).unwrap_or_default();
            set_label(x, "exam", &msg, None);
        }
        "save_words" => {
            let v = entry_text(x, "study_video_words");
            let words: Vec<String> = v.split(',').map(|w| w.trim().to_lowercase()).filter(|w| !w.is_empty()).collect();
            let n = words.len();
            cfg_set("study_video_words", json!(words));
            set_label(x, "words", &format!("Saved {n} words."), None);
        }
        "update_save" => {
            let v = entry_text(x, "update_source").trim().to_string();
            let empty = v.is_empty();
            cfg_set("update_source", json!(v));
            cfg_set("update_checked", json!(0));
            set_label(x, "update", if empty { "Updates are off until you add a source." } else { "Saved." }, None);
        }
        "lockin" => {
            let on = crate::win::with_fox(|fox, _| fox.lockin.is_some()).unwrap_or(false);
            let n = entry_text(x, "lockin_minutes").trim().parse::<f64>().map(|n| (n as i64).clamp(1, 600)).unwrap_or(50);
            cfg_set("lockin_minutes", json!(n));
            let msg = crate::win::with_fox(move |fox, t| if on { fox.lockin_stop(t, false, false) } else {
                fox.lockin_start(t, Some(n), false).unwrap_or_default()
            }).unwrap_or_default();
            set_label(x, "lockin", &msg, None);
            rebuild(x);
        }
        "lockin_save" => {
            let txt = x.set.as_ref().map(|p| p.lock_text).and_then(|t| match &x.hub.ui.ws[t].kind {
                Kind::Text { t, .. } => Some(t.get()),
                _ => None,
            }).unwrap_or_default();
            let items: Vec<String> = txt.lines().map(|l| l.trim().to_string()).filter(|l| !l.is_empty()).collect();
            let n = items.len();
            cfg_set("lockin_extra", json!(items));
            set_label(x, "lockin", &format!("Saved {n} extra item{}.", if n == 1 { "" } else { "s" }), None);
        }
        "rule_add" => {
            let mut site = x.hub.ui.ws.iter().find_map(|w| match &w.kind {
                Kind::Menu { text, cmd, .. } if cmd == "set_menu:rule_site" => Some(text.clone()),
                _ => None,
            }).unwrap_or_else(|| "YouTube".into());
            if site == "Other\u{2026}" {
                site = entry_text(x, "rule_other").trim().to_string();
            }
            let words = zenko_desktop::watch::rule_words(&entry_text(x, "rule_words")).join(", ");
            if (site.is_empty() || site.to_lowercase().starts_with("any")) && words.is_empty() {
                set_label(x, "rule_msg", "Pick a site, or type some words for 'Any site or app'.", None);
                return;
            }
            let mut rules = cfg_get("zk_rules").as_array().cloned().unwrap_or_default();
            let do_ = if x.set_rule_do.is_empty() { "keep".to_string() } else { x.set_rule_do.clone() };
            rules.push(json!({"site": site, "words": words, "do": do_, "on": true}));
            let msg = format!("Added: {}", zenko_desktop::routes17::rule_text(rules.last().unwrap()));
            cfg_set("zk_rules", Value::Array(rules));
            rebuild(x);
            set_label(x, "rule_msg", &msg, None);
        }
        _ if b.starts_with("rule_on:") => {
            let i: usize = b[8..].parse().unwrap_or(usize::MAX);
            let mut rules = cfg_get("zk_rules").as_array().cloned().unwrap_or_default();
            if let Some(r) = rules.get_mut(i) {
                let on = r.get("on").and_then(|v| v.as_bool()).unwrap_or(true);
                r["on"] = json!(!on);
                cfg_set("zk_rules", Value::Array(rules));
            }
            rebuild(x);
        }
        "speak_test" | "test_voice" => {
            crate::win::with_fox(|fox, _| fox.trace.push("speak_test".into()));
            out("run:speak_test");
        }
        "update_check" => out("run:update_check"),
        _ if b.starts_with("preset_use:") || b.starts_with("preset_play:") => {
            save_presets(x);
            out(&format!("run:{b}"));
        }
        _ => {
            // (the installers, downloads, sign-ups and dialogs aren't ported yet: the app is asked for them)
            let s = format!("set_btn:{b}");
            crate::win::with_fox(move |fox, _| fox.trace.push(s));
            let _ = th;
        }
    }
}

/// available_fonts(): FONT_CHOICES (Zen Maru Gothic first) that Windows has
fn available_fonts() -> Vec<String> {
    ["Zen Maru Gothic", "Segoe UI Variable Display", "Segoe UI", "Bahnschrift", "Nunito", "Poppins", "Inter", "Lexend",
     "Calibri", "Trebuchet MS", "Candara", "Corbel", "Georgia", "Verdana"]
        .iter().filter(|f| crate::gdi::font_exists(f)).map(|f| f.to_string()).collect()
}

/// the choices of a menu button
fn menu_items(key: &str) -> Vec<String> {
    match key {
        "app_font" => available_fonts(),
        "voice_key" | "screen_key" => HOTKEYS.iter().map(|s| s.to_string()).collect(),
        "tts_voice" => std::iter::once("Windows default".to_string())
            .chain(PIPER_VOICES.iter().map(|(n, _)| format!("Natural: {n}"))).collect(),
        "rule_site" => RULE_SITES.iter().map(|s| s.to_string()).chain(std::iter::once("Other\u{2026}".into())).collect(),
        k if k.starts_with("preset_kind:") => ["Playlist", "Song", "Album", "Artist"].iter().map(|s| s.to_string()).collect(),
        _ => vec![],
    }
}

/// a menu button pressed: its choices pop up under it (the OptionMenu), and the pick is kept
pub(super) unsafe fn popup(hwnd: HWND, m: usize) {
    let Some((ax, ay, h, cur, key)) = with(|x| {
        let (ax, ay) = x.hub.ui.abs(m);
        let key = match &x.hub.ui.ws[m].kind {
            Kind::Menu { cmd, .. } => cmd.strip_prefix("set_menu:").unwrap_or("").to_string(),
            _ => String::new(),
        };
        (ax, ay, x.hub.ui.ws[m].h, x.hub.ui.text_of(m), key)
    }) else { return };
    let items = menu_items(&key);
    if items.is_empty() {
        return;
    }
    let mut p = POINT { x: ax, y: ay + h };
    let r = unsafe {
        ClientToScreen(hwnd, &mut p);
        let pm = CreatePopupMenu();
        for (i, t) in items.iter().enumerate() {
            let w: Vec<u16> = t.encode_utf16().chain(std::iter::once(0)).collect();
            let fl = MF_STRING | if *t == cur { MF_CHECKED } else { 0 } | if i > 0 && i % 24 == 0 { MF_MENUBARBREAK } else { 0 };
            AppendMenuW(pm, fl, i + 1, w.as_ptr());
        }
        let r = TrackPopupMenu(pm, TPM_RETURNCMD | TPM_LEFTALIGN | TPM_TOPALIGN, p.x, p.y, 0, hwnd, std::ptr::null());
        DestroyMenu(pm);
        r
    };
    with(|x| {
        if r > 0
            && let Some(t) = items.get(r as usize - 1).cloned() {
            if let Kind::Menu { text, .. } = &mut x.hub.ui.ws[m].kind {
                *text = t.clone();
            }
            picked(x, &key, &t);
        }
        relayout(x);
    });
}

fn picked(x: &mut HubW, key: &str, t: &str) {
    match key {
        "app_font" => {
            cfg_set("app_font", json!(t));
            out("run:app_font");
        }
        "voice_key" | "screen_key" => cfg_set(key, json!(t)),
        "tts_voice" => {
            let v = PIPER_VOICES.iter().find(|(n, _)| format!("Natural: {n}") == t).map(|(_, v)| format!("piper:{v}"))
                .unwrap_or_default();
            let piper = !v.is_empty();
            cfg_set("tts_voice", json!(v));
            set_label(x, "voice_status", if piper {
                "Natural voices need a free add-on. Press 'Install natural voices' first."
            } else { "" }, None);
            if !piper {
                out("run:speak_test");
            }
        }
        k if k.starts_with("preset_kind:") => save_presets(x),
        _ => {}
    }
}

/// a press on the page that isn't a button: the colour and bib swatches, the speed slider, the section chips, the
/// rules' arrows and crosses
pub(super) fn click(x: &mut HubW, mx: i32, my: i32) -> bool {
    if x.set.is_none() || x.hub.side.page.as_deref() != Some("settings") {
        return false;
    }
    let inside = |x: &HubW, id: usize| {
        let (ax, ay) = x.hub.ui.abs(id);
        let w = &x.hub.ui.ws[id];
        w.mapped && mx >= ax && my >= ay && mx < ax + w.w && my < ay + w.h
    };
    let p = x.set.as_ref().unwrap();
    let (nav, scale) = (p.nav, p.scale);
    if let Some((_, k)) = p.swatches.iter().find(|(id, _)| inside(x, *id)).cloned() {
        out(&format!("run:theme:{k}"));
        return true;
    }
    if let Some((_, col)) = p.scarves.iter().find(|(id, _)| inside(x, *id)).cloned() {
        cfg_set("scarf", json!(col));
        crate::win::with_fox(|fox, _| fox.look_changed());
        let th = x.hub.t.clone();
        let bg = th.card.clone();
        x.set.as_ref().unwrap().draw_scarves(&mut x.hub.ui, &th, &col, &bg);
        return true;
    }
    if let Some((what, _)) = p.labels.iter().find(|(k, id)| (k.starts_with("rule_move:") || k.starts_with("rule_remove:"))
        && inside(x, *id)).cloned() {
        let mut rules = cfg_get("zk_rules").as_array().cloned().unwrap_or_default();
        let parts: Vec<&str> = what.split(':').collect();
        let i: usize = parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(usize::MAX);
        if i < rules.len() {
            if parts[0] == "rule_remove" {
                rules.remove(i);
            } else {
                let d: i64 = parts.get(2).and_then(|s| s.parse().ok()).unwrap_or(0);
                let j = i as i64 + d;
                if j >= 0 && (j as usize) < rules.len() {
                    rules.swap(i, j as usize);
                }
            }
            cfg_set("zk_rules", Value::Array(rules));
            rebuild(x);
        }
        return true;
    }
    if scale != 0 && inside(x, scale) {
        let (ax, _) = x.hub.ui.abs(scale);
        let w = x.hub.ui.ws[scale].w;
        if let Kind::Scale { from, to, val, slen, .. } = &mut x.hub.ui.ws[scale].kind {
            let rel = ((mx - ax - *slen / 2) as f64 / (w - *slen).max(1) as f64).clamp(0.0, 1.0);
            *val = (*from + rel * (*to - *from)).round();
        }
        cmd(x, "set_scale:tts_speed");
        return true;
    }
    if inside(x, nav) {
        let (ax, _) = x.hub.ui.abs(nav);
        let cx = (mx - ax) as f64;
        if let Some(&(_, _, i)) = x.set.as_ref().unwrap().boxes.iter().find(|(a, b, _)| cx >= *a && cx < *b) {
            let (th, f) = (x.hub.t.clone(), faces(x));
            let p = x.set.as_mut().unwrap();
            p.jump(&mut x.hub.ui, i);
            p.draw_nav(&mut x.hub.ui, &th, &f, &x.fonts);
            relayout(x);
            let th = x.hub.t.clone();
            let sc = x.set.as_ref().unwrap().sc.clone();
            sc.draw_bar(&mut x.hub.ui, false, &th);
        }
        return true;
    }
    false
}
