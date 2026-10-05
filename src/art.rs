//! Every sprite in the game is generated here, pixel by pixel, at startup.
//! Everything lands in one texture atlas so the renderer can batch draws.

use crate::font;
use crate::util::*;
use macroquad::prelude::*;
use std::collections::HashMap;

const ATLAS: u32 = 2048;
const OUTLINE: Color = Color { r: 0.05, g: 0.035, b: 0.06, a: 1.0 };

/// A sprite in the atlas. `anchor` is the pixel that sits on the object's ground point.
#[derive(Clone, Copy, Debug)]
pub struct Spr {
    pub r: Rect,
    pub anchor: Vec2,
}

/// Walk-cycle frames for one character: [view][frame], view 0 = facing camera, 1 = facing away.
pub struct CharArt {
    pub frames: [[Spr; 4]; 2],
    pub flash: [[Spr; 4]; 2],
}

pub struct Art {
    pub tex: Texture2D,
    pub white: Rect,
    pub soft: Rect,
    pub shadow: Rect,
    pub ring: Rect,
    pub glyphs: HashMap<char, Rect>,
    pub player: CharArt,
    pub zombies: Vec<Vec<CharArt>>, // [kind][variant]
    pub boss: CharArt,
    pub gun: Spr,
    pub pillar: Spr,
    pub vending: Vec<Spr>,
    pub bench: Spr,
    pub gate: Spr,
    pub bin: Spr,
    pub train_body: Spr,
    pub train_door: Spr,
    pub train_door_open: Spr,
    pub train_gap: Spr,
    pub train_cab: Spr,
    pub onigiri: Spr,
    pub grenade: Rect,
    pub casing: Rect,
    pub psd: [Spr; 2],
    pub suitcases: Vec<Spr>,
    pub boxes: Spr,
    pub kiosk: Spr,
    pub barrier: Spr,
    pub crow: [Spr; 4], // stand, peck, wings up, wings down
    /// Ordinary commuters: walk cycles, holding a strap, sitting, and on their knees.
    pub commuters: Vec<CharArt>,
    pub strap: Vec<Spr>,
    pub sit: Vec<Spr>,
    pub kneel: Spr,
    pub officer: CharArt,
    pub pistol: Spr,
    pub esc_step: Vec<Spr>, // escalator slices, rising
    pub esc_rail: Vec<Spr>,
    pub cabinet: Spr,
    pub car_seat: Spr,
    pub car_wall: Spr,
    pub car_door: Spr,
    pub straps: Spr,
    pub ads: Vec<Spr>,
    pub maze: Vec<Spr>,
    pub coin: Rect,
}

pub const ESC_SLICES: usize = 14;

pub fn esc_height(i: usize) -> f32 {
    3.0 + i as f32 * 3.0
}

/// Station-specific hanging signs, built per map (they carry the station's name).
pub struct SignArt {
    pub tex: Texture2D,
    pub name: Spr,
    pub led: [Spr; 2], // two frames so the LED board can blink
}

// ---------------------------------------------------------------- atlas packing

struct Packer {
    img: Image,
    x: u32,
    y: u32,
    row_h: u32,
    size: u32,
}

impl Packer {
    fn new() -> Self {
        Self::new_sized(ATLAS)
    }
    fn new_sized(size: u32) -> Self {
        Packer { img: Image::gen_image_color(size as u16, size as u16, Color::new(0., 0., 0., 0.)), x: 1, y: 1, row_h: 0, size }
    }
    fn add(&mut self, src: &Image) -> Rect {
        let (w, h) = (src.width as u32, src.height as u32);
        if self.x + w + 2 > self.size {
            self.x = 1;
            self.y += self.row_h + 2;
            self.row_h = 0;
        }
        assert!(self.y + h < self.size, "atlas full");
        for yy in 0..h {
            for xx in 0..w {
                let c = src.get_pixel(xx, yy);
                if c.a > 0.0 {
                    self.img.set_pixel(self.x + xx, self.y + yy, c);
                }
            }
        }
        let r = Rect::new(self.x as f32, self.y as f32, w as f32, h as f32);
        self.x += w + 2;
        self.row_h = self.row_h.max(h);
        r
    }
}

// ---------------------------------------------------------------- small pixel canvas

#[derive(Clone)]
struct Cv {
    w: i32,
    h: i32,
    p: Vec<Option<Color>>,
}

impl Cv {
    fn new(w: i32, h: i32) -> Self {
        Cv { w, h, p: vec![None; (w * h) as usize] }
    }
    fn set(&mut self, x: i32, y: i32, c: Color) {
        if x >= 0 && y >= 0 && x < self.w && y < self.h {
            self.p[(y * self.w + x) as usize] = Some(c);
        }
    }
    fn get(&self, x: i32, y: i32) -> Option<Color> {
        if x >= 0 && y >= 0 && x < self.w && y < self.h { self.p[(y * self.w + x) as usize] } else { None }
    }
    fn clear(&mut self, x: i32, y: i32) {
        if x >= 0 && y >= 0 && x < self.w && y < self.h {
            self.p[(y * self.w + x) as usize] = None;
        }
    }
    /// Copy rows [y0, y1) of another canvas into this one, shifted down by `dy`.
    fn blit_rows(&mut self, src: &Cv, y0: i32, y1: i32, dy: i32) {
        for y in y0..y1 {
            for x in 0..src.w {
                if let Some(c) = src.get(x, y) {
                    self.set(x, y + dy, c);
                }
            }
        }
    }
    fn rect(&mut self, x: i32, y: i32, w: i32, h: i32, c: Color) {
        for yy in y..y + h {
            for xx in x..x + w {
                self.set(xx, yy, c);
            }
        }
    }
    /// Returns (sprite with 1px dark outline, white silhouette for hit flashes).
    fn finish(&self) -> (Image, Image) {
        let (w, h) = (self.w + 2, self.h + 2);
        let mut img = Image::gen_image_color(w as u16, h as u16, Color::new(0., 0., 0., 0.));
        let mut sil = img.clone();
        for y in 0..h {
            for x in 0..w {
                let inner = self.get(x - 1, y - 1);
                if let Some(c) = inner {
                    img.set_pixel(x as u32, y as u32, c);
                    sil.set_pixel(x as u32, y as u32, WHITE);
                } else {
                    let near = [(-1, 0), (1, 0), (0, -1), (0, 1)]
                        .iter()
                        .any(|(dx, dy)| self.get(x - 1 + dx, y - 1 + dy).is_some());
                    if near {
                        img.set_pixel(x as u32, y as u32, OUTLINE);
                        sil.set_pixel(x as u32, y as u32, WHITE);
                    }
                }
            }
        }
        (img, sil)
    }
}

// ---------------------------------------------------------------- isometric box rasterizer

#[derive(Clone, Copy, PartialEq)]
pub enum Face {
    Top,
    Left,  // the +y face, facing down-left on screen
    Right, // the +x face, facing down-right on screen
}

/// Rasterize a box of footprint (fx, fy) tiles and `h` pixels tall. The closure gets
/// (face, u, v) where u runs along the face in tiles and v is height in pixels
/// (for the top face: u = x, v = y in tiles). Returning None leaves a hole.
pub fn iso_box(fx: f32, fy: f32, h: f32, shade: impl Fn(Face, f32, f32) -> Option<Color>) -> (Image, Vec2) {
    let minx = -fy * HALF_W;
    let maxx = fx * HALF_W;
    let miny = -h;
    let maxy = (fx + fy) * HALF_H;
    let w = (maxx - minx).ceil() as u32;
    let hh = (maxy - miny).ceil() as u32;
    let mut img = Image::gen_image_color(w as u16, hh as u16, Color::new(0., 0., 0., 0.));
    for iy in 0..hh {
        for ix in 0..w {
            let sx = ix as f32 + 0.5 + minx;
            let sy = iy as f32 + 0.5 + miny;
            // top face
            let a = sx / HALF_W;
            let b = (sy + h) / HALF_H;
            let x = (a + b) * 0.5;
            let y = (b - a) * 0.5;
            let mut col = None;
            if x >= 0.0 && x <= fx && y >= 0.0 && y <= fy {
                col = shade(Face::Top, x, y);
            } else {
                let x = sx / HALF_W + fy;
                let z = (x + fy) * HALF_H - sy;
                if x >= 0.0 && x <= fx && z >= 0.0 && z <= h {
                    col = shade(Face::Left, x, z);
                } else {
                    let y = fx - sx / HALF_W;
                    let z = (fx + y) * HALF_H - sy;
                    if y >= 0.0 && y <= fy && z >= 0.0 && z <= h {
                        col = shade(Face::Right, y, z);
                    }
                }
            }
            if let Some(c) = col {
                img.set_pixel(ix, iy, c);
            }
        }
    }
    (img, vec2(-minx, -miny))
}

