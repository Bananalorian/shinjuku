//! The render pipeline. Three low-res passes, then an integer upscale:
//!   1. scene  : baked floor, depth-sorted sprites, particles
//!   2. light  : ambient + additive light meshes (lamps, muzzle flashes, flashlight)
//!   3. post   : lighting composite with dithered light bands, pixel-locked tilt-shift
//!               blur, bloom, haze, miniature color grade, shockwave, vignette, grain
//! The HUD is drawn afterwards at full resolution in hud.rs.

use crate::art::{Art, Spr};
use crate::level::{PropKind, TRACK_DROP};
use crate::util::*;
use crate::world::*;
use macroquad::miniquad::{BlendFactor, BlendState, BlendValue, Equation, PipelineParams};
use macroquad::prelude::*;

const VERT: &str = r#"#version 100
attribute vec3 position;
attribute vec2 texcoord;
attribute vec4 color0;
varying vec2 uv;
varying vec4 color;
uniform mat4 Model;
uniform mat4 Projection;
void main() {
    gl_Position = Projection * Model * vec4(position, 1.0);
    color = color0 / 255.0;
    uv = texcoord;
}
"#;

const ADD_FRAG: &str = r#"#version 100
precision mediump float;
varying vec2 uv;
varying vec4 color;
uniform sampler2D Texture;
void main() {
    gl_FragColor = texture2D(Texture, uv) * color;
}
"#;

const POST_FRAG: &str = r#"#version 100
#ifdef GL_FRAGMENT_PRECISION_HIGH
precision highp float;
#else
precision mediump float;
#endif
varying vec2 uv;
varying vec4 color;
uniform sampler2D Texture;   // the scene
uniform sampler2D LightTex;  // the light map
uniform vec2 Res;            // low-res size in pixels
uniform float Time;
uniform float Focus;         // tilt-shift focus line (0..1 down the screen)
uniform vec4 Shock;          // xy center, z radius, w strength
uniform vec4 Fx;             // x hurt, y flash, z saturation, w tilt-shift radius
uniform vec3 Fog;
uniform vec2 Cam;            // camera offset, keeps the mist anchored to the world
uniform float Mist;          // drifting mist amount

float h21(vec2 q) { return fract(sin(dot(q, vec2(127.1, 311.7))) * 43758.5453); }
float vnoise(vec2 q) {
    vec2 i = floor(q);
    vec2 f = fract(q);
    f = f * f * (3.0 - 2.0 * f);
    return mix(mix(h21(i), h21(i + vec2(1.0, 0.0)), f.x), mix(h21(i + vec2(0.0, 1.0)), h21(i + vec2(1.0, 1.0)), f.x), f.y);
}

float bayer2(vec2 a) { a = floor(a); return fract(a.x / 2.0 + a.y * a.y * 0.75); }
float bayer4(vec2 a) { return bayer2(0.5 * a) * 0.25 + bayer2(a); }

vec3 lit(vec2 p, float d) {
    vec3 a = texture2D(Texture, p).rgb;
    vec3 l = texture2D(LightTex, p).rgb;
    l = floor(l * 8.0 + 0.5 + d * 0.65) / 8.0;   // banded, dithered light = pixel-art shading
    return 1.0 - exp(-a * l * 3.0);        // soft filmic shoulder, no blown-out sprites
}

void main() {
    vec2 px = floor(uv * Res) + 0.5;      // lock everything to the low-res pixel grid
    vec2 p = px / Res;

    // shockwave ring distortion
    vec2 d = p - Shock.xy;
    d.x *= Res.x / Res.y;
    float dist = length(d);
    float sk = (dist - Shock.z) * 16.0;
    float ring = Shock.w * exp(-sk * sk);
    vec2 n = d / max(dist, 0.0001);
    n.x *= Res.y / Res.x;
    p -= n * ring * 0.03;

    float dth = bayer4(px) - 0.5;

    // tilt-shift: blur grows away from the focus line
    float fy = abs(p.y - Focus);
    float r = smoothstep(0.12, 0.5, fy) * Fx.w;
    vec3 col = vec3(0.0);
    for (int i = 0; i < 12; i++) {
        float fi = float(i);
        float ang = fi * 2.39996;
        float rr = sqrt((fi + 0.5) / 12.0) * r;
        col += lit(p + vec2(cos(ang), sin(ang)) * rr / Res, dth);
    }
    col /= 12.0;

    // bloom from the hot spots
    vec3 bloom = vec3(0.0);
    for (int i = 0; i < 8; i++) {
        float fi = float(i);
        float ang = fi * 0.785 + 0.39;
        float rad = 3.0 + mod(fi, 2.0) * 4.0;
        bloom += max(lit(p + vec2(cos(ang), sin(ang)) * rad / Res, dth) - 0.62, 0.0);
    }
    col += bloom * 0.3;

    // drifting mist, lit by whatever lamps are nearby
    vec2 wp = px + Cam;
    float fg = vnoise(wp * 0.018 + vec2(Time * 0.15, Time * 0.05)) * 0.65 + vnoise(wp * 0.045 - vec2(Time * 0.22, 0.0)) * 0.35;
    fg = smoothstep(0.48, 0.95, fg) * Mist;
    col += fg * (Fog * 0.7 + texture2D(LightTex, p).rgb * 0.45);

    // haze in the distance (top of the screen)
    float far = smoothstep(0.5, 0.0, p.y);
    col = mix(col, Fog, far * 0.3);

    // miniature grade: punchy saturation and contrast
    col = clamp(col, 0.0, 1.0);
    float lum = dot(col, vec3(0.299, 0.587, 0.114));
    col = mix(vec3(lum), col, Fx.z);
    col = clamp(col, 0.0, 1.0);
    col = mix(col, col * col * (3.0 - 2.0 * col), 0.45);
    col += vec3(0.015, 0.0, 0.035) * (1.0 - lum);

    // vignette, damage, flash, grain
    vec2 vc = (p - 0.5) * vec2(1.0, 1.25);
    col *= clamp(1.0 - dot(vc, vc) * 1.1, 0.0, 1.0);
    float edge = smoothstep(0.2, 0.7, length(vc));
    col = mix(col, vec3(0.55, 0.0, 0.03), clamp(Fx.x, 0.0, 1.0) * edge * 0.85);
    col += vec3(Fx.y);
    float g = fract(sin(dot(px + floor(Time * 24.0) * 7.31, vec2(12.9898, 78.233))) * 43758.5453);
    col += (g - 0.5) * 0.035;
    gl_FragColor = vec4(col, 1.0);
}
"#;

