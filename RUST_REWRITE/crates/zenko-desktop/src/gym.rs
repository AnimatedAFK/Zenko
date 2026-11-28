//! ZenkoGym.gym_route (the Exercise tab by chat): weight, food (the built-in table: food_find, food_estimate,
//! food_sane), water, which split day today is, sleep, and "what's my workout".

use crate::chat::capitalize;
use crate::plan::iso;
use crate::pydifflib::get_close_matches;
use crate::rewards::{Later, local_days};
use crate::routes17::routes_data;
use crate::session::clip;
use crate::{Companion, read_json};
use fancy_regex::Regex as FRegex;
use regex::Regex;
use serde_json::{Value, json};
use std::sync::OnceLock;

macro_rules! rx {
    ($p:expr) => {{
        static R: OnceLock<Regex> = OnceLock::new();
        R.get_or_init(|| Regex::new($p).unwrap())
    }};
}
macro_rules! frx {
    ($p:expr) => {{
        static R: OnceLock<FRegex> = OnceLock::new();
        R.get_or_init(|| FRegex::new($p).unwrap())
    }};
}

pub const GYM_WATER_GOAL: i64 = 8;

/// one FOODS row: (names, serving, grams per serving, kcal per serving, protein per serving)
#[derive(Clone, Debug)]
pub struct Food {
    pub names: Vec<String>,
    pub serving: String,
    pub g: f64,
    pub kcal: f64,
    pub prot: f64,
}

/// FOODS[i]
pub fn food_row(i: usize) -> Food {
    foods()[i].clone()
}

fn foods() -> &'static Vec<Food> {
    static F: OnceLock<Vec<Food>> = OnceLock::new();
    F.get_or_init(|| {
        routes_data()["FOODS"].as_array().unwrap().iter().map(|e| Food {
            names: e[0].as_array().unwrap().iter().map(|n| n.as_str().unwrap().to_string()).collect(),
            serving: e[1].as_str().unwrap().into(),
            g: e[2].as_f64().unwrap(),
            kcal: e[3].as_f64().unwrap(),
            prot: e[4].as_f64().unwrap(),
        }).collect()
    })
}

/// _food_words: lower-case words, simple plurals made singular
pub fn food_words(text: &str) -> Vec<String> {
    let low = text.to_lowercase();
    let cleaned: String = low.chars().map(|c| if c.is_ascii_lowercase() || c == ' ' { c } else { ' ' }).collect();
    cleaned.split_whitespace().map(|w| {
        let n = w.chars().count();
        if n > 4 && w.ends_with("ies") {
            if !["cookies", "smoothies", "brownies"].contains(&w) { format!("{}y", &w[..w.len() - 3]) } else { w[..w.len() - 1].to_string() }
        } else if n > 4 && (w.ends_with("ches") || w.ends_with("shes") || w.ends_with("oes")) {
            w[..w.len() - 2].to_string()
        } else if n > 3 && w.ends_with('s') && !(w.ends_with("ss") || w.ends_with("us") || w.ends_with("is")) {
            w[..w.len() - 1].to_string()
        } else {
            w.to_string()
        }
    }).collect()
}

struct Index {
    rows: Vec<(usize, String, String)>,
    singles: Vec<String>,
}

fn index() -> &'static Index {
    static I: OnceLock<Index> = OnceLock::new();
    I.get_or_init(|| {
        let mut rows = vec![];
        for (i, e) in foods().iter().enumerate() {
            for n in &e.names {
                rows.push((i, n.clone(), food_words(n).join(" ")));
            }
        }
        let mut singles: Vec<String> = rows.iter().map(|r| r.2.clone()).filter(|nn| !nn.contains(' ')).collect();
        singles.sort();
        singles.dedup();
        Index { rows, singles }
    })
}