fn outline_image(src: &Image) -> Image {
    let (w, h) = (src.width as i32, src.height as i32);
    let mut out = src.clone();
    for y in 0..h {
        for x in 0..w {
            if src.get_pixel(x as u32, y as u32).a > 0.0 {
                continue;
            }
            let near = [(-1, 0), (1, 0), (0, -1), (0, 1)].iter().any(|(dx, dy)| {
                let (nx, ny) = (x + dx, y + dy);
                nx >= 0 && ny >= 0 && nx < w && ny < h && src.get_pixel(nx as u32, ny as u32).a > 0.0
            });
            if near {
                out.set_pixel(x as u32, y as u32, OUTLINE);
            }
        }
    }
    out
}

fn face_k(f: Face) -> f32 {
    match f {
        Face::Top => 1.0,
        Face::Left => 0.84,
        Face::Right => 0.64,
    }
}

// ---------------------------------------------------------------- characters

#[derive(Clone, Copy, PartialEq)]
enum Head {
    Hair,
    LongHair,
    Hood,
    Helmet,
}

#[derive(Clone, Copy)]
struct Look {
    skin: Color,
    hair: Color,
    top: Color,
    accent: Color,
    accent2: Color,
    legs: Color,
    shoes: Color,
    eye: Color,
    head: Head,
    tie: bool,
    skirt: bool,
    zombie: bool,
    scarf: bool,
    seed: u32,
}

fn humanoid(l: &Look, back: bool, frame: usize) -> Cv {
    let mut c = Cv::new(12, 20);
    let lift_l = frame == 1;
    let lift_r = frame == 3;
    let bob = (frame % 2) as i32;

    // legs + shoes
    for (lx, lifted) in [(3, lift_l), (7, lift_r)] {
        let bottom = if lifted { 18 } else { 19 };
        let leg = if l.skirt { shade(l.skin, 0.9) } else { l.legs };
        c.rect(lx, 14, 2, bottom - 14, leg);
        c.set(lx + 1, 15, shade(leg, 0.8));
        c.rect(lx, bottom, 2, 1, l.shoes);
    }

    // torso
    let ty = 7 + bob;
    c.rect(2, ty, 8, 7, l.top);
    c.rect(2, ty, 1, 7, shade(l.top, 1.15));
    c.rect(9, ty, 1, 7, shade(l.top, 0.75));
    c.rect(2, ty + 6, 8, 1, shade(l.top, 0.7));
    if l.skirt {
        c.rect(2, ty + 6, 8, 3, l.legs);
        c.rect(9, ty + 6, 1, 3, shade(l.legs, 0.7));
    }
    if !back {
        if l.tie {
            c.rect(4, ty, 4, 1, l.accent);
            c.rect(5, ty + 1, 2, 1, l.accent);
            c.rect(5, ty + 1, 2, 4, l.accent2);
            c.set(5, ty + 5, l.accent2);
        } else if l.head == Head::Hood {
            c.rect(4, ty + 3, 4, 2, shade(l.top, 0.8));
            c.set(5, ty, l.accent);
            c.set(6, ty, l.accent);
            c.set(5, ty + 1, l.accent);
        } else if l.head == Head::Helmet {
            // tactical vest pouches
            c.rect(3, ty + 3, 2, 2, l.accent2);
            c.rect(7, ty + 3, 2, 2, l.accent2);
            c.rect(5, ty + 2, 2, 3, shade(l.top, 0.8));
        } else if l.skirt {
            c.rect(4, ty, 4, 3, l.accent);
            c.rect(5, ty, 2, 1, shade(l.accent, 0.8));
        }
    } else if l.head == Head::Helmet {
        // backpack
        c.rect(3, ty + 1, 6, 5, l.accent2);
        c.rect(3, ty + 1, 6, 1, shade(l.accent2, 1.2));
        c.rect(4, ty + 3, 4, 1, shade(l.accent2, 0.7));
    }
    if l.scarf {
        c.rect(3, ty, 6, 1, rgb(206, 38, 44));
        if back {
            c.set(7, ty + 1, rgb(206, 38, 44));
            c.set(8, ty + 2, rgb(160, 28, 34));
        }
    }

    // arms
    let (mut al, mut ar) = (8 + bob, 8 + bob);
    if l.zombie {
        if frame == 1 {
            al -= 2;
        }
        if frame == 3 {
            ar -= 2;
        }
    }
    let sleeve = shade(l.top, 0.9);
    c.rect(1, al, 1, 4, sleeve);
    c.rect(10, ar, 1, 4, shade(sleeve, 0.8));
    c.rect(1, al + 4, 1, if l.zombie { 2 } else { 1 }, l.skin);
    c.rect(10, ar + 4, 1, if l.zombie { 2 } else { 1 }, shade(l.skin, 0.85));

    // head
    let hy = bob;
    c.rect(3, hy + 1, 6, 6, l.skin);
    c.rect(8, hy + 1, 1, 6, shade(l.skin, 0.82));
    match l.head {
        Head::Hair | Head::LongHair => {
            c.rect(4, hy, 4, 1, l.hair);
            c.rect(3, hy + 1, 6, 1, l.hair);
            if back {
                c.rect(3, hy + 1, 6, 5, l.hair);
            } else {
                c.set(3, hy + 2, l.hair);
                c.set(8, hy + 2, l.hair);
            }
            if l.head == Head::LongHair {
                c.rect(2, hy + 1, 1, 7, l.hair);
                c.rect(9, hy + 1, 1, 7, l.hair);
                if back {
                    c.rect(3, hy + 6, 6, 3, l.hair);
                }
            }
        }
        Head::Hood => {
            c.rect(2, hy, 8, 7, l.top);
            c.rect(2, hy, 8, 1, shade(l.top, 1.15));
            if !back {
                c.rect(4, hy + 2, 4, 5, l.skin);
            } else {
                c.rect(3, hy + 2, 6, 4, shade(l.top, 0.85));
            }
        }
        Head::Helmet => {
            c.rect(2, hy, 8, 3, l.hair);
            c.rect(4, hy, 4, 1, shade(l.hair, 1.3));
            c.rect(2, hy + 2, 8, 1, shade(l.hair, 0.7));
            if back {
                c.rect(3, hy + 3, 6, 3, shade(l.hair, 0.6));
            }
        }
    }
    if !back {
        let ey = hy + 3;
        if l.head == Head::Helmet {
            c.rect(3, ey, 6, 1, l.eye);
            c.set(4, ey, WHITE);
        } else {
            let (e1, e2) = if l.head == Head::Hood { (4, 7) } else { (4, 7) };
            c.set(e1, ey, l.eye);
            c.set(e2, ey, l.eye);
        }
        if l.zombie {
            c.rect(5, hy + 5, 2, 1, rgb(60, 10, 14));
            c.set(5, hy + 6, rgb(120, 16, 20));
        }
    }

    // zombies get grime and blood
    if l.zombie {
        for i in 0..7 {
            let x = 2 + (hash2(i, frame as i32, l.seed) * 8.0) as i32;
            let y = 7 + (hash2(i, 9, l.seed) * 11.0) as i32;
            if c.get(x, y).is_some() {
                c.set(x, y, if i % 3 == 0 { rgb(70, 8, 12) } else { rgb(122, 14, 20) });
            }
        }
    }
    c
}

