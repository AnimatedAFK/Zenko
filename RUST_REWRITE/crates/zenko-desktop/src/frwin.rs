//! zenko.py's FrenchWindow (辞書 · French ⇄ English): the look-up box and direction pills, a status line, the answer
//! box (your vocab matches, the conjugation, then the online translation), the Explain / Examples / flashcard
//! buttons and the row of recent look-ups.

use crate::mdtext::clip;
use crate::notes::{fr_vocab_search, looks_french};
use crate::theme::Theme;
use crate::toolwin::{Tool, ToolFonts, entry_text, lab_w, set_text};
use crate::ui::{Anchor, Fill, Font, Fonts, Kind, Pack, Side, Ui};


use zenko_art::palette::mixhex;

pub const SIZE: (i32, i32) = (820, 680);
pub const TITLE: &str = "French dictionary";

const INTRO: &str = "Type a word or a whole sentence, in French or English.\n\n\
- **manger** shows its meaning, conjugation and your vocab matches\n\
- **I would like a coffee** translates it\n\
- *Explain the grammar* asks Zenko (gender, agreement, tenses) at ab initio level";

#[derive(Clone, Debug, Default)]
pub struct Last {
    pub q: String,
    pub fr_in: bool,
    pub parts: Vec<String>,
    pub tr: String,
}

pub struct FrWin {
    pub t: Tool,
    pub q: usize,
    pub radios: Vec<(usize, &'static str)>,
    pub dir: &'static str,
    pub status: usize,
    pub out: usize,
    pub hist_row: usize,
    /// the history labels: (label, the look-up it repeats)
    pub hist_labels: Vec<(usize, String)>,
    pub last: Option<Last>,
    /// a translation is on its way for this look-up
    pub pending: Option<String>,
}

fn ends_verb(s: &str) -> bool {
    s.ends_with("er") || s.ends_with("ir") || s.ends_with("re")
}

impl FrWin {
    pub fn build(ui: &mut Ui, th: &Theme, f: &ToolFonts, hist: &[String]) -> FrWin {
        let t = Tool::new(th, f);
        let win = 0;
        ui.ws[win].bg = th.paper.clone();
        t.header(ui, win, "\u{8f9e}\u{66f8}", "Dictionnaire \u{b7} French \u{21c4} English",
                 "Your ab initio vocab, conjugations and a free online translator. Zenko explains the grammar.");
        let bg = th.paper.clone();
        let top = ui.frame(win, &bg);
        ui.pack(top, Pack::default().fill(Fill::X).padx(16, 16).pady(12, 4));
        let faces = crate::pagekit::Faces { ui: f.ui.clone(), title: String::new() };
        let q = crate::pagekit::tt_entry(ui, top, "", 34, th, &faces);
        ui.pack(q, Pack::default().side(Side::Left).fill(Fill::X).expand());
        ui.focus = Some(q);
        let b = t.btn(ui, top, "Look up", "fr_look", true, 9);
        ui.pack(b, Pack::default().side(Side::Left).padx(8, 0));
        let dr = ui.frame(win, &bg);
        ui.pack(dr, Pack::default().fill(Fill::X).padx(16, 16));
        let mut radios = vec![];
        for (val, l) in [("auto", "Guess"), ("en", "English \u{2192} French"), ("fr", "French \u{2192} English")] {
            let r = ui.check(dr, l, &th.field, &th.ink, Font::new(&f.ui, 9, true), val == "auto", &mixhex(&th.field, &th.gold, 0.35),
                             &format!("fr_dir:{val}"), 1, 0);
            if let Kind::Check { radio, noind, .. } = &mut ui.ws[r].kind {
                (*radio, *noind) = (true, true);
            }
            (ui.ws[r].padx, ui.ws[r].pady) = (10, 3);
            ui.pack(r, Pack::default().side(Side::Left).padx(0, 6).pady(4, 4));
            radios.push((r, val));
        }
        let status = lab_w(ui, win, "", &bg, &th.grey, Font::new(&f.ui, 8, false), Anchor::W, false, 0);
        ui.pack(status, Pack::default().fill(Fill::X).padx(16, 16));
        let mut fw = FrWin { t, q, radios, dir: "auto", status, out: 0, hist_row: 0, hist_labels: vec![], last: None, pending: None };
        fw.out = fw.t.text_box(ui, win, 14, 10);
        ui.pack(fw.out, Pack::default().fill(Fill::Both).expand().padx(16, 16).pady(4, 6));
        let br = ui.frame(win, &bg);
        ui.pack(br, Pack::default().fill(Fill::X).padx(16, 16).pady(0, 6));
        let b = fw.t.btn(ui, br, "Explain the grammar", "fr_explain", false, 9);
        ui.pack(b, Pack::default().side(Side::Left));
        let b = fw.t.btn(ui, br, "Example sentences", "fr_examples", false, 9);
        ui.pack(b, Pack::default().side(Side::Left).padx(6, 6));
        let b = fw.t.btn(ui, br, "Add to my flashcards", "fr_card", false, 9);
        ui.pack(b, Pack::default().side(Side::Left));
        fw.hist_row = ui.frame(win, &bg);
        ui.pack(fw.hist_row, Pack::default().fill(Fill::X).padx(16, 16).pady(0, 12));
        let out = fw.out;
        fw.t.show_md(ui, out, INTRO, 10);
        fw.draw_hist(ui, hist);
        fw
    }

