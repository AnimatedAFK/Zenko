//! The study books' practice content (zenko.py chem_problem, math_problem, psy_prompt, the research-methods quiz,
//! english_check, num_answer, close_enough, _sf, fmt_num, bio_norm, fr_norm, fr_match) and its data (BIO_FACTS,
//! FR_VOCAB, PSY_*, ENGLISH_TIPS, SUBJECT_TOPICS ... in data/books.json). Random choices go through PyRandom the way
//! Python's random module makes them, so a seeded run gives the same problems.

use serde_json::Value;
use std::sync::OnceLock;
use zenko_sound::mt::PyRandom;

pub fn data() -> &'static Value {
    static D: OnceLock<Value> = OnceLock::new();
    D.get_or_init(|| serde_json::from_str(include_str!("../data/books.json")).unwrap())
}

/// random.randrange(n)
pub fn randrange(r: &mut PyRandom, n: usize) -> usize {
    r.randbelow(n as u64) as usize
}

/// random.shuffle(x)
pub fn shuffle<T>(r: &mut PyRandom, x: &mut [T]) {
    for i in (1..x.len()).rev() {
        let j = r.randbelow(i as u64 + 1) as usize;
        x.swap(i, j);
    }
}

/// random.sample(population, k)
pub fn sample<T: Clone>(r: &mut PyRandom, pop: &[T], k: usize) -> Vec<T> {
    let n = pop.len();
    let mut setsize = 21usize;
    if k > 5 {
        setsize += 4usize.pow(((3 * k) as f64).log(4.0).ceil() as u32);
    }
    let mut out = Vec::with_capacity(k);
    if n <= setsize {
        let mut pool: Vec<T> = pop.to_vec();
        for i in 0..k {
            let j = r.randbelow((n - i) as u64) as usize;
            out.push(pool[j].clone());
            pool[j] = pool[n - i - 1].clone();
        }
    } else {
        let mut sel: Vec<usize> = vec![];
        for _ in 0..k {
            let mut j = r.randbelow(n as u64) as usize;
            while sel.contains(&j) {
                j = r.randbelow(n as u64) as usize;
            }
            sel.push(j);
            out.push(pop[j].clone());
        }
    }
    out
}

/// repr(float)
pub fn py_repr(v: f64) -> String {
    let s = crate::calc::py_repr_grouped(v, true);
    s.replace(',', "")
}

/// "%.<n>g"
pub fn pyg_n(x: f64, p: usize) -> String {
    if x == 0.0 {
        return "0".into();
    }
    if !x.is_finite() {
        return py_repr(x);
    }
    let e = format!("{:.*e}", p.max(1) - 1, x);
    let (m, ex) = e.split_once('e').unwrap();
    let ex: i32 = ex.parse().unwrap();
    if ex < -4 || ex >= p as i32 {
        let m = if m.contains('.') { m.trim_end_matches('0').trim_end_matches('.') } else { m };
        return format!("{m}e{}{:02}", if ex < 0 { '-' } else { '+' }, ex.abs());
    }
    let prec = (p as i32 - 1 - ex).max(0) as usize;
    let s = format!("{x:.prec$}");
    if s.contains('.') { s.trim_end_matches('0').trim_end_matches('.').to_string() } else { s }
}

/// _sf(x, n): "%.ng" (0 stays "0")
pub fn sf(x: f64, n: usize) -> String {
    if x == 0.0 { "0".into() } else { pyg_n(x, n) }
}

/// fmt_num of a float
pub fn fmt_f(v: f64) -> String {
    crate::calc::fmt_num(&crate::calc::Num::Float(v))
}

/// round(x, n) for a float
pub fn py_round(x: f64, n: usize) -> f64 {
    format!("{x:.n$}").parse().unwrap_or(x)
}

/// a number as Python's "%s" writes it: an int as an int, a float as repr
#[derive(Clone, Copy, Debug)]
pub enum N {
    I(i64),
    F(f64),
}

