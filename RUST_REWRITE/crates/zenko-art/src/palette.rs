//! Palettes: palette letter -> colour. Ported from zenko.py (_PAL_BASE, _fur, FUR, _mixhex,
//! ZenkoExtras.make_pal and Zenko19.make_pal).

pub use crate::palette_data::*;

pub type Rgb = [u8; 3];

#[derive(Clone, Debug, PartialEq)]
pub struct Palette {
    pub map: [Option<Rgb>; 128],
}

pub fn hex_rgb(h: &str) -> Rgb {
    let h = h.trim_start_matches('#');
    let p = |i: usize| u8::from_str_radix(&h[i..i + 2], 16).unwrap_or(0);
    [p(0), p(2), p(4)]
}
pub fn rgb_hex(c: Rgb) -> String {
    format!("#{:02x}{:02x}{:02x}", c[0], c[1], c[2])
}

/// zenko.py _mixhex
pub fn mixhex(c1: &str, c2: &str, t: f64) -> String {
    let (a, b) = (hex_rgb(c1), hex_rgb(c2));
    let v: Vec<i64> = (0..3)
        .map(|i| (a[i] as f64 * (1.0 - t) + b[i] as f64 * t) as i64)
        .collect();
    format!(
        "#{:02x}{:02x}{:02x}",
        v[0].clamp(0, 255),
        v[1].clamp(0, 255),
        v[2].clamp(0, 255)
    )
}

impl Palette {
    pub fn empty() -> Palette {
        Palette { map: [None; 128] }
    }
    pub fn set(&mut self, k: u8, hex: &str) {
        self.map[k as usize] = Some(hex_rgb(hex));
    }
    pub fn get(&self, k: u8) -> Option<Rgb> {
        self.map.get(k as usize).copied().flatten()
    }
    pub fn hex(&self, k: u8) -> Option<String> {
        self.get(k).map(rgb_hex)
    }
    /// FUR[name] (falls back to "snow" like `FUR.get(theme) or SNOW_PAL`).
    pub fn fur(name: &str) -> Palette {
        let mut p = Palette::empty();
        for (k, v) in PAL_BASE {
            p.set(*k, v);
        }
        let entry = FUR
            .iter()
            .find(|(n, _)| *n == name)
            .or_else(|| FUR.iter().find(|(n, _)| *n == "snow"))
            .unwrap();
        for (k, v) in entry.1.iter() {
            p.set(*k, v);
        }
        p
    }
    pub fn from_pairs(pairs: &[(u8, &str)]) -> Palette {
        let mut p = Palette::empty();
        for (k, v) in pairs {
            p.set(*k, v);
        }
        p
    }
}

/// The look settings make_pal reads from config.json.
#[derive(Clone, Debug, Default)]
pub struct LookCfg {
    pub theme: String,
    pub coat: String,
    pub eye_color: String,
    pub foxfire_color: String,
    pub scarf: String,
    /// zenko19: the robe she currently wears (self._robe_cur), "" for none
    pub robe: String,
}

/// The fox's colours: the theme, then her own coat, eyes, foxfire and bib on top (and the robe colours).
pub fn make_pal(c: &LookCfg) -> Palette {
    let theme = if c.theme.is_empty() {
        "snow"
    } else {
        c.theme.as_str()
    };
    let mut pal = Palette::fur(theme);
    if let Some((_, _, fur)) = COATS.iter().find(|(k, _, _)| *k == c.coat) {
        if !fur.is_empty() && FUR.iter().any(|(n, _)| n == fur) {
            let src = Palette::fur(fur);
            for &k in COAT_KEYS {
                if let Some(v) = src.get(k) {
                    pal.map[k as usize] = Some(v);
                }
            }
        }
    }
    if !c.eye_color.is_empty() {
        pal.set(b'e', &c.eye_color);
        pal.set(b'q', &mixhex(&c.eye_color, "#000000", 0.42));
    }
    if !c.foxfire_color.is_empty() {
        let f = &c.foxfire_color;
        pal.set(b'U', f);
        pal.set(b'I', &mixhex(f, "#ffffff", 0.45));
        pal.set(b'A', &mixhex(f, "#ffffff", 0.85));
    }
    if !c.scarf.is_empty() {
        pal.set(b'S', &c.scarf);
        pal.set(b's', &mixhex(&c.scarf, "#000000", 0.28));
    }
    if let Some(r) = ROBES.iter().find(|r| r.key == c.robe) {
        for (k, v) in r.cols {
            pal.set(*k, v);
        }
    }
    pal
}

pub fn robe_of<'a>(acc: &[&'a str]) -> Option<&'a str> {
    acc.iter()
        .copied()
        .find(|a| ROBES.iter().any(|r| r.key == *a))
}
pub fn robe_motif(key: &str) -> &'static str {
    ROBES
        .iter()
        .find(|r| r.key == key)
        .map(|r| r.motif)
        .unwrap_or("")
}
pub fn is_robe(a: &str) -> bool {
    ROBES.iter().any(|r| r.key == a)
}
