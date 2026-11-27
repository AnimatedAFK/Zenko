//! Desktop life (zenko.py life_pose): roaming along the taskbar (roam_*), the four-legged walk (zenko19 walk_pose)
//! and her little idle routines (zenko19 z19_pose: stretch, play bow, pounce, rest, sniff).
//!
//! zenko.py's "nap on top of the window you're using" never happens there: roam_decide calls
//! window_rect(fg_window()), and fg_window returns a tuple, which GetWindowRect can't take, so window_rect always
//! gives None and that pick falls through to chasing the mouse. The port does the same.

use crate::play::{Play, Pose};
use crate::Companion;

/// What the window side tells her about the screen each frame.
#[derive(Clone, Debug, Default)]
pub struct Env {
    /// SPI_GETWORKAREA: left, top, right, bottom
    pub work: (i32, i32, i32, i32),
    pub screen_w: i32,
    pub idle: f64,
    pub dragging: bool,
    /// her speech bubble is showing (zenko.py's bubble_job)
    pub bubble: bool,
}

#[derive(Clone, Debug)]
pub struct Roam {
    pub mode: &'static str,
    pub tx: i32,
    pub ty: i32,
    pub t0: f64,
    pub sx: i32,
    pub sy: i32,
    pub until: f64,
    pub start: f64,
}

#[derive(Clone, Debug)]
pub struct Routine {
    pub seq: Vec<(&'static str, f64, f64)>,
    pub left: bool,
}

#[derive(Clone, Debug, Default)]
pub struct Life {
    pub roam: Option<Roam>,
    pub roam_next: f64,
    pub roam_pause_until: f64,
    pub z19_seq: Option<Routine>,
    pub z19_next: Option<f64>,
}

const ROUTINES: [(&str, &[(&str, f64)]); 5] = [
    ("stretch", &[("stand", 0.5), ("stretch_front", 1.9), ("stand", 0.35), ("stretch_back", 1.6), ("shake", 0.7)]),
    ("playbow", &[("stand", 0.3), ("playbow", 2.8), ("stand", 0.3)]),
    ("pounce", &[("stand", 0.3), ("crouch", 2.0), ("rear", 0.22), ("leap", 0.30), ("dive", 0.24), ("land", 1.4),
                 ("shake", 0.6)]),
    ("rest", &[("stand", 0.4), ("lie", 6.0), ("doze", 14.0), ("lie", 2.0), ("stretch_front", 1.5), ("shake", 0.6)]),
    ("sniff", &[("sniff", 3.2), ("stand", 0.5)]),
];
const ROUTINE_WEIGHTS: [(&str, usize); 5] = [("stretch", 3), ("playbow", 2), ("pounce", 2), ("rest", 2), ("sniff", 1)];

fn pose_dyf(p: &str) -> i32 {
    match p {
        "rear" => -2,
        "leap" => -12,
        "dive" => -5,
        _ => 0,
    }
}

impl Companion {
    fn uniform(&mut self, a: f64, b: f64) -> f64 {
        a + (b - a) * self.rand()
    }

    /// roam_home: the far right of the screen, just above the taskbar
    pub fn roam_home(&self) -> (i32, i32) {
        let (_, _, r, b) = self.env.work;
        let u = self.u;
        (r - self.w - (6.0 * u) as i32, b - self.h - (2.0 * u) as i32)
    }

    fn roam_go(&mut self, mode: &'static str, tx: i32, ty: i32, now: f64, win: (i32, i32)) {
        self.life.roam = Some(Roam { mode, tx, ty, t0: now, sx: win.0, sy: win.1, until: 0.0, start: 0.0 });
    }

    fn roam_go_home(&mut self, now: f64, wait: (f64, f64), win: (i32, i32)) {
        let (hx, hy) = self.roam_home();
        if (win.0 - hx).abs() <= 3 && (win.1 - hy).abs() <= 3 {
            self.life.roam = None;
            return;
        }
        if (win.1 - hy).abs() > 3 {
            self.roam_go("hop", win.0, hy, now, win);
            return;
        }
        self.roam_go("walk", hx, hy, now, win);
        let s = now + self.uniform(wait.0, wait.1);
        if let Some(r) = &mut self.life.roam {
            r.start = s;
        }
    }

    fn roam_ok(&self, now: f64) -> bool {
        self.cfg.b("roam", true)
            && !(self.session.is_some() || self.brk.is_some() || self.lockin_active(now) || self.env.dragging
                || self.play.is_some() || self.sulk_until > 0.0 || self.lock_eating)
            && ["idle", "happy"].contains(&self.current_mood(now).as_str())
            && self.env.idle < 180.0
    }

