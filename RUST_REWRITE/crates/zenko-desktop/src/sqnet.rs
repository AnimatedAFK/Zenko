//! The Town square's network side (zenko.py SquareLink.run / post, square_connect / square_disconnect): one
//! background thread holds the ntfy stream open and feeds every line to the shared link; posts go on their own
//! short-lived threads. The page reads the link (state, messages, reports) every paint.

use crate::net::{Req, request};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use zenko_desktop::squarelink::{SQUARE_SERVER, SQUARE_TOPIC, SquareLink};
use zenko_desktop::setpage::APP_VERSION;

struct Conn {
    link: Arc<Mutex<SquareLink>>,
    stop: Arc<AtomicBool>,
    alive: Arc<AtomicBool>,
}

static SQ: Mutex<Option<Conn>> = Mutex::new(None);
static BOOTED: AtomicBool = AtomicBool::new(false);

fn ua() -> String {
    format!("Zenko/{APP_VERSION}")
}

fn unix_now() -> f64 {
    zenko_desktop::unix_now()
}

/// the link, if there is one (self._sq)
pub fn link() -> Option<Arc<Mutex<SquareLink>>> {
    SQ.lock().unwrap().as_ref().map(|c| c.link.clone())
}

/// SquareLink.start: the stream thread, unless it already runs
fn start(c: &Conn) {
    if c.alive.load(Ordering::SeqCst) {
        return;
    }
    c.stop.store(false, Ordering::SeqCst);
    c.alive.store(true, Ordering::SeqCst);
    let (link, stop, alive) = (c.link.clone(), c.stop.clone(), c.alive.clone());
    std::thread::spawn(move || {
        run(&link, &stop);
        alive.store(false, Ordering::SeqCst);
    });
}

/// SquareLink.run
fn run(link: &Arc<Mutex<SquareLink>>, stop: &Arc<AtomicBool>) {
    let mut backoff = 3.0f64;
    while !stop.load(Ordering::SeqCst) {
        let url = {
            let mut l = link.lock().unwrap();
            l.state = "connecting".into();
            l.stream_url()
        };
        let mut pending: Vec<u8> = vec![];
        let mut live = false;
        let ua = ua();
        let r = request(&Req { url: &url, ua: &ua, timeout_ms: 75000, ..Default::default() }, &mut |b| {
            if stop.load(Ordering::SeqCst) {
                return false;
            }
            if !live {
                live = true;
                link.lock().unwrap().state = "live".into();
            }
            pending.extend_from_slice(b);
            while let Some(i) = pending.iter().position(|&c| c == b'\n') {
                let raw: Vec<u8> = pending.drain(..=i).collect();
                link.lock().unwrap().line(&raw, unix_now());
            }
            true
        });
        if stop.load(Ordering::SeqCst) {
            return;
        }
        match r {
            // (the stream ended cleanly: open it again at once, as `for raw in r` ending does)
            Ok(_) if live => {
                backoff = 3.0;
                continue;
            }
            Ok(_) => {}
            Err(e) => {
                let mut l = link.lock().unwrap();
                l.state = "error".into();
                l.err = e.chars().take(120).collect();
            }
        }
        if live {
            backoff = 3.0;
        }
        let until = std::time::Instant::now() + std::time::Duration::from_secs_f64(backoff);
        while std::time::Instant::now() < until && !stop.load(Ordering::SeqCst) {
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
        backoff = (backoff * 2.0).min(60.0);
    }
}

/// SquareLink.stop
fn stop(c: &Conn) {
    c.stop.store(true, Ordering::SeqCst);
    c.link.lock().unwrap().state = "off".into();
}

/// Zenko23.square_connect (cfg square_on, square_server; the look from z23_my_look_wire)
pub fn connect(cfg: &zenko_desktop::Cfg) {
    if !cfg.b("square_on", false) {
        return;
    }
    let srv = cfg.raw.get("square_server").and_then(|v| v.as_str()).filter(|s| !s.is_empty()).unwrap_or(SQUARE_SERVER).trim()
        .to_string();
    let look = look_wire(cfg);
    let mut g = SQ.lock().unwrap();
    if let Some(c) = g.as_ref()
        && c.link.lock().unwrap().server == srv.trim_end_matches('/')
    {
        c.link.lock().unwrap().look = Some(look);
        start(c);
        return;
    }
    if let Some(c) = g.as_ref() {
        stop(c);
    }
    let mut l = SquareLink::new(&srv, SQUARE_TOPIC);
    l.look = Some(look);
    let c = Conn { link: Arc::new(Mutex::new(l)), stop: Arc::new(AtomicBool::new(false)), alive: Arc::new(AtomicBool::new(false)) };
    start(&c);
    *g = Some(c);
}

/// square_disconnect
pub fn disconnect() {
    if let Some(c) = SQ.lock().unwrap().as_ref() {
        stop(c);
    }
}

/// z23_my_look_wire
pub fn look_wire(cfg: &zenko_desktop::Cfg) -> (i64, bool, i64) {
    let i = |k: &str, d: i64| cfg.raw.get(k).and_then(|v| v.as_i64().or_else(|| v.as_f64().map(|f| f as i64))).unwrap_or(d);
    (i("z23_yukata", 3), cfg.b("z23_mask", false), i("z23_hair", 0))
}

/// the look the link sends changed (z23_set_look)
pub fn set_look(cfg: &zenko_desktop::Cfg) {
    if let Some(l) = link() {
        l.lock().unwrap().look = Some(look_wire(cfg));
    }
}

/// SquareLink.post on a background thread (self.bg); `done` gets None when it went, or the error
pub fn post(uid: String, name: String, text: String, rep: Option<String>, done: impl FnOnce(Option<String>) + Send + 'static) {
    let Some(l) = link() else {
        done(Some("not connected".into()));
        return;
    };
    let (url, body) = l.lock().unwrap().post_req(&uid, &name, &text, rep.as_deref());
    std::thread::spawn(move || {
        let ua = ua();
        let r = request(&Req { url: &url, method: "POST", ua: &ua, headers: "Content-Type: text/plain", body: body.as_bytes(),
                               timeout_ms: 15000, len: None }, &mut |_| true);
        done(r.err());
    });
}

/// the app's start: square_connect 15 s in when the square is on
pub fn boot(t: f64, cfg: &zenko_desktop::Cfg) {
    if t >= 15.0 && !BOOTED.swap(true, Ordering::SeqCst) {
        connect(cfg);
    }
}
