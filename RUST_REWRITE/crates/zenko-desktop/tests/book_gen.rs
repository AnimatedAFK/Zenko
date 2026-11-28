//! The book pages' practice generators against zenko.py (golden/book_gen.json): chem_problem (seed 5, 80 draws),
//! math_problem (seed 6, 90), psy_prompt (seed 7, 30), practice_psych's term quiz (seed 8, 20), num_answer on typed
//! answers and english_check on a short and a long paragraph. Same seed, same random calls, same text.
use serde_json::Value;
use zenko_desktop::bookgen::*;
use zenko_sound::mt::PyRandom;

fn golden() -> Value {
    serde_json::from_str(&std::fs::read_to_string("../../golden/book_gen.json").unwrap()).unwrap()
}

fn chk(p: &Problem, g: &Value, i: usize, what: &str) {
    assert_eq!(p.q, g["q"].as_str().unwrap(), "{what} {i} q");
    assert_eq!(p.a, g["a"].as_f64().unwrap(), "{what} {i} a ({})", p.q);
    assert_eq!(p.sol, g["sol"].as_str().unwrap(), "{what} {i} sol");
    assert_eq!(p.topic, g["topic"].as_str().unwrap(), "{what} {i} topic");
    assert_eq!(p.unit.as_deref(), g["unit"].as_str(), "{what} {i} unit");
    assert_eq!(p.p, g["p"].as_i64(), "{what} {i} p");
}

#[test]
fn chem_and_math_problems_match_zenko_py() {
    let g = golden();
    let mut r = PyRandom::new(5);
    for (i, w) in g["chem"].as_array().unwrap().iter().enumerate() {
        chk(&chem_problem(&mut r), w, i, "chem");
    }
    let mut r = PyRandom::new(6);
    for (i, w) in g["math"].as_array().unwrap().iter().enumerate() {
        chk(&math_problem(&mut r), w, i, "math");
    }
}

#[test]
fn psych_prompts_and_term_quiz_match_zenko_py() {
    let g = golden();
    let mut r = PyRandom::new(7);
    for w in g["psy"].as_array().unwrap() {
        let p = psy_prompt(&mut r);
        let s = |k: &str| w[k].as_str().unwrap().to_string();
        assert_eq!((p.concept, p.context, p.topic, p.q, p.hint), (s("concept"), s("context"), s("topic"), s("q"), s("hint")));
    }
    let mut r = PyRandom::new(8);
    for w in g["psy_terms"].as_array().unwrap() {
        let (term, _, ch) = psy_term_quiz(&mut r);
        assert_eq!(term, w[0].as_str().unwrap());
        let want: Vec<String> = w[1].as_array().unwrap().iter().map(|v| v.as_str().unwrap().into()).collect();
        assert_eq!(ch, want);
    }
}

#[test]
fn answer_checks_match_zenko_py() {
    let g = golden();
    for (t, w) in g["num"].as_object().unwrap() {
        assert_eq!(num_answer(t), w.as_f64(), "num_answer({t:?})");
    }
    let ins = ["short", "The writer uses a metaphor \"a sea of troubles\" which suggests the reader feels overwhelmed; this conveys the author's purpose to persuade the audience that the global issue matters, and the juxtaposition of images highlights contrast. Then after that the story is about a man who goes to town and at the end he leaves, which shows the theme."];
    for (t, w) in ins.iter().zip(g["en"].as_array().unwrap()) {
        assert_eq!(english_check(t), w.as_str().unwrap());
    }
}
