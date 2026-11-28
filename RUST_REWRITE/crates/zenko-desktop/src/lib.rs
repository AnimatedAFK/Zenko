//! The desktop fox (zenko.py FoxBuddy: compute_dims, accessories, tail_count, current_mood, animate,
//! idle_moves, look_vec, draw_belly, belly_text, draw_foxfire, get_image, get_tails, get_orb).
//! Platform independent: `Companion::frame` returns the finished RGBA picture of the whole window.

use serde_json::Value;
use std::collections::HashMap;
use std::path::PathBuf;
use zenko_art::core::{BELLY_X, BELLY_Y, H as H_, TAIL_BASE, W as W_};
use zenko_art::palette::{LookCfg, Palette, ROBES, is_robe, make_pal, mixhex};
use zenko_art::raster::{Image, sprite_photo};
use zenko_art::{Grid, Look, core, curl, rig, sit};

pub mod play;
pub mod session;
pub mod shapes;
pub mod watch;
pub mod tkgrid;
pub mod life;
pub mod lockin;
pub mod rewards;
pub mod tt;
pub mod music;
pub mod slow;
pub mod battery;
pub mod calc;
pub mod extra;
pub mod lifechat;
pub mod miscroutes;
pub mod guard;
pub mod pagekit;
pub mod homepage;
pub mod youpage;
pub mod dailypage;
pub mod ttpage;
pub mod uppage;
pub mod rempage;
pub mod setpage;
pub mod custpage;
pub mod ui;
pub mod checkin;
pub mod tkcanvas;
pub mod aa;
pub mod hubside;
pub mod canvasdraw;
pub mod hubwin;
pub mod hubanim;
pub mod tips;
pub mod tktext;
pub mod chatpage;
pub mod chat;
pub mod ema;
pub mod plan;
pub mod z22;
pub mod routes17;
pub mod gym;
pub mod books;
pub mod study;
pub mod chat_data;
pub mod pydifflib;
pub mod palette;
pub mod shelfpage;
pub mod bookgen;
pub mod bookpage;
pub mod notes;
pub mod notespage;
pub mod mdtext;
pub mod mdtex_data;
pub mod icon;
pub mod sound;
pub mod theme;
pub use play::{Play, Pose, Prop};

/// Things the desktop fox asks the window side to do (the bubble, sounds, menu commands).
#[derive(Clone, Debug)]
pub enum Ev {
    /// say(text, secs, choices): a speech bubble; choices = (label, command)
    Say { text: String, secs: f64, choices: Vec<(String, String)> },
    /// remind_aloud: read this out with the Windows voice
    Speak(String),
    /// the background loop: Some((track, volume)) to play it, None to stop
    Bgm(Option<(String, i64)>),
    /// press a media key ("playpause" / "next" / "previous")
    Media(String),
    /// beep(kind): one of her voice clips (variety = the play-interaction pool)
    Beep { kind: String, variety: bool },
    /// ui_sound(kind)
    Ui(String),
    Command(String),
}

static FAKE_NOW: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// Tests only: pin the wall clock (unix seconds) so results can be compared with zenko.py run at the same time.
pub fn set_fake_now(t: Option<f64>) {
    FAKE_NOW.store(t.map(f64::to_bits).unwrap_or(0), std::sync::atomic::Ordering::Relaxed);
}

pub fn unix_now() -> f64 {
    let f = FAKE_NOW.load(std::sync::atomic::Ordering::Relaxed);
    if f != 0 {
        return f64::from_bits(f);
    }
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs_f64())
        .unwrap_or(0.0)
}

/// Python's json.dump(d, f, indent=1, ensure_ascii=False).
pub fn py_json(v: &Value) -> String {
    let mut out = Vec::new();
    let fmt = serde_json::ser::PrettyFormatter::with_indent(b" ");
    let mut ser = serde_json::Serializer::with_formatter(&mut out, fmt);
    serde::Serialize::serialize(v, &mut ser).ok();
    String::from_utf8(out).unwrap_or_default()
}

/// Python's json.dump(cfg, f, indent=2) (ensure_ascii on: non-ASCII written as \uXXXX), used for config.json.
pub fn py_json_ascii(v: &Value) -> String {
    py_json_ascii_indent(v, b"  ")
}

pub fn py_json_ascii_indent(v: &Value, indent: &[u8]) -> String {
    let mut out = Vec::new();
    let fmt = serde_json::ser::PrettyFormatter::with_indent(indent);
    let mut ser = serde_json::Serializer::with_formatter(&mut out, fmt);
    serde::Serialize::serialize(v, &mut ser).ok();
    let s = String::from_utf8(out).unwrap_or_default();
    let mut o = String::with_capacity(s.len());
    for ch in s.chars() {
        if (ch as u32) < 128 {
            o.push(ch);
        } else {
            let mut b = [0u16; 2];
            for u in ch.encode_utf16(&mut b) {
                o.push_str(&format!("\\u{:04x}", u));
            }
        }
    }
    o
}

/// save_config for the keys the desktop fox changes: config.json is read fresh, those keys set, and written back.
pub fn update_config(cfg: &mut Cfg, f: impl Fn(&mut Value)) {
    f(&mut cfg.raw);
    let mut v = read_json("config.json");
    if !v.is_object() {
        v = Value::Object(Default::default());
    }
    f(&mut v);
    let dir = data_dir();
    let _ = std::fs::create_dir_all(&dir);
    let _ = std::fs::write(dir.join("config.json"), py_json_ascii(&v));
}

/// Read one of Zenko's data files fresh, change it and write it back the way zenko.py does (a .part file, then
/// replace), so only the keys we touch change, even if zenko.py also wrote it.
pub fn update_json(name: &str, f: impl FnOnce(&mut Value)) {
    let mut v = read_json(name);
    if !v.is_object() {
        v = Value::Object(Default::default());
    }
    f(&mut v);
    let dir = data_dir();
    let _ = std::fs::create_dir_all(&dir);
    let p = dir.join(name);
    let tmp = dir.join(format!("{name}.part"));
    if std::fs::write(&tmp, py_json(&v)).is_ok() {
        let _ = std::fs::rename(&tmp, &p);
    }
}

