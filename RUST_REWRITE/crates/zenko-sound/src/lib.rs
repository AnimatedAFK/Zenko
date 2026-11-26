//! Zenko's sound kit, ported line by line from zenko.py (`_koto`, `_bell`, `_tap`, `_drum`, `_pop`, `_suzu`, `_furin`,
//! `_arp`, `_sound_parts`, `extra_sound_parts` (zenko6 + zenko14), `_vox`, `_koto_song`, `_wav_bytes`).
//! The float maths runs in the same order as Python and the noise comes from a copy of Python's own random generator,
//! so the .wav bytes are the same as the ones zenko.py writes (checked by tests/sounds.rs).

pub mod mt;
use mt::PyRandom;
use std::f64::consts::PI;

pub const SR: usize = 22050;
const SRF: f64 = SR as f64;

/// (start seconds, samples, gain)
pub type Part = (f64, Vec<f64>, f64);

fn note(n: &str) -> f64 {
    match n {
        "D4" => 293.66,
        "E4" => 329.63,
        "G4" => 392.0,
        "A4" => 440.0,
        "Bb4" => 466.16,
        "B4" => 493.88,
        "Eb4" => 311.13,
        "D5" => 587.33,
        "E5" => 659.26,
        "G5" => 783.99,
        "A5" => 880.0,
        "B5" => 987.77,
        "D6" => 1174.66,
        "E6" => 1318.51,
        "A6" => 1760.0,
        "Eb5" => 622.25,
        "Bb5" => 932.33,
        _ => {
            // zenko14 `_note`: equal temperament from A4 for anything not in the table
            let names = ["C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B"];
            let (nm, oc) = n.split_at(n.len() - 1);
            let idx = names.iter().position(|x| *x == nm).unwrap_or(9) as i64;
            let oct: i64 = oc.parse().unwrap_or(4);
            440.0 * 2f64.powf((idx - 9) as f64 / 12.0 + oct as f64 - 4.0)
        }
    }
}

fn trunc(x: f64) -> usize {
    x as usize
}

pub fn koto(freq: f64, dur: f64, bright: f64, seed: u64) -> Vec<f64> {
    let mut rnd = PyRandom::new(seed);
    let n = 2.max((SRF / freq) as i64) as usize;
    let mut buf: Vec<f64> = (0..n).map(|_| rnd.uniform(-1.0, 1.0)).collect();
    let passes = 2 - (bright * 2.0) as i64;
    for _ in 0..passes.max(0) {
        buf = (0..n).map(|i| (buf[i] + buf[(i + n - 1) % n]) * 0.5).collect();
    }
    let damp = if freq < 700.0 { 0.996 } else { 0.993 };
    let cnt = trunc(dur * SRF);
    let mut out = Vec::with_capacity(cnt);
    let mut i = 0;
    for _ in 0..cnt {
        let a = buf[i];
        let b = buf[(i + 1) % n];
        buf[i] = (a + b) * 0.5 * damp;
        out.push(a);
        i = (i + 1) % n;
    }
    out
}

pub fn bell(freq: f64, dur: f64, partials: &[(f64, f64)], decay: f64) -> Vec<f64> {
    let mut out = vec![0.0f64; trunc(dur * SRF)];
    for (k, &(mul, amp)) in partials.iter().enumerate() {
        let f = freq * mul;
        let d = decay * (1.0 - 0.12 * k as f64);
        let w = 2.0 * PI * f / SRF;
        let kf = k as f64;
        let sd = SRF * d;
        for (i, o) in out.iter_mut().enumerate() {
            let fi = i as f64;
            *o += amp * (w * fi + kf).sin() * (-fi / sd).exp();
        }
    }
    let att = trunc(0.002 * SRF);
    for i in 0..att.min(out.len()) {
        out[i] *= i as f64 / att as f64;
    }
    out
}

pub fn tap(freq: f64, dur: f64) -> Vec<f64> {
    let mut rnd = PyRandom::new(7);
    (0..trunc(dur * SRF))
        .map(|i| {
            let t = i as f64 / SRF;
            let e = (-t / 0.012).exp();
            let tone = 0.75 * (2.0 * PI * freq * t).sin() + 0.35 * (2.0 * PI * freq * 1.58 * t).sin();
            e * tone + rnd.uniform(-1.0, 1.0) * (-t / 0.003).exp() * 0.4
        })
        .collect()
}

