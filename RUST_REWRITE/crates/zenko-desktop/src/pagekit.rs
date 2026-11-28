//! The building blocks every hub page uses (zenko.py FoxBuddy.hub_card / round_frame / tt_btn / tt_entry /
//! hub_scroller with ZkScrollbar, glow_bar) and Zenko14's zk_fit for a page: the font pass (zk_font / zk_rescale
//! from the size each widget was made with) and zk_wrap (long labels wrap to the room they really have, two columns
//! that don't fit are stacked).

use crate::hubside::zk_size;
use crate::shapes::round_pts_raw;
use crate::theme::Theme;
use crate::tkcanvas::Canvas;
use crate::ui::{Anchor, Fill, Font, Fonts, Grid, Kind, Pack, Side, Ui};
use zenko_art::palette::mixhex;

/// FONT_TITLE (empty: none installed) and FONT_UI
#[derive(Clone, Debug)]
pub struct Faces {
    pub ui: String,
    pub title: String,
}

/// tk.Label(parent, text, bg, fg, font=(family, size[, "bold"]), anchor="w")
#[allow(clippy::too_many_arguments)]
pub fn label_w(ui: &mut Ui, parent: usize, text: &str, bg: &str, fg: &str, family: &str, size: i32, bold: bool) -> usize {
    let l = ui.label(parent, text, bg, fg, Font::new(family, size, bold));
    ui.set_anchor(l, Anchor::W);
    l
}

/// FoxBuddy.hub_card(parent, title, bg, shape): a round_frame card (radius 14 for a leaf, else 12) and, with a title,
/// a row with a gold diamond and the title
pub fn hub_card(ui: &mut Ui, parent: usize, title: Option<&str>, shape: &'static str, th: &Theme, f: &Faces) -> usize {
    let bg = th.card.clone();
    let c = ui.round_frame(parent, &bg, &th.line, if shape == "leaf" { 14 } else { 12 }, shape, 4, &th.ink);
    if let Some(t) = title {
        let row = ui.frame(c, &bg);
        ui.pack(row, Pack::default().fill(Fill::X).padx(16, 16).pady(12, 6));
        let d = ui.label(row, "\u{25c6}", &bg, &th.gold, Font::new(&f.ui, 7, false));
        ui.pack(d, Pack::default().side(Side::Left).padx(0, 6));
        let (fam, n) = if f.title.is_empty() { (f.ui.as_str(), 11) } else { (f.title.as_str(), 13) };
        let l = label_w(ui, row, t, &bg, &th.gold, fam, n, true);
        ui.pack(l, Pack::default().side(Side::Left));
    }
    c
}

/// tt_btn(parent, text, cmd, bg, fg, size, padx, pady, enabled): a PixelButton (bg defaults to FIELD)
#[allow(clippy::too_many_arguments)]
pub fn tt_btn(ui: &mut Ui, parent: usize, text: &str, cmd: &str, bg: Option<&str>, fg: Option<&str>, size: i32, padx: i32,
              pady: i32, th: &Theme, f: &Faces) -> usize {
    let bg = match bg {
        None | Some("white") => th.field.clone(),
        Some(b) => b.to_string(),
    };
    ui.button(parent, text, cmd, &bg, fg.unwrap_or(&th.ink), &f.ui, size, padx, pady, true)
}

/// tt_entry(parent, var, width): FIELD background, (FONT_UI, 11), a 1-pixel highlight ring
pub fn tt_entry(ui: &mut Ui, parent: usize, text: &str, chars: i32, th: &Theme, f: &Faces) -> usize {
    ui.entry(parent, chars, Font::new(&f.ui, 11, false), text, &th.field)
}

/// hub_scroller(pg): a box (grid: the canvas, and a ZkScrollbar that is only gridded while the inside is taller
/// than the canvas) with the inside frame as the canvas's window item
#[derive(Clone, Debug)]
pub struct Scroller {
    pub boxf: usize,
    pub cv: usize,
    pub sb: usize,
    pub inner: usize,
}

/// a tk.Canvas's default size: 10c x 7c at the screen's resolution
pub fn cm(px_per_10cm: i32) -> (i32, i32) {
    (px_per_10cm, (px_per_10cm as f64 * 0.7).round() as i32)
}

