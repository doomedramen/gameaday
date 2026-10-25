//! Race scene and HUD. Layers: pre-rendered ground bitmap (with skid marks
//! stamped in as they happen), pads/pickups/cones, car shadows and cars,
//! particles, high scenery with height parallax, cloud shadows or night
//! lighting, world callouts, then the HUD.

use crate::assets;
use crate::canvas::{Align, Blit, Canvas, H, W, mix};
use crate::font::Face;
use crate::math::{V2, v2};
use crate::race::{Kind, Mode, PKind, Phase, Race};
use crate::track::{Biome, Bitmap, PickupKind};
use std::f32::consts::{FRAC_PI_2, PI};

pub const C_PANEL: u32 = 0x1B2233;
pub const C_TEXT: u32 = 0xF4F6FA;
pub const C_DIM: u32 = 0xA9B3C6;
pub const C_ACCENT: u32 = 0xFF8A3D;
pub const C_CYAN: u32 = 0x5BD8FF;
pub const C_GOLD: u32 = 0xFFD34A;
pub const C_GREEN: u32 = 0x6BE38A;
pub const C_RED: u32 = 0xFF5A5A;

const LW: usize = W / 2;
const LH: usize = H / 2;

pub struct View {
    pub bitmap: Bitmap,
    pub cam: V2,
    pub time: f32,
    noise: Vec<f32>,
    light: Vec<[f32; 3]>,
    mini: Vec<V2>,
    mini_scale: f32,
    mini_off: V2,
    pub shake_on: bool,
    ground_avg: u32,
}

fn value_noise(seed: u64) -> Vec<f32> {
    let mut rng = crate::rng::Rng::new(seed);
    let g: Vec<f32> = (0..16 * 16).map(|_| rng.f()).collect();
    let mut out = vec![0.0; 128 * 128];
    for y in 0..128 {
        for x in 0..128 {
            let fx = x as f32 / 8.0;
            let fy = y as f32 / 8.0;
            let (ix, iy) = (fx.floor() as usize, fy.floor() as usize);
            let (tx, ty) = (fx - ix as f32, fy - iy as f32);
            let s = |a: f32| a * a * (3.0 - 2.0 * a);
            let at = |i: usize, j: usize| g[(j % 16) * 16 + (i % 16)];
            let a = at(ix, iy) + (at(ix + 1, iy) - at(ix, iy)) * s(tx);
            let b = at(ix, iy + 1) + (at(ix + 1, iy + 1) - at(ix, iy + 1)) * s(tx);
            out[y * 128 + x] = a + (b - a) * s(ty);
        }
    }
    out
}

impl View {
    pub fn new(race: &Race) -> View {
        let bitmap = race.track.render_bitmap();
        let t = &race.track;
        let (bw, bh) = (230.0, 170.0);
        let sx = bw / (t.max.x - t.min.x - 2.0 * (crate::track::WALL + 420.0));
        let sy = bh / (t.max.y - t.min.y - 2.0 * (crate::track::WALL + 420.0));
        let k = sx.min(sy);
        let inner_min = t.min + v2(crate::track::WALL + 420.0, crate::track::WALL + 420.0);
        let span = (t.max - t.min)
            - v2(
                2.0 * (crate::track::WALL + 420.0),
                2.0 * (crate::track::WALL + 420.0),
            );
        let off = v2((bw - span.x * k) / 2.0, (bh - span.y * k) / 2.0) - inner_min * k;
        let mini: Vec<V2> = t.pts.iter().step_by(6).map(|p| *p * k + off).collect();
        let ground_avg = assets::get().s(t.spec.biome.ground()).average();
        View {
            bitmap,
            cam: race.player().car.pos,
            time: 0.0,
            noise: value_noise(t.spec.seed),
            light: vec![[0.0; 3]; LW * LH],
            mini,
            mini_scale: k,
            mini_off: off,
            shake_on: true,
            ground_avg,
        }
    }

    /// Stamp pending skid marks into the ground and move the camera.
    pub fn update(&mut self, race: &mut Race, dt: f32) {
        self.time += dt;
        for (p0, p1, a, c) in race.skids.drain(..) {
            let (q0, q1) = (p0 - self.bitmap.origin, p1 - self.bitmap.origin);
            let n = (q0.dist(q1) / 2.0).ceil().clamp(1.0, 20.0) as usize;
            for k in 0..n {
                let q = q0.lerp(q1, (k as f32 + 0.5) / n as f32);
                stamp(
                    &mut self.bitmap,
                    q.x,
                    q.y,
                    3.2,
                    c,
                    a / (n as f32).sqrt().max(1.0) * 0.8,
                );
            }
        }
        let car = &race.player().car;
        let ahead = car.vel * 0.3;
        let ahead = if ahead.len() > 230.0 {
            ahead.norm() * 230.0
        } else {
            ahead
        };
        let target = car.pos + ahead;
        let k = (1.0 - (-6.0 * dt).exp()).min(1.0);
        self.cam = self.cam.lerp(target, k);
    }

    pub fn snap(&mut self, race: &Race) {
        self.cam = race.player().car.pos;
    }

    fn noise_at(&self, x: f32, y: f32) -> f32 {
        let (fx, fy) = (x.floor(), y.floor());
        let (tx, ty) = (x - fx, y - fy);
        let at = |i: i64, j: i64| {
            self.noise[(j.rem_euclid(128) as usize) * 128 + i.rem_euclid(128) as usize]
        };
        let (i, j) = (fx as i64, fy as i64);
        let a = at(i, j) + (at(i + 1, j) - at(i, j)) * tx;
        let b = at(i, j + 1) + (at(i + 1, j + 1) - at(i, j + 1)) * tx;
        a + (b - a) * ty
    }
}

fn stamp(bm: &mut Bitmap, cx: f32, cy: f32, r: f32, c: u32, a: f32) {
    let x0 = (cx - r).floor().max(0.0) as i32;
    let y0 = (cy - r).floor().max(0.0) as i32;
    let x1 = ((cx + r).ceil() as i32).min(bm.w as i32 - 1);
    let y1 = ((cy + r).ceil() as i32).min(bm.h as i32 - 1);
    for y in y0..=y1 {
        for x in x0..=x1 {
            let d = ((x as f32 + 0.5 - cx).powi(2) + (y as f32 + 0.5 - cy).powi(2)).sqrt();
            let cov = (r - d).clamp(0.0, 1.0) * a;
            if cov > 0.0 {
                let i = y as usize * bm.w + x as usize;
                bm.px[i] = mix(bm.px[i], c, cov);
            }
        }
    }
}

