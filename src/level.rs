//! Station definitions and map building: the baked floor/wall image, props,
//! lights, spawn points, collision, and the zombie flow field.

use crate::art::{build_signs, Art, SignArt};
use crate::font;
use crate::util::*;
use macroquad::prelude::*;
use std::collections::VecDeque;

pub const WALL_H: f32 = 76.0;
pub const TRACK_DROP: f32 = 6.0;
pub const MARGIN: i32 = 5;

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Band {
    Platform,
    Track,
    Concourse,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Tile {
    Wall,
    Platform,
    Track,
    Concourse,
}

pub struct StationDef {
    pub name: &'static str,
    pub code: &'static str,
    pub width: i32,
    pub bands: Vec<(Band, i32)>,
    pub quota: u32,
    pub max_alive: usize,
    pub spawn_rate: (f32, f32),
    pub mix: [f32; 3], // walker, runner, brute
    pub ambient: Color,
    pub wall: Color,
    pub floor: Color,
    pub lamp: Color,
    pub neon: Vec<Color>,
    pub flicker: f32,
    pub boss: bool,
    pub tagline: &'static str,
    /// The inside of a train car (the opening scene), not a station.
    pub car: bool,
    /// The opening station: an escalator up to the concourse and an arcade cabinet.
    pub intro_props: bool,
    /// Campaign levels: the concourse becomes a maze of shuttered shops.
    pub maze: bool,
}

/// Doors along a car, as x centers (both sides line up).
pub const CAR_DOORS: [f32; 4] = [3.0, 8.5, 14.0, 19.5];

/// The opening scene: a crowded Yamanote line car between stations.
pub fn car_def() -> StationDef {
    StationDef {
        name: "YAMANOTE LINE",
        code: "JY",
        width: 22,
        bands: vec![(Band::Concourse, 5)],
        quota: 0,
        max_alive: 0,
        spawn_rate: (0.0, 0.0),
        mix: [1.0, 0.0, 0.0],
        ambient: Color::new(0.2, 0.2, 0.21, 1.0),
        wall: rgb(196, 198, 204),
        floor: rgb(92, 100, 116),
        lamp: Color::new(1.0, 0.96, 0.88, 1.0),
        neon: vec![rgb(255, 255, 255)],
        flicker: 0.0,
        boss: false,
        tagline: "",
        car: true,
        intro_props: false,
        maze: false,
    }
}

/// One stop on the Yamanote loop. Size: 0 small, 1 medium, 2 large (halls below), 3 huge.
pub struct LoopStop {
    pub name: &'static str,
    pub code: &'static str,
    pub size: u8,
    pub boss: bool,
}

/// The whole loop, from Akihabara all the way around (through Ueno, Ikebukuro,
/// Shinjuku, Shibuya, Shinagawa, Tokyo and Kanda) and back to Akihabara.
pub const LOOP: [LoopStop; 31] = [
    LoopStop { name: "AKIHABARA", code: "JY03", size: 2, boss: false },
    LoopStop { name: "OKACHIMACHI", code: "JY04", size: 0, boss: false },
    LoopStop { name: "UENO", code: "JY05", size: 3, boss: true },
    LoopStop { name: "UGUISUDANI", code: "JY06", size: 0, boss: false },
    LoopStop { name: "NIPPORI", code: "JY07", size: 1, boss: false },
    LoopStop { name: "NISHI-NIPPORI", code: "JY08", size: 0, boss: false },
    LoopStop { name: "TABATA", code: "JY09", size: 1, boss: false },
    LoopStop { name: "KOMAGOME", code: "JY10", size: 0, boss: false },
    LoopStop { name: "SUGAMO", code: "JY11", size: 1, boss: false },
    LoopStop { name: "OTSUKA", code: "JY12", size: 0, boss: false },
    LoopStop { name: "IKEBUKURO", code: "JY13", size: 3, boss: true },
    LoopStop { name: "MEJIRO", code: "JY14", size: 0, boss: false },
    LoopStop { name: "TAKADANOBABA", code: "JY15", size: 1, boss: false },
    LoopStop { name: "SHIN-OKUBO", code: "JY16", size: 0, boss: false },
    LoopStop { name: "SHINJUKU", code: "JY17", size: 3, boss: true },
    LoopStop { name: "YOYOGI", code: "JY18", size: 1, boss: false },
    LoopStop { name: "HARAJUKU", code: "JY19", size: 1, boss: false },
    LoopStop { name: "SHIBUYA", code: "JY20", size: 3, boss: true },
    LoopStop { name: "EBISU", code: "JY21", size: 1, boss: false },
    LoopStop { name: "MEGURO", code: "JY22", size: 1, boss: false },
    LoopStop { name: "GOTANDA", code: "JY23", size: 1, boss: false },
    LoopStop { name: "OSAKI", code: "JY24", size: 0, boss: false },
    LoopStop { name: "SHINAGAWA", code: "JY25", size: 3, boss: true },
    LoopStop { name: "TAKANAWA GATEWAY", code: "JY26", size: 1, boss: false },
    LoopStop { name: "TAMACHI", code: "JY27", size: 1, boss: false },
    LoopStop { name: "HAMAMATSUCHO", code: "JY28", size: 1, boss: false },
    LoopStop { name: "SHIMBASHI", code: "JY29", size: 2, boss: false },
    LoopStop { name: "YURAKUCHO", code: "JY30", size: 1, boss: false },
    LoopStop { name: "TOKYO", code: "JY01", size: 3, boss: true },
    LoopStop { name: "KANDA", code: "JY02", size: 1, boss: false },
    LoopStop { name: "AKIHABARA", code: "JY03", size: 2, boss: true },
];

/// Build stop `i` of the loop: its size sets the layout, its position sets the difficulty.
pub fn loop_def(i: usize) -> StationDef {
    use Band::*;
    let stop = &LOOP[i];
    let f = i as f32;
    let (width, bands, mult) = match stop.size {
        0 => (36, vec![(Platform, 6), (Track, 3), (Platform, 7), (Track, 3), (Platform, 6), (Concourse, 6)], 0.8),
        1 => (42, vec![(Platform, 7), (Track, 3), (Platform, 8), (Track, 3), (Platform, 6), (Concourse, 10)], 1.0),
        2 => (50, vec![(Platform, 6), (Track, 3), (Platform, 7), (Track, 3), (Platform, 5), (Concourse, 30)], 1.2),
        _ => (54, vec![(Platform, 6), (Track, 3), (Platform, 6), (Track, 3), (Platform, 6), (Track, 3), (Platform, 5), (Concourse, 28)], 1.4),
    };
    // a handful of looks so neighbouring stations feel different
    let looks: [(Color, Color, Color, Color, Vec<Color>); 6] = [
        (Color::new(0.14, 0.10, 0.19, 1.0), rgb(214, 200, 210), rgb(138, 134, 140), Color::new(0.85, 0.8, 1.0, 1.0), vec![rgb(255, 60, 200), rgb(40, 230, 255)]),
        (Color::new(0.09, 0.14, 0.15, 1.0), rgb(196, 210, 204), rgb(132, 138, 134), Color::new(0.8, 1.0, 0.95, 1.0), vec![rgb(60, 220, 160), rgb(250, 250, 240)]),
        (Color::new(0.17, 0.12, 0.07, 1.0), rgb(222, 206, 180), rgb(146, 138, 124), Color::new(1.0, 0.86, 0.62, 1.0), vec![rgb(255, 140, 40), rgb(255, 210, 90)]),
        (Color::new(0.06, 0.07, 0.12, 1.0), rgb(180, 190, 206), rgb(118, 122, 132), Color::new(0.75, 0.85, 1.0, 1.0), vec![rgb(80, 120, 255)]),
        (Color::new(0.12, 0.12, 0.12, 1.0), rgb(210, 210, 204), rgb(134, 132, 128), Color::new(1.0, 0.97, 0.9, 1.0), vec![rgb(255, 230, 60), rgb(255, 255, 255)]),
        (Color::new(0.13, 0.08, 0.13, 1.0), rgb(206, 196, 214), rgb(130, 126, 136), Color::new(0.95, 0.85, 1.0, 1.0), vec![rgb(200, 120, 255), rgb(120, 200, 255)]),
    ];
    let (mut ambient, wall, floor, lamp, neon) = looks[(i * 7 + 3) % 6].clone();
    if stop.boss {
        ambient = Color::new(0.16, 0.05, 0.05, 1.0); // emergency red where something big is waiting
    }
    StationDef {
        name: stop.name,
        code: stop.code,
        width,
        bands,
        // like arcade: kill this many and the station is clear
        quota: ((45.0 + f * 6.0) * mult) as u32,
        max_alive: (45.0 + f * 7.0).min(320.0) as usize,
        spawn_rate: ((1.4 + f * 0.18).min(10.0), (2.8 + f * 0.32).min(16.0)),
        mix: [1.0, (0.08 + f * 0.03).min(0.8), if i >= 5 { (0.02 + f * 0.008).min(0.25) } else { 0.0 }],
        ambient,
        wall,
        floor,
        lamp,
        neon,
        flicker: (0.1 + f * 0.012).min(0.5),
        boss: stop.boss,
        tagline: "",
        car: false,
        intro_props: false,
        maze: stop.size >= 2,
    }
}

/// Campaign: Kanda, where the opening happens.
pub fn kanda_def() -> StationDef {
    StationDef {
        name: "KANDA",
        code: "JY02",
        width: 30,
        bands: vec![(Band::Platform, 5), (Band::Track, 3), (Band::Platform, 7), (Band::Track, 3), (Band::Platform, 6)],
        quota: 40,
        max_alive: 32,
        spawn_rate: (1.5, 3.5),
        mix: [1.0, 0.15, 0.0],
        ambient: Color::new(0.15, 0.12, 0.13, 1.0),
        wall: rgb(220, 206, 196),
        floor: rgb(140, 134, 130),
        lamp: Color::new(1.0, 0.92, 0.82, 1.0),
        neon: vec![rgb(255, 200, 120), rgb(120, 200, 255)],
        flicker: 0.15,
        boss: false,
        tagline: "",
        car: false,
        intro_props: true,
        maze: false,
    }
}


/// The route: clockwise up the Yamanote line from Akihabara to Shinjuku.
pub fn stations() -> Vec<StationDef> {
    use Band::*;
    vec![
        StationDef {
            name: "AKIHABARA",
            code: "JY03",
            width: 34,
            bands: vec![(Platform, 6), (Track, 3), (Platform, 7), (Track, 3), (Platform, 6)],
            quota: 90,
            max_alive: 110,
            spawn_rate: (3.0, 7.0),
            mix: [1.0, 0.0, 0.0],
            ambient: Color::new(0.15, 0.10, 0.20, 1.0),
            wall: rgb(214, 200, 210),
            floor: rgb(140, 136, 140),
            lamp: Color::new(0.85, 0.80, 1.0, 1.0),
            neon: vec![rgb(255, 60, 200), rgb(40, 230, 255), rgb(255, 230, 60)],
            flicker: 0.12,
            boss: false,
            tagline: "ELECTRIC TOWN. THE NEON NEVER WENT OUT.",
            car: false,
            intro_props: false,
            maze: false,
        },
        StationDef {
            name: "UENO",
            code: "JY05",
            width: 38,
            bands: vec![(Platform, 6), (Track, 3), (Platform, 7), (Track, 3), (Platform, 5), (Concourse, 5)],
            quota: 140,
            max_alive: 160,
            spawn_rate: (4.0, 9.0),
            mix: [1.0, 0.25, 0.0],
            ambient: Color::new(0.09, 0.14, 0.15, 1.0),
            wall: rgb(196, 210, 204),
            floor: rgb(132, 138, 134),
            lamp: Color::new(0.8, 1.0, 0.95, 1.0),
            neon: vec![rgb(60, 220, 160), rgb(250, 250, 240)],
            flicker: 0.2,
            boss: false,
            tagline: "THE PARK GATES ARE SHUT. THEY CAME UP THE STAIRS.",
            car: false,
            intro_props: false,
            maze: false,
        },
        StationDef {
            name: "IKEBUKURO",
            code: "JY13",
            width: 40,
            bands: vec![(Platform, 5), (Track, 3), (Platform, 7), (Track, 3), (Platform, 7), (Track, 3), (Platform, 5)],
            quota: 190,
            max_alive: 210,
            spawn_rate: (5.0, 11.0),
            mix: [1.0, 0.4, 0.08],
            ambient: Color::new(0.17, 0.12, 0.07, 1.0),
            wall: rgb(222, 206, 180),
            floor: rgb(146, 138, 124),
            lamp: Color::new(1.0, 0.86, 0.62, 1.0),
            neon: vec![rgb(255, 140, 40), rgb(255, 210, 90)],
            flicker: 0.25,
            boss: false,
            tagline: "TWO MILLION COMMUTERS A DAY. MOST NEVER LEFT.",
            car: false,
            intro_props: false,
            maze: false,
        },
        StationDef {
            name: "TAKADANOBABA",
            code: "JY15",
            width: 38,
            bands: vec![(Platform, 6), (Track, 3), (Platform, 8), (Track, 3), (Platform, 6), (Concourse, 5)],
            quota: 240,
            max_alive: 270,
            spawn_rate: (6.0, 13.0),
            mix: [1.0, 0.6, 0.15],
            ambient: Color::new(0.05, 0.06, 0.11, 1.0),
            wall: rgb(180, 190, 206),
            floor: rgb(118, 122, 132),
            lamp: Color::new(0.75, 0.85, 1.0, 1.0),
            neon: vec![rgb(80, 120, 255)],
            flicker: 0.5,
            boss: false,
            tagline: "THE LIGHTS ARE DYING. STAY IN THE LIGHT.",
            car: false,
            intro_props: false,
            maze: false,
        },
        StationDef {
            name: "SHINJUKU",
            code: "JY17",
            width: 46,
            bands: vec![
                (Platform, 6),
                (Track, 3),
                (Platform, 6),
                (Track, 3),
                (Platform, 6),
                (Track, 3),
                (Platform, 6),
                (Track, 3),
                (Platform, 5),
                (Concourse, 5),
            ],
            quota: 300,
            max_alive: 340,
            spawn_rate: (7.0, 15.0),
            mix: [1.0, 0.7, 0.2],
            ambient: Color::new(0.16, 0.05, 0.05, 1.0),
            wall: rgb(210, 200, 196),
            floor: rgb(130, 124, 124),
            lamp: Color::new(1.0, 0.9, 0.85, 1.0),
            neon: vec![rgb(255, 40, 40), rgb(255, 255, 255), rgb(120, 200, 255)],
            flicker: 0.3,
            boss: true,
            tagline: "THE BUSIEST STATION ON EARTH. END OF THE LINE.",
            car: false,
            intro_props: false,
            maze: false,
        },
    ]
}

#[derive(Clone, Copy)]
pub struct Obstacle {
    pub min: Vec2,
    pub max: Vec2,
    /// Waist-high: blocks walking but bullets fly over it.
    pub low: bool,
}

#[derive(Clone, Copy, PartialEq)]
#[allow(dead_code)]
pub enum PropKind {
    Pillar,
    Vending(usize),
    Bench,
    Gate,
    Bin,
    Psd,
    SignName,
    SignLed,
    Kiosk,
    Suitcase(usize),
    Boxes,
    Barrier,
    EscStep(usize),
    EscRail(usize),
    Cabinet,
    CarSeat,
    CarWall,
    CarDoor,
    Straps,
    Ad(usize),
    MazeWall(usize),
}

impl PropKind {
    /// Tall things that should fade out when the player walks behind them.
    pub fn occludes(&self) -> bool {
        matches!(self, PropKind::Pillar | PropKind::SignName | PropKind::SignLed | PropKind::Kiosk | PropKind::Vending(_) | PropKind::Cabinet | PropKind::EscRail(_) | PropKind::EscStep(_) | PropKind::Straps | PropKind::Ad(_) | PropKind::MazeWall(_))
    }
}

/// Train composition shared by the world (train sprites) and the map (door gaps).
/// 0 = cab, 1 = body, 2 = door, 3 = gap between cars.
pub fn train_pattern() -> Vec<u8> {
    let mut v = Vec::new();
    for car in 0..4 {
        v.extend_from_slice(&[if car == 0 { 0 } else { 1 }, 1, 2, 1, 1, 2, 1]);
        if car < 3 {
            v.push(3);
        }
    }
    v
}

/// Where the train's head stops at a station of width `w`.
pub fn train_stop_head(w: i32) -> f32 {
    (w - 2) as f32
}

/// Centers of the train doors when stopped; platform screen doors leave gaps here.
pub fn door_xs(w: i32) -> Vec<f32> {
    let head = train_stop_head(w);
    train_pattern()
        .iter()
        .enumerate()
        .filter(|(_, k)| **k == 2)
        .map(|(i, _)| head - 1.0 - i as f32 + 0.5)
        .filter(|x| *x > 1.8)
        .collect()
}

/// Tiny deterministic RNG for laying out each station the same way every time.
struct Lcg(u32);
impl Lcg {
    fn f(&mut self) -> f32 {
        self.0 = self.0.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        (self.0 >> 8) as f32 / 16_777_216.0
    }
    fn range(&mut self, a: f32, b: f32) -> f32 {
        a + self.f() * (b - a)
    }
}

#[derive(Clone, Copy)]
pub struct Prop {
    pub kind: PropKind,
    pub pos: Vec2, // min corner in world tiles
    pub size: Vec2,
}

#[derive(Clone, Copy)]
pub struct LightDef {
    pub pos: Vec2,
    pub radius: f32,
    pub color: Color,
    pub flicker: f32,
    pub phase: f32,
    pub strobe: bool,
}

struct Sign {
    u0: f32,
    u1: f32,
    text: String,
}

struct Ad {
    u0: f32,
    u1: f32,
    a: Color,
    b: Color,
    pattern: u32,
}

pub struct Map {
    pub w: i32,
    pub h: i32,
    pub gw: i32,
    pub gh: i32,
    pub tiles: Vec<Tile>,
    pub blocked: Vec<bool>,
    pub obstacles: Vec<Obstacle>,
    obs_grid: Vec<Vec<u16>>,
    pub props: Vec<Prop>,
    pub lights: Vec<LightDef>,
    pub tunnel_spawns: Vec<Vec2>,
    pub edge_spawns: Vec<Vec2>,
    pub track_ys: Vec<f32>,
    pub signs: SignArt,
    /// Escalator: bottom entrance, top, and its y center (opening only).
    pub escalator: Option<(Vec2, Vec2)>,
    pub arcade: Option<Vec2>,
    pub is_car: bool,
    walls_h: Vec<bool>, // wall between cell (x, y) and (x, y + 1): platform screen doors
    pub floor: Image,
    floor_mask: Vec<bool>,
    pub floor_tex: Texture2D,
    pub origin: Vec2,
    dirty: Option<(i32, i32, i32, i32)>,
    flow: Vec<u16>,
    flow_cell: (i32, i32),
}

const FLOW_MAX: u16 = u16::MAX;

impl Map {
    pub fn build(def: &StationDef, index: usize, next: &str, _art: &Art) -> Map {
        let w = def.width;
        let h = 1 + def.bands.iter().map(|b| b.1).sum::<i32>();
        let gw = w + MARGIN;
        let gh = h + MARGIN;

        // row -> band lookup
        let mut row_band = vec![Band::Concourse; gh as usize];
        let mut band_ranges = Vec::new();
        let mut y = 1;
        for (b, n) in &def.bands {
            band_ranges.push((*b, y, y + n));
            for r in y..y + n {
                row_band[r as usize] = *b;
            }
            y += n;
        }
        let last = def.bands.last().map(|b| b.0).unwrap_or(Band::Platform);
        for r in y..gh {
            row_band[r as usize] = if last == Band::Track { Band::Platform } else { last };
        }

        let mut tiles = vec![Tile::Platform; (gw * gh) as usize];
        for ty in 0..gh {
            for tx in 0..gw {
                let t = if tx < 1 || ty < 1 {
                    Tile::Wall
                } else {
                    match row_band[ty as usize] {
                        Band::Platform => Tile::Platform,
                        Band::Track => Tile::Track,
                        Band::Concourse => Tile::Concourse,
                    }
                };
                tiles[(ty * gw + tx) as usize] = t;
            }
        }

        let track_bands: Vec<(i32, i32)> =
            band_ranges.iter().filter(|b| b.0 == Band::Track).map(|b| (b.1, b.2)).collect();
        let track_ys: Vec<f32> = track_bands.iter().map(|(a, b)| (*a + *b) as f32 * 0.5).collect();

        let seed = 1000 + index as u32 * 77;
        let mut props = Vec::new();
        let mut obstacles = Vec::new();
        let mut lights = Vec::new();
        let add_prop = |kind: PropKind, center: Vec2, size: Vec2, solid: bool, props: &mut Vec<Prop>, obstacles: &mut Vec<Obstacle>| {
            let pos = center - size * 0.5;
            props.push(Prop { kind, pos, size });
            if solid {
                let low = matches!(kind, PropKind::Bench | PropKind::Gate | PropKind::Bin);
                obstacles.push(Obstacle { min: pos, max: pos + size, low });
            }
        };

        let neon = |i: u32| def.neon[(i as usize) % def.neon.len().max(1)];
        let mut k = 0u32;
        let escalator = None;
        let mut arcade = None;
        if def.car {
            // seats along both sides between the doors, straps, hanging ads, ceiling lights
            let gaps: Vec<(f32, f32)> = CAR_DOORS.iter().map(|d| (d - 0.75, d + 0.75)).collect();
            let in_gap = |x: f32| gaps.iter().any(|(a, b)| x + 1.0 > *a && x < *b);
            let mut x = 0.4;
            while x + 1.0 <= w as f32 - 0.3 {
                if !in_gap(x) {
                    for y in [1.0f32, h as f32 - 0.55] {
                        props.push(Prop { kind: PropKind::CarSeat, pos: vec2(x, y), size: vec2(1.0, 0.55) });
                        obstacles.push(Obstacle { min: vec2(x, y), max: vec2(x + 1.0, y + 0.55), low: true });
                    }
                }
                x += 1.0;
            }
            let mut x = 0.5;
            while x + 0.5 <= w as f32 {
                let door = gaps.iter().any(|(a, b)| x + 0.25 > *a && x + 0.25 < *b);
                props.push(Prop { kind: if door { PropKind::CarDoor } else { PropKind::CarWall }, pos: vec2(x, h as f32 - 0.02), size: vec2(0.5, 0.1) });
                x += 0.5;
            }
            for y in [2.2f32, h as f32 - 1.6] {
                let mut x = 0.5;
                while x + 1.0 <= w as f32 {
                    props.push(Prop { kind: PropKind::Straps, pos: vec2(x, y), size: vec2(1.0, 0.05) });
                    x += 1.0;
                }
            }
            let mut x = 1.2;
            let mut n = 0;
            while x + 1.1 < w as f32 {
                props.push(Prop { kind: PropKind::Ad(n % 3), pos: vec2(x, 3.4), size: vec2(1.1, 0.04) });
                n += 1;
                x += 2.6;
            }
            for y in [2.2f32, h as f32 - 1.6] {
                let mut x = 1.0;
                while x < w as f32 {
                    lights.push(LightDef { pos: vec2(x, y), radius: 2.4, color: def.lamp, flicker: 0.0, phase: 0.0, strobe: false });
                    x += 2.6;
                }
            }
        }
        if def.intro_props {
            // an arcade cabinet in the corner of the middle platform
            if let Some((_, y0, y1)) = band_ranges.iter().find(|b| b.0 == Band::Platform && b.1 > 1) {
                let c = vec2(2.1, (*y0 + *y1) as f32 * 0.5 - 0.35);
                props.push(Prop { kind: PropKind::Cabinet, pos: c - vec2(0.4, 0.35), size: vec2(0.8, 0.7) });
                obstacles.push(Obstacle { min: c - vec2(0.4, 0.35), max: c + vec2(0.4, 0.35), low: false });
                arcade = Some(c + vec2(0.0, 0.9));
                lights.push(LightDef { pos: c + vec2(0.0, 1.0), radius: 2.0, color: rgb(140, 120, 255), flicker: 0.0, phase: 0.0, strobe: false });
            }
        }
        for (bi, (band, y0, y1)) in band_ranges.iter().enumerate().filter(|_| !def.car) {
            let (y0f, y1f) = (*y0 as f32, *y1 as f32);
            let yc = (y0f + y1f) * 0.5;
            let above_track = bi > 0 && band_ranges[bi - 1].0 == Band::Track;
            let below_track = bi + 1 < band_ranges.len() && band_ranges[bi + 1].0 == Band::Track;
            match band {
                Band::Platform => {
                    // ceiling lights
                    let mut x = 2.5;
                    while x < w as f32 - 1.0 {
                        let f = if hash2(x as i32, *y0, seed) < def.flicker { 1.0 } else { 0.0 };
                        lights.push(LightDef { pos: vec2(x, yc), radius: 3.4, color: def.lamp, flicker: f, phase: hash2(x as i32, 3, seed) * 10.0, strobe: false });
                        x += 4.5;
                    }
                    if above_track && below_track {
                        // island platform: pillar row with stuff between
                        let mut x = 3.5;
                        while x < w as f32 - 1.5 {
                            add_prop(PropKind::Pillar, vec2(x, yc), vec2(0.75, 0.75), true, &mut props, &mut obstacles);
                            let mx = x + 2.5;
                            if mx < w as f32 - 2.0 {
                                let r = hash2(mx as i32, *y0, seed + 1);
                                if r < 0.35 {
                                    let v = (hash2(mx as i32, 9, seed) * 3.0) as usize % 3;
                                    add_prop(PropKind::Vending(v), vec2(mx, yc), vec2(0.85, 0.55), true, &mut props, &mut obstacles);
                                    let lc = [rgb(255, 120, 120), rgb(120, 170, 255), rgb(255, 255, 240)][v];
                                    lights.push(LightDef { pos: vec2(mx, yc + 0.9), radius: 1.7, color: lc, flicker: 0.0, phase: 0.0, strobe: false });
                                } else if r < 0.75 {
                                    add_prop(PropKind::Bench, vec2(mx, yc + 0.9), vec2(1.6, 0.42), true, &mut props, &mut obstacles);
                                    add_prop(PropKind::Bench, vec2(mx, yc - 0.9), vec2(1.6, 0.42), true, &mut props, &mut obstacles);
                                } else if r < 0.85 {
                                    add_prop(PropKind::Bin, vec2(mx, yc), vec2(0.4, 0.4), true, &mut props, &mut obstacles);
                                }
                            }
                            x += 5.0;
                        }
                    } else if *y0 == 1 {
                        // against the back wall: vending machines and bins
                        let mut x = 3.0;
                        while x < w as f32 - 2.0 {
                            let r = hash2(x as i32, 41, seed);
                            if r < 0.4 {
                                let v = (hash2(x as i32, 7, seed) * 3.0) as usize % 3;
                                add_prop(PropKind::Vending(v), vec2(x, 1.45), vec2(0.85, 0.55), true, &mut props, &mut obstacles);
                                let lc = [rgb(255, 120, 120), rgb(120, 170, 255), rgb(255, 255, 240)][v];
                                lights.push(LightDef { pos: vec2(x, 2.3), radius: 1.7, color: lc, flicker: 0.0, phase: 0.0, strobe: false });
                            } else if r < 0.6 {
                                add_prop(PropKind::Bench, vec2(x, 1.5), vec2(1.6, 0.42), true, &mut props, &mut obstacles);
                            } else if r < 0.7 {
                                add_prop(PropKind::Bin, vec2(x, 1.4), vec2(0.4, 0.4), true, &mut props, &mut obstacles);
                            }
                            x += 3.0 + hash2(x as i32, 2, seed) * 2.0;
                        }
                    } else {
                        let mut x = 4.0;
                        while x < w as f32 - 2.0 {
                            add_prop(PropKind::Pillar, vec2(x, yc + 0.5), vec2(0.75, 0.75), true, &mut props, &mut obstacles);
                            x += 6.5;
                        }
                    }
                }
                Band::Concourse => {
                    let gy = y0f + 1.6;
                    let mut x = (w as f32 * 0.22).floor() + 0.5;
                    while x < w as f32 * 0.78 {
                        add_prop(PropKind::Gate, vec2(x, gy), vec2(0.3, 1.1), true, &mut props, &mut obstacles);
                        x += 1.3;
                    }
                    let mut x = 3.0;
                    while x < w as f32 - 1.0 {
                        lights.push(LightDef { pos: vec2(x, yc), radius: 3.6, color: shade(def.lamp, 0.95), flicker: 0.0, phase: 0.0, strobe: false });
                        x += 5.0;
                    }
                    let mut x = 6.0;
                    while x < w as f32 - 2.0 && !def.maze {
                        add_prop(PropKind::Pillar, vec2(x, y1f - 1.2), vec2(0.75, 0.75), true, &mut props, &mut obstacles);
                        x += 7.0;
                    }
                }
                Band::Track => {
                    // red tunnel lamps and a colored neon glow down the line
                    lights.push(LightDef { pos: vec2(1.5, yc - 1.4), radius: 1.2, color: rgb(255, 40, 30), flicker: 0.0, phase: k as f32, strobe: false });
                    lights.push(LightDef { pos: vec2(1.5, yc + 1.4), radius: 1.2, color: rgb(255, 40, 30), flicker: 0.0, phase: k as f32 + 1.0, strobe: false });
                    k += 1;
                }
            }
        }

        // ---- campaign underground: big halls off a central passage, like a subway concourse
        if def.maze {
            if let Some((_, c0, c1)) = band_ranges.iter().find(|b| b.0 == Band::Concourse) {
                let mut rng = Lcg(seed.wrapping_mul(97));
                let top = *c0 + 4; // room for the ticket gates
                let bottom = *c1;
                let span = bottom - top;
                // two rows of halls with a 4-tile passage between them
                let row1 = top + (span - 4) / 2;
                let hall_a = (top, row1);
                let hall_b = (row1 + 5, bottom);
                let wall = |tx: i32, ty: i32, props: &mut Vec<Prop>, obstacles: &mut Vec<Obstacle>| {
                    let kind = ((tx * 7 + ty * 13).rem_euclid(5) as usize).min(2);
                    props.push(Prop { kind: PropKind::MazeWall(kind), pos: vec2(tx as f32, ty as f32), size: vec2(1.0, 1.0) });
                    obstacles.push(Obstacle { min: vec2(tx as f32, ty as f32), max: vec2(tx as f32 + 1.0, ty as f32 + 1.0), low: false });
                };
                // the passage walls, with a wide opening into every hall
                let mut cuts = vec![1];
                let mut x = 1;
                while x < w - 6 {
                    x += 9 + (rng.f() * 6.0) as i32;
                    cuts.push(x.min(w - 1));
                }
                cuts.push(w);
                for (y_wall, hall) in [(row1, hall_a), (row1 + 4, hall_b)] {
                    for pair in cuts.windows(2) {
                        let (a, b) = (pair[0], pair[1]);
                        let door = a + (b - a) / 2 - 1 + (rng.f() * 3.0) as i32 - 1;
                        let open_wide = rng.f() < 0.2; // sometimes the hall just opens straight onto the passage
                        for tx in a..b {
                            let in_door = (door..door + 3).contains(&tx) || open_wide;
                            if !in_door && tx >= 1 && tx < w {
                                wall(tx, y_wall, &mut props, &mut obstacles);
                            }
                        }
                    }
                    // walls between the halls, each with a doorway
                    for &cx in &cuts[1..cuts.len() - 1] {
                        let (y0, y1) = (hall.0.min(hall.1), hall.0.max(hall.1));
                        let door = y0 + (y1 - y0) / 2 - 1;
                        for ty in y0..y1 {
                            if !(door..door + 3).contains(&ty) && ty != y_wall {
                                wall(cx, ty, &mut props, &mut obstacles);
                            }
                        }
                    }
                    // inside each hall: square columns in the big ones, lights, the odd kiosk
                    for pair in cuts.windows(2) {
                        let (a, b) = (pair[0] + 1, pair[1]);
                        let (y0, y1) = (hall.0.min(hall.1), hall.0.max(hall.1));
                        let wide = b - a >= 10;
                        if wide {
                            let mut px = a + 3;
                            while px < b - 2 {
                                let py = (y0 + y1) / 2;
                                if rng.f() < 0.8 {
                                    add_prop(PropKind::Pillar, vec2(px as f32 + 0.5, py as f32 + 0.5), vec2(0.75, 0.75), true, &mut props, &mut obstacles);
                                }
                                px += 4;
                            }
                        }
                        let c = def.neon[(rng.f() * def.neon.len() as f32) as usize % def.neon.len()];
                        let mid = vec2((a + b) as f32 * 0.5, (y0 + y1) as f32 * 0.5);
                        lights.push(LightDef { pos: mid + vec2(-2.0, -1.5), radius: 3.6, color: def.lamp, flicker: if rng.f() < def.flicker { 1.0 } else { 0.0 }, phase: rng.f() * 10.0, strobe: false });
                        lights.push(LightDef { pos: mid + vec2(2.0, 1.5), radius: 3.0, color: c, flicker: 0.0, phase: 0.0, strobe: false });
                    }
                }
                // lights down the passage
                let mut x = 3.0;
                while x < w as f32 {
                    lights.push(LightDef { pos: vec2(x, row1 as f32 + 2.5), radius: 2.8, color: def.lamp, flicker: if rng.f() < def.flicker { 1.0 } else { 0.0 }, phase: x, strobe: false });
                    x += 5.0;
                }
            }
        }

        // ---- platform screen doors along every platform edge, gaps where train doors stop
        let doors = if def.car { Vec::new() } else { door_xs(w) };
        let mut walls_h = vec![false; (gw * gh) as usize];
        let platform_row = |r: i32| r >= 1 && r < h && row_band[r as usize] != Band::Track;
        for (a, b) in &track_bands {
            for (edge_ok, panel_y, wall_row) in [(platform_row(a - 1), *a as f32 - 0.16, a - 1), (platform_row(*b), *b as f32, b - 1)] {
                if !edge_ok {
                    continue;
                }
                let mut run: Option<f32> = None;
                let mut x = 1.5;
                loop {
                    let end = x + 0.5 > w as f32 - 0.5;
                    let in_gap = doors.iter().any(|d| (x + 0.25 - d).abs() < 0.6);
                    if end || in_gap {
                        if let Some(x0) = run.take() {
                            obstacles.push(Obstacle { min: vec2(x0, panel_y), max: vec2(x, panel_y + 0.16), low: true });
                        }
                        if end {
                            break;
                        }
                    } else {
                        props.push(Prop { kind: PropKind::Psd, pos: vec2(x, panel_y), size: vec2(0.5, 0.16) });
                        run.get_or_insert(x);
                    }
                    x += 0.5;
                }
                for cx in 1..w {
                    let gap = doors.iter().any(|d| (cx as f32 + 0.5 - d).abs() < 0.1);
                    if !gap {
                        walls_h[(wall_row * gw + cx) as usize] = true;
                    }
                }
            }
        }

        // ---- hanging signs over the platforms: station name boards and LED departure boards
        let signs = build_signs(def.name, next);

        let mut k_sign = 0;
        for (band, y0, y1) in band_ranges.iter().filter(|_| !def.car) {
            if *band != Band::Platform || y1 - y0 < 5 {
                continue;
            }
            let yc = (*y0 + *y1) as f32 * 0.5 - 0.06;
            let mut x = 5.5 + (k_sign % 2) as f32 * 4.0;
            while x < w as f32 - 6.0 {
                let led = k_sign % 2 == 1;
                let (spr, kind) = if led { (signs.led[0], PropKind::SignLed) } else { (signs.name, PropKind::SignName) };
                let fx = (spr.r.w - spr.anchor.x) / HALF_W; // footprint length from the sprite
                props.push(Prop { kind, pos: vec2(x, yc), size: vec2(fx.max(1.0), 0.12) });
                k_sign += 1;
                x += 12.0;
            }
        }

        // ---- clutter: a kiosk, abandoned luggage, boxes, barriers
        let mut rng = Lcg(seed.wrapping_mul(2_654_435_761));
        let edge_dist_y = |y: f32| -> f32 {
            let mut d = 99.0f32;
            for (a, b) in &track_bands {
                let (a, b) = (*a as f32, *b as f32);
                if y < a { d = d.min(a - y); } else if y >= b { d = d.min(y - b); }
            }
            d
        };
        let free = |c: Vec2, half: Vec2, obstacles: &Vec<Obstacle>, props: &Vec<Prop>| -> bool {
            if c.x - half.x < 1.8 || c.x + half.x > w as f32 - 1.0 || c.y - half.y < 1.3 || c.y + half.y > h as f32 - 0.8 {
                return false;
            }
            let ty = c.y.floor() as i32;
            if ty < 1 || ty >= h || row_band[ty as usize] == Band::Track || edge_dist_y(c.y) < 0.9 {
                return false;
            }
            let m = 0.45;
            let hit_o = obstacles.iter().any(|o| c.x + half.x + m > o.min.x && c.x - half.x - m < o.max.x && c.y + half.y + m > o.min.y && c.y - half.y - m < o.max.y);
            let hit_p = props.iter().any(|p| c.x + half.x + m > p.pos.x && c.x - half.x - m < p.pos.x + p.size.x && c.y + half.y + m > p.pos.y && c.y - half.y - m < p.pos.y + p.size.y && !matches!(p.kind, PropKind::SignName | PropKind::SignLed));
            !hit_o && !hit_p
        };
        // the kiosk goes against the back wall, or on the concourse
        for _ in 0..(if def.car { 0 } else { 40 }) {
            let c = vec2(rng.range(4.0, w as f32 - 5.0), 1.85);
            let half = vec2(1.1, 0.5);
            if free(c, half, &obstacles, &props) {
                props.push(Prop { kind: PropKind::Kiosk, pos: c - half, size: half * 2.0 });
                obstacles.push(Obstacle { min: c - half, max: c + half, low: false });
                lights.push(LightDef { pos: c + vec2(0.0, 1.0), radius: 2.6, color: rgb(255, 236, 190), flicker: 0.0, phase: 0.0, strobe: false });
                break;
            }
        }
        let n_luggage = if def.car { 0 } else { 6 + index * 2 };
        let mut placed = 0;
        for _ in 0..200 {
            if placed >= n_luggage {
                break;
            }
            let c = vec2(rng.range(2.0, w as f32 - 1.5), rng.range(1.6, h as f32 - 1.0));
            let roll = rng.f();
            let (kind, half) = if roll < 0.55 {
                let v = (rng.f() * 4.0) as usize % 4;
                (PropKind::Suitcase(v), if v == 3 { vec2(0.225, 0.15) } else { vec2(0.15, 0.09) })
            } else if roll < 0.85 {
                (PropKind::Boxes, vec2(0.3, 0.25))
            } else {
                (PropKind::Barrier, vec2(0.5, 0.07))
            };
            if free(c, half, &obstacles, &props) {
                props.push(Prop { kind, pos: c - half, size: half * 2.0 });
                obstacles.push(Obstacle { min: c - half, max: c + half, low: true });
                placed += 1;
            }
        }

        // signs and ads on the back wall
        let mut wall_signs = Vec::new();
        let mut ads = Vec::new();
        let name_w = font::text_width(def.name) as f32 * 0.25 + 1.2;
        let mut u = 4.0;
        let mut toggle = 0;
        while u + name_w < w as f32 + 2.0 && !def.car {
            if toggle % 2 == 0 {
                wall_signs.push(Sign { u0: u, u1: u + name_w, text: def.name.to_string() });
                lights.push(LightDef { pos: vec2(u + name_w * 0.5, 1.7), radius: 2.4, color: rgb(255, 255, 245), flicker: 0.0, phase: 0.0, strobe: false });
                u += name_w + 2.0;
            } else {
                let aw = 2.4 + hash2(u as i32, 1, seed) * 1.6;
                let a = neon(k);
                let b = neon(k + 1);
                ads.push(Ad { u0: u, u1: u + aw, a, b, pattern: (hash2(u as i32, 5, seed) * 4.0) as u32 });
                lights.push(LightDef { pos: vec2(u + aw * 0.5, 1.8), radius: 2.2, color: a, flicker: if def.flicker > 0.4 { 1.0 } else { 0.0 }, phase: u, strobe: false });
                k += 1;
                u += aw + 1.5;
            }
            toggle += 1;
        }
        if def.boss {
            // Shinjuku runs on emergency strobes
            for i in 0..5 {
                let x = 5.0 + i as f32 * (w as f32 - 8.0) / 4.0;
                lights.push(LightDef { pos: vec2(x, h as f32 * 0.5), radius: 7.0, color: rgb(255, 30, 20), flicker: 0.0, phase: i as f32 * 1.3, strobe: true });
            }
        }

        // spawn points
        let tunnel_spawns = if def.car { Vec::new() } else { track_ys.iter().map(|y| vec2(1.2, *y)).collect() };
        let mut edge_spawns = Vec::new();
        let mut yy = 1.5;
        while yy < gh as f32 - 1.0 {
            edge_spawns.push(vec2(w as f32 + 3.0, yy));
            yy += 1.0;
        }
        let mut xx = 1.5;
        while xx < gw as f32 - 1.0 {
            edge_spawns.push(vec2(xx, h as f32 + 3.0));
            xx += 1.0;
        }

        // blocked cells for the flow field
        let mut blocked = vec![false; (gw * gh) as usize];
        for ty in 0..gh {
            for tx in 0..gw {
                let c = vec2(tx as f32 + 0.5, ty as f32 + 0.5);
                let mut b = tx < 1 || ty < 1;
                for o in &obstacles {
                    if c.x > o.min.x - 0.15 && c.x < o.max.x + 0.15 && c.y > o.min.y - 0.15 && c.y < o.max.y + 0.15 {
                        b = true;
                    }
                }
                blocked[(ty * gw + tx) as usize] = b;
            }
        }
        let mut obs_grid = vec![Vec::new(); (gw * gh) as usize];
        for (i, o) in obstacles.iter().enumerate() {
            let x0 = ((o.min.x - 1.0).floor() as i32).max(0);
            let x1 = ((o.max.x + 1.0).floor() as i32).min(gw - 1);
            let y0 = ((o.min.y - 1.0).floor() as i32).max(0);
            let y1 = ((o.max.y + 1.0).floor() as i32).min(gh - 1);
            for ty in y0..=y1 {
                for tx in x0..=x1 {
                    obs_grid[(ty * gw + tx) as usize].push(i as u16);
                }
            }
        }

        let (mut floor, floor_mask, origin) = bake_floor(def, w, h, gw, gh, &tiles, &track_bands, &wall_signs, &ads);
        if !def.car {
            paint_details(&mut floor, &floor_mask, origin, def, index, w, h, &tiles, gw, gh, &track_bands, &doors);
        }
        let floor_tex = Texture2D::from_image(&floor);
        floor_tex.set_filter(FilterMode::Nearest);

        Map {
            w,
            h,
            gw,
            gh,
            tiles,
            blocked,
            obstacles,
            obs_grid,
            props,
            lights,
            tunnel_spawns,
            edge_spawns,
            track_ys,
            signs,
            escalator,
            arcade,
            is_car: def.car,
            walls_h,
            floor,
            floor_mask,
            floor_tex,
            origin,
            dirty: None,
            flow: vec![FLOW_MAX; (gw * gh) as usize],
            flow_cell: (-1, -1),
        }
    }

    pub fn tile(&self, x: f32, y: f32) -> Tile {
        let (tx, ty) = (x.floor() as i32, y.floor() as i32);
        if tx < 1 || ty < 1 {
            return Tile::Wall;
        }
        if tx >= self.gw || ty >= self.gh {
            return Tile::Platform;
        }
        self.tiles[(ty * self.gw + tx) as usize]
    }

    /// Visual height offset: things standing on the tracks sit lower.
    pub fn ground_z(&self, p: Vec2) -> f32 {
        if self.tile(p.x, p.y) == Tile::Track { -TRACK_DROP } else { 0.0 }
    }

    /// Push a circle out of walls and obstacles.
    pub fn resolve(&self, p: &mut Vec2, r: f32, inner_only: bool) {
        self.resolve_ex(p, r, inner_only, false);
    }

    /// `smash_low`: big things (the boss) walk straight through waist-high props.
    pub fn resolve_ex(&self, p: &mut Vec2, r: f32, inner_only: bool, smash_low: bool) {
        let maxx = if inner_only { self.w as f32 - 0.4 } else { self.gw as f32 - r };
        let maxy = if inner_only { self.h as f32 - 0.4 } else { self.gh as f32 - r };
        p.x = p.x.clamp(1.0 + r, maxx);
        p.y = p.y.clamp(1.0 + r, maxy);
        let (tx, ty) = (p.x.floor() as i32, p.y.floor() as i32);
        if tx < 0 || ty < 0 || tx >= self.gw || ty >= self.gh {
            return;
        }
        for &i in &self.obs_grid[(ty * self.gw + tx) as usize] {
            let o = self.obstacles[i as usize];
            if smash_low && o.low {
                continue;
            }
            let c = vec2(p.x.clamp(o.min.x, o.max.x), p.y.clamp(o.min.y, o.max.y));
            let d = *p - c;
            let dist = d.length();
            if dist < r {
                if dist > 1e-4 {
                    *p += d / dist * (r - dist);
                } else {
                    // center is inside the box: push out along the shortest axis
                    let left = p.x - o.min.x;
                    let right = o.max.x - p.x;
                    let up = p.y - o.min.y;
                    let down = o.max.y - p.y;
                    let m = left.min(right).min(up).min(down);
                    if m == left {
                        p.x = o.min.x - r;
                    } else if m == right {
                        p.x = o.max.x + r;
                    } else if m == up {
                        p.y = o.min.y - r;
                    } else {
                        p.y = o.max.y + r;
                    }
                }
            }
        }
    }

    /// Does a point hit a wall or obstacle? (used by bullets)
    pub fn solid_at(&self, p: Vec2) -> bool {
        if p.x < 1.0 || p.y < 1.0 {
            return true;
        }
        let (tx, ty) = (p.x.floor() as i32, p.y.floor() as i32);
        if tx >= self.gw || ty >= self.gh {
            return false;
        }
        for &i in &self.obs_grid[(ty * self.gw + tx) as usize] {
            let o = self.obstacles[i as usize];
            if !o.low && p.x >= o.min.x && p.x <= o.max.x && p.y >= o.min.y && p.y <= o.max.y {
                return true;
            }
        }
        false
    }

    #[allow(dead_code)]
    pub fn blocked_at(&self, p: Vec2) -> bool {
        let (x, y) = (p.x.floor() as i32, p.y.floor() as i32);
        x < 0 || y < 0 || x >= self.gw || y >= self.gh || self.blocked[(y * self.gw + x) as usize]
    }

    /// Can you step from cell (x, y) by (dx, dy)? Platform screen doors block row crossings.
    fn crossing_ok(&self, x: i32, y: i32, dx: i32, dy: i32) -> bool {
        if dy == 0 {
            return true;
        }
        let row = if dy > 0 { y } else { y - 1 };
        if row < 0 || row >= self.gh {
            return true;
        }
        let wall = |cx: i32| cx >= 0 && cx < self.gw && self.walls_h[(row * self.gw + cx) as usize];
        !(wall(x) || (dx != 0 && wall(x + dx)))
    }

    // ------------------------------------------------------------ flow field

    pub fn update_flow(&mut self, target: Vec2) {
        let (mut cx, mut cy) = (target.x.floor() as i32, target.y.floor() as i32);
        cx = cx.clamp(1, self.gw - 1);
        cy = cy.clamp(1, self.gh - 1);
        if (cx, cy) == self.flow_cell {
            return;
        }
        self.flow_cell = (cx, cy);
        let gw = self.gw;
        self.flow.iter_mut().for_each(|d| *d = FLOW_MAX);
        let mut q = VecDeque::new();
        self.flow[(cy * gw + cx) as usize] = 0;
        q.push_back((cx, cy));
        while let Some((x, y)) = q.pop_front() {
            let d = self.flow[(y * gw + x) as usize];
            for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1), (1, 1), (1, -1), (-1, 1), (-1, -1)] {
                let (nx, ny) = (x + dx, y + dy);
                if nx < 0 || ny < 0 || nx >= gw || ny >= self.gh {
                    continue;
                }
                let ni = (ny * gw + nx) as usize;
                if self.blocked[ni] || self.flow[ni] != FLOW_MAX {
                    continue;
                }
                if dx != 0 && dy != 0 && (self.blocked[(y * gw + nx) as usize] || self.blocked[(ny * gw + x) as usize]) {
                    continue;
                }
                if !self.crossing_ok(x, y, dx, dy) {
                    continue;
                }
                self.flow[ni] = d + 1;
                q.push_back((nx, ny));
            }
        }
    }

    /// A separate distance field toward any target (used to walk NPCs somewhere).
    pub fn bfs(&self, target: Vec2) -> Vec<u16> {
        let gw = self.gw;
        let mut field = vec![FLOW_MAX; (gw * self.gh) as usize];
        let (cx, cy) = (target.x.floor().clamp(1.0, (gw - 1) as f32) as i32, target.y.floor().clamp(1.0, (self.gh - 1) as f32) as i32);
        let mut q = VecDeque::new();
        field[(cy * gw + cx) as usize] = 0;
        q.push_back((cx, cy));
        while let Some((x, y)) = q.pop_front() {
            let d = field[(y * gw + x) as usize];
            for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1), (1, 1), (1, -1), (-1, 1), (-1, -1)] {
                let (nx, ny) = (x + dx, y + dy);
                if nx < 0 || ny < 0 || nx >= gw || ny >= self.gh {
                    continue;
                }
                let ni = (ny * gw + nx) as usize;
                if self.blocked[ni] || field[ni] != FLOW_MAX || !self.crossing_ok(x, y, dx, dy) {
                    continue;
                }
                if dx != 0 && dy != 0 && (self.blocked[(y * gw + nx) as usize] || self.blocked[(ny * gw + x) as usize]) {
                    continue;
                }
                field[ni] = d + 1;
                q.push_back((nx, ny));
            }
        }
        field
    }

    pub fn dir_in(&self, field: &[u16], p: Vec2) -> Option<Vec2> {
        let (x, y) = (p.x.floor() as i32, p.y.floor() as i32);
        if x < 0 || y < 0 || x >= self.gw || y >= self.gh {
            return None;
        }
        let gw = self.gw;
        let cur = field[(y * gw + x) as usize];
        if cur == 0 || cur == FLOW_MAX {
            return None;
        }
        let mut best = cur;
        let mut dir = None;
        for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1), (1, 1), (1, -1), (-1, 1), (-1, -1)] {
            let (nx, ny) = (x + dx, y + dy);
            if nx < 0 || ny < 0 || nx >= gw || ny >= self.gh || !self.crossing_ok(x, y, dx, dy) {
                continue;
            }
            let d = field[(ny * gw + nx) as usize];
            if d < best {
                best = d;
                dir = Some(vec2(nx as f32 + 0.5, ny as f32 + 0.5));
            }
        }
        dir.map(|t| (t - p).normalize_or_zero())
    }

    pub fn flow_dir(&self, p: Vec2) -> Option<Vec2> {
        let (x, y) = (p.x.floor() as i32, p.y.floor() as i32);
        if x < 0 || y < 0 || x >= self.gw || y >= self.gh {
            return None;
        }
        let gw = self.gw;
        let cur = self.flow[(y * gw + x) as usize];
        if cur == 0 || cur == FLOW_MAX {
            return None;
        }
        let mut best = cur;
        let mut dir = None;
        for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1), (1, 1), (1, -1), (-1, 1), (-1, -1)] {
            let (nx, ny) = (x + dx, y + dy);
            if nx < 0 || ny < 0 || nx >= gw || ny >= self.gh {
                continue;
            }
            let d = self.flow[(ny * gw + nx) as usize];
            if d < best {
                if dx != 0 && dy != 0 && (self.blocked[(y * gw + nx) as usize] || self.blocked[(ny * gw + x) as usize]) {
                    continue;
                }
                if !self.crossing_ok(x, y, dx, dy) {
                    continue;
                }
                best = d;
                dir = Some(vec2(nx as f32 + 0.5, ny as f32 + 0.5));
            }
        }
        dir.map(|t| (t - p).normalize_or_zero())
    }

    // ------------------------------------------------------------ decals

    /// Paint a splat into the baked floor. `kind` 0 = blood, 1 = scorch, 2 = dark blood.
    pub fn stamp(&mut self, wp: Vec2, size: f32, kind: u32) {
        let mut c = iso(wp.x, wp.y) - self.origin;
        if self.tile(wp.x, wp.y) == Tile::Track {
            c.y += TRACK_DROP;
        }
        let rx = (size * 18.0).max(1.0);
        let ry = rx * 0.5;
        let blobs: Vec<(Vec2, f32)> = (0..(2 + (size * 4.0) as usize).min(7))
            .map(|i| {
                let off = if i == 0 { Vec2::ZERO } else { rand_dir() * rnd(0.2, 0.9) * rx };
                (vec2(off.x, off.y * 0.5), rnd(0.35, 0.8) * if i == 0 { 1.0 } else { 0.6 })
            })
            .collect();
        let x0 = (c.x - rx * 1.8).floor() as i32;
        let x1 = (c.x + rx * 1.8).ceil() as i32;
        let y0 = (c.y - ry * 1.8).floor() as i32;
        let y1 = (c.y + ry * 1.8).ceil() as i32;
        let (iw, ih) = (self.floor.width as i32, self.floor.height as i32);
        let (x0, x1, y0, y1) = (x0.max(0), x1.min(iw - 1), y0.max(0), y1.min(ih - 1));
        if x0 >= x1 || y0 >= y1 {
            return;
        }
        let s = rand::gen_range(0u32, 100000);
        for py in y0..=y1 {
            for px in x0..=x1 {
                if !self.floor_mask[(py * iw + px) as usize] {
                    continue;
                }
                let mut hit = false;
                for (o, r) in &blobs {
                    let d = vec2((px as f32 + 0.5 - c.x - o.x) / (rx * r), (py as f32 + 0.5 - c.y - o.y) / (ry * r));
                    if d.length_squared() < 1.0 - hash2(px, py, s) * 0.35 {
                        hit = true;
                        break;
                    }
                }
                if !hit {
                    continue;
                }
                let old = self.floor.get_pixel(px as u32, py as u32);
                let n = hash2(px, py, s + 1);
                let (col, a) = match kind {
                    0 => (Color::new(0.45 + n * 0.12, 0.03, 0.05, 1.0), 0.85),
                    2 => (Color::new(0.24, 0.02, 0.03, 1.0), 0.9),
                    _ => (Color::new(0.05 + n * 0.05, 0.045, 0.04, 1.0), 0.8),
                };
                self.floor.set_pixel(px as u32, py as u32, mix(old, col, a));
            }
        }
        self.dirty = Some(match self.dirty {
            None => (x0, y0, x1, y1),
            Some((a, b, cc, d)) => (a.min(x0), b.min(y0), cc.max(x1), d.max(y1)),
        });
    }

    /// Upload painted decals to the GPU once per frame.
    pub fn flush(&mut self) {
        if let Some((x0, y0, x1, y1)) = self.dirty.take() {
            let (w, h) = (x1 - x0 + 1, y1 - y0 + 1);
            let sub = self.floor.sub_image(Rect::new(x0 as f32, y0 as f32, w as f32, h as f32));
            self.floor_tex.update_part(&sub, x0, y0, w, h);
        }
    }
}

