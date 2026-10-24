//! Procedural sprites: Wren, foes, anchors, pickups and the Stormheart.
//! Everything is drawn from vector shapes each frame so it can animate.

use super::canvas::{Canvas, lerp_color, scale};
use super::palette::Pal;
use crate::boss::{BOSS_R, BState, Boss};
use crate::level::*;
use crate::sim::*;

fn sinf(x: f32) -> f32 {
    x.sin()
}

/// Wren's pose inputs.
pub struct WrenPose {
    pub x: f32,
    pub y: f32,
    pub facing: f32,
    pub vx: f32,
    pub vy: f32,
    pub on_ground: bool,
    pub wall: i8,
    pub run_t: f32,
    pub anchor: Option<(f32, f32)>,
    pub invuln: f32,
    pub hurt_t: f32,
    pub charged: bool,
    pub dying: f32,
    pub time: f32,
    pub shield: bool,
    pub landed_squash: f32,
}

pub fn draw_wren(c: &mut Canvas, p: &WrenPose, scarf: &[(f32, f32); 9]) {
    // blink while invulnerable
    if p.invuln > 0.0 && p.hurt_t <= 0.0 && ((p.time * 18.0) as i32) % 2 == 0 && p.dying <= 0.0 {
        return;
    }
    let f = p.facing;
    let airborne = !p.on_ground;
    // squash & stretch around the feet
    let stretch = if airborne {
        1.0 + (p.vy.abs() / 1400.0).min(0.16)
    } else {
        1.0 - p.landed_squash * 0.18
    };
    let widen = 1.0 / stretch.sqrt();
    let feet_y = p.y + 15.0;
    let tx = |ox: f32| p.x + ox * widen;
    let ty = |oy: f32| feet_y + (oy - 15.0) * stretch;
    let flash = p.hurt_t > 0.0 && ((p.time * 30.0) as i32) % 2 == 0;
    let tint = |col: u32| if flash { 0xffffff } else { col };

    // shadow
    if p.on_ground {
        c.ellipse(p.x, feet_y + 1.0, 11.0, 3.0, 0x000000, 0.28);
    }
    // charged aura
    if p.charged {
        c.glow(p.x, p.y, 44.0, 0xaee8ff, 0.5);
        c.glow(p.x, p.y, 26.0, 0xffffff, 0.35);
    }
    if p.shield {
        c.ring(
            p.x,
            p.y,
            20.0,
            22.0,
            2.0,
            0x9ce8ff,
            0.45 + 0.2 * sinf(p.time * 6.0),
        );
    }

    // scarf (behind body): a tapering ribbon with a forked, fluttering tail
    {
        let n = scarf.len();
        let mut left: Vec<(f32, f32)> = Vec::with_capacity(n);
        let mut right: Vec<(f32, f32)> = Vec::with_capacity(n);
        for i in 0..n {
            let (px, py) = scarf[i];
            let (nx, ny) = if i + 1 < n {
                (scarf[i + 1].0 - px, scarf[i + 1].1 - py)
            } else {
                (px - scarf[i - 1].0, py - scarf[i - 1].1)
            };
            let l = (nx * nx + ny * ny).sqrt().max(0.001);
            let t = i as f32 / (n - 1) as f32;
            let w = (5.0 - 3.2 * t) * (1.0 + 0.25 * (p.time * 9.0 + i as f32 * 1.2).sin() * t);
            left.push((px - ny / l * w, py + nx / l * w));
            right.push((px + ny / l * w, py - nx / l * w));
        }
        for i in 0..n - 1 {
            let t = i as f32 / (n - 1) as f32;
            let col = lerp_color(0xff7a3c, 0xffc24a, t);
            c.poly(
                &[left[i], left[i + 1], right[i + 1], right[i]],
                tint(col),
                1.0,
            );
        }
        // light edge along the top
        for i in 0..n - 1 {
            c.line(
                left[i].0,
                left[i].1,
                left[i + 1].0,
                left[i + 1].1,
                1.0,
                0xffd8a0,
                0.55,
            );
        }
        // forked tail
        let (ex, ey) = scarf[n - 1];
        let (dx, dy) = (
            scarf[n - 1].0 - scarf[n - 2].0,
            scarf[n - 1].1 - scarf[n - 2].1,
        );
        let l = (dx * dx + dy * dy).sqrt().max(0.001);
        let (ux, uy) = (dx / l, dy / l);
        let flick = (p.time * 11.0).sin() * 2.0;
        for s in [-1.0f32, 1.0] {
            c.poly(
                &[
                    (ex - uy * s * 1.4, ey + ux * s * 1.4),
                    (
                        ex + ux * 7.0 - uy * s * (3.5 + flick),
                        ey + uy * 7.0 + ux * s * (3.5 + flick),
                    ),
                    (ex + ux * 1.0 - uy * s * 0.2, ey + uy * 1.0 + ux * s * 0.2),
                ],
                tint(0xffc24a),
                0.95,
            );
        }
    }

    // legs
    let (la, lb) = if p.on_ground {
        if p.vx.abs() > 20.0 {
            let a = sinf(p.run_t * 1.0) * 0.95;
            (a, -a)
        } else {
            (0.05, -0.05)
        }
    } else if p.wall != 0 {
        (0.4, 0.55)
    } else if p.vy < -60.0 {
        (0.7, -0.15)
    } else {
        (0.25, -0.5)
    };
    for (k, ang) in [lb, la].iter().enumerate() {
        let hip = (tx(f * (-0.5 + k as f32 * 1.5)), ty(6.0));
        let len = 9.0 * stretch;
        let knee_x = hip.0 + (ang * f).sin() * len * 0.9;
        let knee_y = hip.1 + (ang.cos()) * len * 0.8;
        let foot_x = knee_x + (ang * f * 0.5).sin() * 2.0 + f * 2.0;
        let foot_y = (hip.1 + len).min(knee_y + 4.0).max(hip.1 + 6.0);
        let col = if k == 0 { 0x231f38 } else { 0x2e2a46 };
        c.line(hip.0, hip.1, knee_x, knee_y, 4.2, tint(col), 1.0);
        c.line(
            knee_x,
            knee_y,
            foot_x,
            foot_y.min(feet_y),
            4.0,
            tint(col),
            1.0,
        );
        c.ellipse(
            foot_x + f * 1.5,
            foot_y.min(feet_y) - 0.5,
            4.2,
            2.6,
            tint(0x6b4a32),
            1.0,
        );
    }
    // satchel behind
    c.round_rect(
        tx(-f * 9.0) - 4.0,
        ty(-3.0),
        8.0,
        10.0,
        2.5,
        tint(0x7a4e2c),
        1.0,
    );
    // torso
    c.round_rect(
        tx(0.0) - 8.0,
        ty(-9.0),
        16.0,
        17.0 * stretch,
        5.5,
        tint(0x24a8a4),
        1.0,
    );
    c.round_rect(
        tx(0.0) - 8.0,
        ty(-9.0),
        16.0,
        6.0,
        4.0,
        tint(0x4ed6cc),
        0.55,
    );
    c.fill_rect(tx(0.0) - 8.0, ty(3.5), 16.0, 2.4, tint(0x5a3a22), 1.0);
    c.fill_rect(tx(f * 1.5) - 1.5, ty(3.3), 3.0, 3.0, 0xffd24a, 1.0);
    // head
    let hx = tx(f * 1.0);
    let hy = ty(-14.0);
    c.circle(hx, hy, 7.6, tint(0xf2c9a0), 1.0);
    // hair
    c.poly(
        &[
            (hx - f * 8.5, hy + 3.0),
            (hx - f * 7.0, hy - 6.0),
            (hx + f * 1.0, hy - 9.0),
            (hx + f * 7.5, hy - 3.0),
            (hx + f * 2.0, hy - 3.0),
            (hx - f * 2.0, hy + 2.0),
        ],
        tint(0x4a2c1a),
        1.0,
    );
    // goggles on the forehead
    c.line(
        hx - f * 6.0,
        hy - 3.5,
        hx + f * 7.0,
        hy - 3.5,
        2.2,
        tint(0x2a2a3a),
        1.0,
    );
    c.circle(hx + f * 3.4, hy - 3.6, 3.3, tint(0x3a3a50), 1.0);
    c.circle(hx + f * 3.4, hy - 3.6, 2.4, tint(0x9ce8ff), 1.0);
    c.circle(hx + f * 2.7, hy - 4.3, 0.9, 0xffffff, 0.9);
    // eye
    c.circle(hx + f * 3.8, hy + 0.8, 1.1, 0x2a1a18, 1.0);
    // arms
    let sh = (tx(f * 1.5), ty(-5.0));
    if let Some((ax, ay)) = p.anchor {
        let dx = ax - sh.0;
        let dy = ay - sh.1;
        let d = (dx * dx + dy * dy).sqrt().max(1.0);
        let hand = (sh.0 + dx / d * 11.0, sh.1 + dy / d * 11.0);
        c.line(sh.0, sh.1, hand.0, hand.1, 4.0, tint(0x24a8a4), 1.0);
        c.circle(hand.0, hand.1, 2.8, tint(0xf2c9a0), 1.0);
    } else {
        let sw = if p.on_ground && p.vx.abs() > 20.0 {
            sinf(p.run_t + 3.14) * 0.9
        } else if airborne {
            -1.1 + p.vy.clamp(-600.0, 600.0) / 700.0
        } else {
            0.0
        };
        let hand = (sh.0 + (sw * f).sin() * 9.0 + f * 1.0, sh.1 + sw.cos() * 7.5);
        c.line(sh.0, sh.1, hand.0, hand.1, 4.0, tint(0x1c8e8c), 1.0);
        c.circle(hand.0, hand.1, 2.6, tint(0xf2c9a0), 1.0);
    }
}

