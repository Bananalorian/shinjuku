//! HUD and menu screens, all drawn at full resolution with the 5x7 pixel font.

use crate::art::Art;
use crate::font;
use crate::input::{Input, Mode};
use crate::level::StationDef;
use crate::util::*;
use crate::world::*;
use macroquad::prelude::*;

pub const GREEN: Color = Color { r: 0.43, g: 0.72, b: 0.18, a: 1.0 };
pub const RED: Color = Color { r: 0.86, g: 0.17, b: 0.2, a: 1.0 };
pub const CREAM: Color = Color { r: 0.96, g: 0.94, b: 0.88, a: 1.0 };
pub const DIM: Color = Color { r: 0.62, g: 0.62, b: 0.66, a: 1.0 };

pub fn text_w(s: &str, px: f32) -> f32 {
    font::text_width(s) as f32 * px
}

pub fn text(art: &Art, s: &str, x: f32, y: f32, px: f32, color: Color) {
    text_ex(art, s, x, y, px, color, true);
}

pub fn text_ex(art: &Art, s: &str, x: f32, y: f32, px: f32, color: Color, drop_shadow: bool) {
    let px = px.max(1.0).round();
    let mut cx = x.round();
    let y = y.round();
    for ch in s.chars() {
        let up = ch.to_ascii_uppercase();
        if let Some(r) = art.glyphs.get(&up) {
            if color.a > 0.0 && drop_shadow {
                draw_texture_ex(&art.tex, cx + px, y + px, Color::new(0.0, 0.0, 0.0, color.a * 0.75), DrawTextureParams { source: Some(*r), dest_size: Some(vec2(5.0 * px, 7.0 * px)), ..Default::default() });
            }
            if color.a > 0.0 {
                draw_texture_ex(&art.tex, cx, y, color, DrawTextureParams { source: Some(*r), dest_size: Some(vec2(5.0 * px, 7.0 * px)), ..Default::default() });
            }
        }
        cx += 6.0 * px;
    }
}

pub fn text_c(art: &Art, s: &str, cx: f32, y: f32, px: f32, color: Color) {
    let px = px.max(1.0).round();
    text(art, s, cx - text_w(s, px) * 0.5, y, px, color);
}

/// Fit a line of text to a width by shrinking the pixel size if needed.
pub fn fit(s: &str, px: f32, max_w: f32) -> f32 {
    let mut p = px.round().max(1.0);
    while p > 1.0 && text_w(s, p) > max_w {
        p -= 1.0;
    }
    p
}

fn bar(x: f32, y: f32, w: f32, h: f32, k: f32, fg: Color, u: f32) {
    draw_rectangle(x - u, y - u, w + 2.0 * u, h + 2.0 * u, Color::new(0.0, 0.0, 0.0, 0.7));
    draw_rectangle(x, y, w, h, Color::new(0.18, 0.16, 0.2, 1.0));
    draw_rectangle(x, y, (w * k.clamp(0.0, 1.0)).round(), h, fg);
    draw_rectangle(x, y, (w * k.clamp(0.0, 1.0)).round(), u, shade(fg, 1.3));
}

fn station_badge(art: &Art, code: &str, x: f32, y: f32, u: f32) {
    let s = 21.0 * u;
    draw_rectangle(x - u, y - u, s + 2.0 * u, s + 2.0 * u, Color::new(0.0, 0.0, 0.0, 0.7));
    draw_rectangle(x, y, s, s, GREEN);
    draw_rectangle(x + 2.0 * u, y + 2.0 * u, s - 4.0 * u, s - 4.0 * u, WHITE);
    let (line, num) = code.split_at(2);
    text_ex(art, line, x + 5.0 * u, y + 3.0 * u, u, GREEN, false);
    text_ex(art, num, x + 5.0 * u, y + 11.0 * u, u, BLACK, false);
}