pub fn ordinal(n: usize) -> &'static str {
    match n {
        1 => "1st",
        2 => "2nd",
        3 => "3rd",
        4 => "4th",
        5 => "5th",
        6 => "6th",
        7 => "7th",
        8 => "8th",
        _ => "9th",
    }
}

pub fn fmt_time(t: f32) -> String {
    let m = (t / 60.0).floor() as i32;
    let s = t - m as f32 * 60.0;
    format!("{m}:{s:05.2}")
}

pub struct RenderOpts {
    pub hints: bool,
    pub paused: bool,
}

/// Draw the whole race frame.
pub fn draw_race(cv: &mut Canvas, race: &Race, view: &mut View, o: &RenderOpts) {
    draw(cv, race, view, o, true);
}

/// The world without HUD (menu backdrops).
pub fn draw_scene_only(cv: &mut Canvas, race: &Race, view: &mut View) {
    draw(
        cv,
        race,
        view,
        &RenderOpts {
            hints: false,
            paused: false,
        },
        false,
    );
}

fn draw(cv: &mut Canvas, race: &Race, view: &mut View, o: &RenderOpts, hud: bool) {
    let a = assets::get();
    let t = view.time;
    let biome = race.track.spec.biome;
    let mut cam = view.cam;
    if view.shake_on && race.shake > 0.0 {
        let s = race.shake * race.shake * 16.0;
        cam += v2((t * 61.0).sin() * s, (t * 47.0).cos() * s);
    }
    let ox = (cam.x - W as f32 / 2.0).round();
    let oy = (cam.y - H as f32 / 2.0).round();
    let to_screen = |p: V2| v2(p.x - ox, p.y - oy);

    // --- ground
    {
        let bm = &view.bitmap;
        let bx = (ox - bm.origin.x) as i64;
        let by = (oy - bm.origin.y) as i64;
        let fill = view.ground_avg;
        for y in 0..H {
            let wy = by + y as i64;
            let row = &mut cv.px[y * W..(y + 1) * W];
            if wy < 0 || wy >= bm.h as i64 {
                row.fill(fill);
                continue;
            }
            let src = &bm.px[wy as usize * bm.w..(wy as usize + 1) * bm.w];
            for (x, px) in row.iter_mut().enumerate() {
                let wx = bx + x as i64;
                *px = if wx < 0 || wx >= bm.w as i64 {
                    fill
                } else {
                    src[wx as usize]
                };
            }
        }
    }

    let on = |p: V2, m: f32| p.x > -m && p.y > -m && p.x < W as f32 + m && p.y < H as f32 + m;

    // --- boost pads, pickups, cones
    for pad in &race.track.pads {
        let p = to_screen(pad.pos);
        if !on(p, 80.0) {
            continue;
        }
        let pulse = 0.5 + 0.5 * (t * 6.0).sin();
        cv.glow(p.x, p.y, 46.0, 0xFFC93C, 0.18 + 0.12 * pulse);
        for k in 0..2 {
            let q = p + V2::from_angle(pad.angle) * (k as f32 * 22.0 - 11.0);
            cv.blit(
                a.s("arrow_yellow"),
                q.x,
                q.y,
                Blit {
                    rot: pad.angle + FRAC_PI_2,
                    scale: 0.62,
                    alpha: 0.85 + 0.15 * pulse,
                    ..Blit::default()
                },
            );
        }
    }
    for pk in &race.pickups {
        if pk.respawn > 0.0 {
            continue;
        }
        let p = to_screen(pk.pos);
        if !on(p, 40.0) {
            continue;
        }
        let bob = (t * 4.0 + pk.pos.x * 0.01).sin() * 2.5;
        match pk.kind {
            PickupKind::Nitro => {
                cv.glow(p.x, p.y, 30.0, C_CYAN, 0.35);
                cv.blit(
                    a.s("barrel_blue"),
                    p.x + 3.0,
                    p.y + 4.0,
                    Blit {
                        scale: 0.75,
                        alpha: 0.3,
                        solid: Some(0x102030),
                        ..Blit::default()
                    },
                );
                cv.blit(
                    a.s("barrel_blue"),
                    p.x,
                    p.y + bob,
                    Blit {
                        scale: 0.75,
                        rot: t * 1.5,
                        ..Blit::default()
                    },
                );
                cv.text(
                    "N",
                    p.x,
                    p.y + bob - 8.0,
                    14.0,
                    Face::Bold,
                    C_TEXT,
                    1.0,
                    Align::Center,
                );
            }
            PickupKind::Repair => {
                cv.glow(p.x, p.y, 30.0, C_GREEN, 0.35);
                cv.circle(p.x + 2.0, p.y + 3.0, 14.0, 0x102030, 0.3);
                cv.circle(p.x, p.y + bob, 14.0, 0xF4F6FA, 1.0);
                cv.fill_rect(p.x - 3.0, p.y + bob - 9.0, 6.0, 18.0, 0x3DBB6A, 1.0);
                cv.fill_rect(p.x - 9.0, p.y + bob - 3.0, 18.0, 6.0, 0x3DBB6A, 1.0);
            }
        }
    }
    for c in &race.cones {
        let p = to_screen(c.pos);
        if !on(p, 30.0) {
            continue;
        }
        let spr = if c.down { "cone_down" } else { "cone_straight" };
        cv.blit(
            a.s(spr),
            p.x,
            p.y,
            Blit {
                rot: c.rot,
                ..Blit::default()
            },
        );
    }

    // --- ghost
    if let Some((gp, gh)) = race.ghost_pose() {
        let p = to_screen(gp);
        if on(p, 60.0) {
            cv.blit(
                a.s(race.player().driver.sprite),
                p.x,
                p.y,
                Blit {
                    rot: gh + FRAC_PI_2,
                    alpha: 0.38,
                    tint: 0x9FD8FF,
                    ..Blit::default()
                },
            );
        }
    }

    // --- cars
    let mut order: Vec<usize> = (0..race.racers.len()).collect();
    order.sort_by_key(|&i| race.racers[i].is_player());
    for &i in &order {
        let r = &race.racers[i];
        let car = &r.car;
        let p = to_screen(car.pos);
        if !on(p, 80.0) {
            continue;
        }
        let spr = a.s(r.driver.sprite);
        let rot = car.heading + FRAC_PI_2;
        let scale = 1.0 + car.bob * 0.012;
        if r.driver.kind == Kind::Champion {
            cv.glow(p.x, p.y, 54.0, C_GOLD, 0.25 + 0.1 * (t * 3.0).sin());
        }
        if car.drifting && car.drift_tier() > 0 {
            let col = [0, 0x5BC8FF, 0xFF9A3C, 0xC86BFF][car.drift_tier() as usize];
            cv.glow(p.x, p.y, 48.0, col, 0.35);
        }
        cv.blit(
            spr,
            p.x + 5.0,
            p.y + 7.0,
            Blit {
                rot,
                scale,
                alpha: 0.32,
                solid: Some(0x0D1520),
                ..Blit::default()
            },
        );
        cv.blit(
            spr,
            p.x,
            p.y,
            Blit {
                rot,
                scale,
                flash: car.hit_flash * 0.7,
                ..Blit::default()
            },
        );
        // brake lights
        if r.controls.throttle < -0.1 && car.fwd_speed() > 20.0 {
            let f = car.fwd();
            let rt = car.right();
            for side in [-10.0f32, 10.0] {
                let q = p - f * 31.0 + rt * side;
                cv.glow(q.x, q.y, 10.0, 0xFF2020, 0.9);
            }
        }
        if race.racers[i].ai.telegraphing() || race.racers[i].ai.ramming() {
            let pulse = 0.5 + 0.5 * (t * 18.0).sin();
            cv.ring(p.x, p.y, 40.0 + pulse * 6.0, 4.0, C_RED, 0.8);
            cv.circle(p.x, p.y - 58.0, 15.0, C_RED, 0.95);
            cv.text(
                "!",
                p.x,
                p.y - 72.0,
                22.0,
                Face::Bold,
                C_TEXT,
                1.0,
                Align::Center,
            );
        }
    }

    // --- particles
    for pt in &race.particles {
        let p = to_screen(pt.pos);
        if !on(p, 30.0) {
            continue;
        }
        let f = pt.life / pt.max;
        match pt.kind {
            PKind::Smoke | PKind::Dust => {
                let size = pt.size * (1.0 + (1.0 - f) * 1.6);
                cv.circle(p.x, p.y, size, pt.color, 0.45 * f);
            }
            PKind::Spark | PKind::Star => {
                let tail = pt.vel * 0.03;
                cv.line(
                    p.x,
                    p.y,
                    p.x - tail.x,
                    p.y - tail.y,
                    pt.size,
                    pt.color,
                    f.min(1.0),
                );
                cv.glow(p.x, p.y, pt.size * 3.0, pt.color, 0.4 * f);
            }
            PKind::Flame => {
                cv.glow(p.x, p.y, pt.size * (0.6 + f), pt.color, 0.8 * f);
                cv.circle(p.x, p.y, pt.size * 0.4 * f, 0xFFFFFF, 0.8 * f);
            }
            PKind::Confetti => {
                let w = pt.size * (pt.rot * 3.0).sin().abs().max(0.2);
                cv.fill_rect(
                    p.x - w,
                    p.y - pt.size * 0.6,
                    w * 2.0,
                    pt.size * 1.2,
                    pt.color,
                    f.min(1.0),
                );
            }
            PKind::Wind => {
                let d = V2::from_angle(pt.rot) * pt.size;
                cv.line(p.x, p.y, p.x - d.x, p.y - d.y, 2.0, pt.color, 0.55 * f);
            }
        }
    }

    // --- high scenery with parallax (drawn above cars)
    let center = v2(W as f32 / 2.0, H as f32 / 2.0);
    for pr in &race.track.props {
        if pr.height <= 0.0 {
            continue;
        }
        let base = to_screen(pr.pos);
        if !on(base, 180.0) {
            continue;
        }
        let spr = a.s(pr.sprite);
        let lift = (base - center) * (pr.height * 0.07);
        let p = base + lift;
        cv.blit(
            spr,
            base.x + 10.0,
            base.y + 14.0,
            Blit {
                rot: pr.rot,
                scale: pr.scale,
                alpha: 0.22,
                solid: Some(0x0D1A20),
                ..Blit::default()
            },
        );
        let sway = if pr.sprite.starts_with("tree") {
            (t * 1.3 + pr.pos.x * 0.01).sin() * 0.03
        } else {
            0.0
        };
        cv.blit(
            spr,
            p.x,
            p.y,
            Blit {
                rot: pr.rot + sway,
                scale: pr.scale,
                ..Blit::default()
            },
        );
    }
    // lamp posts
    for lp in &race.track.lamps {
        let base = to_screen(*lp);
        if !on(base, 60.0) {
            continue;
        }
        let p = base + (base - center) * 0.07;
        cv.line(base.x, base.y, p.x, p.y, 5.0, 0x2A2E38, 1.0);
        cv.circle(p.x, p.y, 7.0, 0x3A3F4C, 1.0);
        cv.circle(p.x, p.y, 4.0, 0xFFF4C8, 1.0);
    }
    // spectators by the grandstands wave
    for (k, pr) in race.track.props.iter().enumerate() {
        if pr.sprite != "tribune_full" {
            continue;
        }
        let base = to_screen(pr.pos);
        if !on(base, 200.0) {
            continue;
        }
        let f = V2::from_angle(pr.rot);
        let side = f.perp();
        for j in 0..6 {
            let q = base + f * (j as f32 * 30.0 - 75.0) + side * -66.0;
            let jump = ((t * 7.0 + (j + k) as f32 * 1.7).sin().max(0.0))
                * if race.phase == Phase::Cooldown {
                    6.0
                } else {
                    2.5
                };
            let names = [
                "character_black_blue",
                "character_blonde_red",
                "character_brown_green",
                "character_black_white",
                "character_blonde_green",
                "character_brown_red",
            ];
            cv.blit(
                a.s(names[(j + k) % names.len()]),
                q.x,
                q.y - jump,
                Blit {
                    rot: pr.rot + PI,
                    ..Blit::default()
                },
            );
        }
    }

    // --- atmosphere
    if biome.night() {
        night_light(cv, race, view, ox, oy);
    } else {
        cloud_shadows(cv, view, ox, oy, biome);
    }

    // --- world callouts
    for pu in &race.popups {
        let p = to_screen(pu.pos);
        let rise = pu.t * 50.0;
        let a = (1.0 - pu.t / if pu.big { 1.6 } else { 1.1 }).clamp(0.0, 1.0);
        let size = if pu.big { 34.0 } else { 18.0 };
        cv.text_outline(
            &pu.text,
            p.x,
            p.y - 60.0 - rise,
            size,
            Face::Bold,
            pu.color,
            0x1A1F2B,
            a,
            Align::Center,
        );
    }
    if !hud {
        return;
    }
    // name tags for nearby rivals
    let pp = race.player().car.pos;
    for r in &race.racers {
        if r.is_player() {
            continue;
        }
        let d = r.car.pos.dist(pp);
        if d > 420.0 {
            continue;
        }
        let p = to_screen(r.car.pos);
        let alpha = (1.0 - d / 420.0).min(0.85);
        let w = cv.text_width(r.driver.name, 13.0, Face::Body) + 18.0;
        cv.round_rect(
            p.x - w / 2.0,
            p.y + 36.0,
            w,
            18.0,
            9.0,
            C_PANEL,
            0.6 * alpha,
        );
        cv.circle(p.x - w / 2.0 + 9.0, p.y + 45.0, 4.0, r.driver.color, alpha);
        cv.text(
            r.driver.name,
            p.x + 4.0,
            p.y + 38.0,
            13.0,
            Face::Body,
            C_TEXT,
            alpha,
            Align::Center,
        );
    }

    // speed lines while boosting
    let me = &race.player().car;
    if me.boosting() && me.speed() > 450.0 {
        let k = ((me.speed() - 450.0) / 300.0).clamp(0.0, 1.0);
        let c = v2(W as f32 / 2.0, H as f32 / 2.0);
        for i in 0..28 {
            let a = (i as f32 * 2.399 + (t * 9.0).floor() * 0.7) % std::f32::consts::TAU;
            let d = V2::from_angle(a);
            let r0 = 420.0 + ((i * 37) % 11) as f32 * 18.0;
            let p0 = c + v2(d.x * r0 * 1.4, d.y * r0 * 0.8);
            let p1 = c + v2(d.x * (r0 + 160.0) * 1.4, d.y * (r0 + 160.0) * 0.8);
            cv.line(
                p0.x,
                p0.y,
                p1.x,
                p1.y,
                2.0,
                if me.nitro_on { 0xCFF3FF } else { 0xFFFFFF },
                0.35 * k,
            );
        }
    }
    if race.flash > 0.0 {
        cv.fill_rect(0.0, 0.0, W as f32, H as f32, 0xFFFFFF, race.flash * 0.35);
    }
    draw_hud(cv, race, view, o);
}

