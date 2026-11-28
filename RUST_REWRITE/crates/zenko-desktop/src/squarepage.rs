//! The hub's Town square page (zenko.py Zenko22 hub_build_square + paint, Zenko23's festival street Z23Street):
//! the status row (online dot, your nickname and tag, Go online / Leave), the festival street above the board with
//! its crowd, named people for whoever chatted lately, speech bubbles, lantern flicker, fireworks and the season's
//! air, the message board (a Text), the message entry with its counter, and the safety note.

use crate::canvasdraw::text_bbox;
use crate::chatpage::scrollbar_draw;
use crate::gardenpage::zoom_px;
use crate::hubwin::{canvas_of, hub_title_draw};
use crate::pagekit::{Faces, WrapState, hub_card, tt_btn, zk_fonts, zk_wrap};
use crate::squareart::{Look, StreetLay, fest, fest_next, hair, hair_n, look, person_px, square_tag, stall, street, yukata,
                       yukata_n, SQUARE_MAX, SQUARE_SLOW};
use crate::theme::Theme;
use crate::tkcanvas::Photo;
use crate::tktext::{Justify, TagCfg, Text, Wrap};
use crate::ui::{Anchor, Fill, Font, Fonts, Kind, Pack, Place, Side, Ui};
use std::collections::{HashMap, HashSet};
use std::rc::Rc;
use zenko_art::palette::mixhex;
use zenko_sound::mt::PyRandom;

pub const TITLE: &str = "Town square";
pub const SUB: &str = "One friendly chat room for everyone using Zenko. Online only, nicknames only";
pub const NOTE: &str = "Be kind. Links, phone numbers, emails and social handles are removed. Never share where you live or go \
to school. Right-click a message to report or block. Three reports hide it for everyone.";

/// a message on the square (SquareLink.msgs), with the sender's festival look when they sent one
#[derive(Clone, Debug, PartialEq)]
pub struct Msg {
    pub id: String,
    pub uid: String,
    pub name: String,
    pub text: String,
    pub ts: f64,
    /// (yukata, mask, hair)
    pub look: Option<(i64, bool, i64)>,
}

/// what the page shows
#[derive(Clone, Debug, Default)]
pub struct SquareData {
    pub on: bool,
    /// the link's state: off, connecting, live, error
    pub state: String,
    pub uid: String,
    pub name: String,
    /// square_visible() minus what you reported (cfg square_hidden)
    pub msgs: Vec<Msg>,
    /// SquareLink.changed
    pub changed: i64,
    /// cfg z23_yukata / z23_mask
    pub yukata: i64,
    pub mask: bool,
    /// now (unix seconds), the local offset and the local day / hour
    pub now: f64,
    pub offset: i64,
    pub day: i64,
    pub hour: u32,
    /// the festival a preview shows instead of today's
    pub preview: Option<String>,
    pub jp: String,
}

impl SquareData {
    pub fn ev(&self) -> String {
        self.preview.clone().unwrap_or_else(|| crate::squareart::fest_today(self.day).to_string())
    }
    pub fn mode(&self) -> &'static str {
        crate::squareart::sky_mode(self.hour)
    }
    /// z23_my_look(): your own fox in the crowd (orange hair and ears, your yukata, the mask if you wear it)
    pub fn my_look(&self) -> Look {
        let mut l = look(&mut PyRandom::new_str(&self.uid), &crate::squareart::fest_today(self.day), true);
        l.y = yukata(self.yukata.rem_euclid(yukata_n() as i64) as usize);
        l.acc = self.mask.then(|| "mask".to_string());
        l
    }
}

fn hm(ts: f64, off: i64) -> (i64, String) {
    let s = ts.floor() as i64 + off;
    (s.div_euclid(86400), format!("{:02}:{:02}", s.rem_euclid(86400) / 3600, s.rem_euclid(3600) / 60))
}

/// "%a %d %b" of a day number
fn day_label(day: i64) -> String {
    let (_, m, d) = crate::civil_from_days_pub(day);
    let wd = ["Thu", "Fri", "Sat", "Sun", "Mon", "Tue", "Wed"][day.rem_euclid(7) as usize];
    let mon = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"][(m - 1) as usize];
    format!("{wd} {d:02} {mon}")
}

// ---- the festival street ---------------------------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub struct Person {
    pub x: f64,
    pub dir: i64,
    pub lane: usize,
    pub speed: f64,
    pub look: Look,
    pub stop: i64,
    pub name: Option<String>,
    pub uid: Option<String>,
    pub bub: Option<String>,
    pub bub_t: f64,
    pub dance: Option<f64>,
    pub y: f64,
    pub id: u32,
    pub wire: Option<(i64, bool, i64)>,
    pub mine: bool,
    pub tag_ids: [u32; 2],
}

#[derive(Clone, Debug)]
struct Part {
    fw: bool,
    kind: String,
    x: f64,
    y: f64,
    vx: f64,
    vy: f64,
    life: f64,
    dec: f64,
    g: f64,
    c: String,
    id: u32,
    sz: f64,
    ph: f64,
    r: f64,
}

#[derive(Clone, Debug)]
struct Bat {
    x: f64,
    y: f64,
    v: f64,
    id: u32,
}

/// Z23Street
pub struct Street {
    pub cv: usize,
    pub ev: String,
    pub mode: String,
    pub lay: Option<StreetLay>,
    pub aw: i64,
    pub ah: i64,
    img: Option<Rc<Photo>>,
    pub people: Vec<Person>,
    /// uid -> index in people
    pub named: Vec<(String, usize)>,
    parts: Vec<Part>,
    bats: Vec<Bat>,
    lights: Vec<(u32, String, String)>,
    pub seen: Option<HashSet<String>>,
    pub frame_n: i64,
    pub next_fw: f64,
    pub rnd: PyRandom,
    count_id: u32,
    sprites: HashMap<(String, i32), Rc<Photo>>,
    fu: String,
}

pub const Z: i64 = 2;

impl Street {
    pub fn new(ui: &mut Ui, parent: usize, height: i32, d: &SquareData, fu: &str, seed: u64) -> Street {
        let cv = ui.canvas(parent, "#120c18", 0, height);
        canvas_of(ui, cv);
        Street { cv, ev: d.ev(), mode: d.mode().into(), lay: None, aw: 0, ah: 0, img: None, people: vec![], named: vec![],
                 parts: vec![], bats: vec![], lights: vec![], seen: None, frame_n: 0, next_fw: d.now + 2.0, rnd: PyRandom::new(seed),
                 count_id: 0, sprites: HashMap::new(), fu: fu.into() }
    }

