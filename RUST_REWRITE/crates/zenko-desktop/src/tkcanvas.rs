//! A Tk canvas as zenko.py uses it: items in stacking order with tags and Tk's per-type defaults, created, found,
//! raised, lowered, re-coloured and deleted the way tkCanvas.c does (raise/lower keep the moved items' order).

use crate::ui::Font;
use std::rc::Rc;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CKind {
    Rect,
    Oval,
    Line,
    Poly,
    Text,
    Image,
    /// create_arc: `start` / `extent` in degrees (normalised as ConfigureArc does), `style` arc / pieslice / chord
    Arc,
}

impl CKind {
    pub fn tk_name(self) -> &'static str {
        match self {
            CKind::Rect => "rectangle",
            CKind::Oval => "oval",
            CKind::Line => "line",
            CKind::Poly => "polygon",
            CKind::Text => "text",
            CKind::Image => "image",
            CKind::Arc => "arc",
        }
    }
}

/// a Tk photo image: RGBA pixels (alpha 0 = see-through)
#[derive(Clone, Debug, PartialEq)]
pub struct Photo {
    pub w: usize,
    pub h: usize,
    pub px: Vec<[u8; 4]>,
}

#[derive(Clone, Debug)]
pub struct CItem {
    pub id: u32,
    pub kind: CKind,
    pub coords: Vec<f64>,
    pub tags: Vec<String>,
    pub fill: String,
    pub outline: String,
    pub width: f64,
    pub smooth: bool,
    pub capstyle: &'static str,
    pub joinstyle: &'static str,
    pub hidden: bool,
    pub anchor: &'static str,
    pub text: String,
    pub font: Option<Font>,
    /// a text item's -width (wrap length)
    pub wrap: f64,
    pub image: Option<Rc<Photo>>,
    /// an arc's start, extent and style
    pub start: f64,
    pub extent: f64,
    pub style: &'static str,
    /// a text item's -justify (left / center / right), an item's -stipple ("" or gray50 ...), a text font's underline
    pub justify: &'static str,
    pub stipple: &'static str,
    pub underline: bool,
    /// -dash: the dash list (empty: a solid line)
    pub dash: Vec<u8>,
}

impl CItem {
    pub fn fill(&mut self, c: &str) -> &mut CItem {
        self.fill = c.into();
        self
    }
    pub fn outline(&mut self, c: &str) -> &mut CItem {
        self.outline = c.into();
        self
    }
    pub fn width(&mut self, w: f64) -> &mut CItem {
        self.width = w;
        self
    }
    pub fn smooth(&mut self) -> &mut CItem {
        self.smooth = true;
        self
    }
    pub fn cap(&mut self, c: &'static str) -> &mut CItem {
        self.capstyle = c;
        self
    }
    pub fn join(&mut self, j: &'static str) -> &mut CItem {
        self.joinstyle = j;
        self
    }
    pub fn anchor(&mut self, a: &'static str) -> &mut CItem {
        self.anchor = a;
        self
    }
    pub fn font(&mut self, f: Font) -> &mut CItem {
        self.font = Some(f);
        self
    }
    /// -width: a screen distance, which Tk keeps as whole pixels (Tk_GetPixels rounds)
    pub fn wrap(&mut self, w: f64) -> &mut CItem {
        self.wrap = (w + if w < 0.0 { -0.5 } else { 0.5 }).trunc();
        self
    }
    pub fn hide(&mut self, h: bool) -> &mut CItem {
        self.hidden = h;
        self
    }
    pub fn justify(&mut self, j: &'static str) -> &mut CItem {
        self.justify = j;
        self
    }
    pub fn stipple(&mut self, st: &'static str) -> &mut CItem {
        self.stipple = st;
        self
    }
    pub fn dash(&mut self, d: &[u8]) -> &mut CItem {
        self.dash = d.to_vec();
        self
    }
    pub fn underline(&mut self) -> &mut CItem {
        self.underline = true;
        self
    }
    pub fn tags(&mut self, t: &[&str]) -> &mut CItem {
        self.tags = t.iter().map(|s| s.to_string()).collect();
        self
    }
    pub fn has(&self, tag: &str) -> bool {
        tag == "all" || self.tags.iter().any(|t| t == tag) || tag.parse::<u32>().ok() == Some(self.id)
    }
}

#[derive(Clone, Debug, Default)]
pub struct Canvas {
    pub items: Vec<CItem>,
    next: u32,
}

