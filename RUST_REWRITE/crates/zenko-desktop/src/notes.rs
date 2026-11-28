//! The Notes page's filing (zenko.py zenko21's module helpers): nb_safe, nb_subjects, nb_line_scores, nb_split (free
//! notes followed line by line into the right subject), nb_folder_for, nb_match_subject, nb_import_plan / nb_import,
//! nb_file_text, and looks_french / fr_vocab_search that the French line scores use.

use regex::Regex;
use serde_json::Value;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

macro_rules! rx {
    ($p:expr) => {{
        static R: OnceLock<Regex> = OnceLock::new();
        R.get_or_init(|| Regex::new($p).unwrap())
    }};
}

pub const NB_UNSORTED: &str = "Unsorted";
pub const NB_IMPORTED: &str = "Imported";
pub const NB_TEXT: [&str; 4] = [".md", ".txt", ".markdown", ".text"];

pub fn nb_dir() -> PathBuf {
    crate::data_dir().join("notebooks")
}

pub fn nb_scratch() -> PathBuf {
    crate::data_dir().join("notes_scratch.txt")
}

/// one of SUBJECT_DEFS with life["subject_names"]: (sid, name, short, aliases, topic words)
#[derive(Clone, Debug)]
pub struct Def {
    pub sid: String,
    pub name: String,
    pub short: String,
    pub aliases: Vec<String>,
    pub topics: Vec<String>,
}

fn strs(v: &Value) -> Vec<String> {
    v.as_array().map(|a| a.iter().filter_map(|x| x.as_str().map(String::from)).collect()).unwrap_or_default()
}

/// subject_defs(life)
pub fn subject_defs(life: &Value) -> Vec<Def> {
    let names = &life["subject_names"];
    crate::watch::data().raw["SUBJECTS"].as_array().into_iter().flatten().map(|d| {
        let sid = d[0].as_str().unwrap_or("").to_string();
        let name = names.get(&sid).and_then(|v| v.as_str()).map(String::from).unwrap_or_else(|| d[1].as_str().unwrap_or("").into());
        Def { sid, name, short: d[2].as_str().unwrap_or("").into(), aliases: strs(&d[3]), topics: strs(&d[4]) }
    }).collect()
}

/// subject_name(sid, life, short)
pub fn subject_name(sid: &str, life: &Value, short: bool) -> String {
    subject_defs(life).into_iter().find(|d| d.sid == sid).map(|d| if short { d.short } else { d.name })
        .unwrap_or_else(|| if sid.is_empty() { "?".into() } else { sid.into() })
}

/// _has(low, word)
pub fn has(low: &str, word: &str) -> bool {
    if word.ends_with(' ') {
        return format!(" {low} ").contains(&format!(" {word}"));
    }
    crate::watch::has_word(low, word)
}

