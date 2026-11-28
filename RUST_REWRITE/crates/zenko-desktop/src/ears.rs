//! zenko.py's ears on Windows: MicRecorder (winmm waveIn, eight 100 ms buffers, the voice detector), LocalVoice
//! (whisper.cpp's CLI on the recording), the older VoiceEngine helpers (faster-whisper in Python, or Windows speech
//! through PowerShell), push-to-talk keys (hotkey_loop every 40 ms, a 0.35 s hold), the chat's Mic and Talk buttons,
//! the voice conversation (conv_start / conv_listen / conv_tick / conv_stop), and Settings > Talk's Set up / Use
//! files I downloaded. "Look at my screen" by voice needs the vision brain (stage 5b): it answers as typed text.

use crate::voicehost::{NO_WINDOW, bg, post};
use std::cell::RefCell;
use std::io::{BufRead, Write};
use std::os::windows::process::CommandExt;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use zenko_desktop::voice::{self, mono};
use zenko_desktop::{Companion, Ev};

/// what the helpers report: ("text" | "status" | "error" | "auto_end", value)
static Q: Mutex<Vec<(String, String)>> = Mutex::new(Vec::new());

fn put(k: &str, v: &str) {
    Q.lock().unwrap().push((k.into(), v.into()));
}

// ---- the microphone ----------------------------------------------------------------------------------------------

struct Rec {
    stop: Arc<AtomicBool>,
    chunks: Arc<Mutex<Vec<u8>>>,
    err: Arc<Mutex<Option<String>>>,
    heard: Arc<AtomicBool>,
    th: Option<std::thread::JoinHandle<()>>,
    auto_stop: bool,
    id: u64,
}

impl Rec {
    fn start(auto_stop: bool, id: u64) -> Rec {
        let (stop, chunks, err, heard) = (Arc::new(AtomicBool::new(false)), Arc::new(Mutex::new(vec![])),
                                          Arc::new(Mutex::new(None)), Arc::new(AtomicBool::new(false)));
        let (s2, c2, e2, h2) = (stop.clone(), chunks.clone(), err.clone(), heard.clone());
        let th = std::thread::spawn(move || {
            let mut vad = voice::Vad::default();
            let r = unsafe { run_win(&s2, &c2, &mut vad, auto_stop, 40.0) };
            h2.store(vad.heard, Ordering::SeqCst);
            let auto = match r {
                Ok(a) => a,
                Err(e) => {
                    *e2.lock().unwrap() = Some(e);
                    true
                }
            };
            if auto {
                put("auto_end", &id.to_string());
            }
        });
        Rec { stop, chunks, err, heard, th: Some(th), auto_stop, id }
    }

