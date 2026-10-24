//! Composes a frame of the world: backdrop, tiles, hazards, foes, Wren,
//! effects and the dark-cave light map.

use super::bg;
use super::canvas::{Canvas, H, W, lerp_color, pack, scale, split};
use super::fx::Fx;
use super::palette::{Pal, pal};
use super::sprites::*;
use super::tiles::{Atlas, CLEAR, TS};
use crate::boss::BState;
use crate::game::Game;
use crate::level::*;
use crate::sim::*;

pub struct Camera {
    pub x: f32,
    pub y: f32,
    init: bool,
}

impl Default for Camera {
    fn default() -> Camera {
        Camera::new()
    }
}

impl Camera {
    pub fn new() -> Camera {
        Camera {
            x: 0.0,
            y: 0.0,
            init: false,
        }
    }

    pub fn snap(&mut self, w: &World) {
        self.init = false;
        self.follow(w, 1.0);
    }

    pub fn follow(&mut self, w: &World, dt: f32) {
        let p = &w.p;
        let mut tx = p.x + (p.vx * 0.28).clamp(-150.0, 150.0) - W as f32 / 2.0;
        let mut ty = p.y + (p.vy * 0.1).clamp(-50.0, 110.0) - H as f32 * 0.56;
        if let Some(b) = &w.boss {
            // keep the Stormheart and Wren in the same frame
            tx = p.x * 0.6 + b.x * 0.4 - W as f32 / 2.0;
            ty = p.y * 0.55 + b.y * 0.45 - H as f32 * 0.5 + 70.0;
        }
        if let Some(r) = &p.rope {
            // keep the anchor in frame
            tx += (r.ax - p.x) * 0.25;
            ty += ((r.ay - p.y) * 0.4).min(0.0);
        }
        let max_x = (w.level.pixel_w() - W as f32).max(0.0);
        let max_y = (w.level.pixel_h() - H as f32).max(0.0);
        tx = tx.clamp(0.0, max_x);
        ty = ty.clamp(0.0, max_y);
        if !self.init {
            self.x = tx;
            self.y = ty;
            self.init = true;
        } else {
            let k = 1.0 - (-dt * 5.5).exp();
            self.x += (tx - self.x) * k;
            self.y += (ty - self.y) * (1.0 - (-dt * 4.5).exp());
        }
    }
}

pub struct SceneCache {
    pub biome: Biome,
    pub pal: Pal,
    atlas: Atlas,
    light: Vec<f32>,
}

impl SceneCache {
    pub fn new(biome: Biome) -> SceneCache {
        let p = pal(biome);
        // draw one backdrop frame now so cached textures are ready before play
        let mut warm = Canvas::new(W, H);
        bg::draw_background(&mut warm, biome, &p, 0.0, 0.0, 0.0);
        SceneCache {
            biome,
            pal: p,
            atlas: Atlas::new(biome, &p),
            light: vec![0.0; (W / 2) * (H / 2)],
        }
    }
}

fn solid_for_mask(l: &Level, x: i32, y: i32) -> bool {
    matches!(l.tile(x, y), ROCK | ICE)
}

fn blit_tile(c: &mut Canvas, tile: &[u32], sx: i32, sy: i32) {
    for py in 0..TS as i32 {
        let y = sy + py;
        if y < 0 || y >= H as i32 {
            continue;
        }
        for px in 0..TS as i32 {
            let x = sx + px;
            if x < 0 || x >= W as i32 {
                continue;
            }
            let v = tile[py as usize * TS + px as usize];
            if v != CLEAR {
                c.px[y as usize * W + x as usize] = v;
            }
        }
    }
}

/// Everything the HUD/app need to know about what was drawn.
pub struct SceneInfo {
    pub targeted_anchor: Option<usize>,
}