// ---------------------------------------------------------------- baking the floor

#[allow(clippy::too_many_arguments)]
fn bake_floor(
    def: &StationDef,
    w: i32,
    h: i32,
    gw: i32,
    gh: i32,
    tiles: &[Tile],
    tracks: &[(i32, i32)],
    signs: &[Sign],
    ads: &[Ad],
) -> (Image, Vec<bool>, Vec2) {
    let minx = -(gh as f32) * HALF_W;
    let maxx = gw as f32 * HALF_W;
    let miny = -WALL_H - 4.0;
    let maxy = (gw + gh) as f32 * HALF_H;
    let iw = (maxx - minx).ceil() as u32;
    let ih = (maxy - miny).ceil() as u32;
    let mut img = Image::gen_image_color(iw as u16, ih as u16, BLACK);
    let mut mask = vec![false; (iw * ih) as usize];

    let tile_at = |x: f32, y: f32| -> Tile {
        let (tx, ty) = (x.floor() as i32, y.floor() as i32);
        if tx < 1 || ty < 1 {
            return Tile::Wall;
        }
        if tx >= gw || ty >= gh {
            return Tile::Platform;
        }
        tiles[(ty * gw + tx) as usize]
    };
    let edge_dist = |y: f32| -> f32 {
        let mut d = 99.0f32;
        for (a, b) in tracks {
            let (a, b) = (*a as f32, *b as f32);
            if y < a {
                d = d.min(a - y);
            } else if y >= b {
                d = d.min(y - b);
            }
        }
        d
    };
    let track_center = |y: f32| -> f32 {
        for (a, b) in tracks {
            if y >= *a as f32 && y < *b as f32 {
                return (*a + *b) as f32 * 0.5;
            }
        }
        y
    };
    let fract = |v: f32| v - v.floor();

    for iy in 0..ih {
        for ix in 0..iw {
            let sx = ix as f32 + 0.5 + minx;
            let sy = iy as f32 + 0.5 + miny;
            let a = sx / HALF_W;
            let b = sy / HALF_H;
            let x = (a + b) * 0.5;
            let y = (b - a) * 0.5;
            let n = hash2(ix as i32, iy as i32, 3);
            let mut col: Option<Color> = None;
            let mut is_floor = false;

            if x < 1.0 || y < 1.0 {
                // ---- walls
                let xr = sx / HALF_W + 1.0;
                let zr = (xr + 1.0) * HALF_H - sy;
                let yc = 1.0 - sx / HALF_W;
                let zc = (1.0 + yc) * HALF_H - sy;
                if xr >= 1.0 && xr <= gw as f32 && (0.0..=WALL_H).contains(&zr) {
                    col = Some(if def.car { car_wall(xr, zr, n, w) } else { wall_row(def, xr, zr, signs, ads, n) });
                } else if yc >= 1.0 && yc <= gh as f32 && (0.0..=WALL_H).contains(&zc) {
                    col = Some(if def.car { car_end(yc, zc, n) } else { wall_col(def, yc, zc, tracks, n) });
                } else {
                    let b2 = (sy + WALL_H) / HALF_H;
                    let xt = (a + b2) * 0.5;
                    let yt = (b2 - a) * 0.5;
                    if ((0.0..=1.0).contains(&yt) && xt >= 0.0 && xt <= gw as f32)
                        || ((0.0..=1.0).contains(&xt) && yt >= 0.0 && yt <= gh as f32)
                    {
                        col = Some(rgb(16, 15, 18));
                    }
                }
                // fade the far ends of the walls into darkness too
                if let Some(c) = col {
                    let along = if xr >= 1.0 { xr - w as f32 } else { yc - h as f32 };
                    let k = 1.0 - smoothstep(0.0, 4.5, along);
                    col = Some(shade(c, k.max(0.0)));
                }
            } else if x <= gw as f32 && y <= gh as f32 {
                is_floor = true;
                let t = tile_at(x, y);
                let c = match t {
                    Tile::Track => {
                        let (lx, ly) = (x - TRACK_DROP / HALF_W, y - TRACK_DROP / HALF_W);
                        let t2 = tile_at(lx, ly);
                        if t2 == Tile::Wall {
                            rgb(10, 8, 10)
                        } else if t2 != Tile::Track {
                            // the platform's edge face
                            let depth = (ly.ceil() - ly).clamp(0.0, 1.0);
                            shade(rgb(92, 90, 88), 0.55 + depth * 0.35 + n * 0.08)
                        } else {
                            let local = ly - track_center(ly);
                            let al = local.abs();
                            if (0.5..0.62).contains(&al) {
                                if al < 0.545 { rgb(200, 204, 212) } else { rgb(110, 112, 120) }
                            } else if fract(lx * 2.0) < 0.26 && al < 0.85 {
                                shade(rgb(96, 84, 72), 0.8 + n * 0.3)
                            } else {
                                shade(rgb(52, 48, 46), 0.65 + n * 0.55)
                            }
                        }
                    }
                    Tile::Platform | Tile::Wall => {
                        let d = edge_dist(y);
                        let base = def.floor;
                        if d < 0.09 {
                            rgb(226, 224, 214)
                        } else if (0.3..0.62).contains(&d) {
                            let bump = (fract(x * 8.0) - 0.5).powi(2) + (fract(y * 8.0) - 0.5).powi(2) < 0.09;
                            if bump { rgb(252, 214, 70) } else { rgb(222, 172, 34) }
                        } else {
                            let grout = fract(x * 2.0) < 0.05 || fract(y * 2.0) < 0.05;
                            let tile_var = hash2((x * 2.0) as i32, (y * 2.0) as i32, 9) * 0.08;
                            shade(base, if grout { 0.8 } else { 0.94 + tile_var + n * 0.04 })
                        }
                    }
                    Tile::Concourse if def.car => {
                        // speckled linoleum with yellow lines marking the door areas
                        let near_door = CAR_DOORS.iter().any(|d| (x - d).abs() < 0.75);
                        let edge = y < 1.25 || y > h as f32 - 0.25;
                        if near_door && (y < 1.6 || y > h as f32 - 0.6) && (fract(x * 4.0) < 0.12) {
                            rgb(220, 180, 40)
                        } else if near_door && edge {
                            shade(def.floor, 0.8)
                        } else {
                            shade(def.floor, 0.92 + n * 0.12 + if hash2(ix as i32 / 2, iy as i32 / 2, 4) > 0.93 { 0.12 } else { 0.0 })
                        }
                    }
                    Tile::Concourse => {
                        let checker = ((x.floor() + y.floor()) as i32) % 2 == 0;
                        let grout = fract(x) < 0.04 || fract(y) < 0.04;
                        let base = if checker { rgb(158, 146, 122) } else { rgb(136, 126, 106) };
                        shade(base, if grout { 0.75 } else { 0.95 + n * 0.06 })
                    }
                };
                let out = (x - w as f32).max(y - h as f32);
                let k = 1.0 - smoothstep(0.0, 4.5, out);
                col = Some(shade(c, k.max(0.0)));
                if k < 0.05 {
                    is_floor = false;
                }
            }
            if let Some(c) = col {
                img.set_pixel(ix, iy, c);
            }
            mask[(iy * iw + ix) as usize] = is_floor;
        }
    }
    (img, mask, vec2(minx, miny))
}

