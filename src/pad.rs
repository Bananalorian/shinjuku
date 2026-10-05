//! Gamepads. Desktop builds read controllers through gilrs; the web build reads the
//! browser Gamepad API through a tiny JS plugin that lives in web/template.html.
//! Both are normalized to the W3C "standard" layout (Xbox naming below).

use macroquad::prelude::*;

pub const A: usize = 0;
pub const B: usize = 1;
pub const Y: usize = 3;
pub const X: usize = 2;
pub const LB: usize = 4;
pub const RB: usize = 5;
pub const RT: usize = 7;
pub const BACK: usize = 8;
pub const START: usize = 9;
pub const UP: usize = 12;
pub const DOWN: usize = 13;
pub const LEFT: usize = 14;
pub const RIGHT: usize = 15;
const NB: usize = 17;

#[derive(Default, Clone, Copy)]
pub struct PadState {
    pub connected: bool,
    pub axes: [f32; 4], // left x, left y, right x, right y (y points down)
    pub buttons: [f32; NB],
}

pub struct Pads {
    pub cur: PadState,
    prev: PadState,
    rumble_until: f64,
    rumble_level: f32,
    #[cfg(not(target_arch = "wasm32"))]
    native: native::Native,
}

fn deadzone(v: Vec2, dz: f32) -> Vec2 {
    let l = v.length();
    if l < dz { Vec2::ZERO } else { v / l * ((l - dz) / (1.0 - dz)).min(1.0) }
}

impl Pads {
    pub fn new() -> Self {
        Pads {
            cur: PadState::default(),
            prev: PadState::default(),
            rumble_until: 0.0,
            rumble_level: 0.0,
            #[cfg(not(target_arch = "wasm32"))]
            native: native::Native::new(),
        }
    }

    pub fn poll(&mut self) {
        self.prev = self.cur;
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.cur = self.native.read();
        }
        #[cfg(target_arch = "wasm32")]
        {
            self.cur = web::read();
        }
    }

    pub fn just_connected(&self) -> bool {
        self.cur.connected && !self.prev.connected
    }
    pub fn pressed(&self, b: usize) -> bool {
        self.cur.buttons[b] > 0.5 && self.prev.buttons[b] <= 0.5
    }
    pub fn trigger(&self, b: usize) -> f32 {
        self.cur.buttons[b]
    }
    pub fn left(&self) -> Vec2 {
        deadzone(vec2(self.cur.axes[0], self.cur.axes[1]), 0.2)
    }
    pub fn right(&self) -> Vec2 {
        deadzone(vec2(self.cur.axes[2], self.cur.axes[3]), 0.25)
    }
    fn prev_left(&self) -> Vec2 {
        deadzone(vec2(self.prev.axes[0], self.prev.axes[1]), 0.2)
    }

    /// Menu navigation from the d-pad or a flick of the left stick: (-1, 0, 1) per axis.
    pub fn nav(&self) -> IVec2 {
        let mut n = IVec2::ZERO;
        if self.pressed(LEFT) { n.x -= 1; }
        if self.pressed(RIGHT) { n.x += 1; }
        if self.pressed(UP) { n.y -= 1; }
        if self.pressed(DOWN) { n.y += 1; }
        let (l, p) = (self.left(), self.prev_left());
        if l.x < -0.6 && p.x >= -0.6 { n.x -= 1; }
        if l.x > 0.6 && p.x <= 0.6 { n.x += 1; }
        if l.y < -0.6 && p.y >= -0.6 { n.y -= 1; }
        if l.y > 0.6 && p.y <= 0.6 { n.y += 1; }
        n
    }

    pub fn active(&self) -> bool {
        self.cur.connected
            && (self.left().length() > 0.3 || self.right().length() > 0.3 || self.cur.buttons.iter().any(|b| *b > 0.5))
    }

    /// Rumble, if the controller and platform support it. A weaker request
    /// never cuts off a stronger one that's still playing.
    pub fn rumble(&mut self, strong: f32, weak: f32, ms: u32) {
        let now = get_time();
        let level = strong.max(weak);
        if now < self.rumble_until && level < self.rumble_level {
            return;
        }
        self.rumble_until = now + ms as f64 / 1000.0;
        self.rumble_level = level;
        #[cfg(not(target_arch = "wasm32"))]
        self.native.rumble(strong, weak, ms);
        #[cfg(target_arch = "wasm32")]
        web::rumble(strong, weak, ms);
    }
}

