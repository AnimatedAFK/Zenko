//! zenko.py Speaker + SapiVoice: Windows' built-in speech engine (SAPI ISpVoice through its raw vtable, like the
//! ctypes code), on a background thread. Used by remind_aloud (timetable alerts). `piper:` voices need zenko.py's
//! Python helper and aren't ported; they fall back to the default Windows voice.

use std::ffi::c_void;
use std::sync::Mutex;
use std::sync::mpsc::{Sender, channel};
use windows_sys::Win32::System::Com::{CLSCTX_INPROC_SERVER, CLSCTX_LOCAL_SERVER, CoCreateInstance, CoInitializeEx,
                                      CoTaskMemFree};
use windows_sys::core::GUID;

const CLSID_SPVOICE: GUID = GUID::from_u128(0x96749377_3391_11D2_9EE3_00C04F797396);
const IID_ISPVOICE: GUID = GUID::from_u128(0x6C44DF74_72B9_4992_A1EC_EF996E0422D4);
const CLSID_CAT: GUID = GUID::from_u128(0xA910187F_0C7A_45AC_92CC_59EDAFB77B53);
const IID_CAT: GUID = GUID::from_u128(0x2D3D3845_39AF_4850_BBF9_40B49780011D);
const VOICE_KEYS: [&str; 2] = [r"HKEY_LOCAL_MACHINE\SOFTWARE\Microsoft\Speech\Voices",
                               r"HKEY_LOCAL_MACHINE\SOFTWARE\Microsoft\Speech_OneCore\Voices"];

type P = *mut c_void;

unsafe fn vt(obj: P, i: usize) -> *const c_void {
    unsafe { *(*(obj as *const *const *const c_void)).add(i) }
}

unsafe fn release(obj: P) {
    if !obj.is_null() {
        let f: unsafe extern "system" fn(P) -> u32 = unsafe { std::mem::transmute(vt(obj, 2)) };
        unsafe { f(obj) };
    }
}

unsafe fn create(clsid: &GUID, iid: &GUID) -> Option<P> {
    let mut o: P = std::ptr::null_mut();
    let hr = unsafe { CoCreateInstance(clsid, std::ptr::null_mut(), CLSCTX_INPROC_SERVER | CLSCTX_LOCAL_SERVER, iid, &mut o) };
    (hr == 0 && !o.is_null()).then_some(o)
}

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(Some(0)).collect()
}

/// _sapi_tokens: [(short name, token)] for every voice, SAPI then OneCore; the caller releases them
unsafe fn tokens() -> Vec<(String, P)> {
    let mut out: Vec<(String, P)> = vec![];
    for key in VOICE_KEYS {
        let Some(cat) = (unsafe { create(&CLSID_CAT, &IID_CAT) }) else { continue };
        unsafe {
            let set_id: unsafe extern "system" fn(P, *const u16, i32) -> i32 = std::mem::transmute(vt(cat, 15));
            let k = wide(key);
            if set_id(cat, k.as_ptr(), 0) != 0 {
                release(cat);
                continue;
            }
            let enum_t: unsafe extern "system" fn(P, *const u16, *const u16, *mut P) -> i32 = std::mem::transmute(vt(cat, 18));
            let mut en: P = std::ptr::null_mut();
            if enum_t(cat, std::ptr::null(), std::ptr::null(), &mut en) != 0 || en.is_null() {
                release(cat);
                continue;
            }
            let next: unsafe extern "system" fn(P, u32, *mut P, *mut u32) -> i32 = std::mem::transmute(vt(en, 3));
            loop {
                let mut tok: P = std::ptr::null_mut();
                let mut got = 0u32;
                if next(en, 1, &mut tok, &mut got) != 0 || got == 0 {
                    break;
                }
                let gsv: unsafe extern "system" fn(P, *const u16, *mut *mut u16) -> i32 = std::mem::transmute(vt(tok, 6));
                let mut pn: *mut u16 = std::ptr::null_mut();
                let mut name = String::new();
                if gsv(tok, std::ptr::null(), &mut pn) == 0 && !pn.is_null() {
                    let mut n = 0;
                    while *pn.add(n) != 0 {
                        n += 1;
                    }
                    name = String::from_utf16_lossy(std::slice::from_raw_parts(pn, n));
                    CoTaskMemFree(pn as *const c_void);
                }
                let short = name.split(" - ").next().unwrap_or("").trim().to_string();
                if !short.is_empty() && !out.iter().any(|(s, _)| s.to_lowercase() == short.to_lowercase()) {
                    out.push((short, tok));
                } else {
                    release(tok);
                }
            }
            release(en);
            release(cat);
        }
    }
    out
}

fn run(name: String, rate: i32, rx: std::sync::mpsc::Receiver<String>) {
    unsafe {
        CoInitializeEx(std::ptr::null(), 0);
        let Some(v) = create(&CLSID_SPVOICE, &IID_ISPVOICE) else { return };
        if !name.is_empty() {
            let toks = tokens();
            let l = name.to_lowercase();
            let pick = toks.iter().find(|(n, _)| n.to_lowercase() == l).or_else(|| toks.iter().find(|(n, _)| n.to_lowercase().contains(&l)));
            if let Some((_, t)) = pick {
                let set_voice: unsafe extern "system" fn(P, P) -> i32 = std::mem::transmute(vt(v, 18));
                set_voice(v, *t);
            }
            for (_, t) in toks {
                release(t);
            }
        }
        let set_rate: unsafe extern "system" fn(P, i32) -> i32 = std::mem::transmute(vt(v, 28));
        set_rate(v, rate.clamp(-10, 10));
        let speak: unsafe extern "system" fn(P, *const u16, u32, *mut u32) -> i32 = std::mem::transmute(vt(v, 20));
        for item in rx {
            let w = wide(&item);
            let fl = 1 | 2 | if item.starts_with('<') { 8 } else { 0 }; // async + purge (+XML)
            speak(v, w.as_ptr(), fl, std::ptr::null_mut());
        }
        release(v);
    }
}

static SPK: Mutex<Option<((String, i32), Sender<String>)>> = Mutex::new(None);

/// Speaker.say(text, voice, speed, pitch)
pub fn say(text: &str, voice: &str, speed: i64, pitch: i64) {
    if text.is_empty() {
        return;
    }
    let speed = speed.clamp(-3, 3) as i32;
    let name = voice.strip_prefix("win:").unwrap_or("").to_string();
    let key = (voice.to_string(), speed);
    let mut g = SPK.lock().unwrap();
    if g.as_ref().is_none_or(|(k, _)| *k != key) {
        let (tx, rx) = channel();
        std::thread::spawn(move || run(name, speed * 2, rx));
        *g = Some((key, tx));
    }
    let url = regex::Regex::new(r"https?://\S+").unwrap();
    let nl = regex::Regex::new(r"[\r\n•]+").unwrap();
    let clean = url.replace_all(text, "");
    let clean: String = nl.replace_all(&clean, ". ").chars().take(600).collect();
    let line = if pitch != 0 {
        let esc = clean.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;");
        format!("<pitch absmiddle=\"{}\">{}</pitch>", pitch.clamp(0, 10), esc)
    } else {
        clean
    };
    let line = line.trim().to_string();
    if !line.is_empty() {
        if let Some((_, tx)) = g.as_ref() {
            let _ = tx.send(line);
        }
    }
}
