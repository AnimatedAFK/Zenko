//! zenko.py's voice, minus the speakers and the microphone themselves (those are Windows calls in the program):
//! VOICEVOX over HTTP (speakers, audio_query + synthesis, her exclamation clips, the per-sentence cache, the engine
//! finder), her Japanese voice (jp_clean, the fox-sound kana, jp_fallback, jp_translate with its cache), the
//! speakable text (vv_say_text), .wav joining, and the offline ears' pieces (the voice detector, mic_peak,
//! pcm_to_wav, stt_paths, whisper's output cleaning, stt_install with its zip).

use crate::web::{Req, download, fetch, http_json, q};
use regex::Regex;
use serde_json::{Map, Value, json};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

macro_rules! rx {
    ($p:expr) => {{
        static R: OnceLock<Regex> = OnceLock::new();
        R.get_or_init(|| Regex::new($p).unwrap())
    }};
}

/// data/voice.json (tools/make_golden_voice.py)
pub fn data() -> &'static Value {
    static D: OnceLock<Value> = OnceLock::new();
    D.get_or_init(|| serde_json::from_str(include_str!("../data/voice.json")).unwrap())
}

fn tics() -> &'static Vec<(Regex, Regex, String)> {
    static T: OnceLock<Vec<(Regex, Regex, String)>> = OnceLock::new();
    T.get_or_init(|| data()["JP_TICS"].as_array().unwrap().iter().map(|p| {
        let pat = p[0].as_str().unwrap();
        (Regex::new(pat).unwrap(), Regex::new(&format!("(?i){pat}")).unwrap(), p[1].as_str().unwrap().to_string())
    }).collect())
}

/// the characters zenko.py keeps: below U+2600, or CJK U+3000..U+9FFF
fn keep_char(c: char) -> bool {
    (c as u32) < 0x2600 || (0x3000..=0x9fff).contains(&(c as u32))
}

/// str.strip(chars)
fn strip_set(s: &str, set: &str) -> String {
    s.trim_matches(|c| set.contains(c)).to_string()
}

/// re.split(r"(?<=[set])\s+", t)
fn split_after(t: &str, set: &str) -> Vec<String> {
    let mut out = vec![];
    let mut cur = String::new();
    let cs: Vec<char> = t.chars().collect();
    let mut i = 0;
    while i < cs.len() {
        let c = cs[i];
        if c.is_whitespace() && i > 0 && set.contains(cs[i - 1]) {
            let mut j = i;
            while j < cs.len() && cs[j].is_whitespace() {
                j += 1;
            }
            out.push(std::mem::take(&mut cur));
            i = j;
            continue;
        }
        cur.push(c);
        i += 1;
    }
    out.push(cur);
    out
}

fn py_len(s: &str) -> usize {
    s.chars().count()
}

/// jp_clean(text): the part of a bubble worth saying
pub fn jp_clean(text: &str) -> String {
    static ACT: OnceLock<fancy_regex::Regex> = OnceLock::new();
    let act = ACT.get_or_init(|| fancy_regex::Regex::new(r"(?<!\*)\*[^*\n]{1,60}\*(?!\*)").unwrap());
    let t = act.replace_all(text, " ").into_owned();
    let t = crate::mdtext::md_plain(&t);
    let t = rx!(r"https?://\S+|\$[^$]*\$").replace_all(&t, " ");
    let t: String = t.chars().filter(|&c| keep_char(c)).collect();
    let t = rx!(r"[~_#>|`]+").replace_all(&t.replace('\n', " "), " ").into_owned();
    let t = strip_set(rx!(r"\s+").replace_all(&t, " ").as_ref(), " .,-");
    let mut out = String::new();
    for p in split_after(&t, ".!?") {
        if !out.is_empty() && py_len(&out) + py_len(&p) > 150 {
            break;
        }
        out = format!("{out} {p}").trim().to_string();
    }
    out.chars().take(220).collect()
}

/// jp_tics_only(t): a line made only of her fox sounds, in kana (else "")
pub fn jp_tics_only(t: &str) -> String {
    let low = t.to_lowercase();
    let mut rest = low.clone();
    for (re, _, _) in tics() {
        rest = re.replace_all(&rest, " ").into_owned();
    }
    if !rx!(r"[\s!?.,~\-…]+").replace_all(&rest, "").is_empty() {
        return String::new();
    }
    let mut out = low;
    for (re, _, kana) in tics() {
        out = re.replace_all(&out, kana.as_str()).into_owned();
    }
    rx!(r"\s+").replace_all(&out, "").replace('~', "\u{301c}").replace('!', "\u{ff01}").replace('?', "\u{ff1f}")
}

