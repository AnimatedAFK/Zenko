//! Playing with her (zenko.py ZenkoExtras: PLAY_DUR, play_pose, show_prop, PLAY_LINES, interact, give_gift,
//! start_sulk, forgive, fox_clicked, bond_add; ZenkoLife.interact -> feed(25); Zenko's SOUND_VARIETY wrappers).

use crate::{Companion, Ev, update_json};
use serde_json::{Value, json};
use zenko_art::palette::Palette;

pub struct Play {
    pub kind: String,
    pub t0: f64,
    pub dur: f64,
    pub arg: Option<String>,
    pub found: bool,
}

/// The pose for one frame (zenko.py's `st` dict in animate).
#[derive(Clone, Debug)]
pub struct Pose {
    pub state: String,
    pub hx: i32,
    pub ear: i32,
    pub look: (i32, i32, i32, i32),
    pub frame: i32,
    pub acc: Vec<String>,
    pub curled: bool,
    pub tframe: i32,
    pub n: i32,
    pub dxf: i32,
    pub dyf: i32,
    pub hide_fire: bool,
}

pub fn play_dur(kind: &str) -> f64 {
    match kind {
        "pet" | "belly" => 2.6,
        "boop" | "highfive" => 1.8,
        "tickle" | "praise" => 2.4,
        "treat" | "gift" => 3.4,
        "yarn" => 6.0,
        "hide" => 12.0,
        "found" | "tail" => 2.2,
        "dance" => 4.5,
        "brush" => 3.2,
        "sing" => 6.4,
        "yawn" => 1.4,
        "eat" => 1.6,
        "forgive" => 3.0,
        _ => 2.5,
    }
}

pub const TREATS: [&str; 5] = ["inari sushi", "aburaage", "dango", "onigiri", "mochi"];
pub const GIFTS: [&str; 12] = [
    "a shiny acorn", "a red maple leaf", "a tiny golden bell", "a ball of moonlight-blue yarn", "a lucky omamori charm",
    "a smooth river pebble", "a sakura-petal bookmark", "a little paper crane", "a pack of aburaage",
    "a star-shaped konpeito sweet", "a feather from a crane", "a mini shrine lantern",
];

pub fn play_lines(kind: &str) -> &'static [&'static str] {
    match kind {
        "pet" => &["Ehehe~ headpats! My ears go all floppy~", "Kyuu~ more, more! ...okay, back to work after this one.",
                   "Kon kon~ *happy tail wags*", "Waai~ you give the best headpats!",
                   "Mmh~ I could purr. Wait, foxes don't purr. Ehehe~"],
        "belly" => &["Kyaa~ belly rubs! Ehehe, that tickles!", "Uwaa~ only my favourite human gets belly rubs, ne~",
                     "*rolls over* Ehehe~ okay, okay, one more!"],
        "boop" => &["Boop! Kya~ my nose!", "Hyah! *sneezes* Ehehe~ boop back!", "Bweh~ you booped the snoot!"],
        "tickle" => &["Kyahaha~ stop, stop! Ehehehe~!", "Nyahaha~ not the paws! Kyaa~", "Ehehehe~ I give up, I give up!"],
        "tail" => &["Kya! My tails are sensitive~! ...but you can fluff them a little.",
                    "Mou~ every tail counts, be gentle! Ehehe~"],
        "treat" => &["Itadakimasu! *munch munch* Oishii~!", "Waai~ {item}! My favourite! *nom nom*",
                     "Hamu hamu... {item} is the best~kon!", "*happy munching* Kon kon~ arigatou!"],
        "yarn" => &["A ball of yarn! *pounce* *pounce* Ehehe~ I got it! ...it got me.",
                    "Kon! Don't let it roll away~ *bat bat*"],
        "hide" => &["Hide and seek? Okay! Close your eyes and count to ten~ ...Ready! Find me! (click where I was)"],
        "found" => &["Kya! You found me! Ehehe~ you're good at this!", "Uwaa~ caught! My ears gave me away, didn't they?"],
        "highfive" => &["High five! *pa-chin!* Yatta~!", "Paw up! *slap* Ehehe~ we're a great team!"],
        "dance" => &["Kon kon, step step~ *spins* Ehehe~", "Dance break! Tails left, tails right~ kon!"],
        "brush" => &["Ahh~ brushies... my fur is so fluffy now~", "Mmh~ every tail brushed. I'm sparkling, ne~?"],
        "sing" => &["\u{266a} Kon kon, little fox~ studying all day~ foxfire lights the way~ \u{266a}",
                    "\u{266a} La la~ Inari's fox is here~ one more page, my dear~ \u{266a}"],
        "yawn" => &["Fuwaa~ *big yawn* ...I wasn't sleeping. I was resting my eyes.", "Fuwaa~ sleepy... let's both stretch, ne?"],
        "forgive" => &["...Fine. I forgive you. Ehehe~ *tail wag*", "Hmph... okay, you're forgiven~ *nuzzle*"],
        "praise" => &["Ehehe~ you think I'm a good fox? *blush* Kon kon~", "Waai~ thank you! You're a good human too!"],
        _ => &["Ehehe~"],
    }
}