fn brute(back: bool, frame: usize) -> Cv {
    let mut c = Cv::new(16, 24);
    let bob = (frame % 2) as i32;
    let skin = rgb(120, 140, 104);
    let vest = rgb(232, 112, 26);
    let shirt = rgb(70, 74, 86);
    let pants = rgb(58, 66, 96);
    // legs
    for (lx, lifted) in [(3, frame == 1), (9, frame == 3)] {
        let bottom = if lifted { 22 } else { 23 };
        c.rect(lx, 16, 4, bottom - 16, pants);
        c.rect(lx + 3, 16, 1, bottom - 16, shade(pants, 0.7));
        c.rect(lx, bottom, 4, 1, rgb(40, 30, 24));
    }
    let ty = 7 + bob;
    c.rect(1, ty, 14, 10, shirt);
    c.rect(2, ty, 12, 10, vest);
    c.rect(13, ty, 1, 10, shade(vest, 0.72));
    c.rect(2, ty + 3, 12, 1, rgb(220, 230, 225));
    c.rect(2, ty + 7, 12, 1, rgb(220, 230, 225));
    if !back {
        c.rect(7, ty, 2, 10, shade(vest, 0.8));
    }
    // arms
    let al = ty + 1 - if frame == 1 { 2 } else { 0 };
    let ar = ty + 1 - if frame == 3 { 2 } else { 0 };
    c.rect(0, al, 2, 7, shirt);
    c.rect(14, ar, 2, 7, shade(shirt, 0.75));
    c.rect(0, al + 7, 2, 2, skin);
    c.rect(14, ar + 7, 2, 2, shade(skin, 0.8));
    // head
    let hy = bob;
    c.rect(4, hy + 2, 8, 6, skin);
    c.rect(11, hy + 2, 1, 6, shade(skin, 0.8));
    c.rect(3, hy, 10, 3, rgb(240, 200, 40));
    c.rect(2, hy + 3, 12, 1, rgb(200, 160, 30));
    c.rect(6, hy, 4, 1, rgb(255, 236, 120));
    if !back {
        c.set(5, hy + 5, rgb(255, 80, 40));
        c.set(10, hy + 5, rgb(255, 80, 40));
        c.rect(6, hy + 7, 4, 1, rgb(60, 10, 14));
    }
    for i in 0..8 {
        let x = 2 + (hash2(i, frame as i32, 77) * 12.0) as i32;
        let y = 8 + (hash2(i, 3, 77) * 14.0) as i32;
        if c.get(x, y).is_some() {
            c.set(x, y, rgb(110, 14, 18));
        }
    }
    c
}

fn boss(frame: usize) -> Cv {
    let mut c = Cv::new(34, 48);
    let bob = (frame % 2) as i32;
    let skin = rgb(110, 124, 98);
    let suit = rgb(34, 36, 50);
    let glow = if frame % 2 == 0 { rgb(255, 70, 50) } else { rgb(255, 140, 60) };
    for (lx, lifted) in [(8, frame == 1), (19, frame == 3)] {
        let bottom = if lifted { 46 } else { 47 };
        c.rect(lx, 32, 7, bottom - 32, suit);
        c.rect(lx + 6, 32, 1, bottom - 32, shade(suit, 0.7));
        c.rect(lx - 1, bottom, 8, 1, rgb(20, 16, 16));
    }
    let ty = 12 + bob;
    c.rect(5, ty, 24, 21, suit);
    c.rect(27, ty, 2, 21, shade(suit, 0.7));
    // torn shirt and glowing rib cage
    c.rect(11, ty, 12, 14, rgb(200, 196, 180));
    c.rect(12, ty + 2, 10, 11, rgb(70, 16, 20));
    for r in 0..4 {
        c.rect(12, ty + 3 + r * 3, 10, 1, rgb(214, 200, 176));
    }
    c.rect(16, ty + 4, 2, 7, glow);
    c.rect(15, ty + 6, 4, 3, glow);
    c.rect(16, ty, 2, 4, rgb(160, 20, 26));
    // huge arms with claws
    let al = ty + 2 - if frame == 1 { 4 } else { 0 };
    let ar = ty + 2 - if frame == 3 { 4 } else { 0 };
    c.rect(0, al, 5, 20, suit);
    c.rect(29, ar, 5, 20, shade(suit, 0.75));
    c.rect(0, al + 20, 5, 4, skin);
    c.rect(29, ar + 20, 5, 4, shade(skin, 0.8));
    for k in 0..3 {
        c.set(k * 2, al + 24, rgb(230, 230, 210));
        c.set(29 + k * 2, ar + 24, rgb(230, 230, 210));
    }
    // head
    let hy = bob;
    c.rect(10, hy + 1, 14, 12, skin);
    c.rect(22, hy + 1, 2, 12, shade(skin, 0.8));
    c.rect(11, hy, 12, 2, rgb(30, 30, 30));
    c.rect(18, hy + 1, 3, 3, rgb(150, 20, 24));
    for (ex, ey) in [(13, 5), (19, 5), (16, 3), (21, 8), (12, 8)] {
        c.set(ex, hy + ey, glow);
        c.set(ex + 1, hy + ey, glow);
    }
    c.rect(13, hy + 10, 8, 2, rgb(40, 6, 10));
    for k in 0..4 {
        c.set(14 + k * 2, hy + 10, rgb(230, 225, 200));
    }
    for i in 0..18 {
        let x = 2 + (hash2(i, frame as i32, 99) * 30.0) as i32;
        let y = 12 + (hash2(i, 5, 99) * 34.0) as i32;
        if c.get(x, y).is_some() {
            c.set(x, y, rgb(120, 12, 18));
        }
    }
    c
}

fn char_art(p: &mut Packer, make: impl Fn(bool, usize) -> Cv) -> CharArt {
    let mut frames = [[Spr { r: Rect::default(), anchor: Vec2::ZERO }; 4]; 2];
    let mut flash = frames;
    for view in 0..2 {
        for f in 0..4 {
            let cv = make(view == 1, f);
            let (img, sil) = cv.finish();
            let anchor = vec2((cv.w / 2 + 1) as f32, cv.h as f32);
            frames[view][f] = Spr { r: p.add(&img), anchor };
            flash[view][f] = Spr { r: p.add(&sil), anchor };
        }
    }
    CharArt { frames, flash }
}

// ---------------------------------------------------------------- props

fn tile_wall(u: f32, z: f32, base: Color) -> Color {
    let ux = (u * 16.0) as i32;
    let zi = z as i32;
    if ux % 4 == 0 || zi % 5 == 0 { shade(base, 0.78) } else { base }
}

fn make_pillar() -> (Image, Vec2) {
    let cream = rgb(212, 206, 190);
    let green = rgb(110, 178, 46);
    let (img, a) = iso_box(0.75, 0.75, 62.0, |f, u, v| {
        if f == Face::Top {
            return Some(rgb(40, 40, 44));
        }
        let k = face_k(f);
        let c = if v < 3.0 {
            rgb(60, 60, 64)
        } else if (30.0..34.0).contains(&v) {
            if v > 33.0 { shade(green, 1.2) } else { green }
        } else if f == Face::Left && (38.0..47.0).contains(&v) && (0.1..0.65).contains(&u) {
            let edge = v < 39.0 || v > 46.0 || u < 0.14 || u > 0.61;
            if edge {
                green
            } else if (v as i32 == 42 || v as i32 == 44) && ((u * 16.0) as i32 % 3 != 0) {
                rgb(30, 30, 34)
            } else {
                rgb(245, 245, 240)
            }
        } else if v > 56.0 {
            rgb(70, 70, 76)
        } else {
            tile_wall(u, v, cream)
        };
        Some(shade(c, k))
    });
    (outline_image(&img), a + vec2(0.0, 0.0))
}

