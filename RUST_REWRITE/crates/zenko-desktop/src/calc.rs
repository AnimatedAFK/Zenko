//! zenko.py calc_eval + fmt_num: '15% of 240', '2^10', 'sqrt(81) + 3' -> a number. Python parses the cleaned-up text
//! with ast.parse and walks only maths nodes, so this is a small Python-expression parser with Python's number rules:
//! exact big integers, floats, the complex numbers a negative base to a fractional power gives, floor // and %, and
//! the math module's errors (a ZeroDivisionError is told apart: she has her own line for it).

use fancy_regex::Regex as FRegex;
use num_bigint::{BigInt, Sign};
use num_integer::Integer;
use num_traits::{One, Signed, ToPrimitive, Zero};
use regex::Regex;
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

#[derive(Clone, Debug, PartialEq)]
pub enum Num {
    Int(BigInt),
    Float(f64),
    Complex(f64, f64),
}

#[derive(Debug, PartialEq)]
pub enum CalcErr {
    ZeroDiv,
    Other,
}

type R<T> = Result<T, CalcErr>;
use CalcErr::{Other, ZeroDiv};

/// Python's float(int): correctly rounded (half to even), OverflowError when too big.
pub fn big_to_f64(b: &BigInt) -> R<f64> {
    scaled_to_f64(b, 0, false)
}

/// (q + sticky) * 2^exp rounded half-even to a float.
fn scaled_to_f64(q: &BigInt, exp: i64, sticky: bool) -> R<f64> {
    let neg = q.sign() == Sign::Minus;
    let m = q.abs();
    let n = m.bits() as i64;
    if n == 0 {
        return Ok(if neg { -0.0 } else { 0.0 });
    }
    let (mut top, e) = if n > 53 {
        let sh = (n - 53) as usize;
        let top: BigInt = &m >> sh;
        let rem: BigInt = &m - (&top << sh);
        let half: BigInt = BigInt::one() << (sh - 1);
        let mut t = top.to_u64().unwrap();
        if rem > half || (rem == half && (sticky || t & 1 == 1)) {
            t += 1;
        }
        (t, exp + sh as i64)
    } else {
        (m.to_u64().unwrap(), exp)
    };
    if top == 1u64 << 53 {
        top >>= 1;
        return finish(top, e + 1, neg);
    }
    finish(top, e, neg)
}

fn finish(top: u64, e: i64, neg: bool) -> R<f64> {
    let bits = 64 - top.leading_zeros() as i64;
    if bits + e > 1024 {
        return Err(Other);
    }
    let v = ldexp(top as f64, e);
    Ok(if neg { -v } else { v })
}

fn ldexp(x: f64, e: i64) -> f64 {
    let mut x = x;
    let mut e = e;
    while e > 1000 {
        x *= 2f64.powi(1000);
        e -= 1000;
    }
    while e < -1000 {
        x *= 2f64.powi(-1000);
        e += 1000;
    }
    x * 2f64.powi(e as i32)
}

/// int / int, correctly rounded like Python's long_true_divide.
fn int_truediv(a: &BigInt, b: &BigInt) -> R<f64> {
    if b.is_zero() {
        return Err(ZeroDiv);
    }
    if a.is_zero() {
        return Ok(if (a.sign() == Sign::Minus) != (b.sign() == Sign::Minus) { -0.0 } else { 0.0 });
    }
    let neg = (a.sign() == Sign::Minus) != (b.sign() == Sign::Minus);
    let (x, y) = (a.abs(), b.abs());
    let shift = 55 - (x.bits() as i64 - y.bits() as i64);
    let (xs, ys) = if shift >= 0 { (&x << shift as usize, y.clone()) } else { (x.clone(), &y << (-shift) as usize) };
    let (q, r) = xs.div_rem(&ys);
    let q2: BigInt = (q << 1usize) + if r.is_zero() { 0 } else { 1 };
    let v = scaled_to_f64(&q2, -shift - 1, false)?;
    Ok(if neg { -v } else { v })
}

