//! The coin lockers: permanent upgrades bought with banked coins before each run.

use crate::art::Art;
use crate::hud::{self, text, text_c, text_w, CREAM, DIM, GREEN, RED};
use crate::input::Mode;
use crate::storage::Meta;
use crate::world::Stats;
use macroquad::prelude::*;

pub struct Perk {
    pub key: &'static str,
    pub name: &'static str,
    pub desc: &'static str,
    pub costs: &'static [u32],
}

pub const PERKS: [Perk; 9] = [
    Perk { key: "hp", name: "TOUGHER", desc: "+15 MAX HP", costs: &[30, 60, 110, 180, 280] },
    Perk { key: "dmg", name: "STEADY HANDS", desc: "+10% DAMAGE", costs: &[40, 80, 140, 220, 330] },
    Perk { key: "rate", name: "QUICK TRIGGER", desc: "+8% FIRE RATE", costs: &[40, 80, 140, 220, 330] },
    Perk { key: "speed", name: "RUNNING SHOES", desc: "+5% MOVE SPEED", costs: &[30, 70, 130] },
    Perk { key: "dash", name: "DASH TRAINING", desc: "-10% DASH COOLDOWN", costs: &[30, 70, 130] },
    Perk { key: "nade", name: "GRENADE POUCH", desc: "-12% GRENADE COOLDOWN", costs: &[30, 70, 130] },
    Perk { key: "luck", name: "LUCKY", desc: "+25% COINS FROM SMASHING", costs: &[25, 60, 120] },
    Perk { key: "magnet", name: "MAGNET", desc: "COINS PULL IN FROM FARTHER", costs: &[20, 50] },
    Perk { key: "start", name: "HEAD START", desc: "START WITH A FREE CARD", costs: &[150, 400] },
];

fn level(m: &Meta, key: &str) -> f32 {
    PERKS.iter().position(|p| p.key == key).map_or(0.0, |i| m.levels[i] as f32)
}

/// Your stats at the start of a run, with every permanent upgrade applied.
pub fn starting_stats(m: &Meta) -> Stats {
    let mut s = Stats::default();
    s.max_hp += 15.0 * level(m, "hp");
    s.dmg *= 1.0 + 0.10 * level(m, "dmg");
    s.rate *= 1.0 + 0.08 * level(m, "rate");
    s.speed *= 1.0 + 0.05 * level(m, "speed");
    s.dash_cd *= 1.0 - 0.10 * level(m, "dash");
    s.gren_cd *= 1.0 - 0.12 * level(m, "nade");
    s
}

pub fn coin_mult(m: &Meta) -> f32 {
    1.0 + 0.25 * level(m, "luck")
}

pub fn magnet(m: &Meta) -> f32 {
    2.6 + 1.5 * level(m, "magnet")
}

pub fn head_start(m: &Meta) -> usize {
    level(m, "start") as usize
}

/// What the player did on the shop screen this frame.
pub enum Action {
    None,
    Start,
    Back,
}

pub struct Shop {
    pub sel: usize,
    flash: f32,
    flash_ok: bool,
    rows: Vec<Rect>,
}

impl Shop {
    pub fn new() -> Shop {
        Shop { sel: PERKS.len() + 1, flash: 0.0, flash_ok: true, rows: Vec::new() }
    }

    fn row_count(m: &Meta) -> usize {
        // the perks, then the opening toggle (once you've seen it), then START RUN
        PERKS.len() + 2 - if m.seen_intro { 0 } else { 1 }
    }

    /// `buy` = confirm pressed; returns what to do next. Plays its own sounds through the flag it returns.
    pub fn update(&mut self, m: &mut Meta, nav_y: i32, confirm: bool, back: bool, tap: Option<Vec2>, mouse: Option<Vec2>, dt: f32) -> (Action, Option<bool>) {
        self.flash = (self.flash - dt).max(0.0);
        let n = Self::row_count(m);
        if self.sel >= n {
            self.sel = n - 1;
        }
        if nav_y != 0 {
            self.sel = (self.sel as i32 + nav_y).rem_euclid(n as i32) as usize;
        }
        if let Some(mp) = mouse {
            if let Some(i) = self.rows.iter().position(|r| r.contains(mp)) {
                self.sel = i;
            }
        }
        let mut press = confirm;
        if let Some(t) = tap {
            if let Some(i) = self.rows.iter().position(|r| r.contains(t)) {
                // first tap selects, a second tap on the same row presses it
                press = i == self.sel;
                self.sel = i;
            }
        }
        if back {
            return (Action::Back, None);
        }
        if !press {
            return (Action::None, None);
        }
        let start_row = n - 1;
        if self.sel == start_row {
            return (Action::Start, None);
        }
        if self.sel == PERKS.len() && m.seen_intro {
            m.skip_intro = !m.skip_intro;
            m.save();
            return (Action::None, Some(true));
        }
        let i = self.sel;
        let lvl = m.levels[i] as usize;
        if lvl >= PERKS[i].costs.len() {
            self.flash = 0.4;
            self.flash_ok = false;
            return (Action::None, Some(false));
        }
        let cost = PERKS[i].costs[lvl];
        if m.bank < cost {
            self.flash = 0.4;
            self.flash_ok = false;
            return (Action::None, Some(false));
        }
        m.bank -= cost;
        m.levels[i] += 1;
        m.save();
        self.flash = 0.4;
        self.flash_ok = true;
        (Action::None, Some(true))
    }