// ---- props: snacks, yarn, gifts (small pictures drawn in front of her) -----------------
pub fn prop_rows(name: &str) -> Option<&'static [&'static str]> {
    Some(match name {
        "inari sushi" => &["..KKKKKKK..", ".KwwwwwwwK.", "KOwwwwwwwOK", "KOOOOOOOOOK", "KOoOOoOOoOK", "KoOOOOOOOoK",
                           ".KooooooooK", "..KKKKKKK.."],
        "aburaage" => &["KKKKKKKKKK", "KlOOlOOOlK", "KOOoOOlOOK", "KOlOOOoOOK", "KOOOlOOOoK", "KKKKKKKKKK"],
        "dango" => &["....KKK..KKK..KKK...", "...KpppKKwwwKKgggK..", "bbbKpmpKKwwwKKgggKbb", "...KpppKKwwwKKgggK..",
                     "....KKK..KKK..KKK..."],
        "onigiri" => &["....KK....", "...KwwK...", "..KwwwwK..", ".KwwwwwwK.", ".KwwwwwwK.", "KwwwwwwwwK", "KwwnnnnwwK",
                       "KwwnnnnwwK", "KKKKKKKKKK"],
        "mochi" => &["..KKKKK..", ".KmmmmmK.", "KmwwmmmmK", "KmmmmmmmK", ".KKKKKKK."],
        "yarn" => &["..KKKKK..", ".KppRppK.", "KpRpppRpK", "KppRpRppK", "KpppRpppK", "KppRpRppK", "KpRpppRpK",
                    ".KppRppK.", "..KKKKK.."],
        "gift" => &["...Y...Y...", "....YyY....", "KKKKKyKKKKK", "KRRRRyRRRRK", "KyyyyyyyyyK", "KRRRRyRRRRK",
                    "KRRRRyRRRRK", "KRRRRyRRRRK", "KKKKKKKKKKK"],
        "gift_open" => &["Y....Y....Y", "...........", ".Y..sYs..Y.", "KKKKsYsKKKK", "KRRRRyRRRRK", "KyyyyyyyyyK",
                         "KRRRRyRRRRK", "KRRRRyRRRRK", "KKKKKKKKKKK"],
        _ => return None,
    })
}

pub fn prop_pal() -> Palette {
    Palette::from_pairs(&[
        (b'K', "#3a2414"), (b'o', "#c98a35"), (b'O', "#eab25a"), (b'l', "#ffd98a"), (b'w', "#fff6e0"),
        (b'p', "#ffb3c6"), (b'R', "#c2273a"), (b'r', "#ff7a8e"), (b'g', "#7cc77a"), (b'b', "#8a5a2e"),
        (b'y', "#f2c94c"), (b'Y', "#fff1a8"), (b'n', "#22252e"), (b'm', "#ffe1ec"), (b's', "#e8d8ff"),
    ])
}

/// A prop to draw this frame: (name, x, y in fox pixels, eaten 0..1).
pub type Prop = (String, i32, i32, f64);

fn alt(now: f64, per: f64, a: i32, b: i32) -> i32 {
    if (now / per) as i64 % 2 != 0 { a } else { b }
}

