//! zenko.py's Spotify account (class Spotify): the PKCE sign-in with its reply caught on 127.0.0.1:8765, the saved
//! token (spotify_token.json, refreshed when it runs out), the Web API with its friendly errors, waking a device,
//! now(), play_query(), do(), spotify_link_to_uri, find_preset and play_preset. Network calls go through web::fetch.

use crate::web::{self, Req};
use serde_json::{Value, json};
use std::path::PathBuf;

pub const PORT: u16 = 8765;
pub const REDIRECT: &str = "http://127.0.0.1:8765/callback";
pub const SCOPES: &str = "user-read-playback-state user-modify-playback-state user-read-currently-playing \
                          playlist-read-private playlist-read-collaborative user-library-read user-library-modify";

/// SPOTIFY: auth / token / api (FOX_SPOTIFY_BASE moves them, for tests)
pub fn urls() -> (String, String, String) {
    match std::env::var("FOX_SPOTIFY_BASE") {
        Ok(b) => {
            let b = b.trim_end_matches('/').to_string();
            (format!("{b}/authorize"), format!("{b}/api/token"), format!("{b}/v1"))
        }
        Err(_) => ("https://accounts.spotify.com/authorize".into(), "https://accounts.spotify.com/api/token".into(),
                   "https://api.spotify.com/v1".into()),
    }
}

pub fn token_path() -> PathBuf {
    crate::data_dir().join("spotify_token.json")
}

fn now_s() -> f64 {
    crate::unix_now()
}

/// urllib.parse.urlencode (quote_plus)
pub fn urlencode(pairs: &[(&str, String)]) -> String {
    pairs.iter().map(|(k, v)| format!("{}={}", web::quote(k, " ").replace(' ', "+"), web::quote(v, " ").replace(' ', "+")))
        .collect::<Vec<_>>().join("&")
}

fn b64url(d: &[u8]) -> String {
    const A: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
    let mut o = String::new();
    for c in d.chunks(3) {
        let n = (c[0] as u32) << 16 | (*c.get(1).unwrap_or(&0) as u32) << 8 | *c.get(2).unwrap_or(&0) as u32;
        for i in 0..c.len() + 1 {
            o.push(A[((n >> (18 - 6 * i)) & 63) as usize] as char);
        }
    }
    o
}

fn random_bytes(n: usize) -> Vec<u8> {
    use std::hash::{BuildHasher, Hasher};
    let mut out = vec![];
    while out.len() < n {
        let mut h = std::collections::hash_map::RandomState::new().build_hasher();
        h.write_usize(out.len());
        h.write_u128(std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0));
        out.extend(h.finish().to_le_bytes());
    }
    out.truncate(n);
    out
}

/// secrets.token_urlsafe(n)
pub fn token_urlsafe(n: usize) -> String {
    b64url(&random_bytes(n))
}

/// the PKCE challenge: base64url(sha256(verifier)) without '='
pub fn challenge(verifier: &str) -> String {
    let mut h = web::Sha256::new();
    h.update(verifier.as_bytes());
    let hex = h.hex();
    let bytes: Vec<u8> = (0..hex.len() / 2).map(|i| u8::from_str_radix(&hex[2 * i..2 * i + 2], 16).unwrap_or(0)).collect();
    b64url(&bytes)
}

/// spotify_link_to_uri(v): a share link as spotify:type:id
pub fn link_to_uri(v: &str) -> Option<String> {
    let v = v.trim();
    if v.starts_with("spotify:") {
        return Some(v.into());
    }
    let re = regex::Regex::new(r"open\.spotify\.com/(?:intl-[a-z]+/)?(track|playlist|album|artist)/([A-Za-z0-9]+)").unwrap();
    re.captures(v).map(|c| format!("spotify:{}:{}", &c[1], &c[2]))
}

fn ratio(a: &str, b: &str) -> f64 {
    crate::pydifflib::ratio(a, b)
}

