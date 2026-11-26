//! Palette grids -> RGBA pixels, and zenko21's "HD" pixel-art upscaler (Scale2x / Scale3x),
//! ported from zenko.py sprite_rows, _zk_s2x, _zk_s3x, _zk_nearest, zk_upscale, sprite_png_hd.

use crate::core::Grid;
use crate::palette::Palette;

pub type Px = [u8; 4];

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Image {
    pub w: usize,
    pub h: usize,
    pub px: Vec<Px>,
}

impl Image {
    #[inline]
    pub fn at(&self, x: usize, y: usize) -> Px {
        self.px[y * self.w + x]
    }
}

/// Palette-letter grids (drawn in order) -> RGBA (letters missing from the palette stay see-through).
pub fn sprite_rows(layers: &[&Grid], pal: &Palette, bg: Option<[u8; 3]>) -> Image {
    let (w, h) = (layers[0].w as usize, layers[0].h as usize);
    let clear = match bg {
        None => [0, 0, 0, 0],
        Some(c) => [c[0], c[1], c[2], 255],
    };
    let mut px = vec![clear; w * h];
    for g in layers {
        for (i, &ch) in g.c.iter().enumerate() {
            if ch != b'.' {
                if let Some(c) = pal.get(ch) {
                    px[i] = [c[0], c[1], c[2], 255];
                }
            }
        }
    }
    Image { w, h, px }
}

fn s2x(src: &Image) -> Image {
    let (w, h) = (src.w, src.h);
    let mut out = vec![[0u8; 4]; w * h * 4];
    let ow = w * 2;
    for y in 0..h {
        let up = if y > 0 { y - 1 } else { y };
        let dn = if y + 1 < h { y + 1 } else { y };
        for x in 0..w {
            let e = src.at(x, y);
            let b = src.at(x, up);
            let hh = src.at(x, dn);
            let d = if x > 0 { src.at(x - 1, y) } else { e };
            let f = if x + 1 < w { src.at(x + 1, y) } else { e };
            let (o0, o1, o2, o3) = if b != hh && d != f {
                (
                    if d == b { d } else { e },
                    if b == f { f } else { e },
                    if d == hh { d } else { e },
                    if hh == f { f } else { e },
                )
            } else {
                (e, e, e, e)
            };
            out[(2 * y) * ow + 2 * x] = o0;
            out[(2 * y) * ow + 2 * x + 1] = o1;
            out[(2 * y + 1) * ow + 2 * x] = o2;
            out[(2 * y + 1) * ow + 2 * x + 1] = o3;
        }
    }
    Image {
        w: ow,
        h: h * 2,
        px: out,
    }
}

fn s3x(src: &Image) -> Image {
    let (w, h) = (src.w, src.h);
    let ow = w * 3;
    let mut out = vec![[0u8; 4]; w * h * 9];
    for y in 0..h {
        let up = if y > 0 { y - 1 } else { y };
        let dn = if y + 1 < h { y + 1 } else { y };
        for x in 0..w {
            let e = src.at(x, y);
            let b = src.at(x, up);
            let hh = src.at(x, dn);
            let xl = if x > 0 { x - 1 } else { x };
            let xr = if x + 1 < w { x + 1 } else { x };
            let (d, f) = (src.at(xl, y), src.at(xr, y));
            let cells: [Px; 9] = if b != hh && d != f {
                let (a, c, g, i) = (
                    src.at(xl, up),
                    src.at(xr, up),
                    src.at(xl, dn),
                    src.at(xr, dn),
                );
                [
                    if d == b { d } else { e },
                    if (d == b && e != c) || (b == f && e != a) {
                        b
                    } else {
                        e
                    },
                    if b == f { f } else { e },
                    if (d == b && e != g) || (d == hh && e != a) {
                        d
                    } else {
                        e
                    },
                    e,
                    if (b == f && e != i) || (hh == f && e != c) {
                        f
                    } else {
                        e
                    },
                    if d == hh { d } else { e },
                    if (d == hh && e != i) || (hh == f && e != g) {
                        hh
                    } else {
                        e
                    },
                    if hh == f { f } else { e },
                ]
            } else {
                [e; 9]
            };
            for r in 0..3 {
                for c in 0..3 {
                    out[(3 * y + r) * ow + 3 * x + c] = cells[r * 3 + c];
                }
            }
        }
    }
    Image {
        w: ow,
        h: h * 3,
        px: out,
    }
}

fn nearest(src: &Image, k: usize) -> Image {
    let (ow, oh) = (src.w * k, src.h * k);
    let mut px = Vec::with_capacity(ow * oh);
    for y in 0..oh {
        for x in 0..ow {
            px.push(src.at(x / k, y / k));
        }
    }
    Image { w: ow, h: oh, px }
}

/// Rows of RGBA -> `zoom` times bigger, smoothed like hand-made high-res pixel art.
pub fn zk_upscale(img: &Image, zoom: usize) -> Image {
    let mut z = zoom;
    let mut cur = img.clone();
    while z > 1 {
        if z % 2 == 0 {
            cur = s2x(&cur);
            z /= 2;
        } else if z % 3 == 0 {
            cur = s3x(&cur);
            z /= 3;
        } else {
            return nearest(&cur, z);
        }
    }
    cur
}

/// zenko21 sprite_png_hd (pixels before PNG encoding).
pub fn sprite_hd(layers: &[&Grid], pal: &Palette, zoom: usize, bg: Option<[u8; 3]>) -> Image {
    let px = sprite_rows(layers, pal, bg);
    if zoom == 5 || zoom == 7 {
        let big = zk_upscale(&px, zoom - 1);
        let (n, m) = (px.h, px.w);
        let ys: Vec<usize> = (0..n * zoom)
            .map(|i| (i as f64 * (zoom - 1) as f64 / zoom as f64) as usize)
            .collect();
        let xs: Vec<usize> = (0..m * zoom)
            .map(|i| (i as f64 * (zoom - 1) as f64 / zoom as f64) as usize)
            .collect();
        let mut out = Vec::with_capacity(ys.len() * xs.len());
        for &y in &ys {
            for &x in &xs {
                out.push(big.at(x, y));
            }
        }
        Image {
            w: xs.len(),
            h: ys.len(),
            px: out,
        }
    } else {
        zk_upscale(&px, zoom)
    }
}

/// What Tk shows for sprite_photo(layers, pal, zoom): HD smoothing when zoom >= 2 (cfg hd_art on),
/// else plain whole-step zoom.
pub fn sprite_photo(layers: &[&Grid], pal: &Palette, zoom: usize, hd: bool) -> Image {
    if hd && zoom >= 2 {
        return sprite_hd(layers, pal, zoom, None);
    }
    let px = sprite_rows(layers, pal, None);
    if zoom > 1 { nearest(&px, zoom) } else { px }
}
