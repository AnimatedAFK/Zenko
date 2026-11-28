//! http_get (zenko.py urllib with the "FoxBuddy/1.0 (personal focus app)" User-Agent) through Windows' own WinHTTP,
//! and weather_poll's background job: geocode the home city once, then read the current weather code.

use std::sync::Mutex;
use windows_sys::Win32::Networking::WinHttp::*;
use zenko_desktop::slow::{forecast_url, geocode_pick, geocode_url};

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(Some(0)).collect()
}

pub fn http_get(url: &str, timeout_ms: i32) -> Option<Vec<u8>> {
    let mut body = vec![];
    match request(&Req { url, ua: "FoxBuddy/1.0 (personal focus app)", timeout_ms,
                         headers: "Accept: application/json, application/xml, */*", ..Default::default() },
                  &mut |b| {
                      body.extend_from_slice(b);
                      true
                  }) {
        Ok(st) if st < 400 => Some(body),
        _ => None,
    }
}

/// one HTTP request (urllib.request.urlopen as zenko.py makes them)
#[derive(Default)]
pub struct Req<'a> {
    pub url: &'a str,
    /// GET when empty
    pub method: &'a str,
    pub ua: &'a str,
    /// extra header lines, "\r\n" between them
    pub headers: &'a str,
    pub body: &'a [u8],
    pub timeout_ms: i32,
}

/// sends `r` through WinHTTP and hands each piece of the answer to `chunk` as it arrives (false stops reading);
/// gives the HTTP status, or what went wrong
pub fn request(r: &Req, chunk: &mut dyn FnMut(&[u8]) -> bool) -> Result<u32, String> {
    let (secure, rest) = if let Some(x) = r.url.strip_prefix("https://") {
        (true, x)
    } else if let Some(x) = r.url.strip_prefix("http://") {
        (false, x)
    } else {
        return Err(format!("unknown url type: {}", r.url));
    };
    let (hostport, path) = rest.split_once('/').map(|(h, p)| (h, format!("/{p}"))).unwrap_or((rest, "/".into()));
    let (host, port) = match hostport.rsplit_once(':') {
        Some((h, p)) if p.bytes().all(|b| b.is_ascii_digit()) && !p.is_empty() => (h, p.parse::<u16>().unwrap_or(0)),
        _ => (hostport, if secure { INTERNET_DEFAULT_HTTPS_PORT as u16 } else { INTERNET_DEFAULT_HTTP_PORT as u16 }),
    };
    let method = if r.method.is_empty() { "GET" } else { r.method };
    let t = if r.timeout_ms > 0 { r.timeout_ms } else { 30000 };
    unsafe {
        let s = WinHttpOpen(wide(r.ua).as_ptr(), WINHTTP_ACCESS_TYPE_AUTOMATIC_PROXY, std::ptr::null(), std::ptr::null(), 0);
        if s.is_null() {
            return Err("WinHttpOpen failed".into());
        }
        WinHttpSetTimeouts(s, t, t, t, t);
        let mut res = Err(format!("<urlopen error [Errno 11001] getaddrinfo failed: {host}>"));
        let c = WinHttpConnect(s, wide(host).as_ptr(), port, 0);
        if !c.is_null() {
            let q = WinHttpOpenRequest(c, wide(method).as_ptr(), wide(&path).as_ptr(), std::ptr::null(), std::ptr::null(),
                                       std::ptr::null(), if secure { WINHTTP_FLAG_SECURE } else { 0 });
            if !q.is_null() {
                let hdr = wide(r.headers);
                let (hp, hl) = if r.headers.is_empty() { (std::ptr::null(), 0) } else { (hdr.as_ptr(), u32::MAX) };
                if WinHttpSendRequest(q, hp, hl, r.body.as_ptr() as *const _, r.body.len() as u32, r.body.len() as u32, 0) != 0
                    && WinHttpReceiveResponse(q, std::ptr::null_mut()) != 0
                {
                    let mut status = 0u32;
                    let mut sz = 4u32;
                    WinHttpQueryHeaders(q, WINHTTP_QUERY_STATUS_CODE | WINHTTP_QUERY_FLAG_NUMBER, std::ptr::null(),
                                        &mut status as *mut u32 as *mut _, &mut sz, std::ptr::null_mut());
                    if status >= 400 {
                        res = Err(format!("HTTP Error {status}"));
                    } else {
                        res = Ok(status);
                    }
                    let mut buf = vec![0u8; 16384];
                    loop {
                        let mut got = 0u32;
                        if WinHttpReadData(q, buf.as_mut_ptr() as *mut _, buf.len() as u32, &mut got) == 0 {
                            if res.is_ok() {
                                res = Err("The read operation timed out".into());
                            }
                            break;
                        }
                        if got == 0 || !chunk(&buf[..got as usize]) {
                            break;
                        }
                    }
                } else {
                    res = Err(format!("<urlopen error {}>", windows_sys::Win32::Foundation::GetLastError()));
                }
                WinHttpCloseHandle(q);
            }
            WinHttpCloseHandle(c);
        }
        WinHttpCloseHandle(s);
        res
    }
}

fn http_json(url: &str) -> Option<serde_json::Value> {
    serde_json::from_slice(&http_get(url, 12000)?).ok()
}

/// (city, geocode result) cached like self.wx["geo"]
static GEO: Mutex<Option<(String, serde_json::Value)>> = Mutex::new(None);
/// the weather code the last poll found, picked up by the window loop
pub static CODE: Mutex<Option<i64>> = Mutex::new(None);

/// weather_poll's job, on a background thread
pub fn weather_poll(city: String) {
    std::thread::spawn(move || {
        let geo = {
            let g = GEO.lock().unwrap().clone();
            match g {
                Some((c, v)) if c == city => Some(v),
                _ => {
                    let v = http_json(&geocode_url(&city)).and_then(|j| geocode_pick(&city, &j));
                    if let Some(v) = &v {
                        *GEO.lock().unwrap() = Some((city.clone(), v.clone()));
                    }
                    v
                }
            }
        };
        let Some(geo) = geo else { return };
        if let Some(code) = http_json(&forecast_url(&geo)).and_then(|j| j["current"]["weather_code"].as_i64()) {
            *CODE.lock().unwrap() = Some(code);
        }
    });
}
