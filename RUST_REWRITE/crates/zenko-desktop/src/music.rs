//! Music (zenko.py): the background loops (music_apply / music_watch / BackgroundMusic; the window side plays them
//! with MCI like zenko.py) and the music tablet in her paws while Spotify is open (player_poll, tablet_grid,
//! draw_tablet, tablet_hit, tablet_press). With a Spotify account (spotify.rs) the tablet also reads music_poll's
//! now-playing and its buttons drive Spotify; without one it reads the window title and presses the media keys.

use crate::rewards::Later;
use crate::{Companion, Ev, FONT35};
use serde_json::Value;
use std::sync::OnceLock;
use unicode_normalization::UnicodeNormalization;

pub fn ldata() -> &'static Value {
    static D: OnceLock<Value> = OnceLock::new();
    D.get_or_init(|| serde_json::from_str(include_str!("../data/life.json")).expect("life.json"))
}

pub const TAB_X: i32 = 15;
pub const TAB_Y: i32 = 49;
pub const TAB_W: usize = 42;
pub const TAB_H: usize = 24;

#[derive(Clone, Debug, PartialEq, Default)]
pub struct Player {
    pub playing: bool,
    pub title: String,
    pub artist: String,
}

#[derive(Clone, Debug, Default)]
pub struct Music {
    /// what music_apply last asked for: (on, track, volume)
    pub bgm_state: Option<(bool, String, i64)>,
    pub player: Option<Player>,
    pub song_memory: Option<(String, String)>,
    pub last_song: Option<(String, String)>,
    pub tab_flash: (String, f64),
    /// music_now: what the Spotify account says is on (music_poll, every 7 s)
    pub now: Option<crate::spotify::Now>,
}

fn font35(ch: char) -> Option<&'static [&'static str; 5]> {
    FONT35.iter().find(|(c, _)| *c == ch).map(|(_, g)| g)
}

impl Companion {
    /// music_apply: start / stop / switch the background loop to match the settings (and focus-only mode)
    pub fn music_apply(&mut self) {
        self.trace.push("music_apply".into());
        let mut on = self.cfg.b("bgm", true);
        if on && self.cfg.s("bgm_when", "always") == "focus" {
            on = self.session.is_some() || self.lockin.is_some();
        }
        let track = self.cfg.s("bgm_track", "garden");
        let vol = match self.cfg.f("bgm_volume", 2.0) as i64 {
            0 => 2,
            v => v,
        };
        let want = (on, track.clone(), vol);
        if self.music.bgm_state.as_ref() == Some(&want) {
            return;
        }
        self.music.bgm_state = Some(want);
        self.events.push(Ev::Bgm(if on { Some((track, vol)) } else { None }));
    }

    /// music_apply 300 ms after a session starts or ends
    pub fn music_apply_soon(&mut self, now: f64) {
        self.later(now + 0.3, Later::MusicApply);
    }

    /// player_poll, every 1.5 s: `win` = the Spotify window's state (spotify_window), None when it isn't open
    pub fn player_poll(&mut self, win: Option<Player>) {
        let mut p = None;
        let api = self.music.now.clone().filter(|n| !n.title.is_empty());
        if let Some(w) = win {
            let mut q = w.clone();
            if !q.title.is_empty() {
                self.music.song_memory = Some((q.title.clone(), q.artist.clone()));
            } else if let Some(a) = &api {
                q.title = a.title.clone();
                q.artist = a.artist.clone();
                q.playing = q.playing || a.playing;
            } else if let Some((t, a)) = &self.music.song_memory {
                q.title = t.clone();
                q.artist = a.clone();
            }
            p = Some(q);
        } else if let Some(a) = &api {
            p = Some(Player { playing: a.playing, title: a.title.clone(), artist: a.artist.clone() });
        }
        if let Some(q) = &mut p {
            if q.title.is_empty() {
                q.title = "Spotify".into();
            }
        }
        if let Some(q) = &p {
            if q.playing {
                let sid = (q.title.clone(), q.artist.clone());
                if self.music.last_song.as_ref() != Some(&sid) && q.title != "Spotify" {
                    self.music.last_song = Some(sid);
                    if self.cfg.b("announce_songs", true) && self.session.is_none() {
                        let by = if q.artist.is_empty() { String::new() } else { format!(", by {}", q.artist) };
                        self.say(&format!("~ {}{by}", q.title), 6.0);
                    }
                }
            }
        }
        self.music.player = p;
    }

