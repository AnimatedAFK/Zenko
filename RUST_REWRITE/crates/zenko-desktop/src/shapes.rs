//! zenko.py round_pts / _stair / SHAPES: the pixel-stepped boxes used all round the app.


fn pyround(x: f64) -> f64 {
    x.round_ties_even()
}

fn stair(r: i32, step: i32, cut: bool) -> Vec<(f64, f64)> {
    if r <= 0 {
        return vec![(0.0, 0.0)];
    }
    let rf = r as f64;
    let mut pts = vec![(0.0, rf)];
    let n = 1.max(pyround(rf / step as f64) as i32);
    let mut px = 0.0;
    for k in 1..=n {
        let y = rf - k as f64 * rf / n as f64;
        let mut x = if cut { rf - y } else { rf - (0f64.max(rf * rf - (rf - y) * (rf - y))).sqrt() };
        x = if k < n { pyround(x / step as f64) * step as f64 } else { rf };
        let y = pyround(y);
        pts.push((px, y));
        pts.push((x, y));
        px = x;
    }
    let mut out: Vec<(f64, f64)> = vec![];
    for p in pts {
        if out.last() != Some(&p) {
            out.push(p);
        }
    }
    out
}

/// round_pts with SHAPES[shape](r); no notch. Returns integer canvas points (Tk rounds coordinates the same way).
pub fn round_pts(x1: i32, y1: i32, x2: i32, y2: i32, r: i32, step: i32, shape: &str) -> Vec<(i32, i32)> {
    let half = ((x2 - x1).div_euclid(2)).min((y2 - y1).div_euclid(2));
    // SHAPES[shape](r) is built from the r passed in; each corner is then clamped to the box
    let cs: [(bool, i32); 4] = match shape {
        "pill" => [(false, 999); 4],
        "ema" => [(true, r + 4), (true, r + 4), (false, r), (false, r)],
        "leaf" => [(false, r * 2), (false, r.div_euclid(2)), (false, r * 2), (false, r.div_euclid(2))],
        _ => [(false, r); 4],
    };
    let cs: Vec<(bool, i32)> = cs.iter().map(|&(k, rr)| (k, 0.max(rr.min(half)))).collect();
    let (x1f, y1f, x2f, y2f) = (x1 as f64, y1 as f64, x2 as f64, y2 as f64);
    let mut out: Vec<(f64, f64)> = vec![];
    out.extend(stair(cs[0].1, step, cs[0].0).iter().map(|&(ox, oy)| (x1f + ox, y1f + oy)));
    out.extend(stair(cs[1].1, step, cs[1].0).iter().map(|&(ox, oy)| (x2f - oy, y1f + ox)));
    out.extend(stair(cs[2].1, step, cs[2].0).iter().map(|&(ox, oy)| (x2f - ox, y2f - oy)));
    out.extend(stair(cs[3].1, step, cs[3].0).iter().map(|&(ox, oy)| (x1f + oy, y2f - ox)));
    out.iter().map(|&(x, y)| (tk_coord(x), tk_coord(y))).collect()
}

/// Tk canvas coordinate to device pixel (TkCanvTranslatePath rounds half away from zero).
fn tk_coord(v: f64) -> i32 {
    if v > 0.0 { (v + 0.5) as i32 } else { (v - 0.5) as i32 }
}

