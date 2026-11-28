//! The hub's About you page (zenko.py hub_build_you): the profile fields (entries and Text boxes) with Save, how it's
//! used, and what she has learned with the two learning switches and Forget everything.

use crate::hubwin::{canvas_of, hub_title_draw};
use crate::pagekit::{Faces, Scroller, WrapState, hub_card, hub_scroller, label_w, tt_btn, tt_entry, zk_fonts, zk_wrap};
use crate::theme::Theme;
use crate::tkcanvas::Photo;
use crate::tktext::{Text, Wrap};
use crate::ui::{Anchor, Fill, Font, Fonts, Kind, Pack, Side, Ui};
use std::rc::Rc;

pub const TITLE: &str = "About you";
pub const SUB: &str = "Tell the fox about yourself. It uses this to talk to you and give better help.";

/// (config key, label, width, lines)
pub const FIELDS: [(&str, &str, i32, i32); 5] = [
    ("user_name", "Your name (the fox calls you by your first name)", 30, 1),
    ("user_role", "What do you do? (e.g. 'pathology professor', 'medical student')", 50, 1),
    ("user_studying", "What are you working on or studying these days?", 60, 2),
    ("user_goals", "Your goals (exams, deadlines, habits...)", 60, 3),
    ("user_notes", "Anything else the fox should know (how you like to be spoken to, your routine...)", 60, 5),
];

pub const USED: &str = "Your name appears in greetings, nudges and the fox's voice. Everything here is sent only to the offline \
smart brain on this PC (Ollama), so its answers, note summaries and screen help fit you. It's stored in Zenko's settings on \
this PC and never uploaded.";
pub const TEACH: &str = "Teach her in Chat:  learn: good morning = Ohayo! Let's get to work.    Undo with:  forget good morning\n\
Shorthand:  when I say sched I mean timetable.    Ask her:  what's my shorthand?   what did you look up?";

pub struct YouPage {
    pub pg: usize,
    pub title: usize,
    pub sc: Scroller,
    /// the five fields: (key, entry or text widget, is a Text)
    pub fields: Vec<(&'static str, usize, bool)>,
    pub status: usize,
    pub info: usize,
    pub learn: usize,
    pub style: usize,
    pub wrap: WrapState,
    seig: Option<(String, Rc<Photo>)>,
}

fn wrapped(ui: &mut Ui, l: usize, wl: i32) {
    if let Kind::Label { wrap, left, .. } = &mut ui.ws[l].kind {
        (*wrap, *left) = (wl, true);
    }
}

impl YouPage {
    /// hub_build_you(pg): `cfg` is the app's config (the field values and the two switches), `learned` learned_summary()
    pub fn build(ui: &mut Ui, pg: usize, th: &Theme, f: &Faces, cm10: i32, cfg: &serde_json::Value, learned: &str) -> YouPage {
        let title = ui.canvas(pg, &th.banner, cm10, 96);
        canvas_of(ui, title);
        ui.pack(title, Pack::default().fill(Fill::X));
        let sc = hub_scroller(ui, pg, th, cm10);
        let body = ui.frame(sc.inner, &th.paper);
        ui.pack(body, Pack::default().fill(Fill::X).padx(24, 24).pady(14, 14));
        let bg = th.card.clone();
        let c = hub_card(ui, body, Some("Profile"), "round", th, f);
        ui.pack(c, Pack::default().fill(Fill::X));
        let status = label_w(ui, c, "", &bg, &th.accd, &f.ui, 9, true);
        let mut fields = vec![];
        for (key, lab, width, lines) in FIELDS {
            let l = label_w(ui, c, lab, &bg, &th.grey, &f.ui, 9, false);
            ui.pack(l, Pack::default().fill(Fill::X).padx(16, 16).pady(8, 2));
            let v = cfg.get(key).and_then(|v| v.as_str()).unwrap_or("");
            if lines == 1 {
                let e = tt_entry(ui, c, v, width, th, f);
                ui.pack(e, Pack::default().anchor(Anchor::W).padx(16, 16));
                fields.push((key, e, false));
            } else {
                let mut t = Text::new(Font::new(&f.ui, 11, false), &th.ink, &th.field);
                (t.width_ch, t.height_ln, t.wrap) = (width, lines, Wrap::Word);
                (t.padx, t.pady, t.hl) = (6, 4, 1);
                t.insert_end(v, &[]);
                t.edit = Some(0);
                let id = ui.text(c, t, &th.line);
                ui.pack(id, Pack::default().fill(Fill::X).padx(16, 16));
                fields.push((key, id, true));
            }
        }
        let r = ui.frame(c, &bg);
        ui.pack(r, Pack::default().fill(Fill::X).padx(16, 16).pady(12, 4));
        let b = tt_btn(ui, r, "Save", "you_save", Some(&th.acc), Some(&th.onacc), 11, 18, 3, th, f);
        ui.pack(b, Pack::default().side(Side::Left));
        ui.pack(status, Pack::default().fill(Fill::X).padx(16, 16).pady(0, 12));
        let c2 = hub_card(ui, body, Some("How it's used"), "round", th, f);
        ui.pack(c2, Pack::default().fill(Fill::X).pady(12, 0));
        let l = label_w(ui, c2, USED, &bg, &th.ink, &f.ui, 10, false);
        wrapped(ui, l, 700);
        ui.pack(l, Pack::default().fill(Fill::X).padx(16, 16).pady(0, 14));
        let c3 = hub_card(ui, body, Some("What Zenko has learned"), "round", th, f);
        ui.pack(c3, Pack::default().fill(Fill::X).pady(12, 0));
        let on = |k: &str| cfg.get(k).map(|v| v.as_bool().unwrap_or(!v.is_null())).unwrap_or(true);
        let learn = ui.check(c3, "Let Zenko learn my habits (kept only on this PC)", &bg, &th.ink, Font::new(&f.ui, 10, false),
                             on("learn_habits"), &th.field, "you_learn", 0, 0);
        ui.pack(learn, Pack::default().fill(Fill::X).padx(14, 14).pady(0, 2));
        let style = ui.check(c3, "Let Zenko learn my shorthand, e.g. sched = timetable (kept only on this PC)", &bg, &th.ink,
                             Font::new(&f.ui, 10, false), on("learn_style"), &th.field, "you_style", 0, 0);
        ui.pack(style, Pack::default().fill(Fill::X).padx(14, 14).pady(0, 6));
        let info = label_w(ui, c3, learned, &bg, &th.ink, &f.ui, 10, false);
        wrapped(ui, info, 700);
        ui.pack(info, Pack::default().fill(Fill::X).padx(16, 16));
        let tip = label_w(ui, c3, TEACH, &bg, &th.grey, &f.ui, 9, false);
        wrapped(ui, tip, 700);
        ui.pack(tip, Pack::default().fill(Fill::X).padx(16, 16).pady(6, 0));
        let r3 = ui.frame(c3, &bg);
        ui.pack(r3, Pack::default().fill(Fill::X).padx(16, 16).pady(8, 12));
        let b = tt_btn(ui, r3, "Forget everything", "you_forget", Some(&th.delbtn), None, 9, 10, 2, th, f);
        ui.pack(b, Pack::default().side(Side::Left));
        YouPage { pg, title, sc, fields, status, info, learn, style, wrap: WrapState::default(), seig: None }
    }

