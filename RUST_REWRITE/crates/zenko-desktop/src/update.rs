//! zenko.py's installing and updating, the parts that aren't Windows calls: version_tuple / version_newer,
//! write_lnk (a .lnk written byte for byte from [MS-SHLLINK]), update_check (GitHub Releases or a latest.json),
//! update_download (checksum, size cap, the file check), update_file_ok and zk_log_error. The Windows side
//! (copying, shortcuts' folders, the registry, swapping the running exe) is the bin's install.rs.

use crate::web::{self, Req};
use serde_json::{Value, json};
use std::path::Path;

/// version_tuple(v): every number in it
pub fn version_tuple(v: &str) -> Vec<u64> {
    regex::Regex::new(r"\d+").unwrap().find_iter(v).map(|m| m.as_str().parse::<u64>().unwrap_or(u64::MAX)).collect()
}

/// version_newer(a, b): 'v2026.12.01-zenko16' > 'v2026.11.22-zenko15'
pub fn version_newer(a: &str, b: &str) -> bool {
    let (ta, tb) = (version_tuple(a), version_tuple(b));
    !ta.is_empty() && ta > tb
}

/// write_lnk(target, args, workdir, icon, desc): the shortcut's bytes (`ansi` = target in the PC's code page)
pub fn lnk_bytes(target: &str, args: &str, workdir: &str, icon: &str, desc: &str, ansi: &[u8]) -> Vec<u8> {
    let mut flags: u32 = 0x02 | 0x80 | 0x10 | 0x40;
    if !desc.is_empty() {
        flags |= 0x04;
    }
    if !args.is_empty() {
        flags |= 0x20;
    }
    let mut hdr = vec![];
    hdr.extend(0x4Cu32.to_le_bytes());
    hdr.extend([0x01, 0x14, 0x02, 0, 0, 0, 0, 0, 0xc0, 0, 0, 0, 0, 0, 0, 0x46]);
    hdr.extend(flags.to_le_bytes());
    hdr.extend(0x20u32.to_le_bytes());
    hdr.extend([0u8; 24]); // three FILETIMEs
    hdr.extend(0u32.to_le_bytes()); // file size
    hdr.extend(0u32.to_le_bytes()); // icon index
    hdr.extend(1u32.to_le_bytes()); // SW_SHOWNORMAL
    hdr.extend(0u16.to_le_bytes()); // hotkey
    hdr.extend(0u16.to_le_bytes());
    hdr.extend(0u32.to_le_bytes());
    hdr.extend(0u32.to_le_bytes());
    let mut vol = vec![];
    for v in [0x11u32, 3, 0, 0x10] {
        vol.extend(v.to_le_bytes());
    }
    vol.push(0);
    let mut base_a = ansi.to_vec();
    base_a.push(0);
    let mut base_u: Vec<u8> = target.encode_utf16().flat_map(|c| c.to_le_bytes()).collect();
    base_u.extend([0, 0]);
    let head = 0x24u32;
    let off_vol = head;
    let off_base = off_vol + vol.len() as u32;
    let off_suffix = off_base + base_a.len() as u32;
    let off_base_u = off_suffix + 1;
    let off_suffix_u = off_base_u + base_u.len() as u32;
    let mut body = vol.clone();
    body.extend(&base_a);
    body.push(0);
    body.extend(&base_u);
    body.extend([0, 0]);
    let size = head + body.len() as u32;
    let mut info = vec![];
    for v in [size, head, 0x01, off_vol, off_base, 0, off_suffix, off_base_u, off_suffix_u] {
        info.extend(v.to_le_bytes());
    }
    info.extend(body);
    let sd = |s: &str| {
        let u: Vec<u16> = s.encode_utf16().collect();
        let mut o = (u.len() as u16).to_le_bytes().to_vec();
        o.extend(u.iter().flat_map(|c| c.to_le_bytes()));
        o
    };
    let mut strings = vec![];
    if !desc.is_empty() {
        strings.extend(sd(desc));
    }
    let wd = if workdir.is_empty() {
        // os.path.dirname (ntpath): everything before the last \ or /, without trailing slashes
        let i = target.rfind(['\\', '/']).map(|i| i + 1).unwrap_or(0);
        let h = &target[..i];
        let t = h.trim_end_matches(['\\', '/']);
        if t.is_empty() { h.to_string() } else { t.to_string() }
    } else {
        workdir.to_string()
    };
    strings.extend(sd(&wd));
    if !args.is_empty() {
        strings.extend(sd(args));
    }
    strings.extend(sd(if icon.is_empty() { target } else { icon }));
    let mut data = hdr;
    data.extend(info);
    data.extend(strings);
    data.extend([0u8; 4]);
    data
}