fn cloud_shadows(cv: &mut Canvas, view: &View, ox: f32, oy: f32, biome: Biome) {
    let t = view.time;
    let strength = match biome {
        Biome::DuneSea => 0.08,
        Biome::Timberline => 0.16,
        _ => 0.12,
    };
    for by in (0..H).step_by(4) {
        for bx in (0..W).step_by(4) {
            let wx = (ox + bx as f32) / 36.0 + t * 0.9;
            let wy = (oy + by as f32) / 36.0 + t * 0.45;
            let n = view.noise_at(wx, wy);
            let s = ((n - 0.52) * 4.0).clamp(0.0, 1.0) * strength;
            if s <= 0.003 {
                continue;
            }
            let k = ((1.0 - s) * 256.0) as u32;
            for y in by..(by + 4).min(H) {
                for x in bx..(bx + 4).min(W) {
                    let p = &mut cv.px[y * W + x];
                    let rb = ((*p & 0xFF00FF) * k >> 8) & 0xFF00FF;
                    let g = ((*p & 0x00FF00) * k >> 8) & 0x00FF00;
                    *p = rb | g;
                }
            }
        }
    }
}

fn add_light(
    buf: &mut [[f32; 3]],
    cx: f32,
    cy: f32,
    r: f32,
    col: [f32; 3],
    k: f32,
    cone: Option<(V2, f32)>,
) {
    // works in half resolution
    let (cx, cy, r) = (cx / 2.0, cy / 2.0, r / 2.0);
    let x0 = (cx - r).floor().max(0.0) as i32;
    let y0 = (cy - r).floor().max(0.0) as i32;
    let x1 = ((cx + r).ceil() as i32).min(LW as i32);
    let y1 = ((cy + r).ceil() as i32).min(LH as i32);
    for y in y0..y1 {
        for x in x0..x1 {
            let dx = x as f32 + 0.5 - cx;
            let dy = y as f32 + 0.5 - cy;
            let d2 = dx * dx + dy * dy;
            let f = 1.0 - d2 / (r * r);
            if f <= 0.0 {
                continue;
            }
            let mut w = f * f * k;
            if let Some((dir, half)) = cone {
                let d = d2.sqrt().max(1e-3);
                let c = (dx * dir.x + dy * dir.y) / d;
                let edge = ((c - half.cos()) / (1.0 - half.cos())).clamp(0.0, 1.0);
                w *= edge.sqrt();
                if d < 6.0 {
                    w *= d / 6.0;
                }
            }
            let l = &mut buf[y as usize * LW + x as usize];
            l[0] += col[0] * w;
            l[1] += col[1] * w;
            l[2] += col[2] * w;
        }
    }
}

