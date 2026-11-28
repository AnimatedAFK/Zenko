//! zenko.py's Spotify on the app side: music(action, arg, quiet) in a thread with fox_reply, presets (find_preset /
//! play_preset), music_poll every 7 s (the tablet and "~ song, by artist"), music_refresh for the Home page's
//! card, and Settings > Spotify's Connect / Disconnect / Copy redirect URI / Open dashboard.

use crate::voicehost::{bg, post};
use std::sync::Mutex;
use zenko_desktop::spotify::{self, Spotify};
use zenko_desktop::Companion;

static SP: Mutex<Option<Spotify>> = Mutex::new(None);

fn open_app() {
    crate::toolwins::open_url("spotify:");
}

/// the account (one, shared by the threads like zenko.py's self.spotify with its lock)
fn with_sp<R>(cid: &str, f: impl FnOnce(&mut Spotify) -> R) -> R {
    let mut g = SP.lock().unwrap();
    let sp = g.get_or_insert_with(|| {
        let mut s = Spotify::new(cid);
        s.open_app = Some(open_app);
        s
    });
    sp.client_id = cid.trim().to_string();
    f(sp)
}

fn connected(cid: &str) -> bool {
    with_sp(cid, |s| s.connected())
}

/// what the Home page's music card should show: (title, sub line, playing)
pub static HOME: Mutex<Option<(String, String, Option<bool>)>> = Mutex::new(None);

thread_local! {
    static NEXT_POLL: std::cell::Cell<f64> = const { std::cell::Cell::new(0.0) };
    static POLLING: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

fn clip(s: &str, n: usize) -> String {
    zenko_desktop::extra::clip(s, n)
}

/// music(action, arg, quiet)
pub fn music(fox: &mut Companion, t: f64, action: &str, arg: Option<&str>, quiet: bool) -> Option<String> {
    let cid = fox.cfg.s("spotify_client_id", "").trim().to_string();
    if !connected(&cid) {
        let msg = "Spotify isn't connected yet. Settings > Spotify (one-time setup, about 3 minutes).".to_string();
        if !quiet {
            fox.fox_reply(t, &msg);
        }
        return Some(msg);
    }
    if action == "play" {
        if let Some(a) = arg.filter(|a| !a.is_empty()) {
            let ps = fox.cfg.raw.get("music_presets").and_then(|v| v.as_array()).cloned().unwrap_or_default();
            if let Some(p) = spotify::find_preset(&ps, a) {
                return Some(play_preset(fox, p, quiet));
            }
        }
    }
    let (action, arg) = (action.to_string(), arg.map(String::from));
    let c2 = cid.clone();
    bg(move || with_sp(&c2, |sp| {
        if action == "volume_rel" {
            let n = sp.now().ok().flatten();
            let v = n.and_then(|n| n.volume).unwrap_or(50) + arg.as_deref().unwrap_or("0").parse::<i64>().unwrap_or(0);
            return sp.do_("volume", Some(&v.to_string()));
        }
        sp.do_(&action, arg.as_deref())
    }), move |fox, t, r| {
        if quiet && r.is_ok() {
            post_later(0.4, refresh);
            return;
        }
        fox.fox_reply(t, &r.unwrap_or_else(|e| e));
        refresh(fox);
        post_later(0.8, |fox| {
            let cid = fox.cfg.s("spotify_client_id", "");
            bg(move || with_sp(&cid, |s| s.now()), |fox, _, r| {
                if let Ok(n) = r {
                    fox.music.now = n;
                }
            });
        });
    });
    None
}

fn post_later(secs: f64, f: impl FnOnce(&mut Companion) + Send + 'static) {
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_secs_f64(secs));
        post(move |fox, _| f(fox));
    });
}

/// play_preset(p, quiet)
pub fn play_preset(fox: &mut Companion, p: serde_json::Value, quiet: bool) -> String {
    let cid = fox.cfg.s("spotify_client_id", "");
    bg(move || with_sp(&cid, |sp| sp.play_preset(&p)), move |fox, t, r| {
        if !(quiet && r.is_ok()) {
            fox.fox_reply(t, &r.unwrap_or_else(|e| e));
        }
        refresh(fox);
    });
    "On it...".into()
}

