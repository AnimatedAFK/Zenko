//! The hub's sidebar, drawn on one canvas the way zenko.py's final class draws it: FoxBuddy.hub_side_draw (night
//! gradient, stars, moon, hills, torii, lanterns, title) + Zenko (status line placement) + Zenko19.z19_side_sky
//! (time-of-day sky, sun, snow caps / sakura or momiji branch) + Zenko22.z22_side_extras (scene, pinned badges,
//! browser light); the menu is FoxBuddy.hub_side_nav + Zenko (text size) + Zenko19 (pixel icons); the layout is
//! Zenko.hub_side_layout + Zenko20 (compact rows).

use crate::aa::{Shape, aa_pixels};
use crate::theme::Theme;
use crate::tkcanvas::{Canvas, Photo};
use crate::ui::Font;
use std::collections::HashMap;
use std::rc::Rc;
use zenko_art::core::{Grid, H as H_, W as W_};
use zenko_art::palette::{Palette, hex_rgb, mixhex};
use zenko_sound::mt::PyRandom;

/// Z19_ICONS (and zenko22's square): 11 x 11 menu icons. # gold, + gold shade, * gold light, r vermilion, w cream,
/// g moss, b indigo
pub fn z19_icon(key: &str) -> Option<[&'static str; 11]> {
    Some(match key {
        "home" => ["rrrrrrrrrrr", ".*rrrrrrr*.", "..r.....r..", ".rrrrrrrrr.", "..r..#..r..", "..r.....r..", "..r.....r..",
                   "..r.....r..", "..r.....r..", ".+++...+++.", "..........."],
        "timetable" => ["..+++++++..", ".+*#####*+.", "..wwwwwww..", "..w+w+w+w..", "..wwwwwww..", "..w+w+w+w..",
                        "..wwwwwww..", "..w+w+r+w..", "..wwwwwww..", ".+*#####*+.", "..+++++++.."],
        "deadlines" => ["###########", ".+*******+.", "..+wwwww+..", "...+www+...", "....+w+....", "....+r+....",
                        "...+.r.+...", "..+..r..+..", ".+.rrrrr.+.", "###########", "..........."],
        "reminders" => [".....r.....", "....r.r....", "...+###+...", "..+#***#+..", ".+#wwwww#+.", ".#wwwwwww#.",
                        ".#ww+++ww#.", ".#wwwwwww#.", ".#ww++www#.", ".#########.", "....r.r...."],
        "notebook" => ["...........", ".###...###.", "#*ww#.#ww*#", "#ww+w#w+ww#", "#wwww#wwww#", "#ww+w#w+ww#",
                       "#wwww#wwww#", "#ww+w#w+ww#", "#####+#####", ".....r.....", ".....r....."],
        "daily" => [".....#.....", ".#...#...#.", "..#.***.#..", "...*###*...", "##*##r##*##", "...*###*...", "..#.***.#..",
                    ".#...#...#.", ".....#.....", "...........", "..........."],
        "exercise" => ["...........", "...........", ".++.....++.", "+##+...+##+", "+#*+###+*#+", "+##+###+##+",
                       "+##+...+##+", ".++.....++.", "...........", "...........", "..........."],
        "garden" => ["....ggg....", "..ggggggg..", ".ggg*ggggg.", "..ggg.ggg..", ".....#.....", "....##.....", ".....##....",
                     "..rrrrrrr..", "..+rrrrr+..", "...+++++...", "..........."],
        "map" => ["....###....", "..#+...+#..", ".#...r...#.", ".+...r...+.", "#...r*r...#", "#....*....#", "#...w*w...#",
                  ".+...w...+.", ".#...w...#.", "..#+...+#..", "....###...."],
        "screen" => ["###########", "#bbbbbbbbb#", "#b*bbbbbbb#", "#bb*bbbbbb#", "#bbbbbbbbb#", "#bbbbbbbbb#", "###########",
                     "....+#+....", "...+###+...", "..+++++++..", "..........."],
        "chat" => ["..#######..", ".#wwwwwww#.", "#wwwwwwwww#", "#ww+w+w+ww#", "#wwwwwwwww#", ".#wwwwwww#.", "..##w####..",
                   "...#w#.....", "...##......", "...........", "..........."],
        "notes" => [".........##", "........#*#", ".......#*#.", "......#*#..", ".....#*#...", "....+#+....", "...+++.....",
                    "..rrr......", ".rrr.......", ".rr........", "r.........."],
        "you" => ["#.........#", "##.......##", "#r#.....#r#", "#ww#####ww#", "#wwwwwwwww#", "#wrrwwwrrw#", "#wwwwwwwww#",
                  ".#www+www#.", "..#wwrww#..", "...#####...", "..........."],
        "customize" => ["...........", "..+#####+..", ".+#*#*#*#+.", "+#r#r#r#r#+", "+#w#w#w#w#+", ".+#w#w#w#+.",
                        "..+#w#w#+..", "...+#w#+...", "....+#+....", ".....#.....", "..........."],
        "settings" => ["....#.#....", "..#.###.#..", "...#####...", ".####+####.", "..##+++##..", "###++*++###",
                       "..##+++##..", ".####+####.", "...#####...", "..#.###.#..", "....#.#...."],
        "square" => ["...........", ".#####.....", "#wwwww#....", "#w+w+w#####", "#wwwww#www#", ".##w##ww+w#", "..#w#wwwww#",
                     "..##.##w##.", "......#w#..", "......##...", "..........."],
        _ => return None,
    })
}

