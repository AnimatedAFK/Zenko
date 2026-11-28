//! zenko.py's voice on Windows: the VOICEVOX engine opened hidden and closed again (vv_autostart,
//! voicevox_start_engine / voicevox_stop_engine), the 30 s "is it up?" check (z17_vv_check), her lines said in
//! Japanese with the bubble waiting for the voice (Zenko.say / _jp_wait / _jp_show / _jp_flush), vv_speak / jp_speak
//! / vv_hush, speak() with the Windows voice as the fallback, beeps and UI sounds kept quiet while she talks, the
//! timetable alerts in her voice (remind_aloud), and the Customize page's Find VOICEVOX / Browse for run.exe / Hear her.

use std::os::windows::process::CommandExt;
use std::path::PathBuf;
use std::sync::Mutex;
use windows_sys::Win32::Media::Audio::{PlaySoundW, SND_ASYNC, SND_FILENAME, SND_NODEFAULT};
use zenko_desktop::voice::{self, Ready, mono, state};
use zenko_desktop::{Cfg, Companion};

pub const NO_WINDOW: u32 = 0x0800_0000;

type Job = Box<dyn FnOnce(&mut Companion, f64) + Send>;
static DONE: Mutex<Vec<Job>> = Mutex::new(Vec::new());

/// self.bg(job, done): job in a thread, done back on the UI thread (drained by tick)
pub fn bg<R: Send + 'static>(job: impl FnOnce() -> R + Send + 'static, done: impl FnOnce(&mut Companion, f64, R) + Send + 'static) {
    std::thread::spawn(move || {
        let r = job();
        DONE.lock().unwrap().push(Box::new(move |fox, t| done(fox, t, r)));
    });
}

pub fn post(f: impl FnOnce(&mut Companion, f64) + Send + 'static) {
    DONE.lock().unwrap().push(Box::new(f));
}

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(Some(0)).collect()
}

/// a little random number for the threads (random.choice in jp_fallback)
fn rnd() -> usize {
    use std::hash::{BuildHasher, Hasher};
    let mut h = std::collections::hash_map::RandomState::new().build_hasher();
    h.write_u64(mono().to_bits());
    h.finish() as usize
}

fn pick(seq: &[&'static str]) -> &'static str {
    seq[rnd() % seq.len().max(1)]
}

fn speaker(c: &Cfg) -> i64 {
    let s = c.f("voicevox_speaker", 8.0) as i64;
    if s == 0 { 8 } else { s }
}

// ---- the engine --------------------------------------------------------------------------------------------------

pub type Waiter = Box<dyn FnOnce(&mut Companion, &str, &str, &str) + Send>;

struct Eng {
    busy: bool,
    child: Option<std::process::Child>,
    waiters: Vec<Waiter>,
}

static ENG: Mutex<Eng> = Mutex::new(Eng { busy: false, child: None, waiters: Vec::new() });

pub fn places() -> voice::Places {
    let env = |k: &str| std::env::var_os(k).map(PathBuf::from).unwrap_or_default();
    let app = std::env::current_exe().ok().and_then(|p| p.parent().map(|d| d.to_path_buf())).unwrap_or_default();
    voice::Places { home: env("USERPROFILE"), local_app_data: env("LOCALAPPDATA"), program_files: env("ProgramFiles"), app_dir: app }
}

/// voicevox_start_engine(path, wait=120): running | started | exited | timeout | notfound
fn start_engine(path: &str, wait: f64) -> &'static str {
    if voice::vv_alive(1.0) {
        return "running";
    }
    let p = std::path::Path::new(path);
    if path.is_empty() || !p.is_file() {
        return "notfound";
    }
    {
        let mut g = ENG.lock().unwrap();
        let dead = match g.child.as_mut() {
            None => true,
            Some(c) => c.try_wait().map(|s| s.is_some()).unwrap_or(true),
        };
        if dead {
            let port = voice::vv_url().rsplit(':').next().and_then(|s| s.trim_end_matches('/').parse::<u16>().ok()).unwrap_or(50021);
            let r = std::process::Command::new(p).args(["--host", "127.0.0.1", "--port", &port.to_string()])
                .current_dir(p.parent().unwrap_or(p)).stdin(std::process::Stdio::null()).stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null()).creation_flags(NO_WINDOW).spawn();
            match r {
                Ok(c) => g.child = Some(c),
                Err(_) => return "exited",
            }
        }
    }
    let t0 = mono();
    while mono() - t0 < wait {
        let exited = ENG.lock().unwrap().child.as_mut().map(|c| c.try_wait().map(|s| s.is_some()).unwrap_or(true)).unwrap_or(true);
        if exited {
            return if voice::vv_alive(2.0) { "running" } else { "exited" };
        }
        if voice::vv_alive(1.5) {
            return "started";
        }
        std::thread::sleep(std::time::Duration::from_secs(1));
    }
    "timeout"
}