impl Canvas {
    fn add(&mut self, kind: CKind, coords: Vec<f64>) -> &mut CItem {
        self.next += 1;
        // Tk's defaults for each item type
        let (fill, outline, join) = match kind {
            CKind::Rect | CKind::Oval => ("", "black", "round"),
            CKind::Line => ("black", "", "round"),
            CKind::Poly => ("black", "", "round"),
            CKind::Text => ("black", "", "round"),
            CKind::Image => ("", "", "round"),
            CKind::Arc => ("", "black", "round"),
        };
        self.items.push(CItem { id: self.next, kind, coords, tags: vec![], fill: fill.into(), outline: outline.into(),
                                width: 1.0, smooth: false, capstyle: "butt", joinstyle: join, hidden: false,
                                anchor: "center", text: String::new(), font: None, wrap: 0.0, image: None,
                                start: 0.0, extent: 90.0, style: "pieslice", justify: "left", stipple: "",
                                underline: false, dash: vec![] });
        self.items.last_mut().unwrap()
    }
    pub fn rect(&mut self, x1: f64, y1: f64, x2: f64, y2: f64) -> &mut CItem {
        self.add(CKind::Rect, vec![x1, y1, x2, y2])
    }
    /// create_arc(x1, y1, x2, y2, start, extent, style); ConfigureArc keeps start in [0, 360) and extent in (-360, 360)
    pub fn arc(&mut self, x1: f64, y1: f64, x2: f64, y2: f64, start: f64, extent: f64, style: &'static str) -> &mut CItem {
        let mut st = start - (start / 360.0).trunc() * 360.0;
        if st < 0.0 {
            st += 360.0;
        }
        let ex = extent - (extent / 360.0).trunc() * 360.0;
        let it = self.add(CKind::Arc, vec![x1, y1, x2, y2]);
        (it.start, it.extent, it.style) = (st, ex, style);
        it
    }
    pub fn oval(&mut self, x1: f64, y1: f64, x2: f64, y2: f64) -> &mut CItem {
        self.add(CKind::Oval, vec![x1, y1, x2, y2])
    }
    pub fn line(&mut self, pts: Vec<f64>) -> &mut CItem {
        self.add(CKind::Line, pts)
    }
    pub fn poly(&mut self, pts: Vec<f64>) -> &mut CItem {
        self.add(CKind::Poly, pts)
    }
    pub fn text(&mut self, x: f64, y: f64, text: &str) -> &mut CItem {
        let it = self.add(CKind::Text, vec![x, y]);
        it.text = text.into();
        it
    }
    pub fn image(&mut self, x: f64, y: f64, img: Rc<Photo>) -> &mut CItem {
        let it = self.add(CKind::Image, vec![x, y]);
        it.image = Some(img);
        it
    }
    pub fn last_id(&self) -> u32 {
        self.next
    }
    pub fn find_withtag(&self, tag: &str) -> Vec<u32> {
        self.items.iter().filter(|i| i.has(tag)).map(|i| i.id).collect()
    }
    pub fn get(&mut self, id: u32) -> Option<&mut CItem> {
        self.items.iter_mut().find(|i| i.id == id)
    }
    pub fn delete(&mut self, tag: &str) {
        self.items.retain(|i| !i.has(tag));
    }
    /// raise: the tagged items, in their order, go above everything (or just above the top `above` item)
    pub fn tag_raise(&mut self, tag: &str) {
        let (moved, rest): (Vec<CItem>, Vec<CItem>) = self.items.drain(..).partition(|i| i.has(tag));
        self.items = rest;
        self.items.extend(moved);
    }
    /// tag_raise(tag, above): the tagged items, in their order, go just above the topmost `above` item
    pub fn tag_raise_above(&mut self, tag: &str, above: &str) {
        let (moved, rest): (Vec<CItem>, Vec<CItem>) = self.items.drain(..).partition(|i| i.has(tag) && !i.has(above));
        let at = rest.iter().rposition(|i| i.has(above)).map(|p| p + 1).unwrap_or(rest.len());
        let mut v = rest;
        let tail = v.split_off(at);
        v.extend(moved);
        v.extend(tail);
        self.items = v;
    }
    /// move(tag, dx, dy)
    pub fn move_items(&mut self, tag: &str, dx: f64, dy: f64) {
        for it in self.items.iter_mut().filter(|i| i.has(tag)) {
            for (k, c) in it.coords.iter_mut().enumerate() {
                *c += if k % 2 == 0 { dx } else { dy };
            }
        }
    }
    /// lower: the tagged items, in their order, go below everything, or just below the lowest `below` item
    pub fn tag_lower(&mut self, tag: &str, below: Option<&str>) {
        let (moved, rest): (Vec<CItem>, Vec<CItem>) = self.items.drain(..).partition(|i| i.has(tag));
        let at = match below {
            Some(b) => rest.iter().position(|i| i.has(b)).unwrap_or(rest.len()),
            None => 0,
        };
        let mut v = rest;
        let tail = v.split_off(at);
        v.extend(moved);
        v.extend(tail);
        self.items = v;
    }
}