    /// the street's state started over with its random stream seeded (as the golden recorder does)
    pub fn reset(&mut self, seed: u64) {
        (self.rnd, self.named, self.seen, self.people, self.parts, self.frame_n, self.aw) =
            (PyRandom::new(seed), vec![], None, vec![], vec![], 0, 0);
        self.next_fw = 1e18;
    }

    fn sprite(&mut self, l: &Look, frame: i32) -> Rc<Photo> {
        let k = (l.key(), frame);
        if let Some(p) = self.sprites.get(&k) {
            return p.clone();
        }
        if self.sprites.len() > 400 {
            self.sprites.clear();
        }
        let p = Rc::new(zoom_px(13, 26, &person_px(l, frame), Z as usize));
        self.sprites.insert(k, p.clone());
        p
    }

    fn named_ix(&self, uid: &str) -> Option<usize> {
        self.named.iter().find(|n| n.0 == uid).map(|n| n.1)
    }

    /// build(): the painting made for the canvas's size (and the sky / festival now), the lantern lights, stall signs,
    /// the crowd and the named people again
    pub fn build(&mut self, ui: &mut Ui, fonts: &dyn Fonts, d: &SquareData) {
        let (cw, ch) = (ui.ws[self.cv].w as i64, ui.ws[self.cv].h as i64);
        if cw < 60 || ch < 60 {
            return;
        }
        let (aw, ah) = (cw / Z + 1, ch / Z + 1);
        if (aw, ah) == (self.aw, self.ah) && self.lay.is_some() && self.mode == d.mode() && self.ev == d.ev() {
            return;
        }
        self.ev = d.ev();
        self.mode = d.mode().into();
        let (r, lay) = street(aw, ah, &self.ev, &self.mode, d.day + 719163);
        (self.aw, self.ah) = (aw, ah);
        let img = Rc::new(zoom_px(r.w as usize, r.h as usize, &r.rgba(), Z as usize));
        self.img = Some(img.clone());
        let zf = Z as f64;
        let fu = self.fu.clone();
        {
            let cv = canvas_of(ui, self.cv);
            cv.delete("all");
            cv.image(0.0, 0.0, img).anchor("nw").tags(&["bg"]);
        }
        self.parts.clear();
        let lit = self.mode != "day";
        self.lights.clear();
        let hx = |c: (u8, u8, u8), k: f64, j: f64| -> String {
            let f = |v: u8| ((v as f64 * k + 255.0 * j) as i64).min(255);
            format!("#{:02x}{:02x}{:02x}", f(c.0), f(c.1), f(c.2))
        };
        {
            let cv = canvas_of(ui, self.cv);
            for &(lx, ly, col) in &lay.lanterns {
                if !lit {
                    break;
                }
                let a = hx(col, 0.4, 0.6);
                cv.rect((lx - 1.0) * zf, (ly + 2.0) * zf, (lx + 2.0) * zf, (ly + 5.0) * zf).fill(&a).outline("").tags(&["lit"]);
                self.lights.push((cv.last_id(), a, hx(col, 0.7, 0.3)));
            }
            for (tx, ty, col) in &lay.twinkles {
                cv.rect(tx * zf, ty * zf, (tx + 1.0) * zf, (ty + 1.0) * zf).fill(col).outline("").tags(&["lit"]);
                self.lights.push((cv.last_id(), col.clone(), mixhex(col, "#1a1020", 0.6)));
            }
            for s in &lay.stalls {
                let sign = stall(&s.key)[0].as_str().unwrap_or("").to_string();
                let n = sign.chars().count();
                let size = if n <= 3 { 9 } else if n <= 4 { 8 } else { 7 };
                let latin = sign.chars().next().is_some_and(|c| c.is_ascii_alphabetic());
                cv.text((s.x0 + s.x1) as f64 / 2.0 * zf, s.sign_y as f64 * zf, &sign).fill("#3a2414")
                    .font(Font::new(if latin { &fu } else { &d.jp }, size, true)).tags(&["sign"]);
            }
        }
        self.lay = Some(lay);
        let old: Vec<Person> = self.named.iter().map(|n| self.people[n.1].clone()).collect();
        self.people.clear();
        self.named.clear();
        let n = 6.max(26.min(aw / 24));
        for _ in 0..n {
            let x = self.rnd.uniform(0.0, aw as f64);
            self.spawn(ui, Some(x), None, None, None, None);
        }
        if self.ev == "obon" {
            let mid = self.lay.as_ref().unwrap().mid as f64;
            for k in 0..6 {
                let i = self.spawn(ui, Some(mid), None, None, None, None);
                self.people[i].dance = Some(k as f64 * 6.283 / 6.0);
            }
        }
        let me = d.uid.clone();
        let myname = if d.name.is_empty() { "you".to_string() } else { d.name.clone() };
        self.person_for(ui, d, &me, &myname, None, true);
        for p in old {
            if let Some(u) = &p.uid
                && *u != me
            {
                self.person_for(ui, d, u, p.name.as_deref().unwrap_or(""), p.wire, false);
            }
        }
        self.banner(ui, d);
        let cv = canvas_of(ui, self.cv);
        for t in ["p0", "p1", "named", "tag", "bub", "banner"] {
            cv.tag_raise(t);
        }
        let _ = fonts;
    }

    fn banner(&mut self, ui: &mut Ui, d: &SquareData) {
        let w = ui.ws[self.cv].w as f64;
        let fu = self.fu.clone();
        let cv = canvas_of(ui, self.cv);
        cv.delete("banner");
        let f = fest(&self.ev);
        for (dx, dy, col) in [(1.0, 1.0, "#1a1020"), (0.0, 0.0, "#ffe7b0")] {
            cv.text(14.0 + dx, 12.0 + dy, f[0].as_str().unwrap_or("")).anchor("nw").fill(col).font(Font::new(&d.jp, 15, true))
                .tags(&["banner"]);
        }
        let mut sub = f[1].as_str().unwrap_or("").to_string();
        if d.preview.is_some() {
            sub += " (preview, right-click to go back)";
        } else if let Some((k, a)) = fest_next(d.day) {
            let days = a - d.day;
            let nm = fest(k)[1].as_str().unwrap_or("").split(',').next().unwrap_or("").to_string();
            sub += &format!("   \u{b7}   next: {nm} in {days} day{}", if days == 1 { "" } else { "s" });
        }
        for (dx, dy, col) in [(1.0, 1.0, "#1a1020"), (0.0, 0.0, "#f4ead8")] {
            cv.text(15.0 + dx, 42.0 + dy, &sub).anchor("nw").fill(col).font(Font::new(&fu, 9, false)).tags(&["banner"]);
        }
        cv.text(w - 14.0, 14.0, "").anchor("ne").fill("#f4ead8").font(Font::new(&fu, 9, false)).tags(&["banner"]);
        self.count_id = cv.last_id();
    }