#[allow(clippy::too_many_arguments)]
pub fn draw_scene(
    c: &mut Canvas,
    cache: &mut SceneCache,
    g: &Game,
    cam: &Camera,
    fx: &Fx,
    time: f32,
    aim: Option<(f32, f32)>,
    landed_squash: f32,
) -> SceneInfo {
    let w = &g.world;
    let l = &*w.level;
    let pal = cache.pal;
    // screen shake offsets
    let sh = fx.shake;
    let (shx, shy) = if sh > 0.05 {
        (
            (time * 97.0).sin() * sh * 0.8 + (time * 53.0).cos() * sh * 0.4,
            (time * 83.0).cos() * sh * 0.8,
        )
    } else {
        (0.0, 0.0)
    };
    let camx = (cam.x + shx).round();
    let camy = (cam.y + shy).round();
    bg::draw_background(c, l.biome, &pal, camx, camy, time);

    // visible tile range
    let tx0 = (camx / T).floor() as i32 - 1;
    let tx1 = ((camx + W as f32) / T).ceil() as i32 + 1;
    let ty0 = (camy / T).floor() as i32 - 1;
    let ty1 = ((camy + H as f32) / T).ceil() as i32 + 1;

    // pit glow / lava below the level
    if matches!(l.biome, Biome::Cinderworks) {
        let top = l.pixel_h() - 70.0 - camy;
        if top < H as f32 {
            let t = time;
            for x in (0..W as i32).step_by(4) {
                let wave = ((x as f32 + camx) * 0.02 + t * 1.5).sin() * 5.0;
                c.fill_rect(x as f32, top + wave, 4.0, H as f32 - top, 0xff5a10, 1.0);
                c.fill_rect(x as f32, top + wave, 4.0, 5.0, 0xffd060, 1.0);
            }
            c.glow(640.0, top, 700.0, 0xff6a20, 0.4);
        }
    }

    // gate and lamps sit behind the action
    let (ex, ey) = l.exit;
    draw_gate(c, ex - camx, ey - camy, !l.boss || w.gate_open, time, &pal);
    for (i, &(lx, ly)) in l.checkpoints.iter().enumerate() {
        draw_lamp(c, lx - camx, ly - camy, w.cp_next > i, time);
    }

    // wind streaks
    for e in &w.ents {
        if e.kind != EntKind::Wind || !e.alive || e.hw > 900.0 {
            continue;
        }
        if e.x + e.hw < camx || e.x - e.hw > camx + W as f32 {
            continue;
        }
        let f = wind_factor(time, e.c);
        let dirx = if e.a != 0.0 {
            e.a.signum() * f.signum()
        } else {
            0.0
        };
        let diry = if e.b != 0.0 {
            e.b.signum() * f.signum()
        } else {
            0.0
        };
        let n = ((e.hw * e.hh) / 7000.0).clamp(6.0, 40.0) as i32;
        for k in 0..n {
            let h1 = (super::canvas::hash2(k, e.x as i32) & 0xffff) as f32 / 65535.0;
            let h2 = (super::canvas::hash2(k, e.y as i32 + 9) & 0xffff) as f32 / 65535.0;
            let sp = 0.5 + h1;
            let mut px = e.x - e.hw + h1 * e.hw * 2.0;
            let mut py = e.y - e.hh + h2 * e.hh * 2.0;
            let tt = (time * sp * 0.6 + h2) % 1.0;
            px += dirx * (tt - 0.5) * 140.0;
            py += diry * (tt - 0.5) * 220.0;
            let a = (1.0 - (tt - 0.5).abs() * 2.0) * 0.4 * f.abs();
            let streak = if matches!(l.biome, Biome::Frostspire | Biome::Mossbluff) {
                0x6a8ab8
            } else {
                0xffffff
            };
            c.line(
                px - camx - dirx * 22.0,
                py - camy - diry * 28.0,
                px - camx,
                py - camy,
                1.8,
                streak,
                a * 1.4,
            );
        }
    }

    // tiles
    for ty in ty0..ty1 {
        for tx in tx0..tx1 {
            let t = l.tile(tx, ty);
            if t != ROCK && t != ICE {
                continue;
            }
            let up = solid_for_mask(l, tx, ty - 1);
            let right = solid_for_mask(l, tx + 1, ty);
            let down = solid_for_mask(l, tx, ty + 1);
            let left = solid_for_mask(l, tx - 1, ty);
            let mask =
                up as usize | (right as usize) << 1 | (down as usize) << 2 | (left as usize) << 3;
            let mut depth = 0;
            if up {
                depth = 1;
                if solid_for_mask(l, tx, ty - 2) {
                    depth = 2;
                    if solid_for_mask(l, tx, ty - 3) {
                        depth = 3;
                    }
                }
            }
            let var = (super::canvas::hash2(tx, ty) % 3) as usize;
            let tile = cache.atlas.tile(depth, var, mask);
            blit_tile(
                c,
                tile,
                (tx as f32 * T - camx) as i32,
                (ty as f32 * T - camy) as i32,
            );
        }
    }
    // special tiles
    let near_x = w.p.x;
    let near_y = w.p.y;
    for ty in ty0..ty1 {
        for tx in tx0..tx1 {
            let t = l.tile(tx, ty);
            match t {
                GLASS => {
                    if l.block_at(tx, ty).is_some_and(|b| w.blocks[b].solid) {
                        let near = (near_x - tx as f32 * T).abs() < 200.0
                            && (near_y - ty as f32 * T).abs() < 200.0;
                        draw_glass(c, tx, ty, camx, camy, time, near);
                    }
                }
                CRUMBLE => {
                    if let Some(b) = l.block_at(tx, ty)
                        && w.blocks[b].solid
                    {
                        draw_crumble(c, tx, ty, camx, camy, w.blocks[b].timer, &pal);
                    }
                }
                x if is_spike(x) => draw_spikes(c, tx, ty, x, camx, camy, &pal),
                _ => {}
            }
        }
    }
    // grass tufts / icicles / drips on exposed surfaces
    decorate_edges(c, l, tx0, tx1, ty0, ty1, camx, camy, time, &pal);

    // anchors and the rope
    let player_range = w.mods.hook_range;
    let targeted = {
        let ctl = Controls {
            aim,
            ..Controls::default()
        };
        if w.p.rope.is_none() {
            w.find_anchor(&ctl)
        } else {
            None
        }
    };
    if let Some(t) = targeted {
        // a faint dotted line from Wren to the anchor the hook would catch
        let a = w.anchors[t];
        let (x0, y0) = (w.p.x - camx, w.p.y - camy - 4.0);
        let (x1, y1) = (a.x - camx, a.y - camy);
        let d = dist(x0, y0, x1, y1);
        let n = (d / 14.0) as i32;
        for k in 1..n {
            let f = k as f32 / n as f32;
            c.circle(
                x0 + (x1 - x0) * f,
                y0 + (y1 - y0) * f,
                1.4,
                0xffffff,
                0.35 * (1.0 - f * 0.5),
            );
        }
    }
    // rails first
    for (i, a) in l.anchors.iter().enumerate() {
        if a.kind == AnchorKind::Rail {
            c.line(
                a.x - camx,
                a.y - camy,
                a.x2 - camx,
                a.y2 - camy,
                2.0,
                0x4a4468,
                0.8,
            );
            let _ = i;
        }
    }
    for (i, st) in w.anchors.iter().enumerate() {
        let a = &l.anchors[i];
        if st.x < camx - 60.0
            || st.x > camx + W as f32 + 60.0
            || st.y < camy - 60.0
            || st.y > camy + H as f32 + 60.0
        {
            continue;
        }
        let d = dist(w.p.x, w.p.y, st.x, st.y);
        let near = d <= player_range;
        // while the Stormheart is down, light the anchor above it
        let boss_hint = w.boss.as_ref().is_some_and(|b| {
            b.vulnerable()
                && crate::boss::SLAM_SPOTS
                    .iter()
                    .position(|s| (s * T - b.spot).abs() < 2.0)
                    == Some(i)
        });
        draw_anchor(
            c,
            st.x - camx,
            st.y - camy,
            a.kind,
            st.alive,
            st.stress,
            near || boss_hint,
            targeted == Some(i) || boss_hint,
            time,
            &pal,
        );
    }

    // entities
    for e in &w.ents {
        if !e.alive {
            continue;
        }
        let x = e.x - camx;
        let y = e.y - camy;
        if x < -200.0 || x > W as f32 + 200.0 || y < -300.0 || y > H as f32 + 300.0 {
            // saws and strikes have long rails / columns
            if !matches!(e.kind, EntKind::Saw | EntKind::Strike) {
                continue;
            }
        }
        match e.kind {
            EntKind::Crystal => draw_crystal(c, x, y, time, e.home_x * 0.013),
            EntKind::EchoOrb | EntKind::HeartShard | EntKind::Chest => {
                draw_pickup(c, e.kind, x, y, time)
            }
            EntKind::Skitter => draw_skitter(c, e, x, y, time),
            EntKind::Bulwark => draw_bulwark(c, e, x, y, time),
            EntKind::Spitter => {
                draw_spitter_aim(c, e, x, y);
                draw_spitter(c, e, x, y, time);
            }
            EntKind::Saw => draw_saw(
                c,
                e,
                x,
                y,
                e.home_x - camx,
                e.home_y - camy,
                e.a - camx,
                e.b - camy,
                time,
            ),
            EntKind::Geyser => draw_geyser(c, e, x, y + 6.0, time),
            EntKind::Wisp => draw_wisp(c, e, x, y, time),
            EntKind::Bolt => draw_bolt(c, e, x, y, time),
            EntKind::Bloom => draw_bloom(c, e, x, y + 8.0, time),
            EntKind::Strike => {
                if e.state == 1 && e.t >= 0.0 {
                    let sx = e.a - camx;
                    let k = (e.t / 1.05).clamp(0.0, 1.0);
                    let blink = if (time * (8.0 + 18.0 * k)).sin() > 0.0 {
                        1.0
                    } else {
                        0.5
                    };
                    c.line(
                        sx,
                        0.0,
                        sx,
                        H as f32,
                        1.0 + 2.0 * k,
                        0xe8e0ff,
                        0.12 + 0.3 * k * blink,
                    );
                    let gy = e.c - camy + 30.0;
                    c.ring(sx, gy, 24.0 - 8.0 * k, 7.0, 2.5, 0xe8e0ff, 0.4 + 0.5 * k);
                } else if e.state == 2 {
                    let sx = e.a - camx;
                    c.glow(sx, e.c - camy, 220.0, 0xc8b8ff, 0.6);
                    c.fill_rect(sx - 22.0, 0.0, 44.0, H as f32, 0xc8b8ff, 0.5);
                    c.fill_rect(sx - 9.0, 0.0, 18.0, H as f32, 0xffffff, 0.95);
                    // jagged bolt
                    let mut px = sx;
                    let mut py = 0.0;
                    for k in 0..16 {
                        let nx = sx
                            + ((super::canvas::hash2(k, (time * 40.0) as i32) & 31) as f32 - 15.0);
                        let ny = py + H as f32 / 16.0;
                        c.line(px, py, nx, ny, 3.0, 0xffffff, 1.0);
                        px = nx;
                        py = ny;
                    }
                }
            }
            EntKind::Wind => {}
        }
    }

    // the Stormheart
    if let Some(b) = &w.boss {
        let bx = b.x - camx;
        let by = b.y - camy;
        draw_boss_telegraph(c, b, bx, by, b.floor - camy, camx, time);
        if b.state != BState::Dead || b.t < 2.2 {
            draw_boss(c, b, bx, by, time, &pal);
        }
    }

    // rope and hook
    let p = &w.p;
    let pose_anchor = p.rope.map(|r| (r.ax - camx, r.ay - camy));
    if let Some(r) = &p.rope {
        let (hx, hy) = (p.x - camx + p.facing as f32 * 3.0, p.y - camy - 4.0);
        let (ax, ay) = (r.ax - camx, r.ay - camy);
        let d = dist(hx, hy, ax, ay);
        let sag = ((r.len - d) * 0.6).clamp(0.0, 30.0) + 1.0 * (time * 9.0).sin().abs();
        let steps = 14;
        let mut prev = (hx, hy);
        for k in 1..=steps {
            let t = k as f32 / steps as f32;
            let px = hx + (ax - hx) * t;
            let py = hy + (ay - hy) * t + sag * 4.0 * t * (1.0 - t);
            c.line(prev.0, prev.1, px, py, 3.2, 0x3a2a1a, 1.0);
            c.line(prev.0, prev.1, px, py, 1.6, 0xe8d0a0, 1.0);
            prev = (px, py);
        }
        // strain glow when fast
        if p.speed > 440.0 {
            c.line(hx, hy, ax, ay, 5.0, 0xaee8ff, 0.18);
        }
    }
    if let Some((tx, ty, t)) = w.hook_miss {
        let k = (t / 0.2).clamp(0.0, 1.0);
        let (hx, hy) = (p.x - camx, p.y - camy - 4.0);
        let ex = hx + (tx - camx - hx) * (1.0 - k * 0.0);
        let ey = hy + (ty - camy - hy) * (1.0 - k * 0.0);
        c.line(hx, hy, ex, ey, 2.0, 0xe8d0a0, 0.9 * k);
        c.circle(ex, ey, 3.0, 0xffffff, 0.8 * k);
    }

    // afterimage trail when moving fast
    for (i, tr) in fx.trail.iter().enumerate() {
        let a = (tr.2 / 0.22).clamp(0.0, 1.0) * 0.35;
        let _ = i;
        c.ellipse(tr.0 - camx, tr.1 - camy, 9.0, 14.0, 0xaee8ff, a);
    }

    // Wren
    let pose = WrenPose {
        x: p.x - camx,
        y: p.y - camy,
        facing: p.facing as f32,
        vx: p.vx,
        vy: p.vy,
        on_ground: p.on_ground,
        wall: p.wall,
        run_t: p.run_t,
        anchor: pose_anchor,
        invuln: p.invuln,
        hurt_t: p.hurt_t,
        charged: p.charged,
        dying: p.dying,
        time,
        shield: p.shield,
        landed_squash,
    };
    let mut scarf = fx.scarf;
    for s in &mut scarf {
        s.0 -= camx;
        s.1 -= camy;
    }
    if p.dying <= 0.0 || (time * 12.0) as i32 % 2 == 0 {
        draw_wren(c, &pose, &scarf);
    }

    // world-space particles
    {
        let (ox, oy) = (c.ox, c.oy);
        c.ox = -camx;
        c.oy = -camy;
        // particles carry world coordinates; offset them for drawing
        for part in &fx.parts {
            let mut pp = *part;
            pp.x -= 0.0;
            pp.y -= 0.0;
            super::fx::draw_particle(c, &pp, true);
        }
        c.ox = ox;
        c.oy = oy;
    }

    // light map for dark biomes
    if pal.ambient < 0.95 {
        light_pass(c, cache, g, camx, camy, time);
    }
    // screen-space weather above everything
    fx.draw_weather(c);
    // popups (world coords)
    for pu in &fx.popups {
        let a = (pu.life * 1.6).min(1.0);
        c.text_center(
            pu.x - camx,
            pu.y - camy,
            &pu.text,
            2.0 * pu.scale,
            pu.col,
            a,
        );
    }
    // speed lines when charged with momentum
    if w.p.speed > 440.0 && w.p.dying <= 0.0 {
        let k = ((w.p.speed - 440.0) / 400.0).clamp(0.0, 1.0);
        let (dx, dy) = (w.p.vx / w.p.speed, w.p.vy / w.p.speed);
        let n = (10.0 + 14.0 * k) as i32;
        for i in 0..n {
            let h = super::canvas::hash2(i, (time * 30.0) as i32);
            let ang = (h & 0xff) as f32 / 255.0 * std::f32::consts::TAU;
            let rad = 240.0 + ((h >> 8) & 0xff) as f32 * 1.6;
            let sx = W as f32 / 2.0 + ang.cos() * rad * 1.4;
            let sy = H as f32 / 2.0 + ang.sin() * rad * 0.8;
            let len = 50.0 + 90.0 * k;
            c.line(
                sx,
                sy,
                sx - dx * len,
                sy - dy * len,
                1.6,
                0xffffff,
                0.22 * k.max(0.3),
            );
        }
    }
    // vignette
    vignette(c, if l.dark() { 0.55 } else { 0.38 });
    SceneInfo {
        targeted_anchor: targeted,
    }
}