impl N {
    pub fn f(self) -> f64 {
        match self {
            N::I(i) => i as f64,
            N::F(f) => f,
        }
    }
    pub fn s(self) -> String {
        match self {
            N::I(i) => i.to_string(),
            N::F(f) => py_repr(f),
        }
    }
}

/// a practice problem: question, answer, unit, worked solution, guide topic and (maths) paper style
#[derive(Clone, Debug, PartialEq)]
pub struct Problem {
    pub q: String,
    pub a: f64,
    pub unit: Option<String>,
    pub sol: String,
    pub topic: String,
    pub p: Option<i64>,
}

fn chem_m() -> Vec<(String, f64)> {
    data()["CHEM_M"].as_array().unwrap().iter().map(|v| (v[0].as_str().unwrap().into(), v[1].as_f64().unwrap())).collect()
}

fn pick<T: Copy>(r: &mut PyRandom, v: &[T]) -> T {
    *r.choice(v)
}

/// chem_problem(rng)
pub fn chem_problem(r: &mut PyRandom) -> Problem {
    let k = randrange(r, 8);
    let pr = |q: String, a: f64, unit: &str, sol: String, topic: &str| Problem { q, a, unit: Some(unit.into()), sol,
                                                                               topic: topic.into(), p: None };
    let rp = py_repr;
    match k {
        0 => {
            let cm = chem_m();
            let (f, mm) = r.choice(&cm).clone();
            let m = pick(r, &[2.5, 5.0, 10.0, 12.6, 25.0, 4.4, 8.0, 36.0]);
            let n = m / mm;
            pr(format!("How many moles are in {} g of {f}? (M = {} g mol\u{207b}\u{b9})", rp(m), rp(mm)), n, "mol",
               format!("n = m / M = {} / {} = {} mol", rp(m), rp(mm), sf(n, 3)), "Structure 1.4 The mole")
        }
        1 => {
            let n = pick(r, &[0.05, 0.1, 0.125, 0.2, 0.25, 0.5]);
            let v = pick(r, &[100i64, 200, 250, 500, 1000]);
            let vd = v as f64 / 1000.0;
            let c = n / vd;
            pr(format!("{} mol of solute is dissolved to make {v} cm\u{b3} of solution. Concentration in mol dm\u{207b}\u{b3}?", rp(n)),
               c, "mol dm\u{207b}\u{b3}",
               format!("V = {v}/1000 = {} dm\u{b3}; c = n/V = {} / {} = {} mol dm\u{207b}\u{b3}", sf(vd, 3), rp(n), sf(vd, 3), sf(c, 3)),
               "Structure 1.4 Concentration")
        }
        2 => {
            let c1 = pick(r, &[0.5, 1.0, 2.0]);
            let v1 = pick(r, &[10i64, 20, 25, 50]);
            let v2 = pick(r, &[100i64, 250, 500]);
            let c2 = c1 * v1 as f64 / v2 as f64;
            pr(format!("{v1} cm\u{b3} of {} mol dm\u{207b}\u{b3} HCl is diluted to {v2} cm\u{b3}. New concentration?", rp(c1)), c2,
               "mol dm\u{207b}\u{b3}",
               format!("c\u{2081}V\u{2081} = c\u{2082}V\u{2082} \u{2192} c\u{2082} = {} \u{d7} {v1} / {v2} = {} mol dm\u{207b}\u{b3}", rp(c1), sf(c2, 3)),
               "Structure 1.4 Dilution")
        }
        3 => {
            let n = pick(r, &[0.1, 0.25, 0.5, 1.0, 2.0]);
            let t = pick(r, &[273i64, 298, 300, 350]);
            let p = pick(r, &[N::I(100), N::F(101.3), N::I(150), N::I(200)]);
            let v = n * 8.31 * t as f64 / p.f();
            pr(format!("Volume (dm\u{b3}) of {} mol of ideal gas at {t} K and {} kPa? (R = 8.31 J K\u{207b}\u{b9} mol\u{207b}\u{b9})",
                       rp(n), p.s()), v, "dm\u{b3}",
               format!("pV = nRT \u{2192} V = nRT/p = {} \u{d7} 8.31 \u{d7} {t} / {} = {} dm\u{b3} (kPa with dm\u{b3} keeps the units matched)",
                       rp(n), p.s(), sf(v, 3)), "Structure 1.5 Ideal gases")
        }
        4 => {
            let e = pick(r, &[1i64, 2, 3, 4, 5]);
            let a = pick(r, &[1.0, 2.0, 5.0]);
            let h = a * 10f64.powf(-(e as f64));
            let ph = -h.log10();
            pr(format!("pH of a strong acid solution with [H\u{207a}] = {} \u{d7} 10\u{207b}{e} mol dm\u{207b}\u{b3}?", rp(a)), ph, "",
               format!("pH = \u{2212}log\u{2081}\u{2080}[H\u{207a}] = \u{2212}log\u{2081}\u{2080}({}) = {}", sf(h, 3), sf(ph, 3)),
               "Reactivity 3.1 pH")
        }
        5 => {
            let m = pick(r, &[50i64, 100, 150, 200]);
            let dt = pick(r, &[4.5, 6.0, 8.2, 10.0, 12.5]);
            let q = m as f64 * 4.18 * dt;
            pr(format!("{m} g of water warms by {} \u{b0}C. Heat absorbed in J? (c = 4.18 J g\u{207b}\u{b9} K\u{207b}\u{b9})", rp(dt)), q, "J",
               format!("Q = mc\u{394}T = {m} \u{d7} 4.18 \u{d7} {} = {} J", rp(dt), sf(q, 4)),
               "Reactivity 1.1 Measuring enthalpy changes")
        }
        6 => {
            let th = pick(r, &[5.0, 8.0, 10.0, 12.5, 20.0]);
            let act = py_round(th * r.uniform(0.55, 0.95), 2);
            let y = act / th * 100.0;
            pr(format!("Theoretical yield {} g, actual yield {} g. Percentage yield?", rp(th), rp(act)), y, "%",
               format!("% yield = actual / theoretical \u{d7} 100 = {} / {} \u{d7} 100 = {}%", rp(act), rp(th), sf(y, 3)),
               "Reactivity 2.1 Yield")
        }
        _ => {
            let cm = chem_m();
            let (f, mm) = r.choice(&cm).clone();
            let n = pick(r, &[0.02, 0.05, 0.15, 0.3, 1.5]);
            let m = n * mm;
            pr(format!("Mass in g of {} mol of {f}? (M = {} g mol\u{207b}\u{b9})", rp(n), rp(mm)), m, "g",
               format!("m = nM = {} \u{d7} {} = {} g", rp(n), rp(mm), sf(m, 3)), "Structure 1.4 The mole")
        }
    }
}