    pub fn tablet_shown(&self, state: &str) -> bool {
        self.music.player.is_some() && self.cfg.b("belly_music", true) && state != "curl"
    }

    /// tablet_grid: (rows of palette letters, scroll offset, flashed button)
    pub fn tablet_grid(&self, now: f64) -> (Vec<Vec<u8>>, usize, Option<String>) {
        let p = self.music.player.clone().unwrap_or_default();
        let (w, h) = (TAB_W, TAB_H);
        let mut g = vec![vec![b'.'; w]; h];
        for y in 0..h {
            for x in 0..w {
                let (dx, dy) = (x.min(w - 1 - x), y.min(h - 1 - y));
                if dx + dy < 2 || (dx == 0 && dy < 3) || (dy == 0 && dx < 3) {
                    continue;
                }
                let edge = dx == 0 || dy == 0 || (dx == 1 && dy == 1) || (dx + dy == 2);
                g[y][x] = if edge { b'K' } else if dx < 3 || dy < 3 { b'z' } else { b'y' };
            }
        }
        for x in 4..w - 4 {
            g[1][x] = b'f';
        }
        let mut raw = p.title.clone();
        if !p.artist.is_empty() {
            raw += " - ";
            raw += &p.artist;
        }
        let raw: String = raw.nfkd().filter(|c| c.is_ascii()).collect::<String>().to_uppercase();
        let text: String = raw.chars().map(|c| if font35(c).is_some() { c } else { ' ' }).collect();
        let text = text.trim_matches(|c: char| c.is_whitespace()).to_string();
        let text = if text.is_empty() { "SPOTIFY".to_string() } else { text };
        let mut cols: Vec<[bool; 5]> = vec![];
        for ch in text.chars() {
            let gl = font35(ch).unwrap_or(&["", "", "", "", ""]);
            for cx in 0..gl[0].len() {
                let mut col = [false; 5];
                for r in 0..5 {
                    col[r] = gl[r].as_bytes()[cx] == b'#';
                }
                cols.push(col);
            }
            cols.push([false; 5]);
        }
        let vis = w - 10;
        let mut off = 0;
        let window: Vec<[bool; 5]> = if cols.len() - 1 <= vis {
            let pad = (vis - cols.len() + 1) / 2;
            let mut v = vec![[false; 5]; pad];
            v.extend_from_slice(&cols[..cols.len() - 1]);
            v
        } else {
            let mut lp = cols.clone();
            lp.extend(std::iter::repeat_n([false; 5], 8));
            off = (now / 0.12) as usize % lp.len();
            (0..vis).map(|i| lp[(off + i) % lp.len()]).collect()
        };
        for (i, col) in window.iter().take(vis).enumerate() {
            for r in 0..5 {
                if col[r] {
                    g[5 + r][5 + i] = b'H';
                }
            }
        }
        let playing = p.playing;
        for i in 5..w - 5 {
            let fill = 5 + (((now / 3.0) % 1.0) * (w - 10) as f64) as usize;
            g[12][i] = if playing && i < fill { b'v' } else { b'f' };
        }
        let (fl, ft) = &self.music.tab_flash;
        let flash = (now - ft < 0.18 && !fl.is_empty()).then(|| fl.clone());
        let (cx, cy) = (20.5f64, 18.5f64);
        for y in 14..23 {
            for x in 15..27 {
                let d = (x as f64 + 0.5 - cx).hypot(y as f64 + 0.5 - cy);
                if d <= 4.6 {
                    g[y][x] = if d <= 3.8 { if flash.as_deref() == Some("playpause") { b'H' } else { b'v' } } else { b'u' };
                }
            }
        }
        if playing {
            for y in 16..21 {
                for x in [18, 19, 22, 23] {
                    g[y][x] = b'y';
                }
            }
        } else {
            for (k, y) in (16..21).enumerate() {
                let n = if k < 3 { k + 1 } else { 5 - k };
                for x in 19..19 + n {
                    g[y][x] = b'y';
                }
            }
        }
        let ink = |n: &str| if flash.as_deref() == Some(n) { b'v' } else { b'H' };
        let (ip, inx) = (ink("previous"), ink("next"));
        for (k, y) in (15..22).enumerate() {
            let w_ = 4 - (k as i32 - 3).abs();
            for x in (12 - w_)..12 {
                g[y][x as usize] = ip;
            }
            g[y][6] = ip;
            g[y][7] = ip;
            for x in 29..29 + w_ {
                g[y][x as usize] = inx;
            }
            g[y][33] = inx;
            g[y][34] = inx;
        }
        (g, off, flash)
    }

