//! Procedural audio. Every sound effect and music loop is synthesized here at
//! startup and handed to macroquad as an in-memory WAV. No audio files.

use std::f32::consts::{PI, TAU};

pub const SR: f32 = 44100.0;

// ---------------------------------------------------------------- building blocks

pub struct Rng(u32);
impl Rng {
    pub fn new(seed: u32) -> Self {
        Rng(seed.wrapping_mul(747_796_405).wrapping_add(2_891_336_453) | 1)
    }
    /// White noise in -1..1
    pub fn noise(&mut self) -> f32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 17;
        self.0 ^= self.0 << 5;
        (self.0 as f32 / u32::MAX as f32) * 2.0 - 1.0
    }
    pub fn range(&mut self, a: f32, b: f32) -> f32 {
        a + (self.noise() * 0.5 + 0.5) * (b - a)
    }
}

#[derive(Default)]
struct Lp(f32);
impl Lp {
    fn run(&mut self, x: f32, fc: f32) -> f32 {
        let a = 1.0 - (-TAU * fc.max(1.0) / SR).exp();
        self.0 += a * (x - self.0);
        self.0
    }
}

#[derive(Default)]
struct Hp(Lp);
impl Hp {
    fn run(&mut self, x: f32, fc: f32) -> f32 {
        x - self.0.run(x, fc)
    }
}

/// RBJ biquad, recomputed per call so cutoffs can sweep.
#[derive(Default)]
struct Bq {
    x1: f32,
    x2: f32,
    y1: f32,
    y2: f32,
}
impl Bq {
    fn process(&mut self, x: f32, b0: f32, b1: f32, b2: f32, a0: f32, a1: f32, a2: f32) -> f32 {
        let y = (b0 * x + b1 * self.x1 + b2 * self.x2 - a1 * self.y1 - a2 * self.y2) / a0;
        self.x2 = self.x1;
        self.x1 = x;
        self.y2 = self.y1;
        self.y1 = y;
        y
    }
    fn bandpass(&mut self, x: f32, fc: f32, q: f32) -> f32 {
        let w = TAU * fc.clamp(20.0, SR * 0.45) / SR;
        let alpha = w.sin() / (2.0 * q);
        self.process(x, alpha, 0.0, -alpha, 1.0 + alpha, -2.0 * w.cos(), 1.0 - alpha)
    }
    fn lowpass(&mut self, x: f32, fc: f32, q: f32) -> f32 {
        let w = TAU * fc.clamp(20.0, SR * 0.45) / SR;
        let (c, alpha) = (w.cos(), w.sin() / (2.0 * q));
        let b1 = 1.0 - c;
        self.process(x, b1 * 0.5, b1, b1 * 0.5, 1.0 + alpha, -2.0 * c, 1.0 - alpha)
    }
}

fn saw(ph: f32) -> f32 {
    2.0 * (ph - ph.floor()) - 1.0
}
fn sq(ph: f32, duty: f32) -> f32 {
    if ph - ph.floor() < duty { 1.0 } else { -1.0 }
}
fn bell_env(t: f32, len: f32) -> f32 {
    if t < 0.0 || t > len { 0.0 } else { (PI * t / len).sin() }
}
fn attack(t: f32, a: f32) -> f32 {
    (t / a).min(1.0)
}
fn note(semis_from_a4: f32) -> f32 {
    440.0 * 2f32.powf(semis_from_a4 / 12.0)
}

fn buf(secs: f32) -> Vec<f32> {
    vec![0.0; (secs * SR) as usize]
}

fn normalize(b: &mut [f32], peak: f32) {
    let m = b.iter().fold(0.0f32, |m, x| m.max(x.abs()));
    if m > 1e-6 {
        let k = peak / m;
        b.iter_mut().for_each(|x| *x *= k);
    }
}

fn drive(b: &mut [f32], amount: f32) {
    b.iter_mut().for_each(|x| *x = (*x * amount).tanh());
}

/// Small Schroeder reverb (4 combs + 2 allpasses). `size` scales the delay lines.
fn reverb(b: &[f32], mix: f32, size: f32, fb: f32) -> Vec<f32> {
    let combs = [0.0297, 0.0371, 0.0411, 0.0437].map(|d| ((d * size * SR) as usize).max(1));
    let aps = [0.005, 0.0017].map(|d| ((d * SR) as usize).max(1));
    let mut lines: Vec<Vec<f32>> = combs.iter().map(|&n| vec![0.0; n]).collect();
    let mut idx = [0usize; 4];
    let mut damp = [0.0f32; 4];
    let mut out = Vec::with_capacity(b.len());
    for &x in b {
        let mut acc = 0.0;
        for k in 0..4 {
            let y = lines[k][idx[k]];
            damp[k] = y * 0.6 + damp[k] * 0.4;
            lines[k][idx[k]] = x + damp[k] * fb;
            idx[k] = (idx[k] + 1) % lines[k].len();
            acc += y;
        }
        out.push(acc * 0.25);
    }
    for &n in &aps {
        let mut line = vec![0.0; n];
        let mut i = 0;
        for s in out.iter_mut() {
            let d = line[i];
            let y = -*s * 0.5 + d;
            line[i] = *s + d * 0.5;
            i = (i + 1) % n;
            *s = y;
        }
    }
    b.iter().zip(out).map(|(d, w)| d * (1.0 - mix * 0.5) + w * mix).collect()
}

