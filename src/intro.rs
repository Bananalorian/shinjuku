//! The opening: a playable, scripted sequence. The ride in on a crowded Yamanote
//! car, getting off at Akihabara, the quake, waking up in the dark, the first
//! infected (learn to dash), the arcade, and the officer who hands you a pistol.
//! When it ends, the same world flows straight into the Akihabara fight.

use crate::art::Art;
use crate::hud::{self, CREAM, DIM, GREEN, RED};
use crate::input::{Mode, Ui};
use crate::level::{car_def, door_xs, train_stop_head, Tile, CAR_DOORS};
use crate::render::Fx;
use crate::util::*;
use crate::world::*;
use macroquad::prelude::*;

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Stage {
    CarRide,
    CarArrive,
    Platform,
    Escalator,
    Blackout,
    Wake,
    Explore,
    Turn,
    Charge,
    DashPrompt,
    Flee,
    Wander,
    Ambush,
    Overrun,
    FirePrompt,
    Armed,
    Search,
    Done,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Prompt {
    Move,
    Dash,
    Play,
    Fire,
    Search,
    Light,
}

pub struct Dialog {
    pub who: &'static str,
    pub text: String,
    pub t: f32,
    pub dur: f32,
}

/// What the intro needs main.rs to do this frame.
#[derive(Default)]
pub struct Out {
    pub need_platform: bool,
    pub open_arcade: bool,
    pub finished: bool,
    pub skipped: bool,
}

pub struct Intro {
    pub stage: Stage,
    pub t: f32,
    pub clock: f32,
    pub dialog: Option<Dialog>,
    pub prompt: Option<Prompt>,
    pub objective: Option<&'static str>,
    pub black: f32,
    pub marker: Option<Vec2>,
    pub arcade_played: bool,
    lurch_t: f32,
    retch_t: f32,
    attract_t: f32,
    explore_t: f32,
    leaving: bool,
    pub skip_hold: f32,
    vomiter: Option<Vec2>,
    turned: Option<u32>,
    flee_target: Vec2,
    officer: Option<usize>,
    ambush: Vec<u32>,
    shots: u32,
    shot_t: f32,
    feed_t: f32,
    play_btn: Rect,
    pub letterbox: f32,
    cop_spot: Vec2,
    cop_body: Option<usize>,
    light_hint_t: f32,
}

pub fn make_car_world(art: &Art) -> World {
    let def = car_def();
    let mut w = World::from_def(&def, "FOR AKIHABARA", 0, art, Stats::default(), 0, false);
    w.phase = Phase::Intro;
    w.msg = None;
    w.has_gun = false;
    w.has_light = false;
    w.light_on = false;
    w.locked = true;
    w.birds.clear();
    w.car_speed = 9.0;
    w.player.pos = vec2(11.6, 3.3);
    w.player.iframes = 0.0;
    w.player.aim_screen = vec2(0.6, 0.8).normalize();
    w.cam = iso(11.6, 3.3);
    // a 70% crowd: sitters along the back bench, standers holding straps
    let mut k = 0usize;
    for pr in w.map.props.clone().iter().filter(|p| p.kind == crate::level::PropKind::CarSeat && p.pos.y < 2.0) {
        if hash2((pr.pos.x * 2.0) as i32, 1, 9) < 0.8 {
            let mut n = Npc::new(vec2(pr.pos.x + 0.5, pr.pos.y + 0.3), k, NpcState::Sit);
            n.face_left = false;
            w.npcs.push(n);
            k += 1;
        }
    }
    let mut tries = 0;
    while w.npcs.len() < 34 && tries < 400 {
        tries += 1;
        let p = vec2(rnd(1.0, 21.0), rnd(2.0, 4.9));
        if p.distance(w.player.pos) < 1.0 || w.npcs.iter().any(|n| n.pos.distance(p) < 0.62) {
            continue;
        }
        let near_rail = (p.y - 2.2).abs() < 0.5 || (p.y - 4.4).abs() < 0.5;
        let mut n = Npc::new(p, k, if near_rail { NpcState::Strap } else { NpcState::Stand });
        n.back = chance(0.4);
        w.npcs.push(n);
        k += 1;
    }
    w
}

impl Intro {
    pub fn new() -> Intro {
        Intro {
            stage: Stage::CarRide,
            t: 0.0,
            clock: 0.0,
            dialog: None,
            prompt: None,
            objective: None,
            black: 1.0,
            marker: None,
            arcade_played: false,
            lurch_t: 3.0,
            retch_t: 0.0,
            attract_t: 0.0,
            explore_t: 0.0,
            leaving: false,
            skip_hold: 0.0,
            vomiter: None,
            turned: None,
            flee_target: Vec2::ZERO,
            officer: None,
            ambush: Vec::new(),
            shots: 0,
            shot_t: 0.0,
            feed_t: 0.0,
            play_btn: Rect::default(),
            letterbox: 0.0,
            cop_spot: Vec2::ZERO,
            cop_body: None,
            light_hint_t: 0.0,
        }
    }

    fn say(&mut self, who: &'static str, text: &str) {
        let dur = 1.8 + text.len() as f32 * 0.045;
        self.dialog = Some(Dialog { who, text: text.to_string(), t: 0.0, dur });
    }

    fn go(&mut self, s: Stage) {
        self.stage = s;
        self.t = 0.0;
    }

    fn lurch(w: &mut World, dir: Vec2, k: f32) {
        for n in &mut w.npcs {
            if matches!(n.state, NpcState::Stand | NpcState::Strap) {
                n.vel += dir * k * rnd(0.7, 1.2);
            }
        }
        w.player.vel += dir * k * 0.7;
        w.shake = (w.shake + 0.12 * k).min(0.5);
    }

    /// Set up Akihabara for the arrival: the train at the platform, the crowd pouring out.
    pub fn setup_platform(&mut self, w: &mut World) {
        w.phase = Phase::Intro;
        w.msg = None;
        w.has_gun = false;
        w.has_light = false;
        w.light_on = false;
        // where the officer makes his stand: the middle platform, east of the arcade
        if let Some(y) = w.map.track_ys.first() {
            self.cop_spot = vec2(w.map.w as f32 * 0.55, y + 4.4);
        }
        w.locked = false;
        w.birds.clear();
        let ty = *w.map.track_ys.last().unwrap_or(&18.5);
        let head = train_stop_head(w.map.w);
        w.spawn_train(ty, head, head, true, false);
        let doors = door_xs(w.map.w);
        let edge = ty + 1.5;
        let mine = doors[doors.len() / 2];
        w.player.pos = vec2(mine, edge + 0.55);
        w.player.vel = Vec2::ZERO;
        w.player.iframes = 0.0;
        w.cam = iso(w.player.pos.x, w.player.pos.y);
        if let Some((bottom, _)) = w.map.escalator {
            w.npc_field = w.map.bfs(bottom);
            for i in 0..22 {
                let d = doors[i % doors.len()];
                let p = vec2(d + rnd(-0.3, 0.3), edge + rnd(0.45, 1.3));
                let mut n = Npc::new(p, i, NpcState::Stand);
                n.t = -rnd(0.0, 4.0); // staggered start
                n.target = bottom;
                n.after = NpcState::Ride;
                w.npcs.push(n);
            }
            self.marker = Some(bottom);
        }
        self.objective = Some("FOLLOW THE CROWD TO THE ESCALATOR");
        self.go(Stage::Platform);
        self.black = 1.0;
    }

    /// After the quake: the dark, the dead, and someone being sick in the corner.
    pub fn setup_aftermath(&mut self, w: &mut World) {
        w.train = None;
        w.npcs.clear();
        w.light_scale = 1.0;
        w.emergency = 1.0;
        w.player_z = 0.0;
        w.player_lying = true;
        w.locked = true;
        w.shake = 0.0;
        let (bottom, _) = w.map.escalator.unwrap_or((w.player.pos, w.player.pos));
        w.player.pos = bottom + vec2(-1.0, 0.4);
        w.cam = iso(w.player.pos.x, w.player.pos.y);
        let ty = *w.map.track_ys.last().unwrap_or(&18.5);
        let edge = ty + 1.5;
        let h = w.map.h as f32;
        let mut placed = 0;
        let mut tries = 0;
        while placed < 14 && tries < 300 {
            tries += 1;
            let p = vec2(rnd(4.0, w.map.w as f32 - 2.0), rnd(edge + 0.6, h - 0.8));
            if w.map.solid_at(p) || p.distance(w.player.pos) < 1.5 {
                continue;
            }
            let mut n = Npc::new(p, rand::gen_range(0, 8), NpcState::Corpse);
            n.face_left = chance(0.5);
            w.npcs.push(n);
            w.map.stamp(p, rnd(0.5, 0.9), 2);
            placed += 1;
        }
        let v = vec2(2.6, edge + 2.4);
        let mut n = Npc::new(v, 1, NpcState::Kneel);
        n.face_left = true;
        w.npcs.push(n);
        self.vomiter = Some(v);
        self.marker = None;
        self.objective = None;
        self.prompt = None;
        self.black = 1.0;
        self.go(Stage::Wake);
    }

    pub fn update(&mut self, w: &mut World, ctl: &mut Controls, ui: &Ui, _input_mode: Mode, dt: f32, fx: &Fx) -> Out {
        let mut out = Out::default();
        self.t += dt;
        self.clock += dt;
        if let Some(d) = &mut self.dialog {
            d.t += dt;
            if d.t > d.dur {
                self.dialog = None;
            }
        }
        // hold to skip
        let holding = is_key_down(KeyCode::Escape) || is_key_down(KeyCode::Tab) || ui.pause;
        self.skip_hold = if holding { self.skip_hold + dt } else { (self.skip_hold - dt * 2.0).max(0.0) };
        if let Some(tap) = ui.tap {
            let (sw, _) = (screen_width(), screen_height());
            if tap.x > sw - 120.0 && tap.y < 70.0 {
                self.skip_hold = 2.0;
            }
        }
        if self.skip_hold > 1.0 {
            out.finished = true;
            out.skipped = true;
            return out;
        }
        let _ = fx;

        match self.stage {
            // ------------------------------------------------ on the train
            Stage::CarRide => {
                self.black = (1.0 - self.t * 0.8).max(0.0);
                let at = self.t;
                if at > 0.8 && at - dt <= 0.8 {
                    w.sfx.push(Sfx::Chime);
                    self.say("ANNOUNCEMENT", "THANK YOU FOR RIDING THE YAMANOTE LINE.");
                }
                if at > 6.0 && at - dt <= 6.0 {
                    self.say("ANNOUNCEMENT", "THIS TRAIN IS BOUND FOR UENO AND IKEBUKURO.");
                }
                if at > 12.0 && at - dt <= 12.0 {
                    w.sfx.push(Sfx::Chime);
                    self.say("ANNOUNCEMENT", "THE NEXT STATION IS KANDA. THE DOORS ON THE LEFT SIDE WILL OPEN.");
                }
                if at > 20.0 {
                    w.car_speed = approach(w.car_speed, 0.0, dt * 3.2);
                }
                if at > 20.0 && at - dt <= 20.0 {
                    Self::lurch(w, vec2(1.0, 0.0), 2.0);
                }
                // the car sways side to side and everyone sways with it
                self.lurch_t -= dt;
                if self.lurch_t <= 0.0 && w.car_speed > 3.0 {
                    self.lurch_t = rnd(3.0, 5.5);
                    let dir = if chance(0.5) { vec2(0.0, 1.0) } else { vec2(0.0, -1.0) };
                    Self::lurch(w, dir, rnd(1.2, 2.2));
                }
                if at > 23.0 {
                    w.car_speed = 0.0;
                    w.car_doors_open = true;
                    w.sfx.push(Sfx::Chime);
                    self.say("ANNOUNCEMENT", "KANDA. KANDA.");
                    w.locked = false;
                    self.prompt = Some(Prompt::Move);
                    self.objective = Some("GET OFF THE TRAIN");
                    self.go(Stage::CarArrive);
                }
            }
            Stage::CarArrive => {
                // everyone shuffles toward the open doors
                let h = w.map.h as f32;
                for n in w.npcs.iter_mut() {
                    let go = match n.state {
                        NpcState::Stand | NpcState::Strap => self.t > (n.pos.x - w.player.pos.x).abs() * 0.15,
                        NpcState::Sit => self.t > 2.5 + n.pos.x * 0.1,
                        _ => false,
                    };
                    if go {
                        let d = *CAR_DOORS.iter().min_by(|a, b| (*a - n.pos.x).abs().partial_cmp(&(*b - n.pos.x).abs()).unwrap()).unwrap();
                        n.state = NpcState::Walk;
                        n.target = vec2(d, h - 0.1);
                        n.after = NpcState::Gone;
                        n.speed = rnd(1.2, 1.7);
                    }
                }
                if ctl.mv.length() > 0.2 && self.t > 1.0 {
                    self.prompt = None;
                }
                let p = w.player.pos;
                let at_door = CAR_DOORS.iter().any(|d| (p.x - d).abs() < 0.75) && p.y > h - 0.75;
                if at_door && !self.leaving {
                    self.leaving = true;
                    self.t = 0.0;
                }
                if self.leaving {
                    w.locked = true;
                    self.black = (self.t * 2.0).min(1.0);
                    if self.t > 0.6 {
                        out.need_platform = true;
                        self.leaving = false;
                    }
                }
            }
            // ------------------------------------------------ Akihabara, before
            Stage::Platform => {
                self.black = (1.0 - self.t * 1.5).max(0.0);
                if self.prompt == Some(Prompt::Move) && ctl.mv.length() > 0.2 && self.t > 1.5 {
                    self.prompt = None;
                }
                for n in w.npcs.iter_mut() {
                    if n.state == NpcState::Stand && n.t > 0.0 {
                        n.state = NpcState::Walk;
                        n.speed = rnd(1.3, 1.9);
                    }
                    // the escalator only takes one person at a time
                    if n.state == NpcState::Ride && n.t < 0.02 {
                        n.t = 0.02;
                    }
                }
                if self.t > 12.0 {
                    if let Some(t) = &mut w.train {
                        if !t.pass {
                            t.doors_open = false;
                            t.pass = true;
                            t.target = w.map.gw as f32 + 45.0;
                            w.sfx.push(Sfx::Horn);
                        }
                    }
                }
                if let Some((bottom, _)) = w.map.escalator {
                    if w.player.pos.distance(bottom) < 0.9 {
                        w.locked = true;
                        self.objective = None;
                        self.marker = None;
                        self.prompt = None;
                        self.go(Stage::Escalator);
                    }
                }
            }
            Stage::Escalator => {
                if let Some((a, b)) = w.map.escalator {
                    let ride = 8.5;
                    let f = (self.t / ride).min(1.0);
                    w.player.pos = a.lerp(b, f);
                    w.player.vel = Vec2::ZERO;
                    w.player_z = f * crate::art::esc_height(crate::art::ESC_SLICES - 1) + 2.0;
                    w.player.aim_screen = vec2(0.4, -0.9).normalize();
                    w.cam_focus = Some(w.player.pos);
                    // a third of the way up, the ground starts to shake
                    let quake_at = 3.2;
                    if self.t > quake_at && self.t - dt <= quake_at {
                        w.sfx.push(Sfx::Quake);
                        self.say("YOU", "...?");
                    }
                    if self.t > quake_at {
                        w.shake = (w.shake + dt * 0.8).min(1.0);
                        let wild = ((self.t - quake_at) / 3.0).min(1.0);
                        w.light_scale = if chance(0.12 + wild * 0.3) { rnd(0.0, 0.4) } else { rnd(0.7, 1.0) };
                        if chance(dt * 2.0) {
                            let p = w.player.pos + rand_dir() * rnd(3.0, 8.0);
                            w.sfx.push(Sfx::Scream(p));
                        }
                    }
                    if self.t > quake_at + 3.6 {
                        w.light_scale = 0.0;
                        self.black = ((self.t - quake_at - 3.6) * 1.2).min(1.0);
                    }
                    if self.t > quake_at + 4.6 {
                        w.cam_focus = None;
                        self.go(Stage::Blackout);
                    }
                }
            }
            Stage::Blackout => {
                self.black = 1.0;
                if self.t > 3.0 {
                    self.setup_aftermath(w);
                }
            }
            // ------------------------------------------------ Akihabara, after
            Stage::Wake => {
                self.black = (1.0 - self.t * 0.35).max(0.0);
                if self.t > 2.8 && w.player_lying {
                    w.player_lying = false;
                    self.say("YOU", "MY HEAD... WHAT HAPPENED?");
                }
                if self.t > 4.0 {
                    w.locked = false;
                    self.objective = Some("LOOK AROUND");
                    self.go(Stage::Explore);
                }
            }
            Stage::Explore => {
                if let Some(v) = self.vomiter {
                    self.retch_t -= dt;
                    if self.retch_t <= 0.0 {
                        self.retch_t = rnd(2.0, 3.5);
                        w.sfx.push(Sfx::Retch(v));
                    }
                    if chance(dt * 6.0) {
                        w.vomit(v + vec2(-0.2, 0.1), vec2(-0.6, 0.4));
                    }
                    if w.player.pos.distance(v) < 4.6 {
                        // they stop retching, and look up at you
                        w.npcs.retain(|n| n.state != NpcState::Kneel);
                        w.add_zombie(ZKind::Runner, v);
                        if let Some(z) = w.zombies.last_mut() {
                            z.speed = 0.0;
                            z.age = 1.0;
                            z.variant = 0;
                            self.turned = Some(z.id);
                        }
                        w.sfx.push(Sfx::Scream(v));
                        self.say("???", "...HHHRRK...");
                        self.go(Stage::Turn);
                    }
                }
            }
            Stage::Turn => {
                if let Some(z) = self.turned.and_then(|id| w.zombies.iter_mut().find(|z| z.id == id)) {
                    z.attack_cd = 9.0;
                    if self.t > 1.3 {
                        z.speed = 4.4;
                        self.go(Stage::Charge);
                    }
                }
            }
            Stage::Charge => {
                let ppos = w.player.pos;
                if let Some(z) = self.turned.and_then(|id| w.zombies.iter_mut().find(|z| z.id == id)) {
                    z.attack_cd = 9.0;
                    if z.pos.distance(ppos) < 1.8 {
                        w.freeze = true;
                        self.prompt = Some(Prompt::Dash);
                        self.go(Stage::DashPrompt);
                    }
                } else {
                    self.go(Stage::Wander);
                }
            }
            Stage::DashPrompt => {
                if ctl.dash {
                    w.freeze = false;
                    self.prompt = None;
                    if ctl.mv.length() < 0.2 {
                        // dash sideways, away from the lunge
                        if let Some(z) = self.turned.and_then(|id| w.zombies.iter().find(|z| z.id == id)) {
                            let away = w.player.pos - z.pos;
                            let side = vec2(-away.y, away.x).normalize_or_zero();
                            ctl.mv = world_to_screen_dir(side).normalize_or_zero();
                        }
                    }
                    // the infected stumbles past and bolts for the tracks
                    let ty = *w.map.track_ys.last().unwrap_or(&18.5);
                    if let Some(z) = self.turned.and_then(|id| w.zombies.iter().find(|z| z.id == id)) {
                        let gap = door_xs(w.map.w).into_iter().min_by(|a, b| (a - z.pos.x).abs().partial_cmp(&(b - z.pos.x).abs()).unwrap()).unwrap_or(4.5);
                        self.flee_target = vec2(gap, ty);
                    }
                    self.go(Stage::Flee);
                } else {
                    // keep the world frozen until they press dash
                    w.freeze = true;
                }
            }
            Stage::Flee => {
                let target = self.flee_target;
                let mut alive = false;
                let mut on_track = false;
                if let Some(z) = self.turned.and_then(|id| w.zombies.iter_mut().find(|z| z.id == id)) {
                    alive = true;
                    z.attack_cd = 9.0;
                    z.speed = 0.0; // the script steers it now
                    if self.t > 0.35 {
                        // first to the gap in the screen doors, then out onto the rails
                        let mouth = vec2(target.x, target.y + 2.1);
                        if z.pos.y < mouth.y + 0.05 || z.pos.distance(mouth) < 0.2 {
                            self.leaving = true;
                        }
                        let goal = if self.leaving { target } else { mouth };
                        let d = goal - z.pos;
                        if d.length() > 0.1 {
                            z.pos += d.normalize() * (4.2 * dt).min(d.length());
                            z.vel = d.normalize() * 4.2;
                        } else {
                            z.vel = Vec2::ZERO;
                        }
                        on_track = (z.pos.y - target.y).abs() < 0.8;
                    }
                }
                if alive && on_track && w.train.is_none() {
                    // it stands on the rails, staring... and an out-of-service train barrels through
                    w.spawn_train(target.y, target.x - 28.0, w.map.gw as f32 + 45.0, false, true);
                    w.sfx.push(Sfx::Horn);
                }
                if !alive && w.train.is_none() && self.t > 2.0 {
                    self.leaving = false;
                    self.say("YOU", "WHAT IS HAPPENING...?");
                    self.objective = Some("EXPLORE THE STATION");
                    self.go(Stage::Wander);
                }
            }
            Stage::Wander => {
                self.explore_t += dt;
                if let Some(a) = w.map.arcade {
                    self.attract_t -= dt;
                    if self.attract_t <= 0.0 && w.player.pos.distance(a) < 12.0 {
                        self.attract_t = 6.0;
                        w.sfx.push(Sfx::Attract(a));
                    }
                    let near = w.player.pos.distance(a) < 1.3;
                    self.prompt = if near { Some(Prompt::Play) } else if self.prompt == Some(Prompt::Play) { None } else { self.prompt };
                    let tapped = ui.tap.map_or(false, |t| self.play_btn.contains(t));
                    if near && (ui.interact || tapped) {
                        out.open_arcade = true;
                        self.prompt = None;
                    }
                }
                // walk near the middle platform after poking around, and the officer's stand plays out
                let near_spot = w.player.pos.distance(self.cop_spot) < 9.0;
                if (self.arcade_played || self.explore_t > 40.0) && (near_spot || self.explore_t > 100.0) {
                    if self.start_ambush(w) {
                        self.go(Stage::Ambush);
                    }
                } else if self.arcade_played && self.objective != Some("FIND A WAY OUT") {
                    self.objective = Some("FIND A WAY OUT");
                    self.marker = Some(self.cop_spot);
                }
            }
            Stage::Ambush => {
                // a cutscene you watch: the camera pans over, the bars come in
                let Some(oi) = self.officer else { self.go(Stage::Wander); return out };
                w.locked = true;
                self.letterbox = (self.letterbox + dt * 2.0).min(1.0);
                self.prompt = None;
                let opos = w.npcs.get(oi).map(|n| n.pos).unwrap_or(w.player.pos);
                w.cam_focus = Some(opos.lerp(w.player.pos, 0.25));
                let mut nearest: Option<(Vec2, f32)> = None;
                for z in w.zombies.iter_mut().filter(|z| self.ambush.contains(&z.id)) {
                    z.attack_cd = 9.0;
                    z.speed = 0.0;
                    let d = opos - z.pos;
                    if d.length() > 0.6 {
                        z.pos += d.normalize() * 0.85 * dt;
                        z.vel = d.normalize() * 0.85;
                    }
                    let dist = d.length();
                    if nearest.map_or(true, |(_, nd)| dist < nd) {
                        nearest = Some((z.pos, dist));
                    }
                }
                if self.t > 0.6 && self.t - dt <= 0.6 {
                    self.say("OFFICER", "STAY BACK! I SAID STAY BACK!");
                }
                if self.t > 4.0 && self.t - dt <= 4.0 {
                    // more of them shamble out of the dark
                    let away = (opos - w.player.pos).normalize_or_zero();
                    for j in 0..2 {
                        let zp = opos + away * 4.5 + vec2(-away.y, away.x) * (j as f32 * 2.0 - 1.0) * 1.4;
                        w.add_zombie(ZKind::Walker, zp);
                        if let Some(z) = w.zombies.last_mut() {
                            z.age = 1.0;
                            self.ambush.push(z.id);
                        }
                    }
                    self.say("OFFICER", "THERE'S TOO MANY OF THEM...");
                }
                self.shot_t -= dt;
                if let Some((zp, dist)) = nearest {
                    if let Some(n) = w.npcs.get_mut(oi) {
                        n.aim = zp - opos;
                    }
                    if self.shot_t <= 0.0 && self.shots < 7 && self.t > 1.0 {
                        self.shot_t = 0.75;
                        self.shots += 1;
                        w.fire_bullet(opos, zp - opos, 1.2);
                    }
                    if dist < 0.75 && self.t > 6.0 {
                        self.say("OFFICER", "KID! IT'S TOO DANGEROUS HERE... TAKE THIS!");
                        let to = w.player.pos - opos;
                        let land = opos + to * 0.8;
                        let mut pk = Pickup::new(opos, PickupKind::Pistol);
                        pk.z = 10.0;
                        pk.vz = 150.0;
                        pk.vel = (land - opos) / 0.7;
                        w.pickups.push(pk);
                        if let Some(n) = w.npcs.get_mut(oi) {
                            n.state = NpcState::Corpse;
                            n.flashlight = true;
                            n.loot = 60;
                        }
                        self.cop_body = Some(oi);
                        w.blood_burst(opos, (opos - zp).normalize_or_zero(), 24);
                        w.sfx.push(Sfx::Scream(opos));
                        self.feed_t = 3.0;
                        self.go(Stage::Overrun);
                    }
                } else if self.t > 2.0 {
                    let away = (opos - w.player.pos).normalize_or_zero();
                    w.add_zombie(ZKind::Walker, opos + away * 3.0);
                    if let Some(z) = w.zombies.last() {
                        self.ambush.push(z.id);
                    }
                }
            }
            Stage::Overrun => {
                self.feed_t -= dt;
                let opos = self.officer.and_then(|i| w.npcs.get(i)).map(|n| n.pos).unwrap_or(w.player.pos);
                for z in w.zombies.iter_mut().filter(|z| self.ambush.contains(&z.id)) {
                    // they crowd over him and feed until you're armed
                    z.attack_cd = 9.0;
                    z.speed = 0.0;
                    let d = opos - z.pos;
                    if d.length() > 0.7 {
                        z.pos += d.normalize() * 1.2 * dt;
                    }
                    z.vel = Vec2::ZERO;
                }
                if self.t > 1.6 {
                    w.cam_focus = None;
                    w.locked = false;
                    self.letterbox = (self.letterbox - dt * 2.0).max(0.0);
                    self.objective = Some("GRAB THE PISTOL");
                }
                if w.has_gun {
                    w.cam_focus = None;
                    w.locked = false;
                    self.letterbox = 0.0;
                    w.freeze = true;
                    self.prompt = Some(Prompt::Fire);
                    self.objective = Some("PUT THEM DOWN");
                    self.go(Stage::FirePrompt);
                }
            }
            Stage::FirePrompt => {
                // freeze until they pull the trigger once
                if ctl.fire {
                    w.freeze = false;
                    self.prompt = None;
                    for z in w.zombies.iter_mut().filter(|z| self.ambush.contains(&z.id)) {
                        z.speed = 1.4;
                        z.attack_cd = 0.6;
                    }
                    self.go(Stage::Armed);
                } else {
                    w.freeze = true;
                }
            }
            Stage::Armed => {
                for z in w.zombies.iter_mut().filter(|z| self.ambush.contains(&z.id)) {
                    z.speed = z.speed.max(1.4);
                }
                let left = w.zombies.iter().filter(|z| self.ambush.contains(&z.id)).count();
                if left == 0 && self.t > 0.5 {
                    self.objective = Some("SEARCH THE OFFICER");
                    self.marker = self.cop_body.and_then(|i| w.npcs.get(i)).map(|n| n.pos);
                    self.go(Stage::Search);
                }
            }
            Stage::Search => {
                if let Some(i) = self.cop_body {
                    let near = w.npcs.get(i).map_or(false, |n| n.pos.distance(w.player.pos) < 1.3);
                    if self.light_hint_t <= 0.0 {
                        self.prompt = if near { Some(Prompt::Search) } else { None };
                    }
                    let auto = cfg!(not(target_arch = "wasm32")) && std::env::var("SJ_AUTO").is_ok() && self.t > 1.0;
                    if near && (ui.interact || auto) && self.light_hint_t <= 0.0 {
                        w.search(i);
                        self.marker = None;
                        self.objective = None;
                        self.prompt = Some(Prompt::Light);
                        self.light_hint_t = 5.0;
                        self.say("YOU", "A FLASHLIGHT. AND SOME COINS. SORRY, OFFICER.");
                    }
                    if self.light_hint_t > 0.0 {
                        self.light_hint_t -= dt;
                        if self.light_hint_t <= 0.0 {
                            self.prompt = None;
                            self.say("YOU", "THE SHOTS... MORE OF THEM ARE COMING.");
                            self.go(Stage::Done);
                        }
                    }
                } else {
                    self.go(Stage::Done);
                }
            }
            Stage::Done => {
                w.emergency = (w.emergency - dt * 0.4).max(0.0);
                if self.t > 2.5 {
                    self.prompt = None;
                    self.objective = None;
                    w.emergency = 0.0;
                    w.phase = Phase::Fight;
                    w.kills = 0;
                    w.say("JY02  KANDA", "EMERGENCY POWER ON. HOLD OUT UNTIL A TRAIN COMES.", 4.5);
                    out.finished = true;
                }
            }
        }

        // the officer leaving his post: pick a lit spot in view, a few tiles away
        out
    }

    fn start_ambush(&mut self, w: &mut World) -> bool {
        let p = w.player.pos;
        for k in 0..41 {
            let a = k as f32 * 0.7;
            let spot = if k == 0 && self.cop_spot.distance(p) < 10.0 { self.cop_spot } else { p + vec2(a.cos(), a.sin()) * rnd(6.0, 8.0) };
            if spot.x < 2.5 || spot.y < 2.0 || spot.x > w.map.w as f32 - 2.0 || spot.y > w.map.h as f32 - 1.0 {
                continue;
            }
            if w.map.tile(spot.x, spot.y) == Tile::Track || w.map.solid_at(spot) {
                continue;
            }
            let mut n = Npc::new(spot, 0, NpcState::Shoot);
            n.officer = true;
            w.npcs.push(n);
            self.officer = Some(w.npcs.len() - 1);
            self.marker = None;
            let away = (spot - p).normalize_or_zero();
            for j in 0..3 {
                let zp = spot + away * rnd(2.8, 3.6) + vec2(-away.y, away.x) * (j as f32 - 1.0) * 1.2;
                w.add_zombie(ZKind::Walker, zp);
                if let Some(z) = w.zombies.last_mut() {
                    z.age = 1.0;
                    self.ambush.push(z.id);
                }
            }
            w.sfx.push(Sfx::Shot);
            self.objective = Some("SOMEONE'S SHOOTING");
            self.prompt = None;
            return true;
        }
        false
    }

    // ------------------------------------------------------------ overlay

    pub fn draw(&mut self, art: &Art, u: f32, mode: Mode, w: &World, fx: &Fx) {
        let (sw, sh) = (screen_width(), screen_height());
        // objective marker bouncing over the target
        if let Some(m) = self.marker {
            let s = fx.to_screen(w, m, 30.0 + (self.clock * 5.0).sin() * 3.0);
            hud::text_c(art, "V", s.x, s.y, 2.0 * u, GREEN);
        }
        if let Some(o) = self.objective {
            hud::text(art, o, 6.0 * u, 6.0 * u, u, CREAM);
        }
        if self.stage != Stage::Done {
            let skip = match mode {
                Mode::Touch => "TAP HERE TO SKIP",
                Mode::Pad => "HOLD START TO SKIP",
                Mode::Mouse => "HOLD ESC TO SKIP",
            };
            let a = if self.skip_hold > 0.0 { 1.0 } else { 0.5 };
            hud::text(art, skip, sw - hud::text_w(skip, u) - 6.0 * u, 6.0 * u, u, with_alpha(DIM, a));
            if self.skip_hold > 0.0 {
                draw_rectangle(sw - 6.0 * u - hud::text_w(skip, u), 15.0 * u, hud::text_w(skip, u) * self.skip_hold.min(1.0), u, CREAM);
            }
        }
        if w.has_gun || matches!(self.stage, Stage::Explore | Stage::Wander | Stage::Turn | Stage::Charge | Stage::Flee | Stage::Ambush | Stage::Overrun) {
            hud::text(art, "HP", 6.0 * u, sh - 19.0 * u, u, CREAM);
            let k = w.player.hp / w.stats.max_hp;
            draw_rectangle(6.0 * u, sh - 10.0 * u, 70.0 * u, 4.0 * u, Color::new(0.18, 0.16, 0.2, 1.0));
            draw_rectangle(6.0 * u, sh - 10.0 * u, 70.0 * u * k, 4.0 * u, RED);
        }

        // dialogue box
        if let Some(d) = &self.dialog {
            let bw = (sw * 0.86).min(300.0 * u);
            let bh = 30.0 * u;
            let bx = (sw - bw) * 0.5;
            let by = sh - bh - 22.0 * u - sh * 0.11 * self.letterbox;
            let a = (d.t * 5.0).min(1.0) * ((d.dur - d.t) * 3.0).clamp(0.0, 1.0);
            draw_rectangle(bx, by, bw, bh, Color::new(0.03, 0.03, 0.05, 0.85 * a));
            draw_rectangle(bx, by, bw, u, with_alpha(if d.who == "ANNOUNCEMENT" { GREEN } else { CREAM }, a));
            hud::text(art, d.who, bx + 6.0 * u, by + 5.0 * u, u, with_alpha(if d.who == "ANNOUNCEMENT" { GREEN } else { RED }, a));
            let shown = ((d.t * 40.0) as usize).min(d.text.len());
            let line = &d.text[..shown];
            let px = {
                let mut p = u;
                while p > 1.0 && hud::text_w(&d.text, p) > bw - 12.0 * u {
                    p -= 1.0;
                }
                p
            };
            // wrap long lines in two
            if hud::text_w(&d.text, u) > bw - 12.0 * u {
                let cut = d.text[..d.text.len() / 2 + 1].rfind(' ').unwrap_or(d.text.len() / 2);
                let (l1, l2) = d.text.split_at(cut);
                let s1 = &l1[..shown.min(l1.len())];
                let s2 = if shown > l1.len() { &l2[..(shown - l1.len()).min(l2.len())] } else { "" };
                hud::text(art, s1, bx + 6.0 * u, by + 14.0 * u, u, with_alpha(CREAM, a));
                hud::text(art, s2.trim_start(), bx + 6.0 * u, by + 22.0 * u, u, with_alpha(CREAM, a));
            } else {
                hud::text(art, line, bx + 6.0 * u, by + 15.0 * u, px, with_alpha(CREAM, a));
            }
        }

        // tutorial prompt
        if let Some(p) = self.prompt {
            let text = match (p, mode) {
                (Prompt::Move, Mode::Touch) => "DRAG ON THE LEFT SIDE TO MOVE",
                (Prompt::Move, Mode::Pad) => "LEFT STICK TO MOVE",
                (Prompt::Move, Mode::Mouse) => "WASD TO MOVE",
                (Prompt::Dash, Mode::Touch) => "TAP DASH!",
                (Prompt::Dash, Mode::Pad) => "PRESS A TO DASH!",
                (Prompt::Dash, Mode::Mouse) => "PRESS SPACE TO DASH!",
                (Prompt::Play, Mode::Touch) => "TAP PLAY",
                (Prompt::Play, Mode::Pad) => "PRESS X TO PLAY",
                (Prompt::Play, Mode::Mouse) => "PRESS F TO PLAY",
                (Prompt::Fire, Mode::Touch) => "DRAG ON THE RIGHT SIDE TO AIM AND FIRE",
                (Prompt::Fire, Mode::Pad) => "RIGHT STICK TO AIM AND FIRE",
                (Prompt::Fire, Mode::Mouse) => "AIM WITH THE MOUSE, HOLD LEFT CLICK TO FIRE",
                (Prompt::Search, Mode::Touch) => "TAP USE TO SEARCH",
                (Prompt::Search, Mode::Pad) => "PRESS X TO SEARCH",
                (Prompt::Search, Mode::Mouse) => "PRESS F TO SEARCH",
                (Prompt::Light, Mode::Touch) => "TAP LIGHT TO TURN YOUR FLASHLIGHT ON AND OFF",
                (Prompt::Light, Mode::Pad) => "D-PAD UP TURNS YOUR FLASHLIGHT ON AND OFF",
                (Prompt::Light, Mode::Mouse) => "PRESS L TO TURN YOUR FLASHLIGHT ON AND OFF",
            };
            let big = matches!(p, Prompt::Dash | Prompt::Fire);
            let px = if big { 3.0 * u } else { 2.0 * u };
            let mut px2 = px;
            while px2 > u && hud::text_w(text, px2) > sw * 0.9 {
                px2 -= 1.0;
            }
            let y = if big { sh * 0.42 } else { sh * 0.22 };
            let pulse = 0.75 + 0.25 * (self.clock * 6.0).sin();
            if big {
                draw_rectangle(0.0, y - 8.0 * u, sw, 7.0 * px2 + 16.0 * u, Color::new(0.0, 0.0, 0.0, 0.5));
            }
            hud::text_c(art, text, sw * 0.5, y, px2, with_alpha(if big { GREEN } else { CREAM }, pulse));
            if p == Prompt::Play && mode == Mode::Touch {
                let bw = 60.0 * u;
                self.play_btn = Rect::new((sw - bw) * 0.5, y + 14.0 * u, bw, 16.0 * u);
                draw_rectangle(self.play_btn.x, self.play_btn.y, bw, 16.0 * u, Color::new(0.2, 0.5, 0.25, 0.9));
                hud::text_c(art, "PLAY", sw * 0.5, self.play_btn.y + 4.5 * u, u, CREAM);
            }
        }

        if self.letterbox > 0.0 {
            let bar = sh * 0.11 * self.letterbox;
            draw_rectangle(0.0, 0.0, sw, bar, BLACK);
            draw_rectangle(0.0, sh - bar, sw, bar, BLACK);
        }
        if self.black > 0.0 {
            draw_rectangle(0.0, 0.0, sw, sh, Color::new(0.0, 0.0, 0.0, self.black));
        }
        if self.stage == Stage::Blackout && self.t > 1.0 {
            hud::text_c(art, "...", sw * 0.5, sh * 0.5, 2.0 * u, with_alpha(DIM, ((self.t - 1.0) * 2.0).min(1.0)));
        }
    }
}
