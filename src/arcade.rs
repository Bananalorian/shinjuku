//! STAR COMMUTER: the arcade cabinet in Akihabara station. A tiny shooter that
//! runs inside the game. (Coins will be currency later; today it's free play.)

use crate::art::Art;
use crate::hud;
use crate::synth::Id;
use macroquad::prelude::*;

const FW: f32 = 96.0;
const FH: f32 = 128.0;

const ALIENS: [[&str; 5]; 2] = [
    [" #   # ", "  ###  ", " ##### ", "## # ##", "#     #"],
    ["  ###  ", " ##### ", "## # ##", " #####  ", " # # # "],
];
const SHIP: [&str; 5] = ["   #   ", "  ###  ", "  # #  ", " ##### ", "## # ##"];

#[derive(Clone, Copy)]
struct Alien {
    p: Vec2,
    kind: usize,
    dive: Option<f32>,
}

pub struct Arcade {
    ship: f32,
    shots: Vec<Vec2>,
    bombs: Vec<Vec2>,
    aliens: Vec<Alien>,
    dir: f32,
    speed: f32,
    wave: u32,
    pub score: u32,
    pub hi: u32,
    lives: i32,
    cool: f32,
    t: f32,
    state: u8, // 0 attract, 1 playing, 2 game over
    inv: f32,
    booms: Vec<(Vec2, f32)>,
    pub sounds: Vec<Id>,
}

pub struct ArcadeIn {
    pub x: f32,
    pub fire: bool,
    pub exit: bool,
}

impl Arcade {
    pub fn new(hi: u32) -> Arcade {
        let mut a = Arcade {
            ship: FW * 0.5,
            shots: Vec::new(),
            bombs: Vec::new(),
            aliens: Vec::new(),
            dir: 1.0,
            speed: 10.0,
            wave: 0,
            score: 0,
            hi,
            lives: 3,
            cool: 0.0,
            t: 0.0,
            state: 0,
            inv: 0.0,
            booms: Vec::new(),
            sounds: Vec::new(),
        };
        a.formation();
        a
    }

    fn formation(&mut self) {
        self.aliens.clear();
        for r in 0..3 {
            for c in 0..6 {
                self.aliens.push(Alien { p: vec2(14.0 + c as f32 * 12.0, 16.0 + r as f32 * 10.0), kind: (r % 2) as usize, dive: None });
            }
        }
        self.speed = 10.0 + self.wave as f32 * 4.0;
        self.dir = 1.0;
    }