    fn roam_decide(&mut self, now: f64, win: (i32, i32)) {
        let (l, _, r, _) = self.env.work;
        let (hx, hy) = self.roam_home();
        let x = win.0;
        let pick = self.rand();
        // pick < 0.25 would be the window-edge nap, which never finds a window (see the top of this file)
        if pick < 0.45 {
            self.roam_go("chase", x, hy, now, win);
            if let Some(r) = &mut self.life.roam {
                r.until = now + 6.0;
            }
            return;
        }
        let (lo, hi) = (l as f64, (r - self.w) as f64);
        let mut tx = if self.rand() < 0.7 { self.uniform(lo, hi) } else { hx as f64 };
        if (tx - x as f64).abs() < 80.0 {
            tx = if (hx - x).abs() > 80 { hx as f64 } else { (lo + hi) / 2.0 };
        }
        self.roam_go("walk", tx as i32, hy, now, win);
    }

    /// roam_step: called each frame; moves the window (self.win_move) and sets a walking pose.
    fn roam_step(&mut self, now: f64, st: &mut Pose, win: (i32, i32), pointer: (i32, i32)) {
        let Some(mut rm) = self.life.roam.clone() else {
            if self.roam_ok(now) && now >= self.life.roam_next && now >= self.life.roam_pause_until {
                self.life.roam_next = now + self.uniform(40.0, 110.0);
                self.roam_decide(now, win);
            }
            return;
        };
        if !self.cfg.b("roam", true) || self.session.is_some() || self.brk.is_some() || self.lockin_active(now)
            || self.env.dragging
        {
            self.life.roam = None;
            if !self.env.dragging {
                self.win_move = Some(self.roam_home());
            }
            return;
        }
        let (x, _y) = win;
        let u = self.u;
        if rm.mode == "hop" {
            let e = (now - rm.t0) / 0.9;
            if e >= 1.0 {
                self.win_move = Some((rm.tx, rm.ty));
                self.roam_go_home(now, (0.4, 1.0), (rm.tx, rm.ty));
                return;
            }
            let nx = rm.sx as f64 + (rm.tx - rm.sx) as f64 * e;
            let ny = rm.sy as f64 + (rm.ty - rm.sy) as f64 * e - (std::f64::consts::PI * e).sin() * 90.0 * u;
            self.win_move = Some((nx as i32, ny as i32));
            st.state = if e < 0.5 { "boop2".into() } else { "giggle".into() };
            return;
        }
        if rm.mode == "chase" {
            let (l, _, r, _) = self.env.work;
            let fw = self.w - self.colw;
            rm.tx = l.max((r - self.w).min(pointer.0 - self.colw - fw.div_euclid(2)));
            self.life.roam = Some(rm.clone());
            if now > rm.until {
                self.set_mood(now, "happy", 2.0);
                self.roam_go_home(now, (0.8, 1.6), win);
                return;
            }
        }
        if rm.mode == "walk" && now < rm.start {
            if let Some(r) = &mut self.life.roam {
                r.t0 = now;
            }
            return;
        }
        let dx = rm.tx - x;
        let step = 2.max((3.0 * u) as i32) * if rm.mode == "chase" { 2 } else { 1 };
        if dx.abs() <= step {
            self.win_move = Some((rm.tx, rm.ty));
            if rm.mode == "walk" {
                if self.rand() < 0.3 {
                    let kind = if self.rand() < 0.5 { "yawn" } else { "tail" };
                    self.play = Some(Play { kind: kind.into(), t0: now, dur: 1.6, arg: None, found: false });
                }
                self.roam_go_home(now, (1.5, 4.0), (rm.tx, rm.ty));
            }
            return;
        }
        // ease in for the first steps and ease out near the end, so she doesn't slide
        let run = now - rm.t0;
        let ease = (0.35 + run / 0.6).min(1.0) * (0.4 + dx.abs() as f64 / (40.0 * u)).min(1.0);
        let mv = 1.max((step as f64 * ease).round_ties_even() as i32);
        let nx = x + if dx > 0 { mv } else { -mv };
        self.win_move = Some((nx, rm.ty));
        self.walk_pose(now, st, if dx > 0 { 1 } else { -1 }, rm.mode == "chase");
    }

    /// zenko19 walk_pose: a four-legged lateral-sequence walk, facing where she's going.
    pub fn walk_pose(&self, now: f64, st: &mut Pose, d: i32, fast: bool) {
        let frame = (now / if fast { 0.075 } else { 0.10 }) as i64 % 8;
        st.state = format!("qwalk{}{}", if d < 0 { "L" } else { "R" }, frame);
        st.hx = 0;
        st.ear = 0;
        st.dyf = 0;
        st.look = (0, 0, 0, 0);
    }

