//! zenko.py BackgroundMusic (MCI, alias zenko_bgm), music_file / music_file_vol (the .wav files in the sounds folder,
//! byte-identical to zenko.py's, so either app reuses the other's), spotify_window and media_key.

use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Sender, channel};
use windows_sys::Win32::Foundation::{CloseHandle, HWND, LPARAM};
use windows_sys::Win32::Media::Multimedia::mciSendStringW;
use windows_sys::Win32::System::Threading::{OpenProcess, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION,
                                            QueryFullProcessImageNameW};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{KEYEVENTF_EXTENDEDKEY, KEYEVENTF_KEYUP, keybd_event};
use windows_sys::Win32::UI::WindowsAndMessaging::{EnumWindows, GetWindowTextLengthW, GetWindowTextW,
                                                  GetWindowThreadProcessId};
use zenko_desktop::music::Player;
use zenko_sound::music::{music_wav, music_wav_vol, track_or_garden};

const ALIAS: &str = "zenko_bgm";

enum Cmd {
    Play(String, i64),
    Stop,
    Keep,
}

/// set when Windows wouldn't play the loop (zenko.py then says so once per music_apply)
pub static FAILED: AtomicBool = AtomicBool::new(false);
static TX: Mutex<Option<Sender<Cmd>>> = Mutex::new(None);

fn mci(cmd: &str) -> (u32, String) {
    let w: Vec<u16> = cmd.encode_utf16().chain(Some(0)).collect();
    let mut buf = [0u16; 256];
    let err = unsafe { mciSendStringW(w.as_ptr(), buf.as_mut_ptr(), 255, std::ptr::null_mut()) };
    let n = buf.iter().position(|&c| c == 0).unwrap_or(0);
    (err, String::from_utf16_lossy(&buf[..n]))
}

/// music_file(track), then music_file_vol(track, vol): composed once and kept in the sounds folder
fn file_vol(track: &str, vol: i64) -> Option<std::path::PathBuf> {
    let dir = zenko_desktop::sound::sound_dir();
    std::fs::create_dir_all(&dir).ok()?;
    let tr = track_or_garden(track);
    let base = dir.join(format!("bgm_{tr}_v1.wav"));
    if !base.exists() {
        let part = dir.join(format!("bgm_{tr}_v1.wav.part"));
        std::fs::write(&part, music_wav(tr)).ok()?;
        std::fs::rename(&part, &base).ok()?;
    }
    let vol = vol.clamp(1, 10);
    let p = dir.join(format!("bgm_{tr}_v1_q{vol:02}.wav"));
    if p.exists() {
        return Some(p);
    }
    let b = std::fs::read(&base).ok()?;
    let part = dir.join(format!("bgm_{tr}_v1_q{vol:02}.wav.part"));
    std::fs::write(&part, music_wav_vol(&b, vol)).ok()?;
    std::fs::rename(&part, &p).ok()?;
    if let Ok(rd) = std::fs::read_dir(&dir) {
        for e in rd.flatten() {
            let f = e.file_name().to_string_lossy().to_string();
            if f.starts_with("bgm_") && f.contains("_q") && f.ends_with(".wav") && e.path() != p {
                let _ = std::fs::remove_file(e.path());
            }
        }
    }
    Some(p)
}

fn run(rx: std::sync::mpsc::Receiver<Cmd>) {
    let (mut open, mut cur): (bool, Option<(String, std::path::PathBuf)>) = (false, None);
    let mode = |open: bool| if open { let (e, s) = mci(&format!("status {ALIAS} mode")); if e != 0 { String::new() } else { s } } else { String::new() };
    for c in rx {
        match c {
            Cmd::Play(track, vol) => {
                let Some(p) = file_vol(&track, vol) else {
                    FAILED.store(true, Ordering::Relaxed);
                    continue;
                };
                if open && cur.as_ref().is_some_and(|(t, q)| *t == track && *q == p) {
                    if mode(open) != "playing" && mci(&format!("play {ALIAS} from 0 repeat")).0 != 0 {
                        mci(&format!("play {ALIAS} from 0"));
                    }
                    continue;
                }
                if open {
                    mci(&format!("stop {ALIAS}"));
                    mci(&format!("close {ALIAS}"));
                    open = false;
                }
                let ps = p.to_string_lossy();
                if mci(&format!("open \"{ps}\" type mpegvideo alias {ALIAS}")).0 != 0
                    && mci(&format!("open \"{ps}\" type waveaudio alias {ALIAS}")).0 != 0
                {
                    FAILED.store(true, Ordering::Relaxed);
                    continue;
                }
                open = true;
                cur = Some((track, p));
                mci(&format!("setaudio {ALIAS} volume to 1000"));
                if mci(&format!("play {ALIAS} repeat")).0 != 0 {
                    mci(&format!("play {ALIAS}"));
                }
            }
            Cmd::Stop => {
                if open {
                    mci(&format!("stop {ALIAS}"));
                    mci(&format!("close {ALIAS}"));
                }
                open = false;
            }
            Cmd::Keep => {
                if open && mode(open) == "stopped" {
                    mci(&format!("play {ALIAS} from 0"));
                }
            }
        }
    }
}

