//! Plays the synthesized sound bank: one-shots with variation and throttling,
//! distance falloff, zombie groans driven by the horde, and crossfaded loops.

use crate::synth::{self, Id};
use crate::world::{Sfx, World, ZKind};
use macroquad::audio::{load_sound_from_bytes, play_sound, set_sound_volume, stop_sound, PlaySoundParams, Sound};
use macroquad::prelude::*;
use std::collections::HashMap;

const LOOPS: [Id; 4] = [Id::Hum, Id::Music, Id::Ambient, Id::Ride];

struct LoopState {
    cur: f32,
    target: f32,
    playing: bool,
}

pub struct Audio {
    bank: HashMap<Id, Vec<Sound>>,
    last_play: HashMap<Id, f64>,
    last_variant: HashMap<Id, usize>,
    loops: HashMap<Id, LoopState>,
    groan_t: f32,
    pub muted: bool,
}

/// Base mix level and minimum gap between plays (stops machine-gun stacking).
fn mix(id: Id) -> (f32, f64) {
    match id {
        Id::Shot => (0.30, 0.045),
        Id::Tick => (0.12, 0.07),
        Id::Hit => (0.22, 0.035),
        Id::Splat => (0.42, 0.03),
        Id::Groan => (0.30, 0.2),
        Id::Shriek => (0.28, 0.3),
        Id::Growl => (0.4, 0.5),
        Id::Roar => (0.95, 1.0),
        Id::Explode => (0.85, 0.06),
        Id::Slam => (1.0, 0.2),
        Id::Throw => (0.4, 0.1),
        Id::Dash => (0.35, 0.1),
        Id::Hurt => (0.6, 0.15),
        Id::Pickup => (0.5, 0.05),
        Id::Chime => (0.55, 1.0),
        Id::Clear => (0.6, 1.0),
        Id::Horn => (0.7, 1.0),
        Id::Train => (0.85, 1.0),
        Id::GameOver => (0.7, 1.0),
        Id::Victory => (0.7, 1.0),
        Id::Select => (0.4, 0.05),
        Id::Caw => (0.45, 0.6),
        Id::Quake => (1.0, 2.0),
        Id::Scream => (0.5, 0.4),
        Id::Retch => (0.6, 1.0),
        Id::Pew => (0.3, 0.05),
        Id::Boom8 => (0.4, 0.05),
        Id::Coin => (0.5, 0.1),
        Id::Attract => (0.45, 2.0),
        Id::Zap => (0.35, 0.3),
        _ => (1.0, 0.0),
    }
}

impl Audio {
    /// Synthesize and load everything. On the web each sound decodes asynchronously.
    pub async fn load() -> Audio {
        let mut bank: HashMap<Id, Vec<Sound>> = HashMap::new();
        for (id, bytes) in synth::bank() {
            if let Ok(s) = load_sound_from_bytes(&bytes).await {
                bank.entry(id).or_default().push(s);
            }
        }
        let loops = LOOPS.iter().map(|id| (*id, LoopState { cur: 0.0, target: 0.0, playing: false })).collect();
        Audio { bank, last_play: HashMap::new(), last_variant: HashMap::new(), loops, groan_t: 1.0, muted: false }
    }

    pub fn play(&mut self, id: Id, vol: f32) {
        if self.muted || vol <= 0.01 {
            return;
        }
        let (base, gap) = mix(id);
        let now = get_time();
        if let Some(t) = self.last_play.get(&id) {
            if now - t < gap {
                return;
            }
        }
        let Some(list) = self.bank.get(&id) else { return };
        if list.is_empty() {
            return;
        }
        // pick a variant, avoiding an immediate repeat
        let mut k = rand::gen_range(0, list.len());
        if list.len() > 1 && self.last_variant.get(&id) == Some(&k) {
            k = (k + 1) % list.len();
        }
        self.last_variant.insert(id, k);
        self.last_play.insert(id, now);
        let v = (base * vol * rand::gen_range(0.85, 1.0)).min(1.0);
        play_sound(&list[k], PlaySoundParams { looped: false, volume: v });
    }

    fn falloff(world: &World, p: Vec2) -> f32 {
        let d = world.player.pos.distance(p);
        (1.0 - (d - 3.0) / 18.0).clamp(0.05, 1.0)
    }

