//! Procedural ascent generator. Levels are built left to right from
//! traversal primitives (runs, jumps, stairs, wall shafts, rope swings,
//! drops) whose geometry is chosen so that Wren's measured movement can
//! always cross them; hazards, pickups and decoration are layered on top.

use crate::level::*;
use crate::rng::Rng;
use crate::sim::HH;

pub const LEVEL_H: i32 = 54;
const START_ROW: i32 = 32;
const MIN_ROW: i32 = 20;
const MAX_ROW: i32 = 40;

#[derive(Clone, Debug)]
pub struct StageSpec {
    pub biome: Biome,
    /// 0..=4 for the five ascents, 5 for the boss.
    pub stage: usize,
    pub heat: u8,
    /// Daring route: harder, richer.
    pub daring: bool,
    pub seed: u64,
    pub tutorial: bool,
}

impl StageSpec {
    pub fn new(stage: usize, seed: u64) -> StageSpec {
        StageSpec {
            biome: if stage < 5 {
                Biome::RUN[stage]
            } else {
                Biome::Beacon
            },
            stage,
            heat: 0,
            daring: false,
            seed,
            tutorial: false,
        }
    }

    /// Difficulty on a 0..1.5 scale.
    pub fn difficulty(&self) -> f32 {
        let base = [0.0, 0.25, 0.5, 0.75, 1.0, 1.1][self.stage.min(5)];
        let route = if self.daring { 0.18 } else { -0.05 };
        (base + route + self.heat as f32 * 0.16).clamp(0.0, 1.5)
    }
}

#[derive(Clone, Copy, Debug)]
struct Plat {
    x0: i32,
    x1: i32,
    y: i32,
}

struct G {
    l: Level,
    rng: Rng,
    d: f32,
    stage: usize,
    biome: Biome,
    cx: i32,
    cy: i32,
    plats: Vec<Plat>,
    need_top: Vec<i32>,
    surface: Tile,
    next_checkpoint: i32,
}

fn px(tile: i32) -> f32 {
    tile as f32 * T
}

/// Player centre y when standing on floor row `row`.
fn stand_y(row: i32) -> f32 {
    px(row) - HH
}

impl G {
    fn new(spec: &StageSpec, w: i32) -> G {
        let mut l = Level::new(w, LEVEL_H, spec.biome, spec.seed);
        l.tutorial = spec.tutorial;
        G {
            rng: Rng::new(spec.seed ^ (spec.stage as u64 * 0x9E37) ^ 0x51ab),
            d: spec.difficulty(),
            stage: spec.stage,
            biome: spec.biome,
            cx: 0,
            cy: START_ROW,
            plats: vec![],
            need_top: vec![LEVEL_H; w as usize],
            surface: if spec.biome == Biome::Frostspire {
                ICE
            } else {
                ROCK
            },
            next_checkpoint: 46,
            l,
        }
    }

    fn need(&mut self, x0: i32, x1: i32, row: i32) {
        for x in x0.max(0)..x1.min(self.l.w) {
            let n = &mut self.need_top[x as usize];
            *n = (*n).min(row);
        }
    }

    /// A floating island: tapering rock slab with a surface layer.
    fn slab(&mut self, x0: i32, x1: i32, y: i32, thick: i32, flat_end: bool) {
        let len = x1 - x0;
        for r in 0..thick {
            let inset = if r < 2 { 0 } else { (r - 1).min((len - 2) / 2) };
            let inset_r = if flat_end { 0 } else { inset };
            for x in (x0 + inset)..(x1 - inset_r) {
                let t = if r == 0 { self.surface } else { ROCK };
                self.l.set(x, y + r, t);
            }
        }
        self.need(x0, x1, y - 9);
        self.plats.push(Plat { x0, x1, y });
    }

    /// Is every tile in the rectangle open air?
    fn region_air(&self, x0: i32, x1: i32, y0: i32, y1: i32) -> bool {
        (y0..y1).all(|y| (x0..x1).all(|x| matches!(self.l.tile(x, y), AIR)))
    }

    /// A 3x3 patch of air around the tile (so floaters do not sit in rock).
    fn clear_air(&self, x: i32, y: i32) -> bool {
        (-1..=1).all(|dy| (-1..=1).all(|dx| matches!(self.l.tile(x + dx, y + dy), AIR)))
    }

    fn crystal(&mut self, x: f32, y: f32) {
        self.l.ents.push(EntSpawn::new(EntKind::Crystal, x, y));
    }

