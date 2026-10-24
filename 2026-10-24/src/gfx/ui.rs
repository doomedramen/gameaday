//! HUD and panel helpers. Everything is drawn with the canvas primitives.

use super::canvas::{Canvas, H, W, lerp_color};
use super::sprites::draw_heart;
use crate::boss::BState;
use crate::game::Game;
use crate::run::{Boon, Medal};

pub const GOLD: u32 = 0xffd66b;
pub const TEXT: u32 = 0xf4f0ff;
pub const MUTED: u32 = 0xa8a0c8;
pub const PANEL: u32 = 0x120e28;
pub const EDGE: u32 = 0x6a5aa8;

#[derive(Clone, Copy, Debug)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl Rect {
    pub fn new(x: f32, y: f32, w: f32, h: f32) -> Rect {
        Rect { x, y, w, h }
    }

    pub fn contains(&self, p: (f32, f32)) -> bool {
        p.0 >= self.x && p.0 <= self.x + self.w && p.1 >= self.y && p.1 <= self.y + self.h
    }
}

pub fn panel(c: &mut Canvas, r: Rect, accent: u32, alpha: f32) {
    c.round_rect(r.x + 3.0, r.y + 5.0, r.w, r.h, 10.0, 0x000000, 0.3);
    c.round_rect(r.x, r.y, r.w, r.h, 10.0, PANEL, alpha);
    c.round_rect(r.x, r.y, r.w, r.h * 0.5, 10.0, 0xffffff, 0.04);
    c.round_rect_outline(r.x, r.y, r.w, r.h, 10.0, accent, 0.9);
}

pub fn button(c: &mut Canvas, r: Rect, label: &str, hover: bool, enabled: bool, accent: u32) {
    let base = if hover && enabled {
        lerp_color(PANEL, accent, 0.35)
    } else {
        PANEL
    };
    c.round_rect(r.x + 2.0, r.y + 4.0, r.w, r.h, 9.0, 0x000000, 0.3);
    c.round_rect(r.x, r.y, r.w, r.h, 9.0, base, 0.92);
    c.round_rect_outline(
        r.x,
        r.y,
        r.w,
        r.h,
        9.0,
        if enabled { accent } else { 0x4a4468 },
        if hover { 1.0 } else { 0.7 },
    );
    let col = if enabled {
        if hover { 0xffffff } else { TEXT }
    } else {
        0x7a7498
    };
    let s = if r.h >= 44.0 { 2.4 } else { 2.0 };
    c.text_center(
        r.x + r.w / 2.0,
        r.y + (r.h - 8.0 * s) / 2.0,
        label,
        s,
        col,
        1.0,
    );
}

pub fn mmss(t: f32) -> String {
    let t = t.max(0.0);
    let m = (t / 60.0) as u32;
    let s = t - m as f32 * 60.0;
    format!("{m}:{:04.1}", s)
}

pub fn medal_color(m: Medal) -> u32 {
    match m {
        Medal::None => 0x8a8aa0,
        Medal::Bronze => 0xd08850,
        Medal::Silver => 0xd8e0f0,
        Medal::Gold => 0xffd24a,
    }
}

pub fn draw_medal(c: &mut Canvas, x: f32, y: f32, m: Medal, s: f32) {
    let col = medal_color(m);
    c.line(
        x - 8.0 * s,
        y - 20.0 * s,
        x - 3.0 * s,
        y - 4.0 * s,
        5.0 * s,
        0xc04050,
        1.0,
    );
    c.line(
        x + 8.0 * s,
        y - 20.0 * s,
        x + 3.0 * s,
        y - 4.0 * s,
        5.0 * s,
        0x4060c0,
        1.0,
    );
    c.glow(
        x,
        y + 4.0 * s,
        40.0 * s,
        col,
        if m == Medal::None { 0.0 } else { 0.5 },
    );
    c.circle(x, y + 4.0 * s, 15.0 * s, scale_col(col, 0.7), 1.0);
    c.circle(x, y + 4.0 * s, 12.5 * s, col, 1.0);
    c.circle(x - 3.0 * s, y + 1.0 * s, 4.0 * s, 0xffffff, 0.35);
    if m != Medal::None {
        let mut pts = Vec::with_capacity(10);
        for k in 0..10 {
            let a = -std::f32::consts::FRAC_PI_2 + k as f32 * std::f32::consts::PI / 5.0;
            let r = if k % 2 == 0 { 9.0 } else { 3.8 } * s;
            pts.push((x + a.cos() * r, y + 4.0 * s + a.sin() * r));
        }
        c.poly(&pts, scale_col(col, 0.55), 1.0);
    }
}