/// voicevox_stop_engine(): closed again, but only if Zenko opened it
pub fn stop_engine() {
    let c = ENG.lock().unwrap().child.take();
    let Some(mut c) = c else { return };
    if c.try_wait().map(|s| s.is_some()).unwrap_or(true) {
        return;
    }
    let _ = std::process::Command::new("taskkill").args(["/PID", &c.id().to_string(), "/T", "/F"]).creation_flags(NO_WINDOW)
        .stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null()).status();
    if c.try_wait().map(|s| s.is_none()).unwrap_or(false) {
        let _ = c.kill();
    }
}

/// vv_autostart(force, on_done): the engine opened hidden when her voice is VOICEVOX (never freezes the UI)
pub fn autostart(fox: &Companion, force: bool, on_done: Option<Waiter>) {
    if !force && (fox.cfg.s("voice_clips", "synth") != "voicevox" || !fox.cfg.b("voicevox_autostart", true)) {
        return;
    }
    {
        let mut g = ENG.lock().unwrap();
        if let Some(w) = on_done {
            g.waiters.push(w);
        }
        if g.busy {
            return;
        }
        g.busy = true;
    }
    let saved = fox.cfg.s("voicevox_engine_path", "");
    let sv = saved.clone();
    bg(move || {
        if voice::vv_alive(1.0) {
            return ("running", sv);
        }
        let path = voice::find_engine(&sv, &places());
        if path.is_empty() {
            return ("notfound", String::new());
        }
        (start_engine(&path, 120.0), path)
    }, move |fox, t, (st, path)| {
        ENG.lock().unwrap().busy = false;
        if !path.is_empty() && path != saved {
            fox.cfg_set("voicevox_engine_path", serde_json::json!(path));
        }
        if (st == "running" || st == "started") && fox.cfg.s("voice_clips", "synth") == "voicevox" {
            let sid = speaker(&fox.cfg);
            std::thread::spawn(move || {
                let _ = voice::make_clips(sid); // makes any clips still missing
            });
        }
        let ws = std::mem::take(&mut ENG.lock().unwrap().waiters);
        for w in ws {
            w(fox, st, &path, "");
        }
        let _ = t;
    });
}

// ---- speaking ----------------------------------------------------------------------------------------------------

pub type Got = Box<dyn FnOnce(f64) + Send>;

/// vv_speak(text, speaker, done): every sentence made first, then done(seconds) and the whole line in one go
pub fn vv_speak(text: &str, sid: i64, done: Option<Got>) -> bool {
    let gen_now = {
        let mut s = state();
        s.vv_gen += 1;
        s.vv_gen
    };
    let parts = voice::vv_say_text(text);
    if parts.is_empty() {
        return false;
    }
    std::thread::spawn(move || match voice::vv_prepare(&parts, sid, gen_now) {
        Ready::Cancelled => {}
        Ready::Down => {
            if let Some(d) = done {
                d(-1.0);
            }
        }
        Ready::Play(p, secs) => {
            if let Some(d) = done {
                d(secs);
            }
            {
                let s = state();
                if s.vv_gen != gen_now || s.muted {
                    return;
                }
            }
            let w: Vec<u16> = p.to_string_lossy().encode_utf16().chain(Some(0)).collect();
            unsafe { PlaySoundW(w.as_ptr(), std::ptr::null_mut(), SND_FILENAME | SND_NODEFAULT) }; // blocking, in this thread
        }
    });
    true
}