pub struct Fx {
    /// Miniature-style depth-of-field. Toggle with T (keyboard) or Y (controller).
    pub tilt_shift: bool,
    pub vw: u32,
    pub vh: u32,
    pub scale: f32,
    scene: RenderTarget,
    light: RenderTarget,
    post_rt: RenderTarget,
    post: Material,
    add: Material,
    size_key: (u32, u32),
}

fn make_rt(w: u32, h: u32, linear: bool) -> RenderTarget {
    let rt = render_target(w, h);
    rt.texture.set_filter(if linear { FilterMode::Linear } else { FilterMode::Nearest });
    rt
}

fn rt_cam(rt: &RenderTarget, w: u32, h: u32, top_left: Vec2) -> Camera2D {
    Camera2D {
        target: top_left + vec2(w as f32 * 0.5, h as f32 * 0.5),
        zoom: vec2(2.0 / w as f32, 2.0 / h as f32),
        render_target: Some(rt.clone()),
        ..Default::default()
    }
}

impl Fx {
    pub fn new() -> Fx {
        let additive = PipelineParams {
            color_blend: Some(BlendState::new(Equation::Add, BlendFactor::Value(BlendValue::SourceAlpha), BlendFactor::One)),
            alpha_blend: Some(BlendState::new(Equation::Add, BlendFactor::Zero, BlendFactor::One)),
            ..Default::default()
        };
        let add = load_material(
            ShaderSource::Glsl { vertex: VERT, fragment: ADD_FRAG },
            MaterialParams { pipeline_params: additive, ..Default::default() },
        )
        .expect("additive shader");
        let post = load_material(
            ShaderSource::Glsl { vertex: VERT, fragment: POST_FRAG },
            MaterialParams {
                uniforms: vec![
                    UniformDesc::new("Res", UniformType::Float2),
                    UniformDesc::new("Time", UniformType::Float1),
                    UniformDesc::new("Focus", UniformType::Float1),
                    UniformDesc::new("Shock", UniformType::Float4),
                    UniformDesc::new("Fx", UniformType::Float4),
                    UniformDesc::new("Fog", UniformType::Float3),
                    UniformDesc::new("Cam", UniformType::Float2),
                    UniformDesc::new("Mist", UniformType::Float1),
                ],
                textures: vec!["LightTex".to_string()],
                ..Default::default()
            },
        )
        .expect("post shader");
        let (vw, vh, scale) = Self::dims();
        Fx {
            tilt_shift: true,
            vw,
            vh,
            scale,
            scene: make_rt(vw, vh, false),
            light: make_rt(vw, vh, true),
            post_rt: make_rt(vw, vh, false),
            post,
            add,
            size_key: (screen_width() as u32, screen_height() as u32),
        }
    }

    /// Pick an integer pixel scale so the low-res view is about 427x240 worth of pixels.
    fn dims() -> (u32, u32, f32) {
        let (sw, sh) = (screen_width().max(64.0), screen_height().max(64.0));
        let s = ((sw * sh / 102_400.0).sqrt()).round().max(1.0);
        ((sw / s).ceil() as u32, (sh / s).ceil() as u32, s)
    }

    pub fn ensure_size(&mut self) {
        let key = (screen_width() as u32, screen_height() as u32);
        if key != self.size_key {
            self.size_key = key;
            let (vw, vh, s) = Self::dims();
            self.vw = vw;
            self.vh = vh;
            self.scale = s;
            self.scene = make_rt(vw, vh, false);
            self.light = make_rt(vw, vh, true);
            self.post_rt = make_rt(vw, vh, false);
        }
    }

    // ------------------------------------------------------------------ frame

    pub fn render(&mut self, w: &World, art: &Art, saturation: f32) {
        let (vw, vh) = (self.vw, self.vh);
        let shake = vec2(rnd(-1.0, 1.0), rnd(-1.0, 1.0)) * w.shake * 5.0;
        let tl = (w.cam - vec2(vw as f32, vh as f32) * 0.5 + shake).round();
        let view = Rect::new(tl.x - 40.0, tl.y - 60.0, vw as f32 + 80.0, vh as f32 + 120.0);

        // 1. scene
        set_camera(&rt_cam(&self.scene, vw, vh, tl));
        clear_background(BLACK);
        draw_texture(&w.map.floor_tex, w.map.origin.x, w.map.origin.y, WHITE);
        draw_car_outside(w, art);
        draw_ground_hazards(w, art);
        draw_rings(w, art);
        draw_sorted(w, art, view);
        draw_xray(w, art);
        draw_particles(w, art, false, view);
        gl_use_material(&self.add);
        draw_particles(w, art, true, view);
        draw_bullets(w, art);
        draw_air_hazards(w, art);
        draw_car_windows(w, art);
        draw_torch_glow(w, art);
        gl_use_default_material();

        // 2. light
        set_camera(&rt_cam(&self.light, vw, vh, tl));
        let amb = w.ambient;
        let e = w.emergency;
        let s = w.light_scale.max(0.25 * (1.0 - e));
        clear_background(Color::new(amb.r * s * (1.0 - 0.55 * e) + 0.03 * e, amb.g * s * (1.0 - 0.65 * e), amb.b * s * (1.0 - 0.6 * e), 1.0));
        gl_use_material(&self.add);
        draw_lights(w, view);
        gl_use_default_material();

        // 3. post
        set_camera(&rt_cam(&self.post_rt, vw, vh, Vec2::ZERO));
        let pscreen = iso(w.player.pos.x, w.player.pos.y) - tl;
        let focus = (pscreen.y / vh as f32).clamp(0.2, 0.8);
        let (sc, sr, ss) = match w.shock {
            Some((at, t)) => {
                let s = (iso(at.x, at.y) - tl) / vec2(vw as f32, vh as f32);
                (s, t * 0.6, (1.0 - t / 0.8).max(0.0))
            }
            None => (vec2(-5.0, -5.0), 0.0, 0.0),
        };
        let low_hp = if w.player.hp < 30.0 && w.phase != Phase::Dead {
            (0.25 + 0.2 * (w.time * 6.0).sin()).max(0.0)
        } else {
            0.0
        };
        self.post.set_uniform("Res", vec2(vw as f32, vh as f32));
        self.post.set_uniform("Time", w.time);
        self.post.set_uniform("Focus", focus);
        self.post.set_uniform("Shock", vec4(sc.x, sc.y, sr, ss));
        let tilt = if self.tilt_shift { 3.4 } else { 0.0 };
        self.post.set_uniform("Fx", vec4(w.hurt * 0.8 + low_hp, w.flash, saturation, tilt));
        let fog = w.ambient;
        self.post.set_uniform("Fog", vec3(fog.r * 0.7, fog.g * 0.7, fog.b * 0.9));
        self.post.set_uniform("Cam", tl);
        self.post.set_uniform("Mist", 0.2f32);
        self.post.set_texture("LightTex", self.light.texture.clone());
        gl_use_material(&self.post);
        draw_texture_ex(&self.scene.texture, 0.0, 0.0, WHITE, DrawTextureParams { dest_size: Some(vec2(vw as f32, vh as f32)), ..Default::default() });
        gl_use_default_material();

        // 4. upscale to the screen
        set_default_camera();
        clear_background(BLACK);
        draw_texture_ex(
            &self.post_rt.texture,
            0.0,
            0.0,
            WHITE,
            DrawTextureParams { dest_size: Some(vec2(vw as f32 * self.scale, vh as f32 * self.scale)), ..Default::default() },
        );
    }

