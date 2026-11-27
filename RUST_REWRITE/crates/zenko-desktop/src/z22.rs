//! Zenko22.gym_route's chat commands: fishing, the koi book, passport, badges, Town square, Zenko Browser link.

use crate::rewards::{Later, journey_state};
use crate::session::clip;
use crate::{Companion, read_json, unix_now};
use regex::Regex;
use serde_json::Value;
use std::sync::OnceLock;

macro_rules! rx {
    ($p:expr) => {{
        static R: OnceLock<Regex> = OnceLock::new();
        R.get_or_init(|| Regex::new($p).unwrap())
    }};
}

/// len(FISH)
pub const FISH_KINDS: usize = 32;

impl Companion {
    fn zb_now() -> serde_json::Map<String, Value> {
        match read_json("browser_now.json") {
            Value::Object(o) => o,
            _ => Default::default(),
        }
    }

    pub fn zb_linked(&self) -> bool {
        if !self.cfg.b("zb_link_on", true) {
            return false;
        }
        let d = Self::zb_now();
        let t = match d.get("time") {
            Some(Value::Number(n)) => n.as_f64(),
            Some(Value::String(s)) => s.trim().parse().ok(),
            Some(Value::Null) | None => Some(0.0),
            Some(Value::Bool(b)) => Some(*b as i64 as f64),
            _ => None,
        };
        !d.is_empty() && t.is_some_and(|t| unix_now() - t < 75.0)
    }

    pub fn zb_status_text(&self) -> String {
        if !self.cfg.b("zb_link_on", true) {
            return "The Zenko Browser link is switched off in Settings.".into();
        }
        let d = Self::zb_now();
        if d.is_empty() {
            return "I can't see Zenko Browser yet. Open it and we link up by ourselves~".into();
        }
        if !self.zb_linked() {
            return "Zenko Browser isn't open right now. When it is, this light glows.".into();
        }
        let pick = |k: &str| d.get(k).and_then(|v| v.as_str()).filter(|s| !s.is_empty()).map(String::from);
        let t = clip(&pick("title").or_else(|| pick("host")).unwrap_or_else(|| "a new tab".into()), 60);
        format!("Linked with Zenko Browser {}~ You're on \u{201c}{t}\u{201d}. Right-click any page there to summarise it, \
                 quiz yourself or add dates to me.", pick("version").unwrap_or_default())
    }

    pub(crate) fn z22_route(&mut self, now: f64, text: &str) -> Option<String> {
        let low = text.to_lowercase();
        let low = low.trim_matches([' ', '.', '!', '?', '~']);
        let lw = rx!(r"^(?:hey |hi |ok |okay )?(?:zenko[, ]+)?(?:please |pls |can you |could you |let'?s )?").replace(low, "").into_owned();
        let hub = |me: &mut Self, p: &str| me.later(now, Later::Command(format!("open_hub:{p}")));
        if rx!(r"^(?:go )?fish(?:ing)?$|^cast(?: a| the)? line$|^catch (?:a )?(?:fish|koi)$").is_match(&lw) {
            hub(self, "garden");
            self.later(now + 0.6, Later::Command("fish_cast".into()));
            return Some("To the pond~ Answer right and something bites!".into());
        }
        if rx!(r"^(?:my |open )?(?:koi|fish) ?(?:book|collection|dex)$").is_match(&lw) {
            self.later(now, Later::Command("koi_book".into()));
            let n = read_json("zenko_life.json")["fishing"]["book"].as_object().map(|o| o.len()).unwrap_or(0);
            return Some(format!("Here's your koi book: {n} of {FISH_KINDS} kinds caught~"));
        }
        if rx!(r"^(?:my |open )?(?:passport|stamps|journey|world map)$").is_match(&lw) {
            hub(self, "map");
            let n = journey_state(&read_json("zenko_life.json")).1.len();
            return Some(format!("Your passport~ {n} places stamped so far."));
        }
        if rx!(r"^(?:my |open )?(?:badges|ema badges|achievements)$").is_match(&lw) {
            hub(self, "daily");
            let n = read_json("zenko_life.json")["badges"].as_object().map(|o| o.len()).unwrap_or(0);
            return Some(format!("Here are your ema badges~ {n} earned."));
        }
        if rx!(r"^(?:open )?(?:town square|global chat|square)$").is_match(&lw) {
            hub(self, "square");
            return Some("The Town square~ Be kind out there!".into());
        }
        if rx!(r"^(?:browser|zenko browser|are you linked|browser link)\??$").is_match(&lw) {
            return Some(self.zb_status_text());
        }
        None
    }
}
