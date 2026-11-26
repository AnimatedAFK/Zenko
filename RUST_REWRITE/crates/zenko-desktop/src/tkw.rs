//! Tk widgets as zenko.py's dialogs use them, drawn into a GDI display list: Label (tkWinButton.c geometry), Entry
//! (tkEntry.c) and zenko.py's PixelButton (a canvas with a pixel-rounded pill).

use crate::gdi::{self, Op, rgb};
use windows_sys::Win32::Graphics::Gdi::*;
use zenko_art::palette::mixhex;

#[allow(dead_code)]
pub struct Metrics {
    pub ascent: i32,
    pub descent: i32,
    pub linespace: i32,
}

pub fn metrics(font: HFONT) -> Metrics {
    unsafe {
        let dc = CreateCompatibleDC(std::ptr::null_mut());
        let old = SelectObject(dc, font as _);
        let mut tm: TEXTMETRICW = std::mem::zeroed();
        GetTextMetricsW(dc, &mut tm);
        SelectObject(dc, old);
        DeleteDC(dc);
        Metrics { ascent: tm.tmAscent, descent: tm.tmDescent, linespace: tm.tmAscent + tm.tmDescent }
    }
}

pub fn text_width(font: HFONT, s: &[u16]) -> i32 {
    unsafe {
        let dc = CreateCompatibleDC(std::ptr::null_mut());
        let old = SelectObject(dc, font as _);
        let w = gdi::extent(dc, s);
        SelectObject(dc, old);
        DeleteDC(dc);
        w
    }
}

/// tk.Label(text=..., font=..., wraplength=...): borderwidth 2 (flat), padx/pady 1.
pub struct Label {
    pub lay: gdi::TextLayout,
    pub font: HFONT,
    pub fg: u32,
}

impl Label {
    pub fn new(font: HFONT, text: &str, fg: &str, wrap: i32) -> Label {
        Label { lay: gdi::layout(font, text, wrap), font, fg: rgb(fg) }
    }
    pub fn size(&self) -> (i32, i32) {
        (self.lay.width + 6, self.lay.height + 6)
    }
    pub fn draw(self, ops: &mut Vec<Op>, x: i32, y: i32) {
        ops.push(Op::Text { x: x + 3, y: y + 3, lay: self.lay, font: self.font, color: self.fg, right: false });
    }
}

/// zenko.py PixelButton: a rounded, pixel-edged button with hover glow and a press dip.
pub struct PixelButton {
    pub text: String,
    pub font: HFONT,
    pub bg: String,
    pub fg: String,
    pub padx: i32,
    pub pady: i32,
    pub hover: bool,
    pub down: bool,
    pub rect: (i32, i32, i32, i32),
}

impl PixelButton {
    pub fn new(text: &str, font: HFONT, bg: &str, fg: &str, padx: i32, pady: i32) -> PixelButton {
        PixelButton { text: text.into(), font, bg: bg.into(), fg: fg.into(), padx, pady, hover: false, down: false,
                      rect: (0, 0, 0, 0) }
    }
    pub fn req(&self) -> (i32, i32) {
        let w: Vec<u16> = self.text.encode_utf16().collect();
        (text_width(self.font, &w) + 2 * self.padx + 10, metrics(self.font).linespace + 2 * self.pady + 6)
    }
    pub fn hit(&self, x: i32, y: i32) -> bool {
        let (rx, ry, w, h) = self.rect;
        rx <= x && x < rx + w && ry <= y && y < ry + h
    }
    /// _draw, on a canvas of the parent's colour `pbg` at self.rect
    pub fn draw(&self, ops: &mut Vec<Op>, pbg: &str, t: &zenko_desktop::theme::Theme) {
        let (ox, oy, w, h) = self.rect;
        ops.push(Op::Rect { x: ox, y: oy, w, h, color: rgb(pbg) });
        let bg = self.bg.as_str();
        let fill = if self.down {
            mixhex(bg, "#000000", 0.22)
        } else if self.hover {
            mixhex(bg, "#ffffff", 0.12)
        } else {
            bg.to_string()
        };
        let edge = if self.hover {
            t.gold.clone()
        } else if bg == t.acc {
            mixhex(bg, "#ffffff", 0.35)
        } else {
            mixhex(bg, "#ffffff", 0.2)
        };
        let dy = self.down as i32;
        let r = (h / 2).min(14);
        let pts = gdi::round_pts(ox + 1, oy + 1 + dy, ox + w - 2, oy + h - 2 + dy, r, 2, "pill");
        ops.push(Op::Poly { pts, fill: Some(rgb(&fill)), outline: Some(rgb(&edge)), width: 1 });
        if !self.down {
            let hi = mixhex(&fill, "#ffffff", 0.22);
            ops.push(Op::Line { pts: vec![(ox + 1 + r, oy + 3), (ox + w - 2 - r, oy + 3)], color: rgb(&hi), width: 1 });
        }
        let lay = gdi::layout(self.font, &self.text, 0);
        let (tx, ty) = (ox + w / 2 - lay.width / 2, oy + h / 2 + dy - lay.height / 2);
        ops.push(Op::Text { x: tx, y: ty, lay, font: self.font, color: rgb(&self.fg), right: false });
    }
}