    /// Crystals along a floor stretch.
    fn crystal_row(&mut self, x0: i32, x1: i32, row: i32, every: i32) {
        let mut x = x0;
        while x < x1 {
            self.crystal(px(x) + T / 2.0, px(row) - 44.0);
            x += every;
        }
    }

    /// Crystals along the jump arc between two stand points.
    fn crystal_arc(&mut self, xa: f32, ya: f32, xb: f32, yb: f32, n: i32, lift: f32) {
        for i in 1..=n {
            let t = i as f32 / (n + 1) as f32;
            let x = xa + (xb - xa) * t;
            let y = ya + (yb - ya) * t - lift * 4.0 * t * (1.0 - t);
            self.crystal(x, y - 30.0);
        }
    }

    fn beat(&mut self, kind: BeatKind, x0: f32, y0: f32, x1: f32, y1: f32, edge_x: f32) {
        self.l.beats.push(Beat {
            kind,
            x0,
            y0,
            x1,
            y1,
            anchors: vec![],
            edge_x,
        });
    }

    fn jump_gap(&mut self) -> i32 {
        let hi = 3 + (self.d > 0.45) as i32 + (self.d > 0.95) as i32;
        let hi = hi.min(4);
        self.rng.range(2, hi)
    }

    // --------------------------------------------------------- primitives

    /// Lay the opening platform.
    fn start(&mut self, len: i32) {
        self.slab(0, len, self.cy, 6, false);
        self.l.spawn = (px(3) + T / 2.0, stand_y(self.cy));
        self.cx = len;
    }

    /// Extend with a run platform of `len` tiles starting at the cursor.
    fn run(&mut self, len: i32) {
        let (x0, x1, y) = (self.cx, self.cx + len, self.cy);
        self.slab(x0, x1, y, 5, false);
        self.cx = x1;
    }

    /// Jump across `gap` tiles to a platform `rise` rows higher (negative =
    /// lower) with `len` tiles of floor.
    fn jump(&mut self, gap: i32, rise: i32, len: i32) {
        let edge = self.cx;
        let from_y = self.cy;
        let to_y = (self.cy - rise).clamp(MIN_ROW, MAX_ROW);
        let x0 = edge + gap;
        self.slab(x0, x0 + len, to_y, 5, false);
        self.beat(
            BeatKind::Jump,
            px(edge),
            px(from_y),
            px(x0) + T,
            px(to_y),
            px(edge),
        );
        if gap >= 2 {
            self.crystal_arc(
                px(edge),
                px(from_y),
                px(x0),
                px(to_y),
                gap.min(3),
                44.0 + gap as f32 * 14.0,
            );
        }
        self.need(edge, x0, from_y.min(to_y) - 9);
        self.cx = x0 + len;
        self.cy = to_y;
    }

    /// Series of small crumbling platforms.
    fn crumble_bridge(&mut self, n: i32) {
        for _ in 0..n {
            let gap = self.rng.range(2, 3);
            let edge = self.cx;
            let y = self.cy;
            let x0 = edge + gap;
            let len = self.rng.range(2, 3);
            for r in 0..2 {
                for x in x0..x0 + len {
                    self.l.set(x, y + r, CRUMBLE);
                }
            }
            self.need(x0, x0 + len, y - 9);
            self.plats.push(Plat {
                x0,
                x1: x0 + len,
                y: -1,
            });
            self.beat(BeatKind::Jump, px(edge), px(y), px(x0) + T, px(y), px(edge));
            self.crystal(px(x0) + T, px(y) - 70.0);
            self.cx = x0 + len;
        }
        // solid landing after the bridge
        let gap = 2;
        let edge = self.cx;
        let x0 = edge + gap;
        let y = self.cy;
        self.slab(x0, x0 + 6, y, 5, false);
        self.beat(BeatKind::Jump, px(edge), px(y), px(x0) + T, px(y), px(edge));
        self.cx = x0 + 6;
    }

