//! The hub's "Ask Zenko" page (hub_build_chat -> FoxBuddy.open_chat(parent=pg) with Zenko's override): banner, the
//! quick chips, the chat Text with its rich-text tags (md_tags) and slim scrollbar (ZkScrollbar / zk_text_scroll), the
//! status line and the entry row (Send, Mic, Talk, Image, Screen, Mute).

use crate::hubside::zk_size;
use crate::hubwin::{canvas_of, hub_title_draw};
use crate::theme::Theme;
use crate::tkcanvas::Photo;
use crate::tktext::{TagCfg, Text, Wrap};
use crate::ui::{Fill, Font, Fonts, Kind, Pack, Place, Side, Ui};
use std::rc::Rc;
use zenko_art::palette::mixhex;

pub const CHIPS: [(&str, &str); 7] = [("Weather", "weather"), ("Time left", "time left"), ("What's next", "next"),
    ("Headlines", "news"), ("What's playing", "what's playing"), ("My notes", "notes"), ("Help", "help")];
pub const TITLE: &str = "Ask Zenko";
pub const SUB: &str = "Commands, questions, weather, news, Wikipedia, currency... typos are fine";

pub struct ChatPage {
    pub pg: usize,
    pub title: usize,
    pub text: usize,
    pub sb: usize,
    pub chips: usize,
    pub status: usize,
    pub attbar: usize,
    pub row: usize,
    pub entry: usize,
    /// every PixelButton with the size it was made with (zk_rescale)
    pub buttons: Vec<(usize, i32)>,
    pub mute: usize,
    pub sb_hover: bool,
    pub seig: Option<(String, Rc<Photo>)>,
    md_ready: bool,
}

/// md_tags(t, size, fg): the rich-text tags, once per widget
pub fn md_tags(t: &mut Text, size: i32, fg: &str, th: &Theme, ui: &str, serif: &str, mono: &str) {
    let f = |fam: &str, n: i32, bold: bool, italic: bool| Some(Font { family: fam.into(), size: n, bold, italic });
    let mut tag = |name: &str, c: TagCfg| {
        *t.tag(name) = TagCfg { name: name.into(), ..c };
    };
    let d = TagCfg::default;
    let s = |v: &str| Some(v.to_string());
    tag("md_b", TagCfg { font: f(ui, size, true, false), ..d() });
    tag("md_i", TagCfg { font: f(ui, size, false, true), ..d() });
    tag("md_bi", TagCfg { font: f(ui, size, true, true), ..d() });
    tag("md_s", TagCfg { overstrike: Some(true), ..d() });
    tag("md_h1", TagCfg { font: f(serif, size + 6, false, false), fg: s(&th.gold), spacing1: Some(8), spacing3: Some(4), ..d() });
    tag("md_h2", TagCfg { font: f(serif, size + 4, false, false), fg: s(&th.gold), spacing1: Some(6), spacing3: Some(3), ..d() });
    tag("md_h3", TagCfg { font: f(ui, size + 1, true, false), fg: s(fg), spacing1: Some(4), spacing3: Some(2), ..d() });
    tag("md_code", TagCfg { font: f(mono, size, false, false), bg: s(&th.field), fg: s(&th.gold), ..d() });
    tag("md_pre", TagCfg { font: f(mono, size - 1, false, false), bg: s(&th.field), lmargin1: Some(12), lmargin2: Some(12),
                           spacing1: Some(2), spacing3: Some(2), ..d() });
    tag("md_quote", TagCfg { fg: s(&th.grey), font: f(serif, size, false, true), lmargin1: Some(6), lmargin2: Some(20), ..d() });
    for k in 0..4 {
        tag(&format!("md_li{k}"), TagCfg { lmargin1: Some(8 + 18 * k), lmargin2: Some(22 + 18 * k), spacing1: Some(1), ..d() });
    }
    tag("md_bullet", TagCfg { fg: s(&th.gold), ..d() });
    tag("md_num", TagCfg { fg: s(&th.gold), font: f(ui, size, true, false), ..d() });
    tag("md_table", TagCfg { font: f(mono, size - 1, false, false), lmargin1: Some(6), ..d() });
    tag("md_th", TagCfg { font: f(mono, size - 1, true, false), fg: s(&th.gold), lmargin1: Some(6), ..d() });
    tag("md_hr", TagCfg { fg: s(&th.line), ..d() });
    tag("md_link", TagCfg { fg: s(&th.start), underline: Some(true), ..d() });
    tag("md_math", TagCfg { font: f(serif, size + 1, false, true), ..d() });
    tag("md_sup", TagCfg { offset: Some(3.max(size / 2 - 1)), font: f(serif, 7.max(size - 3), false, false), ..d() });
    tag("md_sub", TagCfg { offset: Some(-(2.max(size / 3))), font: f(serif, 7.max(size - 3), false, false), ..d() });
    for k in ["md_sup", "md_sub", "md_link"] {
        t.tag_raise(k);
    }
}

