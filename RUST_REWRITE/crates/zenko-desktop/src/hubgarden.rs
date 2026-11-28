//! The hub's Zen garden page in the window (zenko.py Zenko + Zenko22): the 50 ms garden_anim, the foxfire flicker,
//! the breathing session (breath_start / breath_stop / breath_finish with its tones), the 5-4-3-2-1 grounding steps,
//! the tree's buttons, and koi fishing (fish_cast, fish_question, fish_roll, fish_ask, fish_answer, fish_show_catch,
//! z22_after_buttons) with its settings (depth, question source, subject menu, golden bait).

use super::*;
use serde_json::{Value, json};
use zenko_desktop::fishart::{fish_all, fish_by_id, gdata};
use zenko_desktop::gardenpage::{GAct, GardenData, GardenPage, breath_new};

fn cm10(x: &HubW) -> i32 {
    (100.0 * x.dpi / 25.4 + 0.5) as i32
}

fn s(v: &Value) -> String {
    v.as_str().unwrap_or("").to_string()
}

/// what the page shows, read from the app now (ground_i is the page's own)
fn data(x: &HubW) -> GardenData {
    let (mincho, serif, jpg) = hubbook::fonts_pub(&x.hub.font_ui);
    let ground_i = x.garden.as_ref().map(|p| p.d.ground_i).unwrap_or(-1);
    crate::win::with_fox(|fox, t| {
        let life = zenko_desktop::read_json("zenko_life.json");
        let secs = zenko_desktop::unix_now().floor() as i64 + zenko_desktop::local_offset_pub();
        let today = secs.div_euclid(86400);
        let iso = zenko_desktop::plan::iso(today);
        let (hours, got) = zenko_desktop::rewards::journey_state(&life);
        let tails = fox.tail_count(t);
        let c = &fox.cfg;
        GardenData {
            garden: life["garden"].clone(),
            fishing: life["fishing"].clone(),
            cards: life["cards"].as_array().cloned().unwrap_or_default(),
            cfg: c.raw.clone(),
            lockin: fox.lockin.is_some(),
            today,
            hour: (secs.rem_euclid(86400) / 3600) as u32,
            minute: (secs.rem_euclid(3600) / 60) as u32,
            month: iso[5..7].parse().unwrap_or(1),
            sky: c.b("sky_daynight", true),
            seasons: c.b("sky_seasons", true),
            weather: fox.slow.wx.code.map(|_| fox.wx_kind().to_string()),
            wx_code: fox.slow.wx.code,
            hours,
            got,
            pal: fox.pal.clone(),
            look: fox.look_ref().clone(),
            tails,
            hd: c.b("hd_art", true),
            jp: mincho.clone(),
            jpg: jpg.clone(),
            serif: serif.clone(),
            ground_i,
        }
    }).unwrap_or_else(|| GardenData { jp: mincho, jpg, serif, ground_i, ..Default::default() })
}

pub(super) fn build(x: &mut HubW, pg: usize) {
    let (th, f) = (x.hub.t.clone(), faces(x));
    let (d, c) = (data(x), cm10(x));
    x.garden = Some(GardenPage::build(&mut x.hub.ui, pg, &th, &f, c, &d));
}

pub(super) fn fit(x: &mut HubW, w: i32, h: i32) {
    let (th, f) = (x.hub.t.clone(), faces(x));
    let Some(p) = x.garden.as_mut() else { return };
    p.fit(&mut x.hub.ui, &x.fonts, &th, &f, x.hub.zk_f, w, h);
    let sc = p.sc.clone();
    sc.draw_bar(&mut x.hub.ui, false, &th);
}

/// garden_panel() with what the app has now
fn panel(x: &mut HubW) {
    let d = data(x);
    let Some(p) = x.garden.as_mut() else { return };
    p.panel(&mut x.hub.ui, &d);
    let (w, h) = x.size;
    fit(x, w, h);
}

/// garden_refresh(rebuild): the painting (made again when asked) and the panel
fn refresh(x: &mut HubW, rebuild: bool) {
    if rebuild && let Some(p) = x.garden.as_mut() {
        p.forget();
    }
    panel(x);
}

/// hub_after_show("garden")
pub(super) fn shown(x: &mut HubW) {
    refresh(x, false);
}

fn on_page(x: &HubW) -> bool {
    x.hub.side.page.as_deref() == Some("garden") && x.garden.is_some()
}

fn say(t: String, secs: f64) {
    crate::win::with_fox(move |fox, _| fox.say(&t, secs));
}