/// food_find: the best table match in text, (row, matched name)
pub fn food_find(text: &str) -> Option<(usize, String)> {
    let ix = index();
    let mut words = vec![];
    for w in food_words(text) {
        let mut w = w;
        if w.chars().count() >= 5 && !ix.singles.contains(&w) {
            if let Some(m) = get_close_matches(&w, &ix.singles, 1, 0.8).into_iter().next() {
                w = m;
            }
        }
        words.push(w);
    }
    let t = format!(" {} ", words.join(" "));
    let mut best: Option<(usize, String)> = None;
    let mut key: (i64, i64) = (-1, -1);
    for (e, n, nn) in &ix.rows {
        if let Some(k) = t.find(&format!(" {nn} ")) {
            // Python counts characters; these strings are ASCII
            let sc = ((k + nn.len()) as i64, nn.len() as i64);
            if sc > key {
                best = Some((*e, n.clone()));
                key = sc;
            }
        }
    }
    best
}

/// round(x, n) the way Python does (the exact double, half to even)
fn pyround(x: f64, n: usize) -> f64 {
    format!("{x:.n$}").parse().unwrap()
}

/// food_sane: no single food is 230,000 kcal
pub fn food_sane(kcal: f64, prot: f64) -> (i64, f64) {
    let kcal = (kcal.round_ties_even() as i64).clamp(0, 3000);
    let cap = if kcal != 0 { kcal as f64 / 4.0 } else { 0.0 };
    let prot = 0f64.max(200f64.min(prot).min(cap));
    (kcal, pyround(prot, 1))
}

/// fmt_num of a float
pub fn fmt_float(v: f64) -> String {
    if (v - v.round()).abs() < 1e-9 && v.abs() < 1e15 {
        return crate::routes17::fmt_int(v.round() as i64);
    }
    let s = format!("{:.6}", v);
    let (i, f) = s.split_once('.').unwrap();
    let ii: i64 = i.parse().unwrap_or(0);
    let mut out = if i.starts_with("-0") { "-0".to_string() } else { crate::routes17::fmt_int(ii) };
    let f = f.trim_end_matches('0');
    if !f.is_empty() {
        out.push('.');
        out.push_str(f);
    }
    out
}

pub fn clean_num(s: &str) -> String {
    let s = frx!(r"(\d),(\d{3})(?!\d)").replace_all(s, "$1$2").into_owned();
    frx!(r"(\d),(\d{1,2})(?!\d)").replace_all(&s, "$1.$2").into_owned()
}

