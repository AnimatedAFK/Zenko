//! The tool windows' maths against zenko.py (golden/tools.json from tools/make_golden_tools.py): electron
//! configurations and table places for all 118 elements, molar masses, the CG50's calc_prepare / cg_eval (degrees and
//! radians, memories, x) / calc_fmt (exact and decimal) / polynomial roots / linear systems / SolveN / statistics /
//! regression / normal and binomial distributions, the French vocab search, Bible references, HTML stripping,
//! STEPBible rows, grammar hints, lexicon lookups and the calculator's key tokens.
use serde_json::{Value, json};
use std::collections::BTreeMap;
use zenko_desktop::bible::*;
use zenko_desktop::cgmath::*;
use zenko_desktop::ptable::*;
use zenko_desktop::{notes, routes17};

fn g() -> Value {
    serde_json::from_str(&std::fs::read_to_string("../../golden/tools.json").unwrap()).unwrap()
}

/// the same shape as the golden script's tryit()
fn tr<T>(r: Result<T, CgErr>, f: impl Fn(T) -> Value) -> Value {
    match r {
        Ok(v) => json!({"ok": f(v)}),
        Err(CgErr::Calc(m)) => json!({"err": "CalcError", "msg": m}),
        Err(CgErr::Py(n)) => json!({"err": n}),
    }
}

fn fl(v: f64) -> Value {
    if v.is_nan() { json!("nan") } else { json!(v) }
}

/// equal, with numbers compared as floats (Python's 15 and 15.0 are the same number on screen) and a Python
/// exception compared by its type only
fn same(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Number(x), Value::Number(y)) => x.as_f64() == y.as_f64(),
        (Value::Array(x), Value::Array(y)) => x.len() == y.len() && x.iter().zip(y).all(|(p, q)| same(p, q)),
        (Value::Object(x), Value::Object(y)) => {
            if let (Some(e), Some(f)) = (x.get("err"), y.get("err")) {
                return e == f && (e != "CalcError" || x.get("msg") == y.get("msg"));
            }
            x.len() == y.len() && x.iter().all(|(k, v)| y.get(k).is_some_and(|w| same(v, w)))
        }
        _ => a == b,
    }
}

fn check(name: &str, cases: &[(String, Value, Value)]) {
    let bad: Vec<_> = cases.iter().filter(|(_, got, want)| !same(got, want)).collect();
    for (k, got, want) in bad.iter().take(12) {
        eprintln!("{name} {k}: got {got} want {want}");
    }
    assert!(bad.is_empty(), "{name}: {} of {} differ", bad.len(), cases.len());
}

fn fv(v: &Value) -> Vec<f64> {
    v.as_array().unwrap().iter().map(|x| x.as_f64().unwrap()).collect()
}

#[test]
fn periodic_table() {
    let g = g();
    let mut c = vec![];
    for (i, row) in g["pt"].as_array().unwrap().iter().enumerate() {
        let z = i + 1;
        let (r, col) = pt_place(z);
        c.push((format!("z{z}"), json!([pt_config(z), pt_full_config(z), [r, col]]), row.clone()));
    }
    for row in g["molar"].as_array().unwrap() {
        let f = row[0].as_str().unwrap();
        let got = match molar_mass(f) {
            Ok((m, t)) => json!({"ok": [m, Value::Object(t.into_iter().map(|(k, v)| (k, json!(v))).collect())]}),
            Err(m) => json!({"err": "ValueError", "msg": m}),
        };
        let mut want = row[1].clone();
        if want.get("err").is_some() {
            want["err"] = json!("ValueError");
            got.get("msg").map(|m| assert_eq!(m, &row[1]["msg"], "molar {f} message"));
        }
        c.push((f.into(), got, want));
    }
    check("pt", &c);
}

fn mem() -> BTreeMap<String, f64> {
    [("A", 2.0), ("B", 3.0), ("X", 5.0)].into_iter().map(|(k, v)| (k.to_string(), v)).collect()
}

#[test]
fn calculator_eval() {
    let g = g();
    let m = mem();
    let mut c = vec![];
    for r in g["prep"].as_array().unwrap() {
        c.push((r[0].to_string(), json!(calc_prepare(r[0].as_str().unwrap())), r[1].clone()));
    }
    for r in g["eval"].as_array().unwrap() {
        let e = r[0].as_str().unwrap();
        let deg = r[1].as_bool().unwrap();
        c.push((format!("{e:?} deg={deg}"), tr(cg_eval(e, 7.0, deg, None, Some(&m)), |v| json!(v)), r[2].clone()));
    }
    for r in g["eval_x"].as_array().unwrap() {
        let e = r[0].as_str().unwrap();
        let x = r[1].as_f64().unwrap();
        c.push((format!("{e:?} x={x}"), tr(cg_eval(e, 0.0, false, Some(x), Some(&m)), |v| json!(v)), r[2].clone()));
    }
    for r in g["fmt"].as_array().unwrap() {
        let v = r[0].as_f64().unwrap();
        let v = if r[0].to_string() == "-0.0" { -0.0 } else { v };
        c.push((format!("{v:e} exact"), tr(calc_fmt(v, true), |s| json!(s)), r[1].clone()));
        c.push((format!("{v:e} dec"), tr(calc_fmt(v, false), |s| json!(s)), r[2].clone()));
    }
    check("calc", &c);
}