pub const FONT35: &[(char, [&str; 5])] = &[
    ('0', ["###", "#.#", "#.#", "#.#", "###"]),
    ('1', [".#.", "##.", ".#.", ".#.", "###"]),
    ('2', ["###", "..#", "###", "#..", "###"]),
    ('3', ["###", "..#", "###", "..#", "###"]),
    ('4', ["#.#", "#.#", "###", "..#", "..#"]),
    ('5', ["###", "#..", "###", "..#", "###"]),
    ('6', ["###", "#..", "###", "#.#", "###"]),
    ('7', ["###", "..#", "..#", "..#", "..#"]),
    ('8', ["###", "#.#", "###", "#.#", "###"]),
    ('9', ["###", "#.#", "###", "..#", "###"]),
    (':', [".", "#", ".", "#", "."]),
    ('A', [".#.", "#.#", "###", "#.#", "#.#"]),
    ('B', ["##.", "#.#", "##.", "#.#", "##."]),
    ('C', [".##", "#..", "#..", "#..", ".##"]),
    ('D', ["##.", "#.#", "#.#", "#.#", "##."]),
    ('E', ["###", "#..", "##.", "#..", "###"]),
    ('F', ["###", "#..", "##.", "#..", "#.."]),
    ('G', [".##", "#..", "#.#", "#.#", ".##"]),
    ('H', ["#.#", "#.#", "###", "#.#", "#.#"]),
    ('I', ["###", ".#.", ".#.", ".#.", "###"]),
    ('J', ["..#", "..#", "..#", "#.#", ".#."]),
    ('K', ["#.#", "#.#", "##.", "#.#", "#.#"]),
    ('L', ["#..", "#..", "#..", "#..", "###"]),
    ('M', ["#.#", "###", "###", "#.#", "#.#"]),
    ('N', ["##.", "#.#", "#.#", "#.#", "#.#"]),
    ('O', [".#.", "#.#", "#.#", "#.#", ".#."]),
    ('P', ["##.", "#.#", "##.", "#..", "#.."]),
    ('Q', [".#.", "#.#", "#.#", "##.", ".##"]),
    ('R', ["##.", "#.#", "##.", "#.#", "#.#"]),
    ('S', [".##", "#..", ".#.", "..#", "##."]),
    ('T', ["###", ".#.", ".#.", ".#.", ".#."]),
    ('U', ["#.#", "#.#", "#.#", "#.#", "###"]),
    ('V', ["#.#", "#.#", "#.#", "#.#", ".#."]),
    ('W', ["#.#", "#.#", "###", "###", "#.#"]),
    ('X', ["#.#", "#.#", ".#.", "#.#", "#.#"]),
    ('Y', ["#.#", "#.#", ".#.", ".#.", ".#."]),
    ('Z', ["###", "..#", ".#.", "#..", "###"]),
    (' ', ["..", "..", "..", "..", ".."]),
    ('-', ["...", "...", "###", "...", "..."]),
    ('.', [".", ".", ".", ".", "#"]),
    (',', ["..", "..", "..", ".#", "#."]),
    ('\'', ["#", "#", ".", ".", "."]),
    ('!', ["#", "#", "#", ".", "#"]),
    ('?', ["##.", "..#", ".#.", "...", ".#."]),
    ('&', [".#.", "#.#", ".#.", "#.#", ".##"]),
    ('(', [".#", "#.", "#.", "#.", ".#"]),
    (')', ["#.", ".#", ".#", ".#", "#."]),
    ('/', ["..#", "..#", ".#.", "#..", "#.."]),
    ('~', ["..#.", ".#.#", ".#..", "##..", "##.."]),
];

// ---- where Zenko keeps her things (%APPDATA%\FoxBuddy, same as zenko.py) -------------------------------
pub fn data_dir() -> PathBuf {
    let base = std::env::var_os("APPDATA")
        .or_else(|| std::env::var_os("HOME"))
        .unwrap_or_default();
    PathBuf::from(base).join("FoxBuddy")
}
pub fn read_json(name: &str) -> Value {
    std::fs::read_to_string(data_dir().join(name))
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .map(|mut v: Value| {
            if name == "zenko_life.json" {
                study::apply_pending(&mut v);
                guard::apply_screen(&mut v);
            }
            v
        })
        .unwrap_or_else(|| {
            let mut v = Value::Null;
            if name == "zenko_life.json" {
                study::apply_pending(&mut v);
            }
            v
        })
}