/// The long side of the train car: windows, doors, LCD screens, ads above the windows.
fn car_wall(u: f32, z: f32, n: f32, w: i32) -> Color {
    if u > w as f32 {
        return rgb(8, 8, 10);
    }
    let silver = rgb(206, 210, 216);
    let door = CAR_DOORS.iter().find(|d| (u - *d).abs() < 0.7);
    if z < 3.0 {
        return rgb(60, 62, 68);
    }
    if z > 62.0 {
        return if z < 64.0 { rgb(150, 154, 160) } else { rgb(232, 232, 228) }; // ceiling
    }
    if let Some(d) = door {
        let du = u - d;
        if (50.0..57.0).contains(&z) && du.abs() < 0.55 {
            // LCD screen over the door
            let lit = ((u * 16.0) as i32 + z as i32) % 3 != 0 && (52.0..55.0).contains(&z);
            return if lit { rgb(255, 160, 40) } else { rgb(14, 16, 20) };
        }
        if z < 48.0 {
            if du.abs() < 0.03 {
                return rgb(30, 30, 34);
            }
            if (26.0..44.0).contains(&z) && du.abs() > 0.12 && du.abs() < 0.5 {
                return if hash2((u * 16.0) as i32, z as i32, 3) > 0.97 { rgb(120, 140, 170) } else { rgb(18, 24, 34) };
            }
            return shade(rgb(184, 190, 198), 0.95 + n * 0.05);
        }
    }
    if (26.0..46.0).contains(&z) {
        let frame = (z < 27.5) || (z > 44.5) || ((u * 16.0) as i32 % 26 == 0);
        if frame {
            return rgb(150, 154, 162);
        }
        return if hash2((u * 16.0) as i32, z as i32, 6) > 0.985 { rgb(110, 130, 160) } else { rgb(14, 18, 28) };
    }
    if (48.0..60.0).contains(&z) {
        let pal = [rgb(240, 120, 90), rgb(90, 170, 230), rgb(250, 220, 90), rgb(150, 210, 120)];
        let k = ((u / 1.3) as usize) % 4;
        return if ((u * 16.0) as i32) % 21 < 2 { silver } else { shade(pal[k], 0.9) };
    }
    if (20.0..22.0).contains(&z) {
        return rgb(110, 178, 46);
    }
    shade(silver, 0.92 + n * 0.06)
}