pub fn hub_scroller(ui: &mut Ui, pg: usize, th: &Theme, cm10: i32) -> Scroller {
    let boxf = ui.frame(pg, &th.paper);
    ui.pack(boxf, Pack::default().fill(Fill::Both).expand());
    ui.ws[boxf].gw_rows = vec![1];
    ui.ws[boxf].gw_cols = vec![1];
    let (w, h) = cm(cm10);
    let cv = ui.canvas(boxf, &th.paper, w, h);
    let sb = ui.canvas(boxf, &th.paper, 10, 40);
    ui.grid(cv, Grid { row: 0, col: 0, sticky: "nsew", ..Default::default() });
    ui.grid(sb, Grid { row: 0, col: 1, sticky: "ns", ..Default::default() });
    let inner = ui.frame(cv, &th.paper);
    ui.ws[inner].cwin = true;
    Scroller { boxf, cv, sb, inner }
}

impl Scroller {
    /// conf(): the scrollbar is gridded while the inside is taller than the canvas (+2), else grid_remove'd.
    /// True when that changed (lay out again).
    pub fn sync(&self, ui: &mut Ui) -> bool {
        let need = ui.ws[self.inner].rh > ui.ws[self.cv].h + 2;
        let has = ui.ws[self.sb].grid.is_some();
        if need == has {
            return false;
        }
        ui.ws[self.sb].grid = need.then(|| Grid { row: 0, col: 1, sticky: "ns", ..Default::default() });
        true
    }
}

impl Scroller {
    /// the canvas's yview over its scrollregion (the inside frame)
    pub fn yview(&self, ui: &Ui) -> (f64, f64) {
        let (rh, h) = (ui.ws[self.inner].rh.max(1) as f64, ui.ws[self.cv].h as f64);
        let y = ui.ws[self.cv].yoff as f64;
        ((y / rh).max(0.0), ((y + h) / rh).min(1.0))
    }

    /// yview moveto, kept inside the scrollregion (confine)
    pub fn set_y(&self, ui: &mut Ui, y: i32) {
        let top = (ui.ws[self.inner].rh - ui.ws[self.cv].h).max(0);
        ui.ws[self.cv].yoff = y.clamp(0, top);
    }

    /// z17_wheel over the page: `n` notches of a tenth of the canvas each, only while there's somewhere to go.
    /// True when it moved (lay out again).
    pub fn wheel(&self, ui: &mut Ui, delta: i32) -> bool {
        let (a, b) = self.yview(ui);
        let d = if delta > 0 { -1 } else { 1 };
        let n = 1.max(delta.abs() / 120);
        if (a > 0.0005 || b < 0.9995) && !((d < 0 && a <= 0.0005) || (d > 0 && b >= 0.9995)) {
            let unit = ui.ws[self.cv].h / 10;
            let y = ui.ws[self.cv].yoff + d * n * unit;
            self.set_y(ui, y);
            return true;
        }
        false
    }

    /// the ZkScrollbar's thumb for the view now
    pub fn draw_bar(&self, ui: &mut Ui, hover: bool, th: &Theme) {
        if ui.ws[self.sb].grid.is_some() {
            let (a, b) = self.yview(ui);
            crate::chatpage::scrollbar_draw(ui, self.sb, a, b, hover, th);
        }
    }
}

/// glow_bar(cv, x1, y1, x2, y2, frac): a dark trough, the foxfire fill with a bright head
pub fn glow_bar(cv: &mut Canvas, x1: f64, y1: f64, x2: f64, y2: f64, frac: f64, th: &Theme) {
    let pts = |a: f64, b: f64, c: f64, d: f64| round_pts_raw(a, b, c, d, 99, 2, "pill").into_iter().flat_map(|(x, y)| [x, y]).collect();
    cv.poly(pts(x1, y1, x2, y2)).fill(&mixhex(&th.card, "#000000", 0.25)).outline(&th.line).join("miter");
    if frac > 0.0 {
        let fx = x1 + 1.0 + (x2 - x1 - 2.0) * frac.min(1.0);
        if fx - x1 > 6.0 {
            cv.poly(pts(x1 + 1.0, y1 + 1.0, fx, y2 - 1.0)).fill(&th.acc).outline("").join("miter");
            cv.line(vec![x1 + 5.0, y1 + 2.0, fx - 4.0, y1 + 2.0]).fill(&mixhex(&th.acc, "#ffffff", 0.4));
        }
        cv.oval(fx - 3.0, y1 - 1.0, fx + 3.0, y2 + 1.0).outline("").fill(&th.glow);
    }
}

// ---- zk_fit for a page ---------------------------------------------------------------------------------------------

/// every widget under `root` (zk_walk: depth first, last child first; it does not go inside a PixelButton)
pub fn walk(ui: &Ui, root: usize) -> Vec<usize> {
    let (mut out, mut stack) = (vec![], vec![root]);
    while let Some(x) = stack.pop() {
        out.push(x);
        if matches!(ui.ws[x].kind, Kind::Button { .. }) {
            continue;
        }
        stack.extend(ui.ws[x].kids.iter().copied());
    }
    out
}

