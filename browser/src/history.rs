// Browsing history for the address bar's autocomplete ("y" -> youtube.com).
// Sources: Edge's history (imported copy, read with Windows' own built-in SQLite, winsqlite3.dll, so we
// ship no database engine) plus every page you open in Zenko Browser (history.json).
use std::{collections::HashMap, path::Path};

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct Entry {
    pub t: String, // title
    pub c: u32,    // visits
    pub y: u32,    // typed / opened from the address bar
    pub l: u64,    // last visit, unix seconds
}

pub fn now() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

pub fn load(path: &Path) -> HashMap<String, Entry> {
    std::fs::read_to_string(path).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default()
}

pub fn save(path: &Path, h: &HashMap<String, Entry>) {
    // keep the 6000 most useful entries
    let mut v: Vec<(&String, &Entry)> = h.iter().collect();
    v.sort_by(|a, b| score(b.1).partial_cmp(&score(a.1)).unwrap_or(std::cmp::Ordering::Equal));
    let keep: HashMap<&String, &Entry> = v.into_iter().take(6000).collect();
    if let Ok(t) = serde_json::to_string(&keep) {
        let tmp = path.with_extension("tmp");
        if std::fs::write(&tmp, t).is_ok() {
            let _ = std::fs::rename(&tmp, path);
        }
    }
}

pub fn score(e: &Entry) -> f64 {
    let age_days = (now().saturating_sub(e.l)) as f64 / 86400.0;
    (e.c as f64 + 4.0 * e.y as f64) / (1.0 + age_days / 30.0)
}

pub fn visit(h: &mut HashMap<String, Entry>, url: &str, title: &str) {
    if !(url.starts_with("http://") || url.starts_with("https://")) || url.len() > 2000 {
        return;
    }
    let e = h.entry(url.to_string()).or_default();
    e.c += 1;
    e.l = now();
    if !title.is_empty() {
        e.t = title.chars().take(200).collect();
    }
}

pub fn set_title(h: &mut HashMap<String, Entry>, url: &str, title: &str) {
    if let Some(e) = h.get_mut(url) {
        e.t = title.chars().take(200).collect();
    }
}

/// The list the address bar uses, best first.
pub fn for_ui(h: &HashMap<String, Entry>) -> Value {
    let mut v: Vec<(&String, &Entry)> = h.iter().collect();
    v.sort_by(|a, b| score(b.1).partial_cmp(&score(a.1)).unwrap_or(std::cmp::Ordering::Equal));
    Value::Array(v.into_iter().take(4000).map(|(u, e)| json!({"u": u, "t": e.t, "s": (score(e) * 10.0).round() / 10.0})).collect())
}

/// Read Edge's History database (our imported copy) into `h`. Only called before WebView2 starts.
#[cfg(windows)]
pub fn merge_edge(h: &mut HashMap<String, Entry>, db: &Path) -> usize {
    use std::ffi::{c_char, c_int, c_void, CStr, CString};
    use windows::{
        core::{s, w},
        Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryW},
    };
    if !db.exists() {
        return 0;
    }
    type Open = unsafe extern "system" fn(*const c_char, *mut *mut c_void, c_int, *const c_char) -> c_int;
    type Prep = unsafe extern "system" fn(*mut c_void, *const c_char, c_int, *mut *mut c_void, *mut *const c_char) -> c_int;
    type Step = unsafe extern "system" fn(*mut c_void) -> c_int;
    type ColT = unsafe extern "system" fn(*mut c_void, c_int) -> *const u8;
    type ColI = unsafe extern "system" fn(*mut c_void, c_int) -> i64;
    type Fin = unsafe extern "system" fn(*mut c_void) -> c_int;
    let mut n = 0;
    unsafe {
        let Ok(lib) = LoadLibraryW(w!("winsqlite3.dll")) else { return 0 };
        let (Some(open), Some(prep), Some(step), Some(colt), Some(coli), Some(fin), Some(close)) = (
            GetProcAddress(lib, s!("sqlite3_open_v2")),
            GetProcAddress(lib, s!("sqlite3_prepare_v2")),
            GetProcAddress(lib, s!("sqlite3_step")),
            GetProcAddress(lib, s!("sqlite3_column_text")),
            GetProcAddress(lib, s!("sqlite3_column_int64")),
            GetProcAddress(lib, s!("sqlite3_finalize")),
            GetProcAddress(lib, s!("sqlite3_close")),
        ) else {
            return 0;
        };
        let open: Open = std::mem::transmute(open);
        let prep: Prep = std::mem::transmute(prep);
        let step: Step = std::mem::transmute(step);
        let colt: ColT = std::mem::transmute(colt);
        let coli: ColI = std::mem::transmute(coli);
        let fin: Fin = std::mem::transmute(fin);
        let close: Fin = std::mem::transmute(close);
        // read-only + immutable: never touches or locks the file
        let uri = format!("file:///{}?immutable=1", db.to_string_lossy().replace('\\', "/").replace(' ', "%20"));
        let Ok(curi) = CString::new(uri) else { return 0 };
        let mut con: *mut c_void = std::ptr::null_mut();
        if open(curi.as_ptr(), &mut con, 0x01 | 0x40, std::ptr::null()) != 0 {
            if !con.is_null() {
                close(con);
            }
            return 0;
        }
        let sql = CString::new("SELECT url, title, visit_count, typed_count, last_visit_time FROM urls WHERE hidden = 0 ORDER BY visit_count DESC LIMIT 5000").unwrap();
        let mut st: *mut c_void = std::ptr::null_mut();
        if prep(con, sql.as_ptr(), -1, &mut st, std::ptr::null_mut()) == 0 {
            while step(st) == 100 {
                let txt = |i: c_int| {
                    let p = colt(st, i);
                    if p.is_null() { String::new() } else { CStr::from_ptr(p as *const c_char).to_string_lossy().into_owned() }
                };
                let url = txt(0);
                if !(url.starts_with("http://") || url.starts_with("https://")) {
                    continue;
                }
                // Chromium time = microseconds since 1601-01-01
                let lv = coli(st, 4);
                let unix = if lv > 11_644_473_600_000_000 { ((lv - 11_644_473_600_000_000) / 1_000_000) as u64 } else { 0 };
                let e = h.entry(url).or_default();
                if e.t.is_empty() {
                    e.t = txt(1).chars().take(200).collect();
                }
                e.c = e.c.max(coli(st, 2).max(0) as u32);
                e.y = e.y.max(coli(st, 3).max(0) as u32);
                e.l = e.l.max(unix);
                n += 1;
            }
            fin(st);
        }
        close(con);
    }
    n
}
#[cfg(not(windows))]
pub fn merge_edge(_h: &mut HashMap<String, Entry>, _db: &Path) -> usize {
    0
}