    /// spawn(x, look, lane, name, uid): a person on a lane; returns its index
    fn spawn(&mut self, ui: &mut Ui, x: Option<f64>, lk: Option<Look>, lane: Option<usize>, name: Option<String>, uid: Option<String>)
             -> usize {
        let lane = lane.unwrap_or_else(|| self.rnd.randint(0, 1) as usize);
        let dir = *self.rnd.choice(&[-1i64, 1]);
        let x = x.unwrap_or(if dir > 0 { -12.0 } else { self.aw as f64 + 1.0 });
        let speed = self.rnd.uniform(0.35, 0.8);
        let ev = self.ev.clone();
        let lk = lk.unwrap_or_else(|| look(&mut self.rnd, &ev, false));
        let lanes = self.lay.as_ref().unwrap().lanes;
        let y = (if lane == 0 { lanes.0 } else { lanes.1 } + self.rnd.randint(-1, 1)) as f64;
        let img = self.sprite(&lk, 0);
        let lt = format!("p{lane}");
        let cv = canvas_of(ui, self.cv);
        cv.image(((x * Z as f64) as i64) as f64, (((y - 26.0) * Z as f64) as i64) as f64, img).anchor("nw")
            .tags(&[&lt, if name.is_some() { "named" } else { "npc" }]);
        let id = cv.last_id();
        self.people.push(Person { x, dir, lane, speed, look: lk, stop: 0, name, uid, bub: None, bub_t: 0.0, dance: None, y, id,
                                  wire: None, mine: false, tag_ids: [0, 0] });
        self.people.len() - 1
    }

    fn look_from_wire(&self, uid: &str, wire: Option<(i64, bool, i64)>) -> Look {
        let mut l = look(&mut PyRandom::new_str(uid), &self.ev, false);
        l.ghost = false;
        if let Some((y, m, h)) = wire {
            l.y = yukata(y.rem_euclid(yukata_n() as i64) as usize);
            l.h = hair(h.rem_euclid(hair_n() as i64) as usize);
            l.acc = m.then(|| "mask".to_string());
        }
        l
    }

    /// person_for(uid, name, wire, mine): the named person for someone who chatted (made the first time)
    pub fn person_for(&mut self, ui: &mut Ui, d: &SquareData, uid: &str, name: &str, wire: Option<(i64, bool, i64)>, mine: bool)
                      -> usize {
        if let Some(i) = self.named_ix(uid) {
            if wire.is_some() && wire != self.people[i].wire {
                self.people[i].wire = wire;
                self.people[i].look = self.look_from_wire(uid, wire);
            }
            return i;
        }
        let lk = if mine { d.my_look() } else { self.look_from_wire(uid, wire) };
        let aw = self.aw as f64;
        let x = if mine { aw * (0.42 + self.rnd.random() * 0.16) } else { self.rnd.uniform(aw * 0.1, aw * 0.9) };
        let i = self.spawn(ui, Some(x), Some(lk), Some(1), Some(name.into()), Some(uid.into()));
        let zf = Z as f64;
        let fu = self.fu.clone();
        let p = &mut self.people[i];
        p.wire = wire;
        p.mine = mine;
        if mine {
            p.speed = 0.3;
        }
        let cx = (p.x + 6.5) * zf;
        let ty = (p.y - 27.0) * zf;
        let fg = if mine { "#ffb066" } else { "#ffe7b0" };
        let cv = canvas_of(ui, self.cv);
        cv.text(cx + 1.0, ty + 1.0, name).fill("#1a1020").font(Font::new(&fu, 8, true)).tags(&["tag"]);
        let a = cv.last_id();
        cv.text(cx, ty, name).fill(fg).font(Font::new(&fu, 8, true)).tags(&["tag"]);
        p.tag_ids = [a, cv.last_id()];
        self.named.push((uid.into(), i));
        for t in ["tag", "bub", "banner"] {
            cv.tag_raise(t);
        }
        i
    }

    /// restyle_me(): your look changed (yukata, mask)
    pub fn restyle_me(&mut self, d: &SquareData) {
        if let Some(i) = self.named_ix(&d.uid) {
            self.people[i].look = d.my_look();
        }
    }

    /// say(p, text): a speech bubble over a named person for 9 s (floated above anyone else's)
    pub fn say(&mut self, ui: &mut Ui, fonts: &dyn Fonts, i: usize, text: &str, now: f64) {
        let w = ui.ws[self.cv].w as f64;
        let zf = Z as f64;
        let fu = self.fu.clone();
        let others: Vec<String> = self.people.iter().enumerate().filter(|(k, o)| *k != i && o.bub.is_some())
            .map(|(_, o)| o.bub.clone().unwrap()).collect();
        let p = &mut self.people[i];
        let cv = canvas_of(ui, self.cv);
        if let Some(b) = &p.bub {
            cv.delete(b);
        }
        let tag = format!("bub{}", p.id);
        p.bub = Some(tag.clone());
        p.bub_t = now + 9.0;
        let cx = (p.x + 6.5) * zf;
        let top = (p.y - 30.0) * zf;
        let clipped = crate::extra::clip(text, 90);
        cv.text(cx, top - 6.0, &clipped).wrap(170.0).fill("#2a1a10").font(Font::new(&fu, 8, false)).anchor("s").tags(&[&tag, "bub"]);
        let t = cv.last_id();
        let bb = text_bbox(cv.get(t).unwrap(), fonts);
        let (x0, y0, x1, y1) = ((bb[0] - 7) as f64, (bb[1] - 4) as f64, (bb[2] + 7) as f64, (bb[3] + 4) as f64);
        let r = 6.0;
        let pts = vec![x0 + r, y0, x1 - r, y0, x1, y0, x1, y0 + r, x1, y1 - r, x1, y1, x1 - r, y1, cx + 5.0, y1, cx, y1 + 6.0, cx - 5.0,
                       y1, x0 + r, y1, x0, y1, x0, y1 - r, x0, y0 + r, x0, y0];
        cv.poly(pts).smooth().fill("#fff6e6").outline("#5a3a22").width(1.0).tags(&[&tag, "bub"]);
        let b = cv.last_id();
        cv.tag_raise_above(&t.to_string(), &b.to_string());
        let shift = if x0 < 4.0 { 4.0 - x0 } else if x1 > w - 4.0 { w - 4.0 - x1 } else { 0.0 };
        if shift != 0.0 {
            cv.move_items(&t.to_string(), shift, 0.0);
            cv.move_items(&b.to_string(), shift, 0.0);
        }
        for _ in 0..4 {
            let bbx = crate::canvasdraw::item_bbox(cv, b, fonts);
            let mut hit = None;
            for o in &others {
                if let Some(ob) = crate::canvasdraw::tag_bbox(cv, o, fonts)
                    && let Some(bb) = bbx
                    && ob[0] < bb[2] && bb[0] < ob[2] && ob[1] < bb[3] && bb[1] < ob[3]
                {
                    hit = Some(ob);
                    break;
                }
            }
            let (Some(hb), Some(bb)) = (hit, bbx) else { break };
            if hb[1] - (bb[3] - bb[1]) < 2 {
                break;
            }
            let dy = (hb[1] - 3 - bb[3]) as f64;
            cv.move_items(&t.to_string(), 0.0, dy);
            cv.move_items(&b.to_string(), 0.0, dy);
        }
        cv.tag_raise("bub");
        cv.tag_raise("banner");
        p.stop = p.stop.max(40);
    }

