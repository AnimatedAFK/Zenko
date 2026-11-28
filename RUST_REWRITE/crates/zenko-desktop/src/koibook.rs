//! The koi book window's contents (zenko.py Zenko22.koi_book): a banner with the count, a scrolling grid of every
//! fish (caught ones in colour, the rest as dark silhouettes) and a detail panel for the picked fish (its picture,
//! names, rarity, catches and note with a pond toggle, or a hint where it bites).

use crate::fishart::{fish_all, fish_by_id, fish_rgba, gdata};
use crate::gardenpage::{cm_txt, fish_book_l, zoom_px};
use crate::hubwin::canvas_of;
use crate::pagekit::{Faces, cm, tt_btn};
use crate::shapes::round_pts_raw;
use crate::theme::Theme;
use crate::tkcanvas::Photo;
use crate::ui::{Fill, Font, Fonts, Kind, Pack, Side, Ui};
use serde_json::Value;
use std::collections::HashMap;
use std::rc::Rc;
use zenko_art::palette::mixhex;

pub const TITLE: &str = "Koi book \u{b7} \u{9bc9}\u{56f3}\u{9451}";

/// the window's size and place on a screen this big: min(980, sw - 80) x min(680, sh - 100), centred, a third down
pub fn geometry(sw: i32, sh: i32) -> (i32, i32, i32, i32) {
    let (w, h) = (980.min(sw - 80), 680.min(sh - 100));
    (w, h, 0.max((sw - w).div_euclid(2)), 0.max((sh - h).div_euclid(3)))
}

pub struct KoiBook {
    pub cv: usize,
    pub sb: usize,
    pub right: usize,
    pub sel: Option<String>,
    /// the canvas's yview offset (pixels) and the scrollregion's height
    pub yoff: i32,
    pub region: i32,
    /// tiles: (x1, y1, x2, y2) in scrollregion coordinates, by fish
    tiles: Vec<(f64, f64, f64, f64)>,
    pub fishing: Value,
    imgs: HashMap<(String, i64, i64, i64), Rc<Photo>>,
    th: Theme,
    fu: String,
    jp: String,
    serif: String,
    pub clicks: Vec<(usize, String)>,
}

fn s(v: &Value) -> String {
    v.as_str().unwrap_or("").to_string()
}

impl KoiBook {
    /// koi_book(): `jp` is _mincho_font() or _jp_font() or FONT_UI, `serif` _serif_font()
    pub fn build(ui: &mut Ui, th: &Theme, fu: &str, jp: &str, serif: &str, fishing: &Value, cm10: i32) -> KoiBook {
        let root = 0;
        ui.ws[root].bg = th.paper.clone();
        let head = ui.frame(root, &th.banner);
        ui.pack(head, Pack::default().fill(Fill::X));
        let l = ui.label(head, "\u{9bc9}\u{56f3}\u{9451}  Koi book", &th.banner, &th.gold, Font::new(fu, 16, true));
        ui.pack(l, Pack::default().side(Side::Left).padx(18, 18).pady(10, 10));
        let book = fishing["book"].as_object().cloned().unwrap_or_default();
        let n: i64 = book.values().map(|v| v["n"].as_i64().unwrap_or(0)).sum();
        let l = ui.label(head, &format!("{} of {} kinds  \u{b7}  {} caught", book.len(), fish_all().len(), n), &th.banner, &th.ink,
                         Font::new(fu, 10, false));
        ui.pack(l, Pack::default().side(Side::Left).padx(6, 6));
        let body = ui.frame(root, &th.paper);
        ui.pack(body, Pack::default().fill(Fill::Both).expand().padx(14, 14).pady(12, 12));
        let left = ui.frame(body, &th.paper);
        ui.pack(left, Pack::default().side(Side::Left).fill(Fill::Both).expand());
        let right = ui.frame(body, &th.card);
        ui.ws[right].opt_w = 300;
        ui.ws[right].hl = 1;
        ui.ws[right].hlc = th.line.clone();
        ui.ws[right].propagate = false;
        ui.pack(right, Pack::default().side(Side::Left).fill(Fill::Y).padx(12, 0));
        let (cw, ch) = cm(cm10);
        let cv = ui.canvas(left, &th.paper, cw, ch);
        canvas_of(ui, cv);
        let sb = ui.canvas(left, &th.paper, 10, 40);
        canvas_of(ui, sb);
        ui.pack(sb, Pack::default().side(Side::Right).fill(Fill::Y));
        ui.pack(cv, Pack::default().side(Side::Left).fill(Fill::Both).expand());
        let mut k = KoiBook { cv, sb, right, sel: book.keys().next().cloned(), yoff: 0, region: 0, tiles: vec![],
                              fishing: fishing.clone(), imgs: HashMap::new(), th: th.clone(), fu: fu.into(), jp: jp.into(),
                              serif: serif.into(), clicks: vec![] };
        k.detail(ui);
        k
    }