/// zk_fit's font pass over a page: Labels and Entries keep their first size as _zk_f0 and take zk_size of it,
/// PixelButtons zk_rescale from the size they were made with (a label showing a picture has no size to scale)
pub fn zk_fonts(ui: &mut Ui, fonts: &dyn Fonts, root: usize, zk_f: f64) {
    for id in walk(ui, root) {
        let w = &mut ui.ws[id];
        if let Kind::Text { t, .. } = &mut w.kind {
            if w.f0 == 0 {
                w.f0 = t.font.size;
            }
            let n = zk_size(zk_f, w.f0);
            if t.font.size != n {
                t.font.size = n;
                t.relayout(fonts);
            }
            continue;
        }
        let font = match &mut w.kind {
            Kind::Label { font, image: None, .. } | Kind::Entry { font, .. } | Kind::Button { font, .. }
            | Kind::Check { font, .. } | Kind::Menu { font, .. } | Kind::List { font, .. } => font,
            _ => continue,
        };
        if font.family == "TkDefaultFont" {
            continue;
        }
        if w.f0 == 0 {
            w.f0 = font.size;
        }
        let n = zk_size(zk_f, w.f0);
        let changed = font.size != n;
        font.size = n;
        if let Kind::Button { sized, .. } = &mut w.kind {
            if changed {
                *sized = None;
            }
        }
    }
}

/// zk_wrap's memory on a page: its width last time, the labels it wrapped and the frames whose rows it reflowed
#[derive(Clone, Debug, Default)]
pub struct WrapState {
    pub lastw: i32,
    /// (label, wraplength it was made with, wrapped by zk_wrap)
    pub wl0: Vec<(usize, i32, bool)>,
    /// (frame, [(kid, its pack before)]): _zk_orig
    pub orig: Vec<(usize, Vec<(usize, Pack)>)>,
    /// _zk_reflowed
    pub reflowed: Vec<usize>,
    /// z21_note_reflows' _z21_need: how wide a reflowed frame must be to go back to one row
    pub need: Vec<(usize, i32)>,
}

fn label_text(ui: &Ui, id: usize) -> Option<(String, i32)> {
    match &ui.ws[id].kind {
        Kind::Label { text, wrap, image: None, .. } | Kind::Check { text, wrap, .. } => Some((text.clone(), *wrap)),
        _ => None,
    }
}

fn set_wrap(ui: &mut Ui, id: usize, wl: i32, left_too: bool) {
    if let Kind::Label { wrap, left, .. } | Kind::Check { wrap, left, .. } = &mut ui.ws[id].kind {
        *wrap = wl;
        if left_too {
            *left = true;
        }
    }
}

fn extra_pad(ui: &Ui, id: usize) -> i32 {
    2 * ui.ws[id].padx + if matches!(ui.ws[id].kind, Kind::Check { .. }) { 28 } else { 0 }
}

/// _z21_room(w, right): how wide a widget could be where it sits (its parent's width, not its own squeezed width)
fn z21_room(ui: &Ui, id: usize, right: i32) -> i32 {
    let Some(m) = ui.ws[id].parent else { return ui.ws[id].w };
    let x0 = ui.abs(id).0;
    let own = ui.ws[id].w;
    let mr = ui.abs(m).0 + ui.ws[m].w;
    if let Some(p) = &ui.ws[id].pack {
        if matches!(p.side, Side::Left | Side::Right) {
            let sibs = ui.packed(m);
            let after = sibs.iter().position(|&k| k == id).map(|i| sibs[i + 1..].to_vec()).unwrap_or_default();
            let used: i32 = after.iter().filter(|&&k| ui.ws[k].pack.as_ref().is_some_and(|q| q.side == p.side))
                .map(|&k| ui.ws[k].rw).sum();
            return own.max((mr - x0 - used - 6).min(right - x0));
        }
        return (mr - x0 - p.padx.1).min(right - x0);
    }
    if ui.ws[id].grid.is_some() {
        return own.max(right - x0 - 40);
    }
    own.min(right - x0)
}

/// put a reflowed frame's widgets back the way they were packed
fn restore(ui: &mut Ui, st: &mut WrapState, id: usize) -> bool {
    let Some(k) = st.orig.iter().position(|e| e.0 == id) else { return false };
    let (_, kids) = st.orig.remove(k);
    for (c, _) in &kids {
        ui.ws[*c].grid = None;
        ui.ws[*c].pack = None;
    }
    for (c, p) in kids {
        ui.pack(c, p);
    }
    st.reflowed.retain(|&r| r != id);
    true
}