    /// World point -> full-resolution screen point (for the HUD and touch aiming).
    pub fn to_screen(&self, w: &World, p: Vec2, z: f32) -> Vec2 {
        let tl = (w.cam - vec2(self.vw as f32, self.vh as f32) * 0.5).round();
        (iso3(p.x, p.y, z) - tl) * self.scale
    }
}

// ---------------------------------------------------------------------- sprites

fn spr(art: &Art, s: &Spr, at: Vec2, flip: bool, color: Color) {
    let x = if flip { at.x - (s.r.w - s.anchor.x) } else { at.x - s.anchor.x };
    draw_texture_ex(
        &art.tex,
        x.round(),
        (at.y - s.anchor.y).round(),
        color,
        DrawTextureParams { source: Some(s.r), flip_x: flip, ..Default::default() },
    );
}

fn shadow(art: &Art, at: Vec2, scale: f32) {
    let (w, h) = (art.shadow.w * scale, art.shadow.h * scale);
    draw_texture_ex(
        &art.tex,
        (at.x - w * 0.5).round(),
        (at.y - h * 0.5).round(),
        WHITE,
        DrawTextureParams { source: Some(art.shadow), dest_size: Some(vec2(w.round(), h.round())), ..Default::default() },
    );
}

#[derive(Clone, Copy)]
enum Item {
    Prop(usize),
    Zombie(usize),
    Player,
    Pickup(usize),
    Grenade(usize),
    Slice(usize),
    Ghost(usize),
    Bird(usize),
    Npc(usize),
}

fn draw_sorted(w: &World, art: &Art, view: Rect) {
    let mut items: Vec<(f32, Item)> = Vec::with_capacity(w.zombies.len() + w.map.props.len() + 64);
    let inview = |p: Vec2| view.contains(iso(p.x, p.y));
    for (i, pr) in w.map.props.iter().enumerate() {
        let c = pr.pos + pr.size * 0.5;
        if inview(c) && !pr.broken {
            items.push((c.x + c.y, Item::Prop(i)));
        }
    }
    for (i, z) in w.zombies.iter().enumerate() {
        if inview(z.pos) {
            items.push((z.pos.x + z.pos.y, Item::Zombie(i)));
        }
    }
    if !w.demo {
        items.push((w.player.pos.x + w.player.pos.y, Item::Player));
    }
    for (i, k) in w.pickups.iter().enumerate() {
        items.push((k.pos.x + k.pos.y, Item::Pickup(i)));
    }
    for (i, g) in w.grenades.iter().enumerate() {
        items.push((g.pos.x + g.pos.y, Item::Grenade(i)));
    }
    for (i, g) in w.ghosts.iter().enumerate() {
        items.push((g.pos.x + g.pos.y - 0.01, Item::Ghost(i)));
    }
    for (i, n) in w.npcs.iter().enumerate() {
        if inview(n.pos) {
            let d = if n.state == NpcState::Corpse { -0.3 } else { 0.0 };
            items.push((n.pos.x + n.pos.y + d, Item::Npc(i)));
        }
    }
    for (i, b) in w.birds.iter().enumerate() {
        if inview(b.pos) {
            items.push((b.pos.x + b.pos.y + b.z * 0.01, Item::Bird(i)));
        }
    }
    if let Some(t) = &w.train {
        for i in 0..t.slices.len() {
            let x = t.slice_x(i);
            if x >= 0.6 {
                items.push((x + 0.5 + t.y, Item::Slice(i)));
            }
        }
    }
    items.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));

    let pdepth = w.player.pos.x + w.player.pos.y;
    let pscreen = iso3(w.player.pos.x, w.player.pos.y, 10.0);
    for (depth, it) in items {
        match it {
            Item::Prop(i) => {
                let pr = &w.map.props[i];
                let blink = ((w.time * 1.4) as i32 % 2) as usize;
                let (tex, s) = match pr.kind {
                    PropKind::Pillar => (&art.tex, &art.pillar),
                    PropKind::Vending(v) => (&art.tex, &art.vending[v]),
                    PropKind::Bench => (&art.tex, &art.bench),
                    PropKind::Gate => (&art.tex, &art.gate),
                    PropKind::Bin => (&art.tex, &art.bin),
                    PropKind::Psd => (&art.tex, &art.psd[((pr.pos.x * 2.0) as i32 % 5 == 2) as usize]),
                    PropKind::Kiosk => (&art.tex, &art.kiosk),
                    PropKind::Suitcase(v) => (&art.tex, &art.suitcases[v.min(art.suitcases.len() - 1)]),
                    PropKind::Boxes => (&art.tex, &art.boxes),
                    PropKind::Barrier => (&art.tex, &art.barrier),
                    PropKind::EscStep(i) => (&art.tex, &art.esc_step[i]),
                    PropKind::EscRail(i) => (&art.tex, &art.esc_rail[i]),
                    PropKind::Cabinet => (&art.tex, &art.cabinet),
                    PropKind::CarSeat => (&art.tex, &art.car_seat),
                    PropKind::CarWall => (&art.tex, &art.car_wall),
                    PropKind::CarDoor => {
                        if w.car_doors_open {
                            continue;
                        }
                        (&art.tex, &art.car_door)
                    }
                    PropKind::Straps => (&art.tex, &art.straps),
                    PropKind::Ad(n) => (&art.tex, &art.ads[n % art.ads.len()]),
                    PropKind::MazeWall(k) => (&art.tex, &art.maze[k % art.maze.len()]),
                    PropKind::SignName => (&w.map.signs.tex, &w.map.signs.name),
                    PropKind::SignLed => (&w.map.signs.tex, &w.map.signs.led[blink]),
                };
                let at = iso(pr.pos.x, pr.pos.y);
                let mut col = WHITE;
                // tall things fade out when you walk behind them
                if pr.kind.occludes() && depth > pdepth && !w.demo {
                    let r = Rect::new(at.x - s.anchor.x - 2.0, at.y - s.anchor.y, s.r.w + 4.0, s.r.h);
                    if r.contains(pscreen) {
                        col = Color::new(1.0, 1.0, 1.0, 0.35);
                    }
                }
                draw_texture_ex(
                    tex,
                    (at.x - s.anchor.x).round(),
                    (at.y - s.anchor.y).round(),
                    col,
                    DrawTextureParams { source: Some(s.r), ..Default::default() },
                );
            }
            Item::Zombie(i) => draw_zombie(w, art, &w.zombies[i]),
            Item::Player => draw_player(w, art),
            Item::Pickup(i) => {
                let k = &w.pickups[i];
                let gz = w.map.ground_z(k.pos);
                shadow(art, iso3(k.pos.x, k.pos.y, gz), 0.7);
                let bob = gz + 4.0 + k.z + if k.z <= 0.0 { (k.t * 4.0).sin() * 2.0 } else { 0.0 };
                if let PickupKind::Coin(_) = k.kind {
                    // a little spinning coin
                    let at = iso3(k.pos.x, k.pos.y, gz + 3.0 + k.z);
                    let spin = ((k.t * 8.0 + k.pos.x * 3.0).sin().abs() * 5.0).max(1.0);
                    draw_texture_ex(&art.tex, (at.x - spin * 0.5).round(), (at.y - 2.5).round(), WHITE, DrawTextureParams { source: Some(art.coin), dest_size: Some(vec2(spin.round(), 5.0)), ..Default::default() });
                } else if k.kind == PickupKind::Pistol {
                    spr(art, &art.pistol, iso3(k.pos.x, k.pos.y, bob), false, WHITE);
                } else if k.t < 15.0 || (k.t * 8.0) as i32 % 2 == 0 {
                    spr(art, &art.onigiri, iso3(k.pos.x, k.pos.y, bob), false, WHITE);
                }
            }
            Item::Grenade(i) => {
                let g = &w.grenades[i];
                let gz = w.map.ground_z(g.pos);
                shadow(art, iso3(g.pos.x, g.pos.y, gz), 0.4);
                let at = iso3(g.pos.x, g.pos.y, gz + g.z + 2.0);
                draw_texture_ex(&art.tex, at.x.round() - 3.0, at.y.round() - 3.0, WHITE, DrawTextureParams { source: Some(art.grenade), ..Default::default() });
            }
            Item::Ghost(i) => {
                let g = &w.ghosts[i];
                let gz = w.map.ground_z(g.pos);
                let view = if g.back { 1 } else { 0 };
                let a = (g.life / 0.25) * 0.55;
                spr(art, &art.player.flash[view][g.frame], iso3(g.pos.x, g.pos.y, gz), g.face_left, Color::new(0.3, 0.9, 1.0, a));
            }
            Item::Npc(i) => draw_npc(w, art, &w.npcs[i]),
            Item::Bird(i) => {
                let b = &w.birds[i];
                let gz = w.map.ground_z(b.pos);
                if b.z < 40.0 {
                    shadow(art, iso3(b.pos.x, b.pos.y, gz), 0.45 * (1.0 - b.z / 40.0) + 0.1);
                }
                let frame = if b.flying { 2 + (b.anim as usize % 2) } else if (b.t * 1.7).fract() > 0.72 { 1 } else { 0 };
                spr(art, &art.crow[frame], iso3(b.pos.x, b.pos.y, gz + b.z), b.face_left, WHITE);
            }
            Item::Slice(i) => {
                let t = w.train.as_ref().unwrap();
                let base = (iso(t.head - 1.0, t.y - 0.7) + vec2(0.0, TRACK_DROP)).round();
                let at = base - vec2(16.0, 8.0) * i as f32;
                let s = match t.slices[i] {
                    SliceK::Cab => &art.train_cab,
                    SliceK::Body => &art.train_body,
                    SliceK::Gap => &art.train_gap,
                    SliceK::Door => {
                        if t.doors_open { &art.train_door_open } else { &art.train_door }
                    }
                };
                spr(art, s, at, false, WHITE);
            }
        }
    }
}