    fn photo(&mut self, fid: &str, seed: i64, l: i64, zoom: i64, dark: bool) -> Rc<Photo> {
        let key = (fid.to_string(), seed, l, zoom * if dark { -1 } else { 1 });
        if let Some(p) = self.imgs.get(&key) {
            return p.clone();
        }
        let (w, h, mut px) = fish_rgba(fid, seed, l, 0.0, 1.0);
        if dark {
            for p in px.iter_mut() {
                if p[3] != 0 {
                    *p = [30, 36, 60, p[3]];
                }
            }
        }
        let p = Rc::new(zoom_px(w, h, &px, zoom as usize));
        self.imgs.insert(key, p.clone());
        p
    }

    /// detail(): the picked fish in the right-hand panel
    pub fn detail(&mut self, ui: &mut Ui) {
        let right = self.right;
        for k in ui.ws[right].kids.clone() {
            ui.destroy(k);
        }
        self.clicks.clear();
        let (th, fu) = (self.th.clone(), self.fu.clone());
        let ff = Faces { ui: fu.clone(), title: String::new() };
        let bg = th.card.clone();
        let serif = self.serif.clone();
        let it = |n: i32| Font { family: serif.clone(), size: n, bold: false, italic: true };
        let wrap = |ui: &mut Ui, l: usize, w: i32| {
            if let Kind::Label { wrap, .. } = &mut ui.ws[l].kind {
                *wrap = w;
            }
        };
        let Some(fid) = self.sel.clone() else {
            let l = ui.label(right, "Catch a fish in the Zen garden pond and it appears here~", &bg, &th.grey, it(11));
            wrap(ui, l, 260);
            ui.pack(l, Pack::default().pady(40, 40).padx(16, 16));
            return;
        };
        let Some(fi) = fish_by_id(&fid).cloned() else { return };
        let g = gdata();
        let b = self.fishing["book"].get(&fid).cloned();
        if let Some(b) = &b {
            let bl = fish_book_l(&fid);
            let img = self.photo(&fid, b["seed"].as_i64().unwrap_or(1), 130.min(bl), if bl <= 64 { 2 } else { 1 }, false);
            let il = ui.image_label(right, img, &mixhex(&bg, "#0e1a3a", 0.5), 2);
            (ui.ws[il].padx, ui.ws[il].pady) = (10, 10);
            ui.pack(il, Pack::default().pady(16, 8).padx(12, 12).fill(Fill::X));
        }
        let l = ui.label(right, &s(&fi[1]), &bg, &mixhex(&th.gold, &th.ink, 0.25), Font::new(&self.jp, 22, false));
        ui.pack(l, Pack::default());
        let l = ui.label(right, if b.is_some() { fi[2].as_str().unwrap_or("") } else { "???" }, &bg, &th.ink,
                         Font::new(&serif, 13, false));
        ui.pack(l, Pack::default());
        let r = fi[3].as_i64().unwrap_or(1) as usize;
        let l = ui.label(right, &s(&g["FISH_RARITY"][r]), &bg, &s(&g["FISH_RCOL"][r]), Font::new(&fu, 10, true));
        ui.pack(l, Pack::default().pady(2, 6));
        if let Some(b) = &b {
            let l = ui.label(right, &format!("Caught {}\u{d7}  \u{b7}  biggest {} cm\nfirst caught {}", b["n"].as_i64().unwrap_or(0),
                                            cm_txt(b["best"].as_f64().unwrap_or(0.0)), s(&b["first"])),
                             &bg, &th.ink, Font::new(&fu, 9, false));
            ui.pack(l, Pack::default());
            let l = ui.label(right, &s(&fi[8]), &bg, &th.grey, Font::new(&fu, 9, false));
            wrap(ui, l, 260);
            ui.pack(l, Pack::default().pady(8, 8).padx(12, 12));
            let inpond = self.fishing["pond"].as_array().is_some_and(|p| p.iter().any(|p| p[0].as_str() == Some(fid.as_str())));
            let bt = tt_btn(ui, right, if inpond { "Take out of the pond" } else { "Show in my pond" }, "koi_pond",
                            Some(if inpond { &th.field } else { &th.acc }), Some(if inpond { &th.ink } else { &th.onacc }),
                            10, 12, 2, &th, &ff);
            ui.pack(bt, Pack::default());
            let l = ui.label(right, "Up to 6 fish swim in the pond.", &bg, &th.grey, Font::new(&fu, 8, false));
            ui.pack(l, Pack::default().pady(4, 0));
        } else {
            let c = &fi[6];
            let on = |k: &str| c.get(k).is_some_and(|v| v.as_bool().unwrap_or(false) || v.as_i64().unwrap_or(0) != 0 || v.is_string());
            let mut hint: Vec<String> = vec![];
            if on("night") {
                hint.push("at night".into());
            }
            if on("day") {
                hint.push("in daylight".into());
            }
            if on("rain") {
                hint.push("in the rain".into());
            }
            if on("snow_or_winter") {
                hint.push("in snow or winter".into());
            }
            if let Some(se) = c["season"].as_str().filter(|v| !v.is_empty()) {
                hint.push(format!("in {se}"));
            }
            if let Some(l) = c["lantern"].as_i64().filter(|v| *v != 0) {
                hint.push(format!("with a {}", s(&g["FISH_LANTERNS"][l as usize][1]).to_lowercase()));
            }
            if let Some(r) = c["rod"].as_i64().filter(|v| *v != 0) {
                hint.push(format!("with a {}", s(&g["FISH_RODS"][r as usize][1]).to_lowercase()));
            }
            let dp = c["depth"].as_i64().unwrap_or(1);
            if dp >= 2 {
                hint.push(format!("in the {}", s(&g["FISH_DEPTHS"][(dp - 1) as usize][1])));
            }
            if let Some(pl) = c["place"].as_str().filter(|v| !v.is_empty()) {
                let name = crate::rewards::rdata()["JOURNEY_ALL"].as_array().and_then(|a| a.iter().find(|p| p[0] == pl))
                    .map(|p| s(&p[2])).unwrap_or_default();
                hint.push(format!("after you reach {name}"));
            }
            let t = if hint.is_empty() { "Keep casting~ it's out there.".to_string() } else { format!("Bites {}.", hint.join(", ")) };
            let l = ui.label(right, &t, &bg, &th.grey, it(10));
            wrap(ui, l, 250);
            ui.pack(l, Pack::default().pady(10, 10).padx(12, 12));
        }
    }