fn send(c: Cmd) {
    let mut g = TX.lock().unwrap();
    if g.is_none() {
        let (tx, rx) = channel();
        std::thread::spawn(move || run(rx));
        *g = Some(tx);
    }
    if let Some(tx) = g.as_ref() {
        let _ = tx.send(c);
    }
}

pub fn play(track: &str, vol: i64) {
    send(Cmd::Play(track.to_string(), vol));
}

pub fn stop() {
    send(Cmd::Stop);
}

/// music_watch: restart the loop if Windows stopped it
pub fn keep_going() {
    send(Cmd::Keep);
}

fn pid_name(pid: u32) -> String {
    unsafe {
        let h = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
        if h.is_null() {
            return String::new();
        }
        let mut pb = vec![0u16; 520];
        let mut size = 520u32;
        let mut out = String::new();
        if QueryFullProcessImageNameW(h, PROCESS_NAME_WIN32, pb.as_mut_ptr(), &mut size) != 0 {
            let full = String::from_utf16_lossy(&pb[..size as usize]);
            out = full.rsplit(['\\', '/']).next().unwrap_or("").to_lowercase();
        }
        CloseHandle(h);
        out
    }
}

unsafe extern "system" fn enum_cb(hwnd: HWND, l: LPARAM) -> i32 {
    let titles = unsafe { &mut *(l as *mut Vec<String>) };
    let n = unsafe { GetWindowTextLengthW(hwnd) };
    if n <= 0 {
        return 1;
    }
    let mut pid = 0u32;
    unsafe { GetWindowThreadProcessId(hwnd, &mut pid) };
    if pid_name(pid) == "spotify.exe" {
        let mut buf = vec![0u16; n as usize + 1];
        let got = unsafe { GetWindowTextW(hwnd, buf.as_mut_ptr(), n + 1) };
        let t = String::from_utf16_lossy(&buf[..got.max(0) as usize]).trim().to_string();
        let junk = t.contains("GDI+") || t.contains("IME") || t.contains("MSCTF") || t.contains("Legacy Window")
            || t.starts_with("Default");
        if !t.is_empty() && !junk {
            titles.push(t);
        }
    }
    1
}

/// spotify_window(): None = Spotify isn't open; otherwise playing / title / artist from its window title
pub fn spotify_window() -> Option<Player> {
    let mut titles: Vec<String> = vec![];
    unsafe { EnumWindows(Some(enum_cb), &mut titles as *mut _ as LPARAM) };
    if titles.is_empty() {
        return None;
    }
    for t in &titles {
        if t.contains(" - ") && !t.to_lowercase().starts_with("spotify") {
            let (artist, title) = t.split_once(" - ").unwrap();
            return Some(Player { playing: true, title: title.trim().into(), artist: artist.trim().into() });
        }
    }
    Some(Player::default())
}

/// media_key(which)
pub fn media_key(which: &str) {
    let vk = match which {
        "playpause" => 0xB3u8,
        "next" => 0xB0,
        "previous" => 0xB1,
        _ => return,
    };
    unsafe {
        keybd_event(vk, 0, KEYEVENTF_EXTENDEDKEY, 0);
        keybd_event(vk, 0, KEYEVENTF_EXTENDEDKEY | KEYEVENTF_KEYUP, 0);
    }
}