    /// Wall-jump shaft climbing `h` rows. Bottom floor is the current row.
    fn shaft(&mut self, h: i32) {
        let width = 4;
        let sx = self.cx; // left wall starts here
        let floor = self.cy;
        let top = (floor - h).max(MIN_ROW - 2);
        let h = floor - top;
        let ix0 = sx + 2;
        let ix1 = ix0 + width;
        // floor through the shaft bottom
        self.l.fill(sx, floor, ix1 + 2, floor + 5, ROCK);
        // left wall above the opening
        self.l.fill(sx, top - 4, sx + 2, floor - 5, ROCK);
        // right wall up to the new platform
        self.l.fill(ix1, top, ix1 + 2, floor, ROCK);
        for x in ix1..ix1 + 2 {
            self.l.set(x, top, self.surface);
        }
        self.need(sx, ix1 + 2, top - 9);
        self.beat(
            BeatKind::Shaft,
            px(sx) - T,
            px(floor),
            px(ix1) + T,
            px(top),
            px(sx),
        );
        // crystals up the middle
        let mut r = floor - 4;
        while r > top + 1 {
            self.crystal(px(ix0 + width / 2), px(r));
            r -= 3;
        }
        self.plats.push(Plat {
            x0: ix1,
            x1: ix1 + 2,
            y: top,
        });
        self.cx = ix1 + 2;
        self.cy = top;
        let _ = h;
    }

    /// Chasm with rope anchors. `gw` tiles wide, landing `rise` rows higher.
    fn swing(&mut self, gw: i32, rise: i32, len: i32, vault: bool) {
        let edge = self.cx;
        let from_y = self.cy;
        let to_y = (self.cy - rise).clamp(MIN_ROW, MAX_ROW);
        let x_land = edge + gw;
        let n = ((gw as f32) / 8.5).ceil() as i32;
        let mut ids = vec![];
        for i in 0..n {
            let frac = (i as f32 + 0.5) / n as f32;
            let ax = px(edge) + px(gw) * frac + T * 0.5;
            // anchors hang 8 rows above the lower of the two floors, drifting
            // toward the landing height
            let base = from_y as f32 + (to_y - from_y) as f32 * frac;
            let ay = px(0) + (base - 8.0 - self.rng.range(0, 1) as f32) * T;
            let kind = self.pick_anchor_kind();
            let (x2, y2) = if kind == AnchorKind::Rail {
                (ax + 2.5 * T, ay)
            } else {
                (ax, ay)
            };
            let ax = if kind == AnchorKind::Rail { ax - T } else { ax };
            ids.push(self.l.anchors.len());
            self.l.anchors.push(Anchor {
                x: ax,
                y: ay,
                kind,
                x2: x2 - if kind == AnchorKind::Rail { T } else { 0.0 },
                y2,
                phase: self.rng.f32() * 6.28,
            });
            self.need(
                ((ax - 3.0 * T) / T) as i32,
                ((ax + 3.5 * T) / T) as i32,
                (ay / T) as i32 - 3,
            );
        }
        let _ = vault;
        self.slab(x_land, x_land + len, to_y, 5, false);
        let mut b = Beat {
            kind: BeatKind::Swing,
            x0: px(edge),
            y0: px(from_y),
            x1: px(x_land) + T,
            y1: px(to_y),
            anchors: ids,
            edge_x: px(edge),
        };
        // the bot lands on top of the landing platform, not in the vault
        b.x1 = px(x_land) + T * 1.5;
        self.l.beats.push(b);
        self.crystal_arc(px(edge), px(from_y), px(x_land), px(to_y), 3, 120.0);
        self.cx = x_land + len;
        self.cy = to_y;
    }

    fn pick_anchor_kind(&mut self) -> AnchorKind {
        let r = self.rng.f32();
        match self.biome {
            Biome::Glimmerdeep | Biome::Frostspire if r < 0.28 + self.d * 0.2 => {
                AnchorKind::Brittle
            }
            Biome::Cinderworks if r < 0.3 => AnchorKind::Rail,
            Biome::Cinderworks if r < 0.5 => AnchorKind::Brittle,
            Biome::Stormcrown if r < 0.25 => AnchorKind::Rail,
            Biome::Stormcrown if r < 0.45 => AnchorKind::Brittle,
            _ => AnchorKind::Ring,
        }
    }

    // -------------------------------------------------------- composition