    /// feed(): new messages walk their sender in and give them a bubble; the count in the corner
    pub fn feed(&mut self, ui: &mut Ui, fonts: &dyn Fonts, d: &SquareData) {
        let msgs: Vec<Msg> = if d.on { d.msgs.clone() } else { vec![] };
        let now = d.now;
        if self.seen.is_none() {
            self.seen = Some(msgs.iter().map(|m| m.id.clone()).collect());
            let (mut recent, mut got): (Vec<&Msg>, HashSet<&str>) = (vec![], HashSet::new());
            for m in msgs.iter().rev() {
                if !got.contains(m.uid.as_str()) && now - m.ts < 3.0 * 3600.0 {
                    got.insert(&m.uid);
                    recent.push(m);
                }
                if recent.len() >= 7 {
                    break;
                }
            }
            for m in recent.iter().rev() {
                self.person_for(ui, d, &m.uid, &m.name, m.look, m.uid == d.uid);
            }
            if let Some(last) = recent.first() {
                let i = self.named_ix(&last.uid).unwrap();
                self.say(ui, fonts, i, &last.text, now);
            }
        } else {
            for m in &msgs {
                if self.seen.as_ref().unwrap().contains(&m.id) {
                    continue;
                }
                self.seen.as_mut().unwrap().insert(m.id.clone());
                let i = self.person_for(ui, d, &m.uid, &m.name, m.look, m.uid == d.uid);
                self.say(ui, fonts, i, &m.text, now);
            }
        }
        let cutoff = now - 3600.0;
        let n = msgs.iter().filter(|m| m.ts > cutoff).map(|m| m.uid.as_str()).collect::<HashSet<_>>().len();
        let t = if d.on && n > 0 {
            format!("{n} chatting in the last hour")
        } else if d.on {
            "Online".into()
        } else {
            "Offline \u{b7} press Go online to join the crowd".into()
        };
        if let Some(it) = canvas_of(ui, self.cv).get(self.count_id) {
            it.text = t;
        }
    }

    fn firework(&mut self, ui: &mut Ui) {
        let lay = self.lay.as_ref().unwrap();
        let gy = lay.gy as f64;
        let f = fest(&self.ev);
        let mut cols: Vec<String> = f[2].as_array().unwrap().iter().map(|c| c.as_str().unwrap().to_string()).collect();
        cols.extend(["#fff4d0".to_string(), "#ffd27a".to_string()]);
        if self.ev == "halloween" {
            cols = ["#ff8a2a", "#9a5ad8", "#7be07a", "#ffd27a"].map(String::from).to_vec();
        } else if self.ev == "diwali" || self.ev == "holi" {
            cols = ["#ffd23a", "#ff5fa2", "#4fc3ff", "#7be07a", "#ff8a2a", "#ffffff"].map(String::from).to_vec();
        }
        let aw = self.aw as f64;
        let x = self.rnd.uniform(aw * 0.12, aw * 0.88);
        let y = self.rnd.uniform(gy * 0.14, gy * 0.42);
        let mut kinds = vec!["peony", "peony", "ring", "willow"];
        if self.ev == "valentine" {
            kinds.extend(["heart"; 3]);
        }
        let kind = *self.rnd.choice(&kinds);
        let col = self.rnd.choice(&cols).clone();
        let col2 = self.rnd.choice(&cols).clone();
        let n = if kind != "ring" { 30 } else { 22 };
        let sp = self.rnd.uniform(1.1, 1.7);
        for k in 0..n {
            let a = k as f64 * 6.2832 / n as f64;
            let (vx, vy) = if kind == "heart" {
                (16.0 * a.sin().powi(3) / 16.0 * sp,
                 -(13.0 * a.cos() - 5.0 * (2.0 * a).cos() - 2.0 * (3.0 * a).cos() - (4.0 * a).cos()) / 16.0 * sp)
            } else {
                let s = sp * if kind == "ring" { 1.0 } else { self.rnd.uniform(0.55, 1.05) };
                (a.cos() * s, a.sin() * s)
            };
            let mut c = if k % 3 != 0 || kind == "willow" { col.clone() } else { col2.clone() };
            if kind == "willow" {
                c = "#ffcf6a".into();
            }
            let cv = canvas_of(ui, self.cv);
            cv.rect(0.0, 0.0, 0.0, 0.0).fill(&c).outline("").tags(&["fw"]);
            let id = cv.last_id();
            let w = kind == "willow";
            self.parts.push(Part { fw: true, kind: String::new(), x, y, vx, vy, life: 1.0, dec: if w { 0.012 } else { 0.022 },
                                   g: if w { 0.02 } else { 0.03 }, c, id, sz: 1.3, ph: 0.0, r: 0.0 });
        }
        canvas_of(ui, self.cv).tag_lower("fw", Some("lit"));
    }

