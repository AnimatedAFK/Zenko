//! zenko.py's periodic table data and maths: PT (from data/tools.json), pt_config, pt_full_config, pt_place and
//! molar_mass (brackets, hydrates and charges).

use regex::Regex;
use serde_json::Value;
use std::sync::OnceLock;

macro_rules! rx {
    ($p:expr) => {{
        static R: OnceLock<Regex> = OnceLock::new();
        R.get_or_init(|| Regex::new($p).unwrap())
    }};
}

pub fn tools() -> &'static Value {
    static D: OnceLock<Value> = OnceLock::new();
    D.get_or_init(|| serde_json::from_str(include_str!("../data/tools.json")).unwrap())
}

#[derive(Clone, Debug)]
pub struct Elem {
    pub z: usize,
    pub sym: String,
    pub name: String,
    pub mass: String,
    pub cat: String,
    pub en: Option<f64>,
}

pub fn pt() -> &'static Vec<Elem> {
    static D: OnceLock<Vec<Elem>> = OnceLock::new();
    D.get_or_init(|| {
        tools()["PT"].as_array().unwrap().iter().enumerate().map(|(i, e)| Elem {
            z: i + 1,
            sym: e[0].as_str().unwrap().into(),
            name: e[1].as_str().unwrap().into(),
            mass: e[2].as_str().unwrap().into(),
            cat: e[3].as_str().unwrap().into(),
            en: e[4].as_f64(),
        }).collect()
    })
}

pub fn by_sym(s: &str) -> Option<&'static Elem> {
    pt().iter().find(|e| e.sym == s)
}

/// PT_CATS: [(key, name, colour)]
pub fn cats() -> Vec<(String, String, String)> {
    tools()["PT_CATS"].as_array().unwrap().iter()
        .map(|c| (c[0].as_str().unwrap().into(), c[1].as_str().unwrap().into(), c[2].as_str().unwrap().into())).collect()
}

pub fn cat_of(k: &str) -> (String, String) {
    cats().into_iter().find(|c| c.0 == k).map(|c| (c.1, c.2)).unwrap_or_default()
}

const AUFBAU: [&str; 19] = ["1s", "2s", "2p", "3s", "3p", "4s", "3d", "4p", "5s", "4d", "5p", "6s", "4f", "5d", "6p", "7s", "5f", "6d", "7p"];
const NOBLE: [(usize, &str); 6] = [(86, "Rn"), (54, "Xe"), (36, "Kr"), (18, "Ar"), (10, "Ne"), (2, "He")];

pub fn pt_config(z: usize) -> String {
    if let Some(s) = tools()["PT_EXC"].get(z.to_string()).and_then(|v| v.as_str()) {
        return s.into();
    }
    let cap = |c: char| match c { 's' => 2, 'p' => 6, 'd' => 10, _ => 14 };
    let (mut fill, mut left): (Vec<(&str, usize)>, usize) = (vec![], z);
    for orb in AUFBAU {
        if left == 0 {
            break;
        }
        let n = cap(orb.chars().nth(1).unwrap()).min(left);
        fill.push((orb, n));
        left -= n;
    }
    let mut core = String::new();
    for (nz, sym) in NOBLE {
        if z > nz {
            core = format!("[{sym}]");
            let mut k = nz;
            let mut kept = vec![];
            for (orb, n) in fill {
                if k >= n {
                    k -= n;
                    continue;
                }
                kept.push((orb, n - k));
                k = 0;
            }
            fill = kept;
            break;
        }
    }
    fill.sort_by_key(|o| (o.0[..1].parse::<u32>().unwrap(), "spdf".find(o.0.chars().nth(1).unwrap()).unwrap()));
    let body: Vec<String> = fill.iter().map(|(o, n)| format!("{o}{n}")).collect();
    format!("{core} {}", body.join(" ")).trim().to_string()
}

pub fn pt_full_config(z: usize) -> String {
    let c = pt_config(z);
    let Some(m) = rx!(r"^\[(\w+)\]\s*(.*)").captures(&c) else { return c };
    let core = NOBLE.iter().find(|(_, s)| *s == &m[1]).unwrap().0;
    let rest = m[2].to_string();
    if core > 2 { format!("{} {rest}", pt_full_config(core)).trim().to_string() } else { format!("1s2 {rest}").trim().to_string() }
}

