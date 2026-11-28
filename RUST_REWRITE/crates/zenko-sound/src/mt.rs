//! Python's `random.Random(seed)` (MT19937 with CPython's seeding), so every synthesised sound uses the very same noise.

const N: usize = 624;
const M: usize = 397;

pub struct PyRandom {
    mt: [u32; N],
    idx: usize,
    gauss_next: Option<f64>,
}

impl PyRandom {
    /// `random.Random(seed)` for a non-negative int seed (CPython `random_seed`: abs(seed) as 32-bit words).
    pub fn new(seed: u64) -> PyRandom {
        let mut key: Vec<u32> = Vec::new();
        let mut s = seed;
        while s > 0 {
            key.push(s as u32);
            s >>= 32;
        }
        PyRandom::from_key(key)
    }

    /// `random.Random(text)` for a str seed (CPython version 2 seeding: the text's UTF-8 bytes then their SHA-512,
    /// read as one big-endian int)
    pub fn new_str(text: &str) -> PyRandom {
        use sha2::{Digest, Sha512};
        let mut b = text.as_bytes().to_vec();
        b.extend_from_slice(&Sha512::digest(text.as_bytes()));
        let mut key = Vec::new();
        let mut end = b.len();
        while end > 0 {
            let st = end.saturating_sub(4);
            let mut w = 0u32;
            for &x in &b[st..end] {
                w = (w << 8) | x as u32;
            }
            key.push(w);
            end = st;
        }
        while key.len() > 1 && *key.last().unwrap() == 0 {
            key.pop();
        }
        PyRandom::from_key(key)
    }

    fn from_key(mut key: Vec<u32>) -> PyRandom {
        if key.is_empty() {
            key.push(0);
        }
        let mut r = PyRandom { mt: [0; N], idx: N, gauss_next: None };
        r.init_genrand(19650218);
        let mt = &mut r.mt;
        let (mut i, mut j) = (1usize, 0usize);
        let mut k = N.max(key.len());
        while k > 0 {
            mt[i] = (mt[i] ^ ((mt[i - 1] ^ (mt[i - 1] >> 30)).wrapping_mul(1664525)))
                .wrapping_add(key[j])
                .wrapping_add(j as u32);
            i += 1;
            j += 1;
            if i >= N {
                mt[0] = mt[N - 1];
                i = 1;
            }
            if j >= key.len() {
                j = 0;
            }
            k -= 1;
        }
        k = N - 1;
        while k > 0 {
            mt[i] = (mt[i] ^ ((mt[i - 1] ^ (mt[i - 1] >> 30)).wrapping_mul(1566083941))).wrapping_sub(i as u32);
            i += 1;
            if i >= N {
                mt[0] = mt[N - 1];
                i = 1;
            }
            k -= 1;
        }
        mt[0] = 0x8000_0000;
        r
    }

    fn init_genrand(&mut self, s: u32) {
        self.mt[0] = s;
        for i in 1..N {
            self.mt[i] = 1812433253u32.wrapping_mul(self.mt[i - 1] ^ (self.mt[i - 1] >> 30)).wrapping_add(i as u32);
        }
        self.idx = N;
    }

    pub fn genrand(&mut self) -> u32 {
        const UPPER: u32 = 0x8000_0000;
        const LOWER: u32 = 0x7fff_ffff;
        const MATRIX: u32 = 0x9908_b0df;
        if self.idx >= N {
            let mt = &mut self.mt;
            for kk in 0..N {
                let y = (mt[kk] & UPPER) | (mt[(kk + 1) % N] & LOWER);
                let v = mt[(kk + M) % N] ^ (y >> 1) ^ if y & 1 != 0 { MATRIX } else { 0 };
                mt[kk] = v;
            }
            self.idx = 0;
        }
        let mut y = self.mt[self.idx];
        self.idx += 1;
        y ^= y >> 11;
        y ^= (y << 7) & 0x9d2c_5680;
        y ^= (y << 15) & 0xefc6_0000;
        y ^= y >> 18;
        y
    }

    /// `random.random()`
    pub fn random(&mut self) -> f64 {
        let a = (self.genrand() >> 5) as f64;
        let b = (self.genrand() >> 6) as f64;
        (a * 67108864.0 + b) * (1.0 / 9007199254740992.0)
    }

    /// `random.getrandbits(k)` for k <= 32
    pub fn getrandbits(&mut self, k: u32) -> u32 {
        if k == 0 { 0 } else { self.genrand() >> (32 - k) }
    }

    /// `random._randbelow(n)`
    pub fn randbelow(&mut self, n: u64) -> u64 {
        let k = 64 - n.leading_zeros();
        let mut r = self.getrandbits(k) as u64;
        while r >= n {
            r = self.getrandbits(k) as u64;
        }
        r
    }

    /// `random.randint(a, b)`
    pub fn randint(&mut self, a: i64, b: i64) -> i64 {
        a + self.randbelow((b - a + 1) as u64) as i64
    }

    /// `random.choice(seq)`
    pub fn choice<'a, T>(&mut self, seq: &'a [T]) -> &'a T {
        &seq[self.randbelow(seq.len() as u64) as usize]
    }

    /// `random.uniform(a, b)`
    pub fn uniform(&mut self, a: f64, b: f64) -> f64 {
        a + (b - a) * self.random()
    }

    /// `random.gauss(mu, sigma)` (CPython 3.12: Box-Muller, keeps the second value for the next call)
    pub fn gauss(&mut self, mu: f64, sigma: f64) -> f64 {
        let z = match self.gauss_next.take() {
            Some(z) => z,
            None => {
                let x2pi = self.random() * (2.0 * std::f64::consts::PI);
                let g2rad = (-2.0 * (1.0 - self.random()).ln()).sqrt();
                self.gauss_next = Some(x2pi.sin() * g2rad);
                x2pi.cos() * g2rad
            }
        };
        mu + z * sigma
    }
}
