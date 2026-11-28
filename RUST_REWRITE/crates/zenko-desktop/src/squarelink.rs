//! The Town square's link (zenko.py SquareLink + SquareLink23) without the network: what a streamed ntfy event adds
//! (take), what the board may show (visible), and what a post sends (post_req). The window's thread streams
//! `{server}/{topic},{topic}_reports/json?since=…` and feeds every line to `take`.

use crate::squareart::{SQUARE_HIDE_AT, hair_n, square_clean, square_name_ok, yukata_n};
use crate::squarepage::Msg;
use serde_json::{Value, json};
use std::collections::{BTreeSet, HashMap};

pub const SQUARE_SERVER: &str = "https://ntfy.sh";
pub const SQUARE_TOPIC: &str = "zenko_town_square_v1_7f3c1a";

#[derive(Clone, Debug, Default)]
pub struct SquareLink {
    pub server: String,
    pub topic: String,
    /// newest last
    pub msgs: Vec<Msg>,
    /// msg id -> reporter uids
    pub reports: HashMap<String, BTreeSet<String>>,
    /// off, connecting, live, error
    pub state: String,
    pub err: String,
    pub changed: i64,
    /// SquareLink23.look: your festival look sent with each message (yukata, mask, hair)
    pub look: Option<(i64, bool, i64)>,
    /// ntfy's `since`: 12h at first, then the last message id seen
    pub since: String,
}

/// Python's int() of a JSON value (number, numeric string or bool); None when it would raise
fn py_int(v: &Value) -> Option<i64> {
    match v {
        Value::Number(n) => n.as_i64().or_else(|| n.as_f64().filter(|f| f.is_finite()).map(|f| f.trunc() as i64)),
        Value::String(s) => s.trim().parse::<i64>().ok(),
        Value::Bool(b) => Some(*b as i64),
        _ => None,
    }
}

/// Python truthiness of a JSON value
fn truthy(v: &Value) -> bool {
    match v {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::Number(n) => n.as_f64() != Some(0.0),
        Value::String(s) => !s.is_empty(),
        Value::Array(a) => !a.is_empty(),
        Value::Object(o) => !o.is_empty(),
    }
}

/// str() of a JSON value as Python shows it (strings as they are)
fn py_str(v: &Value) -> String {
    match v {
        Value::Null => "None".into(),
        Value::String(s) => s.clone(),
        Value::Bool(b) => if *b { "True".into() } else { "False".into() },
        other => other.to_string(),
    }
}

fn cut(s: &str, n: usize) -> String {
    s.chars().take(n).collect()
}

impl SquareLink {
    pub fn new(server: &str, topic: &str) -> SquareLink {
        let server = if server.is_empty() { SQUARE_SERVER } else { server }.trim_end_matches('/').to_string();
        let topic = if topic.is_empty() { SQUARE_TOPIC } else { topic }.to_string();
        SquareLink { server, topic, state: "off".into(), since: "12h".into(), ..Default::default() }
    }

    /// the stream's URL now
    pub fn stream_url(&self) -> String {
        format!("{}/{},{}_reports/json?since={}", self.server, self.topic, self.topic, self.since)
    }

    /// one line of the stream (run's loop body): only "message" events count, and each moves `since` on
    pub fn line(&mut self, raw: &[u8], now: f64) {
        let Ok(ev) = serde_json::from_str::<Value>(&String::from_utf8_lossy(raw)) else { return };
        if ev.get("event").and_then(Value::as_str) != Some("message") {
            return;
        }
        if let Some(id) = ev.get("id").filter(|v| truthy(v)) {
            self.since = py_str(id);
        }
        self.take(&ev, now);
    }

    /// SquareLink23.take: SquareLink.take, then the sender's look on the message it added
    pub fn take(&mut self, ev: &Value, now: f64) {
        let before = self.msgs.len();
        let body = self.take0(ev, now);
        if self.msgs.len() > before
            && let Some(Value::Object(a)) = body.as_ref().and_then(|b| b.get("a"))
        {
            let y = a.get("y").map(py_int).unwrap_or(Some(-1));
            let h = a.get("h").map(py_int).unwrap_or(Some(-1));
            if let (Some(y), Some(h)) = (y, h) {
                let m = a.get("m").map(truthy).unwrap_or(false);
                self.msgs.last_mut().unwrap().look = Some((y.rem_euclid(yukata_n() as i64), m, h.rem_euclid(hair_n() as i64)));
            }
        }
    }

