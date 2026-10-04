//! Small shared helpers: isometric projection, hashing, colors, randomness.

use macroquad::prelude::*;

/// Half the width / height of one isometric floor tile, in low-res pixels.
pub const HALF_W: f32 = 16.0;
pub const HALF_H: f32 = 8.0;

/// World (tile units) -> screen pixels on the floor plane.
#[inline]
pub fn iso(x: f32, y: f32) -> Vec2 {
    vec2((x - y) * HALF_W, (x + y) * HALF_H)
}

/// World -> screen with a height (z, in pixels) above the floor.
#[inline]
pub fn iso3(x: f32, y: f32, z: f32) -> Vec2 {
    vec2((x - y) * HALF_W, (x + y) * HALF_H - z)
}

/// A direction on screen -> the matching direction on the floor plane.
pub fn screen_to_world_dir(d: Vec2) -> Vec2 {
    vec2(d.x / HALF_W + d.y / HALF_H, d.y / HALF_H - d.x / HALF_W) * 0.5
}

/// A direction on the floor plane -> screen direction.
pub fn world_to_screen_dir(d: Vec2) -> Vec2 {
    vec2((d.x - d.y) * HALF_W, (d.x + d.y) * HALF_H)
}

/// Deterministic 0..1 hash, used for all the procedural texture noise.
#[inline]
pub fn hash2(x: i32, y: i32, seed: u32) -> f32 {
    let mut h = (x as u32).wrapping_mul(374_761_393)
        ^ (y as u32).wrapping_mul(668_265_263)
        ^ seed.wrapping_mul(2_246_822_519);
    h = (h ^ (h >> 13)).wrapping_mul(1_274_126_177);
    h ^= h >> 16;
    (h & 0x00ff_ffff) as f32 / 16_777_216.0
}

pub fn rgb(r: u8, g: u8, b: u8) -> Color {
    Color::from_rgba(r, g, b, 255)
}

pub fn shade(c: Color, k: f32) -> Color {
    Color::new((c.r * k).min(1.0), (c.g * k).min(1.0), (c.b * k).min(1.0), c.a)
}

pub fn mix(a: Color, b: Color, t: f32) -> Color {
    Color::new(
        a.r + (b.r - a.r) * t,
        a.g + (b.g - a.g) * t,
        a.b + (b.b - a.b) * t,
        a.a + (b.a - a.a) * t,
    )
}

pub fn with_alpha(c: Color, a: f32) -> Color {
    Color::new(c.r, c.g, c.b, a)
}

#[inline]
pub fn rnd(a: f32, b: f32) -> f32 {
    rand::gen_range(a, b)
}

#[inline]
pub fn chance(p: f32) -> bool {
    rand::gen_range(0.0, 1.0) < p
}

pub fn rand_dir() -> Vec2 {
    let a = rnd(0.0, std::f32::consts::TAU);
    vec2(a.cos(), a.sin())
}

pub fn smoothstep(e0: f32, e1: f32, x: f32) -> f32 {
    let t = ((x - e0) / (e1 - e0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

pub fn approach(cur: f32, target: f32, rate: f32) -> f32 {
    if cur < target { (cur + rate).min(target) } else { (cur - rate).max(target) }
}