/// food_estimate: ([(label, kcal, protein)], unknown parts)
pub fn food_estimate(text: &str) -> (Vec<(String, i64, f64)>, Vec<String>) {
    let mut text = clean_num(text);
    text = rx!(r"\([^)]*\)").replace_all(&text, |c: &regex::Captures| c[0].replace(',', " ").replace(" and ", " & ")).into_owned();
    for e in foods() {
        for n in &e.names {
            if n.contains(" and ") {
                let r = Regex::new(&format!("(?i){}", regex::escape(n))).unwrap();
                text = r.replace_all(&text, regex::NoExpand(&n.replace(" and ", "_n_"))).into_owned();
            }
        }
    }
    let parts: Vec<String> = frx!(r"(?i),|;|\n|\+|\band\b|\bwith\b|(?<!\()&").split(&text)
        .filter_map(|p| p.ok())
        .filter(|p| !p.trim().is_empty())
        .map(|p| p.trim().replace("_n_", " and "))
        .collect();
    let (mut out, mut unknown) = (vec![], vec![]);
    let fd = foods();
    for p in parts {
        let low = p.to_lowercase();
        let ek = rx!(r"(\d+(?:\.\d+)?)\s*(?:kcal|calories|calorie|cals?)\b").captures(&low);
        let ep = rx!(r"(\d+(?:\.\d+)?)\s*g(?:rams?)?\s*(?:of\s+)?protein").captures(&low);
        let stripped = rx!(r"\d+(?:\.\d+)?\s*(?:kcal|calories|calorie|cals?|g(?:rams?)?\s*(?:of\s+)?protein)\b").replace_all(&low, " ");
        let e = food_find(&stripped).map(|x| &fd[x.0]);
        let label = capitalize(p.trim());
        if let Some(ek) = ek {
            let kcal: f64 = ek[1].parse().unwrap();
            let prot = match &ep {
                Some(c) => c[1].parse().unwrap(),
                None => match e {
                    Some(e) if e.kcal != 0.0 => kcal * (e.prot / e.kcal),
                    _ => kcal * 0.04,
                },
            };
            let (k, pr) = food_sane(kcal, prot);
            out.push((label, k, pr));
            continue;
        }
        let Some(e) = e else {
            unknown.push(p);
            continue;
        };
        let rest = rx!(r"\d+(?:\.\d+)?\s*g(?:rams?)?\s*(?:of\s+)?protein").replace_all(&low, " ").into_owned();
        let mut qty = 1.0;
        let mut grams: Option<f64> = None;
        let mg = rx!(r"(\d+(?:\.\d+)?)\s*(kg|g|grams?|gm|ml|l|oz)\b").captures(&rest);
        let mq = frx!(r"(?<![\d.])(\d+(?:\.\d+)?|\d+/\d+)(?!\s*(?:kg|g|grams?|gm|ml|l|oz)\b)(?![\d.])").captures(&rest).ok().flatten();
        let mut cnt: Option<f64> = None;
        if let Some(mq) = mq {
            let v = mq.get(1).unwrap().as_str();
            let c = match v.split_once('/') {
                Some((a, b)) => a.parse::<f64>().unwrap() / b.parse::<f64>().unwrap(),
                None => v.parse().unwrap(),
            };
            cnt = if c > 0.0 && c <= 20.0 { Some(c) } else { None };
        }
        if let Some(mg) = mg {
            let unit = match &mg[2] {
                "kg" | "l" => 1000.0,
                "oz" => 28.35,
                _ => 1.0,
            };
            let mut g = 2500f64.min(mg[1].parse::<f64>().unwrap() * unit);
            if let Some(c) = cnt.filter(|_| rx!(r"\beach\b").is_match(&rest)) {
                g = 2500f64.min(g * c);
            }
            grams = Some(g);
        } else {
            if let Some(c) = cnt {
                qty = c;
            } else {
                // sorted(_FOOD_NUM.items(), key=-len): longest first, dict order among equals
                const ORDER: [(&str, f64); 16] = [("a couple of", 2.0), ("couple of", 2.0), ("a half", 0.5), ("double", 2.0),
                    ("three", 3.0), ("a few", 3.0), ("four", 4.0), ("five", 5.0), ("half", 0.5), ("some", 1.0),
                    ("one", 1.0), ("two", 2.0), ("six", 6.0), ("few", 3.0), ("an", 1.0), ("a", 1.0)];
                for (w, v) in ORDER.iter() {
                    if Regex::new(&format!(r"\b{}\b", regex::escape(w))).unwrap().is_match(&rest) {
                        qty = *v;
                        break;
                    }
                }
            }
            if rx!(r"\b(?:large|big|huge)\b").is_match(&rest) {
                qty *= 1.4;
            } else if rx!(r"\bsmall\b").is_match(&rest) && !e.serving.contains("small bag") {
                qty *= 0.7;
            }
        }
        let k = match grams {
            None => qty,
            Some(g) => g / e.g,
        };
        let (k_, mut p_) = food_sane(e.kcal * k, e.prot * k);
        if let Some(ep) = &ep {
            p_ = food_sane(k_ as f64, ep[1].parse().unwrap()).1;
        }
        let what = match grams {
            Some(g) if g != 0.0 => format!("{}g", g as i64),
            _ => format!("{} \u{d7} {}", fmt_float(pyround(qty, 2)), e.serving),
        };
        out.push((format!("{label} ({what})"), k_, p_));
    }
    (out, unknown)
}

