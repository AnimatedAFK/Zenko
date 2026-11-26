//! http_get (zenko.py urllib with the "FoxBuddy/1.0 (personal focus app)" User-Agent) through Windows' own WinHTTP,
//! and weather_poll's background job: geocode the home city once, then read the current weather code.

use std::sync::Mutex;
use windows_sys::Win32::Networking::WinHttp::*;
use zenko_desktop::slow::{forecast_url, geocode_pick, geocode_url};

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(Some(0)).collect()
}

pub fn http_get(url: &str, timeout_ms: i32) -> Option<Vec<u8>> {
    let rest = url.strip_prefix("https://")?;
    let (host, path) = rest.split_once('/').map(|(h, p)| (h, format!("/{p}"))).unwrap_or((rest, "/".into()));
    unsafe {
        let s = WinHttpOpen(wide("FoxBuddy/1.0 (personal focus app)").as_ptr(), WINHTTP_ACCESS_TYPE_AUTOMATIC_PROXY,
                            std::ptr::null(), std::ptr::null(), 0);
        if s.is_null() {
            return None;
        }
        WinHttpSetTimeouts(s, timeout_ms, timeout_ms, timeout_ms, timeout_ms);
        let mut out = None;
        let c = WinHttpConnect(s, wide(host).as_ptr(), INTERNET_DEFAULT_HTTPS_PORT as u16, 0);
        if !c.is_null() {
            let r = WinHttpOpenRequest(c, wide("GET").as_ptr(), wide(&path).as_ptr(), std::ptr::null(), std::ptr::null(),
                                       std::ptr::null(), WINHTTP_FLAG_SECURE);
            if !r.is_null() {
                let hdr = wide("Accept: application/json, application/xml, */*");
                if WinHttpSendRequest(r, hdr.as_ptr(), u32::MAX, std::ptr::null(), 0, 0, 0) != 0
                    && WinHttpReceiveResponse(r, std::ptr::null_mut()) != 0
                {
                    let mut status = 0u32;
                    let mut sz = 4u32;
                    WinHttpQueryHeaders(r, WINHTTP_QUERY_STATUS_CODE | WINHTTP_QUERY_FLAG_NUMBER, std::ptr::null(),
                                        &mut status as *mut u32 as *mut _, &mut sz, std::ptr::null_mut());
                    if status < 400 {
                        let mut body = vec![];
                        loop {
                            let mut avail = 0u32;
                            if WinHttpQueryDataAvailable(r, &mut avail) == 0 || avail == 0 {
                                break;
                            }
                            let mut buf = vec![0u8; avail as usize];
                            let mut got = 0u32;
                            if WinHttpReadData(r, buf.as_mut_ptr() as *mut _, avail, &mut got) == 0 {
                                break;
                            }
                            body.extend_from_slice(&buf[..got as usize]);
                        }
                        out = Some(body);
                    }
                }
                WinHttpCloseHandle(r);
            }
            WinHttpCloseHandle(c);
        }
        WinHttpCloseHandle(s);
        out
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
