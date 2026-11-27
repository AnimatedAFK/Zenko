//! zenko.py calc_eval + fmt_num (golden/calc.json, tools/make_golden_calc.py): every expression's answer, error and
//! divide-by-zero must match.

use zenko_desktop::calc::{CalcErr, calc_eval, fmt_num};

#[test]
fn calc_matches_zenko_py() {
    let g: Vec<(String, String)> = serde_json::from_str(&std::fs::read_to_string("../../golden/calc.json").unwrap()).unwrap();
    let mut bad = vec![];
    for (e, want) in &g {
        let got = match calc_eval(e) {
            Ok(v) => fmt_num(&v),
            Err(CalcErr::ZeroDiv) => "ZD".into(),
            Err(CalcErr::Other) => "ERR".into(),
        };
        if &got != want {
            bad.push(format!("{e:?}: want {want} got {got}"));
        }
    }
    assert!(bad.is_empty(), "{} differ:\n{}", bad.len(), bad.join("\n"));
}