fn ui_sound(k: &str) {
    let k = k.to_string();
    crate::win::with_fox(move |fox, _| fox.events.push(zenko_desktop::Ev::Ui(k)));
}

fn beep(k: &str) {
    let k = k.to_string();
    crate::win::with_fox(move |fox, _| fox.events.push(zenko_desktop::Ev::Beep { kind: k, variety: false }));
}

fn rnd() -> f64 {
    crate::win::with_fox(|fox, _| fox.rand()).unwrap_or(0.5)
}

fn life_do(f: impl FnOnce(&mut Value)) {
    crate::win::with_fox(move |fox, _| fox.life_do(f));
}

fn cfg_set(k: &'static str, v: Value) {
    crate::win::with_fox(move |fox, _| zenko_desktop::update_config(&mut fox.cfg, |c| c[k] = v.clone()));
}

fn today_iso() -> String {
    let secs = zenko_desktop::unix_now().floor() as i64 + zenko_desktop::local_offset_pub();
    zenko_desktop::plan::iso(secs.div_euclid(86400))
}

// ---- breathing ----------------------------------------------------------------------------------------------------

/// breath_play(kind, secs, vol): the tone file made once (in a thread) and played
fn breath_play(kind: &str, secs: f64) {
    let vol = crate::win::with_fox(|fox, _| zenko_desktop::sound::SoundCfg::from(&fox.cfg).vol).unwrap_or(2);
    let (kind, dir) = (kind.to_string(), zenko_desktop::sound::sound_dir());
    std::thread::spawn(move || {
        let p = dir.join(zenko_sound::breath_file_name(&kind, secs, vol));
        if !p.exists() {
            std::fs::create_dir_all(&dir).ok();
            let part = p.with_extension("wav.part");
            if std::fs::write(&part, zenko_sound::breath_tone(&kind, secs, vol)).is_err() || std::fs::rename(&part, &p).is_err() {
                return;
            }
        }
        play_wav(&p);
    });
}

fn play_wav(p: &std::path::Path) {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Media::Audio::{PlaySoundW, SND_ASYNC, SND_FILENAME, SND_NODEFAULT};
    let w: Vec<u16> = p.as_os_str().encode_wide().chain(std::iter::once(0)).collect();
    unsafe { PlaySoundW(w.as_ptr(), std::ptr::null_mut(), SND_FILENAME | SND_ASYNC | SND_NODEFAULT) };
}

/// breath_hush(): whatever cue is playing stops
fn breath_hush() {
    use windows_sys::Win32::Media::Audio::PlaySoundW;
    unsafe { PlaySoundW(std::ptr::null(), std::ptr::null_mut(), 0) };
}

/// breath_start(key, minutes)
pub(super) fn breath_start(x: &mut HubW, key: Option<&str>, minutes: Option<f64>) {
    let pats = &gdata()["BREATH_PATTERNS"];
    let d = data(x);
    let mut k = key.filter(|k| pats.get(*k).is_some()).map(String::from)
        .unwrap_or_else(|| d.cfg["breath_kind"].as_str().unwrap_or("box").to_string());
    if pats.get(&k).is_none() {
        k = "box".into();
    }
    cfg_set("breath_kind", json!(k));
    let cm = d.cfg["breath_minutes"].as_f64().unwrap_or(3.0);
    let b = breath_new(&k, minutes, cm, now(x));
    if x.hub.side.page.as_deref() != Some("garden") {
        show(x, "garden");
    }
    if let Some(p) = x.garden.as_mut() {
        p.breath = Some(b);
    }
    panel(x);
}

/// breath_stop(quiet)
fn breath_stop(x: &mut HubW, quiet: bool) {
    let had = x.garden.as_ref().is_some_and(|p| p.breath.is_some());
    breath_hush();
    if let Some(p) = x.garden.as_mut() {
        p.breath_stop(&mut x.hub.ui);
    }
    if had && !quiet {
        panel(x);
    }
}