pub fn draw_hud(w: &World, art: &Art, u: f32, defs: &[StationDef], input: &Input) {
    let (sw, sh) = (screen_width(), screen_height());
    let m = 6.0 * u;

    // station + progress
    station_badge(art, w.code, m, m, u);
    let nx = m + 26.0 * u;
    text(art, w.name, nx, m, 2.0 * u, CREAM);
    let prog = (w.kills as f32 / w.quota as f32).min(1.0);
    let bw = 92.0 * u;
    bar(nx, m + 17.0 * u, bw, 3.0 * u, prog, GREEN, u);
    let label = match w.phase {
        Phase::Boss if w.story => "KILL IT".to_string(),
        Phase::Train => "BOARD THE TRAIN".to_string(),
        Phase::Boss => format!("KILL {}", w.boss_kind.name()),
        _ => "CLEAR THE STATION".to_string(), // the bar shows progress; the count stays hidden
    };
    text(art, &label, nx, m + 23.0 * u, u, DIM);

    // where you are on the loop
    if let Some((i, n)) = w.loop_pos {
        let s1 = format!("STOP {} OF {}", i + 1, n);
        text(art, &s1, sw - m - text_w(&s1, u), m, u, CREAM);
        let k = (i as f32 + 0.5) / n as f32;
        bar(sw - m - 80.0 * u, m + 10.0 * u, 80.0 * u, 2.0 * u, k, GREEN, u);
        if !w.next_name.is_empty() {
            let s2 = format!("NEXT {}", w.next_name);
            text(art, &s2, sw - m - text_w(&s2, u), m + 16.0 * u, u, DIM);
        }
    }
    // the route along the Yamanote line (arcade only)
    if !w.story {
    let n = defs.len();
    let gap = 15.0 * u;
    let rx = sw - m - gap * (n - 1) as f32 - 3.0 * u;
    let ry = m + 4.0 * u;
    draw_rectangle(rx, ry - u * 0.5, gap * (n - 1) as f32, u * 2.0, Color::new(0.0, 0.0, 0.0, 0.6));
    draw_rectangle(rx, ry, gap * (n - 1) as f32, u, GREEN);
    for i in 0..n {
        let cx = rx + gap * i as f32;
        let r = if i == w.station { 3.5 * u * (1.0 + 0.15 * (w.time * 5.0).sin()) } else { 2.5 * u };
        let col = if i < w.station { GREEN } else if i == w.station { CREAM } else { Color::new(0.25, 0.25, 0.28, 1.0) };
        draw_circle(cx, ry + u * 0.5, r + u, BLACK);
        draw_circle(cx, ry + u * 0.5, r, col);
    }
    if w.station + 1 < n {
        let s = format!("NEXT {}", defs[w.station + 1].name);
        text(art, &s, sw - m - text_w(&s, u), ry + 7.0 * u, u, DIM);
    } else {
        let s = "END OF THE LINE";
        text(art, s, sw - m - text_w(s, u), ry + 7.0 * u, u, RED);
    }

    }
    // health, grenade, dash
    let by = sh - m - 4.0 * u;
    let hk = w.player.hp / w.stats.max_hp;
    let hcol = if hk < 0.3 { mix(RED, WHITE, (w.time * 8.0).sin() * 0.5 + 0.5) } else { RED };
    text(art, "HP", m, by - 9.0 * u, u, CREAM);
    bar(m, by, 70.0 * u, 4.0 * u, hk, hcol, u);
    let gk = 1.0 - (w.player.gren_cd / w.stats.gren_cd).clamp(0.0, 1.0);
    let gx = m + 78.0 * u;
    text(art, "NADE", gx, by - 9.0 * u, u, if gk >= 1.0 { CREAM } else { DIM });
    bar(gx, by, 23.0 * u, 4.0 * u, gk, Color::new(1.0, 0.6, 0.2, 1.0), u);
    let dk = 1.0 - (w.player.dash_cd / w.stats.dash_cd).clamp(0.0, 1.0);
    let dx = gx + 31.0 * u;
    text(art, "DASH", dx, by - 9.0 * u, u, if dk >= 1.0 { CREAM } else { DIM });
    bar(dx, by, 23.0 * u, 4.0 * u, dk, Color::new(0.35, 0.9, 1.0, 1.0), u);
    let ks = format!("{} KILLS", w.total_kills);
    text(art, &ks, sw - m - text_w(&ks, u), by - 2.0 * u, u, DIM);
    if w.story {
        let cs = format!("{}", w.coins);
        let cx = sw - m - text_w(&cs, 2.0 * u);
        let cy = by - 22.0 * u;
        text(art, &cs, cx, cy, 2.0 * u, Color::new(1.0, 0.85, 0.3, 1.0));
        draw_texture_ex(&art.tex, cx - 14.0 * u, cy + 2.0 * u, WHITE, DrawTextureParams { source: Some(art.coin), dest_size: Some(vec2(10.0 * u, 10.0 * u)), ..Default::default() });
    }

    // boss bar
    if let Some(b) = w.boss() {
        let bw = (sw * 0.5).min(200.0 * u);
        let bx = (sw - bw) * 0.5;
        let byy = m + 48.0 * u;
        text_c(art, w.boss_kind.name(), sw * 0.5, byy - 9.0 * u, u, RED);
        bar(bx, byy, bw, 4.0 * u, b.hp / b.max_hp, RED, u);
    }

    // big center message
    if let Some(msg) = &w.msg {
        let a = (msg.t * 4.0).min(1.0) * ((msg.dur - msg.t) * 2.0).clamp(0.0, 1.0);
        let px = fit(&msg.title, 3.0 * u, sw * 0.9);
        let y = sh * 0.3;
        text_c(art, &msg.title, sw * 0.5, y, px, with_alpha(CREAM, a));
        if !msg.sub.is_empty() {
            let spx = fit(&msg.sub, u, sw * 0.9);
            text_c(art, &msg.sub, sw * 0.5, y + 28.0 * u, spx, with_alpha(DIM, a));
        }
    }

    if input.touch_mode() {
        draw_touch(art, input, u);
    }
}