    /// stop(): the recorded PCM
    fn stop(&mut self) -> Vec<u8> {
        self.stop.store(true, Ordering::SeqCst);
        if let Some(t) = self.th.take() {
            let t0 = mono();
            while !t.is_finished() && mono() - t0 < 3.0 {
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
        }
        std::mem::take(&mut *self.chunks.lock().unwrap())
    }
}

/// MicRecorder._run_win: 16 kHz mono 16-bit; true when the voice detector ended it
unsafe fn run_win(stop: &AtomicBool, chunks: &Mutex<Vec<u8>>, vad: &mut voice::Vad, auto_stop: bool, max_secs: f64)
                  -> Result<bool, String> {
    use windows_sys::Win32::Media::Audio::*;
    const RATE: u32 = 16000;
    let fmt = WAVEFORMATEX { wFormatTag: 1, nChannels: 1, nSamplesPerSec: RATE, nAvgBytesPerSec: RATE * 2, nBlockAlign: 2,
                             wBitsPerSample: 16, cbSize: 0 };
    let mut h: HWAVEIN = std::ptr::null_mut();
    let rc = unsafe { waveInOpen(&mut h, WAVE_MAPPER, &fmt, 0, 0, 0) };
    if rc != 0 {
        return Err(format!("I can't open a microphone (Windows said {rc}). Check one is plugged in, and that Settings > \
                            Privacy > Microphone lets desktop apps use it."));
    }
    let (n_buf, size) = (8usize, (RATE * 2 / 10) as usize);
    let mut bufs: Vec<Vec<u8>> = (0..n_buf).map(|_| vec![0u8; size]).collect();
    let mut hdrs: Vec<Box<WAVEHDR>> = vec![];
    let hs = std::mem::size_of::<WAVEHDR>() as u32;
    for b in bufs.iter_mut() {
        let mut hd: Box<WAVEHDR> = Box::new(unsafe { std::mem::zeroed() });
        hd.lpData = b.as_mut_ptr();
        hd.dwBufferLength = size as u32;
        unsafe {
            waveInPrepareHeader(h, &mut *hd, hs);
            waveInAddBuffer(h, &mut *hd, hs);
        }
        hdrs.push(hd);
    }
    let mut auto = false;
    unsafe { waveInStart(h) };
    let mut i = 0;
    while !stop.load(Ordering::SeqCst) {
        let hd = &mut hdrs[i];
        std::sync::atomic::fence(Ordering::SeqCst);
        let flags = unsafe { std::ptr::addr_of!(hd.dwFlags).read_unaligned() };
        if flags & WHDR_DONE != 0 {
            let n = unsafe { std::ptr::addr_of!(hd.dwBytesRecorded).read_unaligned() } as usize;
            let data = bufs[i][..n.min(size)].to_vec();
            chunks.lock().unwrap().extend_from_slice(&data);
            hd.dwBytesRecorded = 0;
            hd.dwFlags &= !WHDR_DONE; // cleared ourselves before handing it back
            unsafe { waveInAddBuffer(h, &mut **hd, hs) };
            i = (i + 1) % n_buf;
            if vad.feed(&data, auto_stop, max_secs) {
                auto = true;
                break;
            }
        } else {
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
    }
    unsafe {
        waveInStop(h);
        waveInReset(h);
        for hd in hdrs.iter_mut() {
            waveInUnprepareHeader(h, &mut **hd, hs);
        }
        waveInClose(h);
    }
    Ok(auto)
}

/// stt_transcribe(wav, model_key, extra_dir)
fn transcribe(wav: &std::path::Path, key: &str, extra: &str) -> Result<String, String> {
    let (Some(cli), Some(model)) = voice::stt_paths(key, extra) else {
        return Err("My offline ears aren't set up yet (Settings > Talk with Zenko).".into());
    };
    let threads = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(2).saturating_sub(1).clamp(1, 4);
    let (args, txt) = voice::stt_args(&cli, &model, wav, threads);
    let _ = std::fs::remove_file(&txt);
    let r = std::process::Command::new(&cli).args(&args[1..]).current_dir(cli.parent().unwrap_or(&cli))
        .creation_flags(NO_WINDOW).output().map_err(|e| format!("The speech engine stopped with an error: {e}"))?;
    let mut out = String::from_utf8_lossy(&r.stdout).into_owned();
    if let Ok(t) = std::fs::read(&txt) {
        let t = String::from_utf8_lossy(&t).into_owned();
        if !t.trim().is_empty() {
            out = t;
        }
    }
    let text = voice::stt_clean(&out);
    if !r.status.success() && text.is_empty() {
        let e = String::from_utf8_lossy(&r.stderr).trim().lines().last().unwrap_or("unknown error").chars().take(160).collect::<String>();
        return Err(format!("The speech engine stopped with an error: {e}"));
    }
    Ok(text)
}

// ---- the helper processes (VoiceEngine) --------------------------------------------------------------------------

/// system_python(): a real python.exe on PATH (not the Store's stub)
pub fn system_python() -> Option<std::path::PathBuf> {
    let path = std::env::var_os("PATH")?;
    for name in ["python", "py", "python3"] {
        for d in std::env::split_paths(&path) {
            let p = d.join(format!("{name}.exe"));
            if p.is_file() && !p.to_string_lossy().to_lowercase().contains("windowsapps") {
                return Some(p);
            }
        }
    }
    None
}

static WHISPER_OK: Mutex<(f64, bool)> = Mutex::new((0.0, false));

/// whisper_ready(): faster-whisper importable (checked at most every 10 minutes)
fn whisper_ready() -> bool {
    let (t, ok) = *WHISPER_OK.lock().unwrap();
    if t != 0.0 && mono() - t < 600.0 {
        return ok;
    }
    let ok = system_python().map(|py| std::process::Command::new(py).args(["-c", "import faster_whisper, sounddevice, numpy"])
        .creation_flags(NO_WINDOW).stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null()).status()
        .map(|s| s.success()).unwrap_or(false)).unwrap_or(false);
    *WHISPER_OK.lock().unwrap() = (mono(), ok);
    ok
}

fn helper(name: &str, body: &str) -> std::path::PathBuf {
    let p = voice::ext_dir("helpers").join(name);
    let _ = std::fs::write(&p, body);
    p
}

// ---- the ears' state (the UI thread) -----------------------------------------------------------------------------

struct Conv {
    misses: i32,
    await_: f64,
    spoke: bool,
}

#[derive(Default)]
struct Ears {
    kind: Option<&'static str>,
    on: bool,
    ready: bool,
    listening: bool,
    busy: Arc<AtomicBool>,
    rec: Option<Rec>,
    rec_seq: u64,
    auto_stop: bool,
    child: Option<std::process::Child>,
    stdin: Option<std::process::ChildStdin>,
    pending: bool,
    mode: String,
    conv: Option<Conv>,
    keys: std::collections::HashMap<String, f64>,
    key_on: std::collections::HashMap<String, bool>,
    key_early: std::collections::HashMap<String, bool>,
    stt_busy: bool,
    speak_end: f64,
    next_hot: f64,
    next_conv: f64,
    later: Vec<(f64, &'static str)>,
}

thread_local! {
    static E: RefCell<Ears> = RefCell::new(Ears::default());
}

fn with<R>(f: impl FnOnce(&mut Ears) -> R) -> R {
    E.with(|e| f(&mut e.borrow_mut()))
}

pub fn conv_on() -> bool {
    with(|e| e.conv.is_some())
}

/// speak() was asked to read a line: the conversation waits for her to finish
pub fn on_speak(words: usize) {
    with(|e| {
        if let Some(c) = e.conv.as_mut() {
            c.spoke = true;
        }
        e.speak_end = mono() + 0.8 + words as f64 / 2.6;
    });
}

pub fn speak_end(at: f64) {
    with(|e| e.speak_end = at);
}

fn alive(e: &mut Ears) -> bool {
    if e.kind == Some("local") {
        return e.on;
    }
    e.child.as_mut().map(|c| c.try_wait().map(|s| s.is_none()).unwrap_or(false)).unwrap_or(false)
}

fn send(e: &mut Ears, cmd: &str) -> bool {
    e.stdin.as_mut().map(|s| s.write_all(format!("{cmd}\n").as_bytes()).and_then(|_| s.flush()).is_ok()).unwrap_or(false)
}

/// LocalVoice.start(prefer, model): the offline ears first, else the Python / PowerShell helpers
fn start(e: &mut Ears, fox: &Companion) -> bool {
    if alive(e) {
        return true;
    }
    let prefer = fox.cfg.s("voice_engine", "auto");
    let (key, extra) = (fox.cfg.s("stt_model", "fast"), fox.cfg.s("stt_dir", ""));
    if (prefer == "auto" || prefer == "local") && voice::stt_ready(&key, &extra) {
        (e.kind, e.on, e.ready) = (Some("local"), true, true);
        put("status", "ready");
        return true;
    }
    if prefer == "local" {
        put("error", "My offline ears aren't set up yet. Settings > Talk with Zenko > Set up (one time).");
        return false;
    }
    let d = voice::data();
    let mut cmd;
    if (prefer == "auto" || prefer == "whisper") && whisper_ready() {
        let p = helper("whisper_worker.py", d["WHISPER_WORKER"].as_str().unwrap_or(""));
        cmd = std::process::Command::new(system_python().unwrap_or_default());
        cmd.arg(p).arg(fox.cfg.s("whisper_model", "small.en")).arg(d["MED_PROMPT"].as_str().unwrap_or(""));
        e.kind = Some("whisper");
    } else {
        let p = helper("winspeech.ps1", d["WINSPEECH_WORKER"].as_str().unwrap_or(""));
        cmd = std::process::Command::new("powershell");
        cmd.args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-File"]).arg(p);
        e.kind = Some("windows");
    }
    let r = cmd.stdin(std::process::Stdio::piped()).stdout(std::process::Stdio::piped()).stderr(std::process::Stdio::null())
        .creation_flags(NO_WINDOW).spawn();
    let mut c = match r {
        Ok(c) => c,
        Err(err) => {
            put("error", &format!("Couldn't start voice ({err})."));
            return false;
        }
    };
    e.ready = false;
    e.stdin = c.stdin.take();
    let out = c.stdout.take();
    e.child = Some(c);
    std::thread::spawn(move || {
        if let Some(o) = out {
            for line in std::io::BufReader::new(o).lines() {
                let Ok(line) = line else { break };
                let Ok(d) = serde_json::from_str::<serde_json::Value>(line.trim()) else { continue };
                if d["ready"].as_bool() == Some(true) {
                    put("_ready", "");
                    put("status", "ready");
                } else if d["listening"].as_bool() == Some(true) {
                    put("status", "listening");
                } else if d["cancelled"].as_bool() == Some(true) {
                    put("_idle", "");
                    put("status", "cancelled");
                } else if d.get("text").is_some() {
                    put("_idle", "");
                    put("text", d["text"].as_str().unwrap_or(""));
                } else if let Some(err) = d["error"].as_str() {
                    put("error", err);
                }
            }
        }
        put("_idle", "");
        put("status", "stopped");
    });
    true
}

fn press(e: &mut Ears) -> bool {
    if e.kind != Some("local") {
        if !alive(e) || e.listening {
            return false;
        }
        e.listening = true;
        return send(e, "start");
    }
    if !e.on || e.listening || e.busy.load(Ordering::SeqCst) {
        return false;
    }
    e.listening = true;
    e.rec_seq += 1;
    e.rec = Some(Rec::start(e.auto_stop, e.rec_seq));
    e.auto_stop = false; // one-shot: push-to-talk keys never auto-stop
    put("status", "listening");
    true
}

fn release(e: &mut Ears, fox: &Companion) {
    if e.kind != Some("local") {
        if e.kind == Some("whisper") && e.listening {
            send(e, "stop");
        }
        return;
    }
    if e.listening && e.rec.is_some() {
        finish(e, fox);
    }
}

/// LocalVoice._finish: the recording checked and sent to whisper.cpp in a thread
fn finish(e: &mut Ears, fox: &Companion) {
    let Some(mut rec) = e.rec.take() else { return };
    let pcm = rec.stop();
    e.listening = false;
    if let Some(err) = rec.err.lock().unwrap().clone() {
        put("error", &err);
        return;
    }
    if pcm.len() >= 16000 && voice::mic_peak(&pcm) < 24 {
        put("error", "I only heard total silence. Is the microphone muted? Check Windows Settings > Privacy & security > \
                      Microphone > 'Let desktop apps access your microphone', and the input device in Settings > System > Sound.");
        return;
    }
    if (pcm.len() as f64) < 16000.0 * 2.0 * 0.4 || (rec.auto_stop && !rec.heard.load(Ordering::SeqCst)) {
        put("text", "");
        return;
    }
    e.busy.store(true, Ordering::SeqCst);
    put("status", "transcribing");
    let busy = e.busy.clone();
    let (key, extra) = (fox.cfg.s("stt_model", "fast"), fox.cfg.s("stt_dir", ""));
    std::thread::spawn(move || {
        let wav = voice::ext_dir("speech_tmp").join("said.wav");
        let r = voice::pcm_to_wav(&pcm, &wav, 16000).map_err(|e| e.to_string()).and_then(|_| transcribe(&wav, &key, &extra));
        match r {
            Ok(t) => put("text", &t),
            Err(err) => put("error", &err),
        }
        busy.store(false, Ordering::SeqCst);
    });
}

fn cancel(e: &mut Ears) {
    if e.kind != Some("local") {
        if e.kind == Some("whisper") && e.listening {
            send(e, "cancel");
        }
        return;
    }
    if let Some(mut r) = e.rec.take() {
        r.stop();
    }
    if e.listening {
        e.listening = false;
        put("status", "cancelled");
    }
}

fn stop(e: &mut Ears) {
    if e.kind == Some("local") {
        cancel(e);
        e.on = false;
        return;
    }
    if alive(e) {
        send(e, "quit");
        if let Some(c) = e.child.as_mut() {
            let _ = c.kill();
        }
    }
    e.child = None;
    e.stdin = None;
}

/// the Settings page changed the ears (stt_model / stt_dir / voice_engine): voice.stop()
pub fn restart() {
    with(stop);
}

// ---- the app side ------------------------------------------------------------------------------------------------

/// voice_ui(on): the bubble, her mood and the chat's Mic / Talk buttons
fn voice_ui(e: &Ears, fox: &mut Companion, t: f64, on: bool) {
    if on {
        fox.set_mood(t, "focus", 30.0);
        if e.kind == Some("local") {
            fox.say(&format!("Listening...{}", if e.auto_stop_ui() { " (I'll stop when you pause)" } else {
                " (let go of the key when you're done)" }), 30.0);
        } else {
            fox.say(&format!("Listening...{}", if e.kind == Some("whisper") { " (release the key when done)" } else { "" }), 30.0);
        }
    } else {
        fox.mood_override = None;
    }
    crate::hubw::chat_voice_look(on, e.conv.is_some());
}

impl Ears {
    /// the recorder's auto_stop as voice_ui sees it (zenko17's voice_ui reads self.voice.auto_stop after press, which
    /// press() has already reset: so it always says "let go of the key")
    fn auto_stop_ui(&self) -> bool {
        self.auto_stop
    }
}

/// voice_press(mode)
pub fn voice_press(fox: &mut Companion, t: f64, mode: &str) {
    crate::voicehost::vv_hush();
    with(|e| {
        if e.kind == Some("local") || voice::stt_ready(&fox.cfg.s("stt_model", "fast"), &fox.cfg.s("stt_dir", "")) {
            e.auto_stop = mode == "button" || mode == "conv";
        }
        if e.listening {
            return;
        }
        e.mode = mode.into();
        if !alive(e) || !e.ready {
            if !alive(e) && !start(e, fox) {
                return;
            }
            if e.kind == Some("local") {
                e.auto_stop = mode == "button" || mode == "conv";
            }
            e.pending = true;
            fox.say("Waking up my ears... (first time takes a few seconds)", 6.0);
            return;
        }
        press(e);
        voice_ui(e, fox, t, true);
    });
}

fn voice_release(fox: &mut Companion) {
    with(|e| {
        if e.pending && e.mode != "button" {
            e.pending = false;
            fox.say("My ears are ready now. Hold the key and talk.", 6.0);
            return;
        }
        release(e, fox);
    });
}

/// the chat's Mic button: click to talk, click again to stop
pub fn voice_toggle(fox: &mut Companion, t: f64) {
    let listening = with(|e| e.listening);
    if listening {
        with(|e| release(e, fox));
    } else {
        voice_press(fox, t, "button");
    }
}

/// Settings' Test my voice: voice_toggle("chat")
pub fn voice_toggle_chat(fox: &mut Companion, t: f64) {
    let listening = with(|e| e.listening);
    if listening {
        with(|e| release(e, fox));
    } else {
        voice_press(fox, t, "chat");
    }
}

/// conv_start(): Talk (again: stop)
pub fn conv_start(fox: &mut Companion, t: f64) {
    if conv_on() {
        return conv_stop(fox, t, Some("Okay, I'll stop listening~"), false);
    }
    let eng = fox.cfg.s("voice_engine", "auto");
    let whisper = with(|e| e.kind == Some("whisper"));
    if !voice::stt_ready(&fox.cfg.s("stt_model", "fast"), &fox.cfg.s("stt_dir", "")) && !((eng == "auto" || eng == "whisper") && whisper) {
        fox.events.push(Ev::Say { text: "To talk out loud I need my offline ears first: a one-time download of about 60 MB. Set it up now?".into(),
                                  secs: 16.0, choices: vec![("Set it up".into(), "stt_setup".into()), ("Later".into(), String::new())] });
        return;
    }
    with(|e| {
        e.conv = Some(Conv { misses: 0, await_: 0.0, spoke: false });
        e.speak_end = 0.0;
        voice_ui(e, fox, t, false);
        e.later.push((mono() + 0.9, "listen"));
        e.next_conv = mono() + 0.3;
    });
    fox.voice_conv = true;
    let m = *fox.brain.rng.choice(&["I'm listening~ Just talk to me. Say 'bye Zenko' when you're done.",
                                    "Kon! Ears up. Talk whenever you're ready~"]);
    fox.say(m, 6.0);
}

fn conv_listen(fox: &mut Companion, t: f64) {
    let go = with(|e| e.conv.is_some() && !e.listening && !e.busy.load(Ordering::SeqCst));
    if go {
        voice_press(fox, t, "conv");
    }
}

pub fn conv_stop(fox: &mut Companion, t: f64, msg: Option<&str>, speak: bool) {
    with(|e| {
        e.conv = None;
        if e.listening {
            cancel(e);
        }
        voice_ui(e, fox, t, false);
    });
    fox.voice_conv = false;
    if let Some(m) = msg {
        if speak {
            fox.voice_reply_until = mono() + 5.0;
            fox.fox_reply(t, m);
        } else {
            fox.say(m, 6.0);
        }
    }
}

/// on_voice_event(kind, val): zenko17's then zenko.py's
fn on_event(fox: &mut Companion, t: f64, kind: &str, val: &str) {
    if kind == "status" && val == "stopped" && with(|e| e.pending) {
        // the helper died before it was ready (e.g. PowerShell is blocked on a school PC): say so
        with(|e| {
            e.pending = false;
            voice_ui(e, fox, t, false);
        });
        fox.events.push(Ev::Say { text: "My ears didn't start: this PC blocked the speech helper. Set up my offline ears instead? \
                                         (one-time download, about 60 MB, no PowerShell needed)".into(), secs: 20.0,
                                  choices: vec![("Set them up".into(), "stt_setup".into()), ("Later".into(), String::new())] });
        return;
    }
    if kind == "status" && val == "transcribing" {
        fox.say("Mm-hm... *ears twitch*", 8.0);
        return;
    }
    let conv = conv_on();
    if conv && kind == "error" {
        with(|e| e.conv = None);
        fox.voice_conv = false;
    }
    if conv && kind == "text" {
        let tx = val.trim().to_string();
        if tx.is_empty() {
            let misses = with(|e| {
                voice_ui(e, fox, t, false);
                let c = e.conv.as_mut().unwrap();
                c.misses += 1;
                c.misses
            });
            if misses >= 2 {
                conv_stop(fox, t, Some("I'll stop listening for now. Press Talk when you want me again~"), false);
            } else {
                with(|e| e.later.push((mono() + 0.5, "listen")));
            }
            return;
        }
        with(|e| e.conv.as_mut().unwrap().misses = 0);
        let lw = tx.to_lowercase();
        let lw = lw.trim_matches(|c| " .!?".contains(c));
        let bye = regex::Regex::new(r"^(?:ok(?:ay)?,? )?(?:bye|goodbye|good night|goodnight|see you|stop listening|that'?s all|thanks?,? that'?s (?:it|all)|stop talking|end (?:the )?(?:chat|conversation))\b").unwrap();
        if bye.is_match(lw) {
            with(|e| voice_ui(e, fox, t, false));
            fox.chat_line(t, "You", &tx);
            let m = *fox.brain.rng.choice(&["Bye bye~ Talk soon, kon!", "Okay~ I'll be right here.", "Mata ne~ Call me anytime."]);
            conv_stop(fox, t, Some(m), true);
            return;
        }
        with(|e| {
            let c = e.conv.as_mut().unwrap();
            c.await_ = mono();
            c.spoke = false;
        });
    }
    // zenko.py's on_voice_event
    if kind == "status" {
        if val == "ready" && with(|e| e.pending) {
            with(|e| {
                e.pending = false;
                press(e);
                voice_ui(e, fox, t, true);
            });
        } else if val == "stopped" {
            with(|e| voice_ui(e, fox, t, false));
        }
        return;
    }
    if kind == "error" {
        with(|e| voice_ui(e, fox, t, false));
        fox.fox_reply(t, val);
        return;
    }
    if kind == "text" {
        with(|e| voice_ui(e, fox, t, false));
        let tx = val.trim();
        if tx.is_empty() {
            fox.say("I didn't catch that. Try again?", 5.0);
            return;
        }
        // (mode "screen" asks the vision brain: stage 5b; until then it's a normal question)
        fox.voice_reply_until = mono() + 120.0;
        crate::hubw::chat_send_voice(tx);
    }
}

fn key_down(vk: i32) -> bool {
    unsafe { windows_sys::Win32::UI::Input::KeyboardAndMouse::GetAsyncKeyState(vk) as u16 & 0x8000 != 0 }
}

/// every frame: the helpers' news, push-to-talk keys (40 ms), the conversation (300 ms), the little waits
pub fn tick(fox: &mut Companion, t: f64) {
    let evs: Vec<(String, String)> = std::mem::take(&mut *Q.lock().unwrap());
    for (k, v) in evs {
        match k.as_str() {
            "_ready" => with(|e| e.ready = true),
            "_idle" => with(|e| e.listening = false),
            "auto_end" => {
                let id: u64 = v.parse().unwrap_or(0);
                with(|e| {
                    if e.rec.as_ref().is_some_and(|r| r.id == id) {
                        finish(e, fox);
                    }
                });
            }
            _ => on_event(fox, t, &k, &v),
        }
    }
    let n = mono();
    let due: Vec<&'static str> = with(|e| {
        let (now_, later_): (Vec<_>, Vec<_>) = e.later.drain(..).partition(|(at, _)| *at <= n);
        e.later = later_;
        now_.into_iter().map(|x| x.1).collect()
    });
    for d in due {
        if d == "listen" {
            conv_listen(fox, t);
        }
    }
    // conv_tick
    if conv_on() && n >= with(|e| e.next_conv) {
        with(|e| e.next_conv = n + 0.3);
        let go = with(|e| {
            let (listening, busy, se) = (e.listening, e.busy.load(Ordering::SeqCst), e.speak_end);
            let c = e.conv.as_mut().unwrap();
            if c.await_ != 0.0 && !listening && !busy && ((c.spoke && n > se + 0.5) || n - c.await_ > 45.0) {
                c.await_ = 0.0;
                return true;
            }
            false
        });
        if go {
            conv_listen(fox, t);
        }
    }
    // hotkey_loop
    if n < with(|e| e.next_hot) {
        return;
    }
    with(|e| e.next_hot = n + 0.04);
    if !fox.cfg.b("voice_enabled", true) {
        return;
    }
    let hk = &voice::data()["HOTKEYS"];
    for (mode, key) in [("chat", fox.cfg.s("voice_key", "Right Ctrl")), ("screen", fox.cfg.s("screen_key", "F8"))] {
        let Some(vk) = hk[key.as_str()].as_i64().filter(|v| *v != 0) else { continue };
        let down = key_down(vk as i32);
        let t0 = with(|e| e.keys.get(mode).copied());
        let held = with(|e| e.key_on.get(mode).copied().unwrap_or(false));
        let early = with(|e| e.key_early.get(mode).copied().unwrap_or(false));
        if down && t0.is_none() {
            with(|e| {
                e.keys.insert(mode.into(), n);
                if matches!(e.kind, Some("whisper") | Some("local")) && alive(e) && e.ready && !e.listening {
                    e.mode = mode.into();
                    press(e); // record from the very first moment
                    e.key_early.insert(mode.into(), true);
                }
            });
        } else if down && t0.is_some_and(|t0| n - t0 >= 0.35) && !held {
            with(|e| e.key_on.insert(mode.into(), true)); // held: it's push-to-talk, not Ctrl+C
            if early {
                with(|e| {
                    e.mode = mode.into();
                    voice_ui(e, fox, t, true);
                });
            } else {
                voice_press(fox, t, mode);
            }
        } else if !down && t0.is_some() {
            if held {
                voice_release(fox);
            } else if early {
                with(cancel); // just a quick tap (e.g. Ctrl+C)
            }
            with(|e| {
                e.key_on.insert(mode.into(), false);
                e.keys.remove(mode);
                e.key_early.insert(mode.into(), false);
            });
        }
    }
}