/// breath_finish(): the calm minutes logged, the tree watered a little, a kind word and "Well done~"
fn breath_finish(x: &mut HubW, key: &str, mins: f64) {
    breath_stop(x, true);
    let (d, t) = (today_iso(), local_hm());
    life_do(move |l| {
        let g = &mut l["garden"];
        if !g["calm"].is_array() {
            g["calm"] = json!([]);
        }
        let k = key_owned(key);
        let c = g["calm"].as_array_mut().unwrap();
        c.push(json!({"d": d, "t": t, "k": k, "m": mins}));
        let n = c.len();
        if n > 200 {
            c.drain(..n - 200);
        }
        g["health"] = json!(1.0f64.min(zenko_desktop::rewards::garden_health(g) + 0.1));
    });
    let lines = gdata()["BREATH_PATTERNS"][key]["done"].as_array().cloned().unwrap_or_default();
    if !lines.is_empty() {
        let i = ((rnd() * lines.len() as f64) as usize).min(lines.len() - 1);
        say(s(&lines[i]), 9.0);
    }
    let n = now(x);
    if let Some(p) = x.garden.as_mut() {
        p.well_done(&mut x.hub.ui, n);
    }
    panel(x);
}

fn key_owned(k: &str) -> String {
    k.to_string()
}

fn local_hm() -> String {
    let secs = zenko_desktop::unix_now().floor() as i64 + zenko_desktop::local_offset_pub();
    format!("{:02}:{:02}", secs.rem_euclid(86400) / 3600, secs.rem_euclid(3600) / 60)
}

// ---- the loop -----------------------------------------------------------------------------------------------------

/// garden_anim every 50 ms while the page shows (the breathing session ends when you leave), the foxfire's 230 ms
/// flicker, "Well done~" fading, a caught fish's card after 900 ms, and a cast that waited for the picture
pub(super) fn step(x: &mut HubW, n: f64) -> bool {
    if x.garden.is_none() {
        return false;
    }
    if !on_page(x) || x.slide.is_some() {
        if !on_page(x) {
            if let Some(p) = x.garden.as_mut() {
                p.anim_on = false;
            }
            if x.garden.as_ref().is_some_and(|p| p.breath.is_some()) {
                breath_stop(x, true);
            }
        }
        return false;
    }
    let mut changed = false;
    if let Some(c) = x.gd_pending.take() {
        if x.garden.as_ref().is_some_and(|p| p.lay.is_some()) {
            cmd(x, &c);
            changed = true;
        } else {
            x.gd_pending = Some(c);
        }
    }
    if let Some((at, fid, seed, cm, new)) = x.gd_catch.clone()
        && n >= at {
        x.gd_catch = None;
        show_catch(x, &fid, seed, cm, new);
        changed = true;
    }
    if x.gd_badge_at > 0.0 && n >= x.gd_badge_at {
        x.gd_badge_at = 0.0;
        crate::win::with_fox(|fox, t| fox.z22_badge_check(t));
    }
    let Some(p) = x.garden.as_mut() else { return changed };
    changed |= p.flicker(&mut x.hub.ui, n);
    changed |= p.done_clear(&mut x.hub.ui, n);
    if !p.anim_on || n < x.gd_next {
        return changed;
    }
    x.gd_next = n + 0.05;
    let mut r = || rnd();
    let acts = p.frame(&mut x.hub.ui, n, &mut r);
    for a in acts {
        match a {
            GAct::BreathCue(ph, secs) => breath_play(&ph, secs),
            GAct::BreathDone(k, m) => breath_finish(x, &k, m),
            GAct::FishAsk => fish_ask(x),
            GAct::Sound(k) => ui_sound(k),
        }
    }
    true
}

/// a click on a breathing pattern's row
pub(super) fn click(x: &mut HubW, mx: i32, my: i32) -> bool {
    if !on_page(x) {
        return false;
    }
    let Some(c) = x.garden.as_ref().and_then(|p| p.click_at(&x.hub.ui, mx, my)) else { return false };
    cmd(x, &c);
    true
}

/// the fishing card's keys: 1-4 answer and Esc lets it go; on a result card Esc and Return close it
pub(super) fn key(x: &mut HubW, vk: u16) -> bool {
    if !on_page(x) {
        return false;
    }
    let kind = x.garden.as_ref().map(|p| if p.fcard.is_some() { p.fcard_kind } else { 0 }).unwrap_or(0);
    if kind == 1 {
        let n = x.garden.as_ref().and_then(|p| p.fish.as_ref()).and_then(|f| f.q["opts"].as_array().map(|a| a.len())).unwrap_or(0);
        if (b'1' as u16..=b'4' as u16).contains(&vk) && ((vk - b'0' as u16) as usize) <= n {
            cmd(x, &format!("gd_ans:{}", vk - b'1' as u16));
            return true;
        }
        if vk == 0x1b {
            fish_answer(x, None);
            return true;
        }
    } else if kind == 2 && (vk == 0x1b || vk == 0x0d) {
        cmd(x, "gd_done");
        return true;
    }
    false
}