    fn ambient(&mut self, ui: &mut Ui) {
        let kind = fest(&self.ev)[4].as_str().unwrap_or("").to_string();
        if kind.is_empty() || (kind == "fireflies" && self.mode == "day") {
            return;
        }
        let live = self.parts.iter().filter(|p| !p.fw).count();
        if live >= if kind != "colour" { 26 } else { 10 } {
            return;
        }
        let (aw, ah, gy) = (self.aw as f64, self.ah as f64, self.lay.as_ref().unwrap().gy as f64);
        let cols: &[&str] = match kind.as_str() {
            "snow" => &["#f4f8ff", "#e4ecff"],
            "sakura" => &["#ffc6de", "#ffe0ee", "#ff9fc6"],
            "petals" => &["#ffd0e4", "#fff0f6"],
            "peach" => &["#ff9fc6", "#ffc6de"],
            "leaves" => &["#e0502a", "#f08a2a", "#c8372d", "#f2b33a"],
            "fireflies" => &["#d8f08a", "#f4ff9a"],
            "colour" => &["#ff5fa2", "#ffd23a", "#4fc3ff", "#7be07a"],
            _ => &["#ff7aa8"],
        };
        let c = self.rnd.choice(cols).to_string();
        let r = &mut self.rnd;
        let mut p = Part { fw: false, kind: kind.clone(), x: 0.0, y: 0.0, vx: 0.0, vy: 0.0, life: 1.0, dec: 0.0, g: 0.0, c: c.clone(), id: 0,
                           sz: 0.0, ph: 0.0, r: 0.0 };
        if kind == "fireflies" {
            p.x = r.uniform(0.0, aw);
            p.y = r.uniform(gy - 30.0, ah - 4.0);
            p.vx = r.uniform(-0.15, 0.15);
            p.vy = r.uniform(-0.08, 0.05);
            p.ph = r.random() * 6.0;
        } else if kind == "colour" {
            p.x = r.uniform(0.0, aw);
            p.y = ah - 2.0;
            p.vx = r.uniform(-0.2, 0.2);
            p.vy = -r.uniform(0.15, 0.35);
            p.r = r.uniform(4.0, 9.0);
        } else {
            p.x = r.uniform(-10.0, aw);
            p.y = -2.0;
            p.vx = r.uniform(0.05, 0.35);
            p.vy = if kind == "snow" { r.uniform(0.25, 0.5) } else { r.uniform(0.3, 0.6) };
            p.ph = r.random() * 6.0;
        }
        let cv = canvas_of(ui, self.cv);
        if kind == "colour" {
            cv.oval(0.0, 0.0, 0.0, 0.0).fill(&c).outline("").stipple("gray50").tags(&["amb"]);
        } else {
            cv.rect(0.0, 0.0, 0.0, 0.0).fill(&c).outline("").tags(&["amb"]);
        }
        p.id = cv.last_id();
        cv.tag_raise_above("amb", "p1");
        self.parts.push(p);
    }

    /// puffs(): Holi's colour clouds from the gulal stall
    pub fn puffs(&mut self, ui: &mut Ui) {
        let Some(lay) = &self.lay else { return };
        let gy = lay.gy as f64;
        for _ in 0..8 {
            let r = &mut self.rnd;
            let c = r.choice(&["#ff5fa2", "#ffd23a", "#4fc3ff", "#7be07a", "#b07aff"]).to_string();
            let x = self.aw as f64 * r.uniform(0.3, 0.7);
            let vx = r.uniform(-0.6, 0.6);
            let vy = -r.uniform(0.3, 0.7);
            let rr = r.uniform(5.0, 11.0);
            let cv = canvas_of(ui, self.cv);
            cv.oval(0.0, 0.0, 0.0, 0.0).fill(&c).outline("").stipple("gray50").tags(&["amb"]);
            self.parts.push(Part { fw: false, kind: "colour".into(), x, y: gy + 10.0, vx, vy, life: 1.0, dec: 0.0, g: 0.0, c,
                                   id: cv.last_id(), sz: 0.0, ph: 0.0, r: rr });
        }
    }

