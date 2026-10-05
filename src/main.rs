// No console window behind the game on Windows release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

//! LAST TRAIN TO SHINJUKU
//! An isometric twin-stick zombie shooter. Clear each station on the Yamanote line,
//! catch the train, pick an upgrade, and fight your way to Shinjuku.
//!
//! Everything you see is generated in code: sprites (art.rs), the font (font.rs),
//! the stations (level.rs), and the lighting/post shaders (render.rs).

mod arcade;
mod art;
mod audio;
mod font;
mod hud;
mod input;
mod intro;
mod level;
mod pad;
mod render;
mod synth;
mod util;
mod world;

use art::Art;
use input::Mode;
use audio::Audio;
use synth::Id;
use level::{stations, StationDef};
use macroquad::prelude::*;
use util::*;
use world::*;

#[derive(Clone, Copy, PartialEq)]
enum GameMode {
    Campaign,
    Arcade,
}

/// What carries from one campaign level to the next.
#[derive(Clone, Copy, Default)]
struct Camp {
    level: u8, // 1 Kanda, 2 Akihabara
    has_light: bool,
    light_on: bool,
    coins: u32,
}

enum Scene {
    Title,
    Intro,
    Ride,
    Play,
    Upgrade { choices: [usize; 3] },
    GameOver,
    Victory,
}

struct Game {
    art: Art,
    audio: Audio,
    defs: Vec<StationDef>,
    fx: render::Fx,
    input: input::Input,
    world: World,
    scene: Scene,
    scene_t: f32,
    stats: Stats,
    stats_at_start: Stats,
    kills_at_start: u32,
    run_time: f32,
    fade: f32,
    paused: bool,
    up_sel: usize,
    toast: Option<(String, f32)>,
    intro: Option<intro::Intro>,
    arcade: Option<arcade::Arcade>,
    arcade_hi: u32,
    game_mode: GameMode,
    menu_sel: usize,
    camp: Camp,
}

impl Game {
    fn new(audio: Audio) -> Game {
        let art = Art::build();
        let defs = stations();
        let world = Self::demo_world(&defs, &art);
        Game {
            fx: render::Fx::new(),
            input: input::Input::new(),
            scene: Scene::Title,
            scene_t: 0.0,
            stats: Stats::default(),
            stats_at_start: Stats::default(),
            kills_at_start: 0,
            run_time: 0.0,
            fade: 1.0,
            paused: false,
            up_sel: 0,
            toast: None,
            intro: None,
            arcade: None,
            arcade_hi: 0,
            game_mode: GameMode::Campaign,
            menu_sel: 0,
            camp: Camp::default(),
            world,
            art,
            audio,
            defs,
        }
    }

    fn demo_world(defs: &[StationDef], art: &Art) -> World {
        let mut w = World::new(0, defs, art, Stats::default(), 0, true);
        for _ in 0..40 {
            let p = vec2(rnd(3.0, w.map.w as f32 - 2.0), rnd(3.0, w.map.h as f32 - 2.0));
            let kind = if chance(0.2) { ZKind::Runner } else { ZKind::Walker };
            w.add_zombie(kind, p);
        }
        w
    }

    /// Controller rumble keyed off the same cues the audio uses.
    fn rumble_for_events(&mut self) {
        let pads = &mut self.input.pads;
        for e in &self.world.sfx {
            match *e {
                Sfx::Shot => pads.rumble(0.0, 0.12, 45),
                Sfx::Dash => pads.rumble(0.0, 0.25, 90),
                Sfx::Hurt => pads.rumble(0.7, 0.5, 220),
                Sfx::Explode(_) => pads.rumble(0.85, 0.6, 320),
                Sfx::Slam(_) => pads.rumble(1.0, 0.9, 500),
                Sfx::Roar => pads.rumble(0.35, 0.2, 700),
                Sfx::Train => pads.rumble(0.3, 0.1, 1200),
                Sfx::Splat(_, ZKind::Brute) => pads.rumble(0.5, 0.3, 160),
                Sfx::Splat(_, ZKind::Boss) => pads.rumble(1.0, 1.0, 900),
                _ => {}
            }
        }
    }

    fn start_station(&mut self, station: usize) {
        self.stats_at_start = self.stats;
        self.kills_at_start = self.world.total_kills;
        self.world = World::new(station, &self.defs, &self.art, self.stats, self.kills_at_start, false);
        self.scene = Scene::Play;
        self.scene_t = 0.0;
        self.fade = 1.0;
        self.paused = false;
        self.input.release_all();
    }