/// a command from the app (chat or the menu): breath:k, grounding, fish_cast, koi_book. It waits for the picture.
pub fn app_cmd(x: &mut HubW, c: &str) {
    if x.hub.side.page.as_deref() != Some("garden") {
        show(x, "garden");
    }
    if c == "grounding" {
        if let Some(p) = x.garden.as_mut() {
            p.d.ground_i = 0;
        }
        panel(x);
        return;
    }
    let c = match c {
        "fish_cast" => "gd_cast".to_string(),
        "koi_book" => "gd_koibook".to_string(),
        _ => c.replace("breath:", "gd_breath_start:"),
    };
    x.gd_pending = Some(c);
}

pub(super) fn cmd(x: &mut HubW, cmd: &str) {
    let (a, arg) = cmd.split_once(':').unwrap_or((cmd, ""));
    match a {
        "gd_breath_pick" => {
            if x.garden.as_ref().is_some_and(|p| p.breath.is_some()) {
                return;
            }
            cfg_set("breath_kind", json!(arg));
            ui_sound("toggle");
            panel(x);
        }
        "gd_bmin" => cfg_set("breath_minutes", json!(arg.parse::<i64>().unwrap_or(3))),
        "gd_bsound" => {
            let on = !data(x).cfg["breath_sound"].as_bool().unwrap_or(true);
            cfg_set("breath_sound", json!(on));
            if !on {
                breath_hush();
            }
        }
        "gd_breath_start" => breath_start(x, Some(arg), None),
        "gd_breath_stop" => breath_stop(x, false),
        "gd_ground" => {
            let n = gdata()["GROUNDING"].as_array().map(|a| a.len()).unwrap_or(5) as i64;
            let j = x.garden.as_ref().map(|p| p.d.ground_i).unwrap_or(-1) + 1;
            if let Some(p) = x.garden.as_mut() {
                p.d.ground_i = if j < n { j } else { -1 };
            }
            if j >= n {
                say("You're here. You're okay. Kon~".into(), 6.0);
            }
            panel(x);
        }
        "gd_lockin" => {
            crate::win::with_fox(|fox, t| {
                fox.lockin_start(t, None, false);
            });
            x.gd_panel_at = now(x) + 0.4;
        }
        "gd_plant" => plant_new(x),
        "gd_depth" => {
            let d: i64 = arg.parse().unwrap_or(1);
            life_do(move |l| l["fishing"]["depth"] = json!(d));
            panel(x);
        }
        "gd_src" => {
            let v = arg.to_string();
            life_do(move |l| l["fishing"]["src"] = json!(v));
            panel(x);
        }
        "gd_subj" => {
            let v = arg.to_string();
            life_do(move |l| l["fishing"]["subj"] = json!(v));
            panel(x);
        }
        "gd_subj_menu" => x.gd_menu = true,
        "gd_gold" => {
            let on = !data(x).cfg["fish_gold"].as_bool().unwrap_or(false);
            cfg_set("fish_gold", json!(on));
        }
        "gd_cast" => fish_cast(x),
        "gd_ans" => {
            let i: usize = arg.parse().unwrap_or(0);
            let o = x.garden.as_ref().and_then(|p| p.fish.as_ref()).and_then(|f| f.q["opts"].get(i).map(s));
            if let Some(o) = o {
                fish_answer(x, Some(o));
            }
        }
        "gd_again" => {
            fish_clear(x);
            fish_cast(x);
        }
        "gd_book2" => {
            fish_clear(x);
            koi_book(x);
        }
        "gd_koibook" => koi_book(x),
        "gd_done" => {
            fish_clear(x);
            refresh(x, false);
        }
        _ => {}
    }
}

/// garden_plant_new(): a grown tree joins the grove on the hills and a new seed goes in
fn plant_new(x: &mut HubW) {
    let d = data(x);
    let (i, _, _) = d.stage();
    let last = zenko_desktop::rewards::rdata()["GARDEN_STAGES"].as_array().map(|a| a.len()).unwrap_or(1) - 1;
    if i < last {
        return;
    }
    let today = today_iso();
    let seed = 1 + (rnd() * 9998.0) as i64;
    life_do(move |l| {
        let g = &mut l["garden"];
        let e = json!({"planted": g["planted"].clone(), "done": today.clone(), "xp": g["xp"].clone()});
        if !g["grove"].is_array() {
            g["grove"] = json!([]);
        }
        g["grove"].as_array_mut().unwrap().push(e);
        g["xp"] = json!(0.0);
        g["seed"] = json!(seed);
        g["planted"] = json!(today);
    });
    say("Your sacred tree joins the grove on the hills~ and a new seed goes in the ground. Kon!".into(), 8.0);
    refresh(x, true);
}

