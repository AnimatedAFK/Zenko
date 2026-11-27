//! The hub's Settings page: zenko.py hub_build_settings with every layer's cards (Appearance, Focus and breaks,
//! Background music, Internet lookups, Online brain, General, the voice / Spotify / presets / window cards, talk,
//! rules, study helpers, updates, Zenko19's sky card and section chips, Zenko20/21's speed and pixel-art cards,
//! Zenko22's linked apps and Town square, Zenko23's ema planning, lock-in, desktop life, wellbeing, the planner and
//! her needs), in the same order, with the same widgets, fonts and padding.
//!
//! Every control's command is a string the hub turns into a config change: "set_chk:<key>" flips a checkbox,
//! "set_rad:<key>:<s|i|f>:<value>" picks a radio, "set_btn:<what>" is a button, "set_menu:<key>" opens a menu.

use crate::hubwin::{canvas_of, hub_title_draw};
use crate::pagekit::{Faces, Scroller, WrapState, hub_card, hub_scroller, tt_btn, tt_entry, zk_fonts, zk_wrap};
use crate::theme::Theme;
use crate::tkcanvas::{Canvas, Photo};
use crate::ui::{Anchor, Fill, Font, Fonts, Kind, Pack, Side, Ui};
use serde_json::Value;
use std::rc::Rc;
use zenko_art::palette::{Palette, mixhex};

pub const TITLE: &str = "Settings";
pub const SUB: &str = "Everything saves automatically";
pub const FUR_NAMES: [(&str, &str); 8] = [("snow", "Byakko"), ("classic", "Inari"), ("ember", "Ember"), ("fennec", "Desert"),
                                          ("midnight", "Midnight"), ("sakura", "Sakura"), ("mint", "Jade"), ("silver", "Moonlight")];
pub const SCARVES: [&str; 8] = ["", "#d9536b", "#e0b43a", "#2f8f83", "#3b6ea8", "#7d6cae", "#ffffff", "#2b2b2b"];
pub const HOTKEYS: [&str; 11] = ["F6", "F7", "F8", "F9", "F10", "F11", "F12", "Right Ctrl", "Right Alt", "Pause", "Scroll Lock"];
pub const PIPER_VOICES: [(&str, &str); 7] = [("Alan (British, male)", "en_GB-alan-medium"),
                                             ("Jenny (British, female)", "en_GB-jenny_dioco-medium"),
                                             ("Alba (Scottish, female)", "en_GB-alba-medium"),
                                             ("Northern English (male)", "en_GB-northern_english_male-medium"),
                                             ("Amy (American, female)", "en_US-amy-medium"),
                                             ("Lessac (American, female)", "en_US-lessac-medium"),
                                             ("Ryan (American, male)", "en_US-ryan-medium")];
pub const MUSIC_TRACKS: [(&str, &str, &str); 3] = [("garden", "Shrine garden", "soft koto, a low hum and a glass wind chime"),
                                                   ("bamboo", "Bamboo rain", "a breathy bamboo flute over gentle rain"),
                                                   ("temple", "Moon temple", "slow temple bowls and a deep, calm drone")];
pub const ONLINE_PROVIDERS: [(&str, &str); 3] = [("gemini", "Gemini (best, searches the web)"), ("groq", "Groq (very fast)"),
                                                 ("openrouter", "OpenRouter (free models)")];
pub const STT_MODELS: [(&str, &str); 2] = [("fast", "Fast (57 MB)"), ("accurate", "More accurate (181 MB, a bit slower)")];
pub const RULE_SITES: [&str; 16] = ["YouTube", "Reddit", "Instagram", "TikTok", "X / Twitter", "Netflix", "Discord", "Twitch",
                                    "Facebook", "Spotify", "Pinterest", "Steam", "Roblox", "WhatsApp", "ChatGPT",
                                    "Any site or app"];
pub const GROUPS: [(&str, &[&str]); 7] = [
    ("Look", &["appearance", "look", "sky", "colour", "font", "text", "frame", "border"]),
    ("Focus", &["focus", "timer", "session", "break", "lock", "pomodoro", "study"]),
    ("Voice", &["voice", "talk", "speech", "sound", "music"]),
    ("Life", &["life", "desktop", "wellbeing", "sleep", "health", "remind"]),
    ("Brain", &["brain", "model", "ollama", "online", "search", "web", "ai"]),
    ("Apps", &["spotify", "remarkable", "rule", "keep", "close", "app", "screen"]),
    ("Data", &["data", "backup", "update", "privacy", "start", "about", "reset"])];
pub const SQUARE_SERVER: &str = "https://ntfy.sh";
pub const SPOTIFY_HELP: &str = "One-time setup (uses your Premium account):\n1. Open the Spotify dashboard, log in, Create app (any \
                                name).\n2. Redirect URI: paste the one below. Tick 'Web API'. Save.\n3. Copy the app's Client ID here, \
                                then press Connect.";

/// What the page shows, from the app (Companion::set_data and the window)
#[derive(Clone, Debug, Default)]
pub struct SetData {
    /// the whole config
    pub cfg: Value,
    /// IS_WIN (Start with Windows), the Start with Windows box, FONT_UI
    pub is_win: bool,
    pub autostart: bool,
    pub font_ui: String,
    /// self.spotify.connected(), self.lockin, zb_linked()
    pub spotify: bool,
    pub lockin: bool,
    pub zb_linked: bool,
    /// texts the app works out: check_engine's line, the talk card's status, "You have ...", the speed card's line,
    /// exam_date().isoformat(), the try-a-title result (text, colour)
    pub whisper: String,
    pub stt: String,
    pub version: String,
    pub live: Option<String>,
    pub install: bool,
    pub frozen: bool,
    pub speed: String,
    pub exam_iso: String,
    pub update_source: String,
    pub rule_try: (String, String),
}

pub struct SetPage {
    pub pg: usize,
    pub title: usize,
    pub sc: Scroller,
    pub nav: usize,
    /// (chip name, the card it jumps to), the gold one, the chips' x spans
    pub chips: Vec<(String, usize)>,
    pub cur: usize,
    pub boxes: Vec<(f64, f64, usize)>,
    /// the cards with titles in the order they were made
    pub cards: Vec<(String, usize)>,
    /// entries: (config key, entry)
    pub entries: Vec<(String, usize)>,
    /// the theme swatches and bib swatches (canvas, key)
    pub swatches: Vec<(usize, String)>,
    pub scarves: Vec<(usize, String)>,
    /// the speed scale, the lock-in list, the status labels by name
    pub scale: usize,
    pub lock_text: usize,
    pub labels: Vec<(String, usize)>,
    /// radio groups: (config key, [(radio, value)])
    pub radios: Vec<(String, Vec<(usize, String)>)>,
    pub wrap: WrapState,
    seig: Option<(String, Rc<Photo>)>,
    zk_f: f64,
}

fn s_of(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Null => String::new(),
        Value::Number(n) => {
            if let Some(i) = n.as_i64() { i.to_string() } else { py_float(n.as_f64().unwrap_or(0.0)) }
        }
        Value::Bool(b) => if *b { "True".into() } else { "False".into() },
        v => v.to_string(),
    }
}

/// str(float) as Python writes it (1.0, 1.15)
fn py_float(f: f64) -> String {
    if f.fract() == 0.0 && f.abs() < 1e16 { format!("{f:.1}") } else { format!("{f}") }
}

fn truthy(v: &Value) -> bool {
    match v {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::Number(n) => n.as_f64() != Some(0.0),
        Value::String(s) => !s.is_empty(),
        Value::Array(a) => !a.is_empty(),
        Value::Object(o) => !o.is_empty(),
    }
}

struct B<'a> {
    ui: &'a mut Ui,
    th: &'a Theme,
    f: &'a Faces,
    cfg: &'a Value,
    cm10: i32,
    bg: String,
}

