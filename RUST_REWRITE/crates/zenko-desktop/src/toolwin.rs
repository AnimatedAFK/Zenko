//! zenko.py's ZkToolWindow family (zenko17): the shared look of the study-tool windows (the night header with its
//! kanji, title and subtitle over a gold rule; tt_btn buttons; a Text box with its slim ZkScrollbar; md_insert) and
//! the window's size and place on the screen.

use crate::hubwin::canvas_of;
use crate::pagekit::{Faces, tt_btn};
use crate::theme::Theme;
use crate::tktext::{Text, Wrap};
use crate::ui::{Anchor, Fill, Font, Fonts, Kind, Pack, Place, Side, Ui};
use zenko_art::palette::mixhex;

/// "%s · %s" % (TITLE, APP_NAME)
pub fn win_title(title: &str) -> String {
    format!("{title} \u{b7} Zenko")
}

/// geometry for SIZE on a sw x sh screen: (w, h, x, y)
pub fn geometry(size: (i32, i32), sw: i32, sh: i32) -> (i32, i32, i32, i32) {
    let (w, h) = (size.0.min(sw - 60), size.1.min(sh - 90));
    (w, h, 0.max((sw - w).div_euclid(2) + 40), 0.max((sh - h).div_euclid(3)))
}

/// the fonts a tool window uses: FONT_UI, _serif_font, _mincho_font
#[derive(Clone, Debug)]
pub struct ToolFonts {
    pub ui: String,
    pub serif: String,
    pub jp: String,
}

/// the pieces every tool window shares
#[derive(Clone, Debug)]
pub struct Tool {
    pub th: Theme,
    pub f: ToolFonts,
    /// (text box, its scrollbar canvas)
    pub bars: Vec<(usize, usize)>,
}

pub fn lab(ui: &mut Ui, parent: usize, text: &str, bg: &str, fg: &str, font: Font) -> usize {
    ui.label(parent, text, bg, fg, font)
}

/// a Label with anchor / justify="left" / wraplength
pub fn lab_w(ui: &mut Ui, parent: usize, text: &str, bg: &str, fg: &str, font: Font, anchor: Anchor, left: bool, wrap: i32) -> usize {
    let l = ui.label(parent, text, bg, fg, font);
    if let Kind::Label { anchor: a, left: lf, wrap: w, .. } = &mut ui.ws[l].kind {
        (*a, *lf, *w) = (anchor, left, wrap);
    }
    l
}

pub fn set_text(ui: &mut Ui, l: usize, s: &str) {
    if let Kind::Label { text, .. } = &mut ui.ws[l].kind {
        *text = s.to_string();
    }
}

pub fn set_fg(ui: &mut Ui, l: usize, c: &str) {
    if let Kind::Label { fg, .. } = &mut ui.ws[l].kind {
        *fg = c.to_string();
    }
}

pub fn set_wrap(ui: &mut Ui, l: usize, v: i32) {
    if let Kind::Label { wrap, .. } = &mut ui.ws[l].kind {
        *wrap = v;
    }
}

pub fn set_btn_text(ui: &mut Ui, b: usize, s: &str) {
    if let Kind::Button { text, sized, .. } = &mut ui.ws[b].kind {
        if sized.is_none() {
            *sized = Some(text.clone());
        }
        *text = s.to_string();
    }
}

impl Tool {
    pub fn new(th: &Theme, f: &ToolFonts) -> Tool {
        Tool { th: th.clone(), f: f.clone(), bars: vec![] }
    }

    pub fn fu(&self, size: i32, bold: bool) -> Font {
        Font::new(&self.f.ui, size, bold)
    }
    pub fn fi(&self, family: &str, size: i32, bold: bool, italic: bool) -> Font {
        Font { family: family.into(), size, bold, italic }
    }