/// ZkScrollbar._draw on its canvas
pub fn scrollbar_draw(ui: &mut Ui, sb: usize, lo: f64, hi: f64, hover: bool, th: &Theme) {
    let (w, h, bg) = (ui.ws[sb].w, ui.ws[sb].h, ui.ws[sb].bg.clone());
    let cv = canvas_of(ui, sb);
    *cv = Default::default();
    if w < 3 || h < 3 || hi - lo >= 0.999 {
        return;
    }
    let track = mixhex(&bg, &th.ink, 0.07);
    let thumb = mixhex(&th.gold, &bg, if hover { 0.05 } else { 0.4 });
    let l = h.max(1);
    let (mut a, mut b) = ((lo * l as f64) as i32, (hi * l as f64) as i32);
    if b - a < 18 {
        let m = (a + b).div_euclid(2);
        (a, b) = (0.max(m - 9), l.min(m + 9));
    }
    let (x1, x2) = (2.0, (w - 2) as f64);
    cv.line(vec![(w / 2) as f64, 3.0, (w / 2) as f64, (h - 3) as f64]).fill(&track).width(1.max(w - 6) as f64).cap("round");
    let r = x2 - x1;
    let (a, b) = (a as f64, b as f64);
    cv.oval(x1, a + 1.0, x2, a + 1.0 + r).fill(&thumb).outline("");
    cv.oval(x1, b - 1.0 - r, x2, b - 1.0).fill(&thumb).outline("");
    let r2 = (r as i32).div_euclid(2) as f64;
    cv.rect(x1, a + 1.0 + r2, x2, b - 1.0 - r2).fill(&thumb).outline("");
}