pub fn drum(freq: f64, dur: f64) -> Vec<f64> {
    let mut ph = 0.0;
    (0..trunc(dur * SRF))
        .map(|i| {
            let t = i as f64 / SRF;
            let f = freq * (1.0 + 0.8 * (-t / 0.03).exp());
            ph += 2.0 * PI * f / SRF;
            ph.sin() * (-t / 0.11).exp()
        })
        .collect()
}

pub fn pop(f0: f64, f1: f64, dur: f64) -> Vec<f64> {
    let mut ph = 0.0;
    let len = dur * SRF;
    (0..trunc(len))
        .map(|i| {
            let t = i as f64 / len;
            ph += 2.0 * PI * (f0 + (f1 - f0) * t) / SRF;
            ph.sin() * (PI * t).sin()
        })
        .collect()
}

pub fn suzu(start: f64, hits: usize, seed: u64) -> Vec<Part> {
    let mut rnd = PyRandom::new(seed);
    let mut parts = Vec::new();
    let mut t = start;
    for _h in 0..hits {
        let f = 2350.0 * rnd.uniform(0.97, 1.03);
        let b = bell(f, 0.32, &[(1.0, 0.6), (2.32, 0.3), (4.25, 0.18), (6.63, 0.1)], 0.07);
        let g = rnd.uniform(0.35, 0.6);
        parts.push((t, b, g));
        t += rnd.uniform(0.045, 0.075);
    }
    parts
}

pub fn furin(start: f64, f: f64, gain: f64) -> Vec<Part> {
    vec![
        (start, bell(f, 1.3, &[(1.0, 0.55), (2.76, 0.25), (5.40, 0.12)], 0.45), gain),
        (start + 0.11, bell(f * 1.335, 1.0, &[(1.0, 0.4), (2.76, 0.18)], 0.35), gain * 0.6),
    ]
}

fn arp(notes: &[&str], start: f64, step: f64, dur: f64, gain: f64) -> Vec<Part> {
    notes
        .iter()
        .enumerate()
        .map(|(i, n)| (start + i as f64 * step, koto(note(n), dur, 0.55, i as u64 + 3), gain))
        .collect()
}

fn cat(mut a: Vec<Part>, b: Vec<Part>) -> Vec<Part> {
    a.extend(b);
    a
}

// ---- her voice (zenko6 VOX + the zenko14 additions) ----------------------------------------------------------------

type Syl = (&'static str, &'static str, f64, f64, f64);

fn vow(v: &str) -> Option<[f64; 3]> {
    Some(match v {
        "a" => [850.0, 1300.0, 2900.0],
        "e" => [560.0, 2250.0, 3000.0],
        "i" => [380.0, 2800.0, 3500.0],
        "o" => [560.0, 950.0, 2800.0],
        "u" => [420.0, 1350.0, 2600.0],
        "n" => [290.0, 1800.0, 2700.0],
        "m" => [280.0, 1150.0, 2500.0],
        _ => return None,
    })
}