impl B<'_> {
    fn flag(&self, key: &str, default: bool) -> bool {
        match self.cfg.get(key) {
            None => default,
            Some(v) => truthy(v),
        }
    }
    fn get(&self, key: &str, default: &str) -> String {
        match self.cfg.get(key) {
            None => default.into(),
            Some(v) => s_of(v),
        }
    }
    fn font(&self, n: i32, bold: bool) -> Font {
        Font::new(&self.f.ui, n, bold)
    }
    /// tk.Checkbutton(parent, text, **kw) (bd 2, highlight 0, FONT_UI 10, anchor w)
    fn chk(&mut self, parent: usize, text: &str, on: bool, cmd: &str) -> usize {
        let bg = self.ui.ws[parent].bg.clone();
        let fnt = self.font(10, false);
        self.ui.check(parent, text, &bg, &self.th.ink, fnt, on, &self.th.field, cmd, 0, 2)
    }
    /// check(parent, text, var, cmd): packed fill x, padx 14
    fn check(&mut self, parent: usize, text: &str, key: &str, default: bool) -> usize {
        let on = self.flag(key, default);
        let c = self.chk(parent, text, on, &format!("set_chk:{key}"));
        self.ui.pack(c, Pack::default().fill(Fill::X).padx(14, 14));
        c
    }
    fn radio(&mut self, parent: usize, text: &str, on: bool, cmd: &str, size: i32) -> usize {
        let bg = self.ui.ws[parent].bg.clone();
        let fnt = self.font(size, false);
        let r = self.ui.check(parent, text, &bg, &self.th.ink, fnt, on, &self.th.field, cmd, 0, 2);
        if let Kind::Check { radio, .. } = &mut self.ui.ws[r].kind {
            *radio = true;
        }
        r
    }
    /// radios(parent, items, var, cmd): a row of radios (padx 0 10) in a frame (fill x, padx 14, pady 0 4)
    fn radios(&mut self, parent: usize, items: &[(&str, &str)], key: &str, cur: &str, ty: char,
              groups: &mut Vec<(String, Vec<(usize, String)>)>) {
        let bg = self.bg.clone();
        let fr = self.ui.frame(parent, &bg);
        self.ui.pack(fr, Pack::default().fill(Fill::X).padx(14, 14).pady(0, 4));
        let mut g = vec![];
        for (lab, val) in items {
            let r = self.radio(fr, lab, cur == *val, &format!("set_rad:{key}:{ty}:{val}"), 10);
            self.ui.pack(r, Pack::default().side(Side::Left).padx(0, 10));
            g.push((r, val.to_string()));
        }
        groups.push((key.into(), g));
    }
    /// label(parent, text): GREY, FONT_UI 9, anchor w, fill x, padx 14, pady (6, 0)
    fn label(&mut self, parent: usize, text: &str) -> usize {
        let bg = self.bg.clone();
        let fnt = self.font(9, false);
        let l = self.ui.label(parent, text, &bg, &self.th.grey, fnt);
        self.ui.set_anchor(l, Anchor::W);
        self.ui.pack(l, Pack::default().fill(Fill::X).padx(14, 14).pady(6, 0));
        l
    }
    /// a label made with anchor w, justify left and a wraplength
    fn note(&mut self, parent: usize, text: &str, fg: &str, size: i32, bold: bool, wrap: i32) -> usize {
        let bg = self.ui.ws[parent].bg.clone();
        let fnt = self.font(size, bold);
        let l = self.ui.label(parent, text, &bg, fg, fnt);
        self.ui.set_anchor(l, Anchor::W);
        if let Kind::Label { wrap: w, left, .. } = &mut self.ui.ws[l].kind {
            *w = wrap;
            *left = true;
        }
        l
    }
    fn lbl(&mut self, parent: usize, text: &str, fg: &str, size: i32) -> usize {
        let bg = self.ui.ws[parent].bg.clone();
        let fnt = self.font(size, false);
        self.ui.label(parent, text, &bg, fg, fnt)
    }
    fn btn(&mut self, parent: usize, text: &str, cmd: &str, bg: Option<&str>, fg: Option<&str>, size: i32, padx: i32,
           pady: i32) -> usize {
        tt_btn(self.ui, parent, text, cmd, bg, fg, size, padx, pady, self.th, self.f)
    }
    fn entry(&mut self, parent: usize, text: &str, chars: i32) -> usize {
        tt_entry(self.ui, parent, text, chars, self.th, self.f)
    }
    /// tk.OptionMenu configured (bg FIELD, relief flat, highlight 1 in LINE, width in characters)
    fn menu(&mut self, parent: usize, text: &str, key: &str, size: i32, chars: i32, ring: bool) -> usize {
        let fnt = self.font(size, false);
        let m = self.ui.menubutton(parent, text, &format!("set_menu:{key}"), fnt, &self.th.field, &self.th.ink, &self.th.line,
                                   chars, self.cm10);
        if !ring {
            self.ui.ws[m].hl = 0;
        }
        m
    }
    /// tk.Frame(c, bg=bg, height=8).pack()
    fn gap(&mut self, parent: usize) {
        let bg = self.bg.clone();
        let fr = self.ui.frame(parent, &bg);
        self.ui.ws[fr].opt_h = 8;
        self.ui.pack(fr, Pack::default());
    }
    fn row(&mut self, parent: usize, padx: i32, pady: (i32, i32)) -> usize {
        let bg = self.bg.clone();
        let r = self.ui.frame(parent, &bg);
        self.ui.pack(r, Pack::default().fill(Fill::X).padx(padx, padx).pady(pady.0, pady.1));
        r
    }
}

impl SetPage {
    /// hub_build_settings(pg) (+ z19_settings_nav)
    pub fn build(ui: &mut Ui, pg: usize, th: &Theme, f: &Faces, cm10: i32, d: &SetData) -> SetPage {
        let title = ui.canvas(pg, &th.banner, cm10, 96);
        canvas_of(ui, title);
        ui.pack(title, Pack::default().fill(Fill::X));
        let sc = hub_scroller(ui, pg, th, cm10);
        let mut p = SetPage { pg, title, sc: sc.clone(), nav: 0, chips: vec![], cur: 0, boxes: vec![], cards: vec![],
                              entries: vec![], swatches: vec![], scarves: vec![], scale: 0, lock_text: 0, labels: vec![],
                              radios: vec![], wrap: WrapState::default(), seig: None, zk_f: 1.0 };
        let mut b = B { ui, th, f, cfg: &d.cfg, cm10, bg: th.card.clone() };
        let outer = b.ui.frame(sc.inner, &th.paper);
        b.ui.pack(outer, Pack::default().fill(Fill::X).padx(24, 24).pady(12, 12));
        let left = b.ui.frame(outer, &th.paper);
        b.ui.pack(left, Pack::default().side(Side::Left).fill(Fill::Both).expand().padx(0, 8));
        let right = b.ui.frame(outer, &th.paper);
        b.ui.pack(right, Pack::default().side(Side::Left).fill(Fill::Both).expand().padx(8, 0));
        p.appearance(&mut b, left, d);
        p.focus(&mut b, left);
        p.music(&mut b, right);
        p.internet(&mut b, right);
        p.online(&mut b, right);
        p.general(&mut b, right, d);
        p.extras(&mut b, left, right, d);
        p.lockin(&mut b, right, d);
        p.life(&mut b, left, right);
        // z19_settings_nav: a chip for the first card of each group
        let mut used: Vec<String> = vec![];
        for (name, words) in GROUPS {
            for (t, fr) in &p.cards {
                let tl = t.to_lowercase();
                if words.iter().any(|w| tl.contains(w)) && !used.contains(t) {
                    p.chips.push((name.into(), *fr));
                    for (t2, _) in &p.cards {
                        if words.iter().any(|w| t2.to_lowercase().contains(w)) {
                            used.push(t2.clone());
                        }
                    }
                    break;
                }
            }
        }
        if p.chips.len() < 3 {
            p.chips = p.cards.iter().take(9).map(|(t, fr)| {
                let n = if t.chars().count() < 18 { t.clone() } else { t.chars().take(16).collect::<String>() + "\u{2026}" };
                (n, *fr)
            }).collect();
        }
        let nav = b.ui.canvas(pg, &th.paper, cm10, 30);
        canvas_of(b.ui, nav);
        b.ui.pack_before(nav, sc.boxf, Pack::default().fill(Fill::X).padx(24, 24).pady(8, 0));
        p.nav = nav;
        p
    }

    fn card(&mut self, b: &mut B, parent: usize, title: &str, pady: (i32, i32)) -> usize {
        let c = hub_card(b.ui, parent, Some(title), "round", b.th, b.f);
        b.ui.pack(c, Pack::default().fill(Fill::X).pady(pady.0, pady.1));
        self.cards.push((title.into(), c));
        c
    }

    fn entry_for(&mut self, b: &mut B, parent: usize, key: &str, text: &str, chars: i32) -> usize {
        let e = b.entry(parent, text, chars);
        self.entries.push((key.into(), e));
        e
    }