/// z19_icon_pal(state, base)
pub fn z19_icon_pal(state: &str, base: &str, gold: &str) -> Palette {
    let fade = match state {
        "on" => 0.0,
        "hover" => 0.22,
        _ => 0.48,
    };
    let mut p = Palette { map: [None; 128] };
    for (k, v) in [(b'#', gold.to_string()), (b'+', mixhex(gold, "#000000", 0.42)), (b'*', mixhex(gold, "#ffffff", 0.5)),
                   (b'r', "#d9483c".into()), (b'w', "#f3e6cc".into()), (b'g', "#5fae7a".into()), (b'b', "#3b5a8a".into())] {
        p.map[k as usize] = Some(hex_rgb(&mixhex(&v, base, fade)));
    }
    p
}

/// sprite_photo([rows], pal, zoom): zenko21's HD smoothing from zoom 2 up, plain pixels below
pub fn sprite_photo(rows: &[&str], pal: &Palette, zoom: usize, hd: bool) -> Photo {
    let g = Grid::from_rows(rows);
    let img = if hd && zoom >= 2 {
        zenko_art::raster::sprite_hd(&[&g], pal, zoom, None)
    } else {
        let one = zenko_art::raster::sprite_rows(&[&g], pal, None);
        zenko_art::raster::zk_upscale(&one, 1.max(zoom)) // zoom 1 only here (PhotoImage.zoom for more)
    };
    Photo { w: img.w, h: img.h, px: img.px }
}

/// z19_daypart(): (part, how far into it)
pub fn daypart(hour: u32, minute: u32) -> (&'static str, f64) {
    let h = hour as f64 + minute as f64 / 60.0;
    if (5.0..7.5).contains(&h) {
        ("dawn", (h - 5.0) / 2.5)
    } else if (7.5..17.0).contains(&h) {
        ("day", (h - 7.5) / 9.5)
    } else if (17.0..19.5).contains(&h) {
        ("dusk", (h - 17.0) / 2.5)
    } else {
        ("night", 0.0)
    }
}

pub fn season_of(month: u32) -> &'static str {
    match month {
        12 | 1 | 2 => "winter",
        3..=5 => "spring",
        6..=8 => "summer",
        _ => "autumn",
    }
}

/// z19_sky(top, bot, part)
pub fn z19_sky(top: &str, bot: &str, part: &str) -> (String, String) {
    let (t, b, k) = match part {
        "dawn" => ("#3a3466", "#a8707a", 0.30),
        "day" => ("#2c4a74", "#5f8a8c", 0.30),
        "dusk" => ("#3d2e5c", "#b8704a", 0.32),
        _ => return (top.into(), bot.into()),
    };
    (mixhex(top, t, k), mixhex(bot, b, k * 1.1))
}

/// what the sidebar needs to know about the app this frame
#[derive(Clone, Debug)]
pub struct SideCtx {
    pub t: Theme,
    pub font_ui: String,
    /// _jp_font(): a Japanese-capable font, or ""
    pub jp: String,
    pub fox_name: String,
    pub catchphrase: String,
    pub part: String,
    pub season: String,
    /// Z19_LOOK: sky_daynight, sky_seasons, weather
    pub sky: bool,
    pub seasons: bool,
    pub weather: Option<String>,
    pub nav_compact: bool,
    pub zk_f: f64,
    pub hd: bool,
    /// pinned badges (id, kanji, name, tier), already filtered to the ones you have, at most 3
    pub pins: Vec<(String, String, String, i64)>,
    pub linked: bool,
    /// cfg side_scene if unlocked, else ""
    pub scene: String,
}

pub const BADGE_TIER_COL: [&str; 4] = ["#5a5048", "#c27a3a", "#c9d1dc", "#f2c94c"];

#[derive(Default)]
pub struct Side {
    pub cv: Canvas,
    pub w: i32,
    pub h: i32,
    pub logo_zoom: i32,
    pub navkeys: Vec<(String, String)>,
    pub page: Option<String>,
    pub hover: Option<String>,
    pub pill_y: Option<f64>,
    pub pill_to: Option<f64>,
    pub stars: Vec<(u32, f64, f64)>,
    moon: Option<(Rc<Photo>, Rc<Photo>)>,
    sun: Option<((String, String, String), Rc<Photo>)>,
    icons: HashMap<(String, String, usize, String, String), Rc<Photo>>,
    pub logo_items: Option<(u32, u32)>,
    pub status_item: Option<u32>,
    pub status_text: String,
}

fn s(v: &str) -> String {
    v.to_string()
}

impl Side {
    pub fn new(navkeys: Vec<(String, String)>, logo_zoom: i32) -> Side {
        Side { navkeys, logo_zoom, ..Default::default() }
    }