/// tk.Entry as zenko.py's tt_entry makes it: flat, borderwidth 1, highlightthickness 1 (LINE, GOLD when focused),
/// FIELD background, INK text and cursor, ORANGE selection with ON_ACC text.
pub struct Entry {
    pub text: Vec<u16>,
    pub chars: i32,
    pub font: HFONT,
    pub pos: usize,
    pub anchor: usize,
    pub sel: Option<(usize, usize)>,
    pub left: usize,
    pub focus: bool,
    pub rect: (i32, i32, i32, i32),
    pub blink_t0: f64,
}

impl Entry {
    pub fn new(font: HFONT, text: &str, chars: i32) -> Entry {
        Entry { text: text.encode_utf16().collect(), chars, font, pos: 0, anchor: 0, sel: None, left: 0, focus: false,
                rect: (0, 0, 0, 0), blink_t0: 0.0 }
    }
    pub fn req(&self) -> (i32, i32) {
        let avg = text_width(self.font, &['0' as u16]).max(1);
        (self.chars * avg + 4, metrics(self.font).linespace + 4)
    }
    pub fn value(&self) -> String {
        String::from_utf16_lossy(&self.text)
    }
    fn x_of(&self, i: usize) -> i32 {
        text_width(self.font, &self.text[..i.min(self.text.len())])
    }
    /// the character index nearest to window x
    pub fn index_at(&self, x: i32) -> usize {
        let base = self.rect.0 + 2 - self.x_of(self.left);
        let mut best = 0;
        for i in 0..=self.text.len() {
            let cx = base + self.x_of(i);
            if i > 0 {
                let px = base + self.x_of(i - 1);
                if x < (px + cx) / 2 {
                    break;
                }
            }
            best = i;
        }
        best
    }
    fn see(&mut self) {
        let inner = self.rect.2 - 4;
        if self.pos < self.left {
            self.left = self.pos;
        }
        while self.left < self.pos && self.x_of(self.pos) - self.x_of(self.left) > inner - 2 {
            self.left += 1;
        }
    }
    fn del_sel(&mut self) -> bool {
        if let Some((a, b)) = self.sel.take() {
            self.text.drain(a..b);
            self.pos = a;
            true
        } else {
            false
        }
    }
    pub fn insert(&mut self, s: &[u16]) {
        self.del_sel();
        for (k, c) in s.iter().enumerate() {
            self.text.insert(self.pos + k, *c);
        }
        self.pos += s.len();
        self.see();
    }
    pub fn select_all(&mut self) {
        self.sel = if self.text.is_empty() { None } else { Some((0, self.text.len())) };
        self.anchor = 0;
        self.pos = self.text.len();
        self.see();
    }
    pub fn selected(&self) -> Option<Vec<u16>> {
        self.sel.map(|(a, b)| self.text[a..b].to_vec())
    }
    /// a key press (virtual key); returns true when it was handled
    pub fn key(&mut self, vk: u16, shift: bool, ctrl: bool) -> bool {
        let n = self.text.len();
        let mv = |e: &mut Entry, to: usize| {
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
            e.see();
        };
        let word_left = |e: &Entry| {
            let mut i = e.pos;
            while i > 0 && e.text[i - 1] == ' ' as u16 {
                i -= 1;
            }
            while i > 0 && e.text[i - 1] != ' ' as u16 {
                i -= 1;
            }
            i
        };
        let word_right = |e: &Entry| {
            let mut i = e.pos;
            while i < e.text.len() && e.text[i] != ' ' as u16 {
                i += 1;
            }
            while i < e.text.len() && e.text[i] == ' ' as u16 {
                i += 1;
            }
            i
        };
        match vk {
            0x25 => {
                // Left
                let to = if !shift && self.sel.is_some() {
                    self.sel.unwrap().0
                } else if ctrl {
                    word_left(self)
                } else {
                    self.pos.saturating_sub(1)
                };
                mv(self, to);
            }
            0x27 => {
                let to = if !shift && self.sel.is_some() {
                    self.sel.unwrap().1
                } else if ctrl {
                    word_right(self)
                } else {
                    (self.pos + 1).min(n)
                };
                mv(self, to);
            }
            0x24 => mv(self, 0),
            0x23 => mv(self, n),
            0x08 => {
                if !self.del_sel() && self.pos > 0 {
                    let a = if ctrl { word_left(self) } else { self.pos - 1 };
                    self.text.drain(a..self.pos);
                    self.pos = a;
                }
                self.see();
            }
            0x2E => {
                if !self.del_sel() && self.pos < n {
                    self.text.remove(self.pos);
                }
                self.see();
            }
            _ => return false,
        }
        true
    }