    #[allow(dead_code)]
    fn new_run(&mut self) {
        self.stats = Stats::default();
        self.world.total_kills = 0;
        self.run_time = 0.0;
        self.start_station(0);
    }

    /// Arcade: the original run, Akihabara to Shinjuku with upgrade cards.
    fn start_arcade(&mut self) {
        self.game_mode = GameMode::Arcade;
        self.stats = Stats::arcade();
        self.world.total_kills = 0;
        self.run_time = 0.0;
        self.start_station(0);
    }

    fn save_camp(&mut self) {
        self.camp.has_light = self.world.has_light;
        self.camp.light_on = self.world.light_on;
        self.camp.coins = self.world.coins;
    }

    /// Campaign, Kanda: hold out until a train comes (after the opening, or on retry).
    fn start_kanda(&mut self) {
        self.camp.level = 1;
        self.stats_at_start = self.stats;
        self.kills_at_start = self.world.total_kills;
        let mut w = World::from_def(&level::kanda_def(), "FOR AKIHABARA", 0, &self.art, self.stats, self.kills_at_start, false);
        w.story = true;
        w.has_light = self.camp.has_light;
        w.light_on = self.camp.light_on;
        w.coins = self.camp.coins;
        w.birds.clear();
        w.say("JY02  KANDA", "HOLD OUT UNTIL A TRAIN COMES.", 4.0);
        self.world = w;
        self.scene = Scene::Play;
        self.scene_t = 0.0;
        self.fade = 1.0;
        self.paused = false;
        self.input.release_all();
    }

    /// Campaign, Akihabara: the station opened up. Explore, kill, loot.
    fn start_akiba(&mut self) {
        self.camp.level = 2;
        self.stats_at_start = self.stats;
        self.kills_at_start = self.world.total_kills;
        let mut w = World::from_def(&level::akiba_campaign_def(), "OUT OF SERVICE", 1, &self.art, self.stats, self.kills_at_start, false);
        w.story = true;
        w.campaign = true;
        w.has_light = self.camp.has_light;
        w.light_on = self.camp.light_on;
        w.coins = self.camp.coins;
        w.populate_idle(55);
        w.scatter_bodies(30);
        w.say("JY03  AKIHABARA", "THE WHOLE STATION IS OPEN. SEARCH THE DEAD FOR COINS.", 5.0);
        self.world = w;
        self.scene = Scene::Play;
        self.scene_t = 0.0;
        self.fade = 1.0;
        self.paused = false;
        self.input.release_all();
    }

    /// A new game starts with the opening on the train.
    fn start_intro(&mut self) {
        self.game_mode = GameMode::Campaign;
        self.camp = Camp::default();
        self.stats = Stats::default();
        self.stats_at_start = self.stats;
        self.kills_at_start = 0;
        self.run_time = 0.0;
        self.world = intro::make_car_world(&self.art);
        self.intro = Some(intro::Intro::new());
        self.arcade = None;
        self.scene = Scene::Intro;
        self.scene_t = 0.0;
        self.fade = 0.0;
        self.paused = false;
        self.input.release_all();
    }

    fn roll_upgrades() -> [usize; 3] {
        let n = upgrades().len();
        let mut pool: Vec<usize> = (0..n).collect();
        let mut out = [0; 3];
        for slot in &mut out {
            let i = rand::gen_range(0, pool.len());
            *slot = pool.swap_remove(i);
        }
        out
    }