impl Companion {
    /// gym(): the Exercise tab's part of zenko_life.json, with its setdefaults
    pub fn gym_do<R>(&mut self, f: impl FnOnce(&mut Value) -> R) -> R {
        self.life_do(|l| {
            if !l["gym"].is_object() {
                l["gym"] = json!({});
            }
            let g = &mut l["gym"];
            for (k, d) in [("next", Value::Null), ("log", json!([])), ("weights", json!([])), ("food", json!({})),
                           ("checks", json!({})), ("profile", json!({"age": 17})), ("water", json!({})), ("sleep", json!({}))] {
                if g.get(k).is_none() {
                    g[k] = d;
                }
            }
            if !g.get("day_v2").is_some_and(truthy) {
                g["next"] = Value::Null;
                g["day_v2"] = json!(1);
            }
            if !g.get("food_v2").is_some_and(truthy) {
                g["food_v2"] = json!(1);
                gym_food_fix(g);
            }
            f(g)
        })
    }

    pub fn gym_trend_text(w: &[Value]) -> String {
        if w.len() < 2 {
            return "Logged. Weigh in once or twice a week, same time of day, and I'll show your trend~".into();
        }
        let day = |x: &Value| crate::plan::parse_iso(x["date"].as_str().unwrap_or("")).unwrap_or(0);
        let last = &w[w.len() - 1];
        let d0 = day(last);
        let old: Vec<&Value> = w.iter().filter(|x| d0 - day(x) >= 13).collect();
        let r = old.last().copied().unwrap_or(&w[0]);
        let days = 1.max(d0 - day(r));
        let ch = last["kg"].as_f64().unwrap_or(0.0) - r["kg"].as_f64().unwrap_or(0.0);
        let wk = ch / days as f64 * 7.0;
        let txt = format!("{ch:+.1} kg over {days} days (about {wk:+.2} kg a week). ");
        let tail = if days < 10 {
            "Day-to-day weight jumps with water and food, so the 2-week trend matters most."
        } else if wk < -0.75 {
            "That's faster than ideal for a growing body. Eat a little more, especially protein."
        } else if wk < -0.1 {
            "A steady, healthy pace. Keep going~"
        } else if wk <= 0.25 {
            "Holding steady. If you're getting stronger, that can mean muscle is replacing fat."
        } else {
            "Going up a bit. Check your portions and snacks, and keep training."
        };
        txt + tail
    }

    fn gym_food_today_sum(&self) -> i64 {
        let today = iso(local_days());
        read_json("zenko_life.json")["gym"]["food"][today.as_str()].as_array()
            .map(|a| a.iter().map(|i| i["kcal"].as_f64().unwrap_or(0.0)).sum::<f64>() as i64).unwrap_or(0)
    }

