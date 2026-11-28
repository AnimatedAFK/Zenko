//! zenko.py's fx-CG50 calculator maths: calc_prepare, cg_eval (a safe walk over Python's ast of the prepared text),
//! calc_fmt (the natural display: fractions, π-multiples, surds), calc_roots_poly, calc_solve_linear, cg_solve,
//! calc_stats1, calc_linreg and the normal / binomial distributions. Float rules follow CPython 3.12 exactly
//! (Neumaier sum, Fraction.limit_denominator, cmath.sqrt, complex ** int, math module domain errors).

use crate::calc::{Num, Tok, big_to_f64, c_mul, c_powu, c_quot, float_pow, int_truediv, lex};
use num_bigint::BigInt;
use num_integer::Integer;
use num_traits::{One, Signed, ToPrimitive, Zero};
use regex::Regex;
use std::collections::BTreeMap;
use std::sync::OnceLock;

macro_rules! rx {
    ($p:expr) => {{
        static R: OnceLock<Regex> = OnceLock::new();
        R.get_or_init(|| Regex::new($p).unwrap())
    }};
}

/// What cg_eval and friends raise: a CalcError (its message is what the screen shows) or a bare Python exception
/// that zenko.py lets through (int(nan) inside `**`, Fraction(inf) in calc_fmt...).
#[derive(Clone, Debug, PartialEq)]
pub enum CgErr {
    Calc(&'static str),
    Py(&'static str),
}

impl CgErr {
    pub fn msg(&self) -> &'static str {
        match self {
            CgErr::Calc(m) => m,
            CgErr::Py(m) => m,
        }
    }
}

const SYN: CgErr = CgErr::Calc("Syntax ERROR");
const MATH: CgErr = CgErr::Calc("Math ERROR");

pub const CALC_FUNCS: [&str; 22] = ["sin", "cos", "tan", "asin", "acos", "atan", "sinh", "cosh", "tanh", "log", "ln", "sqrt",
                                    "cbrt", "abs", "exp", "nCr", "nPr", "fact", "logab", "Int", "Frac", "root"];

/// calc_prepare(expr): calculator keys -> a Python expression with implicit multiplication.
pub fn calc_prepare(expr: &str) -> String {
    let mut s = expr.to_string();
    for (a, b) in [("sin⁻¹(", "asin("), ("cos⁻¹(", "acos("), ("tan⁻¹(", "atan("), ("³√(", "cbrt("), ("√(", "sqrt("),
                   ("×10^", "*10**"), ("ᴇ", "*10**"), ("e^(", "exp("), ("10^(", "10**("), ("×", "*"), ("÷", "/"),
                   ("−", "-"), ("⁻¹", "**(-1)"), ("²", "**2"), ("³", "**3"), ("π", "pi"), ("^", "**"), ("θ", "x"),
                   ("Ans", "ans")] {
        s = s.replace(a, b);
    }
    let s = rx!(r"(\d+(?:\.\d*)?)!").replace_all(&s, "fact($1)").into_owned();
    let digit = rx!(r"^(\d|\.)");
    let name = rx!(r"^[A-Za-z_]+$");
    let mut out: Vec<&str> = vec![];
    for m in rx!(r"\d+\.?\d*(?:[eE][+-]?\d+)?|\.\d+|[A-Za-z_]+|\*\*|[-+*/(),]|\S").find_iter(&s) {
        let t = m.as_str();
        if let Some(&p) = out.last() {
            let pf = CALC_FUNCS.contains(&p);
            let p_val = digit.is_match(p) || p == ")" || (name.is_match(p) && !pf);
            let t_val = digit.is_match(t) || t == "(" || name.is_match(t);
            if p_val && t_val && !pf && !(digit.is_match(p) && digit.is_match(t)) {
                out.push("*");
            }
        }
        out.push(t);
    }
    out.concat()
}

// ---- the parsed tree --------------------------------------------------------------------------------------------
#[derive(Clone, Copy, Debug, PartialEq)]
enum BOp {
    Add,
    Sub,
    Mul,
    Div,
    Pow,
}

/// Only what ev() looks at. `Bad` fails with Syntax ERROR when reached (without looking inside); `BadBin` is a
/// BinOp ev() doesn't do (// % @ << >> & | ^): both sides are worked out first, then Syntax ERROR.
#[derive(Clone, Debug)]
enum Nd {
    Int(BigInt),
    Flt(f64),
    Name(String),
    Bin(BOp, Box<Nd>, Box<Nd>),
    BadBin(Box<Nd>, Box<Nd>),
    Neg(Box<Nd>),
    Pos(Box<Nd>),
    Call(String, Vec<Nd>),
    Bad,
}

const KW: [&str; 35] = ["False", "None", "True", "and", "as", "assert", "async", "await", "break", "class", "continue", "def",
                        "del", "elif", "else", "except", "finally", "for", "from", "global", "if", "import", "in", "is",
                        "lambda", "nonlocal", "not", "or", "pass", "raise", "return", "try", "while", "with", "yield"];

struct P {
    t: Vec<Tok>,
    i: usize,
    depth: usize,
}

type PR<T> = Result<T, ()>;

impl P {
    fn op(&self, o: &str) -> bool {
        matches!(self.t.get(self.i), Some(Tok::Op(x)) if *x == o)
    }
    fn kw(&self, k: &str) -> bool {
        matches!(self.t.get(self.i), Some(Tok::Name(x)) if x == k)
    }
    fn kw_at(&self, i: usize, k: &str) -> bool {
        matches!(self.t.get(i), Some(Tok::Name(x)) if x == k)
    }
    fn eat(&mut self, o: &str) -> bool {
        if self.op(o) {
            self.i += 1;
            true
        } else {
            false
        }
    }
    /// expressions: expression (',' expression)* [','] -> a Tuple when there's a comma
    fn exprs(&mut self) -> PR<Nd> {
        let e = self.test()?;
        if !self.op(",") {
            return Ok(e);
        }
        while self.eat(",") {
            if self.at_end_of_list() {
                break;
            }
            self.star_or_test()?;
        }
        Ok(Nd::Bad)
    }
    fn at_end_of_list(&self) -> bool {
        self.i >= self.t.len() || self.op(")")
    }
    fn star_or_test(&mut self) -> PR<Nd> {
        if self.eat("*") {
            self.bitor()?;
            return Ok(Nd::Bad);
        }
        self.test()
    }
    /// expression: disjunction ['if' disjunction 'else' expression] | lambda (not supported -> SyntaxError)
    fn test(&mut self) -> PR<Nd> {
        if self.kw("lambda") {
            return Err(());
        }
        let e = self.or_test()?;
        if self.kw("if") {
            self.i += 1;
            self.or_test()?;
            if !self.kw("else") {
                return Err(());
            }
            self.i += 1;
            self.test()?;
            return Ok(Nd::Bad);
        }
        Ok(e)
    }
    fn or_test(&mut self) -> PR<Nd> {
        let e = self.and_test()?;
        if !self.kw("or") {
            return Ok(e);
        }
        while self.kw("or") {
            self.i += 1;
            self.and_test()?;
        }
        Ok(Nd::Bad)
    }
    fn and_test(&mut self) -> PR<Nd> {
        let e = self.not_test()?;
        if !self.kw("and") {
            return Ok(e);
        }
        while self.kw("and") {
            self.i += 1;
            self.not_test()?;
        }
        Ok(Nd::Bad)
    }
    fn not_test(&mut self) -> PR<Nd> {
        if self.kw("not") {
            self.i += 1;
            self.not_test()?;
            return Ok(Nd::Bad);
        }
        self.comparison()
    }
    fn comp_op(&mut self) -> bool {
        for o in ["<", ">", "==", ">=", "<=", "!="] {
            if self.eat(o) {
                return true;
            }
        }
        if self.kw("in") {
            self.i += 1;
            return true;
        }
        if self.kw("not") && self.kw_at(self.i + 1, "in") {
            self.i += 2;
            return true;
        }
        if self.kw("is") {
            self.i += 1;
            if self.kw("not") {
                self.i += 1;
            }
            return true;
        }
        false
    }
    fn comparison(&mut self) -> PR<Nd> {
        let e = self.bitor()?;
        let mut cmp = false;
        while self.comp_op() {
            cmp = true;
            self.bitor()?;
        }
        Ok(if cmp { Nd::Bad } else { e })
    }
    fn bad_bin_level(&mut self, ops: &[&str], next: fn(&mut P) -> PR<Nd>) -> PR<Nd> {
        let mut l = next(self)?;
        loop {
            if !ops.iter().any(|o| self.op(o)) {
                return Ok(l);
            }
            self.i += 1;
            let r = next(self)?;
            l = Nd::BadBin(Box::new(l), Box::new(r));
        }
    }
    fn bitor(&mut self) -> PR<Nd> {
        self.bad_bin_level(&["|"], P::bitxor)
    }
    fn bitxor(&mut self) -> PR<Nd> {
        self.bad_bin_level(&["^"], P::bitand)
    }
    fn bitand(&mut self) -> PR<Nd> {
        self.bad_bin_level(&["&"], P::shift)
    }
    fn shift(&mut self) -> PR<Nd> {
        self.bad_bin_level(&["<<", ">>"], P::arith)
    }
    fn arith(&mut self) -> PR<Nd> {
        let mut l = self.term()?;
        loop {
            let op = if self.op("+") { BOp::Add } else if self.op("-") { BOp::Sub } else { return Ok(l) };
            self.i += 1;
            let r = self.term()?;
            l = Nd::Bin(op, Box::new(l), Box::new(r));
        }
    }
    fn term(&mut self) -> PR<Nd> {
        let mut l = self.factor()?;
        loop {
            let op = if self.op("*") {
                Some(BOp::Mul)
            } else if self.op("/") {
                Some(BOp::Div)
            } else if self.op("//") || self.op("%") || self.op("@") {
                None
            } else {
                return Ok(l);
            };
            self.i += 1;
            let r = self.factor()?;
            l = match op {
                Some(op) => Nd::Bin(op, Box::new(l), Box::new(r)),
                None => Nd::BadBin(Box::new(l), Box::new(r)),
            };
        }
    }
    fn factor(&mut self) -> PR<Nd> {
        self.depth += 1;
        if self.depth > 3000 {
            return Err(());
        }
        let r = if self.op("-") || self.op("+") || self.op("~") {
            let o = if self.op("-") { 0 } else if self.op("+") { 1 } else { 2 };
            self.i += 1;
            let f = self.factor()?;
            Ok(match o {
                0 => Nd::Neg(Box::new(f)),
                1 => Nd::Pos(Box::new(f)),
                _ => Nd::Bad,
            })
        } else {
            self.power()
        };
        self.depth -= 1;
        r
    }
    fn power(&mut self) -> PR<Nd> {
        if self.kw("await") {
            return Err(());
        }
        let a = self.primary()?;
        if self.eat("**") {
            let b = self.factor()?;
            return Ok(Nd::Bin(BOp::Pow, Box::new(a), Box::new(b)));
        }
        Ok(a)
    }
    fn primary(&mut self) -> PR<Nd> {
        let mut a = self.atom()?;
        while self.eat("(") {
            let mut pos = vec![];
            let (mut seen_kw, mut seen_dstar) = (false, false);
            let mut kws: Vec<String> = vec![];
            while !self.op(")") {
                if self.eat("**") {
                    self.test()?;
                    seen_dstar = true;
                } else if self.eat("*") {
                    if seen_dstar {
                        return Err(());
                    }
                    self.test()?;
                    pos.push(Nd::Bad);
                } else if matches!(self.t.get(self.i), Some(Tok::Name(_))) && matches!(self.t.get(self.i + 1), Some(Tok::Op("="))) {
                    let Some(Tok::Name(k)) = self.t.get(self.i).cloned() else { return Err(()) };
                    if KW.contains(&k.as_str()) || kws.contains(&k) {
                        return Err(());
                    }
                    kws.push(k);
                    self.i += 2;
                    self.test()?;
                    seen_kw = true;
                } else {
                    if seen_kw || seen_dstar {
                        return Err(());
                    }
                    pos.push(self.test()?);
                }
                if !self.eat(",") {
                    break;
                }
            }
            if !self.eat(")") {
                return Err(());
            }
            a = match a {
                Nd::Name(n) => Nd::Call(n, pos),
                _ => Nd::Bad,
            };
        }
        if self.op(".") || self.op("[") {
            return Err(());
        }
        Ok(a)
    }
    fn atom(&mut self) -> PR<Nd> {
        match self.t.get(self.i).cloned() {
            Some(Tok::Num(n)) => {
                self.i += 1;
                Ok(match n {
                    Num::Int(i) => {
                        if i.magnitude().bits() > 14_300 && i.to_string().trim_start_matches('-').len() > 4300 {
                            return Err(());
                        }
                        Nd::Int(i)
                    }
                    Num::Float(f) => Nd::Flt(f),
                    Num::Complex(..) => Nd::Bad,
                })
            }
            Some(Tok::Imag) => {
                self.i += 1;
                Ok(Nd::Bad)
            }
            Some(Tok::Name(n)) => {
                self.i += 1;
                match n.as_str() {
                    "True" => Ok(Nd::Int(BigInt::one())),
                    "False" => Ok(Nd::Int(BigInt::zero())),
                    "None" => Ok(Nd::Bad),
                    k if KW.contains(&k) => Err(()),
                    _ => Ok(Nd::Name(n)),
                }
            }
            Some(Tok::Op("...")) => {
                self.i += 1;
                Ok(Nd::Bad)
            }
            Some(Tok::Op("(")) => {
                self.i += 1;
                self.depth += 1;
                if self.depth > 200 {
                    return Err(());
                }
                if self.eat(")") {
                    self.depth -= 1;
                    return Ok(Nd::Bad); // ()
                }
                let first_star = self.op("*");
                let e = if first_star { self.star_or_test()? } else { self.test()? };
                let mut tuple = first_star;
                while self.eat(",") {
                    tuple = true;
                    if self.op(")") {
                        break;
                    }
                    self.star_or_test()?;
                }
                if !self.eat(")") {
                    return Err(());
                }
                self.depth -= 1;
                Ok(if tuple { Nd::Bad } else { e })
            }
            _ => Err(()),
        }
    }
}

/// A prepared, parsed expression (cg_solve evaluates the same one thousands of times).
#[derive(Clone, Debug)]
pub struct CgExpr(Nd);

impl CgExpr {
    /// calc_prepare + ast.parse(mode="eval"); Err is "Syntax ERROR"
    pub fn compile(expr: &str) -> Result<CgExpr, CgErr> {
        let src = calc_prepare(expr);
        if src.trim().is_empty() {
            return Err(SYN);
        }
        let t = lex(&src).map_err(|_| SYN)?;
        let mut p = P { t, i: 0, depth: 0 };
        let n = p.exprs().map_err(|_| SYN)?;
        if p.i != p.t.len() {
            return Err(SYN);
        }
        Ok(CgExpr(n))
    }

