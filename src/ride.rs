//! The ride between stations: an empty Yamanote car rattling through the dark,
//! the auto voice announcing the next stop, and sometimes Miyake on the speaker.

use crate::art::Art;
use crate::intro::{draw_dialog, make_car_world, Dialog};
use crate::level::LoopStop;
use crate::util::*;
use crate::world::*;
use macroquad::prelude::*;

/// The radio operator who asks you to clear the line, and checks in along the way.
pub const MIYAKE: &str = "MIYAKE";

/// What Miyake says on the ride after clearing loop stop `i`.
fn checkin(i: usize) -> Option<&'static [&'static str]> {
    Some(match i {
        0 => &["AKIHABARA'S QUIET. GOOD.", "KEEP GOING. I'LL BE WATCHING THE CAMERAS."],
        2 => &["WHAT WAS THAT THING?!", "...THERE WILL BE MORE OF THOSE, WON'T THERE."],
        4 => &["SOME OF THE VENDING MACHINES STILL HAVE POWER.", "IF YOU'VE GOT COINS, USE THEM."],
        9 => &["IKEBUKURO IS NEXT. IT'S HUGE.", "BE CAREFUL. PLEASE."],
        10 => &["YOU'RE A THIRD OF THE WAY AROUND.", "I DIDN'T THINK ANYONE COULD DO THIS."],
        13 => &["SHINJUKU. THE BUSIEST STATION IN THE WORLD.", "...I'M SORRY."],
        14 => &["HALFWAY.", "I HAVEN'T SLEPT. I DON'T THINK YOU HAVE EITHER."],
        17 => &["SHIBUYA'S CLEAR. I SAW PEOPLE MOVING ON THE HARAJUKU CAMERAS.", "ALIVE. YOU'RE GIVING THEM A CHANCE."],
        22 => &["SHINAGAWA. THE SOUTH END OF THE LOOP.", "FROM HERE, YOU'RE HEADING HOME."],
        27 => &["TOKYO STATION IS NEXT.", "THE CAMERAS THERE WENT DARK AN HOUR AGO."],
        28 => &["KANDA, THEN AKIHABARA. WHERE THIS STARTED.", "FINISH IT."],
        29 => &["SOMETHING IS WAITING IN AKIHABARA. SOMETHING BIG.", "...GOOD LUCK."],
        _ => return None,
    })
}