fn as_f64(v: &Num) -> R<f64> {
    match v {
        Num::Int(i) => big_to_f64(i),
        Num::Float(f) => Ok(*f),
        Num::Complex(..) => Err(Other),
    }
}

fn as_c(v: &Num) -> R<(f64, f64)> {
    match v {
        Num::Complex(a, b) => Ok((*a, *b)),
        _ => Ok((as_f64(v)?, 0.0)),
    }
}

fn float_floordiv(vx: f64, wx: f64) -> R<f64> {
    if wx == 0.0 {
        return Err(ZeroDiv);
    }
    let (_, div) = float_divmod(vx, wx);
    Ok(div)
}

/// CPython _float_div_mod
fn float_divmod(vx: f64, wx: f64) -> (f64, f64) {
    let mut m = vx % wx;
    let mut div = (vx - m) / wx;
    if m != 0.0 {
        if (wx < 0.0) != (m < 0.0) {
            m += wx;
            div -= 1.0;
        }
    } else {
        m = 0f64.copysign(wx);
    }
    let fd = if div != 0.0 {
        let mut f = div.floor();
        if div - f > 0.5 {
            f += 1.0;
        }
        f
    } else {
        0f64.copysign(vx / wx)
    };
    (m, fd)
}

fn float_mod(vx: f64, wx: f64) -> R<f64> {
    if wx == 0.0 {
        return Err(ZeroDiv);
    }
    let mut m = vx % wx;
    if m != 0.0 {
        if (wx < 0.0) != (m < 0.0) {
            m += wx;
        }
    } else {
        m = 0f64.copysign(wx);
    }
    Ok(m)
}

/// CPython float_pow
fn float_pow(iv: f64, iw: f64) -> R<Num> {
    if iw == 0.0 {
        return Ok(Num::Float(1.0));
    }
    if iv.is_nan() {
        return Ok(Num::Float(if iw == 0.0 { 1.0 } else { iv }));
    }
    if iw.is_nan() {
        return Ok(Num::Float(if iv == 1.0 { 1.0 } else { iw }));
    }
    if iw.is_infinite() {
        let a = iv.abs();
        return Ok(Num::Float(if a == 1.0 {
            1.0
        } else if (iw > 0.0) == (a > 1.0) {
            f64::INFINITY
        } else {
            0.0
        }));
    }
    let w_odd = iw.floor() == iw && (iw % 2.0).abs() == 1.0;
    if iv.is_infinite() {
        return Ok(Num::Float(if iw > 0.0 {
            if w_odd { iv } else { iv.abs() }
        } else if w_odd {
            0f64.copysign(iv)
        } else {
            0.0
        }));
    }
    if iv == 0.0 {
        if iw < 0.0 {
            return Err(ZeroDiv);
        }
        return Ok(Num::Float(if w_odd { iv } else { 0.0 }));
    }
    let mut v = iv;
    let mut neg = false;
    if iv < 0.0 {
        if iw != iw.floor() {
            return c_pow_any((iv, 0.0), (iw, 0.0)).map(|(a, b)| Num::Complex(a, b));
        }
        v = -iv;
        neg = w_odd;
    }
    if v == 1.0 {
        return Ok(Num::Float(if neg { -1.0 } else { 1.0 }));
    }
    let r = v.powf(iw);
    if r.is_infinite() {
        return Err(Other);
    }
    Ok(Num::Float(if neg { -r } else { r }))
}

fn c_mul(a: (f64, f64), b: (f64, f64)) -> (f64, f64) {
    (a.0 * b.0 - a.1 * b.1, a.0 * b.1 + a.1 * b.0)
}

/// CPython _Py_c_quot
fn c_quot(a: (f64, f64), b: (f64, f64)) -> R<(f64, f64)> {
    let (ar, ai, br, bi) = (a.0, a.1, b.0, b.1);
    let abs_br = br.abs();
    let abs_bi = bi.abs();
    if abs_br >= abs_bi {
        if abs_br == 0.0 {
            return Err(ZeroDiv);
        }
        let ratio = bi / br;
        let denom = br + bi * ratio;
        Ok(((ar + ai * ratio) / denom, (ai - ar * ratio) / denom))
    } else if abs_bi >= abs_br {
        let ratio = br / bi;
        let denom = br * ratio + bi;
        Ok(((ar * ratio + ai) / denom, (ai * ratio - ar) / denom))
    } else {
        Ok((f64::NAN, f64::NAN))
    }
}