    fn z19_can_pose(&self, now: f64) -> bool {
        self.cfg.b("fox_poses", true) && self.session.is_none() && self.brk.is_none() && !self.lockin_active(now)
            && self.play.is_none() && self.sulk_until == 0.0 && self.life.roam.is_none() && !self.env.dragging
            && !self.sleeping && self.current_mood(now) == "idle"
    }

    pub fn z19_pose_start(&mut self, now: f64, name: &str, say: bool, win: (i32, i32)) -> bool {
        let Some((_, steps)) = ROUTINES.iter().find(|(k, _)| *k == name) else { return false };
        let x = win.0 + self.colw + (self.w - self.colw).div_euclid(2);
        let left = x as f64 > self.env.screen_w as f64 * 0.5;
        let mut seq = vec![];
        let mut t = now;
        for &(p, d) in steps.iter() {
            let d = if p == "doze" { self.uniform(10.0, 22.0) } else { d };
            seq.push((p, t, t + d));
            t += d;
        }
        self.life.z19_seq = Some(Routine { seq, left });
        if say && (name == "stretch" || name == "playbow") && self.rand() < 0.5 {
            let lines: [&str; 2] = if name == "stretch" {
                ["Nnnnh~ *big stretch*", "Fuwaa~ stretchy stretch~"]
            } else {
                ["*play bow* Play with me~?", "Kon! *wiggles* Wanna play?"]
            };
            let l = lines[(self.rand() * 2.0) as usize % 2];
            self.say(l, 3.0);
        }
        true
    }

    fn z19_pose(&mut self, now: f64, st: &mut Pose, win: (i32, i32)) {
        if self.life.z19_seq.is_none() {
            let Some(nxt) = self.life.z19_next else {
                self.life.z19_next = Some(now + self.uniform(90.0, 200.0));
                return;
            };
            if now >= nxt && self.z19_can_pose(now) && !st.state.starts_with("qwalk") && !st.state.starts_with("rig") {
                let live = match self.cfg.s("idle_lively", "normal").as_str() {
                    "calm" => 0.6,
                    "lively" => 1.6,
                    _ => 1.0,
                };
                self.life.z19_next = Some(now + self.uniform(170.0, 420.0) / live);
                let pool: Vec<&str> =
                    ROUTINE_WEIGHTS.iter().flat_map(|(k, w)| std::iter::repeat_n(*k, *w)).collect();
                let k = pool[((self.rand() * pool.len() as f64) as usize).min(pool.len() - 1)];
                self.z19_pose_start(now, k, true, win);
            } else {
                return;
            }
        }
        let mo = self.mood_override.as_ref().is_some_and(|(m, u)| now < *u && m != "idle" && m != "happy");
        if self.session.is_some() || self.brk.is_some() || self.lockin_active(now) || self.play.is_some()
            || self.env.dragging || self.life.roam.is_some() || mo
        {
            self.life.z19_seq = None;
            return;
        }
        let seq = self.life.z19_seq.clone().unwrap();
        let Some(&(p, a, _b)) = seq.seq.iter().find(|(_, a, b)| *a <= now && now < *b) else {
            self.life.z19_seq = None;
            return;
        };
        let fr = if ["playbow", "lie", "doze", "crouch", "sniff"].contains(&p) {
            ((now - a) / if ["playbow", "sniff", "crouch"].contains(&p) { 0.16 } else { 0.5 }) as i64 % 8
        } else if p == "shake" {
            ((now - a) / 0.09) as i64 % 2
        } else {
            0
        };
        let dyf = pose_dyf(p).max(-((self.fox_y as f64 / 1.max(self.z) as f64) as i32));
        st.state = format!("rig{}:{}:{}", if seq.left { "L" } else { "R" }, p, fr);
        st.hx = 0;
        st.ear = 0;
        st.look = (0, 0, 0, 0);
        st.dyf = dyf;
    }

    /// life_pose (ZenkoLife + Zenko19): roaming, then the idle routines.
    pub fn life_pose(&mut self, now: f64, st: &mut Pose, win: (i32, i32), pointer: (i32, i32)) {
        self.roam_step(now, st, win, pointer);
        self.z19_pose(now, st, win);
    }

    /// on_release after a drag: stop roaming and stay put for five minutes.
    pub fn dragged(&mut self, now: f64) {
        self.life.roam = None;
        self.life.roam_pause_until = now + 300.0;
    }

    /// "go for a walk" (chat command): roam as soon as she can.
    pub fn go_roam(&mut self) {
        self.life.roam_next = 0.0;
        self.life.roam_pause_until = 0.0;
    }
}