    fn compose(&mut self, target_x: i32) {
        let mut last_kind = 0;
        let mut swings = 0;
        let mut shafts = 0;
        let mut gates = 0;
        while self.cx < target_x - 40 {
            // checkpoint on a platform we are standing at the end of
            let weights = self.weights(last_kind, swings, shafts, gates);
            let pick = self.rng.range(0, weights.iter().sum::<i32>() - 1);
            let mut acc = 0;
            let mut kind = 0;
            for (i, w) in weights.iter().enumerate() {
                acc += *w;
                if pick < acc {
                    kind = i;
                    break;
                }
            }
            last_kind = kind;
            match kind {
                0 => {
                    // jump chain
                    let gap = self.jump_gap();
                    let rise = if gap <= 3 {
                        self.rng.range(-2, 2)
                    } else {
                        self.rng.range(-3, 1)
                    };
                    let len = self.rng.range(5, 9 - (self.d * 2.0) as i32);
                    self.jump(gap, rise, len);
                }
                1 => {
                    // stairs
                    let n = self.rng.range(2, 3);
                    for _ in 0..n {
                        let gap = self.rng.range(1, 2);
                        self.jump(gap, 2, 4);
                    }
                    let n = self.rng.range(2, 5);
                    self.run(n);
                }
                2 => {
                    // long run with hazards later
                    let n = self.rng.range(7, 12);
                    self.run(n);
                }
                3 => {
                    let h = 3 * self.rng.range(3, 4 + (self.d > 0.6) as i32);
                    if self.cy - h >= MIN_ROW - 2 {
                        self.shaft(h);
                        shafts += 1;
                        let n = self.rng.range(4, 7);
                        self.run(n);
                    } else {
                        self.jump(3, -3, 6);
                    }
                }
                4 => {
                    let gw = self.rng.range(9, 10 + (self.d * 3.0) as i32);
                    let rise = self.rng.range(-1, 2);
                    let len = self.rng.range(7, 10);
                    self.swing(gw, rise, len, false);
                    swings += 1;
                }
                5 => {
                    // drop down
                    let gap = self.rng.range(2, 3);
                    let drop = self.rng.range(4, 6);
                    self.jump(gap, -drop, 6);
                }
                6 => {
                    let n = self.rng.range(2, 3 + (self.d > 0.6) as i32);
                    self.crumble_bridge(n);
                }
                _ => {
                    // a glass-sealed tunnel through a cliff: needs momentum
                    self.run(5);
                    self.smash_gate(false);
                    gates += 1;
                    self.run(4);
                }
            }
            self.maybe_checkpoint();
        }
        if gates == 0 && self.stage >= 2 && self.cx + 48 < self.l.w {
            // later ascents always include at least one glass door
            self.run(5);
            self.smash_gate(false);
            self.run(4);
        }
        if swings == 0 && self.stage > 0 {
            // guarantee at least one rope crossing in later ascents
            self.run(4);
            self.swing(10, 0, 8, false);
        }
        self.run(6);
    }

    fn weights(&self, last: usize, swings: i32, shafts: i32, gates: i32) -> [i32; 8] {
        // jump, stairs, run, shaft, swing, drop, crumble bridge, glass gate
        let mut w = [30, 12, 8, 14, 20, 10, 8, 0];
        if self.stage >= 1 && gates == 0 {
            w[7] = 9;
        } else if self.stage >= 3 && gates < 2 {
            w[7] = 5;
        }
        if self.stage == 0 {
            w[4] = 10 + 6 * (swings == 0) as i32;
            w[3] = 8;
            w[6] = 14;
        }
        if self.biome == Biome::Cinderworks || self.biome == Biome::Stormcrown {
            w[4] += 8;
        }
        if self.biome == Biome::Frostspire {
            w[6] = 0;
        }
        if self.cy > MAX_ROW - 6 {
            w[5] = 0;
            w[3] += 20;
        }
        if self.cy < MIN_ROW + 6 {
            w[3] = 0;
            w[1] = 0;
            w[5] += 25;
        }
        if shafts > 2 {
            w[3] = 2;
        }
        if last < 8 {
            w[last] = (w[last] / 3).max(if last == 7 { 0 } else { 1 });
        }
        // a gate needs a landing row we can drop four rows from
        if self.cy > MAX_ROW - 6 {
            w[7] = 0;
        }
        if self.cx + 48 >= self.l.w {
            w[7] = 0;
        }
        w
    }

    fn maybe_checkpoint(&mut self) {
        if self.cx >= self.next_checkpoint {
            // platform we just built (last in list, non-crumble)
            if let Some(p) = self
                .plats
                .iter()
                .rev()
                .find(|p| p.y > 0 && p.x1 - p.x0 >= 5)
            {
                let x = px((p.x0 + p.x1) / 2) + T / 2.0;
                let y = px(p.y);
                let far = self
                    .l
                    .checkpoints
                    .last()
                    .map(|c| x - c.0 > 900.0)
                    .unwrap_or(true);
                if far {
                    self.l.checkpoints.push((x, y));
                    self.next_checkpoint = self.cx + 44;
                }
            }
        }
    }