fn draw_npc(w: &World, art: &Art, n: &Npc) {
    let gz = w.map.ground_z(n.pos);
    let ground = iso3(n.pos.x, n.pos.y, gz);
    let look = n.look % art.commuters.len();
    let ca = if n.officer { &art.officer } else { &art.commuters[look] };
    let sway = if matches!(n.state, NpcState::Stand | NpcState::Strap) { (n.vel.x - n.vel.y).clamp(-1.0, 1.0) * 2.0 } else { 0.0 };
    match n.state {
        NpcState::Corpse => {
            let s = &ca.frames[0][0];
            let at = ground;
            draw_texture_ex(
                &art.tex,
                (at.x - s.r.h * 0.5).round(),
                (at.y - s.r.w * 0.5).round(),
                Color::new(0.72, 0.66, 0.66, 1.0),
                DrawTextureParams { source: Some(s.r), rotation: if n.face_left { -std::f32::consts::FRAC_PI_2 } else { std::f32::consts::FRAC_PI_2 }, ..Default::default() },
            );
            return;
        }
        NpcState::Sit => {
            spr(art, &art.sit[look], ground, n.face_left, WHITE);
            return;
        }
        _ => {}
    }
    shadow(art, ground, 1.0);
    match n.state {
        NpcState::Strap => spr(art, &art.strap[look], ground + vec2(sway, 0.0), n.face_left, WHITE),
        NpcState::Kneel => spr(art, &art.kneel, ground, n.face_left, WHITE),
        _ => {
            let view = if n.back { 1 } else { 0 };
            let moving = matches!(n.state, NpcState::Walk | NpcState::Ride);
            let frame = if moving { (n.anim as usize) % 4 } else { 0 };
            let at = iso3(n.pos.x, n.pos.y, gz + n.z) + vec2(sway, 0.0);
            spr(art, &ca.frames[view][frame], at, n.face_left, WHITE);
            if n.officer && n.state == NpcState::Shoot {
                let sd = world_to_screen_dir(n.aim).normalize_or_zero();
                let a = sd.y.atan2(sd.x);
                let pivot = (at + vec2(sd.x.signum() * 2.0, -9.0)).round();
                let left = sd.x < 0.0;
                let s = &art.pistol;
                let ay = if left { s.r.h - s.anchor.y } else { s.anchor.y };
                draw_texture_ex(&art.tex, pivot.x - s.anchor.x, pivot.y - ay, WHITE, DrawTextureParams { source: Some(s.r), rotation: a, pivot: Some(pivot), flip_y: left, ..Default::default() });
            }
        }
    }
}