pub fn draw_anchor(
    c: &mut Canvas,
    x: f32,
    y: f32,
    kind: AnchorKind,
    alive: bool,
    stress: f32,
    near: bool,
    targeted: bool,
    time: f32,
    pal: &Pal,
) {
    if !alive {
        // faint ghost where it will return
        c.ring(x, y, 12.0, 12.0, 1.5, 0x888888, 0.15);
        return;
    }
    let pulse = 0.5 + 0.5 * sinf(time * 4.0 + x * 0.01);
    let base = match kind {
        AnchorKind::Ring => 0xf0b85a,
        AnchorKind::Brittle => 0xc8e8ff,
        AnchorKind::Rail => 0xff9a5a,
    };
    let glow_a = if targeted {
        0.9
    } else if near {
        0.5 + 0.2 * pulse
    } else {
        0.28
    };
    c.glow(x, y, if targeted { 54.0 } else { 38.0 }, base, glow_a);
    // mounting plate
    c.circle(x, y, 13.0, 0x2a2438, 0.9);
    c.ring(x, y, 12.0, 12.0, 3.4, base, 1.0);
    c.ring(x, y, 12.0, 12.0, 1.0, 0xffffff, 0.5);
    c.circle(x, y, 4.0, scale(base, 0.7), 1.0);
    match kind {
        AnchorKind::Brittle => {
            // cracks that widen with stress
            let k = (stress / BRITTLE_HOLD).clamp(0.0, 1.0);
            c.line(x - 8.0, y - 6.0, x - 1.0, y + 1.0, 1.4, 0x3a4a66, 0.9);
            c.line(x - 1.0, y + 1.0, x + 6.0, y + 9.0, 1.4, 0x3a4a66, 0.9);
            if k > 0.0 {
                c.line(
                    x + 8.0,
                    y - 7.0,
                    x + 1.0,
                    y + 1.0,
                    1.4 + k * 2.0,
                    0xff6040,
                    0.4 + k * 0.6,
                );
                c.glow(x, y, 30.0 + 20.0 * k, 0xff6040, 0.5 * k);
            }
        }
        AnchorKind::Rail => {
            c.circle(x, y, 7.0, 0xffe0b0, 0.5);
        }
        AnchorKind::Ring => {}
    }
    if targeted {
        // reticle
        let r = 22.0 + 2.5 * sinf(time * 8.0);
        for k in 0..4 {
            let a = k as f32 * 1.5708 + time * 1.5;
            c.line(
                x + a.cos() * (r - 6.0),
                y + a.sin() * (r - 6.0),
                x + a.cos() * r,
                y + a.sin() * r,
                2.0,
                0xffffff,
                0.9,
            );
        }
    }
    let _ = pal;
}