fn draw_touch(art: &Art, input: &Input, u: f32) {
    let r = input.stick_radius();
    for s in [&input.left, &input.right] {
        if s.id.is_some() {
            draw_circle_lines(s.origin.x, s.origin.y, r, 2.0, Color::new(1.0, 1.0, 1.0, 0.25));
            let k = s.origin + (s.pos - s.origin).clamp_length_max(r);
            draw_circle(k.x, k.y, r * 0.38, Color::new(1.0, 1.0, 1.0, 0.25));
        }
    }
    for ((c, br), label, vis) in [(input.dash_btn, "DASH", input.dash_pressed_vis), (input.bomb_btn, "NADE", input.bomb_pressed_vis), (input.light_btn, "LIGHT", 0.0), (input.use_btn, "USE", 0.0)] {
        let a = if vis > 0.0 { 0.45 } else { 0.18 };
        draw_circle(c.x, c.y, br, Color::new(1.0, 1.0, 1.0, a * 0.5));
        draw_circle_lines(c.x, c.y, br, 2.0, Color::new(1.0, 1.0, 1.0, a + 0.15));
        let px = fit(label, u, br * 1.6);
        text_c(art, label, c.x, c.y - 3.5 * px, px, Color::new(1.0, 1.0, 1.0, 0.8));
    }
}

