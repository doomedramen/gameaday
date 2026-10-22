//! Procedural vector sprites for units, props and pickups. Every sprite is
//! drawn relative to its feet at (x, y) and tinted by the local light.
#![allow(clippy::too_many_arguments)]

use super::canvas::{Canvas, hash2, lerp_color, scale};
use super::palette::Palette;
use crate::map::{Biome, Pickup};
use crate::units::Kind;

/// Per-draw parameters.
#[derive(Clone, Copy, Debug)]
pub struct Look {
    pub facing: f32,
    pub t: f32,
    /// Light multiplier 0..1.
    pub light: f32,
    pub alpha: f32,
    /// White hit flash 0..1.
    pub flash: f32,
    pub shrouded: bool,
    pub wounded: bool,
}

fn tint(c: u32, l: &Look) -> u32 {
    let base = scale(c, l.light);
    if l.flash > 0.0 {
        lerp_color(base, 0xffffff, l.flash)
    } else {
        base
    }
}

/// Visual height of a unit sprite above its feet (for HP bars).
pub fn unit_height(k: Kind) -> f32 {
    match k {
        Kind::Warden => 44.0,
        Kind::Ranger => 44.0,
        Kind::Lamplighter => 56.0,
        Kind::Shade => 40.0,
        Kind::Wisp => 52.0,
        Kind::Snuffer => 36.0,
        Kind::Brute => 48.0,
        Kind::Hexer => 50.0,
        Kind::HollowKing => 96.0,
    }
}

pub fn draw_unit(c: &mut Canvas, k: Kind, x: f32, y: f32, l: Look) {
    match k {
        Kind::Warden => warden(c, x, y, l),
        Kind::Ranger => ranger(c, x, y, l),
        Kind::Lamplighter => lamplighter(c, x, y, l),
        Kind::Shade => shade(c, x, y, l),
        Kind::Wisp => wisp(c, x, y, l),
        Kind::Snuffer => snuffer(c, x, y, l),
        Kind::Brute => brute(c, x, y, l),
        Kind::Hexer => hexer(c, x, y, l),
        Kind::HollowKing => king(c, x, y, l),
    }
}

fn breathe(l: &Look, speed: f32, amp: f32) -> f32 {
    (l.t * speed).sin() * amp
}

fn warden(c: &mut Canvas, x: f32, y: f32, l: Look) {
    let f = l.facing;
    let a = l.alpha;
    let b = breathe(&l, 2.0, 0.8);
    // Legs
    c.fill_rect(x - 6.0, y - 10.0, 4.0, 10.0, tint(0x2a3448, &l), a);
    c.fill_rect(x + 2.0, y - 10.0, 4.0, 10.0, tint(0x2a3448, &l), a);
    // Sword behind
    c.line(
        x - f * 7.0,
        y - 14.0,
        x - f * 13.0,
        y - 40.0 + b,
        2.5,
        tint(0xd8dde8, &l),
        a,
    );
    c.line(
        x - f * 9.0,
        y - 18.0,
        x - f * 5.0,
        y - 19.0,
        3.0,
        tint(0x8a6a3a, &l),
        a,
    );
    // Body / cloak
    c.poly(
        &[
            (x - 10.0, y - 8.0),
            (x + 10.0, y - 8.0),
            (x + 8.0, y - 27.0 + b),
            (x - 8.0, y - 27.0 + b),
        ],
        tint(0x3c5888, &l),
        a,
    );
    c.poly(
        &[
            (x - 6.0, y - 25.0 + b),
            (x + 6.0, y - 25.0 + b),
            (x + 5.0, y - 13.0),
            (x - 5.0, y - 13.0),
        ],
        tint(0x8c9cb8, &l),
        a,
    );
    c.fill_rect(x - 10.0, y - 13.0, 20.0, 3.0, tint(0x5a4024, &l), a);
    // Hip lantern
    c.circle(x - f * 7.0, y - 10.0, 2.5, 0xffd080, a);
    c.glow(x - f * 7.0, y - 10.0, 14.0, 0xff9a40, 0.35 * a);
    // Head + helm
    c.circle(x, y - 32.0 + b, 6.5, tint(0x9aa6bc, &l), a);
    c.fill_rect(x - 6.5, y - 33.0 + b, 13.0, 3.0, tint(0x2a3040, &l), a);
    c.poly(
        &[
            (x - 2.0, y - 38.0 + b),
            (x + 2.0, y - 38.0 + b),
            (x - f * 5.0, y - 46.0 + b),
        ],
        tint(0xc03a3a, &l),
        a,
    );
    // Shield in front
    let sx = x + f * 9.0;
    let sy = y - 22.0 + b;
    c.poly(
        &[
            (sx - 6.0, sy - 9.0),
            (sx + 6.0, sy - 9.0),
            (sx + 6.0, sy + 3.0),
            (sx, sy + 10.0),
            (sx - 6.0, sy + 3.0),
        ],
        tint(0xc8a050, &l),
        a,
    );
    c.poly(
        &[
            (sx - 4.5, sy - 7.5),
            (sx + 4.5, sy - 7.5),
            (sx + 4.5, sy + 2.0),
            (sx, sy + 7.5),
            (sx - 4.5, sy + 2.0),
        ],
        tint(0x2c4a7c, &l),
        a,
    );
    c.circle(sx, sy - 1.0, 2.2, tint(0xffe090, &l), a);
}