/// The VOX table in its final order (zenko6 keys, then zenko14's `VOX.update`).
pub fn vox_table() -> Vec<(&'static str, Vec<Syl>)> {
    vec![
        ("v_kon", vec![("k", "o", 0.12, 1.0, 1.1), ("", "n", 0.06, 1.1, 1.02), ("-", "", 0.07, 1.0, 1.0),
                       ("k", "o", 0.12, 1.12, 1.24), ("", "n", 0.12, 1.22, 0.96)]),
        ("v_hehe", vec![("", "e", 0.08, 1.05, 1.1), ("h", "e", 0.08, 1.14, 1.04), ("h", "e", 0.16, 1.08, 0.9)]),
        ("v_kyu", vec![("k", "i", 0.035, 1.18, 1.25), ("", "u", 0.30, 1.25, 1.42)]),
        ("v_yatta", vec![("y", "a", 0.11, 1.0, 1.1), ("t", "a", 0.22, 1.28, 1.06)]),
        ("v_mou", vec![("m", "o", 0.15, 1.02, 0.92), ("", "u", 0.2, 0.92, 1.18)]),
        ("v_uu", vec![("", "u", 0.34, 0.98, 0.8)]),
        ("v_nom", vec![("", "n", 0.05, 1.0, 1.0), ("", "o", 0.08, 1.05, 1.0), ("", "m", 0.06, 1.0, 0.95),
                       ("-", "", 0.05, 1.0, 1.0), ("", "n", 0.05, 1.05, 1.05), ("", "o", 0.08, 1.1, 1.0),
                       ("", "m", 0.08, 1.0, 0.92)]),
        ("v_kya", vec![("k", "i", 0.03, 1.32, 1.42), ("", "a", 0.18, 1.45, 1.22)]),
        ("v_fuwa", vec![("h", "u", 0.1, 1.0, 0.96), ("", "a", 0.42, 0.98, 0.78)]),
        ("v_nade", vec![("n", "a", 0.1, 1.0, 1.05), ("d", "e", 0.1, 1.08, 1.0), ("n", "a", 0.1, 1.04, 1.08),
                        ("d", "e", 0.2, 1.1, 0.92)]),
        ("v_ureshii", vec![("", "u", 0.07, 1.0, 1.05), ("r", "e", 0.08, 1.08, 1.1), ("sh", "i", 0.26, 1.2, 1.32)]),
        ("v_kusugu", vec![("k", "u", 0.06, 1.1, 1.15), ("s", "u", 0.06, 1.2, 1.2), ("g", "u", 0.06, 1.25, 1.25),
                          ("t", "a", 0.08, 1.3, 1.32), ("", "i", 0.24, 1.35, 1.12)]),
        ("v_motto", vec![("m", "o", 0.12, 1.0, 1.06), ("t", "o", 0.32, 1.1, 0.86)]),
        ("v_waai", vec![("w", "a", 0.28, 1.05, 1.3), ("", "i", 0.12, 1.3, 1.22)]),
        ("v_fufu", vec![("h", "u", 0.07, 1.1, 1.12), ("-", "", 0.04, 1.0, 1.0), ("h", "u", 0.12, 1.16, 1.0)]),
        ("v_hai", vec![("h", "a", 0.2, 1.06, 1.2), ("", "i", 0.14, 1.2, 1.1)]),
        ("v_sugoi", vec![("s", "u", 0.07, 1.0, 1.05), ("g", "o", 0.12, 1.1, 1.2), ("", "i", 0.2, 1.25, 1.1)]),
        ("v_nee", vec![("n", "e", 0.12, 1.0, 1.1), ("", "e", 0.2, 1.1, 1.18)]),
        ("v_ganbare", vec![("g", "a", 0.07, 1.0, 1.02), ("n", "n", 0.05, 1.02, 1.04), ("b", "a", 0.08, 1.06, 1.1),
                           ("r", "e", 0.22, 1.16, 1.0)]),
        ("v_ehe", vec![("", "e", 0.06, 1.1, 1.12), ("h", "e", 0.18, 1.2, 1.05)]),
    ]
}

pub fn vox(sylls: &[Syl], base: f64, seed: u64) -> Vec<f64> {
    let mut rnd = PyRandom::new(seed);
    let mut out: Vec<f64> = Vec::new();
    let mut ph = 0.0f64;
    let mut lp = 0.0f64;
    let va = vow("a").unwrap();
    for &(cons, vw, dur, p0, p1) in sylls {
        if cons == "-" {
            out.extend(std::iter::repeat_n(0.0, trunc(dur * SRF)));
            continue;
        }
        if cons == "k" || cons == "t" {
            out.extend(std::iter::repeat_n(0.0, trunc(0.016 * SRF)));
            for i in 0..trunc(0.018 * SRF) {
                out.push(rnd.uniform(-1.0, 1.0) * 0.35 * (-(i as f64) / (0.005 * SRF)).exp());
            }
        } else if cons == "h" {
            let m = 0.05 * SRF;
            for i in 0..trunc(m) {
                lp = lp * 0.6 + rnd.uniform(-1.0, 1.0) * 0.4;
                out.push(lp * 0.22 * (PI * i as f64 / m).sin());
            }
        }
        let target = vow(vw).unwrap_or(va);
        let mut f = target;
        let n = trunc(dur * SRF);
        let mut amps: Vec<f64> = Vec::new();
        let blk = 96;
        for i in 0..n {
            let fi = i as f64;
            let t = fi / n.max(1) as f64;
            let vib = 1.0 + 0.028 * (2.0 * PI * 6.2 * fi / SRF).sin() * (t * 3.0).min(1.0);
            let f0 = base * (p0 + (p1 - p0) * t) * vib;
            if cons == "y" && t < 0.25 {
                let vi = vow("i").unwrap();
                f = [0, 1, 2].map(|j| vi[j] + (target[j] - vi[j]) * (t / 0.25));
            }
            ph += 2.0 * PI * f0 / SRF;
            if i % blk == 0 {
                amps.clear();
                let mut k = 1i64;
                while (k as f64) * f0 < 5200.0 && k <= 12 {
                    let fk = k as f64 * f0;
                    let a = 1.0 / (1.0 + sq((fk - f[0]) / 130.0)) + 0.65 / (1.0 + sq((fk - f[1]) / 170.0))
                        + 0.28 / (1.0 + sq((fk - f[2]) / 240.0));
                    amps.push(a / (k as f64).powf(0.35));
                    k += 1;
                }
                if vw == "n" || vw == "m" {
                    for a in amps.iter_mut() {
                        *a *= 0.55;
                    }
                }
            }
            let mut s = 0.0;
            for (k, a) in amps.iter().enumerate() {
                s += a * ((k as f64 + 1.0) * ph).sin();
            }
            s += rnd.uniform(-1.0, 1.0) * 0.03;
            let env = (fi / (0.012 * SRF)).min(1.0) * (((n - i) as f64) / (0.035 * SRF)).min(1.0);
            out.push(s * env);
        }
    }
    let peak = out.iter().fold(1e-6f64, |m, v| m.max(v.abs()));
    out.iter().map(|v| v / peak).collect()
}