    /// hub_side_layout(): (zoom, top, title_y, nav_y, rowh)
    pub fn layout(&mut self, compact: bool) -> (i32, i32, i32, i32, i32) {
        let hh = self.h;
        if hh < 700 && self.logo_zoom > 1 {
            self.logo_zoom = 1;
        }
        let z = self.logo_zoom;
        let top = 14;
        let title_y = top + H_ * z + 8;
        let nav_y = title_y + 66;
        let n = if self.navkeys.is_empty() { 9 } else { self.navkeys.len() as i32 };
        let mut rowh = 23;
        for r in [42, 38, 34, 31, 28, 26, 24, 23] {
            rowh = r;
            if nav_y + n * r + 150 <= hh {
                break;
            }
        }
        if compact {
            rowh = rowh.min(32);
        }
        (z, top, title_y, nav_y, rowh)
    }

    /// hub_side_key(y): the menu item under a canvas y
    pub fn key_at(&mut self, y: i32, compact: bool) -> Option<String> {
        let (_, _, _, nav_y, rowh) = self.layout(compact);
        let i = (y - nav_y).div_euclid(rowh);
        if y >= nav_y && 0 <= i && (i as usize) < self.navkeys.len() { Some(self.navkeys[i as usize].0.clone()) } else { None }
    }

