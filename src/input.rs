//! Twin-stick input from three sources: keyboard + mouse, touch screens (two
//! virtual sticks), and gamepads. Whichever you touched last becomes the active
//! mode, which also decides the on-screen hints.

use crate::pad::{self, Pads};
use crate::world::Controls;
use macroquad::prelude::*;

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Mode {
    Mouse,
    Touch,
    Pad,
}

#[derive(Default, Clone, Copy)]
pub struct Stick {
    pub id: Option<u64>,
    pub origin: Vec2,
    pub pos: Vec2,
}

impl Stick {
    pub fn value(&self, radius: f32) -> Vec2 {
        if self.id.is_none() {
            return Vec2::ZERO;
        }
        let d = (self.pos - self.origin) / radius;
        if d.length() > 1.0 { d.normalize() } else { d }
    }
}

#[derive(Default)]
pub struct Ui {
    pub confirm: bool,
    pub tap: Option<Vec2>,
    pub choice: Option<usize>,
    pub nav: IVec2,
    pub pause: bool,
    pub mute: bool,
    pub tilt: bool,
    pub interact: bool,
    pub back: bool,
}

pub struct Input {
    pub mode: Mode,
    pub pads: Pads,
    pub left: Stick,
    pub right: Stick,
    pub dash_btn: (Vec2, f32),
    pub bomb_btn: (Vec2, f32),
    pub dash_pressed_vis: f32,
    pub bomb_pressed_vis: f32,
    pub pad_toast: f32,
    seen: Vec<u64>,
    last_mouse: Vec2,
}

impl Input {
    pub fn new() -> Self {
        simulate_mouse_with_touch(false);
        Input {
            mode: Mode::Mouse,
            pads: Pads::new(),
            left: Stick::default(),
            right: Stick::default(),
            dash_btn: (Vec2::ZERO, 0.0),
            bomb_btn: (Vec2::ZERO, 0.0),
            dash_pressed_vis: 0.0,
            bomb_pressed_vis: 0.0,
            pad_toast: 0.0,
            seen: Vec::new(),
            last_mouse: mouse_position().into(),
        }
    }

    pub fn touch_mode(&self) -> bool {
        self.mode == Mode::Touch
    }

    pub fn stick_radius(&self) -> f32 {
        screen_height().min(screen_width()) * 0.13
    }

    fn layout(&mut self) {
        let (sw, sh) = (screen_width(), screen_height());
        let br = (sh.min(sw) * 0.075).max(24.0);
        self.dash_btn = (vec2(br * 1.7, sh * 0.45), br);
        self.bomb_btn = (vec2(sw - br * 1.7, sh * 0.45), br);
    }