/// jp_fallback(t): a short Japanese reaction; `pick(choices)` stands in for random.choice
pub fn jp_fallback(t: &str, pick: &mut dyn FnMut(&[&'static str]) -> &'static str) -> String {
    let low = t.to_lowercase();
    for (re, _, kana) in tics().iter().take(24) {
        if re.is_match(&low) {
            return format!("{kana}\u{ff01}");
        }
    }
    if rx!(r"\b(well done|great|amazing|proud|yay|congrat|nice|you did it|finished|done)\b").is_match(&low) {
        return pick(&["やったぁ！すごいね！", "えへへ、えらいえらい！"]).into();
    }
    if rx!(r"\b(sorry|sad|miss|tired|rough|hungry)\b").is_match(&low) {
        return pick(&["うぅ…", "だいじょうぶ？"]).into();
    }
    if rx!(r"\b(focus|study|back to work|distract|close)\b").is_match(&low) {
        return pick(&["集中しよう！", "がんばって！", "もう〜！勉強しよう？"]).into();
    }
    if t.contains('?') {
        return "ねぇねぇ？".into();
    }
    pick(&["こんこん！", "ふふっ", "はーい！"]).into()
}

/// vv_say_text(text): the sentences to say (no Markdown, maths, links or emoji; about 260 characters)
pub fn vv_say_text(text: &str) -> Vec<String> {
    let t = crate::mdtext::md_plain(text);
    let t = rx!(r"\$[^$]*\$|\\\(.*?\\\)|https?://\S+").replace_all(&t, " ");
    let t = rx!(r"[~*_`#>|\[\]{}]").replace_all(&t, " ");
    let t: String = t.chars().filter(|&c| keep_char(c)).collect();
    let t = rx!(r"\s+").replace_all(&t, " ").trim().to_string();
    let mut out: Vec<String> = vec![];
    let mut n = 0;
    for p in split_after(&t, ".!?\u{3002}\u{ff01}\u{ff1f}") {
        if p.is_empty() {
            continue;
        }
        if n + py_len(&p) > 260 && !out.is_empty() {
            break;
        }
        n += py_len(&p);
        out.push(p);
    }
    out
}

// ---- the shared state zenko.py keeps in JPV / _JPS / _VV_SAY -----------------------------------------------------

#[derive(Default, Debug, Clone)]
pub struct VoiceState {
    /// JPV["said"]: when she last started a real line (monotonic seconds); her clips stay quiet just after
    pub said: f64,
    /// JPV["until"]: interface sounds are quiet until then
    pub until: f64,
    pub muted: bool,
    /// _JPS["alive"]: VOICEVOX answered the last check
    pub alive: bool,
    /// _JPS["net_bad"]: the online translator failed; don't try again until then
    pub net_bad: f64,
    /// _JPS["gen"] / _VV_SAY["gen"]: a newer line (or hush) cancels the older one
    pub jp_gen: u64,
    pub vv_gen: u64,
}

pub fn state() -> std::sync::MutexGuard<'static, VoiceState> {
    static S: Mutex<VoiceState> = Mutex::new(VoiceState { said: 0.0, until: 0.0, muted: false, alive: false, net_bad: 0.0,
                                                          jp_gen: 0, vv_gen: 0 });
    S.lock().unwrap_or_else(|e| e.into_inner())
}

/// time.monotonic()
pub fn mono() -> f64 {
    static T0: OnceLock<std::time::Instant> = OnceLock::new();
    T0.get_or_init(std::time::Instant::now).elapsed().as_secs_f64() + 1000.0
}

// ---- VOICEVOX ------------------------------------------------------------------------------------------------------

pub fn vv_url() -> String {
    std::env::var("FOX_VOICEVOX").unwrap_or_else(|_| "http://127.0.0.1:50021".into())
}

fn local(url: &str, method: &str, body: Vec<u8>, ctype: &str, timeout_s: f64) -> Result<Vec<u8>, String> {
    let r = Req { url: url.into(), method: method.into(), ua: "Python-urllib/3.12".into(),
                  headers: if ctype.is_empty() { String::new() } else { format!("Content-Type: {ctype}") }, body,
                  timeout_ms: (timeout_s * 1000.0) as i32 };
    let mut out = vec![];
    match fetch(&r, &mut |b, _| {
        out.extend_from_slice(b);
        true
    }) {
        Ok(s) if s < 400 => Ok(out),
        Ok(s) => Err(format!("HTTP Error {s}")),
        Err(e) => Err(e),
    }
}

/// voicevox_speakers(timeout): [(id, "Character (style)")], or None when VOICEVOX isn't running
pub fn speakers(timeout_s: f64) -> Option<Vec<(i64, String)>> {
    let b = local(&format!("{}/speakers", vv_url()), "", vec![], "", timeout_s).ok()?;
    let j: Value = serde_json::from_slice(&b).ok()?;
    let mut out = vec![];
    for sp in j.as_array()? {
        for st in sp["styles"].as_array().into_iter().flatten() {
            out.push((st["id"].as_i64().unwrap_or(0),
                      format!("{} ({})", sp["name"].as_str().unwrap_or("?"), st["name"].as_str().unwrap_or(""))));
        }
    }
    Some(out)
}

/// voicevox_wav(text, speaker, pitch, speed)
pub fn vv_wav(text: &str, speaker: i64, pitch: f64, speed: f64) -> Result<Vec<u8>, String> {
    let b = local(&format!("{}/audio_query?text={}&speaker={speaker}", vv_url(), q(text)), "POST", vec![], "", 20.0)?;
    let mut aq: Value = serde_json::from_slice(&b).map_err(|e| e.to_string())?;
    aq["pitchScale"] = json!(pitch);
    aq["speedScale"] = json!(speed);
    aq["intonationScale"] = json!(1.25);
    local(&format!("{}/synthesis?speaker={speaker}", vv_url()), "POST", serde_json::to_vec(&aq).unwrap(), "application/json", 40.0)
}

pub fn vv_alive(timeout_s: f64) -> bool {
    local(&format!("{}/version", vv_url()), "", vec![], "", timeout_s).is_ok()
}

pub fn sound_dir() -> PathBuf {
    crate::data_dir().join("sounds")
}

pub fn clip_path(key: &str, speaker: i64) -> PathBuf {
    sound_dir().join(format!("vv{speaker}_{key}.wav"))
}

/// voicevox_make_clips(speaker): every exclamation in VOX_JP made once; how many there are now
pub fn make_clips(speaker: i64) -> Result<usize, String> {
    let _ = std::fs::create_dir_all(sound_dir());
    let mut made = 0;
    for (key, jp) in data()["VOX_JP"].as_object().unwrap() {
        let p = clip_path(key, speaker);
        if p.exists() {
            made += 1;
            continue;
        }
        let d = vv_wav(jp.as_str().unwrap_or(""), speaker, 0.04, 1.05)?;
        let part = PathBuf::from(format!("{}.part", p.display()));
        std::fs::write(&part, d).map_err(|e| e.to_string())?;
        std::fs::rename(&part, &p).map_err(|e| e.to_string())?;
        made += 1;
    }
    Ok(made)
}

/// _ext_dir(name): a folder in the data folder, made if needed
pub fn ext_dir(name: &str) -> PathBuf {
    let p = crate::data_dir().join(name);
    let _ = std::fs::create_dir_all(&p);
    p
}

/// _vv_cached_wav(sentence, speaker): from the disk cache, or made and kept (at most 500 files)
pub fn cached_wav(p: &str, speaker: i64) -> Result<PathBuf, String> {
    let d = ext_dir("speech_cache");
    let mut h = Sha1::new();
    h.update(format!("{speaker}|{p}|v1").as_bytes());
    let key: String = h.hex().chars().take(20).collect();
    let path = d.join(format!("{key}.wav"));
    if std::fs::metadata(&path).map(|m| m.len() > 1000).unwrap_or(false) {
        if let Ok(f) = std::fs::File::options().append(true).open(&path) {
            let _ = f.set_modified(std::time::SystemTime::now());
        }
        return Ok(path);
    }
    let data = vv_wav(p, speaker, 0.04, 1.05)?;
    let part = PathBuf::from(format!("{}.part", path.display()));
    std::fs::write(&part, data).map_err(|e| e.to_string())?;
    std::fs::rename(&part, &path).map_err(|e| e.to_string())?;
    if let Ok(rd) = std::fs::read_dir(&d) {
        let mut names: Vec<(std::time::SystemTime, PathBuf)> = rd.flatten()
            .filter(|e| e.file_name().to_string_lossy().ends_with(".wav"))
            .map(|e| (e.metadata().and_then(|m| m.modified()).unwrap_or(std::time::UNIX_EPOCH), e.path())).collect();
        if names.len() > 500 {
            names.sort();
            let n = names.len() - 500 + 50;
            for (_, f) in names.into_iter().take(n) {
                let _ = std::fs::remove_file(f);
            }
        }
    }
    Ok(path)
}

/// a .wav's (format chunk, sample frames, frame rate, frame size)
fn wav_parts(b: &[u8]) -> Option<(Vec<u8>, Vec<u8>, u32, u32)> {
    if b.len() < 12 || &b[..4] != b"RIFF" || &b[8..12] != b"WAVE" {
        return None;
    }
    let (mut fmt, mut data) = (None, None);
    let mut i = 12;
    while i + 8 <= b.len() {
        let id = &b[i..i + 4];
        let n = u32::from_le_bytes(b[i + 4..i + 8].try_into().ok()?) as usize;
        let body = &b[i + 8..(i + 8 + n).min(b.len())];
        if id == b"fmt " {
            fmt = Some(body.to_vec());
        } else if id == b"data" {
            data = Some(body.to_vec());
        }
        i += 8 + n + (n & 1);
    }
    let fmt = fmt?;
    if fmt.len() < 16 {
        return None;
    }
    let rate = u32::from_le_bytes(fmt[4..8].try_into().ok()?);
    let align = u16::from_le_bytes(fmt[12..14].try_into().ok()?) as u32;
    let data = data?;
    let frames = data.len() as u32 / align.max(1);
    Some((fmt, data[..(frames * align.max(1)) as usize].to_vec(), rate, align))
}

/// the whole line as one .wav (wave.open: the first file's params, every file's frames) and its seconds
pub fn join_wavs(paths: &[PathBuf], out: &Path) -> Result<f64, String> {
    let mut fmt = None;
    let mut pcm = vec![];
    let mut rate = 24000;
    let mut align = 2;
    for (k, p) in paths.iter().enumerate() {
        let b = std::fs::read(p).map_err(|e| e.to_string())?;
        let (f, d, r, a) = wav_parts(&b).ok_or("not a wav")?;
        if k == 0 {
            (fmt, rate, align) = (Some(f), r, a);
        }
        pcm.extend(d);
    }
    let fmt = fmt.ok_or("no files")?;
    let fmt16 = &fmt[..16];
    let mut o = b"RIFF".to_vec();
    o.extend((36 + pcm.len() as u32).to_le_bytes());
    o.extend(b"WAVEfmt ");
    o.extend(16u32.to_le_bytes());
    o.extend(fmt16);
    o.extend(b"data");
    o.extend((pcm.len() as u32).to_le_bytes());
    o.extend(&pcm);
    std::fs::write(out, o).map_err(|e| e.to_string())?;
    Ok((pcm.len() as u32 / align.max(1)) as f64 / if rate == 0 { 24000.0 } else { rate as f64 })
}

/// a .wav's length in seconds
pub fn wav_secs(p: &Path) -> Option<f64> {
    let b = std::fs::read(p).ok()?;
    let (_, d, r, a) = wav_parts(&b)?;
    Some((d.len() as u32 / a.max(1)) as f64 / if r == 0 { 24000.0 } else { r as f64 })
}

/// vv_speak's thread, up to the sound: every sentence made (or cached) first. Some((file to play, seconds)), or
/// None when VOICEVOX isn't there (done(-1)) or a newer line took over (`gen` changed: nothing at all)
pub enum Ready {
    Play(PathBuf, f64),
    Down,
    Cancelled,
}

pub fn vv_prepare(parts: &[String], speaker: i64, want_gen: u64) -> Ready {
    let mut paths = vec![];
    for p in parts {
        if state().vv_gen != want_gen {
            return Ready::Cancelled;
        }
        match cached_wav(p, speaker) {
            Ok(x) => paths.push(x),
            Err(_) => {
                if paths.is_empty() {
                    return Ready::Down;
                }
                break;
            }
        }
    }
    if state().vv_gen != want_gen {
        return Ready::Cancelled;
    }
    let mut play = paths[0].clone();
    let res = (|| -> Result<f64, String> {
        if paths.len() > 1 {
            play = ext_dir("speech_tmp").join(format!("reply{}.wav", want_gen % 2));
            return join_wavs(&paths, &play);
        }
        wav_secs(&play).ok_or_else(|| "bad wav".into())
    })();
    match res {
        Ok(s) => Ready::Play(play, s),
        Err(_) => Ready::Play(paths[0].clone(), py_len(&parts[0]) as f64 / 12.0),
    }
}

// ---- Japanese lines ------------------------------------------------------------------------------------------------

fn jp_file() -> PathBuf {
    crate::data_dir().join("jp_lines.json")
}

fn jp_cache() -> std::sync::MutexGuard<'static, Option<Map<String, Value>>> {
    static C: Mutex<Option<Map<String, Value>>> = Mutex::new(None);
    let mut g = C.lock().unwrap_or_else(|e| e.into_inner());
    if g.is_none() {
        *g = Some(std::fs::read_to_string(jp_file()).ok().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default());
    }
    g
}

