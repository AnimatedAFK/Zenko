//! zenko.py's Bible on disk and on the network: bible_chapter (bolls.life, then bible-api.com, then the KJV, cached
//! per chapter), the STEPBible index and sections (_step_index, _step_files_for, step_download_file split into
//! per-chapter files, _orig_path's format check), step_have_all / step_download_all, bible_original and the lexicon
//! that a downloaded file bigger than the bundle replaces.

use crate::bible::{book, n_books, step_parse_line};
use crate::routes17::bible_book;
use crate::web::{download, http_json, q, quote};
use regex::Regex;
use serde_json::{Map, Value, json};
use std::collections::{BTreeSet, HashMap};
use std::path::PathBuf;
use std::sync::{Arc, Mutex, OnceLock};

macro_rules! rx {
    ($p:expr) => {{
        static R: OnceLock<Regex> = OnceLock::new();
        R.get_or_init(|| Regex::new($p).unwrap())
    }};
}

pub const ORIG_FORMAT: &str = "2";
pub const TRANS: [(&str, &str); 5] = [("NKJV", "New King James Version (online, \u{a9} Thomas Nelson)"),
                                       ("KJV", "King James Version (public domain)"), ("WEB", "World English Bible (public domain)"),
                                       ("ASV", "American Standard Version 1901 (public domain)"),
                                       ("YLT", "Young's Literal Translation (public domain, very literal)")];

pub fn notice(t: &str) -> &'static str {
    match t {
        "NKJV" => "Scripture taken from the New King James Version\u{ae}. Copyright \u{a9} 1982 by Thomas Nelson. Used by \
                   permission. All rights reserved. (Fetched chapter by chapter for your personal reading.)",
        "KJV" => "King James Version (public domain).",
        "WEB" => "World English Bible (public domain).",
        "ASV" => "American Standard Version (public domain).",
        "YLT" => "Young's Literal Translation (public domain).",
        _ => "",
    }
}

fn src(key: &str) -> String {
    let (env, def) = match key {
        "bolls" => ("FOX_BOLLS", "https://bolls.life"),
        "bibleapi" => ("FOX_BIBLEAPI", "https://bible-api.com"),
        "gh_api" => ("FOX_STEP_API", "https://api.github.com/repos/STEPBible/STEPBible-Data/contents"),
        _ => ("FOX_STEP_RAW", "https://raw.githubusercontent.com/STEPBible/STEPBible-Data/master"),
    };
    std::env::var(env).unwrap_or_else(|_| def.into())
}

const MIRRORS: [&str; 1] = ["https://cdn.jsdelivr.net/gh/STEPBible/STEPBible-Data@master"];

pub fn guess(key: &str) -> Vec<String> {
    let v: &[&str] = match key {
        "TAHOT" => &["Translators Amalgamated OT+NT/TAHOT Gen-Deu - Translators Amalgamated Hebrew OT - STEPBible.org CC BY.txt",
                     "Translators Amalgamated OT+NT/TAHOT Jos-Est - Translators Amalgamated Hebrew OT - STEPBible.org CC BY.txt",
                     "Translators Amalgamated OT+NT/TAHOT Job-Sng - Translators Amalgamated Hebrew OT - STEPBible.org CC BY.txt",
                     "Translators Amalgamated OT+NT/TAHOT Isa-Mal - Translators Amalgamated Hebrew OT - STEPBible.org CC BY.txt"],
        "TAGNT" => &["Translators Amalgamated OT+NT/TAGNT Mat-Jhn - Translators Amalgamated Greek NT - STEPBible.org CC-BY.txt",
                     "Translators Amalgamated OT+NT/TAGNT Act-Rev - Translators Amalgamated Greek NT - STEPBible.org CC-BY.txt"],
        "TBESH" => &["Lexicons/TBESH - Translators Brief lexicon of Extended Strongs for Hebrew - STEPBible.org CC BY.txt"],
        "TBESG" => &["Lexicons/TBESG - Translators Brief lexicon of Extended Strongs for Greek - STEPBible.org CC BY.txt"],
        _ => &[],
    };
    v.iter().map(|s| s.to_string()).collect()
}