fn tr(n: i64, from: &str, to: &str) -> String {
    let f: Vec<char> = from.chars().collect();
    let t: Vec<char> = to.chars().collect();
    n.to_string().chars().map(|c| f.iter().position(|x| *x == c).map(|i| t[i]).unwrap_or(c)).collect()
}

/// _subs(n): subscript digits
pub fn subs(n: i64) -> String {
    tr(n, "0123456789-", "\u{2080}\u{2081}\u{2082}\u{2083}\u{2084}\u{2085}\u{2086}\u{2087}\u{2088}\u{2089}\u{208b}")
}

/// _sup(n): superscript digits (nothing for 1)
pub fn sup(n: i64) -> String {
    if n == 1 {
        return String::new();
    }
    tr(n, "0123456789-", "\u{2070}\u{b9}\u{b2}\u{b3}\u{2074}\u{2075}\u{2076}\u{2077}\u{2078}\u{2079}\u{207b}")
}

fn gcd(a: i64, b: i64) -> i64 {
    if b == 0 { a.abs() } else { gcd(b, a % b) }
}

fn frac(a: i64, b: i64) -> String {
    let g = gcd(a, b);
    let (a, b) = (a / g, b / g);
    if b == 1 { a.to_string() } else { format!("{a}/{b}") }
}