    fn frame(&mut self, dt: f32, scripted: Option<Controls>) {
        self.fx.ensure_size();
        self.scene_t += dt;
        self.fade = (self.fade - dt * 1.6).max(0.0);
        let u = self.fx.scale;
        let pscreen = self.fx.to_screen(&self.world, self.world.player.pos, 9.0);
        let (mut ctl, ui) = self.input.poll(pscreen);
        if let Some(s) = scripted {
            ctl = s;
        }
        if ui.mute {
            self.audio.toggle_mute();
        }
        if ui.light && self.world.has_light {
            self.world.light_on = !self.world.light_on;
            self.audio.play(Id::Select, 0.6);
        }
        if ui.tilt {
            self.fx.tilt_shift = !self.fx.tilt_shift;
            let s = if self.fx.tilt_shift { "TILT-SHIFT ON" } else { "TILT-SHIFT OFF" };
            self.toast = Some((s.to_string(), 1.8));
        }
        let mode = self.input.mode;
        let loops: &[(Id, f32)] = match self.scene {
            Scene::Title => &[(Id::Ambient, 0.5), (Id::Hum, 0.12)],
            Scene::Intro if self.arcade.is_some() => &[(Id::Hum, 0.05)],
            Scene::Intro => match self.intro.as_ref().map(|i| i.stage) {
                Some(intro::Stage::CarRide) | Some(intro::Stage::CarArrive) => &[(Id::Ride, 0.45)],
                Some(intro::Stage::Platform) => &[(Id::Hum, 0.16), (Id::Ambient, 0.18)],
                Some(intro::Stage::Quake) | Some(intro::Stage::Blackout) => &[],
                Some(intro::Stage::Armed) | Some(intro::Stage::Done) => &[(Id::Music, 0.22), (Id::Hum, 0.06)],
                _ => &[(Id::Hum, 0.07)],
            },
            Scene::Play if self.paused => &[(Id::Music, 0.1), (Id::Hum, 0.08)],
            Scene::Play => match self.world.phase {
                Phase::Dead => &[(Id::Hum, 0.1)],
                Phase::Boss => &[(Id::Music, 0.4), (Id::Hum, 0.14)],
                _ => &[(Id::Music, 0.3), (Id::Hum, 0.16)],
            },
            Scene::Upgrade { .. } => &[(Id::Ride, 0.55)],
            Scene::GameOver => &[(Id::Hum, 0.1)],
            Scene::Ride => &[(Id::Ride, 0.55)],
            Scene::Victory => &[(Id::Ambient, 0.4)],
        };
        self.audio.set_loops(loops);
        self.audio.update_loops(dt);

        match self.scene {
            Scene::Title => {
                let t = self.scene_t;
                self.world.update(dt, &Controls::default());
                self.world.sfx.clear();
                let center = iso(self.world.map.w as f32 * 0.5, self.world.map.h as f32 * 0.5);
                self.world.cam = center + vec2((t * 0.12).sin() * 140.0, (t * 0.09).cos() * 50.0);
                self.fx.render(&self.world, &self.art, 1.25);
                if ui.nav.y != 0 {
                    self.menu_sel = (self.menu_sel as i32 + ui.nav.y).rem_euclid(2) as usize;
                    self.audio.play(Id::Select, 0.5);
                }
                let rects = hud::draw_title(&self.art, u, t, &self.defs, mode, self.menu_sel);
                if mode == Mode::Mouse {
                    let m: Vec2 = mouse_position().into();
                    if let Some(i) = rects.iter().position(|r| r.contains(m)) {
                        self.menu_sel = i;
                    }
                }
                let mut go = ui.confirm && t > 0.4;
                if let Some(tap) = ui.tap {
                    if let Some(i) = rects.iter().position(|r| r.contains(tap)) {
                        self.menu_sel = i;
                    } else if mode == Mode::Touch {
                        go = false; // taps outside the menu just pick nothing
                    }
                }
                if go {
                    self.audio.play(Id::Select, 1.0);
                    if self.menu_sel == 0 { self.start_intro(); } else { self.start_arcade(); }
                }
            }
            Scene::Intro => {
                if let Some(mut a) = self.arcade.take() {
                    // standing at the cabinet
                    let mut x = ctl.mv.x;
                    if is_key_down(KeyCode::Left) { x -= 1.0; }
                    if is_key_down(KeyCode::Right) { x += 1.0; }
                    let ain = arcade::ArcadeIn { x: x.clamp(-1.0, 1.0), fire: ctl.fire || ctl.dash || ui.confirm || is_key_down(KeyCode::Space), exit: ui.back || ui.pause };
                    let leave = a.update(dt, &ain);
                    for id in a.sounds.drain(..) {
                        self.audio.play(id, 1.0);
                    }
                    self.arcade_hi = self.arcade_hi.max(a.hi);
                    self.fx.render(&self.world, &self.art, 1.35);
                    a.draw(&self.art, u);
                    if leave {
                        if let Some(i) = &mut self.intro {
                            i.arcade_played = true;
                        }
                    } else {
                        self.arcade = Some(a);
                    }
                } else if let Some(mut it) = self.intro.take() {
                    let out = it.update(&mut self.world, &mut ctl, &ui, mode, dt, &self.fx);
                    if out.need_platform {
                        self.world = World::from_def(&level::kanda_def(), "FOR AKIHABARA", 0, &self.art, self.stats, 0, false);
                        self.world.story = true;
                        it.setup_platform(&mut self.world);
                    }
                    self.world.update(dt, &ctl);
                    self.audio.handle(&self.world.sfx, &self.world);
                    self.world.sfx.clear();
                    if self.world.phase != Phase::Intro {
                        self.audio.horde(&self.world, dt);
                    }
                    self.fx.render(&self.world, &self.art, 1.35);
                    it.draw(&self.art, u, mode, &self.world, &self.fx);
                    let died = self.world.events.iter().any(|e| *e == Event::Died);
                    self.world.events.clear();
                    if out.open_arcade {
                        self.arcade = Some(arcade::Arcade::new(self.arcade_hi));
                        self.audio.play(Id::Coin, 1.0);
                    }
                    if died {
                        self.audio.play(Id::GameOver, 1.0);
                        self.scene = Scene::GameOver;
                        self.scene_t = 0.0;
                    } else if out.finished {
                        if out.skipped {
                            // skipping still hands you what the opening would have
                            self.camp.has_light = true;
                            self.camp.light_on = true;
                            self.start_kanda();
                        } else {
                            self.camp.level = 1;
                            self.save_camp();
                            self.stats_at_start = self.stats;
                            self.kills_at_start = 0;
                            self.scene = Scene::Play;
                            self.scene_t = 0.0;
                        }
                    } else {
                        self.intro = Some(it);
                    }
                }
            }
            Scene::Play if self.arcade.is_some() => {
                let mut a = self.arcade.take().unwrap();
                let mut x = ctl.mv.x;
                if is_key_down(KeyCode::Left) { x -= 1.0; }
                if is_key_down(KeyCode::Right) { x += 1.0; }
                let ain = arcade::ArcadeIn { x: x.clamp(-1.0, 1.0), fire: ctl.fire || ctl.dash || ui.confirm || is_key_down(KeyCode::Space), exit: ui.back || ui.pause };
                let leave = a.update(dt, &ain);
                for id in a.sounds.drain(..) {
                    self.audio.play(id, 1.0);
                }
                self.arcade_hi = self.arcade_hi.max(a.hi);
                self.fx.render(&self.world, &self.art, 1.35);
                a.draw(&self.art, u);
                if !leave {
                    self.arcade = Some(a);
                }
            }
            Scene::Play => {
                if ui.pause && self.world.phase != Phase::Dead {
                    self.paused = !self.paused;
                }
                if !self.paused {
                    self.run_time += dt;
                    self.world.update(dt, &ctl);
                    if mode == Mode::Pad {
                        self.rumble_for_events();
                    }
                    self.audio.handle(&self.world.sfx, &self.world);
                    self.world.sfx.clear();
                    self.audio.horde(&self.world, dt);
                }
                self.fx.render(&self.world, &self.art, 1.35);
                hud::draw_hud(&self.world, &self.art, u, &self.defs, &self.input);
                if !self.paused && self.world.story {
                    // search the dead
                    if let Some(i) = self.world.searchable() {
                        let at = self.fx.to_screen(&self.world, self.world.npcs[i].pos, 14.0);
                        let label = match mode { Mode::Touch => "USE: SEARCH", Mode::Pad => "X: SEARCH", Mode::Mouse => "F: SEARCH" };
                        hud::text_c(&self.art, label, at.x, at.y - 10.0 * u, u, hud::CREAM);
                        if ui.interact {
                            self.world.search(i);
                        }
                    } else if let Some(a) = self.world.map.arcade {
                        if self.world.player.pos.distance(a) < 1.3 {
                            let at = self.fx.to_screen(&self.world, a, 34.0);
                            let label = match mode { Mode::Touch => "USE: PLAY", Mode::Pad => "X: PLAY", Mode::Mouse => "F: PLAY" };
                            hud::text_c(&self.art, label, at.x, at.y - 10.0 * u, u, hud::CREAM);
                            if ui.interact {
                                self.arcade = Some(arcade::Arcade::new(self.arcade_hi));
                                self.audio.play(Id::Coin, 1.0);
                            }
                        }
                    }
                }
                if self.paused {
                    hud::draw_pause(&self.art, u, mode, self.fx.tilt_shift, self.audio.muted);
                }
                let events: Vec<Event> = self.world.events.drain(..).collect();
                for e in events {
                    match e {
                        Event::GotPistol => {}
                        Event::Boarded if self.game_mode == GameMode::Campaign => {
                            self.save_camp();
                            self.scene = Scene::Ride;
                            self.scene_t = 0.0;
                            self.fade = 1.0;
                        }
                        Event::Boarded => {
                            if self.world.station + 1 < self.defs.len() {
                                self.scene = Scene::Upgrade { choices: Self::roll_upgrades() };
                                self.up_sel = 0;
                                self.scene_t = 0.0;
                                self.fade = 1.0;
                            } else {
                                self.scene = Scene::Victory;
                                self.scene_t = 0.0;
                            }
                        }
                        Event::Died => {
                            self.audio.play(Id::GameOver, 1.0);
                            self.scene = Scene::GameOver;
                            self.scene_t = 0.0;
                        }
                        Event::Won => {
                            self.audio.play(Id::Victory, 1.0);
                            self.scene = Scene::Victory;
                            self.scene_t = 0.0;
                        }
                    }
                }
            }
            Scene::Upgrade { choices } => {
                let next = &self.defs[self.world.station + 1];
                // navigate with d-pad/arrows, or hover with the mouse
                let n = ui.nav.x + ui.nav.y;
                if n != 0 {
                    self.up_sel = ((self.up_sel as i32 + n).rem_euclid(3)) as usize;
                }
                let rects = hud::draw_upgrade(&self.art, u, self.scene_t, next, &choices, None, mode);
                if mode == Mode::Mouse {
                    let m: Vec2 = mouse_position().into();
                    if let Some(i) = rects.iter().position(|r| r.contains(m)) {
                        self.up_sel = i;
                    }
                }
                let hover = if mode == Mode::Touch { None } else { Some(self.up_sel) };
                let rects = hud::draw_upgrade(&self.art, u, self.scene_t, next, &choices, hover, mode);
                let mut pick = ui.choice;
                if let Some(tap) = ui.tap {
                    if let Some(i) = rects.iter().position(|r| r.contains(tap)) {
                        pick = Some(i);
                    }
                } else if ui.confirm && mode != Mode::Touch {
                    pick = Some(self.up_sel);
                }
                if self.scene_t > 0.6 {
                    if let Some(k) = pick {
                        self.audio.play(Id::Select, 1.0);
                        (upgrades()[choices[k]].apply)(&mut self.stats);
                        let next = self.world.station + 1;
                        self.start_station(next);
                    }
                }
            }
            Scene::Ride => {
                hud::draw_ride(&self.art, u, self.scene_t, "AKIHABARA", "JY03");
                if self.scene_t > 4.0 || (ui.confirm && self.scene_t > 1.0) {
                    self.start_akiba();
                }
            }
            Scene::GameOver => {
                self.world.update(dt, &Controls::default());
                self.world.sfx.clear();
                self.fx.render(&self.world, &self.art, 0.7);
                let lines = if self.game_mode == GameMode::Campaign {
                    vec![format!("{} KILLS", self.world.total_kills), format!("{} COINS", self.camp.coins)]
                } else {
                    vec![format!("{} KILLS", self.world.total_kills), "YOUR UPGRADES ARE KEPT".to_string()]
                };
                let prompt = match mode {
                    Mode::Touch => "TAP TO RETRY THIS STATION",
                    Mode::Pad => "PRESS A TO RETRY",
                    Mode::Mouse => "PRESS ENTER TO RETRY",
                };
                let title = format!("OVERRUN AT {}", self.world.name);
                hud::draw_end(&self.art, u, self.scene_t, &title, &lines, prompt, hud::RED);
                if ui.confirm && self.scene_t > 1.0 {
                    self.stats = self.stats_at_start;
                    self.world.total_kills = self.kills_at_start;
                    if self.game_mode == GameMode::Campaign {
                        match self.camp.level {
                            2 => self.start_akiba(),
                            _ => self.start_kanda(),
                        }
                    } else {
                        let s = self.world.station;
                        self.start_station(s);
                    }
                }
            }
            Scene::Victory => {
                self.world.update(dt, &Controls::default());
                self.world.sfx.clear();
                self.fx.render(&self.world, &self.art, 1.5);
                let secs = self.run_time as u32;
                let lines = vec![
                    "THE LAST TRAIN IS YOURS".to_string(),
                    format!("{} KILLS", self.world.total_kills),
                    format!("TIME {}:{:02}", secs / 60, secs % 60),
                ];
                let prompt = match mode {
                    Mode::Touch => "TAP TO RIDE AGAIN",
                    Mode::Pad => "PRESS A TO RIDE AGAIN",
                    Mode::Mouse => "PRESS ENTER TO RIDE AGAIN",
                };
                hud::draw_end(&self.art, u, self.scene_t, "SHINJUKU CLEARED", &lines, prompt, hud::GREEN);
                if ui.confirm && self.scene_t > 2.0 {
                    self.world = Self::demo_world(&self.defs, &self.art);
                    self.scene = Scene::Title;
                    self.scene_t = 0.0;
                    self.fade = 1.0;
                }
            }
        }
        for (pos, txt, t, col) in &self.world.floaters {
            if *t < 0.0 {
                continue;
            }
            let at = self.fx.to_screen(&self.world, *pos, 18.0 + t * 14.0);
            hud::text_c(&self.art, txt, at.x, at.y, u, with_alpha(*col, (1.6 - t).min(1.0)));
        }
        if let Some((msg, t)) = &mut self.toast {
            *t -= dt;
            let u = self.fx.scale;
            let a = t.min(1.0).max(0.0);
            hud::text_c(&self.art, msg, screen_width() * 0.5, screen_height() * 0.16, u, with_alpha(hud::CREAM, a));
            if *t <= 0.0 {
                self.toast = None;
            }
        }
        if self.input.pad_toast > 0.0 {
            let u = self.fx.scale;
            let a = self.input.pad_toast.min(1.0);
            hud::text_c(&self.art, "CONTROLLER CONNECTED", screen_width() * 0.5, screen_height() - 24.0 * u, u, with_alpha(hud::GREEN, a));
        }
        if self.audio.muted {
            let u = self.fx.scale;
            let label = if self.input.mode == Mode::Pad { "SOUND OFF  (BACK)" } else { "SOUND OFF  (M)" };
            hud::text_c(&self.art, label, screen_width() * 0.5, screen_height() - 12.0 * u, u, hud::DIM);
        }
        if self.fade > 0.0 {
            draw_rectangle(0.0, 0.0, screen_width(), screen_height(), Color::new(0.0, 0.0, 0.0, self.fade));
        }
    }
}