    // ------------------------------------------------------------- hazards

    fn decorate(&mut self) {
        let plats = self.plats.clone();
        let n = plats.len();
        let d = self.d;
        let heat_bonus = if self.l.tutorial { -1.0 } else { 0.0 };
        let mut geyser_count = 0;
        for (i, p) in plats.iter().enumerate() {
            if p.y < 0 || self.l.tutorial {
                continue;
            }
            let len = p.x1 - p.x0;
            let first = i == 0;
            let last = i + 1 >= n;
            if first || last || i < 2 {
                continue;
            }
            // skip platforms holding a checkpoint
            let has_cp = self
                .l
                .checkpoints
                .iter()
                .any(|c| c.0 > px(p.x0) && c.0 < px(p.x1));
            let mut taken = false;
            // crystals everywhere
            if len >= 4 && self.rng.chance(0.7) {
                self.crystal_row(p.x0 + 1, p.x1 - 1, p.y, 2);
            }
            if has_cp || len < 6 {
                continue;
            }
            let chance = |s: &mut G, base: f32| {
                s.rng
                    .chance(((base + d * 0.25) * (1.0 + heat_bonus)).clamp(0.0, 0.9))
            };
            // spikes
            if len >= 8 && !taken && chance(self, 0.18 + 0.05 * self.stage as f32) {
                let slen = self.rng.range(2, 3);
                let sx = self.rng.range(p.x0 + 3, p.x1 - 3 - slen);
                for x in sx..sx + slen {
                    self.l.set(x, p.y - 1, SPIKE_UP);
                }
                self.crystal_arc(px(sx - 1), px(p.y), px(sx + slen + 1), px(p.y), 2, 60.0);
                taken = true;
            }
            // walkers
            if !taken && len >= 7 {
                let bulwark_ok = self.stage >= 1;
                if chance(self, 0.30) {
                    let kind = if bulwark_ok && self.rng.chance(0.35 + d * 0.15) {
                        EntKind::Bulwark
                    } else {
                        EntKind::Skitter
                    };
                    let (hw, hh) = if kind == EntKind::Bulwark {
                        (16.0, 13.0)
                    } else {
                        (12.0, 8.0)
                    };
                    let mut e = EntSpawn::new(kind, px((p.x0 + p.x1) / 2), px(p.y) - hh);
                    e.a = if self.rng.chance(0.5) { 1.0 } else { -1.0 };
                    let _ = hw;
                    self.l.ents.push(e);
                    taken = true;
                }
            }
            // turrets
            if !taken
                && len >= 6
                && matches!(
                    self.biome,
                    Biome::Glimmerdeep | Biome::Cinderworks | Biome::Stormcrown
                )
                && chance(self, 0.30)
            {
                let e = EntSpawn::new(EntKind::Spitter, px(p.x1 - 2) + T / 2.0, px(p.y) - 13.0);
                self.l.ents.push(e);
                taken = true;
            }
            // geysers
            if !taken
                && self.biome == Biome::Cinderworks
                && len >= 7
                && geyser_count < 6
                && chance(self, 0.45)
            {
                let mut e = EntSpawn::new(
                    EntKind::Geyser,
                    px((p.x0 + p.x1) / 2) + T / 2.0,
                    px(p.y) - 6.0,
                );
                e.a = 4.2 - d * 0.8;
                e.b = self.rng.f32() * 4.0;
                self.l.ents.push(e);
                geyser_count += 1;
                taken = true;
            }
            // lightning
            if self.biome == Biome::Stormcrown && len >= 6 && chance(self, 0.4) {
                let mut e = EntSpawn::new(
                    EntKind::Strike,
                    px((p.x0 + p.x1) / 2) + T / 2.0,
                    px(p.y) - 150.0,
                );
                e.hw = px(len) / 2.0 + 90.0;
                e.hh = 220.0;
                self.l.ents.push(e);
            }
            // wisps float above
            if !taken
                && matches!(
                    self.biome,
                    Biome::Glimmerdeep | Biome::Cinderworks | Biome::Stormcrown
                )
                && chance(self, 0.18)
            {
                // hover where there is open air (a shaft wall or ledge may be above)
                let wx = (p.x0 + p.x1) / 2;
                let mut wy = p.y - 5;
                while wy > 2 && !self.clear_air(wx, wy) {
                    wy -= 1;
                }
                if self.clear_air(wx, wy) {
                    let mut e = EntSpawn::new(EntKind::Wisp, px(wx), px(wy));
                    e.a = self.rng.f32() * 6.0;
                    self.l.ents.push(e);
                    taken = true;
                }
            }
            // springblooms with a bonus ledge
            let ledge_row = (p.y - 6).max(MIN_ROW - 4);
            if matches!(self.biome, Biome::Mossbluff | Biome::Stormcrown)
                && len >= 7
                && self.rng.chance(0.28)
                && !taken
                && self.region_air(p.x0 + 2, p.x0 + 8, ledge_row - 5, ledge_row + 3)
            {
                let bx = p.x0 + 2;
                let e = EntSpawn::new(EntKind::Bloom, px(bx) + T / 2.0, px(p.y) - 8.0);
                self.l.ents.push(e);
                // ledge up and to the right, with a treat
                let ly = (p.y - 6).max(MIN_ROW - 4);
                self.l.fill(bx + 1, ly, bx + 5, ly + 2, ROCK);
                self.need(bx + 1, bx + 5, ly - 5);
                self.crystal_row(bx + 1, bx + 5, ly, 1);
                if self.rng.chance(0.55) {
                    let r = self.rng.f32();
                    let kind = if r < 0.35 {
                        EntKind::Chest
                    } else if r < 0.7 {
                        EntKind::EchoOrb
                    } else {
                        EntKind::HeartShard
                    };
                    let dy = if kind == EntKind::Chest { 18.0 } else { 80.0 };
                    self.l
                        .ents
                        .push(EntSpawn::new(kind, px(bx + 3), px(ly) - dy));
                }
            }
        }
        // gap hazards
        self.decorate_gaps();
    }