#[test]
fn calculator_solvers() {
    let g = g();
    let mut c = vec![];
    for r in g["roots"].as_array().unwrap() {
        c.push((r[0].to_string(), tr(calc_roots_poly(&fv(&r[0])), |v| json!(v.iter().map(|z| [z.0, z.1]).collect::<Vec<_>>())), r[1].clone()));
    }
    for r in g["linear"].as_array().unwrap() {
        let rows: Vec<Vec<f64>> = r[0].as_array().unwrap().iter().map(fv).collect();
        c.push((r[0].to_string(), tr(calc_solve_linear(&rows), |v| json!(v)), r[1].clone()));
    }
    for r in g["solve"].as_array().unwrap() {
        let e = r[0].as_str().unwrap();
        let deg = r[1].as_bool().unwrap();
        c.push((format!("{e:?} deg={deg}"), tr(cg_solve(e, 0.0, deg, None, -1000.0, 1000.0, 12), |v| json!(v)), r[2].clone()));
    }
    let ma: BTreeMap<String, f64> = [("A".to_string(), 9.0)].into();
    c.push(("x^2=A".into(), tr(cg_solve("x^2=A", 0.0, false, Some(&ma), -1000.0, 1000.0, 12), |v| json!(v)), g["solve_mem"][0][1].clone()));
    for r in g["stats"].as_array().unwrap() {
        c.push((r[0].to_string(), tr(calc_stats1(&fv(&r[0])), |v| json!(v.iter().map(|(k, x)| json!([k, fl(*x)])).collect::<Vec<_>>())), r[1].clone()));
    }
    for r in g["linreg"].as_array().unwrap() {
        let got = tr(calc_linreg(&fv(&r[0]), &fv(&r[1])), |v| json!(v.iter().map(|(k, x)| match x {
            Sv::F(f) => json!([k, fl(*f)]),
            Sv::S(s) => json!([k, s]),
        }).collect::<Vec<_>>()));
        c.push((r[0].to_string(), got, r[2].clone()));
    }
    for r in g["normcd"].as_array().unwrap() {
        let a = fv(&r[0]);
        c.push((r[0].to_string(), tr(calc_normcd(a[0], a[1], a[2], a[3]), |v| json!(v)), r[1].clone()));
    }
    for r in g["invnorm"].as_array().unwrap() {
        let a = r[0].as_array().unwrap();
        let (ar, sd, mu, tail) = (a[0].as_f64().unwrap(), a[1].as_f64().unwrap(), a[2].as_f64().unwrap(), a[3].as_str().unwrap());
        c.push((r[0].to_string(), tr(calc_invnorm(ar, sd, mu, tail), |(lo, hi)| match hi { Some(h) => json!([lo, h]), None => json!(lo) }), r[1].clone()));
    }
    for r in g["binpd"].as_array().unwrap() {
        let a = fv(&r[0]);
        c.push((r[0].to_string(), tr(calc_binpd(a[0], a[1], a[2]), |v| json!(v)), r[1].clone()));
    }
    for r in g["bincd"].as_array().unwrap() {
        let a = fv(&r[0]);
        c.push((r[0].to_string(), tr(calc_bincd(a[0], a[1], a[2], a[3]), |v| json!(v)), r[1].clone()));
    }
    for (i, r) in g["poly_form"].as_array().unwrap().iter().enumerate() {
        c.push((format!("poly {}", i + 2), json!(poly_form(i + 2)), r.clone()));
    }
    for r in g["tokens"].as_array().unwrap() {
        c.push((r[0].to_string(), json!(cg_tokens(r[0].as_str().unwrap())), r[1].clone()));
    }
    check("solvers", &c);
}

#[test]
fn french_and_bible() {
    let g = g();
    let mut c = vec![];
    for r in g["fr"].as_array().unwrap() {
        let q = r[0].as_str().unwrap();
        let hits: Vec<Value> = notes::fr_vocab_search(q, 12).into_iter().map(|(a, b, t)| json!([a, b, t])).collect();
        c.push((q.into(), json!([hits, notes::looks_french(q)]), json!([r[1], r[2]])));
    }
    for r in g["refs"].as_array().unwrap() {
        let got = match routes17::bible_parse_ref(r[0].as_str().unwrap()) {
            Some((b, ch, v)) => json!([b, ch, v]),
            None => Value::Null,
        };
        c.push((r[0].to_string(), got, r[1].clone()));
    }
    for r in g["books"].as_array().unwrap() {
        c.push((r[0].to_string(), json!(routes17::bible_book(r[0].as_str().unwrap())), r[1].clone()));
    }
    for r in g["strip"].as_array().unwrap() {
        c.push((r[0].to_string(), json!(bible_strip(r[0].as_str().unwrap())), r[1].clone()));
    }
    for r in g["step"].as_array().unwrap() {
        let got = match step_parse_line(r[0].as_str().unwrap()) {
            Some((b, ch, v, w)) => json!([b, ch, v, w]),
            None => Value::Null,
        };
        c.push((r[0].to_string(), got, r[1].clone()));
    }
    for r in g["morph"].as_array().unwrap() {
        c.push((r[0].to_string(), json!(morph_text(r[0].as_str().unwrap(), r[1].as_str().unwrap())), r[2].clone()));
    }
    let lex = step_lexicon("Greek");
    for r in g["lookup"].as_array().unwrap() {
        c.push((r[0].to_string(), step_lookup(lex, r[0].as_str().unwrap()).cloned().unwrap_or(Value::Null), r[1].clone()));
    }
    check("fr/bible", &c);
}