    pub fn eval(&self, ans: f64, deg: bool, x: Option<f64>, mem: Option<&BTreeMap<String, f64>>) -> Result<f64, CgErr> {
        let env = Env { ans, k: if deg { std::f64::consts::PI / 180.0 } else { 1.0 }, x: x.unwrap_or_else(|| mem.and_then(|m| m.get("X").copied()).unwrap_or(0.0)), mem };
        let v = match ev(&self.0, &env, 0) {
            Ok(v) => v,
            Err(E::Calc(m)) => return Err(CgErr::Calc(m)),
            Err(E::Ovf) | Err(E::Zero) => return Err(MATH),
            Err(E::Rec) => return Err(CgErr::Calc("Stack ERROR")),
            Err(E::Val) => return Err(CgErr::Py("ValueError")),
            Err(E::Type) => return Err(CgErr::Py("TypeError")),
        };
        if v.is_nan() || v.is_infinite() {
            return Err(MATH);
        }
        Ok(v)
    }
}

/// cg_eval(expr, ans, deg, x, mem)
pub fn cg_eval(expr: &str, ans: f64, deg: bool, x: Option<f64>, mem: Option<&BTreeMap<String, f64>>) -> Result<f64, CgErr> {
    CgExpr::compile(expr)?.eval(ans, deg, x, mem)
}

struct Env<'a> {
    ans: f64,
    k: f64,
    x: f64,
    mem: Option<&'a BTreeMap<String, f64>>,
}