    /// Returns true when the player walks away from the cabinet.
    pub fn update(&mut self, dt: f32, i: &ArcadeIn) -> bool {
        self.t += dt;
        if i.exit {
            return true;
        }
        match self.state {
            0 | 2 => {
                if i.fire && self.t > 0.6 {
                    self.state = 1;
                    self.score = 0;
                    self.lives = 3;
                    self.wave = 0;
                    self.shots.clear();
                    self.bombs.clear();
                    self.formation();
                    self.sounds.push(Id::Coin);
                    self.t = 0.0;
                }
                return false;
            }
            _ => {}
        }
        self.inv -= dt;
        self.cool -= dt;
        self.ship = (self.ship + i.x * 70.0 * dt).clamp(5.0, FW - 5.0);
        if i.fire && self.cool <= 0.0 && self.shots.len() < 3 {
            self.cool = 0.22;
            self.shots.push(vec2(self.ship, FH - 14.0));
            self.sounds.push(Id::Pew);
        }
        for s in &mut self.shots {
            s.y -= 150.0 * dt;
        }
        self.shots.retain(|s| s.y > 0.0);
        // formation marches, steps down at the edges, speeds up as it thins out
        let n = self.aliens.len().max(1) as f32;
        let sp = self.speed * (1.0 + (18.0 - n) * 0.08);
        let (mut minx, mut maxx) = (FW, 0.0f32);
        for a in self.aliens.iter().filter(|a| a.dive.is_none()) {
            minx = minx.min(a.p.x);
            maxx = maxx.max(a.p.x);
        }
        let mut step = false;
        if (maxx > FW - 6.0 && self.dir > 0.0) || (minx < 6.0 && self.dir < 0.0) {
            self.dir = -self.dir;
            step = true;
        }
        let ship = self.ship;
        for a in &mut self.aliens {
            match &mut a.dive {
                None => {
                    a.p.x += self.dir * sp * dt;
                    if step {
                        a.p.y += 4.0;
                    }
                }
                Some(t) => {
                    *t += dt;
                    a.p.y += 55.0 * dt;
                    a.p.x += (ship - a.p.x).signum() * 25.0 * dt + (*t * 6.0).sin() * 20.0 * dt;
                    if a.p.y > FH + 6.0 {
                        a.p.y = 8.0;
                        a.dive = None;
                    }
                }
            }
        }
        if !self.aliens.is_empty() && rand::gen_range(0.0, 1.0) < dt * (0.8 + self.wave as f32 * 0.3) {
            let k = rand::gen_range(0, self.aliens.len());
            if rand::gen_range(0.0, 1.0) < 0.3 {
                self.aliens[k].dive = Some(0.0);
            } else {
                let p = self.aliens[k].p;
                self.bombs.push(p + vec2(0.0, 4.0));
            }
        }
        for b in &mut self.bombs {
            b.y += 60.0 * dt;
        }
        self.bombs.retain(|b| b.y < FH);
        // hits
        let mut dead = Vec::new();
        for (si, s) in self.shots.iter().enumerate() {
            for (ai, a) in self.aliens.iter().enumerate() {
                if (s.x - a.p.x).abs() < 4.0 && (s.y - a.p.y).abs() < 3.5 && !dead.iter().any(|d: &(usize, usize)| d.1 == ai) {
                    dead.push((si, ai));
                    break;
                }
            }
        }
        for (_si, ai) in dead.iter().rev() {
            let a = self.aliens[*ai];
            self.score += if a.dive.is_some() { 50 } else { 10 + (2 - (a.p.y / 12.0) as u32).min(2) * 10 };
            self.booms.push((a.p, 0.0));
            self.sounds.push(Id::Boom8);
        }
        let mut ais: Vec<usize> = dead.iter().map(|d| d.1).collect();
        ais.sort_unstable();
        for ai in ais.iter().rev() {
            self.aliens.remove(*ai);
        }
        let mut sis: Vec<usize> = dead.iter().map(|d| d.0).collect();
        sis.sort_unstable();
        sis.dedup();
        for si in sis.iter().rev() {
            if *si < self.shots.len() {
                self.shots.remove(*si);
            }
        }
        // you get hit
        let shipp = vec2(self.ship, FH - 8.0);
        let hit = self.bombs.iter().any(|b| (b.x - shipp.x).abs() < 4.0 && (b.y - shipp.y).abs() < 3.0)
            || self.aliens.iter().any(|a| (a.p - shipp).length() < 5.0 || a.p.y > FH - 10.0);
        if hit && self.inv <= 0.0 {
            self.lives -= 1;
            self.inv = 1.5;
            self.bombs.clear();
            self.booms.push((shipp, 0.0));
            self.sounds.push(Id::Boom8);
            for a in &mut self.aliens {
                if a.p.y > FH - 30.0 {
                    a.p.y = 10.0;
                    a.dive = None;
                }
            }
            if self.lives <= 0 {
                self.state = 2;
                self.t = 0.0;
                self.hi = self.hi.max(self.score);
            }
        }
        if self.aliens.is_empty() {
            self.wave += 1;
            self.formation();
            self.sounds.push(Id::Coin);
        }
        for b in &mut self.booms {
            b.1 += dt;
        }
        self.booms.retain(|b| b.1 < 0.35);
        self.hi = self.hi.max(self.score);
        false
    }

