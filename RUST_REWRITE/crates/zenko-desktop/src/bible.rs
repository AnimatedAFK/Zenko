//! zenko.py's Bible helpers that need no window: _bible_strip (with html.unescape), step_parse_line (STEPBible TAHOT /
//! TAGNT word rows), the bundled brief lexicons (data/lexicon.json), step_lookup, BibleWindow.morph_text and the
//! CG50 key helpers that sit beside them in data/tools.json.

use crate::ptable::tools;
use crate::routes17::bible_book;
use regex::Regex;
use serde_json::{Map, Value, json};
use std::collections::HashMap;
use std::sync::OnceLock;

macro_rules! rx {
    ($p:expr) => {{
        static R: OnceLock<Regex> = OnceLock::new();
        R.get_or_init(|| Regex::new($p).unwrap())
    }};
}

/// BIBLE_BOOKS[i] = (code, name, chapters, slug, group)
pub fn book(i: usize) -> (&'static str, &'static str, i64, &'static str, &'static str) {
    let b = &tools()["BIBLE_BOOKS"][i];
    (b[0].as_str().unwrap(), b[1].as_str().unwrap(), b[2].as_i64().unwrap(), b[3].as_str().unwrap(), b[4].as_str().unwrap())
}

pub fn n_books() -> usize {
    tools()["BIBLE_BOOKS"].as_array().unwrap().len()
}

/// html.unescape (numeric references and the named entities Bible sites use)
pub fn html_unescape(s: &str) -> String {
    if !s.contains('&') {
        return s.to_string();
    }
    let re = rx!(r"&(#[0-9]+;?|#[xX][0-9a-fA-F]+;?|[^\t\n\f <&#;]{1,32};?)");
    re.replace_all(s, |c: &regex::Captures| {
        let g = &c[1];
        if let Some(num) = g.strip_prefix('#') {
            let num = num.trim_end_matches(';');
            let v = if let Some(h) = num.strip_prefix(['x', 'X']) { u32::from_str_radix(h, 16).ok() } else { num.parse::<u32>().ok() };
            let v = v.unwrap_or(0x110000);
            const CP1252: [u32; 32] = [0x20ac, 0x81, 0x201a, 0x0192, 0x201e, 0x2026, 0x2020, 0x2021, 0x02c6, 0x2030, 0x0160, 0x2039,
                                       0x0152, 0x8d, 0x017d, 0x8f, 0x90, 0x2018, 0x2019, 0x201c, 0x201d, 0x2022, 0x2013, 0x2014,
                                       0x02dc, 0x2122, 0x0161, 0x203a, 0x0153, 0x9d, 0x017e, 0x0178];
            if (0x80..=0x9f).contains(&v) {
                return char::from_u32(CP1252[(v - 0x80) as usize]).unwrap().to_string();
            }
            if v == 0 || (0xd800..=0xdfff).contains(&v) || v > 0x10ffff {
                return "\u{fffd}".into();
            }
            return char::from_u32(v).map(|c| c.to_string()).unwrap_or_default();
        }
        const NAMED: [(&str, &str); 24] = [("amp", "&"), ("lt", "<"), ("gt", ">"), ("quot", "\""), ("apos", "'"), ("nbsp", "\u{a0}"),
            ("copy", "©"), ("reg", "®"), ("mdash", "—"), ("ndash", "–"), ("lsquo", "‘"), ("rsquo", "’"), ("ldquo", "“"),
            ("rdquo", "”"), ("hellip", "…"), ("para", "¶"), ("sect", "§"), ("middot", "·"), ("deg", "°"), ("laquo", "«"),
            ("raquo", "»"), ("shy", "\u{ad}"), ("times", "×"), ("divide", "÷")];
        const NO_SEMI: [&str; 10] = ["amp", "lt", "gt", "quot", "nbsp", "copy", "reg", "para", "sect", "deg"];
        if let Some(n) = g.strip_suffix(';') {
            if let Some((_, v)) = NAMED.iter().find(|(k, _)| *k == n) {
                return v.to_string();
            }
        }
        // the longest entity name (without ';') that starts the text
        for k in (1..=g.len().min(32)).rev() {
            if !g.is_char_boundary(k) {
                continue;
            }
            let (a, rest) = g.split_at(k);
            if NO_SEMI.contains(&a) {
                return NAMED.iter().find(|(n, _)| *n == a).unwrap().1.to_string() + rest;
            }
        }
        format!("&{g}")
    }).into_owned()
}