    /// header(kanji, title, sub)
    pub fn header(&self, ui: &mut Ui, win: usize, kanji: &str, title: &str, sub: &str) -> usize {
        let th = &self.th;
        let hdr = ui.frame(win, &th.night);
        ui.pack(hdr, Pack::default().fill(Fill::X));
        let k = ui.label(hdr, kanji, &th.night, &th.gold, Font::new(&self.f.jp, 20, false));
        ui.pack(k, Pack::default().side(Side::Left).padx(18, 10).pady(8, 8));
        let col = ui.frame(hdr, &th.night);
        ui.pack(col, Pack::default().side(Side::Left).pady(6, 6));
        let t = ui.label(col, title, &th.night, "#f3ead6", Font::new(&self.f.serif, 15, false));
        ui.pack(t, Pack::default().anchor(Anchor::W));
        if !sub.is_empty() {
            let s = ui.label(col, sub, &th.night, &mixhex(&th.night, &th.ink, 0.6), self.fi(&self.f.serif.clone(), 9, false, true));
            ui.pack(s, Pack::default().anchor(Anchor::W));
        }
        let rule = ui.frame(win, &th.gold);
        ui.ws[rule].opt_h = 1;
        ui.pack(rule, Pack::default().fill(Fill::X));
        hdr
    }

    /// btn(parent, text, cmd, accent, size)
    pub fn btn(&self, ui: &mut Ui, parent: usize, text: &str, cmd: &str, accent: bool, size: i32) -> usize {
        let faces = Faces { ui: self.f.ui.clone(), title: String::new() };
        let bg = if accent { self.th.start.clone() } else { self.th.field.clone() };
        tt_btn(ui, parent, text, cmd, Some(&bg), None, size, 10, 2, &self.th, &faces)
    }

    /// text_box(parent, height, size) with zk_text_scroll
    pub fn text_box(&mut self, ui: &mut Ui, parent: usize, height: i32, size: i32) -> usize {
        let bg = mixhex(&self.th.card, &self.th.field, 0.5);
        let mut t = Text::new(Font::new(&self.f.ui, size, false), &self.th.ink, &bg);
        (t.width_ch, t.height_ln, t.wrap, t.hl, t.bd) = (80, height, Wrap::Word, 0, 1);
        (t.padx, t.pady) = (12, 10);
        let id = ui.text(parent, t, "");
        self.scroll(ui, id);
        id
    }

    /// zk_text_scroll(text): a ZkScrollbar placed inside the Text's right edge
    pub fn scroll(&mut self, ui: &mut Ui, text: usize) {
        let bg = match &ui.ws[text].kind {
            Kind::Text { t, .. } => t.bg.clone(),
            _ => ui.ws[text].bg.clone(),
        };
        let sb = ui.canvas(text, &bg, 9, 40);
        canvas_of(ui, sb);
        ui.ws[sb].place = Some(Place { relx: 1.0, x: -1, y: 2, relh: Some(1.0), h: Some(-4), ne: true, ..Default::default() });
        self.bars.push((text, sb));
    }

    /// show_md(t, text, size)
    pub fn show_md(&self, ui: &mut Ui, text: usize, s: &str, size: i32) {
        let th = self.th.clone();
        let (fu, serif) = (self.f.ui.clone(), self.f.serif.clone());
        let t = ui.text_mut(text);
        t.delete_all();
        crate::chatpage::md_tags(t, size, &th.ink, &th, &fu, &serif, "Consolas");
        for (sg, tg) in crate::mdtext::md_segments(s, &[]) {
            let tags: Vec<&str> = tg.iter().map(|x| x.as_str()).collect();
            t.insert_end(&sg, &tags);
        }
    }

    /// the scrollbars drawn for the text boxes' views now
    pub fn paint_bars(&self, ui: &mut Ui, fonts: &dyn Fonts) {
        for &(t, sb) in &self.bars {
            if t >= ui.ws.len() || sb >= ui.ws.len() {
                continue;
            }
            let (lo, hi) = match &ui.ws[t].kind {
                Kind::Text { t, .. } => t.yview(fonts),
                _ => continue,
            };
            crate::chatpage::scrollbar_draw(ui, sb, lo, hi, false, &self.th);
        }
    }
}

/// an Entry's text set (a StringVar .set)
pub fn set_entry(ui: &mut Ui, e: usize, s: &str) {
    if let Kind::Entry { e: et, .. } = &mut ui.ws[e].kind {
        et.text = s.encode_utf16().collect();
        et.pos = et.text.len();
        et.anchor = et.pos;
        et.sel = None;
    }
}

pub fn entry_text(ui: &Ui, e: usize) -> String {
    match &ui.ws[e].kind {
        Kind::Entry { e: et, .. } => String::from_utf16_lossy(&et.text),
        _ => String::new(),
    }
}