/// If the player is hidden behind the train, draw a silhouette on top so you never lose yourself.
fn draw_xray(w: &World, art: &Art) {
    let Some(t) = &w.train else { return };
    let p = &w.player;
    if w.demo || w.phase == Phase::Dead || !(p.pos.y < t.y && p.pos.x > t.tail() - 1.0 && p.pos.x < t.head + 1.5) {
        return;
    }
    let gz = w.map.ground_z(p.pos);
    let view = if p.back { 1 } else { 0 };
    let frame = if p.moving { (p.anim as usize) % 4 } else { 0 };
    spr(art, &art.player.flash[view][frame], iso3(p.pos.x, p.pos.y, gz), p.face_left, Color::new(0.4, 0.95, 1.0, 0.55));
}

fn draw_zombie(w: &World, art: &Art, z: &Zombie) {
    let gz = w.map.ground_z(z.pos);
    let ground = iso3(z.pos.x, z.pos.y, gz);
    let ca = match z.kind {
        ZKind::Walker => &art.zombies[0][z.variant.min(art.zombies[0].len() - 1)],
        ZKind::Runner => &art.zombies[1][z.variant.min(art.zombies[1].len() - 1)],
        ZKind::Brute => &art.zombies[2][0],
        ZKind::Boss => &art.boss,
    };
    let sh = match z.kind {
        ZKind::Boss => 3.0,
        ZKind::Brute => 1.4,
        _ => 1.0,
    };
    shadow(art, ground, sh);
    let view = if z.back && z.kind != ZKind::Boss { 1 } else { 0 };
    let frame = (z.anim as usize) % 4;
    let alpha = (z.age / 0.4).min(1.0);
    let mut at = ground;
    if z.kind == ZKind::Boss && z.slam_t > 0.0 {
        at += vec2(rnd(-1.0, 1.0), -((0.9 - z.slam_t) * 10.0).min(8.0));
    }
    if z.kind == ZKind::Boss && w.boss_charge.map_or(false, |c| c.1 > 0.0) {
        at += vec2(rnd(-1.5, 1.5), 0.0); // shaking with the windup
    }
    let tint = if z.kind == ZKind::Boss { w.boss_kind.tint() } else { WHITE };
    if z.flash > 0.0 && z.kind == ZKind::Boss {
        spr(art, &ca.frames[view][frame], at, z.face_left, tint);
        spr(art, &ca.flash[view][frame], at, z.face_left, Color::new(1.0, 0.85, 0.8, 0.45));
    } else if z.flash > 0.0 {
        spr(art, &ca.flash[view][frame], at, z.face_left, Color::new(1.0, 1.0, 1.0, alpha));
    } else {
        spr(art, &ca.frames[view][frame], at, z.face_left, Color::new(tint.r, tint.g, tint.b, alpha));
    }
}

/// Acid on the floor, and the line a boss is about to charge down. Drawn under everyone.
fn draw_ground_hazards(w: &World, art: &Art) {
    for h in &w.hazards {
        if h.kind == HazardKind::Acid {
            let at = iso(h.pos.x, h.pos.y);
            let fade = (h.t * 3.0).min(1.0) * ((h.life - h.t) * 1.5).min(1.0);
            let wob = 1.0 + (w.time * 6.0 + h.pos.x).sin() * 0.05;
            let (rw, rh) = (h.r * HALF_W * 2.0 * wob, h.r * HALF_H * 2.0 * wob);
            draw_texture_ex(&art.tex, at.x - rw * 0.5, at.y - rh * 0.5, Color::new(0.45, 0.9, 0.25, 0.55 * fade), DrawTextureParams { source: Some(art.soft), dest_size: Some(vec2(rw, rh)), ..Default::default() });
            draw_texture_ex(&art.tex, at.x - rw * 0.3, at.y - rh * 0.3, Color::new(0.75, 1.0, 0.4, 0.4 * fade), DrawTextureParams { source: Some(art.soft), dest_size: Some(vec2(rw * 0.6, rh * 0.6)), ..Default::default() });
        }
    }
    if let (Some((dir, wind, _)), Some(b)) = (w.boss_charge, w.boss()) {
        if wind > 0.0 {
            let a = iso(b.pos.x, b.pos.y);
            let e = iso(b.pos.x + dir.x * 11.0, b.pos.y + dir.y * 11.0);
            let pulse = 0.3 + 0.3 * (w.time * 20.0).sin().abs();
            draw_line(a.x, a.y, e.x, e.y, 6.0, Color::new(1.0, 0.15, 0.1, pulse));
            draw_line(a.x, a.y, e.x, e.y, 2.0, Color::new(1.0, 0.6, 0.4, pulse + 0.2));
        }
    }
}

/// Sparks, shockwaves and acid in flight, drawn over everything.
fn draw_air_hazards(w: &World, art: &Art) {
    for h in &w.hazards {
        match h.kind {
            HazardKind::Spark => {
                let at = iso3(h.pos.x, h.pos.y, w.map.ground_z(h.pos) + 6.0);
                let flick = rnd(3.0, 6.0);
                draw_texture_ex(&art.tex, at.x - flick, at.y - flick, Color::new(0.6, 0.85, 1.0, 0.9), DrawTextureParams { source: Some(art.soft), dest_size: Some(vec2(flick * 2.0, flick * 2.0)), ..Default::default() });
                draw_rectangle(at.x - 1.0, at.y - 1.0, 2.0, 2.0, WHITE);
            }
            HazardKind::Wave => {
                let c = iso(h.pos.x, h.pos.y);
                let n = 40;
                let fade = 1.0 - h.t / h.life;
                for k in 0..n {
                    let a0 = k as f32 / n as f32 * std::f32::consts::TAU;
                    let a1 = (k + 1) as f32 / n as f32 * std::f32::consts::TAU;
                    let p0 = c + vec2((a0.cos() - a0.sin()) * HALF_W, (a0.cos() + a0.sin()) * HALF_H) * h.r;
                    let p1 = c + vec2((a1.cos() - a1.sin()) * HALF_W, (a1.cos() + a1.sin()) * HALF_H) * h.r;
                    draw_line(p0.x, p0.y, p1.x, p1.y, 3.0, Color::new(1.0, 0.55, 0.2, 0.8 * fade));
                }
            }
            HazardKind::Blob => {
                let g = iso(h.pos.x, h.pos.y);
                shadow(art, g, 0.4);
                let at = iso3(h.pos.x, h.pos.y, h.z);
                draw_circle(at.x, at.y, 3.0, Color::new(0.55, 0.95, 0.3, 1.0));
                draw_circle(at.x - 1.0, at.y - 1.0, 1.2, Color::new(0.9, 1.0, 0.7, 1.0));
            }
            HazardKind::Acid => {}
        }
    }
}