    fn appearance(&mut self, b: &mut B, left: usize, d: &SetData) {
        let th = b.th;
        let bg = b.bg.clone();
        let c = self.card(b, left, "Appearance", (0, 0));
        b.label(c, "Spirit colours (the fox and the whole app follow)");
        let sw = b.ui.frame(c, &bg);
        b.ui.pack(sw, Pack::default().fill(Fill::X).padx(14, 14).pady(4, 2));
        let cur = b.get("theme", "snow");
        for (i, (key, name)) in FUR_NAMES.iter().enumerate() {
            let fr = b.ui.frame(sw, &bg);
            b.ui.grid(fr, crate::ui::Grid { row: i / 4, col: i % 4, padx: (4, 4), pady: (3, 3), sticky: "w", ..Default::default() });
            let cv = b.ui.canvas(fr, &bg, 34, 34);
            let fp = Palette::fur(key);
            let (o, k, w) = (fp.hex(b'O').unwrap_or_default(), fp.hex(b'K').unwrap_or_default(), fp.hex(b'W').unwrap_or_default());
            let ring = if *key == cur {
                crate::theme::theme(&crate::Cfg::from(serde_json::json!({"theme": key}))).acc
            } else {
                bg.clone()
            };
            let t = canvas_of(b.ui, cv);
            t.oval(2.0, 2.0, 32.0, 32.0).outline(&ring).width(3.0);
            t.oval(7.0, 7.0, 27.0, 27.0).fill(&o).outline(&k).width(2.0);
            t.arc(7.0, 7.0, 27.0, 27.0, 200.0, 140.0, "chord").fill(&w).outline("");
            t.oval(7.0, 7.0, 27.0, 27.0).outline(&k).width(2.0);
            b.ui.pack(cv, Pack::default().side(Side::Left));
            let on = *key == cur;
            let l = b.ui.label(fr, name, &bg, if on { &th.ink } else { &th.grey }, b.font(9, on));
            b.ui.pack(l, Pack::default().side(Side::Left).padx(2, 6));
            self.swatches.push((cv, key.to_string()));
            self.swatches.push((l, key.to_string()));
            self.swatches.push((fr, key.to_string()));
        }
        b.label(c, "Little extras");
        b.check(c, "Glasses on during focus sessions", "glasses_in_session", true);
        b.check(c, "Headphones on while Spotify plays", "headphones_with_music", true);
        b.check(c, "Music tablet in her paws whenever Spotify is open", "belly_music", true);
        b.check(c, "Say each new song in a bubble", "announce_songs", true);
        b.check(c, "Curl up and sleep when I'm away", "curl_when_away", true);
        b.check(c, "Foxfire wisps float around her", "foxfire", true);
        b.label(c, "Tails (a zenko earns a new tail with wisdom)");
        let tails = b.get("tails", "grow");
        b.radios(c, &[("Grow as I focus (3 to 9)", "grow"), ("1", "1"), ("3", "3"), ("9", "9")], "tails", &tails, 's',
                 &mut self.radios);
        b.label(c, "Inari bib (the red shrine bib under her bell)");
        let scf = b.ui.frame(c, &bg);
        b.ui.pack(scf, Pack::default().fill(Fill::X).padx(14, 14).pady(2, 4));
        for col in SCARVES {
            let cv = b.ui.canvas(scf, &bg, 26, 26);
            canvas_of(b.ui, cv);
            b.ui.pack(cv, Pack::default().side(Side::Left).padx(1, 1));
            self.scarves.push((cv, col.to_string()));
        }
        self.draw_scarves(b.ui, th, &b.get("scarf", ""), &bg);
        b.label(c, "Font");
        let m = b.menu(c, &d.font_ui, "app_font", 10, 24, true);
        b.ui.pack(m, Pack::default().anchor(Anchor::W).padx(14, 14).pady(2, 2));
        let bf = b.get("bubble_font", "app") == "pixel";
        let cb = b.chk(c, "Retro typewriter font in the fox's speech bubbles", bf, "set_chk:bubble_font");
        b.ui.pack(cb, Pack::default().fill(Fill::X).padx(14, 14));
        b.label(c, "Fox size");
        let size = b.get("size", "4");
        b.radios(c, &[("Tiny", "3"), ("Small", "4"), ("Medium", "5"), ("Large", "6")], "size", &size, 'i', &mut self.radios);
        b.label(c, "Fox mood when you slack");
        let mode = b.get("mode", "gentle");
        b.radios(c, &[("Gentle", "gentle"), ("Mixed", "mixed"), ("Savage", "savage")], "mode", &mode, 's', &mut self.radios);
        b.check(c, "Fox looks at my mouse", "follow_mouse", true);
        b.check(c, "Zenko's sounds (greetings, nudges, bells)", "sounds", true);
        b.check(c, "Interface sounds (soft taps, plucks and chimes)", "ui_sounds", true);
        b.label(c, "Sound volume");
        let vol = match b.cfg.get("sound_volume") { Some(v) if truthy(v) => s_of(v), _ => "2".into() };
        b.radios(c, &[("Soft", "1"), ("Medium", "2"), ("Loud", "3")], "sound_volume", &vol, 'i', &mut self.radios);
        b.gap(c);
    }

    /// draw_sc(): the bib swatches with a ring on the one she wears
    pub fn draw_scarves(&self, ui: &mut Ui, th: &Theme, cur: &str, bg: &str) {
        for (cv, col) in &self.scarves {
            let t = canvas_of(ui, *cv);
            *t = Canvas::default();
            let on = cur == col;
            t.oval(2.0, 2.0, 24.0, 24.0).outline(if on { &th.acc } else { bg }).width(3.0);
            if !col.is_empty() {
                t.oval(6.0, 6.0, 20.0, 20.0).fill(col).outline(&th.line).width(1.0);
            } else {
                t.line(vec![7.0, 19.0, 19.0, 7.0]).fill(&th.grey).width(2.0);
            }
        }
    }

    fn focus(&mut self, b: &mut B, left: usize) {
        let th = b.th;
        let c = self.card(b, left, "Focus and breaks", (12, 0));
        b.check(c, "Nag me even without a session", "watch_always", false);
        b.check(c, "Auto-close distractions during a session", "auto_close", false);
        b.check(c, "Start a break when a session finishes", "auto_break_after_session", true);
        let r = b.row(c, 14, (6, 10));
        for (text, key, default) in [("Rest reminder every", "break_after_minutes", "50"), ("min  \u{b7}  break length", "break_minutes", "5")] {
            let l = b.lbl(r, text, &th.ink, 10);
            b.ui.pack(l, Pack::default().side(Side::Left));
            let v = b.get(key, default);
            let e = self.entry_for(b, r, key, &v, 4);
            b.ui.pack(e, Pack::default().side(Side::Left).padx(6, 6));
        }
        let l = b.lbl(r, "min", &th.ink, 10);
        b.ui.pack(l, Pack::default().side(Side::Left));
    }

    fn music(&mut self, b: &mut B, right: usize) {
        let bg = b.bg.clone();
        let c = self.card(b, right, "Background music", (0, 12));
        b.check(c, "Play soft Japanese background music", "bgm", true);
        b.label(c, "Track (original music made on your PC)");
        let tv = b.get("bgm_track", "garden");
        let fr = b.ui.frame(c, &bg);
        b.ui.pack(fr, Pack::default().fill(Fill::X).padx(14, 14));
        let mut g = vec![];
        for (key, name, desc) in MUSIC_TRACKS {
            let r = b.radio(fr, &format!("{name}  ({desc})"), tv == key, &format!("set_rad:bgm_track:s:{key}"), 10);
            b.ui.pack(r, Pack::default().fill(Fill::X));
            g.push((r, key.to_string()));
        }
        self.radios.push(("bgm_track".into(), g));
        b.label(c, "Volume");
        let vv = match b.cfg.get("bgm_volume") { Some(v) if truthy(v) => s_of(v), _ => "2".into() };
        b.radios(c, &[("Barely there", "1"), ("Under a whisper", "2"), ("Whisper", "4"), ("Soft", "6"), ("Louder", "9")],
                 "bgm_volume", &vv, 'i', &mut self.radios);
        b.label(c, "When");
        let wv = b.get("bgm_when", "always");
        b.radios(c, &[("Always", "always"), ("Only while focusing", "focus")], "bgm_when", &wv, 's', &mut self.radios);
        b.gap(c);
    }

    fn internet(&mut self, b: &mut B, right: usize) {
        let th = b.th;
        let c = self.card(b, right, "Internet lookups and smart brain", (0, 0));
        b.check(c, "Allow internet lookups (weather, news, web search)", "web_enabled", true);
        b.check(c, "Search the web when Zenko doesn't know something", "auto_search", true);
        b.label(c, "Home city (used when you just say 'weather')");
        let hv = b.get("home_city", "");
        let e = self.entry_for(b, c, "home_city", &hv, 26);
        b.ui.pack(e, Pack::default().anchor(Anchor::W).padx(14, 14).pady(2, 4));
        b.label(c, "Temperature unit");
        let uv = b.get("temp_unit", "C");
        b.radios(c, &[("Celsius", "C"), ("Fahrenheit", "F")], "temp_unit", &uv, 's', &mut self.radios);
        b.check(c, "Use offline smart brain (Ollama)", "llm_enabled", true);
        let bt = b.btn(c, "Smart brain setup...", "set_btn:brain", Some(&th.start), None, 9, 10, 2);
        b.ui.pack(bt, Pack::default().anchor(Anchor::W).padx(14, 14).pady(4, 12));
    }

    fn online(&mut self, b: &mut B, right: usize) {
        let th = b.th;
        let bg = b.bg.clone();
        let c = self.card(b, right, "Online brain (optional, free)", (12, 0));
        let note = |b: &mut B, s: &mut SetPage, text: &str, fg: &str, size: i32| {
            let l = b.note(c, text, fg, size, false, 360);
            b.ui.pack(l, Pack::default().fill(Fill::X).padx(14, 14).pady(2, 2));
            s.labels.push(("nb_wrap".into(), l));
            l
        };
        note(b, self, "When you're online, a free online AI finds the answer (Gemini can also search the web), then your own \
                       models on this PC re-check it so it uses IB DP terms and the 2027 syllabus. Offline, or if the free \
                       limit runs out, Zenko just uses your PC's models like before.", &th.ink, 9);
        b.check(c, "Use it when I'm online", "online_on", false);
        b.check(c, "Re-check with my PC's models", "online_check", true);
        b.label(c, "Which one");
        let pv = b.get("online_provider", "gemini");
        let mut g = vec![];
        for (val, txt) in ONLINE_PROVIDERS {
            let r = b.radio(c, txt, pv == val, &format!("set_rad:online_provider:s:{val}"), 9);
            b.ui.pack(r, Pack::default().fill(Fill::X).padx(14, 14));
            g.push((r, val.to_string()));
        }
        self.radios.push(("online_provider".into(), g));
        b.label(c, "Your free key (saved only on this PC)");
        let kr = b.ui.frame(c, &bg);
        b.ui.pack(kr, Pack::default().fill(Fill::X).padx(14, 14).pady(2, 4));
        let key = b.cfg.get("online_keys").and_then(|k| k.get(&pv)).map(s_of).unwrap_or_default();
        let ke = self.entry_for(b, kr, "online_key", &key, 18);
        if let Kind::Entry { e, .. } = &mut b.ui.ws[ke].kind {
            e.show = Some('\u{2022}');
        }
        b.ui.pack(ke, Pack::default().side(Side::Left));
        let status = note(b, self, "", &th.grey, 8);
        self.labels.push(("online_status".into(), status));
        let t = b.btn(kr, "Test", "set_btn:online_test", Some(&th.field), None, 8, 8, 1);
        b.ui.pack(t, Pack::default().side(Side::Left).padx(6, 0));
        let g2 = b.btn(c, "Get a free key...", "set_btn:online_signup", Some(&th.start), None, 9, 10, 2);
        b.ui.pack(g2, Pack::default().anchor(Anchor::W).padx(14, 14).pady(4, 2));
        note(b, self, "Gemini key in 1 minute: 1) press 'Get a free key' and sign in with a Google account. 2) Press 'Create API \
                       key'. 3) Copy it, paste it above, press Test. No card needed. Free limits are small but plenty for \
                       studying.", &th.grey, 8);
        note(b, self, "Privacy: with this on, your tutor questions, files you ask about and screenshots you send to chat go to \
                       the online AI. Zenko never sends your screen on its own.", &th.grey, 8);
        b.gap(c);
    }