/// _bible_strip(t)
pub fn bible_strip(t: &str) -> String {
    let t = rx!(r"<S>\s*\d+\s*</S>").replace_all(t, "");
    let t = rx!(r"(?s)<sup>.*?</sup>").replace_all(&t, "");
    let t = rx!(r"<br\s*/?>").replace_all(&t, " ");
    let t = rx!(r"<[^>]+>").replace_all(&t, "");
    let t = html_unescape(&t);
    rx!(r"\s+").replace_all(&t, " ").trim().to_string()
}

/// step_parse_line(line) -> (book, chapter, verse, word dict)
pub fn step_parse_line(line: &str) -> Option<(usize, i64, i64, Value)> {
    let line = line.trim_end_matches(['\r', '\n']);
    let cols: Vec<&str> = line.split('\t').collect();
    if cols.len() < 4 {
        return None;
    }
    let m = rx!(r"^([1-3]?[A-Za-z]{2,3})\.(\d+)\.(\d+)(?:[(\[{][^)\]}]*[)\]}])?#(\d+)(?:=([\w()]+))?").captures(cols[0].trim())?;
    let bi = bible_book(&m[1])?;
    let (ch, vs): (i64, i64) = (m[2].parse().ok()?, m[3].parse().ok()?);
    let kind = m.get(5).map(|x| x.as_str()).unwrap_or("");
    let col = |i: usize| cols.get(i).copied();
    if bi < 39 {
        let word = cols[1].replace(['/', '\\'], "").trim().to_string();
        let translit = col(2).map(|c| c.replace('/', "").trim().to_string()).unwrap_or_default();
        let gloss = col(3).map(|c| rx!(r"\s*/\s*").replace_all(c, " ").trim().to_string()).unwrap_or_default();
        let ds = col(4).map(|c| c.trim()).unwrap_or("");
        let gram = col(5).map(|c| c.trim()).unwrap_or("");
        let strong = match rx!(r"\{([HAG]\d{4}[A-Za-z]?)\}").captures(ds) {
            Some(c) => c[1].to_string(),
            None => rx!(r"[HAG]\d{4}[A-Za-z]?").find_iter(ds).last().map(|m| m.as_str().to_string()).unwrap_or_default(),
        };
        let lang = if gram.starts_with('A') { "Aramaic" } else { "Hebrew" };
        let mut lemma = String::new();
        if !strong.is_empty() {
            let re = Regex::new(&format!("{}=([^=]+)=", regex::escape(&strong))).unwrap();
            for c in cols.iter().skip(6) {
                if let Some(mm) = re.captures(c) {
                    lemma = mm[1].to_string();
                    break;
                }
            }
        }
        return Some((bi, ch, vs, json!({"w": word, "t": translit, "g": gloss, "s": strong, "m": gram, "l": lemma, "lang": lang})));
    }
    let gk = cols[1].trim();
    let (word, translit) = match rx!(r"^(?s)(.*?)\s*\(([^)]*)\)\s*$").captures(gk) {
        Some(mm) => (mm[1].to_string(), mm[2].to_string()),
        None => (gk.to_string(), String::new()),
    };
    let gloss = col(2).map(|c| c.trim()).unwrap_or("");
    let sg = col(3).map(|c| c.trim()).unwrap_or("");
    let (strong, gram) = sg.split_once('=').unwrap_or((sg, ""));
    let (lemma, lg) = col(4).and_then(|c| c.split_once('=')).unwrap_or(("", ""));
    let variant = !kind.is_empty() && !kind.contains('N') && !kind.contains('K');
    Some((bi, ch, vs, json!({"w": word, "t": translit, "g": gloss, "s": strong.trim(), "m": gram.trim(), "l": lemma.trim(),
                             "lg": lg.trim(), "lang": "Greek", "var": variant})))
}