/// the subject menu of the fishing card (at the pointer), after the click
pub(super) unsafe fn subj_popup(hwnd: HWND) {
    let subj = with(|x| data(x).fishing["subj"].as_str().unwrap_or("any").to_string()).unwrap_or_default();
    let items: Vec<(String, String)> = gdata()["FISH_Q_SUBJECTS"].as_array().cloned().unwrap_or_default().iter()
        .map(|q| (s(&q[0]), s(&q[1]))).collect();
    let r = unsafe {
        let mut p = POINT { x: 0, y: 0 };
        GetCursorPos(&mut p);
        let pm = CreatePopupMenu();
        for (i, (k, lab)) in items.iter().enumerate() {
            let t = format!("{}{}", if *k == subj { "\u{25cf} " } else { "   " }, lab);
            let w: Vec<u16> = t.encode_utf16().chain(std::iter::once(0)).collect();
            AppendMenuW(pm, MF_STRING, i + 1, w.as_ptr());
        }
        let r = TrackPopupMenu(pm, TPM_RETURNCMD | TPM_LEFTALIGN | TPM_TOPALIGN, p.x, p.y, 0, hwnd, std::ptr::null());
        DestroyMenu(pm);
        r
    };
    with(|x| {
        if r > 0
            && let Some((k, _)) = items.get(r as usize - 1) {
            cmd(x, &format!("gd_subj:{k}"));
        }
        relayout(x);
    });
}

/// the Lock in button's panel refresh 400 ms later
pub(super) fn tick(x: &mut HubW, n: f64) {
    if x.gd_panel_at > 0.0 && n >= x.gd_panel_at {
        x.gd_panel_at = 0.0;
        panel(x);
    }
}

// ---- fishing ------------------------------------------------------------------------------------------------------

/// random.choice
fn pick<T: Clone>(v: &[T]) -> Option<T> {
    if v.is_empty() {
        return None;
    }
    Some(v[((rnd() * v.len() as f64) as usize).min(v.len() - 1)].clone())
}

/// random.shuffle
fn shuffle<T>(v: &mut [T]) {
    for i in (1..v.len()).rev() {
        let j = ((rnd() * (i + 1) as f64) as usize).min(i);
        v.swap(i, j);
    }
}

/// fish_question(depth, src, subj): one of your flashcards as multiple choice, or the built-in bank
fn fish_question(d: &GardenData, depth: i64, src: &str, subj: &str) -> Value {
    let cards: Vec<Value> = d.cards.iter().filter(|c| !s(&c["q"]).is_empty() && !s(&c["a"]).is_empty()).cloned().collect();
    let use_cards = src == "cards" || (src == "mix" && cards.len() >= 4 && rnd() < 0.5);
    if use_cards && cards.len() >= 4 {
        let due: Vec<Value> = crate::win::with_fox(|fox, _| fox.cards_due()).unwrap_or_default();
        let pool = if !due.is_empty() && rnd() < 0.7 { due } else { cards.clone() };
        if let Some(card) = pick(&pool) {
            let a = s(&card["a"]).trim().to_string();
            let mut others: Vec<String> = vec![];
            for c in &cards {
                let o = s(&c["a"]).trim().to_string();
                if o.to_lowercase() != a.to_lowercase() && !others.contains(&o) {
                    others.push(o);
                }
            }
            shuffle(&mut others);
            if others.len() >= 3 {
                let mut opts = vec![a.clone()];
                opts.extend(others.into_iter().take(3));
                shuffle(&mut opts);
                return json!({"q": card["q"], "a": a, "opts": opts, "card": card, "subj": card["subject"].as_str().unwrap_or(""),
                              "diff": depth});
            }
        }
    }
    let qs = gdata()["FISH_QS"].as_array().unwrap();
    let any = matches!(subj, "any" | "");
    let mut bank: Vec<&Value> = qs.iter().filter(|q| q[1].as_i64() == Some(depth) && (any || q[0] == subj)).collect();
    if bank.is_empty() {
        bank = qs.iter().filter(|q| any || q[0] == subj).collect();
        if bank.is_empty() {
            bank = qs.iter().collect();
        }
    }
    let recent: Vec<String> = d.fishing["recent_q"].as_array().map(|a| a.iter().map(s).collect()).unwrap_or_default();
    let fresh: Vec<&Value> = bank.iter().copied().filter(|q| !recent.contains(&s(&q[2]))).collect();
    let fresh = if fresh.is_empty() { bank } else { fresh };
    let q = pick(&fresh).unwrap().clone();
    let qt = s(&q[2]);
    life_do(move |l| {
        let f = &mut l["fishing"];
        if !f["recent_q"].is_array() {
            f["recent_q"] = json!([]);
        }
        let r = f["recent_q"].as_array_mut().unwrap();
        r.push(json!(qt));
        let n = r.len();
        if n > 40 {
            r.drain(..n - 40);
        }
    });
    let mut opts = vec![s(&q[3])];
    opts.extend(q[4].as_array().unwrap().iter().map(s));
    shuffle(&mut opts);
    json!({"q": q[2], "a": q[3], "opts": opts, "card": null, "subj": q[0], "diff": q[1]})
}