fn draw_player(w: &World, art: &Art) {
    let p = &w.player;
    if w.phase == Phase::Dead || w.player_lying {
        // lying in a pool of it (or knocked out)
        let gz = w.map.ground_z(p.pos);
        let at = iso3(p.pos.x, p.pos.y, gz);
        let s = &art.player.frames[0][0];
        draw_texture_ex(
            &art.tex,
            (at.x - s.r.h * 0.5).round(),
            (at.y - s.r.w * 0.5).round(),
            Color::new(0.8, 0.7, 0.7, 1.0),
            DrawTextureParams { source: Some(s.r), rotation: std::f32::consts::FRAC_PI_2, ..Default::default() },
        );
        return;
    }
    if p.iframes > 0.0 && p.dash_t <= 0.0 && (p.iframes * 20.0) as i32 % 2 == 0 {
        return;
    }
    let gz = w.map.ground_z(p.pos);
    shadow(art, iso3(p.pos.x, p.pos.y, gz), 1.0);
    let ground = iso3(p.pos.x, p.pos.y, gz + w.player_z);
    let view = if p.back { 1 } else { 0 };
    let frame = if p.moving { (p.anim as usize) % 4 } else { 0 };
    let gun = |art: &Art| {
        let a = p.aim_screen.y.atan2(p.aim_screen.x);
        let pivot = (ground + vec2(p.aim_screen.x.signum() * 2.0, -9.0)).round();
        let left = p.aim_screen.x < 0.0;
        let s = if w.stats.multishot > 1 || w.stats.rate > 7.5 { &art.gun } else { &art.pistol };
        let ay = if left { s.r.h - s.anchor.y } else { s.anchor.y };
        draw_texture_ex(
            &art.tex,
            pivot.x - s.anchor.x,
            pivot.y - ay,
            WHITE,
            DrawTextureParams { source: Some(s.r), rotation: a, pivot: Some(pivot), flip_y: left, ..Default::default() },
        );
    };
    if p.back && w.has_gun {
        gun(art);
    }
    spr(art, &art.player.frames[view][frame], ground, p.face_left, WHITE);
    if !p.back && w.has_gun {
        gun(art);
    }
}

fn draw_rings(w: &World, art: &Art) {
    for r in &w.rings {
        let c = iso(r.pos.x, r.pos.y);
        let k = r.t / r.max;
        let pulse = 0.5 + 0.5 * (r.t * 30.0).sin();
        for (scale, alpha) in [(1.0, 0.5 + 0.4 * pulse), (k, 0.6)] {
            let (rw, rh) = (r.r * 2.0 * 22.6 * scale, r.r * 2.0 * 11.3 * scale);
            draw_texture_ex(
                &art.tex,
                (c.x - rw * 0.5).round(),
                (c.y - rh * 0.5).round(),
                with_alpha(r.color, alpha),
                DrawTextureParams { source: Some(art.ring), dest_size: Some(vec2(rw.round(), rh.round())), ..Default::default() },
            );
        }
    }
}

fn draw_particles(w: &World, art: &Art, additive: bool, view: Rect) {
    for p in &w.particles {
        if p.additive() != additive {
            continue;
        }
        let at = iso3(p.p.x, p.p.y, p.p.z);
        if !view.contains(at) {
            continue;
        }
        let k = (p.life / p.max).clamp(0.0, 1.0);
        match p.kind {
            PK::Casing => {
                draw_texture_ex(&art.tex, at.x.round(), at.y.round(), WHITE, DrawTextureParams { source: Some(art.casing), ..Default::default() });
            }
            PK::Smoke | PK::Flash => {
                let s = p.size * 2.0;
                let a = if p.kind == PK::Smoke { p.color.a * k } else { 1.0 };
                draw_texture_ex(
                    &art.tex,
                    (at.x - s * 0.5).round(),
                    (at.y - s * 0.5).round(),
                    with_alpha(p.color, a),
                    DrawTextureParams { source: Some(art.soft), dest_size: Some(vec2(s.round(), s.round())), ..Default::default() },
                );
            }
            PK::Spark => {
                let sv = world_to_screen_dir(vec2(p.v.x, p.v.y)) - vec2(0.0, p.v.z);
                let len = (sv.length() * 0.012).clamp(1.0, 4.0);
                let a = sv.y.atan2(sv.x);
                draw_texture_ex(
                    &art.tex,
                    at.x.round(),
                    at.y.round(),
                    with_alpha(p.color, k),
                    DrawTextureParams { source: Some(art.white), dest_size: Some(vec2(len, 1.0)), rotation: a, pivot: Some(at.round()), ..Default::default() },
                );
            }
            _ => {
                let s = p.size.round().max(1.0);
                let a = match p.kind {
                    PK::Dust => p.color.a * (k * 3.0).min(1.0) * ((1.0 - k) * 4.0).min(1.0),
                    PK::Ember => k,
                    _ => if p.rest { k.min(0.25) * 4.0 } else { 1.0 },
                };
                draw_texture_ex(
                    &art.tex,
                    at.x.round(),
                    at.y.round(),
                    with_alpha(p.color, a.min(1.0)),
                    DrawTextureParams { source: Some(art.white), dest_size: Some(vec2(s, s)), ..Default::default() },
                );
            }
        }
    }
}

/// Beyond the car's open side: the tunnel wall streaking past while the train moves.
/// When the train stops, it fades away and you see the platform.
fn draw_car_outside(w: &World, art: &Art) {
    if !w.map.is_car {
        return;
    }
    let k = (w.car_speed / 4.0).clamp(0.0, 1.0);
    if k <= 0.0 {
        return;
    }
    let (h, gw, gh) = (w.map.h as f32 + 0.05, w.map.gw as f32 + 4.0, w.map.gh as f32 + 4.0);
    let (a, b, c, d) = (iso(-4.0, h), iso(gw, h), iso(gw, gh), iso(-4.0, gh));
    let dark = Color::new(0.03, 0.03, 0.04, k);
    draw_triangle(a, b, c, dark);
    draw_triangle(a, c, d, dark);
    let (e, f, g, hh) = (iso(w.map.w as f32 + 0.1, 0.0), iso(gw, 0.0), iso(gw, h), iso(w.map.w as f32 + 0.1, h));
    draw_triangle(e, f, g, dark);
    draw_triangle(e, g, hh, dark);
    // motion-blurred streaks: wall seams, cable runs and the odd lamp, all sliding past
    let len = gw + 4.0;
    for i in 0..26 {
        let y = h + 0.3 + (i % 9) as f32 * 0.55 + (i / 9) as f32 * 0.17;
        let speed = w.car_speed * (1.4 + (i % 3) as f32 * 0.25);
        let x = len - ((w.time * speed + i as f32 * 7.3) % (len + 8.0)) - 4.0;
        let streak = (w.car_speed * 0.5).clamp(1.0, 5.0) + (i % 4) as f32;
        let p0 = iso(x, y);
        let p1 = iso(x + streak, y);
        let lamp = i % 7 == 0;
        let col = if lamp { Color::new(1.0, 0.85, 0.55, 0.55 * k) } else { Color::new(0.35, 0.36, 0.42, 0.45 * k) };
        let thick = if lamp { 2.0 } else { 1.0 };
        draw_line(p0.x, p0.y, p1.x, p1.y, thick, col);
    }
    let _ = art;
}

