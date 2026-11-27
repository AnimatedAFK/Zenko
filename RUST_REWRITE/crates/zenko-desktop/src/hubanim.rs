//! hub_anim's sidebar life (FoxBuddy with Zenko19's additions): the twinkling stars, the drifting particles that
//! follow the season and weather, and the little fox at the top who blinks, sways her head, wears what the desktop
//! fox wears, curls up asleep, and acts out the page you're on (PAGE_POSE: reading, lifting, zen, walking...).

use crate::Companion;
use crate::hubside::Side;
use crate::hubwin::mix;
use crate::tkcanvas::Photo;
use std::collections::HashMap;
use std::rc::Rc;
use zenko_art::core::build_tails;
use zenko_art::core::Grid;
use zenko_art::palette::{Palette, is_robe, mixhex};

/// Zenko19.PAGE_POSE: page -> (kind, what)
pub const PAGE_POSE: [(&str, &str, &str); 13] = [
    ("notebook", "sit", "read"), ("exercise", "sit", "lift"), ("garden", "sit", "zen"), ("map", "rig", "walk"),
    ("settings", "rig", "lie"), ("customize", "sit", "hearts"), ("you", "rig", "playbow"), ("chat", "sit", "talk"),
    ("screen", "rig", "stand"), ("deadlines", "sit", "focus"), ("timetable", "sit", "focus"),
    ("reminders", "sit", "happy"), ("daily", "sit", "happy"),
];

/// PARTICLES: each colour scheme's own drift
pub fn particles_of(theme: &str) -> &'static str {
    match theme {
        "silver" | "midnight" => "stars",
        "mint" | "classic" | "fennec" => "fireflies",
        "sakura" => "petals",
        "ember" => "embers",
        _ => "snow",
    }
}

/// a sidebar flake: (item, x, y, speed, phase, size, colour)
#[derive(Clone, Debug)]
pub struct Flake {
    pub it: u32,
    pub x: f64,
    pub y: f64,
    pub v: f64,
    pub ph: f64,
    pub sz: f64,
}

#[derive(Debug, Default)]
pub struct HubAnim {
    pub blink_at: f64,
    pub blink_until: f64,
    pub flakes: Vec<Flake>,
    pub pstyle: String,
    cache: HashMap<String, Rc<Photo>>,
    /// h["logo_cache"]'s page-pose bodies: the first picture made for a key stays (as zenko.py's cache keeps it)
    grids: HashMap<String, Grid>,
    logo_key: String,
    seed: u64,
}

/// what the logo shows this frame: the tails (None = hidden) and the body, with a key that names them
pub struct LogoFrame {
    pub key: String,
    pub tails: Option<(String, Grid)>,
    pub body: (String, Grid),
}

impl HubAnim {
    pub fn new(now: f64, seed: u64) -> HubAnim {
        HubAnim { blink_at: now + 2.0, seed: seed | 1, ..Default::default() }
    }

    /// Python's global random stands in here (its sequence isn't part of what you see)
    fn rnd(&mut self) -> f64 {
        self.seed ^= self.seed << 13;
        self.seed ^= self.seed >> 7;
        self.seed ^= self.seed << 17;
        (self.seed >> 11) as f64 / (1u64 << 53) as f64
    }

    fn uniform(&mut self, a: f64, b: f64) -> f64 {
        a + (b - a) * self.rnd()
    }

    fn photo(&mut self, key: &str, g: &Grid, pal: &Palette, zoom: usize, hd: bool) -> Rc<Photo> {
        if self.cache.len() > 120 {
            self.cache.clear();
            self.grids.clear();
        }
        self.cache.entry(key.to_string()).or_insert_with(|| {
            let im = zenko_art::raster::sprite_photo(&[g], pal, zoom, hd);
            Rc::new(Photo { w: im.w, h: im.h, px: im.px })
        }).clone()
    }