    pub fn draw_hist(&mut self, ui: &mut Ui, hist: &[String]) {
        for k in ui.ws[self.hist_row].kids.clone() {
            ui.destroy(k);
        }
        self.hist_labels.clear();
        let th = self.t.th.clone();
        if !hist.is_empty() {
            let l = ui.label(self.hist_row, "Recent:", &th.paper, &th.grey, self.t.fu(8, false));
            ui.pack(l, Pack::default().side(Side::Left));
        }
        let n = hist.len();
        for h in hist[n.saturating_sub(8)..].iter().rev() {
            let l = ui.label(self.hist_row, &clip(h, 18), &th.field, &th.ink, self.t.fu(8, false));
            ui.ws[l].padx = 6;
            ui.ws[l].hand = true;
            ui.pack(l, Pack::default().side(Side::Left).padx(2, 2));
            self.hist_labels.push((l, h.clone()));
        }
    }

    pub fn set_dir(&mut self, ui: &mut Ui, d: &str) {
        for &(r, v) in &self.radios {
            if let Kind::Check { on, .. } = &mut ui.ws[r].kind {
                *on = v == d;
            }
            if v == d {
                self.dir = v;
            }
        }
    }

    pub fn query(&self, ui: &Ui) -> String {
        entry_text(ui, self.q)
    }

    /// look(): the vocab and conjugation shown at once; `hist` is life fr_hist (updated here). Returns the job for the
    /// translator: (q, dir, fr_in), or None for an empty box.
    pub fn look(&mut self, ui: &mut Ui, hist: &mut Vec<String>) -> Option<(String, String, bool)> {
        let q = self.query(ui).trim().to_string();
        if q.is_empty() {
            return None;
        }
        hist.retain(|h| *h != q);
        hist.push(q.clone());
        let n = hist.len();
        if n > 30 {
            hist.drain(..n - 30);
        }
        self.draw_hist(ui, &hist.clone());
        let d = self.dir;
        let fr_in = d == "fr" || (d == "auto" && looks_french(&q));
        let mut parts = vec![];
        let hits = fr_vocab_search(&q, 12);
        let word = q.split_whitespace().count() <= 3;
        if !hits.is_empty() {
            let rows: Vec<String> = hits.iter().take(8).map(|(fr, en, th)| format!("- **{fr}**: {en}  *({th})*")).collect();
            parts.push(format!("### In your vocab list\n{}", rows.join("\n")));
        }
        let mut conj = String::new();
        if word {
            let v = q.to_lowercase().trim().to_string();
            let mut cand: Vec<String> = if d != "en" { vec![v] } else { vec![] };
            cand.extend(hits.iter().map(|h| h.0.to_lowercase()).filter(|f| ends_verb(f)).take(1));
            let irreg = &crate::routes17::routes_data()["FR_IRREG"];
            for c in cand {
                if ends_verb(&c) || irreg.get(&c).is_some() {
                    conj = crate::study::conjugate_fr(&c).unwrap_or_default();
                }
                if !conj.is_empty() {
                    break;
                }
            }
        }
        if !conj.is_empty() {
            let lines: Vec<&str> = conj.split('\n').collect();
            let rest: Vec<String> = lines[1..].iter().map(|l| format!("- {l}")).collect();
            parts.push(format!("### Conjugation\n**{}**\n{}", lines[0], rest.join("\n")));
        }
        self.last = Some(Last { q: q.clone(), fr_in, parts: parts.clone(), tr: String::new() });
        let body = parts.join("\n\n");
        let out = self.out;
        self.t.show_md(ui, out, if body.is_empty() { "Translating..." } else { &body }, 10);
        set_text(ui, self.status, "Translating online...");
        self.pending = Some(q.clone());
        Some((q, d.to_string(), fr_in))
    }