    fn general(&mut self, b: &mut B, right: usize, d: &SetData) {
        let th = b.th;
        let c = self.card(b, right, "General", (12, 0));
        if d.is_win {
            let cb = b.chk(c, "Start with Windows", d.autostart, "set_chk:autostart");
            b.ui.pack(cb, Pack::default().fill(Fill::X).padx(14, 14));
        }
        b.check(c, "Open this window when the app starts", "open_hub_on_start", true);
        let row = b.row(c, 14, (8, 12));
        let x = b.btn(row, "Edit distracting sites...", "set_btn:distractions", Some(&th.start), None, 9, 10, 2);
        b.ui.pack(x, Pack::default().side(Side::Left).padx(0, 8));
        let x = b.btn(row, "Export timetable PDF", "set_btn:export_pdf", Some(&th.field), None, 9, 10, 2);
        b.ui.pack(x, Pack::default().side(Side::Left));
    }

    /// hub_settings_extras down the class layers
    fn extras(&mut self, b: &mut B, left: usize, right: usize, d: &SetData) {
        self.voice(b, left, d);
        self.fox_voice(b, left);
        self.spotify(b, right, d);
        self.presets(b, right);
        self.window(b, right);
        self.talk(b, left, d);
        self.rules(b, right, d);
        self.helpers(b, right, d);
        self.updates(b, right, d);
        self.sky(b, right);
        self.speed(b, left, d);
        self.linked(b, right, d);
        self.ema_plan(b, right);
    }

    fn voice(&mut self, b: &mut B, left: usize, d: &SetData) {
        let th = b.th;
        let bg = b.bg.clone();
        let c = self.card(b, left, "Voice and screen buddy", (12, 0));
        b.check(c, "Push-to-talk hotkeys on", "voice_enabled", true);
        let r = b.row(c, 14, (4, 0));
        for (text, key, default) in [("Hold to talk", "voice_key", "Right Ctrl"), ("Hold to ask about screen", "screen_key", "F8")] {
            let l = b.lbl(r, text, &th.ink, 10);
            b.ui.pack(l, Pack::default().side(Side::Left));
            let v = b.get(key, default);
            let m = b.menu(r, &v, key, 9, 9, true);
            b.ui.pack(m, Pack::default().side(Side::Left).padx(6, 14));
        }
        b.label(c, "Speech engine");
        let eng = b.get("voice_engine", "auto");
        b.radios(c, &[("Best available", "auto"), ("Windows built-in", "windows"), ("Whisper", "whisper")], "voice_engine",
                 &eng, 's', &mut self.radios);
        b.label(c, "Whisper accuracy (bigger = more accurate, slower; downloads once)");
        let wm = b.get("whisper_model", "small.en");
        b.radios(c, &[("Fast", "base.en"), ("Accurate", "small.en"), ("Best", "medium.en")], "whisper_model", &wm, 's',
                 &mut self.radios);
        let st = b.note(c, &d.whisper, &th.grey, 9, false, 420);
        self.labels.push(("whisper".into(), st));
        let rr = b.ui.frame(c, &bg);
        b.ui.pack(rr, Pack::default().fill(Fill::X).padx(14, 14).pady(4, 2));
        let x = b.btn(rr, "Install Whisper", "set_btn:install_whisper", Some(&th.start), None, 9, 10, 1);
        b.ui.pack(x, Pack::default().side(Side::Left).padx(0, 6));
        let x = b.btn(rr, "Test my voice", "set_btn:test_voice", Some(&th.field), None, 9, 10, 1);
        b.ui.pack(x, Pack::default().side(Side::Left));
        b.ui.pack(st, Pack::default().fill(Fill::X).padx(14, 14).pady(4, 0));
        b.label(c, "Screen buddy 'eyes' (Ollama vision model)");
        let vm = b.get("vision_model", "qwen2.5vl:3b");
        let e = self.entry_for(b, c, "vision_model", &vm, 20);
        b.ui.pack(e, Pack::default().anchor(Anchor::W).padx(14, 14).pady(2, 12));
    }

    fn fox_voice(&mut self, b: &mut B, left: usize) {
        let th = b.th;
        let c = self.card(b, left, "Fox's voice", (12, 0));
        b.label(c, "Fox talks back");
        let sm = b.get("speak_mode", "voice");
        b.radios(c, &[("Off", "off"), ("When I talk", "voice"), ("Always", "always")], "speak_mode", &sm, 's', &mut self.radios);
        b.label(c, "Voice");
        let cur = b.get("tts_voice", "");
        let mut name = "Windows default".to_string();
        if let Some((n, _)) = PIPER_VOICES.iter().find(|(_, v)| format!("piper:{v}") == cur) {
            name = format!("Natural: {n}");
        } else if !cur.is_empty() {
            name = format!("Windows: {}", cur.chars().skip(4).collect::<String>());
        }
        let m = b.menu(c, &name, "tts_voice", 10, 32, true);
        b.ui.pack(m, Pack::default().anchor(Anchor::W).padx(14, 14).pady(2, 0));
        let vst = b.note(c, "", &th.grey, 9, false, 420);
        self.labels.push(("voice_status".into(), vst));
        b.label(c, "Speed");
        let v = b.cfg.get("tts_speed").and_then(|v| v.as_f64()).unwrap_or(0.0).trunc();
        let sp = b.ui.scale(c, -3.0, 3.0, v, 240, 14, 26, &th.acc, &th.field, "set_scale:tts_speed");
        b.ui.pack(sp, Pack::default().anchor(Anchor::W).padx(14, 14).pady(2, 0));
        self.scale = sp;
        let bg = b.bg.clone();
        let ticks = b.ui.frame(c, &bg);
        (b.ui.ws[ticks].opt_w, b.ui.ws[ticks].opt_h) = (240, 16);
        b.ui.ws[ticks].propagate = false;
        b.ui.pack(ticks, Pack::default().anchor(Anchor::W).padx(14, 14));
        for (txt, rx, anc) in [("slower", 0.0, 0), ("normal", 0.5, 1), ("faster", 1.0, 2)] {
            let l = b.lbl(ticks, txt, &th.grey, 8);
            b.ui.ws[l].place = Some(crate::ui::Place { relx: rx, y: 0, n: anc == 1, ne: anc == 2, ..Default::default() });
        }
        let r = b.row(c, 14, (6, 2));
        let x = b.btn(r, "Test voice", "set_btn:speak_test", Some(&th.start), None, 9, 10, 1);
        b.ui.pack(x, Pack::default().side(Side::Left).padx(0, 6));
        let x = b.btn(r, "Install natural voices", "set_btn:install_piper", Some(&th.field), None, 9, 10, 1);
        b.ui.pack(x, Pack::default().side(Side::Left));
        b.ui.pack(vst, Pack::default().fill(Fill::X).padx(14, 14).pady(4, 12));
    }

    fn spotify(&mut self, b: &mut B, right: usize, d: &SetData) {
        let th = b.th;
        let c = self.card(b, right, "Spotify", (12, 0));
        let sst = b.note(c, if d.spotify { "Connected." } else { "" }, &th.accd, 9, true, 420);
        self.labels.push(("spotify".into(), sst));
        if !d.spotify {
            let l = b.note(c, SPOTIFY_HELP, &th.ink, 9, false, 0);
            b.ui.pack(l, Pack::default().fill(Fill::X).padx(14, 14));
            let r = b.row(c, 14, (6, 2));
            let x = b.btn(r, "Open dashboard", "set_btn:spotify_dashboard", Some(&th.field), None, 9, 8, 1);
            b.ui.pack(x, Pack::default().side(Side::Left).padx(0, 6));
            let x = b.btn(r, "Copy redirect URI", "set_btn:spotify_copy", Some(&th.field), None, 9, 8, 1);
            b.ui.pack(x, Pack::default().side(Side::Left));
        }
        b.label(c, "Client ID");
        let cid = b.get("spotify_client_id", "");
        let e = self.entry_for(b, c, "spotify_client_id", &cid, 34);
        b.ui.pack(e, Pack::default().anchor(Anchor::W).padx(14, 14).pady(2, 4));
        let r = b.row(c, 14, (2, 4));
        if d.spotify {
            let x = b.btn(r, "Disconnect", "set_btn:spotify_forget", Some(&th.delbtn), None, 9, 10, 1);
            b.ui.pack(x, Pack::default().side(Side::Left));
        } else {
            let x = b.btn(r, "Connect", "set_btn:spotify_connect", Some(&th.acc), Some(&th.onacc), 10, 14, 2);
            b.ui.pack(x, Pack::default().side(Side::Left));
        }
        b.check(c, "Play music when a focus session starts (pause on breaks)", "spotify_auto", false);
        b.label(c, "Focus playlist (name of one of yours, or a search like 'lofi beats')");
        let fp = b.get("spotify_focus_playlist", "");
        let e = self.entry_for(b, c, "spotify_focus_playlist", &fp, 30);
        b.ui.pack(e, Pack::default().anchor(Anchor::W).padx(14, 14).pady(2, 4));
        b.ui.pack(sst, Pack::default().fill(Fill::X).padx(14, 14).pady(2, 12));
    }