    /// step(): one 90 ms frame. Returns true when the street wants building again (the sky or festival changed).
    pub fn step(&mut self, ui: &mut Ui, fonts: &dyn Fonts, d: &SquareData) -> bool {
        if self.lay.is_none() {
            return false;
        }
        self.frame_n += 1;
        let fn_ = self.frame_n;
        let now = d.now;
        if fn_ % 11 == 1 {
            self.feed(ui, fonts, d);
        }
        if fn_ % 600 == 0 && (self.mode != d.mode() || self.ev != d.ev()) {
            return true;
        }
        let zf = Z as f64;
        if !self.lights.is_empty() && fn_ % 3 == 0 {
            for _ in 0..1.max(self.lights.len() / 6) {
                let (i, a, b) = self.rnd.choice(&self.lights).clone();
                let flick = self.rnd.random() < 0.35;
                if let Some(it) = canvas_of(ui, self.cv).get(i) {
                    it.fill = if flick { b } else { a };
                }
            }
        }
        let aw = self.aw as f64;
        let mid = self.lay.as_ref().unwrap().mid as f64;
        let ev = self.ev.clone();
        for k in 0..self.people.len() {
            let mut dx: Option<f64>;
            let walking;
            {
                let p = &mut self.people[k];
                if let Some(dn) = p.dance {
                    let dn = dn + 0.035;
                    p.dance = Some(dn);
                    let nx = mid - 6.0 + dn.cos() * 26.0;
                    p.dir = if nx > p.x { 1 } else { -1 };
                    dx = Some(nx - p.x);
                    p.x = nx;
                    walking = true;
                } else if p.stop > 0 {
                    p.stop -= 1;
                    dx = Some(0.0);
                    walking = false;
                } else {
                    let v = p.dir as f64 * p.speed;
                    p.x += v;
                    dx = Some(v);
                    walking = true;
                }
            }
            if self.people[k].dance.is_none() && walking {
                let mine = self.people[k].mine;
                if self.rnd.random() < if !mine { 0.006 } else { 0.012 } {
                    self.people[k].stop = self.rnd.randint(20, 70);
                }
                let p = &mut self.people[k];
                if mine && (p.x - aw * 0.5).abs() > aw * 0.3 && (p.x - aw * 0.5) * p.dir as f64 > 0.0 {
                    p.dir = -p.dir;
                }
                if p.x < -14.0 || p.x > aw + 2.0 {
                    if p.name.is_some() {
                        p.dir = -p.dir;
                        p.x = (-13.0f64).max((aw + 1.0).min(p.x));
                    } else {
                        let d2 = *self.rnd.choice(&[-1i64, 1]);
                        let p = &mut self.people[k];
                        p.dir = d2;
                        p.x = if d2 > 0 { -12.0 } else { aw + 1.0 };
                        let lk = look(&mut self.rnd, &ev, false);
                        self.people[k].look = lk;
                        dx = None;
                    }
                }
            }
            let fr = if walking { ((fn_ / 3) % 2) as i32 } else { 0 };
            let lk = self.people[k].look.clone();
            let img = self.sprite(&lk, fr);
            let p = &mut self.people[k];
            let bob = if walking && fr == 1 { zf } else { 0.0 };
            let cv = canvas_of(ui, self.cv);
            if let Some(it) = cv.get(p.id) {
                it.image = Some(img);
                it.coords = vec![((p.x * zf) as i64) as f64, (((p.y - 26.0) * zf) as i64) as f64 - bob];
            }
            if p.name.is_some() {
                let cx = (p.x + 6.5) * zf;
                let ty = (p.y - 27.0) * zf - bob;
                if let Some(it) = cv.get(p.tag_ids[0]) {
                    it.coords = vec![cx + 1.0, ty + 1.0];
                }
                if let Some(it) = cv.get(p.tag_ids[1]) {
                    it.coords = vec![cx, ty];
                }
                if let Some(b) = p.bub.clone() {
                    if now > p.bub_t {
                        cv.delete(&b);
                        p.bub = None;
                    } else if let Some(v) = dx
                        && v != 0.0
                    {
                        cv.move_items(&b, v * zf, 0.0);
                    }
                }
            }
        }
        let f = fest(&self.ev);
        let rate = f[5].as_f64().unwrap_or(0.0) * match self.mode.as_str() {
            "night" => 1.0,
            "dusk" => 0.6,
            _ => 0.25,
        };
        if now > self.next_fw && rate > 0.0 {
            self.firework(ui);
            self.next_fw = now + self.rnd.expovariate(rate / 60.0) + 0.6;
        }
        if fn_ % 4 == 0 {
            self.ambient(ui);
        }
        let sky = self.lay.as_ref().unwrap().sky.clone();
        let ah = self.ah as f64;
        let mut alive = vec![];
        let parts = std::mem::take(&mut self.parts);
        for mut q in parts {
            q.x += q.vx;
            q.y += q.vy;
            let cv = canvas_of(ui, self.cv);
            if q.fw {
                q.vy += q.g;
                q.vx *= 0.97;
                q.vy *= 0.985;
                q.life -= q.dec;
                let xi = q.x as i64;
                let hide = q.life <= 0.0 || xi < 0 || xi >= self.aw || q.y >= sky[xi.clamp(0, self.aw - 1) as usize] as f64 - 1.0;
                if q.life <= 0.0 {
                    cv.delete(&q.id.to_string());
                    continue;
                }
                let s = q.sz * zf;
                if let Some(it) = cv.get(q.id) {
                    if hide {
                        it.coords = vec![0.0; 4];
                    } else {
                        it.coords = vec![q.x * zf, q.y * zf, q.x * zf + s, q.y * zf + s];
                        if fn_ % 2 == 0 && q.life < 0.6 {
                            it.fill = mixhex(&q.c, "#2a1830", 1.0 - q.life / 0.6);
                        }
                    }
                }
                alive.push(q);
                continue;
            }
            q.ph += 0.08;
            match q.kind.as_str() {
                "fireflies" => {
                    q.vx += self.rnd.uniform(-0.02, 0.02);
                    q.vy += self.rnd.uniform(-0.015, 0.015);
                    q.vx = q.vx.clamp(-0.25, 0.25);
                    q.vy = q.vy.clamp(-0.15, 0.15);
                    q.life -= 0.004;
                    let on = (q.ph * 0.6).sin() > 0.2;
                    if let Some(it) = cv.get(q.id) {
                        it.coords = vec![q.x * zf, q.y * zf, (q.x + 1.0) * zf, (q.y + 1.0) * zf];
                        it.hidden = !on;
                    }
                }
                "colour" => {
                    q.life -= 0.012;
                    let r = q.r * (1.6 - q.life * 0.6) * zf;
                    if let Some(it) = cv.get(q.id) {
                        it.coords = vec![q.x * zf - r, q.y * zf - r, q.x * zf + r, q.y * zf + r];
                    }
                }
                kind => {
                    q.x += q.ph.sin() * if kind != "snow" { 0.25 } else { 0.12 };
                    let sz = if kind == "leaves" { zf * 1.5 } else { zf };
                    let w2 = sz * if kind == "snow" { 1.0 } else if (q.ph * 1.7).sin() > 0.0 { 1.6 } else { 0.8 };
                    if let Some(it) = cv.get(q.id) {
                        it.coords = vec![q.x * zf, q.y * zf, q.x * zf + w2, q.y * zf + sz];
                    }
                    if q.y > ah - self.rnd.uniform(0.0, 8.0) {
                        q.life = 0.0;
                    }
                }
            }
            if q.life <= 0.0 || q.x < -20.0 || q.x > aw + 20.0 || q.y < -20.0 {
                canvas_of(ui, self.cv).delete(&q.id.to_string());
                continue;
            }
            alive.push(q);
        }
        self.parts = alive;
        if self.ev == "halloween" && fn_ % 2 == 0 {
            self.bats(ui, fn_);
        }
        false
    }

    fn bats(&mut self, ui: &mut Ui, fn_: i64) {
        let zf = Z as f64;
        if self.bats.len() < 3 && self.rnd.random() < 0.02 {
            let y = self.rnd.uniform(8.0, self.lay.as_ref().unwrap().gy as f64 * 0.4);
            let v = self.rnd.uniform(0.5, 0.9);
            let cv = canvas_of(ui, self.cv);
            cv.line(vec![0.0; 6]).fill("#1a1020").width(zf).tags(&["fw"]);
            self.bats.push(Bat { x: -6.0, y, v, id: cv.last_id() });
        }
        let aw = self.aw as f64;
        let cv = canvas_of(ui, self.cv);
        self.bats.retain_mut(|b| {
            b.x += b.v * 2.0;
            b.y += (b.x / 6.0).sin() * 0.6;
            let flap = if (fn_ / 2) % 2 == 1 { 2.0 } else { -1.0 };
            let (x, y) = (b.x * zf, b.y * zf);
            if let Some(it) = cv.get(b.id) {
                it.coords = vec![x - 4.0 * zf, y - flap * zf, x, y, x + 4.0 * zf, y - flap * zf];
            }
            if b.x > aw + 8.0 {
                cv.delete(&b.id.to_string());
                false
            } else {
                true
            }
        });
    }