pub struct Ride {
    pub world: World,
    pub t: f32,
    script: Vec<(f32, &'static str, String)>,
    next_line: usize,
    pub dialog: Option<Dialog>,
    end: f32,
}

impl Ride {
    /// `first` is the call from Miyake right after Kanda; `cleared` is the stop just finished.
    pub fn new(art: &Art, next: &LoopStop, first: bool, cleared: Option<usize>) -> Ride {
        let mut w = make_car_world(art);
        // the trains are empty now, except for those who never got off
        w.npcs.retain(|n| n.state == NpcState::Sit && hash2((n.pos.x * 4.0) as i32, 0, 1) < 0.15);
        for n in w.npcs.iter_mut() {
            n.state = NpcState::Corpse;
        }
        w.map.stamp(vec2(6.0, 4.2), 0.9, 2);
        w.map.stamp(vec2(15.5, 2.6), 0.7, 0);
        w.map.flush();
        w.ambient = Color::new(0.12, 0.12, 0.14, 1.0);
        let side = if hash2(next.code.len() as i32, next.name.len() as i32, 5) < 0.5 { "LEFT" } else { "RIGHT" };
        let announce = format!("THE NEXT STATION IS {}. THE DOORS ON THE {} SIDE WILL OPEN.", next.name, side);
        let mut script: Vec<(f32, &'static str, String)> = Vec::new();
        let mut t;
        if first {
            // the call: someone in operations has been watching you on the cameras
            let lines: [(f32, &str); 14] = [
                (1.5, "*KSSHHH*  HELLO?"),
                (3.0, "...HELLO?"),
                (3.5, "...UM. YES. YOU. ON THE TRAIN."),
                (3.5, "I CAN SEE YOU ON THE CAMERAS. YOU HAVE A GUN, RIGHT?"),
                (4.0, "...YEAH. OKAY. OKAY."),
                (3.0, "MY NAME IS MIYAKE. I'M IN THE YAMANOTE LINE OPERATIONS CENTER."),
                (4.2, "THE DOORS HERE ARE STEEL. I'M SAFE. FOR NOW."),
                (3.6, "IT'S NOT JUST KANDA. IT'S EVERY STATION ON THE LOOP."),
                (3.8, "I CAN STILL RUN ONE TRAIN. THIS ONE."),
                (3.2, "I'M ASKING FOR HELP. I NEED SOMEONE TO CLEAR THE STATIONS."),
                (4.2, "ALL OF THEM. ONE BY ONE, ALL THE WAY AROUND THE LOOP."),
                (4.0, "CLEAR A STATION AND I'LL SEND THE TRAIN BACK FOR YOU."),
                (3.8, "...I'M SORRY. I DON'T KNOW WHO ELSE TO ASK."),
                (4.0, "*CLICK*"),
            ];
            t = 0.0;
            for (gap, line) in lines {
                t += gap;
                script.push((t, MIYAKE, line.to_string()));
            }
            t += 3.0;
        } else {
            t = 0.5;
            if let Some(lines) = cleared.and_then(checkin) {
                for line in lines {
                    t += 3.6;
                    script.push((t, MIYAKE, line.to_string()));
                }
                t += 3.6;
            }
        }
        script.push((t, "ANNOUNCEMENT", announce));
        let end = t + 6.5;
        Ride { world: w, t: 0.0, script, next_line: 0, dialog: None, end }
    }

    /// Returns true when the train pulls in. `skip` jumps to the next line (or the end).
    pub fn update(&mut self, dt: f32, skip: bool) -> bool {
        self.t += dt;
        if skip && self.t > 0.8 {
            // jump ahead to the next line; the last skip ends the ride
            if self.next_line < self.script.len() {
                self.t = self.t.max(self.script[self.next_line].0);
            } else {
                return true;
            }
        }
        while self.next_line < self.script.len() && self.t >= self.script[self.next_line].0 {
            let (_, who, text) = &self.script[self.next_line];
            if *who == "ANNOUNCEMENT" {
                self.world.sfx.push(Sfx::Chime);
            }
            let dur = 1.8 + text.len() as f32 * 0.045;
            self.dialog = Some(Dialog { who, text: text.clone(), t: 0.0, dur });
            self.next_line += 1;
        }
        if let Some(d) = &mut self.dialog {
            d.t += dt;
            if d.t > d.dur {
                self.dialog = None;
            }
        }
        // rattle along, then brake into the station
        let braking = self.t > self.end - 3.5;
        self.world.car_speed = if braking { approach(self.world.car_speed, 0.0, dt * 3.0) } else { approach(self.world.car_speed, 9.0, dt * 3.0) };
        if chance(dt * 0.25) && !braking {
            self.world.player.vel += vec2(0.0, if chance(0.5) { 1.0 } else { -1.0 }) * 1.2;
            self.world.shake = (self.world.shake + 0.15).min(0.4);
        }
        self.world.update(dt, &Controls::default());
        self.world.sfx.retain(|s| matches!(s, Sfx::Chime));
        self.t >= self.end
    }

    pub fn draw(&self, art: &Art, u: f32) {
        if let Some(d) = &self.dialog {
            draw_dialog(art, u, d, 0.0);
        }
        let (sw, _) = (screen_width(), screen_height());
        let hint = "TAP / ENTER / A TO SKIP";
        crate::hud::text(art, hint, sw - crate::hud::text_w(hint, u) - 6.0 * u, 6.0 * u, u, with_alpha(crate::hud::DIM, 0.5));
    }
}