    pub fn draw(&self, art: &Art, u: f32) {
        let (sw, sh) = (screen_width(), screen_height());
        draw_rectangle(0.0, 0.0, sw, sh, Color::new(0.0, 0.0, 0.0, 0.7));
        let s = ((sh * 0.72) / FH).min((sw * 0.8) / FW).floor().max(1.0);
        let (w, h) = (FW * s, FH * s);
        let (x0, y0) = ((sw - w) * 0.5, (sh - h) * 0.5 + 6.0 * u);
        // cabinet bezel and marquee
        draw_rectangle(x0 - 10.0 * u, y0 - 26.0 * u, w + 20.0 * u, h + 36.0 * u, Color::new(0.27, 0.12, 0.47, 1.0));
        draw_rectangle(x0 - 6.0 * u, y0 - 22.0 * u, w + 12.0 * u, 14.0 * u, Color::new(0.05, 0.02, 0.1, 1.0));
        let title = "STAR COMMUTER";
        let pal = [Color::new(1.0, 0.35, 0.8, 1.0), Color::new(0.35, 0.9, 1.0, 1.0), Color::new(1.0, 0.9, 0.3, 1.0)];
        let tp = ((w + 12.0 * u) / (hud::text_w(title, 1.0) + 4.0)).min(2.0 * u).floor().max(1.0);
        hud::text_c(art, title, sw * 0.5, y0 - 19.0 * u, tp, pal[(self.t * 3.0) as usize % 3]);
        draw_rectangle(x0, y0, w, h, Color::new(0.02, 0.03, 0.08, 1.0));
        let px = |x: f32, y: f32, c: Color| draw_rectangle((x0 + x.floor() * s).round(), (y0 + y.floor() * s).round(), s, s, c);
        let bitmap = |rows: &[&str], cx: f32, cy: f32, c: Color| {
            for (ry, row) in rows.iter().enumerate() {
                for (rx, ch) in row.chars().enumerate() {
                    if ch == '#' {
                        px(cx - 3.0 + rx as f32, cy - 2.0 + ry as f32, c);
                    }
                }
            }
        };
        // stars
        for k in 0..40 {
            let x = (k as f32 * 37.3) % FW;
            let y = (k as f32 * 53.1 + self.t * (10.0 + (k % 3) as f32 * 12.0)) % FH;
            px(x, y, Color::new(0.5, 0.5, 0.7, 0.6));
        }
        let frame = (self.t * 2.0) as usize % 2;
        for a in &self.aliens {
            let c = if a.kind == 0 { Color::new(0.45, 1.0, 0.55, 1.0) } else { Color::new(1.0, 0.45, 0.8, 1.0) };
            let rows = if frame == 0 { ALIENS[a.kind] } else { ALIENS[1 - a.kind] };
            bitmap(&rows, a.p.x, a.p.y, c);
        }
        if self.state == 1 && (self.inv <= 0.0 || (self.t * 12.0) as i32 % 2 == 0) {
            bitmap(&SHIP, self.ship, FH - 8.0, Color::new(0.9, 0.95, 1.0, 1.0));
        }
        for sh in &self.shots {
            px(sh.x, sh.y, Color::new(1.0, 1.0, 0.6, 1.0));
            px(sh.x, sh.y + 1.0, Color::new(1.0, 1.0, 0.6, 1.0));
        }
        for b in &self.bombs {
            px(b.x, b.y, Color::new(1.0, 0.4, 0.3, 1.0));
            px(b.x, b.y + 1.0, Color::new(1.0, 0.4, 0.3, 1.0));
        }
        for (p, t) in &self.booms {
            let r = t * 20.0;
            for k in 0..8 {
                let a = k as f32 * 0.785;
                px(p.x + a.cos() * r, p.y + a.sin() * r, Color::new(1.0, 0.8, 0.3, 1.0 - t * 2.5));
            }
        }
        // scanlines
        let mut y = y0;
        while y < y0 + h {
            draw_rectangle(x0, y, w, 1.0, Color::new(0.0, 0.0, 0.0, 0.25));
            y += 3.0;
        }
        let tu = (s * 0.75).floor().max(1.0);
        hud::text(art, &format!("SCORE {}", self.score), x0 + 2.0 * s, y0 + 2.0 * s, tu, hud::CREAM);
        let hi = format!("HI {}", self.hi);
        hud::text(art, &hi, x0 + w - hud::text_w(&hi, tu) - 2.0 * s, y0 + 2.0 * s, tu, hud::CREAM);
        for l in 0..self.lives.max(0) {
            bitmap(&SHIP, 6.0 + l as f32 * 9.0, FH - 2.0 - 0.5, Color::new(0.6, 0.7, 0.9, 1.0));
        }
        let blink = (self.t * 2.0) as i32 % 2 == 0;
        if self.state == 0 && blink {
            hud::text_c(art, "PRESS FIRE", sw * 0.5, y0 + h * 0.6, tu, hud::GREEN);
        }
        if self.state == 2 {
            hud::text_c(art, "GAME OVER", sw * 0.5, y0 + h * 0.45, tu * 1.5, hud::RED);
            if blink {
                hud::text_c(art, "PRESS FIRE", sw * 0.5, y0 + h * 0.6, tu, hud::GREEN);
            }
        }
        hud::text_c(art, "FREE PLAY", sw * 0.5, y0 + h - 10.0 * s, tu, Color::new(1.0, 0.85, 0.3, if blink { 1.0 } else { 0.4 }));
        hud::text_c(art, "ESC / B TO STEP AWAY", sw * 0.5, y0 + h + 3.0 * u, u, hud::DIM);
    }
}