    /// at(e): the stall under a point on the canvas
    pub fn at(&self, x: f64, y: f64) -> Option<String> {
        let lay = self.lay.as_ref()?;
        let (ax, ay) = (x / Z as f64, y / Z as f64);
        lay.stalls.iter().find(|s| (s.x0 - 1) as f64 <= ax && ax <= (s.x1 + 1) as f64 && (s.top - 11) as f64 <= ay && ay <= lay.gy as f64)
            .map(|s| s.key.clone())
    }
}

// ---- the page --------------------------------------------------------------------------------------------------------

pub struct SquarePage {
    pub pg: usize,
    pub title: usize,
    pub wrap_f: usize,
    pub dot: usize,
    pub st: usize,
    pub ne: usize,
    pub tag: usize,
    pub go: usize,
    pub board: usize,
    pub txt: usize,
    pub sb: usize,
    pub me: usize,
    pub cnt: usize,
    pub send: usize,
    pub street: Option<Street>,
    pub shown: i64,
    /// text line -> message (for the right-click menu)
    pub lines: Vec<(usize, Msg)>,
    /// when you last sent (slow mode)
    pub last: f64,
    pub wrap: WrapState,
    seig: Option<(String, Rc<Photo>)>,
    th: Theme,
    f: Faces,
    pub d: SquareData,
    pub zk_f: f64,
}

impl SquarePage {
    /// hub_build_square(pg) (Zenko23: the board 7 lines tall and the festival street packed before it); `win_h` is the
    /// hub window's height (the street is 40% of it, 220 to 340 px); `seed` seeds the street's own random stream
    #[allow(clippy::too_many_arguments)]
    pub fn build(ui: &mut Ui, pg: usize, th: &Theme, f: &Faces, cm10: i32, d: &SquareData, win_h: i32, seed: u64) -> SquarePage {
        let title = ui.canvas(pg, &th.banner, cm10, 96);
        canvas_of(ui, title);
        ui.pack(title, Pack::default().fill(Fill::X));
        let bg = th.card.clone();
        let fu = f.ui.clone();
        let wrap_f = ui.frame(pg, &th.paper);
        ui.pack(wrap_f, Pack::default().fill(Fill::Both).expand().padx(24, 24).pady(4, 16));
        let top = ui.frame(wrap_f, &th.paper);
        ui.pack(top, Pack::default().fill(Fill::X));
        let c = hub_card(ui, top, None, "", th, f);
        ui.pack(c, Pack::default().fill(Fill::X));
        let row = ui.frame(c, &bg);
        ui.pack(row, Pack::default().fill(Fill::X).padx(14, 14).pady(10, 10));
        let dot = ui.canvas(row, &bg, 14, 14);
        canvas_of(ui, dot);
        ui.pack(dot, Pack::default().side(Side::Left));
        let st = ui.label(row, "", &bg, &th.ink, Font::new(&fu, 10, true));
        ui.pack(st, Pack::default().side(Side::Left).padx(6, 14));
        let l = ui.label(row, "Your name", &bg, &th.grey, Font::new(&fu, 9, false));
        ui.pack(l, Pack::default().side(Side::Left));
        let ne = ui.entry(row, 16, Font::new(&fu, 10, false), &d.name, &th.field);
        ui.ws[ne].hlc = th.line.clone();
        ui.pack(ne, Pack::default().side(Side::Left).padx(6, 4).ipady(3));
        let tag = ui.label(row, &format!("#{}", square_tag(&d.uid)), &bg, &th.grey, Font::new(&fu, 9, false));
        ui.pack(tag, Pack::default().side(Side::Left));
        let go = tt_btn(ui, row, "Go online", "sq_toggle", Some(&th.acc), None, 10, 10, 3, th, f);
        ui.pack(go, Pack::default().side(Side::Right));
        let board = hub_card(ui, wrap_f, None, "", th, f);
        ui.pack(board, Pack::default().fill(Fill::Both).expand().pady(10, 0));
        let mut t = Text::new(Font::new(&fu, 10, false), &th.ink, &bg);
        (t.wrap, t.padx, t.pady, t.hl, t.height_ln, t.spacing1, t.spacing3) = (Wrap::Word, 14, 10, 0, 7, 3, 3);
        let txt = ui.text(board, t, "");
        let sb = ui.canvas(board, &ui.ws[board].bg.clone(), 10, 40);
        canvas_of(ui, sb);
        ui.pack(sb, Pack::default().side(Side::Right).fill(Fill::Y).pady(6, 6));
        ui.pack(txt, Pack::default().side(Side::Left).fill(Fill::Both).expand());
        {
            let tx = ui.text_mut(txt);
            *tx.tag("name") = TagCfg { name: "name".into(), fg: Some(th.gold.clone()), font: Some(Font::new(&fu, 10, true)), ..Default::default() };
            *tx.tag("me") = TagCfg { name: "me".into(), fg: Some(th.acc.clone()), font: Some(Font::new(&fu, 10, true)), ..Default::default() };
            *tx.tag("tag") = TagCfg { name: "tag".into(), fg: Some(th.grey.clone()), font: Some(Font::new(&fu, 8, false)), ..Default::default() };
            *tx.tag("note") = TagCfg { name: "note".into(), fg: Some(th.grey.clone()),
                                       font: Some(Font { family: fu.clone(), size: 9, bold: false, italic: true }),
                                       justify: Some(Justify::Center), ..Default::default() };
        }
        let bottom = ui.frame(wrap_f, &th.paper);
        ui.pack(bottom, Pack::default().fill(Fill::X).pady(10, 0));
        let me = ui.entry(bottom, 20, Font::new(&fu, 11, false), "", &th.field);
        ui.ws[me].hlc = th.line.clone();
        ui.pack(me, Pack::default().side(Side::Left).fill(Fill::X).expand().ipady(6));
        let cnt = ui.label(bottom, "", &th.paper, &th.grey, Font::new(&fu, 9, false));
        ui.ws[cnt].opt_w = 9;
        ui.pack(cnt, Pack::default().side(Side::Left).padx(6, 6));
        let send = tt_btn(ui, bottom, "Send", "sq_send", Some(&th.acc), None, 10, 10, 3, th, f);
        ui.pack(send, Pack::default().side(Side::Left));
        let l = ui.label(wrap_f, NOTE, &th.paper, &th.grey, Font::new(&fu, 8, false));
        ui.set_anchor(l, Anchor::W);
        if let Kind::Label { left, .. } = &mut ui.ws[l].kind {
            *left = true;
        }
        ui.pack(l, Pack::default().fill(Fill::X).pady(6, 0));
        // Zenko23: the festival street, packed before the board
        let h = (220.0f64.max(340.0f64.min((if win_h > 0 { win_h } else { 760 }) as f64 * 0.40))) as i32;
        let street = Street::new(ui, wrap_f, h, d, &fu, seed);
        ui.pack_before(street.cv, board, Pack::default().fill(Fill::X).pady(10, 0));
        let mut p = SquarePage { pg, title, wrap_f, dot, st, ne, tag, go, board, txt, sb, me, cnt, send, street: Some(street), shown: -1,
                                 lines: vec![], last: 0.0, wrap: WrapState::default(), seig: None, th: th.clone(),
                                 f: Faces { ui: f.ui.clone(), title: f.title.clone() }, d: d.clone(), zk_f: 1.0 };
        let _ = &mut p;
        p
    }