fn car_end(u: f32, z: f32, n: f32) -> Color {
    if (2.6..4.4).contains(&u) && z < 50.0 {
        if (24.0..44.0).contains(&z) && (2.9..4.1).contains(&u) {
            return rgb(20, 26, 36); // window into the next car
        }
        return rgb(176, 182, 190);
    }
    if z > 62.0 {
        return rgb(232, 232, 228);
    }
    shade(rgb(196, 200, 206), 0.9 + n * 0.06)
}

fn wall_row(def: &StationDef, u: f32, z: f32, signs: &[Sign], ads: &[Ad], n: f32) -> Color {
    let green = rgb(110, 178, 46);
    if z < 4.0 {
        return rgb(46, 46, 52);
    }
    if z > 62.0 {
        return if z < 63.5 { rgb(80, 80, 88) } else { rgb(34, 34, 40) };
    }
    if (50.0..54.0).contains(&z) {
        return if z > 53.0 { shade(green, 1.2) } else { green };
    }
    for s in signs {
        if u >= s.u0 && u <= s.u1 && (16.0..38.0).contains(&z) {
            let frame = z < 17.5 || z > 36.5 || u < s.u0 + 0.12 || u > s.u1 - 0.12;
            if frame {
                return rgb(30, 30, 36);
            }
            if z > 33.0 {
                return green;
            }
            if z < 19.0 {
                return shade(green, 0.85);
            }
            let fx = ((u - s.u0 - 0.6) / 0.25).floor();
            let fy = ((32.5 - z) / 2.0).floor();
            if fx >= 0.0 && fy >= 0.0 {
                let fx = fx as usize;
                let ci = fx / (font::GW + 1);
                let cx = fx % (font::GW + 1);
                if let Some(ch) = s.text.chars().nth(ci) {
                    if font::bit(ch, cx, fy as usize) {
                        return rgb(20, 20, 26);
                    }
                }
            }
            return rgb(244, 244, 236);
        }
    }
    for ad in ads {
        if u >= ad.u0 && u <= ad.u1 && (14.0..40.0).contains(&z) {
            let frame = z < 15.0 || z > 39.0 || u < ad.u0 + 0.1 || u > ad.u1 - 0.1;
            if frame {
                return rgb(20, 20, 24);
            }
            let tu = (u - ad.u0) / (ad.u1 - ad.u0);
            let tz = (z - 14.0) / 26.0;
            let base = mix(ad.a, ad.b, tz);
            let shape = match ad.pattern {
                0 => (vec2(tu - 0.5, (tz - 0.55) * 0.6)).length() < 0.22,
                1 => ((tu * 6.0 + tz * 4.0) as i32) % 2 == 0,
                2 => (0.2..0.35).contains(&tz) || (0.6..0.7).contains(&tz),
                _ => tu > 0.15 && tu < 0.85 && (0.45..0.85).contains(&tz) && ((tu * 10.0) as i32 % 2 == 0),
            };
            return if shape { shade(base, 1.35) } else { shade(base, 0.75) };
        }
    }
    let ui = (u * 16.0) as i32;
    let grout = ui % 4 == 0 || (z as i32) % 5 == 0;
    shade(def.wall, if grout { 0.72 } else { 0.86 + n * 0.06 })
}