fn c_powu(x: (f64, f64), n: i64) -> (f64, f64) {
    let mut r = (1.0, 0.0);
    let mut p = x;
    let mut mask: i64 = 1;
    while mask > 0 && n >= mask {
        if n & mask != 0 {
            r = c_mul(r, p);
        }
        mask <<= 1;
        p = c_mul(p, p);
    }
    r
}

fn c_pow(a: (f64, f64), b: (f64, f64)) -> R<(f64, f64)> {
    if b.0 == 0.0 && b.1 == 0.0 {
        return Ok((1.0, 0.0));
    }
    if a.0 == 0.0 && a.1 == 0.0 {
        if b.1 != 0.0 || b.0 < 0.0 {
            return Err(ZeroDiv);
        }
        return Ok((0.0, 0.0));
    }
    let vabs = a.0.hypot(a.1);
    let mut len = vabs.powf(b.0);
    let at = a.1.atan2(a.0);
    let mut phase = at * b.0;
    if b.1 != 0.0 {
        len /= (at * b.1).exp();
        phase += b.1 * vabs.ln();
    }
    Ok((len * phase.cos(), len * phase.sin()))
}

/// complex_pow
fn c_pow_any(a: (f64, f64), b: (f64, f64)) -> R<(f64, f64)> {
    let p = if b.1 == 0.0 && b.0 == b.0.floor() && b.0.abs() <= 100.0 {
        let n = b.0 as i64;
        if n > 0 { c_powu(a, n) } else { c_quot((1.0, 0.0), c_powu(a, -n))? }
    } else {
        c_pow(a, b)?
    };
    if !p.0.is_finite() || !p.1.is_finite() {
        if a.0.is_finite() && a.1.is_finite() {
            return Err(Other);
        }
    }
    Ok(p)
}

#[derive(Clone, Copy, Debug)]
enum Op {
    Add,
    Sub,
    Mul,
    Div,
    FloorDiv,
    Mod,
    Pow,
}

fn binop(op: Op, a: Num, b: Num) -> R<Num> {
    use Num::*;
    if matches!(a, Complex(..)) || matches!(b, Complex(..)) {
        let (x, y) = (as_c(&a)?, as_c(&b)?);
        return Ok(match op {
            Op::Add => Complex(x.0 + y.0, x.1 + y.1),
            Op::Sub => Complex(x.0 - y.0, x.1 - y.1),
            Op::Mul => {
                let r = c_mul(x, y);
                Complex(r.0, r.1)
            }
            Op::Div => {
                let r = c_quot(x, y)?;
                Complex(r.0, r.1)
            }
            Op::Pow => {
                let r = c_pow_any(x, y)?;
                Complex(r.0, r.1)
            }
            Op::FloorDiv | Op::Mod => return Err(Other),
        });
    }
    if let (Int(x), Int(y)) = (&a, &b) {
        return Ok(match op {
            Op::Add => Int(x + y),
            Op::Sub => Int(x - y),
            Op::Mul => Int(x * y),
            Op::Div => Float(int_truediv(x, y)?),
            Op::FloorDiv => {
                if y.is_zero() {
                    return Err(ZeroDiv);
                }
                Int(x.div_floor(y))
            }
            Op::Mod => {
                if y.is_zero() {
                    return Err(ZeroDiv);
                }
                Int(x.mod_floor(y))
            }
            Op::Pow => {
                if y.is_negative() {
                    return float_pow(big_to_f64(x)?, big_to_f64(y)?);
                }
                Int(num_traits::pow(x.clone(), y.to_usize().ok_or(Other)?))
            }
        });
    }
    let (x, y) = (as_f64(&a)?, as_f64(&b)?);
    Ok(match op {
        Op::Add => Float(x + y),
        Op::Sub => Float(x - y),
        Op::Mul => Float(x * y),
        Op::Div => {
            if y == 0.0 {
                return Err(ZeroDiv);
            }
            Float(x / y)
        }
        Op::FloorDiv => Float(float_floordiv(x, y)?),
        Op::Mod => Float(float_mod(x, y)?),
        Op::Pow => return float_pow(x, y),
    })
}