pub fn draw_crystal(c: &mut Canvas, x: f32, y: f32, time: f32, seed: f32) {
    let spin = (time * 2.6 + seed).cos();
    let w = 7.0 * (0.35 + 0.65 * spin.abs());
    let bob = sinf(time * 3.0 + seed) * 2.0;
    let y = y + bob;
    c.glow(x, y, 26.0, 0x5fe8ff, 0.5);
    let top = (x, y - 11.0);
    let bot = (x, y + 11.0);
    c.poly(
        &[top, (x + w, y - 2.0), bot, (x - w, y - 2.0)],
        0x38c8f0,
        1.0,
    );
    c.poly(
        &[top, (x + w * 0.2, y - 2.0), bot, (x - w, y - 2.0)],
        0x80f0ff,
        1.0,
    );
    c.poly(
        &[top, (x + w, y - 2.0), (x + w * 0.2, y - 2.0)],
        0xd8ffff,
        0.9,
    );
    let tw = (time * 5.0 + seed * 3.0).sin();
    if tw > 0.8 {
        c.line(x - 5.0, y - 6.0, x + 5.0, y - 6.0, 1.2, 0xffffff, 0.9);
        c.line(x, y - 11.0, x, y - 1.0, 1.2, 0xffffff, 0.9);
    }
}

pub fn draw_pickup(c: &mut Canvas, kind: EntKind, x: f32, y: f32, time: f32) {
    match kind {
        EntKind::EchoOrb => {
            let bob = sinf(time * 2.5) * 3.0;
            c.glow(x, y + bob, 40.0, 0x8aa8ff, 0.7);
            c.circle(x, y + bob, 9.0, 0x6a7cff, 1.0);
            c.circle(x - 2.0, y + bob - 2.5, 4.2, 0xdde4ff, 0.9);
            for k in 0..2 {
                let t = ((time * 0.9 + k as f32 * 0.5) % 1.0).clamp(0.0, 1.0);
                c.ring(
                    x,
                    y + bob,
                    10.0 + 18.0 * t,
                    10.0 + 18.0 * t,
                    1.5,
                    0xb8c8ff,
                    0.7 * (1.0 - t),
                );
            }
        }
        EntKind::HeartShard => {
            let bob = sinf(time * 2.8) * 3.0;
            c.glow(x, y + bob, 38.0, 0xff6f7f, 0.7);
            draw_heart(c, x, y + bob, 1.1, 0xff5a70, 1.0);
        }
        EntKind::Chest => {
            let bob = sinf(time * 1.8) * 1.5;
            c.glow(x, y - 6.0, 52.0, 0xffe066, 0.6);
            c.round_rect(x - 16.0, y - 8.0 + bob, 32.0, 20.0, 3.0, 0x7a4a24, 1.0);
            c.round_rect(x - 16.0, y - 14.0 + bob, 32.0, 10.0, 5.0, 0x9a6230, 1.0);
            c.fill_rect(x - 16.0, y - 1.0 + bob, 32.0, 3.0, 0xffd24a, 1.0);
            c.fill_rect(x - 2.5, y - 5.0 + bob, 5.0, 8.0, 0xffe9a0, 1.0);
            let tw = (time * 4.0).sin().max(0.0);
            c.circle(x + 10.0, y - 12.0 + bob, 1.8 + 1.5 * tw, 0xffffff, 0.9);
        }
        _ => {}
    }
}

pub fn draw_heart(c: &mut Canvas, x: f32, y: f32, s: f32, col: u32, a: f32) {
    let r = 5.0 * s;
    c.circle(x - r * 0.9, y - r * 0.6, r, col, a);
    c.circle(x + r * 0.9, y - r * 0.6, r, col, a);
    c.poly(
        &[
            (x - r * 1.85, y - r * 0.3),
            (x + r * 1.85, y - r * 0.3),
            (x, y + r * 2.2),
        ],
        col,
        a,
    );
}

