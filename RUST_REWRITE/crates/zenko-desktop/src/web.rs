//! zenko.py's web helpers: http_get / http_json (WebError messages as the user sees them), _download (a .part file,
//! progress, sha256 and size checks) and urllib.parse.quote. The request itself goes through whatever the program
//! registers with set_fetch (WinHTTP in the Windows build, a mock in the tests), so this file stays portable.

use serde_json::Value;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, RwLock};

/// one HTTP request (urllib.request.Request)
#[derive(Default, Clone, Debug)]
pub struct Req {
    pub url: String,
    /// GET when empty
    pub method: String,
    pub ua: String,
    /// extra header lines, "\r\n" between them
    pub headers: String,
    pub body: Vec<u8>,
    pub timeout_ms: i32,
}

/// sends a request and hands each piece of the answer to `chunk(bytes, content_length or 0)` (false stops reading);
/// gives the HTTP status, or Err("HTTP Error <code>") / Err(what went wrong)
pub type Fetcher = dyn Fn(&Req, &mut dyn FnMut(&[u8], u64) -> bool) -> Result<u32, String> + Send + Sync;

static FETCH: RwLock<Option<Arc<Fetcher>>> = RwLock::new(None);
static TEST: Mutex<()> = Mutex::new(());

pub fn set_fetch(f: Arc<Fetcher>) {
    *FETCH.write().unwrap() = Some(f);
}

/// tests that swap the fetcher hold this
pub fn test_lock() -> std::sync::MutexGuard<'static, ()> {
    TEST.lock().unwrap_or_else(|e| e.into_inner())
}

pub fn fetch(r: &Req, chunk: &mut dyn FnMut(&[u8], u64) -> bool) -> Result<u32, String> {
    let f = FETCH.read().unwrap().clone();
    match f {
        Some(f) => f(r, chunk),
        None => Err("<urlopen error no network>".into()),
    }
}

pub const UA: &str = "FoxBuddy/1.0 (personal focus app)";
pub const E_NET: &str = "I can't reach the internet right now. Check your Wi-Fi, or turn the VPN off/on, and try again.";
pub const E_READ: &str = "I got a reply I couldn't read. Try again later.";

/// the HTTP status in an "HTTP Error 404" answer
fn http_code(e: &str) -> Option<&str> {
    e.strip_prefix("HTTP Error ").map(|s| s.split(':').next().unwrap_or(s).trim())
}

/// e.__class__.__name__ for a failed urlopen
pub fn err_class(e: &str) -> &'static str {
    if http_code(e).is_some() {
        "HTTPError"
    } else if e.contains("timed out") {
        "TimeoutError"
    } else {
        "URLError"
    }
}

/// http_get(url, timeout): the body, or the WebError message
pub fn http_get(url: &str, timeout_s: f64) -> Result<Vec<u8>, String> {
    let r = Req { url: url.into(), ua: UA.into(), headers: "Accept: application/json, application/xml, */*".into(),
                  timeout_ms: (timeout_s * 1000.0) as i32, ..Default::default() };
    let mut body = vec![];
    match fetch(&r, &mut |b, _| {
        body.extend_from_slice(b);
        true
    }) {
        Ok(st) if st < 400 => Ok(body),
        Ok(st) => Err(format!("The website answered with an error ({st}). Try again in a moment.")),
        Err(e) => match http_code(&e) {
            Some(c) => Err(format!("The website answered with an error ({c}). Try again in a moment.")),
            None => Err(E_NET.into()),
        },
    }
}

/// http_json(url, timeout)
pub fn http_json(url: &str, timeout_s: f64) -> Result<Value, String> {
    let b = http_get(url, timeout_s)?;
    serde_json::from_str(&String::from_utf8_lossy(&b)).map_err(|_| E_READ.to_string())
}

/// urllib.parse.quote(s, safe)
pub fn quote(s: &str, safe: &str) -> String {
    let mut o = String::new();
    for b in s.bytes() {
        if b.is_ascii_alphanumeric() || b"_.-~".contains(&b) || safe.as_bytes().contains(&b) {
            o.push(b as char);
        } else {
            o.push_str(&format!("%{b:02X}"));
        }
    }
    o
}