    fn presets(&mut self, b: &mut B, right: usize) {
        let th = b.th;
        let c = self.card(b, right, "Music presets  (say 'play focus', 'play my favourite song'...)", (12, 0));
        let l = b.lbl(c, "Type a name to search, or paste a Spotify link (in Spotify: Share > Copy link).", &th.grey, 9);
        b.ui.set_anchor(l, Anchor::W);
        b.ui.pack(l, Pack::default().fill(Fill::X).padx(14, 14));
        let mut presets: Vec<Value> = b.cfg.get("music_presets").and_then(|v| v.as_array()).cloned().unwrap_or_else(|| {
            (0..3).map(|i| serde_json::json!({"name": format!("Preset {}", i + 1), "kind": "playlist", "value": ""})).collect()
        });
        while presets.len() < 3 {
            presets.push(serde_json::json!({"name": format!("Preset {}", presets.len() + 1), "kind": "playlist", "value": ""}));
        }
        let pst = b.note(c, "", &th.accd, 9, true, 0);
        self.labels.push(("presets".into(), pst));
        let kinds = [("Playlist", "playlist"), ("Song", "track"), ("Album", "album"), ("Artist", "artist")];
        for (i, ps) in presets.iter().take(3).enumerate() {
            let r = b.row(c, 14, (3, 3));
            let nv = ps.get("name").map(s_of).unwrap_or_default();
            let kind = ps.get("kind").and_then(|v| v.as_str()).unwrap_or("playlist");
            let kv = kinds.iter().find(|(_, v)| *v == kind).map(|(k, _)| *k).unwrap_or("Playlist");
            let pv = ps.get("value").map(s_of).unwrap_or_default();
            let ne = self.entry_for(b, r, &format!("preset_name:{i}"), &nv, 13);
            b.ui.pack(ne, Pack::default().side(Side::Left));
            let m = b.menu(r, kv, &format!("preset_kind:{i}"), 9, 7, true);
            b.ui.pack(m, Pack::default().side(Side::Left).padx(4, 4));
            let ve = self.entry_for(b, r, &format!("preset_value:{i}"), &pv, 20);
            b.ui.pack(ve, Pack::default().side(Side::Left).padx(0, 4));
            let x = b.btn(r, "Play", &format!("set_btn:preset_play:{i}"), Some(&th.start), None, 9, 8, 1);
            b.ui.pack(x, Pack::default().side(Side::Left).padx(2, 2));
            let x = b.btn(r, "Use now", &format!("set_btn:preset_use:{i}"), Some(&th.field), None, 9, 6, 1);
            b.ui.pack(x, Pack::default().side(Side::Left).padx(2, 2));
        }
        b.ui.pack(pst, Pack::default().fill(Fill::X).padx(14, 14).pady(2, 12));
    }

    fn window(&mut self, b: &mut B, right: usize) {
        let c = self.card(b, right, "Window", (12, 0));
        b.check(c, "Borderless window with rounded corners", "borderless", true);
        b.check(c, "Battery meter on the fox's tail (warns when low)", "battery_on_tail", true);
        b.check(c, "Dates in files are day-first (05/12 = 5 December)", "date_day_first", true);
        b.check(c, "Drifting snow, petals or fireflies in the sidebar", "sidebar_particles", true);
        b.gap(c);
    }

    fn talk(&mut self, b: &mut B, left: usize, d: &SetData) {
        let th = b.th;
        let c = self.card(b, left, "Talk with Zenko (offline)", (12, 0));
        b.label(c, "Talk out loud and she answers out loud. Your voice is turned into text on this PC by whisper.cpp \
                    (free, offline). Nothing is sent anywhere.");
        let st = b.note(c, &d.stt, &th.gold, 9, false, 0);
        self.labels.push(("stt".into(), st));
        b.label(c, "Ears (speech model)");
        let mv = b.get("stt_model", "fast");
        let items: Vec<(&str, &str)> = STT_MODELS.iter().map(|(k, lab)| (*lab, *k)).collect();
        b.radios(c, &items, "stt_model", &mv, 's', &mut self.radios);
        let r = b.row(c, 14, (4, 2));
        let x = b.btn(r, "Set up", "set_btn:stt_setup", Some(&th.acc), Some(&th.onacc), 9, 10, 2);
        b.ui.pack(x, Pack::default().side(Side::Left));
        let x = b.btn(r, "Use files I downloaded\u{2026}", "set_btn:stt_dir", None, None, 9, 10, 2);
        b.ui.pack(x, Pack::default().side(Side::Left).padx(6, 6));
        let x = b.btn(r, "Talk now", "set_btn:conv_start", Some(&th.start), None, 9, 10, 2);
        b.ui.pack(x, Pack::default().side(Side::Left));
        b.ui.pack(st, Pack::default().fill(Fill::X).padx(14, 14).pady(4, 2));
        b.label(c, "If the school network blocks the download: at home, get whisper-bin-x64.zip from \
                    github.com/ggml-org/whisper.cpp/releases and ggml-base.en-q5_1.bin from \
                    huggingface.co/ggerganov/whisper.cpp, unzip both into one folder, then 'Use files I downloaded'.");
        b.label(c, "Her voice when she answers");
        let tv = b.get("talk_voice", "english");
        b.radios(c, &[("Clear English voice", "english"), ("Anime voice (VOICEVOX)", "voicevox")], "talk_voice", &tv, 's',
                 &mut self.radios);
        b.label(c, "The anime voice needs the free VOICEVOX app running (Customize > Her voice). She says the Japanese \
                    version of her answer out loud while the chat stays in English (switch that off in Customize > Her \
                    voice). The English voice is picked in 'Fox's voice' below.");
        b.gap(c);
    }

    fn rules(&mut self, b: &mut B, right: usize, d: &SetData) {
        let th = b.th;
        let bg = b.bg.clone();
        let c = self.card(b, right, "Close or keep \u{b7} your rules", (12, 0));
        b.label(c, "Tell her exactly what to close and what to leave open in focus sessions and lock-in. Your rules come \
                    first, top to bottom. Example: YouTube + \u{201c}bio, cell, khan\u{201d} \u{2192} Keep open.");
        let lst = b.ui.frame(c, &bg);
        b.ui.pack(lst, Pack::default().fill(Fill::X).padx(14, 14).pady(4, 2));
        let rules: Vec<Value> = b.cfg.get("zk_rules").and_then(|v| v.as_array()).cloned().unwrap_or_default();
        if rules.is_empty() {
            let f = Font { family: b.f.ui.clone(), size: 9, bold: false, italic: true };
            let l = b.ui.label(lst, "No rules yet.", &bg, &th.grey, f);
            b.ui.set_anchor(l, Anchor::W);
            b.ui.pack(l, Pack::default().fill(Fill::X));
        }
        let rb = mixhex(&bg, &th.field, 0.6);
        for (i, r) in rules.iter().enumerate() {
            let row = b.ui.frame(lst, &rb);
            b.ui.pack(row, Pack::default().fill(Fill::X).pady(2, 2));
            let keep = r.get("do").and_then(|v| v.as_str()) == Some("keep");
            let on = r.get("on").map(truthy).unwrap_or(true);
            let l = b.ui.label(row, if keep { "\u{2713}" } else { "\u{2715}" }, &rb, if keep { "#5fae7e" } else { "#e5484d" },
                               b.font(11, true));
            b.ui.ws[l].opt_w = 2;
            b.ui.pack(l, Pack::default().side(Side::Left).padx(6, 2));
            let cb = b.ui.check(row, "", &rb, &th.ink, b.font(9, false), on, &th.field, &format!("set_btn:rule_on:{i}"), 1, 2);
            b.ui.pack(cb, Pack::default().side(Side::Left));
            let t = b.note(row, &crate::routes17::rule_text(r), if on { &th.ink } else { &th.grey }, 9, false, 300);
            b.ui.pack(t, Pack::default().side(Side::Left).fill(Fill::X).expand().pady(4, 4));
            for (sym, dd) in [("\u{25b2}", "-1"), ("\u{25bc}", "1")] {
                let a = b.ui.label(row, sym, &rb, &th.grey, b.font(8, false));
                b.ui.ws[a].padx = 3;
                b.ui.pack(a, Pack::default().side(Side::Left));
                self.labels.push((format!("rule_move:{i}:{dd}"), a));
            }
            let x = b.ui.label(row, "Remove", &rb, &th.grey, b.font(8, false));
            if let Kind::Label { under, .. } = &mut b.ui.ws[x].kind {
                *under = true;
            }
            b.ui.ws[x].padx = 6;
            b.ui.pack(x, Pack::default().side(Side::Left));
            self.labels.push((format!("rule_remove:{i}"), x));
        }
        let add = b.row(c, 14, (8, 2));
        let l = b.lbl(add, "On", &th.ink, 9);
        b.ui.pack(l, Pack::default().side(Side::Left));
        let m = b.menu(add, "YouTube", "rule_site", 9, 12, false);
        b.ui.pack(m, Pack::default().side(Side::Left).padx(4, 4));
        let other = self.entry_for(b, add, "rule_other", "", 12);
        let _ = other;
        let add2 = b.row(c, 14, (4, 2));
        let l = b.lbl(add2, "if the title has", &th.ink, 9);
        b.ui.pack(l, Pack::default().side(Side::Left));
        let we = self.entry_for(b, add2, "rule_words", "", 22);
        b.ui.pack(we, Pack::default().side(Side::Left).padx(4, 4));
        let l = b.lbl(add2, "(optional, commas)", &th.grey, 8);
        b.ui.pack(l, Pack::default().side(Side::Left));
        let add3 = b.row(c, 14, (4, 2));
        let mut g = vec![];
        for (val, lab) in [("keep", "\u{2713} Keep it open"), ("close", "\u{2715} Close it")] {
            let r = b.ui.check(add3, lab, &th.field, &th.ink, b.font(9, true), val == "keep", &mixhex(&th.field, &th.gold, 0.35),
                               &format!("set_rad:rule_do:s:{val}"), 1, 0);
            if let Kind::Check { radio, noind, .. } = &mut b.ui.ws[r].kind {
                *radio = true;
                *noind = true;
            }
            (b.ui.ws[r].padx, b.ui.ws[r].pady) = (10, 3);
            b.ui.pack(r, Pack::default().side(Side::Left).padx(0, 6));
            g.push((r, val.to_string()));
        }
        self.radios.push(("rule_do".into(), g));
        let msg = b.note(c, "", &th.grey, 8, false, 360);
        self.labels.push(("rule_msg".into(), msg));
        let x = b.btn(add3, "Add rule", "set_btn:rule_add", Some(&th.start), None, 9, 10, 1);
        b.ui.pack(x, Pack::default().side(Side::Left).padx(8, 0));
        b.ui.pack(msg, Pack::default().fill(Fill::X).padx(14, 14).pady(4, 2));
        let tr = b.row(c, 14, (6, 12));
        let l = b.lbl(tr, "Try a title", &th.grey, 8);
        b.ui.pack(l, Pack::default().side(Side::Left));
        let te = self.entry_for(b, tr, "rule_try", "Cell biology: mitosis explained - YouTube", 26);
        b.ui.pack(te, Pack::default().side(Side::Left).padx(4, 4));
        let res = b.ui.label(tr, &d.rule_try.0, &bg, &d.rule_try.1, b.font(8, true));
        b.ui.pack(res, Pack::default().side(Side::Left).padx(4, 4));
        self.labels.push(("rule_res".into(), res));
    }