    /// hub_side_draw() of the final class, on a canvas of size (w, h)
    pub fn draw(&mut self, c: &SideCtx) {
        let (wd, hh) = (self.w, self.h);
        if wd < 50 || hh < 50 {
            return;
        }
        let t = &c.t;
        let (wf, hf) = (wd as f64, hh as f64);
        let cv = &mut self.cv;
        cv.delete("bg");
        cv.delete("nav");
        cv.delete("deco");
        let (top, bot) = (t.night.clone(), t.night2.clone());
        let mut y = 0;
        while y < hh {
            cv.rect(0.0, y as f64, wf, (y + 2) as f64).fill(&mixhex(&top, &bot, y as f64 / hf)).outline("").tags(&["bg"]);
            y += 2;
        }
        let mut rnd = PyRandom::new(7);
        self.stars.clear();
        for _ in 0..46 {
            let x = rnd.uniform(4.0, wf - 4.0);
            let y = rnd.uniform(4.0, hf * 0.8);
            let sz = *rnd.choice(&[1.0, 1.0, 1.0, 2.0, 2.0, 3.0]);
            let col = mixhex(&top, &t.ink, rnd.uniform(0.25, 0.8));
            cv.rect(x, y, x + sz, y + sz).fill(&col).outline("").tags(&["bg"]);
            let it = cv.last_id();
            let (ph, sp) = (rnd.uniform(0.0, 6.28), rnd.uniform(0.6, 2.2));
            self.stars.push((it, ph, sp));
        }
        // crescent moon with a soft halo
        let (mx, my) = (wf - 46.0, 46.0);
        if self.moon.is_none() {
            let gold = &t.gold;
            let moon = mixhex(gold, "#ffffff", 0.55);
            let soft = mixhex(&moon, gold, 0.5);
            let a = |c: &str, k: f64| Some((c.to_string(), k));
            let halo = aa_pixels(72, 72, &[(Shape::Circle(36.0, 36.0, 33.0), a(gold, 0.05)),
                                          (Shape::Circle(36.0, 36.0, 26.0), a(gold, 0.07)),
                                          (Shape::Circle(36.0, 36.0, 20.0), a(gold, 0.08))], None);
            let body = aa_pixels(72, 72, &[(Shape::Circle(36.0, 36.0, 15.0), a(&moon, 1.0)),
                                          (Shape::Circle(30.0, 36.0, 4.0), a(&soft, 0.5)),
                                          (Shape::Circle(39.0, 44.0, 2.5), a(&soft, 0.45)),
                                          (Shape::Circle(45.0, 30.0, 14.0), None)], None);
            self.moon = Some((Rc::new(halo), Rc::new(body)));
        }
        let (m0, m1) = self.moon.clone().unwrap();
        cv.image(mx - 36.0, my - 36.0, m0).anchor("nw").tags(&["bg"]);
        cv.image(mx - 36.0, my - 36.0, m1).anchor("nw").tags(&["bg"]);
        // hills and a torii gate at the bottom
        let far = mixhex(&bot, &t.acc, 0.10);
        let near = mixhex(&bot, "#000000", 0.45);
        let mut pts = vec![0.0, hf];
        let mut x = 0;
        while x < wd + 12 {
            let xf = x as f64;
            pts.extend([xf, hf - 64.0 - 18.0 * (xf / 38.0).sin() - 8.0 * (xf / 13.0).sin()]);
            x += 12;
        }
        pts.extend([wf, hf]);
        cv.poly(pts).fill(&far).outline("").tags(&["bg"]);
        let mut pts = vec![0.0, hf];
        let mut x = 0;
        while x < wd + 12 {
            let xf = x as f64;
            pts.extend([xf, hf - 36.0 - 10.0 * (xf / 27.0 + 1.3).sin()]);
            x += 12;
        }
        pts.extend([wf, hf]);
        cv.poly(pts).fill(&near).outline("").tags(&["bg"]);
        let red = mixhex("#e8463c", &top, 0.18);
        let dark = mixhex(&red, "#000000", 0.35);
        let (tx, ty, k) = (40.0, hf - 40.0, 3.0);
        for (x, y, w_, h_, col) in [(-1.0, -21.0, 26.0, 2.0, &red), (0.0, -19.0, 24.0, 1.0, &dark), (-2.0, -22.0, 2.0, 1.0, &red),
                                    (25.0, -22.0, 2.0, 1.0, &red), (2.0, -16.0, 20.0, 2.0, &red), (4.0, -19.0, 2.0, 19.0, &red),
                                    (18.0, -19.0, 2.0, 19.0, &red), (11.0, -19.0, 2.0, 3.0, &red), (4.0, -14.0, 2.0, 14.0, &dark),
                                    (18.0, -14.0, 2.0, 14.0, &dark), (3.0, -1.0, 4.0, 1.0, &near), (17.0, -1.0, 4.0, 1.0, &near)] {
            cv.rect(tx + x * k / 2.0, ty + y * k / 2.0, tx + (x + w_) * k / 2.0, ty + (y + h_) * k / 2.0).fill(col).outline("")
                .tags(&["bg"]);
        }
        for lx in [wf - 60.0, wf - 30.0] {
            let ly = hf - 30.0;
            cv.rect(lx - 1.0, ly - 14.0, lx + 7.0, ly - 12.0).fill(&near).outline("").tags(&["bg"]);
            cv.rect(lx, ly - 12.0, lx + 6.0, ly - 6.0).fill(&mixhex(&t.gold, "#ff9a3a", 0.3)).outline("").tags(&["bg"]);
            cv.rect(lx + 1.0, ly - 6.0, lx + 5.0, ly).fill(&near).outline("").tags(&["bg"]);
        }
        // title
        let (z, top_, title_y, _nav_y, _rowh) = self.layout(c.nav_compact);
        let cv = &mut self.cv;
        let name = &c.fox_name;
        let spaced: Vec<String> = name.to_uppercase().chars().map(|ch| ch.to_string()).collect();
        let n_chars = name.chars().count();
        cv.text((wd / 2) as f64, (title_y + 16) as f64, &spaced.join("  ")).fill(&t.gold)
            .font(Font::new(&c.font_ui, if n_chars <= 7 { 20 } else { 14 }, true)).tags(&["deco"]);
        let sub = c.catchphrase.to_lowercase();
        if !c.jp.is_empty() {
            cv.text((wd / 2) as f64, (title_y + 44) as f64, &format!("\u{5584}\u{72d0}  \u{00b7}  {sub}"))
                .fill(&mixhex(&top, &t.ink, 0.6)).font(Font::new(&c.jp, 9, false)).tags(&["deco"]);
        } else {
            cv.text((wd / 2) as f64, (title_y + 44) as f64, &sub).fill(&mixhex(&top, &t.ink, 0.6))
                .font(Font::new(&c.font_ui, 9, false)).tags(&["deco"]);
        }
        let (cx, ty0) = ((wd / 2) as f64, title_y as f64);
        cv.line(vec![cx - 70.0, ty0 + 58.0, cx + 70.0, ty0 + 58.0]).fill(&mixhex(&top, &t.gold, 0.35)).tags(&["deco"]);
        cv.poly(vec![cx, ty0 + 55.0, cx + 3.0, ty0 + 58.0, cx, ty0 + 61.0, cx - 3.0, ty0 + 58.0]).fill(&t.gold).outline("")
            .tags(&["deco"]);
        if self.logo_items.is_none() || cv.find_withtag("logo").is_empty() {
            let blank = Rc::new(Photo { w: 0, h: 0, px: vec![] });
            cv.image(0.0, 0.0, blank.clone()).anchor("nw").tags(&["logo"]);
            let a = cv.last_id();
            cv.image(0.0, 0.0, blank).anchor("nw").tags(&["logo"]);
            self.logo_items = Some((a, cv.last_id()));
        }
        let lw = W_ * z;
        let (a, b) = self.logo_items.unwrap();
        for it in [a, b] {
            if let Some(i) = cv.get(it) {
                i.coords = vec![((wd - lw).div_euclid(2) + 6) as f64, top_ as f64];
            }
        }
        cv.tag_raise("logo");
        self.nav(c);
        self.status_place(c, 24.0, hf - 150.0, wf - 40.0);
        self.cv.tag_raise("flake");
        self.cv.tag_raise("status");
        // Zenko: the status line sits under the menu, or hides when there's no room
        let (_, _, _, nav_y, rowh) = self.layout(c.nav_compact);
        let end = nav_y + self.navkeys.len() as i32 * rowh + 6;
        let mut y = end.max(hh - 150);
        if y + 30 > hh - 92 {
            y = -400;
        }
        self.status_place(c, 24.0, y as f64, wf - 40.0);
        self.z19_side_sky(c);
        self.z22_side_extras(c);
    }