fn wall_col(def: &StationDef, u: f32, z: f32, tracks: &[(i32, i32)], n: f32) -> Color {
    for (a, b) in tracks {
        let cy = (*a + *b) as f32 * 0.5;
        let du = (u - cy).abs();
        let hw = 1.45;
        if du < hw + 0.25 {
            let top = 30.0 + 8.0 * (1.0 - (du / (hw + 0.25)).powi(2)).max(0.0).sqrt();
            if du < hw && z < top - 2.0 {
                // tunnel mouth: darkness with a few distant lamps
                let far = hash2((u * 16.0) as i32, z as i32, 8) > 0.996;
                return if far { rgb(120, 30, 20) } else { rgb(4, 3, 5) };
            }
            if z < top + 1.5 {
                return shade(rgb(84, 82, 86), 0.8 + n * 0.1);
            }
        }
    }
    let shaded = shade(def.wall, 0.72);
    if z < 4.0 {
        return rgb(40, 40, 46);
    }
    if z > 62.0 {
        return if z < 63.5 { rgb(70, 70, 78) } else { rgb(28, 28, 34) };
    }
    if (50.0..54.0).contains(&z) {
        return shade(rgb(110, 178, 46), 0.8);
    }
    let ui = (u * 16.0) as i32;
    let grout = ui % 4 == 0 || (z as i32) % 5 == 0;
    shade(shaded, if grout { 0.78 } else { 0.95 + n * 0.06 })
}