/// _github_api(url)
fn github_api(url: &str, timeout_s: f64) -> Result<Value, String> {
    let r = Req { url: url.into(), method: String::new(), ua: "Zenko-updater".into(), headers: "Accept: application/vnd.github+json".into(),
                  body: vec![], timeout_ms: (timeout_s * 1000.0) as i32 };
    let mut out = vec![];
    match web::fetch(&r, &mut |b, _| {
        out.extend_from_slice(b);
        true
    }) {
        Ok(404) => Err("I couldn't find any releases there yet. Check the name in Settings > Updates.".into()),
        Ok(c) if c >= 400 => Err(format!("GitHub answered with an error ({c}). Try again later.")),
        Ok(_) => serde_json::from_slice(&out)
            .map_err(|_| "I can't reach the update server right now (no internet, or the network blocks it).".into()),
        Err(_) => Err("I can't reach the update server right now (no internet, or the network blocks it).".into()),
    }
}

fn basename(u: &str) -> String {
    u.rsplit('/').next().unwrap_or(u).to_string()
}

/// update_check(source, prefer): the newest release {version, url, sha256, notes, name, size, kind}, or None.
/// `exe_name` is the exe asset to look for first ("zenko.exe" in zenko.py); `can_py` = this app can run a zenko.py
/// (zenko.py and Zenko.exe can; the Rust app can't, so it only takes an exe).
pub fn update_check(source: &str, prefer: Option<&str>, exe_name: &str, can_py: bool, timeout_s: f64) -> Result<Option<Value>, String> {
    let source = source.trim();
    if source.is_empty() {
        return Ok(None);
    }
    let mut want = prefer.unwrap_or("py").to_string();
    let low = source.to_lowercase();
    if (low.starts_with("https://") || low.starts_with("http://"))
        && !regex::Regex::new(r"^https?://github\.com/[^/]+/[^/]+/?$").unwrap().is_match(source) {
        let j = web::http_json(source, timeout_s).map_err(|e| e.to_string())?;
        let has = |k: &str| j[k].as_str().is_some_and(|s| !s.is_empty());
        if !has(&want) {
            want = if want == "py" { "exe".into() } else { "py".into() };
        }
        if want == "py" && !can_py {
            return Ok(None);
        }
        let url = j[want.as_str()].as_str().unwrap_or("").to_string();
        if url.is_empty() {
            return Ok(None);
        }
        let v = match &j["version"] { Value::String(s) => s.clone(), Value::Null => String::new(), o => o.to_string() };
        let sha = match &j[format!("{want}_sha256").as_str()] { Value::String(s) => s.to_lowercase(), Value::Null => String::new(), o => o.to_string().to_lowercase() };
        let notes: String = match &j["notes"] { Value::String(s) => s.clone(), Value::Null => String::new(), o => o.to_string() }.chars().take(400).collect();
        return Ok(Some(json!({"version": v, "url": url, "sha256": sha, "notes": notes,
                              "name": basename(url.split('?').next().unwrap_or("")), "size": 0, "kind": want})));
    }
    let s2 = source.replace("https://github.com/", "");
    let re = regex::Regex::new(r"([\w.-]+)/([\w.-]+?)(?:\.git)?/?$").unwrap();
    let Some(m) = re.captures(&s2) else {
        return Err("Updates need a GitHub name like yourname/zenko.".into());
    };
    let j = github_api(&format!("https://api.github.com/repos/{}/{}/releases/latest", &m[1], &m[2]), timeout_s)?;
    let assets = j["assets"].as_array().cloned().unwrap_or_default();
    let mut names: Vec<(String, Value)> = vec![];
    for a in assets {
        let n = a["name"].as_str().unwrap_or("").to_lowercase();
        if let Some(e) = names.iter_mut().find(|(k, _)| *k == n) {
            e.1 = a; // a dict keeps the last one
        } else {
            names.push((n, a));
        }
    }
    let get = |n: &str| names.iter().find(|(k, _)| k == n).map(|x| x.1.clone());
    let exe_pick = get(exe_name).or_else(|| if exe_name == "zenko.exe" {
        names.iter().find(|(n, _)| n.ends_with(".exe") && !n.contains("setup")).map(|x| x.1.clone())
    } else { None });
    let py_pick = if can_py { get("zenko.py") } else { None };
    let mut pick = if want == "py" { py_pick.clone() } else { exe_pick.clone() };
    if pick.is_none() {
        (pick, want) = if want == "py" { (exe_pick, "exe".into()) } else { (py_pick, "py".into()) };
    }
    let Some(pick) = pick else { return Ok(None) };
    let dg = pick["digest"].as_str().unwrap_or("").to_string();
    let mut sha = if dg.to_lowercase().starts_with("sha256:") { dg.split_once(':').unwrap().1.to_lowercase() } else { String::new() };
    let pname = pick["name"].as_str().unwrap_or("").to_string();
    if sha.is_empty() {
        if let Some(side) = get(&format!("{}.sha256", pname.to_lowercase())) {
            sha = web::http_get(side["browser_download_url"].as_str().unwrap_or(""), timeout_s).ok()
                .map(|b| String::from_utf8_lossy(&b).split_whitespace().next().unwrap_or("").to_lowercase()).unwrap_or_default();
        }
    }
    let ver = j["tag_name"].as_str().filter(|s| !s.is_empty()).or(j["name"].as_str()).unwrap_or("").to_string();
    let notes: String = j["body"].as_str().unwrap_or("").chars().take(400).collect();
    Ok(Some(json!({"version": ver, "url": pick["browser_download_url"].as_str().unwrap_or(""), "sha256": sha, "notes": notes,
                   "name": pname, "size": pick["size"].as_i64().unwrap_or(0), "kind": want})))
}