pub fn draw_skitter(c: &mut Canvas, e: &Ent, x: f32, y: f32, time: f32) {
    let d = e.dir;
    let t = e.t * 9.0;
    // legs
    for k in 0..3 {
        let ph = t + k as f32 * 2.1;
        let off = (k as f32 - 1.0) * 7.0;
        let lift = sinf(ph).max(0.0) * 3.0;
        for s in [-1.0f32, 1.0] {
            let _ = s;
        }
        c.line(
            x + off,
            y + 2.0,
            x + off - d * 3.0 + sinf(ph) * 3.0,
            y + 8.0 - lift,
            1.8,
            0x2a1c40,
            1.0,
        );
        c.line(
            x + off,
            y + 2.0,
            x + off + d * 4.0 - sinf(ph) * 3.0,
            y + 8.0 - lift * 0.6,
            1.8,
            0x3a2a58,
            1.0,
        );
    }
    c.ellipse(x, y, 13.0, 9.0, 0x4a2c78, 1.0);
    c.ellipse(x, y - 2.5, 11.0, 6.0, 0x7a52b8, 1.0);
    c.ellipse(x - d * 2.0, y - 4.0, 6.0, 2.6, 0xb08ae8, 0.7);
    // head and eyes
    c.circle(x + d * 10.0, y - 1.0, 5.0, 0x3a2260, 1.0);
    c.circle(x + d * 11.5, y - 2.5, 2.1, 0xffe066, 1.0);
    c.circle(x + d * 12.0, y - 2.5, 0.9, 0x2a1030, 1.0);
    c.glow(x + d * 11.5, y - 2.5, 9.0, 0xffe066, 0.5);
    let _ = time;
}

pub fn draw_bulwark(c: &mut Canvas, e: &Ent, x: f32, y: f32, time: f32) {
    let d = e.dir;
    let dazed = e.state == 9;
    let t = e.t * 6.0;
    for k in 0..2 {
        let ph = t + k as f32 * 3.14;
        c.line(
            x - 7.0 + k as f32 * 12.0,
            y + 6.0,
            x - 7.0 + k as f32 * 12.0 + sinf(ph) * 4.0,
            y + 14.0,
            3.4,
            0x3a2c22,
            1.0,
        );
    }
    c.ellipse(x, y, 18.0, 12.0, 0x8a6a44, 1.0);
    c.ellipse(x - d * 3.0, y - 3.0, 14.0, 7.0, 0xb08a58, 1.0);
    // armour plates
    for k in 0..3 {
        let px = x - d * (6.0 - k as f32 * 6.0);
        c.poly(
            &[
                (px - 5.0, y - 11.0),
                (px + 5.0, y - 11.0),
                (px + 4.0, y + 1.0),
                (px - 4.0, y + 1.0),
            ],
            0xd8d0c0,
            1.0,
        );
        c.fill_rect(px - 4.0, y - 11.0, 8.0, 2.0, 0xffffff, 0.5);
    }
    // front shield
    c.poly(
        &[
            (x + d * 16.0, y - 11.0),
            (x + d * 22.0, y - 4.0),
            (x + d * 22.0, y + 8.0),
            (x + d * 16.0, y + 12.0),
        ],
        0x7a8aa0,
        1.0,
    );
    c.line(
        x + d * 20.0,
        y - 3.0,
        x + d * 20.0,
        y + 7.0,
        1.5,
        0xd0e0f0,
        0.8,
    );
    c.circle(x + d * 12.0, y - 5.0, 1.7, 0xff5a3a, 1.0);
    if dazed {
        for k in 0..3 {
            let a = time * 7.0 + k as f32 * 2.1;
            c.circle(
                x + a.cos() * 11.0,
                y - 18.0 + a.sin() * 3.0,
                2.0,
                0xffe066,
                1.0,
            );
        }
    }
}

pub fn draw_spitter(c: &mut Canvas, e: &Ent, x: f32, y: f32, time: f32) {
    let charging = e.state == 1;
    c.ellipse(x, y + 12.0, 14.0, 3.0, 0x000000, 0.25);
    c.round_rect(x - 13.0, y - 12.0, 26.0, 26.0, 7.0, 0x6a6684, 1.0);
    c.round_rect(x - 13.0, y - 12.0, 26.0, 9.0, 6.0, 0x8e8aa8, 0.9);
    c.fill_rect(x - 13.0, y + 6.0, 26.0, 2.0, 0x3a364c, 0.8);
    // rune marks
    c.line(x - 8.0, y + 10.0, x - 4.0, y + 4.0, 1.3, 0x4a465e, 1.0);
    c.line(x + 8.0, y + 10.0, x + 4.0, y + 4.0, 1.3, 0x4a465e, 1.0);
    // eye socket and aiming pupil
    c.circle(x, y - 2.0, 7.5, 0x1c1828, 1.0);
    let (px, py) = if charging || e.state == 0 {
        (e.a.cos() * 3.0, e.a.sin() * 3.0)
    } else {
        (0.0, 0.0)
    };
    let eye = if charging {
        lerp_color(0xff8040, 0xffffff, (e.t * 6.0).fract())
    } else {
        0xff5a3a
    };
    c.circle(x + px, y - 2.0 + py, 4.4, eye, 1.0);
    c.glow(
        x + px,
        y - 2.0 + py,
        if charging { 26.0 } else { 14.0 },
        0xff6a30,
        if charging { 0.9 } else { 0.4 },
    );
    let _ = time;
}