    pub(crate) fn gym_route(&mut self, now: f64, text: &str) -> Option<String> {
        let raw = text.trim();
        let lw = raw.to_lowercase();
        let today = iso(local_days());
        if let Some(m) = rx!(r"^(?:i\s+)?(?:now\s+)?(?:weigh|weight(?: is)?|am)\s+(\d{2,3}(?:\.\d)?)\s*(kg|kgs|kilos?|lbs?|pounds)\b").captures(&lw) {
            let kg: f64 = m[1].parse::<f64>().unwrap() * if m[2].starts_with("lb") || m[2].starts_with("pound") { 0.4536 } else { 1.0 };
            if (30.0..=250.0).contains(&kg) {
                let t = today.clone();
                let w = self.gym_do(|g| {
                    let mut w: Vec<Value> = g["weights"].as_array().cloned().unwrap_or_default().into_iter()
                        .filter(|x| x["date"] != t.as_str()).collect();
                    w.push(json!({"date": t, "kg": pyround(kg, 1)}));
                    w.sort_by(|a, b| a["date"].as_str().cmp(&b["date"].as_str()));
                    let cut = w.len().saturating_sub(400);
                    w.drain(..cut);
                    g["weights"] = Value::Array(w.clone());
                    w
                });
                return Some(format!("Logged {kg:.1} kg~ {}", Self::gym_trend_text(&w)));
            }
        }
        if let Some(m) = rx!(r"^(?:i\s+)?(?:just\s+)?(ate|eaten|had|log food:?|food:?)\s+(.+?)(?:\s+for\s+(?:breakfast|lunch|dinner|a snack|snack))?[.!]*$").captures(&lw) {
            let verb = &m[1];
            if ["ate", "eaten", "log food", "log food:", "food", "food:"].contains(&verb) || food_find(&m[2]).is_some() {
                let what = m[2].to_string();
                let (items, unknown) = food_estimate(&what);
                let t = today.clone();
                self.gym_do(|g| {
                    if !g["food"][t.as_str()].is_array() {
                        g["food"][t.as_str()] = json!([]);
                    }
                    let f = g["food"][t.as_str()].as_array_mut().unwrap();
                    for (lab, kcal, prot) in &items {
                        f.push(json!({"t": lab, "kcal": kcal, "p": prot, "src": "table"}));
                    }
                    let mut keys: Vec<String> = g["food"].as_object().unwrap().keys().cloned().collect();
                    keys.sort();
                    let n = keys.len().saturating_sub(60);
                    for k in &keys[..n] {
                        g["food"].as_object_mut().unwrap().shift_remove(k);
                    }
                });
                if !unknown.is_empty() {
                    // (zenko.py looks the rest up online; this build has no lookup yet, so it answers like a failed one)
                    let web = self.cfg.b("web_enabled", true);
                    let miss = unknown.iter().map(|u| clip(u, 30)).collect::<Vec<_>>().join(", ");
                    let msg = format!("I couldn't find {miss}{}. Try a simpler name with a weight, like '150 g rice', or tell me \
                                       the calories: 'protein bar 210 kcal 20 g protein'.",
                                      if web { "" } else { " (internet lookups are off in Settings)" });
                    self.later(now + 0.5, Later::FoxReply(msg));
                    return Some("Itadakimasu! Estimating that for you...".into());
                }
                let kcal: i64 = items.iter().map(|i| i.1).sum();
                return Some(format!("Logged about {} kcal. Today so far: {} kcal~", crate::routes17::fmt_int(kcal),
                                    crate::routes17::fmt_int(self.gym_food_today_sum())));
            }
        }
        let wm = rx!(r"^(?:i\s+)?(?:just\s+)?(?:drank|had|drink)\s+(?:a\s+|one\s+|(\d)\s+)?(?:glass(?:es)?|cups?|bottles?)?\s*(?:of\s+)?water\b").captures(&lw);
        if wm.is_some() || ["water", "drank water", "+water", "+1 water"].contains(&lw.as_str()) {
            let n: i64 = wm.as_ref().and_then(|m| m.get(1)).map(|x| x.as_str().parse().unwrap()).unwrap_or(1);
            let t = today.clone();
            let v = self.gym_do(|g| {
                let v = 20.min(g["water"][t.as_str()].as_i64().unwrap_or(0) + n);
                g["water"][t.as_str()] = json!(v);
                v
            });
            return Some(format!("Gulp gulp~ {v} of {GYM_WATER_GOAL} glasses today."));
        }
        if let Some(m) = rx!(r"\b(?:today(?:'s| is)?|it'?s|its)\s+(?:a\s+|my\s+)?(?:day\s*([123])|(chest|push|back|pull|legs?|rest|off)\s*day)\b|^(chest|push|back|pull|legs?|rest|off)\s+day\s+today\b|^i'?m\s+(resting)\s+today\b").captures(&lw) {
            let w = m.get(2).or(m.get(3)).or(m.get(4)).map(|x| x.as_str()).unwrap_or("");
            let i: i64 = match m.get(1) {
                Some(d) => d.as_str().parse::<i64>().unwrap() - 1,
                None => match w {
                    "chest" | "push" => 0,
                    "back" | "pull" => 1,
                    "rest" | "off" | "resting" => 3,
                    _ => 2,
                },
            };
            self.gym_do(|g| g["next"] = json!(i));
            if i == 3 {
                return Some("Got it~ Today is your rest day. Stretch, walk, eat your protein and sleep early~".into());
            }
            let d = &routes_data()["GYM_ALL"][i as usize];
            return Some(format!("Got it~ Today is {}\u{306e}\u{65e5} (day {}): {}. Ganbatte!", d[0].as_str().unwrap(), i + 1,
                                d[1].as_str().unwrap().to_lowercase()));
        }
        if let Some(m) = rx!(r"^(?:i\s+)?slept\s+(?:(?:really\s+)?(badly|bad|poorly|terribly|okay|ok|fine|well|good|great|amazing)|(\d)\s*/\s*5)").captures(&lw) {
            let n: i64 = match m.get(2) {
                Some(d) => d.as_str().parse().unwrap(),
                None => match &m[1] {
                    "terribly" | "badly" => 1,
                    "bad" | "poorly" => 2,
                    "okay" | "ok" | "fine" => 3,
                    "well" | "good" => 4,
                    _ => 5,
                },
            }
            .clamp(1, 5);
            let t = today.clone();
            self.gym_do(|g| g["sleep"][t.as_str()] = json!(n));
            return Some(format!("Noted: sleep {n}/5. {}", if n <= 2 { "Go gentle today~" } else { "Nice, a rested fox is a strong fox~" }));
        }
        if rx!(r"\b(?:workout|gym day|gym today|what(?:'s| is) (?:my |today'?s )?(?:workout|training|gym)|leg day|open (?:the )?(?:exercise|gym))\b").is_match(&lw) {
            self.later(now + 0.15, Later::Command("open_hub:exercise".into()));
            let t = self.gym_do(|g| g["next"].as_i64().map(|n| n.rem_euclid(4)));
            let Some(t) = t else {
                return Some("Which day is today? Pick it in Exercise, or tell me 'today is back day' (or 'rest day')~".into());
            };
            let all = &routes_data()["GYM_ALL"];
            if t == 3 {
                return Some(format!("Today is your rest day~ Muscles grow while you rest. Tomorrow: {}.",
                                    all[0][1].as_str().unwrap().to_lowercase()));
            }
            let d = &all[t as usize];
            return Some(format!("Today is {}\u{306e}\u{65e5} (day {}): {}. Ganbatte~", d[0].as_str().unwrap(), t + 1,
                                d[1].as_str().unwrap().to_lowercase()));
        }
        None
    }
}

