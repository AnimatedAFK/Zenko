// Auto-update from GitHub Releases of AnimatedAFK/Zenko. Browser releases use tags like
// "browser-v0.1.0" and are marked pre-release, so they never get in the way of Zenko's own updates.
use std::io::Read;

use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use tao::event_loop::EventLoopProxy;

use crate::{Ev, VERSION};

const REPO: &str = "AnimatedAFK/Zenko";
const ASSET: &str = "ZenkoBrowser.exe";

fn parse(v: &str) -> Vec<u64> {
    v.trim_start_matches("browser-")
        .trim_start_matches('v')
        .split(|c: char| !c.is_ascii_digit())
        .filter(|s| !s.is_empty())
        .map(|s| s.parse().unwrap_or(0))
        .collect()
}

pub fn newer(a: &str, b: &str) -> bool {
    parse(a) > parse(b)
}

fn get(url: &str) -> Result<ureq::Response, String> {
    // Windows' own TLS (schannel) so school networks with their own certificates still work.
    let tls = native_tls::TlsConnector::new().map_err(|e| e.to_string())?;
    let agent = ureq::AgentBuilder::new().tls_connector(std::sync::Arc::new(tls)).build();
    agent
        .get(url)
        .set("User-Agent", "ZenkoBrowser-updater")
        .set("Accept", "application/vnd.github+json")
        .timeout(std::time::Duration::from_secs(60))
        .call()
        .map_err(|e| e.to_string())
}

fn check() -> Result<Value, String> {
    let list: Value = get(&format!("https://api.github.com/repos/{REPO}/releases?per_page=40"))?
        .into_json()
        .map_err(|e| e.to_string())?;
    let mut best: Option<&Value> = None;
    for r in list.as_array().into_iter().flatten() {
        let tag = r["tag_name"].as_str().unwrap_or("");
        if !tag.starts_with("browser-v") || r["draft"].as_bool().unwrap_or(false) {
            continue;
        }
        if best.map(|b| newer(tag, b["tag_name"].as_str().unwrap_or(""))).unwrap_or(true) {
            best = Some(r);
        }
    }
    let Some(r) = best else { return Ok(json!({"state": "none"})) };
    let tag = r["tag_name"].as_str().unwrap_or("");
    if !newer(tag, VERSION) {
        return Ok(json!({"state": "none"}));
    }
    let assets = r["assets"].as_array().cloned().unwrap_or_default();
    let find = |name: &str| {
        assets
            .iter()
            .find(|a| a["name"] == name)
            .and_then(|a| a["browser_download_url"].as_str().map(String::from))
    };
    let Some(exe_url) = find(ASSET) else { return Ok(json!({"state": "none"})) };
    let want = find(&format!("{ASSET}.sha256"))
        .and_then(|u| get(&u).ok()?.into_string().ok())
        .map(|s| s.trim().split_whitespace().next().unwrap_or("").to_ascii_lowercase());

    let mut bytes = vec![];
    get(&exe_url)?
        .into_reader()
        .take(200 * 1024 * 1024)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() < 500_000 || &bytes[..2] != b"MZ" {
        return Err("the download was not a program".into());
    }
    let got = format!("{:x}", Sha256::digest(&bytes));
    if let Some(w) = want {
        if !w.is_empty() && w != got {
            return Err("the download was damaged (checksum mismatch)".into());
        }
    }
    install(&bytes)?;
    Ok(json!({"state": "ready", "version": tag.trim_start_matches("browser-v")}))
}

/// Windows lets a running program be renamed, so: rename us to .old, put the new one in our place.
fn install(bytes: &[u8]) -> Result<(), String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let new = exe.with_extension("new");
    let old = exe.with_extension("old");
    std::fs::write(&new, bytes).map_err(|e| e.to_string())?;
    let _ = std::fs::remove_file(&old);
    std::fs::rename(&exe, &old).map_err(|e| e.to_string())?;
    if let Err(e) = std::fs::rename(&new, &exe) {
        let _ = std::fs::rename(&old, &exe);
        return Err(e.to_string());
    }
    Ok(())
}

pub fn cleanup_old() {
    if let Ok(exe) = std::env::current_exe() {
        let _ = std::fs::remove_file(exe.with_extension("old"));
        let _ = std::fs::remove_file(exe.with_extension("new"));
    }
}

pub fn start(proxy: EventLoopProxy<Ev>, manual: bool) {
    if cfg!(debug_assertions) && !manual {
        return;
    }
    std::thread::spawn(move || {
        if !manual {
            std::thread::sleep(std::time::Duration::from_secs(8));
        }
        let v = match check() {
            Ok(v) => v,
            Err(e) => json!({"state": "error", "error": e}),
        };
        if manual || v["state"] == "ready" {
            let _ = proxy.send_event(Ev::Update(v));
        }
    });
}

#[cfg(test)]
mod t {
    #[test]
    fn versions() {
        assert!(super::newer("browser-v0.2.0", "0.1.9"));
        assert!(super::newer("browser-v0.10.0", "0.9.0"));
        assert!(!super::newer("browser-v0.1.0", "0.1.0"));
    }
}