/// zk_flow_reflow: a row of buttons / labels wraps like text, each line keeping its own spacing
fn flow_reflow(ui: &mut Ui, row: usize, kids: &[usize], avail: i32) {
    let info: Vec<(usize, (i32, i32))> = kids.iter().map(|&k| (k, ui.ws[k].pack.as_ref().unwrap().padx)).collect();
    for &k in kids {
        ui.ws[k].pack = None;
    }
    let bg = ui.ws[row].bg.clone();
    let (mut line, mut x) = (None, 0);
    for (k, px) in info {
        let wd = ui.ws[k].rw + px.0 + px.1;
        if line.is_none() || (x > 0 && x + wd > avail) {
            let l = ui.frame(row, &bg);
            // (zenko.py's "0 if this is the first line" test runs after the new line is made: always 3)
            ui.pack(l, Pack::default().side(Side::Top).anchor(Anchor::W).pady(3, 0));
            line = Some(l);
            x = 0;
        }
        ui.pack_into(k, line.unwrap(), Pack::default().side(Side::Left).padx(px.0, px.1));
        ui.lift(k);
        x += wd;
    }
}

/// zk_grid_reflow: a row of stretchy tiles that doesn't fit becomes an even grid (e.g. 5 tiles -> 3 + 2)
fn grid_reflow(ui: &mut Ui, row: usize, kids: &[usize], avail: i32) {
    let n = kids.len() as i32;
    let need = kids.iter().map(|&k| ui.ws[k].rw).max().unwrap_or(0) + 12;
    let mut per = 1.max(n.min(avail.div_euclid(need.max(1))));
    if per >= n {
        per = n - 1;
    }
    if avail >= need * ((n + 1) / 2) {
        per = per.max((n + 1) / 2);
    }
    for &k in kids {
        ui.ws[k].pack = None;
    }
    let per = per.max(1) as usize;
    let w = &mut ui.ws[row];
    if w.gw_cols.len() < per {
        w.gw_cols.resize(per, 0);
    }
    if w.gu_cols.len() < per {
        w.gu_cols.resize(per, "");
    }
    for c in 0..per {
        (w.gw_cols[c], w.gu_cols[c]) = (1, "zkr");
    }
    for (i, &k) in kids.iter().enumerate() {
        let (r, c) = (i / per, i % per);
        ui.grid(k, Grid { row: r, col: c, sticky: "nsew", padx: (if c == 0 { 0 } else { 10 }, 0),
                          pady: (if r == 0 { 0 } else { 10 }, 0), ..Default::default() });
    }
}

/// z21_unwrap: labels wrapped narrower than the room they have now unwrap, reflowed rows go back to one line when
/// there's room for it again
fn z21_unwrap(ui: &mut Ui, fonts: &dyn Fonts, pg: usize, st: &mut WrapState, pw: i32, ph: i32) {
    ui.layout_at(fonts, pw, ph);
    let right = ui.abs(pg).0 + ui.ws[pg].w - 18;
    let mut changed = false;
    for id in walk(ui, pg) {
        if let Some((_, cur)) = label_text(ui, id) {
            let Some(k) = st.wl0.iter().position(|e| e.0 == id && e.2) else { continue };
            let wl = z21_room(ui, id, right) - extra_pad(ui, id) - 4;
            if wl > cur + 30 {
                let wl0 = st.wl0[k].1;
                if wl0 != 0 && wl0 <= wl {
                    set_wrap(ui, id, wl0, false);
                    st.wl0[k].2 = false;
                } else {
                    set_wrap(ui, id, wl, false);
                }
                changed = true;
            }
        } else if matches!(ui.ws[id].kind, Kind::Frame) {
            let Some(&(_, need)) = st.need.iter().find(|e| e.0 == id) else { continue };
            if need == 0 || !st.orig.iter().any(|e| e.0 == id) {
                continue;
            }
            let room = (ui.ws[id].w.min(right - ui.abs(id).0)).max(z21_room(ui, id, right));
            if room >= need {
                restore(ui, st, id);
                // the extra line frames the flow reflow made
                for ch in ui.ws[id].kids.clone() {
                    if matches!(ui.ws[ch].kind, Kind::Frame) && ui.ws[ch].kids.is_empty() {
                        ui.destroy(ch);
                    }
                }
                st.need.retain(|e| e.0 != id);
                changed = true;
            }
        }
    }
    if changed {
        ui.layout_at(fonts, pw, ph);
    }
}