fn bundle() -> &'static Value {
    static D: OnceLock<Value> = OnceLock::new();
    D.get_or_init(|| serde_json::from_str(include_str!("../data/lexicon.json")).unwrap_or(Value::Object(Map::new())))
}

/// _lex_bundled(key): {strong: [lemma, translit, gloss, meaning]}
pub fn lex_bundled(key: &str) -> HashMap<String, Value> {
    let b = &bundle()[key];
    let mut lex: HashMap<String, Value> = b["e"].as_object().map(|o| o.iter().map(|(k, v)| (k.clone(), v.clone())).collect()).unwrap_or_default();
    if let Some(a) = b["a"].as_object() {
        for (k, v) in a {
            if let Some(e) = v.as_str().and_then(|v| lex.get(v)).cloned() {
                lex.insert(k.clone(), e);
            }
        }
    }
    lex
}

/// step_lexicon(lang) from the bundle (zenko.py uses a downloaded TBESH/TBESG.json only when one bigger than the
/// bundle is on disk; Zenko ships the bundle, so that's the one every lookup reads)
pub fn step_lexicon(lang: &str) -> &'static HashMap<String, Value> {
    static H: OnceLock<HashMap<String, Value>> = OnceLock::new();
    static G: OnceLock<HashMap<String, Value>> = OnceLock::new();
    if lang == "Greek" { G.get_or_init(|| lex_bundled("TBESG")) } else { H.get_or_init(|| lex_bundled("TBESH")) }
}

pub fn step_lookup<'a>(lex: &'a HashMap<String, Value>, strong: &str) -> Option<&'a Value> {
    if strong.is_empty() {
        return None;
    }
    let no_suffix = rx!(r"[A-Za-z]$").replace(strong, "").into_owned();
    let first5: String = strong.chars().take(5).collect();
    for k in [strong.to_string(), no_suffix, first5] {
        if let Some(v) = lex.get(&k) {
            return Some(v);
        }
    }
    None
}