    /// twinkle every third star, drift the particles, put the logo's pictures in: one hub_anim frame
    #[allow(clippy::too_many_arguments)]
    pub fn frame(&mut self, side: &mut Side, now: f64, night: &str, ink: &str, style: &str, col: &str, on: bool,
                 logo: Option<LogoFrame>, pal: &Palette, hd: bool) {
        let (w, hh) = (side.w as f64, side.h as f64);
        if side.w < 100 {
            return;
        }
        for (it, ph, sp) in side.stars.clone().into_iter().step_by(3) {
            let a = 0.5 + 0.5 * (now * sp + ph).sin();
            if let Some(i) = side.cv.get(it) {
                i.fill = mixhex(night, ink, 0.2 + 0.65 * a);
            }
        }
        if self.pstyle != style {
            side.cv.delete("flake");
            self.flakes.clear();
            self.pstyle = style.into();
        }
        if !on {
            if !self.flakes.is_empty() {
                side.cv.delete("flake");
                self.flakes.clear();
            }
        } else {
            if self.flakes.is_empty() {
                for _ in 0..26 {
                    let (x, y) = (self.uniform(0.0, w), self.uniform(0.0, hh));
                    let sz = [2.0, 2.0, 3.0, 3.0, 4.0][(self.rnd() * 5.0) as usize % 5];
                    let shade = mix(night, col, self.uniform(0.45, 0.95));
                    side.cv.rect(x, y, x + sz, y + sz).fill(&shade).outline("").tags(&["flake"]);
                    let it = side.cv.last_id();
                    let (v, ph) = (self.uniform(0.3, 1.1), self.uniform(0.0, 6.28));
                    self.flakes.push(Flake { it, x, y, v, ph, sz });
                }
            }
            for k in 0..self.flakes.len() {
                let mut f = self.flakes[k].clone();
                f.ph += 0.05;
                match style {
                    "stars" | "fireflies" => {
                        let a = (f.ph * if style == "stars" { 0.5 } else { 0.9 }).sin().abs();
                        if let Some(i) = side.cv.get(f.it) {
                            i.fill = mix(night, col, 0.15 + 0.85 * a);
                        }
                        if style == "fireflies" {
                            f.x += 0.35 * (f.ph * 0.7).sin();
                            f.y -= 0.18 * f.v;
                        }
                    }
                    "rain" => f.y += 5.5 + 2.5 * f.v,
                    _ => {
                        let soft = matches!(style, "petals" | "leaves");
                        let down = if matches!(style, "snow" | "petals" | "leaves") { 1.0 } else { -1.0 };
                        f.y += f.v * down * if soft { 0.55 } else { 0.9 };
                        f.x += if soft { 0.45 } else { 0.2 } * f.ph.sin();
                    }
                }
                if f.y > hh + 6.0 {
                    f.y = -6.0;
                    f.x = self.uniform(0.0, w);
                } else if f.y < -6.0 {
                    f.y = hh + 6.0;
                    f.x = self.uniform(0.0, w);
                }
                let wd = f.sz + if matches!(style, "petals" | "leaves") && (f.ph * 2.0) as i64 % 2 != 0 { 1.0 } else { 0.0 };
                if let Some(i) = side.cv.get(f.it) {
                    i.coords = if style == "rain" {
                        vec![f.x, f.y, f.x + 1.0, f.y + 3.0 * f.sz]
                    } else {
                        vec![f.x, f.y, f.x + wd, f.y + f.sz]
                    };
                }
                self.flakes[k] = f;
            }
            side.cv.tag_raise("flake");
        }
        if let (Some(lf), Some((ti, bi))) = (logo, side.logo_items)
            && lf.key != self.logo_key {
            self.logo_key = lf.key.clone();
            let z = side.logo_zoom.max(1) as usize;
            let tails = lf.tails.map(|(k, g)| self.photo(&k, &g, pal, z, hd));
            let body = self.photo(&lf.body.0, &lf.body.1, pal, z, hd);
            if let Some(i) = side.cv.get(ti) {
                match tails {
                    Some(p) => {
                        i.image = Some(p);
                        i.hidden = false;
                    }
                    None => i.hidden = true,
                }
            }
            if let Some(i) = side.cv.get(bi) {
                i.image = Some(body);
            }
        }
    }
}

/// hub_anim's particle style (Zenko19: the season and weather when sky_seasons is on) and colour
pub fn particle_style(theme: &str, sky_seasons: bool, weather: Option<&str>, season: &str, gold: &str, glow: &str,
                      acc: &str) -> (String, String) {
    let style = if sky_seasons {
        match weather {
            Some("rain") => "rain",
            Some("snow") => "snow",
            _ => match season {
                "winter" => "snow",
                "spring" => "petals",
                "summer" => "fireflies",
                _ => "leaves",
            },
        }
    } else {
        particles_of(theme)
    };
    let col = match style {
        "petals" => "#ffb8d6".to_string(),
        "stars" => gold.to_string(),
        "fireflies" => mixhex(gold, glow, 0.3),
        "embers" => acc.to_string(),
        "leaves" => "#d9703a".to_string(),
        "rain" => "#9fb8e8".to_string(),
        _ => "#e6eeff".to_string(),
    };
    (style.to_string(), col)
}

