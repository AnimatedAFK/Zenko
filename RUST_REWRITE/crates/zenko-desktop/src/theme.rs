//! The app colours (zenko.py _realm, THEMES, apply_theme + the zenko20 accent override, Z22_BUBBLES).

use crate::Cfg;
use zenko_art::palette::mixhex;

#[derive(Clone, Debug)]
pub struct Theme {
    pub ink: String,
    pub paper: String,
    pub acc: String,
    pub accd: String,
    pub grey: String,
    pub card: String,
    pub night: String,
    pub gold: String,
    pub glow: String,
    pub onacc: String,
    pub field: String,
    pub line: String,
    pub bub: String,
    pub bubink: String,
    pub good: String,
    pub bad: String,
    pub night2: String,
    pub banner: String,
    pub edge: String,
    pub hover: String,
    pub foot: String,
    pub start: String,
    pub nowbg: String,
    pub soonbg: String,
    pub donebg: String,
    pub missbg: String,
    pub donebtn: String,
    pub delbtn: String,
}

#[allow(clippy::too_many_arguments)]
fn realm(night: &str, paper: &str, card: &str, ink: &str, grey: &str, acc: &str, accd: &str, gold: &str, glow: &str,
         onacc: &str) -> Theme {
    Theme {
        ink: ink.into(),
        paper: paper.into(),
        acc: acc.into(),
        accd: accd.into(),
        grey: grey.into(),
        card: card.into(),
        night: night.into(),
        gold: gold.into(),
        glow: glow.into(),
        onacc: onacc.into(),
        field: mixhex(card, ink, 0.07),
        line: mixhex(card, ink, 0.16),
        bub: mixhex(night, card, 0.35),
        bubink: ink.into(),
        good: "#5fd39a".into(),
        bad: "#ff6b7d".into(),
        night2: mixhex(night, acc, 0.16),
        banner: mixhex(paper, acc, 0.10),
        edge: mixhex(card, ink, 0.30),
        hover: mixhex(card, ink, 0.12),
        foot: mixhex(paper, acc, 0.05),
        start: mixhex(card, acc, 0.24),
        nowbg: mixhex(card, acc, 0.17),
        soonbg: mixhex(card, gold, 0.07),
        donebg: mixhex(card, "#5fd39a", 0.13),
        missbg: mixhex(card, "#ff6b7d", 0.13),
        donebtn: mixhex(card, "#5fd39a", 0.34),
        delbtn: mixhex(card, "#ff6b7d", 0.32),
    }
}

const THEMES: [(&str, [&str; 10]); 8] = [
    ("snow", ["#0b0f23", "#11162f", "#191f42", "#eef1ff", "#98a1cd", "#6fb6ff", "#a4d3ff", "#e9c46a", "#7fd8ff", "#06122a"]),
    ("classic", ["#130a0f", "#1a1015", "#25161d", "#fff1e2", "#c1a49a", "#ff7a3d", "#ffad7d", "#f4c25b", "#6fd6ff", "#1e0b05"]),
    ("ember", ["#110607", "#190b0b", "#251212", "#fff0e8", "#c69c90", "#ff5a3a", "#ff9c7d", "#ffcf4d", "#ffb43a", "#1d0602"]),
    ("fennec", ["#110e08", "#18140c", "#241e13", "#fff6e6", "#bba988", "#e9b45a", "#f6d293", "#f0c96a", "#56e0b4", "#1d1405"]),
    ("midnight", ["#08071a", "#0e0c23", "#181633", "#f1eeff", "#9c96ca", "#a28cff", "#c9bcff", "#e3b04b", "#b9a6ff", "#120a33"]),
    ("sakura", ["#140914", "#1b0d1c", "#28152a", "#fff0f7", "#c89cb6", "#ff86bd", "#ffb6d8", "#f3c56b", "#ff9fd0", "#2a0718"]),
    ("mint", ["#05110d", "#0a1914", "#11261e", "#ecfff7", "#90baa9", "#3fd6a4", "#8aeecb", "#e9c46a", "#5ff0c0", "#03140d"]),
    ("silver", ["#0c0e13", "#12151c", "#1b1f28", "#f2f5fb", "#9ba4b6", "#8fb0e8", "#bfd2f6", "#d8c08a", "#a8c4ff", "#0b1222"]),
];