    /// grid(): every fish as a tile (per = max(2, W // 150) a row, 112 px tall), drawn at the view's offset
    pub fn grid(&mut self, ui: &mut Ui) {
        let wc = ui.ws[self.cv].w;
        if wc < 100 {
            return;
        }
        let per = 2.max(wc / 150) as usize;
        let (cw, ch) = (wc as f64 / per as f64, 112.0);
        let (th, fu) = (self.th.clone(), self.fu.clone());
        let bg = th.card.clone();
        let mut items: Vec<(f64, f64, f64, f64, bool, bool, String, i64, i64, bool)> = vec![];
        self.tiles.clear();
        for (i, f) in fish_all().iter().enumerate() {
            let (x, y) = ((i % per) as f64 * cw, (i / per) as f64 * ch);
            let fid = s(&f[0]);
            let b = self.fishing["book"].get(&fid).cloned();
            let shape = f[4].as_str().unwrap_or("");
            let l = ((cw - 30.0) as i64).min(if shape != "oar" { fish_book_l(&fid) } else { (cw - 24.0) as i64 });
            let ll = 24.max(if shape != "medaka" { l } else { l.div_euclid(2) });
            let seed = b.as_ref().and_then(|b| b["seed"].as_i64()).unwrap_or(1);
            items.push((x, y, cw, ch, b.is_some(), self.sel.as_deref() == Some(fid.as_str()), fid, ll, seed,
                        f[3].as_i64().is_some()));
            self.tiles.push((x + 4.0, y + 4.0, x + cw - 4.0, y + ch - 4.0));
        }
        let rows = fish_all().len().div_ceil(per);
        self.region = (rows as f64 * ch + 8.0) as i32;
        let top = (self.region - ui.ws[self.cv].h).max(0);
        self.yoff = self.yoff.clamp(0, top);
        let yo = self.yoff as f64;
        let g = gdata();
        let mut pics = vec![];
        for it in &items {
            pics.push(if it.4 { self.photo(&it.6, it.8, it.7, 1, false) } else { self.photo(&it.6, 1, it.7, 1, true) });
        }
        let cv = canvas_of(ui, self.cv);
        cv.delete("all");
        for (i, (it, img)) in items.iter().zip(pics).enumerate() {
            let (x, y, cw, ch, got, on, ref fid, _, _, _) = *it;
            let y = y - yo;
            let f = fish_by_id(fid).unwrap();
            let r = f[3].as_i64().unwrap_or(1) as usize;
            let rcol = s(&g["FISH_RCOL"][r]);
            let tag = format!("t{i}");
            let pts: Vec<f64> = round_pts_raw(x + 4.0, y + 4.0, x + cw - 4.0, y + ch - 4.0, 12, 2, "round").into_iter()
                .flat_map(|(a, b)| [a, b]).collect();
            cv.poly(pts).fill(&if on { mixhex(&bg, &th.acc, 0.18) } else { bg.clone() })
                .outline(if got { &rcol } else { &th.line }).width(if on { 2.0 } else { 1.0 }).join("miter").tags(&[&tag]);
            cv.image(x + cw / 2.0, y + 44.0, img).tags(&[&tag]);
            cv.text(x + cw / 2.0, y + 82.0, if got { f[2].as_str().unwrap_or("") } else { "???" })
                .fill(if got { &th.ink } else { &th.grey }).font(Font::new(&fu, 9, true)).tags(&[&tag]);
            let n = self.fishing["book"][fid.as_str()]["n"].as_i64().unwrap_or(0);
            cv.text(x + cw / 2.0, y + 97.0, &if got { format!("\u{d7}{n}") } else { s(&g["FISH_RARITY"][r]) })
                .fill(if got { &rcol } else { &th.grey }).font(Font::new(&fu, 8, false)).tags(&[&tag]);
        }
        self.bar(ui);
    }