    fn helpers(&mut self, b: &mut B, right: usize, d: &SetData) {
        let th = b.th;
        let c = self.card(b, right, "Study helpers and screen", (12, 0));
        b.label(c, "Text size (it also grows and shrinks with the window)");
        let ts = b.cfg.get("text_scale").map(|v| match v {
            Value::Number(n) => py_float(n.as_f64().unwrap_or(1.0)),
            v => s_of(v),
        }).unwrap_or_else(|| "1.0".into());
        b.radios(c, &[("Small", "0.9"), ("Normal", "1.0"), ("Large", "1.15"), ("Extra large", "1.3")], "text_scale", &ts, 'f',
                 &mut self.radios);
        b.check(c, "Shrine frame around the window (reopen the window to see it change)", "ui_border", true);
        b.check(c, "Make flashcards from my Study book chats automatically", "auto_cards", true);
        b.check(c, "Pop quiz from my due cards now and then when I'm not in a session", "idle_quiz", true);
        b.check(c, "Show the exam countdown on her ema plaque", "ema_countdown", true);
        b.check(c, "Nudge me about IA / EE / TOK deadlines", "ib_nudges", true);
        b.check(c, "Let YouTube videos about my subjects through in focus sessions and lock-in", "study_videos", true);
        b.label(c, "Extra words that mean a video is for study (comma separated, e.g. a teacher's or channel's name)");
        let rw = b.row(c, 14, (2, 8));
        let words: Vec<String> = b.cfg.get("study_video_words").and_then(|v| v.as_array())
            .map(|a| a.iter().map(s_of).collect()).unwrap_or_default();
        let we = self.entry_for(b, rw, "study_video_words", &words.join(", "), 30);
        b.ui.pack(we, Pack::default().side(Side::Left));
        let wst = b.lbl(rw, "", &th.grey, 9);
        let x = b.btn(rw, "Save", "set_btn:save_words", Some(&th.start), None, 9, 10, 1);
        b.ui.pack(x, Pack::default().side(Side::Left).padx(6, 6));
        b.ui.pack(wst, Pack::default().side(Side::Left));
        self.labels.push(("words".into(), wst));
        b.label(c, "First exam day (YYYY-MM-DD), for the countdown");
        let r = b.row(c, 14, (2, 12));
        let e = self.entry_for(b, r, "exam_date", &d.exam_iso, 12);
        b.ui.pack(e, Pack::default().side(Side::Left));
        let st = b.lbl(r, "", &th.grey, 9);
        let x = b.btn(r, "Save", "set_btn:save_exam", Some(&th.start), None, 9, 10, 1);
        b.ui.pack(x, Pack::default().side(Side::Left).padx(6, 6));
        b.ui.pack(st, Pack::default().side(Side::Left));
        self.labels.push(("exam".into(), st));
    }

    fn updates(&mut self, b: &mut B, right: usize, d: &SetData) {
        let th = b.th;
        let c = self.card(b, right, "Updates and install", (12, 0));
        b.label(c, &d.version);
        if let Some(live) = &d.live {
            b.label(c, live);
            let rb = b.row(c, 14, (2, 4));
            let x = b.btn(rb, "Go back to the built-in version", "set_btn:update_back", None, None, 9, 10, 2);
            b.ui.pack(x, Pack::default().side(Side::Left));
        }
        let st = b.note(c, "", &th.gold, 9, false, 0);
        self.labels.push(("update".into(), st));
        if d.frozen && !d.install {
            let r0 = b.row(c, 14, (2, 6));
            let x = b.btn(r0, "Install Zenko on this PC", "set_btn:install_app", Some(&th.acc), Some(&th.onacc), 9, 10, 2);
            b.ui.pack(x, Pack::default().side(Side::Left));
        }
        b.label(c, "Where updates come from: a GitHub name like yourname/zenko (free GitHub Releases), or a link to a \
                    latest.json file on your website.");
        let r = b.row(c, 14, (2, 6));
        let e = self.entry_for(b, r, "update_source", &d.update_source, 26);
        b.ui.pack(e, Pack::default().side(Side::Left));
        let x = b.btn(r, "Save", "set_btn:update_save", Some(&th.start), None, 9, 10, 1);
        b.ui.pack(x, Pack::default().side(Side::Left).padx(6, 6));
        b.label(c, "When there's a new version");
        let mv = b.get("update_mode", "auto");
        b.radios(c, &[("Ask me first", "ask"), ("Install it automatically", "auto"), ("Don't check", "off")], "update_mode",
                 &mv, 's', &mut self.radios);
        let r2 = b.row(c, 14, (6, 4));
        let x = b.btn(r2, "Check now", "set_btn:update_check", None, None, 9, 10, 2);
        b.ui.pack(x, Pack::default().side(Side::Left));
        let x = b.btn(r2, "Install from a file\u{2026}", "set_btn:update_file", None, None, 9, 10, 2);
        b.ui.pack(x, Pack::default().side(Side::Left).padx(6, 6));
        b.ui.pack(st, Pack::default().fill(Fill::X).padx(14, 14).pady(2, 4));
        b.label(c, &format!("If the school network blocks the download, get the new {} at home (or from the website) and use \
                             'Install from a file'.", if d.frozen { "Zenko.exe" } else { "zenko.py" }));
        b.gap(c);
    }

    fn sky(&mut self, b: &mut B, right: usize) {
        let th = b.th;
        let bg = b.bg.clone();
        let c = self.card(b, right, "Sky, seasons and motion", (12, 0));
        for (text, key) in [("Sky follows the time of day (soft dawn, day and dusk; night stays the shrine)", "sky_daynight"),
                            ("Seasons and weather in the sidebar and garden", "sky_seasons"),
                            ("Foxfire wisp when you switch pages", "page_fx"),
                            ("Dim foxfire progress glow in the window frame during focus", "focus_glow"),
                            ("The little fox at the top acts out the page you're on", "sidebar_living"),
                            ("Fox poses now and then (stretches, play bows, naps, pounces)", "fox_poses")] {
            b.check(c, text, key, true);
        }
        let l = b.lbl(c, "Study alerts and reminders are said in", &th.grey, 9);
        b.ui.set_anchor(l, Anchor::W);
        b.ui.pack(l, Pack::default().fill(Fill::X).padx(14, 14).pady(6, 0));
        let rv = b.get("remind_voice", "zenko");
        let fr = b.ui.frame(c, &bg);
        b.ui.pack(fr, Pack::default().fill(Fill::X).padx(14, 14).pady(0, 10));
        let mut g = vec![];
        for (lab, val) in [("Her voice (VOICEVOX)", "zenko"), ("Windows voice (English)", "english"), ("Silent", "off")] {
            let r = b.radio(fr, lab, rv == val, &format!("set_rad:remind_voice:s:{val}"), 10);
            b.ui.pack(r, Pack::default().side(Side::Left).padx(0, 10));
            g.push((r, val.to_string()));
        }
        self.radios.push(("remind_voice".into(), g));
    }