/// Fold the tail of a buffer back onto its start so it loops seamlessly.
fn wrap_loop(mut b: Vec<f32>, loop_len: usize) -> Vec<f32> {
    if b.len() > loop_len {
        let tail: Vec<f32> = b[loop_len..].to_vec();
        b.truncate(loop_len);
        for (i, t) in tail.iter().enumerate() {
            b[i % loop_len] += t;
        }
    }
    b
}

/// 16-bit PCM WAV. `ch` channels, samples interleaved.
pub fn wav(samples: &[f32], ch: u16) -> Vec<u8> {
    let data_len = (samples.len() * 2) as u32;
    let mut v = Vec::with_capacity(44 + data_len as usize);
    let sr = SR as u32;
    v.extend_from_slice(b"RIFF");
    v.extend_from_slice(&(36 + data_len).to_le_bytes());
    v.extend_from_slice(b"WAVEfmt ");
    v.extend_from_slice(&16u32.to_le_bytes());
    v.extend_from_slice(&1u16.to_le_bytes());
    v.extend_from_slice(&ch.to_le_bytes());
    v.extend_from_slice(&sr.to_le_bytes());
    v.extend_from_slice(&(sr * ch as u32 * 2).to_le_bytes());
    v.extend_from_slice(&(ch * 2).to_le_bytes());
    v.extend_from_slice(&16u16.to_le_bytes());
    v.extend_from_slice(b"data");
    v.extend_from_slice(&data_len.to_le_bytes());
    for s in samples {
        let i = (s.clamp(-1.0, 1.0) * 32767.0) as i16;
        v.extend_from_slice(&i.to_le_bytes());
    }
    v
}

fn interleave(l: &[f32], r: &[f32]) -> Vec<f32> {
    l.iter().zip(r).flat_map(|(a, b)| [*a, *b]).collect()
}

// ---------------------------------------------------------------- weapons & impacts

pub fn shot(seed: u32, pitch: f32) -> Vec<f32> {
    let mut b = buf(0.3);
    let mut r = Rng::new(seed);
    let (mut lp, mut hp, mut ph) = (Lp::default(), Hp::default(), 0.0f32);
    for (i, s) in b.iter_mut().enumerate() {
        let t = i as f32 / SR;
        let n = r.noise();
        let crack = hp.run(lp.run(n, 2200.0 + 9000.0 * (-t * 30.0).exp()), 380.0) * (-t * 28.0).exp();
        ph += (48.0 + pitch * (-t * 24.0).exp()) / SR;
        let thump = (TAU * ph).sin() * (-t * 17.0).exp() * 0.9;
        let click = if t < 0.0025 { n } else { 0.0 };
        *s = crack * 1.1 + thump + click;
    }
    let mut b = reverb(&b, 0.2, 0.7, 0.55);
    drive(&mut b, 1.7);
    normalize(&mut b, 0.92);
    b
}

pub fn tick(seed: u32) -> Vec<f32> {
    let mut b = buf(0.08);
    let mut r = Rng::new(seed);
    let mut hp = Hp::default();
    let f = r.range(2600.0, 3600.0);
    for (i, s) in b.iter_mut().enumerate() {
        let t = i as f32 / SR;
        *s = hp.run(r.noise(), 2500.0) * (-t * 120.0).exp() + (TAU * f * t).sin() * (-t * 45.0).exp() * 0.35;
    }
    normalize(&mut b, 0.7);
    b
}

pub fn hit(seed: u32) -> Vec<f32> {
    let mut b = buf(0.14);
    let mut r = Rng::new(seed);
    let mut lp = Lp::default();
    let f = r.range(80.0, 110.0);
    for (i, s) in b.iter_mut().enumerate() {
        let t = i as f32 / SR;
        *s = lp.run(r.noise(), 900.0) * (-t * 38.0).exp() * 1.6 + (TAU * f * t).sin() * (-t * 34.0).exp();
    }
    drive(&mut b, 1.4);
    normalize(&mut b, 0.8);
    b
}

