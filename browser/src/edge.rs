// Import from Microsoft Edge, so you stay signed in everywhere.
//
// Zenko Browser runs on WebView2, which is the same engine as Edge and stores its profile in exactly the
// same file formats. So instead of re-reading every cookie and password, we copy Edge's profile files
// (cookies, saved passwords, autofill, history, bookmarks, site storage) into our own profile, together
// with the key that unlocks them. This must happen before any WebView2 window starts.
//
// Limits, stated honestly:
// - Edge must be fully closed while copying (it keeps the cookie file locked). We can close it for you.
// - If Edge has switched your cookies to "app-bound" encryption (only Edge itself can unlock those), the
//   sign-ins cannot be carried over; we detect this and say so. Passwords and the rest still come across.
use std::{
    io::ErrorKind,
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

use serde_json::{json, Value};

pub fn edge_root() -> PathBuf {
    let base = std::env::var_os("LOCALAPPDATA").map(PathBuf::from).unwrap_or_default();
    base.join("Microsoft").join("Edge").join("User Data")
}

/// Files inside the Edge profile folder that we copy (path relative to the profile).
const FILES: &[(&str, &str)] = &[
    ("Network/Cookies", "sign-ins"),
    ("Cookies", "sign-ins"),
    ("Login Data", "passwords"),
    ("Login Data For Account", "passwords"),
    ("Web Data", "autofill"),
    ("History", "history"),
    ("Bookmarks", "bookmarks"),
    ("Favicons", "icons"),
    ("Top Sites", "top sites"),
];
const DIRS: &[(&str, &str)] = &[("Local Storage", "site storage"), ("Session Storage", "site storage")];

fn read_json(p: &Path) -> Value {
    std::fs::read_to_string(p).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or(json!({}))
}

fn locked(e: &std::io::Error) -> bool {
    matches!(e.raw_os_error(), Some(32) | Some(33)) || e.kind() == ErrorKind::PermissionDenied
}

fn copy_retry(src: &Path, dst: &Path) -> Result<(), std::io::Error> {
    let t0 = Instant::now();
    loop {
        match std::fs::copy(src, dst) {
            Ok(_) => return Ok(()),
            Err(e) if locked(&e) && t0.elapsed() < Duration::from_secs(6) => std::thread::sleep(Duration::from_millis(400)),
            Err(e) => return Err(e),
        }
    }
}

fn copy_dir(src: &Path, dst: &Path) -> std::io::Result<u64> {
    let mut n = 0;
    std::fs::create_dir_all(dst)?;
    for e in std::fs::read_dir(src)? {
        let e = e?;
        let p = e.path();
        let q = dst.join(e.file_name());
        if p.is_dir() {
            n += copy_dir(&p, &q)?;
        } else if e.file_name() != "LOCK" {
            n += std::fs::copy(&p, &q)?;
        }
    }
    Ok(n)
}

/// Rough check of how Edge encrypted the cookies: "v10"/"v11" (DPAPI key, portable to WebView2) or
/// "v20" (app-bound, Edge-only).
fn app_bound(cookies: &Path) -> bool {
    let Ok(b) = std::fs::read(cookies) else { return false };
    let count = |pat: &[u8]| b.windows(3).filter(|w| *w == pat).count();
    let v20 = count(b"v20");
    v20 > 20 && v20 > (count(b"v10") + count(b"v11")) * 2
}

pub fn has_edge() -> bool {
    edge_root().join("Local State").exists()
}

/// Copy the Edge profile into `web_dir` (our WebView2 user data folder). Returns a report for the UI.
pub fn run(web_dir: &Path) -> Value {
    let root = edge_root();
    if !root.join("Local State").exists() {
        return json!({"state": "noedge"});
    }
    let edge_state = read_json(&root.join("Local State"));
    let profile = edge_state["profile"]["last_used"].as_str().filter(|s| !s.is_empty()).unwrap_or("Default").to_string();
    let src = root.join(&profile);
    if !src.exists() {
        return json!({"state": "noedge"});
    }
    let ours = web_dir.join("EBWebView");
    let dst = ours.join("Default");
    let _ = std::fs::create_dir_all(dst.join("Network"));

    let mut got: Vec<&str> = vec![];
    let mut busy: Vec<&str> = vec![];
    for (rel, what) in FILES {
        let s = src.join(rel);
        if !s.exists() {
            continue;
        }
        let d = dst.join(rel);
        if let Some(parent) = d.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        match copy_retry(&s, &d) {
            Ok(_) => {
                if !got.contains(what) {
                    got.push(what);
                }
                // the old journal belongs to our previous file; drop it so SQLite does not replay it
                let _ = std::fs::remove_file(d.with_file_name(format!("{}-journal", d.file_name().unwrap().to_string_lossy())));
            }
            Err(e) => {
                if locked(&e) && !busy.contains(what) {
                    busy.push(what);
                }
            }
        }
    }
    for (rel, what) in DIRS {
        let s = src.join(rel);
        if s.exists() && copy_dir(&s, &dst.join(rel)).is_ok() && !got.contains(what) {
            got.push(what);
        }
    }
    // The key that unlocks cookies and passwords lives in "Local State" -> os_crypt.
    if !edge_state["os_crypt"].is_null() {
        let lp = ours.join("Local State");
        let mut st = read_json(&lp);
        if !st.is_object() {
            st = json!({});
        }
        st["os_crypt"] = edge_state["os_crypt"].clone();
        let _ = std::fs::write(&lp, st.to_string());
    }
    let cookie_file = if dst.join("Network/Cookies").exists() { dst.join("Network/Cookies") } else { dst.join("Cookies") };
    let appbound = got.contains(&"sign-ins") && app_bound(&cookie_file);
    let state = if busy.contains(&"sign-ins") { "locked" } else { "done" };
    json!({"state": state, "profile": profile, "copied": got, "busy": busy, "appbound": appbound})
}

/// Bookmarks (from our imported copy) flattened for the command bar.
pub fn bookmarks(web_dir: &Path) -> Value {
    let v = read_json(&web_dir.join("EBWebView").join("Default").join("Bookmarks"));
    let mut out = vec![];
    fn walk(n: &Value, folder: &str, out: &mut Vec<Value>) {
        if out.len() >= 4000 {
            return;
        }
        match n["type"].as_str() {
            Some("url") => out.push(json!({"t": n["name"], "u": n["url"], "f": folder})),
            Some("folder") => {
                let name = n["name"].as_str().unwrap_or("");
                for c in n["children"].as_array().into_iter().flatten() {
                    walk(c, name, out);
                }
            }
            _ => {}
        }
    }
    if let Some(roots) = v["roots"].as_object() {
        for (_, r) in roots {
            walk(r, "", &mut out);
        }
    }
    Value::Array(out)
}

// ---------------------------------------------------------------- closing Edge (Windows)

#[cfg(windows)]
fn edge_pids() -> Vec<u32> {
    use windows::Win32::{
        Foundation::CloseHandle,
        System::Diagnostics::ToolHelp::{CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W, TH32CS_SNAPPROCESS},
    };
    let mut out = vec![];
    unsafe {
        let Ok(snap) = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) else { return out };
        let mut e = PROCESSENTRY32W { dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32, ..Default::default() };
        if Process32FirstW(snap, &mut e).is_ok() {
            loop {
                let n = e.szExeFile.iter().position(|&c| c == 0).unwrap_or(e.szExeFile.len());
                let name = String::from_utf16_lossy(&e.szExeFile[..n]).to_ascii_lowercase();
                if name == "msedge.exe" {
                    out.push(e.th32ProcessID);
                }
                if Process32NextW(snap, &mut e).is_err() {
                    break;
                }
            }
        }
        let _ = CloseHandle(snap);
    }
    out
}
#[cfg(not(windows))]
fn edge_pids() -> Vec<u32> {
    vec![]
}