fn quad(a: i64, b: i64, c: i64) -> String {
    let mut s = match a {
        1 => "x\u{b2}".to_string(),
        -1 => "\u{2212}x\u{b2}".to_string(),
        _ => format!("{a}x\u{b2}"),
    };
    if b != 0 {
        s += &format!(" {} {}x", if b > 0 { "+" } else { "\u{2212}" }, if b.abs() == 1 { String::new() } else { b.abs().to_string() });
    }
    if c != 0 {
        s += &format!(" {} {}", if c > 0 { "+" } else { "\u{2212}" }, c.abs());
    }
    s + " = 0"
}

fn comb(n: i64, k: i64) -> i64 {
    let mut r = 1i64;
    for i in 0..k {
        r = r * (n - i) / (i + 1);
    }
    r
}

/// math_problem(rng)
pub fn math_problem(r: &mut PyRandom) -> Problem {
    let k = randrange(r, 9);
    let pr = |q: String, a: f64, sol: String, topic: &str, p: i64| Problem { q, a, unit: None, sol, topic: topic.into(),
                                                                            p: Some(p) };
    let sg = |b: i64| if b >= 0 { "+" } else { "\u{2212}" };
    match k {
        0 => {
            let u1 = r.randint(-5, 12);
            let d = pick(r, &[2i64, 3, 4, 5, -2, -3]);
            let n = r.randint(8, 30);
            let a = u1 + (n - 1) * d;
            pr(format!("Arithmetic sequence: u\u{2081} = {u1}, d = {d}. Find u{}.", subs(n)), a as f64,
               format!("u\u{2099} = u\u{2081} + (n\u{2212}1)d = {u1} + {}\u{d7}{d} = {a}", n - 1), "1 Number and algebra: sequences", 1)
        }
        1 => {
            let u1 = r.randint(1, 10);
            let d = r.randint(2, 6);
            let n = r.randint(10, 25);
            let s = (n * (2 * u1 + (n - 1) * d)) as f64 / 2.0;
            pr(format!("Find the sum of the first {n} terms of {u1}, {}, {}, ...", u1 + d, u1 + 2 * d), s,
               format!("S\u{2099} = n/2 (2u\u{2081} + (n\u{2212}1)d) = {n}/2 \u{d7} ({} + {}) = {}", 2 * u1, (n - 1) * d, fmt_f(s)),
               "1 Number and algebra: series", 1)
        }
        2 => {
            let u1 = pick(r, &[1i64, 2, 3, 5]);
            let rr = pick(r, &[N::I(2), N::I(3), N::F(0.5)]);
            let n = r.randint(5, 9);
            let s = match rr {
                N::I(ri) => (u1 * (ri.pow(n as u32) - 1)) as f64 / (ri - 1) as f64,
                N::F(rf) => u1 as f64 * (rf.powf(n as f64) - 1.0) / (rf - 1.0),
            };
            pr(format!("Geometric series: u\u{2081} = {u1}, r = {}. Find S{}.", rr.s(), subs(n)), s,
               format!("S\u{2099} = u\u{2081}(r\u{207f} \u{2212} 1)/(r \u{2212} 1) = {u1}({}^{n} \u{2212} 1)/({} \u{2212} 1) = {}", rr.s(),
                       rr.s(), fmt_f(py_round(s, 4))), "1 Number and algebra: geometric series", 2)
        }
        3 => {
            let a = r.randint(2, 6);
            let n = r.randint(2, 4);
            let b = r.randint(-6, 6);
            let x = r.randint(-3, 3);
            let v = a * n * x.pow((n - 1) as u32) + b;
            pr(format!("f(x) = {a}x{} {} {}x. Find f\u{2032}({x}).", sup(n), sg(b), b.abs()), v as f64,
               format!("f\u{2032}(x) = {}x{} {} {}, so f\u{2032}({x}) = {v}", a * n, if n > 2 { sup(n - 1) } else { String::new() }, sg(b),
                       b.abs()), "5 Calculus: differentiation", 1)
        }
        4 => {
            let a = r.randint(1, 4);
            let b = r.randint(-5, 5);
            let (lo, hi) = (0i64, r.randint(1, 3));
            let v = (a * (hi.pow(3) - lo.pow(3))) as f64 / 3.0 + (b * (hi - lo)) as f64;
            pr(format!("Evaluate \u{222b} from {lo} to {hi} of ({a}x\u{b2} {} {}) dx.", sg(b), b.abs()), v,
               format!("= [{} x\u{b3} {} {}x] from {lo} to {hi} = {}", frac(a, 3), sg(b), b.abs(), fmt_f(py_round(v, 4))),
               "5 Calculus: definite integrals", 1)
        }
        5 => {
            let a = pick(r, &[1i64, 2, 3, -1]);
            let b = r.randint(-8, 8);
            let c = r.randint(-6, 9);
            let dd = b * b - 4 * a * c;
            let n = if dd > 0 { 2 } else if dd == 0 { 1 } else { 0 };
            pr(format!("How many real roots does {} have? (answer 0, 1 or 2)", quad(a, b, c)), n as f64,
               format!("\u{394} = b\u{b2} \u{2212} 4ac = {} \u{2212} 4({a})({c}) = {dd} \u{2192} {}", b * b,
                       ["no real roots", "one repeated root", "two real roots"][n]), "2 Functions: quadratics", 1)
        }
        6 => {
            let b = pick(r, &[2i64, 3, 5, 10]);
            let kk = r.randint(2, 4);
            let x = b.pow(kk as u32);
            pr(format!("Solve log{}(x) = {kk}.", subs(b)), x as f64, format!("x = {b}^{kk} = {x}"),
               "1 Number and algebra: logarithms", 1)
        }
        7 => {
            let n = r.randint(5, 9);
            let rr = r.randint(2, 4);
            let a = pick(r, &[1i64, 2, 3]);
            let c = comb(n, rr);
            let v = c * a.pow((n - rr) as u32);
            pr(format!("Find the coefficient of x{} in the expansion of ({a} + x){}.", sup(rr), sup(n)), v as f64,
               format!("\u{207f}C\u{1d63} a^(n\u{2212}r) = C({n},{rr}) \u{d7} {a}^{} = {c} \u{d7} {} = {v}", n - rr, a.pow((n - rr) as u32)),
               "1 Number and algebra: binomial theorem", 1)
        }
        _ => {
            let a = r.randint(4, 12);
            let b = r.randint(4, 12);
            let cc = pick(r, &[30i64, 45, 60, 75, 110, 120]);
            let c = ((a * a + b * b) as f64 - (2 * a * b) as f64 * (cc as f64).to_radians().cos()).sqrt();
            pr(format!("Triangle: a = {a} cm, b = {b} cm, angle C = {cc}\u{b0}. Find c (3 s.f.)."), c,
               format!("Cosine rule: c\u{b2} = a\u{b2} + b\u{b2} \u{2212} 2ab cos C = {} + {} \u{2212} {} cos {cc}\u{b0} \u{2192} c = {} cm",
                       a * a, b * b, 2 * a * b, sf(c, 3)), "3 Geometry and trigonometry: cosine rule", 2)
        }
    }
}