pub fn pt_place(z: usize) -> (usize, usize) {
    if (57..=71).contains(&z) {
        return (8, z - 57 + 3);
    }
    if (89..=103).contains(&z) {
        return (9, z - 89 + 3);
    }
    let starts = [(1, 1), (3, 2), (11, 3), (19, 4), (37, 5), (55, 6), (87, 7)];
    let (first, period) = *starts.iter().filter(|(s, _)| z >= *s).max_by_key(|(_, p)| *p).unwrap();
    let i = z - first;
    match period {
        1 => (1, if z == 1 { 1 } else { 18 }),
        2 | 3 => (period, if i < 2 { i + 1 } else { i + 11 }),
        4 | 5 => (period, i + 1),
        _ if i < 2 => (period, i + 1),
        _ => (period, if i >= 17 { i - 14 + 1 } else { 3 }),
    }
}

/// molar_mass(formula) -> (g/mol, [(element, count)] in first-seen order); Err is the ValueError message
pub fn molar_mass(formula: &str) -> Result<(f64, Vec<(String, i64)>), String> {
    let f = formula.trim().replace(' ', "");
    let f = if rx!(r"[+\-]$").is_match(&f) { rx!(r"[\^]?\d*[+\-]$").replace(&f, "").into_owned() } else { f };
    let mut total: Vec<(String, i64)> = vec![];
    fn add(m: &mut Vec<(String, i64)>, k: &str, v: i64) {
        match m.iter_mut().find(|e| e.0 == k) {
            Some(e) => e.1 += v,
            None => m.push((k.to_string(), v)),
        }
    }
    for part in rx!(r"[·.*•]").split(&f) {
        if part.is_empty() {
            continue;
        }
        let mut mult = 1i64;
        let mut part = part.to_string();
        if let Some(m) = rx!(r"^(\d+)(.*)$").captures(&part.clone()) {
            mult = m[1].parse().map_err(|_| "A number is too big.".to_string())?;
            part = m[2].to_string();
        }
        let mut stack: Vec<Vec<(String, i64)>> = vec![vec![]];
        let mut i = 0;
        while i < part.len() {
            let ch = part[i..].chars().next().unwrap();
            if "([{".contains(ch) {
                stack.push(vec![]);
                i += 1;
            } else if ")]}".contains(ch) {
                if stack.len() < 2 {
                    return Err("A bracket doesn't match.".into());
                }
                let grp = stack.pop().unwrap();
                i += 1;
                let m = rx!(r"^\d+").find(&part[i..]);
                let n: i64 = m.map(|m| m.as_str().parse().unwrap_or(1)).unwrap_or(1);
                i += m.map(|m| m.len()).unwrap_or(0);
                for (k, v) in grp {
                    add(stack.last_mut().unwrap(), &k, v * n);
                }
            } else {
                let Some(m) = rx!(r"^([A-Z][a-z]?)(\d*)").captures(&part[i..]) else {
                    return Err(format!("I can't read “{}” (element symbols start with a capital letter).", &part[i..]));
                };
                let el = &m[1];
                if by_sym(el).is_none() {
                    return Err(format!("“{el}” isn't an element symbol."));
                }
                let c: i64 = if m[2].is_empty() { 1 } else { m[2].parse().unwrap_or(1) };
                add(stack.last_mut().unwrap(), el, c);
                i += m[0].len();
            }
        }
        if stack.len() != 1 {
            return Err("A bracket is left open.".into());
        }
        for (k, v) in stack.pop().unwrap() {
            add(&mut total, &k, v * mult);
        }
    }
    if total.is_empty() {
        return Err("Type a formula like H2SO4.".into());
    }
    let mut mm = 0.0;
    for (k, v) in &total {
        mm += by_sym(k).unwrap().mass.trim_matches(['[', ']']).parse::<f64>().unwrap() * *v as f64;
    }
    Ok((mm, total))
}