// ---- Settings > Talk with Zenko ----------------------------------------------------------------------------------

/// what the Settings page's status lines should say now: (label key, text)
pub static SET_MSG: Mutex<Vec<(String, String)>> = Mutex::new(Vec::new());

fn set_msg(k: &str, t: &str) {
    SET_MSG.lock().unwrap().push((k.into(), t.into()));
}

pub fn set_msg_pub(k: &str, t: &str) {
    set_msg(k, t);
}

/// stt_setup(status): the one-time download of whisper.cpp and the model
pub fn stt_setup(fox: &mut Companion, to_card: bool) {
    if with(|e| std::mem::replace(&mut e.stt_busy, true)) {
        return;
    }
    let key = fox.cfg.s("stt_model", "fast");
    let say = move |fox: &mut Companion, m: &str| {
        if to_card {
            set_msg("stt", m);
        } else {
            fox.say(m, 12.0);
        }
    };
    say(fox, &format!("Downloading my offline ears ({})... you can keep working~", voice::stt_model(&key).2));
    let last = Arc::new(Mutex::new(0.0f64));
    bg(move || voice::stt_install(&key, &mut |label, f| {
        let mut l = last.lock().unwrap();
        if mono() - *l > 1.0 {
            *l = mono();
            let m = format!("Downloading {label}... {}%", (f * 100.0) as i64);
            post(move |fox, _| if to_card { set_msg("stt", &m) } else { fox.say(&m, 12.0) });
        }
    }), move |fox, _, r| {
        with(|e| e.stt_busy = false);
        match r {
            Err(err) => say(fox, &err),
            Ok(_) => {
                with(stop);
                say(fox, &format!("My ears are ready! Press Talk (or say nothing and just hold {}) and speak~",
                                  fox.cfg.s("voice_key", "Right Ctrl")));
            }
        }
        if to_card {
            let s = voice::stt_status(&fox.cfg.s("stt_model", "fast"), &fox.cfg.s("stt_dir", ""));
            set_msg("stt", &s);
        }
    });
}