pub fn bible_dir() -> PathBuf {
    crate::data_dir().join("bible")
}

fn now() -> f64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs_f64()).unwrap_or(0.0)
}

/// _STEP: the index, whether the format was checked, the lexicons read from disk
#[derive(Default)]
struct Step {
    index: Option<Value>,
    checked: bool,
    lex: HashMap<&'static str, Arc<HashMap<String, Value>>>,
}

fn step() -> std::sync::MutexGuard<'static, Step> {
    static S: OnceLock<Mutex<Step>> = OnceLock::new();
    S.get_or_init(Default::default).lock().unwrap_or_else(|e| e.into_inner())
}

/// forget everything cached (a test's fresh start)
pub fn reset() {
    *step() = Step::default();
}

/// bible_chapter(bi, ch, trans, allow_net): ([(verse, text)], the translation used)
pub fn bible_chapter(bi: usize, ch: i64, trans: &str, allow_net: bool) -> Result<(Vec<(i64, String)>, String), String> {
    let (code, name, ..) = book(bi);
    let path = bible_dir().join("text").join(trans).join(code).join(format!("{ch}.json"));
    if let Some(d) = std::fs::read_to_string(&path).ok().and_then(|s| serde_json::from_str::<Value>(&s).ok()) {
        if let Some(vs) = d["verses"].as_array().filter(|a| !a.is_empty()) {
            return Ok((vs.iter().map(|v| (v[0].as_i64().unwrap_or(0), v[1].as_str().unwrap_or("").to_string())).collect(),
                       trans.into()));
        }
    }
    if !allow_net {
        return Err("This chapter isn't saved on this PC yet, and I'm offline.".into());
    }
    let mut verses: Vec<(i64, String)> = vec![];
    let mut err: Option<&str> = None;
    let bolls = src("bolls");
    for url in [format!("{bolls}/get-text/{trans}/{}/{ch}/", bi + 1), format!("{bolls}/get-chapter/{trans}/{}/{ch}/", bi + 1)] {
        match http_json(&url, 15.0) {
            Ok(Value::Array(j)) if !j.is_empty() => {
                let mut out = vec![];
                let mut bad = false;
                for v in &j {
                    let vv = &v["verse"];
                    if vv.is_null() || vv == &json!(0) || vv == &json!("") {
                        continue;
                    }
                    match vv.as_i64().or_else(|| vv.as_str().and_then(|s| s.trim().parse().ok())) {
                        Some(n) => out.push((n, crate::bible::bible_strip(v["text"].as_str().unwrap_or("")))),
                        None => bad = true,
                    }
                }
                if bad {
                    err = Some("ValueError");
                    continue;
                }
                verses = out;
                break;
            }
            Ok(_) => {}
            Err(_) => err = Some("WebError"),
        }
    }
    if verses.is_empty() && (trans == "KJV" || trans == "WEB") {
        match http_json(&format!("{}/{}+{ch}?translation={}", src("bibleapi"), q(name), trans.to_lowercase()), 15.0) {
            Ok(j) => {
                verses = j["verses"].as_array().into_iter().flatten()
                    .filter_map(|v| Some((v["verse"].as_i64()?, crate::bible::bible_strip(v["text"].as_str().unwrap_or("")))))
                    .collect();
            }
            Err(_) => err = Some("WebError"),
        }
    }
    if verses.is_empty() {
        if trans != "KJV" {
            return bible_chapter(bi, ch, "KJV", allow_net);
        }
        return Err(format!("I couldn't fetch {name} {ch} ({}). The Bible sites may be blocked on this network.",
                           err.unwrap_or("no answer")));
    }
    verses.sort();
    if let Some(d) = path.parent() {
        let _ = std::fs::create_dir_all(d);
        let _ = std::fs::write(&path, serde_json::to_string(&json!({"verses": verses, "t": now()})).unwrap_or_default());
    }
    Ok((verses, trans.into()))
}