    /// SquareLink.take; gives back the parsed body
    fn take0(&mut self, ev: &Value, now: f64) -> Option<Value> {
        let msg = ev.get("message").filter(|v| truthy(v)).map(py_str).unwrap_or_else(|| "{}".into());
        let body: Value = serde_json::from_str(&msg).ok()?;
        if !body.is_object() || body.get("z") != Some(&json!(1)) {
            return None;
        }
        let g = |k: &str| body.get(k).cloned().unwrap_or(Value::Null);
        if truthy(&g("rep")) {
            let by = body.get("by").map(py_str).unwrap_or_default();
            self.reports.entry(cut(&py_str(&g("rep")), 40)).or_default().insert(cut(&by, 40));
        } else {
            let text = square_clean(&body.get("t").map(py_str).unwrap_or_default());
            let name = square_name_ok(&body.get("n").map(py_str).unwrap_or_default()).unwrap_or_else(|| "fox".into());
            if text.is_empty() {
                return Some(body);
            }
            let ts = ev.get("time").filter(|v| truthy(v)).and_then(|v| match v {
                Value::Number(n) => n.as_f64(),
                Value::String(s) => s.trim().parse().ok(),
                _ => None,
            }).unwrap_or(now);
            let m = Msg { id: ev.get("id").map(py_str).unwrap_or_default(), uid: cut(&body.get("u").map(py_str).unwrap_or_default(), 40),
                          name, text, ts, look: None };
            let from = self.msgs.len().saturating_sub(50);
            if self.msgs[from..].iter().any(|x| x.id == m.id) {
                return Some(body);
            }
            self.msgs.push(m);
            if self.msgs.len() > 300 {
                self.msgs.drain(..self.msgs.len() - 300);
            }
        }
        self.changed += 1;
        Some(body)
    }

    /// square_visible(): the messages minus blocked senders and ones hidden by enough reports
    pub fn visible(&self, block: &[String]) -> Vec<Msg> {
        let hidden: BTreeSet<&String> = self.reports.iter().filter(|(_, v)| v.len() >= SQUARE_HIDE_AT).map(|(k, _)| k).collect();
        self.msgs.iter().filter(|m| !block.contains(&m.uid) && !hidden.contains(&m.id)).cloned().collect()
    }

    /// post(): the URL and JSON body to POST (Content-Type text/plain); a report goes to the _reports topic
    pub fn post_req(&self, uid: &str, name: &str, text: &str, rep: Option<&str>) -> (String, String) {
        match rep {
            Some(r) if !r.is_empty() => (format!("{}/{}_reports", self.server, self.topic),
                                         py_dumps(&json!({"z": 1, "u": uid, "rep": r, "by": uid}))),
            _ => {
                let body = match self.look {
                    Some((y, m, h)) => json!({"z": 1, "u": uid, "n": name, "t": text, "a": {"y": y, "m": m, "h": h}}),
                    None => json!({"z": 1, "u": uid, "n": name, "t": text}),
                };
                (format!("{}/{}", self.server, self.topic), py_dumps(&body))
            }
        }
    }
}

/// json.dumps as Python writes it (", " and ": " separators, ASCII with \uXXXX escapes, keys in insertion order)
pub fn py_dumps(v: &Value) -> String {
    let mut out = String::new();
    dump(v, &mut out);
    out
}

fn dump(v: &Value, o: &mut String) {
    match v {
        Value::Object(m) => {
            o.push('{');
            for (i, (k, x)) in m.iter().enumerate() {
                if i > 0 {
                    o.push_str(", ");
                }
                dump(&Value::String(k.clone()), o);
                o.push_str(": ");
                dump(x, o);
            }
            o.push('}');
        }
        Value::Array(a) => {
            o.push('[');
            for (i, x) in a.iter().enumerate() {
                if i > 0 {
                    o.push_str(", ");
                }
                dump(x, o);
            }
            o.push(']');
        }
        Value::String(s) => {
            o.push('"');
            for c in s.chars() {
                match c {
                    '"' => o.push_str("\\\""),
                    '\\' => o.push_str("\\\\"),
                    '\n' => o.push_str("\\n"),
                    '\r' => o.push_str("\\r"),
                    '\t' => o.push_str("\\t"),
                    '\u{8}' => o.push_str("\\b"),
                    '\u{c}' => o.push_str("\\f"),
                    c if (c as u32) < 0x20 || (c as u32) > 0x7e => {
                        let mut b = [0u16; 2];
                        for u in c.encode_utf16(&mut b) {
                            o.push_str(&format!("\\u{:04x}", u));
                        }
                    }
                    c => o.push(c),
                }
            }
            o.push('"');
        }
        Value::Bool(b) => o.push_str(if *b { "true" } else { "false" }),
        Value::Null => o.push_str("null"),
        Value::Number(n) => o.push_str(&n.to_string()),
    }
}

// ---- the stalls on the festival street (Zenko23.z23_stall) -----------------------------------------------------------

