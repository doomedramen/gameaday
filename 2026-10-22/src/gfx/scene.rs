//! Draws the island: per-pixel lit isometric tiles with floating cliffs,
//! props, units, overlays, projectiles and particles.

use super::canvas::{Canvas, hash2, lerp_color, lit, scale, split};
use super::iso::{HALF_H, HALF_W, STEP, pos_center, surface, tile_center, tile_top};
use super::palette::Palette;
use super::sprites::{self, Look};
use super::view::{PKind, View};
use crate::battle::{Battle, Fx};
use crate::map::{Map, N, Pos, Terrain};
use crate::units::{Kind, Team};

/// What the UI wants highlighted on the board.
#[derive(Clone, Debug, Default)]
pub struct Overlay {
    pub moves: Vec<Pos>,
    pub targets: Vec<Pos>,
    pub target_color: u32,
    pub aoe: Vec<Pos>,
    pub path: Vec<Pos>,
    pub hover: Option<Pos>,
    pub danger: Vec<Pos>,
    pub threat: Vec<Pos>,
    pub selected: Option<u32>,
    /// Push preview: landing tile and whether it is lethal.
    pub push: Option<(Pos, bool)>,
}

fn corner_light(glow: &[f32]) -> Vec<f32> {
    let n = N as usize;
    let mut out = vec![0.0; (n + 1) * (n + 1)];
    for cy in 0..=n {
        for cx in 0..=n {
            let mut sum = 0.0;
            let mut cnt = 0.0;
            for (dx, dy) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
                let tx = cx as i32 - dx;
                let ty = cy as i32 - dy;
                if tx >= 0 && ty >= 0 && tx < N && ty < N {
                    sum += glow[ty as usize * n + tx as usize];
                    cnt += 1.0;
                }
            }
            out[cy * (n + 1) + cx] = if cnt > 0.0 { sum / cnt } else { 0.0 };
        }
    }
    out
}

fn ground_color(map: &Map, pal: &Palette, p: Pos) -> u32 {
    let t = map.tile(p);
    let h = t.height.clamp(0, 3) as usize;
    let base = pal.ground[h];
    let v = (t.deco as f32 / 255.0 - 0.5) * 0.12;
    scale(base, 1.0 + v)
}

#[allow(clippy::too_many_arguments)]
fn draw_top(c: &mut Canvas, map: &Map, pal: &Palette, p: Pos, corners: &[f32], t: f32) {
    let tile = map.tile(p);
    let h = surface(map, p);
    let (tx, ty) = tile_top(p.x as f32, p.y as f32, h);
    let n1 = (N + 1) as usize;
    let ci = |x: i32, y: i32| corners[y as usize * n1 + x as usize];
    let c00 = ci(p.x, p.y);
    let c10 = ci(p.x + 1, p.y);
    let c01 = ci(p.x, p.y + 1);
    let c11 = ci(p.x + 1, p.y + 1);
    let water = tile.terrain == Terrain::Water;
    let base = if water {
        pal.water
    } else {
        ground_color(map, pal, p)
    };
    let tx = tx + c.ox;
    let ty = ty + c.oy;
    let x0 = (tx - HALF_W).floor() as i32;
    let y0 = ty.floor() as i32;
    for yy in y0..y0 + (HALF_H * 2.0) as i32 + 1 {
        if yy < 0 || yy >= c.h as i32 {
            continue;
        }
        let dy = yy as f32 + 0.5 - ty;
        for xx in x0..x0 + (HALF_W * 2.0) as i32 + 1 {
            let dx = xx as f32 + 0.5 - tx;
            let u = (dy / HALF_H + dx / HALF_W) * 0.5;
            let v = (dy / HALF_H - dx / HALF_W) * 0.5;
            if !(0.0..=1.0).contains(&u) || !(0.0..=1.0).contains(&v) {
                continue;
            }
            let g = (c00 + (c10 - c00) * u) * (1.0 - v) + (c01 + (c11 - c01) * u) * v;
            let hsh = hash2(xx, yy);
            let mut col = base;
            if water {
                let w = (t * 2.0 + xx as f32 * 0.12 + yy as f32 * 0.35).sin();
                let w2 = (t * 1.3 - xx as f32 * 0.07 + yy as f32 * 0.2).sin();
                col = lerp_color(
                    pal.water,
                    pal.water_hi,
                    ((w + w2) * 0.25 + 0.25).clamp(0.0, 1.0) * 0.5,
                );
                if w * w2 > 0.85 && hsh.is_multiple_of(5) {
                    col = pal.water_hi;
                }
            } else if hsh.is_multiple_of(13) {
                col = lerp_color(col, pal.speckle, 0.35);
            } else if hsh.is_multiple_of(17) {
                col = scale(col, 0.85);
            }
            if u < 0.035 || v < 0.035 || u > 0.965 || v > 0.965 {
                col = scale(col, 0.82);
            }
            let out = lit(col, pal.ambient, g, pal.warm);
            c.put(xx, yy, out);
        }
    }
}