fn num_abs(v: &Num) -> R<Num> {
    Ok(match v {
        Num::Int(i) => Num::Int(i.abs()),
        Num::Float(f) => Num::Float(f.abs()),
        Num::Complex(a, b) => {
            let h = a.hypot(*b);
            if h.is_infinite() && a.is_finite() && b.is_finite() {
                return Err(Other);
            }
            Num::Float(h)
        }
    })
}

fn gt200(v: &Num) -> R<bool> {
    Ok(match num_abs(v)? {
        Num::Int(i) => i > BigInt::from(200),
        Num::Float(f) => f > 200.0,
        Num::Complex(..) => false,
    })
}

/// math_1: a NaN out of a non-NaN is a ValueError, an inf out of a finite number an OverflowError
fn math1(x: &Num, f: fn(f64) -> f64) -> R<Num> {
    let x = as_f64(x)?;
    let r = f(x);
    if (r.is_nan() && !x.is_nan()) || (r.is_infinite() && x.is_finite()) {
        return Err(Other);
    }
    Ok(Num::Float(r))
}

/// math.log / math.log10 (huge ints go through frexp, like loghelper)
fn log_helper(x: &Num, f: fn(f64) -> f64) -> R<Num> {
    if let Num::Int(i) = x {
        if !i.is_positive() {
            return Err(Other);
        }
        if let Ok(v) = big_to_f64(i) {
            return Ok(Num::Float(f(v)));
        }
        let e = i.bits() as i64;
        let sh = e - 60;
        let top = (i >> sh as usize).to_u64().unwrap() as f64 / 2f64.powi(60);
        return Ok(Num::Float(f(top) + f(2.0) * e as f64));
    }
    let v = as_f64(x)?;
    if v <= 0.0 {
        return Err(Other);
    }
    if v.is_infinite() {
        return Ok(Num::Float(v));
    }
    Ok(Num::Float(f(v)))
}

fn f_to_int(f: f64) -> R<BigInt> {
    if !f.is_finite() {
        return Err(Other);
    }
    let bits = f.to_bits();
    let exp = ((bits >> 52) & 0x7ff) as i64;
    let man = if exp == 0 { (bits & ((1 << 52) - 1)) << 1 } else { (bits & ((1 << 52) - 1)) | (1 << 52) };
    let e = exp - 1075;
    let m = BigInt::from(man);
    let v = if e >= 0 { m << e as usize } else { m >> (-e) as usize };
    Ok(if f < 0.0 { -v } else { v })
}

fn call(name: &str, v: &Num) -> R<Num> {
    match name {
        "sqrt" => math1(v, f64::sqrt),
        "sin" => math1(&Num::Float(as_f64(v)? * (std::f64::consts::PI / 180.0)), f64::sin),
        "cos" => math1(&Num::Float(as_f64(v)? * (std::f64::consts::PI / 180.0)), f64::cos),
        "tan" => math1(&Num::Float(as_f64(v)? * (std::f64::consts::PI / 180.0)), f64::tan),
        "log" => log_helper(v, f64::log10),
        "ln" => log_helper(v, f64::ln),
        "abs" => num_abs(v),
        "round" => match v {
            Num::Int(_) => Ok(v.clone()),
            Num::Float(f) => {
                let r = f.round_ties_even();
                Ok(Num::Int(f_to_int(r)?))
            }
            Num::Complex(..) => Err(Other),
        },
        "exp" => math1(v, f64::exp),
        "floor" | "ceil" => match v {
            Num::Int(_) => Ok(v.clone()),
            Num::Float(f) => Ok(Num::Int(f_to_int(if name == "floor" { f.floor() } else { f.ceil() })?)),
            Num::Complex(..) => Err(Other),
        },
        _ => Err(Other),
    }
}