/// fish_roll(depth, gold, row): which fish bites (rarer with deeper water, a better rod, golden bait and a streak)
fn fish_roll(d: &GardenData, depth: i64, gold: bool, row: i64) -> (Value, f64) {
    let now = d.fish_now();
    let (rod, lan, _) = d.fish_gear();
    let mut w: Vec<i64> = match depth {
        1 => vec![70, 24, 5, 1, 0],
        2 => vec![42, 33, 17, 6, 2],
        _ => vec![22, 28, 26, 16, 8],
    };
    let shift = rod as i64 * 4 + 10.min(row * 2) + if gold { 12 } else { 0 };
    for _ in 0..shift {
        for k in 0..4 {
            if w[k] > 2 {
                w[k] -= 1;
                w[k + 1] += 1;
                break;
            }
        }
    }
    if gold {
        w[3] *= 2;
        w[4] *= 2;
    }
    // random.choices([1..5], weights=w)
    let tot: i64 = w.iter().sum();
    let u = rnd() * tot as f64;
    let mut acc = 0.0;
    let mut rar = 5;
    for (i, &wi) in w.iter().enumerate() {
        acc += wi as f64;
        if u < acc {
            rar = i as i64 + 1;
            break;
        }
    }
    while rar >= 1 {
        let pool: Vec<&Value> = fish_all().iter().filter(|f| f[3].as_i64() == Some(rar) && d.fish_can(f, &now, depth, rod, lan)).collect();
        if let Some(f) = pick(&pool) {
            let (lo, hi) = (f[7][0].as_f64().unwrap_or(20.0), f[7][1].as_f64().unwrap_or(40.0));
            // random.triangular(lo, hi, lo + (hi - lo) * 0.4)
            let mode = lo + (hi - lo) * 0.4;
            let mut u = rnd();
            let mut c = if hi == lo { 0.5 } else { (mode - lo) / (hi - lo) };
            let (mut a, mut b) = (lo, hi);
            if u > c {
                u = 1.0 - u;
                c = 1.0 - c;
                (a, b) = (b, a);
            }
            let cm = a + (b - a) * (u * c).sqrt();
            return (f.clone(), (cm * 10.0).round_ties_even() / 10.0);
        }
        rar -= 1;
    }
    (fish_by_id("kohaku").cloned().unwrap_or(Value::Null), 30.0)
}

/// (left, can): casts left for z22_after_buttons
fn can_cast(d: &GardenData) -> bool {
    let f = &d.fishing;
    let left = if s(&f["free_day"]) == today_iso() { 3 - f["free_used"].as_i64().unwrap_or(0) } else { 3 };
    left > 0 || f["bait"].as_f64().unwrap_or(0.0) >= 1.0
        || (d.cfg["fish_gold"].as_bool().unwrap_or(false) && f["gold"].as_i64().unwrap_or(0) != 0)
}