trait LevelDark {
    fn dark(&self) -> bool;
}
impl LevelDark for Level {
    fn dark(&self) -> bool {
        self.biome.dark()
    }
}

fn vignette(c: &mut Canvas, strength: f32) {
    // cheap: darken edges with a few translucent bands
    let bands = 10;
    for i in 0..bands {
        let t = i as f32 / bands as f32;
        let a = (1.0 - t).powi(2) * strength * 0.45;
        let x = i as f32 * 7.0;
        c.fill_rect(x, 0.0, 7.0, H as f32, 0x000010, a);
        c.fill_rect(W as f32 - x - 7.0, 0.0, 7.0, H as f32, 0x000010, a);
        let y = i as f32 * 5.0;
        c.fill_rect(0.0, y, W as f32, 5.0, 0x000010, a * 0.7);
        c.fill_rect(0.0, H as f32 - y - 5.0, W as f32, 5.0, 0x000010, a * 0.7);
    }
}

fn decorate_edges(
    c: &mut Canvas,
    l: &Level,
    tx0: i32,
    tx1: i32,
    ty0: i32,
    ty1: i32,
    camx: f32,
    camy: f32,
    time: f32,
    pal: &Pal,
) {
    for ty in ty0..ty1 {
        for tx in tx0..tx1 {
            let t = l.tile(tx, ty);
            if t != ROCK && t != ICE {
                continue;
            }
            let h = super::canvas::hash2(tx, ty);
            let x = tx as f32 * T - camx;
            let y = ty as f32 * T - camy;
            // blades / tufts on exposed tops
            if !solid_for_mask(l, tx, ty - 1) && !is_spike(l.tile(tx, ty - 1)) {
                match l.biome {
                    Biome::Mossbluff => {
                        for k in 0..3 {
                            let bx = x + 5.0 + k as f32 * 10.0 + (h >> (k * 3) & 3) as f32;
                            let sway = (time * 2.0 + bx * 0.04 + k as f32).sin() * 2.2;
                            let bh = 6.0 + (h >> (k * 4) & 7) as f32;
                            c.line(
                                bx,
                                y + 3.0,
                                bx + sway,
                                y - bh,
                                2.0,
                                lerp_color(pal.surface_dark, pal.surface, 0.5),
                                1.0,
                            );
                            c.line(bx, y + 3.0, bx + sway, y - bh, 0.8, pal.surface, 1.0);
                        }
                        if h % 9 == 0 {
                            let fx = x + 8.0 + (h >> 6 & 15) as f32;
                            c.line(fx, y + 2.0, fx, y - 10.0, 1.4, 0x3a9a3a, 1.0);
                            c.circle(
                                fx,
                                y - 11.0,
                                3.0,
                                [0xff7aa8, 0xffe066, 0xffffff][(h >> 9) as usize % 3],
                                1.0,
                            );
                        }
                    }
                    Biome::Glimmerdeep => {
                        if h % 5 == 0 {
                            let cx = x + 6.0 + (h >> 5 & 15) as f32;
                            let hh = 8.0 + (h >> 9 & 7) as f32;
                            c.poly(
                                &[(cx - 3.0, y + 2.0), (cx, y - hh), (cx + 3.0, y + 2.0)],
                                0x62f2ff,
                                0.9,
                            );
                            c.poly(
                                &[(cx - 1.0, y + 2.0), (cx, y - hh), (cx + 3.0, y + 2.0)],
                                0xc8ffff,
                                0.8,
                            );
                            c.glow(cx, y - hh * 0.5, 22.0, 0x62f2ff, 0.4);
                        }
                    }
                    Biome::Frostspire => {
                        if h % 4 == 0 {
                            let cx = x + 6.0 + (h >> 5 & 15) as f32;
                            c.circle(cx, y, 5.0, 0xffffff, 0.9);
                            c.circle(cx + 6.0, y + 1.0, 4.0, 0xeaf6ff, 0.9);
                        }
                    }
                    Biome::Cinderworks => {
                        if h % 6 == 0 {
                            let cx = x + 10.0;
                            let fl = (time * 8.0 + h as f32).sin() * 2.0;
                            c.poly(
                                &[(cx - 4.0, y + 4.0), (cx + fl, y - 8.0), (cx + 4.0, y + 4.0)],
                                0xff7a30,
                                0.8,
                            );
                            c.glow(cx, y - 2.0, 18.0, 0xff7a30, 0.4);
                        }
                    }
                    Biome::Stormcrown | Biome::Beacon => {
                        if h % 7 == 0 {
                            let cx = x + 8.0 + (h >> 5 & 15) as f32;
                            c.line(cx, y + 2.0, cx + 1.0, y - 7.0, 1.6, pal.surface_dark, 1.0);
                            c.circle(cx + 1.0, y - 8.0, 2.4, pal.glow, 0.8);
                        }
                    }
                }
            }
            // icicles / stalactites / vines under overhangs
            if !solid_for_mask(l, tx, ty + 1)
                && !matches!(l.tile(tx, ty + 1), GLASS | CRUMBLE)
                && h % 3 == 0
            {
                let bx = x + 6.0 + (h >> 4 & 15) as f32;
                let len = 6.0 + (h >> 8 & 7) as f32 * 1.6;
                match l.biome {
                    Biome::Frostspire => {
                        c.poly(
                            &[
                                (bx - 3.0, y + 28.0),
                                (bx, y + 28.0 + len),
                                (bx + 3.0, y + 28.0),
                            ],
                            0xd8f0ff,
                            0.95,
                        );
                        c.line(bx - 1.0, y + 28.0, bx, y + 26.0 + len, 1.0, 0xffffff, 0.9);
                    }
                    Biome::Mossbluff => {
                        let sway = (time * 1.5 + bx * 0.05).sin() * 2.0;
                        c.line(
                            bx,
                            y + 28.0,
                            bx + sway,
                            y + 28.0 + len * 1.4,
                            2.0,
                            0x3a8a44,
                            1.0,
                        );
                        c.circle(bx + sway, y + 28.0 + len * 1.4, 1.8, 0x7bd45a, 1.0);
                    }
                    Biome::Glimmerdeep | Biome::Cinderworks => {
                        c.poly(
                            &[
                                (bx - 3.0, y + 28.0),
                                (bx, y + 28.0 + len),
                                (bx + 3.0, y + 28.0),
                            ],
                            scale(pal.rock[1], 0.8),
                            1.0,
                        );
                        if l.biome == Biome::Glimmerdeep && h % 2 == 0 {
                            c.circle(bx, y + 28.0 + len, 1.6, 0x62f2ff, 0.9);
                        }
                    }
                    _ => {}
                }
            }
        }
    }
}