pub fn draw_spitter_aim(c: &mut Canvas, e: &Ent, x: f32, y: f32) {
    if e.state != 1 {
        return;
    }
    // telegraph: a thin line that thickens toward the shot
    let k = (e.t / 0.8).clamp(0.0, 1.0);
    let len = 420.0;
    let (dx, dy) = (e.a.cos(), e.a.sin());
    c.line(
        x,
        y - 6.0,
        x + dx * len,
        y - 6.0 + dy * len,
        1.0 + 3.0 * k,
        0xff5a3a,
        0.15 + 0.4 * k,
    );
    c.line(
        x,
        y - 6.0,
        x + dx * len * 0.5,
        y - 6.0 + dy * len * 0.5,
        1.0,
        0xffffff,
        0.5 * k,
    );
}

pub fn draw_saw(
    c: &mut Canvas,
    e: &Ent,
    x: f32,
    y: f32,
    hx: f32,
    hy: f32,
    ex: f32,
    ey: f32,
    time: f32,
) {
    // rail with ticks
    c.line(hx, hy, ex, ey, 3.0, 0x2a2438, 0.8);
    c.line(hx, hy, ex, ey, 1.0, 0x6a6488, 0.8);
    for p in [(hx, hy), (ex, ey)] {
        c.circle(p.0, p.1, 5.0, 0x4a4468, 1.0);
        c.circle(p.0, p.1, 2.0, 0xa8a4c8, 1.0);
    }
    let spin = time * 14.0 + e.dir;
    c.glow(x, y, 36.0, 0xff4a3a, 0.35);
    let n = 10;
    let mut pts = Vec::with_capacity(n * 2);
    for k in 0..n * 2 {
        let a = spin + k as f32 * std::f32::consts::PI / n as f32;
        let r = if k % 2 == 0 { 19.0 } else { 13.0 };
        pts.push((x + a.cos() * r, y + a.sin() * r));
    }
    c.poly(&pts, 0xc8ccd8, 1.0);
    c.circle(x, y, 10.0, 0x8a90a4, 1.0);
    c.circle(x, y, 4.0, 0x3a3a50, 1.0);
    c.circle(x - 2.0, y - 2.0, 2.0, 0xffffff, 0.6);
}

pub fn draw_geyser(c: &mut Canvas, e: &Ent, x: f32, y: f32, time: f32) {
    // vent
    c.ellipse(x, y + 2.0, 15.0, 5.0, 0x1c0e0c, 1.0);
    let heat = match e.state {
        0 => 0.25,
        1 => 0.6 + 0.4 * sinf(time * 30.0),
        _ => 1.0,
    };
    c.ellipse(x, y + 1.0, 11.0, 3.5, 0xff6a20, heat);
    c.glow(x, y, 34.0, 0xff6a20, 0.4 * heat);
    if e.state == 1 {
        // telegraph: rising bubbles and a faint column outline
        c.line(x - 13.0, y, x - 13.0, y - 170.0, 1.0, 0xff9a3a, 0.18);
        c.line(x + 13.0, y, x + 13.0, y - 170.0, 1.0, 0xff9a3a, 0.18);
        for k in 0..3 {
            let t = ((time * 2.0 + k as f32 * 0.33) % 1.0).clamp(0.0, 1.0);
            c.circle(
                x + sinf(time * 9.0 + k as f32) * 6.0,
                y - t * 40.0,
                2.5,
                0xffb040,
                1.0 - t,
            );
        }
    }
    if e.state == 2 {
        let h = 170.0;
        let w = 14.0 + 2.0 * sinf(time * 40.0);
        c.glow(x, y - h / 2.0, 120.0, 0xff7a30, 0.45);
        c.poly(
            &[
                (x - w, y),
                (x - w * 0.7, y - h * 0.55),
                (x - w * 0.2, y - h),
                (x + w * 0.3, y - h * 0.9),
                (x + w * 0.8, y - h * 0.5),
                (x + w, y),
            ],
            0xff5a1a,
            0.95,
        );
        c.poly(
            &[
                (x - w * 0.55, y),
                (x - w * 0.2, y - h * 0.7),
                (x + w * 0.4, y - h * 0.4),
                (x + w * 0.55, y),
            ],
            0xffb040,
            0.95,
        );
        c.poly(
            &[(x - w * 0.25, y), (x, y - h * 0.4), (x + w * 0.25, y)],
            0xfff0a0,
            1.0,
        );
    }
}

pub fn draw_wisp(c: &mut Canvas, e: &Ent, x: f32, y: f32, time: f32) {
    let bob = sinf(e.t * 3.0 + e.dir) * 4.0;
    let y = y + bob;
    c.glow(x, y, 46.0, 0x62f2ff, 0.55);
    // flame trail
    for k in 1..5 {
        let t = k as f32 / 5.0;
        c.circle(
            x - sinf(time * 5.0 + k as f32) * 3.0,
            y + t * 14.0,
            8.0 * (1.0 - t * 0.6),
            0x62f2ff,
            0.4 * (1.0 - t),
        );
    }
    c.poly(
        &[
            (x, y - 17.0),
            (x - 9.0, y + 3.0),
            (x, y + 10.0),
            (x + 9.0, y + 3.0),
        ],
        0x62f2ff,
        0.95,
    );
    c.circle(x, y, 8.0, 0xb8fbff, 1.0);
    c.circle(x - 2.6, y - 1.0, 1.8, 0x203060, 1.0);
    c.circle(x + 2.6, y - 1.0, 1.8, 0x203060, 1.0);
    c.line(x - 2.0, y + 3.2, x + 2.0, y + 3.2, 1.2, 0x203060, 1.0);
}