// ---- the parser: Python's expression grammar (what ast.parse would accept, as far as maths goes) ----
#[derive(Debug, Clone, PartialEq)]
enum Tok {
    Num(Num),
    Imag,
    Name(String),
    Op(&'static str),
}

fn lex(s: &str) -> R<Vec<Tok>> {
    let c: Vec<char> = s.chars().collect();
    let mut out = vec![];
    let mut i = 0;
    let digits = |c: &[char], mut i: usize, hex: bool| -> R<usize> {
        // digitpart: digit (["_"] digit)*
        let ok = |ch: char| if hex { ch.is_ascii_hexdigit() } else { ch.is_ascii_digit() };
        if i >= c.len() || !ok(c[i]) {
            return Err(Other);
        }
        while i < c.len() {
            if ok(c[i]) {
                i += 1;
            } else if c[i] == '_' && i + 1 < c.len() && ok(c[i + 1]) {
                i += 2;
            } else {
                break;
            }
        }
        Ok(i)
    };
    while i < c.len() {
        let ch = c[i];
        if ch == ' ' || ch == '\t' || ch == '\x0c' {
            i += 1;
            continue;
        }
        if ch.is_ascii_digit() || (ch == '.' && i + 1 < c.len() && c[i + 1].is_ascii_digit()) {
            let st = i;
            if ch == '0' && i + 1 < c.len() && "xXoObB".contains(c[i + 1]) {
                let base = match c[i + 1].to_ascii_lowercase() {
                    'x' => 16,
                    'o' => 8,
                    _ => 2,
                };
                let mut j = i + 2;
                if j < c.len() && c[j] == '_' {
                    j += 1;
                }
                let k = digits(&c, j, base == 16)?;
                let txt: String = c[j..k].iter().filter(|x| **x != '_').collect();
                if base != 16 && txt.chars().any(|x| x.to_digit(base).is_none()) {
                    return Err(Other);
                }
                let v = BigInt::parse_bytes(txt.as_bytes(), base).ok_or(Other)?;
                i = k;
                if i < c.len() && (c[i].is_alphanumeric() || c[i] == '_') {
                    return Err(Other);
                }
                out.push(Tok::Num(Num::Int(v)));
                continue;
            }
            let mut j = i;
            let mut is_float = false;
            if ch != '.' {
                j = digits(&c, j, false)?;
            }
            if j < c.len() && c[j] == '.' {
                is_float = true;
                j += 1;
                if j < c.len() && c[j].is_ascii_digit() {
                    j = digits(&c, j, false)?;
                }
            }
            if j < c.len() && (c[j] == 'e' || c[j] == 'E') {
                let mut k = j + 1;
                if k < c.len() && (c[k] == '+' || c[k] == '-') {
                    k += 1;
                }
                if k < c.len() && c[k].is_ascii_digit() {
                    j = digits(&c, k, false)?;
                    is_float = true;
                } else {
                    return Err(Other);
                }
            }
            let txt: String = c[st..j].iter().filter(|x| **x != '_').collect();
            i = j;
            if i < c.len() && (c[i] == 'j' || c[i] == 'J') {
                i += 1;
                if i < c.len() && (c[i].is_alphanumeric() || c[i] == '_') {
                    return Err(Other);
                }
                out.push(Tok::Imag);
                continue;
            }
            if i < c.len() && (c[i].is_alphanumeric() || c[i] == '_') {
                return Err(Other);
            }
            if is_float {
                out.push(Tok::Num(Num::Float(txt.parse::<f64>().map_err(|_| Other)?)));
            } else {
                if txt.len() > 1 && txt.starts_with('0') && txt.chars().any(|x| x != '0') {
                    return Err(Other);
                }
                out.push(Tok::Num(Num::Int(txt.parse::<BigInt>().map_err(|_| Other)?)));
            }
            continue;
        }
        if ch.is_alphabetic() || ch == '_' {
            let st = i;
            while i < c.len() && (c[i].is_alphanumeric() || c[i] == '_') {
                i += 1;
            }
            out.push(Tok::Name(c[st..i].iter().collect()));
            continue;
        }
        let rest: String = c[i..c.len().min(i + 3)].iter().collect();
        const OPS: [&str; 30] = ["**=", "//=", ">>=", "<<=", "...", "**", "//", "<=", ">=", "==", "!=", "->", ":=", "<<", ">>",
                                 "+", "-", "*", "/", "%", "(", ")", ",", "<", ">", "&", "|", "^", "~", "@"];
        let op = OPS.iter().find(|o| rest.starts_with(**o));
        match op {
            Some(o) => {
                out.push(Tok::Op(o));
                i += o.chars().count();
            }
            None => {
                // other punctuation: brackets, dots, quotes... nothing maths can use
                if "[]{}.:;=!'\"".contains(ch) || ch.is_ascii_punctuation() {
                    out.push(Tok::Op("?"));
                    i += 1;
                } else {
                    return Err(Other);
                }
            }
        }
    }
    Ok(out)
}

/// The tree, kept only as far as evaluation needs it. `Bad` is any valid-looking Python we don't do maths with;
/// it fails only when it is reached, like calc_eval's ev().
#[derive(Debug)]
enum Node {
    Lit(Num),
    Name(String),
    Bin(Op, Box<Node>, Box<Node>),
    Neg(Box<Node>),
    Pos(Box<Node>),
    Call(String, Vec<Node>),
    Bad,
}

struct P {
    t: Vec<Tok>,
    i: usize,
}

impl P {
    fn peek(&self) -> Option<&Tok> {
        self.t.get(self.i)
    }
    fn is_op(&self, o: &str) -> bool {
        matches!(self.peek(), Some(Tok::Op(x)) if *x == o)
    }
    fn arith(&mut self) -> R<Node> {
        let mut l = self.term()?;
        loop {
            let op = if self.is_op("+") { Op::Add } else if self.is_op("-") { Op::Sub } else { break };
            self.i += 1;
            let r = self.term()?;
            l = Node::Bin(op, Box::new(l), Box::new(r));
        }
        Ok(l)
    }
    fn term(&mut self) -> R<Node> {
        let mut l = self.factor()?;
        loop {
            let op = if self.is_op("*") {
                Some(Op::Mul)
            } else if self.is_op("/") {
                Some(Op::Div)
            } else if self.is_op("//") {
                Some(Op::FloorDiv)
            } else if self.is_op("%") {
                Some(Op::Mod)
            } else if self.is_op("@") {
                None
            } else {
                break;
            };
            self.i += 1;
            let r = self.factor()?;
            l = match op {
                Some(op) => Node::Bin(op, Box::new(l), Box::new(r)),
                None => Node::Bad,
            };
        }
        Ok(l)
    }
    fn factor(&mut self) -> R<Node> {
        if self.is_op("-") || self.is_op("+") || self.is_op("~") {
            let neg = self.is_op("-");
            let inv = self.is_op("~");
            self.i += 1;
            let f = self.factor()?;
            return Ok(if inv { Node::Bad } else if neg { Node::Neg(Box::new(f)) } else { Node::Pos(Box::new(f)) });
        }
        self.power()
    }
    fn power(&mut self) -> R<Node> {
        let a = self.primary()?;
        if self.is_op("**") {
            self.i += 1;
            let b = self.factor()?;
            return Ok(Node::Bin(Op::Pow, Box::new(a), Box::new(b)));
        }
        Ok(a)
    }
    fn primary(&mut self) -> R<Node> {
        let mut a = self.atom()?;
        while self.is_op("(") {
            self.i += 1;
            let mut args = vec![];
            if !self.is_op(")") {
                loop {
                    args.push(self.arith()?);
                    if self.is_op(",") {
                        self.i += 1;
                        if self.is_op(")") {
                            break;
                        }
                        continue;
                    }
                    break;
                }
            }
            if !self.is_op(")") {
                return Err(Other);
            }
            self.i += 1;
            a = match a {
                Node::Name(n) => Node::Call(n, args),
                _ => Node::Bad,
            };
        }
        Ok(a)
    }
    fn atom(&mut self) -> R<Node> {
        match self.peek().cloned() {
            Some(Tok::Num(n)) => {
                self.i += 1;
                Ok(Node::Lit(n))
            }
            Some(Tok::Imag) => {
                self.i += 1;
                Ok(Node::Bad)
            }
            Some(Tok::Name(n)) => {
                self.i += 1;
                const KW: [&str; 35] = ["False", "None", "True", "and", "as", "assert", "async", "await", "break", "class",
                                        "continue", "def", "del", "elif", "else", "except", "finally", "for", "from",
                                        "global", "if", "import", "in", "is", "lambda", "nonlocal", "not", "or", "pass",
                                        "raise", "return", "try", "while", "with", "yield"];
                match n.as_str() {
                    "True" => Ok(Node::Lit(Num::Int(BigInt::one()))),
                    "False" => Ok(Node::Lit(Num::Int(BigInt::zero()))),
                    "None" => Ok(Node::Bad),
                    k if KW.contains(&k) => Err(Other),
                    _ => Ok(Node::Name(n)),
                }
            }
            Some(Tok::Op("(")) => {
                self.i += 1;
                if self.is_op(")") {
                    self.i += 1;
                    return Ok(Node::Bad); // ()
                }
                let e = self.arith()?;
                let mut tuple = false;
                while self.is_op(",") {
                    tuple = true;
                    self.i += 1;
                    if self.is_op(")") {
                        break;
                    }
                    self.arith()?;
                }
                if !self.is_op(")") {
                    return Err(Other);
                }
                self.i += 1;
                Ok(if tuple { Node::Bad } else { e })
            }
            _ => Err(Other),
        }
    }
}

fn ev(n: &Node) -> R<Num> {
    match n {
        Node::Lit(v) => Ok(v.clone()),
        Node::Name(s) => match s.as_str() {
            "pi" => Ok(Num::Float(std::f64::consts::PI)),
            "e" => Ok(Num::Float(std::f64::consts::E)),
            _ => Err(Other),
        },
        Node::Bin(op, a, b) => {
            let (x, y) = (ev(a)?, ev(b)?);
            if matches!(op, Op::Pow) && gt200(&y)? {
                return Err(Other);
            }
            binop(*op, x, y)
        }
        Node::Neg(a) => Ok(match ev(a)? {
            Num::Int(i) => Num::Int(-i),
            Num::Float(f) => Num::Float(-f),
            Num::Complex(x, y) => Num::Complex(-x, -y),
        }),
        Node::Pos(a) => ev(a),
        Node::Call(f, args) => {
            const FNS: [&str; 11] = ["sqrt", "sin", "cos", "tan", "log", "ln", "abs", "round", "exp", "floor", "ceil"];
            if !FNS.contains(&f.as_str()) || args.len() != 1 {
                return Err(Other);
            }
            call(f, &ev(&args[0])?)
        }
        Node::Bad => Err(Other),
    }
}

/// calc_eval(expr)
pub fn calc_eval(expr: &str) -> R<Num> {
    let s = expr.to_lowercase();
    let s = s.trim().trim_end_matches(['=', '?', '.', ' ']);
    let s = s.replace('×', "*").replace('÷', "/").replace('−', "-").replace(',', "");
    let s = rx!(r"\b(?:what is|what's|whats|calculate|calc|compute|solve)\b").replace_all(&s, " ").into_owned();
    let s = rx!(r"(\d+(?:\.\d+)?)\s*%\s*off\s*(\d+(?:\.\d+)?)").replace_all(&s, "($2*(1-$1/100))").into_owned();
    let s = rx!(r"(\d+(?:\.\d+)?)\s*%\s*of\s*").replace_all(&s, "($1/100)*").into_owned();
    let s = rx!(r"(\d+(?:\.\d+)?)\s*%").replace_all(&s, "($1/100)").into_owned();
    let mut s = s;
    for (w, o) in [(rx!(r"\bplus\b"), "+"), (rx!(r"\bminus\b"), "-"), (rx!(r"\btimes\b"), "*"), (rx!(r"\bmultiplied by\b"), "*"),
                   (rx!(r"\bdivided by\b"), "/"), (rx!(r"\bover\b"), "/"), (rx!(r"\bsquared\b"), "**2"), (rx!(r"\bcubed\b"), "**3"),
                   (rx!(r"\bto the power of\b"), "**"), (rx!(r"\bmod\b"), "%"), (rx!(r"\bsquare root of\b"), "sqrt "),
                   (rx!(r"\broot of\b"), "sqrt ")] {
        s = w.replace_all(&s, o).into_owned();
    }
    let s = frx!(r"(?<=[\d)])\s*x\s*(?=[\d(])").replace_all(&s, "*").into_owned();
    let s = s.replace('^', "**");
    let s = rx!(r"\bsqrt\s+(\d+(?:\.\d+)?)").replace_all(&s, "sqrt($1)").into_owned();
    let s = s.trim();
    if s.is_empty() {
        return Err(Other);
    }
    let t = lex(s)?;
    let mut p = P { t, i: 0 };
    let n = p.arith()?;
    if p.i != p.t.len() {
        // a comparison, boolean, if-else... parses in Python but isn't maths; anything else is a SyntaxError
        return Err(Other);
    }
    ev(&n)
}

fn group(int_digits: &str) -> String {
    let (sign, d) = match int_digits.strip_prefix('-') {
        Some(r) => ("-", r),
        None => ("", int_digits),
    };
    let mut out = String::new();
    for (k, ch) in d.chars().enumerate() {
        if k > 0 && (d.len() - k) % 3 == 0 {
            out.push(',');
        }
        out.push(ch);
    }
    format!("{sign}{out}")
}

/// Python's repr(float) digits (shortest round-trip), with "," grouping: format(v, ",") / complex parts.
pub fn py_repr_grouped(v: f64, dot0: bool) -> String {
    if v.is_nan() {
        return "nan".into();
    }
    if v.is_infinite() {
        return if v > 0.0 { "inf".into() } else { "-inf".into() };
    }
    let e = format!("{v:e}");
    let (mant, ex) = e.split_once('e').unwrap();
    let ex: i32 = ex.parse().unwrap();
    let neg = mant.starts_with('-');
    let digits: String = mant.chars().filter(|c| c.is_ascii_digit()).collect();
    let sign = if neg { "-" } else { "" };
    if !(-4..16).contains(&ex) {
        let m = if digits.len() > 1 { format!("{}.{}", &digits[..1], &digits[1..]) } else { digits.clone() };
        return format!("{sign}{m}e{}{:02}", if ex < 0 { '-' } else { '+' }, ex.abs());
    }
    let (ip, fp) = if ex >= 0 {
        let k = ex as usize + 1;
        if digits.len() <= k {
            (format!("{digits}{}", "0".repeat(k - digits.len())), String::new())
        } else {
            (digits[..k].to_string(), digits[k..].to_string())
        }
    } else {
        ("0".to_string(), format!("{}{digits}", "0".repeat((-ex - 1) as usize)))
    };
    let fp = if fp.is_empty() && dot0 { "0".to_string() } else { fp };
    let ip = group(&ip);
    if fp.is_empty() { format!("{sign}{ip}") } else { format!("{sign}{ip}.{fp}") }
}

/// fmt_num(v)
pub fn fmt_num(v: &Num) -> String {
    match v {
        Num::Int(i) => group(&i.to_string()),
        Num::Float(f) => {
            let f = *f;
            if !f.is_finite() {
                return py_repr_grouped(f, true);
            }
            let r = f.round_ties_even();
            if (f - r).abs() < 1e-9 && f.abs() < 1e15 {
                return group(&format!("{}", r as i64));
            }
            let s = format!("{f:.6}");
            let (ip, fp) = s.split_once('.').unwrap();
            let s = format!("{}.{fp}", group(ip));
            s.trim_end_matches('0').trim_end_matches('.').to_string()
        }
        Num::Complex(a, b) => {
            let im = py_repr_grouped(*b, false);
            if *a == 0.0 && a.is_sign_positive() {
                return format!("{im}j");
            }
            let re = py_repr_grouped(*a, false);
            let im = if im.starts_with('-') { im } else { format!("+{im}") };
            format!("({re}{im}j)")
        }
    }
}