impl ChatPage {
    /// open_chat(parent=pg) as Zenko builds it; `cm10` is a canvas's default width (10c at the screen's dpi), `muted`
    /// is cfg voice_muted
    pub fn build(ui: &mut Ui, pg: usize, th: &Theme, font_ui: &str, muted: bool, cm10: i32) -> ChatPage {
        let title = ui.canvas(pg, &th.banner, cm10, 96);
        canvas_of(ui, title);
        ui.pack(title, Pack::default().fill(Fill::X));
        let mut t = Text::new(Font::new(font_ui, 11, false), &th.ink, &th.card);
        (t.width_ch, t.height_ln, t.wrap) = (20, 10, Wrap::Word);
        (t.padx, t.pady, t.spacing1, t.spacing3, t.hl, t.bd) = (16, 12, 2, 4, 1, 1);
        let text = ui.text(pg, t, &th.line);
        let chips = ui.frame(pg, &th.paper);
        ui.pack(chips, Pack::default().fill(Fill::X).padx(24, 24).pady(12, 0));
        let mut buttons = vec![];
        for (lab, cmd) in CHIPS {
            let b = ui.button(chips, lab, &format!("chat:{cmd}"), &th.start, &th.ink, font_ui, 9, 9, 1, true);
            ui.pack(b, Pack::default().side(Side::Left).padx(0, 6));
            buttons.push((b, 9));
        }
        ui.pack(text, Pack::default().fill(Fill::Both).expand().padx(24, 24).pady(10, 0));
        let status = ui.label(pg, "", &th.paper, &th.grey, Font { family: font_ui.into(), size: 9, bold: false, italic: true });
        ui.set_anchor(status, crate::ui::Anchor::W);
        ui.pack(status, Pack::default().fill(Fill::X).padx(24, 24));
        {
            let tx = ui.text_mut(text);
            for (name, fg, bold) in [("you", th.accd.clone(), true), ("foxname", th.gold.clone(), true), ("fox", th.ink.clone(), false)] {
                *tx.tag(name) = TagCfg { name: name.into(), fg: Some(fg), font: bold.then(|| Font::new(font_ui, 11, true)),
                                         ..Default::default() };
            }
        }
        let attbar = ui.frame(pg, &th.paper);
        let row = ui.frame(pg, &th.paper);
        ui.pack(row, Pack::default().fill(Fill::X).padx(24, 24).pady(0, 14));
        let entry = ui.entry(row, 16, Font::new(font_ui, 11, false), "", &th.field);
        ui.pack(entry, Pack::default().side(Side::Left).fill(Fill::X).expand().ipady(3));
        let mut mute = 0;
        for (i, (lab, cmd)) in [("Send", "chat_send"), ("Mic", "chat_mic"), ("Talk", "chat_talk"), ("Image", "chat_image"),
                                ("Screen", "chat_screen"), (if muted { "Unmute" } else { "Mute" }, "chat_mute")].iter().enumerate() {
            let (bg, fg, padx) = if i == 0 || (i == 5 && muted) { (&th.acc, &th.onacc, if i == 0 { 14 } else { 12 }) }
                                 else { (&th.start, &th.ink, 12) };
            let b = ui.button(row, lab, cmd, bg, fg, font_ui, 10, padx, 3, true);
            ui.pack(b, Pack::default().side(Side::Left).padx(if i == 0 { 8 } else { 6 }, 0));
            buttons.push((b, 10));
            mute = b;
        }
        // zk_text_scroll: ZkScrollbar(text, width=9) placed inside the text's right edge
        let sb = ui.canvas(text, &th.card, 9, 40);
        canvas_of(ui, sb);
        ui.ws[sb].place = Some(Place { relx: 1.0, x: -1, y: 2, relh: Some(1.0), h: Some(-4), ne: true, ..Default::default() });
        ChatPage { pg, title, text, sb, chips, status, attbar, row, entry, buttons, mute, sb_hover: false, seig: None,
                   md_ready: false }
    }

    /// zk_fit's font pass: zk_font on the Text, Label and Entry, zk_rescale on every PixelButton
    pub fn fit(&mut self, ui: &mut Ui, fonts: &dyn Fonts, zk_f: f64) {
        let n = zk_size(zk_f, 11);
        let t = ui.text_mut(self.text);
        if t.font.size != n {
            t.font.size = n;
            t.relayout(fonts);
        }
        if let Kind::Label { font, .. } = &mut ui.ws[self.status].kind {
            font.size = zk_size(zk_f, 9);
        }
        if let Kind::Entry { font, .. } = &mut ui.ws[self.entry].kind {
            font.size = zk_size(zk_f, 11);
        }
        for &(b, base) in &self.buttons {
            if let Kind::Button { font, .. } = &mut ui.ws[b].kind {
                font.size = zk_size(zk_f, base);
            }
        }
    }

    /// the banner for the canvas's width now; true when its height changed (lay out again)
    pub fn draw_title(&mut self, ui: &mut Ui, fonts: &dyn Fonts, th: &Theme, font_ui: &str, font_title: &str, zk_f: f64) -> bool {
        let w = ui.ws[self.title].w;
        if w < 20 {
            return false;
        }
        let mut cv = std::mem::take(canvas_of(ui, self.title));
        let hh = hub_title_draw(&mut cv, w, TITLE, SUB, th, font_ui, font_title, zk_f, fonts, &mut self.seig);
        *canvas_of(ui, self.title) = cv;
        let changed = ui.ws[self.title].opt_h != hh;
        ui.ws[self.title].opt_h = hh;
        changed
    }