pub fn splat(seed: u32) -> Vec<f32> {
    let mut b = buf(0.5);
    let mut r = Rng::new(seed);
    let mut bp = Bq::default();
    let mut ph = 0.0f32;
    let drops: Vec<(f32, f32)> = (0..7).map(|_| (r.range(0.0, 0.28), r.range(220.0, 520.0))).collect();
    for (i, s) in b.iter_mut().enumerate() {
        let t = i as f32 / SR;
        let squelch = bp.bandpass(r.noise(), 900.0 * (-t * 3.5).exp() + 220.0, 1.4) * (-t * 7.0).exp() * 2.2;
        ph += (48.0 + 90.0 * (-t * 14.0).exp()) / SR;
        let thump = (TAU * ph).sin() * (-t * 13.0).exp() * 0.9;
        let mut bubbles = 0.0;
        for &(t0, f) in &drops {
            if t > t0 {
                let dt = t - t0;
                bubbles += (TAU * f * dt * (1.0 + dt * 6.0)).sin() * (-dt * 45.0).exp() * 0.35;
            }
        }
        *s = squelch + thump + bubbles;
    }
    drive(&mut b, 1.5);
    normalize(&mut b, 0.85);
    b
}

pub fn explosion(seed: u32, heavy: bool) -> Vec<f32> {
    let len = if heavy { 1.4 } else { 2.0 };
    let mut b = buf(len);
    let mut r = Rng::new(seed);
    let (mut l1, mut l2) = (Lp::default(), Lp::default());
    let mut ph = 0.0f32;
    let top = if heavy { 2200.0 } else { 5500.0 };
    for (i, s) in b.iter_mut().enumerate() {
        let t = i as f32 / SR;
        let mut n = r.noise();
        if r.noise() > 0.996 - 0.01 * (-t * 3.0).exp() {
            n *= 6.0 * (-t * 2.0).exp(); // crackle
        }
        let cut = 140.0 + top * (-t * 4.5).exp();
        let body = l2.run(l1.run(n, cut), cut) * (-t * 2.6).exp() * attack(t, 0.004) * 2.0;
        ph += (28.0 + 62.0 * (-t * 5.5).exp()) / SR;
        let boom = (TAU * ph).sin() * (-t * if heavy { 2.8 } else { 3.6 }).exp() * if heavy { 1.6 } else { 1.2 };
        *s = body + boom;
    }
    let mut b = reverb(&b, 0.38, 1.6, 0.7);
    drive(&mut b, 1.8);
    normalize(&mut b, 0.95);
    b
}

pub fn whoosh(seed: u32, len: f32, f0: f32, f1: f32) -> Vec<f32> {
    let mut b = buf(len);
    let mut r = Rng::new(seed);
    let mut bp = Bq::default();
    for (i, s) in b.iter_mut().enumerate() {
        let t = i as f32 / SR;
        let k = t / len;
        *s = bp.bandpass(r.noise(), f0 * (f1 / f0).powf(k), 1.6) * bell_env(t, len).powf(0.7);
    }
    normalize(&mut b, 0.8);
    b
}

pub fn throw(seed: u32) -> Vec<f32> {
    let mut b = whoosh(seed, 0.32, 500.0, 1600.0);
    let mut r = Rng::new(seed + 9);
    let mut hp = Hp::default();
    for (i, s) in b.iter_mut().enumerate().take((0.06 * SR) as usize) {
        let t = i as f32 / SR;
        let c = |t0: f32| if t > t0 && t < t0 + 0.004 { 1.0 } else { 0.0 };
        *s = *s * 0.6 + hp.run(r.noise(), 2000.0) * (c(0.0) + c(0.045)) * 0.9;
    }
    normalize(&mut b, 0.8);
    b
}

// ---------------------------------------------------------------- voices

fn vocal(seed: u32, len: f32, f0: f32, f_end: f32, vib: (f32, f32), f1: f32, f2: f32, grit: f32, dist: f32) -> Vec<f32> {
    let mut b = buf(len);
    let mut r = Rng::new(seed);
    let (mut b1, mut b2, mut b3) = (Bq::default(), Bq::default(), Bq::default());
    let mut lp = Lp::default();
    let mut ph = 0.0f32;
    let mut jitter = 0.0f32;
    for (i, s) in b.iter_mut().enumerate() {
        let t = i as f32 / SR;
        let k = t / len;
        jitter = jitter * 0.999 + r.noise() * 0.001;
        let f = (f0 + (f_end - f0) * k) * (1.0 + vib.1 * (TAU * vib.0 * t).sin() + jitter * 3.0);
        ph += f / SR;
        let src = saw(ph) + saw(ph * 1.006) * 0.5;
        let n = r.noise();
        let v = b1.bandpass(src, f1, 5.0) + b2.bandpass(src, f2, 6.0) * 0.6 + b3.bandpass(n, 1500.0, 0.9) * grit;
        let env = attack(t, len * 0.2) * ((len - t) / (len * 0.35)).clamp(0.0, 1.0);
        *s = lp.run(v, 3000.0) * env;
    }
    drive(&mut b, dist);
    normalize(&mut b, 0.85);
    b
}