fn night_light(cv: &mut Canvas, race: &Race, view: &mut View, ox: f32, oy: f32) {
    let amb = [0.14, 0.16, 0.31];
    for l in view.light.iter_mut() {
        *l = amb;
    }
    let to_screen = |p: V2| v2(p.x - ox, p.y - oy);
    for lp in &race.track.lamps {
        let p = to_screen(*lp);
        if p.x < -400.0 || p.y < -400.0 || p.x > W as f32 + 400.0 || p.y > H as f32 + 400.0 {
            continue;
        }
        add_light(
            &mut view.light,
            p.x,
            p.y,
            250.0,
            [1.0, 0.84, 0.55],
            0.75,
            None,
        );
    }
    for r in &race.racers {
        let p = to_screen(r.car.pos);
        if p.x < -500.0 || p.y < -500.0 || p.x > W as f32 + 500.0 || p.y > H as f32 + 500.0 {
            continue;
        }
        let f = r.car.fwd();
        let nose = p + f * 26.0;
        add_light(
            &mut view.light,
            nose.x,
            nose.y,
            420.0,
            [1.0, 0.97, 0.85],
            1.5,
            Some((f, 0.42)),
        );
        add_light(&mut view.light, p.x, p.y, 70.0, [0.6, 0.6, 0.7], 0.8, None);
        let tail = p - f * 30.0;
        add_light(
            &mut view.light,
            tail.x,
            tail.y,
            60.0,
            [1.0, 0.15, 0.1],
            if r.controls.throttle < 0.0 { 1.6 } else { 0.6 },
            None,
        );
        if r.car.boosting() {
            let c = if r.car.nitro_on {
                [0.4, 0.8, 1.0]
            } else {
                [1.0, 0.6, 0.2]
            };
            add_light(&mut view.light, tail.x, tail.y, 120.0, c, 1.4, None);
        }
    }
    for pad in &race.track.pads {
        let p = to_screen(pad.pos);
        add_light(&mut view.light, p.x, p.y, 110.0, [1.0, 0.8, 0.2], 0.9, None);
    }
    for pk in &race.pickups {
        if pk.respawn > 0.0 {
            continue;
        }
        let p = to_screen(pk.pos);
        let c = if pk.kind == PickupKind::Nitro {
            [0.3, 0.8, 1.0]
        } else {
            [0.4, 1.0, 0.5]
        };
        add_light(&mut view.light, p.x, p.y, 70.0, c, 0.9, None);
    }
    for pt in &race.particles {
        if matches!(pt.kind, PKind::Spark | PKind::Flame | PKind::Star) {
            let p = to_screen(pt.pos);
            add_light(
                &mut view.light,
                p.x,
                p.y,
                40.0,
                [1.0, 0.8, 0.5],
                0.6 * pt.life / pt.max,
                None,
            );
        }
    }
    // multiply with bilinear upsampling of the light buffer
    for y in 0..H {
        let fy = (y as f32 - 0.5) / 2.0;
        let y0 = fy.floor().clamp(0.0, (LH - 1) as f32) as usize;
        let y1 = (y0 + 1).min(LH - 1);
        let ty = (fy - y0 as f32).clamp(0.0, 1.0);
        for x in 0..W {
            let fx = (x as f32 - 0.5) / 2.0;
            let x0 = fx.floor().clamp(0.0, (LW - 1) as f32) as usize;
            let x1 = (x0 + 1).min(LW - 1);
            let tx = (fx - x0 as f32).clamp(0.0, 1.0);
            let l00 = view.light[y0 * LW + x0];
            let l10 = view.light[y0 * LW + x1];
            let l01 = view.light[y1 * LW + x0];
            let l11 = view.light[y1 * LW + x1];
            let p = &mut cv.px[y * W + x];
            let mut out = 0u32;
            for (c, sh) in [(0usize, 16u32), (1, 8), (2, 0)] {
                let a = l00[c] + (l10[c] - l00[c]) * tx;
                let b = l01[c] + (l11[c] - l01[c]) * tx;
                let l = (a + (b - a) * ty).min(1.2);
                let v = (((*p >> sh) & 255) as f32 * l).min(255.0) as u32;
                out |= v << sh;
            }
            *p = out;
        }
    }
}