impl Companion {
    fn pick<'a>(&mut self, v: &[&'a str]) -> &'a str {
        let i = (self.rng.next() * v.len() as f64) as usize;
        v[i.min(v.len() - 1)]
    }

    pub fn say(&mut self, text: &str, secs: f64) {
        self.events.push(Ev::Say { text: text.to_string(), secs, choices: vec![] });
    }

    /// beep(kind) with SOUND_VARIETY on (play interactions pick from her voice pool).
    pub fn beep_play(&mut self, kind: &str) {
        self.events.push(Ev::Beep { kind: kind.to_string(), variety: true });
    }

    pub fn bond_add(&mut self, n: i64) {
        update_json("zenko_extras.json", |d| {
            let b = d.get("bond").and_then(|v| v.as_i64()).unwrap_or(10);
            d["bond"] = json!((b + n).clamp(0, 100));
        });
    }

    fn feed(&mut self, n: f64) {
        update_json("zenko_life.json", |l| {
            let now = crate::unix_now();
            let h0 = l.get("hunger").and_then(|v| v.as_f64()).unwrap_or(30.0);
            let t = l.get("hunger_t").and_then(|v| v.as_f64()).unwrap_or(now);
            let h = (h0 + (now - t) / 3600.0 * 3.0).clamp(0.0, 100.0);
            l["hunger"] = json!((h - n).max(0.0));
            l["hunger_t"] = json!(now);
        });
    }

    /// Play with her: pet, belly, boop, tickle, tail, treat, yarn, hide, highfive, gift, dance, brush, sing, yawn.
    pub fn interact(&mut self, now: f64, kind: &str, arg: Option<String>) -> String {
        self.sleeping = false;
        if self.sulk_until > 0.0 && now < self.sulk_until {
            if ["treat", "gift", "pet", "brush"].contains(&kind) {
                self.sulk_until = 0.0;
                if kind == "treat" || kind == "gift" {
                    self.bond_add(1);
                }
                return self.forgive(now, false, kind, arg);
            }
            self.beep_play("nag");
            let line = self.pick(&["Hmph! I'm still sulking. *turns away*", "Mou~! Not talking to you. ...a treat might help.",
                                   "Hmph. Say sorry first~"]);
            self.say(line, 5.0);
            return line.to_string();
        }
        if kind == "gift" {
            return self.give_gift(now);
        }
        let mut arg = arg;
        if kind == "treat" {
            if arg.is_none() {
                arg = Some(self.pick(&TREATS).to_string());
            }
            update_json("zenko_extras.json", |d| {
                let t = d.get("treats").and_then(|v| v.as_i64()).unwrap_or(0);
                d["treats"] = json!(t + 1);
            });
        }
        self.play = Some(Play { kind: kind.into(), t0: now, dur: play_dur(kind), arg: arg.clone(), found: false });
        self.mood_override = None;
        let snd = match kind {
            "pet" | "brush" | "praise" => "pet",
            "belly" | "tickle" | "tail" | "hide" => "giggle",
            "boop" => "boop",
            "treat" => "munch",
            "dance" | "sing" => "song",
            "yawn" => "yawn",
            _ => "happy",
        };
        self.beep_play(snd);
        let b = match kind {
            "pet" | "belly" | "highfive" | "dance" | "sing" => 1,
            "treat" | "yarn" | "hide" | "brush" => 2,
            _ => 0,
        };
        self.bond_add(b);
        let line = self.pick(play_lines(kind)).replace("{item}", arg.as_deref().unwrap_or(""));
        self.say(&line, 7.0);
        if kind == "treat" {
            self.feed(25.0); // ZenkoLife.interact: a treat fills her up
        }
        line
    }

    pub fn give_gift(&mut self, now: f64) -> String {
        let item = self.pick(&GIFTS).to_string();
        self.play = Some(Play { kind: "gift".into(), t0: now, dur: play_dur("gift"), arg: Some(item.clone()), found: false });
        self.beep_play("gift");
        let it = item.clone();
        update_json("zenko_extras.json", |d| {
            let mut tr: Vec<Value> = d.get("treasures").and_then(|v| v.as_array()).cloned().unwrap_or_default();
            tr.push(json!(it));
            let k = tr.len().saturating_sub(40);
            d["treasures"] = Value::Array(tr[k..].to_vec());
        });
        self.bond_add(3);
        let line = format!("A present? For me?! *unwraps* Uwaa~ {item}! I'll keep it in my treasure box forever~kon!");
        self.delayed.push((now + 1.0, Ev::Say { text: line.clone(), secs: 8.0, choices: vec![] }));
        line
    }

    pub fn start_sulk(&mut self, now: f64, why: &str) -> String {
        self.play = None;
        self.sulk_until = now + 75.0;
        self.bond_add(-2);
        self.beep_play("nag");
        let line = if why.is_empty() {
            self.pick(&["Hmph! *turns away and sulks*", "Mou~! I'm sulking now. Don't look at me!", "Hmph. Meanie. *pouts*"])
                .to_string()
        } else {
            why.to_string()
        };
        let full = format!("{line} (Say sorry, or give me a treat~)");
        self.say(&full, 8.0);
        full
    }

    pub fn forgive(&mut self, now: f64, auto: bool, via: &str, arg: Option<String>) -> String {
        self.sulk_until = 0.0;
        let k = if via == "treat" { "treat" } else { "forgive" };
        let arg = arg.or_else(|| if via == "treat" { Some(self.pick(&TREATS).to_string()) } else { None });
        self.play = Some(Play { kind: k.into(), t0: now, dur: play_dur(k), arg: arg.clone(), found: false });
        self.beep_play("pet");
        let line = if auto {
            "...Okay, I can't stay mad at you. Ehehe~ forgiven!".to_string()
        } else if via == "treat" {
            format!("...*sniff sniff* Is that {}? ...Fine. Forgiven! *munch*", arg.unwrap_or_default())
        } else if via == "gift" {
            "A present...? Hmph. ...Okay, okay, I forgive you~ *tail wag*".to_string()
        } else {
            self.pick(play_lines("forgive")).to_string()
        };
        self.say(&line, 7.0);
        line
    }

    /// A click on the fox: where you touch her matters (head, nose, belly, tails). x, y are window pixels.
    pub fn fox_clicked(&mut self, now: f64, x: i32, y: i32) {
        if let Some(p) = &mut self.play {
            if p.kind == "hide" && !p.found {
                p.found = true;
                self.play = Some(Play { kind: "found".into(), t0: now, dur: play_dur("found"), arg: None, found: false });
                self.beep_play("giggle");
                self.bond_add(1);
                let l = self.pick(play_lines("found"));
                self.say(l, 6.0);
                return;
            }
        }
        self.pokes.retain(|t| now - t < 4.0);
        self.pokes.push(now);
        if self.pokes.len() >= 8 && self.sulk_until == 0.0 {
            self.pokes.clear();
            self.start_sulk(now, "Mou~!! Too many pokes! Hmph!");
            return;
        }
        let fx = (x - self.colw) as f64 / self.z as f64;
        let fy = (y - self.fox_y) as f64 / self.z as f64;
        let kind = if (31.0..=41.0).contains(&fx) && (32.0..=38.0).contains(&fy) {
            "boop"
        } else if fx >= 58.0 || (fx >= 50.0 && fy >= 46.0 && fy < 64.0) {
            "tail"
        } else if fy < 44.0 {
            "pet"
        } else {
            "belly"
        };
        if self.sulk_until > 0.0 && now < self.sulk_until {
            self.beep_play("nag");
            let l = self.pick(&["Hmph!", "*turns away* Hmph.", "...", "Mou~ still sulking!"]);
            self.say(l, 3.0);
            return;
        }
        self.interact(now, kind, None);
    }

    /// How she moves while you play with her. Returns the prop to draw in front of her, if any.
    pub fn play_pose(&mut self, now: f64, st: &mut Pose) -> Option<Prop> {
        if let Some(p) = &self.play {
            if now > p.t0 + p.dur {
                let unfound_hide = p.kind == "hide" && !p.found;
                self.play = None;
                if unfound_hide {
                    let l = self.pick(&["Ehehe~ you didn't find me! I was right here~kon!",
                                        "Kon kon~ I win! Hide and seek champion!"]);
                    self.say(l, 6.0);
                    self.events.push(Ev::Beep { kind: "giggle".into(), variety: false });
                }
            }
        }
        let Some(p) = &self.play else {
            if self.sulk_until > 0.0 {
                if now < self.sulk_until {
                    st.state = "sulk".into();
                    st.hx = -1;
                    st.look = (-1, 0, -1, 0);
                } else {
                    self.sulk_until = 0.0;
                    self.forgive(now, true, "", None);
                }
            }
            return None;
        };
        let (k, e) = (p.kind.clone(), now - p.t0);
        let arg = p.arg.clone();
        let found = p.found;
        let f2 = (now / 0.18) as i64 % 2 != 0;
        let mut prop = None;
        match k.as_str() {
            "pet" | "forgive" | "praise" => {
                st.state = "hearts".into();
                st.hx = alt(now, 0.2, 1, -1);
                st.ear = 1;
            }
            "belly" => {
                st.state = "giggle".into();
                st.hx = alt(now, 0.12, 1, -1);
            }
            "boop" => st.state = if e < 0.35 { "boop" } else { "boop2" }.into(),
            "tickle" => {
                st.state = "giggle".into();
                st.hx = alt(now, 0.08, 1, -1);
                st.dxf = alt(now, 0.1, 1, -1);
            }
            "tail" => {
                st.state = if e < 0.4 { "boop" } else { "giggle" }.into();
                st.hx = alt(now, 0.15, 1, 0);
            }
            "treat" => {
                st.state = if e < 0.5 {
                    "boop2"
                } else if e < 2.4 {
                    if f2 { "munch" } else { "munch2" }
                } else {
                    "hearts"
                }
                .into();
                if e < 2.4 {
                    let eaten = ((e - 0.5) / 1.9).clamp(0.0, 1.0);
                    prop = Some((arg.unwrap_or_else(|| "inari sushi".into()), 41, 37, eaten));
                }
            }
            "yarn" => {
                let bx = 30.0 + 26.0 * (e * 2.1).sin();
                let by = 64.0 - (e * 4.2).sin().abs() * 9.0;
                let lx = if bx > 40.0 { 1 } else if bx < 28.0 { -1 } else { 0 };
                st.state = if (e * 2.0) as i64 % 3 != 0 { "boop2" } else { "giggle" }.into();
                st.look = (lx, 1, lx, 0);
                st.hx = alt(now, 0.3, lx, 0);
                prop = Some(("yarn".into(), bx as i32, by as i32, 0.0));
            }
            "hide" => {
                if found || e < 0.5 {
                    st.state = "giggle".into();
                } else {
                    let peek = e % 2.6 > 1.9;
                    st.state = "boop2".into();
                    st.dyf = if peek { 46 } else { 72 };
                    st.hide_fire = true;
                }
            }
            "found" => {
                st.state = "giggle".into();
                st.hx = alt(now, 0.12, 1, -1);
            }
            "highfive" => st.state = if e < 1.3 { "highfive" } else { "hearts" }.into(),
            "gift" => {
                st.state = if e < 1.0 { "boop2" } else { "hearts" }.into();
                if e < 2.2 {
                    prop = Some((if e < 1.0 { "gift" } else { "gift_open" }.into(), 40, 60, 0.0));
                }
            }
            "dance" => {
                st.state = if f2 { "sing" } else { "sing2" }.into();
                st.hx = alt(now, 0.25, 1, -1);
                st.dxf = alt(now, 0.5, 2, -2);
            }
            "brush" => {
                st.state = if f2 { "giggle" } else { "hearts" }.into();
                st.hx = alt(now, 0.5, 1, 0);
            }
            "sing" => {
                st.state = if (now / 0.4) as i64 % 2 != 0 { "sing" } else { "sing2" }.into();
                st.hx = alt(now, 0.48, 1, -1);
            }
            "yawn" => {
                st.state = "yawn".into();
                st.hx = 0;
            }
            "eat" => st.state = if f2 { "munch" } else { "munch2" }.into(),
            _ => {}
        }
        prop
    }
}