pub fn groan(seed: u32, f0: f32, f1: f32, f2: f32) -> Vec<f32> {
    vocal(seed, 1.5, f0, f0 * 0.8, (5.5, 0.03), f1, f2, 0.25, 1.6)
}
pub fn shriek(seed: u32, f0: f32) -> Vec<f32> {
    vocal(seed, 0.75, f0, f0 * 1.25, (11.0, 0.05), 850.0, 1700.0, 0.5, 2.4)
}
pub fn growl(seed: u32) -> Vec<f32> {
    vocal(seed, 1.3, 62.0, 52.0, (8.0, 0.06), 480.0, 950.0, 0.5, 2.6)
}
pub fn hurt(seed: u32) -> Vec<f32> {
    let mut b = vocal(seed, 0.3, 175.0, 115.0, (9.0, 0.02), 650.0, 1100.0, 0.3, 2.0);
    let h = hit(seed + 3);
    for (s, x) in b.iter_mut().zip(h) {
        *s = *s * 0.8 + x * 0.6;
    }
    normalize(&mut b, 0.85);
    b
}
pub fn roar(seed: u32) -> Vec<f32> {
    let a = vocal(seed, 2.3, 72.0, 38.0, (7.0, 0.07), 600.0, 1150.0, 0.6, 3.2);
    let c = vocal(seed + 1, 2.3, 36.0, 24.0, (6.0, 0.05), 350.0, 2200.0, 0.3, 3.0);
    let mix: Vec<f32> = a.iter().zip(&c).map(|(x, y)| x + y * 0.7).collect();
    let mut b = reverb(&mix, 0.35, 1.8, 0.7);
    normalize(&mut b, 0.95);
    b
}

pub fn caw(seed: u32, double: bool) -> Vec<f32> {
    let one = vocal(seed, 0.3, 560.0, 450.0, (26.0, 0.035), 1300.0, 2500.0, 0.8, 3.4);
    let mut b = buf(if double { 0.75 } else { 0.4 });
    for (i, x) in one.iter().enumerate() {
        b[i] += x;
        if double {
            let j = i + (0.38 * SR) as usize;
            if j < b.len() {
                b[j] += x * 0.85;
            }
        }
    }
    let mut b = reverb(&b, 0.25, 1.3, 0.55);
    normalize(&mut b, 0.8);
    b
}

pub fn zap(seed: u32) -> Vec<f32> {
    let mut b = buf(0.35);
    let mut r = Rng::new(seed);
    let mut bp = Bq::default();
    let mut gate = 1.0f32;
    for (i, s) in b.iter_mut().enumerate() {
        let t = i as f32 / SR;
        if i % 400 == 0 {
            gate = if r.noise() > -0.2 { 1.0 } else { 0.15 };
        }
        let crackle = bp.bandpass(r.noise(), 3200.0, 1.2) * gate * 2.0;
        let buzz = sq(100.0 * t, 0.5) * 0.25 + (TAU * 200.0 * t).sin() * 0.2;
        *s = (crackle + buzz) * (-t * 9.0).exp();
    }
    drive(&mut b, 1.6);
    normalize(&mut b, 0.7);
    b
}

// ---------------------------------------------------------------- tones, bells, jingles

/// FM bell: `ratio` sets how metallic it is.
fn bell(b: &mut [f32], t0: f32, f: f32, amp: f32, ratio: f32, index: f32, decay: f32) {
    let start = (t0 * SR) as usize;
    for i in start..b.len() {
        let t = (i - start) as f32 / SR;
        let env = (-t * decay).exp();
        if env < 0.0005 {
            break;
        }
        let m = (TAU * f * ratio * t).sin() * index * (-t * decay * 2.0).exp();
        b[i] += (TAU * f * t + m).sin() * env * amp * attack(t, 0.002);
    }
}

pub fn chime() -> Vec<f32> {
    // the two-tone "pin-pon" before an announcement
    let mut b = buf(2.0);
    bell(&mut b, 0.0, note(7.0), 0.8, 2.0, 1.6, 2.6);
    bell(&mut b, 0.42, note(3.0), 0.8, 2.0, 1.6, 2.4);
    let mut b = reverb(&b, 0.3, 1.4, 0.6);
    normalize(&mut b, 0.8);
    b
}

pub fn clear_jingle() -> Vec<f32> {
    // an original little departure-melody style phrase in D major
    let mut b = buf(3.4);
    let beat = 0.19;
    let seq: [(f32, f32); 9] = [(17.0, 0.0), (21.0, 1.0), (24.0, 2.0), (21.0, 3.0), (19.0, 4.0), (14.0, 5.0), (17.0, 6.0), (16.0, 7.0), (17.0, 8.0)];
    for (n, at) in seq {
        bell(&mut b, at * beat, note(n), 0.6, 3.5, 1.2, 4.0);
    }
    bell(&mut b, 8.0 * beat, note(12.0), 0.35, 3.5, 1.0, 2.0);
    bell(&mut b, 8.0 * beat, note(9.0), 0.3, 3.5, 1.0, 2.0);
    let mut b = reverb(&b, 0.3, 1.3, 0.6);
    normalize(&mut b, 0.8);
    b
}

