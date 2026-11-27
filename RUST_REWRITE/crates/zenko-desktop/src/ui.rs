//! A small Tk for zenko.py's windows: widgets in a tree, sized the way Windows Tk 8.6 sizes them (tkWinButton.c
//! labels, tkEntry.c entries, tkFrame.c frames, tkCanvas.c canvases, zenko.py's PixelButton), laid out with Tk's own
//! pack and place (tkPack.c ArrangePacking / XExpansion / YExpansion, tkPlace.c RecomputePlacement), then drawn
//! into a platform-free display list. Fonts are measured through `Fonts` (GDI on Windows, Tk's numbers in tests).

use crate::shapes::round_pts;
use crate::theme::Theme;
use zenko_art::palette::mixhex;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Font {
    pub family: String,
    pub size: i32,
    pub bold: bool,
    pub italic: bool,
}

impl Font {
    pub fn new(family: &str, size: i32, bold: bool) -> Font {
        Font { family: family.into(), size, bold, italic: false }
    }
}

pub trait Fonts {
    /// Tk_TextWidth
    fn width(&self, f: &Font, s: &str) -> i32;
    /// (ascent, descent) from Tk_GetFontMetrics; linespace = ascent + descent
    fn metrics(&self, f: &Font) -> (i32, i32);
    fn linespace(&self, f: &Font) -> i32 {
        let (a, d) = self.metrics(f);
        a + d
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Side {
    Top,
    Bottom,
    Left,
    Right,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Anchor {
    N,
    NE,
    E,
    SE,
    S,
    SW,
    W,
    NW,
    Center,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Fill {
    None,
    X,
    Y,
    Both,
}

/// pack options as zenko.py passes them (padx/pady as (before, after); ipadx/ipady as given, Tk keeps twice that)
#[derive(Clone, Debug)]
pub struct Pack {
    pub side: Side,
    pub fill: Fill,
    pub expand: bool,
    pub padx: (i32, i32),
    pub pady: (i32, i32),
    pub ipadx: i32,
    pub ipady: i32,
    pub anchor: Anchor,
}

impl Default for Pack {
    fn default() -> Pack {
        Pack { side: Side::Top, fill: Fill::None, expand: false, padx: (0, 0), pady: (0, 0), ipadx: 0, ipady: 0,
               anchor: Anchor::Center }
    }
}

impl Pack {
    pub fn side(mut self, s: Side) -> Pack {
        self.side = s;
        self
    }
    pub fn fill(mut self, f: Fill) -> Pack {
        self.fill = f;
        self
    }
    pub fn expand(mut self) -> Pack {
        self.expand = true;
        self
    }
    pub fn padx(mut self, a: i32, b: i32) -> Pack {
        self.padx = (a, b);
        self
    }
    pub fn pady(mut self, a: i32, b: i32) -> Pack {
        self.pady = (a, b);
        self
    }
    pub fn ipadx(mut self, v: i32) -> Pack {
        self.ipadx = v;
        self
    }
    pub fn ipady(mut self, v: i32) -> Pack {
        self.ipady = v;
        self
    }
    pub fn anchor(mut self, a: Anchor) -> Pack {
        self.anchor = a;
        self
    }
}

/// place options (anchor nw; bordermode inside unless `ignore`)
#[derive(Clone, Debug, Default)]
pub struct Place {
    /// bordermode="ignore": measured from the widget's outer edge, not inside its border
    pub ignore: bool,
    pub x: i32,
    pub y: i32,
    pub relx: f64,
    pub rely: f64,
    pub w: Option<i32>,
    pub h: Option<i32>,
    pub relw: Option<f64>,
    pub relh: Option<f64>,
}

/// One canvas item, in canvas coordinates (already rounded the way Tk rounds them)
#[derive(Clone, Debug, PartialEq)]
pub enum Item {
    Poly { pts: Vec<(i32, i32)>, fill: Option<String>, outline: Option<String> },
    Line { pts: Vec<(i32, i32)>, color: String },
    Oval { x1: i32, y1: i32, x2: i32, y2: i32, fill: String },
    /// create_text at (x, y), anchor center
    Text { x: i32, y: i32, text: String, font: Font, color: String },
}

/// What gets drawn, in window coordinates
#[derive(Clone, Debug, PartialEq)]
pub enum Draw {
    Rect { x: i32, y: i32, w: i32, h: i32, color: String },
    Poly { pts: Vec<(i32, i32)>, fill: Option<String>, outline: Option<String> },
    Line { pts: Vec<(i32, i32)>, color: String },
    Oval { x1: i32, y1: i32, x2: i32, y2: i32, fill: String },
    /// one line of text, top-left at (x, y)
    Text { x: i32, y: i32, text: String, font: Font, color: String },
    Clip(Option<(i32, i32, i32, i32)>),
    /// a Tk canvas's own drawing calls (canvasdraw.rs), its origin at (x, y)
    Tk { x: i32, y: i32, ops: Vec<crate::canvasdraw::TkOp> },
}

/// zenko.py round_frame's background canvas: rbox(0, 0, W-1, H-1, r, fill, outline, shape) and the lit rim
#[derive(Clone, Debug)]
pub struct Card {
    pub fill: String,
    pub edge: String,
    pub r: i32,
    pub shape: &'static str,
    pub rim: String,
}

/// tk.Entry's text, cursor and selection
#[derive(Clone, Debug, Default)]
pub struct EntryText {
    pub text: Vec<u16>,
    pub pos: usize,
    pub anchor: usize,
    pub sel: Option<(usize, usize)>,
    pub left: usize,
    pub blink_t0: f64,
}

#[derive(Clone, Debug)]
pub enum Kind {
    Frame,
    Label { text: String, font: Font, fg: String, anchor: Anchor, wrap: i32 },
    /// zenko.py PixelButton (a canvas): its own colour, font (bold), padding, hover / press state and command
    Button { text: String, font: Font, color: String, fg: String, padx: i32, pady: i32, enabled: bool,
             shape: &'static str, hover: bool, down: bool, cmd: String },
    Entry { font: Font, chars: i32, e: EntryText },
    Canvas { items: Vec<Item>, card: Option<Card>, tk: Option<Box<crate::tkcanvas::Canvas>> },
}

#[derive(Clone, Debug)]
pub struct W {
    pub kind: Kind,
    pub parent: Option<usize>,
    pub kids: Vec<usize>,
    pub bg: String,
    /// round_frame's _zk_fill: the colour children treat as their parent's background
    pub zk_fill: Option<String>,
    pub bd: i32,
    pub hl: i32,
    /// -padx / -pady of a frame or label (internal)
    pub padx: i32,
    pub pady: i32,
    /// -width / -height options (frames and canvases: pixels)
    pub opt_w: i32,
    pub opt_h: i32,
    pub pack: Option<Pack>,
    pub place: Option<Place>,
    pub propagate: bool,
    pub name: String,
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
    pub rw: i32,
    pub rh: i32,
    pub mapped: bool,
    /// lowered under its siblings (tk.Misc.lower)
    pub low: bool,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Cursor {
    Arrow,
    Hand,
    IBeam,
}

#[derive(Default)]
pub struct Ui {
    pub ws: Vec<W>,
    pub focus: Option<usize>,
    pub active: bool,
    pressed: Option<usize>,
    dragging: bool,
}

/// Tk_ComputeTextLayout for one paragraph set: (lines, width of the widest line without trailing spaces)
pub fn text_layout(fonts: &dyn Fonts, font: &Font, text: &str, wrap: i32) -> (Vec<String>, i32) {
    let wd = |s: &[char]| fonts.width(font, &s.iter().collect::<String>());
    let trim = |s: &[char]| {
        let mut n = s.len();
        while n > 0 && s[n - 1] == ' ' {
            n -= 1;
        }
        n
    };
    let mut lines = vec![];
    let mut width = 0;
    for para in text.split('\n') {
        let u: Vec<char> = para.chars().collect();
        if wrap <= 0 || wd(&u) <= wrap {
            width = width.max(wd(&u[..trim(&u)]));
            lines.push(para.to_string());
            continue;
        }
        let mut start = 0usize;
        while start < u.len() {
            let mut end = start;
            let mut best = start;
            while end < u.len() {
                let mut e = end;
                while e < u.len() && u[e] == ' ' {
                    e += 1;
                }
                while e < u.len() && u[e] != ' ' {
                    e += 1;
                }
                if wd(&u[start..e]) <= wrap {
                    best = e;
                    end = e;
                } else {
                    break;
                }
            }
            if best == start {
                let mut e = start + 1;
                while e < u.len() && wd(&u[start..e + 1]) <= wrap {
                    e += 1;
                }
                best = e;
            }
            let line = &u[start..best];
            width = width.max(wd(&line[..trim(line)]));
            lines.push(line.iter().collect());
            start = best;
            while start < u.len() && u[start] == ' ' {
                start += 1;
            }
        }
    }
    (lines, width)
}

impl Ui {
    /// a toplevel (id 0) with background `bg`
    pub fn new(bg: &str) -> Ui {
        let mut u = Ui { active: true, ..Default::default() };
        u.add(None, Kind::Frame, bg);
        u
    }

    fn add(&mut self, parent: Option<usize>, kind: Kind, bg: &str) -> usize {
        let id = self.ws.len();
        self.ws.push(W { kind, parent, kids: vec![], bg: bg.into(), zk_fill: None, bd: 0, hl: 0, padx: 0, pady: 0,
                         opt_w: 0, opt_h: 0, pack: None, place: None, propagate: true, name: String::new(), x: 0, y: 0,
                         w: 1, h: 1, rw: 1, rh: 1, mapped: parent.is_none(), low: false });
        if let Some(p) = parent {
            self.ws[p].kids.push(id);
        }
        id
    }

    /// the colour a PixelButton / round_frame sees as its parent's: _zk_fill, else bg
    pub fn fill_of(&self, id: usize) -> String {
        self.ws[id].zk_fill.clone().unwrap_or_else(|| self.ws[id].bg.clone())
    }

    pub fn frame(&mut self, parent: usize, bg: &str) -> usize {
        self.add(Some(parent), Kind::Frame, bg)
    }

    /// tk.Label(parent, text, bg, fg, font, anchor, wraplength): borderwidth 2, padx 1, pady 1
    pub fn label(&mut self, parent: usize, text: &str, bg: &str, fg: &str, font: Font) -> usize {
        let id = self.add(Some(parent), Kind::Label { text: text.into(), font, fg: fg.into(), anchor: Anchor::Center,
                                                      wrap: 0 }, bg);
        let w = &mut self.ws[id];
        (w.bd, w.padx, w.pady) = (2, 1, 1);
        id
    }

    pub fn set_anchor(&mut self, id: usize, a: Anchor) {
        if let Kind::Label { anchor, .. } = &mut self.ws[id].kind {
            *anchor = a;
        }
    }

    /// zenko.py PixelButton(parent, text, cmd, bg, fg, size, padx, pady, enabled, shape="pill")
    #[allow(clippy::too_many_arguments)]
    pub fn button(&mut self, parent: usize, text: &str, cmd: &str, color: &str, fg: &str, family: &str, size: i32,
                  padx: i32, pady: i32, enabled: bool) -> usize {
        let pbg = self.fill_of(parent);
        self.add(Some(parent), Kind::Button { text: text.into(), font: Font::new(family, size, true), color: color.into(),
                                              fg: fg.into(), padx, pady, enabled, shape: "pill", hover: false,
                                              down: false, cmd: cmd.into() }, &pbg)
    }

    /// zenko.py tt_entry: relief flat, borderwidth 1, highlightthickness 1
    pub fn entry(&mut self, parent: usize, chars: i32, font: Font, text: &str, bg: &str) -> usize {
        let e = EntryText { text: text.encode_utf16().collect(), ..Default::default() };
        let id = self.add(Some(parent), Kind::Entry { font, chars, e }, bg);
        let w = &mut self.ws[id];
        (w.bd, w.hl) = (1, 1);
        id
    }

    pub fn canvas(&mut self, parent: usize, bg: &str, w: i32, h: i32) -> usize {
        let id = self.add(Some(parent), Kind::Canvas { items: vec![], card: None, tk: None }, bg);
        (self.ws[id].opt_w, self.ws[id].opt_h) = (w, h);
        id
    }

    /// zenko.py round_frame(parent, bg, edge, r, shape, pad): a frame (parent's colour, padx = pady = pad) with a
    /// canvas placed over all of its inside, lowered under the children, that draws the card
    #[allow(clippy::too_many_arguments)]
    pub fn round_frame(&mut self, parent: usize, bg: &str, edge: &str, r: i32, shape: &'static str, pad: i32,
                       ink: &str) -> usize {
        let pbg = self.fill_of(parent);
        let f = self.frame(parent, &pbg);
        (self.ws[f].padx, self.ws[f].pady) = (pad, pad);
        self.ws[f].zk_fill = Some(bg.into());
        let cv = self.canvas(f, &pbg, 0, 0);
        if let Kind::Canvas { card, .. } = &mut self.ws[cv].kind {
            *card = Some(Card { fill: bg.into(), edge: edge.into(), r, shape, rim: mixhex(bg, ink, 0.10) });
        }
        self.ws[cv].place = Some(Place { relw: Some(1.0), relh: Some(1.0), ..Default::default() });
        f
    }

    pub fn pack(&mut self, id: usize, p: Pack) -> usize {
        self.ws[id].pack = Some(p);
        id
    }

    pub fn named(&mut self, id: usize, name: &str) -> usize {
        self.ws[id].name = name.into();
        id
    }

    pub fn find(&self, name: &str) -> Option<usize> {
        self.ws.iter().position(|w| w.name == name)
    }

    /// the internal border (Tk_InternalBorderLeft etc.; all four sides are equal here)
    fn ib(&self, id: usize) -> (i32, i32) {
        let w = &self.ws[id];
        match w.kind {
            Kind::Frame => (w.bd + w.hl + w.padx, w.bd + w.hl + w.pady),
            Kind::Label { .. } => (w.bd + w.hl, w.bd + w.hl),
            Kind::Entry { .. } => (w.bd + w.hl + 1, w.bd + w.hl + 1),
            _ => (w.bd + w.hl, w.bd + w.hl),
        }
    }

    /// a widget's own geometry request, before any geometry manager propagation
    fn own_req(&self, fonts: &dyn Fonts, id: usize) -> (i32, i32) {
        let w = &self.ws[id];
        let (r, rr) = match &w.kind {
            Kind::Frame => {
                if w.opt_w > 0 || w.opt_h > 0 { (w.opt_w, w.opt_h) } else { (1, 1) }
            }
            Kind::Label { text, font, wrap, .. } => {
                let (lines, tw) = text_layout(fonts, font, text, *wrap);
                let th = if text.is_empty() { fonts.linespace(font) } else { lines.len() as i32 * fonts.linespace(font) };
                let inset = w.bd + w.hl;
                (tw + 2 * w.padx + 2 * inset, th + 2 * w.pady + 2 * inset)
            }
            Kind::Button { text, font, padx, pady, .. } => {
                (fonts.width(font, text) + 2 * padx + 10, fonts.linespace(font) + 2 * pady + 6)
            }
            Kind::Entry { font, chars, e } => {
                let inset = w.bd + w.hl + 1;
                let avg = fonts.width(font, "0");
                let ls = fonts.linespace(font);
                let width = if *chars > 0 {
                    chars * avg
                } else {
                    let t = String::from_utf16_lossy(&e.text);
                    if t.is_empty() { avg } else { fonts.width(font, &t) }
                };
                (width + 2 * inset, ls + 2 * inset)
            }
            Kind::Canvas { .. } => (w.opt_w + 2 * (w.bd + w.hl), w.opt_h + 2 * (w.bd + w.hl)),
        };
        (r, rr)
    }

    /// requested sizes, bottom up (pack propagates; place doesn't), then the layout, top down
    pub fn layout(&mut self, fonts: &dyn Fonts) {
        self.req(fonts, 0);
        let (rw, rh) = (self.ws[0].rw, self.ws[0].rh);
        (self.ws[0].w, self.ws[0].h) = (rw, rh);
        self.arrange(0);
    }

    /// lay out with the toplevel at a given size
    pub fn layout_at(&mut self, fonts: &dyn Fonts, w: i32, h: i32) {
        self.req(fonts, 0);
        (self.ws[0].w, self.ws[0].h) = (w, h);
        self.arrange(0);
    }

    fn packed(&self, id: usize) -> Vec<usize> {
        self.ws[id].kids.iter().copied().filter(|&k| self.ws[k].pack.is_some()).collect()
    }

    fn req(&mut self, fonts: &dyn Fonts, id: usize) {
        for k in self.ws[id].kids.clone() {
            self.req(fonts, k);
        }
        let (mut rw, mut rh) = self.own_req(fonts, id);
        let content = self.packed(id);
        if !content.is_empty() && self.ws[id].propagate {
            let (bx, by) = self.ib(id);
            let (mut width, mut height) = (2 * bx, 2 * by);
            let (mut max_w, mut max_h) = (width, height);
            for &c in &content {
                let p = self.ws[c].pack.clone().unwrap();
                let (cw, ch) = (self.ws[c].rw, self.ws[c].rh);
                let (padx, pady) = (p.padx.0 + p.padx.1, p.pady.0 + p.pady.1);
                if matches!(p.side, Side::Top | Side::Bottom) {
                    max_w = max_w.max(cw + padx + 2 * p.ipadx + width);
                    height += ch + pady + 2 * p.ipady;
                } else {
                    max_h = max_h.max(ch + pady + 2 * p.ipady + height);
                    width += cw + padx + 2 * p.ipadx;
                }
            }
            rw = max_w.max(width);
            rh = max_h.max(height);
        }
        let w = &mut self.ws[id];
        (w.rw, w.rh) = (rw.max(1), rh.max(1));
    }

    fn arrange(&mut self, id: usize) {
        let (bx, by) = self.ib(id);
        let (cw0, ch0) = (self.ws[id].w, self.ws[id].h);
        let content = self.packed(id);
        let (mut cav_x, mut cav_y) = (bx, by);
        let (mut cav_w, mut cav_h) = (cw0 - 2 * bx, ch0 - 2 * by);
        for (i, &c) in content.iter().enumerate() {
            let p = self.ws[c].pack.clone().unwrap();
            let (rw, rh) = (self.ws[c].rw, self.ws[c].rh);
            let (pad_x, pad_y) = (p.padx.0 + p.padx.1, p.pady.0 + p.pady.1);
            let (ipx, ipy) = (2 * p.ipadx, 2 * p.ipady);
            let (frame_x, frame_y, frame_w, frame_h);
            if matches!(p.side, Side::Top | Side::Bottom) {
                frame_w = cav_w;
                let mut fh = rh + pad_y + ipy;
                if p.expand {
                    fh += self.y_expansion(&content[i..], cav_h);
                }
                cav_h -= fh;
                if cav_h < 0 {
                    fh += cav_h;
                    cav_h = 0;
                }
                frame_h = fh;
                frame_x = cav_x;
                if p.side == Side::Top {
                    frame_y = cav_y;
                    cav_y += fh;
                } else {
                    frame_y = cav_y + cav_h;
                }
            } else {
                frame_h = cav_h;
                let mut fw = rw + pad_x + ipx;
                if p.expand {
                    fw += self.x_expansion(&content[i..], cav_w);
                }
                cav_w -= fw;
                if cav_w < 0 {
                    fw += cav_w;
                    cav_w = 0;
                }
                frame_w = fw;
                frame_y = cav_y;
                if p.side == Side::Left {
                    frame_x = cav_x;
                    cav_x += fw;
                } else {
                    frame_x = cav_x + cav_w;
                }
            }
            let (bl, br, bt, bb) = (p.padx.0, pad_x - p.padx.0, p.pady.0, pad_y - p.pady.0);
            let mut width = rw + ipx;
            if matches!(p.fill, Fill::X | Fill::Both) || width > frame_w - pad_x {
                width = frame_w - pad_x;
            }
            let mut height = rh + ipy;
            if matches!(p.fill, Fill::Y | Fill::Both) || height > frame_h - pad_y {
                height = frame_h - pad_y;
            }
            let cx = |a: Anchor| match a {
                Anchor::N | Anchor::S | Anchor::Center => frame_x + (bl + frame_w - width - br) / 2,
                Anchor::NE | Anchor::E | Anchor::SE => frame_x + frame_w - width - br,
                _ => frame_x + bl,
            };
            let cy = |a: Anchor| match a {
                Anchor::E | Anchor::W | Anchor::Center => frame_y + (bt + frame_h - height - bb) / 2,
                Anchor::SE | Anchor::S | Anchor::SW => frame_y + frame_h - height - bb,
                _ => frame_y + bt,
            };
            let (x, y) = (cx(p.anchor), cy(p.anchor));
            let w = &mut self.ws[c];
            if width <= 0 || height <= 0 {
                w.mapped = false;
            } else {
                (w.x, w.y, w.w, w.h, w.mapped) = (x, y, width, height, true);
            }
        }
        // place: bordermode inside
        for k in self.ws[id].kids.clone() {
            let Some(pl) = self.ws[k].place.clone() else { continue };
            let (mx, my, mw, mh) = if pl.ignore {
                (0.0, 0.0, cw0 as f64, ch0 as f64)
            } else {
                (bx as f64, by as f64, (cw0 - 2 * bx) as f64, (ch0 - 2 * by) as f64)
            };
            let rnd = |v: f64| if v > 0.0 { (v + 0.5) as i32 } else { (v - 0.5) as i32 };
            let x1 = pl.x as f64 + mx + pl.relx * mw;
            let x = rnd(x1);
            let y1 = pl.y as f64 + my + pl.rely * mh;
            let y = rnd(y1);
            let width = if pl.w.is_some() || pl.relw.is_some() {
                pl.w.unwrap_or(0) + pl.relw.map(|r| rnd(x1 + r * mw) - x).unwrap_or(0)
            } else {
                self.ws[k].rw
            };
            let height = if pl.h.is_some() || pl.relh.is_some() {
                pl.h.unwrap_or(0) + pl.relh.map(|r| rnd(y1 + r * mh) - y).unwrap_or(0)
            } else {
                self.ws[k].rh
            };
            let w = &mut self.ws[k];
            if width <= 0 || height <= 0 {
                w.mapped = false;
            } else {
                (w.x, w.y, w.w, w.h, w.mapped) = (x, y, width, height, true);
            }
        }
        for k in self.ws[id].kids.clone() {
            if self.ws[k].mapped {
                self.arrange(k);
            }
        }
    }

    fn x_expansion(&self, rest: &[usize], mut cav_w: i32) -> i32 {
        let mut min_e = cav_w;
        let mut n = 0;
        for &c in rest {
            let p = self.ws[c].pack.as_ref().unwrap();
            let cw = self.ws[c].rw + p.padx.0 + p.padx.1 + 2 * p.ipadx;
            if matches!(p.side, Side::Top | Side::Bottom) {
                if n > 0 {
                    min_e = min_e.min((cav_w - cw) / n);
                }
            } else {
                cav_w -= cw;
                if p.expand {
                    n += 1;
                }
            }
        }
        if n > 0 {
            min_e = min_e.min(cav_w / n);
        }
        min_e.max(0)
    }

    fn y_expansion(&self, rest: &[usize], mut cav_h: i32) -> i32 {
        let mut min_e = cav_h;
        let mut n = 0;
        for &c in rest {
            let p = self.ws[c].pack.as_ref().unwrap();
            let ch = self.ws[c].rh + p.pady.0 + p.pady.1 + 2 * p.ipady;
            if matches!(p.side, Side::Left | Side::Right) {
                if n > 0 {
                    min_e = min_e.min((cav_h - ch) / n);
                }
            } else {
                cav_h -= ch;
                if p.expand {
                    n += 1;
                }
            }
        }
        if n > 0 {
            min_e = min_e.min(cav_h / n);
        }
        min_e.max(0)
    }

    /// a widget's top-left corner in window coordinates
    pub fn abs(&self, id: usize) -> (i32, i32) {
        let (mut x, mut y) = (0, 0);
        let mut c = Some(id);
        while let Some(i) = c {
            if self.ws[i].parent.is_some() {
                x += self.ws[i].x;
                y += self.ws[i].y;
            }
            c = self.ws[i].parent;
        }
        (x, y)
    }

    /// stacking order: a canvas made by round_frame is lowered under its siblings
    pub fn stack(&self, id: usize) -> Vec<usize> {
        let mut v: Vec<usize> = self.ws[id].kids.iter().copied()
            .filter(|&k| self.ws[k].low || matches!(&self.ws[k].kind, Kind::Canvas { card: Some(_), .. })).collect();
        v.extend(self.ws[id].kids.iter().copied()
            .filter(|&k| !self.ws[k].low && !matches!(&self.ws[k].kind, Kind::Canvas { card: Some(_), .. })));
        v
    }

    pub fn draw(&self, fonts: &dyn Fonts, t: &Theme, now: f64) -> Vec<Draw> {
        let mut out = vec![];
        self.draw_w(fonts, t, now, 0, (0, 0, self.ws[0].w, self.ws[0].h), &mut out);
        out.push(Draw::Clip(None));
        out
    }

    fn draw_w(&self, fonts: &dyn Fonts, t: &Theme, now: f64, id: usize, clip: (i32, i32, i32, i32), out: &mut Vec<Draw>) {
        let w = &self.ws[id];
        if !w.mapped {
            return;
        }
        let (ax, ay) = self.abs(id);
        let (x1, y1) = (ax.max(clip.0), ay.max(clip.1));
        let (x2, y2) = ((ax + w.w).min(clip.0 + clip.2), (ay + w.h).min(clip.1 + clip.3));
        if x2 <= x1 || y2 <= y1 {
            return;
        }
        let me = (x1, y1, x2 - x1, y2 - y1);
        out.push(Draw::Clip(Some(me)));
        out.push(Draw::Rect { x: ax, y: ay, w: w.w, h: w.h, color: w.bg.clone() });
        match &w.kind {
            Kind::Frame => {}
            Kind::Label { text, font, fg, anchor, wrap } => {
                let (lines, tw) = text_layout(fonts, font, text, *wrap);
                let ls = fonts.linespace(font);
                let th = lines.len() as i32 * ls;
                let ib = w.bd + w.hl;
                let x = match anchor {
                    Anchor::NW | Anchor::W | Anchor::SW => ib + w.padx,
                    Anchor::N | Anchor::Center | Anchor::S => (w.w - tw - 2 * ib) / 2 + ib,
                    _ => w.w - ib - w.padx - tw,
                };
                let y = match anchor {
                    Anchor::NW | Anchor::N | Anchor::NE => ib + w.pady,
                    Anchor::W | Anchor::Center | Anchor::E => (w.h - th - 2 * ib) / 2 + ib,
                    _ => w.h - ib - w.pady - th,
                };
                for (i, l) in lines.iter().enumerate() {
                    let lw = fonts.width(font, l.trim_end_matches(' '));
                    // justify center (the Label default)
                    let lx = x + (tw - lw) / 2;
                    out.push(Draw::Text { x: ax + lx, y: ay + y + i as i32 * ls, text: l.clone(), font: font.clone(),
                                          color: fg.clone() });
                }
            }
            Kind::Entry { font, e, .. } => {
                let focus = self.active && self.focus == Some(id);
                let ib = w.bd + w.hl + 1;
                let (x, y, wd, ht) = (ax, ay, w.w, w.h);
                out.push(Draw::Rect { x, y, w: wd, h: ht, color: t.field.clone() });
                let ring = if focus { t.gold.clone() } else { t.line.clone() };
                for (rx, ry, rw, rh) in [(x, y, wd, 1), (x, y + ht - 1, wd, 1), (x, y, 1, ht), (x + wd - 1, y, 1, ht)] {
                    out.push(Draw::Rect { x: rx, y: ry, w: rw, h: rh, color: ring.clone() });
                }
                let (asc, desc) = fonts.metrics(font);
                let ls = asc + desc;
                let ty = y + (ht + asc - desc) / 2 - asc;
                let xo = |i: usize| fonts.width(font, &String::from_utf16_lossy(&e.text[..i.min(e.text.len())]));
                let tx = x + ib - xo(e.left);
                let inner = (x1.max(x + 2), y1.max(y + 2), (x2.min(x + wd - 2) - x1.max(x + 2)).max(0),
                             (y2.min(y + ht - 2) - y1.max(y + 2)).max(0));
                out.push(Draw::Clip(Some(inner)));
                if let Some((a, b)) = e.sel {
                    let (sx, ex) = (tx + xo(a), tx + xo(b));
                    out.push(Draw::Rect { x: sx, y: ty, w: ex - sx, h: ls, color: t.acc.clone() });
                }
                out.push(Draw::Text { x: tx, y: ty, text: String::from_utf16_lossy(&e.text), font: font.clone(),
                                      color: t.ink.clone() });
                if let Some((a, b)) = e.sel {
                    out.push(Draw::Text { x: tx + xo(a), y: ty, text: String::from_utf16_lossy(&e.text[a..b]),
                                          font: font.clone(), color: t.onacc.clone() });
                }
                // the insertion cursor: 2 pixels wide, blinking 600 ms on / 300 ms off while focused
                if focus && ((now - e.blink_t0) * 1000.0) as i64 % 900 < 600 {
                    out.push(Draw::Rect { x: tx + xo(e.pos) - 1, y: ty, w: 2, h: ls, color: t.ink.clone() });
                }
                out.push(Draw::Clip(Some(me)));
            }
            Kind::Canvas { tk: Some(cv), .. } => {
                out.push(Draw::Tk { x: ax, y: ay, ops: crate::canvasdraw::render(cv, fonts) });
            }
            Kind::Button { .. } | Kind::Canvas { .. } => {
                for it in self.items_of(id, t) {
                    match it {
                        Item::Poly { pts, fill, outline } => out.push(Draw::Poly { pts: off(&pts, ax, ay), fill, outline }),
                        Item::Line { pts, color } => out.push(Draw::Line { pts: off(&pts, ax, ay), color }),
                        Item::Oval { x1, y1, x2, y2, fill } => out.push(Draw::Oval { x1: ax + x1, y1: ay + y1, x2: ax + x2,
                                                                                     y2: ay + y2, fill }),
                        Item::Text { x, y, text, font, color } => self.canvas_text(fonts, ax + x, ay + y, &text, &font, &color, out),
                    }
                }
            }
        }
        for k in self.stack(id) {
            self.draw_w(fonts, t, now, k, me, out);
        }
        out.push(Draw::Clip(Some(clip)));
    }

    /// what is on a canvas, in its own coordinates, as Tk holds it: the round_frame card, a PixelButton's _draw, or
    /// the items put there
    pub fn items_of(&self, id: usize, t: &Theme) -> Vec<Item> {
        let w = &self.ws[id];
        let mut v = vec![];
        match &w.kind {
            Kind::Button { text, font, color, fg, enabled, shape, hover, down, .. } => {
                let (wd, ht) = (w.w.max(4), w.h.max(4));
                let bg = color.as_str();
                let (fill, edge, fgc) = if !enabled {
                    (mixhex(bg, &t.paper, 0.35), t.line.clone(), mixhex(bg, &t.ink, 0.35))
                } else {
                    let fill = if *down {
                        mixhex(bg, "#000000", 0.22)
                    } else if *hover {
                        mixhex(bg, "#ffffff", 0.12)
                    } else {
                        bg.to_string()
                    };
                    let edge = if *hover {
                        t.gold.clone()
                    } else if bg == t.acc {
                        mixhex(bg, "#ffffff", 0.35)
                    } else {
                        mixhex(bg, "#ffffff", 0.2)
                    };
                    (fill, edge, fg.clone())
                };
                let dy = *down as i32;
                let r = (ht / 2).min(14);
                v.push(Item::Poly { pts: round_pts(1, 1 + dy, wd - 2, ht - 2 + dy, r, 2, shape), fill: Some(fill.clone()),
                                    outline: Some(edge) });
                if *enabled && !*down {
                    v.push(Item::Line { pts: vec![(1 + r, 3), (wd - 2 - r, 3)], color: mixhex(&fill, "#ffffff", 0.22) });
                }
                v.push(Item::Text { x: wd / 2, y: ht / 2 + dy, text: text.clone(), font: font.clone(), color: fgc });
            }
            Kind::Canvas { items, card, .. } => {
                if let Some(c) = card {
                    let (wd, ht) = (w.w, w.h);
                    if wd >= 8 && ht >= 8 {
                        v.push(Item::Poly { pts: round_pts(0, 0, wd - 1, ht - 1, c.r, 2, c.shape), fill: Some(c.fill.clone()),
                                            outline: Some(c.edge.clone()) });
                        // SHAPES[shape](r)[0][1]: the top-left corner's radius
                        let k = match c.shape {
                            "ema" => c.r + 4,
                            "leaf" => c.r * 2,
                            "pill" => 999,
                            _ => c.r,
                        };
                        let xa = (k + 2).min(wd / 2);
                        let xb = (wd - 1 - c.r.min(wd / 2) - 2).max(wd / 2);
                        v.push(Item::Line { pts: vec![(xa, 1), (xb, 1)], color: c.rim.clone() });
                    }
                }
                v.extend(items.iter().cloned());
            }
            _ => {}
        }
        v
    }

    /// create_text(x, y, anchor center): every line centred (justify left, but one line is the common case)
    #[allow(clippy::too_many_arguments)]
    fn canvas_text(&self, fonts: &dyn Fonts, x: i32, y: i32, text: &str, font: &Font, color: &str, out: &mut Vec<Draw>) {
        let (lines, tw) = text_layout(fonts, font, text, 0);
        let ls = fonts.linespace(font);
        let (lx, ty) = (x - tw / 2, y - lines.len() as i32 * ls / 2);
        for (i, l) in lines.iter().enumerate() {
            out.push(Draw::Text { x: lx, y: ty + i as i32 * ls, text: l.clone(), font: font.clone(), color: color.into() });
        }
    }

    /// the widget under a window point (topmost first)
    pub fn hit(&self, x: i32, y: i32) -> Option<usize> {
        let mut found = None;
        self.hit_w(0, x, y, &mut found);
        found
    }

    fn hit_w(&self, id: usize, x: i32, y: i32, found: &mut Option<usize>) {
        let w = &self.ws[id];
        if !w.mapped {
            return;
        }
        let (ax, ay) = self.abs(id);
        if x < ax || y < ay || x >= ax + w.w || y >= ay + w.h {
            return;
        }
        *found = Some(id);
        for k in self.stack(id) {
            self.hit_w(k, x, y, found);
        }
    }

    pub fn cursor_at(&self, x: i32, y: i32) -> Cursor {
        match self.hit(x, y).map(|i| &self.ws[i].kind) {
            Some(Kind::Button { enabled: true, .. }) => Cursor::Hand,
            Some(Kind::Entry { .. }) => Cursor::IBeam,
            _ => Cursor::Arrow,
        }
    }

    /// <Motion>: buttons glow under the pointer (<Enter>/<Leave>); a drag in an entry selects
    pub fn motion(&mut self, fonts: &dyn Fonts, x: i32, y: i32, now: f64) {
        let over = self.hit(x, y);
        for (i, w) in self.ws.iter_mut().enumerate() {
            if let Kind::Button { hover, down, enabled: true, .. } = &mut w.kind {
                let h = over == Some(i);
                if *hover && !h {
                    *down = false; // <Leave> lets go of the press; re-entering doesn't bring it back
                }
                *hover = h;
            }
        }
        if self.dragging {
            if let Some(f) = self.focus {
                let i = self.index_at(fonts, f, x);
                if let Kind::Entry { e, .. } = &mut self.ws[f].kind {
                    e.pos = i;
                    let (a, b) = (e.anchor.min(i), e.anchor.max(i));
                    e.sel = if a == b { None } else { Some((a, b)) };
                    e.blink_t0 = now;
                }
            }
        }
    }

    pub fn leave(&mut self) {
        for w in &mut self.ws {
            if let Kind::Button { hover, down, .. } = &mut w.kind {
                *hover = false;
                *down = false;
            }
        }
    }

    fn index_at(&self, fonts: &dyn Fonts, id: usize, x: i32) -> usize {
        let (ax, _) = self.abs(id);
        let w = &self.ws[id];
        let Kind::Entry { font, e, .. } = &w.kind else { return 0 };
        let xo = |i: usize| fonts.width(font, &String::from_utf16_lossy(&e.text[..i]));
        let base = ax + w.bd + w.hl + 1 - xo(e.left);
        let mut best = 0;
        for i in 0..=e.text.len() {
            if i > 0 && x < (base + xo(i - 1) + base + xo(i)) / 2 {
                break;
            }
            best = i;
        }
        best
    }

    /// <Button-1>
    pub fn press(&mut self, fonts: &dyn Fonts, x: i32, y: i32, now: f64, double: bool) {
        let Some(i) = self.hit(x, y) else { return };
        match &self.ws[i].kind {
            Kind::Button { enabled: true, .. } => {
                if let Kind::Button { down, .. } = &mut self.ws[i].kind {
                    *down = true;
                }
                self.pressed = Some(i);
            }
            Kind::Entry { .. } => {
                let k = self.index_at(fonts, i, x);
                self.focus = Some(i);
                if let Kind::Entry { e, .. } = &mut self.ws[i].kind {
                    if double {
                        let (mut a, mut b) = (k, k);
                        let sp = |c: u16| c == ' ' as u16;
                        while a > 0 && !sp(e.text[a - 1]) {
                            a -= 1;
                        }
                        while b < e.text.len() && !sp(e.text[b]) {
                            b += 1;
                        }
                        e.sel = if a < b { Some((a, b)) } else { None };
                        (e.anchor, e.pos) = (a, b);
                    } else {
                        (e.pos, e.anchor, e.sel) = (k, k, None);
                        self.dragging = true;
                    }
                    e.blink_t0 = now;
                }
            }
            _ => {}
        }
    }

    /// <ButtonRelease-1>: the command of a button let go of inside itself (PixelButton._release)
    pub fn release(&mut self, x: i32, y: i32) -> Option<String> {
        self.dragging = false;
        let i = self.pressed.take()?;
        let (ax, ay) = self.abs(i);
        let (ww, wh) = (self.ws[i].w, self.ws[i].h);
        if let Kind::Button { down, cmd, .. } = &mut self.ws[i].kind {
            let was = *down;
            *down = false;
            if was && ax <= x && x < ax + ww && ay <= y && y < ay + wh {
                return Some(cmd.clone());
            }
        }
        None
    }

    pub fn text_of(&self, id: usize) -> String {
        match &self.ws[id].kind {
            Kind::Entry { e, .. } => String::from_utf16_lossy(&e.text),
            Kind::Label { text, .. } | Kind::Button { text, .. } => text.clone(),
            _ => String::new(),
        }
    }

    fn entry_mut(&mut self) -> Option<(&mut EntryText, Font, i32)> {
        let f = self.focus?;
        let inner = self.ws[f].w - 2 * (self.ws[f].bd + self.ws[f].hl + 1);
        match &mut self.ws[f].kind {
            Kind::Entry { e, font, .. } => Some((e, font.clone(), inner)),
            _ => None,
        }
    }

    /// typed characters into the focused entry
    pub fn insert(&mut self, fonts: &dyn Fonts, s: &[u16], now: f64) {
        if let Some((e, font, inner)) = self.entry_mut() {
            del_sel(e);
            for (k, c) in s.iter().enumerate() {
                e.text.insert(e.pos + k, *c);
            }
            e.pos += s.len();
            see(e, fonts, &font, inner);
            e.blink_t0 = now;
        }
    }

    pub fn select_all(&mut self, fonts: &dyn Fonts) {
        if let Some((e, font, inner)) = self.entry_mut() {
            e.sel = if e.text.is_empty() { None } else { Some((0, e.text.len())) };
            e.anchor = 0;
            e.pos = e.text.len();
            see(e, fonts, &font, inner);
        }
    }

    pub fn selected(&self) -> Option<Vec<u16>> {
        let f = self.focus?;
        match &self.ws[f].kind {
            Kind::Entry { e, .. } => e.sel.map(|(a, b)| e.text[a..b].to_vec()),
            _ => None,
        }
    }

    /// Tk's entry bindings for the editing keys (virtual-key codes); true when handled
    pub fn key(&mut self, fonts: &dyn Fonts, vk: u16, shift: bool, ctrl: bool, now: f64) -> bool {
        let Some((e, font, inner)) = self.entry_mut() else { return false };
        let n = e.text.len();
        let word_left = |e: &EntryText| {
            let mut i = e.pos;
            while i > 0 && e.text[i - 1] == ' ' as u16 {
                i -= 1;
            }
            while i > 0 && e.text[i - 1] != ' ' as u16 {
                i -= 1;
            }
            i
        };
        let word_right = |e: &EntryText| {
            let mut i = e.pos;
            while i < e.text.len() && e.text[i] != ' ' as u16 {
                i += 1;
            }
            while i < e.text.len() && e.text[i] == ' ' as u16 {
                i += 1;
            }
            i
        };
        let mv = |e: &mut EntryText, to: usize| {
            if shift {
                if e.sel.is_none() {
                    e.anchor = e.pos;
                }
                e.pos = to;
                let (a, b) = (e.anchor.min(to), e.anchor.max(to));
                e.sel = if a == b { None } else { Some((a, b)) };
            } else {
                e.sel = None;
                e.pos = to;
            }
        };
        match vk {
            0x25 => {
                let to = match e.sel {
                    Some((a, _)) if !shift => a,
                    _ if ctrl => word_left(e),
                    _ => e.pos.saturating_sub(1),
                };
                mv(e, to);
            }
            0x27 => {
                let to = match e.sel {
                    Some((_, b)) if !shift => b,
                    _ if ctrl => word_right(e),
                    _ => (e.pos + 1).min(n),
                };
                mv(e, to);
            }
            0x24 => mv(e, 0),
            0x23 => mv(e, n),
            0x08 => {
                if !del_sel(e) && e.pos > 0 {
                    let a = if ctrl { word_left(e) } else { e.pos - 1 };
                    e.text.drain(a..e.pos);
                    e.pos = a;
                }
            }
            0x2E => {
                if !del_sel(e) && e.pos < n {
                    e.text.remove(e.pos);
                }
            }
            _ => return false,
        }
        see(e, fonts, &font, inner);
        e.blink_t0 = now;
        true
    }
}

fn off(pts: &[(i32, i32)], dx: i32, dy: i32) -> Vec<(i32, i32)> {
    pts.iter().map(|&(x, y)| (x + dx, y + dy)).collect()
}

fn del_sel(e: &mut EntryText) -> bool {
    if let Some((a, b)) = e.sel.take() {
        e.text.drain(a..b);
        e.pos = a;
        true
    } else {
        false
    }
}

/// keep the cursor in view (tkEntry.c EntryVisibleRange / see)
fn see(e: &mut EntryText, fonts: &dyn Fonts, font: &Font, inner: i32) {
    let xo = |i: usize| fonts.width(font, &String::from_utf16_lossy(&e.text[..i]));
    if e.pos < e.left {
        e.left = e.pos;
    }
    while e.left < e.pos && xo(e.pos) - xo(e.left) > inner {
        e.left += 1;
    }
}