    /// what the fields hold now (save(): each stripped and cut to 1200 characters)
    pub fn values(&self, ui: &Ui) -> Vec<(&'static str, String)> {
        self.fields.iter().map(|&(k, id, text)| {
            let v = if text {
                match &ui.ws[id].kind {
                    Kind::Text { t, .. } => t.get(),
                    _ => String::new(),
                }
            } else {
                ui.text_of(id)
            };
            (k, v.trim().chars().take(1200).collect())
        }).collect()
    }

    pub fn set_status(&self, ui: &mut Ui, s: &str) {
        if let Kind::Label { text, .. } = &mut ui.ws[self.status].kind {
            *text = s.into();
        }
    }

    pub fn set_info(&self, ui: &mut Ui, s: &str) {
        if let Kind::Label { text, .. } = &mut ui.ws[self.info].kind {
            *text = s.into();
        }
    }

    pub fn checked(ui: &Ui, id: usize) -> bool {
        matches!(ui.ws[id].kind, Kind::Check { on: true, .. })
    }

    /// zk_fit for the page and the banner for its width; lays out at the window's size (pw, ph)
    pub fn fit(&mut self, ui: &mut Ui, fonts: &dyn Fonts, th: &Theme, f: &Faces, zk_f: f64, pw: i32, ph: i32) {
        zk_fonts(ui, fonts, self.pg, zk_f);
        ui.layout_at(fonts, pw, ph);
        let w = ui.ws[self.title].w;
        if w >= 20 {
            let mut cv = std::mem::take(canvas_of(ui, self.title));
            let hh = hub_title_draw(&mut cv, w, TITLE, SUB, th, &f.ui, &f.title, zk_f, fonts, &mut self.seig);
            *canvas_of(ui, self.title) = cv;
            if ui.ws[self.title].opt_h != hh {
                ui.ws[self.title].opt_h = hh;
                ui.layout_at(fonts, pw, ph);
            }
        }
        zk_wrap(ui, fonts, self.pg, &mut self.wrap, pw, ph);
        if self.sc.sync(ui) {
            ui.layout_at(fonts, pw, ph);
        }
    }
}