pub fn pickup() -> Vec<f32> {
    let mut b = buf(0.5);
    for (k, n) in [15.0f32, 19.0, 22.0, 27.0].iter().enumerate() {
        let t0 = k as f32 * 0.06;
        let f = note(*n);
        let start = (t0 * SR) as usize;
        for i in start..b.len() {
            let t = (i - start) as f32 / SR;
            b[i] += sq(f * t, 0.25) * (-t * if k == 3 { 9.0 } else { 22.0 }).exp() * 0.3;
        }
    }
    let mut b = reverb(&b, 0.2, 0.8, 0.5);
    normalize(&mut b, 0.7);
    b
}

pub fn select() -> Vec<f32> {
    let mut b = buf(0.12);
    for (i, s) in b.iter_mut().enumerate() {
        let t = i as f32 / SR;
        let f = if t < 0.05 { 880.0 } else { 1320.0 };
        *s = sq(f * t, 0.5) * (-t * 18.0).exp() * 0.4;
    }
    normalize(&mut b, 0.6);
    b
}

pub fn horn() -> Vec<f32> {
    // short-long electronic horn, like a commuter train
    let mut b = buf(1.6);
    let mut lp = Bq::default();
    let (mut p1, mut p2) = (0.0f32, 0.0f32);
    for (i, s) in b.iter_mut().enumerate() {
        let t = i as f32 / SR;
        let gate = |a: f32, z: f32| attack(t - a, 0.015).max(0.0) * ((z - t) / 0.06).clamp(0.0, 1.0) * if t >= a { 1.0 } else { 0.0 };
        let env = gate(0.0, 0.17) + gate(0.24, 1.35);
        let vib = 1.0 + 0.004 * (TAU * 5.0 * t).sin();
        p1 += 370.0 * vib / SR;
        p2 += 466.0 * vib / SR;
        let v = saw(p1) + saw(p2) * 0.8 + sq(p1 * 0.5, 0.5) * 0.3;
        *s = lp.lowpass(v, 2400.0, 0.9) * env;
    }
    drive(&mut b, 1.6);
    let mut b = reverb(&b, 0.35, 1.7, 0.65);
    normalize(&mut b, 0.85);
    b
}

fn clack(b: &mut [f32], t0: f32, amp: f32, r: &mut Rng) {
    let start = (t0 * SR) as usize;
    let mut hp = Hp::default();
    for i in start..(start + (0.06 * SR) as usize).min(b.len()) {
        let t = (i - start) as f32 / SR;
        b[i] += (hp.run(r.noise(), 300.0) * (-t * 90.0).exp() * 0.8 + (TAU * 210.0 * t).sin() * (-t * 50.0).exp() * 0.7) * amp;
    }
}

pub fn train_arrive(seed: u32) -> Vec<f32> {
    let len = 5.2;
    let mut b = buf(len);
    let mut r = Rng::new(seed);
    let (mut l1, mut l2) = (Lp::default(), Lp::default());
    let mut hp = Hp::default();
    let stop = 3.7;
    let v = |t: f32| if t < stop { 22.0 * (1.0 - t / stop).powf(0.9) } else { 0.0 };
    // every bogie of every car rolls over the rail joint by the platform edge:
    // cars are 20 m long with bogie pairs at 0/2.5 m and 15/17.5 m
    let mut marks: Vec<f32> = Vec::new();
    for car in 0..10 {
        for off in [0.0, 2.5, 15.0, 17.5] {
            marks.push(car as f32 * 20.0 + off);
        }
    }
    let mut x = 0.0f32;
    let mut m = 0;
    let dt = 1.0 / SR;
    let mut clacks = Vec::new();
    let mut t = 0.0;
    while t < stop && m < marks.len() {
        x += v(t) * dt;
        while m < marks.len() && x >= marks[m] {
            clacks.push((t, v(t) / 22.0));
            m += 1;
        }
        t += dt;
    }
    for (t0, k) in clacks {
        if t0 < len {
            clack(&mut b, t0, 0.25 + 0.6 * k, &mut r);
        }
    }
    let mut sq_ph = 0.0f32;
    let mut flutter = 1.0f32;
    for (i, s) in b.iter_mut().enumerate() {
        let t = i as f32 / SR;
        let k = v(t) / 22.0;
        let rumble = l2.run(l1.run(r.noise(), 110.0 + 220.0 * k), 160.0 + 260.0 * k) * (0.25 + k) * attack(t, 1.2) * 3.0;
        let low = (TAU * 42.0 * t).sin() * k * 0.25;
        flutter = (flutter + r.noise() * 0.02).clamp(0.4, 1.0);
        sq_ph += (2900.0 + 70.0 * (TAU * 7.0 * t).sin()) / SR;
        let squeal = ((TAU * sq_ph).sin() + (TAU * sq_ph * 1.5).sin() * 0.3) * bell_env(t - 2.0, 1.75) * 0.22 * flutter;
        let hiss = if t > stop + 0.1 { hp.run(r.noise(), 3200.0) * (-(t - stop - 0.1) * 2.2).exp() * 0.35 } else { 0.0 };
        *s += rumble + low + squeal + hiss;
    }
    let mut b = reverb(&b, 0.3, 1.6, 0.6);
    normalize(&mut b, 0.9);
    b
}