    /// the translator's answer: Ok((translation, service)) or Err (offline); `fr_in` the job's final guess
    pub fn look_done(&mut self, ui: &mut Ui, q: &str, res: Result<(String, String), String>, fr_in: bool) {
        if self.pending.as_deref() != Some(q) {
            // (an older look-up: zenko.py still shows it, the newest one's answer comes after)
        }
        let Some(last) = self.last.as_mut() else { return };
        let parts = last.parts.clone();
        match res {
            Err(_) => {
                set_text(ui, self.status, "The online translator isn't reachable (offline, or blocked here). Your vocab and \
                                           conjugations still work, and 'Explain' uses Zenko's brain.");
                if parts.is_empty() {
                    let out = self.out;
                    self.t.show_md(ui, out, "I couldn't translate that offline. Try *Explain the grammar* (uses your brain).", 10);
                }
            }
            Ok((tr, service)) => {
                if fr_in {
                    last.fr_in = true;
                }
                last.tr = tr.clone();
                let fr = last.fr_in;
                set_text(ui, self.status, &format!("Translated with {service}"));
                let mut s = format!("### {}\n**{tr}**\n\n*{q}*", if fr { "French \u{2192} English" } else { "English \u{2192} French" });
                if !parts.is_empty() {
                    s += &format!("\n\n{}", parts.join("\n\n"));
                }
                let out = self.out;
                self.t.show_md(ui, out, &s, 10);
            }
        }
    }

    /// explain(examples): the question for nb_ask("french", ..., "explain"), or None for an empty box
    pub fn explain(&mut self, ui: &mut Ui, examples: bool) -> Option<String> {
        let q = self.query(ui).trim().to_string();
        if q.is_empty() {
            return None;
        }
        set_text(ui, self.status, "Asking Zenko...");
        Some(if examples {
            format!("Give 4 short example sentences using '{q}' at IB French ab initio level, each with its English \
                     translation. Mark the gender of nouns (le/la) and point out any agreement.")
        } else {
            format!("Explain '{q}' for an IB French ab initio student: what it means, its gender and plural if it's a noun, \
                     which tense/mood and why if it's a verb form or sentence, any agreement, and one common mistake to avoid. \
                     Keep it short.")
        })
    }

    pub fn explained(&mut self, ui: &mut Ui, txt: &str) {
        set_text(ui, self.status, "");
        let out = self.out;
        self.t.show_md(ui, out, txt, 10);
    }

    /// add_card(): Some((question, answer)) for card_add, or the status line says why not
    pub fn add_card(&mut self, ui: &mut Ui) -> Option<(String, String)> {
        let l = match &self.last {
            Some(l) if !l.tr.is_empty() => l.clone(),
            _ => {
                set_text(ui, self.status, "Look a word up first (with the translator), then add it.");
                return None;
            }
        };
        let (fr, en) = if l.fr_in { (l.q.clone(), l.tr.clone()) } else { (l.tr.clone(), l.q.clone()) };
        Some((format!("{en} (in French)"), fr))
    }

    pub fn card_added(&mut self, ui: &mut Ui, msg: &str) {
        set_text(ui, self.status, &clip(&crate::mdtext::md_plain(msg), 120));
    }

    pub fn fit(&mut self, ui: &mut Ui, fonts: &dyn Fonts, w: i32, h: i32) {
        ui.layout_at(fonts, w, h);
        self.t.paint_bars(ui, fonts);
    }
}

/// the translator job: guess the language with "auto" when unsure. `tr(q, target, source)` -> (text, _, service).
/// Returns the answer and whether it turned out to be French.
pub fn look_job(q: &str, d: &str, fr_in: bool, tr: &mut dyn FnMut(&str, &str, &str) -> Result<(String, String, String), String>)
                -> (Result<(String, String), String>, bool) {
    let norm = |s: &str| s.trim().to_lowercase().trim_matches(['.', '!', '?']).to_string();
    if d == "auto" && !fr_in {
        match tr(q, "en", "auto") {
            Ok(r) => {
                if norm(&r.0) != q.to_lowercase().trim_matches(['.', '!', '?']) {
                    return (Ok((r.0, r.2)), true);
                }
            }
            Err(e) => return (Err(e), false),
        }
    }
    let (src, tgt) = if fr_in { ("fr", "en") } else { ("en", "fr") };
    (tr(q, tgt, src).map(|r| (r.0, r.2)), false)
}