/// psy_prompt(rng): concept, context, topic, question, hint
#[derive(Clone, Debug, PartialEq)]
pub struct PsyPrompt {
    pub concept: String,
    pub context: String,
    pub topic: String,
    pub q: String,
    pub hint: String,
}

pub fn psy_prompt(r: &mut PyRandom) -> PsyPrompt {
    let d = data();
    let concepts: Vec<(String, String)> = d["PSY_CONCEPTS"].as_object().unwrap().iter()
        .map(|(k, v)| (k.clone(), v.as_str().unwrap().to_string())).collect();
    let strs = |v: &Value| -> Vec<String> { v.as_array().unwrap().iter().map(|x| x.as_str().unwrap().to_string()).collect() };
    let (c, hint) = r.choice(&concepts).clone();
    let ctxs = strs(&d["PSY_CONTEXTS"]);
    let ctx = r.choice(&ctxs).clone();
    let tops = strs(&d["PSY_TOPICS"][&ctx]);
    let top = r.choice(&tops).clone();
    let aps = strs(&d["PSY_APPROACHES"]);
    let ap = r.choice(&aps).clone();
    let i = randrange(r, 4);
    let cl = c.to_lowercase();
    let q = match i {
        0 => format!("To what extent does {cl} affect what we can conclude about {top}? (concept: {c})"),
        1 => format!("Discuss {cl} in research on {top} from the {ap} approach."),
        2 => format!("Evaluate one study of {top} with reference to {cl}."),
        _ => format!("How has the concept of {cl} shaped how psychologists study {top}?"),
    };
    PsyPrompt { concept: c, context: ctx, topic: top, q, hint }
}