fn jp_save(c: &mut Map<String, Value>) {
    if c.len() > 4000 {
        let drop: Vec<String> = c.keys().take(c.len() - 3000).cloned().collect();
        for k in drop {
            c.remove(&k);
        }
    }
    let tmp = PathBuf::from(format!("{}.part", jp_file().display()));
    if std::fs::write(&tmp, serde_json::to_string(&Value::Object(c.clone())).unwrap_or_default()).is_ok() {
        let _ = std::fs::rename(&tmp, jp_file());
    }
}

/// jp_translate(text, allow_net): the cache, the fox-sound kana, the free online translator, then a fitting short
/// reaction. (zenko.py also asks a local Ollama model before the reaction: that brain is stage 5b.)
pub fn jp_translate(text: &str, allow_net: bool, pick: &mut dyn FnMut(&[&'static str]) -> &'static str) -> String {
    let t = jp_clean(text);
    if t.is_empty() {
        return String::new();
    }
    let key = t.to_lowercase();
    if let Some(v) = jp_cache().as_ref().and_then(|c| c.get(&key)).and_then(|v| v.as_str()) {
        return v.to_string();
    }
    let tic = jp_tics_only(&t);
    if !tic.is_empty() {
        return tic;
    }
    let mut pre = t.clone();
    for (_, rei, kana) in tics() {
        pre = rei.replace_all(&pre, kana.as_str()).into_owned();
    }
    let mut out = String::new();
    let bad = state().net_bad;
    if allow_net && mono() > bad {
        match crate::web::translate_text(&pre, "ja", "en") {
            Ok(r) => out = r.0,
            Err(_) => state().net_bad = mono() + 600.0,
        }
    }
    if !out.is_empty() {
        let o2 = rx!(r"[A-Za-z]{3,}").replace_all(&out, "").trim().to_string();
        let out = if o2.is_empty() { out } else { o2 };
        let mut g = jp_cache();
        let c = g.as_mut().unwrap();
        c.insert(key, json!(out));
        jp_save(c);
        return out;
    }
    jp_fallback(&t, pick)
}

// ---- the VOICEVOX engine finder ------------------------------------------------------------------------------------

/// _vv_walk(root, depth): the run.exe under root, preferring paths with "engine" and then shorter ones
fn vv_walk(root: &Path, depth: usize) -> Option<PathBuf> {
    let mut found = vec![];
    let mut stack = vec![(root.to_path_buf(), 0usize)];
    while let Some((d, dep)) = stack.pop() {
        let Ok(rd) = std::fs::read_dir(&d) else { continue };
        let mut ents: Vec<_> = rd.flatten().collect();
        ents.sort_by_key(|e| e.file_name());
        for e in ents {
            let p = e.path();
            if p.is_dir() {
                if dep < depth {
                    stack.push((p, dep + 1));
                }
            } else if e.file_name().to_string_lossy().to_lowercase() == "run.exe" {
                found.push(p);
            }
        }
    }
    found.sort_by_key(|x| (!x.to_string_lossy().to_lowercase().contains("engine"), x.to_string_lossy().chars().count()));
    found.into_iter().next()
}

/// where voicevox_find_engine looks: home, %LOCALAPPDATA%, Program Files and Zenko's own folder
pub struct Places {
    pub home: PathBuf,
    pub local_app_data: PathBuf,
    pub program_files: PathBuf,
    pub app_dir: PathBuf,
}

/// voicevox_find_engine(saved)
pub fn find_engine(saved: &str, pl: &Places) -> String {
    if !saved.is_empty() && Path::new(saved).is_file() {
        return saved.to_string();
    }
    let (home, la, app) = (&pl.home, &pl.local_app_data, &pl.app_dir);
    let mut roots = vec![home.join("voicevox_engine"), app.join("voicevox_engine"), la.join("Programs").join("VOICEVOX").join("vv-engine"),
                         la.join("Programs").join("VOICEVOX"), pl.program_files.join("VOICEVOX")];
    let parent = app.parent().map(Path::to_path_buf).unwrap_or_else(|| app.clone());
    for base in [home.clone(), home.join("Downloads"), home.join("Desktop"), home.join("Documents"), app.clone(), parent] {
        if let Ok(rd) = std::fs::read_dir(&base) {
            let mut ns: Vec<String> = rd.flatten().map(|e| e.file_name().to_string_lossy().into_owned()).collect();
            ns.sort();
            for n in ns {
                if n.to_lowercase().contains("voicevox") && base.join(&n).is_dir() {
                    roots.push(base.join(&n));
                }
            }
        }
    }
    let mut seen: Vec<String> = vec![];
    for r in roots {
        let k = r.to_string_lossy().to_lowercase();
        if seen.contains(&k) || !r.is_dir() {
            continue;
        }
        seen.push(k);
        if let Some(h) = vv_walk(&r, 4) {
            return h.to_string_lossy().into_owned();
        }
    }
    String::new()
}

// ---- the offline ears ----------------------------------------------------------------------------------------------

/// MicRecorder's voice detector state
#[derive(Default, Debug, Clone, PartialEq)]
pub struct Vad {
    pub n: i64,
    pub floor: f64,
    pub loud: i64,
    pub quiet: i64,
    pub heard: bool,
}

fn samples(pcm: &[u8]) -> Vec<i16> {
    pcm.chunks_exact(2).map(|c| i16::from_le_bytes([c[0], c[1]])).collect()
}

impl Vad {
    /// _vad(data): true when the recording should end by itself (auto_stop)
    pub fn feed(&mut self, data: &[u8], auto_stop: bool, max_secs: f64) -> bool {
        let a = samples(data);
        if a.is_empty() {
            return false;
        }
        let sub: Vec<f64> = a.iter().step_by(4).map(|&v| v as f64).collect();
        let rms = (sub.iter().map(|v| v * v).sum::<f64>() / (sub.len().max(1) as f64)).sqrt();
        self.n += 1;
        if self.n <= 3 {
            self.floor = (self.floor * (self.n - 1) as f64 + rms) / self.n as f64;
            return false;
        }
        let loud = rms > (self.floor * 2.6).max(380.0);
        if loud {
            self.loud += 1;
            self.quiet = 0;
            if self.loud >= 2 {
                self.heard = true;
            }
        } else {
            self.quiet += 1;
            self.floor = self.floor * 0.97 + rms * 0.03;
        }
        if !auto_stop {
            return false;
        }
        let secs = self.n as f64 * 0.1;
        if self.heard && self.quiet >= 12 {
            return true;
        }
        if !self.heard && secs >= 8.0 {
            return true;
        }
        secs >= max_secs
    }
}

/// mic_peak(pcm): the loudest of every 7th sample
pub fn mic_peak(pcm: &[u8]) -> i32 {
    samples(pcm).iter().step_by(7).map(|&v| (v as i32).abs()).max().unwrap_or(0)
}

/// pcm_to_wav(pcm, path, rate)
pub fn pcm_to_wav(pcm: &[u8], path: &Path, rate: u32) -> std::io::Result<()> {
    let mut o = b"RIFF".to_vec();
    o.extend((36 + pcm.len() as u32).to_le_bytes());
    o.extend(b"WAVEfmt ");
    for v in [16u32] {
        o.extend(v.to_le_bytes());
    }
    o.extend(1u16.to_le_bytes());
    o.extend(1u16.to_le_bytes());
    o.extend(rate.to_le_bytes());
    o.extend((rate * 2).to_le_bytes());
    o.extend(2u16.to_le_bytes());
    o.extend(16u16.to_le_bytes());
    o.extend(b"data");
    o.extend((pcm.len() as u32).to_le_bytes());
    o.extend(pcm);
    std::fs::write(path, o)
}

pub fn stt_dir() -> PathBuf {
    crate::data_dir().join("speech")
}

/// STT_MODELS[key]: (file, MB, label)
pub fn stt_model(key: &str) -> (String, i64, String) {
    let m = &data()["STT_MODELS"];
    let v = if m[key].is_array() { &m[key] } else { &m["fast"] };
    (v[0].as_str().unwrap().into(), v[1].as_i64().unwrap(), v[2].as_str().unwrap().into())
}

/// stt_paths(model_key, extra_dir): (whisper-cli, model .bin)
pub fn stt_paths(model_key: &str, extra_dir: &str) -> (Option<PathBuf>, Option<PathBuf>) {
    let want = stt_model(model_key).0;
    let mut cli: Option<PathBuf> = None;
    let mut models: Vec<PathBuf> = vec![];
    let sd = stt_dir();
    let roots: Vec<PathBuf> = [PathBuf::from(extra_dir), sd].into_iter().filter(|d| !d.as_os_str().is_empty() && d.is_dir()).collect();
    for root in roots {
        // os.walk, top down: a folder's files, then its folders (in name order), at most 3 levels below
        let mut stack = vec![(root.clone(), 0usize)];
        while let Some((dp, dep)) = stack.pop() {
            if dep > 3 {
                continue;
            }
            let Ok(rd) = std::fs::read_dir(&dp) else { continue };
            let mut files = vec![];
            let mut dirs = vec![];
            for e in rd.flatten() {
                if e.path().is_dir() { dirs.push(e.path()) } else { files.push(e.file_name().to_string_lossy().into_owned()) }
            }
            files.sort();
            dirs.sort();
            let low: Vec<String> = files.iter().map(|f| f.to_lowercase()).collect();
            if cli.is_none() {
                for name in ["whisper-cli.exe", "whisper-cli", "main.exe"] {
                    if let Some(i) = low.iter().position(|f| f == name) {
                        if name != "main.exe" || low.iter().any(|f| f == "whisper.dll") {
                            cli = Some(dp.join(&files[i]));
                            break;
                        }
                    }
                }
            }
            for f in &files {
                let l = f.to_lowercase();
                if l.starts_with("ggml-") && l.ends_with(".bin") {
                    models.push(dp.join(f));
                }
            }
            for d in dirs.into_iter().rev() {
                stack.push((d, dep + 1));
            }
        }
    }
    let mut model = None;
    if !models.is_empty() {
        let exact: Vec<&PathBuf> = models.iter().filter(|m| m.file_name().is_some_and(|n| n.to_string_lossy() == want)).collect();
        model = Some(match exact.first() {
            Some(m) => (*m).clone(),
            None => {
                let size = |m: &PathBuf| std::fs::metadata(m).map(|x| x.len()).unwrap_or(0);
                let mut best = models[0].clone();
                for m in &models {
                    if size(m) > size(&best) {
                        best = m.clone();
                    }
                }
                best
            }
        });
    }
    (cli, model)
}

pub fn stt_ready(model_key: &str, extra_dir: &str) -> bool {
    let (c, m) = stt_paths(model_key, extra_dir);
    c.is_some() && m.is_some()
}

/// stt_transcribe's text from whisper's output
pub fn stt_clean(out: &str) -> String {
    let lines: Vec<&str> = out.lines().filter(|l| !l.trim().is_empty() && !l.starts_with("whisper_") && !l.starts_with("main:")
        && !l.starts_with("system_info")).map(str::trim).collect();
    let text = lines.join(" ");
    let text = rx!(r"(?i)\[[^\]]*\]|\((?:music|noise|silence|inaudible|blank_audio|laughs?|applause)[^)]*\)").replace_all(&text, " ");
    rx!(r"\s+").replace_all(&text, " ").trim().to_string()
}

/// whisper-cli's arguments for one recording (stt_transcribe)
pub fn stt_args(cli: &Path, model: &Path, wav: &Path, threads: usize) -> (Vec<String>, PathBuf) {
    let base = wav.with_extension("");
    let out_base = PathBuf::from(format!("{}_text", base.display()));
    let s = |p: &Path| p.to_string_lossy().into_owned();
    (vec![s(cli), "-m".into(), s(model), "-f".into(), s(wav), "-l".into(), "en".into(), "-nt".into(), "-np".into(),
          "-t".into(), threads.to_string(), "--prompt".into(), data()["STT_PROMPT"].as_str().unwrap().into(), "-otxt".into(),
          "-of".into(), s(&out_base)], out_base)
}

/// stt_install(model_key, progress): whisper.cpp's Windows build and one English model into the speech folder
pub fn stt_install(model_key: &str, progress: &mut dyn FnMut(&str, f64)) -> Result<(PathBuf, PathBuf), String> {
    let sd = stt_dir();
    let _ = std::fs::create_dir_all(&sd);
    let (cli, _) = stt_paths(model_key, "");
    if cli.is_none() {
        let zip_name = "whisper-bin-x64.zip";
        let mut url = format!("https://github.com/ggml-org/whisper.cpp/releases/latest/download/{zip_name}");
        let mut sha = String::new();
        if let Ok(j) = http_json("https://api.github.com/repos/ggml-org/whisper.cpp/releases/latest", 15.0) {
            for a in j["assets"].as_array().into_iter().flatten() {
                if a["name"].as_str().unwrap_or("").to_lowercase() == zip_name {
                    if let Some(u) = a["browser_download_url"].as_str() {
                        url = u.into();
                    }
                    let d = a["digest"].as_str().unwrap_or("");
                    sha = if d.to_lowercase().starts_with("sha256:") { d[7..].into() } else { String::new() };
                }
            }
        }
        let zp = download(&url, &sd.join(zip_name), &sha, 500000, Some(progress), "the speech engine")?;
        let bin = sd.join("bin");
        let _ = std::fs::create_dir_all(&bin);
        let res = std::fs::read(&zp).map_err(|e| e.to_string()).and_then(|b| crate::unzip::entries(&b));
        let _ = std::fs::remove_file(&zp);
        let ents = res.map_err(|_| "The speech engine download was damaged. Try again later.".to_string())?;
        for (name, data) in ents {
            let name = name.replace('\\', "/");
            if name.ends_with('/') || name.split('/').any(|p| p == "..") || name.starts_with('/') {
                continue;
            }
            let out = name.split('/').fold(bin.clone(), |p, s| p.join(s));
            if let Some(d) = out.parent() {
                let _ = std::fs::create_dir_all(d);
            }
            std::fs::write(&out, data).map_err(|e| e.to_string())?;
        }
    }
    let (_, model) = stt_paths(model_key, "");
    let want = stt_model(model_key);
    if model.as_ref().and_then(|m| m.file_name()).is_none_or(|n| n.to_string_lossy() != want.0) {
        let dest = sd.join(&want.0);
        download(&format!("https://huggingface.co/ggerganov/whisper.cpp/resolve/main/{}", want.0), &dest, "",
                 (want.1 * 1000000 / 2) as u64, Some(progress), "the English model")?;
        let head = std::fs::read(&dest).map(|b| b.get(..4).map(|x| x.to_vec()).unwrap_or_default()).unwrap_or_default();
        if head != b"lmgg" {
            let _ = std::fs::remove_file(&dest);
            return Err("The model file isn't a whisper model (the network may have sent a web page instead).".into());
        }
    }
    match stt_paths(model_key, "") {
        (Some(c), Some(m)) => Ok((c, m)),
        (None, _) => Err("I couldn't find whisper-cli.exe in the speech engine download.".into()),
        (Some(_), None) => Err("I couldn't find whisper-cli.exe in the speech engine download.".into()),
    }
}

/// hashlib.sha1 (the speech cache's file names)
pub struct Sha1 {
    h: [u32; 5],
    buf: Vec<u8>,
    len: u64,
}

impl Default for Sha1 {
    fn default() -> Self {
        Self::new()
    }
}

impl Sha1 {
    pub fn new() -> Sha1 {
        Sha1 { h: [0x67452301, 0xEFCDAB89, 0x98BADCFE, 0x10325476, 0xC3D2E1F0], buf: vec![], len: 0 }
    }
    fn block(h: &mut [u32; 5], b: &[u8]) {
        let mut w = [0u32; 80];
        for i in 0..16 {
            w[i] = u32::from_be_bytes([b[4 * i], b[4 * i + 1], b[4 * i + 2], b[4 * i + 3]]);
        }
        for i in 16..80 {
            w[i] = (w[i - 3] ^ w[i - 8] ^ w[i - 14] ^ w[i - 16]).rotate_left(1);
        }
        let [mut a, mut bb, mut c, mut d, mut e] = *h;
        for (i, wi) in w.iter().enumerate() {
            let (f, k) = match i {
                0..=19 => ((bb & c) | (!bb & d), 0x5A827999),
                20..=39 => (bb ^ c ^ d, 0x6ED9EBA1),
                40..=59 => ((bb & c) | (bb & d) | (c & d), 0x8F1BBCDC),
                _ => (bb ^ c ^ d, 0xCA62C1D6),
            };
            let t = a.rotate_left(5).wrapping_add(f).wrapping_add(e).wrapping_add(k).wrapping_add(*wi);
            (e, d, c, bb, a) = (d, c, bb.rotate_left(30), a, t);
        }
        for (x, v) in h.iter_mut().zip([a, bb, c, d, e]) {
            *x = x.wrapping_add(v);
        }
    }
    pub fn update(&mut self, d: &[u8]) {
        self.len += d.len() as u64;
        self.buf.extend_from_slice(d);
        while self.buf.len() >= 64 {
            let blk: Vec<u8> = self.buf.drain(..64).collect();
            Self::block(&mut self.h, &blk);
        }
    }
    pub fn hex(&self) -> String {
        let mut h = self.h;
        let mut b = self.buf.clone();
        b.push(0x80);
        while b.len() % 64 != 56 {
            b.push(0);
        }
        b.extend((self.len * 8).to_be_bytes());
        for c in b.chunks(64) {
            Self::block(&mut h, c);
        }
        h.iter().map(|x| format!("{x:08x}")).collect()
    }
}

/// talk_card's refresh(): what the Talk card says about the offline ears
pub fn stt_status(model_key: &str, extra_dir: &str) -> String {
    let base = |p: &Path| p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    match stt_paths(model_key, extra_dir) {
        (Some(c), Some(m)) => format!("Ready: {} with {}.", base(&c), base(&m)),
        (Some(_), None) => "The engine is here but the English model is missing. Press Set up.".into(),
        _ => format!("Not set up yet. Set up downloads about {} MB once (GitHub + Hugging Face).", 5 + stt_model(model_key).1),
    }
}