fn sq(x: f64) -> f64 {
    x * x
}

fn koto_song() -> Vec<Part> {
    let notes: [(&str, i32); 21] = [
        ("D5", 1), ("E5", 1), ("G5", 1), ("A5", 2), ("G5", 1), ("E5", 1), ("D5", 2),
        ("E5", 1), ("G5", 1), ("A5", 1), ("B5", 2), ("A5", 1), ("G5", 1), ("E5", 2),
        ("A5", 1), ("G5", 1), ("E5", 1), ("D5", 1), ("E5", 1), ("G5", 1), ("D5", 3),
    ];
    let (mut parts, mut t, beat) = (Vec::new(), 0.0f64, 0.24f64);
    for (i, &(nm, ln)) in notes.iter().enumerate() {
        parts.push((t, koto(note(nm), 0.55 + 0.2 * ln as f64, 0.6, i as u64 + 2), 0.55));
        if ln >= 2 {
            parts.push((t, koto(note(nm) / 2.0, 0.8, 0.4, i as u64 + 40), 0.25));
        }
        t += beat * ln as f64;
    }
    cat(parts, suzu(t + 0.05, 3, 4))
}

/// zenko6 `extra_sound_parts` (her voice and the play sounds).
fn extra_base(kind: &str, pitch: f64) -> Option<Vec<Part>> {
    if let Some((_, sy)) = vox_table().into_iter().find(|(k, _)| *k == kind) {
        return Some(vec![(0.0, vox(&sy, 640.0 * pitch, kind.chars().count() as u64), 0.8)]);
    }
    Some(match kind {
        "song" => koto_song(),
        "munch" => (0..6).map(|i| (i as f64 * 0.17, tap((900 + 140 * (i % 2)) as f64, 0.05), 0.5)).collect(),
        "chomp" => {
            let mut p = vec![(0.0, pop(300.0, 1500.0, 0.12), 0.6)];
            p.extend((0..5).map(|i| (0.16 + i as f64 * 0.12, drum((140 + 30 * (i % 2)) as f64, 0.12), 0.7)));
            cat(p, suzu(0.82, 3, 8))
        }
        "boop" => vec![(0.0, pop(900.0, 1800.0, 0.06), 0.5)],
        "giggle" => arp(&["A5", "B5", "A5", "D6"], 0.0, 0.07, 0.3, 0.4),
        "gift" => cat(suzu(0.0, 4, 2), arp(&["D5", "G5", "B5", "D6"], 0.25, 0.08, 0.6, 0.5)),
        "omikuji" => cat(
            (0..9).map(|i| (i as f64 * 0.07, tap((1300 + 200 * (i % 3)) as f64, 0.04), 0.45)).collect(),
            furin(0.7, 2349.0, 0.45),
        ),
        "yawn" => vec![(0.0, koto(note("D5"), 0.6, 0.3, 1), 0.25)],
        _ => return None,
    })
}