/// pip install in the PC's Python (install_whisper / inst_piper): Ok, or the error line
fn pip(pkgs: &[&str]) -> Result<(), String> {
    let py = system_python().ok_or("I couldn't find Python on this PC. Install it from python.org first.")?;
    let r = std::process::Command::new(py).args(["-m", "pip", "install", "--upgrade"]).args(pkgs).creation_flags(NO_WINDOW)
        .output().map_err(|e| e.to_string())?;
    if !r.status.success() {
        let tail = String::from_utf8_lossy(&r.stderr).trim().lines().last().unwrap_or("unknown error").chars().take(200).collect::<String>();
        return Err(format!("pip failed: {tail}"));
    }
    Ok(())
}

/// Settings > Voice: check_engine()
pub fn check_engine() {
    set_msg("whisper", "Checking...");
    bg(whisper_ready, |fox, _, ok| {
        let msg = if ok {
            let mut m = "Whisper is installed. ".to_string();
            let (alive_, kind) = with(|e| (alive(e), e.kind));
            if alive_ && kind.is_some() {
                m += &format!("In use now: {}.", if kind == Some("whisper") { format!("Whisper ({})", fox.cfg.s("whisper_model", "small.en")) }
                                                  else { "Windows built-in (switch the engine above to Whisper)".into() });
            } else {
                m += "It starts the next time you talk.";
            }
            m
        } else {
            "Whisper isn't installed for the Python I can see, so Windows' built-in speech is used (less accurate). Press \
             Install Whisper.".into()
        };
        set_msg("whisper", &msg);
    });
}

pub fn install_whisper() {
    set_msg("whisper", "Installing Whisper... this can take a few minutes. You can keep working.");
    bg(|| pip(&["faster-whisper", "sounddevice", "numpy"]), |_, _, r| {
        *WHISPER_OK.lock().unwrap() = (0.0, false);
        set_msg("whisper", &match r { Ok(_) => "Whisper installed! First use downloads its model.".into(), Err(e) => e });
        with(stop);
    });
}

pub fn install_piper() {
    if system_python().is_none() {
        set_msg("voice_status", "I couldn't find Python on this PC.");
        return;
    }
    set_msg("voice_status", "Installing natural voices (free, offline)... a few minutes.");
    bg(|| pip(&["piper-tts"]), |_, _, r| {
        set_msg("voice_status", &match r { Ok(_) => "Installed! Now pick a 'Natural' voice above.".into(), Err(e) => e });
    });
}