/// fish_cast()
fn fish_cast(x: &mut HubW) {
    if !on_page(x) || x.garden.as_ref().is_some_and(|p| p.lay.is_none()) {
        app_cmd(x, "fish_cast");
        return;
    }
    if x.garden.as_ref().and_then(|p| p.fish.as_ref()).is_some_and(|f| f.phase != "done") {
        return;
    }
    let d = data(x);
    let f = &d.fishing;
    let today = today_iso();
    let used = if s(&f["free_day"]) == today { f["free_used"].as_i64().unwrap_or(0) } else { 0 };
    let free = used < 3;
    let gold = d.cfg["fish_gold"].as_bool().unwrap_or(false) && f["gold"].as_i64().unwrap_or(0) > 0;
    if !free && f["bait"].as_f64().unwrap_or(0.0) < 1.0 {
        say("Out of bait~ Every 10 minutes of focus earns one, and you get 3 free casts each day.".into(), 7.0);
        return;
    }
    let t2 = today.clone();
    life_do(move |l| {
        let f = &mut l["fishing"];
        if f["free_day"].as_str() != Some(t2.as_str()) {
            f["free_day"] = json!(t2);
            f["free_used"] = json!(0);
        }
        if gold {
            f["gold"] = json!(f["gold"].as_i64().unwrap_or(0) - 1);
        } else if free {
            f["free_used"] = json!(f["free_used"].as_i64().unwrap_or(0) + 1);
        } else {
            f["bait"] = json!(f["bait"].as_f64().unwrap_or(0.0) - 1.0);
        }
    });
    let depth = f["depth"].as_i64().filter(|v| *v != 0).unwrap_or(1);
    let (a, rr) = (rnd() * 6.28, 0.1 + rnd() * 0.45);
    let q = fish_question(&d, depth, f["src"].as_str().unwrap_or("mix"), f["subj"].as_str().unwrap_or("any"));
    let n = now(x);
    if let Some(p) = x.garden.as_mut() {
        p.fish = Some(zenko_desktop::gardenpage::Fish {
            phase: "cast".into(), t: n, bx: a.cos() * rr, by: a.sin() * rr * 0.8, depth, gold, q, bite_at: 1e9, items: None,
            ripples: vec![], jump: None,
        });
    }
    ui_sound("tap");
    panel(x);
}

fn fish_clear(x: &mut HubW) {
    x.gd_catch = None;
    if let Some(p) = x.garden.as_mut() {
        p.fish_clear(&mut x.hub.ui);
    }
}

/// fish_ask(): the question card
fn fish_ask(x: &mut HubW) {
    let Some(p) = x.garden.as_mut() else { return };
    let Some(f) = p.fish.as_mut().filter(|f| f.phase == "bite") else { return };
    f.phase = "asking".into();
    let q = f.q.clone();
    let subj = s(&q["subj"]);
    let base = zenko_desktop::watch::data().subjects.iter().any(|d| d.sid == subj);
    let life = zenko_desktop::read_json("zenko_life.json");
    let mut sub = if base {
        zenko_desktop::notes::subject_name(&subj, &life, false)
    } else {
        gdata()["FISH_Q_SUBJECTS"].as_array().unwrap().iter().find(|e| e[0] == subj.as_str()).map(|e| s(&e[1])).unwrap_or_default()
    };
    if sub.is_empty() && !q["card"].is_null() {
        sub = "Your flashcards".into();
    }
    p.fish_ask_card(&mut x.hub.ui, &q, &sub);
    let (w, h) = x.size;
    fit(x, w, h);
}