/// The settings the desktop fox reads (zenko.py DEFAULTS for anything missing).
#[derive(Clone, Debug)]
pub struct Cfg {
    pub raw: Value,
}
impl Cfg {
    pub fn load() -> Cfg {
        Cfg {
            raw: read_json("config.json"),
        }
    }
    pub fn from(v: Value) -> Cfg {
        Cfg { raw: v }
    }
    pub fn s(&self, k: &str, d: &str) -> String {
        self.raw
            .get(k)
            .and_then(|v| v.as_str())
            .unwrap_or(d)
            .to_string()
    }
    pub fn b(&self, k: &str, d: bool) -> bool {
        match self.raw.get(k) {
            Some(Value::Bool(b)) => *b,
            Some(Value::Number(n)) => n.as_f64().unwrap_or(0.0) != 0.0,
            Some(Value::Null) | None => d,
            Some(Value::String(s)) => !s.is_empty(),
            Some(_) => d,
        }
    }
    pub fn f(&self, k: &str, d: f64) -> f64 {
        match self.raw.get(k) {
            Some(Value::Number(n)) => n.as_f64().unwrap_or(d),
            Some(Value::String(s)) => s.parse().unwrap_or(d),
            _ => d,
        }
    }
    pub fn list(&self, k: &str) -> Vec<String> {
        self.raw
            .get(k)
            .and_then(|v| v.as_array())
            .map(|a| {
                a.iter()
                    .filter_map(|x| x.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default()
    }
}

pub fn today_iso() -> String {
    // local date via the C runtime-free civil-from-days on local time offset (see platform `local_now`)
    let (y, m, d, _, _, _) = local_now();
    format!("{y:04}-{m:02}-{d:02}")
}

/// Local wall-clock time (year, month, day, hour, minute, second).
pub fn local_now() -> (i32, u32, u32, u32, u32, u32) {
    let secs = unix_now().floor() as i64 + local_offset_secs();
    let days = secs.div_euclid(86400);
    let sod = secs.rem_euclid(86400) as u32;
    let (y, m, d) = civil_from_days(days);
    (y, m, d, sod / 3600, sod / 60 % 60, sod % 60)
}
/// datetime.now().weekday(): Monday = 0
pub fn local_weekday() -> u32 {
    let secs = unix_now().floor() as i64 + local_offset_secs();
    (secs.div_euclid(86400) + 3).rem_euclid(7) as u32
}

pub fn civil_from_days_pub(d: i64) -> (i32, u32, u32) {
    civil_from_days(d)
}

pub fn local_offset_pub() -> i64 {
    local_offset_secs()
}

fn civil_from_days(z: i64) -> (i32, u32, u32) {
    let z = z + 719468;
    let era = z.div_euclid(146097);
    let doe = z.rem_euclid(146097);
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    ((if m <= 2 { y + 1 } else { y }) as i32, m, d)
}
#[cfg(windows)]
fn local_offset_secs() -> i64 {
    use windows_sys::Win32::System::Time::{GetTimeZoneInformation, TIME_ZONE_INFORMATION};
    unsafe {
        let mut tz: TIME_ZONE_INFORMATION = std::mem::zeroed();
        let r = GetTimeZoneInformation(&mut tz);
        let bias = tz.Bias
            + if r == 2 {
                tz.DaylightBias
            } else if r == 1 {
                tz.StandardBias
            } else {
                0
            };
        -(bias as i64) * 60
    }
}
#[cfg(not(windows))]
fn local_offset_secs() -> i64 {
    std::env::var("ZK_TZ_OFFSET")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(0)
}

pub fn stats() -> Value {
    read_json("stats.json")
}

// ---- tiny RNG (Python's random.uniform; the exact sequence never mattered) -----------------------------
pub struct Rng(u64);
impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(seed | 1)
    }
    pub fn next(&mut self) -> f64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        (self.0 >> 11) as f64 / (1u64 << 53) as f64
    }
    pub fn uniform(&mut self, a: f64, b: f64) -> f64 {
        a + (b - a) * self.next()
    }
}

pub struct Session {
    pub task: String,
    pub planned: f64,
    pub elapsed: f64,
    pub dist: f64,
    pub away: f64,
    pub paused: bool,
    pub idle_paused: bool,
    pub last_motiv: f64,
    pub half: bool,
    pub final_: bool,
    pub closed: i64,
    /// the timetable item this session came from (index into Companion::tt)
    pub item: Option<usize>,
}

impl Session {
    pub fn new(task: &str, minutes: f64) -> Session {
        Session { task: task.into(), planned: minutes * 60.0, elapsed: 0.0, dist: 0.0, away: 0.0, paused: false,
                  idle_paused: false, last_motiv: 0.0, half: false, final_: false, closed: 0, item: None }
    }
}

pub struct Mote {
    pub x: i32,
    pub y: i32,
    pub col: [u8; 3],
    pub born: f64,
    pub life: f64,
}

/// The finished window picture (RGBA, straight alpha) plus where the fox sits in it.
pub struct Frame {
    pub w: usize,
    pub h: usize,
    pub px: Vec<[u8; 4]>,
}

pub struct Companion {
    pub cfg: Cfg,
    pub u: f64,
    pub z: i32,
    pub colw: i32,
    pub w: i32,
    pub h: i32,
    pub fox_y: i32,
    pub pal: Palette,
    look: Look,
    robe_cur: Option<String>,
    images: HashMap<String, Image>,
    rng: Rng,
    pub session: Option<Session>,
    pub sleeping: bool,
    pub mood_override: Option<(String, f64)>,
    next_blink: f64,
    blink_until: f64,
    next_ear: f64,
    ear_until: f64,
    next_lick: f64,
    lick_until: f64,
    motes: Vec<Mote>,
    tails_n: (f64, i32),
    hrs: (f64, f64),
    pub hd: bool,
    pub last_state: String,
    pub play: Option<Play>,
    pub sulk_until: f64,
    /// what she did that has no event of its own (apply_look, music_apply): tests compare it with zenko.py's stubs
    pub trace: Vec<String>,
    /// tests: the omikuji's random.Random() seed (None = from the OS)
    pub omikuji_seed: Option<u64>,
    /// stage 3g: sleep guard, water, screen time, pop quiz, the minute checks
    pub guard: guard::Guard,
    /// self.quiz: the flashcard quiz running in chat
    pub quiz: Option<lifechat::Quiz>,
    /// the host's open_app(name, as typed) (Windows: Start-menu apps, sites); None = not on Windows
    pub open_app_hook: Option<fn(&str, &str) -> String>,
    /// the last file dropped on her: (name, text)
    pub last_file: Option<(String, String)>,
    pub pokes: Vec<f64>,
    pub events: Vec<Ev>,
    pub delayed: Vec<(f64, Ev)>,
    pub pose: Option<Pose>,
    pub watch: watch::Watch,
    pub brk: Option<watch::Brk>,
    pub life: life::Life,
    pub env: life::Env,
    /// where roaming wants the window this frame (the window side moves it)
    pub win_move: Option<(i32, i32)>,
    pub lockin: Option<lockin::Lockin>,
    pub lock_eating: bool,
    pub esc_t: f64,
    pub rewards: rewards::Rewards,
    /// the timetable (load_tt at start-up), check_timetable's throttle and what it already announced
    pub tt: Vec<serde_json::Value>,
    pub tt_last: f64,
    pub announced: std::collections::HashSet<String>,
    pub pomo: Option<tt::Pomo>,
    pub pomo_id: u64,
    pub music: music::Music,
    pub ema_t0: f64,
    pub belly_t0: f64,
    /// battery_status(): (percent, charging), None on a desktop PC
    pub battery: Option<(i64, bool)>,
    pub batt_warned: Vec<i64>,
    /// where draw_battery puts it this frame: (dx, dy, curled)
    pub batt_at: (i32, i32, bool),
    pub slow: slow::Slow,
    /// chat_log: (who, text), the last 200
    pub chat_log: Vec<(String, String)>,
    /// her chat brain's state (chat.rs)
    pub brain: chat::Brain,
    /// the study planner's state (plan.rs)
    pub planner: plan::Planner,
    /// the 急 delay (books.rs)
    pub books: books::BooksState,
    /// zenko8's study tracker (study.rs: self.st)
    pub study: study::Study,
    /// how many lines chat_line has ever added (the chat page types the new ones)
    pub chat_n: u64,
    /// the hub page on show while the hub window is open and not minimised (gym_look_on, the sidebar fox)
    pub hub_page: Option<String>,
    /// z19: a session was completed with the hub open: the petal burst starts at this time
    pub glow_burst: Option<f64>,
}