pub fn draw_bolt(c: &mut Canvas, e: &Ent, x: f32, y: f32, time: f32) {
    if e.state == 5 {
        // low shockwave
        let h = 22.0 + 4.0 * sinf(time * 30.0);
        c.glow(x, y, 40.0, 0xff7ad0, 0.6);
        c.poly(
            &[
                (x - 10.0, y + 12.0),
                (x, y + 12.0 - h),
                (x + 10.0, y + 12.0),
            ],
            0xff7ad0,
            0.9,
        );
        c.poly(
            &[
                (x - 5.0, y + 12.0),
                (x, y + 12.0 - h * 0.6),
                (x + 5.0, y + 12.0),
            ],
            0xffffff,
            0.9,
        );
        return;
    }
    let len = (e.vx * e.vx + e.vy * e.vy).sqrt().max(1.0);
    c.line(
        x,
        y,
        x - e.vx / len * 22.0,
        y - e.vy / len * 22.0,
        4.0,
        0xff6a30,
        0.45,
    );
    c.glow(x, y, 24.0, 0xff7a40, 0.85);
    c.circle(x, y, 5.2, 0xffa050, 1.0);
    c.circle(x, y, 2.6, 0xffffff, 1.0);
}

pub fn draw_bloom(c: &mut Canvas, e: &Ent, x: f32, y: f32, time: f32) {
    let squash = if e.state == 1 { 0.5 } else { 1.0 };
    let sway = sinf(time * 2.0 + x * 0.05) * 1.5;
    c.ellipse(x, y + 7.0, 18.0, 4.0, 0x2a5a2a, 0.8);
    c.line(x, y + 7.0, x + sway, y - 2.0 * squash, 3.0, 0x3a9a3a, 1.0);
    for k in 0..6 {
        let a = k as f32 * 1.0472 + 0.3;
        let px = x + sway + a.cos() * 13.0;
        let py = y - 3.0 * squash + a.sin() * 5.0 * squash;
        c.ellipse(
            px,
            py,
            7.0,
            4.0 * squash,
            if k % 2 == 0 { 0xff7aa8 } else { 0xffa0c0 },
            1.0,
        );
    }
    c.circle(x + sway, y - 3.0 * squash, 5.0, 0xffe066, 1.0);
    c.glow(x + sway, y - 4.0, 28.0, 0xff9ac0, 0.35);
}

pub fn draw_lamp(c: &mut Canvas, x: f32, y: f32, lit: bool, time: f32) {
    c.line(x, y, x, y - 54.0, 4.0, 0x3a3048, 1.0);
    c.round_rect(x - 8.0, y - 4.0, 16.0, 4.0, 1.0, 0x3a3048, 1.0);
    c.round_rect(x - 8.0, y - 70.0, 16.0, 18.0, 4.0, 0x2a2438, 1.0);
    if lit {
        let fl = 0.85 + 0.15 * sinf(time * 9.0 + x);
        c.glow(x, y - 62.0, 120.0 * fl, 0xffd890, 0.7);
        c.circle(x, y - 62.0, 6.5, 0xfff0b0, 1.0);
        c.circle(x, y - 62.0, 3.0, 0xffffff, 1.0);
    } else {
        c.circle(x, y - 62.0, 5.5, 0x5a5068, 1.0);
        c.glow(x, y - 62.0, 40.0, 0x8a80a8, 0.2);
    }
}

pub fn draw_gate(c: &mut Canvas, x: f32, y: f32, open: bool, time: f32, pal: &Pal) {
    // y is gate centre; arch rises to y-80
    let top = y - 64.0;
    c.glow(x, y, 160.0, pal.glow, if open { 0.55 } else { 0.18 });
    c.round_rect(x - 36.0, top, 12.0, 112.0, 3.0, 0x3a3450, 1.0);
    c.round_rect(x + 24.0, top, 12.0, 112.0, 3.0, 0x3a3450, 1.0);
    c.round_rect(x - 38.0, top - 8.0, 76.0, 14.0, 5.0, 0x4a4468, 1.0);
    if open {
        for k in 0..6 {
            let t = (time * 0.7 + k as f32 / 6.0) % 1.0;
            let xx = x - 22.0 + k as f32 * 8.5;
            c.line(
                xx,
                y + 48.0,
                xx + sinf(time * 2.0 + k as f32) * 4.0,
                y + 48.0 - t * 112.0,
                3.0,
                pal.glow,
                0.5 * (1.0 - t),
            );
        }
        c.poly(
            &[
                (x - 22.0, y + 48.0),
                (x + 22.0, y + 48.0),
                (x + 20.0, top + 6.0),
                (x - 20.0, top + 6.0),
            ],
            pal.glow,
            0.28,
        );
        c.circle(x, top - 16.0, 8.0 + sinf(time * 4.0), 0xffffff, 0.9);
        c.glow(x, top - 16.0, 50.0, 0xffe9a0, 0.8);
    } else {
        c.poly(
            &[
                (x - 22.0, y + 48.0),
                (x + 22.0, y + 48.0),
                (x + 20.0, top + 6.0),
                (x - 20.0, top + 6.0),
            ],
            0x1a1630,
            0.85,
        );
        c.line(x - 20.0, top + 6.0, x + 20.0, y + 48.0, 1.2, 0x6a6488, 0.6);
    }
}

