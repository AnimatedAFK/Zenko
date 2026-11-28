//! The lock-in pounce (zenko21 z21_eat_overlay): a see-through window over the distraction. She crouches in the
//! corner, leaps in an arc, lands, gobbles it to darkness bite by bite from her mouth outwards, shakes happily with
//! sparkles and hops off while the dark melts away.

use crate::gdi::{self, KEY, Op, rgb};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::time::Instant;
use windows_sys::Win32::Foundation::*;
use windows_sys::Win32::Graphics::Gdi::*;
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::UI::WindowsAndMessaging::*;
use zenko_art::palette::mixhex;
use zenko_desktop::theme::Theme;

type Img = (i32, i32, Rc<Vec<[u8; 4]>>);

struct Spark {
    x: f64,
    y: f64,
    vx: f64,
    vy: f64,
    s: f64,
    g: f64,
    end: f64,
    col: u32,
}

pub struct Overlay {
    hwnd: HWND,
    w: i32,
    h: i32,
    z: i32,
    fw: f64,
    fh: f64,
    sx: f64,
    sy: f64,
    tx: f64,
    ty: f64,
    imgs: HashMap<(String, String, i32), Img>,
    bites: Vec<Op>,
    rims: Vec<Op>,
    spots: Vec<(f64, f64, f64)>,
    nb: usize,
    sparks: Vec<Spark>,
    dust: bool,
    done: bool,
    t0: Instant,
    last: Option<f64>,
    fox: (String, String, i32, f64, f64),
    text: Option<Op>,
    font: HFONT,
    night: u32,
    rim: u32,
    th: Theme,
    site: String,
    rng: u64,
    pub finished: bool,
    pub closed: bool,
}

thread_local! {
    pub static OV: RefCell<Option<Overlay>> = const { RefCell::new(None) };
}

impl Overlay {
    fn rand(&mut self) -> f64 {
        // xorshift: the pounce's own scatter of bites and crumbs (zenko.py uses an unseeded random.Random())
        self.rng ^= self.rng << 13;
        self.rng ^= self.rng >> 7;
        self.rng ^= self.rng << 17;
        (self.rng >> 11) as f64 / (1u64 << 53) as f64
    }
    fn uni(&mut self, a: f64, b: f64) -> f64 {
        a + (b - a) * self.rand()
    }
    fn choice<T: Copy>(&mut self, v: &[T]) -> T {
        let i = ((self.rand() * v.len() as f64) as usize).min(v.len() - 1);
        v[i]
    }
    fn spark(&mut self, x: f64, y: f64, cols: &[u32], v: f64, g: f64, life: f64, size: Option<f64>, now: f64) {
        let z = self.z as f64;
        let s = size.unwrap_or_else(|| self.choice(&[z, z, 2.0 * z]));
        let col = self.choice(cols);
        let vx = self.uni(-v, v);
        let vy = self.uni(-v * 1.4, -v * 0.3);
        self.sparks.push(Spark { x, y, vx, vy, s, g, end: now + life, col });
    }
    fn bite(&mut self, bx: f64, by: f64, r: f64) {
        let tc = zenko_desktop::shapes::tk_coord;
        let oval = |x1: f64, y1: f64, x2: f64, y2: f64, c: u32| Op::Oval { x1: tc(x1), y1: tc(y1), x2: tc(x2), y2: tc(y2), fill: c };
        self.bites.push(oval(bx - r, by - r, bx + r, by + r, self.night));
        for j in 0..7 {
            // a scalloped, toothy edge
            let a = j as f64 / 7.0 * 2.0 * std::f64::consts::PI + self.uni(-0.2, 0.2);
            let rr = r * 0.32;
            let (cx, cy) = (bx + a.cos() * r * 0.92, by + a.sin() * r * 0.92);
            self.bites.push(oval(cx - rr, cy - rr, cx + rr, cy + rr, self.night));
        }
        let start = self.uni(0.0, 360.0);
        let rr = r * 1.05;
        let pts = arc_pts(bx, by, rr, start, 70.0);
        self.rims.push(Op::Line { pts, color: self.rim, width: 1.max(self.z / 3) });
    }
}