    fn decorate_gaps(&mut self) {
        let beats = self.l.beats.clone();
        let d = self.d;
        if self.l.tutorial {
            return;
        }
        for (i, b) in beats.iter().enumerate() {
            if i < 3 {
                continue;
            }
            match b.kind {
                BeatKind::Jump => {
                    let gap = (b.x1 - T - b.edge_x) / T;
                    if (3.0..=4.0).contains(&gap) {
                        let cxm = (b.edge_x + b.x1 - T) / 2.0;
                        let cym = b.y0.min(b.y1);
                        if self.biome == Biome::Frostspire && self.rng.chance(0.55) {
                            let mut e = EntSpawn::new(EntKind::Wind, cxm, cym - 90.0);
                            e.hw = gap * T / 2.0 + 90.0;
                            e.hh = 200.0;
                            e.a = if self.rng.chance(0.5) { 620.0 } else { -620.0 };
                            e.c = 4.4;
                            self.l.ents.push(e);
                        } else if matches!(self.biome, Biome::Frostspire | Biome::Cinderworks)
                            && d > 0.45
                            && self.rng.chance(0.35)
                        {
                            let mut e = EntSpawn::new(EntKind::Saw, cxm, cym - 36.0);
                            e.a = cxm;
                            e.b = cym - 130.0;
                            e.c = 1.9;
                            e.hw = self.rng.f32() * 6.28;
                            self.l.ents.push(e);
                        } else if self.biome == Biome::Stormcrown
                            && b.y1 < b.y0 - 30.0
                            && self.rng.chance(0.6)
                        {
                            // an updraft helping up to the higher ledge
                            let mut e = EntSpawn::new(EntKind::Wind, cxm, cym + 20.0);
                            e.hw = gap * T / 2.0;
                            e.hh = 130.0;
                            e.b = -700.0;
                            self.l.ents.push(e);
                        }
                    }
                }
                BeatKind::Swing if d > 0.3 => {
                    // a saw rail beneath the arc raises the cost of falling short
                    if self.biome == Biome::Cinderworks && self.rng.chance(0.4) {
                        let cxm = (b.x0 + b.x1) / 2.0;
                        let mut e = EntSpawn::new(EntKind::Saw, cxm - 90.0, b.y0.max(b.y1) + 40.0);
                        e.a = cxm + 90.0;
                        e.b = b.y0.max(b.y1) + 40.0;
                        e.c = 1.5;
                        e.hw = 0.0;
                        self.l.ents.push(e);
                    }
                }
                _ => {}
            }
        }
    }