pub fn draw_spikes(c: &mut Canvas, tx: i32, ty: i32, t: Tile, camx: f32, camy: f32, pal: &Pal) {
    let x0 = tx as f32 * T - camx;
    let y0 = ty as f32 * T - camy;
    let n = 3;
    for k in 0..n {
        let f = (k as f32 + 0.5) / n as f32 * T;
        let w = T / n as f32 * 0.5;
        let (a, b, tip) = match t {
            SPIKE_UP => (
                (x0 + f - w, y0 + T),
                (x0 + f + w, y0 + T),
                (x0 + f, y0 + 10.0),
            ),
            SPIKE_DOWN => ((x0 + f - w, y0), (x0 + f + w, y0), (x0 + f, y0 + T - 10.0)),
            SPIKE_LEFT => (
                (x0 + T, y0 + f - w),
                (x0 + T, y0 + f + w),
                (x0 + 10.0, y0 + f),
            ),
            _ => ((x0, y0 + f - w), (x0, y0 + f + w), (x0 + T - 10.0, y0 + f)),
        };
        c.poly(&[a, b, tip], 0x8e90a8, 1.0);
        c.poly(
            &[a, ((a.0 + tip.0) / 2.0, (a.1 + tip.1) / 2.0), tip],
            0xd8dcf0,
            0.9,
        );
        c.circle(tip.0, tip.1, 1.4, 0xffffff, 0.9);
    }
    let _ = pal;
}

pub fn draw_glass(c: &mut Canvas, tx: i32, ty: i32, camx: f32, camy: f32, time: f32, near: bool) {
    let x = tx as f32 * T - camx;
    let y = ty as f32 * T - camy;
    c.fill_rect(x, y, T, T, 0x9ae8ff, 0.30 + if near { 0.1 } else { 0.0 });
    c.rect_outline(x, y, T, T, 0xd8f8ff, 0.6);
    c.line(
        x + 4.0,
        y + T - 6.0,
        x + T - 8.0,
        y + 4.0,
        2.0,
        0xffffff,
        0.45,
    );
    c.line(
        x + 12.0,
        y + T - 3.0,
        x + T - 4.0,
        y + 12.0,
        1.2,
        0xffffff,
        0.3,
    );
    let sh = ((time * 0.8 + tx as f32 * 0.3 + ty as f32 * 0.2) % 3.0) / 3.0;
    if sh < 0.3 {
        c.glow(x + sh * 3.0 * T, y + T / 2.0, 14.0, 0xffffff, 0.5);
    }
}

pub fn draw_crumble(c: &mut Canvas, tx: i32, ty: i32, camx: f32, camy: f32, timer: f32, pal: &Pal) {
    let shake = if timer > 0.0 {
        (timer * 90.0).sin() * 1.8
    } else {
        0.0
    };
    let x = tx as f32 * T - camx + shake;
    let y = ty as f32 * T - camy;
    c.round_rect(x, y, T, T, 4.0, 0x8e7a5a, 1.0);
    c.round_rect(
        x,
        y,
        T,
        9.0,
        4.0,
        lerp_color(0xb8a47e, pal.surface, 0.35),
        1.0,
    );
    c.line(x + 6.0, y + 12.0, x + 14.0, y + 22.0, 1.4, 0x4a3c28, 1.0);
    c.line(x + 14.0, y + 22.0, x + 11.0, y + 30.0, 1.4, 0x4a3c28, 1.0);
    c.line(x + 24.0, y + 8.0, x + 20.0, y + 18.0, 1.4, 0x4a3c28, 1.0);
    if timer > 0.0 {
        c.fill_rect(x, y, T, T, 0xff9a52, 0.18);
    }
}