impl Companion {
    pub fn new(cfg: Cfg, u: f64, now: f64) -> Companion {
        let mut c = Companion {
            cfg,
            u,
            z: 1,
            colw: 0,
            w: 0,
            h: 0,
            fox_y: 0,
            pal: Palette::empty(),
            look: Look::default(),
            robe_cur: None,
            images: HashMap::new(),
            rng: Rng::new(now.to_bits() ^ 0x9E3779B97F4A7C15),
            session: None,
            sleeping: false,
            mood_override: None,
            next_blink: now + 3.0,
            blink_until: 0.0,
            next_ear: now + 5.0,
            ear_until: 0.0,
            next_lick: now + 14.0,
            lick_until: 0.0,
            motes: vec![],
            tails_n: (-1e9, 3),
            hrs: (-1e9, 0.0),
            hd: true,
            last_state: String::new(),
            play: None,
            sulk_until: 0.0,
            trace: vec![],
            omikuji_seed: None,
            guard: guard::Guard::new(now),
            quiz: None,
            open_app_hook: None,
            last_file: None,
            pokes: vec![],
            events: vec![],
            delayed: vec![],
            pose: None,
            watch: Default::default(),
            brk: None,
            life: life::Life::default(),
            env: life::Env::default(),
            win_move: None,
            lockin: None,
            lock_eating: false,
            esc_t: 0.0,
            rewards: Default::default(),
            tt: tt::load_tt(),
            tt_last: 0.0,
            announced: Default::default(),
            pomo: None,
            pomo_id: 0,
            music: Default::default(),
            ema_t0: 0.0,
            belly_t0: 0.0,
            battery: None,
            batt_warned: vec![],
            batt_at: (0, 0, false),
            slow: Default::default(),
            chat_log: vec![],
            planner: plan::Planner::default(),
            books: books::BooksState::default(),
            study: study::Study::new(now),
            brain: chat::Brain::new(now.to_bits()),
            chat_n: 0,
            hub_page: None,
            glow_burst: None,
        };
        c.compute_dims();
        c.apply_look();
        c.life.roam_next = now + 45.0 + 45.0 * c.rand();
        c.slow_init(now);
        c
    }

    /// random.random() for picks that never needed to match zenko.py's sequence
    pub fn rand(&mut self) -> f64 {
        self.rng.next()
    }

    pub fn compute_dims(&mut self) {
        let u = self.u;
        self.z = 1.max(((self.cfg.f("size", 4.0) - 2.0) * u).round_ties_even() as i32);
        let (fw, fh) = (W_ * self.z, H_ * self.z);
        self.colw = (250.0 * u) as i32;
        self.w = self.colw + fw;
        self.h = (fh + (20.0 * u) as i32).max((230.0 * u) as i32);
        self.fox_y = self.h - fh - (2.0 * u) as i32;
    }

    pub fn apply_look(&mut self) {
        let c = &self.cfg;
        self.look = Look {
            ears: c.s("ear_style", "pointy"),
            blush: c.s("blush_style", "soft"),
        };
        self.pal = make_pal(&LookCfg {
            theme: c.s("theme", "snow"),
            coat: c.s("coat", ""),
            eye_color: c.s("eye_color", ""),
            foxfire_color: c.s("foxfire_color", ""),
            scarf: c.s("scarf", ""),
            robe: self.robe_cur.clone().unwrap_or_default(),
        });
        self.hd = c.b("hd_art", true);
        self.images.clear();
        self.tails_n = (-1e9, 3);
    }