fn make_vending(body: Color) -> (Image, Vec2) {
    let fx = 0.85;
    let (img, a) = iso_box(fx, 0.55, 30.0, |f, u, v| {
        let k = face_k(f);
        match f {
            Face::Top => Some(shade(body, 0.7)),
            Face::Right => {
                let c = if (18.0..22.0).contains(&v) { WHITE } else { body };
                Some(shade(c, k))
            }
            Face::Left => {
                let edge = u < 0.06 || u > fx - 0.06;
                let ui = (u * 16.0) as i32;
                let c = if v < 2.0 {
                    rgb(30, 30, 34)
                } else if edge || v >= 26.0 {
                    if v >= 26.0 && v < 29.0 && !edge { shade(body, 1.25) } else { body }
                } else if v >= 13.0 {
                    let vi = v as i32;
                    if vi == 17 || vi == 21 || vi == 25 {
                        rgb(150, 150, 160)
                    } else if ui % 2 == 0 {
                        let pal = [rgb(230, 50, 50), rgb(40, 110, 230), rgb(60, 200, 90), rgb(250, 160, 30), rgb(250, 250, 250), rgb(140, 70, 200)];
                        let row = (vi - 13) / 4;
                        pal[(hash2(ui, row, 5) * 6.0) as usize % 6]
                    } else {
                        rgb(250, 250, 238)
                    }
                } else if v >= 9.0 {
                    if ui % 2 == 0 { rgb(90, 240, 255) } else { rgb(40, 40, 50) }
                } else if (3.0..6.0).contains(&v) && (0.2..0.65).contains(&u) {
                    rgb(8, 8, 10)
                } else {
                    rgb(56, 58, 66)
                };
                Some(shade(c, if v >= 9.0 && v < 26.0 && !edge { 1.0 } else { k }))
            }
        }
    });
    (outline_image(&img), a)
}

fn make_bench() -> (Image, Vec2) {
    let wood = rgb(150, 96, 52);
    let (img, a) = iso_box(1.6, 0.42, 7.0, |f, u, v| match f {
        Face::Top => {
            let slat = ((v * 16.0) as i32) % 3 == 0;
            Some(if slat { shade(wood, 0.6) } else { wood })
        }
        _ => {
            let k = face_k(f);
            let leg = if f == Face::Left { u < 0.12 || u > 1.48 } else { true };
            if v >= 5.0 {
                Some(shade(wood, k))
            } else if leg {
                Some(shade(rgb(120, 124, 130), k))
            } else {
                None
            }
        }
    });
    (outline_image(&img), a)
}

fn make_gate() -> (Image, Vec2) {
    let body = rgb(200, 204, 210);
    let (img, a) = iso_box(0.3, 1.1, 14.0, |f, u, v| {
        let k = face_k(f);
        match f {
            Face::Top => {
                if (0.12..0.38).contains(&v) {
                    Some(rgb(40, 150, 255))
                } else if (0.75..0.92).contains(&v) {
                    Some(rgb(60, 230, 120))
                } else {
                    Some(body)
                }
            }
            Face::Right => {
                let c = if (8.0..10.0).contains(&v) && (0.3..0.8).contains(&u) { rgb(80, 255, 140) } else { body };
                Some(shade(c, k))
            }
            Face::Left => Some(shade(rgb(70, 74, 84), k)),
        }
    });
    (outline_image(&img), a)
}

fn make_bin() -> (Image, Vec2) {
    let (img, a) = iso_box(0.4, 0.4, 12.0, |f, u, _v| {
        let k = face_k(f);
        match f {
            Face::Top => Some(if (0.15..0.25).contains(&u) { rgb(10, 10, 10) } else { rgb(90, 90, 96) }),
            Face::Left => Some(shade(rgb(40, 120, 200), k)),
            Face::Right => Some(shade(rgb(50, 160, 80), k)),
        }
    });
    (outline_image(&img), a)
}

#[derive(Clone, Copy, PartialEq)]
enum Slice {
    Body,
    Door,
    DoorOpen,
    Gap,
    Cab,
}

fn make_train(kind: Slice) -> (Image, Vec2) {
    let silver = rgb(196, 202, 210);
    let green = rgb(118, 190, 40);
    let lit = rgb(255, 242, 196);
    iso_box(1.0, 1.4, 28.0, |f, u, v| {
        let k = face_k(f);
        if kind == Slice::Gap {
            return Some(shade(rgb(46, 48, 54), k * if (v as i32) % 3 == 0 { 0.7 } else { 1.0 }));
        }
        match f {
            Face::Top => {
                let c = if (0.55..0.85).contains(&v) { rgb(120, 124, 132) } else { rgb(160, 164, 172) };
                Some(c)
            }
            Face::Right => {
                if kind != Slice::Cab {
                    return Some(rgb(30, 30, 36));
                }
                let c = if v < 3.0 {
                    rgb(30, 30, 34)
                } else if (6.0..9.0).contains(&v) && ((0.15..0.35).contains(&u) || (1.05..1.25).contains(&u)) {
                    rgb(255, 255, 220)
                } else if (12.0..14.0).contains(&v) {
                    green
                } else if (15.0..24.0).contains(&v) && (0.12..1.28).contains(&u) {
                    let refl = ((u * 16.0) as i32 + v as i32) % 9 == 0;
                    if refl { rgb(90, 110, 130) } else { rgb(18, 24, 34) }
                } else if (25.0..27.0).contains(&v) && (0.35..1.05).contains(&u) {
                    if ((u * 16.0) as i32) % 2 == 0 { rgb(255, 150, 30) } else { rgb(30, 20, 10) }
                } else {
                    shade(silver, k)
                };
                Some(c)
            }
            Face::Left => {
                let ui = (u * 16.0) as i32;
                let door_zone = (0.15..0.85).contains(&u);
                let c = if v < 2.0 {
                    rgb(24, 24, 28)
                } else if v < 4.0 {
                    rgb(70, 72, 80)
                } else if (kind == Slice::Door || kind == Slice::DoorOpen) && door_zone && v < 22.0 {
                    if kind == Slice::DoorOpen {
                        if v < 3.5 { rgb(120, 120, 120) } else { lit }
                    } else if (12.0..20.0).contains(&v) && ((0.25..0.45).contains(&u) || (0.55..0.75).contains(&u)) {
                        lit
                    } else if ui == 8 {
                        rgb(40, 40, 48)
                    } else if v > 20.0 {
                        green
                    } else {
                        shade(rgb(176, 182, 190), k)
                    }
                } else if (23.0..26.0).contains(&v) {
                    green
                } else if (11.0..21.0).contains(&v) && (0.08..0.92).contains(&u) && kind != Slice::Cab {
                    if v < 13.0 {
                        rgb(60, 120, 80)
                    } else if (v as i32 == 19) && ui % 3 == 0 {
                        rgb(60, 60, 60)
                    } else {
                        lit
                    }
                } else if kind == Slice::Cab && (13.0..21.0).contains(&v) && (0.1..0.5).contains(&u) {
                    lit
                } else if ui % 8 == 0 {
                    shade(silver, k * 0.85)
                } else {
                    shade(silver, k)
                };
                Some(c)
            }
        }
    })
}

fn make_psd(sticker: bool) -> (Image, Vec2) {
    // waist-high platform screen door panel, half a tile long
    let panel = rgb(214, 218, 222);
    let green = rgb(110, 178, 46);
    let (img, a) = iso_box(0.5, 0.16, 15.0, |f, u, v| {
        let k = face_k(f);
        let c = match f {
            Face::Top => rgb(96, 100, 108),
            _ => {
                if v < 1.5 {
                    rgb(50, 52, 58)
                } else if v > 13.0 {
                    rgb(120, 124, 132)
                } else if (10.5..12.0).contains(&v) {
                    green
                } else if f == Face::Left && ((u * 16.0) as i32 == 0) {
                    rgb(150, 154, 160)
                } else if sticker && f == Face::Left && (4.0..7.0).contains(&v) && (0.18..0.32).contains(&u) {
                    rgb(240, 196, 40) // "do not lean" sticker
                } else {
                    panel
                }
            }
        };
        Some(shade(c, k))
    });
    (outline_image(&img), a)
}