fn ranger(c: &mut Canvas, x: f32, y: f32, l: Look) {
    let f = l.facing;
    let a = l.alpha;
    let b = breathe(&l, 2.2, 0.7);
    c.fill_rect(x - 5.0, y - 9.0, 3.5, 9.0, tint(0x3a2a1e, &l), a);
    c.fill_rect(x + 1.5, y - 9.0, 3.5, 9.0, tint(0x3a2a1e, &l), a);
    // Quiver behind
    c.poly(
        &[
            (x - f * 4.0, y - 30.0 + b),
            (x - f * 9.0, y - 30.0 + b),
            (x - f * 7.0, y - 14.0),
            (x - f * 3.0, y - 14.0),
        ],
        tint(0x6a4428, &l),
        a,
    );
    for i in 0..3 {
        c.line(
            x - f * (5.0 + i as f32 * 1.5),
            y - 30.0 + b,
            x - f * (6.0 + i as f32 * 1.5),
            y - 36.0 + b,
            1.5,
            tint(0xe8e0d0, &l),
            a,
        );
    }
    // Cloak
    c.poly(
        &[
            (x - 9.0, y - 7.0),
            (x + 9.0, y - 7.0),
            (x + 6.0, y - 28.0 + b),
            (x - 6.0, y - 28.0 + b),
        ],
        tint(0x3a6a36, &l),
        a,
    );
    c.fill_rect(x - 8.0, y - 15.0, 16.0, 2.5, tint(0x6a4428, &l), a);
    // Hood
    c.poly(
        &[
            (x - 8.0, y - 25.0 + b),
            (x + 8.0, y - 25.0 + b),
            (x + f * 2.0, y - 43.0 + b),
        ],
        tint(0x2c5428, &l),
        a,
    );
    c.circle(x + f * 2.0, y - 31.0 + b, 3.6, tint(0xc89a74, &l), a);
    // Bow
    let bx = x + f * 9.0;
    let by = y - 24.0 + b;
    let mut prev = None;
    for i in 0..=10 {
        let ang = -1.15 + 2.3 * i as f32 / 10.0;
        let p = (bx + f * ang.cos() * 6.0 - f * 4.0, by + ang.sin() * 14.0);
        if let Some(q) = prev {
            let (qx, qy): (f32, f32) = q;
            c.line(qx, qy, p.0, p.1, 2.2, tint(0x8a5a2a, &l), a);
        }
        prev = Some(p);
    }
    c.line(
        bx - f * 1.7,
        by - 12.8,
        bx - f * 1.7,
        by + 12.8,
        1.0,
        tint(0xeeeeee, &l),
        a * 0.8,
    );
    c.circle(x - f * 6.0, y - 11.0, 2.0, 0xffd080, a);
    c.glow(x - f * 6.0, y - 11.0, 12.0, 0xff9a40, 0.3 * a);
}