/// Tunnel lights streaking past the car windows while the train moves.
fn draw_car_windows(w: &World, art: &Art) {
    if !w.map.is_car {
        return;
    }
    let len = w.map.w as f32;
    for k in 0..7 {
        let x = len - ((w.time * w.car_speed * 1.6 + k as f32 * len / 7.0) % len);
        for z in [31.0f32, 37.0, 42.0] {
            let at = iso3(x, 1.0, z).round();
            let n = (w.car_speed * 1.2).clamp(2.0, 14.0);
            draw_texture_ex(&art.tex, at.x, at.y, Color::new(1.0, 0.92, 0.7, 0.55), DrawTextureParams { source: Some(art.white), dest_size: Some(vec2(n, 1.0)), rotation: 0.4636, ..Default::default() });
        }
    }
}

/// A small hot spot at the lens, at hand height, so the light reads as held.
fn draw_torch_glow(w: &World, art: &Art) {
    if !(w.has_light && w.light_on) || w.demo || w.phase == Phase::Dead || w.player_lying {
        return;
    }
    let p = &w.player;
    let gz = w.map.ground_z(p.pos);
    let at = iso3(p.pos.x, p.pos.y, gz + w.player_z + 9.0) + p.aim_screen * 6.0;
    for (s, a) in [(7.0f32, 0.35f32), (3.0, 0.9)] {
        draw_texture_ex(&art.tex, (at.x - s * 0.5).round(), (at.y - s * 0.5).round(), Color::new(1.0, 0.97, 0.85, a), DrawTextureParams { source: Some(art.soft), dest_size: Some(vec2(s, s)), ..Default::default() });
    }
}

fn draw_bullets(w: &World, art: &Art) {
    for b in &w.bullets {
        let gz = w.map.ground_z(b.pos).max(-2.0);
        let at = iso3(b.pos.x, b.pos.y, 9.0 + gz).round();
        let sv = world_to_screen_dir(b.vel);
        let a = sv.y.atan2(sv.x);
        draw_texture_ex(
            &art.tex,
            at.x - 5.0,
            at.y,
            Color::new(1.0, 0.92, 0.6, 1.0),
            DrawTextureParams { source: Some(art.white), dest_size: Some(vec2(6.0, 1.0)), rotation: a, pivot: Some(at), ..Default::default() },
        );
    }
}

// ---------------------------------------------------------------------- lights

fn col_k(c: Color, k: f32) -> Color {
    Color::new((c.r * k).min(1.0), (c.g * k).min(1.0), (c.b * k).min(1.0), 1.0)
}

fn light_circle(center: Vec2, radius: f32, color: Color, k: f32) {
    const N: usize = 16;
    let mut v = Vec::with_capacity(1 + N * 2);
    let c = iso(center.x, center.y);
    v.push(Vertex::new(c.x, c.y, 0.0, 0.5, 0.5, col_k(color, k)));
    for ring in [0.45f32, 1.0] {
        let kk = if ring < 1.0 { k * 0.42 } else { 0.0 };
        for i in 0..N {
            let a = i as f32 / N as f32 * std::f32::consts::TAU;
            let p = iso(center.x + a.cos() * radius * ring, center.y + a.sin() * radius * ring);
            v.push(Vertex::new(p.x, p.y, 0.0, 0.5, 0.5, col_k(color, kk)));
        }
    }
    let mut idx = Vec::with_capacity(N * 9);
    for i in 0..N {
        let j = (i + 1) % N;
        idx.extend_from_slice(&[0, (1 + i) as u16, (1 + j) as u16]);
        let (a, b, c2, d) = (1 + i, 1 + j, 1 + N + i, 1 + N + j);
        idx.extend_from_slice(&[a as u16, c2 as u16, b as u16, b as u16, c2 as u16, d as u16]);
    }
    draw_mesh(&Mesh { vertices: v, indices: idx, texture: None });
}

fn light_cone(origin: Vec2, dir: Vec2, len: f32, half: f32, color: Color, k: f32) {
    const N: usize = 10;
    let base = dir.y.atan2(dir.x);
    let mut v = Vec::with_capacity(1 + (N + 1) * 2);
    let o = iso(origin.x, origin.y);
    v.push(Vertex::new(o.x, o.y, 0.0, 0.5, 0.5, col_k(color, k * 0.6)));
    for (ring, rk) in [(0.35f32, 0.75f32), (1.0, 0.0)] {
        for i in 0..=N {
            let t = i as f32 / N as f32 * 2.0 - 1.0;
            let a = base + t * half;
            let p = origin + vec2(a.cos(), a.sin()) * len * ring;
            let s = iso(p.x, p.y);
            v.push(Vertex::new(s.x, s.y, 0.0, 0.5, 0.5, col_k(color, k * rk * (1.0 - t * t))));
        }
    }
    let mut idx = Vec::new();
    for i in 0..N {
        idx.extend_from_slice(&[0, (1 + i) as u16, (2 + i) as u16]);
        let (a, b, c, d) = (1 + i, 2 + i, 2 + N + i, 3 + N + i);
        idx.extend_from_slice(&[a as u16, c as u16, b as u16, b as u16, c as u16, d as u16]);
    }
    draw_mesh(&Mesh { vertices: v, indices: idx, texture: None });
}