fn draw_sides(c: &mut Canvas, map: &Map, pal: &Palette, p: Pos, glow: f32) {
    let tile = map.tile(p);
    let h = surface(map, p);
    let (tx, ty) = tile_top(p.x as f32, p.y as f32, h);
    let tx = tx + c.ox;
    let ty = ty + c.oy;
    let deco = tile.deco as i32;
    let light = pal.ambient + (1.0 - pal.ambient) * glow * 0.8;
    for (face, nb) in [(0, p.add(0, 1)), (1, p.add(1, 0))] {
        let void = map.terrain(nb) == Terrain::Chasm;
        let base_depth = if void {
            54.0 + h.max(0.0) * STEP + (deco % 5) as f32 * 7.0
        } else {
            (h - surface(map, nb)).max(0.0) * STEP + 3.0
        };
        let shade = if face == 0 { 0.82 } else { 0.6 };
        let side = scale(pal.side, shade * light);
        let dark = scale(pal.side_dark, shade);
        let xs = if face == 0 {
            ((tx - HALF_W).floor() as i32, tx.ceil() as i32)
        } else {
            (tx.floor() as i32, (tx + HALF_W).ceil() as i32)
        };
        for xx in xs.0..xs.1 {
            if xx < 0 || xx >= c.w as i32 {
                continue;
            }
            let fx = xx as f32 + 0.5 - tx;
            let top = if face == 0 {
                ty + HALF_H + (fx + HALF_W) * 0.5
            } else {
                ty + HALF_H * 2.0 - fx * 0.5
            };
            let mut depth = base_depth;
            if void {
                // Taper to a point beneath the tile so islands float.
                let centre = 1.0 - fx.abs() / HALF_W;
                let n = (xx as f32 * 0.23 + deco as f32).sin() * 0.5
                    + (xx as f32 * 0.071 + deco as f32 * 1.7).sin() * 0.5;
                depth += centre * centre.sqrt() * 50.0 + n * 7.0;
            }
            let y0 = top.floor() as i32;
            let y1 = (top + depth).ceil() as i32;
            for yy in y0.max(0)..y1.min(c.h as i32) {
                let k = (yy as f32 - top) / depth.max(1.0);
                let mut col = lerp_color(side, dark, k * 0.9);
                if (yy + deco + (fx * 0.3) as i32).rem_euclid(9) == 0 {
                    col = scale(col, 0.82);
                }
                if face == 0 && k < 0.08 && !void {
                    col = scale(col, 1.1);
                }
                let a = if void && k > 0.7 {
                    1.0 - (k - 0.7) / 0.3
                } else {
                    1.0
                };
                c.blend(xx, yy, col, a);
            }
        }
    }
}

fn diamond(c: &mut Canvas, map: &Map, p: Pos, color: u32, alpha: f32, inset: f32) {
    let (tx, ty) = tile_top(p.x as f32, p.y as f32, surface(map, p));
    let i = inset;
    c.poly(
        &[
            (tx, ty + i * 0.5),
            (tx + HALF_W - i, ty + HALF_H),
            (tx, ty + HALF_H * 2.0 - i * 0.5),
            (tx - HALF_W + i, ty + HALF_H),
        ],
        color,
        alpha,
    );
}