fn scale_col(c: u32, k: f32) -> u32 {
    super::canvas::scale(c, k)
}

pub fn boon_icon(c: &mut Canvas, x: f32, y: f32, b: Boon, r: f32) {
    let col = b.color();
    c.glow(x, y, r * 2.2, col, 0.35);
    c.circle(x, y, r, 0x1a1530, 1.0);
    c.ring(x, y, r - 1.0, r - 1.0, 2.0, col, 1.0);
    let label = match b {
        Boon::LongLine => "LL",
        Boon::QuickReel => "QR",
        Boon::Locket => "+",
        Boon::SecondWind => "2J",
        Boon::Momentum => "MM",
        Boon::EchoWell => "EW",
        Boon::Magnet => "MG",
        Boon::Gilded => "$",
        Boon::SpringHeel => "SH",
        Boon::WallGrip => "WG",
        Boon::SwingBoost => "SB",
        Boon::StormWard => "SW",
        Boon::Glowcap => "GL",
        Boon::Rhythm => "RB",
        Boon::Phoenix => "PH",
    };
    let s = if r > 20.0 { 2.4 } else { 1.0 };
    c.text_center(x, y - 4.0 * s, label, s, col, 1.0);
}

#[allow(clippy::too_many_arguments)]
pub fn draw_hud(
    c: &mut Canvas,
    g: &Game,
    stage_label: &str,
    biome_name: &str,
    boons: &[Boon],
    score: i64,
    show_hint: f32,
    time: f32,
) {
    let w = &g.world;
    let p = &w.p;
    // hearts
    for i in 0..p.max_hp {
        let x = 34.0 + i as f32 * 30.0;
        let full = i < p.hp;
        if full {
            c.glow(x, 34.0, 20.0, 0xff4a60, 0.25);
            draw_heart(c, x, 32.0, 1.35, 0xff4a60, 1.0);
            c.circle(x - 4.0, 28.0, 2.0, 0xffffff, 0.6);
        } else {
            draw_heart(c, x, 32.0, 1.35, 0x3a2a44, 0.9);
        }
    }
    if p.shield {
        let x = 34.0 + p.max_hp as f32 * 30.0;
        c.ring(x, 33.0, 9.0, 9.0, 2.5, 0x9ce8ff, 1.0);
        c.circle(x, 33.0, 3.0, 0x9ce8ff, 0.8);
    }
    // echo meter
    let bx = 22.0;
    let by = 58.0;
    let bw = 170.0 * (g.echo_max / 100.0).min(1.6);
    c.round_rect(bx, by, bw, 12.0, 6.0, 0x08061a, 0.8);
    let k = (g.echo / g.echo_max).clamp(0.0, 1.0);
    let ecol = if g.rewinding {
        0xffffff
    } else {
        lerp_color(0x6a7cff, 0xb8c8ff, k)
    };
    if k > 0.01 {
        c.round_rect(bx + 1.5, by + 1.5, (bw - 3.0) * k, 9.0, 4.5, ecol, 1.0);
        c.round_rect(bx + 1.5, by + 1.5, (bw - 3.0) * k, 4.0, 3.0, 0xffffff, 0.3);
    }
    c.round_rect_outline(bx, by, bw, 12.0, 6.0, 0x8aa8ff, 0.7);
    c.text_shadow(bx, by + 17.0, "ECHO [R] REWIND", 1.0, 0xb8c8ff, 0.85);
    if g.rewinding {
        c.text_center(640.0, 130.0, "<< REWINDING", 3.0, 0xc8d8ff, 0.9);
    }

    // stage banner and progress
    let cx = W as f32 / 2.0;
    let bw2 = 380.0;
    let x0 = cx - bw2 / 2.0;
    c.text_center(
        cx,
        14.0,
        &format!("{stage_label} - {biome_name}"),
        2.0,
        TEXT,
        0.95,
    );
    c.round_rect(x0, 40.0, bw2, 8.0, 4.0, 0x08061a, 0.75);
    let prog = w.progress();
    c.round_rect(
        x0 + 1.0,
        41.0,
        (bw2 - 2.0) * prog.max(0.01),
        6.0,
        3.0,
        GOLD,
        0.9,
    );
    let (sx, _) = w.level.spawn;
    let (ex, _) = w.level.exit;
    for &(lx, _) in &w.level.checkpoints {
        let t = ((lx - sx) / (ex - sx)).clamp(0.0, 1.0);
        c.fill_rect(x0 + bw2 * t - 1.0, 38.0, 2.0, 12.0, 0xffffff, 0.5);
    }
    c.circle(x0 + bw2 * prog, 44.0, 6.0, 0xffffff, 1.0);
    c.circle(x0 + bw2 * prog, 44.0, 3.5, GOLD, 1.0);
    c.circle(x0 + bw2, 44.0, 6.0, 0x5a4a98, 1.0);
    // timer vs par
    let par = w.level.par;
    let t = w.time;
    let medal = crate::run::medal_for(t, par);
    let tcol = if t <= par {
        0xffd24a
    } else if t <= par * 1.45 {
        0xd8e0f0
    } else if t <= par * 2.1 {
        0xd08850
    } else {
        MUTED
    };
    c.text_center(
        cx,
        56.0,
        &format!("{}   PAR {}", mmss(t), mmss(par)),
        1.6,
        tcol,
        0.95,
    );
    let _ = medal;

    // boss bar
    if let Some(b) = &w.boss {
        let y = 92.0;
        c.text_center(cx, y - 6.0, "THE STORMHEART", 2.0, 0xe0d0ff, 1.0);
        for i in 0..b.max_hp {
            let x = cx + (i as f32 - (b.max_hp as f32 - 1.0) / 2.0) * 34.0;
            let alive = i < b.hp;
            c.poly(
                &[
                    (x, y + 14.0),
                    (x + 10.0, y + 26.0),
                    (x, y + 40.0),
                    (x - 10.0, y + 26.0),
                ],
                if alive { 0xb8a0ff } else { 0x2a2444 },
                1.0,
            );
            if alive {
                c.glow(x, y + 27.0, 22.0, 0xb8a0ff, 0.5);
            }
        }
        if b.state == BState::Stun {
            c.text_center(
                cx,
                y + 48.0,
                if p.rope.is_some() {
                    "NOW SWING THROUGH IT!"
                } else {
                    "HOOK THE GLOWING ANCHOR ABOVE IT AND SWING IN FAST!"
                },
                2.0,
                0xffe066,
                0.5 + 0.5 * (time * 8.0).sin().abs(),
            );
        }
    }

    // score and crystals
    c.text_shadow(
        W as f32 - 24.0 - 7.0 * 2.4 * format!("{score}").len() as f32,
        18.0,
        &format!("{score}"),
        2.4,
        GOLD,
        1.0,
    );
    c.text_shadow(
        W as f32 - 132.0,
        46.0,
        &format!("{}", w.stats.crystals),
        2.0,
        0x9df4ff,
        1.0,
    );
    c.poly(
        &[
            (W as f32 - 148.0, 44.0),
            (W as f32 - 143.0, 52.0),
            (W as f32 - 148.0, 62.0),
            (W as f32 - 153.0, 52.0),
        ],
        0x5fe8ff,
        1.0,
    );
    if w.stats.chain >= 2 {
        let mult = w.stats.mult();
        let kk = (w.stats.chain_t / w.mods.chain_time).clamp(0.0, 1.0);
        let col = if mult >= 4 { 0xff9a52 } else { 0xffe066 };
        c.text_shadow(
            W as f32 - 150.0,
            72.0,
            &format!("COMBO x{mult}"),
            2.0,
            col,
            1.0,
        );
        c.round_rect(W as f32 - 150.0, 92.0, 120.0, 6.0, 3.0, 0x08061a, 0.8);
        c.round_rect(W as f32 - 150.0, 92.0, 120.0 * kk, 6.0, 3.0, col, 1.0);
    }

    // boon row
    for (i, b) in boons.iter().enumerate() {
        boon_icon(c, 34.0 + i as f32 * 30.0, H as f32 - 30.0, *b, 12.0);
    }
    // speed indicator when going fast
    if p.charged {
        c.text_center(
            cx,
            H as f32 - 52.0,
            "~ MOMENTUM ~",
            1.6,
            0xaee8ff,
            0.7 + 0.3 * (time * 10.0).sin(),
        );
    }
    // controls hint
    if show_hint > 0.0 {
        let a = show_hint.min(1.0);
        c.text_center(
            cx,
            H as f32 - 28.0,
            "A/D move   SPACE jump   hold CLICK (or SHIFT) hook   R rewind   ESC pause",
            1.4,
            TEXT,
            0.8 * a,
        );
    }
}

/// Bottom-centre tutorial / hint box.
pub fn hint_box(c: &mut Canvas, text: &str, alpha: f32) {
    let w = 760.0;
    let r = Rect::new(W as f32 / 2.0 - w / 2.0, H as f32 - 150.0, w, 66.0);
    panel(c, r, GOLD, 0.9 * alpha);
    let _ = c.text_wrap(r.x + 22.0, r.y + 16.0, r.w - 44.0, text, 1.8, TEXT, alpha);
}