impl Companion {
    /// the sidebar fox this frame (FoxBuddy.hub_anim's logo, then Zenko19.z19_logo's page pose over it)
    pub fn hub_logo(&mut self, now: f64, page: Option<&str>, a: &mut HubAnim, living: bool) -> LogoFrame {
        let mood = self.current_mood(now);
        let mut state = if zenko_art::sit::state(&mood).is_some() { mood.clone() } else { "idle".into() };
        if state == "idle" || state == "focus" {
            if now >= a.blink_at {
                a.blink_until = now + 0.15;
                a.blink_at = now + a.uniform(2.5, 5.5);
            }
            if now < a.blink_until {
                state = "blink".into();
            }
        }
        let tf = (now / if mood == "alert" { 0.08 } else { 0.17 }) as i64 % 8;
        let mut frame = (now / 0.6) as i64 % 2;
        let mut acc = self.accessories(now);
        let curl = mood == "sleep" && self.is_curled(now);
        if curl {
            state = "curl".into();
            acc.retain(|x| x == "headphones");
            frame = (now / 1.6) as i64 % 2;
        }
        let n = self.tail_count(now);
        let mut lhx = 0;
        if matches!(state.as_str(), "idle" | "focus" | "blink") {
            let sw = (now * 0.8).sin();
            lhx = if sw > 0.55 { 1 } else if sw < -0.55 { -1 } else { 0 };
        }
        let accs: Vec<&str> = acc.iter().map(|s| s.as_str()).collect();
        let ak = acc.join(",");
        let tails_of = |tf: i64| (format!("t|{n}|{tf}"), build_tails(n, tf as i32, 8, false));
        let bf = if matches!(state.as_str(), "happy" | "sleep" | "alert" | "alert2" | "curl") { frame } else { 0 };
        let body = if curl {
            (format!("b|curl|{bf}|{ak}|{n}"), zenko_art::curl::build_curl(bf as i32, &accs, n))
        } else {
            (format!("b|{state}|{bf}|{ak}|0|{lhx}"),
             zenko_art::sit::build_frame(&state, bf as i32, None, &accs, lhx, 0, &self.look))
        };
        let mut out = LogoFrame { key: format!("{}|{}", tails_of(tf).0, body.0),
                                  tails: if curl { None } else { Some(tails_of(tf)) }, body };
        // Zenko19: she acts out the page you're on
        let pp = PAGE_POSE.iter().find(|p| Some(p.0) == page);
        let Some(&(_, kind, what)) = pp else { return out };
        if !living || matches!(mood.as_str(), "sleep" | "alert") || (self.session.is_some() && !matches!(what, "read" | "focus")) {
            return out;
        }
        if kind == "rig" {
            let fr = (now / if what == "walk" { 0.11 } else { 0.5 }) as i64 % 8;
            let st = format!("rigR:{what}:{fr}");
            out.body = (format!("z19r|{what}|{fr}|{n}|{ak}"), zenko_art::rig::rig_rows(&st, n, &accs, &self.look));
            out.tails = None;
        } else {
            let tf = (now / 0.17) as i64 % 8;
            let fr = (now / match what { "lift" => 0.5, "talk" => 0.35, _ => 0.9 }) as i64 % 8;
            let (mut st, acc2, extra): (String, Vec<String>, Option<&str>) = match what {
                "read" => {
                    let mut a2 = acc.clone();
                    if !a2.iter().any(|x| x == "glasses") {
                        a2.push("glasses".into());
                    }
                    ("focus".into(), a2, Some("read"))
                }
                "lift" => {
                    let mut a2: Vec<String> = acc.iter().filter(|x| !is_robe(x)).cloned().collect();
                    if !acc.iter().any(|x| x == "headband") {
                        a2.push("headband".into());
                    }
                    (if fr % 4 == 0 { "happy" } else { "focus" }.into(), a2, Some("lift"))
                }
                "zen" => ("blink".into(), acc.clone(), Some("zen")),
                "talk" => (if fr % 2 != 0 { "boop2" } else { "idle" }.into(), acc.clone(), None),
                "hearts" => (if fr % 4 < 2 { "hearts" } else { "happy" }.into(), acc.clone(), None),
                _ => (if zenko_art::sit::state(what).is_some() { what } else { "idle" }.into(), acc.clone(), None),
            };
            if (st == "idle" || st == "focus") && now < a.blink_until {
                st = "blink".into();
            }
            let a2: Vec<&str> = acc2.iter().map(|s| s.as_str()).collect();
            let sub = if extra.is_some() { fr % 4 } else { fr % 2 };
            let key = format!("z19s|{st}|{extra:?}|{sub}|{}", acc2.join(","));
            let rows = match a.grids.get(&key) {
                Some(g) => g.clone(),
                None => {
                    let mut rows = zenko_art::sit::build_frame(&st, (fr % 2) as i32, None, &a2, 0, 0, &self.look);
                    if let Some(e) = extra {
                        rows = zenko_art::sit::sit_extra(&rows, e, fr as i32);
                    }
                    a.grids.insert(key.clone(), rows.clone());
                    rows
                }
            };
            out.body = (key, rows);
            out.tails = Some(tails_of(tf));
        }
        out.key = format!("{}|{}", out.tails.as_ref().map(|t| t.0.as_str()).unwrap_or("-"), out.body.0);
        out
    }
}