/// find_preset(q): the music preset a request names
pub fn find_preset(presets: &[Value], q: &str) -> Option<Value> {
    let q = q.to_lowercase();
    let q = regex::Regex::new(r"\bfavorite\b").unwrap().replace_all(&q, "favourite").into_owned();
    let q = regex::Regex::new(r"\b(my|the|preset|please|music|some)\b").unwrap().replace_all(&q, " ").into_owned();
    let q = regex::Regex::new(r"\s+").unwrap().replace_all(&q, " ").trim().to_string();
    if q.is_empty() {
        return None;
    }
    let has = |p: &Value| p["value"].as_str().is_some_and(|s| !s.is_empty());
    if let Some(c) = regex::Regex::new(r"^(?:number |no\.? ?|#)?(\d)$").unwrap().captures(&q) {
        let i = c[1].parse::<i64>().unwrap_or(0) - 1;
        return (i >= 0).then(|| presets.get(i as usize)).flatten().filter(|p| has(p)).cloned();
    }
    let (mut best, mut sc) = (None, 0.0);
    for p in presets.iter().filter(|p| has(p)) {
        let name = p["name"].as_str().unwrap_or("").to_lowercase();
        let name = regex::Regex::new(r"\bfavorite\b").unwrap().replace_all(&name, "favourite").into_owned();
        let s = ratio(&q, &name) + if q.contains(&name) || name.contains(&q) { 0.4 } else { 0.0 };
        if s > sc {
            (best, sc) = (Some(p.clone()), s);
        }
    }
    if sc >= 0.8 { best } else { None }
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct Now {
    pub title: String,
    pub artist: String,
    pub playing: bool,
    pub uri: String,
    pub volume: Option<i64>,
    pub shuffle: Option<bool>,
    pub progress: i64,
    pub duration: i64,
}

pub struct Spotify {
    pub client_id: String,
    pub tok: Option<Value>,
    /// os.startfile("spotify:"): opens the Spotify app when no device is open (the host sets it)
    pub open_app: Option<fn()>,
    /// time.sleep (tests make it instant)
    pub sleep: fn(f64),
}

fn real_sleep(s: f64) {
    std::thread::sleep(std::time::Duration::from_secs_f64(s));
}

type R<T> = Result<T, String>;

impl Spotify {
    pub fn new(client_id: &str) -> Spotify {
        let tok = std::fs::read_to_string(token_path()).ok().and_then(|s| serde_json::from_str(&s).ok());
        Spotify { client_id: client_id.trim().into(), tok, open_app: None, sleep: real_sleep }
    }

    pub fn connected(&self) -> bool {
        self.tok.as_ref().and_then(|t| t["refresh_token"].as_str()).is_some_and(|s| !s.is_empty())
    }

    fn save(&self) {
        let _ = std::fs::create_dir_all(crate::data_dir());
        if let Some(t) = &self.tok {
            let _ = std::fs::write(token_path(), serde_json::to_string(t).unwrap_or_default());
        }
    }

    pub fn forget(&mut self) {
        self.tok = None;
        let _ = std::fs::remove_file(token_path());
    }

    fn req(url: &str, method: &str, headers: String, body: Vec<u8>, timeout_s: i32) -> (Result<u32, String>, Vec<u8>) {
        let r = Req { url: url.into(), method: method.into(), ua: "Python-urllib/3.12".into(), headers, body, timeout_ms: timeout_s * 1000 };
        let mut out = vec![];
        let s = web::fetch(&r, &mut |b, _| {
            out.extend_from_slice(b);
            true
        });
        (s, out)
    }

    fn post_token(&self, fields: &[(&str, String)]) -> R<Value> {
        let (s, b) = Self::req(&urls().1, "POST", "Content-Type: application/x-www-form-urlencoded".into(), urlencode(fields).into_bytes(), 15);
        match s {
            Ok(c) if c < 400 => serde_json::from_slice(&b).map_err(|_| "Couldn't reach Spotify. Check your internet/VPN.".into()),
            Ok(c) => {
                let msg = serde_json::from_slice::<Value>(&b).ok().and_then(|j| j["error_description"].as_str().map(String::from)).unwrap_or_default();
                Err(format!("Spotify refused the sign-in ({c} {msg}). Check the Client ID and Redirect URI."))
            }
            Err(_) => Err("Couldn't reach Spotify. Check your internet/VPN.".into()),
        }
    }

    /// the sign-in page's address, with its verifier and state
    pub fn auth_url(&self) -> (String, String, String) {
        let verifier: String = token_urlsafe(64).chars().take(100).collect();
        let state = token_urlsafe(12);
        let url = format!("{}?{}", urls().0, urlencode(&[("client_id", self.client_id.clone()), ("response_type", "code".into()),
            ("redirect_uri", REDIRECT.into()), ("code_challenge_method", "S256".into()), ("code_challenge", challenge(&verifier)),
            ("scope", SCOPES.into()), ("state", state.clone())]));
        (url, verifier, state)
    }

    /// login(): opens the browser (open(url)), catches the reply on 127.0.0.1:8765, swaps the code for a token.
    /// Blocking: run it in a thread.
    pub fn login(&mut self, open: &dyn Fn(&str), timeout_s: f64) -> R<String> {
        if self.client_id.is_empty() {
            return Err("Paste your Spotify Client ID in Settings first.".into());
        }
        let (url, verifier, state) = self.auth_url();
        let srv = std::net::TcpListener::bind(("127.0.0.1", PORT))
            .map_err(|_| format!("Port {PORT} is busy. Close other Zenko windows and try again."))?;
        let _ = srv.set_nonblocking(true);
        open(&url);
        let end = now_s() + timeout_s;
        let mut got: Option<(Option<String>, Option<String>, Option<String>)> = None;
        while got.is_none() && now_s() < end {
            match srv.accept() {
                Ok((mut s, _)) => got = Self::answer(&mut s, &state).or(got),
                Err(_) => std::thread::sleep(std::time::Duration::from_millis(100)),
            }
        }
        drop(srv);
        let (code, st, err) = got.unwrap_or((None, None, None));
        let Some(code) = code.filter(|_| st.as_deref() == Some(state.as_str())) else {
            return Err(format!("Spotify sign-in didn't finish ({}).", err.unwrap_or_else(|| "timed out".into())));
        };
        let mut t = self.post_token(&[("grant_type", "authorization_code".into()), ("code", code), ("redirect_uri", REDIRECT.into()),
                                      ("client_id", self.client_id.clone()), ("code_verifier", verifier)])?;
        t["expires_at"] = json!(now_s() + t["expires_in"].as_f64().unwrap_or(3600.0) - 60.0);
        self.tok = Some(t);
        self.save();
        Ok(url)
    }

    /// the handler's do_GET: (code, state, error) for /callback, None for anything else (404)
    fn answer(s: &mut std::net::TcpStream, state: &str) -> Option<(Option<String>, Option<String>, Option<String>)> {
        use std::io::{Read, Write};
        let _ = s.set_nonblocking(false);
        let _ = s.set_read_timeout(Some(std::time::Duration::from_secs(5)));
        let mut buf = vec![0u8; 8192];
        let n = s.read(&mut buf).unwrap_or(0);
        let req = String::from_utf8_lossy(&buf[..n]).into_owned();
        let path = req.split_whitespace().nth(1).unwrap_or("/").to_string();
        let (p, query) = path.split_once('?').unwrap_or((&path, ""));
        if !p.starts_with("/callback") {
            let _ = s.write_all(b"HTTP/1.0 404 Not Found\r\n\r\n");
            return None;
        }
        let mut q = std::collections::HashMap::new();
        for kv in query.split('&') {
            if let Some((k, v)) = kv.split_once('=') {
                let v = crate::web::unquote_plus(v);
                if !v.is_empty() {
                    q.entry(k.to_string()).or_insert(v);
                }
            }
        }
        let (code, st, err) = (q.get("code").cloned(), q.get("state").cloned(), q.get("error").cloned());
        let ok = code.is_some() && st.as_deref() == Some(state);
        let body = format!("<html><body style='font-family:Segoe UI;background:#26304a;color:white;text-align:center;padding-top:80px'>\
                            <h2>{}</h2><p>You can close this tab and go back to Zenko.</p></body></html>",
                           if ok { "Zenko is connected to Spotify!" } else { "Sign-in was cancelled." });
        let _ = s.write_all(format!("HTTP/1.0 200 OK\r\nContent-Type: text/html; charset=utf-8\r\n\r\n{body}").as_bytes());
        Some((code, st, err))
    }

    fn token(&mut self) -> R<String> {
        if !self.connected() {
            return Err("Spotify isn't connected yet (Settings > Spotify > Connect).".into());
        }
        let tok = self.tok.clone().unwrap();
        if now_s() >= tok["expires_at"].as_f64().unwrap_or(0.0) {
            let t = self.post_token(&[("grant_type", "refresh_token".into()), ("refresh_token", tok["refresh_token"].as_str().unwrap_or("").into()),
                                      ("client_id", self.client_id.clone())])?;
            let tk = self.tok.as_mut().unwrap();
            tk["access_token"] = t["access_token"].clone();
            if t["refresh_token"].as_str().is_some_and(|s| !s.is_empty()) {
                tk["refresh_token"] = t["refresh_token"].clone();
            }
            tk["expires_at"] = json!(now_s() + t["expires_in"].as_f64().unwrap_or(3600.0) - 60.0);
            self.save();
        }
        Ok(self.tok.as_ref().unwrap()["access_token"].as_str().unwrap_or("").into())
    }

    pub fn api(&mut self, method: &str, path: &str, params: &[(&str, String)], body: Option<&Value>) -> R<Value> {
        self.api_(method, path, params, body, true)
    }

    fn api_(&mut self, method: &str, path: &str, params: &[(&str, String)], body: Option<&Value>, retry: bool) -> R<Value> {
        let url = format!("{}{path}{}", urls().2, if params.is_empty() { String::new() } else { format!("?{}", urlencode(params)) });
        let data = body.map(|b| serde_json::to_vec(b).unwrap()).unwrap_or_default();
        let tok = self.token()?;
        let (s, raw) = Self::req(&url, method, format!("Authorization: Bearer {tok}\r\nContent-Type: application/json"), data, 12);
        match s {
            Ok(c) if c < 400 => {
                if String::from_utf8_lossy(&raw).trim().is_empty() {
                    return Ok(Value::Null);
                }
                serde_json::from_slice(&raw).map_err(|_| "Couldn't reach Spotify. Check your internet/VPN.".into())
            }
            Ok(code) => {
                let err = serde_json::from_slice::<Value>(&raw).ok().map(|j| j["error"].clone()).unwrap_or(Value::Null);
                let reason = err["reason"].as_str().unwrap_or("").to_string();
                let msg = err["message"].as_str().unwrap_or("").to_string();
                if code == 401 && retry {
                    if let Some(t) = self.tok.as_mut() {
                        t["expires_at"] = json!(0);
                    }
                    return self.api_(method, path, params, body, false);
                }
                let es = if err.is_object() { err.to_string().to_lowercase() } else { String::new() };
                if code == 404 && (reason.contains("NO_ACTIVE_DEVICE") || es.contains("device")) {
                    return Err("NO_DEVICE".into());
                }
                if code == 403 {
                    return Err(format!("SPOTIFY_403:{}", if msg.is_empty() { reason } else { msg }));
                }
                if code == 429 {
                    return Err("Spotify asked me to slow down. Try again in a minute.".into());
                }
                Err(format!("Spotify error {code} {msg}"))
            }
            Err(_) => Err("Couldn't reach Spotify. Check your internet/VPN.".into()),
        }
    }

    fn devices(&mut self) -> R<Vec<Value>> {
        Ok(self.api("GET", "/me/player/devices", &[], None)?["devices"].as_array().cloned().unwrap_or_default())
    }

    /// call(): like api() but wakes up a device (this PC's Spotify app) when nothing is open anywhere
    pub fn call(&mut self, method: &str, path: &str, params: &[(&str, String)], body: Option<&Value>) -> R<Value> {
        match self.api(method, path, params, body) {
            Err(e) if e == "NO_DEVICE" => {}
            r => return r,
        }
        let mut devs = self.devices()?;
        if devs.is_empty() {
            if let Some(o) = self.open_app {
                o();
                for _ in 0..10 {
                    (self.sleep)(1.5);
                    devs = self.devices()?;
                    if !devs.is_empty() {
                        break;
                    }
                }
            }
        }
        if devs.is_empty() {
            return Err("No Spotify device is open. Open the Spotify app on this PC (or your phone) once, then try again.".into());
        }
        let mut ds = devs.clone();
        ds.sort_by_key(|d| (d["type"].as_str() != Some("Computer"), d["is_active"].as_bool() != Some(true)));
        let id = ds[0]["id"].clone();
        self.api("PUT", "/me/player", &[], Some(&json!({"device_ids": [id], "play": false})))?;
        (self.sleep)(0.6);
        self.api(method, path, params, body)
    }

    pub fn now(&mut self) -> R<Option<Now>> {
        let d = self.api("GET", "/me/player", &[], None)?;
        if d.is_null() || d["item"].is_null() {
            return Ok(None);
        }
        let it = &d["item"];
        let people: Vec<Value> = it["artists"].as_array().cloned().filter(|a| !a.is_empty())
            .unwrap_or_else(|| if it["show"].is_object() { vec![it["show"].clone()] } else { vec![] });
        let artist = people.iter().map(|a| a["name"].as_str().unwrap_or("").to_string()).collect::<Vec<_>>().join(", ");
        Ok(Some(Now { title: it["name"].as_str().unwrap_or("").into(), artist, playing: d["is_playing"].as_bool().unwrap_or(false),
                      uri: it["uri"].as_str().unwrap_or("").into(), volume: d["device"]["volume_percent"].as_i64(),
                      shuffle: d["shuffle_state"].as_bool(), progress: d["progress_ms"].as_i64().unwrap_or(0),
                      duration: it["duration_ms"].as_i64().unwrap_or(1) }))
    }

    pub fn play_query(&mut self, q: &str) -> R<String> {
        let q = q.trim();
        let want_pl = regex::RegexBuilder::new(r"\bplaylist\b").case_insensitive(true).build().unwrap().is_match(q);
        let q2 = regex::RegexBuilder::new(r"\b(my|the|playlist|some|music|songs?)\b").case_insensitive(true).build().unwrap()
            .replace_all(q, " ").trim().to_string();
        if !q2.is_empty() {
            let pls = self.api("GET", "/me/playlists", &[("limit", "50".into())], None).ok()
                .and_then(|v| v["items"].as_array().cloned()).unwrap_or_default();
            let (mut best, mut score) = (None::<Value>, 0.0);
            for p in pls.iter().filter(|p| !p.is_null()) {
                let name = p["name"].as_str().unwrap_or("").to_lowercase();
                let mut s = ratio(&q2.to_lowercase(), &name);
                if name.contains(&q2.to_lowercase()) {
                    s += 0.5;
                }
                if s > score {
                    (best, score) = (Some(p.clone()), s);
                }
            }
            if let Some(b) = best.filter(|_| score >= 0.75 || (want_pl && score >= 0.45)) {
                self.call("PUT", "/me/player/play", &[], Some(&json!({"context_uri": b["uri"]})))?;
                return Ok(format!("Playing your playlist \u{201c}{}\u{201d}.", b["name"].as_str().unwrap_or("None")));
            }
        }
        if q2.is_empty() {
            self.call("PUT", "/me/player/play", &[], None)?;
            return Ok("Music on.".into());
        }
        let kind = if want_pl { "playlist" } else { "track" };
        let res = self.api("GET", "/search", &[("q", q2.clone()), ("type", kind.into()), ("limit", "5".into())], None)?;
        let items: Vec<Value> = res[format!("{kind}s")]["items"].as_array().cloned().unwrap_or_default().into_iter().filter(|i| !i.is_null()).collect();
        let Some(it) = items.first() else {
            return Ok(format!("I couldn't find \u{201c}{q2}\u{201d} on Spotify."));
        };
        if kind == "playlist" {
            self.call("PUT", "/me/player/play", &[], Some(&json!({"context_uri": it["uri"]})))?;
            return Ok(format!("Playing the playlist \u{201c}{}\u{201d}.", it["name"].as_str().unwrap_or("None")));
        }
        self.call("PUT", "/me/player/play", &[], Some(&json!({"uris": [it["uri"]]})))?;
        let by = it["artists"].as_array().map(|a| a.iter().map(|x| x["name"].as_str().unwrap_or("").to_string()).collect::<Vec<_>>().join(", ")).unwrap_or_default();
        Ok(format!("Playing \u{201c}{}\u{201d} by {by}.", it["name"].as_str().unwrap_or("None")))
    }

    /// do(action, arg): a sentence, or the error to say
    pub fn do_(&mut self, action: &str, arg: Option<&str>) -> R<String> {
        match self.do_inner(action, arg) {
            Err(e) if e.starts_with("SPOTIFY_403") => {
                let detail = e.split_once(':').map(|x| x.1.trim().to_string()).unwrap_or_default();
                let n = self.now().ok().flatten();
                if action == "pause" && n.as_ref().is_none_or(|n| !n.playing) {
                    return Ok("The music is already paused.".into());
                }
                if action == "play" && arg.is_none_or(|a| a.is_empty()) {
                    if let Some(n) = n.as_ref().filter(|n| n.playing) {
                        return Ok(format!("It's already playing: \u{201c}{}\u{201d}.", n.title));
                    }
                }
                Err(format!("Spotify wouldn't do that right now{}. If the music is on another device (phone, speaker), control it \
                             there or switch it to this PC.", if detail.is_empty() { String::new() } else { format!(" ({detail})") }))
            }
            r => r,
        }
    }

    fn do_inner(&mut self, action: &str, arg: Option<&str>) -> R<String> {
        let a = arg.unwrap_or("");
        match action {
            "play" => {
                if !a.is_empty() {
                    return self.play_query(a);
                }
                let r = self.call("PUT", "/me/player/play", &[], None)?;
                Ok(if r.is_null() { "Music on.".into() } else { r.to_string() })
            }
            "pause" => self.call("PUT", "/me/player/pause", &[], None).map(|_| "Music paused.".into()),
            "next" => self.call("POST", "/me/player/next", &[], None).map(|_| "Skipped.".into()),
            "previous" => self.call("POST", "/me/player/previous", &[], None).map(|_| "Back one song.".into()),
            "volume" => {
                let v = a.trim().parse::<f64>().map_err(|_| format!("invalid literal for int() with base 10: '{a}'"))? as i64;
                let v = v.clamp(0, 100);
                self.call("PUT", "/me/player/volume", &[("volume_percent", v.to_string())], None)?;
                Ok(format!("Volume {v}%."))
            }
            "shuffle" => {
                let on = matches!(a, "true" | "True" | "1" | "on");
                self.call("PUT", "/me/player/shuffle", &[("state", if on { "true" } else { "false" }.into())], None)?;
                Ok(format!("Shuffle {}.", if on { "on" } else { "off" }))
            }
            "like" => {
                let Some(n) = self.now()? else { return Ok("Nothing is playing.".into()) };
                self.api("PUT", "/me/library", &[("uris", n.uri.clone())], None)?;
                Ok(format!("Saved \u{201c}{}\u{201d} to your library.", n.title))
            }
            "now" => Ok(match self.now()? {
                None => "Nothing is playing on Spotify right now.".into(),
                Some(n) => format!("{} \u{201c}{}\u{201d} by {}.", if n.playing { "Now playing:" } else { "Paused:" }, n.title, n.artist),
            }),
            _ => Ok("I don't know that music command.".into()),
        }
    }

    /// play_preset(p)'s job
    pub fn play_preset(&mut self, p: &Value) -> R<String> {
        let val = p["value"].as_str().unwrap_or("").trim().to_string();
        let kind = p["kind"].as_str().unwrap_or("playlist");
        if let Some(uri) = link_to_uri(&val) {
            if uri.starts_with("spotify:track:") {
                self.call("PUT", "/me/player/play", &[], Some(&json!({"uris": [uri]})))?;
            } else {
                self.call("PUT", "/me/player/play", &[], Some(&json!({"context_uri": uri})))?;
            }
            return Ok(format!("Playing your \u{201c}{}\u{201d} preset.", p["name"].as_str().unwrap_or("")));
        }
        if kind == "playlist" {
            return self.play_query(&format!("{val} playlist"));
        }
        if kind == "album" || kind == "artist" {
            let res = self.api("GET", "/search", &[("q", val.clone()), ("type", kind.into()), ("limit", "1".into())], None)?;
            let its: Vec<Value> = res[format!("{kind}s")]["items"].as_array().cloned().unwrap_or_default().into_iter().filter(|i| !i.is_null()).collect();
            let Some(it) = its.first() else {
                return Ok(format!("I couldn't find \u{201c}{val}\u{201d} on Spotify."));
            };
            self.call("PUT", "/me/player/play", &[], Some(&json!({"context_uri": it["uri"]})))?;
            return Ok(format!("Playing {kind} \u{201c}{}\u{201d}.", it["name"].as_str().unwrap_or("None")));
        }
        self.play_query(&val)
    }
}