    /// `player_screen` is the player's position in full-resolution screen pixels.
    pub fn poll(&mut self, player_screen: Vec2) -> (Controls, Ui) {
        self.layout();
        let dt = get_frame_time();
        self.dash_pressed_vis = (self.dash_pressed_vis - dt).max(0.0);
        self.bomb_pressed_vis = (self.bomb_pressed_vis - dt).max(0.0);
        self.pad_toast = (self.pad_toast - dt).max(0.0);
        let mut c = Controls::default();
        let mut ui = Ui::default();
        let sw = screen_width();

        // ---- touch
        for t in touches() {
            self.mode = Mode::Touch;
            // a quick tap can start and end inside one frame; treat it as a start too
            let quick_tap = matches!(t.phase, TouchPhase::Ended) && !self.seen.contains(&t.id);
            if matches!(t.phase, TouchPhase::Started) {
                self.seen.push(t.id);
            }
            if quick_tap {
                ui.tap = Some(t.position);
                ui.confirm = true;
                if t.position.distance(self.dash_btn.0) < self.dash_btn.1 * 1.3 {
                    c.dash = true;
                } else if t.position.distance(self.bomb_btn.0) < self.bomb_btn.1 * 1.3 {
                    c.grenade = true;
                }
            }
            if matches!(t.phase, TouchPhase::Ended | TouchPhase::Cancelled) {
                self.seen.retain(|&id| id != t.id);
            }
            match t.phase {
                TouchPhase::Started => {
                    ui.tap = Some(t.position);
                    ui.confirm = true;
                    if t.position.distance(self.dash_btn.0) < self.dash_btn.1 * 1.3 {
                        c.dash = true;
                        self.dash_pressed_vis = 0.15;
                    } else if t.position.distance(self.bomb_btn.0) < self.bomb_btn.1 * 1.3 {
                        c.grenade = true;
                        self.bomb_pressed_vis = 0.15;
                    } else if t.position.x < sw * 0.5 {
                        if self.left.id.is_none() {
                            self.left = Stick { id: Some(t.id), origin: t.position, pos: t.position };
                        }
                    } else if self.right.id.is_none() {
                        self.right = Stick { id: Some(t.id), origin: t.position, pos: t.position };
                    }
                }
                TouchPhase::Moved | TouchPhase::Stationary => {
                    if self.left.id == Some(t.id) {
                        self.left.pos = t.position;
                    }
                    if self.right.id == Some(t.id) {
                        self.right.pos = t.position;
                    }
                }
                TouchPhase::Ended | TouchPhase::Cancelled => {
                    if self.left.id == Some(t.id) {
                        self.left.id = None;
                    }
                    if self.right.id == Some(t.id) {
                        self.right.id = None;
                    }
                }
            }
        }
        if self.mode == Mode::Touch {
            let r = self.stick_radius();
            c.mv = self.left.value(r);
            let a = self.right.value(r);
            if a.length() > 0.2 {
                c.aim = Some(a.normalize());
                c.fire = true;
            }
            c.auto_aim = true;
        }

        // ---- gamepad
        self.pads.poll();
        if self.pads.just_connected() {
            self.pad_toast = 2.5;
        }
        if self.pads.active() {
            self.mode = Mode::Pad;
        }
        if self.mode == Mode::Pad && self.pads.cur.connected {
            let p = &self.pads;
            c.mv = p.left();
            let r = p.right();
            if r.length() > 0.35 {
                c.aim = Some(r.normalize());
                c.fire = true;
            }
            if p.trigger(pad::RT) > 0.3 {
                c.fire = true;
                c.auto_aim = c.aim.is_none(); // trigger without the stick: soft lock-on
            }
            c.dash |= p.pressed(pad::A) || p.pressed(pad::LB);
            c.grenade |= p.pressed(pad::RB) || p.pressed(pad::B);
            ui.confirm |= p.pressed(pad::A);
            ui.pause |= p.pressed(pad::START);
            ui.mute |= p.pressed(pad::BACK);
            ui.tilt |= p.pressed(pad::Y);
            ui.interact |= p.pressed(pad::X);
            ui.back |= p.pressed(pad::B);
            ui.nav += p.nav();
        }

        // ---- keyboard + mouse
        let m: Vec2 = mouse_position().into();
        let mouse_moved = m.distance(self.last_mouse) > 3.0;
        self.last_mouse = m;
        let mut mv = Vec2::ZERO;
        if is_key_down(KeyCode::W) { mv.y -= 1.0; }
        if is_key_down(KeyCode::S) { mv.y += 1.0; }
        if is_key_down(KeyCode::A) { mv.x -= 1.0; }
        if is_key_down(KeyCode::D) { mv.x += 1.0; }
        let mut aim = Vec2::ZERO;
        if is_key_down(KeyCode::Up) { aim.y -= 1.0; }
        if is_key_down(KeyCode::Down) { aim.y += 1.0; }
        if is_key_down(KeyCode::Left) { aim.x -= 1.0; }
        if is_key_down(KeyCode::Right) { aim.x += 1.0; }
        let clicked = is_mouse_button_pressed(MouseButton::Left) || is_mouse_button_pressed(MouseButton::Right);
        if mv.length_squared() > 0.0 || aim.length_squared() > 0.0 || mouse_moved || clicked {
            if self.mode != Mode::Touch || mv.length_squared() > 0.0 {
                self.mode = Mode::Mouse;
            }
        }
        if self.mode == Mode::Mouse {
            if mv.length_squared() > 0.0 {
                c.mv = mv.normalize();
            }
            if aim.length_squared() > 0.0 {
                c.aim = Some(aim.normalize());
                c.fire = true;
            } else {
                let d = m - player_screen;
                if d.length() > 2.0 {
                    c.aim = Some(d.normalize());
                }
                c.fire = is_mouse_button_down(MouseButton::Left);
            }
            if is_mouse_button_pressed(MouseButton::Right) {
                c.grenade = true;
            }
            if is_mouse_button_pressed(MouseButton::Left) {
                ui.confirm = true;
                ui.tap = Some(m);
            }
        }
        if is_key_pressed(KeyCode::Space) || is_key_pressed(KeyCode::LeftShift) {
            c.dash = true;
        }
        if is_key_pressed(KeyCode::E) || is_key_pressed(KeyCode::Q) {
            c.grenade = true;
        }
        if is_key_pressed(KeyCode::Enter) || is_key_pressed(KeyCode::Space) {
            ui.confirm = true;
        }
        if is_key_pressed(KeyCode::Escape) || is_key_pressed(KeyCode::P) {
            ui.pause = true;
        }
        if is_key_pressed(KeyCode::M) {
            ui.mute = true;
        }
        if is_key_pressed(KeyCode::T) {
            ui.tilt = true;
        }
        if is_key_pressed(KeyCode::F) {
            ui.interact = true;
        }
        if is_key_pressed(KeyCode::Escape) || is_key_pressed(KeyCode::Backspace) {
            ui.back = true;
        }
        if is_key_pressed(KeyCode::Left) { ui.nav.x -= 1; }
        if is_key_pressed(KeyCode::Right) { ui.nav.x += 1; }
        if is_key_pressed(KeyCode::Up) { ui.nav.y -= 1; }
        if is_key_pressed(KeyCode::Down) { ui.nav.y += 1; }
        for (k, key) in [KeyCode::Key1, KeyCode::Key2, KeyCode::Key3].iter().enumerate() {
            if is_key_pressed(*key) {
                ui.choice = Some(k);
            }
        }
        (c, ui)
    }

    pub fn release_all(&mut self) {
        self.left.id = None;
        self.right.id = None;
    }
}