pub fn vv_hush() {
    state().vv_gen += 1;
    unsafe { PlaySoundW(std::ptr::null(), std::ptr::null_mut(), 0) };
}

/// jp_speak(text, speaker, done): translated (in a thread), then vv_speak; done(-1) if VOICEVOX is down
pub fn jp_speak(text: &str, sid: i64, done: Option<Got>) -> bool {
    let gen_now = {
        let mut s = state();
        s.jp_gen += 1;
        s.said = mono();
        s.until = mono() + 4.0;
        s.jp_gen
    };
    let text = text.to_string();
    std::thread::spawn(move || {
        let jp = voice::jp_translate(&text, true, &mut pick);
        let stop = {
            let s = state();
            jp.is_empty() || s.jp_gen != gen_now || s.muted
        };
        if stop {
            if let Some(d) = done {
                d(-1.0);
            }
            return;
        }
        let flag = std::sync::Arc::new(Mutex::new(done));
        let f2 = flag.clone();
        let got: Got = Box::new(move |secs| {
            if secs >= 0.0 {
                state().until = mono() + secs + 0.4;
            }
            if let Some(d) = f2.lock().unwrap().take() {
                d(secs);
            }
        });
        if !vv_speak(&jp, sid, Some(got)) {
            if let Some(d) = flag.lock().unwrap().take() {
                d(-1.0);
            }
        }
    });
    true
}

fn sapi(c: &Cfg, text: &str) {
    crate::speak::say(text, &c.s("tts_voice", ""), c.f("tts_speed", 0.0) as i64, c.f("tts_pitch", 0.0) as i64);
}

/// speak(text): VOICEVOX (Japanese, or the English line itself) with the Windows voice as the fallback
pub fn speak(fox: &mut Companion, text: &str) {
    let conv = crate::ears::conv_on();
    crate::ears::on_speak(text.split_whitespace().count());
    let c = &fox.cfg;
    if (c.b("voice_muted", false) || c.s("speak_mode", "voice") == "off") && !conv {
        return;
    }
    if c.s("talk_voice", "english") == "voicevox" {
        let words = text.split_whitespace().count() as f64;
        crate::ears::speak_end(mono() + 3.0 + words / 2.4);
        let sid = {
            let s = c.f("talk_vv_speaker", c.f("voicevox_speaker", 8.0)) as i64;
            if s == 0 { 8 } else { s }
        };
        let t = text.to_string();
        let got: Got = Box::new(move |secs| {
            if secs < 0.0 {
                // VOICEVOX isn't running: the Windows voice
                post(move |fox, _| {
                    if fox.cfg.s("speak_mode", "voice") != "off" || crate::ears::conv_on() {
                        sapi(&fox.cfg, &t);
                    }
                });
            } else {
                let end = mono() + secs + 0.3;
                post(move |_, _| crate::ears::speak_end(end));
            }
        });
        let ok = if c.b("jp_voice", true) { jp_speak(text, sid, Some(got)) } else { vv_speak(text, sid, Some(got)) };
        if ok {
            return;
        }
    }
    sapi(c, text);
}

/// remind_aloud's VOICEVOX part: in Japanese, waiting up to 20 s for the voice; 'kon kon~' when it can't
pub fn remind_voice(fox: &Companion, t: &str) {
    let sid = speaker(&fox.cfg);
    let t = t.to_string();
    std::thread::spawn(move || {
        let jp = voice::jp_translate(&t, true, &mut pick);
        let mut ok = false;
        if !jp.is_empty() {
            let res = std::sync::Arc::new(Mutex::new(None::<f64>));
            let r2 = res.clone();
            state().said = mono();
            ok = vv_speak(&jp, sid, Some(Box::new(move |secs| {
                *r2.lock().unwrap() = Some(secs);
                if secs >= 0.0 {
                    state().until = mono() + secs + 0.4;
                }
            })));
            let t0 = mono();
            while ok && res.lock().unwrap().is_none() && mono() - t0 < 20.0 {
                std::thread::sleep(std::time::Duration::from_millis(100));
            }
            ok = ok && res.lock().unwrap().unwrap_or(-1.0) >= 0.0;
        }
        if !ok {
            state().said = 0.0;
            post(|fox, _| fox.events.push(zenko_desktop::Ev::Beep { kind: "hello".into(), variety: false }));
        }
    });
}