/// _step_index(): {'TAHOT': [path...], ...} from GitHub's listing (cached 30 days), or the published names
pub fn step_index() -> Value {
    if let Some(ix) = step().index.clone() {
        return ix;
    }
    let p = bible_dir().join("step_index.json");
    if let Some(ix) = std::fs::read_to_string(&p).ok().and_then(|s| serde_json::from_str::<Value>(&s).ok()) {
        if now() - ix["t"].as_f64().unwrap_or(0.0) < 30.0 * 86400.0 && ix["TAHOT"].as_array().is_some_and(|a| !a.is_empty()) {
            step().index = Some(ix.clone());
            return ix;
        }
    }
    let mut ix = Map::new();
    ix.insert("t".into(), json!(now()));
    let api = src("gh_api");
    if let Ok(Value::Array(root)) = http_json(&api, 15.0) {
        let dirs: Vec<String> = root.iter()
            .filter(|d| d["type"] == "dir" && Regex::new(r"(?i)amalgamated|lexicon").unwrap().is_match(d["name"].as_str().unwrap_or("")))
            .filter_map(|d| d["path"].as_str().map(String::from)).collect();
        'outer: for d in dirs {
            match http_json(&format!("{api}/{}", quote(&d, "/")), 15.0) {
                Ok(Value::Array(fs)) => {
                    for f in fs {
                        let nm = f["name"].as_str().unwrap_or("");
                        for key in ["TAHOT", "TAGNT", "TBESH", "TBESG"] {
                            if nm.starts_with(key) && nm.to_lowercase().ends_with(".txt") {
                                let e = ix.entry(key).or_insert_with(|| json!([]));
                                e.as_array_mut().unwrap().push(f["path"].clone());
                            }
                        }
                    }
                }
                _ => break 'outer,
            }
        }
    }
    for k in ["TAHOT", "TAGNT", "TBESH", "TBESG"] {
        if !ix.get(k).and_then(|v| v.as_array()).is_some_and(|a| !a.is_empty()) {
            ix.insert(k.into(), json!(guess(k)));
            ix.insert("guessed".into(), json!(true));
        }
    }
    let ix = Value::Object(ix);
    if ix.get("guessed").is_none() {
        let _ = std::fs::create_dir_all(bible_dir());
        let _ = std::fs::write(&p, serde_json::to_string(&ix).unwrap_or_default());
    }
    step().index = Some(ix.clone());
    ix
}

fn base_name(p: &str) -> &str {
    p.rsplit(['/', '\\']).next().unwrap_or(p)
}

/// _step_files_for(bi): the files whose book range holds this book, narrowest range first
pub fn step_files_for(bi: usize) -> Vec<String> {
    let kind = if bi < 39 { "TAHOT" } else { "TAGNT" };
    let ix = step_index();
    let mut all: Vec<String> = ix[kind].as_array().into_iter().flatten().filter_map(|v| v.as_str().map(String::from)).collect();
    all.extend(guess(kind));
    let mut hits: Vec<(i64, String)> = vec![];
    for path in all {
        let Some(m) = rx!(r"TA[HG][A-Z]T\s+([1-3]?[A-Za-z]{2,3})-([1-3]?[A-Za-z]{2,3})").captures(base_name(&path)) else { continue };
        let (a, b) = (bible_book(&m[1]), bible_book(&m[2]));
        // (`path not in hits` compares a path with the (width, path) pairs, so it never skips one)
        if let (Some(a), Some(b)) = (a, b) {
            if a <= bi && bi <= b {
                hits.push((b as i64 - a as i64, path));
            }
        }
    }
    hits.sort_by_key(|h| h.0);
    let mut out: Vec<String> = vec![];
    for (_, p) in hits {
        if !out.contains(&p) {
            out.push(p);
        }
    }
    out
}

pub fn step_file_for(bi: usize) -> Option<String> {
    step_files_for(bi).into_iter().next()
}