    fn today_focus_minutes(&self) -> i64 {
        (stats()
            .get(today_iso())
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0)
            / 60.0)
            .floor() as i64
    }

    pub fn look_ref(&self) -> &Look {
        &self.look
    }

    /// self.images = {}: the pictures are drawn again (her look changed)
    pub fn clear_images(&mut self) {
        self.images.clear();
    }

    /// small_fox(1): the fox with her tails as a little picture for headers and dialogs (see-through background).
    pub fn small_fox(&mut self, now: f64) -> Image {
        self.small_fox_z(now, 1)
    }

    /// small_fox(zoom) (HD smoothing from zoom 2 while hd_art is on)
    pub fn small_fox_z(&mut self, now: f64, zoom: usize) -> Image {
        let n = self.tail_count(now);
        let tails = core::build_tails(n, 0, 8, false);
        let body = sit::build_frame("happy", 0, None, &[], 0, 0, &self.look);
        let hd = self.cfg.b("hd_art", true);
        sprite_photo(&[&tails, &body], &self.pal, zoom, hd)
    }

    /// A zenko grows tails with wisdom: 3 to start, one more for every 20 focused minutes today (up to 9).
    pub fn tail_count(&mut self, now: f64) -> i32 {
        let (t, n) = self.tails_n;
        if now - t < 20.0 {
            return n;
        }
        let mode = self.cfg.s("tails", "grow");
        let n = if !mode.is_empty() && mode.chars().all(|c| c.is_ascii_digit()) {
            mode.parse::<i32>().unwrap_or(3).clamp(1, 9)
        } else {
            (3 + self.today_focus_minutes() / 20).clamp(3, 9) as i32
        };
        self.tails_n = (now, n);
        n
    }

    fn total_hours(&mut self, now: f64) -> f64 {
        if now - self.hrs.0 > 120.0 {
            let v = stats()
                .as_object()
                .map(|o| o.values().filter_map(|v| v.as_f64()).sum::<f64>())
                .unwrap_or(0.0)
                / 3600.0;
            self.hrs = (now, v);
        }
        self.hrs.1
    }

    fn season(m: u32) -> &'static str {
        if [12, 1, 2].contains(&m) {
            "winter"
        } else if [3, 4, 5].contains(&m) {
            "spring"
        } else if [6, 7, 8].contains(&m) {
            "summer"
        } else {
            "autumn"
        }
    }

    /// zenko19 / zenko22 z19_robe: the robe she wears today.
    fn robe(&mut self, now: f64) -> Option<String> {
        let mode = self.cfg.s("robe", "auto");
        if mode.is_empty() || mode == "none" || self.gym_look_on() {
            return None;
        }
        if let Some(r) = ROBES.iter().find(|r| r.key == mode) {
            if r.hours == 0 && !mode.starts_with("kimono") && mode != "yukata" && mode != "uniform"
            {
                return Some(mode); // travel outfit picked in Customize (only offered once unlocked)
            }
        }
        let (_, m, d, _, _, _) = local_now();
        let season_k = format!("kimono_{}", Self::season(m));
        let want = if mode == "auto" {
            // exam-season uniform needs the deadline list (not ported yet), so auto = yukata in summer festival season, else the seasonal kimono
            if (m, d) >= (7, 15) && (m, d) <= (8, 31) {
                "yukata".to_string()
            } else {
                season_k.clone()
            }
        } else if mode == "kimono" {
            season_k.clone()
        } else {
            mode.clone()
        };
        let r = ROBES.iter().find(|r| r.key == want)?;
        let hrs = self.total_hours(now);
        if hrs < r.hours as f64 {
            if mode == "auto" {
                let k = ROBES.iter().find(|r| r.key == season_k)?;
                return if hrs >= k.hours as f64 {
                    Some(season_k)
                } else {
                    None
                };
            }
            return None;
        }
        Some(want)
    }

    /// ZenkoGym.gym_look_on: hachimaki and towel while the Exercise tab is open, unless music is playing
    pub fn gym_look_on(&self) -> bool {
        self.hub_page.as_deref() == Some("exercise") && self.cfg.b("gym_look", true)
            && !self.music.player.as_ref().is_some_and(|p| p.playing)
    }

    pub fn accessories(&mut self, now: f64) -> Vec<String> {
        let mut acc: Vec<String> = vec![];
        let out = self.cfg.list("outfit");
        if (self.session.is_some() && self.cfg.b("glasses_in_session", true))
            || out.iter().any(|o| o == "glasses_always")
        {
            acc.push("glasses".into());
        }
        for a in &out {
            if ["flower", "bow", "crown", "headband", "mask"].contains(&a.as_str()) {
                acc.push(a.clone());
            }
        }
        if let Some(a) = self.lock_extra_acc() {
            if !acc.iter().any(|x| x == a) {
                acc.push(a.into());
            }
        }
        if self.music.player.as_ref().is_some_and(|p| p.playing) && self.cfg.b("headphones_with_music", true) {
            acc.push("headphones".into());
        }
        if !self.cfg.s("scarf", "").is_empty() {
            acc.push("scarf".into());
        }
        if self.gym_look_on() {
            // the gym look while the Exercise tab is open
            acc.retain(|a| a != "headband" && a != "towel");
            acc.push("headband".into());
            acc.push("towel".into());
        }
        let rb = self.robe(now);
        if rb != self.robe_cur {
            self.robe_cur = rb.clone();
            self.apply_look();
        }
        if let Some(r) = rb {
            acc.push(r);
        }
        acc
    }

    pub fn current_mood(&self, now: f64) -> String {
        if let Some((m, until)) = &self.mood_override {
            if now < *until {
                return m.clone();
            }
        }
        if self.sleeping || self.brk.is_some() {
            return "sleep".into();
        }
        if let Some(s) = &self.session {
            return if s.paused { "sad".into() } else { "focus".into() };
        }
        "idle".into()
    }

    fn idle_moves(&mut self, now: f64, mood: &str) -> (i32, i32) {
        if !["idle", "focus", "sad"].contains(&mood) {
            return (0, 0);
        }
        let live = match self.cfg.s("idle_lively", "normal").as_str() {
            "calm" => 0.6,
            "lively" => 1.6,
            _ => 1.0,
        };
        let s = (now * if mood == "focus" { 0.55 } else { 0.8 } * live).sin();
        let mut hx = if self.cfg.b("idle_sway", true) {
            if s > 0.55 {
                1
            } else if s < -0.55 {
                -1
            } else {
                0
            }
        } else {
            0
        };
        if now >= self.next_ear {
            self.ear_until = now
                + if self.cfg.b("idle_ears", true) {
                    0.32
                } else {
                    0.0
                };
            self.next_ear = now + self.rng.uniform(4.0, 10.0) / live;
        }
        let ear = if now < self.ear_until && (self.ear_until - now) > 0.12 {
            1
        } else {
            0
        };
        if !self.cfg.b("idle_lick", true) {
            self.lick_until = 0.0;
            self.next_lick = now + 30.0;
        }
        if mood == "idle" && now >= self.next_lick && now >= self.lick_until {
            self.lick_until = now + self.rng.uniform(1.8, 2.8);
            self.next_lick = now + self.rng.uniform(16.0, 34.0);
        }
        if now < self.lick_until {
            hx = 0;
        }
        (hx, ear)
    }

    /// (eye shift x, eye shift y, snout shift x, snout shift y) so the fox looks at your mouse.
    pub fn look_vec(
        &self,
        mood: &str,
        pointer: (i32, i32),
        win: (i32, i32),
    ) -> (i32, i32, i32, i32) {
        if !self.cfg.b("look_at_mouse", true) || mood == "sleep" {
            return (0, 0, 0, 0);
        }
        let cx = win.0 + self.colw + 36 * self.z;
        let cy = win.1 + self.fox_y + 30 * self.z;
        let (dx, dy) = ((pointer.0 - cx) as f64, (pointer.1 - cy) as f64);
        let t = 70.0 * self.u;
        let lx = if dx > t {
            1
        } else if dx < -t {
            -1
        } else {
            0
        };
        let ly = if dy > 1.6 * t {
            1
        } else if dy < -1.6 * t {
            -1
        } else {
            0
        };
        let sx = lx * if dx.abs() > 5.0 * t { 2 } else { 1 };
        let sy = if ly < 0 { -1 } else { 0 };
        (lx, ly, sx, sy)
    }

    /// zenko14 belly_text: every minute or so, between the clock, the plaque scrolls 'D-152 TO EXAMS'
    pub fn belly_text(&mut self, now: f64) -> String {
        if self.session.is_some() || self.brk.is_some() || !self.cfg.b("ema_countdown", true) {
            self.ema_t0 = 0.0;
            return self.belly_text_base(now);
        }
        let d = self.exam_days();
        if !(0..=400).contains(&d) {
            return self.belly_text_base(now);
        }
        let txt = if d == 0 { "TODAY: EXAMS! GANBATTE".to_string() } else { format!("D-{d} TO EXAMS") };
        let dur = (txt.chars().count() as f64 * 4.0 + 8.0) * 0.11 + 0.3;
        if self.ema_t0 != 0.0 && now - self.ema_t0 < dur {
            self.belly_t0 = self.ema_t0;
            return txt;
        }
        if now - self.ema_t0 > 55.0 + dur && (unix_now() as i64) % 60 < 2 {
            self.ema_t0 = now;
            self.belly_t0 = now;
            return txt;
        }
        self.belly_text_base(now)
    }

    /// exam_days(): days until cfg exam_date (default 2027-04-23)
    pub fn exam_days(&self) -> i64 {
        let parse = |s: &str| -> Option<i64> {
            let p: Vec<&str> = s.split('-').collect();
            if p.len() != 3 || p[0].len() != 4 || p[1].len() != 2 || p[2].len() != 2 {
                return None;
            }
            let (y, m, d): (i64, i64, i64) = (p[0].parse().ok()?, p[1].parse().ok()?, p[2].parse().ok()?);
            let mdays = [31, if (y % 4 == 0 && y % 100 != 0) || y % 400 == 0 { 29 } else { 28 }, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
            if !(1..=12).contains(&m) || d < 1 || d > mdays[(m - 1) as usize] {
                return None;
            }
            Some(rewards::days_from_civil(y, m, d))
        };
        let raw = match self.cfg.raw.get("exam_date") {
            Some(Value::String(s)) if !s.is_empty() => s.clone(),
            Some(Value::Null) | None => String::new(),
            Some(Value::String(_)) => String::new(),
            Some(v) => v.to_string(),
        };
        let raw = if raw.is_empty() { "2027-04-23".to_string() } else { raw };
        let ex = parse(&raw).or_else(|| parse("2027-04-23")).unwrap();
        let (y, m, d, _, _, _) = local_now();
        ex - rewards::days_from_civil(y as i64, m as i64, d as i64)
    }

    pub fn belly_text_base(&self, now: f64) -> String {
        if let Some(b) = &self.brk {
            let left = (b.planned - b.elapsed).max(0.0) as i64;
            return format!("{:02}:{:02}", left / 60, left % 60);
        }
        if let Some(s) = &self.session {
            if s.paused && (unix_now() as i64) % 2 != 0 {
                return String::new();
            }
            let _ = now;
            let left = (s.planned - s.elapsed).max(0.0) as i64;
            if left >= 3600 {
                return format!("{}:{:02}", left / 3600, left % 3600 / 60);
            }
            return format!("{:02}:{:02}", left / 60, left % 60);
        }
        let (_, _, _, h, m, _) = local_now();
        if self.cfg.b("clock_24h", true) {
            format!("{h:02}:{m:02}")
        } else {
            format!("{}:{:02}", if h % 12 == 0 { 12 } else { h % 12 }, m)
        }
    }

    fn img(
        &mut self,
        key: String,
        make: impl FnOnce(&Look) -> Vec<Grid>,
        pal: Option<Palette>,
    ) -> &Image {
        if !self.images.contains_key(&key) {
            if self.images.len() > 360 {
                self.images.clear();
            }
            let layers = make(&self.look);
            let refs: Vec<&Grid> = layers.iter().collect();
            let p = pal.unwrap_or_else(|| self.pal.clone());
            let im = sprite_photo(&refs, &p, self.z as usize, self.hd);
            self.images.insert(key.clone(), im);
        }
        &self.images[&key]
    }

    fn orb_pal(&self, style: &str) -> Palette {
        let mut pal = self.pal.clone();
        let hx = |k: u8| pal.hex(k).unwrap_or_else(|| "#000000".into());
        if style == "angry" {
            pal.set(b'A', "#fff3c4");
            pal.set(b'I', "#ffb02e");
            pal.set(b'U', "#ff3b1f");
        } else if style == "sleep" {
            let (i, u) = (hx(b'I'), hx(b'U'));
            pal.set(b'A', &i);
            pal.set(b'I', &u);
            pal.set(b'U', &mixhex(&u, "#000000", 0.35));
        }
        pal
    }

    /// One animation step (zenko.py animate, every 60 ms): returns the whole window picture.
    pub fn frame(&mut self, now: f64, pointer: (i32, i32), win: (i32, i32)) -> Frame {
        let mood = self.current_mood(now);
        let z = self.z;
        let speed = if mood == "alert" {
            0.07
        } else if mood == "sleep" {
            0.32
        } else {
            0.15
        };
        let tframe = ((now / speed) as i64 % 8) as i32;
        let mut frame = ((now / 0.6) as i64 % 2) as i32;
        let (mut dx, dy) = (0, 0);
        let mut state = mood.clone();
        let (mut hx, ear) = self.idle_moves(now, &mood);
        if mood == "idle" || mood == "focus" {
            if now >= self.next_blink {
                self.blink_until = now
                    + if self.cfg.b("idle_blink", true) {
                        0.15
                    } else {
                        0.0
                    };
                self.next_blink = now + self.rng.uniform(2.5, 5.5);
            }
            if now < self.lick_until && mood == "idle" {
                state = if (now / 0.22) as i64 % 2 != 0 {
                    "lick".into()
                } else {
                    "lick2".into()
                };
            } else if now < self.blink_until {
                state = "blink".into();
            }
        } else if mood == "alert" {
            state = if (now / 0.3) as i64 % 2 == 0 {
                "alert".into()
            } else {
                "alert2".into()
            };
            dx = if (now / 0.08) as i64 % 2 != 0 { z } else { -z };
        } else if mood == "happy" {
            hx = if (now / 0.16) as i64 % 2 != 0 { 1 } else { -1 };
        }
        let mut look = self.look_vec(&mood, pointer, win);
        let mut acc = self.accessories(now);
        let mo = self.mood_override.as_ref().is_some_and(|(_, u)| now < *u);
        let mut curled = mood == "sleep" && self.sleeping && self.brk.is_none() && !mo && self.cfg.b("curl_when_away", true);
        if curled {
            state = "curl".into();
            look = (0, 0, 0, 0);
            frame = ((now / 1.6) as i64 % 2) as i32;
            acc.retain(|a| a == "headphones");
        }
        let n = self.tail_count(now);
        let mut st = Pose { state, hx, ear, look, frame, acc, curled, tframe, n, dxf: 0, dyf: 0, hide_fire: false };
        let mut prop = None;
        if (self.play.is_some() || self.sulk_until > 0.0) && self.brk.is_none() {
            if curled && self.play.is_some() {
                curled = false;
                st.curled = false;
                st.acc = self.accessories(now);
            }
            prop = self.play_pose(now, &mut st);
        }
        self.life_pose(now, &mut st, win, pointer);
        self.wx_step(now);
        self.pose = Some(st.clone());
        let Pose { state, hx, ear, look, frame, acc, tframe, .. } = st.clone();
        let (dx, dy) = (dx + st.dxf * z, dy + st.dyf * z);
        self.last_state = state.clone();
        let (fw, fh) = (self.w as usize, self.h as usize);
        let mut out = Frame {
            w: fw,
            h: fh,
            px: vec![[0, 0, 0, 0]; fw * fh],
        };
        self.wx_draw(&mut out);
        let (fx0, fy0) = (self.colw + dx, self.fox_y + dy);
        let accs: Vec<&str> = acc.iter().map(|s| s.as_str()).collect();
        let acck = accs.join(",");
        let walking = state.starts_with("qwalk") || state.starts_with("rig");
        let curl_pic = state == "curl";
        self.batt_at = (dx, dy, walking || curl_pic);
        if !curl_pic && !walking {
            let im = self
                .img(
                    format!("tails|{n}|{tframe}"),
                    |_| vec![core::build_tails(n, tframe, 8, true)],
                    None,
                )
                .clone();
            blit(&mut out, &im, fx0, fy0);
        }
        let key = if curl_pic {
            format!("curl|{frame}|{acck}|{n}")
        } else if walking {
            format!("{state}|{acck}|{n}")
        } else {
            format!("{state}|{frame}|{look:?}|{acck}|{hx}|{ear}")
        };
        let st2 = state.clone();
        let accv = accs.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        let im = self
            .img(
                key,
                move |lk| {
                    let a: Vec<&str> = accv.iter().map(|s| s.as_str()).collect();
                    if st2 == "curl" {
                        vec![curl::build_curl(frame, &a, n)]
                    } else if st2.starts_with("qwalk") || st2.starts_with("rig") {
                        vec![rig::rig_rows(&st2, n, &a, lk)]
                    } else {
                        vec![sit::build_frame(&st2, frame, Some(look), &a, hx, ear, lk)]
                    }
                },
                None,
            )
            .clone();
        blit(&mut out, &im, fx0, fy0);
        if !walking {
            if self.tablet_shown(&state) {
                // the music tablet in her paws (draw_tablet) instead of the ema timer
                let rows = self.tablet_rows(now);
                let mut pal = self.pal.clone();
                for (k, v) in music::ldata()["TAB_PAL"].as_object().unwrap() {
                    pal.set(k.as_bytes()[0], v.as_str().unwrap());
                }
                let key = format!("tablet|{}", rows.join("/"));
                let im = self.img(key, |_| vec![Grid::from_rows(&rows)], Some(pal)).clone();
                blit(&mut out, &im, self.colw + dx + music::TAB_X * self.z, self.fox_y + dy + (music::TAB_Y - 3) * self.z);
            } else if !curl_pic {
                self.draw_belly(&mut out, now, dx, dy);
            }
        }
        if !st.hide_fire {
            self.draw_foxfire(&mut out, now, &mood, curled && !walking);
        }
        if let Some((name, x, y, eaten)) = prop {
            if let Some(rows) = play::prop_rows(&name) {
                let cut = if eaten > 0.0 { (rows[0].len() as f64 * eaten) as usize } else { 0 };
                let z = self.z as usize;
                let hd = self.hd;
                let key = format!("prop|{name}|{cut}");
                if !self.images.contains_key(&key) {
                    let rr: Vec<String> = rows.iter().map(|r| ".".repeat(cut) + &r[cut.min(r.len())..]).collect();
                    let g = Grid::from_rows(&rr);
                    self.images.insert(key.clone(), sprite_photo(&[&g], &play::prop_pal(), z, hd));
                }
                let im = self.images[&key].clone();
                blit(&mut out, &im, self.colw + (x + st.dxf) * self.z, self.fox_y + y * self.z);
            }
        }
        self.visitor_draw(&mut out, now);
        out
    }

    fn draw_belly(&mut self, out: &mut Frame, now: f64, dx: i32, dy: i32) {
        let text = self.belly_text(now);
        let mut cols: Vec<[bool; 5]> = vec![];
        for ch in text.chars() {
            if let Some((_, g)) = FONT35.iter().find(|(c, _)| *c == ch) {
                for cx in 0..g[0].len() {
                    let mut col = [false; 5];
                    for r in 0..5 {
                        col[r] = g[r].as_bytes()[cx] == b'#';
                    }
                    cols.push(col);
                }
                cols.push([false; 5]);
            }
        }
        cols.pop();
        let w = 17usize;
        let window: Vec<Option<[bool; 5]>> = if cols.len() <= w {
            let mut v: Vec<Option<[bool; 5]>> = vec![None; (w - cols.len()) / 2];
            v.extend(cols.iter().map(|c| Some(*c)));
            v
        } else {
            let mut lp = cols.clone();
            lp.extend(std::iter::repeat([false; 5]).take(8));
            let off = ((now - self.belly_t0) / 0.11) as usize % lp.len();
            (0..w).map(|i| Some(lp[(off + i) % lp.len()])).collect()
        };
        let ink = self
            .pal
            .get(b'b')
            .or(self.pal.get(b'B'))
            .unwrap_or([0, 0, 0]);
        let z2 = self.z * 2;
        for (i, col) in window.iter().take(w).enumerate() {
            let Some(col) = col else { continue };
            for r in 0..5 {
                if col[r] {
                    let px = self.colw + dx + BELLY_X * self.z + i as i32 * z2;
                    let py = self.fox_y + dy + BELLY_Y * self.z + r as i32 * z2;
                    fill(out, px, py, z2, z2, ink);
                }
            }
        }
    }

    fn draw_foxfire(&mut self, out: &mut Frame, now: f64, mood: &str, curled: bool) {
        if !self.cfg.b("foxfire", true) {
            self.motes.clear();
            return;
        }
        let z = self.z;
        let style = if mood == "alert" {
            "angry"
        } else if mood == "sleep" {
            "sleep"
        } else {
            "calm"
        };
        let spd = match style {
            "angry" => 2.6,
            "sleep" => 0.35,
            _ => 1.0,
        } * if mood == "happy" { 1.8 } else { 1.0 };
        let anchors: &[(f64, f64, f64)] = if !curled {
            &[(2.0, 16.0, 0.0), (66.0, 2.0, 2.1), (98.0, 22.0, 4.2)]
        } else {
            &[(88.0, 30.0, 0.0), (6.0, 34.0, 3.0)]
        };
        // motes first (they sit under the wisps)
        self.motes.retain(|m| now - m.born <= m.life);
        if self.motes.len() < if curled { 2 } else { 5 } && self.rng.next() < 0.25 {
            let (x, y) = if curled {
                (self.rng.uniform(40.0, 100.0), self.rng.uniform(42.0, 60.0))
            } else {
                let a = self.rng.uniform(5.0, 88.0).to_radians();
                let r = self.rng.uniform(30.0, 52.0);
                (TAIL_BASE.0 + r * a.cos(), TAIL_BASE.1 - r * a.sin())
            };
            let pick = (self.rng.next() * 3.0) as usize;
            let col = [self.pal.get(b'I'), self.pal.get(b'A'), self.pal.get(b'x')][pick.min(2)]
                .unwrap_or([255, 243, 184]);
            let life = self.rng.uniform(0.3, 0.9);
            self.motes.push(Mote {
                x: self.colw + x as i32 * z,
                y: self.fox_y + y as i32 * z,
                col,
                born: now,
                life,
            });
        }
        for m in &self.motes {
            fill(out, m.x, m.y, z, z, m.col);
        }
        for i in 0..3usize {
            if i >= anchors.len() || (style == "sleep" && i > 0 && !curled) {
                continue;
            }
            let (ax, ay, ph) = anchors[i];
            let t = now * spd + ph;
            let fi = i as f64;
            let x = ax
                + 3.0 * (t * 0.9).sin()
                + if style == "angry" {
                    1.5 * (now * 23.0 + fi).sin()
                } else {
                    0.0
                };
            let y = ay + 2.5 * (t * 1.7 + fi).sin()
                - if style == "sleep" {
                    2.0 * (t * 0.5).sin()
                } else {
                    0.0
                };
            let fr =
                ((now * if style == "angry" { 11.0 } else { 6.0 } + fi * 2.0) as i64 % 4) as i32;
            let pal = self.orb_pal(style);
            let im = self
                .img(
                    format!("orb|{style}|{fr}"),
                    |_| vec![curl::foxfire_rows(fr)],
                    Some(pal),
                )
                .clone();
            blit(
                out,
                &im,
                self.colw + x as i32 * z,
                self.fox_y + y as i32 * z,
            );
        }
    }
}

pub fn blit(out: &mut Frame, im: &Image, x0: i32, y0: i32) {
    for y in 0..im.h as i32 {
        let oy = y0 + y;
        if oy < 0 || oy >= out.h as i32 {
            continue;
        }
        for x in 0..im.w as i32 {
            let ox = x0 + x;
            if ox < 0 || ox >= out.w as i32 {
                continue;
            }
            let p = im.px[(y as usize) * im.w + x as usize];
            if p[3] != 0 {
                out.px[oy as usize * out.w + ox as usize] = p;
            }
        }
    }
}

pub fn fill(out: &mut Frame, x0: i32, y0: i32, w: i32, h: i32, c: [u8; 3]) {
    for y in y0.max(0)..(y0 + h).min(out.h as i32) {
        for x in x0.max(0)..(x0 + w).min(out.w as i32) {
            out.px[y as usize * out.w + x as usize] = [c[0], c[1], c[2], 255];
        }
    }
}

pub fn is_robe_name(a: &str) -> bool {
    is_robe(a)
}

/// tests: change zenko_life.json the way the app does
pub fn test_life_do(fox: &mut Companion, f: impl FnOnce(&mut Value)) {
    fox.life_do(f)
}
