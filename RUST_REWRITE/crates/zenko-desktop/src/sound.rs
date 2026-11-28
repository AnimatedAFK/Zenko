//! Which sound to play and where its .wav lives (zenko.py sound_file, beep (zenko17: her voice only),
//! varied_sound, _pool_sound_file, ui_sound). The .wav files are the same ones zenko.py makes in
//! %APPDATA%\FoxBuddy\sounds (same names, same bytes), so whichever app made a file first, the other just uses it.

use crate::{Cfg, data_dir};
use std::path::PathBuf;

pub struct SoundCfg {
    pub on: bool,
    pub ui: bool,
    pub vol: i32,
    pub clips: String,
    pub pitch: f64,
    pub vv_speaker: i64,
}

impl SoundCfg {
    pub fn from(c: &Cfg) -> SoundCfg {
        let vol = c.f("sound_volume", 2.0) as i32;
        let pitch = c.f("chirp_pitch", 1.0);
        let vv = c.f("voicevox_speaker", 8.0) as i64;
        SoundCfg {
            on: c.b("sound", true),
            ui: c.b("ui_sounds", true),
            vol: if vol == 0 { 2 } else { vol },
            clips: c.s("voice_clips", "synth"),
            pitch: if pitch == 0.0 { 1.0 } else { pitch },
            vv_speaker: if vv == 0 { 8 } else { vv },
        }
    }
}

pub fn sound_dir() -> PathBuf {
    data_dir().join("sounds")
}

/// sound_file(kind): the .wav for this sound (made on first use, then kept).
pub fn sound_file(kind: &str, sc: &SoundCfg) -> Option<PathBuf> {
    let p = sound_dir().join(zenko_sound::sound_file_name(kind, sc.vol, sc.pitch));
    if !p.exists() {
        let data = zenko_sound::wav_bytes(kind, sc.vol, sc.pitch)?;
        std::fs::create_dir_all(sound_dir()).ok()?;
        let tmp = p.with_extension("wav.part");
        std::fs::write(&tmp, data).ok()?;
        std::fs::rename(&tmp, &p).ok()?;
    }
    Some(p)
}

fn pool_sound_file(key: &str, sc: &SoundCfg) -> Option<PathBuf> {
    if key.starts_with("v_") {
        if sc.clips == "off" {
            return None;
        }
        if sc.clips == "voicevox" {
            let p = sound_dir().join(format!("vv{}_{}.wav", sc.vv_speaker, key));
            if p.exists() {
                return Some(p);
            }
        }
    }
    sound_file(key, sc)
}

/// SOUND_VARIETY: never the clip that played last (or the two before it), and quiet when you click fast.
#[derive(Default)]
pub struct Variety {
    pub last: String,
    pub recent: Vec<String>,
    pub t: f64,
}

impl Variety {
    fn pick(&mut self, kind: &str, now: f64, sc: &SoundCfg, r: f64) -> Option<String> {
        if now - self.t < 2.4 {
            return None;
        }
        let grp = zenko_sound::sound_group(kind).unwrap_or("play");
        let pool = zenko_sound::sound_pool(grp);
        if sc.clips == "off" || pool.is_empty() {
            return None;
        }
        let tail: Vec<&String> = self.recent.iter().rev().take(3).collect();
        let mut fresh: Vec<&str> = pool.iter().copied().filter(|k| !tail.iter().any(|t| t.as_str() == *k)).collect();
        if fresh.is_empty() {
            fresh = pool.iter().copied().filter(|k| *k != self.last).collect();
        }
        if fresh.is_empty() {
            fresh = pool.to_vec();
        }
        let pick = fresh[((r * fresh.len() as f64) as usize).min(fresh.len() - 1)].to_string();
        self.last = pick.clone();
        self.t = now;
        self.recent.push(pick.clone());
        let k = self.recent.len().saturating_sub(6);
        self.recent.drain(..k);
        Some(pick)
    }
}

/// beep(kind): the file of her voice clip to play (0.3 s later), or None for silence.
pub fn beep_file(kind: &str, variety: bool, v: &mut Variety, now: f64, sc: &SoundCfg, r: f64) -> Option<PathBuf> {
    let pick = if variety && zenko_sound::sound_group(kind).is_some() && kind != "song" {
        v.pick(kind, now, sc, r)
    } else {
        zenko_sound::voice_for(kind).or_else(|| zenko_sound::voice_extra(kind)).map(String::from)
    };
    let pick = pick?;
    if !sc.on || sc.clips == "off" {
        return None;
    }
    pool_sound_file(&pick, sc)
}

/// ui_sound(kind): little interface sounds (Settings > General > Interface sounds).
pub fn ui_file(kind: &str, sc: &SoundCfg) -> Option<PathBuf> {
    if !sc.ui {
        return None;
    }
    sound_file(kind, sc)
}
