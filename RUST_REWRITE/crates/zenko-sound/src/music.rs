//! zenko.py's background music (`_hz`, `_scale_note`, `_flute`, `_pad`, `_rain`, `_mix_into`, `music_samples`,
//! `music_file`, `music_gain`, `music_file_vol`): three 72-second loops composed by rules in Japanese pentatonic
//! scales. The buffers are float32 (`array("f")`) exactly like Python's, so every add is rounded to f32 in the same
//! place, and the .wav bytes come out the same (tests/music.rs).

use crate::mt::PyRandom;
use crate::{SR, bell, furin, koto};
use std::f64::consts::PI;

const SRF: f64 = SR as f64;
pub const MUSIC_LEN: f64 = 72.0;
pub const TRACKS: [&str; 3] = ["garden", "bamboo", "temple"];

fn hz(semi: f64, base: f64) -> f64 {
    base * 2.0f64.powf(semi / 12.0)
}

fn hz0(semi: f64) -> f64 {
    hz(semi, 293.66)
}

fn scale_note(scale: &str, step: i64, base: f64) -> f64 {
    let s: &[i64] = match scale {
        "yo" => &[0, 2, 5, 7, 9],
        "minyo" => &[0, 3, 5, 7, 10],
        _ => &[0, 1, 5, 7, 8],
    };
    let n = s.len() as i64;
    let (octv, i) = (step.div_euclid(n), step.rem_euclid(n));
    hz((s[i as usize] + 12 * octv) as f64, base)
}

fn flute(freq: f64, dur: f64, seed: u64) -> Vec<f32> {
    let mut rnd = PyRandom::new(seed);
    let n = (dur * SRF) as usize;
    let mut out = vec![0f32; n];
    let (mut ph, mut lp) = (0.0f64, 0.0f64);
    let (att, rel) = ((0.35 * SRF) as usize, (0.7 * SRF) as usize);
    for i in 0..n {
        let t = i as f64 / SRF;
        let vib = 1.0 + (0.006 * (2.0 * PI * 4.8 * t).sin() * 1.0f64.min(0.0f64.max(t - 0.5) / 0.6));
        ph += 2.0 * PI * freq * vib / SRF;
        let env = 1.0f64.min(i as f64 / att as f64) * 1.0f64.min((n - i) as f64 / rel as f64);
        lp += 0.08 * (rnd.uniform(-1.0, 1.0) - lp);
        out[i] = (env * (0.62 * ph.sin() + 0.16 * (2.0 * ph).sin() + 0.05 * (3.0 * ph).sin()
            + 0.55 * lp * (0.6 + 0.4 * env))) as f32;
    }
    out
}

fn pad(freq: f64, dur: f64, swell: f64) -> Vec<f32> {
    let n = (dur * SRF) as usize;
    let (w1, w2) = (2.0 * PI * freq / SRF, 2.0 * PI * freq * 1.5 / SRF);
    let ws = 2.0 * PI / (swell * SRF);
    (0..n)
        .map(|i| {
            let i = i as f64;
            let a = 0.55 + 0.45 * (ws * i).sin();
            (a * (0.7 * (w1 * i).sin() + 0.25 * (w2 * i + 1.3).sin())) as f32
        })
        .collect()
}

fn rain(dur: f64, seed: u64) -> Vec<f32> {
    let mut rnd = PyRandom::new(seed);
    let n = (dur * SRF) as usize;
    let (mut lp1, mut lp2) = (0.0f64, 0.0f64);
    (0..n)
        .map(|_| {
            let x = rnd.uniform(-1.0, 1.0);
            lp1 += 0.35 * (x - lp1);
            lp2 += 0.02 * (lp1 - lp2);
            ((lp1 - lp2) * 0.9) as f32
        })
        .collect()
}

/// _mix_into: add into the looping buffer, wrapping round the end
fn mix<T: Copy + Into<f64>>(buf: &mut [f32], start_s: f64, samples: &[T], gain: f64) {
    let l = buf.len();
    let o = ((start_s * SRF) as i64).rem_euclid(l as i64) as usize;
    for (i, &v) in samples.iter().enumerate() {
        let mut j = o + i;
        if j >= l {
            j %= l;
        }
        buf[j] = (buf[j] as f64 + v.into() * gain) as f32;
    }
}