fn diamond_outline(
    c: &mut Canvas,
    map: &Map,
    p: Pos,
    color: u32,
    alpha: f32,
    inset: f32,
    thick: f32,
) {
    let (tx, ty) = tile_top(p.x as f32, p.y as f32, surface(map, p));
    let i = inset;
    let pts = [
        (tx, ty + i * 0.5),
        (tx + HALF_W - i, ty + HALF_H),
        (tx, ty + HALF_H * 2.0 - i * 0.5),
        (tx - HALF_W + i, ty + HALF_H),
    ];
    for k in 0..4 {
        let a = pts[k];
        let b = pts[(k + 1) % 4];
        c.line(a.0, a.1, b.0, b.1, thick, color, alpha);
    }
}

fn draw_overlay_tile(c: &mut Canvas, map: &Map, p: Pos, ov: &Overlay, t: f32) {
    let pulse = 0.5 + 0.5 * (t * 4.0).sin();
    if ov.danger.contains(&p) {
        diamond(c, map, p, 0xff2040, 0.16, 3.0);
    }
    if ov.threat.contains(&p) {
        diamond(c, map, p, 0xff6a20, 0.2, 3.0);
        diamond_outline(c, map, p, 0xff8a40, 0.4, 4.0, 1.0);
    }
    if ov.moves.contains(&p) {
        diamond(c, map, p, 0x50b0ff, 0.22, 3.0);
        diamond_outline(c, map, p, 0x90d0ff, 0.55, 4.0, 1.2);
    }
    if ov.aoe.contains(&p) {
        diamond(c, map, p, 0xff9030, 0.3 + 0.1 * pulse, 3.0);
    }
    if ov.targets.contains(&p) {
        diamond(c, map, p, ov.target_color, 0.28 + 0.12 * pulse, 3.0);
        diamond_outline(c, map, p, ov.target_color, 0.9, 3.0, 1.6);
    }
    if ov.hover == Some(p) {
        diamond_outline(c, map, p, 0xffffff, 0.85, 2.0, 1.5);
    }
}

/// Draw the island terrain and props. `units` callback draws per-diagonal
/// objects so they interleave correctly with the terrain.
pub fn draw_island<F: FnMut(&mut Canvas, i32)>(
    c: &mut Canvas,
    map: &Map,
    glow: &[f32],
    pal: &Palette,
    ov: &Overlay,
    t: f32,
    mut objects: F,
) {
    let corners = corner_light(glow);
    for d in 0..(2 * N - 1) {
        for x in 0..N {
            let y = d - x;
            if !(0..N).contains(&y) {
                continue;
            }
            let p = Pos::new(x, y);
            let tile = map.tile(p);
            if tile.terrain == Terrain::Chasm {
                continue;
            }
            let g = glow[Map::idx(p)];
            draw_sides(c, map, pal, p, g);
            draw_top(c, map, pal, p, &corners, t);
            draw_overlay_tile(c, map, p, ov, t);
        }
        // Path preview dots on this diagonal.
        for (i, p) in ov.path.iter().enumerate() {
            if p.x + p.y == d && i > 0 {
                let (x, y) = pos_center(map, *p);
                let last = i == ov.path.len() - 1;
                c.circle(x, y, if last { 4.0 } else { 2.5 }, 0xd0f0ff, 0.9);
            }
        }
        for x in 0..N {
            let y = d - x;
            if !(0..N).contains(&y) {
                continue;
            }
            let p = Pos::new(x, y);
            let tile = map.tile(p);
            let (cx, cy) = pos_center(map, p);
            let g = glow[Map::idx(p)];
            let light = pal.ambient + (1.0 - pal.ambient) * g;
            match tile.terrain {
                Terrain::Rock => sprites::rock(c, cx, cy, tile.deco, pal, light, map.biome),
                Terrain::Tree => sprites::tree(c, cx, cy, tile.deco, pal, light, map.biome, t),
                Terrain::Brazier => sprites::brazier(c, cx, cy, tile.lit, light, t, tile.deco),
                _ => {}
            }
            if let Some(k) = tile.pickup {
                sprites::pickup(c, cx, cy, k, t);
            }
            if tile.flare > 0 {
                let f = 0.8 + 0.2 * (t * 13.0 + x as f32).sin();
                c.glow(cx, cy - 6.0, 26.0 * f, 0xffd080, 0.5);
                c.circle(cx, cy - 4.0, 3.0, 0xffffe0, 0.9);
            }
        }
        objects(c, d);
    }
}