/// BibleWindow.morph_text(m, lang)
pub fn morph_text(m: &str, lang: &str) -> String {
    if m.is_empty() {
        return String::new();
    }
    let mut out: Vec<&str> = vec![];
    let sub = |s: &str, a: usize, b: usize| -> String { s.chars().skip(a).take(b - a).collect() };
    if lang == "Greek" {
        let parts: Vec<&str> = m.split('-').collect();
        let pos = match parts[0] {
            "N" => "noun", "V" => "verb", "A" => "adjective", "T" => "article", "P" => "personal pronoun", "C" => "conjunction",
            "ADV" => "adverb", "PREP" => "preposition", "CONJ" => "conjunction", "R" => "relative pronoun", "D" => "demonstrative",
            "PRT" => "particle", "I" => "interrogative", _ => "",
        };
        if !pos.is_empty() {
            out.push(pos);
        }
        if m.starts_with("V-") {
            let p = parts.get(1).copied().unwrap_or("");
            let tense = match sub(p, 0, 1).as_str() {
                "P" => "present", "I" => "imperfect", "F" => "future", "A" => "aorist", "R" => "perfect", "L" => "pluperfect", _ => "",
            };
            let voice = match sub(p, 1, 2).as_str() {
                "A" => "active", "M" => "middle", "P" => "passive", "E" => "middle/passive", "D" => "middle deponent",
                "N" => "middle/passive deponent", "O" => "passive deponent", _ => "",
            };
            let mood = match sub(p, 2, 3).as_str() {
                "I" => "indicative", "S" => "subjunctive", "O" => "optative", "M" => "imperative", "N" => "infinitive",
                "P" => "participle", _ => "",
            };
            out.extend([tense, voice, mood].into_iter().filter(|x| !x.is_empty()));
        }
    } else {
        let parts: Vec<&str> = m.split('/').collect();
        let mut main = parts[0];
        for p in &parts {
            if p.chars().count() > main.chars().count() {
                main = p;
            }
        }
        let first = sub(main, 0, 1);
        let core: String = if !first.is_empty() && "HA".contains(first.as_str()) { main.chars().skip(1).collect() } else { main.to_string() };
        let pos = match sub(&core, 0, 1).as_str() {
            "N" => "noun", "V" => "verb", "A" => "adjective", "R" => "preposition", "C" => "conjunction", "D" => "adverb",
            "P" => "pronoun", "T" => "particle", "S" => "suffix", _ => "",
        };
        if !pos.is_empty() {
            out.push(pos);
        }
        if sub(&core, 0, 1) == "V" && core.chars().count() > 2 {
            let stem = match sub(&core, 1, 2).as_str() {
                "q" => "Qal", "N" => "Niphal", "p" => "Piel", "P" => "Pual", "h" => "Hiphil", "H" => "Hophal", "t" => "Hithpael", _ => "",
            };
            let form = match sub(&core, 2, 3).as_str() {
                "p" => "perfect", "q" => "sequential perfect", "i" => "imperfect", "w" => "wayyiqtol (past narrative)",
                "v" => "imperative", "r" => "participle", "s" => "passive participle", "c" => "infinitive construct",
                "a" => "infinitive absolute", "j" => "jussive", "h" => "cohortative", _ => "",
            };
            out.extend([stem, form].into_iter().filter(|x| !x.is_empty()));
        }
    }
    if out.is_empty() { m.to_string() } else { format!("{}  ({m})", out.join(", ")) }
}

/// CasioWindow.tokens(s): the edit line split into whole keys (multi-character inserts stay one token)
pub fn cg_tokens(s: &str) -> Vec<String> {
    static MULTI: OnceLock<Vec<String>> = OnceLock::new();
    let multi = MULTI.get_or_init(|| {
        let t = tools();
        let mut v: Vec<String> = t["CG_INS"].as_object().unwrap().values().chain(t["CG_SHIFT"].as_object().unwrap().values())
            .map(|x| x.as_str().unwrap().to_string()).collect();
        v.sort();
        v.dedup();
        // sorted(set(...), key=len, reverse=True): set order is arbitrary but only equal-length ties differ, and two
        // different strings of the same length can't both start at the same place
        v.sort_by_key(|x| std::cmp::Reverse(x.chars().count()));
        v
    });
    let mut out = vec![];
    let mut rest = s;
    while let Some(c) = rest.chars().next() {
        if let Some(m) = multi.iter().find(|m| m.chars().count() > 1 && rest.starts_with(m.as_str())) {
            out.push(m.clone());
            rest = &rest[m.len()..];
        } else {
            out.push(c.to_string());
            rest = &rest[c.len_utf8()..];
        }
    }
    out
}

/// CasioWindow.poly_form(n)
pub fn poly_form(n: usize) -> String {
    let sup = |p: usize| match p { 2 => "²", 3 => "³", 4 => "⁴", 5 => "⁵", 6 => "⁶", _ => "" };
    let parts: Vec<String> = "abcdefg".chars().take(n + 1).enumerate().map(|(i, l)| {
        let p = n - i;
        format!("{l}{}", if p > 1 { format!("x{}", sup(p)) } else if p == 1 { "x".into() } else { String::new() })
    }).collect();
    parts.join("+") + "=0"
}