// ---- HUD --------------------------------------------------------------------

fn panel(cv: &mut Canvas, x: f32, y: f32, w: f32, h: f32) {
    cv.round_rect(x + 2.0, y + 3.0, w, h, 12.0, 0x000000, 0.22);
    cv.round_rect(x, y, w, h, 12.0, C_PANEL, 0.82);
    cv.round_rect_line(x, y, w, h, 12.0, 1.5, 0xFFFFFF, 0.08);
}

fn draw_hud(cv: &mut Canvas, race: &Race, view: &View, o: &RenderOpts) {
    let me = race.player();
    let car = &me.car;
    let t = view.time;
    let tt = race.mode == Mode::TimeTrial;

    // --- position / lap / time (top-left)
    panel(cv, 16.0, 14.0, 250.0, 104.0);
    if tt {
        cv.text(
            "TIME TRIAL",
            32.0,
            26.0,
            16.0,
            Face::Bold,
            C_ACCENT,
            1.0,
            Align::Left,
        );
    } else {
        let place = me.place;
        let col = if place == 1 { C_GOLD } else { C_TEXT };
        cv.text(
            ordinal(place),
            30.0,
            18.0,
            52.0,
            Face::Bold,
            col,
            1.0,
            Align::Left,
        );
        let pw = cv.text_width(ordinal(place), 52.0, Face::Bold);
        cv.text(
            &format!("/{}", race.racers.len()),
            30.0 + pw + 6.0,
            40.0,
            22.0,
            Face::Bold,
            C_DIM,
            1.0,
            Align::Left,
        );
    }
    let lap = race.lap_of(me);
    let lap_txt = if me.finish.is_some() {
        "FINISHED".to_string()
    } else {
        format!("LAP {lap}/{}", race.laps)
    };
    cv.text(
        &lap_txt,
        254.0,
        24.0,
        20.0,
        Face::Bold,
        if lap == race.laps && me.finish.is_none() {
            C_ACCENT
        } else {
            C_TEXT
        },
        1.0,
        Align::Right,
    );
    let lt = if race.phase == Phase::Countdown {
        0.0
    } else {
        race.t - me.lap_start
    };
    cv.text(
        &fmt_time(if me.finish.is_some() {
            me.finish.unwrap_or(0.0)
        } else {
            race.t
        }),
        254.0,
        50.0,
        18.0,
        Face::Body,
        C_TEXT,
        1.0,
        Align::Right,
    );
    cv.text("LAP", 32.0, 82.0, 14.0, Face::Body, C_DIM, 1.0, Align::Left);
    cv.text(
        &fmt_time(lt.max(0.0)),
        64.0,
        80.0,
        17.0,
        Face::Body,
        C_TEXT,
        1.0,
        Align::Left,
    );
    if let Some(b) = race.best_lap {
        cv.text(
            "BEST",
            150.0,
            82.0,
            14.0,
            Face::Body,
            C_DIM,
            1.0,
            Align::Left,
        );
        cv.text(
            &fmt_time(b),
            254.0,
            80.0,
            17.0,
            Face::Body,
            C_GREEN,
            1.0,
            Align::Right,
        );
    }

    // --- minimap (top-right)
    let (mx, my) = (W as f32 - 262.0, 14.0);
    panel(cv, mx, my, 246.0, 186.0);
    let mo = v2(mx + 8.0, my + 8.0);
    for k in 0..view.mini.len() {
        let a = view.mini[k] + mo;
        let b = view.mini[(k + 1) % view.mini.len()] + mo;
        cv.line(a.x, a.y, b.x, b.y, 7.0, 0x0D1320, 0.8);
    }
    for k in 0..view.mini.len() {
        let a = view.mini[k] + mo;
        let b = view.mini[(k + 1) % view.mini.len()] + mo;
        cv.line(a.x, a.y, b.x, b.y, 3.5, 0xC9D6E3, 0.95);
    }
    let start = race.track.pts[0] * view.mini_scale + view.mini_off + mo;
    cv.circle(start.x, start.y, 4.0, C_TEXT, 1.0);
    if let Some((gp, _)) = race.ghost_pose() {
        let p = gp * view.mini_scale + view.mini_off + mo;
        cv.circle(p.x, p.y, 4.0, 0x9FD8FF, 0.7);
    }
    for r in race.racers.iter().filter(|r| !r.is_player()) {
        let p = r.car.pos * view.mini_scale + view.mini_off + mo;
        cv.circle(p.x, p.y, 5.0, 0x0D1320, 0.8);
        cv.circle(p.x, p.y, 4.0, r.driver.color, 1.0);
    }
    let p = car.pos * view.mini_scale + view.mini_off + mo;
    cv.circle(p.x, p.y, 7.0 + (t * 6.0).sin(), 0xFFFFFF, 1.0);
    cv.circle(p.x, p.y, 5.0, me.driver.color, 1.0);

    // --- standings ticker under the minimap (championship)
    if !tt {
        let order = race.classification();
        let y0 = 206.0;
        let rows = order.len();
        panel(cv, mx, y0, 246.0, 10.0 + rows as f32 * 21.0);
        for (k, &i) in order.iter().enumerate() {
            let r = &race.racers[i];
            let y = y0 + 6.0 + k as f32 * 21.0;
            if r.is_player() {
                cv.round_rect(mx + 4.0, y - 1.0, 238.0, 20.0, 6.0, C_ACCENT, 0.35);
            }
            cv.text(
                &format!("{}", k + 1),
                mx + 22.0,
                y,
                15.0,
                Face::Bold,
                C_DIM,
                1.0,
                Align::Right,
            );
            cv.circle(mx + 36.0, y + 9.0, 5.0, r.driver.color, 1.0);
            cv.text(
                r.driver.name,
                mx + 48.0,
                y + 1.0,
                15.0,
                Face::Body,
                C_TEXT,
                1.0,
                Align::Left,
            );
            let gap = if k == 0 {
                String::new()
            } else {
                let lead = &race.racers[order[0]];
                let d = lead.progress - r.progress;
                if r.finish.is_some() {
                    String::new()
                } else {
                    format!("+{:.0}m", d / 10.0)
                }
            };
            cv.text(
                &gap,
                mx + 236.0,
                y + 1.0,
                14.0,
                Face::Body,
                C_DIM,
                1.0,
                Align::Right,
            );
        }
    }

    // --- speedometer, nitro, damage (bottom-left)
    let (gx, gy) = (108.0, H as f32 - 100.0);
    cv.circle(gx + 2.0, gy + 3.0, 86.0, 0x000000, 0.22);
    cv.circle(gx, gy, 86.0, C_PANEL, 0.85);
    let sp = car.speed();
    let frac = (sp / 900.0).clamp(0.0, 1.0);
    let a0 = PI * 0.75;
    cv.arc(gx, gy, 70.0, 9.0, a0, PI * 1.5, 0x3A4458, 1.0);
    let col = if car.nitro_on {
        C_CYAN
    } else if car.boost_t > 0.0 {
        0xFFB347
    } else {
        C_ACCENT
    };
    cv.arc(gx, gy, 70.0, 9.0, a0, PI * 1.5 * frac, col, 1.0);
    let kmh = (sp * 0.36).round() as i32;
    cv.text(
        &format!("{kmh}"),
        gx,
        gy - 28.0,
        40.0,
        Face::Bold,
        C_TEXT,
        1.0,
        Align::Center,
    );
    cv.text(
        "km/h",
        gx,
        gy + 14.0,
        14.0,
        Face::Body,
        C_DIM,
        1.0,
        Align::Center,
    );
    cv.text(
        &format!("GEAR {}", car.gear),
        gx,
        gy + 34.0,
        14.0,
        Face::Bold,
        if car.shift_t > 0.0 { C_GOLD } else { C_DIM },
        1.0,
        Align::Center,
    );

    let bx = 210.0;
    let by = H as f32 - 92.0;
    panel(cv, bx, by - 8.0, 250.0, 86.0);
    // nitro
    cv.text(
        "NITRO",
        bx + 14.0,
        by,
        15.0,
        Face::Bold,
        C_CYAN,
        1.0,
        Align::Left,
    );
    cv.text(
        "SHIFT",
        bx + 236.0,
        by + 1.0,
        13.0,
        Face::Body,
        C_DIM,
        1.0,
        Align::Right,
    );
    cv.round_rect(bx + 14.0, by + 20.0, 222.0, 14.0, 7.0, 0x0D1320, 1.0);
    let nw = 222.0 * car.nitro;
    if nw > 2.0 {
        let glow = if car.nitro_on {
            0.3 + 0.2 * (t * 20.0).sin()
        } else {
            0.0
        };
        cv.round_rect(
            bx + 14.0,
            by + 20.0,
            nw,
            14.0,
            7.0,
            mix(C_CYAN, 0xFFFFFF, glow),
            1.0,
        );
    }
    if car.draft > 0.1 {
        let a = (car.draft * 3.0).min(1.0);
        cv.text(
            "SLIPSTREAM",
            bx + 125.0,
            by + 1.0,
            14.0,
            Face::Bold,
            0xE8F6FF,
            a * (0.7 + 0.3 * (t * 12.0).sin()),
            Align::Center,
        );
    }
    // damage
    cv.text(
        "CAR",
        bx + 14.0,
        by + 42.0,
        15.0,
        Face::Bold,
        C_TEXT,
        1.0,
        Align::Left,
    );
    for k in 0..10 {
        let filled = car.hp >= (k as f32 + 0.5) * 10.0;
        let c = if car.hp < 30.0 {
            C_RED
        } else if car.hp < 60.0 {
            C_GOLD
        } else {
            C_GREEN
        };
        cv.round_rect(
            bx + 60.0 + k as f32 * 17.6,
            by + 44.0,
            14.0,
            14.0,
            3.0,
            if filled { c } else { 0x0D1320 },
            1.0,
        );
    }
    // drift tier pips
    if car.drifting {
        let tier = car.drift_tier();
        let cols = [0x5BC8FF, 0xFF9A3C, 0xC86BFF];
        for k in 0..3 {
            let c = if (k as u8) < tier { cols[k] } else { 0x3A4458 };
            cv.circle(bx + 290.0 + k as f32 * 22.0, by + 30.0, 8.0, c, 1.0);
        }
        cv.text(
            "DRIFT",
            bx + 312.0,
            by + 46.0,
            13.0,
            Face::Bold,
            C_DIM,
            1.0,
            Align::Center,
        );
    }

    // --- style chain (bottom-centre)
    let st = &race.style;
    if st.chain > 0 {
        let cx = W as f32 / 2.0 + 120.0;
        let y = H as f32 - 88.0;
        let fade = (st.timer / 2.6).clamp(0.0, 1.0);
        cv.text_outline(
            &format!("x{}", st.mult()),
            cx,
            y - 6.0,
            40.0,
            Face::Bold,
            C_GOLD,
            0x1A1F2B,
            1.0,
            Align::Center,
        );
        cv.text_outline(
            &format!("{} STYLE", st.chain_pts),
            cx,
            y + 38.0,
            18.0,
            Face::Bold,
            C_TEXT,
            0x1A1F2B,
            1.0,
            Align::Center,
        );
        cv.round_rect(cx - 60.0, y + 64.0, 120.0, 5.0, 2.5, 0x0D1320, 0.8);
        cv.round_rect(cx - 60.0, y + 64.0, 120.0 * fade, 5.0, 2.5, C_GOLD, 1.0);
    }
    if !tt {
        cv.text_sh(
            &format!("STYLE {}", st.total + st.chain_pts),
            W as f32 - 20.0,
            H as f32 - 34.0,
            18.0,
            Face::Bold,
            C_GOLD,
            1.0,
            Align::Right,
        );
    }

    // --- pace notes (top-centre)
    if race.phase != Phase::Countdown && me.finish.is_none() {
        if let Some((d, c)) = race.next_corner() {
            let a = (1.0 - (d - 600.0) / 300.0).clamp(0.0, 1.0);
            if a > 0.0 {
                let cx = W as f32 / 2.0;
                let col = [C_GREEN, C_GREEN, C_GOLD, C_RED][c.severity as usize];
                let w = 230.0;
                cv.round_rect(cx - w / 2.0, 14.0, w, 58.0, 12.0, C_PANEL, 0.8 * a);
                // arrow
                let dir = c.dir;
                let ax = cx - w / 2.0 + 34.0;
                let ay = 43.0;
                let bend: f32 = [0.0, 0.5, 0.9, 1.4][c.severity as usize];
                let tip = v2(
                    ax + dir * 14.0 * bend.min(1.0),
                    ay - 16.0 + if c.severity == 3 { 10.0 } else { 0.0 },
                );
                cv.line(ax, ay + 16.0, ax, ay, 6.0, col, a);
                cv.line(ax, ay, tip.x, tip.y, 6.0, col, a);
                let dirv = (tip - v2(ax, ay)).norm();
                let side = dirv.perp();
                cv.tri(
                    [
                        (tip.x + dirv.x * 9.0, tip.y + dirv.y * 9.0),
                        (tip.x + side.x * 8.0, tip.y + side.y * 8.0),
                        (tip.x - side.x * 8.0, tip.y - side.y * 8.0),
                    ],
                    col,
                    a,
                );
                let name = match c.severity {
                    3 => "HAIRPIN",
                    2 => "TIGHT",
                    _ => "FAST",
                };
                let lr = if dir > 0.0 { "RIGHT" } else { "LEFT" };
                cv.text(
                    &format!("{name} {lr}"),
                    cx - w / 2.0 + 64.0,
                    20.0,
                    20.0,
                    Face::Bold,
                    col,
                    a,
                    Align::Left,
                );
                cv.text(
                    &format!("{:.0} m", d.max(0.0) / 10.0),
                    cx - w / 2.0 + 64.0,
                    46.0,
                    16.0,
                    Face::Body,
                    C_TEXT,
                    a,
                    Align::Left,
                );
            }
        }
    }

    // --- off-screen rivals: edge markers
    let cam = view.cam;
    for r in &race.racers {
        if r.is_player() {
            continue;
        }
        let d = r.car.pos - cam;
        if d.x.abs() < W as f32 / 2.0 - 10.0 && d.y.abs() < H as f32 / 2.0 - 10.0 {
            continue;
        }
        if d.len() > 900.0 {
            continue;
        }
        let k = ((W as f32 / 2.0 - 30.0) / d.x.abs().max(1e-3))
            .min((H as f32 / 2.0 - 30.0) / d.y.abs().max(1e-3));
        let p = v2(W as f32 / 2.0, H as f32 / 2.0) + d * k;
        let dir = d.norm();
        let side = dir.perp();
        let a = (1.0 - d.len() / 900.0).clamp(0.3, 1.0);
        let tip = p + dir * 12.0;
        cv.tri(
            [
                (tip.x, tip.y),
                (
                    p.x + side.x * 10.0 - dir.x * 6.0,
                    p.y + side.y * 10.0 - dir.y * 6.0,
                ),
                (
                    p.x - side.x * 10.0 - dir.x * 6.0,
                    p.y - side.y * 10.0 - dir.y * 6.0,
                ),
            ],
            r.driver.color,
            a,
        );
        if r.ai.telegraphing() {
            cv.circle(p.x - dir.x * 18.0, p.y - dir.y * 18.0, 9.0, C_RED, a);
        }
    }

    // --- countdown
    if race.phase == Phase::Countdown {
        let c = race.countdown;
        let n = c.ceil() as i32;
        let lit = (3 - n.clamp(0, 3)) as usize;
        let (cx, cy) = (W as f32 / 2.0, 150.0);
        cv.round_rect(cx - 130.0, cy - 40.0, 260.0, 80.0, 20.0, 0x14181F, 0.9);
        for k in 0..3 {
            let x = cx - 76.0 + k as f32 * 76.0;
            let on = k < lit + 1 && n <= 3;
            cv.circle(x, cy, 26.0, 0x2A2F3A, 1.0);
            if on {
                cv.circle(x, cy, 22.0, C_RED, 1.0);
                cv.glow(x, cy, 50.0, C_RED, 0.5);
            }
        }
        if n <= 3 {
            let f = c - c.floor();
            cv.text_outline(
                &format!("{n}"),
                cx,
                cy + 50.0,
                64.0 + f * 20.0,
                Face::Bold,
                C_TEXT,
                0x1A1F2B,
                1.0,
                Align::Center,
            );
        }
        if o.hints {
            hint_box(
                cv,
                "Hold UP right as the lights go out for a ROCKET START. Too early and you stall.",
                0,
            );
        }
    } else if race.t < 1.0 {
        let (cx, cy) = (W as f32 / 2.0, 150.0);
        cv.round_rect(
            cx - 130.0,
            cy - 40.0,
            260.0,
            80.0,
            20.0,
            0x14181F,
            0.9 * (1.0 - race.t),
        );
        for k in 0..3 {
            let x = cx - 76.0 + k as f32 * 76.0;
            cv.circle(x, cy, 22.0, C_GREEN, 1.0 - race.t);
            cv.glow(x, cy, 50.0, C_GREEN, 0.5 * (1.0 - race.t));
        }
        cv.text_outline(
            "GO!",
            cx,
            cy + 46.0,
            76.0,
            Face::Bold,
            C_GREEN,
            0x1A1F2B,
            1.0 - race.t,
            Align::Center,
        );
    }

    // --- banners
    if race.final_lap_announced && me.finish.is_none() && me.lap == race.laps {
        let since = race.t - me.lap_start;
        if since < 2.5 {
            let a = (2.5 - since).min(1.0);
            cv.text_outline(
                "FINAL LAP",
                W as f32 / 2.0,
                220.0,
                54.0,
                Face::Bold,
                C_ACCENT,
                0x1A1F2B,
                a,
                Align::Center,
            );
        }
    } else if me.finish.is_none() && me.lap >= 2 && race.t - me.lap_start < 2.0 && race.t > 3.0 {
        let a = (2.0 - (race.t - me.lap_start)).min(1.0);
        cv.text_outline(
            &format!("LAP {}", me.lap),
            W as f32 / 2.0,
            220.0,
            44.0,
            Face::Bold,
            C_TEXT,
            0x1A1F2B,
            a,
            Align::Center,
        );
        if let Some(l) = me.lap_times.last() {
            let best = race.best_lap.is_some_and(|b| (*l - b).abs() < 1e-3);
            cv.text_outline(
                &fmt_time(*l),
                W as f32 / 2.0,
                272.0,
                26.0,
                Face::Bold,
                if best { C_GREEN } else { C_TEXT },
                0x1A1F2B,
                a,
                Align::Center,
            );
        }
    }
    if me.finish.is_some() {
        let txt = if tt {
            "FINISH".to_string()
        } else {
            format!("FINISHED {}", ordinal(me.place).to_uppercase())
        };
        let pulse = 1.0 + 0.04 * (t * 5.0).sin();
        cv.text_outline(
            &txt,
            W as f32 / 2.0,
            200.0,
            60.0 * pulse,
            Face::Bold,
            if me.place == 1 || tt { C_GOLD } else { C_TEXT },
            0x1A1F2B,
            1.0,
            Align::Center,
        );
    }
    if car.hp < 25.0 && me.finish.is_none() {
        let a = 0.5 + 0.5 * (t * 6.0).sin();
        cv.text_sh(
            "HEAVY DAMAGE - grab a wrench",
            W as f32 / 2.0,
            H as f32 - 150.0,
            18.0,
            Face::Bold,
            C_RED,
            a,
            Align::Center,
        );
    }

    // --- tutorial hints
    if o.hints && race.phase == Phase::Running && me.finish.is_none() {
        let tsec = race.t;
        let h = race.hint_flags;
        let msg = if tsec < 7.0 {
            Some(
                "UP accelerate   DOWN brake   LEFT/RIGHT steer.  Follow the pace notes at the top.",
            )
        } else if tsec < 16.0 && race.stats_turbos == 0 {
            Some(
                "In a corner, hold SPACE while steering to DRIFT. Hold it until the sparks turn orange, then let go for a turbo.",
            )
        } else if h & 1 == 0 && tsec < 40.0 {
            Some(
                "Tuck in right behind a rival: the SLIPSTREAM tows you and fills your NITRO meter.",
            )
        } else if h & 1 != 0 && h & 2 == 0 && car.nitro > 0.4 && tsec < 60.0 {
            Some(
                "NITRO charged! Hold SHIFT to fire it. Pull out of a slipstream and boost past for a SLINGSHOT.",
            )
        } else if tsec > 25.0 && tsec < 34.0 {
            Some("Blue cans refill NITRO, green wrenches repair. Yellow arrows are boost pads.")
        } else {
            None
        };
        if let Some(m) = msg {
            hint_box(cv, m, 1);
        }
    }
    if o.paused {
        // handled by the app overlay
    }
}