/// _orig_path(bi, ch), clearing saves of an older format once
pub fn orig_path(bi: usize, ch: i64) -> PathBuf {
    let first = {
        let mut s = step();
        let f = !s.checked;
        s.checked = true;
        f
    };
    if first {
        let root = bible_dir().join("orig");
        let mark = root.join("format.txt");
        let ok = std::fs::read_to_string(&mark).map(|s| s.trim() == ORIG_FORMAT).unwrap_or(false);
        if !ok {
            let _ = std::fs::remove_dir_all(&root);
            let _ = std::fs::remove_file(bible_dir().join("step_index.json"));
            step().index = None;
            if std::fs::create_dir_all(&root).is_ok() {
                let _ = std::fs::write(&mark, ORIG_FORMAT);
            }
        }
    }
    bible_dir().join("orig").join(book(bi).0).join(format!("{ch}.json"))
}

static STEP_LOCK: Mutex<()> = Mutex::new(());

pub type Prog<'a> = Option<&'a mut dyn FnMut(&str, f64)>;

/// _step_get(path, dest): GitHub, then the mirror
fn step_get(path: &str, dest: &std::path::Path, mut progress: Prog, label: &str) -> Result<PathBuf, String> {
    let mut err = None;
    for base in std::iter::once(src("gh_raw")).chain(MIRRORS.iter().map(|s| s.to_string())) {
        let p: Prog = match progress.as_mut() {
            Some(f) => Some(&mut **f),
            None => None,
        };
        match download(&format!("{base}/{}", quote(path, "/")), dest, "", 100000, p, label) {
            Ok(d) => return Ok(d),
            Err(e) => err = Some(e),
        }
    }
    Err(err.unwrap_or_else(|| format!("I couldn't download {label}.")))
}

/// step_download_file(path): one section split into per-chapter files; the (book, chapter)s written
pub fn step_download_file(path: &str, progress: Prog) -> Result<BTreeSet<(usize, i64)>, String> {
    let _g = STEP_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let _ = std::fs::create_dir_all(bible_dir().join("orig"));
    let tag: String = rx!(r"[^A-Za-z0-9]+").replace_all(base_name(path), "_").chars().take(40).collect();
    let tmp = bible_dir().join("orig").join(format!("section_{tag}.txt"));
    step_get(path, &tmp, progress, "the original-language text")?;
    let raw = std::fs::read(&tmp).map_err(|e| e.to_string())?;
    let text = String::from_utf8_lossy(&raw);
    let mut cur: Option<(usize, i64)> = None;
    let mut buf = Map::new();
    let mut wrote = BTreeSet::new();
    let flush = |cur: Option<(usize, i64)>, buf: &Map<String, Value>, wrote: &mut BTreeSet<(usize, i64)>| {
        if let Some(c) = cur {
            if !buf.is_empty() {
                let p = orig_path(c.0, c.1);
                if let Some(d) = p.parent() {
                    let _ = std::fs::create_dir_all(d);
                }
                let t = PathBuf::from(format!("{}.tmp", p.display()));
                if std::fs::write(&t, serde_json::to_string(&json!({"v": buf})).unwrap_or_default()).is_ok() {
                    let _ = std::fs::rename(&t, &p);
                }
                wrote.insert(c);
            }
        }
    };
    for line in text.split_inclusive('\n') {
        let Some((bk, c, v, w)) = step_parse_line(line) else { continue };
        if Some((bk, c)) != cur {
            flush(cur, &buf, &mut wrote);
            cur = Some((bk, c));
            buf = Map::new();
        }
        buf.entry(v.to_string()).or_insert_with(|| json!([])).as_array_mut().unwrap().push(w);
    }
    flush(cur, &buf, &mut wrote);
    let _ = std::fs::remove_file(&tmp);
    if wrote.is_empty() {
        return Err("The original-language file came back empty (the network may have blocked it).".into());
    }
    Ok(wrote)
}