/// music_refresh(): the Home page's card (only while it shows and Spotify is connected)
pub fn refresh(fox: &mut Companion) {
    let cid = fox.cfg.s("spotify_client_id", "");
    if !crate::hubw::is_open() || !connected(&cid) {
        return;
    }
    bg(move || with_sp(&cid, |s| s.now()), |_, _, r| {
        *HOME.lock().unwrap() = Some(match r {
            Err(e) => ("Spotify".into(), e.chars().take(60).collect(), None),
            Ok(None) => ("Nothing playing".into(), "Open Spotify, or type: play focus music".into(), None),
            Ok(Some(n)) => (clip(&n.title, 30), format!("{}{}", if n.playing { "" } else { "Paused · " }, clip(&n.artist, 34)), Some(n.playing)),
        });
    });
}

/// the bubble / chat commands the app gets: spotify:<action>:<arg>, spotify_quiet:<action>:<arg>, music_preset:<i>
pub fn command(fox: &mut Companion, t: f64, c: &str) -> bool {
    let (quiet, rest) = if let Some(r) = c.strip_prefix("spotify_quiet:") { (true, r) }
                        else if let Some(r) = c.strip_prefix("spotify:") { (false, r) } else if let Some(i) = c.strip_prefix("music_preset:") {
        let i: usize = i.parse().unwrap_or(usize::MAX);
        if let Some(p) = fox.cfg.raw.get("music_presets").and_then(|v| v.get(i)).cloned() {
            play_preset(fox, p, false);
        }
        return true;
    } else {
        return false;
    };
    let (a, arg) = rest.split_once(':').unwrap_or((rest, ""));
    let arg = if arg.is_empty() && a != "play" { None } else { Some(arg) };
    music(fox, t, a, arg, quiet);
    true
}

/// music_poll, every 7 s: what's Spotify playing?
pub fn tick(fox: &mut Companion, t: f64) {
    if t < NEXT_POLL.with(|c| c.get()) {
        return;
    }
    NEXT_POLL.with(|c| c.set(t + 7.0));
    let cid = fox.cfg.s("spotify_client_id", "");
    if POLLING.with(|p| p.get()) || !connected(&cid) {
        return;
    }
    POLLING.with(|p| p.set(true));
    bg(move || with_sp(&cid, |s| s.now()), |fox, _, r| {
        POLLING.with(|p| p.set(false));
        let Ok(n) = r else { return };
        fox.music.now = n.clone();
        if let Some(n) = n.filter(|n| n.playing) {
            let sid = (n.title.clone(), n.artist.clone());
            if fox.music.last_song.as_ref() != Some(&sid) {
                fox.music.last_song = Some(sid);
                if fox.cfg.b("announce_songs", true) && fox.session.is_none() {
                    fox.say(&format!("~ {}, by {}", n.title, n.artist), 6.0);
                }
            }
        }
    });
}

/// Settings > Spotify: Connect (the sign-in in a thread), its answer on the status line and in chat
pub fn connect(fox: &mut Companion) {
    let cid = fox.cfg.s("spotify_client_id", "").trim().to_string();
    crate::ears::set_msg_pub("spotify", "Opening Spotify in your browser... approve it there.");
    bg(move || {
        // the sign-in waits up to 3 minutes: it runs on its own account object so music keeps working meanwhile
        let mut sp = Spotify::new(&cid);
        let r = sp.login(&|u| crate::toolwins::open_url(u), 180.0);
        if r.is_ok() {
            *SP.lock().unwrap() = None; // the shared one reloads the new token
        }
        r
    }, |fox, t, r| {
        let msg = match r {
            Ok(_) => "Spotify connected! Try: play focus music".to_string(),
            Err(e) => e,
        };
        crate::ears::set_msg_pub("spotify", &msg);
        fox.fox_reply(t, &msg);
        refresh(fox);
    });
}

pub fn forget(fox: &mut Companion) {
    let cid = fox.cfg.s("spotify_client_id", "");
    with_sp(&cid, |s| s.forget());
    fox.music.now = None;
}