pub fn game_over() -> Vec<f32> {
    let mut b = buf(3.6);
    for (k, n) in [5.0f32, 0.0, -4.0, -7.0].iter().enumerate() {
        bell(&mut b, k as f32 * 0.42, note(*n), 0.5, 1.41, 2.0, 1.4);
    }
    let mut ph = 0.0f32;
    for (i, s) in b.iter_mut().enumerate() {
        let t = i as f32 / SR;
        ph += note(-31.0) / SR;
        *s += saw(ph) * 0.12 * attack(t, 0.8) * (-(t - 1.0).max(0.0) * 1.0).exp();
    }
    let mut b = reverb(&b, 0.45, 1.9, 0.75);
    normalize(&mut b, 0.8);
    b
}

pub fn victory() -> Vec<f32> {
    let mut b = buf(4.0);
    let seq = [(5.0f32, 0.0f32), (9.0, 0.15), (12.0, 0.3), (17.0, 0.45), (21.0, 0.75), (24.0, 0.9)];
    for (n, at) in seq {
        bell(&mut b, at, note(n), 0.55, 3.5, 1.2, 2.5);
    }
    for n in [5.0f32, 9.0, 12.0, 17.0] {
        bell(&mut b, 1.2, note(n), 0.35, 2.0, 0.8, 0.9);
    }
    let mut b = reverb(&b, 0.4, 1.8, 0.7);
    normalize(&mut b, 0.8);
    b
}

// ---------------------------------------------------------------- loops

/// Fluorescent hum. Tokyo's grid runs at 50 Hz, so the tubes buzz at 100 Hz.
pub fn hum() -> Vec<f32> {
    let n = (4.0 * SR) as usize;
    let mut b = vec![0.0; n + (0.5 * SR) as usize];
    let mut r = Rng::new(42);
    let (mut l1, mut l2) = (Lp::default(), Lp::default());
    for (i, s) in b.iter_mut().enumerate() {
        let t = i as f32 / SR;
        let tone = (TAU * 100.0 * t).sin() * 0.3 + (TAU * 200.0 * t).sin() * 0.16 + (TAU * 300.0 * t).sin() * 0.07 + sq(100.0 * t, 0.5) * 0.04;
        let air = l1.run(r.noise(), 350.0) * 0.6 + l2.run(r.noise(), 70.0) * 1.4;
        *s = tone + air;
    }
    // crossfade the extra half second over the start
    let x = b.len() - n;
    for i in 0..x {
        let k = i as f32 / x as f32;
        b[i] = b[i] * k + b[n + i] * (1.0 - k);
    }
    b.truncate(n);
    normalize(&mut b, 0.6);
    b
}

fn pad_voice(out: &mut [f32], t0: f32, len: f32, f: f32, detune: f32, amp: f32, seed: u32) {
    let start = (t0 * SR) as usize;
    let mut lp = Bq::default();
    let mut r = Rng::new(seed);
    let (mut a, mut b) = (r.range(0.0, 1.0), r.range(0.0, 1.0));
    let n = ((len + 0.6) * SR) as usize;
    for i in 0..n {
        let idx = start + i;
        if idx >= out.len() {
            break;
        }
        let t = i as f32 / SR;
        a += f * (1.0 + detune) / SR;
        b += f * (1.0 - detune) / SR;
        let env = attack(t, 0.5) * ((len + 0.6 - t) / 0.6).clamp(0.0, 1.0);
        let cut = 500.0 + 350.0 * (TAU * 0.25 * t).sin();
        out[idx] += lp.lowpass(saw(a) + saw(b), cut, 0.8) * env * amp;
    }
}

