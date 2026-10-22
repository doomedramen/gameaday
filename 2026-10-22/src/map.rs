//! Island grid, terrain and procedural generation.

use crate::rng::Rng;

/// Islands are N x N grids.
pub const N: i32 = 11;
pub const MAX_HEIGHT: i32 = 3;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
pub struct Pos {
    pub x: i32,
    pub y: i32,
}

impl Pos {
    pub const fn new(x: i32, y: i32) -> Self {
        Pos { x, y }
    }
    pub fn manhattan(self, o: Pos) -> i32 {
        (self.x - o.x).abs() + (self.y - o.y).abs()
    }
    pub fn chebyshev(self, o: Pos) -> i32 {
        (self.x - o.x).abs().max((self.y - o.y).abs())
    }
    pub fn add(self, dx: i32, dy: i32) -> Pos {
        Pos::new(self.x + dx, self.y + dy)
    }
    pub fn in_bounds(self) -> bool {
        self.x >= 0 && self.y >= 0 && self.x < N && self.y < N
    }
    pub fn neighbors(self) -> [Pos; 4] {
        [
            self.add(1, 0),
            self.add(-1, 0),
            self.add(0, 1),
            self.add(0, -1),
        ]
    }
    /// Unit step direction from self towards o (dominant axis), used for pushes.
    pub fn dir_to(self, o: Pos) -> (i32, i32) {
        let dx = o.x - self.x;
        let dy = o.y - self.y;
        if dx.abs() >= dy.abs() {
            (dx.signum(), 0)
        } else {
            (0, dy.signum())
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Terrain {
    Ground,
    Water,
    Chasm,
    Rock,
    Tree,
    Brazier,
}

impl Terrain {
    pub fn walkable(self) -> bool {
        self == Terrain::Ground
    }
    /// Pushing a (non-boss) unit onto this is lethal.
    pub fn lethal(self) -> bool {
        matches!(self, Terrain::Water | Terrain::Chasm)
    }
    pub fn blocks(self) -> bool {
        matches!(self, Terrain::Rock | Terrain::Tree | Terrain::Brazier)
    }
    pub fn name(self) -> &'static str {
        match self {
            Terrain::Ground => "Ground",
            Terrain::Water => "Deep water",
            Terrain::Chasm => "The void",
            Terrain::Rock => "Boulder",
            Terrain::Tree => "Tree",
            Terrain::Brazier => "Brazier",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pickup {
    /// Heals 3 HP.
    Ember,
    /// +1 lantern radius for the rest of the battle.
    Lumen,
}

impl Pickup {
    pub fn name(self) -> &'static str {
        match self {
            Pickup::Ember => "Ember shard",
            Pickup::Lumen => "Lumen moth",
        }
    }
    pub fn describe(self) -> &'static str {
        match self {
            Pickup::Ember => "Step here to heal 3 HP",
            Pickup::Lumen => "Step here: +1 lantern radius this battle",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Biome {
    Moss,
    Ash,
    Frost,
    Crystal,
    Hollow,
}

impl Biome {
    pub const NORMAL: [Biome; 4] = [Biome::Moss, Biome::Ash, Biome::Frost, Biome::Crystal];

    pub fn name(self) -> &'static str {
        match self {
            Biome::Moss => "Mossmere",
            Biome::Ash => "Ashen Reach",
            Biome::Frost => "Rimeholt",
            Biome::Crystal => "Glasswild",
            Biome::Hollow => "The Hollow Throne",
        }
    }
    pub fn has_water(self) -> bool {
        matches!(self, Biome::Moss | Biome::Frost)
    }
    /// Fraction of tiles that become obstacles.
    pub fn clutter(self) -> f32 {
        match self {
            Biome::Moss => 0.13,
            Biome::Ash => 0.09,
            Biome::Frost => 0.11,
            Biome::Crystal => 0.12,
            Biome::Hollow => 0.06,
        }
    }
    /// Probability an inner tile is a void crack.
    pub fn cracks(self) -> f32 {
        match self {
            Biome::Ash => 0.06,
            Biome::Hollow => 0.05,
            Biome::Crystal => 0.03,
            _ => 0.015,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Tile {
    pub terrain: Terrain,
    pub height: i32,
    /// Braziers only: currently burning.
    pub lit: bool,
    pub pickup: Option<Pickup>,
    /// Rounds of flare light remaining on this tile.
    pub flare: u8,
    /// Visual variation seed.
    pub deco: u8,
}

#[derive(Clone, Debug)]
pub struct Map {
    pub biome: Biome,
    pub tiles: Vec<Tile>,
    pub player_spawns: Vec<Pos>,
    pub enemy_spawns: Vec<Pos>,
}

impl Map {
    pub fn idx(p: Pos) -> usize {
        (p.y * N + p.x) as usize
    }
    pub fn get(&self, p: Pos) -> Option<&Tile> {
        if p.in_bounds() {
            Some(&self.tiles[Map::idx(p)])
        } else {
            None
        }
    }
    pub fn tile(&self, p: Pos) -> &Tile {
        &self.tiles[Map::idx(p)]
    }
    pub fn tile_mut(&mut self, p: Pos) -> &mut Tile {
        &mut self.tiles[Map::idx(p)]
    }
    /// Terrain at p; out-of-bounds counts as the void.
    pub fn terrain(&self, p: Pos) -> Terrain {
        self.get(p).map(|t| t.terrain).unwrap_or(Terrain::Chasm)
    }
    pub fn height(&self, p: Pos) -> i32 {
        self.get(p).map(|t| t.height).unwrap_or(0)
    }
    pub fn walkable(&self, p: Pos) -> bool {
        self.get(p).is_some_and(|t| t.terrain.walkable())
    }
    pub fn all_pos() -> impl Iterator<Item = Pos> {
        (0..N).flat_map(|y| (0..N).map(move |x| Pos::new(x, y)))
    }
    pub fn braziers(&self) -> impl Iterator<Item = Pos> + '_ {
        Map::all_pos().filter(|p| self.tile(*p).terrain == Terrain::Brazier)
    }

    /// Generate a connected island. `extra_braziers` comes from upgrades.
    pub fn generate(seed: u64, biome: Biome, enemy_slots: usize) -> Map {
        let mut rng = Rng::new(seed);
        for _attempt in 0..200 {
            if let Some(m) = Map::try_generate(&mut rng, biome, enemy_slots) {
                return m;
            }
        }
        // Fallback: a plain flat island (never expected, but never fail).
        Map::flat(biome, enemy_slots)
    }

    fn flat(biome: Biome, enemy_slots: usize) -> Map {
        let mut tiles = vec![
            Tile {
                terrain: Terrain::Ground,
                height: 0,
                lit: false,
                pickup: None,
                flare: 0,
                deco: 0,
            };
            (N * N) as usize
        ];
        tiles[Map::idx(Pos::new(5, 5))].terrain = Terrain::Brazier;
        let player_spawns = vec![Pos::new(9, 9), Pos::new(8, 9), Pos::new(9, 8)];
        let enemy_spawns = Map::all_pos()
            .filter(|p| p.x + p.y <= 6)
            .take(enemy_slots.max(1))
            .collect();
        Map {
            biome,
            tiles,
            player_spawns,
            enemy_spawns,
        }
    }

    fn try_generate(rng: &mut Rng, biome: Biome, enemy_slots: usize) -> Option<Map> {
        let shape = ValueNoise::new(rng, 4);
        let relief = ValueNoise::new(rng, 3);
        let wet = ValueNoise::new(rng, 4);
        let c = (N - 1) as f32 / 2.0;
        let mut tiles = Vec::with_capacity((N * N) as usize);
        for p in Map::all_pos() {
            let fx = p.x as f32 / (N - 1) as f32;
            let fy = p.y as f32 / (N - 1) as f32;
            let dx = (p.x as f32 - c) / c;
            let dy = (p.y as f32 - c) / c;
            let d = (dx * dx + dy * dy).sqrt();
            let inside = d + (shape.sample(fx, fy) - 0.5) * 0.55 < 1.02;
            let h = (relief.sample(fx, fy) * (MAX_HEIGHT as f32 + 1.4) - 0.5)
                .round()
                .clamp(0.0, MAX_HEIGHT as f32) as i32;
            let mut terrain = if inside {
                Terrain::Ground
            } else {
                Terrain::Chasm
            };
            if inside && biome.has_water() && h == 0 && wet.sample(fx, fy) > 0.58 {
                terrain = Terrain::Water;
            }
            if inside && d < 0.8 && rng.chance(biome.cracks()) {
                terrain = Terrain::Chasm;
            }
            tiles.push(Tile {
                terrain,
                height: if terrain == Terrain::Water { 0 } else { h },
                lit: false,
                pickup: None,
                flare: 0,
                deco: (rng.next_u64() & 0xff) as u8,
            });
        }
        let mut map = Map {
            biome,
            tiles,
            player_spawns: vec![],
            enemy_spawns: vec![],
        };
        map.smooth_heights();

        // Player spawn: the walkable tiles nearest the front corner.
        let front = Pos::new(N - 2, N - 2);
        let mut ground: Vec<Pos> = Map::all_pos().filter(|p| map.walkable(*p)).collect();
        ground.sort_by_key(|p| (p.manhattan(front), p.x, p.y));
        let anchor = *ground.first()?;
        if anchor.manhattan(front) > 3 {
            return None;
        }

        // Keep only what is reachable from the anchor; the rest falls away.
        let reach = map.flood(anchor);
        for p in Map::all_pos() {
            if map.walkable(p) && !reach[Map::idx(p)] {
                map.tile_mut(p).terrain = Terrain::Chasm;
            }
        }
        let reachable: Vec<Pos> = Map::all_pos().filter(|p| reach[Map::idx(*p)]).collect();
        if reachable.len() < 55 {
            return None;
        }

        let mut near: Vec<Pos> = reachable.clone();
        near.sort_by_key(|p| (p.manhattan(front), p.x, p.y));
        let player_spawns: Vec<Pos> = near.iter().take(3).copied().collect();
        let mut far: Vec<Pos> = reachable
            .iter()
            .copied()
            .filter(|p| p.manhattan(anchor) >= 8)
            .collect();
        if far.len() < enemy_slots + 2 {
            return None;
        }
        rng.shuffle(&mut far);
        // Prefer the back of the island for enemies.
        far.sort_by_key(|p| p.x + p.y);
        let pool = far.len().min(enemy_slots * 2 + 4);
        let mut enemy_spawns: Vec<Pos> = far[..pool].to_vec();
        rng.shuffle(&mut enemy_spawns);
        enemy_spawns.truncate(enemy_slots.max(1) + 2);

        // Braziers: spread out, away from spawns, never sealing paths.
        let reserved =
            |p: Pos| player_spawns.iter().any(|s| s.manhattan(p) <= 1) || enemy_spawns.contains(&p);
        let mut braziers: Vec<Pos> = vec![];
        let want = 4;
        let mut candidates: Vec<Pos> = reachable
            .iter()
            .copied()
            .filter(|p| !reserved(*p))
            .collect();
        rng.shuffle(&mut candidates);
        while braziers.len() < want {
            let best = candidates
                .iter()
                .copied()
                .filter(|p| !braziers.contains(p))
                .max_by_key(|p| {
                    let d_b = braziers.iter().map(|b| b.manhattan(*p)).min().unwrap_or(20);
                    let d_s = player_spawns
                        .iter()
                        .map(|s| s.manhattan(*p))
                        .min()
                        .unwrap_or(20);
                    // First brazier sits near the squad; later ones spread.
                    if braziers.is_empty() {
                        -(d_s - 3).abs() * 10
                    } else {
                        d_b.min(6) * 10 + d_s.min(5)
                    }
                })?;
            let saved = map.tile(best).terrain;
            map.tile_mut(best).terrain = Terrain::Brazier;
            if !map.connected(&reachable) {
                map.tile_mut(best).terrain = saved;
                candidates.retain(|c| *c != best);
                continue;
            }
            braziers.push(best);
        }
        map.tile_mut(braziers[0]).lit = true;

        // Obstacles.
        let mut clutter: Vec<Pos> = reachable
            .iter()
            .copied()
            .filter(|p| !reserved(*p) && map.walkable(*p))
            .collect();
        rng.shuffle(&mut clutter);
        let count = (clutter.len() as f32 * biome.clutter()) as usize;
        let mut placed = 0;
        for p in clutter {
            if placed >= count {
                break;
            }
            let kind = if rng.chance(0.55) {
                Terrain::Tree
            } else {
                Terrain::Rock
            };
            map.tile_mut(p).terrain = kind;
            let still: Vec<Pos> = reachable
                .iter()
                .copied()
                .filter(|q| map.walkable(*q))
                .collect();
            if map.connected(&still) {
                placed += 1;
            } else {
                map.tile_mut(p).terrain = Terrain::Ground;
            }
        }

        // Pickups.
        let mut free: Vec<Pos> = Map::all_pos()
            .filter(|p| map.walkable(*p) && !reserved(*p))
            .collect();
        rng.shuffle(&mut free);
        for (i, p) in free.iter().take(3).enumerate() {
            map.tile_mut(*p).pickup = Some(if i == 2 { Pickup::Lumen } else { Pickup::Ember });
        }

        map.player_spawns = player_spawns;
        map.enemy_spawns = enemy_spawns;
        if map.enemy_spawns.iter().any(|p| !map.walkable(*p))
            || map.player_spawns.iter().any(|p| !map.walkable(*p))
        {
            return None;
        }
        Some(map)
    }

    /// Clamp heights so neighbouring ground differs by at most one level.
    fn smooth_heights(&mut self) {
        loop {
            let mut changed = false;
            for p in Map::all_pos() {
                if self.tile(p).terrain == Terrain::Chasm {
                    continue;
                }
                for n in p.neighbors() {
                    if let Some(t) = self.get(n)
                        && t.terrain != Terrain::Chasm
                        && self.tile(p).height > t.height + 1
                    {
                        let h = t.height + 1;
                        self.tile_mut(p).height = h;
                        changed = true;
                    }
                }
            }
            if !changed {
                break;
            }
        }
    }

    /// Walkable tiles reachable from `from` (4-connected, height step <= 1).
    pub fn flood(&self, from: Pos) -> Vec<bool> {
        let mut seen = vec![false; (N * N) as usize];
        if !self.walkable(from) {
            return seen;
        }
        let mut stack = vec![from];
        seen[Map::idx(from)] = true;
        while let Some(p) = stack.pop() {
            for n in p.neighbors() {
                if self.walkable(n)
                    && !seen[Map::idx(n)]
                    && (self.height(n) - self.height(p)).abs() <= 1
                {
                    seen[Map::idx(n)] = true;
                    stack.push(n);
                }
            }
        }
        seen
    }

    /// True if every walkable tile in `set` is mutually reachable.
    fn connected(&self, set: &[Pos]) -> bool {
        let live: Vec<Pos> = set.iter().copied().filter(|p| self.walkable(*p)).collect();
        let Some(first) = live.first() else {
            return false;
        };
        let seen = self.flood(*first);
        live.iter().all(|p| seen[Map::idx(*p)])
    }
}

/// Bilinear value noise on a small lattice, sampled in [0,1]^2.
struct ValueNoise {
    cells: usize,
    values: Vec<f32>,
}

impl ValueNoise {
    fn new(rng: &mut Rng, cells: usize) -> Self {
        let values = (0..(cells + 1) * (cells + 1)).map(|_| rng.f32()).collect();
        ValueNoise { cells, values }
    }
    fn sample(&self, x: f32, y: f32) -> f32 {
        let gx = x.clamp(0.0, 1.0) * self.cells as f32;
        let gy = y.clamp(0.0, 1.0) * self.cells as f32;
        let x0 = (gx.floor() as usize).min(self.cells - 1);
        let y0 = (gy.floor() as usize).min(self.cells - 1);
        let tx = smooth(gx - x0 as f32);
        let ty = smooth(gy - y0 as f32);
        let w = self.cells + 1;
        let v = |x: usize, y: usize| self.values[y * w + x];
        let a = v(x0, y0) + (v(x0 + 1, y0) - v(x0, y0)) * tx;
        let b = v(x0, y0 + 1) + (v(x0 + 1, y0 + 1) - v(x0, y0 + 1)) * tx;
        a + (b - a) * ty
    }
}

fn smooth(t: f32) -> f32 {
    t * t * (3.0 - 2.0 * t)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_islands_are_connected_and_valid() {
        for seed in 0..300u64 {
            let biome = if seed % 5 == 4 {
                Biome::Hollow
            } else {
                Biome::NORMAL[(seed % 4) as usize]
            };
            let m = Map::generate(seed, biome, 7);
            assert_eq!(m.player_spawns.len(), 3, "seed {seed}");
            assert!(m.enemy_spawns.len() >= 7, "seed {seed}");
            let reach = m.flood(m.player_spawns[0]);
            for p in Map::all_pos() {
                if m.walkable(p) {
                    assert!(reach[Map::idx(p)], "seed {seed}: {p:?} unreachable");
                }
            }
            for s in m.enemy_spawns.iter().chain(m.player_spawns.iter()) {
                assert!(m.walkable(*s));
            }
            assert_eq!(m.braziers().count(), 4, "seed {seed}");
            assert_eq!(m.braziers().filter(|b| m.tile(*b).lit).count(), 1);
            for p in Map::all_pos() {
                if m.walkable(p) {
                    for n in p.neighbors() {
                        if m.walkable(n) {
                            assert!((m.height(p) - m.height(n)).abs() <= 1);
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn generation_is_deterministic() {
        let a = Map::generate(42, Biome::Moss, 5);
        let b = Map::generate(42, Biome::Moss, 5);
        assert_eq!(a.tiles, b.tiles);
        assert_eq!(a.enemy_spawns, b.enemy_spawns);
    }

    #[test]
    fn out_of_bounds_is_void() {
        let m = Map::generate(1, Biome::Ash, 4);
        assert_eq!(m.terrain(Pos::new(-1, 3)), Terrain::Chasm);
        assert!(m.terrain(Pos::new(N, 0)).lethal());
    }
}