fn make_suitcase(body: Color, lying: bool) -> (Image, Vec2) {
    let (fx, fy, h) = if lying { (0.45, 0.3, 5.0) } else { (0.3, 0.18, 11.0) };
    let (img, a) = iso_box(fx, fy, h, |f, u, v| {
        let k = face_k(f);
        let ridge = if lying { ((u * 16.0) as i32) % 3 == 0 } else { (v as i32) % 3 == 0 };
        let c = match f {
            Face::Top => {
                if !lying && (0.35..0.65).contains(&(u / fx)) {
                    rgb(40, 40, 44) // handle
                } else {
                    shade(body, 1.1)
                }
            }
            _ => if ridge { shade(body, 0.8) } else { body },
        };
        Some(shade(c, k))
    });
    (outline_image(&img), a)
}

fn make_boxes() -> (Image, Vec2) {
    let card = rgb(176, 136, 86);
    let (img, a) = iso_box(0.6, 0.5, 14.0, |f, u, v| {
        let k = face_k(f);
        let c = match f {
            Face::Top => if (0.27..0.33).contains(&u) { rgb(196, 170, 120) } else { shade(card, 1.08) },
            _ => {
                if (6.5..7.5).contains(&v) {
                    shade(card, 0.6) // two boxes stacked
                } else if (0.27..0.33).contains(&u) && v > 11.0 {
                    rgb(196, 170, 120) // tape
                } else {
                    card
                }
            }
        };
        Some(shade(c, k))
    });
    (outline_image(&img), a)
}

fn make_kiosk() -> (Image, Vec2) {
    let body = rgb(46, 120, 74);
    let fx = 2.2;
    let (img, a) = iso_box(fx, 1.0, 26.0, |f, u, v| {
        let k = face_k(f);
        let c = match f {
            Face::Top => shade(body, 0.7),
            Face::Right => if v > 20.0 { rgb(250, 250, 240) } else { body },
            Face::Left => {
                let ui = (u * 16.0) as i32;
                if v > 22.0 {
                    // lit sign band with the word KIOSK
                    let fxp = ((u - 0.6) * 16.0).floor();
                    let fyp = (25.5 - v).floor();
                    let lit = fxp >= 0.0 && fyp >= 0.0 && {
                        let n = fxp as usize;
                        let ch = "KIOSK".chars().nth(n / 6);
                        ch.map_or(false, |c| font::bit(c, n % 6, fyp as usize))
                    };
                    if lit { rgb(30, 60, 40) } else { rgb(255, 250, 225) }
                } else if (11.0..21.0).contains(&v) && (0.1..fx - 0.1).contains(&u) {
                    // shelves of snacks and magazines
                    if (v as i32) % 4 == 0 {
                        rgb(120, 110, 100)
                    } else {
                        let pal = [rgb(230, 70, 60), rgb(250, 200, 60), rgb(80, 140, 230), rgb(240, 240, 235), rgb(90, 200, 120)];
                        pal[(hash2(ui, v as i32 / 4, 31) * 5.0) as usize % 5]
                    }
                } else if (8.0..11.0).contains(&v) {
                    rgb(200, 200, 205) // counter
                } else {
                    body
                }
            }
        };
        Some(shade(c, if f == Face::Left && v > 8.0 { 1.0 } else { k }))
    });
    (outline_image(&img), a)
}

fn make_barrier() -> (Image, Vec2) {
    let (img, a) = iso_box(1.0, 0.14, 10.0, |f, u, v| {
        let k = face_k(f);
        if f == Face::Left && !(u < 0.08 || u > 0.92) && !(4.0..9.0).contains(&v) {
            return None; // open frame below the striped board
        }
        let stripe = (((u * 16.0) as i32 + v as i32) / 3) % 2 == 0;
        let c = if (4.0..9.0).contains(&v) || f == Face::Top {
            if stripe { rgb(240, 200, 30) } else { rgb(30, 30, 30) }
        } else {
            rgb(200, 200, 200)
        };
        Some(shade(c, k))
    });
    (outline_image(&img), a)
}

fn make_crow(frame: usize) -> Cv {
    let mut c = Cv::new(9, 7);
    let k = rgb(18, 18, 24);
    let hi = rgb(46, 52, 80);
    match frame {
        0 | 1 => {
            c.rect(2, 3, 5, 2, k); // body
            c.rect(3, 3, 3, 1, hi);
            c.rect(0, 4, 2, 1, k); // tail
            c.set(3, 5, rgb(60, 60, 64));
            c.set(5, 5, rgb(60, 60, 64)); // legs
            if frame == 0 {
                c.rect(6, 1, 2, 2, k); // head up
                c.set(8, 2, rgb(70, 70, 70));
                c.set(7, 1, rgb(200, 200, 200));
            } else {
                c.rect(6, 4, 2, 2, k); // head down, pecking
                c.set(8, 5, rgb(70, 70, 70));
            }
        }
        _ => {
            c.rect(2, 3, 5, 2, k);
            c.rect(6, 2, 2, 2, k);
            c.set(8, 3, rgb(70, 70, 70));
            c.rect(0, 3, 2, 1, k);
            if frame == 2 {
                c.rect(3, 0, 1, 3, k);
                c.rect(4, 1, 2, 2, hi);
            } else {
                c.rect(3, 5, 3, 2, k);
            }
        }
    }
    c
}

fn make_esc_step(h: f32, bottom: bool) -> (Image, Vec2) {
    // brushed-steel treads with bright yellow edge lines; the bottom slice gets the comb plate
    iso_box(0.5, 1.0, h, |f, u, v| {
        let k = face_k(f);
        Some(match f {
            Face::Top => {
                let gi = (v * 16.0) as i32;
                if u < 0.07 || (bottom && u < 0.2) {
                    rgb(255, 214, 40)
                } else if v < 0.06 || v > 0.94 {
                    rgb(250, 210, 50)
                } else if gi % 2 == 0 {
                    rgb(150, 156, 166)
                } else {
                    rgb(196, 202, 212)
                }
            }
            Face::Right => shade(rgb(130, 136, 148), k),
            Face::Left => {
                // the riser: grooves, with the skirt-light strip near the top
                if v > h - 2.0 {
                    rgb(255, 250, 230)
                } else if (v as i32) % 2 == 0 {
                    shade(rgb(110, 116, 128), k)
                } else {
                    shade(rgb(160, 166, 178), k)
                }
            }
        })
    })
}

fn make_esc_rail(h: f32) -> (Image, Vec2) {
    let top = h + 12.0;
    let (img, a) = iso_box(0.5, 0.12, top, |f, _u, v| {
        Some(if v > top - 2.5 || f == Face::Top {
            rgb(18, 18, 22) // rubber handrail
        } else if v > top - 3.5 {
            rgb(230, 240, 255) // lit strip under the handrail
        } else if v < h - 1.0 {
            shade(rgb(176, 182, 194), face_k(f)) // steel skirt
        } else if v < h + 0.5 {
            rgb(255, 250, 230) // skirt light
        } else {
            Color::new(0.62, 0.82, 0.92, 0.7) // glass balustrade
        })
    });
    (outline_image(&img), a)
}

fn make_maze_wall(kind: usize) -> (Image, Vec2) {
    // shuttered shops, tiled walls with anime-style posters, neon storefronts
    let (img, a) = iso_box(1.0, 1.0, 34.0, |f, u, v| {
        let k = face_k(f);
        let ui = (u * 16.0) as i32;
        let c = match f {
            Face::Top => rgb(30, 30, 36),
            _ => match kind {
                0 => {
                    if v > 30.0 {
                        rgb(80, 80, 88)
                    } else if (v as i32) % 3 == 0 {
                        rgb(120, 124, 132)
                    } else {
                        rgb(168, 172, 180)
                    }
                }
                1 => {
                    if (8.0..26.0).contains(&v) && (0.2..0.8).contains(&u) {
                        let pal = [rgb(255, 120, 180), rgb(120, 210, 255), rgb(255, 220, 120)];
                        let c0 = pal[(hash2(ui / 6, 1, 3) * 3.0) as usize % 3];
                        if (v - 17.0).abs() < 4.0 && (u - 0.5).abs() < 0.12 { shade(c0, 1.3) } else { shade(c0, 0.85) }
                    } else if ui % 4 == 0 || (v as i32) % 5 == 0 {
                        rgb(150, 146, 140)
                    } else {
                        rgb(200, 196, 188)
                    }
                }
                _ => {
                    if (24.0..28.0).contains(&v) {
                        if ui % 2 == 0 { rgb(255, 60, 200) } else { rgb(60, 230, 255) }
                    } else if v < 22.0 && v > 3.0 && (0.1..0.9).contains(&u) {
                        rgb(20, 24, 34) // dark shop window
                    } else {
                        rgb(46, 44, 56)
                    }
                }
            },
        };
        let lit = kind == 2 && f != Face::Top && (24.0..28.0).contains(&v);
        Some(if lit { c } else { shade(c, k) })
    });
    (outline_image(&img), a)
}