// ---------------------------------------------------------------- painted floor details

/// Paint lived-in detail into the baked floor: car-position markers at every door
/// gap, track numbers, puddles, litter, cracks, old blood trails, dropped clear
/// umbrellas, and grime along the walls. Deterministic per station.
#[allow(clippy::too_many_arguments)]
fn paint_details(img: &mut Image, mask: &[bool], origin: Vec2, def: &StationDef, index: usize, w: i32, h: i32, tiles: &[Tile], gw: i32, gh: i32, tracks: &[(i32, i32)], doors: &[f32]) {
    let iw = img.width as i32;
    let ih = img.height as i32;
    let tile_at = |x: f32, y: f32| -> Tile {
        let (tx, ty) = (x.floor() as i32, y.floor() as i32);
        if tx < 1 || ty < 1 {
            return Tile::Wall;
        }
        if tx >= gw || ty >= gh {
            return Tile::Platform;
        }
        tiles[(ty * gw + tx) as usize]
    };
    // Paint every floor pixel whose world position falls inside [x0,x1]x[y0,y1].
    let paint = |img: &mut Image, x0: f32, y0: f32, x1: f32, y1: f32, f: &mut dyn FnMut(f32, f32, Color, i32, i32) -> Option<Color>| {
        let cs = [iso(x0, y0), iso(x1, y0), iso(x0, y1), iso(x1, y1)];
        let minx = (cs.iter().map(|c| c.x).fold(f32::MAX, f32::min) - origin.x).floor().max(0.0) as i32;
        let maxx = (cs.iter().map(|c| c.x).fold(f32::MIN, f32::max) - origin.x).ceil().min(iw as f32 - 1.0) as i32;
        let miny = (cs.iter().map(|c| c.y).fold(f32::MAX, f32::min) - origin.y).floor().max(0.0) as i32;
        let maxy = (cs.iter().map(|c| c.y).fold(f32::MIN, f32::max) - origin.y).ceil().min(ih as f32 - 1.0) as i32;
        for py in miny..=maxy {
            for px in minx..=maxx {
                if !mask[(py * iw + px) as usize] {
                    continue;
                }
                let sx = px as f32 + 0.5 + origin.x;
                let sy = py as f32 + 0.5 + origin.y;
                let (a, b) = (sx / HALF_W, sy / HALF_H);
                let (wx, wy) = ((a + b) * 0.5, (b - a) * 0.5);
                if wx < x0 || wx > x1 || wy < y0 || wy > y1 || wx > w as f32 - 0.1 || wy > h as f32 - 0.1 {
                    continue;
                }
                let t = tile_at(wx, wy);
                if t != Tile::Platform && t != Tile::Concourse {
                    continue;
                }
                let old = img.get_pixel(px as u32, py as u32);
                if let Some(c) = f(wx, wy, old, px, py) {
                    img.set_pixel(px as u32, py as u32, c);
                }
            }
        }
    };
    let platform_row = |r: i32| r >= 1 && r < h && tile_at(2.5, r as f32 + 0.5) != Tile::Track;
    let mut rng = Lcg(0x9e37_79b9 ^ (index as u32 * 7919));

    // grime creeping out from the walls
    for (x0, y0, x1, y1) in [(1.0, 1.0, w as f32, 2.6), (1.0, 1.0, 2.6, h as f32)] {
        paint(img, x0, y0, x1, y1, &mut |wx, wy, old, px, py| {
            let d = (wx - 1.0).min(wy - 1.0);
            let k = 0.72 + 0.28 * smoothstep(0.0, 1.6, d) + hash2(px / 3, py / 3, 5) * 0.06;
            Some(shade(old, k.min(1.0)))
        });
    }

    // car-position markers at each door gap, numbered like the real ones
    for (a, b) in tracks {
        for (k, d) in doors.iter().enumerate() {
            for (ok, yy) in [(platform_row(a - 1), *a as f32 - 1.25), (platform_row(*b), *b as f32 + 0.75)] {
                if !ok {
                    continue;
                }
                let x0 = d - 0.28;
                let digit = std::char::from_digit(((doors.len() - k) % 10) as u32, 10).unwrap_or('0');
                paint(img, x0, yy, x0 + 0.56, yy + 0.5, &mut |wx, wy, _old, _px, _py| {
                    let u = ((wx - x0) * 16.0) as i32;
                    let v = ((wy - yy) * 16.0) as i32;
                    let border = u == 0 || v == 0 || u >= 8 || v >= 7;
                    if border {
                        Some(rgb(236, 236, 228))
                    } else if (2..7).contains(&u) && font::bit(digit, (u - 2) as usize, (v.clamp(0, 6)) as usize) && v < 7 {
                        Some(rgb(250, 250, 244))
                    } else {
                        Some(rgb(54, 150, 78))
                    }
                });
                // queue lanes either side of the marker
                for side in [-0.75f32, 0.55] {
                    let lx = d + side;
                    paint(img, lx, yy - 0.3, lx + 0.2, yy + 0.8, &mut |wx, _wy, old, _px, _py| {
                        if ((wx - lx) * 16.0) as i32 % 3 == 0 { Some(mix(old, rgb(230, 230, 222), 0.7)) } else { None }
                    });
                }
            }
        }
    }

    // big painted track numbers at the west end of each platform edge
    for (t, (a, b)) in tracks.iter().enumerate() {
        let ch = std::char::from_digit(((t + 1) % 10) as u32, 10).unwrap_or('1');
        for (ok, y0) in [(platform_row(a - 1) && a - 1 > 2, *a as f32 - 3.1), (platform_row(*b) && b + 3 < h, *b as f32 + 1.35)] {
            if !ok {
                continue;
            }
            let x0 = 2.4;
            paint(img, x0, y0, x0 + 1.25, y0 + 1.75, &mut |wx, wy, old, px, py| {
                let gx = ((wx - x0) / 0.25) as usize;
                let gy = ((wy - y0) / 0.25) as usize;
                if font::bit(ch, gx, gy) && hash2(px, py, 77) > 0.12 { Some(mix(old, rgb(246, 206, 60), 0.85)) } else { None }
            });
        }
    }

    // puddles from a leaking roof, with glints
    for _ in 0..(8 + index * 2) {
        let c = vec2(rng.range(2.5, w as f32 - 1.5), rng.range(1.8, h as f32 - 1.0));
        let (rx, ry) = (rng.range(0.5, 1.3), rng.range(0.35, 0.9));
        let ph = rng.range(0.0, 6.0);
        paint(img, c.x - rx, c.y - ry, c.x + rx, c.y + ry, &mut |wx, wy, old, px, py| {
            let (dx, dy) = ((wx - c.x) / rx, (wy - c.y) / ry);
            let d = dx * dx + dy * dy + 0.22 * (dx * 5.0 + ph).sin() * (dy * 4.0).cos();
            if d >= 1.0 {
                return None;
            }
            let wet = mix(old, rgb(28, 34, 46), 0.55);
            if hash2(px, py, 21) > 0.985 {
                Some(mix(wet, rgb(190, 200, 225), 0.7))
            } else if d > 0.82 {
                Some(shade(wet, 0.85))
            } else {
                Some(wet)
            }
        });
    }

    // litter: papers, flyers and newspapers
    for _ in 0..(40 + index * 10) {
        let c = vec2(rng.range(2.0, w as f32 - 1.0), rng.range(1.6, h as f32 - 0.8));
        let news = rng.f() < 0.35;
        let (hw, hh): (f32, f32) = if news { (0.28, 0.2) } else { (0.14, 0.1) };
        let ang = rng.range(0.0, std::f32::consts::PI);
        let (ca, sa) = (ang.cos(), ang.sin());
        let tint = if rng.f() < 0.2 { rgb(240, 210, 120) } else { rgb(226, 224, 214) };
        let r = hw.max(hh) * 1.5;
        paint(img, c.x - r, c.y - r, c.x + r, c.y + r, &mut |wx, wy, _old, _px, _py| {
            let (dx, dy) = (wx - c.x, wy - c.y);
            let (lx, ly) = (dx * ca + dy * sa, -dx * sa + dy * ca);
            if lx.abs() > hw || ly.abs() > hh {
                return None;
            }
            let line = if news { ((ly + hh) * 40.0) as i32 % 2 == 0 && lx.abs() < hw - 0.03 } else { ((ly + hh) * 30.0) as i32 % 3 == 0 && lx.abs() < hw - 0.04 };
            Some(if line { rgb(120, 118, 112) } else { tint })
        });
    }

    // cracks
    for _ in 0..(12 + index * 3) {
        let mut p0 = vec2(rng.range(2.0, w as f32 - 1.0), rng.range(1.6, h as f32 - 0.8));
        let mut ang = rng.range(0.0, std::f32::consts::TAU);
        for _ in 0..rng.range(4.0, 10.0) as usize {
            ang += rng.range(-0.8, 0.8);
            let p1 = p0 + vec2(ang.cos(), ang.sin()) * rng.range(0.25, 0.55);
            let (lo, hi) = (p0.min(p1) - 0.05, p0.max(p1) + 0.05);
            let seg = p1 - p0;
            paint(img, lo.x, lo.y, hi.x, hi.y, &mut |wx, wy, old, _px, _py| {
                let q = vec2(wx, wy) - p0;
                let t = (q.dot(seg) / seg.length_squared()).clamp(0.0, 1.0);
                if (q - seg * t).length() < 0.03 { Some(shade(old, 0.5)) } else { None }
            });
            p0 = p1;
        }
    }

    // old blood: pools with drag trails leading away
    for _ in 0..(3 + index) {
        let start = vec2(rng.range(3.0, w as f32 - 2.0), rng.range(2.0, h as f32 - 1.5));
        let dir = vec2(rng.range(-1.0, 1.0), rng.range(-1.0, 1.0)).normalize_or_zero();
        let len = rng.range(1.5, 3.5);
        let mut blobs = vec![(start, 0.55)];
        let mut t = 0.3;
        while t < len {
            blobs.push((start + dir * t + vec2(dir.y, -dir.x) * (t * 3.0).sin() * 0.06, 0.2 * (1.0 - t / len) + 0.06));
            t += 0.12;
        }
        for (c, r) in blobs {
            paint(img, c.x - r, c.y - r, c.x + r, c.y + r, &mut |wx, wy, old, px, py| {
                let d = vec2(wx - c.x, wy - c.y).length() / r;
                if d < 1.0 - hash2(px, py, 9) * 0.3 { Some(mix(old, rgb(70, 16, 12), 0.75)) } else { None }
            });
        }
    }

    // dropped clear vinyl umbrellas, the most Tokyo object there is
    for _ in 0..(4 + index) {
        let c = vec2(rng.range(2.5, w as f32 - 1.5), rng.range(1.8, h as f32 - 1.0));
        let ang = rng.range(0.0, std::f32::consts::TAU);
        let hd = vec2(ang.cos(), ang.sin());
        paint(img, c.x - 0.9, c.y - 0.9, c.x + 0.9, c.y + 0.9, &mut |wx, wy, old, _px, _py| {
            let q = vec2(wx - c.x, wy - c.y);
            let d = q.length();
            if d < 0.42 {
                let a = q.y.atan2(q.x);
                let rib = ((a / std::f32::consts::TAU * 8.0).fract() < 0.08) || d > 0.39;
                Some(if rib { mix(old, rgb(235, 240, 245), 0.75) } else { mix(old, rgb(220, 232, 240), 0.32) })
            } else {
                // the handle
                let t = q.dot(hd);
                let off = (q - hd * t).length();
                if t > 0.3 && t < 0.85 && off < 0.03 { Some(rgb(40, 40, 44)) } else { None }
            }
        });
    }
    let _ = def;
}