    /// the text in the message entry
    pub fn entry_text(&self, ui: &Ui) -> String {
        ui.text_of(self.me)
    }

    /// paint(): the dot, the status, the button, the counter, and the board when the messages changed
    pub fn paint(&mut self, ui: &mut Ui, fonts: &dyn Fonts) {
        let th = self.th.clone();
        let d = self.d.clone();
        let on = d.on;
        let state = if on { d.state.as_str() } else { "off" };
        let col = match state {
            "live" => "#5fd38a".to_string(),
            "connecting" => th.gold.clone(),
            "error" => "#e06a5a".into(),
            _ => th.grey.clone(),
        };
        {
            let cv = canvas_of(ui, self.dot);
            cv.delete("all");
            cv.oval(2.0, 2.0, 12.0, 12.0).fill(&col).outline("");
        }
        let stt = match state {
            "live" => "Online",
            "connecting" => "Connecting...",
            "error" => "Can't reach the square",
            _ => "Offline",
        };
        if let Kind::Label { text, .. } = &mut ui.ws[self.st].kind {
            *text = stt.into();
        }
        if let Kind::Button { text, .. } = &mut ui.ws[self.go].kind {
            *text = if on { "Leave" } else { "Go online" }.into();
        }
        let left = 0.0f64.max(SQUARE_SLOW - (d.now - self.last));
        let n = self.entry_text(ui).chars().count();
        if let Kind::Label { text, .. } = &mut ui.ws[self.cnt].kind {
            *text = if left > 0.0 { format!("wait {}s", left as i64) } else { format!("{n}/{SQUARE_MAX}") };
        }
        let ver = d.changed * 2 + on as i64;
        if ver == self.shown {
            return;
        }
        self.shown = ver;
        let msgs: Vec<Msg> = if on { d.msgs.clone() } else { vec![] };
        let at_end = match &ui.ws[self.txt].kind {
            Kind::Text { t, .. } => t.yview(fonts).1 > 0.98,
            _ => true,
        };
        let mapped = ui.ws[self.txt].h > 1;
        let tx = ui.text_mut(self.txt);
        tx.delete_all();
        self.lines.clear();
        if !on {
            tx.insert_end("\nYou're offline. Press Go online to join the Town square.\n", &["note"]);
        } else if msgs.is_empty() {
            tx.insert_end("\nIt's quiet here... say hello~\n", &["note"]);
        }
        let mut line = 1;
        let mut day: Option<i64> = None;
        let today = (d.now.floor() as i64 + d.offset).div_euclid(86400);
        let skip = msgs.len().saturating_sub(200);
        for m in &msgs[skip..] {
            let (md, t) = hm(m.ts, d.offset);
            if day != Some(md) {
                day = Some(md);
                tx.insert_end(&format!("{}\n", if md == today { "Today".to_string() } else { day_label(md) }), &["note"]);
                line += 1;
            }
            tx.insert_end(&m.name, &[if m.uid == d.uid { "me" } else { "name" }]);
            tx.insert_end(&format!(" #{}  {}\n", square_tag(&m.uid), t), &["tag"]);
            self.lines.push((line, m.clone()));
            tx.insert_end(&format!("{}\n", m.text), &[]);
            self.lines.push((line + 1, m.clone()));
            line += 2;
        }
        tx.relayout(fonts);
        // (Tk's see on a board not yet mapped, as hub_build_square's own paint is, leaves it at the top)
        if (at_end || self.shown <= 1) && mapped {
            let e = tx.end_index();
            tx.see(fonts, e);
        }
        self.sync_scroll(ui, fonts);
    }

    pub fn sync_scroll(&self, ui: &mut Ui, fonts: &dyn Fonts) {
        let (lo, hi) = match &ui.ws[self.txt].kind {
            Kind::Text { t, .. } => t.yview(fonts),
            _ => (0.0, 1.0),
        };
        scrollbar_draw(ui, self.sb, lo.max(0.0), hi.min(1.0), false, &self.th);
    }

    /// the message under a point of the board (menu_at)
    pub fn msg_at(&self, ui: &Ui, fonts: &dyn Fonts, mx: i32, my: i32) -> Option<Msg> {
        let (ax, ay) = ui.abs(self.txt);
        let Kind::Text { t, .. } = &ui.ws[self.txt].kind else { return None };
        let line = t.index_at(fonts, mx - ax, my - ay).0;
        self.lines.iter().find(|l| l.0 == line).map(|l| l.1.clone())
    }

    pub fn fit(&mut self, ui: &mut Ui, fonts: &dyn Fonts, th: &Theme, f: &Faces, zk_f: f64, pw: i32, ph: i32) {
        self.zk_f = zk_f;
        zk_fonts(ui, fonts, self.pg, zk_f);
        ui.layout_at(fonts, pw, ph);
        let w = ui.ws[self.title].w;
        if w >= 20 {
            let mut cv = std::mem::take(canvas_of(ui, self.title));
            let hh = hub_title_draw(&mut cv, w, TITLE, SUB, th, &f.ui, &f.title, zk_f, fonts, &mut self.seig);
            *canvas_of(ui, self.title) = cv;
            if ui.ws[self.title].opt_h != hh {
                ui.ws[self.title].opt_h = hh;
                ui.layout_at(fonts, pw, ph);
            }
        }
        zk_wrap(ui, fonts, self.pg, &mut self.wrap, pw, ph);
        if let Kind::Text { t, .. } = &mut ui.ws[self.txt].kind {
            t.update(fonts);
        }
        self.sync_scroll(ui, fonts);
        let d = self.d.clone();
        if let Some(s) = self.street.as_mut() {
            let (cw, ch) = (ui.ws[s.cv].w as i64, ui.ws[s.cv].h as i64);
            if (cw / Z + 1, ch / Z + 1) != (s.aw, s.ah) {
                s.build(ui, fonts, &d);
            }
        }
        let _ = Place::default();
    }
}