/// what a stall click asks the page to do besides what Zenko says
#[derive(Clone, Debug, PartialEq)]
pub enum StallDo {
    Nothing,
    /// the mask stall: the look changed (z23_set_look(mask=…)): the crowd restyles you and the link sends it
    Look,
    /// Holi's colour stall: a burst of colour puffs on the street
    Puffs,
}

impl crate::Companion {
    fn sq_pick<'a>(&mut self, v: &[&'a str]) -> &'a str {
        let i = ((self.rand() * v.len() as f64) as usize).min(v.len() - 1);
        v[i]
    }

    /// z23_stall(key): eat (once every 10 minutes per stall), scoop a goldfish (+1 koi bait 38% of the time), draw
    /// the day's fortune, wear the fox mask, get a charm, throw colour, or play a game stall
    pub fn z23_stall(&mut self, key: &str, now: f64) -> StallDo {
        let st = crate::squareart::stall(key);
        if !st.is_array() {
            return StallDo::Nothing;
        }
        let s = |i: usize| st[i].as_str().unwrap_or("").to_string();
        let (sign, name, kind) = (s(0), s(1), s(2));
        let last = self.life_do(|l| {
            if !l["z23_stalls"].is_object() {
                l["z23_stalls"] = json!({});
            }
            l["z23_stalls"].clone()
        });
        match kind.as_str() {
            "food" => {
                let t = last[key.to_string().as_str()].as_f64().unwrap_or(0.0);
                if now - t < 600.0 {
                    self.say(&format!("Mmh, still full from the {}~ Maybe another stall?", name.to_lowercase()), 6.0);
                    return StallDo::Nothing;
                }
                let k = key.to_string();
                self.life_do(|l| l["z23_stalls"][k.as_str()] = json!(now));
                self.feed_tummy(12.0);
                let line = if self.rand() < 0.67 {
                    let f = self.sq_pick(&["Itadakimasu! %s, my favourite~", "Kon kon! The %s here is the best~",
                                        "*munch* ...%s at a festival just tastes better~"]);
                    f.replace("%s", &name.to_lowercase())
                } else {
                    format!("{name}! Thank you~ ({sign})")
                };
                self.say(&line, 7.0);
            }
            "kingyo" => {
                if self.rand() < 0.38 {
                    self.life_do(|l| {
                        let f = &mut l["fishing"];
                        if f.is_object() {
                            f["bait"] = json!((f["bait"].as_f64().unwrap_or(0.0) + 1.0).min(99.0));
                        }
                    });
                    self.say("You scooped a goldfish! It swims home to the pond... and leaves you +1 koi bait~", 8.0);
                } else {
                    self.say("Ah, the paper scoop tore~ One more try?", 5.0);
                }
            }
            "omikuji" => {
                let day = crate::today_iso();
                if last["omikuji"].as_str() == Some(day.as_str()) {
                    let f = self.life_do(|l| l["z23_fortune"].as_str().unwrap_or("大吉").to_string());
                    self.say(&format!("One fortune a day~ Today's was: {f}"), 7.0);
                    return StallDo::Nothing;
                }
                let all = crate::squareart::sdata()["Z23_FORTUNES"].as_array().cloned().unwrap_or_default();
                if all.is_empty() {
                    return StallDo::Nothing;
                }
                let f = all[((self.rand() * all.len() as f64) as usize).min(all.len() - 1)].clone();
                let (a, b) = (f[0].as_str().unwrap_or("").to_string(), f[1].as_str().unwrap_or("").to_string());
                let fortune = format!("{a} {b}");
                self.life_do(|l| {
                    l["z23_stalls"]["omikuji"] = json!(day);
                    l["z23_fortune"] = json!(fortune);
                });
                self.say(&format!("おみくじ… {a}! {b}"), 12.0);
            }
            "mask" => {
                let on = !self.cfg.b("z23_mask", false);
                crate::update_config(&mut self.cfg, |v| v["z23_mask"] = json!(on));
                self.say(if on { "A fox mask, just like me~ Kon!" } else { "Mask off~ I can see your face again." }, 6.0);
                return StallDo::Look;
            }
            "charm" => {
                let t = self.sq_pick(&["You got a 学業成就 charm: success in study~", "A 合格 charm: for passing exams! I'll keep it on my ema~",
                                    "A 健康 charm: for good health and good sleep~"]);
                self.say(t, 8.0);
            }
            "colour" => {
                self.say("Splash! Happy Holi~ Now we're both pink and yellow.", 7.0);
                return StallDo::Puffs;
            }
            _ => {
                let f = self.sq_pick(&["Hit! You won a little %s plush~", "So close! The cork bounced off~", "You won a pixel %s keychain~"]);
                let w = self.sq_pick(&["fox", "tanuki", "koi", "daruma"]);
                self.say(&f.replace("%s", w), 6.0);
            }
        }
        StallDo::Nothing
    }
}