/// beep's sound: after 0.3 s, unless she started saying a line out loud
pub fn play_beep(p: PathBuf) {
    let t0 = mono();
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_secs_f64(0.3));
        if state().said > t0 - 0.8 {
            return;
        }
        let w: Vec<u16> = p.to_string_lossy().encode_utf16().chain(Some(0)).collect();
        unsafe { PlaySoundW(w.as_ptr(), std::ptr::null_mut(), SND_FILENAME | SND_ASYNC | SND_NODEFAULT) };
    });
}

/// ui_sound: quiet while she is talking (one sound channel)
pub fn ui_quiet() -> bool {
    mono() < state().until
}

// ---- the bubble waits for her voice ------------------------------------------------------------------------------

pub struct Bub {
    pub text: String,
    pub secs: f64,
    pub choices: Vec<(String, String)>,
}

struct Pending {
    b: Bub,
    ready: std::sync::Arc<Mutex<Option<f64>>>,
    t: f64,
}

thread_local! {
    static PEND: std::cell::RefCell<Option<Pending>> = const { std::cell::RefCell::new(None) };
    static NEXT_CHECK: std::cell::Cell<f64> = const { std::cell::Cell::new(0.0) };
}

fn jp_voice_on(c: &Cfg) -> bool {
    c.b("sound", true) && c.b("jp_voice", true) && !c.b("voice_muted", false) && c.s("voice_clips", "synth") == "voicevox"
        && state().alive
}

/// _jp_ok(text): the line to say out loud, or None
fn jp_ok(c: &Cfg, text: &str, reply_until: f64) -> Option<String> {
    if !jp_voice_on(c) || crate::ears::conv_on() {
        return None;
    }
    let t = text.trim();
    if t.chars().count() < 2 || ["Listening", "(thinking", "Mm-hm", "Reading", "~ ", "Downloading", "Looking", "Checking"]
        .iter().any(|p| t.starts_with(p)) {
        return None;
    }
    if reply_until > mono() || c.s("speak_mode", "voice") == "always" {
        return None; // her full reply is about to be read out anyway
    }
    Some(t.to_string())
}

fn show(p: Pending) -> Bub {
    let mut b = p.b;
    if let Some(sec) = *p.ready.lock().unwrap() {
        if sec > 0.0 {
            b.secs = b.secs.max(sec + 1.5);
        }
    }
    b
}

/// Zenko.say: the bubbles to show now (a waiting one first: _jp_flush). The new one may wait for its voice.
pub fn on_say(c: &Cfg, reply_until: f64, text: String, secs: f64, choices: Vec<(String, String)>) -> Vec<Bub> {
    let mut out = vec![];
    if let Some(p) = PEND.with(|q| q.borrow_mut().take()) {
        out.push(show(p));
    }
    if let Some(t) = jp_ok(c, &text, reply_until) {
        let ready = std::sync::Arc::new(Mutex::new(None));
        let r2 = ready.clone();
        if jp_speak(&t, speaker(c), Some(Box::new(move |sec| *r2.lock().unwrap() = Some(sec)))) {
            PEND.with(|q| *q.borrow_mut() = Some(Pending { b: Bub { text, secs, choices }, ready, t: mono() }));
            return out;
        }
    }
    out.push(Bub { text, secs, choices });
    out
}

/// _jp_wait (every frame): the waiting bubble once her voice is ready, or after 5 s at most
pub fn poll() -> Option<Bub> {
    let due = PEND.with(|q| q.borrow().as_ref().map(|p| p.ready.lock().unwrap().is_some() || mono() - p.t > 5.0).unwrap_or(false));
    if due { PEND.with(|q| q.borrow_mut().take()).map(show) } else { None }
}