    /// Turn gameplay events into sounds.
    pub fn handle(&mut self, events: &[Sfx], world: &World) {
        for e in events {
            match *e {
                Sfx::Shot => self.play(Id::Shot, 1.0),
                Sfx::Tick(p) => self.play(Id::Tick, Self::falloff(world, p)),
                Sfx::Hit(p) => self.play(Id::Hit, Self::falloff(world, p)),
                Sfx::Splat(p, kind) => {
                    let v = Self::falloff(world, p) * if kind == ZKind::Walker || kind == ZKind::Runner { 1.0 } else { 1.4 };
                    self.play(Id::Splat, v);
                }
                Sfx::Explode(p) => self.play(Id::Explode, Self::falloff(world, p).max(0.5)),
                Sfx::Slam(p) => self.play(Id::Slam, Self::falloff(world, p).max(0.6)),
                Sfx::Roar => self.play(Id::Roar, 1.0),
                Sfx::Throw => self.play(Id::Throw, 1.0),
                Sfx::Dash => self.play(Id::Dash, 1.0),
                Sfx::Hurt => self.play(Id::Hurt, 1.0),
                Sfx::Pickup => self.play(Id::Pickup, 1.0),
                Sfx::Horn => self.play(Id::Horn, 1.0),
                Sfx::Train => self.play(Id::Train, 1.0),
                Sfx::Chime => self.play(Id::Chime, 1.0),
                Sfx::Clear => self.play(Id::Clear, 1.0),
                Sfx::Caw(p) => self.play(Id::Caw, Self::falloff(world, p)),
                Sfx::Zap(p) => self.play(Id::Zap, Self::falloff(world, p)),
                Sfx::Quake => self.play(Id::Quake, 1.0),
                Sfx::Scream(p) => self.play(Id::Scream, Self::falloff(world, p)),
                Sfx::Retch(p) => self.play(Id::Retch, Self::falloff(world, p)),
                Sfx::Attract(p) => self.play(Id::Attract, Self::falloff(world, p)),
                Sfx::Coin => self.play(Id::Coin, 1.0),
            }
        }
    }

    /// The horde groans more the closer and bigger it gets.
    pub fn horde(&mut self, world: &World, dt: f32) {
        self.groan_t -= dt;
        if self.groan_t > 0.0 || world.demo {
            return;
        }
        let near: Vec<_> = world.zombies.iter().filter(|z| z.pos.distance(world.player.pos) < 11.0 && z.age > 0.5).collect();
        if near.is_empty() {
            self.groan_t = 0.8;
            return;
        }
        let z = near[rand::gen_range(0, near.len())];
        let v = Self::falloff(world, z.pos);
        match z.kind {
            ZKind::Runner => self.play(Id::Shriek, v),
            ZKind::Brute | ZKind::Boss => self.play(Id::Growl, v),
            ZKind::Walker => self.play(Id::Groan, v),
        }
        self.groan_t = (rand::gen_range(1.6, 3.2) / (near.len() as f32).sqrt()).max(0.28);
    }

    /// Set which loops should be playing and at what level; they crossfade.
    pub fn set_loops(&mut self, want: &[(Id, f32)]) {
        for (id, st) in self.loops.iter_mut() {
            st.target = want.iter().find(|w| w.0 == *id).map(|w| w.1).unwrap_or(0.0);
        }
    }

    pub fn update_loops(&mut self, dt: f32) {
        let muted = self.muted;
        for (id, st) in self.loops.iter_mut() {
            let target = if muted { 0.0 } else { st.target };
            let rate = dt * 0.8;
            st.cur = if st.cur < target { (st.cur + rate).min(target) } else { (st.cur - rate).max(target) };
            let Some(s) = self.bank.get(id).and_then(|l| l.first()) else { continue };
            if st.cur > 0.001 {
                if !st.playing {
                    play_sound(s, PlaySoundParams { looped: true, volume: st.cur });
                    st.playing = true;
                } else {
                    set_sound_volume(s, st.cur);
                }
            } else if st.playing {
                stop_sound(s);
                st.playing = false;
            }
        }
    }

    pub fn toggle_mute(&mut self) {
        self.muted = !self.muted;
    }
}