    /// draw_tablet's picture: the tablet with her paws holding the top edge (rows, TAB_H + 3 high)
    pub fn tablet_rows(&self, now: f64) -> Vec<String> {
        let (g, _, _) = self.tablet_grid(now);
        let mut full = vec![vec![b'.'; TAB_W]; TAB_H + 3];
        for (y, r) in g.into_iter().enumerate() {
            full[y + 3] = r;
        }
        for pcx in [14.5f64, 27.5] {
            for y in 0..8usize {
                for x in (pcx as i32 - 5)..(pcx as i32 + 6) {
                    let d = ((x as f64 + 0.5 - pcx) / 4.2).powi(2) + ((y as f64 + 0.5 - 3.6) / 3.0).powi(2);
                    if (0..TAB_W as i32).contains(&x) {
                        let xu = x as usize;
                        if d <= 1.0 {
                            full[y][xu] = if y >= 4 || (x as usize + y) % 3 != 0 { b'n' } else { b'N' };
                        } else if d <= 1.45 && matches!(full[y][xu], b'.' | b'z' | b'f' | b'K') {
                            full[y][xu] = b'K';
                        }
                    }
                }
            }
            for tx in [pcx - 1.5, pcx + 0.5] {
                full[5][tx.round_ties_even() as usize] = b'K';
            }
        }
        full.into_iter().map(|r| String::from_utf8(r).unwrap()).collect()
    }

    /// tablet_hit(x, y) in window pixels -> "previous" / "playpause" / "next" / "screen"
    pub fn tablet_hit(&self, now: f64, x: i32, y: i32) -> Option<&'static str> {
        if self.music.player.is_none() || !self.cfg.b("belly_music", true) || self.is_curled(now) {
            return None;
        }
        let z = self.z as f64;
        let sx = (x - self.colw) as f64 / z - TAB_X as f64;
        let sy = (y - self.fox_y) as f64 / z - TAB_Y as f64;
        if !(0.0 <= sx && sx < TAB_W as f64 && 0.0 <= sy && sy < TAB_H as f64) {
            return None;
        }
        if sy >= 13.0 {
            for (name, x0, x1) in [("previous", 5.0, 12.0), ("playpause", 16.0, 25.0), ("next", 29.0, 36.0)] {
                if x0 - 1.0 <= sx && sx <= x1 + 1.0 {
                    return Some(name);
                }
            }
        }
        Some("screen")
    }

    /// is_curled: asleep in a ball (away, not on a break, no mood showing)
    pub fn is_curled(&self, now: f64) -> bool {
        self.sleeping && self.brk.is_none() && self.cfg.b("curl_when_away", true)
            && !self.mood_override.as_ref().is_some_and(|(_, u)| now < *u)
    }

    /// tablet_press(name)
    pub fn tablet_press(&mut self, now: f64, name: &str) {
        self.music.tab_flash = (name.to_string(), now);
        let p = self.music.player.clone().unwrap_or_default();
        if name == "screen" {
            let t = if self.music.player.is_some() { p.title.clone() } else { "Spotify".into() };
            let by = if p.artist.is_empty() { String::new() } else { format!(", by {}", p.artist) };
            self.say(&format!("~ {t}{by}"), 6.0);
            return;
        }
        self.events.push(Ev::Beep { kind: "pet".into(), variety: false });
        if self.spotify_connected() {
            let act = match name {
                "previous" | "next" => name,
                _ if p.playing => "pause",
                _ => "play",
            };
            self.events.push(Ev::Command(format!("spotify_quiet:{act}:")));
        } else {
            self.events.push(Ev::Media(name.to_string()));
        }
        if name == "playpause" {
            if let Some(q) = &mut self.music.player {
                q.playing = !q.playing;
            }
        }
        if name == "next" || name == "previous" {
            self.later(now + 0.9, Later::ForgetSong);
        }
    }
}

impl Companion {
    /// music_session_hook(event): the focus playlist with a session, paused for breaks / at the end (spotify_auto)
    pub fn music_session_hook(&mut self, event: &str) {
        if !self.cfg.b("spotify_auto", false) || !self.spotify_connected() {
            return;
        }
        if event == "start" {
            let pl = self.cfg.s("spotify_focus_playlist", "").trim().to_string();
            self.events.push(Ev::Command(format!("spotify_quiet:play:{pl}")));
        } else {
            self.events.push(Ev::Command("spotify_quiet:pause:".into()));
        }
    }
}