pub fn draw_boss(c: &mut Canvas, b: &Boss, x: f32, y: f32, time: f32, pal: &Pal) {
    let stunned = b.state == BState::Stun;
    let dead = b.state == BState::Dead;
    let tell = matches!(
        b.state,
        BState::Fan | BState::Rain | BState::SlamTell | BState::Gale | BState::Summon
    );
    let hit = b.hit_t > 0.0;
    let pulse = 0.5 + 0.5 * sinf(time * 5.0);
    let col_core = if stunned {
        0xff5a3a
    } else if tell {
        0xffd060
    } else {
        0xb8a0ff
    };
    // aura
    c.glow(
        x,
        y,
        200.0,
        if stunned { 0xff6a40 } else { 0x8a6aff },
        if dead { 0.2 } else { 0.4 + 0.15 * pulse },
    );
    if dead {
        // collapsing: shards drift
    }
    // shards orbiting; they fly open when stunned
    let open = if stunned { 1.0 } else { 0.0 };
    let n = 8;
    for k in 0..n {
        let a = time * (0.6 + 0.1 * (k % 3) as f32) + k as f32 * std::f32::consts::TAU / n as f32;
        let r = BOSS_R * (1.05 + 0.5 * open) + 8.0 * sinf(time * 2.0 + k as f32);
        let sx = x + a.cos() * r;
        let sy = y + a.sin() * r * 0.82;
        let ang = a + 1.57;
        let (ca, sa) = (ang.cos(), ang.sin());
        let len = 30.0;
        let wid = 11.0;
        let pts = [
            (
                sx + ca * len * 0.0 - sa * 0.0 + sa * (-len),
                sy + sa * 0.0 - ca * len * 0.0 - ca * len,
            ),
            (sx + ca * wid, sy + sa * wid),
            (sx + sa * len * 0.55, sy - ca * len * 0.55),
            (sx - ca * wid, sy - sa * wid),
        ];
        let base = if hit {
            0xffffff
        } else {
            lerp_color(0x5a4aa8, 0x9a86f0, (k % 3) as f32 / 2.0)
        };
        c.poly(&pts, base, 1.0);
        c.poly(
            &[
                pts[0],
                pts[1],
                ((pts[1].0 + pts[3].0) / 2.0, (pts[1].1 + pts[3].1) / 2.0),
            ],
            0xd8ccff,
            0.55,
        );
        c.glow(sx, sy, 20.0, 0xb8a0ff, 0.25);
    }
    // shell
    let shell_a = if stunned { 0.35 } else { 1.0 };
    c.circle(x, y, BOSS_R, 0x241a58, shell_a);
    c.circle(
        x,
        y,
        BOSS_R * 0.86,
        lerp_color(0x4a3a9a, 0x7a62d8, pulse * 0.4),
        shell_a,
    );
    c.ring(
        x,
        y,
        BOSS_R * 0.95,
        BOSS_R * 0.95,
        2.0,
        0xe0d8ff,
        0.5 * shell_a,
    );
    // cracks for each hit taken
    let dmg = b.max_hp - b.hp;
    for k in 0..dmg {
        let a = k as f32 * 2.1 + 0.5;
        c.line(
            x,
            y,
            x + a.cos() * BOSS_R * 0.95,
            y + a.sin() * BOSS_R * 0.95,
            2.4,
            0xffd8a0,
            0.9,
        );
    }
    // core
    let cr = 22.0 + 4.0 * pulse + if stunned { 8.0 } else { 0.0 };
    c.glow(x, y, cr * 3.0, col_core, 0.9);
    c.circle(x, y, cr, scale(col_core, 0.85), 1.0);
    c.circle(x, y, cr * 0.6, 0xffffff, 0.9);
    // angry eye slit
    if !stunned && !dead {
        let ex = (b.aim.cos()) * 5.0;
        let ey = (b.aim.sin()) * 3.0;
        c.poly(
            &[
                (x - 12.0 + ex, y + ey),
                (x + ex, y - 5.0 + ey),
                (x + 12.0 + ex, y + ey),
                (x + ex, y + 4.0 + ey),
            ],
            0x1a0a30,
            0.9,
        );
        c.circle(x + ex, y + ey, 2.4, 0xff5a8a, 1.0);
    }
    // lightning arcs
    for k in 0..3 {
        let a = time * 3.0 + k as f32 * 2.0;
        if sinf(a * 5.0) > 0.4 {
            let mut px = x;
            let mut py = y;
            for s in 0..5 {
                let ang = a + s as f32 * 0.25;
                let nx = x
                    + ang.cos() * (BOSS_R * 0.4 + s as f32 * 22.0)
                    + sinf(time * 40.0 + s as f32) * 5.0;
                let ny = y + ang.sin() * (BOSS_R * 0.4 + s as f32 * 22.0) * 0.85;
                c.line(px, py, nx, ny, 1.6, 0xe8e0ff, 0.8);
                px = nx;
                py = ny;
            }
        }
    }
    let _ = pal;
}

/// Telegraphs for boss attacks, drawn in world space.
pub fn draw_boss_telegraph(
    c: &mut Canvas,
    b: &Boss,
    x: f32,
    y: f32,
    floor_y: f32,
    camx: f32,
    time: f32,
) {
    match b.state {
        BState::Fan => {
            let k = (b.t / 1.0).clamp(0.0, 1.0);
            for i in 0..9 {
                let f = i as f32 / 8.0 - 0.5;
                let a = b.aim + f * 0.95;
                c.line(
                    x,
                    y,
                    x + a.cos() * 900.0,
                    y + a.sin() * 900.0,
                    1.0 + 2.0 * k,
                    0xff7ad0,
                    0.1 + 0.28 * k,
                );
            }
        }
        BState::SlamTell => {
            let sx = b.spot - camx;
            let k = (b.t / 1.35).clamp(0.0, 1.0);
            c.line(sx, y, sx, floor_y, 2.0 + 6.0 * k, 0xff5a3a, 0.12 + 0.3 * k);
            let r = 80.0 * (1.0 - k) + 40.0;
            c.ring(sx, floor_y - 6.0, r, r * 0.25, 4.0, 0xff5a3a, 0.5 + 0.5 * k);
            c.ring(sx, floor_y - 6.0, 44.0, 11.0, 3.0, 0xffffff, 0.6 * k);
            c.glow(sx, floor_y - 6.0, 110.0, 0xff5a3a, 0.25 + 0.4 * k);
        }
        BState::Gale => {
            for k in 0..14 {
                let yy = 80.0 + k as f32 * 46.0;
                let off = (time * 700.0 + k as f32 * 133.0) % 1400.0 - 100.0;
                let dir = if b.aim.cos() >= 0.0 { 1.0 } else { -1.0 };
                let xx = if dir > 0.0 { off } else { 1280.0 - off };
                c.line(xx, yy, xx + dir * 90.0, yy, 2.0, 0xe8f0ff, 0.35);
            }
        }
        BState::Stun => {
            // open-for-attack marker
            let bob = sinf(time * 6.0) * 4.0;
            c.poly(
                &[
                    (x - 10.0, y - 90.0 + bob),
                    (x + 10.0, y - 90.0 + bob),
                    (x, y - 72.0 + bob),
                ],
                0xffe066,
                0.95,
            );
            c.text_center(x, y - 116.0 + bob, "HIT IT FAST!", 2.0, 0xffe066, 0.95);
        }
        _ => {}
    }
}
