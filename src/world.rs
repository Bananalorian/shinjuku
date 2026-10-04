//! Gameplay: the player, zombies, bullets, grenades, particles, the train, and
//! the boss. No drawing happens here; render.rs reads this state.

use crate::level::*;
use crate::util::*;
use macroquad::prelude::*;

pub const PLAYER_R: f32 = 0.28;
const MAX_PARTICLES: usize = 3600;

// ---------------------------------------------------------------- upgrades

#[derive(Clone, Copy, Debug)]
pub struct Stats {
    pub multishot: i32,
    pub dmg: f32,
    pub rate: f32,
    pub pierce: i32,
    pub gren_cd: f32,
    pub gren_radius: f32,
    pub max_hp: f32,
    pub speed: f32,
    pub dash_cd: f32,
}

impl Default for Stats {
    fn default() -> Self {
        Stats { multishot: 1, dmg: 1.0, rate: 10.0, pierce: 1, gren_cd: 6.0, gren_radius: 2.6, max_hp: 100.0, speed: 4.6, dash_cd: 1.3 }
    }
}

pub struct Upgrade {
    pub name: &'static str,
    pub desc: &'static str,
    pub apply: fn(&mut Stats),
}

pub fn upgrades() -> Vec<Upgrade> {
    vec![
        Upgrade { name: "TWIN BARREL", desc: "+1 BULLET PER SHOT", apply: |s| s.multishot += 1 },
        Upgrade { name: "HOLLOW POINTS", desc: "+40% DAMAGE", apply: |s| s.dmg *= 1.4 },
        Upgrade { name: "HAIR TRIGGER", desc: "+25% FIRE RATE", apply: |s| s.rate *= 1.25 },
        Upgrade { name: "PIERCING ROUNDS", desc: "BULLETS PIERCE +1", apply: |s| s.pierce += 1 },
        Upgrade { name: "FRAG BELT", desc: "BIGGER, FASTER NADES", apply: |s| { s.gren_cd *= 0.65; s.gren_radius *= 1.15 } },
        Upgrade { name: "KONBINI RUN", desc: "+25 MAX HP, HEAL", apply: |s| s.max_hp += 25.0 },
        Upgrade { name: "SNEAKERS", desc: "+12% SPEED, DASH", apply: |s| { s.speed *= 1.12; s.dash_cd *= 0.75 } },
    ]
}

// ---------------------------------------------------------------- input from input.rs

#[derive(Default, Clone, Copy)]
pub struct Controls {
    pub mv: Vec2,          // screen-space, length <= 1
    pub aim: Option<Vec2>, // screen-space unit direction
    pub fire: bool,
    pub dash: bool,
    pub grenade: bool,
    pub auto_aim: bool,
}

// ---------------------------------------------------------------- entities

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum ZKind {
    Walker,
    Runner,
    Brute,
    Boss,
}

#[derive(Clone, Copy)]
pub struct Zombie {
    pub id: u32,
    pub pos: Vec2,
    pub vel: Vec2,
    pub knock: Vec2,
    pub hp: f32,
    pub max_hp: f32,
    pub kind: ZKind,
    pub variant: usize,
    pub r: f32,
    pub speed: f32,
    pub anim: f32,
    pub flash: f32,
    pub age: f32,
    pub attack_cd: f32,
    pub face_left: bool,
    pub back: bool,
    pub dead: bool,
    pub slam_t: f32,
    pub slam_cd: f32,
}

#[derive(Clone, Copy)]
pub struct Bullet {
    pub pos: Vec2,
    pub vel: Vec2,
    pub life: f32,
    pub dmg: f32,
    pub pierce: i32,
    hits: [u32; 8],
    nh: usize,
}

#[derive(Clone, Copy)]
pub struct Grenade {
    pub pos: Vec2,
    pub vel: Vec2,
    pub z: f32,
    pub vz: f32,
    pub fuse: f32,
}

#[derive(Clone, Copy)]
pub struct Pickup {
    pub pos: Vec2,
    pub t: f32,
}

#[derive(Clone, Copy, PartialEq)]
pub enum PK {
    Blood,
    Gib,
    Spark,
    Smoke,
    Casing,
    Ember,
    Debris,
    Dust,
    Flash,
}

#[derive(Clone, Copy)]
pub struct Particle {
    pub p: Vec3, // world x, y, height in px
    pub v: Vec3,
    pub life: f32,
    pub max: f32,
    pub size: f32,
    pub color: Color,
    pub kind: PK,
    pub rest: bool,
}

impl Particle {
    pub fn additive(&self) -> bool {
        matches!(self.kind, PK::Spark | PK::Ember | PK::Dust | PK::Flash)
    }
}

#[derive(Clone, Copy)]
pub struct TempLight {
    pub pos: Vec2,
    pub radius: f32,
    pub color: Color,
    pub life: f32,
    pub max: f32,
}

#[derive(Clone, Copy)]
pub struct Ring {
    pub pos: Vec2,
    pub r: f32,
    pub t: f32,
    pub max: f32,
    pub color: Color,
}

#[derive(Clone, Copy)]
pub struct Ghost {
    pub pos: Vec2,
    pub back: bool,
    pub face_left: bool,
    pub frame: usize,
    pub life: f32,
}

pub struct Player {
    pub pos: Vec2,
    pub vel: Vec2,
    pub hp: f32,
    pub aim: Vec2,
    pub aim_screen: Vec2,
    pub fire_cd: f32,
    pub dash_t: f32,
    pub dash_cd: f32,
    pub dash_dir: Vec2,
    pub iframes: f32,
    pub gren_cd: f32,
    pub anim: f32,
    pub face_left: bool,
    pub back: bool,
    pub moving: bool,
    pub muzzle: f32,
}

#[derive(Clone, Copy, PartialEq)]
pub enum SliceK {
    Cab,
    Body,
    Door,
    Gap,
}

pub struct Train {
    pub head: f32,
    pub target: f32,
    pub y: f32,
    pub slices: Vec<SliceK>,
    pub stopped_t: f32,
    pub doors_open: bool,
}

