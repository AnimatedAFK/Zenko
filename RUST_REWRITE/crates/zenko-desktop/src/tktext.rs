//! Tk 8.6's text widget (tkTextDisp.c) as far as zenko.py uses it: contents with tags, tag styles by priority (GetStyle),
//! word/char wrapping into display lines (LayoutDLine / TkTextCharLayoutProc / MeasureChars with GDI's prefix
//! measuring), the view (UpdateDisplayInfo with its bottom gap fill, TkTextSetYView for `see`, MeasureUp), `dlineinfo`,
//! `bbox`, `yview` fractions and the requested size. Indices are (line from 1, char). Tabs are measured as ordinary
//! characters (zenko.py never inserts one into a chat or a note).

use crate::ui::{Font, Fonts};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Wrap {
    None,
    Char,
    Word,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Justify {
    Left,
    Right,
    Center,
}

/// one tag's options; None = not set (a lower priority tag or the widget decides)
#[derive(Clone, Debug, Default)]
pub struct TagCfg {
    pub name: String,
    pub font: Option<Font>,
    pub fg: Option<String>,
    pub bg: Option<String>,
    pub spacing1: Option<i32>,
    pub spacing2: Option<i32>,
    pub spacing3: Option<i32>,
    pub lmargin1: Option<i32>,
    pub lmargin2: Option<i32>,
    pub rmargin: Option<i32>,
    pub offset: Option<i32>,
    pub underline: Option<bool>,
    pub overstrike: Option<bool>,
    pub wrap: Option<Wrap>,
    pub justify: Option<Justify>,
}

/// StyleValues: what a run of characters is drawn with
#[derive(Clone, Debug, PartialEq)]
pub struct Style {
    pub font: Font,
    pub fg: String,
    pub bg: Option<String>,
    pub spacing1: i32,
    pub spacing2: i32,
    pub spacing3: i32,
    pub lmargin1: i32,
    pub lmargin2: i32,
    pub rmargin: i32,
    pub offset: i32,
    pub underline: bool,
    pub overstrike: bool,
    pub wrap: Wrap,
    pub justify: Justify,
}

#[derive(Clone, Debug)]
struct Run {
    text: String,
    n: usize,
    tags: Vec<usize>,
}

/// a piece of one display line drawn in one style
#[derive(Clone, Debug)]
pub struct Chunk {
    pub start: usize,
    pub n: usize,
    /// the characters drawn (a trailing newline is not drawn)
    pub text: String,
    pub x: i32,
    pub width: i32,
    pub ascent: i32,
    pub descent: i32,
    pub style: Style,
    break_index: i32,
}

#[derive(Clone, Debug)]
pub struct DLine {
    pub line: usize,
    pub ch: usize,
    pub count: usize,
    pub y: i32,
    pub height: i32,
    pub baseline: i32,
    pub space_above: i32,
    pub length: i32,
    pub chunks: Vec<Chunk>,
}

#[derive(Clone, Debug)]
pub struct Text {
    pub font: Font,
    pub fg: String,
    pub bg: String,
    pub padx: i32,
    pub pady: i32,
    pub bd: i32,
    pub hl: i32,
    pub spacing1: i32,
    pub spacing2: i32,
    pub spacing3: i32,
    pub wrap: Wrap,
    /// -width / -height in characters and lines
    pub width_ch: i32,
    pub height_ln: i32,
    /// lowest priority first ("sel" is the first tag of every text widget)
    pub tags: Vec<TagCfg>,
    lines: Vec<Vec<Run>>,
    pub top: (usize, usize),
    pub top_offset: i32,
    /// the widget's size now (1x1 before it is mapped: the requested size is used then)
    pub w: i32,
    pub h: i32,
    pub dlines: Vec<DLine>,
    /// an editable box (the profile fields): where the insertion cursor is, in characters; None = display only
    pub edit: Option<usize>,
}

fn nl_run() -> Run {
    Run { text: "\n".into(), n: 1, tags: vec![] }
}

impl Text {
    pub fn new(font: Font, fg: &str, bg: &str) -> Text {
        Text {
            font,
            fg: fg.into(),
            bg: bg.into(),
            padx: 1,
            pady: 1,
            bd: 1,
            hl: 0,
            spacing1: 0,
            spacing2: 0,
            spacing3: 0,
            wrap: Wrap::Char,
            width_ch: 80,
            height_ln: 24,
            tags: vec![TagCfg { name: "sel".into(), ..Default::default() }],
            lines: vec![vec![nl_run()]],
            top: (1, 0),
            top_offset: 0,
            w: 1,
            h: 1,
            dlines: vec![],
            edit: None,
        }
    }

    pub fn tag(&mut self, name: &str) -> &mut TagCfg {
        let i = self.tag_index(name);
        &mut self.tags[i]
    }

    /// the tag's place in the priority list, made (on top) when new
    pub fn tag_index(&mut self, name: &str) -> usize {
        if let Some(i) = self.tags.iter().position(|t| t.name == name) {
            return i;
        }
        self.tags.push(TagCfg { name: name.into(), ..Default::default() });
        self.tags.len() - 1
    }

    /// tag raise: to the top of the priority list
    pub fn tag_raise(&mut self, name: &str) {
        let i = self.tag_index(name);
        let t = self.tags.remove(i);
        self.tags.push(t);
        let top = self.tags.len() - 1;
        for line in &mut self.lines {
            for r in line.iter_mut() {
                for k in r.tags.iter_mut() {
                    *k = if *k == i { top } else if *k > i { *k - 1 } else { *k };
                }
            }
        }
    }

    /// tag add `name` over every character that has tag `having` (a tag's .first .. .last when it is one run of
    /// text, as a verse's is)
    pub fn tag_add_over(&mut self, name: &str, having: &str) {
        let Some(h) = self.tags.iter().position(|t| t.name == having) else { return };
        let i = self.tag_index(name);
        for line in &mut self.lines {
            for r in line.iter_mut() {
                if r.tags.contains(&h) && !r.tags.contains(&i) {
                    r.tags.push(i);
                    r.tags.sort();
                }
            }
        }
    }

    /// tag remove `name` 1.0 end
    pub fn tag_remove_all(&mut self, name: &str) {
        let Some(i) = self.tags.iter().position(|t| t.name == name) else { return };
        for line in &mut self.lines {
            for r in line.iter_mut() {
                r.tags.retain(|&k| k != i);
            }
        }
    }

    /// the first index that has the tag (tag.first)
    pub fn tag_first(&self, name: &str) -> Option<(usize, usize)> {
        let i = self.tags.iter().position(|t| t.name == name)?;
        for (ln, line) in self.lines.iter().enumerate() {
            let mut ch = 0;
            for r in line {
                if r.tags.contains(&i) {
                    return Some((ln + 1, ch));
                }
                ch += r.n;
            }
        }
        None
    }

    /// the names of the tags on the character at an index
    pub fn tags_at(&self, (line, ch): (usize, usize)) -> Vec<String> {
        let Some(l) = self.lines.get(line.wrapping_sub(1)) else { return vec![] };
        let mut at = 0;
        for r in l {
            if ch < at + r.n {
                return r.tags.iter().map(|&k| self.tags[k].name.clone()).collect();
            }
            at += r.n;
        }
        vec![]
    }

    pub fn delete_all(&mut self) {
        self.lines = vec![vec![nl_run()]];
        self.top = (1, 0);
        self.top_offset = 0;
    }

    /// insert at "end" (before the text's final newline)
    pub fn insert_end(&mut self, s: &str, tags: &[&str]) {
        if s.is_empty() {
            return;
        }
        let mut ids: Vec<usize> = tags.iter().map(|t| self.tag_index(t)).collect();
        ids.sort();
        ids.dedup();
        let last = self.lines.pop().unwrap();
        let mut cur = last;
        let fin = cur.pop().unwrap();
        for (k, part) in s.split('\n').enumerate() {
            if k > 0 {
                push_run(&mut cur, "\n", &ids);
                self.lines.push(std::mem::take(&mut cur));
            }
            push_run(&mut cur, part, &ids);
        }
        cur.push(fin);
        self.lines.push(cur);
    }

    /// the text without Tk's final newline (get("1.0", "end-1c"))
    pub fn plain(&self) -> String {
        let mut s = self.get();
        if s.ends_with('\n') {
            s.pop();
        }
        s
    }

    /// replace all of the text with plain `s` and lay it out again
    pub fn set_plain(&mut self, f: &dyn Fonts, s: &str) {
        self.delete_all();
        self.insert_end(s, &[]);
        self.relayout(f);
    }

    /// a character offset into plain() as a (line, char) index
    pub fn index_of(&self, off: usize) -> (usize, usize) {
        let (mut line, mut ch) = (1, 0);
        for c in self.plain().chars().take(off) {
            if c == '\n' {
                line += 1;
                ch = 0;
            } else {
                ch += 1;
            }
        }
        (line, ch)
    }

    /// number of real lines (Tk's "end" is the line after them)
    pub fn num_lines(&self) -> usize {
        self.lines.len()
    }

    pub fn line_len(&self, line: usize) -> usize {
        self.lines[line - 1].iter().map(|r| r.n).sum()
    }

    /// the text of lines `a..=b`, as `get`
    pub fn get(&self) -> String {
        self.lines.iter().flatten().map(|r| r.text.as_str()).collect()
    }

    /// GetStyle for a set of tags
    pub fn style(&self, tags: &[usize]) -> Style {
        let mut s = Style {
            font: self.font.clone(),
            fg: self.fg.clone(),
            bg: None,
            spacing1: self.spacing1,
            spacing2: self.spacing2,
            spacing3: self.spacing3,
            lmargin1: 0,
            lmargin2: 0,
            rmargin: 0,
            offset: 0,
            underline: false,
            overstrike: false,
            wrap: self.wrap,
            justify: Justify::Left,
        };
        for &i in tags {
            let t = &self.tags[i];
            macro_rules! set {
                ($f:ident) => {
                    if let Some(v) = &t.$f {
                        s.$f = v.clone();
                    }
                };
            }
            set!(font);
            set!(fg);
            if t.bg.is_some() {
                s.bg = t.bg.clone();
            }
            set!(spacing1);
            set!(spacing2);
            set!(spacing3);
            set!(lmargin1);
            set!(lmargin2);
            set!(rmargin);
            set!(offset);
            set!(underline);
            set!(overstrike);
            set!(wrap);
            set!(justify);
        }
        s
    }

    /// Tk_GeometryRequest in TkTextRelayoutWindow: -width characters of "0", -height lines of linespace + spacing1 + spacing3
    pub fn req_size(&self, f: &dyn Fonts) -> (i32, i32) {
        let cw = f.width(&self.font, "0").max(1);
        let ls = f.linespace(&self.font);
        let e = 2 * (self.bd + self.hl);
        (self.width_ch * cw + e + 2 * self.padx, self.height_ln * (ls + self.spacing1 + self.spacing3) + e + 2 * self.pady)
    }

    /// (x, y, maxX, maxY) of TextDInfo. An unmapped widget is 1x1 and lays out in that (Tk 8.6.14, which zenko.py's
    /// Python ships; later Tk lays out in the requested size)
    pub fn dinfo(&self, _f: &dyn Fonts) -> (i32, i32, i32, i32) {
        let (w, h) = (self.w, self.h);
        let x = self.hl + self.bd + self.padx;
        let y = self.hl + self.bd + self.pady;
        let max_x = (w - self.hl - self.bd - self.padx).max(x + 1);
        let max_y = (h - self.hl - self.bd - self.pady).max(y + 1);
        (x, y, max_x, max_y)
    }

    fn char_height(&self, f: &dyn Fonts) -> i32 {
        f.linespace(&self.font).max(1)
    }

    /// TkTextIndexForwBytes over a display line (they never run past the end of their line)
    fn forw(&self, (line, ch): (usize, usize), n: usize) -> (usize, usize) {
        if ch + n >= self.line_len(line) {
            (line + 1, 0)
        } else {
            (line, ch + n)
        }
    }

    /// TkTextCharLayoutProc on `p` (the rest of one segment) at x
    fn layout_proc(&self, f: &dyn Fonts, p: &[char], x: i32, max_x: i32, no_chars_yet: bool, wrap: Wrap, st: &Style)
                   -> Option<(usize, i32, i32)> {
        let max_bytes = p.len();
        let (mut fit, mut next_x) = measure_chars(f, &st.font, p, x, max_x);
        if fit < max_bytes {
            if fit == 0 && no_chars_yet {
                (fit, next_x) = measure_chars(f, &st.font, &p[..1], x, -1);
            }
            if next_x < max_x && (p[fit] == ' ' || p[fit] == '\t') {
                next_x = max_x;
                fit += 1;
            }
            if wrap == Wrap::Word {
                while fit < max_bytes && p[fit] == ' ' {
                    fit += 1;
                }
            }
            if fit < max_bytes && p[fit] == '\n' {
                fit += 1;
            }
            if fit == 0 {
                return None;
            }
        }
        let mut brk = -1;
        if wrap != Wrap::Word {
            brk = fit as i32;
        } else {
            for count in (1..=fit).rev() {
                if matches!(p[count - 1], '\t' | '\n' | '\x0b' | '\x0c' | '\r' | ' ') {
                    brk = count as i32;
                    break;
                }
            }
        }
        Some((fit, next_x - x, brk))
    }

    /// the segments of a line from character `ch`: (first char, chars, style)
    fn pieces(&self, line: usize, ch: usize) -> Vec<(usize, Vec<char>, Style)> {
        let mut out = vec![];
        let mut at = 0;
        for r in &self.lines[line - 1] {
            let end = at + r.n;
            if end > ch {
                let skip = ch.saturating_sub(at);
                out.push((at + skip, r.text.chars().skip(skip).collect(), self.style(&r.tags)));
            }
            at = end;
        }
        out
    }

    /// LayoutDLine
    pub fn layout_dline(&self, f: &dyn Fonts, line: usize, ch: usize) -> DLine {
        let (dx, _, dmax_x, _) = self.dinfo(f);
        let avail = dmax_x - dx;
        let para_start = ch == 0;
        let mut chunks: Vec<Chunk> = vec![];
        let mut no_chars_yet = true;
        let (mut x, mut max_x) = (0, 0);
        let (mut justify, mut rmargin, mut wrap) = (Justify::Left, 0, Wrap::Char);
        let mut brk: Option<(usize, usize)> = None;
        let pieces = self.pieces(line, ch);
        let mut whole = true;
        for (start, p, st) in &pieces {
            if no_chars_yet {
                justify = st.justify;
                rmargin = st.rmargin;
                wrap = st.wrap;
                x = if para_start { st.lmargin1 } else { st.lmargin2 };
                max_x = if wrap == Wrap::None { -1 } else { (avail - rmargin).max(x) };
            }
            let Some((n, width, bi)) = self.layout_proc(f, p, x, max_x, no_chars_yet, wrap, st) else {
                whole = false;
                break;
            };
            no_chars_yet = false;
            let (a, d) = f.metrics(&st.font);
            let text: String = p[..n].iter().filter(|&&c| c != '\n').collect();
            chunks.push(Chunk { start: *start, n, text, x, width, ascent: a + st.offset, descent: d - st.offset,
                                style: st.clone(), break_index: bi });
            x += width;
            if bi > 0 {
                brk = Some((chunks.len() - 1, bi as usize));
            }
            if n != p.len() {
                whole = false;
                break;
            }
        }
        if chunks.is_empty() {
            return DLine { line, ch, count: 0, y: 0, height: 0, baseline: 0, space_above: 0, length: 0, chunks };
        }
        let (bc, bo) = brk.unwrap_or((chunks.len() - 1, chunks.last().unwrap().n));
        if bc != chunks.len() - 1 || bo != chunks[bc].n {
            chunks.truncate(bc + 1);
            if bo != chunks[bc].n {
                let c = &chunks[bc];
                let (_, p, st) = pieces.iter().find(|q| q.0 <= c.start && c.start < q.0 + q.1.len()).unwrap();
                let off = c.start - pieces.iter().find(|q| q.0 <= c.start && c.start < q.0 + q.1.len()).unwrap().0;
                let q = &p[off..off + bo];
                if let Some((n, width, bi)) = self.layout_proc(f, q, c.x, max_x, false, wrap, st) {
                    let c = &mut chunks[bc];
                    c.n = n;
                    c.width = width;
                    c.break_index = bi;
                    c.text = q[..n].iter().filter(|&&ch| ch != '\n').collect();
                }
            }
            whole = false;
        }
        if wrap == Wrap::None {
            max_x = avail - rmargin;
        }
        let last = chunks.last().unwrap();
        let length = last.x + last.width;
        let j = match justify {
            Justify::Left => 0,
            Justify::Right => max_x - length,
            Justify::Center => (max_x - length) / 2,
        };
        let (mut asc, mut desc, mut count) = (0, 0, 0);
        for c in &mut chunks {
            c.x += j;
            count += c.n;
            asc = asc.max(c.ascent);
            desc = desc.max(c.descent);
        }
        let st = &chunks[0].style;
        let above = if ch == 0 { st.spacing1 } else { st.spacing2 - st.spacing2 / 2 };
        let below = if whole { st.spacing3 } else { st.spacing2 / 2 };
        let last = chunks.last().unwrap();
        let length = last.x + last.width;
        DLine { line, ch, count, y: 0, height: asc + desc + above + below, baseline: asc + above, space_above: above, length,
                chunks }
    }

    /// every display line of a logical line
    pub fn line_dlines(&self, f: &dyn Fonts, line: usize) -> Vec<DLine> {
        let mut out = vec![];
        let mut idx = (line, 0);
        while idx.0 == line {
            let d = self.layout_dline(f, idx.0, idx.1);
            idx = self.forw(idx, d.count.max(1));
            out.push(d);
        }
        out
    }

    /// TkTextFindDisplayLineEnd(.., 0): the start of the display line holding idx
    pub fn display_line_start(&self, f: &dyn Fonts, idx: (usize, usize)) -> (usize, usize) {
        let mut cur = (idx.0, 0);
        loop {
            let d = self.layout_dline(f, cur.0, cur.1);
            let next = self.forw(cur, d.count.max(1));
            if next.0 != cur.0 || next.1 > idx.1 {
                return cur;
            }
            cur = next;
        }
    }

    /// UpdateDisplayInfo: lay out the lines in view from the top index, then fill a gap at the bottom by scrolling back
    pub fn update(&mut self, f: &dyn Fonts) {
        let (_, dy, _, max_y) = self.dinfo(f);
        let mut list: Vec<DLine> = vec![];
        let mut idx = self.top;
        let mut new_off = self.top_offset;
        let mut y = dy - new_off;
        while idx.0 <= self.num_lines() {
            let d = self.layout_dline(f, idx.0, idx.1);
            y += d.height;
            idx = self.forw(idx, d.count.max(1));
            list.push(d);
            if y >= max_y {
                break;
            }
        }
        if y < max_y {
            let mut space = max_y - y;
            if space <= new_off {
                new_off -= space;
            } else {
                y += new_off;
                new_off = 0;
                space = max_y - y;
                let (mut line, mut bytes) = match list.first() {
                    None => (self.num_lines() as i64, i64::MAX),
                    Some(d) => {
                        if d.ch == 0 {
                            (d.line as i64 - 1, i64::MAX)
                        } else {
                            (d.line as i64, d.ch as i64)
                        }
                    }
                };
                while line >= 1 && space > 0 {
                    let mut lowest = vec![];
                    let mut at = (line as usize, 0);
                    loop {
                        let d = self.layout_dline(f, at.0, at.1);
                        at = self.forw(at, d.count.max(1));
                        bytes -= d.count as i64;
                        lowest.push(d);
                        if !(bytes > 0 && at.0 == line as usize) {
                            break;
                        }
                    }
                    while let Some(d) = lowest.pop() {
                        space -= d.height;
                        list.insert(0, d);
                        if space <= 0 {
                            break;
                        }
                    }
                    bytes = i64::MAX;
                    line -= 1;
                }
                if space < 0 {
                    new_off = -space;
                }
            }
        }
        if let Some(d) = list.first() {
            self.top = (d.line, d.ch);
        }
        let mut y = dy - new_off;
        for d in &mut list {
            d.y = y;
            y += d.height;
        }
        self.top_offset = new_off;
        self.dlines = list;
    }

    /// FindDLine
    fn find_dline(&self, idx: (usize, usize)) -> Option<usize> {
        let l = &self.dlines;
        if l.is_empty() {
            return None;
        }
        if idx.0 < l[0].line {
            return Some(0);
        }
        let mut i = 0;
        while (l[i].line, l[i].ch) < idx {
            let prev = i;
            i += 1;
            if i == l.len() {
                return if self.forw((l[prev].line, l[prev].ch), l[prev].count) > idx { Some(prev) } else { None };
            }
            if (l[i].line, l[i].ch) > idx {
                if self.forw((l[prev].line, l[prev].ch), l[prev].count) > idx {
                    i = prev;
                }
                break;
            }
        }
        Some(i)
    }

    /// MeasureUp: the display line `distance` pixels above the bottom of the one holding src, and how much of it is
    /// above that point
    fn measure_up(&self, f: &dyn Fonts, src: (usize, usize), mut distance: i32) -> ((usize, usize), i32) {
        let mut bytes = src.1 as i64 + 1;
        let mut line = src.0;
        while line >= 1 {
            let mut lowest = vec![];
            let mut at = (line, 0);
            loop {
                let d = self.layout_dline(f, at.0, at.1);
                at = self.forw(at, d.count.max(1));
                bytes -= d.count as i64;
                lowest.push(d);
                if !(bytes > 0 && at.0 == line) {
                    break;
                }
            }
            while let Some(d) = lowest.pop() {
                distance -= d.height;
                if distance <= 0 {
                    return ((d.line, d.ch), -distance);
                }
            }
            bytes = i64::MAX;
            line -= 1;
        }
        ((1, 0), 0)
    }

    /// the index "end" stands for in TkTextSetYView: the text's last character
    pub fn end_index(&self) -> (usize, usize) {
        let n = self.num_lines();
        (n, self.line_len(n) - 1)
    }

    /// `see idx` (TkTextSetYView with TK_TEXT_PICKPLACE)
    pub fn see(&mut self, f: &dyn Fonts, idx: (usize, usize)) {
        self.update(f);
        let (_, dy, _, max_y) = self.dinfo(f);
        let mut dl = self.find_dline(idx);
        if let Some(i) = dl {
            let d = &self.dlines[i];
            if d.y + d.height > max_y {
                dl = None;
            } else if (d.line, d.ch) <= idx {
                if i == 0 && self.top_offset != 0 {
                    self.top_offset = 0;
                    self.update(f);
                }
                return;
            }
        }
        let start = self.display_line_start(f, idx);
        let line_h = self.layout_dline(f, start.0, start.1).height;
        let ch = self.char_height(f);
        let mut bottom = (dy + max_y + line_h) / 2;
        let close = ((max_y - dy) / 3).max(3 * ch);
        if dl.is_some() {
            let (tmp, _) = self.measure_up(f, self.top, close + ch / 2);
            if tmp <= idx {
                self.top = start;
                self.top_offset = 0;
                self.update(f);
                return;
            }
        } else {
            let (tmp, _) = self.measure_up(f, idx, close + line_h - ch / 2);
            if self.find_dline(tmp).is_some() {
                bottom = max_y - dy;
            }
        }
        if max_y - dy < line_h {
            bottom = line_h;
        }
        let (top, off) = self.measure_up(f, idx, bottom);
        self.top = top;
        self.top_offset = off;
        self.update(f);
    }

    /// after a change of size or font (TkTextRelayoutWindow): the top index moves to the start of its display line
    pub fn relayout(&mut self, f: &dyn Fonts) {
        if self.top.1 != 0 {
            self.top = self.display_line_start(f, self.top);
        }
        self.update(f);
    }

    /// dlineinfo: (x, y, width, height, baseline)
    pub fn dlineinfo(&self, f: &dyn Fonts, idx: (usize, usize)) -> Option<[i32; 5]> {
        let (dx, _, _, max_y) = self.dinfo(f);
        let i = self.find_dline(idx)?;
        let d = &self.dlines[i];
        if (d.line, d.ch) > idx {
            return None;
        }
        let dlx = d.chunks.first().map(|c| c.x).unwrap_or(0);
        let h = if d.y + d.height > max_y { max_y - d.y } else { d.height };
        Some([dx + dlx, d.y, d.length - dlx, h, d.baseline])
    }

    /// index("@x,y") as far as the line goes: the logical line of the display line under y (the first or last shown
    /// one when y is above or below them); the char is the display line's first
    pub fn index_at(&self, _f: &dyn Fonts, _x: i32, y: i32) -> (usize, usize) {
        let mut best = None;
        for d in &self.dlines {
            if best.is_none() || d.y <= y {
                best = Some((d.line, d.ch));
            }
        }
        best.unwrap_or((1, 0))
    }

    /// bbox of one character
    pub fn bbox(&self, f: &dyn Fonts, idx: (usize, usize)) -> Option<[i32; 4]> {
        let (dx, _, dmax_x, max_y) = self.dinfo(f);
        let i = self.find_dline(idx)?;
        let d = &self.dlines[i];
        if (d.line, d.ch) > idx || idx.0 != d.line || idx.1 >= d.ch + d.count {
            return None;
        }
        let c = d.chunks.iter().find(|c| c.start <= idx.1 && idx.1 < c.start + c.n)?;
        let k = idx.1 - c.start;
        let chars: Vec<char> = c.text.chars().collect();
        let x0 = c.x + f.width(&c.style.font, &chars[..k.min(chars.len())].iter().collect::<String>());
        let mut x = dx + x0;
        let mut w = if k >= chars.len() {
            dx + c.x + c.width - x
        } else {
            f.width(&c.style.font, &chars[..k + 1].iter().collect::<String>()) - (x0 - c.x)
        };
        if self.char_at(idx) == '\n' {
            w = dmax_x - x;
        }
        let y = d.y + d.baseline - c.ascent;
        let mut h = c.ascent + c.descent;
        if x + w > dmax_x {
            w = dmax_x - x;
        }
        if x < dx {
            w -= dx - x;
            x = dx;
        }
        if y + h > max_y {
            h = max_y - y;
        }
        if w <= 0 || h <= 0 {
            return None;
        }
        Some([x, y, w, h])
    }

    pub fn char_at(&self, (line, ch): (usize, usize)) -> char {
        let mut at = 0;
        for r in &self.lines[line - 1] {
            if ch < at + r.n {
                return r.text.chars().nth(ch - at).unwrap();
            }
            at += r.n;
        }
        '\n'
    }

    /// every display line of the text: (start index, height)
    fn all_dlines(&self, f: &dyn Fonts) -> Vec<((usize, usize), i32)> {
        let mut v = vec![];
        for line in 1..=self.num_lines() {
            for d in self.line_dlines(f, line) {
                v.push(((d.line, d.ch), d.height));
            }
        }
        v
    }

    /// which display line holds the top index
    fn top_i(all: &[((usize, usize), i32)], top: (usize, usize)) -> usize {
        all.iter().rposition(|(s, _)| *s <= top).unwrap_or(0)
    }

    /// `yview scroll n units` (YScrollByLines): the top moves n display lines down (up when negative) and keeps its
    /// pixel offset, except that running off the beginning shows the very top
    pub fn scroll_units(&mut self, f: &dyn Fonts, n: i32) {
        let all = self.all_dlines(f);
        if all.is_empty() {
            return;
        }
        let to = Self::top_i(&all, self.top) as i32 + n;
        if to < 0 {
            (self.top, self.top_offset) = (all[0].0, 0);
        } else {
            let mut k = (to as usize).min(all.len() - 1);
            // an offset taller than the new top line carries on into the lines below it
            while self.top_offset >= all[k].1 && k + 1 < all.len() {
                self.top_offset -= all[k].1;
                k += 1;
            }
            self.top = all[k].0;
        }
        self.update(f);
    }

    /// `yview scroll n pages`: the display height less two lines (half of it, or one line, when lines are tall)
    pub fn scroll_pages(&mut self, f: &dyn Fonts, n: i32) {
        let (_, dy, _, max_y) = self.dinfo(f);
        let height = max_y - dy;
        let ch = f.linespace(&self.font);
        let px = if ch * 4 >= height { if ch * 2 >= height { height / 2 } else { ch } } else { height - 2 * ch };
        self.scroll_pixels(f, n * px);
    }

    /// YScrollByPixels
    pub fn scroll_pixels(&mut self, f: &dyn Fonts, px: i32) {
        let all = self.all_dlines(f);
        if all.is_empty() {
            return;
        }
        let i = Self::top_i(&all, self.top);
        let before: i32 = all[..i].iter().map(|d| d.1).sum();
        self.top_at_pixel(&all, before + self.top_offset + px);
        self.update(f);
    }

    /// `yview moveto fraction`
    pub fn moveto(&mut self, f: &dyn Fonts, frac: f64) {
        let all = self.all_dlines(f);
        let total: i32 = all.iter().map(|d| d.1).sum();
        self.top_at_pixel(&all, (0.5 + frac.clamp(0.0, 1.0) * total as f64) as i32);
        self.update(f);
    }

    fn top_at_pixel(&mut self, all: &[((usize, usize), i32)], px: i32) {
        let mut acc = 0;
        let px = px.max(0);
        for (k, (s, h)) in all.iter().enumerate() {
            if px < acc + h || k + 1 == all.len() {
                self.top = *s;
                self.top_offset = (px - acc).clamp(0, (*h - 1).max(0));
                return;
            }
            acc += h;
        }
    }

    /// yview: (first, last) fractions of all the text's pixels (GetYView)
    pub fn yview(&self, f: &dyn Fonts) -> (f64, f64) {
        let (_, _, _, max_y) = self.dinfo(f);
        let Some(first_d) = self.dlines.first() else {
            return (0.0, 1.0);
        };
        let mut total = 0;
        let mut before = 0;
        for line in 1..=self.num_lines() {
            for d in self.line_dlines(f, line) {
                if (d.line, d.ch) < (first_d.line, first_d.ch) {
                    before += d.height;
                }
                total += d.height;
            }
        }
        if total == 0 {
            return (0.0, 1.0);
        }
        let first = (before + self.top_offset) as f64 / total as f64;
        let mut count = before;
        for d in &self.dlines {
            count += d.height;
            let extra = d.y + d.height - max_y;
            if extra > 0 {
                count -= extra;
                break;
            }
        }
        (first, count.min(total) as f64 / total as f64)
    }
}

impl Text {
    /// SAME_BACKGROUND
    fn same_bg(a: &Style, b: &Style) -> bool {
        a.bg == b.bg
    }

    /// what DisplayText paints, in the widget's own coordinates: the background, the highlight ring (`ring`: the
    /// highlight colour when focused, else highlightbackground) and every display line clipped to the inside
    /// (DisplayDLine: tag backgrounds as DisplayLineBackground, then each chunk as CharDisplayProc with its underline
    /// and overstrike)
    pub fn ops(&self, f: &dyn Fonts, ring: &str) -> Vec<crate::canvasdraw::TkOp> {
        use crate::canvasdraw::TkOp;
        let (dx, dy, dmax_x, max_y) = self.dinfo(f);
        let mut out = vec![TkOp::FillRect { x: 0, y: 0, w: self.w, h: self.h, color: self.bg.clone() }];
        if self.hl > 0 {
            let (w, h, k) = (self.w, self.h, self.hl);
            for (x, y, ww, hh) in [(0, 0, w, k), (0, h - k, w, k), (0, 0, k, h), (w - k, 0, k, h)] {
                out.push(TkOp::FillRect { x, y, w: ww, h: hh, color: ring.into() });
            }
        }
        out.push(TkOp::Clip(Some([dx, dy, dmax_x - dx, max_y - dy])));
        let max_x = dmax_x - dx;
        for d in &self.dlines {
            if d.y >= max_y || d.y + d.height <= dy {
                continue;
            }
            let mut left = 0;
            for (i, c) in d.chunks.iter().enumerate() {
                if let Some(n) = d.chunks.get(i + 1) {
                    if Self::same_bg(&n.style, &c.style) {
                        continue;
                    }
                }
                let mut right = c.x + c.width;
                if i + 1 == d.chunks.len() && right < max_x {
                    right = max_x;
                }
                if let Some(bg) = &c.style.bg {
                    if right > left {
                        out.push(TkOp::FillRect { x: left + dx, y: d.y, w: right - left, h: d.height, color: bg.clone() });
                    }
                }
                left = right;
                if left >= max_x {
                    break;
                }
            }
            for c in &d.chunks {
                if c.text.is_empty() {
                    continue;
                }
                let st = &c.style;
                let (x, base) = (dx + c.x, d.y + d.baseline - st.offset);
                out.push(TkOp::Text { runs: vec![(x, base, c.text.clone())], font: st.font.clone(), color: st.fg.clone() });
                let (asc, desc) = f.metrics(&st.font);
                let mut pos = desc / 2;
                let mut uh = ((f.pixels(&st.font) / 10.0 + 0.5) as i32).max(1);
                if pos + uh > desc {
                    uh = desc - pos;
                    if uh == 0 {
                        pos -= 1;
                        uh = 1;
                    }
                }
                let w = f.width(&st.font, &c.text);
                if st.underline {
                    out.push(TkOp::FillRect { x, y: base + pos, w, h: uh, color: st.fg.clone() });
                }
                if st.overstrike {
                    out.push(TkOp::FillRect { x, y: base - desc - asc * 3 / 10 + pos, w, h: uh, color: st.fg.clone() });
                }
            }
        }
        out.push(TkOp::Clip(None));
        out
    }
}

/// MeasureChars (tkTextDisp.c) over one run without tabs: stops at a newline; Tk_MeasureChars with TK_ISOLATE_END
fn measure_chars(f: &dyn Fonts, font: &Font, p: &[char], start_x: i32, max_x: i32) -> (usize, i32) {
    let special = p.iter().position(|&c| c == '\n').unwrap_or(p.len());
    if max_x >= 0 && start_x >= max_x {
        return (0, start_x);
    }
    let (n, w) = tk_measure(f, font, &p[..special], if max_x >= 0 { max_x - start_x } else { -1 });
    (n, start_x + w)
}

/// Tk_MeasureChars (tkWinFont.c) with no flags but TK_ISOLATE_END: the longest prefix whose width fits
pub fn tk_measure(f: &dyn Fonts, font: &Font, p: &[char], max_len: i32) -> (usize, i32) {
    if p.is_empty() {
        return (0, 0);
    }
    let all: String = p.iter().collect();
    let w = f.width(font, &all);
    if max_len < 0 || w <= max_len {
        return (p.len(), w);
    }
    let mut last = 0;
    let mut s = String::new();
    for (k, &c) in p.iter().enumerate() {
        s.push(c);
        let w = f.width(font, &s);
        if w > max_len {
            return (k, last);
        }
        last = w;
    }
    (p.len(), last)
}

fn push_run(line: &mut Vec<Run>, s: &str, tags: &[usize]) {
    if s.is_empty() {
        return;
    }
    if let Some(r) = line.last_mut() {
        if r.tags == tags && r.text != "\n" {
            r.text.push_str(s);
            r.n += s.chars().count();
            return;
        }
    }
    line.push(Run { text: s.into(), n: s.chars().count(), tags: tags.to_vec() });
}