    /// the ZkScrollbar's thumb for the canvas's view
    pub fn bar(&self, ui: &mut Ui) {
        let h = ui.ws[self.cv].h as f64;
        let rh = self.region.max(1) as f64;
        let y = self.yoff as f64;
        let (a, b) = ((y / rh).max(0.0), ((y + h) / rh).min(1.0));
        crate::chatpage::scrollbar_draw(ui, self.sb, a, b, false, &self.th);
    }

    /// yview_scroll(int(-delta / 120), "units"): a tenth of the canvas a notch
    pub fn wheel(&mut self, ui: &mut Ui, delta: i32) {
        let n = -delta / 120;
        if n == 0 {
            return;
        }
        let unit = ui.ws[self.cv].h / 10;
        let top = (self.region - ui.ws[self.cv].h).max(0);
        let y = (self.yoff + n * unit).clamp(0, top);
        if y != self.yoff {
            self.yoff = y;
            self.grid(ui);
        }
    }

    /// a click inside the canvas at (mx, my) window pixels: the fish whose tile it is gets picked
    pub fn click(&mut self, ui: &mut Ui, mx: i32, my: i32) -> bool {
        let (ax, ay) = ui.abs(self.cv);
        let (x, y) = ((mx - ax) as f64, (my - ay + self.yoff) as f64);
        if mx < ax || my < ay || mx >= ax + ui.ws[self.cv].w || my >= ay + ui.ws[self.cv].h {
            return false;
        }
        let Some(i) = self.tiles.iter().position(|t| x >= t.0 && x <= t.2 && y >= t.1 && y <= t.3) else { return false };
        self.sel = fish_all()[i][0].as_str().map(String::from);
        self.grid(ui);
        self.detail(ui);
        true
    }

    /// "Show in my pond" / "Take out of the pond": the new pond list (up to 6, newest first)
    pub fn toggle_pond(&mut self) -> Option<Value> {
        let fid = self.sel.clone()?;
        let b = self.fishing["book"].get(&fid)?.clone();
        let pond: Vec<Value> = self.fishing["pond"].as_array().cloned().unwrap_or_default();
        let inpond = pond.iter().any(|p| p[0].as_str() == Some(fid.as_str()));
        let mut np: Vec<Value> = pond.into_iter().filter(|p| p[0].as_str() != Some(fid.as_str())).collect();
        if !inpond {
            np.insert(0, serde_json::json!([fid, b["seed"].as_i64().unwrap_or(1)]));
        }
        np.truncate(6);
        self.fishing["pond"] = Value::Array(np.clone());
        Some(Value::Array(np))
    }

    /// lay out at the window's size and draw the grid for it
    pub fn fit(&mut self, ui: &mut Ui, fonts: &dyn Fonts, w: i32, h: i32) {
        ui.layout_at(fonts, w, h);
        self.grid(ui);
    }
}