pub fn draw_title(art: &Art, u: f32, t: f32, defs: &[StationDef], mode: Mode, sel: usize) -> Vec<Rect> {
    let (sw, sh) = (screen_width(), screen_height());
    draw_rectangle(0.0, 0.0, sw, sh, Color::new(0.0, 0.0, 0.0, 0.35));
    let big = fit("TO SHINJUKU", 4.0 * u, sw * 0.9);
    let y = sh * 0.13;
    text_c(art, "LAST TRAIN", sw * 0.5, y, big, CREAM);
    text_c(art, "TO SHINJUKU", sw * 0.5, y + 9.0 * big, big, RED);
    let sub = "30 STATIONS. ONE LOOP. ONE TRAIN.";
    text_c(art, sub, sw * 0.5, y + 18.0 * big + 4.0 * u, fit(sub, u, sw * 0.92), DIM);

    // route list
    let _ = defs;
    let line = "KANDA > AKIHABARA > UENO > IKEBUKURO > SHINJUKU > SHIBUYA > SHINAGAWA > TOKYO > ...".to_string();
    let lp = fit(&line, u, sw * 0.92);
    text_c(art, &line, sw * 0.5, sh * 0.55, lp, GREEN);

    let help = match mode {
        _ if sh < 0.0 => "",
        Mode::Touch => "LEFT STICK MOVE   RIGHT STICK AIM   AUTO-AIM WHEN IDLE",
        Mode::Pad => "L-STICK MOVE   R-STICK AIM + FIRE   A DASH   RB GRENADE   START PAUSE",
        Mode::Mouse => "WASD MOVE   MOUSE AIM + FIRE   SPACE DASH   RIGHT CLICK GRENADE",
    };
    text_c(art, help, sw * 0.5, sh * 0.62, fit(help, u, sw * 0.92), CREAM);
    let opts = match mode {
        Mode::Touch => "",
        Mode::Pad => "Y TILT-SHIFT   BACK MUTE",
        Mode::Mouse => "T TILT-SHIFT   M MUTE   ESC PAUSE",
    };
    if !opts.is_empty() {
        text_c(art, opts, sw * 0.5, sh * 0.62 + 11.0 * u, fit(opts, u, sw * 0.92), DIM);
    }
    // the menu
    let items = [("NEW GAME", "FROM KANDA, ALL THE WAY AROUND THE YAMANOTE LOOP"), ("ARCADE", "THE ORIGINAL RUN: 5 STATIONS, UPGRADE CARDS")];
    let mut rects = Vec::new();
    for (i, (name, desc)) in items.iter().enumerate() {
        let y = sh * 0.73 + i as f32 * 18.0 * u;
        let px = fit(name, 2.0 * u, sw * 0.5);
        let w = text_w(name, px) + 16.0 * u;
        let r = Rect::new(sw * 0.5 - w * 0.5, y - 3.0 * u, w, 7.0 * px + 6.0 * u);
        let on = i == sel;
        if on {
            draw_rectangle(r.x, r.y, r.w, r.h, Color::new(0.43, 0.72, 0.18, 0.35 + 0.15 * (t * 4.0).sin()));
        }
        text_c(art, name, sw * 0.5, y, px, if on { CREAM } else { DIM });
        if on {
            text_c(art, desc, sw * 0.5, sh * 0.73 + 40.0 * u, fit(desc, u, sw * 0.9), GREEN);
        }
        rects.push(r);
    }
    let _ = (mode, t);
    if sh > sw {
        let s = "TIP: ROTATE YOUR PHONE";
        text_c(art, s, sw * 0.5, sh * 0.95, fit(s, u, sw * 0.9), DIM);
    }
    rects
}


/// The ride between stations: tunnel lights streaking past and three upgrade cards.
/// Returns the screen rects of the cards so taps can be matched.
pub fn draw_upgrade(art: &Art, u: f32, t: f32, next: &StationDef, choices: &[usize; 3], hover: Option<usize>, mode: Mode) -> Vec<Rect> {
    let (sw, sh) = (screen_width(), screen_height());
    clear_background(Color::new(0.03, 0.03, 0.05, 1.0));
    // tunnel lights rushing by the window
    for i in 0..9 {
        let speed = 900.0 * u / 3.0;
        let x = sw - ((t * speed + i as f32 * sw / 4.5) % (sw * 2.0));
        let y = sh * (0.12 + (i % 3) as f32 * 0.035);
        draw_rectangle(x, y, 40.0 * u, u * 1.5, Color::new(1.0, 0.85, 0.55, 0.6));
        draw_rectangle(x - 60.0 * u, y, 60.0 * u, u * 1.5, Color::new(1.0, 0.85, 0.55, 0.12));
    }
    let ups = upgrades();
    let title = format!("NEXT STOP: {}", next.name);
    text_c(art, &title, sw * 0.5, sh * 0.24, fit(&title, 3.0 * u, sw * 0.9), CREAM);
    text_c(art, next.code, sw * 0.5, sh * 0.24 + 26.0 * u, u, GREEN);
    let pick = "PICK ONE UPGRADE";
    text_c(art, pick, sw * 0.5, sh * 0.24 + 38.0 * u, u, DIM);

    let horizontal = sw > sh * 1.1;
    let mut rects = Vec::new();
    let (cw, ch) = if horizontal { (((sw - 40.0 * u) / 3.0).min(140.0 * u), 38.0 * u) } else { ((sw * 0.86).min(170.0 * u), 34.0 * u) };
    // one text size for every card so they read as a set
    let mut tp = 2.0 * u;
    for &ci in choices {
        tp = tp.min(fit(ups[ci].name, 2.0 * u, cw - 20.0 * u)).min(fit(ups[ci].desc, 2.0 * u, cw - 20.0 * u) * 1.0);
    }
    let dp = (tp * 0.5).round().max(u.min(tp));
    for (k, &ci) in choices.iter().enumerate() {
        let (x, y) = if horizontal {
            let total = cw * 3.0 + 10.0 * u * 2.0;
            ((sw - total) * 0.5 + k as f32 * (cw + 10.0 * u), sh * 0.52)
        } else {
            ((sw - cw) * 0.5, sh * 0.45 + k as f32 * (ch + 8.0 * u))
        };
        let r = Rect::new(x, y, cw, ch);
        let hot = hover == Some(k);
        draw_rectangle(x - u, y - u, cw + 2.0 * u, ch + 2.0 * u, if hot { GREEN } else { Color::new(0.3, 0.3, 0.34, 1.0) });
        draw_rectangle(x, y, cw, ch, Color::new(0.09, 0.09, 0.12, 1.0));
        draw_rectangle(x, y, cw, 3.0 * u, GREEN);
        let up = &ups[ci];
        let num = format!("{}", k + 1);
        text(art, &num, x + 5.0 * u, y + 9.0 * u, tp, GREEN);
        text(art, up.name, x + 14.0 * u, y + 9.0 * u, tp, CREAM);
        text(art, up.desc, x + 14.0 * u, y + 9.0 * u + 10.0 * tp, dp.min(tp), DIM);
        rects.push(r);
    }
    let hint = match mode {
        Mode::Touch => "TAP AN UPGRADE",
        Mode::Pad => "D-PAD TO CHOOSE   A TO PICK",
        Mode::Mouse => "CLICK ONE OR PRESS 1-3",
    };
    text_c(art, hint, sw * 0.5, sh * 0.9, fit(hint, u, sw * 0.9), DIM);
    rects
}