fn window_conf() -> Conf {
    #[allow(unused_mut)]
    let (mut ww, mut wh) = (1280, 720);
    #[cfg(not(target_arch = "wasm32"))]
    if let Ok(s) = std::env::var("SJ_SIZE") {
        let v: Vec<i32> = s.split('x').filter_map(|n| n.parse().ok()).collect();
        if v.len() == 2 {
            (ww, wh) = (v[0], v[1]);
        }
    }
    Conf {
        window_title: "Last Train to Shinjuku".to_owned(),
        window_width: ww,
        window_height: wh,
        high_dpi: false,
        window_resizable: true,
        ..Default::default()
    }
}

/// Native-only debug hooks: render scripted frames to PNG for quick visual checks.
///   SJ_SHOTS=60,300   frames to capture into /tmp/sj_<frame>.png, then quit
///   SJ_STATION=4      jump straight into a station
///   SJ_SCENE=train|boss|upgrade|title|dead
///   SJ_AUTO=1         autopilot (wanders, auto-aims, throws grenades)
struct Debug {
    shots: Vec<u32>,
    station: Option<usize>,
    scene: String,
    auto: bool,
}

fn debug_cfg() -> Option<Debug> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let shots = std::env::var("SJ_SHOTS").ok()?;
        Some(Debug {
            shots: shots.split(',').filter_map(|s| s.trim().parse().ok()).collect(),
            station: std::env::var("SJ_STATION").ok().and_then(|s| s.parse().ok()),
            scene: std::env::var("SJ_SCENE").unwrap_or_default(),
            auto: std::env::var("SJ_AUTO").is_ok(),
        })
    }
    #[cfg(target_arch = "wasm32")]
    {
        None
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn env_flag(name: &str) -> bool {
    std::env::var(name).is_ok()
}
#[cfg(target_arch = "wasm32")]
fn env_flag(_name: &str) -> bool {
    false
}

/// Debug only: plays through the opening by itself so it can be screenshotted.
fn intro_autopilot(game: &mut Game, frame: u32) -> Controls {
    use intro::Stage::*;
    let mut c = Controls::default();
    let w = &game.world;
    let p = w.player.pos;
    // path around the screen doors and pillars instead of walking into them
    let toward = |t: Vec2| {
        let field = w.map.bfs(t);
        let d = if p.distance(t) > 1.2 { w.map.dir_in(&field, p).unwrap_or(t - p) } else { t - p };
        world_to_screen_dir(d).normalize_or_zero()
    };
    let Some(it) = game.intro.as_mut() else { return c };
    match it.stage {
        CarArrive => {
            let d = *level::CAR_DOORS.iter().min_by(|a, b| (*a - p.x).abs().partial_cmp(&(*b - p.x).abs()).unwrap()).unwrap();
            c.mv = toward(vec2(d, w.map.h as f32));
        }
        Platform => {
            if let Some(m) = it.marker {
                c.mv = toward(m);
            }
        }
        Explore => {
            c.mv = toward(vec2(2.6, *w.map.track_ys.last().unwrap() + 3.9));
        }
        DashPrompt => c.dash = true,
        Wander => {
            if let Some(a) = w.map.arcade {
                if p.distance(a) > 1.0 && !it.arcade_played {
                    c.mv = toward(a);
                } else if !it.arcade_played && game.arcade.is_none() {
                    game.arcade = Some(arcade::Arcade::new(0));
                } else if let Some(m) = it.marker {
                    c.mv = toward(m);
                }
            }
        }
        FirePrompt => c.fire = true,
        Search => {
            if let Some(m) = it.marker {
                if p.distance(m) > 1.0 {
                    c.mv = toward(m);
                } else if let Some(i) = game.world.searchable() {
                    let _ = i;
                }
            }
        }
        Overrun | Ambush => {
            if let Some(k) = w.pickups.iter().find(|k| k.kind == PickupKind::Pistol && k.z <= 0.0) {
                c.mv = toward(k.pos);
            }
        }
        Armed | Done => {
            c.auto_aim = true;
            c.fire = true;
        }
        _ => {}
    }
    // leave the arcade after a few seconds of play
    if game.arcade.is_some() {
        c.fire = frame % 20 < 10;
        c.mv = vec2(((frame as f32) * 0.05).sin(), 0.0);
        if frame % 240 == 0 {
            game.arcade = None;
            if let Some(i) = &mut game.intro {
                i.arcade_played = true;
            }
        }
    }
    c
}

#[macroquad::main(window_conf)]
async fn main() {
    rand::srand((miniquad::date::now() * 1000.0) as u64);
    #[cfg(not(target_arch = "wasm32"))]
    if std::env::var("SJ_WAV").is_ok() {
        // dump the synthesized bank for inspection, then quit
        let _ = std::fs::create_dir_all("/tmp/sfx");
        let mut n = std::collections::HashMap::new();
        for (id, bytes) in synth::bank() {
            let k = n.entry(format!("{:?}", id)).or_insert(0);
            let _ = std::fs::write(format!("/tmp/sfx/{:?}_{}.wav", id, k), bytes);
            *k += 1;
        }
        return;
    }
    let dbg = debug_cfg();
    clear_background(BLACK);
    let msg = "LOADING SOUND...";
    draw_text(msg, screen_width() * 0.5 - 90.0, screen_height() * 0.5, 30.0, GRAY);
    next_frame().await;
    let audio = Audio::load().await;
    let mut game = Game::new(audio);
    if env_flag("SJ_NOTILT") {
        game.fx.tilt_shift = false;
    }
    if let Some(d) = &dbg {
        rand::srand(7);
        if d.scene == "intro" || d.scene == "intro_after" {
            game.start_intro();
        }
        if d.scene == "intro_after" {
            game.world = World::from_def(&level::kanda_def(), "FOR AKIHABARA", 0, &game.art, game.stats, 0, false);
            if let Some(it) = &mut game.intro {
                it.setup_platform(&mut game.world);
                it.setup_aftermath(&mut game.world);
            }
        }
        if d.scene == "akiba" {
            game.camp.has_light = true;
            game.camp.light_on = true;
            game.game_mode = GameMode::Campaign;
            game.start_akiba();
            #[cfg(not(target_arch = "wasm32"))]
            if let Ok(pos) = std::env::var("SJ_POS") {
                let v: Vec<f32> = pos.split(',').filter_map(|n| n.parse().ok()).collect();
                if v.len() == 2 {
                    game.world.player.pos = vec2(v[0], v[1]);
                    game.world.cam = iso(v[0], v[1]);
                    game.world.msg = None;
                }
            }
        }
        if let Some(s) = d.station {
            game.start_station(s);
            game.fade = 0.0;
            match d.scene.as_str() {
                "train" | "boss" => game.world.kills = game.world.quota,
                "upgrade" => {
                    game.scene = Scene::Upgrade { choices: Game::roll_upgrades() };
                }
                "dead" => {
                    game.world.player.hp = 1.0;
                    game.world.player.iframes = 0.0;
                    for k in 0..6 {
                        let p = game.world.player.pos + vec2(1.0 + k as f32 * 0.3, 0.5);
                        game.world.add_zombie(ZKind::Walker, p);
                    }
                }
                pos if pos.starts_with("pos:") => {
                    let v: Vec<f32> = pos[4..].split(',').filter_map(|n| n.parse().ok()).collect();
                    if v.len() == 2 {
                        game.world.player.pos = vec2(v[0], v[1]);
                        game.world.cam = iso(v[0], v[1]);
                        game.world.msg = None;
                    }
                }
                "crows" => {
                    let p = game.world.player.pos;
                    game.world.msg = None;
                    for (k, b) in game.world.birds.iter_mut().enumerate().take(4) {
                        b.pos = p + vec2(3.4 + k as f32 * 0.5, -0.4 + k as f32 * 0.45);
                        b.flying = false;
                    }
                }
                "wall" => {
                    game.world.player.pos = vec2(7.0, 3.6);
                    game.world.cam = iso(7.0, 3.6);
                }
                "win" => {
                    game.world.kills = game.world.quota;
                    game.world.phase = Phase::Boss;
                    let p = game.world.player.pos + vec2(3.0, -1.5);
                    game.world.add_zombie(ZKind::Boss, p);
                    game.world.zombies.last_mut().unwrap().hp = 3.0;
                }
                "bossnear" => {
                    game.world.kills = game.world.quota;
                    game.world.phase = Phase::Boss;
                    let p = game.world.player.pos + vec2(3.0, -1.5);
                    game.world.add_zombie(ZKind::Boss, p);
                }
                _ => {}
            }
        }
    }
    let mut frame: u32 = 0;
    loop {
        let (dt, scripted) = match &dbg {
            Some(d) => {
                let t = frame as f32 / 60.0;
                let c = if d.auto && game.intro.is_some() {
                    Some(intro_autopilot(&mut game, frame))
                } else if d.auto {
                    let mut c = Controls { mv: vec2((t * 0.45).cos(), (t * 0.62).sin()) * 0.8, auto_aim: true, ..Default::default() };
                    if game.world.phase == Phase::Train {
                        if let Some(tr) = &game.world.train {
                            let door = tr.slices.iter().position(|s| *s == SliceK::Door).map(|i| tr.slice_x(i) + 0.5).unwrap_or(5.0);
                            let side = if game.world.player.pos.y < tr.y { -1.3 } else { 1.3 };
                            let target = vec2(door, tr.y + side);
                            c.mv = world_to_screen_dir(target - game.world.player.pos).normalize_or_zero();
                        }
                    }
                    c.grenade = frame % 240 == 120;
                    c.dash = frame % 300 == 200;
                    if d.scene == "dead" {
                        c.mv = Vec2::ZERO;
                        game.world.player.hp = game.world.player.hp.min(5.0);
                    }
                    Some(c)
                } else {
                    None
                };
                (1.0 / 60.0, c)
            }
            None => (get_frame_time(), None),
        };
        game.frame(dt, scripted);
        frame += 1;
        if let Some(d) = &dbg {
            if d.shots.contains(&frame) {
                get_screen_data().export_png(&format!("/tmp/sj_{}.png", frame));
                if env_flag("SJ_STATS") {
                    let st = game.intro.as_ref().map(|i| format!("{:?} t={:.1}", i.stage, i.t)).unwrap_or("-".into());
                    println!("frame {} intro {} arcade {} npcs {} zombies {} gun {}", frame, st, game.arcade.is_some(), game.world.npcs.len(), game.world.zombies.len(), game.world.has_gun);
                    let zs: Vec<String> = game.world.zombies.iter().map(|z| format!("({:.1},{:.1} spd {:.1})", z.pos.x, z.pos.y, z.speed)).collect();
                    let tr = game.world.train.as_ref().map(|t| format!("head {:.1} target {:.1} y {:.1} pass {}", t.head, t.target, t.y, t.pass)).unwrap_or("none".into());
                    println!("   player ({:.1},{:.1}) zombies {:?} train {}", game.world.player.pos.x, game.world.player.pos.y, zs, tr);
                }
            }
            if frame >= *d.shots.iter().max().unwrap_or(&1) {
                if env_flag("SJ_STATS") {
                    let w = &game.world;
                    let on_track = w.zombies.iter().filter(|z| w.map.tile(z.pos.x, z.pos.y) == level::Tile::Track).count();
                    let slow = w.zombies.iter().filter(|z| z.vel.length() < 0.3 && z.age > 3.0).count();
                    println!("kills {} alive {} on_track {} slow {} hp {:.0}", w.kills, w.zombies.len(), on_track, slow, w.player.hp);
                    let ground = w.birds.iter().filter(|b| !b.flying).count();
                    let near: Vec<String> = w.birds.iter().map(|b| format!("({:.1},{:.1} z{:.0})", b.pos.x - w.player.pos.x, b.pos.y - w.player.pos.y, b.z)).collect();
                    println!("birds {} on ground {} rel {:?}", w.birds.len(), ground, near);
                }
                break;
            }
        }
        next_frame().await
    }
}
