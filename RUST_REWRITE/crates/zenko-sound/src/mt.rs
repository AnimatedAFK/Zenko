//! Python's `random.Random(seed)` (MT19937 with CPython's seeding), so every synthesised sound uses the very same noise.

const N: usize = 624;
const M: usize = 397;

pub struct PyRandom {
    mt: [u32; N],
    idx: usize,
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
        if key.is_empty() {
            key.push(0);
        }
        let mut r = PyRandom { mt: [0; N], idx: N };
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

    /// `random.uniform(a, b)`
    pub fn uniform(&mut self, a: f64, b: f64) -> f64 {
        a + (b - a) * self.random()
    }
}