pub fn draw_pause(art: &Art, u: f32, mode: Mode, tilt: bool, muted: bool) {
    let (sw, sh) = (screen_width(), screen_height());
    draw_rectangle(0.0, 0.0, sw, sh, Color::new(0.0, 0.0, 0.0, 0.55));
    text_c(art, "PAUSED", sw * 0.5, sh * 0.32, fit("PAUSED", 4.0 * u, sw * 0.9), CREAM);
    let s = if mode == Mode::Pad { "PRESS START TO RESUME" } else { "PRESS ESC OR P TO RESUME" };
    text_c(art, s, sw * 0.5, sh * 0.32 + 40.0 * u, fit(s, u, sw * 0.9), DIM);
    let (tk, mk) = if mode == Mode::Pad { ("Y", "BACK") } else { ("T", "M") };
    let onoff = |b: bool| if b { "ON" } else { "OFF" };
    let l1 = format!("{}   TILT-SHIFT  {}", tk, onoff(tilt));
    let l2 = format!("{}   SOUND  {}", mk, onoff(!muted));
    text_c(art, &l1, sw * 0.5, sh * 0.32 + 60.0 * u, fit(&l1, u, sw * 0.9), GREEN);
    text_c(art, &l2, sw * 0.5, sh * 0.32 + 72.0 * u, fit(&l2, u, sw * 0.9), GREEN);
}

pub fn draw_end(art: &Art, u: f32, t: f32, title: &str, lines: &[String], prompt: &str, color: Color) {
    let (sw, sh) = (screen_width(), screen_height());
    let a = (t * 1.5).min(1.0);
    draw_rectangle(0.0, 0.0, sw, sh, Color::new(0.0, 0.0, 0.0, 0.55 * a));
    let px = fit(title, 4.0 * u, sw * 0.9);
    text_c(art, title, sw * 0.5, sh * 0.25, px, with_alpha(color, a));
    for (i, l) in lines.iter().enumerate() {
        text_c(art, l, sw * 0.5, sh * 0.25 + 40.0 * u + i as f32 * 12.0 * u, fit(l, u, sw * 0.9), with_alpha(CREAM, a));
    }
    if t > 1.0 && (t * 2.0) as i32 % 2 == 0 {
        text_c(art, prompt, sw * 0.5, sh * 0.8, fit(prompt, u * 2.0, sw * 0.9), CREAM);
    }
}