fn lamplighter(c: &mut Canvas, x: f32, y: f32, l: Look) {
    let f = l.facing;
    let a = l.alpha;
    let b = breathe(&l, 1.8, 0.8);
    // Robe
    c.poly(
        &[
            (x - 11.0, y - 3.0),
            (x + 11.0, y - 3.0),
            (x + 6.0, y - 29.0 + b),
            (x - 6.0, y - 29.0 + b),
        ],
        tint(0x8a4e22, &l),
        a,
    );
    c.poly(
        &[
            (x - 3.0, y - 3.0),
            (x + 3.0, y - 3.0),
            (x + 2.0, y - 28.0 + b),
            (x - 2.0, y - 28.0 + b),
        ],
        tint(0xd8a050, &l),
        a,
    );
    // Head + hat
    c.circle(x, y - 33.0 + b, 5.5, tint(0xe0b08a, &l), a);
    c.ellipse(x, y - 37.0 + b, 10.0, 2.6, tint(0x3a2618, &l), a);
    c.poly(
        &[
            (x - 5.0, y - 37.5 + b),
            (x + 5.0, y - 37.5 + b),
            (x + f * 3.0, y - 49.0 + b),
        ],
        tint(0x4a3020, &l),
        a,
    );
    // Staff + lantern
    let top = (x + f * 13.0, y - 50.0 + b * 0.5);
    c.line(x + f * 10.0, y, top.0, top.1, 2.5, tint(0x6a4428, &l), a);
    c.line(
        top.0,
        top.1,
        top.0 + f * 4.0,
        top.1 + 2.0,
        2.0,
        tint(0x6a4428, &l),
        a,
    );
    let lx = top.0 + f * 4.0;
    let ly = top.1 + 8.0;
    let flick = 0.85 + 0.15 * (l.t * 11.0).sin() * (l.t * 7.3).cos();
    c.round_rect(lx - 4.0, ly - 6.0, 8.0, 11.0, 2.0, 0x5a4020, a);
    c.round_rect(lx - 2.8, ly - 4.5, 5.6, 8.0, 1.5, 0xffe6a0, a);
    c.glow(lx, ly, 34.0 * flick, 0xffa040, 0.55 * a);
    c.glow(lx, ly, 10.0, 0xffffff, 0.5 * a);
    // Hand
    c.circle(x + f * 9.0, y - 20.0 + b, 2.5, tint(0xe0b08a, &l), a);
}

fn shade(c: &mut Canvas, x: f32, y: f32, l: Look) {
    let f = l.facing;
    let a = l.alpha * if l.shrouded { 0.75 } else { 1.0 };
    let t = l.t;
    let body = tint(0x2e1c44, &l);
    let edge = tint(0x5a3a80, &l);
    // Trailing smoke tail
    for i in 0..5 {
        let k = i as f32;
        c.circle(
            x - f * (6.0 + k * 4.0) + (t * 3.0 + k).sin() * 2.0,
            y - 8.0 - k * 1.5 + (t * 2.0 + k).cos() * 1.5,
            7.0 - k * 1.1,
            body,
            a * (0.7 - k * 0.12),
        );
    }
    let sway = (t * 2.6).sin() * 2.0;
    c.ellipse(x, y - 9.0, 12.0, 8.0, edge, a * 0.8);
    c.ellipse(x, y - 9.0, 10.5, 7.0, body, a);
    c.circle(x + sway * 0.5, y - 20.0, 10.0, body, a);
    c.circle(x + f * 2.0 + sway, y - 30.0, 8.0, body, a);
    // Horn-like wisps
    c.poly(
        &[
            (x + f * 2.0 + sway - 6.0, y - 34.0),
            (x + f * 2.0 + sway - 3.0, y - 35.0),
            (x + f * -3.0 + sway, y - 44.0),
        ],
        body,
        a,
    );
    c.poly(
        &[
            (x + f * 2.0 + sway + 6.0, y - 34.0),
            (x + f * 2.0 + sway + 3.0, y - 35.0),
            (x + f * 7.0 + sway, y - 43.0),
        ],
        body,
        a,
    );
    // Claws
    c.line(x + f * 8.0, y - 18.0, x + f * 15.0, y - 11.0, 2.0, edge, a);
    c.line(x + f * 8.0, y - 16.0, x + f * 14.0, y - 7.0, 2.0, edge, a);
    // Eyes
    let ex = x + f * 4.5 + sway;
    let eye = if l.shrouded { 0xc070ff } else { 0x9ff4ff };
    c.circle(ex - 3.0, y - 31.0, 1.6, eye, a);
    c.circle(ex + 3.0, y - 31.0, 1.6, eye, a);
    c.glow(ex, y - 31.0, 10.0, eye, 0.4 * a);
    c.glow(x, y - 18.0, 22.0, 0x6a30a0, 0.18 * a);
}

