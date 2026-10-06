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
    /// How fast rounds travel (the pistol's are slow enough to see).
    pub bullet_speed: f32,
}

impl Default for Stats {
    fn default() -> Self {
        // the officer's pistol: steady, never runs dry, but slow rounds and it takes a few to drop one
        Stats { multishot: 1, dmg: 1.0, rate: 3.5, pierce: 0, gren_cd: 6.0, gren_radius: 2.6, max_hp: 100.0, speed: 4.6, dash_cd: 1.3, bullet_speed: 15.0 }
    }
}

impl Stats {
    /// Arcade mode keeps the original auto rifle.
    pub fn arcade() -> Self {
        Stats { multishot: 1, dmg: 1.0, rate: 10.0, pierce: 1, bullet_speed: 26.0, ..Stats::default() }
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
    /// Campaign: shambling around until it sees or hears you.
    pub idle: bool,
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
    pub kind: PickupKind,
    pub z: f32,
    pub vz: f32,
    pub vel: Vec2,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum PickupKind {
    Onigiri,
    Pistol,
    Coin(u32),
}

impl Pickup {
    pub fn new(pos: Vec2, kind: PickupKind) -> Self {
        Pickup { pos, t: 0.0, kind, z: 0.0, vz: 0.0, vel: Vec2::ZERO }
    }
}

/// People who aren't (yet) zombies: commuters, the officer, the dead.
#[derive(Clone, Copy, PartialEq, Debug)]
#[allow(dead_code)]
pub enum NpcState {
    Stand,
    Strap,
    Sit,
    Walk,
    Ride,
    Kneel,
    Corpse,
    Shoot,
    Gone,
}

#[derive(Clone, Copy)]
pub struct Npc {
    pub pos: Vec2,
    pub vel: Vec2,
    pub home: Vec2,
    pub look: usize,
    pub state: NpcState,
    pub anim: f32,
    pub face_left: bool,
    pub back: bool,
    pub z: f32,
    pub t: f32,
    pub target: Vec2,
    pub speed: f32,
    pub officer: bool,
    pub aim: Vec2,
    pub after: NpcState, // what to become on reaching the target
    pub loot: u32,
    pub searched: bool,
    pub flashlight: bool,
}

impl Npc {
    pub fn new(pos: Vec2, look: usize, state: NpcState) -> Self {
        Npc { pos, vel: Vec2::ZERO, home: pos, look, state, anim: rnd(0.0, 4.0), face_left: chance(0.5), back: false, z: 0.0, t: rnd(0.0, 3.0), target: pos, speed: rnd(1.5, 2.1), officer: false, aim: Vec2::X, after: NpcState::Gone, loot: 0, searched: false, flashlight: false }
    }
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

/// How much tougher everything is at stop `s` of the loop (1.0 at the start,
/// about 3.4x by Ikebukuro, 5x by Shinjuku, 18x by the end) to keep pace with a
/// growing stack of upgrade cards.
pub fn loop_hp(s: f32) -> f32 {
    1.0 + 0.08 * s + 0.016 * s * s
}

/// Each boss station has its own monster.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum BossKind {
    RushHour,      // Shinjuku (and Arcade): ground slams that throw off runners
    Stampede,      // Ueno: telegraphed charges that smash through anything, and a herd behind it
    Bloated,       // Ikebukuro: lobs acid that pools on the floor, bursts when it dies
    Scramble,      // Shibuya: quick lunges, and a ring of runners converging from every side
    Conductor,     // Shinagawa: fans of crackling sparks, a whistle that whips the dead into a frenzy
    Stationmaster, // Tokyo: slams and expanding shockwaves you have to dash through, brings brutes
    PatientZero,   // Akihabara, the end: all of it, in three phases
}

impl BossKind {
    pub fn name(&self) -> &'static str {
        match self {
            BossKind::RushHour => "THE RUSH HOUR",
            BossKind::Stampede => "THE STAMPEDE",
            BossKind::Bloated => "THE BLOATED",
            BossKind::Scramble => "THE SCRAMBLE",
            BossKind::Conductor => "THE CONDUCTOR",
            BossKind::Stationmaster => "THE STATIONMASTER",
            BossKind::PatientZero => "PATIENT ZERO",
        }
    }
    pub fn entrance(&self) -> &'static str {
        match self {
            BossKind::RushHour => "IT CAME OUT OF THE TUNNEL",
            BossKind::Stampede => "SOMETHING IS CHARGING DOWN THE TRACKS",
            BossKind::Bloated => "IT REEKS. DON'T STAND IN ANYTHING.",
            BossKind::Scramble => "THEY'RE COMING FROM EVERY DIRECTION",
            BossKind::Conductor => "THE RAILS ARE CRACKLING",
            BossKind::Stationmaster => "DASH THROUGH THE SHOCKWAVES",
            BossKind::PatientZero => "WHERE IT ALL STARTED",
        }
    }
    pub fn tint(&self) -> Color {
        match self {
            BossKind::RushHour => WHITE,
            BossKind::Stampede => Color::new(1.0, 0.8, 0.65, 1.0),
            BossKind::Bloated => Color::new(0.7, 1.0, 0.55, 1.0),
            BossKind::Scramble => Color::new(1.0, 0.65, 0.9, 1.0),
            BossKind::Conductor => Color::new(0.65, 0.85, 1.0, 1.0),
            BossKind::Stationmaster => Color::new(0.85, 0.75, 1.0, 1.0),
            BossKind::PatientZero => Color::new(1.0, 0.5, 0.45, 1.0),
        }
    }
    /// Base health, multiplied by how far round the loop it is.
    pub fn hp(&self) -> f32 {
        match self {
            BossKind::RushHour => 160.0,
            BossKind::Stampede => 90.0,
            BossKind::Bloated => 130.0,
            BossKind::Scramble => 170.0,
            BossKind::Conductor => 320.0,
            BossKind::Stationmaster => 380.0,
            BossKind::PatientZero => 520.0,
        }
    }
    pub fn speed(&self) -> f32 {
        match self {
            BossKind::Bloated => 1.15,
            BossKind::Scramble => 2.6,
            BossKind::Stampede => 1.7,
            _ => 1.9,
        }
    }
    fn slams(&self) -> bool {
        matches!(self, BossKind::RushHour | BossKind::Stationmaster | BossKind::PatientZero)
    }
}