    fn speed(&mut self, b: &mut B, left: usize, d: &SetData) {
        let th = b.th;
        let bg = b.bg.clone();
        let c = self.card(b, left, "Speed and smoothness", (12, 0));
        b.check(c, "Get pages ready in the background (instant page switching)", "prebuild_pages", true);
        let l = b.note(c, "Motion level lives in Customize > Look and feel. On battery, Auto calms the animations.", &th.grey, 9,
                       false, 560);
        b.ui.pack(l, Pack::default().fill(Fill::X).padx(16, 16).pady(2, 2));
        let info = b.note(c, &d.speed, &th.grey, 9, false, 560);
        b.ui.pack(info, Pack::default().fill(Fill::X).padx(16, 16).pady(2, 4));
        self.labels.push(("speed".into(), info));
        let row = b.row(c, 12, (0, 12));
        let x = b.btn(row, "Clear picture cache", "set_btn:clear_art", Some(&mixhex(&bg, &th.ink, 0.08)), Some(&th.ink), 9, 10, 3);
        b.ui.pack(x, Pack::default().side(Side::Left));
        let c = self.card(b, left, "Pixel art and fonts", (12, 0));
        let cb = b.check(c, "High-resolution pixel art (same designs, smoother edges, many more pixels)", "hd_art", true);
        let _ = cb;
        b.check(c, "Shrine-style titles (Zen Old Mincho for page and card titles)", "title_font", true);
        let l = b.note(c, "The app font is Zen Maru Gothic: soft and round, made in Japan, and easy to read. Pick another \
                           one under Look > Font. Both fonts are free (SIL Open Font License) and come inside Zenko, so \
                           nothing gets installed on your PC.", &th.grey, 9, false, 560);
        b.ui.pack(l, Pack::default().fill(Fill::X).padx(16, 16).pady(2, 12));
    }

    fn linked(&mut self, b: &mut B, right: usize, d: &SetData) {
        let th = b.th;
        let bg = b.bg.clone();
        let c = self.card(b, right, "Linked apps: Zenko Browser", (12, 0));
        let stl = b.note(c, if d.zb_linked { "\u{25cf} Linked right now" } else { "\u{25cb} Not linked (open Zenko Browser)" },
                         if d.zb_linked { "#5fd38a" } else { &th.grey }, 9, false, 320);
        b.ui.pack(stl, Pack::default().fill(Fill::X).padx(14, 14).pady(2, 4));
        self.labels.push(("zb".into(), stl));
        b.check(c, "Link with Zenko Browser (share my calendar with its new tab)", "zb_link_on", true);
        b.check(c, "Let her notice what I'm reading there", "zb_reading_note", true);
        let l = b.note(c, "Both apps only read each other's small files in %APPDATA%\\FoxBuddy. Nothing goes online.", &th.grey,
                       8, false, 320);
        b.ui.pack(l, Pack::default().fill(Fill::X).padx(14, 14).pady(2, 10));
        let c2 = self.card(b, right, "Town square", (12, 0));
        b.check(c2, "Join the Town square when I'm online", "square_on", false);
        let l = b.lbl(c2, "Message server (ntfy). Leave as it is unless you run your own:", &th.grey, 9);
        b.ui.set_anchor(l, Anchor::W);
        b.ui.pack(l, Pack::default().fill(Fill::X).padx(14, 14).pady(6, 0));
        let sv = match b.cfg.get("square_server") { Some(v) if truthy(v) => s_of(v), _ => SQUARE_SERVER.into() };
        let e = b.ui.entry(c2, 20, b.font(10, false), &sv, &th.field);
        self.entries.push(("square_server".into(), e));
        b.ui.pack(e, Pack::default().fill(Fill::X).padx(14, 14).pady(2, 4).ipady(3));
        let nb = b.cfg.get("square_block").and_then(|v| v.as_array()).map(|a| a.len()).unwrap_or(0);
        let r = b.ui.frame(c2, &bg);
        b.ui.pack(r, Pack::default().fill(Fill::X).padx(14, 14).pady(2, 10));
        let bl = b.lbl(r, &format!("{nb} blocked"), &th.grey, 9);
        b.ui.pack(bl, Pack::default().side(Side::Left));
        self.labels.push(("blocked".into(), bl));
        let x = b.btn(r, "Unblock everyone", "set_btn:unblock", Some(&th.field), None, 10, 10, 3);
        b.ui.pack(x, Pack::default().side(Side::Right));
    }

    fn ema_plan(&mut self, b: &mut B, right: usize) {
        let th = b.th;
        let c = self.card(b, right, "Emas that plan themselves", (12, 0));
        b.check(c, "Find study time for homework-like emas", "ema_plan", true);
        let l = b.note(c, "An ema like \u{201c}finish bio IA by friday\u{201d} or \u{201c}revise chem\u{201d} gets sessions in your \
                           free half hours (from the Upcoming clock) before it's due, around your exam plan. Emas at a set \
                           time keep that time free. Say \u{201c}plan my emas\u{201d}, \u{201c}bio IA takes 3 hours\u{201d} or \
                           \u{201c}don't plan call mum\u{201d}.", &th.grey, 8, false, 320);
        b.ui.pack(l, Pack::default().fill(Fill::X).padx(14, 14).pady(2, 10));
    }

    fn lockin(&mut self, b: &mut B, right: usize, d: &SetData) {
        let th = b.th;
        let c = self.card(b, right, "Lock-in mode", (12, 0));
        let l = b.note(c, "The moment a distracting site or app comes to the front, she pounces on it, eats it, and sends you \
                           back to work. Exit any time: say 'unlock', right-click her > Exit lock-in, or hold Esc for 2 \
                           seconds.", &th.grey, 9, false, 420);
        b.ui.pack(l, Pack::default().fill(Fill::X).padx(14, 14).pady(2, 6));
        let row = b.row(c, 14, (0, 6));
        let l = b.lbl(row, "Length", &th.ink, 10);
        b.ui.pack(l, Pack::default().side(Side::Left));
        let mv = b.get("lockin_minutes", "50");
        let me = self.entry_for(b, row, "lockin_minutes", &mv, 4);
        b.ui.pack(me, Pack::default().side(Side::Left).padx(6, 6));
        let l = b.lbl(row, "min", &th.ink, 10);
        b.ui.pack(l, Pack::default().side(Side::Left));
        let st = b.note(c, "", &th.accd, 9, true, 0);
        self.labels.push(("lockin".into(), st));
        let x = b.btn(row, if d.lockin { "Exit lock-in" } else { "Start lock-in" }, "set_btn:lockin", Some(&th.acc), Some(&th.onacc),
                      10, 14, 2);
        b.ui.pack(x, Pack::default().side(Side::Left).padx(12, 0));
        b.ui.pack(st, Pack::default().fill(Fill::X).padx(14, 14));
        let l = b.lbl(c, "What she does with a distraction", &th.grey, 9);
        b.ui.set_anchor(l, Anchor::W);
        b.ui.pack(l, Pack::default().fill(Fill::X).padx(14, 14).pady(6, 0));
        let v = b.get("lockin_action", "smart");
        let mut g = vec![];
        for (lab, val) in [("Smart: close just that browser tab, hide other apps", "smart"), ("Hide it (minimise the window)", "minimise"),
                           ("Close it (tab or the whole app)", "close")] {
            let r = b.radio(c, lab, v == val, &format!("set_rad:lockin_action:s:{val}"), 10);
            b.ui.pack(r, Pack::default().fill(Fill::X).padx(14, 14));
            g.push((r, val.to_string()));
        }
        self.radios.push(("lockin_action".into(), g));
        b.check(c, "Start a focus session with it", "lockin_session", true);
        b.check(c, "Play the pounce-and-eat animation", "lockin_animation", true);
        let l = b.note(c, "Extra things that count in lock-in (one per line: a word in the window title, or an app like discord.exe). \
                           Your normal distracting sites always count too.", &th.grey, 9, false, 420);
        b.ui.pack(l, Pack::default().fill(Fill::X).padx(14, 14).pady(8, 2));
        let mut t = crate::tktext::Text::new(Font::new("Consolas", 10, false), &th.ink, &th.field);
        (t.width_ch, t.height_ln) = (40, 5);
        t.hl = 1;
        let items: Vec<String> = b.cfg.get("lockin_extra").and_then(|v| v.as_array()).map(|a| a.iter().map(s_of).collect())
            .unwrap_or_default();
        t.insert_end(&items.join("\n"), &[]);
        t.edit = Some(0);
        let tid = b.ui.text(c, t, &th.line);
        b.ui.pack(tid, Pack::default().anchor(Anchor::W).padx(14, 14).pady(2, 2));
        self.lock_text = tid;
        let r2 = b.row(c, 14, (4, 12));
        let x = b.btn(r2, "Save list", "set_btn:lockin_save", Some(&th.start), None, 9, 10, 2);
        b.ui.pack(x, Pack::default().side(Side::Left));
        let x = b.btn(r2, "Edit distracting sites...", "set_btn:distractions", Some(&th.field), None, 9, 10, 2);
        b.ui.pack(x, Pack::default().side(Side::Left).padx(8, 0));
    }