pub fn psy_terms() -> Vec<(String, String)> {
    data()["PSY_TERMS"].as_array().unwrap().iter().map(|v| (v[0].as_str().unwrap().into(), v[1].as_str().unwrap().into())).collect()
}

/// practice_psych's mnew: (term, definition, the four choices in order)
pub fn psy_term_quiz(r: &mut PyRandom) -> (String, String, Vec<String>) {
    let terms = psy_terms();
    let (term, dfn) = r.choice(&terms).clone();
    let others: Vec<String> = terms.iter().filter(|t| t.0 != term).map(|t| t.0.clone()).collect();
    let mut ch = sample(r, &others, 3);
    ch.push(term.clone());
    shuffle(r, &mut ch);
    (term, dfn, ch)
}

/// num_answer(text): a typed number (3.5, -2, 1/3, 2.5e-3, 3 x 10^-4, 45%)
pub fn num_answer(text: &str) -> Option<f64> {
    static R1: OnceLock<regex::Regex> = OnceLock::new();
    static R2: OnceLock<regex::Regex> = OnceLock::new();
    let t = text.trim().to_lowercase().replace('\u{2212}', "-").replace(',', ".").replace('%', "").replace(' ', "");
    let t = R1.get_or_init(|| regex::Regex::new(r"(?:x|\*|\u{d7})10\^?(-?\d+)").unwrap()).replace_all(&t, "e$1").to_string();
    if let Some(m) = R2.get_or_init(|| regex::Regex::new(r"^(-?\d+)/(\d+)$").unwrap()).captures(&t) {
        let a: f64 = m[1].parse().ok()?;
        let b: f64 = m[2].parse().ok()?;
        return if b == 0.0 { None } else { Some(a / b) };
    }
    if let Some(v) = py_float_parse(&t) {
        return Some(v);
    }
    match crate::calc::calc_eval(&t) {
        Ok(crate::calc::Num::Float(f)) => Some(f),
        Ok(crate::calc::Num::Int(i)) => i.to_string().parse().ok(),
        _ => None,
    }
}

/// float(t) as Python reads it
fn py_float_parse(t: &str) -> Option<f64> {
    let s = t.trim().replace('_', "");
    let low = s.to_lowercase();
    match low.trim_start_matches(['+', '-']) {
        "inf" | "infinity" | "nan" => return s.parse::<f64>().ok().or(Some(if low.starts_with('-') { f64::NEG_INFINITY } else if low.contains("nan") { f64::NAN } else { f64::INFINITY })),
        _ => {}
    }
    if s.is_empty() || !s.chars().all(|c| c.is_ascii_digit() || "+-.eE".contains(c)) {
        return None;
    }
    s.parse::<f64>().ok()
}

