// The Zenko link: Zenko (zenko.py) reads this little file to see what you are reading in the
// browser, so study time is logged per subject automatically (Claude chats count too).
use std::path::PathBuf;

pub fn zenko_dir() -> PathBuf {
    let base = std::env::var_os("APPDATA").map(PathBuf::from).unwrap_or_else(std::env::temp_dir);
    base.join("FoxBuddy")
}

/// Zenko's folder, only when Zenko is installed on this account.
pub fn foxbuddy_dir() -> Option<PathBuf> {
    let d = zenko_dir();
    d.exists().then_some(d)
}

pub fn host_of(url: &str) -> String {
    let rest = match url.find("://") {
        Some(i) => &url[i + 3..],
        None => return String::new(),
    };
    let end = rest.find(|c| c == '/' || c == '?' || c == '#').unwrap_or(rest.len());
    let h = &rest[..end];
    let h = h.rsplit('@').next().unwrap_or(h);
    let h = h.split(':').next().unwrap_or(h);
    h.trim_start_matches("www.").to_ascii_lowercase()
}

pub fn write_now(title: &str, url: &str, host: &str, in_claude_panel: bool) {
    let dir = zenko_dir();
    if !dir.exists() {
        return; // Zenko is not installed on this account
    }
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let v = serde_json::json!({
        "app": "Zenko Browser", "version": crate::VERSION, "time": now,
        "title": title, "url": url, "host": host,
        "claude": in_claude_panel || host == "claude.ai",
        "pid": std::process::id(),
    });
    let tmp = dir.join("browser_now.tmp");
    if std::fs::write(&tmp, v.to_string()).is_ok() {
        let _ = std::fs::rename(&tmp, dir.join("browser_now.json"));
    }
}

/// What Zenko shares with the browser (zenko_link.json: upcoming deadlines, today's plan, ema reminders).
/// Returns the data (if it is under three days old) and whether Zenko is running right now.
pub fn zenko_link() -> Option<(serde_json::Value, bool)> {
    let t = std::fs::read_to_string(zenko_dir().join("zenko_link.json")).ok()?;
    let v: serde_json::Value = serde_json::from_str(&t).ok()?;
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs_f64())
        .unwrap_or(0.0);
    let age = now - v["time"].as_f64().unwrap_or(0.0);
    if !(-120.0..259_200.0).contains(&age) {
        return None;
    }
    Some((v, age < 50.0))
}

/// Is Zenko open right now (she rewrites zenko_link.json every 15 seconds)?
pub fn zenko_live() -> bool {
    zenko_link().map(|x| x.1).unwrap_or(false)
}

#[cfg(test)]
mod t {
    #[test]
    fn hosts() {
        assert_eq!(super::host_of("https://www.YouTube.com/watch?v=1"), "youtube.com");
        assert_eq!(super::host_of("https://claude.ai/chat/abc"), "claude.ai");
        assert_eq!(super::host_of("http://localhost:8080/x"), "localhost");
        assert_eq!(super::host_of("about:blank"), "");
    }
}