    /// _CanvasText.place(x, y, width)
    fn status_place(&mut self, c: &SideCtx, x: f64, y: f64, width: f64) {
        let t = &c.t;
        let have = self.status_item.filter(|&id| !self.cv.find_withtag(&id.to_string()).is_empty());
        match have {
            None => {
                let text = self.status_text.clone();
                self.cv.text(x, y, &text).anchor("nw").wrap(width).fill(&mixhex(&t.night, &t.ink, 0.7))
                    .font(Font::new(&c.font_ui, 10, false)).tags(&["status"]);
                self.status_item = Some(self.cv.last_id());
            }
            Some(id) => {
                let it = self.cv.get(id).unwrap();
                it.coords = vec![x, y];
                it.wrap = width;
            }
        }
    }

    /// hub_side_nav() of the final class
    pub fn nav(&mut self, c: &SideCtx) {
        let t = &c.t;
        let wf = self.w as f64;
        self.cv.delete("nav");
        let (_z, _top, _title_y, nav_y, rowh) = self.layout(c.nav_compact);
        let base = t.night.clone();
        let keys: Vec<String> = self.navkeys.iter().map(|k| k.0.clone()).collect();
        let page = self.page.clone();
        if let Some(i) = page.as_ref().and_then(|p| keys.iter().position(|k| k == p)) {
            let ty = (nav_y + i as i32 * rowh) as f64;
            let y = self.pill_y.unwrap_or(ty);
            self.pill_to = Some(ty);
            let (fillc, edge) = (mixhex(&base, &t.acc, 0.30), mixhex(&base, &t.gold, 0.55));
            let pts = round_pts_flat(14.0, y + 3.0, wf - 14.0, y + rowh as f64 - 3.0, 99, 2, "pill");
            self.cv.poly(pts).fill(&fillc).outline(&edge).join("miter").tags(&["nav", "pill"]);
            let cy = y + (rowh / 2) as f64;
            self.cv.poly(vec![wf - 30.0, cy - 4.0, wf - 26.0, cy, wf - 30.0, cy + 4.0, wf - 34.0, cy]).fill(&t.gold).outline("")
                .tags(&["nav", "pill"]);
            self.pill_y = Some(y);
        }
        let hv = self.hover.clone();
        if let Some(i) = hv.as_ref().filter(|h| Some(*h) != page.as_ref()).and_then(|h| keys.iter().position(|k| k == h)) {
            let y = (nav_y + i as i32 * rowh) as f64;
            let pts = round_pts_flat(14.0, y + 3.0, wf - 14.0, y + rowh as f64 - 3.0, 99, 2, "pill");
            self.cv.poly(pts).fill(&mixhex(&base, &t.ink, 0.08)).outline(&mixhex(&base, &t.ink, 0.16)).join("miter")
                .tags(&["nav"]);
        }
        // Zenko: the menu text follows the window and never overlaps
        let zk = zk_size(c.zk_f, 12);
        let size = 10.max(zk.min((rowh as f64 * 0.36) as i32));
        for (i, (key, label)) in self.navkeys.iter().enumerate() {
            let y = nav_y + i as i32 * rowh;
            let on = Some(key) == page.as_ref();
            let col = if on {
                t.ink.clone()
            } else if Some(key) == hv.as_ref() {
                mixhex(&base, &t.ink, 0.85)
            } else {
                mixhex(&base, &t.ink, 0.6)
            };
            self.cv.text(62.0, (y + rowh / 2) as f64, label).anchor("w").fill(&col)
                .font(Font::new(&c.font_ui, size, on)).tags(&["nav"]);
        }
        // Zenko19: the pixel-art icons
        let zoom = if rowh >= 24 { 2 } else { 1 };
        for (i, (key, _)) in self.navkeys.clone().iter().enumerate() {
            let Some(rows) = z19_icon(key) else { continue };
            let stt = if Some(key) == page.as_ref() { "on" } else if Some(key) == hv.as_ref() { "hover" } else { "off" };
            let ck = (key.clone(), s(stt), zoom, t.gold.clone(), base.clone());
            let img = self.icons.entry(ck).or_insert_with(|| {
                Rc::new(sprite_photo(&rows, &z19_icon_pal(stt, &base, &t.gold), zoom, c.hd))
            }).clone();
            let y = nav_y + i as i32 * rowh + rowh / 2 - (11 * zoom as i32) / 2;
            self.cv.image(if zoom == 2 { 28.0 } else { 36.0 }, y as f64, img).anchor("nw").tags(&["nav"]);
        }
    }

    fn z19_moon(&mut self, show: bool) {
        let Some((a, b)) = self.moon.clone() else { return };
        for it in self.cv.items.iter_mut() {
            if it.has("bg") && it.image.as_ref().is_some_and(|im| Rc::ptr_eq(im, &a) || Rc::ptr_eq(im, &b)) {
                it.hidden = !show;
            }
        }
    }

