//! The battery meter on her tail (zenko.py battery_tick every 30 s, draw_battery): a little cell at the base of her
//! tails, green / amber / red, with a lightning bolt while charging, and a warning at 20% and 10%.

use crate::Companion;
use crate::shapes::tk_coord;
use crate::theme::theme;
use zenko_art::palette::mixhex;

/// What draw_battery puts on the canvas, in window pixels (Tk rounds the coordinates itself).
#[derive(Clone, Debug, PartialEq)]
pub enum Shape {
    /// create_rectangle(x1, y1, x2, y2, fill, outline, width)
    Rect { x1: i32, y1: i32, x2: i32, y2: i32, fill: String, outline: Option<(String, i32)> },
    /// create_polygon(points, fill, outline, width=1)
    Poly { pts: Vec<(i32, i32)>, fill: String, outline: String },
}

impl Companion {
    /// battery_tick(): `st` = battery_status()
    pub fn battery_tick(&mut self, now: f64, st: Option<(i64, bool)>) {
        self.battery = st;
        let Some((pct, charging)) = st else { return };
        if !self.cfg.b("battery_on_tail", true) {
            return;
        }
        let warn = if pct <= 10 { Some(10) } else if pct <= 20 { Some(20) } else { None };
        if charging || pct > 25 {
            self.batt_warned.clear();
        } else if let Some(w) = warn {
            if !self.batt_warned.contains(&w) {
                self.batt_warned.push(w);
                self.set_mood(now, "sad", 4.0);
                self.events.push(crate::Ev::Beep { kind: "sad".into(), variety: false });
                self.say(&format!("My tail says {pct}% battery! Plug in the charger{}.", if w == 10 { " NOW" } else { "" }), 12.0);
            }
        }
    }

    /// draw_battery(dx, dy, curled)
    pub fn battery_shapes(&self, dx: i32, dy: i32, curled: bool) -> Vec<Shape> {
        let Some((pct, charging)) = self.battery.filter(|_| self.cfg.b("battery_on_tail", true)) else { return vec![] };
        let t = theme(&self.cfg);
        let z = self.z * 2;
        let (bx, by) = if curled { (58, 50) } else { (60, 66) };
        let (ox, oy) = (self.colw + dx + bx * self.z, self.fox_y + dy + by * self.z);
        let (w, h) = (8 * z, 5 * z);
        let dark = self.pal.hex(b'K').unwrap_or_else(|| "#222222".into());
        let zf = z as f64;
        let (oxf, oyf) = (ox as f64, oy as f64);
        let mut v = vec![
            Shape::Rect { x1: ox, y1: oy, x2: ox + w, y2: oy + h, fill: mixhex(&t.night, "#000000", 0.2),
                          outline: Some((t.gold.clone(), 1.max(self.z))) },
            Shape::Rect { x1: ox + w, y1: tk_coord(oyf + zf * 1.5), x2: ox + w + z, y2: tk_coord(oyf + h as f64 - zf * 1.5),
                          fill: t.gold.clone(), outline: None },
        ];
        let col = if pct > 50 { "#5fd39a" } else if pct > 20 { "#f2c14e" } else { "#ff5468" };
        let fillw = z.max(((w - 2 * z) as f64 * pct as f64 / 100.0) as i32);
        v.push(Shape::Rect { x1: ox + z, y1: oy + z, x2: ox + z + fillw, y2: oy + h - z, fill: col.into(), outline: None });
        v.push(Shape::Rect { x1: ox + z, y1: oy + z, x2: ox + z + fillw, y2: oy + z + 1.max(self.z),
                             fill: mixhex(col, "#ffffff", 0.5), outline: None });
        if charging {
            let p = [(4.5, 0.6), (2.6, 2.7), (4.0, 2.7), (3.3, 4.4), (5.4, 2.2), (4.0, 2.2)];
            let pts = p.iter().map(|&(a, b)| (tk_coord(oxf + a * zf), tk_coord(oyf + b * zf))).collect();
            v.push(Shape::Poly { pts, fill: "#ffe066".into(), outline: dark });
        }
        v
    }
}