impl Train {
    pub fn slice_x(&self, i: usize) -> f32 {
        self.head - 1.0 - i as f32
    }
    pub fn tail(&self) -> f32 {
        self.head - self.slices.len() as f32
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Phase {
    Fight,
    Boss,
    Train,
    Won,
    Dead,
}

/// Sound cues for audio.rs; the world never touches the audio API directly.
#[derive(Clone, Copy, Debug)]
pub enum Sfx {
    Shot,
    Tick(Vec2),
    Hit(Vec2),
    Splat(Vec2, ZKind),
    Explode(Vec2),
    Slam(Vec2),
    Roar,
    Throw,
    Dash,
    Hurt,
    Pickup,
    Horn,
    Train,
    Chime,
    Clear,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Event {
    Boarded,
    Died,
    Won,
}

pub struct Msg {
    pub title: String,
    pub sub: String,
    pub t: f32,
    pub dur: f32,
}

pub struct World {
    pub station: usize,
    pub name: &'static str,
    pub code: &'static str,
    pub ambient: Color,
    pub map: Map,
    pub stats: Stats,
    pub player: Player,
    pub zombies: Vec<Zombie>,
    pub bullets: Vec<Bullet>,
    pub grenades: Vec<Grenade>,
    pub pickups: Vec<Pickup>,
    pub particles: Vec<Particle>,
    pub lights: Vec<TempLight>,
    pub rings: Vec<Ring>,
    pub ghosts: Vec<Ghost>,
    pub kills: u32,
    pub quota: u32,
    pub total_kills: u32,
    pub time: f32,
    pub phase: Phase,
    phase_t: f32,
    pub train: Option<Train>,
    pub shake: f32,
    pub flash: f32,
    pub hurt: f32,
    pub shock: Option<(Vec2, f32)>,
    pub cam: Vec2,
    pub msg: Option<Msg>,
    pub events: Vec<Event>,
    pub sfx: Vec<Sfx>,
    pub has_boss: bool,
    pub slowmo: f32,
    max_alive: usize,
    spawn_rate: (f32, f32),
    mix: [f32; 3],
    spawn_acc: f32,
    next_id: u32,
    grid: Vec<Vec<u16>>,
    dust_acc: f32,
    pub demo: bool,
}

fn emit(ps: &mut Vec<Particle>, p: Particle) {
    if ps.len() < MAX_PARTICLES {
        ps.push(p);
    }
}

fn particle(kind: PK, pos: Vec2, z: f32, v: Vec3, life: f32, size: f32, color: Color) -> Particle {
    Particle { p: vec3(pos.x, pos.y, z), v, life, max: life, size, color, kind, rest: false }
}

fn blood_spray(ps: &mut Vec<Particle>, pos: Vec2, z: f32, dir: Vec2, n: usize, power: f32) {
    for _ in 0..n {
        let d = (dir * rnd(0.3, 1.0) + rand_dir() * rnd(0.0, 0.9)).normalize_or_zero();
        let sp = rnd(1.0, 4.5) * power;
        let c = Color::new(rnd(0.45, 0.75), rnd(0.0, 0.05), rnd(0.02, 0.07), 1.0);
        emit(ps, particle(PK::Blood, pos, z, vec3(d.x * sp, d.y * sp, rnd(40.0, 160.0) * power), rnd(0.6, 1.2), rnd(1.0, 2.0), c));
    }
}

fn gibs(ps: &mut Vec<Particle>, pos: Vec2, n: usize, power: f32) {
    for _ in 0..n {
        let d = rand_dir();
        let sp = rnd(1.0, 3.5) * power;
        let c = if chance(0.3) { Color::new(0.35, 0.42, 0.3, 1.0) } else { Color::new(rnd(0.3, 0.5), 0.02, 0.04, 1.0) };
        emit(ps, particle(PK::Gib, pos, rnd(6.0, 14.0), vec3(d.x * sp, d.y * sp, rnd(90.0, 200.0)), rnd(1.5, 2.5), 2.0, c));
    }
}

impl World {
    pub fn new(station: usize, defs: &[StationDef], art: &crate::art::Art, stats: Stats, total_kills: u32, demo: bool) -> World {
        let def = &defs[station];
        let map = Map::build(def, station, art);
        let start_y = if map.track_ys.len() >= 2 {
            (map.track_ys[0] + map.track_ys[1]) * 0.5
        } else {
            map.track_ys.first().map(|y| y + 3.5).unwrap_or(4.0)
        };
        let mut start = vec2(map.w as f32 * 0.5 + 0.5, start_y + 1.6);
        for _ in 0..4 {
            map.resolve(&mut start, PLAYER_R + 0.1, true);
        }
        let grid = vec![Vec::new(); (map.gw * map.gh) as usize];
        let mut w = World {
            station,
            name: def.name,
            code: def.code,
            ambient: def.ambient,
            stats,
            player: Player {
                pos: start,
                vel: Vec2::ZERO,
                hp: stats.max_hp,
                aim: vec2(1.0, 0.0),
                aim_screen: vec2(1.0, 0.5).normalize(),
                fire_cd: 0.0,
                dash_t: 0.0,
                dash_cd: 0.0,
                dash_dir: Vec2::X,
                iframes: 1.5,
                gren_cd: 0.0,
                anim: 0.0,
                face_left: false,
                back: false,
                moving: false,
                muzzle: 0.0,
            },
            zombies: Vec::new(),
            bullets: Vec::new(),
            grenades: Vec::new(),
            pickups: Vec::new(),
            particles: Vec::new(),
            lights: Vec::new(),
            rings: Vec::new(),
            ghosts: Vec::new(),
            kills: 0,
            quota: def.quota,
            total_kills,
            time: 0.0,
            phase: Phase::Fight,
            phase_t: 0.0,
            train: None,
            shake: 0.0,
            flash: 0.0,
            hurt: 0.0,
            shock: None,
            cam: iso(start.x, start.y),
            msg: if demo {
                None
            } else {
                Some(Msg { title: format!("{}  {}", def.code, def.name), sub: def.tagline.to_string(), t: 0.0, dur: 4.5 })
            },
            events: Vec::new(),
            sfx: Vec::new(),
            has_boss: def.boss,
            slowmo: 1.0,
            max_alive: def.max_alive,
            spawn_rate: def.spawn_rate,
            mix: def.mix,
            spawn_acc: 0.0,
            next_id: 1,
            grid,
            dust_acc: 0.0,
            demo,
            map,
        };
        w.map.update_flow(w.player.pos);
        w
    }

    pub fn boss(&self) -> Option<&Zombie> {
        self.zombies.iter().find(|z| z.kind == ZKind::Boss && !z.dead)
    }

    fn say(&mut self, title: &str, sub: &str, dur: f32) {
        self.msg = Some(Msg { title: title.to_string(), sub: sub.to_string(), t: 0.0, dur });
    }

    // ------------------------------------------------------------ main update

    pub fn update(&mut self, raw_dt: f32, c: &Controls) {
        let dt = raw_dt.min(1.0 / 30.0) * self.slowmo;
        self.time += dt;
        self.phase_t += dt;
        if let Some(m) = &mut self.msg {
            m.t += raw_dt;
            if m.t > m.dur {
                self.msg = None;
            }
        }

        if !self.demo && self.phase != Phase::Dead {
            self.update_player(dt, c);
        }
        self.map.update_flow(self.player.pos);
        if !self.demo {
            self.spawn(dt);
        }
        self.update_zombies(dt);
        self.update_bullets(dt);
        self.update_grenades(dt);
        self.update_pickups(dt);
        self.update_train(dt);
        self.update_particles(dt);
        self.update_phase(dt);

        // ambient dust motes drifting through the lamplight
        self.dust_acc += dt * 10.0;
        while self.dust_acc >= 1.0 {
            self.dust_acc -= 1.0;
            let p = self.player.pos + vec2(rnd(-9.0, 9.0), rnd(-9.0, 9.0));
            emit(&mut self.particles, particle(PK::Dust, p, rnd(10.0, 50.0), vec3(rnd(-0.1, 0.1), rnd(-0.1, 0.1), rnd(-2.0, 2.0)), rnd(3.0, 6.0), 1.0, Color::new(1.0, 0.95, 0.85, 0.35)));
        }

        for l in &mut self.lights {
            l.life -= dt;
        }
        self.lights.retain(|l| l.life > 0.0);
        for r in &mut self.rings {
            r.t += dt;
        }
        self.rings.retain(|r| r.t < r.max);
        for g in &mut self.ghosts {
            g.life -= dt;
        }
        self.ghosts.retain(|g| g.life > 0.0);
        self.zombies.retain(|z| !z.dead);

        // camera follows with a little look-ahead toward where you aim
        let target = iso(self.player.pos.x, self.player.pos.y) + self.player.aim_screen * 18.0 + vec2(0.0, -8.0);
        self.cam += (target - self.cam) * (1.0 - (-6.0 * raw_dt).exp());
        self.shake *= (-7.0 * raw_dt).exp();
        self.flash = (self.flash - raw_dt * 3.0).max(0.0);
        self.hurt = (self.hurt - raw_dt * 1.6).max(0.0);
        if let Some((_, t)) = &mut self.shock {
            *t += raw_dt;
            if *t > 0.8 {
                self.shock = None;
            }
        }
        self.slowmo = approach(self.slowmo, 1.0, raw_dt * 0.6);
        self.map.flush();
    }

    // ------------------------------------------------------------ player

    fn update_player(&mut self, dt: f32, c: &Controls) {
        let st = self.stats;
        let p = &mut self.player;
        p.fire_cd -= dt;
        p.dash_cd -= dt;
        p.iframes -= dt;
        p.gren_cd -= dt;
        p.muzzle -= dt;

        let mut aim_screen = c.aim;
        let mut fire = c.fire;
        if c.auto_aim && aim_screen.is_none() {
            // phones: lock onto the nearest zombie when the right stick is idle
            let mut best = 8.0f32;
            let mut tgt = None;
            for z in &self.zombies {
                let d = z.pos.distance(p.pos);
                if d < best && z.age > 0.3 {
                    best = d;
                    tgt = Some(z.pos);
                }
            }
            if let Some(t) = tgt {
                aim_screen = Some(world_to_screen_dir(t - p.pos).normalize_or_zero());
                fire = true;
            }
        }
        if let Some(a) = aim_screen {
            if a.length_squared() > 0.0 {
                p.aim_screen = a.normalize();
                p.aim = screen_to_world_dir(p.aim_screen).normalize_or_zero();
            }
        }

        let mv_world = if c.mv.length() > 0.05 { screen_to_world_dir(c.mv).normalize_or_zero() * c.mv.length().min(1.0) } else { Vec2::ZERO };
        if c.dash && p.dash_cd <= 0.0 {
            p.dash_t = 0.17;
            p.dash_cd = st.dash_cd;
            p.iframes = p.iframes.max(0.28);
            p.dash_dir = if mv_world.length() > 0.1 { mv_world.normalize() } else { p.aim };
            self.sfx.push(Sfx::Dash);
            for _ in 0..10 {
                emit(&mut self.particles, particle(PK::Smoke, p.pos + rand_dir() * 0.2, 2.0, vec3(-p.dash_dir.x * 1.5, -p.dash_dir.y * 1.5, 6.0), 0.5, 3.0, Color::new(0.75, 0.75, 0.8, 0.5)));
            }
        }
        if p.dash_t > 0.0 {
            p.dash_t -= dt;
            p.vel = p.dash_dir * 13.0;
            self.ghosts.push(Ghost { pos: p.pos, back: p.back, face_left: p.face_left, frame: (p.anim as usize) % 4, life: 0.25 });
        } else {
            let target = mv_world * st.speed;
            p.vel += (target - p.vel) * (1.0 - (-14.0 * dt).exp());
        }
        p.moving = p.vel.length() > 0.4;
        p.pos += p.vel * dt;
        self.map.resolve(&mut p.pos, PLAYER_R, true);
        if let Some(t) = &self.train {
            push_out_of_train(t, &mut p.pos, PLAYER_R);
        }
        if p.moving {
            p.anim += dt * p.vel.length() * 2.2;
        } else {
            p.anim = 0.0;
        }
        p.face_left = p.aim_screen.x < 0.0;
        p.back = p.aim_screen.y < -0.45;

        // shooting
        if fire && p.fire_cd <= 0.0 {
            p.fire_cd = 1.0 / st.rate;
            p.muzzle = 0.05;
            self.sfx.push(Sfx::Shot);
            let base = p.aim.y.atan2(p.aim.x);
            let n = st.multishot;
            let muzzle = p.pos + p.aim * 0.5;
            for k in 0..n {
                let a = base + (k as f32 - (n - 1) as f32 * 0.5) * 0.11 + rnd(-0.035, 0.035);
                let d = vec2(a.cos(), a.sin());
                self.bullets.push(Bullet { pos: muzzle, vel: d * 26.0, life: 0.85, dmg: st.dmg, pierce: st.pierce, hits: [0; 8], nh: 0 });
            }
            self.lights.push(TempLight { pos: muzzle, radius: 2.8, color: Color::new(1.0, 0.8, 0.45, 1.0), life: 0.06, max: 0.06 });
            emit(&mut self.particles, particle(PK::Flash, muzzle, 10.0, Vec3::ZERO, 0.05, 6.0, Color::new(1.0, 0.85, 0.5, 1.0)));
            let side = vec2(-p.aim.y, p.aim.x);
            emit(&mut self.particles, particle(PK::Casing, p.pos + p.aim * 0.2, 9.0, vec3(side.x * rnd(1.0, 2.0), side.y * rnd(1.0, 2.0), rnd(60.0, 100.0)), 2.5, 1.0, WHITE));
            self.shake = (self.shake + 0.02).min(1.0);
        }

        if c.grenade && p.gren_cd <= 0.0 {
            p.gren_cd = st.gren_cd;
            self.sfx.push(Sfx::Throw);
            self.grenades.push(Grenade { pos: p.pos + p.aim * 0.4, vel: p.aim * 7.2, z: 12.0, vz: 110.0, fuse: 0.95 });
        }
    }

    fn damage_player(&mut self, amount: f32, from: Vec2) {
        let p = &mut self.player;
        if p.iframes > 0.0 || self.phase == Phase::Dead || self.phase == Phase::Won {
            return;
        }
        p.hp -= amount;
        p.iframes = 0.55;
        self.sfx.push(Sfx::Hurt);
        let away = (p.pos - from).normalize_or_zero();
        p.vel += away * 5.0;
        self.hurt = 1.0;
        self.shake = (self.shake + 0.25).min(1.0);
        let pos = p.pos;
        blood_spray(&mut self.particles, pos, 10.0, away, 10, 1.0);
        if self.player.hp <= 0.0 {
            self.player.hp = 0.0;
            self.phase = Phase::Dead;
            self.phase_t = 0.0;
            self.slowmo = 0.35;
            self.say("OVERRUN", "", 99.0);
        }
    }

    // ------------------------------------------------------------ spawning

    fn spawn(&mut self, dt: f32) {
        let alive = self.zombies.len();
        let (rate, cap) = match self.phase {
            Phase::Fight => {
                if self.kills as usize + alive >= self.quota as usize {
                    return;
                }
                let prog = self.kills as f32 / self.quota as f32;
                (self.spawn_rate.0 + (self.spawn_rate.1 - self.spawn_rate.0) * prog, self.max_alive)
            }
            Phase::Boss => (3.0, 90),
            _ => return,
        };
        self.spawn_acc += rate * dt;
        while self.spawn_acc >= 1.0 {
            self.spawn_acc -= 1.0;
            if self.zombies.len() >= cap {
                break;
            }
            // pick a spawn point away from the player; tunnels are favorites
            // prefer points 8-20 tiles away so hordes arrive quickly even on big maps
            let mut at = None;
            let mut fallback: Option<(Vec2, f32)> = None;
            for _ in 0..14 {
                let from_tunnel = chance(0.3) && !self.map.tunnel_spawns.is_empty();
                let list = if from_tunnel { &self.map.tunnel_spawns } else { &self.map.edge_spawns };
                let p = list[rand::gen_range(0, list.len())];
                let d = p.distance(self.player.pos);
                if (8.0..20.0).contains(&d) {
                    at = Some(p);
                    break;
                }
                if d > 7.0 && fallback.map_or(true, |(_, fd)| d < fd) {
                    fallback = Some((p, d));
                }
            }
            let at = at.or(fallback.map(|f| f.0));
            let Some(p) = at else { continue };
            let group = if chance(0.35) { rand::gen_range(2, 5) } else { 1 };
            for g in 0..group {
                let jitter = if g == 0 { Vec2::ZERO } else { rand_dir() * rnd(0.3, 0.9) };
                let kind = self.pick_kind();
                self.add_zombie(kind, p + jitter);
            }
        }
    }

    fn pick_kind(&self) -> ZKind {
        let total: f32 = self.mix.iter().sum();
        let mut r = rnd(0.0, total);
        for (i, w) in self.mix.iter().enumerate() {
            if r < *w {
                return [ZKind::Walker, ZKind::Runner, ZKind::Brute][i];
            }
            r -= w;
        }
        ZKind::Walker
    }

    pub fn add_zombie(&mut self, kind: ZKind, pos: Vec2) {
        let s = self.station as f32;
        let (hp, r, speed, variants) = match kind {
            ZKind::Walker => (3.0 + s * 0.5, 0.27, rnd(1.2, 1.75) * (1.0 + s * 0.05), 4),
            ZKind::Runner => (2.0 + s * 0.3, 0.26, rnd(2.9, 3.4), 3),
            ZKind::Brute => (22.0 + s * 4.0, 0.42, rnd(0.95, 1.15), 1),
            ZKind::Boss => (900.0, 1.0, 1.9, 1),
        };
        let id = self.next_id;
        self.next_id += 1;
        self.zombies.push(Zombie {
            id,
            pos,
            vel: Vec2::ZERO,
            knock: Vec2::ZERO,
            hp,
            max_hp: hp,
            kind,
            variant: rand::gen_range(0, variants),
            r,
            speed,
            anim: rnd(0.0, 4.0),
            flash: 0.0,
            age: 0.0,
            attack_cd: 0.0,
            face_left: false,
            back: false,
            dead: false,
            slam_t: 0.0,
            slam_cd: 3.0,
        });
    }

    // ------------------------------------------------------------ zombies

    fn cell(&self, p: Vec2) -> Option<usize> {
        let (x, y) = (p.x.floor() as i32, p.y.floor() as i32);
        if x < 0 || y < 0 || x >= self.map.gw || y >= self.map.gh {
            None
        } else {
            Some((y * self.map.gw + x) as usize)
        }
    }

    fn rebuild_grid(&mut self) {
        for c in &mut self.grid {
            c.clear();
        }
        for i in 0..self.zombies.len() {
            if let Some(c) = self.cell(self.zombies[i].pos) {
                self.grid[c].push(i as u16);
            }
        }
    }

    fn neighbors(&self, p: Vec2, reach: i32, out: &mut Vec<u16>) {
        out.clear();
        let (cx, cy) = (p.x.floor() as i32, p.y.floor() as i32);
        for y in cy - reach..=cy + reach {
            for x in cx - reach..=cx + reach {
                if x < 0 || y < 0 || x >= self.map.gw || y >= self.map.gh {
                    continue;
                }
                out.extend_from_slice(&self.grid[(y * self.map.gw + x) as usize]);
            }
        }
    }

    fn update_zombies(&mut self, dt: f32) {
        self.rebuild_grid();
        let n = self.zombies.len();
        let ppos = self.player.pos;
        let player_alive = self.phase != Phase::Dead && !self.demo;
        let mut push = vec![Vec2::ZERO; n];
        let mut near = Vec::with_capacity(64);
        for i in 0..n {
            let zi = self.zombies[i];
            let reach = if zi.kind == ZKind::Boss { 2 } else { 1 };
            self.neighbors(zi.pos, reach, &mut near);
            for &j in &near {
                let j = j as usize;
                if j == i {
                    continue;
                }
                let zj = &self.zombies[j];
                let d = zi.pos - zj.pos;
                let rr = zi.r + zj.r;
                let l2 = d.length_squared();
                if l2 < rr * rr && l2 > 1e-6 {
                    let l = l2.sqrt();
                    let heavy = if zi.kind == ZKind::Boss { 0.1 } else if zj.kind == ZKind::Boss { 1.0 } else { 0.5 };
                    push[i] += d / l * (rr - l) * heavy;
                }
            }
        }

        let mut attacks: Vec<(f32, Vec2)> = Vec::new();
        let mut slams: Vec<Vec2> = Vec::new();
        for i in 0..n {
            let flow = self.map.flow_dir(self.zombies[i].pos);
            let z = &mut self.zombies[i];
            z.age += dt;
            z.flash -= dt;
            z.attack_cd -= dt;
            let to_p = ppos - z.pos;
            let dist = to_p.length();
            let mut desired = if !player_alive {
                // wander and crowd around the body
                if dist < 1.2 { Vec2::ZERO } else { to_p / dist * 0.5 }
            } else if dist < 1.8 || flow.is_none() {
                to_p / dist.max(0.001)
            } else {
                flow.unwrap()
            };
            if self.demo {
                desired = vec2((z.age * 0.7 + z.id as f32).sin(), (z.age * 0.5 + z.id as f32 * 1.7).cos()) * 0.4;
            }
            let mut speed = z.speed;
            if z.kind == ZKind::Boss {
                z.slam_cd -= dt;
                if z.slam_t > 0.0 {
                    z.slam_t -= dt;
                    speed = 0.0;
                    if z.slam_t <= 0.0 {
                        slams.push(z.pos);
                        z.slam_cd = 4.2;
                    }
                } else if z.slam_cd <= 0.0 && dist < 5.5 {
                    z.slam_t = 0.9;
                    self.sfx.push(Sfx::Roar);
                    self.rings.push(Ring { pos: z.pos, r: 3.4, t: 0.0, max: 0.9, color: Color::new(1.0, 0.2, 0.1, 1.0) });
                }
            }
            let target_v = desired * speed;
            z.vel += (target_v - z.vel) * (1.0 - (-6.0 * dt).exp());
            z.knock *= (-7.0 * dt).exp();
            z.pos += (z.vel + z.knock) * dt + push[i] * 0.6;
            let inner = false;
            self.map.resolve(&mut z.pos, z.r.min(0.45), inner);
            if let Some(t) = &self.train {
                push_out_of_train(t, &mut z.pos, z.r);
            }
            z.anim += dt * (z.vel.length() * 2.6 + 0.3);
            let sd = world_to_screen_dir(z.vel);
            if sd.x.abs() > 0.5 {
                z.face_left = sd.x < 0.0;
            }
            z.back = sd.y < -2.0;
            if player_alive && dist < z.r + PLAYER_R + 0.12 && z.attack_cd <= 0.0 && z.age > 0.4 && z.slam_t <= 0.0 {
                z.attack_cd = 0.8;
                let dmg = match z.kind {
                    ZKind::Walker => 10.0,
                    ZKind::Runner => 8.0,
                    ZKind::Brute => 22.0,
                    ZKind::Boss => 25.0,
                };
                attacks.push((dmg, z.pos));
            }
        }
        for (dmg, from) in attacks {
            self.damage_player(dmg, from);
        }
        for at in slams {
            self.boss_slam(at);
        }
    }

    fn boss_slam(&mut self, at: Vec2) {
        self.sfx.push(Sfx::Slam(at));
        self.shake = 1.0;
        self.shock = Some((at, 0.0));
        self.flash = 0.15;
        self.lights.push(TempLight { pos: at, radius: 6.0, color: Color::new(1.0, 0.3, 0.15, 1.0), life: 0.5, max: 0.5 });
        self.map.stamp(at, 1.8, 1);
        for _ in 0..40 {
            let d = rand_dir();
            let sp = rnd(2.0, 7.0);
            emit(&mut self.particles, particle(PK::Debris, at + d * 0.8, 4.0, vec3(d.x * sp, d.y * sp, rnd(80.0, 220.0)), rnd(0.8, 1.6), 2.0, Color::new(0.45, 0.43, 0.4, 1.0)));
            emit(&mut self.particles, particle(PK::Smoke, at + d * rnd(0.5, 2.5), 4.0, vec3(d.x * 1.5, d.y * 1.5, 10.0), rnd(0.8, 1.6), rnd(4.0, 8.0), Color::new(0.5, 0.48, 0.45, 0.6)));
        }
        if self.player.pos.distance(at) < 3.4 {
            self.damage_player(25.0, at);
            let away = (self.player.pos - at).normalize_or_zero();
            self.player.vel += away * 9.0;
        }
        for _ in 0..5 {
            let p = at + rand_dir() * rnd(1.5, 2.5);
            self.add_zombie(ZKind::Runner, p);
        }
    }

    fn hit_zombie(&mut self, i: usize, dmg: f32, dir: Vec2) {
        let z = &mut self.zombies[i];
        if z.dead {
            return;
        }
        z.hp -= dmg;
        if z.flash < -0.12 {
            z.flash = 0.07; // rate-limited so a boss under fire doesn't turn into a white blob
        }
        let kb = match z.kind {
            ZKind::Walker => 2.6,
            ZKind::Runner => 3.2,
            ZKind::Brute => 0.5,
            ZKind::Boss => 0.05,
        };
        z.knock += dir * kb;
        let (pos, kind) = (z.pos, z.kind);
        self.sfx.push(Sfx::Hit(pos));
        let h = if kind == ZKind::Boss { 30.0 } else { 10.0 };
        blood_spray(&mut self.particles, pos, h, dir, if kind == ZKind::Boss { 6 } else { 3 }, 0.8);
        if self.zombies[i].hp <= 0.0 {
            self.kill_zombie(i, dir);
        }
    }

    fn kill_zombie(&mut self, i: usize, dir: Vec2) {
        let z = &mut self.zombies[i];
        z.dead = true;
        let (pos, kind) = (z.pos, z.kind);
        self.sfx.push(Sfx::Splat(pos, kind));
        let big = match kind {
            ZKind::Boss => 3.0,
            ZKind::Brute => 1.6,
            _ => 1.0,
        };
        blood_spray(&mut self.particles, pos, 10.0 * big, dir, (14.0 * big) as usize, 1.0 + big * 0.2);
        gibs(&mut self.particles, pos, (4.0 * big) as usize, big.sqrt());
        self.map.stamp(pos + dir * 0.2, rnd(0.45, 0.8) * big, 0);
        if kind != ZKind::Boss {
            self.kills += 1;
            self.total_kills += 1;
        }
        let drop = match kind {
            ZKind::Brute => 0.35,
            ZKind::Boss => 0.0,
            _ => 0.02,
        };
        if chance(drop) {
            self.pickups.push(Pickup { pos, t: 0.0 });
        }
        if kind == ZKind::Brute {
            self.shake = (self.shake + 0.2).min(1.0);
        }
        if kind == ZKind::Boss {
            self.phase = Phase::Won;
            self.phase_t = 0.0;
            self.slowmo = 0.25;
            self.flash = 0.6;
            self.shake = 1.0;
            self.shock = Some((pos, 0.0));
            self.say("SHINJUKU CLEARED", "THE LAST TRAIN IS YOURS", 99.0);
            for z in &mut self.zombies {
                if z.kind != ZKind::Boss {
                    z.hp = 0.0;
                }
            }
            let ids: Vec<usize> = (0..self.zombies.len()).filter(|&k| !self.zombies[k].dead).collect();
            for k in ids {
                let d = (self.zombies[k].pos - pos).normalize_or_zero();
                self.kill_zombie(k, d);
            }
        }
    }

    // ------------------------------------------------------------ bullets

    fn update_bullets(&mut self, dt: f32) {
        let mut near = Vec::with_capacity(32);
        let mut i = 0;
        while i < self.bullets.len() {
            let mut b = self.bullets[i];
            b.life -= dt;
            let steps = ((b.vel.length() * dt) / 0.22).ceil().max(1.0) as i32;
            let step = b.vel * dt / steps as f32;
            let dir = b.vel.normalize_or_zero();
            let mut alive = b.life > 0.0;
            'steps: for _ in 0..steps {
                if !alive {
                    break;
                }
                b.pos += step;
                if self.map.solid_at(b.pos) {
                    self.sfx.push(Sfx::Tick(b.pos));
                    for _ in 0..6 {
                        let d = (-dir + rand_dir() * 0.9).normalize_or_zero();
                        emit(&mut self.particles, particle(PK::Spark, b.pos - dir * 0.05, 10.0, vec3(d.x * rnd(2.0, 6.0), d.y * rnd(2.0, 6.0), rnd(20.0, 120.0)), rnd(0.15, 0.4), 1.0, Color::new(1.0, 0.8, 0.4, 1.0)));
                    }
                    self.lights.push(TempLight { pos: b.pos, radius: 1.0, color: Color::new(1.0, 0.7, 0.4, 1.0), life: 0.05, max: 0.05 });
                    alive = false;
                    break;
                }
                self.neighbors(b.pos, 1, &mut near);
                for &zi in &near {
                    let zi = zi as usize;
                    let z = &self.zombies[zi];
                    if z.dead || z.age < 0.15 {
                        continue;
                    }
                    if b.hits[..b.nh].contains(&z.id) {
                        continue;
                    }
                    if z.pos.distance(b.pos) < z.r + 0.08 {
                        if b.nh < b.hits.len() {
                            b.hits[b.nh] = z.id;
                            b.nh += 1;
                        }
                        self.hit_zombie(zi, b.dmg, dir);
                        b.pierce -= 1;
                        if b.pierce < 0 {
                            alive = false;
                            break 'steps;
                        }
                    }
                }
            }
            if alive {
                self.bullets[i] = b;
                i += 1;
            } else {
                self.bullets.swap_remove(i);
            }
        }
    }

    // ------------------------------------------------------------ grenades

    fn update_grenades(&mut self, dt: f32) {
        let mut boom = Vec::new();
        for g in &mut self.grenades {
            g.fuse -= dt;
            g.vz -= 300.0 * dt;
            g.z += g.vz * dt;
            let next = g.pos + g.vel * dt;
            if self.map.solid_at(next) {
                g.vel = -g.vel * 0.4;
            } else {
                g.pos = next;
            }
            if g.z <= 0.0 {
                g.z = 0.0;
                g.vz = -g.vz * 0.35;
                g.vel *= 0.55;
            }
            if g.fuse <= 0.0 {
                boom.push(g.pos);
            }
        }
        self.grenades.retain(|g| g.fuse > 0.0);
        for at in boom {
            self.explode(at);
        }
    }

    fn explode(&mut self, at: Vec2) {
        let r = self.stats.gren_radius;
        self.sfx.push(Sfx::Explode(at));
        self.shake = (self.shake + 0.55).min(1.0);
        self.flash = 0.22;
        self.shock = Some((at, 0.0));
        self.lights.push(TempLight { pos: at, radius: r * 2.6, color: Color::new(1.0, 0.6, 0.25, 1.0), life: 0.55, max: 0.55 });
        self.map.stamp(at, r * 0.55, 1);
        for _ in 0..46 {
            let d = rand_dir();
            let sp = rnd(2.0, 9.0);
            emit(&mut self.particles, particle(PK::Spark, at, 6.0, vec3(d.x * sp, d.y * sp, rnd(40.0, 260.0)), rnd(0.25, 0.7), rnd(1.0, 2.0), Color::new(1.0, rnd(0.5, 0.85), 0.25, 1.0)));
        }
        for _ in 0..22 {
            let d = rand_dir();
            let sp = rnd(0.5, 2.5);
            emit(&mut self.particles, particle(PK::Ember, at + d * rnd(0.0, 0.8), rnd(4.0, 20.0), vec3(d.x * sp, d.y * sp, rnd(10.0, 60.0)), rnd(0.8, 1.8), 1.0, Color::new(1.0, 0.45, 0.1, 1.0)));
        }
        for _ in 0..18 {
            let d = rand_dir();
            emit(&mut self.particles, particle(PK::Smoke, at + d * rnd(0.0, 1.2), rnd(4.0, 14.0), vec3(d.x * 1.2, d.y * 1.2, rnd(8.0, 24.0)), rnd(1.0, 2.2), rnd(5.0, 10.0), Color::new(0.3, 0.28, 0.27, 0.7)));
        }
        for _ in 0..14 {
            let d = rand_dir();
            let sp = rnd(2.0, 6.0);
            emit(&mut self.particles, particle(PK::Debris, at, 4.0, vec3(d.x * sp, d.y * sp, rnd(100.0, 220.0)), rnd(1.0, 2.0), 2.0, Color::new(0.35, 0.33, 0.3, 1.0)));
        }
        let dmg = 14.0 * self.stats.dmg;
        let hits: Vec<(usize, f32, Vec2)> = self
            .zombies
            .iter()
            .enumerate()
            .filter(|(_, z)| !z.dead && z.pos.distance(at) < r + z.r)
            .map(|(i, z)| {
                let d = z.pos.distance(at);
                (i, 1.0 - (d / (r + z.r)) * 0.5, (z.pos - at).normalize_or_zero())
            })
            .collect();
        for (i, k, dir) in hits {
            self.zombies[i].knock += dir * 7.0 * k;
            self.hit_zombie(i, dmg * k, dir);
        }
    }

    // ------------------------------------------------------------ pickups

    fn update_pickups(&mut self, dt: f32) {
        let ppos = self.player.pos;
        let maxhp = self.stats.max_hp;
        let mut got = Vec::new();
        for (i, k) in self.pickups.iter_mut().enumerate() {
            k.t += dt;
            if k.pos.distance(ppos) < 0.65 && self.phase != Phase::Dead {
                got.push(i);
            }
        }
        for &i in got.iter().rev() {
            let at = self.pickups[i].pos;
            self.pickups.swap_remove(i);
            self.sfx.push(Sfx::Pickup);
            self.player.hp = (self.player.hp + 25.0).min(maxhp);
            for _ in 0..14 {
                let d = rand_dir();
                emit(&mut self.particles, particle(PK::Spark, at, 6.0, vec3(d.x * 1.5, d.y * 1.5, rnd(40.0, 120.0)), 0.5, 1.0, Color::new(0.5, 1.0, 0.6, 1.0)));
            }
            self.lights.push(TempLight { pos: at, radius: 2.0, color: Color::new(0.5, 1.0, 0.6, 1.0), life: 0.4, max: 0.4 });
        }
        self.pickups.retain(|k| k.t < 18.0);
    }

    // ------------------------------------------------------------ the train

    fn update_train(&mut self, dt: f32) {
        let Some(t) = &mut self.train else { return };
        if t.head < t.target {
            let sp = ((t.target - t.head) * 1.3).clamp(2.2, 22.0);
            t.head = (t.head + sp * dt).min(t.target);
        } else {
            t.stopped_t += dt;
            if !t.doors_open && t.stopped_t > 0.7 {
                t.doors_open = true;
                self.sfx.push(Sfx::Chime);
            }
        }
        // anything on the tracks in front of a moving train has a bad day
        let (tail, head, ty) = (t.tail(), t.head, t.y);
        let moving = t.head < t.target;
        if moving {
            let ids: Vec<usize> = (0..self.zombies.len())
                .filter(|&i| {
                    let z = &self.zombies[i];
                    !z.dead && z.kind != ZKind::Boss && (z.pos.y - ty).abs() < 0.9 && z.pos.x < head + 0.3 && z.pos.x > tail
                })
                .collect();
            for i in ids {
                let dir = vec2(1.0, rnd(-0.6, 0.6)).normalize();
                self.kill_zombie(i, dir);
            }
        }
        if let Some(t) = &self.train {
            if t.doors_open && self.phase == Phase::Train {
                let p = self.player.pos;
                for (i, s) in t.slices.iter().enumerate() {
                    if *s == SliceK::Door {
                        let cx = t.slice_x(i) + 0.5;
                        if (p.x - cx).abs() < 0.6 && (p.y - t.y).abs() < 1.5 {
                            self.events.push(Event::Boarded);
                            self.phase = Phase::Won; // freeze gameplay while main swaps scenes
                            return;
                        }
                    }
                }
            }
        }
    }

    fn update_phase(&mut self, _dt: f32) {
        match self.phase {
            Phase::Fight => {
                if self.kills >= self.quota && !self.demo {
                    if self.has_boss {
                        self.phase = Phase::Boss;
                        self.phase_t = 0.0;
                        let at = self.map.tunnel_spawns.iter().copied().min_by(|a, b| {
                            a.distance(self.player.pos).partial_cmp(&b.distance(self.player.pos)).unwrap()
                        });
                        let at = at.unwrap_or(vec2(2.0, 3.0)) + vec2(1.0, 0.0);
                        self.add_zombie(ZKind::Boss, at);
                        self.sfx.push(Sfx::Roar);
                        self.shake = 1.0;
                        self.say("THE RUSH HOUR", "IT CAME OUT OF THE TUNNEL", 3.5);
                    } else {
                        self.phase = Phase::Train;
                        self.phase_t = 0.0;
                        self.sfx.push(Sfx::Clear);
                        self.say("STATION CLEAR", "TRAIN ARRIVING - GET TO THE DOORS", 4.0);
                    }
                }
            }
            Phase::Train => {
                if self.train.is_none() && self.phase_t > 1.5 {
                    let y = self
                        .map
                        .track_ys
                        .iter()
                        .copied()
                        .min_by(|a, b| (a - self.player.pos.y).abs().partial_cmp(&(b - self.player.pos.y).abs()).unwrap())
                        .unwrap_or(5.0);
                    let mut slices = Vec::new();
                    for car in 0..4 {
                        let first = if car == 0 { SliceK::Cab } else { SliceK::Body };
                        slices.extend_from_slice(&[first, SliceK::Body, SliceK::Door, SliceK::Body, SliceK::Body, SliceK::Door, SliceK::Body]);
                        if car < 3 {
                            slices.push(SliceK::Gap);
                        }
                    }
                    let target = (self.map.w - 2) as f32;
                    self.train = Some(Train { head: 0.0, target, y, slices, stopped_t: 0.0, doors_open: false });
                    self.sfx.push(Sfx::Horn);
                    self.sfx.push(Sfx::Train);
                    self.shake = 0.3;
                }
                if let Some(t) = &self.train {
                    if t.doors_open && t.stopped_t < 0.75 {
                        self.say("BOARD THE TRAIN", "FIND A GLOWING DOOR", 99.0);
                    }
                }
            }
            Phase::Dead => {
                if self.phase_t > 2.2 && !self.events.contains(&Event::Died) {
                    self.events.push(Event::Died);
                }
            }
            Phase::Won => {
                if self.has_boss && self.phase_t > 3.0 && !self.events.contains(&Event::Won) && !self.events.contains(&Event::Boarded) {
                    self.events.push(Event::Won);
                }
            }
            Phase::Boss => {}
        }
    }

    // ------------------------------------------------------------ particles

    fn update_particles(&mut self, dt: f32) {
        let mut stamps: Vec<(Vec2, f32, u32)> = Vec::new();
        for p in &mut self.particles {
            p.life -= dt;
            if p.rest {
                continue;
            }
            let gz = self.map.ground_z(vec2(p.p.x, p.p.y));
            match p.kind {
                PK::Blood | PK::Gib | PK::Debris | PK::Casing | PK::Spark => {
                    let g = match p.kind {
                        PK::Blood => 520.0,
                        PK::Spark => 320.0,
                        _ => 480.0,
                    };
                    p.v.z -= g * dt;
                    p.p += p.v * dt;
                    if p.p.z <= gz {
                        p.p.z = gz;
                        match p.kind {
                            PK::Blood => {
                                if chance(0.55) {
                                    stamps.push((vec2(p.p.x, p.p.y), rnd(0.08, 0.2), 0));
                                }
                                p.life = 0.0;
                            }
                            PK::Spark => p.life = 0.0,
                            _ => {
                                if p.v.z.abs() < 40.0 {
                                    p.rest = true;
                                    if p.kind == PK::Gib {
                                        stamps.push((vec2(p.p.x, p.p.y), 0.14, 2));
                                    }
                                } else {
                                    p.v.z = -p.v.z * 0.35;
                                    p.v.x *= 0.5;
                                    p.v.y *= 0.5;
                                }
                            }
                        }
                    }
                }
                PK::Smoke => {
                    p.v *= (-2.0 * dt).exp();
                    p.p += p.v * dt;
                    p.size += dt * 4.0;
                }
                PK::Ember => {
                    p.v.z += 30.0 * dt;
                    p.v *= (-1.5 * dt).exp();
                    p.p += p.v * dt;
                }
                PK::Dust => {
                    p.p += p.v * dt;
                    p.p.x += (p.life * 1.3).sin() * 0.05 * dt;
                }
                PK::Flash => {}
            }
        }
        self.particles.retain(|p| p.life > 0.0);
        for (at, s, k) in stamps.into_iter().take(80) {
            self.map.stamp(at, s, k);
        }
    }
}

fn push_out_of_train(t: &Train, p: &mut Vec2, r: f32) {
    let half = 0.7 + r;
    if p.x > t.tail() && p.x < t.head + r && (p.y - t.y).abs() < half {
        p.y = t.y + if p.y < t.y { -half } else { half };
    }
}
