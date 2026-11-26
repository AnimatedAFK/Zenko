//! Tk's geometry managers, for laying out zenko.py's dialogs to the pixel: grid (tkGrid.c ResolveConstraints,
//! AdjustForSticky) and pack (tkPack.c, the simple cases zenko.py uses). Slot weights, minsizes and uniform groups
//! are never set on these dialogs, so they are left out.

/// One widget gridded into a container.
#[derive(Clone, Debug)]
pub struct Cell {
    pub row: usize,
    pub col: usize,
    pub rows: usize,
    pub cols: usize,
    /// Tk_ReqWidth / Tk_ReqHeight
    pub w: i32,
    pub h: i32,
    /// padx / pady as (before, after)
    pub padx: (i32, i32),
    pub pady: (i32, i32),
    /// ipadx / ipady as given (Tk stores twice this)
    pub ipadx: i32,
    pub ipady: i32,
    /// "n", "s", "e", "w" letters
    pub sticky: &'static str,
}

impl Cell {
    pub fn new(row: usize, col: usize, w: i32, h: i32) -> Cell {
        Cell { row, col, rows: 1, cols: 1, w, h, padx: (0, 0), pady: (0, 0), ipadx: 0, ipady: 0, sticky: "" }
    }
    pub fn span(mut self, cols: usize) -> Cell {
        self.cols = cols;
        self
    }
    pub fn sticky(mut self, s: &'static str) -> Cell {
        self.sticky = s;
        self
    }
    pub fn padx(mut self, a: i32, b: i32) -> Cell {
        self.padx = (a, b);
        self
    }
    pub fn pady(mut self, a: i32, b: i32) -> Cell {
        self.pady = (a, b);
        self
    }
    pub fn ipad(mut self, x: i32, y: i32) -> Cell {
        self.ipadx = x;
        self.ipady = y;
        self
    }
}

#[derive(Clone, Copy, Default)]
struct Slot {
    min_size: i32,
    min_offset: i32,
    max_offset: i32,
}

/// ResolveConstraints for one direction. `items` = (first slot, span, size). Returns (offsets, required size).
pub fn resolve(items: &[(usize, usize, i32)]) -> (Vec<i32>, i32) {
    let n = items.iter().map(|&(s, k, _)| s + k).max().unwrap_or(0);
    // index 0 is Tk's layoutPtr[-1]
    let mut l = vec![Slot::default(); n + 1];
    let mut bins: Vec<Vec<(usize, i32)>> = vec![vec![]; n + 1];
    for &(s, k, size) in items {
        let right = s + k - 1;
        if k > 1 {
            bins[right + 1].insert(0, (k, size));
        } else if size > l[right + 1].min_size {
            l[right + 1].min_size = size;
        }
    }
    let mut offset = 0;
    for slot in 1..=n {
        l[slot].min_offset = l[slot].min_size + offset;
        for &(span, size) in &bins[slot] {
            let required = size + l[slot - span].min_offset;
            if required > l[slot].min_offset {
                l[slot].min_offset = required;
            }
        }
        offset = l[slot].min_offset;
    }
    let required_size = offset;
    for s in l.iter_mut().skip(1) {
        s.max_offset = offset;
    }
    let mut slot = n;
    while slot > 1 {
        for &(span, size) in &bins[slot] {
            let require = offset - size;
            let start = slot as i64 - span as i64; // Tk index (slot-1) - span
            if start >= 1 && require < l[start as usize].max_offset {
                l[start as usize].max_offset = require;
            }
        }
        offset -= l[slot].min_size;
        slot -= 1;
        if l[slot].max_offset < offset {
            offset = l[slot].max_offset;
        } else {
            l[slot].max_offset = offset;
        }
    }
    // share out the slack inside each run of slots whose position is not fixed
    let mut start = 1;
    while start <= n {
        if l[start].min_offset == l[start].max_offset {
            start += 1;
            continue;
        }
        let mut end = start + 1;
        while end <= n {
            if l[end].min_offset == l[end].max_offset {
                break;
            }
            end += 1;
        }
        let end = end.min(n);
        let mut need = 0;
        for s in &l[start..=end] {
            need += s.min_size;
        }
        let mut have = l[end].max_offset - l[start - 1].min_offset;
        let total_weight = (end - start + 1) as i32; // no weights: every slot counts as 1
        loop {
            let mut prev_min = l[start - 1].min_offset;
            let mut prev_grow = 0;
            let mut acc = 0;
            let mut s = start;
            let mut broke = false;
            while s <= end {
                acc += 1;
                let mut grow = (have - need) * acc / total_weight - prev_grow;
                prev_grow += grow;
                if prev_min + l[s].min_size + grow > l[s].max_offset {
                    grow = l[s].max_offset - l[s].min_size - prev_min;
                    let mut new_have = grow * total_weight;
                    if new_have > total_weight {
                        new_have = new_have / total_weight * total_weight;
                    }
                    if new_have <= 0 {
                        new_have = (have - need) - 1;
                        if new_have > 3 * total_weight {
                            new_have = new_have * 3 / 4;
                        }
                        if new_have > total_weight {
                            new_have = new_have / total_weight * total_weight;
                        }
                        if new_have <= 0 {
                            new_have = 1;
                        }
                    }
                    have = new_have + need;
                    broke = true;
                    break;
                }
                prev_min += l[s].min_size + grow;
                if prev_min < l[s].min_offset {
                    prev_min = l[s].min_offset;
                }
                s += 1;
            }
            if !broke {
                break;
            }
        }
        let mut prev_grow = 0;
        let mut acc = 0;
        for s in start..=end {
            acc += 1;
            let grow = (have - need) * acc / total_weight - prev_grow;
            prev_grow += grow;
            l[s].min_size += grow;
            if l[s - 1].min_offset + l[s].min_size > l[s].min_offset {
                l[s].min_offset = l[s - 1].min_offset + l[s].min_size;
            }
        }
        for s in (start + 1..=end).rev() {
            if l[s].max_offset - l[s].min_size < l[s - 1].max_offset {
                l[s - 1].max_offset = l[s].max_offset - l[s].min_size;
            }
        }
        // no start = end + 1: Tk goes round the same run again until its first slot is pinned
    }
    (l[1..].iter().map(|s| s.min_offset).collect(), required_size)
}