fn truthy(v: &Value) -> bool {
    match v {
        Value::Null | Value::Bool(false) => false,
        Value::Number(n) => n.as_f64() != Some(0.0),
        Value::String(s) => !s.is_empty(),
        Value::Array(a) => !a.is_empty(),
        Value::Object(o) => !o.is_empty(),
        _ => true,
    }
}

/// gym_food_fix: re-read food logged by the old portion reader
fn gym_food_fix(g: &mut Value) {
    let Some(food) = g["food"].as_object().cloned() else { return };
    for (day, f) in food {
        let mut out = vec![];
        for it in f.as_array().cloned().unwrap_or_default() {
            if !it.is_object() {
                continue;
            }
            if it["src"] == "table" {
                let t = match &it["t"] {
                    Value::String(s) => s.clone(),
                    Value::Null => String::new(),
                    v => v.to_string(),
                };
                let base = if t.ends_with(')') && t.contains(" (") { t.rsplit_once(" (").unwrap().0.to_string() } else { t };
                let (items, _) = food_estimate(&base);
                if !items.is_empty() {
                    for (lab, k, p) in items {
                        out.push(json!({"t": lab, "kcal": k, "p": p, "src": "table"}));
                    }
                    continue;
                }
            }
            let mut it = it;
            let (k, p) = food_sane(it["kcal"].as_f64().unwrap_or(0.0), it["p"].as_f64().unwrap_or(0.0));
            it["kcal"] = json!(k);
            it["p"] = json!(p);
            out.push(it);
        }
        g["food"][day.as_str()] = Value::Array(out);
    }
}