/// zk_wrap(pg) as Zenko21 runs it: z21_unwrap, then long labels wrap to the room they really have and rows that
/// don't fit are stacked (two columns) or reflowed (a row of three or more), then z21_note_reflows. Lays out (the
/// update_idletasks) at the page's window size (pw, ph) between passes.
pub fn zk_wrap(ui: &mut Ui, fonts: &dyn Fonts, pg: usize, st: &mut WrapState, pw: i32, ph: i32) {
    z21_unwrap(ui, fonts, pg, st, pw, ph);
    ui.layout_at(fonts, pw, ph);
    let right = ui.abs(pg).0 + ui.ws[pg].w - 18;
    let grew = ui.ws[pg].w > st.lastw + 30;
    st.lastw = ui.ws[pg].w;
    for pass in 0..2 {
        let mut changed = false;
        for id in walk(ui, pg) {
            if let Some((txt, cur)) = label_text(ui, id) {
                if txt.chars().count() < 6 {
                    continue;
                }
                let k = match st.wl0.iter().position(|e| e.0 == id) {
                    Some(k) => k,
                    None => {
                        st.wl0.push((id, cur, false));
                        st.wl0.len() - 1
                    }
                };
                if grew && st.wl0[k].2 && pass == 0 {
                    let wl0 = st.wl0[k].1;
                    set_wrap(ui, id, wl0, false);
                    st.wl0[k].2 = false;
                    changed = true;
                    continue;
                }
                let width = ui.ws[id].w;
                if width <= 1 || !ui.ws[id].mapped {
                    continue;
                }
                let room = width.min(right - ui.abs(id).0);
                if ui.ws[id].rw > room + 2 && room > 60 {
                    let wl = 80.max(room - extra_pad(ui, id) - 4);
                    if cur == 0 || cur > wl {
                        set_wrap(ui, id, wl, true);
                        st.wl0[k].2 = true;
                        changed = true;
                    }
                }
            } else if matches!(ui.ws[id].kind, Kind::Frame) {
                if grew && pass == 0 && restore(ui, st, id) {
                    changed = true;
                    continue;
                }
                if st.reflowed.contains(&id) || !ui.ws[id].mapped {
                    continue;
                }
                let width = ui.ws[id].w;
                if width <= 1 {
                    continue;
                }
                let room = width.min(right - ui.abs(id).0);
                if ui.ws[id].rw > room + 4 && room > 120 {
                    let kids: Vec<usize> = ui.ws[id].kids.iter().copied()
                        .filter(|&k| ui.ws[k].pack.is_some() && ui.ws[k].pack_in.is_none()).collect();
                    let left = |k: usize| ui.ws[k].pack.as_ref().unwrap().side == Side::Left;
                    let two = kids.len() == 2 && kids.iter().all(|&k| left(k) && matches!(ui.ws[k].kind, Kind::Frame)
                                                                     && ui.ws[k].rw > 200);
                    let squeezed = two && kids.iter().any(|&k| ui.ws[k].w < (ui.ws[k].rw - 8).min(340));
                    if two && (room < 860 || squeezed) {
                        st.reflowed.push(id);
                        st.orig.push((id, kids.iter().map(|&k| (k, ui.ws[k].pack.clone().unwrap())).collect()));
                        for (n, &k) in kids.iter().enumerate() {
                            ui.pack(k, Pack::default().side(Side::Top).fill(Fill::X).pady(if n == 0 { 0 } else { 4 }, 0));
                        }
                        changed = true;
                    } else if kids.len() >= 3 && kids.iter().all(|&k| left(k)) {
                        let stretch = kids.iter().all(|&k| ui.ws[k].pack.as_ref().unwrap().expand);
                        st.reflowed.push(id);
                        if !st.orig.iter().any(|e| e.0 == id) {
                            st.orig.push((id, kids.iter().map(|&k| (k, ui.ws[k].pack.clone().unwrap())).collect()));
                        }
                        if stretch {
                            grid_reflow(ui, id, &kids, room);
                        } else {
                            flow_reflow(ui, id, &kids, room);
                        }
                        changed = true;
                    }
                }
            }
        }
        if !changed {
            break;
        }
        ui.layout_at(fonts, pw, ph);
    }
    // z21_note_reflows: how wide each reflowed frame needs to be to go back
    for (id, kids) in &st.orig {
        if st.reflowed.contains(id) && !st.need.iter().any(|e| e.0 == *id && e.1 != 0) {
            let need: i32 = kids.iter().map(|(k, p)| ui.ws[*k].rw + p.padx.0 + p.padx.1).sum();
            let v = (need + 8).max(if kids.len() == 2 { 860 } else { 0 });
            st.need.retain(|e| e.0 != *id);
            st.need.push((*id, v));
        }
    }
}