/// create_arc(style="arc", start, extent) as a polyline (Tk draws the arc outline with the same pen)
fn arc_pts(cx: f64, cy: f64, r: f64, start: f64, extent: f64) -> Vec<(i32, i32)> {
    let n = 24;
    (0..=n)
        .map(|i| {
            let a = (start + extent * i as f64 / n as f64).to_radians();
            (zenko_desktop::shapes::tk_coord(cx + r * a.cos()), zenko_desktop::shapes::tk_coord(cy - r * a.sin()))
        })
        .collect()
}

pub fn is_running() -> bool {
    OV.with(|o| o.try_borrow().map(|g| g.is_some()).unwrap_or(true))
}

/// Start the pounce over `rect`. `img(side, pose, frame, Z)` draws her frames.
pub fn start(rect: (i32, i32, i32, i32), site: &str, th: &Theme, font_family: &str, dpi: f64,
             mut img: impl FnMut(&str, &str, i32, usize) -> zenko_art::raster::Image) {
    unsafe {
        let (mut x1, mut y1, mut x2, mut y2) = rect;
        let (sw, sh) = (GetSystemMetrics(SM_CXSCREEN), GetSystemMetrics(SM_CYSCREEN));
        x1 = x1.max(-8);
        y1 = y1.max(-8);
        x2 = x2.min(sw + 8);
        y2 = y2.min(sh + 8);
        let (w, h) = (320.max(x2 - x1), 240.max(y2 - y1));
        let (gw, gh) = (110.0, 78.0);
        let z0 = ((w as f64 * 0.42 / gw).min(h as f64 * 0.5 / gh)).round_ties_even() as i32;
        let mut z = 2.max(12.min(z0));
        let hd = if z >= 4 { 4 } else { z };
        let k = 1.max(z / hd);
        z = hd * k;
        let (fw, fh) = (gw * z as f64, gh * z as f64);
        let mut imgs = HashMap::new();
        let states: Vec<(&str, &str, i32)> = [("rigL", "crouch", [0, 2, 4, 6].as_slice()), ("rigL", "leap", &[0, 2, 4, 6]),
                                              ("rigL", "dive", &[0, 4]), ("rigL", "land", &[0]), ("rigL", "playbow", &[0, 4]),
                                              ("rigL", "shake", &[0, 2, 4, 6]), ("rigR", "leap", &[0, 2, 4, 6])]
            .iter()
            .flat_map(|(s, p, fs)| fs.iter().map(move |f| (*s, *p, *f)))
            .collect();
        for (s, p, f) in states {
            let im = img(s, p, f, z as usize);
            imgs.insert((s.to_string(), p.to_string(), f), (im.w as i32, im.h as i32, Rc::new(im.px)));
        }
        let night_hex = mixhex(&th.night, "#000000", 0.25);
        let rim_hex = mixhex(&night_hex, &th.gold, 0.18);
        let (sx, sy) = (w as f64 - fw * 0.92, h as f64 - fh * 1.02);
        let (tx, ty) = ((w as f64 - fw) / 2.0 + fw * 0.1, (h as f64 - fh) / 2.0);
        let fs = 13.max(28.min(w / 38));
        let family = if gdi::font_exists("Zen Old Mincho") { "Zen Old Mincho".to_string() } else { font_family.to_string() };
        let font = gdi::tk_font(&family, fs, true, dpi);
        let seed = (Instant::now().elapsed().as_nanos() as u64) ^ (std::process::id() as u64) << 32 ^ 0x9E37_79B9_7F4A_7C15;
        let mut ov = Overlay {
            hwnd: std::ptr::null_mut(),
            w,
            h,
            z,
            fw,
            fh,
            sx,
            sy,
            tx,
            ty,
            imgs,
            bites: vec![],
            rims: vec![],
            spots: vec![],
            nb: 22,
            sparks: vec![],
            dust: false,
            done: false,
            t0: Instant::now(),
            last: None,
            fox: ("rigL".into(), "crouch".into(), 0, sx, sy),
            text: None,
            font,
            night: rgb(&night_hex),
            rim: rgb(&rim_hex),
            th: th.clone(),
            site: site.to_string(),
            rng: seed | 1,
            finished: false,
            closed: false,
        };
        // bite centres: a spiral from her mouth outwards, so the window disappears where she's eating
        let (mx0, my0) = (tx + fw * 0.2, ty + fh * 0.66);
        let far = (mx0.max(w as f64 - mx0)).hypot(my0.max(h as f64 - my0));
        for i in 0..ov.nb {
            let q = (i + 1) as f64 / ov.nb as f64;
            let a = i as f64 * 2.39996 + ov.uni(-0.3, 0.3);
            let d = far * q.powf(0.8) * 0.9;
            ov.spots.push((mx0 + a.cos() * d, my0 + a.sin() * d * 0.8, w.max(h) as f64 * (0.07 + 0.2 * q)));
        }
        let hinst = GetModuleHandleW(std::ptr::null());
        let cls = gdi::wide("ZenkoRustEat");
        let wc = WNDCLASSEXW {
            cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
            style: 0,
            lpfnWndProc: Some(wndproc),
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: hinst,
            hIcon: std::ptr::null_mut(),
            hCursor: LoadCursorW(std::ptr::null_mut(), IDC_ARROW),
            hbrBackground: std::ptr::null_mut(),
            lpszMenuName: std::ptr::null(),
            lpszClassName: cls.as_ptr(),
            hIconSm: std::ptr::null_mut(),
        };
        RegisterClassExW(&wc);
        let hwnd = CreateWindowExW(WS_EX_LAYERED | WS_EX_TOPMOST | WS_EX_TOOLWINDOW, cls.as_ptr(), std::ptr::null(),
                                   WS_POPUP, x1, y1, w, h, std::ptr::null_mut(), std::ptr::null_mut(), hinst,
                                   std::ptr::null());
        SetLayeredWindowAttributes(hwnd, KEY, 255, LWA_COLORKEY | LWA_ALPHA);
        ov.hwnd = hwnd;
        OV.with(|o| *o.borrow_mut() = Some(ov));
        ShowWindow(hwnd, SW_SHOWNOACTIVATE);
        SetTimer(hwnd, 1, 16, None);
        SetTimer(hwnd, 2, 5000, None); // the safety net: gone after 5 seconds whatever happens
        step(hwnd);
    }
}

