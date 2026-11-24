// The Zenko link: Zenko (zenko.py) reads this little file to see what you are reading in the
// browser, so study time is logged per subject automatically (Claude chats count too).
use std::path::PathBuf;

pub fn zenko_dir() -> PathBuf {
    let base = std::env::var_os("APPDATA").map(PathBuf::from).unwrap_or_else(std::env::temp_dir);
    base.join("FoxBuddy")
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