fn wisp(c: &mut Canvas, x: f32, y: f32, l: Look) {
    let a = l.alpha * if l.shrouded { 0.75 } else { 1.0 };
    let t = l.t;
    let hy = y - 30.0 + (t * 2.4).sin() * 4.0;
    for i in 0..4 {
        let k = i as f32;
        c.circle(
            x + (t * 3.0 + k).sin() * 3.0,
            hy + 8.0 + k * 5.0,
            3.0 - k * 0.6,
            0x60e0d0,
            a * (0.5 - k * 0.1),
        );
    }
    c.glow(x, hy, 26.0, 0x30c0b0, 0.5 * a);
    // Orbiting ring
    let rx = 12.0;
    let ry = 4.0;
    c.ring(x, hy, rx, ry, 1.5, tint(0xa0fff0, &l), a * 0.8);
    let ang = t * 3.0;
    c.circle(x + ang.cos() * rx, hy + ang.sin() * ry, 2.0, 0xe0fff8, a);
    c.circle(x, hy, 6.0, lerp_color(0x80fff0, 0xffffff, l.flash), a);
    c.circle(x - 1.5, hy - 1.5, 2.5, 0xffffff, a);
}

fn snuffer(c: &mut Canvas, x: f32, y: f32, l: Look) {
    let f = l.facing;
    let a = l.alpha * if l.shrouded { 0.8 } else { 1.0 };
    let b = breathe(&l, 5.0, 1.0);
    // Pole with snuffer bell
    let px = x + f * 16.0;
    let py = y - 30.0 + b;
    c.line(x - f * 7.0, y - 4.0, px, py, 2.0, tint(0x5a4a3a, &l), a);
    c.poly(
        &[(px - 4.0, py + 1.0), (px + 4.0, py + 1.0), (px, py - 7.0)],
        tint(0xa08a50, &l),
        a,
    );
    // Body
    c.ellipse(x, y - 9.0 + b * 0.5, 9.0, 8.0, tint(0x231a14, &l), a);
    c.circle(x + f * 5.0, y - 16.0 + b, 5.0, tint(0x2c2018, &l), a);
    // Nose
    c.poly(
        &[
            (x + f * 8.0, y - 17.0 + b),
            (x + f * 8.0, y - 14.0 + b),
            (x + f * 14.0, y - 13.0 + b),
        ],
        tint(0x3a2a20, &l),
        a,
    );
    // Bell hat
    c.poly(
        &[
            (x + f * 5.0 - 6.0, y - 18.0 + b),
            (x + f * 5.0 + 6.0, y - 18.0 + b),
            (x + f * 4.0, y - 33.0 + b),
        ],
        tint(0x8a7440, &l),
        a,
    );
    c.fill_rect(
        x + f * 5.0 - 7.0,
        y - 19.0 + b,
        14.0,
        2.0,
        tint(0x5a4a28, &l),
        a,
    );
    c.circle(x + f * 7.0, y - 16.0 + b, 1.3, 0xffe040, a);
    c.glow(x + f * 7.0, y - 16.0 + b, 6.0, 0xffd040, 0.4 * a);
}

fn brute(c: &mut Canvas, x: f32, y: f32, l: Look) {
    let f = l.facing;
    let a = l.alpha * if l.shrouded { 0.85 } else { 1.0 };
    let b = breathe(&l, 1.5, 1.2);
    let body = tint(0x2a2034, &l);
    // Arms
    c.ellipse(x - 17.0, y - 18.0 + b, 6.0, 11.0, body, a);
    c.ellipse(x + 17.0, y - 18.0 + b, 6.0, 11.0, body, a);
    c.circle(x - 18.0, y - 7.0, 6.0, tint(0x3a2c44, &l), a);
    c.circle(x + 18.0, y - 7.0, 6.0, tint(0x3a2c44, &l), a);
    c.poly(
        &[
            (x - 15.0, y - 3.0),
            (x + 15.0, y - 3.0),
            (x + 16.0, y - 26.0 + b),
            (x + 8.0, y - 40.0 + b),
            (x - 8.0, y - 40.0 + b),
            (x - 16.0, y - 26.0 + b),
        ],
        body,
        a,
    );
    // Horns
    c.poly(
        &[
            (x - 8.0, y - 38.0 + b),
            (x - 4.0, y - 40.0 + b),
            (x - 14.0, y - 50.0 + b),
        ],
        tint(0xc8b8a0, &l),
        a,
    );
    c.poly(
        &[
            (x + 8.0, y - 38.0 + b),
            (x + 4.0, y - 40.0 + b),
            (x + 14.0, y - 50.0 + b),
        ],
        tint(0xc8b8a0, &l),
        a,
    );
    // Molten veins
    let pulse = 0.6 + 0.4 * (l.t * 3.0).sin();
    c.line(
        x - 6.0,
        y - 30.0 + b,
        x - 2.0,
        y - 18.0,
        1.6,
        0xff6a20,
        a * pulse,
    );
    c.line(
        x - 2.0,
        y - 18.0,
        x - 7.0,
        y - 8.0,
        1.6,
        0xff6a20,
        a * pulse,
    );
    c.line(
        x + 7.0,
        y - 26.0 + b,
        x + 3.0,
        y - 12.0,
        1.6,
        0xff6a20,
        a * pulse,
    );
    c.glow(x, y - 20.0, 22.0, 0xff4010, 0.18 * a * pulse);
    // Eyes
    c.circle(x + f * 2.0 - 3.5, y - 33.0 + b, 1.8, 0xff3020, a);
    c.circle(x + f * 2.0 + 3.5, y - 33.0 + b, 1.8, 0xff3020, a);
    c.glow(x + f * 2.0, y - 33.0 + b, 9.0, 0xff3020, 0.4 * a);
}