/// zenko14 `extra_sound_parts` (the interaction sound pool), falling back to zenko6's.
fn extra(kind: &str, pitch: f64) -> Option<Vec<Part>> {
    Some(match kind {
        "i_koto_up" => ["D5", "E5", "A5"].iter().enumerate()
            .map(|(i, n)| (i as f64 * 0.07, koto(note(n), 0.5, 0.7, i as u64 + 2), 0.45)).collect(),
        "i_koto_down" => ["B5", "A5", "E5"].iter().enumerate()
            .map(|(i, n)| (i as f64 * 0.08, koto(note(n), 0.5, 0.6, i as u64 + 5), 0.45)).collect(),
        "i_koto_trill" => (0..6)
            .map(|i| (i as f64 * 0.045, koto(note(["A5", "B5"][i % 2]), 0.25, 0.8, i as u64), 0.3)).collect(),
        "i_suzu_roll" => suzu(0.0, 7, 13),
        "i_suzu_one" => cat(suzu(0.0, 2, 21), vec![(0.12, koto(note("E6"), 0.4, 0.8, 1), 0.25)]),
        "i_furin2" => cat(furin(0.0, 2637.0, 0.38), furin(0.18, 1975.5, 0.25)),
        "i_mokugyo" => (0..2).map(|i| (i as f64 * 0.16, tap((620 + 40 * i) as f64, 0.09), 0.7)).collect(),
        "i_hyoshigi" => vec![(0.0, tap(1900.0, 0.05), 0.7), (0.11, tap(2100.0, 0.05), 0.6)],
        "i_bubble" => (0..3)
            .map(|i| (i as f64 * 0.06, pop((500 + 160 * i) as f64, (1300 + 220 * i) as f64, 0.06), 0.4)).collect(),
        "i_sparkle" => ["A6", "E6", "B5", "E6"].iter().enumerate()
            .map(|(i, n)| (i as f64 * 0.05, bell(note(n), 0.5, &[(1.0, 0.5), (2.76, 0.18)], 0.25), 0.28)).collect(),
        "i_shamisen" => vec![(0.0, koto(note("E5"), 0.35, 0.95, 9), 0.55), (0.1, koto(note("B5"), 0.45, 0.95, 4), 0.45)],
        "i_purr" => {
            let mut ph = 0.0f64;
            let out = (0..trunc(0.55 * SRF))
                .map(|i| {
                    let t = i as f64 / SRF;
                    ph += 2.0 * PI * (92.0 + 6.0 * (t * 9.0).sin()) / SRF;
                    let amp = (0.55 + 0.45 * (2.0 * PI * 24.0 * t).sin()) * (t / 0.05).min(1.0)
                        * ((0.55 - t) / 0.12).min(1.0);
                    amp * (ph.sin() + 0.4 * (2.0 * ph).sin())
                })
                .collect();
            vec![(0.0, out, 0.5)]
        }
        "i_flute" => {
            let mut rnd = PyRandom::new(4);
            let (mut ph, mut lp) = (0.0f64, 0.0f64);
            let a5 = note("A5");
            let out = (0..trunc(0.6 * SRF))
                .map(|i| {
                    let t = i as f64 / SRF;
                    ph += 2.0 * PI * a5 * (1.0 + 0.01 * (2.0 * PI * 5.0 * t).sin()) / SRF;
                    lp = lp * 0.8 + rnd.uniform(-1.0, 1.0) * 0.2;
                    let env = (t / 0.08).min(1.0) * ((0.6 - t) / 0.2).min(1.0);
                    env * (0.8 * ph.sin() + 0.15 * (2.0 * ph).sin() + 0.35 * lp)
                })
                .collect();
            vec![(0.0, out, 0.4)]
        }
        "i_taiko_tap" => vec![(0.0, drum(150.0, 0.18), 0.6), (0.12, tap(1500.0, 0.04), 0.3)],
        "i_chime_pair" => arp(&["E5", "B5"], 0.0, 0.12, 0.6, 0.45),
        _ => return extra_base(kind, pitch),
    })
}