fn step(hwnd: HWND) {
    let mut close = false;
    OV.with(|o| {
        let Ok(mut g) = o.try_borrow_mut() else { return };
        let Some(ov) = g.as_mut() else { return };
        let now = ov.t0.elapsed().as_secs_f64();
        let e = now;
        let dtf = (now - ov.last.unwrap_or(now)).min(0.05);
        ov.last = Some(now);
        let (z, fw, fh, w, h) = (ov.z as f64, ov.fw, ov.fh, ov.w as f64, ov.h as f64);
        let (sx, sy, tx, ty) = (ov.sx, ov.sy, ov.tx, ov.ty);
        let th = ov.th.clone();
        let glow = rgb(&th.glow);
        let gold = rgb(&th.gold);
        let white = 0xffffff;
        let mut alpha = 255u8;
        if e < 0.34 {
            // crouch: eyes on the prey, a little butt wiggle
            let f = [0, 2, 4, 6][(e / 0.085) as usize % 4];
            ov.fox = ("rigL".into(), "crouch".into(), f, sx + if (e / 0.06) as i64 % 2 != 0 { z } else { -z }, sy);
        } else if e < 0.80 {
            // the leap: a real arc, stretching out, diving down
            let t = (e - 0.34) / 0.46;
            let ez = t * t * (3.0 - 2.0 * t);
            let x = sx + (tx - sx) * ez;
            let y = sy + (ty - sy) * ez - (std::f64::consts::PI * t).sin() * (h * 0.22).min((ty - 4.0).max(0.0));
            ov.fox = if t < 0.55 {
                ("rigL".into(), "leap".into(), [0, 2, 4, 6][3.min((t / 0.55 * 4.0) as usize)], x, y)
            } else {
                ("rigL".into(), "dive".into(), if t < 0.8 { 0 } else { 4 }, x, y)
            };
            if ov.rand() < 0.8 {
                let c2 = rgb(&mixhex(&th.glow, "#ffffff", 0.5));
                ov.spark(x + fw * 0.75, y + fh * 0.35, &[glow, c2], 1.5, 0.0, 0.35, None, now);
            }
        } else if e < 0.92 {
            // landing: front paws down, a puff of dust
            ov.fox = ("rigL".into(), "land".into(), 0, tx, ty + z);
            if !ov.dust {
                let cols = [rgb(&mixhex(&th.ink, &th.night, 0.4)), rgb(&th.grey)];
                for side in [-1.0, 1.0] {
                    for _ in 0..6 {
                        ov.spark(tx + fw * (0.5 + 0.45 * side), ty + fh * 0.95, &cols, 4.0, 0.15, 0.4, None, now);
                    }
                }
                ov.dust = true;
            }
        } else if e < 1.75 {
            // gobble gobble: head down, bites spread from her mouth
            let chew = ((e - 0.92) / 0.085) as i64 % 2;
            ov.fox = ("rigL".into(), "playbow".into(), if chew != 0 { 0 } else { 4 }, tx, ty + if chew != 0 { z } else { 0.0 });
            let want = ov.nb.min(((e - 0.92) / 0.78 * ov.nb as f64) as usize + 1);
            while ov.bites.len() / 8 < want {
                let (bx, by, r) = ov.spots[ov.bites.len() / 8];
                ov.bite(bx, by, r);
                let (mx, my) = (tx + fw * 0.2, ty + fh * 0.66);
                for _ in 0..4 {
                    ov.spark(mx, my, &[gold, rgb(&th.acc), rgb(&th.ink), white], 7.0, 0.9, 0.7, None, now);
                }
            }
        } else if !ov.done {
            // gone. Gulp!
            ov.done = true;
            ov.bites.push(Op::Rect { x: 0, y: 0, w: ov.w, h: ov.h, color: ov.night });
            ov.rims.clear();
            ov.finished = true;
            let ty2 = if ty + fh + 44.0 < h { ty + fh + 26.0 } else { (ty - 24.0).max(30.0) };
            let msg = format!("Gulp! {} eaten. Back to work~ kon!", ov.site);
            let lay = gdi::layout(ov.font, &msg, 0);
            // anchor center: x - width/2, y - height/2
            let (cx, cy) = (zenko_desktop::shapes::tk_coord(w / 2.0), zenko_desktop::shapes::tk_coord(ty2));
            let (x, y) = (cx - lay.width / 2, cy - lay.height / 2);
            ov.text = Some(Op::Text { x, y, lay, font: ov.font, color: gold, right: false });
        } else if e < 2.35 {
            // a happy full-tummy shake with sparkles
            let f = [0, 2, 4, 6][((e - 1.75) / 0.07) as usize % 4];
            ov.fox = ("rigL".into(), "shake".into(), f, tx, ty);
            if ov.rand() < 0.35 {
                let (a, b) = (ov.uni(0.1, 0.9), ov.uni(0.0, 0.6));
                ov.spark(tx + a * fw, ty + b * fh, &[gold, glow, white], 2.0, -0.05, 0.5, None, now);
            }
        } else if e < 2.85 {
            // and off she hops, while the dark melts away
            let t = (e - 2.35) / 0.5;
            let f = [0, 2, 4, 6][3.min((t * 4.0) as usize)];
            let x = tx + (w - tx + fw * 0.2) * t * t;
            let y = ty - (std::f64::consts::PI * (t * 1.2).min(1.0)).sin() * h * 0.12;
            ov.fox = ("rigR".into(), "leap".into(), f, x, y);
            alpha = ((1.0 - t).max(0.0) * 255.0) as u8;
        } else {
            close = true;
        }
        // crumbs, dust and foxfire sparkles
        let k = dtf * 60.0;
        ov.sparks.retain_mut(|c| {
            if now >= c.end {
                return false;
            }
            c.x += c.vx * k;
            c.y += c.vy * k;
            c.vy += c.g * k;
            true
        });
        if !close {
            unsafe {
                SetLayeredWindowAttributes(hwnd, KEY, alpha, LWA_COLORKEY | LWA_ALPHA);
                InvalidateRect(hwnd, std::ptr::null(), 0);
            }
        }
    });
    if close {
        unsafe { DestroyWindow(hwnd) };
    }
}