pub fn edge_running() -> bool {
    !edge_pids().is_empty()
}

/// Closes every Edge process (Edge restores its tabs next time it opens).
pub fn close_edge() {
    #[cfg(windows)]
    unsafe {
        use windows::Win32::{
            Foundation::CloseHandle,
            System::Threading::{OpenProcess, TerminateProcess, PROCESS_TERMINATE},
        };
        for pid in edge_pids() {
            if let Ok(h) = OpenProcess(PROCESS_TERMINATE, false, pid) {
                let _ = TerminateProcess(h, 0);
                let _ = CloseHandle(h);
            }
        }
    }
    let t0 = Instant::now();
    while edge_running() && t0.elapsed() < Duration::from_secs(5) {
        std::thread::sleep(Duration::from_millis(200));
    }
    std::thread::sleep(Duration::from_millis(500));
}

/// When we restart ourselves, wait for the old copy (and its WebView2 helpers) to let go of the files.
pub fn wait_for_pid(pid: u32) {
    #[cfg(windows)]
    unsafe {
        use windows::Win32::{
            Foundation::CloseHandle,
            System::Threading::{OpenProcess, WaitForSingleObject, PROCESS_SYNCHRONIZE},
        };
        if let Ok(h) = OpenProcess(PROCESS_SYNCHRONIZE, false, pid) {
            let _ = WaitForSingleObject(h, 10_000);
            let _ = CloseHandle(h);
        }
    }
    let _ = pid;
    std::thread::sleep(Duration::from_millis(1500));
}