/// Tense combat loop: 120 bpm, 4 bars of D minor, stereo.
pub fn combat_music() -> (Vec<f32>, Vec<f32>) {
    let step = 0.125;
    let loop_len = (64.0 * step * SR) as usize;
    let total = loop_len + (1.5 * SR) as usize;
    let (mut l, mut rgt) = (vec![0.0f32; total], vec![0.0f32; total]);
    let mut r = Rng::new(7);
    let d2 = -31.0; // D2 relative to A4
    let rest = 99.0;
    let bars: [[f32; 16]; 4] = [
        [0.0, rest, 0.0, rest, rest, 0.0, 3.0, rest, 0.0, rest, -2.0, rest, 0.0, rest, rest, rest],
        [0.0, rest, 0.0, rest, rest, 0.0, 3.0, rest, 0.0, rest, -2.0, rest, 0.0, rest, 5.0, rest],
        [-4.0, rest, -4.0, rest, rest, -4.0, 0.0, rest, -4.0, rest, -2.0, rest, -4.0, rest, rest, rest],
        [-5.0, rest, -5.0, rest, rest, -5.0, -1.0, rest, -5.0, rest, -1.0, rest, 2.0, rest, 3.0, rest],
    ];
    let chords: [[f32; 3]; 4] = [[12.0, 15.0, 19.0], [12.0, 15.0, 19.0], [8.0, 12.0, 15.0], [7.0, 11.0, 14.0]];
    for bar in 0..4 {
        let bt = bar as f32 * 16.0 * step;
        // pad
        for (k, n) in chords[bar].iter().enumerate() {
            let f = note(d2 + n);
            pad_voice(&mut l, bt, 16.0 * step, f, 0.004, 0.05, (bar * 10 + k) as u32);
            pad_voice(&mut rgt, bt, 16.0 * step, f, 0.006, 0.05, (bar * 10 + k + 5) as u32);
        }
        for s in 0..16 {
            let t0 = bt + s as f32 * step;
            let start = (t0 * SR) as usize;
            // bass
            let n = bars[bar][s];
            if n != rest {
                let f = note(d2 + n);
                let mut lp = Bq::default();
                let mut ph = 0.0f32;
                for i in 0..(0.24 * SR) as usize {
                    let t = i as f32 / SR;
                    ph += f / SR;
                    let v = saw(ph) + sq(ph * 0.5, 0.5) * 0.5;
                    let y = lp.lowpass(v, 180.0 + 1100.0 * (-t * 14.0).exp(), 1.4) * (-t * 7.0).exp() * attack(t, 0.003) * 0.32;
                    l[start + i] += y;
                    rgt[start + i] += y;
                }
            }
            // kick
            let kicks: &[usize] = if bar == 3 { &[0, 3, 6, 8, 10, 12, 14] } else { &[0, 6, 8, 11] };
            if kicks.contains(&s) {
                let mut ph = 0.0f32;
                for i in 0..(0.3 * SR) as usize {
                    let t = i as f32 / SR;
                    ph += (42.0 + 110.0 * (-t * 30.0).exp()) / SR;
                    let y = ((TAU * ph).sin() * (-t * 9.0).exp() * 0.9).tanh() * 0.7;
                    l[start + i] += y;
                    rgt[start + i] += y;
                }
            }
            // snare / clap
            if s == 4 || s == 12 {
                let mut bp = Bq::default();
                for i in 0..(0.25 * SR) as usize {
                    let t = i as f32 / SR;
                    let y = bp.bandpass(r.noise(), 1600.0, 0.8) * (-t * 16.0).exp() * 0.9 + (TAU * 185.0 * t).sin() * (-t * 25.0).exp() * 0.25;
                    l[start + i] += y * 0.8;
                    rgt[start + i] += y;
                }
            }
            // hats
            let hat = if bar == 3 { true } else { s % 2 == 0 };
            if hat {
                let mut hp = Hp::default();
                let acc = if s % 4 == 2 { 0.16 } else { 0.08 };
                let pan = if s % 4 == 2 { 0.7 } else { 1.0 };
                for i in 0..(0.06 * SR) as usize {
                    let t = i as f32 / SR;
                    let y = hp.run(r.noise(), 7000.0) * (-t * 60.0).exp() * acc;
                    l[start + i] += y * pan;
                    rgt[start + i] += y * (1.7 - pan);
                }
            }
        }
        // a distant bell every other bar
        if bar % 2 == 0 {
            bell(&mut l, bt, note(12.0), 0.06, 3.5, 1.0, 1.5);
            bell(&mut rgt, bt + 0.01, note(12.0), 0.06, 3.5, 1.0, 1.5);
        }
    }
    let mut l = wrap_loop(l, loop_len);
    let mut rgt = wrap_loop(rgt, loop_len);
    for v in [&mut l, &mut rgt] {
        drive(v, 1.2);
    }
    let m = l.iter().chain(rgt.iter()).fold(0.0f32, |m, x| m.max(x.abs()));
    for v in [&mut l, &mut rgt] {
        v.iter_mut().for_each(|x| *x *= 0.8 / m.max(1e-6));
    }
    (l, rgt)
}

/// Slow, uneasy pad for the title screen.
pub fn ambient_music() -> (Vec<f32>, Vec<f32>) {
    let loop_len = (12.0 * SR) as usize;
    let total = loop_len + (2.0 * SR) as usize;
    let (mut l, mut r) = (vec![0.0f32; total], vec![0.0f32; total]);
    let d3 = -19.0;
    for (k, (n, t0, len)) in [(0.0f32, 0.0f32, 12.0f32), (7.0, 0.0, 12.0), (15.0, 0.0, 6.0), (14.0, 6.0, 6.0), (19.0, 3.0, 6.0)].iter().enumerate() {
        pad_voice(&mut l, *t0, *len - 0.6, note(d3 + n), 0.003, 0.08, k as u32);
        pad_voice(&mut r, *t0, *len - 0.6, note(d3 + n), 0.005, 0.08, k as u32 + 20);
    }
    for (n, at) in [(29.0f32, 1.0f32), (24.0, 4.2), (27.0, 7.5), (22.0, 10.0)] {
        bell(&mut l, at, note(d3 + n), 0.05, 3.5, 1.0, 1.0);
        bell(&mut r, at + 0.02, note(d3 + n), 0.05, 3.5, 1.0, 1.0);
    }
    let l = reverb(&wrap_loop(l, loop_len), 0.4, 2.0, 0.75);
    let r = reverb(&wrap_loop(r, loop_len), 0.4, 2.1, 0.75);
    let (mut l, mut r) = (wrap_loop(l, loop_len), wrap_loop(r, loop_len));
    let m = l.iter().chain(r.iter()).fold(0.0f32, |m, x| m.max(x.abs()));
    for v in [&mut l, &mut r] {
        v.iter_mut().for_each(|x| *x *= 0.75 / m.max(1e-6));
    }
    (l, r)
}

