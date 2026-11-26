//! Renders the desktop fox for a few seconds into a GIF (no window needed):
//!   cargo run --release -p zenko-desktop --example preview -- out.gif [seconds] [mood]
use zenko_desktop::{Cfg, Companion, Session};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let out = args.get(1).cloned().unwrap_or("zenko_preview.gif".into());
    let secs: f64 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(6.0);
    let mood = args.get(3).cloned().unwrap_or_default();
    let cfg = Cfg::from(
        serde_json::json!({"theme": std::env::var("ZK_THEME").unwrap_or("snow".into()),
                                          "outfit": [], "robe": std::env::var("ZK_ROBE").unwrap_or("none".into())}),
    );
    let mut fox = Companion::new(cfg, 1.0, 1000.0);
    if mood == "focus" {
        fox.session = Some(Session {
            planned: 1500.0,
            started: 1000.0,
        });
    } else if mood == "sleep" {
        fox.sleeping = true;
    } else if !mood.is_empty() {
        fox.mood_override = Some((mood.clone(), 1e9));
    }
    // crop to the fox (the window's left column is empty space for her speech bubble)
    let (x0, y0) = (fox.colw as usize - 10, fox.fox_y as usize - 10);
    let (w, h) = (fox.w as usize - x0, fox.h as usize - y0);
    let f = std::fs::File::create(&out).unwrap();
    let mut enc = gif::Encoder::new(f, w as u16, h as u16, &[]).unwrap();
    enc.set_repeat(gif::Repeat::Infinite).unwrap();
    let bg = [0x11u8, 0x16, 0x2f];
    let mut t = 1000.0;
    while t < 1000.0 + secs {
        let fr = fox.frame(t, (-5000, -5000), (0, 0));
        let mut rgba = Vec::with_capacity(w * h * 4);
        for y in 0..h {
            for x in 0..w {
                let p = fr.px[(y0 + y) * fr.w + x0 + x];
                if p[3] == 0 {
                    rgba.extend_from_slice(&[bg[0], bg[1], bg[2], 255])
                } else {
                    rgba.extend_from_slice(&p)
                }
            }
        }
        let mut gf = gif::Frame::from_rgba_speed(w as u16, h as u16, &mut rgba, 10);
        gf.delay = 6;
        enc.write_frame(&gf).unwrap();
        t += 0.06;
    }
    println!("{out}: {} x {}, state now {}", w, h, fox.last_state);
}