/// close_enough(got, want)
pub fn close_enough(got: Option<f64>, want: f64) -> bool {
    let Some(g) = got else { return false };
    if want == 0.0 {
        return g.abs() < 1e-6;
    }
    (g - want).abs() <= (0.012 * want.abs()).max(0.006)
}

fn strip_marks(s: &str) -> String {
    use unicode_normalization::UnicodeNormalization;
    s.nfd().filter(|c| !unicode_normalization::char::is_combining_mark(*c)).collect()
}

/// fr_norm(s)
pub fn fr_norm(s: &str) -> String {
    static R1: OnceLock<regex::Regex> = OnceLock::new();
    static R2: OnceLock<regex::Regex> = OnceLock::new();
    let s = strip_marks(&s.to_lowercase().trim().to_string()).replace('\u{153}', "oe");
    let s = R1.get_or_init(|| regex::Regex::new(r"^(le|la|les|l'|un|une|des|to)\s*").unwrap()).replace(&s, "").to_string();
    R2.get_or_init(|| regex::Regex::new(r"[^a-z' ]+").unwrap()).replace_all(&s, " ").trim().to_string()
}

/// bio_norm(s)
pub fn bio_norm(s: &str) -> String {
    strip_marks(&s.to_lowercase()).chars().filter(|c| c.is_ascii_lowercase() || c.is_ascii_digit()).collect()
}

/// fr_match(guess, answer): (right?, accents perfect?)
pub fn fr_match(guess: &str, answer: &str) -> (bool, bool) {
    static R1: OnceLock<regex::Regex> = OnceLock::new();
    static R2: OnceLock<regex::Regex> = OnceLock::new();
    let g = guess.trim().to_lowercase();
    if g.is_empty() {
        return (false, false);
    }
    let noparen = R1.get_or_init(|| regex::Regex::new(r"\(.*?\)").unwrap()).replace_all(answer, "").to_string();
    let mut alts: Vec<String> = noparen.split(['/', ',']).map(|a| a.trim().to_string()).collect();
    alts.push(answer.to_string());
    let art = R2.get_or_init(|| regex::Regex::new(r"^(le|la|les|l')\s*").unwrap());
    for a in alts {
        if a.is_empty() {
            continue;
        }
        let al = a.to_lowercase();
        if g == al || g == art.replace(&al, "") {
            return (true, true);
        }
        let fg = fr_norm(&g);
        if !fg.is_empty() && fg == fr_norm(&a) {
            return (true, false);
        }
    }
    (false, false)
}