/// Things bosses leave in the world: acid pools, sparks, shockwaves, acid in flight.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum HazardKind {
    Acid,
    Spark,
    Wave,
    Blob,
}

#[derive(Clone, Copy)]
pub struct Hazard {
    pub kind: HazardKind,
    pub pos: Vec2,
    pub vel: Vec2,
    pub z: f32,
    pub vz: f32,
    pub r: f32,
    pub t: f32,
    pub life: f32,
    pub hit: bool,
}

/// Tokyo's crows. They peck around the platforms and scatter when you get close or start shooting.
#[derive(Clone, Copy)]
pub struct Bird {
    pub pos: Vec2,
    pub z: f32,
    pub vel: Vec3,
    pub flying: bool,
    pub leaving: bool,
    pub target: Vec2,
    pub t: f32,
    pub anim: f32,
    pub face_left: bool,
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
    /// Passing through (express, or leaving): removed once it's gone.
    pub pass: bool,
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
    /// The opening: no spawning, the intro script drives everything.
    Intro,
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
    Caw(Vec2),
    Zap(Vec2),
    Quake,
    Scream(Vec2),
    Retch(Vec2),
    Attract(Vec2),
    Coin,
    Smash(Vec2, bool),
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Event {
    GotPistol,
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
    pub birds: Vec<Bird>,
    pub boss_kind: BossKind,
    pub hazards: Vec<Hazard>,
    boss_cd: [f32; 4],
    /// A charge: direction, windup left, dash left.
    pub boss_charge: Option<(Vec2, f32, f32)>,
    boss_phase: u8,
    pub frenzy: f32,
    pub npcs: Vec<Npc>,
    pub npc_field: Vec<u16>,
    pub has_gun: bool,
    pub locked: bool,
    pub freeze: bool,
    pub light_scale: f32,
    pub emergency: f32,
    pub player_z: f32,
    pub player_lying: bool,
    pub car_speed: f32,
    pub car_doors_open: bool,
    /// Scripted moments slow your walk (1.0 = normal); dashing is off while it's below 1.
    pub walk_scale: f32,
    /// Permanent upgrades: more coins per smash, and how far coins pull in from.
    pub coin_mult: f32,
    pub magnet: f32,
    /// Where we are on the loop (index, total) and the next stop, for the HUD.
    pub loop_pos: Option<(usize, usize)>,
    pub next_name: String,
    /// The last boss: killing it ends the game instead of calling the train.
    pub final_boss: bool,
    pub campaign: bool,
    /// Any campaign world (shows coins, allows searching bodies).
    pub story: bool,
    pub has_light: bool,
    pub light_on: bool,
    pub coins: u32,
    pub floaters: Vec<(Vec2, String, f32, Color)>,
    pub cam_focus: Option<Vec2>,
    kill_spots: Vec<Vec2>,
    bird_t: f32,
    loud: (Vec2, f32), // where and when something loud happened
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
        let next = match defs.get(station + 1) {
            Some(n) => format!("FOR {}", n.name),
            None => "OUT OF SERVICE".to_string(),
        };
        Self::from_def(&defs[station], &next, station, art, stats, total_kills, demo)
    }

    pub fn from_def(def: &StationDef, next: &str, station: usize, art: &crate::art::Art, stats: Stats, total_kills: u32, demo: bool) -> World {
        let map = Map::build(def, station, next, art);
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
            birds: Vec::new(),
            boss_kind: BossKind::RushHour,
            hazards: Vec::new(),
            boss_cd: [3.0, 5.0, 8.0, 6.0],
            boss_charge: None,
            boss_phase: 0,
            frenzy: 0.0,
            npcs: Vec::new(),
            npc_field: Vec::new(),
            has_gun: true,
            locked: false,
            freeze: false,
            light_scale: 1.0,
            emergency: 0.0,
            player_z: 0.0,
            player_lying: false,
            car_speed: 0.0,
            car_doors_open: false,
            walk_scale: 1.0,
            coin_mult: 1.0,
            magnet: 2.6,
            loop_pos: None,
            next_name: String::new(),
            final_boss: false,
            campaign: false,
            story: false,
            has_light: true,
            light_on: true,
            coins: 0,
            floaters: Vec::new(),
            cam_focus: None,
            kill_spots: Vec::new(),
            bird_t: 4.0,
            loud: (Vec2::ZERO, -99.0),
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
        // a few crows already picking at the platforms
        let n = 4 + station * 2;
        let mut tries = 0;
        while w.birds.len() < n && tries < 200 {
            tries += 1;
            let p = vec2(rnd(2.5, w.map.w as f32 - 1.5), rnd(1.8, w.map.h as f32 - 1.0));
            if w.map.tile(p.x, p.y) == Tile::Track || w.map.solid_at(p) || p.distance(w.player.pos) < 5.0 {
                continue;
            }
            let mut q = p;
            w.map.resolve(&mut q, 0.2, true);
            w.birds.push(Bird { pos: q, z: 0.0, vel: Vec3::ZERO, flying: false, leaving: false, target: q, t: rnd(0.0, 5.0), anim: 0.0, face_left: chance(0.5) });
        }
        w
    }

    pub fn boss(&self) -> Option<&Zombie> {
        self.zombies.iter().find(|z| z.kind == ZKind::Boss && !z.dead)
    }

    pub fn say(&mut self, title: &str, sub: &str, dur: f32) {
        self.msg = Some(Msg { title: title.to_string(), sub: sub.to_string(), t: 0.0, dur });
    }

    // ------------------------------------------------------------ main update