#[cfg(target_arch = "wasm32")]
mod web {
    use super::{PadState, NB};

    unsafe extern "C" {
        fn sj_pad_poll(ptr: *mut f32, len: u32) -> u32;
        fn sj_pad_rumble(strong: f32, weak: f32, ms: f32);
    }

    /// Lets the JS loader confirm the plugin matches this build.
    #[unsafe(no_mangle)]
    pub extern "C" fn sj_gamepad_crate_version() -> u32 {
        1
    }

    pub fn read() -> PadState {
        let mut buf = [0.0f32; 24];
        let ok = unsafe { sj_pad_poll(buf.as_mut_ptr(), buf.len() as u32) };
        let mut s = PadState::default();
        if ok == 1 {
            s.connected = true;
            s.axes.copy_from_slice(&buf[2..6]);
            s.buttons.copy_from_slice(&buf[6..6 + NB]);
        }
        s
    }

    pub fn rumble(strong: f32, weak: f32, ms: u32) {
        unsafe { sj_pad_rumble(strong, weak, ms as f32) };
    }
}

#[cfg(not(target_arch = "wasm32"))]
mod native {
    use super::{PadState, NB};
    use gilrs::{Axis, Button, GamepadId, Gilrs};

    const MAP: [Button; NB] = [
        Button::South,
        Button::East,
        Button::West,
        Button::North,
        Button::LeftTrigger,
        Button::RightTrigger,
        Button::LeftTrigger2,
        Button::RightTrigger2,
        Button::Select,
        Button::Start,
        Button::LeftThumb,
        Button::RightThumb,
        Button::DPadUp,
        Button::DPadDown,
        Button::DPadLeft,
        Button::DPadRight,
        Button::Mode,
    ];

    pub struct Native {
        gilrs: Option<Gilrs>,
        active: Option<GamepadId>,
        effect: Option<gilrs::ff::Effect>,
    }

    impl Native {
        pub fn new() -> Self {
            Native { gilrs: Gilrs::new().ok(), active: None, effect: None }
        }

        pub fn read(&mut self) -> PadState {
            let mut s = PadState::default();
            let Some(g) = &mut self.gilrs else { return s };
            while let Some(ev) = g.next_event() {
                self.active = Some(ev.id);
            }
            if self.active.map_or(true, |id| !g.gamepad(id).is_connected()) {
                self.active = g.gamepads().next().map(|(id, _)| id);
            }
            let Some(id) = self.active else { return s };
            let gp = g.gamepad(id);
            s.connected = true;
            s.axes = [gp.value(Axis::LeftStickX), -gp.value(Axis::LeftStickY), gp.value(Axis::RightStickX), -gp.value(Axis::RightStickY)];
            for (i, b) in MAP.iter().enumerate() {
                let analog = gp.button_data(*b).map(|d| d.value()).unwrap_or(0.0);
                s.buttons[i] = if gp.is_pressed(*b) { analog.max(1.0) } else { analog.min(0.49) };
            }
            s
        }

        pub fn rumble(&mut self, strong: f32, weak: f32, ms: u32) {
            use gilrs::ff::{BaseEffect, BaseEffectType, EffectBuilder, Replay, Ticks};
            let (Some(g), Some(id)) = (&mut self.gilrs, self.active) else { return };
            if !g.gamepad(id).is_ff_supported() {
                return;
            }
            let rep = Replay { play_for: Ticks::from_ms(ms), ..Default::default() };
            let mag = |k: f32| (k.clamp(0.0, 1.0) * 65535.0) as u16;
            let eff = EffectBuilder::new()
                .add_effect(BaseEffect { kind: BaseEffectType::Strong { magnitude: mag(strong) }, scheduling: rep, ..Default::default() })
                .add_effect(BaseEffect { kind: BaseEffectType::Weak { magnitude: mag(weak) }, scheduling: rep, ..Default::default() })
                .gamepads(&[id])
                .finish(g);
            if let Ok(e) = eff {
                let _ = e.play();
                self.effect = Some(e); // keep it alive until the next one replaces it
            }
        }
    }
}