/// nb_safe(name): a name that can be a file or folder
pub fn nb_safe(name: &str) -> String {
    let n = rx!(r#"[<>:"/\\|?*\x00-\x1f]"#).replace_all(name.trim(), "-").to_string();
    let n = n.trim_matches(|c| c == '.' || c == ' ');
    let n: String = n.chars().take(80).collect();
    if n.is_empty() { "Untitled".into() } else { n }
}

/// nb_subjects(life): (sid, folder name, short name)
pub fn nb_subjects(life: &Value) -> Vec<(String, String, String)> {
    subject_defs(life).into_iter().map(|d| (d.sid, nb_safe(&d.name), d.short)).collect()
}

fn strip_acc(s: &str) -> String {
    use unicode_normalization::UnicodeNormalization;
    s.nfd().filter(|c| !unicode_normalization::char::is_combining_mark(*c)).collect()
}

/// fr_vocab_search(q, limit): the built-in vocab's matches, best first: (french, english, theme)
pub fn fr_vocab_search(q: &str, limit: usize) -> Vec<(String, String, String)> {
    let ql = q.trim().to_lowercase();
    if ql.is_empty() {
        return vec![];
    }
    let qa = strip_acc(&ql);
    let re_a = Regex::new(&format!(r"(?:^|[^a-z]){}", regex::escape(&qa))).unwrap();
    let re_e = Regex::new(&format!(r"(?:^|[^a-z]){}", regex::escape(&ql))).unwrap();
    let mut hits: Vec<(i32, String, String, String)> = vec![];
    for (theme, pairs) in crate::bookgen::data()["FR_VOCAB"].as_object().into_iter().flatten() {
        for p in pairs.as_array().into_iter().flatten() {
            let (fr, en) = (p[0].as_str().unwrap_or(""), p[1].as_str().unwrap_or(""));
            let (fa, ea) = (strip_acc(&fr.to_lowercase()), en.to_lowercase());
            let base = rx!(r"^(?:le |la |les |l'|un |une |se |s')").replace(&fa, "").to_string();
            let sc = if qa == fa || qa == base || ql == ea || ea.split(',').any(|w| w.trim() == qa) {
                3
            } else if re_a.is_match(&fa) || re_e.is_match(&ea) {
                2
            } else if qa.chars().count() > 3 && (fa.contains(&qa) || ea.contains(&ql)) {
                1
            } else {
                0
            };
            if sc > 0 {
                hits.push((sc, fr.into(), en.into(), theme.clone()));
            }
        }
    }
    hits.sort_by_key(|h| -h.0);
    hits.into_iter().take(limit).map(|h| (h.1, h.2, h.3)).collect()
}

/// looks_french(q)
pub fn looks_french(q: &str) -> bool {
    let t = q.to_lowercase();
    if rx!(r"[àâçéèêëîïôûùüÿœæ]").is_match(&t) {
        return true;
    }
    const FR: &str = "le la les un une des du de et est je tu il elle nous vous ils elles suis sont avec pour dans sur \
                      pas ne que qui mon ma mes ton ta tes son sa ses au aux ce cette être avoir faire aller bonjour merci \
                      oui non très bien";
    let words: Vec<&str> = rx!(r"[a-z']+").find_iter(&t).map(|m| m.as_str()).collect();
    if !words.is_empty() {
        let n = words.iter().filter(|w| FR.split(' ').any(|f| f == **w)).count();
        if n as f64 / words.len() as f64 >= 0.34 {
            return true;
        }
    }
    if words.len() == 1 {
        let w = words[0];
        if crate::routes17::routes_data()["FR_IRREG"].get(w).is_some() {
            return true;
        }
        let re = Regex::new(&format!(r"(?:^|[^a-zà-ÿ]){}(?:$|[^a-zà-ÿ])", regex::escape(w))).unwrap();
        for (fr, ..) in fr_vocab_search(w, 30) {
            if re.is_match(&fr.to_lowercase()) {
                return true;
            }
        }
    }
    false
}

/// nb_line_scores(line, life): each subject's score for one line, and whether its name / alias is in it
pub fn nb_line_scores(line: &str, life: &Value) -> (Vec<(String, i64)>, HashMap<String, bool>) {
    let low = line.to_lowercase();
    let mut sc: Vec<(String, i64)> = vec![];
    let mut alias = HashMap::new();
    for d in subject_defs(life) {
        let a = d.aliases.iter().any(|x| has(&low, x));
        alias.insert(d.sid.clone(), a);
        let t = d.topics.iter().filter(|w| w.trim().chars().count() > 1 && has(&low, w.trim())).count() as i64;
        sc.push((d.sid.clone(), if a { 4 } else { 0 } + t));
    }
    for (w, sid) in life["subj_words"].as_object().into_iter().flatten() {
        if let Some(sid) = sid.as_str()
            && let Some(e) = sc.iter_mut().find(|e| e.0 == sid)
            && w.chars().count() > 3 && has(&low, w) {
            e.1 += 2;
        }
    }
    if let Some(e) = sc.iter_mut().find(|e| e.0 == "french") {
        let words = rx!(r"[a-zà-ÿœæ']+").find_iter(&low).count();
        if rx!(r"[àâçéèêëîïôûùüÿœæ]").is_match(&low) || (words >= 3 && looks_french(line)) {
            e.1 += 3;
        }
    }
    if let Some(e) = sc.iter_mut().find(|e| e.0 == "math")
        && rx!(r"[=^√∫∑π≤≥]|\d\s*[x×*/+\-]\s*\d|\b(?:dy/dx|f\(x\)|sin|cos|tan|log|ln|lim)\b").is_match(&low) {
        e.1 += 2;
    }
    (sc, alias)
}

struct Info {
    sc: Vec<(String, i64)>,
    best: Option<String>,
    bs: i64,
    second: i64,
    header: bool,
}

/// nb_split(text, life): free notes -> [(subject id or None, text)], following the topic line by line
pub fn nb_split(text: &str, life: &Value) -> Vec<(Option<String>, String)> {
    let text = text.replace("\r\n", "\n");
    let lines: Vec<&str> = text.split('\n').collect();
    let mut info: Vec<Option<Info>> = vec![];
    for ln in &lines {
        if ln.trim().is_empty() {
            info.push(None);
            continue;
        }
        let (sc, alias) = nb_line_scores(ln, life);
        let mut order = sc.clone();
        order.sort_by_key(|e| -e.1);
        let (best, bs) = order.first().map(|e| (Some(e.0.clone()), e.1)).unwrap_or((None, 0));
        let second = order.get(1).map(|e| e.1).unwrap_or(0);
        let words = rx!(r"\w+").find_iter(ln).count();
        let header = best.as_ref().is_some_and(|b| alias.get(b).copied().unwrap_or(false)) && bs > second && words <= 7;
        info.push(Some(Info { sc, best: if bs > second { best } else { None }, bs, second, header }));
    }
    let ahead = |i: usize, sid: &str| -> bool {
        let mut seen = 0;
        for it in info.iter().skip(i + 1) {
            let Some(it) = it else { continue };
            seen += 1;
            match &it.best {
                Some(b) if b == sid => return true,
                Some(_) => return false,
                None => {}
            }
            if seen >= 2 {
                return false;
            }
        }
        false
    };
    let mut parts: Vec<(Option<String>, Vec<&str>)> = vec![];
    let mut cur: Option<String> = None;
    let mut pending: Vec<&str> = vec![];
    let mut prev_blank = true;
    for (i, ln) in lines.iter().enumerate() {
        let Some(it) = &info[i] else {
            match parts.last_mut() {
                Some(p) => p.1.push(ln),
                None => pending.push(ln),
            }
            prev_blank = true;
            continue;
        };
        let mut switch = None;
        if let Some(best) = &it.best
            && Some(best) != cur.as_ref() {
            let here = cur.as_ref().map(|c| it.sc.iter().find(|e| &e.0 == c).map(|e| e.1).unwrap_or(0)).unwrap_or(0);
            if it.header || cur.is_none()
                || (here == 0 && (it.bs >= 3 || (it.bs >= 2 && it.bs >= it.second + 2) || prev_blank || ahead(i, best))) {
                switch = Some(best.clone());
            }
        }
        if let Some(s) = switch {
            cur = Some(s);
            parts.push((cur.clone(), vec![]));
            if !pending.is_empty() {
                parts.last_mut().unwrap().1.append(&mut pending);
            }
        }
        match parts.last_mut() {
            Some(p) => p.1.push(ln),
            None => pending.push(ln),
        }
        prev_blank = false;
    }
    if !pending.is_empty() {
        match parts.first_mut() {
            Some(p) => {
                let mut v = pending.clone();
                v.append(&mut p.1);
                p.1 = v;
            }
            None => parts.push((None, pending.clone())),
        }
    }
    let mut merged: Vec<(Option<String>, String)> = vec![];
    for (sid, ls) in parts {
        let body = ls.join("\n");
        let body = body.trim_matches('\n').to_string();
        if body.trim().is_empty() {
            continue;
        }
        match merged.last_mut() {
            Some(m) if m.0 == sid => m.1 = format!("{}\n{body}", m.1),
            _ => merged.push((sid, body)),
        }
    }
    merged
}

/// nb_folder_for(sid, life)
pub fn nb_folder_for(sid: &str, life: &Value) -> String {
    nb_subjects(life).into_iter().find(|s| s.0 == sid).map(|s| s.1).unwrap_or_else(|| NB_UNSORTED.into())
}

/// nb_match_subject(name, life): 'Bio', 'IB Chemistry', 'French notes' -> the subject
pub fn nb_match_subject(name: &str, life: &Value) -> Option<String> {
    let low = rx!(r"[_\-.]+").replace_all(&name.to_lowercase(), " ").to_string();
    subject_defs(life).into_iter().find(|d| d.aliases.iter().any(|a| has(&low, a)) || has(&low, &d.name.to_lowercase())
        || has(&low, &d.short.to_lowercase())).map(|d| d.sid)
}

/// nb_ensure(life): the notebooks folder with one folder per subject
pub fn nb_ensure(life: &Value) {
    let d = nb_dir();
    let _ = std::fs::create_dir_all(&d);
    for (_, folder, _) in nb_subjects(life) {
        let _ = std::fs::create_dir_all(d.join(folder));
    }
}

/// os.walk(top) order: the folder's files, then each sub-folder in turn (names as listdir gives them)
fn walk(dir: &Path, rel: &[String], out: &mut Vec<(Vec<String>, String, PathBuf)>) {
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    let mut ents: Vec<_> = rd.flatten().collect();
    ents.sort_by_key(|e| e.file_name());
    let mut dirs = vec![];
    for e in &ents {
        let n = e.file_name().to_string_lossy().to_string();
        let p = e.path();
        if p.is_dir() {
            if !n.starts_with('.') {
                dirs.push((n, p));
            }
        } else {
            out.push((rel.to_vec(), n, p));
        }
    }
    for (n, p) in dirs {
        let mut r = rel.to_vec();
        r.push(n);
        walk(&p, &r, out);
    }
}

/// nb_import_plan(src, life): where each file of a folder goes (source, destination)
pub fn nb_import_plan(src: &Path, life: &Value) -> Vec<(PathBuf, PathBuf)> {
    let base = src.file_name().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
    let root_name = nb_safe(if base.is_empty() { "Imported folder" } else { &base });
    let root_sid = nb_match_subject(&root_name, life);
    let mut files = vec![];
    walk(src, &[], &mut files);
    let mut plan = vec![];
    for (rel, fname, sp) in files {
        if fname.starts_with('.') || matches!(fname.to_lowercase().as_str(), "thumbs.db" | "desktop.ini") {
            continue;
        }
        match std::fs::metadata(&sp) {
            Ok(m) if m.len() <= 300 * 1024 * 1024 => {}
            _ => continue,
        }
        let parts: Vec<String> = rel.iter().map(|p| nb_safe(p)).collect();
        let dest: Vec<String> = if let Some(r) = &root_sid {
            std::iter::once(nb_folder_for(r, life)).chain(parts.iter().cloned()).collect()
        } else {
            let stem = Path::new(&fname).file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
            let top = if parts.is_empty() { nb_match_subject(&stem, life) } else { nb_match_subject(&parts[0], life) };
            match top {
                Some(t) => std::iter::once(nb_folder_for(&t, life)).chain(parts.iter().skip(1).cloned()).collect(),
                None => [NB_IMPORTED.to_string(), root_name.clone()].into_iter().chain(parts.iter().cloned()).collect(),
            }
        };
        let mut dp = nb_dir();
        for d in dest {
            dp.push(d);
        }
        dp.push(nb_safe(&fname));
        plan.push((sp, dp));
    }
    plan
}

/// nb_import(src, life, only_new): copy a folder in; (copied, skipped, {top folder: n})
pub fn nb_import(src: &Path, life: &Value) -> (usize, usize, Vec<(String, usize)>) {
    let (mut copied, mut skipped) = (0, 0);
    let mut where_: Vec<(String, usize)> = vec![];
    for (sp, dp) in nb_import_plan(src, life) {
        if let (Ok(a), Ok(b)) = (std::fs::metadata(&sp), std::fs::metadata(&dp)) {
            let mt = |m: &std::fs::Metadata| m.modified().ok().and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_secs_f64()).unwrap_or(0.0);
            if a.len() == b.len() && mt(&a) <= mt(&b) + 1.0 {
                skipped += 1;
                continue;
            }
        }
        let ok = dp.parent().is_some_and(|p| std::fs::create_dir_all(p).is_ok()) && std::fs::copy(&sp, &dp).is_ok();
        if ok {
            copied += 1;
            let top = dp.strip_prefix(nb_dir()).ok().and_then(|r| r.iter().next()).map(|s| s.to_string_lossy().to_string())
                .unwrap_or_default();
            match where_.iter_mut().find(|w| w.0 == top) {
                Some(w) => w.1 += 1,
                None => where_.push((top, 1)),
            }
        } else {
            skipped += 1;
        }
    }
    (copied, skipped, where_)
}

/// nb_file_text(path): a text file's words (PDFs aren't read yet)
pub fn nb_file_text(path: &Path, limit: usize) -> String {
    let ext = path.extension().map(|e| format!(".{}", e.to_string_lossy().to_lowercase())).unwrap_or_default();
    if NB_TEXT.contains(&ext.as_str()) {
        return std::fs::read(path).map(|b| String::from_utf8_lossy(&b).chars().take(limit).collect()).unwrap_or_default();
    }
    String::new()
}
