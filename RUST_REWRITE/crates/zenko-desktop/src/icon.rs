//! zenko.py icon_pixels(64, theme): Zenko's face on a night-sky tile with a gold rim (the window icon).

use crate::theme::theme_named;
use zenko_art::palette::{Palette, hex_rgb, mixhex};
use zenko_art::{Look, core, sit};

pub fn icon_pixels(theme: &str) -> Vec<[u8; 4]> {
    let pal = Palette::fur(theme);
    let th = theme_named(theme);
    let top = hex_rgb(&th.night);
    let bot = hex_rgb(&mixhex(&th.night, &th.acc, 0.45));
    let gold = hex_rgb(&th.gold);
    let g = 64i32;
    let mut px = vec![[0u8; 4]; (g * g) as usize];
    let r = 13;
    for y in 0..g {
        for x in 0..g {
            let dx = (r - x).max(0).max(x - (g - 1 - r));
            let dy = (r - y).max(0).max(y - (g - 1 - r));
            let d2 = dx * dx + dy * dy;
            if d2 <= r * r {
                let t = y as f64 / (g - 1) as f64;
                let mut c = [0u8; 3];
                for i in 0..3 {
                    c[i] = (top[i] as f64 + (bot[i] as f64 - top[i] as f64) * t) as i64 as u8;
                }
                let edge = [0, 1, g - 1, g - 2];
                if (d2 > (r - 2) * (r - 2) && (dx != 0 || dy != 0)) || edge.contains(&x) || edge.contains(&y) {
                    c = gold;
                } else if ((x * 37 + y * 91) ^ (x * y)).rem_euclid(61) == 0 && y < 34 {
                    c = [255, 255, 230];
                }
                px[(y * g + x) as usize] = [c[0], c[1], c[2], 255];
            }
        }
    }
    let layers = [core::build_tails(3, 0, 8, false), sit::build_frame("happy", 1, None, &[], 0, 0, &Look::default())];
    let (ox, oy) = (-4, 8);
    for rows in &layers {
        for y in 0..rows.h {
            for x in 0..rows.w {
                let ch = rows.at(x, y);
                let (xx, yy) = (ox + x, oy + y);
                if let Some(col) = pal.get(ch) {
                    if !b"gYV".contains(&ch) && (2..g - 2).contains(&xx) && (2..g - 2).contains(&yy)
                        && px[(yy * g + xx) as usize][3] != 0
                    {
                        px[(yy * g + xx) as usize] = [col[0], col[1], col[2], 255];
                    }
                }
            }
        }
    }
    px
}