/// Riding between stations: rumble and the clickety-clack of rail joints.
pub fn ride_loop() -> Vec<f32> {
    let loop_len = (4.0 * SR) as usize;
    let mut b = vec![0.0f32; loop_len + (0.3 * SR) as usize];
    let mut r = Rng::new(11);
    for k in 0..4 {
        let t0 = k as f32;
        clack(&mut b, t0, 0.7, &mut r);
        clack(&mut b, t0 + 0.13, 0.6, &mut r);
    }
    let (mut l1, mut l2) = (Lp::default(), Lp::default());
    for (i, s) in b.iter_mut().enumerate() {
        let t = i as f32 / SR;
        *s += l2.run(l1.run(r.noise(), 140.0), 200.0) * 3.5 + (TAU * 100.0 * t).sin() * 0.04;
    }
    let mut b = wrap_loop(b, loop_len);
    normalize(&mut b, 0.8);
    b
}

// ---------------------------------------------------------------- the whole bank

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Id {
    Shot,
    Tick,
    Hit,
    Splat,
    Groan,
    Shriek,
    Growl,
    Roar,
    Explode,
    Slam,
    Throw,
    Dash,
    Hurt,
    Pickup,
    Chime,
    Clear,
    Horn,
    Train,
    GameOver,
    Victory,
    Select,
    Caw,
    Zap,
    Hum,
    Music,
    Ambient,
    Ride,
}

/// Every sound as (id, wav bytes). Variants share an id.
pub fn bank() -> Vec<(Id, Vec<u8>)> {
    let mono = |b: Vec<f32>| wav(&b, 1);
    let mut v = Vec::new();
    for (k, p) in [170.0, 190.0, 210.0].iter().enumerate() {
        v.push((Id::Shot, mono(shot(100 + k as u32, *p))));
    }
    for k in 0..2 {
        v.push((Id::Tick, mono(tick(200 + k))));
    }
    for k in 0..3 {
        v.push((Id::Hit, mono(hit(300 + k))));
    }
    for k in 0..4 {
        v.push((Id::Splat, mono(splat(400 + k))));
    }
    for (k, (f0, f1, f2)) in [(95.0, 450.0, 800.0), (110.0, 700.0, 1150.0), (85.0, 520.0, 1190.0), (120.0, 330.0, 900.0), (100.0, 600.0, 1000.0)].iter().enumerate() {
        v.push((Id::Groan, mono(groan(500 + k as u32, *f0, *f1, *f2))));
    }
    for (k, f0) in [190.0, 230.0].iter().enumerate() {
        v.push((Id::Shriek, mono(shriek(600 + k as u32, *f0))));
    }
    for k in 0..2 {
        v.push((Id::Growl, mono(growl(700 + k))));
    }
    v.push((Id::Roar, mono(roar(800))));
    for k in 0..2 {
        v.push((Id::Explode, mono(explosion(900 + k, false))));
    }
    v.push((Id::Slam, mono(explosion(950, true))));
    v.push((Id::Throw, mono(throw(1000))));
    v.push((Id::Dash, mono(whoosh(1100, 0.26, 320.0, 2600.0))));
    for k in 0..2 {
        v.push((Id::Hurt, mono(hurt(1200 + k))));
    }
    v.push((Id::Pickup, mono(pickup())));
    v.push((Id::Chime, mono(chime())));
    v.push((Id::Clear, mono(clear_jingle())));
    v.push((Id::Horn, mono(horn())));
    v.push((Id::Train, mono(train_arrive(1300))));
    v.push((Id::GameOver, mono(game_over())));
    v.push((Id::Victory, mono(victory())));
    v.push((Id::Select, mono(select())));
    v.push((Id::Caw, mono(caw(1400, false))));
    v.push((Id::Caw, mono(caw(1401, true))));
    v.push((Id::Zap, mono(zap(1500))));
    v.push((Id::Zap, mono(zap(1501))));
    v.push((Id::Hum, mono(hum())));
    let (l, r) = combat_music();
    v.push((Id::Music, wav(&interleave(&l, &r), 2)));
    let (l, r) = ambient_music();
    v.push((Id::Ambient, wav(&interleave(&l, &r), 2)));
    v.push((Id::Ride, mono(ride_loop())));
    v
}