/// `_sound_parts(kind)`: what each sound is made of. "a+b" mixes two (b, her voice, on top). `pitch` is
/// `SOUND["pitch"]` (config `chirp_pitch`).
pub fn sound_parts(kind: &str, pitch: f64) -> Vec<Part> {
    if let Some((a, b)) = kind.split_once('+') {
        let mut p: Vec<Part> = sound_parts(a, pitch).into_iter().map(|(s, m, g)| (s, m, g * 0.5)).collect();
        p.extend(sound_parts(b, pitch));
        return p;
    }
    if let Some(ex) = extra(kind, pitch) {
        return ex;
    }
    match kind {
        "tap" => vec![(0.0, tap(1700.0, 0.06), 0.55)],
        "toggle" => vec![(0.0, koto(note("A5"), 0.28, 0.8, 1), 0.45), (0.0, tap(2400.0, 0.03), 0.25)],
        "nav" => vec![(0.0, koto(note("E5"), 0.45, 0.7, 1), 0.42), (0.05, koto(note("A5"), 0.45, 0.7, 2), 0.32)],
        "open" => furin(0.0, 2093.0, 0.42),
        "close" => vec![(0.0, koto(note("B5"), 0.35, 0.6, 1), 0.35), (0.07, koto(note("E5"), 0.45, 0.6, 2), 0.35)],
        "send" => vec![(0.0, pop(420.0, 1100.0, 0.07), 0.5)],
        "reply" => vec![(0.0, koto(note("D6"), 0.35, 0.8, 1), 0.35), (0.08, koto(note("A5"), 0.4, 0.8, 4), 0.3)],
        "hello" => cat(arp(&["D5", "A5"], 0.0, 0.13, 0.5, 0.6), suzu(0.22, 2, 3)),
        "pet" => cat(arp(&["E6", "A6"], 0.0, 0.07, 0.35, 0.5), suzu(0.1, 2, 9)),
        "happy" => arp(&["D5", "E5", "A5", "D6"], 0.0, 0.09, 0.6, 0.55),
        "start" => cat(suzu(0.0, 5, 3), arp(&["D5", "E5", "G5", "A5", "D6"], 0.28, 0.09, 0.7, 0.5)),
        "done" => cat(
            cat(suzu(0.0, 6, 5), arp(&["A4", "D5", "E5", "A5", "B5", "D6"], 0.3, 0.1, 0.9, 0.5)),
            furin(0.95, 2349.0, 0.32),
        ),
        "ding" => furin(0.0, 2349.0, 0.5),
        "eyes_done" => cat(cat(suzu(0.0, 3, 7), arp(&["A5", "E6"], 0.12, 0.16, 0.9, 0.6)), furin(0.45, 2637.0, 0.45)),
        "sad" => arp(&["Bb4", "A4", "Eb4", "D4"], 0.0, 0.2, 0.9, 0.6),
        "nag" => cat(vec![(0.0, drum(90.0, 0.32), 0.9), (0.18, drum(110.0, 0.32), 0.7)], suzu(0.34, 3, 11)),
        _ => vec![],
    }
}

/// `_wav_bytes(kind, vol)`: the finished 16-bit mono .wav, or None for an unknown sound.
pub fn wav_bytes(kind: &str, vol: i32, pitch: f64) -> Option<Vec<u8>> {
    let parts = sound_parts(kind, pitch);
    if parts.is_empty() {
        return None;
    }
    let n = parts.iter().map(|(st, sm, _)| trunc(st * SRF) + sm.len()).max().unwrap() + trunc(0.02 * SRF);
    let mut mixb = vec![0.0f64; n];
    for (st, sm, gn) in &parts {
        let o = trunc(st * SRF);
        for (i, v) in sm.iter().enumerate() {
            mixb[o + i] += v * gn;
        }
    }
    let peak = mixb.iter().fold(1e-6f64, |m, v| m.max(v.abs()));
    let level = match vol {
        1 => 0.3,
        3 => 0.85,
        _ => 0.55,
    };
    let k = 32767.0 * level / peak.max(1.0);
    let fade = trunc(0.015 * SRF);
    for i in 0..fade {
        mixb[n - 1 - i] *= i as f64 / fade as f64;
    }
    let mut data = Vec::with_capacity(n * 2);
    for v in &mixb {
        let s = ((v * k / 32767.0).clamp(-1.0, 1.0) * 32767.0) as i16;
        data.extend_from_slice(&s.to_le_bytes());
    }
    let mut w = Vec::with_capacity(44 + data.len());
    w.extend_from_slice(b"RIFF");
    w.extend_from_slice(&(36 + data.len() as u32).to_le_bytes());
    w.extend_from_slice(b"WAVEfmt ");
    w.extend_from_slice(&16u32.to_le_bytes());
    w.extend_from_slice(&1u16.to_le_bytes());
    w.extend_from_slice(&1u16.to_le_bytes());
    w.extend_from_slice(&(SR as u32).to_le_bytes());
    w.extend_from_slice(&(SR as u32 * 2).to_le_bytes());
    w.extend_from_slice(&2u16.to_le_bytes());
    w.extend_from_slice(&16u16.to_le_bytes());
    w.extend_from_slice(b"data");
    w.extend_from_slice(&(data.len() as u32).to_le_bytes());
    w.extend_from_slice(&data);
    Some(w)
}