fn paint(ov: &Overlay, dc: HDC) {
    let tc = zenko_desktop::shapes::tk_coord;
    let mut ops = vec![Op::Rect { x: 0, y: 0, w: ov.w, h: ov.h, color: KEY }];
    ops.extend(ov.bites.iter().map(clone_op));
    ops.extend(ov.rims.iter().map(clone_op));
    if let Some(t) = &ov.text {
        ops.push(clone_op(t));
    }
    let (s, p, f, x, y) = &ov.fox;
    if let Some((iw, ih, px)) = ov.imgs.get(&(s.clone(), p.clone(), *f)) {
        ops.push(Op::Img { x: tc(*x), y: tc(*y), w: *iw, h: *ih, px: px.clone() });
    }
    for c in &ov.sparks {
        let (x1, y1, x2, y2) = (tc(c.x), tc(c.y), tc(c.x + c.s), tc(c.y + c.s));
        ops.push(Op::Rect { x: x1, y: y1, w: x2 - x1, h: y2 - y1, color: c.col });
    }
    unsafe {
        let mdc = CreateCompatibleDC(dc);
        let bmp = CreateCompatibleBitmap(dc, ov.w, ov.h);
        let ob = SelectObject(mdc, bmp as _);
        gdi::draw(mdc, &ops);
        BitBlt(dc, 0, 0, ov.w, ov.h, mdc, 0, 0, SRCCOPY);
        SelectObject(mdc, ob);
        DeleteObject(bmp as _);
        DeleteDC(mdc);
    }
}