    pub fn draw(&self, ops: &mut Vec<Op>, t: &zenko_desktop::theme::Theme, now: f64, focus: bool) {
        let (x, y, w, h) = self.rect;
        let m = metrics(self.font);
        // background (the flat 1-pixel border is the same colour), then the 1-pixel highlight ring
        ops.push(Op::Rect { x, y, w, h, color: rgb(&t.field) });
        let ring = rgb(if focus { &t.gold } else { &t.line });
        ops.push(Op::Rect { x, y, w, h: 1, color: ring });
        ops.push(Op::Rect { x, y: y + h - 1, w, h: 1, color: ring });
        ops.push(Op::Rect { x, y, w: 1, h, color: ring });
        ops.push(Op::Rect { x: x + w - 1, y, w: 1, h, color: ring });
        let ty = y + (h - m.linespace) / 2;
        let tx = x + 2 - self.x_of(self.left);
        ops.push(Op::Clip(Some((x + 2, y + 2, w - 4, h - 4))));
        if let Some((a, b)) = self.sel {
            let (sx, ex) = (tx + self.x_of(a), tx + self.x_of(b));
            ops.push(Op::Rect { x: sx, y: ty, w: ex - sx, h: m.linespace, color: rgb(&t.acc) });
        }
        let s = String::from_utf16_lossy(&self.text);
        let lay = gdi::layout(self.font, &s, 0);
        ops.push(Op::Text { x: tx, y: ty, lay, font: self.font, color: rgb(&t.ink), right: false });
        if let Some((a, b)) = self.sel {
            let part = String::from_utf16_lossy(&self.text[a..b]);
            let lay = gdi::layout(self.font, &part, 0);
            ops.push(Op::Text { x: tx + self.x_of(a), y: ty, lay, font: self.font, color: rgb(&t.onacc), right: false });
        }
        // the insertion cursor: 2 pixels wide, blinking 600 ms on / 300 ms off while focused
        if focus && ((now - self.blink_t0) * 1000.0) as i64 % 900 < 600 {
            let cx = tx + self.x_of(self.pos) - 1;
            ops.push(Op::Rect { x: cx, y: ty, w: 2, h: m.linespace, color: rgb(&t.ink) });
        }
        ops.push(Op::Clip(None));
    }
}