pub const SOUND_KINDS: [&str; 16] = [
    "tap", "toggle", "nav", "open", "close", "send", "reply", "hello", "pet", "happy", "start", "done", "ding", "sad",
    "nag", "eyes_done",
];
pub const SYNTH_POOL: [&str; 15] = [
    "i_koto_up", "i_koto_down", "i_koto_trill", "i_suzu_roll", "i_suzu_one", "i_furin2", "i_mokugyo", "i_hyoshigi",
    "i_bubble", "i_sparkle", "i_shamisen", "i_purr", "i_flute", "i_taiko_tap", "i_chime_pair",
];
pub const EXTRA_PLAIN: [&str; 8] = ["song", "munch", "chomp", "boop", "giggle", "gift", "omikuji", "yawn"];

/// `VOICE_FOR`: her voice line for each Zenko sound.
pub fn voice_for(kind: &str) -> Option<&'static str> {
    Some(match kind {
        "hello" | "start" => "v_kon",
        "pet" => "v_kyu",
        "happy" => "v_hehe",
        "done" | "gift" => "v_yatta",
        "nag" => "v_mou",
        "sad" => "v_uu",
        "munch" | "chomp" => "v_nom",
        "giggle" | "boop" | "pounce" => "v_kya",
        "yawn" => "v_fuwa",
        _ => return None,
    })
}

/// `_VOICE_EXTRA`: the voice line for a sound that has no voice of its own.
pub fn voice_extra(kind: &str) -> Option<&'static str> {
    Some(match kind {
        "ding" | "eyes_done" => "v_hai",
        "omikuji" | "yarn" => "v_waai",
        "found" => "v_sugoi",
        "praise" | "forgive" => "v_ureshii",
        "brush" => "v_nade",
        "tickle" | "belly" => "v_kusugu",
        "tail" => "v_kya",
        "highfive" => "v_yatta",
        "treat" => "v_nom",
        "hide" => "v_fufu",
        _ => return None,
    })
}

/// `_SOUND_GROUP` + the zenko17 voice-only `SOUND_POOLS`.
pub fn sound_group(kind: &str) -> Option<&'static str> {
    Some(match kind {
        "pet" | "brush" | "praise" | "forgive" => "soft",
        "boop" | "giggle" | "belly" | "tickle" | "tail" | "hide" | "found" => "silly",
        "munch" | "treat" => "food",
        "happy" | "gift" | "yarn" | "highfive" => "play",
        "yawn" => "sleepy",
        _ => return None,
    })
}

pub fn sound_pool(group: &str) -> &'static [&'static str] {
    match group {
        "soft" => &["v_nade", "v_ureshii", "v_kyu", "v_fufu", "v_motto", "v_ehe"],
        "silly" => &["v_kya", "v_kusugu", "v_hehe", "v_fufu", "v_ehe"],
        "food" => &["v_nom", "v_ureshii", "v_waai", "v_hehe"],
        "sleepy" => &["v_fuwa", "v_uu"],
        _ => &["v_waai", "v_yatta", "v_sugoi", "v_hai", "v_nee", "v_ganbare"],
    }
}

/// `sound_file` name: "<kind>_v<vol>_<tag>.wav" (the same files zenko.py keeps in FoxBuddy\sounds, so both apps share them).
pub fn sound_file_name(kind: &str, vol: i32, pitch: f64) -> String {
    let tag = if kind.contains("v_") { format!("k1p{}", (pitch * 100.0) as i64) } else { "k1".to_string() };
    format!("{}_v{}_{}.wav", kind.replace('+', "_"), vol, tag)
}