    fn z19_side_sky(&mut self, c: &SideCtx) {
        let (wd, hh) = (self.w, self.h);
        if wd < 50 || hh < 50 {
            return;
        }
        let t = &c.t;
        let (wf, hf) = (wd as f64, hh as f64);
        let part = if c.sky { c.part.as_str() } else { "night" };
        let weather = c.weather.as_deref();
        let (mut top, mut bot) = z19_sky(&t.night, &t.night2, part);
        if weather == Some("rain") {
            (top, bot) = (mixhex(&top, "#2a3040", 0.25), mixhex(&bot, "#3a4256", 0.25));
        }
        if (top.as_str(), bot.as_str()) != (t.night.as_str(), t.night2.as_str()) {
            for it in self.cv.items.iter_mut() {
                if it.has("bg") && it.kind == crate::tkcanvas::CKind::Rect {
                    let (x0, y0, x1, y1) = (it.coords[0], it.coords[1], it.coords[2], it.coords[3]);
                    if x0 == 0.0 && y1 - y0 == 2.0 && x1 >= wf - 1.0 {
                        it.fill = mixhex(&top, &bot, y0 / hf);
                    }
                }
            }
        }
        self.cv.delete("z19sky");
        if matches!(part, "day" | "dawn" | "dusk") {
            for &(it, _, _) in &self.stars {
                if let Some(i) = self.cv.get(it) {
                    i.hidden = part == "day";
                }
            }
            let key = (s("sun"), s(part), t.gold.clone());
            if self.sun.as_ref().map(|s| &s.0) != Some(&key) {
                let sun = match part {
                    "day" => mixhex(&t.gold, "#fff6dc", 0.35),
                    "dawn" => mixhex(&t.gold, "#ffb08a", 0.45),
                    _ => mixhex(&t.gold, "#ff8a5a", 0.45),
                };
                let sun = mixhex(&sun, &top, 0.30);
                let a = |c: &str, k: f64| Some((c.to_string(), k));
                let img = aa_pixels(72, 72, &[(Shape::Circle(36.0, 36.0, 33.0), a(&sun, 0.05)),
                                             (Shape::Circle(36.0, 36.0, 25.0), a(&sun, 0.08)),
                                             (Shape::Circle(36.0, 36.0, 17.0), a(&mixhex(&top, &sun, 0.55), 1.0)),
                                             (Shape::Circle(36.0, 36.0, 14.0), a(&sun, 1.0))], None);
                self.sun = Some((key, Rc::new(img)));
            }
            let (mx, my) = (wf - 46.0, 46.0 + if matches!(part, "dawn" | "dusk") { 18.0 } else { 0.0 });
            self.z19_moon(false);
            let img = self.sun.as_ref().unwrap().1.clone();
            self.cv.image(mx - 36.0, my - 36.0, img).anchor("nw").tags(&["z19sky", "bg"]);
        } else {
            self.z19_moon(true);
            for &(it, _, _) in &self.stars {
                if let Some(i) = self.cv.get(it) {
                    i.hidden = false;
                }
            }
        }
        let season = c.season.as_str();
        if c.seasons {
            let tg = ["z19sky", "bg"];
            if season == "winter" || weather == Some("snow") {
                let snow = mixhex("#e8eeff", &top, 0.25);
                let mut pts = vec![];
                let mut x = 0;
                while x < wd + 12 {
                    let xf = x as f64;
                    pts.extend([xf, hf - 36.0 - 10.0 * (xf / 27.0 + 1.3).sin()]);
                    x += 12;
                }
                self.cv.line(pts).fill(&snow).width(2.0).tags(&tg);
                let (tx, ty) = (40.0, hf - 40.0);
                self.cv.rect(tx - 2.0, ty - 33.0, tx + 39.0, ty - 31.0).fill(&snow).outline("").tags(&tg);
                for lx in [wf - 60.0, wf - 30.0] {
                    self.cv.rect(lx - 1.0, hf - 46.0, lx + 7.0, hf - 44.0).fill(&snow).outline("").tags(&tg);
                }
            } else if season == "spring" || season == "autumn" {
                let wood = mixhex("#6a4430", &top, 0.30);
                let wood2 = mixhex(&wood, "#000000", 0.35);
                let branches: [(&[(f64, f64)], f64); 4] = [(&[(-4.0, 14.0), (14.0, 19.0), (30.0, 27.0), (46.0, 31.0), (60.0, 40.0)], 4.0),
                                                            (&[(14.0, 19.0), (21.0, 8.0)], 2.0), (&[(30.0, 27.0), (35.0, 43.0)], 2.0),
                                                            (&[(46.0, 31.0), (54.0, 24.0)], 2.0)];
                for (pts, wd_) in branches {
                    let flat: Vec<f64> = pts.iter().flat_map(|p| [p.0, p.1]).collect();
                    self.cv.line(flat.clone()).fill(&wood2).width(wd_ + 1.0).smooth().cap("round").tags(&tg);
                    self.cv.line(flat).fill(&wood).width(wd_).smooth().cap("round").tags(&tg);
                }
                let mut rnd = PyRandom::new(19);
                let cols: [&str; 3] = if season == "spring" { ["#f7b3cf", "#ee8db5", "#fde8f1"] } else { ["#c9552e", "#d9803a", "#a8402c"] };
                let shape: &[(f64, f64)] = if season == "spring" {
                    &[(0.0, -2.0), (-2.0, 0.0), (0.0, 0.0), (2.0, 0.0), (0.0, 2.0)]
                } else {
                    &[(0.0, -2.0), (-2.0, -1.0), (2.0, -1.0), (-1.0, 0.0), (0.0, 0.0), (1.0, 0.0), (0.0, 1.0), (0.0, 2.0)]
                };
                for (x, y) in [(10.0, 13.0), (18.0, 21.0), (21.0, 9.0), (25.0, 25.0), (31.0, 29.0), (35.0, 43.0), (41.0, 32.0),
                               (47.0, 29.0), (54.0, 24.0), (58.0, 40.0), (63.0, 43.0), (8.0, 20.0), (38.0, 36.0), (51.0, 36.0)] {
                    let cc = mixhex(rnd.choice(&cols), &top, 0.30);
                    for &(dx, dy) in shape {
                        self.cv.rect(x + dx * 1.5, y + dy * 1.5, x + dx * 1.5 + 2.0, y + dy * 1.5 + 2.0).fill(&cc).outline("")
                            .tags(&tg);
                    }
                    let hi = mixhex(&cc, if season == "spring" { "#fff3c0" } else { "#000000" }, 0.35);
                    self.cv.rect(x, y, x + 2.0, y + 2.0).fill(&hi).outline("").tags(&tg);
                }
            }
        }
        for tg in ["logo", "nav", "deco", "flake", "status"] {
            self.cv.tag_raise(tg);
        }
    }