pub fn hint_box(cv: &mut Canvas, msg: &str, slot: i32) {
    let w = cv.text_width(msg, 18.0, Face::Body) + 40.0;
    let y = if slot == 0 { 250.0 } else { H as f32 - 250.0 };
    let x = W as f32 / 2.0 - w / 2.0;
    cv.round_rect(x, y, w, 40.0, 12.0, 0x10151F, 0.88);
    cv.round_rect_line(x, y, w, 40.0, 12.0, 2.0, C_ACCENT, 0.9);
    cv.text(
        msg,
        W as f32 / 2.0,
        y + 10.0,
        18.0,
        Face::Body,
        C_TEXT,
        1.0,
        Align::Center,
    );
}

/// Small helper for UI: draw a car sprite at full resolution, upright.
pub fn car_portrait(cv: &mut Canvas, sprite: &str, x: f32, y: f32, scale: f32, rot: f32) {
    let a = assets::get();
    let s = a.big(sprite);
    cv.blit(
        s,
        x + 6.0,
        y + 8.0,
        Blit {
            rot,
            scale,
            alpha: 0.3,
            solid: Some(0x0D1520),
            ..Blit::default()
        },
    );
    cv.blit(
        s,
        x,
        y,
        Blit {
            rot,
            scale,
            ..Blit::default()
        },
    );
}