fn hexer(c: &mut Canvas, x: f32, y: f32, l: Look) {
    let f = l.facing;
    let a = l.alpha * if l.shrouded { 0.8 } else { 1.0 };
    let b = breathe(&l, 1.7, 1.5);
    // Staff
    c.line(
        x + f * 10.0,
        y,
        x + f * 11.0,
        y - 46.0 + b,
        2.0,
        tint(0x3a2a3a, &l),
        a,
    );
    c.poly(
        &[
            (x + f * 11.0, y - 54.0 + b),
            (x + f * 11.0 + 3.0, y - 48.0 + b),
            (x + f * 11.0, y - 43.0 + b),
            (x + f * 11.0 - 3.0, y - 48.0 + b),
        ],
        0xd080ff,
        a,
    );
    c.glow(x + f * 11.0, y - 48.0 + b, 14.0, 0xb050ff, 0.5 * a);
    // Robe
    c.poly(
        &[
            (x - 9.0, y - 1.0),
            (x + 9.0, y - 1.0),
            (x + 4.0, y - 34.0 + b),
            (x - 4.0, y - 34.0 + b),
        ],
        tint(0x3a1a4e, &l),
        a,
    );
    c.poly(
        &[
            (x - 7.0, y - 30.0 + b),
            (x + 7.0, y - 30.0 + b),
            (x + f * 1.0, y - 47.0 + b),
        ],
        tint(0x2a1238, &l),
        a,
    );
    c.circle(x + f * 1.5, y - 34.0 + b, 3.6, 0x08040c, a);
    c.circle(x + f * 1.5 - 1.6, y - 34.5 + b, 1.0, 0xe0a0ff, a);
    c.circle(x + f * 1.5 + 1.6, y - 34.5 + b, 1.0, 0xe0a0ff, a);
    // Orbiting runes
    for i in 0..3 {
        let ang = l.t * 1.6 + i as f32 * 2.094;
        let rx = x + ang.cos() * 15.0;
        let ry = y - 24.0 + ang.sin() * 5.0;
        c.poly(
            &[
                (rx, ry - 3.5),
                (rx + 2.5, ry),
                (rx, ry + 3.5),
                (rx - 2.5, ry),
            ],
            0xe0b0ff,
            a * 0.9,
        );
        c.glow(rx, ry, 7.0, 0xa040ff, 0.35 * a);
    }
}