fn make_cabinet() -> (Image, Vec2) {
    let body = rgb(70, 30, 120);
    let (img, a) = iso_box(0.8, 0.7, 32.0, |f, u, v| {
        let k = face_k(f);
        let ui = (u * 16.0) as i32;
        let c = match f {
            Face::Top => rgb(20, 18, 26),
            Face::Right => {
                if ((u * 10.0 + v * 0.3) as i32) % 4 == 0 { rgb(255, 210, 60) } else { body }
            }
            Face::Left => {
                if v > 27.0 && v < 31.0 {
                    // marquee
                    let pal = [rgb(255, 80, 200), rgb(80, 220, 255), rgb(255, 230, 80)];
                    pal[((ui + v as i32) / 2) as usize % 3]
                } else if (16.0..26.0).contains(&v) && (0.1..0.7).contains(&u) {
                    // the screen: a formation of tiny aliens
                    let vi = v as i32;
                    if (vi == 23 || vi == 21) && ui % 2 == 0 {
                        if vi == 23 { rgb(120, 255, 140) } else { rgb(255, 120, 200) }
                    } else if vi == 17 && ui == 6 {
                        rgb(255, 255, 255)
                    } else {
                        rgb(10, 14, 40)
                    }
                } else if (12.0..15.0).contains(&v) {
                    if v as i32 == 13 && (ui == 4 || ui == 8) { rgb(255, 50, 50) } else if v as i32 == 13 && ui == 6 { rgb(60, 120, 255) } else { rgb(20, 20, 24) }
                } else {
                    body
                }
            }
        };
        let lit = f == Face::Left && v > 15.0;
        Some(if lit { c } else { shade(c, k) })
    });
    (outline_image(&img), a)
}

fn make_car_seat() -> (Image, Vec2) {
    let cushion = rgb(40, 140, 120);
    let (img, a) = iso_box(1.0, 0.55, 10.0, |f, u, v| {
        let k = face_k(f);
        Some(match f {
            Face::Top => if ((u * 16.0) as i32) % 4 == 0 { shade(cushion, 0.8) } else { cushion },
            _ => if v > 6.0 { shade(cushion, k) } else { shade(rgb(150, 154, 160), k * 0.8) },
        })
    });
    (outline_image(&img), a)
}

fn make_car_wall(door: bool) -> (Image, Vec2) {
    let (img, a) = iso_box(0.5, 0.1, 13.0, |f, u, v| {
        let k = face_k(f);
        Some(shade(
            if f == Face::Top {
                rgb(170, 174, 182)
            } else if door {
                if (u * 16.0) as i32 == 0 { rgb(30, 30, 34) } else if v > 8.0 { rgb(40, 50, 60) } else { rgb(176, 182, 190) }
            } else if (9.0..11.0).contains(&v) {
                rgb(110, 178, 46)
            } else {
                rgb(196, 202, 210)
            },
            k,
        ))
    });
    (outline_image(&img), a)
}

fn make_straps() -> (Image, Vec2) {
    iso_box(1.0, 0.05, 66.0, |f, u, v| {
        if f == Face::Top {
            return None;
        }
        let rail = (60.0..62.0).contains(&v);
        let loop_at = |c: f32| (u - c).abs() < 0.04;
        let strap = (52.0..60.0).contains(&v) && (loop_at(0.25) || loop_at(0.75));
        let ring = (48.0..52.0).contains(&v) && ((u - 0.25).abs() < 0.1 || (u - 0.75).abs() < 0.1) && !((49.0..51.0).contains(&v) && ((u - 0.25).abs() < 0.05 || (u - 0.75).abs() < 0.05));
        if rail {
            Some(rgb(180, 184, 192))
        } else if strap {
            Some(rgb(60, 60, 66))
        } else if ring {
            Some(rgb(236, 236, 230))
        } else {
            None
        }
    })
}

fn make_ad(seed: i32) -> (Image, Vec2) {
    let pal = [rgb(250, 90, 70), rgb(70, 160, 240), rgb(250, 210, 70), rgb(120, 200, 110), rgb(240, 120, 200)];
    let a = pal[seed as usize % 5];
    let b = pal[(seed as usize + 2) % 5];
    iso_box(1.1, 0.04, 66.0, |f, u, v| {
        if f == Face::Top {
            return None;
        }
        if (60.0..64.0).contains(&v) && (u - 0.55).abs() < 0.03 {
            return Some(rgb(120, 120, 126));
        }
        if !(46.0..60.0).contains(&v) {
            return None;
        }
        let ui = (u * 16.0) as i32;
        let edge = v < 47.0 || v > 59.0 || ui == 0 || ui >= 17;
        Some(if edge {
            rgb(240, 240, 236)
        } else if v > 53.0 {
            a
        } else if (v as i32) % 2 == 0 && ui > 2 && ui < 15 {
            rgb(40, 40, 46)
        } else {
            mix(b, WHITE, 0.6)
        })
    })
}

/// Hanging signs for one station: a name board and a blinking LED departure board.
pub fn build_signs(name: &str, l1: &str) -> SignArt {
    let mut p = Packer::new_sized(512);
    let green = rgb(110, 178, 46);
    let text_at = |text: &str, u: f32, z: f32, u0: f32, ztop: f32| -> bool {
        let fx = ((u - u0) * 16.0).floor();
        let fy = (ztop - z).floor();
        if fx < 0.0 || fy < 0.0 {
            return false;
        }
        let n = fx as usize;
        text.chars().nth(n / 6).map_or(false, |c| font::bit(c, n % 6, fy as usize))
    };
    let rods = |u: f32, fx: f32, v: f32, top: f32| v > top && v < 74.0 && ((0.12..0.18).contains(&u) || (fx - 0.18..fx - 0.12).contains(&u));

    // station name board
    let nw = font::text_width(name) as f32 / 16.0 + 0.7;
    let (img, a) = iso_box(nw, 0.12, 74.0, |f, u, v| {
        if rods(u, nw, v, 60.0) {
            return Some(rgb(70, 70, 76));
        }
        if !(46.0..=60.0).contains(&v) && f != Face::Top {
            return None;
        }
        Some(match f {
            Face::Top => {
                if v > 0.0 { return None; } // the box top sits at the ceiling; hide it
                rgb(40, 40, 46)
            }
            Face::Right => rgb(60, 60, 66),
            Face::Left => {
                if v < 48.0 {
                    green
                } else if v > 59.0 || u < 0.06 || u > nw - 0.06 {
                    rgb(40, 40, 46)
                } else if text_at(name, u, v, 0.35, 57.5) {
                    rgb(24, 24, 30)
                } else {
                    rgb(246, 246, 240)
                }
            }
        })
    });
    let name_spr = Spr { r: p.add(&outline_image(&img)), anchor: a };

    // LED departure board (two frames: second line blinks)
    let l2 = "LAST TRAIN 00:12";
    let lw = (font::text_width(l1).max(font::text_width(l2)) as f32) / 16.0 + 0.7;
    let mut led = [name_spr, name_spr];
    for frame in 0..2 {
        let (img, a) = iso_box(lw, 0.12, 74.0, |f, u, v| {
            if rods(u, lw, v, 62.0) {
                return Some(rgb(70, 70, 76));
            }
            if !(44.0..=62.0).contains(&v) {
                return None;
            }
            Some(match f {
                Face::Top => return None,
                Face::Right => rgb(40, 40, 46),
                Face::Left => {
                    if v > 61.0 || v < 45.0 || u < 0.06 || u > lw - 0.06 {
                        rgb(56, 56, 62)
                    } else if text_at(l1, u, v, 0.35, 59.5) {
                        rgb(255, 170, 40)
                    } else if frame == 0 && text_at(l2, u, v, 0.35, 51.5) {
                        rgb(255, 120, 30)
                    } else if ((u * 16.0) as i32 + v as i32) % 2 == 0 {
                        rgb(14, 12, 10)
                    } else {
                        rgb(26, 20, 16)
                    }
                }
            })
        });
        led[frame] = Spr { r: p.add(&outline_image(&img)), anchor: a };
    }
    let tex = Texture2D::from_image(&p.img);
    tex.set_filter(FilterMode::Nearest);
    SignArt { tex, name: name_spr, led }
}