fn clone_op(o: &Op) -> Op {
    match o {
        Op::Oval { x1, y1, x2, y2, fill } => Op::Oval { x1: *x1, y1: *y1, x2: *x2, y2: *y2, fill: *fill },
        Op::Line { pts, color, width } => Op::Line { pts: pts.clone(), color: *color, width: *width },
        Op::Rect { x, y, w, h, color } => Op::Rect { x: *x, y: *y, w: *w, h: *h, color: *color },
        Op::Text { x, y, lay, font, color, right } => Op::Text {
            x: *x,
            y: *y,
            lay: gdi::TextLayout { lines: lay.lines.clone(), width: lay.width, height: lay.height, line_h: lay.line_h },
            font: *font,
            color: *color,
            right: *right,
        },
        _ => Op::Clip(None),
    }
}

/// Did the pounce reach the "gone" moment since the last call (time to close or minimise the window)?
pub fn take_finished() -> bool {
    OV.with(|o| {
        o.try_borrow_mut().ok().and_then(|mut g| g.as_mut().map(|ov| std::mem::take(&mut ov.finished))).unwrap_or(false)
    })
}

/// Is it over (the window gone)? Clears it.
pub fn take_closed() -> bool {
    OV.with(|o| {
        let Ok(mut g) = o.try_borrow_mut() else { return false };
        if g.as_ref().is_some_and(|ov| ov.closed) {
            if let Some(ov) = g.take() {
                unsafe { DeleteObject(ov.font as _) };
            }
            return true;
        }
        false
    })
}

unsafe extern "system" fn wndproc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    unsafe {
        match msg {
            WM_TIMER => {
                if wp == 2 {
                    DestroyWindow(hwnd);
                } else {
                    step(hwnd);
                }
                0
            }
            WM_PAINT => {
                let mut ps: PAINTSTRUCT = std::mem::zeroed();
                let dc = BeginPaint(hwnd, &mut ps);
                OV.with(|o| {
                    if let Ok(g) = o.try_borrow() {
                        if let Some(ov) = g.as_ref() {
                            paint(ov, dc);
                        }
                    }
                });
                EndPaint(hwnd, &ps);
                0
            }
            WM_ERASEBKGND => 1,
            WM_DESTROY => {
                OV.with(|o| {
                    if let Ok(mut g) = o.try_borrow_mut() {
                        if let Some(ov) = g.as_mut() {
                            ov.closed = true;
                        }
                    }
                });
                0
            }
            WM_MOUSEACTIVATE => MA_NOACTIVATE as isize,
            _ => DefWindowProcW(hwnd, msg, wp, lp),
        }
    }
}