/// fish_answer(choice): a miss lets it go (and saves the question as a flashcard); a right answer reels one in
fn fish_answer(x: &mut HubW, choice: Option<String>) {
    let n = now(x);
    let Some(fish) = x.garden.as_ref().and_then(|p| p.fish.clone()).filter(|f| f.phase == "asking") else { return };
    let q = fish.q.clone();
    let ok = choice.as_deref().is_some_and(|c| c.trim().to_lowercase() == s(&q["a"]).trim().to_lowercase());
    life_do(|l| l["fishing"]["asked"] = json!(l["fishing"]["asked"].as_i64().unwrap_or(0) + 1));
    if !q["card"].is_null() {
        let card = q["card"].clone();
        crate::win::with_fox(move |fox, t| fox.srs_update(t, &card, ok, ""));
    }
    if !ok {
        life_do(|l| l["fishing"]["row"] = json!(0));
        if let Some(f) = x.garden.as_mut().and_then(|p| p.fish.as_mut()) {
            f.phase = "lost".into();
            f.t = n;
        }
        beep("pet");
        let saved = !q["card"].is_null() || {
            // z22_save_miss(q)
            let (qq, aa) = (s(&q["q"]), s(&q["a"]));
            crate::win::with_fox(move |fox, _| {
                let have = zenko_desktop::read_json("zenko_life.json")["cards"].as_array()
                    .is_some_and(|c| c.iter().any(|c| c["q"].as_str() == Some(qq.as_str())));
                if !have {
                    fox.card_add(&qq, &aa);
                }
            }).is_some()
        };
        let can = can_cast(&data(x));
        if let Some(p) = x.garden.as_mut() {
            p.fish_lost_card(&mut x.hub.ui, &q, saved, can);
        }
        panel(x);
        return;
    }
    let mut row = 0;
    life_do(|l| {
        let f = &mut l["fishing"];
        f["right"] = json!(f["right"].as_i64().unwrap_or(0) + 1);
        f["row"] = json!(f["row"].as_i64().unwrap_or(0) + 1);
    });
    let d = data(x);
    row = d.fishing["row"].as_i64().unwrap_or(row);
    let award = |id: &'static str| {
        crate::win::with_fox(move |fox, t| fox.award(t, id));
    };
    if row >= 10 {
        award("hotq");
    }
    let (f_, cm) = fish_roll(&d, fish.depth, fish.gold, row);
    let now_ = d.fish_now();
    let seed = 1 + (rnd() * 999999.0) as i64;
    let fid = s(&f_[0]);
    let today = today_iso();
    let new = d.fishing["book"][&fid]["n"].as_i64().unwrap_or(0) == 0;
    let w = if now_.rain { "rain" } else if now_.snow { "snow" } else { "" };
    let (fid2, t2) = (fid.clone(), today.clone());
    life_do(move |l| {
        let f = &mut l["fishing"];
        if !f["book"].is_object() {
            f["book"] = json!({});
        }
        let b = f["book"].as_object_mut().unwrap().entry(fid2.clone())
            .or_insert_with(|| json!({"n": 0, "best": 0.0, "first": t2.clone(), "seed": seed}));
        b["n"] = json!(b["n"].as_i64().unwrap_or(0) + 1);
        if cm > b["best"].as_f64().unwrap_or(0.0) {
            b["best"] = json!(cm);
            b["seed"] = json!(seed);
        }
        let mut log = f["log"].as_array().cloned().unwrap_or_default();
        log.push(json!({"f": fid2, "cm": cm, "d": t2, "seed": seed, "w": w}));
        let k = log.len().saturating_sub(60);
        f["log"] = Value::Array(log.split_off(k));
        let mut pond: Vec<Value> = f["pond"].as_array().cloned().unwrap_or_default().into_iter()
            .filter(|p| p[0].as_str() != Some(fid2.as_str())).collect();
        pond.insert(0, json!([fid2, seed]));
        pond.truncate(6);
        f["pond"] = Value::Array(pond);
    });
    let dx = if rnd() < 0.5 { -40.0 } else { 40.0 };
    if let Some(p) = x.garden.as_mut() {
        if let Some(f) = p.fish.as_mut() {
            f.phase = "caught".into();
            f.t = n;
        }
        p.fish_jump(&mut x.hub.ui, &fid, seed, n, dx);
    }
    let rar = f_[3].as_i64().unwrap_or(1);
    beep(if rar >= 3 { "gift" } else { "happy" });
    award("koi1");
    if fid == "ogon" {
        award("golden");
    }
    if fid == "ryu" {
        award("dragon");
    }
    if rar == 5 {
        award("s_legend");
    }
    if cm >= 80.0 && f_[4].as_str() != Some("oar") {
        award("bigfish");
    }
    if now_.rain {
        award("rainfish");
    }
    if now_.snow {
        award("snowfish");
    }
    let log = zenko_desktop::read_json("zenko_life.json")["fishing"]["log"].as_array().cloned().unwrap_or_default();
    let mut rars: Vec<i64> = log.iter().filter(|e| e["d"].as_str() == Some(today.as_str()))
        .filter_map(|e| fish_by_id(e["f"].as_str().unwrap_or("")).and_then(|f| f[3].as_i64())).collect();
    rars.sort();
    rars.dedup();
    if rars.len() >= 4 {
        award("s_rainbow");
    }
    x.gd_catch = Some((n + 0.9, fid, seed, cm, new));
    x.gd_badge_at = n + 1.2;
}

/// fish_show_catch(fid, seed, cm, new)
fn show_catch(x: &mut HubW, fid: &str, seed: i64, cm: f64, new: bool) {
    if x.garden.as_ref().is_none_or(|p| p.fish.is_none()) {
        return;
    }
    let can = can_cast(&data(x));
    if let Some(p) = x.garden.as_mut() {
        p.fish_catch_card(&mut x.hub.ui, fid, seed, cm, new, can);
    }
    panel(x);
    if let Some(p) = x.garden.as_mut() {
        p.forget(); // (ui["key"] = None: the new fish joins the pond at the next repaint)
    }
}

/// koi_book(): its own window
fn koi_book(x: &mut HubW) {
    let d = data(x);
    crate::koiwin::open(&d, &x.hub.t, &x.hub.font_ui, x.dpi);
}