fn add_light(map: &mut [f32], cx: f32, cy: f32, r: f32, power: f32) {
    let w2 = (W / 2) as i32;
    let h2 = (H / 2) as i32;
    let (cx, cy, r) = (cx * 0.5, cy * 0.5, r * 0.5);
    let x0 = ((cx - r) as i32).max(0);
    let x1 = ((cx + r) as i32 + 1).min(w2);
    let y0 = ((cy - r) as i32).max(0);
    let y1 = ((cy + r) as i32 + 1).min(h2);
    for y in y0..y1 {
        for x in x0..x1 {
            let dx = x as f32 - cx;
            let dy = y as f32 - cy;
            let d2 = (dx * dx + dy * dy) / (r * r);
            if d2 < 1.0 {
                let f = (1.0 - d2).powi(2);
                let i = (y * w2 + x) as usize;
                map[i] += f * power;
            }
        }
    }
}

fn light_pass(c: &mut Canvas, cache: &mut SceneCache, g: &Game, camx: f32, camy: f32, time: f32) {
    let w = &g.world;
    let ambient = cache.pal.ambient;
    for v in cache.light.iter_mut() {
        *v = ambient;
    }
    let map = &mut cache.light;
    let on_screen = |x: f32, y: f32, m: f32| {
        x > camx - m && x < camx + W as f32 + m && y > camy - m && y < camy + H as f32 + m
    };
    // Wren's lantern
    let flicker = 1.0 + 0.04 * (time * 11.0).sin() + 0.03 * (time * 23.0).sin();
    add_light(
        map,
        w.p.x - camx,
        w.p.y - camy,
        250.0 * w.mods.lamp * flicker,
        1.1,
    );
    for e in &w.ents {
        if !e.alive || !on_screen(e.x, e.y, 200.0) {
            continue;
        }
        let (r, p) = match e.kind {
            EntKind::Crystal => (80.0, 0.5),
            EntKind::EchoOrb | EntKind::HeartShard | EntKind::Chest => (110.0, 0.7),
            EntKind::Wisp => (130.0, 0.8),
            EntKind::Bolt => (80.0, 0.7),
            EntKind::Geyser => (if e.state == 2 { 220.0 } else { 90.0 }, 0.8),
            EntKind::Saw => (70.0, 0.4),
            EntKind::Spitter => (if e.state == 1 { 130.0 } else { 50.0 }, 0.5),
            EntKind::Bloom => (60.0, 0.3),
            _ => continue,
        };
        add_light(map, e.x - camx, e.y - camy, r, p);
    }
    for (i, &(lx, ly)) in w.level.checkpoints.iter().enumerate() {
        if on_screen(lx, ly, 300.0) {
            add_light(
                map,
                lx - camx,
                ly - 62.0 - camy,
                if w.cp_next > i { 260.0 } else { 70.0 },
                1.0,
            );
        }
    }
    let (ex, ey) = w.level.exit;
    if on_screen(ex, ey, 300.0) {
        add_light(map, ex - camx, ey - camy, 240.0, 0.9);
    }
    for (i, st) in w.anchors.iter().enumerate() {
        if st.alive && on_screen(st.x, st.y, 100.0) {
            let _ = i;
            add_light(map, st.x - camx, st.y - camy, 70.0, 0.45);
        }
    }
    if let Some(b) = &w.boss {
        add_light(map, b.x - camx, b.y - camy, 340.0, 0.8);
    }
    // lava glow in the Cinderworks
    if w.level.biome == Biome::Cinderworks {
        let top = w.level.pixel_h() - 70.0 - camy;
        for x in (0..W as i32).step_by(80) {
            add_light(map, x as f32, top, 260.0, 0.5);
        }
    }
    // apply: tinted shadows multiplied by the light map
    let (tr, tg, tb) = match w.level.biome {
        Biome::Cinderworks => (0.30, 0.16, 0.14),
        _ => (0.16, 0.19, 0.38),
    };
    let w2 = W / 2;
    for y in 0..H {
        let row = (y / 2) * w2;
        for x in 0..W {
            let k = cache.light[row + x / 2].clamp(0.0, 1.0);
            let i = y * W + x;
            let (r, g2, b) = split(c.px[i]);
            let kr = tr + (1.0 - tr) * k;
            let kg = tg + (1.0 - tg) * k;
            let kb = tb + (1.0 - tb) * k;
            c.px[i] = pack(r * kr, g2 * kg, b * kb);
        }
    }
}