    fn life(&mut self, b: &mut B, left: usize, right: usize) {
        let th = b.th;
        let num_row = |b: &mut B, s: &mut SetPage, parent: usize, items: &[(&str, &str, &str, bool)]| {
            let r = b.row(parent, 14, (4, 6));
            for (text, key, default, clock) in items {
                let l = b.lbl(r, text, &th.ink, 10);
                b.ui.pack(l, Pack::default().side(Side::Left));
                let v = b.get(key, default);
                let e = s.entry_for(b, r, key, &v, if *clock { 6 } else { 4 });
                b.ui.pack(e, Pack::default().side(Side::Left).padx(4, 10));
            }
        };
        let c = self.card(b, left, "Desktop life (turn off anything distracting)", (12, 0));
        b.check(c, "Roaming (she walks about, naps on windows)", "roam", true);
        let l = b.lbl(c, "   never during focus sessions or lock-in", &th.grey, 8);
        b.ui.set_anchor(l, Anchor::W);
        b.ui.pack(l, Pack::default().fill(Fill::X).padx(14, 14));
        b.check(c, "Seasons (snow, sakura, fireflies, maple leaves)", "seasons", true);
        b.check(c, "Match the real weather in my home city", "seasons_weather", true);
        b.check(c, "Day and night (yawns, nudges me to bed)", "daynight", true);
        num_row(b, self, c, &[("Bedtime", "bedtime", "23:00", true), ("Wake up", "wake_time", "06:00", true)]);
        let c = self.card(b, left, "Wellbeing", (12, 0));
        b.check(c, "20-20-20 eye breaks", "remind_eyes", true);
        b.check(c, "Posture checks", "remind_posture", true);
        b.check(c, "Water reminders", "remind_water", true);
        num_row(b, self, c, &[("Eyes every", "remind_eyes_every", "20", false), ("posture", "remind_posture_every", "40", false),
                              ("water", "remind_water_every", "60", false)]);
        let l = b.lbl(c, "   minutes of screen time (a 5-min break resets them)", &th.grey, 8);
        b.ui.set_anchor(l, Anchor::W);
        b.ui.pack(l, Pack::default().fill(Fill::X).padx(14, 14));
        b.check(c, "Ask how I'm feeling once a day (mood journal)", "daily_checkin", true);
        let x = b.btn(c, "Breathing exercise", "set_btn:breathing", Some(&th.start), None, 9, 10, 1);
        b.ui.pack(x, Pack::default().anchor(Anchor::W).padx(14, 14).pady(6, 12));
        let c = self.card(b, right, "Study planner", (12, 0));
        b.check(c, "Auto-plan study before exams (on my timetable)", "planner", true);
        b.label(c, "Session length");
        let sl = match b.cfg.get("plan_session") { Some(v) if truthy(v) => s_of(v), _ => "50".into() };
        b.radios(c, &[("25 min", "25"), ("45 min", "45"), ("50 min", "50"), ("60 min", "60")], "plan_session", &sl, 'i',
                 &mut self.radios);
        b.label(c, "Most sessions per subject in one day");
        let dc = match b.cfg.get("plan_day_cap") { Some(v) if truthy(v) => s_of(v), _ => "4".into() };
        b.radios(c, &[("2", "2"), ("3", "3"), ("4", "4"), ("6", "6")], "plan_day_cap", &dc, 'i', &mut self.radios);
        let x = b.btn(c, "Mark my free hours on the clock", "set_btn:free_hours", Some(&th.start), None, 9, 10, 1);
        b.ui.pack(x, Pack::default().anchor(Anchor::W).padx(14, 14).pady(6, 12));
        let c = self.card(b, right, "Her mood, hunger and surprises", (12, 0));
        b.check(c, "Mood and hunger (focus feeds her)", "pet_needs", true);
        b.check(c, "Surprises (haiku, gifts, fox visitors)", "random_events", true);
        b.gap(c);
    }

    /// z19's draw(): the section chips with the gold mark under the current one
    pub fn draw_nav(&mut self, ui: &mut Ui, th: &Theme, f: &Faces, fonts: &dyn Fonts) {
        let w = ui.ws[self.nav].w as f64;
        let mut cv = Canvas::default();
        let mut x = 2.0;
        self.boxes.clear();
        let n = 9.max(crate::hubside::zk_size(self.zk_f, 9));
        let fnt = Font::new(&f.ui, n, false);
        for (i, (name, _)) in self.chips.iter().enumerate() {
            let on = i == self.cur;
            cv.text(x + 10.0, 15.0, name).anchor("w").font(fnt.clone())
                .fill(&if on { th.gold.clone() } else { mixhex(&th.paper, &th.ink, 0.55) });
            let bb = crate::canvasdraw::text_bbox(cv.items.last().unwrap(), fonts);
            let x1 = bb[2] as f64 + 10.0;
            if on {
                cv.line(vec![x + 10.0, 26.0, x1 - 10.0, 26.0]).fill(&mixhex(&th.paper, &th.gold, 0.7)).width(2.0);
                cv.poly(vec![x + 4.0, 15.0, x + 6.0, 13.0, x + 8.0, 15.0, x + 6.0, 17.0]).fill(&th.gold).outline("");
            }
            self.boxes.push((x, x1, i));
            x = x1 + 4.0;
            if x > w - 40.0 {
                break;
            }
        }
        cv.line(vec![0.0, 29.0, w, 29.0]).fill(&mixhex(&th.paper, &th.line, 0.6));
        *canvas_of(ui, self.nav) = cv;
    }

    /// follow(): the gold mark follows the scroll (the last chip whose card's top is within 40 pixels above the view)
    pub fn follow(&mut self, ui: &mut Ui, th: &Theme, f: &Faces, fonts: &dyn Fonts) {
        let top = ui.ws[self.sc.cv].yoff;
        let iy = ui.abs(self.sc.inner).1 + top;
        let mut best = 0;
        for (i, (_, fr)) in self.chips.iter().enumerate() {
            let y = ui.abs(*fr).1 + top - iy;
            if y - 40 <= top {
                best = i;
            }
        }
        if best != self.cur {
            self.cur = best;
            self.draw_nav(ui, th, f, fonts);
        }
    }

    /// jump(i): scroll so the chip's card is at the top
    pub fn jump(&mut self, ui: &mut Ui, i: usize) {
        let Some(&(_, fr)) = self.chips.get(i) else { return };
        let top = ui.ws[self.sc.cv].yoff;
        let y = ui.abs(fr).1 - ui.abs(self.sc.inner).1;
        let _ = top;
        self.sc.set_y(ui, (y - 8).max(0));
        self.cur = i;
    }

    /// nb_wrap(lab): each such label wraps to its card's width less 34, never wider than the page shows (the card's
    /// <Configure>, label by label, until nothing moves)
    fn nb_wrap(&self, ui: &mut Ui, fonts: &dyn Fonts, pw: i32, ph: i32) {
        for _ in 0..4 {
            let mut moved = false;
            for (k, l) in &self.labels {
                if k != "nb_wrap" {
                    continue;
                }
                let par = ui.ws[*l].parent.unwrap();
                let mut w = ui.ws[par].w;
                let cv = self.sc.cv;
                if ui.ws[cv].w > 50 {
                    w = w.min(ui.ws[cv].w - 0.max(ui.abs(par).0 - ui.abs(cv).0) - 4);
                }
                let nw = 160.max(w - 34);
                if let Kind::Label { wrap, .. } = &mut ui.ws[*l].kind
                    && *wrap != nw {
                    *wrap = nw;
                    moved = true;
                    ui.layout_at(fonts, pw, ph);
                }
            }
            if !moved {
                break;
            }
        }
    }

    /// zk_fit for the page, the banner and the chips; lays out at the window's size (pw, ph)
    #[allow(clippy::too_many_arguments)]
    pub fn fit(&mut self, ui: &mut Ui, fonts: &dyn Fonts, th: &Theme, f: &Faces, zk_f: f64, pw: i32, ph: i32) {
        self.zk_f = zk_f;
        zk_fonts(ui, fonts, self.pg, zk_f);
        ui.layout_at(fonts, pw, ph);
        let w = ui.ws[self.title].w;
        if w >= 20 {
            let mut cv = std::mem::take(canvas_of(ui, self.title));
            let hh = hub_title_draw(&mut cv, w, TITLE, SUB, th, &f.ui, &f.title, zk_f, fonts, &mut self.seig);
            *canvas_of(ui, self.title) = cv;
            if ui.ws[self.title].opt_h != hh {
                ui.ws[self.title].opt_h = hh;
                ui.layout_at(fonts, pw, ph);
            }
        }
        zk_wrap(ui, fonts, self.pg, &mut self.wrap, pw, ph);
        self.nb_wrap(ui, fonts, pw, ph);
        if self.sc.sync(ui) {
            ui.layout_at(fonts, pw, ph);
        }
        self.draw_nav(ui, th, f, fonts);
    }
}

/// zenko.py's APP_VERSION, the version this port shows and matches
pub const APP_VERSION: &str = "v2026.11.25-zenko23";
/// UPDATE_SOURCE_DEFAULT
pub const UPDATE_SOURCE_DEFAULT: &str = "AnimatedAFK/Zenko";

/// STT_MODELS[key][1]: the speech model's download in MB
pub fn stt_mb(key: &str) -> i64 {
    if key == "accurate" { 181 } else { 57 }
}

impl crate::Companion {
    /// what the Settings page shows, from the app (the window adds FONT_UI and its own timing line)
    pub fn set_data(&mut self) -> SetData {
        let cfg = self.cfg.raw.clone();
        let exe_dir = std::env::current_exe().ok().and_then(|p| p.parent().map(|d| d.display().to_string()))
            .unwrap_or_default();
        let src = cfg.get("update_source").and_then(|v| v.as_str()).unwrap_or("").trim().to_string();
        let exam = self.exam_days();
        let today = (crate::unix_now() + crate::local_offset_pub() as f64).div_euclid(86400.0) as i64;
        SetData {
            is_win: cfg!(windows),
            autostart: false,
            spotify: false,
            lockin: self.lockin.is_some(),
            zb_linked: self.zb_linked(),
            whisper: "Whisper isn't installed for the Python I can see, so Windows' built-in speech is used (less \
                      accurate). Press Install Whisper.".into(),
            stt: format!("Not set up yet. Set up downloads about {} MB once (GitHub + Hugging Face).",
                         5 + stt_mb(cfg.get("stt_model").and_then(|v| v.as_str()).unwrap_or("fast"))),
            version: format!("You have {APP_VERSION}. Running from {exe_dir}."),
            live: None,
            install: true,
            frozen: true,
            speed: String::new(),
            exam_iso: crate::plan::iso(today + exam),
            update_source: if src.is_empty() { UPDATE_SOURCE_DEFAULT.into() } else { src },
            rule_try: ("\u{2713} allowed (no rule needed)".into(), String::new()),
            font_ui: String::new(),
            cfg,
        }
    }
}