    fn z22_side_extras(&mut self, c: &SideCtx) {
        let (wd, hh) = (self.w, self.h);
        if wd < 50 || hh < 200 {
            return;
        }
        let t = &c.t;
        let (wf, hf) = (wd as f64, hh as f64);
        self.cv.delete("z22side");
        if !c.scene.is_empty() {
            self.z22_side_scene(c, wf, hf);
        }
        let (_, _, _, nav_y, rowh) = self.layout(c.nav_compact);
        let end = nav_y + self.navkeys.len() as i32 * rowh + 6;
        let y = (end.max(hh - 150) + 30) as f64;
        if y + 30.0 > hf - 88.0 {
            return;
        }
        let mut x = 26.0;
        let tg = ["z22side", "status"];
        for (_bid, kanji, name, tier) in &c.pins {
            let edge = if *tier > 0 { BADGE_TIER_COL[(*tier as usize).min(3)] } else { "#7a4a1c" };
            let pts = vec![x + 12.0, y, x + 24.0, y + 6.0, x + 24.0, y + 26.0, x, y + 26.0, x, y + 6.0];
            self.cv.poly(pts).fill("#c98a35").outline(edge).width(2.0).tags(&tg);
            let (txt, fam) = if !c.jp.is_empty() {
                (kanji.clone(), c.jp.clone())
            } else {
                (name.chars().take(1).collect(), c.font_ui.clone())
            };
            self.cv.text(x + 12.0, y + 16.0, &txt).fill("#3a2414").font(Font::new(&fam, 10, true)).tags(&tg);
            x += 30.0;
        }
        if c.linked {
            let lx = if !c.pins.is_empty() { (x + 8.0).max(wf - 92.0) } else { 26.0 };
            self.cv.text(lx + 14.0, y + 13.0, "Browser").anchor("w").fill(&mixhex(&t.gold, "#ffffff", 0.2))
                .font(Font::new(&c.font_ui, 9, false)).tags(&tg);
            self.cv.oval(lx - 2.0, y + 7.0, lx + 10.0, y + 19.0).fill("").outline(&mixhex("#7fd8ff", &t.night, 0.5)).width(2.0)
                .tags(&tg);
            self.cv.oval(lx + 1.0, y + 10.0, lx + 7.0, y + 16.0).fill("#7fd8ff").outline("").tags(&tg);
        }
        self.cv.tag_raise("z22side");
    }