/// _q(s): quote(s, safe="")
pub fn q(s: &str) -> String {
    quote(s, "")
}

fn file_name(p: &Path) -> String {
    p.file_name().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default()
}

/// _download(url, dest, sha256, min_size, progress, label): into dest + ".part", then moved over dest
pub fn download(url: &str, dest: &Path, sha256: &str, min_size: u64, progress: Option<&mut dyn FnMut(&str, f64)>,
                label: &str) -> Result<PathBuf, String> {
    use std::io::Write;
    let tmp = PathBuf::from(format!("{}.part", dest.display()));
    let r = Req { url: url.into(), ua: "Zenko".into(), timeout_ms: 40000, ..Default::default() };
    let mut h = Sha256::new();
    let mut got = 0u64;
    let mut progress = progress;
    let res = (|| -> Result<(), String> {
        let mut f = std::fs::File::create(&tmp).map_err(|_| "FileNotFoundError".to_string())?;
        let mut werr = None;
        let st = fetch(&r, &mut |b, total| {
            if let Err(_e) = f.write_all(b) {
                werr = Some("OSError".to_string());
                return false;
            }
            h.update(b);
            got += b.len() as u64;
            if let (Some(p), true) = (progress.as_mut(), total > 0) {
                p(label, got as f64 / total as f64);
            }
            true
        });
        if let Some(e) = werr {
            return Err(e);
        }
        match st {
            Ok(s) if s < 400 => Ok(()),
            Ok(_) => Err("HTTPError".into()),
            Err(e) => Err(err_class(&e).into()),
        }
    })();
    let name = if label.is_empty() { file_name(dest) } else { label.to_string() };
    if let Err(cls) = res {
        let _ = std::fs::remove_file(&tmp);
        return Err(format!("The download of {name} didn't finish ({cls}). The network may block it: see Settings > Talk for \
                            how to add the files by hand."));
    }
    if !sha256.is_empty() && h.hex() != sha256.to_lowercase() {
        let _ = std::fs::remove_file(&tmp);
        return Err(format!("{} didn't match its checksum, so I threw it away.", if label.is_empty() { "The download" } else { label }));
    }
    if std::fs::metadata(&tmp).map(|m| m.len()).unwrap_or(0) < min_size {
        let _ = std::fs::remove_file(&tmp);
        return Err(format!("{} came back too small (probably a blocked page instead of the file).",
                           if label.is_empty() { "The download" } else { label }));
    }
    std::fs::rename(&tmp, dest).map_err(|e| e.to_string())?;
    Ok(dest.to_path_buf())
}

/// hashlib.sha256
pub struct Sha256 {
    h: [u32; 8],
    buf: Vec<u8>,
    len: u64,
}

const K: [u32; 64] = [
    0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5, 0xd807aa98, 0x12835b01,
    0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc,
    0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147,
    0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85,
    0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070, 0x19a4c116, 0x1e376c08,
    0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3, 0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208,
    0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2,
];

impl Default for Sha256 {
    fn default() -> Self {
        Self::new()
    }
}

impl Sha256 {
    pub fn new() -> Sha256 {
        Sha256 { h: [0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab, 0x5be0cd19], buf: vec![], len: 0 }
    }