/// errors inside the walk: CalcError, or the Python exception that would be raised
#[derive(Debug)]
enum E {
    Calc(&'static str),
    Ovf,
    Zero,
    Val,
    Type,
    Rec,
}

impl<'a> Env<'a> {
    fn mem_val(&self, n: &str) -> Option<f64> {
        let mut cs = n.chars();
        let (Some(c), None) = (cs.next(), cs.next()) else { return None };
        if !(c.is_alphabetic() && c.is_uppercase()) {
            return None;
        }
        self.mem?.get(n).copied()
    }
    fn value(&self, n: &str) -> Option<f64> {
        if let Some(v) = self.mem_val(n) {
            return Some(v);
        }
        match n {
            "pi" => Some(std::f64::consts::PI),
            "e" => Some(std::f64::consts::E),
            "ans" => Some(self.ans),
            "x" => Some(self.x),
            _ => None,
        }
    }
    fn callable(&self, n: &str) -> bool {
        self.mem_val(n).is_none() && CALC_FUNCS.contains(&n)
    }
}

/// math_1: a NaN out of a non-NaN is a domain error (ValueError), an inf out of a finite an OverflowError
fn m1(x: f64, f: fn(f64) -> f64, can_ovf: bool) -> Result<f64, E> {
    let r = f(x);
    if r.is_nan() && !x.is_nan() {
        return Err(E::Val);
    }
    if r.is_infinite() && x.is_finite() {
        return Err(if can_ovf { E::Ovf } else { E::Val });
    }
    Ok(r)
}

/// math.log / math.log10 of a float
fn m_log(x: f64, f: fn(f64) -> f64) -> Result<f64, E> {
    if x.is_nan() {
        return Ok(x);
    }
    if x > 0.0 {
        return Ok(f(x));
    }
    Err(E::Val)
}

fn ln2(x: f64, b: Option<f64>) -> Result<f64, E> {
    let n = m_log(x, f64::ln)?;
    match b {
        None => Ok(n),
        Some(b) => {
            let d = m_log(b, f64::ln)?;
            if d == 0.0 {
                return Err(E::Zero);
            }
            Ok(n / d)
        }
    }
}

fn fdiv(a: f64, b: f64) -> Result<f64, E> {
    if b == 0.0 {
        return Err(E::Zero);
    }
    Ok(a / b)
}

fn fpow(a: f64, b: f64) -> Result<f64, E> {
    match float_pow(a, b) {
        Ok(Num::Float(f)) => Ok(f),
        Ok(_) => Err(E::Val),
        Err(crate::calc::CalcErr::ZeroDiv) => Err(E::Zero),
        Err(_) => Err(E::Ovf),
    }
}

/// int(v) for a float: OverflowError for inf, ValueError for NaN
fn f_int(v: f64) -> Result<f64, E> {
    if v.is_nan() {
        return Err(E::Val);
    }
    if v.is_infinite() {
        return Err(E::Ovf);
    }
    Ok(v.trunc() + 0.0)
}

fn big_of(v: f64) -> BigInt {
    num_rational_big(v).0
}

fn bigf(b: &BigInt) -> Result<f64, E> {
    big_to_f64(b).map_err(|_| E::Ovf)
}

fn comb(n: &BigInt, r: &BigInt) -> Result<f64, E> {
    let r = std::cmp::min(r.clone(), n - r);
    let rr = r.to_u64().ok_or(E::Ovf)?;
    let mut c = BigInt::one();
    for i in 0..rr {
        c = c * (n - BigInt::from(i)) / BigInt::from(i + 1);
        if c.bits() > 1100 {
            return Err(E::Ovf);
        }
    }
    bigf(&c)
}

fn perm(n: &BigInt, r: &BigInt) -> Result<f64, E> {
    let rr = r.to_u64().ok_or(E::Ovf)?;
    let mut c = BigInt::one();
    for i in 0..rr {
        c *= n - BigInt::from(i);
        if c.bits() > 1100 {
            return Err(E::Ovf);
        }
    }
    bigf(&c)
}

fn call(env: &Env, f: &str, a: &[f64]) -> Result<f64, E> {
    let k = env.k;
    let one = |a: &[f64]| if a.len() == 1 { Ok(a[0]) } else { Err(E::Type) };
    let two = |a: &[f64]| if a.len() == 2 { Ok((a[0], a[1])) } else { Err(E::Type) };
    let int_chk = |n: f64| -> Result<bool, E> { Ok(n != f_int(n)?) };
    match f {
        "sin" => m1(one(a)? * k, f64::sin, false),
        "cos" => m1(one(a)? * k, f64::cos, false),
        "tan" => m1(one(a)? * k, f64::tan, false),
        "asin" => Ok(m1(one(a)?, f64::asin, false)? / k),
        "acos" => Ok(m1(one(a)?, f64::acos, false)? / k),
        "atan" => Ok(m1(one(a)?, f64::atan, false)? / k),
        "sinh" => m1(one(a)?, f64::sinh, true),
        "cosh" => m1(one(a)?, f64::cosh, true),
        "tanh" => m1(one(a)?, f64::tanh, false),
        "log" => match a.len() {
            1 => m_log(a[0], f64::log10),
            2 => {
                let n = m_log(a[1], f64::ln)?;
                fdiv(n, m_log(a[0], f64::ln)?)
            }
            _ => Err(E::Type),
        },
        "logab" => {
            let (x, y) = two(a)?;
            let n = m_log(y, f64::ln)?;
            fdiv(n, m_log(x, f64::ln)?)
        }
        "ln" => match a.len() {
            1 => ln2(a[0], None),
            2 => ln2(a[0], Some(a[1])),
            _ => Err(E::Type),
        },
        "sqrt" => m1(one(a)?, f64::sqrt, false),
        "cbrt" => {
            let v = one(a)?;
            Ok(fpow(v.abs(), 1.0 / 3.0)?.copysign(v))
        }
        "root" => {
            let (n, v) = two(a)?;
            Ok(fpow(v.abs(), fdiv(1.0, n)?)?.copysign(v))
        }
        "abs" => Ok(one(a)?.abs()),
        "exp" => m1(one(a)?, f64::exp, true),
        "fact" => {
            let n = one(a)?;
            if n < 0.0 || int_chk(n)? || n > 170.0 {
                return Err(E::Calc("Math ERROR"));
            }
            let mut c = BigInt::one();
            for i in 2..=(n as u64) {
                c *= i;
            }
            bigf(&c)
        }
        "nCr" | "nPr" => {
            let (n, r) = two(a)?;
            if n.min(r) < 0.0 || int_chk(n)? || int_chk(r)? || r > n {
                return Err(E::Calc("Math ERROR"));
            }
            let (bn, br) = (big_of(n), big_of(r));
            if f == "nCr" { comb(&bn, &br) } else { perm(&bn, &br) }
        }
        "Int" => f_int(one(a)?),
        "Frac" => {
            let v = one(a)?;
            Ok(v - f_int(v)?)
        }
        _ => Err(E::Type),
    }
}

fn ev(n: &Nd, env: &Env, depth: usize) -> Result<f64, E> {
    if depth > 990 {
        return Err(E::Rec);
    }
    match n {
        Nd::Int(i) => bigf(i),
        Nd::Flt(f) => Ok(*f),
        Nd::Name(s) => {
            if env.callable(s) {
                return Err(E::Calc("Syntax ERROR"));
            }
            env.value(s).ok_or(E::Calc("Syntax ERROR"))
        }
        Nd::Neg(a) => Ok(-ev(a, env, depth + 1)?),
        Nd::Pos(a) => ev(a, env, depth + 1),
        Nd::BadBin(l, r) => {
            ev(l, env, depth + 1)?;
            ev(r, env, depth + 1)?;
            Err(E::Calc("Syntax ERROR"))
        }
        Nd::Bin(op, l, r) => {
            let a = ev(l, env, depth + 1)?;
            let b = ev(r, env, depth + 1)?;
            match op {
                BOp::Add => Ok(a + b),
                BOp::Sub => Ok(a - b),
                BOp::Mul => Ok(a * b),
                BOp::Div => {
                    if b == 0.0 {
                        return Err(E::Calc("Math ERROR"));
                    }
                    Ok(a / b)
                }
                BOp::Pow => {
                    if b.abs() > 1e4 && a.abs() > 1.0 {
                        return Err(E::Calc("Math ERROR"));
                    }
                    if a < 0.0 && b != f_int(b)? {
                        let fr = Frac::from_f64(b).limit_denominator(50);
                        if (fr.to_f64() - b).abs() < 1e-12 && fr.den.is_odd() {
                            let r = fpow(a.abs(), b)?;
                            return Ok(if fr.num.is_odd() { -r } else { r });
                        }
                        return Err(E::Calc("Non-Real ERROR"));
                    }
                    fpow(a, b)
                }
            }
        }
        Nd::Call(f, args) => {
            if !env.callable(f) {
                return Err(E::Calc("Syntax ERROR"));
            }
            let mut vs = Vec::with_capacity(args.len());
            for a in args {
                vs.push(ev(a, env, depth + 1)?);
            }
            match call(env, f, &vs) {
                Ok(v) => Ok(v),
                Err(E::Calc(m)) => Err(E::Calc(m)),
                Err(_) => Err(E::Calc("Math ERROR")),
            }
        }
        Nd::Bad => Err(E::Calc("Syntax ERROR")),
    }
}

// ---- fractions.Fraction (just what calc_fmt and `**` use) ------------------------------------------------------
fn num_rational_big(v: f64) -> (BigInt, BigInt) {
    // exact numerator / denominator of a finite float
    if v == 0.0 {
        return (BigInt::zero(), BigInt::one());
    }
    let bits = v.to_bits();
    let neg = bits >> 63 == 1;
    let ex = ((bits >> 52) & 0x7ff) as i64;
    let mut man = bits & ((1u64 << 52) - 1);
    let e = if ex == 0 { -1074 } else {
        man |= 1u64 << 52;
        ex - 1075
    };
    let tz = man.trailing_zeros() as i64;
    let (man, e) = (man >> tz, e + tz);
    let mut n = BigInt::from(man);
    let mut d = BigInt::one();
    if e >= 0 {
        n <<= e as usize;
    } else {
        d <<= (-e) as usize;
    }
    if neg {
        n = -n;
    }
    (n, d)
}

#[derive(Clone, Debug, PartialEq)]
pub struct Frac {
    pub num: BigInt,
    pub den: BigInt,
}

impl Frac {
    pub fn from_f64(v: f64) -> Frac {
        let (num, den) = num_rational_big(v);
        Frac { num, den }
    }
    fn new(n: BigInt, d: BigInt) -> Frac {
        let g = n.gcd(&d);
        let (mut n, mut d) = if g.is_zero() { (n, d) } else { (n / &g, d / &g) };
        if d.is_negative() {
            n = -n;
            d = -d;
        }
        Frac { num: n, den: d }
    }
    pub fn to_f64(&self) -> f64 {
        int_truediv(&self.num, &self.den).unwrap_or(f64::INFINITY)
    }
    /// Fraction.limit_denominator (3.12)
    pub fn limit_denominator(&self, max_d: i64) -> Frac {
        let max_d = BigInt::from(max_d);
        if self.den <= max_d {
            return self.clone();
        }
        let (mut p0, mut q0, mut p1, mut q1) = (BigInt::zero(), BigInt::one(), BigInt::one(), BigInt::zero());
        let (mut n, mut d) = (self.num.clone(), self.den.clone());
        loop {
            let a = n.div_floor(&d);
            let q2 = &q0 + &a * &q1;
            if q2 > max_d {
                break;
            }
            let np1 = &p0 + &a * &p1;
            p0 = std::mem::replace(&mut p1, np1);
            q0 = std::mem::replace(&mut q1, q2);
            let nd = &n - &a * &d;
            n = std::mem::replace(&mut d, nd);
        }
        let k = (&max_d - &q0).div_floor(&q1);
        if BigInt::from(2) * &d * (&q0 + &k * &q1) <= self.den {
            Frac { num: p1, den: q1 }
        } else {
            Frac { num: &p0 + &k * &p1, den: &q0 + &k * &q1 }
        }
    }
}

fn frac_of(q: f64) -> Result<Frac, CgErr> {
    if q.is_nan() {
        return Err(CgErr::Py("ValueError"));
    }
    if q.is_infinite() {
        return Err(CgErr::Py("OverflowError"));
    }
    Ok(Frac::from_f64(q))
}

/// Python's "%.<p>g"
pub fn py_g(v: f64, p: usize) -> String {
    if v.is_nan() {
        return "nan".into();
    }
    if v.is_infinite() {
        return if v < 0.0 { "-inf".into() } else { "inf".into() };
    }
    let p = p.max(1);
    if v == 0.0 {
        return if v.is_sign_negative() { "-0".into() } else { "0".into() };
    }
    let e_s = format!("{:.*e}", p - 1, v);
    let (m, ex) = e_s.split_once('e').unwrap();
    let x: i32 = ex.parse().unwrap();
    if x < -4 || x >= p as i32 {
        let m = if m.contains('.') { m.trim_end_matches('0').trim_end_matches('.') } else { m };
        format!("{m}e{}{:02}", if x < 0 { '-' } else { '+' }, x.abs())
    } else {
        let s = format!("{:.*}", (p as i32 - 1 - x).max(0) as usize, v);
        if s.contains('.') { s.trim_end_matches('0').trim_end_matches('.').to_string() } else { s }
    }
}

/// calc_fmt(v, exact)
pub fn calc_fmt(v: f64, exact: bool) -> Result<String, CgErr> {
    if exact {
        for (mult, tag) in [(1.0, ""), (std::f64::consts::PI, "π")] {
            let q = v / mult;
            let fr = frac_of(q)?.limit_denominator(1000);
            if (fr.to_f64() - q).abs() < 1e-11 * 1f64.max(q.abs()) && (!fr.den.is_one() || !tag.is_empty())
                && fr.num.abs() < BigInt::from(10_000_000) {
                if !tag.is_empty() && fr.num.is_zero() {
                    continue;
                }
                let num = &fr.num;
                let n_s = if !tag.is_empty() {
                    let a = num.abs();
                    let s = (if a.is_one() { String::new() } else { a.to_string() }) + "π";
                    (if num.is_negative() { "-" } else { "" }).to_string() + &s
                } else {
                    num.to_string()
                };
                return Ok(if fr.den.is_one() { n_s } else { format!("{n_s}/{}", fr.den) });
            }
            if tag.is_empty() {
                let qq = q * q;
                let r = frac_of(qq)?.limit_denominator(1000);
                if q != 0.0 && (r.to_f64() - qq).abs() < 1e-10 * 1f64.max(qq) && r.den <= BigInt::from(100) {
                    for d in 1..=100i64 {
                        let rad = Frac::new(&r.num * d * d, r.den.clone());
                        if rad.den.is_one() {
                            let n_int = rad.num.clone();
                            if n_int >= BigInt::from(1_000_000) {
                                break;
                            }
                            let n_int = n_int.to_i64().unwrap();
                            let (mut out_c, mut rest) = (1i64, n_int);
                            let mut p = 2i64;
                            while p * p <= rest {
                                while rest % (p * p) == 0 {
                                    rest /= p * p;
                                    out_c *= p;
                                }
                                p += 1;
                            }
                            if rest > 1 && n_int < 1_000_000 {
                                let c = (if out_c == 1 { String::new() } else { out_c.to_string() }) + "√" + &rest.to_string();
                                let c = (if q < 0.0 { "-" } else { "" }).to_string() + &c;
                                return Ok(if d == 1 { c } else { format!("{c}/{d}") });
                            }
                            break;
                        }
                    }
                }
            }
        }
    }
    let iv = f_int(v).map_err(|e| CgErr::Py(if matches!(e, E::Val) { "ValueError" } else { "OverflowError" }))?;
    if v == iv && v.abs() < 1e10 {
        return Ok(format!("{}", iv as i64));
    }
    let a = v.abs();
    if a >= 1e10 || a < 1e-9 {
        let s = format!("{v:.9e}");
        let (m, ex) = s.split_once('e').unwrap();
        let m = m.trim_end_matches('0').trim_end_matches('.');
        return Ok(format!("{m}×10^{}", ex.parse::<i32>().unwrap()));
    }
    Ok(py_g(v, 10))
}

// ---- polynomials, linear systems, SolveN --------------------------------------------------------------------------
type C = (f64, f64);

fn c_add(a: C, b: C) -> C {
    (a.0 + b.0, a.1 + b.1)
}
fn c_sub(a: C, b: C) -> C {
    (a.0 - b.0, a.1 - b.1)
}
fn c_abs(a: C) -> f64 {
    if a.0.is_infinite() || a.1.is_infinite() {
        return f64::INFINITY;
    }
    a.0.hypot(a.1)
}
fn cdiv(a: C, b: C) -> Result<C, CgErr> {
    c_quot(a, b).map_err(|_| CgErr::Py("ZeroDivisionError"))
}

/// cmath.sqrt (finite inputs)
pub fn c_sqrt(z: C) -> C {
    if z.0 == 0.0 && z.1 == 0.0 {
        return (0.0, z.1);
    }
    let (mut ax, ay) = (z.0.abs(), z.1.abs());
    let s = if ax < f64::MIN_POSITIVE && ay < f64::MIN_POSITIVE {
        ax = ax * 2f64.powi(53);
        (ax + ax.hypot(ay * 2f64.powi(53))).sqrt() * 2f64.powi(-27)
    } else {
        ax /= 8.0;
        2.0 * (ax + ax.hypot(ay / 8.0)).sqrt()
    };
    let d = ay / (2.0 * s);
    if z.0 >= 0.0 { (s, d.copysign(z.1)) } else { (d, s.copysign(z.1)) }
}

/// calc_roots_poly(coefs): every root, highest power first
pub fn calc_roots_poly(coefs: &[f64]) -> Result<Vec<C>, CgErr> {
    let mut c: Vec<C> = coefs.iter().map(|&x| (x, 0.0)).collect();
    while !c.is_empty() && c_abs(c[0]) < 1e-15 {
        c.remove(0);
    }
    if c.len() < 2 {
        return Err(MATH);
    }
    let n = c.len() - 1;
    if n == 1 {
        return Ok(vec![cdiv((-c[1].0, -c[1].1), c[0])?]);
    }
    if n == 2 {
        let (a, b, cc) = (c[0], c[1], c[2]);
        let d = c_sqrt(c_sub(c_mul(b, b), c_mul(c_mul((4.0, 0.0), a), cc)));
        let nb = (-b.0, -b.1);
        let a2 = c_mul((2.0, 0.0), a);
        return Ok(vec![cdiv(c_add(nb, d), a2)?, cdiv(c_sub(nb, d), a2)?]);
    }
    let a0 = c[0];
    let c: Vec<C> = c.iter().map(|&x| cdiv(x, a0)).collect::<Result<_, _>>()?;
    let mut roots: Vec<C> = (0..n).map(|k| c_powu((0.4, 0.9), k as i64)).collect();
    for _ in 0..500 {
        let mut new = Vec::with_capacity(n);
        for (i, &r) in roots.iter().enumerate() {
            let mut num = (0.0, 0.0);
            for &x in &c {
                num = c_add(c_mul(num, r), x);
            }
            let mut den = (1.0, 0.0);
            for (j, &s) in roots.iter().enumerate() {
                if j != i {
                    den = c_mul(den, c_sub(r, s));
                }
            }
            new.push(if den != (0.0, 0.0) { c_sub(r, cdiv(num, den)?) } else { r });
        }
        let done = new.iter().zip(&roots).map(|(a, b)| c_abs(c_sub(*a, *b))).fold(f64::NEG_INFINITY, py_max) < 1e-14;
        roots = new;
        if done {
            break;
        }
    }
    Ok(roots.iter().map(|r| (py_round(r.0, 12), py_round(r.1, 12))).collect())
}

/// Python's max(): keeps the first when the new one isn't bigger (NaN never wins)
fn py_max(a: f64, b: f64) -> f64 {
    if b > a || a == f64::NEG_INFINITY && !b.is_nan() { b } else { a }
}

pub fn py_round(x: f64, n: usize) -> f64 {
    if !x.is_finite() {
        return x;
    }
    crate::bookgen::py_round(x, n)
}

/// calc_solve_linear(rows)
pub fn calc_solve_linear(rows: &[Vec<f64>]) -> Result<Vec<f64>, CgErr> {
    let n = rows.len();
    let mut m: Vec<Vec<f64>> = rows.to_vec();
    for i in 0..n {
        let mut p = i;
        for r in i..n {
            let v = m[r].get(i).copied().ok_or(CgErr::Py("IndexError"))?;
            if v.abs() > m[p][i].abs() {
                p = r;
            }
        }
        if m[p][i].abs() < 1e-13 {
            return Err(CgErr::Calc("No Solution / Infinitely Many"));
        }
        m.swap(i, p);
        for r in 0..n {
            if r != i {
                let f = m[r][i] / m[i][i];
                for c in i..=n {
                    let sub = f * m[i].get(c).copied().ok_or(CgErr::Py("IndexError"))?;
                    let cell = m[r].get_mut(c).ok_or(CgErr::Py("IndexError"))?;
                    *cell -= sub;
                }
            }
        }
    }
    (0..n).map(|i| m[i].get(n).map(|v| v / m[i][i]).ok_or(CgErr::Py("IndexError"))).collect()
}

/// cg_solve(eq, ans, deg, mem, lo, hi, limit)
pub fn cg_solve(eq: &str, ans: f64, deg: bool, mem: Option<&BTreeMap<String, f64>>, lo: f64, hi: f64, limit: usize) -> Result<Vec<f64>, CgErr> {
    let eq = eq.replace('＝', "=");
    if eq.matches('=').count() > 1 {
        return Err(SYN);
    }
    let mut parts = eq.splitn(2, '=');
    let lhs = parts.next().unwrap_or("").to_string();
    let rhs = parts.next().unwrap_or("0").to_string();
    if lhs.trim().is_empty() {
        return Err(SYN);
    }
    let rhs = if rhs.trim().is_empty() { "0".to_string() } else { rhs.trim().to_string() };
    if !lhs.contains('x') && !rhs.contains('x') {
        cg_eval(&lhs.replace('x', "(1)"), ans, deg, None, mem)?;
    }
    let (le, re) = (CgExpr::compile(&lhs).ok(), CgExpr::compile(&rhs).ok());
    let f = |x: f64| -> Option<f64> {
        let a = le.as_ref()?.eval(ans, deg, Some(x), mem).ok()?;
        let b = re.as_ref()?.eval(ans, deg, Some(x), mem).ok()?;
        let v = a - b;
        if v == v && v.abs() < 1e300 { Some(v) } else { None }
    };
    let bisect = |mut a: f64, mut fa: f64, mut b: f64| -> Option<f64> {
        for _ in 0..80 {
            let m = (a + b) / 2.0;
            let fm = f(m)?;
            if fm == 0.0 {
                return Some(m);
            }
            if (fm < 0.0) == (fa < 0.0) {
                a = m;
                fa = fm;
            } else {
                b = m;
            }
        }
        Some((a + b) / 2.0)
    };
    let mut roots: Vec<f64> = vec![];
    let add = |r: Option<f64>, roots: &mut Vec<f64>| {
        let Some(mut r) = r else { return };
        if !(lo <= r && r <= hi) {
            return;
        }
        let Some(fr) = f(r) else { return };
        let scale = 1f64.max(f(r + 1e-3).unwrap_or(0.0).abs()).max(f(r - 1e-3).unwrap_or(0.0).abs());
        if fr.abs() > 1e-7 * scale && fr.abs() > 1e-9 {
            return;
        }
        r = py_round(r, 12);
        if r.abs() < 1e-12 {
            r = 0.0;
        }
        if roots.iter().all(|q| (r - q).abs() > 1e-7 * 1f64.max(r.abs())) {
            roots.push(r);
        }
    };
    for (a0, b0, n) in [(lo.max(-10.0), hi.min(10.0), 4000usize), (lo, hi, 4000)] {
        if a0 >= b0 {
            continue;
        }
        let step = (b0 - a0) / n as f64;
        let xs: Vec<f64> = (0..=n).map(|i| a0 + i as f64 * step).collect();
        let ys: Vec<Option<f64>> = xs.iter().map(|&x| f(x)).collect();
        for i in 0..n {
            let (Some(y0), Some(y1)) = (ys[i], ys[i + 1]) else { continue };
            if y0 == 0.0 {
                add(Some(xs[i]), &mut roots);
            } else if (y0 < 0.0) != (y1 < 0.0) {
                add(bisect(xs[i], y0, xs[i + 1]), &mut roots);
            } else if i > 0 && ys[i - 1].is_some_and(|p| y0.abs() <= p.abs()) && y0.abs() <= y1.abs()
                && y0.abs() < 1e-2 * 1f64.max(y1.abs()) {
                let (mut a, mut b) = (xs[i - 1], xs[i + 1]);
                for _ in 0..100 {
                    let (m1, m2) = (b - (b - a) * 0.618, a + (b - a) * 0.618);
                    let (Some(f1), Some(f2)) = (f(m1), f(m2)) else { break };
                    if f1.abs() < f2.abs() {
                        b = m2;
                    } else {
                        a = m1;
                    }
                }
                add(Some((a + b) / 2.0), &mut roots);
            }
        }
    }
    roots.sort_by(|a, b| a.abs().partial_cmp(&b.abs()).unwrap());
    roots.truncate(limit);
    roots.sort_by(|a, b| a.partial_cmp(b).unwrap());
    Ok(roots)
}

// ---- statistics and distributions ---------------------------------------------------------------------------------
/// sum() of floats in Python 3.12 (Neumaier compensated)
pub fn py_sum(xs: impl IntoIterator<Item = f64>) -> f64 {
    let (mut s, mut c) = (0.0f64, 0.0f64);
    for x in xs {
        let t = s + x;
        if s.abs() >= x.abs() {
            c += (s - t) + x;
        } else {
            c += (x - t) + s;
        }
        s = t;
    }
    if c != 0.0 && c.is_finite() {
        s += c;
    }
    s
}

#[derive(Clone, Debug, PartialEq)]
pub enum Sv {
    F(f64),
    S(&'static str),
}

fn med(a: &[f64]) -> f64 {
    let k = a.len();
    if k == 0 {
        return f64::NAN;
    }
    if k % 2 == 1 { a[k / 2] } else { (a[k / 2 - 1] + a[k / 2]) / 2.0 }
}

/// calc_stats1(xs)
pub fn calc_stats1(xs: &[f64]) -> Result<Vec<(&'static str, f64)>, CgErr> {
    let n = xs.len();
    if n == 0 {
        return Err(CgErr::Calc("Dimension ERROR"));
    }
    let mut s = xs.to_vec();
    s.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let nf = n as f64;
    let mean = py_sum(xs.iter().copied()) / nf;
    let sx2 = py_sum(xs.iter().map(|v| v * v));
    let var = py_sum(xs.iter().map(|v| (v - mean) * (v - mean))) / nf;
    let half = n / 2;
    let q1 = med(&s[..half]);
    let q3 = med(&s[half + n % 2..]);
    Ok(vec![("x̄", mean), ("Σx", py_sum(xs.iter().copied())), ("Σx²", sx2), ("σx", var.sqrt()),
            ("sx", if n > 1 { (var * nf / (nf - 1.0)).sqrt() } else { f64::NAN }), ("n", nf), ("minX", s[0]), ("Q1", q1),
            ("Med", med(&s)), ("Q3", q3), ("maxX", s[n - 1])])
}

/// calc_linreg(xs, ys)
pub fn calc_linreg(xs: &[f64], ys: &[f64]) -> Result<Vec<(&'static str, Sv)>, CgErr> {
    let n = xs.len();
    if n < 2 || n != ys.len() {
        return Err(CgErr::Calc("Dimension ERROR"));
    }
    let nf = n as f64;
    let (mx, my) = (py_sum(xs.iter().copied()) / nf, py_sum(ys.iter().copied()) / nf);
    let sxx = py_sum(xs.iter().map(|x| (x - mx) * (x - mx)));
    let syy = py_sum(ys.iter().map(|y| (y - my) * (y - my)));
    let sxy = py_sum(xs.iter().zip(ys).map(|(x, y)| (x - mx) * (y - my)));
    if sxx == 0.0 {
        return Err(MATH);
    }
    let b = sxy / sxx;
    let a = my - b * mx;
    let r = if syy != 0.0 { sxy / (sxx * syy).sqrt() } else { f64::NAN };
    Ok(vec![("a", Sv::F(a)), ("b", Sv::F(b)), ("r", Sv::F(r)), ("r²", Sv::F(r * r)), ("y=", Sv::S("a+bx"))])
}

unsafe extern "C" {
    fn erf(x: f64) -> f64;
}

pub fn norm_cdf(z: f64) -> f64 {
    0.5 * (1.0 + unsafe { erf(z / 2f64.sqrt()) })
}

pub fn calc_normcd(lo: f64, hi: f64, sd: f64, mu: f64) -> Result<f64, CgErr> {
    if sd <= 0.0 {
        return Err(MATH);
    }
    Ok(norm_cdf((hi - mu) / sd) - norm_cdf((lo - mu) / sd))
}

/// calc_invnorm: one value, or (lower, upper) for tail "center"
pub fn calc_invnorm(area: f64, sd: f64, mu: f64, tail: &str) -> Result<(f64, Option<f64>), CgErr> {
    if !(0.0 < area && area < 1.0) || sd <= 0.0 {
        return Err(MATH);
    }
    let target = match tail {
        "right" => 1.0 - area,
        "center" => 0.5 + area / 2.0,
        _ => area,
    };
    let (mut lo, mut hi) = (-40.0f64, 40.0f64);
    for _ in 0..200 {
        let mid = (lo + hi) / 2.0;
        if norm_cdf(mid) < target {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    let z = (lo + hi) / 2.0;
    if tail == "center" {
        return Ok((mu - z * sd, Some(mu + z * sd)));
    }
    Ok((mu + z * sd, None))
}

pub fn calc_binpd(x: f64, n: f64, p: f64) -> Result<f64, CgErr> {
    let cvt = |e: E| match e {
        E::Zero => CgErr::Py("ZeroDivisionError"),
        E::Val => CgErr::Py("ValueError"),
        _ => CgErr::Py("OverflowError"),
    };
    if !(0.0 <= p && p <= 1.0) || n != f_int(n).map_err(cvt)? || n < 0.0 {
        return Err(MATH);
    }
    if x != f_int(x).map_err(cvt)? || x < 0.0 || x > n {
        return Ok(0.0);
    }
    let c = comb(&big_of(n), &big_of(x)).map_err(cvt)?;
    Ok(c * fpow(p, x).map_err(cvt)? * fpow(1.0 - p, n - x).map_err(cvt)?)
}

pub fn calc_bincd(x: f64, n: f64, p: f64, lo: f64) -> Result<f64, CgErr> {
    let a = lo.max(0.0).trunc() as i64;
    let b = x.min(n).trunc() as i64;
    let mut v = vec![];
    for k in a..=b {
        v.push(calc_binpd(k as f64, n, p)?);
    }
    Ok(py_sum(v))
}