/// english_check(text): quick feedback on an analysis paragraph
pub fn english_check(text: &str) -> String {
    static WS: OnceLock<regex::Regex> = OnceLock::new();
    static WD: OnceLock<regex::Regex> = OnceLock::new();
    static QT: OnceLock<regex::Regex> = OnceLock::new();
    static ITH: OnceLock<regex::Regex> = OnceLock::new();
    static SH: OnceLock<regex::Regex> = OnceLock::new();
    static SENT: OnceLock<fancy_regex::Regex> = OnceLock::new();
    let d = data();
    let list = |k: &str| -> Vec<String> { d[k].as_array().unwrap().iter().map(|x| x.as_str().unwrap().to_string()).collect() };
    let t = format!(" {} ", WS.get_or_init(|| regex::Regex::new(r"\s+").unwrap()).replace_all(&text.to_lowercase(), " "));
    let words = WD.get_or_init(|| regex::Regex::new(r"[a-z']+").unwrap()).find_iter(&t).count();
    if words < 25 {
        return "Write at least a few sentences (one full paragraph) and I'll look at it~".into();
    }
    let mut out: Vec<String> = vec![];
    let mut good: Vec<String> = vec![];
    let quotes = QT.get_or_init(|| regex::Regex::new("[\"\u{201c}\u{201d}\u{2018}\u{2019}'][^\"\u{201c}\u{201d}]{2,80}[\"\u{201c}\u{201d}\u{2019}']").unwrap())
        .find_iter(text).count();
    let mut techs: Vec<String> = list("EN_TECH").into_iter().filter(|k| t.contains(k.as_str())).collect();
    techs.sort();
    techs.dedup();
    let effs = list("EN_EFFECT").into_iter().filter(|k| t.contains(k.as_str())).count();
    let purp = list("EN_PURPOSE").into_iter().filter(|k| t.contains(k.as_str())).count();
    let retell: usize = list("EN_RETELL").iter().map(|k| t.matches(&format!(" {k} ")).count()).sum();
    if quotes > 0 {
        good.push(format!("evidence: {quotes} quote{}", if quotes == 1 { "" } else { "s" }));
    } else {
        out.push("No quotes found. Add 1-3 short embedded quotes as evidence.".into());
    }
    if !techs.is_empty() {
        good.push(format!("techniques named: {}", techs.iter().take(5).cloned().collect::<Vec<_>>().join(", ")));
    } else {
        out.push("Name the technique precisely (e.g. juxtaposition, direct address, imagery, tricolon).".into());
    }
    if effs >= 2 {
        good.push("you explain effects".into());
    } else {
        out.push("Explain the EFFECT: '...which conveys/evokes/emphasises ... for the reader'.".into());
    }
    if purp > 0 {
        good.push("linked to reader/purpose".into());
    } else {
        out.push("Link it to purpose or audience, or back to the guiding question.".into());
    }
    if retell >= 3 {
        out.push("It reads a bit like retelling ('then...', 'happens'). Analyse HOW, not WHAT happens.".into());
    }
    if ITH.get_or_init(|| regex::Regex::new(r"\bi think\b|\bin my opinion\b|\bi feel\b").unwrap()).is_match(&t) {
        out.push("Drop 'I think': argue it ('This suggests...').".into());
    }
    if SH.get_or_init(|| regex::Regex::new(r"\bshows\b").unwrap()).is_match(&t) && t.matches(" shows ").count() >= 2 {
        out.push("'shows' appears a lot. Try conveys / foregrounds / undermines.".into());
    }
    let sre = SENT.get_or_init(|| fancy_regex::Regex::new(r"(?<=[.!?])\s+").unwrap());
    let trimmed = text.trim();
    let mut sents = vec![];
    let mut last = 0;
    for m in sre.find_iter(trimmed).flatten() {
        sents.push(&trimmed[last..m.start()]);
        last = m.end();
    }
    sents.push(&trimmed[last..]);
    let sents: Vec<&str> = sents.into_iter().filter(|s| !s.is_empty()).collect();
    if let Some(s0) = sents.first()
        && s0.split_whitespace().count() < 6 {
        out.push("Start with a topic sentence that makes a claim.".into());
    }
    let score = 4 - out.len().min(4);
    let head = ["Hmm, needs work", "Getting there", "Good", "Strong", "Excellent"][score];
    let mut s = if good.is_empty() {
        format!("{head}!\n")
    } else {
        format!("{head}! {}.\n", capitalize(&good.join("; ")))
    };
    if !out.is_empty() {
        s += &format!("\nTo improve:\n{}", out.iter().map(|o| format!("\u{2022} {o}")).collect::<Vec<_>>().join("\n"));
    } else {
        s += "\nThis has the full technique \u{2192} effect \u{2192} purpose chain. Kon~ now check it answers the question.";
    }
    s
}

/// str.capitalize(): the first letter upper, the rest lower
pub fn capitalize(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        Some(f) => f.to_uppercase().collect::<String>() + &c.as_str().to_lowercase(),
        None => String::new(),
    }
}
