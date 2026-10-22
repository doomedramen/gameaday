//! Isometric projection between grid and screen space.

use crate::map::{Map, Pos};

pub const HALF_W: f32 = 32.0;
pub const HALF_H: f32 = 16.0;
/// Screen pixels per height level.
pub const STEP: f32 = 14.0;
pub const ORIGIN_X: f32 = 640.0;
pub const ORIGIN_Y: f32 = 158.0;

/// Top corner of the tile's top face.
pub fn tile_top(x: f32, y: f32, h: f32) -> (f32, f32) {
    (
        ORIGIN_X + (x - y) * HALF_W,
        ORIGIN_Y + (x + y) * HALF_H - h * STEP,
    )
}

/// Centre of the tile's top face, where units stand.
pub fn tile_center(x: f32, y: f32, h: f32) -> (f32, f32) {
    let (tx, ty) = tile_top(x, y, h);
    (tx, ty + HALF_H)
}

pub fn pos_center(map: &Map, p: Pos) -> (f32, f32) {
    tile_center(p.x as f32, p.y as f32, surface(map, p))
}

/// Visual surface height (water sits a little lower).
pub fn surface(map: &Map, p: Pos) -> f32 {
    match map.get(p) {
        Some(t) if t.terrain == crate::map::Terrain::Water => -0.35,
        Some(t) => t.height as f32,
        None => 0.0,
    }
}

/// Topmost tile whose top face contains the screen point.
pub fn pick(map: &Map, sx: f32, sy: f32) -> Option<Pos> {
    let mut best: Option<Pos> = None;
    let mut best_depth = -1;
    for p in Map::all_pos() {
        let t = map.tile(p);
        if t.terrain == crate::map::Terrain::Chasm {
            continue;
        }
        let (tx, ty) = tile_top(p.x as f32, p.y as f32, surface(map, p));
        let dx = (sx - tx).abs() / HALF_W;
        let dy = (sy - ty - HALF_H).abs() / HALF_H;
        if dx + dy <= 1.0 {
            let depth = p.x + p.y;
            if depth > best_depth {
                best_depth = depth;
                best = Some(p);
            }
        }
    }
    best
}