    pub fn update(&mut self, raw_dt: f32, c: &Controls) {
        if self.freeze {
            return;
        }
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
        self.update_boss(dt);
        self.update_hazards(dt);
        self.update_birds(dt);
        self.update_npcs(dt);
        self.update_atmosphere(dt);
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
        let target = match self.cam_focus {
            Some(f) => iso(f.x, f.y) + vec2(0.0, -8.0),
            None => iso(self.player.pos.x, self.player.pos.y) + self.player.aim_screen * 18.0 + vec2(0.0, -8.0),
        };
        for f in &mut self.floaters {
            f.2 += raw_dt;
        }
        self.floaters.retain(|f| f.2 < 1.6);
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
        if c.auto_aim && aim_screen.is_none() && self.has_gun {
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

        let mv_world = if c.mv.length() > 0.05 && !self.locked { screen_to_world_dir(c.mv).normalize_or_zero() * c.mv.length().min(1.0) } else { Vec2::ZERO };
        if self.locked {
            p.dash_t = 0.0;
        }
        if c.dash && p.dash_cd <= 0.0 && !self.locked && self.walk_scale >= 0.99 {
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
            let target = mv_world * st.speed * self.walk_scale;
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
        if fire && p.fire_cd <= 0.0 && self.has_gun && !self.locked {
            p.fire_cd = 1.0 / st.rate;
            p.muzzle = 0.05;
            self.sfx.push(Sfx::Shot);
            self.loud = (p.pos, self.time);
            let base = p.aim.y.atan2(p.aim.x);
            let n = st.multishot;
            let muzzle = p.pos + p.aim * 0.5;
            for k in 0..n {
                let a = base + (k as f32 - (n - 1) as f32 * 0.5) * 0.11 + rnd(-0.035, 0.035);
                let d = vec2(a.cos(), a.sin());
                self.bullets.push(Bullet { pos: muzzle, vel: d * st.bullet_speed, life: 22.0 / st.bullet_speed, dmg: st.dmg, pierce: st.pierce, hits: [0; 8], nh: 0 });
            }
            self.lights.push(TempLight { pos: muzzle, radius: 2.8, color: Color::new(1.0, 0.8, 0.45, 1.0), life: 0.06, max: 0.06 });
            emit(&mut self.particles, particle(PK::Flash, muzzle, 10.0, Vec3::ZERO, 0.05, 6.0, Color::new(1.0, 0.85, 0.5, 1.0)));
            let side = vec2(-p.aim.y, p.aim.x);
            emit(&mut self.particles, particle(PK::Casing, p.pos + p.aim * 0.2, 9.0, vec3(side.x * rnd(1.0, 2.0), side.y * rnd(1.0, 2.0), rnd(60.0, 100.0)), 2.5, 1.0, WHITE));
            self.shake = (self.shake + 0.02).min(1.0);
        }

        if c.grenade && p.gren_cd <= 0.0 && self.has_gun && !self.locked {
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
        if self.campaign && self.phase == Phase::Fight {
            // campaign levels: a slow trickle of wanderers from the dark edges
            self.spawn_acc += self.spawn_rate.0 * dt;
            if self.spawn_acc >= 1.0 && alive < self.max_alive {
                self.spawn_acc = 0.0;
                let pts: Vec<Vec2> = self.map.edge_spawns.iter().chain(self.map.tunnel_spawns.iter()).copied().filter(|p| p.distance(self.player.pos) > 14.0).collect();
                if !pts.is_empty() {
                    let p = pts[rand::gen_range(0, pts.len())];
                    let kind = self.pick_kind();
                    self.add_zombie(kind, p);
                    let hunter = chance((self.station as f32 * 0.02).min(0.5));
                    if let Some(z) = self.zombies.last_mut() {
                        z.idle = !hunter;
                    }
                }
            }
            return;
        }
        let (rate, cap) = match self.phase {
            Phase::Fight => {
                // a few spare zombies keep coming near the end, so one stuck somewhere
                // unreachable can never stall the station
                if self.kills as usize + alive >= self.quota as usize + 12 {
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
            // the loop: toughness climbs to keep up with a stack of upgrade cards
            ZKind::Walker if self.story => (3.0 * loop_hp(s), 0.27, rnd(1.2, 1.75) * (1.0 + (s * 0.012).min(0.35)), 4),
            ZKind::Runner if self.story => (2.0 * loop_hp(s), 0.26, rnd(2.9, 3.4), 3),
            ZKind::Brute if self.story => (22.0 * loop_hp(s), 0.42, rnd(0.95, 1.15), 1),
            ZKind::Boss if self.story => (self.boss_kind.hp() * loop_hp(s), 1.0, self.boss_kind.speed(), 1),
            // arcade: the original five-station numbers
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
            idle: false,
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
            if z.idle {
                let heard = self.time - self.loud.1 < 0.5 && z.pos.distance(self.loud.0) < 13.0;
                if (player_alive && dist < 8.5) || heard || z.hp < z.max_hp {
                    z.idle = false;
                }
            }
            let mut desired = if z.idle {
                vec2((z.age * 0.3 + z.id as f32).sin(), (z.age * 0.23 + z.id as f32 * 1.7).cos()) * 0.3
            } else if !player_alive {
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
            let mut speed = z.speed * if self.frenzy > 0.0 && z.kind != ZKind::Boss { 1.5 } else { 1.0 };
            if z.kind == ZKind::Boss && self.boss_charge.is_some() {
                speed = 0.0;
            }
            if z.kind == ZKind::Boss && self.boss_kind.slams() {
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
            self.map.resolve_ex(&mut z.pos, z.r.min(0.45), false, z.kind == ZKind::Boss);
            if let Some(t) = &self.train {
                // a moving train runs them down (see update_train); a stopped one is a wall
                if t.head >= t.target {
                    push_out_of_train(t, &mut z.pos, z.r);
                }
            }
            z.anim += dt * (z.vel.length() * 2.6 + 0.3);
            let sd = world_to_screen_dir(z.vel);
            if sd.x.abs() > 0.5 {
                z.face_left = sd.x < 0.0;
            }
            z.back = sd.y < -2.0;
            if player_alive && !z.idle && dist < z.r + PLAYER_R + 0.12 && z.attack_cd <= 0.0 && z.age > 0.4 && z.slam_t <= 0.0 {
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

    pub fn hit_zombie(&mut self, i: usize, dmg: f32, dir: Vec2) {
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
        if self.kill_spots.len() >= 16 {
            self.kill_spots.remove(0);
        }
        self.kill_spots.push(pos);
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
            self.pickups.push(Pickup::new(pos, PickupKind::Onigiri));
        }
        if kind == ZKind::Brute {
            self.shake = (self.shake + 0.2).min(1.0);
        }
        if kind == ZKind::Boss {
            self.hazards.clear();
            self.boss_charge = None;
            self.frenzy = 0.0;
            if self.boss_kind == BossKind::Bloated {
                for k in 0..8 {
                    let a = k as f32 * std::f32::consts::TAU / 8.0;
                    let p = pos + vec2(a.cos(), a.sin()) * 2.2;
                    self.hazards.push(Hazard { kind: HazardKind::Acid, pos: p, vel: Vec2::ZERO, z: 0.0, vz: 0.0, r: 1.1, t: 0.0, life: 4.0, hit: false });
                }
            }
        }
        if kind == ZKind::Boss && self.story && !self.final_boss {
            // mid-loop bosses: the station is yours, and the train comes
            self.phase = Phase::Train;
            self.phase_t = 0.0;
            self.slowmo = 0.3;
            self.flash = 0.5;
            self.shake = 1.0;
            self.shock = Some((pos, 0.0));
            self.say("IT'S DOWN", "THE TRAIN IS COMING. GET TO THE DOORS.", 5.0);
            for z in &mut self.zombies {
                z.hp = 0.0;
            }
            let ids: Vec<usize> = (0..self.zombies.len()).filter(|&k| !self.zombies[k].dead).collect();
            for k in ids {
                let d = (self.zombies[k].pos - pos).normalize_or_zero();
                self.kill_zombie(k, d);
            }
        } else if kind == ZKind::Boss {
            self.phase = Phase::Won;
            self.phase_t = 0.0;
            self.slowmo = 0.25;
            self.flash = 0.6;
            self.shake = 1.0;
            self.shock = Some((pos, 0.0));
            let (t1, t2) = if self.story { ("THE LOOP IS CLEAR", "EVERY STATION. ALL THE WAY AROUND.") } else { ("SHINJUKU CLEARED", "THE LAST TRAIN IS YOURS") };
            self.say(t1, t2, 99.0);
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
                if let Some(pi) = self.map.breakable_at(b.pos) {
                    self.hit_prop(pi, b.dmg, dir);
                    alive = false;
                    break;
                }
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
        self.loud = (at, self.time);
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
        let near: Vec<usize> = (0..self.map.props.len())
            .filter(|&i| {
                let pr = &self.map.props[i];
                pr.hp > 0.0 && !pr.broken && (pr.pos + pr.size * 0.5).distance(at) < r + 0.5
            })
            .collect();
        for pi in near {
            let c = self.map.props[pi].pos + self.map.props[pi].size * 0.5;
            self.hit_prop(pi, dmg, (c - at).normalize_or_zero());
        }
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
        let magnet = self.magnet;
        let mut got = Vec::new();
        for (i, k) in self.pickups.iter_mut().enumerate() {
            k.t += dt;
            if k.z > 0.0 || k.vz != 0.0 {
                k.vz -= 420.0 * dt;
                k.z += k.vz * dt;
                k.pos += k.vel * dt;
                if k.z <= 0.0 {
                    k.z = 0.0;
                    k.vz = if k.vz.abs() > 60.0 { -k.vz * 0.35 } else { 0.0 };
                    k.vel *= 0.4;
                }
            }
            if let PickupKind::Coin(_) = k.kind {
                // coins get pulled in when you're close
                let d = ppos - k.pos;
                if d.length() < magnet && k.z <= 0.5 && k.t > 0.35 {
                    k.pos += d.normalize_or_zero() * (7.0 * dt).min(d.length());
                }
            }
            let reach = if k.kind == PickupKind::Pistol { 0.8 } else { 0.65 };
            if k.pos.distance(ppos) < reach && k.z < 6.0 && self.phase != Phase::Dead {
                got.push(i);
            }
        }
        for &i in got.iter().rev() {
            let k = self.pickups.swap_remove(i);
            let at = k.pos;
            if let PickupKind::Coin(v) = k.kind {
                self.coins += v;
                self.sfx.push(Sfx::Coin);
                continue;
            }
            self.sfx.push(Sfx::Pickup);
            match k.kind {
                PickupKind::Onigiri => self.player.hp = (self.player.hp + 25.0).min(maxhp),
                PickupKind::Pistol => {
                    self.has_gun = true;
                    self.events.push(Event::GotPistol);
                }
                PickupKind::Coin(_) => {}
            }
            for _ in 0..14 {
                let d = rand_dir();
                emit(&mut self.particles, particle(PK::Spark, at, 6.0, vec3(d.x * 1.5, d.y * 1.5, rnd(40.0, 120.0)), 0.5, 1.0, Color::new(0.5, 1.0, 0.6, 1.0)));
            }
            self.lights.push(TempLight { pos: at, radius: 2.0, color: Color::new(0.5, 1.0, 0.6, 1.0), life: 0.4, max: 0.4 });
        }
        self.pickups.retain(|k| k.t < 18.0 || k.kind != PickupKind::Onigiri);
    }

    /// Campaign: zombies already shambling around the level when you arrive.
    #[allow(dead_code)]
    pub fn populate_idle(&mut self, n: usize) {
        let mut tries = 0;
        let mut placed = 0;
        while placed < n && tries < n * 30 {
            tries += 1;
            let p = vec2(rnd(2.0, self.map.w as f32 - 1.0), rnd(2.0, self.map.h as f32 - 1.0));
            if self.map.solid_at(p) || p.distance(self.player.pos) < 10.0 || self.map.blocked_at(p) {
                continue;
            }
            let kind = self.pick_kind();
            self.add_zombie(kind, p);
            if let Some(z) = self.zombies.last_mut() {
                z.idle = true;
                z.age = 1.0;
            }
            placed += 1;
        }
    }

    /// Campaign: the dead, some with coins in their pockets.
    #[allow(dead_code)]
    pub fn scatter_bodies(&mut self, n: usize) {
        let mut tries = 0;
        let mut placed = 0;
        while placed < n && tries < n * 30 {
            tries += 1;
            let p = vec2(rnd(2.0, self.map.w as f32 - 1.0), rnd(2.0, self.map.h as f32 - 1.0));
            if self.map.solid_at(p) || self.map.blocked_at(p) || self.map.tile(p.x, p.y) == Tile::Track || p.distance(self.player.pos) < 3.0 {
                continue;
            }
            let mut b = Npc::new(p, rand::gen_range(0, 8), NpcState::Corpse);
            b.face_left = chance(0.5);
            b.loot = if chance(0.3) { 0 } else { rand::gen_range(5, 41) };
            self.npcs.push(b);
            self.map.stamp(p, rnd(0.4, 0.8), 2);
            placed += 1;
        }
    }

    /// The closest body you can still search, if you're standing next to one.
    pub fn searchable(&self) -> Option<usize> {
        self.npcs
            .iter()
            .enumerate()
            .filter(|(_, n)| n.state == NpcState::Corpse && !n.searched && (n.loot > 0 || n.flashlight) && n.pos.distance(self.player.pos) < 1.2)
            .min_by(|a, b| a.1.pos.distance(self.player.pos).partial_cmp(&b.1.pos.distance(self.player.pos)).unwrap())
            .map(|(i, _)| i)
    }

    /// Search a body: coins, and maybe something better. Returns true if it held the flashlight.
    pub fn search(&mut self, i: usize) -> bool {
        let n = &mut self.npcs[i];
        n.searched = true;
        let (pos, loot, light) = (n.pos, n.loot, n.flashlight);
        if light {
            self.has_light = true;
            self.light_on = true;
            self.floaters.push((pos, "FLASHLIGHT".to_string(), 0.0, Color::new(1.0, 0.95, 0.7, 1.0)));
        }
        if loot > 0 {
            self.coins += loot;
            self.floaters.push((pos + vec2(0.3, 0.3), format!("+{} COINS", loot), if light { -0.5 } else { 0.0 }, Color::new(1.0, 0.85, 0.3, 1.0)));
            self.sfx.push(Sfx::Coin);
        } else if !light {
            self.floaters.push((pos, "NOTHING".to_string(), 0.0, Color::new(0.6, 0.6, 0.65, 1.0)));
        }
        light
    }

    // ------------------------------------------------------------ bosses

    fn spawn_hazard(&mut self, kind: HazardKind, pos: Vec2, vel: Vec2, r: f32, life: f32) {
        self.hazards.push(Hazard { kind, pos, vel, z: 0.0, vz: 0.0, r, t: 0.0, life, hit: false });
    }

    fn lob_acid(&mut self, from: Vec2, to: Vec2) {
        let flight = 0.9;
        self.hazards.push(Hazard { kind: HazardKind::Blob, pos: from, vel: (to - from) / flight, z: 28.0, vz: 150.0, r: 0.3, t: 0.0, life: 3.0, hit: false });
    }

    fn update_boss(&mut self, dt: f32) {
        self.frenzy = (self.frenzy - dt).max(0.0);
        let Some(bi) = self.zombies.iter().position(|z| z.kind == ZKind::Boss && !z.dead) else {
            self.boss_charge = None;
            return;
        };
        let ppos = self.player.pos;
        let (bpos, hp, max_hp) = (self.zombies[bi].pos, self.zombies[bi].hp, self.zombies[bi].max_hp);
        let to_p = ppos - bpos;
        for c in &mut self.boss_cd {
            *c -= dt;
        }
        let mut kind = self.boss_kind;
        // Patient Zero changes as it's hurt: each phase adds what the others did
        if kind == BossKind::PatientZero {
            let phase = if hp > max_hp * 0.66 { 0 } else if hp > max_hp * 0.33 { 1 } else { 2 };
            if phase != self.boss_phase {
                self.boss_phase = phase;
                self.sfx.push(Sfx::Roar);
                self.shake = 1.0;
                self.flash = 0.3;
                self.say("PATIENT ZERO", if phase == 1 { "IT'S CHANGING" } else { "IT'S ANGRY NOW" }, 2.5);
                self.zombies[bi].speed = BossKind::PatientZero.speed() * (1.0 + 0.15 * phase as f32);
            }
            kind = match (phase, (self.time as i32 / 6) % 2) {
                (0, _) => BossKind::Stampede,
                (1, 0) => BossKind::Conductor,
                (1, _) => BossKind::Bloated,
                (_, 0) => BossKind::Stationmaster,
                _ => BossKind::Scramble,
            };
        }

        // charges: wind up (you see the line), then a straight-line rush that smashes everything
        if let Some((dir, wind, run)) = self.boss_charge {
            if wind > 0.0 {
                self.boss_charge = Some((dir, wind - dt, run));
            } else if run > 0.0 {
                let step = dir * 13.0 * dt;
                self.zombies[bi].pos += step;
                self.zombies[bi].vel = dir * 13.0;
                let at = self.zombies[bi].pos;
                if let Some(pi) = self.map.breakable_at(at) {
                    self.hit_prop(pi, 999.0, dir);
                }
                if at.distance(ppos) < 1.3 && self.player.iframes <= 0.0 {
                    self.damage_player(if self.boss_kind == BossKind::Stampede { 18.0 } else { 24.0 }, at);
                    self.player.vel += dir * 10.0;
                }
                if self.map.solid_at(at + dir * 0.9) {
                    self.boss_charge = None;
                    self.shake = 0.8;
                    self.sfx.push(Sfx::Slam(at));
                } else {
                    self.boss_charge = Some((dir, 0.0, run - dt));
                }
            } else {
                self.boss_charge = None;
            }
            return;
        }

        match kind {
            BossKind::RushHour => {}
            BossKind::Stampede => {
                if self.boss_cd[0] <= 0.0 && to_p.length() < 11.0 {
                    self.boss_cd[0] = 4.6;
                    self.boss_charge = Some((to_p.normalize_or_zero(), 0.9, 0.85));
                    self.sfx.push(Sfx::Roar);
                }
                if self.boss_cd[1] <= 0.0 {
                    self.boss_cd[1] = 11.0;
                    for k in 0..3 {
                        let p = bpos + rand_dir() * (1.5 + k as f32 * 0.3);
                        self.add_zombie(ZKind::Walker, p);
                    }
                }
            }
            BossKind::Bloated => {
                if self.boss_cd[0] <= 0.0 {
                    self.boss_cd[0] = 3.4;
                    for k in 0..3 {
                        let spread = vec2(-to_p.y, to_p.x).normalize_or_zero() * (k as f32 - 1.0) * 1.8;
                        self.lob_acid(bpos, ppos + spread + self.player.vel * 0.4);
                    }
                    self.sfx.push(Sfx::Retch(bpos));
                }
            }
            BossKind::Scramble => {
                if self.boss_cd[0] <= 0.0 && to_p.length() < 8.0 {
                    self.boss_cd[0] = 3.0;
                    self.boss_charge = Some((to_p.normalize_or_zero(), 0.4, 0.45));
                }
                if self.boss_cd[1] <= 0.0 {
                    // the scramble: they come from every side at once
                    self.boss_cd[1] = 9.0;
                    self.sfx.push(Sfx::Horn);
                    self.say("", "SCRAMBLE", 1.2);
                    for k in 0..10 {
                        let a = k as f32 * std::f32::consts::TAU / 10.0;
                        let p = ppos + vec2(a.cos(), a.sin()) * 7.5;
                        if !self.map.solid_at(p) && self.map.tile(p.x, p.y) != Tile::Wall {
                            self.add_zombie(ZKind::Runner, p);
                        }
                    }
                }
            }
            BossKind::Conductor => {
                if self.boss_cd[0] <= 0.0 {
                    self.boss_cd[0] = 2.6;
                    let base = to_p.normalize_or_zero();
                    for k in 0..5 {
                        let a = (k as f32 - 2.0) * 0.28;
                        let d = vec2(base.x * a.cos() - base.y * a.sin(), base.x * a.sin() + base.y * a.cos());
                        self.spawn_hazard(HazardKind::Spark, bpos + d * 0.8, d * 5.5, 0.35, 2.8);
                    }
                    self.sfx.push(Sfx::Zap(bpos));
                }
                if self.boss_cd[1] <= 0.0 {
                    // the whistle: every one of them speeds up
                    self.boss_cd[1] = 10.0;
                    self.frenzy = 4.0;
                    self.sfx.push(Sfx::Horn);
                    self.say("", "THE WHISTLE. THEY'RE FASTER.", 1.6);
                }
            }
            BossKind::Stationmaster => {
                if self.boss_cd[0] <= 0.0 {
                    self.boss_cd[0] = 5.0;
                    self.spawn_hazard(HazardKind::Wave, bpos, Vec2::ZERO, 0.6, 1.25);
                    self.sfx.push(Sfx::Slam(bpos));
                    self.shake = 0.6;
                }
                if self.boss_cd[2] <= 0.0 {
                    self.boss_cd[2] = 13.0;
                    for _ in 0..2 {
                        let p = bpos + rand_dir() * 2.5;
                        self.add_zombie(ZKind::Brute, p);
                    }
                }
            }
            BossKind::PatientZero => {}
        }
    }

    fn update_hazards(&mut self, dt: f32) {
        let ppos = self.player.pos;
        let mut hurt: Vec<(f32, Vec2)> = Vec::new();
        let mut pools: Vec<Vec2> = Vec::new();
        for h in &mut self.hazards {
            h.t += dt;
            match h.kind {
                HazardKind::Acid => {
                    if h.pos.distance(ppos) < h.r && (h.t * 4.0).fract() < dt * 4.0 {
                        hurt.push((5.0, h.pos)); // about 20 a second while you stand in it
                    }
                }
                HazardKind::Spark => {
                    h.pos += h.vel * dt;
                    if !h.hit && h.pos.distance(ppos) < h.r + 0.25 {
                        h.hit = true;
                        h.t = h.life;
                        hurt.push((12.0, h.pos));
                    }
                }
                HazardKind::Wave => {
                    h.r += 7.0 * dt;
                    let d = h.pos.distance(ppos);
                    if !h.hit && (d - h.r).abs() < 0.45 {
                        h.hit = true;
                        hurt.push((16.0, h.pos));
                    }
                }
                HazardKind::Blob => {
                    h.pos += h.vel * dt;
                    h.vz -= 420.0 * dt;
                    h.z += h.vz * dt;
                    if h.z <= 0.0 {
                        h.t = h.life;
                        pools.push(h.pos);
                    }
                }
            }
        }
        for (dmg, from) in hurt {
            if self.player.iframes <= 0.0 || dmg < 6.0 {
                self.damage_player(dmg, from);
            }
        }
        for p in pools {
            self.spawn_hazard(HazardKind::Acid, p, Vec2::ZERO, 1.15, 4.5);
            self.sfx.push(Sfx::Splat(p, ZKind::Walker));
        }
        // sparks die on walls; everything else when its time is up
        let map = &self.map;
        self.hazards.retain(|h| h.t < h.life && !(h.kind == HazardKind::Spark && map.solid_at(h.pos)));
    }

    /// Debug: stop new zombies arriving.
    pub fn max_alive_override(&mut self, n: usize) {
        self.max_alive = n;
        self.spawn_rate = (0.0, 0.0);
    }

    /// A round (or a blast) hits something breakable.
    pub fn hit_prop(&mut self, pi: usize, dmg: f32, dir: Vec2) {
        let pr = self.map.props[pi];
        let c = pr.pos + pr.size * 0.5;
        let glass = matches!(pr.kind, PropKind::Vending(_) | PropKind::Kiosk);
        for _ in 0..4 {
            let d = (-dir + rand_dir() * 0.8).normalize_or_zero();
            let col = if glass { Color::new(0.8, 0.9, 1.0, 1.0) } else { Color::new(0.7, 0.6, 0.45, 1.0) };
            emit(&mut self.particles, particle(PK::Debris, c - dir * 0.3, rnd(4.0, 16.0), vec3(d.x * rnd(1.0, 3.0), d.y * rnd(1.0, 3.0), rnd(40.0, 110.0)), rnd(0.6, 1.2), 1.0, col));
        }
        if self.map.damage_prop(pi, dmg) {
            self.break_prop(pi, dir);
        } else {
            self.sfx.push(Sfx::Tick(c));
        }
    }

    fn break_prop(&mut self, pi: usize, dir: Vec2) {
        let pr = self.map.props[pi];
        let c = pr.pos + pr.size * 0.5;
        let glass = matches!(pr.kind, PropKind::Vending(_) | PropKind::Kiosk);
        self.sfx.push(Sfx::Smash(c, glass));
        self.shake = (self.shake + if glass { 0.25 } else { 0.1 }).min(1.0);
        let n = if glass { 30 } else { 14 };
        for _ in 0..n {
            let d = (dir * 0.5 + rand_dir()).normalize_or_zero();
            let sp = rnd(1.5, 5.0);
            let col = if glass && chance(0.5) {
                Color::new(0.75, 0.9, 1.0, 1.0)
            } else if matches!(pr.kind, PropKind::Vending(_)) && chance(0.4) {
                [Color::new(0.9, 0.2, 0.2, 1.0), Color::new(0.2, 0.5, 0.95, 1.0), Color::new(0.95, 0.75, 0.2, 1.0)][rand::gen_range(0, 3)]
            } else {
                Color::new(0.55, 0.5, 0.45, 1.0)
            };
            emit(&mut self.particles, particle(PK::Debris, c, rnd(4.0, 22.0), vec3(d.x * sp, d.y * sp, rnd(60.0, 200.0)), rnd(0.8, 1.8), rnd(1.0, 2.0), col));
        }
        if glass {
            for _ in 0..12 {
                let d = rand_dir();
                emit(&mut self.particles, particle(PK::Spark, c, 14.0, vec3(d.x * rnd(1.0, 4.0), d.y * rnd(1.0, 4.0), rnd(20.0, 120.0)), rnd(0.2, 0.5), 1.0, Color::new(0.7, 0.9, 1.0, 1.0)));
            }
            self.lights.push(TempLight { pos: c, radius: 2.4, color: Color::new(0.7, 0.85, 1.0, 1.0), life: 0.15, max: 0.15 });
        }
        // coins spill out
        let total = ((pr.coins() as f32) * self.coin_mult).round() as u32;
        let pieces = (total as f32 / 2.0).ceil().max(1.0) as u32;
        for k in 0..pieces {
            let v = if k + 1 == pieces { total - 2 * (pieces - 1) } else { 2 }.max(1);
            let d = rand_dir();
            let mut pk = Pickup::new(c, PickupKind::Coin(v));
            pk.z = 10.0;
            pk.vz = rnd(80.0, 160.0);
            pk.vel = d * rnd(1.0, 3.0);
            self.pickups.push(pk);
        }
    }

    /// Fire a bullet from anyone (the officer in the opening).
    pub fn fire_bullet(&mut self, from: Vec2, dir: Vec2, dmg: f32) {
        let d = dir.normalize_or_zero();
        self.bullets.push(Bullet { pos: from + d * 0.5, vel: d * 15.0, life: 1.5, dmg, pierce: 0, hits: [0; 8], nh: 0 });
        self.muzzle(from + d * 0.5);
        self.sfx.push(Sfx::Shot);
    }

    /// Effects the opening script needs.
    pub fn blood_burst(&mut self, at: Vec2, dir: Vec2, n: usize) {
        blood_spray(&mut self.particles, at, 10.0, dir, n, 1.2);
        gibs(&mut self.particles, at, n / 4, 1.0);
        self.map.stamp(at, 0.8, 0);
    }

    pub fn vomit(&mut self, at: Vec2, dir: Vec2) {
        for _ in 0..3 {
            let d = (dir + rand_dir() * 0.3).normalize_or_zero();
            let c = Color::new(rnd(0.55, 0.7), rnd(0.6, 0.72), rnd(0.2, 0.3), 1.0);
            emit(&mut self.particles, particle(PK::Blood, at + dir * 0.2, 9.0, vec3(d.x * rnd(0.8, 1.6), d.y * rnd(0.8, 1.6), rnd(10.0, 40.0)), 0.6, rnd(1.0, 2.0), c));
        }
    }

    pub fn muzzle(&mut self, at: Vec2) {
        self.lights.push(TempLight { pos: at, radius: 2.8, color: Color::new(1.0, 0.8, 0.45, 1.0), life: 0.06, max: 0.06 });
        emit(&mut self.particles, particle(PK::Flash, at, 10.0, Vec3::ZERO, 0.05, 6.0, Color::new(1.0, 0.85, 0.5, 1.0)));
        self.loud = (at, self.time);
    }

    pub fn spawn_train(&mut self, y: f32, head: f32, target: f32, doors_open: bool, pass: bool) {
        let slices: Vec<SliceK> = train_pattern()
            .iter()
            .map(|k| match k {
                0 => SliceK::Cab,
                2 => SliceK::Door,
                3 => SliceK::Gap,
                _ => SliceK::Body,
            })
            .collect();
        self.train = Some(Train { head, target, y, slices, stopped_t: if doors_open { 9.0 } else { 0.0 }, doors_open, pass });
    }

    fn update_npcs(&mut self, dt: f32) {
        let n = self.npcs.len();
        let snapshot: Vec<(Vec2, NpcState)> = self.npcs.iter().map(|k| (k.pos, k.state)).collect();
        let field = std::mem::take(&mut self.npc_field);
        for i in 0..n {
            let mut k = self.npcs[i];
            k.t += dt;
            match k.state {
                NpcState::Stand | NpcState::Strap => {
                    k.vel += (k.home - k.pos) * 5.0 * dt;
                    k.vel *= (-3.5 * dt).exp();
                    k.pos += k.vel * dt;
                }
                NpcState::Sit | NpcState::Kneel | NpcState::Corpse | NpcState::Gone => {}
                NpcState::Shoot => {
                    k.face_left = world_to_screen_dir(k.aim).x < 0.0;
                }
                NpcState::Walk => {
                    let to = k.target - k.pos;
                    let dir = if !field.is_empty() && to.length() > 1.5 {
                        self.map.dir_in(&field, k.pos).unwrap_or(to.normalize_or_zero())
                    } else {
                        to.normalize_or_zero()
                    };
                    let mut push = Vec2::ZERO;
                    for (j, (q, st)) in snapshot.iter().enumerate() {
                        if j != i && matches!(st, NpcState::Walk | NpcState::Stand) {
                            let d = k.pos - *q;
                            let l = d.length();
                            if l < 0.5 && l > 1e-4 {
                                push += d / l * (0.5 - l);
                            }
                        }
                    }
                    k.vel += (dir * k.speed - k.vel) * (1.0 - (-6.0 * dt).exp());
                    k.pos += k.vel * dt + push * 0.5;
                    self.map.resolve(&mut k.pos, 0.22, false);
                    k.anim += dt * k.vel.length() * 2.6;
                    let sd = world_to_screen_dir(k.vel);
                    if sd.x.abs() > 0.3 {
                        k.face_left = sd.x < 0.0;
                    }
                    k.back = sd.y < -1.5;
                    if to.length() < 0.45 {
                        k.state = k.after;
                        k.t = 0.0;
                        k.home = k.pos;
                    }
                }
                NpcState::Ride => {
                    if let Some((a, b)) = self.map.escalator {
                        let f = (k.t / 6.5).min(1.0);
                        k.pos = a.lerp(b, f);
                        k.z = f * crate::art::esc_height(crate::art::ESC_SLICES - 1) + 2.0;
                        k.anim += dt * 2.0;
                        k.face_left = false;
                        k.back = true;
                        if f >= 1.0 {
                            k.state = NpcState::Gone;
                        }
                    } else {
                        k.state = NpcState::Gone;
                    }
                }
            }
            self.npcs[i] = k;
        }
        self.npc_field = field;
        self.npcs.retain(|k| k.state != NpcState::Gone);
    }

    // ------------------------------------------------------------ the train

    fn update_train(&mut self, dt: f32) {
        if let Some(t) = &self.train {
            if t.pass && t.tail() > self.map.gw as f32 + 2.0 {
                self.train = None;
                return;
            }
        }
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
                    !z.dead && z.kind != ZKind::Boss && (z.pos.y - ty).abs() < 1.0 && z.pos.x < head + 0.3 && z.pos.x > tail
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
                        if (p.x - cx).abs() < 0.6 && (p.y - t.y).abs() < 2.2 {
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
                        self.boss_cd = [3.0, 5.0, 8.0, 6.0];
                        self.boss_phase = 0;
                        self.say(self.boss_kind.name(), self.boss_kind.entrance(), 3.5);
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
                    let slices: Vec<SliceK> = train_pattern()
                        .iter()
                        .map(|k| match k {
                            0 => SliceK::Cab,
                            2 => SliceK::Door,
                            3 => SliceK::Gap,
                            _ => SliceK::Body,
                        })
                        .collect();
                    let target = train_stop_head(self.map.w);
                    self.train = Some(Train { head: 0.0, target, y, slices, stopped_t: 0.0, doors_open: false, pass: false });
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
            Phase::Boss | Phase::Intro => {}
        }
    }

    // ------------------------------------------------------------ crows

    fn update_birds(&mut self, dt: f32) {
        let ppos = self.player.pos;
        let (loud_at, loud_t) = self.loud;
        let recent_noise = self.time - loud_t < 0.4;
        let mut caws = Vec::new();
        for b in &mut self.birds {
            b.t += dt;
            if !b.flying {
                // idle: peck, hop now and then
                if b.vel.length_squared() > 0.0 {
                    b.pos += vec2(b.vel.x, b.vel.y) * dt;
                    b.vel *= (-10.0 * dt).exp();
                    if b.vel.length() < 0.05 {
                        b.vel = Vec3::ZERO;
                    }
                } else if chance(dt * 0.5) {
                    let d = rand_dir();
                    b.vel = vec3(d.x, d.y, 0.0) * 1.5;
                    b.face_left = world_to_screen_dir(d).x < 0.0;
                }
                let scared = b.pos.distance(ppos) < 3.2 && self.phase != Phase::Dead
                    || (recent_noise && b.pos.distance(loud_at) < 7.0)
                    || self.zombies.iter().any(|z| z.pos.distance(b.pos) < 0.9);
                if scared {
                    let away = (b.pos - ppos).normalize_or_zero();
                    let d = (away + rand_dir() * 0.5).normalize_or_zero();
                    b.flying = true;
                    b.leaving = true;
                    b.vel = vec3(d.x * rnd(3.0, 5.0), d.y * rnd(3.0, 5.0), rnd(45.0, 75.0));
                    b.face_left = world_to_screen_dir(d).x < 0.0;
                    caws.push(b.pos);
                }
            } else if b.leaving {
                b.pos += vec2(b.vel.x, b.vel.y) * dt;
                b.z += b.vel.z * dt;
                b.anim += dt * 16.0;
            } else {
                // gliding in to land
                let to = b.target - b.pos;
                b.pos += to.normalize_or_zero() * (to.length() * 1.5).min(4.0) * dt;
                b.z = (b.z - 38.0 * dt).max(0.0);
                b.anim += dt * 9.0;
                if b.z <= 0.0 {
                    b.flying = false;
                    b.vel = Vec3::ZERO;
                }
            }
        }
        self.birds.retain(|b| !(b.leaving && b.z > 120.0));
        for p in caws.into_iter().take(2) {
            self.sfx.push(Sfx::Caw(p));
        }
        // new crows drift in to pick at the dead
        self.bird_t -= dt;
        let max = 4 + self.station * 2;
        if self.bird_t <= 0.0 && self.birds.len() < max && !self.kill_spots.is_empty() && !self.demo {
            self.bird_t = rnd(5.0, 10.0);
            let target = self.kill_spots[rand::gen_range(0, self.kill_spots.len())] + rand_dir() * rnd(0.3, 1.0);
            if target.distance(ppos) > 5.0 && self.map.tile(target.x, target.y) != Tile::Track {
                let from = target + rand_dir() * 6.0;
                self.birds.push(Bird { pos: from, z: 90.0, vel: Vec3::ZERO, flying: true, leaving: false, target, t: 0.0, anim: 0.0, face_left: (target - from).x - (target - from).y < 0.0 });
            }
        }
    }

    // ------------------------------------------------------------ atmosphere

    fn update_atmosphere(&mut self, dt: f32) {
        let ppos = self.player.pos;
        // failing fluorescent tubes spit sparks
        let lamps: Vec<Vec2> = self.map.lights.iter().filter(|l| l.flicker > 0.0 && l.pos.distance(ppos) < 14.0).map(|l| l.pos).collect();
        for at in lamps {
            if chance(dt * 0.22) {
                for _ in 0..rand::gen_range(6, 12) {
                    let d = rand_dir();
                    emit(&mut self.particles, particle(PK::Spark, at, 58.0, vec3(d.x * rnd(0.5, 2.0), d.y * rnd(0.5, 2.0), rnd(-20.0, 40.0)), rnd(0.5, 1.0), 1.0, Color::new(0.75, 0.9, 1.0, 1.0)));
                }
                self.lights.push(TempLight { pos: at, radius: 2.2, color: Color::new(0.7, 0.85, 1.0, 1.0), life: 0.12, max: 0.12 });
                self.sfx.push(Sfx::Zap(at));
            }
        }
        // stale air drifting out of the tunnels
        let tunnels: Vec<Vec2> = self.map.tunnel_spawns.iter().copied().filter(|t| t.distance(ppos) < 18.0).collect();
        for t in tunnels {
            if chance(dt * 1.4) {
                let p = t + vec2(0.1, rnd(-1.2, 1.2));
                emit(&mut self.particles, particle(PK::Smoke, p, rnd(2.0, 18.0), vec3(rnd(0.3, 0.7), rnd(-0.1, 0.1), rnd(1.0, 3.0)), rnd(3.0, 5.0), rnd(6.0, 10.0), Color::new(0.45, 0.45, 0.5, 0.22)));
            }
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