    fn block(&mut self, b: &[u8]) {
        let mut w = [0u32; 64];
        for i in 0..16 {
            w[i] = u32::from_be_bytes([b[4 * i], b[4 * i + 1], b[4 * i + 2], b[4 * i + 3]]);
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16].wrapping_add(s0).wrapping_add(w[i - 7]).wrapping_add(s1);
        }
        let mut v = self.h;
        for i in 0..64 {
            let s1 = v[4].rotate_right(6) ^ v[4].rotate_right(11) ^ v[4].rotate_right(25);
            let ch = (v[4] & v[5]) ^ (!v[4] & v[6]);
            let t1 = v[7].wrapping_add(s1).wrapping_add(ch).wrapping_add(K[i]).wrapping_add(w[i]);
            let s0 = v[0].rotate_right(2) ^ v[0].rotate_right(13) ^ v[0].rotate_right(22);
            let maj = (v[0] & v[1]) ^ (v[0] & v[2]) ^ (v[1] & v[2]);
            let t2 = s0.wrapping_add(maj);
            v = [t1.wrapping_add(t2), v[0], v[1], v[2], v[3].wrapping_add(t1), v[4], v[5], v[6]];
        }
        for i in 0..8 {
            self.h[i] = self.h[i].wrapping_add(v[i]);
        }
    }

    pub fn update(&mut self, d: &[u8]) {
        self.len += d.len() as u64;
        self.buf.extend_from_slice(d);
        let n = self.buf.len() / 64 * 64;
        let full: Vec<u8> = self.buf.drain(..n).collect();
        for c in full.chunks(64) {
            self.block(c);
        }
    }

    pub fn hex(&self) -> String {
        let mut s = Sha256 { h: self.h, buf: self.buf.clone(), len: self.len };
        let bits = s.len * 8;
        let mut pad = vec![0x80u8];
        while (s.buf.len() + pad.len()) % 64 != 56 {
            pad.push(0);
        }
        pad.extend_from_slice(&bits.to_be_bytes());
        s.buf.extend(pad);
        let full = std::mem::take(&mut s.buf);
        for c in full.chunks(64) {
            s.block(c);
        }
        s.h.iter().map(|x| format!("{x:08x}")).collect()
    }
}

/// translate_text(text, target, source): Google's public endpoint, then MyMemory -> (translation, romanisation, engine)
pub fn translate_text(text: &str, target: &str, source: &str) -> Result<(String, String, String), String> {
    let url = format!("https://translate.googleapis.com/translate_a/single?client=gtx&sl={source}&tl={target}&dt=t&dt=rm&q={}", q(text));
    if let Ok(b) = http_get(&url, 10.0) {
        if let Ok(j) = serde_json::from_str::<Value>(&String::from_utf8_lossy(&b)) {
            let segs = j[0].as_array().cloned().unwrap_or_default();
            let truthy = |v: &Value| match v {
                Value::Null => false,
                Value::String(s) => !s.is_empty(),
                Value::Array(a) => !a.is_empty(),
                _ => true,
            };
            let out: String = segs.iter().filter(|s| truthy(s) && truthy(&s[0])).map(|s| s[0].as_str().unwrap_or("").to_string()).collect();
            let roman: String = segs.iter()
                .filter(|s| truthy(s) && s[0].is_null() && s.as_array().is_some_and(|a| a.len() > 2) && s[2].is_string())
                .map(|s| s[2].as_str().unwrap().to_string()).collect();
            if !out.trim().is_empty() {
                return Ok((out.trim().to_string(), roman.trim().to_string(), "Google Translate".into()));
            }
        }
    }
    let mut src = if source == "auto" { "en" } else { source };
    if src == target {
        src = if target != "en" { "en" } else { "ja" };
    }
    let cut: String = text.chars().take(450).collect();
    let j = http_json(&format!("https://api.mymemory.translated.net/get?q={}&langpair={src}|{target}", q(&cut)), 10.0)?;
    let out = j["responseData"]["translatedText"].as_str().unwrap_or("").trim().to_string();
    if out.is_empty() {
        return Err("I couldn't translate that right now. Try again in a moment?".into());
    }
    Ok((out, String::new(), "MyMemory".into()))
}

/// urllib.parse.unquote_plus(s)
pub fn unquote_plus(s: &str) -> String {
    let b = s.as_bytes();
    let mut o = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        match b[i] {
            b'+' => o.push(b' '),
            b'%' if i + 2 < b.len() => {
                match u8::from_str_radix(std::str::from_utf8(&b[i + 1..i + 3]).unwrap_or("zz"), 16) {
                    Ok(v) => {
                        o.push(v);
                        i += 2;
                    }
                    Err(_) => o.push(b'%'),
                }
            }
            c => o.push(c),
        }
        i += 1;
    }
    String::from_utf8_lossy(&o).into_owned()
}