// ---------------------------------------------------------------- build everything

impl Art {
    pub fn build() -> Art {
        let mut p = Packer::new();

        let mut white_img = Image::gen_image_color(4, 4, WHITE);
        let white = p.add(&white_img);
        white_img = Image::gen_image_color(16, 16, Color::new(1., 1., 1., 0.));
        for y in 0..16 {
            for x in 0..16 {
                let d = vec2(x as f32 - 7.5, y as f32 - 7.5).length() / 8.0;
                let a = (1.0 - d).clamp(0.0, 1.0).powf(1.6);
                white_img.set_pixel(x, y, Color::new(1., 1., 1., a));
            }
        }
        let soft = p.add(&white_img);

        let mut sh = Image::gen_image_color(14, 7, Color::new(0., 0., 0., 0.));
        for y in 0..7 {
            for x in 0..14 {
                let d = vec2((x as f32 - 6.5) / 7.0, (y as f32 - 3.0) / 3.5).length();
                if d < 1.0 {
                    sh.set_pixel(x, y, Color::new(0.0, 0.0, 0.02, if d < 0.7 { 0.55 } else { 0.35 }));
                }
            }
        }
        let shadow = p.add(&sh);

        let mut ring = Image::gen_image_color(64, 32, Color::new(0., 0., 0., 0.));
        for y in 0..32 {
            for x in 0..64 {
                let d = vec2((x as f32 - 31.5) / 32.0, (y as f32 - 15.5) / 16.0).length();
                if (0.86..1.0).contains(&d) {
                    ring.set_pixel(x, y, WHITE);
                } else if d < 0.86 {
                    ring.set_pixel(x, y, Color::new(1., 1., 1., 0.18 * d));
                }
            }
        }
        let ring = p.add(&ring);

        let mut glyphs = HashMap::new();
        for ch in font::CHARS.chars() {
            let mut g = Image::gen_image_color(5, 7, Color::new(0., 0., 0., 0.));
            for y in 0..7 {
                for x in 0..5 {
                    if font::bit(ch, x, y) {
                        g.set_pixel(x as u32, y as u32, WHITE);
                    }
                }
            }
            glyphs.insert(ch, p.add(&g));
        }

        // the player: helmet, cyan goggles, red scarf so you can find yourself in a crowd
        let pl = Look {
            skin: rgb(226, 178, 140),
            hair: rgb(84, 98, 58),
            top: rgb(56, 64, 50),
            accent: rgb(206, 38, 44),
            accent2: rgb(96, 82, 56),
            legs: rgb(36, 38, 44),
            shoes: rgb(22, 18, 16),
            eye: rgb(90, 240, 255),
            head: Head::Helmet,
            tie: false,
            skirt: false,
            zombie: false,
            scarf: true,
            seed: 1,
        };
        let player = char_art(&mut p, |b, f| humanoid(&pl, b, f));

        let zskin = rgb(128, 150, 118);
        let zeye = rgb(255, 214, 80);
        let base = Look {
            skin: zskin,
            hair: rgb(30, 28, 30),
            top: rgb(36, 44, 76),
            accent: rgb(220, 218, 206),
            accent2: rgb(170, 30, 40),
            legs: rgb(40, 44, 60),
            shoes: rgb(16, 14, 14),
            eye: zeye,
            head: Head::Hair,
            tie: true,
            skirt: false,
            zombie: true,
            scarf: false,
            seed: 10,
        };
        let walkers = vec![
            base,
            Look { top: rgb(70, 72, 78), legs: rgb(62, 64, 70), accent2: rgb(40, 60, 140), seed: 11, ..base },
            Look { top: rgb(96, 70, 52), legs: rgb(60, 48, 40), accent2: rgb(30, 90, 60), hair: rgb(90, 90, 92), seed: 12, ..base },
            Look { top: rgb(40, 40, 46), legs: rgb(34, 34, 40), accent: rgb(230, 228, 220), head: Head::LongHair, tie: false, skirt: true, seed: 13, ..base },
        ];
        let runners = vec![
            Look { top: rgb(170, 40, 46), accent: rgb(230, 230, 230), legs: rgb(52, 70, 110), head: Head::Hood, tie: false, eye: rgb(255, 90, 60), seed: 20, ..base },
            Look { top: rgb(120, 124, 132), accent: rgb(230, 230, 230), legs: rgb(40, 40, 46), head: Head::Hood, tie: false, eye: rgb(255, 90, 60), seed: 21, ..base },
            Look { top: rgb(200, 180, 60), accent: rgb(40, 40, 40), legs: rgb(30, 30, 34), head: Head::Hood, tie: false, eye: rgb(255, 90, 60), seed: 22, ..base },
        ];
        let mut zombies = Vec::new();
        zombies.push(walkers.iter().map(|l| char_art(&mut p, |b, f| humanoid(l, b, f))).collect());
        zombies.push(runners.iter().map(|l| char_art(&mut p, |b, f| humanoid(l, b, f))).collect());
        zombies.push(vec![char_art(&mut p, brute)]);
        let boss = char_art(&mut p, |_b, f| boss(f));

        let mut gun = Cv::new(11, 5);
        gun.rect(2, 1, 7, 2, rgb(40, 42, 48));
        gun.rect(6, 1, 5, 1, rgb(60, 62, 70));
        gun.rect(3, 0, 5, 1, rgb(100, 104, 112));
        gun.rect(4, 3, 2, 2, rgb(30, 30, 34));
        gun.rect(0, 2, 2, 1, rgb(50, 44, 40));
        gun.set(10, 1, rgb(140, 140, 150));
        let (gimg, _) = gun.finish();
        let gun = Spr { r: p.add(&gimg), anchor: vec2(4.0, 3.0) };

        let mut prop = |(img, a): (Image, Vec2)| Spr { r: p.add(&img), anchor: a };
        let pillar = prop(make_pillar());
        let vending = vec![
            prop(make_vending(rgb(200, 30, 36))),
            prop(make_vending(rgb(36, 90, 200))),
            prop(make_vending(rgb(226, 226, 230))),
        ];
        let bench = prop(make_bench());
        let gate = prop(make_gate());
        let bin = prop(make_bin());
        let train_body = prop(make_train(Slice::Body));
        let train_door = prop(make_train(Slice::Door));
        let train_door_open = prop(make_train(Slice::DoorOpen));
        let train_gap = prop(make_train(Slice::Gap));
        let train_cab = prop(make_train(Slice::Cab));

        let mut on = Cv::new(9, 8);
        for y in 0..8 {
            let half = (y + 1).min(5);
            on.rect(4 - half + 1, y, half * 2 - 1, 1, rgb(248, 246, 236));
        }
        on.rect(3, 5, 3, 3, rgb(20, 40, 28));
        on.set(2, 2, rgb(255, 255, 255));
        let (oimg, _) = on.finish();
        let onigiri = Spr { r: p.add(&oimg), anchor: vec2(5.0, 9.0) };

        let mut gr = Cv::new(4, 4);
        gr.rect(0, 0, 4, 4, rgb(70, 90, 50));
        gr.set(1, 0, rgb(140, 160, 110));
        gr.set(3, 0, rgb(200, 200, 200));
        let (grimg, _) = gr.finish();
        let grenade = p.add(&grimg);
        let mut cs = Image::gen_image_color(2, 1, rgb(220, 170, 70));
        cs.set_pixel(1, 0, rgb(150, 110, 40));
        let casing = p.add(&cs);

        let psd = [false, true].map(|st| { let (i, a) = make_psd(st); Spr { r: p.add(&i), anchor: a } });
        let suitcases = [(rgb(200, 40, 50), false), (rgb(40, 60, 120), false), (rgb(190, 194, 200), false), (rgb(60, 60, 64), true)]
            .iter()
            .map(|(c, lying)| { let (i, a) = make_suitcase(*c, *lying); Spr { r: p.add(&i), anchor: a } })
            .collect();
        let boxes = { let (i, a) = make_boxes(); Spr { r: p.add(&i), anchor: a } };
        let kiosk = { let (i, a) = make_kiosk(); Spr { r: p.add(&i), anchor: a } };
        let barrier = { let (i, a) = make_barrier(); Spr { r: p.add(&i), anchor: a } };
        let mut crow = [Spr { r: Rect::default(), anchor: Vec2::ZERO }; 4];
        for (f, slot) in crow.iter_mut().enumerate() {
            let cv = make_crow(f);
            let (img, _) = cv.finish();
            *slot = Spr { r: p.add(&img), anchor: vec2(5.0, 7.0) };
        }

        // ---- ordinary people for the opening
        let skin = [rgb(226, 178, 140), rgb(204, 156, 116), rgb(238, 198, 164)];
        let human = Look { zombie: false, eye: rgb(40, 30, 30), scarf: false, seed: 40, ..base };
        let looks = vec![
            Look { skin: skin[0], top: rgb(36, 44, 76), legs: rgb(40, 44, 60), ..human },
            Look { skin: skin[1], top: rgb(70, 72, 78), legs: rgb(62, 64, 70), accent2: rgb(40, 60, 140), ..human },
            Look { skin: skin[2], top: rgb(40, 40, 46), legs: rgb(34, 34, 40), head: Head::LongHair, tie: false, skirt: true, ..human },
            Look { skin: skin[0], top: rgb(176, 146, 104), legs: rgb(50, 46, 44), tie: false, hair: rgb(60, 40, 30), ..human },
            Look { skin: skin[1], top: rgb(60, 130, 90), legs: rgb(52, 70, 110), tie: false, head: Head::Hood, accent: rgb(230, 230, 230), ..human },
            Look { skin: skin[2], top: rgb(110, 80, 60), legs: rgb(70, 66, 60), hair: rgb(180, 180, 184), ..human },
            Look { skin: skin[0], top: rgb(40, 110, 200), legs: rgb(200, 190, 160), tie: false, hair: rgb(200, 170, 90), ..human },
            Look { skin: skin[2], top: rgb(236, 236, 240), legs: rgb(36, 40, 70), head: Head::LongHair, tie: false, skirt: true, accent: rgb(236, 236, 240), hair: rgb(70, 46, 30), ..human },
        ];
        let commuters: Vec<CharArt> = looks.iter().map(|l| char_art(&mut p, |b, f| humanoid(l, b, f))).collect();
        let strap: Vec<Spr> = looks
            .iter()
            .map(|l| {
                let mut c = humanoid(l, false, 0);
                let sleeve = shade(l.top, 0.75);
                for y in 6..14 {
                    c.clear(10, y);
                }
                c.rect(10, 1, 1, 7, sleeve);
                c.set(10, 0, shade(l.skin, 0.85));
                let (img, _) = c.finish();
                Spr { r: p.add(&img), anchor: vec2(7.0, 20.0) }
            })
            .collect();
        let sit: Vec<Spr> = looks
            .iter()
            .map(|l| {
                let stand = humanoid(l, false, 0);
                let mut c = Cv::new(12, 18);
                c.blit_rows(&stand, 0, 14, 1);
                let leg = if l.skirt { shade(l.skin, 0.9) } else { l.legs };
                c.rect(3, 15, 2, 2, leg);
                c.rect(7, 15, 2, 2, leg);
                c.rect(3, 17, 2, 1, l.shoes);
                c.rect(7, 17, 2, 1, l.shoes);
                let (img, _) = c.finish();
                Spr { r: p.add(&img), anchor: vec2(7.0, 18.0) }
            })
            .collect();
        let kneel = {
            let l = &looks[1];
            let stand = humanoid(l, false, 0);
            let mut c = Cv::new(12, 20);
            c.blit_rows(&stand, 0, 14, 5);
            c.rect(2, 18, 8, 2, l.legs); // knees on the ground
            c.rect(1, 17, 1, 2, l.skin);
            c.rect(10, 17, 1, 2, l.skin); // hands braced on the floor
            let (img, _) = c.finish();
            Spr { r: p.add(&img), anchor: vec2(7.0, 20.0) }
        };
        let cop = Look {
            skin: skin[0],
            hair: rgb(30, 40, 80),
            top: rgb(40, 52, 96),
            accent: rgb(150, 190, 230),
            accent2: rgb(30, 30, 34),
            legs: rgb(36, 44, 76),
            eye: rgb(40, 30, 30),
            head: Head::Helmet,
            tie: false,
            skirt: false,
            zombie: false,
            scarf: false,
            seed: 50,
            shoes: rgb(16, 14, 14),
        };
        let officer = char_art(&mut p, |b, f| {
            let mut c = humanoid(&cop, b, f);
            c.set(5, 1 + (f % 2) as i32, rgb(250, 210, 60)); // cap badge
            c
        });
        let mut pistol = Cv::new(7, 4);
        pistol.rect(1, 0, 6, 2, rgb(40, 42, 48));
        pistol.rect(2, 0, 4, 1, rgb(96, 100, 110));
        pistol.rect(1, 2, 2, 2, rgb(50, 40, 34));
        let (pimg, _) = pistol.finish();
        let pistol = Spr { r: p.add(&pimg), anchor: vec2(2.0, 3.0) };
        let esc_step = (0..ESC_SLICES).map(|i| { let (img, a) = make_esc_step(esc_height(i), i == 0); Spr { r: p.add(&img), anchor: a } }).collect();
        let maze = (0..3).map(|k| { let (img, a) = make_maze_wall(k); Spr { r: p.add(&img), anchor: a } }).collect();
        let mut coin_img = Image::gen_image_color(5, 5, Color::new(0., 0., 0., 0.));
        for y in 0..5u32 {
            for x in 0..5u32 {
                let d = vec2(x as f32 - 2.0, y as f32 - 2.0).length();
                if d < 2.4 {
                    coin_img.set_pixel(x, y, if d < 1.2 { rgb(255, 236, 120) } else { rgb(220, 170, 40) });
                }
            }
        }
        let coin = p.add(&coin_img);
        let esc_rail = (0..ESC_SLICES).map(|i| { let (img, a) = make_esc_rail(esc_height(i)); Spr { r: p.add(&img), anchor: a } }).collect();
        let cabinet = { let (i, a) = make_cabinet(); Spr { r: p.add(&i), anchor: a } };
        let car_seat = { let (i, a) = make_car_seat(); Spr { r: p.add(&i), anchor: a } };
        let car_wall = { let (i, a) = make_car_wall(false); Spr { r: p.add(&i), anchor: a } };
        let car_door = { let (i, a) = make_car_wall(true); Spr { r: p.add(&i), anchor: a } };
        let straps = { let (i, a) = make_straps(); Spr { r: p.add(&i), anchor: a } };
        let ads = (0..3).map(|k| { let (i, a) = make_ad(k); Spr { r: p.add(&i), anchor: a } }).collect();

        let tex = Texture2D::from_image(&p.img);
        tex.set_filter(FilterMode::Nearest);

        Art {
            maze,
            coin,
            commuters,
            strap,
            sit,
            kneel,
            officer,
            pistol,
            esc_step,
            esc_rail,
            cabinet,
            car_seat,
            car_wall,
            car_door,
            straps,
            ads,
            psd,
            suitcases,
            boxes,
            kiosk,
            barrier,
            crow,
            tex,
            white,
            soft,
            shadow,
            ring,
            glyphs,
            player,
            zombies,
            boss,
            gun,
            pillar,
            vending,
            bench,
            gate,
            bin,
            train_body,
            train_door,
            train_door_open,
            train_gap,
            train_cab,
            onigiri,
            grenade,
            casing,
        }
    }
}