/// music_samples(track): one looping piece (float32)
pub fn music_samples(track: &str) -> Vec<f32> {
    let length = MUSIC_LEN;
    let mut rnd = PyRandom::new(7 + track.chars().count() as u64);
    let l = (length * SRF) as usize;
    let mut buf = vec![0f32; l];
    if track == "bamboo" {
        mix(&mut buf, 0.0, &rain(length, 11), 0.05);
        mix(&mut buf, 0.0, &pad(hz0(-24.0), length, 24.0), 0.06);
        let (mut t, mut step) = (1.5f64, 5i64);
        while t < length - 0.5 {
            for _ in 0..rnd.randint(3, 5) {
                step = 3.max(11.min(step + *rnd.choice(&[-2, -1, -1, 1, 1, 2])));
                let d = *rnd.choice(&[1.8, 2.4, 3.2]);
                mix(&mut buf, t, &flute(scale_note("minyo", step, 293.66 / 2.0), d + 0.6, (t * 10.0) as u64), 0.20);
                t += d;
            }
            if rnd.random() < 0.6 {
                let k = rnd.randint(0, 4);
                mix(&mut buf, t + 0.3, &koto(scale_note("minyo", k, 293.66 / 2.0), 2.2, 0.3, t as u64), 0.16);
            }
            t += rnd.uniform(3.0, 5.0);
        }
    } else if track == "temple" {
        mix(&mut buf, 0.0, &pad(hz0(-24.0), length, 20.0), 0.10);
        mix(&mut buf, 0.0, &pad(hz0(-17.0), length, 29.0), 0.05);
        let mut t = 0.8f64;
        let bowls = [hz0(-5.0), hz0(0.0), hz0(2.0), hz0(7.0)];
        while t < length - 0.5 {
            let f = *rnd.choice(&bowls);
            mix(&mut buf, t, &bell(f, 7.0, &[(1.0, 0.6), (2.71, 0.25), (5.15, 0.08), (0.5, 0.2)], 2.4), 0.16);
            if rnd.random() < 0.45 {
                let at = t + rnd.uniform(2.0, 3.5);
                let k = rnd.randint(5, 9);
                mix(&mut buf, at, &koto(scale_note("in", k, 293.66), 2.0, 0.35, (t * 3.0) as u64), 0.12);
            }
            t += rnd.uniform(6.5, 10.0);
        }
    } else {
        mix(&mut buf, 0.0, &pad(hz0(-24.0), length, 16.0), 0.07);
        let (mut t, mut step) = (0.6f64, 7i64);
        while t < length - 0.5 {
            let n = rnd.randint(4, 6);
            for k in 0..n {
                step = 2.max(12.min(step + *rnd.choice(&[-2, -1, -1, 1, 1, 2, 0])));
                let f = scale_note("yo", step, 293.66 / 2.0);
                mix(&mut buf, t, &koto(f, 2.4, 0.45, (t * 7.0) as u64 + k as u64), 0.30);
                if k == 0 || rnd.random() < 0.25 {
                    mix(&mut buf, t, &koto(f / 2.0, 2.6, 0.3, (t * 5.0) as u64 + k as u64), 0.14);
                }
                t += *rnd.choice(&[0.55, 0.7, 0.85, 1.1]);
            }
            if rnd.random() < 0.55 {
                let st = t + rnd.uniform(0.3, 1.2);
                let fr = *rnd.choice(&[2093.0, 2349.0, 2637.0]);
                for (s0, sm, g) in furin(0.0, fr, 0.25) {
                    mix(&mut buf, st + s0, &sm, g * 0.5);
                }
            }
            t += rnd.uniform(2.6, 4.2);
        }
    }
    // a soft room echo (multi-tap, wrapping) and a gentle top-end roll-off
    let dry = buf.clone();
    for (d_s, g) in [(0.113, 0.22), (0.197, 0.17), (0.293, 0.13), (0.419, 0.10), (0.587, 0.07), (0.811, 0.05)] {
        let d = (d_s * SRF) as usize;
        for i in 0..l {
            let v = dry[(i + l - d) % l] as f64;
            buf[i] = (buf[i] as f64 + v * g) as f32;
        }
    }
    let mut lp = buf[l - 1] as f64;
    for i in 0..l {
        lp += 0.55 * (buf[i] as f64 - lp);
        buf[i] = lp as f32;
    }
    buf
}

/// CPython 3.12 sum() over floats (Neumaier compensated)
pub fn py_sum(it: impl Iterator<Item = f64>) -> f64 {
    let mut it = it;
    let Some(mut f) = it.next() else { return 0.0 };
    let mut c = 0.0f64;
    for x in it {
        let t = f + x;
        if f.abs() >= x.abs() {
            c += (f - t) + x;
        } else {
            c += (x - t) + f;
        }
        f = t;
    }
    if c != 0.0 && c.is_finite() {
        f += c;
    }
    f
}

fn wav(samples: &[i16]) -> Vec<u8> {
    let n = samples.len() * 2;
    let mut v = Vec::with_capacity(44 + n);
    v.extend(b"RIFF");
    v.extend(((36 + n) as u32).to_le_bytes());
    v.extend(b"WAVEfmt ");
    v.extend(16u32.to_le_bytes());
    v.extend(1u16.to_le_bytes());
    v.extend(1u16.to_le_bytes());
    v.extend((SR as u32).to_le_bytes());
    v.extend((SR as u32 * 2).to_le_bytes());
    v.extend(2u16.to_le_bytes());
    v.extend(16u16.to_le_bytes());
    v.extend(b"data");
    v.extend((n as u32).to_le_bytes());
    for s in samples {
        v.extend(s.to_le_bytes());
    }
    v
}

/// music_file(track) bytes: every track equally quiet (rms 0.1), never clipping
pub fn music_wav(track: &str) -> Vec<u8> {
    let buf = music_samples(track);
    let peak = buf.iter().fold(0.0f64, |m, &v| m.max((v as f64).abs())).max(1e-6);
    let ss = py_sum(buf.iter().map(|&v| v as f64 * v as f64));
    let rms = (ss / buf.len() as f64).sqrt().max(1e-6);
    let k = (0.9 / peak).min(0.1 / rms);
    let s: Vec<i16> = buf.iter().map(|&v| ((v as f64 * k).clamp(-1.0, 1.0) * 32767.0) as i16).collect();
    wav(&s)
}

/// music_gain(vol): level 1 is about -33 dB, each step adds 3 dB
pub fn music_gain(vol: i64) -> f64 {
    let vol = vol.clamp(1, 10);
    10.0f64.powf((-33.0 + 3.0 * (vol - 1) as f64) / 20.0)
}

/// music_file_vol: the base .wav with its volume baked in
pub fn music_wav_vol(base: &[u8], vol: i64) -> Vec<u8> {
    let g = music_gain(vol);
    let data = &base[44.min(base.len())..];
    let s: Vec<i16> = data.chunks_exact(2).map(|c| (i16::from_le_bytes([c[0], c[1]]) as f64 * g) as i16).collect();
    wav(&s)
}

pub fn track_or_garden(track: &str) -> &'static str {
    TRACKS.iter().find(|t| **t == track).copied().unwrap_or("garden")
}
