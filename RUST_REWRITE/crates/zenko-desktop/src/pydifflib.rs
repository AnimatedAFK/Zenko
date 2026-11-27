//! Python's difflib.SequenceMatcher (isjunk None, autojunk on) and get_close_matches, step for step, so fuzzy
//! replies and the shorthand learner pick exactly what zenko.py picks.

use std::collections::HashMap;
use std::hash::Hash;

pub struct Matcher<'a, T> {
    a: &'a [T],
    b: &'a [T],
    b2j: HashMap<&'a T, Vec<usize>>,
}

impl<'a, T: Eq + Hash> Matcher<'a, T> {
    pub fn new(a: &'a [T], b: &'a [T]) -> Self {
        let mut b2j: HashMap<&T, Vec<usize>> = HashMap::new();
        for (i, e) in b.iter().enumerate() {
            b2j.entry(e).or_default().push(i);
        }
        let n = b.len();
        if n >= 200 {
            let ntest = n / 100 + 1;
            b2j.retain(|_, v| v.len() <= ntest);
        }
        Matcher { a, b, b2j }
    }

    pub fn find_longest_match(&self, alo: usize, ahi: usize, blo: usize, bhi: usize) -> (usize, usize, usize) {
        let (a, b) = (self.a, self.b);
        let (mut besti, mut bestj, mut bestsize) = (alo, blo, 0usize);
        let mut j2len: HashMap<usize, usize> = HashMap::new();
        for i in alo..ahi {
            let mut newj2len = HashMap::new();
            if let Some(js) = self.b2j.get(&a[i]) {
                for &j in js {
                    if j < blo {
                        continue;
                    }
                    if j >= bhi {
                        break;
                    }
                    let k = j.checked_sub(1).and_then(|p| j2len.get(&p)).copied().unwrap_or(0) + 1;
                    newj2len.insert(j, k);
                    if k > bestsize {
                        besti = i + 1 - k;
                        bestj = j + 1 - k;
                        bestsize = k;
                    }
                }
            }
            j2len = newj2len;
        }
        // no junk: only the "popular" elements left out of b2j can extend a match
        while besti > alo && bestj > blo && a[besti - 1] == b[bestj - 1] {
            besti -= 1;
            bestj -= 1;
            bestsize += 1;
        }
        while besti + bestsize < ahi && bestj + bestsize < bhi && a[besti + bestsize] == b[bestj + bestsize] {
            bestsize += 1;
        }
        (besti, bestj, bestsize)
    }

    pub fn matching_blocks(&self) -> Vec<(usize, usize, usize)> {
        let (la, lb) = (self.a.len(), self.b.len());
        let mut queue = vec![(0, la, 0, lb)];
        let mut mb = vec![];
        while let Some((alo, ahi, blo, bhi)) = queue.pop() {
            let (i, j, k) = self.find_longest_match(alo, ahi, blo, bhi);
            if k > 0 {
                mb.push((i, j, k));
                if alo < i && blo < j {
                    queue.push((alo, i, blo, j));
                }
                if i + k < ahi && j + k < bhi {
                    queue.push((i + k, ahi, j + k, bhi));
                }
            }
        }
        mb.sort();
        let (mut i1, mut j1, mut k1) = (0, 0, 0);
        let mut out = vec![];
        for (i2, j2, k2) in mb {
            if i1 + k1 == i2 && j1 + k1 == j2 {
                k1 += k2;
            } else {
                if k1 > 0 {
                    out.push((i1, j1, k1));
                }
                (i1, j1, k1) = (i2, j2, k2);
            }
        }
        if k1 > 0 {
            out.push((i1, j1, k1));
        }
        out.push((la, lb, 0));
        out
    }

    /// (tag, i1, i2, j1, j2)
    pub fn opcodes(&self) -> Vec<(&'static str, usize, usize, usize, usize)> {
        let (mut i, mut j) = (0, 0);
        let mut out = vec![];
        for (ai, bj, size) in self.matching_blocks() {
            let tag = if i < ai && j < bj {
                "replace"
            } else if i < ai {
                "delete"
            } else if j < bj {
                "insert"
            } else {
                ""
            };
            if !tag.is_empty() {
                out.push((tag, i, ai, j, bj));
            }
            i = ai + size;
            j = bj + size;
            if size > 0 {
                out.push(("equal", ai, i, bj, j));
            }
        }
        out
    }

    fn calc(&self, matches: usize) -> f64 {
        let l = self.a.len() + self.b.len();
        if l > 0 { 2.0 * matches as f64 / l as f64 } else { 1.0 }
    }

    pub fn ratio(&self) -> f64 {
        self.calc(self.matching_blocks().iter().map(|m| m.2).sum())
    }

    pub fn quick_ratio(&self) -> f64 {
        let mut full: HashMap<&T, i64> = HashMap::new();
        for e in self.b {
            *full.entry(e).or_default() += 1;
        }
        let mut avail: HashMap<&T, i64> = HashMap::new();
        let mut matches = 0;
        for e in self.a {
            let n = *avail.get(e).unwrap_or_else(|| full.get(e).unwrap_or(&0));
            avail.insert(e, n - 1);
            if n > 0 {
                matches += 1;
            }
        }
        self.calc(matches)
    }

    pub fn real_quick_ratio(&self) -> f64 {
        self.calc(self.a.len().min(self.b.len()))
    }
}

/// difflib.get_close_matches(word, possibilities, n, cutoff)
pub fn get_close_matches(word: &str, possibilities: &[String], n: usize, cutoff: f64) -> Vec<String> {
    let w: Vec<char> = word.chars().collect();
    let mut res: Vec<(f64, &String)> = vec![];
    for x in possibilities {
        let xc: Vec<char> = x.chars().collect();
        let s = Matcher::new(&xc, &w);
        if s.real_quick_ratio() >= cutoff && s.quick_ratio() >= cutoff {
            let r = s.ratio();
            if r >= cutoff {
                res.push((r, x));
            }
        }
    }
    // heapq.nlargest: biggest (score, word) first; equal pairs keep their order
    res.sort_by(|p, q| q.0.partial_cmp(&p.0).unwrap().then_with(|| q.1.cmp(p.1)));
    res.into_iter().take(n).map(|(_, x)| x.clone()).collect()
}