/// update_file_ok(path, "exe"): (ok, why)
pub fn exe_ok(path: &Path) -> (bool, String) {
    let Ok(b) = std::fs::read(path) else { return (false, "I couldn't read that file.".into()) };
    if !b.starts_with(b"MZ") || b.len() < 2 * 1024 * 1024 {
        return (false, "That isn't a Zenko.exe.".into());
    }
    (true, String::new())
}

/// update_download(info, dest, progress): the file checked (checksum, the exe check) and moved into place
pub fn update_download(info: &Value, dest: &Path, progress: &mut dyn FnMut(f64)) -> Result<(), String> {
    let tmp = std::path::PathBuf::from(format!("{}.part", dest.display()));
    let r = Req { url: info["url"].as_str().unwrap_or("").into(), method: String::new(), ua: "Zenko-updater".into(), headers: String::new(),
                  body: vec![], timeout_ms: 30000 };
    let mut f = std::fs::File::create(&tmp).map_err(|e| e.to_string())?;
    let mut h = web::Sha256::new();
    let mut got = 0u64;
    let mut too_big = false;
    let size = info["size"].as_u64().unwrap_or(0);
    let res = web::fetch(&r, &mut |b, total| {
        use std::io::Write;
        if f.write_all(b).is_err() {
            return false;
        }
        h.update(b);
        got += b.len() as u64;
        if got > 400 * 1024 * 1024 {
            too_big = true;
            return false;
        }
        let t = if total > 0 { total } else { size };
        if t > 0 {
            progress(got as f64 / t as f64);
        }
        true
    });
    drop(f);
    if too_big {
        let _ = std::fs::remove_file(&tmp);
        return Err("That download is far too big to be Zenko.".into());
    }
    if !matches!(res, Ok(c) if c < 400) {
        let _ = std::fs::remove_file(&tmp);
        return Err("The download didn't finish (the network may block it). You can also download the file yourself and use \
                    Settings > Updates > Install from a file.".into());
    }
    let want = info["sha256"].as_str().unwrap_or("");
    if !want.is_empty() && h.hex().to_lowercase() != want {
        let _ = std::fs::remove_file(&tmp);
        return Err("The download didn't match its checksum, so I threw it away. Try again later.".into());
    }
    let (ok, why) = exe_ok(&tmp);
    if !ok {
        let _ = std::fs::remove_file(&tmp);
        return Err(why);
    }
    std::fs::rename(&tmp, dest).map_err(|e| e.to_string())
}

/// zk_log_error(text): error_log.txt in the data folder (kept under 400 KB)
pub fn log_error(text: &str, version: &str) {
    let p = crate::data_dir().join("error_log.txt");
    if std::fs::metadata(&p).map(|m| m.len() > 400000).unwrap_or(false) {
        let _ = std::fs::rename(&p, crate::data_dir().join("error_log.txt.old"));
    }
    let _ = std::fs::create_dir_all(crate::data_dir());
    let (y, mo, d, h, mi, se) = crate::local_now();
    let stamp = format!("{y:04}-{mo:02}-{d:02} {h:02}:{mi:02}:{se:02}");
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(&p) {
        use std::io::Write;
        let _ = write!(f, "\n[{stamp} {version}]\n{text}\n");
    }
}