    fn z22_side_scene(&mut self, c: &SideCtx, wf: f64, hf: f64) {
        let t = &c.t;
        let base = t.night2.clone();
        let far = mixhex(&base, "#000000", 0.25);
        let near = mixhex(&base, "#000000", 0.42);
        let gold = mixhex(&t.gold, &base, 0.55);
        let (y0, x0) = (hf - 34.0, wf * 0.52);
        let tg = ["z22side", "bg"];
        let cv = &mut self.cv;
        match c.scene.as_str() {
            "fuji" => {
                cv.poly(vec![x0 - 40.0, y0, x0 + 30.0, y0 - 58.0, x0 + 46.0, y0 - 58.0, x0 + 120.0, y0]).fill(&far).outline("").tags(&tg);
                cv.poly(vec![x0 + 18.0, y0 - 44.0, x0 + 30.0, y0 - 58.0, x0 + 46.0, y0 - 58.0, x0 + 58.0, y0 - 44.0, x0 + 48.0,
                             y0 - 48.0, x0 + 38.0, y0 - 42.0, x0 + 28.0, y0 - 48.0])
                    .fill(&mixhex("#e8eeff", &base, 0.45)).outline("").tags(&tg);
            }
            "kinkaku" => {
                for (yy, w_, h_) in [(0.0, 64.0, 14.0), (14.0, 56.0, 12.0), (26.0, 44.0, 12.0)] {
                    cv.rect(x0 + 30.0 - w_ / 2.0, y0 - yy - h_, x0 + 30.0 + w_ / 2.0, y0 - yy).fill(&gold).outline("").tags(&tg);
                    cv.poly(vec![x0 + 30.0 - w_ / 2.0 - 8.0, y0 - yy - h_, x0 + 30.0, y0 - yy - h_ - 7.0, x0 + 30.0 + w_ / 2.0 + 8.0,
                                 y0 - yy - h_]).fill(&near).outline("").tags(&tg);
                }
                cv.line(vec![x0 + 30.0, y0 - 59.0, x0 + 30.0, y0 - 68.0]).fill(&gold).width(2.0).tags(&tg);
            }
            "pagoda" => {
                for k in 0..5 {
                    let w_ = 34.0 - k as f64 * 5.0;
                    let yy = y0 - k as f64 * 12.0;
                    cv.rect(x0 + 30.0 - w_ / 2.0 + 4.0, yy - 10.0, x0 + 30.0 + w_ / 2.0 - 4.0, yy).fill(&far).outline("").tags(&tg);
                    cv.poly(vec![x0 + 30.0 - w_ / 2.0 - 4.0, yy - 9.0, x0 + 30.0, yy - 14.0, x0 + 30.0 + w_ / 2.0 + 4.0, yy - 9.0])
                        .fill(&near).outline("").tags(&tg);
                }
                cv.line(vec![x0 + 30.0, y0 - 62.0, x0 + 30.0, y0 - 76.0]).fill(&gold).width(2.0).tags(&tg);
            }
            "greatwall" => {
                let yk = |k: i32| y0 - 22.0 - 14.0 * (k as f64 * 0.55).sin();
                let pts: Vec<f64> = (0..15).flat_map(|k| [x0 - 30.0 + k as f64 * 11.0, yk(k)]).collect();
                cv.line(pts).fill(&far).width(7.0).smooth().tags(&tg);
                for k in (0..15).step_by(2) {
                    let (xx, yy) = (x0 - 30.0 + k as f64 * 11.0, yk(k));
                    cv.rect(xx - 2.0, yy - 8.0, xx + 2.0, yy - 3.0).fill(&far).outline("").tags(&tg);
                }
                for k in [3, 10] {
                    let (xx, yy) = (x0 - 30.0 + k as f64 * 11.0, yk(k));
                    cv.rect(xx - 7.0, yy - 18.0, xx + 7.0, yy).fill(&near).outline("").tags(&tg);
                }
            }
            "dunes" => {
                cv.poly(vec![x0 - 40.0, y0, x0 + 10.0, y0 - 26.0, x0 + 70.0, y0 - 8.0, x0 + 130.0, y0 - 30.0, x0 + 160.0, y0])
                    .fill(&mixhex("#c8a060", &base, 0.6)).outline("").smooth().tags(&tg);
                for (k, xx) in [x0 + 20.0, x0 + 42.0].into_iter().enumerate() {
                    let yy = y0 - 20.0 - k as f64 * 3.0;
                    cv.poly(vec![xx, yy, xx + 4.0, yy - 7.0, xx + 7.0, yy - 4.0, xx + 10.0, yy - 7.0, xx + 14.0, yy, xx + 16.0, yy - 9.0,
                                 xx + 18.0, yy - 9.0, xx + 16.0, yy, xx + 14.0, yy + 6.0, xx + 12.0, yy, xx + 4.0, yy, xx + 2.0, yy + 6.0])
                        .fill(&near).outline("").tags(&tg);
                }
            }
            "moon" => {
                cv.oval(x0 + 4.0, y0 - 92.0, x0 + 56.0, y0 - 40.0).fill(&mixhex("#f4ecce", &base, 0.35)).outline("").tags(&tg);
                for (yy, w_, h_) in [(0.0, 70.0, 10.0), (10.0, 50.0, 10.0), (20.0, 30.0, 10.0)] {
                    cv.rect(x0 + 30.0 - w_ / 2.0, y0 - yy - h_, x0 + 30.0 + w_ / 2.0, y0 - yy).fill(&near).outline("").tags(&tg);
                    cv.poly(vec![x0 + 30.0 - w_ / 2.0 - 6.0, y0 - yy - h_, x0 + 30.0, y0 - yy - h_ - 6.0, x0 + 30.0 + w_ / 2.0 + 6.0,
                                 y0 - yy - h_]).fill(&far).outline("").tags(&tg);
                }
            }
            _ => {}
        }
        if !cv.find_withtag("nav").is_empty() {
            cv.tag_lower("z22side", Some("nav"));
        }
    }
}

/// zk_size(base) with the window's text factor
pub fn zk_size(f: f64, base: i32) -> i32 {
    let neg = base < 0;
    let b = base.abs();
    let mut n = (b as f64 * f).round_ties_even() as i32;
    if f < 1.0 {
        n = n.max(b.min(8));
    }
    n = 6.max(64.min(n));
    if neg { -n } else { n }
}

/// round_pts(x1, y1, x2, y2, r, step, SHAPES[shape](r)) as the flat float list Tk is given
pub fn round_pts_flat(x1: f64, y1: f64, x2: f64, y2: f64, r: i32, step: i32, shape: &str) -> Vec<f64> {
    crate::shapes::round_pts_raw(x1, y1, x2, y2, r, step, shape).into_iter().flat_map(|(x, y)| [x, y]).collect()
}