/// grid: lay the cells out in a container whose inner area starts at (bx, by). Returns the container's requested
/// inner size and each cell's (x, y, w, h).
pub fn grid(cells: &[Cell], bx: i32, by: i32) -> ((i32, i32), Vec<(i32, i32, i32, i32)>) {
    let cols: Vec<_> =
        cells.iter().map(|c| (c.col, c.cols, c.w + c.padx.0 + c.padx.1 + 2 * c.ipadx)).collect();
    let rows: Vec<_> =
        cells.iter().map(|c| (c.row, c.rows, c.h + c.pady.0 + c.pady.1 + 2 * c.ipady)).collect();
    let (co, w) = resolve(&cols);
    let (ro, h) = resolve(&rows);
    let mut out = vec![];
    for c in cells {
        let mut x = if c.col > 0 { co[c.col - 1] } else { 0 };
        let mut y = if c.row > 0 { ro[c.row - 1] } else { 0 };
        let mut cw = co[c.col + c.cols - 1] - x;
        let mut ch = ro[c.row + c.rows - 1] - y;
        x += bx + c.padx.0;
        cw -= c.padx.0 + c.padx.1;
        y += by + c.pady.0;
        ch -= c.pady.0 + c.pady.1;
        let (rw, rh) = (c.w + 2 * c.ipadx, c.h + 2 * c.ipady);
        let (mut dx, mut dy) = (0, 0);
        if cw > rw {
            dx = cw - rw;
            cw = rw;
        }
        if ch > rh {
            dy = ch - rh;
            ch = rh;
        }
        let st = |k: char| c.sticky.contains(k);
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
        out.push((x, y, cw, ch));
    }
    ((w, h), out)
}

/// pack(side="left") of widgets (w, h, padx before, padx after) into a container: its requested size and each
/// widget's (x, y) when the container is exactly that size (widgets centred vertically in their parcels).
pub fn pack_left(items: &[(i32, i32, i32, i32)]) -> ((i32, i32), Vec<(i32, i32)>) {
    let w: i32 = items.iter().map(|i| i.0 + i.2 + i.3).sum();
    let h = items.iter().map(|i| i.1).max().unwrap_or(0);
    let mut x = 0;
    let mut out = vec![];
    for &(iw, ih, a, b) in items {
        out.push((x + a, (h - ih) / 2));
        x += iw + a + b;
    }
    ((w, h), out)
}

/// pack(anchor="w") (side top) of widgets (w, h): requested size and each widget's (x, y).
pub fn pack_top_w(items: &[(i32, i32)]) -> ((i32, i32), Vec<(i32, i32)>) {
    let w = items.iter().map(|i| i.0).max().unwrap_or(0);
    let mut y = 0;
    let mut out = vec![];
    for &(_, ih) in items {
        out.push((0, y));
        y += ih;
    }
    ((w, y), out)
}