    /// the scrollbar follows the text (yscrollcommand)
    pub fn sync_scroll(&self, ui: &mut Ui, fonts: &dyn Fonts, th: &Theme) {
        let (lo, hi) = match &ui.ws[self.text].kind {
            Kind::Text { t, .. } => t.yview(fonts),
            _ => (0.0, 1.0),
        };
        scrollbar_draw(ui, self.sb, lo.max(0.0), hi.min(1.0), self.sb_hover, th);
    }

    /// _chat_insert: "You:  " / "<fox name>:  " then the message (rich text for the fox) and a blank line, then `see end`
    #[allow(clippy::too_many_arguments)]
    pub fn insert(&mut self, ui: &mut Ui, fonts: &dyn Fonts, th: &Theme, font_ui: &str, serif: &str, mono: &str,
                  fox_name: &str, you: bool, text: &str) {
        let ready = self.md_ready;
        let t = ui.text_mut(self.text);
        if you {
            t.insert_end("You:  ", &["you"]);
            t.insert_end(&format!("{text}\n\n"), &["fox"]);
        } else {
            t.insert_end(&format!("{fox_name}:  "), &["foxname"]);
            if !ready {
                md_tags(t, 10, &th.ink, th, font_ui, serif, mono);
            }
            for (s, tg) in crate::mdtext::md_segments(text, &["fox"]) {
                let tags: Vec<&str> = tg.iter().map(|x| x.as_str()).collect();
                t.insert_end(&s, &tags);
            }
            t.insert_end("\n\n", &["fox"]);
        }
        if !you {
            self.md_ready = true;
        }
        let e = t.end_index();
        t.see(fonts, e);
    }

    /// _chat_pump's typed reply: her name goes in now, the message comes back as the chunks to type, `step`
    /// characters of a segment at a time (one every 14 ms)
    #[allow(clippy::too_many_arguments)]
    pub fn begin_typed(&mut self, ui: &mut Ui, th: &Theme, font_ui: &str, serif: &str, mono: &str, fox_name: &str,
                       text: &str) -> Vec<(String, Vec<String>)> {
        let ready = self.md_ready;
        let t = ui.text_mut(self.text);
        t.insert_end(&format!("{fox_name}:  "), &["foxname"]);
        if !ready {
            md_tags(t, 10, &th.ink, th, font_ui, serif, mono);
        }
        self.md_ready = true;
        let step = 2.max(text.chars().count() / 45);
        let mut out = vec![];
        for (sg, tg) in crate::mdtext::md_segments(text, &["fox"]) {
            let c: Vec<char> = sg.chars().collect();
            for k in (0..c.len()).step_by(step) {
                out.push((c[k..(k + step).min(c.len())].iter().collect(), tg.clone()));
            }
        }
        out
    }

    /// one typed chunk (or, with `done`, the closing blank line), then `see end`
    pub fn type_chunk(&mut self, ui: &mut Ui, fonts: &dyn Fonts, s: &str, tags: &[String]) {
        let t = ui.text_mut(self.text);
        let tg: Vec<&str> = tags.iter().map(|x| x.as_str()).collect();
        t.insert_end(s, &tg);
        let e = t.end_index();
        t.see(fonts, e);
    }

    /// the wheel / the scrollbar: "units" (3 a notch), "pages" or "moveto"
    pub fn scroll(&mut self, ui: &mut Ui, fonts: &dyn Fonts, how: &str, v: f64) {
        let t = ui.text_mut(self.text);
        match how {
            "units" => t.scroll_units(fonts, v as i32),
            "pages" => t.scroll_pages(fonts, v as i32),
            _ => t.moveto(fonts, v),
        }
    }
}