    pub fn draw(&mut self, art: &Art, u: f32, m: &Meta, mode: Mode, t: f32) {
        let (sw, sh) = (screen_width(), screen_height());
        clear_background(Color::new(0.04, 0.04, 0.06, 1.0));
        let gold = Color::new(1.0, 0.85, 0.3, 1.0);
        let title = "COIN LOCKERS";
        let tp = hud::fit(title, 3.0 * u, sw * 0.55);
        text_c(art, title, sw * 0.5, 8.0 * u, tp, CREAM);
        let sub = "PERMANENT UPGRADES. THEY CARRY INTO EVERY RUN.";
        text_c(art, sub, sw * 0.5, 8.0 * u + 7.0 * tp + 4.0 * u, hud::fit(sub, u, sw * 0.9), DIM);
        let bank = format!("BANK {}", m.bank);
        let bp = hud::fit(&bank, 2.0 * u, sw * 0.2);
        text(art, &bank, sw - text_w(&bank, bp) - 8.0 * u, 8.0 * u, bp, gold);

        let n = Self::row_count(m);
        let top = 8.0 * u + 7.0 * tp + 16.0 * u;
        let bottom_pad = 18.0 * u;
        let rh = ((sh - top - bottom_pad) / n as f32).min(15.0 * u).max(9.0);
        let px = if rh >= 12.0 * u { u } else { (rh / 12.0).max(1.0).floor() };
        let w = (sw * 0.9).min(330.0 * u);
        let x0 = (sw - w) * 0.5;
        self.rows.clear();
        for i in 0..n {
            let y = top + i as f32 * rh;
            let r = Rect::new(x0, y, w, rh - 2.0 * px);
            self.rows.push(r);
            let on = i == self.sel;
            let flash = on && self.flash > 0.0;
            let bg = if flash {
                if self.flash_ok { Color::new(0.2, 0.45, 0.2, 0.9) } else { Color::new(0.5, 0.12, 0.12, 0.9) }
            } else if on {
                Color::new(0.16, 0.18, 0.24, 1.0)
            } else {
                Color::new(0.09, 0.1, 0.13, 1.0)
            };
            draw_rectangle(r.x, r.y, r.w, r.h, bg);
            // a locker door: number plate on the left
            draw_rectangle(r.x, r.y, 3.0 * px, r.h, if on { GREEN } else { Color::new(0.3, 0.32, 0.36, 1.0) });
            let ty = r.y + (r.h - 7.0 * px) * 0.5;
            if i < PERKS.len() {
                let p = &PERKS[i];
                let lvl = m.levels[i] as usize;
                let max = p.costs.len();
                text(art, &format!("{:02}", i + 1), r.x + 6.0 * px, ty, px, DIM);
                text(art, p.name, r.x + 20.0 * px, ty, px, if on { CREAM } else { Color::new(0.8, 0.8, 0.78, 1.0) });
                text(art, p.desc, r.x + 20.0 * px + text_w("GRENADE POUCH  ", px), ty, px, DIM);
                // level pips
                let pip_x = r.x + r.w - 52.0 * px;
                for k in 0..max {
                    let c = if k < lvl { GREEN } else { Color::new(0.25, 0.27, 0.3, 1.0) };
                    draw_rectangle(pip_x + k as f32 * 4.0 * px, ty + 2.0 * px, 3.0 * px, 3.0 * px, c);
                }
                let price = if lvl >= max { "MAX".to_string() } else { format!("{}", p.costs[lvl]) };
                let afford = lvl < max && m.bank >= p.costs[lvl];
                let pc = if lvl >= max { DIM } else if afford { gold } else { RED };
                text(art, &price, r.x + r.w - text_w(&price, px) - 6.0 * px, ty, px, pc);
            } else if i == PERKS.len() && m.seen_intro {
                let s = if m.skip_intro { "OPENING: SKIP IT, START AT AKIHABARA" } else { "OPENING: PLAY IT FROM THE TRAIN" };
                text(art, s, r.x + 20.0 * px, ty, px, if on { CREAM } else { DIM });
            } else {
                let pulse = 0.75 + 0.25 * (t * 4.0).sin();
                text_c(art, "START RUN", r.x + r.w * 0.5, ty, px, with_alpha(GREEN, if on { pulse } else { 0.7 }));
            }
        }
        let hint = match mode {
            Mode::Touch => "TAP A LOCKER, TAP AGAIN TO BUY",
            Mode::Pad => "D-PAD TO CHOOSE   A TO BUY   B FOR TITLE",
            Mode::Mouse => "UP/DOWN OR MOUSE TO CHOOSE   ENTER OR CLICK TO BUY   ESC FOR TITLE",
        };
        text_c(art, hint, sw * 0.5, sh - 12.0 * u, hud::fit(hint, u, sw * 0.95), DIM);
    }
}

fn with_alpha(c: Color, a: f32) -> Color {
    Color::new(c.r, c.g, c.b, a)
}