pub fn draw_battle(c: &mut Canvas, b: &Battle, v: &View, pal: &Palette, ov: &Overlay) {
    let t = v.time;
    let glow: &[f32] = if v.glow.len() == b.glow.len() {
        &v.glow
    } else {
        &b.glow
    };
    // Order units by their visual depth.
    let mut order: Vec<(i32, u32)> = v
        .vis
        .iter()
        .filter(|(_, vis)| vis.visible)
        .map(|(id, vis)| {
            let d = ((vis.x + vis.y).ceil() as i32).clamp(0, 2 * N - 2);
            (d, *id)
        })
        .collect();
    order.sort();
    let selected = ov.selected;
    draw_island(c, &b.map, glow, pal, ov, t, |c, d| {
        for (ud, id) in order.iter() {
            if *ud != d {
                continue;
            }
            draw_unit(c, b, v, glow, pal, *id, selected == Some(*id));
        }
    });
    draw_effects(c, v, pal);
}

fn draw_unit(
    c: &mut Canvas,
    b: &Battle,
    v: &View,
    glow: &[f32],
    pal: &Palette,
    id: u32,
    selected: bool,
) {
    let Some(vis) = v.vis.get(&id) else {
        return;
    };
    let u = b.unit(id);
    let (mut x, mut y) = tile_center(vis.x, vis.y, vis.h);
    x += vis.lunge.0;
    y += vis.lunge.1;
    let feet_y = y;
    y += -vis.hop + vis.drop + (1.0 - vis.spawn) * 12.0;
    let tile_glow = {
        let px = vis.x.round().clamp(0.0, (N - 1) as f32) as i32;
        let py = vis.y.round().clamp(0.0, (N - 1) as f32) as i32;
        glow[Map::idx(Pos::new(px, py))]
    };
    let squad = u.team() == Team::Squad;
    let light = if squad {
        0.78 + 0.22 * tile_glow
    } else if u.kind == Kind::HollowKing {
        0.7 + 0.3 * tile_glow
    } else {
        0.45 + 0.55 * tile_glow
    };
    let alpha = vis.alpha * vis.spawn.max(0.05);
    let shrouded = !squad && u.alive && !b.is_lit(u.pos) && !v.busy();
    let big = u.kind == Kind::HollowKing;
    // Shadow and selection ring
    if vis.drop < 1.0 {
        c.ellipse(
            x,
            feet_y + 1.0,
            if big { 30.0 } else { 14.0 },
            if big { 10.0 } else { 5.0 },
            0x000000,
            0.35 * alpha,
        );
    }
    if selected {
        let p = 0.5 + 0.5 * (v.time * 5.0).sin();
        c.ring(x, feet_y, 18.0, 7.0, 2.0, 0xffd060, 0.6 + 0.4 * p);
        c.glow_ellipse(x, feet_y, 26.0, 10.0, 0xffc040, 0.25);
    }
    if u.elite {
        c.ring(x, feet_y, 16.0, 6.0, 1.5, 0xffd040, 0.7 * alpha);
    }
    if u.guarded || vis.guard_fx > 0.0 {
        let g = 0.5 + vis.guard_fx;
        c.ring(x, y - 20.0, 17.0, 24.0, 1.5, 0x80c0ff, 0.35 * g * alpha);
    }
    let spent = squad && u.moved && u.acted && b.phase == crate::battle::Phase::Player;
    let look = Look {
        facing: u.facing as f32,
        t: v.time + id as f32 * 0.7,
        light: if spent { light * 0.6 } else { light },
        alpha,
        flash: vis.flash,
        shrouded,
        wounded: u.hp * 2 < u.max_hp,
    };
    sprites::draw_unit(c, u.kind, x, y, look);
    if shrouded {
        c.glow_ellipse(x, y - 18.0, 20.0, 26.0, 0x5020a0, 0.25);
    }
    // HP pips
    if vis.drop < 1.0 && alpha > 0.3 {
        let top = y - sprites::unit_height(u.kind) - 8.0;
        draw_hp(c, x, top, vis.hp, u.max_hp, squad, big);
        let mut ix = x + if big { 40.0 } else { 18.0 };
        if u.weakened > 0 {
            c.poly(
                &[
                    (ix - 3.0, top - 4.0),
                    (ix + 3.0, top - 4.0),
                    (ix, top + 2.0),
                ],
                0xc070ff,
                1.0,
            );
            ix += 8.0;
        }
        if shrouded {
            c.ellipse(ix, top - 1.0, 4.0, 2.6, 0xb090ff, 0.95);
            c.circle(ix, top - 1.0, 1.4, 0x200830, 1.0);
        }
        if u.kind == Kind::Shade && u.lurking && b.phase == crate::battle::Phase::Enemy {
            c.glow(x, y - 30.0, 10.0, 0xff3040, 0.5);
        }
    }
    let _ = pal;
}