/// step_download_section(bi, progress, want)
pub fn step_download_section(bi: usize, mut progress: Prog, want: Option<i64>) -> Result<bool, String> {
    let paths = step_files_for(bi);
    if paths.is_empty() {
        return Err(format!("I couldn't find the original-language file for {}.", book(bi).1));
    }
    let mut err = None;
    let have = |w: Option<i64>| w.is_some_and(|w| orig_path(bi, w).exists());
    for path in paths {
        if have(want) {
            return Ok(true);
        }
        let p: Prog = match progress.as_mut() {
            Some(f) => Some(&mut **f),
            None => None,
        };
        match step_download_file(&path, p) {
            Err(e) => {
                err = Some(e);
                continue;
            }
            Ok(wrote) => {
                if wrote.iter().any(|&(b, _)| b == bi) && (want.is_none() || have(want)) {
                    return Ok(true);
                }
            }
        }
    }
    if have(want) {
        return Ok(true);
    }
    if let Some(e) = err {
        return Err(e);
    }
    Err(format!("That download didn't include {} {}. Try the 'Interlinear' button for this one.", book(bi).1,
                want.map(|w| w.to_string()).unwrap_or_default()))
}

/// step_have_all(): (chapters on disk, chapters in the Bible)
pub fn step_have_all() -> (i64, i64) {
    let total: i64 = (0..n_books()).map(|i| book(i).2).sum();
    let mut have = 0;
    for i in 0..n_books() {
        let d = orig_path(i, 1).parent().unwrap().to_path_buf();
        if let Ok(rd) = std::fs::read_dir(d) {
            have += rd.flatten().filter(|e| e.file_name().to_string_lossy().ends_with(".json")).count() as i64;
        }
    }
    (have.min(total), total)
}

/// step_download_all(progress): ((have, total), errors)
pub fn step_download_all(mut progress: Prog) -> ((i64, i64), Vec<String>) {
    let mut done: Vec<String> = vec![];
    let mut errs = vec![];
    for bi in 0..n_books() {
        if orig_path(bi, 1).exists() && orig_path(bi, book(bi).2).exists() {
            continue;
        }
        let Some(path) = step_file_for(bi) else { continue };
        if done.contains(&path) {
            continue;
        }
        done.push(path.clone());
        let p: Prog = match progress.as_mut() {
            Some(f) => Some(&mut **f),
            None => None,
        };
        if let Err(e) = step_download_file(&path, p) {
            errs.push(e);
        }
    }
    for lang in ["Hebrew", "Greek"] {
        let _ = step_lexicon(lang);
    }
    (step_have_all(), errs)
}

/// bible_original(bi, ch, vs, allow_net, progress): the verse's original words
pub fn bible_original(bi: usize, ch: i64, vs: i64, allow_net: bool, progress: Prog) -> Result<Vec<Value>, String> {
    let p = orig_path(bi, ch);
    if !p.exists() {
        if !allow_net {
            return Err("The original text for this book isn't on this PC yet.".into());
        }
        step_download_section(bi, progress, Some(ch))?;
    }
    match std::fs::read_to_string(&p).ok().and_then(|s| serde_json::from_str::<Value>(&s).ok()) {
        Some(d) => Ok(d["v"][vs.to_string()].as_array().cloned().unwrap_or_default()),
        None => {
            let _ = std::fs::remove_file(&p);
            Err("The saved original text for this chapter was damaged, so I removed it. Click the verse again to re-download it.".into())
        }
    }
}

/// step_lexicon(lang): a downloaded TBESH/TBESG.json of 200000 bytes or more, else the bundled one
pub fn step_lexicon(lang: &str) -> Arc<HashMap<String, Value>> {
    let key: &'static str = if lang == "Greek" { "TBESG" } else { "TBESH" };
    if let Some(l) = step().lex.get(key) {
        return l.clone();
    }
    let p = bible_dir().join(format!("{key}.json"));
    let big = std::fs::metadata(&p).map(|m| m.len() >= 200000).unwrap_or(false);
    let lex = if big {
        match std::fs::read_to_string(&p).ok().and_then(|s| serde_json::from_str::<HashMap<String, Value>>(&s).ok()) {
            Some(l) => l,
            None => crate::bible::step_lexicon(lang).clone(),
        }
    } else {
        crate::bible::step_lexicon(lang).clone()
    };
    let l = Arc::new(lex);
    step().lex.insert(key, l.clone());
    l
}
