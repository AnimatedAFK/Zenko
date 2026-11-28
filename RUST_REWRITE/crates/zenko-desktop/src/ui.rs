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
    /// TkFontGetPixels: the font's size in pixels (points * dpi / 72)
    fn pixels(&self, f: &Font) -> f64 {
        f.size as f64 * 96.0 / 72.0
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
    /// anchor="ne": (x, y) is the widget's top right corner
    pub ne: bool,
    /// anchor="n": (x, y) is the widget's top middle (tkPlace.c: x -= width / 2)
    pub n: bool,
    /// a canvas window item's anchor="center" (tkCanvWind.c: x -= width / 2, y -= height / 2)
    pub center: bool,
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
    /// round_frame's notch: half-circle bites in the middle of the left and right sides (a ticket)
    pub notch: i32,
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
    /// a placeholder shown in GREY while the box is empty and not focused (zenko.py's ph_show / ph_hide)
    pub ph: Option<String>,
    /// -show: every character drawn as this one (a password box)
    pub show: Option<char>,
    /// a plain tk.Entry (relief flat, bd 0, no ring) on the widget's bg: its (fg, insertbackground)
    pub flat: Option<(String, String)>,
}

#[derive(Clone, Debug)]
pub enum Kind {
    Frame,
    Label { text: String, font: Font, fg: String, anchor: Anchor, wrap: i32, image: Option<std::rc::Rc<crate::tkcanvas::Photo>>,
            left: bool, strike: bool, under: bool },
    /// zenko.py PixelButton (a canvas): its own colour, font (bold), padding, hover / press state and command
    Button { text: String, font: Font, color: String, fg: String, padx: i32, pady: i32, enabled: bool,
             shape: &'static str, hover: bool, down: bool, cmd: String },
    Entry { font: Font, chars: i32, e: EntryText },
    Canvas { items: Vec<Item>, card: Option<Card>, tk: Option<Box<crate::tkcanvas::Canvas>> },
    /// a tk.Checkbutton / tk.Radiobutton with its indicator: `select` is selectcolor, `cmd` what a click does
    Check { text: String, font: Font, fg: String, on: bool, anchor: Anchor, wrap: i32, left: bool, select: String,
            cmd: String, radio: bool, noind: bool },
    /// a tk.Text (tktext.rs); `ring` is its highlightbackground
    Text { t: Box<crate::tktext::Text>, ring: String },
    /// a tk.OptionMenu's Menubutton (borderwidth 2, indicatoron, -width in characters in opt_w): `ind` is the
    /// indicator's (width, height) from the screen's millimetres, `cmd` what a click asks for (the menu)
    Menu { text: String, font: Font, fg: String, ind: (i32, i32), cmd: String },
    /// a horizontal tk.Scale (showvalue off): from / to / value, -length, -width, -sliderlength, the trough's colour
    /// (the slider is the widget's bg), `cmd` what a change asks for
    /// tk.Scale(orient horizontal): with a -label and -showvalue drawn above the trough in `font` / `fg`
    Scale { from: f64, to: f64, val: f64, length: i32, width: i32, slen: i32, trough: String, cmd: String, label: String,
            show: bool, font: Font, fg: String },
    /// a tk.Listbox (relief flat, selectborderwidth 0, activestyle none): its rows with their own -fg (itemconfig),
    /// the selection, the top row (yview), -width in characters and -height in rows; a click selects and asks for
    /// `cmd:<row>` (a double click `cmd_open:<row>`)
    List { items: Vec<(String, Option<String>)>, font: Font, fg: String, selbg: String, selfg: String, sel: Option<usize>,
           top: usize, chars: i32, lines: i32, cmd: String },
}

#[derive(Clone, Debug)]
pub struct W {
    /// cursor="hand2" on a widget that isn't a button
    pub hand: bool,
    pub kind: Kind,
    pub parent: Option<usize>,
    pub kids: Vec<usize>,
    pub bg: String,
    /// round_frame's _zk_fill: the colour children treat as their parent's background
    pub zk_fill: Option<String>,
    pub bd: i32,
    pub hl: i32,
    /// highlightbackground: the ring a frame / menubutton with highlightthickness draws ("" = none)
    pub hlc: String,
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
    /// when it was packed: the packer's order is the order of the pack calls
    pub pack_seq: u64,
    /// grid(row, column, sticky, padx, pady); None = not gridded (or grid_remove'd)
    pub grid: Option<Grid>,
    /// grid_columnconfigure / grid_rowconfigure weights of a container
    pub gw_cols: Vec<i32>,
    pub gw_rows: Vec<i32>,
    /// grid_columnconfigure(uniform=...): each column's group ("" = none)
    pub gu_cols: Vec<&'static str>,
    /// a frame shown in its parent canvas by create_window(0, 0, anchor nw) with the canvas's width
    pub cwin: bool,
    /// a canvas's yview: how far its window item is scrolled up
    pub yoff: i32,
    /// zk_font's _zk_f0: the font size the widget was made with (0 = not seen yet)
    pub f0: i32,
    /// pack(in_=master): packed into a sibling frame instead of its parent
    pub pack_in: Option<usize>,
}

impl W {
    /// a canvas widget's Tk canvas items (None for other widgets, or a canvas drawn the old way)
    pub fn tk_canvas(&self) -> Option<&crate::tkcanvas::Canvas> {
        match &self.kind {
            Kind::Canvas { tk: Some(c), .. } => Some(c),
            _ => None,
        }
    }
}

/// grid() options
#[derive(Clone, Debug, Default)]
pub struct Grid {
    pub row: usize,
    pub col: usize,
    pub sticky: &'static str,
    pub padx: (i32, i32),
    pub pady: (i32, i32),
    /// columnspan / rowspan (0 = 1)
    pub cspan: usize,
    pub rspan: usize,
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
    /// size check- and radiobuttons by X11 Tk's rules (the golden runs) instead of Windows Tk's
    pub x11: bool,
    pub focus: Option<usize>,
    pub active: bool,
    pressed: Option<usize>,
    dbl: bool,
    dragging: bool,
    pack_n: u64,
}

/// tkUnix3d.c TkpGetShadows for a colour display: the (light, dark) bevel colours of a 3-D border on `bg`
pub fn shadows(bg: &str) -> (String, String) {
    let c = zenko_art::palette::hex_rgb(bg);
    let (r, g, b) = (c[0] as i64 * 257, c[1] as i64 * 257, c[2] as i64 * 257);
    const M: i64 = 65535;
    let dark = if (r * r) as f64 * 0.5 + (g * g) as f64 + (b * b) as f64 * 0.28 < M as f64 * 0.05 * M as f64 {
        [(M + 3 * r) / 4, (M + 3 * g) / 4, (M + 3 * b) / 4]
    } else {
        [60 * r / 100, 60 * g / 100, 60 * b / 100]
    };
    let light = if g as f64 > M as f64 * 0.95 {
        [90 * r / 100, 90 * g / 100, 90 * b / 100]
    } else {
        let f = |v: i64| (14 * v / 10).min(M).max((M + v) / 2);
        [f(r), f(g), f(b)]
    };
    let hx = |a: [i64; 3]| format!("#{:02x}{:02x}{:02x}", a[0] / 257, a[1] / 257, a[2] / 257);
    (hx(light), hx(dark))
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
            // (a line that isn't wrapped keeps its trailing spaces: Tk only drops the one it breaks a line at)
            width = width.max(wd(&u));
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
        self.ws.push(W { hand: false, kind, parent, kids: vec![], bg: bg.into(), zk_fill: None, bd: 0, hl: 0, hlc: String::new(), padx: 0, pady: 0,
                         opt_w: 0, opt_h: 0, pack: None, place: None, propagate: true, name: String::new(), x: 0, y: 0,
                         w: 1, h: 1, rw: 1, rh: 1, mapped: parent.is_none(), low: false, pack_seq: 0, grid: None,
                         gw_cols: vec![], gw_rows: vec![], gu_cols: vec![], cwin: false, yoff: 0, f0: 0, pack_in: None });
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
                                                      wrap: 0, image: None, left: false, strike: false,
                                                      under: false }, bg);
        let w = &mut self.ws[id];
        (w.bd, w.padx, w.pady) = (2, 1, 1);
        id
    }

    /// tk.Checkbutton(parent, text, bg, fg, font, anchor, selectcolor, highlightthickness, bd); padx = pady = 1
    #[allow(clippy::too_many_arguments)]
    pub fn check(&mut self, parent: usize, text: &str, bg: &str, fg: &str, font: Font, on: bool, select: &str, cmd: &str,
                 hl: i32, bd: i32) -> usize {
        let id = self.add(Some(parent), Kind::Check { text: text.into(), font, fg: fg.into(), on, anchor: Anchor::W, wrap: 0,
                                                      left: false, select: select.into(), cmd: cmd.into(), radio: false,
                                                      noind: false }, bg);
        let w = &mut self.ws[id];
        (w.bd, w.hl, w.padx, w.pady) = (bd, hl, 1, 1);
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

    /// tk.OptionMenu(parent, var, *values) configured as zenko.py does (relief flat, highlightthickness 1 in `ring`,
    /// width in characters); padx / pady are Tk's 4p / 3p and the indicator 4 mm + 2 x 1.7 mm on a screen of
    /// `cm10` pixels per 10 cm
    #[allow(clippy::too_many_arguments)]
    /// tk.Listbox(parent, font, bg, fg, relief flat, highlightthickness 1, highlightbackground ring, selectbackground,
    /// selectforeground, activestyle none, exportselection 0, width chars)
    #[allow(clippy::too_many_arguments)]
    pub fn listbox(&mut self, parent: usize, chars: i32, font: Font, bg: &str, fg: &str, ring: &str, selbg: &str, selfg: &str,
                   cmd: &str) -> usize {
        let id = self.add(Some(parent), Kind::List { items: vec![], font, fg: fg.into(), selbg: selbg.into(), selfg: selfg.into(),
                                                     sel: None, top: 0, chars, lines: 10, cmd: cmd.into() }, bg);
        (self.ws[id].bd, self.ws[id].hl, self.ws[id].hlc) = (1, 1, ring.into());
        id
    }

    /// a listbox's rows that fit whole, its row height
    pub fn list_rows(&self, fonts: &dyn Fonts, id: usize) -> (usize, i32) {
        let w = &self.ws[id];
        let Kind::List { font, .. } = &w.kind else { return (0, 1) };
        let lh = fonts.linespace(font) + 1;
        (((w.h - 2 * (w.bd + w.hl)) / lh).max(1) as usize, lh)
    }

    /// yview scroll n units, kept inside the rows
    pub fn list_scroll(&mut self, fonts: &dyn Fonts, id: usize, n: i32) -> bool {
        let (full, _) = self.list_rows(fonts, id);
        let Kind::List { items, top, .. } = &mut self.ws[id].kind else { return false };
        let max = items.len().saturating_sub(full) as i32;
        let t = (*top as i32 + n).clamp(0, max.max(0)) as usize;
        let ch = t != *top;
        *top = t;
        ch
    }

    /// see(i): the row scrolled into view
    pub fn list_see(&mut self, fonts: &dyn Fonts, id: usize, i: usize) {
        let (full, _) = self.list_rows(fonts, id);
        if let Kind::List { top, items, .. } = &mut self.ws[id].kind {
            if i < *top {
                *top = i;
            } else if i >= *top + full {
                *top = (i + 1).saturating_sub(full).min(items.len().saturating_sub(full));
            }
        }
    }

    /// yview(): the first and last fractions shown
    pub fn list_yview(&self, fonts: &dyn Fonts, id: usize) -> (f64, f64) {
        let (full, _) = self.list_rows(fonts, id);
        let Kind::List { items, top, .. } = &self.ws[id].kind else { return (0.0, 1.0) };
        if items.is_empty() {
            return (0.0, 1.0);
        }
        let n = items.len() as f64;
        (*top as f64 / n, ((*top + full) as f64 / n).min(1.0))
    }

    pub fn menubutton(&mut self, parent: usize, text: &str, cmd: &str, font: Font, bg: &str, fg: &str, ring: &str,
                      chars: i32, cm10: i32) -> usize {
        let pt = |p: f64| (p * cm10 as f64 / 100.0 * 25.4 / 72.0 + 0.5) as i32;
        let ih = 17 * cm10 / 1000;
        let iw = 40 * cm10 / 1000 + 2 * ih;
        let id = self.add(Some(parent), Kind::Menu { text: text.into(), font, fg: fg.into(), ind: (iw, ih), cmd: cmd.into() }, bg);
        let w = &mut self.ws[id];
        (w.bd, w.hl, w.hlc, w.padx, w.pady, w.opt_w) = (2, 1, ring.into(), pt(4.0), pt(3.0), chars);
        // its tk.Menu (a child that is never mapped)
        self.frame(id, bg);
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
            *card = Some(Card { fill: bg.into(), edge: edge.into(), r, shape, rim: mixhex(bg, ink, 0.10), notch: 0 });
        }
        self.ws[cv].place = Some(Place { relw: Some(1.0), relh: Some(1.0), ..Default::default() });
        f
    }

    pub fn pack(&mut self, id: usize, p: Pack) -> usize {
        self.ws[id].pack_in = None;
        self.ws[id].pack = Some(p);
        self.pack_n += 1;
        self.ws[id].pack_seq = self.pack_n;
        id
    }

    /// pack(in_=master): `master` is a frame beside the widget (the same parent)
    /// pack(before=sib): packed in the sibling's place in the packing order
    pub fn pack_before(&mut self, id: usize, sib: usize, p: Pack) {
        self.pack(id, p);
        let seq = self.ws[sib].pack_seq;
        for w in self.ws.iter_mut() {
            if w.pack.is_some() && w.pack_seq >= seq {
                w.pack_seq += 1;
            }
        }
        self.ws[id].pack_seq = seq;
        self.ws[id].pack_in = self.ws[sib].pack_in;
    }

    /// tk.Scale(parent, from_, to, orient="horizontal", length, width, sliderlength, showvalue=0, bd=1,
    /// highlightthickness=0, bg, troughcolor, command)
    #[allow(clippy::too_many_arguments)]
    pub fn scale(&mut self, parent: usize, from: f64, to: f64, val: f64, length: i32, width: i32, slen: i32, bg: &str,
                 trough: &str, cmd: &str) -> usize {
        let id = self.add(Some(parent), Kind::Scale { from, to, val, length, width, slen, trough: trough.into(),
                                                      cmd: cmd.into(), label: String::new(), show: false,
                                                      font: Font::new("Segoe UI", 9, false), fg: "#000000".into() }, bg);
        (self.ws[id].bd, self.ws[id].hl) = (1, 0);
        id
    }

    pub fn pack_into(&mut self, id: usize, master: usize, p: Pack) {
        self.pack(id, p);
        self.ws[id].pack_in = Some(master);
    }

    /// tk.Misc.lift: to the top of its siblings' stacking order (winfo_children order)
    pub fn lift(&mut self, id: usize) {
        if let Some(p) = self.ws[id].parent {
            self.ws[p].kids.retain(|&k| k != id);
            self.ws[p].kids.push(id);
        }
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
            // Windows Tk adds a pixel for the focus ring
            Kind::Check { .. } => {
                let i = w.bd + w.hl + if self.x11 { 0 } else { 1 };
                (i, i)
            }
            Kind::Entry { .. } => (w.bd + w.hl + 1, w.bd + w.hl + 1),
            // Tk_SetInternalBorderEx: border + highlight + padding
            Kind::Text { ref t, .. } => (t.bd + t.hl + t.padx, t.bd + t.hl + t.pady),
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
            Kind::Label { image: Some(img), .. } => {
                // (-padx / -pady pad text only)
                let inset = w.bd + w.hl;
                (img.w as i32 + 2 * inset, img.h as i32 + 2 * inset)
            }
            Kind::Label { text, font, wrap, .. } => {
                let (lines, tw) = text_layout(fonts, font, text, *wrap);
                // -width in characters: that many "0"s wide
                let tw = if w.opt_w > 0 { w.opt_w * fonts.width(font, "0") } else { tw };
                let th = if text.is_empty() { fonts.linespace(font) } else { lines.len() as i32 * fonts.linespace(font) };
                let inset = w.bd + w.hl;
                (tw + 2 * w.padx + 2 * inset, th + 2 * w.pady + 2 * inset)
            }
            Kind::Check { text, font, wrap, noind: true, .. } => {
                // indicatoron 0: a button of its text and padding
                let (lines, tw) = text_layout(fonts, font, text, *wrap);
                let th = lines.len() as i32 * fonts.linespace(font);
                let (ib, _) = self.ib(id);
                (tw + 2 * w.padx + 2 * ib, th + 2 * w.pady + 2 * ib)
            }
            Kind::Scale { length, width, label, show, font, .. } => {
                // tkScale.c ComputeScaleGeometry (horizontal): inset = highlight + border; a label and the value each
                // take a line and SPACING (2) above the trough, and one more SPACING after them
                let inset = w.bd + w.hl;
                let ls = fonts.linespace(font);
                let mut y = inset;
                let mut extra = 0;
                if !label.is_empty() {
                    y += ls + 2;
                    extra = 2;
                }
                if *show {
                    y += ls + 2;
                    extra = 2;
                }
                (length + 2 * inset, y + extra + width + 2 * w.bd + inset)
            }
            Kind::Check { text, font, wrap, radio, .. } => {
                let (lines, tw) = text_layout(fonts, font, text, *wrap);
                // -width in characters: that many "0"s instead of the text's width
                let tw = if w.opt_w > 0 { w.opt_w * fonts.width(font, "0") } else { tw };
                let ls = fonts.linespace(font);
                let th = if text.is_empty() { 0 } else { lines.len() as i32 * ls };
                let (ib, _) = self.ib(id);
                if self.x11 {
                    // tkUnixButton.c: an indicator 80% (check) of a line, then one "0" of space
                    let d = if *radio { ls } else { 80 * ls / 100 };
                    let space = d + fonts.width(font, "0");
                    (tw + 2 * w.padx + space + 2 * ib, th + 2 * w.pady + 2 * ib)
                } else {
                    // tkWinButton.c: 10 DLUs high less the 4 pixels of default padding, a 13-pixel box and 3 DLUs
                    let avg = (fonts.width(font, "abcdefghijklmnopqurstuvwzyABCDEFGHIJKLMNOPQURSTUVWZY") + 26) / 52;
                    let (hdlu, vdlu) = (avg as f64 / 4.0, ls as f64 / 8.0);
                    let mut h = th + (0.5 + 2.0 * vdlu) as i32 - 4 + 2 * w.pady;
                    h = h.max(13);
                    let space = 13 + (0.5 + 3.0 * hdlu) as i32;
                    (tw + 2 * w.padx + space + 2 * ib, h + 2 * ib)
                }
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
            Kind::List { font, chars, lines, .. } => {
                // tkListbox.c ListboxComputeGeometry: -width "0"s, -height rows of linespace + 1, the inset around
                let inset = w.bd + w.hl;
                (chars * fonts.width(font, "0") + 2 * inset, lines * (fonts.linespace(font) + 1) + 2 * inset)
            }
            Kind::Text { t, .. } => t.req_size(fonts),
            Kind::Menu { text, font, ind, .. } => {
                // tkUnixMenubu.c TkpComputeMenubuttonGeometry: -width "0"s (else the text), one line, the indicator,
                // the padding, then border and highlight
                let tw = if w.opt_w > 0 { w.opt_w * fonts.width(font, "0") } else { fonts.width(font, text) };
                let inset = w.bd + w.hl;
                (tw + ind.0 + 2 * w.padx + 2 * inset, fonts.linespace(font) + 2 * w.pady + 2 * inset)
            }
        };
        (r, rr)
    }

    /// requested sizes, bottom up (pack propagates; place doesn't), then the layout, top down
    pub fn layout(&mut self, fonts: &dyn Fonts) {
        self.req(fonts, 0);
        let (rw, rh) = (self.ws[0].rw, self.ws[0].rh);
        (self.ws[0].w, self.ws[0].h) = (rw, rh);
        self.arrange(0);
        self.sync_texts(fonts);
    }

    /// lay out with the toplevel at a given size
    pub fn layout_at(&mut self, fonts: &dyn Fonts, w: i32, h: i32) {
        self.req(fonts, 0);
        (self.ws[0].w, self.ws[0].h) = (w, h);
        self.arrange(0);
        self.sync_texts(fonts);
    }

    /// a Text widget whose size changed lays itself out again (TkTextRelayoutWindow); unmapped ones are 1x1
    pub fn sync_texts(&mut self, fonts: &dyn Fonts) {
        for i in 0..self.ws.len() {
            let shown = self.shown(i);
            let w = &mut self.ws[i];
            let (ww, hh) = if shown { (w.w, w.h) } else { (1, 1) };
            if let Kind::Text { t, .. } = &mut w.kind
                && (t.w, t.h) != (ww, hh) {
                (t.w, t.h) = (ww, hh);
                t.relayout(fonts);
            }
        }
    }

    /// mapped, and every parent too
    pub fn shown(&self, mut id: usize) -> bool {
        loop {
            if !self.ws[id].mapped {
                return false;
            }
            match self.ws[id].parent {
                Some(p) => id = p,
                None => return true,
            }
        }
    }

    /// tk.Text(parent, ...): the widget's own options are set on the returned text
    pub fn text(&mut self, parent: usize, t: crate::tktext::Text, ring: &str) -> usize {
        let bg = t.bg.clone();
        let (bd, hl) = (t.bd, t.hl);
        let id = self.add(Some(parent), Kind::Text { t: Box::new(t), ring: ring.into() }, &bg);
        (self.ws[id].bd, self.ws[id].hl) = (bd, hl);
        id
    }

    pub fn text_mut(&mut self, id: usize) -> &mut crate::tktext::Text {
        match &mut self.ws[id].kind {
            Kind::Text { t, .. } => t,
            _ => panic!("not a text"),
        }
    }

    /// pack_slaves(): what is packed into `id`, in packing order
    pub fn packed(&self, id: usize) -> Vec<usize> {
        let mut v: Vec<usize> = self.ws[id].kids.iter().copied()
            .filter(|&k| self.ws[k].pack.is_some() && self.ws[k].pack_in.is_none()).collect();
        if let Some(p) = self.ws[id].parent {
            v.extend(self.ws[p].kids.iter().copied().filter(|&k| self.ws[k].pack.is_some() && self.ws[k].pack_in == Some(id)));
        }
        v.sort_by_key(|&k| self.ws[k].pack_seq);
        v
    }

    fn req(&mut self, fonts: &dyn Fonts, id: usize) {
        // frames that other kids are packed into come after those kids
        let kids = self.ws[id].kids.clone();
        let into: Vec<usize> = kids.iter().filter_map(|&k| self.ws[k].pack_in).collect();
        for &k in kids.iter().filter(|k| !into.contains(k)).chain(kids.iter().filter(|k| into.contains(k))) {
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
        let gridded = self.gridded(id);
        if !gridded.is_empty() && self.ws[id].propagate {
            let (bx, by) = self.ib(id);
            let mut cells = self.cells(&gridded);
            for (col, w) in self.uniform_cols(id, &cells) {
                cells.push(crate::tkgrid::Cell::new(0, col, w, 0));
            }
            let ((gw, gh), _) = crate::tkgrid::grid(&cells, bx, by);
            (rw, rh) = (gw + 2 * bx, gh + 2 * by);
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
            let (mut x, mut y) = (cx(p.anchor), cy(p.anchor));
            if self.ws[c].pack_in.is_some() {
                // (the master is a sibling: the widget's place is relative to their parent)
                (x, y) = (x + self.ws[id].x, y + self.ws[id].y);
            }
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
            let mut x = rnd(x1);
            let y1 = pl.y as f64 + my + pl.rely * mh;
            let mut y = rnd(y1);
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
            if pl.ne {
                x -= width;
            } else if pl.n {
                x -= width / 2;
            } else if pl.center {
                x -= width / 2;
                y -= height / 2;
            }
            let w = &mut self.ws[k];
            if width <= 0 || height <= 0 {
                w.mapped = false;
            } else {
                (w.x, w.y, w.w, w.h, w.mapped) = (x, y, width, height, true);
            }
        }
        self.grid_arrange(id);
        for k in self.ws[id].kids.clone() {
            if self.ws[k].cwin {
                // the canvas's window item: at (0, -yview), the canvas's width, its own requested height
                let (cw, rh, yo) = (self.ws[id].w, self.ws[k].rh, self.ws[id].yoff);
                let w = &mut self.ws[k];
                (w.x, w.y, w.w, w.h, w.mapped) = (0, -yo, cw, rh, true);
            }
        }
        for k in self.ws[id].kids.clone() {
            if self.ws[k].mapped {
                self.arrange(k);
            }
        }
    }

    /// tkGrid.c step 2b: the columns of a uniform group all get the group's largest (per weight) minimum size;
    /// returned as extra one-column demands
    fn uniform_cols(&self, id: usize, cells: &[crate::tkgrid::Cell]) -> Vec<(usize, i32)> {
        let gu = &self.ws[id].gu_cols;
        if gu.iter().all(|g| g.is_empty()) {
            return vec![];
        }
        let weight = |c: usize| self.ws[id].gw_cols.get(c).copied().unwrap_or(0).max(1);
        let mut min = vec![0; gu.len()];
        for c in cells.iter().filter(|c| c.cols == 1 && c.col < gu.len()) {
            min[c.col] = min[c.col].max(c.w + c.padx.0 + c.padx.1);
        }
        let mut out = vec![];
        for (c, g) in gu.iter().enumerate().filter(|(_, g)| !g.is_empty()) {
            let m = gu.iter().enumerate().filter(|(_, h)| *h == g).map(|(k, _)| (min[k] + weight(k) - 1) / weight(k)).max().unwrap_or(0);
            out.push((c, m * weight(c)));
        }
        out
    }

    fn gridded(&self, id: usize) -> Vec<usize> {
        self.ws[id].kids.iter().copied().filter(|&k| self.ws[k].grid.is_some()).collect()
    }

    fn cells(&self, ks: &[usize]) -> Vec<crate::tkgrid::Cell> {
        ks.iter().map(|&k| {
            let g = self.ws[k].grid.clone().unwrap();
            let mut c = crate::tkgrid::Cell::new(g.row, g.col, self.ws[k].rw, self.ws[k].rh).sticky(g.sticky)
                .padx(g.padx.0, g.padx.1).pady(g.pady.0, g.pady.1).span(g.cspan.max(1));
            c.rows = g.rspan.max(1);
            c
        }).collect()
    }

    /// tkGrid.c ArrangeGrid: slots at their natural sizes, the container's extra room shared out by weight
    /// (AdjustOffsets), anchor nw, then each widget stuck into its cell
    fn grid_arrange(&mut self, id: usize) {
        for k in self.ws[id].kids.clone() {
            // forgotten / grid_remove'd: not on screen
            if self.ws[k].pack.is_none() && self.ws[k].place.is_none() && !self.ws[k].cwin && self.ws[k].grid.is_none() {
                self.ws[k].mapped = false;
            }
        }
        let ks = self.gridded(id);
        if ks.is_empty() {
            return;
        }
        let (bx, by) = self.ib(id);
        let cells = self.cells(&ks);
        let axis = |items: Vec<(usize, usize, i32)>, weights: &[i32], size: i32| -> Vec<i32> {
            let (mut off, _) = crate::tkgrid::resolve(&items);
            let n = off.len();
            let ws: Vec<i32> = (0..n).map(|i| weights.get(i).copied().unwrap_or(0)).collect();
            let total: i32 = ws.iter().sum();
            let diff = size - off.last().copied().unwrap_or(0);
            if total > 0 && diff != 0 {
                let mut acc = 0;
                for i in 0..n {
                    acc += ws[i];
                    off[i] += diff * acc / total;
                }
            }
            off
        };
        let mut cols: Vec<_> = cells.iter().map(|c| (c.col, c.cols, c.w + c.padx.0 + c.padx.1)).collect();
        cols.extend(self.uniform_cols(id, &cells).into_iter().map(|(c, w)| (c, 1, w)));
        let rows: Vec<_> = cells.iter().map(|c| (c.row, c.rows, c.h + c.pady.0 + c.pady.1)).collect();
        let (cw0, ch0) = (self.ws[id].w - 2 * bx, self.ws[id].h - 2 * by);
        let co = axis(cols, &self.ws[id].gw_cols.clone(), cw0);
        let ro = axis(rows, &self.ws[id].gw_rows.clone(), ch0);
        for (c, &k) in cells.iter().zip(&ks) {
            let mut x = if c.col > 0 { co[c.col - 1] } else { 0 };
            let mut y = if c.row > 0 { ro[c.row - 1] } else { 0 };
            let mut cw = co[c.col + c.cols - 1] - x;
            let mut ch = ro[c.row + c.rows - 1] - y;
            x += bx + c.padx.0;
            cw -= c.padx.0 + c.padx.1;
            y += by + c.pady.0;
            ch -= c.pady.0 + c.pady.1;
            let (rw, rh) = (c.w, c.h);
            let (mut dx, mut dy) = (0, 0);
            if cw > rw {
                dx = cw - rw;
                cw = rw;
            }
            if ch > rh {
                dy = ch - rh;
                ch = rh;
            }
            let st = |q: char| c.sticky.contains(q);
            if st('e') && st('w') {
                cw += dx;
            }
            if st('n') && st('s') {
                ch += dy;
            }
            if !st('w') {
                x += if st('e') { dx } else { dx / 2 };
            }
            if !st('n') {
                y += if st('s') { dy } else { dy / 2 };
            }
            let w = &mut self.ws[k];
            if cw <= 0 || ch <= 0 {
                w.mapped = false;
            } else {
                (w.x, w.y, w.w, w.h, w.mapped) = (x, y, cw, ch, true);
            }
        }
    }

    pub fn grid(&mut self, id: usize, g: Grid) -> usize {
        self.ws[id].grid = Some(g);
        id
    }

    /// a label showing a picture (tk.Label(image=...)); bd as given
    /// destroy(): the widget and everything inside it leave the tree (their slots stay, unreachable)
    pub fn destroy(&mut self, id: usize) {
        if let Some(p) = self.ws[id].parent.take() {
            self.ws[p].kids.retain(|&k| k != id);
        }
        let mut stack = vec![id];
        while let Some(x) = stack.pop() {
            let w = &mut self.ws[x];
            (w.pack, w.place, w.grid, w.mapped) = (None, None, None, false);
            stack.extend(w.kids.iter().copied());
        }
        if self.focus.is_some_and(|f| self.under(f, id)) {
            self.focus = None;
        }
    }

    fn under(&self, mut x: usize, root: usize) -> bool {
        loop {
            if x == root {
                return true;
            }
            match self.ws[x].parent {
                Some(p) => x = p,
                None => return false,
            }
        }
    }

    pub fn image_label(&mut self, parent: usize, img: std::rc::Rc<crate::tkcanvas::Photo>, bg: &str, bd: i32) -> usize {
        let id = self.label(parent, "", bg, "#000000", Font::new("TkDefaultFont", 9, false));
        if let Kind::Label { image, .. } = &mut self.ws[id].kind {
            *image = Some(img);
        }
        self.ws[id].bd = bd;
        id
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
        if w.hl > 0 && !w.hlc.is_empty() && matches!(w.kind, Kind::Frame | Kind::Menu { .. }) {
            for (rx, ry, rw, rh) in [(ax, ay, w.w, w.hl), (ax, ay + w.h - w.hl, w.w, w.hl), (ax, ay, w.hl, w.h),
                                     (ax + w.w - w.hl, ay, w.hl, w.h)] {
                out.push(Draw::Rect { x: rx, y: ry, w: rw, h: rh, color: w.hlc.clone() });
            }
        }
        match &w.kind {
            Kind::Frame => {}
            Kind::Menu { text, font, fg, ind, .. } => {
                // TkpDisplayMenubutton: the text centred with the indicator's room, then the raised indicator bar
                let tw = fonts.width(font, text);
                let ls = fonts.linespace(font);
                let x = (w.w - (tw + ind.0)) / 2;
                let y = (w.h - ls) / 2;
                out.push(Draw::Text { x: ax + x, y: ay + y, text: text.clone(), font: font.clone(), color: fg.clone() });
                let inset = w.bd + w.hl;
                let (bx, by, bw, bh) = (ax + w.w - inset - ind.0 + ind.1, ay + (w.h - ind.1) / 2, ind.0 - 2 * ind.1, ind.1);
                let b = 1.max((ind.1 + 1) / 3);
                let (light, dark) = shadows(&w.bg);
                out.push(Draw::Rect { x: bx, y: by, w: bw, h: bh, color: dark });
                out.push(Draw::Rect { x: bx, y: by, w: bw - b, h: bh - b, color: light });
                out.push(Draw::Rect { x: bx + b, y: by + b, w: (bw - 2 * b).max(0), h: (bh - 2 * b).max(0), color: w.bg.clone() });
            }
            Kind::Label { image: Some(img), .. } => {
                let ib = w.bd + w.hl;
                let (iw, ih) = (img.w as i32, img.h as i32);
                let x = (w.w - iw - 2 * ib) / 2 + ib;
                let y = (w.h - ih - 2 * ib) / 2 + ib;
                out.push(Draw::Tk { x: ax, y: ay, ops: vec![crate::canvasdraw::TkOp::Photo { x, y, img: img.clone() }] });
            }
            Kind::Label { text, font, fg, anchor, wrap, left, strike, under, .. } => {
                let (lines, tw) = text_layout(fonts, font, text, *wrap);
                // -width in characters: that many "0"s wide
                let tw = if w.opt_w > 0 { w.opt_w * fonts.width(font, "0") } else { tw };
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
                    // justify center (the Label default) or left
                    let lx = if *left { x } else { x + (tw - lw) / 2 };
                    out.push(Draw::Text { x: ax + lx, y: ay + y + i as i32 * ls, text: l.clone(), font: font.clone(),
                                          color: fg.clone() });
                    if *strike {
                        // overstrike: Tk's underline-style bar through the middle of the x height (ascent / 2 up)
                        let (asc, _) = fonts.metrics(font);
                        let sy = ay + y + i as i32 * ls + asc - asc / 2;
                        let th = 1.max(font.size / 10);
                        out.push(Draw::Rect { x: ax + lx, y: sy, w: lw, h: th, color: fg.clone() });
                    }
                    if *under {
                        // Tk_UnderlineChars: desc / 2 under the baseline, a tenth of the size high (at least 1)
                        let (asc, desc) = fonts.metrics(font);
                        let mut pos = desc / 2;
                        let mut uh = ((fonts.pixels(font) / 10.0 + 0.5) as i32).max(1);
                        if pos + uh > desc {
                            uh = (desc - pos).max(1);
                            pos = pos.min(desc - 1);
                        }
                        out.push(Draw::Rect { x: ax + lx, y: ay + y + i as i32 * ls + asc + pos, w: lw, h: uh, color: fg.clone() });
                    }
                }
            }
            Kind::Check { text, font, fg, on, wrap, select, noind: true, .. } => {
                // a button-style radiobutton: selectcolor while chosen, the text centred
                if *on {
                    out.push(Draw::Rect { x: ax, y: ay, w: w.w, h: w.h, color: select.clone() });
                }
                let (lines, _) = text_layout(fonts, font, text, *wrap);
                let ls = fonts.linespace(font);
                let y = (w.h - lines.len() as i32 * ls) / 2;
                for (i, l) in lines.iter().enumerate() {
                    let lw = fonts.width(font, l.trim_end_matches(' '));
                    out.push(Draw::Text { x: ax + (w.w - lw) / 2, y: ay + y + i as i32 * ls, text: l.clone(), font: font.clone(),
                                          color: fg.clone() });
                }
            }
            Kind::Scale { from, to, val, slen, trough, label, show, font, fg, width, .. } => {
                // tkUnixScale.c DisplayHorizontalScale: the label and the value over it, the sunken trough, the flat
                // slider at TkScaleValueToPixel
                let inset = w.bd + w.hl;
                let ls = fonts.linespace(font);
                let mut top = inset;
                if !label.is_empty() {
                    out.push(Draw::Text { x: ax + inset + ls / 2, y: ay + top + 2, text: label.clone(), font: font.clone(),
                                          color: fg.clone() });
                    top += ls + 2;
                }
                let range0 = to - from;
                let pr0 = (w.w - slen - 2 * inset - 2 * w.bd) as f64;
                let cx0 = if range0 == 0.0 { 0.0 } else { (val - from) * pr0 / range0 } + *slen as f64 / 2.0 + inset as f64
                    + w.bd as f64 + 0.5;
                if *show {
                    let v = format!("{}", *val as i64);
                    let vw = fonts.width(font, &v);
                    out.push(Draw::Text { x: ax + cx0 as i32 - vw / 2, y: ay + top + 2, text: v, font: font.clone(),
                                          color: fg.clone() });
                    top += ls + 2;
                }
                if top > inset {
                    top += 2;
                }
                let th0 = width + 2 * w.bd;
                let (tx, ty, tw, th) = (ax + inset, ay + top, w.w - 2 * inset, th0);
                let (light, dark) = shadows(&w.bg);
                out.push(Draw::Rect { x: tx, y: ty, w: tw, h: th, color: light });
                out.push(Draw::Rect { x: tx, y: ty, w: tw - w.bd, h: th - w.bd, color: dark });
                out.push(Draw::Rect { x: tx + w.bd, y: ty + w.bd, w: tw - 2 * w.bd, h: th - 2 * w.bd, color: trough.clone() });
                let range = to - from;
                let pr = (w.w - slen - 2 * inset - 2 * w.bd) as f64;
                let cx = if range == 0.0 { 0.0 } else { (val - from) * pr / range }
                    + *slen as f64 / 2.0 + inset as f64 + w.bd as f64 + 0.5;
                let sx = ax + cx as i32 - slen / 2;
                out.push(Draw::Rect { x: sx, y: ty + w.bd, w: *slen, h: th - 2 * w.bd, color: w.bg.clone() });
            }
            Kind::Check { text, font, fg, on, wrap, left, select, radio, anchor, .. } => {
                let (lines, tw) = text_layout(fonts, font, text, *wrap);
                let ls = fonts.linespace(font);
                let th = lines.len() as i32 * ls;
                let (ib, _) = self.ib(id);
                let (rw, _) = self.own_req(fonts, id);
                let space = rw - tw - 2 * w.padx - 2 * ib;
                let x = match anchor {
                    Anchor::Center | Anchor::N | Anchor::S => (w.w - rw) / 2 + ib + space + w.padx,
                    _ => ib + space + w.padx,
                };
                let y = (w.h - th) / 2;
                // the indicator: a 13-pixel box (round for a radiobutton) in selectcolor, its mark when on
                let (bx, by) = (ax + x - space, ay + (w.h - 13) / 2);
                if *radio {
                    out.push(Draw::Oval { x1: bx, y1: by, x2: bx + 13, y2: by + 13, fill: t.line.clone() });
                    out.push(Draw::Oval { x1: bx + 1, y1: by + 1, x2: bx + 12, y2: by + 12, fill: select.clone() });
                    if *on {
                        out.push(Draw::Oval { x1: bx + 4, y1: by + 4, x2: bx + 9, y2: by + 9, fill: fg.clone() });
                    }
                } else {
                    out.push(Draw::Rect { x: bx, y: by, w: 13, h: 13, color: t.line.clone() });
                    out.push(Draw::Rect { x: bx + 1, y: by + 1, w: 11, h: 11, color: select.clone() });
                    if *on {
                        for (k, (dx, dy)) in [(3, 5), (4, 6), (5, 7), (6, 6), (7, 5), (8, 4), (9, 3)].iter().enumerate() {
                            let _ = k;
                            out.push(Draw::Rect { x: bx + dx, y: by + dy, w: 1, h: 3, color: fg.clone() });
                        }
                    }
                }
                for (i, l) in lines.iter().enumerate() {
                    let lw = fonts.width(font, l.trim_end_matches(' '));
                    let lx = if *left { x } else { x + (tw - lw) / 2 };
                    out.push(Draw::Text { x: ax + lx, y: ay + y + i as i32 * ls, text: l.clone(), font: font.clone(),
                                          color: fg.clone() });
                }
            }
            Kind::Entry { font, e, .. } => {
                let focus = self.active && self.focus == Some(id);
                let ib = w.bd + w.hl + 1;
                let (x, y, wd, ht) = (ax, ay, w.w, w.h);
                let (ink, ins) = match &e.flat {
                    Some((fg, ic)) => {
                        out.push(Draw::Rect { x, y, w: wd, h: ht, color: w.bg.clone() });
                        (fg.clone(), ic.clone())
                    }
                    None => {
                        out.push(Draw::Rect { x, y, w: wd, h: ht, color: t.field.clone() });
                        let ring = if focus { t.gold.clone() } else { t.line.clone() };
                        for (rx, ry, rw, rh) in [(x, y, wd, 1), (x, y + ht - 1, wd, 1), (x, y, 1, ht), (x + wd - 1, y, 1, ht)] {
                            out.push(Draw::Rect { x: rx, y: ry, w: rw, h: rh, color: ring.clone() });
                        }
                        (t.ink.clone(), t.ink.clone())
                    }
                };
                let (asc, desc) = fonts.metrics(font);
                let ls = asc + desc;
                let ty = y + (ht + asc - desc) / 2 - asc;
                let shown: Vec<u16> = match e.show { Some(c) => vec![c as u16; e.text.len()], None => e.text.clone() };
                let xo = |i: usize| fonts.width(font, &String::from_utf16_lossy(&shown[..i.min(shown.len())]));
                let tx = x + ib - xo(e.left);
                let inner = (x1.max(x + 2), y1.max(y + 2), (x2.min(x + wd - 2) - x1.max(x + 2)).max(0),
                             (y2.min(y + ht - 2) - y1.max(y + 2)).max(0));
                out.push(Draw::Clip(Some(inner)));
                if let Some((a, b)) = e.sel {
                    let (sx, ex) = (tx + xo(a), tx + xo(b));
                    out.push(Draw::Rect { x: sx, y: ty, w: ex - sx, h: ls, color: t.acc.clone() });
                }
                match &e.ph {
                    Some(ph) if e.text.is_empty() && !focus => out.push(Draw::Text { x: x + ib, y: ty, text: ph.clone(),
                                                                                    font: font.clone(), color: t.grey.clone() }),
                    _ => out.push(Draw::Text { x: tx, y: ty, text: String::from_utf16_lossy(&shown), font: font.clone(),
                                               color: ink.clone() }),
                }
                if let Some((a, b)) = e.sel {
                    out.push(Draw::Text { x: tx + xo(a), y: ty, text: String::from_utf16_lossy(&shown[a..b]),
                                          font: font.clone(), color: t.onacc.clone() });
                }
                // the insertion cursor: 2 pixels wide, blinking 600 ms on / 300 ms off while focused
                if focus && ((now - e.blink_t0) * 1000.0) as i64 % 900 < 600 {
                    out.push(Draw::Rect { x: tx + xo(e.pos) - 1, y: ty, w: 2, h: ls, color: ins });
                }
                out.push(Draw::Clip(Some(me)));
            }
            Kind::List { items, font, fg, selbg, selfg, sel, top, .. } => {
                // DisplayListbox: each row from the inset down, its text at inset + selectborderwidth; the selected row
                // filled in selectbackground across the inside
                let inset = w.bd + w.hl;
                if w.hl > 0 && !w.hlc.is_empty() {
                    let ring = if self.active && self.focus == Some(id) { t.gold.clone() } else { w.hlc.clone() };
                    for (rx, ry, rw, rh) in [(ax, ay, w.w, w.hl), (ax, ay + w.h - w.hl, w.w, w.hl), (ax, ay, w.hl, w.h),
                                             (ax + w.w - w.hl, ay, w.hl, w.h)] {
                        out.push(Draw::Rect { x: rx, y: ry, w: rw, h: rh, color: ring.clone() });
                    }
                }
                let lh = fonts.linespace(font) + 1;
                let inner = (x1.max(ax + inset), y1.max(ay + inset), (x2.min(ax + w.w - inset) - x1.max(ax + inset)).max(0),
                             (y2.min(ay + w.h - inset) - y1.max(ay + inset)).max(0));
                out.push(Draw::Clip(Some(inner)));
                for (k, (txt, col)) in items.iter().enumerate().skip(*top) {
                    let y = ay + inset + (k - *top) as i32 * lh;
                    if y >= ay + w.h - inset {
                        break;
                    }
                    let on = *sel == Some(k);
                    if on {
                        out.push(Draw::Rect { x: ax + inset, y, w: w.w - 2 * inset, h: lh, color: selbg.clone() });
                    }
                    let c = if on { selfg.clone() } else { col.clone().unwrap_or_else(|| fg.clone()) };
                    out.push(Draw::Text { x: ax + inset, y, text: txt.clone(), font: font.clone(), color: c });
                }
                out.push(Draw::Clip(Some(me)));
            }
            Kind::Canvas { tk: Some(cv), .. } => {
                out.push(Draw::Tk { x: ax, y: ay, ops: crate::canvasdraw::render(cv, fonts) });
            }
            Kind::Text { t, ring } => {
                out.push(Draw::Tk { x: ax, y: ay, ops: t.ops(fonts, ring) });
                // the insertion cursor of a focused editable box (blinking like an entry's)
                if let Some(off) = t.edit
                    && self.active && self.focus == Some(id) && (now * 1000.0) as i64 % 900 < 600 {
                    let idx = t.index_of(off);
                    if let Some(b) = t.bbox(fonts, idx).or_else(|| t.dlineinfo(fonts, idx).map(|d| [d[0], d[1], 0, d[3]])) {
                        out.push(Draw::Rect { x: ax + b[0], y: ay + b[1], w: 2, h: b[3], color: t.fg.clone() });
                    }
                }
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
                        v.push(Item::Poly { pts: crate::shapes::round_pts_notch(0, 0, wd - 1, ht - 1, c.r, 2, c.shape, c.notch),
                                            fill: Some(c.fill.clone()),
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
            _ if self.hit(x, y).is_some_and(|i| self.ws[i].hand) => Cursor::Hand,
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
        let shown: Vec<u16> = match e.show { Some(c) => vec![c as u16; e.text.len()], None => e.text.clone() };
        let xo = |i: usize| fonts.width(font, &String::from_utf16_lossy(&shown[..i]));
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
            Kind::Check { .. } => self.pressed = Some(i),
            Kind::List { .. } => {
                let (_, ay) = self.abs(i);
                let inset = self.ws[i].bd + self.ws[i].hl;
                let (_, lh) = self.list_rows(fonts, i);
                self.focus = Some(i);
                if let Kind::List { items, sel, top, .. } = &mut self.ws[i].kind {
                    let k = *top + ((y - ay - inset).max(0) / lh) as usize;
                    if k < items.len() {
                        *sel = Some(k);
                        self.pressed = Some(i);
                        self.dbl = double;
                    }
                }
            }
            Kind::Text { t, .. } if t.edit.is_some() => {
                self.focus = Some(i);
                if let Kind::Text { t, .. } = &mut self.ws[i].kind {
                    t.edit = Some(t.plain().chars().count());
                }
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
        let inside = ax <= x && x < ax + ww && ay <= y && y < ay + wh;
        if let Kind::List { sel: Some(k), cmd, .. } = &self.ws[i].kind
            && !cmd.is_empty() {
            return Some(if self.dbl { format!("{cmd}_open:{k}") } else { format!("{cmd}:{k}") });
        }
        if let Kind::Check { on, radio, cmd, .. } = &mut self.ws[i].kind
            && inside && !cmd.is_empty() {
            // (state="disabled" checkbuttons have no command and don't change)
            // a checkbutton flips; a radiobutton is chosen (its command sets the others)
            *on = if *radio { true } else { !*on };
            return Some(cmd.clone());
        }
        None
    }

    pub fn text_of(&self, id: usize) -> String {
        match &self.ws[id].kind {
            Kind::Entry { e, .. } => String::from_utf16_lossy(&e.text),
            Kind::Label { text, .. } | Kind::Button { text, .. } | Kind::Check { text, .. } | Kind::Menu { text, .. } => text.clone(),
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

    /// the focused editable Text box: its plain text and cursor
    fn text_edit(&mut self) -> Option<&mut crate::tktext::Text> {
        let f = self.focus?;
        match &mut self.ws[f].kind {
            Kind::Text { t, .. } if t.edit.is_some() => Some(t),
            _ => None,
        }
    }

    /// typed characters into the focused entry
    pub fn insert(&mut self, fonts: &dyn Fonts, s: &[u16], now: f64) {
        if let Some(t) = self.text_edit() {
            let mut v: Vec<char> = t.plain().chars().collect();
            let at = t.edit.unwrap().min(v.len());
            let add: Vec<char> = String::from_utf16_lossy(s).chars().collect();
            v.splice(at..at, add.iter().copied());
            t.set_plain(fonts, &v.iter().collect::<String>());
            t.edit = Some(at + add.len());
            return;
        }
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
        if let Some(t) = self.text_edit() {
            // an editable Text box: arrows, Home / End of the line, Backspace, Delete and Return
            let mut v: Vec<char> = t.plain().chars().collect();
            let at = t.edit.unwrap().min(v.len());
            let bol = v[..at].iter().rposition(|&c| c == '\n').map(|i| i + 1).unwrap_or(0);
            let eol = v[at..].iter().position(|&c| c == '\n').map(|i| at + i).unwrap_or(v.len());
            let to = match vk {
                0x25 => at.saturating_sub(1),
                0x27 => (at + 1).min(v.len()),
                0x24 => bol,
                0x23 => eol,
                0x08 if at > 0 => {
                    v.remove(at - 1);
                    t.set_plain(fonts, &v.iter().collect::<String>());
                    at - 1
                }
                0x2E if at < v.len() => {
                    v.remove(at);
                    t.set_plain(fonts, &v.iter().collect::<String>());
                    at
                }
                0x0D => {
                    v.insert(at, '\n');
                    t.set_plain(fonts, &v.iter().collect::<String>());
                    at + 1
                }
                0x08 | 0x2E => at,
                _ => return false,
            };
            t.edit = Some(to);
            return true;
        }
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