fn draw_lights(w: &World, view: Rect) {
    let visible = |p: Vec2, r: f32| {
        let s = iso(p.x, p.y);
        s.x > view.x - r * 24.0 && s.x < view.x + view.w + r * 24.0 && s.y > view.y - r * 12.0 && s.y < view.y + view.h + r * 12.0
    };
    for l in &w.map.lights {
        if !visible(l.pos, l.radius) {
            continue;
        }
        // after the quake: most tubes are dead, the survivors flicker
        let survivor = hash2((l.pos.x * 3.0) as i32, (l.pos.y * 3.0) as i32, 17) < 0.3;
        if w.emergency > 0.0 && !survivor {
            if hash2((l.pos.x * 3.0) as i32, (l.pos.y * 3.0) as i32, 23) < 0.35 {
                light_circle(l.pos, 1.4, Color::new(1.0, 0.15, 0.1, 1.0), 0.3 * w.emergency);
            }
            continue;
        }
        let mut k = 0.5 * w.light_scale;
        if l.flicker > 0.0 || (w.emergency > 0.0 && survivor) {
            let n = hash2((w.time * 14.0 + l.phase * 3.0) as i32, l.phase as i32, 7);
            let slow = ((w.time * 0.7 + l.phase).sin() * 0.5 + 0.5) > 0.25;
            k *= if slow && n > 0.15 { 1.0 } else { 0.12 };
        }
        if l.strobe {
            let s = ((w.time * 3.0 + l.phase).sin() * 0.5 + 0.5).powi(3);
            k = 0.35 * s;
        }
        light_circle(l.pos, l.radius, l.color, k);
    }
    for l in &w.lights {
        light_circle(l.pos, l.radius, l.color, (l.life / l.max).clamp(0.0, 1.0) * 0.9);
    }
    if w.map.is_car && w.car_speed > 0.5 {
        let len = w.map.w as f32;
        for k in 0..3 {
            let x = len - ((w.time * w.car_speed * 1.6 + k as f32 * len / 3.0) % len);
            light_circle(vec2(x, 1.6), 1.6, Color::new(1.0, 0.85, 0.6, 1.0), 0.35);
        }
    }
    for b in &w.bullets {
        light_circle(b.pos, 0.9, Color::new(1.0, 0.85, 0.5, 1.0), 0.3);
    }
    for k in &w.pickups {
        if let PickupKind::Coin(_) = k.kind {
            light_circle(k.pos, 0.6, Color::new(1.0, 0.85, 0.3, 1.0), 0.25);
        } else {
            light_circle(k.pos, 1.2, Color::new(0.6, 1.0, 0.7, 1.0), 0.35);
        }
    }
    for h in &w.hazards {
        match h.kind {
            HazardKind::Spark => light_circle(h.pos, 1.6, Color::new(0.5, 0.75, 1.0, 1.0), 0.6),
            HazardKind::Acid => light_circle(h.pos, h.r * 1.4, Color::new(0.4, 0.9, 0.2, 1.0), 0.25),
            HazardKind::Wave => {}
            HazardKind::Blob => light_circle(h.pos, 0.8, Color::new(0.5, 1.0, 0.3, 1.0), 0.3),
        }
    }
    if w.frenzy > 0.0 {
        if let Some(b) = w.boss() {
            light_circle(b.pos, 4.0, Color::new(0.5, 0.7, 1.0, 1.0), 0.25 * (w.time * 10.0).sin().abs());
        }
    }
    // the officer's own flashlight, so you can see his last stand
    for n in &w.npcs {
        if n.officer && n.state == NpcState::Shoot {
            let a = n.aim.normalize_or_zero();
            light_circle(n.pos, 2.0, Color::new(0.9, 0.9, 1.0, 1.0), 0.35);
            light_cone(n.pos + a * 0.55, a, 6.5, 0.4, Color::new(1.0, 0.95, 0.8, 1.0), 0.8);
        }
    }
    for z in &w.zombies {
        if z.kind == ZKind::Boss {
            light_circle(z.pos, 3.0, Color::new(1.0, 0.25, 0.1, 1.0), 0.3);
        }
    }
    if let Some(t) = &w.train {
        light_cone(vec2(t.head, t.y), vec2(1.0, 0.0), 9.0, 0.35, Color::new(1.0, 0.95, 0.75, 1.0), 0.9);
        for i in (1..t.slices.len()).step_by(3) {
            let x = t.slice_x(i) + 0.5;
            if x < 1.0 {
                continue;
            }
            light_circle(vec2(x, t.y + 1.3), 1.9, Color::new(1.0, 0.92, 0.7, 1.0), 0.45);
            light_circle(vec2(x, t.y - 1.3), 1.9, Color::new(1.0, 0.92, 0.7, 1.0), 0.35);
        }
        if t.doors_open {
            let pulse = 0.7 + 0.3 * (w.time * 5.0).sin();
            for (i, s) in t.slices.iter().enumerate() {
                if *s == SliceK::Door {
                    let x = t.slice_x(i) + 0.5;
                    light_circle(vec2(x, t.y + 1.2), 1.5, Color::new(0.5, 1.0, 0.6, 1.0), pulse);
                    light_circle(vec2(x, t.y - 1.2), 1.5, Color::new(0.5, 1.0, 0.6, 1.0), pulse * 0.8);
                }
            }
        }
    }
    // glowing eyes: punch a few pixels of full light into the light map so the
    // horde stays readable in the dark (the eye pixels in the sprite then blow out)
    for z in &w.zombies {
        if z.back || z.age < 0.3 || z.kind == ZKind::Boss {
            continue;
        }
        let gz = w.map.ground_z(z.pos);
        let at = iso3(z.pos.x, z.pos.y, gz).round();
        if !view.contains(at) {
            continue;
        }
        let bob = ((z.anim as usize) % 4 % 2) as f32;
        let (ox, oy, dx) = if z.kind == ZKind::Brute { (-3.0, -18.0 + bob, 5.0) } else { (-2.0, -16.0 + bob, 3.0) };
        let (ex, ey) = if z.face_left { (at.x - ox - dx, at.y + oy) } else { (at.x + ox, at.y + oy) };
        let c = if z.kind == ZKind::Runner { Color::new(1.0, 0.5, 0.35, 1.0) } else { Color::new(1.0, 0.9, 0.55, 1.0) };
        draw_rectangle(ex - 1.0, ey - 1.0, dx + 3.0, 3.0, with_alpha(c, 0.18));
        draw_rectangle(ex, ey, 1.0, 1.0, c);
        draw_rectangle(ex + dx, ey, 1.0, 1.0, c);
    }
    if !w.demo && w.phase != Phase::Dead {
        let p = &w.player;
        light_circle(p.pos, if w.has_light && w.light_on { 2.3 } else { 1.6 }, Color::new(0.9, 0.9, 1.0, 1.0), if w.has_light && w.light_on { 0.32 } else { 0.2 });
        if w.has_light && w.light_on {
            // the beam leaves the hand and lands on the floor ahead of you
            light_cone(p.pos + p.aim * 0.55, p.aim, 7.5, 0.4, Color::new(1.0, 0.96, 0.85, 1.0), 0.75);
        }
    }
}