const ACCENTS: [(&str, [&str; 4]); 7] = [
    ("gold", ["#e9b45a", "#f6d293", "#f0c96a", "#ffd88a"]),
    ("vermilion", ["#ff6a3d", "#ffa27d", "#f4c25b", "#ff9a6a"]),
    ("sakura", ["#ff86bd", "#ffb6d8", "#f3c56b", "#ff9fd0"]),
    ("jade", ["#3fd6a4", "#8aeecb", "#e9c46a", "#5ff0c0"]),
    ("sky", ["#6fb6ff", "#a4d3ff", "#e9c46a", "#7fd8ff"]),
    ("wisteria", ["#a28cff", "#c9bcff", "#e3b04b", "#b9a6ff"]),
    ("moon", ["#8fb0e8", "#bfd2f6", "#d8c08a", "#a8c4ff"]),
];

/// THEMES.get(name) or THEMES["snow"] (no accent).
pub fn theme_named(name: &str) -> Theme {
    let t = THEMES.iter().find(|(k, _)| *k == name).unwrap_or(&THEMES[0]).1;
    realm(t[0], t[1], t[2], t[3], t[4], t[5], t[6], t[7], t[8], t[9])
}

/// apply_theme(cfg["theme"]) with the accent from cfg["accent"] on top.
pub fn theme(cfg: &Cfg) -> Theme {
    let name = cfg.s("theme", "snow");
    let t = THEMES.iter().find(|(k, _)| *k == name).unwrap_or(&THEMES[0]).1;
    let acc = cfg.s("accent", "theme");
    if let Some((_, a)) = ACCENTS.iter().find(|(k, _)| *k == acc) {
        return realm(t[0], t[1], t[2], t[3], t[4], a[0], a[1], a[2], a[3], t[9]);
    }
    // zenko22: the accent colours unlocked on the journey join ACCENTS
    if let Some(row) = crate::rewards::rdata()["Z22_ACCENTS"].as_array().and_then(|v| v.iter().find(|r| r[0] == acc.as_str())) {
        if crate::rewards::unlocked_accents().contains(&acc) {
            let c: Vec<&str> = (0..4).map(|i| row[2][i].as_str().unwrap_or("#000000")).collect();
            return realm(t[0], t[1], t[2], t[3], t[4], c[0], c[1], c[2], c[3], t[9]);
        }
    }
    realm(t[0], t[1], t[2], t[3], t[4], t[5], t[6], t[7], t[8], t[9])
}

/// z22_bub_style: (fill, ink, rim, inner rim or None).
pub fn bubble_style(cfg: &Cfg, t: &Theme) -> (String, String, String, Option<String>) {
    let s: Option<[&str; 4]> = match cfg.s("bubble_style", "").as_str() {
        "shinto" => Some(["#fbf8f1", "#2a1c18", "#d23b2a", "#efb6a8"]),
        "kinpaku" => Some(["#2b2112", "#fff1cc", "#f2c94c", "#a8842e"]),
        "hanji" => Some(["#f1e6cf", "#2f2418", "#8a6a44", "#d4c09a"]),
        "lacquer" => Some(["#5a1410", "#ffe9d6", "#e0b43a", "#9a3a22"]),
        "silk" => Some(["#1d2452", "#eef0ff", "#e0b43a", "#4d5aa0"]),
        "cloud" => Some(["#e9f1ff", "#1f2a44", "#a9c4f0", "#ffffff"]),
        "foxfire" => Some(["#1c1238", "#e6f4ff", "#7fd8ff", "#4a4aa8"]),
        _ => None,
    };
    match s {
        Some(s) => (s[0].into(), s[1].into(), s[2].into(), Some(s[3].into())),
        None => (t.bub.clone(), t.bubink.clone(), t.gold.clone(), None),
    }
}