/// voice_mute(on): her spoken lines stop (pet/boop clips keep playing)
pub fn set_muted(on: bool) {
    state().muted = on;
    if on {
        vv_hush();
        state().jp_gen += 1;
    }
}

/// every frame: the background jobs' answers, and z17_vv_check every 30 s (first at 2.5 s)
pub fn tick(fox: &mut Companion, t: f64) {
    let jobs: Vec<Job> = std::mem::take(&mut *DONE.lock().unwrap());
    for j in jobs {
        j(fox, t);
    }
    state().muted = fox.cfg.b("voice_muted", false);
    if NEXT_CHECK.with(|c| c.get()) == 0.0 {
        NEXT_CHECK.with(|c| c.set(t + 2.5));
    }
    if t >= NEXT_CHECK.with(|c| c.get()) {
        NEXT_CHECK.with(|c| c.set(t + 30.0));
        if fox.cfg.s("voice_clips", "synth") == "voicevox" && fox.cfg.b("jp_voice", true) {
            bg(|| voice::vv_alive(1.0), |_, _, ok| state().alive = ok);
        } else {
            state().alive = false;
        }
    }
}

// ---- Customize > Her voice ---------------------------------------------------------------------------------------

/// what the Customize page shows: (label key, text), and the speakers' menu when Find VOICEVOX found some
pub static CZ: Mutex<Vec<(String, String)>> = Mutex::new(Vec::new());
pub static CZ_MENU: Mutex<Option<(Vec<String>, String)>> = Mutex::new(None);

fn cz(k: &str, t: &str) {
    CZ.lock().unwrap().push((k.into(), t.into()));
}

fn path_show(fox: &Companion) {
    let p = fox.cfg.s("voicevox_engine_path", "");
    cz("engine", &if p.is_empty() {
        "Engine: not found yet (she looks in your voicevox_engine folder and the VOICEVOX app's folder)".to_string()
    } else {
        format!("Engine: {p}")
    });
}

/// voice_demo: one of her clips
pub fn voice_demo(fox: &mut Companion) {
    let k = ["hello", "pet", "happy", "giggle"][rnd() % 4];
    fox.events.push(zenko_desktop::Ev::Beep { kind: k.into(), variety: false });
}

/// vv_find: the engine opened if needed, then the voices it has
pub fn vv_find(fox: &mut Companion) {
    cz("vv_status", "Looking for VOICEVOX on this PC (opening the engine if needed, the first start can take a minute)...");
    autostart(fox, true, Some(Box::new(|fox, _, _, _| {
        path_show(fox);
        bg(|| voice::speakers(3.0), |fox, _, res| {
            let Some(res) = res.filter(|r| !r.is_empty()) else {
                cz("vv_status", "Couldn't reach VOICEVOX. Press 'Browse for run.exe' and pick run.exe inside your voicevox_engine \
                                 folder (e.g. voicevox_engine\\windows-cpu\\run.exe), or open the VOICEVOX app, then press Find \
                                 again. The built-in chirps work without it.");
                return;
            };
            let names: Vec<String> = res.iter().map(|(i, n)| format!("{i}: {n}")).collect();
            let cur = speaker(&fox.cfg);
            let sel = names.iter().find(|n| n.starts_with(&format!("{cur}:"))).cloned().unwrap_or_else(|| names[0].clone());
            *CZ_MENU.lock().unwrap() = Some((names, sel));
            cz("vv_status", &format!("Found {} VOICEVOX voices. Pick one (cute picks: 春日部つむぎ, 四国めたん あまあま, 冥鳴ひまり).", res.len()));
        });
    })));
}