fn king(c: &mut Canvas, x: f32, y: f32, l: Look) {
    let f = l.facing;
    let a = l.alpha;
    let b = breathe(&l, 1.2, 2.0);
    let t = l.t;
    // Aura of darkness at the feet
    for i in 0..8 {
        let ang = t * 0.7 + i as f32 * 0.785;
        c.circle(
            x + ang.cos() * 26.0,
            y - 4.0 + ang.sin() * 9.0,
            10.0 + (t * 2.0 + i as f32).sin() * 3.0,
            0x0a040c,
            a * 0.55,
        );
    }
    c.glow_ellipse(x, y - 6.0, 60.0, 22.0, 0x900020, 0.35 * a);
    // Cape
    let cape = tint(0x140a18, &l);
    c.poly(
        &[
            (x - 30.0, y - 2.0),
            (x - 18.0, y + 2.0 + (t * 2.0).sin() * 2.0),
            (x, y - 1.0),
            (x + 18.0, y + 2.0 + (t * 2.3).cos() * 2.0),
            (x + 30.0, y - 2.0),
            (x + 18.0, y - 56.0 + b),
            (x - 18.0, y - 56.0 + b),
        ],
        cape,
        a,
    );
    c.poly(
        &[
            (x - 24.0, y - 4.0),
            (x - 14.0, y - 52.0 + b),
            (x - 11.0, y - 52.0 + b),
            (x - 19.0, y - 4.0),
        ],
        tint(0x7a0a1e, &l),
        a,
    );
    c.poly(
        &[
            (x + 24.0, y - 4.0),
            (x + 14.0, y - 52.0 + b),
            (x + 11.0, y - 52.0 + b),
            (x + 19.0, y - 4.0),
        ],
        tint(0x7a0a1e, &l),
        a,
    );
    // Armoured chest
    c.poly(
        &[
            (x - 13.0, y - 56.0 + b),
            (x + 13.0, y - 56.0 + b),
            (x + 9.0, y - 26.0 + b),
            (x - 9.0, y - 26.0 + b),
        ],
        tint(0x2a2230, &l),
        a,
    );
    if l.wounded {
        let p = 0.5 + 0.5 * (t * 6.0).sin();
        c.line(
            x - 6.0,
            y - 52.0 + b,
            x + 2.0,
            y - 38.0 + b,
            2.0,
            0xff3030,
            a * p,
        );
        c.line(
            x + 2.0,
            y - 38.0 + b,
            x - 3.0,
            y - 28.0 + b,
            2.0,
            0xff3030,
            a * p,
        );
        c.glow(x, y - 40.0 + b, 30.0, 0xff2020, 0.3 * a * p);
    }
    // Great blade
    c.line(
        x + f * 22.0,
        y - 10.0,
        x + f * 30.0,
        y - 74.0 + b,
        4.0,
        tint(0x40384a, &l),
        a,
    );
    c.line(
        x + f * 22.5,
        y - 12.0,
        x + f * 30.0,
        y - 72.0 + b,
        1.2,
        0xff4060,
        a * 0.7,
    );
    // Head
    c.circle(x, y - 64.0 + b, 10.0, tint(0x1a1220, &l), a);
    c.circle(x + f * 3.0 - 4.0, y - 64.0 + b, 2.0, 0xff2030, a);
    c.circle(x + f * 3.0 + 4.0, y - 64.0 + b, 2.0, 0xff2030, a);
    c.glow(x + f * 3.0, y - 64.0 + b, 16.0, 0xff1020, 0.6 * a);
    // Crown
    let cy = y - 72.0 + b;
    let mut pts = vec![(x - 11.0, cy + 2.0)];
    for i in 0..5 {
        let px = x - 10.0 + i as f32 * 5.0;
        pts.push((px - 2.0, cy - 2.0));
        pts.push((px, cy - 11.0 - if i == 2 { 5.0 } else { 0.0 }));
        pts.push((px + 2.0, cy - 2.0));
    }
    pts.push((x + 11.0, cy + 2.0));
    c.poly(&pts, tint(0x8a6a2a, &l), a);
    c.circle(x, cy - 3.0, 2.0, 0xff2040, a);
    c.glow(x, cy - 3.0, 8.0, 0xff2040, 0.6 * a);
}

// ------------------------------------------------------------------- props

pub fn rock(c: &mut Canvas, x: f32, y: f32, deco: u8, pal: &Palette, light: f32, biome: Biome) {
    let seed = deco as i32;
    let mut pts = vec![];
    let n = 9;
    for i in 0..n {
        let ang = std::f32::consts::PI + i as f32 / (n - 1) as f32 * std::f32::consts::PI;
        let r = 1.0 + ((hash2(seed, i) % 100) as f32 / 100.0 - 0.5) * 0.35;
        pts.push((x + ang.cos() * 15.0 * r, y - 6.0 + ang.sin() * 17.0 * r));
    }
    pts.push((x + 14.0, y + 2.0));
    pts.push((x - 14.0, y + 2.0));
    let base = scale(pal.rock, light);
    c.poly(&pts, base, 1.0);
    // Lit facet
    c.poly(
        &[
            (x - 11.0, y - 8.0),
            (x - 3.0, y - 20.0),
            (x + 4.0, y - 17.0),
            (x - 2.0, y - 6.0),
        ],
        scale(lerp_color(pal.rock, 0xffffff, 0.25), light),
        0.8,
    );
    c.line(
        x - 2.0,
        y - 6.0,
        x + 9.0,
        y - 2.0,
        1.2,
        scale(pal.rock, light * 0.6),
        0.8,
    );
    match biome {
        Biome::Frost => c.ellipse(x - 2.0, y - 18.0, 9.0, 3.0, scale(0xf0f6ff, light), 0.95),
        Biome::Moss => c.ellipse(
            x - 4.0,
            y - 17.0,
            7.0,
            2.5,
            scale(pal.foliage_hi, light),
            0.8,
        ),
        _ => {}
    }
}