fn draw_hp(c: &mut Canvas, x: f32, y: f32, hp: i32, max: i32, squad: bool, big: bool) {
    let full = if squad { 0x7ae070 } else { 0xff5a5a };
    let frac = hp.max(0) as f32 / max.max(1) as f32;
    let col = if squad && frac < 0.35 { 0xffc040 } else { full };
    if big || max > 16 {
        let w = if big { 70.0 } else { 36.0 };
        c.fill_rect(x - w / 2.0 - 1.0, y - 1.0, w + 2.0, 6.0, 0x000000, 0.7);
        c.fill_rect(x - w / 2.0, y, w * frac, 4.0, col, 1.0);
        return;
    }
    let pip = 3.0;
    let gap = 1.0;
    let w = max as f32 * (pip + gap) - gap;
    c.fill_rect(
        x - w / 2.0 - 1.5,
        y - 1.5,
        w + 3.0,
        pip + 3.0,
        0x000000,
        0.65,
    );
    for i in 0..max {
        let px = x - w / 2.0 + i as f32 * (pip + gap);
        let on = i < hp;
        c.fill_rect(px, y, pip, pip, if on { col } else { 0x3a3a3a }, 1.0);
    }
}

fn draw_effects(c: &mut Canvas, v: &View, pal: &Palette) {
    // Projectile
    if let Some(pr) = &v.projectile {
        let (ax, ay) = pr.from;
        let (bx, by) = pr.to;
        let dist = ((bx - ax).powi(2) + (by - ay).powi(2)).sqrt();
        let arc = match pr.fx {
            Fx::Arrow | Fx::Volley => dist * 0.25,
            Fx::Flare => dist * 0.35,
            _ => dist * 0.1,
        };
        let at = |t: f32| {
            (
                ax + (bx - ax) * t,
                ay + (by - ay) * t - (t * std::f32::consts::PI).sin() * arc,
            )
        };
        let (x, y) = at(pr.t);
        let (px, py) = at((pr.t - 0.06).max(0.0));
        match pr.fx {
            Fx::Arrow | Fx::Volley => {
                let n = if pr.fx == Fx::Volley { 3 } else { 1 };
                for i in 0..n {
                    let o = (i as f32 - (n - 1) as f32 / 2.0) * 7.0;
                    let dx = x - px;
                    let dy = y - py;
                    let len = (dx * dx + dy * dy).sqrt().max(0.01);
                    let (ux, uy) = (dx / len, dy / len);
                    c.line(
                        x - ux * 12.0 + o,
                        y - uy * 12.0,
                        x + o,
                        y,
                        1.5,
                        0xe8e0d0,
                        1.0,
                    );
                    c.line(
                        x - ux * 12.0 + o,
                        y - uy * 12.0,
                        x - ux * 9.0 + o - uy * 3.0,
                        y - uy * 9.0 + ux * 3.0,
                        1.2,
                        0xffffff,
                        0.8,
                    );
                }
            }
            Fx::Flare => {
                for k in 0..6 {
                    let (tx, ty) = at((pr.t - k as f32 * 0.03).max(0.0));
                    c.glow(tx, ty, 14.0 - k as f32 * 1.8, 0xff8a30, 0.6);
                }
                c.circle(x, y, 4.0, 0xffffd0, 1.0);
            }
            Fx::Bolt => {
                c.glow(x, y, 16.0, 0x30e0c0, 0.8);
                c.circle(x, y, 3.5, 0xe0fff8, 1.0);
            }
            _ => {
                c.glow(x, y, 16.0, 0xa040ff, 0.8);
                c.ring(x, y, 6.0, 6.0, 1.5, 0xe0b0ff, 0.9);
            }
        }
    }
    // Eclipse shockwave
    if v.eclipse > 0.0 {
        let (ex, ey) = v.eclipse_at;
        let r = v.eclipse * 760.0;
        c.ring(
            ex,
            ey - 20.0,
            r,
            r * 0.5,
            30.0 * (1.0 - v.eclipse) + 4.0,
            0x000000,
            0.6 * (1.0 - v.eclipse),
        );
        c.ring(
            ex,
            ey - 20.0,
            r,
            r * 0.5,
            3.0,
            0xff2040,
            0.8 * (1.0 - v.eclipse),
        );
    }
    // Particles
    for p in &v.particles {
        let k = (p.life / p.max).clamp(0.0, 1.0);
        match p.kind {
            PKind::Smoke => c.circle(p.x, p.y, p.size, p.color, 0.45 * k),
            PKind::Ring => {
                let r = p.size * (1.0 - k) + 4.0;
                c.ring(p.x, p.y, r, r * 0.5, 2.0, p.color, k);
            }
            PKind::Spark => {
                c.line(
                    p.x,
                    p.y,
                    p.x - p.vx * 0.03,
                    p.y - p.vy * 0.03,
                    p.size,
                    p.color,
                    k,
                );
            }
            PKind::Dot => {
                if p.additive {
                    c.glow(p.x, p.y, p.size * 3.0, p.color, 0.7 * k);
                    c.circle(p.x, p.y, p.size * 0.6, p.color, k);
                } else {
                    c.circle(p.x, p.y, p.size, p.color, k);
                }
            }
        }
    }
    // Floating text
    for t in &v.texts {
        let k = (t.life / t.max).clamp(0.0, 1.0);
        let a = (k * 2.5).min(1.0);
        c.text_center(t.x, t.y, &t.text, t.scale, t.color, a);
    }
    let _ = pal;
}

/// Precomputed vignette multiplier.
pub fn vignette(w: usize, h: usize) -> Vec<f32> {
    let mut out = vec![1.0; w * h];
    for y in 0..h {
        for x in 0..w {
            let dx = (x as f32 / w as f32 - 0.5) * 2.0;
            let dy = (y as f32 / h as f32 - 0.48) * 2.0;
            let d = (dx * dx * 0.6 + dy * dy * 0.8).sqrt();
            out[y * w + x] = (1.15 - d * 0.45).clamp(0.45, 1.0);
        }
    }
    out
}

pub fn apply_vignette(c: &mut Canvas, vig: &[f32]) {
    for (p, k) in c.px.iter_mut().zip(vig.iter()) {
        if *k < 0.999 {
            let (r, g, b) = split(*p);
            *p = super::canvas::pack(r * k, g * k, b * k);
        }
    }
}

pub fn apply_flash(c: &mut Canvas, color: u32, a: f32) {
    if a <= 0.0 {
        return;
    }
    let (cr, cg, cb) = split(color);
    for p in c.px.iter_mut() {
        let (r, g, b) = split(*p);
        *p = super::canvas::pack(r + (cr - r) * a, g + (cg - g) * a, b + (cb - b) * a);
    }
}