    // ---------------------------------------------------------- finishing

    fn ceiling(&mut self) {
        if !self.biome.cave() {
            return;
        }
        let w = self.l.w;
        let mut rng = Rng::new(self.l.seed ^ 0xCE11);
        let mut base = 6.0f32;
        for x in 0..w {
            // smooth the requirement across neighbours
            let mut lim = LEVEL_H;
            for k in -3..=3 {
                let xx = (x + k).clamp(0, w - 1);
                lim = lim.min(self.need_top[xx as usize]);
            }
            base += (rng.f32() - 0.5) * 1.6;
            base = base.clamp(3.0, 12.0);
            let ceil_row = (base as i32).min(lim - 2).max(1);
            for y in 0..ceil_row {
                self.l.set(x, y, ROCK);
            }
        }
        // stalactites
        for x in 0..w {
            if rng.chance(0.18) {
                let mut y = 0;
                while self.l.tile(x, y) == ROCK && y < LEVEL_H {
                    y += 1;
                }
                if y > 15 {
                    // inside a solid cliff, not under the cave ceiling
                    continue;
                }
                let len = rng.range(1, 3);
                let lim = self.need_top[x as usize] - 2;
                for k in 0..len {
                    if y + k < lim {
                        self.l.set(x, y + k, ROCK);
                    }
                }
            }
        }
    }

    fn finish(&mut self) {
        // exit gate on the last platform
        let last = *self.plats.iter().rev().find(|p| p.y > 0).unwrap();
        let ex = px(last.x1 - 3) + T / 2.0;
        self.l.exit = (ex, px(last.y) - 48.0);
        self.l.beats.push(Beat {
            kind: BeatKind::Jump,
            x0: ex - T,
            y0: px(last.y),
            x1: ex,
            y1: px(last.y),
            anchors: vec![],
            edge_x: ex,
        });
        self.ceiling();
        // par from the stage length
        let len = self.l.exit.0 - self.l.spawn.0;
        self.l.par = len / 140.0;
        self.l.finalize();
    }
}

/// Build one stage.
pub fn generate(spec: &StageSpec) -> Level {
    if spec.tutorial {
        return tutorial(spec);
    }
    if spec.stage >= 5 {
        return boss_arena(spec);
    }
    let target = [260, 290, 320, 350, 380][spec.stage.min(4)];
    let mut g = G::new(spec, target + 70);
    g.start(16);
    g.compose(target);
    g.decorate();
    g.finish();
    g.l
}

// ------------------------------------------------------------------ tutorial

fn tutorial(spec: &StageSpec) -> Level {
    let mut g = G::new(spec, 230);
    g.start(18);
    let p = |x0: i32, x1: i32, text: &'static str| Prompt {
        x0: px(x0),
        x1: px(x1),
        text,
    };
    g.l.prompts.push(p(
        2,
        15,
        "A / D or arrows to run.  SPACE to jump (hold for height).",
    ));
    g.run(2);
    g.jump(2, 0, 7);
    g.l.prompts.push(p(
        20,
        30,
        "Hold jump to leap higher. Release early for a short hop.",
    ));
    g.jump(3, 0, 7);
    g.jump(4, 1, 8);
    // stairs up
    g.jump(1, 2, 4);
    g.jump(1, 2, 4);
    g.run(3);
    g.l.prompts.push(p(
        52,
        64,
        "Press into a wall to slide, then JUMP to wall-jump up the shaft.",
    ));
    g.run(2);
    g.shaft(12);
    g.run(5);
    let swing_x = g.cx;
    g.l.prompts.push(p(
        swing_x - 8,
        swing_x + 12,
        "Aim with the mouse and hold LEFT CLICK (or hold SHIFT) to hook. A/D pumps the swing. Let go to fly!",
    ));
    g.swing(10, 0, 9, false);
    g.run(2);
    // spikes and a skitter to stomp
    let sx = g.cx;
    g.run(12);
    for x in sx + 4..sx + 6 {
        g.l.set(x, g.cy - 1, SPIKE_UP);
    }
    let mut e = EntSpawn::new(EntKind::Skitter, px(sx + 9), px(g.cy) - 8.0);
    e.a = -1.0;
    g.l.ents.push(e);
    g.l.prompts.push(p(
        sx,
        sx + 12,
        "Spikes and critters hurt. Land on a Skitter's head to stomp it. Hurt? Hold R to REWIND time.",
    ));
    // smash gate: swing fast into the glass
    g.run(2);
    g.smash_gate(true);
    g.finish_tutorial();
    g.l
}

