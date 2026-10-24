//! Tile atlas: every solid tile shape (by which neighbours are solid),
//! variant and depth is rendered once per biome, then blitted.

use super::canvas::{hash2, lerp_color, rgb, scale, split};
use super::palette::Pal;
use crate::level::Biome;

pub const TS: usize = 32;
const TPX: usize = TS * TS;
pub const CLEAR: u32 = 0xFF00_0000;
const DEPTHS: usize = 4;
const VARS: usize = 3;

pub struct Atlas {
    px: Vec<u32>,
}

#[inline]
fn slot(depth: usize, var: usize, mask: usize) -> usize {
    ((depth * VARS + var) * 16 + mask) * TPX
}

impl Atlas {
    pub fn new(biome: Biome, pal: &Pal) -> Atlas {
        let mut px = vec![CLEAR; DEPTHS * VARS * 16 * TPX];
        for depth in 0..DEPTHS {
            for var in 0..VARS {
                for mask in 0..16 {
                    let o = slot(depth, var, mask);
                    render_tile(&mut px[o..o + TPX], biome, pal, mask, var, depth);
                }
            }
        }
        Atlas { px }
    }

    pub fn tile(&self, depth: usize, var: usize, mask: usize) -> &[u32] {
        let o = slot(depth.min(DEPTHS - 1), var % VARS, mask & 15);
        &self.px[o..o + TPX]
    }
}

fn soft(a: f32, b: f32, x: f32) -> f32 {
    ((x - a) / (b - a)).clamp(0.0, 1.0)
}

fn render_tile(out: &mut [u32], biome: Biome, pal: &Pal, mask: usize, var: usize, depth: usize) {
    let up = mask & 1 != 0;
    let right = mask & 2 != 0;
    let down = mask & 4 != 0;
    let left = mask & 8 != 0;
    let shade = 1.0 - 0.13 * depth as f32;
    let vs = var as i32 * 53;
    for py in 0..TS as i32 {
        for px in 0..TS as i32 {
            let n = (hash2(px + vs, py + mask as i32 * 7) & 0xff) as f32 / 255.0;
            let n2 = (hash2(px / 3 + vs, py / 3 + 91) & 0xff) as f32 / 255.0;
            // corner rounding where two exposed sides meet
            let r = 7.0;
            let mut clear = false;
            let corner = |cx: f32, cy: f32| -> bool {
                let dx = px as f32 + 0.5 - cx;
                let dy = py as f32 + 0.5 - cy;
                dx * dx + dy * dy > r * r
            };
            if !up && !left && px < r as i32 && py < r as i32 && corner(r, r) {
                clear = true;
            }
            if !up
                && !right
                && px >= TS as i32 - r as i32
                && py < r as i32
                && corner(TS as f32 - r, r)
            {
                clear = true;
            }
            if !down
                && !left
                && px < r as i32
                && py >= TS as i32 - r as i32
                && corner(r, TS as f32 - r)
            {
                clear = true;
            }
            if !down
                && !right
                && px >= TS as i32 - r as i32
                && py >= TS as i32 - r as i32
                && corner(TS as f32 - r, TS as f32 - r)
            {
                clear = true;
            }
            // ragged underside
            if !down {
                let cut =
                    26 + (hash2(px + vs, 5) & 3) as i32 + if (px / 4) % 2 == 0 { 1 } else { 0 };
                if py > cut + 1 {
                    clear = true;
                }
            }
            if clear {
                out[(py as usize) * TS + px as usize] = CLEAR;
                continue;
            }
            // base: strata between mid and dark
            let strata = ((py + vs / 3) as f32 * 0.35 + (px as f32 * 0.07)).sin() * 0.5 + 0.5;
            let mut col = lerp_color(pal.rock[1], pal.rock[2], 0.25 + 0.4 * strata + 0.2 * n2);
            col = lerp_color(col, pal.rock[0], 0.12 * n);
            col = scale(col, shade);
            // distance to exposed edges
            let dl = if left { 99.0 } else { px as f32 };
            let dr = if right {
                99.0
            } else {
                (TS as i32 - 1 - px) as f32
            };
            let dt = if up { 99.0 } else { py as f32 };
            let db = if down {
                99.0
            } else {
                (TS as i32 - 1 - py) as f32
            };
            let d = dl.min(dr).min(dt).min(db);
            // lit rim on top/left, dark rim bottom/right
            if dt < 3.0 {
                col = lerp_color(col, pal.rock[0], 0.55 * (1.0 - dt / 3.0));
            }
            if dl < 2.0 {
                col = lerp_color(col, pal.rock[0], 0.3 * (1.0 - dl / 2.0));
            }
            if dr < 2.0 || db < 3.0 {
                col = scale(col, 0.78);
            }
            if d < 1.0 {
                col = scale(col, 0.62);
            }
            // surface cap
            if !up {
                let cap_h = 5.0
                    + 3.0 * ((hash2(px / 2 + vs, 3) & 0xff) as f32 / 255.0)
                    + if px % 5 == 2 { 2.0 } else { 0.0 };
                if (py as f32) < cap_h {
                    let t = soft(0.0, cap_h, py as f32);
                    let mut cap = lerp_color(pal.surface, pal.surface_dark, t * 0.8);
                    match biome {
                        Biome::Frostspire => {
                            // sparkle
                            if hash2(px * 5 + vs, py * 3) & 31 == 0 {
                                cap = 0xffffff;
                            }
                        }
                        Biome::Glimmerdeep => {
                            if hash2(px * 5 + vs, py * 3) & 15 == 0 {
                                cap = pal.glow;
                            }
                        }
                        Biome::Cinderworks => {
                            // glowing cracks
                            if (py == 4 || py == 5) && ((px + var as i32 * 7) % 11 < 4) {
                                cap = pal.glow;
                            }
                        }
                        Biome::Beacon => {
                            if py == 0 {
                                cap = 0xfff2b8;
                            }
                        }
                        _ => {
                            if py < 2 {
                                cap = lerp_color(cap, 0xffffff, 0.18);
                            }
                        }
                    }
                    col = scale(cap, shade.max(0.85));
                } else if (py as f32) < cap_h + 1.5 {
                    col = scale(col, 0.7);
                }
            }
            // speckles of stone
            if hash2(px * 3 + vs, py * 7 + 13) & 31 == 0 {
                col = lerp_color(col, pal.rock[0], 0.4);
            } else if hash2(px * 7 + vs, py * 3 + 41) & 31 == 0 {
                col = scale(col, 0.7);
            }
            out[(py as usize) * TS + px as usize] = col;
        }
    }
    // a few cracks on interior tiles
    if mask == 15 && var == 1 {
        let (r, g, b) = split(pal.rock[2]);
        let c = rgb((r * 0.6) as u8, (g * 0.6) as u8, (b * 0.6) as u8);
        let mut x = 6;
        for y in 4..22 {
            x += (hash2(y, var as i32) % 3) as i32 - 1;
            let xi = x.clamp(0, 31) as usize;
            out[(y as usize) * TS + xi] = c;
        }
    }
}