/// a voice picked in the speakers' menu
pub fn vv_pick(fox: &mut Companion, val: &str) {
    let sid: i64 = val.split(':').next().and_then(|s| s.trim().parse().ok()).unwrap_or(8);
    fox.cfg_set("voicevox_speaker", serde_json::json!(sid));
    fox.cfg_set("voice_clips", serde_json::json!("voicevox"));
    fox.look_changed();
    let credit = val.split_once(": ").map(|x| x.1).unwrap_or("").split(" (").next().unwrap_or("").to_string();
    cz("vv_status", &format!("Making her clips with {val}... (credit: VOICEVOX:{credit})"));
    cz("radio:voice_clips", "voicevox");
    bg(move || voice::make_clips(sid), move |fox, _, r| match r {
        Ok(n) => {
            cz("vv_status", &format!("Ready! {n} clips made. Credit: VOICEVOX:{credit}"));
            voice_demo(fox);
        }
        Err(e) => cz("vv_status", &format!("VOICEVOX couldn't make the clips ({e}).")),
    });
}

/// vv_browse's filedialog.askopenfilename for run.exe (called outside the app's borrow: the dialog is modal)
pub fn vv_browse_pick(owner: windows_sys::Win32::Foundation::HWND) -> Option<PathBuf> {
    let home = std::env::var("USERPROFILE").unwrap_or_default();
    let start = PathBuf::from(&home).join("voicevox_engine");
    let init = if start.is_dir() { start } else { PathBuf::from(&home) };
    open_file(owner, "Pick VOICEVOX's run.exe", &init, &[("VOICEVOX engine", "run.exe"), ("Programs", "*.exe")])
}

/// vv_browse: the picked run.exe saved, then the engine opened
pub fn vv_browse(fox: &mut Companion, f: PathBuf) {
    fox.cfg_set("voicevox_engine_path", serde_json::json!(f.to_string_lossy()));
    path_show(fox);
    cz("vv_status", "Opening the VOICEVOX engine... (the first start can take a minute)");
    autostart(fox, true, Some(Box::new(|_, st, _, err| {
        let m = match st {
            "running" => "VOICEVOX is running. Press Find VOICEVOX to pick her voice.".to_string(),
            "started" => "VOICEVOX engine started! Press Find VOICEVOX to pick her voice.".into(),
            "exited" => "That run.exe closed straight away. Make sure the whole engine folder was extracted.".into(),
            "timeout" => "The engine is still loading. Give it a moment, then press Find VOICEVOX.".into(),
            "notfound" => "That file wasn't found.".into(),
            _ => format!("Couldn't open the engine ({}).", if err.is_empty() { st } else { err }),
        };
        cz("vv_status", &m);
    })));
}

/// filedialog.askopenfilename(title, initialdir, filetypes)
pub fn open_file(owner: windows_sys::Win32::Foundation::HWND, title: &str, init: &std::path::Path, types: &[(&str, &str)]) -> Option<PathBuf> {
    use windows_sys::Win32::UI::Controls::Dialogs::*;
    let mut filt: Vec<u16> = vec![];
    for (n, p) in types {
        filt.extend(format!("{n} ({p})").encode_utf16());
        filt.push(0);
        filt.extend(p.encode_utf16());
        filt.push(0);
    }
    filt.push(0);
    let (t, d) = (wide(title), wide(&init.to_string_lossy()));
    let mut buf = vec![0u16; 1024];
    let mut ofn: OPENFILENAMEW = unsafe { std::mem::zeroed() };
    ofn.lStructSize = std::mem::size_of::<OPENFILENAMEW>() as u32;
    ofn.hwndOwner = owner;
    ofn.lpstrFilter = filt.as_ptr();
    ofn.lpstrFile = buf.as_mut_ptr();
    ofn.nMaxFile = buf.len() as u32;
    ofn.lpstrTitle = t.as_ptr();
    ofn.lpstrInitialDir = d.as_ptr();
    ofn.Flags = OFN_FILEMUSTEXIST | OFN_PATHMUSTEXIST | OFN_NOCHANGEDIR | OFN_EXPLORER;
    if unsafe { GetOpenFileNameW(&mut ofn) } == 0 {
        return None;
    }
    let n = buf.iter().position(|&c| c == 0).unwrap_or(0);
    Some(PathBuf::from(String::from_utf16_lossy(&buf[..n])))
}