pub fn tree(
    c: &mut Canvas,
    x: f32,
    y: f32,
    deco: u8,
    pal: &Palette,
    light: f32,
    biome: Biome,
    t: f32,
) {
    let sway = (t * 1.3 + deco as f32).sin() * 1.2;
    let s = 0.85 + (deco % 7) as f32 * 0.05;
    match biome {
        Biome::Moss => {
            c.fill_rect(
                x - 2.5,
                y - 20.0 * s,
                5.0,
                20.0 * s,
                scale(pal.trunk, light),
                1.0,
            );
            let fol = scale(pal.foliage, light);
            let hi = scale(pal.foliage_hi, light);
            c.circle(x - 8.0 + sway, y - 26.0 * s, 10.0 * s, fol, 1.0);
            c.circle(x + 8.0 + sway, y - 27.0 * s, 9.0 * s, fol, 1.0);
            c.circle(x + sway, y - 36.0 * s, 12.0 * s, fol, 1.0);
            c.circle(x - 4.0 + sway, y - 40.0 * s, 6.0 * s, hi, 0.7);
            c.circle(x - 10.0 + sway, y - 29.0 * s, 4.0 * s, hi, 0.5);
        }
        Biome::Ash => {
            let col = scale(pal.trunk, light.max(0.4));
            c.line(x, y, x + sway, y - 34.0 * s, 4.0, col, 1.0);
            c.line(
                x + sway * 0.5,
                y - 18.0 * s,
                x - 10.0,
                y - 28.0 * s,
                2.0,
                col,
                1.0,
            );
            c.line(
                x + sway * 0.7,
                y - 24.0 * s,
                x + 11.0,
                y - 34.0 * s,
                2.0,
                col,
                1.0,
            );
            c.line(x + sway, y - 30.0 * s, x - 5.0, y - 44.0 * s, 1.5, col, 1.0);
            let ember = 0.5 + 0.5 * (t * 2.0 + deco as f32).sin();
            c.glow(x + 11.0, y - 34.0 * s, 6.0, 0xff6020, 0.6 * ember);
        }
        Biome::Frost => {
            c.fill_rect(x - 2.0, y - 8.0, 4.0, 8.0, scale(pal.trunk, light), 1.0);
            for i in 0..3 {
                let k = i as f32;
                let w = (14.0 - k * 3.5) * s;
                let by = y - 6.0 - k * 11.0 * s;
                let ty = by - 16.0 * s;
                c.poly(
                    &[(x - w, by), (x + w, by), (x + sway * k * 0.4, ty)],
                    scale(pal.foliage, light),
                    1.0,
                );
                c.poly(
                    &[
                        (x - w * 0.45, ty + 7.0 * s),
                        (x + w * 0.45, ty + 7.0 * s),
                        (x + sway * k * 0.4, ty),
                    ],
                    scale(pal.foliage_hi, light),
                    0.95,
                );
            }
        }
        Biome::Crystal => {
            let col = scale(pal.foliage, light.max(0.6));
            let hi = scale(pal.foliage_hi, light.max(0.6));
            for (dx, h, w) in [(-7.0, 26.0, 4.5), (5.0, 36.0, 5.5), (0.0, 22.0, 4.0)] {
                let h = h * s;
                c.poly(
                    &[
                        (x + dx - w, y - 2.0),
                        (x + dx + w, y - 2.0),
                        (x + dx + w, y - h + 6.0),
                        (x + dx, y - h),
                        (x + dx - w, y - h + 6.0),
                    ],
                    col,
                    0.95,
                );
                c.line(x + dx, y - 2.0, x + dx, y - h, 1.0, hi, 0.8);
            }
            let p = 0.6 + 0.4 * (t * 1.5 + deco as f32).sin();
            c.glow(x, y - 20.0, 26.0, pal.foliage, 0.35 * p);
        }
        Biome::Hollow => {
            let col = scale(pal.foliage, 1.0);
            c.poly(
                &[
                    (x - 7.0, y),
                    (x + 7.0, y),
                    (x + 3.0, y - 22.0),
                    (x + 8.0, y - 30.0),
                    (x + 1.0, y - 28.0),
                    (x + sway, y - 46.0 * s),
                    (x - 2.0, y - 26.0),
                    (x - 9.0, y - 32.0),
                    (x - 4.0, y - 20.0),
                ],
                col,
                1.0,
            );
            c.glow(x + sway, y - 46.0 * s, 6.0, pal.foliage_hi, 0.7);
            c.glow(x + 8.0, y - 30.0, 4.0, pal.foliage_hi, 0.6);
        }
    }
}