impl G {
    /// Mandatory glass door closing off the way forward: a tunnel mouth at
    /// the bottom of the rope's arc, sealed with glass. Swing in fast.
    fn smash_gate(&mut self, closed: bool) {
        let edge = self.cx;
        let y = self.cy;
        let gw = 10;
        let x_land = edge + gw;
        // anchor 3 tiles before the wall, 7 rows above the floor
        let ax = px(x_land - 3) + T / 2.0;
        let ay = px(y - 7);
        self.l.anchors.push(Anchor {
            x: ax,
            y: ay,
            kind: AnchorKind::Ring,
            x2: ax,
            y2: ay,
            phase: 0.0,
        });
        let aidx = self.l.anchors.len() - 1;
        // solid wall right up to the sky, tunnel chamber inside, glass door
        self.l.fill(x_land, 0, x_land + 18, y + 9, ROCK);
        let chamber_end = if closed { x_land + 17 } else { x_land + 18 };
        self.l.fill(x_land + 2, y + 1, chamber_end, y + 4, AIR);
        for r in 1..=3 {
            self.l.set(x_land, y + r, GLASS);
            self.l.set(x_land + 1, y + r, GLASS);
        }
        self.beat(
            BeatKind::Swing,
            px(edge),
            px(y),
            px(x_land + 6),
            px(y + 4),
            px(edge),
        );
        self.l.beats.last_mut().unwrap().anchors = vec![aidx];
        self.need(edge - 2, x_land + 1, y - 10);
        if closed {
            self.l.prompts.push(Prompt {
                x0: px(edge - 6),
                x1: px(x_land),
                text: "SPEED breaks glass. Jump, hook the anchor, swing low and let go to fly into the door.",
            });
            self.l.exit = (px(x_land + 14), px(y + 4) - 48.0);
        }
        self.plats.push(Plat {
            x0: x_land + 2,
            x1: x_land + 18,
            y: -1,
        });
        self.cx = x_land + 18;
        self.cy = y + 4;
    }

    fn finish_tutorial(&mut self) {
        self.l.par = 200.0;
        let ex = self.l.exit.0;
        self.l.prompts.push(Prompt {
            x0: ex - 400.0,
            x1: ex + 40.0,
            text: "Training complete! Step into the gate. Boons, routes and the Stormheart await.",
        });
        self.l.finalize();
    }
}

// ----------------------------------------------------------------- boss arena

fn boss_arena(spec: &StageSpec) -> Level {
    let w = 76;
    let h = 46;
    let mut l = Level::new(w, h, Biome::Beacon, spec.seed);
    l.boss = true;
    let floor = 44;
    // floor, walls, ceiling
    l.fill(0, floor, w, h, ROCK);
    l.fill(0, 0, 3, floor, ROCK);
    l.fill(w - 3, 0, w, floor, ROCK);
    l.fill(0, 0, w, 5, ROCK);
    // side platforms to dodge shockwaves from
    l.fill(4, 41, 11, floor, ROCK);
    l.fill(w - 11, 41, w - 4, floor, ROCK);
    // anchors: [0..3) hang above the three slam spots, the rest are for
    // getting around
    for (ax, ay) in [
        (19.5f32, 34.0f32),
        (38.5, 33.0),
        (w as f32 - 19.5, 34.0),
        (28.5, 36.0),
        (w as f32 - 28.5, 36.0),
        (10.0, 30.0),
        (w as f32 - 10.0, 30.0),
    ] {
        l.anchors.push(Anchor {
            x: ax * T,
            y: ay * T,
            kind: AnchorKind::Ring,
            x2: ax * T,
            y2: ay * T,
            phase: 0.0,
        });
    }
    l.spawn = (px(7), px(41) - HH - 1.0);
    l.exit = (px(w / 2) + T / 2.0, px(floor) - 48.0);
    l.checkpoints.clear();
    l.par = 120.0;
    l.beats.push(Beat {
        kind: BeatKind::Boss,
        x0: px(6),
        y0: px(floor),
        x1: px(w / 2),
        y1: px(floor),
        anchors: vec![0, 1, 2, 3, 4, 5, 6],
        edge_x: px(12),
    });
    l.finalize();
    l
}