pub fn brazier(c: &mut Canvas, x: f32, y: f32, lit: bool, light: f32, t: f32, deco: u8) {
    let stone = scale(0x6a6470, light.max(0.35));
    c.poly(
        &[
            (x - 9.0, y + 2.0),
            (x + 9.0, y + 2.0),
            (x + 5.0, y - 14.0),
            (x - 5.0, y - 14.0),
        ],
        stone,
        1.0,
    );
    c.fill_rect(
        x - 10.0,
        y - 2.0,
        20.0,
        3.0,
        scale(0x4a4450, light.max(0.35)),
        1.0,
    );
    c.ellipse(
        x,
        y - 17.0,
        12.0,
        4.5,
        scale(0x3a2e28, light.max(0.35)),
        1.0,
    );
    c.ellipse(x, y - 18.0, 10.0, 3.0, 0x1a1210, 1.0);
    if lit {
        let ph = t * 9.0 + deco as f32;
        let fl = 0.9 + 0.1 * ph.sin();
        c.glow(x, y - 26.0, 90.0 * fl, 0xff8a30, 0.22);
        c.glow(x, y - 24.0, 30.0, 0xffb050, 0.5);
        for (i, (col, w, h)) in [
            (0xff5a10, 9.0, 26.0),
            (0xffa030, 6.5, 19.0),
            (0xfff0a0, 3.5, 11.0),
        ]
        .iter()
        .enumerate()
        {
            let sway = (ph + i as f32 * 1.7).sin() * 2.0;
            let hh = h * (0.9 + 0.1 * (ph * 1.3 + i as f32).cos());
            c.poly(
                &[
                    (x - w, y - 19.0),
                    (x - w * 0.6, y - 19.0 - hh * 0.5),
                    (x + sway, y - 19.0 - hh),
                    (x + w * 0.6, y - 19.0 - hh * 0.5),
                    (x + w, y - 19.0),
                ],
                *col,
                0.95,
            );
        }
    } else {
        let p = 0.5 + 0.5 * (t * 1.3 + deco as f32).sin();
        c.glow(x, y - 18.0, 9.0, 0xa02010, 0.4 * p);
    }
}

pub fn pickup(c: &mut Canvas, x: f32, y: f32, k: Pickup, t: f32) {
    let bob = (t * 2.5 + x * 0.1).sin() * 3.0;
    let cy = y - 16.0 + bob;
    match k {
        Pickup::Ember => {
            c.glow(x, cy, 18.0, 0xff7020, 0.6);
            c.poly(
                &[(x, cy - 8.0), (x + 5.0, cy), (x, cy + 8.0), (x - 5.0, cy)],
                0xff8a30,
                1.0,
            );
            c.poly(
                &[(x, cy - 5.0), (x + 2.5, cy), (x, cy + 4.0), (x - 2.5, cy)],
                0xffe0a0,
                1.0,
            );
        }
        Pickup::Lumen => {
            let flap = (t * 14.0).sin().abs();
            c.glow(x, cy, 22.0, 0xfff0a0, 0.55);
            c.ellipse(
                x - 4.0,
                cy - 1.0,
                5.0 * (0.3 + flap * 0.7),
                4.0,
                0xfff6d0,
                0.9,
            );
            c.ellipse(
                x + 4.0,
                cy - 1.0,
                5.0 * (0.3 + flap * 0.7),
                4.0,
                0xfff6d0,
                0.9,
            );
            c.ellipse(x, cy, 1.5, 4.0, 0x8a7a50, 1.0);
        }
    }
    c.ellipse(x, y, 6.0, 2.5, 0x000000, 0.25);
}
